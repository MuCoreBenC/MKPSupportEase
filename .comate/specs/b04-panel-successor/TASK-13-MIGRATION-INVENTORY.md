# Task 13 迁移盘点：后处理内核

**只读取证，不含任何改动。** 这份文件回答五个问题：迁什么、依赖什么、怎么分类、
怎么切批、每批拿什么验收。落地顺序仍以 [MIGRATION-PLAN.md](./MIGRATION-PLAN.md)
为准，这里不另立一套。

## 0. 基线、口径与一句话结论

| | |
|---|---|
| 源 | `G:\project\mkp-ssr` @ `a2a47ca`（**工作区是脏的**，见 §5.4） |
| 源范围 | `crates/core`（包名 `mkp-pp`，lib `mkp_pp`） |
| 接收方 | `G:\project\MKPSupportEase` @ `6880403`（Task 12 收口后） |
| 行数口径 | `[System.IO.File]::ReadAllLines().Length`。**`Get-Content \| Measure-Object -Line` 会少算**（同一文件实测 1 642 vs 1 840 行），谁复核都请用前者 |
| 方法 | 静态取证（读源码、跑计数命令）。**没有编译、没有跑测试** —— 源树是只读的，那一侧不许写任何东西（含 `target/`） |

**一句话结论**：这一包**可以直接搬**。零本地 crate 依赖、零网络、零 `unsafe`、
零环境变量、零系统时钟入数据。真正要想清楚的只有四件 ——
**改名的影响面（64 处 / 19 文件）、两个 asset JSON 的过渡期存在理由、
一条写盘纪律的豁免要不要登记、以及 bin 那三个依赖该不该跟着 lib 一起进来**。
工作量的大头不在代码，在 **3.08 MB golden 数据**与它们那两万行的零容差比对。

---

## 1. 问题一：哪些目录和模块需要迁入

### 1.1 规模（实测）

| 部分 | 文件数 | 行数 / 体积 |
|---|---:|---:|
| `crates/core/src`（54 个 `.rs`，一个 lib + 一个 bin） | 54 | 21 217 行 |
| `crates/core/tests`（20 个 `.rs`：19 个测试二进制 + `golden_common/mod.rs`） | 20 | 4 050 行 |
| `crates/core/tests` 下的**非 `.rs` 数据**（golden / 机型配置 / 基线） | 44 | 3 226 574 字节 |
| `crates/core/assets`（2 个 JSON，`include_str!` 嵌进二进制） | 2 | 5 641 字节 |
| `crates/core/templates`（`tower_layer.gcode`，同样 `include_str!`） | 1 | 949 字节 |
| **合计要动的文件** | **121** | 25 267 行代码 + 3.23 MB 数据 |

与 `MIGRATION-PLAN.md:16` 的对照：它写 `core` src 54 文件 / 21 217 行、tests 20 文件 /
4 050 行 —— **逐项吻合**。（那份表里「合计 67 文件」是 core + preset 两包的和，
不是 core 一个包的，别误读。）

### 1.2 `src` 的模块图（谁依赖谁）

```
main.rs（bin：cli 解析 / dump-ir / check）
  └─ config ──┬─ ir ── diag
              └─ diag
  └─ pipeline ─┬─ config / diag / ir
               ├─ postproc ──┬─ ir / diag / gcode
               │             └─（自身各子模块）
               └─ gcode  ← 唯一一处跨层跳跃：pipeline/hooks.rs
```

- **`gcode` 是真叶子**：`src/gcode/**` 里没有任何 `crate::` 引用（`line_builder.rs`
  那处是 `cfg(test)` 内部自引用）。
- **零反向依赖**：全 `src` 里 `crate::pipeline` / `crate::config` 只出现在 `pipeline`
  自身；没有下层反向引用上层。
- 六条对外出口：`diag`（`CancelToken` / `PostprocError`）、`ir`（`fill_defaults` /
  `num_strip` / `types::*`）、`gcode`（`coord::*` / `format::*` / 伪随机复位）、
  `postproc`（只 `pub mod`）、`config`、`pipeline`。

### 1.3 测试面（20 个文件、各读什么）

