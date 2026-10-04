# 双仓对齐报告（2026-10-02）

> 对比 **MKPSupportEase**（本仓，真产品 / 真后端）与
> **mkp-adaptive-console**（`/Users/wzy/projects/mkp-adaptive-console`，前端稿号试验台 / 假后端）。
>
> 坐标（作者给）：B 的客户端最新稿 = **A43**（`src/versions/a43/`）；工作台最新稿 = **C15**（`src/versions/c15/`）。
> **C15 工作台几乎没变、最接近本仓现状；A43 客户端是旧的。**
>
> A43 的 `README.md` 开头自己写着：「**要搬去 `G:\project\MKPSupportEase` 的就是这一半**」——
> 那个仓库的使命就是"前端试验台 → 搬进真产品"。本报告是那次搬运前的地图。

---

## 0. 一句话结论

**两个仓库不是"分叉"，是两种角色**：

| | B：mkp-adaptive-console | A：MKPSupportEase（本仓） |
| --- | --- | --- |
| 定位 | **前端 UI 试验台**（稿号制，每稿整目录复制） | **真产品**（Tauri + Rust 真后端） |
| 后端 | **假**：`src/server/` + `src/mock/` + localStorage + `public/cloud/presets.json` | **真**：catalog / 交付 / 草稿链 / Bootstrap |
| 强项 | 参数树 UI 交互（折叠 / 装订子卡 / 动效 / 弃用标记） | 数据架构 / Preset 生命周期 / 交付链 |
| 契约 | `MkpApi` = **四轨并集（38 方法）** | `MkpApi` = **纯用户端口径**；工作台走独立 `wb_*` |

**唯一对齐面 = `src/api/contract.ts`**（数据契约）。契约一致 = 可自行推进；契约不一致 = 冲突，找作者裁决。

---

## 1. 数据契约对比（`src/api/contract.ts`）

### 1.1 口径差异（最重要的一条）

**两边的 `MkpApi` 不是同一份契约**：

- **B（试验台）**：把四个轨的方法并在一张表里（工作台 + 客户端 + 配方本 + 打包），约 **38 个方法**，
  含旧世界的 `getRecipeBook` / `saveRecipeBook` / `getBundles` / `buildClientData` / `uploadToCloud`。
- **A（本仓）**：`MkpApi` 是**纯用户端**口径；工作台那一套走**独立的 `src/workbench/api.ts`（`wb_*`）**。
  A 的 `contract.ts:666-677` 明确注释：「这一份是产品仓的口径，不是试验场那份的照抄」。

→ **这不是冲突，是 A 有意收窄**（工作台与用户端分离）。但对齐时要意识到：**B 的方法不能整张表照搬**。

### 1.2 A 有、B 无（B 缺的真业务契约）

`UserPresetFile` · `BasedOn` · `MineState` · `PresetDraft` · `CommittedDraft` · `UserFileIdentity` ·
`ImportStage` / `StagedImport` / `ImportItem` / `ImportOutcome` · `RuntimeCatalog` 一族 ·
`CatalogRegistry` 一族 · `ArchivedFile` · `DeliveryTrust` · `ActivePreset` / `ActiveOrigin` ·
`RemoteUpdateCheck` · `DownloadStage` / `DownloadTick` / `DownloadOutcome` · `PresetSource` ·
`ErrorCode` / `AppError`

→ 全是**这一轮做出来的真业务**（用户文件生命周期 / 交付信任 / Bootstrap / 结构化错误）。**B 完全没有。**

### 1.3 B 有、A 无（B 的前端实验需要、A 还没接）

| 类型 | B 的位置 | 说明 |
| --- | --- | --- |
| `MachineDimensions.plate?: string` | B `contract.ts:237` | 塔地图**选板**用（A41 做的）。A 无此字段 |
| `ClientFieldDef.deprecated?: boolean` | B `contract.ts:758` | **字段级弃用标记** |
| `ClientFieldDef.choices[].deprecated?` | B `contract.ts:764` | **选项级弃用标记**（"护套"划线） |
| `TestModel` · `RecipePreview` · `RecipeHealth` · `LocalUserFile` · `AppliedPreset` · `FallbackRule` … | B 各处 | 试验台口径（部分对应 A 的 `UserPresetFile` / `ActivePreset`，但**字段模型不同**） |

→ **这三条是真实的前端需求，A 后端要补**（尤其 `deprecated` 与 `plate`）。

### 1.4 字段级差异（同名类型）

