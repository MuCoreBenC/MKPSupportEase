# 参数台 v2 · 状态一致性协议（开工前定死）

> 这份文件不是设计稿，是**施工前必须先签字的那几条规则**。
> 新页 `src/workbench/sheet/**` 只要违反其中一条，就算功能"看起来对"也不许合。
>
> 起因：旧参数台不是"某处写错"，而是**值 / 保存值 / 脏判据三件事在同一条链上被反复折叠**
> （见 `docs/WORKBENCH-STATE-REARCH-2026-11-02.md`）。新页唯一的优势就是**这三件事各归各位**，
> 所以每一条隔离都必须写死，不许"顺手复用一下"。

---

## 0. 新旧隔离（作者的硬要求）

> 「新的就是新的，旧的就是旧的，不要混着用。」

### 共享（只读、无状态）

| 共享什么 | 为什么可以 |
|---|---|
| `src/workbench/components/field/**`（数字步进 / 分段 / 开关 / gcode 控件） | 纯受控组件，不持有状态、不知道"脏"是什么 |
| `src/i18n` 基础设施（`t` / `useT` / `Text`） | 全仓唯一出口 |
| `c14.module.css` 的视觉令牌（颜色 / 间距 / 字号） | 保证长相一致 |
| 后端只读能力（`effective_of` / `has_own` / `layout_sections`） | 唯一实现，不许抄第二份 |

### 不共享（新页自己写一份）

| 不共享什么 | 为什么 |
|---|---|
| `ParamsPage.tsx` / `ParamDetail.tsx` / `BatchEdit.tsx` | 旧页冻结，`import` 它等于把旧状态带进来 |
| `editQueue.ts` | 它的 `patchRow` 会直接改乐观层，正是"闪一秒"的来源。新页用 `sheet/writeQueue.ts` |
| `derive::Book::cell` 的 `dirty` / `rest` / `restPinned` | 新页**不消费**这三个字段（见 §2） |

**判据**：`src/workbench/sheet/**` 里出现 `from '../views/` 或 `from '../editQueue'` 即为违规。

---

## 1. `saved` 的定义与来源

```text
saved[key][col]      = 会话 BASE 在这一格的有效值（版本覆盖 → 机型基底 → 出厂）
savedPinned[key][col]= 会话 BASE 在**这一层**是否自己钉着
```

- 唯一来源：后端 `wb_param_sheet` 一次返回，**前端不做任何层运算**。
- BASE 的定义与推进由既有机制保证（`Ctx::base` + `txn::SaveStage` + `recover_save_stage`），
  新页不重新定义它，也不缓存跨会话。
- **重取时机只有四个**：打开页面、`wb_save` 成功、`wb_discard` 成功、`wb_reload` 成功。
  其余时刻**一律不重取** —— 编辑路径重取 `saved` 会让徽章在编辑中途跳变。

### 前置条件（不满足就不许开工验收）

`saved` 的正确性完全建立在 BASE 可信上。以下四条必须已有测试通过：

1. 保存中断后重启：基准被补完（`an_interrupted_save_does_not_look_like_unsaved_work`）
2. 放弃回到打开那一刻（`discard_returns_to_the_opening_snapshot_not_the_current_disk`）
3. 外部改动不许被保存静默覆盖（`saving_over_an_externally_changed_presets_is_refused`）
4. 目录替换不产生半新半旧（`a_crash_between_the_two_renames_finishes_the_replacement` 等 7 条）

**这四条红着的时候，新页任何验收结论都不成立** —— 因为它读到的 `saved` 本身就是错的。

---

## 2. `draft` 的形状与比较

### 键的存在性：`hasOwn`，不用 `undefined` 兼任

```ts
// ✗ 错：null 是合法值时会被当成"没有草稿"
const shown = draft[k] ?? saved[k]

// ✓ 对：键存不存在是独立事实
const hasDraft = (k: string) => Object.hasOwn(draft, k)
const shown = (k: string) => (hasDraft(k) ? draft[k] : saved[k])
```

**为什么**：参数值允许 `null`（可空字段 / 解钉）。用 `??` 会让"用户把它设成 null"
和"用户没动过它"collapse 成同一件事。

### 值的比较：规范化，不用 `!==` / `===`

```ts
/** 参数值的规范化键：数字按数值、数组/对象按键序递归、其余按字面。 */
function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a === 'number' && typeof b === 'number') return a === b || (Number.isNaN(a) && Number.isNaN(b))
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((x, i) => sameValue(x, b[i]))
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const ka = Object.keys(a as object).sort()
    const kb = Object.keys(b as object).sort()
    return ka.length === kb.length
      && ka.every((key, i) => key === kb[i] && sameValue((a as never)[key], (b as never)[key]))
  }
  return false
}
```

