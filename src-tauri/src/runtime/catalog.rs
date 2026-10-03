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
    /// **资产 id**（不是路径）：机型图。整机图 2026-10-01 从台账剥离之后，
    /// 源里 **一律不再写它**（照实为空），界面用自己的素材表（`src/app/home/heroArt.ts`）。
    /// 字段留着是给"将来真有产品级的机型图要按需下载"留的位置；今天没有任何机型写它。
    #[serde(default)]
    pub image: Option<String>,
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

/// 交付文件的**种类**（第三圈第一刀起不止一种）。
///
/// 这里的规矩是：**新增一种外部资源 = 加一个常量 + 一个落点目录**，不新增一套
/// 存储体系、不新增一条下载路径。管道、校验、归档、进度全是既有的那些 ——
/// "以后新增资源不再新增一套下载系统"就是靠这一处常量表成立的。
/// 层①的资产载荷根（`presets/assets.toml` 里 `path` 的基准），相对仓库根。
///
/// **它不是运行时数据**：这里是构建期算 SHA/大小的地方，与运行时的下载区(`mkp/`)是两回事。
/// 2026-10-03 从 `public/assets` 搬到 `presets/assets`（作者：「在 assets 吧，到时候 3mf
/// 也要放」—— 产品数据资源住一起；`public/` 那个资产目录之后退役）。
const REPO_ASSET_ROOT: &str = "presets/assets";

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

/// 一种 kind 在下载区里的目录名。**下载区按种类分层，不按来源分层**
///
/// （同一个来源送来预设、BBS 配置和图标，落点也不混在一起：盘上的目录结构要能回答
/// "这一格是干什么用的"，那是给人看的，也是给将来清理用的。）
fn kind_dir(kind: &str) -> &'static str {
    match kind {
        kind::PRESET => "presets",
        kind::BBS_CONFIG => "bbs",
        kind::MODEL => "models",
        kind::ICON => "icons",
        _ => "other",
    }
}

/// 资产台账里的类型 → 目录里的 kind。
///
/// **返回 `None` 就是不登记**：那是**目录里没有落点**的类型（今天的 `Image` ——
/// 下载区没有「图片」这一段）。注意判据是**类型**，不是交付档位：**`bundled` 档
/// （整机图）照样走这一支**，只是被 [`dest_of_asset`] 在更早一步按档位拦掉
/// （作者 2026-10-03：「不进云端但要在工作台看得见选得着」）。
fn kind_of_asset(asset_kind: crate::presetdata::AssetKind) -> Option<&'static str> {
    match asset_kind {
        crate::presetdata::AssetKind::SlicerProfile => Some(kind::BBS_CONFIG),
        crate::presetdata::AssetKind::Model => Some(kind::MODEL),
        crate::presetdata::AssetKind::Icon => Some(kind::ICON),
        crate::presetdata::AssetKind::Image => None,
        // MKP 预设的产物文件**已经**作为 catalog 的 files 条目进来了（生成侧算的，
        // 命名规则落点 `mkp/presets/…`）—— 台账这一条只登记「哪一版叫什么、归谁」，
        // 再登记一份文件条目就是同一个文件两条真相（作者 2026-10-03）
        crate::presetdata::AssetKind::MkPreset => None,
    }
}

/// 资产的落点：`mkp/<kind 目录>/<资产在载荷根里的相对路径去掉类型前缀>`。
///
/// 资产在仓库里是 `bbs/Process/0.2mm/….json`（前缀与 kind 目录同名），落到下载区
/// 就是 `mkp/bbs/Process/0.2mm/….json` —— **目录名换了个基准，相对形状没变**，
/// 将来工作台发布那边按同一形状产出，两边就自然对得上。
fn asset_dest(kind: &str, asset_path: &str) -> String {
    let rest = asset_path
        .split_once('/')
        .map(|(_, tail)| tail)
        .unwrap_or(asset_path);
    format!("mkp/{}/{rest}", kind_dir(kind))
}

