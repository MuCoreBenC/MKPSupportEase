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
//! # 现在切到哪一步了（b04 Task 8）
//!
//! | 谁 | 读哪一层 |
//! |---|---|
//! | 机型与版本的**清单**（有哪些机型、每台有哪些版本、版本六个元字段） | **这一层** |
//! | 字段定义（74 条）、界面布局、资源与套餐清单 | 还在 `upstream` |
//!
//! 清单换源的后果一句话：**清单是可写的**。「加一个版本」保存之后它真的出现在树上，
//! 而不是留下一个谁都不认的孤儿文件（那是换源之前的死胡同，见 `app/storage.rs` 里
//! `a_new_version_shows_up_on_the_tree_after_saving`）。
//!
//! 剩下那半边（三层取值改读这一层的 `[params.machineVariants]`、删掉我们自造的
//! `workbench/machines|versions/*.json`）是 Task 8 后面几步。
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
pub use registry::{ParamRegistry, ShowOp, ShowWhen, TabMeta, UiComponent, ValueType};

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

/// 把字符串包成一个**单引号（字面量）表示**的 TOML 值。
///
/// # 为什么不能直接用 `toml_edit::value(s)`
///
/// 它输出双引号，而真数据的字符串**全用单引号**（`name = '标准版'`）。
/// 每改一个字段就多一处不一致，文件会**逐渐漂成混合风格** —— 那是不可逆的熵增，
/// 而且每次 diff 都多一行噪音。
///
/// 这一条不是审美：`one_edit_only` 那条判据说的是「别处没动」，
/// 而引号属于「这一处」，所以它**通得过** —— 但保真的本意是
/// 「只有我改的那个值变了」，不包括表示形式。
/// 这个漂移是靠一条专门去问的断言才发现的，测试全绿并不代表它没发生。
///
/// # 怎么做到的
///
/// `toml_edit` 没有公开 API 能直接设置「表示形式」（`set_repr_unchecked` 是私有的），
/// 所以走一条更朴素的路：**解析一小段 `k = '值'`**，让 `toml_edit` 自己按原文建出带
/// 单引号表示的值，再把它取出来。
///
/// 这条路比私有 API 更可靠：**解析成功本身就证明那个表示是合法的**。
/// 解析失败（说明 `can_be_literal` 判漏了）就退回双引号 —— 自带兜底，不会写出坏 TOML。
///
/// # 退回双引号的条件
///
/// TOML 的字面量字符串**不支持任何转义**：里面不能有单引号，也不能跨行或含控制字符。
/// 含这些的值只能用双引号 + 转义 —— 那时风格让位于正确。
/// 多行 G-code 走的就是这一支（真数据里它们本来就是带 `\n` 转义的双引号字符串）。
pub(crate) fn literal_str(s: &str) -> toml_edit::Item {
    if can_be_literal(s) {
        if let Ok(doc) = format!("k = '{s}'").parse::<DocumentMut>() {
            if let Some(item) = doc.get("k") {
                return item.clone();
            }
        }
    }
    toml_edit::value(s)
}

pub(crate) fn can_be_literal(s: &str) -> bool {
    !s.contains('\'') && !s.chars().any(char::is_control)
}

