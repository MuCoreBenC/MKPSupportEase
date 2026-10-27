# 校准模型档：把硬编码几何搬进 presets/calibration\_models.toml（兼容旧模型 + 新模型）

## Context（为什么做、要什么结果）

`crates/postprocess/src/postproc/disk/calibration.rs` 把校准模型的几何**写死在常量里**：
标称外形（ZOffset = 119.58 × 13.063、XY = 50.5 × 50.5）、图案基准偏移
（`Z_PATTERN_OFFSET_X/Y = 0.079 / 3.755`、`XY_PATTERN_*`）、方块间距/数量（11.0 / 11）、
以及质心旋转判据的期望符号。用户把自己那套 ZOffset 校准模型的打印区改了
（低沿往下延长 1mm、高沿往上延长 2mm → 深 13.484→16.484，中间圆点未动），
切片后实测外形变成 **119.5111 × 16.0630**，于是 `check_model_size` 报
`E_CAL_MISMATCH_002`。

目标：几何改成**一份按模型分档的数据文件**驱动，旧档数值原样搬（保证老模型逐字节不变），
新档新增；切片进来的模型按实测外形尺寸**自动认档**；并且新模型要登记进资产、前端「打开模型」
能选旧版/新版。后续再加校准模型 = 往这份文件里加一条。

## 已用实测几何钉死的事实（写档的依据）

| 量                           | 旧 ZOffset        | 新 ZOffset（用户 STL）                  |
| --------------------------- | ---------------- | ---------------------------------- |
| 模型整体 bbox                   | 136.219 × 34.019 | 136.219 × 34.019（未变）               |
| 打印区（顶面抬升区）mesh              | 120.000 × 13.484 | 120.000 × 16.484                   |
| 打印区相对 bbox 低沿               | 11.417 .. 24.901 | 10.417 .. 26.901（低沿 −1、高沿 +2）      |
| 打印区 X                       | 8.031 .. 128.031 | 8.031 .. 128.031（逐位相同）             |
| 切片后实测外形（=mesh − 0.421 挤出内缩） | 119.58 × 13.063  | **119.58 × 16.063**（与报错 actual 吻合） |

证据来源：`presets/assets/models/ZOffset_Calibration.3mf`（`3D/Objects/object_9.model`）
与用户 STL `F:\工程文件\3D打印\[v2.0]A1MINI新型MKP组件全盘1026\校准模型-ZOffset Calibration.001.stl`。

**由此推出的新档基准**：`datum_x` 不变（0.079）；`datum_y = 3.755 + 1 = 4.755`
（包围盒低沿往下移了 1mm，而图案要落在同一个物理点上）；方块间距 11.0、数量 11 不变；
质心期望不变（X 质量分布未动，且 ZOffset 判据只看长轴）。

## 改动清单

### ① 新数据文件 `presets/calibration_models.toml`（①开发源层，单文件多档）

照 `presets/plates/*.toml`「一块独立实体 = 一份数据、机型只引用 id」的惯例，
以及 `presets/registry/param_registry.toml` 的 `[[params]]` 形态，一个文件里 `[[models]]` 多档。
首批三条（前两条**逐字照搬今天代码里的常量**，第三条是新增）：

```toml
[[models]]
id = 'precise'            # mode = 'XY'（Precise/Rough 共用）
footprintW = 50.5   footprintH = 50.5   tolerance = 0.5
pattern = { lineSpacing = 4.0, hLineOffsetY = 10.49, hLineStartX = 0.5, hLineEndX = 10.5,
            vLineOffsetX = 10.5, vLineStartY = 0.49, vLineEndY = 10.49 }
centroid = { dxSign = -1, dySign = -1, minRatio = 0.02, square = true }

[[models]]
id = 'zoffset'            # 现行出厂
footprintW = 119.58   footprintH = 13.063   tolerance = 0.5
datumX = 0.079   datumY = 3.755   squareSpacing = 11.0   squareCount = 11
centroid = { dxSign = 1, dySign = 0, minRatio = 0.01, square = false }

[[models]]
id = 'zoffset-ext'        # 用户新版：低沿 −1 / 高沿 +2
footprintW = 119.58   footprintH = 16.063   tolerance = 0.5
datumX = 0.079   datumY = 4.755   squareSpacing = 11.0   squareCount = 11
centroid = { dxSign = 1, dySign = 0, minRatio = 0.01, square = false }
```

每档再挂一个 `asset = '<catalog 资产 id>'`（前端「打开模型」按它取版本）。

### ② Rust 读取层（复用已有逃生链，不新造路径依赖）

* 新增 `crates/postprocess/src/postproc/calib_models.rs`：`ModelProfile` 结构 + `load(dir)`。

