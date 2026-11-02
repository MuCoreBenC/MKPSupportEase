# HANDOFF 归档（2026-11-01 清出的旧内容）

> ## 这是什么
>
> `HANDOFF.md` 2026-11-01 第三次清理时**搬出来**的增量账，**一个字没删**，只是不再住在活台账里：
>
> - **增量之四十五 ~ 五十八**（2026-10-19 ~ 10-29）—— 撤后处理预览 / 工作台入口 / 参数编辑跟手化 /
>   预设页六件事 / 云端更新链路 / 全站旧值记账 / 拖拽露白 / i18n 四刀 / 管线明细存事实 /
>   发布闸把「没有要发的东西」说出来。
>
> **现行规则不在这里**：纪律 / 判据 / 仓库状态 / 怎么验证，以及**未结账那一张表**、最近的几条增量，
> 都在 [`../HANDOFF.md`](../HANDOFF.md)。更早的两批在
> [`HANDOFF-ARCHIVE-2026-10-19.md`](HANDOFF-ARCHIVE-2026-10-19.md) /
> [`HANDOFF-ARCHIVE-2026-10-20.md`](HANDOFF-ARCHIVE-2026-10-20.md)。
>
> ★ 归档件**只读、不追加**：新东西写进 `HANDOFF.md`；等它又长了，再从它头顶搬下一批成一份新的归档。
> ★ 搬走的段里凡有"待验收 / 未做 / 待拍板"的，都已经抄进 `HANDOFF.md` §2.4 —— 找活干看那一张表，不用翻这里。
> ★ 内文里的相对链接（`docs/…`、`presets/…`、`scripts/…`）是按 `HANDOFF.md` 的位置写的，
>   从 `docs/` 这一层点不进，按仓库根去看。

---

## 增量之四十五 · 撤掉后处理「预览」整组（2026-10-19，分支 `feat/dark-mode`）

> 作者：「把预览全部取消吧」（截图为证：开发台里那 15 格预览还在，而矩阵那一块当时跑的是
> 旧的工作台进程，所以看不到校准两档 —— 代码里其实已经有了）。
> 前置条件已满足：校准件两档真入口做通了（增量之四十四）。

**撤掉的九格 + 两处前端钩子**（都在 `devsim::scenes()` 与两处前端）：

| 撤掉的预览格 | 真入口里怎么看到 |
| --- | --- |
| 成功 / 失败 / 已停止 / 机型不匹配那一问 | 矩阵 `small-ok` / `big-ok` / `hard-fail` / `mismatch` |
| 那一问 · 等超时 | 矩阵 `mismatch` 不理它，两分钟自己收场 |
| 那一问 · 点下去已经晚了 | 真机就会遇到；报错那一屏现在有「停止并关闭」 |
| 跑着 · 收进后台 | 真跑起来点那颗「后台运行」 |
| 校准件 · 问怎么跑 / 等超时 | 矩阵 `calib-ok` / `calib-question`（真 3MF 切片件） |
| 校准件 · 先打印成功 / 直接校准成功 | 矩阵 `calib-ok` 答完就是那两屏 |
| 碰撞 · 问要不要挪塔 | 矩阵 `collision` 档（真撞） |

**唯一留下的**：`postprocess-running`（处理中·停住不动）—— 当时理由是它**没有真实等价物**
（真跑那一屏会自己往前跑）。

### 追加（同日）：这一格也撤了

作者看完截图说「这个不需要了吧」⇒ **后处理那一组一格都不剩**。
定格的需求改由矩阵 `big-ok`（8.5 MB / 约 8 秒）或直接对真跑那一屏截图承担，
不再维护一条只为了"停住"的演出路径。

判据再次反转：`the_postprocess_group_is_gone_from_the_dev_desk`（`pp.is_empty()`）。
演出引擎（`crate::devsim` 的 `Mode` / `Options` / `drive`）**一行没拆** ——
假钩子 CLI（`src/bin/hook-sim.rs`）还在用，撤的只是"开发台上架的那份清单"。

**配套清理**：`src/app/dev/scene.ts` 层类表只剩那一格；
`PostProcessModal` 撤掉 `useDevPreviewNonce('postprocess-answer-stale' | 'postprocess-backgrounded')`
两处开发台钩子（`setErr` / `dismiss` 本身是产品行为，**没动**）。

**判据反过来了**：`the_postprocess_group_is_keep_only_the_frozen_running_frame`
从"四档都在"改成"四档**都不在**、只剩一格" —— 防止以后有人顺手把某一格加回来"方便调试"。

**教训（给下次的自己和用户）**：改完开发台/矩阵**要重启工作台**（`npm run tauri:workbench:dev`）
才看得见新档位；工作台用的是 `target-workbench/` 那套产物，不随客户端的 `target/debug/` 更新。

**验证**：fmt / 双档 `clippy -D warnings` / `cargo test` /
`cargo test -p mkp-support-ease --lib --features workbench` **989 全绿** /
`npx tsc -b --force` / `npm run lint` / `npm run build` ×2 全绿。

## 增量之四十六 · 补 `tauri:workbench` 入口：`npm run tauri:workbench dev` 现在能用了（2026-10-19，分支 `feat/dark-mode`）

**症状**：`npm run tauri:workbench dev` 报 `npm error Missing script: "tauri:workbench"`。

**根因**（不是设备问题，任何机器一样）：npm 的 `npm run <script> [args]` 里，
**脚本名之后的一律当参数透传**（本次已用 `npm run tauri info` 实证 —— 跑的就是 `tauri info`）。
仓库里 `tauri` 这个名字**存在**（`"tauri": "tauri"`），所以 `npm run tauri dev` 能用；
但工作台**从来没有** `tauri:workbench` 这个脚本 —— 它的入口一直叫 `tauri:workbench:dev`
（`npm run tauri:workbench:dev`）。即"照 `npm run tauri dev` 的形状写"少写了一位。

**修法**（只加、不动既有）：`package.json` 补一条
`"tauri:workbench": "node scripts/tauri-workbench.mjs"`。此后两种写法都成立 ——
`npm run tauri:workbench dev` / `npm run tauri:workbench build`（与 `npm run tauri dev` 对称）
和原有的 `npm run tauri:workbench:dev` / `:build` 都照旧，既有习惯不受影响。

**验证**：`npm run tauri:workbench` 命中脚本并打印用法（退出码 2，符合脚本预期）；
参数透传已由 `npm run tauri info` 实证，故 `... dev` / `... build` 会分别转发 `dev` / `build`。

## 增量之四十七 · 参数编辑「跟手化」：本地即时 + 单飞合批 + 橙色「改之前的值」（2026-10-19，分支 `feat/dark-mode`）

**症状**（作者实测）：「改个参数它一直在跳 —— 很慢很卡；我连续点击，停下之后它才慢慢追上我点击的次数。」

**根因**：一次值写入 = `ParamsPage.writeValue` → 外壳 `run()` → `await wb.applyDraft(...)`，紧接着 `setTick(n + 1)` 与 `refreshReport()`。
`tick` 又让 `ParamsPage` 三个 effect 各再发一条 IPC（`wb_registry` / **整页 `wb_desk`** / `wb_matrix`），而 Rust 侧这些 `#[tauri::command]`
**全是同步的（跑主线程）**，`wb_preflight` 更重（parse recipes + 摸盘建 catalog）。
连点步进器时主线程被这一串命令占死 ⇒ 数字不跟手、停下才"追账"。

**改法**（前端为主，**后端草稿那条架构一行没动**）：

1. **`src/workbench/editQueue.ts`（新）** —— 值写入的单飞合批队列 + 乐观覆盖层。
   同一格 latest-wins；**真尾随 120ms**（每次 push 重新计时，人还在连点就一条都不发）；**单飞**（同一时刻只有一条 `applyDraft` 在飞）；
   响应**只写 base**、overlay 永远最后叠；回来先比 `refreshKey`，对不上（切了机型 / 改了搜索）就是过期派生，丢弃。
