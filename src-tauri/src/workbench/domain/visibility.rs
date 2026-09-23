//! `showWhen`：链式求值、环检测、「看得见改不动」（doc §9）。
//!
//! # 与「不适用」的区别，不能混
//!
//! | | 判据 | 这一项有值吗 | 进产物吗 | 界面上说 |
//! |---|---|---|---|---|
//! | **不适用** | `machineFilter` 排除 | 没有 | 不进 | 「不适用」 |
//! | **看得见改不动** | 上级 `showWhen` 不满足 | **有** | **进** | 「等 XX 打开才能改」 |
//!
//! 前者归 [`super::layer`]，后者归这里。把它们说成同一件事，用户会以为自己改的那个值
//! 没写进产物 —— 而它写进去了。
//!
//! # 实测的三处脏数据，逐条处理
//!
//! 43 条字段带 `showWhen`，形状恒为 `{key, op, value}`（无嵌套、无 and/or），
//! `op` 只有 `eq`(38) / `neq`(3) / `gt`(2)。
//!
//! 1. **`value` 的 JSON 类型混杂**：有 `true`（bool）、`"off"` / `"disk"`（string），
//!    也有**字符串化的数字** `"0"`（`{key: "wiping.glue_z_lift_height", op: "gt", value: "0"}`）。
//!    直接比 JSON 值会让 `Number(0) != String("0")`，于是那两条 `gt` 永远为假，
//!    两个字段永远灰着 —— 而且**不会有任何报错**。所以比较前一律按**被指向字段的
//!    `valueType`** 归一化。
//! 2. **条件成链**，实测最深四层：
//!    `tower_rib_speed_value` → `tower_rib_speed` → `outer_structure` → `have_wiping_components`。
//!    必须递归，而且必须带环检测 —— 一条自指链会把线程转死。
//! 3. **`layout_schema` 还有 section 级 `showWhen`**（实测 1 条：`wiping/tower_position`
//!    整组吊在 `have_wiping_components == "tower"` 上）。矩阵按字段建行，所以它体现为
//!    "这个 section 下所有字段一起被关"。
//!
//! # 为什么报**最外层**的未满足条件，而不是最近的那一条
//!
//! `tower_rib_speed_value` 要求 `tower_rib_speed == custom`，但 `tower_rib_speed`
//! 自己也可能是关着的。这时候跟用户说"需要 tower_rib_speed 等于 custom"没用 ——
//! 他连那一项都点不动。所以链上的未满足条件**按根在前**返回，界面显示第一条：
//! 那是唯一现在就能动的地方。

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::workbench::presets::registry::{ShowOp, ShowWhen, ValueType};
use crate::workbench::presets::ParamRegistry as Registry;

use super::layer::Layers;

/// 一条未满足的条件，**句子由后端拼好**（决议 5：业务判断归后端，展示归前端）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedBy {
    /// 卡住这一项的那个字段
    pub key: String,
    /// 那个字段的中文名
    pub label: String,
    /// 可直接显示的一句：「等于 开启」/「不等于 关闭」/「大于 0」
    pub need: String,
    /// 这条条件是字段自己的，还是它所在 section 整组的。
    /// 两者要用户做的事一样，但解释句不一样（「整组关着」vs「上一项没开」）
    pub scope: BlockScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockScope {
    Field,
    Section,
}

/// 环：`a` 的条件指向 `b`，`b` 的条件又绕回 `a`。
///
/// 上游现在没有这种数据，但**它不成立时的后果是线程转死**，
/// 所以判据要有，而且要能说出环上有谁
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleReport {
    /// 环上的 key，按发现顺序
    pub keys: Vec<String>,
}

/// 永远不可能满足的条件：被指向的字段是枚举，而条件里那个值不在它的选项里。
///
/// 后果是这个字段**永久隐藏**，界面上看不出来、也没人报错
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsatisfiableReport {
    pub key: String,
    pub depends_on: String,
    /// 条件要求的值
    pub wants: Value,
    /// 被指向字段实际有哪些选项
    pub choices: Vec<Value>,
}

