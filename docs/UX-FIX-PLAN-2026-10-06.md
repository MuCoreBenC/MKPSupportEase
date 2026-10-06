# UX 场景测试修复清单（2026-10-06）

- 依据：[`UX-SCENARIO-FINDINGS-2026-10-06.md`](UX-SCENARIO-FINDINGS-2026-10-06.md) 首轮黑盒用户故事测试。
- 范围：**只修三个 A 级**。B11（机型列继承）属身世那一批，按 [`PRESET-COPY-LINEAGE.md`](PRESET-COPY-LINEAGE.md) §3.2 另行施工；B8（时间词汇）跟时间语义那批走；C / D 不动。
- 施工顺序（作者定）：**A3 → A1 → A2**。修完跑一轮回归（探针 + cargo test），**不重跑用户故事**（那留到下一轮）。
- 2026-10-06 第二轮（A3 数据链验收，不动代码）：A1 / A2 完结；A3 改记「**写链已收口，读链待收口**」，新增「A3 根治验收」与「A3 读链收口施工」两节 —— 施工前先钉死读取/订阅层设计，再一次性拆回页签对账，不打补丁。
- 2026-10-06 第三轮（读链收口施工完成）：架构再上移一层 —— **推翻「一种状态一个文件」**，立 `AppState`（`run/app-state.json` + 唯一读写入口 + 事件广播，见 [`APP-STATE.md`](APP-STATE.md)）。A3 按「写链 ✅ + 读链 ✅」完结，状态见总表。

## 总表

| # | 问题（用户视角） | 根因（实现层） | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| A3 | 应用了「我的 A1 涂胶.toml」，首页设备卡却说"预设文件 A1F.toml"；校准页 / 参数页同病 | 只有预设页横幅直接读唯一底账（`run/active-preset.json`）；首页 / 校准页 / 参数页显示的都是"选中 combo → 目录反查的官方文件名"，底账只用来反填三级选择。参数页更连"正在编辑哪一份"都不显示（编辑目标可能是我的文件，标签却永远是目录那份） | 三处显示统一规则：**底账命中当前 combo 时，显示底账那一份**（我的文件带「正在使用 · 我的文件」说明）；参数页的文件标签跟着**编辑目标**走（同 combo 我的文件优先——这条表本来就存在）；参数页保存确认框按编辑目标说清真实落点 | ✅ 读链已收口（AppState，见下） |
| A1 | 官方预设想复制一份自己改：右键"另存为一份新的"灰掉、提示指向不存在的按钮；参数页确认框说"将写入官方文件"，用户不敢点 | 另存为对 release 行一律禁用（`copyWhyNot`）；正路（改这份→保存为用户文件）只在详情面板里且提示不说它在哪；参数页确认框说的落点（官方文件）与实际落盘（`presets-mine/…（已修改）.toml`）不一致 | 按 [`PRESET-COPY-LINEAGE.md`](PRESET-COPY-LINEAGE.md) §3.1 实施：**release 且 `releaseState === 'ok'` 的行放开「另存为一份新的」**——新命令 `copy_release_as_new(file_name, new_name)`（可信字节 + 血统三行 + 落 `presets-mine/`，不覆盖、不碰状态）；复用同一条命名抽屉；提示与 toast 改口；参数页确认 / 底栏按编辑目标说"另存成你自己的一份 / 写回你自己那一份" | ✅ |
| A2 | 失败提示是开发者聊天记录："没有 asset id —— 而契约的 applyPreset / copyToSlicer 只认 asset id。给一个点了必报错的按钮比不给糟"；且把登记在案的官方文件说成"你自己放进预设目录的" | `NO_ASSET_WHY` 是写给开发者的守卫文案，还被"理论不可达"的按钮分支（`PagePresets.tsx:889`）在演示数据的旧行形状下真实触发；`NotImplementedError` 渲染成"[API] 未实现的接口: xxx"；错误条没有"能怎么办"的后半句 | ① 文案全部改成"发生了什么 + 能干什么"，不再断言文件是谁放的；② 按钮层修缝：官方 MKP 行**没有交付身份（releaseUid）就不给「应用」按钮**（灰杠 + 人话原因），守卫退回真正的兜底；③ `NotImplementedError` 带人话 hint（"浏览器预览里没有下载区——下载要用桌面版"），技术形式只进控制台 | ✅ |

