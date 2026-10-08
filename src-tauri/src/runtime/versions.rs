//! **官方版本清单**：这台机器上"见过"的官方预设版本有哪些。
//!
//! # 见过的 = 历次目录里登记过的
//!
//! 客户端不记账本（盘与目录就是底账，与 [`super::delivery`] 同一条纪律）。它见过
//! 的每一个官方版本都在**目录**里登记着：
//!
//! ```text
//! 当前目录 <appDataDir>/catalog.json           ← 现在生效的这一代
//! 版本链   <appDataDir>/archive/catalogs/*.json ← 每一次换代被换下的旧目录（追加，不覆盖）
//! ```
//!
//! 于是"云端有哪几版这个预设"= 把这几代目录里同一个 `file_name` 的条目合起来、
//! 按内容摘要去重。**不需要新的落盘账本**：目录换代时那条链已经替我们记着了。
//!
//! # 「已下载」不在这里判
//!
//! 这一层只回答"官方登记过哪些版本"。**这一版我这台机器下过没有**由
//! [`super::baseline`] 回答（按摘要寻址，下过就有一份基准）。两件事分开，
//! 于是"云端有新版本"与"我下过这一版"各自有唯一、可验证的判据。
//!
//! # 版本日期
//!
//! 目录里**没有**每一个文件的发布时间（只有整代目录的 `publishedAt`）。所以这里
//! 带回来的 `published_at` 是**目录代时间**，只能当兜底；真正的"这个预设这一版
//! 是什么时候发的"在文件自己头上（`# release_time`），下载之后才读得到。

use std::path::Path;

use super::Catalog;

/// 目录里登记过的一个官方预设版本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialVersion {
    /// 归属的官方预设（`catalog` 里的 `fileName`，如 `A1-fast.toml`）
    pub file_name: String,
    /// 内容摘要 —— 版本的**唯一身份**（跨目录代不变，界面不显示）
    pub sha256: String,
    /// 这一版所属目录代的发布时间（`catalog.publishedAt`）。只有整代的时间，**不是**
    /// 这个预设自己的发布日；界面按"目录发布时间"如实标注，不冒充预设发布日期
    pub published_at: Option<String>,
    /// 这一版就是**当前目录登记的那一版**（= 现在点「下载」真能拿到的那一版）。
    ///
    /// 为什么必须有这一格：下载命令认的是**当前目录里的那一条**（`catalog.path` 的字节），
    /// 历史版本没法按摘要单独重下。所以界面只让"当前版"给「下载」—— 别的历史版本
    /// 若不在本机，就给不出一个真能成的动作（不给点了没反应的按钮）。
    pub current: bool,
}

/// 见过的官方版本（同一 `(file_name, sha)` 只出现一次；按目录代时间升序）。
///
/// 只认**登记了摘要**的条目：随包 bootstrap 目录故意不填 SHA（见 [`CatalogFile`]），
/// 没有摘要就没有"版本身份"，拿它当一版只会造出一个认不出来的行。
///
/// [`CatalogFile`]: super::catalog::CatalogFile
pub fn official_versions(root: &Path) -> Vec<OfficialVersion> {
    let mut out: Vec<OfficialVersion> = Vec::new();

    /* 版本链（历次被换下的旧目录）在前、当前目录在后 —— 同一版两处都有时，
    后面那一条（current = true）要赢 */
    if let Ok(entries) = std::fs::read_dir(super::release::catalog_chain_dir(root)) {
        let mut chain: Vec<std::path::PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        chain.sort();
        for path in chain {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(catalog) = Catalog::parse(&bytes) {
                    collect(&catalog, &mut out, false);
                }
            }
        }
    }
    if let Ok(catalog) = super::load_released_catalog(root) {
        collect(&catalog, &mut out, true);
    }

    /* 去重：先按身份聚到一起，`current` 的那条排前面（`dedup_by` 留第一条） */
    out.sort_by(|a, b| {
        a.file_name
            .cmp(&b.file_name)
            .then_with(|| a.sha256.cmp(&b.sha256))
            .then_with(|| b.current.cmp(&a.current))
    });
    out.dedup_by(|a, b| a.file_name == b.file_name && a.sha256 == b.sha256);

    /* 展示顺序：按目录代时间升序（没有时间的排最前，它们是更早的形态） */
    out.sort_by(|a, b| {
        a.published_at
            .cmp(&b.published_at)
            .then_with(|| a.file_name.cmp(&b.file_name))
            .then_with(|| a.sha256.cmp(&b.sha256))
    });
    out
}

