//! **逐参数「云端更新」的两条命令**：读三方账 / 把采用·保持的决定落下去。
//!
//! # 它兑现的产品规则
//!
//! 用户世界里只有**一份自己的预设**（`presets-local/…`）；云端那一版默认值住在
//! 隐藏 baseline 里（[`crate::runtime::baseline`]，用户看不见它）。于是：
//!
//! ```text
//! 读（get_preset_param_sync）    我这份文件 / 云端旧值（baseline） / 云端当前值（目录那一版）
//! 写（apply_preset_param_decisions）
//!     采用(adopt)  把我那份文件里这一项写成云端新值 + 水位推到当前版
//!     保持(hold)   我那份文件这一项还是我原来的值   + 水位推到当前版
//! ```
//!
//! # 写 = **以云端当前版为底重写这一份**（2026-10-20）
//!
//! 「处理云端更新」这件事要带走的不止是那几项值：**云端新版改过的注释、
//! 换过的时间戳、新增的字段**都要跟着进来，否则"注释级变更"会永远挂着
//! 「有新版」（血统摘要比的是全文）。所以写那一趟的形状是：
//!
//! ```text
//!   骨架 = 云端当前版正文（注释 / 时间戳 / 新字段都在里面）
//!   值    采用(adopt) → 云端新值；其余 → **我这份文件里现在写着的值**（一个不许被顶掉）
//!   血统  换成云端当前版：based_on / based_on_release_time / based_on_sha256
//! ```
//!
//! 于是 **0 项也是一次真写**（"自动应用 0 项"）：值一个不动，注释 / 时间戳 /
//! 新字段 / 血统跟着新版走 —— 点「更新」之后「有新版」自然消失。
//!
//! 「水位」= [`crate::runtime::app_state`] 里的逐参数决定账。它是
//! 「这一项我处理到哪一版了」的唯一记录 —— 云端连着发几版也不需要在本地存历史
//! （比对的是「我处理到的那一版 → 当前版」，见 [`crate::runtime::param_sync`]）。
//!
//! # 边界（与 [`super::preset_params`] 同一条）
//!
//! - **只认 `presets-local/`**（[`crate::runtime::local`] 的前缀 + 防穿越两道闸）：
//!   baseline 与云端当前版**永远不被写**；
//! - **结构保真**：采用走 [`crate::presetdata::params::apply_param_edits`] ——
//!   注释 / 键序 / 多行字面量 / 行尾一个字节不动；任何一项失败**整批不落**；
//! - **不碰草稿、不碰使用中指针**：这一层只管"这一项写什么"与"水位推到哪一版"。
//!
//! # 云端当前版的字节：只在本机找，除非明确要求取回来
//!
//! `fetch_missing` 是调用方给的开关。界面在**打开某一份预设时**可以传 `true`
//! （"我愿意等一下，把云端新版取回来看个清楚"），列表页那种一屏几十行的读一律
//! `false` —— 铁律是**启动零网络**，读命令不许偷偷发请求。

use postprocess::diag::messages;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::{internal_root, user_root};
use crate::ipc::traced;
use crate::presetdata::params as param_alg;
use crate::presetdata::patch::FieldEdit;
use crate::runtime;
use crate::runtime::app_state::{ParamDecision, ParamDecisionKind};
use crate::runtime::catalog::{hex, Catalog, CatalogFile};
use crate::runtime::param_sync::{self, SyncInput};

/// 一个参数上的四方账（前端那一份形状；`None` = 这一格没有值，不是空串）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamSyncEntryDto {
    pub key: String,
    /// 我这份文件里现在写着的值
    pub local: Option<String>,
    /// 云端旧值（我上次处理到的那一版）
    pub baseline_old: Option<String>,
    /// 云端当前最新值
    pub cloud_new: Option<String>,
    /// 云端改过这一项、而我还没处理
    pub pending: bool,
    /// 云端改了它、**我也改过它**，两边的值还不一样 —— 更新时必须让用户选
    pub conflict: bool,
    /// 我已经对当前云端版处理过这一项：`adopt` / `hold`
    pub decided: Option<&'static str>,
}

/// 一份用户预设的「云端更新」总账。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetParamSyncDto {
    /// 相对用户根的路径（`presets-local/A1-fast.toml`）
    pub path: String,
    pub file_name: String,
    /// 这份预设自己的归属（文件头 `# machine:` / `# variant:` 归一化后的；认不出是 `null`）
    pub machine_id: Option<String>,
    pub version_id: Option<String>,
    /// 云端那一份的文件名（`A1-fast.toml`）。认不出 ⇒ `null`（那就比不出云端更新）
    pub cloud_file_name: Option<String>,
    /// 云端当前版的正文摘要（**界面不显示**，它只是"是哪一版"的身份）
    pub current_sha256: Option<String>,
    /// 我这份当初基于的那一版云端（血统里的 `based_on_release_time`，给人看的）
    pub based_on_release_time: Option<String>,
    /// 我这份当初基于的那一版云端的 `# release_time`
    pub current_release_time: Option<String>,
    /// 云端当前版的字节**在本机可用**（读得到就能逐项比）
    pub cloud_ready: bool,
    /// 云端换版了：当前版与我这份血统里那一版不是同一份
    pub version_advanced: bool,
    /// 待处理条数（= `entries` 里 `pending` 的个数）
    pub pending_count: usize,
    pub entries: Vec<ParamSyncEntryDto>,
}

/// 一条要落下去的决定。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDecisionDto {
    pub param_key: String,
    /// `adopt` = 采用云端新值；`hold` = 保持本地值
    pub kind: String,
}