| 测试 | 依赖的数据 |
|---|---|
| `golden_g1` / `golden_g2` | `tests/golden/42274.2.gcode` + `42274.2.expected.gcode` / `.pass2.expected.gcode` + `tests/fixtures/ir/golden_42274_2.json` |
| `golden_g3` | `tests/golden/tower_matrix.golden` |
| `golden_integrity` | 上述四份 golden，**sha256 + 字节数钉死** |
| `cli` / `cancel` / `process_with_ir` / `end_to_end` / `machine_dims_must_exist` / `pipeline` / `pipeline_wiping_source` / `marks_protocol` / `perf_probe` | `golden/42274.2.gcode`（10 个文件都读它）+ `tests/fixtures/config/A1.toml` / `tests/fixtures/ir/build9/*.json`（9 份）/ `fixtures/presets/*.toml`（9 份） |
| `calibration_insert` / `pass1_state` | `fixtures/ir/golden_42274_2.json` |
| `reference_calibration` | `tests/reference/calibration/{ZOffset,Precise,Rough,Repetition}.input.json` + 对应 `.expected.gcode`，另有 4 组 `.new.*` |
| `reference_printtime` | `tests/reference/printtime/input.gcode` + `expected_result.json` |
| `gcode_real_segments` | 无外部数据（片段内联为常量） |
| `boundaries` | 无数据，但**扫目录**（见 §5.2） |

用例规模：`#[test]` 共 **233 处**（`src` 内联 146 + `tests/` 87）。
其中 `golden_common/mod.rs` 的 3 条被 6 个测试二进制各编译一份，所以 `cargo test`
实际跑的条数**约 248** —— 这是**推算**，M2 落地时请以真实输出为准并记进那段验收。

---

## 2. 问题二：依赖什么，哪些会违反分层

### 2.1 第三方依赖（9 + dev 1）

| crate | 用途落点 | 备注 |
|---|---|---|
| `thiserror` / `tracing` / `serde` / `toml` / `sha2` | lib | `serde_json` 见下 |
| `serde_json` | lib + bin | **必须带 `features = ["float_roundtrip", "preserve_order"]`**，见 §5.1 |
| `tracing-subscriber` | **bin（`main.rs`）+ lib 的一条测试** | `pipeline/progress.rs` 的 `#[test] fn emitting_progress_writes_zero_log_events` 用它装捕获层 |
| `clap` / `ctrlc` | **只有 bin** | Ctrl-C → `CancelToken` |
| dev：`tempfile` | 测试 | 只有 `process_with_ir.rs` 一处用 |

### 2.2 本地 crate 依赖：**零**

`Cargo.toml` 里没有任何 `path` 依赖。四处提到 `mkp_preset` / `src-tauri` 的地方
全是注释（`ir/defaults.rs:5`、`postproc/support.rs:60`、`postproc/printtime.rs:1245`），
`ir/types.rs` 里的 `preset_name` 只是 IR 的字符串字段。**它不依赖 `crates/preset`，
反而 `crates/preset` 依赖它** —— 所以先搬它、后搬 preset 的顺序是安全的。

### 2.3 运行时对外界的全部接触面（这就是全部，没有别的）

| 类别 | 实测 |
|---|---|
| `include_str!` | 3 处：`postproc/machine_dims.rs:14,16`（两个 asset JSON）、`postproc/tower/template.rs:3`（`templates/tower_layer.gcode`）。**都是 crate 内相对路径，搬家后照用** |
| 环境变量 / 当前工作目录 / 绝对路径 | `src` 内**零**。测试用 `UPDATE_GOLDEN`（`tests/golden_common/mod.rs`）与 `env::temp_dir()` |
| 网络 / 随机数 | **零**。随机取 `gcode` 的确定性伪随机（`gcode/mod.rs` 导出 `reset_pseudo_random`） |
| 系统时钟 | **无 `SystemTime` / 无 `chrono`**。有 `Instant`，只用于取消位的 100 ms 节流（`postproc/cancel.rs` + 4 处调用点）——**不进任何输出字节**，所以输出可复现 |
| `unsafe` | **零**（`ir/types.rs` 那个 `unsafe_close` 只是字段名）。crate 自己领 `unsafe_code = "forbid"` |
| `std::process::Command` / 线程 | `src` 内零（只在测试里起子进程）；`std::thread` 只在 `diag/cancel.rs` 的 `cfg(test)` 里 |
| 运行时文件 IO | 只有 4 处：`config.rs:35,46`（读 TOML）、`pipeline/mod.rs:349`（读 G-code）、`pipeline/mod.rs:597-606`（写 `.part` 再 rename）、`main.rs:248`（写） |
| `unwrap()` / `expect()` | `src` 内 21 处 / 34 处。**要紧的一处**：`postproc/machine_dims.rs` 里由 `include_str!` 解出的 JSON 用 `expect` 兜着 —— 那两个 asset 一旦被改坏就是 panic，不是报错 |

