//! 字段定义与界面布局：`presets/registry/param_registry.toml` + `presets/layout_schema.toml`。
//!
//! # 模型是复用的，不是重写的
//!
//! 这两个文件的键名是 **camelCase**（`tomlKey` / `defaultValue` / `machineFilter` / `sectionId`），
//! 与旧层读的那份 JSON **一模一样** —— 因为那份 JSON 就是从这个 TOML 构建出来的。
//! 所以 `ParamDef` / `TabMeta` / `SectionMeta` 直接从 `upstream::registry` 借过来用，
//! 只换 loader。**两份模型迟早分岔，一份模型两个 loader 不会。**
//!
//! （`upstream` 这个模块名已经名不副实，它的类型会在清场那一轮搬家。
//! 现在先借用而不是复制 —— 复制才是真正会留下来的债。）
//!
//! # 同一份文本解析两遍，这是刻意的
//!
//! - `DocumentMut`：保原文与排版，**写回靠它**
//! - serde 反序列化：拿到好用的模型，**读靠它**
//!
//! 56 KB 解析两遍的代价可以忽略，换来的是"改一个字段不会重排整个文件"。

use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::workbench::upstream::registry::{ParamDef, TabMeta};

/// `param_registry.toml` 的反序列化目标
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryFile {
    #[serde(default)]
    updated: String,
    #[serde(default)]
    tabs: Vec<TabMeta>,
    #[serde(default)]
    params: Vec<ParamDef>,
}

/// 字段定义表。74 条字段 + 页签/分组元信息
pub struct ParamRegistry {
    params: Vec<ParamDef>,
    tabs: Vec<TabMeta>,
    updated: String,
    doc: DocumentMut,
    file: PathBuf,
}

impl std::fmt::Debug for ParamRegistry {
    /// 手写：`DocumentMut` 打出来是整个 56 KB 文件
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParamRegistry")
            .field("params", &self.params.len())
            .field("tabs", &self.tabs.len())
            .field("updated", &self.updated)
            .finish()
    }
}

impl ParamRegistry {
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join("registry").join("param_registry.toml");
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let parsed: RegistryFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(format!("{} 的结构对不上字段定义模型", file.display()))
                .with_detail(e.to_string())
        })?;
        Ok(Self {
            params: parsed.params,
            tabs: parsed.tabs,
            updated: parsed.updated,
            doc,
            file,
        })
    }

    pub fn params(&self) -> &[ParamDef] {
        &self.params
    }

    pub fn tabs(&self) -> &[TabMeta] {
        &self.tabs
    }

    pub fn param(&self, key: &str) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.key == key)
    }

    pub fn updated(&self) -> &str {
        &self.updated
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// 写回用的文本。**没改过就逐字节等于读进来那份**
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn write_back(&self) -> Result<(), AppError> {
        atomic_write(&self.file, self.to_toml().as_bytes())
    }
}

/* ---------- 界面布局 ---------- */

