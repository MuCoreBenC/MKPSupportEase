# A43 参数页 → A 真实数据 · 移植账（2026-10-02）

> **口径（作者定，先读这条）**：
> **搬的是 A43 的「前端实现经验」，不是 A43 当年的数据假设。**
> 每一项都先问：**这个东西现在 A 的真实业务里应该由谁提供？**
> A 有真实定义 → 以 A 为准；A 没有 → **补 A 自己的真契约**，不塞 A43 的假数据。
>
> 坐标：A = 本仓（真后端）；B = `mkp-adaptive-console`，客户端最新稿 `src/versions/a43/`。

---

## 0. 一句话现状

**A 的参数页本来就与 A43 同源**（`CategoryPills` / `ParamCard` / `ParamRow` / `SearchField` /
`GcodeBlock` / `HistoryDrawer` / `PresetPickerDrawer` / `highlight` / `useParams` / `PageParams`
全部同名同构）。A43 是这份代码**继续演进**的版本。

所以这一轮 **不是"把 A43 搬进 A"**，而是：

```
A43 的 UI 增量（ParamCard 装订子卡/折叠/手风琴头 + 塔地图 + 弃用标记）
        +
A 的真实数据链（catalog / 草稿链 / 编辑目标 / 操作记录）
```

**`useParams` 不搬** —— A 那份（1191 行）比 A43（1088 行）**领先**（草稿链 / 编辑目标 /
操作记录是 A43 没有的）。搬的是它的**兄弟文件**。

---

## 1. A43 参数页依赖的数据 → A 真实来源（逐项）

| # | A43 需要 | A43 从哪来 | A 当前项目 | 处理 |
| --- | --- | --- | --- | --- |
| 1 | 参数字段（key/label/desc/unit/group/tab） | `src/server/data/param_registry.json`（假注册表） | ✅ **真**：`presets/registry/param_registry.toml` → catalog definition → `ParamMeta` | **接 A 的 catalog** |
| 2 | 值（当前值） | 包的 `machines[].versions[].values` | ✅ **真**：`getMachineParams`（三层取值） | **接 A 的 IPC** |
| 3 | 三层值（出厂/机型/版本） | ⚠️ A43 **不做继承推导**（包里只有一层有效值） | ✅ **真**：`presetdata::resolve` 三层 | **A 更全，用 A 的** |
| 4 | 分类 / 分组结构 | `param_registry.json` tabs + `layout_schema.json` | ✅ **真**：catalog registry 的 tabs/layout | **接 A 的** |
| 5 | 机型 / 版本 | 包的 `machines[].versions[]` | ✅ **真**：catalog | **接 A 的** |
| 6 | `choices` | `param_registry.json` 的 `params[].choices` | ✅ 真（`RecipeParamDto.choices` 已下发 value/label） | **接 A 的**，但**选项级 deprecated 要补**（见 §3） |
| 7 | `min` / `max` / `step` | 假注册表 | ✅ 真（`ParamMeta`） | **接 A 的** |
| 8 | `showWhen`（条件显隐） | 假注册表 | ✅ 真（`ParamMeta.showWhen` + `blockedBy`） | **接 A 的** |
| 9 | `deprecated`（**字段级**） | 包 `ClientFieldDef.deprecated` ← 假注册表 | ⚠️ **半通**：上游 ✅ / catalog ✅ / `MetaDto` ✅ / **`RecipeParamDto` ❌** / **前端 `src/app/` ❌** | **补两段**（见 §3） |
| 10 | `deprecated`（**选项级**） | `choices[].deprecated` ← 假注册表（1 处：`outer_structure=护套`） | ⚠️ **半通**：上游 ✅（真 TOML 里就有！）/ **IPC `ChoiceDto` ❌** / 契约 ❌ / 前端 ❌ | **补三段**（见 §3） |
| 11 | `dimensions.plate`（选板 id） | 包 `MachineDimensions.plate` | ❌ **A 两侧都没有** | **补契约**（§4） |
| 12 | 板轮廓几何（`bedOutline`） | `bedOutlineA43.ts` 手写的 `A1`/`A1_MINI` 路径 | ❌ **A 没有** | **补**（§4） |
| 13 | 塔的尺寸 / 位置 | `useParamsA43` / `PageParamsA43` 里**按参数值现算** | ⚠️ 取决于参数是否在真注册表里（见 §5） | **接 A 的参数值** |
| 14 | 模型 / STL / 3MF | **A43 参数页不读任何模型**（只有文案提到） | ⚠️ **有资产、契约弱、UI 全缺**（见 §6） | **单独一刀** |
| 15 | preset 状态 / 两条线 / 草稿 | localStorage（假） | ✅ **真**：`run/active-preset.json` + `run/draft-preset.json` + 血统 | **接 A 的**（A 更全） |
| 16 | mock 数据（`mockA43.ts`） | 手编演示数据 | — | ❌ **不搬** |

