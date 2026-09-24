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
    - 6.4: ➡️ **裁决（2026-09-24）：并入 Task 14**（见 14.9），现在不单开。任务到此收口 —— 规则（§10.6）、落点闸、两条判据都在位；工作台那个入口是**功能开发**（要看 diff、要确认），提前做会把功能开发与资产域任务搅在一起
    - 6.5: ✅ 两条判据，各堵一半：
        - `write_discipline_scan.rs::the_baseline_has_exactly_one_write_path` —— 源码扫描（`crates/*/src` + `src-tauri/src`），`sync_baseline` 只允许出现在定义处与 `bin/gen_presets.rs`；断言是**相等**不是子集（入口搬走会红）
        - `tests/baseline_stays_untouched_on_the_generate_path.rs` —— 真跑 `check_all` / `check_baseline` / `write_all`(临时目录)，基线九份**内容 sha256** 不变；附 `the_snapshot_notices_a_change` 证明检测器不空转
    - 6.5a: ✅ 两条都做了反空转探针（都实测会红）：放一个不被编译的 `preset/src/__probe_scan.rs` → 扫描判据红并点出文件名；临时给 `write_all` 加一行写基线 → 运行时判据红。探针已撤，基线九份 sha256 回到迁移前的值
    - 6.5b: 边界照实记：判据锁的是 **Rust 源码**（crates/*/src + src-tauri/src）；`scripts/`、CI、`*.py` 今天对基线零引用（人工核对），但**没有判据锁着**

- [x] Task 7: 旧仓迁移盘点收口（只盘点，不搬运）
    - 7.0: ✅ 产物是 `asset-inventory.md`（口径与证据、`assets/` 六个子目录、`models/` 三种形态、迁移清单 15 条、只读证据与「我们这边还引用它吗」）
    - 7.1: ✅ doc §12.4 已定的（BBS 9 份 13.8 KB、套餐 5 份 0.7 KB、`source/assets` 18 份 3.2 KB）只做引用与体量复核，不重复盘点
    - 7.2: ✅ 六个子目录逐个体量/归属/引用：`faq` 29 份 5.37 MB、`machines` 11 份 247.5 KB、`models` 8 份 30.9 KB、`avatars` 3 份 14.1 KB、`icons/machine` 3(+1) 份 1.1 KB、`qr` 8 份 0 B。归属取自旧仓自己的 `manifest.json` 与 `content/asset_usage.json`
    - 7.3: ✅ **成立**：8 份全是 `*.png.gitkeep`、0 字节、没有一张真图 → 整目录舍弃
    - 7.4: ✅ **定案：只交付 `.3mf`**。实测两组配对是**逐字节相同**的同一份（`MKP_support_test_models.3mf` = `.zip`、`ZOffset_Calibration.3mf` = `.zip`），解压目录的 17 个条目与那个 `.3mf` 里的 17 个条目一一对应 → 三选一无信息损失。另记一条事实：这三份 `.3mf` **不在 manifest 的 72 条里**，是 `model_copy.models[].modelFile` 按裸名引用的
    - 7.5: ✅ `faq/` 29 份 5.37 MB 标**舍弃**（D-4）；同一口径下 `avatars/`（about）与 `assets/models/model_*.webp`（model_copy）一并舍弃
    - 7.6: ✅ 迁移清单 15 条：直接保留 3 条（机器图 5 张 / 图标 3 份 / 模型 3 份 `3mf`）、需转换 3 条（BBS / 套餐 / 资产元数据）、舍弃 9 条、不迁 1 条。保留+转换 ≈ 4.08 MB，舍弃 ≈ 10.89 MB
    - 7.7: ✅ 只读不删有据：拷贝没有 `.git`，改用**它自己的账** —— `manifest.json` 72 条 sha256 **逐份核过，0 缺 0 不符**（盘点前后各核一次）；全仓 mtime 一致（`2026-09-14 19:26`）；本次只跑读操作
    - 7.8: ✅ 两条空档照实记在 `asset-inventory.md` §5，**已裁决（2026-09-24）**：① `assets/models/` 与 `assets/faq/` **不要** → 预览图与 FAQ 一并舍弃；模型本体的**名字与预览图暂缓**（Task 8 只保留 `model` 类型，本轮不虚构元数据）；② 机型图**用现有新版那批大图**（`public/printers/bambu/`）—— 附注：那批只覆盖 A1 / A1_MINI（2 张）/ P1S，**P2S 与 X1C 没有新版**（见 9.1a）

- [ ] Task 8: 资产域①层 —— 集中式资产定义与目录约定（除 8.6 外已收口；8.6 按裁决挪到 Task 9）
    - 8.1: ✅ 数据形状：`id` / `type` / `machineId?` / `name` / `path`（+ `slicer`·`profile` 只给切片器预设）。**不写** `fileName`（路径只有一处）、**不写** `sha256`/`size`（交付时按真实字节算）。详见 `asset-domain-design.md` §1
    - 8.2: ✅ 类型集合：`image` / `icon` / `model` / `slicerProfile`。裁决（2026-09-24）：**`model` 保留为类型**、名称与预览图**暂缓**（本轮不虚构元数据）
    - 8.3: ✅ **不给 MKP 预设建条目** —— 而且是**结构化**的保证：enum 里没有 `mkpPreset` 这一档（想登记得先改 enum）
    - 8.4: ✅ 目录约定：资产根 `public/assets/`（vite 静态目录下的唯一子根，URL 前缀 `/assets/`）；`path` **相对资产根**，不相对定义文件；根按需建（`resolve_in` 第三道要求根存在，空目录靠 `.gitkeep` 进库）
    - 8.5: ✅ `presets/assets.rs`：模型 + 加载期判据 + `add`/`write`（原子写，走仓库唯一写盘出口）+ 零编辑往返逐字节相同；`Presets.assets` 接入并参与 `check_cross_consistency`；`wb_assets`（**只读**）与前端类型一起落地
    - 8.6: ➡️ **挪到 Task 9 一起做**（裁决）：改机型引用前必须已有 id 与文件，否则造出一批"指向不存在资产的引用" —— 而那正是 9.7 要拦的东西。Task 9 一次做完：**搬文件 → 写条目 → 改引用**
    - 8.7: ✅ `api.ts`：`AssetKind` / `AssetView` / `AssetList` + `wb.assets()` + `assetUrl()`（路径里有空格，编码只做一次）
    - 8.8: ✅ 两条加载期判据 + 反空转输入：id 唯一（大小写不敏感，`a1-image` vs `A1-Image` 实测红）、path 落在资产根内（`../` / 绝对路径 / `a/../../b` 三条实测红）。**另加三条**：键名写错要响亮（`deny_unknown_fields`）、切片器字段搭配、归属机型必须是真机型（跨文件，进 `check_cross_consistency`，实测一条假归属会让所有加载类判据一起红）
    - 8.9: ✅ 顺手抓到的一条真事，补成判据：**①层 `.toml` 必须是 LF** —— 工具新建文件会写成 CRLF，而 `toml_edit` 写回统一成 LF ⇒ 第一次保存整份被改写、diff 一片红。本轮的 `presets/assets.toml` 正是被这条判据抓出来的（`.gitattributes` 管得住入库那一份，管不住工作区那一份）
    - 8.10: 边界照实记：`wb_assets` 现在**只有读**（写入口在数据层，接上它要有界面 —— Task 14.6）；`present` 字段现在普遍 `false`（条目与文件一起在 Task 9 落地），这是"还没搬"不是错，Task 11.1 会把它升成校验层的一条

- [x] Task 9: 资产迁移执行与引用反查
    - 9.1: ✅ **21 份文件搬进资产根**（`public/assets/`）：机型图 6（4 张新版**移动**自 `public/printers/bambu/`、P2S/X1C 两张**复制**自旧仓）、图标 3、模型 3（`.3mf`）、BBS 9。复制件逐份 sha256 与源**相同**、移动件 git 记 **100% 相似度**
    - 9.1a: ✅ 裁决：**P2S / X1C 暂用旧仓那两张**（条目的名字里标了"待换新"），将来补新图再换；A2L 本来就没有图
    - 9.2: ✅ `presets/assets.toml` 落了 **21 条**：`id` / `type` / `machineId?` / `name` / `path`（+ `slicer`·`profile`）；路径沿用源文件名与目录结构
    - 9.3: ✅ BBS 9 份按新结构重写成条目（`slicer = 'bbs'` + `profile = 'process'`），**不照抄一文件一条**；文件本体沿用旧仓的目录与文件名
    - 9.3a: ⏳ **待裁决**：BBS 文件名里还带着旧云端代号（`MKPProcess X1 …`，不是 `X1C`）—— 它与 G-0 那套产物命名不是一回事，而 asset id 已经是稳定键。改名与否等 Task 12 定交付命名时一起谈
    - 9.4: ✅ `Presets::asset_usage` + `wb_asset_usage`：机型那一档说得出是谁（实测 `p1s-icon` → P1S、P2S、X1C）；**没有 `versions` 字段** —— 版本今天不直接引用资产（版本 → 套餐 → 资产是间接的），留一个恒空字段比说清"还没有"更糟；套餐那一档等 Task 10
    - 9.5: ✅ `Presets::remove_asset`：**先反查**，有人引用就拒绝并列出引用方（实测删 `a1-image` 被拦且报出 A1；换个大小写照样拦；没人引用的那条删得掉，且落盘后重读确实少了它）
    - 9.6: ✅ **部分**（照实记）：`src/app/heroArt.ts` 的路径改成 `/assets/printers/…`，并顺带点亮 `p2s` / `x1c`（图现在有了）；`public/printers/` 已空。**旧 app 那一侧仍是硬编码表** —— 它没接资产命令（资产选择是工作台的事，Task 14.6）
    - 9.7: ✅ 判据两条：`every_machine_asset_ref_points_at_a_real_file`（真数据 11 条引用逐条解析到真实文件，反空转 ≥10）与 `the_real_asset_list_is_complete_and_present`（21 条、每条 `present`、URL 前缀只有后端一处、每条有名字）
    - 8.6: ✅（从 Task 8 挪来）机型定义的 `image` / `icon` 改成**资产 id**；A2L 的 `image = ''` **删掉那一行** —— 空串读成"填过但填了个空"，不写才是"没有"
    - 9.8: ✅ 加载期补了**反向**检查：机型引用的 id **认不出来是 error**（打错字与 `machineVariants` 键写错同一类）；id 认得出、文件还没搬是 **warning 级**（现在由 `wb_assets` 的 `present` 说出来，Task 11.1 接手升成一条 warning）
    - 9.9: ✅ 旧仓全程只读：迁移后再核 `manifest.json` 的 72 条 sha256，**0 缺 0 不符**

- [x] Task 10: 套餐域①层 —— 集中式套餐定义与悬空引用消除
    - 10.1: ✅ 数据形状就是旧数据那五个：`id` / `display` / `machineId` / `assetRefs` / `updatedAt`，**照实搬不造默认值**（`updatedAt` 沿用旧值 `2026-07-12` —— 迁移不改内容，写今天就是把"搬了个文件"记成"改了套餐"）；`assetRefs` 换成我们的资产 id
    - 10.2: ✅ `presets/bundles.toml` 一份集中定义（旧仓 5 份离门槛还远；门槛同资产域：超过 20 个再拆）
    - 10.3: ✅ 5 份迁入，**逐份核对**旧仓 `source/bundles/*_default.toml`：id / display / machineId / updatedAt 逐字段一致；`assetRefs` 经旧仓资产条目反解到同一份文件（如 `a1_bbs_mkpprocess_a1_04_020` → `MKPProcess A1 0.4 0.20.json` → `a1-bbs-04-020`），5 条映射零错位
    - 10.4: ✅ 数据层读写全就位（`Bundles::load_from` / `get` / `add` / `write` / `drop_asset_refs`）；命令层 `wb_bundles`（**只读**，每个 ref 带 `resolvable` / `isBbs` 解析状态）+ `api.ts` 类型与封装 —— 写命令等界面（Task 14），与 `wb_assets` 同一条纪律。`drop_asset_refs` 自带空套餐守卫：去掉某套餐唯一的引用整次拦截（与加载期拦空 `assetRefs` 是同一条判据的两端）
    - 10.5: ✅ `defaultBundle` / `recommendedBundle` 转为**可解析引用**：加载期 `Presets::check_bundle_refs` 查「机型与版本引用的套餐存在 + 套餐归属的机型存在」，认不出是 error（doc §9：套餐定义落地后悬空转 error）；空串跳过（A2L 四折空，与 `image` 同口径）
    - 10.6: ✅ `presets/mod.rs` 模块头的搬迁状态表更新：套餐已搬，剩 `preset_registry.toml`（Task 12 的交付索引，不在这里存第二份）
    - 10.7: ✅ 判据真数据实测：`the_real_bundle_references_resolve`（**14 处非空引用**逐条可解析、指向 5 条套餐、归属机型一致；反空转 ≥14）与 `the_real_bundle_asset_refs_resolve`（5 条套餐的每个 `assetRef` 解析到真实资产且逐条 BBS、BBS 归属与套餐归属一致、`a1-bbs-04-020` 反查 → `A1_default` 且大小写不敏感）；另有 `wb_bundles` 的 5 份全量核对
    - 10.8: ✅ 加载期拦「套餐里没有一条 BBS」+ `drop_asset_refs` 拦「去掉最后一条 BBS」。10.8 的语义耦合在本层可表达的就是「套餐必须含 BBS」—— MKP 预设不建资产条目、路径由命名规则算（doc §12.5），它的"缺"不经过资产域
    - 10.9（顺手抓到的）：`bundles.toml` 一度被工具写成 CRLF，被 `the_source_files_keep_lf_line_endings` 当场抓出（Task 8.9 那条判据的又一次实弹验证）—— 已转 LF

- [x] Task 11: 校验层补齐（阶段②「检查内容」）
    - 11.1: ✅ 引用完整性盘点：`image`/`icon`→资产（9.8）、bundle→套餐、`assetRef`→资产（10.5/10.7）已在加载期拦成 error；**`presetFile` 刻意不查** —— 它是 G-2 要删的 B 套悬空名字（Task 16.3），报 9 条噪音没有行动价值。参数源（配方）这一面见 11.5/11.6
    - 11.2: ✅ 盘点 + **修了一个真 bug**：值越界 / 枚举 / 条件环 / 废弃 / 孤儿键已由 `issues.rs` 覆盖（写入口另在 `apply_values` 拦未知键）；但枚举检查对 **bool(switch) 字段误报** —— 真数据里它们挂的 `'off'/'on'` choices 是显示文案不是取值域，真清单跑一遍会炸出 54 条假阻断（wiping.* 五条 + first_pen_revitalization_flag × 9 版本）。已修：`valueType == bool` 跳过枚举检查，取值域由类型保证。判据：夹具枚举阻断测试照旧红（wiping.mode 是真枚举），真数据 `the_real_recipe_and_catalog_line_up` 里无一条 choice 阻断
    - 11.3: ✅ 机型 id（跨文件）与版本 id（机型内）**大小写不敏感**唯一 —— 加载期 `check_unique_ids`，撞了整目录拒载（corrupted），错误带撞的是哪两个 id。资产 / 套餐 id 已在各自 load 时查（Task 8.8 / 10），四类齐了。判据：直接喂列表（Windows 写不出仅大小写不同的文件名，输入面收小之后这个形状才测得到）+ 副本目录加载期两条实测红
    - 11.4: ✅ 孤儿 = 配方里没有任何版本定义指向的参数源：机型级（配方有清单不认）与变体级（`机型:变体` 没有对应版本定义，gen-presets 会生成一份谁也不引用的内置预设而生成不报错）→ **待办**
    - 11.5: ✅ 清单 ↔ 配方（G-1 方案甲：`preset_recipes.toml`）三个方向对齐检查 `recipe_alignment`：机型存在配方缺它 / 配方有清单不认（机型级 + 变体级）/ 清单版本配方缺变体。**占位机型（无 `[dimensions]`）整台跳过** —— 不参与交付，两条链都没有它是闭合的（与 A2L 占位提示同一口径）。doc §5.1 的「检测那一半」；修复入口归 Task 16.2 的跨文件事务
    - 11.6: ✅ 分级：配方对齐全部**待办**（warning 语义：机型 / 版本可以先存在，参数源后补，不挡工作台生成 —— render 读参数注册表不读配方；`gen-presets --check` 那条链构建时才真正报错）。结构上分成两个闸门：`wb_generate` 用的 `inspect` **不含**配方对齐（生成不读配方），预检用 `preflight`（inspect 的全部 + 对齐）
    - 11.7: ✅ 盘点确认：加载期错误带文件路径 + detail（既有风格）；校验层 `Issue.at` 带 view / 机型 / 版本 / 字段（`every_issue_is_actionable` 判据锁着）。新检查的错误都带 id 与撞名清单
    - 11.8: ✅（后端部分；UI 归生成视角那一轮）`wb_preflight` 升级为 `issues::preflight`：四类检查（引用 / 参数 / 唯一性 / 孤儿+对齐）结果全进 Report，配方读不回来是**报告里的一条**而不是命令失败 —— 校验层停摆比数据坏了更糟。`api.ts` 的 `IssueReport` 类型已就位；`App.tsx` 明写生成视角含校验三档的展示在 Task 17 落地，本轮不动 UI
    - 11.9: ✅ 清单里有、上游不认的机型与版本 → `upstream_drift`（**提示**档：render 已用自己的清单，这类不一致从此不再以「机型不存在」暴露，但发布侧还看着上游的过渡期里要有处可见）。机型级报过就 continue，版本不逐条刷屏。真数据实测：夹具上游 + 真清单 → A1_MINI / P2S / X1C 三条机型级 + A1.FASTV3.3 一条版本级（夹具上游只有两版）
    - 11.10（过程）：`preset::PRESET_RECIPES_TOML` 公开（原在 gen-presets 的 bin 里私有 include），`gen_presets` 转引 —— 两处 `include_str!` 字面量靠人眼盯着同一文件的隐患消除；K-G7 复验绿

- [x] Task 12: 交付层 —— `dist-presets/` 结构与目录类 JSON
    - 12.1: ✅ 目录结构定稿（`app/dist.rs` 模块头）：`content/`（三份 JSON）+ `presets/mkp/`（**子层保留** —— MKP 与 BBS 是两类预设，G-3 开放维度，且 wb_generate 已按此写，定稿是承认现状为契约）+ `assets/`（**沿用资产根目录形状**，`printers/` 而非示意的 `machines/` —— path 在两个根下逐字节同形，不存在第二份路径映射；改用示意等于重写 21 条 path）+ `manifest.json`（最后写，既有）
    - 12.2: ✅ `machine_catalog.json`：brands + 6 台机型 10 版（含 A2L 占位，`hasDimensions: false` 是它的标注，与上游契约同形）；版本条目带 `mkpPresetAssetId` 连接键（指向 manifest 的 assets[].id）；**不带 `presetFile`**（G-2 待删字段不进交付契约）
    - 12.3: ✅ `bundles.json`：`bundles.toml` 五字段直出（id/display/machineId/assetRefs/updatedAt —— 旧契约没有 updatedAt，多给无害）
    - 12.4: ✅ `assets_index.json`：**只编进交付的引用可达集**（机型 image/icon + 套餐 assetRefs，去重照 assets.toml 登记顺序；正式的可达性分析在 Task 13，集合不变）—— sha256/size 是 Task 13.6 的事（与 manifest 扩容一起），这里只管清单、位置、归属
    - 12.5: ✅ 文件名字段全部由命名函数算出，边界照实写进 `dist.rs` 模块头：MKP 产物名 = `preset::preset_file_name`（唯一实现）；资产 path 是**登记值**（assets.toml 唯一一份路径），JSON 原样引用、生成器里零字面量 ——「不存在手写字面量」指生成器里不再出现第二份名字，不是把登记值改成计算值
    - 12.6: ✅ 判据两条：夹具级 `write_content_copies_every_referenced_file…`（可达集逐条复制 + 缺文件资产报错带 id）与真数据 `the_real_delivery_set_matches_the_real_references`（**可达集 13 = 图 5 + 图标 3 + BBS 5**；模型 3 与 0.2mm BBS 4 刻意不进——没被引用；12.6 逐条存在 + 套餐 assetRefs join 资产索引闭合；反空转锚点 6 台 / 10 版 / 5 套餐 / 13 条）
    - 12.7（侦察结论，9.3a 不改名）：BBS 旧云端代号（`MKPProcess X1 0.4 0.24.json`）活在**两处**——文件名 + JSON 内容身份（`name`/`print_settings_id` = "MKPProcess X1"），而 `inherits` 已用新名（`@BBL X1C`）。**仓内代码零依赖**（assetUrl 只做空格编码透传，无 TSX 调用方；assets.toml 的 path 是唯一引用处）。裁决建议：文件名与内容身份是**切片器侧用户可见的预设名**，改了会让已导入用户的预设列表漂移；asset id 已是稳定键、G-0 命名函数只管 MKP 产物 —— **建议维持登记值不改**，9.3a 挂到「消费端（切片器导入行为）确认后再裁决」，不为交付层阻塞
    - 12.8（边界照实记）：wb_publish 现在写三份 JSON + 13 份资产 + manifest（仍只编 mkp_preset 9 条）——manifest 扩容到全资产、残留文件 blocker、可达性正式化都在 Task 13

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
    - 14.9: **「同步基线」做成显式、带 diff 确认的独立入口**（来源：Task 6.4 的裁决 —— 并入本任务）。三条要求不许松：① 先展示 diff，人确认后才写；② 是独立动作，不出现在「新增版本」的必经步骤里；③ 写的是 `crates/postprocess/tests/fixtures/presets/`，落点闸只认它或系统临时目录（`generate::check_baseline_target`）。底层规则见 `docs/ARCHITECTURE.md` §10.6

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