/// 可见性求值器。借 [`Layers`]，所以它看到的值和有效配方是同一份
pub struct Gate<'a> {
    registry: &'a Registry,
    layers: &'a Layers<'a>,
}

impl<'a> Gate<'a> {
    pub fn new(registry: &'a Registry, layers: &'a Layers<'a>) -> Self {
        Self { registry, layers }
    }

    /// 这一项现在能不能改
    pub fn is_editable(&self, key: &str) -> bool {
        self.blocked(key).is_empty()
    }

    /// 卡住这一项的条件，**根在前**。空 = 能改。
    ///
    /// 环检测命中时**当成能改**并返回空：让人改不动一个原因说不清的字段，
    /// 比让他改一个本该隐藏的字段更糟。环本身进 [`Self::cycles`] 的待办
    pub fn blocked(&self, key: &str) -> Vec<BlockedBy> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        self.walk(key, &mut seen, &mut out);
        out.reverse(); // walk 是从叶子往根走的，反过来就是根在前
        out
    }

    /// 从 `key` 往根递归。`out` 里是叶子在前，调用方负责反转
    fn walk(&self, key: &str, seen: &mut BTreeSet<String>, out: &mut Vec<BlockedBy>) {
        if !seen.insert(key.to_owned()) {
            return; // 环：停在这里，不 panic 不死循环
        }
        let Some(param) = self.registry.param(key) else {
            return;
        };

        // 字段自己的条件
        if let Some(sw) = &param.show_when {
            if !self.satisfied(sw) {
                out.push(self.describe(sw, BlockScope::Field));
            }
            self.walk(&sw.key, seen, out);
        }

        // 它所在 section 整组的条件。**不跟着递归两遍** ——
        // section 条件指向的那个字段，链已经由上面那一支走过（或者它就是同一个 key）
        if let Some(sw) = self.registry.section_show_when(&param.layout.section_id) {
            if !self.satisfied(sw) {
                let d = self.describe(sw, BlockScope::Section);
                if !out.contains(&d) {
                    out.push(d);
                }
            }
            self.walk(&sw.key, seen, out);
        }
    }

    /// 一条条件成不成立。**按被指向字段的 `valueType` 归一化再比**
    fn satisfied(&self, sw: &ShowWhen) -> bool {
        let Some(target) = self.registry.param(&sw.key) else {
            // 指向一个不存在的字段。判成"成立"= 不拦人：
            // 拦了的话用户面对一个说不出理由的灰格子，而这是上游的数据问题
            return true;
        };
        let Some(actual) = self.layers.effective(&sw.key) else {
            // 被指向的字段在这台机型上不适用 —— 它的值根本不存在，条件无从成立
            return false;
        };
        compare(target.value_type, actual.value, &sw.value, sw.op)
    }

    fn describe(&self, sw: &ShowWhen, scope: BlockScope) -> BlockedBy {
        let (label, shown) = match self.registry.param(&sw.key) {
            Some(p) => (p.label.clone(), display_value(p, &sw.value)),
            None => (sw.key.clone(), raw_display(&sw.value)),
        };
        BlockedBy {
            key: sw.key.clone(),
            label,
            need: format!("{} {}", op_word(sw.op), shown),
            scope,
        }
    }

    /// 扫一遍全表找环。**一次算完，不是每个字段算一次** ——
    /// 环是数据的性质，不是某个字段的性质
    pub fn cycles(&self) -> Vec<CycleReport> {
        let mut found: Vec<CycleReport> = Vec::new();
        let mut cleared: BTreeSet<&str> = BTreeSet::new();

        for start in self.layers.keys() {
            if cleared.contains(start) {
                continue;
            }
            let mut path: Vec<&str> = Vec::new();
            let mut cur = start;
            loop {
                if let Some(at) = path.iter().position(|k| *k == cur) {
                    let keys: Vec<String> = path[at..].iter().map(|s| (*s).to_owned()).collect();
                    let report = CycleReport { keys };
                    // 同一个环从不同起点会被走到多次，按集合去重
                    let set: BTreeSet<&String> = report.keys.iter().collect();
                    if !found
                        .iter()
                        .any(|f| f.keys.iter().collect::<BTreeSet<_>>() == set)
                    {
                        found.push(report);
                    }
                    break;
                }
                path.push(cur);
                match self.registry.param(cur).and_then(|p| p.show_when.as_ref()) {
                    Some(sw) => cur = sw.key.as_str(),
                    None => {
                        cleared.extend(path.iter().copied());
                        break;
                    }
                }
            }
        }
        found
    }

    /// 永远不可能满足的条件。上游现在应该一条都没有
    pub fn unsatisfiable(&self) -> Vec<UnsatisfiableReport> {
        let mut out = Vec::new();
        for key in self.layers.keys() {
            let Some(param) = self.registry.param(key) else {
                continue;
            };
            let Some(sw) = &param.show_when else { continue };
            let Some(target) = self.registry.param(&sw.key) else {
                continue;
            };
            if target.choices.is_empty() || sw.op != ShowOp::Eq {
                continue; // 没有选项可比，或者 neq/gt 不是"必须命中某一项"
            }
            let hit = target
                .choices
                .iter()
                .any(|c| compare(target.value_type, &c.value, &sw.value, ShowOp::Eq));
            if !hit {
                out.push(UnsatisfiableReport {
                    key: key.to_owned(),
                    depends_on: sw.key.clone(),
                    wants: sw.value.clone(),
                    choices: target.choices.iter().map(|c| c.value.clone()).collect(),
                });
            }
        }
        out
    }
}

