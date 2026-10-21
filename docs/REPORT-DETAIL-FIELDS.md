# 报告页字段清单（照搬 frontend-lab spec）

> 本文件 = `frontend-lab/lab/report-detail-fields-spec.md` 全文照搬（2026-10-11），
> 文首加两条**本仓口径**。原文在下，一行未改。

## 本仓口径（搬运当天记）

1. **spec §2 那五组字段本仓后端尚未透传**（涂胶 / 擦料塔 / 切片工艺 / 管线每步 detail / estimatedPrintSeconds）。
   页面口径：**后端没给就不摆** —— 不编占位值，整组不出现。
   后续补字段 = `ReportDetail` 加键 + `ipc/report.rs` 透传 + `buildGroups` 加一组，页面样式不用动。
2. **本仓后续后处理产出的 `_meta.json` 要与旧 Go 程序那份 JSON 同形**
   （作者 2026-10-11 口径；样本：`/Users/wzy/Desktop/MKPSE_log/后处理执行报告导出/27351.0_.gcode_2026-08-19_11-28-46/27351.0_20260819_112718_meta.json`）。
   同形是硬要求：报告页的分组、键名、统计格全部按那份 JSON 的字段名取值。
3. **§2 的五组已落地（2026-10-11，分支 `fix/postprocess-command`）**：写侧 `archive.rs`
   （`ReportFacts`，素材只认内核搬出来的 IR 终态 / 统计 / 头部事实）按 §3 的 v3 提案产出
   `detail.estimatedPrintSeconds` + `glue` / `tower` / `slicer` 三个分包（键名照旧版同名搬）
   + `pipeline[].detail`；读侧 `ipc/report.rs` 透传；页面 `buildGroups()` 三组
   + 打印组「比切片器预估 +N%」。第 1 条的「没给就不摆」照旧执行：失败 / 取消记录
   三组整组缺键；§4「明确不要的」逐条遵守。
4. **展示格式的三处本仓改动（2026-10-11，作者口径；§2 表格里的「显示格式」以此为准）**：
   - **单位跟着值走**，不挂在字段名上：`塔高 0 mm` / `最高 Z 180 mm`（原文是标签写
     「塔高 mm」、值光秃秃一个数）。
   - **`penOffsetX/Y/Z` 拆三行**（`笔偏移 X` / `Y` / `Z`，各带 `mm`）——原文 `-0.9, 23, 4`
     逗号串一格，三个数谁是谁看不出来。
   - **键值是一张两列表**（`PageReport.module.css` 的 `.kv`）：字段一列、参数一列，
     同一组里的值从同一条竖线起排 —— §5 第 3 条「同一列的值左缘对齐」的落地。

5. **参数快照（2026-10-21）**：`detail.snapshot = { params, ir }` ——
   `params` 是**切片那一刻**预设 TOML 里读到的全量参数（注册表 key → 控件看得懂的字符串），
   `ir` 是 IR 终态原样 JSON（键为 PascalCase）。写侧在钩子 `archive_success` 里算
   （读 `job.toml` + 运行时 `catalog.json` 的注册表）；**失败 / 取消记录不写这一格**
   （没有 IR 终态，谈不上"当时认成了什么"）。
   报告页「参数快照」组按 TOML 节折叠、默认收起、可搜索，末尾附 IR 原文；
   「擦料塔」组下面多一张俯视图 —— **预设写的坐标（实线）vs 程序认到的坐标（虚线）**，
   两者不一致时标红（真机上那次"塔位置不是我的位置"就是靠它才看清的）。

---

# 报告页详情字段清单（`_meta.json` 要补什么）

2026-10-11。报告页已按「左列表 + 右详情 / 分组页签 / 组间横线」落地
（`src/versions/a05/report/PageReport.tsx`），但**详情页有一半内容是空的** ——
不是前端没画，是 schema v2 的 `ReportDetail` 没把旧版 `_meta.json` 里那些字段透出来。
这份文档写清楚：**要哪些字段、在哪一层、什么类型、前端拿它干什么、没有的时候怎么办**。

对照样本：旧版真记录
`/Users/wzy/Desktop/MKPSE_log/后处理执行报告导出/27351.0_.gcode_2026-08-19_11-28-46/27351.0_20260819_112718_meta.json`
（2026-08-19 11:27:20，A1_MINI / A1MF_260628.toml，成功，2.07 秒）。

---

## 0. 一句话结论

