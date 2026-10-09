# 03 · 契约层：命令面、DTO 与前后端边界

> 唯一定义在 `src/api/contract.ts`（`MkpApi`，65 个方法）；桥在 `src/api/bridge.ts`；浏览器回退在 `src/api/mock.ts`。
> Rust 侧命令在 `src-tauri/src/ipc/*`，注册在 `src-tauri/src/lib.rs` 的**两份** `generate_handler!`。

## 0. 小结

| 项 | 值 |
| --- | --- |
| 契约方法 | **65** |
| IPC 命令 | **62** |
| 非命令 | 3（`pickImportFiles` 走 plugin-dialog；`copyToSlicer` / `downloadFiles` 在桥里直接抛） |
| 契约 ↔ Rust | 一一对齐（未发现"契约有后端没有"或反之） |
| **注册面** | **两份清单彼此不一致**：客户端构建缺 7 条 `ipc::update::*`；工作台构建缺 3 条 mine 命令（X-01 / X-08） |
| 命令层测试 | 只有 `ipc/mod.rs`(2)、`ipc/presets.rs`(6)、`ipc/param_sync.rs`(15) 有 `#[test]`；`catalog.rs` / `mine.rs` / `import.rs` / `preset_params.rs` / `preset_baseline.rs` / `update.rs` **命令层无测试** |
| 内核测试 | 较足（`runtime/delivery.rs` 32 个、`runtime/mine.rs` 46 个等） |
| 前端测试 | **无 `*.test.ts*`**；替代物是 11 个 playwright 探针 + 4 个 check 脚本 |

## 1. 命令清单（65 条）

「注册」列：**俩** = 两份清单都有；**仅C** = 只在客户端那份；**仅W** = 只在工作台那份；**—** = 不是命令。

### 1.1 机型 / 文件 / 参数读（只读 catalog）

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 测试 | 产品路径 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-01 | `getMachines` | `get_machines` @ `ipc/presets.rs::301` | 俩 | `usePresetData::594`、`useParams::690`、`useCatalog::67` | 有 | 读 catalog | ✔ | 是 | 已证实 |
| C-02 | `getVersionFiles` | `get_version_files` @ `::435` | 俩 | `usePreset::100`、`usePresetData::609`、`useBundleFiles::85` | 有 | 读 catalog | ✔ | 是 | 已证实 |
| C-03 | `getPresetFiles` | `get_preset_files` @ `::524` | 俩 | `usePresetData::489/593` | 有 | 读 catalog | ✔ | 是 | 已证实 |
| C-04 | `getMenu` | `get_menu` @ `::564` | 俩 | `usePresetData::595` | 有 | 读 catalog | ✔ | 是 | 已证实 |
| C-05 | `getParamMeta` | `get_param_meta` @ `::645` | 俩 | `useParams::691` | 有 | 读 catalog.registry | ✔ | 是 | 已证实 |
| C-06 | `getMachineParams` | `get_machine_params` @ `::721` | 俩 | `usePreset::101`、`usePresetData::610` | 有 | 读 catalog（后端摊平） | ✔ | 是 | 已证实 |
| C-07 | `getLocalFiles` | `get_local_files` @ `::812` | 俩 | `usePresetData::596`、`presetTree::1236` | 有 | **桩：恒 `Ok(vec![])`** | ✔ | 是 | 已证实 |
| C-08 | `getSlicerCopied` | `get_slicer_copied` @ `::818` | 俩 | `usePresetData::598/793` | 有 | **桩：恒空** | ✔ | 是 | 已证实 |