2. **外壳 `run(..., { quiet: true })`**（`App.tsx`）—— 值写入那一路**不拨 `tick`、不置 `busy`**；`refreshReport` 改 `refreshReportSoon`（700ms 尾随去抖）。
   `undo` / `redo` / `save` / `discard` 仍是低频结构操作、照旧拨 `tick`，但动手前先 `await settleAll()` 把飞在空中的写落完
   （`save` / `discard` 另读 `bookRef` 判脏 —— 否则"刚点完步进器马上点保存"会**静默丢一笔**）。
3. **参数台吃 `out.desk` / `out.matrix`** —— 后端 `wb_apply_draft` 本来就按 `refresh` 顺带返回（以前只吃了 matrix），省掉事后再问一次。
4. **`NumberField`** 两条：手势进行中不回写 prop（`gestureRef`）；本地跑在 prop 前面时也不回写（`aheadRef`）。
   少了这两条，**连点 10 下只进 8**（被动的 `[value]` 回写 effect 会晚一步落在两次点击之间，把 `numRef` 拽回上一步 ⇒ 下一次 `commit()` 当"没变"不提交）。
5. **橙色「改之前的值」** —— `cell.dirty && cell.rest !== null` 时，中间行与右栏「各版本取值」各画一枚 `↩ <改前那个数>`（新增 `.pRevert`，
   复用两边同源的 `--warn-*` token），点了写回 `Cell.rest`（盘上钉着的那个）。与右栏那枚 `↶`（挂回继承、写 `null` 删键）**语义不同**，title 里说清。
6. **其余页** —— `AssetsPage.applyVis` 改走 quiet（它写完自己就 `load()`，再吃一次 `tick` 只是白读第二遍）；
   `BundlesPage` / `BuildPage` / `MachinesPage` 的 `tick` 只在生成 / 保存 / 放弃 / 建删版本等**结构操作**上动，语义正确 —— **不动**。

**验证**（全绿）：
- `npx tsc -b --force` / `npm run lint`（eslint + stylelint）/ `npm run build` / `npm run build:workbench` / `npm run check:bundle` / `cargo fmt --check`；
  Rust 一行没改，故 clippy / cargo test 不受影响（照旧口径仍应各跑一遍）。
- **新探针 `scripts/probes/params-lag.mjs`**（连点 10 下，起 `npm run dev:workbench` 后 `node scripts/probes/params-lag.mjs`）：
  **跟手 +10** · `wb_apply_draft = 1` · `wb_desk = 0` · `wb_registry = 0` · `wb_preflight = 0` · 橙色 chip `↩ 45` 出现 · 点它回 45 且 chip 消失 —— **7 条全 PASS**。
- 手动（`npm run tauri:workbench:dev`）：连点步进器跟手不回跳、长按松手只进 1 条撤销、Ctrl+Z 一次退一格、chip 点一下回干净态。

## 增量之四十八 · 跟手化补修（真机回跳）+ 客户端草稿「圆点不亮 / 重开记不住」（2026-10-19，分支 `feat/dark-mode`）

### 一、工作台：显示在"在飞那一趟"里掉回旧值（增量之四十七的第二刀）

**症状**（作者录屏 + 文字）：「最后跳到的是正确的值，只不过过程不好看、不跟手 —— 从 0.1 点一下该变 0.05，可它先像是 0.15 才变成 0.05。」

**根因**（已定位到行）：`editQueue.pump()` **在发出之前就把这一批从队列里删掉** ⇒ 乐观覆盖层消失 ⇒
IPC 在飞的那几十~几百毫秒里 `shownDesk` 退回 base（旧值）⇒ 屏上数字先跳回旧值、响应回来才跳到正确值。
假后端的 `wb_apply_draft` 几乎瞬时返回，所以第一版探针漏掉了这个窗口。

**第一刀**：`pump` 只挑不删；批次**落地之后**才收（否则 IPC 在飞的那几十~几百毫秒里显示就退回旧值）。

**第二刀（作者 18:43 录屏抓到的那一帧）**：把数值框单独抽帧看（12 帧 @ 1/60s）抓到
**`0.2 → 0.15 → 0.2`** —— 数字只在中间**一帧**变成别的值。根因：**显示值仍然被后端返回值牵着走**。
第一刀只是把"收"挪到落地之后，落地那一刻本地层照样消失 ⇒ 显示落回后端那份 desk（手上还是上一次的值）。
**客户端不闪的真正原因**：它的显示**只认自己那份本地 `draft` 层，后端往返从不回写显示**
（`useParams` 的 `valueOf = draft[key] ?? savedValueOf(key)`）。
→ 改成同一套模型：`EditEntry` 加 `sent` 标记，**落地之后也不撤**（它本身就是这一屏的草稿层）；
`settleAll` 的循环条件改成"还有没发的 / 还有在飞的"（按 `queue.size` 判会死循环）；
新增 `editQueue.clear()`，由外壳在 **undo / redo / save / discard** 时整层丢掉（那几刻后端那份 desk 才是真相）；
失败也**不回落显示**（与客户端 `patchDraft` 同一条口径：失败只记 `draftError`，不回滚界面值）。

**判据**：`scripts/probes/params-lag.mjs` —— 跟手 +10 / **全程单调不回跳** / **到最终值之后一帧都不许再动** /
`apply_draft=1` / `desk=registry=preflight=0` / 橙 chip / chip 回退；给 `wb_apply_draft` 人为加
**300ms 延迟**（真机量级）才照得出来 —— **9 条全绿**。

### 二、客户端：改了不保存关掉之后，草稿"看不见 / 圆点点不亮"

三处都已定位到行：

1. **广播根本没接上（主因）**：`state/draftState.ts` 的头注写着「发：参数页（`useParams`：改值 / 保存 / 取消 / 采用云端新值）」，
   可 `git grep draftMutated` 只有 `usePresetData` 那两处（正文链）—— 参数链**一次都不发** ⇒
   预设列表行上那枚橙色「未保存」圆点（`PresetTable` 的 `unsavedDot`）只能等整表重读才可能亮（作者：「那个橙色的点也不显示了」）。
   → `useParams` 在 **改值（`patchDraft`）/ 保存（`commitDraft`）/ 取消（`discardAll`）** 三处广播；改值走 150ms 尾随合并，保存 / 取消当场发。
2. **判据漏了 `dirty`**：两处 `setDraftPath(d?.path ?? null)` 没看 `dirty`，而 `begin_preset_edit` 一打开就写出草稿文件
   ⇒「只打开过、一个字没改」也会亮圆点。契约与头注写的都是「**真改过才算**」→ 两处都加 `d && d.dirty`。
3. **重开看不到改动、也回不到编辑模式**：`useParams` 拿 `beginPresetEdit` 的返回值时把 `reused` 丢了、
   把 `d.text`（含没保存改动的草稿正文）**一次都没用**；内存 `draft` 从 `{}` 起、**从不 seed**
   ⇒ 重开显示的是盘上那份文件的旧值（而草稿还在盘上，点保存会把它一起存下去）。
   → ① **`src/app/params/paramSessionCache.ts`（新）** —— 照 `PagePresets` 的 `textHistCache` 抄：按"编辑目标"记住
   **草稿层 + 撤销/重做栈**，卸载时写、`reused` 时读回来（**不存** 修改历史 `log`/`seq` —— 那两样后端按文件自己持久化，
   存了会打架）；② `useParams` 新增 **`draftOnDisk`**（读一眼 `get_preset_draft_summary().dirty`），
   `PageParams` 据此**只自动进一次编辑模式**（与兄弟链「编辑文件」的 `reused && dirty` 同一条口径）。