fn collect(catalog: &Catalog, out: &mut Vec<OfficialVersion>, current: bool) {
    for f in &catalog.files {
        let Some(sha) = f.expected_sha() else {
            continue;
        };
        out.push(OfficialVersion {
            file_name: f.file_name.clone(),
            sha256: sha.to_ascii_lowercase(),
            published_at: catalog.published_at.clone(),
            current,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest as _;

    fn write_catalog(root: &Path, revision: &str, published_at: &str, name: &str, text: &str) {
        let sha = super::super::catalog::hex(&sha2::Sha256::digest(text.as_bytes()));
        let json = format!(
            r#"{{"catalogSchema":1,"revision":"{revision}","publishedAt":"{published_at}",
                "files":[{{"kind":"mkp_preset","fileName":"{name}",
                "path":"delivery/mkp/presets/{name}","machineId":"A1","versionId":"STANDARD",
                "sha256":"{sha}","size":{size}}}]}}"#,
            size = text.len()
        );
        crate::fsx::atomic::atomic_write(
            &super::super::paths::catalog_file(root),
            json.as_bytes(),
        )
        .unwrap();
    }

    /// 换了一代目录之后，旧的与新的**都在**清单里（版本链记着）—— 这是"多版本并存"的地基
    #[test]
    fn both_generations_show_up_after_a_catalog_change() {
        let d = tempfile::tempdir().unwrap();
        write_catalog(d.path(), "rev1", "2026-10-08T00:00:00Z", "A1-fast.toml", "v1");
        // 换代：旧目录进链，新目录生效
        let new_sha = super::super::catalog::hex(&sha2::Sha256::digest(b"v2"));
        let json = format!(
            r#"{{"catalogSchema":1,"revision":"rev2","publishedAt":"2026-10-15T00:00:00Z",
                "files":[{{"kind":"mkp_preset","fileName":"A1-fast.toml",
                "path":"delivery/mkp/presets/A1-fast.toml","machineId":"A1","versionId":"STANDARD",
                "sha256":"{new_sha}","size":2}}]}}"#
        );
        let old = std::fs::read(super::super::paths::catalog_file(d.path())).unwrap();
        std::fs::create_dir_all(super::super::release::catalog_chain_dir(d.path())).unwrap();
        crate::fsx::atomic::atomic_write(
            &super::super::release::catalog_chain_dir(d.path()).join("rev1"),
            &old,
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &super::super::paths::catalog_file(d.path()),
            json.as_bytes(),
        )
        .unwrap();

        let got = official_versions(d.path());
        assert_eq!(got.len(), 2, "两代各一版：{got:?}");
        assert_eq!(
            got[0].published_at.as_deref(),
            Some("2026-10-08T00:00:00Z"),
            "旧的在前（按代时间升序）"
        );
        assert_eq!(got[1].published_at.as_deref(), Some("2026-10-15T00:00:00Z"));
        assert_ne!(got[0].sha256, got[1].sha256);
        assert!(!got[0].current, "旧的那一版不是当前版");
        assert!(got[1].current, "当前目录登记的那一版才是「点下载真能拿到」的那一版");
    }

    /// 没换版：同一份摘要出现两代也只算一版（不虚报版本）
    #[test]
    fn an_unchanged_file_is_one_version_not_two() {
        let d = tempfile::tempdir().unwrap();
        write_catalog(d.path(), "rev1", "2026-10-08T00:00:00Z", "A1-fast.toml", "v1");
        let old = std::fs::read(super::super::paths::catalog_file(d.path())).unwrap();
        // 新一代目录里这一份没变，但 revision / publishedAt 变了
        std::fs::create_dir_all(super::super::release::catalog_chain_dir(d.path())).unwrap();
        crate::fsx::atomic::atomic_write(
            &super::super::release::catalog_chain_dir(d.path()).join("rev1"),
            &old,
        )
        .unwrap();
        let same_sha = super::super::catalog::hex(&sha2::Sha256::digest(b"v1"));
        let json = format!(
            r#"{{"catalogSchema":1,"revision":"rev2","publishedAt":"2026-10-15T00:00:00Z",
                "files":[{{"kind":"mkp_preset","fileName":"A1-fast.toml",
                "path":"delivery/mkp/presets/A1-fast.toml","machineId":"A1","versionId":"STANDARD",
                "sha256":"{same_sha}","size":2}}]}}"#
        );
        crate::fsx::atomic::atomic_write(
            &super::super::paths::catalog_file(d.path()),
            json.as_bytes(),
        )
        .unwrap();

        let got = official_versions(d.path());
        assert_eq!(got.len(), 1, "字节没变 = 还是同一版：{got:?}");
        assert!(got[0].current, "链与当前目录都有它时，这一版仍算当前版");
    }

    /// 没有摘要的条目不算一版（随包 bootstrap 目录故意不填 SHA）
    #[test]
    fn entries_without_a_digest_are_not_versions() {
        let d = tempfile::tempdir().unwrap();
        let json = r#"{"catalogSchema":1,"revision":"rev1",
            "files":[{"kind":"mkp_preset","fileName":"A1-fast.toml",
            "path":"delivery/mkp/presets/A1-fast.toml","machineId":"A1","versionId":"STANDARD"}]}"#;
        crate::fsx::atomic::atomic_write(
            &super::super::paths::catalog_file(d.path()),
            json.as_bytes(),
        )
        .unwrap();
        assert!(official_versions(d.path()).is_empty());
    }
}
