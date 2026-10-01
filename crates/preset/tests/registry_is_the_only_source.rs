//! **注册表只有一份**（b04 Task 15 / M4c）。
//!
//! 搬进来之前，`crates/preset/assets/param_registry.toml` 与我们的
//! `presets/registry/param_registry.toml` 是同一份数据的两个副本，实测差 4 处：
//! 两处 `label`（下笔/收笔 → 装载/卸载胶箱）、两处 `uiComponent`（segmented → select）。
//!
//! （原来还有第 5 处：`wiping.ironing_coverage_threshold` 多挂了 0/50/90 三条
//! `[[params.choices]]`。那三条在控件与校验两层都没有消费方 —— 它是 `float`，而
//! 「`choices` 是取值域」这道门只对 `value_type == "string"` 开 —— 于是被一路误读成
//! 「只能三选一」，真身却是一个能填的百分比框。已按同一道判据从真源删掉。）
//!
//! 那 4 处**都不会让任何判据变红**：`label` / `uiComponent` 在 `ParamEntry` 里只有
//! serde 读写、没有分支读。
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

    // 指纹：`wiping.disk_stagger_swing_mode` 在我们那份里是 `select`、来源仓库那份是
    // `segmented` —— 读到 `select` 就证明读到的是**我们那份**，不是来源仓库那份。
    let entry = reg
        .params
        .iter()
        .find(|p| p.param_key == "wiping.disk_stagger_swing_mode")
        .expect("真源应有 wiping.disk_stagger_swing_mode");
    assert_eq!(
        entry.ui_component, "select",
        "我们那份把这条的 uiComponent 改成了 `select`（来源仓库那份是 `segmented`）—— \
         这条断言是「读到的是真源」的指纹"
    );
}
