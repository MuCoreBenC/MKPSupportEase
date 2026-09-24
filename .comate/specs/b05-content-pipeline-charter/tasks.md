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
    - 3.1: ✅ 九处手写 `fixtures/presets` 统一走 `preset::generate::fixtures_dir()`（`tests/recipe.rs` 的本地 `fixtures_dir()` 一并删掉；顺手覆盖了任务清单外的 src 内两处：`src/validate.rs`、`src/build.rs` 的单测 —— 同一类重复，同一笔改）
    - 3.2: ✅ `builtin_presets_match_dir.rs` 的本地 `builtin_dir()` 与 `key.rs` 的内置预设路径改为 `preset::generate::assets_dir()`
    - 3.3: ✅ **裁决取②：不删，改指仓内基线** —— 原文前提已被实测证伪（`../mkp-ssr/...` 在开发机上存在、判据真比了 9 份；两侧基线 sha256 9/9 相同，只差文件名）。改为：基线走 `preset::generate::fixtures_dir()`；配对读文件头 `# machine:` / `# variant:`（不认文件名，Task 5 改名后不用再动）；9 份一份不少地比到 —— 少了 / 重了 / 身份读不出来都直接失败，删掉「没找到就 return」那条静默分支
    - 3.3a: ✅ 随之把 `render()` 的机型查询从上游那份换成**我们自己那份清单**（`book.machines()`）：没有这一改，这条判据在 CI 里仍跑不起来（上游不在仓库里）。没有任何判据覆盖「上游没有这台」那条旧分支，而上游那层正在退出（b04 Task 12）—— 理由与代价写在 `render()` 的注释里
    - 3.3b: ✅ 复验：判据实测 **9/9 配上并逐份比过**（值全对，只剩「段内键序」那条已知差异，仍是 Task 11 的收尾项）；`cargo test -p mkp-support-ease --features workbench` 265/265
    - 3.3c: ✅ **那处生产代码改动的独立审查**（结论单独成篇：`render-machine-source-review.md`，与 Task 5 分开汇报）：① `book.machines()` 是当前正式机型来源（b04 Task 8 起，清单来自 `presets/machines/*.toml`）；② 渲染链里只剩这一处读上游的机型身份，上游那层正在退出（b04 Task 12）；③ **需要独立判据** —— 已补 `render_takes_the_machine_from_our_own_catalog`，并做了反空转验证（改回上游那份 → 它与 M0 同时红，改回来 266/266 绿）。残余风险一条：真 `presets/` 与真上游之间没有任何判据比机型集合 → 并入 Task 11
    - 3.4: ✅ `cargo test -p mkpse-preset` 全绿（77 lib + 39 集成）、`cargo fmt --all --check` 与 `cargo clippy -p mkpse-preset --all-targets -- -D warnings` 全过

- [x] Task 4: 旧名 → 新身份映射表（数据文档，不改文件）
    - 4.1: ✅ 落在 `migration-map.md`：九行「旧云端名 ↔ 机型 id ↔ 版本 id ↔ 新文件名」，附字节数与 sha256 前 16 位（**两侧相同**，实测逐对相等）
    - 4.2: ✅ 与 `crates/preset/tests/key.rs:32-57` 的硬编码四元组逐条对照：顺序与内容完全一致；那条判据除了读表还反空转（`len == 9`）并逐份断身份与 `key.file_name()`
    - 4.3: ✅ 记了 B 套的读法（`M`=mini 档位、`F`=快拆、`_260628`=开源版日期），并写明两条要点：**B 套不可算**（要查表或读文件头）、日期后缀不是版本语义
    - 4.4: ✅ 文档头尾都标了有效期：迁移完成后归档失效、不进长期代码路径；附「B 套今天还留在哪」的五类收尾清单

