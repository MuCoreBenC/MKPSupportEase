# 参数台 v2 · 交付记录（2026-11-02）

> 规则书：`docs/WORKBENCH-SHEET-V2-PROTOCOL.md`。病历：`docs/WORKBENCH-STATE-REARCH-2026-11-02.md`。
> 本文只记「这一版最后做成了什么 / 怎么验 / 还差什么」。

---

## 0. 一句话

**界面与旧参数台一模一样，内核换掉。**

作者的两次裁定拼在一起才是最终口径：

1. 「不要再去修了，现在一点都不要动它。去做新的……」（→ 新建页，旧页冻结）
2. 「你把新的前端页面改成和旧的一样……逻辑也是用新的，只是前端页面看起来不变」
   （→ **新页照抄旧页的版式与操作**，只有数据与写入是新的）

第一版把新页做成了一个"简化版"（左树 + 卡片网格 + 右栏），缺了页签、状态列、
分组折叠、右键菜单、对照模式、批量、分组抽屉、定义框 —— 作者当场否掉：
「你这都改了好多功能都缺失了呀」。

---

## 1. 版式从哪来（这一版的关键决定）

```
src/workbench/views/ParamsPage.tsx   ──整份复制──▶   src/workbench/sheet/ParamSheetPage.tsx
```

复制而不是"照着写一个像的"：

- 2389 行里绝大多数是版式与交互细节（左树三种形态、页签条、分组卡、行内缩进、
  徽章位置、右栏、抽屉、右键菜单、对照矩阵…）。照着写必然漏，漏一处就是
  「功能缺失」。
- 复制之后**只改 4 处**（见 §2），其余一字不动。
- 旧页此后**一个字节都不动**，仍可随时对照。

跨目录 import 是故意的：`sheet/ParamSheetPage.tsx` 用 `../views/CellEditor`、
`../views/ParamDetail`、`../views/BatchEdit` 等**只读组件**。协议 §0 那条
「不许出现 `from '../views/`」只对"另一套状态模型"成立，这一版恰恰是
**同一套 UI、换掉状态**，所以不适用。

---

## 2. 换掉的 4 处（这就是"新内核"）

| # | 旧页 | 新页 |
|---|---|---|
| 1 | `editQueue`（模块级乐观层，落地也不撤） | `sheet/draftOverlay.ts` 的草稿层，**后端跟上就删** |
| 2 | `editQueue.push(...)` + 120ms 尾随合批 | `sheet/writeQueue.ts` 的 **单飞队列**（一次只发一条） |
| 3 | 徽章 `String(cell.rest)` | `rawOf(cell.rest)`（`null` → `''`，**不是 `"null"`**） |
| 4 | 保存前只 `settleAll()`（旧队列） | `sheetWrites.settle()`（新队列也排空）+ 放弃/撤销/重做后 `drop()` |

### 2.1 草稿层为什么这样设计

```text
显示 = 草稿有这一格 ? 草稿的原文 : 后端那份（cell.raw）
脏   = 草稿里这一格「还没被后端跟上」
```

旧 `editQueue` 的条目**落地之后也不撤**（它同时兼任"本地草稿层"），于是
「我改过，后端还没跟上」与「我改过，后端已经跟上了」分不开 —— 徽章要等
保存/放弃才灭，而 `patchRow` 又把 `dirty` 强行置 true。

新草稿层多一条 `clean`：

| 用户做了什么 | 草稿记什么 | 显示 / 徽章 |
|---|---|---|
| 改成别的值 | `{raw: 新值, clean: false}` | 立刻显示新值、徽章亮 |
| 改回保存值 | `{raw: 保存值, clean: true}` | **立刻**显示保存值、徽章灭（不等后端） |
| 后端把这些写回来了 | 整条删掉（`settleDraft`） | 显示交回后端那份，值不变 |

`settleDraft` 的判据是 `cell.raw === 草稿.raw && cell.dirty === !clean` ——
**只删确认对上的**，其余原样留着，所以不会出现"后端还没跟上就闪回旧值"。

### 2.2 单飞队列

旧队列是"120ms 尾随合批 + 一次发一批"。合批本身没问题，但它把"同一格连点"
与"跨格并发"混在一套 seq 里；新队列一次只放**一条**出去，前一条没回来后一条
根本不发 —— 落盘顺序恒等于发起顺序，不可能被旧值覆盖。

`sheetWrites`（`writeQueue.ts` 末尾）是它挂到外壳上的那一条线：

```text
App.save()     → sheetWrites.settle()   排不空就拒绝保存（不带着未确认的编辑去落盘）
App.discard()  → settle() + drop()      放弃之后草稿作废
App.undo/redo  → drop()                 后端把那一笔撤了，草稿层不再是真话
```

**不接管页头那一槽** —— 接管会把页头换成「已写进仓库 / 放弃 / 保存」三件
（作者截图里那一版），而旧参数台的页头是「查看改动」一颗。挂后台钩子，
外观一个字不动。

### 2.3 「点 ↩ 变 0」

