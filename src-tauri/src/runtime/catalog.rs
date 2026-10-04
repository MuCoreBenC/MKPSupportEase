//! 运行时说明书（catalog）：首屏需要的**全部定义** + 每份交付文件的 SHA 与下载落点。
//!
//! # 它是谁的产物
//!
//! 发布构建（`cargo run --bin gen-catalog`）从**层①**（`<repo>/presets` 的源 TOML +
//! `crates/preset/assets/presets` 的交付产物真字节）构建出来，落成
//! `src/runtime/catalog.generated.json` 编进二进制（**层②**），首启释放进
//! `<appDataDir>/catalog.json`（**层③**）。层③的运行时只读释放下来的那一份。
//! 工作台发布（`wb_publish`）用同一个构建本体产出 `dist/catalog.json` ——
//! **发布方与消费方说的是同一种语言**（两端共用契约，R11 起生效）。
//!
//! # 加厚（第二圈）：definition 进 catalog
//!
//! 从"只有机型/版本/文件清单"长出**完整定义**：品牌、机型的全部字段（含尺寸与禁区）、
//! 资产、套餐、字段定义与界面布局。definition 的类型**直接复用** [`crate::presetdata`]
//! 的 serde 类型（`Asset` / `Bundle` / `ParamDef` / `Dimensions` / `TabMeta`…）——
//! 它们本来就是这些事实的类型化表达，另造一套镜像只会让两边慢慢漂。
//! 从此客户端首屏只读 catalog 这**一个文件**，不再解析 13 份源 TOML（旧世界退役）。
//!
//! [`CATALOG_SCHEMA`] 表达格式代次：**加字段不升号**（definition 全部带
//! `#[serde(default)]`，旧 catalog 也能读，缺的定义当"没有"），改语义才升。
//!
//! # revision 是什么
//!
//! 对 brands + machines + assets + bundles + registry + files 的稳定序列化取的
//! SHA256 前 16 位 ——「这份目录描述的输入和上次是不是同一份」。definition 在输入里：
//! 改一个字段定义、挪一个布局项，revision 就变，检查更新看得见。它**不是**完整性校验
//! （那是文件条目里每个 `sha256` 的事）。刻意没有 `generatedAt`：没有可信时间源之前
//! 不编一个上去（与 ClientDataPackage 同一条裁决）。

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::presetdata::{Asset, Bundle, Dimensions, LayoutTab, MachineVersion, ParamDef, TabMeta};

/// 目录格式的代次。读到的文件比这新 → `CORRUPTED`（程序老）；比这旧同理（文件是旧程序写的，
/// 这一版还没有那种文件，判据先立着）
pub const CATALOG_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub catalog_schema: u32,
    /// 目录指纹。源或交付产物变了它就变
    pub revision: String,
    /// 品牌清单（机型文件里写的是 id，给人看的名字在这里）
    #[serde(default)]
    pub brands: Vec<crate::presetdata::Brand>,
    /// 机型的完整定义：元信息、尺寸、版本、禁区
    #[serde(default)]
    pub machines: Vec<CatalogMachine>,
    /// 资产定义（图标 / 模型 / 切片器预设的登记，总纲欠账 #3 的登记面）。
    /// 整机图**在**这里（`type = 'Image'`，2026-10-03 恢复登记）：它随包但不进
    /// `files[]` —— 那是 `dest_of_asset` 按交付档位拦的，不按类型拦。
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// 套餐定义（一版一套：MKP 与配套 BBS 的成套配发关系）
    #[serde(default)]
    pub bundles: Vec<Bundle>,
    /// 字段定义与界面布局（参数表 74 条 + 页签/分组元数据 + 参数摆放）
    #[serde(default)]
    pub registry: CatalogRegistry,
    /// 打印板（2026-10-02）：机型只持引用（`plateIds`），几何在这里。
    /// 塔地图按 `defaultPlateId` 从这一份里按 id 查板
    #[serde(default)]
    pub plates: Vec<crate::presetdata::Plate>,
    /// 一份交付文件。`path` 是相对**内部根**的落点 —— 下载它就该落到那（铁律 3：
    /// 没下载就没有；下载了才出现在 `mkp/`）
    pub files: Vec<CatalogFile>,
    /// **结构代次签名**（`runtime::structure`）——「这批数据是什么结构」的机器真值。
    ///
    /// 客户端拿它跟自己的 `SUPPORTED_SIGNATURES` 对：命中就说明"这个构建有读懂它的
    /// 代码"，**不看版本号**（Dev 场景：安装包还没发出去也不该把自己锁死）。
    ///
    /// `#[serde(default)]` 是刻意的：老客户端读到这一格不认识也该照常解析（旧版的
    /// 判断只剩 [`Catalog::min_client_version`] 那条路）。**不进 [`revision_of`]** ——
    /// 它是结构的函数，内容不变它就不变，没理由惊动"检查更新"。
    #[serde(default)]
    pub structure_signature: String,
    /// **最低正式客户端版本**（`presets/structure-signatures.toml` 里登记的）——
    /// "读得懂这一代的最老那个正式版"。`None` = 规则表还没登记这一代
    /// （发布闸会拦住，所以**发出去的目录里不会缺它**）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
}

/// catalog 里 definition 的注册表部分：字段定义 + 分组元数据 + 参数摆放。
///
/// 与 [`crate::presetdata::ParamRegistry`] 的差别只有**没有写回状态**（`DocumentMut`
/// 与文件路径是工作台的编辑侧资产）——读所需的 params / tabs / layout 三样原样在。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRegistry {
    #[serde(default)]
    pub params: Vec<ParamDef>,
    /// 页签与分组的元数据（中文名、排序、图标的唯一权威）
    #[serde(default)]
    pub tabs: Vec<TabMeta>,
    /// 参数摆放（`layout_schema`：哪个参数落在哪个 section、section 级可见性）
    #[serde(default)]
    pub layout: Vec<LayoutTab>,
}

