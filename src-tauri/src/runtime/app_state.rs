//! **应用持久化状态（AppState）**——`run/app-state.json` 的唯一读写入口。
//!
//! # 这是 2026-10-06 的架构决策（作者裁决，见 `docs/APP-STATE.md`）
//!
//! 推翻 [`super::state`] 模块头那条「一种状态一个文件」：以前"使用中指针 / 编辑中草稿 /
//! 数据源设置"各占一个文件，读端却各自缓存、回页签对账 —— 文件只有一个真相，
//! 应用里到处是副本（A3 根治验收钉出来的）。现在**一个应用一个 AppState**：
//!
//! ```text
//! run/app-state.json          ← 「应用当前记住的全部持久化状态」（一个文件）
//!        ▲
//!        │ 读-改-写（进程内互斥串行，整份原子替换）
//!        │
//!    app_state.rs（唯一读写入口）—— 页面 / 命令不得另立状态文件、不得直读写状态文件
//!        │
//!        ├── activePreset   使用中指针（原 run/active-preset.json）
//!        ├── draftPreset    编辑中草稿（原 run/draft-preset.json）
//!        └── presetSource   数据源设置（原 run/preset-source.json）
//! ```
//!
//! # 规矩（从旧四条里继承三条，第一条被本决策取代）
//!
//! 1. ~~一种状态一个文件~~ → **一个应用一个状态文件**，里面一格一个状态；
//! 2. 住在内部根 `run/` 下，用户能看见、能单独删（删 `app-state.json` = 重置全部状态）；
//! 3. 写走 [`crate::fsx::atomic::atomic_write`] —— 全仓唯一写盘出口，没有例外；
//! 4. 坏档不静默 —— 读出来解析不了就报 `CORRUPTED`。静默吞状态是编数据的近亲。
//!
//! # 迁移（一次性、惰性，不设启动顺序）
//!
//! - **读**：`app-state.json` 在 → 只信它；不在 → 按格读三个旧文件拼快照
//!   （缺席 = None，坏档照旧响）；
//! - **写**：第一次写时若 `app-state.json` 不在，先把旧文件的内容并进来一起写下去，
//!   写成功后删掉旧文件 —— 从这一刻起旧文件退役。写失败什么都不动（含不删旧文件）。
//!
//! # 与内容的关系
//!
//! AppState 只存**指针与状态**，不存内容：`activePreset` 说的是"用哪一份"，
//! 预设正文仍在交付区 / `presets-local/`。事件账、出处账、下载文件、图片模型
//! 都不进这里（它们是业务数据 / 历史记录，各有各的落点）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;

use super::catalog::{hex, CatalogFile};
use super::source::PresetSource;
use super::state::{ActiveOrigin, ActivePreset, DraftSubject, PresetDraft, PresetTextDraft};

/// 一次「云端更新」的处理决定（逐参数）。
///
/// 用户世界里只有一份自己的预设；云端那一版默认值住在**隐藏 baseline** 里
/// （[`super::baseline`]，按摘要寻址）。于是"这一项我处理过了"这件事盘上不存在，
/// 只能记在这里 —— 它回答的是**程序当前怎么管理这一份**（不是"这个预设文件是什么"），
/// 所以照 [`super::lineage`] 定的那条界，它属于 `run/`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamDecisionKind {
    /// 采用了云端的新值（文件里那一项已经写成云端新值）
    Adopt,
    /// 明确保持本地值（文件里那一项一个字没动）
    Hold,
}

/// 一个参数上"我做过的那次决定"。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDecision {
    /// 做决定那一刻的云端版本（正文全文 sha256）。
    ///
    /// 它同时是**这一项"处理到哪一版了"的水位**：下次云端再改，云端当前版的摘要与它不同，
    /// 这一项就会重新进入待处理 —— 云端连着发几版也不需要在本地存历史。
    pub sha256: String,
    pub kind: ParamDecisionKind,
}

/// **界面主题档位**（2026-10-12 整套客户端做深色模式那一刀）。
///
/// 三档，与前端 `src/api/contract.ts` 的 `ThemePref` 一一对应 —— **id 是契约**
/// （盘上 / 界面 / 前端都认它，换名字要三处一起改）：
///
/// - `light` / `dark`：用户点名的那一档，界面照它切；
/// - `system`：跟随系统深浅 —— 程序**不猜**操作系统此刻是深是浅，那是界面的事
///   （webview 的 `prefers-color-scheme` 才是真值，这里只记"用户选了跟随"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    Light,
    Dark,
    System,
}

impl ThemePref {
    /// 线上字符串 → 枚举；认不出给 `None`（调用方如实报错，**不静默回落到浅色**）
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// 枚举 → 线上字符串。只写这一处（盘上 / DTO / 前端共用同一个字面量）
    pub fn wire(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
}

/// 应用状态格式的代次。加 section / 加字段不升号，改语义才升（与 catalog 同一条）。
///
/// 代次史：
/// - `1` —— 最初的整份状态（使用中指针 / 草稿 / 正文草稿 / 数据源 / 逐参数账 / 主题 / 自动清理）。
///   那一代里 `activePreset.origin` 写作 `"cloud"`（更早还写过 `"mine"`），
///   本地文件落点在 `presets-mine/…`。
/// - `2`（2026-10-14）—— 两条线的名字定成 `"preset"` / `"local"`，本地文件落点
///   `presets-local/…`。**盘上那份 1 代状态必须原地升上来**：读的时候按
///   [`migrate_state`] 转（指针与草稿里的旧枚举值、旧落点前缀），随后第一次写落盘
///   就是 2 代。不升号、只靠 serde 别名的话，指针里那个 `presets-mine/…` 路径会一直
///   留在盘上，而 [`super::local::check_local_prefix`] 只认新前缀 ⇒ 那份本地预设
///   在界面上从此不可见（作者 2026-10-14 实测到的那个状态）。
pub const APP_STATE_SCHEMA: u32 = 2;

/// 迁移前的旧本地预设目录前缀。**只有迁移读它**（`presets-mine/…` → `presets-local/…`）；
/// 业务代码一律走 [`super::paths::LOCAL_DIR`]，别在别处再写这个字面量。
const LEGACY_LOCAL_DIR: &str = "presets-mine";

const STATE_FILE: &str = "run/app-state.json";

/// **应用持久化状态**（整份）。每格 `Option` + `serde(default)`：
/// 没有就是"这一格还没状态"，合法；将来加新 section 时旧文件照读，不迁移、不升号。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateFile {
    pub app_state_schema: u32,
    /// 使用中指针：当前用的是哪一份预设
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_preset: Option<ActivePreset>,
    /// 编辑中的那一份（临时文件账：改的是谁、正文到哪了）—— **参数页**那条链
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<PresetDraft>,
    /// 「编辑文件」那条链的草稿（2026-10-10：与参数草稿**分开存**、互不覆盖）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_draft: Option<PresetTextDraft>,
    /// 数据源设置：当前用哪个远端
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_source: Option<PresetSource>,
    /// **逐参数「云端更新」处理账**：`presets-local/…` 相对路径 → `参数 key → 决定`。
    ///
    /// 空的格不写进文件（`skip_serializing_if`）—— 没处理过云端更新的应用，状态文件与以前一样。
    /// 删掉/改名一份用户文件时这本账跟着走（[`forget_param_decisions`] / [`repoint_local`]）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub param_decisions: BTreeMap<String, BTreeMap<String, ParamDecision>>,
    /// **界面主题偏好**（浅 / 深 / 跟随系统）。`None` = 还没选过 ⇒ 界面按"跟随系统"。
    ///
    /// 它是"应用当前记着的界面状态"，不是另一份偏好文件 —— 所以照
    /// [`super::lineage`] 那条界住在这里；前端也不另存副本（读口只有
    /// `src/app/state/appState.ts` 一处，与 activePreset 同一套广播）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemePref>,
    /// **后处理记录自动清理**（2026-10-13）：开着时按「条数 + 空间」双限静默清掉最旧的
    /// 执行记录（见 [`crate::retention`]）。
    ///
    /// `None` = **还没选过 ⇒ 开着**（出厂默认）。与主题那一格同一个读法（"没选过"
    /// 与"明确选了"要分得开），但默认值不同 —— 盘空间是稀缺的那一侧。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_clean_history: Option<bool>,
}

