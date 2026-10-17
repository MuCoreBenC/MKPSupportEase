//! **修改历史账本**（2026-10-17）：一份文件一本，git 式 —— 不清空、重开软件还在。
//!
//! # 为什么不再是"会话内的内存日志"
//!
//! 原来参数弹窗那本历史只活在 React 的 state 里：关掉弹窗、切个组合、退出软件，
//! 它就没了（作者 2026-10-17：「修改历史是类似 git 的保存的那种形式吧，不要清空，
//! 每次重新打开软件都存在」）。于是它得落到盘上，而且**按文件存** ——
//! 因为"我改过什么"这件事属于那一份文件，不属于这一屏、也不属于这一次会话。
//!
//! ```text
//! <appDataDir>/run/preset-history/<指纹>.json
//!        ▲
//!        │ 整份读-改-写（atomic_write_json）
//!        │
//!    history.rs（唯一读写入口）
//! ```
//!
//! # 一把钥匙认一份文件
//!
//! 钥匙 = `origin + 身份` 的 sha256：
//!
//! - **用户线（local）**：身份是**相对用户根的路径**（`presets-local/A1 涂胶.toml`）——
//!   用户目录里同名文件很正常（他自己分文件夹），只比文件名会把 A 的历史接到 B 上；
//! - **云端线（preset）**：身份是**文件名**（落点由目录给，那一刻还没有用户路径）。
//!
//! ★ 于是**导入的 toml 也有自己的一本**（它进来之后就是用户线的一份文件）：
//! "不管导入的还是什么的"这条口径由钥匙的形状保证，不需要额外开关。
//!
//! # 规矩（照 [`super::app_state`] 那四条）
//!
//! 1. 一本账一份文件，键是上面那把钥匙；
//! 2. 住在内部根 `run/` 下（用户不需要翻它，它跟 AppState 同层）；
//! 3. 写走 [`crate::fsx::atomic::atomic_write_json`]（全仓唯一写盘出口）；
//! 4. 坏档不静默 —— 解析不了报 `CORRUPTED`，不清空、不猜。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;

/// 历史文件格式的代次。加字段不升号，改语义才升（与 catalog / app-state 同一条）
pub const HISTORY_SCHEMA: u32 = 1;

/// 账本目录（相对内部根）
const HISTORY_DIR: &str = "run/preset-history";

/// 一条改动里的**一项**（一个参数）。
///
/// 字段形状与前端 `HistoryItem`（`src/app/params/useParams.ts`）一一对应 ——
/// 前后端共用同一套字面量，改名字要两边一起改。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub key: String,
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(default)]
    pub tab: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub unit: Option<String>,
}

/// 一条改动（一次动作）。`state` 与前端同一个枚举的字面量
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub no: i64,
    pub action: String,
    /// 发生那一刻（UTC epoch 秒）。持久化之后才有的字段 —— 内存日志时代不需要它
    #[serde(default)]
    pub ts_unix: i64,
    pub state: String,
    /// 第几次保存把它落地的；`null` = 还没保存过
    #[serde(default)]
    pub batch: Option<u32>,
    pub items: Vec<HistoryItem>,
}

/// 一份文件的整本历史
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFile {
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// 这一份保存过几次（批号的水位；与 `HistoryEntry.batch` 同一套数）
    #[serde(default)]
    pub batches: u32,
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}

fn default_schema() -> u32 {
    HISTORY_SCHEMA
}

/// 账本目录的绝对路径
pub fn history_dir(root: &Path) -> PathBuf {
    root.join(HISTORY_DIR)
}

/// **一把钥匙认一份文件**（见模块头）：`origin + 身份` 的 sha256（hex）。
///
/// 不把身份直接当文件名用：用户文件名里有中文 / 空格 / 括号，三个平台的文件系统
/// 对它们的容忍度不一样；摘要是纯 ASCII，落盘没有第二种写法。
pub fn key_of(origin: &str, id: &str) -> String {
    let mut h = Sha256::new();
    h.update(origin.as_bytes());
    h.update(b":");
    h.update(id.as_bytes());
    crate::runtime::catalog::hex(&h.finalize())
}

fn file_of(root: &Path, key: &str) -> PathBuf {
    history_dir(root).join(format!("{key}.json"))
}

/// 读一本。**没写过 = 空本**（不是错误）—— 第一次改某份文件时本来就没有历史。
pub fn read(root: &Path, key: &str) -> Result<HistoryFile, AppError> {
    let at = file_of(root, key);
    let Ok(bytes) = std::fs::read(&at) else {
        return Ok(HistoryFile::default());
    };
    serde_json::from_slice::<HistoryFile>(&bytes).map_err(|e| {
        AppError::corrupted("修改历史那本账读不出来（文件坏了，不是没有）")
            .with_detail(e.to_string())
    })
}

