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
}