impl Default for AppStateFile {
    fn default() -> Self {
        Self {
            app_state_schema: APP_STATE_SCHEMA,
            active_preset: None,
            draft: None,
            text_draft: None,
            preset_source: None,
            param_decisions: BTreeMap::new(),
            theme: None,
            auto_clean_history: None,
        }
    }
}

fn state_file(root: &Path) -> PathBuf {
    root.join(STATE_FILE)
}

/// 迁移完成后要退役的旧文件（读端已不再看它们；写端首次写成功后顺手删掉）。
fn legacy_files(root: &Path) -> [PathBuf; 3] {
    [
        root.join(super::state::ACTIVE_FILE),
        root.join(super::state::DRAFT_FILE),
        root.join(super::paths::SOURCE_FILE),
    ]
}

/// 进程内互斥：AppState 的每次读-改-写都必须整份进行，
/// 两个并发写（比如连点两次「应用」）不许互相踩掉对方的格。
fn state_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 读整份状态（**唯一读入口**）。`app-state.json` 在 → 只信它；
/// 不在 → 按格读旧文件拼快照（缺席 = None）。**读不落盘**：没写过的应用不留状态文件。
pub fn load(root: &Path) -> Result<AppStateFile, AppError> {
    let _guard = state_lock();
    load_locked(root)
}

fn load_locked(root: &Path) -> Result<AppStateFile, AppError> {
    let bytes = match std::fs::read(state_file(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return legacy_snapshot(root);
        }
        Err(e) => return Err(AppError::io("读不到应用状态").with_detail(e.to_string())),
    };
    /*
     * **先探代次、低代次先迁、迁完再解析**：`1` 代文件里可能有**已经退役的枚举值**
     * （`"origin":"mine"`），直接 `from_slice` 会先失败 ⇒ 迁移就没机会跑。
     * 探不出代次的（坏 JSON / 没有 `appStateSchema`）交给下面如实报错。
     */
    let mut value = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|e| AppError::corrupted("应用状态读不出来").with_detail(e.to_string()))?;
    migrate_state(&mut value)?;
    serde_json::from_value(value)
        .map_err(|e| AppError::corrupted("应用状态读不出来").with_detail(e.to_string()))
}

/// **已知代次 → 当前代次**的升级（就地改 JSON 树，只在内存里转，不落盘）。
///
/// 三条判据（缺一条都会变成"要么静默丢数据、要么把新数据当旧数据改"）：
/// - **当前代次**：原样返回；
/// - **低代次（已知）**：按明确路径逐代升。到 2026-10-14 只有一条 `1 → 2`：
///   指针与草稿里的旧枚举值改成现名、本地线落点从 `presets-mine/…` 改成
///   `presets-local/…`（**纯重命名**：文件本体在盘上的位置不动，动的是记在哪）；
/// - **未来代次**：**拒绝**，如实报错。降级程序按旧结构去读新文件，越读越糟；
///   代次认不了就不动手，原文件一个字节不改。
fn migrate_state(value: &mut serde_json::Value) -> Result<(), AppError> {
    let Some(schema) = value
        .get("appStateSchema")
        .and_then(serde_json::Value::as_u64)
    else {
        return Err(AppError::corrupted(
            "应用状态里没有格式代次（appStateSchema）",
        ));
    };
    if schema > u64::from(APP_STATE_SCHEMA) {
        return Err(AppError::corrupted(format!(
            "应用状态的格式代次比程序新（文件是 {schema}，程序认到 {APP_STATE_SCHEMA}）—— \
             这一版程序读不了，换新版再打开"
        )));
    }
    if schema < 2 {
        migrate_v1_to_v2(value);
    }
    value["appStateSchema"] = serde_json::Value::from(APP_STATE_SCHEMA);
    Ok(())
}

/// `1 → 2`：本地线落点换代（`presets-mine/` → `presets-local/`），
/// **外加**把那一代里已经退役的枚举值（`"mine"`）改写成现名。
///
/// 为什么在 `serde_json::Value` 上做、而不是在结构体上做：`"mine"` 那条别名已经退役
/// （见 [`super::state::ActiveOrigin`]），**硬解析会先失败** ⇒ 结构体那条路根本走不到。
/// 先按 JSON 树把两件事做完（旧枚举值改现名 + 路径换前缀），再交给 serde 判定一次：
/// 改完还解析不了，那就是真坏档，照旧 `CORRUPTED`。
///
/// 只动**确实在旧前缀下**的那些路径、以及**确实写着旧枚举值**的那种字段，其余原样。
fn migrate_v1_to_v2(value: &mut serde_json::Value) {
    let prefix = format!("{LEGACY_LOCAL_DIR}/");
    /* 前缀改指与旧三文件那条路**共用同一处实现**（同一事实一套规则） */
    let to_new = |rel: &str| -> String { repoint_legacy_prefix(rel) };

    /* 指针与草稿：枚举值（"mine" → "local"）+ 落点前缀（正文草稿只认路径，没有枚举） */
    for key in ["activePreset", "draft", "textDraft"] {
        let Some(section) = value
            .get_mut(key)
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        if section.get("origin").and_then(serde_json::Value::as_str) == Some("mine") {
            section.insert("origin".into(), serde_json::Value::from("local"));
        }
        if let Some(rel) = section.get("path").and_then(serde_json::Value::as_str) {
            let moved = to_new(rel);
            section.insert("path".into(), serde_json::Value::from(moved));
        }
    }

    /*
     * 逐参数「云端更新」处理账的键是**本地线路径**（与草稿同一把钥匙，见 [`repoint_local`]）：
     * 不跟着改的话，"这一项我已经处理过了"会凭空消失，云端下一次发布又全翻出来。
     */
    if let Some(books) = value
        .get_mut("paramDecisions")
        .and_then(serde_json::Value::as_object_mut)
    {
        let moved: Vec<(String, String, serde_json::Value)> = books
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| (k.clone(), to_new(k), v.clone()))
            .collect();
        for (old_key, new_key, book) in moved {
            books.remove(&old_key);
            books.insert(new_key, book);
        }
    }
}

