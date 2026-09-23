# B04 Source → Generator → Consumer 数据契约

状态：**Draft 1.0 —— P0 审计核实过的部分已冻结**，其余仍标「待定」。
证据来源：[AUDIT-EVIDENCE.md](./AUDIT-EVIDENCE.md)（两仓 commit 固定：
MKPSupportEase `8bf3fcb` / mkp-ssr `a2a47ca`，读的是本机工作副本）。
关联计划：[REBUILD-PLAN.md](./REBUILD-PLAN.md) · [INTEGRATION-PLAN.md](./INTEGRATION-PLAN.md)

> 0.1 版里那句「凡标记待核实的不得当成已实现事实」继续有效。
> 这一版的改动是：**P0 查明的那些从「待核实」变成「已核实并冻结」，并注明代码里落地了没有。**
> 没查明的仍然明确标着「待定」—— 不把审计没碰到的东西写成协议。

## 1. 目标与非目标

目标：唯一权威源、确定性生成、消费端显式校验、错误可见、版本可协商、可复现验证。

非目标：让客户端直接编辑源 TOML；让 UI 自己解析/写文件；保留两份可编辑真相；
用默认值或 fallback 掩盖契约不匹配。

> 0.1 版的非目标里有一条「在未确认实际消费路径前强行规定最终文件扩展名」——
> **消费路径已经确认了**（§5），所以这一条完成了它的使命，从非目标里去掉。

## 2. 数据角色（不得混用）

| 角色 | 责任 | 是否权威 | 持有者 | P0 核实 |
|---|---|---:|---|---|
| Source TOML | 人工维护的机型、参数定义与值 | **是** | `MKPSupportEase/presets/` | ✅ 12 个文件 |
| Domain IR | 解析、校验、继承合并后的语义模型 | 否 | **消费端进程内，不落盘** | ✅ `mkp_pp::ir::Ir` |
| Intermediate JSON | **不存在于真实链路** | — | — | ✅ 见下 |
| Deliverable | 消费端实际读取的发布文件 | 否（可再生） | `dist-presets/` | ✅ 是 **TOML** |
| UI Draft | 尚未保存的编辑态 | 否 | `workbench/.draft/` | ✅ |

**「中间 JSON」这一环可以划掉。** P0 查明：消费端 `load_ir()` 直接读 **TOML 预设**
（`mkp-ssr/crates/preset/src/lib.rs:151-212`），映射成内存里的 `mkp_pp::ir::Ir`，
中途不落任何 JSON。历史上那份 `content/*.json` 是旧客户端的构建产物，
两个旧应用都已退役 —— 它不在新链路里。

## 3. 权威源与写入规则

1. 正式配置仅以 `MKPSupportEase/presets/` 下的 Source TOML 为权威源。
2. `param_registry.toml` 的参数定义/范围/机型适用性保持 SSOT；**不得在前端或消费端复制维护**。
   ⚠️ **这一条现在是被违反的**：消费端 `crates/preset/assets/param_registry.toml`
   是一份编译嵌入的副本，而且**已经与我们这份分岔**（74 条 key 相同，
   4 处实质差异，见 AUDIT-EVIDENCE §1）。处置：改由我们这边生成/同步过去。
3. 所有修改经统一 IPC → application/domain → source writer；前端不得直接读写配置文件。
4. 参数保存必须对指定 machine + variant 执行 set/clear 语义；**不得重写其他 variant 的有效值**。
   ✅ 已落地：`ParamRegistry::set_variant` / `clear_variant`，键形状
   `A1`（整台机型）与 `A1:FAST`（某一版）。
5. Writer 必须原子写入；保留无关注释、未知字段与未修改值的格式。
   ✅ 已落地：`toml_edit` 保原文 + `one_edit_only` 判据（只有被改的那一处变）。
   写回口径按真数据实测：纯机型键裸写、带冒号的键单引号、`float` 保 `.0`、
   `int` 不写成浮点、多行 G-code 退回双引号 + `\n`。
