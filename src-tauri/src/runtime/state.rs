//! 程序自己产生的持久状态怎么存（第一圈 ⑤ 的规则，总纲第③层）。
//!
//! # 规则（新数据世界的"状态"都按这一套来）
//!
//! 1. **一种状态一个文件**——JSON、带 `*Schema` 代次字段，与 catalog 同一惯例；
//! 2. **住在内部根 `run/` 下**——那是"运行状态"的目录（`ARCHITECTURE.md` §4），
//!    文件名就是状态的名字，用户能看见、能单独删；
//! 3. **写走 [`crate::fsx::atomic::atomic_write`]**——全仓唯一写盘出口，没有例外；
//! 4. **坏档不静默**——读出来解析不了就报 `CORRUPTED`，不把它当"没有"。
//!    静默吞状态是编数据的近亲：用户明明"使用中"，界面却说没在用，那是撒谎。
//!    状态可重建（再点一次就是），所以宁可响。
//!
//! # 第一个住进来的状态：使用中指针
//!
//! [`ActivePreset`] 记"当前使用的是下载区（`mkp/`）里的哪一份"。全局唯一——
//! 产品规则定的「同一时刻只能有一份处于已应用状态」，在构造上成立：
//! 状态就一个文件，写新的自然盖旧的（原子替换，没有中间态）。
//!
//! 指针里带着**应用时刻的 SHA**：`mkp/` 是只读原件区，这份 SHA 就是"文件没被动过"
//! 的凭证——盘上的字节漂了，界面能看出「文件已经不是当时应用的那份」。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::catalog::hex;
use super::catalog::CatalogFile;

/// 状态格式的代次。加字段不升号，改语义才升（与 [`super::catalog::CATALOG_SCHEMA`] 同一条）
pub const ACTIVE_SCHEMA: u32 = 1;

const ACTIVE_FILE: &str = "run/active-preset.json";

/// 使用中指针。全局唯一，`None` = 还没用任何一份（合法状态，不是错误）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePreset {
    pub active_schema: u32,
    /// `mkp/` 里的文件名。用 catalog 的 `file_name` 做键，不存路径——路径是目录的职责
    pub file_name: String,
    /// 应用时刻的指纹。将来拿它发现"文件已经不是当时那份"
    pub sha256: String,
}

/// 状态文件的落点：`<appDataDir>/run/active-preset.json`
pub fn active_file(root: &Path) -> PathBuf {
    root.join(ACTIVE_FILE)
}

/// 读使用中指针。文件不存在 → `Ok(None)`；解析不了 → `Err(CORRUPTED)`（坏档不静默）
pub fn load_active(root: &Path) -> Result<Option<ActivePreset>, AppError> {
    let bytes = match std::fs::read(active_file(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io("读不到使用中状态").with_detail(e.to_string()));
        }
    };
    let state: ActivePreset = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("使用中状态读不出来").with_detail(e.to_string()))?;
    if state.active_schema != ACTIVE_SCHEMA {
        return Err(AppError::corrupted(format!(
            "使用中状态的格式代次认不了：文件是 {}，程序认 {}",
            state.active_schema, ACTIVE_SCHEMA
        )));
    }
    Ok(Some(state))
}

/// 记下"用这一份"。`file` 来自 catalog（名字与 SHA 都是目录登记的），整份替换旧的。
pub fn save_active(root: &Path, file: &CatalogFile) -> Result<ActivePreset, AppError> {
    let state = ActivePreset {
        active_schema: ACTIVE_SCHEMA,
        file_name: file.file_name.clone(),
        sha256: file.sha256.clone(),
    };
    let json = serde_json::to_vec_pretty(&state)
        .map_err(|e| AppError::internal("使用中状态序列化失败").with_detail(e.to_string()))?;
    atomic_write(&active_file(root), &json)?;
    Ok(state)
}

/// 撤销使用 = 删掉那个状态文件。文件本来就不在 → 不算错（撤销的幂等性）
pub fn clear_active(root: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(active_file(root)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io("撤销使用失败").with_detail(e.to_string())),
    }
}

/// 应用时刻的指纹 → 当下的盘。`Ok(true)` = 文件还是当时那份。
/// 状态不在 → `Ok(false)`；盘上的文件没了也算"漂了"（指针指向的东西不存在了）。
pub fn active_matches_disk(root: &Path, state: &ActivePreset) -> bool {
    let bytes = std::fs::read(root.join("mkp").join(&state.file_name));
    match bytes {
        Ok(bytes) => hex(&Sha256::digest(&bytes)) == state.sha256,
        Err(_) => false,
    }
}

