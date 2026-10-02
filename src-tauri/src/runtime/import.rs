//! **通用文件导入入口**（第十二层）—— 「外部文件怎么安全地进入应用」。
//!
//! 这一层**不是**"实现 Preset 导入"，而是**入口本身**（作者 2026-10-02 定的边界）：
//! 拖拽 / 文件选择器把外部路径交进来，这里管好三件事，Preset 只是第一个消费者——
//! 以后「设置 → 备份与恢复」的 ZIP / 备份包继续复用这套接收机制：
//!
//! ```text
//! 外部文件 ──认领（导入器注册表）──▶ 落点检查（重名？）──▶ 复制进 presets-mine/
//!                 │                        │
//!             现在只有 Preset(.toml)      重名不覆盖 —— 名字由用户在界面上改
//!             以后 ZIP / 备份包往这加     （不自动改名）
//! ```
//!
//! # 边界（逐条定死）
//!
//! - **源文件只读**：不改、不删、不移；复制是"读出字节 → 原子写进用户根"；
//! - **落点固定 `presets-mine/`**，不给用户选目录（跨目录 = 文件夹管理，不在这层）；
//! - **内容按字节复制**：已有血统三行原样带过去，没有血统**允许导入、不编造来源**；
//! - **不校验 TOML 内容**：能不能当 Preset 用是后面 Preset 语义入口的事 ——
//!   导入只是"把外部文件纳入我的文件所有权范围"，**不是"安装 Preset"**；
//! - **不覆盖**：重名进改名流程，名字的门槛与第十层改名 / 第十一层另存为
//!   **同一套**（[`super::mine::check_new_name`]），`new_name` 由用户在界面上给；
//! - **一个状态都不碰**：不改使用中指针、不迁移 / 不创建草稿、不进 archive。
//!
//! 安全检查全部走现有用户文件边界（[`super::mine::check_mine_prefix`] 与
//! [`crate::fsx::paths::resolve_in`]）—— 这一层不新造第二套路径规则。

use std::path::{Path, PathBuf};

use crate::fsx::paths::{resolve_in, MINE_DIR};

use super::mine::{check_mine_prefix, check_new_name};

/// 认领这一份的导入器。**现在只有一个**：Preset（`.toml`）。
///
/// 以后 ZIP / 备份包在这里加变体，并各自决定"认领之后做什么"（恢复？解包？）；
/// 而"文件怎么进来、重名怎么办、边界在哪"这些**共用**本模块，不各写一套。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importer {
    /// MKP 预设（`.toml`）→ 复制进 `presets-mine/`
    Preset,
}

impl Importer {
    /// 注册表的稳定词（将来给日志 / 界面用）
    pub fn id(self) -> &'static str {
        match self {
            Importer::Preset => "preset",
        }
    }
}

/// 导入器注册表：认领一份文件（按文件名判）。认不了 = `None`。
///
/// **ZIP / 备份包现在不在这里** —— 所以它们不会被当成 Preset 复制进用户根
/// （作者定死：别让备份包提前污染 Preset 的文件语义）。
pub fn importer_of(file_name: &str) -> Option<Importer> {
    if file_name.to_ascii_lowercase().ends_with(".toml") {
        return Some(Importer::Preset);
    }
    None
}

/// 从外部路径取文件名（落点的默认名字）。取不出来 = 这个路径不是一份文件。
fn base_name(source: &str) -> Option<String> {
    Path::new(source)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
}

/// 落点检查的一档（`Ready` 之外都带一句原因，界面原话显示）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageState {
    /// 可以导入（按源文件名落进 `presets-mine/`）
    Ready,
    /// 落点已有同名（或这一批里前面已经占了这个名字）—— 界面进改名流程
    Collision,
    /// 收不了（没有认领它的导入器 / 源不是文件）—— `String` 是原因
    Rejected(String),
}

/// 一份外部文件的落点检查结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    /// 用户给的那个外部路径（原样带回，只为对号入座；界面不显示它）
    pub source: String,
    /// 源文件名
    pub file_name: String,
    pub state: StageState,
}

