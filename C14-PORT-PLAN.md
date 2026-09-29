# C14 → 产品仓移植方案（工作台前端接通 = HANDOFF 14b + C14 增量）

> 起草：2026-09-29。起草时产品仓在 `feat/b04-p3-migration`（14a 已闭环，见 `HANDOFF.md`）。
> 本方案是 **14b 的施工图**，同时把试验场 C14 这二十八轮攒下的界面增量一次搬齐。

## 0. 已定决策（2026-09-29 与作者确认）

| # | 决策 | 内容 |
|---|------|------|
| 1 | 路线 | **整体移植进产品仓**，接现有 Rust 后端。不搞跨仓实时对接（浏览器里的 C14 够不到 Tauri `invoke`，双份契约永远同步不划算） |
| 2 | 范围 | **全量对齐 C14**：HANDOFF 14b（menu/build 视角、资产模态框、闸门按钮）与 C14 界面增量一次收口，不分两波 |
| 3 | 契约 | **以产品仓 `wb_*` 为准**。先出差异清单（§4），Rust 只补增量，不动既有形状与纪律测试白名单 |
| 4 | 落点 | **全部工作在本仓库**，新开 feature 分支走八道闸 PR 流程。试验场仓库一个字不动 |

## 1. 现状一句话

- **试验场 C14**：工作台原型 React 稿。六页（机型 / 参数台 / 对比 / 套餐 / 生成与发布 / 资产库）+ 三只编辑模态（CellEditor / ParamDetail / BatchEdit）+ 一批共用件（GcodeEditor、Modal、Splitter、AssetPicker）。数据是 `store/data.ts` 里的**生成快照**（假后端数据编译进页面），判定在 `store/derive.ts` / `cellWrite.ts`（前端）。
- **本仓库**：Rust 后端 38 个 `wb_*` 命令全部在册（14a 闭环），纪律测试（写入口唯一、基线单写路径）就位；前端只接了三个视角（machines / params / compare），menu 视角还是「Task 16 落地」占位符。
- **两个已核实的关键事实**：
  1. C14 的 `store/data.ts` 文件头明写「**字段形状对齐 MKPSupportEase/src/workbench/api.ts（真实后端契约镜像）**」——两边契约早就是对着的，不是两套语言。
  2. C14 `store/state.ts` 文件头明写「那些判定搬去真后端时会**一起搬走，UI 一行不用改**」——C06 起原型就是按这次搬运设计的。**这次移植就是把 derive 搬进 Rust 已有的派生层、把 UI 换个数据源**，不是重写。

## 2. 页面对照表（移植映射）

| C14 页面 | 产品视角 | 走哪些 `wb_*` | 现状与动作 |
|---|---|---|---|
| MachinesPageC14 | `machines` | `wb_machines` + add/remove/set_version_field/set_machine_field/copy_version/orphans | 三视角已接。**以 C14 版替换**：右键菜单、复制版本两步流（14.3/14.5 的 UI）、参数源待补标（`hasRecipe`）等增量对齐 |
| ParamsPageC14 | `params` | `wb_desk` + `wb_registry` + `wb_apply_draft` | 已接 ParamDesk。**以 C14 版替换**：G-code 两层编辑器 + 放大模态框、弃用标记（划线/徽章/选项级）、行高规矩、`resize:none` 等二十八轮细节 |
| CompareMatrixC14 | `compare` | `wb_matrix` + `wb_preview_bulk` | 已接 ParamsMatrix。以 C14 版对齐（矩阵四处修正、批量预览模态） |
| BundlesPageC14 | `menu` | `wb_bundles` + `wb_set_bundle`（Patch）+ `wb_set_version_field('recommendedBundle')` | **产品是占位符，这次落地**（14.1 的一半）。C14 第十九轮已定「一版一套」：套餐按版本建，版本 `recommendedBundle` 指过去 |
| BuildPageC14 | `build` | `wb_preflight` / `wb_generate` / `wb_publish` / `wb_revert_preview` / `wb_preview_toml` / `wb_baseline_diff` / `wb_sync_baseline` / `wb_dist_strays` / `wb_clean_dist_strays` | **落地**（14.1 另一半）。14.2 闸门按钮：生成/发布按钮消费 inspect 结果做禁用约束（后端硬闸已在） |
| AssetsPageC14 | `stock` | `wb_stock` + `wb_assets` + `wb_asset_usage` + Patch `setVisibility` | **落地**（14.6）：资产选择模态框接 `wb_assets`，不让人手填路径。切片器三根轴（切片器·喷嘴·层高）见 §4-3 |
| 回退 / 回收站 | （含在 stock/build 视角内） | `wb_fallback` / `wb_trash` | 命令已在，UI 随页搬 |
| 壳（AppC14 导航/状态栏/抽屉） | shell | `wb_boot` / `wb_reload` / `wb_words` / `wb_ui` / `wb_save_ui` | 产品已有壳；C14 的徽章（dirty/build/publish）、问题跳转映射（`IssueView → 视角`）照搬 |