**附带修一处假后端与真机口径不一致**：`src/api/mock.ts` 的 `getPresetDraftSummary` 用户线原来拿
「带血统三行的盘上正文」直接比草稿（草稿是去血统的）⇒ **恒判脏**，"只打开没改"也会亮圆点。
改成与真机同一条比法（`withoutLineage(base)`）—— 不修这条，预览会一直骗人，第 2 条改动看着像坏的。

**判据**：新增 `scripts/probes/params-draft-dot.mjs`（`npm run build` + `npx vite preview --port 4173` 后跑）——
本地那份行右键「修改参数」→ 只打开不改（**不亮**）→ 进编辑模式改一个值 → Esc 关掉（**亮**）→
重开（**直接编辑模式 + 改动还在**）→ 保存（**灭**）—— **10 条全 PASS**。

**验证总账**：`npx tsc -b --force` / `npm run lint`（eslint + stylelint）/ `npm run build` / `npm run build:workbench` /
`npm run check:bundle` / `cargo fmt --check` 全绿；**Rust 一行未改**（照旧口径仍应各跑一遍 clippy / cargo test）。

---

## 增量之四十九 · 预设页六件事：复制件不认云端 / 副本族排序 / 三循环 / 改名不失败 / 参数页不自动取字节 / 云端页首帧（2026-10-20，分支 `fix/preset-copy-cloud`）

> 起因：作者六张图 —— 复制出来的那一份也显示「有新版」并在参数页显示逐项差异（图 1 / 图 3）、
> `A1-fastv3.3 copy copy.toml` 排到原件上面（图 2）、原名改名受长度限制直接失败（图 5）、
> 原件改名后徽章要照旧（图 6）、云端页第一次进去没有「更新」按钮（图 7）。
> 作者另起两条口径：**排序三循环的"默认"那档 = 族优先**；
> **参数页不自动把云端新版字节取回来做比较**（理由见下）。

### 一、四条根因（都已定位到行）

| 现象 | 根因 |
| --- | --- |
| 复制件有更新徽章 | `ipc/local.rs` 的 `get_local_preset_index` **只按血统认亲**，而 `runtime::local::copy_as_new` 是**按字节复制、三行 `# based_on*` 血统原样带过去** ⇒ 复制件必然进账本。`presetTree.ts` 那段注释声称「只认账本、复制件不在里面」，**与实现不符** |
| 副本排到原件上面 | `presetTree.ts` 的 `copyRankOf` 正则**只剥一层** ` copy`：`A copy copy.toml` 反解出 base = `A copy.toml`，与原件 `A.toml` **不同族** ⇒ 退化成整串文件名比较 |
| 云端页首帧陈旧 | 目录换版后只重读 `readPresetFeedInto()`（机型树 + 交付），**没有重读身份账本**；而云端行三态正是「账本 × 当前目录」算出来的 |
| 参数页自动取字节 | `useParams` 打开时传了 `fetchMissing: true` |

### 二、改了什么

1. **复制件独立（判定沉到后端，不在前端按名字猜）**：账本出口 `get_local_preset_index` 与
   逐参数账 `get_preset_param_sync` **共一道闸** —— 查 `runtime::provenance`（出处账，本来就有，
   `copy_local_preset` 在写、`rename` 时 `repath` 改两端）里有 `kind: copy` 的那一份 ⇒ 排除。
   契约侧两处跟着变：`getPresetParamSync` 返回 `Promise<PresetParamSync \| null>`（null = 这一份不认云端，
   界面整段不画 —— **不许当成"零项待处理"**，那会让人以为云端这一版没动过）。
2. **开关**（作者：「复制出来的也保留显示？我突然感觉挺方便的」→ 设置页「预设」块，出厂 = 关）：
   `AppStateFile` 新开 `presets` 格（`copies_follow_cloud: Option<bool>`），两条命令
   `get_copies_follow_cloud` / `set_copies_follow_cloud`；**"没选过"（null）由后端解释成出厂档**，
   前端不拿 null 猜（与自动清理那条同一条口径）。改完广播新的 `presetPrefsMutated()` 代次 ——
   只用 `readLocalPresets()` 重读（**不发网络**），不并进 `deliveryMutated`（那条会引着整条目录链重读）。
3. **排序**：`copyRankOf` 递归剥到只剩原件名（链按**靠近原件的在前**存 ⇒ 剥完要 `reverse()`，
   否则 `copy 2 copy` 会排到 `copy 2` 前面）；名称列三循环 **`family`(默认) → `asc` → `desc`**，
   `PresetSortDir` 加 `'family'`，`SortTh` 那档不画方向箭头（`aria-sort='other'`）。
4. **改名不再失败**：`check_new_name` → `normalize_new_name` 返回 `(名字, adjusted)`：
   剔路径分隔符与控制字符、按 **255 字节**上限按 UTF-8 字符边界截断（后缀一个字节不动）；
   **空 / 后缀不一致仍旧拒绝**（那两档没有可修的东西）。改名 / 复制 / 第八层另存 / 导入改名
   四处共用这一道门槛，回执带 `adjusted` 供界面照实说一句。四个老测试跟着改判。
5. **参数页按需取字节**（作者裁定）：打开时 `fetchMissing: false`；`syncNeedsFetch`
   （`versionAdvanced && !cloudReady`）时不画逐项「云端 旧 → 新」与两颗按钮，只说「云端有新版本」。
   ★ 作者的原话理由：「有些本地的没改过的参数会因为自动更新就变了，这个是不好的体验」。
6. **云端页首帧**（作者要"两条都兜住"）：首屏 `boot.changed` 后补 `readLocalPresets()`（命中根因）；
   **外加**切进云端页时 `data.refreshLocal()` 对一眼（只扫盘、不发网络）。
   没有新增"目录代次"：经核，那两处是本地扫盘即可，再挂一条代次只是为同一个目的多跑一次。
7. **文案**：参数页「选择要跟随的更新」→ **「选择要更新的参数」**（作者：「选择要更新的参数的意思吧」）。

### 三、网络请求总账（作者问"我都没点更新，本地怎么知道参数变了"）

**静默（用户什么都没点，程序自己发）**

| 请求 | 位置 | 时机 | 次数 / 拉什么 |
| --- | --- | --- | --- |
| 远端目录指纹检查 `check_remote_update` | `ipc/catalog.rs` → `usePresetData.ts` 的 `checkBootstrapOnce` | 预设页挂载（页签常驻 ⇒ **打开软件就跑**），本次运行只一次 | 1 次 HTTP，拉 1 份 `catalog.json` |
| 应用远端目录 `apply_remote_update` | 同上 | 上一步发现目录换了才发 | 1 次 HTTP，拉 1 份 `catalog.json` |
| 软件更新 `update_info` | `ipc/update.rs` → `UpdateIndicator.tsx`（挂载后 **1.2s**） | 打开软件 1.2 秒后 | 1 次 HTTP，拉 1 份 `release.json` |

**主动（用户点了才发）**

| 请求 | 位置 | 用户点了什么 | 次数 / 拉什么 |
| --- | --- | --- | --- |
| 取回云端当前版 `ensure_cloud_bytes`（全应用只有两个调用者，见下） | `ipc/catalog.rs:586` | 云端行的「下载 / 更新」（含首页「下载并应用」）；点「更新」/「选择要更新的参数」（= `get_preset_param_sync` 的 `fetchMissing=true`） | 每份 1 次 HTTP；**本机已有当前版就 0 次** |
| 交付文件下载 `download_runtime_file(s)` | `ipc/catalog.rs:255 / 359` | 下载 / 批量下载 | 1 份 1 次（批量并发 N 次） |
| 下载安装包 `start_update` | `ipc/update.rs` + `runtime/updater.rs`（`stream_get`） | 「更新 SupportEase」 | 1 次流式，拉 1 个安装包 |
| 数据源探通 `probe_current` + 形状判定 `real_fetch` | `runtime/source.rs:706 / 383` | 设置页保存数据源地址 | 1～数次（形状判定依次试几个 URL） |