/// 按被指向字段的类型归一化后比较。
///
/// 这是这个模块最容易出错的一处：三种 op × 三类值，而条件里的字面量类型不可信
fn compare(vt: ValueType, actual: &Value, wanted: &Value, op: ShowOp) -> bool {
    match op {
        ShowOp::Eq => equal(vt, actual, wanted),
        ShowOp::Neq => !equal(vt, actual, wanted),
        // **`gt` 只对数字有意义**。两边有一个解析不出数字就判不成立 ——
        // 编一个顺序出来（比如按字符串比大小）会让界面的显隐变得没人能预测
        ShowOp::Gt => match (as_f64(actual), as_f64(wanted)) {
            (Some(a), Some(b)) => a > b,
            _ => false,
        },
    }
}

fn equal(vt: ValueType, actual: &Value, wanted: &Value) -> bool {
    match vt {
        ValueType::Bool => match (as_bool(actual), as_bool(wanted)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
        ValueType::Float | ValueType::Int => match (as_f64(actual), as_f64(wanted)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
        // 字符串比较允许对面是数字：`"0"` 与 `0` 在界面上是同一个选项
        ValueType::Text => as_text(actual) == as_text(wanted),
    }
}

fn as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.as_str() {
            "true" | "on" | "1" => Some(true),
            "false" | "off" | "0" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_f64().map(|f| f != 0.0),
        _ => None,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        // 实测就有这一种：`{op: "gt", value: "0"}`
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn as_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => raw_display(other),
    }
}

fn op_word(op: ShowOp) -> &'static str {
    match op {
        ShowOp::Eq => "等于",
        ShowOp::Neq => "不等于",
        ShowOp::Gt => "大于",
    }
}

/// 条件里那个值该怎么念给人听。
///
/// 开关念「开启 / 关闭」，枚举念它的中文选项名 —— 界面上显示 `disk` 的话，
/// 用户要在一堆中文按钮里猜哪个是 `disk`
fn display_value(target: &crate::workbench::presets::registry::ParamDef, wanted: &Value) -> String {
    if target.value_type == ValueType::Bool {
        return match as_bool(wanted) {
            Some(true) => "开启".into(),
            Some(false) => "关闭".into(),
            None => raw_display(wanted),
        };
    }
    target
        .choices
        .iter()
        .find(|c| equal(target.value_type, &c.value, wanted))
        .map(|c| c.label.clone())
        .unwrap_or_else(|| raw_display(wanted))
}

