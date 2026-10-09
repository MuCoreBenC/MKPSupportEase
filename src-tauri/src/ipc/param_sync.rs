//! **逐参数「官方更新」的两条命令**：读三方账 / 把采用·保持的决定落下去。
//!
//! # 它兑现的产品规则
//!
//! 用户世界里只有**一份自己的预设**（`presets-mine/…`）；官方那一版默认值住在
//! 隐藏 baseline 里（[`crate::runtime::baseline`]，用户看不见它）。于是：
//!
//! ```text
//! 读（get_preset_param_sync）    我这份文件 / 官方旧值（baseline） / 官方当前值（目录那一版）
//! 写（apply_preset_param_decisions）
//!     采用(adopt)  把我那份文件里这一项写成官方新值 + 水位推到当前版
//!     保持(hold)   我那份文件一个字不动             + 水位推到当前版
//! ```
//!
//! 「水位」= [`crate::runtime::app_state`] 里的逐参数决定账。它是
//! 「这一项我处理到哪一版了」的唯一记录 —— 官方连着发几版也不需要在本地存历史
//! （比对的是「我处理到的那一版 → 当前版」，见 [`crate::runtime::param_sync`]）。
//!
//! # 边界（与 [`super::preset_params`] 同一条）
//!
//! - **只认 `presets-mine/`**（[`crate::runtime::mine`] 的前缀 + 防穿越两道闸）：
//!   baseline 与官方当前版**永远不被写**；
//! - **结构保真**：采用走 [`crate::presetdata::params::apply_param_edits`] ——
//!   注释 / 键序 / 多行字面量 / 行尾一个字节不动；任何一项失败**整批不落**；
//! - **不碰草稿、不碰使用中指针**：这一层只管"这一项写什么"与"水位推到哪一版"。
//!
//! # 官方当前版的字节：只在本机找，除非明确要求取回来
//!
//! `fetch_missing` 是调用方给的开关。界面在**打开某一份预设时**可以传 `true`
//! （"我愿意等一下，把官方新版取回来看个清楚"），列表页那种一屏几十行的读一律
//! `false` —— 铁律是**启动零网络**，读命令不许偷偷发请求。

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
    pub mine: Option<String>,
    /// 官方旧值（我上次处理到的那一版）
    pub baseline_old: Option<String>,
    /// 官方当前最新值
    pub official_new: Option<String>,
    /// 官方改过这一项、而我还没处理
    pub pending: bool,
    /// 我已经对当前官方版处理过这一项：`adopt` / `hold`
    pub decided: Option<&'static str>,
}

/// 一份用户预设的「官方更新」总账。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetParamSyncDto {
    /// 相对用户根的路径（`presets-mine/A1-fast.toml`）
    pub path: String,
    pub file_name: String,
    /// 这份预设自己的归属（文件头 `# machine:` / `# variant:` 归一化后的；认不出是 `null`）
    pub machine_id: Option<String>,
    pub version_id: Option<String>,
    /// 官方那一份的文件名（`A1-fast.toml`）。认不出 ⇒ `null`（那就比不出官方更新）
    pub official_file_name: Option<String>,
    /// 官方当前版的正文摘要（**界面不显示**，它只是"是哪一版"的身份）
    pub current_sha256: Option<String>,
    /// 我这份当初基于的那一版官方（血统里的 `based_on_release_time`，给人看的）
    pub based_on_release_time: Option<String>,
    /// 我这份当初基于的那一版官方的 `# release_time`
    pub current_release_time: Option<String>,
    /// 官方当前版的字节**在本机可用**（读得到就能逐项比）
    pub official_ready: bool,
    /// 官方换版了：当前版与我这份血统里那一版不是同一份
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
    /// `adopt` = 采用官方新值；`hold` = 保持我的值
    pub kind: String,
}

/// 某一份用户预设现在是什么情况：逐项三方账 + 待处理条数。
#[tauri::command]
pub async fn get_preset_param_sync(
    app: AppHandle,
    path: String,
    fetch_missing: Option<bool>,
) -> Result<PresetParamSyncDto, AppError> {
    let handle = app.clone();
    let fetch = fetch_missing.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        traced("getPresetParamSync", |_| {
            let root = internal_root(&handle)?;
            let user = user_root(&handle)?;
            sync_at(&root, &user, &path, fetch)
        })
    })
    .await
    .map_err(|e| AppError::internal("读官方更新账没跑到终局").with_detail(e.to_string()))?
}

