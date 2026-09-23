//! 我们自己那份预设数据 —— `<repo>/presets/*.toml` 的读写。
//!
//! # 这一层为什么存在
//!
//! 它**取代**旧的 `upstream/`（那一层读 `mkpse-presets/content/*.json`，也就是别人的构建产物）。
//! 前提变了两次，记在 `.comate/specs/b04-panel-successor/doc.md`：
//!
//! 1. `content/*.json` 是产物，源是 `source/*.toml` —— 实测 `source/machines/A1.toml`
//!    与 `content/machine_catalog.json` 逐字段对应。
//! 2. mkppanel 与旧客户端都退役 ⇒ 没有现存消费者 ⇒ **不欠任何人字节级兼容**。
//!
//! 所以数据搬进我们项目（`<repo>/presets/`），这一层**读它也写它**。
//!
//! # 一条判据先于一切写入
//!
//! **读进来不改再写出去，字节必须不变。**
//!
//! 它保护的是我们自己的数据：否则第一次点保存就可能把六个机型文件的格式搅乱，
//! 而那种损坏在 diff 里是一片红 —— 真正改了什么反而看不出来。
//!
//! 实现上它不是靠"我复刻了原作者的 writer"，而是靠 [`toml_edit`] 保留原文：
//! 只有被改的那一处会变，别处连空行和引号风格都不动。**保真在构造上成立，不靠自觉。**

pub mod catalog;
pub mod registry;

use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::workbench::paths;

pub use catalog::{Brand, Catalog, Machine, MachineField, MachineVersion, VersionField, Zone};
pub use registry::{Layout, LayoutItem, LayoutSection, LayoutTab, ParamRegistry};

/// 读一个源文件。读不到要**说出是哪个文件** —— 只说"读不到"没法据以行动
pub(crate) fn read(path: &Path) -> Result<String, AppError> {
    std::fs::read_to_string(path).map_err(|e| {
        AppError::not_found(format!("读不到 {}", path.display())).with_detail(e.to_string())
    })
}

/// 解析成保留原文的文档。**文本已经在手上时用这个**，别为了解析再读一遍盘 ——
/// 读两遍之间文件可能变，那种不一致查起来最费劲
pub(crate) fn parse_text(text: &str, path: &Path) -> Result<DocumentMut, AppError> {
    text.parse::<DocumentMut>().map_err(|e| {
        AppError::corrupted(format!("{} 不是合法 TOML", path.display())).with_detail(e.to_string())
    })
}

/// 一次加载的全部预设数据。
///
/// 目前是机型目录 + 字段定义 + 界面布局。还没搬的是 `assets/` / `bundles/` /
/// `preset_registry.toml` —— 它们属于「资源与套餐」，按「先只搬核心」留到后面
/// （机型文件里的 `defaultBundle` / `presetFile` 因此是悬空引用，**这是刻意的**：
/// 它们只是字符串，编辑机型时照样显示照样改，只有生成与校验才需要解析）。
pub struct Presets {
    pub catalog: Catalog,
    pub registry: ParamRegistry,
    pub layout: Layout,
    root: PathBuf,
}