/// 某一份用户预设现在是什么情况：逐项三方账 + 待处理条数。
///
/// ★ `null`（2026-10-20）：**这一份不认云端** —— 它是复制出来的那一份，而开关
/// 「复制出来的也跟着云端更新」关着（出厂档）。界面据此**整段「云端更新」都不画**
/// （不是"零项待处理"：那会让用户以为云端这一版没动过任何一项）。
/// 判据与身份账本那一道闸同源（出处账 + 同一格开关），两处不许各说一套。
#[tauri::command]
pub async fn get_preset_param_sync(
    app: AppHandle,
    path: String,
    fetch_missing: Option<bool>,
) -> Result<Option<PresetParamSyncDto>, AppError> {
    let handle = app.clone();
    let fetch = fetch_missing.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        traced("getPresetParamSync", |_| {
            let root = internal_root(&handle)?;
            let user = user_root(&handle)?;
            /* 复制件不认云端 ⇒ 整本账都不给它（连目录都不必读） */
            if !runtime::app_state::copies_follow_cloud(&root)?
                && runtime::provenance::copy_targets(&user).contains(&path)
            {
                return Ok(None);
            }
            sync_at(&root, &user, &path, fetch).map(Some)
        })
    })
    .await
    .map_err(|e| {
        AppError::internal(messages::text("ui.ipc.param_sync.106", &[])).with_detail(e.to_string())
    })?
}

/// 把一批决定落下去（采用 / 保持），回来的是**落完之后的**那一份总账。
///
/// 采用要动文件，所以云端当前版必须在本机 —— 不在就如实报错（让界面先去取回来），
/// 不许"记一个水位、值却没写"（那会让这一项永远从待处理里消失，用户以为处理过了）。
#[tauri::command]
pub async fn apply_preset_param_decisions(
    app: AppHandle,
    path: String,
    decisions: Vec<ParamDecisionDto>,
) -> Result<PresetParamSyncDto, AppError> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        traced("applyPresetParamDecisions", |_| {
            let root = internal_root(&handle)?;
            let user = user_root(&handle)?;
            let out = apply_at(&root, &user, &path, &decisions)?;
            /* 状态账（决定）改了 → 广播（docs/APP-STATE.md §3.5） */
            super::notify_app_state(&handle);
            Ok(out)
        })
    })
    .await
    .map_err(|e| {
        AppError::internal(messages::text("ui.ipc.param_sync.107", &[])).with_detail(e.to_string())
    })?
}

/* ---------- 读 ---------- */

/// 读一份用户预设的三方账。`fetch` = 允许为了拿到云端当前版的字节发一次网络请求。
///
/// 两个根由调用方给（不是从 `AppHandle` 现取）：于是这一条**端到端可判**——
/// 摆一个临时目录（catalog + 基准 + 我那份文件）就能把三方账与落决定都跑一遍，
/// 不必起一个真应用。
fn sync_at(
    root: &std::path::Path,
    user: &std::path::Path,
    path: &str,
    fetch: bool,
) -> Result<PresetParamSyncDto, AppError> {
    let catalog = runtime::load_released_catalog(root)?;

    let text = runtime::local::read_text(user, path)?;
    let lineage = runtime::lineage::parse_lineage_from_content(&text);
    /* 读得出来就顺带把"这份当初基于哪一版"的日期给人看（读不出来就是不写，不编） */
    let based_on_release_time = lineage
        .as_ref()
        .and_then(|l| l.based_on_release_time.clone());
    let based_on_sha256 = lineage
        .as_ref()
        .and_then(|l| l.based_on_sha256.as_deref())
        .map(str::to_ascii_lowercase);
    let based_on_sha256 = based_on_sha256.as_deref();

    let file_name = path.rsplit('/').next().unwrap_or(path).to_owned();
    let cloud = cloud_file_of(&catalog, &text, lineage.as_ref());

    let (owner_machine, owner_version) = owner_of(&catalog, &text, lineage.as_ref());

    /* 云端当前版的正文 + 它的身份摘要（拿不到就是 None：界面照实说"还没取回来"） */
    let (cloud_text, current_sha256) = match cloud {
        Some(f) => {
            let text = cloud_text_for(root, f, fetch)?;
            let sha = match f.expected_sha() {
                Some(want) => Some(want.to_ascii_lowercase()),
                None => text.as_deref().map(|t| hex(&Sha256::digest(t.as_bytes()))),
            };
            (text, sha)
        }
        None => (None, None),
    };
    let current_release_time = cloud_text
        .as_deref()
        .and_then(runtime::lineage::parse_release_time);

    let defs = &catalog.registry.params;
    let decisions = decisions_of(root, path)?;

    /* 要比的那几版：当前版（可能刚取回来）+ 逐项水位 / 血统指的那些 */
    let mut by_sha: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    if let (Some(sha), Some(text)) = (current_sha256.as_deref(), cloud_text.as_deref()) {
        by_sha.insert(sha.to_owned(), param_alg::read_param_values(text, defs)?);
    }
    for sha in param_sync::needed_shas(defs, &decisions, based_on_sha256) {
        if by_sha.contains_key(&sha) {
            continue;
        }
        if let Some(text) = runtime::baseline::read_baseline(root, &sha)? {
            by_sha.insert(sha.clone(), param_alg::read_param_values(&text, defs)?);
        }
    }

    let local = param_alg::read_param_values(&text, defs)?;
    let (entries, pending_count) = param_sync::diff(&SyncInput {
        defs,
        local: &local,
        decisions: &decisions,
        based_on_sha256,
        current_sha256: current_sha256.as_deref(),
        by_sha: &by_sha,
    });

    Ok(PresetParamSyncDto {
        path: path.to_owned(),
        file_name,
        machine_id: owner_machine,
        version_id: owner_version,
        cloud_file_name: cloud.map(|f| f.file_name.clone()),
        version_advanced: matches!(
            (based_on_sha256, current_sha256.as_deref()),
            (Some(b), Some(c)) if b != c
        ),
        current_sha256,
        based_on_release_time,
        current_release_time,
        cloud_ready: cloud_text.is_some(),
        pending_count,
        entries: entries
            .into_iter()
            .map(|e| {
                /* 冲突也是这一项的四方账算出来的 —— 先比完再挪值（同源、同一处口径） */
                let conflict = e.is_conflict();
                ParamSyncEntryDto {
                    key: e.key,
                    local: e.local,
                    baseline_old: e.cloud_old,
                    cloud_new: e.cloud_new,
                    pending: e.pending,
                    conflict,
                    decided: e.decided.map(kind_text),
                }
            })
            .collect(),
    })
}

/// 这一份的逐参数决定账。路径先过用户线那两道闸 —— 状态账的键也必须是这一格里的。
fn decisions_of(
    root: &std::path::Path,
    path: &str,
) -> Result<BTreeMap<String, ParamDecision>, AppError> {
    runtime::local::check_local_prefix(path)?;
    runtime::app_state::param_decisions(root, path)
}