**本轮摘掉的那一条**：参数页打开时的 `getPresetParamSync(path, true)` → `false` ——
作者问的那句「我都没点更新，本地怎么知道参数变了」就是它。现在**打开参数页 0 次网络**。

**不联网的（别被名字骗了）**：本地行的「使用」（`applyActivePreset`）、「复制」（`copy_as_new` 按字节复制）、
改名 / 删除 / 编辑草稿 —— 都在 `presets-local/` 里做，0 次网络。
★ 旧词「另存为本地预设」2026-10-13 已整条退役（作者：「这些字都去掉」），
它对应的动作今天就是云端行的「下载 / 更新」（`fetch_cloud_preset` 内部会 `save_cloud_as_new`）——
走的是上面那条，**本机已有当前版时 0 次**。
`save_cloud_as_new` 自己**不取字节**（它收的是调用方给的 `cloud_text`），联网那一步在 `ensure_cloud_bytes`。

不算网络：`get_file_url`（复制链接，只算地址、不发请求）。

### 四、验证

- Rust：`cargo fmt` / 双 feature `cargo clippy --all-targets -D warnings` / `cargo test` **全绿**（610 通过）。
  新名门槛补 6 条单测（含中文字符边界截断那一条）；带路径那四个老测试按新行为改判。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint`（eslint + stylelint）全绿；
  四道 check（bundle / zero-network / channel-args / release-source）全绿。
- 排序行为用临时脚本对着真模块验过一遍（副本链 + 三档全对），验完即删 —— 仓库没有前端测试框架，
  本轮不为了它引入 vitest。
- **分支上另有两处预先存在的红**（与本轮无关，本轮没碰 `presets/` 与 `workbench/`）：
  `workbench::app::build::tests::our_render_matches_the_machine_verified_baseline` 与
  `workbench::domain::variants::tests::real_data_matches_the_documented_split`
  （盘上 `A1-fastv3.3.toml` 已是 `offset_x = -0.7`，机器核过的基线还是 `0.1` —— 数据待同步）。

---

## 增量之五十 · 云端更新链路重做：点「更新」= 应用（0 项也真写）+ 弹窗默认不勾机器自己的数（2026-10-20，分支 `fix/preset-copy-cloud`）

> 起因：作者多张图 —— 云端只改了注释 / 时间戳（值一项没动），客户端却长期挂着「有新版 / 更新」，
> 点「更新」只吐一句底部文案什么也没写；点「更新」应该顺势出现「选择要更新的参数」那一屏；
> 那一屏 0 项时也该能「应用这 0 项」，应用之后「有新版」就看不到；
> XYZ 偏移与涂胶速度**不要替用户勾**（全选也要跳过、单独一组排末尾）；
> 「取消」改叫「暂不修改」，提示放在**底栏左侧**（不是页面级 toast）；
> 本地表那一行也要看到「有新版」。

### 一、根因

| 现象 | 根因 |
| --- | --- |
| 注释级换版永远挂「有新版」 | 血统比的是**全文**摘要（`lineage::sha256_hex`，含注释），换版就 `version_advanced`；而 `PagePresets.runUpdate` 在 `pending === 0` 时**提前 return**：不写盘、不推进血统 ⇒ 下一次读还是「有新版」 |
| 点「更新」只出 toast | `runUpdate` 三步里写盘那一步只写"我没改过"的项，且**从不弹框**；`ParamSyncModal` 只活在参数页 |
| 0 项不能应用 | 弹窗 `disabled={all === 0}`，且 `decideSync` 在 `keys.length === 0` 时直接 return |

### 二、改了什么

1. **写那一趟换形状（后端 `ipc::param_sync::apply_at`，2026-10-20 定的规矩）**：
   **骨架 = 云端当前版正文**（注释 / `# release_time` / 云端新增字段都在里面），
   值按决定填 —— `adopt` 写云端新值，其余（hold / 没决定）**写我这份现在写着的值**；
   血统三行换成当前版（`based_on` = 目录里那份路径、`based_on_sha256` = 当前版摘要）。
   于是 **0 项 = 一次真写**（"自动应用 0 项"）：值一个不动，注释 / 新字段 / 时间戳 / 血统跟着新版走。
   写之前留原文镜像、与原文相同不落盘（与参数页保存同一条规矩）。
2. **血统推不推进看"有没有落下的项"**：这一批之后还剩未表态的待处理项 ⇒ 血统照旧
   （那几项仍然挂着，不许被这一趟悄悄抹掉）；都处理了（含 0 项）⇒ 推到当前版，「有新版」收场。
3. **预设页「更新」顺势**：`pending === 0` ⇒ 直接 `applyParamDecisions(path, [])`；
   `pending > 0` ⇒ 打开「修改参数」并带开「选择要更新的参数」（`syncFirst`），**不再静默采用任何一项**。
   删掉原来那两句 toast（「没有要处理的参数差异」「N 项你改过 —— 去「修改参数」处理」）。
4. **弹窗**：`NEVER_PRESET_KEYS`（`toolhead.offset.x/y/z` + `toolhead.speed_limit`）默认不勾、
   **全选跳过**、单独一组（标题「机器自己的数（不替你勾）」）排在末尾；
   按钮「取消」→「暂不修改」，0 项可应用；底栏**左侧**新增常驻提示位（`hint` 可覆盖）；
   字节还没取回来时不显示「应用这 0 项」（那时 `pendingCount` 天然是 0，不是"没有要改的"）。
5. **一次写完**：新增 `useParams.decideSyncMany(decisions)`（可混合 adopt / hold，一趟写完）——
   分两趟写会出现"头一趟还有没表态的项 ⇒ 血统不推进"的中间态。`decideSync` 退化为它的同批薄封装。
6. **本地行也画「有新版」**（判据 = 这一份的 `basedOn === 'outdated'`），紧跟文件名，与云端行同一枚样式。
7. 假后端（`src/api/mock.ts`）同一条写规矩；演示那份 `A1 涂胶.toml` 的 `# based_on_sha256` 落到**旧版**那一档，
   于是它真挂着两条待处理（与那段注释说的一致），点「更新」才看得到那一屏。

### 三、验证

- Rust：`cargo fmt` / 双 feature `cargo clippy --all-targets -- -D warnings` / `cargo test --lib` **全绿（612）**；
  新增 2 条（注释级换版被 0 项应用消化 / 云端新增字段跟着进来），场景 C 按新规矩改判
  （保持 = **值还是我原来的**，不再是"文件一个字节不动"）。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check 全绿。

## 增量之五十一 · 「上一次保存之前的值」全站记账 + 写值统一浮点尾闸（2026-10-21，分支 `feat/dark-mode`）

### 一、作者的两条诉求（同一天三张截图）

1. 「我希望的是它在我点击完了保存之后还是那样显示的，不是这样都变成新的 —— 还需要一个我的旧值；
   **不只是更新才有，我希望所有的都能记录下来**，我的旧值，就是上一次保存之前的值。」
   —— 此前保存后 `sync.local` 立即变新值，「我的值 / 云端旧值 / 云端新值」三格全成同一个数，对比凭空消失。
2. 「前端显示可能截断了，但是后端怎么还是这种浮点数，能不能修好，**文件里面记录的就是两位小数的才对**，
   你可以看日志或者做一道**统一的闸门给所有动文件的用」。—— 修改历史里摆着 `3.8000000000000003`。

### 二、改了什么

1. **`prevSaved`（上一次保存之前的值）**：`useParams` 每份文件一页（钥匙 = `historyKeyText`）；
   `save()` 里对这一批**真的动了值**的项记下保存前的已保存值。**跟着修改历史账本一起落盘**
   （`PresetHistory.prevSaved`：Rust `HistoryFile.prev_saved: BTreeMap<String,String>`，`serde(default)`
   旧账照读；IPC DTO / bridge / mock 同步），清空历史一并清。
