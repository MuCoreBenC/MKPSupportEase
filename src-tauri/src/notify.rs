//! **系统通知**（2026-10-17，设置页那一格「处理完成发送系统通知」）。
//!
//! # 为什么不加 tauri-plugin-notification
//!
//! 那条插件会把 `notification:default` 这套 capability 拉进来，而本项目的 capability
//! 是**按"谁能调什么"手写的**（`capabilities/default.json`）—— 加一个插件就意味着
//! 多一份要维护的权限面，而它做的事只是"弹一条系统横幅"。
//!
//! 这里走**系统自带的东西**：macOS 走原生 `NSUserNotification`（失败退回 `osascript`）、
//! Windows 走 PowerShell + WinRT Toast。好处是零新依赖、零权限面。
//!
//! # 图标（2026-11-18 作者截图：「我的通知没有图标」）
//!
//! 系统通知的图标**不是正文里的图片**，是"这条通知属于哪个应用"那个身份：
//!
//! - **Windows**：不走身份就没有图标（显示一个灰色默认块）。所以这里做两件事：
//!   ① 把内嵌的那张 PNG 落到 `run/notify-icon.png`，并在
//!      `HKCU\Software\Classes\AppUserModelId\SupportEase` 登记 `DisplayName` + `IconUri`；
//!   ② toast 用 `ToastGeneric` 模板，再挂一张 `appLogoOverride` 图（圆形裁切）。
//!   两处都做是因为：①决定通知中心里的应用身份，②决定横幅左上角那张图 —— 旧机器上
//!   只有 ② 也看得见图。
//! - **macOS**：`osascript` 发出去的通知算 **osascript/Script Editor 的**，图标永远是脚本
//!   编辑器那个。要让通知算在我们头上，必须**从本进程**发（`NSUserNotification`），
//!   系统才拿 `.app` 的图标。所以那条原生路优先，`osascript` 只当兜底。
//!
//! # 失败怎么办
//!
//! **不发出去也不吵人**：通知是锦上添花，没发出去不该让一趟已经成功的后处理
//! 变成"有问题"。所以一律 `let _ =`，只留一条 WARN 日志 ——
//! 用户真要查，日志里能看到是哪一步没成。这与其他"提示"的口径不同（那些必须说出来），
//! 因为那些是**用户主动等的结果**，通知不是。
//!
//! # 只在两个平台上真的做
//!
//! Linux 上不弹（没有统一入口，`notify-send` 未必装）。这与 `chrome.rs` 那套
//! "平台差异就地写明"的口径一致 —— 宁可不说，也不假装弹了。

/// 发一条系统通知：`title` 是应用名那一行，`body` 是正文。
///
/// 调用方负责判断"这一趟该不该发"（`notify_on_finish` + 跑成功了），这里只管发。
pub fn notify(title: &str, body: &str) {
    if let Err(e) = try_notify(title, body) {
        /* 失败只记日志：见模块头"失败怎么办" */
        tracing::warn!(error = %e, "系统通知没发出去（不影响这一趟的结果）");
    }
}

/* ---------- macOS：原生优先，osascript 兜底 ---------- */

#[cfg(target_os = "macos")]
fn try_notify(title: &str, body: &str) -> Result<(), String> {
    if deliver_native(title, body) {
        return Ok(());
    }
    /* 兜底：`display notification` 是 osascript 的标准说法；两段文字都要转义双引号反斜杠。
    ★ 这条路发出去的图标是 Script Editor 的 —— 只在原生那条走不通时才用。 */
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape(body),
        escape(title)
    );
    let out = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| {
            ::postprocess::diag::messages::text(
                "ui.notify.why_osascript",
                &[("detail", &e.to_string())],
            )
        })?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// 从**本进程**发一条通知 —— 这样系统才把通知算在 SupportEase 头上（带它的图标）。
