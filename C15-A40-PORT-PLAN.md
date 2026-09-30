# C15 / A40 → 产品仓移植方案（客户端 A21→A40 补齐 · 工作台 C14→C15 增量 · 两边联动）

> 起草：2026-09-30。起草时的两个仓：
> - 试验场 `mkp-adaptive-console` @ `64738aa`（main，C15 / A40 双活；T7 / T8 / T13–T15 已合并，PR #43 收尾）
> - 产品仓 `MKPSupportEase` @ `1544223`（`feat/b05-14b-c14-port`，14b 的 P1–P5 已闭环，工作树干净）
>
> **试验场是个活靶子**（2026-09-30 复核）：起草之后它又并了 T16（PR #44–#47，HEAD 现在是 `e509255`），
> 改动落在 `src/api/contract.ts`（`PresetFileInfo` 增 `statFrom`）、假后端（新增 `resolve/bbsFiles.ts`
> + `data/bbs_files.json`、`machine_catalog.json` 的套餐指向修正）、`src/versions/a40/presets/**`
> （**预设页那一版改了**：切片器改吃真文件、页面说实话）、以及 `public/presets/bbs/**`（真 BBS 预设快照）。
> 于是规矩定两条：**① 每阶段开工前先 `git log` 一次试验场，按当时 HEAD 取源**；
> **② 已落地的部分做一次一致性核对** —— 契约 37 个类型里 35 个与试验场逐字相同，
> 唯一不同的是 `MkpApi`（是我们刻意裁的 38 → 16），16 个没搬的类型全是配方/套餐/回退/测试端那一类。
> 本方案记的"差多少"仍是 `64738aa` 那次的口径；T16 之后预设页的增量按新 HEAD 重取。
>
> 本方案是**本轮施工图**：只做前端，不新增 Rust 命令；业务接入（真后端）留到下一轮。
> 体例照 `C14-PORT-PLAN.md`（同一套页面对照表 / 契约差异 / 分阶段 / 待裁决）。

---

## 0. 已定决策（2026-09-30 与作者确认）

| # | 决策 | 内容 |
|---|------|------|
| 1 | 范围 | **A40 客户端全量 + C15 工作台增量**，一次收口，不分两波 |
| 2 | 性质 | **本轮只做前端**：数据一律走 mock；native 窗口里未接入的方法报「未接入」，不白屏、不假装 |
| 3 | 路线 | **整体移植、不重写**（与 v023 / 14b 同一条原则）。试验场一个字不动 |
| 4 | 节奏 | 先出本方案 → 作者过目 → 再开工（P0）；之后每阶段一个 PR，走八道闸 |
| 5 | 联动 | 工作台「上传云端」→ `localStorage` → 客户端「同步」。两个入口同源，**本轮就要求真的联通** |
| 6 | 判据 | 本方案里所有「差多少」都是量出来的（口径见 §1.2），不凭感觉 |
| 7 | 首页 / 校准页 | **换成 A40 版**（作者 2026-09-30 裁决）；产品壳那几件（`ResizeEdges` / `useTitlebarDrag` / `window.ts` / `TraceTag`）保留 —— 换的是页面，不是窗口 |
| 8 | BBS | 页签与页面**保留**，但**不打包那 285 个静态快照**；改成**读本机 BBS 目录**（真机上本来就在那儿），读不到就是空态 |
| 9 | 报告 / 设置 | 本轮**直接做空态**（不搬 A40 里那份手编数据 —— 那是「接业务逻辑」时的事） |
| 10 | 猫 | 不需要（`public/cat-poses/` 21 文件不进仓） |
| 11 | 验收工具 | **可以用 `.mjs` 探针**（playwright + msedge channel，与试验场同一套手法）；验收不再只靠肉眼 |
| 12 | localStorage 键名 | **待定**：本轮集中写在一个模块里（`src/api/storageKeys.ts`），改名是一处的事；云端那一格先用 `mkp.cloud.presets`（工作台与客户端要读同一格） |
| 13 | 分支 | `feat/b06-c15-a40-port`（照 14b 的命名） |

---

## 1. 现状一句话

### 1.1 四行账

