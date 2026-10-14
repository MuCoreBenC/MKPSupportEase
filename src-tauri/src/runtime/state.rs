//! 状态的**数据形状**与纯判定：使用中指针、编辑中草稿这两格 section 的类型、
//! 两条线的落点解析与指纹比对，以及迁移用的旧档读取器。
//!
//! # 文件级的读写不在这里（2026-10-06 架构决策，见 `docs/APP-STATE.md`）
//!
//! 以前这个模块按「一种状态一个文件」管着 `run/active-preset.json` 与
//! `run/draft-preset.json` 两个文件；现在应用状态统一住 `run/app-state.json`，
//! **唯一读写入口是 [`super::app_state`]**。这里只剩三样东西：
//!
//! - section 的**数据形状**（[`ActivePreset`] / [`PresetDraft`] / [`DraftSubject`]）
//!   —— 字段就是原文件的字段，原样搬家，语义没变；
//! - **纯判定**（落点解析 [`active_target`]、指纹比对 [`active_matches_disk`]）；
//! - **旧档读取器**（[`read_active_file`] / [`read_draft_file`]）：迁移的读半边 ——
//!   `app-state.json` 还不在时，AppState 按格读旧文件拼快照。
//!
//! 旧规矩里活下来的三条：住 `run/` 下、原子写、坏档不静默（读出来解析不了就报
//! `CORRUPTED`，不把它当"没有"—— 静默吞状态是编数据的近亲：用户明明"使用中"，
//! 界面却说没在用，那是撒谎）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

/// 状态格式的代次。加字段不升号，改语义才升（与 [`super::catalog::CATALOG_SCHEMA`] 同一条）
pub const ACTIVE_SCHEMA: u32 = 1;

/// 迁移前的旧档（`run/active-preset.json`）。首次写成功后由 [`super::app_state`] 退役
pub(super) const ACTIVE_FILE: &str = "run/active-preset.json";

/// 使用中那一份**来自哪条线**。
///
/// 缺省是 [`ActiveOrigin::Preset`] —— 第七层之前写下的指针文件里没有这个字段，
/// 读出来就是它（那些文件记的确实都是云端预设文件，**语义没变，所以不升 schema**）。
///
/// ★★ **这里的 `Preset` 与列表那一套不是同一套**（2026-10-14 定名）。
/// 前端 `src/app/presets/presetTree.ts` 的 `PresetOrigin` 是**列表行**的分类，三值
/// `asset` / `preset` / `local`；这里两值，说的是**这两类里能成为"使用中"的那些** ——
/// 云端发布的 MKP 预设（列表的 `preset`）+ 用户自己那份（列表的 `local`）。
/// 资源文件（列表的 `asset`：切片器配置 json / 图标 / 模型）**从来没有"使用中"这回事**，
/// 所以它在这个枚举里没有对应物，别把两个枚举合成一个。
/// 界面上两者都显示「云端」/「本地」，所以**别靠名字猜它们是不是一回事**。
///
/// # 一个只读别名
///
/// `alias = "cloud"` **不是兼容层**（语义一个字没变，只是名字换了），而是"读得懂自己
/// 写下的旧值"：2026-10-14 把 `Cloud` 定名为 `Preset`，而在此之前写下的盘上值就是
/// `"cloud"`（更早那一刀把 `Official` 改名成 `Cloud`）。没有它，老 `app-state.json`
/// 整份解析失败，**所有读 AppState 的界面一起报「应用状态读不出来」**。
///
/// 它只管**读**：写回一律新名（`"preset"`），老盘那份在第一次写时自然归一。
/// 但**代次不能靠别名升**：`presets-mine/…` 那种旧落点要靠
/// [`crate::runtime::app_state`] 的 `1 → 2` 迁移改过来，别名管不了路径。
///
/// ★ `mine` 那条别名（`Mine` → `Local` 那一刀留下的）**已经退役**：它服务的是
/// 未发布阶段的测试盘，按作者 2026-10-14 的裁定不保留（`presets-mine/` 里的用户文件
/// 由 [`crate::runtime::local::adopt_legacy_dir`] 搬运，不靠这个别名读）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveOrigin {
    /// 云端预设线：目录（catalog）登记的 MKP 预设（落点 = `catalog.path`）
    #[default]
    #[serde(alias = "cloud")]
    Preset,
    /// 用户线：用户自己那份（`presets-local/…`）
    Local,
}

