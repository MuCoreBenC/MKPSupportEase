//! 更新检查：本地目录 vs 远端目录（两端共用契约的直接消费者）。
//!
//! # 契约是什么
//!
//! 工作台发布（`wb_publish`）把 `dist/catalog.json` 与交付文件一起产出——
//! 与客户端首屏读的那份 [`super::Catalog`] **同一个类型、同一个 schema、同一套指纹**。
//! 所以"检查更新"就是两个字段的比较：`local_revision != remote_revision` = 有新目录。
//! 这是[`super::delivery::Source`]（拿文件）之外的第二半个远端口子：拿**清单**。
//! 真云端来了，manifest 字节从网络来，这条比较逻辑一行不改。
//!
//! # 更新的应用不在这里
//!
//! 应用 = [`super::release::release_bytes`]（旧目录归档、新目录生效）+
//! 对 Stale 文件重跑 [`super::delivery::deliver`]（旧文件归档、新文件落盘）。
//! 全是既有管道，没有第三条更新路径。

use super::structure;
use super::Catalog;

#[derive(Debug)]
pub struct RemoteUpdate {
    pub up_to_date: bool,
    pub local_revision: String,
    pub remote_revision: String,
    /// 本构建能不能读这一代远端目录（**能力优先、版本兜底**，见 [`structure::can_read`]）。
    ///
    /// ★ 它与 `up_to_date` 回答的是**两件不同的事**：
    /// - `up_to_date` = "远端有没有新目录"（比指纹）
    /// - `readable`   = "这一代我读不读得懂"（比结构能力）
    ///
    /// 混成一句话，用户就既不知道"是不是该等更新"，也不知道"要不要升级客户端"。
    pub readable: bool,
}

/// 比较本地与远端。指纹一致 = 已是最新；不一致 = 有新目录
/// （多出新文件、少了文件、换了字节，都表现为指纹变化——不需要逐项 diff 才知道"有没有"）。
///
/// 顺带答一件**独立**的事：这一代远端目录本构建读不读得懂（`readable`）。
/// 判定只写在 [`structure::can_read`] 一处 —— 这里不重复实现"什么算读得懂"。
pub fn check(local: &Catalog, remote: &Catalog) -> RemoteUpdate {
    RemoteUpdate {
        up_to_date: local.revision == remote.revision,
        local_revision: local.revision.clone(),
        remote_revision: remote.revision.clone(),
        readable: structure::can_read(
            &remote.structure_signature,
            remote.min_client_version.as_deref(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(revision: &str) -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: revision.to_owned(),
            ..Catalog::default()
        }
    }

    #[test]
    fn same_revision_is_up_to_date() {
        let r = check(&catalog("aaa"), &catalog("aaa"));
        assert!(r.up_to_date);
        assert_eq!(r.local_revision, "aaa");
    }

    #[test]
    fn different_revision_means_update_available() {
        let r = check(&catalog("aaa"), &catalog("bbb"));
        assert!(!r.up_to_date);
        assert_eq!(r.remote_revision, "bbb");
    }

    /// ★ **`readable` 与"有没有更新"是两件事**：指纹不同（有更新）但结构读不懂时，
    /// `up_to_date == false` 且 `readable == false` —— 两个字段各自答各的，不许互相覆盖。
    #[test]
    fn the_update_check_reports_readability() {
        let readable = Catalog {
            revision: "bbb".to_owned(),
            structure_signature: structure::SUPPORTED_SIGNATURES[0].to_owned(),
            ..catalog("aaa")
        };
        let r = check(&catalog("aaa"), &readable);
        assert!(!r.up_to_date, "指纹不同 = 有新目录");
        assert!(r.readable, "签名命中 = 读得懂");

        let unreadable = Catalog {
            revision: "ccc".to_owned(),
            structure_signature: "ffffffffffffffff".to_owned(),
            min_client_version: Some("99.0.0".to_owned()),
            ..catalog("aaa")
        };
        let r = check(&catalog("aaa"), &unreadable);
        assert!(!r.up_to_date);
        assert!(!r.readable, "签名不命中且版本不够 = 读不懂");
    }
}