## A3 显示规则（一处规则、三个界面）

```text
底账（active）命中当前选中的 机型:版本 时：
  显示底账的 fileName（我的文件另带一行「正在使用 · 我的文件」）
  首页后处理脚本的 --Toml 路径也跟着底账的 path 走
底账没命中 / 没有底账：
  维持现状（显示目录里这份 combo 的文件）—— 预设页横幅仍是"正在使用"的唯一权威
```

| 界面 | 改哪 | 说明 |
| --- | --- | --- |
| 首页 | `activeSelection.ts`（新增 `useActivePresetOnTab`）、`PageHome.tsx`（主页 sheet 文件名 / 路径 / 脚本路径、picker sheet 的 `PresetStack`）、`PresetStack.tsx`（新增 `inUse` 说明行） | 回 tab 时已按底账反填三级选择（既有），补的是"文件名也用底账那份" |
| 校准页 | `PageCalib.tsx`（预设 pill） | 同一条规则 |
| 参数页 | `useParams.ts`（`fileLabel` 优先取 `editTargetByCombo`——同 combo 我的文件优先的表已存在）、`PageParams.tsx`（底栏 `stateText`、保存确认框按 `editingPreset.origin` 分口） | 编辑目标 = 我的文件时，pill 显示我的文件名，确认框说"写回你自己那一份"；官方目标说"另存成你自己的一份（presets-mine/…（已修改）.toml），官方原件一个字不动" |

## A3 根治验收（2026-10-06 第二轮）：写链已收口，读链待收口

> 上面「A3 显示规则」一节记的是第一轮已实施的**显示层**修复（统一读数 + 回页签对账）。第二轮按数据链验收（只审计代码路径，不动代码），结论：**A3 不能叫根治 —— 当前形态是「一个真相源 + 多个页面快照 + 回页签对账」，是同步机制，不是单一真相源。**

A3 的最终判据（本轮钉下，收口以此为准）：

> 如果 `run/active-preset.json` 是唯一真相源，那么任何页面都不应该需要「回页签」「重新打开」「重新点一下」才能知道它变了。手改底账、不碰任何界面，所有页面自然跟着变 —— 做不到，就还是同步机制。

**写链 —— 已收口 ✅**
- 后端：`save_active` / `save_active_mine` / `repoint_active_mine`（改名跟随）/ `clear_active`（删除撤指针）全部汇到私有 `write_active`，原子写同一个文件（`runtime/state.rs`）。
- 前端：全 UI 唯一写入口 = 预设页 `applyActivePreset` / `clearActivePreset`（`usePresetData.ts`）。首页 / 校准页抽屉里的「应用」只是换选择、不写底账（应用只发生在预设页）。
- 底账文件只存最小身份（origin / fileName / sha256 / path）；`machineId` / `versionId` / `intact` **读时现算**（官方线查目录登记、我的线读文件头血统，`ipc/catalog.rs` 的 `active_dto`）—— 手改底账也会被正确解析，不存在会陈旧的富化缓存。

**读链 —— 待收口 ❌**
- 每页各持一份快照：首页 `sel`、校准页 `sel`（另有模块级 `selCache`）、参数页 `pick` + `active`、预设页 `active`。除预设页外都靠「回页签读一次」对账（`useActivePresetOnTab` + 三处 effect）；预设页**不收 `active` prop、`load` 依赖不含页签** —— 启动快照 + 自己的写操作之外永不更新。
- 手改底账（测试 4）：首页 ✅、校准 ✅（回页签重读）、参数 ✅（回页签弹回 combo；`active` 快照只随 combo 变化重读，「同 combo 换一份文件」的边角会滞留）、**预设页 ❌**（横幅与表格「已应用」停在启动快照）。
- 严格判据（当前可见页、不点任何东西跟着变）：四页全 ❌ —— 底账变化没有任何推送通道（后端唯一 `emit` 是软件更新进度，`ipc/update.rs`）。

