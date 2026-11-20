/* SupportEase 的 Windows 安装器（轻壳）。
 *
 * 结构：一个 Win32 无边框窗口 + 一个 WebView2 控件。界面是 `ui/index.html`
 * （HTML/CSS），中间只有一条 `postMessage(JSON)` 通道 —— **没有 Tauri**。
 *
 * ★ 为什么不用 Tauri：它那 5 MB 是框架（命令注册 / 资源协议 / 插件体系），
 *   而这里只需要「一个窗口 + 一个 WebView2 + 一条 JSON 消息」。WebView2 本身
 *   是系统组件（Win10 1803+ 自带），安装包里不带浏览器内核。
 *
 * ★ 安装本身只做五件事：解压随包的应用 exe、写卸载项、建快捷方式、
 *   把自己复制成 uninstall.exe、静默模式下不建窗口。
 */

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use webview2_com::Microsoft::Web::WebView2::Win32::{
    CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2, ICoreWebView2Controller,
    ICoreWebView2Environment,
};
use webview2_com::{
    take_pwstr, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, NavigationCompletedEventHandler,
    WebMessageReceivedEventHandler,
};
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::*;

const HTML_TEMPLATE: &str = include_str!("../ui/index.html");
const ICON_BLACK: &str = include_str!("../ui/icon-sm.b64");
const ICON_WHITE: &str = include_str!("../ui/icon-sm-white.b64");

/// 由 build.rs 复制进 OUT_DIR（构建脚本生成；没有时是空文件，运行时明确报错）
#[cfg(not(feature = "uninstall-only"))]
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.xz"));

/// 卸载器不内嵌 payload —— 这一份 exe 只有界面 + 删除逻辑
#[cfg(feature = "uninstall-only")]
const PAYLOAD: &[u8] = b"";

const PRODUCT: &str = "SupportEase";
const APP_EXE: &str = "mkp-support-ease.exe";
const PUBLISHER: &str = "MuCoreBenC";
/// 与客户端共用同一个卸载项键（旧 NSIS 版写的也是这里）
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\SupportEase";

const WM_PROGRESS: u32 = WM_APP + 1;
const WM_FINISHED: u32 = WM_APP + 2;
/// WebView2 首帧就绪（或兜底超时）后再显示窗口 —— 先显示窗口会看到一段白框
const WM_REVEAL: u32 = WM_APP + 3;

thread_local! {
    static WEBVIEW: RefCell<Option<ICoreWebView2>> = const { RefCell::new(None) };
    static CONTROLLER: RefCell<Option<ICoreWebView2Controller>> = const { RefCell::new(None) };
}

/// 这个 exe 这次是以「安装器」还是「卸载器」身份跑（由 --uninstall 决定）
static UNINSTALL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

fn is_uninstall() -> bool {
    *UNINSTALL.get().unwrap_or(&false)
}

fn app_version() -> &'static str {
    option_env!("MKP_APP_VERSION").unwrap_or("0.0.0")
}

fn install_dir_of_self() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(default_install_dir)
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

fn build_html() -> String {
    HTML_TEMPLATE
        .replace("{{ICON_BLACK}}", ICON_BLACK.trim())
        .replace("{{ICON_WHITE}}", ICON_WHITE.trim())
}

/* ---------- 与页面之间的消息 ---------- */

struct Progress {
    pct: u32,
    what: String,
}

fn send_json(value: serde_json::Value) {
    let s = HSTRING::from(value.to_string().as_str());
    WEBVIEW.with(|w| {
        if let Some(web) = w.borrow().as_ref() {
            unsafe {
                let _ = web.PostWebMessageAsJson(&s);
            }
        }
    });
}

/// 静默更新必须沿用旧安装目录：更新脚本只传 /S、不传 /D（见 updater.rs），
/// 用户当初装在 D/E/F 盘时若回退默认目录，就会双份安装、且脚本最后拉起的还是旧 exe。
/// 目录不存在（例如用户自己删了）才回退默认位置。
fn silent_install_dir() -> PathBuf {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(UNINSTALL_KEY) {
        if let Ok(raw) = key.get_value::<String, _>("InstallLocation") {
            let trimmed = raw.trim().trim_matches('"');
            let candidate = PathBuf::from(trimmed);
            if !trimmed.is_empty() && candidate.is_dir() {
                return candidate;
            }
        }
    }
    default_install_dir()
}

