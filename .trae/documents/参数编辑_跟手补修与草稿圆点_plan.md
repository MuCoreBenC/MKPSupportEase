# 参数编辑两处修复：① 工作台"跟手化"补修 ② 客户端草稿"橙点/记不住"

> 说明：作者附的录屏是 `.mp4`，我这边**读不了视频**（工具只支持图片/文本，本机也没有 ffmpeg）。
> 下面 Part A 的结论是依据作者的文字描述（"跳到的是正确的值，过程不好看、不跟手；0.1 点一下该变 0.05，
> 可它先像是 0.15 才变 0.05"）+ 代码走查定位出来的，不是猜测。

---

## Context

两件事，按作者要求的顺序：**先修工作台那个没修好的，再修客户端这个新的**。

- **A（工作台）**：`npm run tauri:workbench:dev` 里连点数字步进器，**最终值是对的，但过程不跟手** ——
  数字会先回落/跳到别的值，再落到正确值；看起来像"点了没反应"。
- **B（客户端）**：预设页右键「修改参数」→ 进编辑模式 → 改了几处 → **不保存直接关掉**。
  期望：下次进来还是编辑模式、改动还看得见；**外面那张表那一行上有一枚橙色「未保存」圆点**。
  实际：那枚橙点不亮，重开也看不到上次的改动。

---

## Part A 工作台：显示在"在飞那一趟"里掉回旧值

### Current State（根因，已定位到行）

`src/workbench/editQueue.ts` 的 `pump()` **在发出 IPC 之前就把这一批从队列里删掉了**
（`for (const e of batch) queue.delete(e.cellKey); rebuild()`）。于是：

1. 点击 → `NumberField` 本地值立刻变 → `commit()` → `push` → 乐观覆盖层生效 → 屏幕**立刻**显示新值 ✅
2. 120ms 尾随到点 → `pump()` **把队列条目删掉** → 覆盖层消失
3. **IPC 在飞的那几十~几百毫秒里**，`shownDesk = patchDesk(desk, 空 overlay) = base`（旧值）
   → `NumberField` 的 `value` prop 变回旧值 → `[value]` 回写 effect 把屏上数字**拽回旧值**
4. 响应回来 → `setDesk(out.desk)` → 才跳到正确值

⇒ 屏幕上就是"新值 → 旧值 → 新值"的跳，正是作者描述的"0.15 → 0.05"。

**为什么上一轮的探针没抓住**：假后端的 `wb_apply_draft` 几乎瞬时返回（`mock.ts:2151` 同步 resolve），
第 3 步那个窗口宽度≈0；而探针只在"停手那一刻"和"400ms 之后"两处采样，两点正好都落在正确的一侧。
真机（Tauri）往返几十~几百毫秒，窗口肉眼可见。

### 改法（只动 `editQueue.ts`）

**批次落地后才从队列里删**（发出去的时候不删）：

```ts
// pump(): 取一组 → 记进 inFlight（条目仍留在 queue 里，覆盖层照旧生效）→ await send → 落地后再收
function pump(): Promise<void> {
  if (active !== null) return active
  const all = [...queue.values()]
  if (all.length === 0) return Promise.resolve()
  const key = all[0].refreshKey
  const batch = all.filter((e) => e.refreshKey === key)
  const send = sender
  active = (async () => {
    try {
      if (send !== null) await send(batch)
    } catch { /* 同下 */ }
    finally { active = null }
    settle(batch)              // ← 落地后才收；同键被更新的那条**留着**，交给下一趟
    if (queue.size > 0) schedule()
  })()
  return active
}

/** 这一批有了结果（成功落进 base / 失败回落）—— 把覆盖层收掉。
 *  按 `queue.get(cellKey) === e` 判：飞入期间又被点过的那一格留着新值，不会被这趟收走。 */
function settle(batch: EditEntry[]): void {
  let touched = false
  for (const e of batch) if (queue.get(e.cellKey) === e) { queue.delete(e.cellKey); touched = true }
  if (touched) rebuild()
}
```

- 失败也走 `settle`：显示回落 base 才是对的（错误横幅由外壳 `run` 说）。
- `settleAll()` 的循环条件（`active !== null || queue.size > 0`）不用改 —— 条目改成"落地后才删"，循环照样收敛。
- 附带好处：**飞在空中的那一趟也还在 `snapshot()` 里**，所以行上的橙色「改之前的值」chip 与
  `dirty` 徽章在往返期间不会闪灭。

### 验证（把这次的病**做成判据**）