/// 一台机型的完整定义。与 [`crate::presetdata::Machine`] 的只读视图同构 ——
/// 那边拖着 `DocumentMut`（保真写回用），这边是纯数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMachine {
    pub id: String,
    /// 界面上显示的名字。空串当"没有"处理（访问方法会给 id）
    pub display: String,
    /// TOML 里的 `name` 字段（实测有的机型 display 才是人看的，name 是空串）
    #[serde(default)]
    pub name: String,
    /// **品牌显示名**（构建时已从 brands.toml 换算，与 get_machines 同一条）
    pub brand: String,
    #[serde(default)]
    pub default_bundle: Option<String>,
    /// 外部别名（`A1C` / `A1F` 这种）。**不许与任何机型 ID 相撞**（构建源已保证）
    #[serde(default)]
    pub external_aliases: Vec<String>,
    /// **资产 id**（不是路径）：机型图。2026-10-03 起源里真写它
    /// （`image = 'a1-image'`，指向台账 `type = 'image'` 的那条），客户端按 id 查
    /// `assets[]` 拿 `path` 拼 `/assets/<path>` —— **不再有第二张表**。
    #[serde(default)]
    pub image: Option<String>,
    /// **第二个图位**：装了快拆件那张外观图（今天只有 A1 mini 有）。客户端在选到版本
    /// 那一级显示它，缺则回落 `image`。见 `presetdata::Machine::image_variant`
    #[serde(default)]
    pub image_variant: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// 这台机型能用的打印板 id（`plates` 域里按 id 查）。空 = 没有板规格
    #[serde(default)]
    pub plate_ids: Vec<String>,
    /// 默认用哪一块板（塔地图按它选）。`None` = 没指定
    #[serde(default)]
    pub default_plate_id: Option<String>,
    /// `None` = 占位机型，还没配 `[dimensions]`
    #[serde(default)]
    pub dimensions: Option<Dimensions>,
    pub versions: Vec<MachineVersion>,
    /// 禁区。没有就是空数组（构建源里只有 P1S / P2S / X1C 有）
    #[serde(default)]
    pub zones: Vec<crate::presetdata::Zone>,
}

/// 层①的资产载荷根（`presets/assets.toml` 里 `path` 的基准），相对仓库根。
///
/// **它不是运行时数据**：构建期从它读产物、算 SHA/大小。
/// 2026-10-03 从 `public/assets` 搬来（产品数据资源住一起；`public/` 那个资产目录之后退役）。
const REPO_ASSET_ROOT: &str = "presets/assets";

/// **发布根**（`catalog.path` 的基准）—— 相对仓库根。
///
/// ★ 2026-10-04 定：**发布根 = `presets/`**。客户端 `baseUrl` 锚在 `source.json`
/// 所在目录（`presets/dist/`），而 `catalog.path` 是相对发布根的 ——
/// 两者拼起来正好是云端真实位置。
pub(crate) const REPO_PUBLISH_ROOT: &str = "presets";

/// 资产在发布根下的第一段目录名：`assets/<台账 path>`。
///
/// ★ **这一段不能省**：`baseUrl` = `.../presets/dist/`，A 类资产在 `.../presets/assets/` ——
/// 兄弟目录，必须靠这个前缀跳到正确位置。见 `docs/PUBLISH-ARCHITECTURE.md` §2.2。
const ASSET_PREFIX: &str = "assets";

/// B 类渲染产物（MKP 预设 TOML）在发布根下的目录：`dist/mkp/presets/`。
///
/// 与 [`ASSET_PREFIX`] 对称：A 类原地不动，B 类落在 `dist/` 里（它源里没有实体）。
pub const PRESET_DEST_DIR: &str = "dist/mkp/presets";

/// 交付文件的**种类**（第三圈第一刀起不止一种）。
///
/// 规矩：**新增一种外部资源 = 加一个常量 + 一个落点目录**，不新增存储体系、不新增下载路径。
/// 管道、校验、归档、进度全是既有的那些。
pub mod kind {
    /// MKP 预设（随软件发布的成品内容）
    pub const PRESET: &str = "mkp_preset";
    /// 切片器预设。今天只有 BBS（`slicer = 'bbs'`、档位 `profile = 'process'`）
    pub const BBS_CONFIG: &str = "bbs_config";
    /// 测试 / 校准用的 3mf 模型
    pub const MODEL: &str = "model";
    /// 机型图标
    pub const ICON: &str = "icon";
}

/// 资产台账里的类型 → 目录里的 kind。
///
/// **返回 `None` 就是不登记**。今天两种：
///
/// - `Image`：图片**不进下载面**（整机图 / 品牌图都走 `bundled` 档随包，
///   客户端靠构建期装配拿它们，不按 URL 取）；
/// - `MkPreset`：MKP 预设的产物文件**已经**作为 `files` 条目进来了（B 类，
///   生成侧算的，落点 `dist/mkp/presets/…`）—— 台账这一条只登记「哪一版叫什么、归谁」，
///   再登记一份文件条目就是同一个文件两条真相（作者 2026-10-03）。
///
/// 注意判据是**类型**，不是交付档位：`bundled` 档照样走这一支，只是被
/// [`dest_of_asset`] 在更早一步按档位拦掉。
fn kind_of_asset(asset_kind: crate::presetdata::AssetKind) -> Option<&'static str> {
    match asset_kind {
        crate::presetdata::AssetKind::SlicerProfile => Some(kind::BBS_CONFIG),
        crate::presetdata::AssetKind::Model => Some(kind::MODEL),
        crate::presetdata::AssetKind::Icon => Some(kind::ICON),
        crate::presetdata::AssetKind::Image => None,
        crate::presetdata::AssetKind::MkPreset => None,
    }
}

/// 资产在**发布根**下的相对路径：`assets/<台账 path>`。
///
/// # 唯一路径语义（2026-10-04 作者裁决 C，**甲**）
///
/// 这一个值同时是**三件事**，没有第二套路由规则：
///
/// ```text
/// catalog.path │
///              ├─ 云端取哪：  <publish-root>/assets/…      （Git 仓库里的真实位置）
///              └─ 本地放哪：  <appDataDir>/assets/…        （下载落点，同一相对路径）
/// ```
///
/// **它不再按 kind 猜目录**。以前是 `mkp/<kind 目录>/<去掉类型前缀>` —— 那让
/// 同一份字节在云端与本地各有一套算法，任何目录调整都得改两处。现在台账 `path`
/// 的基准是**资产载荷根**（`presets/assets/`），而载荷根相对发布根就是 `assets/`，
/// 所以只补这一段前缀，形状原样保留。
///
/// ★ `assets/` 这一段不能省：客户端 `baseUrl` 锚在 `source.json` 所在目录
/// （`presets/dist/`），而 A 类资产在 `presets/assets/` —— 兄弟目录，故必须带
/// `assets/` 才拼得对。见 `docs/PUBLISH-ARCHITECTURE.md` §2.2。
fn asset_dest(asset_path: &str) -> String {
    format!("{ASSET_PREFIX}/{asset_path}")
}