/// 写一本（整份替换，原子）。
///
/// ★ **空本就删掉文件**：界面上"没有改动记录"与"盘上没有这本账"是同一件事，
///   留一个 0 条的文件只是让人以为清掉了还有残留。
pub fn write(root: &Path, key: &str, file: &HistoryFile) -> Result<(), AppError> {
    let at = file_of(root, key);
    if file.entries.is_empty() && file.batches == 0 {
        if at.is_file() {
            std::fs::remove_file(&at)
                .map_err(|e| AppError::io("清不掉这本历史").with_detail(e.to_string()))?;
        }
        return Ok(());
    }
    atomic_write_json(&at, file)
}

/// **清空一本**（抽屉右上那颗「清空历史」）。删掉文件就是清干净了
pub fn clear(root: &Path, key: &str) -> Result<(), AppError> {
    write(root, key, &HistoryFile::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> HistoryItem {
        HistoryItem {
            key: "toolhead.offset_y".to_owned(),
            from: "22.4".to_owned(),
            to: "1.23".to_owned(),
            label: "Y 轴偏移".to_owned(),
            tab: "偏移".to_owned(),
            section: "空间偏移".to_owned(),
            unit: Some("mm".to_owned()),
        }
    }

    /// 没写过 = 空本，不是错误（第一次改某份文件就是这种）
    #[test]
    fn a_never_written_ledger_is_empty_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        let file = read(d.path(), &key_of("local", "presets-local/A1 涂胶.toml")).unwrap();
        assert_eq!(file.entries.len(), 0);
        assert_eq!(file.batches, 0);
    }

    /// **钥匙按"哪条线 + 身份"分开**：用户目录里两份同名文件各有各的一本
    /// （只比文件名会把 A 的历史接到 B 上）
    #[test]
    fn the_key_separates_two_files_with_the_same_name() {
        let d = tempfile::tempdir().unwrap();
        let a = key_of("local", "presets-local/fast/A1.toml");
        let b = key_of("local", "presets-local/slow/A1.toml");
        assert_ne!(a, b, "同名不同路径必须是两本账");

        let mut fa = HistoryFile::default();
        fa.entries.push(HistoryEntry {
            id: 1,
            no: 1,
            action: "改 Y 轴偏移".to_owned(),
            ts_unix: 1,
            state: "draft".to_owned(),
            batch: None,
            items: vec![item()],
        });
        write(d.path(), &a, &fa).unwrap();

        assert_eq!(read(d.path(), &a).unwrap().entries.len(), 1);
        assert_eq!(
            read(d.path(), &b).unwrap().entries.len(),
            0,
            "另一份同名文件不该看到它"
        );
        /* 云端线与用户线即使名字相同也是两本 */
        assert_ne!(key_of("preset", "A1.toml"), a);
    }

    /// 写了能读回来（带批号），且**重开还在** —— 这就是"不要清空"的判据：
    /// 换一个"进程"（这里是换一个 root 读同一份盘）读出来还是同一本
    #[test]
    fn what_is_written_survives_a_fresh_read() {
        let d = tempfile::tempdir().unwrap();
        let key = key_of("local", "presets-local/A1 涂胶.toml");
        let mut file = HistoryFile::default();
        file.batches = 2;
        file.entries.push(HistoryEntry {
            id: 3,
            no: 3,
            action: "改 Y 轴偏移".to_owned(),
            ts_unix: 1_700_000_000,
            state: "saved".to_owned(),
            batch: Some(2),
            items: vec![item()],
        });
        write(d.path(), &key, &file).unwrap();

        let again = read(d.path(), &key).unwrap();
        assert_eq!(again.batches, 2);
        assert_eq!(again.entries.len(), 1);
        assert_eq!(again.entries[0].items[0].to, "1.23");
        assert_eq!(again.entries[0].batch, Some(2));
    }

    /// 清空 = 盘上真的没了（界面上"没有改动记录"与"没有这一本"是同一件事）
    #[test]
    fn clearing_removes_the_file_itself() {
        let d = tempfile::tempdir().unwrap();
        let key = key_of("preset", "A1-standard.toml");
        let mut file = HistoryFile::default();
        file.entries.push(HistoryEntry {
            id: 1,
            no: 1,
            action: "改 X".to_owned(),
            ts_unix: 1,
            state: "draft".to_owned(),
            batch: None,
            items: vec![item()],
        });
        write(d.path(), &key, &file).unwrap();
        assert!(file_of(d.path(), &key).is_file());

        clear(d.path(), &key).unwrap();
        assert!(
            !file_of(d.path(), &key).is_file(),
            "清了就不该留一个 0 条的文件"
        );
        assert_eq!(read(d.path(), &key).unwrap().entries.len(), 0);
    }

    /// 坏档照实报 `CORRUPTED`，不静默当空本（静默吞状态是编数据的近亲）
    #[test]
    fn a_broken_file_is_reported_not_silently_ignored() {
        let d = tempfile::tempdir().unwrap();
        let key = key_of("preset", "A1-standard.toml");
        let at = file_of(d.path(), &key);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, "{ 这不是 json".as_bytes()).unwrap();

        let err = read(d.path(), &key).expect_err("坏档得报错");
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
    }
}
