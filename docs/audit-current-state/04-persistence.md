# 04 · 副作用、持久化与一致性审计

> 本层的目标是找出"数据到底怎么变成真实文件的"。所有写盘入口来自对全仓 `std::fs::*` 与 `atomic_write` 调用点的搜索 + 逐文件核对。

## 0. 统一写盘出口

| 项 | 结论 | 证据 |
| --- | --- | --- |
| 唯一写**内容**的出口 | `fsx/atomic.rs::atomic_write::28`：同目录 `NamedTempFile` → `write_all` → `sync_all` → `persist`（同分区 rename）→ 父目录 `sync_all` | `src-tauri/src/fsx/atomic.rs::28` |
| 同字节优化 | 内容完全相同则**整份跳过**（连 mtime 不动）—— 有测试钉住 | `src-tauri/src/fsx/atomic.rs::110` |
| 平台 | Unix 原子；Windows 走 `MoveFileEx`（tempfile 语义），同目录保证同卷、不退化 copy | 同上 |
| 失败语义 | **写失败不碰目标**（目标保持旧份） | 测试 `a_failed_write_touches_nothing` |
| 不覆盖的操作 | 删除 / 改名 / 建目录按设计直调 `std::fs`（`atomic_write` 表达不了这些语义） | 见 §2 与 §1.4 |

## 1. 写盘入口清单（23 条）

### 1.1 客户端默认构建（`feature="workbench"` 关闭时编入）

| id | 代码位置 | 目标 | 方式 | 触发 | 失败处理 |
| --- | --- | --- | --- | --- | --- |
| W-01 | `runtime/app_state.rs::with_state::194` | `run/app-state.json` | 原子写 JSON | 应用/撤用/存草稿/改名跟随/数据源/参数决定 | 写失败不写不删旧文件 |
| W-02 | `runtime/app_state.rs::with_state::196` | `run/active-preset.json` / `draft-preset.json` / `preset-source.json` | **直接 remove_file** | 首次写成功后一次性退役旧档 | 删失败仅 warn |
| W-03 | `runtime/mine.rs::commit_draft::411` | `user/presets-mine/<原名>（已修改）.ext` | 原子写 | `commitPresetDraft`（官方线另存） | 返回错误，原件不动 |
| W-04 | `runtime/mine.rs::save_back::444` | `presets-mine/<rel>`（原路径） | 原子写 | `commitPresetDraft`（用户线写回） | 文件不在 → NotFound，**不去别处新建** |
| W-05 | `runtime/mine.rs::set_machine_variant::478` | `presets-mine/<rel>` | 原子写 | `setUserPresetMachineVersion` | NotFound |
| W-06 | `runtime/mine.rs::write_text::527` | `presets-mine/<rel>` | 原子写 | `savePresetCalibration` / `savePresetParams` / `applyPresetParamDecisions` | NotFound |
| W-07 | `runtime/mine.rs::rename_file::681` | 同目录新名 | **`std::fs::rename`** | `renameUserPreset` | 目标已存在 → 拒绝（不覆盖） |
| W-08 | `runtime/mine.rs::copy_as_new::731` | `presets-mine/<新名>` | 原子写（先 `fs::read` 源） | `copyUserPreset` | 撞名 → InvalidArgument |
| W-09 | `runtime/mine.rs::save_official_as_new::777` | `presets-mine/<原名>` | 原子写 | `fetchOfficialPreset` | 撞名 → 拒绝 |
| W-10 | `runtime/mine.rs::delete_file::799` | `presets-mine/<rel>` | **`std::fs::remove_file`** | `deleteUserPreset` | 真删，无垃圾桶 |
| W-11 | `runtime/delivery.rs::deliver::229` | `archive/<catalog.path>` | 原子写（旧份归档） | 下载 / 更新 | 槽位已存在则跳过（保留最早） |
| W-12 | `runtime/delivery.rs::deliver::236` | `<appDataDir>/<catalog.path>` | 原子写（新份落点） | `downloadCatalogFile(s)` / `fetchOfficialPreset` | SHA/大小不过 → **整个拒绝、不碰盘** |
| W-13 | `runtime/delivery.rs::remove_file_idempotent::573` | `<catalog.path>` / `archive/<path>` | **`std::fs::remove_file`** | `deleteDeliveryFile` / `deleteArchivedFile` | 不存在＝成功（幂等） |
| W-14 | `runtime/baseline.rs::ensure_baseline::99` | `baseline/<sha256>.toml` | 原子写 | deliver 收尾 / param_sync 读侧 | 落不上仅告警 |
| W-15 | `runtime/release.rs::ensure_released::109` | `catalog.json` | 原子写 | **启动**（`lib.rs::setup::86`） | 盘上已有 → 一个字节不动 |
| W-16 | `runtime/release.rs::release_bytes::161/166/169` | `archive/catalogs/<rev>.json` / `archive/catalog.json` / `catalog.json` | 原子写 ×3 | `applyRemoteUpdate` | 先解析后写；解析不过不碰盘 |
| W-17 | `runtime/preset_events.rs::save::200` | `preset_events.json` | 原子写 | deliver / bootstrap init | 追加失败仅告警 |
| W-18 | `runtime/provenance.rs::save::134` | `user/provenance.json` | 原子写 | `copyUserPreset` / `commitImport` / `renameUserPreset` | 记不上不影响主流程 |
| W-19 | `runtime/remarks.rs::save::94` | `user/preset-remarks.json` | 原子写 | `setPresetRemark` / 删除连带 | 同族 |
| W-20 | `runtime/import.rs::commit_one::237` | `presets-mine/<name>` | 原子写 | `commitImport` | 撞名/源丢 → 逐份 fail |
| W-21 | `fsx/paths.rs::ensure_dirs::64` | `cloud/ archive/ logs/ run/` 与 `user/{exports,reports,presets-mine}` | `create_dir_all` | 每次 `internal_root` / `user_root` | 建不出 → `AppError::io` |