| | 产品仓（本仓） | 试验场 | 差 |
|---|---|---|---|
| **客户端**（用户端 UI） | v023 / **A21** 整份移植：6 个页签，只有首页 + 校准有内容，预设 / 参数 / 报告 / 设置是 `PagePlaceholder`。`src/app/` **55 文件 / 7,058 行**（另有公共件 `src/api` + `src/components` + `src/hooks` ≈ 560 行） | **A40**：8 个页签全部有内容。**120 文件 / 24,102 行**（.ts/.tsx/.css 口径） | **19 稿**（A22–A40），行数 ×3.4 |
| **工作台**（后厨工作台） | **C14** 移植（14b 的 P1–P5 已闭环，`src/workbench/` 63 文件 / 17,871 行，~40 条 `wb_*` 命令已通） | **C15**（38 文件 / 18,140 行） | **只差 2 个新文件 + 4 个改文件**（§3 是逐字 diff 的结果） |
| **契约** | `src/api/contract.ts` 98 行 / **4 个方法**（`getPreset` / `saveOffsets` / `getCalibModels` / `openModel`） | `MkpApi` 1,116 行 / 38 个方法；**A40 页面实际调用 16 个** | 客户端要补 12 个方法 |
| **页面数据** | 4 个命令 + 3 份手编预设 | 假后端：`src/server/` 2,790 行 TS + 7 个 JSON ≈ **123 KB**（`param_registry.json` 77 KB / 74 条参数、43 条带条件）；静态云端 `public/cloud/presets.json` 109 KB；`public/bbs/**` 285 文件 1.29 MB | 见 §4 |

**一句话结论**：差距的主体在**客户端**（A21 → A40 差了 19 稿：预设页、参数页、同步页、BBS 预设查看器、报告页、设置页全是空的）；工作台那半只差 C15 那一小截增量。

### 1.2 口径（可复核）

- 文件数与行数：`Get-ChildItem -Recurse`（`.ts/.tsx/.css`）+ 逐文件 `Measure-Object -Line`，两侧同一个脚本、同一次跑。
- C15 差量：把 `c15/` 的每个文件与 `c14/` 同名文件（文件名与内容里 `C15→C14`、`c15→c14` 归一后）逐字比较 —— **新增 2、修改 4、删除 0**。
- A40 页面调用的 api 方法：`grep -o 'api\.[a-zA-Z]*('` 去重。

---

## 2. 页面对照表（客户端 A40 → 产品仓）

### 2.1 八个页签

| A40 页签 | 文件 | 产品仓现状 | 动作 |
|---|---|---|---|
| 首页 `home` | `a40/home/` 25 文件 / 4,534 行 | 有（A21 版：`PageHome.tsx` 620 + 组件） | **整体替换成 A40 版**（P7）；产品壳那几件保留 |
| 预设 `preset` | `a40/presets/` 12 文件 / 4,481 行 | **无**（`PagePlaceholder`） | 搬（P2） |
| 校准 `calib` | `a40/calib/` 12 文件 / 2,206 行 | 有（A21 版） | **整体替换成 A40 版**（P7） |
| 参数 `params` | `a40/params/` 19 文件 / 4,899 行 | **无**（`PagePlaceholder`） | 搬（P3） |
| 同步 `sync` | `a40/pages/PagePackageA40.tsx` | **无** | 搬（P4） |
| BBS 预设 `bbs` | `a40/bbs/` 27 文件 / 3,772 行 | **无** | 搬（P5，另带 285 个静态文件） |
| 报告 `report` | `a40/report/` 2 文件 / 603 行 | **无**（`PagePlaceholder`） | 搬（P4） |
| 设置 `settings` | `a40/pages/PageSettingsA40.tsx` | **无**（`PagePlaceholder`） | 搬（P4） |

页签由 6 个变 8 个（新增「同步」「BBS 预设」）—— `AppA40.tsx` 的 `TABS` 是写死的常量表，我们照搬进 `src/app/constants/tabs.ts`，同时保留产品仓已有的两处覆盖（首页叫「首页」不叫「机型」、设置换真齿轮）。

### 2.2 共享件（A40 侧，产品仓缺的）

