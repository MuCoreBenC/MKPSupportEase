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
//! # 现在切到哪一步了（b05 Task 10）
//!
//! | 谁 | 读哪一层 |
//! |---|---|
//! | 机型与版本的**清单**（有哪些机型、每台有哪些版本、版本六个元字段） | **这一层** |
//! | 资产定义（21 条）、套餐定义（5 条） | **这一层** |
//! | 字段定义（74 条）、界面布局 | **这一层**（b04 Task 9 起） |
//! | `preset_registry.toml`（交付索引：`nozzle` / `layerHeight`） | 还没搬 —— Task 12 |
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

pub mod assets;
pub mod bundles;
pub mod catalog;
/// 按**参数 key** 改一份 MKP 预设 TOML 里的一个值（保真写回）。参数页底座 ②
pub mod patch;
pub mod registry;
/// 三层取值（出厂 → 机型基底 → 版本覆盖）与来源层。**纯计算**，不碰盘。
///
/// 原住 `workbench/domain/layer.rs`。搬过来的理由：客户端预设页要显示每项参数的
/// 有效值与来源，而那正是这一份算法 —— **代码共用一份，两边各自的数据根喂给它**。
pub mod resolve;

use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use crate::error::AppError;

pub use assets::{Asset, AssetKind, Assets};
pub use bundles::{Bundle, Bundles};
pub use catalog::{
    Brand, BrandField, Catalog, Dimensions, Machine, MachineField, MachineVersion, Plate,
    PlateFrame, VersionField, Zone,
};
pub use registry::{
    LayoutTab, ParamDef, ParamRegistry, SectionMeta, ShowOp, ShowWhen, TabMeta, UiComponent,
    ValueType,
};
pub use resolve::{no_overrides, Layers, Level, Origin, Overrides, ValueOrigin};

/// **测试专用的**仓库预设根：`<repo>/presets`。
///
/// 这一层**不决定数据根** —— 根由调用方给（[`Presets::load_from`]）。这个函数只为
/// 「拿真数据当判据」的测试存在，且 `CARGO_MANIFEST_DIR` 是编译期的仓库路径，
/// **运行时不会有任何人调用它**（正式客户端跑在没有仓库的机器上）。
#[cfg(test)]
pub(crate) fn repo_presets_root() -> Option<PathBuf> {
    // CARGO_MANIFEST_DIR = <repo>/src-tauri
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("presets");
    // 判据是**标志文件**而不是 `is_dir()`：一个同名空目录不该骗过定位
    root.join("registry")
        .join("param_registry.toml")
        .is_file()
        .then_some(root)
}

/// **测试专用的**仓库资产载荷根：`<repo>/presets/assets`。
///
/// 与 [`repo_presets_root`] 同一条理由：只为"拿真数据当判据"的测试存在。
/// 定义与载荷都在 `presets/` 下（`presets/assets.toml` ↔ `presets/assets/<kind>/`，
/// 2026-10-03 起同根），但运行时的载荷根**仍由调用方给**
/// （[`Presets::set_asset_root`]）—— 客户端那一侧根本没有载荷根。
#[cfg(test)]
pub(crate) fn repo_assets_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("presets")
        .join("assets");
    root.is_dir().then_some(root)
}

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

/// MKP 产物的文件名：`<机型 id>-<版本 id 小写>.toml`（`A1` + `FASTV3.3` → `A1-fastv3.3.toml`）。
///
/// # 为什么这里有一份副本
///
/// 权威实现在 `crates/preset/src/generate.rs::file_name`（由 `preset_file_name` 重导出），
/// 工作台那一侧转调它。**客户端不能转调** —— 那个 crate 挂在 `workbench` feature 下，
/// 是个 optional 依赖，正式客户端里根本不存在。
///
/// 所以这里放一份**逐字副本**，由 [`tests::mkp_file_name_matches_the_generator`]
/// 钉住 `generate.rs` 自己那几条样例。漂移的表现很硬：消费端按这个名字找不到文件，
/// 而界面上只表现为"这个版本没有产物"，没有任何一步会报错。
pub fn mkp_file_name(machine: &str, version: &str) -> String {
    format!("{machine}-{}.toml", version.to_lowercase())
}

/// 一个"可选引用"字段：**trim 后非空才算引用**。
///
/// 空串与 `None` 同义（"没有"），这是 8.6 定下的口径 —— 旧数据里 `recommendedBundle = ''`
/// 读成"填过但填了个空"，查存在性时必须跳过而不是报错
fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
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

/// 一条资产被谁引用（b05 Task 9.4，套餐那一档 Task 10）。
///
/// **没有 `versions` 字段**：版本今天不直接引用资产（`recommendedBundle` → 套餐 → 资产，
/// 是间接的）。留一个永远为空的字段比说清"还没有"更糟 —— 它会被当成"查过了，没有版本在用"。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetUsage {
    /// 直接引用它的机型（`image` / `icon` 字段写着这个 id）
    pub machines: Vec<String>,
    /// 引用它的套餐（`assetRefs` 写着这个 id 的套餐 id）。
    /// **归属不是引用**：`p1s-icon` 归 P1S，但引用它的是 P1S / P2S / X1C 三台 ——
    /// 这里列的是后者
    pub bundles: Vec<String>,
}

/// 一次加载的全部预设数据。
///
/// 现在是机型目录 + 资产定义 + 套餐定义 + 字段定义 + 界面布局。还没搬的只有
/// `preset_registry.toml` —— 它是**交付索引**（消费端按 `nozzle` / `layerHeight` 挑预设），
/// 属于 Task 12；在这里存一份就是第二份会过期的真相。
///
/// 机型文件里的 `defaultBundle` / `recommendedBundle` 在 Task 10 之前是**刻意的悬空引用**
/// （只是字符串）；现在它们必须能解析到 `bundles.toml` 里的一条套餐，
/// 加载期查（[`Self::check_bundle_refs`]）。
pub struct Presets {
    pub catalog: Catalog,
    /// 资产域①层（b05 Task 8）
    pub assets: Assets,
    /// 套餐域①层（b05 Task 10）。`defaultBundle` / `recommendedBundle` 指向的是它的条目
    pub bundles: Bundles,
    pub registry: ParamRegistry,
    root: PathBuf,
}

impl Presets {
    /// 从一个**由调用方给定的**根读进来。
    ///
    /// # 为什么不在这里决定根
    ///
    /// 这是客户端与工作台之间那条边界的落点：**代码共用一份，数据根必须是两套**。
    ///
    /// - 工作台：`Presets::load_from(<repo>/presets)` —— 仓库里的开发源数据；
    /// - 客户端：**不再走这里**（2026-10-01 换源收口）——它只读释放进内部根的 catalog，
    ///   definition 由发布构建从同一批源算出并序列化（类型直接复用本层的 serde 形态）。
    ///
    /// 这一层只要一个 root，**不猜、不 fallback、不读环境变量**。定位根的职责
    /// 分别住在 `workbench::paths`（仓库）与 `runtime::release`（catalog 释放）。
    ///
    /// # 读不到就报错，不用空数据装成能跑
    ///
    /// 空的机型列表与"读不出来"是两件事，后者要能被说出来。
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let out = Self {
            catalog: Catalog::load_from(root)?,
            assets: Assets::load_from(root)?,
            bundles: Bundles::load_from(root)?,
            registry: ParamRegistry::load_from(root)?,
            root: root.to_path_buf(),
        };
        out.check_cross_consistency()?;
        Ok(out)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 重读**同一个根**，载荷根跟着走。
    ///
    /// 重读只有这一条出口：盘上刚被别处改过（机型页直接写 `presets/`）时要靠它刷新。
    /// 走 [`Self::load_from`] 重新拼一个的话，**载荷根会悄悄丢掉** ——
    /// 表现是全部资产突然变成"文件不在"，而那种错查起来最费劲。
    pub fn reload(&self) -> Result<Self, AppError> {
        let mut p = Self::load_from(&self.root)?;
        if let Some(asset_root) = self.assets.asset_root() {
            p.set_asset_root(asset_root);
        }
        Ok(p)
    }

