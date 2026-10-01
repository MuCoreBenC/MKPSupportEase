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
//! [`ActivePreset`] 记"当前使用的是哪一份"。全局唯一——
//! 产品规则定的「同一时刻只能有一份处于已应用状态」，在构造上成立：
//! 状态就一个文件，写新的自然盖旧的（原子替换，没有中间态）。
//!
//! 指针里带着**应用时刻的 SHA**：这份 SHA 就是"文件没被动过"的凭证 ——
//! 盘上的字节漂了，界面能看出「文件已经不是当时应用的那份」。
//!
//! # 两条线都能进来（第七层，2026-10-02 作者定）
//!
//! ```text
//! 官方线  云端 → mkp/…        只读，只有"云端换版本"能替换它   ┐
//!                                                          ├─ 都能成为使用中
//! 用户线  另存 → presets-mine/ 用户自己可改，不属任何官方版本  ┘
//! ```
//!
//! **"只读"是文件归属的属性，不是"能不能被使用"的属性**（作者原话）。
//! 所以指针多了 [`ActiveOrigin`]：它只说"这一份住在哪条线上"，**不是两套 Preset 模型** ——
//! 两条线的落点解析各按自己的根来（[`active_target`]），其余（唯一性、指纹、撤销）一模一样。
//!
//! **两条线上"字节漂了"的意思不一样**：官方线是"它不是我们交付的那一版了"（可疑，见
//! [`super::delivery::FileTrust`]）；用户线是"用户自己又改了它"（正常 —— 那份是他的）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::MINE_DIR;

use super::catalog::hex;
use super::catalog::CatalogFile;

/// 状态格式的代次。加字段不升号，改语义才升（与 [`super::catalog::CATALOG_SCHEMA`] 同一条）
pub const ACTIVE_SCHEMA: u32 = 1;

const ACTIVE_FILE: &str = "run/active-preset.json";

/// 使用中那一份**来自哪条线**。
///
/// 缺省是 [`ActiveOrigin::Official`] —— 第七层之前写下的指针文件里没有这个字段，
/// 读出来就是它（那些文件记的确实都是官方交付文件，**语义没变，所以不升 schema**）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveOrigin {
    /// 官方线：目录（catalog）登记的交付文件（`mkp/…`）
    #[default]
    Official,
    /// 用户线：用户自己那份（`presets-mine/…`）
    Mine,
}

/// 使用中指针。全局唯一，`None` = 还没用任何一份（合法状态，不是错误）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePreset {
    pub active_schema: u32,
    /// 这一份住在哪条线上。**旧档没有这个字段 ⇒ 官方线**（见 [`ActiveOrigin`]）
    #[serde(default)]
    pub origin: ActiveOrigin,
    /// 文件名。官方线：catalog 的 `file_name` 就是键（**不存路径——路径是目录的职责**）；
    /// 用户线：它只是给人看的名字，落点看 `path`
    pub file_name: String,
    /// 应用时刻的指纹。拿它发现"文件已经不是当时那份"
    pub sha256: String,
    /// **用户线**的落点：相对**用户根**的路径（`presets-mine/我的/另存.toml`）。
    /// 官方线不写它 —— 落点由目录给。用户目录里可以自己分文件夹，所以这里必须存路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
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

/// 记下"用这一份官方的"。`file` 来自 catalog（名字与 SHA 都是目录登记的），整份替换旧的。
pub fn save_active(root: &Path, file: &CatalogFile) -> Result<ActivePreset, AppError> {
    write_active(
        root,
        ActivePreset {
            active_schema: ACTIVE_SCHEMA,
            origin: ActiveOrigin::Official,
            file_name: file.file_name.clone(),
            sha256: file.sha256.clone(),
            path: None,
        },
    )
}

/// 记下"用我自己那一份"（`presets-mine/…`）。`rel` 是**相对用户根**的路径（[`super::mine`]
/// 那一套口径），`sha256` 是应用那一刻它的字节摘要（用户后来自己改它就是"漂了"，
/// 见模块头：那条对用户线是正常的，不是可疑）。
pub fn save_active_mine(root: &Path, rel: &str, sha256: &str) -> Result<ActivePreset, AppError> {
    let file_name = Path::new(rel)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_owned());
    write_active(
        root,
        ActivePreset {
            active_schema: ACTIVE_SCHEMA,
            origin: ActiveOrigin::Mine,
            file_name,
            sha256: sha256.to_owned(),
            path: Some(rel.to_owned()),
        },
    )
}