| A40 | 文件/LOC | 产品仓现状 | 动作 |
|---|---|---|---|
| `ui/`（Controls / Modal） | 3 / 567 | 有近似物（`src/app/ui/` 625 行） | 按需补齐（按钮/分段/弹层尺码） |
| `shared/`（Drawer / OriginChip / NoteBar / useStickyState / useDrawerWidth） | 9 / 661 | **无** | 搬 |
| `shell/`（TopTabs 8 页签 + 图标） | 3 / 491 | 有 `TopTabs`（6 页签） | 改 |
| `store/packageA40.ts`（同步层） | 1 / 371 | **无** | 搬（P4，键名去稿号 §8） |
| `src/components/field/*`（字段控件 + FieldLayer + FieldPopover） | 11 / ~1,400 | 产品客户端**没有**（只有工作台有一份） | 搬进 `src/components/field/`（P1） |

> `FieldLayer` 是硬依赖：A40 的下拉 / 浮层 / 抽屉全部挂在它上面。产品客户端要接新页面，这一层必须先有。

### 2.3 A40 页面调用的 api（16 个）

```
copyToSlicer  downloadFiles  getAppliedPreset  getCalibModels  getLocalFiles
getLocalUserFiles  getMachineParams  getMachines  getMenu  getParamMeta
getPreset  getPresetFiles  getSlicerCopied  getVersionFiles  openModel  saveOffsets
```

产品仓现有 4 个（`getPreset` / `saveOffsets` / `getCalibModels` / `openModel`）→ **要补 12 个**（含类型与 mock）。

契约上还要补的**字段**（都是「只加不改」—— 试验场 T7 / T8 / T15 同一条纪律）：

- `ClientFieldDef` += `valueType`（值本身是什么：float/string/bool/int）+ `showWhen`（「要 X = Y 才显示」）+ `tabId`（**英文**页签 id）
  > `tabId` 是 T8 修出来的一条根因：`tab` 存的是中文名，而客户端分类条的图标表按英文 id 查 —— 六个分类图标全塌成兜底那一个。
- `ClientDataPackage` / `Release` / `ReleasePreset` 三个形状：一次发布 = 说明书 JSON + N 份 `preset.toml`，同一个 preset identity（同包版本 + 同 `inputsHash`）

---

## 3. 工作台 C15 增量（逐字 diff 的结果）

| 文件 | 状态 | 这一份装的是什么 |
|---|---|---|
| `store/cloud.ts` | **新** | 模拟云端：`CLOUD_KEY = 'mkp.cloud.presets'`（**键名刻意不带稿号**）+ `public/cloud/presets.json` 静态快照；工作台「上传」写 localStorage，客户端读同一格 |
| `store/compat.ts` | **新** | `CLIENT_COMPAT` 兼容性清单（`id / since / label / detect`）+ `minClientOf(pkg)`：**木桶算法**（取命中特性里最大的 `since`）+ `verdictTextOf()` 说清依据。清单七行：基础包 1.0.0 · `showWhen` 1.0.0 · `tabId` 1.1.0 · `uiComponent` 1.1.0 · 版本带 `files` 1.1.0 · 版本带 `optionalFiles` 1.1.0 · `schemaVersion ≠ 1` → 2.0.0（版本轴与测试端 D01 共用：`0.9.0 / 1.0.0 / 1.1.0 / 2.0.0`） |
| `store/derive.ts` | 改 | `buildClientPackage(state)` —— 工作台真生成说明书（字段集 = 各机型可见并集、顺序走布局表、`tab`/`group` 给中文名、值摊平、`inputsHash`）；**读 `saved`**（没保存的改动不许进包）。外加模式开关的**整卡收起**判据 |
| `store/state.ts` | 改 | `saved` / 工作态两态，新键持久化 |
| `pages/BuildPageC15.tsx` | 改 | ② 客户端数据包变成真包（`6 台机型 · 10 个版本 · 74 字段 · 带条件 43`）+「查看 JSON」；③ 上传云端；版本轴：包版本快捷 `+0.0.1 / +0.1.0 / +1.0.0`、最低客户端版本「**自动判断**」、`schemaVersion` 只读 |
| `c15.module.css` | 改 | 整卡收起等尺码（4,078 行，改动集中在参数台那几段） |

**对应到产品仓的落点**：