///
/// `NSUserNotification` 是弃用 API（10.14 起建议 `UNUserNotificationCenter`），但它
/// 不需要授权弹窗、也不需要 bundle 里登记 `NSUserNotificationAlertStyle`；对"跑完弹一句"
/// 够用。走 `msg_send` 而不是类型化绑定，是为了不吃 `objc2-foundation` 的 feature 开关
/// （那个开关一改，macOS 那侧就编不过，而 CI 里没有 macOS）。
///
/// 返回 `false` = 这条路没走通（类不在 / 拿不到通知中心），调用方退回 `osascript`。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)] /* 四五个 msg_send，全是定长消息，见下面 SAFETY */
fn deliver_native(title: &str, body: &str) -> bool {
    use objc2::msg_send;
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSString;

    let (Some(note_cls), Some(center_cls)) = (
        AnyClass::get(c"NSUserNotification"),
        AnyClass::get(c"NSUserNotificationCenter"),
    ) else {
        return false;
    };

    // SAFETY: 全是 AppKit/Foundation 的定长消息：`alloc`/`init` 拿一个新的通知对象（+1），
    // 三个 setter 只传本帧里活得比调用久的 `NSString`，最后交给通知中心并 `release` 掉
    // 我们自己那份 +1。类名取不到时上面已经提前返回，不会把空指针传进消息。
    unsafe {
        /*
         * ★ `alloc` / `init` 的返回类型必须写出来（2026-11-18 CI 在 macOS 上红的那处）：
         *   `msg_send!` 推不出这条消息的所有权语义（`E0283`），`objc2` 0.6 的官方写法是
         *   `Allocated<T>` 接 `alloc`、`Retained<T>` 接 `init`。取回裸指针之后仍是我们自己
         *   那一份 +1、结尾自己 `release`。
         */
        let alloc: Allocated<AnyObject> = msg_send![note_cls, alloc];
        let at: Retained<AnyObject> = msg_send![alloc, init];
        let note: *mut AnyObject = Retained::into_raw(at);
        if note.is_null() {
            return false;
        }
        let t = NSString::from_str(title);
        let b = NSString::from_str(body);
        let _: () = msg_send![note, setTitle: &*t];
        let _: () = msg_send![note, setInformativeText: &*b];

        let center: *mut AnyObject = msg_send![center_cls, defaultUserNotificationCenter];
        if center.is_null() {
            let _: () = msg_send![note, release];
            return false;
        }
        let _: () = msg_send![center, deliverNotification: note];
        /* 中心自己留一份，我们这份 +1 还掉 */
        let _: () = msg_send![note, release];
    }
    true
}

/* ---------- Windows：ToastGeneric + 应用身份 ---------- */

