//! 状态词与解释句的唯一出处（doc §13）。
//!
//! # 为什么状态枚举也放在这里
//!
//! 每个枚举变体对应界面上**一个词**。变体与它的中文挨着写，「一个词一处来源」
//! 才是结构上成立的，而不是靠记性 —— 枚举在 `derive.rs`、中文在这里的话，
//! 加一个变体不会有任何东西提醒你去补词。
//!
//! [`tests::every_variant_has_a_word`] 逐个变体过一遍，缺一个就失败。
//!
//! # 三条纪律（决议 5）
//!
//! 1. **解释句写「改了会怎样」，不写「来自哪一层」。** 「这一项来自机型基底」是实现细节；
//!    「改这里会影响这台机型下所有没单独设过的版本」才是用户要判断的事。
//! 2. **每个被禁用的控件都要有一句为什么。** 禁用而不说理由，用户只能猜是不是坏了。
//! 3. **不编空表**。没有内容时写出「为什么没有」，而不是留白 —— 空白会被读成「还没算」。
//!
//! # 四个近义词，各管一件事，不许混用
//!
//! | 词 | 什么时候用 | 反例 |
//! |---|---|---|
//! | **暂不支持** | 这台机型/这个版本，我们还没打算做这件事 | 不要用在「上游没给数据」上 |
//! | **暂无资源** | 配方在、参数都有值，但**没有可交付的产物**（A2L） | 不要写成「暂不支持」——参数是好的 |
//! | **未配置** | 该有人填的位置还空着（有效配方为空、套餐没挑东西） | 不要用在「上游没声明」上 |
//! | **未声明** | **上游**没给这项声明（`minimumClient` 是空串） | 不要写成「未配置」——不是我们该填的 |
//!
//! A2L 是这四个词最容易混的地方：它的**参数不缺**（落到出厂默认，灰色「出厂」，有值可看），
//! 缺的只有资源。所以参数那一面什么提示都不该有，资源那一面写「暂无资源」。

use serde::Serialize;
use serde_json::Value;

use crate::workbench::presets::registry::{ParamDef, UiComponent, ValueType};

use super::layer::{Level, Origin};
use super::patch::Visibility;

/* ---------- 生成状态四档（doc §10.1） ---------- */

/// 判据是**快照指纹比对，不看文件时间**。
///
/// 看时间的下场有两个，都很难查：碰一下文件就变「新」，而配方改了却看不出来；
/// 产物的时间戳一变，客户端就可能莫名要更新
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildState {
    /// 当前有效配方 == 上次成功生成时的指纹
    Built,
    /// 不相等
    Stale,
    /// 没有生成记录
    NeverBuilt,
    /// 没有 MKP 产物、也没有 BBS，而且我们的两层一项都没写（全靠出厂默认）。
    /// **与「未配置」不是一回事** —— 参数照样能看能改
    NoResources,
}

impl BuildState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Built => "已生成",
            Self::Stale => "待生成",
            Self::NeverBuilt => "未生成",
            Self::NoResources => "暂无资源",
        }
    }

    /// 「改了会怎样 / 现在该做什么」，不是「这个状态怎么算出来的」
    pub fn explain(self) -> &'static str {
        match self {
            Self::Built => "产物和当前配方一致，不用重新生成",
            Self::Stale => "配方改过了，产物还是旧的；生成之后客户端才会拿到新的",
            Self::NeverBuilt => "还没生成过，客户端现在下载不到这一版",
            Self::NoResources => "没有可交付的产物，参数也全是出厂默认 —— 还没有为它写过配方。参数照样能看能改",
        }
    }

    /// 这一档能不能进生成队列
    pub fn buildable(self) -> bool {
        matches!(self, Self::Stale | Self::NeverBuilt)
    }
}

