# 交接记录：Task 14 工作台接通（MKPSupportEase）

> 起草 2026-09-24（14a 后）；同日 14b、14c 相继收口。**Task 14 全部完成**。
> 适用分支：`feat/b04-p3-migration`（注：仓库里任务文档称 b05，分支名是 b04-p3，同一件事）。

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

## 3. 遗留（非阻塞，均已登记）

- **AGENTS.md**：独立后续任务（裁决：不插入本轮）。
- **`presetFile` 字段**：机型页仍可编辑；G-2 / Task 16.3 收口时同步删/锁（复制已刻意不抄）。
- **9.3a** BBS 文件名旧云端代号：等消费端（切片器导入）确认后裁决，不为交付层阻塞。
- **套餐写命令**：MenuPage 只读（10.4 纪律：写入口等界面；界面已到，写还没建）。
- **`.snapshots/` 的 HANDOFF 勘误**已记：它不是「只写不读」，恢复配方在读它。

## 4. 仓库状态

- Task 14 全部改动（14b 前端 + 14c 判据 + 文档）在**本分支一笔提交**，**不推送**（指令）。
- 推送目标就是本分支，不合并 main。
- 后续入口：Task 15（空白初始化）/ Task 16（决策闸收口，见 tasks.md）。