/// `layout_schema.toml` 只有三层 id + `paramKey`：页签 → 分组 → 条目。
///
/// 它与 `param_registry.toml` 里的 `[params.layout]` **是两件事**：
/// 那个是参数自己声明的归属，这个是界面实际的排布与顺序。两者对不上时以哪个为准，
/// 属于旧层已有的启动断言，这一轮不动。
#[derive(Debug, Deserialize)]
struct LayoutFile {
    #[serde(default)]
    tabs: Vec<LayoutTab>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutTab {
    pub id: String,
    #[serde(default)]
    pub sections: Vec<LayoutSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSection {
    pub id: String,
    #[serde(default)]
    pub items: Vec<LayoutItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutItem {
    pub id: String,
    pub param_key: String,
}

pub struct Layout {
    tabs: Vec<LayoutTab>,
    doc: DocumentMut,
    file: PathBuf,
}

impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("tabs", &self.tabs.len())
            .finish()
    }
}

impl Layout {
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join("layout_schema.toml");
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let parsed: LayoutFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(format!("{} 的结构对不上布局模型", file.display()))
                .with_detail(e.to_string())
        })?;
        Ok(Self {
            tabs: parsed.tabs,
            doc,
            file,
        })
    }

    pub fn tabs(&self) -> &[LayoutTab] {
        &self.tabs
    }

    /// 布局里一共排了多少个条目 —— 它应当等于"可见字段数"
    pub fn item_count(&self) -> usize {
        self.tabs
            .iter()
            .flat_map(|t| t.sections.iter())
            .map(|s| s.items.len())
            .sum()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn write_back(&self) -> Result<(), AppError> {
        atomic_write(&self.file, self.to_toml().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::paths;

    fn load() -> Option<(ParamRegistry, Layout)> {
        let root = paths::presets_root()?;
        Some((
            ParamRegistry::load_from(&root).expect("字段定义读不通"),
            Layout::load_from(&root).expect("布局读不通"),
        ))
    }

    /// 74 条字段全读到，而且关键字段真的落进模型 ——
    /// 只断言条数的话，一个全是默认值的空壳也能通过
    #[test]
    fn all_seventy_four_params_load_with_their_fields() {
        let Some((r, _l)) = load() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert_eq!(r.params().len(), 74, "字段条数");
        assert!(!r.tabs().is_empty());
        assert!(!r.updated().is_empty(), "updated 没读到");

        // 一条普通数值字段：单位、范围、步进、界面归属
        let p = r.param("toolhead.MKP_retract").expect("回抽长度必须在");
        assert_eq!(p.label, "回抽长度");
        assert_eq!(p.toml_key, "MKP_retract");
        assert_eq!(p.unit.as_deref(), Some("mm"));
        assert_eq!(p.min, Some(-50.0));
        assert_eq!(p.max, Some(50.0));
        assert_eq!(p.layout.section_id, "motion");

        // 四个子表各至少有一条真数据 —— 它们是最容易在换 loader 时静默丢掉的部分
        assert!(
            r.params().iter().any(|p| !p.choices.is_empty()),
            "[[params.choices]] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| p.show_when.is_some()),
            "[params.showWhen] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| !p.machine_variants.is_empty()),
            "[params.machineVariants] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| !p.machine_filter.is_empty()),
            "machineFilter 没被拆成数组"
        );
    }

    /// 布局的三层结构读通，条目数与可见字段数对得上
    #[test]
    fn the_layout_lists_every_visible_param_once() {
        let Some((r, l)) = load() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert!(!l.tabs().is_empty());
        assert!(l.item_count() > 0, "布局里一个条目都没有");

        // 布局引用的每个 paramKey 都得真的存在 —— 否则界面上会排一个空位
        for t in l.tabs() {
            for s in &t.sections {
                for it in &s.items {
                    assert!(
                        r.param(&it.param_key).is_some(),
                        "布局里的 {} 指向一个不存在的字段（{}/{}）",
                        it.param_key,
                        t.id,
                        s.id
                    );
                }
            }
        }

        // 同一个字段不许在布局里出现两次
        let mut seen = std::collections::BTreeSet::new();
        for t in l.tabs() {
            for s in &t.sections {
                for it in &s.items {
                    assert!(
                        seen.insert(it.param_key.clone()),
                        "{} 在布局里出现了两次",
                        it.param_key
                    );
                }
            }
        }
    }

    /// 往返保真：这两个文件也归这条判据管。
    ///
    /// `param_registry.toml` 是 56 KB、含多行 G-code 字符串与一堆浮点，
    /// **它是保真最容易破的地方** —— 如果哪天换回"反序列化再重新序列化"，
    /// 这条会第一个红
    #[test]
    fn registry_and_layout_survive_a_round_trip() {
        let Some((r, l)) = load() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        for (path, round) in [(r.file(), r.to_toml()), (l.file(), l.to_toml())] {
            let original = std::fs::read_to_string(path).expect("刚读过的文件");
            assert_eq!(
                original,
                round,
                "{} 读进来再写出去变了。差异从第 {} 个字节开始",
                path.display(),
                original
                    .bytes()
                    .zip(round.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(original.len().min(round.len())),
            );
        }
    }

    /// 反空转：多行 G-code 字符串是保真的硬骨头，专门盯一条
    #[test]
    fn a_multiline_gcode_value_is_preserved_verbatim() {
        let Some((r, _l)) = load() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = r
            .params()
            .iter()
            .find(|p| !p.machine_variants.is_empty() && p.toml_key.contains("gcode"))
            .expect("装载/卸载 G-code 那两条必须在");
        let v = p
            .machine_variants
            .values()
            .find_map(|v| v.as_str())
            .expect("至少一个机型变体是字符串");
        assert!(v.contains('\n'), "这条本该是多行 G-code：{v:?}");
        // 原文里它是带 \n 转义的双引号字符串；写回之后那一段必须还在
        assert!(
            r.to_toml().contains("G92 E0"),
            "写回之后 G-code 内容不见了"
        );
    }
}
