# 切片器档的喷嘴 / 层高（+ 一次「本地看不到」的核实）（2026-10-07）

> 分支：`feat/build-delivery-files`。作者真机反馈两件事：
> ① 切片器档（预设页 → 切片器配置 → 云端）9 份 BBS 的**喷嘴 / 层高整列都是「—」**；
> ② 首页套餐下载的 BBS，在预设页「本地」看不到。
> **两件都是 bug，都已修**：① 见 §1；② 的根因是**页签常驻的跨页陈旧数据**，见 §2
> （第一版结论曾写"不是 bug"，被真机实验推翻，当场更正）。

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

## 2. 「本地看不到」——**是 bug**：页签常驻的跨页陈旧数据（已修）

作者现象：首页套餐（A1 mini / 快拆版260628，之后还有 X1C / lite）下载并应用之后，
预设页切片器档「本地」看不到刚下的那些行。

### 2.1 逐层核实（都在作者本机真实数据上跑）

| 层 | 核实方式 | 结果 |
|---|---|---|
| 盘上 | `%APPDATA%\SupportEase\assets\bbs\…` | 3 份都在（A1 0.2 / A1 mini 0.4 / **X1 0.4**）；size / sha256 与目录登记逐字节一致 |
| 事件账 | `preset_events.json` | 每份都有 `DeliveryDownloaded`（23:20:43 / 23:21:21 / 23:35:09） |
| 后端判定 | 临时探针（`cargo test`，真实数据根跑 `downloaded_entries`）| 返回 7 份 = 4 MKP + **3 份 BBS**（含 X1 那份）⇒ 全是「已下载」 |
| 前端代码 | 从 vite dev server（5321）抓**运行时**模块比对 | 是最新代码（`nozzle: p.nozzle`、`subtitle: ""` 都在）|
| 前端链 | 浏览器探针（mock）切到切片器 / 本地 | 本地表 4 行（含 release 行）⇒ release 行进本地表这条路通 |
| **界面实测** | 直接在作者窗口上截图 + 点击（只读操作）| 切片器本地表**停在 2 行**（A1 0.2 / A1 mini 0.4）——X1 那行不在；**切走再切回也不刷新** |

### 2.2 根因

**外壳的页签 2026-10-05 起是「常驻 + 切显示」**（`src/app/App.tsx`：所有页面都常驻
在 DOM 里，切 tab 只换 `visibility`，**不重挂载**）—— 预设页因此**只在首次进入时读一次
数据**；而下载发生在**首页**（写盘 + 写使用中指针），预设页手里那份快照永远是旧的。
「切走再切回」不重挂载 ⇒ 也不重读（界面实验复现：数据停在 2 行）。

> 第一版核实停在"后端与前端逻辑都没问题"，据此写成"不是 bug、应是下载前的应然状态"；
> **在窗口上直接实验后被推翻**。教训一句话：**跨页数据流的问题，两边各自都"对"，
> 错在两页之间没人负责** —— 常驻页与跨页写是一对，改的时候要成对看。

### 2.3 修法

预设页订阅**使用中指针的实质变化**（`activeKey` = origin/machineId/versionId/fileName）→
重读 delivery 那一路。它是**每一条"下载并应用"都会写的格子**（首页那颗按钮必写它），
所以跨页那一半补上了；本页自己的下载 / 删除早已自带重读，不重复。
首帧（ready 之后）会多读一次，接受。

### 2.4 顺带说清的两件事（不是 bug，是期望落差）

- **套餐里的切片器配置每机型只有一份**（`A1_MINI_*` 与 `X1C_default` 的 assetRefs
  各引 `*-bbs-04-020` / `*-bbs-04-024` 一份）；**0.2mm 那几份是可选**（不在任何
  bundle 里），要手动到切片器档云端表点「下载」。
- 作者看到的**喷嘴 / 层高整列「—」**由 §1 修掉（release 行按 fileName 对 `getPresetFiles`）。

## 3. 落点清单

| 层 | 文件 | 内容 |
|---|---|---|
| 前端数据形状 | `src/app/presets/presetTree.ts` | `ReleasePresetSource.nozzle/layerHeight`；`localRows` / `cloudRows` 的 release 行照搬 |
| 前端读 | `src/app/presets/usePresetData.ts` | ① `readRelease` 多读 `getPresetFiles()`，按 `fileName` 建对表；② **跨页再同步**：`activeKey` 实质变化 → 重读 delivery |
| 探针 | `scripts/probes/presets.mjs` | 切片器档喷嘴 / 层高不许整列「—」 |

**没动但记一笔**：首页那张「下载并应用」按钮的"已下载 / 漂移"判据是**同一个坑的
另一面**（页签常驻 + 跨页写）—— 预设页下载后回首页，按钮状态可能还是旧的。
它有自己的数据流（`useBundleFiles` 等），这一刀不碰，真机再看到再处理。

## 4. 验证

`npx tsc -b`、`npm run build`、`npm run lint` 全绿；预设页探针（4173 静态预览）全绿，
其中新断言实测输出：

```text
[分类边界 · 切片器喷嘴层高] MKPProcess A1 0.2 0.10.json(0.2/0.10)
  || MKPProcess A1 0.4 0.20.json(0.4/0.20) || OrcaProcess A1 0.2 0.10.json(—/—)
```

**真机核实用过的三件工具**（都用完即删，不留在仓库里）：临时 `cargo test` 探针
（真实数据根跑 `downloaded_entries`）；从 vite dev server 抓运行时模块与源码比对；
在作者窗口上截图 + 只读点击（DPI 感知的客户区坐标 + `mouse_event`）。