/* ---------- 第二个住进来的状态：编辑中的那一份（临时文件） ---------- */

/// 草稿格式的代次
pub const DRAFT_SCHEMA: u32 = 1;

const DRAFT_FILE: &str = "run/draft-preset.json";

/// **编辑中的那一份**。全局唯一 —— 同一时刻只改一份（与"使用中指针"同一条道理：
/// 一个状态一个文件，写新的自然盖旧的）。
///
/// 它是"临时编辑"这条链的第一步（总纲 §1③）：
///
/// ```text
/// mkp/presets/A1-fast.toml     官方原件 —— 编辑全程**一动不动**
///        │ 点「改这份」：正文复制出来
///        ▼
/// run/draft-preset.json        临时文件（用户改的是它；改到一半关掉也还在）
///        │ 点「保存为用户文件」
///        ▼
/// presets-mine/A1-fast（已修改）.toml
/// ```
///
/// **为什么不跟官方原件同目录**（示意里那个 `A1MF_260701.tmp.toml`）：`mkp/` 是**下载区**，
/// 它的判据是"盘上每个文件都在目录里登记"（[`super::delivery::stale_files`] 就是靠这条
/// 认陈旧文件的）—— 往里塞一个 `.tmp`，它立刻变成"目录里没有的陈旧文件"，
/// 污染交付那一层的每一条判据。草稿是**运行状态**（"我正在改哪一份"），住 `run/`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraft {
    pub draft_schema: u32,
    /// 从哪一份改出来的（`mkp/` 里的**文件名** —— catalog 的键，不存路径）
    pub source_file_name: String,
    /// 打开那一刻官方原件的字节指纹。**留着是为了以后能回答"你改的是不是当时那一版"**，
    /// 这一层不做判断（判断要连上"官方更新"那条线，见 HANDOFF §3.5 的第 6/7 步）
    pub source_sha256: String,
    /// 正文：用户改到哪算哪
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）。不引时间库，读时用
    pub updated_unix: u64,
}

/// 草稿的落点：`<appDataDir>/run/draft-preset.json`
pub fn draft_file(root: &Path) -> PathBuf {
    root.join(DRAFT_FILE)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 读草稿。不在 → `Ok(None)`；解析不了 → `Err(CORRUPTED)`（坏档不静默，与使用中指针同一条）
pub fn load_draft(root: &Path) -> Result<Option<PresetDraft>, AppError> {
    let bytes = match std::fs::read(draft_file(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::io("读不到编辑中的那份").with_detail(e.to_string())),
    };
    let draft: PresetDraft = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("编辑中的那份读不出来").with_detail(e.to_string()))?;
    if draft.draft_schema != DRAFT_SCHEMA {
        return Err(AppError::corrupted(format!(
            "草稿的格式代次认不了：文件是 {}，程序认 {}",
            draft.draft_schema, DRAFT_SCHEMA
        )));
    }
    Ok(Some(draft))
}

/// 落一份草稿（整份替换旧的 —— 一个状态一个文件）
pub fn save_draft(
    root: &Path,
    source_file_name: &str,
    source_sha256: &str,
    text: &str,
) -> Result<PresetDraft, AppError> {
    let draft = PresetDraft {
        draft_schema: DRAFT_SCHEMA,
        source_file_name: source_file_name.to_owned(),
        source_sha256: source_sha256.to_owned(),
        text: text.to_owned(),
        updated_unix: now_unix(),
    };
    let json = serde_json::to_vec_pretty(&draft)
        .map_err(|e| AppError::internal("草稿序列化失败").with_detail(e.to_string()))?;
    atomic_write(&draft_file(root), &json)?;
    Ok(draft)
}