/* ---------- 整本的产物状态（状态条那一格） ---------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactState {
    Fresh,
    Stale,
    Missing,
}

impl ArtifactState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fresh => "已生成",
            Self::Stale => "待生成",
            Self::Missing => "未生成",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Fresh => "所有该有产物的版本都是最新的",
            Self::Stale => "有版本的配方改过了，产物还没跟上",
            Self::Missing => "一个产物都还没生成过",
        }
    }
}

/* ---------- 草稿状态（文件头与状态栏） ---------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveState {
    Saved,
    Dirty,
}

impl SaveState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Saved => "已保存",
            Self::Dirty => "未保存",
        }
    }
}

/* ---------- 崩溃快照状态 ---------- */

/// 草稿快照（`.draft/book.json`）的三态。
///
/// **与 [`SaveState`] 是两件事，不许合成一句**：
/// 「未保存」说的是仓库文件里还没有这些改动；
/// 「待落盘」说的是崩溃快照还没跟上 —— 草稿本身在内存里，是真相。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotState {
    Current,
    Pending,
    Failed,
}

impl SnapshotState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Current => "快照已跟上",
            Self::Pending => "待落盘",
            Self::Failed => "快照写不进去",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Current => "停手之后已经写过一次崩溃快照，现在崩了也不丢",
            Self::Pending => "刚改的还在内存里，停手 2 秒后会写一次快照",
            Self::Failed => "快照写不进去，这会儿崩了会丢掉未保存的改动。改动本身没受影响",
        }
    }
}

/// 快照失败那条提示的前缀。后面接系统给的原因
pub const SNAPSHOT_FAILED: &str = "草稿快照写不进去（改动还在，但崩了会丢）";

/* ---------- BBS 三态（doc 的进口饮料） ---------- */

/// 一条 BBS 曲线在**交付**上的身份。
///
/// 与「有没有进套餐」是**两件正交的事**：一条曲线可以已分配给某个版本、
/// 同时不属于任何套餐
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BbsAssign {
    /// 已经挂在某个版本（或机型默认）上
    Assigned,
    /// 在菜单里，但还没挂给谁
    Optional,
    /// 仅归档：仓库里有，客户看不到
    ArchiveOnly,
}

impl BbsAssign {
    pub fn label(self) -> &'static str {
        match self {
            Self::Assigned => "已分配",
            Self::Optional => "可选",
            Self::ArchiveOnly => "仅归档",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Assigned => "有版本用着它，改动会影响那些版本的交付",
            Self::Optional => "客户能看到，但还没有版本指定用它",
            Self::ArchiveOnly => "仓库里留着，客户看不到也下载不到",
        }
    }
}

/// 一个版本的 BBS 是自己挑的还是跟着机型默认
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BbsSource {
    Own,
    InheritedFromMachine,
}

impl BbsSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Own => "本版本单独一份",
            Self::InheritedFromMachine => "跟机型默认",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Own => "改机型默认不会影响这一版",
            Self::InheritedFromMachine => "改机型默认，这一版跟着变",
        }
    }
}

/* ---------- 来源三档 ---------- */

pub fn origin_label(origin: Origin) -> &'static str {
    match origin {
        Origin::Factory => "出厂",
        Origin::Machine => "机型",
        Origin::Version => "版本",
    }
}

/// **写「改了会怎样」**（决议 5）。
///
/// 「这一项来自机型基底」是实现细节；用户要判断的是「我在这儿改一下，谁会跟着变」
pub fn origin_explain(origin: Origin) -> &'static str {
    match origin {
        Origin::Factory => "没人设过，用的是字段定义里的默认值；在这里改会写到你选中的那一层",
        Origin::Machine => "改机型这一层，这台机型下所有没单独设过的版本都跟着变",
        Origin::Version => "只改这一版，别的版本不受影响",
    }
}

pub fn level_label(level: Level) -> &'static str {
    match level {
        Level::Machine => "机型基底",
        Level::Version => "版本覆盖",
    }
}

pub fn visibility_label(v: Visibility) -> &'static str {
    match v {
        Visibility::Menu => "上菜单",
        Visibility::ArchiveOnly => "仅归档",
    }
}