### 2.4 与接收方的版本对照（会撞车的地方）

| crate | 我们（`src-tauri/Cargo.toml`） | 它 | 结论 |
|---|---|---|---|
| `serde_json` | `1.0`，**没开 `float_roundtrip`** | `1.0` + `float_roundtrip` + `preserve_order` | ⚠️ **workspace feature 是并集**，统一到根之后我们也会拿到这两个 feature → **我们的 264 条必须重跑**。这是本次迁移唯一可能改动既有行为的依赖项 |
| `toml` | **dev-dependencies** `1.0.7`；生产用 `toml_edit 0.22` | 生产依赖 `toml` | 统一到根：`toml` 从 dev 升成生产依赖，版本取较新一档 |
| `thiserror` | `2` | 同族 | 取新 |
| `tracing` / `serde` / `sha2` / `tempfile` | 有 | 有 | 取新；`sha2` 我们这边是 optional（`workbench` feature） |
| `tracing-subscriber` | `0.3` + `env-filter` | `0.3` | 合并 features |
| `clap` / `ctrlc` | **没有** | 有 | 新增 —— 只服务那个 bin，但 cargo 不区分 per-target 依赖，见 §3.3 |
| `edition` / `rust-version` | `2021` / `1.77.2` | `workspace = true`（在那边根上） | 统一时取我们这档就够（`1.77.2` 是 Tauri 那边定的下限，别抬） |

### 2.5 三条要处理的分层冲突

**① 写盘纪律：`std::fs::write` 有 2 处，而接收方全禁它。**

- 接收方：`docs/ARCHITECTURE.md:81-92` §5「唯一的写盘出口」——必须走
  `fsx::atomic::atomic_write`；由 `src-tauri/clippy.toml` 的 `disallowed-methods`
  强制（`std::fs::write` / `std::fs::File::create` / `tokio::fs::write` 全禁，
  唯一豁免是 `fsx/atomic.rs` 顶上的 `#[allow]`）。
- 源：`pipeline/mod.rs:597` 与 `main.rs:248` 直接用 `std::fs::write`。
  （前者是 `.part` + `rename` 的原子替换，形态上更接近 `fsx::atomic`；后者是 CLI 的写出口。）
- **关键事实**：`clippy.toml` 躺在 `src-tauri/` 下，**不会自动作用于
  `crates/postprocess`**。也就是说这条纪律在新 crate 里**没有守卫**。
- **状态：待决策**（见 §6 第 4 条）。三个候选方案与其影响：
  (a) 新 crate 也放一份 `clippy.toml`（`disallowed-methods` 在那里重新声明一遍）；
  (b) 迁完单独一段把那 2 处接进 `fsx::atomic`（改行为，要单独验，且会让 postprocess
  依赖我们的 `fsx`，与「它只依赖 9 个第三方」这条干净边界冲突）；
  (c) 明确写「这个 crate 的写盘不属于工作台数据根」并加一条源码扫描断言把豁免钉住。
- **不能选的那种做法**：为了迁移通过而默认"它反正管不到"，留下一个**永久漏检口**。
  Task 19.1 那条「所有写盘只经一处」的全局扫描一定会撞上它 —— 要么那条扫描显式绕开
  这个 crate 并写明理由，要么就在这里把边界收掉，二者必居其一。

**② 分层 lints 不能往 workspace 根塞 clippy 规则。**

`crates/core/Cargo.toml` 的 `[lints]` 是 `workspace = true`，并且留了一段实测注释：
**在领了 workspace lints 的 crate 里不能再写 `[lints.clippy]`**（cargo 1.97 报
`cannot override workspace.lints in lints`）。所以 workspace 根的 `[lints]` 只能放
`unsafe_code` 这一类；想 deny 某条 clippy 规则得从根上统一，不能按 crate 加。
它另外**刻意不 deny `clippy::suboptimal_flops`** —— 那条建议正是 `mul_add`，
与「几何层禁 FMA 融合」直接矛盾，真守卫是 `tests/boundaries.rs` 的文本扫描。

**③ `src-tauri` 刻意不领 workspace lints。**

Task 13.4 已经定了，理由照抄：Tauri 的宏会碰到 `unsafe_code = forbid`。
落地时 `src-tauri/Cargo.toml` 要显式**不写** `[lints] workspace = true`。

---

## 3. 问题三：原样迁 / 必须适配 / 不该迁

