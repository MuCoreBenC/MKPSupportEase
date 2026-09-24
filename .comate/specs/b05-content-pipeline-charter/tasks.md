# 新工作台产品流程与数据契约 —— 落地任务计划

> 依据 `doc.md`。四道决策闸（G-0 命名规范 / G-1 配方组织形式 / G-2 `presetFile` 去留 / G-3 BBS 与 Orca）没开的任务不执行。
> 贯穿全程的硬防线：**九份产物内容逐字节不变**（sha256 与 doc §2.4 表一致）。改名只改名字，参数值一个都不许动。
> 旧仓参考在 `local-reference/mkpse-presets/`，**全程只读，不删**。

---

## 决策记录（已拍板）

| 闸 | 决定 | 落点 |
| --- | --- | --- |
| G-0 命名规范 | `{机型id}-{版本id小写}.toml`；身份保持原样，小写只在命名函数里发生一次 | Task 1 已写进 `docs/ARCHITECTURE.md` §10 |
| G-1 配方组织 | **方案甲：保持现状**，实现 §5.1 选定的失败处理 | Task 16.2 |
| G-2 `presetFile` | **删除**（机型文件字段、`presets/catalog.rs` 的 `VersionField`、前端类型三处同步） | Task 16.3 |
| G-3 切片器资源 | **纳入 BBS**，保留开放的切片器扩展维度；删除现有 Orca 文件 | Task 16.4；**删除范围以 Task 7 盘点结论为准，不提前扩大** |
| Task 2 实现方式 | **①：`mkpse-preset` 作为 `workbench` feature 下的可选依赖**，转调权威实现 | 本文件 Task 2.3 |

**提交节奏**：每个 Task 收口提交一次。其中 Task 5（重命名 + sha256 复验）与 Task 9（资产迁移）
各自独立提交，不与别的改动混在一起 —— 这两笔是「改了很多文件但内容该逐字节不变」，混进去就验不清。

---

- [x] Task 1: 冻结命名与身份规范（闸 G-0；只出文档，不动代码与数据）
    - 1.1: 按 doc §6.3 定稿五项：字段组成、大小写、分隔符、扩展名、ID→文件名的映射方向
    - 1.2: 在 `docs/` 写一节「版本身份与文件命名规范」，明确内部身份（机型 id + 版本 id）与外部文件名的职责分界
    - 1.3: 写明规范覆盖的五个使用点：版本定义、参数源关联、构建产物、消费端查找、发布文件名
    - 1.4: 写明不变式「身份稳定、文件名可算」，并点出现存的三处违反（`generate.rs:55-57`、`build.rs:219-221`、机型定义的 `presetFile` 字面量）
    - 1.5: 写明大小写风险：文件名小写化后，仅大小写不同的 id 会冲突成同一个文件

- [x] Task 2: 命名实现收敛成唯一一处（决定：① 可选依赖）
    - 2.1: ✅ 权威实现定在 `crates/preset/src/generate.rs` 的 `file_name`，小写化在它内部发生一次
    - 2.2: ✅ 从 crate 根导出为 `preset::preset_file_name`（`lib.rs`）
    - 2.3: ✅ 取 ①：`mkpse-preset = { path = "../crates/preset", optional = true }`，只挂在 `workbench` feature 下；`src-tauri` 的 `preset_file_name` 改为转调（薄壳，不再自己拼字符串）
    - 2.3a: ✅ 边界实测：`cargo tree -p mkp-support-ease`（默认 feature）里**没有** `mkpse-preset` / `mkpse-postprocess`，**发布构建的依赖图与产物都不含**那 56 KB 注册表与 9 份预设；代价只落在工作台构建（开发态）。引用点全在 `#[cfg(feature = "workbench")]` 门内，不带 feature 时 `use preset` 直接编不过
    - 2.4: ✅ 判据 `naming_follows_the_one_rule`：小写化、机型 id 原样、分隔符 `-`、且对配方侧是恒等变换
    - 2.5: ✅ 判据 `case_only_differences_collide`：仅大小写不同的 id 必然塌成同一文件名（校验层据此拦）
    - 2.6: ✅ 前端**没有第二份实现**（全仓核对：没有任何地方自己拼 `{机型}-{版本}.toml`）。前端唯一掌握的预设名是机型定义里的 `presetFile` 字段 —— 那是 G-2 的删除对象（Task 16.3），删掉后前端只展示后台算出的名字
    - 2.7: ✅ 判据 `naming_matches_the_preset_crate`：薄壳与权威实现逐例相等 —— 这是跨 crate 连着两处的那根线（转调本身没有类型系统兜底）
    - 2.8: ✅ `cargo test -p mkpse-preset --lib` 77/77 绿；`cargo test -p mk-support-ease --features workbench` 265/265 绿；`fmt` 与两档 `clippy -D warnings` 全过；`stored_presets_match_the_recipe` 与 `kg0p_products_match_the_baseline` 都过 —— 九份产物字节未变