/// 双击 setup.exe 时先看注册表里有没有装过 —— 只认卸载项，不扫 C/D/E/F 盘。
/// 旧 NSIS 版写的也是同一个键，靠 UninstallString / 目录内容区分是哪一代装的。
/// 绿色版布局：目录里是 SupportEase.exe（不是安装版的 mkp-support-ease.exe），
/// 数据在同目录的 app-data/。绿色版卸载只删自己所在的文件夹，
/// 绝不能碰注册表卸载项 / 快捷方式 / %APPDATA%SupportEase —— 那是安装版的东西。
fn is_portable_layout(dir: &Path) -> bool {
    dir.join("SupportEase.exe").exists() && !dir.join(APP_EXE).exists()
}

fn installed_info() -> serde_json::Value {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey(UNINSTALL_KEY) else {
        return serde_json::json!({ "state": "none" });
    };
    let read = |name: &str| -> String {
        key.get_value::<String, _>(name).unwrap_or_default()
    };
    let version = read("DisplayVersion");
    // 旧 NSIS 写的路径值带引号（"C:\...\SupportEase"），先剥掉再显示/使用
    let location = read("InstallLocation");
    let location = location.trim().trim_matches('"').to_string();
    let uninstall = read("UninstallString");
    if location.is_empty() {
        return serde_json::json!({ "state": "none" });
    }

    let dir = Path::new(&location);
    let self_installed = uninstall.contains("--uninstall")
        || (dir.join("uninstall.exe").exists() && dir.join("LICENSE").exists());
    let generation = if self_installed {
        "self"
    } else if !uninstall.is_empty() {
        "nsis"
    } else {
        "unknown"
    };
    serde_json::json!({
        "state": "installed",
        "version": version,
        "location": location,
        "generation": generation,
        "uninstallString": uninstall,
    })
}

/// 「已安装页」点卸载：自研装的就是它目录里那颗小 uninstall.exe；
/// 旧 NSIS 装的按注册表 UninstallString 原样交给 cmd 执行。
fn uninstall_existing(location: &str, uninstall_string: &str, generation: &str) {
    if generation == "self" {
        let _ = Command::new(Path::new(location).join("uninstall.exe"))
            .arg("--uninstall")
            .spawn();
        return;
    }
    if uninstall_string.is_empty() {
        return;
    }
    let _ = hidden("cmd")
        .args(["/c", "start", "", uninstall_string])
        .spawn();
}