### 1.2 软件自更新（命令仅在 workbench 构建注册）

| id | 代码位置 | 目标 | 方式 | 失败处理 |
| --- | --- | --- | --- | --- |
| W-22 | `runtime/updater.rs::download_into::416` + `write_result::263` | `update/<asset>.part` → 改名；`run/update-result.json` | **流式 `File::create` + rename**（刻意不走 atomic，几十 MB；代码里标了退役条件） | `.part` 失败即删；**无启动清理** |

### 1.3 工作台（默认构建**不编译**）

`workbench/store.rs::write_doc::103`、`sandbox.rs`（`fs::copy` / `remove_dir_all`）、`app/delivery.rs::705`（rename）等。`feature="workbench"` 门内，客户端二进制搜不到。

### 1.4 绕过统一出口的直接写

- **内容写**：客户端运行时**没有**绕过 `atomic_write` 直接写文件内容的地方（唯一例外是自更新的流式 `File::create`，但它自带 `.part` + rename，等价安全）。
- **天然不走 atomic 的**（按设计）：删除（`remove_file`）、改名（`rename`）、建目录（`create_dir_all`）、工作台 copy / `remove_dir_all`。
- **纪律覆盖缺口（重要）**：
  1. `crates/preset/tests/write_discipline_scan.rs` 的 `FORBIDDEN` 清单只扫 `crates/preset/src` 与 `crates/postprocess/src`（`::162`），**完全不扫 `src-tauri`**。
  2. `clippy.toml` 只管 `fs::write` / `File::create` / `OpenOptions`（且**刻意不禁** `File::create_new`），**不管 `remove_file` / `rename` / `copy` / `remove_dir_all`**。
  3. 结果：`src-tauri` 里的删除/改名/复制**没有任何源码扫描兜底**，只靠人工与 code review（X-17）。

## 2. 持久化载体清单（18 个）

