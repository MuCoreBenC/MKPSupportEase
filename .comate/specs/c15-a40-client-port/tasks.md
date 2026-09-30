# 任务清单：客户端 A40 全量 + 工作台 C15 增量（产品仓）

> 设计契约（**doc**）在仓库根：`C15-A40-PORT-PLAN.md` —— 页面对照表、C15 逐字差量、
> 契约与数据来源、分阶段计划、已裁决表、风险与移植规矩都在那里面。
> 本文件只管**进度与验收证据**：每一格是"做完了没有 + 凭什么说做完了"。
>
> 分支：`feat/b06-c15-a40-port`（自 `feat/b05-14b-c14-port` 的 P5 收尾点开出）。
> 作者裁决（2026-09-30）：首页/校准换成 A40 版 · BBS 不搬静态文件、页内改读本机目录 ·
> 报告/设置本轮做空态 · 猫不要 · 可用 `.mjs` 探针 · 键名待定（集中一处）· 本轮只做前端。
>
> **源仓是活靶子**：起草时 `64738aa`，2026-09-30 复核已是 `e509255`（T16 落地：切片器改吃真文件，
> 动了契约 / 假后端 / **预设页** / `public/presets/bbs`）。规矩：每阶段开工前先看一次试验场 HEAD，
> 按当时那一版取源；已落地的按新 HEAD 核一遍。**契约一致性已核**：我们 37 个类型里 35 个与试验场
> 逐字相同，唯一不同的是 `MkpApi`（刻意裁的 38 → 16 个方法），未搬的 16 个类型全是配方/套餐/
> 回退/测试端那一类。

---

## 阶段账

| 阶段 | 内容 | 状态 | 验收证据 |
|---|---|---|---|
| **P1** | 8 页签骨架 + 字段控件层 + 契约扩容（16 方法） + `NotImplementedError` + `storageKeys` + tokens 补 danger/warn | ✅ 完成（`2372775`） | `npx tsc -b` ✓ · `npm run lint` ✓ · `npm run build` ✓ · 探针 `scripts/probes/tabs.mjs` → 8 页签全开、0 条 console error |
| **P1b** | **假后端数据层**：`src/api/mockServer/**`（试验场 `src/server/` 的客户端那半）+ `mock.ts` 接线，替掉 P1 的空夹具 | ✅ 完成（见下条提交） | `tsc -b` ✓ · `lint` ✓ · `build` ✓（136 模块）· 数据独立复核：参数 74 / 带条件 43 / 条件参数 11 / 已弃用 7 · 机型 6 / 版本 10（5 台有尺寸）· 资产 20（11 bbs + 9 mkp）· 仓库条目 9 · 套餐 9。`getPresetFiles` 20 与试验场发布包的 9 不同属预期（20 是整个资产库，6 份 `optional`） |
| **P1c** | **客户端骨架**：`src/app/{home,calib,ui,shared,store}/`（A40 的首页 + 校准 + 两套共用件 + 同步层），删掉旧 v023 那一簇 | ✅ 完成（`e6b85b9` + 收尾提交） | `tsc -b` ✓ · `lint` ✓ · `build` ✓（136 模块）· 探针 `tabs.mjs` 8 页签全开 0 error · 探针 `home-flow.mjs`：首页欢迎 → 点「现在开始」进选择页（品牌 → 机型 → 版本分级揭示）→ 点「回主页」回摘要卡，**大图 `/assets/printers/p1s.webp` 挂上且解码成功（naturalWidth 1600）**；校准页板子 + 11 热点 + 预设下拉列出新数据层那 9 份 MKP 预设。键名全走 `storageKeys` · `TraceTag` 已接回 · 产品壳四件未动。<br>**收尾三件**：`heroArt.ts` 的图片路径按本仓目录重写（试验场那五条在本仓一律 404，选到机型才会露出来）· `shared/useStickyState` 改名 `useSessionState`（与 `src/hooks/useStickyState` 同名不同命）· `mock.ts` 删掉已无消费者的 `presetCatalog`（假数据从 api 层漏进页面那处彻底收口） |
| **P2** | 预设页（`presets/` 12 文件）：MKP/切片器 × 本地/云端两张表、两排工具栏、已应用状态条、喷嘴/层高筛选、点行展开、右键菜单、跳 BBS。**取源按 T16 之后那一版**（切片器改吃真文件、`statFrom` 分真值/演示值） | ✅ 完成（见下条提交） | `tsc` ✓ · `lint` ✓ · `build` ✓（152 模块）· 探针 `presets.mjs`：两轴可点、**四张表真的在切**（MKP 6 列含版本 / 切片器 7 列含喷嘴+层高）、点行展开（dl 0→1）、右键菜单七项出得来、**跨页那条通**（切片器行 →「在 BBS 查看器中打开」→ tab 切到 BBS 空态）；0 console error / 0 ≥400。计数逐个对上：仓库 20 · 本机 4 + 我的 3，且「共 N 项」是筛后真计数（1/3/3/3）。截图 ultra+compact 在 `tmp-shots/` |
| **P3** | 参数页（`params/` 19 文件）：分类条、卡片、行、抽屉、G-code 块、搜索（命中带父子）、快捷键、撤销/重做 | ⬜ 待办 | 与 A40 README 第三十三～四十轮实测条目逐条复量 |
| **P4** | 同步页（`pages/PagePackage` + `store/package` 已在 P1c 落地）：说明书自动同步、三态分开、空态；**报告/设置本轮只做空态** | ⬜ 待办 | 三态不串；空态写"本版未接入" |
| **P5** | BBS 预设页（`bbs/` 27 文件）：**不搬静态快照**，改读本机 BBS 目录（dev 走 dev-server 中间件、真机走 IPC） | ⬜ 待办 | 有本机目录时出清单；没有时是空态 |
| **P6** | 工作台 C15 增量 + **联动端到端**：工作台生成 → 上传云端 → 客户端同步 → 下载 → 应用 | ⬜ 待办（地基已落：`src/workbench/{compat,cloud}.ts`） | 一条链走通（照 T7.7 口径量数） |
| **P7** | 工作台参数台的模式开关整卡收起 + 抽屉说清类型（C15 的 A2/B1） | ⬜ 待办 | C15 `c15.module.css` / `derive.ts` 那几条版面判据复量 |
| **P8** | **与试验场对齐复核**：试验场是活靶子（T16 之后 HEAD `e509255`），每阶段开工前看一次它的 HEAD，已落地的按新 HEAD 核一遍 | ⬜ 持续 | 契约 37 类型已核：35 个逐字相同，仅 `MkpApi` 按产品口径裁过 |