强化 `scripts/probes/params-lag.mjs`：
1. 包一层 `invoke` 时，给 `wb_apply_draft` **人为加 ~300ms 延迟**（模拟真机往返；
   不这么干，假后端太快，这条病永远抓不住）；
2. 连点期间 + 落库之后，每 ~20ms 采一次 `input.value`，断言**单调不倒退**（本次的病因就是它会倒退一次）；
3. 原有 5 条（跟手 +10 / `apply_draft=1` / `desk=registry=preflight=0` / 橙 chip / chip 回退）保持全绿。

---

## Part B 客户端：草稿"看不见 / 圆点不亮"

### Current State（三处，都已定位到行）

**B1 —— 橙点根本不亮（主因）。**
`src/app/state/draftState.ts:23-25` 的头注写着"发：参数页（`useParams`：改值 / 保存 / 取消 / 采用云端新值）"，
但 `git grep draftMutated` 的实际调用者只有 **`usePresetData.ts:955` 与 `:974`**（正文链的保存 / 放弃）。
参数链**一次都不广播** ⇒ 预设页那支 `usePresetData.ts:806-815`（`useDraftRevision` → 读 `getPresetDraftSummary`）
从不重跑 ⇒ 行上那枚 `unsavedDot`（`PresetTable.tsx:620-627`，`background: var(--warn)`，橙）
只能等整表重读（重开机型 / 重载）才可能亮 —— 这就是"橙点不显示"。

**B2 —— 判据漏了 `dirty`。**
`usePresetData.ts:681` 与 `:811` 都是 `setDraftPath(d?.path ?? null)`，**没看 `dirty`**。
而 `begin_preset_edit`（`src-tauri/src/ipc/local.rs:262-266,313`）一打开就 `set_draft`（写出草稿文件）
⇒ "只打开过、一个字没改"也会命中 `draftPath` ⇒ 一旦 B1 接上，橙点会**误亮**。
契约与头注都写的是"**真改过才算**"（`contract.ts:1026` / `presetTree.ts:1546`）。

**B3 —— 重开看不到改动、也没回到编辑模式。**
- `useParams.ts:1894-1904` 拿 `beginPresetEdit` 的返回值时**只用了 fileName/origin/path**，
  把 `reused` 丢了、把 `d.text`（草稿正文，**含没保存的改动**）**一次都没用**；
- `valueOf = draft[key] ?? savedValueOf(key)`（`:1230-1233`），而 `savedValueOf` 对着**盘上那份文件**
  （`readPresetParams` → `read_preset_params` → `runtime::local::read_text`，`ipc/preset_params.rs:65`）；
- 内存 `draft` 从 `{}` 起（`:720`），只在 `save`（`:1593`）/`switchTo`（`:1649`）清，**从不 seed**
⇒ 重开时显示的是**文件的旧值**，自己的改动**看不见**（而草稿还在盘上，点保存会把它一起存下去）。
另外 `PageParams.tsx:403` `useState(gateEdit !== true || openSyncOnStart)` ⇒ 重开永远先只读。

**兄弟链已经这么做了**（这就是本仓的"房子风格"）：`PagePresets.tsx:1337-1366`（「编辑文件」抽屉）
`const unsaved = draft.reused && draft.dirty` → `setEditMode(unsaved)`，并引了作者 2026-10-16 的原话
「下次打开这个模态框，它里面还是显示编辑模式…… 好的提示、或者通知，就是说刚才那些修改还没保存、没生效」；
它还用一个模块级 `textHistCache` 把**撤销/重做栈按 path 记着**跨开关保留。

### 改法（**纯前端，不动 Rust**）

**B1**：`useParams` 在草稿真变过的地方发广播 ——
- 每个改动落进草稿后（`:864` 的 `patchPresetDraft` 那一路，含撤销/重做/批量还原/采用云端新值）；
- `commitPresetDraft` 成功之后（`:800`，草稿被清 ⇒ 橙点灭）；
- 作废（`discardAll` / 切 combo 清草稿，`:1649`）之后。
为避免连点刷 IPC，加一个**尾随 150ms 合并**的小助手 `broadcastDraftSoon()`（同一批改动合一次），
但 **保存 / 作废 / 采用云端新值这三处立刻发**（橙点必须当场灭）。

**B2**：`usePresetData.ts` 两处都改成 `setDraftPath(d !== null && d.dirty ? (d.path ?? null) : null)`。
（云端线 `path` 仍是 `null` ⇒ 只有本地表的「本地文件」行会亮，与 `presetTree.ts:2096` 的判据一致，别改。）

