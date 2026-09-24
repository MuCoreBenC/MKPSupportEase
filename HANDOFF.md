# 交接记录：Task 14 工作台接通（MKPSupportEase）

> 起草 2026-09-24（14a 后）；同日 14b、14c 相继收口。**Task 14 全部完成**。
> 适用分支：`feat/b04-p3-migration`（注：仓库里任务文档称 b05，分支名是 b04-p3，同一件事）。
>
> **方向变更（2026-09-24）**：现有工作台前端**冻结在 `e4f85bd`**（Task 14 收口点，可用状态），并行立项全新工作台前端（Task 17 提案，见 tasks.md），不迁就旧 UI 形态、不迁就消费客户端契约；Task 15 的后端（初始化/引导/降级）先行，与前端形态无关。

## 1. Task 14 最终状态

| 子项 | 状态 | 落点 |
|------|------|------|
| 14.1 六页面映射 | ✅ | menu=`MenuPage`（只读接 `wb_bundles`）；build=`BuildPage`（检查/生成/基线/发布四段）；机型/配方/对比沿用既有接入 |
| 14.2 闸门交互约束 | ✅ | BuildPage 按钮禁用：`blocks>0` 禁生成、`artifact!=='fresh'` 或残留>0 禁发布；理由全引后端，前端非第二套判定 |
| 14.3 复制已有版本 | ✅ | `catalog::copy_version` + `wb_copy_version` + `MachinesPage::CopyVersionForm`（模板下拉 + 预填）|
| 14.4 参数源待补 | ✅ | `version_has_variants` + `VersionView.hasRecipe` + 版本卡标注 |
| 14.5 参数正文复制 | ✅ | `wb_copy_recipe` + CopyVersionForm 第二步（失败分态：版本已落盘 / 可重试）|
| 14.6 资产选择模态框 | ✅ | `AssetPicker.tsx`：image/icon 从 `wb_assets` 选**资产 id**，`present:false` 不可选；`recommendedBundle` 改套餐下拉 |
| 14.7 workbench 子目录职责 | ✅ | `bbs/` 移出 `WORKBENCH_DIRS`（零读写核实）；勘误：`.snapshots/` 读写都就位（`wb_generate` 写、`wb_revert_preview` 读）|
| 14.8 端到端 11 步 | ✅ | 判据 `the_eleven_steps_of_doc_4_3_run_end_to_end`（`app/build.rs` tests）|
| 14.9 基线 diff + 同步 | ✅ | 后端 `app/build.rs`；前端 BuildPage（diff→分类展示→确认→sync→**重读 diff**）|

## 2. 14c 端到端验收结论（2026-09-24）

- **判据**：`the_eleven_steps_of_doc_4_3_run_end_to_end` —— 隔离环境按 doc §4.3 逐步走：复制版本（只写定义）→ 待补 → 复制正文（含反向拒绝）→ `storage::load` 真实重建后 preflight 全绿 → 渲染（唯一命名 + 快照与模板逐键一致）→ 基线 diff→sync→重读全绿 → 交付集合闭合 + 发布全链。边界三条照实写在判据注释里（命令层靠静态核对锁定 / 复制值经 JSON 往返的浮点表示 / 新版本不进交付集是上游 manifest 契约，11.9 的 drift 报这一形状）。
- **六条验收重点的核对结论**：
  1. 复制版本分态：成功 / 版本已落盘但正文失败（`NO_SUCH` 反向判据 + CopyVersionForm 重试入口）✅
  2. hasRecipe 随真实参数源翻转（端到端步 6→7 落盘重读断言）✅
  3. 生成/基线/发布全部经对应后端命令（BuildPage → `wb_generate`/`wb_baseline_diff`/`wb_sync_baseline`/`wb_publish`，lib.rs 注册清单核对）✅
  4. 基线确认前零写入（diff 只读判据 + 确认区才调 sync）、完成后重读 diff（BuildPage.sync → loadDiff）✅
  5. 残留清理后重读清单与 book（cleanStrays → distStrays 重拉 + onView 刷新 artifact）✅
  6. `bbs/` 移出 bootstrap 无回归（paths 6/6、lib 313/313、写纪律 5/5、基线 2/2）✅