要补的是**四组**内容：**涂胶 / 擦料塔 / 切片工艺 / 管线每步的 detail 多行文案**，
外加**一个数字**（`estimatedPrintSeconds`，用来算「比切片器预估多了多久」）。
其余前端已经能显示。

---

## 1. 现在已经有的（schema v2，前端已消费，不用动）

| 前端用在哪 | JSON 路径（旧版） | 键 | 类型 | 契约字段 |
|---|---|---|---|---|
| 列表行 | 顶层 | `dateTime` | string | `ReportSummary.startedAt` |
| 列表行 | `day` 目录名 | — | string | `ReportSummary.day` |
| 列表行 / 结论 | 顶层 | `duration` | string | `ReportSummary.elapsedMs` |
| 列表行 / 结论 | 顶层 | `presetName` | string | `ReportSummary.presetName` |
| 列表行 / 结论 | `detail` | `machineType` | string | `ReportSummary.machineType` |
| 列表行 / 结论 | 顶层 | `originalPath` 基名 | string | `ReportSummary.gcodeName` |
| 列表行 | 顶层 | `success` / 错误 | bool / string | `ReportSummary.ok` / `.error` |
| 详情 · 打印 | `detail` | `towerHeight` | number | `ReportDetail.stats` |
| 详情 · 打印 | `detail` | `glueLayerCount` | number | `stats` |
| 详情 · 打印 | `detail` | `totalLayerNumber` | number | `stats` |
| 详情 · 打印 | `detail` | `maxZHeight` | number | `stats` |
| 详情 · 打印 | `detail` | `originalPrintTime` | string | `stats` |
| 详情 · 打印 | `printTime` | `totalSeconds` / `segments` | number | `ReportDetail.printTime` |
| 详情 · 管线 | `pipeline[]` | `name` / `duration` | string | `pipeline[].id` / `.elapsedMs` |
| 详情 · 文件 | 顶层 | `originalPath` / `finalPath` | string | `inputFile.path` / `outputFile.path` |
| 详情 · 文件 | `detail` | `gcodeSizeMB` / `outputSizeMB` | number | `inputFile.sizeBytes` / `outputFile.sizeBytes` |
| 详情 · 文件 | 顶层 | `originalSha256` | string | `inputFile.sha256` |
| 详情 · 文件 | `invocation` | — | — | `ReportDetail.invocation` |
| 详情 · 调用 | 顶层 | `presetName` 路径 | string | `ReportDetail.presetPath` |

---

## 2. 要补的（前端等着用，缺了那几组就整组不显示）

### 2.1 涂胶（组名「涂胶」）

| 键（照旧版 `detail` 同名） | 类型 | 样本值 | 前端标签 | 显示格式 |
|---|---|---|---|---|
| `glueTotalLength` | number（mm） | `9350.845588078173` | 涂胶总长 | `9.35 m`（÷1000，两位小数） |
| `gluePenSwitches` | number | `30` | 换笔 | `30 次` |
| `gluePassCount` | number | `1` | 涂胶遍数 | `1 遍` |
| `glueSpeed` | number | `70` | 涂胶速度 | `70 mm/s` |
| `penOffsetX` / `penOffsetY` / `penOffsetZ` | number | `-0.9` / `23` / `4` | 笔偏移 XYZ | `-0.9, 23, 4` |
| `glueZLiftHeight` | number | `5` | 抬笔高度 | `5 mm` |
| `glueZLiftHeightFirstLayers` | number | `6` | 抬笔高度（首层） | 并在同一行：`5 mm（首层 6）` |
| `glueInterModelLiftHeight` | number | `2` | 模型间抬笔 | `2 mm` |
| `glueZOffset` | number | `-0.05` | Z 偏移 | `-0.05 mm` |
| `primeTriggerLayers` | number | `30` | 润笔 | `30 层 · 102.4 mm`（与下一条同一行） |
| `primeTotalLength` | number（mm） | `102.4131416821871` | 润笔 | 见上（一位小数） |

组摘要（`组名` 后面那句）：`30 层 · 9.35 m · 换笔 30 次`。

### 2.2 擦料塔（组名「擦料塔」）

