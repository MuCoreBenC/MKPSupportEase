# HANDOFF 归档（2026-10-20 清出的旧内容）

> ## 这是什么
>
> `HANDOFF.md` 2026-10-20 第二次清理时**搬出来**的增量账，**一个字没删**，只是不再住在活台账里：
>
> - **增量之三十六 ~ 四十四**（2026-10-16 ~ 10-19）—— 图片引导 / 放大层拖动 / Windows 构建兜底 /
>   校准模型档 / 自定义数据源认仓库页 / 交付错误带码 / 开发台真实后处理（单趟 → 六档矩阵 → 校准两档 → 撤预览）。
>
> **现行规则不在这里**：纪律 / 判据 / 仓库状态 / 怎么验证，以及**未结账那一张表**、最近的几条增量，
> 都在 [`../HANDOFF.md`](../HANDOFF.md)。上一批（§A + 增量二十五~三十五）在
> [`HANDOFF-ARCHIVE-2026-10-19.md`](HANDOFF-ARCHIVE-2026-10-19.md)。
>
> ★ 归档件**只读、不追加**：新东西写进 `HANDOFF.md`；等它又长了，再从它头顶搬下一批成一份新的归档。
> ★ 搬走的段里凡有"待验收 / 未做 / 待拍板"的，都已经抄进 `HANDOFF.md` §2.4 —— 找活干看那一张表，不用翻这里。

---

## 增量之三十六 · 图片引导（客户端弹窗 + 工作台标注台）：能缩放 / 缺图留空 / 源稿进仓库（2026-10-16，分支 `feat/dark-mode`）

> 交接文档：[`docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md`](docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md)
> —— 那一份写的是**下一刀**（① 图片按需下载 ② "一组/套装"的三件裁决）与不许违背的六条口径；
> 本节只记这一轮**做了什么**。

**§1 客户端那个弹窗**（`src/app/guide/`）

- 版式：图在左、**文字（状态句 + 步骤）全在右**；图 = `width/height:auto` + `max-width:min(314px,46vw)` + `max-height:min(46vh,460px)`
  ⇒ 两个方向谁紧听谁，任何窗高都完整显示、框身不滚。
- **图可点开放大**：走 `Modal` 新加的 `overlay` 槽（同一个遮罩里浮起来、不另开窗），进出 = **FLIP + WAAPI**
  （进 240ms / 收 200ms 倒放；遮罩底画在 `::before` 上、父级从不改 opacity —— 这是"缩小时会闪"的正解）。
- **放大后可缩放**：`wheel`（`ctrlKey` = 触控板双指捏合，否则普通滚轮）+ 两指 pointer；只写内联 `transform` + rAF、
  以指针为不动点、`scale ∈ [0.5,4]` 带阻尼；**每次打开复位成默认大小**。框的线 / 圆角 / 编号视觉大小恒定
  （`calc(… / var(--zoom-k))`；线改用 `box-shadow` —— `border-width` 被引擎钳在最小 1px）。
- **图取不到 = 整块留空**：`img.onError` ⇒ 与"台账里没有这条"同一条落法（不画那块、右列占满、不留裂图、不写 console）。

**§2 工作台「标注台」**（`src/workbench/views/AnnotatePage.tsx`，nav 排在「资产库」后）

- 图上拖框 / 拉边 / 方向键 / ⌘Z / 删除 + 图层列表 + 四格 px；**框的样子**三格（圆角 / 线宽 / 编号直径，缺省 8/2/18，
  与客户端**同名 CSS 变量 + 同缺省**）；选图**按组摆**（`<optgroup>` = 一组 2 张 · 共用一份标注）+ 页脚点名同组那张。
- **源稿进仓库**（作者：「像 PS 那样能再编辑、跟着项目仓库走，换设备也能接着编」）：
  `presets/assets/guides/<对>.anno.json`（与图同层），读写走 `wb_anno_read` / `wb_anno_write`
  （`src-tauri/src/workbench/app/annotation.rs`：路径由后端从台账派生、`updatedAt` 后端盖、坏值整次拒绝、
  **不进资产也不进交付**）；localStorage 退成"没写进仓库的暂存"（保存即清；**内容真的不一样**才弹冲突）。

**§3 「随包资产只兜底」收口**（"图片不见了"那次的根治）

- `runtime::catalog_for_ui` = ② + `Catalog::with_missing_bundled_from(&EMBEDDED)`
  （只补 ② 里没有的 `bundled` 条目、② 里已有的绝不动、下载面一条不补、**盘上字节一个不改**）；
  `ipc::catalog::get_runtime_catalog` 走它 ⇒ 随包的东西不再依赖任何一份云端文件。

**§4 验证**

- `cargo fmt` / 双 feature `clippy --all-targets -D warnings` / `cargo test --lib`（**550**）/
  `--features workbench --lib`（**911**，+4）/ `npx tsc -b` / `npm run lint` / `npm run build` + 三道 check。
