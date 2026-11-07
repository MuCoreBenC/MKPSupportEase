# 地图 2 · 业务流程地图（主链）

> 最后核对：2026-11-07（只读代码取证；未在 UI 上执行写操作）
> 范围：**套餐 → 预设/参数 → 生成 → G-code 后处理**。每条结论带 `文件:行`，读不到/需运行才能确认的标「未核实」。
> 不写沿革，只写现在怎么跑。
> **视角提醒**：本篇是"流程视角"（一条动作链走下来）。"归谁负责 / 谁有权改它的数据 / 用什么契约"见 [`06-cores.md`](06-cores.md) —— 那条链上的四段分属不同核心，流程相邻不等于架构耦合。

---

## 1. 套餐链

### 1.1 它是什么（一句话 + 证据）

套餐是**独立的业务对象**，正文在 `presets/bundles.toml`（`presetdata/bundles.rs:50`），内容是一份**资产 id 清单**（`assetRefs`），硬性要求是「至少一条是 BBS 切片器预设」（`presetdata/mod.rs:729/790/1010`）。它**不含任何参数值，不参与渲染**。

- 真数据 9 份，每份两条（`mkPreset` + `slicerProfile`）：`presets/bundles.toml:42-107`。
- `id` 是主键，机型/版本以**裸 id** 引用它，禁止空格与 `/`（`bundles.rs:533`）。

### 1.2 结构

| 字段 | 含义 | 谁写 | 谁读 |
|---|---|---|---|
| `id` | 主键 | `Bundles::add/rename`（`bundles.rs:134/274`） | `check_bundle_refs`（`mod.rs:681`）、`bundle_bbs`（`derive.rs:298`）、`bundles_json`（`delivery.rs:444`） |
| `display` | 给人看的名字 | add / rename / copy | 工作台列表、交付 `BundleEntry.display`（`delivery.rs:300`） |
| `machineId` | 归属机型（必填） | add / copy | 加载期反查（`mod.rs:706`） |
| `assetRefs` | 资产 id 列表（不限制类型，只要解析得到） | `set_refs` 整列替换（`bundles.rs:207`）、删资产时 `drop_asset_refs`（`:386`） | `bundle_bbs` 只取 `slicer='bbs'`（`derive.rs:308-312`）、客户端 `version_files_dto`（`ipc/presets.rs:435`）、交付可达集（`delivery.rs:378-382`） |
| `updatedAt` | 上次改动日期（只展示） | add / rename / set_refs | 界面、交付 |

派生字段（不落盘）：`users`（哪些版本 `recommendedBundle` 指向它）、`default_for`（哪些机型 `defaultBundle` 指向它）——`app/bundles.rs:96-100/187-208`。

### 1.3 两档指向（谁决定"这一版用哪套"）

```
版本 recommendedBundle（一版一套） ──有──> 用这份套餐
        │没有/空串
        └──> 机型 defaultBundle ──有──> 用这份
                     │没有
                     └──> 无套餐（BBS 为空）
```

- 解析链：`Book::bundle_bbs`（`derive.rs:298-317`，草稿 overlay 优先）→ `effective_bbs`（`:327-347`）→ `bbs_source`（`:351-365`，界面据此说"本版本一份 / 跟机型默认"）。
- 改指向只改**机型文件**：`wb_set_version_field(..., 'recommendedBundle')` → `app/machines.rs:291` → `write_machine`（`:313`）；批量走 `wb_assign_bundle_versions`（`app/bundles.rs:413` → `mod.rs:1031`）。

### 1.4 页面动作 → IPC → 落盘

| 动作 | IPC | 内核 | 写哪里 |
|---|---|---|---|
| 列表 | `wb_bundles`（`app/bundles.rs:228`） | `list_of`（`:126`） | 只读 |
| 改"装了哪些文件" | `wb_set_bundle_refs`（`:255`） | `Presets::set_bundle_refs`（`mod.rs:758`）→ `set_refs`（`bundles.rs:207`） | `presets/bundles.toml`（`bundles.rs:190` 原子写） |
| 新建 / 改名 / 复制 / 删除 | `wb_add_bundle`（`:290`）/ `wb_rename_bundle`（`:329`）/ `wb_copy_bundle`（`:375`）/ `wb_remove_bundle`（`:444`） | `mod.rs:811/851/896/932` | `bundles.toml`；改名还要逐台重指 `machines/*.toml`（`mod.rs:872-889`） |
| 改指向（单/批） | `wb_set_version_field` / `wb_assign_bundle_versions` | 见 §1.3 | `machines/*.toml` |

删除被引用的套餐会被**整次拒绝并点名**（`mod.rs:938-954`）；删一条被引用的 BBS 会先摘引用，若套餐只剩它则整次拒绝（`bundles.rs:386-405`）。