### 1.2 用户线（`presets-mine`）

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 测试 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-09 | `getUserPresetFiles` | `get_user_preset_files` @ `ipc/mine.rs::92` | 俩 | `usePresetData::597`、`useParams::729` | 有 | 扫盘 | ✗ | 已证实 |
| C-10 | `readUserPresetText` | `read_user_preset_text` @ `::801` | 俩 | `PagePresets::669` | 条件抛 | 读文件 | ✗ | 已证实 |
| C-11 | `getPresetRemarks` | `get_preset_remarks` @ `::724` | **仅C** | `usePresetData::599` | 有 | 读备注账 | ✗ | 已发现冲突（X-08） |
| C-12 | `setPresetRemark` | `set_preset_remark` @ `::736` | **仅C** | `usePresetData::912` | 有 | 写备注账 | ✗ | 已发现冲突（X-08） |
| C-13 | `setUserPresetMachineVersion` | `set_user_preset_machine_version` @ `::755` | **仅C** | `usePresetData::923` | 有 | 改文件头两行 | ✗ | 已发现冲突（X-08） |
| C-14 | `beginPresetEdit` | `begin_preset_edit` @ `::247` | 俩 | `usePresetData::842` | 条件抛 | 读盘 + 写草稿 + 广播 | ✗ | 已证实 |
| C-15 | `putPresetDraft` | `put_preset_draft` @ `::351` | 俩 | `usePresetData::846` | 有 | 写草稿 + 广播 | ✗ | 已证实 |
| C-16 | `patchPresetDraft` | `patch_preset_draft` @ `::374` | 俩 | `useParams::665` | 有 | 结构保真改草稿 + 广播 | ✗ | 已证实 |
| C-17 | `discardPresetDraft` | `discard_preset_draft` @ `::402` | 俩 | `usePresetData::848` | 有 | 清草稿 + 广播 | ✗ | 已证实 |
| C-18 | `commitPresetDraft` | `commit_preset_draft` @ `::422` | 俩 | `usePresetData::851` | 条件抛 | 写用户根 + 清草稿 + 广播 | ✗ | 已证实 |
| C-19 | `renameUserPreset` | `rename_user_preset` @ `::487` | 俩 | `usePresetData::863` | 有 | `fs::rename` + 迁指针/草稿/出处 + 广播 | ✗ | 已证实 |
| C-20 | `copyUserPreset` | `copy_user_preset` @ `::532` | 俩 | `usePresetData::896` | 有 | 字节复制 + 出处账 | ✗ | 已证实 |
| C-21 | `getUserCopyFor` | `get_user_copy_for` @ `::576` | 俩 | `usePreset::103` | 有 | 读盘 + 归属匹配 | ✗ | 已证实 |
| C-22 | `savePresetCalibration` | `save_preset_calibration` @ `::630` | 俩 | `useCalibration::254` | 有 | 结构保真写用户文件 | ✗ | 已证实 |
| C-23 | `deleteUserPreset` | `delete_user_preset` @ `::662` | 俩 | `usePresetData::870` | 有 | 真删 + 撤指针/草稿 + 删备注 + 忘决策账（吞错） | ✗ | 已证实 |
| C-24 | `revealInFolder` | `reveal_in_folder` @ `::708` | 俩 | `usePresetData::903` | 有 | 开系统文件管理器 | ✗ | 已证实 |

### 1.3 对比台 / 基线 / 逐参数

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 测试 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-25 | `getOfficialVersions` | `get_official_versions` @ `ipc/preset_baseline.rs::55` | 俩 | `usePresetData::496` | 有 | 读目录 + 版本链 + baseline | ✗ | 已证实 |
| C-26 | `readPresetParams` | `read_preset_params` @ `ipc/preset_params.rs::57` | 俩 | `CompareModal::161/246`、`useParams::1523` | 有 | 读用户文件（**失败转 `problem` 不抛**） | ✗ | 已证实 |
| C-27 | `savePresetParams` | `save_preset_params` @ `::97` | 俩 | `CompareModal::237` | 有 | 结构保真写；**空改动/无变化直接 Ok 不写盘** | ✗ | 已证实（X-12） |
| C-28 | `getPresetParamSync` | `get_preset_param_sync` @ `ipc/param_sync.rs::104` | 俩 | `useParams::1551` | 有 | 读三方账；`fetchMissing=true` 时**发一次网络** | ✔ | 已证实 |
| C-29 | `applyPresetParamDecisions` | `apply_preset_param_decisions` @ `::127` | 俩 | `useParams::1578` | 有 | 写用户文件 + 决策账 + 广播 | ✔ | 已证实 |