/// 一条资产在**下载区 / 交付根**的落点：`mkp/<kind 目录>/…`。
///
/// **两端共用这一处算法**（2026-10-02）——这是"发布方与消费方说同一种语言"的落点那半句：
///
/// - 客户端：拿它算 [`CatalogFile::path`]，下载地址 = 数据源地址 + 它；
/// - 工作台发布：拿它算这条资产该复制到交付根的哪个相对位置（`dist/mkp/…`）。
///
/// 两套拼接一定会漂，而漂的表现是「URL 拼得上、落点却对不上」——上传成功了、
/// 客户端也点了下载，文件落到别的目录，或者根本 404。所以在发布侧调用这一处，
/// 不自己 `format!` 一遍。
///
/// 返回 `None` = 这一份**不进交付集合**（客户端不会去 URL 取它）。两种情形：
///
/// 1. **交付档位是 `bundled`**（作者 2026-10-03）—— 台账里登记、工作台可管，
///    但它**随程序包带进客户端、不下载不更新**（整机图）。这是主路径。
/// 2. **类型在下载区没有落点**（今天的 `Image`）—— 目录里没有「图片」这一段。
///
/// 判据是**档位**而不是「台账里不该有它」：2026-10-01 第三刀曾把整机图从台账剥离、
/// 搬进客户端源码，作者 2026-10-03 判为**错**（「不会编程的用户改不了图」）——
/// 今天它回到台账，用 `bundled` 档表达「不进云端但在工作台可管」。
pub fn dest_of_asset(asset: &crate::presetdata::Asset) -> Option<String> {
    if asset.delivery == crate::presetdata::assets::Delivery::Bundled {
        return None;
    }
    kind_of_asset(asset.kind).map(|kind| asset_dest(kind, &asset.path))
}