/// 云端那一份预设是谁：血统里的 `based_on` 优先（那是**当初**的来源），
/// 认不出就退回这份文件自己的归属（`# machine:` / `# variant:`）。
fn cloud_file_of<'a>(
    catalog: &'a Catalog,
    text: &str,
    lineage: Option<&runtime::lineage::Lineage>,
) -> Option<&'a CatalogFile> {
    if let Some(based_on) = lineage.and_then(|l| l.based_on.as_deref()) {
        let name = based_on.rsplit('/').next().unwrap_or(based_on);
        if let Some(f) = catalog
            .files
            .iter()
            .find(|f| f.kind == runtime::catalog::kind::PRESET && f.file_name == name)
        {
            return Some(f);
        }
    }
    let (machine, version) = owner_of(catalog, text, lineage);
    let machine = machine?;
    catalog.files.iter().find(|f| {
        f.kind == runtime::catalog::kind::PRESET
            && f.machine_id.eq_ignore_ascii_case(&machine)
            && version
                .as_deref()
                .map_or(true, |v| f.version_id.eq_ignore_ascii_case(v))
    })
}

/// 这份文件自己的归属：`# machine:` / `# variant:` 两行，对着目录**大小写无关**归一化成
/// 目录里的 id；头里没有 / 认不出就回落到血统里那份来源的归属。认不出就是 `(None, None)`。
fn owner_of(
    catalog: &Catalog,
    text: &str,
    lineage: Option<&runtime::lineage::Lineage>,
) -> (Option<String>, Option<String>) {
    let machine_raw = runtime::lineage::parse_machine_from_content(text);
    let version_raw = runtime::lineage::parse_variant_from_content(text);
    let by_machine = machine_raw.as_deref().and_then(|m| {
        catalog
            .machines
            .iter()
            .find(|x| x.id.eq_ignore_ascii_case(m))
    });
    if let Some(m) = by_machine {
        let version = version_raw.as_deref().and_then(|v| {
            m.versions
                .iter()
                .find(|x| x.id.eq_ignore_ascii_case(v))
                .map(|x| x.id.clone())
        });
        return (Some(m.id.clone()), version);
    }
    /* 头里认不出 → 回落到血统里那份来源（与用户线列表同一套口径） */
    let source = lineage
        .and_then(|l| l.based_on.as_deref())
        .and_then(|b| b.rsplit('/').next())
        .and_then(|name| catalog.files.iter().find(|f| f.file_name == name));
    match source {
        Some(f) => (Some(f.machine_id.clone()), Some(f.version_id.clone())),
        None => (None, None),
    }
}

/// 云端当前版的正文，**只在本机找**（除非 `fetch` 明确要求取回）。
///
/// 找法有先后，两个落点都信得过：
///   ① 盘上那份（`<appDataDir>/<catalog.path>`，下载区，字节与目录登记对得上才算）；
///   ② 基准区里那一份（`baseline/<sha>.toml`，下载管道落的）。
///
/// 都没有 ⇒ `Ok(None)`：**这不是错误**（还没取回云端新版），界面照实说。
fn cloud_text_for(
    root: &std::path::Path,
    file: &CatalogFile,
    fetch: bool,
) -> Result<Option<String>, AppError> {
    if let Some(text) = local_cloud_text(root, file) {
        return Ok(Some(text));
    }
    if let Some(want) = file.expected_sha() {
        if let Some(text) = runtime::baseline::read_baseline(root, want)? {
            return Ok(Some(text));
        }
    }
    if !fetch {
        return Ok(None);
    }
    /*
     * 取不回来**不算这条命令失败**：离线 / 源不通都是常态，用户要的仍然是他那份参数。
     * 表现成 `cloudReady: false` + `versionAdvanced: true`（界面说"云端有新版本，
     * 还没取回来"）—— 比整页报错有用得多。
     */
    if let Err(e) = super::catalog::ensure_cloud_bytes(root, file) {
        tracing::warn!(file = %file.file_name, detail = %e.message, "云端当前版没取回来（照旧比不了）");
        return Ok(None);
    }
    if let Some(text) = local_cloud_text(root, file) {
        return Ok(Some(text));
    }
    Ok(file
        .expected_sha()
        .map(|w| runtime::baseline::read_baseline(root, w))
        .transpose()?
        .flatten())
}

/// 盘上那份云端正文（下载区）。字节与目录登记对不上（旧版本 / 被改过）⇒ `None`。
///
/// 顺手把它落进基准区（幂等）—— 于是"这份云端当前版"以后离线也读得到，
/// 用户不用为了看一次云端更新而再连一次网。
fn local_cloud_text(root: &std::path::Path, file: &CatalogFile) -> Option<String> {
    let bytes = std::fs::read(root.join(&file.path)).ok()?;
    let sha = hex(&Sha256::digest(&bytes));
    if let Some(want) = file.expected_sha() {
        if !want.eq_ignore_ascii_case(&sha) {
            return None;
        }
    }
    if let Err(e) = runtime::baseline::ensure_baseline(root, &sha, &bytes) {
        tracing::warn!(file = %file.file_name, detail = %e.message, "基准没落上（不影响这次比对）");
    }
    String::from_utf8(bytes).ok()
}

/* ---------- 写 ---------- */

