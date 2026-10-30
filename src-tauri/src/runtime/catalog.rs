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
//! （那是文件条目里每个 `sha256` 的事）。
//!
//! # `publishedAt`（2026-10-06）：目录里唯一的可信时间
//!
//! 预设页云端表的「时间」要显示"这次发布的时刻"，来源只有发布侧的落盘那一刻 ——
//! 所以 `publishedAt` 由**发布侧写**（工作台 `write_catalog_json`，与 manifest 的
//! `updated` 同一个戳），随目录字节走到每台客户端。两条边界：
//!
//! - **随包 bootstrap 目录不带它**：`gen-catalog` 的产物必须逐字节可复现
//!   （`embedded_matches_rebuild` 钉着），而且"随包"没有发布事件 —— 编一个就是撒谎；
//!   那种目录在界面上照实说「未知」；
//! - **不进 revision 输入**：同内容重发指纹不变，检查更新不该被一次重发惊动；
//!   `#[serde(default)]` 也进不了结构签名（探针判的就是"删了还能不能解析"）。

use postprocess::diag::messages;

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
    /// **发布时刻**（RFC3339 / UTC，秒级）。发布侧落盘时盖（工作台 `write_catalog_json`，
    /// 与 manifest 的 `updated` 同一个戳）；随包 bootstrap 目录**刻意不带**（见模块头），
    /// 界面对那种目录照实说「未知」
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
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
    /// **分组登记**（2026-10-16）：哪几张图共用同一份标注源稿 —— 组名 / 成员 / 每个成员的角色。
    /// 客户端按它挑图（作者：「客户端也不该写死 id」），工作台按它摆"套装"。
    /// 与 `assets` 同一类东西：进 catalog = 云端可消费 ⇒ 改组不用重编客户端。
    #[serde(default)]
    pub groups: Vec<crate::presetdata::Group>,
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
    /// **发布线身份**（`support-ease`）：逻辑上是同一条发布线的目录都写同一个值 ——
    /// GitHub / Gitee 镜像与随包 bootstrap 因为来自同一仓库的同一笔提交而天然同 ID。
    ///
    /// ★ 它是**本程序自己的交付线名**（取自产品名），**不代表任何厂商官方身份**：
    ///   这套预设是第三方整理分发的。不写 = 不知道是哪条线，**不是"默认官方"**。
    ///
    /// # 为什么不能推导
    ///
    /// 路径、品牌、机型集合相同**都不证明**是同一条线（作者 2026-10-28）：两个独立目录
    /// 恰好长一样，也会是不同的演化。所以它只能由发布账本（
    /// `presets/delivery/publish-serial.json`）持有并被抄进这里，不从别处推。
    ///
    /// **不进 [`revision_of`]**：与 `published_at` 同一条理由 —— 它不描述目录内容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    /// **单调发布序号**：只有正式发布事务（`workbench::app::delivery::publish_into`）
    /// 才能让它 +1，`gen-catalog` / 生成只沿用上一次发布的值。
    ///
    /// 它回答的是 `revision` 回答不了的那个问题：**发布有没有往前走**。
    /// `revision` 是内容指纹（SHA，不可比较大小）；这里才是顺序。回滚 = 一次新的发布
    /// （serial 继续 +1，而 `revision` 回到旧值），所以序号只增不减。
    ///
    /// 与 [`Catalog::catalog_id`] **同生同死**（只有一个 ⇒ 目录是坏的，见 [`Catalog::parse`]）。
    ///
    /// **不进 [`revision_of`]**。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_serial: Option<u64>,
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
pub const REPO_ASSET_ROOT: &str = "presets/assets";

/// **发布根**（`catalog.path` 的基准，即 Manifest 的 `filesRoot: ".."` 落点）—— 相对仓库根。
///
/// ★ 2026-10-04 定：**发布根 = `presets/`**。交付根（`presets/delivery/`，source.json
/// 所在目录）是它的子目录；`catalog.path` 相对发布根，客户端由 Manifest 的
/// `filesRoot` 声明跳出去 —— 锚点差异是 Resolver 的实现细节
/// （`docs/RESOURCE-ADDRESSING-ROADMAP.md` §2.4）。
pub const REPO_PUBLISH_ROOT: &str = "presets";

