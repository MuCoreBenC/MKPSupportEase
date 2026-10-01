//! 正式客户端的预设读命令 —— A41 预设页的真后端。
//!
//! # 数据源是 catalog（第二圈换源后的唯一数据源）
//!
//! 每条命令都从 [`runtime::load_released_catalog_bytes`] 读**释放进内部根的那份**
//! `<appDataDir>/catalog.json`，不是仓库、也不再是铺进数据根的 13 份源 TOML
//! （旧世界 `client/` 已随本次换源退役）。九条命令的 **DTO 一个都没动** ——
//! 变的只是后面喂数的那只手。见 `client::paths` 退役前的 DATA-INVENTORY §3.1 R3/R4/R5。
//!
//! | 命令 | 数据源 |
//! |---|---|
//! | `get_machines` | catalog 机型的完整定义（含尺寸、禁区、品牌显示名） |
//! | `get_version_files` | MKP 那支 = catalog 的文件条目（**带真 size/SHA**）；切片器支 = 套餐 → 资产 |
//! | `get_preset_files` | catalog 资产定义（**只出切片器预设**，见下） |
//! | `get_menu` | 资产定义 + 套餐反查 |
//! | `get_param_meta` | 字段定义（**全部**，含废弃） |
//! | `get_machine_params` | `machineVariants` 的三层取值 |
//! | `get_local_files` | 本轮恒为空集合（见下） |
//! | `get_local_user_files` | 本轮恒为空集合 |
//! | `get_slicer_copied` | 本轮恒为空集合 |
//!
//! # 只出切片器预设，不出机型图
//!
//! catalog 的资产定义里同时登记机型图 / 图标 / 模型，而 A41 那张「云端表」要的是
//! **可下载的预设文件**。把一张 `.webp` 贴成 `mkp_preset` 塞进 MKP 表是错的 ——
//! 所以 [`get_preset_files`] 只出 [`AssetKind::SlicerProfile`]。
//!
//! MKP 产物**不在这张表里**：它是 catalog 文件条目（`kind = mkp_preset`），
//! 由 [`get_version_files`] 提供（预设页的版本节点读的就是它）。
//!
//! # 这几条读为什么是 `async` 的
//!
//! Tauri 把**同步**命令放在主线程上跑、一条一条排队；`async` 的才丢进异步运行时并发跑。
//! 预设页一打开会同时发二十多条读，串行的话总耗时就是各条之和 —— 用户看到的正是
//! "打开先转圈、转完才出来"。这些命令只读缓存里的 catalog，彼此没有共享可变状态。

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::presetdata::registry::{Scope, VariantMode};
use crate::presetdata::resolve::{effective_of, visible_keys_of, Overrides};
use crate::presetdata::{Asset, AssetKind, Dimensions, ShowOp, UiComponent, ValueType};
use crate::runtime;

use super::traced;

/* ---------- 数据源与缓存 ---------- */

/// catalog 的缓存 —— **同一份字节只 parse 一次**。
///
/// # 为什么非有不可
///
/// 预设页一打开会发二十多条读，而每条都要把 catalog（含 74 条字段定义的 definition）
/// 从 JSON 解析一遍、重建索引。二十几条串起来就是好几秒，用户会以为在下载。
/// **这些命令一条网络都不碰**，慢的纯粹是重复解析。
///
/// # 缓存的边界与失效
///
/// 缓存按**字节内容**配对：读盘拿到字节，和缓存里的相等就直接给缓存的解析结果。
/// 目录换新（升级、应用远端更新）后字节变，下一读自动 miss、重新 parse ——
/// **没有失效钩子要调**，构造上就不会出现"换了目录还在用旧定义"。
///
/// 用 Vec 而不是单值是为了测试里能并发拿两个临时根；同一进程里目录只有一个，
/// 这张表事实上最多一条。
static CATALOG_CACHE: Mutex<Vec<(Vec<u8>, Arc<runtime::Catalog>)>> = Mutex::new(Vec::new());