/// 一条资产在**发布根 / 客户端内部根**下的落点：`assets/<台账 path>`。
///
/// **两端共用这一处算法**（客户端拼 URL 与落点、工作台发布复制）。
///
/// 返回 `None` = 这一份**不进交付集合**（客户端不会去 URL 取它）。两种情形：
///
/// 1. **交付档位是 `bundled`**（作者 2026-10-03）—— 台账里登记、工作台可管，
///    但它**随程序包带进客户端、不下载不更新**（整机图 / 品牌图）。这是主路径。
/// 2. **类型在下载面没有落点**（今天的 `Image`，与上一条重叠但判据不同）。
pub fn dest_of_asset(asset: &crate::presetdata::Asset) -> Option<String> {
    if asset.delivery == crate::presetdata::assets::Delivery::Bundled {
        return None;
    }
    kind_of_asset(asset.kind).map(|_| asset_dest(&asset.path))
}

/// 一份交付文件的**身份与落点**：有这一份、它叫什么、下载它该落到哪。
///
/// # 期望值是可选的（2026-10-04）
///
/// `sha256` / `size` 只在**发布侧**登记（工作台对 `dist/` 真字节算 —— 远端目录是下载
/// 校验的权威）。**随包 bootstrap 目录不登记它们**：交付产物的字节不在这一侧，
/// 拿构建用 TOML 算出来的 SHA 只是自找不同步（它会落后于云端、把下载判成 `SHA_MISMATCH`）。
///
/// `None` = "这一侧不该为它背书"，与 `Some` 的"这是权威期望值"是两回事，类型上分得开。
/// 详见总纲 §1③「catalog 的一生：bootstrap → OTA → 当前」。
///
/// `path` 是相对**内部根**的落点（铁律 3），形状统一为 `mkp/<kind 目录>/…`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFile {
    pub kind: String,
    /// 界面上的名字与下载时用的键。**必须唯一** —— 批量下载按它分辨"哪一份成了"
    pub file_name: String,
    pub path: String,
    /// 归属机型。**非预设类的资产没有版本概念时为空串**（空串 = 不适用，不是"没查出来"）
    pub machine_id: String,
    pub version_id: String,
    /// 交付产物真字节的期望值（产品规则 §10 的校验用它）。`None` = 这一侧不登记
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

impl CatalogFile {
    /// 下载校验的期望值。`None` **不是"校验通过"**，是"这份期望值不在这一侧"——
    /// 调用方据此跳过字节校验，交给 OTA 目录生效后的那次下载。
    pub fn expected_sha(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    /// 期望大小（也当下载水位用）。`None` = 这一侧不知道，不设上限
    pub fn expected_size(&self) -> Option<u64> {
        self.size
    }
}

impl Catalog {
    pub fn parse(bytes: &[u8]) -> Result<Catalog, AppError> {
        let catalog: Catalog = serde_json::from_slice(bytes)
            .map_err(|e| AppError::corrupted("catalog 读不出来").with_detail(e.to_string()))?;
        if catalog.catalog_schema != CATALOG_SCHEMA {
            return Err(AppError::corrupted(format!(
                "catalog 格式代次认不了：文件是 {}，程序认 {}",
                catalog.catalog_schema, CATALOG_SCHEMA
            )));
        }
        Ok(catalog)
    }

    /// 产物形态：pretty JSON + 结尾换行（进仓库当判据资产，diff 要能看）
    pub fn to_pretty_json(&self) -> Result<String, AppError> {
        let mut s = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::internal("catalog 序列化失败").with_detail(e.to_string()))?;
        s.push('\n');
        Ok(s)
    }
}

impl Catalog {
    /// 从**层①**构建：`<repo>/presets`（源定义）+ `crates/preset/assets/presets`（交付产物真字节）。
    ///
    /// 产物字节取**入库**的那一份（`BUILTIN_PRESETS` 编进二进制的同一批文件）——
    /// `presets/dist/` 是本机 gitignore 掉的暂存，进不了 CI，不能当判据输入。
    /// 每个机型版本都必须配齐产物，缺一份就失败 —— 宁可红着，不让目录里出现
    /// 「版本在、文件没有」这种静默的坑（那正是要收掉的旧账）。
    pub fn build_from_repo(repo_root: &Path) -> Result<Catalog, AppError> {
        let mut presets =
            crate::presetdata::Presets::load_from(&repo_root.join(REPO_PUBLISH_ROOT))?;
        // 资产载荷根（`presets/assets.toml` 里 `path` 的基准）。**只有构建期有仓库时才给得出** ——
        // 用户机器上那份定义还在，但载荷没有：那种时候资产一律表现为"还没下载"，
        // catalog 里登记的是"应该有这些文件"，不是"这些文件已经在了"。
        presets.set_asset_root(&repo_root.join(REPO_ASSET_ROOT));
        let assets = repo_root
            .join("crates")
            .join("preset")
            .join("assets")
            .join("presets");
        /*
         * **随包 bootstrap 目录：不读交付产物字节、不算 SHA**（2026-10-04）。
         *
         * `artifacts_dir`（`crates/preset/assets/presets/`）是构建用 TOML，不是客户端内置的
         * 最终资源（`mkp/` 初始为空是铁律 3）。以前拿它的真字节算 SHA 写进随包目录，
         * 那份期望值注定与云端不同步、下载必 `SHA_MISMATCH`。
         *
         * 于是走 `FileHashes::None`：条目照出（`get_version_files` 要用），`sha256`/`size`
         * 留 `None` —— 期望值归 OTA 目录。产物存在性另有判据守着（`gen-presets --check`）。
         */
        let (catalog, missing) = Self::collect(&presets, &assets, FileHashes::None);
        if let Some(first) = missing.first() {
            return Err(AppError::not_found(format!(
                "{first} —— 入库产物目录里没有这一份，先补齐再构建目录"
            )));
        }
        // 随包那份也要带「最低正式客户端版本」—— 客户端首启释放的就是它。
        // 查不到就留空：这一层不裁决"能不能发"（那是发布闸 ⑫ 的事）。
        let rules = crate::runtime::structure::RuleTable::load_from_repo(repo_root)?;
        let mut catalog = catalog.finalize();
        catalog.apply_min_client(&rules);
        Ok(catalog)
    }

