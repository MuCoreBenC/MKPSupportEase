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
    CreateCoreWebView2EnvironmentCompletedHandler, WebMessageReceivedEventHandler,
};
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
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
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.xz"));

const PRODUCT: &str = "SupportEase";
const APP_EXE: &str = "mkp-support-ease.exe";
const PUBLISHER: &str = "MuCoreBenC";

const WM_PROGRESS: u32 = WM_APP + 1;
const WM_FINISHED: u32 = WM_APP + 2;

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
        })),
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
        "browse" => {
            let current = v["current"].as_str().unwrap_or("").to_string();
            if let Some(path) = pick_dir(&current) {
                send_json(serde_json::json!({ "event": "dir", "path": path }));
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

fn run_install(
    dir: &Path,
    desktop: bool,
    start_menu: bool,
    mut report: impl FnMut(u32, &str),
) -> Result<(), String> {
    report(2, "正在准备安装目录…");
    std::fs::create_dir_all(dir).map_err(|e| format!("创建不了安装目录：{e}"))?;

    // 覆盖安装：先把还在跑的那个关掉（它正在占着 exe 文件）
    let _ = Command::new("taskkill").args(["/f", "/im", APP_EXE]).output();

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
    if raw.len() < 8 {
        return Err("安装包损坏：头部缺失".to_string());
    }
    let license_len = u32::from_le_bytes(raw[0..4].try_into().unwrap()) as usize;
    let exe_len = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    if raw.len() < 8 + license_len + exe_len {
        return Err("安装包损坏：长度对不上".to_string());
    }

    report(12, "正在写入许可证…");
    std::fs::write(dir.join("LICENSE"), &raw[8..8 + license_len])
        .map_err(|e| format!("写 LICENSE 失败：{e}"))?;

    let exe = &raw[8 + license_len..8 + license_len + exe_len];
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

    report(82, "正在写入卸载信息…");
    let me = std::env::current_exe().map_err(|e| format!("找不到安装器自身：{e}"))?;
    std::fs::copy(&me, dir.join("uninstall.exe")).map_err(|e| format!("写卸载器失败：{e}"))?;

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
    let status = Command::new("powershell")
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

fn remove_uninstall_entry() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{PRODUCT}");
    let _ = hkcu.delete_subkey_all(path);
}

/// 卸载：删快捷方式 / 卸载项 /（可选）用户数据，然后把「删目录」交给一个临时副本 ——
/// 运行中的 exe 删不掉自己，必须先跳出安装目录。
fn run_uninstall(dir: &Path, wipe: bool, mut report: impl FnMut(u32, &str)) -> Result<(), String> {
    report(4, "正在关闭 SupportEase…");
    let _ = Command::new("taskkill").args(["/f", "/im", APP_EXE]).output();

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
    let _ = std::fs::remove_dir_all(dir);
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

/// 选文件夹：走系统的 FolderBrowserDialog（PowerShell 一句）。
/// 它是模态的，会短暂卡住消息循环 —— 与原生对话框的行为一致。
fn pick_dir(current: &str) -> Option<String> {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms;\
         $d=New-Object System.Windows.Forms.FolderBrowserDialog;\
         $d.SelectedPath='{}';\
         if($d.ShowDialog() -eq 'OK'){{$d.SelectedPath}}",
        current.replace('\'', "''")
    );
    let out = Command::new("powershell")
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

    let uninstall = args.iter().any(|a| a == "--uninstall");
    let _ = UNINSTALL.set(uninstall);
    let silent = args.iter().any(|a| a == "--silent" || a == "/S");

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        if silent {
            let result = if uninstall {
                run_uninstall(&install_dir_of_self(), false, |_, _| {})
            } else {
                run_install(&default_install_dir(), true, true, |_, _| {})
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
        let class = w!("SupportEaseSetup");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wndproc),
            hInstance: HINSTANCE(hinstance.0),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: LoadIconW(Some(HINSTANCE(hinstance.0)), PCWSTR(1 as *const u16))
                .unwrap_or_default(),
            lpszClassName: class,
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
            class,
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
            Err(_) => std::process::exit(1),
        };

        // Win11 圆角（Win10 上这条会失败，窗口保持方角）
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const _,
            std::mem::size_of_val(&preference) as u32,
        );

        let _ = ShowWindow(hwnd, SW_SHOW);
        init_webview(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
