# 开发态正常、发布态出错：2026-11-23 一次「仓库兜底」事故复盘

> 起因：用户在同一台 Windows 电脑上，开发/测试时后处理正常，装了发布物（绿色版 / 安装版）后连续踩到两个错：
> ① Bambu 弹 `Error code: 101`；② 二合一校准件被判「不是校准件」、按普通模型跑。
> **「测试没问题、发布出去就有问题」的判断是对的，而且不需要换电脑** ——
> 同一台机器上「从仓库跑」与「跑发布物」就是两个不同的世界。本文记录根因、修复与防复发的测试。
> 本轮代码改动在 `fix/source-switch-refresh` 工作区，**未提交**。

---

## 1. 一句话结论

**开发态有一条发布态没有的兜底：`machine_dims::tables()` 会去仓库的 `presets/` 目录现读机型表。**
仓库在，测试永远绿；发布物没有这个目录，就两次翻车：

| | 开发态（`cargo test` / `tauri dev`，仓库根） | 发布态（装机版 / 绿色版，用户机器） |
| --- | --- | --- |
| `presets/` 目录 | 一直在盘上 | **不存在**（打包只带 catalog + 资产，不带源目录） |
| 机型表来源 | 逃生链第三档 `CARGO_MANIFEST_DIR/../../presets` 兜住 | 没人装表 ⇒ 表是空的 / panic |
| 结果 | 测试全绿 | 101 panic；二合一认不出档位 |

这不是「Mac 和 Windows 代码不一样」——两个平台的这份代码**一字不差**。是**同一台电脑上，有没有仓库目录**的差别。
用户说「Mac 上没问题」也一样解释：Mac 上是从仓库跑，兜底生效。

---

## 2. 问题一：`Error code: 101`

### 现象
绿色版在 Bambu 里跑后处理，弹窗只有一句 `Error code: 101`，没有原因。`hook.log` 只有两条
「后处理开始」，没有结论行。

### 定性（不是猜）
程序自己的退出码契约只有 0/1/2（`hook.rs:75-80`），**101 是 Rust panic 的默认退出码**。
Windows 上 Bambu 不接 stderr（`hook.rs:41-45`），所以用户只看到数字。

### 拿到的真实 panic 文本
用用户桌面那份绿色版 exe，在隔离数据根下重跑同一份 gcode + 预设，设 `RUST_BACKTRACE=1`：

```text
thread 'main' panicked at crates/postprocess/src/postproc/machine_dims.rs:251:9:
没有机型表。发布物必须在启动时调 machine_dims::install(..)；
 开发与测试可以设 MKPSE_PRESETS_DIR 指向 presets/ 目录
EXIT_CODE=101
```

### 根因链
```
main.rs（钩子分流）
  └─ hook::run() → preset::load_ir / 校准定案 / 管线
       └─ machine_dims::tables()        ← 进程级 OnceLock
            ① 显式 install()            ← 全仓没有任何一处调用（只有注释提到）
            ② MKPSE_PRESETS_DIR         ← 发布物没有这个变量
            ③ CARGO_MANIFEST_DIR/../../presets  ← 开发态永远成立，发布态永远不成立
            三条都不成立 ⇒ panic 101
```
`src-tauri` 全仓搜 `machine_dims::install`：**只有注释，没有实际调用**。这条欠账（仓库注释里写「M5 要接」）一直没接，被第③档掩盖了整个开发期。

### 修复
`main.rs` 一进门就装表（钩子与界面两条路都覆盖）：
- 机型尺寸 / 别名 / 禁区 ← 随包 `catalog.generated.json`（编译期嵌入、与仓库无关的既有真值）
- 校准档表 ← `include_str!("../../../presets/calibration_models.toml")`（编译期钉入）
- `machine_dims.rs` 加纯构造器 `MachineTables::from_catalog_parts`

**不是**去造一个 `presets/` 文件夹——那会把源真源复制成分发物，制造双真相。