### 1.4 导入

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 测试 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-30 | `stageImport` | `stage_import` @ `ipc/import.rs::49` | 俩 | `FileImport::130` | 有 | 只检查盘 | ✗ | 已证实 |
| C-31 | `commitImport` | `commit_import` @ `::76` | 俩 | `FileImport::136/177` | 有 | 字节复制 + 出处账（`let _ =` 吞错） | ✗ | 已证实 |
| C-32 | `pickImportFiles` | **非命令** @ `bridge.ts::173` | — | **无 UI 触发点** | 有 | plugin-dialog | ✗ | 已证实 |

### 1.5 catalog 与交付（`ipc/catalog.rs`）

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 网络 | 测试 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-33 | `getRuntimeCatalog` | `get_runtime_catalog` @ `::31` | 俩 | 6 处（预设/参数/对比台/BBS/首页） | 有 | 读 catalog | 否 | ✗ | 已证实 |
| C-34 | `getDownloadedFiles` | `get_downloaded_files` @ `::546` | 俩 | `usePresetData::490`、`PageHome::361`、`useBbsDelivered::62` | 有 | 读下载区 + 事件账 | 否 | ✗ | 已证实 |
| C-35 | `getStaleFiles` | `get_stale_files` @ `::565` | 俩 | `usePresetData::491`、`PageHome::362` | 有 | 读盘 + 目录 SHA 比 | 否 | ✗ | 已证实 |
| C-36 | `getDeliveryTrust` | `get_delivery_trust` @ `::603` | 俩 | `usePresetData::492` | 有 | 读盘 + 归档 / 旧目录链 | 否 | ✗ | 已证实 |
| C-37 | `getArchivedFiles` | `get_archived_files` @ `::662` | 俩 | `usePresetData::493` | 有 | 读归档区 | 否 | ✗ | 已证实 |
| C-38 | `readArchivedText` | `read_archived_text` @ `::715` | 俩 | `PagePresets::669` | 恒抛 | 读文件 | 否 | ✗ | 已证实 |
| C-39 | `readDownloadedText` | `read_downloaded_text` @ `::212` | 俩 | `useBbsDelivered::83` | 恒抛 | 读文件 | 否 | ✗ | 已证实 |
| C-40 | `downloadCatalogFile` | `download_runtime_file` @ `::246` | 俩 | `usePresetData::806` | 恒抛 | 下载 + 落盘 + 归档 + baseline + 事件 + Channel | **是** | ✗ | 已证实 |
| C-41 | `downloadCatalogFiles` | `download_runtime_files` @ `::323` | 俩 | `usePresetData::935`、`PageHome::434` | 有 | 同 C-40（逐份、Rust 侧并发） | **是** | ✗ | 已证实 |
| C-42 | `deleteDeliveryFile` | `delete_delivery_file` @ `::746` | 俩 | `usePresetData::881` | 恒抛 | 删字节 + 撤指针/草稿 + 删备注 | 否 | ✗ | 已证实 |
| C-43 | `deleteArchivedFile` | `delete_archived_file` @ `::784` | 俩 | `usePresetData::890` | 恒抛 | 真删归档字节 | 否 | ✗ | 已证实 |
| C-44 | **`fetchOfficialPreset`** | `fetch_official_preset` @ `::434` | 俩 | `usePresetData::826`、`PageHome::460` | 条件抛 | 取回官方 + 落工作副本（幂等，**不改当前使用**） | 条件 | ✗ | 已证实 |
| C-45 | `getActivePreset` | `get_active_preset` @ `::861` | 俩 | `appState.ts::42`（**全应用唯一入口**） | 有 | 读 `run/app-state.json` | 否 | ✗ | 已证实 |
| C-46 | `applyActivePreset` | `apply_active_preset` @ `::889` | 俩 | `usePresetData::780`、`PageHome::461` | 条件抛 | 校验两条线前置 + 写指针 + 广播 | 否 | ✗ | 已证实 |
| C-47 | `clearActivePreset` | `clear_active_preset` @ `::958` | 俩 | `usePresetData::787` | 有 | 清指针 + 广播 | 否 | ✗ | **无界面入口**（"撤销应用"已退役） |
| C-48 | `checkRemoteUpdate` | `check_remote_update` @ `::984` | 俩 | `usePresetData::408` | 恒抛 | 取远端 catalog + 比 revision（附 `readable`） | **是** | ✗ | 已证实 |
| C-49 | `applyRemoteUpdate` | `apply_remote_update` @ `::1012` | 俩 | `usePresetData::415` | 恒抛 | 归档旧目录 + 落新目录；读不懂 → `NOT_SUPPORTED` 且盘零改动 | **是** | ✗ | 已证实 |
| C-50 | `getAppVersion` | `get_app_version` @ `::1049` | 俩 | `PageSettings::93/101` | 有 | 常量 | 否 | ✗ | 已证实 |
| C-51 | `checkSoftwareUpdate` | `check_software_update` @ `::1061` | 俩 | `PageSettings::94` | 有 | 取 `release.json` | **是** | ✗ | 已证实 |
| C-52 | `getPresetSource` | `get_preset_source` @ `::117` | 俩 | `PageSettings::69`、`usePresetData::659/744` | 有 | 读设置 | 否 | ✗ | 已证实 |
| C-53 | `setPresetSource` | `set_preset_source` @ `::130` | 俩 | `PageSettings::145/199` | 有 | **先探一次网络再落盘** + 广播 | **是** | ✗ | 已证实 |
| C-54 | `clearPresetSource` | `clear_preset_source` @ `::173` | 俩 | **无调用者** | 有 | 删设置 + 广播 | 否 | ✗ | **疑似遗留**（X-11） |

