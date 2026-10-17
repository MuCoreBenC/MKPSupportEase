//! **「哪份预设用得最多」——本机学习账**（2026-10-17）。
//!
//! 作者原话：「其实我觉得后面要做自动学习的，除非换了设备要重新学习，不然他可以学习
//! 用哪个的切片后处理次数最多，就优先展示哪个？**这个数据不跟随备份导出**」。
//!
//! ```text
//! <appDataDir>/run/usage.json
//!        ▲
//!        │ 整份读-改-写（atomic_write_json）
//!        │
//!    usage.rs（唯一读写入口）
//! ```
//!
//! # 记什么
//!
//! **被"使用"（应用）过几次** —— `run/app-state.json` 那一步成功之后 +1
//! （`ipc::catalog::apply_active_preset` 是唯一的应用入口，所以计数不需要任何新界面）。
//! 校准保存不计数：那是"改"，不是"选它"。
//!
//! # 钥匙
//!
//! `local:<用户根相对路径>` / `cloud:<内部根相对路径>` —— 与选择器行上的 `ref` 同一口径，
//! 前端拿同一个字符串就能对上。用户目录里同名文件很常见，所以**不按文件名认**。
//!
//! # 为什么它不进备份
//!
//! 备份只打 `runtime::backup` 里 `MANAGED_DIRS` / `MANAGED_FILES` 那几样（app-state /
//! catalog / presets-local / preset-history / baseline / gcode_history），
//! `run/usage.json` 不在其中 —— **"不跟随备份导出"是结构保证的，不需要额外开关**。
//! 换一台机器就是没有这份文件 ⇒ 从零开始学，正好是作者要的"换设备重新学习"。
//!
//! # 坏档：**这次可以重来**
//!
//! 全仓规矩是"坏档报 `CORRUPTED`、不清空、不猜"，但这条只管**用户资产**。学习账是
//! 排序提示：坏了最多是列表回到默认顺序，重新学一遍即可 —— 所以这里**坏档按空账处理**
//! （下次 bump 直接写成一份新的）。这与 catalog / history / app-state 那种"猜一下就
//! 毁数据"的处境不同，别照抄那条。
//!
//! # 规矩
//!
//! 1. 一本账一份文件，键是上面那把钥匙；
//! 2. 住在内部根 `run/` 下（与 AppState 同层，用户不需要翻它）；
//! 3. 写走 [`crate::fsx::atomic::atomic_write_json`]（全仓唯一写盘出口）；
//! 4. 只做加法：没有"清零"这条命令（要清就是删文件，属于用户自己的事）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;

/// 账本格式的代次。加字段不升号，改语义才升（与 catalog / app-state 同一条）
pub const USAGE_SCHEMA: u32 = 1;

/// 账本落点（相对内部根）
const USAGE_FILE: &str = "run/usage.json";

/// 一份预设的用量
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEntry {
    /// 被应用过几次
    pub count: u64,
    /// 最近一次是何时（UTC epoch 秒）—— 排序的第二把尺，也给以后"最近用过"用
    pub last_unix: i64,
}

/// 整本账
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageFile {
    pub schema: u32,
    /// 钥匙 → 用量。`BTreeMap` 是为了 JSON 落盘顺序稳定（人眼看 diff 不跳）
    #[serde(default)]
    pub items: BTreeMap<String, UsageEntry>,
}

impl Default for UsageFile {
    fn default() -> Self {
        Self {
            schema: USAGE_SCHEMA,
            items: BTreeMap::new(),
        }
    }
}

/// 一本账的落点
fn file_of(root: &Path) -> PathBuf {
    root.join(USAGE_FILE)
}

/// 读整本。**没有 = 空账；坏了也当空账**（理由见模块头"坏档：这次可以重来"）
pub fn load(root: &Path) -> UsageFile {
    let bytes = match std::fs::read(file_of(root)) {
        Ok(b) => b,
        Err(_) => return UsageFile::default(),
    };
    match serde_json::from_slice::<UsageFile>(&bytes) {
        Ok(mut f) => {
            /* 旧档没写 schema 时 serde 会报缺字段 —— 上面 `Default` 兜住不存在的；这里
            只把代次抬到当前，不拒绝读（学习账没有"代次不兼容"的语义负担） */
            if f.schema == 0 {
                f.schema = USAGE_SCHEMA;
            }
            f
        }
        Err(_) => UsageFile::default(),
    }
}

