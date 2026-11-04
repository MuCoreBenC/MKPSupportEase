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
//! 那 9 份 `assets/presets/*.toml` 现由工作台 `wb_generate` 生成（2026-11-04 起
//! `gen-presets` 已退役），头部是 `# machine` / `# variant` / `# generated_at`。
//! 这里除了「表 == 目录」，还咬住**客户端读取面**能吃下这份新格式。

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

/// 客户端消费端闸门（2026-11-04）：旧生成体系退役后，正式产物仍是客户端读得动的输入。
///
/// 现役头 = `# machine` / `# variant` / `# generated_at`；`uuid`（2026-10-05 删）与
/// `release_time`（2026-11-01 换成写盘记录）都不许回来。`read_preset_from_bytes` 靠
/// `# machine` 判归属（缺了直接 `MissingMachine`），参数主体走 `TomlConfig` 自身 schema、
/// 与老配方体系无关 —— 解析成功就证明这条链没被破坏性重构一起删掉。
///
/// 两个时间各归各（2026-11-04 裁定）：新产物只有 `# generated_at` 写盘记录，
/// **没有** `# release_time` 配方版本标记 ⇒ `release_time` 必须是 `None`，
/// 界面照实写「未知」，不许拿写盘时刻顶（`AGENTS.md` §6）。
#[test]
fn builtin_presets_are_readable_by_the_client_without_legacy_header() {
    for (name, content) in preset::BUILTIN_PRESETS {
        let file = preset::read_preset_from_bytes((*content).to_owned())
            .unwrap_or_else(|e| panic!("{name} 客户端读取面解析失败：{e}"));
        assert!(!file.machine.is_empty(), "{name} 的 # machine 不能丢");
        assert!(
            file.release_time.is_none(),
            "{name} 没有 # release_time，读取面却编出了一个：{:?}",
            file.release_time
        );
        assert!(
            file.generated_at.is_some(),
            "{name} 的写盘记录 # generated_at 要单独读出来"
        );
        assert!(
            !content.contains("# uuid:"),
            "{name} 把已删的 uuid 加回来了"
        );
        assert!(
            !content.contains("# release_time:"),
            "{name} 把已退役的 release_time 加回来了"
        );
        assert!(
            content.contains("# generated_at:"),
            "{name} 缺写盘记录 generated_at"
        );
    }
}