/// 使用中指针。全局唯一，`None` = 还没用任何一份（合法状态，不是错误）。
///
/// 指针里带着**应用时刻的 SHA**：这份 SHA 就是"文件没被动过"的凭证 ——
/// 盘上的字节漂了，界面能看出「文件已经不是当时应用的那份」。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePreset {
    pub active_schema: u32,
    /// 这一份住在哪条线上。**旧档没有这个字段 ⇒ 云端线**（见 [`ActiveOrigin`]）
    #[serde(default)]
    pub origin: ActiveOrigin,
    /// 文件名。云端线：catalog 的 `file_name` 就是键（**不存路径——路径是目录的职责**）；
    /// 用户线：它只是给人看的名字，落点看 `path`
    pub file_name: String,
    /// 应用时刻的指纹。拿它发现"文件已经不是当时那份"
    pub sha256: String,
    /// **用户线**的落点：相对**用户根**的路径（`presets-local/另存.toml`）。
    /// 云端线不写它 —— 落点由目录给。用户目录里可以自己分文件夹，所以这里必须存路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// 旧档读取器（迁移的读半边）：文件不存在 → `Ok(None)`；
/// 解析不了 → `Err(CORRUPTED)`（坏档不静默）
pub(super) fn read_active_file(root: &Path) -> Result<Option<ActivePreset>, AppError> {
    let bytes = match std::fs::read(root.join(ACTIVE_FILE)) {
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

/// 使用中那一份**现在在盘上的哪儿**（两条线各按自己的根解析），
/// 以及它是否还找得到（`None` = 找不到了）。
///
/// - 云端线：落点由**目录**给（`catalog.files` 里同名的那个 `path`）——
///   指针不存路径，所以目录换了布局、这份下线了，这里自然就找不到了；
/// - 用户线：落点在用户根下（`presets-local/…`），过一道防穿越闸。
pub fn active_target(
    internal_root: &Path,
    user_root: &Path,
    catalog: &super::Catalog,
    state: &ActivePreset,
) -> Option<PathBuf> {
    match state.origin {
        ActiveOrigin::Preset => {
            let file = catalog
                .files
                .iter()
                .find(|f| f.file_name == state.file_name)?;
            crate::fsx::paths::resolve_in(internal_root, &file.path).ok()
        }
        ActiveOrigin::Local => {
            let rel = state.path.as_deref()?;
            if !rel.starts_with(&format!("{LOCAL_DIR}/")) {
                return None; // 指针里记的不是用户根那一格：按"找不到"处理，不去猜
            }
            crate::fsx::paths::resolve_in(user_root, rel).ok()
        }
    }
}

use crate::fsx::paths::LOCAL_DIR;

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

/* ---------- 第二格：编辑中的那一份（临时文件账） ---------- */

/// 草稿格式的代次
pub const DRAFT_SCHEMA: u32 = 1;

/// 迁移前的旧档（`run/draft-preset.json`）。首次写成功后由 [`super::app_state`] 退役
pub(super) const DRAFT_FILE: &str = "run/draft-preset.json";

/// **编辑中的那一份**。全局唯一 —— 同一时刻只改一份（AppState 里写新的自然盖旧的）。
///
/// 它是"临时编辑"这条链的第一步（总纲 §1③）：
///
/// ```text
/// mkp/presets/A1-fast.toml     云端原件 —— 编辑全程**一动不动**
///        │ 点「改这份」：正文复制出来
///        ▼
/// run/app-state.json (draft)   临时文件账（用户改的是它；改到一半关掉也还在）
///        │ 点「保存为用户文件」
///        ▼
/// presets-local/A1-fast（已修改）.toml
/// ```
///
/// **为什么不跟云端原件同目录**（示意里那个 `A1MF_260701.tmp.toml`）：交付文件的落点
/// 由 `catalog.path` 定（A 类 `assets/…`、B 类 `delivery/mkp/presets/…`），它的判据是
/// "盘上每个文件都在目录里登记"（[`super::delivery::stale_files`] 就是靠这条
/// 认陈旧文件的）—— 往里塞一个 `.tmp`，它立刻变成"目录里没有的陈旧文件"，
/// 污染交付那一层的每一条判据。草稿是**运行状态**（"我正在改哪一份"），住 AppState。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraft {
    pub draft_schema: u32,
    /// 改的是**哪条线上**那一份（第八层起用户自己那份也能改）。
    /// 缺省是云端线 —— 第七层之前写下的草稿都是云端的，**语义没变，所以不升 schema**。
    #[serde(default)]
    pub origin: ActiveOrigin,
    /// 从哪一份改出来的。云端线是 `mkp/` 里的**文件名**（catalog 的键，不存路径）；
    /// 用户线是**给人看的文件名**（落点看 `path`）
    pub source_file_name: String,
    /// **用户线**的落点：相对**用户根**的路径（`presets-local/另存.toml`）。
    /// 云端线不写它 —— 落点由目录给
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 打开那一刻那一份的字节指纹（云端线 = 目录登记的 SHA；用户线 = 它的全文摘要）。
    ///
    /// 留着是为了回答两个问题：云端线"你改的是不是当时那一版"；用户线
    /// "这份文件在你编辑期间被外面换过没有"（**第九层**的事，这一层只如实记下来）
    pub source_sha256: String,
    /// 正文：用户改到哪算哪
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）。不引时间库，读时用
    pub updated_unix: u64,
}

/// 「编辑文件」那条链的草稿（**与上面 `PresetDraft` 分开存**，2026-10-10 作者裁决）。
///
/// 两条链可以同时存在：`PresetDraft` 是"参数页用控件在改"，这一份是"预设页编辑器逐字在改"。
/// 分开存的理由：互不干扰，也不互相覆盖彼此的正文。
///
/// 两条链**互相作废**：任一侧保存都会让另一侧手上的正文过期（盘上字节换了），
/// 所以保存一侧要顺手把另一侧丢掉（前端 `draftState.ts` 还为此发一次广播）。
///
/// 编辑器只服务**用户线**（`path` 恒有值）：云端那份要先拷成自己的才归用户改。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetTextDraft {
    pub draft_schema: u32,
    /// 「改的是哪一份」的人话名（`A1-fast（已修改）.toml`）
    pub file_name: String,
    /// 相对**用户根**的路径（`presets-local/A1-fast（已修改）.toml`）
    pub path: String,
    /// 正文：**带这一份自己的血统三行**（编辑这份时它们就在文件里，保存时照写回）
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）
    pub updated_unix: u64,
}

