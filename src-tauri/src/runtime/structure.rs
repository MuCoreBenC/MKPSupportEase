//! **结构代次**与「最低客户端版本」（`docs/PUBLISH-ARCHITECTURE.md` §5.3）。
//!
//! # 它解决的那件事
//!
//! 「这批云端数据，哪个客户端起才读得懂？」—— 客户端**不许硬吃**它读不懂的新结构，
//! 发布侧也**不许猜**兼容性（作者定的硬规矩：宁可发布不了，也不能让程序自作聪明）。
//!
//! 所以这件事拆成两半，**各自都有显式的真值**：
//!
//! ```text
//! ① 结构签名 structrue signature    —— 机器算得出来的那半（这块的 [signature_from]）
//! ② 签名 → 最低正式客户端版本       —— 人必须显式登记的那半（[RuleTable]）
//! ```
//!
//! 查不到 ② 就是 **Fail、禁止发布**（发布闸的 ⑫）；闸会把该登记的那一行直接印出来。
//!
//! # 签名是怎么算出来的（★ 这块的规矩都在这）
//!
//! 签名**不是**对整份 catalog 取哈希 —— 那样改一个参数名、加一台机型都会变，于是
//! 每条规则都要重新登记，规则表立刻变成没人看的噪音。
//!
//! 签名只收「**客户端读不动的那些结构事实**」：**必填字段的路径 + 它的 JSON 形态**。
//!
//! 而「必填」这件事**从类型真值探出来**，不靠人维护一张字段表：
//!
//! > 拿一份真实的 catalog JSON，把某个字段删掉，再解析一遍 ——
//! > 还能解析成功就说明它有 `#[serde(default)]`（可选），
//! > 解析失败就说明它没它不行（必填）。
//!
//! 这条推论是**构造性**的，不是启发式：一份能解析成功的 JSON，它缺席的键必然是
//! 可选的。于是：
//!
//! - **加一个可选字段**（`#[serde(default)]`）→ 删它照样解析得过 → 不进签名 → **签名不变** ✅
//! - **加一个必填字段 / 把可选的改成必填** → 删它解析失败 → 进签名 → 签名变
//!   → 规则表里查不到 → **禁止发布**（逼你显式声明"哪个客户端起支持它"）✅
//! - **改字段类型**（`string` → `number`）→ 形态变了 → 签名变 ✅
//!
//! ## 类型看不出来的那半：`STRUCTURE_EPOCH`
//!
//! 「路径语义改了」（`catalog.path` 从 `mkp/…` 基准改成发布根基准就是一次）、
//! 「asset kind 的语义改了」、「客户端该按哪条规则解释这份数据」——这些**类型上
//! 一模一样**，机器算不出来。所以留一个**人显式声明**的代次 [`STRUCTURE_EPOCH`]：
//! 那种改动发生时必须手工 +1，于是一定会逼出一条新的规则登记。
//!
//! 判定原则一句话：**机器能算的自动算，算不出来的必须有人签过字。**
//!
//! # 采样从哪来（为什么签名与"用哪份数据"无关）
//!
//! 上面那套探法只问「删了还能不能解析」，答案**只取决于 serde 属性**，与你拿哪一份
//! catalog 去探无关。唯一要小心的是"探针点必须存在"（空数组探不到里面）——
//! 所以模板写成 `machines[*].zones[*]` 这种**扫描式**路径，判据
//! `every_probe_template_resolves` 再钉一次"每个模板都真的落到了东西上"。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::runtime::catalog::Catalog;

/// **结构代次**：机器看不出来的那类结构变化，由人在这里显式 +1。
///
/// # 什么时候必须 +1（照 §5.3）
///
/// - **路径语义**：`catalog.path` 的基准变了（例：`mkp/…` → 发布根，2026-10-04 那次）
/// - **asset kind 语义**：`type` / `delivery` 的含义或映射规则变了
/// - **客户端读取方式**：客户端该按哪条规则解释这份数据变了（加载顺序、回落链改了）
/// - **数据的解释方式**：某个字段的含义变了但类型没变（例：`path` 从"相对内部根"
///   变成"相对发布根"，两边都是 string）
///
/// # 什么时候**不要**动它
///
/// 加可选字段、加内容（机型 / 资产 / 参数）、改名字、改默认值 —— 这些签名自己会处理，
/// 或者根本不构成"客户端读不动"。乱 +1 的代价是规则表里多一条没人看懂的登记。
pub const STRUCTURE_EPOCH: u32 = 2;

