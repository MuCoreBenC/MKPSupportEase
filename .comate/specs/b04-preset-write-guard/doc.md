# B04 —— 给 `crates/` 补上写盘纪律（preset 能写真源这件事没人管）

日期：2026-09-24　分支：`feat/b04-p3-migration`
上游：Task 15 / M4 的收尾发现（见 `.comate/specs/b04-m4-preset-unify/summary.md` §5）
长期总纲：`.comate/specs/b04-panel-successor/tasks.md`。本轮属于 **Task 19（写入纪律）的提前一小块**，
不是新任务 —— 因为 M4 刚把一把枪装好（`registry_edit` 现在指着真数据），不能等到 Task 19 再说。

---

## 0. 为什么现在做

M4c 把 `registry_edit::registry_path()` 从 crate 自带的副本改成了
`presets/registry/param_registry.toml`（真源）。于是：

- 它**有能力覆盖真数据**，而真数据的验收是"12 个文件 sha256 全程不变"—— 那是**人工核的**，代码里没有判据
- 唯一的缓和因素：`set_range` / `set_range_in_text` 在生产代码里**零调用者**，
  `src-tauri` 根本不依赖 `mkpse-preset`（`src-tauri/Cargo.toml` 的依赖表里没有它）。
  **装好的枪还没接扳机** —— 所以我们有时间把闸做对，而不是赶着补

---

## 1. 现状核准（全部实测）

### 1.1 两个新 crate 完全在纪律之外

| 机制 | 配置在哪 | 作用范围 |
|---|---|---|
| Clippy 禁列（A1 方案） | `src-tauri/clippy.toml`（29 行，5 条 `disallowed-methods`） | **只有 `src-tauri`** |
| 源码扫描断言（C 方案） | `src-tauri/src/workbench/upstream/mod.rs:114` | **只扫 `src/workbench/upstream/` 一层目录** |
| `unsafe_code = "forbid"` | 根 `Cargo.toml:40` 的 `[workspace.lints.rust]` | preset / postprocess 领；src-tauri 刻意不领 |

`crates/preset` 与 `crates/postprocess` **两条写盘纪律都不覆盖**。

### 1.2 `clippy.toml` 的查找规则 —— 做了两次探针实测，不是查文档

| 实验 | 做法 | 结果 |
|---|---|---|
| 一 | 根放 `clippy.toml`，禁 `std::fs::write` | preset / postprocess **被管**，报出 4 处 |
| 二 | 根那份改成禁 `std::fs::read_to_string`（src-tauri 里到处在用） | `src-tauri` **零命中**；preset 侧报 5 处 |

⇒ **就近优先，不合并**：有自己那份的 crate 用自己那份，没有的向上找到根那份。
两份探针文件都已删除，工作区干净。

**这条规则决定了方案**：放一份根 `clippy.toml` 就能覆盖两个新 crate，
且完全不影响 `src-tauri`（它继续读自己那份）—— 但那样就有**两份禁列各自维护**，
迟早漂移。所以我倾向**把 `src-tauri/clippy.toml` 整份上移到仓库根**，全 workspace 一份。

### 1.3 现有生产写盘点（clippy 禁列一开就会红的，生产 5 处）

> **实测口径（这一条是坑）**：`cargo clippy` 会跳过未变更 crate 的重新 lint。
> 第一次把 `clippy.toml` 放到根上跑，只报 **9 处**且 `crates/postprocess` 一处都没有；
> `cargo clean -p mkpse-preset -p mkpse-postprocess` 之后重跑，变成 **15 处**。
> **改 `clippy.toml` 之后不 clean，拿到的命中清单是假的。**

15 处 = **生产 5 处**（下表）+ **测试 10 处**（§1.3.1）。

| # | 位置 | 写到哪 | 现有闸 | 性质 |
|---|---|---|---|---|
| 1 | `crates/preset/src/registry_edit.rs:195` | **`presets/registry/param_registry.toml`（真源）** | 只有"无改动不写" | **本轮主目标** |
| 2 | `crates/preset/src/generate.rs:284`（`sync_baseline`） | **`crates/postprocess/tests/fixtures/presets/`** | 内容相同就不写 | **它能改内核判据的夹具**，比 #1 更隐蔽 |
| 3 | `crates/preset/src/generate.rs:145`（`write_all`） | `crates/preset/assets/presets/` | 无 | 开发工具产物，多余文件还不删 |
| 4 | `crates/postprocess/src/pipeline/mod.rs:597..606` | 用户指定的输出文件 | **已是 `.part` + rename 原子模式**，rename 失败回落 copy | 语义正当 |
| 5 | `crates/postprocess/src/main.rs:248` | CLI 输出文件 | 无 | 语义正当（命令行工具本职） |