| 键 | 类型 | 样本值 | 前端标签 | 显示格式 |
|---|---|---|---|---|
| `towerLayers` | number | `38` | 层数 | `38 层 · 6.46 mm`（与 `towerHeight` 同一行） |
| `towerHeight` | number（mm） | `6.46` | 层数 / 高度 | 见上 |
| `towerExtrudeRatio` | number | `1.4` | 挤出比 | `1.4` |
| `towerZLift` | number | `0.7` | 抬笔 / 安全 Z | `0.7 / 0.7 mm`（与 `towerSafeZOffset` 同一行） |
| `towerSafeZOffset` | number | `0.7` | 抬笔 / 安全 Z | 见上 |
| `towerTravelSpeed` | number | `200` | 移动速度 / 模式 | `200 mm/s · fast`（与 `fastTowerMode` 同一行） |
| `fastTowerMode` | string | `"fast"` | 移动速度 / 模式 | 见上 |

组摘要：`38 层 · 6.46 mm`。

### 2.3 切片工艺（组名「切片工艺」）

| 键 | 类型 | 样本值 | 前端标签 | 显示格式 |
|---|---|---|---|---|
| `slicerVersion` | string | `"BambuStudio 02.08.02.60"` | 切片软件 | 原样 |
| `machinePresetName` | string | `"Bambu Lab A1 mini 0.4 nozzle"` | 机型预设 | 原样 |
| `processPresetName` | string | `"0.20mm Standard @BBL A1M"` | 工艺预设 | 原样 |
| `filamentType` | string | `"PETG"` | 耗材 / 喷嘴 | `PETG · 0.4 mm`（与 `nozzleDiameter` 同一行） |
| `nozzleDiameter` | number | `0.4` | 耗材 / 喷嘴 | 见上 |
| `slicerIroningEnabled` | bool | `true` | 熨平 | `开 · auto · 阈值 7.9`（与下两条同一行） |
| `ironingMode` | string | `"auto"` | 熨平 | 见上 |
| `ironingThreshold` | number | `7.9` | 熨平 | 见上 |
| `treeSupport` | bool | `false` | 支撑 | `树状 关 · 圆盘擦拭 关`（与 `useDiskWipe` 同一行） |
| `useDiskWipe` | bool | `false` | 支撑 | 见上 |

组摘要：`BambuStudio 02.08.02.60`。

### 2.4 管线每步的 detail（`pipeline[]`）

| 键 | 类型 | 样本值 | 前端怎么用 |
|---|---|---|---|
| `pipeline[].detail` | string（**可含 `\n`**） | `"擦料塔层数: 38\n擦料塔高度: 6.46mm\n…"` | 每步名字下面按行铺小字（`white-space: pre-line`） |

这一步是**排查时最值钱的一段**：机型维度 180×180、禁区数量 0、擦料塔位置 (20.0, 20.0)、无碰撞 ……
现在 `ReportPipelineStep` 只有 `id` / `elapsedMs` / `message` / `status`，`message` 是一行进度文案
（"第一遍扫描: 99%"），detail 全丢了。

### 2.5 一个数字：打印时间对比

| 键 | 类型 | 样本值 | 前端怎么用 |
|---|---|---|---|
| `estimatedPrintSeconds` | number（秒） | `2952.9109519850203` | 与 `stats.originalPrintTime` 比 → **「比切片器预估 +43%」** |

现在只能显示 `printTime.totalSeconds`（49 分 13 秒）+ 切片器原话字符串
（`3h 16m 52s; total estimated time: 3h 22m 55s`），**差值是算不出来的** ——
字符串要解析才能比。给了这个数字，对比条 / `+43%` 才能上（HTML 抽卡里作者最看好的那块）。

---

## 3. 建议的 JSON 形状（v3 提案）

旧版把这些键平铺在顶层 `detail` 里（70+ 个键混在一起）。建议**按组分包**，前端按组直接渲染：