- 探针两条：`scripts/probes/guide-image.mjs`（~70 条：版式 + 放大层 + 缩放 + 404 留空）、
  `scripts/probes/annotate.mjs`（一组 / 拖拽 / 样式 / 源稿读写）。**两条全绿**。

## 增量之三十七 · 客户端放大层可拖动 + trae 清场（2026-10-17，分支 `feat/dark-mode`）

**§1 客户端图片引导放大层：拖动平移（作者：「我希望客户端的也可以拖动，现在只有缩放还没法拖动」）**

- 病根与工作台标注台那条同款：`GuideFigure` 的 `<img>` 没禁**原生拖图**——按住一拖，
  WebKit 起的是"拖走这张图"，pointer 序列当场断掉 ⇒ 平移废了（平移的 pointer 逻辑本来就在，
  单指就是拖、双指是捏合，只是被原生拖拽截胡）。
- 治法：`GuideFigure` 的 img 加 `draggable={false}`；`.zoomLayer` 补 `user-select` / `-webkit-user-select`
  （WKWebView 只认带前缀的）+ `.zoomLayer img { -webkit-user-drag: none }`。
- 放大层提示句补全：「拖动挪位置 · 滚轮 / 捏合缩放 · 点任意处或 Esc 收起」。

**§2 trae 清场（作者：「检查一下有没有 trae 留下的记号之类的，清理掉」）**

