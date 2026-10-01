//! 正式客户端的预设读命令 —— A41 预设页的真后端。
//!
//! # 数据根是客户端自己的那一份
//!
//! 每条命令都从 [`paths::presets_root`]（`<appDataDir>/presets`）读，**不是仓库**。
//! 解析代码与工作台共用同一份（[`crate::presetdata`]），但**根是两套** ——
//! 装了客户端的用户机器上根本没有仓库。见 `client::paths` 的文件头。
//!
//! # 这一轮读得到什么
//!
//! | 命令 | 数据源 |
//! |---|---|
//! | `get_machines` | 机型文件 + `brands.toml` + 禁区文件 |
//! | `get_version_files` | 机型版本 + 套餐 → 资产（MKP 那一支按命名规则算出） |
//! | `get_preset_files` | 资产定义（**只出切片器预设**，见下） |
//! | `get_menu` | 资产定义 + 套餐反查 |
//! | `get_param_meta` | 字段定义（**全部**，含废弃） |
//! | `get_machine_params` | `machineVariants` 的三层取值 |
//! | `get_local_files` | 资产定义 + 载荷是否真的在本机 |
//! | `get_local_user_files` | 本轮恒为空集合 |
//! | `get_slicer_copied` | 本轮恒为空集合 |
//!
//! # 只出切片器预设，不出机型图
//!
//! `presets/assets.toml` 里同时登记机型图 / 图标 / 模型（19 条里 10 条是这些），
//! 而 A41 那张「云端表」要的是**可下载的预设文件**。把一张 `.webp` 贴成 `mkp_preset`
//! 塞进 MKP 表是错的 —— 所以 [`get_preset_files`] 只出 [`AssetKind::SlicerProfile`]。
//!
//! MKP 产物**不在这张表里**：它按命名规则算路径（`presets/mkp/<机型>-<版本>.toml`），
//! 由 [`get_version_files`] 提供（预设页的版本节点读的就是它）。
//!
//! # 大小与时间这一轮不给
//!
//! `PresetFileInfo.sizeText` / `modifiedText` / `statFrom` 一律省略 —— 本机没有这些
//! 文件本体，编不出来也不该编。界面拿不到就写「未知」，那是对的（契约本来就是可选的）。

use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;

use crate::client::paths;
use crate::error::AppError;
use crate::presetdata::registry::{Scope, VariantMode};
use crate::presetdata::{
    mkp_file_name, Asset, AssetKind, Dimensions, Layers, Origin, Overrides, Presets, ShowOp,
    UiComponent, ValueType,
};

use super::traced;

/* ---------- 数据根 ---------- */

