//! **放入事务**：把一份已经存在的资源写进切片器的账号目录。
//!
//! # 目标环境由调用方给
//!
//! 「切片器目录长在哪」是平台相关的枚举（`ipc::presets::process_dirs_of`），住 IPC 层。
//! 这里只收一份 `(账号名, 目录)` 列表，逐目录原子写 —— 事务本体因此能被单测钉住，
//! 不依赖这台机器上装没装切片器。
//!
//! # 成功判据
//!
//! **至少写进去一份，且没有失败**。部分失败时回执把失败的那几个账号与原因原样列出来：
//! 「八份成了、两份因为什么没成」必须能被界面说清，不再只打一行日志了事。

use std::path::PathBuf;

use serde::Serialize;

use crate::fsx::atomic::atomic_write;

/// 某一个账号目录没写进去的原因。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountFailure {
    /// 账号目录名（`default` / `BBL` / 数字 uid）
    pub account: String,
    /// 来自写盘那一层的原话，不由这里造句
    pub message: String,
}

/// 一次放入的结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReceipt {
    pub file_name: String,
    /// 这次打算写几个目录
    pub targets: usize,
    /// 真写进去几个
    pub written: usize,
    /// 目标之前就有同名文件（换掉了，不是新建）
    pub replaced: bool,
    /// 没写进去的那几个账号
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<AccountFailure>,
}

impl InstallReceipt {
    /// 事务意义上的成功：写进去至少一份，且一个失败都没有。
    pub fn ok(&self) -> bool {
        self.written > 0 && self.failed.is_empty()
    }

    /// 一份都没写进去（目标目录不可用 / 全被拒）。
    pub fn all_failed(&self) -> bool {
        self.written == 0
    }

    /// 失败账号拼成一句给错误详情用（`default：权限不足；BBL：…`）。
    pub fn failure_detail(&self) -> String {
        self.failed
            .iter()
            .map(|f| format!("{}：{}", f.account, f.message))
            .collect::<Vec<_>>()
            .join("；")
    }
}

/// 事务本体：逐目录写，**一份失败不拦其余**。
///
/// `targets` 是 `(账号名, 目录)`；文件落成 `<目录>/<file_name>`。
/// 写盘只走 [`atomic_write`]（与全仓同一条：同目录临时文件 → rename，
/// 切片器正开着也读不到半个文件）。
pub fn install_into(
    targets: &[(String, PathBuf)],
    file_name: &str,
    bytes: &[u8],
) -> InstallReceipt {
    let mut replaced = false;
    let mut written = 0usize;
    let mut failed = Vec::new();
    for (account, dir) in targets {
        let target = dir.join(file_name);
        if target.is_file() {
            replaced = true;
        }
        match atomic_write(&target, bytes) {
            Ok(()) => written += 1,
            Err(e) => failed.push(AccountFailure {
                account: account.clone(),
                message: e.message,
            }),
        }
    }
    InstallReceipt {
        file_name: file_name.to_owned(),
        targets: targets.len(),
        written,
        replaced,
        failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// 两个账号目录都写进去：回执说 2/2，`ok()` 为真。
    #[test]
    fn every_account_directory_gets_the_same_bytes() {
        let a = dir();
        let b = dir();
        let targets = vec![
            ("default".to_owned(), a.path().to_path_buf()),
            ("BBL".to_owned(), b.path().to_path_buf()),
        ];
        let r = install_into(&targets, "MKPProcess.json", b"{}");
        assert!(r.ok());
        assert_eq!((r.targets, r.written), (2, 2));
        assert!(!r.replaced, "目标本来是空的");
        assert_eq!(std::fs::read(a.path().join("MKPProcess.json")).unwrap(), b"{}");
        assert_eq!(std::fs::read(b.path().join("MKPProcess.json")).unwrap(), b"{}");
    }

    /// 目标已有同名 ⇒ `replaced` 为真（界面据此说「切片器里那份换了」）。
    #[test]
    fn an_existing_copy_is_marked_as_replaced() {
        let a = dir();
        crate::fsx::atomic::atomic_write(&a.path().join("MKPProcess.json"), b"old").unwrap();
        let targets = vec![("default".to_owned(), a.path().to_path_buf())];
        let r = install_into(&targets, "MKPProcess.json", b"new");
        assert!(r.ok());
        assert!(r.replaced);
        assert_eq!(
            std::fs::read(a.path().join("MKPProcess.json")).unwrap(),
            b"new"
        );
    }

    /// 一份失败不拖累别人：回执里能看见是哪个账号、为什么。
    #[test]
    fn one_bad_target_does_not_stop_the_others() {
        let good = dir();
        let bad = dir();
        /* 把目标文件名先占成一个目录 ⇒ 写那个名字必失败 */
        std::fs::create_dir(bad.path().join("MKPProcess.json")).unwrap();
        let targets = vec![
            ("default".to_owned(), good.path().to_path_buf()),
            ("BBL".to_owned(), bad.path().to_path_buf()),
        ];
        let r = install_into(&targets, "MKPProcess.json", b"{}");
        assert_eq!(r.written, 1);
        assert_eq!(r.failed.len(), 1);
        assert_eq!(r.failed[0].account, "BBL");
        assert!(!r.ok(), "有失败就不算事务成功");
        assert!(r.failure_detail().contains("BBL"));
    }

    /// 目标目录列表为空 ⇒ 一份都没写，`all_failed()` 为真（调用方据此报错）。
    #[test]
    fn no_targets_is_not_success() {
        let r = install_into(&[], "x.json", b"{}");
        assert_eq!((r.targets, r.written), (0, 0));
        assert!(r.all_failed());
        assert!(!r.ok());
    }
}