6. 删除参数表示显式 clear/继承语义，**不得替换为零、空串或默认值**。
   ✅ 已落地：清空 = 删键；删到那张表为空时连 `[params.machineVariants]` 表头一起删。
   ⚠️ 已知不一致：源数据里 A2L 写着 `presetFile = ''`（空串），我们读成「没填」
   但写的时候删键 —— 文件里会同时存在两种写法。语义一致，**刻意不洗**
   （洗一遍会让 12 个文件全变，diff 就失去意义）。
7. UI Draft 与正式 Source 分离；不得悄悄提交正式源。

## 4. 生成协议

`Source TOML → Parse → Validate → Resolve/Derive → Domain IR → Generate → Deliverable + Manifest`

- 相同 Source、相同 generator、相同选项 → 语义等价的产物。
  ✅ 已落地：产物头部的 `uuid` 按**指纹**算而不是随机数，否则「字节没变不重写」永远不生效。
- 生成器只读 Source；输出写入独立目标目录（`dist-presets/`），禁止覆盖输入。✅
- 必需源缺失、语法错误、引用悬空、重复 ID、类型/范围不合法 → 生成失败并定位。
  ✅ 部分落地：阻断只有三种来法（显示条件成环 / 值越界 / 枚举值不在选项里）。
- 未知字段默认报错或进入显式兼容策略；禁止静默丢弃。
  ✅ 已落地判据：`every_field_in_the_real_file_is_either_read_or_listed`
  （真数据里每个字段要么读进来，要么在清单里写明为什么不读）。
- 发布采用 staging → 校验 → 原子替换。✅ 原子写已落地；**发布前的一致性校验缺失**（§9 缺陷①）。
- 生成 diff 必须区分新增、删除、变更；删除项必须可见。⏸ 待做。
- **占位机型不参与交付**：机型文件里没有 `[dimensions]` 的机型（当前是 A2L）
  不生成、不进清单。✅ 已落地，且这是「提示」不是「待办」——
  它是刻意占位，不是缺数据（§6.3）。

## 5. Deliverable 契约（**已核实，冻结**）

交付物是 **TOML**，不是 JSON。消费端 `load_ir(preset_path, gcode_text)` 直接读它。

### 5.1 文件名：`{机型}-{版本小写}.toml`

例：`A1-standard.toml`、`A1_MINI-fastv3.3.toml`、`P1S-lite.toml`。

依据：消费端内置的 9 份预设全是这个形状（`crates/preset/src/lib.rs:76-112`），
而它按这个名字找文件。**名字对不上的后果不是报错，是它找不到。**

✅ 已落地：`app/build.rs` 的 `preset_file_name()`，生成与发布共用一处；
判据 `generated_names_match_what_the_consumer_looks_for` 把我们清单算出来的 10 个名字
与消费端实测的 9 份 + 占位的 A2L 逐个比。

这条同时**断掉了对上游 manifest `file_name` 的依赖** —— 名字只由机型 id + 版本 id 决定。

### 5.2 文件形状

```toml
# uuid: f444aeaf-d447-453f-8871-cf7645c2d5df   ← 按内容指纹算，不是随机
# release_time: 2026-08-19 01:38:13            ← 空格分隔、无时区、UTC
# machine: A1                                  ← 规范机型 id（大写）
# variant: standard                            ← 版本 id 小写
#胶笔配置

[toolhead]
offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移
speed_limit = 70 # 涂胶速度上限 (mm/s)
# 自定义工具头抓取 G-code
custom_mount_gcode = """
G92 E0
...
"""
```

逐项冻结：

| 项 | 规则 | 落地 |
|---|---|---|
| 头部五行 | 顺序固定：uuid / release_time / machine / variant / `#胶笔配置` | ✅ |
| `release_time` | `%Y-%m-%d %H:%M:%S`，UTC，**不带 T/Z** | ✅ `clock::now_release_time()` |
| 段名与段序 | 段名 = 参数的 `section`；段序 = 段内最小 `layout.order` | ✅ |
| 键名 | `tomlKey` | ✅ |
| 内联表 | 多个参数共享同一个 `tomlKey` 时，成员名取 `jsonKey` | ✅ |
| 数值 | 整数不写小数点；其余按 f64 原样 | ✅ |
| 多行 G-code | 三引号，注释写在上一行 | ✅ |
| 行内注释 | `tomlComment`；空注释不写井号 | ✅ |
| 写哪些键 | `visible_keys(machine)`：非 deprecated 且过 `machineFilter` | ✅ |