### 验证
同一份输入（用户桌面绿色版 + 真实 gcode + 预设）：
- 修复前：`EXIT_CODE=101`，hook.log 只有开始行
- 修复后：`处理完成 … ms=158`，落完整 `_meta.json`

---

## 3. 问题二：二合一被判「不是校准件」（修复过程中我自己引入的回归）

### 现象
二合一（Z + XY 合体）切片后，程序报 `E_CALIB_UNRECOGNIZED_001`「不是校准件」、按普通模型跑完，预览里自然看不到校准图案。

### 定性
用户给的 `_meta.json` 写得很清楚：
```
modelName = Calibration_Combined
verdict = from_3mf          ← 身份读到了
identity3mf = Combined      ← 身份值正确
exists = true               ← 3MF 在盘上
error = 不是校准件 …
```
**身份完全读到，是定档那一步失败的。**

### 根因（我引入）
修问题一时，我装的机型表里 `calibration_models` 传了 `None`。而二合一（Combined）按设计**必须靠「实测外形 + 校准档表」定档**：
档表为空 ⇒ `find_profile` 永远不命中 ⇒ `decide_calibration(Combined, size, [])` 只能判 `Unrecognized` ⇒ 报「不是校准件」。

Mac 上同样是这套代码却不犯：Mac 从仓库跑，第③档逃生链把 `presets/calibration_models.toml` 读到了。又是「仓库兜底」。

### 修复
- `calib_models.rs` 加 `parse(raw, where)`，`load(path)` 复用它
- `runtime/mod.rs` 把 `presets/calibration_models.toml` 编进二进制，装表时一起装入
- 回归判据钉住「每个带外形的档按自己的标称尺寸都能被认出」

### 验证（用用户那次的真实现场）
拿 `15120.0_20261123_193559` 那份原件 + 真实的 `origin.txt`（指向存在的 `Calibration_Combined.3mf`）：
- 修复前：`E_CALIB_UNRECOGNIZED_001`、退出码 2、「不是校准件」
- 修复后：弹出 `calibration-mode` 那一问（"先打印再校准 / 直接校准"）⇒ **识别成校准件了**
  命令行没人答，110 秒后按停止收场（`W_CALIB_MODE_001`、退出码 1、原文件不动）——正确行为

### 关于「识别不出要不要兜底」
用户一开始说「不要兜底、识别不出就报错」，随后自己又觉得不对。定案口径（也是现在代码的口径）：
- **没读到身份** ⇒ 普通模型（不是校准件，事实如此，不猜）
- **读到了 Combined 身份但认不出档位** ⇒ 硬错（不许静默按普通模型跑）
这正是用户最终要的行为。

---

## 4. 防复发：完整测试

### 结构性盲区
CI 的 Rust job 都在仓库根跑，`presets/` 永远在 ⇒ 第③档逃生链永远成立 ⇒ 上面两类坑**在 CI 里结构性不可见**。

### 加了两层

**① CI 自动判据（进 `cargo test`，CI 自动跑）**
在 `runtime/mod.rs`：
- `catalog_machine_tables_carry_dimensions_aliases_and_zones`：随包表数据完整（尺寸/别名/禁区/档表）
- `bundled_calibration_profiles_are_recognisable_by_size`：随包档表里每个带外形的档，按标称尺寸必须能被认档函数命中

**② 本地端到端探针（手动，`npm run probe:release-hook`）**
用隔离数据根 + 清 `MKPSE_PRESETS_DIR` 模拟发布物。**先检测界面端点，没起就退出 2、零副作用**——不拉窗、不弹空的
`[::1] 拒绝连接` 页面（那是 debug 界面在连没启动的 dev server，不是产品问题）。按用户定的方式：先起真界面再跑：

```bash
# 终端 A：真界面，数据根与探针一致（脚本会打印具体路径）
MKP_SUPPORT_EASE_DATA="<脚本打印的隔离根>" npm run tauri dev
# 终端 B：
npm run probe:release-hook
```
真窗弹校准问句，点哪颗由人决定。三档：普通 / ZOffset / 二合一。