`ParamLine` 里原来是 `cell.restPinned ? String(cell.rest) : null`。
`rest === null`（这一层没钉）而 `restPinned === true`（理论上不该同时发生，
但历史数据里出现过）时 `String(null)` 得到 `"null"`，数字控件解析失败回落成
`0` 并提交 —— 这就是视频里那一幕。现在：

```ts
cell.restPinned && cell.rest !== null ? rawOf(cell.rest) : null
```

`rawOf(null) === ''`，且 `writeValue` 收到 `null` = **删键挂回继承**，不猜默认值。

---

## 3. 后端

| 东西 | 状态 |
|---|---|
| `Cell.rest` / `Cell.restPinned` | 刀 1（`39cf88c`）+ `3c9c447` 之后 = **BASE 的三层有效值**，正是 `saved` 该有的语义 |
| `wb_param_sheet`（`app/sheet.rs`） | 已就绪：只读、不返回当前值、`saved`/`savedPinned`/`savedText`/`level`/`savedOrigin`，2 条判据钉着。**当前页面没吃它**（走 `wb_desk` 的 `Cell.rest`，同一条口径），留给后续把一屏数据收成一次请求 |
| `lib.rs` 注册 + 只读白名单 | 已加 |

这一版没有新增/修改任何"值与脏"的算法：那条链在刀 1–3 已经修完，
本版只是让**界面按新口径用它们**。

---

## 4. 文件清单

```text
src/workbench/sheet/ParamSheetPage.tsx   [新增] 由 ParamsPage.tsx 整份复制，改 4 处
src/workbench/sheet/draftOverlay.ts      [新增] 草稿层：rawOf / savedRawOf / applyDraftToDesk / settleDraft
src/workbench/sheet/writeQueue.ts        [新增] 单飞写入队列 + 挂外壳的 sheetWrites
src/workbench/sheet/sheet.test.ts        [新增] 14 条纯逻辑判据
src/workbench/App.tsx                    [改]   导航加「参数台（新）」；save/discard/undo/redo 接 sheetWrites
src/workbench/c14/types.ts               [改]   PageId 加 'params2'
src/workbench/api.ts                     [改]   Sheet* 类型与 wb.paramSheet（后端契约的前端镜像）
src-tauri/src/workbench/app/sheet.rs     [新增] wb_param_sheet（见 §3）
src-tauri/src/workbench/app/mod.rs       [改]   pub mod sheet; + 命令 + 2 条判据
src-tauri/src/lib.rs                     [改]   注册 + 只读白名单
docs/WORKBENCH-SHEET-V2-PROTOCOL.md      [新增] 规则书
docs/WORKBENCH-PARAM-SHEET-V2.md         [新增] 本文件
```

旧的 `ParamsPage.tsx` / `ParamDetail.tsx` / `BatchEdit.tsx` / `editQueue.ts` / `derive.rs`
**一个字节没动**。

---

## 5. 验收对照表

| # | 场景 | 期望 | 现在靠什么 |
|---|---|---|---|
| V1 | 机型基底列 70 → 80 | 徽章 `↩ 70` 立刻出现 | 草稿层置 `dirty`（前端即刻）+ 后端 `dirty_of` 比有效值 |
| V2 | 继续 80 → 90 | 徽章仍 `↩ 70` | 徽章文本 = `cell.rest`（保存值），与当前值无关 |
| V3 | 切到版本列 | 同一格判据一致 | `Cell.rest` 三层算完，与"哪一层钉着"无关（`3c9c447`） |
| V4 | 点 `↩ 70` | 回 70，**不许出现 0** | §2.3；单测 `rawOf(null) === ''` |
| V5 | 改回 70 | 徽章立刻消失 | 草稿 `clean: true`（不必等保存） |
| V6 | 保存 | 徽章全消失，新值成为新 saved | `shellWrites.settle()` → `wb_save` → `drop()` |
| V7 | 放弃 | 回打开那一刻 | `wb_discard`（刀 3）+ `drop()` |
| V8 | 快速连改 5 次（跨格） | 最终值与最后输入一致 | 单飞队列（单测：同一时刻只有一条在飞） |
| V9 | 写入失败 | 该格退回保存值并报错 | `QueueEntry` 失败 → `onFail` 删该格草稿 + toast |
| V10 | 外部改后保存 | `E_WB_SAVE_STALE_001` | 后端守卫（刀 6），前端原样展示 |

界面部分**仍需作者真机过一遍**（本轮没有起 GUI）。

---

## 6. 这一版没做的

| 没做 | 说明 |
|---|---|
| 把 `wb_desk` 换成 `wb_param_sheet` | 目前 `desk` 一份数据够用（`rest` 已是 BASE 有效值）。切过去能把"值 + 结构"收成一次请求，但会牵动对照矩阵与抽屉，留到下一轮 |
| 前端多级撤销栈 | 撤销仍走后端 `wb_txn_undo`（状态栏那两颗） |
| pin/unpin 独立编辑 | 仍只有 `revert` 一条路能改钉状态 |
