# tests/reference 与 tests/fixtures —— 参考资产说明

本目录的输入/输出对是**本项目自己的人工确认基准**。

它们回答的是「行为有没有变」，不是「行为对不对」。工艺正确性由业务规则、真机验收与
人工审阅承担 —— 参考文件里多一行、少一行，只有人看过并说「这是我想要的」，它才成为基准。

据此，全目录只有一条更新规则：

> **先看差异，确认是想要的，再改期望值。** 原因写进提交信息。
> 不挂在生成、发布、同步任何自动路径上。

判据只消费这些静态文件，不联网、不调用外部程序。

## SupportEase 自有整链 golden

`crates/postprocess/tests/golden/42274.2.e2e-A1.support-ease.gcode` 是**本项目自己的**参考输出，
替代「整链与外部实现逐字节相同」那条判据。理由：那份外部参考物在本仓无法复现，
等于把**当时的参数值**一起钉死 —— 参数源一改就红，只能去外部重出。

| 项 | 规定 |
|---|---|
| 输入 | `tests/fixtures/presets-frozen/A1-standard.frozen.toml`（**冻结件**，属测试资产） |
| 谁维护 | 跟着代码走；**不跟随** `presets/` 参数更新 |
| `--sync-baseline` | **不碰**它（那只同步 `tests/fixtures/presets/` 的数据基线） |
| 参数更新时 | 一个字都不动 —— 输入冻结 ⇒ 输出不变 ⇒ 判据不红 |
| 映射/管线改了 | 会红，这正是它的用途：提醒有人检查差异 |
| 唯一更新入口 | `MKPSE_RECORD_PRESET_GOLDEN=1 cargo test -p mkpse-preset --test load_ir preset_chain_matches_the_support_ease_golden` |
| 更新前必须 | 人工确认差异**是想要的**，并把原因写进提交信息 |
| 证据等级 | **自比自**：证明「没变」，不证明「对」 |

配套的三条数据无关判据（映射逐字段 / 参数行为差分 / 确定性与结构）见
`crates/preset/tests/load_ir.rs` 的文件头表格；它们才是「参数确实生效」的那一层。

## 冻结 config 路径的整链参考

`crates/postprocess/tests/{end_to_end,process_with_ir}.rs` 与
`tests/golden/42274.2.e2e-A1.config.gcode` **纯逐字节**比对，不做任何剔除。
它们喂的是 `tests/fixtures/config/A1.toml`（IR TOML，**派生自 IR JSON，不读预设文件**），
所以预设参数更新不会碰它们 —— 它们守的是「编排与后处理字节没变」。

与预设链的 `42274.2.e2e-A1.support-ease.gcode` 分工不重叠：那份走「冻结预设 → IR」，
这份走「IR TOML 配置」。两条入口都要有自己的字节锁。

| 项 | 值 |
|---|---|
| 基线 | `tests/golden/42274.2.e2e-A1.config.gcode`（672,282 B，27,188 行） |
| 输入 | `tests/golden/42274.2.gcode` + `tests/fixtures/config/A1.toml` 原样 |
| 判据 | 逐字节相等；两条反空转（基线不是输入的副本、改 `Glue.PassCount` 必须变） |
| 再生入口 | `cargo test -p mkpse-postprocess --test end_to_end -- --ignored regenerate_the_chain_baseline` |
| 更新前必须 | 逐行看 diff、说清为什么变；原因写进提交信息 |

`A1.toml` 是派生件，不是手写配置。派生关系有判据盯着：

| 项 | 值 |
|---|---|
| 派生自 | `tests/fixtures/ir/build9/A1-standard.json`，经 `config::to_toml` |
| 唯一改动 | `Machine.MachineType = "A1"`（不填则这份配置跑不起来） |
| 判据 | `pipeline::fixture_is_derived_from_the_ir_json` 比**结构相等**，不是文本相等 |
| 更新入口 | `cargo test --test pipeline -- --ignored regenerate_config_fixture_from_ir_json` |
| 注意 | 它会把当前默认值显式写进 TOML。默认值之后再变，fixture 不受影响 |

## 校准参考（`calibration/`）

