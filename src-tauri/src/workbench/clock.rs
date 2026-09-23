//! 时间戳。只有两种格式，各有其用途。
//!
//! 为什么单独一个文件：时间格式是那种"每处随手写一遍、最后每处都不一样"的东西。
//! 快照里记的时间与回收站文件名里的时间必须能对上，所以来源只有一个。

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::OffsetDateTime;

/// 给人看、也给 JSON 用的时间。UTC，秒级，带 `Z` 后缀。
///
/// 刻意不取本地时区：这些时间戳会随产物进 git、进发布数据，跨时区看同一个值必须是
/// 同一个时刻。界面上要显示本地时间由前端去转。
///
/// **刻意砍掉亚秒部分**：RFC3339 默认带纳秒，而这些时间戳会写进入库的 JSON ——
/// 微秒级精度在这里没有任何用处，只会让 diff 更吵。
pub fn now_iso8601() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

/// 给文件名用的时间。`20260922T143102Z` —— 不含 `:`，Windows 上冒号不能进文件名
pub fn now_stamp() -> String {
    let fmt = format_description!("[year][month][day]T[hour][minute][second]Z");
    OffsetDateTime::now_utc()
        .format(fmt)
        .unwrap_or_else(|_| "19700101T000000Z".into())
}

/// **产物头部那一行 `# release_time:` 专用**：`2026-08-19 01:38:13`。
///
/// 空格分隔、无时区后缀 —— 这不是我们的偏好，是消费端已有的 9 份预设都长这样
/// （b04 P0 审计 §3）。产物是给它读的，所以格式跟着它。
///
/// 为什么单独一个函数而不是在拼接处 `format!`：这个格式**只允许出现在一处**。
/// 随手写一遍的后果是两个地方各写一种，然后"为什么这份产物的时间格式不一样"
/// 变成一个查半天的问题。
///
/// 仍然是 UTC：这些时间戳会随产物进 git、进发布数据，跨时区看必须是同一个时刻
pub fn now_release_time() -> String {
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    OffsetDateTime::now_utc()
        .format(fmt)
        .unwrap_or_else(|_| "1970-01-01 00:00:00".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_looks_like_rfc3339_utc() {
        let s = now_iso8601();
        assert!(s.ends_with('Z'), "不是 UTC：{s}");
        assert_eq!(s.len(), 20, "长度不对（应为 2026-09-22T14:31:02Z 这种）：{s}");
    }

    /// 文件名用的那份绝不能含冒号 —— 这条不是风格，是 Windows 上能不能落盘
    #[test]
    fn stamp_is_filename_safe() {
        let s = now_stamp();
        for bad in [':', '/', '\\', '*', '?', '"', '<', '>', '|'] {
            assert!(!s.contains(bad), "{s} 含非法字符 {bad}");
        }
        assert_eq!(s.len(), 16, "长度不对：{s}");
    }

    /// 产物头部那份：**空格分隔、无时区后缀**，形如 `2026-08-19 01:38:13`。
    ///
    /// 消费端已有的 9 份预设都是这个格式，产物是给它读的。
    /// 写成 RFC3339（带 `T` 和 `Z`）就是另一种格式了 —— 这条判据盯的就是那件事
    #[test]
    fn release_time_matches_what_the_consumer_writes() {
        let s = now_release_time();
        assert_eq!(s.len(), 19, "长度不对（应为 2026-08-19 01:38:13 这种）：{s}");
        assert!(!s.contains('T') && !s.contains('Z'), "不该带 T/Z：{s}");
        assert_eq!(s.as_bytes()[10], b' ', "第 11 位该是空格：{s}");
        // 与另外两种格式都不一样 —— 三者混用是这个文件存在的理由
        assert_ne!(s, now_iso8601());
        assert_ne!(s, now_stamp());
    }
}
