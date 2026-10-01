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
> 按当时那一版取源；已落地的按新 HEAD 核一遍。**契约一致性已核（2026-09-30 实测重核）**：我们 37 个
> 类型里 **34 个与试验场当前 HEAD 逐字相同**（剥掉注释、折叠空白后比对），第 35 个是刻意裁过的
> `MkpApi`（38 → 16 个方法），另 2 个是本仓独有（`ErrorCode` / `AppError` —— 产品仓的错误码那一档）；
> 试验场独有 16 个类型全是配方/套餐/回退/测试端那一类。

---

## 阶段账

| 阶段 | 内容 | 状态 | 验收证据 |
|---|---|---|---|
| **P1** | 8 页签骨架 + 字段控件层 + 契约扩容（16 方法） + `NotImplementedError` + `storageKeys` + tokens 补 danger/warn | ✅ 完成（`2372775`） | `npx tsc -b` ✓ · `npm run lint` ✓ · `npm run build` ✓ · 探针 `scripts/probes/tabs.mjs` → 8 页签全开、0 条 console error |
| **P1b** | **假后端数据层**：`src/api/mockServer/**`（试验场 `src/server/` 的客户端那半）+ `mock.ts` 接线，替掉 P1 的空夹具 | ✅ 完成（见下条提交） | `tsc -b` ✓ · `lint` ✓ · `build` ✓（136 模块）· 数据独立复核：参数 74 / 带条件 43 / 条件参数 11 / 已弃用 7 · 机型 6 / 版本 10（5 台有尺寸）· 资产 20（11 bbs + 9 mkp）· 仓库条目 9 · 套餐 9。`getPresetFiles` 20 与试验场发布包的 9 不同属预期（20 是整个资产库，6 份 `optional`） |
| **P1c** | **客户端骨架**：`src/app/{home,calib,ui,shared,store}/`（A40 的首页 + 校准 + 两套共用件 + 同步层），删掉旧 v023 那一簇 | ✅ 完成（`e6b85b9` + 收尾提交） | `tsc -b` ✓ · `lint` ✓ · `build` ✓（136 模块）· 探针 `tabs.mjs` 8 页签全开 0 error · 探针 `home-flow.mjs`：首页欢迎 → 点「现在开始」进选择页（品牌 → 机型 → 版本分级揭示）→ 点「回主页」回摘要卡，**大图 `/assets/printers/p1s.webp` 挂上且解码成功（naturalWidth 1600）**；校准页板子 + 11 热点 + 预设下拉列出新数据层那 9 份 MKP 预设。键名全走 `storageKeys` · `TraceTag` 已接回 · 产品壳四件未动。<br>**收尾三件**：`heroArt.ts` 的图片路径按本仓目录重写（试验场那五条在本仓一律 404，选到机型才会露出来）· `shared/useStickyState` 改名 `useSessionState`（与 `src/hooks/useStickyState` 同名不同命）· `mock.ts` 删掉已无消费者的 `presetCatalog`（假数据从 api 层漏进页面那处彻底收口） |
| **P2** | 预设页（`presets/` 12 文件）：MKP/切片器 × 本地/云端两张表、两排工具栏、已应用状态条、喷嘴/层高筛选、点行展开、右键菜单、跳 BBS。**取源按 T16 之后那一版**（切片器改吃真文件、`statFrom` 分真值/演示值） | ✅ 完成（`2998433`） | `tsc` ✓ · `lint` ✓ · `build` ✓（152 模块）· 探针 `presets.mjs`：两轴可点、**四张表真的在切**（MKP 6 列含版本 / 切片器 7 列含喷嘴+层高）、点行展开（dl 0→1）、右键菜单七项出得来、**跨页那条通**（切片器行 →「在 BBS 查看器中打开」→ tab 切到 BBS 空态）；0 console error / 0 ≥400。计数逐个对上：仓库 20 · 本机 4 + 我的 3，且「共 N 项」是筛后真计数（1/3/3/3）。截图 ultra+compact 在 `tmp-shots/` |
| **P3** | 参数页（`params/` 19 文件）：分类条、卡片、行、抽屉、G-code 块、搜索（命中带父子）、快捷键、撤销/重做 | ✅ 完成（产物在工作树，待提交）+ 已接线（`case 'params'`） | 19 文件 / 4,765 行；**页面零 api 值导入** —— 每个字（页签 / 分组 / 条数 / 类型 / 选项 / 条件 / 值）只来自同步下来的说明书；逐字一致按前后缀比对：10 个代码文件 7 个零差异，CSS 忽略全部空白后全等。三闸 ✓ · 探针 `params-sync.mjs` 两档尺寸：没包时诚实空态 → 互跳同步页 → 回来是包里的真内容（分类 6，74 条摊成 5/27/3/19/2/18），0 console error / 0 个 ≥400 |
| **P4** | 同步页（`pages/PagePackage` + `store/package` 已在 P1c 落地）：说明书自动同步、三态分开、空态；**报告/设置本轮只做空态** | ✅ 完成（`65501a9`）+ 已接线（`case 'sync'`） | 三态分开（`autoSync` 的 empty / synced / upToDate，判据是 `inputsHash`，不是版本号）；报告 / 设置两页仍是占位（写"这一页本版未接入"）。探针 `params-sync.mjs` 两档尺寸：自动同步出说明书 + 摊出布局与模式开关 + 「去看参数页」互跳回来有真内容 |
| **P5** | BBS 预设页（`bbs/` 27 文件）：**不搬预设快照**，改读本机 BBS 目录（serve 期只读端点，dev + preview 都挂）；**元数据四份随包分发**（改判，见下） | ✅ 完成（产物在工作树，待提交）+ 已接线（`case 'bbs'`） | 27 文件 / 4,156 行；**逐字一致按前后缀比对**（去稿号 + import 路径 + `useStickyStateA40`→`useSessionState` 归一后）：**27 个里 20 个零差异**，改动的 7 个正是「不搬快照」的连带（`bbsTypes` / `bbsSource` / `useBbsData` / `useBbsPreset` / `BbsStatusBar` / `PageBbs` / `useBbsDrawer`） · `public/bbs/` 四份元数据 514.7 KB（来源与许可写在同目录 `PROVENANCE.md`）· 端点 `tools/dev-server/bbsFs.mjs`（`configureServer` + `configurePreviewServer` 共用一段 handler，安全边界照搬试验场）· 三闸 ✓ · `build:workbench` ✓（267 模块）· 探针 `scripts/probes/bbs.mjs` 两档尺寸 **24/24**：状态条「本机 BBS · 用户 29 / 系统 233」（278 份那一档）、抽屉三组（用户 13 / 系统 10 / 其他 16）、默认选中一份且面板真画出 68 行 / 5 个分类页签、跨页那条通（切页 + 目标对不上时如实说一句）、**端点桩成「没装 BBS」→ 重扫 → 诚实空态（0 行）**；0 console error / 0 个 ≥400。`tabs.mjs` 复跑：8 页签全开、BBS 预设已是「真页面」、报告/设置仍占位 |
| **P6** | 工作台 C15 增量（§3）+ **联动端到端**：工作台生成 → 上传 → 客户端同步 → 下载 → 应用 | ✅ 完成（产物在工作树，待提交） | **工作台侧**：② 卡变成**真说明书**（`src/workbench/clientPackage.ts` 现装 —— 「前端算业务」边上那个**已登记的例外**，见债 #1/#8）· 「最低客户端版本 · 自动判断」给出数（木桶原理，实测 `1.1.0`）· 「上传到云端」真写 `STORAGE.cloud`（整份 release：说明书 + N 份 TOML，同属一个 preset identity）· 三闸 ✓ · `build:workbench` ✓（268 模块）。<br>**端到端**：新探针 `scripts/probes/chain.mjs`（同源两个入口、同一个 localStorage）两档尺寸 **22/22** —— 工作台报 2 机型 / 3 版本 / 13 字段（带条件 5），上传 2 份 `MKPProcess_A1_*.toml`；客户端同步下来那份**指纹相同**（`63d9c773`）、报的数与工作台逐字一致；云端表出「工作台发布 · 1.0.0」两行 → 点「下载」变「已下载」→ 本地表点「应用」变「已应用」，唯一底账写 `{"kind":"release","ref":"A1/FAST"}`；0 console error / 0 个 ≥400 |
| **P7** | 工作台参数台的模式开关整卡收起 + 抽屉说清类型（C15 的 A2/B1） | ✅ 完成（产物在工作树，待提交） | 判据全来自后端已给的每行 `blocked` / `blockedHint`：切到「圆盘擦拭」→ **两张卡整卡收起**（section 级那句「『擦料方式』选了，这一组 2 项现在不生效」+ 字段级那句「要 擦料方式 等于 擦料塔 才可改」各一），收起后那几行**不在页面上**（不是灰行），点「仍然展开看」两处都摊得回来；抽屉「基本信息」里有值类型 / 控件，「技术信息」只剩注册键。探针 `workbench-build.mjs`（自带桩的那份构建 + 4174）**全绿**，且**两档尺寸各走一遍**（ultra 1760×900 与窄档 900×640 另起一段 reload 后重量：整卡收起、收起后无行、抽屉类型三样都在） |
| **P8** | **与试验场对齐复核**：试验场是活靶子（T16 之后 HEAD `e509255`），每阶段开工前看一次它的 HEAD，已落地的按新 HEAD 核一遍 | ✅ 本轮复核完成（持续项保留） | 2026-09-30 **实测重核**，四笔：① **契约** —— 37 个类型里 35 个与试验场同名，剥注释 + 折叠空白后 **34 个逐字相同**；唯一不同的是刻意裁过的 `MkpApi`（38 → 16 个方法）；本仓独有 `ErrorCode` / `AppError`；试验场独有 16 个类型全是配方/套餐/回退/测试端那类。<br>② **假后端夹具** —— 7 份 JSON 里 **6 份 sha256 逐字节相同**；`bbs_files.json` 只差一行生成时刻（`syncedAt` 13:39:13 vs 13:24:43，同一脚本两次跑的产物，其余逐行相同）。<br>③ **T16 那一刀**三处都落地且**真被消费**：`PresetFileInfo.statFrom`（契约逐字相同）→ `mockServer/resources.ts` 按它分真值/演示值 → `presetTree` / `PresetTable` 读它；`resolve/bbsFiles.ts` + `data/bbs_files.json` 都在；`machine_catalog.json` 的套餐指向逐字节相同。<br>④ **HEAD 复核** —— 仍是 `e509255`（无新增量），各阶段「已落地」由各阶段探针的**行为**判据背书，不靠字面比对。<br>另：A40 各目录文件数逐个对上（bbs 27 / params 19 / presets 12 / home 25 / calib 12 / shared 9 / ui 3 / store 1）；`pages` 4→3、`shell` 3→1、`report` 2→0 都是裁决范围内的刻意裁剪 |

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
| **P5** BBS 页 | `hooks/useDensity` · `hooks/useStickyState` ✓ | `shared/useDrawerWidth` ✓ · `shared/useSessionState` ✓（就是搬来的 `useStickyStateA40`，P1c 已按职责改名） | 页面零缺件、自包含。**另外两件要自己开**：① 清单源 `tools/dev-server/bbsFs.mjs`（serve 期只读端点，本仓新写，安全边界照搬试验场）；② 元数据 `public/bbs/{registry,layout,defaults,icons}.json` —— 预设快照 285 个不搬，但这四份**得随包**（见下方改判） |