/// 把一批决定落下去（采用 / 保持），回来的是**落完之后的**那一份总账。
///
/// 采用要动文件，所以官方当前版必须在本机 —— 不在就如实报错（让界面先去取回来），
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
    .map_err(|e| AppError::internal("落官方更新决定没跑到终局").with_detail(e.to_string()))?
}

/* ---------- 读 ---------- */

/// 读一份用户预设的三方账。`fetch` = 允许为了拿到官方当前版的字节发一次网络请求。
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

    let text = runtime::mine::read_text(user, path)?;
    let lineage = runtime::lineage::parse_lineage_from_content(&text);
    /* 读得出来就顺带把"这份当初基于哪一版"的日期给人看（读不出来就是不写，不编） */
    let based_on_release_time = lineage.as_ref().and_then(|l| l.based_on_release_time.clone());
    let based_on_sha256 = lineage
        .as_ref()
        .and_then(|l| l.based_on_sha256.as_deref())
        .map(str::to_ascii_lowercase);
    let based_on_sha256 = based_on_sha256.as_deref();

    let file_name = path.rsplit('/').next().unwrap_or(path).to_owned();
    let official = official_file_of(&catalog, &text, lineage.as_ref());

    let (owner_machine, owner_version) = owner_of(&catalog, &text, lineage.as_ref());

    /* 官方当前版的正文 + 它的身份摘要（拿不到就是 None：界面照实说"还没取回来"） */
    let (official_text, current_sha256) = match official {
        Some(f) => {
            let text = official_text_for(root, f, fetch)?;
            let sha = match f.expected_sha() {
                Some(want) => Some(want.to_ascii_lowercase()),
                None => text.as_deref().map(|t| hex(&Sha256::digest(t.as_bytes()))),
            };
            (text, sha)
        }
        None => (None, None),
    };
    let current_release_time = official_text
        .as_deref()
        .and_then(runtime::lineage::parse_release_time);

    let defs = &catalog.registry.params;
    let decisions = decisions_of(root, path)?;

    /* 要比的那几版：当前版（可能刚取回来）+ 逐项水位 / 血统指的那些 */
    let mut by_sha: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    if let (Some(sha), Some(text)) = (current_sha256.as_deref(), official_text.as_deref()) {
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

    let mine = param_alg::read_param_values(&text, defs)?;
    let (entries, pending_count) = param_sync::diff(&SyncInput {
        defs,
        mine: &mine,
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
        official_file_name: official.map(|f| f.file_name.clone()),
        version_advanced: matches!(
            (based_on_sha256, current_sha256.as_deref()),
            (Some(b), Some(c)) if b != c
        ),
        current_sha256,
        based_on_release_time,
        current_release_time,
        official_ready: official_text.is_some(),
        pending_count,
        entries: entries
            .into_iter()
            .map(|e| ParamSyncEntryDto {
                key: e.key,
                mine: e.mine,
                baseline_old: e.official_old,
                official_new: e.official_new,
                pending: e.pending,
                decided: e.decided.map(kind_text),
            })
            .collect(),
    })
}

/// 这一份的逐参数决定账。路径先过用户线那两道闸 —— 状态账的键也必须是这一格里的。
fn decisions_of(
    root: &std::path::Path,
    path: &str,
) -> Result<BTreeMap<String, ParamDecision>, AppError> {
    runtime::mine::check_mine_prefix(path)?;
    runtime::app_state::param_decisions(root, path)
}

/// 官方那一份预设是谁：血统里的 `based_on` 优先（那是**当初**的来源），
/// 认不出就退回这份文件自己的归属（`# machine:` / `# variant:`）。
fn official_file_of<'a>(
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