/// 本构建**能读**的结构签名清单（客户端那一半的"结构能力声明"）。
///
/// # 为什么是清单而不是版本号
///
/// Dev 场景（§5.3 正文）：本地代码已经支持结构 B，但正式安装包还是 0.7.0。
/// 拿"当前 App 的版本号"当"这个构建能不能理解这个结构"用，会把开发环境自己锁死
/// —— 因为那个版本号的安装包**还没发出来**。
///
/// 所以顺序是：**先看能力（这份清单），再看版本（[`RuleTable`] 里登记的最低版本）**。
/// 见 [`can_read`]。
///
/// ★ 改了结构（签名变了）而忘了登记新签名 → `cargo test` 的
/// `supported_signatures_cover_the_current_structure` 立刻红，且把该抄的值印出来。
pub const SUPPORTED_SIGNATURES: &[&str] = &["cb1080919d39b2bd", "2242174c52e8a9b6"];

/// 本构建的版本号（唯一来源 = `Cargo.toml`，不再由三处各写一份）。
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 规则表在**发布根**下的文件名（`<repo>/presets/structure-signatures.toml`）。
pub const RULES_FILE: &str = "structure-signatures.toml";

/// 规则表相对**仓库根**的路径（构建期从 repo 走这条路）。
pub const RULES_REL_PATH: &str = "presets/structure-signatures.toml";

/// 探针模板：覆盖「客户端会读的每一个类型」。
///
/// `[*]` = **扫描式**的"第一个存在这种东西的实例"（数组空着就换个实例找）。
/// 写成扫描式而不是 `[0]`，是因为"第 0 台机型没有禁区"这种数据事实不该让探针落空。
///
/// 加一个类型进客户端契约 = 这里加一行；删一个 = 删一行（**那会改签名、逼你重新登记**
/// —— 这是刻意的：覆盖面本身也是契约的一部分）。
pub const PROBE_TEMPLATES: &[&str] = &[
    "machines[*]",
    "machines[*].versions[*]",
    "machines[*].dimensions",
    "machines[*].zones[*]",
    "assets[*]",
    "bundles[*]",
    "plates[*]",
    "brands[*]",
    "files[*]",
    "registry",
    "registry.params[*]",
    "registry.tabs[*]",
    "registry.tabs[*].sections[*]",
    "registry.layout[*]",
];

/* ---------- 签名 ---------- */

/// 结构签名：`epoch` + 必填字段清单（**模板化路径** + JSON 形态）的稳定序列化摘要。
///
/// ★ 输入里用的是**模板路径**（`machines[*].zones[*].id`）而不是具体下标 ——
/// 否则"前面插一台机型"就会让签名变，规则表会被无关的数据变化刷爆。
pub fn signature_of(epoch: u32, required: &[(String, String)]) -> String {
    let mut input = format!("mkpse-structure/epoch={epoch}\n");
    for (path, kind) in required {
        input.push_str(path);
        input.push('=');
        input.push_str(kind);
        input.push('\n');
    }
    crate::runtime::catalog::hex(&Sha256::digest(input.as_bytes()))[..16].to_owned()
}

/// 从一份真实的 catalog JSON 算签名。
pub fn signature_from(sample: &Value) -> String {
    signature_of(STRUCTURE_EPOCH, &required_shape(sample))
}

/// 把一份 catalog 值摊成 JSON 再算签名（调用方最常见的形态）。
pub fn signature_of_catalog(catalog: &Catalog) -> String {
    let sample = serde_json::to_value(catalog).unwrap_or(Value::Null);
    signature_from(&sample)
}