- [x] Task 5: 按新规范重命名基线、夹具与硬编码名单
    - 5.1: ✅ 那条 `assert_ne!` 连同它所在的判据一起重写成 `pairing_goes_by_file_name` —— 现在咬「两边文件名集合相同」（少一份、多一份、名字写岔了都红）
    - 5.2: ✅ `pair_by_head` / `head_value` 整个删掉，换成 `toml_files()`（按文件名索引）；`check_baseline` / `sync_baseline` 都改成按名字直配。顺带把 `tests/recipe.rs` 的 `fixtures_by_combo()`（**另一处**按头配对）改成按名字取 fixture —— 现在全仓没有按 `# machine:` 配对的代码了（工作台那条 M0 判据除外，它的理由写在文件里）
    - 5.3: ✅ 九份基线改名，git 记为相似度 100% 的改名：18 份夹具合计 **0 增 0 删**
    - 5.4: ✅ 九份 IR 夹具同步改名（内容里本来就没有文件名：`PresetName` 是空串）
    - 5.5: ✅ 四处硬编码名单全换新名，并写明「名字是命名规则算出来的、写死字面量是刻意的」（防从目录遍历推出来）；`registry_branch_diff.snapshot` 按文档命令重生成，**diff 只有段落名，`0 处` 一条没变**
    - 5.6: ✅ **无需改动**：`BUILTIN_PRESETS` 与它的 `include_str!` 路径本来就是新名（产物侧一直在 A 套），本轮没动它一个字节
    - 5.7: ✅ 硬防线实测（改名前后逐份比）：九份基线 sha256 **与迁移前完全相同**（`0B19FEAA…` / `115E061F…` / … 见 `migration-map.md`）；`gen-presets --check` 与 `--baseline` 都绿，后者报「9 份同名文件逐字节相同」
    - 5.8: ✅ `preset_recipes.toml` / `test_recipes.toml` 的链路说明改成真实路径 +「两边同名、按名字直配」；`docs/ARCHITECTURE.md` §10.5 改写成「已收口」并指向映射表
    - 5.9: ✅ **越出清单但同类的一处**：`PresetKey::file_name` 有变体那一档改转调 `generate::file_name`（它以前自己拼一遍，是全仓第三处 `format!` 命名 —— Task 2 那句「唯一实现」此前并不成立）。无变体那一档保留为**显式例外**：老预设没有 `# variant:`，规则（身份 → 名字）在这一档上不成立

- [ ] Task 6: 把基线维护从日常生产流程里摘出来（除 6.4 外已收口）
    - 6.1: ✅ 写进 `docs/ARCHITECTURE.md` §10.6：对照基线是**判据资产**，不是产物副本、不是交付文件；与产物同名同内容但职责不同，**不合并它们**（合了就是自比自）
    - 6.2: ✅ 同一节 + `gen-presets` 文件头写清流程：只在「本次变更确实预期产物变化」时更新；先看差异（`--baseline` 或 `git diff`）→ 确认是要的 → 手动 `--sync-baseline`
    - 6.3: ✅ **核对结论：生成与发布路径本来就没有自动同步** —— `--write` 只写 `assets/presets/`；`wb_generate` 写 `dist-presets/presets/mkp/` + 快照；`wb_publish` 写 `dist-presets/` + manifest；`sync_baseline` 在 `src-tauri` 零命中、唯一调用点是那个 CLI；`scripts/` 与 CI 里也没有任何引用。本轮没有要删的调用 —— 改的是**把这个事实锁住**
    - 6.4: ⛔ **工作台里没有这个入口，本轮也没建** —— 今天能做这件事的只有 CLI。要建的是「显式 + 带 diff 确认 + 不在新增版本必经步骤里」的 UI 动作（Rust 命令 + 前端视图 + 类型），属功能开发而非"摘出来"；约束已写进 §10.6 与 doc §4.3 第 10 步。**等裁决：并入 Task 14，还是现在单开**
    - 6.5: ✅ 两条判据，各堵一半：
        - `write_discipline_scan.rs::the_baseline_has_exactly_one_write_path` —— 源码扫描（`crates/*/src` + `src-tauri/src`），`sync_baseline` 只允许出现在定义处与 `bin/gen_presets.rs`；断言是**相等**不是子集（入口搬走会红）
        - `tests/baseline_stays_untouched_on_the_generate_path.rs` —— 真跑 `check_all` / `check_baseline` / `write_all`(临时目录)，基线九份**内容 sha256** 不变；附 `the_snapshot_notices_a_change` 证明检测器不空转
    - 6.5a: ✅ 两条都做了反空转探针（都实测会红）：放一个不被编译的 `preset/src/__probe_scan.rs` → 扫描判据红并点出文件名；临时给 `write_all` 加一行写基线 → 运行时判据红。探针已撤，基线九份 sha256 回到迁移前的值
    - 6.5b: 边界照实记：判据锁的是 **Rust 源码**（crates/*/src + src-tauri/src）；`scripts/`、CI、`*.py` 今天对基线零引用（人工核对），但**没有判据锁着**

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
    - 11.9: 上游存在时，检查「我们清单里有、上游不认」的机型与版本，报 warning（来源：Task 3.3c 的残余风险 —— `render()` 改用我们自己的清单之后，这类不一致不再以「机型不存在」暴露）

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