/// 官方当前版的正文，**只在本机找**（除非 `fetch` 明确要求取回）。
///
/// 找法有先后，两个落点都信得过：
///   ① 盘上那份（`<appDataDir>/<catalog.path>`，下载区，字节与目录登记对得上才算）；
///   ② 基准区里那一份（`baseline/<sha>.toml`，下载管道落的）。
///
/// 都没有 ⇒ `Ok(None)`：**这不是错误**（还没取回官方新版），界面照实说。
fn official_text_for(
    root: &std::path::Path,
    file: &CatalogFile,
    fetch: bool,
) -> Result<Option<String>, AppError> {
    if let Some(text) = local_official_text(root, file) {
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
     * 表现成 `officialReady: false` + `versionAdvanced: true`（界面说"官方有新版本，
     * 还没取回来"）—— 比整页报错有用得多。
     */
    if let Err(e) = super::catalog::ensure_official_bytes(root, file) {
        tracing::warn!(file = %file.file_name, detail = %e.message, "官方当前版没取回来（照旧比不了）");
        return Ok(None);
    }
    if let Some(text) = local_official_text(root, file) {
        return Ok(Some(text));
    }
    Ok(file
        .expected_sha()
        .map(|w| runtime::baseline::read_baseline(root, w))
        .transpose()?
        .flatten())
}

/// 盘上那份官方正文（下载区）。字节与目录登记对不上（旧版本 / 被改过）⇒ `None`。
///
/// 顺手把它落进基准区（幂等）—— 于是"这份官方当前版"以后离线也读得到，
/// 用户不用为了看一次官方更新而再连一次网。
fn local_official_text(root: &std::path::Path, file: &CatalogFile) -> Option<String> {
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

/// 落一批决定：采用的那几项写进文件（结构保真），保持的一个字节不动；
/// 所有项的水位都推到官方当前版。
fn apply_at(
    root: &std::path::Path,
    user: &std::path::Path,
    path: &str,
    decisions: &[ParamDecisionDto],
) -> Result<PresetParamSyncDto, AppError> {
    runtime::mine::check_mine_prefix(path)?;
    if decisions.is_empty() {
        return sync_at(root, user, path, false);
    }

    /* 先读一份总账：采用需要"官方新值"，水位需要"官方当前版是谁" —— 两者都在里面 */
    let before = sync_at(root, user, path, false)?;
    let Some(current_sha) = before.current_sha256.clone() else {
        return Err(AppError::invalid_argument(
            "认不出这份预设对应哪一版官方 —— 现在没法处理官方更新",
        ));
    };

    let mut edits: Vec<FieldEdit> = Vec::new();
    let mut record: Vec<(String, ParamDecision)> = Vec::new();
    for d in decisions {
        let kind = match d.kind.as_str() {
            "adopt" => ParamDecisionKind::Adopt,
            "hold" => ParamDecisionKind::Hold,
            other => {
                return Err(AppError::invalid_argument(format!(
                    "不认得这种处理方式：{other}（只有 adopt / hold）"
                )))
            }
        };
        if kind == ParamDecisionKind::Adopt {
            let entry = before
                .entries
                .iter()
                .find(|e| e.key == d.param_key)
                .ok_or_else(|| {
                    AppError::invalid_argument(format!("这份预设里没有参数 {}", d.param_key))
                })?;
            let new = entry.official_new.clone().ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "官方当前版的正文还不在本机 —— 先把官方新版取回来，再采用 {} 的新值",
                    d.param_key
                ))
            })?;
            edits.push(FieldEdit::new(d.param_key.clone(), new));
        }
        record.push((
            d.param_key.clone(),
            ParamDecision {
                sha256: current_sha.clone(),
                kind,
            },
        ));
    }

    if !edits.is_empty() {
        let text = runtime::mine::read_text(user, path)?;
        let catalog = runtime::load_released_catalog(root)?;
        let next = param_alg::apply_param_edits(&text, &catalog.registry.params, &edits)?;
        /*
         * 算出来与原文一样就**不写盘**：一次空操作不该刷新 mtime
         * （与 [`super::preset_params::save_preset_params`] 同一条规矩）。
         */
        if next != text {
            runtime::mine::write_text(user, path, &next)?;
        }
    }

    runtime::app_state::record_param_decisions(root, path, &record)?;

    sync_at(root, user, path, false)
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
    /// 而"官方当前版是谁"这件事在逐参数更新里必须知道 —— 所以判据里现算一遍写进去。
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

    /// 官方那一份按**血统里的来源**认（`A1-standard` 那份不该被这份自己的归属带偏）
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
        let f = official_file_of(&c, text, lineage.as_ref()).expect("该认出来源那一份");
        assert_eq!(f.file_name, "A1-standard.toml");
    }

    /// 没有血统：退回这份自己的归属 —— 于是导入的官方原件也认得自己对应哪一版
    #[test]
    fn without_lineage_the_owner_decides() {
        let c = catalog();
        let text = "# machine: A1\n# variant: fast\n[toolhead]\noffset_x = -1\n";
        let f = official_file_of(&c, text, None).expect("该按归属认出来");
        assert_eq!(f.file_name, "A1-fast.toml");
    }

    /// 一份真目录 + 一份真预设正文：三方账算得出来，且「我改过的那一项」没被官方顶掉
    #[test]
    fn a_sync_report_finds_the_changed_param() {
        let defs = defs();
        let official = std::fs::read_to_string(
            repo_root().join("presets/delivery/mkp/presets/A1-fast.toml"),
        )
        .expect("读官方交付件");

        let mut new_values = param_alg::read_param_values(&official, &defs).unwrap();
        let key = "toolhead.offset.x";
        let old_value = new_values.get(key).cloned().expect("这一项该有值");
        /* 官方改了一项（模拟新版），我的那份写的是自己的值 */
        let changed = format!("{old_value}9");
        new_values.insert(key.to_owned(), changed.clone());

        let mut old_map = BTreeMap::new();
        old_map.insert(key.to_owned(), old_value);
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), old_map);
        by_sha.insert("new".to_owned(), new_values);
        let mut mine = BTreeMap::new();
        mine.insert(key.to_owned(), "7.5".to_owned());

        let (entries, pending) = param_sync::diff(&SyncInput {
            defs: &defs,
            mine: &mine,
            decisions: &BTreeMap::new(),
            based_on_sha256: Some("old"),
            current_sha256: Some("new"),
            by_sha: &by_sha,
        });

        assert_eq!(pending, 1, "只有那一项该进待处理");
        let e = entries.iter().find(|e| e.key == key).unwrap();
        assert!(e.pending);
        assert_eq!(e.mine.as_deref(), Some("7.5"), "我的值不许被顶掉");
        assert_eq!(e.official_new.as_deref(), Some(changed.as_str()));
    }

    /* ---------- 端到端：摆一个真世界，走一遍场景 A–E ---------- */

    /// 「我改的那一项」—— 用它当整条链上的那一个参数
    const KEY: &str = "toolhead.offset.x";
    /// 演示用的用户文件名
    const MINE: &str = "presets-mine/我的 A1.toml";

    /// 一个摆得出来的世界：真目录（真注册表 + 9 份交付件的清单）+ 基准区 + 我那一份。
    ///
    /// 官方"当前版"和"我这份当初基于的那一版"都从**仓库里那份真交付件**派生
    /// （后者用同一个保真写口改一项）—— 不是手编两段 TOML。
    struct World {
        /// 名字带下划线是因为它**只用来占住这个临时目录**（删了就没了）
        _dir: tempfile::TempDir,
        root: std::path::PathBuf,
        user: std::path::PathBuf,
        /// 官方当前版的正文与摘要
        current_text: String,
        current_sha: String,
        /// 我这份当初基于的那一版（= 官方当前版改过 `KEY` 的那一份）
        based_text: String,
    }

    impl World {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            let user = root.join("user");
            std::fs::create_dir_all(user.join("presets-mine")).unwrap();

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
            .expect("读官方交付件");
            runtime::baseline::ensure_baseline(&root, &current_sha, current_text.as_bytes())
                .unwrap();

            /* 我这份当初基于的那一版：官方当前版把这一项改成别的（同一个保真写口） */
            let based_text = param_alg::apply_param_edits(
                &current_text,
                &defs,
                &[FieldEdit::new(KEY, "-2")],
            )
            .unwrap();
            let based_sha = runtime::lineage::sha256_hex(&based_text);
            runtime::baseline::ensure_baseline(&root, &based_sha, based_text.as_bytes()).unwrap();

            /* 我那份 = 基于那一版 + 我自己把它改成 -5（场景 B：改过的值不许被顶掉） */
            let copy = runtime::lineage::make_copy(&based_text, "delivery/mkp/presets/A1-fast.toml");
            let mine = param_alg::apply_param_edits(&copy, &defs, &[FieldEdit::new(KEY, "-5")])
                .unwrap();
            crate::fsx::atomic::atomic_write(&user.join(MINE), mine.as_bytes()).unwrap();

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
            sync_at(&self.root, &self.user, MINE, false).unwrap()
        }

        fn at(&self, key: &str) -> ParamSyncEntryDto {
            self.sync()
                .entries
                .into_iter()
                .find(|e| e.key == key)
                .expect("这一项该在结果里")
        }

        fn mine_text(&self) -> String {
            runtime::mine::read_text(&self.user, MINE).unwrap()
        }

        fn decide(&self, key: &str, kind: &str) -> PresetParamSyncDto {
            apply_at(
                &self.root,
                &self.user,
                MINE,
                &[ParamDecisionDto {
                    param_key: key.to_owned(),
                    kind: kind.to_owned(),
                }],
            )
            .unwrap()
        }

        /// 官方再发一版：把 `KEY` 改成 `next`，并把目录里的摘要推进过去
        fn publish(&self, next: &str) {
            let defs = defs();
            let text =
                param_alg::apply_param_edits(&self.current_text, &defs, &[FieldEdit::new(KEY, next)])
                    .unwrap();
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

    /// 场景 A/B：官方改过的那一项进待处理；**我改过的值（-5）一个字没动**
    #[test]
    fn scenario_a_and_b_an_update_shows_up_without_touching_my_value() {
        let w = World::new();
        let report = w.sync();
        assert_eq!(report.pending_count, 1, "只有官方改过的那一项：{report:?}");
        assert!(report.version_advanced, "官方换版了");
        assert_eq!(report.official_file_name.as_deref(), Some("A1-fast.toml"));
        assert_eq!(report.machine_id.as_deref(), Some("A1"));
        assert_eq!(report.version_id.as_deref(), Some("FAST"));
        assert!(report.based_on_release_time.is_some(), "血统里那行日期该读得到");

        let e = w.at(KEY);
        assert!(e.pending);
        assert_eq!(e.mine.as_deref(), Some("-5"), "我的值不许被顶掉");
        assert_eq!(e.baseline_old.as_deref(), Some("-2"), "官方旧值 = 我处理到的那一版");
        assert_eq!(e.official_new.as_deref(), Some("-0.7"));
        assert_eq!(e.decided, None, "还没处理过");
    }

    /// 场景 D：采用 = 文件里那一项写成官方新值 + 水位推到当前版；之后不再挂着
    #[test]
    fn scenario_d_adopting_writes_the_new_value_and_retires_the_flag() {
        let w = World::new();
        let after = w.decide(KEY, "adopt");
        assert_eq!(after.pending_count, 0, "处理过就不该再挂着");

        let e = after.entries.iter().find(|e| e.key == KEY).unwrap();
        assert_eq!(e.decided.as_deref(), Some("adopt"));
        assert_eq!(e.mine.as_deref(), Some("-0.7"), "文件里那一项该是官方新值");

        /* 文件真的被改了，而且**只改了那一行**（保真：注释 / 别的行一个字不动） */
        let text = w.mine_text();
        assert!(text.contains("offset_x = -0.7"), "\n{text}");
        assert!(text.contains("# 笔尖偏移"), "行尾注释要留着\n{text}");
        assert_eq!(
            param_alg::read_param_values(&text, &defs()).unwrap()[KEY],
            "-0.7"
        );
    }

    /// 场景 C：保持 = 文件一个字不动 + 水位推到当前版；之后同一版不再待处理
    #[test]
    fn scenario_c_holding_keeps_the_file_byte_identical() {
        let w = World::new();
        let before = w.mine_text();
        let after = w.decide(KEY, "hold");

        assert_eq!(after.pending_count, 0);
        assert_eq!(
            after.entries.iter().find(|e| e.key == KEY).unwrap().decided.as_deref(),
            Some("hold")
        );
        assert_eq!(w.mine_text(), before, "保持 = 一个字节都不动");
    }

    /// 场景 C（续）/ E：官方又发一版 —— 旧值取「我处理到的那一版」，我的值还是我保持的那个
    #[test]
    fn scenario_e_the_next_release_compares_against_my_watermark() {
        let w = World::new();
        assert_eq!(w.decide(KEY, "hold").pending_count, 0, "先处理掉这一版");

        /* 官方再改一次这一项 */
        w.publish("-9");

        let report = w.sync();
        assert_eq!(report.pending_count, 1, "官方又改了这一项");
        let e = report.entries.iter().find(|e| e.key == KEY).unwrap();
        assert!(e.pending);
        assert_eq!(
            e.baseline_old.as_deref(),
            Some("-0.7"),
            "旧值 = 上次处理到的那一版（不是我最初血统里那一版）"
        );
        assert_eq!(e.official_new.as_deref(), Some("-9"));
        assert_eq!(
            e.mine.as_deref(),
            Some("-5"),
            "我的值还是我保持下的那个（= 我那份文件里写着的）"
        );
    }

    /// 水位是**逐项**的：同一份里一项处理过、另一项没处理，两边各自算各自的
    #[test]
    fn the_watermark_is_per_param() {
        let w = World::new();
        let defs = defs();
        /* 官方再改第二项：于是这一份上有两项待处理 */
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
        let second = after.entries.iter().find(|e| e.key == "toolhead.offset.y").unwrap();
        assert!(second.pending);
        assert_eq!(second.decided, None, "它没被处理过");
        assert_eq!(
            second.baseline_old.as_deref(),
            Some("26.3"),
            "它的旧值仍取血统里那一版（没被第一项的水位带跑）"
        );
    }

    /// 血统里的来源名字**不在目录里**时退回这份自己的归属 —— 照样认得出对应哪一版官方
    #[test]
    fn a_missing_lineage_source_falls_back_to_the_own_attribution() {
        let w = World::new();
        let text = runtime::lineage::make_copy(&w.based_text, "mkp/presets/不存在的预设.toml");
        crate::fsx::atomic::atomic_write(&w.user.join(MINE), text.as_bytes()).unwrap();

        let report = w.sync();
        assert_eq!(report.official_file_name.as_deref(), Some("A1-fast.toml"));
        assert_eq!(report.pending_count, 1, "退回归属之后照样算得出更新");
    }

    /// 连归属都认不出（头里的机型不在目录里、也没有血统）⇒ 一项都不报，也不报错
    #[test]
    fn a_preset_that_maps_to_no_official_file_is_not_an_error() {
        let w = World::new();
        let text = w.based_text.replace("# machine: A1", "# machine: 不存在的机型");
        crate::fsx::atomic::atomic_write(&w.user.join(MINE), text.as_bytes()).unwrap();

        let report = w.sync();
        assert_eq!(report.pending_count, 0);
        assert!(report.official_file_name.is_none());
        assert!(!report.official_ready);
    }

    /// 采用一个**官方新版还没取回来**的项：如实报错，不许"记了水位、值却没写"
    #[test]
    fn adopting_before_the_official_bytes_are_here_is_an_error() {
        let w = World::new();
        /* 把官方当前版的基准抽掉（模拟"这一版还没取回来"） */
        std::fs::remove_file(runtime::baseline::baseline_path(&w.root, &w.current_sha)).unwrap();
        let report = w.sync();
        assert!(!report.official_ready);
        assert_eq!(report.pending_count, 0, "拿不到新值就不许报待处理");

        let err = apply_at(
            &w.root,
            &w.user,
            MINE,
            &[ParamDecisionDto {
                param_key: KEY.to_owned(),
                kind: "adopt".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(runtime::app_state::param_decisions(&w.root, MINE).unwrap().is_empty(), "一项水位都不许留下");
    }

    /// 认不出的处理方式 / 不存在的参数：都拒绝（不让界面写出第三种决定来）
    #[test]
    fn an_unknown_decision_or_param_is_refused() {
        let w = World::new();
        let bad_kind = apply_at(
            &w.root,
            &w.user,
            MINE,
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
            MINE,
            &[ParamDecisionDto {
                param_key: "没有.这个参数".to_owned(),
                kind: "adopt".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(bad_key.code, crate::error::ErrorCode::InvalidArgument);
    }
}