/// 读客户端那一份预设数据。**载荷根 = 预设根本身**。
///
/// 定义（`assets.toml` 的 `path`）与载荷（文件本体）在仓库里是两个地方
/// （`presets/` ↔ `public/assets/`）；客户端这一轮**只释放定义、不释放文件本体**，
/// 所以载荷根指到预设根上，[`crate::presetdata::Assets::present`] 一律为 false ——
/// 界面上表现为"文件还没到"，而不是一个查不出来的状态。
fn load_presets(app: &AppHandle) -> Result<Presets, AppError> {
    let root = paths::presets_root(app)?;
    let mut p = Presets::load_from(&root)?;
    p.set_asset_root(&root);
    Ok(p)
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

/// 机型清单。前端预设页的三级树、首页的机型卡片都读它
#[tauri::command]
pub fn get_machines(app: AppHandle) -> Result<Vec<MachineDto>, AppError> {
    traced("getMachines", |_| {
        let preset = load_presets(&app)?;
        // 品牌显示名：机型文件里写的是 id（`Bambu Lab`），给人看的是 `拓竹 (Bambu Lab)`
        let brands: std::collections::HashMap<&str, &str> = preset
            .catalog
            .brands()
            .iter()
            .map(|b| (b.id.as_str(), b.name.as_str()))
            .collect();

        Ok(preset
            .catalog
            .machines()
            .iter()
            .map(|m| MachineDto {
                id: m.id.clone(),
                display: if m.display.trim().is_empty() {
                    m.id.clone()
                } else {
                    m.display.clone()
                },
                brand: brands
                    .get(m.brand.as_str())
                    .map(|s| (*s).to_owned())
                    .unwrap_or_else(|| m.brand.clone()),
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
                forbidden_zones: preset
                    .catalog
                    .zones(&m.id)
                    .unwrap_or(&[])
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
            .collect())
    })
}

/* ---------- getVersionFiles ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRefDto {
    pub kind: &'static str,
    pub file_name: String,
    pub path: String,
    /// 字节数。**本轮恒不给** —— 真值要到发布打 manifest 时才算得出来
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
#[tauri::command]
pub fn get_version_files(
    app: AppHandle,
    machine_id: String,
    version_id: String,
) -> Result<Option<VersionFilesDto>, AppError> {
    traced("getVersionFiles", |_| {
        let preset = load_presets(&app)?;
        let Some(machine) = preset.catalog.machine(&machine_id) else {
            return Ok(None);
        };
        let Some(version) = machine.versions.iter().find(|v| v.id == version_id) else {
            return Ok(None);
        };

        let mut files = Vec::new();
        let mut missing = Vec::new();

        // —— MKP 预设一支 ——
        // 路径由命名规则算出（数据里没有 `presetFile` 这个字段了），所以**恒有**。
        let mkp_name = mkp_file_name(&machine_id, &version_id);
        files.push(FileRefDto {
            kind: "mkp_preset",
            file_name: mkp_name.clone(),
            path: format!("presets/mkp/{mkp_name}"),
            size: None,
            sha256: None,
        });

        // —— 切片器一支 ——
        let bundle_id = non_empty(version.recommended_bundle.as_deref().unwrap_or_default())
            .or_else(|| non_empty(machine.default_bundle.as_deref().unwrap_or_default()));
        match bundle_id {
            None => missing.push(format!(
                "{machine_id} / {version_id} 没有配 bundle，取不到切片器配置"
            )),
            Some(id) => match preset.bundles.get(&id) {
                None => missing.push(format!(
                    "{machine_id} / {version_id} 指向的 bundle 不存在：{id}"
                )),
                Some(bundle) => {
                    for r in &bundle.asset_refs {
                        match preset.assets.get(r) {
                            None => missing
                                .push(format!("bundle {} 引用了不存在的 asset：{r}", bundle.id)),
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

        Ok(Some(VersionFilesDto {
            incomplete: !missing.is_empty(),
            missing,
            files,
        }))
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
    // sizeText / modifiedText / statFrom 本轮一律不给（见文件头）
}

/// 装了这条资产的套餐 id（大小写不敏感）
fn bundles_of(preset: &Presets, asset_id: &str) -> Vec<String> {
    let want = asset_id.trim().to_lowercase();
    preset
        .bundles
        .items()
        .iter()
        .filter(|b| b.asset_refs.iter().any(|r| r.trim().to_lowercase() == want))
        .map(|b| b.id.clone())
        .collect()
}

/// 云端表：仓库里那些**可下载的预设文件**（只出切片器预设，见文件头）
#[tauri::command]
pub fn get_preset_files(app: AppHandle) -> Result<Vec<PresetFileInfoDto>, AppError> {
    traced("getPresetFiles", |_| {
        let preset = load_presets(&app)?;
        Ok(preset
            .assets
            .items()
            .iter()
            .filter(|a| a.kind == AssetKind::SlicerProfile)
            .map(|a| {
                let in_bundles = bundles_of(&preset, &a.id);
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
                    // 没有 MKP 资产条目（doc §12.5），所以这一栏本轮恒空
                    used_by_versions: Vec::new(),
                }
            })
            .collect())
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuEntryDto {
    pub file_id: String,
    pub visibility: &'static str,
}

/// 菜单：哪些官方文件是分配过的（`bundled`）、哪些是可选的（`optional`）。
/// **本轮没有 `archived`** —— 归档是工作台那边的事，客户端拿到的都是可见的
#[tauri::command]
pub fn get_menu(app: AppHandle) -> Result<Vec<MenuEntryDto>, AppError> {
    traced("getMenu", |_| {
        let preset = load_presets(&app)?;
        Ok(preset
            .assets
            .items()
            .iter()
            .map(|a| MenuEntryDto {
                file_id: a.id.clone(),
                visibility: if bundles_of(&preset, &a.id).is_empty() {
                    "optional"
                } else {
                    "bundled"
                },
            })
            .collect())
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
#[tauri::command]
pub fn get_param_meta(app: AppHandle) -> Result<Vec<ParamMetaDto>, AppError> {
    traced("getParamMeta", |_| {
        let preset = load_presets(&app)?;
        Ok(preset
            .registry
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
            .collect())
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
/// 三层取值用 [`Layers`] 的**原样三层**（版本键 → 机型键 → 出厂），
/// **不做工作台那套归并/上提**（那是编辑侧为了少写几个键才需要的）。
/// 有效值与工作台一致；只有被上提过的键，来源标签两者不同（工作台说机型、这里说版本），
/// 而前端本来就是按"版本键在不在"判 `origin` 的，所以这里的口径与它一致。
#[tauri::command]
pub fn get_machine_params(
    app: AppHandle,
    machine_id: String,
    version_id: Option<String>,
) -> Result<Vec<RecipeParamDto>, AppError> {
    traced("getMachineParams", |_| {
        let preset = load_presets(&app)?;
        let machine = preset
            .catalog
            .machine(&machine_id)
            .ok_or_else(|| AppError::not_found(format!("没有机型 {machine_id}")))?;
        if let Some(v) = &version_id {
            if !machine.versions.iter().any(|x| x.id == *v) {
                return Err(AppError::not_found(format!("{machine_id} 没有版本 {v}")));
            }
        }

        // 两张稀疏表：裸键是机型层，`机型:版本` 是版本层（与 `resolve` 的模型同源）
        let mut base = Overrides::new();
        let mut over = Overrides::new();
        for p in preset.registry.params() {
            if let Some(v) = p.machine_variants.get(&machine_id) {
                base.insert(p.key.clone(), v.clone());
            }
            if let Some(v) = version_id
                .as_ref()
                .and_then(|vid| p.machine_variants.get(&format!("{machine_id}:{vid}")))
            {
                over.insert(p.key.clone(), v.clone());
            }
        }
        let layers = Layers::new(&preset.registry, &machine_id, &base, &over);

        let mut out = Vec::new();
        for key in layers.keys() {
            let Some(p) = preset.registry.param(key) else {
                continue;
            };
            let Some(eff) = layers.effective(key) else {
                continue;
            };
            let is_variant = eff.origin == Origin::Version;
            out.push(RecipeParamDto {
                key: p.key.clone(),
                label: p.label.clone(),
                desc: p.desc.clone(),
                group: preset
                    .registry
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
    })
}

/* ---------- 本机状态（本轮恒空） ---------- */

/// 本机已经有的官方文件（asset id）。
///
/// 判据是**载荷真的在本机**（[`crate::presetdata::Assets::present`]）。
/// 这一轮只释放了定义、没释放文件本体，所以恒为 `[]` —— 那是"还没下载"，
/// 与"读不出来"是两件事。
#[tauri::command]
pub fn get_local_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getLocalFiles", |_| {
        let preset = load_presets(&app)?;
        Ok(preset
            .assets
            .items()
            .iter()
            .filter(|a| preset.assets.present(a))
            .map(|a| a.id.clone())
            .collect())
    })
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

    /// MKP 那一支的路径与文件名都是算出来的 —— 机型段原样、版本段小写
    #[test]
    fn mkp_ref_path_uses_the_naming_rule() {
        let name = mkp_file_name("A1", "FASTV3.3");
        assert_eq!(name, "A1-fastv3.3.toml");
        assert_eq!(
            format!("presets/mkp/{name}"),
            "presets/mkp/A1-fastv3.3.toml"
        );
    }
}
