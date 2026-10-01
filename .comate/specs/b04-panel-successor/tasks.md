# B04 工作台接管 mkppanel —— 任务计划（第六版：并入后处理迁移）

## 为什么又重排

第五版的顺序（先统一数据、再按页交付界面）**仍然成立**，但范围变了：
doc §8 定案「后处理与预设解析迁进来，只留一个项目」。这带来三件事：

1. **多了两处已发生的分岔**：参数注册表两份（4 处实质差异）、
   值也有两个真源（`preset_recipes.toml` 自称真源，那 9 份内置预设是从它生成的）。
   第五版把「删掉自造 json」当成统一数据的终点 —— 现在它只是三分之一。
2. **原 Task 12「删掉 upstream 层」不再是收尾，而是迁移的产物**：
   迁入之后那一层自然没有对象了，不用专门去删。
3. **原 P2「跨仓纵向切片」作废** —— 迁进来之后，产物由我们生成也由我们读，
   在同一个进程里验，比跨仓库跑干净得多。

所以第六版把迁移的七段（M0–M7，见 [MIGRATION-PLAN.md](./MIGRATION-PLAN.md)）
与剩下的界面任务编成一条线。原则没变，只是「一条真相」现在有三处要收：
**自造的 json、注册表两份、值两个真源。**

## 终点（doc §8.6，逐条可验收）

1. `cargo test` 在 workspace 根跑，三个 crate 的判据全绿
2. `rg mkp_pp` / `mkp_preset` / `mkpse-presets` / `content/` 在非测试代码里零命中
3. 「改一个值 → 生成 → `load_ir` 读通 → IR 里是新值」有一条自动判据
4. 至少一条真实后处理用例通过（真 G-code 进去，输出可验证）
5. `presets/` 12 个文件的 sha256 与迁移前一致（或有逐文件审过的差异记录）
6. 侧边栏一页一件事，没做完的功能不摆按钮
7. 文档与代码里不再有把我们自己说成"下游"的地方

---

## 已完成（Task 1–10）

- [✓] Task 1: 摸清结构 + 定方向；12 个核心源文件搬进 `presets/`
- [✓] Task 2: 读通 `presets/*.toml` 五类源（模型不重写，只换 loader）
- [✓] Task 3: 往返保真 —— 含 56 KB `param_registry.toml` 逐字节、`one_edit_only`
- [✓] Task 4: 加版本，端到端（四条拒绝路径 + 写完重读盘）
- [✓] Task 5: 加机型（`create_new` 原子占路径；ID 查 id 与别名两处）
- [✓] Task 6: 删版本（跨文件孤儿引用 + 两步确认 + 留痕）
- [✓] Task 7: 改机型与版本的元字段（清空 = 删键；修掉引号漂移）
- [✓] Task 8: **清单**换源 —— `Committed.catalog` 来自 `presets/machines/*.toml`；
      新建版本落进清单；那条死胡同正面转正
- [✓] Task 9: 字段定义换源（读）—— `upstream/registry.rs` 删掉，模型与三条一致性断言
      搬进 `presets/registry.rs`；抓到 `0` vs `0.0` 那处真差异
- [✓] Task 10: 参数值**写回**能力 —— `set_variant` / `clear_variant`，六条判据；
      写回口径按真数据实测定（裸键 / 单引号 / `.0` / G-code 退双引号）

**这一轮另外做完的（不属于原编号）**：P0 双仓审计（`AUDIT-EVIDENCE.md`）、
P1 契约 1.0（`DATA-CONTRACT.md`）、A2L 占位机型的定性与话术、
产物名与 `release_time` 换成消费端口径并断掉对上游 `file_name` 的依赖。

---

## 新顺序