### 1.5 下游影响（按"改动一份套餐之后谁变"）

| 下游 | 变什么 | 证据 |
|---|---|---|
| 工作台版本树 | `bbs_count` / `bbs_source` 徽标 | `derive.rs:515/602` |
| 生成页 | 该版是否"有资源"（配方空 **且** 无 BBS ⇒ `NoResources`） | `derive.rs:383-393` |
| 产物 TOML | **不变**（产物里搜 `bbs|bundle` = 0 命中） | `build.rs` render 不读 bundle |
| `content/bundles.json` | 变（字段直出） | `delivery.rs:444` |
| `content/assets_index.json` | 可达集变化（bundle 的 refs 并入） | `delivery.rs:378-382` |
| `content/machine_catalog.json` | 版本的 `defaultBundle` / `recommendedBundle` | `delivery.rs:348/420` |
| `catalog.json` | `bundles` 数组 + `revision` 变化 | `runtime/catalog.rs:718`、`:862` |
| `manifest.json` | **不变**（v3 已删 `bundles` 字段） | `delivery.rs:78-81` |
| 客户端 | `get_version_files` 返回的切片器文件集换成新套餐的 `assetRefs` | `ipc/presets.rs:387/417-468` |

**套餐页的读写都不走草稿**（B5 已核实）：内容编辑 `wb_set_bundle_refs` 走 `with_struct_ctx` 即时写 `bundles.toml`（`app/bundles.rs:255-263`）；改指向 `wb_set_version_field` 走 `with_struct_ctx` 即时 `write_machine`（`app/machines.rs:291-315`）。`Book::bundle_bbs` 里那条"草稿 overlay 优先"（`derive.rs:298-301`）对应的 `Patch::SetBundle` 在前端**只有类型定义、没有构造点**（`api.ts:806` 是唯一出现处）——是一条**潜在死路径**，不是活动链路。

### 1.6 套餐与"参数"的关系：**无**

- `effective_recipe` 的唯一实现 `presetdata/resolve.rs:303` 只吃 `Layers`（registry + 两层 overrides），**整文件 0 处 bundle**。
- 生成时取 `layers.effective_recipe()`（`build.rs:310-316`），不碰 bundles。
- 唯一间接落点：`build_state` 用 `effective_bbs` 判 `NoResources`（`derive.rs:383-393`）→ 生成跳过该版（`build.rs:610-612`）。即套餐只影响"这一版算不算有资源可生成"，**不影响任何参数值**。

### 1.7 套餐与"后处理"的关系：**无**

- `crates/postprocess/**` 搜 `bundles.toml|assetRefs` = 0；`hook.rs`（161 KB）搜 `bundle` / `bbs` = 0。
- 后处理读的是 **G-code 头里的切片器设定**（`crates/postprocess/src/gcode/header.rs:20-22`），不是套餐。套餐只是把"MKP 预设 + BBS 预设"两份文件**捆在一起发**（`presets/bundles.toml:24-29` 的注释是领域约定，不是代码判据）。

---

## 2. 参数 → 生成链

### 2.1 参数从哪来

1. **定义**：`presets/registry/param_registry.toml` + `presets/layout_schema.toml` → `ParamRegistry::load_from`（`registry.rs:790`）。
2. **每项字段**：`ParamDef`（`registry.rs:167-212`）—— `key / toml_key / json_key / label / section / value_type / ui_component / default_value(出厂) / min / max / step / layout.order / machineFilter`。
3. **三层取值**：`machineVariants` 里裸键 `A1` = 机型层，`A1:FAST` = 版本层；查找顺序 版本层 → 机型层 → 出厂（`resolve.rs:114-131`），并用 `Origin` 报"值来自哪层"。

### 2.2 合并规则（5 条）

1. 版本层 > 机型层 > 出厂默认（`resolve.rs:123-131`）。
2. 删键 = 挂回继承，**一次退到出厂**（机型层没有"上游另一半"）。
3. 先过滤：`machineFilter` 不含该机型、或该机型上 `deprecated` ⇒ 不进有效配方（`resolve.rs:120-121`）。
4. 有值但失效的键进 `orphan_keys`（`resolve.rs:317`）。
5. "值为空串" ≠ "没设过"（`toolhead.custom_mount_gcode` 出厂就是空串）。

### 2.3 编辑 → 草稿 → 保存

```
参数台 ParamSheetPage → wb_apply_draft（app/mod.rs:2568）
   → 只改内存（apply_patches / history.record / 草稿层）      ← 不落盘
   → 「保存」wb_save → 写 presets/（机型/版本 override）
```

`wb_apply_draft` 不写盘是设计（`app/mod.rs:2568` 起）；未保存状态由草稿与撤销栈承载。

### 2.4 渲染（参数 → 产物 TOML）

`render`（`build.rs:290`）：