pub fn visibility_explain(v: Visibility) -> &'static str {
    match v {
        Visibility::Menu => "客户看得到、能下载",
        Visibility::ArchiveOnly => "仓库里留着，客户看不到。不是删除",
    }
}

/* ---------- 空值与占位 ---------- */

/// 空串显示成「空」而**不是空白**（doc §8.3）。
///
/// 空白格子和「这一项没有值」在屏幕上长得一样，而它们是两件事 ——
/// 那两个 G-code 字段的出厂默认就是空串
pub const BLANK: &str = "空";
/// 这一项在这台机型上不存在
pub const NOT_APPLICABLE: &str = "不适用";
/// 上游没给这项声明
pub const UNDECLARED: &str = "未声明";
/// 该填的位置还空着
pub const UNCONFIGURED: &str = "未配置";
/// 我们还没打算支持
pub const UNSUPPORTED: &str = "暂不支持";

/// 值在格子里显示成什么。**格式化一律由后端做**，前端不碰（doc §8.3）。
///
/// 三条：空串写「空」；开关写「开启 / 关闭」；有 `choices` 的写命中项的中文名 ——
/// 界面上显示 `disk` 的话，用户得在一堆中文按钮里猜哪个是 `disk`
pub fn value_text(param: &ParamDef, value: &Value) -> String {
    if param.value_type == ValueType::Bool {
        return match value {
            Value::Bool(true) => "开启".into(),
            Value::Bool(false) => "关闭".into(),
            other => plain(other),
        };
    }
    if let Some(hit) = param.choices.iter().find(|c| &c.value == value) {
        return if hit.deprecated {
            // 当前值正好是废弃项时要标出来（doc §15）：它还能用，但不该继续用
            format!("{}（已废弃）", hit.label)
        } else {
            hit.label.clone()
        };
    }
    let text = plain(value);
    if text.is_empty() {
        return BLANK.to_owned();
    }
    match &param.unit {
        Some(u) if !u.is_empty() => format!("{text} {u}"),
        _ => text,
    }
}

/// G-code 字段在格子里不显示内容，只报行数 + 一个点开的入口（doc §8.3）。
///
/// 把几十行 G-code 挤进一个单元格，既看不清也会把行高撑坏
pub fn gcode_text(value: &Value) -> (usize, String) {
    let lines = match value {
        Value::String(s) if !s.is_empty() => s.lines().count(),
        _ => 0,
    };
    if lines == 0 {
        (0, BLANK.to_owned())
    } else {
        (lines, format!("{lines} 行 · 点开"))
    }
}

pub fn is_gcode(param: &ParamDef) -> bool {
    param.ui_component == UiComponent::Gcode
}

/// JSON 值的裸文本。**字符串不带引号** —— 界面上出现 `"tower"` 这种带引号的值
/// 会被当成值的一部分
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/* ---------- 「为什么不能点」 ---------- */

/// 每个可能被禁用的动作各一句。**禁用而不说理由，用户只能猜是不是坏了**
pub mod disabled {
    /// 「挂回继承」在这一层本来就没写过这一项时不能点
    pub const DETACH_NOTHING_TO_DETACH: &str = "这一层没有单独设过这一项，本来就是继承来的";
    /// 能点时也给一句，说清点下去会发生什么
    pub const DETACH_READY: &str = "删掉这一层的这一项，让它跟着上一层变";
    /// 被上级条件关着
    pub const BLOCKED_BY_CONDITION: &str = "上一项没打开，这一项现在不生效，所以不让改";
    /// 这台机型没有这个字段
    pub const NOT_APPLICABLE: &str = "这台机型没有这一项，不是值为空";
    /// G-code 拒绝批量
    pub const BULK_REFUSES_GCODE: &str =
        "G-code 不做批量：一段多行脚本被整体盖掉是不可逆的误操作，请逐列点开改";
    /// 有阻断时生成按钮全禁用
    pub const BUILD_BLOCKED: &str = "有阻断问题没解决，生成一定会出错";
    /// 没有可生成的项
    pub const BUILD_NOTHING_TO_DO: &str = "所有产物都和当前配方一致，没有要生成的";
    /// 这一版没有可交付的产物
    pub const BUILD_NO_RESOURCES: &str = "这一版没有可交付的产物，生成不出东西来";
    /// 机型没配尺寸 —— **占位机型，不参与交付**。
    ///
    /// 与「暂无资源」分开是因为两件事的性质不同：缺资源是**漏**（该有的没有），
    /// 占位是**刻意**（这台还没开始做）。说成缺资源会让人去找资源，
    /// 而这里根本没有要找的东西。
    ///
    /// 也刻意不写成「先把尺寸补上」那种催办口气：空着是闭合的 ——
    /// 这台不生成、不进清单，消费端也拿不到（它的内置尺寸表里同样没有这台）
    pub const BUILD_NO_DIMENSIONS: &str = "这台机型还没配尺寸（占位），不参与交付";
    /// 草稿干净时保存 / 丢弃都不能点
    pub const NOTHING_TO_SAVE: &str = "没有未保存的改动";
    /// 撤销栈空
    pub const NOTHING_TO_UNDO: &str = "没有可以撤销的操作";
    /// 这次手势不可撤销
    pub const NOT_UNDOABLE: &str = "删除和生成记录不进撤销栈；删掉的版本在回收站里";
}

