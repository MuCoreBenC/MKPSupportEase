# 工作台参数编辑「跟手化」改造方案

## Context（为什么改）

作者实测：工作台**参数台**里连点数字步进器时，数字**不跟手** —— 停下之后才"慢慢追上"点击的次数，整体又慢又卡。作者要把它做成客户端（`src/app/params/`）那种手感，并把客户端那枚**橙色「改之前的值」**回退 chip 抄过来。

根因**不是**视觉重排，而是**一次点击引爆一串同步 IPC + 整页重取**：

- 一次值写入：`ParamsPage.writeValue` → 外壳 `App.run()` → `await wb.applyDraft(...)`，随后 `setBook(out.view)`、**`setTick(n+1)`**、`refreshReport()`。
- `tick` 又让 ParamsPage 三个 effect 各再发一次 IPC：`wb_registry`、**整页 `wb_desk`**、对照模式的 `wb_matrix` + 抽屉 desk。
- Rust 侧这些 `#[tauri::command]` 全是**同步的（跑主线程）**；`wb_preflight` 更重（parse recipes + 摸盘建 catalog）。主线程被连续占死 ⇒ 数字滞后、停下才追上来。

目标：**本地即时反馈 + 一次手势最多一次落库 + 长列表不再整片重取**，并补上橙色回退 chip。

## 方案

### 1. 让「值写入」变轻（核心）
- `App.run(...)` 增 `opts.quiet`：值写入走 quiet —— **不拨 `tick`、不置全局 `busy`**；`refreshReport()` 改成 **700ms 尾随去抖**（`refreshReportSoon`），不再每笔一跑 preflight。
- 值写入**不再靠 `tick` 重取整页**：`writeValue` 直接吃 `wb_apply_draft` **已经返回**的 `out.desk`（后端按 `refresh` 顺带给了，今天只吃了 `out.matrix`，desk 白多问一次）。
- 撤销 / 重做 / 保存 / 放弃 / 建删版本等**结构操作照旧拨 `tick`** —— 那条全页重取的路**一行不动**（`tick` 的其它消费者 AssetsPage / BundlesPage / BuildPage 不受影响）。

### 2. 连续手势合批（「跟手」的关键）
- 新建 `src/workbench/editQueue.ts`（模块级 store，照 `c14/toast.ts` / `pageActions.ts` 的写法）：
  - 按 `cellKey`（`机型|版本uid|level|key`）**latest-wins** 收集待写值；**单飞**（同一时刻只有一条 `applyDraft` 在飞）+ 120ms 尾随；飞行中又来了点击 → 落地后立即续发。单飞是硬要求（两条在飞时晚回的先到会覆盖早发的 base）。
  - 显示值 = **乐观 overlay 叠在 base 上**：`shown = applyOverlay(desk|matrix|drawerDesk, overlay)`；overlay 永远最后叠，响应**只写 base**；响应回来先比 `refreshKey`，与当前 `refresh` 不一致就丢弃（交给 effect 重取）。
  - 提交走 `onApply(label, patches, refresh, { quiet: true })`。
  - 失败：清掉该批 overlay 条目 + `toasts.push`，回落到 base。
- `NumberField` 补一条：按压连发 / 尾随提交期间**不回写 prop**（`holdOn`）—— 否则响应一到就把连发基点拽回旧值（"手滑改了"就是这么来的）。
- 效果：连点 20 下 = 本地数字**立刻跟手**，落库**只 1 条 `apply_draft`**，撤销**也只 1 条**（正好合既有口径"一次手势 = 一条撤销"）。