### 1.6 校准模型

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 测试 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C-55 | `getCalibModels` | `get_calib_models` @ `ipc/mod.rs::122` | 俩 | **无调用者** | 有 | 内存静态表 | ✔ | **疑似遗留**（X-11） |
| C-56 | `openModel` | `open_model` @ `::128` | 俩 | `PageCalib::400` | 有（仅 `console.info`） | **仅打日志** | ✗ | 已证实（X-06） |

### 1.7 软件自更新（**只在工作台那份清单里**）

| id | 契约方法 | Rust 命令 @ 位置 | 注册 | 前端调用者 | mock | 副作用 | 等级 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C-57 | `updateInfo` | `update_info` @ `ipc/update.rs::70` | **仅W** | `UpdateIndicator::88` | 有 | 读 release.json + 上次结果 | 已发现冲突（X-01） |
| C-58 | `startUpdate` | `start_update` @ `::92` | **仅W** | `UpdateIndicator::206`、`PageSettings::124` | 有 | spawn 后**立即 Ok**；进度走事件 | 已发现冲突（X-01） |
| C-59 | `pauseUpdate` | `pause_update` @ `::117` | **仅W** | `UpdateIndicator::178` | 有 | 开关 | 已发现冲突 |
| C-60 | `resumeUpdate` | `resume_update` @ `::122` | **仅W** | `UpdateIndicator::188` | 有 | 开关 | 已发现冲突 |
| C-61 | `cancelUpdate` | `cancel_update` @ `::127` | **仅W** | `UpdateIndicator::181/191` | 有 | 开关 | 已发现冲突 |
| C-62 | `installUpdate` | `install_update` @ `::138` | **仅W** | `UpdateIndicator::114` | 有 | spawn 脚本 + 200ms + `app.exit(0)` | 已发现冲突 |
| C-63 | `openUrl` | `open_url` @ `::172` | **仅W** | `PageSettings::115` | 有 | 系统浏览器（只收 http(s)） | 已发现冲突 |

### 1.8 非命令

| id | 契约方法 | 实现 | 说明 | 等级 |
| --- | --- | --- | --- | --- |
| C-64 | `copyToSlicer` | `bridge.ts::262` `notWired` → 恒抛 | **mock 有实现**（只改内存）；真机未接线（X-02） | 已发现冲突 |
| C-65 | `downloadFiles` | `bridge.ts::263` `notWired` → 恒抛 | mock 也恒抛（X-09） | 已证实 |