/// 迁移的读半边：`app-state.json` 还不在时，按格读三个旧文件拼出快照。
/// 旧文件缺席 = `None`（合法）；坏档照旧 `CORRUPTED` —— 迁移不降低"坏档不静默"。
fn legacy_snapshot(root: &Path) -> Result<AppStateFile, AppError> {
    let mut snapshot = AppStateFile {
        app_state_schema: APP_STATE_SCHEMA,
        active_preset: super::state::read_active_file(root)?,
        draft: super::state::read_draft_file(root)?,
        /* 正文草稿这一格是 2026-10-10 才有的：旧档里本来就没有它 */
        text_draft: None,
        preset_source: super::source::read_source_file(root)?,
        /* 逐参数决定账是 2026-10-09 才有的格：旧档里本来就没有它 */
        param_decisions: BTreeMap::new(),
        /* 主题这一格是 2026-10-12 才有的：旧档里本来就没有它（没选过 = None） */
        theme: None,
        /* 自动清理这一格是 2026-10-13 才有的：旧档里本来就没有它（没选过 = 开着） */
        auto_clean_history: None,
    };
    retire_legacy_paths(&mut snapshot);
    Ok(snapshot)
}

/* ---------- 旧目录前缀的读写半边（**只服务迁移**，别在别处再用这个字面量） ---------- */

/// 把 `presets-mine/…` 改成 `presets-local/…`；不在旧前缀下就原样返回。
///
/// 两处调用：`1 → 2`（读老 `app-state.json`）与 [`retire_legacy_paths`]（旧三文件那条路）。
/// 抽成一处是因为这两条路上的判据必须**一字不差地一样** —— 同一事实两处实现，
/// 早晚有一天只有一处被改（这个仓库在别处已经踩过）。
fn repoint_legacy_prefix(rel: &str) -> String {
    let prefix = format!("{LEGACY_LOCAL_DIR}/");
    match rel.strip_prefix(&prefix) {
        Some(rest) => format!("{}/{rest}", crate::fsx::paths::LOCAL_DIR),
        None => rel.to_owned(),
    }
}

/// 旧三文件那条路（[`legacy_snapshot`]）的**补正**：把路径从旧目录前缀换成新前缀。
///
/// 为什么需要它：那三个文件（`run/active-preset.json` / `run/draft-preset.json`）
/// 是 **1 代之前**的"一种状态一个文件"时代的产物，里面写的路径是**当时的**目录名
/// `presets-mine/…`；而今天 [`super::local::check_local_prefix`] 只认 `presets-local/…`
/// ⇒ 不换的话，那份"我自己的预设"在界面上是**隐形的**（列表里没有它，指针也解析不到）。
///
/// 与 `1 → 2` 是同一个语义、同一套规则，只是入口不同（那条在读 `app-state.json`，
/// 这条在读老三文件），所以共用 [`repoint_legacy_prefix`]。
fn retire_legacy_paths(state: &mut AppStateFile) {
    if let Some(active) = state.active_preset.as_mut() {
        if active.origin == super::state::ActiveOrigin::Local {
            if let Some(rel) = active.path.as_deref() {
                active.path = Some(repoint_legacy_prefix(rel));
            }
        }
    }
    if let Some(draft) = state.draft.as_mut() {
        if draft.origin == super::state::ActiveOrigin::Local {
            if let Some(rel) = draft.path.as_deref() {
                draft.path = Some(repoint_legacy_prefix(rel));
            }
        }
    }
}

/// AppState 的**唯一写入口**：读出整份 → 改这一格 → 整份原子写回。
/// `app-state.json` 不在时先把旧文件并进来（惰性迁移），写成功后退役旧文件；
/// `f` 失败则什么都不写、什么都不删。
fn with_state<T>(
    root: &Path,
    f: impl FnOnce(&mut AppStateFile) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let _guard = state_lock();
    let migrating = !state_file(root).exists();
    let mut state = load_locked(root)?;
    let out = f(&mut state)?;
    atomic_write_json(&state_file(root), &state)?;
    if migrating {
        for legacy in legacy_files(root) {
            match std::fs::remove_file(&legacy) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => tracing::warn!(
                    path = %legacy.display(),
                    detail = %e,
                    "旧状态文件没删掉（不影响：读端只认 app-state.json）"
                ),
            }
        }
    }
    Ok(out)
}

/* ---------- activePreset：使用中指针 ---------- */

/// 当前使用的是哪一份。`None` = 还没用任何一份（合法状态，不是错误）
pub fn active_preset(root: &Path) -> Result<Option<ActivePreset>, AppError> {
    Ok(load(root)?.active_preset)
}

/// 记下"用这一份云端的"。`file` 来自 catalog；
/// 指纹优先用目录登记的期望值，没登记就对盘上真字节算一次（算不出 = 文件不在，报错）。
pub fn set_active_preset(root: &Path, file: &CatalogFile) -> Result<ActivePreset, AppError> {
    let sha256 = match file.expected_sha() {
        Some(want) => want.to_owned(),
        None => {
            let bytes = std::fs::read(root.join(&file.path)).map_err(|_| {
                AppError::not_found(format!("{} 还不在本机 —— 先下载，再使用", file.file_name))
            })?;
            hex(&Sha256::digest(&bytes))
        }
    };
    with_state(root, |state| {
        let active = ActivePreset {
            active_schema: super::state::ACTIVE_SCHEMA,
            origin: ActiveOrigin::Preset,
            file_name: file.file_name.clone(),
            sha256,
            path: None,
        };
        state.active_preset = Some(active.clone());
        Ok(active)
    })
}