七个模式各一对输入/输出。输入是自包含的 IR 快照 + BBox + 质心 + dwell 秒数，
消费者是 `crates/postprocess/tests/reference_calibration.rs`，逐字节比对。

| 模式 | 行数 | 文件 |
|---|---|---|
| ZOffset | 900 | `ZOffset.{input.json,expected.gcode}` |
| Precise | 238 | `Precise.…` |
| Rough | 238 | `Rough.…` |
| Repetition | 116 | `Repetition.…` |
| ZOffset.new | 900 | `ZOffset.new.…` |
| Precise.new | 260 | `Precise.new.…` |
| Rough.new | 260 | `Rough.new.…` |

**没有再生开关**：改期望值就是人工生成一份新输出、逐行看过、替换文件。
测试不会自己覆写判据。

### 校准块的开头五行（当前形态）

```
1  ; ZOffset Calibration
2  G1 Z10 F1200
3  ; FEATURE: Outer wall
4  G1 X100 Y100 Z10 E0.001 F3000 ; MKPSE: virtual extrusion …
5  ; FEATURE: Custom
```

第 3、5 行的作用是**换角色**：切片器建路径包围盒时排除 `erCustom`，而校准块整段活在
start gcode 的 `; FEATURE: Custom` 之下 —— 不换角色，第 4 行那条虚拟挤出等于没写，
`direct_calibrate`（模型路径被删光）那一支第二盘起会被判「路径在床外」。
探针之后立刻换回来，校准图案的角色保持原样。

第 2 行单独抬 Z 同样不可省：第 4 行把 XY 与 Z 写在同一句里，打印机会线性插值，
于是「从当前 Z 平移 200mm」这一路是贴着板面拖过去的。先抬到目标 Z 再平移，
坐标不变、过程不同。

### New 变体为什么平移 BBox

`{Precise,Rough,ZOffset}.new.*` 是 BBox 动态定位那两支的输入/输出对。输入由对应经典
模式的 `input.json` 派生：把 `Safety.XYCalibration`（或 `ZCalibration`）改成 `"new"`，
再补上**平移过的** BBox 与质心。

| 参考 | 实算 BBox（宽×高） | 平移 | 参考里的 BBox |
|---|---|---|---|
| `Precise.new` / `Rough.new` | 50.5 × 50.5 | (+20, −10) | x[86.023, 136.523] y[56.34, 106.84] |
| `ZOffset.new` | 119.58 × 13.063 | (+5, +8) | x[35.131, 154.711] y[92.618, 105.681] |

**为什么必须平移**：真机样例恰好摆在机型默认校准位（A1_MINI 的
`yLineX=66.523 = bbox.x_min + 0.5`、`zStartX=30.21 = bbox.x_min + 0.079`）。
不平移的话 New 分支算出来的坐标与经典分支完全重合，参考就验不出「原点跟着模型走」。
质心按同一位移平移，所以形变/旋转检测仍然放过。

行数关系：XY 的 260 = 经典 238 + 22 条 `G4` 停留（New 每轮都停，经典分支不消费 dwell）；
Z 的 900 与经典同行数（Z 经典本来就有停留），差别只在坐标原点。

### `ZOffset.new` 的重切（2026-10-19）

`zoffset` 档的 datum 被对着打印件实调了（X：0.079 → −0.271，即 −0.35；
Y：3.755 → 3.255，即 −0.5 —— 见 `presets/calibration_models.toml` 那一档的注释：
打出来比对标定线偏右、整排偏上；X 试过 −0.1、−0.4，最后回调到 −0.35），
于是这份参考按新 datum 重切，其余一律不动。

重切时三步做完（X −0.1、X −0.3、再 X +0.05 / Y −0.5），每步逐行核对差异来源：
**170 行逐字节相同**（home / 回原点这类固定坐标）、**727 行只差 X/Y 平移**（图案相关），
**没有任何第三种差异**。也就是说它是「同一套结构 + 两个实调常数」，不是照着当刻输出重录。

现在这份是 **900 行**：在 897 的基础上多了校准块开头那 2 行角色切换（`; FEATURE:` 一对）。
若哪天再调 datum，这条参考要跟着重切，`z_new_matches_classic_up_to_the_datum_trim`
也会按新实调量重算差值。

