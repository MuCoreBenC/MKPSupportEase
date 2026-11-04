//! 两端一致性判据：**客户端读到的来源记录，与工作台读到的是同一份**。
//!
//! # 为什么要有这一条
//!
//! 2026-11-04 之前，来源记录是**同一个概念在两处实现**（工作台 `crates/preset/src/lineage.rs`
//! 与客户端 `src-tauri/src/runtime/lineage.rs` 各写一遍头部三行），靠这条判据逐字节钉住。
//!
//! 现在源头只有一份（`preset::provenance`，客户端转调它），这条判据的作用换了一个：
//! 它守的是「客户端这一层封装**没有偷偷加料**」——
//! 客户端 `runtime::provenance` 的建副本 / 剥除 / 摘要必须与库里那份给出同样的结果。
//! 谁在客户端那份里改了位置、键名、行尾处理，这里就红。
//!
//! 输入仍用 9 份入库产物的**真字节**：inline table、`"""` 多行 G-code、约 70 条注释、
//! 混大小写键 —— 造的样例盖不到这些。

use crate::runtime::provenance as app;
use preset::generate::fixtures_dir;
use preset::model::SourceRef;
use preset::provenance as lib;

/// 入库产物：`(文件名, 全文)`
fn fixtures() -> Vec<(String, String)> {
    let dir = fixtures_dir();
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("读入库产物目录")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&p).expect("读产物");
            (name, text)
        })
        .collect();
    out.sort();
    assert!(out.len() >= 9, "入库产物应有 9 份，实测 {}", out.len());
    out
}

fn label_of(name: &str) -> String {
    format!("mkp/presets/{name}")
}

fn origin(name: &str, sha: String) -> SourceRef {
    SourceRef {
        kind: Some("copy".to_owned()),
        file: Some(label_of(name)),
        sha256: Some(sha),
        ..SourceRef::default()
    }
}

/// **建副本**：客户端封装与库里实现必须给出逐字节相同的文本
#[test]
fn client_and_library_agree_on_the_copy_bytes() {
    for (name, src) in fixtures() {
        let src_ref = origin(&name, lib::sha256_hex(&src));
        let via_client = app::copy_from_cloud(&src, src_ref.clone(), src_ref.clone());
        let via_lib = lib::copy_with_provenance(&src, src_ref.clone(), src_ref);
        assert_eq!(
            via_client, via_lib,
            "客户端与库建出来的副本不一样（{name}）—— 来源记录的形状漂了"
        );
        /* 顺带钉住"副本 = 来源 + 尾部记录"这条共有的不变式 */
        assert_eq!(
            app::strip_for_compare(&via_client),
            src,
            "客户端剥掉记录之后与来源不同（{name}）"
        );
    }
}

/// **读记录**：同一份副本文本，两边读出来的必须一样（含"没有就是没有"）
#[test]
fn client_and_library_read_the_same_provenance() {
    for (name, src) in fixtures() {
        let src_ref = origin(&name, lib::sha256_hex(&src));
        let copy = lib::copy_with_provenance(&src, src_ref.clone(), src_ref);

        let via_client = app::parse(&copy).expect("记录都在，客户端该读得到");
        let via_lib = lib::parse(&copy).expect("记录都在，库里该读得到");
        assert_eq!(via_client, via_lib, "两边读出来的来源记录不一样（{name}）");

        /* 云端原件本身没有记录：两边都必须给 None（不是空壳） */
        assert_eq!(app::parse(&src), None, "云端原件不该有来源记录（{name}）");
        assert_eq!(lib::parse(&src), None, "云端原件不该有来源记录（{name}）");
    }
}

/// 摘要算法也得是同一个：记录里存的是来源**全文**的 sha256
#[test]
fn client_and_library_agree_on_the_digest() {
    for (name, src) in fixtures() {
        assert_eq!(
            app::sha256_hex(&src),
            lib::sha256_hex(&src),
            "全文摘要算法不一致（{name}）—— 「云端变了没有」就会两边各说一套"
        );
    }
}