### 3. 橙色「改之前的值」
- 判据：`cell.dirty && cell.rest !== null`。`Cell.rest` = **该层盘上钉着的值（草稿不算）**（[api.ts](file:///g:/project/MKPSupportEase/src/workbench/api.ts#L341-L345)；`BatchEdit` 已在用它做"恢复修改前的"）。`rest === null` 时没有"改之前"可言，不摆 chip。
- 位置（控件右缘不动，箭头不左右跳）：
  - 中间行 `ParamLine` 的 `.pRowCtl` 里、`CellEditor` **之前**（[ParamsPage.tsx:1512-1525](file:///g:/project/MKPSupportEase/src/workbench/views/ParamsPage.tsx#L1512-L1525)）；
  - 右栏 `ParamDetail.layerRow` 的 `.pEdCtl` 里、控件**之前**（[ParamDetail.tsx:233-244](file:///g:/project/MKPSupportEase/src/workbench/views/ParamDetail.tsx#L233-L244)）；
  - 对照模式格子太挤 → 主入口放右栏「各版本取值」行，不在矩阵格子里挤。
- 点击 = 写回该层 `rest`（与「恢复修改前的」同一条口径）。CSS 新增 `.pRevert`（`c14.module.css` 里 `.pEdReset` 附近），**照抄**客户端 [ParamRow.module.css:301-326](file:///g:/project/MKPSupportEase/src/app/params/ParamRow.module.css#L301-L326)，颜色直接用现成 token `--warn-surface / --warn-ink / --warn-line`（[c14tokens.css:68-70](file:///g:/project/MKPSupportEase/src/workbench/c14tokens.css#L68-L70) 亮 / 220-225 暗）—— 与客户端像素同源。
- 注意：chip（写回 `rest`）与 `↶`（挂回继承、写 `null` 删键）**语义不同**，两枚并存时 title 要说清。

### 4. 其余页顺一遍（最小改，不重写）
- **AssetsPage**：`run` 后 `tick++` 与 `applyVis` 自己 `load()` 把同一份清单读了两次 → 值写改走 `quiet`。`setCalib` 改完重读盘真字节是作者点名要的，**保留**。
- **BundlesPage / BuildPage / MachinesPage**：`tick` 只在生成 / 保存 / 放弃 / 建删版本等结构操作时动，语义正确 → **保留**（仅补注说明）。
- **AnnotatePage**：自带页头槽、自管撤销，**不动**。

## 改动文件
| 文件 | 改什么 |
| --- | --- |
| `src/workbench/editQueue.ts` | **新建**：单飞 + 120ms 合批 + overlay + settleAll |
| `src/workbench/App.tsx` | `run` 加 `opts.quiet`；preflight 去抖（`refreshReportSoon`）；undo/redo/save/discard 前 `await settleAll()` |
| `src/workbench/views/ParamsPage.tsx` | 吃 `out.desk`；overlay 后的 desk/matrix 传给 `GroupCard`/`CompareMatrix`/`ParamDetail`；行上 chip |
| `src/workbench/components/field/NumberField.tsx` | 连发/尾随期间 `holdOn` 不回写 prop |
| `src/workbench/views/ParamDetail.tsx` | 各版本取值行加 chip |
| `src/workbench/c14.module.css` | 新增 `.pRevert`（橙色 chip） |
| `src/workbench/views/AssetsPage.tsx` | 值写走 quiet |

## 验证
命令（全绿才收）：
```
cargo fmt --check
cargo clippy --all-targets -- -D warnings                     # 默认 + --features workbench 两套
cargo test                                                    # 默认
cargo test -p mkp-support-ease --features workbench --lib
npx tsc -b --force
npm run lint
npm run build && npm run build:workbench
```
手动（`npm run tauri:workbench:dev`）：
- 连点步进器 20 下 → 数字**跟手**、不回跳、停下不"追账"；
- 长按松手 → 只进 **1 条**撤销；Ctrl+Z 一次退一格；
- 搜索 / 切机型过程中连点 → 不串行、不丢点击；
- 橙色 chip 点一下 → 该行回到已保存值、chip 消失。

可选探针：照 [params-settings.mjs](file:///g:/project/MKPSupportEase/scripts/probes/params-settings.mjs) 的起法，给 mock 加调用计数（`window.__wbCalls`），断言"**一次点击 = 1×apply_draft、0×desk / 0×registry / 0×preflight**"。

## 风险
- overlay 与 base 错位 → 靠 `refreshKey` 丢弃；**隐藏页的待写不能丢**（settle **不按"当前页"筛**，非激活页仍挂载）。
- 合批别把不同格并成一条撤销 → 按 `cellKey` + `refreshKey` 切批。
- preflight 去抖后页头徽标最多迟 700ms（低频、可接受）。
- chip 与 `↶` 两条语义别混。

## 交付后
按项目规矩在 `docs/HANDOFF.md` §3 追加"增量之四十七"，并把"新东西一律写 §3 / 只留最近约 6 条"的老规矩照办。