/// 丢掉草稿（放弃 / 保存完之后）。文件本来就不在 → 不算错（幂等）
pub fn clear_draft(root: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(draft_file(root)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io("丢掉草稿失败").with_detail(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, content: &[u8]) -> CatalogFile {
        CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: hex(&Sha256::digest(content)),
            size: content.len() as u64,
        }
    }

    #[test]
    fn save_then_load_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        let file = entry("A1-standard.toml", b"content");

        let saved = save_active(d.path(), &file).unwrap();
        let loaded = load_active(d.path()).unwrap().expect("该读回来");

        assert_eq!(saved.file_name, loaded.file_name);
        assert_eq!(saved.sha256, loaded.sha256);
        assert_eq!(loaded.active_schema, ACTIVE_SCHEMA);
    }

    #[test]
    fn absent_state_is_none_not_error() {
        let d = tempfile::tempdir().unwrap();
        assert!(load_active(d.path()).unwrap().is_none(), "没用过 = None");
    }

    /// 坏档不静默：解析不了要响，不能装作"没有"
    #[test]
    fn corrupted_state_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        crate::fsx::atomic::atomic_write(&active_file(d.path()), "{ 不是 JSON }".as_bytes())
            .unwrap();

        let e = load_active(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    #[test]
    fn future_schema_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"activeSchema\": {}, \"fileName\": \"x\", \"sha256\": \"y\" }}",
            ACTIVE_SCHEMA + 1
        );
        crate::fsx::atomic::atomic_write(&active_file(d.path()), json.as_bytes()).unwrap();

        let e = load_active(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /* ---------- 编辑中的那一份（临时文件） ---------- */

    /// 草稿存得下、读得回、丢掉之后就是 None（幂等）
    #[test]
    fn draft_roundtrips_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let saved = save_draft(d.path(), "A1-fast.toml", "abc", "涂胶 = 1.2").unwrap();
        let loaded = load_draft(d.path()).unwrap().expect("该读回来");

        assert_eq!(loaded.source_file_name, "A1-fast.toml");
        assert_eq!(loaded.source_sha256, "abc", "打开那一刻的指纹要留着");
        assert_eq!(loaded.text, "涂胶 = 1.2");
        assert_eq!(loaded.updated_unix, saved.updated_unix);
        assert_eq!(loaded.draft_schema, DRAFT_SCHEMA);

        clear_draft(d.path()).unwrap();
        assert!(load_draft(d.path()).unwrap().is_none());
        clear_draft(d.path()).unwrap(); // 再丢一次也不报错
    }

    /// 没草稿 = None，不是错误（还没开始改就是这个状态）
    #[test]
    fn absent_draft_is_none_not_error() {
        let d = tempfile::tempdir().unwrap();
        assert!(load_draft(d.path()).unwrap().is_none());
    }

    /// 坏档不静默：解析不了要响
    #[test]
    fn corrupted_draft_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        crate::fsx::atomic::atomic_write(&draft_file(d.path()), "{ 不是 JSON }".as_bytes())
            .unwrap();

        assert_eq!(
            load_draft(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// 草稿与使用中指针**两个文件、互不干扰**：改一份不等于在用它
    #[test]
    fn draft_and_active_are_separate_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        save_active(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        save_draft(d.path(), "A1-fast.toml", "x", "涂胶 = 2").unwrap();

        assert_eq!(
            load_active(d.path()).unwrap().unwrap().file_name,
            "A1-standard.toml",
            "存草稿不动使用中指针"
        );
        clear_draft(d.path()).unwrap();
        assert!(
            load_active(d.path()).unwrap().is_some(),
            "丢草稿也不动使用中指针"
        );
    }

    /// 全局唯一的"应用"：写新的自然盖旧的，构造上没有两份并存的可能
    #[test]
    fn applying_another_replaces_the_first() {
        let d = tempfile::tempdir().unwrap();
        save_active(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        save_active(d.path(), &entry("A1-fast.toml", b"b")).unwrap();

        let state = load_active(d.path()).unwrap().expect("该有一份");
        assert_eq!(state.file_name, "A1-fast.toml", "后应用的赢");
    }

    #[test]
    fn clear_is_idempotent() {
        let d = tempfile::tempdir().unwrap();
        save_active(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        clear_active(d.path()).unwrap();
        assert!(load_active(d.path()).unwrap().is_none());
        clear_active(d.path()).unwrap(); // 再撤一次也不报错
    }

    #[test]
    fn drift_is_detected_when_disk_bytes_change() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("mkp")).unwrap();
        let file = entry("A1-standard.toml", "当时的内容".as_bytes());
        save_active(d.path(), &file).unwrap();
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/A1-standard.toml"),
            "被动过".as_bytes(),
        )
        .unwrap();

        let state = load_active(d.path()).unwrap().unwrap();
        assert!(
            !active_matches_disk(d.path(), &state),
            "盘上的字节漂了要看得见"
        );
    }
}