/// 读客户端那一份预设数据：释放进内部根的 catalog。
fn load_presets(app: &AppHandle) -> Result<Arc<runtime::Catalog>, AppError> {
    let root = internal_root(app)?;
    let bytes = runtime::load_released_catalog_bytes(&root)?;
    let mut cache = CATALOG_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, hit)) = cache.iter().find(|(b, _)| *b == bytes) {
        return Ok(Arc::clone(hit));
    }
    let catalog = Arc::new(runtime::Catalog::parse(&bytes)?);
    cache.push((bytes, Arc::clone(&catalog)));
    Ok(catalog)
}

/* ---------- 小工具 ---------- */

/// 写进契约的字符串化口径，与前端 `mockServer/params.ts::toText` 同一条：
/// 空 → `''`，布尔 → `'on'` / `'off'`，数字去掉多余的 `.0`（对齐 JS 的 `String()`）。
fn to_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "on" } else { "off" }.to_owned(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                let f = n.as_f64().unwrap_or_default();
                if f.is_finite() && f.fract() == 0.0 {
                    format!("{}", f as i64)
                } else {
                    format!("{f}")
                }
            }
        }
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// trim 后非空才算有值
fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// 路径的最后一段 —— 界面上显示的文件名
fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

/// 切片器条目的类型。契约只有三种 `FileKind`，切片器这一档只有 bbs / orca 两个来源
fn file_kind(a: &Asset) -> &'static str {
    match a.slicer.as_deref() {
        Some("orca") => "orca_profile",
        _ => "bbs_profile",
    }
}

/// 切片器条目的喷嘴与层高。
///
/// 与 `workbench/app/assets.rs::slicer_axes` 同一条判据（C14 `derive.slicerMetaOf` 的
/// 后端版）：喷嘴是路径里那段 `0.2mm`（去掉 `mm` 后还得是数字，防止把 `mm` 目录误伤），
/// 层高是文件名尾部那串数字（`MKPProcess A1 0.2 0.10.json` → `0.10`）。
fn slicer_axes(path: &str) -> (Option<String>, Option<String>) {
    let numeric = |s: &str| {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            && s.bytes().any(|b| b.is_ascii_digit())
    };
    let nozzle = path
        .split(['/', '\\'])
        .find_map(|seg| seg.strip_suffix("mm"))
        .filter(|s| numeric(s))
        .map(str::to_owned);
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = file
        .rfind('.')
        .filter(|&i| file[i + 1..].eq_ignore_ascii_case("json"))
        .map_or(file, |i| &file[..i]);
    let layer = stem
        .trim_end_matches('.')
        .rsplit_once(' ')
        .map(|(_, tail)| tail.trim())
        .filter(|tail| numeric(tail))
        .map(str::to_owned);
    (nozzle, layer)
}