- [ ] Task 3: 收拢重复路径常量与清理死引用（零行为变更）
    - 3.1: 把手写 7 次的 `fixtures/presets` 路径改为复用一处常量（`tests/recipe.rs:22`、`ranges.rs:185,228`、`lineage.rs:17`、`write_roundtrip.rs:17,62,118,160`、`key.rs:11`）
    - 3.2: 把 `builtin_presets_match_dir.rs:21`、`key.rs:90` 的 `assets/presets` 改为复用 `assets_dir()`
    - 3.3: 删除 `src-tauri/src/workbench/app/build.rs:748` 指向 `../mkp-ssr/...` 的引用（该仓库在本工作区不存在），不留兼容分支
    - 3.4: 跑全量测试，确认无行为变化

- [ ] Task 4: 旧名 → 新身份映射表（数据文档，不改文件）
    - 4.1: 把 doc §2.4 的九对对应关系落成显式映射：旧云端名 ↔ 机型 id ↔ 版本 id ↔ 新文件名
    - 4.2: 核对 `crates/preset/tests/key.rs:32-57` 的硬编码四元组与映射一致
    - 4.3: 记录 B 套命名语义（`M`=mini、`F`=快拆、`_260628`=开源版日期）作为迁移解释，不作为新规则
    - 4.4: 标注有效期：迁移完成后失效，不进入长期代码路径

- [ ] Task 5: 按新规范重命名基线、夹具与硬编码名单
    - 5.1: 重写 `crates/preset/src/generate.rs:402-417` 的 `assert_ne!`（它现在要求两边文件名必须不同）
    - 5.2: 配对从 `pair_by_head` 改为按文件名直配，删除 `pair_by_head` / `head_value`（`generate.rs:172-226`）
    - 5.3: 重命名 `crates/postprocess/tests/fixtures/presets/` 下九份文件
    - 5.4: 同步重命名派生夹具 `crates/postprocess/tests/fixtures/ir/build9/` 下九份 JSON
    - 5.5: 更新四处硬编码名单：`pipeline_wiping_source.rs:30-40`、`build9.rs:35`、`registry_branch_diff.rs:35`、`crates/preset/src/build.rs:1119`
    - 5.6: 更新 `crates/preset/src/lib.rs:85-122` 的 `BUILTIN_PRESETS` 手写表与 `include_str!` 路径
    - 5.7: 复验硬防线：九份产物 sha256 与 doc §2.4 表完全一致
    - 5.8: 更新 `crates/preset/assets/preset_recipes.toml:11-17` 的链路说明与 `docs/` 相关段落

- [ ] Task 6: 把基线维护从日常生产流程里摘出来
    - 6.1: 明确对照基线是判据资产，不是产物副本，也不是交付文件
    - 6.2: 基线更新只在「本次变更确实预期产物变化」时发生，走独立流程
    - 6.3: 从生成与发布路径中移除任何自动同步基线的动作
    - 6.4: 工作台里「同步基线」做成显式、带 diff 确认的独立入口，不出现在新增版本的必经步骤里
    - 6.5: 判据：普通生成与发布路径不写入基线目录