* **复用** [machine\_dims.rs](file:///g:/project/MKPSupportEase/crates/postprocess/src/postproc/machine_dims.rs#L165-L238)
  那条链：显式 `install`（发布物）→ `MKPSE_PRESETS_DIR` → 仓库相对 `../../presets`；
  最好直接并进 `MachineTables` / `load_presets_dir`，一处读、一处装，不新增第二个进程级表。

* 解析失败的姿态照 [messages.rs](file:///g:/project/MKPSupportEase/crates/postprocess/src/diag/messages.rs#L14-L26) 那条：
  `deny_unknown_fields`，解析不了/一条都没有 = 构建或配置坏了 → 明确报错（不静默退化）。

### ③ 消费点改造 [disk/calibration.rs](file:///g:/project/MKPSupportEase/crates/postprocess/src/postproc/disk/calibration.rs#L134-L157)

* 删掉 `Z_PATTERN_*` / `XY_PATTERN_*` 常量，改为 `find_profile(mode, bbox)`：
  在**同一 mode 的档**里按 `|Δw|,|Δh| ≤ tolerance` 选偏差最小的一档。

* `check_model_size`（[L709-L744](file:///g:/project/MKPSupportEase/crates/postprocess/src/postproc/disk/calibration.rs#L709-L744)）改为消费档里的 w/h/tol；
  **认不出任何档时仍报** **`E_CAL_MISMATCH_002`**，但正文要列出「已知档位外形尺寸」清单
  （用户自己就能看出是缩放还是该换档）—— 沿用「坏就让它看得见」的既有口径。

* 图案定位：`generate_z_offset_calibration_new` 的 `fr_calibe_x_start/y_start` 取档里 `datumX/datumY`，
  循环用档里的 `squareSpacing/squareCount`；`generate_xy_calibration_new` 的 7 个模板偏移取档里 `pattern`。

* 旋转判据 `check_rotation_by_centroid` 的 `expected_dx_sign/expected_dy_sign/min_offset_ratio/is_square_model`
  取档里 `centroid`。

* 旧档数值与今天的常量逐个相等 ⇒ 现有 reference 逐字节（ByteExact）判据应**全绿**，这是「迁移零变」的证据。

### ④ 判据/测试

* 新增用例：旧 bbox(119.58×13.063) 命中 `zoffset` 且生成的图案逐字节不变；
  新 bbox(119.58×16.063) 命中 `zoffset-ext` 且 datum\_y 生效；未知尺寸（如 100×100）报错且正文含档位清单。

* 现有 [calibration\_insert.rs](file:///g:/project/MKPSupportEase/crates/postprocess/tests/calibration_insert.rs#L229-L252)、
  [reference\_calibration.rs](file:///g:/project/MKPSupportEase/crates/postprocess/tests/reference_calibration.rs)、
  `tests/fixtures/**`：凡断言里写死常量/外形的地方改为读档或补 fixture 档。

### ⑤ 资产（先把管线留出来，字节后补）

* `presets/assets.toml` 加一条 model 资产（例：`id = 'zoffset-calibration-ext'`、
  `path = 'models/ZOffset_Calibration_ext.3mf'`）。

* **占位**：把现有 `presets/assets/models/ZOffset_Calibration.3mf` 复制改名成上面那个路径
  （用户原话：先用旧版改名顶替；等他把自己导出的正式 3MF 补进来再换字节）。

* 按仓库发布链惯例跑工作台生成脚本（`gen-presets` / `gen-catalog` 那条链，见
  `docs/PUBLISH-ARCHITECTURE.md` 与 `docs/DATA-ARCHITECTURE.md` §1①）。

### ⑥ 前端「校准」页：模型可选旧版/新版

* [PageCalib.tsx](file:///g:/project/MKPSupportEase/src/app/calib/PageCalib.tsx#L81-L98) 的 `PLATE.z/xy` 现在把 model 写死成
  `'zoffset'` / `'precise'`（对应 `src/calib/*.generated.ts` 的预览）；改为**一档可选列表**。

* 「打开模型」那个 Modal（[L876-L910](file:///g:/project/MKPSupportEase/src/app/calib/PageCalib.tsx#L876-L910)）：在原有「从缓存打开 / 从云端获取」
  之上加**版本选择（旧版 / 新版）**，取的是 catalog 里的模型资产（`api.downloadCatalogFile`）。

* 预览数据：`src/calib/zoffset-calibration.generated.ts` 是按模型生成的（文件头写「勿手改」）。
  ⚠️ **生成器** **`tools/stl-svg`** **不在本仓**（`tools/` 只有 `assets`、`dev-server`，`npm run stl:svg` 也不存在）
  —— 新模型的预览 TS 要么回工作台重跑生成，要么按现有结构从新 STL 手工补一份。**这一条要先跟你确认。**

### ⑦ 文档（仓库根规则要求）

* `docs/DATA-ARCHITECTURE.md` §1① 白名单表加一行：`presets/calibration_models.toml` 是什么、唯一主人是谁。

* `HANDOFF.md` 台账加一条增量（对应本仓「进度台账 · 持续追加」的惯例，没有 AI\_HISTORY 那份文件）。

## 验证

1. `cargo test -p postprocess`（含 `tests/reference_calibration.rs` 的逐字节判据 + 新增档位用例）。
2. 拿用户那份真机 gcode（`.44432.0.gcode`）跑一遍后处理：确认不再报 `E_CAL_MISMATCH_002`，
   且插入的 11 个方块落点与旧模型同参数下逐字节预期的差异**只在 Y 基准上平移 1mm**。
3. 前端 `npm run dev` → 校准页：确认「打开模型」能选旧版/新版，两版预览都对得上。

## 风险 / 待办（如实登记）

* **tools/stl-svg 缺失**：新模型的预览产物怎么生成，需要你定（回工作台重跑 / 手工补）。

* 新模型的正式 3MF 还没有（本轮先占位）。

* 自动认档的已知局限（你已认可）：把旧模型整体放大 3mm 的输入，会被认成新档。