/// 哪个控件。映射表只此一份，与 `mockServer/params.ts::CONTROL` 同一条
fn control_of(c: UiComponent) -> &'static str {
    match c {
        UiComponent::Number => "number",
        UiComponent::Switch => "switch",
        UiComponent::Segmented => "choice",
        UiComponent::Select => "choice",
        UiComponent::Gcode => "text",
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/* ---------- getMachines ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 空串 = 这个版本还没配套餐（与契约同一口径）
    pub bundle: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZonePointDto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForbiddenZoneDto {
    pub points: Vec<ZonePointDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDto {
    pub id: String,
    pub display: String,
    pub brand: String,
    pub image: String,
    pub icon: String,
    pub aliases: Vec<String>,
    pub versions: Vec<VersionDto>,
    /// `null` = 这台机型还没配尺寸。不给空对象也不给 0
    pub dimensions: Option<Dimensions>,
    pub forbidden_zones: Vec<ForbiddenZoneDto>,
}

/// 机型清单。前端预设页的三级树、首页的机型卡片都读它。
///
/// 纯函数（吃 catalog 吐 DTO）：命令层只负责取数据，判据测试直接喂真 catalog。
fn machines_dto(catalog: &runtime::Catalog) -> Vec<MachineDto> {
    catalog
        .machines
        .iter()
        .map(|m| MachineDto {
            id: m.id.clone(),
            display: runtime::Catalog::display_of(m).to_owned(),
            // collect 时已经从 brands 换算成显示名（`拓竹 (Bambu Lab)`），直接用
            brand: m.brand.clone(),
            image: m.image.clone().unwrap_or_default(),
            icon: m.icon.clone().unwrap_or_default(),
            aliases: m.external_aliases.clone(),
            versions: m
                .versions
                .iter()
                .map(|v| VersionDto {
                    id: v.id.clone(),
                    name: v.name.clone(),
                    tag: v.tag.as_deref().and_then(non_empty),
                    description: v.description.as_deref().and_then(non_empty),
                    bundle: non_empty(v.recommended_bundle.as_deref().unwrap_or_default())
                        .or_else(|| non_empty(m.default_bundle.as_deref().unwrap_or_default()))
                        .unwrap_or_default(),
                })
                .collect(),
            dimensions: m.dimensions.clone(),
            forbidden_zones: m
                .zones
                .iter()
                .map(|z| ForbiddenZoneDto {
                    points: z
                        .points
                        .iter()
                        .map(|(x, y)| ZonePointDto { x: *x, y: *y })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

#[tauri::command]
pub async fn get_machines(app: AppHandle) -> Result<Vec<MachineDto>, AppError> {
    traced("getMachines", |_| {
        let catalog = load_presets(&app)?;
        Ok(machines_dto(&catalog))
    })
}

/* ---------- getVersionFiles ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRefDto {
    pub kind: &'static str,
    pub file_name: String,
    pub path: String,
    /// MKP 那支带真值（catalog 文件条目对交付产物真字节算的）；切片器资产本轮不给
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionFilesDto {
    pub files: Vec<FileRefDto>,
    /// true = 该有的文件没配齐。界面要说「未配置」，不是「0 个文件」
    pub incomplete: bool,
    pub missing: Vec<String>,
}

/// 「这个机型这个版本要哪些文件」。机型或版本不存在 → `None`（不是出错）
///
/// MKP 那一支**从 catalog 的文件条目出**（路径、大小、SHA 都是登记值）——
/// 不再按命名规则重算：首屏唯一数据源 = catalog（总纲判据 4）。
/// 切片器那一支与旧世界同构：版本 → 套餐 → 资产。
fn version_files_dto(
    catalog: &runtime::Catalog,
    machine_id: &str,
    version_id: &str,
) -> Option<VersionFilesDto> {
    let machine = catalog.machine(machine_id)?;
    if !machine.versions.iter().any(|v| v.id == version_id) {
        return None;
    }

    let mut files = Vec::new();
    let mut missing = Vec::new();

    // —— MKP 预设一支：catalog 登记的交付文件。条目缺席 = 交付集合里就没有这份
    match catalog.file_of(machine_id, version_id) {
        Some(f) => files.push(FileRefDto {
            kind: "mkp_preset",
            file_name: f.file_name.clone(),
            path: f.path.clone(),
            size: Some(f.size),
            sha256: Some(f.sha256.clone()),
        }),
        None => missing.push(format!(
            "{machine_id} / {version_id} 在目录里没有登记交付文件"
        )),
    }

    // —— 切片器一支 ——
    let bundle_id = non_empty(
        machine
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .and_then(|v| v.recommended_bundle.as_deref())
            .unwrap_or_default(),
    )
    .or_else(|| non_empty(machine.default_bundle.as_deref().unwrap_or_default()));
    match bundle_id {
        None => missing.push(format!(
            "{machine_id} / {version_id} 没有配 bundle，取不到切片器配置"
        )),
        Some(id) => match catalog.bundle(&id) {
            None => missing.push(format!(
                "{machine_id} / {version_id} 指向的 bundle 不存在：{id}"
            )),
            Some(bundle) => {
                for r in &bundle.asset_refs {
                    match catalog.asset(r) {
                        None => {
                            missing.push(format!("bundle {} 引用了不存在的 asset：{r}", bundle.id))
                        }
                        Some(a) => files.push(FileRefDto {
                            kind: file_kind(a),
                            file_name: file_name_of(&a.path),
                            path: format!("presets/{}", a.path),
                            size: None,
                            sha256: None,
                        }),
                    }
                }
            }
        },
    }

    Some(VersionFilesDto {
        incomplete: !missing.is_empty(),
        missing,
        files,
    })
}

#[tauri::command]
pub async fn get_version_files(
    app: AppHandle,
    machine_id: String,
    version_id: String,
) -> Result<Option<VersionFilesDto>, AppError> {
    traced("getVersionFiles", |_| {
        let catalog = load_presets(&app)?;
        Ok(version_files_dto(&catalog, &machine_id, &version_id))
    })
}

/* ---------- getPresetFiles / getMenu ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRefDto {
    pub machine: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetFileInfoDto {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub kind: &'static str,
    pub category: String,
    pub machine_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nozzle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_height: Option<String>,
    pub in_bundles: Vec<String>,
    pub used_by_versions: Vec<VersionRefDto>,
    pub delivery: &'static str,
    // sizeText / modifiedText / statFrom 一律不给（载荷本体不在这台机器上，编不出来）
}

/// 装了这条资产的套餐 id（大小写不敏感）
fn bundles_of(catalog: &runtime::Catalog, asset_id: &str) -> Vec<String> {
    let want = asset_id.trim().to_lowercase();
    catalog
        .bundles
        .iter()
        .filter(|b| b.asset_refs.iter().any(|r| r.trim().to_lowercase() == want))
        .map(|b| b.id.clone())
        .collect()
}

/// 云端表：catalog 里那些**可下载的预设文件**（只出切片器预设，见文件头）
fn preset_files_dto(catalog: &runtime::Catalog) -> Vec<PresetFileInfoDto> {
    catalog
        .assets
        .iter()
        .filter(|a| a.kind == AssetKind::SlicerProfile)
        .map(|a| {
            let in_bundles = bundles_of(catalog, &a.id);
            let (nozzle, layer_height) = slicer_axes(&a.path);
            PresetFileInfoDto {
                id: a.id.clone(),
                file_name: file_name_of(&a.path),
                path: format!("presets/{}", a.path),
                kind: file_kind(a),
                category: a.profile.clone().unwrap_or_default(),
                machine_ids: a.machine_id.clone().into_iter().collect(),
                nozzle,
                layer_height,
                delivery: if in_bundles.is_empty() {
                    "optional"
                } else {
                    "default"
                },
                in_bundles,
                // 没有 MKP 资产条目（doc §12.5），所以这一栏恒空
                used_by_versions: Vec::new(),
            }
        })
        .collect()
}

#[tauri::command]
pub async fn get_preset_files(app: AppHandle) -> Result<Vec<PresetFileInfoDto>, AppError> {
    traced("getPresetFiles", |_| {
        let catalog = load_presets(&app)?;
        Ok(preset_files_dto(&catalog))
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuEntryDto {
    pub file_id: String,
    pub visibility: &'static str,
}

/// 菜单：哪些官方文件是分配过的（`bundled`）、哪些是可选的（`optional`）。
/// **没有 `archived`** —— 归档是工作台那边的事，客户端拿到的都是可见的
fn menu_dto(catalog: &runtime::Catalog) -> Vec<MenuEntryDto> {
    catalog
        .assets
        .iter()
        .map(|a| MenuEntryDto {
            file_id: a.id.clone(),
            visibility: if bundles_of(catalog, &a.id).is_empty() {
                "optional"
            } else {
                "bundled"
            },
        })
        .collect()
}

#[tauri::command]
pub async fn get_menu(app: AppHandle) -> Result<Vec<MenuEntryDto>, AppError> {
    traced("getMenu", |_| {
        let catalog = load_presets(&app)?;
        Ok(menu_dto(&catalog))
    })
}

/* ---------- getParamMeta ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowWhenDto {
    pub key: String,
    pub op: ShowOp,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamMetaDto {
    pub key: String,
    pub toml_key: String,
    pub json_key: String,
    pub config_key: String,
    pub section: String,
    pub section_id: String,
    pub order: f64,
    pub scope: Scope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_mode: Option<VariantMode>,
    pub value_type: ValueType,
    pub ui_component: UiComponent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toml_comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_group: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub pinned: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_when: Option<ShowWhenDto>,
}

/// 全部参数的元信息。**不过滤** —— 界面要能看到废弃的参数为什么不显示
fn param_meta_dto(catalog: &runtime::Catalog) -> Vec<ParamMetaDto> {
    catalog
        .params()
        .iter()
        .map(|p| ParamMetaDto {
            key: p.key.clone(),
            toml_key: p.toml_key.clone(),
            json_key: p.json_key.clone(),
            config_key: p.config_key.clone(),
            section: p.section.clone(),
            section_id: p.layout.section_id.clone(),
            order: p.layout.order,
            scope: p.scope,
            variant_mode: p.variant_mode,
            value_type: p.value_type,
            ui_component: p.ui_component,
            unit: p.unit.as_deref().and_then(non_empty),
            toml_comment: non_empty(&p.toml_comment),
            merge_group: p.merge_group.as_deref().and_then(non_empty),
            pinned: p.pinned,
            deprecated: p.deprecated,
            machine_filter: (!p.machine_filter.is_empty()).then(|| p.machine_filter.clone()),
            show_when: p.show_when.as_ref().map(|s| ShowWhenDto {
                key: s.key.clone(),
                op: s.op,
                value: to_text(&s.value),
            }),
        })
        .collect()
}

#[tauri::command]
pub async fn get_param_meta(app: AppHandle) -> Result<Vec<ParamMetaDto>, AppError> {
    traced("getParamMeta", |_| {
        let catalog = load_presets(&app)?;
        Ok(param_meta_dto(&catalog))
    })
}

/* ---------- getMachineParams ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceDto {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeParamDto {
    pub key: String,
    pub label: String,
    pub desc: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub control: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ChoiceDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// **一律字符串** —— 单位换算与小数位数不该由界面再做一遍
    pub value: String,
    pub origin: &'static str,
    /// 只有 `origin === "variant"` 时有：基础配方里的那个值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_value: Option<String>,
}

/// 一张配方（某个机型 + 某个版本）的全部参数值。
///
/// `version_id` 为 `None` = 只看机型基底，那时每条都是 `origin: "base"`。
///
/// 三层取值走 [`visible_keys_of`] + [`effective_of`] —— 与工作台（[`Layers`]）同一份
/// 算法，只是数据源从 ParamRegistry 换成了 catalog 的 definition 切片。
/// **不做工作台那套归并/上提**（那是编辑侧为了少写几个键才需要的）。
/// 有效值与工作台一致；只有被上提过的键，来源标签两者不同（工作台说机型、这里说版本），
/// 而前端本来就是按"版本键在不在"判 `origin` 的，所以这里的口径与它一致。
#[tauri::command]
pub async fn get_machine_params(
    app: AppHandle,
    machine_id: String,
    version_id: Option<String>,
) -> Result<Vec<RecipeParamDto>, AppError> {
    traced("getMachineParams", |_| {
        let catalog = load_presets(&app)?;
        machine_params_dto(&catalog, &machine_id, version_id.as_deref())
    })
}

fn machine_params_dto(
    catalog: &runtime::Catalog,
    machine_id: &str,
    version_id: Option<&str>,
) -> Result<Vec<RecipeParamDto>, AppError> {
    let machine = catalog
        .machine(machine_id)
        .ok_or_else(|| AppError::not_found(format!("没有机型 {machine_id}")))?;
    if let Some(v) = version_id {
        if !machine.versions.iter().any(|x| x.id == v) {
            return Err(AppError::not_found(format!("{machine_id} 没有版本 {v}")));
        }
    }

    // 两张稀疏表：裸键是机型层，`机型:版本` 是版本层（与 `resolve` 的模型同源）
    let mut base = Overrides::new();
    let mut over = Overrides::new();
    for p in catalog.params() {
        if let Some(v) = p.machine_variants.get(machine_id) {
            base.insert(p.key.clone(), v.clone());
        }
        if let Some(v) =
            version_id.and_then(|vid| p.machine_variants.get(&format!("{machine_id}:{vid}")))
        {
            over.insert(p.key.clone(), v.clone());
        }
    }

    let mut out = Vec::new();
    for key in visible_keys_of(catalog.params(), machine_id) {
        let Some(p) = catalog.param(key) else {
            continue;
        };
        let Some(eff) = effective_of(p, machine_id, &base, &over) else {
            continue;
        };
        let is_variant = eff.origin == crate::presetdata::resolve::Origin::Version;
        out.push(RecipeParamDto {
            key: p.key.clone(),
            label: p.label.clone(),
            desc: p.desc.clone(),
            group: catalog
                .section_meta(&p.layout.section_id)
                .map(|s| s.label.clone())
                .unwrap_or_else(|| p.layout.section_id.clone()),
            unit: p.unit.as_deref().and_then(non_empty),
            control: control_of(p.ui_component),
            choices: (!p.choices.is_empty()).then(|| {
                p.choices
                    .iter()
                    .map(|c| ChoiceDto {
                        value: to_text(&c.value),
                        label: c.label.clone(),
                    })
                    .collect()
            }),
            min: p.min,
            max: p.max,
            step: p.step,
            value: to_text(eff.value),
            origin: if is_variant { "variant" } else { "base" },
            // 被版本盖过才带「还原成」的那个值：先看机型层有没有钉着，没有才是出厂默认
            base_value: is_variant.then(|| to_text(base.get(key).unwrap_or(&p.default_value))),
        });
    }
    Ok(out)
}

/* ---------- 本机状态（本轮恒空） ---------- */

/// 本机已经有的官方文件（asset id）。
///
/// **恒为 `[]`**：切片器资产的载荷不落内部根（内置的随安装包走、前端按 URL 取；
/// 下载的 MKP 预设走 `get_downloaded_files` 那条新世界管道）。空集是"还没下载"，
/// 与"读不出来"是两件事。资产本体登记进交付管道（总纲欠账 #3）之后这一条跟着长。
#[tauri::command]
pub async fn get_local_files(_app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getLocalFiles", |_| Ok(Vec::new()))
}

/// 用户自己放进预设目录的文件。**扫盘那条路要等云端联动那一轮**（见方案 §6）
#[tauri::command]
pub fn get_local_user_files(_app: AppHandle) -> Result<Vec<Value>, AppError> {
    traced("getLocalUserFiles", |_| Ok(Vec::new()))
}

/// 已经复制进切片器目录的那些。**本轮恒空** —— 写盘那一侧还没有
#[tauri::command]
pub fn get_slicer_copied(_app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getSlicerCopied", |_| Ok(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真仓库的源 + 入库产物 → 真目录。DTO 构建判据的共用输入
    fn catalog() -> runtime::Catalog {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        runtime::Catalog::build_from_repo(&repo).expect("真目录构建不出来")
    }

    /// `to_text` 是契约那一栏的唯一口径 —— 布尔要成 `on` / `off`（switch 控件的两个选项），
    /// 整数不能带 `.0`（对齐 JS 的 `String(1)`）
    #[test]
    fn to_text_matches_the_contract() {
        assert_eq!(to_text(&Value::Null), "");
        assert_eq!(to_text(&Value::Bool(true)), "on");
        assert_eq!(to_text(&Value::Bool(false)), "off");
        assert_eq!(to_text(&serde_json::json!(1)), "1");
        assert_eq!(to_text(&serde_json::json!(1.0)), "1");
        assert_eq!(to_text(&serde_json::json!(-0.7)), "-0.7");
        assert_eq!(to_text(&serde_json::json!("标准版")), "标准版");
        assert_eq!(to_text(&serde_json::json!("")), "");
    }

    /// 喷嘴与层高那两格：句点必须是数字，「数字」段不能把 `mm` 目录本身算进去
    #[test]
    fn slicer_axes_reads_nozzle_and_layer() {
        let (n, l) = slicer_axes("bbs/Process/0.4mm/MKPProcess A1 0.4 0.20.json");
        assert_eq!(n.as_deref(), Some("0.4"));
        assert_eq!(l.as_deref(), Some("0.20"));

        // 没有 mm 段、也没有尾部数字：两格都给不出，但**不能编**
        let (n, l) = slicer_axes("bbs/orca/nope.json");
        assert_eq!(n, None);
        assert_eq!(l, None);
    }

    /// **DTO 构建吃的是 catalog（换源判据）。**
    ///
    /// 九条命令的映射层全部从真 catalog 出数：机型、版本文件（MKP 带真 size/SHA）、
    /// 参数元信息、配方值。任何一条从别的来源出数（TOML、硬编码），这条就失守 ——
    /// 那正是「首屏唯一数据源 = catalog」（总纲判据 4）在 Rust 侧的钉子
    #[test]
    fn dto_builders_read_the_catalog_and_nothing_else() {
        let catalog = catalog();

        // —— 机型清单：5 台，A1 带别名与尺寸，品牌是显示名 ——
        let machines = machines_dto(&catalog);
        assert_eq!(machines.len(), 5);
        let a1 = machines.iter().find(|m| m.id == "A1").expect("A1 必须在");
        assert_eq!(a1.display, "A1");
        assert_eq!(a1.brand, "拓竹 (Bambu Lab)", "品牌要显示名，不是 id");
        assert_eq!(a1.aliases, vec!["A1C", "A1F"]);
        assert!(a1.dimensions.is_some());
        assert!(a1.forbidden_zones.is_empty(), "A1 没有禁区");
        assert_eq!(a1.versions.len(), 3);

        // —— 版本文件：MKP 那支从 catalog 文件条目出，带真 size 与 SHA ——
        let vf = version_files_dto(&catalog, "A1", "FASTV3.3").expect("A1/FASTV3.3 该有答案");
        assert!(!vf.incomplete, "A1/FASTV3.3 配齐了：{:?}", vf.missing);
        let mkp = vf
            .files
            .iter()
            .find(|f| f.kind == "mkp_preset")
            .expect("MKP 引用必须在");
        assert_eq!(mkp.file_name, "A1-fastv3.3.toml");
        assert_eq!(mkp.path, "mkp/presets/A1-fastv3.3.toml", "落点相对内部根");
        assert!(mkp.size.unwrap_or(0) > 0, "大小是登记的真值");
        assert_eq!(mkp.sha256.as_deref().map(str::len), Some(64));
        // 切片器那支跟着套餐走：A1_default 至少一条 BBS
        assert!(
            vf.files.iter().any(|f| f.kind == "bbs_profile"),
            "切片器配置要跟着套餐出来"
        );

        // 不存在的机型/版本 → None（不是出错）
        assert!(version_files_dto(&catalog, "NOPE", "X").is_none());
        assert!(version_files_dto(&catalog, "A1", "NOPE").is_none());

        // —— 云端表：只出切片器预设，19 条资产里机型图/图标不进来 ——
        let presets = preset_files_dto(&catalog);
        assert!(!presets.is_empty());
        assert!(presets.iter().all(|p| p.kind != "mkp_preset"));
        assert!(
            presets
                .iter()
                .all(|p| p.kind == "bbs_profile" || p.kind == "orca_profile"),
            "切片器这一档只有 bbs / orca"
        );
        let a1_bbs = presets
            .iter()
            .find(|p| p.id == "a1-bbs-04-020")
            .expect("A1 的 BBS 条目在");
        assert_eq!(a1_bbs.delivery, "default", "被套餐装着的是 default");
        assert!(a1_bbs.in_bundles.contains(&"A1_default".to_owned()));
        assert_eq!(a1_bbs.nozzle.as_deref(), Some("0.4"), "喷嘴从路径段读出");

        // —— 菜单：可见性跟着套餐走 ——
        let menu = menu_dto(&catalog);
        let vis = |id: &str| {
            menu.iter()
                .find(|m| m.file_id == id)
                .unwrap_or_else(|| panic!("{id} 不在菜单里"))
                .visibility
        };
        assert_eq!(vis("a1-bbs-04-020"), "bundled");
        assert_eq!(vis("a1-image"), "optional", "机型图不被套餐引用");

        // —— 参数元信息：全部 74 条，含废弃 ——
        let meta = param_meta_dto(&catalog);
        assert_eq!(meta.len(), 74, "实测 74 条字段定义，一条不少");
        let offset_x = meta
            .iter()
            .find(|p| p.key == "toolhead.offset.x")
            .expect("基础字段在");
        assert_eq!(offset_x.toml_key, "offset");
        assert_eq!(offset_x.value_type, ValueType::Float);
        assert_eq!(offset_x.ui_component, UiComponent::Number);

        // —— 配方值：三层取值的结果与来源层 ——
        let params = machine_params_dto(&catalog, "A1", Some("FASTV3.3")).expect("A1 的配方");
        assert!(!params.is_empty());
        let x = params
            .iter()
            .find(|p| p.key == "toolhead.offset.x")
            .expect("偏移 X 在配方里");
        assert_eq!(x.group, "空间偏移", "分组名来自 [[tabs]] 的元数据");
        // 真数据里 A1:FASTV3.3 把这一项钉在 0.1（版本层）——三层取值要读出覆盖
        assert_eq!(x.value, "0.1");
        assert_eq!(x.origin, "variant");
        // 机型层没写 → 「还原成」给的是出厂默认 0
        assert_eq!(x.base_value.as_deref(), Some("0"));

        // 机型不存在的报错要说出是谁
        let err = machine_params_dto(&catalog, "NOPE", None).unwrap_err();
        assert!(err.message.contains("NOPE"));
    }

    /// 缓存按字节配对：同一份字节复用解析结果，字节变了自动 miss。
    /// 目录换新（升级/应用远端更新）后没有失效钩子可忘 —— 构造上就不存在那个坑
    #[test]
    fn the_byte_cache_serves_and_invalidates_itself() {
        let shared = Arc::new(runtime::Catalog::default());
        let v1 = b"v1".to_vec();
        CATALOG_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((v1.clone(), Arc::clone(&shared)));

        let hit = CATALOG_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|(b, _)| *b == v1)
            .map(|(_, c)| Arc::clone(c));
        assert!(Arc::ptr_eq(&hit.unwrap(), &shared), "同字节命中缓存");

        CATALOG_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        let guard = CATALOG_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let miss = guard.iter().find(|(b, _)| *b == v1);
        assert!(miss.is_none(), "字节变了（或被清）就该重新 parse");
    }
}