- [ ] Task 7: 旧仓迁移盘点收口（只盘点，不搬运）
    - 7.1: BBS 与套餐的盘点结论已在 doc §12.4 落定，此处只补未覆盖的部分
    - 7.2: 盘点旧仓 `assets/` 各子目录（`faq/` `machines/` `models/` `avatars/` `icons/machine/` `qr/`）：文件数、体量、归属机型、是否仍被引用
    - 7.3: 确认 `assets/qr/` 是否只有 `*.png.gitkeep` 占位符而无真图；只有占位符则整目录舍弃
    - 7.4: 盘点 `models/` 下 3mf / zip / 解压目录三种形态，决定交付哪一种（同一模型不重复交付）
    - 7.5: `faq/` 相关图片按 D-4 不纳入管辖 → 标记舍弃，不进新资产库
    - 7.6: 输出迁移清单，逐条标注：直接保留 / 需转换 / 舍弃
    - 7.7: 确认旧仓全程只读、不删

- [ ] Task 8: 资产域①层 —— 集中式资产定义与目录约定
    - 8.1: 定资产定义的数据形状：id、类型、归属机型、相对路径、显示名
    - 8.2: 类型集合按闸 G-3 确定；建议三类：BBS profile、图片 / 图标、模型
    - 8.3: **不给 MKP 预设建资产条目**（doc §12.5：旧仓那 9 份 `mkp_preset` 是冗余，路径由命名规则算出）
    - 8.4: 定资产目录约定，替代 `public/` 下的散落引用
    - 8.5: 实现 Rust 侧数据模型 + 保格式写回
    - 8.6: 机型定义的 `image` / `icon` 从裸文件名改为指向资产 id
    - 8.7: 前端 `src/workbench/api.ts` 补对应类型，与 Rust 侧一一对应
    - 8.8: 判据：资产 id 唯一（**大小写不敏感**）；相对路径必须落在资产根内（走 `resolve_in` 防穿越）

- [ ] Task 9: 资产迁移执行与引用反查
    - 9.1: 按 Task 7 清单把图片、图标、模型搬入新资产目录，文件内容不变
    - 9.2: 为每份搬入的文件生成资产定义条目
    - 9.3: 迁移 BBS 的 9 份 JSON 与其 9 条资产元数据（按新结构重写，不照抄一文件一条）
    - 9.4: 实现「谁在用它」反查：给定资产 id，列出引用它的机型 / 版本 / 套餐
    - 9.5: 删除资产时先反查，有引用则阻止并列出引用方
    - 9.6: 清理 `public/` 下已被资产库接管的硬编码引用
    - 9.7: 判据：所有机型定义引用的资产 id 都能解析到真实存在的文件

- [ ] Task 10: 套餐域①层 —— 集中式套餐定义与悬空引用消除
    - 10.1: 定套餐定义的数据形状：id、显示名、归属机型、`assetRefs`、更新时间
    - 10.2: 采用一份集中定义，写明「超过 20 个再拆」的门槛
    - 10.3: 迁入旧仓 5 份套餐（A1 / A1_MINI / P1S / P2S / X1C 各一个 `*_default`）
    - 10.4: 实现读写与工作台「套餐管理」的后端命令
    - 10.5: `defaultBundle` / `recommendedBundle` 从刻意悬空转为可解析引用
    - 10.6: 更新 `src-tauri/src/workbench/presets/mod.rs:155-157` 的注释（它现在说 `bundles/` 还没搬）
    - 10.7: 判据：机型与版本引用的每个 bundle id 都存在；套餐的每个 `assetRef` 都能解析
    - 10.8: 判据：**MKP 预设与其配套 BBS 预设必须同批交付**（doc §12.4 的语义耦合），缺一方报 error

- [ ] Task 11: 校验层补齐（阶段②「检查内容」）
    - 11.1: 引用完整性：参数源、`image`、`icon`、bundle、`assetRef` 逐项检查存在性
    - 11.2: 参数合法性：键在注册表内、值在区间内（复用现有 registry 能力）
    - 11.3: 唯一性检查必须**大小写不敏感**：机型 id、版本 id（机型内）、资产 id、套餐 id 四类都要查
    - 11.4: 孤儿检测：没有任何版本定义指向的参数源
    - 11.5: 跨文件一致性检测：「机型存在但配方缺它」与反过来，两个方向都要能看见（doc §5.1 的兜底机制）
    - 11.6: 分级：参数源缺失报 warning（允许版本先存在），构建时才升为 error
    - 11.7: 错误信息带上对象 id、字段名、找过的路径
    - 11.8: 扩展 `wb_preflight` 的 `Report`，前端按严重度分组展示

