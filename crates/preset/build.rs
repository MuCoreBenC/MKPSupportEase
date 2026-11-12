//! 从 `assets/presets/` **自动生成** `BUILTIN_PRESETS` 表。
//!
//! # 为什么要有这个脚本
//!
//! 原来 `src/lib.rs` 里那张表是手写的（`include_str!` 的路径必须是字面量，
//! 没法在编译期遍历目录）。于是改了版本 id、生成器写出新名产物之后，表还停在旧名：
//! 客户端随包预设里就缺了新版本，而编译期一声不响 —— 作者 2026-11-12 踩到的正是这个。
//!
//! 现在表由 `OUT_DIR` 里的生成文件提供：目录里有什么，表里就有什么，顺序固定（排序）。
//! 生成动作（`wb_generate`）写完产物、清掉旧名之后，下一次编译自动跟上，
//! **不需要人手改任何源码表**。
//!
//! `include_str!` 仍然只吃字面量：这里为每一份生成一条
//! `concat!(env!("CARGO_MANIFEST_DIR"), "/assets/presets/<名字>")`，
//! `concat!` 在编译期展开成字面量，编译器照旧保证文件真的存在。
//!
//! 表名与内容的双向一致性由 `tests/builtin_presets_match_dir.rs` 钉住，
//! 「清单 ↔ 产物名集合」由工作台 `presets/rendered` 与随包判据守着。

use std::path::{Path, PathBuf};

/// 写的是 Cargo 的 `OUT_DIR`（每次构建重建的生成文件），不是用户数据、也不是
/// 仓库真源：崩在半路顶多下次构建重跑一遍，不会留下半个被截断的用户文件。
/// 与 `src/generate.rs::sync_baseline` 同属「写自己那份派生文件」，但那边改的是
/// 判据期望值，风险高得多；这里没有那一层，所以只需一条就地说明的豁免。
#[allow(clippy::disallowed_methods)]
fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/presets");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读不到内置预设目录 {}：{e}", dir.display()))
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".toml"))
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "{} 里一份 .toml 都没有 —— 表空了等于随包预设全丢",
        dir.display()
    );

    for name in &names {
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.'),
            "预设文件名 {name:?} 含不安全字符，不能拼进 include_str! 路径"
        );
        println!("cargo:rerun-if-changed={}", dir.join(name).display());
    }

    let mut out = String::from(
        "/// 由 `build.rs` 从 `assets/presets/` 自动生成，改 id 后重新生成产物即可，不要手改。\n\
         pub const BUILTIN_PRESETS: &[(&str, &str)] = &[\n",
    );
    for name in &names {
        out.push_str(&format!(
            "    ({name:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/presets/{name}\"))),\n"
        ));
    }
    out.push_str("];\n");

    let dest = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR 由 Cargo 提供"))
        .join("builtin_presets.rs");
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("写不出 {}：{e}", dest.display()));
}