---

## 2. 移植范围（文件级）

### 2.1 直接搬 UI（数据换成 A 的）

| A43 文件 | 行数 | A 对应 | 增量 |
| --- | --- | --- | --- |
| `ParamCardA43.tsx` | **416** | `ParamCard.tsx`（206） | **+210**：装订子卡 / 折叠统一 / 手风琴头 |
| `PageParamsA43.tsx` | **1248** | `PageParams.tsx`（1032） | +216 |
| `GcodeBlockA43.tsx` | **245** | `GcodeBlock.tsx`（155） | +90 |
| `ParamRowA43.tsx` | **306** | `ParamRow.tsx`（252） | +54 |
| `SearchFieldA43` / `CategoryPillsA43` / `PresetPickerDrawerA43` / `highlightA43` | — | 同名 | 基本一致（核对即可） |

### 2.2 A43 独有、A 必须新增（塔地图 · 作者点名"必须"）

| A43 文件 | 行数 | 用途 |
| --- | --- | --- |
| `TowerMapA43.tsx` | 251 | 塔地图（左地图 + 参数行） |
| `TowerCoreSvgA43.tsx` | 372 | 塔的 SVG 表现 |
| `bedOutlineA43.ts` | 92 | 打印板轮廓（`A1` / `A1_MINI`） |

### 2.3 **不搬**

- `useParamsA43.ts`（A 的 `useParams` 领先：草稿链 / 编辑目标 / 操作记录）
- `mockA43.ts`（假数据）
- `packageA43.ts`（A43 的包管理 = localStorage；A 用 catalog + IPC）

---

## 3. `deprecated` 链路断点（**要补的**）

上游**早就真了**（真 TOML 里 8 处：7 条字段级 + 1 处选项级「护套」），断在下游：

| 段 | 字段级 | 选项级 | 要动 |
| --- | --- | --- | --- |
| 上游 `param_registry.toml` | ✅ 7 处 | ✅ 1 处（`sheath`/护套） | — |
| Rust `ParamDef.deprecated` / `Choice.deprecated` | ✅ | ✅ | — |
| catalog 构建（`catalog.rs`） | ✅ 代码已带 | ✅ | — |
| **`catalog.generated.json`** | ⚠️ **0 命中（产物过期）** | ⚠️ | **重跑 `cargo run --bin gen-catalog`** |
| IPC `ParamMetaDto.deprecated` | ✅ | — | — |
| **IPC `RecipeParamDto`** | ❌ 无 | — | `ipc/presets.rs` 加字段 |
| **IPC `ChoiceDto`** | — | ❌ 无 | `ipc/presets.rs` 加字段 + 赋值 |
| TS `ParamMeta.deprecated` | ✅ | — | — |
| **TS `RecipeParam`** | ❌ 无 | — | `contract.ts` 加 |
| **TS `choices[]`** | — | ❌ 无 | `contract.ts` 加 |
| **前端 `src/app/params/`** | ❌ 零消费 | ❌ | `useParams` + 控件（共用件 `FieldOption.deprecated` **已有**，可直接用） |