**顺带钉出的两个滞留 bug（挂在 A3 下，与读链同根，随收口一并消灭）**

「删除放开」让后端在删除时顺手撤掉使用中指针，但预设页删完没有重读底账：

1. `usePresetData.remove`（删用户文件）：只重读用户线 —— 删掉使用中的那份后，横幅继续说「正在使用 X」，而文件已经不存在。
2. `usePresetData.removeRelease`（删官方交付文件）：只重读官方线 —— 同病。

（同页 `rename` 的「写后重读」是做对了的 —— 这是纪律执行不一致，不是不知道纪律。另有两处注释已随「删除放开」失真，随施工一并改：`contract.ts` 的 `deleteUserPreset` 仍写「正在使用的不许删」、`usePresetData.ts` 删除注释同款 —— 后端实际已是「撤指针 + 丢草稿，一律放行」。）

## A3 读链收口施工（✅ 已完成，2026-10-06 第三轮）

> **实施记录**：最终方案比本节当初的设想又上移了一层 —— 不是给 `active-preset.json` 配一个订阅层，
> 而是按作者裁决**推翻「一种状态一个文件」**：三态并进 `run/app-state.json`，唯一读写入口
> `runtime/app_state.rs`（整份读-改-写 + 进程内互斥 + 惰性迁移），每个写命令成功后
> emit `app-state-changed`，TS 唯一客户端（`src/app/state/appState.ts`）接事件重读广播，
> 页面 `useActivePreset()` 订阅派生。决策全文见 [`APP-STATE.md`](APP-STATE.md)。下面是当初的设计条目，全部按此落地：

原则（作者定）：**先把「读取/订阅层」设计钉死，再一次性拆掉回页签对账 —— 不往现有同步方案上打补丁。**

1. **唯一读取/订阅层**（很轻，不引状态库）：全应用只有一处调 `getActivePreset()`，结果进模块级缓存 + 订阅，页面改订阅派生。`apply` / `clear` / `rename` / 删除撤指针这几类写完成后统一「重读底账 → 更新缓存 → 通知订阅者」（把预设页 `rename` 已有的纪律推广到所有写）。
2. **拆掉对账机制**：`useActivePresetOnTab`、首页 / 校准页回页签 `selectionFromActive` 反填、参数页回页签弹回 combo —— 全部退役；预设页横幅降级成普通订阅者。
3. **Active 与 Editing 解耦**（本轮验收照出来的设计问题）：「回页签强制把 combo 拉回 Active」把两个本应独立的概念绑死了。收口后 `active-preset` 只喂「正在使用」显示、`editingTarget` 只喂「正在编辑」，互不篡改。**回页签还弹不弹回 combo = 作者要拍板的点**（不弹回：离开再回来，还能接着编辑另一份；弹回：现状语义）。
4. （可选增强）`write_active` / `clear_active` 之后 Rust `emit` 一个事件、订阅层接住 —— 这是「带外变化（手改底账 / 将来多窗口）不点任何东西就跟着变」的唯一通道。
5. 上面两个滞留 bug 随第 1 条自然消失；注释失真随施工一并改；复验点见「验收判据」的 A3 收口条目。

## A1 实施（= PRESET-COPY-LINEAGE.md §3.1 + §5）

**后端（Rust）**
1. `runtime/mine.rs` 新增 `copy_release_as_new(root, user_root, file_name, new_name)`：
   目录里得有它且是 MKP 预设 → `delivery::official_text` 取**可信字节**（SHA 闸在里面，与「改这份」同一条边界）→ 名字过 `check_new_name` 同一套门槛 → `lineage::make_copy` 写血统三行（`based_on` = 来源交付文件的 `catalog.path`）→ 落 `presets-mine/<new_name>`。不覆盖、不自动改名、**一个状态都不碰**、出处账不记（血统已经答了"从哪来"）。
2. `ipc/mine.rs` 命令 `copy_release_as_new` + `lib.rs` 两份清单注册；带单测（血统头在、官方原件字节不动、撞名拒、非 TOML 拒、没下载拒）。

