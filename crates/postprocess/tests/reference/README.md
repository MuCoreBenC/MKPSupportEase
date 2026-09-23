# tests/reference 与 tests/fixtures —— 证据等级声明（Task 4.6）

**这三份产物与 `tests/golden/` 的证据等级不同，收口报告不得把差别抹平：**

| 产物 | 是什么 | 证据等级 |
|---|---|---|
| `tests/golden/`（G1/G2/G3） | 旧仓库被独立测试守过的**既有**字节快照 | 对照一份既有验收物 |
| `tests/fixtures/ir/golden_42274_2.json` | 本次在旧仓库用一次性工具新导出的 IR 投影 | **对照当前 Go 实现的输出** |
| `tests/reference/calibration/*` | 本次新导出的校准生成器输入/输出对 | 同上 |
| `tests/reference/printtime/*` | 本次新导出的 Estimate 输入/输出对 | 同上 |
| `tests/reference/detail/go_detail_reference.json` | 本次新导出的 `dto.ProcessingDetail` 投影（Task 6.1） | 同上 |

含义：**Go 侧若本来有 bug，这些参考会带着 bug 一起被逐字节复制进 Rust**。
它们是行为锚点，不是正确性背书。G1/G2/G3 至少经过旧仓库自己的 golden 测试
长跑守过；这三份没有这层保护。

## 导出方式（可复现，一次性）

在旧仓库 `mkpsupporte` 写过一个临时测试文件（`internal/processor/export_rust_fixtures_test.go`，
用后即删，旧仓库 git status 已确认 clean），以
`MKP_RUST_EXPORT_DIR=... go test ./internal/processor/ -run TestExportRustFixturesOnce -count=1`
运行。要点：

- IR = `newTestIR()` + `golden_test.go:38-46` 的 7 项覆写 —— 即 G1/G2 golden 的
  确切输入（`Ironing.Mode=off / UsePath=false / ESumThreshold=7.9 /
  NozzleDiameter=0.4 / MachineType="A1_MINI" / Filament="PETG"/"BambuStudio" /
  Wiping.FanSpeed=255`）。
- IR JSON 的键是 Go 结构体字段名（PascalCase，无 json tag）。全树唯一的
  序列化不兼容字段 `StateIR.SupportLayerZSet map[float64]bool`（pass1 运行态，
  导出时刻为 nil）以 `map[string]bool` 影子导出。
- 校准参考：`BBox`/`CentroidResult` 由 `ComputeBBox` / `ComputeExtrusionCentroid`
  对 `42274.2.gcode` 实算；IR 是 **`ir.FillDefaults` 之后**的快照（与真实链
  `helpers.go:162-174` 一致）；`zDwell=2 / xyDwell=3`（preferences 硬编码默认值，
  `service.go:171-172`，注释声明与 SSOT 一致）；`templateDir=""` ⇒ 走内嵌回退模板。
  四个模式各一对输入/输出：ZOffset(897 行) / Precise(235) / Rough(235) / Repetition(113)。
- printtime：`Estimate(input.gcode, {ComputeDelta:true, StartupOverheadSeconds:240})`，
  与旧侧 `process.go:1104` 的真实调用同参。`input.gcode` 导出当刻与
  `tests/golden/42274.2.pass2.expected.gcode` 逐字节相同（sha256 已 `cmp` 验证，
  记录在 `input.meta.json`）。**自 2026-09-12 起两者不再相同**：G2 基线因为 MKP 标记
  协议 v1 的发射而重生成，而这份是 printtime 的**输入** fixture，改它会连带改期望值，
  所以刻意保持原样。无 `_original` 兄弟文件 ⇒ `PostProcessDeltaSeconds`
  为 null，这是确定性事实不是缺失。

## New 变体校准参考的导出（2026-09-08 实测）

`tests/reference/calibration/{Precise,Rough,ZOffset}.new.*` 是 `generator_new.go`
两个 New 变体（BBox 动态定位）的输入/输出对，**与上面四份不是同一次导出**：

| 项 | 值 |
|---|---|
| 源仓库 | `/Users/wzy/projects/mkpse-workspace/mkpse-next_v3/mkpsupporte` |
| commit | `e8d2d9821a59fc2766d72dbc73d9da6bf2ec3fda`（2026-08-26） |
| 工具链 | go 1.26.4 darwin/arm64 |
| 导出方式 | 临时测试 `internal/disk/tmp_export_new_fixtures_test.go`，用后即删（已确认 git status 无残留） |
| 调用 | `GenerateCalibrationGCodeWithCentroid(mode, machineType, ir, &bbox, centroid, nil, zDwell, xyDwell)`，`templateDir` 不传 ⇒ 内嵌回退 |

输入 JSON 由本仓库的 `Precise/Rough/ZOffset.input.json` 派生：把
`IR.Safety.XYCalibration`（或 `ZCalibration`）改成 `"new"`，再补上 BBox 与质心。

BBox / 质心的来源是**真机样例实算**再**刻意平移**：