> **注意**：选项级弃用**不是为满足 A43 UI 硬造的** —— 真注册表里**本来就有**
> （`wiping.outer_structure` 的 `sheath` = 护套，`param_registry.toml:1430-1433`），
> 所以这条是**把真数据接通**，不是编字段。

---

## 4. Plate（打印板）—— **作者 2026-10-02 定死的概念**

> **Plate 是独立于机型的实体。机器只引用一个 `plateId`，不再用机型 ID 推导板子。**
> 这样以后增加板子不会把机器模型和板子模型绑死，将来"同一台机器可选多块板"也不会破坏模型。

```text
Machine
  ├─ id: "A1"
  ├─ display: "A1"
  └─ plateId: "single-latch-256"     ← 引用，不是推导

Plate
  ├─ id: "single-latch-256"          ← 描述性名字，**不做机型耦合命名**（不叫 `a1-plate`）
  ├─ name: "单卡舌 256"
  ├─ w / d: 板件外轮廓尺寸（mm）
  ├─ frame: { x, y, w, h }           ← 可打印区在轮廓坐标里的位置
  ├─ bodyPath: 板身路径（不含卡舌/把手）
  └─ path: 完整外轮廓（含卡舌/把手，孔洞靠 evenodd）
```

**板 id 命名规范（避免机型耦合）**：

```toml
[[plates]]
id = 'single-latch-256'    # 单背卡舌 + 前缘把手，256 可打印区
name = '单卡舌 256'

[[plates]]
id = 'dual-latch-180'      # 两只背卡舌，180 可打印区
name = '双卡舌 180'
```

**机器侧**：

```toml
id = 'A1'
plateId = 'single-latch-256'
```

→ 于是 `A1` / `P1S` / `P2S` / `X1C` / 未来的 `A2L` 都可能指向**同一块板**，
而"同一台机器换板"只是改一个引用。

### 4.1 A 侧的现状：**从零开凿**（四条链全无）

| 段 | 现状 | 要动 |
| --- | --- | --- |
| 板数据源 | ❌ **无**（`presets/machines/A1.toml` 无任何 plate 键） | **新增**（位置见 §4.2） |
| Rust `Dimensions`（`presetdata/catalog.rs:135-145`） | ❌ 无 | `Machine` 加 `plate_id: Option<String>` |
| 机器加载（`load_machines` :792-874 / `load_dimensions` :901-979） | ❌ 无 | 读 `plateId` |
| TS 契约（`contract.ts:142-150, 162-182`） | ❌ 无 | `Machine.plateId?: string` |
| IPC / catalog 产物 | ❌ 无 | 跟着 catalog 带出 |
| 板几何常量 | ❌ 无 | 搬 A43 的 `bedOutlineA43.ts`（**几何常量**，非假业务数据） |
| 前端消费 | ❌ 无 | 塔地图（下一刀） |

### 4.2 板数据放哪（**待作者定**）

现有两种范式都在用：

| 候选 | 形态 | 像谁 |
| --- | --- | --- |
| `presets/plates/*.toml` | 一板一文件，目录扫描 | **像 `machines/*.toml` / `forbidden_zones/*.toml`** |
| `presets/registry/plates.toml` | 一份集中定义，`[[plates]]` 数组表 | 像 `param_registry.toml` |

**A43 的板几何是"生成物"**（`scripts/bed-outline.mjs` 从官方板件模型扫出来的路径），
而 A 里**没有那个生成脚本、也没有源模型**（`assets-src/bed/` 不在 A）。

所以 `bodyPath` / `path` 这两条长路径**要么搬常量、要么把生成脚本一起搬过来**（待定，见下）。

### 4.3 A43 的板 id 是机型耦合的（**要改掉的**）

`bedOutlineA43.ts` 的键是 `A1` / `A1_MINI` / `P1S` / `P2S` / `X1C`，其中
**P1S / P2S / X1C 与 A1 的几何逐字节相同**（只是复制了三份）—— 实际只有 **2 块板**：

| A43 键 | 真实 | 应改成 |
| --- | --- | --- |
| `A1` / `P1S` / `P2S` / `X1C`（几何相同） | 单卡舌 256 | `single-latch-256` |
| `A1_MINI` | 双卡舌 180 | `dual-latch-180` |