    /// 资产**载荷**根（`path` 那一栏的基准）。**可选，且由调用方给**。
    ///
    /// 定义（`presets/assets.toml`）与载荷（资产文件本体）是两个概念，即使今天同根：
    /// 工作台给的载荷根是仓库的 `presets/assets/`，而客户端这一轮
    /// **只释放定义、不释放文件本体**，所以它压根没有载荷根 —— 这时
    /// [`Assets::present`] 一律为 false，界面上表现为"文件还没到"，
    /// 而不是一个查不出来的状态。
    pub fn set_asset_root(&mut self, root: &Path) {
        self.assets.set_asset_root(root);
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

    /// **机型引用的资产 id 必须存在**（b05 Task 9 / 8.6）。
    ///
    /// 今天机型的 `image` **一律为空**（整机图 2026-10-01 从台账剥离、搬进
    /// `src/app/assets/`），实际走这条的是 `icon`；image 这一支留着是因为字段还在，
    /// 哪天有产品级的机型图要登记回来（那时 `kind_of_asset` 也得跟着放开）。
    ///
    /// 存在性**分两级**（Task 11.6 同一口径）：
    ///
    /// - id 认不出来 ⇒ **error**。那是打错了字，与 `machineVariants` 的键写错同一类：
    ///   界面上只会表现为"那张图没了"，没有任何东西报错；
    /// - id 认得出、但文件还没搬进来 ⇒ **warning**（Task 11.1）。允许"先把关联建起来、
    ///   文件后补"（导入一张图之前就要能选机型），但**不许静默** ——
    ///   现在由 `wb_assets` 的 `present: false` 说出来，校验层接手后升成一条 warning。
    fn check_asset_refs(&self) -> Result<(), AppError> {
        for m in self.catalog.machines() {
            for (field, value) in [
                ("image", &m.image),
                ("imageVariant", &m.image_variant),
                ("icon", &m.icon),
            ] {
                let Some(id) = value.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                if self.assets.get(id).is_none() {
                    return Err(AppError::corrupted(format!(
                        "机型 {} 的 {field} 指向一个不存在的资产：{id}",
                        m.id
                    ))
                    .with_detail(
                        "资产定义在 presets/assets.toml。id 打错的后果是那张图/图标静默消失"
                            .to_owned(),
                    ));
                }
            }
            /* 版本级那一格（`[[versions]] image`）与机型级同一把尺：打错字的后果一样
            —— 那一版的图静默回落成机型图，看不出是打错了还是没配 */
            for v in &m.versions {
                let Some(id) = v.image.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                if self.assets.get(id).is_none() {
                    return Err(AppError::corrupted(format!(
                        "机型 {} 的版本 {} 的 image 指向一个不存在的资产：{id}",
                        m.id, v.id
                    ))
                    .with_detail(
                        "资产定义在 presets/assets.toml。版本图打错的后果是**静默回落成机型图** \
                         —— 界面上看不出是打错了字还是压根没配"
                            .to_owned(),
                    ));
                }
            }
        }
        /* 品牌图（`brands.toml` 的 `logo`）同样是跨文件引用：打错字的后果是**静默回落内置字标** */
        for b in self.catalog.brands() {
            let Some(id) = b.logo.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            if self.assets.get(id).is_none() {
                return Err(AppError::corrupted(format!(
                    "品牌 {} 的 logo 指向一个不存在的资产：{id}",
                    b.id
                ))
                .with_detail(
                    "资产定义在 presets/assets.toml。品牌图打错的后果是**静默回落成内置字标** \
                     —— 界面上看不出是打错了字还是压根没配"
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// **机型引用的板必须都能落地**（2026-10-02，与 [`Self::check_asset_refs`] 同一范式）。
    ///
    /// 三条：
    ///
    /// 1. `plateIds` 的每一项都要解析到 `presets/plates/*.toml` 里一块真实的板 ——
    ///    悬空引用只表现为「塔地图那层不出」，没有任何东西报错，所以升级成 error；
    /// 2. `defaultPlateId` 若写了，必须在 `plateIds` 里（默认板不在可用板列表里是打字错误）；
    /// 3. `defaultPlateId` 若写了，必须解析到真实板（同第 1 条）。
    ///    没有 `plateIds` 却写了 `defaultPlateId` 也归到第 2 条 —— 那条默认板无处可依。
    fn check_plate_refs(&self) -> Result<(), AppError> {
        for m in self.catalog.machines() {
            for id in &m.plate_ids {
                if self.catalog.plate(id).is_none() {
                    return Err(AppError::corrupted(format!(
                        "机型 {} 的 plateIds 指向一个不存在的板：{id}",
                        m.id
                    ))
                    .with_detail(
                        "板定义在 presets/plates/*.toml（一板一文件）。\
                         id 打错的后果是塔地图那一层静默消失"
                            .to_owned(),
                    ));
                }
            }
            let Some(default) = m
                .default_plate_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            if !m.plate_ids.iter().any(|id| id == default) {
                return Err(AppError::corrupted(format!(
                    "机型 {} 的 defaultPlateId（{default}）不在 plateIds 里",
                    m.id
                ))
                .with_detail(
                    "默认板必须是这台机型 plateIds 里的一块 —— 否则塔地图会去选一块这台机器用不了的板"
                        .to_owned(),
                ));
            }
            if self.catalog.plate(default).is_none() {
                return Err(AppError::corrupted(format!(
                    "机型 {} 的 defaultPlateId 指向一个不存在的板：{default}",
                    m.id
                ))
                .with_detail("板定义在 presets/plates/*.toml（一板一文件）".to_owned()));
            }
        }
        Ok(())
    }

    /// **套餐域的引用必须都能落地**（b05 Task 10.5 / 10.7 / 10.8）。
    ///
    /// 三条，方向各不同：
    ///
    /// 1. **机型与版本 → 套餐**（10.5：悬空引用消除）。`defaultBundle` 与每个版本的
    ///    `recommendedBundle` 写着的 id 必须能在 `bundles.toml` 里查到 —— 悬空的时候
    ///    它们只是字符串，查不到只表现为"那台机器没有推荐套餐"，现在升级成 error
    ///    （doc §9：套餐定义落地后转为 error）。**空串跳过**（旧数据里那台四个字段皆空），
    ///    与 [`Self::check_asset_refs`] 同一口径 —— "没有"就该不写那一行；
    /// 2. **套餐 → 机型**（归属必须真实）。套餐的 `machineId` 指着不存在的机型，
    ///    与资产条目的假归属是同一类错（Task 8.8 那条的反方向）；
    /// 3. **套餐 → 资产**（10.7 后半 + 10.8）。每个 `assetRef` 必须解析到一条真实资产，
    ///    且**至少一条是 BBS 预设**（10.8）：MKP 侧 `use_ironing_path = true` 只在 BBS 侧
    ///    那组值成立时才有意义（doc §12.4），发了 MKP 不发 BBS，用户打出来的结果是错的。
    ///    这一条要看得见资产定义，所以放在这里而不是 [`super::bundles`] 的单文件检查里
    fn check_bundle_refs(&self) -> Result<(), AppError> {
        for m in self.catalog.machines() {
            if let Some(id) = non_empty(&m.default_bundle) {
                self.bundles.get(id).ok_or_else(|| {
                    AppError::corrupted(format!(
                        "机型 {} 的 defaultBundle 指向一个不存在的套餐：{id}",
                        m.id
                    ))
                    .with_detail("套餐定义在 presets/bundles.toml".to_owned())
                })?;
            }
            for v in &m.versions {
                if let Some(id) = non_empty(&v.recommended_bundle) {
                    self.bundles.get(id).ok_or_else(|| {
                        AppError::corrupted(format!(
                            "机型 {} 版本 {} 的 recommendedBundle 指向一个不存在的套餐：{id}",
                            m.id, v.id
                        ))
                        .with_detail("套餐定义在 presets/bundles.toml".to_owned())
                    })?;
                }
            }
        }

        for b in self.bundles.items() {
            self.catalog.machine(&b.machine_id).ok_or_else(|| {
                AppError::corrupted(format!(
                    "套餐 {} 的 machineId 指向一个不存在的机型：{}",
                    b.id, b.machine_id
                ))
            })?;

            let mut has_bbs = false;
            for r in &b.asset_refs {
                let a = self.assets.get(r).ok_or_else(|| {
                    AppError::corrupted(format!(
                        "套餐 {} 的 assetRefs 指向一个不存在的资产：{r}",
                        b.id
                    ))
                    .with_detail(
                        "资产定义在 presets/assets.toml；id 打错的后果是这套 BBS 预设静默缺席"
                            .to_owned(),
                    )
                })?;
                if a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs") {
                    has_bbs = true;
                }
            }
            if !has_bbs {
                return Err(AppError::corrupted(format!(
                    "套餐 {} 的 assetRefs 里没有一条 BBS 预设",
                    b.id
                ))
                .with_detail(
                    "套餐的内容就是 BBS 引用（doc §12.4）：MKP 预设与其配套 BBS 预设必须\
                     成套配发，发了 MKP 不发 BBS，用户打出来的结果是错的"
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// 改一份套餐的文件清单（b05 Task 14 / P4：套餐内容编辑的领域入口）。
    ///
    /// 跨文件的两条判据在这里拦（加载期 [`Self::check_bundle_refs`] 的同款，方向相反：
    /// 那边拦「已写坏的定义读进来」，这边拦「写出去之前就不合格」）：
    ///
    /// 1. 每个 `assetRef` 必须解析到一条真实资产 —— id 打错的后果是这套 BBS 预设静默缺席；
    /// 2. 改完**至少一条 BBS 预设**（10.8：MKP 与 BBS 成套配发，发 MKP 不发 BBS，
    ///    用户打出来的结果是错的）。
    ///
    /// 全部通过才落一个文件（`bundles.toml`），由 [`super::bundles::Bundles::set_refs`]
    /// 保证内存与文档面一起改。内容没变就不写 —— `updatedAt` 不能被一次空操作刷新。
    ///
    /// `now_iso8601` 由**调用方**给（工作台给 `workbench::clock::now_iso8601()`）：
    /// 这一层不该为了写一个日期而把时间源也认下来 —— 与"根由调用方给"同一条纪律。
    pub fn set_bundle_refs(
        &mut self,
        bundle_id: &str,
        refs: &[String],
        now_iso8601: &str,
    ) -> Result<bool, AppError> {
        let Some(cur) = self.bundles.get(bundle_id) else {
            return Err(AppError::not_found(format!("查无此套餐：{bundle_id}")));
        };
        let same = |a: &[String], b: &[String]| {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| x.trim().to_lowercase() == y.trim().to_lowercase())
        };
        if same(&cur.asset_refs, refs) {
            return Ok(false);
        }

        // 先整套查一遍再动手：任何一条不合格就一个字节都不写（同 remove_asset 的纪律）
        let mut has_bbs = false;
        for r in refs {
            let a = self.assets.get(r.trim()).ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "套餐 {bundle_id} 的 assetRef 指向一个不存在的资产：{r}"
                ))
                .with_detail("资产定义在 presets/assets.toml；引用必须先在资产库里登记".to_owned())
            })?;
            if a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs") {
                has_bbs = true;
            }
        }
        if !has_bbs {
            return Err(AppError::invalid_argument(format!(
                "套餐 {bundle_id} 的 assetRefs 里没有一条 BBS 预设"
            ))
            .with_detail(
                "套餐的内容就是 BBS 引用（doc §12.4）：MKP 预设与其配套 BBS 预设必须\
                 成套配发，发了 MKP 不发 BBS，用户打出来的结果是错的"
                    .to_owned(),
            ));
        }

        self.bundles.set_refs(bundle_id, refs, now_iso8601)?;
        self.bundles.write()?;
        Ok(true)
    }

    /// **新建一条套餐**（作者 2026-10-03：套餐页补 CRUD —— C15 的能力，产品侧真写）。
    ///
    /// 同 [`Self::set_bundle_refs`] 的两道闸：引用都要解析得到、至少一条 BBS
    /// （10.8 成套配发 —— 所以新建时就得挑内容，不能落一份空套餐出去）。
    /// id 走 [`bundles::check_bundle_id`] 的门槛且不与现有套餐撞（大小写不敏感）。
    pub fn add_bundle(
        &mut self,
        id: &str,
        machine_id: &str,
        display: &str,
        refs: &[String],
        now_iso8601: &str,
    ) -> Result<(), AppError> {
        bundles::check_bundle_id(id)?;
        if self.bundles.get(id).is_some() {
            return Err(AppError::invalid_argument(format!(
                "套餐 id 已经存在：{id}"
            )));
        }
        if self.catalog.machine(machine_id).is_none() {
            return Err(AppError::not_found(format!("没有机型 {machine_id}")));
        }
        if display.trim().is_empty() {
            return Err(AppError::invalid_argument(
                "套餐要有给人看的名字（display）",
            ));
        }
        self.check_bundle_content(id, refs)?;
        self.bundles.add(Bundle {
            id: id.trim().to_owned(),
            display: display.trim().to_owned(),
            machine_id: machine_id.trim().to_owned(),
            asset_refs: refs.iter().map(|s| s.trim().to_owned()).collect(),
            updated_at: Some(now_iso8601.get(..10).unwrap_or(now_iso8601).to_owned()),
        })?;
        self.bundles.write()?;
        Ok(())
    }

    /// **改一条套餐的 id 与/或显示名**。
    ///
    /// id 被机型文件引用着（`defaultBundle` / `recommendedBundle`）—— 改 id 就要
    /// **连带重指**那 14 处引用，否则那些机型文件指向一条不存在的套餐（静默损坏）。
    /// 先改本文件、再逐台重指重写，两边都过了才算改了。
    pub fn rename_bundle(
        &mut self,
        id: &str,
        new_id: &str,
        display: Option<&str>,
        now_iso8601: &str,
    ) -> Result<(), AppError> {
        self.bundles.rename(id, new_id, display, now_iso8601)?;
        // 引用重指（大小写不敏感对上旧 id 的都换）。先在内存里改、逐台落盘
        let old = id.trim().to_lowercase();
        let same =
            |v: &Option<String>| v.as_deref().is_some_and(|s| s.trim().to_lowercase() == old);
        let touched: Vec<String> = self
            .catalog
            .machines()
            .iter()
            .filter(|m| {
                same(&m.default_bundle) || m.versions.iter().any(|v| same(&v.recommended_bundle))
            })
            .map(|m| m.id.clone())
            .collect();
        for mid in &touched {
            {
                let m = self.catalog.machine_mut(mid)?;
                if same(&m.default_bundle) {
                    m.set_default_bundle(Some(new_id))?;
                }
                let vids: Vec<String> = m
                    .versions
                    .iter()
                    .filter(|v| same(&v.recommended_bundle))
                    .map(|v| v.id.clone())
                    .collect();
                for vid in vids {
                    m.set_version_field(&vid, VersionField::RecommendedBundle, Some(new_id))?;
                }
            }
            self.catalog.write_machine(mid)?;
        }
        self.bundles.write()?;
        Ok(())
    }

    /// **复制一条套餐**：内容（assetRefs）与归属照抄，id 必须是新的。
    /// 复制出来的那份**不被任何版本指着** —— 指向是要人显式改的动作
    pub fn copy_bundle(
        &mut self,
        id: &str,
        new_id: &str,
        display: Option<&str>,
        now_iso8601: &str,
    ) -> Result<(), AppError> {
        bundles::check_bundle_id(new_id)?;
        let src = self
            .bundles
            .get(id)
            .ok_or_else(|| AppError::not_found(format!("查无此套餐：{id}")))?
            .clone();
        if self.bundles.get(new_id).is_some() {
            return Err(AppError::invalid_argument(format!(
                "套餐 id 已经存在：{new_id}"
            )));
        }
        self.bundles.add(Bundle {
            id: new_id.trim().to_owned(),
            display: display
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&src.display)
                .to_owned(),
            machine_id: src.machine_id.clone(),
            asset_refs: src.asset_refs.clone(),
            updated_at: Some(now_iso8601.get(..10).unwrap_or(now_iso8601).to_owned()),
        })?;
        self.bundles.write()?;
        Ok(())
    }

    /// **删一条套餐**。被机型默认或任何版本指着时**整次拒绝**并点名 ——
    /// 静默删掉会让那些版本生成时一条 BBS 都拿不到（10.8 的静默漏洞）。
    /// 出路：去套餐页把指向逐个取消（版本回到机型默认），或改指别的套餐
    pub fn remove_bundle(&mut self, id: &str) -> Result<(), AppError> {
        let want = id.trim().to_lowercase();
        let same = |v: &Option<String>| {
            v.as_deref()
                .is_some_and(|s| s.trim().to_lowercase() == want)
        };
        let mut holders: Vec<String> = Vec::new();
        for m in self.catalog.machines() {
            if same(&m.default_bundle) {
                holders.push(format!("机型 {} 的 defaultBundle", m.id));
            }
            for v in &m.versions {
                if same(&v.recommended_bundle) {
                    holders.push(format!("版本 {}/{}", m.id, v.id));
                }
            }
        }
        if !holders.is_empty() {
            return Err(
                AppError::invalid_argument(format!("套餐 {id} 还被引用着，不能删"))
                    .with_detail(holders.join("、")),
            );
        }
        self.bundles.remove(id)?;
        self.bundles.write()?;
        Ok(())
    }

    /// 改一条资产的**交付档位**（`download` ↔ `bundled`），落盘。
    /// 守卫在 [`Assets::set_delivery`]（mkPreset 不许随包）
    pub fn set_asset_delivery(&mut self, id: &str, delivery: &str) -> Result<(), AppError> {
        let d = match delivery.trim() {
            "download" => crate::presetdata::assets::Delivery::Download,
            "bundled" => crate::presetdata::assets::Delivery::Bundled,
            other => {
                return Err(AppError::invalid_argument(format!(
                    "交付档位只有 download / bundled，收到：{other}"
                )))
            }
        };
        self.assets.set_delivery(id, d)?;
        self.assets.write()?;
        Ok(())
    }

    /// 新建套餐那两道闸（[`Self::add_bundle`] 与 [`Self::set_bundle_refs`] 共用）：
    /// 每条引用都解析得到、至少一条 BBS
    fn check_bundle_content(&self, bundle_id: &str, refs: &[String]) -> Result<(), AppError> {
        let mut has_bbs = false;
        for r in refs {
            let a = self.assets.get(r.trim()).ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "套餐 {bundle_id} 的 assetRef 指向一个不存在的资产：{r}"
                ))
                .with_detail("资产定义在 presets/assets.toml；引用必须先在资产库里登记".to_owned())
            })?;
            if a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs") {
                has_bbs = true;
            }
        }
        if !has_bbs {
            return Err(AppError::invalid_argument(format!(
                "套餐 {bundle_id} 的 assetRefs 里没有一条 BBS 预设"
            ))
            .with_detail(
                "套餐的内容就是 BBS 引用（doc §12.4）：MKP 预设与其配套 BBS 预设必须\
                 成套配发，发了 MKP 不发 BBS，用户打出来的结果是错的"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    /// **把一批版本指到这份套餐**（作者 2026-10-03：多选 + 确认）。
    ///
    /// 一次手势 = 一次落盘序列：逐台机型文件改 `recommendedBundle`，
    /// **已经指着它的跳过**（空操作不写盘）。返回真正改动的 uid 列表 ——
    /// 界面拿它回答「哪些被影响了」。
    ///
    /// **不限机型**（作者：「不应该限制」）：一版一套的约束只有「一个版本只指
    /// 一个套餐」，套餐可以被任何版本指向。
    pub fn assign_versions(
        &mut self,
        bundle_id: &str,
        uids: &[String],
    ) -> Result<Vec<String>, AppError> {
        if self.bundles.get(bundle_id).is_none() {
            return Err(AppError::not_found(format!("查无此套餐：{bundle_id}")));
        }
        // 先整套查一遍：任何一条不合格就一个字节都不写（同 set_bundle_refs 的纪律）
        let mut plan: Vec<(String, String, String)> = Vec::new();
        for uid in uids {
            let (mid, vid) = uid.trim().split_once('/').ok_or_else(|| {
                AppError::invalid_argument(format!("版本 uid 形状不对：{uid}（要 机型/版本）"))
            })?;
            let m = self
                .catalog
                .machine(mid)
                .ok_or_else(|| AppError::not_found(format!("版本 {uid} 的机型 {mid} 不存在")))?;
            if !m.versions.iter().any(|v| v.id == vid) {
                return Err(AppError::not_found(format!("{mid} 没有叫 {vid} 的版本")));
            }
            plan.push((mid.to_owned(), vid.to_owned(), uid.trim().to_owned()));
        }

        let mut changed: Vec<String> = Vec::new();
        for (mid, vid, uid) in plan {
            {
                let m = self.catalog.machine_mut(&mid)?;
                if m.versions
                    .iter()
                    .find(|v| v.id == vid)
                    .and_then(|v| v.recommended_bundle.as_deref())
                    .is_some_and(|s| s.trim().eq_ignore_ascii_case(bundle_id))
                {
                    continue; // 已经指着它 —— 空操作
                }
                m.set_version_field(&vid, VersionField::RecommendedBundle, Some(bundle_id))?;
            }
            self.catalog.write_machine(&mid)?;
            changed.push(uid);
        }
        Ok(changed)
    }

    /// **谁在用它**（b05 Task 9.4，套餐那一档 b05 Task 10）：删资产之前必须先问这一条。
    ///
    /// 删掉一张还被机型引用着的图，界面上只表现为"那台机型的图没了" ——
    /// 与 `machineVariants` 的孤儿同一类：不报错，只是没了。
    pub fn asset_usage(&self, id: &str) -> Result<AssetUsage, AppError> {
        if self.assets.get(id).is_none() {
            return Err(AppError::not_found(format!("没有资产 {id}")));
        }
        let want = id.trim().to_lowercase();
        let same = |v: &Option<String>| {
            v.as_deref()
                .is_some_and(|s| s.trim().to_lowercase() == want)
        };

        let machines: Vec<String> = self
            .catalog
            .machines()
            .iter()
            .filter(|m| same(&m.image) || same(&m.icon))
            .map(|m| m.id.clone())
            .collect();

        // 引用它的套餐：`assetRefs` 里写着这个 id 的。反查的是**套餐 id**，
        // 因为那是机型文件引用的东西 —— 人拿着它去机型文件里能找到出处
        let bundles: Vec<String> = self
            .bundles
            .items()
            .iter()
            .filter(|b| b.asset_refs.iter().any(|r| r.trim().to_lowercase() == want))
            .map(|b| b.id.clone())
            .collect();

        Ok(AssetUsage { machines, bundles })
    }

    /// 删一条资产，**先反查**（b05 Task 9.5）：有人引用就拒绝，并说出是谁
    pub fn remove_asset(&mut self, id: &str) -> Result<(), AppError> {
        let usage = self.asset_usage(id)?;
        if !usage.machines.is_empty() || !usage.bundles.is_empty() {
            return Err(
                AppError::invalid_argument(format!("资产 {id} 还被引用着，不能删")).with_detail(
                    format!(
                        "引用它的机型：{}；套餐：{}",
                        if usage.machines.is_empty() {
                            "（无）".to_owned()
                        } else {
                            usage.machines.join("、")
                        },
                        if usage.bundles.is_empty() {
                            "（无）".to_owned()
                        } else {
                            usage.bundles.join("、")
                        }
                    ),
                ),
            );
        }
        self.assets.remove(id)?;
        self.assets.write()
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

        // 资产条目归属的机型必须存在（b05 Task 8）。与下面那条同一类错：
        // 写一个不存在的机型 id，界面上只表现为"这台机型的图没了"，没有任何东西报错
        self.assets.check_against_machines(&machines)?;
        // MKP 预设的「归属版本」必须真实存在（作者 2026-10-03：这一类靠
        // machineId + versionId 定位那一版的产物）—— 指向不存在的版本，界面上只会
        // 表现为「这一版没有徽章」，而没有任何东西报错
        for a in self.assets.items() {
            if a.kind != crate::presetdata::AssetKind::MkPreset {
                continue;
            }
            let (Some(mid), Some(vid)) = (
                a.machine_id.as_deref().map(str::trim),
                a.version_id.as_deref().map(str::trim),
            ) else {
                continue; // 形状那条（check_one）已经报过了
            };
            let known = self
                .catalog
                .machine(mid)
                .is_some_and(|m| m.versions.iter().any(|v| v.id == vid));
            if !known {
                return Err(AppError::corrupted(format!(
                    "资产 {} 归属的版本不存在：{mid}/{vid}",
                    a.id
                ))
                .with_detail(
                    "MKP 预设在台账里靠「机型 + 版本」定位那一版的产物 —— \
                     版本 id 写错的话它指向的是空气"
                        .to_owned(),
                ));
            }
        }
        // 反方向：机型引用的资产 id 必须存在（b05 Task 9 / 8.6）
        self.check_asset_refs()?;
        // 机型引用的板 id 必须存在（2026-10-02，与 check_asset_refs 同一范式）
        self.check_plate_refs()?;
        // 套餐域：机型/版本 → 套餐、套餐 → 机型、套餐 → 资产（b05 Task 10.5/10.7/10.8）
        self.check_bundle_refs()?;

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
        let root = repo_presets_root()?;
        Some(Presets::load_from(&root).expect("presets/ 里的机型目录读不齐"))
    }

    /* ---------- 套餐 CRUD（2026-10-03，夹具级） ---------- *
     * 全部写在 `workbench` feature 下：夹具与时间源都住在工作台那一侧，
     * 这一层刻意不认它们（写路径的日期由调用方给）—— 与
     * `set_bundle_refs_replaces_the_list_or_refuses` 同一条理由 */

    #[cfg(feature = "workbench")]
    fn now() -> String {
        crate::workbench::clock::now_iso8601()
    }

    /// 新建的四道闸 + 正路径。**新建时就得挑内容**：落一份空套餐出去，
    /// 下一次加载就会被「成套配发」判据拦住 —— 所以闸在写之前而不是读的时候
    #[cfg(feature = "workbench")]
    #[test]
    fn a_new_bundle_needs_a_bbs_a_real_machine_and_a_free_id() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;

        // 空内容 / 没有 BBS / 引用不存在 / 机型不存在 / id 撞 —— 各拦一次
        assert!(p.add_bundle("A1_NEW", "A1", "新套餐", &[], &now()).is_err());
        assert!(p
            .add_bundle("A1_NEW", "A1", "新套餐", &["a1-icon".to_owned()], &now())
            .is_err());
        assert!(p
            .add_bundle("A1_NEW", "A1", "新套餐", &["ghost".to_owned()], &now())
            .is_err());
        assert!(p
            .add_bundle(
                "A1_NEW",
                "NOPE",
                "新套餐",
                &["a1-bbs-04-020".to_owned()],
                &now()
            )
            .is_err());
        assert!(p
            .add_bundle(
                "A1_default",
                "A1",
                "新套餐",
                &["a1-bbs-04-020".to_owned()],
                &now()
            )
            .is_err());

        // 正路径：落盘重读真的有了，refs 照抄
        p.add_bundle(
            "A1_NEW",
            "A1",
            "新套餐",
            &[
                "a1-standard-bbs-placeholder".to_owned(),
                "a1-bbs-04-020".to_owned(),
            ],
            &now(),
        )
        .expect_err("夹具里没有 mkPreset 资产，先只用 BBS");
        p.add_bundle(
            "A1_NEW",
            "A1",
            "新套餐",
            &["a1-bbs-04-020".to_owned(), "p1s-bbs-02-010".to_owned()],
            &now(),
        )
        .expect("新建");
        let again = Presets::load_from(p.root()).expect("重读");
        let b = again.bundles.get("A1_NEW").expect("落盘之后重读得到");
        assert_eq!(b.machine_id, "A1");
        assert_eq!(b.asset_refs.len(), 2);
    }

    /// **改 id 要连带重指机型文件**：defaultBundle 与各版本的 recommendedBundle
    /// 一起换、一起落盘 —— 只改 bundles.toml 的话，机型文件指向一条不存在的套餐
    /// （静默损坏：生成时那条 BBS 就没了）
    #[cfg(feature = "workbench")]
    #[test]
    fn renaming_a_bundle_repoints_the_machine_files() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;

        p.rename_bundle("A1_default", "A1_BASE", Some("基础套餐"), &now())
            .expect("改名");
        let again = Presets::load_from(p.root()).expect("重读");
        let a1 = again.catalog.machine("A1").expect("A1 在");
        assert_eq!(
            a1.default_bundle.as_deref(),
            Some("A1_BASE"),
            "机型默认跟着改"
        );
        for v in &a1.versions {
            assert_eq!(
                v.recommended_bundle.as_deref(),
                Some("A1_BASE"),
                "版本 {} 的指向跟着改",
                v.id
            );
        }
        assert_eq!(
            again.bundles.get("A1_BASE").expect("新 id 在").display,
            "基础套餐"
        );
        assert!(again.bundles.get("A1_default").is_none(), "旧 id 没了");
        let expected_today = now();
        assert_eq!(
            again.bundles.get("A1_BASE").unwrap().updated_at.as_deref(),
            Some(expected_today[..10].to_owned()).as_deref(),
            "内容变了 updatedAt 要盖今天"
        );
    }

    /// 复制：内容照抄、id 必须是新的、**复制出来的那份没人指着**
    #[cfg(feature = "workbench")]
    #[test]
    fn copying_a_bundle_keeps_the_refs_and_stays_unreferenced() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;
        p.copy_bundle("A1_default", "A1_COPY", None, &now())
            .expect("复制");
        let mut again = Presets::load_from(p.root()).expect("重读");
        let src = again.bundles.get("A1_default").expect("原份还在");
        let copied = again.bundles.get("A1_COPY").expect("新份在");
        assert_eq!(copied.asset_refs, src.asset_refs, "内容照抄");
        assert_eq!(copied.machine_id, src.machine_id);
        // 没有任何机型 / 版本指着它
        for m in again.catalog.machines() {
            assert_ne!(m.default_bundle.as_deref(), Some("A1_COPY"));
            for v in &m.versions {
                assert_ne!(v.recommended_bundle.as_deref(), Some("A1_COPY"));
            }
        }
        assert!(
            again
                .copy_bundle("A1_default", "A1_COPY", None, &now())
                .is_err(),
            "撞 id 拒"
        );
    }

    /// 删除：被机型默认或版本指着时整次拒绝并点名；解除之后真删掉
    #[cfg(feature = "workbench")]
    #[test]
    fn removing_a_bundle_refuses_while_pointed_at() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;
        let err = p
            .remove_bundle("A1_default")
            .expect_err("夹具里 A1 的默认与两版都指着它");
        assert!(err
            .detail
            .clone()
            .unwrap_or_default()
            .contains("A1/STANDARD"));
        // 解除所有指向之后删得掉，重读真没了
        for vid in ["STANDARD", "FAST"] {
            p.catalog
                .machine_mut("A1")
                .unwrap()
                .set_version_field(vid, VersionField::RecommendedBundle, None)
                .unwrap();
        }
        p.catalog
            .machine_mut("A1")
            .unwrap()
            .set_default_bundle(None)
            .unwrap();
        p.catalog.write_machine("A1").unwrap();
        p.remove_bundle("A1_default").expect("删掉");
        assert!(Presets::load_from(p.root())
            .expect("重读")
            .bundles
            .get("A1_default")
            .is_none());
    }

    /// 交付档位：download ↔ bundled 一起改内存与文档面；mkPreset 不许随包
    #[cfg(feature = "workbench")]
    #[test]
    fn assigning_versions_moves_a_batch_and_skips_the_ones_already_there() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;

        // 坏输入整次拒绝：套餐不存在 / uid 形状不对 / 机型或版本不存在
        assert!(p
            .assign_versions("no_such", &["P1S/LITE".to_owned()])
            .is_err());
        assert!(p
            .assign_versions("A1_default", &["P1S".to_owned()])
            .is_err());
        assert!(p
            .assign_versions("A1_default", &["GHOST/LITE".to_owned()])
            .is_err());
        assert!(p
            .assign_versions("A1_default", &["P1S/NOPE".to_owned()])
            .is_err());

        // 跨机型批量：夹具里 A1 两版本来就指着 A1_default（所以它们是「跳过」那两档），
        // 真正改的只有 P1S/LITE
        let changed = p
            .assign_versions(
                "A1_default",
                &[
                    "P1S/LITE".to_owned(),
                    "A1/FAST".to_owned(),
                    "A1/STANDARD".to_owned(),
                ],
            )
            .expect("批量指向");
        assert_eq!(changed, vec!["P1S/LITE".to_owned()], "已指着的不算改动");

        let again = Presets::load_from(p.root()).expect("重读");
        assert_eq!(
            again
                .catalog
                .machine("P1S")
                .unwrap()
                .versions
                .first()
                .unwrap()
                .recommended_bundle
                .as_deref(),
            Some("A1_default"),
            "跨机型也指得过去（作者：不应该限制）"
        );
        // 全都指过来之后再指一次 = 空操作，一个字节都不动
        let before = std::fs::read_to_string(p.catalog.machine("P1S").unwrap().file()).unwrap();
        let again_changed = p
            .assign_versions("A1_default", &["P1S/LITE".to_owned()])
            .expect("再指一次");
        assert!(again_changed.is_empty(), "已经指着的不算改动");
        assert_eq!(
            std::fs::read_to_string(p.catalog.machine("P1S").unwrap().file()).unwrap(),
            before,
            "空操作不该动文件"
        );
    }

    #[cfg(feature = "workbench")]
    #[test]
    fn set_delivery_moves_between_tiers_and_refuses_mk_preset() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut p = f.presets;
        p.set_asset_delivery("a1-icon", "bundled").expect("设随包");
        let again = Presets::load_from(p.root()).expect("重读");
        assert_eq!(
            again.assets.get("a1-icon").expect("在").delivery,
            crate::presetdata::assets::Delivery::Bundled,
            "落盘之后重读得到"
        );
        // 改回去：文档面上那条键被删掉（回到缺省 download）
        p.set_asset_delivery("a1-icon", "download").expect("改回");
        assert_eq!(
            Presets::load_from(p.root())
                .expect("重读")
                .assets
                .get("a1-icon")
                .expect("在")
                .delivery,
            crate::presetdata::assets::Delivery::Download
        );

        // mkPreset 不许设成随包（产物不在资产根，没有随包复制这条路径）——
        // 造一份只有 mkPreset 的台账来验这道闸
        let dir = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(
            &dir.path().join(assets::ASSETS_FILE),
            "[[assets]]\nid = 'a1-std'\ntype = 'mkPreset'\nmachineId = 'A1'\nversionId = 'STANDARD'\nname = 'A1 标准版预设'\n".as_bytes(),
        )
        .unwrap();
        let mut assets = assets::Assets::load_from(dir.path()).unwrap();
        assert!(
            assets
                .set_delivery("a1-std", crate::presetdata::assets::Delivery::Bundled)
                .is_err(),
            "mkPreset 设随包必须被拦"
        );
        assert!(
            assets
                .set_delivery("a1-std", crate::presetdata::assets::Delivery::Download)
                .is_ok(),
            "mkPreset 保持按需下载是合法动作"
        );
    }

    /// **产物命名规则的那份副本必须与生成器一致。**
    ///
    /// 样例逐字取自 `crates/preset/src/generate.rs` 自己的测试。客户端里没有那个 crate
    /// （它挂在 `workbench` feature 下），所以只能靠这一条钉住 —— 漂移的时候产物名对不上，
    /// 消费端会以为"这个版本没有文件"。
    #[test]
    fn mkp_file_name_matches_the_generator() {
        assert_eq!(mkp_file_name("A1", "FASTV3.3"), "A1-fastv3.3.toml");
        assert_eq!(mkp_file_name("A1", "fastv3.3"), "A1-fastv3.3.toml");
        assert_eq!(
            mkp_file_name("A1_MINI", "STANDARD"),
            "A1_MINI-standard.toml"
        );
        assert_eq!(mkp_file_name("P1S", "lite"), "P1S-lite.toml");
        // 大小写不敏感：命名规则只把版本那段小写，机型 id 原样
        assert_eq!(mkp_file_name("A1", "Fast"), mkp_file_name("A1", "FAST"));
    }

    #[test]
    fn the_real_catalog_loads() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        // 搬进来的是 5 个机型
        assert_eq!(p.catalog.machines().len(), 5, "搬进来的机型数");
        assert!(!p.catalog.brands().is_empty());

        let a1 = p.catalog.machine("A1").expect("A1 必须在");
        assert_eq!(a1.display, "A1");
        assert_eq!(a1.brand, "Bambu Lab");
        assert_eq!(a1.versions.len(), 3, "A1 三个版本");
        assert_eq!(a1.versions[0].id, "STANDARD");
        assert_eq!(a1.versions[0].name, "标准版");
        assert_eq!(a1.external_aliases, vec!["A1C", "A1F"]);

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

    /// 递归收 `.toml`（`registry/` 与 `forbidden_zones/` 都在子目录里）。
    /// **跳过 `dist/`** —— 那是机器生成的产物目录，不是手维护的源
    fn collect_toml(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                // 交付目录里的产物 TOML 不是源：跳过（旧名 dist 也一并认，防历史检出）
                if p.file_name()
                    .is_some_and(|n| n == "dist" || n == "delivery")
                {
                    continue;
                }
                collect_toml(&p, out);
            } else if p.extension().is_some_and(|x| x == "toml") {
                out.push(p);
            }
        }
    }

    /// **①层的数据文件必须是 LF 换行。**
    ///
    /// 为什么值得一条判据：`toml_edit` 写回时把换行统一成 LF，于是一个 CRLF 的文件
    /// **读进来没事，第一次保存就整份被改写** —— diff 里一片红，而真正改了什么反而看不出来。
    /// `.gitattributes` 管得住入库那一份，管不住工作区里手写或工具生成的那一份
    /// （真踩过：新加的定义文件被工具按 CRLF 写出来，正是这条判据把它抓出来的）。
    ///
    /// 扫 `presets/` 下全部 `.toml`。**不看 `*.json`** —— 那些是上游产物，不在我们写回的面上。
    #[test]
    fn the_source_files_keep_lf_line_endings() {
        let Some(root) = repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let mut files = Vec::new();
        collect_toml(&root, &mut files);
        assert!(
            files.len() >= 10,
            "只扫到 {} 个 .toml —— 路径大概不对，这条判据在空转",
            files.len()
        );

        let mut bad: Vec<String> = Vec::new();
        for p in &files {
            let bytes = std::fs::read(p).expect("读得到");
            if bytes.contains(&b'\r') {
                bad.push(p.strip_prefix(&root).unwrap_or(p).display().to_string());
            }
        }
        assert!(
            bad.is_empty(),
            "这些 .toml 是 CRLF：{bad:?}\n\
             读进来没事，但第一次保存会被 toml_edit 整份改写成 LF —— diff 一片红，\
             真正改了什么反而看不出来。存成 LF（本仓 .gitattributes 也是这么规定的）"
        );
    }

    /// **机型引用的资产必须能解析到真实文件**（b05 Task 9.7）。
    ///
    /// 与上面那条加载期检查的分工：加载期只管"id 认不认得出来"（打错字是 error），
    /// 这条管"文件真的在不在" —— 那是搬运的验收，也是 Task 11.1 的 warning 级。
    #[test]
    fn every_machine_asset_ref_points_at_a_real_file() {
        let Some(mut p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let Some(asset_root) = repo_assets_root() else {
            eprintln!("没定位到 <repo>/presets/assets，这条检查未执行（不是通过）");
            return;
        };
        p.set_asset_root(&asset_root);
        let mut checked = 0usize;
        for m in p.catalog.machines() {
            for (field, value) in [
                ("image", &m.image),
                ("imageVariant", &m.image_variant),
                ("icon", &m.icon),
            ] {
                let Some(id) = value.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                let a = p
                    .assets
                    .get(id)
                    .unwrap_or_else(|| panic!("{} 的 {field} 指向一个不存在的资产 {id}", m.id));
                assert!(
                    p.assets.present(a),
                    "资产 {id}（{}）的文件不在：{}",
                    a.path,
                    p.assets
                        .resolve(a)
                        .map(|x| x.display().to_string())
                        .unwrap_or_else(|e| format!("解析失败：{e}"))
                );
                checked += 1;
            }
        }
        // 反空转：真数据里 5 条图标引用（整机图 2026-10-01 剥离台账后不再占这一档）
        // —— 少于 5 条就是漏查了
        assert!(checked >= 5, "只查了 {checked} 条引用 —— 这条判据在空转");
    }

    /// **机型引用的板必须能解析到真实的一块板**（2026-10-02，③）。
    ///
    /// 真数据里 5 台机型各引 1 块板（A1/P1S/P2S/X1C → 单卡舌，A1_MINI → 双卡舌），
    /// 目录里恰好 2 块（去重后）。这条同时钉住「引用可解析」与「去重生效」。
    #[test]
    fn every_machine_plate_ref_points_at_a_real_plate() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        // 目录里 2 块板，id 是稳定主键
        let ids: Vec<&str> = p.catalog.plates().map(|x| x.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["dual-latch-180", "single-latch-256"],
            "去重后 2 块板"
        );

        let mut checked = 0usize;
        for m in p.catalog.machines() {
            for id in &m.plate_ids {
                assert!(
                    p.catalog.plate(id).is_some(),
                    "{} 的 plateIds 指向一个不存在的板 {id}",
                    m.id
                );
                checked += 1;
            }
            if let Some(default) = m.default_plate_id.as_deref() {
                assert!(
                    m.plate_ids.iter().any(|id| id == default),
                    "{} 的默认板 {default} 不在自己的 plateIds 里",
                    m.id
                );
                assert!(p.catalog.plate(default).is_some(), "默认板要能被解析");
            }
        }
        assert_eq!(checked, 5, "5 台机型各引一块板 —— 少查就是空转");
    }

    /// **改一份套餐的文件清单**（b05 Task 14 / P4）：真写盘 + `updatedAt` 盖新值、
    /// 内容没变不写、悬空引用与「没有一条 BBS」被整拦下。
    ///
    /// 用夹具而不是真数据：写路径的测试不许动真仓库（ bundles.toml 是真源，别的
    /// 测试还在并行读它）。
    ///
    /// 写在 `workbench` feature 下：夹具与时间源都住在工作台那一侧，而这一层
    /// 刻意不认它们（写路径的日期由调用方给）。
    #[cfg(feature = "workbench")]
    #[test]
    fn set_bundle_refs_replaces_the_list_or_refuses() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut presets = f.presets;
        let file = presets.bundles.file().to_path_buf();
        let original = std::fs::read_to_string(&file).expect("读夹具原文");
        let now = || crate::workbench::clock::now_iso8601();

        // 内容没变：Ok(false)，文件一个字节都不动（updatedAt 不许被空操作刷新）
        let changed = presets
            .set_bundle_refs("A1_default", &["a1-bbs-04-020".to_owned()], &now())
            .expect("没变也是成功的");
        assert!(!changed);
        assert_eq!(
            std::fs::read_to_string(&file).expect("重读"),
            original,
            "空操作不该写盘"
        );

        // 换内容：返回 true，落盘重读真的变了、日期盖今天（原来那格是迁移照抄的 2026-07-12）
        let changed = presets
            .set_bundle_refs(
                "a1_default",
                &["p1s-bbs-02-010".to_owned(), "a1-bbs-04-020".to_owned()],
                &now(),
            )
            .expect("换内容");
        assert!(changed);
        let on_disk = std::fs::read_to_string(&file).expect("读盘");
        assert!(
            on_disk.contains("p1s-bbs-02-010"),
            "要真落盘，不能只在内存里：{on_disk}"
        );
        let today = crate::workbench::clock::now_iso8601()[..10].to_owned();
        assert!(on_disk.contains(&today), "updatedAt 要盖上今天：{on_disk}");
        assert!(
            presets.bundles.get("A1_default").is_some(),
            "id 查询大小写不敏感，改过的那条还在"
        );
        assert_eq!(
            presets.bundles.get("A1_default").unwrap().asset_refs.len(),
            2,
            "内存里也要跟上 —— 下一次 write 才不会把旧值写回去"
        );

        // 悬空引用被拦；拦下之后文件还是刚才那份
        let err = presets
            .set_bundle_refs("A1_default", &["ghost-asset".to_owned()], &now())
            .expect_err("悬空引用必须被拦");
        assert!(err.message.contains("不存在"), "实测：{}", err.message);
        assert_eq!(
            std::fs::read_to_string(&file).expect("重读"),
            on_disk,
            "被拦下就不该动文件"
        );

        // 只装图片不装 BBS 也被拦（10.8 成套配发）
        let err = presets
            .set_bundle_refs("A1_default", &["a1-image".to_owned()], &now())
            .expect_err("没有 BBS 必须被拦");
        assert!(err.message.contains("BBS"), "实测：{}", err.message);

        // 空列表同一条判据的另一端：加载期拦空 assetRefs，这里拦写出去的空套餐
        let err = presets
            .set_bundle_refs("A1_default", &[], &now())
            .expect_err("空套餐必须被拦");
        assert!(err.message.contains("BBS"), "实测：{}", err.message);

        // 查无此套餐
        assert!(presets
            .set_bundle_refs("no_such", &["a1-bbs-04-020".to_owned()], &now())
            .is_err());
    }

    /// **反查与删除守卫**（b05 Task 9.4 / 9.5，套餐那一档 b05 Task 10）。
    ///
    /// 用夹具而不是真数据：真数据里删东西是破坏性的，而这里要验的正是"删"这条路径。
    #[cfg(feature = "workbench")]
    #[test]
    fn an_asset_that_is_still_used_cannot_be_removed() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut presets = f.presets;

        // 反查说得出是谁：夹具里 A1 的 `image` 指着它
        let usage = presets.asset_usage("a1-image").expect("反查");
        assert_eq!(usage.machines, vec!["A1".to_owned()]);
        assert!(usage.bundles.is_empty(), "机型图不该被任何套餐引用");

        let err = presets
            .remove_asset("a1-image")
            .expect_err("有人引用就不能删");
        assert!(err.message.contains("还被引用"), "实测：{}", err.message);
        assert!(
            err.detail.unwrap_or_default().contains("A1"),
            "要说清是谁在用，不然人只能一个个机型去翻"
        );
        assert!(presets.assets.get("a1-image").is_some(), "被拦下就不该真删");

        // 大小写不敏感：换个写法同样拦得住
        presets
            .remove_asset("A1-IMAGE")
            .expect_err("id 查询是大小写不敏感的，删除守卫也该是");

        // **被套餐引用的 BBS 也删不掉**（b05 Task 10）：它一没，套餐里那台机器的
        // `use_ironing_path` 就成了没人配套的孤招 —— 守卫要说得出是哪条套餐在用
        let usage = presets.asset_usage("a1-bbs-04-020").expect("反查");
        assert!(usage.machines.is_empty(), "BBS 预设不被机型直接引用");
        assert_eq!(
            usage.bundles,
            vec!["A1_default".to_owned()],
            "套餐那一档必须说得出是谁"
        );
        let err = presets
            .remove_asset("a1-bbs-04-020")
            .expect_err("被套餐引用就不能删");
        let detail = err.detail.clone().unwrap_or_default();
        assert!(
            detail.contains("A1_default"),
            "要报出套餐 id，实测：{detail:?}"
        );
        assert!(
            presets.assets.get("a1-bbs-04-020").is_some(),
            "被拦下就不该真删"
        );

        // 没人引用的一条：删得掉，而且落盘后重读确实少了它
        let before = presets.assets.items().len();
        presets
            .remove_asset("a1-extra-image")
            .expect("没人引用就该删得掉");
        assert_eq!(presets.assets.items().len(), before - 1);
        let again = Presets::load_from(presets.root()).expect("重读");
        assert!(
            again.assets.get("a1-extra-image").is_none(),
            "删了要真的落盘，不能只改内存"
        );
        assert!(
            again.assets.get("a1-icon").is_some(),
            "别的条目一个都不许少"
        );
    }

    /// 删一个不存在的资产要**报"没有"**，而不是静默成功
    #[cfg(feature = "workbench")]
    #[test]
    fn removing_an_unknown_asset_says_so() {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let mut presets = f.presets;
        let err = presets.remove_asset("no-such-asset").expect_err("必须报错");
        assert!(err.message.contains("没有资产"), "实测：{}", err.message);
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

    /// **机型与版本引用的每个 bundle id 都存在**（b05 Task 10.5 / 10.7，真数据）。
    ///
    /// 加载期检查（[`Self::check_bundle_refs`]）在 `real()` 里已经跑过一遍，
    /// 这里要的是**能看见数字**：引用了多少处、指向了哪几条套餐 ——
    /// 一条全靠"加载没报错"的判据分不清"检查过了"和"检查在空转"。
    #[test]
    fn the_real_bundle_references_resolve() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let mut checked = 0usize;
        let mut named = std::collections::BTreeSet::new();
        for m in p.catalog.machines() {
            // 机型层只有 defaultBundle 一格（版本层是 recommendedBundle）——
            // 写成单元素循环会被 clippy（新版 stable）判 single_element_loop
            if let Some(id) = m
                .default_bundle
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let b = p
                    .bundles
                    .get(id)
                    .unwrap_or_else(|| panic!("{} 的 defaultBundle 指向不存在的套餐 {id}", m.id));
                assert_eq!(b.machine_id, m.id, "套餐归属与引用它的机型不一致");
                checked += 1;
                named.insert(id.to_owned());
            }
            for v in &m.versions {
                if let Some(id) = v
                    .recommended_bundle
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    p.bundles.get(id).unwrap_or_else(|| {
                        panic!(
                            "{} 版本 {} 的 recommendedBundle 指向不存在的套餐 {id}",
                            m.id, v.id
                        )
                    });
                    checked += 1;
                    named.insert(id.to_owned());
                }
            }
        }
        // 反空转：真数据实测 14 处非空引用（5 defaultBundle + 9 recommendedBundle；
        // A2L 的空串不算）。少于它就是机型文件变了 —— 说清为什么再改这个数
        assert!(
            checked >= 14,
            "只查了 {checked} 处 bundle 引用 —— 这条判据在空转"
        );
        // 一版一套（2026-10-03）：9 份套餐各被引用着，条数变了要在提交里说清为什么
        assert_eq!(
            named.len(),
            9,
            "引用到的套餐数：{named:?} —— A1 / A1_MINI 各三份 + P1S / P2S / X1C 各一份"
        );
    }

    /// **套餐引用的每个 assetRef 都能解析，且每条套餐至少一条 BBS**
    /// （b05 Task 10.7 / 10.8，真数据）。
    ///
    /// 与加载期检查的分工同 9.7 那条：加载期只管"id 认不认得出来"，
    /// 这里把**归属也对上**（套餐的 machineId ↔ 资产的 machineId）并点出数字。
    #[test]
    fn the_real_bundle_asset_refs_resolve() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let items = p.bundles.items();
        assert_eq!(
            items.len(),
            9,
            "一版一套（2026-10-03，照 C15 模型重排）—— 条数变了就在提交里说清为什么"
        );

        let mut bbs_total = 0usize;
        for b in items {
            assert_eq!(
                p.catalog.machine(&b.machine_id).map(|m| m.id.as_str()),
                Some(b.machine_id.as_str()),
                "套餐 {} 的归属机型不存在",
                b.id
            );
            assert!(
                !b.asset_refs.is_empty(),
                "套餐 {} 的 assetRefs 是空的 —— 加载期就该拦下，到这里还在说明检查没接上",
                b.id
            );
            for r in &b.asset_refs {
                let a = p
                    .assets
                    .get(r)
                    .unwrap_or_else(|| panic!("套餐 {} 引用不存在的资产 {r}", b.id));
                if a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs") {
                    bbs_total += 1;
                    assert_eq!(
                        a.machine_id.as_deref(),
                        Some(b.machine_id.as_str()),
                        "套餐 {}（{}）引用了别家机型的 BBS：{r}（{}）",
                        b.id,
                        b.machine_id,
                        a.machine_id.as_deref().unwrap_or("（无归属）")
                    );
                }
            }
        }
        // 反空转：9 条套餐各引 1 条 BBS（实测），BBS 引用总数为 0 说明上面的判定路径没走通
        assert!(
            bbs_total >= 9,
            "9 条套餐只对上 {bbs_total} 条 BBS 引用 —— 10.8 的判定在空转"
        );

        // **反查实测**（10.4 判据的反向）：`a1-bbs-04-020` 被 A1 的三份套餐各装一次；
        // 换个大小写查同一个资产，结果必须一致
        let u = p.asset_usage("a1-bbs-04-020").expect("反查");
        assert_eq!(
            u.bundles,
            vec![
                "A1_STANDARD".to_owned(),
                "A1_FAST".to_owned(),
                "A1_FASTV3.3".to_owned()
            ]
        );
        let u2 = p.asset_usage("A1-BBS-04-020").expect("反查（大写）");
        assert_eq!(u, u2, "反查是大小写不敏感的");
        // 图标那一档不被套餐引用：归属 ≠ 引用
        let u = p.asset_usage("a1-icon").expect("反查");
        assert_eq!(u.machines, vec!["A1".to_owned()]);
        assert!(u.bundles.is_empty(), "图标不该出现在套餐的 assetRefs 里");
    }
}