- 指纹：`:312`；有效配方：`:313`（`effective_recipe`）。
- 段名 = `ParamDef.section`，段序按段内最小 `layout.order`（`:339-350`）。
- 段内键按 `tomlKey` 字母序（`:370-378`）；内联表成员按 `jsonKey`（`:381-394`）；G-code 用 `"""`（`:401-407`）。
- 产物头参考：`presets/delivery/mkp/presets/A1-standard.toml` 目前只有 `[toolhead]` 与 `[wiping]` 两段。

**含义**：注册表条目一挪，产物键序就会漂 —— 产物字节是"注册表 + 有效配方"的纯函数。

### 2.5 生成事务（预演 → 生成 → 回滚）

| 步骤 | 入口 | 行为 |
|---|---|---|
| 预演 | `wb_generate_preview`（`build.rs:1158`） | 只算不写，给逐份 diff |
| 生成 | `wb_generate`（`:627`）→ `generate_with`（`:713`）→ `generate_inner`（`:784`） | 先过 `require_clean`/影子树同步；**先全部渲染**（`:810-813`，任一失败整批不动） |
| 事务 | `delivery::generate_transaction`（`delivery.rs:1723`） | 候选集 `delivery_diffs`（`:1735`）→ 备份 `:1771` → 写/移回收站 → 回读验证 |
| 失败 | `GenerateTx::rollback`（`delivery.rs:1682`） | 逐份还原或删除；回滚也失败则报"事务失败且回滚未完成"（`:1794`） |
| 收尾 | `build.rs:858/984` | 写 `.snapshots/` 快照；写台账 `workbench/built.json`（`storage.rs:182`） |

### 2.6 产物 → 客户端 → 切片器（这条才算"消费"）

产物 TOML 的路径被拼进钩子命令：`post_process_dto`（`ipc/mod.rs:626-654`）→ `post_process_command`（`:557`）→ 用户把它贴进切片器（或客户端"复制命令"）。

### 2.7 机型尺寸与"生成"的关系（B4 已核实）

| 问 | 答 | 证据 |
|---|---|---|
| 尺寸进产物 TOML 吗 | **不进**。`render` 只取 `machine.id` 与参数层 | `build.rs:309`、产物只有 `[toolhead]`/`[wiping]` |
| 尺寸影响"能不能生成"吗 | 影响。没有 `[dimensions]` 的占位机型整台跳过 | `build.rs:601-608`（`has_dimensions`）、`presetdata/catalog.rs:683` |
| 改尺寸要重新生成产物吗 | **不需要**（字节不变） | 同上 |
| 改尺寸要让客户端看见吗 | 要重发目录：`revision_of` 把 `machines`（含 `dimensions`）序列化进指纹 | `runtime/catalog.rs:842`、`:194/628` |
| 改尺寸影响后处理吗 | 影响：`machine_dims` 直接读 `presets/`（不经交付） | `postproc/machine_dims.rs:175-237` |

---

## 3. G-code → 后处理链

### 3.1 入口（谁真的发起）

| 入口 | 位置 | 说明 |
|---|---|---|
| **切片器钩子（生产唯一真跑）** | `main.rs:72` `Mode::Hook` → `hook::run_with_channel`（`hook.rs:1917`）→ `hook::run`（`:626`） | argv `--Toml <预设> --Gcode <gcode>`（`args.rs:112-152`） |
| GUI | `ipc/postprocess.rs:23/32/42/55` + `PostProcessModal.tsx` | **只看进度 / 停 / 答问句**，不发起 |
| CLI 诊断 | `crates/postprocess/src/main.rs:84-120` | `run / init / dump-ir / check`，`-c` 传 IR 配置 |

### 3.2 一次钩子执行的动作

```
argv 解析 → 读 G-code 全文 → 归档原始文件（gcode_history/日期/名_original.gcode，hook.rs:694）
  → preset::load_ir（crates/preset/src/lib.rs:176，调用点 hook.rs:708）   ← 预设 TOML → IR
  → postproc_prefs::apply_to_ir（hook.rs:751-754）                      ← 设置页偏好叠加
  → pipeline::process_with_ir_with_tower_ask（hook.rs:1069/1082）
  → 原地写回（.part + rename），另存三件套归档
```

### 3.3 pipeline 十二阶段（`crates/postprocess/src/pipeline/mod.rs`）