| 类型.字段 | A（本仓） | B（试验台） | 判定 |
| --- | --- | --- | --- |
| `RecipeParam.tomlKey` | ✅ **有**（① 刚补，`contract.ts:58`） | ❌ 无（只有 `ParamMeta.tomlKey`，`contract.ts:355`） | **A 领先**；B 将来做字段级 patch 也要补 |
| `ParamMeta.tomlKey` | ✅ 有 | ✅ 有 | 一致 |
| `ParamMeta.deprecated` | ✅ 有 | ✅ 有 | 一致 |
| `PresetFileInfo`（全部字段） | 一致 | 一致 | 一致 |
| `UserPresetFile` vs `LocalUserFile` | 扫盘 + **血统**（basedOn*）+ 文件级状态 | 假后端固定演示集 + 手标 `machineIds` | **冲突**：模型不同（A 是真扫盘） |
| `ActivePreset` vs `AppliedPreset` | origin / fileName / path / sha256 / intact | assetId / path / machineId… | **冲突**：A 是"两条线 + 指纹"模型 |

---

## 2. 页面映射（客户端）

| A（本仓 `src/app/`） | B（`src/versions/a43/`） | 差异 |
| --- | --- | --- |
| 首页 | 首页（模拟选板） | B 的选板是模拟的 |
| 预设页 `presets/` | `presets/`（`usePresetDataA43`） | **A 是两条线 + 草稿链 + 下载四态 + SHA + 导入 + 改名/删/另存/撤销应用**；B 是"下载的包"驱动 |
| 参数页 `params/` | `params/`（`ParamCardA43` 受控参数树） | **B 的 UI 领先**（折叠统一 / 装订子卡 / 条件小签 / 动效）；A 是平铺 |
| 校准页 `calib/` | `calib/` | 两边都在早期 |
| BBS / 报告 / 设置 | 有 | 需逐页比 |
| **导入入口 `import/`** | ❌ **无** | **A 有通用文件导入（第十二层），B 没有** |
| **设置页 Bootstrap / 数据源** | ❌ 无 | **A 有（第十七刀），B 没有** |

---

## 3. 预设 / 交付业务逻辑

| 概念 | A（本仓） | B（试验台） |
| --- | --- | --- |
| 两条线 | ✅ official / mine（`origin`） | ❌ 无此概念（只有 assetId / path） |
| 血统三行 | ✅ `based_on*` 写进 TOML 头 | ❌ 无 |
| 草稿链 | ✅ `begin/put/patch/commit/discard` + `run/draft-preset.json` | ⚠️ 有包下载，无"编辑草稿"链 |
| 应用指针 | ✅ `run/active-preset.json`（带 SHA + intact） | ⚠️ localStorage `ACTIVE_KEY` |
| 下载四态 | ✅ 未下载 / 已下载 / 旧版本 / **内容异常**（SHA 校验） | ⚠️ 有"已下载/过期"，无 SHA 异常档 |
| SHA 校验 | ✅ 下载后核对 `catalog.files[].sha256` | ❌ 无 |
| 导入 | ✅ 通用入口（拖拽 + 选择器 + 重名抽屉） | ❌ 无 |
| 改名/删除/另存/撤销应用 | ✅ 全有（第十/十一层 + 本轮补的撤销应用） | ❌ 无 |
| **自动同步** | ⚠️ 第十七刀：进预设后台检查一次（`check/apply_remote_update`） | ✅ `autoSync()` + `inputsHash` + `minClientVersion`（**逻辑更细**） |

→ **B 的 `inputsHash` / `minClientVersion` 版本判定是它领先的地方**，A 那套只比 revision。**值得搬。**

---

## 4. 参数页业务逻辑

| 概念 | A（本仓 `useParams`） | B（A43 `useParamsA43` + `ParamCardA43`） |
| --- | --- | --- |
| 三层取值 | ✅ base / variant / factory（`origin`） | ✅ 同样三层 |
| 条件显隐 | ✅ `showWhen` + `blockedBy` | ✅ 有，且**折叠成条**（条件不满足 → 虚线卡头） |
| **受控参数树** | ❌ 平铺卡片 | ✅ **分支装订子卡 + 条件小签 + 竖轨/横肘** |
| **折叠统一** | ❌ 无 | ✅ A43：判据 = 条件满不满足（非"竞争支数"） |
| **写值闸** | ⚠️ 有基本校验 | ✅ 拦弃用字段 / 选项（原子拒绝 + 说人话） |
| **弃用标记** | ⚠️ `deprecated` 字段有，UI 未落地 | ✅ 行名红线 + 徽章 + 选项划线（C15/A42 两轮对齐） |
| **草稿链** | ✅ 本轮接上（patch → 草稿 TOML） | ⚠️ 内存态 |
| **修改历史** | ✅ 底座（条目自带上下文） | ❓ 需查（B 的 `HistoryModal`） |
| **交接动效** | ❌ 无 | ✅ 手风琴头（240ms，G05-1 抽卡定的案） |