## 2. DTO 字段字典（14 个结构 / 84 个字段）

> 这里给每个结构的**核心字段与语义**；逐字段的 `必填 / 默认 / 谁生成 / 何时变 / 谁读 / 是否持久化 / 是否派生 / 是否用户可见` 全表在 `data/contract.json` 的 `dtos`，HTML「契约」分节可展开逐字段查看。

### D-01 `UserPresetFile`（`mine.rs::37`，不持久化，盘本身是底账）
`path`（相对用户根，内部钥匙）· `fileName` / `size` / `modifiedUnix`（可见）· `kind`（`FileKind|null`：`.json` 分不出 bbs/orca ⇒ null）· `state`（`ok|unreadable|null`，**只判可读性，不判是否合法 Preset**）· `stateDetail` · `basedOn`（`current|outdated|unknown`）· `basedOnLabel` / `basedOnRelease`（血统原文，持久化在文件头）· `basedOnMachineId` / `basedOnVersionId`（来源那份现在对应哪台机哪版）· `machineId` / `versionId`（**这一份自己的归属**）· `copiedFrom` / `copiedFromName` / `provenance`（出处账）。

### D-02 `PresetFileInfo`（`presets.rs::457`，发布产物）
`id` / `fileName` / `path` / `kind` / `category` / `machineIds` / `nozzle?` / `layerHeight?` / `inBundles` / `usedByVersions` / `delivery`（`default|optional`，**由 bundles 与版本引用推出**）· `sizeText?` / `modifiedText?`（**已格式化串**）· `statFrom?`（`file|demo`，标明上两格是"真值"还是"演示推值"）。

### D-03 `RuntimeCatalog` / `RuntimeCatalogFile`（`runtime/catalog.rs::56`）
`catalogSchema` · `revision`（指纹；**不含** publishedAt）· `publishedAt?`（发布侧盖；随包目录刻意不带）· `brands?` / `machines` / `assets?` / `bundles` / `registry` / `plates` · `structureSignature` / `minClientVersion?`（能力判定）· `files[]`（`kind`/`fileName`/`path`/`machineId`/`versionId`/`sha256?`/`size?` —— 随包侧两个可空字段是**空的**）。

### D-04 `ActivePreset`（`ipc/catalog.rs::796`）
`origin`（`official|mine`）· `fileName`（官方线=目录键，用户线=显示名）· `path`（用户线相对用户根；官方线 null）· `sha256`（应用时刻指纹）· `machineId` / `versionId`（**认不出是空串**，不是 null）· `intact`（**派生**：盘上是否仍是应用时刻那份；用户线被再改 ⇒ false 是正常）。

### D-05 `FetchOfficialResult`（`catalog.rs::402`）
`fetched`（本次是否真取回）· `created`（本次是否新落一份工作副本）· `fileName` / `path`（我那一份的落点）。**不持久化**，只说明"这次做了什么"。

### D-06 `ArchivedFile`（`catalog.rs::634`）
`path` / `fileName` / `size` / `sha256`（对号键）· `publishedAt`（**这一版**云端发布时刻；链建立之前 null，绝不拿换下时刻顶）· `replacedUnix`（被换下时刻，事件账）· `machineId` / `versionId` / `kind`（认不出 null，不猜）。

### D-07 `OnDiskFile`（`catalog.rs::537`）
`fileName` · `downloadedUnix` / `replacedUnix`（**至多一个有值**）· `publishedAt`（这份字节属于哪一代目录、那一代发布时刻）。**时间全部来自事件，mtime 不上界面。**

### D-08 `DeliveryTrust`（`catalog.rs::594`）
`fileName` · `verdict`（`old`=认得出是官方某一版旧版 / `tampered`=查不出属于哪一版）· `archivedPath?`。**只列有事的**（没下载 / 与目录一致的不出现）。

### D-09 `OfficialVersion`（`preset_baseline.rs::37`）⚠️ 见 X-07
`fileName` / `sha256`（界面不显示）· `releaseTime?`（正文 `# release_time` 真值）· `publishedAt?`（目录代时间，兜底）· `downloaded`（**= baseline 里有没有这一版**，与磁盘口径冲突）· `current`（是不是当前目录登记的那一版）。