fn on_message(hwnd: isize, raw: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return;
    };
    match v["cmd"].as_str().unwrap_or("") {
        "init" => send_json(serde_json::json!({
            "event": "init",
            "mode": if is_uninstall() { "uninstall" } else { "install" },
            "dir": if is_uninstall() {
                install_dir_of_self().to_string_lossy().into_owned()
            } else {
                default_install_dir().to_string_lossy().into_owned()
            },
            "version": app_version(),
            "portable": is_uninstall() && is_portable_layout(&install_dir_of_self()),
            // 直接随 init 带过去：少一次消息往返，也不会因 WebView 消息时序丢一帧
            "installed": installed_info(),
        })),
        "installed-info" => send_json(installed_info()),
        "uninstall-existing" => {
            let location = v["location"].as_str().unwrap_or("").to_string();
            let uninstall = v["uninstallString"].as_str().unwrap_or("").to_string();
            let generation = v["generation"].as_str().unwrap_or("").to_string();
            uninstall_existing(&location, &uninstall, &generation);
        }
        "close" => unsafe {
            let _ = DestroyWindow(HWND(hwnd as *mut _));
        },
        "minimize" => unsafe {
            let _ = ShowWindow(HWND(hwnd as *mut _), SW_MINIMIZE);
        },
        "drag" => unsafe {
            let _ = ReleaseCapture();
            let _ = SendMessageW(
                HWND(hwnd as *mut _),
                WM_NCLBUTTONDOWN,
                Some(WPARAM(HTCAPTION as usize)),
                Some(LPARAM(0)),
            );
        },
        "default-dir" => send_json(serde_json::json!({
            "event": "dir",
            "path": default_install_dir().to_string_lossy(),
        })),
        "browse" => {
            let current = v["current"].as_str().unwrap_or("").to_string();
            let picked = unsafe { pick_folder(HWND(hwnd as *mut _), &current) };
            match picked {
                Some(path) => {
                    // 用户选的是"装在哪个位置"，不是"装在哪个文件夹"：自动补一层产品名。
                    // 选中的目录本来就叫 SupportEase 时不再叠加。
                    let mut target = PathBuf::from(&path);
                    if target.file_name().and_then(|n| n.to_str()) != Some(PRODUCT) {
                        target = target.join(PRODUCT);
                    }
                    send_json(serde_json::json!({
                        "event": "dir",
                        "path": target.to_string_lossy(),
                    }))
                }
                // 取消也要回一条：前端靠它恢复按钮，否则按钮会一直灰着
                None => send_json(serde_json::json!({ "event": "dir-cancel" })),
            }
        }
        "launch" => {
            let dir = v["dir"].as_str().unwrap_or("").to_string();
            let _ = Command::new(PathBuf::from(dir).join(APP_EXE)).spawn();
        }
        "install" => {
            let dir = v["dir"].as_str().unwrap_or("").to_string();
            let desktop = v["desktop"].as_bool().unwrap_or(true);
            let start_menu = v["startMenu"].as_bool().unwrap_or(true);
            std::thread::spawn(move || {
                let result = run_install(Path::new(&dir), desktop, start_menu, |pct, what| {
                    let boxed = Box::new(Progress {
                        pct,
                        what: what.to_string(),
                    });
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(hwnd as *mut _)),
                            WM_PROGRESS,
                            WPARAM(0),
                            LPARAM(Box::into_raw(boxed) as isize),
                        );
                    }
                });
                let boxed = Box::new(result.err());
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(hwnd as *mut _)),
                        WM_FINISHED,
                        WPARAM(0),
                        LPARAM(Box::into_raw(boxed) as isize),
                    );
                }
            });
        }
        "uninstall" => {
            let wipe = v["wipe"].as_bool().unwrap_or(false);
            std::thread::spawn(move || {
                let dir = install_dir_of_self();
                let result = run_uninstall(&dir, wipe, |pct, what| {
                    let boxed = Box::new(Progress {
                        pct,
                        what: what.to_string(),
                    });
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(hwnd as *mut _)),
                            WM_PROGRESS,
                            WPARAM(0),
                            LPARAM(Box::into_raw(boxed) as isize),
                        );
                    }
                });
                let boxed = Box::new(result.err());
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(hwnd as *mut _)),
                        WM_FINISHED,
                        WPARAM(0),
                        LPARAM(Box::into_raw(boxed) as isize),
                    );
                }
            });
        }
        _ => {}
    }
}