/// 记下"用我自己那一份"（`presets-local/…`）。`rel` 是相对用户根的路径，
/// `sha256` 是应用那一刻它的字节摘要（用户后来自己改它就是"漂了"—— 对用户线是正常事）。
pub fn set_active_local(root: &Path, rel: &str, sha256: &str) -> Result<ActivePreset, AppError> {
    let file_name = Path::new(rel)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_owned());
    with_state(root, |state| {
        let active = ActivePreset {
            active_schema: super::state::ACTIVE_SCHEMA,
            origin: ActiveOrigin::Local,
            file_name,
            sha256: sha256.to_owned(),
            path: Some(rel.to_owned()),
        };
        state.active_preset = Some(active.clone());
        Ok(active)
    })
}

/// 撤销使用。幂等：本来就没在用也不报错。
pub fn clear_active_preset(root: &Path) -> Result<(), AppError> {
    with_state(root, |state| {
        state.active_preset = None;
        Ok(())
    })
}

/* ---------- draft：编辑中的那一份 ---------- */

/// 读草稿。没有 → `None`（还没开始改就是这个状态）
pub fn draft(root: &Path) -> Result<Option<PresetDraft>, AppError> {
    Ok(load(root)?.draft)
}

/// 落一份草稿（整份替换旧的 —— 同一时刻只改一份，写新的自然盖旧的）
pub fn set_draft(
    root: &Path,
    subject: &DraftSubject,
    source_sha256: &str,
    text: &str,
) -> Result<PresetDraft, AppError> {
    with_state(root, |state| {
        let draft = PresetDraft {
            draft_schema: super::state::DRAFT_SCHEMA,
            origin: subject.origin,
            source_file_name: subject.file_name.clone(),
            path: subject.path.clone(),
            source_sha256: source_sha256.to_owned(),
            text: text.to_owned(),
            updated_unix: now_unix(),
        };
        state.draft = Some(draft.clone());
        Ok(draft)
    })
}

/// 丢掉草稿（放弃 / 保存完之后）。没有草稿也不算错（幂等）
pub fn clear_draft(root: &Path) -> Result<(), AppError> {
    with_state(root, |state| {
        state.draft = None;
        Ok(())
    })
}

/* ---------- text_draft：「编辑文件」那条链（2026-10-10） ---------- */

/// 读正文草稿。没有 → `None`（编辑器没开就是这个状态）
pub fn text_draft(root: &Path) -> Result<Option<PresetTextDraft>, AppError> {
    Ok(load(root)?.text_draft)
}

/// 落一份正文草稿（整份替换 —— 同一时刻只改一份）
pub fn set_text_draft(
    root: &Path,
    file_name: &str,
    path: &str,
    text: &str,
) -> Result<PresetTextDraft, AppError> {
    with_state(root, |state| {
        let draft = PresetTextDraft {
            draft_schema: super::state::DRAFT_SCHEMA,
            file_name: file_name.to_owned(),
            path: path.to_owned(),
            text: text.to_owned(),
            updated_unix: now_unix(),
        };
        state.text_draft = Some(draft.clone());
        Ok(draft)
    })
}

/// 丢掉正文草稿（放弃 / 保存完 / 被另一侧作废）。没有也不算错（幂等）
pub fn clear_text_draft(root: &Path) -> Result<(), AppError> {
    with_state(root, |state| {
        state.text_draft = None;
        Ok(())
    })
}

/// **用户文件改过名之后，两本状态账跟着走**（使用中指针 + 草稿，一次写里一起改 ——
/// 以前是两个文件各写各的，现在合并成一次原子替换）。只动"正指着这一份 / 改的是这一份"
/// 的**用户线**条目：路径与文件名换成新的，指纹与正文原样（文件一个字节没动）。
/// 返回 `(指针跟了, 草稿跟了)`。
pub fn repoint_local(
    root: &Path,
    old_rel: &str,
    new_rel: &str,
    new_file_name: &str,
) -> Result<(bool, bool), AppError> {
    with_state(root, |state| {
        let mut active_followed = false;
        if let Some(active) = state.active_preset.as_mut() {
            if active.origin == ActiveOrigin::Local && active.path.as_deref() == Some(old_rel) {
                active.file_name = new_file_name.to_owned();
                active.path = Some(new_rel.to_owned());
                active_followed = true;
            }
        }
        let mut draft_followed = false;
        if let Some(draft) = state.draft.as_mut() {
            if draft.origin == ActiveOrigin::Local && draft.path.as_deref() == Some(old_rel) {
                draft.source_file_name = new_file_name.to_owned();
                draft.path = Some(new_rel.to_owned());
                draft_followed = true;
            }
        }
        /*
         * 逐参数决定账按**路径**认这一份（与草稿同一条钥匙）：改名之后它得跟到新路径上，
         * 否则"这项云端更新我已经处理过了"会凭空消失，云端下一次发布又把它们全翻出来。
         */
        if let Some(book) = state.param_decisions.remove(old_rel) {
            state.param_decisions.insert(new_rel.to_owned(), book);
        }
        Ok((active_followed, draft_followed))
    })
}

/* ---------- paramDecisions：逐参数「云端更新」处理账 ---------- */

/// 这一份用户预设上，**我做过的那些决定**（参数 key → 决定）。没有处理过就是空表。
pub fn param_decisions(
    root: &Path,
    rel: &str,
) -> Result<BTreeMap<String, ParamDecision>, AppError> {
    Ok(load(root)?
        .param_decisions
        .get(rel)
        .cloned()
        .unwrap_or_default())
}

/// 记下/覆盖若干项决定（同一份文件上的**合并写**，别的项一个不动）。
///
/// 幂等：同样的决定记两次结果一样。一次写里整份状态原子替换 ——
/// 「采用」这种一次改几十项的动作也只有一个中间的态。
pub fn record_param_decisions(
    root: &Path,
    rel: &str,
    entries: &[(String, ParamDecision)],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }
    with_state(root, |state| {
        let book = state.param_decisions.entry(rel.to_owned()).or_default();
        for (key, decision) in entries {
            book.insert(key.clone(), decision.clone());
        }
        Ok(())
    })
}

/// 忘掉这一份的整本决定账（文件被删掉时用）。没有账也不报错（幂等）。
pub fn forget_param_decisions(root: &Path, rel: &str) -> Result<(), AppError> {
    with_state(root, |state| {
        state.param_decisions.remove(rel);
        Ok(())
    })
}

/* ---------- presetSource：数据源设置 ---------- */

/// 读数据源设置。没有 → `None`（**没配过是合法状态**）
pub fn preset_source(root: &Path) -> Result<Option<PresetSource>, AppError> {
    Ok(load(root)?.preset_source)
}