### 3.1 可以原样迁（搬过去一个字节不改）

- `src` 全部 54 个文件：**除了 §3.2 那几处点名的地方，其余零适配**。
  路径锚点全是 crate 内相对路径，`include_str!` 搬家照用。
- `tests` 全部 20 个文件与 44 个数据文件：路径一律从 `env!("CARGO_MANIFEST_DIR")`
  起步（两种写法：`PathBuf::from(env!(...))` 包装成 `repo_root()`，或
  `Path::new(env!(...)).join(...)`），**没有硬编码绝对路径参与逻辑**
  （有 4 处 macOS 路径，都在注释里，见 §5.4）。
- `assets/` 2 个 JSON + `templates/` 1 个 `gcode`：`include_str!` 要它们在编译期存在，
  **必须先跟着搬**，去向另说（§3.4 第 1 条）。
- `tests/boundaries.rs`（236 行）：扫的是 `CARGO_MANIFEST_DIR` 下的 `src` 与 `tests`，
  两条判据分别是「`gcode` 模块保持纯净（禁 `crate::diag|ir|postproc|pipeline|config`
  与任何 `tracing`）」与「全项目禁 `mul_add`」。搬家后照跑。

### 3.2 必须适配（逐条，都不是"顺手改好一点"）

| # | 位置 | 必须改什么 | 为什么不能不改 |
|---|---|---|---|
| 1 | 19 个文件、**64 行**含 `mkp_pp` | `mkp_pp::` → `postprocess::` | Task 13.3：不留过渡别名，`rg mkp_pp` 零命中 |
| 2 | `tests/cli.rs:20`、`tests/end_to_end.rs:56`、`tests/machine_dims_must_exist.rs:37` | `env!("CARGO_BIN_EXE_mkp-pp")` 里的 bin 名 | **编译期宏**，bin 一改名就编译失败（不是运行期失败） |
| 3 | `crates/core/Cargo.toml` | 包名 `mkp-pp` → `mkpse-postprocess`、`[lib] name = "postprocess"` | Task 13.2/13.3 |
| 4 | workspace 根 | 统一依赖版本 + `serde_json` 的 `float_roundtrip`/`preserve_order` | §2.4 那两张表 |
| 5 | `.github/workflows/ci.yml` | 两处 `working-directory: src-tauri` 与两处缓存 key `hashFiles('src-tauri/Cargo.lock')` | 建了 workspace 之后锁文件在**根**，`cargo fmt --all` / `clippy --all-targets` 在 `src-tauri` 下也跑不到新 crate |

### 3.3 需要你拍一下的三件（都不是"照抄"能解决的）

**① bin 那三个依赖要不要跟着 lib 一起进来。**
`clap` / `ctrlc` 只被 `main.rs` 用，`tracing-subscriber` 还被 lib 的一条测试用。
如果 `src-tauri` 依赖这个 crate 的 **lib** 目标，这三个会进应用依赖树（cargo 没有
per-target 依赖声明）。三个选项：(a) 照搬现状，接受多编译三个 crate；
(b) 把 bin 拆成第二个 crate（`crates/postprocess-cli`），布局偏离 MIGRATION-PLAN §2 的命名表；
(c) 把三者声明为 optional，用 `required-features` 挂在 bin 上 —— 但 lib 那条测试要
`tracing-subscriber`，得再给它配 dev-dependency。**我倾向 (a)**：M2 的目标是零改写，
这个代价等到 M5 接线时按真实体积再定。

**② 两个 asset JSON 的过渡期定位。**
`machine_dimensions.json`（3 651 B）与 `machine_catalog_extra.json`（1 990 B）
是 `include_str!` 的编译期输入，**M2 阶段必须一起搬，否则编译不过**；
而 M3（原 Task 14）的定案是「不搬，改从 `presets/` 读」。所以它们在 M2/M3 之间
是一段**短暂的双真相**。建议：搬过来时在文件头上加一行注释写明「这是过渡件，
M3 删除，来源已确认为 `presets/machines/*.toml` 的等值快照」——
`MIGRATION-PLAN.md:71-72` 已经给出等值证据（125/125 字段、别名 23 条、禁区逐点相同）。

**③ 9 份 `tests/fixtures/presets/*.toml` 与 9 份 `fixtures/ir/build9/*.json` 也是产物。**
前者是那边 `gen-presets` 生成的预设产物、后者是 `ir::build()` 的产物。
`pipeline_wiping_source.rs` 现在读它们，所以 M2 必须一起搬；但它们与
`presets/` 是同一件事的两种快照，M5 接线后应当改由我们生成。
建议与 ② 同样处理：搬 + 头注 + 记进 M5 的收尾清单。