## 3. 架构差异三条，搬运规则随之而定

1. **派生位置反转**：C14 的 `derive.ts` 在前端算（showWhen、blocked、origin、stale、deprecatedValuesOf……）；产品契约是「**前端不算业务**」——状态、文案、来源、可否编辑全部由后端算好给（`Row`/`Cell`/`Desk`/`Words`）。
   → 规则：`derive.ts` / `cellWrite.ts` 的**判定全部不搬**，UI 只认后端视图字段；C14 判定里产品契约还没覆盖的，进 §4 增量清单让 Rust 补，**不许在前端重新长出来**。
2. **写模型几乎同构，直接对上**：C14 = 「一份工作态 + 一条历史 + 一个 save」；产品 = `wb_apply_draft(label, Patch[])`（返回 inverse 供撤销）+ `wb_diff_draft` + `wb_save`/`wb_discard`。
   → 规则：C14 的动作函数改成「把手势翻成 `Patch[]`」，撤销栈留在前端会话内存（吃 `inverse`），**全应用仍然只有 `wb_apply_draft` 一条写路**。
3. **数据来源**：C14 读静态种子，产品每次 IPC 且 `Refresh` 参数能让一次写顺带带回新页。
   → 规则：一次手势一次 IPC，能带 `refresh` 的都带；`wb_words` 开场取一次，前端**禁止**再写第二份词表。

## 4. Rust 侧增量清单（以 wb_* 为准，只补这些）

> **P1 施工中新增的发现（2026-09-29）**：机型页移植时逐条对过 C14 的动作面，
> 下面这批原型功能**产品后端没有对应命令**（C14 的弹窗副标题自己也写着
> 「真后端还没有 wb_clone_machine / wb_remove_machine」），P1 一律**不渲染入口**，
> 是否补后端由作者裁决：
>
> | C14 动作 | 后端现状 | 备注 |
> |---|---|---|
> | 改机型 id / 改版本 id | 无（产品语义：改 id = 删+加） | 原型是一步原子改名 |
> | 复制机型 / 删除机型 | 无 | 原型副标题明写「替后端摆需求」 |
> | 尺寸九格编辑 | 无（MachineView 只给 hasDimensions / zoneCount） | 尺寸卡降级为只读状态 |
> | 版本图 | 无（VersionField 白名单没有 image；原型本地字段） | |
> | 改配方文件名 | 无，且 G-2 已裁决**删除** presetFile | 不移植 |
> | 就地新建套餐 | 无（bundles.toml 的写命令还没开） | 「选择已有套餐」已接 |
> | 就地改套餐内容 | **模型在迁移中**：`Patch::setBundle` 写的是上游 manifest 套餐空间（stock id），而版本 `recommendedBundle` 指向 `bundles.toml`（资产域 id）—— 两套套餐概念待 doc §4.2 的 menu/stock 映射收口后随 P4 一起接 | |
>
> 另两个核实结论：`wb_add_machine` **三格都必填**（catalog.rs 逐格拦空），C14 弹窗
> 「只有 id 必填」是原型许愿，移植版照真契约；版本/机型 id 字符集是
> `大写字母/数字/下划线`（validate_new_version_id），比 C14 的 idOk（允许点）更严。

| # | 项 | 证据 | 动作 |
|---|---|---|---|
| 1 | `words.rs` 补「已弃用」一档 | 全 `src-tauri` grep 无 `paramDeprecated`/弃用话术；C14 README 点名「它不在后端 words.rs 里，是原型先加的一档，**接后端时要带过去**」 | 词表加键：行标记、徽章、写值拦截话术（「已弃用，不能改」+ 选项级那句） |
| 2 | 选项级弃用判据 | C14 `derive.deprecatedValuesOf`：选某一档后，被它放开的参数**全部**已弃用（且至少放开一条）→ 该选项标记弃用。产品 `ChoiceView.deprecated` 字段已在，判据是否一致**实现时核对** | 在 Rust 派生层补齐/对齐；实测只命中 `外围结构=护套` 一档 |
| 3 | StockRow 切片器轴 | C14 第二十八轮：资产库按 切片器(BBS/Orca)·喷嘴·层高 三根轴筛。产品 `StockRow` 有 `nozzle`/`layerHeight`，**无 `slicer` 字段**（`AssetView` 才有） | 核实后给 `StockRow` 补 `slicer`（或等价派生），三根轴的筛选在后端做 |
| 4 | menu/build 视角映射 | HANDOFF 14.1：后端硬闸就位、menu/build 待前端接（doc §4.2 映射表） | 按映射表接通；发现缺口先记本表再补。**P1 新发现**：套餐内容编辑牵涉两套套餐空间（见上表末行），P4 动工前先收口这个映射 |
| 5 | 14.7 workbench 子目录职责 | `bbs/` 零读写、`.snapshots/` 只写不读等 | 定职责并在视图上显式（不留给前端猜） |
| 6 | 上表「待裁决」七项 | P1 机型页逐条核对的结果 | 作者逐条勾：补后端命令 or 放弃该入口 |