    /// 从**已加载的预设源 + 一个产物目录**构建（**发布侧**语义）。
    ///
    /// - 安装包侧已改走 [`Catalog::build_from_repo`]（随包 bootstrap 目录，不算 SHA）；
    /// - 发布侧（工作台 `wb_publish`）：产物目录 = `dist/mkp/presets`（与客户端落点同形），
    ///   **宽松**——没有产物的版本是合法状态（交付集合本来就不含它），跳过。
    ///
    /// 保留这个严格版是**给手工/未来的严格发布**用；当前工作台只调宽松版。
    pub fn build_from_presets(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Result<Catalog, AppError> {
        let (catalog, missing) = Self::collect(presets, artifacts_dir, FileHashes::FromArtifacts);
        if let Some(first) = missing.first() {
            return Err(AppError::not_found(format!(
                "{first} —— 入库产物目录里没有这一份，先补齐再构建目录"
            )));
        }
        Ok(catalog.finalize())
    }

    /// 宽松版：没有产物的版本合法，只登记真实存在的产物。工作台发布 `dist/catalog.json` 用它。
    ///
    /// **对真字节算 SHA**（发布侧是期望值的权威）。
    pub fn build_from_presets_lenient(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Catalog {
        let (catalog, _) = Self::collect(presets, artifacts_dir, FileHashes::FromArtifacts);
        catalog.finalize()
    }

    /// 走一遍源 + 产物目录。返回目录与**缺失清单**（严格/宽松由调用方裁决）。
    ///
    /// definition 直接从已加载的 [`crate::presetdata::Presets`] 抄——那里是**加载期校验过**
    /// 的定义（id 唯一、引用落地、布局双射），catalog 不做第二次校验：同一份数据两套门禁,
    /// 就会有两套不同的答案。
    ///
    /// [`FileHashes`] 说**交付文件的期望值这一侧该不该算**：发布侧对真字节算
    /// （`FromArtifacts`），随包 bootstrap 侧不算（`None`）。见 [`CatalogFile`] 的说明。
    fn collect(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
        hashes: FileHashes,
    ) -> (Catalog, Vec<String>) {
        // 品牌显示名：机型文件里写的是 id，给人看的是 brands.toml 里的名字（与 get_machines 同一条）
        let brands: std::collections::HashMap<&str, &str> = presets
            .catalog
            .brands()
            .iter()
            .map(|b| (b.id.as_str(), b.name.as_str()))
            .collect();

        let mut machines = Vec::new();
        let mut files = Vec::new();
        let mut missing = Vec::new();

        for m in presets.catalog.machines() {
            machines.push(CatalogMachine {
                id: m.id.clone(),
                display: if m.display.trim().is_empty() {
                    m.id.clone()
                } else {
                    m.display.clone()
                },
                name: m.name.clone(),
                brand: brands
                    .get(m.brand.as_str())
                    .map(|s| (*s).to_owned())
                    .unwrap_or_else(|| m.brand.clone()),
                default_bundle: m.default_bundle.clone(),
                external_aliases: m.external_aliases.clone(),
                image: m.image.clone(),
                image_variant: m.image_variant.clone(),
                icon: m.icon.clone(),
                plate_ids: m.plate_ids.clone(),
                default_plate_id: m.default_plate_id.clone(),
                dimensions: m.dimensions.clone(),
                versions: m.versions.clone(),
                zones: presets
                    .catalog
                    .zones(&m.id)
                    .map(<[crate::presetdata::Zone]>::to_vec)
                    .unwrap_or_default(),
            });

            for v in &m.versions {
                let file_name = crate::presetdata::mkp_file_name(&m.id, &v.id);
                let (sha256, size) = match file_read(artifacts_dir, &file_name, hashes) {
                    Ok(hit) => hit,
                    Err(e) => {
                        missing.push(format!("{} / {}（{}）", m.id, v.id, e));
                        continue;
                    }
                };
                files.push(CatalogFile {
                    kind: kind::PRESET.to_owned(),
                    path: format!("{PRESET_DEST_DIR}/{file_name}"),
                    file_name,
                    machine_id: m.id.clone(),
                    version_id: v.id.clone(),
                    sha256,
                    size,
                });
            }
        }

        // 资产载荷（第三圈第一刀：切片器预设 / BBS）。
        // **登记的是盘上真文件的字节**：定义说有、文件不在 = 缺失（交给调用方裁决严格还是宽松），
        // 绝不登记一个"应该在但没见到"的条目——那等于把期望值编进目录里。
        for asset in presets.assets.items() {
            let Some(kind) = kind_of_asset(asset.kind) else {
                continue;
            };
            let Ok(full) = presets.assets.resolve(asset) else {
                missing.push(format!(
                    "资产 {}（{}）：这一侧没有资产载荷根",
                    asset.id, asset.path
                ));
                continue;
            };
            let Some(file_name) = Path::new(&asset.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
            else {
                missing.push(format!(
                    "资产 {}（{}）：路径取不出文件名",
                    asset.id, asset.path
                ));
                continue;
            };
            let (sha256, size) = match hashes {
                // 发布侧：对载荷真字节算期望值
                FileHashes::FromArtifacts => match std::fs::read(&full) {
                    Ok(bytes) => (Some(hex(&Sha256::digest(&bytes))), Some(bytes.len() as u64)),
                    Err(_) => {
                        missing.push(format!("资产 {}（{}）：载荷文件不在", asset.id, asset.path));
                        continue;
                    }
                },
                // 随包侧：不算 —— 期望值归 OTA 目录（见 [`CatalogFile`]）
                FileHashes::None => (None, None),
            };
            files.push(CatalogFile {
                kind: kind.to_owned(),
                path: asset_dest(&asset.path),
                file_name,
                machine_id: asset.machine_id.clone().unwrap_or_default(),
                version_id: String::new(),
                sha256,
                size,
            });
        }

        (
            Catalog {
                catalog_schema: CATALOG_SCHEMA,
                revision: String::new(),
                brands: presets.catalog.brands().to_vec(),
                machines,
                assets: presets.assets.items().to_vec(),
                bundles: presets.bundles.items().to_vec(),
                registry: CatalogRegistry {
                    params: presets.registry.params().to_vec(),
                    tabs: presets.registry.tabs().to_vec(),
                    layout: presets.registry.layout().to_vec(),
                },
                plates: presets.catalog.plates().cloned().collect(),
                files,
                // 两格都由 `finalize` 收口（签名从自己算；最低版本由发布侧查规则表填）
                structure_signature: String::new(),
                min_client_version: None,
            },
            missing,
        )
    }

    /// 收口。构建路径（严格/宽松）与将来的手工组装都从这里过 ——
    /// revision 与结构签名的算法各只有这一处。
    ///
    /// 签名的样本是**自己**（`to_value(&self)`）而不是随包那份：随包那份是上一代的
    /// 产物，拿它算出来的是上一代的结构 —— 那正好会漏掉"这一次改了结构"。
    /// 自指没问题：`structureSignature` 有 `#[serde(default)]`，它进不了必填清单。
    pub(crate) fn finalize(mut self) -> Catalog {
        self.structure_signature = crate::runtime::structure::signature_of_catalog(&self);
        self.revision = revision_of(&self);
        self
    }

    /// 把规则表登记的最低客户端版本填进来（发布侧与构建侧各调一次）。
    ///
    /// **查不到就留 `None`** —— 这一层不裁决"能不能发"，那是发布闸 ⑫ 的事。
    /// 但发出去的目录不会缺它：闸红着就出不了门。
    pub fn apply_min_client(&mut self, rules: &crate::runtime::structure::RuleTable) {
        self.min_client_version = rules
            .rule_of(&self.structure_signature)
            .map(|r| r.min_client.clone());
    }

    /* ---------- 消费端的只读访问面（ipc/presets 的九条命令从这里出数） ---------- */

    /// 按 ID 找机型。**别名不走这里** —— 别名是外部叫法，不是主键
    pub fn machine(&self, id: &str) -> Option<&CatalogMachine> {
        self.machines.iter().find(|m| m.id == id)
    }

    /// 品牌清单
    pub fn brands(&self) -> &[crate::presetdata::Brand] {
        &self.brands
    }

    /// 资产定义清单
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    /// 套餐定义清单
    pub fn bundles(&self) -> &[Bundle] {
        &self.bundles
    }

    /// 参数定义清单
    pub fn params(&self) -> &[ParamDef] {
        &self.registry.params
    }

    /// 显示用的机型名：`display` 空串当"没有"，给 id
    pub fn display_of(m: &CatalogMachine) -> &str {
        if m.display.trim().is_empty() {
            &m.id
        } else {
            m.display.as_str()
        }
    }

    /// 按 id 取资产。**大小写不敏感**（与 presetdata 的 `Assets::get` 同一口径——
    /// 机型文件引用时不保证大小写一致）
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        let want = id.trim().to_lowercase();
        self.assets.iter().find(|a| a.id.to_lowercase() == want)
    }

    /// 按 id 取套餐。**大小写不敏感**（与 presetdata 的 `Bundles::get` 同一口径）
    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        let want = id.trim().to_lowercase();
        self.bundles.iter().find(|b| b.id.to_lowercase() == want)
    }

    /// 按全局主键找参数定义
    pub fn param(&self, key: &str) -> Option<&ParamDef> {
        self.registry.params.iter().find(|p| p.key == key)
    }

    /// 一个 section 的中文名与组内序（中文名与顺序的唯一权威是 `[[tabs]]`，
    /// `layout_schema` 全文没有 label / order——与 ParamRegistry::section_meta 同一条）
    pub fn section_meta(&self, section_id: &str) -> Option<&crate::presetdata::SectionMeta> {
        self.registry
            .tabs
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == section_id)
    }

