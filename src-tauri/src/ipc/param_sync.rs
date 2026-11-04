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
//! 「有新版」（来源记录摘要比的是全文）。所以写那一趟的形状是：
//!
//! ```text
//!   骨架 = 云端当前版正文（注释 / 时间戳 / 新字段都在里面）
//!   值    采用(adopt) → 云端新值；其余 → **我这份文件里现在写着的值**（一个不许被顶掉）
//!   来源记录  换成云端当前版：based_on / based_on_sha256 / based_on_release_time
//!         （后者写的是目录 `publishedAt`，不是文件头；新产物没有 `# release_time`）
//! ```
//!
//! 于是 **0 项也是一次真写**（"自动应用 0 项"）：值一个不动，注释 / 时间戳 /
//! 新字段 / 来源记录跟着新版走 —— 点「更新」之后「有新版」自然消失。
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
use crate::runtime::app_state::{ParamDecision, ParamDecisionKind, ParamOrigin};
use crate::runtime::catalog::{hex, Catalog, CatalogFile};
use crate::runtime::param_sync::{self, SyncInput};
use preset::model::{SourceProvenance, SourceRef};

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
    /// **上次云端值**（2026-10-31 的滚动记录）：界面那个「上次云端」用它，
    /// 不被采用/保持推进。`None` = 还没记过（旧档 / 从没改过）⇒ 界面退回 `baselineOld`。
    pub origin: Option<String>,
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
    /// 「我这份当初基于的那一版」发布时刻：来源记录里有就用（用户接受某一版时写的是
    /// 目录 `publishedAt`），否则取来路账（下载那一刻定格）。都没有 ⇒ `None`。
    pub based_on_release_time: Option<String>,
    /// 「云端当前这一版」的发布时刻 = 当前目录的 `catalog.publishedAt`。
    /// 认不出云端文件 ⇒ `None`。
    pub current_release_time: Option<String>,
    /// 云端当前版的字节**在本机可用**（读得到就能逐项比）
    pub cloud_ready: bool,
    /// 云端换版了：当前版与我这份来源记录里那一版不是同一份
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
            /* 拷贝件不认云端 ⇒ 整本账都不给它（连目录都不必读） */
            if !runtime::app_state::copies_follow_cloud(&root)?
                && runtime::source_ledger::copy_paths(&root).contains(&path)
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
    let source = runtime::provenance::parse(&text);
    /*
     * 「我这份当初基于哪一版」的发布时刻 —— **只认文件尾部的来源记录**
     * （2026-11-04 作者裁定：详情页一律以尾部记录为准，不再用 `run/` 的账顶）：
     *   · 记录里履历最后一条的 `at` ⇒ 用它（接受 / 下载那一刻目录的 `publishedAt`）；
     *   · 没有记录（手工拖进来的 / 老副本）⇒ `None`，界面照实说未知。
     * 不许拿文件头的 `generated_at`（写盘时刻）顶。
     */
    let latest = source.as_ref().and_then(|p| p.latest_version());
    let based_on_release_time = latest.and_then(|v| v.at.clone());
    let based_on_sha256 = latest
        .and_then(|v| v.sha256.as_deref())
        .map(str::to_ascii_lowercase);
    let based_on_sha256 = based_on_sha256.as_deref();

    let file_name = path.rsplit('/').next().unwrap_or(path).to_owned();
    let cloud = cloud_file_of(&catalog, &text, source.as_ref());

    let (owner_machine, owner_version) = owner_of(&catalog, &text, source.as_ref());

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
    /*
     * 「云端当前这一版」的发布时刻（2026-11-04 接上）：目录代的 `publishedAt` 才是
     * 发布事件真值。文件头的 `# release_time` 只是配方自己的标记，新产物已不再写它，
     * 不许拿它冒充。认不出云端文件（`cloud == None`）⇒ 未知。
     */
    let current_release_time = if cloud.is_some() {
        catalog.published_at.clone()
    } else {
        None
    };

    let defs = &catalog.registry.params;
    let decisions = decisions_of(root, path)?;

    /* 要比的那几版：当前版（可能刚取回来）+ 逐项水位 / 来源记录指的那些 */
    let mut by_sha: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    if let (Some(sha), Some(text)) = (current_sha256.as_deref(), cloud_text.as_deref()) {
        by_sha.insert(sha.to_owned(), param_alg::read_param_values(text, defs)?);
    }
    for sha in param_sync::needed_shas(defs, &decisions, based_on_sha256) {
        if by_sha.contains_key(&sha) {
            continue;
        }
        if let Some(text) = runtime::content::read(root, &sha)? {
            by_sha.insert(sha.clone(), param_alg::read_param_values(&text, defs)?);
        }
    }

    let local = param_alg::read_param_values(&text, defs)?;
    /* 「上次云端值」的滚动记录（没记过 = 空表，界面退回旧口径） */
    let origins = runtime::app_state::param_origins(root, path)?;
    let (entries, pending_count) = param_sync::diff(&SyncInput {
        defs,
        local: &local,
        decisions: &decisions,
        based_on_sha256,
        current_sha256: current_sha256.as_deref(),
        by_sha: &by_sha,
        origins: &origins,
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
                    origin: e.origin,
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

/// 云端那一份预设是谁：来源记录里的 `based_on` 优先（那是**当初**的来源），
/// 认不出就退回这份文件自己的归属（`# machine:` / `# variant:`）。
fn cloud_file_of<'a>(
    catalog: &'a Catalog,
    text: &str,
    source: Option<&SourceProvenance>,
) -> Option<&'a CatalogFile> {
    if let Some(based_on) = source_file_path(source) {
        let name = based_on.rsplit('/').next().unwrap_or(based_on);
        if let Some(f) = catalog
            .files
            .iter()
            .find(|f| f.kind == runtime::catalog::kind::PRESET && f.file_name == name)
        {
            return Some(f);
        }
    }
    let (machine, version) = owner_of(catalog, text, source);
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
/// 目录里的 id；头里没有 / 认不出就回落到来源记录里那份来源的归属。认不出就是 `(None, None)`。
fn owner_of(
    catalog: &Catalog,
    text: &str,
    source: Option<&SourceProvenance>,
) -> (Option<String>, Option<String>) {
    let machine_raw = runtime::provenance::parse_machine_from_content(text);
    let version_raw = runtime::provenance::parse_variant_from_content(text);
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
    /* 头里认不出 → 回落到来源记录里那份（与用户线列表同一套口径） */
    let found = source_file_path(source)
        .and_then(|b| b.rsplit('/').next())
        .and_then(|name| catalog.files.iter().find(|f| f.file_name == name));
    match found {
        Some(f) => (Some(f.machine_id.clone()), Some(f.version_id.clone())),
        None => (None, None),
    }
}

/// 两条记录是不是**同一版云端**（判「这一版已经接受过」，避免重复记履历）。
///
/// 比的是版本身份：发布线 + 序号 + 修订 + 原文摘要。**不比时间** —— `at` 只做展示，
/// 不是身份（AGENTS.md §6：同一判据不许两个源）。
fn same_version(a: &SourceRef, b: &SourceRef) -> bool {
    fn same_opt(x: Option<&String>, y: Option<&String>) -> bool {
        match (x, y) {
            (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
            (None, None) => true,
            // 一边有记录、一边没有 ⇒ 说不清是不是同一版。
            // 宁可按「没接受过」多记一条 —— 多记是照实，少记是篡改履历。
            _ => false,
        }
    }
    a.serial == b.serial
        && same_opt(a.catalog.as_ref(), b.catalog.as_ref())
        && same_opt(a.revision.as_ref(), b.revision.as_ref())
        && same_opt(a.sha256.as_ref(), b.sha256.as_ref())
}

/// 来源记录里「我从哪个云端文件来的」（履历最后一条优先，其次 `origin`）。
fn source_file_path(source: Option<&SourceProvenance>) -> Option<&str> {
    let source = source?;
    source
        .latest_version()
        .and_then(|v| v.file.as_deref())
        .or_else(|| source.origin.as_ref().and_then(|o| o.file.as_deref()))
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
    if !fetch {
        return Ok(None);
    }
    /*
     * 取不回来**不算这条命令失败**：离线 / 源不通都是常态，用户要的仍然是他那份参数。
     * 表现成 `cloudReady: false` + `versionAdvanced: true`（界面说"云端有新版本，
     * 还没取回来"）—— 比整页报错有用得多。
     */
    if let Err(e) = super::catalog::deliver_cloud_bytes(root, file) {
        tracing::warn!(file = %file.file_name, detail = %e.message, "云端当前版没取回来（照旧比不了）");
        return Ok(None);
    }
    Ok(local_cloud_text(root, file))
}

/// 本机的云端当前版正文。
///
/// 预设：`content/<目录登记 SHA>`（内容仓库；同内容被别的 path 下过也能用），
/// 再退回索引里"手上这一版"（只有它等于目录登记值时才可用）。
/// 其余：旧路径读盘（字节与目录登记对不上 ⇒ `None`）。
fn local_cloud_text(root: &std::path::Path, file: &CatalogFile) -> Option<String> {
    if file.kind == runtime::catalog::kind::PRESET {
        let want = file.expected_sha();
        if let Some(w) = want {
            if let Ok(Some(text)) = runtime::content::read(root, w) {
                return Some(text);
            }
        }
        let entry = runtime::delivery_index::entry(root, &file.path)?;
        let text = runtime::content::read(root, &entry.sha256).ok().flatten()?;
        return match want {
            Some(w) if !entry.sha256.eq_ignore_ascii_case(w) => None,
            _ => Some(text),
        };
    }
    let bytes = std::fs::read(root.join(&file.path)).ok()?;
    let sha = hex(&Sha256::digest(&bytes));
    if let Some(want) = file.expected_sha() {
        if !want.eq_ignore_ascii_case(&sha) {
            return None;
        }
    }
    String::from_utf8(bytes).ok()
}

/* ---------- 写 ---------- */

/// **滚动「上次云端值」**：整份对齐到新云端之后，把每一格换成**被换下来的那一版**的值。
///
/// 与 [`ParamOrigin`] 的两条规则配套（作者 2026-10-31 定的）：
///
/// ```text
/// 下载 A → 改成 B（记 A）→ 下载 C：上次云端仍是 A（这一项云端没改过）
///                        → 下载 D：上次云端滚成 C
/// ```
///
/// 判据 = 这一项的「旧云端值」与「新云端值」不同。相同就**不动** —— 那是"云端这一项
/// 没改"，用户手改时记下的 A 必须留着；不同才滚，且滚的是**旧的那一侧**（不是我改的值）。
/// 拿不到旧值 ⇒ 跳过这一项（不编一个）。
fn roll_origins(
    root: &std::path::Path,
    path: &str,
    before: &PresetParamSyncDto,
    before_based_on: Option<&str>,
    cloud_new: &BTreeMap<String, String>,
) -> Result<(), AppError> {
    let mut entries: Vec<(String, ParamOrigin)> = Vec::new();
    for e in &before.entries {
        let Some(old) = e.baseline_old.as_deref() else {
            continue;
        };
        let Some(new) = cloud_new.get(&e.key) else {
            continue;
        };
        if old == new {
            continue;
        }
        entries.push((
            e.key.clone(),
            ParamOrigin {
                sha256: before_based_on.map(str::to_owned),
                value: old.to_owned(),
            },
        ));
    }
    runtime::app_state::record_param_origins(root, path, &entries)
}

/// 落一批决定：**以云端当前版为底重写这一份**（注释 / 时间戳 / 新字段跟着新版走），
/// 采用的那几项写成云端新值、其余**保持我这份现在写着的值**，来源记录换成当前版。
///
/// `decisions` 为空 = 「应用 0 项」：值一个不动，骨架与来源记录照样推进 —— 于是云端
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
    /* 滚动要用「应用前的来源版本」：那一版就是这一趟即将被换下来的"上次云端" */
    let before_based_on = runtime::provenance::parse(&local_text)
        .and_then(|p| p.latest_version().and_then(|v| v.sha256.clone()));
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
     * 来源记录推不推到云端当前版，看**这一批之后还有没有落下的待处理项**：
     *   都处理了（含"一项都没有"）⇒ 出处换成当前版 —— 「有新版」就此收场；
     *   还剩几项没表态        ⇒ 出处照旧 —— 那几项仍然挂在那儿，不该被这一趟悄悄抹掉。
     *
     * ★ **这里是唯一"用户明确接受当前版本"的动作**（2026-10-28）。所以只有这一支
     *   才写发布线与序号：`based_on_catalog` / `based_on_serial` 取**当前目录**的
     *   身份，等于说"我这份已经对齐到第 N 号发布了"。
     *
     *   反过来，`remaining > 0`（部分更新）时来源记录原样保留 —— 序号也**不前进**，
     *   于是它仍然显示「有新版」。那是对的：这一趟确实没接受完。
     */
    let remaining = before
        .entries
        .iter()
        .filter(|e| e.pending && !kinds.contains_key(e.key.as_str()))
        .count();
    /* 更新后账上要写"对应哪一版"——先用云端文件名，别等 cloud_path 被 move 走 */
    let cloud_file = cloud_path
        .rsplit('/')
        .next()
        .unwrap_or(&cloud_path)
        .to_owned();
    // 身份三格之外的来源信息（平台 / 仓库 / 分支）沿用本机那份记录里的，
    // 换源不发新版时它们不该被抹掉。
    let previous = runtime::provenance::parse(&local_text);
    let identity_of_prev = previous
        .as_ref()
        .and_then(|p| p.latest_version().cloned())
        .unwrap_or_default();
    let next = if remaining == 0 {
        /* 整份接受这一版：追加一条履历（**旧条目一个字不改**） */
        let version = SourceRef {
            file: Some(cloud_path),
            platform: identity_of_prev.platform,
            repo: identity_of_prev.repo,
            branch: identity_of_prev.branch,
            /* ★「来源版本时间」用**目录的发布时刻**（2026-11-01 定的真值）。
             * 拿不到就留空（界面显示「未知」），**不拿文件修改时间顶**。 */
            at: catalog.published_at.clone(),
            sha256: Some(current_sha.clone()),
            catalog: catalog.catalog_id.clone(),
            serial: catalog.publish_serial,
            revision: Some(catalog.revision.clone()),
            ..SourceRef::default()
        };
        /*
         * ★ **同一个云端版本重复合并不重复记**（2026-11-04）。
         *
         * 履历记的是"我接受过云端哪些版本"，不是"我点了几次合并"。用户把同一版
         * 合并两遍（或先合并、又翻了翻再点一次），履历不该长两条一模一样的。
         * 判据用**版本身份**（发布线 + 序号 + 修订 + 摘要），**不看时间** ——
         * 时间是展示，不是身份（AGENTS.md §6）。
         */
        let already_accepted = previous
            .as_ref()
            .and_then(|p| p.latest_version())
            .is_some_and(|last| same_version(last, &version));
        if already_accepted {
            tracing::info!(
                serial = catalog.publish_serial,
                "这一版已经接受过：履历不重复记，旧条目原样保留"
            );
            runtime::provenance::rewrite_keeping_provenance(&skeleton, previous.as_ref())
        } else {
            runtime::provenance::record_version(&skeleton, version)
        }
    } else {
        tracing::info!(
            remaining,
            serial = catalog.publish_serial,
            "部分更新：来源记录原样保留（序号不前进，仍会显示有新版）"
        );
        runtime::provenance::rewrite_keeping_provenance(&skeleton, previous.as_ref())
    };

    /*
     * **参数草稿层要跟着这一趟走**（2026-10-20 实测出来的那条）。
     *
     * 打开过「修改参数」就会留一份草稿（正文 = 打开时盘上那份，哪怕一个字没改）。
     * 这一趟重写了文件（注释 / 时间戳 / 来源记录，采用时还有值），草稿若原样不动，
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
            d.text == runtime::provenance::strip_for_compare(&local_text)
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
        /*
         * **「上次云端值」滚动**：这一趟把这份对齐到了新云端，被换下来的那一版
         * 就成为新的「上次云端」。记不上不影响更新本身（附加信息，与出处账同口径）。
         */
        if let Err(e) = roll_origins(root, path, &before, before_based_on.as_deref(), &cloud) {
            tracing::warn!(error = %e, "「上次云端值」没滚上，界面会退回旧口径");
        }
        /*
         * 来路账跟着前进：剩下的待处理项为 0 ⇒ 这份已经对齐到当前云端版，
         * 「本地版本时间」要指向**这一版**的发布时刻（不是当初下载那一版）。
         * 身份 / 母本原样保留；没有账时不硬填当前源（那可能是别的分支取的字节）。
         */
        if remaining == 0 {
            let stamp = runtime::source_ledger::CatalogStamp {
                catalog_id: catalog.catalog_id.as_deref(),
                serial: catalog.publish_serial,
                revision: &catalog.revision,
                published_at: catalog.published_at.as_deref(),
            };
            let _ = runtime::source_ledger::record_baseline(
                root,
                path,
                None,
                &cloud_file,
                &stamp,
                Some(&current_sha),
            );
        }
    }

    if clean_draft {
        if let Some(d) = draft_before {
            runtime::app_state::set_draft(
                root,
                &d.subject(),
                &d.source_sha256,
                &runtime::provenance::strip_for_compare(&next),
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

/// 云端当前版的正文 + 它在目录里的路径（来源记录那行 `based_on` 写它）。
///
/// 只在本机找（与 [`sync_at`] 同一条：读命令不发网络）。
fn current_cloud_at(
    root: &std::path::Path,
    user: &std::path::Path,
    path: &str,
) -> Result<Option<(String, String)>, AppError> {
    let catalog = runtime::load_released_catalog(root)?;
    let text = runtime::local::read_text(user, path)?;
    let lineage = runtime::provenance::parse(&text);
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

    /// 头里认不出就回落到**来源记录**里那份的归属（不猜）
    #[test]
    fn owner_falls_back_to_the_source_record() {
        let c = catalog();
        let body = "[toolhead]\noffset_x = -1\n";
        let text = runtime::provenance::commit_text(body, "delivery/mkp/presets/A1-fast.toml");
        let source = runtime::provenance::parse(&text);
        let (m, v) = owner_of(&c, &text, source.as_ref());
        assert_eq!(m.as_deref(), Some("A1"));
        assert_eq!(v.as_deref(), Some("FAST"));
    }

    /// 云端那一份按**来源记录里那一份**认（`A1-standard` 不该被这份自己的归属带偏）
    #[test]
    fn the_official_file_comes_from_the_source_record() {
        let c = catalog();
        let body = "# machine: A1\n# variant: fast\n[toolhead]\noffset_x = -1\n";
        let text = runtime::provenance::commit_text(body, "delivery/mkp/presets/A1-standard.toml");
        let source = runtime::provenance::parse(&text);
        let f = cloud_file_of(&c, &text, source.as_ref()).expect("该认出来源那一份");
        assert_eq!(f.file_name, "A1-standard.toml");
    }

    /// 没有来源记录：退回这份自己的归属 —— 于是导入的云端原件也认得自己对应哪一版
    #[test]
    fn without_a_source_record_the_owner_decides() {
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
            origins: &BTreeMap::new(),
        });

        assert_eq!(pending, 1, "只有那一项该进待处理");
        let e = entries.iter().find(|e| e.key == key).unwrap();
        assert!(e.pending);
        assert_eq!(e.local.as_deref(), Some("7.5"), "本地值不许被顶掉");
        assert_eq!(e.cloud_new.as_deref(), Some(changed.as_str()));
    }

    /* ---------- 「上次云端值」的滚动（2026-10-31） ---------- */

    /// 摆一条只用来测滚动的总账：这一项旧云端 `old`、新云端 `new`、记录 `origin`。
    fn sync_book(old: &str, new: &str, origin: Option<&str>) -> PresetParamSyncDto {
        PresetParamSyncDto {
            path: "presets-local/A1.toml".to_owned(),
            file_name: "A1.toml".to_owned(),
            machine_id: None,
            version_id: None,
            cloud_file_name: None,
            based_on_release_time: None,
            current_release_time: None,
            current_sha256: Some("sha-b".to_owned()),
            cloud_ready: true,
            version_advanced: true,
            pending_count: 1,
            entries: vec![ParamSyncEntryDto {
                key: "toolhead.offset.x".to_owned(),
                local: Some("-2".to_owned()),
                baseline_old: Some(old.to_owned()),
                cloud_new: Some(new.to_owned()),
                pending: true,
                conflict: true,
                decided: None,
                origin: origin.map(str::to_owned),
            }],
        }
    }

    /// 云端把这一项改了 ⇒ 记录滚成**被换下来的那一版**（不是"我改的值"）
    #[test]
    fn rolling_takes_the_version_being_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cloud = BTreeMap::new();
        cloud.insert("toolhead.offset.x".to_owned(), "-0.9".to_owned());

        roll_origins(
            tmp.path(),
            "presets-local/A1.toml",
            &sync_book("-0.5", "-0.9", Some("-1")),
            Some("sha-a"),
            &cloud,
        )
        .unwrap();

        let got =
            crate::runtime::app_state::param_origins(tmp.path(), "presets-local/A1.toml").unwrap();
        assert_eq!(
            got["toolhead.offset.x"].value, "-0.5",
            "滚的是旧云端那一侧，不是用户改的值，也不是新值"
        );
        assert_eq!(
            got["toolhead.offset.x"].sha256.as_deref(),
            Some("sha-a"),
            "摘要跟着记下来（排查用）"
        );
    }

    /// 这一项云端**没改** ⇒ 一个字不写（用户手改时记下的那版必须留着）
    #[test]
    fn rolling_leaves_unchanged_params_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cloud = BTreeMap::new();
        cloud.insert("toolhead.offset.x".to_owned(), "-0.9".to_owned());

        roll_origins(
            tmp.path(),
            "presets-local/A1.toml",
            &sync_book("-0.9", "-0.9", Some("-1")),
            Some("sha-a"),
            &cloud,
        )
        .unwrap();

        let got =
            crate::runtime::app_state::param_origins(tmp.path(), "presets-local/A1.toml").unwrap();
        assert!(
            got.is_empty(),
            "没有差异就不该动记录（它还是用户改值那一刻记下的 -1）"
        );
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
            /* 这一份目录代表"云端发布出来的那一版"，所以带上发布时刻 ——
            「云端当前版」那一格读的就是它（2026-11-04 起）。 */
            let mut catalog = catalog_with_shas();
            catalog.published_at = Some("2026-11-01T00:00:00Z".to_owned());
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
            runtime::content::ensure(&root, &current_sha, current_text.as_bytes()).unwrap();

            /* 我这份当初基于的那一版：云端当前版把这一项改成别的（同一个保真写口） */
            let based_text =
                param_alg::apply_param_edits(&current_text, &defs, &[FieldEdit::new(KEY, "-2")])
                    .unwrap();
            let based_sha = runtime::provenance::sha256_hex(&based_text);
            runtime::content::ensure(&root, &based_sha, based_text.as_bytes()).unwrap();

            /*
             * ★ 我那份的来源身份**完整**（平台 / 仓库 / 分支 / 发布线 / 序号 / 版本时间 /
             * 摘要）：真实下载会在那一刻把这些写进文件尾部。只写一半时判定必然
             * 走「说不清」—— 那正是这条判据要覆盖的另一档（见缺失身份那条测试）。
             */
            let version = preset::model::SourceRef {
                file: Some("delivery/mkp/presets/A1-fast.toml".to_owned()),
                platform: Some("github".to_owned()),
                repo: Some("mucoreben/mkpsupportease".to_owned()),
                branch: Some("main".to_owned()),
                at: Some("2026-10-31T00:00:00Z".to_owned()),
                sha256: Some(based_sha.clone()),
                catalog: Some("support-ease".to_owned()),
                serial: Some(1),
                revision: Some("rev-1".to_owned()),
                ..preset::model::SourceRef::default()
            };
            let copy = runtime::provenance::copy_from_cloud(
                &based_text,
                preset::model::SourceRef {
                    kind: Some("download".to_owned()),
                    ..version.clone()
                },
                version,
            );
            /* 我那份 = 基于那一版 + 我自己把它改成 -5（场景 B：改过的值不许被顶掉） */
            let local =
                param_alg::apply_param_edits(&copy, &defs, &[FieldEdit::new(KEY, "-5")]).unwrap();
            crate::fsx::atomic::atomic_write(&user.join(LOCAL), local.as_bytes()).unwrap();

            /* 真实下载会在 `run/provenance.json` 记一笔「下载那一刻目录的 publishedAt」。
            测试也走这本真账 —— 「基于哪一版」不能只靠文件头（新产物没有 release_time）。 */
            std::fs::create_dir_all(root.join("run")).unwrap();
            let identity = crate::runtime::source_identity::SourceIdentity {
                platform: "github".to_owned(),
                repo: "mucoreben/mkpsupportease".to_owned(),
                git_ref: "main".to_owned(),
            };
            let stamp = crate::runtime::source_ledger::CatalogStamp {
                catalog_id: Some("support-ease"),
                serial: Some(1),
                revision: "rev-1",
                published_at: Some("2026-10-31T00:00:00Z"),
            };
            crate::runtime::source_ledger::record_download(
                &root,
                LOCAL,
                Some(&identity),
                "A1-fast.toml",
                &stamp,
                Some(based_sha.as_str()),
            )
            .unwrap();

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
            let sha = runtime::provenance::sha256_hex(&text);
            runtime::content::ensure(&self.root, &sha, text.as_bytes()).unwrap();
            let mut catalog = catalog_with_shas();
            /* 模拟一次**发布**：发布事件会盖 `publishedAt`（生成不会）。
            来源记录里"基于哪一版"现在读的就是它（2026-11-01 起不再读文件头那个渲染时刻）。 */
            catalog.published_at = Some("2026-11-02T00:00:00Z".to_owned());
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
        assert_eq!(
            report.based_on_release_time.as_deref(),
            Some("2026-10-31T00:00:00Z"),
            "「基于哪一版」取来路账里下载那一刻的目录发布时刻（不是写盘记录）"
        );
        assert_eq!(
            report.current_release_time.as_deref(),
            Some("2026-11-01T00:00:00Z"),
            "「云端当前版」取当前目录的 publishedAt —— 与「基于哪一版」是两个版本各自的时间"
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
    /// 动的只有注释 / 时间戳 / 新字段 / 来源记录，**值**必须还是我原来那个。）
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

    /// **应用 0 项**（云端只改了注释 / 时间戳）：新注释与新的来源记录进来、
    /// 我的值一个都不动 —— 「有新版」就此收场
    #[test]
    fn applying_zero_items_digests_a_comment_only_release() {
        let w = World::new();
        /* 先把那一项处理掉（保持我的值）—— 于是"云端换版"只剩注释那点差别 */
        assert_eq!(w.decide(KEY, "hold").pending_count, 0);
        /* 值一项都没有变，只多了一行注释 —— 摘要照样变（来源记录比的是全文） */
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
        let source = runtime::provenance::parse(&local).expect("来源记录还在");
        assert_eq!(
            source
                .latest_version()
                .and_then(|v| v.sha256.as_deref())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            after.current_sha256.as_deref(),
            "来源版本推到云端当前版 —— 这就是「有新版」消失的根据"
        );
        assert_eq!(
            source.history.len(),
            1,
            "接整版 = 一条履历（旧条目一个字不改）"
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
            "旧值 = 上次处理到的那一版（不是我最初来源记录里那一版）"
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
        let sha = runtime::provenance::sha256_hex(&next);
        runtime::content::ensure(&w.root, &sha, next.as_bytes()).unwrap();
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
            "它的旧值仍取来源记录里那一版（没被第一项的水位带跑）"
        );
    }

    /// 来源记录里那份**不在目录里**时退回这份自己的归属 —— 仍然认得出对应哪一版云端。
    ///
    /// 这条测试只抹掉「来源指向哪一份」，所以能断言的只有**认得出云端那一份**。
    ///
    /// 为什么这里 `pending_count == 0`：逐项旧值取自基线那一版的正文
    /// （`based_on_sha256` 指向的那一版）。在这条夹具里来源记录被换成了一个
    /// 只有文件名、没有摘要的写法，基线就取不到 ⇒ 每一项都「比不出来」，
    /// 少了的是**基线**，不是归属。**不许**为了让这条变绿去拿写盘时间 / 目录时间补基线。
    #[test]
    fn a_missing_source_file_falls_back_to_the_own_attribution() {
        let w = World::new();
        let text = runtime::provenance::commit_text(&w.based_text, "mkp/presets/不存在的预设.toml");
        crate::fsx::atomic::atomic_write(&w.user.join(LOCAL), text.as_bytes()).unwrap();

        let report = w.sync();
        assert_eq!(
            report.cloud_file_name.as_deref(),
            Some("A1-fast.toml"),
            "来源那一份不在目录里 ⇒ 退回归属，仍然认得出云端对应哪一份"
        );
        assert!(
            report.cloud_ready,
            "云端当前版的正文在本机（目录里登记的那一份）"
        );
        assert_eq!(
            report.pending_count, 0,
            "没有基线那一版 ⇒ 比不出来就不报（不猜旧值）"
        );
    }

    /// 连归属都认不出（头里的机型不在目录里、也没有来源记录）⇒ 一项都不报，也不报错
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
        std::fs::remove_file(runtime::content::content_path(&w.root, &w.current_sha)).unwrap();
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
        let clean = runtime::provenance::strip_for_compare(&w.local_text());
        let subject = runtime::state::DraftSubject::local("本地 A1.toml", LOCAL);
        runtime::app_state::set_draft(&w.root, &subject, "旧水位", &clean).unwrap();

        /* 「应用 0 项」：值一个不动，但骨架与来源记录换成云端当前版 */
        apply_at(&w.root, &w.user, LOCAL, &[]).unwrap();

        let after = runtime::app_state::draft(&w.root)
            .unwrap()
            .expect("草稿该还在");
        assert_eq!(
            after.text,
            runtime::provenance::strip_for_compare(&w.local_text()),
            "草稿正文要跟着新文件走"
        );
    }

    /// **真改过的草稿不许被踩**：这一趟如实拒绝，文件一个字节不动 ——
    /// 悄悄换掉就成了"用户的字 + 云端新版"揉出来的中间态。
    #[test]
    fn a_dirty_draft_blocks_the_rewrite() {
        let w = World::new();
        let before = w.local_text();
        let mut dirty = runtime::provenance::strip_for_compare(&before);
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