### D-10 `PresetParamSync` / `ParamSyncEntry`（`param_sync.rs::52`）
前者：`path` / `fileName` / `machineId?` / `versionId?` / `officialFileName?`（null ⇒ 比不出更新）· `basedOnReleaseTime?` / `currentReleaseTime?` · `officialReady`（官方当前版字节在本机可用）· `versionAdvanced` · `pendingCount`（**派生给前端，前端不重算**）· `entries`。
后者：`key` · `mine` / `baselineOld` / `officialNew`（**null = 文件里没有这一项，不是空串**）· `pending` · `decided`（`adopt|hold|null`）。
（Rust 侧另有 `currentSha256`，契约未声明、界面不显示。）

### D-11 `PresetDraft` / `CommittedDraft` / `UserFileIdentity`（`mine.rs::181/198/214`）
`PresetDraft`：`origin`（决定保存文案）· `sourceFileName` · `path?`（用户线落点）· `text` · `updatedUnix` · `reused`。**持久化在 `run/app-state.json#draft`**。
`CommittedDraft`：`path` / `fileName` / `size` / `replaced`（用户线恒 true）。
`UserFileIdentity`：`path` + `fileName`（改名 / 另存共用）。

### D-12 `PresetSource` / `BuiltinPresetSource`（`catalog.rs::55/43`）
`mode`（**id 是契约**）· `label` / `address`（空串 = 选了自定义还没填）· `fromUser` · `builtin[]` · `defaultMode` / `builtinDefault?`。

### D-13 `RemoteUpdateCheck` / `SoftwareUpdate` / `UpdateInfo`
`RemoteUpdateCheck`：`upToDate` · `localRevision` / `remoteRevision` · `readable`（**能力判定，与 upToDate 是两件事**）。
`SoftwareUpdate`：`hasUpdate` / `currentVersion`（构建期 `CARGO_PKG_VERSION`）/ `latestVersion` / `notes?` / `url?` / `asset?`（`asset` 缺省 ⇒ 退回"打开下载页"）。
`UpdateInfo`：`state`（tag 判别：idle/downloading/paused/ready/failed/cancelled）· `lastResult?`（**安装脚本写、下次启动读**）。

### D-14 `DownloadTick` / `DownloadOutcome`
`DownloadTick`：`stage`（**只有四档**：connecting / transferring / done / failed —— 没有"校验中/落盘中"，那两个报出来就是编的）· `fileName` · `received` · `total?`（null ⇒ 界面别说百分比）· `message?`（失败原因来自后端）。
`DownloadOutcome`：`fileName` / `ok` / `message`（**逐份结局，一份失败不拖累别人**）。

## 3. 前后端职责边界

### 3.1 规则住在哪一侧

| 规则 | 住在 | 证据 |
| --- | --- | --- |
| 一切文件系统访问、防穿越、原子写 | Rust（`fsx/*`，唯一写盘出口） | `src-tauri/src/fsx/atomic.rs::28` |
| TOML 结构保真写值（注释/键序/行尾不动） | Rust（`presetdata::patch` + `toml_edit`） | `src-tauri/src/presetdata/patch.rs` |
| 名字门槛（不许分隔符/空/换后缀/不覆盖） | Rust | `src-tauri/src/runtime/mine.rs::check_new_name` |
| 血统三行 / 归属归一化 / `basedOn` 判定 | Rust | `src-tauri/src/runtime/lineage.rs`、`mine.rs::179` |
| 只读区不可变 + 两条线前置校验 | Rust | `src-tauri/src/ipc/catalog.rs::889` |
| 并发（批量下载）与事件推送 | Rust | `src-tauri/src/ipc/catalog.rs::323` |
| 页面可见性 / 排序 / 空态文案 | 前端 | `src/app/presets/presetTree.ts` |
| 时间与大小的人话化 | **两边都有**：`modifiedUnix` 前端转；`sizeText` 与模型 `size` 后端给已格式化串 | `src/api/contract.ts::437` |
| 只读偏好（置顶/搜索词/抽屉宽/BBS 4 项） | 前端 localStorage（7 键，**不承载底账**） | `src/api/storageKeys.ts::16` |