/* ---------- 窗口过程 ---------- */

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PROGRESS => {
            let p = Box::from_raw(lp.0 as *mut Progress);
            send_json(serde_json::json!({ "event": "progress", "pct": p.pct, "what": p.what }));
            LRESULT(0)
        }
        WM_FINISHED => {
            let result = Box::from_raw(lp.0 as *mut Option<String>);
            match &*result {
                None => send_json(serde_json::json!({ "event": "done" })),
                Some(e) => send_json(serde_json::json!({ "event": "error", "message": e })),
            }
            LRESULT(0)
        }
        WM_REVEAL => {
            let _ = ShowWindow(hwnd, SW_SHOW);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/* ---------- WebView2 ---------- */

unsafe fn init_webview(hwnd: HWND) {
    let data_dir = std::env::temp_dir().join("SupportEaseSetup");
    let _ = std::fs::create_dir_all(&data_dir);

    let handler =
        CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |result, env| {
            let env: Option<ICoreWebView2Environment> = env;
            let (Some(env), true) = (env, result.is_ok()) else {
                fallback_no_webview(hwnd);
                return Ok(());
            };

            let controller_handler =
                CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                    move |result, controller| {
                        let (Some(controller), true) = (controller, result.is_ok()) else {
                            return Ok(());
                        };
                        controller.SetIsVisible(true)?;
                        let webview = controller.CoreWebView2()?;
                        let mut rc = RECT::default();
                        GetClientRect(hwnd, &mut rc)?;
                        controller.SetBounds(rc)?;

                        let settings = webview.Settings()?;
                        settings.SetAreDefaultContextMenusEnabled(false)?;
                        settings.SetIsStatusBarEnabled(false)?;
                        settings.SetAreDevToolsEnabled(false)?;

                        let hwnd_raw = hwnd.0 as isize;
                        let received = WebMessageReceivedEventHandler::create(Box::new(
                            move |_sender, args| {
                                if let Some(args) = args {
                                    let mut raw = PWSTR::null();
                                    args.WebMessageAsJson(&mut raw)?;
                                    on_message(hwnd_raw, &take_pwstr(raw));
                                }
                                Ok(())
                            },
                        ));
                        let mut token = 0i64;
                        webview.add_WebMessageReceived(&received, &mut token)?;

                        // 页面画好再让窗口露头：先露窗口会看到一段白框
                        let reveal = NavigationCompletedEventHandler::create(Box::new(
                            move |_sender, _args| {
                                unsafe {
                                    let _ = PostMessageW(
                                        Some(hwnd),
                                        WM_REVEAL,
                                        WPARAM(0),
                                        LPARAM(0),
                                    );
                                }
                                Ok(())
                            },
                        ));
                        let mut nav_token = 0i64;
                        webview.add_NavigationCompleted(&reveal, &mut nav_token)?;

                        webview.NavigateToString(&HSTRING::from(build_html()))?;

                        WEBVIEW.with(|w| *w.borrow_mut() = Some(webview));
                        CONTROLLER.with(|c| *c.borrow_mut() = Some(controller));
                        Ok(())
                    },
                ));
            let _ = env.CreateCoreWebView2Controller(hwnd, &controller_handler);
            Ok(())
        }));

    let _ = CreateCoreWebView2EnvironmentWithOptions(
        None,
        &HSTRING::from(data_dir.to_string_lossy().as_ref()),
        None,
        &handler,
    );
}

/// 没有 WebView2（Win7/8 或运行时被卸了的机器）：弹一个系统对话框说明，
/// 然后按静默安装走完 —— 总比"点了没反应"强。
unsafe fn fallback_no_webview(hwnd: HWND) {
    let _ = MessageBoxW(
        Some(hwnd),
        w!("这台机器没有 WebView2 运行时，安装界面无法显示。\n\n将直接静默安装 SupportEase。"),
        w!("SupportEase 安装程序"),
        MB_OK | MB_ICONWARNING,
    );
    let code = match run_install(&default_install_dir(), true, true, |_, _| {}) {
        Ok(()) => 0,
        Err(_) => 1,
    };
    let _ = DestroyWindow(hwnd);
    std::process::exit(code);
}

/* ---------- 安装本体 ---------- */