/* ---------- 「谁把我关了」 ---------- */

/// 带变量的几句。**整句在这里拼好再交给前端**，前端不拿模板去填空 ——
/// 那等于把「句子长什么样」这件事分到两个仓库里，迟早一边改了另一边没跟上。
///
/// 这一组是反馈里「灰色之后我不知道是哪个选项导致它灰色」的直接解药：
/// 关联关系要**常驻可读**，不能只躺在 tooltip 里。
pub mod relate {
    /// 行头常驻：这一项归谁管。悬停之前就该读得到
    pub fn controlled_by(label: &str) -> String {
        format!("受「{label}」控制")
    }

    /// 整组被 section 级条件关着 —— 要用户做的事一样，但解释句不同
    pub fn group_controlled_by(label: &str) -> String {
        format!("整组由「{label}」控制")
    }

    /// 子项行头：它挂在哪个父项下面
    pub fn belongs_to(label: &str) -> String {
        format!("属于：{label}")
    }

    /// 点开一个灰格子时的那一句。`need` 是 `BlockedBy::need`，已经是整句
    pub fn blocked_note(label: &str, need: &str) -> String {
        format!("改不动：由「{label}」控制，需{need}")
    }

    /// 跳过去改那一项
    pub const GO_FIX_IT: &str = "去改那一项";

    /// 控制它的那一项在这台机型上根本没有 —— 这时候**不给跳转**。
    /// 骗用户去点一个不存在的格子，比直接说出这是上游数据问题更糟
    pub fn controller_not_here(label: &str) -> String {
        format!("控制它的「{label}」在这台机型上没有这一项")
    }

    /// 一个父项把自己下面整组关掉了。列表里直接把那几项收起来，只留这一句 ——
    /// 矩阵里只能靠每格一行灰字，那是「信息过载」的来源
    pub fn family_off(label: &str, value: &str, n: usize) -> String {
        format!("「{label}」选了{value}，下面这 {n} 项现在不生效")
    }

    /// 整组被 section 级条件关掉
    pub fn group_off(label: &str, value: &str, n: usize) -> String {
        format!("「{label}」选了{value}，这一组 {n} 项现在不生效")
    }

    /// 收起来的那几项点开看的入口
    pub const SHOW_ANYWAY: &str = "仍然展开看";
}

/* ---------- 不留白 ---------- */

/// 零问题时**明确写出来**，因为空白会被读成「还没校验」（doc §10.2）
pub const NO_ISSUES: &str = "都过了";
/// 回收站空
pub const TRASH_EMPTY: &str = "回收站是空的";
/// 没有被关掉的回退规则
pub const NO_DISABLED_FALLBACK: &str = "当前没有关掉的规则";
/// 矩阵没有命中行
pub const MATRIX_NO_MATCH: &str = "没有匹配的字段";
/// 一列都没勾
pub const MATRIX_NO_COLS: &str = "在左边配方本里勾选机型或版本，勾中的会成为这里的列";
/// 搜索一开，分类过滤让开（doc §8.1）
pub const MATRIX_SEARCH_SPANS_ALL_TABS: &str = "搜索跨全部分类";