| # | 阶段 | 行 | 改什么 | 依赖 |
|---|---|---|---|---|
| 1 | input | 480-492 | 读全文 + sha256 | — |
| 2 | config | 494-510 | 造 IR（CLI 或调用方给） | `-c` / 预设 TOML |
| 3 | machine-detect | 512-517 | 填运动范围/涂胶区/禁区 + G-code 自报机型 | `presets/machines`、`forbidden_zones` |
| 4 | parse | 519-526 | 只读 G-code 头（`printer_settings_id` 等） | G-code 头 |
| 5 | support | 552-556 | 擦嘴用塔还是组件 | `ir.wiping.preferred_mode` |
| 6 | collision | 558-662 | 撞塔判定；Auto 档挪塔并复检 | `ir` + 挤出点 |
| 7 | calib-check | 664-688 | 校准件 BBox/质心 + dry-run | `calibration_models.toml` |
| 8 | pass1 | 690-711 | 全文件扫描收集（glue / wipe / metadata / iface / scan） | `ir.glue/tower/safety` |
| 9 | pass2 | 713-736 | 真正改写行（涂胶、删 wipe、塔几何/螺旋/鞘、圆盘） | `ir` + pass1 |
| 10 | calibration | 738-761 | 插入校准 G-code | `ir.prefs.*` |
| 11 | write | 763-793 | 起始回抽 / 跳振动 / 热床预涂胶 / `;MKP_INFO` 标记 + 原子写 | `ir.prefs.*` |
| 12 | printtime | 796-819 | 只填字段，不改 G-code | 成品文本 |

### 3.4 后处理的"配置"有四个源，且边界清楚

| 源 | 谁读 | 管什么 |
|---|---|---|
| 预设 TOML | `crates/preset`（唯一映射点，`lib.rs:1-33`） | 用户参数 → IR |
| IR TOML（`-c`） | `crates/postprocess/src/config.rs:1-33` | CLI 场景，结构 = IR |
| 设置页偏好 | `runtime/postproc_prefs.rs:50` → `Ir.prefs` | 退刀 / 跳振动 / 停留秒数 |
| 机型表 | `postproc/machine_dims.rs:239-256` | 几何 / 禁区 / 校准档（唯一来源是 `presets/`） |

**不读**：`public/bbs/*.json`（`crates/` 0 命中）、`bundles.toml` / `assets.toml`（`hook.rs` 0 命中）。

---

## 4. 两条链的连接判定

**有连接，但只有"一份预设 TOML 文件"这一条细线，且单向**：

1. **产物 → 钩子**：`post_process_dto` 把具体那份预设文件的绝对路径拼进 `--Toml`（`ipc/mod.rs:626-654`）；钩子 `preset::load_ir` 读它（`hook.rs:708`）。所以产物 TOML 的**字段名/段名/取值范围**必须与 `crates/preset` 的读取面一致，否则钩子当场失败。
2. **共享参数定义**：`presets/registry/param_registry.toml` 同时被工作台（运行时读，`registry.rs:791`）与 `mkp-preset`（`include_str!` 编进二进制，`lib.rs:79`）使用。
3. **共享机型几何**：`machine_dims::tables()`（`machine_dims.rs:175-237`）与工作台"有尺寸才允许生成"（`build.rs:601-608` 的 `has_dimensions`）出自同一棵 `presets/`。

**明确无连接**（查过，不是没找到）：

- 后处理不消费套餐/版本目录/发布结果；
- `hook.rs` 唯一的"目录级"接触是归档快照里的参数记录（`hook.rs:1338-1345/1356-1363`），仅供报告展示，不参与算法；
- 两条链跑在不同的机器与不同的根（生成在作者仓库 `ctx.real_root`，后处理在用户机器 `fsx::paths::*_headless`）。

---

## 5. 验收问题（这张图必须能回答）

1. 把某版本从套餐 A 改指到 B，改哪些文件？客户端看到什么变化？（§1.3/§1.5）
2. 改一份套餐的内容，产物 TOML 会不会变？为什么？（§1.5：不变，render 不读 bundle）
3. 在工作台改一个参数，到产物落盘经过谁、失败怎么回滚？（§2.3-§2.5）
4. 切片器那一趟 argv 到写回 G-code 经过哪些阶段、读哪些配置？（§3.2-§3.4）
5. 如果让后处理"按套餐选择配置"，最小改动点在哪？（§4：只能新增一条读取面，当前 0 连接）

---

## 6. 未核实

1. 客户端"下载并应用"把交付预设落到 `presets-local/` 的具体落点函数（只看了调用点，未逐行读）。
2. `machine_dims::install()` 在发布物中的实际接线（`src-tauri/` 内未搜到调用，`HANDOFF.md:259` 记为欠账）——发布物当前可能依赖 `MKPSE_PRESETS_DIR` 或编译期路径。
3. `postproc/pass1|pass2|tower|disk` 的内部算法只按模块归类，阶段表是行为摘要，不是逐行结论。
4. 客户端下载的 BBS 文件落盘后由谁消费（`HOME-BUNDLE-DOWNLOAD.md:3-4` 说"先只落盘"），未逐行核。
5. `wb_assign_bundle_versions` 的回执只回 `BundleList`，前端拿不到"哪几条真变了"（`app/bundles.rs:422`）——是否需要补未裁决。
