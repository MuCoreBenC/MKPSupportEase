//! 构建随包 catalog —— 层① → 层② 的"做菜"第一刀。
//!
//! ```text
//! <repo>/presets/*.toml（源定义） + crates/preset/assets/presets/*.toml（入库产物真字节）
//!          │  本命令（发布构建）
//!          ▼
//! src-tauri/src/runtime/catalog.generated.json   ← 编进二进制，随安装包走
//! ```
//!
//! 改了源或产物之后跑一遍：
//!
//! ```text
//! cargo run --bin gen-catalog
//! ```
//!
//! 忘了跑也不会带病上线：`runtime::tests::embedded_matches_rebuild` 会在测试里红掉
//! ——「产物没有未经审阅的变化」由判据守着，与对照基线同一思路。

use std::path::{Path, PathBuf};

use mkp_support_ease_lib::fsx::atomic::atomic_write;
use mkp_support_ease_lib::presetdata;
use mkp_support_ease_lib::runtime;

fn main() {
    if let Err(e) = run() {
        eprintln!("gen-catalog 失败：{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest
        .parent()
        .ok_or("src-tauri 上面应该就是仓库根")?
        .to_path_buf();

    let catalog = runtime::Catalog::build_from_repo(&repo)?;
    let json = catalog.to_pretty_json()?;

    let out = manifest.join("src/runtime/catalog.generated.json");
    atomic_write(&out, json.as_bytes())?;

    println!(
        "已写出 {}（{} 台机型 / {} 个版本 / {} 份文件，revision {}）",
        out.display(),
        catalog.machines.len(),
        catalog
            .machines
            .iter()
            .map(|m| m.versions.len())
            .sum::<usize>(),
        catalog.files.len(),
        catalog.revision
    );

    write_delivery_catalog(&repo)?;

    Ok(())
}

/// 重算**交付面的 catalog.json**（`presets/delivery/catalog.json`）。
///
/// 与工作台 `write_catalog_json` **同一条构建内核**（`build_from_presets_lenient`
/// 对 `presets/delivery/mkp/presets/` 真字节算 SHA + 规则表填 minClient）——
/// 这里是同一台机器的 CLI 出口（CI / 批量重建用），产物应与工作台逐字节一致；
/// `delivery::catalog_matches_rebuild` 判据盯着这一致性。
///
/// ★ 只写 catalog.json 一份：manifest（版本/时间戳/渠道）与 source.json（寻址规则）
/// 是**发布事务**的定稿产物，归 `publish_into` —— 生成不替发布定稿。
fn write_delivery_catalog(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use runtime::catalog::Catalog;

    let presets_root = repo.join(runtime::catalog::REPO_PUBLISH_ROOT);
    let mut presets = crate::presetdata::Presets::load_from(&presets_root)?;
    presets.set_asset_root(&repo.join(runtime::catalog::REPO_ASSET_ROOT));

    let delivery_root = presets_root.join("delivery");
    let mut catalog =
        Catalog::build_from_presets_lenient(&presets, &delivery_root.join("mkp/presets"));
    let rules = runtime::structure::RuleTable::load(&presets_root)?;
    catalog.apply_min_client(&rules);

    let out = delivery_root.join(runtime::source::CATALOG_FILE);
    /*
     * **保留旧目录的三格发布信息**（2026-10-28 起多了发布身份）：
     *
     * | 格 | 为什么不能由生成定 |
     * |---|---|
     * | `publishedAt` | 发布事件的戳，不是生成 / 重建的 —— 一次 gen-catalog 不许把"发布时间"顶成"构建时刻" |
     * | `catalogId` | 发布线身份，只有发布账本持有；生成乱写就把线认错了 |
     * | `publishSerial` | **序号只由正式发布事务 +1**；生成重算目录不是发布事件 |
     *
     * 与工作台生成**同一口径**（`delivery::PublishIdentity::inherit_from` +
     * `published_at_of`），否则同一批输入两边写出的 catalog 会差一代。
     */
    if let Ok(bytes) = std::fs::read(&out) {
        if let Ok(prev) = Catalog::parse(&bytes) {
            catalog.published_at = prev.published_at;
            catalog.catalog_id = prev.catalog_id;
            catalog.publish_serial = prev.publish_serial;
        }
    }
    atomic_write(&out, catalog.to_pretty_json()?.as_bytes())?;
    println!(
        "已写出 {}（交付面 catalog，{} 份文件，revision {}，minClient {}）",
        out.display(),
        catalog.files.len(),
        catalog.revision,
        catalog
            .min_client_version
            .as_deref()
            .unwrap_or("<规则表没登记>")
    );
    Ok(())
}
