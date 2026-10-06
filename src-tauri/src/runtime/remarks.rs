//! **备注覆盖账**：用户在客户端给某一份预设改过的副标题（备注）。
//!
//! # 为什么要有这本账（与出处账同一族问题）
//!
//! 客户端预设列表的副标题 = 工作台写的版本**备注**（`MachineVersion.remark`，随
//! catalog 发布链走）。作者定（2026-10-07）：**默认下载的用工作台的那句；用户改了
//! 之后，以后更新不覆盖；除非他把文件删了重新下载** —— 覆盖与否这件事盘上不存在
//! （文件字节、目录登记都说不出"用户改过备注"），所以记在用户根的这本账里。
//!
//! - 键 = 文件的稳定身份：官方交付行是 **`catalog.path`**（换版本时 `fileName`
//!   可能同名、但目录里那份落点不变；删除后再下载 = 账上的键被清掉，回到工作台那句）；
//! - 用户线（`presets-mine/`）是**相对用户根的路径**（与出处账同一把钥匙）。
//!   两个键空间不重叠（`delivery/…` vs `presets-mine/…`），合在一本账里没问题。
//!
//! 账上有的 = 用户改过 ⇒ 界面用它；没有 ⇒ 回落 catalog 里那一版的 `remark`；
//! 连版本都没有 ⇒ 回落路径文本。**账本丢了什么都不坏**（退回工作台的备注）。

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ledger {
    #[serde(default)]
    remarks: BTreeMap<String, String>,
}

/// 账本落点：用户根下的 `preset-remarks.json`（与 `provenance.json` 同层同族）
fn ledger_path(user_root: &Path) -> std::path::PathBuf {
    user_root.join("preset-remarks.json")
}

/// 读整本。读不到（还没改过）/ 坏档 ⇒ 空表 —— 备注覆盖是附加信息，
/// 缺了不许让任何读失败（界面回落工作台那句）。
pub fn load(user_root: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = std::fs::read(ledger_path(user_root)) else {
        return BTreeMap::new();
    };
    serde_json::from_slice::<Ledger>(&bytes)
        .map(|l| l.remarks)
        .unwrap_or_default()
}

/// 查一条（没有 = `None`，界面回落）。
pub fn get(user_root: &Path, key: &str) -> Option<String> {
    load(user_root).get(key).cloned()
}

/// 写一条覆盖。**用户写什么就是什么 —— 包括空串**（2026-10-07 作者改口：
/// 「可以空着，不要回退」—— 空也是用户的选择，副标题就空着）。
/// 只有 [`remove`]（恢复默认 / 删除文件）才把覆盖拿掉、退回回落链。
pub fn set(user_root: &Path, key: &str, remark: Option<&str>) -> Result<(), AppError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::invalid_argument("备注要说是哪一份（缺文件身份）"));
    }
    let mut ledger = Ledger {
        remarks: load(user_root),
    };
    match remark {
        Some(text) => {
            ledger
                .remarks
                .insert(key.to_owned(), text.trim().to_owned());
        }
        None => {
            ledger.remarks.remove(key);
        }
    }
    save(user_root, &ledger)
}

/// 删一条覆盖（= 详情面板的「恢复默认」；删除文件时也跟着删 ——
/// 「删了重新下载就回到工作台的备注」靠这一句兑现）。没有这条就是无事发生。
pub fn remove(user_root: &Path, key: &str) -> Result<(), AppError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::invalid_argument("备注要说是哪一份（缺文件身份）"));
    }
    let mut ledger = Ledger {
        remarks: load(user_root),
    };
    ledger.remarks.remove(key);
    save(user_root, &ledger)
}

fn save(user_root: &Path, ledger: &Ledger) -> Result<(), AppError> {
    let body = serde_json::to_vec_pretty(ledger)
        .map_err(|e| AppError::internal("备注账本序列化不了").with_detail(e.to_string()))?;
    atomic_write(&ledger_path(user_root), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(get(d.path(), "delivery/mkp/presets/A1-standard.toml"), None);
        set(
            d.path(),
            "delivery/mkp/presets/A1-standard.toml",
            Some("我的常用配置"),
        )
        .unwrap();
        assert_eq!(
            get(d.path(), "delivery/mkp/presets/A1-standard.toml").as_deref(),
            Some("我的常用配置"),
        );
    }

    /// **空串也是覆盖**（2026-10-07 作者改口：「可以空着，不要回退」）——
    /// 副标题就空着；只有 `remove`（恢复默认）才退回回落链
    #[test]
    fn an_empty_remark_is_a_valid_override_not_a_removal() {
        let d = tempfile::tempdir().unwrap();
        set(d.path(), "presets-mine/A.toml", Some("x")).unwrap();
        set(d.path(), "presets-mine/A.toml", Some("  ")).unwrap();
        assert_eq!(get(d.path(), "presets-mine/A.toml").as_deref(), Some(""));
        set(d.path(), "presets-mine/A.toml", None).unwrap();
        assert_eq!(get(d.path(), "presets-mine/A.toml"), None);
    }

    /// 删除文件时清键：「删了重新下载就恢复工作台的备注」的判据落在这里；
    /// 没写过就删 = 无事发生；坏档 ⇒ 空表
    #[test]
    fn removing_the_key_brings_back_the_workbench_remark() {
        let d = tempfile::tempdir().unwrap();
        set(d.path(), "delivery/mkp/presets/A1-fast.toml", Some("改过")).unwrap();
        remove(d.path(), "delivery/mkp/presets/A1-fast.toml").unwrap();
        assert_eq!(get(d.path(), "delivery/mkp/presets/A1-fast.toml"), None);
        remove(d.path(), "presets-mine/没有.toml").unwrap();
        atomic_write(&d.path().join("preset-remarks.json"), b"{not json").unwrap();
        assert!(load(d.path()).is_empty());
    }

    /// 缺文件身份（空键）拒绝 —— 这种调用是前端 bug，宁可响
    #[test]
    fn an_empty_key_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let e = set(d.path(), "  ", Some("x")).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }
}