/// 资产在发布根下的第一段目录名：`assets/<台账 path>`。
///
/// ★ **这一段不能省**：`baseUrl` = `.../presets/delivery/`，A 类资产在 `.../presets/assets/` ——
/// 兄弟目录，必须靠这个前缀跳到正确位置。见 `docs/PUBLISH-ARCHITECTURE.md` §2.2。
const ASSET_PREFIX: &str = "assets";

/// B 类渲染产物（MKP 预设 TOML）在发布根下的目录：`delivery/mkp/presets/`。
///
/// 与 [`ASSET_PREFIX`] 对称：A 类原地不动，B 类落在 `delivery/` 里（它源里没有实体）。
pub const PRESET_DEST_DIR: &str = "delivery/mkp/presets";

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
/// **返回 `None` 就是不登记**。今天三种：
///
/// - `Image`：图片**不进下载面**（整机图 / 品牌图都走 `bundled` 档随包，
///   客户端靠构建期装配拿它们，不按 URL 取）；
/// - `Guide`（2026-10-16）：引导图同上 —— 今天 `bundled` 随包、走
///   `/assets/<path>` 取；**将来要按需下载时，这一行才给目录**
///   （见 `presetdata::AssetKind::Guide` 那段）；
/// - `MkPreset`：MKP 预设的产物文件**已经**作为 `files` 条目进来了（B 类，
///   生成侧算的，落点 `delivery/mkp/presets/…`）—— 台账这一条只登记「哪一版叫什么、归谁」，
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
        crate::presetdata::AssetKind::Guide => None,
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
/// （`presets/delivery/`），而 A 类资产在 `presets/assets/` —— 兄弟目录，故必须带
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
        let catalog: Catalog = serde_json::from_slice(bytes).map_err(|e| {
            AppError::corrupted(messages::text("ui.runtime.catalog.377", &[]))
                .with_detail(e.to_string())
        })?;
        if catalog.catalog_schema != CATALOG_SCHEMA {
            return Err(AppError::corrupted(messages::text(
                "ui.runtime.catalog.378",
                &[
                    ("v1", &catalog.catalog_schema.to_string()),
                    ("v2", &CATALOG_SCHEMA.to_string()),
                ],
            )));
        }
        /*
         * **发布身份同生同死**（作者 2026-10-28）：只有一半 = 这份目录被谁手工改坏了。
         *
         * 不补救（不替它推一个 ID、也不假定 serial）—— 那种"猜一半"正是这次要根治的
         * 毛病。老的、从来没带过这两格的目录照常读（两格都 `None` = 不知道哪条线）。
         */
        if catalog.catalog_id.is_some() != catalog.publish_serial.is_some() {
            return Err(AppError::corrupted(messages::text(
                "ui.runtime.catalog.382",
                &[
                    (
                        "id",
                        &catalog
                            .catalog_id
                            .clone()
                            .unwrap_or_else(|| "catalogId 缺失".to_owned()),
                    ),
                    (
                        "serial",
                        &catalog
                            .publish_serial
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "publishSerial 缺失".to_owned()),
                    ),
                ],
            )));
        }
        Ok(catalog)
    }

    /// **这份目录属于哪条发布线**（`None` = 不知道）。
    pub fn line(&self) -> Option<&str> {
        self.catalog_id.as_deref()
    }

    /// **这份目录是这条线的第几次发布**（`None` = 不知道；随包旧目录就是这种）。
    pub fn serial(&self) -> Option<u64> {
        self.publish_serial
    }

    /// 产物形态：pretty JSON + 结尾换行（进仓库当判据资产，diff 要能看）
    pub fn to_pretty_json(&self) -> Result<String, AppError> {
        let mut s = serde_json::to_string_pretty(self).map_err(|e| {
            AppError::internal(messages::text("ui.runtime.catalog.379", &[]))
                .with_detail(e.to_string())
        })?;
        s.push('\n');
        Ok(s)
    }

    /// **② 里没有、而随包那份有的「随包」资产**，从随包那份补进这份的副本 ——
    /// 「随包那份只兜底」（作者 2026-10-16 定的口径）。
    ///
    /// # 为什么要有这么一步
    ///
    /// 随包资产（`delivery = bundled`：机型整机图、品牌图、引导图…）的**字节随程序本体走**
    /// —— 构建期复制进 `/assets/`，客户端**不下载、不更新**它（见 [`dest_of_asset`]）。
    /// 那么"它叫什么、在哪"这件事，**程序自己带着的那份目录就足以作答**；
    /// ②（从云端来的那份）只是"恰好也登记着它"。
    ///
    /// 而 ② 是 OTA 来的、可能**比程序旧**（`ensure_released` 只在缺文件时铺底；
    /// 进预设页那次后台对齐又是"指纹不一样就采用远端"，**没有先后概念**）。
    /// 于是会出现：程序里明明带着两张引导图，② 却是加它们之前的那一版 ⇒ 界面上
    /// 找 id 找不到、图凭空消失（2026-10-16 实测，作者：「诶图片不见了」）。
    /// 补上这一条之后，**随包的东西不再依赖任何一份云端文件**。
    ///
    /// # 只补 bundled，且**② 里有的绝不动**
    ///
    /// - 缺的是 `bundled` ⇒ 补（它的字节就在程序里，补了立刻能显示）；
    /// - 缺的是别的（进交付集合、要按需下载的那些）⇒ **不补** —— 那些要配云端字节
    ///   与 `files[]` 才成立，凭空补一条只会给出一个下不到的条目（"不下载不使用"）；
    /// - ② 里**已经有**同 id 的 ⇒ 一个字不动（② 是运行时的真值，铁律 4）。
    pub fn with_missing_bundled_from(&self, embedded: &Catalog) -> Catalog {
        let mut out = self.clone();
        for asset in &embedded.assets {
            if asset.delivery != crate::presetdata::assets::Delivery::Bundled {
                continue;
            }
            if out.assets.iter().any(|a| a.id == asset.id) {
                continue;
            }
            out.assets.push(asset.clone());
        }
        /*
         * **分组也要兜底**（2026-10-16）：客户端靠 `catalog.groups` 挑图（不认文件名），
         * ② 若比程序旧（还没有这一组），那套引导图就会"两张图都在、却不知道哪张是浅色"
         * —— 与资产那条同一个病。补法同样保守：**id 不在 out 里**、且**成员在补完后的
         * `assets` 里都找得到**的组才补（否则凭空造出一条指着不存在资产的登记）。
         */
        for g in &embedded.groups {
            if out.groups.iter().any(|x| x.id == g.id) {
                continue;
            }
            let all_present = g.members.iter().all(|m| {
                out.assets
                    .iter()
                    .any(|a| a.id.eq_ignore_ascii_case(m.asset.trim()))
            });
            if all_present {
                out.groups.push(g.clone());
            }
        }
        /*
         * **机型的图位也要兜底**（2026-10-30 作者：「随包的到底受不受云端的影响？
         * 云端的比较旧、随包的比较新，会被覆盖吗」）。
         *
         * 上面那条只补了 `assets` / `groups`，机型自己的 `image` / `imageVariant` /
         * `icon` 是**机型行上的字段**，② 旧 ⇒ 那几格是 `None` ⇒ 首页查不到图、
         * 一路回落到品牌字标。2026-10-30 补 P2S / X1C 外观图时正撞上这条：
         * 程序里带着图、盘上那份目录（第 1 号）里 `image` 还是空，界面上就是没图。
         *
         * 补法同口径，**只补空位、② 说了话就一个字不动**：
         * - ② 那台机型的 `image` 是 `None` 且随包那份有 ⇒ 抄随包那份的；
         * - `imageVariant` / `icon` 同理；
         * - ② 里根本没有这台机型（停用了 / 新加的）⇒ **不补** —— 机型在不在是目录说了算。
         */
        for m in &mut out.machines {
            let Some(src) = embedded.machines.iter().find(|x| x.id == m.id) else {
                continue;
            };
            if m.image.is_none() {
                m.image = src.image.clone();
            }
            if m.image_variant.is_none() {
                m.image_variant = src.image_variant.clone();
            }
            if m.icon.is_none() {
                m.icon = src.icon.clone();
            }
        }
        out
    }
}