→ **参数页是 B 全面领先的地方，也是"搬过来"的主战场。**

---

## 5. 假逻辑清单核对（B 的 `HANDOVER-T17.md §4` 七条 → A 是否已真）

| # | B 里的假逻辑 | A 里是否已真 | A 的位置 |
| --- | --- | --- | --- |
| 1 | 云端（`public/cloud/presets.json` + localStorage） | ✅ **真** | `runtime/net.rs` + `delivery::Source` + Bootstrap `source.json` |
| 2 | 客户端持久化（localStorage） | ✅ **真** | 文件系统 + `run/*.json`（state.rs） |
| 3 | 自动同步版本判定（`inputsHash`） | ⚠️ **部分** | A 只比 `revision`；**`inputsHash` 那套更细，未搬** |
| 4 | 预设 TOML（云端数组带内容） | ✅ **真** | 真文件下载 + catalog `path` + SHA |
| 5 | 上游注册表（`src/server/data/param_registry.json`） | ✅ **真** | `presets/registry/param_registry.toml` → catalog definition |
| 6 | mock 数据（`mockA43.ts`） | ✅ **真** | catalog 驱动 |
| 7 | 假后端 api（`src/api/mock.ts`） | ✅ **真** | Tauri IPC（`bridge.ts`） |

→ **七条里六条已真，第 3 条（`inputsHash` / `minClientVersion`）值得回头搬。**

---

## 6. 工作台对比（A `src/workbench/` vs B `src/versions/c15/`）

| | A（本仓） | B（C15） |
| --- | --- | --- |
| 命令 | `wb_*`（独立 `src/workbench/api.ts`） | 与客户端同一张 `MkpApi`（四轨并集） |
| 设置页 | ✅ `views/SettingsPage.tsx`（**官方源 Bootstrap**） | ❌ **无** |
| 桩 | 显式 `?mock=1`（PR #17） | 隐式（自动装） |
| "演示数据"徽标 / "恢复默认" | ❌ 已删（"每一笔写都是真写盘"） | ✅ 仍有 |
| 发布 | `wb_publish` → `presets/dist/` | ⚠️ `uploadToCloud` → **localStorage（假云端）** |

→ **C15 确实接近 A 的旧状态**，A 工作台**领先**（设置页 / 真发布 / 显式桩 / 去演示标记）。

---

## 7. 对齐动作清单（按"能不能自己决定"分类）

### 7.1 可自行推进（契约一致，无冲突）

1. **A 后端补 `deprecated`**（`ClientFieldDef` 字段级 + 选项级）—— B 的 UI 在等它。
2. **A 后端补 `MachineDimensions.plate`** —— 塔地图选板用。
3. **`RecipeParam.tomlKey`** —— A 已领先（①）；B 将来做字段级 patch 时也要补。

### 7.2 需要作者裁决（契约模型冲突）

1. **`UserPresetFile` vs `LocalUserFile`**：A 是"真扫盘 + 血统"，B 是"假演示集"。
   → 搬 B 的预设页过来时，**用 A 的模型**（B 的模型是假的）。**确认？**
2. **`ActivePreset` vs `AppliedPreset`**：同上，用 A 的。
3. **`MkpApi` 四轨并集 vs 纯用户端**：A 不收窄回去（工作台独立）。**确认？**

### 7.3 明确两边一起删（作者已定）

- **「同步」页**：A 已退役（第十六刀）。**B 后续也删。**

### 7.4 注定分叉（不需要对齐）

- **B 的稿号试验（A32–A43 / C01–C15 / G 轨抽卡）**：那是**工作方式**，不是产品代码。
  搬的是"**挑中的那一稿的前端**"，不是整个稿号体系。

---

## 8. 建议的下一步顺序

1. **先搬参数页 UI**（B 领先最多、价值最高）：把 A43 的受控参数树（折叠 / 子卡 / 弃用标记 / 动效）
   搬到 A 的 `src/app/params/`，**接 A 的真草稿链**（本轮刚做完的 ③）。
   - 前置：A 后端补 `deprecated`（7.1#1）。
2. **历史 UI**（本阶段剩的另一块）：可在 A 做（底座已在），或先看 B 的 `HistoryModal`。
3. **设置 → 备份与恢复**（ZIP）。
4. 之后才是「整理那一刀」。

---

## 附：待补的核对项（本次未读完）

- B 的 `HistoryModal.tsx` 与 A 的 `HistoryDrawer` 逐项对比（历史 UI 谁来搬）。
- B 的 BBS / 报告 / 校准页与 A 的逐页差异。
- B 的 `store/packageA43.ts` 全文（包管理模型）与 A 的 `store/package.ts` 逐函数对比。
