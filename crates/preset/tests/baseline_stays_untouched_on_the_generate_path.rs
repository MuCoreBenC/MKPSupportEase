//! **普通生成 / 检查路径不写对照基线**（b05 Task 6.5）。
//!
//! 基线是判据资产（`docs/ARCHITECTURE.md` §10.6）：唯一能改它的动作是
//! `gen-presets --sync-baseline`，而它要求人先看过差异。这条判据把
//! 「其余路径一个字节都不碰它」变成一句可执行的话 —— 真跑一遍生成与检查，
//! 再逐份比基线的内容哈希。
//!
//! # 口径
//!
//! 比的是**内容 sha256**，不是 mtime：`sync_baseline` 对内容相同的文件本来就不写
//! （注释里写着"内容相同就不写：mtime 变动会让别的判据重跑"），所以 mtime 差异不算违规。
//! 内容变了才是。
//!
//! # 与源码扫描那条判据的分工
//!
//! `write_discipline_scan.rs` 的 `the_baseline_has_exactly_one_write_path` 扫的是
//! **调用点**（证明没有别的入口）；这条跑的是**行为**（证明真跑起来确实没动）。
//! 「没有调用」本身没有运行时表现，所以缺任何一条都留口子。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::path::Path;

use preset::generate::{assets_dir, check_all, check_baseline, fixtures_dir, write_all};
use preset::lineage::sha256_hex;
use preset::recipe::Recipe;

const RECIPE: &str = include_str!("../assets/preset_recipes.toml");

/// `文件名 → 内容 sha256`。读不到就**响亮失败**，不返回一个空表装作没事。
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("读不到 {}：{e}", dir.display()));
    let mut out = BTreeMap::new();
    for entry in entries {
        let path = entry.expect("目录项").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        out.insert(name, sha256_hex(&text));
    }
    out
}

fn recipe() -> Recipe {
    Recipe::parse(RECIPE).expect("仓库里的配方")
}

/// **判据的判据**：`snapshot` 认得出内容变化 —— 不然上面那条判据在空转。
#[test]
fn the_snapshot_notices_a_change() {
    let dir = tempfile::tempdir().expect("临时目录");
    let file = dir.path().join("probe.toml");
    std::fs::write(&file, "k = 1\n").expect("写探针");

    let before = snapshot(dir.path());
    assert_eq!(before.len(), 1, "反空转：探针目录里应有一份");

    std::fs::write(&file, "k = 2\n").expect("改写探针");
    let after = snapshot(dir.path());
    assert_ne!(before, after, "内容变了而快照没变 —— 这条判据的检测器坏了");
}

/// 生成与检查路径跑一遍，基线目录**一个字节都不许变**。
#[test]
fn the_generate_and_check_paths_do_not_touch_the_baseline() {
    let base = fixtures_dir();
    let before = snapshot(&base);
    assert_eq!(
        before.len(),
        9,
        "反空转：对照基线应当正好 9 份，实测 {}",
        before.len()
    );

    let assets = assets_dir();
    let r = recipe();

    // ① 两条只读检查：产物 vs 配方（K-G7）、产物 vs 基线（K-G0'）
    let report = check_all(&r, &assets).expect("检查能跑完");
    assert_eq!(
        report.checked, 9,
        "只比了 {} 份（产物 vs 配方）—— 这条判据在空转，K-G7 那边会先红",
        report.checked
    );
    let report = check_baseline(&assets, &base).expect("检查能跑完");
    assert_eq!(
        report.checked, 9,
        "只比了 {} 份（产物 vs 基线）—— 这条判据在空转，K-G0' 那边会先红",
        report.checked
    );

    // ② 生成路径：写进**临时目录**（真 `--write` 写的是 assets/，这里不碰仓库；
    //    工作台那条 `wb_generate` 写的是 dist-presets/ —— 两者都不该碰基线）
    let tmp = tempfile::tempdir().expect("临时目录");
    assert_eq!(write_all(&r, tmp.path()).expect("写产物"), 9);

    let after = snapshot(&base);
    assert_eq!(
        before, after,
        "普通生成 / 检查路径动了对照基线 —— 它只能被 `gen-presets --sync-baseline` \
         显式改（人先看过差异），见 `docs/ARCHITECTURE.md` §10.6"
    );
}