fn write_active(root: &Path, state: ActivePreset) -> Result<ActivePreset, AppError> {
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

/// 使用中那一份**现在在盘上的哪儿**（两条线各按自己的根解析），
/// 以及它是否还找得到（`None` = 找不到了）。
///
/// - 官方线：落点由**目录**给（`catalog.files` 里同名的那个 `path`）——
///   指针不存路径，所以目录换了布局、这份下线了，这里自然就找不到了；
/// - 用户线：落点在用户根下（`presets-mine/…`），过一道防穿越闸。
pub fn active_target(
    internal_root: &Path,
    user_root: &Path,
    catalog: &super::Catalog,
    state: &ActivePreset,
) -> Option<PathBuf> {
    match state.origin {
        ActiveOrigin::Official => {
            let file = catalog
                .files
                .iter()
                .find(|f| f.file_name == state.file_name)?;
            crate::fsx::paths::resolve_in(internal_root, &file.path).ok()
        }
        ActiveOrigin::Mine => {
            let rel = state.path.as_deref()?;
            if !rel.starts_with(&format!("{MINE_DIR}/")) {
                return None; // 指针里记的不是用户根那一格：按"找不到"处理，不去猜
            }
            crate::fsx::paths::resolve_in(user_root, rel).ok()
        }
    }
}

/// 应用时刻的指纹 → 当下的盘。`Ok(true)` = 文件还是当时那份。
///
/// 找不到那一份（目录里没有了 / 用户把它删了）也算"漂了" —— 指针指向的东西不存在了。
pub fn active_matches_disk(
    internal_root: &Path,
    user_root: &Path,
    catalog: &super::Catalog,
    state: &ActivePreset,
) -> bool {
    match active_target(internal_root, user_root, catalog, state) {
        None => false,
        Some(target) => match std::fs::read(&target) {
            Ok(bytes) => hex(&Sha256::digest(&bytes)) == state.sha256,
            Err(_) => false,
        },
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
    /// 改的是**哪条线上**那一份（第八层起用户自己那份也能改）。
    /// 缺省是官方线 —— 第七层之前写下的草稿都是官方的，**语义没变，所以不升 schema**。
    #[serde(default)]
    pub origin: ActiveOrigin,
    /// 从哪一份改出来的。官方线是 `mkp/` 里的**文件名**（catalog 的键，不存路径）；
    /// 用户线是**给人看的文件名**（落点看 `path`）
    pub source_file_name: String,
    /// **用户线**的落点：相对**用户根**的路径（`presets-mine/我的/另存.toml`）。
    /// 官方线不写它 —— 落点由目录给
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 打开那一刻那一份的字节指纹（官方线 = 目录登记的 SHA；用户线 = 它的全文摘要）。
    ///
    /// 留着是为了回答两个问题：官方线"你改的是不是当时那一版"；用户线
    /// "这份文件在你编辑期间被外面换过没有"（**第九层**的事，这一层只如实记下来）
    pub source_sha256: String,
    /// 正文：用户改到哪算哪
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）。不引时间库，读时用
    pub updated_unix: u64,
}

/// **这份草稿改的是哪一份**（两条线各自的钥匙）。
///
/// 官方线认**文件名**（目录的键）；用户线认**路径** —— 用户目录里两份文件同名很正常
/// （他自己分文件夹），只比文件名会把 A 的草稿接到 B 上。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftSubject {
    pub origin: ActiveOrigin,
    pub file_name: String,
    pub path: Option<String>,
}

impl DraftSubject {
    /// 官方线：目录里那一份（认文件名）
    pub fn official(file_name: &str) -> Self {
        Self {
            origin: ActiveOrigin::Official,
            file_name: file_name.to_owned(),
            path: None,
        }
    }

    /// 用户线：用户自己那份（认路径）
    pub fn mine(file_name: &str, path: &str) -> Self {
        Self {
            origin: ActiveOrigin::Mine,
            file_name: file_name.to_owned(),
            path: Some(path.to_owned()),
        }
    }

    /// 是不是同一份（两条线各自的键，见类型注释）
    pub fn matches(&self, other: &Self) -> bool {
        self == other
    }
}

impl PresetDraft {
    /// 这份草稿改的是哪一份
    pub fn subject(&self) -> DraftSubject {
        DraftSubject {
            origin: self.origin,
            file_name: self.source_file_name.clone(),
            path: self.path.clone(),
        }
    }
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
    subject: &DraftSubject,
    source_sha256: &str,
    text: &str,
) -> Result<PresetDraft, AppError> {
    let draft = PresetDraft {
        draft_schema: DRAFT_SCHEMA,
        origin: subject.origin,
        source_file_name: subject.file_name.clone(),
        path: subject.path.clone(),
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
            /* 真实布局：交付根相对路径 = 客户端落点（都是 `mkp/presets/…`） */
            path: format!("mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: hex(&Sha256::digest(content)),
            size: content.len() as u64,
        }
    }

    fn catalog_with(files: Vec<CatalogFile>) -> crate::runtime::Catalog {
        crate::runtime::Catalog {
            catalog_schema: crate::runtime::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            files,
            ..crate::runtime::Catalog::default()
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
        let subject = DraftSubject::official("A1-fast.toml");
        let saved = save_draft(d.path(), &subject, "abc", "涂胶 = 1.2").unwrap();
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

    /// 两条线的钥匙不同：用户目录里两份文件同名很正常（他自己分文件夹），
    /// 只比文件名会把 A 的草稿接到 B 上
    #[test]
    fn a_draft_knows_which_line_it_belongs_to() {
        let d = tempfile::tempdir().unwrap();
        save_draft(
            d.path(),
            &DraftSubject::official("A1-fast.toml"),
            "a",
            "官方",
        )
        .unwrap();
        let mine = DraftSubject::mine("A1-fast.toml", "presets-mine/我的/A1-fast.toml");
        save_draft(d.path(), &mine, "b", "我的").unwrap();

        let loaded = load_draft(d.path()).unwrap().expect("该读回来");
        assert_eq!(loaded.text, "我的");
        assert_eq!(loaded.subject(), mine, "改的是我自己那份");
        assert!(
            !loaded
                .subject()
                .matches(&DraftSubject::official("A1-fast.toml")),
            "同名不同线 ≠ 同一份：「接着改」不许接错"
        );
    }

    /// 旧档（还没写 `origin` 的那批草稿）**就是官方线** —— 语义没变，所以不升 schema
    #[test]
    fn an_older_draft_file_is_still_the_official_line() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"draftSchema\": {DRAFT_SCHEMA}, \"sourceFileName\": \"A1-fast.toml\", \
             \"sourceSha256\": \"abc\", \"text\": \"涂胶 = 1\", \"updatedUnix\": 1 }}"
        );
        crate::fsx::atomic::atomic_write(&draft_file(d.path()), json.as_bytes()).unwrap();

        let loaded = load_draft(d.path()).unwrap().expect("该读回来");
        assert_eq!(loaded.origin, ActiveOrigin::Official);
        assert_eq!(loaded.path, None, "官方线不存路径 —— 落点由目录给");
    }

    /// 草稿与使用中指针**两个文件、互不干扰**：改一份不等于在用它
    #[test]
    fn draft_and_active_are_separate_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        save_active(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        save_draft(
            d.path(),
            &DraftSubject::official("A1-fast.toml"),
            "x",
            "涂胶 = 2",
        )
        .unwrap();

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
        let user = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("mkp/presets")).unwrap();
        let file = entry("A1-standard.toml", "当时的内容".as_bytes());
        save_active(d.path(), &file).unwrap();
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/presets/A1-standard.toml"),
            "被动过".as_bytes(),
        )
        .unwrap();

        let catalog = catalog_with(vec![file]);
        let state = load_active(d.path()).unwrap().unwrap();
        assert!(
            !active_matches_disk(d.path(), user.path(), &catalog, &state),
            "盘上的字节漂了要看得见"
        );
    }

    /* ---------- 两条线都能成为使用中（第七层） ---------- */

    /// **落点由目录给**：官方线按 catalog 的 `path` 解析（`mkp/presets/…` 真布局），
    /// 不按文件名拼路径 —— 一份字节与目录一致的官方文件，在真布局下 intact 必须是 true
    #[test]
    fn the_official_pointer_resolves_through_the_catalog() {
        let d = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("mkp/presets")).unwrap();
        let file = entry("A1-standard.toml", "官方当前版本".as_bytes());
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/presets/A1-standard.toml"),
            "官方当前版本".as_bytes(),
        )
        .unwrap();
        save_active(d.path(), &file).unwrap();
        let state = load_active(d.path()).unwrap().unwrap();

        assert_eq!(state.origin, ActiveOrigin::Official);
        assert!(state.path.is_none(), "官方线不存路径：那是目录的职责");
        let catalog = catalog_with(vec![file]);
        assert!(
            active_matches_disk(d.path(), user.path(), &catalog, &state),
            "按目录的 path 找到的那一份就是应用时那份"
        );
        assert_eq!(
            active_target(d.path(), user.path(), &catalog, &state).as_deref(),
            Some(d.path().join("mkp/presets/A1-standard.toml").as_path())
        );

        /* 目录里已经没有它了（下线 / 换源）：找不到 ⇒ 漂了。不猜一个路径出来 */
        assert!(!active_matches_disk(
            d.path(),
            user.path(),
            &catalog_with(Vec::new()),
            &state
        ));
    }

    /// 旧档（第七层之前写下的、没有 `origin` 字段）读出来就是官方线 —— **语义没变，不升 schema**
    #[test]
    fn an_older_pointer_file_still_means_the_official_line() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"activeSchema\": {ACTIVE_SCHEMA}, \"fileName\": \"A1-standard.toml\", \"sha256\": \"y\" }}"
        );
        crate::fsx::atomic::atomic_write(&active_file(d.path()), json.as_bytes()).unwrap();

        let state = load_active(d.path()).unwrap().unwrap();
        assert_eq!(state.origin, ActiveOrigin::Official);
        assert_eq!(state.path, None);
    }

    /// **用户自己那份也能是使用中的那一份**：落点在用户根（可以带子目录），
    /// 指纹按它的字节算；它后来被用户改了 → "漂了"（对用户线这是正常事，不是可疑）
    #[test]
    fn the_users_own_copy_can_be_the_active_one() {
        let d = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(user.path().join("presets-mine/我的")).unwrap();
        let target = user.path().join("presets-mine/我的/另存.toml");
        crate::fsx::atomic::atomic_write(&target, "涂胶 = 1.4".as_bytes()).unwrap();
        let sha = hex(&Sha256::digest("涂胶 = 1.4".as_bytes()));

        let saved = save_active_mine(d.path(), "presets-mine/我的/另存.toml", &sha).unwrap();
        assert_eq!(saved.origin, ActiveOrigin::Mine);
        assert_eq!(saved.file_name, "另存.toml", "文件名只是给人看的");
        assert_eq!(saved.path.as_deref(), Some("presets-mine/我的/另存.toml"));

        let catalog = catalog_with(Vec::new());
        let loaded = load_active(d.path()).unwrap().unwrap();
        assert!(
            active_matches_disk(d.path(), user.path(), &catalog, &loaded),
            "用户线的落点在用户根下，与目录无关"
        );

        /* 用户自己又改了它：指针还是那一份，但字节已经不是当时那份 */
        crate::fsx::atomic::atomic_write(&target, "涂胶 = 1.5".as_bytes()).unwrap();
        assert!(!active_matches_disk(
            d.path(),
            user.path(),
            &catalog,
            &loaded
        ));
    }

    /// 指针里记的路径不是用户根那一格 ⇒ 按"找不到"处理（不去猜、不去别处找）
    #[test]
    fn a_pointer_pointing_outside_the_mine_dir_is_not_resolved() {
        let d = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let mut stray = save_active_mine(d.path(), "presets-mine/x.toml", "abc").unwrap();
        stray.path = Some("../secret.toml".to_owned());

        assert!(active_target(d.path(), user.path(), &catalog_with(Vec::new()), &stray).is_none());
    }

    /// 全局唯一对两条线同样成立：先应用官方的，再改成自己那份，指针里只剩后者
    #[test]
    fn applying_one_line_replaces_the_other() {
        let d = tempfile::tempdir().unwrap();
        save_active(d.path(), &entry("A1-standard.toml", b"a")).unwrap();
        save_active_mine(d.path(), "presets-mine/我的/另存.toml", "sha").unwrap();

        let state = load_active(d.path()).unwrap().expect("该有一份");
        assert_eq!(state.origin, ActiveOrigin::Mine);
        assert_eq!(state.file_name, "另存.toml");
    }
}
