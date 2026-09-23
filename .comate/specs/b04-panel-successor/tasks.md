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



- [ ] Task 12: 删掉自造的 `workbench/machines|versions/*.json`（原 Task 11）
    - 12.1: `storage::save` 的机型基底与版本覆盖两段改调 `Presets::set_variant` / `clear_variant`
    - 12.2: `storage::load` 删掉 `MachineFile` / `VersionFile`
      （**盘上那个目录是空的，不需要迁移**）
    - 12.3: 归并的逆运算：值原来在机型基底、改完只属于某一版时，
      键形状要从 `A1` 变成 `A1:FAST`。判据逐版本比改动前后的有效值，**只允许目标那一个变**
    - 12.4: 按 `REPORT-2026-09-23.md` §7 的决定删掉清单类 patch
      （改名归机型页 / 不做归档 / BBS 归套餐层 / 新建·克隆·移动·删除版本归机型页）
    - 12.5: 撤销栈只服务值编辑；草稿与懒落盘那一套留用
    - 12.6: 判据：「改一个参数值 → 保存 → 重开工作台，值还在」第一次真正走通

- [ ] Task 13: **M1 + M2 —— 建 workspace，后处理内核迁进来**
    - 13.1: 新建 workspace 根 `Cargo.toml`，`src-tauri` 变成成员；
      依赖版本统一（新增 `tracing-subscriber` / `clap` / `ctrlc`）
    - 13.2: `mkp-ssr/crates/core` → `crates/postprocess`，**纯移动**：
      包名 `mkpse-postprocess`、lib 名 `postprocess`，逻辑一个字不改
    - 13.3: `mkp_pp::` → `postprocess::` 全量替换；**不留过渡别名**
    - 13.4: `src-tauri` 刻意不领 workspace lints（Tauri 的宏会碰到 `unsafe_code = forbid`）
    - 13.5: 判据：它自带的 20 个测试文件全绿 + 我们原有的 274 条仍绿 + `cargo tree -d` 无重复依赖

- [ ] Task 14: **M3 —— 尺寸 / 别名 / 禁区改从 `presets/` 读**
    - 14.1: 删 `crates/postprocess/assets/machine_dimensions.json` 与
      `machine_catalog_extra.json`，改成从 `presets/machines/*.toml` +
      `forbidden_zones/*.toml` 读（或由我们生成同形状的数据）
    - 14.2: 那条「125 字段等值」的判据从**跨仓比对**变成**同仓单一来源**
    - 14.3: 别名 23 条由 `externalAliases` 生成；`normalize_to_canonical` 查不到仍返回空串
    - 14.4: 禁区未命中只 warn 不阻塞这条行为保留，但话术改成我们自己的
    - 14.5: A2L 的路径复查：尺寸表没有它 → `load_ir` 拒它。这条要与「占位机型」口径一致

- [ ] Task 15: **M4 —— 预设解析迁进来，注册表合一**
    - 15.1: `mkp-ssr/crates/preset` → `crates/preset`，**纯移动** +
      包名 `mkpse-preset` / lib `preset`；`mkp_preset::` → `preset::`
    - 15.2: 删 `assets/param_registry.toml`（那份分岔副本），改读
      `presets/registry/param_registry.toml` —— **注册表从此只有一份**
    - 15.3: 那 4 处分岔（两处 label、一处 `uiComponent`、一块 `[[params.choices]]`）
      随之消失；加一条判据确认"只有一份"
    - 15.4: `assets/presets/*.toml` 9 份暂留当对照基线，Task 18 删
    - 15.5: `BUILTIN_PRESETS` 手写表改成按清单校验（缺一份要报错，不许静默）
    - 15.6: 判据：它自带的 8 个测试文件全绿

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
