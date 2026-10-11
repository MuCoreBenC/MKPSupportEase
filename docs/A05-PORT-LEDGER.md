# a05 前端搬运台账（2026-10-11）

> 源 = `frontend-lab/src/versions/a05/`（试验场定稿），落点 = 本仓 `src/app/`。
> 方向裁决：**试验场前端 → 本仓**（本仓是唯一产品本体，Rust / 发布链 / CI 不动）。
> 原则：整份照搬他的（按钮、布局、颜色、文案），不自创；只有四处**本仓领先、不回退**（见 §4）。

## 1. 每段搬了什么

### 段 1 · 报告页
- `app/report/PageReport.tsx` + `.module.css`：整份替换为试验场版（双栏 master–detail、
  分组页签、<880px 容器查询滑盖、拖拽三件套 NARROW_W/PEEK/SNAP）。
  唯一本地化：文案「复制脚本」→ 本仓按钮真名「**复制后处理脚本**」。
- `app/App.tsx`：`PageSlot` 加可选 `page` → `data-page`；五个非首页槽传值，首页槽不传。
- `app/ui/ui.module.css`：追加 `[data-page]` 共用件圆润化段（段 2 之后整份换，见下）。
- `app/shared/NoteBar.module.css`：换他版（9px 16px / `--r-md`）。NoteBar 只有报告页用，零外溢。
- mock：新增 `api/mockServer/reports.ts`（10 条真机记录夹具，首条逐字段照真机）、
  `mockServer/index.ts` 补导出、`mock.ts` 两条换实现。

### 段 2 · `ui/Modal` 退役 + 皮肤整份换
- `components/modal/`：Modal.tsx / index.ts / Modal.module.css 整份换 + 新增 `modalHeadSlot.ts`。
- 三个引用迁走：`PageHome.tsx`（3 个弹窗 → 通用居中框，`open/size/footer` 形态，文案照他）、
  `postprocess/PostProcessModal.tsx`（整份换他版）、`calib/PageCalib.tsx`（随段 3 整份换）。
- 新增 `app/ui/Progress.tsx`（从 ui/Modal 拆出，照他拆法）；**删除 `app/ui/Modal.tsx`**。
- `ui/Controls.tsx` + `ui/ui.module.css`：整份换试验场版 —— **胶囊按钮语言全站生效**。
- `styles/tokens.css` + `styles/global.css`：整份换（补 `--box-x` `--font-label` `--state-bold`、
  Windows 粗体发虚的 font-smoothing 处理、按钮不换行等）。
  ★ `--state-bold` 必须一起进：`ui.module.css` 三处 `text-shadow: var(--state-bold)` 是
  「选中不改宽度」，变量缺失会**静默失效**。

### 段 3 · 校准页
- `calib/{PageCalib.tsx,PageCalib.module.css,CalibHead.tsx,CalibReadings.tsx,CalibReadings.module.css,calibAxes.ts}`：
  整份换 —— 浮动药丸 changeBar（脏/已保存/未选三态 + 弹簧宽度动画）、动作组瘦成「打开模型」、
  步骤条短文案、Modal 换 `components/modal`。
- `calib/usePreset.ts` **保留本仓版**（见 §4③）。
- `PageCalib.tsx` 两处照本仓口径回调：`activateCombo(m, v, fileOf, activeEntry)`。

### 段 4 · 共用层对齐
- `app/shared/OriginChip.tsx`：换他版（去掉 `DELIVERY_WHY` 悬停提示 —— tooltip 收口）。
- 新增 `app/state/draftState.ts`（草稿代次广播 + `textDraftDropped` 单向通知）。
- 新增 `app/presets/tomlPaint.ts`（TOML 上色词法，预设页编辑器用）。
- `state/appState.ts`、`state/deliveryState.ts`、`shared/download.ts` **保留本仓版**（见 §4）。

### 段 5 · Rust 新链（本刀唯一动后端的段）
- `runtime/state.rs`：新增 `PresetTextDraft`（「编辑文件」链的草稿，与参数草稿**分开存**）。
- `runtime/app_state.rs`：`AppStateFile` 加 `text_draft` 格（`serde(default)`，旧档照读不升号）+
  `text_draft / set_text_draft / clear_text_draft`。