impl Catalog {
    /// 从**层①**构建：`<repo>/presets`（源定义）+ `crates/preset/assets/presets`（交付产物真字节）。
    ///
    /// 产物字节取**入库**的那一份（`BUILTIN_PRESETS` 编进二进制的同一批文件）——
    /// `presets/delivery/` 是本机 gitignore 掉的暂存，进不了 CI，不能当判据输入。
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
            return Err(AppError::not_found(messages::text(
                "ui.runtime.catalog.380",
                &[("first", &first.to_string())],
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
    /// - 发布侧（工作台 `wb_publish`）：产物目录 = `delivery/mkp/presets`（与客户端落点同形），
    ///   **宽松**——没有产物的版本是合法状态（交付集合本来就不含它），跳过。
    ///
    /// 保留这个严格版是**给手工/未来的严格发布**用；当前工作台只调宽松版。
    pub fn build_from_presets(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Result<Catalog, AppError> {
        let (catalog, missing) = Self::collect(presets, artifacts_dir, FileHashes::FromArtifacts);
        if let Some(first) = missing.first() {
            return Err(AppError::not_found(messages::text(
                "ui.runtime.catalog.381",
                &[("first", &first.to_string())],
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
            /*
             * **停用的机型整台不进目录**（2026-10-30）：机型与它的预设文件都不发。
             * 客户端因此看不到它（`getMachines` 读的就是这份目录），后处理也查不到它
             * （`hook` 按"目录里没有这台机型"拒绝）。源文件照留、引用都不用动 ——
             * 重新启用 = 去掉 `enabled = false` 再发一版。
             */
            if !m.enabled {
                continue;
            }
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
                        missing.push(::postprocess::diag::messages::text(
                            "ui.runtime.catalog.asset_payload_missing",
                            &[("id", &asset.id), ("path", &asset.path)],
                        ));
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
                // 发布时刻只归发布侧盖 —— 构建出来的目录（随包 / 预检）一律不带
                published_at: None,
                brands: presets.catalog.brands().to_vec(),
                machines,
                assets: presets.assets.items().to_vec(),
                groups: presets.assets.groups().to_vec(),
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
                /*
                 * **构建出来的目录不带发布身份**（2026-10-28）：它回答不了"这是哪条线的
                 * 第几次发布"—— 那是发布事务的事（`publish_into` 从账本抄进来）。
                 * 随包 bootstrap 也走这条路 ⇒ 它要么是发布产物（带身份），要么老实说不知道。
                 */
                catalog_id: None,
                publish_serial: None,
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
///
/// ★ **发布身份（`catalogId` / `publishSerial`）刻意不进这里**（作者 2026-10-28）：
///   `revision` 是内容指纹，同内容重发不许变；序号是另一件事（顺序，不是内容）。
///   两者混一个数，就又回到"一个字段同时回答三个问题"的老毛病。
fn revision_of(catalog: &Catalog) -> String {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Payload<'a> {
        brands: &'a [crate::presetdata::Brand],
        machines: &'a [CatalogMachine],
        assets: &'a [Asset],
        /// ★ 分组也在指纹里：改组（换成员）必须是"检查更新看得见"的一次变化 ——
        /// 不然云端换了组，客户端比 `revision` 说"已是最新"，那套图就一直是旧的
        groups: &'a [crate::presetdata::Group],
        bundles: &'a [Bundle],
        registry: &'a CatalogRegistry,
        plates: &'a [crate::presetdata::Plate],
        files: &'a [CatalogFile],
    }
    let bytes = serde_json::to_vec(&Payload {
        brands: &catalog.brands,
        machines: &catalog.machines,
        assets: &catalog.assets,
        groups: &catalog.groups,
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

        // 预设：9 份，落点 `delivery/mkp/presets/` 下（发布根基准，B 类）
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
        //   - catalog 的 `files[]` 里**要没有**它（不进交付预设那一档，客户端不下载）。
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
            "整机图一律 bundled 档：台账登记、工作台可管，但不进交付预设那一档"
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
        // 今天 35 条 = 9 MKP 预设 + 6 整机图（bundled）+ 1 品牌字标 + 9 BBS + 3 图标
        //              + 5 模型（Z / XY 各多一版「加长版」）+ 2 引导图（bundled，2026-10-16）
        assert_eq!(
            catalog.assets.len(),
            source.assets.items().len(),
            "catalog 的资产定义与 presets/assets.toml 逐条对齐"
        );
        assert_eq!(
            catalog.assets.len(),
            35,
            "实测 35 条（9 MKP 预设 + 6 整机图 + 1 品牌字标 + 9 BBS + 3 图标 + 5 模型 + 2 引导图）——
             2026-10-03 三类进台账（mkPreset / bundled image / 品牌图）、2026-10-16 加了引导图
             （`type = 'guide'`，bundled）、2026-10-18 加了 Z 校准加长版
             （`zoffset-calibration-ext`）、2026-10-19 加了 XY 校准加长版
             （`precise-calibration-ext`）、2026-10-30 加了 P2S / X1C 外观图
             （`p2s-image` / `x1c-image`，bundled）——条数变了要核对台账再改这里的期望"
        );

        // 文件条目与命名规则对得上：A1 + FASTV3.3 → A1-fastv3.3.toml
        let a1_fast = catalog
            .files
            .iter()
            .find(|f| f.machine_id == "A1" && f.version_id == "FASTV3.3")
            .expect("A1/FASTV3.3 该有交付产物");
        assert_eq!(a1_fast.file_name, "A1-fastv3.3.toml");
        assert_eq!(a1_fast.path, "delivery/mkp/presets/A1-fastv3.3.toml");
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
                f.path.starts_with("assets/") || f.path.starts_with("delivery/"),
                "落点必须是发布根基准（assets/ 或 delivery/）：{}",
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
             * `workbench::app::delivery::audit_catalog` 与 `publish_into` 的收尾核对守着。
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
            35,
            "实测 35 条资产定义（9 MKP 预设 + 6 整机图 + 1 品牌字标 + 9 BBS + 3 图标 + 5 模型
             + 2 引导图）——
             2026-10-03：整机图回到台账（bundled 档）、MKP 预设也进了台账（mkPreset 类、
             登记归属不登记路径；两者都不进 files[]）；2026-10-16：引导图进台账（guide 类、bundled）；
             2026-10-18：Z 校准加长版进台账（model 类）；2026-10-19：XY 校准加长版进台账（model 类）；
             2026-10-30：P2S / X1C 外观图进台账（image 类、bundled）"
        );
        assert_eq!(
            catalog.bundles.len(),
            9,
            "2026-10-03 起一版一套：A1 三版 + A1_MINI 三版 + P1S / P2S / X1C 各一（照 C15 模型重排）"
        );
        assert_eq!(catalog.registry.params.len(), 75, "实测 75 条字段定义");
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
        assert_eq!(f.path, "delivery/mkp/presets/A1-fastv3.3.toml");
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

    /// **发布身份同生同死**（2026-10-28）：只写了一格 = 被手工改坏，不猜另一半。
    ///
    /// 三支一起咬：两格都缺（老目录）照常读、只缺一个就 `CORRUPTED`、两格都在就照读。
    /// 判据只认形状，不认值 —— "哪个 ID 才算官方"是发布账本的事，不在解析层。
    #[test]
    fn the_release_identity_is_all_or_nothing() {
        let base = r#""catalogSchema": 1, "revision": "abc", "machines": [], "files": []"#;

        let neither = format!("{{ {base} }}");
        let c = Catalog::parse(neither.as_bytes()).expect("老目录（两格都缺）照常读");
        assert_eq!(c.line(), None, "没写 = 不知道是哪条线，不是默认官方");
        assert_eq!(c.serial(), None);

        let id_only = format!(r#"{{ {base}, "catalogId": "support-ease" }}"#);
        assert_eq!(
            Catalog::parse(id_only.as_bytes()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted,
            "只有 ID 没有序号 —— 不许替它编一个序号"
        );

        let serial_only = format!(r#"{{ {base}, "publishSerial": 7 }}"#);
        assert_eq!(
            Catalog::parse(serial_only.as_bytes()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted,
            "只有序号没有 ID —— 那序号说不清属于哪条线"
        );

        let both = format!(r#"{{ {base}, "catalogId": "support-ease", "publishSerial": 7 }}"#);
        let c = Catalog::parse(both.as_bytes()).expect("两格齐了照读");
        assert_eq!(c.line(), Some("support-ease"));
        assert_eq!(c.serial(), Some(7));
    }

    /// **发布身份不进内容指纹**：同内容重发（只有序号变）`revision` 必须不变。
    ///
    /// 反过来咬的是"别拿序号冒充内容"：序号变了而 `revision` 一分没动 = 一次
    /// 没有内容变化的发布（回执要说"产物无变化"，判定也据此判 `Unchanged`）。
    #[test]
    fn the_release_identity_stays_out_of_the_content_fingerprint() {
        let catalog = Catalog::build_from_repo(&repo_root()).unwrap();
        let before = catalog.revision.clone();

        let mut republished = catalog.clone();
        republished.catalog_id = Some("support-ease".to_owned());
        republished.publish_serial = Some(42);
        republished.published_at = Some("2026-10-28T00:00:00Z".to_owned());
        assert_eq!(
            revision_of(&republished),
            before,
            "序号 / 发布线 / 发布时刻都不描述内容 —— 指纹一动，OTA 就会以为内容变了"
        );

        /* 内容真的变了才许动指纹（这里改一份的字节） */
        let mut changed = republished.clone();
        changed.files[0].sha256 = Some("0".repeat(64));
        assert_ne!(revision_of(&changed), before, "内容变了，指纹得跟着变");
    }

    /// **随包那份只兜底**（2026-10-16，作者「编译进去的，就只当兜底」）——
    /// 三条一起咬：② 里没有的 bundled 补上、② 里已有的**绝不动**、下载面**不许补**。
    ///
    /// 反面就是那次事故：程序里带着两张引导图，② 是加它们之前的那一版
    /// ⇒ 界面按 id 找不着、图凭空消失（作者：「诶图片不见了」）。
    #[test]
    fn bundled_assets_fall_back_to_the_embedded_catalog_only_when_missing() {
        use crate::presetdata::assets::Delivery;
        use crate::presetdata::AssetKind;

        fn asset(id: &str, kind: AssetKind, path: &str, delivery: Delivery) -> Asset {
            Asset {
                id: id.to_owned(),
                kind,
                machine_id: None,
                name: id.to_owned(),
                path: path.to_owned(),
                version_id: None,
                slicer: None,
                profile: None,
                delivery,
                calibration: String::new(),
            }
        }

        let released = Catalog {
            assets: vec![asset(
                "a1-image",
                AssetKind::Image,
                "printers/a1.webp",
                Delivery::Bundled,
            )],
            ..Catalog::default()
        };
        let embedded = Catalog {
            assets: vec![
                /* ② 里也有这条（随包那份里路径不同）—— 以 ② 为准 */
                asset(
                    "a1-image",
                    AssetKind::Image,
                    "printers/换过的路径.webp",
                    Delivery::Bundled,
                ),
                /* ② 里没有 —— 该补（引导图就是这么丢的） */
                asset(
                    "guide-open-with-light",
                    AssetKind::Guide,
                    "guides/open-with-light.png",
                    Delivery::Bundled,
                ),
                /* 要按需下载的，② 里没有 —— 不许补（补了就是个下不到的条目） */
                asset(
                    "mkp-support-models",
                    AssetKind::Model,
                    "models/support-test.3mf",
                    Delivery::Download,
                ),
            ],
            ..Catalog::default()
        };

        let merged = released.with_missing_bundled_from(&embedded);

        assert_eq!(merged.assets.len(), 2, "只补进来一条");
        assert_eq!(
            merged.asset("a1-image").map(|a| a.path.as_str()),
            Some("printers/a1.webp"),
            "② 里已有的那条一个字都不许动"
        );
        assert_eq!(
            merged
                .asset("guide-open-with-light")
                .map(|a| a.path.as_str()),
            Some("guides/open-with-light.png"),
            "② 里没有的随包条目，从随包那份补上"
        );
        assert!(
            merged.asset("mkp-support-models").is_none(),
            "下载面的条目不许凭空补"
        );
        assert_eq!(released.assets.len(), 1, "原那份不动（补的是一份副本）");
    }

    /// **机型的图位同样兜底**（2026-10-30）：程序带着图、云上那份目录还是老的一版
    /// （机型行的 `image` 空着）—— 界面上不该没图。
    ///
    /// 边界同 [`Catalog::with_missing_bundled_from`] 那条：
    /// 只补**空位**，② 说了话的一个字不动；② 里根本没有这台机型就不补。
    #[test]
    fn machine_image_slots_backfill_from_the_bundled_catalog() {
        let machine = |id: &str, image: Option<&str>, icon: Option<&str>| CatalogMachine {
            id: id.to_owned(),
            display: id.to_owned(),
            name: String::new(),
            brand: String::new(),
            default_bundle: None,
            external_aliases: Vec::new(),
            image: image.map(str::to_owned),
            image_variant: None,
            icon: icon.map(str::to_owned),
            plate_ids: Vec::new(),
            default_plate_id: None,
            dimensions: None,
            versions: Vec::new(),
            zones: Vec::new(),
        };

        let released = Catalog {
            machines: vec![
                /* ② 还没配图（加图之前那一版目录） */
                machine("P2S", None, None),
                /* ② 已经说了话 —— 以它为准 */
                machine("A1", Some("a1-image-old"), Some("a1-icon")),
            ],
            ..Catalog::default()
        };
        let embedded = Catalog {
            machines: vec![
                machine("P2S", Some("p2s-image"), Some("p1s-icon")),
                machine("A1", Some("a1-image"), Some("a1-icon")),
                /* ② 里没有这台（停用 / 新加）—— 不许凭空补 */
                machine("X1C", Some("x1c-image"), None),
            ],
            ..Catalog::default()
        };

        let merged = released.with_missing_bundled_from(&embedded);

        assert_eq!(merged.machines.len(), 2, "机型条数由 ② 定，不补新机型");
        assert_eq!(
            merged.machine("P2S").and_then(|m| m.image.as_deref()),
            Some("p2s-image"),
            "空着的图位从随包那份补上"
        );
        assert_eq!(
            merged.machine("P2S").and_then(|m| m.icon.as_deref()),
            Some("p1s-icon"),
            "图标同理"
        );
        assert_eq!(
            merged.machine("A1").and_then(|m| m.image.as_deref()),
            Some("a1-image-old"),
            "② 里已有的一格都不许动"
        );
        assert!(merged.machine("X1C").is_none(), "② 里没有的机型不补");
    }
}
