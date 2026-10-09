# 全仓持久化落点盘点（AppState 收编清单）

2026-10-08 · 计划 `APPSTATE-INVENTORY-2026-10-08` 的交付物（阶段 A 盘点 + 作者裁定）

> 这张清单盘的是「SupportEase 到底把什么记在盘上、由谁写、谁会读」，用来裁定
> 「哪些该收进 AppState」。**裁定已出（2026-10-08）：本轮零收编** —— 结论在 §2、
> 明细在 §5、后续刀在 §10；定案已并回 `docs/APP-STATE.md` 的 §5 边界与 §10。

## 0. 一页结论

1. **AppState 骨架已落地三格**（`activePreset` / `draft` / `presetSource`），旧三档
   （`run/active-preset.json` / `run/draft-preset.json` / `run/preset-source.json`）
   **写口已退役**：`runtime/state.rs` 现在只剩「数据形状 + 纯判定 + 旧档读取器」
   （迁移的读半边，`app-state.json` 不在时才用）。
2. 客户端侧「应该进、但还没进」的候选只有 3 个（`index/offsets.json`、
   `user/preset-remarks.json`、前端 7 格 localStorage）；**作者 2026-10-08 逐项裁定
   全部「不进」** —— 明细见 §2 / §5。
3. `index/offsets.json` 有个特别之处：**只写不读**（写方唯一 = `save_offsets`；
   全仓零读方，前端没有 `getOffsets`）。收编它必须同时定「读回来做什么」，
   否则只是把孤岛换个位置。
4. **写盘纪律核查通过**：客户端二进制里绕过 `fsx::atomic` 的裸写**唯一例外**是
   updater 的下载暂存 `.part`（带显式逃生口与退役条件）。另有两条**不受
   `fs::write` 禁列射程**的落盘路径（日志 writer、安装脚本 printf），登记在 §6。
5. **§5 没提到的落点 11 项**（见 §7），其中 `run/update-result.json`
   有**两条写路径**（Rust 原子写 + 安装脚本 printf）——现存缺陷，已登记。
6. 本刀**不合并** `deliveryState`（它只有信号没有数据）、**不碰**工作台
   （另一程序，identifier `SupportEase-Workbench`）。
7. **裁定结果（作者 2026-10-08）：本轮零收编** —— 表 A 三个候选**全部判「不进」**，
   `AppStateFile` 不再扩格；本刀产出只有这份清单与两份文档的定案（`APP-STATE.md`
   §5 / §10），**不动任何代码**。偏移那条另有产品方案（校准保存 = 另存为预设、
   官方那份留作基线），记在 §5.1 与 §10，另刀做。

## 1. 「收编」是什么意思（判据 + 代价）

**收进 AppState 的一格 = 同时做四件事**，缺一件就会留出"双真相"：

```text
① 落点    从各自的文件 → run/app-state.json 里一格（section，Option + serde(default)）
② 读口    唯一：Rust 走 app_state::load；前端走 appState.ts（页面不持副本）
③ 写口    唯一：命令走 app_state 的 with_state（互斥 + 整份原子写回），写完 notify_app_state
④ 退役    旧落点加进 legacy_files()，惰性迁移（首次写时并进来、写成功删旧文件）
```

**判据（本清单用的）**——三条都满足才建议「进」：

1. **跨重启要记住**；
2. 是「应用/用户的**当前选择**」，不是数据内容 / 历史记录 / 一过性结果；
3. 有一个**明确的"当前值"**（单值或小结构），**不是随时间增长的账**。

按这三条：`offsets` 三条全中；`preset-remarks` 卡在第 3 条（随文件数增长的映射账）；
localStorage 那 7 格卡在"窗口级 UI，非应用级"（见 §5.3）。

## 2. 表 A —— 裁定结果（2026-10-08，已定）