**为什么**：`===` 对数组/对象是引用比较。从 IPC 反序列化出来的 `["a","b"]` 与本地构造的
`["a","b"]` 永远不等 ⇒ 徽章会常亮。`74` 与 `74.0` 也必须算同一个（后端是 `float`）。

### 三个纯函数是**全部**口径

```ts
const shown  = (k) => hasDraft(k) ? draft[k] : saved[k]
const dirty  = (k) => hasDraft(k) && !sameValue(draft[k], saved[k])
const badge  = (k) => dirty(k) ? saved[k] : null        // 显示的一定是 saved
```

**别处不许再算一遍**。`SheetRow` / `SheetDetail` / 批量抽屉都从这三个派生。

---

## 3. `revert` 必须同时恢复「值」与「钉状态」

这是审查里指出的第二处。只 `edit(saved)` 不够：

| BASE 状态 | 用户把它改成 | 只比"值"会怎样 | 正确做法 |
|---|---|---|---|
| 没钉（有效值 70） | 钉成 70 | 值相同 ⇒ 判不脏 ⇒ **结构其实变了** | 删键，回到"没钉" |
| 钉着 80 | 改成 90 | 值不同 ⇒ 脏 | 写回 80 |
| 钉着 80 | 删键（变继承 70） | 值不同 ⇒ 脏 | 写回 80 |

所以：

```text
revert(k):
  savedPinned[k] === true  ⇒ edit(k, saved[k])     // 写回那个值
  savedPinned[k] === false ⇒ edit(k, UNSET)        // 删键挂回继承
```

`UNSET` 是一个**显式的哨兵**（`Symbol('unset')` 或 `{ __unset: true }`），
**不是 `undefined`、不是 `null`、不是 `0`**。

> 视频里那次「点 `↩ 70` 变 0」就是这条没守住：`String(cell.rest)` 拿到空值，
> 经数字控件回落成 `0` 并提交。新页的 `edit` **对非值输入直接抛错**，
> 绝不允许"猜一个默认值"。

### 单独 pin/unpin：2026-11-02 已实装，走**独立 patch**，不污染 `draft`

作者原话：「就算改的值相同，也不一定非要跟着默认，也可以是自己的」。
于是有了「固定为当前值」：写一个**与继承值相同**的值，并要求它留在这一层。

关键决定：**它不是一个二维 `draft`，而是一笔独立的 patch**。

```text
Patch::PinValue { level, owner, key, value }   // value: Some(v) 固定成 v；None 取消固定
```

| 层 | 为什么放在那里 |
|---|---|
| 前端 `draft` | **不变，仍是一维**（`{ raw }`）。固定的结果显示由 `Cell.pinned` 给（后端算） |
| 后端 `Draft.pins` | 只在保存前告诉归一化"这几格别删"，保存后草稿清空就没了 |
| 文件 | `machineVariants` 里那一行 + `pinnedVariants` 里那个键（见下） |

#### 为什么文件里还必须多一句 `pinnedVariants`

第一版以为"那一行本身就是证据"，**错了**。§3.3 的归并会把"全体版本一致"的版本键
上提到机型基底，版本层什么都不留 —— 于是"FAST 版固定成 3.9"和"FAST 跟着基底走"
在派生视图里一模一样，`has_own(Version, key)` 为假，界面画回「机型默认」。
用户点完固定，**数据消失了**。

所以每个 `[[params]]` 多一个可选键：

```toml
pinnedVariants = ["A1:FAST"]
```

- 归并（`variants::digest`）看见它就**不许上提**这一个版本键，让它留在版本层；
- 其余没被固定的版本照旧跟着基底（键仍是冗余登记，仍然会被清理）；
- 清空这一格（`clear_variant`）时一并删掉 —— 取消固定之后它就该重新跟着上一层；
- 空清单不写这个键（与空 `machineVariants` 表头同一条规矩）。

**判据 `Cell.pinned` 只认 `pinnedVariants` 里那句话，不拿"值恰好相等"去猜**（都读归并后的这一层）。
第一版 `pinnedSame` 读 registry 原始表，结果每一格都被说成「已固定」，
而右键菜单（读 `own`）同时说「固定为当前值」—— 两个控件互相拆台。

于是 §2 那三个纯函数一个字都没改；`revert` 仍是 `savedPinned ? edit(saved) : edit(UNSET)`。

**验收**：① 固定一个等于出厂值的格子 → 状态标签变「已固定」（紫），右上角「未保存」+1；
② 保存 → 重开 → 那一行还在，标签仍是「已固定」；③ 取消固定 → 行消失、标签回到
「出厂默认」，且**不带**出一条"未保存"。

### 三种状态标签的颜色（作者 2026-11-02：「颜色你自己定」）