| 参考 | 真机样例 | 实算 BBox（宽×高） | 平移 | 参考里的 BBox |
|---|---|---|---|---|
| `Precise.new` / `Rough.new` | `XY_PLA_6m48s.gcode` | 50.5 × 50.5 | (+20, −10) | x[86.023, 136.523] y[56.34, 106.84] |
| `ZOffset.new` | `ZOffset Calibration_PLA_9m11s.gcode` | 119.58 × 13.063 | (+5, +8) | x[35.131, 154.711] y[92.618, 105.681] |

**为什么要平移**：两份真机样例恰好摆在机型默认校准位（A1_MINI 的
`yLineX=66.523 = bbox.x_min + 0.5`、`zStartX=30.21 = bbox.x_min + 0.079`），
不平移的话 New 分支算出来的坐标与经典分支完全重合，参考就验不出「原点跟着模型走」
这件事。质心按同一位移平移，所以形变/旋转检测仍然放过。

行数与 sha256：

```
257  78cea51254567b058f77f0aff7b6cf1b44dbb28492a1131f9fa5047d6bb117cf  Precise.new.expected.gcode
257  8625e9e3ebd2da201d50e5c0b930a6907c9fceaf7ef7764b1d6412a5af420b5e  Rough.new.expected.gcode
897  3e8f28dff0b93dfdae9e834112a0f621a0373ad6e47606c6eb1fbc382c577990  ZOffset.new.expected.gcode
```

XY 的 257 = 经典 235 + 22 条 `G4` 停留（New 每轮都停，经典分支根本不消费 dwell）；
Z 的 897 与经典同行数（Z 经典本来就有停留），差别只在坐标原点。

证据等级与上面四份相同：**对照当前 Go 实现的输出**，不是正确性背书。

## detail 参考的导出（Task 6.1，2026-08-29 实测）

`tests/reference/detail/go_detail_reference.json`（2,776 B，sha256
`a50a6cd7d39ad78b46b5793bf45052ee102e17de65ca9f2c9d77c587255017e0`）= 旧仓库
`engine.Process` 真跑一遍后 `RunContext.LastDetail` 的 JSON 投影，输入是本仓库自己的
`tests/golden/42274.2.gcode`（sha256 记在文件里）+ `tests/fixtures/presets/A1.toml`，
`goVersion=go1.26.4`。导出工具同样是 `//go:build ignore_export_detail` 的一次性测试，
**已删除**（旧仓库 `git status` 已复核 clean）。

**跑通它需要三处依赖注入，缺任何一处都不是「报错」而是「跑出错的东西」**——这三条
是本次最贵的产出，将来重导必须照做：

1. `logger.SetDefaultLogDir` + `logger.InitLogger`：不注入则 `GetProcessorLogger()`
   nil deref。
2. `config.SetContentProvider`：机型维度（`MovementRange` / `GlueArea`）来自
   `machine_catalog.json` 的 `dimensions`，provider 缺席时 `GetMachineDimensions`
   静默返回 zero-value ⇒ 首次扫描必报 `E_GCODE_BOUNDARY_001`「允许范围 ≤0.0mm」。
3. `machine.SetDefaultCatalog`（**最隐蔽的一条**）：`DefaultCatalog()` 在无人注入时
   **懒建一个空 catalog**（别名表是空 map），于是
   `NormalizeToCanonical("A1")` 返回 `""`、**不报错**，`FillMachineDimsFromToml`
   撞自己的空串守卫直接 return ⇒ 维度仍是 zero-value ⇒ 报的还是上面那个边界错误。
   即**两个独立成因、同一个症状**，而症状指向的是受害者（边界校验）不是真因。
   判据必须是「注入后 `NormalizeToCanonical("A1")` 非空」+「该 canonical 查到的
   `MovementRange.MaxX != 0`」两次真实查询，不是「我调了那个 setter」。

诊断方式登记：`captureReporter` 实现了 `Pipeline` 通道并在失败时逐条 dump —— 「机型识别」
那一步的 Detail 文本（`G-code 机型 / 预设机型 / 机型维度 / 禁区数量`）是唯一能从包外
看到维度填充结果的出口，正是它把真因从边界错误里挖出来的。另记：碰撞检测在维度全零
时照样报「无碰撞、模型坐标均在打印范围内」⇒ **「前面某步过了」不构成维度已填的证据**。

### 这份参考覆盖多少 / 哪些字段不可比

- `dto.ProcessingDetail` 声明 **80** 个 json 键，本次运行时实际出现 **76** 个。
  差的 4 个是 `fallbackObjects` / `warnings` / `warningDiagnostics` /
  `towerLayerHeightWarnings` —— 全是 `omitempty` 且本输入下为零值，**不是导出漏了**。
  故本参考能钉住的是 76 个键；另外 4 个键需要能产出警告/回退对象的输入才有锚点，
  当前**无锚点**，如实登记。
- **环境相关、不可逐字比对**：`gcodePath`（导出时指向 `t.TempDir()` 里的拷贝）、
  `duration`（挂钟耗时）。比对 harness 必须把这两个键列进「不可比」而不是「相等」。