2. **展示（`ParamRow` 新 prop `prevSavedValue`，`ParamCard.prevSavedOf` 通道）**：
   - 展开详情「已保存值」后面新增一格**「上一次保存前」**（有记录且与现值不同才摆）—— 所有参数都记，不限云端更新；
   - 云端对照栏：保存过后「我的值」已是新值 ⇒ 单摆一行**「保存前」**；
   - 行名下保存过的徽章也把数字留着：采用 =「我的旧值 → 云端新值」，保持 =「云端旧值 → 云端新值」。
3. **写值统一浮点尾闸（两道，同一形状：按 1e6 收 IEEE754 尾巴，修尾不补零）**：
   - 前端 `apply()`：任何来路的值（步进器算数 / 云端 JSON / 历史还原）进草稿、进历史 `from/to` 前先
     `formatNum(raw, null)`；历史**读回**也过同一道（旧账里已有的脏值显示、还原、下次整本写回都干净）；
   - 后端 `presetdata::patch::patch_preset_toml` 的 `Float` 臂（**参数页写盘唯一出口**）同样收尾 ——
     `3.8000000000000003` 落盘就是 `3.8`。补零（`3.8 → 3.80`）仍只发生在显示层（`displayDecimals`），不动文件。
4. 来源查证：`3.8000000000000003` 不是文件里的（TOML 里是 `3.8`），是**历史账本 JSON** 里存了带尾巴的草稿值；
   闸设在两处出口后，新账干净、旧账在下次写回时被洗净。

### 三、验证

- Rust：`cargo fmt --all --check`（本机 + `+1.99.0`）/ 双 feature `cargo clippy --all-targets -- -D warnings` /
  `cargo test --lib` **全绿（634）**；新增 3 条（patch 浮点尾闸 / prev_saved 往返与旧档兼容）。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check + `check:calibration-assets` 全绿。

## 增量之五十二 · 拖拽改窗口大小时露白：原生背景色随界面底（2026-10-22，分支 `feat/dark-mode`）

### 一、作者报的现象

「改窗口大小的时候，右边和底部会拖出一条白色，而且内容明显慢半拍」——
鼠标、触控板一样；`npm run tauri dev` 与打包版一样（**与构建模式无关**，这一点先排掉了）。
对照物：QQ / 微信 / 网易云 / Trae / Blender 都跟手；**EcoPaste 同为 Tauri，也有、只是轻**
（作者原话：「有的有一点，没我的这么迟缓」）。

### 二、根因（不是我们的代码，也不是拖动实现）

- `useTitlebarDrag` 走的是系统 `startDragging` → `performWindowDragWithEvent:`，与 QQ 同一条；
  且现象只在**改尺寸**时出现（拖动窗口位置时窗口 bounds 不变，不需要重新布局）。
- 真因是 **WebKit 的 live resize 呈现 bug**：`WebKit#323132`
  （`[macOS] WKWebView content lags behind its bounds during live window resize`），
  下游 **`tauri-apps/tauri#14288`**。机制：AppKit **同步**推进 WKWebView 的 bounds，
  而 WebContent 进程的远程图层树是**异步提交**的 —— 新图层到达前，屏幕上仍是上一次提交的
  旧尺寸页面表面，盖不满新 bounds 的边就露出来（上游量到 20~60px）。
  **Tauri 维护者已表态这个场景 wontfix。**
- issue 里"已验证无效、别再试"的清单（本次也逐条排掉了）：
  `Resized` 里强推 webview / superview 的 frame、`preservesContentDuringLiveResize`、
  `NSViewLayerContentsRedrawPolicy::DuringViewResize`、在 `html`/`body` 反复设背景色、
  在 webview 下面垫一层同色原生视图。
- 本地另排掉三条：**窗口拖动实现**（系统拖动）、**`contentView.layer.masksToBounds` 圆角裁切**
  （关掉无效，四角也不受影响）、**透明标题栏 + 空 `NSToolbar`**（撤掉无效）。
- **试金石实验**（已撤）：监听 `NSWindowWillStartLiveResize` / `DidEndLiveResize`，
  resize 期间把 `contentView` 挪出可见区 —— 作者看到的是一整块**纯色界面底 + 圆角完好、
  严格跟着窗口边长** ⇒ **窗口层完全正常，慢的只有 WKWebView 的页面内容**。结论闭环。
- 为什么做不到 Trae 那样：Chromium 自己有合成器，live resize 时会把**上一帧拉伸**顶住；
  WKWebView 的画面在另一个进程，AppKit 的 `cacheDisplay` 截不到，想截只能走
  `CGWindowList` + **屏幕录制权限** —— 为了拖窗口手感让用户授权录屏，不做。

### 三、改了什么（只治"白"，不治"慢"）

1. **`ipc::prefs::set_window_backdrop`**（同步命令 —— 碰 AppKit 必须在主线程）：
   走 `WebviewWindow::set_background_color`，它一次做三件：
   ① 窗口底色；② wry 侧关掉 webview 的 `drawsBackground`（**这才是露出来的那一层白**）；
   ③ `underPageBackgroundColor`。只设 `NSWindow.backgroundColor` 是上一版的弯路 ——
   露的是 WKWebView 自己的白，不是窗口底，所以"没效果"。
2. **颜色不存第二个源**：前端 `landTheme()`（`appState.ts`，替换两处 `applyTheme`）在
   `<html data-theme>` 落地之后读 `--bg-2` 报上来，真值仍然只在 `tokens.css`。
3. `contract` / `bridge` / `mock` 各一条；浏览器 mock 是空实现。

### 四、验证

- Rust：`cargo fmt --all --check` / 双 feature `cargo clippy --all-targets -- -D warnings` /
  `cargo test` **641 全绿** / `cargo test --features workbench --lib` **1036 全绿**。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check + `check:calibration-assets` 全绿。
- 真机：`setWindowBackdrop` 调用成功（日志 `ok`，无 `[chrome]` 告警），快拖时不再闪白。
## 增量之五十三 · i18n 第二刀：`presetTree` 整文件全量迁 + 三个 shared 模块（2026-10-26，分支 `feat/dark-mode`）

### 一、为什么要这一刀

第一刀（1119 条键）只覆盖各子代理名单内的文件，名单外的四个模块仍是中文字面量 ——
切英文后预设页的分段控件、操作列、展开详情、右键菜单、归档抽屉、下载水位，
以及后处理 12 步、切片器四档、打开模型框里那句话，全都还是中文。

作者裁定：`presetTree.ts` **整文件全量迁**（含 `ARCHIVE_WHY` 这类长说明），
带文案的数据管线函数**一起迁、`tr` 一路穿下去**。

### 二、做法（一条型：常量改存 `StringKey`、函数收 `tr` 首参）

- `src/i18n/index.ts` 新增并导出 `type Tr`（`useT()` 的返回）。`PageReport` 里那份
  本地 `type Tr = ReturnType<typeof useT>` 删掉、改成 import —— 全仓一份定义。
- `presetTree.ts`：约 40 张词表改存 `StringKey`（`Record<Axis, StringKey>` 形状不变，
  调用处 `tr(TABLE[x])`）；13 个函数收 `tr` 首参：
  `sourceTextOf` / `originCellOf` / `archiveOpenText` / `usageCellText` / `basedOnCellText` /
  `localUnreadableWhy` / `presetBatchText` / `versionsText` / `presetPathText` /
  `buildPresetTree` / `localRows` / `cloudRows`。
  `EDIT_TEXT.draftFailed` / `savedBack`、`IMPORT_TAKEN_TEXT.note` 由「前缀拼接」改成
  整句占位符（`{detail}` / `{name}`+`{path}` / `{name}`+`{suggested}`），调用处同步。
- `postprocess/steps.ts`：`StepView.name: StringKey`，`stepName(tr, id)`；认不出的 id 仍原样。
- `shared/slicer.ts`：`SLICER_CHOICES` 的 label/hint 改键（**应用名也进表**，换语言不该
  让下拉认出另一个应用），`slicerOptionsOf(tr, …)` / `systemSlicerLine(tr, …)`。