| # | 落点 | 裁定 | 依据 / 后续 |
| --- | --- | --- | --- |
| 1 | `index/offsets.json`（三轴校准偏移） | **不进**（原"待定"改判） | 形态另有产品方案（校准保存 = 另存为预设、官方那份留作基线，见 §5.1）；**本刀不动它**，方案落地时该落点退役 |
| 2 | `user/preset-remarks.json`（备注覆盖账） | **不进** | 用户数据的伴生账（与 `provenance.json` 同族），读写已唯一；本刀不碰 |
| 3 | 前端 7 格 localStorage | **不进** | 窗口级 UI 偏好、单页面消费、不需跨进程广播；本刀不碰 |
| 4 | 其余全部（§4 全表） | **不进**（作者无异议） | 历史 / 业务数据 / 缓存 / 更新机制 / 工作台 |

**本轮结论：零收编** —— `AppStateFile` 不扩格、没有任何落点搬家；本刀的产出就是这份
清单与两份文档的定案（`APP-STATE.md` §5 / §10），**不动代码**。旧三档的**读半边保留**
（迁移窗口未过），退役条件记在 §10。

## 3. 已收编（3 格，完成）—— 现状核对

| 格 | 落点 | 写口（唯一） | 读口 | 现状 |
| --- | --- | --- | --- | --- |
| `activePreset`（使用中指针） | `run/app-state.json` | `runtime/app_state.rs` 的 `with_state` 写口族；命令 `ipc/catalog.rs`（应用/清除） | `app_state::load` ← `ipc/mine.rs` / `ipc/catalog.rs` / `runtime/source.rs`；前端 `appState.ts` `useActivePreset` | ✅ 收编完成 |
| `draft`（编辑中那一份） | 同上 | `app_state.rs` 写口；命令 `ipc/mine.rs` | `app_state::load` ← `ipc/mine.rs` | ✅ 收编完成 |
| `presetSource`（数据源设置） | 同上 | `app_state.rs` 写口；命令 `ipc/catalog.rs` | `runtime/source.rs` 读 | ✅ 收编完成 |

旧三档的状态：**生产写口已退役**（`runtime/state.rs` 里不再有写路径），
读半边 `read_active_file` / `read_draft_file` 只在 `app-state.json` 不存在时被
`legacy_files()` 调用（迁移兼容）；首次写成功后旧文件被删。

## 4. 已判「不进」（依据充分）—— 全表

### 4.1 客户端 Rust（内部根 = `appDataDir`，用户根 = `appDataDir/user`）

| 落点 | 性质 | 依据 |
| --- | --- | --- |
| `catalog.json` + `archive/**`（单槽归档、版本链、交付归档） | 业务数据 / 历史 | §5 明确不进；目录链全体在读 |
| `<catalog.path>` 交付文件（`delivery/mkp/**`、`assets/**`）、`user/presets-mine/**` | 业务数据（文件正文） | §5 明确不进（"AppState 只存指针与状态，不存内容"） |
| `preset_events.json`（下载/替换事件账） | 历史记录 | §5 点名不进 |
| `user/provenance.json`（出处账） | 历史记录 | §5 点名不进 |
| `logs/supportease.log.<date>`（tracing rolling） | 日志 | §5 点名不进；另见 §6 登记 |
| `run/update/`（升级暂存）、`run/update-result.json`（上次安装结果） | 更新机制 / 一过性结果 | 见 §5.4；且有双写路径缺陷 |
| `cloud/`、`user/exports/`、`user/reports/` | 预建空目录（零写方） | 无数据可收；盘上存在但没人写 |

### 4.2 前端（WebView 存储 + 内存）

| 落点 | 性质 | 依据 |
| --- | --- | --- |
| 7 格 localStorage（§5.3 表） | 纯 UI 偏好 | 待你勾（建议不进） |
| `useSessionState`（模块级 Map，不落盘） | 会话态 | 不跨重启、无事件、无原子写 —— 恰是 AppState 的反面；建议在 §5 里**明确写成"不进"** |
| `deliveryState.ts`（内存代次） | 通知信号 | 只有"要不要重读"，不是状态容器；将来收口时把代次并进 AppState、消费者不变（本刀不合并） |

