//! **客户端统一提交事务**（2026-10-31）。
//!
//! # 它解决什么
//!
//! 客户端原来有两类写盘：
//!
//! | 形态 | 例子 | 风险 |
//! |---|---|---|
//! | 单份文件 | 参数保存、备注、校准 | 已经是原子的，但**各自为政** |
//! | 多份联动 | 改名（文件 + 镜像 + 指针 + 来路账）、删除（文件 + 镜像 + 账本） | **中途失败留半套** |
//!
//! 第二类是这次要根治的：改名时文件已挪走、指针没更新，界面就会指着一份不存在的文件。
//! 修法不是给每个写盘函数加补偿代码（那是 N 份容易写漏的手工逻辑），而是**在操作外面
//! 套一层事务**：先记下这几份文件"改之前是什么"，闭包跑完只要有一条报错，就把它们
//! 全部按原样写回去。
//!
//! # 与工作台那套的区别（别混）
//!
//! 工作台的 `workbench::domain::txn` 是**编辑事务**：影子树 + 操作日志 + 撤销栈，
//! 因为它的编辑是"攒着、最后一次性发布"。客户端不一样 —— 作者 2026-10-31 定的是：
//!
//! - **参数编辑**才进草稿、才有撤销（那是"试错"）；
//! - 改名 / 删除 / 复制 / 归属 / 备注**立即执行**，不进撤销栈，删除靠回收站恢复；
//! - 但"立即执行"不等于"可以写一半" ⇒ 它们全部走这里，保证**要么全成、要么全回原样**。
//!
//! 所以这个模块**没有** `undo` / `redo` / `dirty` —— 别往这里加。撤销住在参数页
//! （`src/app/params` 的会话栈），跟文件操作是两件事。
//!
//! # 边界
//!
//! - **只回滚自己声明过的路径**：调用方必须把这次会动到的每一份都列进 `paths`。
//!   漏一份就回滚不到它 —— 那是调用方的责任，所以 [`atomically`] 的文档要求写清。
//! - **不碰云端内容仓库**：那套是发布产物，只读。这里给的路径只有用户目录 / 状态账。
//! - **不做跨进程锁**：切片器目录可能被外部程序并发改，那一路仍然走它自己的
//!   "读完校验摘要再写"（见 `ipc::slicer`），不塞进本事务。

use std::path::{Path, PathBuf};

use crate::error::AppError;
use postprocess::diag::messages;

/// 一份文件在事务开始前的样子（`None` = 当时还不存在）。
struct Before {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

/// 一次事务的记录。`tracing` 用，出问题时也能看出动了哪几份。
#[derive(Debug, Clone)]
pub struct Report {
    pub label: &'static str,
    /// 声明的路径份数（去重后）
    pub tracked: usize,
    /// 真正发生变化的份数（内容或存在性变了）
    pub changed: usize,
}

/// 把一组写盘动作包成"要么全成、要么全回原样"。
///
/// `paths` 必须**列全**这次操作会动的每一份文件（含状态账、镜像、来路账）。
/// 列漏的那一份不会被回滚 —— 这是本函数唯一无法自动发现的事。
///
/// 语义：
/// 1. 先给 `paths` 每份拍一张"改前"快照（不存在也记下来）；
/// 2. 跑 `f`；
/// 3. `f` 返回 `Err` ⇒ 把每一份按快照写回（原来不存在的就删掉），然后把这个错原样抛回；
///    回滚本身失败 ⇒ 在错误详情里点名"哪几份没退回去"（不能静默）。
///
/// `f` 返回 `Ok` 时不做任何验证 —— 它写了什么就是什么（本层不管业务语义）。
pub fn atomically<T>(
    label: &'static str,
    paths: &[PathBuf],
    f: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let mut tracked: Vec<Before> = Vec::with_capacity(paths.len());
    for path in paths {
        /* 同一个路径别拍两次：第二次拍到的会是"已经改过"的样子，回滚就退错了 */
        if tracked.iter().any(|b| b.path == *path) {
            continue;
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                return Err(AppError::io(messages::text("ui.txn.pre_read_failed", &[]))
                    .with_detail(format!("{}: {e}", path.display())))
            }
        };
        tracked.push(Before {
            path: path.clone(),
            bytes,
        });
    }

