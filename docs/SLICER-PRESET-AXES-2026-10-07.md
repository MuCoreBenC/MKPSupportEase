# 切片器档的喷嘴 / 层高（+ 一次「本地看不到」的核实）（2026-10-07）

> 分支：`feat/build-delivery-files`。作者真机反馈两件事：
> ① 切片器档（预设页 → 切片器配置 → 云端）9 份 BBS 的**喷嘴 / 层高整列都是「—」**；
> ② 首页套餐下载的 BBS，在预设页「本地」看不到。
> ① 是 bug（已修）；② 核实下来**不是** bug（结论与证据见 §2）。

## 1. 喷嘴 / 层高整列「—」——根因与修法

### 1.1 根因

切片器档那几行是 **release 行**（`catalog.files` 里 `kind = bbs_config` 的条目经
`catalogKindToFileKind` 分流过来）。而 **catalog 里没有喷嘴 / 层高这两格** ——
发布产物里登记的是身份（kind / fileName / path / machineId / sha256 / size），
不是"从路径读出来的轴"。

这两格的**唯一算法**在后端：`ipc/presets.rs::slicer_axes`（路径段 `0.4mm` → 喷嘴、
文件名尾部数字 → 层高），只有 `getPresetFiles()`（SlicerProfile 资产那条路）用它。
release 行从前根本没带这两个字段 ⇒ 表里那两列全落 `undefined` ⇒ 画成「—」。
（`ReleasePresetSource` 里连字段都没有，不是"值丢了"。）

### 1.2 修法（前端只搬，不重算）

```text
getPresetFiles()   ── fileName（文件本名，与 catalog 的 fileName 同一把钥匙）
       │                nozzle / layerHeight（slicer_axes 算好的）
       ▼
usePresetData.readRelease()   按 fileName 建对表 → 填进 ReleasePresetSource
       ▼
localRows / cloudRows 的 release 行   搬进行上 → 表格两列有值
```

- `presetTree.ts`：`ReleasePresetSource` 增 `nozzle?` / `layerHeight?`；两处 release 行照搬。
- `usePresetData.ts`：`readRelease` 从五个读变六个读（多一条 `api.getPresetFiles()`），
  按 `fileName` 建 `axesByName` 对表。
- **不在 TS 里从路径重算** —— 那会变成 `slicer_axes` 的第二份手抄；前端只做"把同一份
  文件的两处数据对起来"，与下载 / 应用 / 读正文认 `fileName` 是同一条口径。
- MKP 预设本来就没有这两格（`.toml` 不是切片器 profile）—— 字段不给，界面照旧不画
  （切片器档才有那两列）。Orca 那两条上游注册表里没有轴值，照实「—」，**不编**。

### 1.3 判据

- 探针 `scripts/probes/presets.mjs`：切片器档（云端）**喷嘴 / 层高不许整列「—」**
  （至少一行填得出值）。整列空 = 按 fileName 对 `getPresetFiles` 那一步断了。
- 后端侧算法本身的判据既有：`cargo test` 的 `slicer_axes_reads_nozzle_and_layer`
  与 `a1_bbs.nozzle == Some("0.4")`（`ipc/presets.rs`）。

## 2. 「本地看不到」的核实（不是 bug）

作者现象：首页套餐（A1 mini / 快拆版260628）下载后，预设页切片器档「本地」为空。
逐层核实（都在作者本机真实数据上跑）：

| 层 | 核实方式 | 结果 |
|---|---|---|
| 盘上 | `%APPDATA%\SupportEase\assets\bbs\…` | 两份在：`MKPProcess A1 0.2 0.10.json`、`MKPProcess A1 mini 0.4 0.20.json`；size / sha256 与目录登记逐字节一致 |
| 事件账 | `preset_events.json` | 两份都有 `DeliveryDownloaded`（23:20:43 / 23:21:21） |
| 后端判定 | 临时探针（`cargo test`，用真实数据根跑 `downloaded_entries`）| 返回 5 份 = 3 MKP + **这 2 份 BBS** ⇒ 是「已下载」 |
| 前端链 | 浏览器探针（mock）切到切片器 / 本地 | 本地表 4 行（含 release 行）⇒ release 行进本地表这条路通 |

**时间线证据**（截图自带的字）：切片器云端表那行「未下载 **9** 份」只可能出现在
**23:20:43 首页套餐下载之前** —— 之后至少 A1 mini 0.4mm 已在盘上，应显示「未下载 8 份」。
（同批截图里 MKP 档是「未下载 8 份」= 9 份减已下载的 `A1_MINI-fast.toml`，与此一致。）

⇒ 那两张截图拍的是**下载之前**的状态：那时切片器本地为空**是应然**（一份都没下）。
修好 §1 之后在客户端里**切一下 tab 或重开窗口**再进预设页，切片器 / 本地应出现
已下载的那两份；若仍为空，再按"真机复现"往下查（前端状态刷新那条链）。

## 3. 落点清单

| 层 | 文件 | 内容 |
|---|---|---|
| 前端数据形状 | `src/app/presets/presetTree.ts` | `ReleasePresetSource.nozzle/layerHeight`；`localRows` / `cloudRows` 的 release 行照搬 |
| 前端读 | `src/app/presets/usePresetData.ts` | `readRelease` 多读 `getPresetFiles()`，按 `fileName` 建对表 |
| 探针 | `scripts/probes/presets.mjs` | 切片器档喷嘴 / 层高不许整列「—」 |

## 4. 验证

`npx tsc -b`、`npm run build`、`npm run lint` 全绿；预设页探针（4173 静态预览）全绿，
其中新断言实测输出：

```text
[分类边界 · 切片器喷嘴层高] MKPProcess A1 0.2 0.10.json(0.2/0.10)
  || MKPProcess A1 0.4 0.20.json(0.4/0.20) || OrcaProcess A1 0.2 0.10.json(—/—)
```