纪律注意：`write_discipline_scan.rs` 的白名单是**相等断言**，新增命令必须登记；能不新增命令就不新增（§2 表里 90% 的行零新增）。

## 5. 分阶段计划（每阶段一个 PR，走八道闸）

| 阶段 | 内容 | 验收 |
|---|---|---|
| P0 | 本方案 + §4 清单确认 | 作者过目 |
| P1 | 分支 `feat/b05-14b-c14-port`；壳对齐（boot/words/ui）；机型页以 C14 版替换（含复制版本两步流 14.3/14.5 UI） | lint + cargo test/clippy 绿；机型页操作全部走既有命令 |
| **P2 ✅（2026-09-29，见 §8）** | 参数台：`wb_desk` 读路径 + `wb_apply_draft` 写路径 + G-code 编辑器/模态框 + 弃用全套（含 Rust 词表 §4-1/2） | C14 README 里「量过的」逐条复量 |
| P3 | 对比矩阵 + 批量编辑模态（`wb_preview_bulk`） | 同上 |
| P4 | 套餐（menu 视角落地）+ 资产库（stock 视角 + 资产选择模态框 14.6 + 切片器三轴 §4-3）+ 回退/回收站 | 替掉「Task 16 落地」占位符 |
| P5 | 生成与发布（build 视角 + 14.2 闸门按钮 + 基线 diff/同步 + 残留清理）+ 14.7 | 闸门按钮与后端硬闸行为一致 |
| P6 | 端到端验收 = doc §4.3 全部 11 步走查（14.8）；HANDOFF 收口、Task 14 整体勾选 | 11 步全绿 |

## 6. 风险与既知的债

- **不碰试验场**：C14 及其 `src/server` 原样保留（原型继续可跑），后续作者若再迭代，按「每轮人工搬一次」的节奏另行处理——本方案不建立自动同步。
- **词表唯一出处**：C14 `store/words.ts` 只把产品缺的那几条**译进 Rust**，前端不留第二份；否则 Rust 改措辞界面还是老词。
- **C14 data.ts 的手补债在产品侧不存在**：`deprecated` 7 条与第二十八轮的 Orca 两份，在产品侧都从注册表/资产表直接读，不需要任何手补。
- **公开仓库 / AGPL**：本仓库 public 且 main 只能 PR 推进——试验场代码是同一作者的移植，无授权问题，但任何密钥、本机路径中的用户名不得入库。
- **规模最大的单块是 P2（参数台）**：C14 这块积累了十八轮以上的量过判据，搬运时逐条对照 C14 README 的「量过的」清单验收，不凭感觉说像。

## 7. 待作者确认

1. §5 的阶段划分与 PR 粒度是否可以（也可以 P1+P2 并成一个大 PR）。
2. 产品现有三视角（machines/params/compare）是 **C14 版整体替换**还是只搬增量？本方案推荐**整体替换**——「整体移植、不重写」是 v23 以来的既定原则，C14 是作者最新拍板的稿。
3. 分支名 `feat/b05-14b-c14-port` 是否合意。

## 8. P2 落地记录（2026-09-29）

### 8.1 交付了什么