### 3.4 不该迁

| 对象 | 理由 |
|---|---|
| `crates/preset`（13 + 8 文件、7 482 行） | **不在 Task 13 范围**，那是 M4。它依赖 `core`，顺序必须它在后 |
| `mkp-ssr` 的 Tauri 应用层（`src-tauri/`、`crates/` 其余、`dist/`、前端 `src/`） | MIGRATION-PLAN §2：「不迁，我们已经有自己的」 |
| `crates/core/tests/reference/detail/go_detail_reference.json`、`reference/printtime/input.meta.json`、`reference/README.md` | **孤儿**：没有测试读它们。建议**仍然搬**——它们是基线证据，删了将来没法复核；但登记为「只作参考，不参与判据」 |

---

## 4. 问题四：怎么切批

**切法不是"按文件切"，是"按不变量切"。** 54 个源文件互相依赖，横着切必然编译不过；
能独立验证的维度只有「一次只动一个不变量」：

| 批 | 动的不变量 | 内容 | 验收 | 提交粒度 |
|---|---|---|---|---|
| **B1** | 目录布局 | 建 workspace 根 `Cargo.toml`，`src-tauri` 变成员；**不碰任何 Rust 代码**；CI 两处改到根 | 我们原有 **264**（`--features workbench`）/ 18（默认）仍绿；`cargo tree -d` 空 | 1 笔 |
| **B2** | **搬运不变量** | 整包复制 `crates/core` → `crates/postprocess`，**连包名 `mkp-pp`、lib `mkp_pp` 都先不改** | 「搬运没搬错」：逐文件 sha256 与源一致（74 个 `.rs` + 47 个数据文件）；`cargo test -p mkp-pp` 全绿（条数以真实输出为准） | 1 笔 |
| **B3** | 命名 | `mkp_pp::` → `postprocess::`（64 行 / 19 文件）；包名 / lib 名；bin → `mkpse-pp`；3 处 `CARGO_BIN_EXE_*` | `rg mkp_pp` 零命中；改名前后**逐文件 sha256 + 只允许出现机械替换**；测试仍全绿 | 1 笔 |
| **B4** | —（不做） | M3：尺寸 / 别名 / 禁区改从 `presets/` 读，删两个 asset | 见 MIGRATION-PLAN §5 | 下一段 |

**B2 与 B3 必须分开，这是这份盘点的主要建议。** 理由来自源 crate 自己的注释
（`crates/core/Cargo.toml:1-14`）：那边**刻意**保持包名不变，就是为了让
「搬家没搬错」这条判据成立在**内容零 diff** 上；改名会把机械改动与搬运错误
混进同一条 diff，谁也说不清哪一处不同是故意的 —— 而且它会动 19 个文件（含 3 个
测试文件），那 20 个测试文件"全绿"就不能再当搬运证据了。

分成两笔之后：B2 拿零 diff 当证据，B3 拿「机械替换」当证据，两者都不需要
那边那套 K1..K7 前置条件。**这不是过渡别名** —— B3 一笔改完，仓库里从没有过
`pub use mkp_pp as postprocess` 这种东西，Task 13.3 的要求仍然满足。

---

## 5. 问题五：每批的验收判据与假红清单

### 5.1 会假红的单点（这一节比上面那些都重要）

1. **`serde_json` 的 `float_roundtrip`。** 源仓根 `Cargo.toml` 的注释写着「漏了会得到
   一堆莫名的 golden 假红」。受害点是 `tests/reference_printtime.rs` 的**逐位浮点比较**
   （`value.to_bits()` 比 5 项时间指标；默认 feature 下 `11637377359379175 f64 / 1e13`
   差 1 ULP）。统一到 workspace 根时这一条最容易被漏 —— 它是**功能开关**，不是优化项。
2. **`preserve_order`**：`dump-ir` 的键序依赖它。
3. **golden 是零容差逐行比较**（`golden_common::compare_line_wise`），比对在**文本层**，
   且 `split_lines` 是 **LF-only**。搬运动作要保证行尾与编码没被 Windows 工具改写。
4. **`UPDATE_GOLDEN=1` 会回写 `tests/golden/`**。迁移期间**不要设这个环境变量** ——
   一设就把"判据"变成"把现状抄成期望"。