- [ ] Task 11: **M0 —— 先比值，再搬代码**
    - 11.1: ✅ 判据落地：`our_render_matches_the_machine_verified_baseline`
      —— 我们的 `render()` 对真数据渲染 9 份，与基线（那边 `gen-presets` 生成、
      **上过真机**的 9 份）逐份比。头部 `uuid` / `release_time` 两行按定义会变，不参与比较
    - 11.2: ✅ **结论：值全对上了。9 份逐行同集合，一处值差异都没有。**
      两套真源（`presets/` 与 `preset_recipes.toml`）等值 —— 迁移**不需要修数据**，
      Task 11.3/11.4/11.5 原来担心的"哪边对、要不要并数据"整个消失
    - 11.3: ✅ 判据分了两层（这是它能给出上面那个结论的原因）：
      先比排序后的行集合（值），再比原序（键序）。合在一起报「不一致」
      等于把一个能一句话解决的问题说成一个要查数据的问题
    - 11.4: ✅ **键序规则查清了**（实测 `A1-standard.toml` 的 `[toolhead]` 8 键与
      `[wiping]` 57 键）：不是注册表出现序，也不是 `layout.order`，而是
      **「领头键 + 其余按大小写不敏感的字母序」** ——
      `[toolhead]` 领头 `offset` `speed_limit`，`[wiping]` 领头 `have_wiping_components`，
      其余严格字母序（`MKP_retract` 排在 `glue_z_offset` 与 `prime_length` 之间，
      证明大小写不敏感）
    - 11.5: ⏸ **精确的领头键清单要读那边的 struct 才能定**（那形状像 Go 的
      「struct 显式字段 + map 按 key 排序」）。落地时机因此是 **Task 15 之后** ——
      那时 `crates/preset` 的模型就在我们仓库里，照着它排即可，不用猜
    - 11.6: ⏸ 键序改完后把那个 `eprintln!` 换成 `assert!`（现在每次跑测试都会打一行
      「已知差异，待收尾」，故意放在眼前而不是躺在清单里）
    - 11.7: ✅ **这一段的结论：可以放心搬代码了。** 值等值已证，
      剩下的键序是纯口径问题、不影响语义，而且它的落地依赖迁移本身



- [x] Task 12: 删掉自造的 `workbench/machines|versions/*.json`（原 Task 11）
    - 12.0: ✅ **批量写入口**：`Presets::apply_values(&[(key, owner, Option<值>)])` ——
      先把 owner 与字段全查一遍，再改内存文档，最后**写一次**。
      逐条写的代价不只是把 56 KB 写 N 遍，而是中途失败会留下
      「改了前三条、没改后两条」的文件，而那种状态没有任何判据能描述它。
      `set_variant` / `clear_variant` 现在都是它的单条壳子。三条判据：
      一批一次落盘 / 一条不合法整批不写（盘与内存都一字未动）/ 空批次不碰 mtime
    - 12.1: ✅ `storage::save` 的值改动合成一批 `plan_value_edits`，一次 `apply_values` 落盘。
      剩下只写 `delivery.json` / `built.json` 与清草稿
    - 12.2: ✅ `storage::load` 删掉 `MachineFile` / `VersionFile`
      （**盘上那个目录本来就是空的，没做迁移**）
    - 12.3: ✅ **归并的逆运算**。两条判据（`app::storage`）：
      - `a_machine_level_edit_rewrites_the_promoted_version_keys` ——
        归并会把「每个版本都写了同一个值」**上提**成机型基底。只写裸键 `P1S`
        会被 `P1S:LITE` 那条更具体的键盖回去：**用户点了保存，值又变回去，全程没有一步报错**。
        所以机型层的一刀要同时落到那一整组上提过的版本键上。
        判定与 `digest` 共用 `variants::promoted_to_base`，不许各写一遍
      - `a_version_level_edit_touches_only_its_own_key` —— 另一半：
        版本层只写 `A1:STANDARD` 这一个键，**裸键 `A1` 要留着**，
        `A1/FAST` 还靠它继承。顺手删掉它会让别的版本失去继承来的值
    - 12.4: ✅ 按 `REPORT-2026-09-23.md` §7 删掉七种清单类 patch：
      改名 / 归档 / 还原 / 挑 BBS / 新建 / 克隆 / 移动 / 删除版本。
      `Patch` 只剩 `SetValue / SetVisibility / SetBundle / MarkBuilt`；
      `MovePreview` 那一整组只读推演与 `wb_preview_move` 一起删了
    - 12.5: ✅ **撤销栈只服务值编辑**。`Draft` 只剩 `values / visibility / bundles / built`，
      草稿与懒落盘那一套留用
    - 12.6: ✅ 判据「改一个参数值 → 保存 → 重开工作台，值还在」第一次真正走通
      （`app::tests::a_value_edited_then_saved_survives_reopening_the_workbench`：
      改值 → 保存 → `reload_from_disk` → 盘上 `machineVariants` 是 7.0 →
      来源层是「版本」→ 产物指纹已变）
    - 12.7: ✅ **`Layers` 五层确实退回三层了**（这一条是整个 Task 12 最重的部分）：
      - `Layers::new(registry, machine_id, base, over)` 四个参数；`without_upstream` 合并进来
      - 查找顺序从五档（我们的版本→表里版本→我们的机型→表里机型→出厂）
        变成三档（版本→机型→出厂）
      - `Book` 不再有 `digests`；`bases` / `overs` 现在是「`machineVariants` 归并结果 ⊕ 草稿」
      - 翻掉的两条旧约定写进了 `layer.rs` 模块文档：**挂回继承现在逐层退**
        （以前退的是"上游那一半"），以及"没法压回出厂默认"那条代价没有了
      - 「每层有几项 / 其中自有几项」两个数是同一个了 —— 前端 DTO 收成一个 `items`
      - 归档、`is_new`、版本自带 BBS 一并消失（`VersionIdentity` 只剩身份）

  **验收**：Rust 264 绿（两种 feature）、`clippy --features workbench -D warnings` 零警告、
  `npm run lint` 与 `npm run build` 都过。没改任何数据文件。