#[cfg(test)]
mod tests {
    use super::*;

    /// **每个变体都要有词，缺一个就失败。**
    ///
    /// 这条判据存在的理由：加一个枚举变体时，编译器会提醒你补 `match` 分支，
    /// 但不会提醒你那句中文是不是随手写的占位。这里逐个过一遍，
    /// 顺手把「词不能为空、不能撞、解释句不能等于词本身」也钉住
    #[test]
    fn every_variant_has_a_word() {
        let build = [
            BuildState::Built,
            BuildState::Stale,
            BuildState::NeverBuilt,
            BuildState::NoResources,
        ];
        let mut seen: Vec<&str> = Vec::new();
        for s in build {
            check(s.label(), s.explain(), &mut seen);
        }
        for s in [ArtifactState::Fresh, ArtifactState::Stale, ArtifactState::Missing] {
            assert!(!s.label().is_empty());
            assert!(!s.explain().is_empty());
        }
        let mut seen: Vec<&str> = Vec::new();
        for s in [SaveState::Saved, SaveState::Dirty] {
            assert!(!s.label().is_empty());
            assert!(!seen.contains(&s.label()));
            seen.push(s.label());
        }
        let mut seen: Vec<&str> = Vec::new();
        for s in [BbsAssign::Assigned, BbsAssign::Optional, BbsAssign::ArchiveOnly] {
            check(s.label(), s.explain(), &mut seen);
        }
        let mut seen: Vec<&str> = Vec::new();
        for s in [BbsSource::Own, BbsSource::InheritedFromMachine] {
            check(s.label(), s.explain(), &mut seen);
        }
        let mut seen: Vec<&str> = Vec::new();
        for o in [Origin::Factory, Origin::Machine, Origin::Version] {
            check(origin_label(o), origin_explain(o), &mut seen);
        }
        let mut seen: Vec<&str> = Vec::new();
        for l in [Level::Machine, Level::Version] {
            assert!(!level_label(l).is_empty());
            assert!(!seen.contains(&level_label(l)));
            seen.push(level_label(l));
        }
        let mut seen: Vec<&str> = Vec::new();
        for v in [Visibility::Menu, Visibility::ArchiveOnly] {
            check(visibility_label(v), visibility_explain(v), &mut seen);
        }
    }

    fn check(label: &'static str, explain: &str, seen: &mut Vec<&'static str>) {
        assert!(!label.trim().is_empty(), "有变体没有词");
        assert!(!explain.trim().is_empty(), "{label} 没有解释句");
        assert_ne!(explain, label, "{label} 的解释句只是把词重复了一遍");
        assert!(!seen.contains(&label), "两个变体用了同一个词：{label}");
        seen.push(label);
    }