| 试验场 | 产品仓落点 | 备注 |
|---|---|---|
| `store/cloud.ts` | `src/workbench/cloud.ts`（新） | 纯前端能力，没有真云端 |
| `store/compat.ts` | `src/workbench/compat.ts`（新） | 判定在前端 —— **§6-3 待裁决**（产品纪律是「前端不算业务」） |
| `pages/BuildPageC15.tsx` | `src/workbench/views/BuildPage.tsx`（改） | 产品仓这页已有 ①生成 ②数据包 ③发布 ④基线 ⑤残留 ⑥回收站 ⑦子目录职责；本轮补：真包内容 / 上传云端 / 版本轴自动化 |
| `derive.buildClientPackage` | 先在前端算（同 §6-3） | 产品仓的产物生成在 Rust（`wb_publish` / `wb_preview_toml`），说明书形状要单独补 |
| 模式开关整卡收起 | `src/workbench/views/ParamsPage.tsx` + `workbench.css` | 判据来自后端已给的每行 `blocked` / `blockedHint`，前端只做「整卡都关着 → 收起来」的版面决定 |
| 抽屉说清类型 | `src/workbench/views/ParamDetail.tsx` | 把「技术信息」（`valueType` / 控件 / 步进）提进「基本信息」并排（C15 的 B1 定稿） |

> **「两边共用一份 mock」不照搬**：那是试验场为了让同一浏览器里的两个稿互相喂数据而做的装置。产品仓的两个入口（客户端 `index.html`、工作台 `workbench.html`）本来就同源，等价物是**经云端那一格联动**（`mkp.cloud.presets`），所以只搬 `cloud.ts` 这一层。

---

## 4. 数据从哪来（本轮只有前端）

1. **客户端的读**：把试验场的假后端搬成 `src/api/mock/**`（`src/server/resolve/*.ts` + 7 个 JSON，≈ 123 KB）——页面只认 `contract.ts`，将来换真后端不动页面（这条与 v023 移植时的分法一致：UI 结构数据进 `src/app/constants/`，业务夹具进 `src/api/`）。
2. **native 窗口**：`bridge.ts` 对未接入的方法**抛 `NOT_IMPLEMENTED`**（产品仓的 `AppError` 已经有这一档）—— 界面上是一块统一的「后端未接入」空态，不是白屏。
3. **静态资产**：`public/cloud/presets.json`（109 KB，等于「云端已经有别人发过的」）、`public/bbs/**`（285 文件 1.29 MB）、`public/cat-poses/**`（21 文件 981 KB，猫已退役 —— §6-4）。
4. **联动链路**（本轮要真的走通）：

```
工作台 ① 生成 → ③ 上传 → localStorage['mkp.cloud.presets']
   → 客户端「同步」页自动同步说明书（指纹比对，界面只说「已是最新 / 上次同步」）
   → 预设页云端表出现「工作台发布 · 1.0.0」→ 下载（写本机预设）→ 应用（唯一底账）
```

三个状态**分开报**（说明书已同步 / 预设已获取 / 当前使用）—— T7 的硬规矩，不许合成一格。

T8 又把这条链切了一刀，搬的时候照它：**「同步」页只说说明书（JSON）**，不再摆「下载 JSON」；**TOML 归预设页** —— 云端表里多出一档来源「工作台发布 · 1.0.0」，行上只有「下载」，装好之后仍用本地表的「应用」去生效（不新造「使用」按钮）。下载写的是本机预设那一格，应用写的是当前使用那一格 —— 两格不是同一件事。

---

## 5. 分阶段计划（每阶段一个 PR，走八道闸）

| 阶段 | 内容 | 验收 |
|---|---|---|
| **P0** | 本方案 + §6 待裁决清单 | 作者过目 |
| **P1** | 骨架与数据层：页签 8 个 · `src/components/field/`（FieldLayer + 五件字段控件）· 契约扩到 16 方法 + 新类型 · `src/api/mock/**` 假后端 · 页面级 token 覆盖（`--r-sm/md/lg`、`--card-r`、`--sel-h`、`--seg-*`）· 「后端未接入」空态组件 | `tsc -b` + `npm run lint` + `npm run build` 绿；浏览器里 8 个页签都能点开，未搬的页是空态不是白屏 |
| **P2** | 预设页（`presets/` 12 文件）：MKP / 切片器 × 本地 / 云端两张表、两排工具栏、已应用状态条、喷嘴 / 层高筛选、6 列 → 点行展开、右键菜单、跳 BBS | 浏览器走查：两张表各自成表、四档密度不折行、右键菜单可用 |
| **P3** | 参数页（`params/` 19 文件）：分类条、卡片、行、抽屉、G-code 块、搜索（含命中带父子）、快捷键、撤销 / 重做 | 与 A40 README 第三十三～四十轮的实测条目逐条复量 |
| **P4** | 同步页（`pages/` 402 行级）：说明书自动同步（指纹比对）、三态分开（已同步 / 已获取 / 使用中）、空态；**报告页 / 设置页本轮只做空态** | 三态不串；空态说清「这一页本版未接入」，不是白屏 |
| **P5** | BBS 预设页（`bbs/` 27 文件）：**不打包静态快照**，改读本机 BBS 目录（dev 走 dev-server 中间件、真机走 IPC）；读不到就是空态 | 本机有 BBS 目录时清单出得来（278 份那一档）；没有时是空态 |
| **P6** | 工作台 C15 增量（§3）+ **联动端到端**：工作台生成 → 上传 → 客户端同步 → 下载 → 应用 | 端到端一条链走通（照 T7.7 那次的口径量数） |
| **P7** | 首页 / 校准页**整体替换成 A40 版**；产品壳那几件（`ResizeEdges` / `useTitlebarDrag` / `window.ts` / `TraceTag`）保留 | 替换前后同尺寸截图比对；`docs/DESIGN-SPACING.md` 的 clamp 表复量 |