/// **第一段：看落点**。只检查，不动盘。
///
/// 重名不在这里解决（**不自动改名**）—— 只如实说"这儿已经有这个名字了"，
/// 名字由用户在界面上改（改完走 [`commit`] 的 `new_name`）。
pub fn stage(user_root: &Path, sources: &[String]) -> Vec<Staged> {
    let mut reserved: Vec<String> = Vec::new();
    sources
        .iter()
        .map(|source| {
            let Some(file_name) = base_name(source) else {
                return Staged {
                    source: source.clone(),
                    file_name: source.clone(),
                    state: StageState::Rejected(
                        "这个路径里没有文件名 —— 它不是一份能导入的文件".to_owned(),
                    ),
                };
            };
            if importer_of(&file_name).is_none() {
                return Staged {
                    source: source.clone(),
                    file_name,
                    state: StageState::Rejected(
                        "现在只收 .toml 预设 —— 这种文件还没有认领它的导入器（ZIP / 备份包以后再说）"
                            .to_owned(),
                    ),
                };
            }
            if !Path::new(source).is_file() {
                return Staged {
                    source: source.clone(),
                    file_name,
                    state: StageState::Rejected(
                        "找不到这个文件（或者它不是一个文件）".to_owned(),
                    ),
                };
            }
            if reserved.iter().any(|n| n == &file_name) || target_taken(user_root, &file_name) {
                return Staged {
                    source: source.clone(),
                    file_name,
                    state: StageState::Collision,
                };
            }
            reserved.push(file_name.clone());
            Staged {
                source: source.clone(),
                file_name,
                state: StageState::Ready,
            }
        })
        .collect()
}

/// 落点已经有东西了吗（含"那儿是个越界的符号链接"—— 那也不能往里写）
fn target_taken(user_root: &Path, file_name: &str) -> bool {
    let rel = format!("{MINE_DIR}/{file_name}");
    match resolve_in(user_root, &rel) {
        Ok(path) => path.exists(),
        /* 落点解析不过（越过用户根）：按"那儿有东西、不能覆盖"处理 —— 改名流程会换个名字 */
        Err(_) => true,
    }
}

/// 提交导入的一份：`new_name` 只在"重名、用户改了名"时给
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub source: String,
    pub new_name: Option<String>,
}

/// 逐份结局（与下载同一副规矩：一份出错不拖累别人）
#[derive(Debug, Clone)]
pub struct ImportOutcome {
    pub source: String,
    pub ok: bool,
    /// 成功 = 落进用户根之后的相对路径（`presets-mine/…`）；失败 = 空串
    pub path: String,
    pub file_name: String,
    /// 失败原因（可直接显示）；成功 = 空串
    pub message: String,
}

/// **第二段：真的复制**。逐份独立：一份出问题不影响别的份。
///
/// 源文件全程只读；写走全仓唯一那个出口（[`crate::fsx::atomic::atomic_write`]，
/// 它会把 `presets-mine/` 建出来）。
pub fn commit(user_root: &Path, items: &[ImportItem]) -> Vec<ImportOutcome> {
    items
        .iter()
        .map(|item| commit_one(user_root, item))
        .collect()
}