#[cfg(not(feature = "uninstall-only"))]
fn run_install(
    dir: &Path,
    desktop: bool,
    start_menu: bool,
    mut report: impl FnMut(u32, &str),
) -> Result<(), String> {
    report(2, "正在准备安装目录…");
    std::fs::create_dir_all(dir).map_err(|e| format!("创建不了安装目录：{e}"))?;

    // 覆盖安装：先把还在跑的那个关掉（它正在占着 exe 文件）
    let _ = hidden("taskkill").args(["/f", "/im", APP_EXE]).output();

    report(6, "正在解压文件…");
    if PAYLOAD.is_empty() {
        return Err("安装包不完整（构建时没打入 payload）".to_string());
    }
    let mut raw = Vec::new();
    {
        use std::io::Read;
        xz2::read::XzDecoder::new(std::io::Cursor::new(PAYLOAD))
            .read_to_end(&mut raw)
            .map_err(|e| format!("安装包损坏：{e}"))?;
    }
    // 容器：[license_len][app_len][uninstaller_len][license][app][uninstaller]
    if raw.len() < 12 {
        return Err("安装包损坏：头部缺失".to_string());
    }
    let license_len = u32::from_le_bytes(raw[0..4].try_into().unwrap()) as usize;
    let app_len = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let un_len = u32::from_le_bytes(raw[8..12].try_into().unwrap()) as usize;
    let expected = 12usize
        .saturating_add(license_len)
        .saturating_add(app_len)
        .saturating_add(un_len);
    if raw.len() < expected {
        return Err("安装包损坏：长度对不上".to_string());
    }
    let app_start = 12 + license_len;
    let un_start = app_start + app_len;

    report(12, "正在写入许可证…");
    std::fs::write(dir.join("LICENSE"), &raw[12..app_start])
        .map_err(|e| format!("写 LICENSE 失败：{e}"))?;

    let exe = &raw[app_start..un_start];
    let mut file =
        std::fs::File::create(dir.join(APP_EXE)).map_err(|e| format!("写应用失败：{e}"))?;
    const CHUNK: usize = 1 << 20;
    let chunks = exe.len().div_ceil(CHUNK).max(1);
    for (i, part) in exe.chunks(CHUNK).enumerate() {
        use std::io::Write;
        file.write_all(part).map_err(|e| format!("写应用失败：{e}"))?;
        report(16 + (i as u32 * 62 / chunks as u32), "正在解压文件…");
    }
    drop(file);

    report(82, "正在写入卸载器…");
    // 放进去的是单独编的、不带 payload 的小 exe（约 0.4 MB），
    // 而不是把带 payload 的安装器整个复制过去（那会多占 7 MB 磁盘）
    std::fs::write(dir.join("uninstall.exe"), &raw[un_start..expected])
        .map_err(|e| format!("写卸载器失败：{e}"))?;

    if desktop {
        report(88, "正在创建桌面快捷方式…");
        create_shortcut(&user_desktop().join(format!("{PRODUCT}.lnk")), dir)?;
    }
    if start_menu {
        report(92, "正在创建开始菜单快捷方式…");
        create_shortcut(&start_menu_programs().join(format!("{PRODUCT}.lnk")), dir)?;
    }

    report(97, "正在写入卸载信息…");
    write_uninstall_entry(dir)?;

    report(100, "完成");
    Ok(())
}

#[cfg(feature = "uninstall-only")]
fn run_install(
    _dir: &Path,
    _desktop: bool,
    _start_menu: bool,
    _report: impl FnMut(u32, &str),
) -> Result<(), String> {
    Err("这是卸载器，不能用来安装".to_string())
}

/// 起控制台程序（taskkill / powershell）时不要新开控制台窗口。
/// 安装器本身是 GUI 进程，没有控制台可继承 —— 不挂这个标志就会闪一下黑框。
fn hidden(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(program);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd
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
    let status = hidden("powershell")
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

/// 「应用和功能」里显示的大小取自卸载项的 EstimatedSize（KB）。
/// 不写这一项时系统就空着那一栏 —— 不是它算不出来，是我们没告诉它。
fn dir_size_kb(path: &Path) -> u32 {
    fn walk(path: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        entries.flatten().fold(0u64, |sum, entry| {
            let Ok(meta) = entry.metadata() else {
                return sum;
            };
            if meta.is_dir() {
                sum + walk(&entry.path())
            } else {
                sum + meta.len()
            }
        })
    }
    (walk(path) / 1024).min(u32::MAX as u64) as u32
}

fn write_uninstall_entry(dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(UNINSTALL_KEY)
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
    // DWORD，单位 KB —— 「应用和功能」就靠它显示大小
    key.set_value("EstimatedSize", &dir_size_kb(dir)).ok();
    Ok(())
}

fn remove_uninstall_entry() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);
}