### 5.3 `contract_version`：**目前不存在，待定**

两边都没有这个字段：产物头部只有 uuid/时间/机型/变体，
我们的 `manifest.json` 有 `manifestVersion: 2`（那是分发清单的版本，不是数据契约的）。

这不是遗漏，是**现状**。要不要加、加在哪（产物头部 vs manifest）、
消费端怎么用它拒绝，都留给 P1 的 ADR。**在加之前不许声称"版本可协商"已经成立。**

## 6. 消费端读取协议（**已核实**）

### 6.1 真实入口与调用链

```
预设 TOML
  → mkp_preset::load_ir(path, gcode)        crates/preset/src/lib.rs:151
     ① read_preset                          :159
     ② normalize_to_canonical（别名归一）    :163
     ③ validate_config                      :165
     ④ load_param_registry（嵌入的副本）      :167
     ⑤ validate_against_registry            :169-174
     ⑥ 空机型拦截                            :176-180
     ⑦ build → Ir                           :182
     ⑧ 机型归一 + 尺寸表命中                  :189-208
     ⑨ fill_machine_facts                   :210
  → mkp_pp::ir::Ir（内存，不落盘）
```

产品代码**只有三个调用点**：`hook.rs:253`（切片器钩子，唯一喂 G-code 的）、
`ir_view.rs:95`（IR 只读视图）、`preset_apply.rs:210`（**写回后的复检**）。
IPC 三条都隔了一层：`preset_ir` / `preset_apply` / `preset_edit_or_fork`
（`commands.rs:461` / `498` / `514`）。

### 6.2 校验语义（两条要记住的豁免）

- **`0` 被当成「未设置」跳过量程校验**（`validate.rs:299-301`）。
  而我们这边把 `0` 当成一个真实的值（`0` 与"没这一行"是两件事）。
  两套语义不冲突（它只是少校验一项），但**不要以为消费端会拦住 0**。
- 没有量程的参数不校验（`validate.rs:290-292`）。
- 量程支持 `机型:变体` 覆盖优先（`registry.rs:205-229` / `259-269`）。

拒绝时统一 `PostprocError::InvalidConfig`，两种话术：
「不在允许的选项里」/「超出区间」。

### 6.3 机型与尺寸

- 机型别名 23 条，大写 → 规范 id；**查不到返回空串**（`machine_dims.rs:148-153`）。
- **尺寸表里没有的机型直接被拒**：`load_ir` 第 8 步返回 `MissingMachine`
  （`lib.rs:199-208`）。别名表有 A2L、尺寸表没有 —— 所以 **A2L 就算给它预设也读不进去**。
  我们这边因此把它当占位机型处理（不生成、不进清单、一条提示收起来）。
- 禁区未命中**只 warn 不阻塞**（`machine_dims.rs:157-170`）——
  禁区丢了不报错，只是碰撞检测少一块。

### 6.4 三类数据两边已经等值

尺寸 125/125 字段全等、别名 23 条双向全覆盖、禁区点集逐点相同（AUDIT-EVIDENCE §6）。
所以这三类的权威源定在我们这边**不需要任何数据修正**，只需要建生成入口。

### 6.5 其余原则（沿用 0.1）

只从约定入口读；应用前校验；不静默降级到 mock/旧缓存/内置猜测；
读取失败保留既有状态；**不回写或"修复"发布文件**；缓存必须可丢弃且绑定 hash。

## 7. 错误模型

错误至少含稳定 `code`、`stage`、人类可读 `message`；可定位时附
`file` / `path` / `machine_id` / `variant_id`。

阶段码建议：`parse` / `validate` / `resolve` / `generate` / `publish` / `consume`。
⏸ 待定：消费端现在用的是 `PostprocError`（`InvalidConfig` / `MissingMachine` …），
我们这边是 `AppError`（`NotFound` / `InvalidArgument` / `Corrupted` …）。
两套怎么统一留给 P3 迁入时决定 —— **现在不硬造一层映射**。

## 8. 兼容策略