| 档 | 颜色 | 意思 |
|---|---|---|
| 出厂默认 | 灰 | 这台机器、这个版本都没写过它 |
| 机型默认 / 本版修改 | 蓝 / 白底描边 | 这一层有自己的值，且值**不等于**上一层 |
| **已固定** | **紫** | 这一层有自己的值，但值和上一层**一样** —— 它在那儿只因为人决定要它 |
| 已修改 | 橙 | 改了还没保存（压过上面所有档） |

紫色是这一屏唯一没被占用的语义色（蓝=主操作、绿=成功、红=弃用、橙=未保存、灰=缺省），
且它天然读作"人做的决定"，不需要额外图例。

---

## 4. 前端 `draft` ↔ 后端工作树：单飞写入队列

审查第三条，也是协议里最硬的一条。

### 权威划分

| 状态 | 权威在哪 | 谁写 |
|---|---|---|
| `saved` | 后端 BASE | 后端（打开 / 保存 / 放弃） |
| `draft`（**显示与徽章**） | **前端** | 用户手势 |
| 编辑树（`presets/` 视角） | 后端 WORK | `wb_apply_draft` |
| 落盘 | 后端 `presets/` | `wb_save` |

**前端 `draft` 是显示的权威，后端 WORK 是保存的权威。两者靠队列对齐，不靠"谁后到听谁的"。**

### 队列规则（`sheet/writeQueue.ts`）

```text
pending: Map<cellKey, { seq, patch }>   // 同一格只保留最新一条
inflight: { seq, cellKey } | null       // 同时最多一条

edit(k, v):
  1. 本地立刻生效（hasDraft / draft / dirty / badge 全部立即正确）
  2. pending.set(k, { seq: ++seq, patch })
  3. pump()

pump():
  if inflight !== null: return           // 单飞：一次只发一条
  const next = 取 pending 里 seq 最小的一条（按发起顺序，避免饿死）
  if none: return
  pending.delete(next.cellKey)
  inflight = next
  await wb_apply_draft(label, [patch], { matrix })
  inflight = null
  pump()                                 // 继续排空

失败:
  - 该格的 draft **回滚到 saved**（不是回滚到"上一次成功值" —— 那个值可能已经过期）
  - 报错给用户（走既有消息通道，带 code）
  - 继续 pump（不因一条失败卡死其它格）
```

**为什么单飞**：审查举的时序（80 发出、90 发出、B 先完成、A 后完成 ⇒ 后端落成 80）
在串行队列下不可能发生 —— 前一发没回来，后一发根本不会发。

**为什么按 seq 最小（不是最新）**：如果用户 A 格改三次、B 格改一次，
按"最新优先"会让 A 的中间态永远发不出去，而 A 格最终值又依赖最后一次 —— 顺序必须是**发起顺序**。

### 保存时必须排空

```text
save():
  1. await drain()          // 等 inflight 结束 + pending 清空
  2. wb_save()
```

`drain()` 有超时（例如 5s）：超时就**拒绝保存并报错**，绝不"带着未确认的编辑去落盘"。

### 放弃时必须取消

```text
discard():
  1. cancelAll()            // 清 pending；inflight 的那条**结果丢弃**
  2. wb_discard()
  3. 用返回的 view 重取 saved 与 draft 清空
```

`inflight` 的响应回来时如果 `cancelAll` 的代数（`epoch`）已变，**直接丢弃**。
否则会出现「放弃之后，一条迟到的写请求又把值写回去」。

---

## 5. Undo 的手势边界

审查第四条：**5 秒 ≠ 一次手势**。

| 控件 | 一次手势 |
|---|---|
| 数字输入框 | `focus` → `blur` 或 `Enter` 为一次（中间不合并） |
| 步进器（上下箭头 / 滚轮） | 一次按下 → 松开为一次；连续按（间隔 < 300ms）合并成一串 |
| 分段选择 / 开关 / 枚举下拉 | 每次选择单独一次 |
| gcode 文本框 | `focus` → `blur` 为一次 |

时间窗口（5s）**只做辅助**：仅当同一格、同一控件、且中间没有 `blur` 时才合并。
**没有手势事件可依的场合（键盘连打）才退回纯时间窗。**

撤销栈在前端（`draft` 的一层层快照）还是后端（`txn`）？

**决定：前端只做「一格一次」的即时撤销入口，全局撤销仍走 `wb_txn_undo`（后端）。**
理由：新页的 `draft` 与后端的 `txn` 必须同源，否则"撤销了但徽章还在"。

> 第一版可以只做 `revert` + 后端 `wb_txn_undo`，**不做**前端多级撤销栈 ——
> 少一个会漂移的状态。

---

## 6. 外部磁盘变化：禁止静默覆盖