| id | 路径 | 格式 | 写者 | 读者 | 清者 | 权威 or 缓存 | 能否从盘重建 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PS-01 | `<内部>/catalog.json` | JSON | `ensure_released`（铺底）、`release_bytes`（OTA） | 全部读命令 | 手删 | **权威** | 是（可重下） | 已证实 |
| PS-02 | `<内部>/archive/catalogs/<rev>.json` | JSON | `release_bytes` | `official_versions` / `generation_index` / `trust` | **无** | 历史记忆 | **否**（中间版指纹唯一来源） | 已证实 |
| PS-03 | `<内部>/archive/catalog.json` | JSON | `release_bytes`（单槽保最早） | `other_known_versions` | **无** | 历史 | 否 | 已证实 |
| PS-04 | `<内部>/<catalog.path>` | 真字节 | `deliver` | 读命令 | `delete_downloaded` | **权威（盘即底账）** | 是（SHA 锚定） | 已证实 |
| PS-05 | `<内部>/archive/<catalog.path>` | 真字节 | `deliver`（is_update） | `archived_files` | `delete_archived` | 历史 | 否（云端只有最新版） | 已证实 |
| PS-06 | `<内部>/baseline/<sha256>.toml` | TOML | `ensure_baseline` | `param_sync` / `preset_baseline` | **无 GC** | **权威（官方旧值）** | 可（按内容重下） | 已证实 |
| PS-07 | `run/app-state.json#activePreset` | JSON | `set_active_*` | `appState.ts` | `clear_*` / 删文件连带 | **权威（指针）** | 否（用户选择） | 已证实 |
| PS-08 | `run/app-state.json#draft` | JSON（含正文全文） | `set_draft` | `draft()` | `clear_draft` / 删文件 | 权威（临时账） | 否（未保存改动） | 已证实 |
| PS-09 | `run/app-state.json#presetSource` | JSON | `set_preset_source` | `current_mode` / `entry` | `clear_preset_source` | 权威 | 否 | 已证实 |
| PS-10 | `run/app-state.json#paramDecisions` | JSON map | `record_param_decisions` | `param_decisions` | `forget_param_decisions` / 改名跟随 | **权威（水位）** | 否 | 已证实 |
| PS-11 | `<内部>/preset_events.json` | JSON | `append` / `init`（bootstrap） | `load_events` | **无（只追加）** | **权威（业务时间唯一来源）** | **否**（bootstrap 从 mtime 补，有损） | 已证实 |
| PS-12 | `user/provenance.json` | JSON | `record` / `repath` | `load` | 改名跟随（悬空保留） | 权威（唯一记"从哪复制/导入"） | 否 | 已证实 |
| PS-13 | `user/preset-remarks.json` | JSON | `set` / `remove` | `load` | `remove` / 删文件连带 | 权威（覆盖层） | 否（可退回工作台值） | 已证实 |
| PS-14 | `user/presets-mine/**.toml` | TOML + 文件头 | §1.1 各写点 | `mine_files` 与各读 | `delete_file` | **权威（盘即底账）** | 否（用户自己的文件） | 已证实 |
| PS-15 | `user/exports`、`user/reports` | — | 仅建目录 | — | — | 占位（现无写者） | — | 已证实 |
| PS-16 | `run/update-result.json` | JSON | 更新脚本 / `write_result` | 下次启动 | — | 权威 | 否 | 已证实 |
| PS-17 | `<内部>/logs/` | tracing | `obs::tracing` | 人 | 无 | 缓存 | 是 | 已证实 |
| PS-18 | `run/{active-preset,draft-preset,preset-source}.json`（**已退役**） | JSON | （迁移时删） | （读侧兜底） | 首次写成功后删 | 已退役 | — | 已证实 |

> **能完全从磁盘重建的**：catalog、交付件、baseline、logs。
> **不可重建（历史或用户选择）**：`preset_events.json`、`archive/catalogs/`、`provenance.json`、`preset-remarks.json`、`app-state.json#paramDecisions`、`#activePreset`、`#draft`。

## 3. localStorage 与会话态

`src/api/storageKeys.ts` 共 **7 个键**，**全部是纯前端偏好，无一是底账**（文件头明写）：

| 键 | 语义 | 写入点 | 承载底账？ |
| --- | --- | --- | --- |
| `mkp.A40.presets.pinned` | 预设置顶集合 | `usePresetData.ts::1021` | 否 |
| `mkp.A40.params.searchHistory` | 参数页最近搜索词 | `PageParams.tsx::183` | 否 |
| `mkp.A40.params.historyDrawerW` | 历史抽屉宽度 | `PageParams.tsx::409` | 否 |
| `mkp.A40.bbs.view` | 看全部 / 跟 BBS 一样 | `PageBbs.tsx::65` | 否 |
| `mkp.A40.bbs.light` | 皮肤深浅 | `PageBbs.tsx::66` | 否 |
| `mkp.A40.bbs.drawerMode` | 抽屉并排 / 浮层 | `PageBbs.tsx::95` | 否 |
| `mkp.A40.bbs.drawerW` | BBS 抽屉宽度 | `useBbsDrawer.ts::22` | 否 |

- 持久化机制：`src/hooks/useStickyState.ts`（localStorage + JSON，坏值/无痕静默退回内存）；所有值**一律 JSON 编码**（仓库约定）。
- **越界键**：`src/workbench/c14/useSplitWidth.ts` 用自己的裸键（不在 `storageKeys.ts`），工作台专用。
- `src/app/shared/useSessionState.ts`：**模块作用域内存 Map，不落 localStorage、刷新即丢**，只在同一会话内切 tab 保留（`presets.query/nozzle/layer`、`bbs.target/query/drawer/paramQuery`）。存的是"这次会话我看到哪"，**不存草稿或未保存改动**。

## 4. 启动初始化

`lib.rs::setup::59-125` 只做四件事：