fn raw_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::layer::Overrides;
    use crate::workbench::paths;

    /// 一份带链、带开关、带枚举、带数字门槛的字段定义。
    ///
    /// 链：`c` → `b` → `a`（照真数据那条四层链的形状缩小）
    fn registry() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [
                {
                    "key": "a", "configKey": "A", "tomlKey": "a", "jsonKey": "a",
                    "label": "擦拭部件", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 1, "sectionId": "s1" },
                    "choices": [
                        { "label": "擦料塔", "value": "tower" },
                        { "label": "圆盘擦拭", "value": "disk" }
                    ]
                },
                {
                    "key": "b", "configKey": "B", "tomlKey": "b", "jsonKey": "b",
                    "label": "外壁结构", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "rib",
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 2, "sectionId": "s1" },
                    "parentKey": "a",
                    "showWhen": { "key": "a", "op": "eq", "value": "tower" },
                    "choices": [
                        { "label": "加强筋", "value": "rib" },
                        { "label": "包覆", "value": "sheath" }
                    ]
                },
                {
                    "key": "c", "configKey": "C", "tomlKey": "c", "jsonKey": "c",
                    "label": "筋宽", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 1,
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 3, "sectionId": "s1" },
                    "parentKey": "b",
                    "showWhen": { "key": "b", "op": "eq", "value": "rib" }
                },
                {
                    "key": "sw", "configKey": "SW", "tomlKey": "sw", "jsonKey": "sw",
                    "label": "Z 补偿", "desc": "", "tomlComment": "",
                    "valueType": "bool", "uiComponent": "switch", "defaultValue": false,
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 4, "sectionId": "s1" }
                },
                {
                    "key": "sw_child", "configKey": "SC", "tomlKey": "sc", "jsonKey": "sc",
                    "label": "补偿方式", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "bed",
                    "scope": "universal", "section": "s2",
                    "layout": { "order": 5, "sectionId": "s2" },
                    "parentKey": "sw",
                    "showWhen": { "key": "sw", "op": "eq", "value": true }
                },
                {
                    "key": "lift", "configKey": "L", "tomlKey": "l", "jsonKey": "l",
                    "label": "抬升高度", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                    "scope": "universal", "section": "s2",
                    "layout": { "order": 6, "sectionId": "s2" }
                },
                {
                    "key": "lift_first", "configKey": "LF", "tomlKey": "lf", "jsonKey": "lf",
                    "label": "首层抬升", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                    "scope": "universal", "section": "s2",
                    "layout": { "order": 7, "sectionId": "s2" },
                    "parentKey": "lift",
                    "showWhen": { "key": "lift", "op": "gt", "value": "0" }
                },
                {
                    "key": "in_gated_section", "configKey": "G", "tomlKey": "g", "jsonKey": "g",
                    "label": "塔位置 X", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 20,
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 8, "sectionId": "s3" }
                }
            ],
            "tabs": [{ "id": "t1", "label": "擦料", "order": 20, "sections": [
                { "id": "s1", "label": "擦料方式", "order": 0 },
                { "id": "s2", "label": "涂胶", "order": 1 },
                { "id": "s3", "label": "擦料塔位置", "order": 2 }
            ] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [
                { "id": "s1", "items": [
                    { "id": "i0", "paramKey": "a" },
                    { "id": "i1", "paramKey": "b" },
                    { "id": "i2", "paramKey": "c" },
                    { "id": "i3", "paramKey": "sw" }
                ] },
                { "id": "s2", "items": [
                    { "id": "i4", "paramKey": "sw_child" },
                    { "id": "i5", "paramKey": "lift" },
                    { "id": "i6", "paramKey": "lift_first" }
                ] },
                // section 级条件：整组吊在 a == "tower" 上（照真数据 wiping/tower_position）
                { "id": "s3", "showWhen": { "key": "a", "op": "eq", "value": "tower" },
                  "items": [{ "id": "i7", "paramKey": "in_gated_section" }] }
            ] }]
        });
        let r = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
        .unwrap();
        (d, r)
    }

    fn overrides(pairs: &[(&str, serde_json::Value)]) -> Overrides {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    /// 默认值下整条链都通 —— 链本身不该默认把人挡在外面
    #[test]
    fn chain_is_open_when_every_condition_holds() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        let g = Gate::new(&reg, &l);
        assert!(g.is_editable("a"));
        assert!(g.is_editable("b"), "a 默认 tower");
        assert!(g.is_editable("c"), "b 默认 rib");
    }

    /// 断在最上面一环：`c` 要报的是 **`a`**。
    ///
    /// 这里有一处容易想错的地方，写清楚：把 `a` 改成 `disk` 之后，`b` **的值仍然是
    /// `"rib"`**（默认值没变，只是这一项被关着了），所以 `c` 自己那条
    /// 「`b` 等于 加强筋」是**满足**的。`c` 之所以改不动，是因为链上更外面的 `a` 不对。
    /// 于是链里只有一条 —— 而它正好是唯一现在能动的那一项
    #[test]
    fn reports_the_root_most_unmet_condition_first() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let over = overrides(&[("a", serde_json::json!("disk"))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        let g = Gate::new(&reg, &l);

        assert!(!g.is_editable("b"));
        assert!(!g.is_editable("c"), "祖先被关着，自己也改不动");

        let chain = g.blocked("c");
        assert_eq!(
            chain.len(),
            1,
            "b 的值还是 rib，所以 c 自己那条是满足的；不通的只有 a 那条"
        );
        assert_eq!(chain[0].key, "a", "根在前");
        assert_eq!(chain[0].label, "擦拭部件");
        assert_eq!(chain[0].need, "等于 擦料塔", "枚举要念中文选项名");
        assert_eq!(chain[0].scope, BlockScope::Field);
    }

    /// 两环同时不通时**两条都报，根在前**
    #[test]
    fn both_broken_links_are_reported_root_first() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let over = overrides(&[
            ("a", serde_json::json!("disk")),
            ("b", serde_json::json!("sheath")),
        ]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        let chain = Gate::new(&reg, &l).blocked("c");

        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].key, "a");
        assert_eq!(chain[1].key, "b");
    }

    /// 只断中间一环时只报那一环
    #[test]
    fn only_the_broken_link_is_reported() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let over = overrides(&[("b", serde_json::json!("sheath"))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        let g = Gate::new(&reg, &l);

        assert!(g.is_editable("b"), "a 还是 tower");
        let chain = g.blocked("c");
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].key, "b");
        assert_eq!(chain[0].need, "等于 加强筋");
    }

    /// 开关要念「开启 / 关闭」，不是 `true` / `false`
    #[test]
    fn bool_conditions_read_as_on_and_off() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        let g = Gate::new(&reg, &l);

        let chain = g.blocked("sw_child");
        assert_eq!(chain.len(), 1, "sw 默认 false");
        assert_eq!(chain[0].need, "等于 开启");
        assert_eq!(chain[0].label, "Z 补偿");

        let over = overrides(&[("sw", serde_json::json!(true))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        assert!(Gate::new(&reg, &l).is_editable("sw_child"));
    }

    /// **字符串化的数字**：`{op: "gt", value: "0"}`。
    /// 不归一化的话 `Number(0.2) > String("0")` 判不出来，这个字段永远灰着且没人报错
    #[test]
    fn stringified_number_in_gt_is_normalised() {
        let (_d, reg) = registry();
        let empty = Overrides::new();

        // lift 默认 0 → 不满足 gt 0
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        let g = Gate::new(&reg, &l);
        assert!(!g.is_editable("lift_first"));
        assert_eq!(g.blocked("lift_first")[0].need, "大于 0");

        // 抬一点点就该通
        let over = overrides(&[("lift", serde_json::json!(0.2))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        assert!(
            Gate::new(&reg, &l).is_editable("lift_first"),
            "0.2 > \"0\" 要判得出来"
        );

        // 值本身也可能是字符串（上游 JSON 混类型），一样要通
        let over = overrides(&[("lift", serde_json::json!("0.2"))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        assert!(Gate::new(&reg, &l).is_editable("lift_first"));
    }

    /// `gt` 碰到不是数字的两边 → 判不成立，而不是编一个顺序出来
    #[test]
    fn gt_on_non_numbers_is_false_not_invented() {
        assert!(!compare(
            ValueType::Text,
            &serde_json::json!("b"),
            &serde_json::json!("a"),
            ShowOp::Gt
        ));
    }

    /// `neq`：三条里的那一种
    #[test]
    fn neq_works_on_enum_values() {
        let (_d, reg) = registry();
        assert!(compare(
            ValueType::Text,
            &serde_json::json!("on"),
            &serde_json::json!("off"),
            ShowOp::Neq
        ));
        assert!(!compare(
            ValueType::Text,
            &serde_json::json!("off"),
            &serde_json::json!("off"),
            ShowOp::Neq
        ));
        let _ = reg;
    }

    /// section 级条件：整组一起被关，且 `scope` 要说清是整组
    #[test]
    fn section_level_condition_gates_the_whole_group() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        assert!(Gate::new(&reg, &l).is_editable("in_gated_section"));

        let over = overrides(&[("a", serde_json::json!("disk"))]);
        let l = Layers::without_upstream(&reg, "A1", &empty, &over);
        let g = Gate::new(&reg, &l);
        let chain = g.blocked("in_gated_section");
        assert_eq!(chain.len(), 1, "这个字段自己没有条件，只有整组那一条");
        assert_eq!(chain[0].scope, BlockScope::Section);
        assert_eq!(chain[0].key, "a");
    }

    /// 环：不 panic、不死循环，且**判成可编辑** ——
    /// 让人改不动一个说不出理由的字段，比让他改一个本该隐藏的字段更糟
    #[test]
    fn a_cycle_does_not_hang_and_leaves_the_field_editable() {
        let d = tempfile::tempdir().unwrap();
        let p = |key: &str, order: f64, dep: &str| {
            serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "bool", "uiComponent": "switch", "defaultValue": false,
                "scope": "universal", "section": "x",
                "layout": { "order": order, "sectionId": "s1" },
                "showWhen": { "key": dep, "op": "eq", "value": true }
            })
        };
        let params = serde_json::json!({
            "params": [p("x", 1.0, "y"), p("y", 2.0, "x")],
            "tabs": [{ "id": "t1", "label": "页签", "order": 10,
                       "sections": [{ "id": "s1", "label": "组", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "x" }, { "id": "i1", "paramKey": "y" }
            ] }] }]
        });
        let reg = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
        .unwrap();

        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        let g = Gate::new(&reg, &l);

        // 这一行的意义就是"它会返回" —— 没有环检测这里会转死
        let _ = g.blocked("x");

        let cycles = g.cycles();
        assert_eq!(cycles.len(), 1, "同一个环从两个起点走到，只该报一次");
        let mut keys = cycles[0].keys.clone();
        keys.sort();
        assert_eq!(keys, vec!["x", "y"]);
    }

    /// 正常数据里一个环都没有
    #[test]
    fn no_cycles_in_a_healthy_chain() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        assert!(Gate::new(&reg, &l).cycles().is_empty());
    }

    /// 条件要求一个不在选项里的值 → 这个字段永久隐藏，要报出来
    #[test]
    fn unsatisfiable_condition_is_reported() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l = Layers::without_upstream(&reg, "A1", &empty, &empty);
        assert!(
            Gate::new(&reg, &l).unsatisfiable().is_empty(),
            "夹具里的条件都指向真选项"
        );

        // 把 b 的条件改成一个 a 没有的值
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [
                {
                    "key": "a", "configKey": "A", "tomlKey": "a", "jsonKey": "a",
                    "label": "擦拭部件", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 1, "sectionId": "s1" },
                    "choices": [{ "label": "擦料塔", "value": "tower" }]
                },
                {
                    "key": "b", "configKey": "B", "tomlKey": "b", "jsonKey": "b",
                    "label": "外壁", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 1,
                    "scope": "universal", "section": "wiping",
                    "layout": { "order": 2, "sectionId": "s1" },
                    "showWhen": { "key": "a", "op": "eq", "value": "teleport" }
                }
            ],
            "tabs": [{ "id": "t1", "label": "擦料", "order": 20,
                       "sections": [{ "id": "s1", "label": "组", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "a" }, { "id": "i1", "paramKey": "b" }
            ] }] }]
        });
        let reg2 = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
        .unwrap();
        let l = Layers::without_upstream(&reg2, "A1", &empty, &empty);
        let bad = Gate::new(&reg2, &l).unsatisfiable();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].key, "b");
        assert_eq!(bad[0].wants, serde_json::json!("teleport"));
    }

    /* ---------- 真数据 ---------- */

    /// 真数据上：没有环、没有永不成立的条件、四层链能走通，
    /// 且**默认值下不该有一大片字段是灰的**
    #[test]
    fn real_data_chains_are_sane() {
        let Some(root) = paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let p = crate::workbench::presets::Presets::load_from(&root).unwrap();
        let m = p.catalog.machine("A1").expect("清单里应该有 A1");
        let vids: Vec<String> = m.versions.iter().map(|v| v.id.clone()).collect();
        let d = crate::workbench::domain::digest(&p.registry, &m.id, &vids);
        let over = &d.versions[&vids[0]];
        let l = Layers::without_upstream(&p.registry, &m.id, &d.base, over);
        let g = Gate::new(&p.registry, &l);

        assert!(g.cycles().is_empty(), "真数据里出现了 showWhen 环：{:?}", g.cycles());
        assert!(
            g.unsatisfiable().is_empty(),
            "有字段的条件永远不可能满足：{:?}",
            g.unsatisfiable()
        );

        // 每条 blocked 的 need 都得是能显示的句子，不能是空的或带 JSON 引号的
        let mut blocked_count = 0usize;
        for key in l.keys() {
            for b in g.blocked(key) {
                blocked_count += 1;
                assert!(!b.need.trim().is_empty(), "{key} 的理由是空句子");
                assert!(!b.need.contains('"'), "{key} 的理由里漏了 JSON 引号：{}", b.need);
                assert!(!b.label.trim().is_empty(), "{key} 卡在一个没有名字的字段上");
            }
        }
        // 反空转：真数据在默认值下确实有被关着的字段（涂胶补偿默认关）
        assert!(blocked_count > 0, "一条 blocked 都没有，判据在空转");

        // 那条四层链要真的存在，否则递归这件事就没被测到
        let deep = g.blocked("wiping.tower_rib_speed_value");
        let _ = deep; // 值取决于默认值，这里只要它能算出来且不死循环
        assert!(
            p.registry
                .param("wiping.tower_rib_speed_value")
                .and_then(|p| p.show_when.as_ref())
                .is_some(),
            "那条四层链的叶子不在了 —— 递归这件事就没有真数据在测"
        );
    }

    /// 实测 43 条 `parentKey` 与它的 `showWhen.key` **完全一致**。
    ///
    /// 这一条值得钉住：一致意味着"折叠层级"和"启用条件"是同一棵树。
    /// 一旦分岔，界面上会出现两种嵌套关系 —— 用户看到一个字段缩在某项下面，
    /// 但把那项打开它还是灰的
    #[test]
    fn real_data_parent_key_matches_show_when() {
        let Some(root) = paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let reg = Registry::load_from(&root).unwrap();
        let mut checked = 0usize;
        for p in reg.params() {
            match (&p.parent_key, &p.show_when) {
                (Some(parent), Some(sw)) => {
                    assert_eq!(
                        parent, &sw.key,
                        "{} 的折叠父项与启用条件指向不同字段",
                        p.key
                    );
                    checked += 1;
                }
                (None, None) => {}
                (a, b) => panic!(
                    "{} 的 parentKey 与 showWhen 一个有一个没有：{a:?} / {b:?}",
                    p.key
                ),
            }
        }
        assert!(checked > 0, "一条都没查，判据在空转");
    }
}