- `shared/download.ts`：`tickText(tr, t)` / `outcomeText(tr, o)`。
- 词典 1119 → **1274 条**（含 `common.clauseSep` 这一条标点，页面里 `join('；')` 也随语言走）。

### 三、两处不是"换个写法"的决定

1. **`buildPresetTree` 的 `后端没有 X 这个组合`**：它产生在**加载 effect** 里（20+ 个请求）。
   直接把 `tr` 塞进那个 effect 的依赖 ⇒ 换语言就整屏重读一遍。
   所以 `usePresetData` 改成存**建树的四份原样输入**（`TreeSeed`）+ `useMemo([treeSeed, tr])`
   出树：换语言只重出一棵树，不重发请求。
   这一条与"页面常驻挂、切页只切 `data-hidden`"有关 —— 切语言时预设页并没卸载，
   若用模块级 `t()` 取那句话，它会一直是上一种语言。
2. **删死表**（全仓 grep 复核、0 引用）：`DELIVERY_TEXT` / `PRESET_STATE_TEXT` /
   `STATUS_WHY` / `ASSET_USE_WHY`。`ORIGIN_TEXT` / `ORIGIN_WHY` / `CLOUD_LOCAL_WHY`
   仍被活路径读，随函数迁成 `StringKey`，不删。
   目的是不留"同一个词两个出处"（`STATUS_TEXT` vs `LIVE_TEXT` 那种）。

### 四、验证

- `npm run check:i18n` **1274 条键，键集与占位符两语完全一致**。
- `npx tsc -b --force` / `npm run lint`（0 error，仅 `PagePresets:621` 那条既有 warning）/
  `npm run build` / 四道 check 全绿。
- 探针：`i18n` **13/13**、`window-gadgets` **36/36**。
- 浏览器实测（`vite preview` + playwright，切 English 后逐屏扫 CJK）：
  预设页本地表 / 展开详情 / 右键菜单（`Use … Delete`）/ 云端表、
  `TextDiffModal` 的「Enter edit mode」→ 底栏「Discard / Save」—— **界面侧 CJK = 0**，
  剩下的全是 mock 数据本身（机型/版本名、备注、文件正文）。
- Rust 四道这一刀没跑：**只改前端**。

### 五、仍未迁完的一批（下一刀，已盘点）

参数页与几处共用组件**整页尚未迁**（不是本刀范围，也不是本刀改出来的）：
`params/PageParams.tsx` 18 处（`撤销（Ctrl+Z）` / `重做` / `丢弃改动` / `恢复默认值` /
`确认保存？` 这一整块底栏与两张 sheet、空态标题、搜索框 aria-label）、
`params/ParamRow.tsx` 17 处（详情 `dt/dd` 那一列 + 云端更新块）、
`params/PresetParamsModal.tsx` 2 处、`calib/PageCalib.tsx` 7 处、
`bbs/PageBbs.tsx` 2 处、`bbs/BbsTabs.tsx` 1 处、`shared/Drawer.tsx` 1 处、`shared/NoteBar.tsx` 1 处、`notify/NotifyHost.tsx` 1 处、
`components/modal/Modal.tsx` 1 处、`app/components/TraceTag.tsx` 1 处，合计约 52 处。
（`presets/CompareModal.tsx` 那 3 处——标题「参数对比」与两句 subtitle——本刀顺手一起迁了。）
`src/workbench/**` 仍全中文 —— 那是开发台、只有作者看，不在这条线上。


## 增量之五十四 · i18n 第三刀：内核 + 主进程的文案也进表（2026-10-26，分支 `feat/dark-mode`）

### 一、作者的原话

「我不想看到任何一个中文」。截图上是设置页更新卡里那段红框：
`The check failed: 从远端取字节失败：https://…` —— 后端那句中文原样冒进英文界面。

根因不是漏翻一条，而是**两套字典**：前端住 `src/i18n`（1274 条），
内核 `mkpse-pp` 那套住 `crates/postprocess/messages.toml`（`Lang` + `--lang` +
`text()`）。第二刀只迁了前端，**后端整条线还是中文源**。

### 二、做法

1. `crates/postprocess/messages.toml` 补了 **582 条 `ui.*`**（其中 51 条原本连 `text_en` 都没有，
   也一并补齐），现在全表 **661 条**、两语齐全、占位符与 `params` 逐条对齐。
2. `src-tauri/src/**` 里所有给用户看的句子改成 `messages::text("…键…", &[…])`：
   `AppError::*` 的参数、`with_detail(…)` 的说明句、`devsim` 演出脚本、
   `hook_ui` 的收场通知、归档那本 `_meta.json`、CLI 的 stderr。
   两个"名字"也进了表：`SlicerKind::label()`（系统默认程序 / 你指定的应用）与
   校准页那 5 张模型卡的名称与说明（`MODEL_DEFS` 改存键）。
3. **语言怎么到后端**：前端语言格已经存在（`app-state.locale`），钩子起进程时带 `--lang`；
   `mkpse-pp` 与 GUI 共用同一个 `Lang` 单值。
4. `net.rs` 那 8 条盖上了码（`E_NET_READ_001` / `E_NET_DNS_001` / `E_NET_FETCH_001` …），
   于是错误话术走"人话 + 尾注码"，前端不必自己拼。
5. **`SlicerKind::label()` 拆成两个**：`label()` 给人看（随语言）、
   `product_name()` 给 Windows 注册表查 `App Paths`（**产品名，不随语言**）——
   这两个用途混在一起时，翻成中文就找不到注册表项了。

### 三、两处判据要跟着改（不是"修测试"，是判据本身过时了）

- `presetdata::registry` 的三条单测原来断言 `contains("悬空")` 这类**中文原文**。
  文案进表后正确写法是断言**那个变量名**（`ghost` / `a.x` / `a.z`），与语言无关。
- `tests/cli.rs` 的 `run_cli` 钉上 `--lang zh-CN`：那一组判的是"报错点没点名那个东西"，
  下面几条断言的中文是**源语言约定**；不钉的话，机器上有个 `LC_ALL=en` 就会红。

### 四、验证

- `cargo fmt --all --check` / 双 feature `clippy --all-targets -D warnings` 0 条。
- `cargo test -p mkpse-postprocess --lib` **235**、`-p mkp-support-ease --lib` **641**、
  `--features workbench --lib` **1036**。
- 前端六道：`check:i18n` **1274 条**、`tsc -b --force`、`eslint` 0 error（仅既有 621 行 warning）、
  `npm run build`、`check:bundle` / `zero-network` / `channel-args` / `release-source` / `calibration-assets`。
- 探针 `i18n` **13/13**、`window-gadgets` **36/36**。
- 浏览器实测切 English 后逐屏扫：6 个页签里剩下的 CJK **全是数据**
  （机型名"拓竹"、版本名"快拆版6月以前"、参数名"层高"/"接缝"、mock 的演示文案、
  "简体中文"这个语言项本身）。
- **`process_with_ir_matches_the_end_to_end_reference` 红**：`git stash` 退回本刀全部改动
  后同一条同样红（长度差 447 字节，值一模一样），是**既有失败**，与本刀无关。

### 五、还没做的一批（已盘点 ≈52 处）

前端参数页整页、`PageCalib` 7 处、`PageBbs` 2 处、`BbsTabs`/`Drawer`/`NoteBar`/
`NotifyHost`/`Modal`/`TraceTag` 各 1 处（见增量之五十三第五节）。
`src/workbench/**` 仍全中文 —— 开发台只有作者看，不进这条线。

## 增量之五十五 · i18n 第四刀：参数页整页 / 校准页 / 工艺页与共用件（2026-10-26，分支 `feat/dark-mode`）

### 一、先修掉一个"打不开"

作者截图：设置页切语言报「没换成：这个功能还没接好（setLocale）」。