---

## 收线记录（2026-09-30 记，四条并行 → 三条已收 + 接线已做）

> 这一节是给"接手的人/下一个会话"看的：四条的产物都在工作树里，P2 / P4 已有提交，
> P3 与 C15 增量的文件还没提交。规矩 #9：并行阶段一律不动 `App.tsx` —— P3 / P4 两页
> 已由主线**一次性接上**（见下面那条）；**P5 是接线之后单开的一块**，一上来就自己接
> （`case 'bbs'` + 消费一直闲着的 `pendingBbs`）。

| 线 | 产物 | 现状 |
|---|---|---|
| **P2 预设页** | `src/app/presets/`（12 文件）+ `src/app/shared/OriginChip.*` + `App.tsx` 的 `case 'preset'` + `storageKeys.ts` 的 `clientPresetsPinned` | ✅ 已提交 `2998433`；探针 `presets.mjs` 过了（两轴四表在切、点行展开、右键菜单、跳 BBS） |
| **P4 同步页** | `src/app/pages/`（3 文件） | ✅ 已提交 `65501a9`；接线后与 P3 互跳，探针 `params-sync.mjs` 两档尺寸 16/16 |
| **P3 参数页** | `src/app/params/`（19 文件，未提交） | ✅ 交付 + 已接线（`case 'params'`）；三闸 ✓ + 探针 16/16（两档尺寸 0 console error） |
| **P5 BBS 预设页** | `src/app/bbs/`（27 文件 4,156 行）+ `tools/dev-server/bbsFs.mjs`（+ `bbsFs.d.mts`）+ `public/bbs/`（四份元数据 514.7 KB + `_sync.json` + `PROVENANCE.md`）+ `vite.config.ts` 挂插件 + `App.tsx` 的 `case 'bbs'` + `storageKeys.ts` 四格 BBS 偏好 + 探针 `bbs.mjs` | ✅ 交付 + 已接线；三闸 ✓ · `build:workbench` ✓（267）· 探针 24/24（两档尺寸，含「桩掉端点 → 诚实空态」那一档） |
| **C15 工作台增量** | `src/workbench/{views/BuildPage,views/ParamsPage,views/ParamDetail}.tsx` + `c14.module.css` + `dev/mockBackend.ts` + **新文件 `clientPackage.ts`** + 探针 `workbench-build.mjs` / `chain.mjs` | ✅ 交付（工作树待提交）；`build:workbench` ✓（268）· 探针要自带桩的那一份构建（命令链写在 `workbench-build.mjs` 头部，用 4174 那台）—— **两个探针都跑过**：`workbench-build.mjs` 全绿、`chain.mjs` 两档 22/22（联动一条链） |