impl PresetTextDraft {
    /// 这条链的钥匙只认路径（用户目录里两份同名很正常）
    pub fn key(&self) -> &str {
        &self.path
    }
}

/// **这份草稿改的是哪一份**（两条线各自的钥匙）。
///
/// 云端线认**文件名**（目录的键）；用户线认**路径** —— 用户目录里两份文件同名很正常
/// （他自己分文件夹），只比文件名会把 A 的草稿接到 B 上。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftSubject {
    pub origin: ActiveOrigin,
    pub file_name: String,
    pub path: Option<String>,
}

impl DraftSubject {
    /// 云端预设线：目录里那一份（认文件名）
    pub fn preset(file_name: &str) -> Self {
        Self {
            origin: ActiveOrigin::Preset,
            file_name: file_name.to_owned(),
            path: None,
        }
    }

    /// 用户线：用户自己那份（认路径）
    pub fn local(file_name: &str, path: &str) -> Self {
        Self {
            origin: ActiveOrigin::Local,
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

/// 旧档读取器（迁移的读半边）：文件不存在 → `Ok(None)`；
/// 解析不了 → `Err(CORRUPTED)`（坏档不静默，与使用中指针同一条）
pub(super) fn read_draft_file(root: &Path) -> Result<Option<PresetDraft>, AppError> {
    let bytes = match std::fs::read(root.join(DRAFT_FILE)) {
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

use super::catalog::hex;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsx::atomic::atomic_write;
    use crate::runtime::catalog::CatalogFile;

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

    fn catalog_with(files: Vec<CatalogFile>) -> crate::runtime::Catalog {
        crate::runtime::Catalog {
            catalog_schema: crate::runtime::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            files,
            ..crate::runtime::Catalog::default()
        }
    }

    /// 坏档不静默：解析不了要响，不能装作"没有"（迁移的读半边同样守这条）
    #[test]
    fn corrupted_legacy_state_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(&d.path().join(ACTIVE_FILE), "{ 不是 JSON }".as_bytes()).unwrap();
        assert_eq!(
            read_active_file(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// 旧档（还没写 `origin` 的那批草稿）**就是云端线** —— 语义没变，所以不升 schema
    #[test]
    fn an_older_draft_file_is_still_the_cloud_line() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"draftSchema\": {DRAFT_SCHEMA}, \"sourceFileName\": \"A1-fast.toml\", \
             \"sourceSha256\": \"abc\", \"text\": \"涂胶 = 1\", \"updatedUnix\": 1 }}"
        );
        atomic_write(&d.path().join(DRAFT_FILE), json.as_bytes()).unwrap();

        let loaded = read_draft_file(d.path()).unwrap().expect("该读回来");
        assert_eq!(loaded.origin, ActiveOrigin::Preset);
        assert_eq!(loaded.path, None, "云端线不存路径 —— 落点由目录给");
    }

    /// `mine` 那条别名**已经退役**（作者 2026-10-14 裁定）：写 `"mine"` 的老指针
    /// 按坏档如实报错，**不静默重置**（旧值属于未发布阶段的测试盘；
    /// `presets-mine/` 里真有的用户文件由 [`crate::runtime::local::adopt_legacy_dir`] 搬，
    /// 不靠这个别名读）。
    #[test]
    fn a_retired_mine_origin_is_a_corrupted_file_not_a_silent_reset() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"activeSchema\": {ACTIVE_SCHEMA}, \"origin\": \"mine\", \
             \"fileName\": \"A1-fast.toml\", \"sha256\": \"abc\" }}"
        );
        atomic_write(&d.path().join(ACTIVE_FILE), json.as_bytes()).unwrap();

        let err = read_active_file(d.path()).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
        /* 原文件一个字节没动 —— 报错不改盘 */
        let still = std::fs::read_to_string(d.path().join(ACTIVE_FILE)).unwrap();
        assert!(still.contains("\"mine\""), "原文件该原样留着：{still}");
    }

    /// C 阶段（2026-10-14）把 `Cloud` 定名成 `Preset`（serde 名 `cloud` → `preset`）——
    /// **盘上更早写下的值还是 `"cloud"`，必须读得进**：那是当时真写下去的值，
    /// 没有这条别名就是又一轮"应用状态读不出来"（整份解析失败，读 AppState 的界面全报错）。
    #[test]
    fn an_older_cloud_origin_still_reads_as_preset() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let json = format!(
            "{{ \"activeSchema\": {ACTIVE_SCHEMA}, \"origin\": \"cloud\", \
             \"fileName\": \"A1-standard.toml\", \"sha256\": \"abc\" }}"
        );
        atomic_write(&d.path().join(ACTIVE_FILE), json.as_bytes()).unwrap();

        let loaded = read_active_file(d.path()).unwrap().expect("老档该读得进");
        assert_eq!(loaded.origin, ActiveOrigin::Preset);
        /* 别名只管读：写回一律新名 ⇒ 盘上那份下次真写入时自然归一 */
        let back = serde_json::to_string(&loaded).unwrap();
        assert!(
            back.contains("\"origin\":\"preset\""),
            "写出去要是新名：{back}"
        );
    }

    /// 只认**现在的名字 + 那条只读别名**，别的一律不认 ——
    /// 别把"读得懂自己写下的旧值"做成"什么字符串都收"。
    #[test]
    fn only_current_name_and_the_cloud_alias_are_accepted() {
        let parse = |s: &str| serde_json::from_str::<ActiveOrigin>(s);
        assert_eq!(parse("\"preset\"").unwrap(), ActiveOrigin::Preset);
        assert_eq!(parse("\"cloud\"").unwrap(), ActiveOrigin::Preset);
        assert_eq!(parse("\"local\"").unwrap(), ActiveOrigin::Local);
        assert!(
            parse("\"mine\"").is_err(),
            "`mine` 那条别名已退役（F 阶段删除；旧目录由 adopt_legacy_dir 搬）"
        );
        assert!(parse("\"official\"").is_err(), "旧得没边的名字不认");
        assert!(parse("\"asset\"").is_err(), "列表那一套的分类值不认");
        /* 写出去恒是新名（别名只参与读） */
        assert_eq!(
            serde_json::to_string(&ActiveOrigin::Preset).unwrap(),
            "\"preset\""
        );
        assert_eq!(
            serde_json::to_string(&ActiveOrigin::Local).unwrap(),
            "\"local\""
        );
    }

    #[test]
    fn drift_is_detected_when_disk_bytes_change() {
        let d = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("mkp/presets")).unwrap();
        let file = entry("A1-standard.toml", "当时的内容".as_bytes());
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/presets/A1-standard.toml"),
            "当时的内容".as_bytes(),
        )
        .unwrap();
        let state = ActivePreset {
            active_schema: ACTIVE_SCHEMA,
            origin: ActiveOrigin::Preset,
            file_name: file.file_name.clone(),
            sha256: file.sha256.clone().unwrap(),
            path: None,
        };
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/presets/A1-standard.toml"),
            "被动过".as_bytes(),
        )
        .unwrap();

        let catalog = catalog_with(vec![file]);
        assert!(
            !active_matches_disk(d.path(), user.path(), &catalog, &state),
            "盘上的字节漂了要看得见"
        );
    }

    /* ---------- 两条线都能成为使用中（第七层） ---------- */

    /// **落点由目录给**：云端线按 catalog 的 `path` 解析（`mkp/presets/…` 真布局），
    /// 不按文件名拼路径 —— 一份字节与目录一致的云端文件，在真布局下 intact 必须是 true
    #[test]
    fn a_preset_pointer_resolves_through_the_catalog() {
        let d = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("mkp/presets")).unwrap();
        let file = entry("A1-standard.toml", "云端当前版本".as_bytes());
        crate::fsx::atomic::atomic_write(
            &d.path().join("mkp/presets/A1-standard.toml"),
            "云端当前版本".as_bytes(),
        )
        .unwrap();
        let state = ActivePreset {
            active_schema: ACTIVE_SCHEMA,
            origin: ActiveOrigin::Preset,
            file_name: file.file_name.clone(),
            sha256: file.sha256.clone().unwrap(),
            path: None,
        };

        assert_eq!(state.origin, ActiveOrigin::Preset);
        assert!(state.path.is_none(), "云端线不存路径：那是目录的职责");
        let catalog = catalog_with(vec![file]);
        assert!(
            active_matches_disk(d.path(), user.path(), &catalog, &state),
            "按目录的 path 找到的那一份就是应用时那份"
        );
        assert_eq!(
            active_target(d.path(), user.path(), &catalog, &state).as_deref(),
            Some(d.path().join("mkp/presets/A1-standard.toml").as_path())
        );
    }
}