验收三件套与 14b 同口径：`npx tsc -b` · `npm run lint` · `npm run build`；**行为验收用 `.mjs` 探针**（playwright + msedge channel，照试验场 `scripts/probe*.mjs` 那一套，探针脚本跑完即删或收进 `scripts/`），两档尺寸走查（Ultra 1760×900 / Compact 900×640）。

---

## 6. 已裁决（作者 2026-09-30）

| # | 题 | 裁决 |
|---|---|---|
| 1 | 首页 / 校准页 | **换成 A40 版**；产品壳那几件保留 |
| 2 | BBS 静态数据（285 文件 1.29 MB） | **不搬**；BBS 页签与页面保留，改读本机 BBS 目录，读不到就是空态 |
| 3 | 猫（`cat-poses` 21 文件） | **不需要** |
| 4 | 报告 / 设置两页 | **直接做空态**（A40 里那份手编数据不搬） |
| 5 | localStorage 键名 | **待定** —— 本轮集中写在一处，方便改名；云端那一格先用 `mkp.cloud.presets` |
| 6 | 验收工具 | **可以用 `.mjs` 探针**（playwright + msedge） |

### 仍待定：C15 的两个判定放哪一层

作者问「C15 的是什么判定」，这里说清这两件是什么、以及为什么它们牵涉「前端算不算业务」：

1. **`buildClientPackage`（生成说明书）** —— 工作台把当前配方导成客户端要读的那份 JSON 包：字段集（各机型看得到的**并集**）、顺序（走布局表）、`tab` / `group` 给中文名、值**摊平**（客户端不做继承推导）、外加 `inputsHash` 和三个版本轴；规矩是**只读已保存的态**（没保存的改动不许混进包）。
2. **`minClientOf`（最低客户端版本自动判断）** —— 一张 `CLIENT_COMPAT` 表说清「包里用到哪个特性，至少需要哪个客户端版本」（`showWhen` 1.0.0、`tabId` 1.1.0、`uiComponent` 1.1.0、版本带文件 1.1.0、`schemaVersion ≠ 1` → 2.0.0），然后**取命中特性里最大的那个版本**（木桶原理），并把依据那句话拼给界面。

**产品仓的处境**：产物生成在我们仓里本来就在 Rust（`wb_publish` / `wb_preview_toml`），产品纪律也是「前端不算业务」（14b 当时把 C14 `derive` 的判定全搬进了 Rust）。所以这两件的**归宿是 Rust**（说明书生成 + 兼容性清单，与 `words.rs` 词表同源）。

**本轮建议**：P6 先在**前端一处模块**（`src/workbench/compat.ts` + 说明书生成）实现，好让联动这条链当场能演示；同时在 Rust 侧登记成增量（不动命令白名单），下一轮搬过去。这样既不违反纪律（债记在案），也不让 P6 卡在「等后端」。

---

## 7. 风险与既知的债