**接线（2026-09-30 完成）**：`App.tsx` 的 `case 'params'` → `PageParams`（`onOpenPackage` 跳同步）、
`case 'sync'` → `PagePackage`（`onOpenParams` 回参数）—— 互跳两程一条链走通；
`case 'bbs'` → `PageBbs`（`pending={pendingBbs}`，预设页右键带过来的**文件名**由那一页按自己的清单消费，
查不到就如实说一句 —— 两页的清单本来就不是同一份）。
证据：`node scripts/probes/{params-sync,bbs}.mjs`（先 `build` + 4173 静态预览）两档尺寸 16/16 与 24/24 全绿，
截图 `tmp-shots/wire-*.png`、`tmp-shots/bbs-*.png`。
各条的提交（一阶段一提交）还没做，等作者发话。

### P5 的两处改判（相对 `C15-A40-PORT-PLAN.md` §6-2 的字面）

| # | 字面 | 实际做的 | 理由 |
|---|---|---|---|
| 1 | 「不打包那 285 个静态快照」 | 285 个里**只有 5 个是元数据**（`registry` / `layout` / `defaults` / `icons` / `_sync`）—— 这 5 个**搬**；`system/` 279 + `samples/` 1 一律不搬 | 元数据是参数表的「骨」：`bbsPanel` 取不到 `registry[key]` 就把每一行都当 `noMeta` 跳过，点开一片空。真正叫「预设快照」的是另外那 280 个。**作者 2026-09-30 已确认**；许可已核 —— Bambu Studio 与产品仓同为 **AGPL-3.0**，同协议再分发成立，来源与许可写在 `public/bbs/PROVENANCE.md` |
| 2 | 「dev 走 dev-server 中间件、真机走 IPC」 | 本轮只做**服务端只读端点**，且 `configureServer` 与 `configurePreviewServer` **都挂**；真机 IPC 没做 | 债 #4：本机 `npm run dev` 被 `target/` 那两棵构建树拖死文件监听，验收一律走 `build` + `preview` —— 只挂 dev 的话端点根本摸不到，探针验不了「278 份那一档」。真机那一档登记成债 #11 |