**前端（TS / React）**
3. 契约 `copyReleaseAsNew(fileName, newName)` + bridge + mock（演示口径：`A1-standard.toml` 可复制；旧版本 / 内容异常照实拒）+ `usePresetData.copyReleaseAsNew`（成功后重读用户线）。
4. `PagePresets.tsx`：`copyWhyNot` 放行 `origin === 'release' && kind === 'mkp_preset' && releaseState === 'ok'`；`submitNaming` 按 `row.origin` 分流到新命令；成功 toast 说清"官方原件一个字节没动、身世跟着来源走"；命名抽屉的说明文字按来源分口（官方版：复制到你的目录，身世跟着它走）。
5. `presetTree.ts`：`COPY_RELEASE` 文案、人话化的 `NO_ASSET_WHY`。

**探针**
6. `scripts/probes/presets.mjs` §5k-① 改判据：官方 `A1-standard.toml` 的「另存为一份新的」**能点**、走完命名抽屉、新行出现；内容存疑的两份**仍然灰**且带原因。

## A2 实施

1. `presetTree.ts`：`NO_ASSET_WHY` → "这份文件不在 SupportEase 的交付目录里，不能直接应用或复制。想用它：把它拖进窗口导入成「我的文件」，或从云端重新下载官方版。"（不再断言"你自己放的"，不再出现 asset id / 契约 / 开发者牢骚）。
2. `PresetTable.tsx`：官方行的「应用」按钮要求 `releaseUid` 在——没有交付身份的官方 MKP 行（演示数据的旧资产行）画灰杠 + 人话原因；`PagePresets.tsx:889` 的守卫退回兜底位（文案同步人话）。
3. `errors.ts`：`NotImplementedError(method, hint?)` —— 界面消息用人话 hint（或"这一步在这一版里还没有"），`[API] 未实现的接口: xxx` 只进控制台；mock 里所有裸抛补 hint（下载 / 删交付 / 删归档 / 读正文 / 远端检查 / 官方应用）。
4. `presetTree.ts`：`notImplementedText` → "浏览器预览做不了这一步，用桌面版再试"；`noContractText` → "这个动作还没有对应的实现"。`PagePresets.download` 删掉按异常类型拼术语的分支，统一走 `errorText`。

## 验收判据（对应原报告的故事 3 / 1 / 10）

- 故事 3 复验点：右键官方 `A1-standard.toml` →「另存为一份新的」可点 → 起名 → 「我的文件」多一份、`看正文` 头三行血统指向来源、官方原件还在且状态不变；撞名被拒且说人话。
- 故事 1 复验点：任何失败提示不再出现 asset id / API / 契约字样；灰按钮悬停能说清"为什么不行 + 换哪条路"。
- 故事 10 复验点：应用「我的 A1 涂胶.toml」后，首页设备卡、校准页 pill、参数页 pill 与预设页横幅说的是**同一个文件名**；参数页确认框不再说"写入官方文件"。
- A3 收口复验点（2026-10-06 第二轮钉下，读链施工完成后跑）：① 手改 `run/active-preset.json` 指向另一份文件、**不碰任何界面**，四个页面自然跟着变（唯一真相源判据）→ **程序内的写已做到"不点任何东西同帧换账"**（事件广播）；带外手改按 [`APP-STATE.md`](APP-STATE.md) §7 的开放点在下一次读取时生效（要不要文件监听待作者定）；② 预设页删除「使用中」的用户文件 / 官方交付文件，横幅与表格立即回「未应用」✅（两个滞留 bug 随 `appStateMutated` 消灭）；③ `useActivePresetOnTab` 与三处回页签对账 effect 已退役 ✅（`selCache`、参数页回页签弹回一并拆掉）。
- 回归：`npm run lint`、`npm run build`、`node scripts/probes/presets.mjs`（更新后）、`params-settings.mjs`、`home-flow.mjs`、`tabs.mjs` 全绿；`cargo test` + `cargo clippy -- -D warnings` 全绿。