/// 卸载：删快捷方式 / 卸载项 /（可选）用户数据，然后把「删目录」交给一个临时副本 ——
/// 运行中的 exe 删不掉自己，必须先跳出安装目录。
fn run_uninstall(dir: &Path, wipe: bool, mut report: impl FnMut(u32, &str)) -> Result<(), String> {
    if is_portable_layout(dir) {
        report(4, "正在关闭 SupportEase…");
        // 绿色版的主程序叫 SupportEase.exe，进程名也就是它
        let _ = hidden("taskkill")
            .args(["/f", "/im", "SupportEase.exe"])
            .output();
        report(78, "正在安排文件清理…");
    } else {
        report(4, "正在关闭 SupportEase…");
        let _ = hidden("taskkill").args(["/f", "/im", APP_EXE]).output();

        report(18, "正在删除快捷方式…");
        let _ = std::fs::remove_file(user_desktop().join(format!("{PRODUCT}.lnk")));
        let _ = std::fs::remove_file(start_menu_programs().join(format!("{PRODUCT}.lnk")));

        report(32, "正在移除卸载信息…");
        remove_uninstall_entry();

        if wipe {
            report(46, "正在删除预设与校准记录…");
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let _ = std::fs::remove_dir_all(PathBuf::from(appdata).join(PRODUCT));
            }
        }
        report(78, "正在安排文件清理…");
    }
    let me = std::env::current_exe().map_err(|e| format!("找不到卸载器自身：{e}"))?;
    let helper = std::env::temp_dir().join(format!(
        "SupportEaseUninstall_{}.exe",
        std::process::id()
    ));
    std::fs::copy(&me, &helper).map_err(|e| format!("写清理程序失败：{e}"))?;
    Command::new(&helper)
        .arg("--cleanup")
        .arg(dir)
        .arg("--wait")
        .arg(std::process::id().to_string())
        .spawn()
        .map_err(|e| format!("启动清理程序失败：{e}"))?;

    report(100, "完成");
    Ok(())
}

/// 临时副本模式：等主进程退出 → 删掉整个安装目录 → 安排重启时删除自己
fn cleanup_mode(dir: &Path, wait_pid: Option<u32>) {
    if let Some(pid) = wait_pid {
        unsafe { wait_for_process(pid) };
    }
    std::thread::sleep(std::time::Duration::from_millis(400));

    // ★ Windows 不允许删除进程的当前工作目录。
    //   用户从安装目录里双击 uninstall.exe 时，副本继承的那个 cwd 就是安装目录，
    //   remove_dir_all 会悄悄失败 —— 表现为"东西都删了，文件夹还在"。
    //   （老 CMD 版里那句 cd /d "%SystemRoot%" 就是在解决同一件事。）
    let _ = std::env::set_current_dir(std::env::temp_dir());

    // 绿色版的 app-data/EBWebView 带 WebView2 自己加的 Everyone:(DENY)(X)，
    // 直接 remove_dir_all 会因拒绝访问而留下残骸。照搬老 CMD 版那两句：
    // takeown 收回所有权 + icacls /reset 去掉那条 DENY，然后再删。
    let appdata = dir.join("app-data");
    if appdata.is_dir() {
        let p = appdata.to_string_lossy().into_owned();
        let _ = hidden("takeown").args(["/f", &p, "/r", "/d", "y"]).output();
        let _ = hidden("icacls")
            .args([&p, "/reset", "/t", "/c", "/q"])
            .output();
    }

    for _ in 0..5 {
        if std::fs::remove_dir_all(dir).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    unsafe { schedule_self_delete() };
}

unsafe fn wait_for_process(pid: u32) {
    if let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
        let _ = WaitForSingleObject(handle, INFINITE);
        let _ = CloseHandle(handle);
    }
}

unsafe fn schedule_self_delete() {
    use std::os::windows::ffi::OsStrExt;

    let Ok(me) = std::env::current_exe() else {
        return;
    };
    let wide: Vec<u16> = me
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let _ = MoveFileExW(
        PCWSTR(wide.as_ptr()),
        PCWSTR::null(),
        MOVEFILE_DELAY_UNTIL_REBOOT,
    );
}

/// 选文件夹：系统的 IFileOpenDialog（Vista 起的现代文件夹选择器）。
///
/// 不走 PowerShell：那条路会弹一个终端窗口、启动还要半秒，用户等不及会连点，
/// 于是同时开好几个 PowerShell —— 看起来就是"卡住"。
unsafe fn pick_folder(hwnd: HWND, _current: &str) -> Option<String> {
    let dialog: IFileOpenDialog =
        CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
    let options = dialog.GetOptions().ok()?;
    dialog
        .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)
        .ok()?;
    dialog
        .SetTitle(&HSTRING::from("选择 SupportEase 的安装文件夹"))
        .ok()?;
    if dialog.Show(Some(hwnd)).is_err() {
        return None;
    }
    let item = dialog.GetResult().ok()?;
    let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
    let text = name.to_string().ok();
    CoTaskMemFree(Some(name.0 as *const _));
    text
}

