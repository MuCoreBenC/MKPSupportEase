# A41 → 产品仓移植账（单分支 · 单 PR）

> 起草：2026-10-01。**这一轮不是 C15/A40 那种 19 稿大移植** —— A41 与 A40 在试验场
> 只差 16 个文件（105 个逐字相同），且其中 5 个的差量只是 localStorage 键名的稿号
> 改名（`mkp.a40.*` → `mkp.a41.*`），产品仓本来就集中在 `src/api/storageKeys.ts`
> 管理、刻意去稿号 —— 所以真正要搬的只有 **首页（home）那一块 8 个文件**。
> 体例与未竟事项照 `C15-A40-PORT-PLAN.md` 的规矩（§8 移植规矩）。

## 0. 口径

| | 值 |
| --- | --- |
| 源 | 试验场 `mkp-adaptive-console` @ `7f6f941`（A41 定稿；`src/versions/a41/`） |
| 基线 | 本仓 `main` @ `22e13b3`（先把本地 main 快进到 origin/main，再开分支） |
| 分支 | `feat/b07-a41-port`（照 bNN 命名；**单分支单 PR**，作者 2026-10-01 裁决） |
| 差量 | a40→a41：16 changed / 0 added / 0 removed（量法：文件名与内容去稿号后逐字比） |

## 1. 每个差量文件的去向

| 试验场 a41 差量文件 | 差量是什么 | 动作 |
| --- | --- | --- |
| `home/SlideDeck.tsx`（+127/−16） | **peek-in / peek-out 两层动画**（露出卡跟着推入滑进、跟着退回滑出）+ 动画值改走 `useDevDefaults` + `data-deck-index/total` + `settleTailMs` | **三方合并**（base=a40 / ours=产品 / theirs=a41），冲突 3 处手解（见 §2） |
| `home/SlideDeck.module.css`（+148/−6） | peek-in / peek-out keyframes + 虚焦/压暗/缩放变量组 | 三方合并，冲突 1 处（注释，取 a41）；重复选择器 `.deck:has(.zoneNext:hover)…` 删一份（产品侧已有，两边逐字相同） |
| `home/CardFrame.module.css`（+94/−10） | frame-settle / frame-tuck 分段跟随 `--glide-ms`、模糊分段注释 | 三方合并，冲突 2 处（注释，两边各取所有句子合写） |
| `home/HeroFade.tsx`（+11/−8） | 曲线值改走 `useHeroCurves` + 根上挂 `data-hero` | 三方合并，冲突 2 处（注释，按产品口径合写；`data-hero` 留着 —— 面板信号，产品里没人读） |
| `home/MachinePicker.tsx`（+3/−2） | `FADE_MS` 常量 → `useDevDefaults()` | 三方合并，零冲突 |
| `home/PageHome.tsx`（+6/−3） | 同上 + `fadeMs` 进 DeckLayerContext | 三方合并，冲突 2 处（注释与 import 路径，取产品路径） |
| `home/devDefaults.ts`（+22/−7） | 试验场把面板接回来（`useDevDefaultsA41` 订阅 devStore） | **按 A41 README 预写的接法**：加 `useDevDefaults()` 直接 return 三个常量 —— 三个调用点一行不改 |
| `home/heroCurves.ts`（+38/−2） | `useHeroCurvesA41` 吃 devStore 曲线 | **按 A41 README 预写的接法**：加同名 hook，实现 = `evalFill` / `evalNudge` 的结果（devStore 那行 import 不进产品） |
| `home/activeSelection.ts`（±1） | 注释里的键名改名 | 不搬（产品侧是 `STORAGE.clientActive`，本就无稿号） |
| `params/useParams.ts`（±2） | 同上 | 不搬 |
| `presets/PagePresets.tsx` / `PresetTable.tsx` / `presetTree.ts` / `usePresetData.ts`（各 ±1~10） | 同上（注释里 `mkp.a40.active` → `mkp.a41.active`） | 不搬 —— **预设页产品侧已接真后端**（#13 + b223a30），这轮一个字不碰 |
| `store/package.ts`（+7/−7） | 键名常量值改名 | 不搬（产品侧键名集中管理） |
| `README.md` | A41 稿内账 | 不搬（试验场自留） |

## 2. 与「分叉」的冲突核对（作者点名要看的）

1. **远端先行 5 个提交**（预设页接真后端 #13、随包 catalog、perf、fmt）：本地 main
   先 `--ff-only` 快进到位再开分支 —— 分支起点就是最新 main，PR 天然无冲突。
2. **预设页真后端**：A41 对预设页的差量只是稿号注释，真后端接线（`usePresetData.ts`
   的 ipc 读数、`bridge.ts`）一个字没动；探针 `presets.mjs` 实测真数据（仓库 20 · 本机 4 + 我的 3）。
3. **`PagePackage.tsx` 的 catalog 增量**（4ddc683 +43 行）：A41 没改这一页，不碰。
4. **工作区里那条未提交的 SlideDeck Windows 修复**（will-change + SETTLE_TAIL）：
   - `will-change`（.plane）：a41 的 module.css 里**已有同一份**（两边逐行相同），合并自然带上；
   - `SETTLE_TAIL_MS`（推入定时器等 blur 归零那段）：a41 已把它**收编成 `settleTailMs`
     （随 `glideMs` 缩放的版本）**——按 a41 为准，产品那份写死 GLIDE_MS 的版本退役。
   - 所以工作区那两条未提交改动**全部被本分支覆盖**，不再单独提交。

## 3. 验收（八道闸口径的三件套 + 探针）

- `npx tsc -b` ✅ · `npm run lint`（eslint + stylelint）✅（重复选择器已清）· `npm run build` ✅
- 探针（build + preview :4173，msedge）：
  - `tabs.mjs` ✅ 8 页签全开、控制台零 error；
  - `home-flow.mjs` ✅ 首页五步向导与校准页都画出来、控制台零 error；
  - `presets.mjs` ✅ 真数据两轴四表、点行展开、右键菜单；
  - `params-sync.mjs` ✅ 同步页自动同步出说明书、参数页照包渲染（两档尺寸 0 error）；
  - `chain.mjs` ⚠️ **基线上同样失败**（工作台「全选待生成」按钮超时）—— 在未含本分支
    改动的 `22e13b3` 上复跑结果一致，**与本次移植无关**（工作台深处既有问题，另行记账）。
- A41 README 的露出卡实测口径复量（1760×900）：露出卡 frame =
  `matrix(0.95, 0, 0, 0.95, …)` · `blur(2.4px)` · `opacity 0.6`；主内容 frame =
  `matrix(1,…)` · filter none —— 与试验场 A41 的定稿值一致。

## 4. 遗留 / 记账

- `chain.mjs` 在 main 上即失败（「全选待生成」超时）：与本次移植无关，**需要单独查**
  （工作台生成页的按钮出现条件或探针等待窗口）。
- 试验场 A41 的 `settleTailMs` 收编了产品那条未提交修复；产品工作区原 patch 已由本分支
  覆盖，无遗留未提交内容。
- 静态资产路径本轮零涉及（home 的图片走 `useCatalog` 文件体系与 data URI，未引新资源）。