/// 一个探针格（签名与判据都从这一套里出，两处不各写一遍）。
#[derive(Debug, Clone)]
pub struct Probe {
    /// 模板化路径（`machines[*].zones[*].id`）—— **进签名的是它**，与具体下标无关
    pub template_path: String,
    /// 具名路径（`machines[2].zones[0].id`）—— 删字段要用它
    pub concrete_path: String,
    /// 删掉它还能解析成功吗（= 有 `#[serde(default)]`，可选）
    pub optional: bool,
    /// JSON 形态
    pub shape: String,
}

/// 走一遍全部探针模板，逐个字段回答"它可选吗"。
pub fn probes(sample: &Value) -> Vec<Probe> {
    let mut out = Vec::new();
    for template in PROBE_TEMPLATES {
        let Some(concrete) = resolve(sample, template) else {
            continue;
        };
        let Some(node) = value_at(sample, &concrete) else {
            continue;
        };
        let Some(obj) = node.as_object() else {
            continue;
        };
        // 一个模板只探第一个实例：必填性是**类型**的属性，与实例无关。
        for (key, value) in obj {
            let concrete_path = format!("{concrete}.{key}");
            let mut probe = sample.clone();
            let optional = !(delete_field(&mut probe, &concrete_path)
                && serde_json::from_value::<Catalog>(probe).is_err());
            out.push(Probe {
                template_path: format!("{template}.{key}"),
                concrete_path,
                optional,
                shape: shape_of(value).to_owned(),
            });
        }
    }
    out
}

/// 必填（`#[serde(default)]` 缺省不了）的字段清单：`(模板路径, JSON 形态)`。
///
/// 见模块头的推理：删掉它还能解析成功 = 可选；失败 = 必填。
pub fn required_shape(sample: &Value) -> Vec<(String, String)> {
    probes(sample)
        .into_iter()
        .filter(|p| !p.optional)
        .map(|p| (p.template_path, p.shape))
        .collect()
}

/// 探针模板今天落到了哪些（判据用：解析不到的模板 = 签名覆盖面少了一块）。
pub fn resolved_templates(sample: &Value) -> Vec<&'static str> {
    PROBE_TEMPLATES
        .iter()
        .filter(|t| resolve(sample, t).is_some())
        .copied()
        .collect()
}

/// JSON 形态。**只分种类**，不记具体值 —— 签名是结构的函数，不是内容的函数。
fn shape_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "none",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// 把模板解析成**第一个**落得实的具名路径（`machines[*].zones[*]` → `machines[2].zones[0]`）。
fn resolve(sample: &Value, template: &str) -> Option<String> {
    let mut frontier: Vec<(String, &Value)> = vec![(String::new(), sample)];
    for seg in template.split('.') {
        let (key, star) = match seg.strip_suffix("[*]") {
            Some(k) => (k, true),
            None => (seg, false),
        };
        let mut next: Vec<(String, &Value)> = Vec::new();
        for (path, node) in &frontier {
            let Some(child) = node.get(key) else {
                continue;
            };
            let here = if path.is_empty() {
                key.to_owned()
            } else {
                format!("{path}.{key}")
            };
            if star {
                if let Some(arr) = child.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        next.push((format!("{here}[{i}]"), item));
                    }
                }
            } else {
                next.push((here, child));
            }
        }
        if next.is_empty() {
            return None;
        }
        frontier = next;
    }
    frontier.into_iter().next().map(|(path, _)| path)
}

/// 只读取路径上的值（支持 `key` 与 `key[i]` 两种段）。
fn value_at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }
    let mut cur = value;
    for (key, idx) in segments(path) {
        if !key.is_empty() {
            cur = cur.as_object()?.get(key)?;
        }
        if let Some(i) = idx {
            cur = cur.as_array()?.get(i)?;
        }
    }
    Some(cur)
}

/// 可变取路径上的值（与 [`value_at`] 同一套段语义）。
fn value_at_mut<'a>(value: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    if path.is_empty() {
        return Some(value);
    }
    let mut cur = value;
    for (key, idx) in segments(path) {
        if !key.is_empty() {
            cur = cur.as_object_mut()?.get_mut(key)?;
        }
        if let Some(i) = idx {
            cur = cur.as_array_mut()?.get_mut(i)?;
        }
    }
    Some(cur)
}