- [ ] Task 12: 交付层 —— `dist-presets/` 结构与目录类 JSON
    - 12.1: 定稿交付目录结构（doc §7 是示意，此处定稿）
    - 12.2: 生成机型目录 JSON：机型、版本及其关系
    - 12.3: 生成套餐 JSON
    - 12.4: 生成资产索引 JSON：清单、位置、归属
    - 12.5: 目录 JSON 的文件名字段全部由命名函数算出，不存在手写字面量
    - 12.6: 判据：目录 JSON 中引用的每个文件都在交付目录中真实存在

- [ ] Task 13: 可达性筛选、残留拦截与 manifest
    - 13.1: 可达性分析：从机型 / 版本 / 套餐的引用出发，算出应交付的文件集合
    - 13.2: 资产库中不可达的文件不进交付
    - 13.3: 校验未通过的内容不进交付
    - 13.4: **残留拦截**：发布前扫描交付目录，不在本次可达集合内的文件一律列出，**有残留就中止发布，不写 manifest**（doc §9.1）
    - 13.5: 提供显式「清理残留」动作，走 `.trash/` 回收而不是直接删
    - 13.6: manifest 作为发布最后一步生成，只描述已落地的文件（路径、大小、sha256、时间）
    - 13.7: 发布全程走 `fsx::atomic::atomic_write`
    - 13.8: 判据：manifest 条目数等于交付目录实际文件数；每条 sha256 与文件真实哈希一致

- [ ] Task 14: 工作台接通四阶段骨架
    - 14.1: 六个页面按 doc §4.2 映射到后台数据
    - 14.2: 阶段闸门落实为交互约束：检查未过不给生成，生成未过不给发布
    - 14.3: 「复制已有版本」走通 doc §4.3 第 2–5 步，保存时**只写版本定义**
    - 14.4: 版本列表显示「参数源待补」状态，不因缺文件而隐藏该版本
    - 14.5: 参数编辑器支持从模板复制出参数正文后改值写回
    - 14.6: 资产选择走资产库模态框，不让人手填路径
    - 14.7: `workbench/` 五个子目录真正投入使用（`machines/ bbs/ .draft/ .trash/ .snapshots/`）
    - 14.8: 端到端跑通 doc §4.3 全部 11 步作为验收场景

- [ ] Task 15: 空白初始化流程
    - 15.1: 从干净的 `presets/` + `workbench/` 出发，能建起第一台机型
    - 15.2: 能为这台机型建第一个版本（不要求参数源已存在）
    - 15.3: 能建第一个套餐并被机型引用
    - 15.4: 能导入第一张图片并关联到机型
    - 15.5: 全程不手动碰任何文件
    - 15.6: 判据：空白初始化后立即跑校验，结果是「可理解的待办清单」而非崩溃或空数据假象

- [ ] Task 16: 决策闸收口与文档同步
    - 16.1: G-0 定稿后：把命名规范写进 `docs/`，并让 Task 2 的判据引用它
    - 16.2: G-1 决议后：选方案甲则实现 §5.1 选定的失败处理；选方案乙则拆分配方并改 `recipe_path` / `Recipe` 解析 / `include_str!` 策略
    - 16.3: G-2 决议后：同步 `presets/catalog.rs` 的 `VersionField`、前端类型、机型文件字段集合
    - 16.4: G-3 决议后：固化资产类型集合与交付目录是否含 BBS；Orca 按建议舍弃则记录理由，不留空目录
    - 16.5: 更新 `docs/ARCHITECTURE.md` 与 `docs/PRESET-PRODUCT-RULES.md` 中与本轮变更相关的段落
    - 16.6: 全量复验：`cargo test`、`cargo clippy -- -D warnings`、`gen-presets --check`、`gen-presets --baseline`
    - 16.7: 确认旧仓数据全程未删；迁移映射表归档后标记失效