/// 一次编辑只动了一处：**除了中间那一段，前后都逐字节不变。**
///
/// 返回 `(被删掉的那段, 被插入的那段)`。
///
/// # 为什么不用「新文本以旧文本为前缀」
///
/// 那一条把**实现细节**写进了判据 —— 它说的是「改动发生在末尾」，
/// 而我们真正想验的是「别处没动」。两句话在"追加"这个场景下碰巧等价，
/// 换成**删除**或**中间插入**就分道扬镳：删一个 `[[versions]]` 块时前缀断言直接失效，
/// 那时候只能退而写一条更弱的判据，而更弱的判据放得过真正的损坏。
///
/// 这一条对三种改动都成立，所以新增 / 删除 / 改一个字段可以共用它 ——
/// 机型文件与 `param_registry.toml` 也共用同一条（b04 Task 10 起）。
#[cfg(test)]
pub(crate) fn one_edit_only(before: &str, after: &str) -> (String, String) {
    let pre = before
        .bytes()
        .zip(after.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(before.len().min(after.len()));
    let max_suf = before.len().min(after.len()) - pre;
    let suf = (0..max_suf)
        .position(|i| {
            before.as_bytes()[before.len() - 1 - i] != after.as_bytes()[after.len() - 1 - i]
        })
        .unwrap_or(max_suf);

    // 切在字符边界上，否则中文会被劈成半个字
    let cut = |s: &str, lo: usize, hi: usize| {
        let mut lo = lo;
        while lo < s.len() && !s.is_char_boundary(lo) {
            lo -= 1;
        }
        let mut hi = hi;
        while hi > lo && !s.is_char_boundary(hi) {
            hi += 1;
        }
        s[lo..hi].to_owned()
    };
    (
        cut(before, pre, before.len() - suf),
        cut(after, pre, after.len() - suf),
    )
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
        let out = Self {
            catalog: Catalog::load_from(root)?,
            registry: ParamRegistry::load_from(root)?,
            root: root.to_path_buf(),
        };
        out.check_cross_consistency()?;
        Ok(out)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 写一个机型 / 版本的值，**写完落盘**（原子写）。
    ///
    /// 单条入口。**一次保存改一批值时不要循环调它** —— 那会把 56 KB 的
    /// `param_registry.toml` 写 N 遍，而且中途失败会留下"改了一半"的文件。
    /// 批量走 [`Self::apply_values`]。
    ///
    /// # 为什么 owner 必须在这一层查
    ///
    /// `machineVariants` 的键指向机型与版本，而那两样在另一个文件里。
    /// 写进一个不存在的键之后，下一次 [`Self::load_from`] 会被
    /// [`Self::check_cross_consistency`] 判成 `Corrupted` —— **整个工作台起不来**。
    /// 所以这一步不是"顺手校验"，是防止把数据写成一个自己都读不回来的状态。
    ///
    /// 写完**重读盘再由调用方返回界面**：界面显示的必须是落盘结果（doc §1 头一条纪律）
    pub fn set_variant(
        &mut self,
        key: &str,
        owner: &str,
        value: &serde_json::Value,
    ) -> Result<(), AppError> {
        self.apply_values(&[(key.to_owned(), owner.to_owned(), Some(value.clone()))])
    }

    /// 清空一个机型 / 版本的值（删键），写完落盘
    pub fn clear_variant(&mut self, key: &str, owner: &str) -> Result<(), AppError> {
        self.apply_values(&[(key.to_owned(), owner.to_owned(), None)])
    }

    /// 一批值一次落盘。`(字段 key, owner, 值)`，值为 `None` = 清空（删键）。
    ///
    /// # 为什么要有批量入口
    ///
    /// 一次「保存」常常是一片改动（改了一个机型基底，顺带调了三个版本）。
    /// 逐条调 [`Self::set_variant`] 的代价是把 56 KB 的文件写 N 遍 ——
    /// 不只是慢：**中途失败会留下一个"改了前三条、没改后两条"的文件**，
    /// 而那种状态没有任何判据能描述它。
    ///
    /// 这里的做法是：**先把 owner 全部校验完，再改内存里的文档，最后写一次**。
    /// 任何一条 owner 不合法 → 一个字节都不写。
    ///
    /// 注意「改内存」这一步之后若写盘失败，内存与盘会不一致 ——
    /// 调用方（`app::Ctx`）在保存后一律重读盘，所以那个窗口不会被看见
    pub fn apply_values(
        &mut self,
        edits: &[(String, String, Option<serde_json::Value>)],
    ) -> Result<(), AppError> {
        if edits.is_empty() {
            return Ok(());
        }
        // **先全查一遍再动手**：一条不合法就整批不写
        for (key, owner, _) in edits {
            self.check_owner_exists(owner)?;
            if self.registry.param(key).is_none() {
                return Err(AppError::invalid_argument(format!("字段定义里没有 {key}")));
            }
        }
        for (key, owner, value) in edits {
            match value {
                Some(v) => self.registry.set_variant(key, owner, v)?,
                None => self.registry.clear_variant(key, owner)?,
            }
        }
        self.registry.write_back()
    }

    /// `A1` 要是真机型，`A1:FAST` 要是真机型的真版本
    fn check_owner_exists(&self, owner: &str) -> Result<(), AppError> {
        let (machine, version) = match owner.split_once(':') {
            Some((m, v)) => (m, Some(v)),
            None => (owner, None),
        };
        let m = self.catalog.machine(machine).ok_or_else(|| {
            AppError::not_found(format!("没有机型 {machine}")).with_detail(
                "值只能写在真机型或真版本上 —— 写别的会让下一次加载直接失败".to_owned(),
            )
        })?;
        if let Some(v) = version {
            if !m.versions.iter().any(|x| x.id == v) {
                return Err(AppError::not_found(format!("{machine} 没有版本 {v}"))
                    .with_detail("版本清单在机型文件的 [[versions]] 里".to_owned()));
            }
        }
        Ok(())
    }

    /// 跨文件那一条：`machineVariants` 的键必须是真的机型或真的机型版本。
    ///
    /// 这是**最容易悄悄坏掉**的一条。键写错一个字母（`"A1_Mini:FAST"`）不会有任何报错 ——
    /// 三步归并（doc §3.3）会把它当成一台不存在的机型的覆盖，然后它既不进任何机型的基底、
    /// 也不进任何版本的覆盖，那个值就凭空消失了。界面上看起来只是"这一项用的是出厂默认"。
    ///
    /// 它只能在这一层查：字段定义在 `registry`，机型与版本在 `catalog`，
    /// 分开读的话没有任何时刻能把两边放在一起看
    pub fn check_cross_consistency(&self) -> Result<(), AppError> {
        let machines: Vec<&str> = self
            .catalog
            .machines()
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        let keys = self.catalog.machine_keys();

        for p in self.registry.params() {
            for (name, map) in [
                ("machineVariants", &p.machine_variants),
                ("machineMinVariants", &p.machine_min_variants),
                ("machineMaxVariants", &p.machine_max_variants),
            ] {
                for k in map.keys() {
                    let ok = if k.contains(':') {
                        keys.contains(k)
                    } else {
                        machines.contains(&k.as_str())
                    };
                    if !ok {
                        return Err(AppError::corrupted(format!(
                            "{} 的 {name} 里有一个认不出的键：{k}",
                            p.key
                        ))
                        .with_detail(
                            "键要么是机型 id（A1），要么是机型:版本（A1:FASTV3.3）。\
                             认不出的键会在归并时凭空消失，界面上只看得到「这一项用的是出厂默认」",
                        ));
                    }
                }
            }
        }
        Ok(())
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