**夹具**：`crates/postprocess/tests/fixtures/dev/calib-combined.gcode`（用户那次真机切片产物，已入仓并写进 README）。

### 验证数字
runtime 5/5、校准 12+2、`cargo fmt --check`、`cargo clippy -D warnings` 全绿；
探针在无界面时退出 2、不留进程。

---

## 5. 用户担心「别人电脑上会不会更多问题」——盘点结果

全仓排查按三类分。前两类是这次同族风险，第三类是已知的跨平台差异。

### A. 仓库数据 vs 随包数据（与本次同源，事故密度最高）
- **残留最高风险**：`machine_dims.rs:258-273` 的第③档逃生链还在。将来任何**新二进制 / 新入口**绕过 `main.rs`
  直接调 `tables()`，101 会原样重现——现在只有一行注释和 CI 判据挡着，判据测的是数据完整、不是入口接线。
- 同族历史事故：`PRINT-FAILURE-2026-11-04` —— 本机预设 `offset_z=3.95`、仓库副本 `3.9`，按仓库数据归因会得出错误结论。
  教训：**归因必须看用户盘上的原件，不能看仓库副本**。

### B. 绿色版 vs 安装版（只有 Windows 有两种交付）
- 数据根分裂：`exe旁/app-data` vs `%APPDATA%`。曾导致界面把端点写在绿色版根、钩子去读安装版旧端口 ⇒ 校准问句永远问不到人（Bambu 只显示 `Error code: 1`）。判据在 `fsx/paths.rs:375-434`。
- WebView2：绿色版没有安装包附带的 bootstrapper，缺运行时会在建窗时静默失败；`main.rs` 有前置提示。
- 自更新落点：绿色版只换自己目录里的 exe，不能落进安装目录（2026-11-20 真机踩过）。

### C. Windows vs Mac（代码分支不同，Mac 开发机不编译 Windows 侧）
- `\\?\` 长路径绕 `MAX_PATH=260`（Mac 不可能复现）
- 切片器跟随：macOS 走 AX、Windows 走 UIA / EnumWindows（class `Static` 判据）
- `#[cfg(windows)]` 专属代码：历史上 `ax.rs` 的签名错误只在打 Windows 包时暴露；CI 有 `rust-windows` job 覆盖编译
- 通知、控制台隐藏、大小写改名判同文件等

**判断**：别人电脑上「更多问题」最可能出在 A 类和 B 类——都是**发布形态/数据位置**问题，不是业务逻辑。
C 类有 CI 的 `rust-windows` 编译保险。真要进一步降风险，下一步该做的是：把 CI 的 Windows job 加一步
「把构建产物复制到无 `presets/` 的目录后跑钩子入口」，让第③档逃生链失效的场景在 CI 里也能出现。

---

## 6. 本轮改动清单（工作区，未提交）

| 文件 | 做什么 |
| --- | --- |
| `src-tauri/src/main.rs` | 入口装机型表（钩子 + 界面） |
| `src-tauri/src/runtime/mod.rs` | 从随包 catalog + 编入的校准档表构表 / 装表；两条新判据 |
| `crates/postprocess/src/postproc/machine_dims.rs` | `MachineTables::from_catalog_parts` |
| `crates/postprocess/src/postproc/calib_models.rs` | `parse(raw, where)`，`load` 复用 |
| `crates/postprocess/src/pipeline/bed_glue.rs`、`postproc/calibration.rs` | 去掉引用 Go 实现的注释（用户要求） |
| `crates/postprocess/tests/fixtures/dev/calib-combined.gcode` + `README.md` | 二合一真机夹具 |
| `scripts/check/release-hook-probe.mjs`、`package.json` | 本地端到端探针（`probe:release-hook`） |

未提交、未推送、未开 PR。全仓其余 77 个文件的 Go 注释清理**未做**（单独任务，避免与 bug 修复混在一笔）。