5. **`boundaries.rs` 的哨兵是下限（`files.len() >= 30`），不是精确值。**
   源码注释里写的「实测 70 个文件」只是记录，断言不会因为文件数变化而红。
   但**若只搬一部分文件**，它可能低于 30 而报「判据不成立」—— 那是设计意图，别去调它。
6. **`#[test]` 数量**：源码 146 + 测试 87 = 233 处，实际跑约 248 条（推算）。
   落地时把真实数字记进来；`MIGRATION-PLAN` 与 Task 13.5 里的「20 个测试文件全绿」
   指的是**这 19 个二进制 + 1 个共享模块**，不是条数。

### 5.2 B1 的判据

- `cargo test --features workbench` = 264、`cargo test` = 18（**Task 13.5 里写的 274
  是 Task 12 之前的数字，应改成 264**）
- `cargo clippy --all-targets -- -D warnings` 两种 feature 组合零警告
- `cargo tree -d` 在 workspace 根输出为空
- `npm run lint` / `npm run build` 不受影响（前端不碰 Rust）

### 5.3 B2 / B3 的判据

- B2：源与目标的 sha256 清单逐行相同（`Get-FileHash`，74 个 `.rs` + 47 个数据文件）；
  `cargo test -p mkp-pp` 与源仓同样的条数通过
- B3：`rg "mkp_pp"` 零命中；`rg "CARGO_BIN_EXE_mkp-pp"` 零命中；测试条数与 B2 相同；
  逐文件 diff 只允许出现机械替换（可用「替换前后 sha256 + 只允许 `mkp_pp`→`postprocess`
  这一种改动」的脚本化断言）
- 两批都不许动 `presets/`：12 个文件的 sha256 与迁移前一致（`MIGRATION-PLAN.md:124`）

### 5.4 关于源树的脏状态（提醒，不是判据）

`mkp-ssr` 工作区有 7 处未提交改动：`docs/HONEST-BOUNDARIES.md`、`make.bat`、
`scripts/` 三份、`src-tauri/src/content.rs`，外加未跟踪的 `.comate/specs/bake-station/`。
**`crates/core` 干净** —— 所以这次迁移的输入是确定的。按红线：只读，不碰。
另外有 4 处 macOS 老路径，全在注释里（`crates/core/Cargo.toml:1`、
`tests/end_to_end.rs:10,16`、`tests/reference_calibration.rs:128`），搬过来之后
按 Task 19 的措辞清场一起处理，不急在这一批。

---

## 6. 定案与待决策（2026-09-23）

| # | 事项 | 状态 | 定案 |
|---|---|---|---|
| 1 | **B2/B3 拆两笔**（先原样搬运拿零 diff 证据，再单独改名） | **已定** | 照 §4 执行，两笔提交 |
| 2 | **bin 的三个依赖**（`clap` / `ctrlc` / `tracing-subscriber`） | **已定** | 先照搬现状，M5 接线时再定是否调整。**不把依赖清理混进迁移** |
| 3 | **两个 asset JSON** 与 9 份测试用预设（§3.3 ②③） | **已定** | 搬迁 + 文件头注 + 记入 M5 收尾清单。**不在 Task 13 顺手清理** |
| 4 | **写盘纪律的豁免**（§2.5 ①） | **待决策** | 三个候选方案见 §2.5 ①。判据是「能不能在新 crate 层面**持续**强制写盘边界」—— 不接受用"clippy.toml 管不到它"当默认结论，那等于永久漏检口。这一项**不阻塞**本盘点入库，单独比较后再定 |

决策 1 的意义值得写下来：它把 Task 13 从「搬 27 000 行代码」变成了
**可以逐项证明、逐步回退**的过程 —— 「搬运对不对」与「改名对不对」变成两条独立判据，
哪一条红了就把那一笔退掉，不必怀疑另一笔。

决策 2 与 3 是同一条纪律：**迁移批次只动一个不变量。** 依赖清理会影响 9 个第三方
的版本解与 `cargo tree -d` 的结论，混进 B2/B3 之后，"零 diff 搬运"这条证据就不成立了。

## 7. 这份盘点没做的事

- 没编译、没跑测试（源树只读）。B2 落地时必须先拿到「源侧 248 条全绿」的实测输出，
  才能把目标侧的数字对上。
- 没盘 `crates/preset`（M4）与 `gen-presets` 的接线（M5）—— 那两段各自要一份盘点。
- 没做任何数据比对：`MIGRATION-PLAN.md:70-72` 的等值结论来自 Task 11（M0）的实测，
  这里只引用，没有复核。
