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

/// **盘上时间戳 → RFC3339 / UTC（秒级）**。文件创建 / 修改时间、目录上次重算时间用它。
///
/// 与 [`now_iso8601`] 同一个口径（UTC、秒级、带 `Z`）：界面上要显示本地时间由前端转，
/// 后端不猜时区 —— 多线程进程里 `current_local_offset()` 可能解析不出来，
/// 那一次回落会静默差几小时。
pub fn iso8601_of(t: std::time::SystemTime) -> String {
    let odt = OffsetDateTime::from(t);
    odt.replace_nanosecond(0)
        .unwrap_or(odt)
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

/* `now_release_time()` 已于 2026-11-01 删除（它写死 UTC、无偏移，而且被当成
"渲染时刻"塞进产物）。产物头现在的 `# generated_at:` 走 [`now_local_rfc3339`]。 */

/// 产物头 `# generated_at:` 用：**本地时间 + UTC 偏移**，形如
/// `2026-11-01T12:30:00+08:00`。
///
/// 为什么带偏移（2026-11-01 作者裁定）：绝对时刻明确、本地可读。没有偏移的
/// `2026-11-01 12:30:00` 换个时区看就是另一件事。偏移取**系统当前时区**
/// （不是写死 +08:00）—— 东京给 +09:00，有夏令时的地区给当时的偏移。
///
/// 这些时刻会随产物进 git、进发布数据，跨时区看的必须是同一个瞬间；
/// 偏移只是"在哪里看"的说明，不影响它指向哪一刻。
pub fn now_local_rfc3339() -> String {
    let fmt = format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
    );
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(fmt)
        .unwrap_or_else(|_| "1970-01-01T00:00:00+00:00".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_looks_like_rfc3339_utc() {
        let s = now_iso8601();
        assert!(s.ends_with('Z'), "不是 UTC：{s}");
        assert_eq!(
            s.len(),
            20,
            "长度不对（应为 2026-09-22T14:31:02Z 这种）：{s}"
        );
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

    /// 产物头 `# generated_at:` 用的格式：**ISO 8601 + 明确偏移**。
    ///
    /// 必须带偏移（`+08:00` / `+09:00` / `-05:00`）：没有偏移的时间放在另一个
    /// 时区看就是另一件事。偏移取系统时区，不写死。
    #[test]
    fn generated_at_carries_an_explicit_utc_offset() {
        let s = now_local_rfc3339();
        assert_eq!(s.len(), 25, "应为 2026-11-01T12:30:00+08:00 这种：{s}");
        assert_eq!(s.as_bytes()[10], b'T', "日期与时间之间是 T：{s}");
        let (_, sign) = s.split_at(19);
        assert!(
            sign.starts_with('+') || sign.starts_with('-'),
            "必须带 +/- 偏移：{s}"
        );
        assert_eq!(sign.len(), 6, "偏移是 ±HH:MM：{s}");
        assert_eq!(sign.as_bytes()[3], b':', "偏移里要有冒号：{s}");
        // 能被标准解析器认出来（客户端就是这么读的）
        assert!(
            time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339).is_ok()
        );
    }

    /// 剩下的两种格式必须能互相区分（产物那份 `release_time` 已于 2026-11-01 删）
    #[test]
    fn the_two_remaining_formats_differ() {
        assert_ne!(now_iso8601(), now_stamp());
    }
}
