# A07 搬运台账 · 交接（2026-10-14）

> 源 = 试验场 `frontend-lab` 的 **a07 稿**（2026-10-13 那批定稿，a07 是最新稿：当晚
> 22:31–23:59 的改动全在它上面）＋ 同仓 `lab/term-rename-local-cloud.md`（术语清单）。
> 落点 = 本仓 `src/app/**`、`src/api/**`、`src/components/**`、`src/styles/tokens.css`、
> `src-tauri/src/**`。方向沿用 [`A06-PORT-LEDGER.md`](A06-PORT-LEDGER.md) 那条：
> **试验场前端 → 本仓**（本仓是唯一产品本体）。

## 0. 一句话现状

| 仓 | 状态 |
| --- | --- |
| 试验场 `frontend-lab` | ✅ 已提交 `ca8133c`：更新演示开关搬到预览外壳工具条 + 工作区那批改动一并提交 |
| 产品仓（本仓）`feat/dark-mode` | ✅ 已提交并推送 `b7ebafb`（a07 界面同步**第一笔**，tsc / lint / build / 四个 check / cargo 全绿） |
| 剩余 | ⏳ **术语改造全量**（见 §3，开工过一半、已回滚）+ 预设页三份的改名族（§2） + 本台账登记进 `DOC-INDEX.md` |

## 1. 已完成（b7ebafb）

| 文件 | 搬了什么 |
| --- | --- |
| `app/settings/PageSettings.{tsx,module.css}` | 整页换 a07 定稿：自绘墨色选中态与开关（`appearance:none`，不动全站 `accent-color`）、「软件更新」提到第一块、动作条右上（**重新检查 → 查看更新 → 更新**，有颜色的压最右）、「切片」改 `SelectField` 下拉 + 「其他应用…」从磁盘指定 |
| `app/home/*` | 复制脚本那一颗（差一步就整颗隐藏）+ 抄不了的原因改全页唯一那颗 `.scriptToast`；板下两条常驻提示退役（`calibAxes` 的 `NEED_PRESET`/`NEED_COPY` 删）；三轴读数三态同壳；`--plate-fixed-vh 303`；删掉试验场专用的 `?lab=apply` 与 `initialSheet` |
| `app/calib/*` | `CalibHead` 三态同壳同尺寸；`PageCalib` 文案与 `slicerArgsOf` |
| `app/params/*` | 28px 控件改胶囊；`ParamSyncModal` 等的文案与字段 |
| `app/bbs/*.module.css` | 硬编码色换全站 token（底 + 栏，与报告页同构）；抽屉那条 `.pickWrap button > span` 覆盖退役 |
| `app/import/FileImport.*` | 结果从 in-flow 横幅改成**模态框**（真机 `onDragDropEvent` 拖拽分支与 `Drawer` 保留） |
| `src/styles/tokens.css`、`app/App.module.css` | 深色**面族去蓝**（`#1c1c1e→#1c1c1c` / `#232326→#232323` / `#303034→#303030` + 窗底渐变一起中性） |
| `src/components/field/SelectField.module.css` | 触发器那个值**去加粗**（全站下拉一起生效） |
| `src/api/contract.ts`、`bridge.ts`、`mock.ts`、`storageKeys.ts` | 新口子（只加不改）：`SlicerId` 含 `'custom'`、`SlicerApp`、`pickSlicerApp()`、`openModel` 的 `opts.slicerApp`、`clientSlicerApp` 键 |
| `app/shared/slicer.ts`、`SlicerPickView.tsx`、`home`/`calib` 调用点 | `SLICER_CHOOSE` / `slicerOptionsOf` / `useSlicerAppPref` / `slicerArgsOf`；「用哪个打开」换自绘下拉 |
| `src-tauri/src/ipc/slicer.rs`、`mod.rs` | `SlicerKind` 加 `Custom`、`launch_with()`；`open_model` 收 `slicer_app`（路径不在盘上 = `NOT_FOUND`，**不静默回落**）＋ 单测 |

**挑选应用走的是前端对话框插件**（与 `pickExportDir` 同一条路，`dialog:allow-open`）——
**没有新增 Rust 命令**（`.app` 是目录：macOS 停在 `/Applications`；Windows 挑 `.exe`）。

## 2. 剩余清单（按顺序做）

