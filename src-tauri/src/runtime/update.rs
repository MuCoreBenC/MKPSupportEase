//! 更新检查：本地目录 vs 远端目录（两端共用契约的直接消费者）。
//!
//! # 契约是什么
//!
//! 工作台发布（`wb_publish`）把 `dist/catalog.json` 与交付文件一起产出——
//! 与客户端首屏读的那份 [`super::Catalog`] **同一个类型、同一个 schema、同一套指纹**。
//! 所以"检查更新"看两件事：`revision`（内容）与 `publishSerial`（发布身份）——
//! 任一前进都算有新目录。**只看 revision 会漏掉"只盖身份、不动字节"的那次发布**
//! （第 1 号就是这种，客户端因此一直拿着没有身份的旧目录）。
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

/// 比较本地与远端。指纹一致 **且发布身份没有前进** = 已是最新。
///
/// 两个来源都必须看（2026-10-29 修正）：
/// - **内容**：多出新文件、少了文件、换了字节 ⇒ `revision` 变化；
/// - **发布身份**：`catalogId` / `publishSerial` 前进 ⇒ 也是一代新目录，**哪怕内容一字未变**。
///
/// ★ 只看 `revision` 会漏掉"只盖身份、不动内容"的那次发布（第 1 号就是这种）：
///   `revision` 刻意不含身份（[`super::catalog`] 模块头），于是客户端永远拿不到
///   带 `catalogId` / `publishSerial` 的目录，每份文件都被判成"版本关系未知" ——
///   作者 2026-10-29 真机上撞到的死循环就是这个。
///
/// 顺带答一件**独立**的事：这一代远端目录本构建读不读得懂（`readable`）。
/// 判定只写在 [`structure::can_read`] 一处 —— 这里不重复实现"什么算读得懂"。
pub fn check(local: &Catalog, remote: &Catalog) -> RemoteUpdate {
    RemoteUpdate {
        up_to_date: !is_newer_generation(local, remote),
        local_revision: local.revision.clone(),
        remote_revision: remote.revision.clone(),
        readable: structure::can_read(
            &remote.structure_signature,
            remote.min_client_version.as_deref(),
        ),
    }
}

/// 远端这一代比本机手里那份"更靠后"吗。
///
/// 两种"更靠后"：内容变了（`revision` 不同），或发布身份前进了（同内容重发的第 N 号）。
/// 发布线的口径与 `runtime::local::based_on` 一致：
/// - 远端没有身份 ⇒ 没发布过，不构成"新目录"（不许因此反复惊动用户）；
/// - 两条**不同的线**不比序号（各自数各自的号）；
/// - 本机没有身份 ⇒ 老目录 / 随包 bootstrap，远端有身份就是换了一代。
fn is_newer_generation(local: &Catalog, remote: &Catalog) -> bool {
    if local.revision != remote.revision {
        return true;
    }
    let Some(remote_serial) = remote.publish_serial else {
        return false;
    };
    if let (Some(mine), Some(theirs)) = (&local.catalog_id, &remote.catalog_id) {
        if mine != theirs {
            return false;
        }
    }
    local
        .publish_serial
        .map_or(true, |mine| remote_serial > mine)
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

    /// ★ **同内容重发也是一代新目录**（2026-10-29）：第 1 号发布只盖身份、不动一个字节，
    /// 若只看 `revision`，客户端永远发现不了它 —— 那之后每份文件都判"版本关系未知"。
    #[test]
    fn a_new_publish_identity_is_an_update_even_when_the_bytes_are_the_same() {
        let local = catalog("aaa"); // 老目录：没有身份
        let remote = Catalog {
            catalog_id: Some("support-ease".to_owned()),
            publish_serial: Some(1),
            ..catalog("aaa")
        };
        assert!(
            !check(&local, &remote).up_to_date,
            "本机没有身份、远端有 = 换了一代"
        );
    }

    /// 同一代（同线、同号、同内容）⇒ 已是最新，不许每次启动都重拉一遍
    #[test]
    fn the_same_identity_is_up_to_date() {
        let same = Catalog {
            catalog_id: Some("support-ease".to_owned()),
            publish_serial: Some(1),
            ..catalog("aaa")
        };
        assert!(check(&same, &same).up_to_date);
    }

    /// 同一条线上序号前进（内容没变）⇒ 有新目录
    #[test]
    fn a_forward_serial_on_the_same_line_is_an_update() {
        let local = Catalog {
            catalog_id: Some("support-ease".to_owned()),
            publish_serial: Some(1),
            ..catalog("aaa")
        };
        let remote = Catalog {
            publish_serial: Some(2),
            ..local.clone()
        };
        assert!(!check(&local, &remote).up_to_date);
    }

    /// 远端没有身份（从没发布过）⇒ 不构成"新目录"，不许凭空惊动
    #[test]
    fn an_unpublished_remote_is_not_an_update() {
        let local = Catalog {
            catalog_id: Some("support-ease".to_owned()),
            publish_serial: Some(1),
            ..catalog("aaa")
        };
        assert!(check(&local, &catalog("aaa")).up_to_date);
    }

    /// 不是同一条发布线 ⇒ 序号没有可比性（各数各的号）
    #[test]
    fn another_lines_serial_does_not_count() {
        let local = Catalog {
            catalog_id: Some("support-ease".to_owned()),
            publish_serial: Some(1),
            ..catalog("aaa")
        };
        let remote = Catalog {
            catalog_id: Some("someone-else".to_owned()),
            publish_serial: Some(99),
            ..catalog("aaa")
        };
        assert!(check(&local, &remote).up_to_date);
    }
}