| 情形 | 行为 |
|---|---|
| 打开时 WORK 与 BASE 不同 | 已经是既有机制（`settle_shadow` 的 `recovered`），新页只如实显示「有未保存改动」 |
| 会话中途 `presets/` 被外部改 | 后端 `wb_save` 已有守卫（`E_WB_SAVE_STALE_001`），**新页必须把这条错误原样展示**，不许吞掉重试 |
| 外部改过之后用户点保存 | 报错 + 引导「重新加载」；**不许自动 reload 然后继续保存** |

新页**不新增**任何"自动对齐"逻辑 —— 那件事归后端，前端只显示。

---

## 7. 验收判据（新页专属）

按视频里那三个现象逐条反转：

| # | 场景 | 期望 |
|---|---|---|
| V1 | 机型基底列：70 → 80 | 徽章 `↩ 70` 立刻出现；右上角「未保存（1）」 |
| V2 | 继续 80 → 90 | 徽章仍 `↩ 70`（**不许变成 80**） |
| V3 | 切到版本列 | 同一格徽章与机型列**判据一致**（值相同时同显同隐） |
| V4 | 点 `↩ 70` | 值回到 70；**不许出现 0**；徽章消失 |
| V5 | 改回 70 | 徽章消失、右上角归零（不靠保存动作） |
| V6 | 保存 | 徽章全消失；新值成为新 `saved`；再改一次徽章显示的是**新值** |
| V7 | 放弃 | 回到打开那一刻；所有徽章消失 |
| V8 | 快速连改 5 次（跨格） | 最终值与最后一次输入一致（单飞队列不发散） |
| V9 | 写入失败（注入） | 该格回到 `saved` 并报错，其它格不受影响 |
| V10 | 外部改 `presets/` 后保存 | 报 `E_WB_SAVE_STALE_001`，不落盘 |

---

## 8. 实施顺序（协议先于 UI）

1. **协议 + 行为规格表**（本文件 + `useParams` 逐行对照）
2. 后端 `wb_param_sheet`（只读，带 `saved` / `savedPinned`）
3. `useSheet` + `writeQueue`（**先写测试，后写 UI**）—— 三个纯函数与队列是纯逻辑，可以单测
4. `SheetRow` / `ParamSheet` / `SheetDetail`
5. 导航接入（新项独立，旧项不动）
6. 按 V1-V10 回归 + 全套验证集

---

## 附录 · 客户端行为规格表（`src/app/params/`，只读参考）

> 逐行对照 `useParams.ts` / `ParamRow.tsx`。**左列是客户端事实，右列是新页的取舍**。
> 「差异」一列必须写理由 —— 没有理由的差异就是又一个 bug。

| # | 客户端事实（文件:行） | 新页怎么做 | 差异理由 |
|---|---|---|---|
| 1 | `savedValueOf`（`useParams.ts:1632`）：`saved → fileValues → baseByKey → ''` 四级回落 | 后端 `wb_param_sheet` 一次算好 `saved` | 客户端的四级是"编辑对象是本地文件"的历史包袱；工作台有 BASE 快照，前端不该再算层 |
| 2 | `valueOf = draft[key] ?? savedValueOf(key)`（`:1642`） | `hasOwn(draft,k) ? draft[k] : saved[k]` | 客户端值是 `string`，`??` 安全；工作台值可为 `null` |
| 3 | `dirtyOf = draft[key] !== undefined`（`:1648`） | `hasOwn && !sameValue(draft[k], saved[k])` | 客户端 `dirty` 判的是"有没有草稿"，不是"值变没变"；工作台必须判值（否则"改回原值"仍判脏） |
| 4 | `factoryOf`（`:1646`）= 出厂值，独立一档 | **不提供** | 新页第一版不做「恢复出厂」，只做「恢复保存值」。少一档状态 |
| 5 | 徽章渲染条件 `dirty \|\| badTyping`（`ParamRow.tsx:399`） | 只 `dirty` | `badTyping` 是"正在打字打错"，与"改过没有"无关；新页把输入校验放在控件内部，不污染徽章 |
| 6 | 徽章文本 `valueText(def, savedValue, tr)`（`:405`） | 直接把 `saved` 交给控件的格式化函数 | 同一个格式化器，不自己拼字符串 |
| 7 | `revertToSaved`（`:700`） | `savedPinned ? edit(saved) : edit(UNSET)` | 客户端没有"钉/不钉"结构（它写的是文件值），工作台必须恢复钉状态 |
| 8 | 保存 = `commitPresetDraft`（`:1076`） | `wb_save`（已有阶段机） | 工作台的保存要落 `presets/`，且要排空写入队列 |
| 9 | 客户端无「放弃到打开那一刻」 | `wb_discard` | 工作台特有（BASE/WORK 模型） |
| 10 | 客户端无统一撤销栈 | `wb_txn_undo`（后端） | 同上 |

**这张表不是"抄一遍"，是"逐条判定能不能抄"** —— 第 3、7 两条是必须改的，
第 5 条是必须删的，其余是同构的。