/* ---------- 入口 ---------- */

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 临时副本：删完安装目录就退，不进消息循环
    if let Some(dir) = arg_value(&args, "--cleanup") {
        let pid = arg_value(&args, "--wait").and_then(|s| s.parse::<u32>().ok());
        cleanup_mode(Path::new(&dir), pid);
        return;
    }

    // 双击 uninstall.exe 时命令行里不会有 --uninstall：文件名也得认，
    // 否则那颗卸载会当成安装器再装一遍（用户看到的就是"还是安装程序"）。
    let self_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()))
        .unwrap_or_default();
    let uninstall = cfg!(feature = "uninstall-only")
        || args.iter().any(|a| a == "--uninstall")
        || self_name.starts_with("uninstall")
        // 绿色版里的文件名是「卸载 SupportEase.exe」，双击时同样没有参数
        || self_name.starts_with("卸载");
    let _ = UNINSTALL.set(uninstall);
    let silent = args.iter().any(|a| a == "--silent" || a == "/S");

    // 安装器与卸载器各用各的窗口类名：开着安装器时点卸载器不该被拦
    let class_name = if uninstall {
        w!("SupportEaseUninstall")
    } else {
        w!("SupportEaseSetup")
    };

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        // 单实例：按窗口类名判断已经开着一个没有 —— 找到就把那个唤到前台，
        // 不开第二个，也不能"点了没反应"。
        // （不用命名互斥量：实测残留句柄会让新进程误判成"已存在"，直接退出。）
        if let Ok(existing) = FindWindowW(class_name, PCWSTR::null()) {
            let _ = ShowWindow(existing, SW_RESTORE);
            let _ = PostMessageW(Some(existing), WM_REVEAL, WPARAM(0), LPARAM(0));
            let _ = SetForegroundWindow(existing);
            return;
        }

        if silent {
            let result = if uninstall {
                run_uninstall(&install_dir_of_self(), false, |_, _| {})
            } else {
                run_install(&silent_install_dir(), true, true, |_, _| {})
            };
            let code = match result {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("静默{}失败：{e}", if uninstall { "卸载" } else { "安装" });
                    1
                }
            };
            std::process::exit(code);
        }

        let hinstance = GetModuleHandleW(None).expect("拿不到模块句柄");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wndproc),
            hInstance: HINSTANCE(hinstance.0),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: LoadIconW(Some(HINSTANCE(hinstance.0)), PCWSTR(1 as *const u16))
                .unwrap_or_default(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let dpi = GetDpiForSystem().max(96) as i32;
        let width = 540 * dpi / 96;
        let height = 430 * dpi / 96;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        let title = if uninstall {
            w!("SupportEase 卸载程序")
        } else {
            w!("SupportEase 安装程序")
        };
        let hwnd = match CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            title,
            WS_POPUP | WS_CLIPCHILDREN,
            (screen_w - width) / 2,
            (screen_h - height) / 2,
            width,
            height,
            None,
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                let msg = format!("CreateWindowExW 失败：{e:?}");
                let _ = std::fs::write(std::env::temp_dir().join("mkp-setup-fail.txt"), &msg);
                let _ = MessageBoxW(None, &HSTRING::from(msg), w!("SupportEase"), MB_OK);
                std::process::exit(1);
            }
        };

        // Win11 圆角（Win10 上这条会失败，窗口保持方角）
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const _,
            std::mem::size_of_val(&preference) as u32,
        );

        init_webview(hwnd);

        // 兜底：WebView2 万一没起来，3 秒后也要把窗口露出来
        let raw = hwnd.0 as isize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = PostMessageW(
                Some(HWND(raw as *mut _)),
                WM_REVEAL,
                WPARAM(0),
                LPARAM(0),
            );
        });

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
