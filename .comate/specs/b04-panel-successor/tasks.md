# B04 工作台接管 mkppanel —— 任务计划（第五版：重排顺序）

## 为什么重排

第四版的顺序在两处失效了：

1. **Task 8 实际上是两件事。** 「清单换源」和「值换源」耦在一个任务里，
   做完前者之后全套测试是绿的、界面行为也对 —— 那说明它们本来就该是两个任务。
   （前者已完成，见下方「已完成」。）
2. **上游层要整个删掉，这是一条定案（2026-09-23）。** 终点是
   **`<repo>/presets/` 一个文件夹装下工作台需要的全部数据**，`mkpse-presets` 不再被读。
   第四版把「删旧层」塞在 Task 13「清场」里，可是每做一页界面都要先面对双轨 ——
   顺序反了。删旧层不是清场，是前置。

所以这一版的排法只有一条原则：**先让数据成为一条真相，再按页交付界面。**
每个任务结束时都要求：两种 feature 的 `cargo test` 绿、clippy 零警告、
`presets/` 的真数据未被意外改动（sha256 比对，不用 `git status`）。

## 终点（doc §6，逐条可验收）

1. `<repo>/presets/` 是唯一数据源，代码里**没有一处**读 `mkpse-presets`
2. 侧边栏若干页，**一页一个数据文件一件事**，页与页之间不共享编辑状态
3. 没做完的功能**不摆按钮**
4. 加机型 / 加版本 / 删版本 / 改元字段 / 改参数值 —— 五条写路径都能一路走到落盘，
   且落盘后界面显示的是重读盘的结果
5. 矩阵是参数页里的一个查看开关，不是一页
6. 生成与发布能产出客户端要的产物与清单

---

## 已完成（第四版 Task 1–8 前半）

- [✓] Task 1: 摸清结构 + 定方向；12 个核心源文件搬进 `MKPSupportEase/presets/`
- [✓] Task 2: 读 `presets/*.toml` 五类源全部读通（`ParamDef` / `TabMeta` 从旧层借用不复制）
- [✓] Task 3: 往返保真 —— 含 56 KB `param_registry.toml` 逐字节、`one_edit_only` 判据
- [✓] Task 4: 加版本，端到端（四条拒绝路径 + 写完重读盘再返回）
- [✓] Task 5: 加机型（`create_new` 原子占路径，存在即拒绝；ID 查 id 与别名两处）
- [✓] Task 6: 删版本（跨文件孤儿引用查询 + 两步确认 + `warn` 留痕）
- [✓] Task 7: 改机型与版本的元字段（清空 = 删键；修掉引号漂移，`literal_str`）
- [✓] Task 8: **清单**换源 —— `Committed.catalog` 来自 `presets/machines/*.toml`；
      新建版本落进清单；那条死胡同（新建版本保存后树上看不见）正面转正

---

## 新顺序

- [x] Task 9: 参数值搬进 `presets`（原 8.3，**读**那一半）
    - 9.1: ✅ 字段定义整个搬家：`upstream/registry.rs` **删掉**，模型（`ParamDef` /
      `TabMeta` / `ShowWhen` / `ValueType` …）与那三条一致性断言移进
      `presets/registry.rs`，`ParamRegistry` 同时持有 `param_registry.toml` 与
      `layout_schema.toml`（一致性要同时看两边才查得出来，所以不分两个所有者）
    - 9.2: ✅ `domain` / `app` 全面换源：`use presets::ParamRegistry as Registry` ——
      签名不动，60 多处调用一处没改；`Book` 多一个 `presets` 字段，
      `digest` 早在 Task 8 就改成收 `(registry, 机型 id, 版本 id 列表)`，这里一个字没动
    - 9.3: ✅ 真数据判据全部改跑 `presets/`（归并无损 / doc §3.3 口径表 /
      showWhen 链 / 空串默认值）。**外加一条一次性对齐判据**
      `the_toml_and_the_retired_json_agree_on_every_value`：逐字段比出厂默认与三张机型表
    - 9.3b: ✅ **那条判据抓到了一处真差异**：42 条字段的 `defaultValue` 在旧 JSON 里是
      整数 `0`，TOML 里是 `0.0`。是旧 JSON 丢了信息（源文件写的就是 `0.0`，
      `valueType = 'float'`，构建那步按 Go 的 `json.Marshal` 印成了 `0`）。
      判据因此按**数值**比，并反过来断言"表示差异确实存在"——
      已知下游影响只有一处：`fingerprint()` 变了，换源后所有版本会被判「待生成」一次
    - 9.4: ✅ `wb_registry` / 健康视图 / 字段定义页的数据源跟着换；
      `updated` 来自 `param_registry.toml`
    - 9.5: ✅ 跨文件断言（`machineVariants` 的键必须是真机型或真版本）从 `Upstream`
      移进 `Presets`；`Catalog::machine_keys()` 补在 presets 一侧
    - 9.6: ✅ 测试夹具：新增 `load_from_json_fixture`（把 JSON 形状写成 TOML 再走真 loader）——
      6 处内联夹具因此只改一行；**生产路径永远只读 TOML**