- [ ] Task 13: **M1 + M2 —— 建 workspace，后处理内核迁进来**
    - 盘点：✅ [TASK-13-MIGRATION-INVENTORY.md](./TASK-13-MIGRATION-INVENTORY.md)
      （只读取证：迁什么 / 依赖什么 / 怎么分类 / 怎么切批 / 拿什么验收）
    - 盘点定案（2026-09-23，见该文件 §6 + [TASK-13-WRITE-DISCIPLINE-OPTIONS.md](./TASK-13-WRITE-DISCIPLINE-OPTIONS.md)）：
      **M2 拆 M2a/M2b**（零 diff 搬运 + 独立改名）；bin 的三个依赖先照搬、M5 再定；
      两个 asset JSON 与 9 份测试用预设按「搬迁 + 头注 + 记入 M5 收尾清单」处理；
      写盘纪律**C（源码扫描断言）为主 + A1（根上唯一一份 `clippy.toml`）为辅**，B 推迟
    - 13.1: ✅ **已完成**（`70eed0c` 布局 + `44c5ebe` 依赖统一）。
      根 `Cargo.toml` 建好、`src-tauri` 成成员、`Cargo.lock` 搬到根且 cargo 未改写它。
      依赖版本统一到 `[workspace.dependencies]`；`serde_json` 开了
      `float_roundtrip` + `preserve_order` —— **我们自己的 264 条重跑过，全绿**
      （这一档变化没动到我们的行为）。`toml` 取 **0.8** 不取 1.x：内核用 0.8 档的 API，
      而 M2a 的搬运证据是内容零 diff，改代码与搬运不能混在同一条 diff 里
    - 13.2 = **M2a**（下一步）：`mkp-ssr/crates/core` → `crates/postprocess`，
      **原样复制，连包名 `mkp-pp` / lib `mkp_pp` 都先不改**。
      判据：121 个文件逐文件 sha256 与源一致 + `cargo test -p mkp-pp` 全绿
    - 13.3 = **M2b**：**单独一笔机械改名**。`mkp_pp::` → `postprocess::`（实测 64 行 / 19 文件）；
      **不留过渡别名**。另有 3 处 `env!("CARGO_BIN_EXE_mkp-pp")` 是编译期宏，bin 改名必改
    - 13.4: ✅ workspace 根的 `[lints.rust] unsafe_code = "forbid"` 已就位，
      `src-tauri` 刻意**不领**（Tauri 的宏与 macOS 那几段 objc2 会碰到 unsafe）。
      另：根的 `[lints]` **不能写 clippy 规则** —— cargo 报
      `cannot override workspace.lints in lints`
    - 13.5: 判据：它自带的 20 个测试文件全绿 + 我们原有的 **264** 条仍绿 +
      `cargo tree -d` **不新增说不清的重复**。
      （"无重复依赖"不可能按字面执行：Tauri 自己的树里本来就有 53 条；
      改口径的证据与逐项比对见 `44c5ebe` 的提交信息。`274` 也是旧数字）
    - 分支：`feat/b04-p3-migration`（本机与远端同步），草稿 PR #11 只作为 CI 的载体 ——
      每 push 一次自动跑一遍，不再逐批开 PR

