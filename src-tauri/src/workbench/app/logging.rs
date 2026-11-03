//! 写盘日志的字段拼装：**一个键一行事实**，`req` 串起一次手势的整条链路。
//!
//! # 为什么单独一个模块
//!
//! 作者 2026-11-02：「现在的问题很多，修了很久了，不如日志系统完善一下方便找问题，
//! 你需要知道我的触发的前端变化后端变化，以及不是我点击的非直接变化」。
//!
//! 那之前的日志只复述**前端发来的意图**（一条 patch 一句话），而真正写进文件的键
//! 可能更多（改一次机型基底会连带写掉几个版本键），也可能更少（同值被丢弃）。
//! 于是 `written=3` 这种行谁也看不出那 3 个是谁、各从多少变到多少 —— 等于没记。
//!
//! 这里的 [`KeyChange`] 描述的是**落盘事实**：哪个键、哪一层、改前是什么、改后是什么、
//! **为什么**它被写。`why` 必须由产生这次写入的那段代码填，调用方不许猜。
//!
//! # 截断
//!
//! 参数值里有 G-code（整段脚本），一条日志几十行会把日志刷成垃圾。所以超过
//! [`MAX_VALUE_CHARS`] 的值只留开头并标总长 —— 找问题要的是"它变了"，不是全文。

use serde_json::Value;

/// 值超过这个长度就截断（唯一实现在 [`crate::logfmt`]）
pub use crate::logfmt::MAX_VALUE_CHARS;

/// 一次写入里**某一个键**的变化。
#[derive(Debug, Clone)]
pub struct KeyChange {
    /// 字段 key（`toolhead.offset.z`）
    pub key: String,
    /// `machineVariants` 里的那种写法：`A1` / `A1:STANDARD`
    pub owner: String,
    /// 写之前这一层的值（`None` = 这一层本来没有那一行）
    pub before: Option<Value>,
    /// 写之后这一层的值（`None` = 那一行被删掉）
    pub after: Option<Value>,
    /// 为什么写它 —— 见 [`Why`]
    pub why: Why,
}

impl KeyChange {
    /// 一行人话：`A1:STANDARD · toolhead.offset.z: 3.9 → 3.95（用户改）`
    ///
    /// `owner` 放前面，因为同一次操作里 key 往往一样、只有层不同 ——
    /// 找问题时先看"是哪一层"。
    pub fn render(&self) -> String {
        format!(
            "{} · {}: {} → {}{}",
            self.owner,
            self.key,
            show_value(&self.before),
            show_value(&self.after),
            self.why.suffix()
        )
    }
}

/// 这次写入**为什么**发生。只有能确定的地方才许填，拿不准就 `Unknown`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// 用户直接改的那一格
    UserEdit,
    /// **随基底生效**：这一版的状态是"跟随"，改机型基底时它跟着一起变
    /// （2026-11-03 起判据是 `[params.follows]`，不是"值等于旧基底"）
    Follows,
    /// 归并上提：改机型基底时，被一起写下去的版本键（本次会话没有直接改它们）
    Promoted,
    /// 复制配方 / 新建版本时铺的整批独立快照
    CopySnapshot,
    /// 清空变体表后连表头一起删掉
    EmptyVariantsDropped,
    /// 位置不明 —— **宁可承认不知道，也不要标成用户操作**
    Unknown,
}

impl Why {
    /// 日志行尾的那半句。`UserEdit` 不印（那是默认情形，印了反而吵）
    pub fn suffix(self) -> &'static str {
        match self {
            Self::UserEdit => "",
            Self::Follows => "（随基底生效）",
            Self::Promoted => "（归并连带）",
            Self::CopySnapshot => "（复制快照）",
            Self::EmptyVariantsDropped => "（空表清理）",
            Self::Unknown => "（来源不明）",
        }
    }

    /// 给结构化日志用的短标签
    pub fn tag(self) -> &'static str {
        match self {
            Self::UserEdit => "user",
            Self::Follows => "follows",
            Self::Promoted => "promoted",
            Self::CopySnapshot => "copy",
            Self::EmptyVariantsDropped => "empty",
            Self::Unknown => "unknown",
        }
    }
}