    /// 某个机型版本的交付文件条目。MKP 产物的路径由 catalog 登记给出，
    /// **不再由命名规则重算** —— 首屏唯一数据源 = catalog（总纲判据 4）
    pub fn file_of(&self, machine_id: &str, version_id: &str) -> Option<&CatalogFile> {
        self.files
            .iter()
            .find(|f| f.machine_id == machine_id && f.version_id == version_id)
    }
}

/// 对 definition + files 的稳定序列化取摘要。`revision` 本身不在输入里，没有自指问题。
/// definition 在输入里：改字段定义、挪布局、换套餐，检查更新都看得见
fn revision_of(catalog: &Catalog) -> String {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Payload<'a> {
        brands: &'a [crate::presetdata::Brand],
        machines: &'a [CatalogMachine],
        assets: &'a [Asset],
        bundles: &'a [Bundle],
        registry: &'a CatalogRegistry,
        plates: &'a [crate::presetdata::Plate],
        files: &'a [CatalogFile],
    }
    let bytes = serde_json::to_vec(&Payload {
        brands: &catalog.brands,
        machines: &catalog.machines,
        assets: &catalog.assets,
        bundles: &catalog.bundles,
        registry: &catalog.registry,
        plates: &catalog.plates,
        files: &catalog.files,
    })
    .unwrap_or_default();
    hex(&Sha256::digest(&bytes))[..16].to_owned()
}

/// SHA256 摘要的 hex 字符串。构建器算指纹、delivery 校验落盘字节，共用这一条实现
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 交付文件的期望值（`sha256` / `size`）这一侧该不该算。
///
/// 两个构建入口各选一边，见 [`CatalogFile`] 的长注释：
/// - [`FileHashes::FromArtifacts`] —— 发布侧（产物就在手边，期望值的权威）；
/// - [`FileHashes::None`] —— 随包 bootstrap 侧（产物不在这一侧，期望值归 OTA 目录）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileHashes {
    FromArtifacts,
    None,
}

