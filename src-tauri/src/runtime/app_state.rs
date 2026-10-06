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
//! 预设正文仍在交付区 / `presets-mine/`。事件账、出处账、下载文件、图片模型
//! 都不进这里（它们是业务数据 / 历史记录，各有各的落点）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;

use super::catalog::{hex, CatalogFile};
use super::source::PresetSource;
use super::state::{ActiveOrigin, ActivePreset, DraftSubject, PresetDraft};

/// 应用状态格式的代次。加 section / 加字段不升号，改语义才升（与 catalog 同一条）。
pub const APP_STATE_SCHEMA: u32 = 1;

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
    /// 编辑中的那一份（临时文件账：改的是谁、正文到哪了）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<PresetDraft>,
    /// 数据源设置：当前用哪个远端
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_source: Option<PresetSource>,
}

impl Default for AppStateFile {
    fn default() -> Self {
        Self {
            app_state_schema: APP_STATE_SCHEMA,
            active_preset: None,
            draft: None,
            preset_source: None,
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
    let state: AppStateFile = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("应用状态读不出来").with_detail(e.to_string()))?;
    if state.app_state_schema != APP_STATE_SCHEMA {
        return Err(AppError::corrupted(format!(
            "应用状态的格式代次认不了：文件是 {}，程序认 {}",
            state.app_state_schema, APP_STATE_SCHEMA
        )));
    }
    Ok(state)
}

/// 迁移的读半边：`app-state.json` 还不在时，按格读三个旧文件拼出快照。
/// 旧文件缺席 = `None`（合法）；坏档照旧 `CORRUPTED` —— 迁移不降低"坏档不静默"。
fn legacy_snapshot(root: &Path) -> Result<AppStateFile, AppError> {
    Ok(AppStateFile {
        app_state_schema: APP_STATE_SCHEMA,
        active_preset: super::state::read_active_file(root)?,
        draft: super::state::read_draft_file(root)?,
        preset_source: super::source::read_source_file(root)?,
    })
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

/// 记下"用这一份官方的"。`file` 来自 catalog；
/// 指纹优先用目录登记的期望值，没登记就对盘上真字节算一次（算不出 = 文件不在，报错）。
pub fn set_active_official(root: &Path, file: &CatalogFile) -> Result<ActivePreset, AppError> {
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
            origin: ActiveOrigin::Official,
            file_name: file.file_name.clone(),
            sha256,
            path: None,
        };
        state.active_preset = Some(active.clone());
        Ok(active)
    })
}

