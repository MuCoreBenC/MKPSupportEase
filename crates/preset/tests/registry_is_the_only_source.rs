//! **注册表只有一份**（b04 Task 15 / M4c）。
//!
//! 搬进来之前，`crates/preset/assets/param_registry.toml` 与我们的
//! `presets/registry/param_registry.toml` 是同一份数据的两个副本，实测差 5 处：
//! 两处 `label`（下笔/收笔 → 装载/卸载胶箱）、两处 `uiComponent`（segmented → select）、
//! 一块 `[[params.choices]]`（`wiping.ironing_coverage_threshold` 多 0/50/90 三条）。
//!
//! 那 5 处**都不会让任何判据变红**：`label` / `uiComponent` 在 `ParamEntry` 里只有
//! serde 读写、没有分支读；`choices` 的白名单在 `validate.rs` 上有
//! `value_type == "string"` 这道门，而那条参数是 `float`；`build.rs` 那处 deprecated
//! 检查要求 `deprecated = true`，新增那 3 条都没写。
//!
//! 换句话说：**双真相在这里是静默的**。所以必须有一条判据专门咬"只有一份"，
//! 而不是指望现有判据顺手发现。
//!
//! 这条判据比"逐字段相等"更强一档：它比**全文字节**。逐字段比只覆盖
//! `ParamEntry` 认识的键，而注册表里还有它不读的字段（`jsonKey` / `mergeGroup` /
//! `showWhen` / `serialization` …）—— 那些字段漂移了，逐字段判据看不见。

use std::path::{Path, PathBuf};

/// 唯一真源的路径。与 `lib.rs` 那个 `include_str!`、`registry_edit::registry_path()`
/// 指同一个文件；三处只要有一处漂了，下面第一条断言就红。
fn the_only_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/registry/param_registry.toml")
}

/// 那份被删掉的分岔副本。**它不许回来。**
fn the_deleted_copy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/param_registry.toml")
}

#[test]
fn embedded_registry_is_byte_identical_to_the_repo_source() {
    let path = the_only_source();
    let from_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到唯一真源 {}：{e}", path.display()));

    assert_eq!(
        preset::PARAM_REGISTRY_TOML.len(),
        from_disk.len(),
        "编进二进制的注册表与 {} 长度不同 —— 要么 include_str! 指错了文件，\
         要么工作区那份被改过而没重新编译",
        path.display()
    );
    assert!(
        preset::PARAM_REGISTRY_TOML == from_disk,
        "编进二进制的注册表与 {} 内容不同（长度相同但字节不同）",
        path.display()
    );
}

#[test]
fn the_diverged_copy_is_gone_and_must_not_come_back() {
    let copy = the_deleted_copy();
    assert!(
        !copy.exists(),
        "{} 又出现了 —— 那是 M4c 删掉的分岔副本。\
         注册表只许有 presets/registry/ 那一份；要加字段就改真源",
        copy.display()
    );
}

/// 反空转：上面两条都靠"解析出来是同一份文本"，但如果两边同时变成空文件也会相等。
#[test]
fn the_only_source_actually_carries_the_registry() {
    let reg = preset::load_param_registry();
    assert_eq!(
        reg.params.len(),
        74,
        "真源应有 74 条 [[params]]（实测基线）；条数变了就该在提交里说清为什么"
    );
    assert_eq!(reg.tabs.len(), 8, "真源应有 8 个 [[tabs]]（实测基线）");

    // 那 5 处差异里唯一有结构的一处：这条参数在我们那份里带 3 条 choices，
    // 而它是 float —— 证明我们读到的是**我们那份**，不是来源仓库那份。
    let entry = reg
        .params
        .iter()
        .find(|p| p.param_key == "wiping.ironing_coverage_threshold")
        .expect("真源应有 wiping.ironing_coverage_threshold");
    assert_eq!(entry.value_type, "float");
    assert_eq!(
        entry.choices.len(),
        3,
        "我们那份给这条参数加了 0/50/90 三个预设档（来源仓库那份没有）—— \
         这条断言是「读到的是真源」的指纹"
    );
    assert!(
        entry.choices.iter().all(|c| !c.deprecated),
        "那 3 条 choices 都不是 deprecated；写成 deprecated 会让 build() 开始拦值"
    );
}