> P7 与计划里的序号不同：**首页/校准的替换已在 P1c 提前做掉**（原计划是 P7）——
> 理由是 A40 的 `ui/` 与产品仓 v023 的 `ui/` 是同一套东西的两代，放着不动会出现两套并存的补丁。
> 剩下的 C15 版面增量（模式开关整卡收起、抽屉类型）是**工作台**侧的事，与客户端无关，因此留作 P7。

---

## 各阶段的依赖（按 T16 之后那一版量过，免得开工才发现缺件）

| 阶段 | 需要的外部件 | 需要的自家件 | 结论 |
|---|---|---|---|
| **P2** 预设页 | `api` ✓ · `components/field` ✓ · `components/menu` ✓ · `hooks/useDensity` ✓ | `shared/OriginChip` ✓ · `shared/useStickyState` ✓ · `store/package` ✓（都在 P1c 落地） | **零缺件**，骨架一完就能开 |
| **P3** 参数页 | `api` ✓ · `components/field`（含 `FieldControl` / `FieldPopover` 深路径）✓ · `hooks/{useDensity,useStickyState}` ✓ | `shared/Drawer` ✓ · `shared/useDrawerWidth` ✓ · `store/package` ✓ · **`shell/iconsA40` ✗ 没搬** | **要补一件**：`shell/icons.tsx`（分类条那六个手画图标就在它里面）。`shell/TopTabs` 仍然不搬（见债 #5），只取图标表 |
| **P4** 同步页 | `hooks/useDensity` ✓ | `store/package` ✓ · `shared/note` + `NoteBar` ✓ · `ui/{Controls,Modal}` ✓ | 零缺件。**只搬 `pages/PagePackage`**；`PageSettings` 与 `report/` 按裁决做空态，它们要的 `mockA40` 手编数据不搬 |
| **P5** BBS 页 | `hooks/useDensity` · `hooks/useStickyState` ✓ | `shared/{useDrawerWidth,useStickyState}` ✓ | 零缺件，自包含；数据改读本机 BBS 目录（不搬 285 个静态文件） |

---

## 在飞（2026-09-30 22:01 记，四条并行、文件互不相交）

> 这一节是给"接手的人/下一个会话"看的：下面四件**都还没提交**，工作树里有它们的产物。
> 规矩 #9：并行阶段一律不动 `App.tsx`（唯一例外是 P2 被授权接 `case 'preset'`，已接完）。