- [x] Task 10: 参数值**写回** `param_registry.toml`（原 8.1，**写**那一半）
    - 10.1: ✅ 定案落地：**不做覆盖层**（doc §1）。改一个机型/版本的值 =
      改 `[params.machineVariants]` 里 `机型` 或 `机型:版本` 那个键
    - 10.2: ✅ `ParamRegistry::set_variant` / `clear_variant`；
      `literal_str` / `can_be_literal` / `one_edit_only` 从 `presets/catalog.rs`
      搬进 `presets/mod.rs`（机型文件与 `param_registry.toml` 共用同一套写入纪律）
    - 10.3: ✅ 「清空」= 删键；删到那张表为空时**连 `[params.machineVariants]` 表头一起删** ——
      留一个空表读起来像"这里有机型差异"，而其实没有
    - 10.4: ✅ **写回口径按真数据实测定**：纯机型键裸写（`P1S = 1.1`）、
      带冒号的键单引号（`'P1S:LITE' = 1.1`）、`float` 保 `.0`、`int` 不写成浮点、
      多行 G-code 退回双引号 + `\n`（`literal_str` 自带兜底）
    - 10.5: ✅ **跨文件那道门禁**：`Presets::set_variant` 先查 owner 是真机型/真版本。
      写进一个不存在的键的后果不是"多一条垃圾"，而是下一次加载被跨文件断言判成
      `Corrupted` —— 整个工作台起不来
    - 10.6: ✅ 失败不留痕：三条拒绝路径各有自己的话，被拒之后**盘上与内存里的文档都一字未动**
    - 10.7: ✅ 六条判据：只动一处 / 清空删键且别的键不动 / G-code 逐字往返 /
      int 与 float 的表示不漂 / 拒绝不留痕 / 不存在的机型与版本被拒（附对照组）
    - 10.8: ⏸ **移到 Task 11**：`storage::save` 改写 TOML、以及「值原来在机型基底、
      改完只属于某一版」那个归并逆运算。它们必须和「删掉自造 json 的读」同时落地 ——
      只换写不换读的话，树上显示的是旧值，那是个比现在更糟的中间态


- [ ] Task 11: 删掉自造的 `workbench/machines|versions/*.json`
    - 11.1: 删 `storage.rs` 里 `MachineFile` / `VersionFile` 的读写与 `store` 的对应路径
    - 11.2: `storage::save` 的机型基底与版本覆盖两段改成调
      `Presets::set_variant` / `clear_variant`（原 10.8，**和 11.1 同时落地**）
    - 11.3: 归并的逆运算：值原来在机型基底、改完只属于某一版时，键形状要跟着变
      （`A1` → `A1:FAST`）。这一步最容易把别的版本的有效值悄悄改掉，
      判据要逐版本比对改动前后的有效值，**只允许目标那一个变**
    - 11.4: 版本身份里那几项（改名 / 归档 / BBS）搬去哪，逐项定：
      改名 → 机型文件的 `name`；归档 → 机型文件加一个键；BBS → 套餐那一层（Task 15）
    - 11.5: 已有的 `workbench/` 旧文件：给一条**一次性迁移**或明确的丢弃提示，
      不静默忽略（静默忽略等于"我的改动去哪了"）
    - 11.6: 草稿与撤销栈那一套留用，只换底下的数据源（Task 8.4 已成立，这里只回归）