/// 删掉路径末端那一格（对象删键、数组删元素）。删不动返回 false。
fn delete_field(value: &mut Value, path: &str) -> bool {
    let (parent_path, last) = match path.rsplit_once('.') {
        Some((p, l)) => (p, l),
        None => ("", path),
    };
    let (key, idx) = match last.split_once('[') {
        Some((k, rest)) => (k, rest.trim_end_matches(']').parse::<usize>().ok()),
        None => (last, None),
    };
    let Some(parent) = value_at_mut(value, parent_path) else {
        return false;
    };
    match idx {
        Some(i) => parent
            .as_array_mut()
            .map(|a| i < a.len() && a.remove(i).is_object())
            .unwrap_or(false),
        None => parent
            .as_object_mut()
            .map(|o| o.remove(key).is_some())
            .unwrap_or(false),
    }
}

/// `a.b[0].c` → `[("a", None), ("b", Some(0)), ("c", None)]`
fn segments(path: &str) -> Vec<(&str, Option<usize>)> {
    path.split('.')
        .map(|seg| match seg.split_once('[') {
            Some((k, rest)) => (k, rest.trim_end_matches(']').parse::<usize>().ok()),
            None => (seg, None),
        })
        .collect()
}

/* ---------- 规则表（人显式登记的那半） ---------- */

/// 一条登记：这个结构签名**最低需要哪个正式客户端版本**。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// 结构签名（16 位 hex，见 [`signature_of`]）
    pub signature: String,
    /// 最低**正式**客户端版本。**不要求这个版本已经发布**（§5.3：Dev 场景）
    pub min_client: String,
    /// 为什么是这个版本（人写给下一个人看的）
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RulesFile {
    #[serde(default)]
    signature: Vec<Rule>,
}

/// 签名 → 最低客户端版本的表（`presets/structure-signatures.toml`）。
///
/// **它是人工维护的**，这是设计的一部分：机器算得出来的只有签名，"这个变化要哪个
/// 客户端"是产品决定。查不到 → 发布闸 ⑫ 红 → 禁止发布。
#[derive(Debug, Clone, Default)]
pub struct RuleTable {
    entries: Vec<Rule>,
    /// 签名 → 下标。查表走它，顺手也把"表里有没有重复签名"这件事变成可查的
    index: HashMap<String, usize>,
}

impl RuleTable {
    /// 从一个发布根（`<repo>/presets`）加载。**文件不存在 = 空表**，不是错 ——
    /// "还没登记过任何一代"是合法状态（它会让 ⑫ 红，那正是该有的表现）。
    pub fn load(publish_root: &Path) -> Result<RuleTable, AppError> {
        let file = publish_root.join(RULES_FILE);
        match std::fs::read_to_string(&file) {
            Ok(text) => Self::parse(&text, &file),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RuleTable::default()),
            Err(e) => {
                Err(AppError::io(format!("读不出 {}", file.display())).with_detail(e.to_string()))
            }
        }
    }

    /// 从仓库根加载（构建期用；仓库根 = 发布根的上一层）。
    pub fn load_from_repo(repo_root: &Path) -> Result<RuleTable, AppError> {
        Self::load(&repo_root.join(crate::runtime::catalog::REPO_PUBLISH_ROOT))
    }

    pub fn parse(text: &str, path: &Path) -> Result<RuleTable, AppError> {
        let parsed: RulesFile = toml_edit::de::from_str(text).map_err(|e| {
            AppError::corrupted(format!("{} 读不成结构规则表", path.display()))
                .with_detail(e.to_string())
        })?;
        let mut index = HashMap::new();
        for (i, rule) in parsed.signature.iter().enumerate() {
            if rule.signature.trim().len() != 16 {
                return Err(AppError::corrupted(format!(
                    "{} 里有一条签名不是 16 位：{:?}",
                    path.display(),
                    rule.signature
                )));
            }
            if rule.min_client.trim().is_empty() {
                return Err(AppError::corrupted(format!(
                    "{} 里签名 {} 没写 minClient —— 没写就是没登记，别用空串糊过去",
                    path.display(),
                    rule.signature
                )));
            }
            if index.insert(rule.signature.clone(), i).is_some() {
                return Err(AppError::corrupted(format!(
                    "{} 里签名 {} 登记了两遍 —— 同一个签名只该有一个答案",
                    path.display(),
                    rule.signature
                )));
            }
        }
        Ok(RuleTable {
            entries: parsed.signature,
            index,
        })
    }

    pub fn entries(&self) -> &[Rule] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 查一个签名对应的登记。`None` = **没有规则 = 不准发**。
    pub fn rule_of(&self, signature: &str) -> Option<&Rule> {
        self.index.get(signature).and_then(|i| self.entries.get(*i))
    }

    /// 该登记的下一行长什么样（发布闸红的时候直接印给人看）。
    pub fn suggestion(&self, signature: &str) -> String {
        format!(
            "[[signature]]\nsignature = '{signature}'\nminClient = '?'  # 哪个正式版起读得懂这一代\n\
             note = '这一代改了什么'"
        )
    }
}