### 1.3.1 测试里的 10 处（`--all-targets` 会编它们，所以也会红）

`crates/preset/src/generate.rs:335` / `:408`（同文件的 `#[cfg(test)] mod tests`）、
`crates/preset/tests/` 的 `lineage.rs:90` / `load_ir.rs:124` / `recipe.rs:118` /
`registry_branch_diff.rs:128`、`crates/postprocess/tests/` 的 `cli.rs:246` /
`golden_common/mod.rs:68` / `pipeline.rs:67` / `pipeline.rs:205`。

**这 10 处都是正当的**（写临时文件、造夹具、`UPDATE_GOLDEN` 那条路），
处置方式是**模块级 / 文件级一次性豁免**，不是逐行加属性 —— 理由统一写
「测试写临时文件是正当的；这条纪律管的是生产代码」。
`#[cfg(test)]` 里的写盘本来也不在源码扫描断言（C 方案）的视野里，两边口径一致。


`registry_edit.rs:195` 的写盘是**裸 `std::fs::write`**：截断式、无备份、无原子、无写后回读。
写盘前的校验（`min > max` / `step <= 0` / `defaultValue` 落在新区间外 / 键不存在）
全在纯内存阶段的 `set_range_in_text` 里；而"只许那三行变"那条逐行断言
**只存在于测试**（`registry_edit.rs:227`），生产路径不跑。

### 1.4 顺带查到的两处失效引用

- `crates/preset/src/generate.rs:12` 提到门禁 `scripts/check_generator_purity.py` —— **本仓不存在**
- `recipe_edit::set_override`（`recipe_edit.rs:129`）文档写着"字段不存在时**新增一行**"，
  与 `write.rs` 的 K-P3「拒绝新增键」立场**相反**。两者各有道理（配方 vs 用户预设），
  但这个反差今天没在任何地方写明

---

## 2. 这一轮要达到什么

1. `crates/*` 的写盘面被 Clippy 禁列覆盖，**全 workspace 一份配置**
2. 每一处生产写盘点要么**走统一的原子写**，要么有**精确列出理由与退役条件**的豁免
3. 写真数据那一处（#1）加真正的闸：原子写 + 写后回读复检 + 生产路径自检"只有那三个键变"
4. 加一条源码扫描断言，覆盖 `crates/preset/src/` 与 `crates/postprocess/src/`，
   **并用探针验证它会响**（照 `upstream/mod.rs:160` 那条的做法）

---

## 3. 方案分层

### A —— `clippy.toml` 上移到仓库根（覆盖面）

把 `src-tauri/clippy.toml` 整份移到根，`src-tauri` 那份删掉。5 条禁列原样带过来。

- 收益：一份配置管三个成员；新加 crate 自动被管
- 代价：§1.3 那 5 处**立刻变红**（CI 是 `-D warnings`），必须在同一批里处置完
- 风险点：`src-tauri` 现在靠"就近"用自己那份，上移后它读根那份 —— 内容不变，
  唯一逃生口 `fsx/atomic.rs:23` 的 `#[allow]` 不受影响

### B —— 原子写下沉，让 `crates/*` 用得上

`fsx::atomic` 住在 `src-tauri`，而依赖方向是 `src-tauri → preset → postprocess`，
**preset 用不了它**。三个选项：

| 选项 | 做法 | 代价 |
|---|---|---|
| **B1** | 把原子写下沉到 `crates/postprocess`（preset 依赖它），`src-tauri` 那份改成转调 | 动内核 —— 而内核这一轮刚搬完、判据靠"零 diff"背书 |
| **B2** | `crates/preset` 里自己写一份小的原子写（同目录临时文件 + fsync + persist） | 两份实现，会漂 |
| **B3** | 只给 #1 加原子写（就地实现），#3/#5 保持裸写但记豁免 | 最小改动面；但"原子写只有一处"这件事要写清 |

我的判断：**B3**。理由是 #1 是唯一写真数据的点，#3 写的是 crate 自己的 assets、
#5 是 CLI 写用户指定的输出 —— 后两者崩溃留半个文件的后果是"重跑一次"，
不是"真源坏掉"。等 Task 19 统一写盘入口时再做 B1。

