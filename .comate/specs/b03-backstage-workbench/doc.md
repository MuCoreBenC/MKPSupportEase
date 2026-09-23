# B03 后厨工作台（第三版 · 照参考实现重做）

> **这一版推翻了前两版。** 前两版把工作台做成"一串各自写盘的 Tauri 命令 + 一套朴素表单"，正确但没法长大：撤销、脏计数、差异列表、状态一致性，每一件都要改十几处才能补上。
>
> 这一版照 `G:\project\htmldemo\homedemo1\src\versions\b03\` 的模型重排，但**不照搬它的纯前端存储架构**：业务规则、真实数据、文件读写、状态派生全部归 Rust。
>
> 五项决议（你定的）：
>
> | # | 决议 |
> |---|---|
> | 1 | **前后端一起重排，Rust 保留完整业务职责**，后端不退化成文件读写器 |
> | 2 | 出厂默认先用**全局 `defaultValue`**，模型上给机型级默认留口子 |
> | 3 | 字段定义从 **`param_registry` 导入真实结构**，它继续当 SSOT，前端不留副本 |
> | 4 | **草稿由 Rust 落盘**，撤销栈前端会话内维护，**所有修改统一走同一条 IPC** |
> | 5 | 文案沿用参考的**说法纪律**，句子自己重写 |

---

## 1. 分层与铁律

```
前端 UI（页面 / 矩阵 / 选中 / 勾选）
      ↓  一次手势 → 一组 patches
前端交互模型（撤销栈 · 会话内存）
      ↓  唯一写入口
统一 IPC 契约
      ↓
Rust Application 层（IPC 入口 / 请求协调 / 返回 DTO）
      ↓
Domain / Service 层（三层解析 · patch 应用 · 状态派生 · 校验 · 影响分析）
      ↓
Repository 层（仓库数据 · 草稿 · 生成快照）
      ↓