#[cfg(target_os = "windows")]
fn try_notify(title: &str, body: &str) -> Result<(), String> {
    use crate::quiet::Quiet as _;

    /* ① 图标落盘 + ② 登记应用身份：没这两步，横幅左上角是系统给的灰块 */
    let icon = write_icon();
    register_app_identity(icon.as_deref());

    let logo = match icon.as_deref() {
        Some(path) => format!(
            "<image placement=\"appLogoOverride\" hint-crop=\"circle\" src=\"{}\"/>",
            file_uri(path)
        ),
        None => String::new(),
    };
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text>{logo}</binding></visual></toast>",
        xml_escape(title),
        xml_escape(body)
    );

    /* ★ 两个 WinRT 类型都要先"点"一下才认（`New-Object` 直接写类型名会报
    "Cannot find type [Windows.Data.Xml.Dom.XmlDocument]" —— 2026-11-18 踩过：
    那次改完通知一条都不弹，正是少了 XmlDocument 这一行）。 */
    let script = format!(
        "[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null; \
         [Windows.UI.Notifications.ToastNotification, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
         $d = New-Object Windows.Data.Xml.Dom.XmlDocument; \
         $d.LoadXml('{}'); \
         [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{}').Show([Windows.UI.Notifications.ToastNotification]::new($d))",
        ps_escape(&xml),
        APP_ID
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        /* ★ 不弹黑框（2026-11-18 作者截图）：powershell 是控制台程序，
        不挂 CREATE_NO_WINDOW 就会在旁边新开一扇控制台窗口。 */
        .quiet()
        .output()
        .map_err(|e| {
            ::postprocess::diag::messages::text(
                "ui.notify.why_powershell",
                &[("detail", &e.to_string())],
            )
        })?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// 通知的**应用身份**（AppUserModelID）。Windows 按这个字符串决定"这条通知算谁的"，
/// 图标、通知中心里的分组、点开之后聚焦谁，全看它。**不许随手改** —— 改了等于换一个应用。
#[cfg(target_os = "windows")]
const APP_ID: &str = "SupportEase";

/// 内嵌的那张图（128×128，与安装包装的是同一张）。
#[cfg(target_os = "windows")]
const ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

/// 把图标落到 `run/notify-icon.png` 并返回它；算不出数据根 / 写不进去 ⇒ `None`
/// （那就只发文字通知，不为了图标把通知整个吞掉）。
///
/// 每次都写一遍而不是"有就跳过"：图标将来换了，这里不用再记一个版本号。
/// 写用的是全仓唯一写盘出口，与别处同一条规矩。
#[cfg(target_os = "windows")]
fn write_icon() -> Option<std::path::PathBuf> {
    let root = crate::fsx::paths::internal_root_headless()?;
    let dir = root.join("run");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("notify-icon.png");
    crate::fsx::atomic::atomic_write(&path, ICON_PNG).ok()?;
    Some(path)
}

/// 登记应用身份：`HKCU\Software\Classes\AppUserModelId\SupportEase`
/// → `DisplayName`（通知里显示的名字）+ `IconUri`（那张 PNG 的路径）。
///
/// 只写 `HKCU`（当前用户，不需要管理员）；写不进去就算了 —— 横幅里那张图由
/// `appLogoOverride` 顶着，只是通知中心里的身份会缺图标。
#[cfg(target_os = "windows")]
fn register_app_identity(icon: Option<&std::path::Path>) {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))
    else {
        return;
    };
    let (key, _) = key;
    let _ = key.set_value("DisplayName", &"SupportEase");
    if let Some(path) = icon {
        let _ = key.set_value("IconUri", &path.display().to_string());
    }
}

/// 本地路径 → `file:///C:/...`（toast 的 `src` 只认 URI；反斜杠要换成斜杠）。
#[cfg(target_os = "windows")]
fn file_uri(path: &std::path::Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    format!("file:///{}", text.trim_start_matches('/'))
}

/* ---------- 其它平台 ---------- */

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn try_notify(_title: &str, _body: &str) -> Result<(), String> {
    Err(::postprocess::diag::messages::text(
        "ui.notify.unimplemented",
        &[],
    ))
}

/* ---------- 转义 ---------- */

/// AppleScript 字符串转义：`\` 与 `"` 是那里唯一会坏事的两样。
#[cfg(target_os = "macos")]
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// PowerShell 单引号字符串转义：里面出现单引号就写两个。
#[cfg(any(target_os = "windows", test))]
fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

/// XML 文本转义：标题/正文里出现 `&` `<` `>` 就会把 toast 的 XML 弄坏。
#[cfg(any(target_os = "windows", test))]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    /* 转义是这一段唯一有判断的地方，其余就是起进程。 */
    #[cfg(target_os = "macos")]
    #[test]
    fn applescript_escaping_keeps_quotes_inside() {
        use super::escape;
        assert_eq!(escape(r#"他说"好"#), r#"他说\"好"#);
        assert_eq!(escape(r"a\b"), r"a\\b");
    }

    #[test]
    fn powershell_escaping_doubles_quotes() {
        use super::ps_escape;
        assert_eq!(ps_escape("it's"), "it''s");
    }

    #[test]
    fn xml_escaping_keeps_the_toast_well_formed() {
        use super::xml_escape;
        assert_eq!(xml_escape("A & B <C>"), "A &amp; B &lt;C&gt;");
    }

    /// 图标那一行只能是本地 URI：反斜杠没换掉，Windows 就不认这个 `src`，图还是不出。
    #[cfg(target_os = "windows")]
    #[test]
    fn the_icon_uri_is_a_file_uri() {
        use super::file_uri;
        let p = std::path::Path::new(r"C:\Users\x\AppData\Roaming\SupportEase\run\notify-icon.png");
        assert_eq!(
            file_uri(p),
            "file:///C:/Users/x/AppData/Roaming/SupportEase/run/notify-icon.png"
        );
    }
}