1. **术语改造全量**（§3）—— 这是唯一的大件，先把 `contract.ts` 与 Rust 一起改，再改页面。
2. 术语落地后，把 `presets/` 那三份的改名族带上：
   - `presets/presetTree.ts`：改名族 **＋ 真修半**（`applyActivePreset(fileName, origin, path)`
     三参漏传的修：原来只传 path，落进 fileName、被判成云端那份 —— 本地表那颗「使用」永远失败）
     ＋ 「MKP 档本地表只列 `presets-local/` 那几行」的分组口径。
   - `presets/PagePresets.tsx` / `PresetTable.tsx`：改名族；**产品仓独有那段必须留**
     （工艺页跳过来的 `pending`、`revealDelivered`/`revealSlicer`、`outcome.replaced` 文案）。
   - `presets/CompareModal.tsx`：文案「还没有『我的预设』可以对比」→「还没有本地预设可以对比」。
   - ⚠ `presets/usePresetData.ts` **不许整文件覆盖**：产品仓那半是探针校订 +
     `revealDelivered` / `revealSlicer` / `copyToSlicer(CopyToSlicerOutcome)` 真机落点。
3. 把本台账登记进 [`DOC-INDEX.md`](DOC-INDEX.md)。
4. 验收与推送（§5、§6）。

## 3. 术语改造施工单（官方/我的 → 云端/本地）

**方向**：只留「本地 / 云端」；作者原话「说没有我的，没有官方这种说法，只有本地和云端」。
试验场 `src/versions/a07` 已按它改完，**可以当对照实现抄**；清单在
`frontend-lab/lab/term-rename-local-cloud.md`。

**开工到哪**（2026-10-14，已回滚，仓库保持 green）：机械替换跑过一遍（53 文件，
含 `mine.rs → local.rs` 的 `git mv`），编译器还差 ~13 处 Rust ＋ ~20 处 TS 没修完，
临时脚本已删、`git reset --hard` 回滚到 `b7ebafb`。**再开工请照下面这张表重跑一遍**。

### 3.1 替换表（长名优先，防前缀误伤）

TS（`.ts/.tsx`）：

```
getUserPresetFiles→getLocalPresetFiles   getUserPresetIndex→getLocalPresetIndex
getUserCopyFor→getLocalCopyFor           renameUserPreset→renameLocalPreset
copyUserPreset→copyLocalPreset           deleteUserPreset→deleteLocalPreset
setUserPresetMachineVersion→setLocalPresetMachineVersion
UserPresetFile→LocalPresetFile  UserPresetIdentity→LocalPresetIdentity  UserPresetText→LocalPresetText
UserFileIdentity→LocalFileIdentity  UserCopyCalibration→LocalCopyCalibration  UserPreset→LocalPreset
MineState→LocalState  MINE_→LOCAL_  mockMineText→mockLocalText  mockMine→mockLocal
mineStateDetail→localStateDetail  mineState→localState  mineUnreadableWhy→localUnreadableWhy
mineCountOfAxis→localCountOfAxis  mineByCombo→localByCombo  setMineMachineVersion→setLocalMachineVersion
setMineIndex→setLocalIndex  appliedIsMine→appliedIsLocal  mineFiles→localPresetFiles
minePath→localPath  mineValue→localValue  mineAttr→localAttr  mineHalf→localHalf
mineCount→localCount  mineOnly→localOnly  mineIndex→localIndex  mineText→localText
officialNew→cloudNew  officialVersions→cloudVersions  officialFileName→cloudFileName
officialReady→cloudReady  officialSpeed→cloudSpeed
presets-mine→presets-local
```

形状上的那一格（正则，改完必须 `tsc -b`）：`.mine→.local`、`.official→.cloud`、
`mine:`（对象键 / 参数名）→`local:`、`'mine'→'local'`、`'official'→'cloud'`。

Rust（`.rs`）：模块 `mine→local`（两个文件 `git mv`：`ipc/mine.rs`、`runtime/mine.rs`）、
`::mine→::local`、`MineState→LocalState`、`MineFile→LocalFile`、`mine_files→local_files`、
`check_mine_prefix→check_local_prefix`、`repoint_mine→repoint_local`、
枚举变体与构造器 `Official→Cloud` / `Mine→Local` / `fn official→fn cloud` / `fn mine→fn local`、
命令名 `*_user_preset_*→*_local_preset_*`、`user_copy_for→local_copy_for`、
`official_new→cloud_new`、`official_text→cloud_text`、`official_versions→cloud_versions`、
`official_old→cloud_old`、`official_file_name→cloud_file_name` 等 `official_*` 一族、
结构体字段 `pub mine:→pub local:` / `pub official:→pub cloud:`、`presets-mine→presets-local`。

中文（**只改代码行、跳过注释行**）：官方交付→云端交付、我的预设→本地预设、
我的一份→本地这份、我那一份→本地那份、你的预设→本地预设、我的文件→本地文件、
官方→云端、我的→本地。

### 3.2 四个坑

1. **`local` 已被切片器语义占用**（`getLocalFiles()` = 本机真有的文件、`scope:'local'|'cloud'`）⇒
   「用户自己那份」在会同框的地方用 `localPresets`（state / 数据字段），只在谈来源的地方用 `local`。