## 5. 作者验收（2026-10-07 四轮）与「下一刀」的边界

作者裁决：**「下载了但预设页看不到」这个 Bug 认定彻底修复**（`9674a98` 收口），
但**不是**"下载功能彻底完成"，也**不是**"全局状态一致性已解决"——两件事分开看：

| 面 | 状态 |
|---|---|
| 文件下载 / 盘上内容 / 后端 `downloaded_entries` / 套餐链 | ✅ |
| 预设页首次读取、首页下载后预设页自动刷新（本次修复） | ✅ |
| 喷嘴 / 层高显示（§1） | ✅ |
| **首页按钮状态是否实时跟随本地下载状态** | ✅ **§6（投递面代次）** |
| 全局持久化状态统一管理 | ⚠️ 尚未收口（`deliveryState` 是往前走的一小步，见 §6） |

**下一刀（单独做，不与大重构混）**：「首页『下载并应用』按钮的状态跟随实际本地状态」
—— 同一个坑（常驻 Tab + 跨页写）的另一面，但它有自己的数据流（`useBundleFiles` 等），
**不许和"统一状态入口"混成一个重构**。

**这一刀挣来的一条架构事实**（以后收状态入口时照着它走）：

> **常驻 Tab 下，任何依赖本地下载状态的页面，都不能把"首次读取"当成永久真相。**

否则就是今天修预设页、明天修首页、后天又冒出第三个页面拿着旧快照。将来把这类
"持久化状态 + 它的订阅面"收进 AppState 那一个入口（`docs/APP-STATE.md` 的方向），
每收一个页面，先问它一句："你手里的快照，别的页面写的时候谁会通知你？"

## 6. 下一刀落地：「投递面代次」（2026-10-07 五轮）

作者发话后按"单独一刀、不混重构"的边界做掉：**首页按钮状态跟随实际本地状态**，
顺带把**第三个消费者**（BBS 页的交付面）一起收了。

### 6.1 为什么不是"照抄第一刀"

第一刀拿「使用中指针」（AppState）当跨页信号 —— 它只覆盖**"下载并应用"**这一类
（必写 active）；而**"只下载不应用"**（预设页的下载，恰是它的本职）**不写 active** ——
照抄第一刀会是**假修复**：首页按钮不知道预设页刚下了东西。所以这一刀换成**直说**。

### 6.2 这一刀做了什么：新模块 `src/app/state/deliveryState.ts`

```text
写侧（凡改投递面处，写完就发）          读侧（放进 effect 依赖，收到就重读）
  预设页 下载 / 批量 / 删本机 / 删归档     预设页 readRelease（第一刀的 activeKey 订阅退役）
  首页「下载并应用」（成没成都发）        首页那两份单子（替代页内 deliveryTick）
                                        BBS 页 useBbsDelivered（第三个消费者，顺手收掉）
```

- 只有**一个自增代次**，不存投递数据（数据仍由各页按需读）——它不是状态容器，
  更不是全局状态入口（那是 AppState 的方向；将来收的时候把代次并进去，消费者不变）；
- **失败也广播**：`deliver` 是"先归档旧份、再换新"，成了一半也是变了 → 放 `finally`
  （首页 `applyCurrent` 与预设页 `downloadRelease` 同一条）；
- **单一触发路径**：预设页那四个写出口不再各自显式重读，只广播。

### 6.3 判据与验证

- `tsc -b` / `npm run build` / `npm run lint` 全绿；
- 探针 `home-flow.mjs`（首页 + 校准页，控制台零 error）与 `presets.mjs`（预设页全量）全绿；
- 跨页刷新**在浏览器（mock）里演不出来**（mock 的下载必抛），留给真机验收：
  **预设页下一份 → 切回首页，按钮应从「下载并应用」变「应用」**。

### 6.4 异步重读的竞态守卫（作者点名要查的那一条）

作者问的：**"投递面代次连续触发两次时，旧的读晚返回、会不会把新快照写回 UI？"**
答案分两半：

- **首页 / BBS 页：不会。** 两处的重读都在**一个 effect 里** —— 依赖变化 → cleanup
  先把上一轮的 `alive` 置 false → 旧读晚回来在 `if (!alive) return` 处被丢掉
  （React 保证"先 cleanup、再跑新 effect"，守卫是硬保证，不是惯例）。
- **预设页：原来会。** 它的快照有**三个读源**（首屏那条链 / 换目录后的重读 /
  投递面代次触发的重读），彼此**不在同一个 effect 里** —— effect 级守卫管不到
  跨 effect 的并发：「首屏那次读先发、慢回」正好撞上「下载完的广播触发的新读」时，
  **旧读晚回来会把新快照盖回去**（新数据闪一下又变旧）。

**修法（`usePresetData.ts`）**：快照的写入口收成一个 —— `readReleaseInto()`，
带**代次守卫（后发者优先）**：

```text
readSeqRef++  →  读（六个 IPC）  →  seq 还对得上才 setRelease
```

三个读源都走它；**只有"最后发起的那次读"允许写快照**。它盖住所有并发组合，
不需要每条读源各写一套 cancel / abort。读失败照抛、不写快照（保持上一份）。

> 这条与"代次信号"是一对，职责别混：**`deliveryState` 管"要不要重读"（通知），
> `readReleaseInto` 的 `seq` 管"读回来的算不算数"（次序）。**

### 6.5 还留着的一笔

**全局持久化状态统一管理**仍是独立方向 —— 本刀只把"投递面"这一族的信号补上了，
不碰 AppState 的收纳。