/// 记下"当前用这个源"（校验在 [`super::source::make_source`]，这里只管落盘）
pub fn set_preset_source(root: &Path, source: PresetSource) -> Result<PresetSource, AppError> {
    with_state(root, |state| {
        state.preset_source = Some(source.clone());
        Ok(source)
    })
}

/// 撤掉用户覆盖：这一格清空（幂等）。「回到内置默认」只有这一条路。
pub fn clear_preset_source(root: &Path) -> Result<(), AppError> {
    with_state(root, |state| {
        state.preset_source = None;
        Ok(())
    })
}

/* ---------- theme：界面主题偏好 ---------- */

/// 读主题偏好。`None` = **还没选过**（与"选了跟随系统"两码事，见 [`ThemePref`] 的模块注释）。
pub fn theme(root: &Path) -> Result<Option<ThemePref>, AppError> {
    Ok(load(root)?.theme)
}

/// 记下主题偏好（整份状态原子替换，别的格一个不动）。返回写进去的那一档。
pub fn set_theme(root: &Path, pref: ThemePref) -> Result<ThemePref, AppError> {
    with_state(root, |state| {
        state.theme = Some(pref);
        Ok(pref)
    })
}

/* ---------- autoCleanHistory：执行记录的自动清理开关 ---------- */

/// 自动清理开着吗。**`None` = 还没选过 ⇒ 开着**（出厂默认，见字段注释）。
///
/// 读者是清理那两条入口（[`crate::retention`]）—— 界面不直接读它，
/// 设置页那一格走 `ipc::prefs` 的命令。
pub fn auto_clean_history(root: &Path) -> Result<bool, AppError> {
    Ok(load(root)?.auto_clean_history.unwrap_or(true))
}