    match f() {
        Ok(out) => {
            let changed = tracked
                .iter()
                .filter(|b| match (&b.bytes, std::fs::read(&b.path)) {
                    (Some(before), Ok(now)) => *before != now,
                    (None, Err(_)) => false,
                    (None, Ok(_)) => true,
                    (Some(_), Err(_)) => true,
                })
                .count();
            tracing::info!(label, tracked = tracked.len(), changed, "客户端事务提交");
            Ok(out)
        }
        Err(e) => {
            let mut failed: Vec<String> = Vec::new();
            for b in tracked.iter().rev() {
                let done = match &b.bytes {
                    Some(bytes) => crate::fsx::atomic::atomic_write(&b.path, bytes),
                    None => match std::fs::remove_file(&b.path) {
                        Ok(()) => Ok(()),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(err) => Err(AppError::io(messages::text(
                            "ui.txn.rollback_remove_failed",
                            &[],
                        ))
                        .with_detail(err.to_string())),
                    },
                };
                if let Err(rollback) = done {
                    failed.push(format!("{}（{}）", b.path.display(), rollback));
                }
            }
            if failed.is_empty() {
                tracing::warn!(label, tracked = tracked.len(), "客户端事务失败，已整体回滚");
                Err(e)
            } else {
                tracing::error!(
                    label,
                    failed = failed.len(),
                    "客户端事务失败，且有几份没能退回去"
                );
                {
                    let joined = failed.join("；");
                    Err(e.with_detail(messages::text(
                        "ui.txn.rollback_incomplete",
                        &[("files", joined.as_str())],
                    )))
                }
            }
        }
    }
}

/// 用户目录里两份文件的改名 + 镜像搬家 + 指针/账本这类"确实存在的那几份"。
///
/// 只做**路径拼装**（不碰盘）：把事务要声明的路径一次拼好，免得调用方漏。
pub fn user_file_paths(user_root: &Path, rel_old: &str, rel_new: &str) -> Vec<PathBuf> {
    vec![user_root.join(rel_old), user_root.join(rel_new)]
}

#[cfg(test)]
// 测试要造文件：写盘纪律管的是生产代码
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use std::fs;

    /// 中途失败 ⇒ 已经改过的几份全部回到原样（这就是"不留半套"）。
    #[test]
    fn a_failure_restores_every_declared_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        let b = dir.path().join("b.toml");
        let c = dir.path().join("c.json");
        fs::write(&a, b"a-original").unwrap();
        fs::write(&b, b"b-original").unwrap();
        fs::write(&c, b"c-original").unwrap();

        let out: Result<(), AppError> = atomically(
            "测试 · 改名联动",
            &[a.clone(), b.clone(), c.clone()],
            || {
                fs::write(&a, b"a-changed").unwrap();
                fs::write(&b, b"b-changed").unwrap();
                // 第三份"失败"：模拟 app-state 写不进去
                Err(AppError::io("第三份故意失败"))
            },
        );

        assert!(out.is_err(), "失败必须传出去");
        assert_eq!(fs::read(&a).unwrap(), b"a-original", "第一份要退回去");
        assert_eq!(fs::read(&b).unwrap(), b"b-original", "第二份要退回去");
        assert_eq!(
            fs::read(&c).unwrap(),
            b"c-original",
            "没动过的第三份不能被改坏"
        );
    }

    /// 事务中途**新建**的文件，回滚时要删掉（否则改名会留一个空壳）。
    #[test]
    fn a_file_created_inside_a_failed_transaction_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let created = dir.path().join("new.toml");
        let existing = dir.path().join("old.toml");
        fs::write(&existing, b"old").unwrap();

        let out: Result<(), AppError> = atomically(
            "测试 · 新建后失败",
            &[created.clone(), existing.clone()],
            || {
                fs::write(&created, b"new").unwrap();
                Err(AppError::io("故意失败"))
            },
        );

        assert!(out.is_err());
        assert!(!created.exists(), "事务里新建的那份必须被删掉");
        assert_eq!(fs::read(&existing).unwrap(), b"old");
    }

    /// 成功路径：不回滚、原样保留结果，并报出"变了几份"。
    #[test]
    fn a_successful_transaction_keeps_its_writes() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        fs::write(&a, b"before").unwrap();

        let report = atomically("测试 · 成功", std::slice::from_ref(&a), || {
            fs::write(&a, b"after").unwrap();
            Ok(Report {
                label: "测试 · 成功",
                tracked: 1,
                changed: 1,
            })
        })
        .unwrap();

        assert_eq!(report.changed, 1);
        assert_eq!(fs::read(&a).unwrap(), b"after");
    }

    /// 同一个路径声明两次不会把"改后"当成"改前"——回滚要退到最初那一版。
    #[test]
    fn a_duplicated_path_is_snapshotted_once() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        fs::write(&a, b"v0").unwrap();

        let _: Result<(), AppError> =
            atomically("测试 · 重复路径", &[a.clone(), a.clone()], || {
                fs::write(&a, b"v1").unwrap();
                Err(AppError::io("故意失败"))
            });

        assert_eq!(fs::read(&a).unwrap(), b"v0", "必须退回最初那一版");
    }
}