**没有发现"同一条业务规则两端各写一份"**（曾经的重复已收口：`usePresetData.ts::45` 明确"已应用不再由前端推"；`PresetFileInfo.sizeText` 的存在理由就是"前端一行都不许自己编"）。

### 3.2 前端自行推断后端没返回的状态（这是最该改的一处）

| 推断 | 在哪 | 后端有对应字段吗 | 证据 | 等级 |
| --- | --- | --- | --- | --- |
| 行状态四档 / 云端三态 / "已应用·已下载" | `presetTree.ts::427`、`::1511`、`::939`、`::2268` | **没有** —— 前端从 5 个读组合 | `src/app/presets/usePresetData.ts::480` | 已证实 |
| 更新进度的"未知总量"暗示 | `UpdateIndicator.tsx::120` | 没有 | — | 已证实 |
| 旧后端缺字段时的 catalog 兜底 | `useParams.ts::678` | 没有 | — | 部分证实 |

### 3.3 命令"返回成功但持久化未完成 / 先返回后写"

| 情况 | 说明 | 证据 | 等级 |
| --- | --- | --- | --- |
| `startUpdate` | spawn 后**立即 Ok**，下载仍在跑（进度靠事件） | `ipc/update.rs::103` | 已证实（设计如此） |
| `installUpdate` | spawn 脚本 + 200ms + `app.exit(0)`；结果**下次启动**读 | `ipc/update.rs::152` | 已证实 |
| `savePresetParams` | `edits` 为空或算出来与原文一致 ⇒ **直接 Ok 不写盘**（连 mtime 不动），DTO **不带**"是否真落盘" | `ipc/preset_params.rs::102` | 已证实（X-12） |

### 3.4 吞错清单（"后端说没有"与"调用失败"在这些地方长得一样）

| 位置 | 吞了什么 | 后果 | 证据 |
| --- | --- | --- | --- |
| `commitImport` 的 `provenance::record` | `let _ =` | 文件落了、出处账可能没记 | `ipc/import.rs::94` |
| `deleteUserPreset` 的 `forget_param_decisions` | 吞掉 | 残留决策账 | `ipc/mine.rs::691` |
| 广播 `emit` | `let _ =` | 写成功后广播失败只是前端不自动刷新 | `ipc/mod.rs::51` |
| 前端 `catch` 共 31 处 | 正式路径关键几处：设置页版本失败当空串、首页下载区/过时列表失败当空数组、`usePresetData` 多处 | 失败被渲染成"没有" | `PageSettings.tsx::93`、`PageHome.tsx::361` |

### 3.5 错误模型

- **结构化为主**：`AppError{code,message,traceId,detail?}`，`ErrorCode` 10 档（含 `NOT_SUPPORTED`），与 `src-tauri/src/error.rs` 逐字段对齐；`traced()` 给每条命令一个 traceId 并记一次日志（`ipc/mod.rs::79`）。
- **兜底**：没过 IPC 的 reject（命令没注册 / 反序列化失败 / panic）由 `bridge.ts::43` 归一成 `INTERNAL` 或 `NOT_IMPLEMENTED`，`traceId: '-'`。**X-01 的 7 条命令就是落到这一支。**
- **有意不用错误的"软失败"**：`readPresetParams` 返回空表 + `problem`；`Import/Download` 逐份 `ok/message`。

### 3.6 重写时建议的边界（**建议，不是现状**）

1. **业务规则全部下沉到 Rust 单一领域层**：前端只做呈现与本地偏好（对应 `RD-06`）。
2. **后端为"用户会问的每一个状态"直接给单一字段**（`RD-02`）：本机有没有 / 官方有没有新版 / 有没有待处理参数 / 当前启用的是哪一份。
3. **命令清单由一处定义**，两个入口引用同一份，并加一条"前端会调的命令都注册了"的自动判据（`RD-01`）。
4. **不做空实现**：命令层宁可少注册，也不留只打日志的实现（`RD-17`）。
