# 增量之三十：Rust 工具链 / 纪律清理

> 开立 2026-10-07（紧跟增量之二十九收刀 `31200ba`），**已施工完成**（2026-10-07）。
> 之所以单开一刀：那一刀一个 `.rs` 都没改，**本刀不负责修 Rust 红**；两件事不许搅在一个 commit / PR 里。

## 1. 先做的不是改代码，是工具链对齐（2026-10-07 实测）

| | 版本 | 怎么知道的 |
|---|---|---|
| **CI**（`rust` / `rust-windows` 两个 job） | **rustc 1.99.0 (b940084d7 2026-09-28)**，rustfmt **1.10.0** | CI 用 `rustup toolchain install stable` + `rustup default stable`（**floating**）；从真实运行日志里读到（run 37427860087 打印 `rustc 1.99.0`） |
| 本机（当时 default） | rustc 1.97.1（2026-07-14），rustfmt 1.9.0，clippy 1.97.0 | `rustc -V` / `cargo fmt --version` |
| 仓库 | **没有 `rust-toolchain.toml`** ⇒ 用哪套取决于各人机器上装了什么 | `ls rust-toolchain*` |

**关键结论：这批红不是工具链漂移。** 装上 CI 同版本（1.99.0）重跑，报出来的是**同一批**：
1.99 / 1.97 两个版本都指向 `runtime/delivery.rs`（8）、`runtime/preset_events.rs`（1）、`runtime/provenance.rs`（1）。
⇒ 是 **feat/app-state 那一刀新写的代码踩了 `clippy.toml` 既有的写盘纪律**（那 13 个提交没推过，CI 从没见过它们），
与"谁的工具链更新"无关。**CI 上 main 是绿的**（最近三次 push 全 success）。

`fmt` 那部分也不是版本差异：`--check` 报的行（`ipc/catalog.rs`×2、`ipc/mine.rs`×1、还有 `runtime/app_state.rs`）
在 `origin/main` 上**根本不存在**（是新写的行），即**写完没跑过 fmt**。
（第一次看漏了第 4 处 —— `--check` 的输出被 `head` 截断了；`cargo fmt` 实际动了 11 个文件。）

## 2. 纪律本身：`clippy.toml` 为什么禁 `std::fs::write`

根上那 5 条禁的是"**会截断已有文件**"的入口 —— `std::fs::write` 先把目标截断再写，崩溃时留半个文件。
文件头写明了逃生口与**退役条件**，也说清了为什么不用"约定 + code review"（失效是静默的）。

施工时按**仓库既有形状**修，没有加 `#[allow]`：

- `src-tauri` 里 `atomic::atomic_write` 有 **117 处**、`std::fs::write` 只在**注释**里出现；
  `runtime/mine.rs` 的注释原文就是「写盘走全仓唯一那个出口（`clippy.toml` 禁 `std::fs::write`）」；
- `preset_events.rs` / `provenance.rs` 顶部本来就 `use crate::fsx::atomic::atomic_write;`
  ⇒ 测试里直接换掉即可，不用 `#[allow]` 糊过去（`clippy.toml` 也写明"用 `#[allow]` 逐处写理由"是给**生产代码**的逃生口）。

## 3. 改了什么

| 类 | 处 | 做法 |
|---|---|---|
| 真问题 | 9 | 测试里 `std::fs::write` → `crate::fsx::atomic::atomic_write`（`delivery.rs` 8、`preset_events.rs` 1、`provenance.rs` 1，全在 `#[cfg(test)]` 的 `tempdir()` 里） |
| 真问题 | 1 | `delivery.rs` 的 doc 列表：末行缺缩进 ⇒ 用空 `///` 分成独立段落 |
| 纯格式 | 11 个文件 | `cargo +1.99.0 fmt`（**用 CI 同版本的 rustfmt**）—— 全是 AppState 那一刀新写的行没跑过 fmt；**1.97.1 与 1.99.0 都认** |

## 4. 完成条件（2026-10-07 实测）

- `cargo +1.99.0 fmt --check` 干净；`cargo fmt --check`（1.97.1）也干净 ✅
- `cargo +1.99.0 clippy --all-targets -- -D warnings` 干净 ✅；`--features workbench` 也干净 ✅
- `cargo +1.99.0 test` 判据 **729 passed / 0 failed** ✅；`-p mkp-support-ease --features workbench --lib` **687 passed** ✅
- 前端那一套没动（本刀零 TS 改动）。

## 5. 留给以后的一条（**没做，故意的**）

**要不要钉版本（`rust-toolchain.toml`）** —— 现在是 CI floating `stable` + 本机各人自装。
钉版本会**同时改 CI 的行为**（`rustup default stable` 会被 rust-toolchain.toml 覆盖），属于另一件事，本刀不做。
本刀采取的是"**用 CI 同版本复核**"（另装了 `1.99.0`，用 `cargo +1.99.0 …` 跑），不动本机 default、不改 CI。
要进一步对齐，二选一（都留给下一刀决定）：本机 `rustup default stable` 升上去，或写 `rust-toolchain.toml` 钉死。