    /// 四个近义词互不相同 —— 混用是这一版最想避开的事之一
    #[test]
    fn the_four_near_synonyms_are_distinct() {
        let all = [NOT_APPLICABLE, UNDECLARED, UNCONFIGURED, UNSUPPORTED, BLANK];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "两个占位词写成了同一个");
            }
        }
        // 「暂无资源」归 BuildState，这里确认它没和上面那几个撞
        assert!(!all.contains(&BuildState::NoResources.label()));
    }

    fn param(value_type: &str, ui: &str, choices: serde_json::Value, unit: Option<&str>) -> ParamDef {
        let mut v = serde_json::json!({
            "key": "k", "configKey": "K", "tomlKey": "k", "jsonKey": "k",
            "label": "字段", "desc": "", "tomlComment": "",
            "valueType": value_type, "uiComponent": ui, "defaultValue": 0,
            "scope": "universal", "section": "x",
            "layout": { "order": 1, "sectionId": "s1" },
            "choices": choices
        });
        if let Some(u) = unit {
            v["unit"] = serde_json::json!(u);
        }
        serde_json::from_value(v).unwrap()
    }

    /// 空串写「空」，不是空白
    #[test]
    fn an_empty_string_reads_as_the_word_blank() {
        let p = param("string", "gcode", serde_json::json!([]), None);
        assert_eq!(value_text(&p, &serde_json::json!("")), BLANK);
    }

    /// 开关写「开启 / 关闭」
    #[test]
    fn a_switch_reads_as_on_or_off() {
        let p = param("bool", "switch", serde_json::json!([]), None);
        assert_eq!(value_text(&p, &serde_json::json!(true)), "开启");
        assert_eq!(value_text(&p, &serde_json::json!(false)), "关闭");
    }

    /// 枚举写命中项的中文名，不写原始值
    #[test]
    fn an_enum_reads_as_its_chinese_choice_label() {
        let p = param(
            "string",
            "segmented",
            serde_json::json!([
                { "label": "擦料塔", "value": "tower" },
                { "label": "圆盘擦拭", "value": "disk" }
            ]),
            None,
        );
        assert_eq!(value_text(&p, &serde_json::json!("disk")), "圆盘擦拭");
        // 没命中任何选项时退回原始值 —— 不能显示成空白，那会让人以为没设
        assert_eq!(value_text(&p, &serde_json::json!("teleport")), "teleport");
    }

    /// 当前值正好是废弃选项时要标出来（doc §15）
    #[test]
    fn a_deprecated_choice_is_marked() {
        let p = param(
            "string",
            "select",
            serde_json::json!([{ "label": "老写法", "value": "legacy", "deprecated": true }]),
            None,
        );
        assert_eq!(value_text(&p, &serde_json::json!("legacy")), "老写法（已废弃）");
    }

    /// 有单位就带上单位
    #[test]
    fn a_unit_is_appended() {
        let p = param("float", "number", serde_json::json!([]), Some("mm"));
        assert_eq!(value_text(&p, &serde_json::json!(1.5)), "1.5 mm");
    }

    /// G-code 只报行数。空的时候报「空」而不是「0 行」——
    /// 「0 行 · 点开」点开是一片空白，没有意义
    #[test]
    fn gcode_reports_line_count_and_says_blank_when_empty() {
        assert_eq!(gcode_text(&serde_json::json!("A\nB\nC")), (3, "3 行 · 点开".to_owned()));
        assert_eq!(gcode_text(&serde_json::json!("")), (0, BLANK.to_owned()));
        assert_eq!(gcode_text(&serde_json::json!(null)), (0, BLANK.to_owned()));
    }

    /// 「挂回继承」能点与不能点各有一句，且不是同一句（doc §8.5）
    #[test]
    fn detach_has_two_different_sentences() {
        assert_ne!(
            disabled::DETACH_READY,
            disabled::DETACH_NOTHING_TO_DETACH,
            "能点和不能点说了同一句话，等于没解释"
        );
    }

    /// 关联那一组：**每个模板都要被调用一次**，且都真的把名字填进去了。
    /// 加了词却没接线的话，界面上只会剩一句「受「」控制」
    #[test]
    fn every_relate_sentence_names_the_field_it_talks_about() {
        let all = [
            relate::controlled_by("擦拭部件"),
            relate::group_controlled_by("擦拭部件"),
            relate::belongs_to("擦拭部件"),
            relate::blocked_note("擦拭部件", "等于 擦料塔"),
            relate::controller_not_here("擦拭部件"),
        ];
        for s in &all {
            assert!(s.contains("擦拭部件"), "这一句没把字段名说出来：{s}");
            assert!(!s.contains("{}"), "占位符没被替换：{s}");
        }
        // 五句互不相同 —— 说同一句话等于没区分场景
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert!(relate::blocked_note("擦拭部件", "等于 擦料塔").contains("等于 擦料塔"));
        assert!(!relate::GO_FIX_IT.is_empty());
    }
}