/// 记一次"用了它"。**尽力而为**：调用方（应用预设那一步）不该因为学习账写不进去而失败。
pub fn bump(root: &Path, key: &str) -> Result<UsageEntry, AppError> {
    let mut book = load(root);
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let entry = book.items.entry(key.to_string()).or_default();
    entry.count = entry.count.saturating_add(1);
    entry.last_unix = now;
    let picked = entry.clone();
    atomic_write_json(&file_of(root), &book)?;
    Ok(picked)
}

/// 整本读给界面（排序用）。前端只认 `key` 与 `count`，`last_unix` 留给以后。
pub fn list(root: &Path) -> Vec<(String, UsageEntry)> {
    load(root).items.into_iter().collect()
}

/// 把**绝对路径**翻成与界面同口径的钥匙。
///
/// 钩子那一趟拿到的是切片器传进来的绝对路径（`--Toml <abs>`），而界面上的钥匙是
/// `local:<用户根相对路径>` / `cloud:<内部根相对路径>`。两个前缀都能剥就剥；
/// 既不落在用户根也不落在内部根（用户把 TOML 放在桌面之类）⇒ `None`，不记。
pub fn key_of_path(internal_root: &Path, user_root: &Path, path: &Path) -> Option<String> {
    let norm = |rel: &Path| rel.to_string_lossy().replace('\\', "/");
    if let Ok(rel) = path.strip_prefix(user_root) {
        return Some(format!("local:{}", norm(rel)));
    }
    if let Ok(rel) = path.strip_prefix(internal_root) {
        return Some(format!("cloud:{}", norm(rel)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_accumulates_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("run")).unwrap();

        assert_eq!(bump(root, "local:presets-local/a.toml").unwrap().count, 1);
        assert_eq!(bump(root, "local:presets-local/a.toml").unwrap().count, 2);
        assert_eq!(bump(root, "cloud:presets/b.toml").unwrap().count, 1);

        let book = load(root);
        assert_eq!(book.items["local:presets-local/a.toml"].count, 2);
        assert_eq!(book.items["cloud:presets/b.toml"].count, 1);
    }

    #[test]
    fn key_of_path_splits_the_two_roots() {
        let internal = Path::new("/app/data");
        let user = Path::new("/app/data/user");

        /* 用户根里的（`presets-local/…`）→ local: */
        assert_eq!(
            key_of_path(
                internal,
                user,
                Path::new("/app/data/user/presets-local/a.toml")
            )
            .as_deref(),
            Some("local:presets-local/a.toml")
        );
        /* 内部根里的（交付目录）→ cloud: */
        assert_eq!(
            key_of_path(
                internal,
                user,
                Path::new("/app/data/delivery/mkp/presets/b.toml")
            )
            .as_deref(),
            Some("cloud:delivery/mkp/presets/b.toml")
        );
        /* 两个根之外（用户把 TOML 放桌面）→ 不记 */
        assert_eq!(
            key_of_path(internal, user, Path::new("/Users/me/Desktop/c.toml")),
            None
        );
    }

    #[test]
    fn corrupt_book_starts_over() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("run")).unwrap();
        /* 测试造坏档，不走 atomic（不是产品数据落点） */
        #[allow(clippy::disallowed_methods)]
        std::fs::write(file_of(root), b"{not json").unwrap();

        assert!(load(root).items.is_empty());
        /* 坏档不挡写：下一次 bump 就把它整份换成新的 */
        assert_eq!(bump(root, "local:presets-local/a.toml").unwrap().count, 1);
        assert!(
            serde_json::from_slice::<UsageFile>(&std::fs::read(file_of(root)).unwrap()).is_ok()
        );
    }
}