- [ ] Task 12: 其余源文件搬进 `presets/`，**上游层删掉**
    - 12.1: `registry/fallback_registry.toml`（20 条应急规则）
    - 12.2: `bundles/*.toml`（5 个套餐）+ `preset_registry.toml`
    - 12.3: `assets/*.toml`（18 条资源登记）—— 它们是清单，不是文件本身
    - 12.4: `release/_meta.toml` + `release/versions/*.toml`（发布轨道与版本说明）
    - 12.5: **待定案**：二进制资源（机型图片 / BBS json / MKP 产物）落在
      `presets/assets|bundles|mkp/` 还是继续外挂 —— 它影响仓库体积与 git 策略，动手前问
    - 12.6: **不在 b04 范围**：`faq.toml` / `notification.toml` / `theme/*` /
      `about.toml` / `event.toml` 这些是客户端内容，工作台这一轮不给它们编辑页；
      要不要先把文件搬过来单独决定
    - 12.7: 删 `src-tauri/src/workbench/upstream/` 整个目录与 `paths::upstream_*`；
      `Roots.upstream` 从 DTO 里去掉
    - 12.8: 判据：**源码扫描断言** —— 非测试代码里不许出现 `mkpse-presets` /
      `content/` / `manifest.json` 这些字样（原 `upstream_layer_has_no_write_path` 的反面）

- [ ] Task 13: 写入纪律（原 Task 9）
    - 13.1: 所有写盘只经一处，源码扫描断言（原 b03 Task 10.8，一直挂着）
    - 13.2: 「外面改过」→ 一条提示，不拦不弹选择（`presets/` 的 mtime 检测）
    - 13.3: 回收站与撤销的口径复查：现在清单可写，「删版本」是不可逆的，
      而「删值」有草稿兜着 —— 两种在界面上要说成两件事

- [ ] Task 14: 侧边栏 + 只摆做完的页（原 Task 10）
    - 14.1: 左侧边栏（内容 / 交付 / 系统 三组），**没做的页不摆**
    - 14.2: 砍掉顶部五页签与左下四项里没做完的那些
    - 14.3: 顶部状态条与保存按钮常驻；切页不丢草稿（doc §5）
    - 14.4: 已做：`data-page` 隔离、底部空态各页自己的话

- [ ] Task 15: 机型与资源页补齐（原 Task 11 + doc §2.1）
    - 15.1: 尺寸（画布 / 移动范围 / 边缘 / 涂胶区 / 擦料 X / 标定点）
    - 15.2: 禁区多边形点集编辑；**有禁区必须有尺寸**（旧层已有这条断言，接过来）
    - 15.3: 套餐合进这一页：版本卡上显示「最终资源（MKP + BBS）」并可换
    - 15.4: 资源的反向引用**只读可跳转**（「这个资源还被 A1_MINI 的 3 个版本用着」）
    - 15.5: 删机型（连带版本进回收站 + 可恢复）

- [ ] Task 16: 参数页并进来（原 Task 12）
    - 16.1: 现有 `ParamDesk` 搬进「参数」页（形态是对的，**不重做**）
    - 16.2: 「横向比较多个版本」开关 → 就地换矩阵；矩阵不再是一个页面
    - 16.3: 删 `compareKey` 那套跨页跳转
    - 16.4: 改字段定义（label / min / max / 默认值）；`tomlKey` **不许改**（doc §5）

- [ ] Task 17: 生成与发布
    - 17.1: 生成产物到 `presets/mkp/*.toml`，Diff → 写入 → 完成 三步
    - 17.2: 新机型没有 `presetFile` → **阻断**，并指到那一格
    - 17.3: 清单定稿（`manifest` 那一份改由我们生成）+ 真哈希盘点

- [ ] Task 18: 清场与验收（原 Task 13 + 14）
    - 18.1: ✅ 已做（`presets/` 在 6935029 就进了 git，HANDOFF 里"未跟踪"那句已过期）——
      `git status` 现在是真判据了

    - 18.2: `b03-*` 两份 doc 顶部各加一句「哪几条已被 b04 推翻」
    - 18.3: 删 `App.tsx` 里作废的结构与死代码
    - 18.4: 两种 feature 的 test + clippy、`npm run lint` 与两个构建
    - 18.5: 手动主线：新增机型 → 新增版本 → 改参数值 → 生成 → 打开 toml 看对不对
    - 18.6: 手动：每页点一遍，确认页与页之间互不影响

---

## 顺序为什么是这个

- **9 → 10 → 11 是一条链**：先读得到新地方（9），再写得进新地方（10），
  才能删掉老地方（11）。反过来任何一步都会留下一个"值在两处、谁赢不确定"的状态。
- **12 在界面之前**：上游层活着的时候，每做一页都要问"这一页的数据读哪一层"，
  而那个问题每页都要重答一次。
- **13 紧跟 12**：写入口刚刚收完，断言要趁这时候钉住，不然下一页又开一个口子。
- **14 在 15/16 之前**：先有壳，页才有地方放；而且"没做完不摆按钮"这条
  要在加页之前先生效，否则界面会先难看一阵。
- **17 最后**：生成的输入是前面全部数据，早做等于对着半份数据调格式。