根因不是命令没注册，而是**这一刀之前那批改动根本编译不过**：
`AppStateFile` 新增了 `locale` 格，但 `Default::default()` 与 `legacy_snapshot()`
两处初始化没补上 ⇒ `cargo build` 直接 E0063 ⇒ `tauri dev` 一直在跑**旧二进制**，
旧二进制里没有 `set_locale` ⇒ 前端桥把 `Command set_locale not found` 翻成那句
「这个功能还没接好（setLocale）」。补两行 `locale: None` 后编译通过，
新二进制里 `get_locale` / `set_locale` 都在（`strings` 可验证）。

**教训**：加 AppState 的格 = 同时改三处（字段 / Default / legacy_snapshot），
漏一处不会红在测试里，只会红在"用户点了没反应"。

### 二、迁完的那一批（字典 1274 → 1430 条）

`PageParams` / `ParamRow` / `ParamCard` / `ParamSyncModal` / `PresetParamsModal` /
`PageCalib` / `PageBbs` / `BbsTabs` / `Drawer` / `NoteBar` / `NotifyHost` /
`Modal` / `TraceTag` / `useParams` —— 逐句进字典，`check:i18n` 1430 条两语一致。

两处结构性改动（不是换字符串）：

1. **`valueText` / `invalidNumberText` 收 `tr` 首参**。它们是模块级函数，
   原先自己调 `t()`（模块级那一格，语言换了组件不重渲染）。现在与
   `presetTree` / `steps` 那批同一条规矩：谁调谁把 `tr` 传进来。
2. **保存回执的语气由产出方给**：`useParams` 新增 `savedTone: 'ok' | 'warn' | 'bad'`，
   `PageParams` 不再用 `savedNote.startsWith('保存失败')` 猜红绿 —— 文案进字典之后
   那个前缀在英文界面里根本不存在，一猜就全绿。

条件三档（`等于/不等于/大于`）与「N 行」「开启/关闭」「空」也一并进表（`params.op.*` /
`params.value.*` / `params.invalid.*`）。

### 三、这一步仍然做不到的（要作者裁）

1. **参数名与说明**：`presets/registry/param_registry.toml` 是**中文数据**
   （136 个参数 + 15 个页签 + 40 个分组），随 `presets/delivery/catalog.json` 发布。
   要随语言变，得给它加可选 `label_en` / `desc_en` 并重发 catalog —— 那是**数据形状**
   的改动，还会牵到工作台的参数定义编辑器（它要能编第二种语言，否则一保存就抹掉）。
   另一条路是英文名只住前端字典（`param.label.<paramKey>`），registry 一个字不动。
2. **历史条目**：`run/preset-history/<hash>.json` 里存的是**当时那句话**（`action`）。
   现在按当前语言生成，但**老条目不会复译** —— 要复译得把 `action` 从句子改成
   记号（`actionKey` + 参数），是账本形状的改动。
3. **`src/workbench/**`**：2147 行含中文（剥注释后），且**一行 i18n 都还没接**。
   它是另一个应用（`workbench.html` + 自己那套视图），不在这一条线上。

### 四、验证

- `cargo fmt --all --check` ✓；双 feature `clippy --all-targets -- -D warnings` 0 条；
  客户端 **641**、内核 **235** 全绿（`process_with_ir` 那条既有红不在本刀范围）。
- 前端：`check:i18n` **1430 条**、`tsc -b --force`、`eslint src` 0 error
  （仅既有 `PagePresets` 621 行那条 warning）、`npm run build`、五道 check 全绿。
- 探针 `i18n` **13/13**（切 English 后设置页 / 页签 / 窗口胶囊逐项断言）。

### 五、参数名/说明：改成双语数据（作者裁定：走 registry，不走前端字典）

`presets/registry/param_registry.toml` 现在每条都带英文：

- `labelEn` / `descEn`：75 个参数全覆盖；`[[params.choices]]` 45 条有 `labelEn`；
  `[[tabs]]` 8 个、`[[tabs.sections]]` 27 个有 `labelEn`（分组还有 `descriptionEn`）。
- 两套解析结构同步加字段：`crates/preset/src/registry.rs`（内核/CLI 读的那套）
  与 `src-tauri/src/presetdata/registry.rs`（应用读的那套，另有 `ParamMetaEdit`）。
- **新的覆盖闸** `presetdata::registry::tests::every_entry_has_english`：真数据里
  任何一条缺英文就红。以后加参数必须顺手写英文（回落中文是安全网，不是常态）。

**语言在哪一层挑**：界面。IPC 两条通道（`getRuntimeCatalog` 的 `registry` 与
`getMachineParams`）把两册一起下发，`useParams` 在**读定义时**按 `locale` 摊开
（`localizeDef` / `localizeTab`，出口是 `defOf` / `defs` / `tabs` / `sectionsOf` /
`defsOfSection`）。这样切语言不必重拉数据 —— 重拉会把用户选的机型顶回"正在用的那一份"。

**工作台不会被保存抹掉**：`ParamMetaEdit` 的 `label_en` / `desc_en` 是 `Option`
（`None` = 不动，只有它主动发了才改写），选项那条按 `value` 沿用文件里原有英文。
工作台的参数编辑器**还没暴露这两格**（界面仍是中文），所以它保存不会破坏英文，
但也编不了英文 —— 那一格等做工作台 i18n 时一起开。

**代价（已付）**：`catalog.generated.json` 与 `presets/delivery/catalog.json` 都重新生成了
（`cargo run --bin gen-catalog`），结构签名随之变化（`revision b654d984…` / `5aeab99c…`）。
**发布预设是另一件事** —— 这两份已提交但没发；下次 `npm run publish:presets`
或工作台「发布」时才真正出门。在那之前，走 OTA 拿到的是旧目录（`registry` 里没有英文），
英文界面会回落中文，不会崩。

**已知的、故意留着的一处**：搜索只匹配**当前语言**的名字（英文界面搜不到中文名、
中文界面搜不到英文名）。要做双语搜索就在 `PageParams` 的匹配里把两册都喂进去。

## 增量之五十六 · 英文界面清扫：三条读取路径收口 + 工艺页转英文 + 复述型 tooltip 退场（2026-10-26）

作者拿 6 张英文界面截图点名：参数弹窗的分类/卡片/参数名、对比台的页签与「一致」、工艺页的组名与
字段名、报告页的管线明细、塔冲突那两颗按钮。裁定三件事：截图放 `tmp-shots/i18n-screens/{before,after}/`、
只删「复述型」tooltip、字典按页分段 + 出一张对照表。

### 一、根因是「同一份数据有两条读取路径」

上一刀把双语挑一册做进了 `useParams`（`localizeDef` / `localizeTab`），**对比台不经过它** ——
它自己从 `getRuntimeCatalog` 摊一遍（`CompareModal.rowsOf`），于是英文界面上整片中文。
工艺页同理（`BbsRow` 硬取 `meta.label.zh`）。

修法不是补第二份挑选逻辑，而是**收口成一个工具**：`src/app/shared/dataLabel.ts`
（`bi` / `localizeParamDef` / `localizeTab` / `localizeParamMeta`）。现在三条路共用它，
`useParams` 里那两个只是改了 import。**以后谁再读这层数据，也走这里。**

另：假后端的 `param_registry.json` 与 `params.ts` 同步补了 `labelEn` / `descEn` / `name_en`
透传 —— 它是真 IPC 的镜像，缺了这层，界面上看着「修好了」而真机没修（或反过来）。

### 二、工艺页（BBS）

- 五组标题（MKP 工艺 / 导入的 / 用户配置 / 系统配置 / 其他机型 / 读不了的）从模块级中文字面量
  改成**字典键**（`bbs.group.*`），`BbsMenuGroup.titleKey`。
