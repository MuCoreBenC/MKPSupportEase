//! **系统通知**（2026-10-17，设置页那一格「处理完成发送系统通知」）。
//!
//! # 为什么不加 tauri-plugin-notification
//!
//! 那条插件会把 `notification:default` 这套 capability 拉进来，而本项目的 capability
//! 是**按"谁能调什么"手写的**（`capabilities/default.json`）—— 加一个插件就意味着
//! 多一份要维护的权限面，而它做的事只是"弹一条系统横幅"。
//!
//! 这里走**系统自带的命令行**：macOS `osascript`、Windows PowerShell。
//! 好处是零依赖、零权限面；代价是"点通知之后做什么"我们控制不了 ——
//! 系统会把焦点交给我们的进程（macOS 会激活 app），而我们的窗口自然浮上来。
//! 这正好对得上作者文案里的「点击通知可打开处理结果」。
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

fn try_notify(title: &str, body: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        /* `display notification` 是 osascript 的标准说法；两段文字都要转义双引号反斜杠 */
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape(body),
            escape(title)
        );
        let out = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("osascript 起不来：{e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
        }
    }

    #[cfg(target_os = "windows")]
    {
        /* ToastNotificationManager 要 AppUserModelID（快捷方式的身份）；
        这里退一步走 balloon tip 那条老路 —— 视觉上一样是横幅，且不需要注册身份 */
        let script = format!(
            "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
             $t = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
             $t.GetElementsByTagName('text').Item(0).AppendChild($t.CreateTextNode('{}')) > $null; \
             $t.GetElementsByTagName('text').Item(1).AppendChild($t.CreateTextNode('{}')) > $null; \
             $n = [Windows.UI.Notifications.ToastNotification]::new($t); \
             [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('MKP SupportEase').Show($n)",
            ps_escape(title),
            ps_escape(body)
        );
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("powershell 起不来：{e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, body);
        Err("这个平台上没有实现系统通知".to_owned())
    }
}

/// AppleScript 字符串转义：`\` 与 `"` 是那里唯一会坏事的两样。
#[cfg(target_os = "macos")]
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// PowerShell 单引号字符串转义：里面出现单引号就写两个。
#[cfg(target_os = "windows")]
fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    /* 转义是这一段唯一有判断的地方，其余就是起进程。两个平台上各测各的。 */
    #[cfg(target_os = "macos")]
    #[test]
    fn applescript_escaping_keeps_quotes_inside() {
        use super::escape;
        assert_eq!(escape(r#"他说"好"#), r#"他说\"好"#);
        assert_eq!(escape(r"a\b"), r"a\\b");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_escaping_doubles_quotes() {
        use super::ps_escape;
        assert_eq!(ps_escape("it's"), "it''s");
    }
}