```jsonc
{
  "dateTime": "2026-08-19T03:27:20Z",
  "duration": "2.069124416s",
  "presetName": "A1MF_260628.toml",
  "success": true,

  "detail": {
    // —— 现状就有（stats 16 项） ——
    "gcodePath": "…",
    "gcodeSizeMB": 1.466983,
    "outputSizeMB": 1.865431,
    "estimatedPrintSeconds": 2952.9109519850203,   // ← 新增（2.5）
    "towerHeight": 6.46,
    "glueLayerCount": 30,
    "totalLayerNumber": 60,
    "maxZHeight": 8.5,
    "originalPrintTime": "28m 15s; total estimated time: 34m 18s",

    // —— 新增三个分包（2.1 / 2.2 / 2.3），键名照旧版 detail 原名搬过来 ——
    "glue": {
      "glueTotalLength": 9350.845588078173,
      "gluePenSwitches": 30,
      "gluePassCount": 1,
      "glueSpeed": 70,
      "penOffsetX": -0.9, "penOffsetY": 23, "penOffsetZ": 4,
      "glueZLiftHeight": 5,
      "glueZLiftHeightFirstLayers": 6,
      "glueInterModelLiftHeight": 2,
      "glueZOffset": -0.05,
      "primeTriggerLayers": 30,
      "primeTotalLength": 102.4131416821871
    },
    "tower": {
      "towerLayers": 38,
      "towerHeight": 6.46,
      "towerExtrudeRatio": 1.4,
      "towerZLift": 0.7,
      "towerSafeZOffset": 0.7,
      "towerTravelSpeed": 200,
      "fastTowerMode": "fast"
    },
    "slicer": {
      "slicerVersion": "BambuStudio 02.08.02.60",
      "machinePresetName": "Bambu Lab A1 mini 0.4 nozzle",
      "processPresetName": "0.20mm Standard @BBL A1M",
      "filamentType": "PETG",
      "nozzleDiameter": 0.4,
      "slicerIroningEnabled": true,
      "ironingMode": "auto",
      "ironingThreshold": 7.9,
      "treeSupport": false,
      "useDiskWipe": false
    }
  },

  "pipeline": [
    {
      "id": "collision",
      "name": "碰撞检测",
      "status": "complete",
      "duration": "49.4565ms",
      "detail": "无碰撞，擦料塔位置安全\n擦料塔位置: (20.0, 20.0)"   // ← 新增（2.4）
    }
  ]
}
```

对应前端契约（`src/api/contract.ts`）建议加：

```ts
export interface ReportGlueFacts { … }   // 2.1 那 11 个键，全 number
export interface ReportTowerFacts { … }  // 2.2 那 7 个键
export interface ReportSlicerFacts { … } // 2.3 那 10 个键
export interface ReportDetail {
  …既有…
  glue?: ReportGlueFacts | null
  tower?: ReportTowerFacts | null
  slicer?: ReportSlicerFacts | null
}
export interface ReportPipelineStep {
  …既有…
  /** 这一 step 自报的事实（多行，可含 \n） */
  detail?: string
}
```

**三个分包一律可选**（`| null`）：旧记录（2026-08-19 那批）顶层 `detail` 里虽然有这些键，
但没分包 —— 读不到就是 `null`，前端**整组不摆**（不是摆一排「—」）。

---

## 4. 明确不要的（别顺手全透传）

| 类别 | 键（举例） | 为什么不摆 |
|---|---|---|
| 内部开关 | `disableFrontCoverAlarm` / `l803LeakPreventFlag` / `nozzleCoolingFlag` / `disable3rdLayerClogDetect` / `firstPenRevitalizationFlag` | 用户看不懂，也不该由这份报告解释 |
| 恒 0 值 | `glueLinesAdded 0` / `interfacesFound 0` / `userDryTime 0` / `ironingSuppressExpand 0` / `diskStaggerAngle` | 摆一排 0 是噪音；真有值了再说 |
| 空串 | `appVersion` / `slicerVersion`（顶层那份，空） / `postProcessScript` / `intermediatePath` / `calibrationMode` | 空值不摆（纪律：没有就不给入口/不给格子） |
| 重复键 | `totalLayers` 与 `totalLayerNumber`（都 60）、`modelHeight` 与 `maxZHeight`（都 8.5） | 各留一个即可 |
| 超长路径 | `/var/folders/xq/…/Metadata/.27351.0.gcode` | 只留基名 + `title` 挂全路径（列表行尤其） |
| 中间态 | `duration` 与 `detail.duration` 两份（2.069124416s / 2.069123125s，差 1µs） | 留一份（顶层那份） |

---

## 5. 前端这边的约定（拿到字段后怎么摆，已写死在代码里）

1. **有值才摆**：一组里所有值都是 `—` 就不摆这一组（`hasAnyValue`），空组占一条横线等于喊狼来了。
2. **单位**：mm 归 mm（`6.46 mm`），总长超过 1000mm 换 m（`9.35 m`），时间 <1000ms 用毫秒、否则秒 / 分。
3. **数字等宽对齐**（`font-variant-numeric: tabular-nums`），同一列的值左缘对齐。
4. **组分隔是横线**（组名 + 摘要坐在线上），不是气泡框。
5. **不编数据**：后端没给的字段，前端一律不填占位数字 —— 宁可整组缺席。
6. 新增一组只需往 `buildGroups()` 里加一项，页面 / 页签 / 滑盖都不用改。