/// 落一批决定：**以云端当前版为底重写这一份**（注释 / 时间戳 / 新字段跟着新版走），
/// 采用的那几项写成云端新值、其余**保持我这份现在写着的值**，血统换成当前版。
///
/// `decisions` 为空 = 「应用 0 项」：值一个不动，骨架与血统照样推进 —— 于是云端
/// 只改了注释 / 时间戳那种换版也会被消化掉，「有新版」不再一直挂着。
fn apply_at(
    root: &std::path::Path,
    user: &std::path::Path,
    path: &str,
    decisions: &[ParamDecisionDto],
) -> Result<PresetParamSyncDto, AppError> {
    runtime::local::check_local_prefix(path)?;

    /* 先读一份总账：采用需要"云端新值"，水位需要"云端当前版是谁" —— 两者都在里面 */
    let before = sync_at(root, user, path, false)?;
    let Some(current_sha) = before.current_sha256.clone() else {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.param_sync.108",
            &[],
        )));
    };

    /* 骨架要用云端当前版正文：拿不到就如实报错（不许"记了水位、文件却没跟上新版"） */
    let Some((cloud_text, cloud_path)) = current_cloud_at(root, user, path)? else {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.param_sync.109",
            &[],
        )));
    };
    let catalog = runtime::load_released_catalog(root)?;
    let defs = &catalog.registry.params;
    let local_text = runtime::local::read_text(user, path)?;
    let local = param_alg::read_param_values(&local_text, defs)?;
    let cloud = param_alg::read_param_values(&cloud_text, defs)?;

    /* 这一批决定：key → 怎么处理。同一项给两次以后一次为准（界面不会这么发） */
    let mut kinds: BTreeMap<&str, ParamDecisionKind> = BTreeMap::new();
    for d in decisions {
        let kind = match d.kind.as_str() {
            "adopt" => ParamDecisionKind::Adopt,
            "hold" => ParamDecisionKind::Hold,
            other => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.ipc.param_sync.110",
                    &[("other", other)],
                )))
            }
        };
        if kind == ParamDecisionKind::Adopt {
            let entry = before
                .entries
                .iter()
                .find(|e| e.key == d.param_key)
                .ok_or_else(|| {
                    AppError::invalid_argument(messages::text(
                        "ui.ipc.param_sync.111",
                        &[("v1", &d.param_key.to_string())],
                    ))
                })?;
            if entry.cloud_new.is_none() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.ipc.param_sync.112",
                    &[("v1", &d.param_key.to_string())],
                )));
            }
        }
        kinds.insert(d.param_key.as_str(), kind);
    }

    /*
     * 骨架上的值：采用 ⇒ 云端新值（= 骨架本来就写着它，不用改）；
     * 其余（保持 / 没决定）⇒ **我这份现在写着的值**（云端不许顶掉它）。
     * 我这份里没有、云端有的（云端新增的字段）⇒ 跟着骨架进来。
     */
    let mut edits: Vec<FieldEdit> = Vec::new();
    for def in defs {
        let key = def.key.as_str();
        let want = match kinds.get(key) {
            Some(ParamDecisionKind::Adopt) => cloud.get(key),
            Some(ParamDecisionKind::Hold) | None => local.get(key),
        };
        let Some(want) = want else { continue };
        /*
         * 保真写口只改**已存在的键**：云端新版里没有这一项（云端把它删了）就跳过去，
         * 不凭空造一个 —— 那种字段随新版走。
         */
        if !cloud.contains_key(key) || cloud.get(key) == Some(want) {
            continue;
        }
        edits.push(FieldEdit::new(key.to_owned(), want.clone()));
    }
    let skeleton = param_alg::apply_param_edits(&cloud_text, defs, &edits)?;

    /*
     * 血统推不推到云端当前版，看**这一批之后还有没有落下的待处理项**：
     *   都处理了（含"一项都没有"）⇒ 出处换成当前版 —— 「有新版」就此收场；
     *   还剩几项没表态        ⇒ 出处照旧 —— 那几项仍然挂在那儿，不该被这一趟悄悄抹掉。
     */
    let remaining = before
        .entries
        .iter()
        .filter(|e| e.pending && !kinds.contains_key(e.key.as_str()))
        .count();
    let lineage = if remaining == 0 {
        runtime::lineage::Lineage {
            based_on: Some(cloud_path),
            based_on_release_time: runtime::lineage::parse_release_time(&cloud_text),
            based_on_sha256: Some(current_sha.clone()),
        }
    } else {
        runtime::lineage::parse_lineage_from_content(&local_text).unwrap_or_default()
    };
    let next = runtime::lineage::rewrite_keeping_lineage(&skeleton, Some(&lineage));

    /*
     * **参数草稿层要跟着这一趟走**（2026-10-20 实测出来的那条）。
     *
     * 打开过「修改参数」就会留一份草稿（正文 = 打开时盘上那份，哪怕一个字没改）。
     * 这一趟重写了文件（注释 / 时间戳 / 血统，采用时还有值），草稿若原样不动，
     * 它跟新文件就只差注释 —— 列表上亮一枚「未保存」的橙点，点进参数页却写着
     * "没有未保存的改动"；更糟的是再点保存会把云端带进来的注释改回去。
     *
     * 所以分两档：
     *   · 草稿**没改过**（仍是打开时那份正文）⇒ 换成新正文，橙点自然灭；
     *   · 草稿**真改过** ⇒ 不许从它脚下换文件 —— 如实拒绝，先去参数页保存 / 放弃。
     *     （悄悄换掉会把用户的字与云端新版揉成一份谁也说不清的中间态。）
     */
    let draft_before = runtime::app_state::draft(root)?;
    let clean_draft = match &draft_before {
        Some(d)
            if d.origin == runtime::state::ActiveOrigin::Local
                && d.path.as_deref() == Some(path) =>
        {
            d.text == runtime::lineage::strip_lineage_lines(&local_text)
        }
        _ => false,
    };
    if let Some(d) = &draft_before {
        if d.origin == runtime::state::ActiveOrigin::Local
            && d.path.as_deref() == Some(path)
            && !clean_draft
        {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.param_sync.113",
                &[],
            )));
        }
    }

    /*
     * 算出来与原文一样就**不写盘**：一次空操作不该刷新 mtime
     * （与 [`super::preset_params::save_preset_params`] 同一条规矩）。
     */
    if next != local_text {
        /* ★ 原文镜像：写盘**之前**存（那一刻盘上还是原文） */
        let _ = runtime::baseline::ensure_origin_copy(root, user, path);
        runtime::local::write_text(user, path, &next)?;
    }

    if clean_draft {
        if let Some(d) = draft_before {
            runtime::app_state::set_draft(
                root,
                &d.subject(),
                &d.source_sha256,
                &runtime::lineage::strip_lineage_lines(&next),
            )?;
        }
    }

    let record: Vec<(String, ParamDecision)> = kinds
        .iter()
        .map(|(key, kind)| {
            (
                (*key).to_owned(),
                ParamDecision {
                    sha256: current_sha.clone(),
                    kind: *kind,
                },
            )
        })
        .collect();
    if !record.is_empty() {
        runtime::app_state::record_param_decisions(root, path, &record)?;
    }

    sync_at(root, user, path, false)
}