- **首页 / 校准页是「量过」的部分**（已定要换成 A40 版，§6-1）：换的时候要照 `docs/DESIGN-SPACING.md` 的 clamp 表逐条复量，并且**只换页面、不换窗口** —— `ResizeEdges` / `useTitlebarDrag` / `window.ts` / `TraceTag` 这几件是产品仓为了真窗口加出来的，A40 版里没有，覆盖时不能连它们一起冲掉。
- **两套圆角尺度并存**：A40 的页面根把 token 覆盖成 C12 的近直角（`--r-sm:4 / --r-md:3 / --card-r:4`），产品仓首页 / 校准页用 `16/12/8`。靠**页面级覆盖**隔离，**不改全局 `tokens.css`**（A40 自己就是这么做的）。
- **契约扩容后的空窗**：P1 之后 native 窗口里新页面会整页「未接入」—— 这是预期，但必须有统一空态，不能白屏（P1 的第一件事）。
- **端口**：产品仓 dev 固定 `:5321`（`strictPort`）；试验场 `:5178`、它自己的探针 `:5301` —— 别互相踩。
- **stylelint**：两边都用 `stylelint-config-standard`，但产品仓多一条「组件级不许写 `::-webkit-scrollbar`」；搬进来的 CSS 要过这一关（滚动条只能写在 `src/styles/global.css`）。
- **BBS 改读本机目录要自己开路**：静态快照不搬（§6-2），所以 BBS 页的数据得从「本机 BBS 目录」来 —— dev 下要有 dev-server 中间件（试验场是 `tools/dev-server/bbsFs.mjs`），真机上要走 IPC。本轮先只做 dev 那一条，并把它做成「有目录就出清单、没有就空态」。
- **BBS 数据源出 Bambu Studio**：因为不打包静态数据，这一条授权顾虑消失；将来若要随包分发，再单独谈。
- **不搬试验场的 `bridge.ts`**：它读 `window.__mkp_api`（浏览器预览器注入），产品仓是 Tauri `invoke` —— 只搬契约类型与页面，不搬那一层。
- **别把工作台的共享件拖进用户包**：两个入口是两个 bundle，`src/components/field/` 只给客户端用；工作台自己那份留在 `src/workbench/components/`，互不 import。

---

## 8. 移植规矩（照 14b 那套）

1. **目录名去稿号**：`src/app/params/`、`src/app/presets/`、`src/app/bbs/`、`src/app/report/` ……；搬进来的文件内容里 `A40` / `C15` 稿号一律去掉（与 v023 移植时「按角色命名、不带稿号」一致）。
2. **共享件进 `src/components/field/`**：不给客户端 import 工作台那一份（两个 bundle 各自独立）。
3. **键名集中一处（名字待定）**：作者说键名还没定 —— 所以本轮把客户端与云端用到的键**全收进一个模块**（`src/api/storageKeys.ts`），改名是一处的事；云端那一格先用 `mkp.cloud.presets`（工作台与客户端必须读同一格，联动才成立）。
4. **mock 集中在 `src/api/mock/**`**：页面只认 `contract.ts`。
5. **只搬界面**：业务判定一律登记（§6-3），不新增 Rust 命令、不动 `write_discipline_scan.rs` 的白名单。
6. **旧页面删除要记账**：`PagePlaceholder` 被替换掉一页就在 §5 的表里划掉一页；四个占位页全替换完，这个组件和 `App.tsx` 里的 `switch` 分支一起收口。
7. **静态资源路径必须按本仓目录重写，不能照抄**（P1c 抓到的第一个真 bug）：A40 的 `home/heroArt.ts` 里五条图片路径写的是试验场 `public/` 根下的文件（`/a1.webp`、`/bambulab.svg`…），本仓一张都没有 —— 而且**页签点得开、控制台也干净**，因为没选机型时那张大图根本不挂 `<img>`；真选到机型才会露出来。本仓的整机图在 `public/assets/printers/`，品牌 logo 是 `src/app/assets/bambuLogo.ts` 里的 data URI。以后每搬一个引用静态资源的文件（P5 的 BBS 目录、云端的 `/cloud/presets.json` 都在这一类里），**逐条开 URL 核一遍**（`Invoke-WebRequest` 看 `Content-Type`：SPA 回落会给你 200 + `text/html`，那不是图片）。
8. **探针的量法也要对**：上面那条第一次是被探针误报的 —— 大图住在牌堆的第 0 张卡，停在选择页时那张卡不在 DOM 里，量到的是 0 张。教训：**先确认"要量的东西这一刻在不在 DOM 里"**，再判它坏没坏（`home-flow.mjs --pick` 现在会先点「回主页」再量，并且把 `naturalWidth` 一起打出来 —— 路径错时它是 0）。