- **前端**：`views/ParamsPage`（C14 版式：页头把手 + 三态左树 + 页签 + 分组卡 + 行内控件 + 按需右栏 + 两条拖线）替换旧 `ParamDesk`；`views/CellEditor` / `views/ParamDetail`（三栏工作台：各版本取值就地改 + ↶ 挂回继承）移植；`c14/GcodeEditor` + `c14/gcode.ts` + `c14/GcodeModal`（双层上色、行号槽、模态框只由角按钮开）移植。旧 `ParamDesk.tsx` / `ParamsMatrix.tsx` / `views/FieldControl.tsx` 删除。导航徽标读数、状态栏撤销/重做/未保存改动沿用 P1 外壳。
- **Rust**（全部登记进 §4，无新增命令）：
  - §4-1 词表：`paramStatus` 四档（出厂默认/机型默认/本版修改/已修改）、`paramDeprecated` / `paramDeprecatedChoice` 两枚、`disabled.deprecatedWriteBlocked`，`relate::blocked_hint`（行上短句「要 X 等于 Y 才可改」→ `Cell.blockedHint`）。
  - §4-2 选项级弃用：`registry::deprecated_choice_values`（判据照 C14：这一档放开的参数全弃用且至少一条，`op = eq` 才算「放开」）+ 上游标的取并集，从 `wb_registry` 的 `ChoiceView.deprecated` 出。
  - `registry::desk_keys`：**弃用的参数在参数台/矩阵这一屏不再隐藏**（C14 §五：划线+禁用，不藏）；生成侧照旧走 `visible_keys` 不进产物。`Layers::view_effective` 让弃用的键照样读得到值。
  - 弃用写闸：`patch::validate` 拒绝往弃用参数写值 / 写弃用的选项档（`null` = 挂回继承不拦）。前端在手势前用同一批后端字段拦一道、只负责说那句人话的 toast。
  - `Desk` 增量（C14 三栏工作台需要的）：`cols`（基底+全部版本各一列）+ `cur`（请求层下标）+ 每行 `impact`（作用域：改这里影响谁/谁在跟着基底）。
- **开发桩**：`src/workbench/dev/mockBackend.ts`（只在 vite dev 且无 Tauri IPC 时安装）——浏览器里复量「量过的」清单用的手写夹具，生产构建摇掉。

### 8.2 「量过的」复量结果（浏览器 + 开发桩，Ultra 1760×900 / Compact 900×640）

| 项 | C14 定稿 | 本仓复量 |
|---|---|---|
| 行高（Ultra） | 全是 43 | **全是 43**（步进/分段/弃用行一致） |
| 行高（Compact） | 全是 39 | **全是 39** |
| 分段控件 | Ultra 11.5px\|10\|23\|白；Compact 10px\|10\|21\|白 | 11.5/10/22.9/白；10/10/20.6/白 |
| 溢出 | Ultra 0 / Compact 0 | **0 / 0** |
| G-code 行上框 | 172×88、resize none、spellcheck=false | **172×88 / none / false** |
| G-code 模态框 | 两层滚动尺寸相等、行号槽=行数 | **591×412 = 591×412**、9 行样本 9 行号 |
| 弃用四处落法 | 红线 rgb(200,69,60)、徽章 64×18 横放、控件禁用、抽屉「状态」第一行+各层「已弃用」、写值拦截 | **逐项一致**；「护套」点得动但存不进去（toast + 值弹回 + 脏计数不动） |
| 前置条件 | 值全是中文选项名、零裁切 | 「要 擦料方式 等于 擦料塔 才可改」，后端拼句 |

### 8.3 与原型的差别（记录在案的取舍）

- **单版本模式只有一档**：页头的「单版本 / 版本对照」两枚模式钮与左树勾选随 P3 的矩阵/批量一起回来（对照列 `机型|版本` 记号、`usePaintSelect` 刷选都在那边才用得上）。
- **组的「仍然展开看」折叠没有搬**：C14 版式的答案是不折叠——不适用的行原地置灰、每行自己的 `blockedNote`/`blockedHint` 把话说完（「位置一跳，人就记不住」）。后端的 `offNote` 字段还在，P3 若要可随时接。
- **抽屉「出厂默认」一行读 `ParamView.defaultText`**（真出厂默认），不照 C14 读基底值——原型那一格读的是基底，标签却是「出厂默认」，照产品语义纠了过来。
- **参数台不再有默认选中**（C14 第十七轮「默认谁都不选」）：外壳不再在启动时塞第一台机型的基底，空态留白 + 把手提示；「编辑参数」跨页定位（goto）不变。
- **工具链发现并修掉的一个前提**：工作台入口缺了试验场 `global.css` 的全局 `box-sizing: border-box`——没有它 C14 量过的数（行高 43、框 172×88）全部虚高几像素。已在 `workbench.css` 里圈 `[data-wb]` 补上。

### 8.4 留给作者的

- §4-6 那批待裁决项仍待勾（P2 没有新增一类）。
- 开发桩（`dev/mockBackend.ts`）是 P2 验收工具；真机验收（`tauri:workbench:dev`）时它不参与。留不留随作者。