生成服务 / 发布服务（TOML · 原子替换 · 出货检查 · 发布）
```

四条铁律，后面每一节都受它约束：

1. **前端不直接读写任何业务文件。** 它连路径都不知道。
2. **前端不实现任何业务规则。** 三层怎么合、状态怎么算、哪条校验不过，一律问后端要结果。
3. **所有修改只有一条写入路径**：`wb_apply_draft(label, patches)`。点一格、整列改、批量改、克隆、移动、归档、生成记录 —— 全部先变成 patches。撤销与重做也走这一条（提交反向 patches），不另开命令。
4. **状态只有一份，由 Rust 派生。** 不允许前端、快照文件、Rust 各存一套"当前状态"。

---

## 2. 数据源：全部接真数据，不再自己造

`G:\project\mkpse-next-v3\mkpse-presets\` 是唯一上游。以下数字全部实测（JSON 解析计数，不是估的）：

| 用途 | 文件 | 实测内容 |
|---|---|---|
| **字段定义 SSOT** | `content/param_registry.json` | `{ params: 74, tabs: 8, updated: "2026-07-05 16:46:54" }`，**无版本字段** |
| 参数摆放 + 分组级条件 | `content/layout_schema.json` | 8 tab / 27 section / 74 item；**只有它有 section 级 `showWhen` 与 `component`** |
| 机型与版本 | `content/machine_catalog.json` | `brands` 1、`models["Bambu Lab"]` = **6 机型**、`dimensions` **5 个（缺 A2L）**、`forbiddenZones` **3 个（P1S/P2S/X1C）**；共 **10 个版本** |
| BBS 索引 | `content/preset_registry.json` | `entries` **9 条**；与 `manifest.assets` 有 8 个字段重叠，**只有 `nozzle` / `layerHeight` 是它独有的** |
| ~~资产索引~~ | ~~`content/assets_index.json`~~ | `assets` **18 条**；**`sha256` 全是空串、`size` 全是 0、`isRegistered` 全是 false（18/18），且没有 `resourceType`**。这 18 条的 `relativePath` **全部**已在 `manifest.assets` 里登记 → **决定不读**（见 2.3） |
| 真哈希与大小 | `manifest.json` → `assets[]` | **72 条 = 9 MKP + 9 BBS + 54 素材**；`sha256` 72/72 非空，`id` 72/72 不重复。**资源这一面的唯一权威** |
| ~~套餐~~ | ~~`content/bundles.json`~~ | 顶层数组 5 条，与 `manifest.bundles` **逐字节相同**，且**不在 `contentFiles` 里**（客户端不会下载它）→ **决定不读**，套餐从 manifest 取 |
| 回退登记表 | `content/fallback_registry.json` | `{version, updated, guide, fallbacks: 20 条}`；规则 9 字段 `id/category/trigger/from/to/enabled/severity/desc/reportField` |
| 分发清单 | `manifest.json` | `manifestVersion: 2`、`contentFiles` 15（每条带真 sha256/size/ttl/strategy）、`machines{}` 6（版本带 `machineKey` 与 `mkpPresetAssetId`）、`minimumClient` **空串** |
| 真实产物 | `presets/mkp/*.toml` | **9 个**（4297–4366 B） |
| 真实 BBS | `presets/bbs/Process/0.2mm/` 4 + `0.4mm/` 5 | **共 9 个真文件** —— 上一版 doc 写"这个目录是空的"是**错的**，我当时没下钻 |

### 2.1 字段定义的真实形状（照它建 Rust 结构）

`params[]` 每条的字段与实测出现次数：

| 字段 | 条数 | 说明 |
|---|---|---|
| `key` `configKey` `tomlKey` `jsonKey` `label` `desc` `tomlComment` | 74 | 四种键名各有用途，不可互相替代 |
| `valueType` | 74 | `float` 43 / `string` 15 / `bool` 12 / `int` 4 |
| `uiComponent` | 74 | `number` 47 / `switch` 12 / `segmented` 11 / `gcode` 2 / `select` 2 |
| `defaultValue` | **74/74** | **单值**（int 42 / str 15 / bool 12 / float 5）→ 决议 2 的落点 |
| `scope` | 74 | `universal` 64 / `machine_specific` 10 |
| `section` | 74 | `wiping` 64 / `toolhead` 10（= key 前缀，数据域，**不是界面分组**） |
| `layout` | 74 | 恒为 `{ order: number, sectionId: string }` ← 界面分组在这 |
| `min` / `max` | 各 47 | |
| `step` | 47 | |
| `unit` | 42 | `mm` 28 / `mm/s` 6 / `层` 2 / `°` 2 / `%` 1 / `次` 1 / `秒` 1 / `0-255` 1 |
| `parentKey` | 43 | 父参数（层级折叠） |
| `showWhen` | 43 | `{key, op, value}`，`op`：`eq` 38 / `neq` 3 / `gt` 2 |
| `choices` | 19 | `[{label, value}]`，**个别选项自带 `deprecated: true`** |
| `variantMode` | 10 | `per_variant` 5 / `shared` 5 |
| `machineFilter` | 8 | **逗号分隔字符串**，不是数组 |
| `deprecated` | 7 | 全 true，无 false 占位 |
| `machineVariants` | 5 | 按机型/版本的默认值覆盖 |
| `machineMinVariants` / `machineMaxVariants` | 各 5 | 按机型/版本的范围覆盖 |
| `mergeGroup` | 5 | `offset` 3 / `wiper` 2 |
| `pinned` | 5 | |
| `serialization` | 1 | 仅 `toolhead.offset.x`：`{subfieldsOrder: ["x","y","z"]}` |
| `mergeable` / `selectable` | 74 | **实测全为 true，当前零区分度** —— 读进来但不给它们任何行为，否则是假的 |

三处必须写成启动断言（上游没有这道门禁）：

1. `params[].key` 与 `layout_schema` 的 `paramKey` **严格双射**（实测双向差集都是空）
2. `params[].layout.sectionId` 的计数与 `layout_schema` 各 section 的 `items[]` 长度**逐个相等**（实测 16/16 相同）
3. `layout_schema` 各 section 的 `items[]` 顺序等于按 `layout.order` 升序 —— 实测 16/16 一致；**渲染以 `layout.order` 为准**（`item.id` 末位数字不能当顺序：`space_offset` 的实际序列是 `item_0_0_1`(offset.x) → `item_0_0_0`(offset.y) → `item_0_0_2`(offset.z)）

### 2.2 分类页签：8 tab / 27 section，但只有 16 个装参数

中文名与顺序的唯一权威是 `param_registry.tabs[]`（`layout_schema` 全文没有 label / order / icon）。

| tab | 中文名 | order | icon | 装参数的 section（数量） |
|---|---|---|---|---|
| `offset` | 偏移 | 10 | wrench | 空间偏移 3 · 运动与挤出 2 |
| `wiping` | 擦料 | 20 | layers | 擦料方式 1 · 擦料塔位置与打印 4 · 安全与高级参数 7 · 塔结构加强 11 · 圆盘动作控制 4 |
| `fan` | 风扇 | 30 | fan | 温控与风扇 3 |
| `glue` | 涂胶 | 40 | paintbrush | 高级运动调控 1 · 支撑面熨烫 8 · 涂胶方向补偿 10 |
| `gcode` | 切换 | 50 | terminal | 自定义G-code 2 |
| `advanced` | 更多 | 60 | sliders | 润笔 7 · 涂胶参数 3 · Z轴抬升 3 · 机型专属 5 |
| `internal` | 内部参数 | 70 | settings | **无 section** |
| `settings` | 软件设置 | 80 | 无 icon | **11 个空 section**（都是 `component` 占位，属于客户端设置页，不是配方） |

**矩阵只渲染前 6 个 tab 的 16 个 section。** 照 27 个建行会多出 12 个永远为空的分类 —— 这是我们要主动避开的坑。

三个 order 的量级不统一（tab 用 10/20/…；`offset` 与 `settings` 的 section 用 100/200…；`wiping`/`fan`/`glue`/`advanced` 的 section 用 0/1/2/3/4），**只能在同一 tab 内比较，不能全局排序**。

### 2.3 一个文件只有一个所有者；两个文件刻意不读

上游有四个文件看起来都在讲资源。实测只有 `manifest.json` 能用，另外两个不读，第四个只取它独有的两列：

| 文件 | 条数 | 有真 sha256 吗 | 结论 |
|---|---|---|---|
| `manifest.json → assets[]` | 72 | **有**（72/72） | 权威 |
| `content/assets_index.json` | 18 | 没有（全空 / size 全 0） | **不读** |
| `content/bundles.json` | 5 | —— | **不读**（与 `manifest.bundles` 逐字节相同，且不在 `contentFiles` 里） |
| `content/preset_registry.json` | 9 | 没有 | 读，但只取 `nozzle` / `layerHeight` |

**不读 `assets_index.json` 的代价是具体的，反过来读它的代价才大**：它的 18 条 `isRegistered` 全是 false，而这 18 个路径全部已在 `manifest.assets` 里登记。读它 = 文件清单页给已登记的文件打「未登记」、给有真哈希的文件显示「大小未知」。参考实现的库存页满屏 `UNKNOWN` 就是因为它只有这一个来源；我们有 manifest，所以**文件清单页不该出现一个 UNKNOWN**。

#### 机型这一面必须读两个文件 —— 这不是双轨，是两份各缺一半

| 只有 `content/machine_catalog.json` 有 | 只有 `manifest.json → machines` 有 |
|---|---|
| `forbiddenZones`（P1S/P2S/X1C 各 2 块，无 `name` 字段） | `machineKey`（`"A1:FASTV3.3"`）—— §3.3 的版本键就是这个形状 |
| `externalAliases`（X1C 有 5 个：`X1`/`X1S`/`X1E`/`X1 Carbon`/`X1CARBON`） | `mkpPresetAssetId` —— 产物的**稳定 id**，不是会变的文件名 |
| `versions[].presetFile`（文件名） | `brand` |

少了左边，禁区画不出来、客户端传来的 `X1 Carbon` 认不出是哪台机型；少了右边，`machineVariants` 的版本键对不上，产物只能靠文件名去认。

**尺寸只认 `machine_catalog` 这一份**（manifest 内联了一份同样的，两处都建类型就是床身尺寸有两个声明），但要对一次**在不在** —— 客户端读的是 manifest 那一份，如果两边一个有一个没有，客户端显示「暂无尺寸」而工作台显示有值，**两边都不会报错**。

#### 上游没有的门禁，都补在这一层

它们不成立时的后果没有一条会自己报错：

| 断言 | 不成立时 |
|---|---|
| `params[].key` ↔ 布局 `paramKey` 双射 | 矩阵静默少行，或布局引用一个不存在的参数 |
| 每 section 两侧计数相等 | 多出永远为空的分类 |
| items 顺序 == `layout.order` 升序 | X 和 Y 调个头，**界面上看不出来** |
| 资源 id 不重复 / 套餐 `assetRefs` 不悬空 | 客户端下载时才 404，错误出现在用户机器上 |
| 交付物 `sha256` 非空 | 状态派生永远判「待生成」 |
| 别名不与别名、不与正名撞车 | 客户端传这个别名过来，匹配到哪台取决于遍历顺序 |
| 有禁区必须有尺寸 | 禁区是床面坐标，没床面画不出来也判不了越界 |
| BBS 归属机型两处一致 | 这台机器下载到别的机器的曲线 |
| `machineVariants` 的键是真机型或真 `机型:版本` | 三步归并时那个值**凭空消失**，界面上只看得到「这一项用的是出厂默认」 |

最后一条跨文件，所以在 `Upstream` 聚合里做 —— 分四次各读一次的话，没有任何时刻能把它们放在一起查。

**单测按不变式而不是条数对齐**：条数是最易腐烂的判据，上游一更新就产出一条与代码无关的红，而它抓不到任何真实缺陷。真上游那几条测试另有一条「上游有没有我没读的字段」的键集比对 —— 上游加字段时才响，那时候确实需要人看一眼。


---

## 3. 三层取值模型 —— 附一处必须先验证的前提

### 3.1 目标形态

```
出厂默认  →  机型基底  →  版本覆盖
（只读）     （可编辑）    （可编辑，只写差值）
```

取值规则与参考一致：版本有 → 用版本，否则机型有 → 用机型，否则出厂，第一个命中即返回并带回来源层。三层都没有 → 返回"不存在"，**与"值是空串"分得开**（有两个 G-code 字段的出厂默认就是空串）。

### 3.2 出厂默认这一层（决议 2）

第一版取 `params[].defaultValue`，**全局单值**（74/74 都有）。

**注意这一层与 `machineVariants` 无关** —— 见 §3.3 的验证结论：那三张 `machine*Variants` 表里，**值**那张（`machineVariants`）根本不属于出厂层，它被消化进机型基底与版本覆盖；只有**范围**那两张（`machineMinVariants` / `machineMaxVariants`，各 5 条）是字段定义的机型特化，决议 2 说的「机型级默认留口子」指的正是它们。第一版校验统一用全局 `min`/`max`，结构上给这两张表留读取位但不接进校验链。

### 3.3 ✅ 验证结论（Task 1，已实测，替换原来的三个「可能」）

**可能 (a) 排除。** `source/machines/*.toml` 里**一个参数覆盖块都没有**——实测 `P1S.toml` 全文只有身份字段（id/name/display/brand/defaultBundle/externalAliases/image/icon）+ `[dimensions]` + `[[versions]]`；`A2L.toml` 只有身份字段 + `[[versions]]`。机型实体 TOML 不含任何参数值。

**真实来源 = 加载时从 `param.machineVariants` 构造**（参考实现 `src/server/resolve/recipe.ts:93-118`），三步：

```
对每台机型，按 visibleKeys(机型)（已按 machineFilter + deprecated 过滤）遍历：
  1. machineVariants 里的**纯机型键**（如 "P1S"）        → 直接进机型基底
  2. **版本键**（如 "A1:FASTV3.3"）：
       这台机型所有版本都写了、且值完全相同  → **上提到机型基底**，版本上不留覆盖
       否则                                → 留在各版本的覆盖里
  3. 没有 machineVariants 的字段（69/74）                → 全靠继承出厂默认
```

**这个归并是无损的**：对任意 `(机型, 版本, key)`，归并前后有效值完全一致，只是记在哪一层不同。刻意**不做**「取多数值当基底、少数派留覆盖」那种更激进的压缩——那要替人挑一个「标准版本」当基准，是凭空造的判断。**只在全体一致时才上提。**

**口径已独立重算并逐位对上**（我用 Python 照上述算法重跑真数据，不是读注释）：

| 机型 | 基底项 | 覆盖项 | 基底里是哪几个 |
|---|---|---|---|
| A1 | **1** | 9 | `custom_mount_gcode` |
| A1_MINI | **2** | 6 | `custom_mount_gcode` `custom_unmount_gcode` |
| A2L | **0** | 0 | （空） |
| P1S | **5** | 0 | 三个 offset + 两个 gcode |
| P2S | **5** | 0 | 同上 |
| X1C | **5** | 0 | 同上 |
| **合计** | **18** | **15** | |

`machineVariants` 的键分布实测：5 个字段有这张表（`toolhead.custom_mount_gcode` / `custom_unmount_gcode` / `offset.x` / `offset.y` / `offset.z`），前四个各 9 个**版本键**、零纯机型键；只有 `offset.z` 有 3 个**纯机型键**（P1S/P2S/X1C）+ 3 个版本键。所以「纯机型键只有 3 个实例」是对的，但**机型基底那 18 项的绝大多数来自第 2 步的上提**，不是来自纯机型键——这是我上一版没想到的那一环。

### 3.4 ✅ 已定：选 A（`machineVariants` 消化进两层）

上一版 doc §3.2 写的「`machineVariants` 第一版不并进取值链，只在字段详情里展示」**是错的**，后果很硬：

不并进去 ⇒ 那 5 个字段全部退回全局 `defaultValue` ⇒ **6 台机型的喷嘴偏移与装卸胶箱 G-code 会变成同一个值**。而 `machineVariants` 恰恰是唯一能回答「这台机器的驻点在哪、挂载指令怎么写」的数据源（A1 的 `offset_x=256` 与 P1S 的 `20` 就是从它来的）。

**定案：照 §3.3 的三步算法，在首次加载时把 `machineVariants` 消化进机型基底与版本覆盖两层。** 与决议 2 不冲突——决议 2 管的是出厂层，而 `machineVariants` 不属于出厂层；真正属于「机型级默认留口子」的是 `machineMinVariants` / `machineMaxVariants` 那两张**范围**表（各 5 条），第一版校验仍用全局 `min`/`max`，结构上留读取位。

### 3.5 上游数据的访问纪律

`G:\project\mkpse-next-v3\mkpse-presets\` **只读这一个文件夹**，它的父仓库与 `homedemo1` 一律不碰（父仓库根上有一份 `AGENTS.md`，任何落在其子目录的读文件操作都会把那份治理文档整篇拉进上下文）。

工程上的后果有两条，都要照做：

1. **人工查看上游数据时走命令行解析**（`python -c "import json…"`），不用读文件/搜索工具 —— 实测前者不触发注入、后者会。
2. **Rust 运行时读它没有这个问题**（那是程序行为，不是我的工具调用），所以 `upstream/` 层照常实现。



### 3.6 归并结果**不落盘**：它是机型层/版本层里「上游给的那一半」

§3.3 的归并结果放哪里，和 §7 那条「首次启动一个配方都不写」直接撞上。三种走法，选第三种：

| 走法 | 问题 |
|---|---|
| 首次加载写进 `machines/*.json` | 违反「不写 seed 配方」；而且上游以后改了 `machineVariants`，已有仓库永远收不到 |
| 只在我们的文件不存在时用归并结果 | 用户一改某台机型的任何一项，就必须把归并结果整本materialize进文件，否则剩下的值全丢。materialize 的时机很难说清 |
| **归并结果是派生的，永不落盘** | 需要把「一层」拆成「上游给的 + 我们写的」两半 |

所以实际取值是五级，而**界面上仍然是三档来源**：

```
版本层  我们写的版本覆盖   ← versions/{版本}.json
        上游的版本差异     ← machineVariants 归并（派生）      ┐ 都算「版本」（蓝）
机型层  我们写的机型基底   ← machines/{机型}.json              │
        上游的机型差异     ← machineVariants 归并（派生）      ┘ 都算「机型」（紫）
出厂层  defaultValue                                           出厂（灰）
```

三处由此确定的语义，都要照这个来：

1. **「N 项自有」与「挂回继承」只看我们写的那两张表。** 「挂回继承」= 删掉我们的那个键，露出上游的机型差异（而不是掉到出厂默认）—— 这正是想要的：把 A1 的 X 轴偏移挂回继承，应该回到 `-1`，不该回到 `0`。
2. **我们的文件只会「多一个键」，不会「删一个键」。** 所以文件里不需要删除标记，`Overrides` 就是一张普通的 map。
3. 代价说清：**没法把某一项压回出厂默认**。上游给 A1 的 `offset.x` 是 `-1`，想让它变成全局默认的 `0`，只能显式写 `0`，不能靠「挂回继承」。这个需求很少，而为它引入删除标记会让每一层都多一种状态。

好处是白拿的：上游改了 `machineVariants`，**我们没动过的项自动跟着变** —— 与 §4.3「草稿只存差异」是同一条理由。



工作台自己编辑的两层落在我们仓库里，**不回写 mkpse-presets**（那是上游，只读）：

```
workbench/machines/{机型}.json            机型基底
workbench/machines/{机型}/versions/{版本}.json   版本覆盖
```

上游 `machine_catalog.json` 给的是**骨架**（有哪些机型、哪些版本、版本的中文名、`presetFile`、尺寸、禁区），工作台不改它；参数值才是我们的。

### 3.5 版本身份：稳定 uid

上游 `versions[].id` **跨机型重名**（`STANDARD` 在 A1 / A1_MINI / A2L / P2S 上都有，`FAST` 在 A1 / A1_MINI 上都有，去重后只有 4 种：`STANDARD` / `FAST` / `FASTV3.3` / `LITE`）。而"机型 + id"这个组合键会在换机型后变掉，于是选中、草稿、勾选列会全部指错。

所以：**由 Rust 给每个版本一个稳定 uid**，加载来的是 `"{加载时的机型}/{id}"`，新建的是 `"new-{序号}"`。uid 不写回上游，只在工作台内部与 IPC 上使用。

上一版我是靠 `rename_snapshot` + `relink_version` **事后搬运**来绕过这个问题的 —— 这一版从根上避免。

---

## 4. 统一写入口、草稿、撤销（决议 4）

### 4.1 patch 的载荷

```rust
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Patch {
    /// 改一个参数值。value 为 None = **删键 = 挂回继承**（不是"值设成空"）
    SetValue { level: Level, owner: String, key: String, value: Option<Value> },
    CloneVersion { from_uid: String, name: String },
    NewVersion { machine_id: String, name: String },
    RenameVersion { uid: String, name: String },
    MoveVersion { uid: String, to_machine_id: String },
    ArchiveVersion { uid: String },
    RestoreVersion { uid: String },
    PurgeVersion { uid: String },
    /// None = 挂回继承机型默认；Some = 本版本独立一份
    SetBbs { uid: String, list: Option<Vec<String>> },
    SetVisibility { file_id: String, visibility: Visibility },
    SetBundle { bundle_id: String, presets: Vec<String>, bbs: Vec<String> },
    MarkBuilt { uids: Vec<String> },
}

pub enum Level { Machine, Version }
```

两条语义必须写死：

- **`SetValue` 必须带 `level`。** 没有它，批量改就只能靠调用方猜，猜错的后果是静默打破继承。
- **`value: None` = 删键 = 挂回继承。** 没有额外的"脱钩"标记 —— 脱钩的载体就是"这个键在不在这一层"，所以它天然是按项算的，不是按版本算的。

### 4.2 唯一写入口

```rust
#[tauri::command]
pub fn wb_apply_draft(label: String, patches: Vec<Patch>) -> Result<BookView, AppError>
```

- `label` 是人类可读的一句话（`"A1 基底 · X 轴偏移"`、`"批量 · X 轴偏移 · 4 列"`、`"生成 5 个版本"`），前端撤销按钮的 title 直接显示 `撤销：{label}`
- **一次调用 = 一次手势 = 一条撤销**
- 返回**完整的派生视图** `BookView`（见第 5 节），前端拿到就刷新，不自己推算

### 4.3 草稿：Rust 落盘

```
workbench/.draft/book.json    整本草稿（只存"改了什么"，不存整本）
```

**只存差异而不存整本的理由**：存整本的话，上游数据一变，草稿就会把旧值糊回去。

草稿结构照参考那份，但归 Rust：

```rust
pub struct Draft {
    /// "m:A1:toolhead.offset.x" / "v:A1/STANDARD:toolhead.offset.x"
    pub values: BTreeMap<String, Option<Value>>,
    pub added: Vec<Version>,
    pub renamed: BTreeMap<String, String>,
    pub moved: BTreeMap<String, String>,
    pub archived: BTreeMap<String, bool>,
    pub purged: Vec<String>,
    pub bbs: BTreeMap<String, Option<Vec<String>>>,
    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    pub built: BTreeMap<String, BuiltRecord>,
    pub seq: u64,
}
```

**应用顺序有讲究：先结构（删/改名/移动/归档/新增/生成记录），再值。** 反过来的话，新建版本上的值会因为"那时还没有这个版本"而被丢掉。

脏计数 `dirty_count` 把**结构操作也各算一处** —— 它们和改一个值一样要被保存。

### 4.4 撤销栈：前端会话内存，但写必须走 IPC

```
用户手势 → 前端生成 patches → wb_apply_draft(label, patches)
                                        ↓
前端把 { label, patches, inverse } 压进撤销栈（会话内存）
                                        ↓
点撤销 → wb_apply_draft("撤销：" + label, inverse)   ← 同一条路径
```

第一版**不做跨重启的撤销历史**（关了重开草稿还在，但撤销栈清空）。理由：跨重启撤销要把栈也落盘并处理"草稿被外部改过"的情形，而实际场景是"我这会儿改错了想退回去"。

反向 patches 由**后端算并返回** —— 它才知道改之前那一层有没有这个键（`Some(旧值)` 还是 `None`）。前端自己算反向会在"原来是继承来的"这种情形上出错。

```rust
pub struct ApplyResult {
    pub view: BookView,
    /// 撤销这次操作要提交的 patches。由后端算，因为只有它知道改之前那一层有没有这个键。
    /// **倒序**：正向 A→B→C，反向是 C⁻¹→B⁻¹→A⁻¹
    pub inverse: Vec<Patch>,
    /// 这次手势能不能进撤销栈。为 false 时界面**不给**撤销按钮 ——
    /// 给一个按下去没反应的按钮比没有按钮更糟
    pub undoable: bool,
}
```

### 4.5 哪些手势不进撤销栈

| 手势 | 可撤销 | 兜底 |
|---|---|---|
| 改值、改名、移动、归档、还原、新建、克隆、套餐、可见性、BBS | ✅ | —— |
| `PurgeVersion` | ❌ | 二次确认 + 回收站（`.trash/`，带时间戳前缀，反复删不互相覆盖） |
| `MarkBuilt` | ❌ | 它是生成的记录，不是编辑；撤销一条「生成过」没有意义 |

刻意**不做「有些删除能撤销、有些不能」**：草稿里新建还没保存的版本，删了确实能从草稿里恢复；已落盘的不能。但「看情况」的撤销比「从来不能」更糟 —— 用户点撤销之前得先想清楚这个版本保存过没有。所以一条规则：**删除不进撤销栈。**

### 4.6 版本清单是上游的，所以「删除」只对两种版本开放

`machine_catalog.json` 说有哪些机型、哪些版本 —— 那是上游的清单，工作台不改它（§3.4）。于是：

**删掉我们的版本文件，只是「这一版没有自有改动了」，它照样出现在树上。**

所以 `PurgeVersion` 只对两种版本开放：

| 版本 | 能删吗 | 不能删时该做什么 |
|---|---|---|
| 上游机型清单里还有的 | ❌ | **归档** —— 它离开树、不参与交付，但版本本身还在 |
| 上游已经不再有的（孤儿文件） | ✅ | 进回收站 |
| 草稿里新建、还没保存的 | ✅ | 直接从草稿里拿掉，连它的值一起 |

不拦的话，用户点了删除、版本还在，而且没有任何提示说为什么。所以拒绝时那句话直接指向归档。

顺带一条：**上游删掉某一版之后，我们那份文件不删也不静默忽略** —— 留着 + 在整本视图上报一条提示（`BookView.notices`）。留着它的值才有机会被搬走；静默忽略的话，「我明明改过」会一直是错的。


另外两条正确性要求，都是为了让「保存」按钮说真话：

- 改成和现在一样的值 ⇒ 不记草稿、不产反向，脏计数不涨。
- 改了又**改回已落盘的那个值** ⇒ 把草稿里那一条拿掉，而不是记一条「改成了旧值」。否则「改错了又改回来」会让保存按钮一直亮着。


---

## 5. 状态派生：谁算什么

**一份状态，一个算处。** Rust 在每次 `wb_apply_draft` 之后、以及被显式查询时，返回一份完整派生视图：

```rust
pub struct BookView {
    // —— 骨架 ——
    pub machines: Vec<MachineNode>,        // 已过滤归档
    pub archived: Vec<VersionBrief>,
    // —— 头部与状态带 ——
    pub badges: Badges,                    // 机型数 / 版本数 / 基底项数 / 覆盖项数
    pub dirty_count: usize,
    pub artifact: ArtifactState,           // fresh / stale / missing
    pub last_build: Option<String>,
    // —— 校验 ——
    pub issues: Vec<Issue>,
    // —— 生成 ——
    pub build_rows: Vec<BuildRow>,
}
```

矩阵那一屏单独查（它的列取决于前端勾了什么）：

```rust
#[tauri::command]
pub fn wb_matrix(cols: Vec<ColRef>, tab: Option<String>, query: String) -> Result<Matrix, AppError>
```

| 谁算 | 算什么 |
|---|---|
| **Rust** | 三层合并与来源层、`showWhen` 链与 `blockedBy`、`machineFilter` 的适用性、生成状态四档、BBS 三态、issues 三档、移动预览四组、批量影响范围、`minClientVersion` |
| **前端** | 选中哪个（`primary`）、勾了哪些列（`checked`）、折叠了哪些、当前分类页签、搜索词、撤销栈、草稿输入的中间态 |

界面状态（折叠/页签/勾选）也落盘，但落在 Rust 的 `.draft/ui.json` —— 它是本机状态，不入库。

---

## 6. IPC 契约（新一套，旧命令不保留）

前两版的 30 多个 `wb_*` 命令全部作废。新契约按"读整本 / 写一条 / 查一屏 / 做一件事"分四类：

```
读
  wb_open()                        -> BookView        首次加载：读上游 + 建骨架 + 恢复草稿
  wb_view()                        -> BookView        重新派生（校验、生成状态等）
  wb_registry()                    -> Registry        字段定义（74 条 + tabs + layout）
  wb_matrix(cols, tab, query)      -> Matrix          一屏矩阵
  wb_effective(machine, uid)       -> Effective       单个版本的有效配方（字段详情用）
  wb_stock()                       -> Stock           仓库盘点（18 个文件 + 三态 + 归属）
  wb_fallback()                    -> FallbackTable   回退登记表（20 条 + guide）
  wb_ui() / wb_save_ui(ui)                            界面状态

写（**只有这一个**）
  wb_apply_draft(label, patches)   -> ApplyResult

查（只读推演，不写）
  wb_preview_move(uid, to)         -> MovePreview     四组
  wb_preview_bulk(key, value, cols)-> Vec<BulkEffect> 批量影响范围
  wb_diff_draft()                  -> Vec<DiffLine>   未保存改动
  wb_diff_built(machine, uid)      -> Vec<DiffLine>   当前配方 vs 上次生成时

做
  wb_save()                        -> BookView        草稿写回仓库文件
  wb_discard()                     -> BookView
  wb_generate(uids)                -> GenReport       原子：全算完再一次性替换
  wb_revert_to_built(machine, uid) -> RevertReport    只改配方，不生成
  wb_preflight()                   -> PreflightReport
  wb_publish()                     -> PublishReport
```

前端的 `api.ts` 只做一件事：把这些包成带类型的函数。**不在前端做任何合并、推算、兜底**。

---

## 7. 前端布局（照参考的骨架）

```
┌ 状态条 26px ───────────────────────────────────────────┐
│ RECIPE {路径} ✓已保存 │ ARTIFACT {路径} ⚠待生成 │ LAST BUILD {时间}  ●待生成 │
├ 文件头 44px ───────────────────────────────────────────┤
│ ▣ 配方本  [6 机型][10 版本][基底 N 项][覆盖 N 项]   ↶撤销 ↷重做 丢弃改动 保存配方 │
├───────────────┬────────────────────────────────────────┤
│ 配方本         │ [参数] [套餐与菜单] [生成 N]           │
│ ▾ A1  基底N项 ☑ ├────────────────────────────────────────┤
│   标准版 〔3项自有〕☑│                                   │
│   ...          │   视角内容                              │
│ ▾ A2L 基底0项 暂无资源│                                  │
│ ──────        │                                        │
│ 维护           │                                        │
│  字段定义       │                                        │
│  仓库盘点       │                                        │
│  应急规则       │                                        │
│  回收站 N      │                                        │
├───────────────┴────────────────────────────────────────┤
│ 6 机型 · 10 版本 · 主选中 A1 基底      基底N项·覆盖N项 ●已保存 │
└────────────────────────────────────────────────────────┘
```

三条纪律照抄（它们是这套架构成立的前提）：

1. **树常驻，不是"当前对象选择器"。** 选中是两套：单击 = 切主选中（点机型行编基底，点版本行编覆盖）；勾选 = 加入对比，给矩阵当列。
2. **三个视角是视角不是步骤。**
3. **维护四项不属于任何机型版本，不受树的选中影响。** 渲染优先级：维护页 > 视角。

### 视觉规范

照参考那份 tokens 建一份我们自己的（值可以照抄，它是同一个产品群的设计系统）：控件高度 26/30/34、徽章圆角 2px、卡片 12px、浮层 16px、文字四级、来源三档色（灰=出厂 / 紫=机型 / 蓝=版本）、`--w-warn` 橙 = **"值变了 / 要留意"不是错误**、`--w-danger` 红 = 真的错了、蓝 = **草稿描边**（`inset 0 0 0 1px`，不换底色 —— 这样"是什么来源"与"改没改"两条信息都在）。

密度用 `ResizeObserver` 量**容器实测宽度**（不是 media query），四档 `ultra/wide/compact/mini`，断点 1600/1080/640。mini 档纪律：**不许藏掉任何可用操作**，只能改布局方向与隐藏说明文字。

---

## 8. 参数矩阵（这一版的技术核心）

### 8.1 列来自勾选，机型基底本身就是一列

这是上一版最大的缺失。列的形状：

```rust
pub struct ColRef {
    pub machine_id: String,
    /// None = 这一列是机型基底那一列
    pub version_uid: Option<String>,
}

pub struct Col {
    pub key: String,          // "A1" / "A1/STANDARD"
    pub machine_id: String,
    pub version_uid: Option<String>,
    pub level: Level,         // 写进哪一层 —— 不许弄反
    pub machine: String,      // 列头第一行 "A1"
    pub label: String,        // 列头第二行 "基底" / "标准版"
    pub own: usize,           // 这一层自己写了几项
}
```

三条判定照抄：

- **列序照配方本的顺序，不按点击顺序** —— 点击顺序会让同一份数据每次长得不一样
- **行取并集不取交集** —— 交集会让"只有 P1S 有的那几条"在勾了 P1S 的时候凭空消失
- **搜索跨全部分类**，命中 `label` / `key` / `tomlKey` / `sectionId` / `desc`；搜索一开分类过滤让开，并在说明条上写明"搜索跨全部分类"

### 8.2 表格

原生 `<table>` + `position: sticky` 双向冻结（`border-collapse: separate`，collapse 会丢 sticky 边框）：corner `z 3` / 列头 `z 2` / 行头 `z 1`，列头 `min-width 160px`，行头 `width 240px`。**不做虚拟滚动**：74 行 × 十几列，走"默认纯文本、点中了才升级成受控控件"这条路。

### 8.3 单元格四条互斥分支（顺序即优先级）

| # | 条件 | 显示 |
|---|---|---|
| 1 | 这台机型没有这个字段（`machineFilter` 排除） | `不适用`，压暗。title 说明"它没有这一项，不是值为空" |
| 2 | `uiComponent == gcode` | `{N} 行 · 点开` 按钮 → 右抽屉 |
| 3 | 被选中且未被条件关着 | 升级成真控件，底色 `--w-primary-soft` |
| 4 | 其他 | 只读文本 + 来源徽章；被条件关着时压暗，title 写明"由「{父字段}」控制，需{条件}" |

**"不适用"与"被关着"是两件不同的事**，必须分开说：前者是这台机型根本没有这一项，后者是有但被上级字段的条件关掉了 —— **看得见、改不动，并且写明由谁控制**。

值的显示文本统一由后端给（前端不做格式化）：空串显示 `空`（不是空白）、`switch` 显示 `开启`/`关闭`、有 `choices` 的显示命中项的 label。

### 8.4 三种改法

| 手势 | 行为 | 撤销粒度 |
|---|---|---|
| 点格子 | 这一格升级成真控件 | 每改一格一条 |
| 点列头 | **整列**同时可编辑，列头出现「列编辑」标记 | 每改一格一条 |
| 点字段名（行头） | 行下插入批量条 → 预览影响 → 确认 | **N 列压一条** |

批量的五步 SOP：**勾列（在树上）→ 点字段名 → 输入新值 → 弹窗看影响 → 确认**。

能落到哪几列由后端判（`wb_preview_bulk`）：这台机型有这个字段、且没被上级条件关着。两类目标**静默跳过**（不在预览里也不计入"N 列"）：不适用的、被关着的。

`gcode` 字段**拒绝批量**：一段多行 G-code 被整体盖掉是不可逆的误操作，批量条上只给一句话说明请逐列点开改。

### 8.5 选中条

选中某格后表下面常驻一条（不是浮层）：位置 → 键名 → 来源徽章 → 被关提示 → `挂回继承` / `收起`。

`挂回继承` 在这一层本来就没写过这一项时禁用，并给出两句不同的 title（能点 / 不能点各一句）。它提交的是 `SetValue{ value: None }`。

---

## 9. `showWhen`：链式求值与"看得见改不动"

实测 43 条字段带 `showWhen`，形状恒为 `{key, op, value}`（无嵌套、无 and/or），`op` 只有 `eq`(38) / `neq`(3) / `gt`(2)。

三处必须处理的真实脏数据：

1. **`value` 的 JSON 类型混杂**：有 `true`（bool）、`"off"`/`"tower"`（string），也有**字符串化的数字** `"0"`（如 `{key: "wiping.glue_z_lift_height", op: "gt", value: "0"}`）。比较要**先按被指向字段的 `valueType` 归一化**，不能直接比 JSON 值。
2. **条件要顺着 key 往上递归** —— 被指向的字段自己也可能有条件。要带**环检测**，否则一条自指链会把线程转死。
3. **`layout_schema` 还有 section 级 `showWhen`** —— 整组隐藏。矩阵按字段建行，所以这一条体现为"该 section 下所有字段一起被关"。

判定结果返回给前端：

```rust
pub struct BlockedBy {
    pub key: String,
    pub label: String,
    /// 已经拼成一句可直接显示的话：「等于 开启」
    pub need: String,
}
```

**句子由后端拼** —— 决议 5 的落点：业务判断归后端，展示归前端。

---

## 10. 生成 / 校验 / 发布

### 10.1 状态四档

```
已生成    当前有效配方 == 上次成功生成时的快照
待生成    不相等
未生成    没有快照
暂无资源  没有 MKP 预设也没有 BBS
```

判据是**快照比对，不看文件时间**。看时间的下场：碰一下文件就变"新"，而配方改了却看不出来；产物的时间戳一变，客户端就可能莫名要更新。

### 10.2 校验三档

```
阻断   数据自相矛盾，生成一定出错。**全程唯一的硬闸门**
待办   要人去填的空。不挡
提示   合法但值得知道。不挡
```

每条必须带**去处理**的落点（视角 + 机型 + 版本），前端据此跳过去并把主选中落上。一条说不清去哪儿的问题等于没报。

零问题不留白 —— 明确显示"都过了"，因为空白会被读成"还没校验"。

「有 N 个文件没进任何套餐」放**提示**档：仓库里放一个 0.2mm 的 profile 不分配给谁，是一种正常的交付身份，不是待修的事。

### 10.3 生成三个入口

主按钮 `生成待更新项（N）`，次要 `全部生成（N）` 与 `生成勾选的（N）`。有阻断时三个全禁用，并在下面写出是哪一条。

**原子**：先把全部产物算完，任一项算不出来则整批不动；全部成功才逐个原子替换。产物字节没变时**不重写文件**（mtime 变化会让同步工具以为有更新）。

### 10.4 恢复配方

恢复到上次成功生成时的配方，**只改配方不生成**。快照里只有有效值，看不出当时是哪一层给的，所以恢复一律写在版本这一层 —— 原本继承来的那几项会从此脱钩，**名单要当场列出来，不许闷着改**。

---

## 11. A2L：参数侧与资源侧是两件事

上一版我把这两件混了，这里钉死。实测 A2L 的缺失是**四重的**：

| 面 | 实测事实 | 界面 |
|---|---|---|
| 参数 | `machineFilter` 里 **A2L 出现 6 处** —— 参数层认它 | 矩阵里**有值有来源色**（基底空 → 每个字段回落到出厂默认，灰色「出厂」徽章） |
| 产物 | `presetFile = ""`、`mkpPresetAssetId = ""` | `暂无资源`，生成时跳过 |
| 套餐 | `defaultBundle` 字段**整个不存在**、`recommendedBundle = ""`、`bundles.json` 里没有它 | 不给下载 |
| 尺寸 | `dimensions = null`，且 `machine_catalog.dimensions` 字典里根本没这个键；无 `forbiddenZones` | 校验里一条**待办**：尺寸未配置 |

所以：**参数有值 + 资源没有，这两件同时成立，不矛盾。** 客户端能看到 A2L 这一组、不允许下载、并**明确列出两条缺失原因**（不是空白也不是红色错误）—— 用词 `暂不支持`。

A2L 是唯一能把"未配置 / 暂无资源 / 暂不支持"三个词区分开的真实样本，所以它是这一版的**主验收用例**。

---

## 12. 兼容性声明：上游还没有，我们不能瞎填

实测结论（全仓 JSON + TOML 无截断搜索）：

- `minClientVersion` / `schemaVersion` / `minVersion` / `compatible` / `requires` / `appVersion` / `clientVersion` / `formatVersion` —— **这 8 个键一个都不存在**
- 唯一为此设计的字段是 `manifest.json` 的 `minimumClient`，实测值是**空字符串** → 必须显式当"未声明"处理，不能拿去做字符串比较
- `content/release.json` 的 `compatibility.minimumBase = "0.0.2"` **不是客户端要求** —— 它约束"应用自更新时允许从哪个基线升"
- 全局数据格式版本号**不存在**。只有两个局部的：`manifest.manifestVersion = 2`、`fallback_registry.version = 1`
- `param_registry.json` 自身**没有任何版本字段**（顶层只有 `params` / `tabs` / `updated`）

所以这一版的做法：

1. 工作台**读**并显示这些事实（含"未声明"这个状态本身）
2. 发布元数据里给 `minClientVersion` 留位，界面上摆出来但**只读**，旁边写明为什么现在填不了 —— 要客户端先给一份兼容性清单，工作台填了就等于对外承诺一件没人验证过的事
3. 出货检查把"最低客户端版本未声明"报成**待办**，不报阻断（上游就是这个状态，阻断等于永远发不出去）

这是相对第二版的**一处退让**：第二版我按 `capability/` 目录算出了 `minClientVersion`，但那份能力定义是我自己按 registry 手写的 —— 它校验的其实是"配方有没有超出我以为客户端支持的范围"，不是真兼容性。这一版把它降级为待办并说清原因，比假装算得出来更诚实。

---

## 13. 文案纪律（决议 5）

沿用参考的纪律，句子自己写。四条：

1. **一个词只有一个出处。** 状态词与解释句集中在 Rust 的一个模块里（它是唯一判定处，文案跟着判定走），前端不许就地造词。参考项目的教训写在它的注释里：同一个状态在一处叫"可选"另一处叫"未关联"，界面开始自相矛盾而两边看起来都对。
2. **解释"改了会怎样"，不解释"来自哪一层"。** 三档来源的解释句要写成：机型基底 → 「改它，这台机型下没自己写过这一项的版本都会跟着变」；版本覆盖 → 「这是这个版本自己写的，改机型基底不影响它」；出厂默认 → 「机型与版本都没写过这一项」。
3. **每个禁用都有一句"为什么不能点"。** 灰按钮没有理由就是个谜。
4. **空值有说法，不留空白。** 统一一个占位符，因为空白分不清"没有"与"没画出来"。读失败一律显示原因 + 一句"不装成空表"的解释 —— 显示"0 个字段"会让人以为注册表是空的。

---

## 14. 受影响文件

### 作废（第二版的产物，整块删掉）

```
g:\project\MKPSupportEase\src\workbench\api.ts
g:\project\MKPSupportEase\src\workbench\pages\*.tsx          （6 个）
g:\project\MKPSupportEase\src-tauri\src\workbench\matrix.rs
g:\project\MKPSupportEase\src-tauri\src\workbench\ops.rs
g:\project\MKPSupportEase\src-tauri\src\workbench\capability.rs
g:\project\MKPSupportEase\workbench\registry.json            （自己写的 10 条，换成读上游 74 条）
g:\project\MKPSupportEase\workbench\capability\*.json         （自己手写的能力定义）
```

### 保留并改造

| 文件 | 怎么改 |
|---|---|
| `src-tauri\src\workbench\paths.rs` | 加上游仓库根的定位（`MKPSE_PRESETS_DIR` 环境变量 + 兄弟目录探测） |
| `src-tauri\src\workbench\store.rs` | 保留原子写与 id 白名单；草稿从"按版本一份"改成"整本一份" |
| `src-tauri\src\workbench\generate.rs` | 保留渲染与原子替换；`minClientVersion` 来源改成"未声明" |
| `src-tauri\src\workbench\publish.rs` | 保留"catalog 最后写"的顺序 |
| `src-tauri\src\workbench\preflight.rs` | 两档改三档（阻断 / 待办 / 提示），每条加 `where` 落点 |
| `src-tauri\src\workbench\clock.rs` | 不动 |

### 新增：Rust 分层

```
src-tauri\src\workbench\upstream\mod.rs        上游仓库读取（只读）+ Upstream 聚合 + 跨文件断言 + 零写入的扫源码判据
src-tauri\src\workbench\upstream\registry.rs   param_registry + layout_schema（含三条一致性断言）
src-tauri\src\workbench\upstream\manifest.rs   manifest.json（资源 72 / 套餐 5 / 内容文件 15 / 兼容声明）+ preset_registry 的两列
src-tauri\src\workbench\upstream\catalog.rs    machine_catalog + manifest 的机型视图 join（机型/版本/尺寸/禁区/别名）
src-tauri\src\workbench\upstream\fallback.rs   fallback_registry

src-tauri\src\workbench\domain\mod.rs
src-tauri\src\workbench\domain\layer.rs        三层取值 + 来源层
src-tauri\src\workbench\domain\visibility.rs   showWhen 链 + 环检测 + blockedBy
src-tauri\src\workbench\domain\patch.rs        Patch 定义 + 应用 + 反向计算
src-tauri\src\workbench\domain\derive.rs       状态派生（BookView / Matrix / BuildRow）
src-tauri\src\workbench\domain\preview.rs      移动预览四组 + 批量影响范围
src-tauri\src\workbench\domain\wording.rs      状态词与解释句的唯一出处

src-tauri\src\workbench\app\mod.rs             IPC 入口（第 6 节那 20 个命令）
src-tauri\src\workbench\app\dto.rs             返回给前端的形状
```

### 新增：前端

```
src\workbench\api.ts                  只包 IPC，不做业务
src\workbench\tokens.css              视觉规范
src\workbench\useDensity.ts           ResizeObserver 四档
src\workbench\AppB.tsx                外壳：状态条 + 文件头 + 树 + 视角 + 底部条
src\workbench\shell\StatusStrip.tsx / PageHeader.tsx / StatusBar.tsx
src\workbench\tree\RecipeTree.tsx / CloneDialog.tsx / MoveDialog.tsx
src\workbench\shared\Modal.tsx / Drawer.tsx / ConfirmDialog.tsx / DiffView.tsx / OriginChip.tsx
src\workbench\views\ParamsMatrix.tsx / MatrixCell.tsx / GcodeDrawer.tsx
src\workbench\views\BundlesMenu.tsx / FilePicker.tsx
src\workbench\views\Build.tsx / Issues.tsx
src\workbench\maintain\FieldDefs.tsx / Stock.tsx / Fallback.tsx / Trash.tsx
src\workbench\undo.ts                 撤销栈（会话内存，写走 IPC）
```

---

## 15. 边界与异常

| 情况 | 处理 |
|---|---|
| 上游仓库找不到 | 工作台**不启动业务**，显示一句"找不到 mkpse-presets，配的路径是 X" + 怎么配。不用空数据装成能跑 |
| 三条一致性断言任一不过 | 只读模式启动 + 明确报是哪一条不过。**不允许在对不上的字段定义上生成产物** |
| `showWhen` 自指或成环 | 环检测命中就当"不可见性判定失败"，该字段标为可编辑并记一条待办，**不静默死循环** |
| `machineFilter` 里有 catalog 里不存在的机型 | 记一条提示，不阻断（上游真的有这种：A2L 在 filter 里但没有 toml） |
| `choices` 里某项标了 `deprecated` | 下拉里仍然列出但标注；当前值正好是废弃项时，在格子上标一句 |
| 草稿里的版本 uid 在上游已经不存在 | 该条草稿丢弃并记一条提示，不让整本草稿失效 |
| 移动到不存在的机型 | 当没移（那会让这个版本从树上整个消失，比不动更糟） |
| 生成中途失败 | 整批不动，旧产物一个字节不改，记失败原因 |
| 前端提交的 patch 带了非法 level / 未知 key | 后端拒绝整批并报明哪一条，**不部分应用** |
| 撤销栈与草稿不一致（外部改了草稿文件） | 下一次 `wb_apply_draft` 返回的 view 与前端预期不符时清空撤销栈并提示 |

---

## 16. 预期结果

1. 工作台仍是同仓库第二个窗口，三道编译隔离一条不减（默认构建的 `dist/` 与二进制里搜不到它）
2. **接真数据**：74 条字段定义、6 机型 10 版本、18 个资源文件、5 个套餐、20 条回退规则，全部来自 `mkpse-presets`，一条都不是我编的
3. **三层取值**，每个格子能看出值来自哪一层，以及"改了会怎样"
4. **一条写入路径**：所有修改经 `wb_apply_draft`，一次手势一条撤销，撤销与重做走同一条路
5. **状态只有一份**，Rust 算，前端只显示
6. 矩阵能同时摊开多个机型的**基底列与版本列**做对比；三种改法；批量先预览再确认
7. `showWhen` 生效：被关着的格子看得见改不动，并说明由谁控制
8. 校验三档，每条能跳过去；阻断是全程唯一硬闸门
9. 生成默认只做待更新项；产物字节没变时不重写文件
10. **A2L 三件事分得清**：参数回落出厂默认（有值有来源色）/ 资源暂无（不给下载）/ 原因明确列出（用词"暂不支持"）
11. 兼容性声明按上游的真实状态处理："未声明"是一个明确的状态，不是 0 也不是空白

---

## 17. 明确不做

- 客户端侧的一切（下载、校验、应用、归档、用户副本）
- 跨重启的撤销历史（第一版撤销栈只在会话内）
- 把机型级默认值并进取值链（结构留口子，第二步做）
- 字段定义的编辑（增删字段、改类型、改范围要改上游 `source/registry/param_registry.toml` 再构建）
- 回写 `mkpse-presets`（它是上游，只读）
- 自己编造 `minClientVersion`（上游没有，我们不填）
- 前端自行实现任何业务规则（这是这一版的第一条铁律）