- `machineType` 实测为 `"UNKNOWN"`（`DetectFromGcode` 在这份 G-code 上无命中），
  而 `presetMachineType` 为 `"A1"`。这是**这份输入的事实**，Rust 侧若给出别的值
  就是差异、不是修正。

### 起点红（Task 6.2/6.4，2026-08-29 实测）

harness = `crates/app/tests/detail_projection.rs`，跑法
`cargo test -p mkp-app --test detail_projection -- --test-threads=1 --nocapture`。
两次复跑数字相同：

| 表 | 数量 |
|---|---|
| 相等 | **1**（`towerHeight`） |
| 不等 | **3**（`estimatedPrintSeconds` / `gcodeSizeMB` / `outputSizeMB`） |
| Rust 缺键 | **70** |
| Rust 多键（本参考无锚点） | 1（`warnings`，即上面那 4 个 `omitempty` 之一） |
| 不可比（已排除） | 2（`gcodePath` / `duration`） |

可比字段合计 74 = 1 + 3 + 70，加上 2 个不可比 = 参考的 76 键。**红因是可数事实**
（缺键 70 项），不是崩溃 —— 这正是 Task 8 的靶子。
（`tasks.md` 6.4 里写的「缺键 72 项」是按旧的 79/80 口径预估的，实测为 70，
不改判据去凑那个数字。）

已定位的两条不等根因（Task 8.4 消化时直接用，不要重新猜）：

- `gcodeSizeMB` / `outputSizeMB` —— **MB 的定义不同**。输入实测 561,363 B，
  Go 给 `0.561363` ⇒ Go 除的是 **10⁶**；Rust `file_size_mb()` 除的是 1 MiB
  = 1,048,576（0.5353574752807617 × 1048576 = 561,363 B，输入本身两侧一致）。
  这是纯单位口径差，不是文件不同。
- `estimatedPrintSeconds`（Go 1472.3859628955183 / Rust 1478.3211598693833）与
  `outputSizeMB` 换算回字节后的 **668,488 B（Go） vs 668,672 B（Rust）**：
  输出内容确有 **184 B** 差异，printtime 的差异与它同源。
  注意它与既有 A 档判据不冲突：G1/G2 与 Task 16.4 的塔模式交叉验证跑的是
  另一组入口/预设组合，本条是「Go GUI 全链 detail 导出」这条新路径上的首次比对。

  **Task 8.4e 勘误（这条已定位，且上面那两个 Rust 侧数字是旧的）**：
  ① Rust 侧实测为 **668,687 B（差 199 B）**——上面的 668,672 / 184 B 是 Task 6 那次
  记录，此后 Rust 侧有过改动，**不修改上面的时点记录，只在此处给出当前实测**；
  ② 根因：Go 一次性导出工具没有 gcodesvc ⇒ `EngineConfig.Hooks` 落 `NoOpHooks`，
  参考输出既没有 begin-stage-retract 块、也没滤掉 M970*/M974；而本链按 Go 自己的
  preferences 默认值（`preferences/service.go:165-180`）**真的执行**这两个 hook。
  决定性实测：临时把两个 hook 默认值置 false ⇒ 输出 **668,488 B** 与 Go 逐字节相同、
  printtime 同时逐位等于 Go 的 1472.3859628955183。
  ⇒ 这两个键是**参考侧 artifact**（同 `machineType` 一族），已在
  `crates/app/tests/detail_projection.rs` 的 `REFERENCE_ARTIFACT` 登记、理由写在
  `docs/HONEST-BOUNDARIES.md` §17.2。**刻意不关 hook 让 harness 变绿。**

  **同批第三条不等的勘误（根因不在算法而在本 README 描述的比对层）**：
  `glueTotalLength` 曾报差 1 ULP，真因是 **serde_json 默认 feature 的浮点解析**在
  17 位有效数字上差 1 ULP ⇒ 参考文件里写的 `1163.7377359379175` 被读成 `…177`。
  根 `Cargo.toml` 已开 `float_roundtrip`。**Task 10 记的「printtime TotalSeconds 与
  Go 差 1 ULP」是同一条缺陷**，修掉后 `reference_printtime` 5 项全部逐位相等，
  该测试的容差分支已删（证据等级上调，arm64 + x86_64 双目标实测）。

**Task 8 收口后的当前实测**（上面那张表是 Task 6 的起点红，保留为时点记录）：
【相等 70 / 不等 0 / Rust 缺键 1（`fallbacks`，有意未移植的子系统）/ Rust 多键 0】。

## 消费方

- `tests/fixtures/ir/golden_42274_2.json` → Task 7.2 的反序列化闸门（字段缺一个
  即失败）+ G1/G2 runner 的 IR 输入。
- `tests/reference/calibration/` → Task 9.5 逐字节比对。
- `tests/reference/printtime/` → Task 10.3 数值比对。
- `tests/reference/detail/` → Task 6.2 逐字段树比对（相等 / 不等 / Rust 缺键三张表）
  与 6.3 键名 × `models.ts` 词表断言。

**CI 不调用 Go**（design.md §九.5）：判据只消费这些静态文件。与真实 Go 二进制
的交叉验证是本机手动步骤（Task 16.4），结论写进收口报告。