**P5 还改了三处代码口径**（都不是重写，是「不搬快照」的连带）：
① `BbsSourceMode` 由 `'live' | 'snapshot'` 收成 `'live' | 'none'` —— 没有退档，读不到就是空态（连带删掉
`encodePathSegment` 与静态路径取单份预设那条路，那条路上的 `encodeURIComponent`/`%40` 坑一并没有了）；
② 清单**空了**（重扫没读到本机目录）时，`useBbsPreset` 把上一次那份连同文档一起放下、`PageBbs` 也不再按版面铺行
—— 不这么做会出现「一边写『没读到本机 BBS 预设目录』、一边照旧画着 249 行出厂默认」的自相矛盾（探针 ⑥ 抓到的）；
③ `'mkp.A40.bbs.*'` 四格键名收进 `src/api/storageKeys.ts`（债 #2 的规矩）。

---

## 已登记的技术债与待决

| # | 事 | 现状 | 归宿 |
|---|---|---|---|
| 1 | **C15 的两个判定**（生成说明书 / 最低客户端版本） | 本轮两件都已落前端、且联动真通：生成侧 = `src/workbench/clientPackage.ts`（P6 新写，**一处模块**）；兼容性清单 = `src/workbench/compat.ts`，界面上的「自动判断：1.1.0」就是它算的 | 搬进 Rust（产品纪律「前端不算业务」；与 `words.rs` 词表同源）。**这是一条已登记的例外**（`C15-A40-PORT-PLAN.md` §6「仍待定」那条建议）—— 搬过去那天界面一个字不动 |
| 2 | **localStorage 键名** | 全部集中在 `src/api/storageKeys.ts`，名字沿用试验场（`mkp.a40.*` / `mkp.cloud.presets`） | 作者定名后改这一处 |
| 3 | **假后端进生产包** | `api` 是**运行时**探测（浏览器走 mock、Tauri 走 invoke），所以假后端会进用户包体积 | 真夹具落地后量一次；必要时给 mock 加动态 import 或按 `import.meta.env.DEV` 分两份 |
| 4 | **`npm run dev` 被文件监听拖死** | `target/` 两棵构建树 5.8 万文件，watcher 一直在爬，HTTP 全超时；验收改用 `build` + `preview`（4173） | 给 `vite.config.ts` 加 `server.watch.ignored`（**产品配置，等作者点头**） |
| 5 | **A40 顶栏没有照搬** | `shell/TopTabsA40` 画假红绿灯、窗口键是空按钮、无拖窗处理（原型里归预览器外壳） | 保留产品仓 `TopTabs`（真窗口 chrome）；只用 A40 的图标集的话另开一小步 |
| 6 | **`public/cloud/presets.json`**（决定改判：**搬**） | 原写"不搬"（我判断那是试验场按自己假数据生成的「别人发过的包」）；P2 提出反对并逐字节拷入（sha256 核对、HTTP 复验为 `200 application/json`）。**改判成立**：作者在 C15 spec 里明确「静态快照（云端初始内容）＋ localStorage（我刚上传的）两条都要」，且该快照与我们的 `mockServer` **同源同一批上游 JSON**（6 机型 / 10 版本 / 74 字段 / 9 份预设文件），不是外来数据 | 它只是"云端初始内容"的种子，**不是我们的发布**；将来自研时用我们自己的工作台生成一份替换。副作用：预设页「MKP 配置 / 云端」由 3 行「官方」变成 3 行「工作台发布 → 下载」（release 行接管官方行，T12 规矩）。**有数据那一态仍以 P6 的真链路验收为准** |
| 7 | **报告/设置两页的手编数据不搬**（作者裁决） | 两页本轮就是空态 | 接业务逻辑时一起做 |
| 8 | **工作台侧拿不到 `ClientDataPackage`**（本轮实测） | `grep ClientData\|ClientField\|Package\|minClient\|schemaVersion\|inputsHash\|说明书\|preset.toml` 扫 `src/workbench/api.ts` → **零命中**；那一族类型只在 `src/api/contract.ts`，是**客户端**的契约。工作台的两个版本轴是从上游 manifest 的 compat 声明读进来的（所以 ② 卡能说「未声明——上游 manifest 没写」） | **P6 已补上产出者**：`src/workbench/clientPackage.ts` 从前端一处模块现装（读 `wb_*` 那几条读命令，装配成契约形状）。于是「最低客户端版本**自动判断**」不再是灰按钮 —— 它扫这份包给出数（实测 1.1.0）并把依据那句挂在 `title` 上。生成侧搬进 Rust 与债 #1 同一笔 |
| 9 | **`App.tsx` 是串行瓶颈** | 每个页签都要在 `App.tsx` 的 switch 里接一次，而两个 agent 同时改同一个文件必然打架 | 规矩：**并行阶段的任务一律不许动 `App.tsx`**，各页只落自己的目录；接线由我在它们都落地后一次性改（P3 与 P5 会按这条并行开工） |
| 10 | **BBS 元数据随包分发**（`public/bbs/` 四份 514.7 KB） | 改判落地（见收线记录）：预设快照 280 个不搬、元数据 5 个搬。许可已核 —— 与 Bambu Studio 同为 AGPL-3.0，来源与许可写在 `public/bbs/PROVENANCE.md` | 将来若改成**闭源**分发，这条路要重做（运行时从本机取或自研等价表）。BBS 出新版（2.7→2.8 参数 712→739）时要用 `machine-motion/tools/bbs-extract.mjs` 重跑一遍，并同步 `_sync.json` 的版本与字节 |
| 11 | **BBS 页在真机上取不到清单** | 本轮的清单源是**服务端只读端点**（`tools/dev-server/bbsFs.mjs`，dev + preview 都挂），Tauri 窗口里没有它 → `sourceMode: 'none'` → 诚实空态（探针 ⑥ 验的就是这一档） | 真机走 IPC 读 `%APPDATA%\BambuStudio\{,Beta}\{system,user\<uid>}\...\process`，与裁决 §8 的原话一致；本轮按「只做前端」的口径没做 |
