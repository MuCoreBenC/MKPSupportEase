/* SupportEase 的 Windows 安装器。
 *
 * 它只做四件事：解压随包的应用 exe、写「应用和功能」里的卸载项、建快捷方式、
 * 把自己复制成 uninstall.exe。界面是 `ui/` 里的三个状态（安装前 / 安装中 / 完成）。
 *
 * ★ 为什么不用 Tauri 自带的 NSIS：那一套的窗口、按钮、进度条都是 Win32 原生控件，
 *   字体与配色改不动。这里用同一个 WebView2 壳，界面完全由 HTML/CSS 控制。
 *
 * ★ 静默安装：`--silent`（也认 NSIS 的 `/S`）。`src-tauri/src/ipc/update.rs` 的更新链
 *   就是靠这个参数在当前版本退出后无界面覆盖安装，**契约不能改**。
 */

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager, WebviewWindow};

/// 由构建脚本在打包前生成（应用 exe + LICENSE 压成 zip）——
/// build.rs 会把它复制进 OUT_DIR；没有时放一个空 zip 占位，源码目录因此不必带二进制
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.zip"));

const PRODUCT: &str = "SupportEase";
const APP_EXE: &str = "mkp-support-ease.exe";
const PUBLISHER: &str = "MuCoreBenC";

/// 应用版本（不是安装器自己的 0.1.0）—— 构建脚本用 MKP_APP_VERSION 注入
fn app_version() -> &'static str {
    option_env!("MKP_APP_VERSION").unwrap_or("0.0.0")
}

#[derive(Clone, Serialize)]
struct Progress {
    pct: u32,
    what: String,
}

fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
}

fn default_install_dir() -> PathBuf {
    local_app_data().join(PRODUCT)
}

fn user_desktop() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"))
        .join("Desktop")
}

fn start_menu_programs() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join(r"Microsoft\Windows\Start Menu\Programs")
}

/* ---------- 前端能调的 ---------- */

#[tauri::command]
fn install_dir() -> String {
    default_install_dir().to_string_lossy().into_owned()
}

#[tauri::command]
fn version() -> String {
    app_version().to_string()
}

#[tauri::command]
fn install(window: WebviewWindow, dir: String, desktop: bool, start_menu: bool) -> Result<(), String> {
    run_install(&PathBuf::from(dir), desktop, start_menu, |pct, what| {
        let _ = window.emit(
            "install-progress",
            Progress {
                pct,
                what: what.to_string(),
            },
        );
    })
}

/// 选文件夹：走系统的 FolderBrowserDialog（PowerShell 一句），不引额外插件
#[tauri::command]
fn pick_dir(current: String) -> Option<String> {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms;\
         $d=New-Object System.Windows.Forms.FolderBrowserDialog;\
         $d.SelectedPath='{}';\
         if($d.ShowDialog() -eq 'OK'){{$d.SelectedPath}}",
        current.replace('\'', "''")
    );
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .output()
        .ok()?;
    let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if picked.is_empty() {
        None
    } else {
        Some(picked)
    }
}

#[tauri::command]
fn run_installed(dir: String) -> Result<(), String> {
    std::process::Command::new(PathBuf::from(dir).join(APP_EXE))
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("启动失败：{e}"))
}

#[tauri::command]
fn close_window(window: WebviewWindow) {
    let _ = window.close();
}

#[tauri::command]
fn minimize_window(window: WebviewWindow) {
    let _ = window.minimize();
}

/* ---------- 安装本体 ---------- */

