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

use std::path::PathBuf;

use mkp_support_ease_lib::fsx::atomic::atomic_write;
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

    let catalog = runtime::catalog::build_from_repo(&repo)?;
    let json = catalog.to_pretty_json()?;

    let out = manifest.join("src/runtime/catalog.generated.json");
    atomic_write(&out, json.as_bytes())?;

    println!(
        "已写出 {}（{} 台机型 / {} 个版本 / {} 份文件，revision {}）",
        out.display(),
        catalog.machines.len(),
        catalog.machines.iter().map(|m| m.versions.len()).sum::<usize>(),
        catalog.files.len(),
        catalog.revision
    );
    Ok(())
}