- **验证矩阵**：`tsc -b` + `vite build --mode workbench` 绿；eslint/stylelint 绿；cargo fmt / clippy `-D warnings` 绿；主 crate lib **313/313**；`write_discipline_scan` 5/5；`baseline_stays_untouched_on_the_generate_path` 2/2（端到端走查没碰真基线）。

## 3. Task 15 开工前裁定与勘察（2026-09-24，锁定后实施）

三条裁定已获批准：**①上游缺失降级**（不再作为启动硬门；依赖上游的功能显式显示原因，不 mock）；**②显式初始化**（不放宽 `presets_root()` 判定；初始化动作创建最小骨架后才被识别；重复初始化绝不覆盖已有数据）；**③空 registry 预检 = 阻断项**（不空集通过、不崩溃）。

勘察结论（细节在 tasks.md 15.0）：上游的实质依赖面已收窄（字段定义 b04 Task 8 起在我们 presets/，剩 mkp_preset 连接键 / stock 整页 / 发布元数据 / boot 统计）；**改造核心是 `Ctx.up` 非可选 + `Ctx::open` 双 `?` 硬门**；最小骨架 = `brands.toml` / `layout_schema.toml` / `registry/param_registry.toml`（标志文件）/ `machines/` 空目录 / `assets.toml` / `bundles.toml` 六件（全空表，`forbidden_zones/` 不需要）；15.3/15.4 数据层 `add/write` 已就位，缺命令层与 UI。**前端接线归 Task 17 新工作台。**

**换设备实拍（同日，验收场景已入 tasks.md 17.5）**：新设备上上游未 clone → 旧前端顶部大面积红横幅「没有上游不启动业务」，但主内容区机型数据照常显示（机型页绕过 boot 自拉数据）——页面间状态割裂，正是裁定①的反例实证。已列为新前端与 Task 15 共用验收场景：区分「上游未配置」与「数据不可用」、上游路径界面内可重配、「可用状态」与「连接状态」分开呈现。

**Task 15 后端实施 —— 前半（同日完成，lib 319/319 + 首开场景 1/1 绿，未提交待验收）**：`Ctx.up: Option<Upstream>`（open 只把 presets 缺失当失败）+ `Book.up: Option<&Upstream>`（90 处调用点显式化）+ derive 四处 None 路径（含 build_rows 显示名收口为我们自己的清单）+ `wb_init_workbench`（六件骨架、幂等拒绝覆盖，判据×3）+ `wb_boot` 三态（initialized 字段）+ preflight 空 registry 阻断（`registry.empty`，判据×1）+ 降级提示与 stock/fallback/publish 的诚实空态 + **`tests/first_run.rs`：新设备首开全场景**（boot 引导 → init → boot 翻转降级 → 建机型版本 → 重复 init 拒绝）。

**Task 15 后端实施 —— 后半：15.3 / 15.4 命令层（同日完成，lib 326/326 + `first_content` 1/1，未提交待验收）**

| 子项 | 状态 | 落点 |
|------|------|------|
| 15.4 导入资产 | ✅ | `wb_import_asset` + 领域体 `app::assets::import_asset`（复制进资产根的类型子目录 + `Assets::add/write` 登记；先复制再登记、登记失败回滚；四条先查再写；**同名绝不覆盖**，`-2`/`-3` 避让）|
| 15.3 建套餐 | ✅ | `wb_add_bundle` + 领域体 `app::bundles::add_bundle`（**先查再写**：归属机型真 / 每条 `assetRef` 解析得到 / 至少一条 BBS —— 不合格一个字节都不写）|
| 15.3 被机型引用 | ✅ | `MachineField::DefaultBundle`（这一格此前没有写入口）；`wb_set_machine_field` / `wb_set_version_field` 对**引用格**先查再写 |

两条纪律值得单列：**① 先查再写** —— 套餐与机型的引用格写错的后果不是报错，是下一次加载把整个 `presets/` 判成 Corrupted（工作台起不来），所以判定必须在写之前；**② 目录约定不是新发明** —— `AssetKind::dir()` 的四个前缀就是真数据 21 条 `path` 的前缀，有判据锁着。