2. **serde 字段名前后端必须同批改**：`officialNew` / `officialOld` / `mine` 这些是 IPC 上的形状，
   只改一边就是运行时对不上（`cargo check` 看不出来）。
3. **注释里的历史叙述保留原词**（"当年为什么把官方那份改成只读"这类）；新写的注释用新词。
4. **不写迁移**：`presets-mine/` → `presets-local/` 就当全新应用，旧目录用户自己删。

### 3.3 验收（清单 §4 那三条 + 本仓那几条）

```powershell
# ① 用户可见文案里不该再有这两对词（注释命中可人工判断）
Get-ChildItem -Recurse -File -Include *.ts,*.tsx,*.rs -Path src,src-tauri/src,crates |
  Select-String -Pattern "官方|我的一份|我的文件|我的预设"
# ② 标识里不该再有 official / mine（presets-local/ 这一处例外）
Get-ChildItem -Recurse -File -Include *.ts,*.tsx,*.rs -Path src,src-tauri/src,crates |
  Select-String -Pattern "'official'|'mine'|\bmine[A-Z]|MINE_" -CaseSensitive
npx tsc -b --pretty false
cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features workbench -- -D warnings
cargo test; cargo test --features workbench
```

## 4. 别碰的（真机接线，覆盖就残废）

`api/bridge.ts`（Tauri invoke 层）、`api/mockServer/**`（预览演示集，含 `bbsLocal.ts`）、
`bbs/{bbsSource,useBbsData,bbsTypes,useBbsDelivered,PageBbs,BbsPresetDrawer}.tsx`
（走 `get_bbs_presets` / `get_bbs_preset` 两条 IPC）、`window.ts` + `useTitlebarDrag` +
`ResizeEdges` + `TraceTag`（真窗口）、`hooks/useWindowMaximized.ts`、
`import/FileImport.tsx` 里 `getCurrentWebview().onDragDropEvent` 那一段。
试验场专属的东西（`labMode()` / `LabTrigger` / `?lab=` / `?postprocess=` 点播 / 演示开关 /
`gallery` / `dark-cards`）**一律不许跟着稿回产品**。

## 5. 怎么看（验收步骤）

**试验场那一刀**（`frontend-lab`，已提交 `ca8133c`）：

1. `npm run dev` → 开 `http://localhost:5173`。
2. 顶栏稿号选 **A07** →（工具条上现在多两组控件）**「更新演示」五档** +
   **「文案」短 / 长** 两颗；把鼠标停在它们上面，`title` 写着「试验场专用 · 只看效果」。
3. 进 **设置页**：页头下面那条虚线框的演示条**没有了**（旧样子：一条虚线框写着
   「试验场专用 · 只看效果，产品里没有这一条」+ 五档按钮）。
4. 在外壳工具条上把「更新演示」依次切到：正在检查 / 有新版本 / 已是最新 / 没查到 ——
   设置页那张更新卡**当场变**（切档**不会**把你弹回首页、页签与滚动位置都留着）。
5. 停在「有新版本」，把「文案」切到**长**：发版说明变成一整篇，说明块**自己滚**、
   卡片高度不跟着长；切回**短**是一句话。同理「没查到」的长/短是失败原因的长短。
6. 切回「跟着真数据」= 与产品里一模一样的真账。

**产品仓那一笔**（`b7ebafb`，`npm run dev` ⇒ `?mock=1`）：

1. **设置页**：选中态是自绘的墨色小圆点（不再是浏览器默认外观 + 品牌绿）；
   「软件更新」是**第一块**，状态在左、动作在右上，顺序 **重新检查 → 查看更新 → 更新**
   （有颜色的那颗压最右）；「切片」是自绘下拉，最后一条是「其他应用…」。
2. **首页**：四步里那一颗「复制脚本」；条件不满足时**整颗不出现**，点不了的原因由整页
   唯一那颗悬浮气泡说（不再有板下那两行常驻提示）。
3. **工艺页（深色）**：底色与报告页同构（页底 + 两条侧栏），字面值不再偏蓝偏绿。
4. **导入**：拖一份 `.toml` 进来，结果是**居中模态框**（旧样子：标签栏下面一条横幅）。
5. **深色模式**：全站面族应当是中性的（R=G=B），报告页抽屉那些灰不再发蓝。

## 6. 静态检查现状（都跑过、都绿）

`npx tsc -b` · `npm run lint` · `npm run build` · `npm run check:bundle` /
`check:zero-network` / `check:channel-args` / `check:release-source` ·
`cargo fmt --all -- --check` · `cargo clippy --all-targets -- -D warnings`
（default + workbench）· `cargo test`（503 通过）· `cargo test --features workbench`（856 通过）。

> 唯一一条挂的是 `obs::hook_log::tests::init_reports_the_daily_file`（写日志目录
> `Os { code: 2, NotFound }`）—— 本机环境问题，**改动前就挂**，与本次无关。