1. 建数据根与日志目录（失败不阻断，日志退 stderr）；
2. `runtime::release::ensure_released`（catalog **只在盘上没有时**铺随包那份 —— OTA 成果绝不被启动覆盖；有源码扫描判据钉死"启动只许调 ensure_released"）；
3. macOS 窗口外观（仅客户端窗口）；
4. 开工作台窗口（仅 `feature="workbench"`）。

**没有**：核对草稿引用的文件是否还在、`app-state` 与盘对账、清理临时文件、事件账重建校验。
→ **启动不做一致性修复**；不一致靠各读命令按需发现（`active_matches_disk`、`stale_files`、`FileTrust`）。

## 5. 一致性审计（逐条判定）

| id | 事项 | 存在？ | 说明 | 证据 | 等级 |
| --- | --- | --- | --- | --- | --- |
| CO-01 | 同一业务状态被多处维护 | **是** | "本机已下载官方文件"有真值（盘）与桩（`get_local_files` 恒空）两套；预设页用后者、release 行用前者 | `presets.rs::812`、`delivery.rs::834` | 已发现冲突 |
| CO-02 | 写文件成功但状态/账未跟上 | **是** | 改名：文件已改名、state/provenance 未跟 → 后端**明确告知**"文件其实已改名"；`commit_draft`：清草稿失败 → 残留草稿（与"已保存"语义不符） | `mine.rs::502`、`::464` | 部分证实 |
| CO-03 | 状态成功但写盘失败 | **否** | `with_state` 先写盘成功才返回；广播是 `let _ =`（失败只是不自动刷新） | `app_state.rs::194` | 已证实 |
| CO-04 | 重复执行产生重复数据 | **否** | 下载同字节 no-op；应用整份替换；另存/写回覆盖自己；复制/导入/改名撞名一律拒绝；出处/备注同键替换；参数决定同项覆盖 | `delivery.rs::184`、`provenance.rs::85` | 已证实 |
| CO-05 | 中途关闭留下什么 | **是** | 草稿（设计如此）；**硬崩溃/断电**可能在目标目录留一个随机名临时文件；自更新崩溃留 `.part` 与暂存目录且**无启动清理** | `atomic.rs::28`、`updater.rs` | 部分证实 |
| CO-06 | 启动修复逻辑 | **极简** | 只有 `ensure_released` 铺目录 | `lib.rs::85` | 已证实 |
| CO-07 | 可从盘重建 vs 依赖不可重建历史 | **是** | 见 §2 末尾两行 | — | 已证实 |
| CO-08 | 多步操作的半完成风险点 | **是** | 见 §6 | — | 部分证实 |

## 6. 多步操作的"半完成"风险点（9 个）

| 流程 | 步骤 | 中断后果 |
| --- | --- | --- |
| `applyActivePreset` | 校验 → 单次原子写 | **低**（一次原子写，无多步） |
| `commitPresetDraft` | 写文件 → 清草稿（两次写） | 文件已存、草稿未清 → 残留草稿 |
| `renameUserPreset` | rename → 改 state → 改 provenance | 悬空指针 / 出处（**已告知**"文件其实已改名"） |
| `deleteUserPreset` | 撤指针 → 清草稿 → 删文件 → 删备注 → 忘决策账 | 删文件后 remark/决策账未清 → 悬空账 |
| `deleteDeliveryFile` | 撤指针 → 清草稿 → 删文件 → 删备注 | 同族；删文件失败时指针已撤 |
| `fetchOfficialPreset` | 取回官方字节 → 落工作副本 | 官方字节已到、工作副本未落 → **重试即补（幂等）** |
| `applyRemoteUpdate` | 结构判定 → 三次原子写（链 / 单槽 / catalog） | 链已加而 catalog 未换 → 下次重跑补；**读不懂则在写之前整次拒绝** |
| `applyPresetParamDecisions` | 写文件（adopt）→ 记水位 | 文件已改、水位未推 → 下次仍显示待处理（**反向已防**：值写失败不记水位） |
| `deliver`（单份） | fetch → 校验 → 归档旧份 → 写新份 → baseline → 事件 | 归档已写、新份未写 → 目标仍旧、归档多一份（重跑幂等，槽位不覆盖） |

> 结论：**客户端的内容写已全部收口到 `atomic_write`**（唯一例外是自更新的流式写，自带 `.part` + rename 且标了退役条件）。真正的短板不是"没有事务"，而是
> ① 写盘纪律的源码扫描不覆盖 `src-tauri`（删除/改名/复制无兜底）；
> ② 有些多步序列把"文件变了但账没变"的风险留给重试与告知，且**没有启动修复**。
> 这两条在重写时分别对应 `RD-16` 与 `RM-03`。