### 4.3 工作台（另一程序，identifier `SupportEase-Workbench`）—— 只列不碰

| 落点 | 性质 |
| --- | --- |
| `<WB appDataDir>/publish-account.json`（平台地址/用户名，无 token） | 配置 |
| `<WB appDataDir>/publish-history.json`（预设发布回执 ≤100 条） | 历史 |
| `<WB appDataDir>/release-history.json`（软件版发布回执） | 历史 |
| `<WB appDataDir>/credentials.json`（token，chmod 0600） | 秘密 |
| `workbench/delivery.json`（菜单可见性 + 套餐） | 业务数据 |
| `workbench/built.json`（生成台账） | 历史 |
| `workbench/.draft/book.json`（草稿崩溃快照，真相在内存） | 会话草稿 |
| `workbench/.draft/ui.json`（界面状态：折叠/页签/勾选） | UI 偏好（落**仓库**，不是 appDataDir） |
| `workbench/bootstrap.json`（官方源地址，入库） | 配置（构建期被 `build.rs` 读） |
| `workbench/.trash/*`、`.snapshots/*`（回收站 / 快照） | 历史 |
| `presets/machines/*.toml`、`forbidden_zones/*.toml`、`presets/registry/param_registry.toml`、`presets/assets.toml`、`presets/bundles.toml` | 业务数据（仓库源文件） |
| `presets/delivery/**`（content JSON、mkp/**、manifest.json、source.json、catalog.json、release.json） | 发布产物 |
| `src-tauri/Cargo.toml`、`package.json`、`src-tauri/tauri.conf.json`、`Cargo.lock`（版本号真值 + 三派生） | 发布机制 |
| `.git/**`（提交/推送/tag） | 版本控制 |

## 5. 裁定明细（含收编含义）

### 5.1 `index/offsets.json`（裁定：**不进**，本刀不碰）

**现状**（已核实）：

- 内容：三轴校准偏移 `Axes`（z / xy 结构，`ipc/mod.rs` 里的类型）；**没有 schema 字段**；
- **写方唯一**：`src-tauri/src/ipc/mod.rs:113` `save_offsets`（`resolve(&app, Root::Internal, "index/offsets.json")` + `atomic_write_json`，注释自称"原子写的第一个真实调用点"）；
  前端链路：`bridge.ts` → `useCalibration.ts` 的 `commitAll`；
- **读方为零**：Rust 无、前端无（没有 `getOffsets`）；
- 校准页的初值**另有来源**：`preset.preset.axes`（预设就位时带来的那份），
  `useCalibration.ts:55` 有明确注释「不能拿 `mkpFull.offsets` 顶 —— 那是上一台机器的数」。

**作者给的方案（2026-10-08，另刀做）**——他讲的比"收不收进 AppState"大一层，是
**保存形态**：

> 「偏移是保存进 toml 里面的……我觉得和编辑的动作一样就可以，另存为一份新的，下次再
> 编辑就用这一份编辑过的。其实我有另外的方案，就是默认就是另存为，官方那一份下载下来
> 就是不动它的（可能可以改后缀，放进一个文件夹），他那一份只是留下来和云端对比有没有
> 新版的，就像是一份**基线**一样。」

| 案 | 校准保存做什么 | 官方那份 |
| --- | --- | --- |
| A | 覆盖官方那份的偏移（就地改） | 被改 |
| B（作者倾向） | **另存为用户线一份新的**（与"编辑预设"同一个动作） | **不动**（改后缀 / 放一个文件夹），留作**基线**，与云端比对有没有新版 |

这条线**天生在 AppState 纪律之下**：不管 A 还是 B，"另存为一份新预设"这个动作都要经
AppState（改 `activePreset` 那一格），**不需要把三个偏移数字塞进状态文件**。
方案落地时，`index/offsets.json` 这个落点**直接退役**（`save_offsets` 一并改道）。

**（原"若判定进"的动作清单，留档备用）**：

1. `AppStateFile` 加一格（如 `calibrationOffsets: Option<Axes>`，加 section 不升号）；
2. `save_offsets` 命令改走 `app_state::with_state` + 写完 `notify_app_state`；
3. **同时定读路径**（本条是前提）：要么给校准页初值换成 AppState 读、要么明确"这一格
   只作留档"并在注释里写清 —— 收编不许留孤岛；
4. `legacy_files()` 加 `index/offsets.json`（惰性迁移，与旧三档同模式）。
   附带：`Root` / `resolve` 这套"哪个根"的抽象目前**全仓只有 offsets 这一处**
   在用（其余一律直接 `internal_root` / `user_root`），收编后它归零 —— 一并登记即可，
   不在本刀强行动它。

### 5.2 `user/preset-remarks.json`（裁定：**不进**，本刀不碰）

**现状**（已核实）：

- 内容：备注覆盖账 `{文件身份 → 用户改过的副标题}`；**没有 schema 字段**；住**用户根**；
- 写方：`runtime/remarks.rs`（`set` / `remove`）；命令 `set_preset_remark`；
  前端 `usePresetData.ts`；
- 读方：`runtime/remarks.rs` 的 `load`；命令 `get_preset_remarks`；前端
  `presetTree.ts`（**只有本地表读它** —— 云端表只读工作台写的那句，见
  `docs/REMARK-SUBTITLE-2026-10-07.md`）；
- 删除文件时后端把键一起清掉（`delete_delivered` 链路），**跟随文件生命周期**。

**为什么判「不进」**（作者 2026-10-08 确认）——它与 `provenance.json` 是双胞胎
（同根、同族、"当空表"坏档策略）：都是**用户数据的伴生账**（随文件数增长、删文件清键），
不是"应用当前状态"。§5 已判 provenance 不进，备注保持同一口径；**不动它** ——
继续住 `user/preset-remarks.json`，读写仍走那两个命令（本就唯一口，不散落）。

**（原"若判定进"的动作，留档备用）**：落点搬 `run/app-state.json` 一格；读写命令改走
`app_state` + `notify_app_state`；`legacy_files()` 加它；删除链路改调 `app_state` 的清键。

### 5.3 前端 7 格 localStorage（裁定：**不进**，本刀不碰）

| 键 | 用途 | 读写点 | 默认 |
| --- | --- | --- | --- |
| `mkp.A40.presets.pinned` | 预设页置顶集合（排序偏好） | `usePresetData.ts` | 空集合 |
| `mkp.A40.params.searchHistory` | 参数页最近搜索词 | `PageParams.tsx` → `useStickyState.ts` | `[]` |
| `mkp.A40.params.historyDrawerW` | 修改历史抽屉宽度 | `PageParams.tsx` → `useDrawerWidth.ts` | 380（280–560） |
| `mkp.A40.bbs.view` | BBS 显示档位 | `PageBbs.tsx` | `'all'` |
| `mkp.A40.bbs.light` | BBS 主题 | `PageBbs.tsx` | `false` |
| `mkp.A40.bbs.drawerMode` | 抽屉并排/浮层 | `PageBbs.tsx` | 宽档 `'side'` |
| `mkp.A40.bbs.drawerW` | BBS 抽屉宽度 | `useBbsDrawer.ts` | 280（200–520） |

（`storageKeys.ts` 文件头已声明"留下的全是纯前端偏好，localStorage 不再承载任何底账"。）

**若判定「进」，代价**：前端目前**没有写入口**（`appState.ts` 是只读客户端），
要新增 7 条 `set_*` 命令 + 事件回流；收益是这些偏好在"换浏览器内核/清 WebView 存储"后
仍在。**建议不进**：它们是**窗口级 UI**（谁看这个窗口就长这样），不是应用级偏好；
§5 里的"用户偏好"宜收窄解释为应用级（如"默认机型"这类）。

### 5.4 其余建议「不进」的详情（登记）

- **`run/update-result.json`**：写路径**两条** —— Rust `updater.rs` 的 `write_result`（原子）
  与安装脚本 `printf`（`:285-289`，进程退出后写）。语义是"上次安装的一次性结果"，
  下一次启动读一次；AppState 没有"读一次即清除"的机制，硬收会引入"何时清格"的新规则。
  **不成格，但两条写路径的缺陷值得另刀合并。**
- **`run/update/` 暂存**：升级下载/解压的中间态，会话级（下次启动清）——缓存性质。
- **`logs/`**：tracing rolling daily；见 §6 登记。
- **`cloud/`、`user/exports/`、`user/reports/`**：预建空目录、零写方 —— 无内容可收；
  盘点文档里显式登记，避免被误当成"有数据的落点"。

## 6. 写盘纪律核查（有没有绕过原子写的裸写）

**客户端二进制（用户机器上跑的代码）**：

| # | 位置 | 写什么 | 状态 |
| --- | --- | --- | --- |
| 1 | `runtime/updater.rs:415-417` `File::create` | 下载暂存 `run/update/*.part` | **唯一受控例外**：`#[allow(clippy::disallowed_methods)]` + 退役条件注释（改成 `.part → atomic rename` 后撤除） |
| 2 | `runtime/updater.rs:285-289`（安装脚本 `printf`） | `run/update-result.json` | **不受 `fs::write` 禁列射程**（脚本进程），登记为"同文件第二条写路径" |
| 3 | `obs/tracing.rs:36-44` `tracing_appender` rolling | `logs/` | **不受禁列射程**（第三方 writer），登记为"日志受控例外" —— 建议在纪律文本里显式豁免它 |

**非客户端（仓库内开发工具，不随安装包走）**：`crates/postprocess`、`crates/preset`、
`tools/assets/sync.mjs`、`scripts/probes/*`（只 mkdir 截图目录）、测试夹具里的
`std::fs::write` —— 均为构建/工具链产物，与"应用状态"无关。

**结论**：本刀**不需要补原子性**，只处理"谁有权写状态"。纪律文本（`runtime/state.rs`
模块头 + `docs/ARCHITECTURE.md`）建议补一条"日志 writer 豁免"。

## 7. §5 未提到的落点（盘点新发现）

1. `user/preset-remarks.json`（与不进族的 provenance 双胞胎，最需要你定性，见 §5.2）；
2. `run/update-result.json` + `run/update/`（更新机制，含双写缺陷）；
3. `logs/`（登记）；
4. `cloud/`、`user/exports/`、`user/reports/`（空目录）；
5. `catalog.json` + `archive/**`（§5 只写了"业务数据"，未点名归档链）；
6. 前端 7 格 localStorage + 会话态 `useSessionState`（§5 没写前端存储）；
7. `deliveryState.ts`（内存代次，自认"将来收进 AppState"）；
8. 工作台全部落点（§5 只谈客户端）；
9. `runtime/catalog.generated.json`（入库构建产物）；
10. 工作台 `bootstrap.json`（入库配置）与 `.draft/ui.json`（UI 状态落仓库）；
11. `presets/delivery/release.json`（软件更新链，客户端 `runtime/release_info.rs` 读）。

## 8. 意外发现 / 风险（登记在案，不属本刀范围）

1. **`Root` / `resolve` 几乎死代码**：全仓只有 `ipc/mod.rs:115` 一处使用
   （正是 offsets 那处），`Root::User` 零使用；"落点必须声明哪个根"的设计意图
   没有在代码里铺开。
2. **offsets 只写不读**：现阶段它的"持久化"没有消费者（页面初值来自预设自带的
   axes）—— 收编前要先答"读回来做什么"。
3. **三个预建空目录**会出现在用户盘上，盘点时别误当有数据。
4. **`update-result.json` 两条写路径**（Rust + shell）——"同一份数据两条真相"的现存实例。
5. **三本账的坏档策略互不相同**：`preset_events` / `provenance` / `remarks` 一律
   "当空表"；`catalog` / `app-state` / 工作台四档一律 `CORRUPTED`。收编任何一本账前
   先定它归哪一档。
6. **工作台草稿的真相在内存，盘上只是崩溃快照**（2s 懒写）——与 AppState"盘是底账"
   正相反；将来若统一规则，这条要先裁决。
7. **"UI 状态"有三份**：客户端 localStorage（7 格）、`workbench/.draft/ui.json`（落仓库）、
   `src/workbench/views/MachinesPage.tsx` 的会话内存（明确不落盘）—— 三份别混着数。
8. **日志 writer 未登记进纪律文本**（见 §6 结论）。
9. **工作台 `version.rs` 会改仓库 4 个文件**（版本号真值 + 三派生），其中 `Cargo.lock`
   另有一条 `cargo update --workspace --offline` 的外部写路径 —— 工作台盘点若要"写方全集"，
   这一条不能漏。

## 9. 勾完之后（阶段 B）—— 本轮实际结果：零收编

表 A 三个候选全判「不进」，所以原本列的收编六步**本轮一步都不做**（没有 schema 改动、
没有落点搬家、没有命令改道、没有代码改动）；阶段 B 的实际动作只有两件：

1. **文档定案**：本文档补裁定结果（§2 / §5 / §10）；`docs/APP-STATE.md` 的 §5 边界
   逐项改判 + 文末新增 §10（盘点结论与后续刀）；
2. **验证与提交**：确认工作区**只有文档改动**（`git status` 可证），提交并推送
   `feat/build-delivery-files`。零代码改动 ⇒ 不跑构建 / 探针（它们的行为不受 `.md` 影响）。

**（原收编六步，留档备用 —— 将来若真有落点判「进」，照它执行）**：

1. **schema 定稿**：扩 `AppStateFile`（一律 `Option + serde(default)`，加 section 不升号），
   每格写明语义；`APP-STATE.md` §2 骨架同步；
2. **后端**：`runtime/app_state.rs` 增读写操作（复用 `with_state`：互斥 + 整份原子写回）；
   直写旧落点的命令改走 AppState 且写完 `notify_app_state`；**读命令同改成对**；
3. **前端**：`appState.ts` 唯一客户端扩格（整份快照 + 派生 hook + 写口 +
   `appStateMutated()` 兜底）；`contract.ts` / `bridge.ts` / `mock.ts` 三件套同形；
4. **退役**：旧落点进 `legacy_files()` 惰性迁移，写成功后删旧文件；旧常量与注释同步退役；
5. **文档**：`APP-STATE.md` §5 / §6 / §8 与本文档同步；
6. **验证**：`cargo fmt` + 双 feature `clippy -D warnings` + `cargo test`（补 `app_state`
   单测：加格旧文件照读 / 惰性迁移退役 / 坏档 CORRUPTED / 并发写不踩格）+ `tsc -b` +
   `build` + 三个 check + `lint` + 预设页与首页探针。

## 10. 后续刀（本轮登记，不在本刀范围）

| # | 刀 | 缘起 | 落点 |
| --- | --- | --- | --- |
| 1 | **校准保存 = 另存为用户线预设**（官方那份退居"基线"） | §5.1 作者方案（A/B 待定） | 落成时 `index/offsets.json` + `save_offsets` 一并退役；"另存"动作本身经 AppState 改 `activePreset` |
| 2 | **`run/update-result.json` 两条写路径合并成一条** | §8 风险 4（Rust 原子写 + 安装脚本 `printf`） | `runtime/updater.rs` |
| 3 | **旧三档读半边退役** | 迁移窗口未过（盘上可能还有没迁过的老档） | `runtime/state.rs` 的 `read_active_file` / `read_draft_file` 与 `legacy_files()` |
| 4 | **工作台那一族收口**（若要做） | 本刀只盘了客户端的落点，工作台 15 类只列不碰 | 另起一份清单（另一程序，identifier `SupportEase-Workbench`） |
