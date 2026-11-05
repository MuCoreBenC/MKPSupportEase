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
        /*
         * 云端产物**不许**带来源记录：`[mkp_source]` 由客户端在下载 / 接受版本那一刻
         * 写进用户自己那份（那是"我从哪来"），不该出现在分发出去的模板里。
         */
        assert!(
            file.provenance.is_none(),
            "{name} 是云端产物，不该带 [mkp_source] 来源记录"
        );
        assert!(
            !content.contains("mkp_source"),
            "{name} 的正文里出现了来源记录"
        );
    }
}

/// **三格时间各说一件事，谁都不许顶替谁**（2026-11-04 作者定的维护规则）：
///
/// | 这一格 | 唯一来源 |
/// |---|---|
/// | 生成时间 | 文件头 `# generated_at`（写盘那一刻） |
/// | 发布时间 | 目录 `catalog.publishedAt`（只说明目录什么时候发布） |
/// | 来源版本时间 | 文件尾部记录里履历最后一条的 `at` |
///
/// 判据做法：造一份「有 generated_at、没有 release_time、记录里 at 是另一个值」的预设，
/// 分别读三格，断言三个值互不相同、且各自来自自己的那一处。
#[test]
fn the_three_times_never_stand_in_for_each_other() {
    let body = "\
# machine: A1
# variant: standard
# generated_at: 2026-11-01T12:30:00+08:00

[toolhead]
speed_limit = 70
";
    /* 只有写盘记录时：配方版本时间是**未知**，不许拿 generated_at 顶 */
    let plain = preset::read_preset_from_bytes(body.to_owned()).expect("读得回来");
    assert_eq!(
        plain.generated_at.as_deref(),
        Some("2026-11-01T12:30:00+08:00"),
        "生成时间读得出来"
    );
    assert_eq!(
        plain.release_time, None,
        "没有 # release_time ⇒ 配方版本时间未知"
    );
    assert_eq!(plain.provenance, None, "云端模板没有来源记录");

    /* 客户端落地后：来源记录里记着**接受那一版**的发布时刻，与生成时间不同 */
    let landed = preset::provenance::copy_with_provenance(
        body,
        preset::model::SourceRef {
            kind: Some("download".to_owned()),
            file: Some("mkp/presets/A1-standard.toml".to_owned()),
            platform: Some("github".to_owned()),
            repo: Some("owner/repo".to_owned()),
            branch: Some("main".to_owned()),
            at: Some("2026-10-31T00:00:00Z".to_owned()),
            ..preset::model::SourceRef::default()
        },
        preset::model::SourceRef {
            at: Some("2026-10-31T00:00:00Z".to_owned()),
            file: Some("mkp/presets/A1-standard.toml".to_owned()),
            ..preset::model::SourceRef::default()
        },
    );
    let file = preset::read_preset_from_bytes(landed).expect("带记录也要读得回来");
    assert_eq!(
        file.generated_at.as_deref(),
        Some("2026-11-01T12:30:00+08:00"),
        "生成时间仍是文件头那一格"
    );
    assert_eq!(
        file.release_time, None,
        "落地不会凭空造出一个 # release_time"
    );
    assert_eq!(
        file.provenance
            .as_ref()
            .and_then(|p| p.latest_version())
            .and_then(|v| v.at.as_deref()),
        Some("2026-10-31T00:00:00Z"),
        "来源版本时间 = 记录里那一刻"
    );
    assert_ne!(
        file.generated_at.as_deref(),
        file.provenance
            .as_ref()
            .and_then(|p| p.latest_version())
            .and_then(|v| v.at.as_deref()),
        "生成时间与来源版本时间是两件事 —— 相等就说明有人在顶替"
    );
}