/* ---------- 客户端那一半：能不能读 ---------- */

/// 这个构建能不能读这批数据（**先看结构能力，再看版本**）。
///
/// 顺序是刻意的：
///
/// - Dev 构建（版本号还没发出去）走前半段 —— 代码已经有新结构的处理能力，放行；
/// - 正式老客户端两段都不满足 —— 拒绝，如实说"有新版 SupportEase"。
///
/// `min_client` 为 `None` = 上游没声明最低版本（规则表还没登记那一代）：
/// 那就只剩"签名命中"这一条路，宁可拒绝也不猜。
pub fn can_read(signature: &str, min_client: Option<&str>) -> bool {
    SUPPORTED_SIGNATURES.contains(&signature)
        || min_client.is_some_and(|m| version_at_least(APP_VERSION, m))
}

/// `actual >= required`（按 `major.minor.patch` 比；`-dev` 这类后缀忽略）。
pub fn version_at_least(actual: &str, required: &str) -> bool {
    let a = version_parts(actual);
    let r = version_parts(required);
    for i in 0..3 {
        let x = a.get(i).copied().unwrap_or(0);
        let y = r.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    true
}

fn version_parts(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .take(3)
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// 规则表在仓库里该住哪（报错与闸的"去修"都指这一处）。
pub fn rules_path_in(publish_root: &Path) -> PathBuf {
    publish_root.join(RULES_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri 上面就是仓库根")
            .to_path_buf()
    }

    /// 真目录的 JSON（探针与判据的共同输入）。
    fn real_sample() -> Value {
        let bytes = crate::runtime::EMBEDDED_CATALOG;
        serde_json::from_slice(bytes).expect("嵌进二进制的 catalog 读不成 JSON")
    }

    /// **每个模板都得落到东西上**。
    ///
    /// 落空 = 签名少覆盖一块，而"少一块"这件事**不报错也会静默变小**。
    /// 红了的处理只有两条：补数据，或者显式删掉那个模板（那会改签名 → 逼一次重新登记）。
    #[test]
    fn every_probe_template_resolves() {
        let sample = real_sample();
        let resolved = resolved_templates(&sample);
        let missing: Vec<&str> = PROBE_TEMPLATES
            .iter()
            .filter(|t| !resolved.contains(*t))
            .copied()
            .collect();
        assert!(
            missing.is_empty(),
            "这些模板在真数据里落不到东西上：{missing:?} —— 签名的覆盖面少了一块，\
             要么补数据，要么显式删掉那个模板（删掉会改签名，那是对的）"
        );
    }

    /// 探针清单自洽：每一条都**真的删得掉**，且「必填 / 可选」的判定与删完能不能解析一致。
    #[test]
    fn the_probes_are_self_consistent() {
        let sample = real_sample();
        let all = probes(&sample);
        assert!(
            all.len() > 40,
            "探针只有 {} 条，太少 —— 探法大概没工作",
            all.len()
        );
        assert!(
            all.iter().any(|p| !p.optional),
            "一条必填都没有 —— 探法大概没工作"
        );
        assert!(
            all.iter().any(|p| p.optional),
            "一条可选都没有 —— 那说明 catalog 里没有一个带 default 的字段，不对劲"
        );
        for p in &all {
            let mut probe = sample.clone();
            assert!(
                delete_field(&mut probe, &p.concrete_path),
                "{} 删不掉 —— 具名路径该是能落地的",
                p.concrete_path
            );
            let parsed = serde_json::from_value::<Catalog>(probe).is_ok();
            assert_eq!(
                parsed, p.optional,
                "{} 的判定与删完的结果不一致 —— 探法自相矛盾",
                p.concrete_path
            );
        }
    }

    /// ★ **核心性质**：加一个字段（`#[serde(default)]` 那种可选扩展）签名不变。
    ///
    /// 这里用"塞一个类型不认识的键"当代理 —— 两种情形在这次探法下**完全同形**：
    /// 多出来的键删掉之后都能解析成功（前者靠 `default`，后者靠 serde 忽略未知键），
    /// 所以都不会进必填清单。
    #[test]
    fn an_added_optional_field_does_not_change_the_signature() {
        let sample = real_sample();
        let before = signature_from(&sample);

        let mut with_field = sample.clone();
        with_field["someBrandNewOptionalField"] = Value::String("扩展".to_owned());
        assert_eq!(before, signature_from(&with_field), "顶层加字段不该改签名");

        let mut nested = sample.clone();
        nested["machines"][0]["someBrandNewOptionalField"] = Value::from(1);
        assert_eq!(before, signature_from(&nested), "机型里加字段不该改签名");
    }

    /// 结构真的变了就要变签名（这是"逼登记"的机制本体）。
    #[test]
    fn the_epoch_is_part_of_the_signature() {
        let shape = required_shape(&real_sample());
        assert_ne!(
            signature_of(1, &shape),
            signature_of(2, &shape),
            "epoch 必须进签名 —— 它就是「机器看不出来的那类变化」唯一能落的地方"
        );
    }

    /// 必填性变了签名就要变（拿"把某个必填字段从清单里去掉"当代理）。
    #[test]
    fn losing_a_required_field_changes_the_signature() {
        let shape = required_shape(&real_sample());
        let mut fewer = shape.clone();
        fewer.pop();
        assert_ne!(
            signature_of(STRUCTURE_EPOCH, &shape),
            signature_of(STRUCTURE_EPOCH, &fewer)
        );
    }

    /// ★ **签名与"拿哪份 catalog 去探"无关**（两份真实来源各算一遍）。
    #[test]
    fn the_signature_does_not_depend_on_which_sample_it_is_taken_from() {
        let embedded = signature_from(&real_sample());
        let rebuilt = signature_of_catalog(
            &Catalog::build_from_repo(&repo_root()).expect("仓库里那份构建不出来"),
        );
        assert_eq!(
            embedded, rebuilt,
            "同一个类型在两份数据上算出两个签名 —— 探针落到了内容上，那是 bug"
        );
    }

    /// ★ **加字段不破坏旧客户端**：把这一代新增的两个字段删掉，旧客户端照样读得动。
    /// （`#[serde(default)]` 的承诺在构造上成立，这条判据把它钉住。）
    #[test]
    fn an_old_client_can_still_read_a_catalog_with_the_new_fields() {
        let mut old_shaped = real_sample();
        assert!(
            delete_field(&mut old_shaped, "structureSignature"),
            "这一代该有 structureSignature"
        );
        assert!(
            delete_field(&mut old_shaped, "minClientVersion"),
            "这一代该有 minClientVersion"
        );
        serde_json::from_value::<Catalog>(old_shaped)
            .expect("旧形态（没有这两个字段）必须还能解析");
    }

    /// ★ **逼登记**：改了结构却忘了声明"这个构建读得懂" → 这条红，且把该抄的值印出来。
    #[test]
    fn supported_signatures_cover_the_current_structure() {
        let current = signature_from(&real_sample());
        assert!(
            SUPPORTED_SIGNATURES.contains(&current.as_str()),
            "结构签名是 {current}，`SUPPORTED_SIGNATURES` 里没有它 —— \
             本构建读不懂自己这一代结构。把它加进那份清单（那是在声明：这个构建有处理它的代码）"
        );
    }

    /// ★ **逼登记（规则表那一半）**：这一代必须在规则表里有主。
    #[test]
    fn the_rule_table_registers_the_current_signature() {
        let current = signature_from(&real_sample());
        let table = RuleTable::load(&repo_root().join("presets")).expect("规则表读不出来");
        assert!(
            table.rule_of(&current).is_some(),
            "{RULES_REL_PATH} 里没有签名 {current} —— 「没有规则 = 不准发」。\
             要登记的就是这一行：\n{}",
            table.suggestion(&current)
        );
    }

    #[test]
    fn the_real_rule_table_is_well_formed() {
        let table = RuleTable::load(&repo_root().join("presets")).expect("规则表读不出来");
        assert!(!table.is_empty(), "规则表是空的");
        for rule in table.entries() {
            assert_eq!(rule.signature.len(), 16, "签名该是 16 位");
            assert!(!rule.min_client.trim().is_empty(), "没写 minClient");
            assert!(!rule.note.trim().is_empty(), "{} 没写 note", rule.signature);
        }
    }

    #[test]
    fn a_missing_rule_table_is_an_empty_table_not_an_error() {
        let table = RuleTable::load(Path::new("/definitely/not/here")).expect("缺席该是空表");
        assert!(table.is_empty());
        assert!(table.rule_of("0123456789abcdef").is_none());
    }

    #[test]
    fn a_broken_rule_table_is_corrupted_not_silently_empty() {
        let bad = "[[signature]]\nsignature = 'tooshort'\nminClient = '1.0.0'\n";
        let err = RuleTable::parse(bad, Path::new("t.toml")).expect_err("该报错");
        assert!(format!("{err:?}").contains("16 位"), "{err:?}");

        let dup = "[[signature]]\nsignature = '0123456789abcdef'\nminClient = '1.0.0'\n\
                   [[signature]]\nsignature = '0123456789abcdef'\nminClient = '2.0.0'\n";
        let err = RuleTable::parse(dup, Path::new("t.toml")).expect_err("该报错");
        assert!(format!("{err:?}").contains("两遍"), "{err:?}");

        let blank = "[[signature]]\nsignature = '0123456789abcdef'\nminClient = ''\n";
        let err = RuleTable::parse(blank, Path::new("t.toml")).expect_err("该报错");
        assert!(format!("{err:?}").contains("minClient"), "{err:?}");
    }

    /// Dev 场景本体：**签名命中就放行，不看版本号**（安装包还没发出去也不锁自己）。
    #[test]
    fn a_dev_build_is_let_in_by_capability_not_by_version() {
        let current = SUPPORTED_SIGNATURES[0];
        assert!(can_read(current, Some("99.0.0")), "签名命中就该放行");
        assert!(can_read(current, None), "没登记最低版本，签名命中照样放行");
    }

    /// 老客户端两段都不满足 → 拒绝（"有新版 SupportEase"）。
    #[test]
    fn an_older_client_is_refused_the_data_it_cannot_read() {
        let unknown = "ffffffffffffffff";
        assert!(!can_read(unknown, Some("99.0.0")), "版本不够就该拒");
        // 版本够（前端"最低版本已满足"）就放行 —— 那是规则表的声明
        assert!(can_read(unknown, Some("0.0.1")), "版本够了就放行");
        // 上游没说最低版本 → 只剩能力这一条路，宁可拒绝也别猜
        assert!(!can_read(unknown, None), "没声明最低版本又不认识 → 拒");
    }

    #[test]
    fn versions_compare_by_three_parts() {
        assert!(version_at_least("0.8.0", "0.7.0"));
        assert!(version_at_least("0.7.0", "0.7.0"));
        assert!(version_at_least("0.8.0-dev", "0.7.0"), "-dev 后缀不该干扰");
        assert!(!version_at_least("0.7.0", "0.8.0"));
        assert!(!version_at_least("1.0.0", "1.0.1"));
        assert!(version_at_least("1.0.1", "1.0.0"));
    }
}
