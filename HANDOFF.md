# 交接计划：Task 14 工作台接通（MKPSupportEase）

> 起草时间：2026-09-24
> 适用分支：`feat/b04-p3-migration`（注：仓库里任务文档称 b05，分支名是 b04-p3，同一件事）
> 起草人上下文：14a 后端链路已闭环并提交 `f4ced5c`，工作区干净，尚未推送。

## 1. 当前进度（已闭环的部分 = 14a）

| 子项 | 状态 | 落点 |
|------|------|------|
| 14.3 复制已有版本 | ✅ | `presets/catalog.rs::copy_version` + `app/machines.rs::wb_copy_version` + 公共 `validate_new_version_id` |
| 14.4 参数源待补 | ✅ | `presets/registry.rs::version_has_variants`（只看 `machineVariants`）+ `app/mod.rs::VersionView.hasRecipe` |
| 14.5 参数正文复制 | ✅ | `app/mod.rs::wb_copy_recipe`（领域体 `copy_recipe` 可测）|
| 14.9 基线 diff + 同步 | ✅ | `app/build.rs::wb_baseline_diff` / `wb_sync_baseline` |
| 14.1 六页面映射 | ⏳ 勘察完成，menu/build 待前端 | — |
| 14.2 闸门交互约束 | ⏳ 后端硬闸已就位，仅剩前端按钮 | — |
| 14.6 资产选择模态框 | ⏳ 接 `wb_assets` | — |
| 14.7 workbench 子目录职责 | ⏳ 待定 | — |
| 14.8 端到端 11 步验收 | ⏳ 待 14c | — |

`Task 14` 整体复选框按纪律**未勾**（子项 Completion 记在 `.comate/specs/b05-content-pipeline-charter/tasks.md`，整体收口时再勾）。

## 2. 上次收尾停在哪（无半截改动）

14a 是完整闭环，没有改动留在某个文件里。按时间顺序最后动过的几个点：

1. **代码收尾**：`src-tauri/src/workbench/app/mod.rs`（`wb_copy_recipe` + `VersionView.hasRecipe` + 14.5 判据）+ `src/app/api.ts`（`BaselineDiffEntry`、`VersionView.hasRecipe` 类型）。
2. **写纪律判据收尾**：`src-tauri/src/workbench/presets/.../write_discipline_scan.rs`，把 `build.rs`（14.9 授权入口）与 `lib.rs`（命令注册清单）登记进 `the_baseline_has_exactly_one_write_path` 的 ALLOWED 白名单。
3. **文档收尾**：`.comate/specs/b05-content-pipeline-charter/tasks.md` 的 14a 子项注记。

## 3. 还剩什么（按优先级）

### 14b 前端接通（最大块，未启动）
- **14.1** 六页面收口：机型 / 配方 / 对比三视角已接后台；**menu / build 两个视角还没接通**（doc §4.2 映射）。
- **14.2** 闸门按钮约束：后端硬闸（`wb_generate` / `wb_publish` 开头的 `inspect` + `first_block`）已就位，14b 只做**前端按钮禁用 / 约束**（检查未过不给生成、生成未过不给发布）。
- **14.6** 资产选择走资产库模态框，复用已就位的 `wb_assets`（零消费），不让人手填路径。
- **14.7** `workbench/` 五个子目录真正投入使用：`bbs/` 零读写、`.snapshots/` 只写不读，定职责（`machines/.draft/.trash` 在用）。

### 14c 端到端验收
- **14.8** 跑通 doc §4.3 全部 11 步作为验收场景。

### 独立遗留（非阻塞）
- **AGENTS.md**：独立后续任务，已登记未动。
- **`presetFile` 字段**：仍可编辑，G-2 / 16.3 收口时同步删/锁（14.3 复制已刻意不抄它）。
- **9.3a** BBS 文件名里的旧云端代号，待 Task 12 命名时一起谈。

## 4. 续做入口（从哪接手）

- 前端六页面：`src/app/`（menu / build 视角，对照 doc §4.2 映射表）。
- 资产模态框：接 `wb_assets`（`src/app/api.ts` 已有 `assets()` + `assetUrl()`）。
- 闸门 UI：消费 `wb_generate` / `wb_publish` 的 `inspect` 结果做按钮约束。
- 端到端验收：在 `src-tauri/.../tests/` 或工作台测试里，对着 doc §4.3 的 11 步走查。

## 5. 已知纪律 / 不要碰

- **基线写入口唯一**：`wb_sync_baseline` → `preset::generate::sync_baseline`，落点闸在其内部，src-tauri 不经手路径。写纪律判据 `the_baseline_has_exactly_one_write_path` 是**活白名单**（相等断言），搬入口会红。
- **版本复制只写版本定义（单文件）**，天然避开 16.2 事务空白；抄 `recommendedBundle`，**不抄 `presetFile`**（G-2 悬空名拒绝扩散）。
- **14.5 是「独立快照」（裁决 A）**：复制 = 取模板完整有效配方（`layer.keys()` = 全部可见键，defaults⊕基底⊕覆盖归并）经 `apply_values` 批量钉成新版本显式覆盖。**缺失参数不伪造**、**无效配方进不了 layers**。改模板版本层 / 改机型基底后，快照版因自身显式值挡住传播——这是与"只复制自有覆盖"的实质差异证据。
- 14.3 刚落盘的版本不在会话缓存里，`wb_copy_recipe` 走**盘上现读**是唯一判定依据。

## 6. 已落地验收判据（别删）

- 14.5：`copied_recipe_is_an_independent_snapshot`（①两边逐键一致；②改模板版本层与改基底后快照均不变；③hasRecipe 翻转；④键数 = 模板有效配方键数）。
- 14.9：真数据基线 9 条全 `same`；`the_baseline_has_exactly_one_write_path` 白名单；`tests/baseline_stays_untouched_on_the_generate_path.rs`（含反空转 `the_snapshot_notices_a_change`）。
- 写纪律：`write_discipline_scan.rs`。

## 7. 仓库状态速记

- 分支 `feat/b04-p3-migration` 本地领先 `origin` 17 个提交（含 14a 的 `f4ced5c`），本 HANDOFF 提交后一并推送。
- `.comate/`（spec / 记忆文档）已正常入库，未被忽略；`workbench/` 被 gitignore 属预期。
- 推送目标就是本分支，**不合并 main**。