/// 一份入库产物的 `(sha256, size)`：**算不算**由 [`FileHashes`] 定，
/// **在不在**两种情况都要查（"版本在、文件没有"是静默的坑，两种目录都不许出现）。
fn file_read(
    dir: &Path,
    file_name: &str,
    hashes: FileHashes,
) -> Result<(Option<String>, Option<u64>), String> {
    match hashes {
        FileHashes::FromArtifacts => match std::fs::read(dir.join(file_name)) {
            Ok(bytes) => Ok((Some(hex(&Sha256::digest(&bytes))), Some(bytes.len() as u64))),
            Err(_) => Err(file_name.to_owned()),
        },
        // 随包侧只确认存在性（不读字节）——期望值归 OTA 目录
        FileHashes::None => match dir.join(file_name).exists() {
            true => Ok((None, None)),
            false => Err(file_name.to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真仓库的源 + 入库产物 → 真目录。与 `embedded_matches_rebuild` 共用这条输入
    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn builds_machines_and_files_of_every_kind() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");
        assert_eq!(catalog.machines.len(), 5, "5 台机型");
        assert_eq!(
            catalog
                .machines
                .iter()
                .map(|m| m.versions.len())
                .sum::<usize>(),
            9,
            "9 个版本，每个版本一份交付产物"
        );
        assert_eq!(catalog.revision.len(), 16, "指纹取 16 位");

        // 预设：9 份，落点 `dist/mkp/presets/` 下（发布根基准，B 类）
        let presets: Vec<&CatalogFile> = catalog
            .files
            .iter()
            .filter(|f| f.kind == kind::PRESET)
            .collect();
        assert_eq!(presets.len(), 9, "每个版本一份预设");
        assert!(
            presets
                .iter()
                .all(|f| f.path.starts_with(&format!("{PRESET_DEST_DIR}/"))),
            "预设一律落在 {PRESET_DEST_DIR}/ 下：{:?}",
            presets.iter().map(|f| &f.path).collect::<Vec<_>>()
        );

        // 资产（第三圈）：**每一种登记过的类型都与 presets/assets.toml 的条数对齐** ——
        // 少登记一条就是"目录说有、下载不到"，多登记一条就是"目录在编数据"
        let source = crate::presetdata::Presets::load_from(&repo_root().join("presets"))
            .expect("源读得出来");
        for asset_kind in [
            crate::presetdata::AssetKind::SlicerProfile,
            crate::presetdata::AssetKind::Model,
            crate::presetdata::AssetKind::Icon,
        ] {
            let want = kind_of_asset(asset_kind).expect("这一类是要登记的");
            let in_toml = source
                .assets
                .items()
                .iter()
                .filter(|a| a.kind == asset_kind)
                .count();
            let got = catalog.files.iter().filter(|f| f.kind == want).count();
            assert_eq!(got, in_toml, "{want} 的条目数与资产台账对齐");
            assert!(got > 0, "{want} 这一类台账里确实有货");
            // ★ 唯一路径语义（2026-10-04）：一律落在 `assets/<台账 path>` 下 ——
            // 不再按 kind 分目录（那是第二套路由规则，已废）
            assert!(
                catalog
                    .files
                    .iter()
                    .filter(|f| f.kind == want)
                    .all(|f| f.path.starts_with(&format!("{ASSET_PREFIX}/"))),
                "{want} 一律落在 {ASSET_PREFIX}/ 下（发布根基准）"
            );
        }

        // **整机图在台账里，但用 `bundled` 档不进交付**（作者 2026-10-03 改判，撤销
        // 2026-10-01 第三刀的剥离）。这条判据守的就是那条边界：
        //   - 台账里**要**有它（工作台要看得见、选得着 —— 硬编码进客户端源码是错的，
        //     不会编程的用户改不了图）；
        //   - catalog 的 `files[]` 里**要没有**它（不进云端交付，客户端不下载）。
        // 拦的地方是 `dest_of_asset`（**按交付档位拦**，不是按类型拦）。
        let images = source
            .assets
            .items()
            .iter()
            .filter(|a| a.kind == crate::presetdata::AssetKind::Image)
            .collect::<Vec<_>>();
        assert!(
            !images.is_empty(),
            "整机图在台账里（bundled 档）—— 第三刀把它搬进客户端源码是错的，已作废"
        );
        assert!(
            images
                .iter()
                .all(|a| a.delivery == crate::presetdata::assets::Delivery::Bundled),
            "整机图一律 bundled 档：台账登记、工作台可管，但不进云端交付"
        );
        assert!(
            !catalog
                .files
                .iter()
                .any(|f| images.iter().any(|a| f.path.ends_with(&a.path))),
            "整机图不进 catalog 的 files[]（bundled 档 = 客户端不下载）"
        );
        assert!(
            !catalog
                .files
                .iter()
                .any(|f| f.path.starts_with("mkp/images/")),
            "下载区没有 images 这一类：整机图不进 Delivery"
        );
        // 反空转：catalog 的资产定义与台账**逐条对齐**（不是只数一个总数）——
        // 今天 28 条 = 9 MKP 预设 + 4 整机图（bundled）+ 9 BBS + 3 图标 + 3 模型
        assert_eq!(
            catalog.assets.len(),
            source.assets.items().len(),
            "catalog 的资产定义与 presets/assets.toml 逐条对齐"
        );
        assert_eq!(
            catalog.assets.len(),
            29,
            "实测 29 条（9 MKP 预设 + 4 整机图 + 1 品牌字标 + 9 BBS + 3 图标 + 3 模型）——
             2026-10-03 三类进台账（mkPreset / bundled image / 品牌图），条数变了要核对台账
             再改这里的期望"
        );

        // 文件条目与命名规则对得上：A1 + FASTV3.3 → A1-fastv3.3.toml
        let a1_fast = catalog
            .files
            .iter()
            .find(|f| f.machine_id == "A1" && f.version_id == "FASTV3.3")
            .expect("A1/FASTV3.3 该有交付产物");
        assert_eq!(a1_fast.file_name, "A1-fastv3.3.toml");
        assert_eq!(a1_fast.path, "dist/mkp/presets/A1-fastv3.3.toml");
        /*
         * ★ **随包 bootstrap 目录不登记交付文件期望值**（2026-10-04）。
         *
         * 这条以前断言 `sha256` 是 64 位 hex —— 那正是旧语义（拿构建用 TOML 的真字节
         * 算 SHA 写进随包目录），于是那份期望值注定与云端不同步、下载必 `SHA_MISMATCH`。
         * 现在 `build_from_repo` 走 `FileHashes::None`：条目照出（还给 `get_version_files`
         * 用），但期望值留空，归 OTA 目录。判据反过来咬"没有期望值"。
         */
        assert!(
            a1_fast.expected_sha().is_none() && a1_fast.expected_size().is_none(),
            "随包 bootstrap 目录不许登记交付文件期望值：{a1_fast:?}"
        );
        // 但"有这一份 + 叫什么 + 落哪"三件事实都得对（去 SHA 不是去条目）
        assert!(a1_fast.file_name.ends_with(".toml"));
        // 落点是**发布根基准**（B 类渲染产物在 `dist/` 下），同时就是客户端内部落点
        assert!(
            a1_fast.path.starts_with(PRESET_DEST_DIR),
            "B 类产物一律落在 {PRESET_DEST_DIR}/ 下：{}",
            a1_fast.path
        );
    }

    /// **打印板随 catalog 下发，机型的引用都能落地**（2026-10-02，③ 的判据）。
    ///
    /// 板是独立实体：几何只住 `presets/plates/*.toml`，机型只持引用。这条钉四件事：
    ///   ① 目录里真有 2 块板，几何（w/d/frame/path/bodyPath）一格不少；
    ///   ② 每台写 `plateIds` 的机型，每一项都能在 `plates` 里查到；
    ///   ③ `defaultPlateId` 必须在自己的 `plateIds` 里（否则塔地图会去选一块用不了的板）；
    ///   ④ A43 那 5 个机型映射去重成了 2 块（A1/P1S/P2S/X1C 同一块、A1_MINI 另一块）。
    #[test]
    fn plates_ride_along_and_machine_refs_resolve() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");

        assert_eq!(catalog.plates.len(), 2, "两个机型映射去重成 2 块板");
        let by_id: std::collections::HashMap<&str, &crate::presetdata::Plate> =
            catalog.plates.iter().map(|p| (p.id.as_str(), p)).collect();
        let single = by_id.get("single-latch-256").expect("单卡舌那块在");
        assert_eq!((single.w, single.d), (258.0, 276.0));
        assert_eq!(
            (
                single.frame.x,
                single.frame.y,
                single.frame.w,
                single.frame.h
            ),
            (1.0, 8.5, 256.0, 256.0),
            "可打印区归板（机型 bedSize 是涂胶/运动口径 260×255，不是这个）"
        );
        assert!(single.body_path.starts_with('M') && single.path.starts_with('M'));
        assert!(single.path.contains('Z'), "轮廓是闭合路径");

        let dual = by_id.get("dual-latch-180").expect("双卡舌那块在");
        assert_eq!((dual.w, dual.d), (184.0, 197.1));
        assert_eq!((dual.frame.w, dual.frame.h), (180.0, 180.0));

        // 每一台机型的引用都能落地
        for m in &catalog.machines {
            for id in &m.plate_ids {
                assert!(
                    by_id.contains_key(id.as_str()),
                    "{} 的板 {id} 要能查到",
                    m.id
                );
            }
            if let Some(default) = m.default_plate_id.as_deref() {
                assert!(
                    m.plate_ids.iter().any(|id| id == default),
                    "{} 的默认板必须在自己的 plateIds 里",
                    m.id
                );
            }
        }

        // A43 的映射去重：四台同一块、A1_MINI 另一块
        let expect: &[(&str, &str)] = &[
            ("A1", "single-latch-256"),
            ("P1S", "single-latch-256"),
            ("P2S", "single-latch-256"),
            ("X1C", "single-latch-256"),
            ("A1_MINI", "dual-latch-180"),
        ];
        for (mid, plate) in expect {
            let m = catalog
                .machine(mid)
                .unwrap_or_else(|| panic!("{mid} 在目录里"));
            assert_eq!(
                m.default_plate_id.as_deref(),
                Some(*plate),
                "{mid} 的默认板"
            );
        }

        // 指纹要跟着板定义走：改一块板的几何，revision 就该变
        let mut tweaked = catalog.clone();
        tweaked.plates[0].w += 1.0;
        assert_ne!(
            super::revision_of(&tweaked),
            catalog.revision,
            "板几何变了指纹就必须变（否则会拿到过期的目录）"
        );
    }

    /// **每一份交付文件都要能唯一定位**（`file_name` 是下载与"已下载"的键）。
    ///
    /// 这条守的是将来接更多资产时的那个坑：两个目录下都叫 `process.json` 的话，
    /// 批量下载的结果与"已下载清单"会串在一起 —— 而那种错在界面上表现为
    /// "点了这一份、勾上的是那一份"。
    #[test]
    fn every_file_has_a_unique_name_and_destination() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");

        let mut names = std::collections::HashSet::new();
        for f in &catalog.files {
            assert!(
                names.insert(f.file_name.as_str()),
                "文件名撞车：{}",
                f.file_name
            );
            assert!(
                f.path.starts_with("assets/") || f.path.starts_with("dist/"),
                "落点必须是发布根基准（assets/ 或 dist/）：{}",
                f.path
            );
            assert!(!f.path.contains(".."), "落点不许有 `..`：{}", f.path);
        }
        let mut paths = std::collections::HashSet::new();
        for f in &catalog.files {
            assert!(paths.insert(f.path.as_str()), "落点撞车：{}", f.path);
        }
    }

    /// **随包 bootstrap 目录：登记条目、不登记期望值**（2026-10-04 起）。
    ///
    /// 这条以前叫 `file_sizes_match_the_bytes_on_disk`，防的是"登记了 size/SHA 但没读
    /// 文件"。现在随包侧**根本不登记** `size`/`sha256`（见 [`CatalogFile`]）——
    /// 那条职责落到发布侧（工作台 `wb_publish` 对 `dist/` 真字节算）。
    ///
    /// 于是这条改成钉**去 SHA 之后仍成立的三件事**：
    ///   ① 每一份交付文件都有条目（去 SHA ≠ 去条目）；
    ///   ② 条目上 `size`/`sha256` 一律 `None`（随包侧不许编期望值）；
    ///   ③ 条目的载荷**在盘上真的存在**（`crates/preset/assets/…`）——
    ///      这条仍有价值：它防"目录列了一份没有的产物"，与 SHA 无关。
    #[test]
    fn bundled_catalog_lists_files_without_expecting_their_bytes() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");
        let repo = repo_root();

        let source =
            crate::presetdata::Presets::load_from(&repo.join("presets")).expect("源读得出来");
        for f in catalog.files.iter().filter(|f| f.kind != kind::PRESET) {
            assert!(
                f.expected_sha().is_none() && f.expected_size().is_none(),
                "随包侧不许登记期望值：{}",
                f.file_name
            );
            let asset = source
                .assets
                .items()
                .iter()
                .find(|a| asset_dest(&a.path) == f.path)
                .unwrap_or_else(|| panic!("{} 在资产台账里找不到", f.path));
            /*
             * ★ 「登记面 == 实体面」的核心判据：`catalog.path` 去掉 `assets/` 前缀就是
             * 载荷根里的相对位置 —— 它必须真的在。发布侧那一半由
             * `workbench::app::dist::audit_catalog` 与 `publish_into` 的收尾核对守着。
             */
            let on_disk = repo.join(REPO_ASSET_ROOT).join(&asset.path);
            assert!(
                on_disk.is_file(),
                "目录列了这一份，载荷却不在：{}（{}）",
                f.path,
                on_disk.display()
            );
            assert_eq!(
                f.path,
                format!("{ASSET_PREFIX}/{}", asset.path),
                "catalog.path 必须是「发布根基准」：assets/ + 台账 path"
            );
        }
    }

    /// **发布侧算真期望值**：产物在 `artifacts_dir` 里时，`sha256`/`size` 就是对真字节算的。
    ///
    /// 这条是上一条的镜像：随包侧不登记，发布侧**必须**登记（远端目录是下载期望值的权威）。
    /// 两边都不登记 = 下载永远没有依据。
    #[test]
    fn published_catalog_expects_the_real_bytes() {
        let repo = repo_root();
        let presets =
            crate::presetdata::Presets::load_from(&repo.join("presets")).expect("源读得出来");
        let artifacts = repo
            .join("crates")
            .join("preset")
            .join("assets")
            .join("presets");
        let catalog = Catalog::build_from_presets_lenient(&presets, &artifacts);

        let mkp: Vec<&CatalogFile> = catalog
            .files
            .iter()
            .filter(|f| f.kind == kind::PRESET)
            .collect();
        assert!(!mkp.is_empty(), "发布侧该登记 MKP 交付文件");
        for f in mkp {
            let bytes = std::fs::read(artifacts.join(&f.file_name))
                .unwrap_or_else(|e| panic!("产物 {} 读不出来：{e}", f.file_name));
            assert_eq!(
                f.expected_size(),
                Some(bytes.len() as u64),
                "{} 的大小",
                f.file_name
            );
            assert_eq!(
                f.expected_sha(),
                Some(hex(&Sha256::digest(&bytes)).as_str()),
                "{} 的 SHA 要对真字节算",
                f.file_name
            );
        }
    }

    /// **加厚判据：definition 真的进了 catalog。**
    ///
    /// 这不是"字段存在"的形状检查，而是**反空转**的精确计数 —— collect 忘抄某一域
    /// （assets / bundles / registry 各一行代码），客户端那一屏就静默地缺一块。
    /// 条数与源 TOML 对齐（变了要在提交里说清为什么，与 presetdata 的真数据判据同一纪律）
    #[test]
    fn the_definition_travels_with_the_catalog() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");

        assert_eq!(catalog.brands.len(), 1, "实测 1 个品牌");
        assert_eq!(
            catalog.assets.len(),
            29,
            "实测 29 条资产定义（9 MKP 预设 + 4 整机图 + 1 品牌字标 + 9 BBS + 3 图标 + 3 模型）——
             2026-10-03：整机图回到台账（bundled 档）、MKP 预设也进了台账（mkPreset 类、
             登记归属不登记路径；两者都不进 files[]）"
        );
        assert_eq!(
            catalog.bundles.len(),
            9,
            "2026-10-03 起一版一套：A1 三版 + A1_MINI 三版 + P1S / P2S / X1C 各一（照 C15 模型重排）"
        );
        assert_eq!(catalog.registry.params.len(), 74, "实测 74 条字段定义");
        assert!(
            !catalog.registry.tabs.is_empty() && !catalog.registry.layout.is_empty(),
            "页签元数据与参数摆放都该在"
        );

        // 机型的完整字段跟着走：A1 的别名、尺寸与版本定义
        let a1 = catalog.machine("A1").expect("A1 必须在");
        assert_eq!(a1.external_aliases, vec!["A1C", "A1F"]);
        assert!(a1.dimensions.is_some(), "A1 配了 [dimensions]");
        let fast = a1
            .versions
            .iter()
            .find(|v| v.id == "FASTV3.3")
            .expect("A1 的第三版");
        assert_eq!(fast.name, "快拆版260628");

        // 禁区：只有 P1S / P2S / X1C 有
        assert!(catalog.machine("A1").unwrap().zones.is_empty());
        assert!(
            !catalog.machine("P1S").unwrap().zones.is_empty(),
            "P1S 有禁区"
        );

        // 访问面：file_of 是 MKP 引用的唯一出处（不再按命名规则重算）
        let f = catalog.file_of("A1", "FASTV3.3").expect("file_of 要找得到");
        assert_eq!(f.path, "dist/mkp/presets/A1-fastv3.3.toml");
        assert!(catalog.file_of("A1", "NOPE").is_none());
    }

    /// definition 的 serde 往返必须无损：序列化出去的 catalog 解析回来，逐字段一致。
    /// 加厚引入了一整批复用类型（ParamDef / LayoutTab / Dimensions…），任何一处的
    /// serde 属性配错（缺 default、rename 不对）都会在这里现形 —— 而不是等客户端首屏空了才查
    #[test]
    fn the_thick_catalog_round_trips_through_json() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");
        let json = catalog.to_pretty_json().expect("序列化失败");
        let back = Catalog::parse(json.as_bytes()).expect("自己写出的目录要读得回来");
        assert_eq!(back.revision, catalog.revision);
        assert_eq!(back.machines.len(), catalog.machines.len());
        assert_eq!(back.assets.len(), catalog.assets.len());
        assert_eq!(back.bundles.len(), catalog.bundles.len());
        assert_eq!(back.registry.params.len(), catalog.registry.params.len());
        assert_eq!(back.registry.layout.len(), catalog.registry.layout.len());
        // 抽一条参数逐字段比：serde 属性配错最先在这种地方现形
        assert_eq!(
            back.registry.params[0].key, catalog.registry.params[0].key,
            "第一条参数的 key 往返丢了"
        );
    }

    #[test]
    fn revision_tracks_the_inputs() {
        let mut a = Catalog::build_from_repo(&repo_root()).unwrap();
        let b = Catalog::build_from_repo(&repo_root()).unwrap();
        assert_eq!(a.revision, b.revision, "同样的输入，指纹必须一样");

        // 交付文件的期望值变了 → 指纹得跟着变（发布侧换了一版产物就是这种情形）
        a.files[0].sha256 = Some("0".repeat(64));
        a.revision = revision_of(&a);
        assert_ne!(a.revision, b.revision, "文件字节变了，指纹得跟着变");

        // **definition 也在指纹的输入里**：改一个字段定义，检查更新要看得见
        a.registry.params[0].label = "改过的名字".to_owned();
        a.revision = revision_of(&a);
        assert_ne!(a.revision, b.revision, "定义变了，指纹也得跟着变");
    }

    /// 格式代次不认就拒 —— 往前与往后都不许静默错读
    #[test]
    fn parse_rejects_a_future_schema() {
        let json = r#"{ "catalogSchema": 99, "revision": "x", "machines": [], "files": [] }"#;
        let e = Catalog::parse(json.as_bytes()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }
}
