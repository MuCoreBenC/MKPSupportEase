//! **这台机器有没有 WebView2 运行时** —— 绿色版（单 exe / zip 解压即用）的第一道门。
//!
//! # 为什么要我们自己问
//!
//! Tauri 在 Windows 上的界面内核就是系统的 WebView2 运行时。安装包（NSIS）会把微软的
//! bootstrapper 一起打进去、装的时候替你装；**绿色版没有这一层** —— 用户拿到的只有一个
//! exe，机器上没那个运行时的话，`tauri::Builder::run` 在建窗那一步就失败，用户看到的是
//! "双击了、什么都没发生"。这里在**建窗之前**先问一句，缺了就用系统消息框说清楚，
//! 直接把他送到微软的下载页（装一次，以后不用再装）。
//!
//! # 判据为什么是注册表
//!
//! 微软自己给的那条判据：`EdgeUpdate\Clients\{F3017226-…}\pv` 有值 = 装上了。
//! 四个位置都要看（每用户 / 全机器 × 64 位 / 32 位视图），装它的人四个里挑一个写。
//! 卸载会留下 `pv = "0.0.0.0"` 的占位 —— 那不算装上，见 [`version_counts`]。
//!
//! ★ 只用**读**：不写注册表、不自己装东西（作者 2026-11-18：「我才不要给用户提供
//!   webview2 的，弹一个提示让他自己去下载就可以了」）。

/// 微软文档里那个固定的产品 ID（WebView2 Runtime 的 EdgeUpdate client），64 位视图。
const RUNTIME_CLIENT: &str =
    r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// 同一个 client，32 位视图（32 位安装器只写这一份）。
const RUNTIME_CLIENT_WOW: &str =
    r"Software\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// 微软的下载页（**不是** bootstrapper 直链：页面上能自己选 x64 / arm64 与常青版）。
const DOWNLOAD_PAGE: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

/// `pv` 这一格算不算"装上了"。
///
/// 空串 = 键在但没版本（半装状态）；`0.0.0.0` = 卸载之后留下的占位（微软的卸载器会这么写）。
/// 两种都不算 —— 真拿这种机器去建窗，照样是"什么都没发生"。
fn version_counts(pv: &str) -> bool {
    let v = pv.trim();
    !v.is_empty() && v != "0.0.0.0"
}

/// 这台机器**缺** WebView2 运行时吗。
#[cfg(target_os = "windows")]
pub fn missing() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let root = RegKey::predef(hive);
        for view in [RUNTIME_CLIENT, RUNTIME_CLIENT_WOW] {
            let Ok(key) = root.open_subkey(view) else {
                continue;
            };
            if let Ok(pv) = key.get_value::<String, _>("pv") {
                if version_counts(&pv) {
                    return false;
                }
            }
        }
    }
    true
}

/// 弹一句「要装 WebView2」，用户点「确定」就打开下载页，点「取消」什么都不做。
///
/// 返回 `true` = 用户要下载（调用方随即退出）。**这里不建任何 Tauri 窗口** ——
/// 这一趟就是来报"建不了窗"的，再去建窗就成了同一个死循环。
#[cfg(target_os = "windows")]
#[allow(unsafe_code)] /* 两个 user32 / shell32 的声明，与 `main.rs::hide_own_console` 同一条理由 */
pub fn prompt_download() -> bool {
    use std::ffi::c_void;

    unsafe extern "system" {
        /// 系统消息框（`user32`）。返回用户点了哪个按钮。
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
        /// 用默认浏览器打开一个链接（`shell32`）。返回值 > 32 才算成功，这里不看。
        fn ShellExecuteW(
            hwnd: *mut c_void,
            op: *const u16,
            file: *const u16,
            params: *const u16,
            dir: *const u16,
            show: i32,
        ) -> *mut c_void;
    }

    const MB_OKCANCEL: u32 = 0x0000_0001;
    const MB_ICONWARNING: u32 = 0x0000_0030;
    const MB_SETFOREGROUND: u32 = 0x0001_0000;
    const IDOK: i32 = 1;
    const SW_SHOWNORMAL: i32 = 1;

    /// Rust 的 `&str` → 以 0 结尾的 UTF-16（Win32 的 W 接口要这个）
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let text = wide(
        "SupportEase 需要系统的 WebView2 运行时才能显示界面，这台机器上还没有装。\n\n\
         点「确定」打开微软的下载页（装一次即可，装完再双击 SupportEase）；\n\
         点「取消」退出。\n\n\
         SupportEase needs the Microsoft WebView2 Runtime to show its window.\n\
         Click OK to open the download page, or Cancel to quit.",
    );
    let caption = wide("SupportEase");

    // SAFETY: 两个调用都只读本函数栈上的宽字符串；窗口句柄传空 = 无父窗（系统消息框允许），
    // 并且不把任何裸指针带出这一帧。
    unsafe {
        let picked = MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OKCANCEL | MB_ICONWARNING | MB_SETFOREGROUND,
        );
        if picked != IDOK {
            return false;
        }
        let op = wide("open");
        let url = wide(DOWNLOAD_PAGE);
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 卸载留下的占位与"键在没版本"都不算装上 —— 否则这类机器会跳过提示、直接"双击没反应"。
    #[test]
    fn only_a_real_version_counts_as_installed() {
        assert!(version_counts("120.0.2210.144"));
        assert!(version_counts(" 120.0.2210.144 "), "两头空白不影响");
        assert!(!version_counts(""), "键在但没版本");
        assert!(!version_counts("   "));
        assert!(!version_counts("0.0.0.0"), "卸载留下的占位");
    }
}
