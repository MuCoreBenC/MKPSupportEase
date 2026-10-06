# 下一刀：Rust 工具链 / 纪律清理（预存红，**未开工**）

> 2026-10-07 开立。作者裁定：**这一刀现在不做**，也不许混进「首页按套餐消费」那一刀的 commit ——
> 那一刀一个 `.rs` 都没改（`git diff --stat` 里没有任何 Rust 文件），**本刀不负责修 Rust 红**。
> 真机四条全绿 → 那一刀提交、合并 → **然后**再来单独处理这一批。

## 1. 现状（本机 2026-10-06/07 实测）

工具链：rustc **1.97.1** / rustfmt **1.9.0-stable**（8bab26f4f6 2026-07-14）/ clippy **1.97.0**。
仓库**没有** `rust-toolchain.toml` ⇒ 用哪一套取决于各人机器上装了什么。

### `cargo fmt --check`：3 处漂移

| 文件 | 处 | 形态 |
|---|---|---|
| `src-tauri/src/ipc/catalog.rs` | 2 | 「一行本来放得下却换了行」（`let source = runtime::source::make_source(...)?;`）、`.and_then(\|rel\| …)` 的折行方式 |
| `src-tauri/src/ipc/mine.rs` | 1 | 块注释续行的缩进（与下面 clippy 那条 doc 缩进是同一样东西的两面） |

### `cargo clippy --all-targets -- -D warnings`（默认 + `--features workbench`，**两边都要跑**）

| 条数 | 是什么 |
|---|---|
| 10 | `disallowed_methods: std::fs::write`（`clippy.toml` 的禁名单，**对测试代码也生效**）⇒ 一律换 `fsx::atomic::atomic_write` |
| 1 | doc 列表项没有缩进（`clippy::doc_lazy_continuation`）⇒ 补全注释缩进 |

两个 feature 报的是**同一批**（同一份源码，不是两套问题）。

## 2. 开工前先定一件事：工具链对齐

形态很像**工具链比上次全绿时更新了**（fmt 的折行偏好**和**一条新 lint 一起出现），而不是谁提交时没跑。
动手之前先定：

- 是"用本机这一套把全仓重排一遍"，还是"钉一个版本写进 `rust-toolchain.toml`"？
- 两个人各用自己的版本格式化一遍 = 反复横跳。**定了再动手**，别一边改一边吵。
- **以 CI 为准**：CI 有 macos / ubuntu / windows 三个 job，它们的 rust 版本可能又不一样。

## 3. 清理清单

1. `cargo fmt` 收掉那 3 处 —— **只这一步会动到别的刀的文件**：单独一个 commit，写明"纯格式、不改语义"。
2. 10 条 `std::fs::write` → `fsx::atomic::atomic_write`。
   **不要**用 `#[allow]` 糊过去（禁名单就是要覆盖测试）；确实只能流式写的（历史上 `updater` 那一处）
   要在代码里写**理由 + 退役条件**。
3. 1 条 doc 缩进 → 补缩进。

## 4. 完成条件

- `cargo fmt --check` 干净；
- **两个 feature** 的 `cargo clippy --all-targets -- -D warnings` 都干净；
- `cargo test` + `cargo test -p mkp-support-ease --features workbench --lib` 仍全绿；
- CI 三个 job 全绿（**以 CI 为准**，不以本机为准）。