| 线 | 目录/文件 | 当时进度 | 交付后要核的三件 |
|---|---|---|---|
| **P2 预设页** | `src/app/presets/`（12 文件已落）+ `src/app/shared/OriginChip.*` + `App.tsx` 的 `case 'preset'` + `storageKeys.ts` 加一格 `clientPresetsPinned`（值未改，合规） | 12 文件已在位，在逐文件做导入/注释遍与自检 | ① 三闸 ② 两轴（MKP/切片器 × 本地/云端）与点行展开、右键菜单 ③ 偏差清单里有没有"造数据" |
| **P4 同步页** | `src/app/pages/`（3 文件：`PagePackage.tsx` + 两个 CSS） | 已落文件，**没接线**（按规矩 #9，`case 'sync'` 由我接） | ① 三闸 ② 三态不串 ③ 缺说明书 / 指纹不一致两条分支 |
| **P3 参数页** | `src/app/params/`（19 文件） | 刚起手（先报了"云端为空"那条发现） | ① 三闸 ② 只吃说明书、不做继承推导 ③ 无说明书时是诚实空态 |
| **C15 工作台增量** | `src/workbench/{views/BuildPage,views/ParamsPage,views/ParamDetail}.tsx` + `c14.module.css` + `dev/mockBackend.ts` | 正在改（5 个文件） | ① 三闸 + `build:workbench` ② 参数台整卡收起读的是**已有的行状态**、没新增领域规则 ③ 自动判断按钮**不渲染**（输入不存在，债 #8） |

接线顺序（我来做）：P2/P3/P4 都落地后，一次性改 `App.tsx` 的 `case 'params'` / `case 'sync'` 与 `bbs` 的 `pendingBbs` 消费，然后跑两档尺寸截图；再把四条各自提交（一阶段一提交，便于回溯）。

---

## 已登记的技术债与待决

| # | 事 | 现状 | 归宿 |
|---|---|---|---|
| 1 | **C15 的两个判定**（生成说明书 / 最低客户端版本） | 本轮住前端：生成侧随 P6 接，兼容性清单已落 `src/workbench/compat.ts` | 搬进 Rust（产品纪律「前端不算业务」；与 `words.rs` 词表同源） |
| 2 | **localStorage 键名** | 全部集中在 `src/api/storageKeys.ts`，名字沿用试验场（`mkp.a40.*` / `mkp.cloud.presets`） | 作者定名后改这一处 |
| 3 | **假后端进生产包** | `api` 是**运行时**探测（浏览器走 mock、Tauri 走 invoke），所以假后端会进用户包体积 | 真夹具落地后量一次；必要时给 mock 加动态 import 或按 `import.meta.env.DEV` 分两份 |
| 4 | **`npm run dev` 被文件监听拖死** | `target/` 两棵构建树 5.8 万文件，watcher 一直在爬，HTTP 全超时；验收改用 `build` + `preview`（4173） | 给 `vite.config.ts` 加 `server.watch.ignored`（**产品配置，等作者点头**） |
| 5 | **A40 顶栏没有照搬** | `shell/TopTabsA40` 画假红绿灯、窗口键是空按钮、无拖窗处理（原型里归预览器外壳） | 保留产品仓 `TopTabs`（真窗口 chrome）；只用 A40 的图标集的话另开一小步 |
| 6 | **`public/cloud/presets.json`**（决定改判：**搬**） | 原写"不搬"（我判断那是试验场按自己假数据生成的「别人发过的包」）；P2 提出反对并逐字节拷入（sha256 核对、HTTP 复验为 `200 application/json`）。**改判成立**：作者在 C15 spec 里明确「静态快照（云端初始内容）＋ localStorage（我刚上传的）两条都要」，且该快照与我们的 `mockServer` **同源同一批上游 JSON**（6 机型 / 10 版本 / 74 字段 / 9 份预设文件），不是外来数据 | 它只是"云端初始内容"的种子，**不是我们的发布**；将来自研时用我们自己的工作台生成一份替换。副作用：预设页「MKP 配置 / 云端」由 3 行「官方」变成 3 行「工作台发布 → 下载」（release 行接管官方行，T12 规矩）。**有数据那一态仍以 P6 的真链路验收为准** |
| 7 | **报告/设置两页的手编数据不搬**（作者裁决） | 两页本轮就是空态 | 接业务逻辑时一起做 |
| 8 | **工作台侧拿不到 `ClientDataPackage`**（本轮实测） | `grep ClientData\|ClientField\|Package\|minClient\|schemaVersion\|inputsHash\|说明书\|preset.toml` 扫 `src/workbench/api.ts` → **零命中**；那一族类型只在 `src/api/contract.ts`，是**客户端**的契约。工作台的两个版本轴是从上游 manifest 的 compat 声明读进来的（所以 ② 卡能说「未声明——上游 manifest 没写」） | 所以「最低客户端版本**自动判断**」这个按钮本轮**不渲染**（输入不存在；按纪律不许在前端造一份包去喂 `minClientOf()`）。它的输入（说明书生成）与清单一起搬进 Rust —— 与债 #1 同一笔 |
| 9 | **`App.tsx` 是串行瓶颈** | 每个页签都要在 `App.tsx` 的 switch 里接一次，而两个 agent 同时改同一个文件必然打架 | 规矩：**并行阶段的任务一律不许动 `App.tsx`**，各页只落自己的目录；接线由我在它们都落地后一次性改（P3 与 P5 会按这条并行开工） |