- `.trae/documents/` 两份设计文档**搬进 docs/**：`PRESET-DATA-SHAPE.md`、`PRESET-PAGE-NATIVE-PLAN.md`
  （`git mv`，内容一字未动），`DOC-INDEX.md` §2 各补一行；`.trae/`、`.comate/`、`.zcodeignore` 删除。
- `.gitignore` 那条注释同步改口径：设计文档一律住 `docs/`。
- `PageCalib.module.css` 里几处 "trae-counter" 是**出处注**（那几条玻璃样式的来源），保留；
  `HANDOFF` / `UpdateIndicator` 里"像 Trae 那样"是作者的原话引用，保留。

**§3 「去重新允许」按钮（同日作者追问：「如果用户后悔了，有再次触发的开关吗」）**

- macOS 的死规矩：**拒绝过一次就永远不会再弹**授权框（TCC 没有"再问一次"的 API）⇒
  反悔的唯一正路是系统设置 → 自动化里把「访达」勾回来。
- 落法：`ipc::slicer::open_automation_settings` 用系统直达 URL
  （`x-apple.systempreferences:com.apple.preference.security?Privacy_Automation`）
  开**那一页**（依旧不替他勾）；前端只在错误码 `PERMISSION_DENIED` 时在引导的
  `statusExtra` 槽摆一颗「去重新允许（系统设置 → 自动化）」+ 小字
  「勾上「访达」后，回来再点一次「去系统里改…」就行」。
  命令注册两份清单都加了（`lib.rs`）；mock 照实拒（浏览器没有系统设置）。
- 验证：`tsc -b` / 双 feature clippy / `cargo test --lib`（558）/ `--features workbench`（934）/
  `npm run lint` / `npm run build` + 三道 check。全绿。

## 增量之三十八 · Windows 构建补平台兜底：`zorder` 跨平台 + `slicer` 名字类型归位（2026-10-18，分支 `feat/dark-mode`）

**病根：拉取最新（`b97f4e7`）后 Windows 上 `npm run tauri dev` 编译不过（5 错）**

- `error[E0433] cannot find zorder` ×3（`ipc/guide.rs` 218 / 631 / 640）：`zorder.rs` 整文件挂着
  `#![cfg(target_os = "macos")]`，Windows 上这个模块是**空的**，而 `ipc::guide` 无条件引用
  `crate::zorder::SlicerWindow` / `slicer_window` —— 文件级 cfg 没给别处留兜底。
- `error[E0308]` ×2（`ipc/slicer.rs:282`）：`known_label` 回 `Option<&'static str>`、
  `app_stem` 回 `String`，`unwrap_or_else` 两侧不同型；下游 `SlicerApp.name` 要 `String`。

**治法（沿用仓里既有的「平台兜底」双 cfg，见 `ax.rs` / `ipc::guide`）**

- `zorder.rs`：删掉文件级 `#![cfg(target_os = "macos")]`，改为只把 `mod mac`（CoreGraphics 那半）
  挂 `#[cfg(target_os = "macos")]`；`slicer_window` 拆双 cfg，非 macOS **恒 `None`**
  —— 调用方自然退回 AX 口径（本就没有"层序"这一说）。文件头「兜底」节补一句非 macOS 说明。
- `lib.rs`（`pub mod zorder` 旁注）：口径同步改成「窗口列表 API 只有 macOS 有（`mod mac` cfg 掉了）；
  其它平台 `slicer_window` 恒 `None`」。
- `ipc/slicer.rs`：`known_label(&path).map(str::to_string).unwrap_or_else(|| app_stem(&path))`。

**验证**

- `cargo check --no-default-features`（Windows）：绿。仅剩 8 条 macOS-only 死代码 warning
  （`ax.rs` / `guide.rs` 里那几张表和函数在别的平台上本来就编不出来，非本次引入）。

## 增量之三十九 · 校准模型档：几何搬进 `presets/calibration_models.toml`，旧版/加长版并存（2026-10-18，分支 `feat/dark-mode`）

**病根：作者把自己那套 Z 校准模型的打印区加长了，后处理直接把正常模型拦下来**

- 改动：低沿往下延长 1mm、高沿往上延长 2mm（中间圆点没动）⇒ 打印区网格 120.000 × 13.484 → **16.484**，
  切片后实测外形 119.58 × 13.063 → **119.58 × 16.063**（宽不变）。实测证据：旧模型 `presets/assets/models/ZOffset_Calibration.3mf`
  打印区相对模型 bbox 低沿 11.417..24.901，作者那份 STL 是 10.417..26.901（低沿 −1、高沿 +2），X 区间逐位相同。
- 后处理报 `E_CAL_MISMATCH_002`「检测到校准模型尺寸已变化」：标称外形与图案基准原来**写死在
  `disk/calibration.rs` 的常量里**（`Z_PATTERN_MODEL_WIDTH/HEIGHT`、`Z_PATTERN_OFFSET_X/Y`、`Z_SQUARE_*`、`XY_PATTERN_*`）
  —— 也就是说「同一台机器换一块校准板」= 改代码重发版。

**治法：一档一条数据；认档按实测外形尺寸自动匹配**

- 新数据文件 **`presets/calibration_models.toml`**（① 开发源层，单文件 `[[models]]` 多档）：每档写标称外形 + 容差、
  图案基准（`datumX/datumY`）、方块间距/数量、XY 模板七个偏移、质心旋转判据的期望。首批四条：
  `precise`（XY 50.5×50.5，数值逐字照搬旧常量）、`zoffset`（119.58×13.063）、**`zoffset-ext`（119.58×16.063）**、
  `repetition`（只给基准，L 形件不做尺寸校验）。
- 读取并进 `machine_dims`：`MachineTables.calibration_models` + `load_presets_dir` 顺手读它，
  **与机型表同一条逃生链**（显式 `install` → `MKPSE_PRESETS_DIR` → 仓库相对 `presets/`）。
  `calib_models::load` 把坏表挡在装载期：`deny_unknown_fields`、逐档点名缺键（ZOffset 缺 `datumY`、XY 缺 `pattern`）、id 撞名。
- `disk/calibration.rs`：删掉那一组常量，改为 `resolve_profile(mode, bbox)` 认档 —— 同一朝向命中就用该档；
  宽高互换命中 ⇒ `E_CAL_MISMATCH_001`（旋转）；都不命中 ⇒ `E_CAL_MISMATCH_002`，**正文把已知档位的外形尺寸列出来**
  （缩放还是换档，用户自己就能看出来）。基准/间距/数量/质心期望全部读档。
- 新档的基准：`datumX` 不变（0.079）；**`datumY = 3.755 + 1 = 4.755`** —— 包围盒低沿往下移了 1mm，而图案要落在
  同一个物理点上；方块间距 11.0 / 数量 11 / 质心期望不变。写成 3.755 的话模型照样认得出、图案却整体印偏 1mm，
  所以判据 `z_new_breaks_the_extended_model_into_its_own_datum` 专门钉这一点。
- 资产与前端：`presets/assets.toml` 加 `zoffset-calibration-ext` —— **正式 3mf 已入库**
  （159132 字节、sha256 `b22822cb…`；实测打印区网格 120.000 × 16.484 ⇒ 切片预期 119.579 × 16.063，
  与 `zoffset-ext` 档的 119.58 × 16.063 一致，占位那次已换成真字节）。
  重算 catalog（`cargo run --bin gen-catalog`：随包 25 份文件；交付面 revision `6374b7c4…`）；IPC `MODEL_DEFS` 加 `z-ext`；
  校准页「打开模型」模态框加「模型版本」（旧版 / 加长版）两颗 —— 选版本只换要打开的那份 3mf，
  板面预览两版共用一张（版式相同、只差 3mm 深）。mock 同步补一条。
- **登记的边界**：① 这两份档位的 3mf 是同一块板的两个尺寸版本，后处理**只按外形尺寸认档** ——
  旧模型整体放大 3mm 会被认成加长版（作者 2026-10-18 认可，记为已知局限）；
  ② 校准模型档走的是机型表那条读取链，`machine_dims::install()` 在发布物里的接线（M5 欠账）也是它的前置。

**验证**

- `cargo test -p mkpse-postprocess`：**全绿**（208 lib + 各集成套件，含 `tests/reference_calibration.rs` 的
  逐字节判据 —— 旧档数值逐字照搬，所以「迁移零变」有据）；新增档位用例：加长版认档 + 基准生效、认不出时列清单。
- `npm run build`（`tsc -b` + vite）、`npm run lint`（eslint + stylelint）、`cargo check -p mkp-support-ease`：绿。
- 剩一条**真机验收**：拿作者那份 `.44432.0.gcode` 跑一遍后处理 —— 应不再报 `E_CAL_MISMATCH_002`，
  且插入的 11 个方块相对旧版同参数**只在 Y 基准上平移 1mm**。

## 增量之四十 · 自定义数据源认仓库 / 分支页（2026-10-18，分支 `feat/dark-mode`）

**症状**：设置页「预设数据源 → 自定义地址」贴 `https://github.com/MuCoreBenC/MKPSupportEase/tree/feat/dark-mode`
点「应用」→「这个地址取不到预设数据，没有改，现在用的还是 GitHub」；校准页「打开模型」于是报
「目录里没有 ZOffset_Calibration_ext.3mf」—— 客户端读的一直是 `main`，加长版只在分支上。

**根因**（两层，第二层才是真原因）：

1. 网页地址 → raw 的转换本身是通的（2026-10-16 那条），`tree/feat/dark-mode` 能变成
   `raw.githubusercontent.com/…/feat/dark-mode`；
2. **但 `source.json` 从来不在仓库根**，它在 `presets/delivery/` 下面 —— 候选表只试了
   `<ref>/source.json`（真机 404），所以必然失败。`curl` 实证：根 404、`presets/delivery` 200；
3. 附带隐患：落盘只有 `customUrl + customShape`，重启后要**按形状再推一遍**才解析得回来 ——
   推导是第二套判据，推不回来就是"设置页看着成、实际读的还是 main"。

**治法**：

- `runtime::source` 加 `CustomShape::Repo`（仓库 / 分支页）+ 约定常量 `REPO_DELIVERY_DIR = "presets/delivery"`；
  候选表顺序 = 按约定（`…/<ref>/presets/delivery/source.json`）→ 仓库根 → 交付目录。
  `tree/` 后面**不假设**第一段就是 ref（`feat/dark-mode` 与"ref + 子目录"在 URL 上同形，
  两条解释都留给候选表）；人已经指到交付目录本身或指着某个文件就不再猜。
- 探通的那一份**存进设置**（`PresetSource::custom_resolved`，加字段不升号）：解析第一依据是它，
  `customShape` 只在老档兜底（`bootstrap_candidates` 稳定排序把记下来的形状排最前 ——
  推导仍只有一张候选表）。用户填的 `customUrl` 原样留着，界面显示与「查看更新」跳转都用它。
- 设置页文案 / 占位符补一句：仓库 / 分支页（`…/tree/<分支>`）也认。

**判据**（`runtime::source::two_source_tests`）：
`a_tree_url_tries_the_repo_delivery_dir_first`（候选顺序 + 指到子目录不再猜）、
`a_branch_page_is_probed_and_remembered`（探 → 落盘 → 重启读入口，实际读的那一份跟着走）。

**顺手修掉三笔既有红**（都是 `eaba9fb` / `8e6ba47` 带进来、CI 会拦的；作者看完报告说「可以修」）：

1. `crates/postprocess` 的 clippy `collapsible_match`（`eaba9fb`）：`calib_models.rs` 的
   `"Repetition"` 臂是「只有一个 `if` + `return`」⇒ 合并成 match guard，行为等价；
2. `cargo fmt --check` 四处（`eaba9fb` 三处：`calib_models.rs:219`、`disk/calibration.rs:818`、
   `postproc/mod.rs:13` 的 `mod` 字母序；`8e6ba47` 一处：`zorder.rs` 的块注释缩进）——
   **块注释那处 1.97 与 1.99 的要求相反**（对齐 `/*` vs 拉回代码缩进），所以改成 `//` 行注释，
   两个 rustfmt 版本才都认；
3. 工作台四条判据的期望值还停在「3 个模型」（`assets.rs` 总条数 31 / model 3 / 可选 10、
   `delivery.rs` 目录 15 条）：`eaba9fb` 加进 `zoffset-calibration-ext` 之后应为 32 / 4 / 11 / 16。

**验证**：`cargo test`（工作区，默认 feature）**全绿**；`cargo test -p mkp-support-ease --features workbench --lib`
**966 全绿**；`cargo clippy --all-targets -- -D warnings` 两档 feature（默认 / workbench）+ `+1.99.0` 全 **exit=0**；
`cargo fmt --all -- --check` 在 **1.97.1 与 1.99.0** 下都干净。

**真机待验收**：设置页贴分支页 → 「当前：你指定：…/tree/feat/dark-mode」→ 校准页「从云端获取」
拿到 `ZOffset_Calibration_ext.3mf` → 「打开模型」能开。

## 增量之四十一 · 交付错误带码 + 内容区给"怎么办" + 开发台故障注入（2026-10-19，分支 `feat/dark-mode`）

**三句话**：① 用户能读到的句子必须带稳定码；② 只给错误标题不够，内容区要给**怎么办**；
③ 开发台要能**真实地**触发任意模态框的错误态。

### §1 错误码与文案瘦身（`src-tauri/src/error.rs` / `runtime/delivery.rs`）

- `AppError` 新增 `messageCode`（稳定码 `E_*_NNN`）+ `AppError::text()`（人话 + 尾注码）
  + `text_err()`；`ipc::traced` 交前端的 `message` 从此**自带码** ⇒ 前端所有 `errorText()`
  调用点天然带码，**不用逐处拼**（两份都带就成了「（码）（码）」）。
- 交付链先盖三个码：`E_DELIVERY_STALE_001`（目录过期）/ `_SHA_001`（文件真坏）/ `_SIZE_001`（大小不对）。
- **文案瘦身**（作者：「这段话这么长」）：界面那句一个技术值都不摆 —— 两串 64 位 SHA 与两个
  revision 全部下线到 `detail` + `tracing::warn!`。原来那句把排查用的因果解释塞给了用户。

### §2 内容区的「怎么办」（前端）

- 前端 `src/api/errors.ts` 出 `UiError{text,code}` + `uiErrorOf()`；两页 `modelErr` 从
  `string | null` 改存整条。
- 新增 `src/app/shared/staleCatalog.ts`（`STALE_CATALOG_CODE` + `isStaleCatalogError`）
  + `StaleCatalogHelp.tsx`（内容区「怎么解决」块，绿 `--ask-*`，**只给动作不解释成因**）。
- ★ **按码出现、不按文案猜**：目录过期 → 给"重启"这条路；文件真坏 → 重启没用，**不许给**
  （指错路比不指更糟）。判据是那个码，不是句子。
- 两页（校准页 / 首页）同时删掉自挂的 `window keydown` 关 Esc —— `Modal` 自己就接 Esc，
  原来一次 Esc 会连关两层。`ExportModal` 那条重复的框身红字删掉，只留标题格。
- 判据/常量与组件分两个文件：`react-refresh/only-export-components` 要求 `.tsx` 只导出组件
  （同 `note.ts` / `NoteBar.tsx` 那套）。

### §3 开发台 · 故障注入（**真机制**，不是 UI 预览）

- **挂在 API 方法上**（作者选的粒度）：一个开关覆盖所有走这个方法的界面，产品代码一行没改。
- `src-tauri/src/devfault.rs`（**无条件编译**，与 `devsim` 同路 —— 造错发生在客户端进程，
  而客户端不带 `workbench` feature）+ `workbench/app/devfault.rs`（三条命令薄壳）。
- 过线：`FromHook::DevFault { fault: Option<String> }` —— **只过 id，不过 `AppError`**，
  错由客户端进程 `devfault::armed()` 自己造 ⇒ **演示的错与真机那条一字不差**
  （为此把 `delivery` 那三个句子/码函数提成 `pub(crate)`，有测试钉住这条）。
- 四档：`delivery-stale` / `delivery-sha` / `delivery-size` / `open-model-stale`。
- 工作台 `DevPage` 加了「故障注入」区（装 / 卸同一颗按钮）。加一档只改 `devfault::faults()`。
- ★ **诚实边界**（作者强调）：它拦在 API 调用层，**不代表真实业务真的执行失败了** ——
  被拦的那次调用可能根本没进后端。这是开发测试用的故障注入，不是真去破坏文件或网络。

### §4 未做：后处理预览档（**只处置了一半，剩下的另开交接**）

- 查证到的硬约束：`HookView::cancel()` → `write()` 在 `!hook_connected()` 时报
  「这一趟已经结束了，没有正在跑的处理」；而演出时钩子根本没连 ⇒
  **那几颗真业务按钮点了必然报错**。所以 10-14「预览里不摆真业务按钮」那条裁定站得住，
  **不是"忘了接"，是接不上** ⇒ 「换成真按钮」不可行。
- 作者的判断（接受）：**不是实现不了真的，而要分清"已实现成真功能"与"只是演示场景"**。
  故障注入属于前者，后处理预览属于后者。真正要的是**从工作台触发真实后处理流程** ——
  与"后处理预览档"是两回事。
- 撤预览档（作者选的方向 A）**本轮没做**：建议等真实入口做完再撤（有对照物）。
- ★ 交接文档：[`docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md`](docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md)
  —— 需求 / 六条验收判据 / 三步路径 / **待核三件事**（真实入口在哪、A1 基准 G-code 在哪、
  现有 hook 协议够不够起一趟真任务）/** 未验证的假设**（工作台直接 spawn 真钩子，可能不必加协议变体）。

**验证**：`cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings`（默认 /
`workbench` 两档）、`cargo test` **598 全绿** / `--features workbench --lib` **975 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` 全绿。

## 增量之四十二 · 开发台「运行真实后处理」（② 落地，2026-10-19，分支 `feat/dark-mode`）

> 接着增量之四十一 §4 那份交接做。交接文档 `docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md`
> 的 **§4.3 已核完、§6 那个假设已验证成立、§4.4 是这一刀的落地记录**。

**一句话**：工作台「开发台」第二块 = **真的跑一趟后处理** —— 选一份预设 + 一份测试 G-code，
工作台 spawn 一个**真的钩子子进程**，客户端窗口跑真管线、显示真进度，界面上**没有"预览"标记**，
那几颗真业务按钮**点了真的有用**。

### §1 先核的三件事（交接文档 §4.3）

1. **真实入口** = 本程序带参数起子进程：`"<本程序>" --Toml "<预设.toml>" --Gcode <输入.gcode>`
   ⇒ `args::Mode::Hook` ⇒ `hook::run_with_channel`（真跑 `process_with_ir_with_tower_ask`，
   产物**原地覆盖**输入，归档真写 `user/gcode_history/<日期>/`，退出码 0/1/2）。
   ★ 顺带澄清：**内核 `mkpse-pp` 不在这条路上** —— 它是诊断 CLI，钩子是进程内调管线、不 spawn 它。
2. **那份 A1 基准（约 23.8 MB）没找到**。本机最大真件是
   `crates/postprocess/tests/fixtures/real/.5926.1.gcode`（**8.5 MB**，BambuStudio 切的 A1 件）。没新造测试数据。
3. **hook 协议够用 ⇒ 一个变体都没加**。真钩子本来就是独立进程（假钩子 `src/bin/hook-sim.rs`
   就是先例），靠 `internal_root_headless()` 认客户端那棵树连 loopback ⇒ 工作台 spawn 一个带
   `--Toml/--Gcode` 的子进程即可，客户端收到的就是真 `Hello/Progress/Question/Finished`，
   `preview` 恒 false。**客户端侧与产品逻辑一行没改。**

### §2 落点（全是工作台这一侧的薄壳）

- `src-tauri/src/workbench/app/devreal.rs`（**新**）：`wb_dev_run_real(toml, gcode)` ——
  ① 校验两个文件 ② 复制副本到 `<客户端内部根>/run/dev-run/` ③ `connect_or_headless` 探客户端在不在
  ④ spawn `current_exe() --Toml --Gcode`（不等）。含 5 条单测。
- `workbench/app/mod.rs`（模块声明）+ `lib.rs`（命令清单，`command_list_parity` 那道闸）
- `src/workbench/api.ts`（`DevRunAck` + `devRunReal`）+ `views/DevPage.tsx`（新区块）
- `src-tauri/capabilities/workbench.json` 补 `dialog:allow-open`（工作台要开系统文件选择器）

**四个取舍**（都写在 `devreal.rs` 模块头）：spawn `current_exe()`（与真机同形，产品同名产物、
都走 Hook 分支）；**必须先复制副本**（真后处理原地覆盖，不复制则第二趟跑的是已处理产物，
"可重复跑"当场失效）；**先探客户端再 spawn**（不替用户拉进程）；stdio 全 null。

### §3 实测（次日真机，8.5 MB 那份 + `A1-fastv3.3.toml`）

```text
处理完成：…/run/dev-run/1792355145-5926-real.gcode（8435 ms）
钩子来了：这一趟开始 gcode=… preset=A1-fastv3.3.toml        ← 客户端 hook_ui 真路径（不是 run_preview）
钩子那一趟结束 exit_code=0 ok=true cancelled=false
归档：user/gcode_history/2026-10-19/…{,.gcode,_meta.json,_original.gcode}
```

**六条判据**：真任务 ✅ / 真界面 ✅（走 `hook_ui` 真路径，`preview` 恒 false）/
真按钮 ⚠️（**链路已成立**：真有一条连接 ⇒ `Handle.connected()` 为真 ⇒ 不再报"没有正在跑的处理"；
最后一步要在客户端点一次「停止」**人工确认**）/ 够慢 ❌（**8.4 s**，一闪而过）/
可重复 ✅（副本机制）/ 不进正式包 ✅（`workbench` feature）。

★ 一个容易误会的事实：**工作台的沙箱管不到这一趟** —— 沙箱是模块级 `AtomicBool`、不写环境变量，
子进程不继承 ⇒ 钩子进程走**客户端正式那棵数据树**，会真写执行记录与归档。界面已写明"别拿正式数据试"。

### §4 待拍板（交接文档 §7）

1. ✅ 三件事已核完 ⇒ 不加协议变体。
2. ⏳ **撤预览档**：可以撤了，但建议**先留着当对照物**（真 vs 演，一眼看出"预览"标记与按钮差别）。
3. ❌ **减速开关要不要做**：8.5 MB 只 8.4 s，**不够慢**；release 只会更快。三条路 ——
   ① 换更大的件（要 30 s+ 得 30 MB+，作者那份 23.8 MB 没找到）；
   ② `#[cfg(feature = "workbench")]` 的 `--dev-slow-ms`（只在进度事件之间 sleep：只改时序不改状态，
   正式包编不到，与 `DevScene`/`DevFault` 住产品文件同一类格局）；③ 不做。
   **我倾向 ②**，但它要在 `args.rs`/`hook.rs` 开 workbench-only 分支 —— 不算零成本，**等你点头**。

**验证**：`cargo fmt --all --check`、双档 `cargo clippy --all-targets -- -D warnings`（默认 /
`workbench`）、`cargo test -p mkp-support-ease --lib --features workbench` **981 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `npm run build:workbench` 全绿。

## 增量之四十三 · 开发台「真实后处理测试矩阵」六档（2026-10-19，分支 `feat/dark-mode`）

> 接着增量之四十二（② 单趟入口）做的第二层。作者：「我怎么触发它成功、怎么触发它失败、
> 怎么触发它碰撞检测？模拟出来的真实可以做到吗？落盘的话，也到时候是用临时文件」。
> 交接文档 `docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md` **§8** 是需求 + 方案 + 落地台账。

**一句话**：六档确定性输入各跑一个**独立临时根**，串行、跑完只按 `_meta.json` 真凭据判定；
问句档（不匹配 / 碰撞 / 取消）**要人在客户端窗口真操作**，数据一个字节不碰正式树。

### §1 六档与实测

| 档 | 判定（只读 `_meta.json`） | 实测 |
| --- | --- | --- |
| 小文件成功 | `error=null && !cancelled` | 101 ms / 9 步 complete |
| 大文件成功 | 同上 | 8435 ms |
| 硬失败 | `error` 含 `E_` | 坏 TOML ⇒ 退出码 2 + `E_TOML_PARSE_001` |
| 挪塔碰撞 | **`relocation != null`**（没有就判失败，不看退出码） | `relocation.choice=timeout`、冲突点 `[30,30]`、建议 X 挪 22.8mm、`W_COLLISION_RELOCATE_002` |
| 机型不匹配 | 人工档：非 `E_` = 问过了 | 待客户端在听时真答一次 |
| 取消 | `cancelled && W_SYS_CANCELLED_001`，别的都不算 | 待人工点停止 |

### §2 落点

- `src-tauri/src/workbench/app/devmatrix.rs`（**新**）：档位表 / `prepare`（临时根 + 抄端口 + 抄偏好
  + 复制夹具）/ 串行 worker（`CURRENT` + `BUSY` 两个静态 + `Child::wait`）/ `judge` / `recent_verdicts`
  / 清空（`BUSY` 时不删）。6 条单测（碰撞要有 relocation、取消只认那个码、没记录一律失败、成功要干净、硬失败要 `E_`、档表 6 档不重）。
- `devreal.rs`：抽出 `spawn_hook(toml, gcode, data_root)` —— **矩阵"数据落临时根、消息发客户端"的全部秘密**
  就一个字：`data_root/run/hook.port` 里写的是客户端端口（端点只是一行文本）。单趟入口也改用它。
- 四条命令：`wb_dev_matrix_cases` / `_state` / `_start`（空数组 = 全部）/ `_clear`。
- 前端：`api.ts` 三个类型 + 四条封装；`DevPage` 加「真实后处理测试矩阵」区块（2 秒轮询，不走事件推送）。
- 夹具：`crates/postprocess/tests/fixtures/dev/{small-ok.gcode,collide-tower.gcode,broken-preset.toml,README.md}`。

### §3 三条坑（都进了夹具 README / §8.4）

1. **缺 `printer_model` 不是硬失败**（管线宽容，111 ms 跑完判成功）⇒ 硬失败档改用坏 TOML。
2. **`rib-tall-min.gcode` 不会撞**（没有 `; FEATURE: Support` ⇒ 擦料决策判不需要 ⇒ `use_towers=false`
   ⇒ `detect_tower_collision` 首行 return）；新夹具补上那一行才真撞。
3. **客户端没在听时，机型不匹配走 `Refuse`** ⇒ `E_CFG_MISMATCH_001` 直接硬错、没有问句；
   矩阵会正确地判失败 —— 所以跑矩阵前客户端必须真在听。也印证"只看 `_meta.json`"是必须的。

★ 另一个环境事实：工作台用 `CARGO_TARGET_DIR=target-workbench`（`scripts/tauri-workbench.mjs`），
与客户端的 `target/debug/` 隔离 —— 但**别手动 `cargo build --features workbench`**（会覆盖客户端产物）。

**验证**：`cargo fmt --all` 干净、双档 `clippy -D warnings` 绿、`cargo test` 绿、
`cargo test -p mkp-support-ease --lib --features workbench` **987 全绿**（981 + 6 条矩阵判定）、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `build:workbench` 全绿。

## 增量之四十四 · 校准件接进矩阵 + 问句屏「停止并关闭」（2026-10-19，分支 `feat/dark-mode`）

> 交接文档 §8.5 是这一刀的完整记录。作者拍板：校准件用**仓库现成的 3MF**；
> 「点下去已经晚了」按**加一颗「停止并关闭」**解决（不做排队）；**预览档这一轮不删**，
> 只把「成功·没有结果读数」「成功·好几行警告」两格删掉。

### §1 校准件是怎么"真跑"起来的

1. **身份**：临时根摆成切片器形状 —— `<根>/input/plate.gcode` + `<根>/origin.txt`（一行 = 3MF 绝对路径）。
   `temp_root()` 取 **gcode 的上一级**，所以 `origin.txt` 放 `input/` 的上一级。
   实测记录：`detail.calibration.identity3mf = "ZOffset"`、`verdict = "from_3mf"`。
2. **切片件**：本机 Bambu Studio **有 CLI** ——
   `BambuStudio --slice 0 --outputdir <OUT> presets/assets/models/ZOffset_Calibration_ext.3mf`
   ⇒ `plate_1.gcode`，挤出 bbox **119.511 × 16.063**，正落 `zoffset-ext`（119.58 × 16.063 ±0.5）。
   已落仓 `fixtures/dev/calib-zoffset-ext.gcode`（121 KB），**运行时不再依赖外部工具**；
   重生成方法写进夹具 README（`--export-3mf` 会报错并给 243，不影响 `--slice`）。
3. **两档判定**：`calib-ok` 要 `error=null` **且** `mode`/`profileId` 都在；
   `calib-question` 要 `W_` + `cancelled=true`（实测 `W_CALIB_MODE_001`）。

★ 反直觉：**「问了没答」的停下，`cancelled` 也是 true**（原文件没动）⇒ 这一档不能照取消档判。
★ 顺手修掉真 bug：`err.contains("E_")` 会把 `W_CALIB_MODE_001`（"MOD**E_**001"）误判成硬错 ⇒
  改成 `is_hard_error`（连括号认 `［E_` / `[E_`）。

### §2 「点下去已经晚了」从死局到一键收场

病根不是缺排队，是**报错之后没出路**：点了选项 ⇒ 问句屏让位给 hero（防连点）⇒ 这一答晚了一步
⇒ 红字落在标题右边、问句屏已经收走 ⇒ 既没按钮也关不掉框。

- 新增 `src/app/postprocess/questionUi.ts`（**纯判据**）：
  `showAsk = asking && (!answering || err !== null)`、
  `offerStopAndClose = running && err !== null`。
- `PostProcessModal`：报错时问句屏**放回来**；报错且还在跑 ⇒ **只摆一颗「停止并关闭」**
  （先 `cancelPostProcess()` **吞掉失败** —— 钩子可能已经没了，这颗按钮的语义是"收场"不是"保证停成功"
  —— 再 `dismiss()`）；`answering` **不再因 `err` 松手**（否则按钮闪回来、人接着点接着错）。

### §3 删两格预览场景

`postprocess-success-no-metrics`（真跑一定算得出读数，摆着会误导）与
`postprocess-success-long-warning`（要凑内核警告条件、且那是排版问题）。
`devsim::scenes()` / `src/app/dev/scene.ts` / `hook_ui.rs` 三处同步删；
`devsim::Options` 的 `metrics` / `many_warnings` 两个**能力留着**（演出引擎不因少两格就拆）。
其余 13 格一条未动。

### §4 未验的与下一步

- `calib-ok`（认成校准件 + 认到档 + 跑完）要在客户端**真答一次** —— 这一"答"没法自动化。
- 「停止并关闭」需在真机制造一次"答晚了"（例如碰撞档等超时后再点）。
- 下一轮：等这两条真机验完，再撤预览（作者：「等校准件到位再撤」）。

**验证**：`cargo fmt --all -- --check`、双档 `clippy -D warnings`、`cargo test`、
`cargo test -p mkp-support-ease --lib --features workbench` **989 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `build:workbench` 全绿。