- 页签名与分组名读 `layout.json` 的 `name_en`（数据本来就有两册）。
- 参数名 / 选项名按 `locale` 挑 `label.zh|en`（`BbsRow` / `BbsControl`）。
- 灰着的原因改成 `reasonKey` + `reasonArgs`：**数据层只说事实，取字在渲染处**
  （`bbsSource.reasonText(it, tr)`）—— 模块级函数拿不到 hook，也不该把某一刻的语言冻进数据。
- 支撑样式、孤儿组、导入/取数失败、抽屉与详情栏的名字，一并进字典（`bbs.*` 新增 ~35 条）。

### 三、提问按钮

`hook.rs` 的两张 `QuestionChoice` 表（挪塔 / 校准）原先写死中文，英文界面下**按钮本身是中文**。
现在走 `messages::text`，新增 8 条（`hook.choice_*`，各带 `Q_*` 码，`every_entry_has_every_language` 钉着）。

### 四、复述型 tooltip 退场（作者口径：只删「把可见文字再说一遍」的）

删了 5 处：分类胶囊的「名字 · 条数」、BBS 值文本与下拉的 `title`、详情栏预设名、
`Switch` 那个**没人传**的 `title` 口子（整个 prop 删掉）。补充型（完整文件名、快捷键、
被截断的全称）与关键解释（采用/保持/还原的后果、脏态关闭、参数键+模式+不可用原因）**全留**。
清单：`tmp-shots/i18n-screens/tooltip-inventory.md`。

### 五、字典分段与对照表

`zh-CN.ts` / `en.ts` 按页插了 **36 段**分节注释（`// ---- 工艺页（BBS） ----`）。文件是逐刀追加长的，
**没有重排键**（重排会让以后每处 diff 对不上行号），所以同一页可能出现好几段，带「（续）」。新增
`docs/I18N-MAP.md`：四类「名字」各住哪（句子 / 后端句子 / 参数名 / 数据名）+ 前缀→页面→文件→条数。

### 六、取证工具

- `scripts/probes/i18n-shots.mjs <before|after>`：9 个页面 × zh/en，视口 + 整页各一张，
  落 `tmp-shots/i18n-screens/<阶段>/<页>/`（不入库）。比对台与参数弹窗是真点开的（右键两下那条老入口）。
- `scripts/probes/i18n-cjk.mjs`：英文模式下扫 `document.body.innerText` 里含中文的行 +
  它所在的 class。**这是截图看漏时的兜底**，当前结果：**29 行**，其中 27 行是报告页管线明细
  （见下）、1 行是语言选项「简体中文」本身、1 行是假后端的「演示：…」注释。

### 七、这一刀**没做**的（都记在案）

1. **报告页管线明细**（`archive.rs::step_detail`）：那是写进 `_meta.json` 的**归档记录**。
   要双语得像「结构化事实 + 前端渲染」那样改归档契约（老记录仍保持原样），单独一轮 —— 不能
   在拼接处换 `messages::text` 了事：那样记录的语言就被写它那一刻的界面语言冻住了。
2. `usePreset.ts` 的 `PENDING_NAME`、`useBundleFiles` 的三句套餐提示、`openWithGuide` 的 `alt`
   —— 都是低频/失败态，且 `alt` 描述的是中文系统截图（译了反而对不上图）。
3. `src/workbench/**`：作者裁定单排一轮。

### 八、验证

`cargo fmt --all --check` ✓ · 双 feature `clippy --all-targets -- -D warnings` 0 ·
客户端 **642** / 内核 **235** 全绿 · `tsc -b --force` ✓ · `eslint src` 0 error
（仅既有 `PagePresets` 621 行 warning）· `npm run build` ✓ · 六道 check 全绿 ·
探针 `i18n` **13/13**、`i18n-cjk` 29 行（逐条见清单）。

## 增量之五十七 · 管线明细存事实、首页文案与字距（2026-10-26）

### 一、管线明细不再存句子

`archive.rs::step_detail()` 那套 `format!("读取 {} 行…")` 全删了，换成
`step_detail_lines()`：每行一个**字典键 + 原始参数**（`report.line.*`，值全是原始事实
`disk` / `true` / `0.70`）。`PipelineStep` 加 `detailLines` 字段。

规矩：**记录里存事实，词的映射在界面**（`PageReport` 的 `argText`）。于是同一份
`_meta.json` 中文界面读中文、英文界面读英文，归档文件本身不再被写它那一刻的语言冻住。
老记录只有 `detail`（当年那句中文）—— 界面回落，**不復译**，这是记录该有的样子。

顺带：报告页的**进度句**（`pipeline[].message`，内核 `timer.begin` 那 13 句）也按步骤 id
换成本地文案，内核那边一个字没动（它写的是日志与归档的事实，不是界面）。

三条老测试（`step_details_carry_the_facts_the_report_shows` 等）从比中文改成
**比键与事实** —— 换字典不会红，漏了行才会红。

假后端同步成同形状，否则浏览器预览永远看不到英文那套。

### 二、首页

- 五步说明精简到 3–4 个词（`Latest reference values` / `Z plate tells which cell` /
  `XY plate tells which line` / `Confirm the result`）。原来那几句在四列档挤成两三行，
  还会压到相邻格上。
- **「看起来像有空格」的真凶**：`.groupTitle` 的 `letter-spacing: 0.18em` —— 字距在
  **最后一个字母后面也加一格**，英文标题后面凭空多出一个空格，再叠 flex 的 gap 就成了
  「Machine   Sets …」。用等量的负右边距（`margin-right: -0.18em`）抵掉，中文看不出来。

### 三、顺手两处

- BBS 单位栏原来硬取 `meta.unit.zh`（`mm 或 %`），改成按语言挑（`mm or %`）。
- Z 轴图例英文那串里的全角竖线 `｜` 换半角。

### 四、现在的残留（`i18n-cjk` 9 行，全是数据）

拓竹 (Bambu Lab) / 两个 `.toml` 文件名 / 三个版本名（快拆版6月以前、标准版）/
一个模型名（帕姆）/ 语言选项「简体中文」/ 假后端的演示注释。

**其中品牌名与版本名是作者点名要英文的下一刀**：catalog 的 `machines[].brand` 与
`versions[].name` 加可选 `nameEn`，界面按 locale 挑，工作台里能编。等作者发话再动。

### 五、验证

`fmt` ✓ · 双 clippy 0 · 客户端 **642** / 内核 **235** 全绿 · `tsc` ✓ · eslint 0 error ·
build ✓ · 六道 check ✓ · 探针 `i18n` 13/13 · `i18n-cjk` **9 行**（29 → 9）。

## 增量之五十八 · 发布闸把「没有要发的东西」说出来（2026-10-29）

作者问：什么都没改，一点发布就要变第 2 号，"那不是没什么意义吗"。

**事实**：事务根本不会分配到 2 —— `publish_tx` 在分配序号**之前**就 `inspect` 账本，
`catalogHash == 盘上 catalog.json 的 sha256` ⇒ 判 `NoChange`（零写入、不提交、序号不动）。
`audit.rs` 也已算出 `nothing_to_publish`，`api.ts` 有类型 —— **只有界面没接**：
`PublishGateModal` 把 `publishSerial`（= 下一号）直接印成"这次会把目录写成第 2 号"。

改了三处（只前端）：

- 闸：`nothingToPublish` 为真时改写「这一代与已发布的 support-ease 第 1 号逐字节相同 ——
  没有要发的东西：不会推进序号、不写一个字节、也不提交」；
- 回执：补 `PublishStage.noChange`（`STAGE_RANK` 原来缺这个键，`tsc --force` 会红）——
  subtitle / 过程 / 结果 / 客户端四块各说一句，不再出现"本轮走到 0/7 步"或"这一轮没走到生成"；
- 假后端补 `nothingToPublish: false`，保持同形状。

验证：`tsc -b --force` ✓ · eslint 0 · `build` ✓ · `build:workbench` ✓ · `check:i18n` ✓（1582 键）。
只改前端，Rust 四道未跑。后已提交 `bb46c00` 并推 origin/main。