/// 日志里的值怎么印 —— **唯一实现在 [`crate::logfmt::show`]**（两档共用那一份）。
pub use crate::logfmt::show as show_value;

/// 一批变化拼成一行 `changes=...`，条数太多时只报个数。
///
/// 上限 [`crate::logfmt::MAX_ITEMS`]：一次批量改 74 条时，把 74 段都印出来对找问题
/// 没帮助，反而把同一次点击的其它行挤到屏幕外。
pub fn render_changes(changes: &[KeyChange]) -> String {
    if changes.is_empty() {
        return "（无）".to_owned();
    }
    if changes.len() > crate::logfmt::MAX_ITEMS {
        return format!(
            "（{} 处，超过 {} 不逐条展开）",
            changes.len(),
            crate::logfmt::MAX_ITEMS
        );
    }
    changes
        .iter()
        .map(KeyChange::render)
        .collect::<Vec<_>>()
        .join("；")
}

/* ---------- 非值类写盘命令的变更摘要 ---------- */

/// 一条**改了什么** —— 定义在 [`crate::logfmt`]（客户端档也要编得到它）。
pub use crate::logfmt::FieldChange;

/// 结构类摘要拼成一行 `fields=...`
pub fn render_fields(fields: &[FieldChange]) -> String {
    crate::logfmt::render(fields)
}

/// **把日志埋进 trace 上下文**：一次手势打了七八行，每一行都要能按同一个号捞出来。
///
/// 请求号只有一份（[`super::current_req`]），这里只是把它摆成一行前缀；
/// 不在命令里（CLI / 测试）时给个空号，不编假值。
pub fn trace_prefix() -> String {
    match super::current_req() {
        Some(req) => format!("[{req}] "),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 值的印法：`None` / `null` / 字符串 / 数字 各说各的话
    #[test]
    fn values_are_printed_distinctly() {
        assert_eq!(show_value(&Some(json!(3.9))), "3.9");
        assert_eq!(show_value(&Some(json!(true))), "true");
        assert_eq!(show_value(&Some(json!("wipe_tower"))), "wipe_tower");
        assert_eq!(show_value(&None), "（跟随上一层）");
        assert_eq!(show_value(&Some(json!(null))), "（空）");
        assert_ne!(
            show_value(&None),
            show_value(&Some(json!(null))),
            "「这一层没有这一行」和「这一行的值是空」不许混成一句"
        );
    }

    /// 长值截断但保留总长 —— 找问题要的是"它变了"，不是全文
    #[test]
    fn long_values_are_truncated_with_a_length_note() {
        let gcode = "G1 X0 Y0\n".repeat(60);
        let shown = show_value(&Some(json!(gcode.clone())));
        assert!(shown.starts_with("G1 X0 Y0"));
        assert!(shown.contains(&format!("共 {} 字符", gcode.chars().count())));
        assert!(shown.chars().count() < gcode.chars().count());
    }

    /// 一次变化的一行话：层在最前，原因跟在最后
    #[test]
    fn one_change_reads_like_an_event() {
        let change = KeyChange {
            key: "toolhead.offset.z".to_owned(),
            owner: "A1:STANDARD".to_owned(),
            before: Some(json!(3.9)),
            after: Some(json!(3.95)),
            why: Why::Promoted,
        };
        assert_eq!(
            change.render(),
            "A1:STANDARD · toolhead.offset.z: 3.9 → 3.95（归并连带）"
        );
    }

    /// 条数过多只报个数（批量改 74 条时别把日志刷成一片）
    #[test]
    fn too_many_changes_are_summarized() {
        let one = KeyChange {
            key: "k".to_owned(),
            owner: "A1".to_owned(),
            before: None,
            after: Some(json!(1)),
            why: Why::UserEdit,
        };
        let many = vec![one; crate::logfmt::MAX_ITEMS + 1];
        assert!(render_changes(&many).contains("超过"));
        assert!(render_changes(&[]).contains("无"));
    }
}