### C —— 源码扫描断言（覆盖 `crates/*`）

照 `upstream/mod.rs:114` 那条的形状新写一份，但要改两点：

1. **扫描范围递归**（那条只扫一层 `read_dir`，preset 有 `src/bin/` 子目录）
2. **不用 `file!()`** —— 那条注释里的血泪：建 workspace 之后 `file!()` 的基准从 crate 根
   变成仓库根，判据会以"报错"形式失效。用 `CARGO_MANIFEST_DIR` + 显式相对路径

断言内容：`crates/preset/src/` 与 `crates/postprocess/src/` 的生产部分
（`#[cfg(test)]` 之前、去掉 `//` 注释行）里，写盘 API 只允许出现在**白名单文件**，
白名单逐条写理由。

---

## 4. 逐处处置（草案，落地时逐条确认）

| # | 处置 | 理由要写进代码注释 |
|---|---|---|
| 1 `registry_edit.rs:195` | **改成原子写 + 写后回读 + 落盘前自检"只有 min/max/step 那几行变"** | 它写的是真源；"只改三行"从测试提升到生产路径 |
| 2 `generate.rs:284` `sync_baseline` | 加 `#[allow]` + 理由，**并在函数文档里写明它会改内核判据夹具**；考虑加"目标必须在 `crates/postprocess/tests/` 下"的路径断言 | 它能动判据基线，这件事必须显式 |
| 3 `generate.rs:145` `write_all` | 加 `#[allow]` + 理由（开发工具、写自己 assets、默认动作是 `--check`） | — |
| 4 `pipeline/mod.rs:597` | 加 `#[allow]` + 理由（已是 `.part` + rename 原子模式） | 已有正确语义，别重写 |
| 5 `main.rs:248` | 加 `#[allow]` + 理由（CLI 写用户指定输出） | — |

**豁免的写法照 PR #10 的定案**：精确列出理由**与退役条件**；「clippy 通过」不等于「写盘纪律已验证」。

---

## 5. 边界与异常

- **不动内核的行为**：#4 只加属性与注释，不改一行逻辑（它的 golden 是逐字节判据）
- **不给 `registry_edit` 加备份**：备份要定"放哪、留几份、谁清理"，那是 Task 19 的范围。
  本轮用"原子写 + 写后回读失败即报错"替代 —— 它保证的是"要么旧的、要么新的完整版"
- **`presets/` 的 sha256 判据**：本轮**不加**自动判据（要定"什么时候允许变"，
  而 Task 18 就要往里加文件）。改为把它写进扫描断言的白名单理由里
- **失效引用**（§1.4 那两处）顺手修掉，但不扩大范围

---

## 6. 判据

1. `cargo clippy --all-targets -- -D warnings` 绿（含两个新 crate）；
   工作台那档也绿
2. 新扫描断言 + **探针**：临时在 `crates/preset/src/` 某个白名单外的文件里加一行
   `std::fs::write(...)` ⇒ 判据必须红且点名那个文件；撤掉 ⇒ 绿
3. `registry_edit` 的新闸各自有判据：
   - 写后回读不一致 ⇒ 报错（用只读目录或临时目录造一次失败）
   - 落盘前自检：构造一个"会多改一行"的输入 ⇒ 拒绝落盘，且**文件一个字节没变**
4. 三组老数字不变：默认 18 / 内核 249+2 / 预设 106+ / 工作台 264
5. `presets/` 12 个文件 sha256 不变（人工核，与前几轮同口径）

---

## 7. 不在本轮范围

- 统一写盘入口、备份与回收站语义（Task 19 本体）
- 原子写下沉到内核（B1）
- `recipe_edit` 与 `write.rs` 的立场反差要不要收口（只在本轮记录，不动代码）
- `presets/` sha256 的自动判据

---

## 8. 需要你点头的三处

1. **`clippy.toml` 上移到仓库根、删掉 `src-tauri` 那份** —— 代价是 §1.3 那 5 处
   必须在同一批处置完。替代方案是"根加一份、src-tauri 保留一份"，覆盖面一样但两份会漂
2. **原子写取 B3（只给写真源那一处加，就地实现）**，不动内核。
   如果你要一次做到 B1（下沉到内核），范围会大一圈且要重新验证内核判据
3. **`sync_baseline` 要不要加"目标必须在 `crates/postprocess/tests/` 下"的路径断言** ——
   它是唯一能改内核判据夹具的写盘点。加了更稳，但也是本轮唯一一处"新增运行时检查"