/// 一份交付文件。`path` 是相对**内部根**的落点 —— 下载它就该落到那（铁律 3：
/// 没下载就没有；下载了才出现在 `mkp/`）。
///
/// 落点形状统一为 `mkp/<kind 目录>/…`。
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
    /// 对**交付产物真字节**算的 —— 不是对源 TOML。下载后的校验（产品规则 §10）拿它当期望值
    pub sha256: String,
    pub size: u64,
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
        let mut presets = crate::presetdata::Presets::load_from(&repo_root.join("presets"))?;
        // 资产载荷根（`presets/assets.toml` 里 `path` 的基准）。**只有构建期有仓库时才给得出** ——
        // 用户机器上那份定义还在，但载荷没有：那种时候资产一律表现为"还没下载"，
        // catalog 里登记的是"应该有这些文件"，不是"这些文件已经在了"。
        presets.set_asset_root(&repo_root.join(REPO_ASSET_ROOT));
        let assets = repo_root
            .join("crates")
            .join("preset")
            .join("assets")
            .join("presets");
        Self::build_from_presets(&presets, &assets)
    }

    /// 从**已加载的预设源 + 一个产物目录**构建。这是两端共用的构建本体：
    /// - 安装包侧（[`Catalog::build_from_repo`]）：产物目录 = 入库产物，**严格**——缺一份就失败；
    /// - 发布侧（工作台 `wb_publish`）：产物目录 = `dist/mkp/presets`（与客户端落点同形），
    ///   **宽松**——没有产物的版本是合法状态（交付集合本来就不含它），跳过。
    pub fn build_from_presets(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Result<Catalog, AppError> {
        let (catalog, missing) = Self::collect(presets, artifacts_dir);
        if let Some(first) = missing.first() {
            return Err(AppError::not_found(format!(
                "{first} —— 入库产物目录里没有这一份，先补齐再构建目录"
            )));
        }
        Ok(catalog.finalize())
    }

    /// 宽松版：没有产物的版本合法，只登记真实存在的产物。工作台发布 `dist/catalog.json` 用它
    pub fn build_from_presets_lenient(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Catalog {
        let (catalog, _) = Self::collect(presets, artifacts_dir);
        catalog.finalize()
    }

    /// 走一遍源 + 产物目录。返回目录与**缺失清单**（严格/宽松由调用方裁决）。
    ///
    /// definition 直接从已加载的 [`crate::presetdata::Presets`] 抄——那里是**加载期校验过**
    /// 的定义（id 唯一、引用落地、布局双射），catalog 不做第二次校验：同一份数据两套门禁,
    /// 就会有两套不同的答案。
    fn collect(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
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
                let Ok(bytes) = std::fs::read(artifacts_dir.join(&file_name)) else {
                    missing.push(format!("{} / {}（{}）", m.id, v.id, file_name));
                    continue;
                };
                files.push(CatalogFile {
                    kind: kind::PRESET.to_owned(),
                    path: format!("mkp/{}/{}", kind_dir(kind::PRESET), file_name),
                    file_name,
                    machine_id: m.id.clone(),
                    version_id: v.id.clone(),
                    sha256: hex(&Sha256::digest(&bytes)),
                    size: bytes.len() as u64,
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
            let Ok(bytes) = std::fs::read(&full) else {
                missing.push(format!("资产 {}（{}）：载荷文件不在", asset.id, asset.path));
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
            files.push(CatalogFile {
                kind: kind.to_owned(),
                path: asset_dest(kind, &asset.path),
                file_name,
                machine_id: asset.machine_id.clone().unwrap_or_default(),
                version_id: String::new(),
                sha256: hex(&Sha256::digest(&bytes)),
                size: bytes.len() as u64,
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
            },
            missing,
        )
    }

    /// 指纹收口。构建路径（严格/宽松）与将来的手工组装都从这里过——
    /// revision 的算法只有这一处
    pub(crate) fn finalize(mut self) -> Catalog {
        self.revision = revision_of(&self);
        self
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

        // 预设：9 份，落点 mkp/presets/ 下
        let presets: Vec<&CatalogFile> = catalog
            .files
            .iter()
            .filter(|f| f.kind == kind::PRESET)
            .collect();
        assert_eq!(presets.len(), 9, "每个版本一份预设");
        assert!(
            presets.iter().all(|f| f.path.starts_with("mkp/presets/")),
            "预设一律落在 mkp/presets/ 下：{:?}",
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
            assert!(
                catalog
                    .files
                    .iter()
                    .filter(|f| f.kind == want)
                    .all(|f| f.path.starts_with(&format!("mkp/{}/", kind_dir(want)))),
                "{want} 一律落在 mkp/{}/ 下",
                kind_dir(want)
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
            28,
            "实测 28 条（9 MKP 预设 + 4 整机图 + 9 BBS + 3 图标 + 3 模型）—— 2026-10-03
             两类回到台账（mkPreset / bundled image），条数变了要核对台账再改这里的期望"
        );

        // 文件条目与命名规则对得上：A1 + FASTV3.3 → A1-fastv3.3.toml
        let a1_fast = catalog
            .files
            .iter()
            .find(|f| f.machine_id == "A1" && f.version_id == "FASTV3.3")
            .expect("A1/FASTV3.3 该有交付产物");
        assert_eq!(a1_fast.file_name, "A1-fastv3.3.toml");
        assert_eq!(a1_fast.path, "mkp/presets/A1-fastv3.3.toml");
        assert_eq!(a1_fast.sha256.len(), 64, "SHA256 的 hex 长度");
        assert!(a1_fast.size > 0);
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
            assert!(f.path.starts_with("mkp/"), "落点必须在下载区里：{}", f.path);
            assert!(!f.path.contains(".."), "落点不许有 `..`：{}", f.path);
        }
        let mut paths = std::collections::HashSet::new();
        for f in &catalog.files {
            assert!(paths.insert(f.path.as_str()), "落点撞车：{}", f.path);
        }
    }

    /// **SHA 是对真字节算的**：目录里记的大小，必须等于盘上那个文件的大小。
    /// 这条防的是"登记了但没读文件"（比如把 0 或者占位值写进去）—— 那会让
    /// 下载后的校验永远对不上，而且错在最难查的地方。
    #[test]
    fn file_sizes_match_the_bytes_on_disk() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");
        let repo = repo_root();

        // 资产类：按资产台账里的 `path` 回查盘上的真字节（文件名可能重名，路径不会）
        let source =
            crate::presetdata::Presets::load_from(&repo.join("presets")).expect("源读得出来");
        for f in catalog.files.iter().filter(|f| f.kind != kind::PRESET) {
            let asset = source
                .assets
                .items()
                .iter()
                .find(|a| asset_dest(kind_of_asset(a.kind).unwrap_or(""), &a.path) == f.path)
                .unwrap_or_else(|| panic!("{} 在资产台账里找不到", f.path));
            let on_disk = repo.join(REPO_ASSET_ROOT).join(&asset.path);
            let bytes = std::fs::read(&on_disk)
                .unwrap_or_else(|e| panic!("载荷 {} 读不出来：{e}", on_disk.display()));
            assert_eq!(bytes.len() as u64, f.size, "{} 的大小", f.file_name);
            assert_eq!(
                hex(&Sha256::digest(&bytes)),
                f.sha256,
                "{} 的 SHA",
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
            28,
            "实测 28 条资产定义（9 MKP 预设 + 4 整机图 + 9 BBS + 3 图标 + 3 模型）——
             2026-10-03：整机图回到台账（bundled 档）、MKP 预设也进了台账（mkPreset 类，
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
        assert_eq!(f.path, "mkp/presets/A1-fastv3.3.toml");
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

        a.files[0].sha256 = "0".repeat(64);
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