fn commit_one(user_root: &Path, item: &ImportItem) -> ImportOutcome {
    let fail = |file_name: String, message: String| ImportOutcome {
        source: item.source.clone(),
        ok: false,
        path: String::new(),
        file_name,
        message,
    };

    let Some(file_name) = base_name(&item.source) else {
        return fail(
            item.source.clone(),
            "这个路径里没有文件名 —— 它不是一份能导入的文件".to_owned(),
        );
    };
    if importer_of(&file_name).is_none() {
        return fail(
            file_name,
            "现在只收 .toml 预设 —— 这种文件还没有认领它的导入器".to_owned(),
        );
    }
    let name = match item.new_name.as_deref() {
        Some(raw) => match check_new_name(&file_name, raw) {
            Ok(name) => name,
            Err(e) => return fail(file_name, e.message),
        },
        None => file_name.clone(),
    };
    let rel = format!("{MINE_DIR}/{name}");
    if let Err(e) = check_mine_prefix(&rel) {
        return fail(name, e.message);
    }
    let target = match resolve_in(user_root, &rel) {
        Ok(path) => path,
        Err(e) => return fail(name, e.message),
    };
    if target.exists() {
        return fail(
            name.clone(),
            format!("已经有一份叫 {name} 的文件了 —— 换个名字（这里不覆盖）"),
        );
    }
    let source = PathBuf::from(&item.source);
    if !source.is_file() {
        return fail(name, "找不到源文件 —— 它可能已经被移走或删掉".to_owned());
    }
    let bytes = match std::fs::read(&source) {
        Ok(bytes) => bytes,
        Err(e) => return fail(name, format!("源文件读不出来：{e}")),
    };
    if let Err(e) = crate::fsx::atomic::atomic_write(&target, &bytes) {
        return fail(name, format!("写不进用户根：{}", e.message));
    }
    ImportOutcome {
        source: item.source.clone(),
        ok: true,
        path: rel,
        file_name: name,
        message: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份"外部"文件（临时目录里的，就是"用户桌面上的那一份"）
    fn write(dir: &Path, rel: &str, text: &str) {
        crate::fsx::atomic::atomic_write(&dir.join(rel), text.as_bytes()).unwrap();
    }

    fn src_of(dir: &Path, name: &str) -> String {
        dir.join(name).to_string_lossy().into_owned()
    }

    const VALID_TOML: &str = "[toolhead]\noffset_x = 1.0\n";

    /// 落点检查：能收的 ready、重名的 collision、收不了的 rejected（各带原因）；
    /// 同一批里两份同名 —— 第二份也算重名（前面那份已经占了名字）
    #[test]
    fn staging_says_ready_collision_and_rejected() {
        let user = tempfile::tempdir().unwrap();
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        write(user.path(), "presets-mine/已有.toml", VALID_TOML);
        write(d1.path(), "新的.toml", VALID_TOML);
        write(d1.path(), "已有.toml", VALID_TOML);
        write(d2.path(), "新的.toml", VALID_TOML);
        write(d1.path(), "备份.zip", "PK");

        let got = stage(
            user.path(),
            &[
                src_of(d1.path(), "新的.toml"),
                src_of(d1.path(), "已有.toml"),
                src_of(d2.path(), "新的.toml"),
                src_of(d1.path(), "备份.zip"),
                "/不存在的目录/不存在.toml".to_owned(),
                "/".to_owned(),
            ],
        );
        assert_eq!(got[0].state, StageState::Ready);
        assert_eq!(got[1].state, StageState::Collision, "落点已有同名");
        assert_eq!(
            got[2].state,
            StageState::Collision,
            "同一批里前面那份已占名"
        );
        assert!(
            matches!(&got[3].state, StageState::Rejected(r) if r.contains(".toml")),
            "{:?}",
            got[3].state
        );
        assert!(
            matches!(&got[4].state, StageState::Rejected(r) if r.contains("找不到")),
            "{:?}",
            got[4].state
        );
        assert!(
            matches!(&got[5].state, StageState::Rejected(r) if r.contains("没有文件名")),
            "{:?}",
            got[5].state
        );
    }

    /// 导入是**字节复制**：目标与源逐字节相同；**源文件一个字节不动**
    #[test]
    fn importing_copies_the_bytes_and_leaves_the_source_alone() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        let with_lineage = "# based_on: mkp/presets/A1-standard.toml\n[toolhead]\noffset_x = 1.0\n";
        write(ext.path(), "外部 A1.toml", with_lineage);

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "外部 A1.toml"),
                new_name: None,
            }],
        );
        assert!(outcomes[0].ok, "{}", outcomes[0].message);
        assert_eq!(outcomes[0].path, "presets-mine/外部 A1.toml");
        let target = user.path().join("presets-mine/外部 A1.toml");
        assert_eq!(
            std::fs::read(&target).unwrap(),
            with_lineage.as_bytes(),
            "字节复制"
        );
        assert_eq!(
            std::fs::read(ext.path().join("外部 A1.toml")).unwrap(),
            with_lineage.as_bytes(),
            "源文件只读：一个字节不动"
        );

        /* 进来之后就是一份正常的用户文件：能列出来、血统原样（没有就不编造） */
        let listed = super::super::mine::mine_files(user.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].state, Some(super::super::mine::MineState::Ok));
        assert_eq!(
            format!("{:?}", listed[0].lineage),
            format!(
                "{:?}",
                super::super::mine::lineage_of_file(&ext.path().join("外部 A1.toml"))
            ),
            "血统原样带过去"
        );
    }

    /// 没有血统的那份：**允许导入、不编造来源**（血统就是"没有"）
    #[test]
    fn importing_does_not_invent_a_lineage() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(ext.path(), "光秃秃.toml", VALID_TOML);

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "光秃秃.toml"),
                new_name: None,
            }],
        );
        assert!(outcomes[0].ok, "{}", outcomes[0].message);
        let listed = super::super::mine::mine_files(user.path());
        assert_eq!(listed[0].lineage, None, "不编造来源");
    }

    /// **不校验 TOML 内容**：语法坏的那份照样进得来（能不能当 Preset 用是语义入口的事）
    #[test]
    fn importing_does_not_reject_invalid_toml() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(ext.path(), "坏的.toml", "[toolhead]\noffset_x = (1");

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "坏的.toml"),
                new_name: None,
            }],
        );
        assert!(outcomes[0].ok, "导入不看内容：{}", outcomes[0].message);
        let listed = super::super::mine::mine_files(user.path());
        assert_eq!(
            listed[0].state,
            Some(super::super::mine::MineState::Unreadable),
            "进来之后，第九层如实说它读不出来"
        );
    }

    /// **不覆盖**：落点已有同名就拒；被撞的那份与源都一个字节没动
    #[test]
    fn importing_never_overwrites() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(
            user.path(),
            "presets-mine/占位.toml",
            "[wiping]\nspeed = 80\n",
        );
        write(ext.path(), "占位.toml", VALID_TOML);

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "占位.toml"),
                new_name: None,
            }],
        );
        assert!(!outcomes[0].ok);
        assert!(
            outcomes[0].message.contains("不覆盖"),
            "{}",
            outcomes[0].message
        );
        assert_eq!(
            std::fs::read(user.path().join("presets-mine/占位.toml")).unwrap(),
            "[wiping]\nspeed = 80\n".as_bytes(),
            "被撞的那份一个字节没动"
        );
        assert_eq!(
            std::fs::read(ext.path().join("占位.toml")).unwrap(),
            VALID_TOML.as_bytes()
        );
    }

    /// 重名改名走**同一套名字门槛**（空 / 路径 / 后缀），改对了就落那个名字
    #[test]
    fn importing_uses_the_same_name_gate_for_a_new_name() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(ext.path(), "源.toml", VALID_TOML);

        for bad in ["换.json", "子目录/源.toml", "..", "   "] {
            let outcomes = commit(
                user.path(),
                &[ImportItem {
                    source: src_of(ext.path(), "源.toml"),
                    new_name: Some(bad.to_owned()),
                }],
            );
            assert!(!outcomes[0].ok, "{bad} 该被拒");
            assert!(
                outcomes[0].message.contains("名字")
                    || outcomes[0].message.contains("后缀")
                    || outcomes[0].message.contains("文件名"),
                "{}",
                outcomes[0].message
            );
        }

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "源.toml"),
                new_name: Some("改名的.toml".to_owned()),
            }],
        );
        assert!(outcomes[0].ok, "{}", outcomes[0].message);
        assert_eq!(outcomes[0].path, "presets-mine/改名的.toml");
    }

    /// **一个状态都不碰**：使用中指针与草稿都还指着原来那一份
    #[test]
    fn importing_touches_no_state() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(user.path(), "presets-mine/A1.toml", VALID_TOML);
        write(ext.path(), "外部.toml", VALID_TOML);
        let active =
            super::super::state::save_active_mine(user.path(), "presets-mine/A1.toml", "sha")
                .unwrap();
        let subject = super::super::state::DraftSubject::mine("A1.toml", "presets-mine/A1.toml");
        super::super::state::save_draft(user.path(), &subject, "sha", "改到一半").unwrap();

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "外部.toml"),
                new_name: None,
            }],
        );
        assert!(outcomes[0].ok, "{}", outcomes[0].message);

        let after_active = super::super::state::load_active(user.path())
            .unwrap()
            .unwrap();
        assert_eq!(after_active.path, active.path, "使用中没动");
        let after_draft = super::super::state::load_draft(user.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            after_draft.path.as_deref(),
            Some("presets-mine/A1.toml"),
            "草稿没被动"
        );
    }

    /// 用户根里 `presets-mine/` 还不存在（全新安装）：导入把它建出来
    #[test]
    fn importing_creates_the_mine_dir_when_missing() {
        let user = tempfile::tempdir().unwrap();
        let ext = tempfile::tempdir().unwrap();
        write(ext.path(), "头一份.toml", VALID_TOML);

        let outcomes = commit(
            user.path(),
            &[ImportItem {
                source: src_of(ext.path(), "头一份.toml"),
                new_name: None,
            }],
        );
        assert!(outcomes[0].ok, "{}", outcomes[0].message);
        assert!(user.path().join("presets-mine/头一份.toml").exists());
    }
}