沿用 0.1：同 major 内可加可选字段；major 不兼容必须显式支持，否则拒绝加载；
删除/改名/改类型/改单位/改继承语义都算破坏性；不做隐式迁移，不在读取时改写用户文件。

⚠️ 前提缺失：`contract_version` 目前不存在（§5.3），所以这一节现在是**预案**，
还不是可执行的策略。

## 9. 已知缺陷（我们这边，都有行号，都还没修）

1. **发布出去的 manifest 里 `bundles` 是悬空引用**：`build.rs:588-599` 把上游
   `bundles` 原样抄进去，而我们自己算的 `assets` 只含 `mkp_preset`，
   一条 `bbs_profile` 都没有。上游那份有一致性检查拦这个
   （`upstream/manifest.rs:462-476`），**我们输出的这份没过任何检查**。
2. **没有上游产物资源的版本永远进不了清单**：`build.rs:560-562` 静默 `continue`。
3. **`channel` / `minimumClient` / `version` 直接透传上游** —— 上游一删就没来源。
4. `paths::resolve_dist` 零调用点，死代码。

## 10. P0 决策：关闭情况

| 决策 | 状态 | 答案 |
|---|---|---|
| 真实消费端仓库、入口与调用链 | ✅ 关闭 | `mkp-ssr`，`load_ir()` + 三个调用点 + 三条 IPC（§6.1） |
| 交付物是 TOML 还是 JSON | ✅ 关闭 | **TOML**。JSON 只剩分发清单 `manifest.json` 这一个角色 |
| 中间 JSON 是否真实链路 | ✅ 关闭 | **不是**。IR 在内存，不落盘（§2） |
| machine/variant ID 规范与映射 | ✅ 关闭 | 规范 id 大写 + 23 条别名归一；变体在产物里小写（§5.1、§6.3） |
| 单位、浮点精度、缺省/继承/删除语义 | ✅ 大部分关闭 | TOML 保 `.0`；`0` 在消费端算"未设置"；删除 = 删键（§3.6、§6.2） |
| Manifest 的消费者、发布路径、原子发布 | ⚠️ 部分 | 路径与原子写已定；**消费者是谁还没查**（客户端怎么用这份清单） |
| `contract_version` 当前值与兼容窗口 | ❌ 未定 | 两边都没有这个字段（§5.3） |
| 是否同仓库/同发布节奏、如何共享 fixture | ⚠️ 部分 | 现在两仓；终点是合并进 MKPSupportEase。共享 fixture 方案待 P1 |

## 11. 双方契约测试（最低集）

同一组 fixture 同时供 generator 与 consumer 使用。前四条已经有对应判据：

- ✅ 产物名与消费端认的那批一致（`generated_names_match_what_the_consumer_looks_for`）
- ✅ `release_time` 格式与消费端一致（`release_time_matches_what_the_consumer_writes`）
- ✅ 改一个值只动一处，别的键不变（`writing_one_variant_touches_nothing_else`）
- ✅ 清空 = 删键，同表别的键不少（`clearing_a_variant_removes_the_key_...`）
- ✅ 多行 G-code 逐字往返（`a_multiline_gcode_variant_round_trips_verbatim`）
- ✅ 被拒之后文件一字未动（`a_refused_write_leaves_the_file_untouched`）
- ⏸ **真实预设经 `load_ir()` 读通**（P2 最小纵向切片，还没做）
- ⏸ 删除某个 variant 的参数后，其他 variant 的输出不变（要逐版本比有效值）
- ⏸ 连续生成产物与 manifest hash 稳定
- ⏸ 输出损坏/截断时消费端不应用部分状态
- ⏸ 不支持的 contract major version 时消费端明确拒绝（先得有这个字段）

## 12. 这份契约现在能声称什么

**能**：交付物格式、文件名、时间格式、机型/变体口径、写入与删除语义、
消费端的入口与校验语义 —— 这些都有代码行号或实测数据支撑，而且大部分已经落地。

**不能**：版本协商（没有 `contract_version`）、端到端互通
（`load_ir()` 还没真读过我们生成的产物 —— 那是 P2）、错误码统一（两套错误类型还没对齐）。
