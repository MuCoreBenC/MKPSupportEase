//! **`BUILTIN_PRESETS` 与入库目录一份不差**（b04 Task 15 / M4d）。
//!
//! `lib.rs` 那张表是手写的 —— `include_str!` 的路径必须是字面量，没法用目录遍历生成。
//! 于是漏一份、多一份、或者名字与内容对不上，**编译期都不会响**：
//! 少一条只是"那个预设没被铺到数据根"，而那种错在界面上表现为"预设列表少一个"，
//! 没人会想到是一张表写漏了。
//!
//! 这条判据把三件事都咬住：
//!
//! 1. 文件名集合与 `assets/presets/*.toml` 完全相同（多一份、少一份都红）
//! 2. 每条的**内容**与同名文件逐字节相等（防"名字对、`include_str!` 指错文件"）
//! 3. 表里没有重名条目（重名会让后写的那份静默覆盖前一份）
//!
//! 那 9 份 `assets/presets/*.toml` 本身是 `gen-presets` 从配方生成的产物，
//! 这一轮**暂留当对照基线**（tasks.md 15.4），Task 18 才删。

use std::collections::BTreeSet;

// 入库产物目录只认 `generate::assets_dir()` 那一处（理由见 `tests/recipe.rs`）
use preset::generate::assets_dir;

fn names_on_disk() -> BTreeSet<String> {
    let dir = assets_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读不到内置预设目录 {}：{e}", dir.display()));
    entries
        .map(|e| e.expect("目录项"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".toml"))
        .collect()
}

#[test]
fn the_handwritten_table_matches_the_directory() {
    let on_disk = names_on_disk();
    let in_table: BTreeSet<String> = preset::BUILTIN_PRESETS
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();

    let missing: Vec<&String> = on_disk.difference(&in_table).collect();
    let extra: Vec<&String> = in_table.difference(&on_disk).collect();

    assert!(
        missing.is_empty(),
        "这些预设在 assets/presets/ 里有、但 BUILTIN_PRESETS 表里漏了：{missing:?}\n\
         漏一份不会报错，只会表现为「数据根里少一个预设」—— 所以要在这里响"
    );
    assert!(
        extra.is_empty(),
        "BUILTIN_PRESETS 表里有、但 assets/presets/ 里没有：{extra:?}\n\
         （这种情况其实编译就过不去，除非文件刚被删而没重新编译）"
    );
    assert_eq!(
        in_table.len(),
        9,
        "内置预设应是 9 份（实测基线）；条数变了就该在提交里说清为什么"
    );
}

#[test]
fn every_entry_carries_the_bytes_of_its_own_file() {
    for (name, content) in preset::BUILTIN_PRESETS {
        let path = assets_dir().join(name);
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
        assert!(
            *content == on_disk,
            "BUILTIN_PRESETS 里 {name} 那一条的内容与 {} 不同 —— \
             多半是 include_str! 的路径指错了文件（名字对、内容不对）",
            path.display()
        );
    }
}

#[test]
fn the_table_has_no_duplicate_names() {
    let mut seen = BTreeSet::new();
    for (name, _) in preset::BUILTIN_PRESETS {
        assert!(
            seen.insert(*name),
            "BUILTIN_PRESETS 里 {name} 出现了两次 —— 铺到数据根时后一份会静默盖掉前一份"
        );
    }
}