**B3a（重开回到编辑模式）**：`useParams` 暴露 `draftOnDisk: boolean`（= 盘上这份草稿"真改过"）：
`beginPresetEdit` 之后**再读一眼摘要**（`api.getPresetDraftSummary()`，轻命令），
按 `origin` + `path`/`sourceFileName` 对上编辑目标且 `dirty === true` 才置真。
`PageParams.tsx` 加一个一次性 effect：`draftOnDisk` 为真 → `setEditMode(true)`（只自动进一次，
用户点了「放弃并退出」之后不再自动进 —— 用 ref 记住已消费）。

**B3b（重开看得见改动 + 留住撤销/重做）**：照 `textHistCache` 抄一个**会话缓存**（`src/app/params/paramSessionCache.ts`，新）：
- 模块级 `Map<targetKey, {head, draft, past, future, log, seq}>`（`head` = 离开时草稿正文的指纹）；
- `useParams` 卸载（模态框关闭）时写进去；`beginPresetEdit` 回来后，若 `cache.head === d.text`
  （与 `textHistCache` **同一条有效性判据**）就把 `draft/past/future/log/seq` 复原；
  对不上就整条丢掉（草稿被别处保存/作废过 ⇒ 旧快照不能再接）。
- 效果：同一运行内关掉再进来，**改动还在、撤销重做还在**，与兄弟链一致。
- **明确记下的边界**：换台机器/重启应用后，盘上草稿还在（橙点亮着提醒你），但这一屏显示的是文件值 ——
  与「编辑文件」抽屉同一条口径（那一条靠 `d.text` 直接当 UI 状态，所以不受影响）。

**本轮明确不做**（记在案，避免越改越大）：
- 橙点对**云端行**也亮（现在是本地行专属判据，作者这次改的是 `presets-local/` 那份）；
- 跨进程重启后把草稿值灌回这一屏（需要后端把草稿正文抽成值表，属另一刀）。

---

## 改动文件

| 文件 | 改什么 |
| --- | --- |
| `src/workbench/editQueue.ts` | `pump` 不再提前删；新增 `settle(batch)`（落地后收；同键被更新的留着） |
| `scripts/probes/params-lag.mjs` | 给 `wb_apply_draft` 加人为延迟 + 全程采样断言"单调不倒退" |
| `src/app/params/useParams.ts` | B1 广播（含 `broadcastDraftSoon`）；B3a 暴露 `draftOnDisk`；B3b 读写会话缓存 |
| `src/app/params/paramSessionCache.ts` | **新**：会话缓存（照 `PagePresets` 的 `textHistCache` 抄） |
| `src/app/params/PageParams.tsx` | B3a：`draftOnDisk` 为真时自动 `setEditMode(true)`（只一次） |
| `src/app/presets/usePresetData.ts` | B2：两处 `setDraftPath` 加 `dirty` 判据 |

## 验证

Part A：
```
npx tsc -b --force && npm run lint
node scripts/probes/params-lag.mjs        # 6 条全绿（含新增的"不倒退"一条）
```
手动：`npm run tauri:workbench:dev` → 连点步进器 10 下：数字**只往前走、不回跳**，停手即到位；
长按松手只进 1 条撤销；Ctrl+Z 一次退一格。

Part B：
```
npx tsc -b --force && npm run lint && npm run build && npm run check:bundle
```
手动（客户端）：右键一份本地预设 →「修改参数」→ 进编辑模式 → 改 2 处 → **不保存直接关掉**
→ 该行出现**橙色圆点** → 重新打开：**直接是编辑模式、两处改动还在、撤销重做还能用**
→ 保存 → 橙点**当场灭**；再确认"只打开不动就关掉"**不**亮橙点。

## 风险 / 回退

- A：`settle` 的 `queue.get(cellKey) === e` 是唯一防线 —— 写错会让"飞入期间又点的那一格"被旧趟收走，
  又出现回跳。探针第 2 条（单调不倒退）就是守它的。
- B1：广播会让预设页每次改动多读一次**轻摘要**；已用 150ms 尾随合并压住。若真机上仍觉得吵，
  退路是把尾随放到 300ms（橙点亮得晚一点，但改动的"记不住感"不受影响）。
- B3b：会话缓存的有效性只看"离开时的草稿正文 == 现在的草稿正文"。`patch_preset_draft` 每次都会改盘上草稿，
  所以**只有本页自己改**才可能对得上；别处改了正文草稿 ⇒ 自动丢弃（与 `textHistCache` 同一条纪律）。
- 全部是前端改动，**Rust 一行不动**，`cargo` 那几道门不受影响（照旧口径仍应各跑一遍）。

## 交付后
按项目规矩在 `docs/HANDOFF.md` §3 追加「增量之四十八」。