impl Presets {
    /// 真仓库。定位不到时**返回错误而不是空数据** ——
    /// 用空数据装成能跑，会让人以为"我们没有机型"，而那和"读不出来"是两件事
    pub fn load() -> Result<Self, AppError> {
        let root = paths::presets_root().ok_or_else(|| {
            AppError::not_found("找不到我们的预设数据目录 presets/").with_detail(
                "期望 <repo>/presets/registry/param_registry.toml 存在；\
                 数据从 mkpse-presets/source/ 搬过来（见 b04 doc §1.2）"
                    .to_owned(),
            )
        })?;
        Self::load_from(&root)
    }

    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        Ok(Self {
            catalog: Catalog::load_from(root)?,
            registry: ParamRegistry::load_from(root)?,
            layout: Layout::load_from(root)?,
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 删掉这个版本之后，`param_registry.toml` 里哪些字段会留下孤儿引用。
    ///
    /// # 为什么这件事必须跨层问
    ///
    /// `[params.machineVariants]` 的键有两种形状：`A1`（整台机型）和 `A1:STANDARD`（某一版）。
    /// 后者指向一个具体版本 —— **版本删了，键还在**，于是它成了指向不存在对象的引用。
    ///
    /// 这种损坏的恶性在于它**不报错**：解析照样通过，只是那一项在那台机器上悄悄不生效了。
    /// 所以删之前要先问一遍并把结果摆给人看，而不是删完再让他自己发现。
    ///
    /// 返回的是**字段的 key**（`wiping.wiper_x` 这种），因为那是用户能据以行动的单位：
    /// 他要去那一页把那几项清掉或改掉。
    pub fn orphans_if_version_removed(&self, machine_id: &str, version_id: &str) -> Vec<String> {
        let needle = format!("{machine_id}:{version_id}");
        self.registry
            .params()
            .iter()
            .filter(|p| p.machine_variants.contains_key(&needle))
            .map(|p| p.key.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真数据上的对齐检查。定位不到就**说清楚这条没执行**，不当成通过 ——
    /// 静默 skip 是这个项目里反复踩过的坑
    fn real() -> Option<Presets> {
        let root = paths::presets_root()?;
        Some(Presets::load_from(&root).expect("presets/ 里的机型目录读不齐"))
    }

    #[test]
    fn the_real_catalog_loads() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        // 搬进来的是 6 个机型，其中 A2L 是四折空
        assert_eq!(p.catalog.machines().len(), 6, "搬进来的机型数");
        assert!(!p.catalog.brands().is_empty());

        let a1 = p.catalog.machine("A1").expect("A1 必须在");
        assert_eq!(a1.display, "A1");
        assert_eq!(a1.brand, "Bambu Lab");
        assert_eq!(a1.versions.len(), 3, "A1 三个版本");
        assert_eq!(a1.versions[0].id, "STANDARD");
        assert_eq!(a1.versions[0].name, "标准版");
        assert_eq!(a1.external_aliases, vec!["A1C", "A1F"]);

        // A2L：有机型有版本，但没尺寸、没禁区。**不许 panic，也不许被跳过**
        let a2l = p.catalog.machine("A2L").expect("A2L 必须在");
        assert!(!a2l.has_dimensions, "A2L 实测没有 [dimensions]");
        assert_eq!(a2l.versions.len(), 1);

        // 禁区只有三台机器有
        assert!(p.catalog.zones("P1S").is_some());
        assert!(p.catalog.zones("A1").is_none(), "A1 没有禁区文件");
    }

    /// 孤儿查询：**这是删版本独有的风险判据**。
    ///
    /// 它验的不是"删对了"，而是"删之前说得出会坏掉什么" ——
    /// `[params.machineVariants]` 里 `A1:FAST` 这样的键在版本删掉之后会变成
    /// 指向不存在对象的引用，而那种损坏**不报错**：解析照样通过，
    /// 只是那一项在那台机器上悄悄不生效了
    #[test]
    fn removing_a_referenced_version_reports_what_will_be_orphaned() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };

        // 先在真数据里找一个「机型:版本」形状的键，用它来验 —— 找不到就说明前提变了
        let sample = p
            .registry
            .params()
            .iter()
            .flat_map(|x| x.machine_variants.keys())
            .find(|k| k.contains(':'))
            .cloned();
        let Some(key) = sample else {
            panic!("真数据里一个 `机型:版本` 形状的 machineVariants 键都没有 —— 这条判据失去对象，要重写");
        };
        let (machine, version) = key.split_once(':').expect("刚判过含冒号");

        let orphans = p.orphans_if_version_removed(machine, version);
        assert!(
            !orphans.is_empty(),
            "{key} 明明被引用着，却报不出任何会变成孤儿的字段"
        );
        // 报出来的必须是真字段的 key（用户要据此去那一页处理）
        for k in &orphans {
            assert!(p.registry.param(k).is_some(), "{k} 不是一个真字段");
        }

        // 反空转：一个不存在的版本**不该**报出任何孤儿。
        // 少了这一条，一个「永远返回全部字段」的实现也会让上面那句通过
        let none = p.orphans_if_version_removed(machine, "NO_SUCH_VERSION_XYZ");
        assert!(none.is_empty(), "不存在的版本却报出了孤儿：{none:?}");
        // 同理：整台机型那种键（没有冒号）不该被算成某一版的引用
        let whole = p.orphans_if_version_removed(machine, "");
        assert!(whole.is_empty(), "空版本号匹配到了东西：{whole:?}");
    }

    /// 别名不许和任何机型 ID 相撞 —— 撞了的话"按 ID 找机型"会有两个答案
    #[test]
    fn aliases_do_not_collide_with_machine_ids() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        for m in p.catalog.machines() {
            for a in &m.external_aliases {
                assert!(
                    p.catalog.machine(a).is_none(),
                    "别名 {a}（{}）和一个真机型 ID 撞了",
                    m.id
                );
            }
        }
    }
}