/// 云端当前版的正文 + 它在目录里的路径（血统那行 `based_on` 写它）。
///
/// 只在本机找（与 [`sync_at`] 同一条：读命令不发网络）。
fn current_cloud_at(
    root: &std::path::Path,
    user: &std::path::Path,
    path: &str,
) -> Result<Option<(String, String)>, AppError> {
    let catalog = runtime::load_released_catalog(root)?;
    let text = runtime::local::read_text(user, path)?;
    let lineage = runtime::lineage::parse_lineage_from_content(&text);
    let Some(file) = cloud_file_of(&catalog, &text, lineage.as_ref()) else {
        return Ok(None);
    };
    let cloud_path = file.path.clone();
    Ok(cloud_text_for(root, file, false)?.map(|t| (t, cloud_path)))
}

fn kind_text(kind: ParamDecisionKind) -> &'static str {
    match kind {
        ParamDecisionKind::Adopt => "adopt",
        ParamDecisionKind::Hold => "hold",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presetdata::registry::{ParamDef, ParamRegistry};
    use crate::runtime::catalog::Catalog;

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri 上面就是仓库根")
            .to_path_buf()
    }

    fn catalog() -> Catalog {
        Catalog::build_from_repo(&repo_root()).expect("真目录构建不出来")
    }

    /// 带上摘要的目录：随包那一份**故意不登记 sha**（见 `CatalogFile`），
    /// 而"云端当前版是谁"这件事在逐参数更新里必须知道 —— 所以判据里现算一遍写进去。
    fn catalog_with_shas() -> Catalog {
        let mut catalog = catalog();
        for f in catalog.files.iter_mut() {
            if f.kind != runtime::catalog::kind::PRESET {
                continue;
            }
            let Ok(bytes) = std::fs::read(repo_root().join("presets").join(&f.path)) else {
                continue;
            };
            f.sha256 = Some(crate::runtime::catalog::hex(&Sha256::digest(&bytes)));
            f.size = Some(bytes.len() as u64);
        }
        catalog
    }

    fn defs() -> Vec<ParamDef> {
        ParamRegistry::load_from(&repo_root().join("presets"))
            .expect("读字段定义")
            .params()
            .to_vec()
    }

    /// 归属归一化：头里写小写 `fast`，目录里是 `FAST` —— 大小写无关地对上
    #[test]
    fn owner_normalizes_case_against_the_catalog() {
        let c = catalog();
        let text = "# machine: a1\n# variant: standard\n[toolhead]\noffset_x = -1\n";
        let (m, v) = owner_of(&c, text, None);
        assert_eq!(m.as_deref(), Some("A1"));
        assert_eq!(v.as_deref(), Some("STANDARD"));
    }

    /// 头里认不出就回落到血统里那份来源的归属（不猜）
    #[test]
    fn owner_falls_back_to_the_lineage_source() {
        let c = catalog();
        let text = "# based_on: delivery/mkp/presets/A1-fast.toml\n[toolhead]\noffset_x = -1\n";
        let lineage = runtime::lineage::parse_lineage_from_content(text);
        let (m, v) = owner_of(&c, text, lineage.as_ref());
        assert_eq!(m.as_deref(), Some("A1"));
        assert_eq!(v.as_deref(), Some("FAST"));
    }

    /// 云端那一份按**血统里的来源**认（`A1-standard` 那份不该被这份自己的归属带偏）
    #[test]
    fn the_official_file_comes_from_the_lineage_source() {
        let c = catalog();
        let text = "\
# machine: A1
# variant: fast
# based_on: delivery/mkp/presets/A1-standard.toml
[toolhead]
offset_x = -1
";
        let lineage = runtime::lineage::parse_lineage_from_content(text);
        let f = cloud_file_of(&c, text, lineage.as_ref()).expect("该认出来源那一份");
        assert_eq!(f.file_name, "A1-standard.toml");
    }

    /// 没有血统：退回这份自己的归属 —— 于是导入的云端原件也认得自己对应哪一版
    #[test]
    fn without_lineage_the_owner_decides() {
        let c = catalog();
        let text = "# machine: A1\n# variant: fast\n[toolhead]\noffset_x = -1\n";
        let f = cloud_file_of(&c, text, None).expect("该按归属认出来");
        assert_eq!(f.file_name, "A1-fast.toml");
    }

    /// 一份真目录 + 一份真预设正文：三方账算得出来，且「我改过的那一项」没被云端顶掉
    #[test]
    fn a_sync_report_finds_the_changed_param() {
        let defs = defs();
        let cloud =
            std::fs::read_to_string(repo_root().join("presets/delivery/mkp/presets/A1-fast.toml"))
                .expect("读交付预设文件");

        let mut new_values = param_alg::read_param_values(&cloud, &defs).unwrap();
        let key = "toolhead.offset.x";
        let old_value = new_values.get(key).cloned().expect("这一项该有值");
        /* 云端改了一项（模拟新版），本地那份写的是自己的值 */
        let changed = format!("{old_value}9");
        new_values.insert(key.to_owned(), changed.clone());

        let mut old_map = BTreeMap::new();
        old_map.insert(key.to_owned(), old_value);
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), old_map);
        by_sha.insert("new".to_owned(), new_values);
        let mut local = BTreeMap::new();
        local.insert(key.to_owned(), "7.5".to_owned());

        let (entries, pending) = param_sync::diff(&SyncInput {
            defs: &defs,
            local: &local,
            decisions: &BTreeMap::new(),
            based_on_sha256: Some("old"),
            current_sha256: Some("new"),
            by_sha: &by_sha,
        });

        assert_eq!(pending, 1, "只有那一项该进待处理");
        let e = entries.iter().find(|e| e.key == key).unwrap();
        assert!(e.pending);
        assert_eq!(e.local.as_deref(), Some("7.5"), "本地值不许被顶掉");
        assert_eq!(e.cloud_new.as_deref(), Some(changed.as_str()));
    }

    /* ---------- 端到端：摆一个真世界，走一遍场景 A–E ---------- */

    /// 「我改的那一项」—— 用它当整条链上的那一个参数
    const KEY: &str = "toolhead.offset.x";
    /// 演示用的用户文件名
    const LOCAL: &str = "presets-local/本地 A1.toml";

    /// 一个摆得出来的世界：真目录（真注册表 + 9 份交付件的清单）+ 基准区 + 本地那份。
    ///
    /// 云端"当前版"和"我这份当初基于的那一版"都从**仓库里那份真交付件**派生
    /// （后者用同一个保真写口改一项）—— 不是手编两段 TOML。
    struct World {
        /// 名字带下划线是因为它**只用来占住这个临时目录**（删了就没了）
        _dir: tempfile::TempDir,
        root: std::path::PathBuf,
        user: std::path::PathBuf,
        /// 云端当前版的正文与摘要
        current_text: String,
        current_sha: String,
        /// 我这份当初基于的那一版（= 云端当前版改过 `KEY` 的那一份）
        based_text: String,
    }

    impl World {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            let user = root.join("user");
            std::fs::create_dir_all(user.join("presets-local")).unwrap();

            let defs = defs();
            let catalog = catalog_with_shas();
            crate::fsx::atomic::atomic_write(
                &root.join("catalog.json"),
                catalog.to_pretty_json().unwrap().as_bytes(),
            )
            .unwrap();
            let current_sha = catalog
                .files
                .iter()
                .find(|f| f.file_name == "A1-fast.toml")
                .and_then(|f| f.expected_sha())
                .expect("目录里该登记着 A1-fast.toml 的摘要")
                .to_owned();
            let current_text = std::fs::read_to_string(
                repo_root().join("presets/delivery/mkp/presets/A1-fast.toml"),
            )
            .expect("读交付预设文件");
            runtime::baseline::ensure_baseline(&root, &current_sha, current_text.as_bytes())
                .unwrap();

            /* 我这份当初基于的那一版：云端当前版把这一项改成别的（同一个保真写口） */
            let based_text =
                param_alg::apply_param_edits(&current_text, &defs, &[FieldEdit::new(KEY, "-2")])
                    .unwrap();
            let based_sha = runtime::lineage::sha256_hex(&based_text);
            runtime::baseline::ensure_baseline(&root, &based_sha, based_text.as_bytes()).unwrap();

            /* 我那份 = 基于那一版 + 我自己把它改成 -5（场景 B：改过的值不许被顶掉） */
            let copy =
                runtime::lineage::make_copy(&based_text, "delivery/mkp/presets/A1-fast.toml");
            let local =
                param_alg::apply_param_edits(&copy, &defs, &[FieldEdit::new(KEY, "-5")]).unwrap();
            crate::fsx::atomic::atomic_write(&user.join(LOCAL), local.as_bytes()).unwrap();

            Self {
                _dir: dir,
                root,
                user,
                current_text,
                current_sha,
                based_text,
            }
        }

        fn sync(&self) -> PresetParamSyncDto {
            sync_at(&self.root, &self.user, LOCAL, false).unwrap()
        }

        fn at(&self, key: &str) -> ParamSyncEntryDto {
            self.sync()
                .entries
                .into_iter()
                .find(|e| e.key == key)
                .expect("这一项该在结果里")
        }

        fn local_text(&self) -> String {
            runtime::local::read_text(&self.user, LOCAL).unwrap()
        }

        fn decide(&self, key: &str, kind: &str) -> PresetParamSyncDto {
            apply_at(
                &self.root,
                &self.user,
                LOCAL,
                &[ParamDecisionDto {
                    param_key: key.to_owned(),
                    kind: kind.to_owned(),
                }],
            )
            .unwrap()
        }

        /// 云端再发一版：把 `KEY` 改成 `next`，并把目录里的摘要推进过去
        fn publish(&self, next: &str) {
            let defs = defs();
            let text = param_alg::apply_param_edits(
                &self.current_text,
                &defs,
                &[FieldEdit::new(KEY, next)],
            )
            .unwrap();
            self.publish_text(text);
        }

        /// 云端发一版**任意正文**（注释级变更那种：值一项都没动，摘要照样变）
        fn publish_text(&self, text: String) {
            let sha = runtime::lineage::sha256_hex(&text);
            runtime::baseline::ensure_baseline(&self.root, &sha, text.as_bytes()).unwrap();
            let mut catalog = catalog_with_shas();
            for f in catalog.files.iter_mut() {
                if f.file_name == "A1-fast.toml" {
                    f.sha256 = Some(sha.clone());
                    f.size = Some(text.len() as u64);
                }
            }
            crate::fsx::atomic::atomic_write(
                &self.root.join("catalog.json"),
                catalog.to_pretty_json().unwrap().as_bytes(),
            )
            .unwrap();
        }
    }

    /// 场景 A/B：云端改过的那一项进待处理；**我改过的值（-5）一个字没动**
    #[test]
    fn scenario_a_and_b_an_update_shows_up_without_touching_my_value() {
        let w = World::new();
        let report = w.sync();
        assert_eq!(report.pending_count, 1, "只有云端改过的那一项：{report:?}");
        assert!(report.version_advanced, "云端换版了");
        assert_eq!(report.cloud_file_name.as_deref(), Some("A1-fast.toml"));
        assert_eq!(report.machine_id.as_deref(), Some("A1"));
        assert_eq!(report.version_id.as_deref(), Some("FAST"));
        assert!(
            report.based_on_release_time.is_some(),
            "血统里那行日期该读得到"
        );

        let e = w.at(KEY);
        assert!(e.pending);
        assert_eq!(e.local.as_deref(), Some("-5"), "本地值不许被顶掉");
        assert_eq!(
            e.baseline_old.as_deref(),
            Some("-2"),
            "云端旧值 = 我处理到的那一版"
        );
        assert_eq!(e.cloud_new.as_deref(), Some("-0.7"));
        assert_eq!(e.decided, None, "还没处理过");
    }

    /// 场景 D：采用 = 文件里那一项写成云端新值 + 水位推到当前版；之后不再挂着
    #[test]
    fn scenario_d_adopting_writes_the_new_value_and_retires_the_flag() {
        let w = World::new();
        let after = w.decide(KEY, "adopt");
        assert_eq!(after.pending_count, 0, "处理过就不该再挂着");

        let e = after.entries.iter().find(|e| e.key == KEY).unwrap();
        assert_eq!(e.decided, Some("adopt"));
        assert_eq!(e.local.as_deref(), Some("-0.7"), "文件里那一项该是云端新值");

        /* 文件真的被改了，而且**只改了那一行**（保真：注释 / 别的行一个字不动） */
        let text = w.local_text();
        assert!(text.contains("offset_x = -0.7"), "\n{text}");
        assert!(text.contains("# 笔尖偏移"), "行尾注释要留着\n{text}");
        assert_eq!(
            param_alg::read_param_values(&text, &defs()).unwrap()[KEY],
            "-0.7"
        );
    }

    /// 场景 C：保持 = **这一项还是我原来的值** + 水位推到当前版；之后同一版不再待处理
    ///
    /// （2026-10-20：写那一趟改成「以云端当前版为底重写」，所以文件不再是"一个字节都不动" ——
    /// 动的只有注释 / 时间戳 / 新字段 / 血统，**值**必须还是我原来那个。）
    #[test]
    fn scenario_c_holding_keeps_my_value() {
        let w = World::new();
        let after = w.decide(KEY, "hold");

        assert_eq!(after.pending_count, 0);
        assert_eq!(
            after.entries.iter().find(|e| e.key == KEY).unwrap().decided,
            Some("hold")
        );
        assert_eq!(
            param_alg::read_param_values(&w.local_text(), &defs()).unwrap()[KEY],
            "-5",
            "保持 = 这一项还是我原来的值"
        );
    }

    /// **应用 0 项**（云端只改了注释 / 时间戳）：新注释与新的血统进来、
    /// 我的值一个都不动 —— 「有新版」就此收场
    #[test]
    fn applying_zero_items_digests_a_comment_only_release() {
        let w = World::new();
        /* 先把那一项处理掉（保持我的值）—— 于是"云端换版"只剩注释那点差别 */
        assert_eq!(w.decide(KEY, "hold").pending_count, 0);
        /* 值一项都没有变，只多了一行注释 —— 摘要照样变（血统比的是全文） */
        w.publish_text(format!("# 这一版只改了注释\n{}", w.current_text));

        let before = w.sync();
        assert_eq!(before.pending_count, 0, "值一项都没动：{before:?}");
        assert!(before.version_advanced, "摘要变了 ⇒ 云端换版了");

        let after = apply_at(&w.root, &w.user, LOCAL, &[]).unwrap();
        assert!(
            !after.version_advanced,
            "应用 0 项之后不再挂着「有新版」：{after:?}"
        );
        assert_eq!(after.pending_count, 0);

        let local = w.local_text();
        assert!(
            local.contains("# 这一版只改了注释"),
            "新注释进来了\n{local}"
        );
        assert_eq!(
            param_alg::read_param_values(&local, &defs()).unwrap()[KEY],
            "-5",
            "我的值一个都没被顶掉"
        );
        let lineage = runtime::lineage::parse_lineage_from_content(&local).expect("血统还在");
        assert_eq!(
            lineage.based_on_sha256.as_deref(),
            after.current_sha256.as_deref(),
            "血统推到云端当前版 —— 这就是「有新版」消失的根据"
        );
    }

    /// **云端新增的字段**跟着新版进来（本地那份里本来没有它）
    #[test]
    fn a_brand_new_cloud_field_comes_in_with_the_new_version() {
        let w = World::new();
        let defs = defs();
        /* 我这份里本来没有这一项（先删掉），云端新版里有它 */
        let local = w.local_text();
        let without = remove_line(&local, "offset_y");
        crate::fsx::atomic::atomic_write(&w.user.join(LOCAL), without.as_bytes()).unwrap();
        assert!(
            !param_alg::read_param_values(&w.local_text(), &defs)
                .unwrap()
                .contains_key(K2),
            "这一项现在不在我这份里"
        );

        let after = apply_at(&w.root, &w.user, LOCAL, &[]).unwrap();
        let values = param_alg::read_param_values(&w.local_text(), &defs).unwrap();
        assert_eq!(
            values.get(K2).map(String::as_str),
            after
                .entries
                .iter()
                .find(|e| e.key == K2)
                .and_then(|e| e.cloud_new.as_deref()),
            "云端新增的字段照云端的值进来"
        );
    }

    /** 另一项参数（"云端新增字段"那条要用） */
    const K2: &str = "toolhead.offset.y";

    /// 删掉含这个片段的那一行（摆"我这份里没有这一项"用）
    fn remove_line(text: &str, needle: &str) -> String {
        text.lines()
            .filter(|l| !l.contains(needle))
            .map(|l| format!("{l}\n"))
            .collect()
    }

    /// 场景 C（续）/ E：云端又发一版 —— 旧值取「我处理到的那一版」，本地值还是我保持的那个
    #[test]
    fn scenario_e_the_next_release_compares_against_my_watermark() {
        let w = World::new();
        assert_eq!(w.decide(KEY, "hold").pending_count, 0, "先处理掉这一版");

        /* 云端再改一次这一项 */
        w.publish("-9");

        let report = w.sync();
        assert_eq!(report.pending_count, 1, "云端又改了这一项");
        let e = report.entries.iter().find(|e| e.key == KEY).unwrap();
        assert!(e.pending);
        assert_eq!(
            e.baseline_old.as_deref(),
            Some("-0.7"),
            "旧值 = 上次处理到的那一版（不是我最初血统里那一版）"
        );
        assert_eq!(e.cloud_new.as_deref(), Some("-9"));
        assert_eq!(
            e.local.as_deref(),
            Some("-5"),
            "本地值还是我保持下的那个（= 我那份文件里写着的）"
        );
    }

    /// 水位是**逐项**的：同一份里一项处理过、另一项没处理，两边各自算各自的
    #[test]
    fn the_watermark_is_per_param() {
        let w = World::new();
        let defs = defs();
        /* 云端再改第二项：于是这一份上有两项待处理 */
        let next = param_alg::apply_param_edits(
            &w.current_text,
            &defs,
            &[FieldEdit::new("toolhead.offset.y", "19")],
        )
        .unwrap();
        let sha = runtime::lineage::sha256_hex(&next);
        runtime::baseline::ensure_baseline(&w.root, &sha, next.as_bytes()).unwrap();
        let mut catalog = catalog_with_shas();
        for f in catalog.files.iter_mut() {
            if f.file_name == "A1-fast.toml" {
                f.sha256 = Some(sha.clone());
            }
        }
        crate::fsx::atomic::atomic_write(
            &w.root.join("catalog.json"),
            catalog.to_pretty_json().unwrap().as_bytes(),
        )
        .unwrap();

        let report = w.sync();
        assert_eq!(report.pending_count, 2, "两项都改了：{report:?}");

        /* 只处理掉第一项 */
        let after = w.decide(KEY, "adopt");
        assert_eq!(after.pending_count, 1, "第二项还等着");
        let second = after
            .entries
            .iter()
            .find(|e| e.key == "toolhead.offset.y")
            .unwrap();
        assert!(second.pending);
        assert_eq!(second.decided, None, "它没被处理过");
        assert_eq!(
            second.baseline_old.as_deref(),
            Some("26.3"),
            "它的旧值仍取血统里那一版（没被第一项的水位带跑）"
        );
    }

    /// 血统里的来源名字**不在目录里**时退回这份自己的归属 —— 照样认得出对应哪一版云端
    #[test]
    fn a_missing_lineage_source_falls_back_to_the_own_attribution() {
        let w = World::new();
        let text = runtime::lineage::make_copy(&w.based_text, "mkp/presets/不存在的预设.toml");
        crate::fsx::atomic::atomic_write(&w.user.join(LOCAL), text.as_bytes()).unwrap();

        let report = w.sync();
        assert_eq!(report.cloud_file_name.as_deref(), Some("A1-fast.toml"));
        assert_eq!(report.pending_count, 1, "退回归属之后照样算得出更新");
    }

    /// 连归属都认不出（头里的机型不在目录里、也没有血统）⇒ 一项都不报，也不报错
    #[test]
    fn a_preset_that_maps_to_no_official_file_is_not_an_error() {
        let w = World::new();
        let text = w
            .based_text
            .replace("# machine: A1", "# machine: 不存在的机型");
        crate::fsx::atomic::atomic_write(&w.user.join(LOCAL), text.as_bytes()).unwrap();

        let report = w.sync();
        assert_eq!(report.pending_count, 0);
        assert!(report.cloud_file_name.is_none());
        assert!(!report.cloud_ready);
    }

    /// 采用一个**云端新版还没取回来**的项：如实报错，不许"记了水位、值却没写"
    #[test]
    fn adopting_before_the_official_bytes_are_here_is_an_error() {
        let w = World::new();
        /* 把云端当前版的基准抽掉（模拟"这一版还没取回来"） */
        std::fs::remove_file(runtime::baseline::baseline_path(&w.root, &w.current_sha)).unwrap();
        let report = w.sync();
        assert!(!report.cloud_ready);
        assert_eq!(report.pending_count, 0, "拿不到新值就不许报待处理");

        let err = apply_at(
            &w.root,
            &w.user,
            LOCAL,
            &[ParamDecisionDto {
                param_key: KEY.to_owned(),
                kind: "adopt".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            runtime::app_state::param_decisions(&w.root, LOCAL)
                .unwrap()
                .is_empty(),
            "一项水位都不许留下"
        );
    }

    /// 认不出的处理方式 / 不存在的参数：都拒绝（不让界面写出第三种决定来）
    #[test]
    fn an_unknown_decision_or_param_is_refused() {
        let w = World::new();
        let bad_kind = apply_at(
            &w.root,
            &w.user,
            LOCAL,
            &[ParamDecisionDto {
                param_key: KEY.to_owned(),
                kind: "随便".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(bad_kind.code, crate::error::ErrorCode::InvalidArgument);

        let bad_key = apply_at(
            &w.root,
            &w.user,
            LOCAL,
            &[ParamDecisionDto {
                param_key: "没有.这个参数".to_owned(),
                kind: "adopt".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(bad_key.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// **打开过参数页留下的干净草稿要跟着重写走**（2026-10-20 实测那条）：
    /// 这一趟把文件换成云端骨架之后，草稿也该是新正文 —— 否则列表亮"未保存"、
    /// 再点保存还会把云端带进来的注释改回去。
    #[test]
    fn a_clean_draft_follows_the_rewrite() {
        let w = World::new();
        let clean = runtime::lineage::strip_lineage_lines(&w.local_text());
        let subject = runtime::state::DraftSubject::local("本地 A1.toml", LOCAL);
        runtime::app_state::set_draft(&w.root, &subject, "旧水位", &clean).unwrap();

        /* 「应用 0 项」：值一个不动，但骨架与血统换成云端当前版 */
        apply_at(&w.root, &w.user, LOCAL, &[]).unwrap();

        let after = runtime::app_state::draft(&w.root)
            .unwrap()
            .expect("草稿该还在");
        assert_eq!(
            after.text,
            runtime::lineage::strip_lineage_lines(&w.local_text()),
            "草稿正文要跟着新文件走"
        );
    }

    /// **真改过的草稿不许被踩**：这一趟如实拒绝，文件一个字节不动 ——
    /// 悄悄换掉就成了"用户的字 + 云端新版"揉出来的中间态。
    #[test]
    fn a_dirty_draft_blocks_the_rewrite() {
        let w = World::new();
        let before = w.local_text();
        let mut dirty = runtime::lineage::strip_lineage_lines(&before);
        dirty.push_str("\n# 用户自己敲的一行\n");
        let subject = runtime::state::DraftSubject::local("本地 A1.toml", LOCAL);
        runtime::app_state::set_draft(&w.root, &subject, "旧水位", &dirty).unwrap();

        let err = apply_at(&w.root, &w.user, LOCAL, &[]).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(w.local_text(), before, "文件不许被换掉");
        assert_eq!(
            runtime::app_state::draft(&w.root).unwrap().unwrap().text,
            dirty,
            "草稿也不许被动"
        );
    }
}