- [x] Task 14: **M3 —— 尺寸 / 别名 / 禁区改从 `presets/` 读**
    - 14.1: ✅ 两份 `assets/*.json` **已删**（`assets/` 目录没剩下东西）。
      数据改成**注入**：`machine_dims::install(tables)` 或 `load_presets_dir(dir)`，
      后者读 `presets/machines/*.toml` 的 `[dimensions]` / `externalAliases`
      与 `presets/forbidden_zones/*.toml` 的 `[[zones]]`
    - 14.2: ✅ 旧快照**降级成基线**（搬到 `tests/reference/legacy_snapshot/`），
      新判据 `presets_are_the_only_source.rs` 三条：尺寸 5 台×25 字段逐字段、
      别名 23 条逐条、禁区 3 台逐点 —— 全部相等。**内核 249 条全绿，
      其中逐字节比对的 golden 一条没变**，这是"等价"的最强证据
    - 14.3: ✅ 别名由 `externalAliases` + `id` 生成（键一律大写，与旧快照同语义）；
      `normalize_to_canonical` 未命中仍返回空串（行为未动）
    - 14.4: ✅ 禁区未命中只 warn 不阻塞的行为保留；文案本来就是我们的口吻
      （只描述了"禁区数据为空，跳过填充"这件事，不引用任何外部仓库）
    - 14.5: ✅ A2L 仍然没有 `[dimensions]` ⇒ 「别名认识它、尺寸表没有它」那个差集
      仍然非空，`machine_dims_must_exist.rs` 的三条 CLI 判据继续有扫描面并通过。
      该文件现在从 `presets/machines/*.toml` 取数（不再读那份 asset）
    - **顺带抓到一条静默数据丢失**：`MachineFile` 少了 `rename_all = "camelCase"`，
      于是 `externalAliases` 全被忽略 —— 别名从 23 条变 6 条、机型识别会全挂，
      而不会有任何报错。是新判据里那条"逐条相等"抓到的
    - **待 M5 接上**：发布物必须在启动时 `install()`（数据根在运行时才知道）；
      现在没装时有一条逃生链（`MKPSE_PRESETS_DIR` → 仓库相对 `../../presets` → panic），
      只为开发与测试成立，理由写在 `machine_dims.rs` 的模块文档里

- [x] Task 15: **M4 —— 预设解析迁进来，注册表合一**
    - 15.1: ✅ `mkp-ssr/crates/preset` → `crates/preset`，包名 `mkpse-preset` /
      `[lib] name = "preset"`；`mkp_preset::` → `preset::`。拆两笔：**M4a 原样搬**
      （35 个文件逐文件 sha256 一致、索引 blob == 工作区 35/35，先用 `exclude` 不进
      `members` 以保住零 diff 证据）+ **M4b 机械改名**（对只读源树重放同一规则再
      rustfmt，**32/35 逐文件相同、零处意外差异**；3 处手改精确到 3 行）
    - 15.2: ✅ `assets/param_registry.toml` 已删，`include_str!` 与
      `registry_edit::registry_path()` 都指 `presets/registry/param_registry.toml` ——
      全仓 `param_registry.toml` **只剩一个文件**
    - 15.3: ✅ 实测是 **5 处**（不是 4 处）：两处 `label`、**两处** `uiComponent`、
      一块 `[[params.choices]]`。而且这 5 处**一条老判据都碰不到** ——
      `label`/`uiComponent` 在 `ParamEntry` 里只有 serde 读写；`choices` 白名单在
      `validate.rs:264` 有 `value_type == "string"` 这道门而那条参数是 `float`；
      `build.rs:533` 那处要求 `deprecated = true`。所以新判据
      `registry_is_the_only_source.rs` 比的是**全文字节**，不是逐字段
      （逐字段看不见 `jsonKey`/`mergeGroup`/`showWhen` 这些它不读的键）
    - 15.4: ✅ `assets/presets/*.toml` 9 份保留未动，Task 18 删
    - 15.5: ✅ `builtin_presets_match_dir.rs` 三条（名字集合 / 每条内容逐字节 /
      无重名），**用探针验过会响**：塞一份多余 toml ⇒ FAILED 且点名，撤掉 ⇒ 绿
    - 15.6: ✅ 它自带的 8 个测试文件全绿；`cargo test -p mkpse-preset` 新基线
      **106 条**（71 单元 + 8 个老测试文件 29 条 + 两份新判据 6 条）。切数据源后
      **老判据一条没红**，`registry_branch_diff` 的快照也没动 ⇒ 本轮**没有动任何 golden**
    - **口径更正**：`cargo tree -d` 基线从 **58 条 / 26 名** 变成 **61 / 27**。
      新增 3 条（`toml_edit` 0.20.2 + 0.22.27、`winnow` 0.7.15）是「`toml` 取 0.8」
      与「写回必须用 toml_edit 0.22」两条既有决定叠加的必然结果，不推翻其一消不掉。
      `Cargo.lock` 只多了 `mkpse-preset` 一个条目，没引入新的第三方版本
    - **仍然欠的**：注册表还是 `include_str!` 编进二进制（数据只有一份了，但发布物里
      改不了）。改成运行时从数据根读 + M3 欠的 `machine_dims::install()` 接线，**两件一起挂 M5**

