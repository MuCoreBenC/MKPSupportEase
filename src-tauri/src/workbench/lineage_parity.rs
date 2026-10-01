//! 两端一致性判据：**客户端那份「血统三行」与工作台那份必须逐字节一样**。
//!
//! # 为什么要有这一条
//!
//! 血统（`# based_on*` 三行）是**同一个概念在两处实现**：
//!
//! ```text
//! 工作台  crates/preset/src/lineage.rs   建副本（`preset_copy` 那一摊的底座）
//! 客户端  src-tauri/src/runtime/lineage.rs  另存成用户自己那份（第七层）
//! ```
//!
//! 客户端不许 `use preset`（隔离纪律：默认构建不编 `mkpse-preset` —— 那个 crate 带
//! 56 KB 参数注册表与 9 份内置预设），所以客户端那份是一份重写。两份之间**没有编译器**，
//! 谁偷偷改了形状（键名、插入位置、摘要算法、CRLF 处理）都不会有人报错 ——
//! 除非有这一条：它在 workbench feature 下同时看得到两边，拿同样的输入比同样的输出。
//!
//! # 输入用入库产物那 9 份真字节
//!
//! 真实预设里有 inline table（`offset = { x = … }`）、`"""` 多行 G-code、约 70 条注释、
//! 混大小写键 —— 造的样例盖不到这些，形状一变也露不出来。
//!
//! 将来若把这段文本逻辑下移到共享小 crate（两边转调、只剩一份实现），这个文件可以整个删掉。

use crate::runtime::lineage as app;
use preset::generate::fixtures_dir;
use preset::lineage as wb;

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

/// **建副本**：两边对同一份来源、同一个标签，必须给出逐字节相同的文本
#[test]
fn both_sides_make_the_byte_identical_copy() {
    for (name, src) in fixtures() {
        let label = label_of(&name);
        let mine = app::make_copy(&src, &label);
        let theirs = wb::make_copy(&src, &label);
        assert_eq!(
            mine, theirs,
            "客户端与工作台建出来的副本不一样（{name}）—— 血统三行的形状漂了"
        );
        /* 顺带钉住"副本 = 来源 + 三行"这条共有的不变式（两边各有一份自己的判据） */
        assert_eq!(
            app::strip_lineage_for_compare(&mine),
            src,
            "客户端剪掉三行之后与来源不同（{name}）"
        );
    }
}

/// **读血统**：同一份副本文本，两边读出来的三项必须一样（逐项比，连缺哪一项都要一样）
#[test]
fn both_sides_read_the_same_lineage() {
    for (name, src) in fixtures() {
        let copy = wb::make_copy(&src, &label_of(&name));

        let mine = app::parse_lineage_from_content(&copy).expect("三行都在，客户端该读得到");
        let theirs =
            preset::read::parse_lineage_from_content(&copy).expect("三行都在，工作台该读得到");
        let theirs = (
            theirs.based_on.clone(),
            theirs.based_on_release_time.clone(),
            theirs.based_on_sha256.clone(),
        );
        let mine = (
            mine.based_on.clone(),
            mine.based_on_release_time.clone(),
            mine.based_on_sha256.clone(),
        );
        assert_eq!(mine, theirs, "两边读出来的血统不一样（{name}）");

        /* 官方原件本身没有血统：两边都必须给 None（不是三项全空的空壳） */
        assert_eq!(app::parse_lineage_from_content(&src), None);
        assert_eq!(preset::read::parse_lineage_from_content(&src), None);
    }
}

/// 摘要算法也得是同一个：血统里记的是来源**全文**的 sha256，客户端与工作台算出来要一样
#[test]
fn both_sides_agree_on_the_digest() {
    for (name, src) in fixtures() {
        assert_eq!(
            app::sha256_hex(&src),
            wb::sha256_hex(&src),
            "全文摘要算法不一致（{name}）—— 「官方变了没有」就会两边各说一套"
        );
    }
}