- `ipc/mine.rs` 新命令六条：`begin_text_edit` `put_text_draft` `commit_text_draft`
  `discard_text_draft` `get_preset_draft_summary` `get_user_preset_index`（lib.rs **两份清单都注册**）。
- **两侧互相作废**：`commit_preset_draft` 顺手 `clear_text_draft`；`commit_text_draft` 顺手
  `clear_draft`（2026-10-10 契约写死：盘上字节换了，另一侧手上的正文就是孤儿）。
- `runtime/param_sync.rs`：`ParamSyncEntry::is_conflict()`（官方改了 + 我也改了 + 值还不一样；
  任一侧缺值 = 说不清 = 不算冲突），DTO 加 `conflict`。
- `ipc/catalog.rs`：`fetch_official_preset` 加 `rename_if_taken` —— 撞名时默认不动它，
  返回 `takenBy` + `suggestedName`（`untaken_name()` 找 `<名>-2<后缀>`…）；用户同意后自动改名落新的。
- 身份账本 = **派生视图**（盘上文件 + 血统 → 云端那一版），**不设持久化账本**
  （「盘就是底账」；期望值只来自当前目录）。`UserPresetIdentity = {id, path, fileName, cloudSource}`。

### 段 6 · 预设页 + 参数页
- `app/presets/**`（含 `presetTree.ts` `usePresetData.ts` `PresetTable.tsx` `PagePresets.tsx` `tomlPaint.ts`）
  与 `app/params/**`：整份换试验场版（本地/云端两档、下载/更新、逐参数冲突、正文编辑器、右键菜单）。
- `contract.ts`：`fetchOfficialPreset(fileName, renameIfTaken?)` + `FetchOfficialResult.takenBy/suggestedName`、
  `ParamSyncEntry.conflict`、`PresetTextDraft`/`PresetDraftSummary`/`UserPresetIdentity` 三类型 +
  六条方法签名。**保留 `getPostProcessCommand`，不引入 `getPostProcessExe`**。
- 下载/更新接线：`bridge.ts` → 真后端 `fetch_official_preset`（下载/更新本来就是真实现，mock 只在 `?mock=1`）。

## 2. 验证（全绿）
- `npx tsc -b`、`npm run lint`（余 2 条预存 warning）、`npm run build` + check:bundle /
  check:zero-network / check:channel-args。
- `cargo fmt`；双 feature `cargo clippy --all-targets -- -D warnings`；`cargo test`
  （820 过；`a_conflicting_tag_does_not_fail_the_fetch` 一条**预存环境红** —— 干净树上同样红，
  是本机 git 闸拦了测试里造的临时 repo，与本次改动无关）。
- 冒烟（`?mock=1`，1440/760 两档截图在 `tmp-shots/`）：报告页宽/窄档、校准页（浮动药丸）、
  预设页本地表 + 云端表 + **下载链路真通**（点「下载」→「已下载 A1-standard.toml」→ 本地表出现）；
  读不到字节的演示文件「使用」时如实报错（不装成功）。

## 3. 留下的差异 / 已知未搬
- 报告页 spec §2 那五组字段：后端未透传，页面「不给就不摆」（口径见 `docs/REPORT-DETAIL-FIELDS.md`）。
- `_meta.json` 同形要求（旧 Go 程序样本）记在台账，后处理那一刀落实。
- 试验场的 `getPostProcessExe`（靠 absPath 决定摆不摆）**未搬**——本仓 2026-10-10 已换
  `getPostProcessCommand`（按钮常驻 + 后端现拼）。
- 探针：报告页无现成探针（冒烟走 `?mock=1`）；★ 若日后退役 `ui/Modal` 相关结构，记得裁掉读它的探针。

## 4. 本仓领先、不回退的四处（下次同步对照）
1. `getPostProcessCommand`（取代试验场 `getPostProcessExe`）。
2. `App.tsx` 的 `detectPlatform()`（不是试验场那种预览外壳 props 注入）。
3. `appState.activateCombo` / `usePreset.ts`：本仓是双线口径（`applyActivePreset(fileName, origin, path)`），
   试验场还在单参 `applyActivePreset(path)` + `localPathOf` —— **以本仓为准**。
4. `state/appState.ts`（状态读窄了）、`state/deliveryState.ts`（范围放宽）、`shared/download.ts`（仅路径深差）
   —— 三处本仓更新，试验场落后。
