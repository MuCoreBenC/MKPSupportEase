//! **下载事务**：一次下载 = 一份身份快照 + 一条提交路径 + 一份回执。
//!
//! # 它解决的是什么
//!
//! 从前「取字节」「决定这份属于哪个源」「写来源记录」散在三处，各自去读一次状态：
//! 取字节走 `source::resolve_source`，写记录却在用户根上读 `run/app-state.json`
//! （那个文件只在内部根）—— 于是记录里永远缺分支。现在身份在**事务开始时固定一次**，
//! 之后不再读盘；写入侧只接收这一份快照。
//!
//! # 三段身份分开，互不推导
//!
//! ```text
//! 数据源身份  SourceIdentity    platform / repo / ref        ← source::current_identity
//! 发布身份    ReleaseIdentity   catalogId / publishSerial    ← 目录自报（没有就是没有）
//! 文件身份    path + sha256                                   ← 本次提交的字节
//! ```
//!
//! 数据源是 Gitee 不代表发布线是「support-ease」—— 后者只能由目录自己说。
//!
//! # 不假装原子
//!
//! 文件系统与记录没法一起原子提交。这里的顺序是：取字节（`delivery::deliver` 内部
//! 先校验再原子落盘）→ 写文件尾来源记录（失败整笔失败）→ 记运行索引（失败只在回执里
//! `indexed = false`，不静默吞）。回执只在全部步骤走完之后产生。

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use postprocess::diag::messages;

use crate::error::AppError;

use super::catalog::{hex, Catalog, CatalogFile};
use super::delivery;
use super::local::CatalogLine;
use super::source_identity::SourceIdentity;

/// 下载开始时固定的身份快照。**构造一次，之后不再读盘**。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadContext {
    /// 数据源身份（`platform#repo@ref` 的原来那份）。认不出 ⇒ `None`，不编
    pub source: Option<SourceIdentity>,
    /// 发布身份。随包目录不带 `catalogId` ⇒ `None`
    pub release: Option<ReleaseIdentity>,
    /// 这一代目录的内容指纹 —— 给回执对照用，**不是**身份
    pub catalog_revision: String,
}

/// 目录自报的发布身份（`catalogId` + 序号 + 发布时刻）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseIdentity {
    pub catalog_id: String,
    pub serial: Option<u64>,
    pub published_at: Option<String>,
}

/// 这一次字节到底有没有被校验过。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Integrity {
    /// 目录登记了期望 SHA，且落盘前比对通过（今天只有 `mkp_preset` 有）
    Verified,
    /// 目录没登记期望值（`bbs_config` / `model` / `icon`）—— **没有校验**，不冒充
    NotRecorded,
}

/// 一次下载完成之后的回执。它只说已经发生的事实。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadReceipt {
    pub file_name: String,
    /// 落点（相对内部根；预设的真实字节在内容仓库里，这里是索引登记的那条路径）
    pub path: String,
    /// 本次提交的字节摘要
    pub sha256: String,
    pub integrity: Integrity,
    pub context: DownloadContext,
    /// 运行索引（`run/provenance.json`）写成了没有。
    /// **没写不等于下载失败** —— 字节已经落了，回执如实说这一格没记上。
    pub indexed: bool,
    /// 这次下载之后有没有顺带放入切片器（**组合层**做的另一件事）。
    /// `None` = 没做（MKP 预设与「下载不放入」）；有值时它自己是一份独立回执。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<super::install::InstallReceipt>,
}

/// 事务入口：把「这一刻生效的源 + 这一代目录的发布身份」定格成一份快照。
///
/// 数据源只走 [`super::source::current_identity`]（它内部走 `current_entry`，
/// 没在设置页选过源时用出厂默认）；发布身份只认目录自报的两格。
pub fn context_of(root: &Path, catalog: &Catalog) -> Result<DownloadContext, AppError> {
    Ok(DownloadContext {
        source: super::source::current_identity(root)?,
        release: CatalogLine::of(catalog).map(|line| ReleaseIdentity {
            catalog_id: line.catalog_id,
            serial: line.serial,
            published_at: line.published_at,
        }),
        catalog_revision: catalog.revision.clone(),
    })
}

/// 下完一份之后组装回执（字节已经落盘，这里只如实读回来）。
///
/// `indexed` 由调用方给：运行索引那一步是可选的附加信息，写不上不影响这次下载本身。
pub fn receipt_of(
    root: &Path,
    file: &CatalogFile,
    context: &DownloadContext,
    indexed: bool,
) -> Result<DownloadReceipt, AppError> {
    let bytes = delivery::stored_bytes(root, file).ok_or_else(|| {
        AppError::not_found(messages::text(
            "ui.runtime.delivery.382",
            &[("v1", &file.file_name.to_string())],
        ))
    })?;
    Ok(DownloadReceipt {
        file_name: file.file_name.clone(),
        path: file.path.clone(),
        sha256: hex(&Sha256::digest(&bytes)),
        integrity: integrity_of(file),
        context: context.clone(),
        indexed,
        placed: None,
    })
}

/// 目录给这一份登记了期望 SHA 吗。登记了 = 管道已经比对过；没登记 = 没校验。
pub fn integrity_of(file: &CatalogFile) -> Integrity {
    if file.expected_sha().is_some() {
        Integrity::Verified
    } else {
        Integrity::NotRecorded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: &str, sha: Option<&str>) -> CatalogFile {
        CatalogFile {
            kind: kind.to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "standard".to_owned(),
            sha256: sha.map(str::to_owned),
            size: None,
            preset_schema_signature: (kind == "mkp_preset").then(preset::preset_schema_signature),
            preset_contract_epoch: (kind == "mkp_preset").then(preset::preset_contract_epoch),
        }
    }

    /// 只有登记了期望值的那一类才算「已校验」—— 别把「没登记」说成「校验通过」。
    #[test]
    fn integrity_is_recorded_only_when_the_catalog_gave_a_digest() {
        assert_eq!(
            integrity_of(&entry("A1-standard.toml", "mkp_preset", Some("abc"))),
            Integrity::Verified
        );
        assert_eq!(
            integrity_of(&entry("MKPProcess.json", "bbs_config", None)),
            Integrity::NotRecorded
        );
        assert_eq!(
            integrity_of(&entry("A1.webp", "icon", None)),
            Integrity::NotRecorded
        );
    }

    /// 随包目录没有发布身份 ⇒ 快照里那格是 `None`，不从数据源身份推导。
    #[test]
    fn a_catalog_without_identity_yields_no_release() {
        let catalog = Catalog::parse(
            br#"{"catalogSchema":1,"revision":"r1","machines":[],"files":[],"assets":[]}"#,
        )
        .expect("最小目录");
        assert_eq!(CatalogLine::of(&catalog), None);
    }

    /// 有身份就原样定格（三格都在，不改写）。
    #[test]
    fn a_catalog_with_identity_is_frozen_as_is() {
        let catalog = Catalog::parse(
            br#"{"catalogSchema":1,"revision":"r1","publishedAt":"2026-11-19T00:00:00Z","catalogId":"support-ease","publishSerial":22,"machines":[],"files":[],"assets":[]}"#,
        )
        .expect("最小目录");
        let line = CatalogLine::of(&catalog).expect("有身份");
        assert_eq!(line.catalog_id, "support-ease");
        assert_eq!(line.serial, Some(22));
        assert_eq!(line.published_at.as_deref(), Some("2026-11-19T00:00:00Z"));
    }
}