**只读验收（2026-09-24，未动功能代码）**：逐条核对了初始化、无上游降级、套餐与资产导入、引用校验、重启后重读，**没有发现与三条裁定不一致的地方**。两条写纪律另有专门验证：

- **回滚不会误删原有文件** —— 全仓唯一一处 `remove_file` 删的是本次挑的新落点（挑点时已确认不存在）、由 `atomic_write` 自己建出来的那个文件。新增判据 `a_failed_registration_rolls_back_only_the_file_it_wrote`（用 `name` 留空逼出 `Assets::add` 拒收，于是复制已发生、回滚被真正走到）：刚复制的落点被删掉 / 挨着它的同名老文件一个字节不动 / 定义里没多条目。**反空转实测**：临时撤掉 `remove_file` 后该判据当场红。
- **引用校验覆盖了所有写 `presets/` 的入口** —— 逐条过了一遍：`wb_add_version`/`wb_copy_version`/`wb_add_machine`/`wb_remove_version` 不写引用格；已加先查再写的是 `wb_set_machine_field`（DefaultBundle/Image/Icon）、`wb_set_version_field`（RecommendedBundle）、`wb_add_bundle`、`wb_import_asset`；`apply_values` 的 owner 校验早已就位。数据层那三个删除写入今天没有命令层，进不了界面。

**两条边界（非本轮引入，已登记在 tasks.md 15.0，Task 17 开工时要带上）**：①「套餐」今天有两套 —— `presets/bundles.toml`（b05 Task 10 的①层，交付可达集也算它）与 `workbench/delivery.json` 的 `BundleEdit`（旧模型，`derive::bundle_bbs` 还在读它，而它的写入支 `Patch::SetBundle` **全仓无 TSX 调用方**，是待删死支），两套 id 撞名（`A1_default`）；新 UI 必须走①层那条。②引用校验成立的前提是"当前没有删除命令" —— 将来加删除入口时，删资产走 `Presets::remove_asset`（守卫在），**删套餐还没有对应领域体**，新建时必须先反查机型/版本的引用。残余窗口一条（未修）：`target_path` 与 `atomic_write` 之间是 check-then-write，彻底消除要换 `create_new`，等裁决。

判据 **+8**（领域体 7 + 命令级 1）：命令级那条是**新文件 `src-tauri/tests/first_content.rs`**（独立进程、`MKPSE_REPO_DIR`→tempdir）：init → 建机型版本 → 导入图并关联 `image` → 导入 BBS → 建套餐 → 机型 `defaultBundle` 引用 → 重开 boot 全绿 → 五种写不成的形状被拦且已有数据不动。它把 15.3 与 15.4 串成一条链：**建套餐必须先有 BBS**（doc §12.4），分开写两条谁也证明不了这条依赖。第 8 条是验收补的回滚判据（见上）。

全量 **327/327 + first_content 1/1 + first_run 1/1**，写纪律 5/5、基线 2/2、fmt / clippy 绿。仍未提交。

## 4. 遗留（非阻塞，均已登记）

- **AGENTS.md**：独立后续任务（裁决：不插入本轮）。
- **`presetFile` 字段**：机型页仍可编辑；G-2 / Task 16.3 收口时同步删/锁（复制已刻意不抄）。
- **9.3a** BBS 文件名旧云端代号：等消费端（切片器导入）确认后裁决，不为交付层阻塞。
- **套餐写命令**：`wb_add_bundle` 已建（15.3）；旧前端 MenuPage 仍是只读 —— 按冻结纪律不接它，**写入口的 UI 归 Task 17 新工作台**。
- **`.snapshots/` 的 HANDOFF 勘误**已记：它不是「只写不读」，恢复配方在读它。

## 5. 仓库状态

- Task 14 全部改动（14b 前端 + 14c 判据 + 文档）在**本分支一笔提交**，**不推送**（指令）。
- 推送目标就是本分支，不合并 main。
- 后续入口：Task 15（空白初始化）/ Task 16（决策闸收口，见 tasks.md）。