- [ ] Task 16: **M5 —— 纵向切片：一条路走通**（原 P2）
    - 16.1: `wb_generate` 之后在**同一个进程里**用 `preset::load_ir()` 复检产物
    - 16.2: 端到端判据：改一个值 → 写进 `param_registry.toml` → 重读盘 →
      生成 `A1-standard.toml` → `load_ir` 读通 → **IR 里那一项是刚改的值**
    - 16.3: 隔离判据：未选变体的产物字节不变；别的机型文件 sha256 不变
    - 16.4: 至少一条真实后处理用例：一份真 G-code 过 `postprocess`，输出可验证
    - 16.5: 复用它的写回纪律：写完用 `load_ir` 复检（原 `preset_apply.rs:210` 那条闸）

- [ ] Task 17: **M6 —— 删掉 `upstream/` 整层**（原 Task 12.7 / 12.8）
    - 17.1: 删 `src-tauri/src/workbench/upstream/` 与 `paths::upstream_*`；
      `Roots.upstream` 从 DTO 里去掉
    - 17.2: 产物与套餐的信息改由 `presets/` + 我们自己的清单提供
    - 17.3: 修 manifest 那三条缺陷：`bundles` 悬空引用、
      没有产物资源的版本静默跳过、`channel`/`minimumClient`/`version` 透传上游
    - 17.4: 删死代码 `paths::resolve_dist`
    - 17.5: **源码扫描断言**：非测试代码里不许出现 `mkpse-presets` / `content/` /
      `manifest.json` / `mkp_pp` / `mkp_preset`

- [ ] Task 18: 其余源文件搬进 `presets/`（原 Task 12.1–12.6）
    - 18.1: `registry/fallback_registry.toml`（20 条应急规则）
    - 18.2: `bundles/*.toml`（5 个套餐）+ `preset_registry.toml`
    - 18.3: `assets/*.toml`（18 条资源登记）—— 是清单，不是文件本身
    - 18.4: `release/_meta.toml` + `release/versions/*.toml`
    - 18.5: **待定案**：二进制资源（机型图片 / BBS 曲线 / MKP 产物）落在
      `presets/` 还是外挂 —— 影响仓库体积与 git 策略，动手前问
    - 18.6: **不在范围**：`faq` / `notification` / `theme` / `about` / `event`
      这些客户端内容，这一轮不给编辑页

- [ ] Task 19: 写入纪律（原 Task 13）
    - 19.1: 所有写盘只经一处，源码扫描断言（b03 Task 10.8 一直挂着）
      —— **纪律的覆盖面这一块已提前做完**（2026-09-24，施工文档
      `.comate/specs/b04-preset-write-guard/`，6 笔提交），因为 M4c 把
      `registry_edit` 指向了 `presets/` 真源，不能等到本任务再补。已落地的：
        - `clippy.toml` 从 `src-tauri/` **上移到仓库根** ⇒ `crates/postprocess` 与
          `crates/preset` 从此受管（此前两个成员完全不受管）；
        - 生产 5 处 + 测试 10 处逐处写明理由与**退役条件**（那 5 处的退役条件全都指向本任务）；
        - 源码扫描断言 `crates/preset/tests/write_discipline_scan.rs`（3 条 + 幂等门禁 1 条），
          白名单逐条写"崩在半路会坏掉什么"，且**用探针验过会响**；
        - 写真源那一处（`registry_edit::set_range`）上了三道闸：no-op 不写盘 /
          落盘前"只许声明过的键变" / 落盘后回读 + 重新解析；
        - `generate::sync_baseline` 加落点断言（它是全仓唯一能改内核判据期望值的写盘点）。
      **本任务仍然欠的**：①「所有写盘只经**一处**」没做到 —— 现在是"每处各自原子写 +
      逐处豁免"，`fsx::atomic` 住在 `src-tauri` 而依赖方向是 `src-tauri → preset → postprocess`，
      要真正统一入口得把它下沉到内核（doc 里记为 B1，本轮刻意没做）；
      ②`presets/` 的 sha256 判据仍是**人工核**，没有自动判据（要先定"什么时候允许变"，
      而 Task 18 就要往 `presets/` 加文件）；③备份 / 回收站语义未定（见 19.3）
    - 19.2: 「外面改过」→ 一条提示，不拦不弹选择
    - 19.3: 回收站与撤销的口径复查：删版本不可逆、删值有草稿兜着，
      界面上要说成两件事