→ 搬到 A 时**按新命名去重**（2 块，不是 5 块），机器各引用一个 id。

---

## 5. 塔（Tower）—— 作者点名"必须"

- A43 的塔**不读专门数据**：`TowerMapA43` / `TowerCoreSvgA43` 从**参数值现算**
  （塔高 / 层数 / 半径等来自涂胶参数）。
- **所以塔的真实来源 = A 的参数值**（走 `getMachineParams`）。
- **待确认**：A43 现算用到的那些参数 key，在 A 的真注册表里**是否都存在**？
  → **移植前必须逐个核对**（若 A 缺某个 key，说明该参数在真业务里不存在，塔图要跟着调整，
  **不能为画塔硬造参数**）。

---

## 6. 模型 / 3MF（单独一刀，建议排后）

**A 的现状 = "有资产、契约弱、UI 全缺"**：

| 段 | 状态 |
| --- | --- |
| `presets/assets.toml` | ✅ 3 条 `model` 登记 |
| 产物 | ✅ 3 个真 `.3mf`（`public/assets/models/`，**无 .stl**） |
| Rust `AssetKind::Model` | ✅ 在用（catalog 里 3 条 `kind:"model"`） |
| TS 契约 | ❌ `FileKind` 不含 `model`；`CatalogFile.kind` 只是弱 `string` |
| `src/app/` 消费 | ❌ **零消费**（只有无关的 `CalibModel` 老链路） |
| 工作台消费 | ✅ `AssetsPage` / `BundleResourcesModal` |

→ **"把 STL/模型搬进来"不是搬 A43 的 mock**（A43 参数页压根不读模型）；而是
**把 A 已有的真资产接上真契约 + 补 UI**。**建议单独一刀**，不混进参数页 UI。

---

## 7. 施工顺序（**作者 2026-10-02 定：Plate 排在塔地图之前**）

作者原话：「不是先把 `bedOutline` 硬搬过来，再想办法认板子，而是**先把 Plate 作为真实数据契约
补起来**，然后塔地图消费它。」

```
① 补 deprecated 链路（重跑 gen-catalog + IPC 两处 + 契约两处 + 前端消费）
        ↓  （这是搬 ParamCard 的前置：弃用 UI 要它）
② 搬参数页 UI：ParamCard / ParamRow / GcodeBlock / PageParams 增量
        ↓  （useParams 保留 A 的；只搬 UI 表现）
③ Plate 正式进入 A 的机器数据
        ↓  machine.plateId → plate registry / geometry
        ↓
   塔地图：TowerMap / TowerCoreSvg / bedOutline（消费 Plate）
        ↓
④ 【单独一刀】模型 / 3MF：补 kind 契约 + 客户端 UI
```

**待作者裁决只剩一条**：**板数据放哪**（`presets/plates/*.toml` 一板一文件 /
`presets/registry/plates.toml` 集中一份 —— §4.2 两个候选，都能贴合现有范式）。

**已自查、无需裁决**：塔现算的 5 个参数 key 在 A 真注册表里**全都有**（§5）。

---

## 8. 附：本账的证据位置

- A43 参数页依赖：`a43/params/useParamsA43.ts:178-247,509-546,642-646,774-794`、
  `a43/params/ParamCardA43.tsx`、`a43/params/PageParamsA43.tsx:178`、
  `a43/store/packageA43.ts`、`src/server/resolve/params.ts:91-221`
- A 的真实链路：`presets/registry/param_registry.toml:1430-1433`、
  `presetdata/registry.rs:113-120,178-180`、`runtime/catalog.rs:403`、
  `ipc/presets.rs:533-536,587-590,594-626,700-703`、`api/contract.ts:48-76,566-614,752-760`
- 模型：`presets/assets.toml:78-94`、`public/assets/models/*.3mf`、
  `presetdata/assets.rs:70-76`、`workbench/views/AssetsPage.tsx:57,62,239`