fn run_install(
    dir: &Path,
    desktop: bool,
    start_menu: bool,
    mut report: impl FnMut(u32, &str),
) -> Result<(), String> {
    report(2, "正在准备安装目录…");
    std::fs::create_dir_all(dir).map_err(|e| format!("创建不了安装目录：{e}"))?;

    // 覆盖安装：先把还在跑的那个关掉（它正在占着 exe 文件）
    let _ = std::process::Command::new("taskkill")
        .args(["/f", "/im", APP_EXE])
        .output();

    report(6, "正在解压文件…");
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(PAYLOAD)).map_err(|e| format!("安装包损坏：{e}"))?;
    let total = archive.len().max(1) as u32;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读安装包失败：{e}"))?;
        let name = entry.name().to_string();
        // payload 是我们自己打的，但仍然按不受信任的输入处理：不许跳出安装目录
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') || name.contains(':')
        {
            return Err(format!("安装包里有非法路径：{name}"));
        }
        let out_path = dir.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).ok();
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
        }
        let mut file = std::fs::File::create(&out_path).map_err(|e| format!("写不了 {name}：{e}"))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| format!("写 {name} 失败：{e}"))?;
        report(6 + i as u32 * 72 / total, "正在解压文件…");
    }

    report(82, "正在写入卸载信息…");
    let me = std::env::current_exe().map_err(|e| format!("找不到安装器自身：{e}"))?;
    std::fs::copy(&me, dir.join("uninstall.exe")).map_err(|e| format!("写卸载器失败：{e}"))?;

    if desktop {
        report(88, "正在创建桌面快捷方式…");
        create_shortcut(&user_desktop().join(format!("{PRODUCT}.lnk")), dir)?;
    }
    if start_menu {
        report(92, "正在创建开始菜单快捷方式…");
        create_shortcut(
            &start_menu_programs().join(format!("{PRODUCT}.lnk")),
            dir,
        )?;
    }

    report(97, "正在写入卸载信息…");
    write_uninstall_entry(dir)?;

    report(100, "完成");
    Ok(())
}

fn quoted(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "''"))
}

fn create_shortcut(lnk: &Path, target_dir: &Path) -> Result<(), String> {
    if let Some(parent) = lnk.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let exe = target_dir.join(APP_EXE);
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut({});$s.TargetPath={};$s.WorkingDirectory={};$s.Save()",
        quoted(lnk),
        quoted(&exe),
        quoted(target_dir),
    );
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|e| format!("调不起 PowerShell：{e}"))?;
    if !status.success() {
        return Err(format!("创建快捷方式失败：{}", lnk.display()));
    }
    Ok(())
}

fn write_uninstall_entry(dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{PRODUCT}");
    let (key, _) = hkcu
        .create_subkey(&path)
        .map_err(|e| format!("写不进卸载项：{e}"))?;

    let install_location = dir.to_string_lossy().into_owned();
    let display_icon = dir.join(APP_EXE).to_string_lossy().into_owned();
    let uninstall_string = format!("\"{}\\uninstall.exe\" --uninstall", install_location);

    let set = |name: &str, value: &str| -> Result<(), String> {
        key.set_value(name, &value.to_string())
            .map_err(|e| format!("写不进卸载项 {name}：{e}"))
    };
    set("DisplayName", PRODUCT)?;
    set("DisplayVersion", app_version())?;
    set("Publisher", PUBLISHER)?;
    set("DisplayIcon", &display_icon)?;
    set("InstallLocation", &install_location)?;
    set("UninstallString", &uninstall_string)?;
    key.set_value("NoModify", &1u32).ok();
    key.set_value("NoRepair", &1u32).ok();
    Ok(())
}

/* ---------- 入口 ---------- */

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let silent = args.iter().any(|a| a == "--silent" || a == "/S");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            install_dir,
            version,
            install,
            pick_dir,
            run_installed,
            close_window,
            minimize_window
        ])
        .setup(move |app| {
            let window = app.get_webview_window("main").expect("窗口没建起来");
            if silent {
                // 更新链：无界面覆盖安装，装完直接退出（退出码就是结果）
                let code = match run_install(&default_install_dir(), true, true, |_, _| {}) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("静默安装失败：{e}");
                        1
                    }
                };
                std::process::exit(code);
            }
            window.show()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("安装器启动失败");
}