- [ ] Task 20: 侧边栏 + 只摆做完的页（原 Task 14）
    - 20.1: 左侧边栏（内容 / 交付 / 系统 三组），**没做的页不摆**
    - 20.2: 砍掉顶部五页签与左下四项里没做完的那些（现在有 6 个不符合）
    - 20.3: 顶部状态条与保存按钮常驻；切页不丢草稿
    - 20.4: 已做：`data-page` 隔离、底部空态各页自己的话

- [ ] Task 21: 机型与资源页补齐 + 参数页并进来（原 Task 15 + 16）
    - 21.1: 尺寸（画布 / 移动范围 / 边缘 / 涂胶区 / 擦料 X / 标定点）
    - 21.2: 禁区多边形点集编辑；**有禁区必须有尺寸**
    - 21.3: 套餐合进这一页：版本卡上显示「最终资源（MKP + BBS）」并可换
    - 21.4: 资源的反向引用只读可跳转
    - 21.5: 删机型（连带版本进回收站 + 可恢复）
    - 21.6: `ParamDesk` 搬进「参数」页（形态是对的，不重做）
    - 21.7: 「横向比较多个版本」开关就地换矩阵；删 `compareKey` 那套跨页跳转
    - 21.8: 改字段定义（label / min / max / 默认值）；`tomlKey` **不许改**

- [ ] Task 22: **M7 + 清场与验收**（原 Task 18）
    - 22.1: 措辞清场：文档与注释里的「上游」「消费端」「下游」换掉 ——
      只有一个项目了，这些词没有指代对象
    - 22.2: 删对照基线（`crates/preset/assets/presets/` 那 9 份）
    - 22.3: `b03-*` 两份 doc 顶部各加一句「哪几条已被 b04 推翻」
    - 22.4: 删 `App.tsx` 里作废的结构与死代码
    - 22.5: 两种 feature 的 test + clippy、`npm run lint` 与两个构建
    - 22.6: 手动主线：新增机型 → 新增版本 → 改参数值 → 生成 → `load_ir` 读 → 打开 toml 看
    - 22.7: 手动：每页点一遍，确认页与页之间互不影响

---

## 顺序为什么是这个

- **11 在最前**：它可能推翻我们的参数值。搬完 3 万行代码再发现值对不上，
  会留下一批"生成出来但和真机验证过的那份不一样"的产物，而那种错在产物上看不出来。
- **12 在迁移之前**：「改一个值能存下来」是这个工作台最基本的功能，
  它现在还断着（写回能力做好了但没接上 `save`）。带着一个断的主功能去搬代码，
  出问题时分不清是搬坏的还是本来就坏的。
- **13 → 14 → 15 是一条链**：先有 workspace（13），才能把内核放进去；
  内核进来之后先把它的数据源换成我们的（14），再迁依赖它的那一层（15）。
  反过来做会出现"两个 crate 各读一份尺寸"的中间态。
- **16 紧跟 15**：注册表刚合一，产物的生产者与消费者第一次在同一个进程里 ——
  这时候验纵向切片最便宜。
- **17 在 16 之后**：上游层要等新链路验证过才能删。删早了，出问题没有对照。
- **18–21 是界面与剩余数据**：它们依赖前面的"一条真相"，顺序沿用第五版。
- **22 最后**：措辞清场要等那些词真的没有指代对象了再做，否则改完又得改回去。