## printtime 参考（`printtime/`）

`input.gcode` + `expected_result.json` + `input.meta.json`，消费者是
`crates/postprocess/tests/reference_printtime.rs`。数值比对，无再生开关。

`input.gcode` 导出当刻与 `tests/golden/42274.2.pass2.expected.gcode` 逐字节相同，
sha256 记在 `input.meta.json`。**两者自 2026-09-12 起不再相同**：G2 基线因为 MKP 标记
协议 v1 的发射而重生成，而这份是 printtime 的**输入** fixture —— 改它会连带改期望值，
所以刻意保持原样。

无 `_original` 兄弟文件 ⇒ `PostProcessDeltaSeconds` 为 `null`。这是确定性事实，不是缺失。

## detail 参考（`detail/`）

`go_detail_reference.json`（2,776 B）是 `dto.ProcessingDetail` 的一份 JSON 投影。

**当前没有判据消费它** —— 当初的比对 harness（`detail_projection.rs`）不在本仓。
它作为历史资产保留：里面记着 80 个 json 键声明、本次运行时实际出现 76 个，
差的 4 个是 `fallbackObjects` / `warnings` / `warningDiagnostics` /
`towerLayerHeightWarnings`，全是 `omitempty` 且该输入下为零值，不是导出漏了。

将来若恢复消费，有三条要照做：

- 不可比字段：`gcodePath`（导出时指向临时目录里的拷贝）、`duration`（挂钟耗时）。
  比对 harness 必须把它们列进「不可比」而不是「相等」。
- `machineType` 实测为 `"UNKNOWN"`、`presetMachineType` 为 `"A1"`。这是**这份输入的事实**；
  若给出别的值那是差异，不是修正。
- 文件名带 `go_` 前缀是历史命名，不代表任何对照关系。

## legacy_snapshot（`legacy_snapshot/`）

`machine_catalog_extra.json` + `machine_dimensions.json`：旧机型表形态的留影。
唯一用途是 `crates/postprocess/tests/presets_are_the_only_source.rs` ——
钉住「旧形态的数据源已经消失、不许回来」。它不是数据基线，不参与产物比对。

## 目录与消费者的对应

| 资产 | 消费者 |
|---|---|
| `tests/fixtures/ir/golden_42274_2.json` | G1/G2 runner 的 IR 输入 + 反序列化闸门（字段缺一个即失败） |
| `tests/fixtures/config/A1.toml` | `pipeline` / `end_to_end` / `process_with_ir` |
| `tests/reference/calibration/` | `reference_calibration.rs` 逐字节 |
| `tests/reference/printtime/` | `reference_printtime.rs` 数值 |
| `tests/reference/detail/` | 当前无消费者 |
| `tests/reference/legacy_snapshot/` | `presets_are_the_only_source.rs` |

## 证据等级

| 产物 | 是什么 | 证据等级 |
|---|---|---|
| `tests/golden/`（G1/G2/G3） | 既有验收物守过的字节快照 | 对照一份既有验收物 |
| `tests/fixtures/ir/golden_42274_2.json` | IR 投影，G1/G2 的确切输入 | 输入冻结，输出自比 |
| `tests/reference/calibration/` | 校准生成器输入/输出对 | 人工确认过的项目输出 |
| `tests/reference/printtime/` | 打印时间估算输入/输出对 | 同上 |
| `tests/reference/detail/` | `ProcessingDetail` 投影 | 历史资产，无判据 |
| `tests/fixtures/presets-frozen/*.frozen.toml` | 冻结预设 | 输入冻结，输出自比 |

**没有一份是「正确性背书」。** 它们全部只能证明「行为没变」。
工艺对不对，只能靠真机与人工验收 —— 这条不因为参考文件的数量而改变。

## 更新期望值时

1. 先出差异，逐行看完，把「为什么会变」说清楚（输入是什么、走哪条路径、改的是哪一步）。
2. 确认这是想要的业务行为，而不是顺手把现状抄成期望。
3. 改期望值，原因写进提交信息。
4. 只重跑受影响的判据，再跑全量确认没波及别的。