/// 记下自动清理开关（整份状态原子替换，别的格一个不动）。返回写进去的那一档。
pub fn set_auto_clean_history(root: &Path, enabled: bool) -> Result<bool, AppError> {
    with_state(root, |state| {
        state.auto_clean_history = Some(enabled);
        Ok(enabled)
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::super::{paths, source, state};
    use super::*;
    use crate::error::ErrorCode;
    use crate::fsx::atomic::atomic_write;

    fn entry(name: &str, content: &[u8]) -> CatalogFile {
        CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            /* 真实布局：交付根相对路径 = 客户端落点（都是 `mkp/presets/…`） */
            path: format!("mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: Some(hex(&Sha256::digest(content))),
            size: Some(content.len() as u64),
        }
    }

    /* ---------- 整份状态文件的规矩 ---------- */

    /// 什么都没写过：整份状态 = 全 None，且**读不落盘**（没写过的应用不留状态文件）
    #[test]
    fn fresh_app_reads_an_all_none_state_without_writing() {
        let d = tempfile::tempdir().unwrap();
        let state = load(d.path()).unwrap();
        assert!(state.active_preset.is_none());
        assert!(state.draft.is_none());
        assert!(state.preset_source.is_none());
        assert_eq!(state.app_state_schema, APP_STATE_SCHEMA);
        assert!(!state_file(d.path()).exists(), "读不落盘");
    }

    /// 坏档不静默：解析不了要响，不能装作"没有"
    #[test]
    fn corrupted_state_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(&state_file(d.path()), "{ 不是 JSON }".as_bytes()).unwrap();
        assert_eq!(load(d.path()).unwrap_err().code, ErrorCode::Corrupted);
    }

    /// `1 → 2`：老盘那份（本地线落点还是 `presets-mine/…`）读进来**自动升代次、换前缀**，
    /// 而且**读不落盘** —— 升出来的结果要等第一次真写入才落盘（与"读不落盘"同一条）。
    #[test]
    fn schema_one_is_migrated_in_memory_and_paths_are_repointed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = r#"{ "appStateSchema": 1,
               "activePreset": { "activeSchema": 1, "origin": "mine",
                 "fileName": "A1-fast.toml", "sha256": "abc",
                 "path": "presets-mine/A1-fast.toml" },
               "draft": { "draftSchema": 1, "origin": "mine",
                 "sourceFileName": "A1-fast.toml", "path": "presets-mine/A1-fast.toml",
                 "sourceSha256": "abc",
                 "text": "涂胶 = 1", "updatedUnix": 1 },
               "textDraft": { "draftSchema": 1, "fileName": "A1-fast.toml",
                 "path": "presets-mine/A1-fast.toml", "text": "涂胶 = 1",
                 "updatedUnix": 1 },
               "paramDecisions": { "presets-mine/A1-fast.toml": { "a":
                 { "sha256": "abc", "kind": "adopt" } } } }"#;
        atomic_write(&state_file(d.path()), json.as_bytes()).unwrap();

        let state = load(d.path()).unwrap();
        assert_eq!(state.app_state_schema, APP_STATE_SCHEMA);
        let active = state.active_preset.expect("指针该读得进");
        assert_eq!(
            active.path.as_deref(),
            Some("presets-local/A1-fast.toml"),
            "本地线落点该换成新目录"
        );
        let draft = state.draft.expect("草稿该读得进");
        assert_eq!(draft.path.as_deref(), Some("presets-local/A1-fast.toml"));
        assert!(
            state
                .param_decisions
                .contains_key("presets-local/A1-fast.toml"),
            "逐参数账按同一把路径钥匙存，也得跟着走"
        );
        assert!(!state
            .param_decisions
            .contains_key("presets-mine/A1-fast.toml"));
        assert!(
            std::fs::read_to_string(state_file(d.path()))
                .unwrap()
                .contains("\"appStateSchema\": 1"),
            "读不该把它写成 2：升代次的结果等第一次真写入落盘"
        );
    }

    /// 2 代再读一次：**不重复动手**（幂等），值原样。
    #[test]
    fn migrating_twice_is_a_no_op() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = r#"{ "appStateSchema": 2,
               "activePreset": { "activeSchema": 1, "origin": "local",
                 "fileName": "A1-fast.toml", "sha256": "abc",
                 "path": "presets-local/A1-fast.toml" } }"#;
        atomic_write(&state_file(d.path()), json.as_bytes()).unwrap();

        for _ in 0..2 {
            let state = load(d.path()).unwrap();
            assert_eq!(state.app_state_schema, 2);
            assert_eq!(
                state.active_preset.unwrap().path.as_deref(),
                Some("presets-local/A1-fast.toml"),
                "已经在新目录里的路径不该被再改一次"
            );
        }
    }

    /// 迁移**不改盘**：失败路径（这里是坏 JSON）要如实报错，原文件一个字节不动。
    #[test]
    fn a_failed_parse_leaves_the_original_file_alone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let broken = "{ \"appStateSchema\": 1, \"activePreset\": { 不是 JSON }".as_bytes();
        atomic_write(&state_file(d.path()), broken).unwrap();

        assert_eq!(load(d.path()).unwrap_err().code, ErrorCode::Corrupted);
        assert_eq!(
            std::fs::read(state_file(d.path())).unwrap(),
            broken,
            "报错不写盘、不重置"
        );
    }

    /// **旧三文件那条路也要改前缀**：`run/active-preset.json` 是 1 代之前写下的，
    /// 里面那条路径是**当时的目录名** `presets-mine/…` —— 不改的话
    /// `check_local_prefix` 认不出来，那份"我自己的预设"在界面上永远是隐形的。
    #[test]
    fn the_legacy_three_files_path_also_gets_repointed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        /* 只造"使用中"那一个旧文件（另外两个缺席 = None，合法） */
        let active_schema = state::ACTIVE_SCHEMA;
        let json = format!(
            "{{ \"activeSchema\": {active_schema}, \"origin\": \"local\", \
             \"fileName\": \"A1_MINE.toml\", \"sha256\": \"abc\", \
             \"path\": \"presets-mine/A1_MINE.toml\" }}"
        );
        atomic_write(
            &d.path().join(super::super::state::ACTIVE_FILE),
            json.as_bytes(),
        )
        .unwrap();

        let state = load(d.path()).unwrap();
        assert_eq!(
            state.active_preset.unwrap().path.as_deref(),
            Some("presets-local/A1_MINE.toml")
        );
        /* 读旧三文件这条路也**不落盘**：老三文件还在原处、没有 app-state.json */
        assert!(d.path().join(super::super::state::ACTIVE_FILE).exists());
        assert!(!state_file(d.path()).exists());
    }

    /// 云端线**不存路径**：那条路上没有东西要改，别把 `None` 变成空串。
    #[test]
    fn repointing_leaves_the_cloud_line_alone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let active_schema = state::ACTIVE_SCHEMA;
        let json = format!(
            "{{ \"activeSchema\": {active_schema}, \"origin\": \"cloud\", \
             \"fileName\": \"A1_MINE.toml\", \"sha256\": \"abc\" }}"
        );
        atomic_write(
            &d.path().join(super::super::state::ACTIVE_FILE),
            json.as_bytes(),
        )
        .unwrap();

        assert!(load(d.path())
            .unwrap()
            .active_preset
            .unwrap()
            .path
            .is_none());
    }

    /// 前置换代的**判据本身**：只认那一个旧前缀，别的一字不改（含已经在新时代的路径、
    /// 以及"名字里恰好含 presets-mine 但不在开头"的那种）。
    #[test]
    fn the_legacy_prefix_rewrite_is_exact() {
        assert_eq!(
            repoint_legacy_prefix("presets-mine/A1.toml"),
            "presets-local/A1.toml"
        );
        assert_eq!(
            repoint_legacy_prefix("presets-mine/子目录/A1.toml"),
            "presets-local/子目录/A1.toml",
            "用户自己分的文件夹结构要保住"
        );
        assert_eq!(
            repoint_legacy_prefix("presets-local/A1.toml"),
            "presets-local/A1.toml",
            "已经在新前缀下的不动"
        );
        assert_eq!(
            repoint_legacy_prefix("exports/presets-mine/A1.toml"),
            "exports/presets-mine/A1.toml",
            "只有开头的那个前缀算数"
        );
    }

    /// 代次认不出同样是坏档
    #[test]
    fn future_schema_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &state_file(d.path()),
            format!("{{ \"appStateSchema\": {} }}", APP_STATE_SCHEMA + 1).as_bytes(),
        )
        .unwrap();
        assert_eq!(load(d.path()).unwrap_err().code, ErrorCode::Corrupted);
    }

    /// 各格互不干扰：存草稿 / 丢草稿都不动使用中指针，反之亦然
    #[test]
    fn sections_do_not_interfere() {
        let d = tempfile::tempdir().unwrap();
        set_active_preset(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        set_draft(
            d.path(),
            &DraftSubject::preset("A1-fast.toml"),
            "x",
            "涂胶 = 2",
        )
        .unwrap();

        assert_eq!(
            active_preset(d.path()).unwrap().unwrap().file_name,
            "A1-standard.toml",
            "存草稿不动使用中指针"
        );
        clear_draft(d.path()).unwrap();
        assert!(
            active_preset(d.path()).unwrap().is_some(),
            "丢草稿也不动使用中指针"
        );
        clear_active_preset(d.path()).unwrap();
        assert!(draft(d.path()).unwrap().is_none(), "撤使用中不动草稿");
    }

    /* ---------- activePreset ---------- */

    #[test]
    fn save_then_load_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        let saved = set_active_preset(d.path(), &entry("A1-standard.toml", b"content")).unwrap();
        let loaded = active_preset(d.path()).unwrap().expect("该读回来");

        assert_eq!(saved.file_name, loaded.file_name);
        assert_eq!(saved.sha256, loaded.sha256);
        assert_eq!(loaded.active_schema, state::ACTIVE_SCHEMA);
    }

    /// 全局唯一的"应用"：写新的自然盖旧的，构造上没有两份并存的可能
    #[test]
    fn applying_another_replaces_the_first() {
        let d = tempfile::tempdir().unwrap();
        set_active_preset(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        set_active_preset(d.path(), &entry("A1-fast.toml", b"b")).unwrap();
        assert_eq!(
            active_preset(d.path()).unwrap().unwrap().file_name,
            "A1-fast.toml",
            "后应用的赢"
        );
    }

    #[test]
    fn clear_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        set_active_preset(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        clear_active_preset(d.path()).unwrap();
        assert!(active_preset(d.path()).unwrap().is_none());
        clear_active_preset(d.path()).unwrap(); // 再撤一次也不报错
    }

    /* ---------- draft ---------- */

    /// 草稿存得下、读得回、丢掉之后就是 None（幂等）
    #[test]
    fn draft_roundtrips_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let subject = DraftSubject::preset("A1-fast.toml");
        let saved = set_draft(d.path(), &subject, "abc", "涂胶 = 1.2").unwrap();
        let loaded = draft(d.path()).unwrap().expect("该读回来");

        assert_eq!(loaded.source_file_name, "A1-fast.toml");
        assert_eq!(loaded.source_sha256, "abc", "打开那一刻的指纹要留着");
        assert_eq!(loaded.text, "涂胶 = 1.2");
        assert_eq!(loaded.updated_unix, saved.updated_unix);

        clear_draft(d.path()).unwrap();
        assert!(draft(d.path()).unwrap().is_none());
        clear_draft(d.path()).unwrap(); // 再丢一次也不报错
    }

    /// 两条线的钥匙不同：用户目录里两份文件同名很正常（他自己分文件夹），
    /// 只比文件名会把 A 的草稿接到 B 上
    #[test]
    fn a_draft_knows_which_line_it_belongs_to() {
        let d = tempfile::tempdir().unwrap();
        set_draft(d.path(), &DraftSubject::preset("A1-fast.toml"), "a", "云端").unwrap();
        let local = DraftSubject::local("A1-fast.toml", "presets-local/本地/A1-fast.toml");
        set_draft(d.path(), &local, "b", "本地").unwrap();

        let loaded = draft(d.path()).unwrap().expect("该读回来");
        assert_eq!(loaded.text, "本地");
        assert_eq!(loaded.subject(), local, "改的是我自己那份");
        assert!(
            !loaded
                .subject()
                .matches(&DraftSubject::preset("A1-fast.toml")),
            "同名不同线 ≠ 同一份：「接着改」不许接错"
        );
    }

    /// 改名跟随：指针与草稿**都只跟"正指着这一份 / 改的是这一份"**，
    /// 别的条目（云端线、指着别人那份的）一概不动；一次写里两格一起跟
    #[test]
    fn repoint_local_follows_only_the_matching_entries() {
        let d = tempfile::tempdir().unwrap();
        set_active_local(d.path(), "presets-local/A1.toml", "sha").unwrap();
        set_active_preset(d.path(), &entry("A1-standard.toml", b"a")).unwrap(); // 这一下把指针换成云端了
        set_active_local(d.path(), "presets-local/A1.toml", "sha").unwrap(); // 换回本地
        set_draft(
            d.path(),
            &DraftSubject::local("A1.toml", "presets-local/A1.toml"),
            "sha",
            "改到一半",
        )
        .unwrap();

        let (active, draft_followed) = repoint_local(
            d.path(),
            "presets-local/A1.toml",
            "presets-local/A2.toml",
            "A2.toml",
        )
        .unwrap();
        assert!(active && draft_followed, "两格都该跟走");

        let active = active_preset(d.path()).unwrap().unwrap();
        assert_eq!(active.file_name, "A2.toml");
        assert_eq!(active.path.as_deref(), Some("presets-local/A2.toml"));
        assert_eq!(active.sha256, "sha", "文件一个字节没动，指纹原样");
        let draft = draft(d.path()).unwrap().unwrap();
        assert_eq!(draft.source_file_name, "A2.toml");
        assert_eq!(draft.text, "改到一半", "正文原样");
    }

    /* ---------- paramDecisions ---------- */

    fn adopt(sha: &str) -> ParamDecision {
        ParamDecision {
            sha256: sha.to_owned(),
            kind: ParamDecisionKind::Adopt,
        }
    }

    /// 记下 → 读回 → 合并写（同一份的别的项一个不动）；没记过的份是空表
    #[test]
    fn param_decisions_roundtrip_and_merge() {
        let d = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1-fast.toml";
        assert!(
            param_decisions(d.path(), rel).unwrap().is_empty(),
            "没处理过 = 空表"
        );

        record_param_decisions(
            d.path(),
            rel,
            &[("toolhead.offset.x".to_owned(), adopt("aa"))],
        )
        .unwrap();
        record_param_decisions(
            d.path(),
            rel,
            &[(
                "wiping.glue_pass_count".to_owned(),
                ParamDecision {
                    sha256: "aa".to_owned(),
                    kind: ParamDecisionKind::Hold,
                },
            )],
        )
        .unwrap();

        let book = param_decisions(d.path(), rel).unwrap();
        assert_eq!(book.len(), 2, "两次记的都在");
        assert_eq!(book["toolhead.offset.x"], adopt("aa"));
        assert_eq!(book["wiping.glue_pass_count"].kind, ParamDecisionKind::Hold);

        /* 同一项再记一次 = 覆盖，不是多一条 */
        record_param_decisions(
            d.path(),
            rel,
            &[("toolhead.offset.x".to_owned(), adopt("bb"))],
        )
        .unwrap();
        let book = param_decisions(d.path(), rel).unwrap();
        assert_eq!(book.len(), 2);
        assert_eq!(book["toolhead.offset.x"].sha256, "bb");
    }

    /// 决定账**不干扰**别的格：存决定不动使用中指针与草稿
    #[test]
    fn param_decisions_do_not_touch_other_sections() {
        let d = tempfile::tempdir().unwrap();
        set_active_preset(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        record_param_decisions(
            d.path(),
            "presets-local/A1.toml",
            &[("toolhead.offset.x".to_owned(), adopt("aa"))],
        )
        .unwrap();
        assert_eq!(
            active_preset(d.path()).unwrap().unwrap().file_name,
            "A1-standard.toml"
        );
        assert!(draft(d.path()).unwrap().is_none());
    }

    /// 删掉那一份文件 → 决定账跟着忘（幂等）
    #[test]
    fn forgetting_a_book_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1.toml";
        record_param_decisions(
            d.path(),
            rel,
            &[("toolhead.offset.x".to_owned(), adopt("aa"))],
        )
        .unwrap();
        forget_param_decisions(d.path(), rel).unwrap();
        assert!(param_decisions(d.path(), rel).unwrap().is_empty());
        forget_param_decisions(d.path(), rel).unwrap(); // 再忘一次也不报错
    }

    /// 改名之后决定账**跟到新路径**：不跟的话云端下一次发布会把处理过的项全翻出来
    #[test]
    fn renaming_moves_the_book_to_the_new_path() {
        let d = tempfile::tempdir().unwrap();
        let old = "presets-local/A1.toml";
        let new = "presets-local/A2.toml";
        record_param_decisions(
            d.path(),
            old,
            &[("toolhead.offset.x".to_owned(), adopt("aa"))],
        )
        .unwrap();

        repoint_local(d.path(), old, new, "A2.toml").unwrap();

        assert!(
            param_decisions(d.path(), old).unwrap().is_empty(),
            "旧路径上不该还留着"
        );
        assert_eq!(
            param_decisions(d.path(), new).unwrap()["toolhead.offset.x"],
            adopt("aa")
        );
    }

    /* ---------- presetSource ---------- */

    #[test]
    fn source_roundtrips_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let source = source::make_source(
            source::SourceMode::Custom,
            Some("https://cdn.example.com/mkp/"),
            source::CustomShape::Unset,
        )
        .unwrap();
        let saved = set_preset_source(d.path(), source).unwrap();
        assert_eq!(preset_source(d.path()).unwrap(), Some(saved));

        clear_preset_source(d.path()).unwrap();
        assert_eq!(preset_source(d.path()).unwrap(), None);
        clear_preset_source(d.path()).unwrap(); // 幂等
    }

    /* ---------- theme ---------- */

    /// 存得下、读得回、覆盖得住；**没选过 = None**（不是"浅色"）——
    /// 界面据此区分"跟随系统"与"用户没碰过"（`ipc::prefs` 那两条命令的口径）
    #[test]
    fn theme_roundtrips_and_starts_as_none() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(theme(d.path()).unwrap(), None, "没选过就是 None");

        assert_eq!(
            set_theme(d.path(), ThemePref::Dark).unwrap(),
            ThemePref::Dark
        );
        assert_eq!(theme(d.path()).unwrap(), Some(ThemePref::Dark));

        set_theme(d.path(), ThemePref::System).unwrap();
        assert_eq!(theme(d.path()).unwrap(), Some(ThemePref::System));
    }

    /* ---------- autoCleanHistory ---------- */

    /// **没选过 = 开着**（出厂默认 —— 与主题那一格刻意相反：盘空间才是稀缺的那一侧）；
    /// 关掉能记住、能再打开，别的格一个不动
    #[test]
    fn auto_clean_starts_on_and_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        assert!(auto_clean_history(d.path()).unwrap(), "没选过就该是开着");

        assert!(!set_auto_clean_history(d.path(), false).unwrap());
        assert!(!auto_clean_history(d.path()).unwrap());

        assert!(set_auto_clean_history(d.path(), true).unwrap());
        assert!(auto_clean_history(d.path()).unwrap());
        /* 与主题那格互不踩：关掉自动清理不该动主题 */
        assert_eq!(theme(d.path()).unwrap(), None);
    }

    /// **认字符串是契约**：只认三个字面量（盘上 / 界面 / 前端都认它），
    /// 别的给 None —— 调用方据此如实拒绝，不静默回落到浅色
    #[test]
    fn theme_wire_ids_are_the_contract() {
        assert_eq!(ThemePref::parse("dark"), Some(ThemePref::Dark));
        assert_eq!(ThemePref::parse(" light "), Some(ThemePref::Light));
        assert_eq!(ThemePref::parse("system"), Some(ThemePref::System));
        assert_eq!(ThemePref::Dark.wire(), "dark");
        assert!(ThemePref::parse("黑").is_none());
        assert!(ThemePref::parse("").is_none());
    }

    /* ---------- 迁移（一次性、惰性） ---------- */

    /// `app-state.json` 不在时，读端按格拼旧文件 —— 三格都能被读到
    #[test]
    fn reads_fall_back_to_legacy_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(state::ACTIVE_FILE),
            r#"{ "activeSchema": 1, "origin": "local", "fileName": "本地 A1.toml",
                  "sha256": "abc", "path": "presets-local/本地 A1.toml" }"#
                .as_bytes(),
        )
        .unwrap();
        atomic_write(
            &d.path().join(paths::SOURCE_FILE),
            br#"{ "sourceSchema": 2, "mode": "github" }"#,
        )
        .unwrap();

        let state = load(d.path()).unwrap();
        assert_eq!(
            state.active_preset.unwrap().file_name,
            "本地 A1.toml",
            "使用中指针从旧文件读到"
        );
        assert!(state.draft.is_none(), "没有的格就是 None");
        assert!(state.preset_source.unwrap().mode == source::SourceMode::Github);
        assert!(!state_file(d.path()).exists(), "迁移在读这半边不落盘");
    }

    /// 1 代数据源老档（只有 baseUrl）也救得回 —— 语义正好是自定义源
    #[test]
    fn a_v1_source_legacy_file_still_reads_as_custom() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(paths::SOURCE_FILE),
            br#"{"sourceSchema":1,"baseUrl":"https://example.com/presets/delivery/"}"#,
        )
        .unwrap();
        let source = preset_source(d.path()).unwrap().expect("老档该读得出来");
        assert_eq!(source.mode, source::SourceMode::Custom);
    }

    /// ★ 迁移的写半边：首次写把旧文件并进来，写成功后**旧文件退役**；
    /// 之后的读只认 app-state.json
    #[test]
    fn first_write_absorbs_legacy_files_and_retires_them() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(state::ACTIVE_FILE),
            br#"{ "activeSchema": 1, "fileName": "A1-standard.toml", "sha256": "old" }"#,
        )
        .unwrap();
        atomic_write(
            &d.path().join(state::DRAFT_FILE),
            r#"{ "draftSchema": 1, "sourceFileName": "A1-fast.toml", "sourceSha256": "s",
                  "text": "旧草稿", "updatedUnix": 1 }"#
                .as_bytes(),
        )
        .unwrap();

        // 首次写：动的是 presetSource 这一格，但 active / draft 也一起并进来
        let source =
            source::make_source(source::SourceMode::Github, None, source::CustomShape::Unset)
                .unwrap();
        set_preset_source(d.path(), source).unwrap();

        assert!(state_file(d.path()).exists());
        assert!(
            !d.path().join(state::ACTIVE_FILE).exists()
                && !d.path().join(state::DRAFT_FILE).exists()
                && !d.path().join(paths::SOURCE_FILE).exists(),
            "旧文件全部退役"
        );

        let state = load(d.path()).unwrap();
        assert_eq!(
            state.active_preset.unwrap().file_name,
            "A1-standard.toml",
            "旧的使用中指针并进来了"
        );
        assert_eq!(
            state.draft.unwrap().text,
            "旧草稿",
            "旧的草稿也并进来了（语义原样搬家）"
        );
        assert!(state.preset_source.is_some());
    }

    /// 写失败什么都不动：不写 app-state.json、不删旧文件
    #[test]
    fn a_failed_write_touches_nothing() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(state::ACTIVE_FILE),
            br#"{ "activeSchema": 1, "fileName": "A1-standard.toml", "sha256": "old" }"#,
        )
        .unwrap();

        // 云端应用在目录**没登记期望值**时要读盘上真字节；文件不在就失败 —— 状态一格不动
        let mut unregistered = entry("A1-fast.toml", b"b");
        unregistered.sha256 = None;
        let e = set_active_preset(d.path(), &unregistered).unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        assert!(!state_file(d.path()).exists(), "失败不落新档");
        assert!(d.path().join(state::ACTIVE_FILE).exists(), "失败也不删旧档");
        assert_eq!(
            active_preset(d.path()).unwrap().unwrap().file_name,
            "A1-standard.toml",
            "状态还是旧的那份"
        );
    }

    /// 旧档坏了要响：迁移不把"坏档不静默"一起迁没
    #[test]
    fn corrupted_legacy_file_is_loud() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(state::ACTIVE_FILE),
            "{ 不是 JSON }".as_bytes(),
        )
        .unwrap();
        assert_eq!(load(d.path()).unwrap_err().code, ErrorCode::Corrupted);
    }
}