/// 记下"用我自己那一份"（`presets-mine/…`）。`rel` 是相对用户根的路径，
/// `sha256` 是应用那一刻它的字节摘要（用户后来自己改它就是"漂了"—— 对用户线是正常事）。
pub fn set_active_mine(root: &Path, rel: &str, sha256: &str) -> Result<ActivePreset, AppError> {
    let file_name = Path::new(rel)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_owned());
    with_state(root, |state| {
        let active = ActivePreset {
            active_schema: super::state::ACTIVE_SCHEMA,
            origin: ActiveOrigin::Mine,
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

/// **用户文件改过名之后，两本状态账跟着走**（使用中指针 + 草稿，一次写里一起改 ——
/// 以前是两个文件各写各的，现在合并成一次原子替换）。只动"正指着这一份 / 改的是这一份"
/// 的**用户线**条目：路径与文件名换成新的，指纹与正文原样（文件一个字节没动）。
/// 返回 `(指针跟了, 草稿跟了)`。
pub fn repoint_mine(
    root: &Path,
    old_rel: &str,
    new_rel: &str,
    new_file_name: &str,
) -> Result<(bool, bool), AppError> {
    with_state(root, |state| {
        let mut active_followed = false;
        if let Some(active) = state.active_preset.as_mut() {
            if active.origin == ActiveOrigin::Mine && active.path.as_deref() == Some(old_rel) {
                active.file_name = new_file_name.to_owned();
                active.path = Some(new_rel.to_owned());
                active_followed = true;
            }
        }
        let mut draft_followed = false;
        if let Some(draft) = state.draft.as_mut() {
            if draft.origin == ActiveOrigin::Mine && draft.path.as_deref() == Some(old_rel) {
                draft.source_file_name = new_file_name.to_owned();
                draft.path = Some(new_rel.to_owned());
                draft_followed = true;
            }
        }
        Ok((active_followed, draft_followed))
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

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{paths, source, state};
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
        assert_eq!(
            load(d.path()).unwrap_err().code,
            ErrorCode::Corrupted
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
        set_active_official(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        set_draft(
            d.path(),
            &DraftSubject::official("A1-fast.toml"),
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
        let saved = set_active_official(d.path(), &entry("A1-standard.toml", b"content")).unwrap();
        let loaded = active_preset(d.path()).unwrap().expect("该读回来");

        assert_eq!(saved.file_name, loaded.file_name);
        assert_eq!(saved.sha256, loaded.sha256);
        assert_eq!(loaded.active_schema, state::ACTIVE_SCHEMA);
    }

    /// 全局唯一的"应用"：写新的自然盖旧的，构造上没有两份并存的可能
    #[test]
    fn applying_another_replaces_the_first() {
        let d = tempfile::tempdir().unwrap();
        set_active_official(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        set_active_official(d.path(), &entry("A1-fast.toml", b"b")).unwrap();
        assert_eq!(
            active_preset(d.path()).unwrap().unwrap().file_name,
            "A1-fast.toml",
            "后应用的赢"
        );
    }

    #[test]
    fn clear_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        set_active_official(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        clear_active_preset(d.path()).unwrap();
        assert!(active_preset(d.path()).unwrap().is_none());
        clear_active_preset(d.path()).unwrap(); // 再撤一次也不报错
    }

    /* ---------- draft ---------- */

    /// 草稿存得下、读得回、丢掉之后就是 None（幂等）
    #[test]
    fn draft_roundtrips_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let subject = DraftSubject::official("A1-fast.toml");
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
        set_draft(
            d.path(),
            &DraftSubject::official("A1-fast.toml"),
            "a",
            "官方",
        )
        .unwrap();
        let mine = DraftSubject::mine("A1-fast.toml", "presets-mine/我的/A1-fast.toml");
        set_draft(d.path(), &mine, "b", "我的").unwrap();

        let loaded = draft(d.path()).unwrap().expect("该读回来");
        assert_eq!(loaded.text, "我的");
        assert_eq!(loaded.subject(), mine, "改的是我自己那份");
        assert!(
            !loaded
                .subject()
                .matches(&DraftSubject::official("A1-fast.toml")),
            "同名不同线 ≠ 同一份：「接着改」不许接错"
        );
    }

    /// 改名跟随：指针与草稿**都只跟"正指着这一份 / 改的是这一份"**，
    /// 别的条目（官方线、指着别人那份的）一概不动；一次写里两格一起跟
    #[test]
    fn repoint_mine_follows_only_the_matching_entries() {
        let d = tempfile::tempdir().unwrap();
        set_active_mine(d.path(), "presets-mine/A1.toml", "sha").unwrap();
        set_active_official(d.path(), &entry("A1-standard.toml", b"a")).unwrap(); // 这一下把指针换成官方了
        set_active_mine(d.path(), "presets-mine/A1.toml", "sha").unwrap(); // 换回我的
        set_draft(
            d.path(),
            &DraftSubject::mine("A1.toml", "presets-mine/A1.toml"),
            "sha",
            "改到一半",
        )
        .unwrap();

        let (active, draft_followed) =
            repoint_mine(d.path(), "presets-mine/A1.toml", "presets-mine/A2.toml", "A2.toml")
                .unwrap();
        assert!(active && draft_followed, "两格都该跟走");

        let active = active_preset(d.path()).unwrap().unwrap();
        assert_eq!(active.file_name, "A2.toml");
        assert_eq!(active.path.as_deref(), Some("presets-mine/A2.toml"));
        assert_eq!(active.sha256, "sha", "文件一个字节没动，指纹原样");
        let draft = draft(d.path()).unwrap().unwrap();
        assert_eq!(draft.source_file_name, "A2.toml");
        assert_eq!(draft.text, "改到一半", "正文原样");
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

    /* ---------- 迁移（一次性、惰性） ---------- */

    /// `app-state.json` 不在时，读端按格拼旧文件 —— 三格都能被读到
    #[test]
    fn reads_fall_back_to_legacy_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join(state::ACTIVE_FILE),
            r#"{ "activeSchema": 1, "origin": "mine", "fileName": "我的 A1.toml",
                  "sha256": "abc", "path": "presets-mine/我的 A1.toml" }"#.as_bytes(),
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
            "我的 A1.toml",
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
                  "text": "旧草稿", "updatedUnix": 1 }"#.as_bytes(),
        )
        .unwrap();

        // 首次写：动的是 presetSource 这一格，但 active / draft 也一起并进来
        let source = source::make_source(
            source::SourceMode::Github,
            None,
            source::CustomShape::Unset,
        )
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

        // 官方应用在目录**没登记期望值**时要读盘上真字节；文件不在就失败 —— 状态一格不动
        let mut unregistered = entry("A1-fast.toml", b"b");
        unregistered.sha256 = None;
        let e = set_active_official(d.path(), &unregistered).unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        assert!(!state_file(d.path()).exists(), "失败不落新档");
        assert!(
            d.path().join(state::ACTIVE_FILE).exists(),
            "失败也不删旧档"
        );
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
        atomic_write(&d.path().join(state::ACTIVE_FILE), "{ 不是 JSON }".as_bytes()).unwrap();
        assert_eq!(load(d.path()).unwrap_err().code, ErrorCode::Corrupted);
    }
}
