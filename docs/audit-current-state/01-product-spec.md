# 01 · 产品规格层：用户实际在使用什么

> 逐页逐动作。字段含义：**入口 / 目的 / 输入**＝用户从哪找到它、想完成什么、给了什么；
> **执行流程**＝真实调用链（前端数据层 → 最终 api）；**成功 / 失败**＝界面与磁盘的两种结果；
> **持久化**＝是否读写磁盘或网络；**可用**＝当前 HEAD 下是否真能用；**证据**＝`路径::符号::行号`。
> 「重写建议」写成编号引用，正文在 `rewrite-decisions.md`（RD-nn）。

## 0. 页面清单

| id | 页签 | 页面 | 组件 | 它解决什么问题 | 展示的主要数据来源 |
| --- | --- | --- | --- | --- | --- |
| P-shell | （外壳） | 外壳 / 标题栏 / 通用导入 | `src/app/App.tsx::App::62` | 切页、窗口按钮、软件更新环、把文件拖进窗口导入 | `appState`（底账订阅）、`deliveryState`（代次）、`api.updateInfo` |
| P-home | `machine` | 首页 | `src/app/home/PageHome.tsx::PageHome::109` | 五步向导：选机型/版本 → 选预设 → Z → XY → 测试模型 | `getMachines` / `getRuntimeCatalog` / `getVersionFiles` / `getMachineParams` / `getUserCopyFor` / `getDownloadedFiles` / `getStaleFiles` / `getActivePreset` |
| P-presets | `preset` | 预设 | `src/app/presets/PagePresets.tsx::PagePresets::250` | MKP/切片器 × 本地/云端 两表：取回官方、启用、编辑、另存、改名、删除、参数入口 | 14 个读（见 §2） |
| P-params | （无页签，右键「打开参数」） | 参数（模态框） | `src/app/params/PresetParamsModal.tsx::39` | 改某一份预设的参数：分类/搜索/卡片/G-code/擦料塔/历史/撤销 + 逐参数官方更新 | `getParamMeta` / `getMachineParams` / `readPresetParams` / `savePresetParams` / `getPresetParamSync` / `applyPresetParamDecisions` / `patchPresetDraft` / `commitPresetDraft` |
| P-calib | `calib` | 校准 | `src/app/calib/PageCalib.tsx::PageCalib::76` | Z / XY / 测试模型 三步校准，保存写进「我的预设」 | `getVersionFiles` / `getMachineParams` / `getUserCopyFor` / `savePresetCalibration` |
| P-bbs | `bbs` | BBS 预设 | `src/app/bbs/PageBbs.tsx::PageBbs::58` | 只读查看 Bambu Studio 工艺预设，可与产品已下载的配置对照 | `fetch /bbs/*.json`（public 与 dev 端点）+ `getRuntimeCatalog` / `getDownloadedFiles` / `readDownloadedText` |
| P-report | `report` | 报告 | `src/app/components/PagePlaceholder.tsx`（`App.tsx::126` 挂载） | 占位：没有数据、没有动作 | — |
| P-settings | `settings` | 设置 | `src/app/settings/PageSettings.tsx::PageSettings::48` | 软件更新 + 预设数据源（开发后门） | `getPresetSource` / `setPresetSource` / `getAppVersion` / `checkSoftwareUpdate` / `openUrl` / `startUpdate` |

> 顶栏页签 6 条：首页 / 预设 / 校准 / BBS 预设 / 报告 / 设置（`src/app/constants/tabs.ts`）。**「参数」页签已于 2026-10-09 退场**，参数改由预设行右键进入，组件仍是同一个 `PageParams`。

## 1. 外壳 / 导入 / 更新环（P-shell）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 顶栏页签 | 换一页 | 点击 | `TopTabs.tsx::108` → `App.tsx::102 setTab` | 该页显示（常驻挂载，状态保留） | — | 无 | 是 | — | `src/app/App.tsx::102` |
| 拖文件进窗口 | 导入我的一份预设 | 拖拽 `.toml` | `FileImport.tsx::269 onDrop` → `importPaths::124` → `api.stageImport` → `api.commitImport` | 落进 `presets-mine/`，列表出现新行 | 非 toml / 重名 → 逐份原因 | 写 `user/presets-mine/<name>` | 是 | — | `src/app/import/FileImport.tsx::130` |
| 重名那一格 | 起个新名字再导入 | 输入名字 | `FileImport.tsx::165 submitModal` → `api.commitImport` | 改名后落盘 | 空名 / 仍重名 → 后端起名门槛拒绝 | 同上 | 是 | — | `src/app/import/FileImport.tsx::165` |
| 更新环：暂停/继续/取消/开始/安装 | 更新软件本体 | 点环 | `UpdateIndicator.tsx::178/188/206/219` → `api.pauseUpdate` 等 | 下载进度 → 安装并重启 | **落到桥兜底 `NOT_IMPLEMENTED`**（客户端构建没注册这 7 条） | `update/…` + `run/update-result.json` | **否** | X-01 | `src/app/components/UpdateIndicator.tsx::206`、`src-tauri/src/lib.rs::396` |
| 提示条上的 traceId | 复制错误追踪号 | 点击 | `TraceTag.tsx::33` → 剪贴板 | 复制成功 | 剪贴板被拒则静默 | 无 | 是 | — | `src/app/components/TraceTag.tsx::33` |

导入的「文件选择器」那一半（`api.pickImportFiles`）**能力在、界面没有入口**（按钮已退役）；拖拽可用。

## 2. 首页（P-home）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「现在开始」/ 滑卡 / 圆点 / 键盘 ←→1-9 | 五步之间翻 | 点击或键盘 | `SlideDeck.tsx::522/534/560/317` → `jumpTo` | 切到目标步 | 当前步脏 → 弹确认挡一下 | 无 | 脏且未确认 | 是 | — | `src/app/home/SlideDeck.tsx::522` |
| 品牌 / 机型 / 版本卡片 | 选机器与版本 | 点击 | `MachinePicker.tsx::95` → `PageHome::209 pick` | 选中态变化 | — | 无（不进底账） | — | 是 | — | `src/app/home/MachinePicker.tsx::95` |
| 「复制后处理脚本」 | 复制脚本路径 | 点击 | `home/CopyAction.tsx` + `displayPath::490` → 剪贴板 | 路径进剪贴板 | 无 presetInfo/displayPath 时按钮不出现 | 无 | 无 showPath | 是 | 复制的是一串路径文本，不是可执行动作 | `src/app/home/PageHome.tsx::553` |
| 第 2 步主按钮（应用 / 下载并应用 / 更新并应用 / 已应用） | 装套餐并启用 | 点击 | `applyCurrent::425` → `api.downloadCatalogFiles` → `api.fetchOfficialPreset` → `api.applyActivePreset` → `appStateMutated()` | 套餐落下载区 + 工作副本落地 + 指针改写 | 逐份结局；浏览器 mock 必失败 | 下载区 + `presets-mine` + `run/app-state.json` | 已应用 ∥ applying ∥ 套餐未知 | 是 | **一次动作串三条命令，中间失败无回滚**；与预设页「使用」重复 | `src/app/home/PageHome.tsx::425` |
| 「打开模型」 | 下载/打开校准板模型 | 点击 | `PageHome::988 setOpenModel`；「从云端获取」只 `setOpenModel(null)` | 弹窗关掉（**什么也没发生**） | 无提示 | 无 | 「从本地缓存打开」写死 disabled | **否** | X-04 | `src/app/home/PageHome.tsx::988` |
| 预设 pill → 抽屉选一份 | 换这一版的预设 | 选一份 | `PageHome::981` → `pickPreset::289` → `applyPreset::274` → `appState.ts::activateCombo::116` → `api.applyActivePreset` | 指针改写，横幅同步 | 浏览器直接 return 不写底账 | `run/app-state.json` | 组合相同 / 取不到文件名 | 是 | **浏览即切换**：看一眼也会改底账（RD-07） | `src/app/home/PageHome.tsx::274` |
| Z/XY「保存」/「放弃改动」 | 把读数写进我那份 | 板上点格 / 手输 | `useCalibration.ts::245 commitAll` → `api.savePresetCalibration` | `presets-mine` 那份被改写 | 没有「我的一份」→ 保存被拦 | 写 `user/presets-mine/<rel>` | `!canSave` | 是 | 必须先有一份自己的预设才能校准 | `src/app/calib/useCalibration.ts::245` |
| 切页 / 换预设时的弹窗 | 处理未保存改动 | 点离开 | `PageHome::1008`、`1052` → `clearAll` / `commitAll` → `leaveTo` | 保存则写回；放弃则丢草稿 | — | 同上 | 保存项仅 `canSave` | 是 | — | `src/app/home/PageHome.tsx::1008` |

## 3. 预设页（P-presets）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 类型分段 / 位置分段 | 切到哪一类、哪一侧 | 点击 | `PresetScopeBar.tsx::74/102` → `setKind::471` / `setScope::476` | 换表，会话态保留 | — | 会话内存（刷新即丢） | — | 是 | 同一对词在 MKP 与切片器含义不同 | `src/app/presets/PresetScopeBar.tsx::74` |
| 搜索框 / × | 按名字/路径找 | 输入 | `page.setQuery` → `fileMatchesQuery::1258` | 表内过滤；空态区分"被筛掉/本来没有" | — | 会话内存 | — | 是 | — | `src/app/presets/presetTree.ts::1258` |
| 已应用 pill 左拍 | 找回"现在用的是哪份" | 点击 | `locateApplied::426` → 清筛选 + 滚动闪行 | 定位并高亮 | — | 无 | — | 是 | — | `src/app/presets/PagePresets.tsx::426` |
| 机型漏斗 / 喷嘴·层高 chips | 按机型与切片器属性筛 | 点选 | `pickMachine::465`、`slicerFilters.set*`、`applySlicerFilters::2448` | 表内过滤 | — | 会话内存 | — | 是 | — | `src/app/presets/presetTree.ts::2448` |
| 云端表批量条（下载/更新 N 份） | 一次补齐一批 | 点击 | `runBatch::601`：MKP 逐份 `fetchOfficial`；切片器 `downloadReleaseBatch` | 逐份结局（全成 / 列出没成的） | 命令级失败单独一句 | 下载区 + `presets-mine` | `batchBusy`；仅云端且 `pending>0` | 是 | MKP 批量没有过程水位 | `src/app/presets/PagePresets.tsx::601` |
| 「去更新」（需新客户端） | 客户端读不懂远端时去升级 | 点击 | `onOpenSettings`（`App.tsx::91`） | 跳设置页 | — | 无 | 仅 `needsNewerClient` | 是 | — | `src/app/presets/PagePresets.tsx::1743` |
| 点表行 | 展开详情 | 点击（非按钮） | `PresetTable.tsx::474` → `setExpandedKey::1789` | 展开事实表 | — | 无 | — | 是 | 详情塞了大量内部概念（RD-05） | `src/app/presets/PresetTable.tsx::474` |
| 云端行「下载」/「更新」 | 取回官方 + 落一份我的 | 点击 | `CloudLocalAction::180` → `download::510` → `data.fetchOfficial` → `api.fetchOfficialPreset` | 官方字节落地（旧份归档）+ `presets-mine` 落一份；**不改当前使用** | 断网 → 提示条原话 | 下载区 + baseline + 事件账 + `presets-mine` | `busy`；「本地已有」不给按钮 | 是 | X-10；参数差异另在「打开参数」 | `src/app/presets/PagePresets.tsx::510` |
| MKP 本地行「使用」 | 设为当前生效 | 点击 | `PresetTable.tsx::638` → `runUse::1046` → `data.apply(fileName,'mine',path)` → `api.applyActivePreset` | 指针改写（**不复制文件**） | 不可读 / 非 TOML → 后端拒 | `run/app-state.json` | 已在用 / 不可读 / 非 toml | 是 | 与首页「应用」重复（RD-04） | `src/app/presets/PagePresets.tsx::1046` |
| 切片器本地行「复制」 | 送进切片器目录才生效 | 点击 | `runLive::974` → `data.copy` → `api.copyToSlicer` | 假后端只改内存（提示条自己写"刷新还原"） | **真机恒抛 NotImplemented** | 真机无 | `busy` | **否** | X-02 | `src/api/bridge.ts::262` |
| 来源格「复制自 X」 | 跳回来源那一行 | 点链接 | `PagePresets::locateRow::446` | 切轴 + 清筛选 + 展开 + 闪一下 | 目标被筛掉时定位不到 | 无 | — | 是 | 「来源」是血统/出处账/路径三套因果拼的 | `src/app/presets/PagePresets.tsx::446` |
| 详情「旧版本 N 份」→ 抽屉 | 看官方换版留下的旧字节 | 点击 | `openArchive::688` → `data.archived` → `api.getArchivedFiles` | 抽屉列出旧版本（可看正文、可两段式删） | — | 读盘；删除写盘 | `archiveCount=0` 不出现 | 是 | 归档是官方生命周期的一部分却给用户管（RD-05/RD-10） | `src/app/presets/PagePresets.tsx::688` |
| 归档抽屉「删除」 | 删一份旧版本 | 两段式点击 | `runRemoveArchived::886` → `api.deleteArchivedFile` | 字节真删；版本链与事件账不动 | 前缀不符 / 不存在 | 删 `archive/<path>` | 两段式 | 是 | 删了就找不回 | `src-tauri/src/ipc/catalog.rs::784` |
| 详情「看正文」 | 只读看一眼 TOML | 点击 | `openMine::705` → `readBody::667` → `api.readUserPresetText` | 抽屉显示正文 | 读不出来照实说 | 只读 | — | 是 | — | `src/app/presets/PagePresets.tsx::667` |
| 详情「改这份」→ 编辑器 | 直接改 TOML 正文 | 编辑文本 | `openEdit::720` → `data.beginEdit`；边改边存 `putDraft`（700ms）；`commitEdit::781` | 改的是临时草稿；保存写回同一个文件 | 草稿落盘失败安静提示；保存失败原话 | `run/app-state.json#draft` → 保存写 `presets-mine` | 仅 mine + mkp + 可读 | 是 | 与「参数编辑」两条草稿链（RD-08） | `src/app/presets/PagePresets.tsx::720` |
| 起名字抽屉（改名 / 另存为一份新的） | 改名或另存 | 输入新名字 | `submitNaming::823` → `api.renameUserPreset` / `api.copyUserPreset` | 改名只动名字（指针与草稿跟着走）；另存新的一份独立存在 | 名字门槛由后端拒，原话留在抽屉 | `fs::rename` / 写新文件 + 出处账 | `busy` / 空名 | 是 | 改名可能"文件已改名、状态账没跟上" | `src/app/presets/PagePresets.tsx::823` |
| 详情「删除」（二次确认） | 删我那份 / 删本机官方件 | 点击 + 确认 | `runRemove::862` → `api.deleteUserPreset` / `api.deleteDeliveryFile` | 我的文件真删（无垃圾桶）；官方件回「未下载」 | 不存在 → NOT_FOUND | 删文件 + 清指针/草稿/备注(+决策账) | 官方仓库行不给删 | 是 | 删除会连带撤「正在使用」并丢草稿（RD-14） | `src/app/presets/PagePresets.tsx::862` |
| 详情「备注」保存 / 恢复默认 | 改列表副标题 | 输入文本 | `RemarkField → runSetRemark::930` → `api.setPresetRemark` | 覆盖账更新（空串也是覆盖） | 写账失败原话 | `user/preset-remarks.json` | `saving/busy/无改动` | 是 | 概念只服务副标题（RD-18） | `src-tauri/src/runtime/remarks.rs::56` |
| 详情「归属」保存 | 标机型 / 版本 | 两个下拉 | `AttributionField → runSetAttribution::949` → `api.setUserPresetMachineVersion` | 文件头两行被改写，列表两列以文件为准 | 机型不在目录 / 文件不在 | 改 `presets-mine` 文件头 | 两级未填满 / 无改动 | 是 | 是"两份清单一字不差"的实际违例（X-08） | `src/app/presets/PagePresets.tsx::949` |
| 工具栏「参数对比」 | 2~3 份我自己的预设并排比 | 选列 + 改值 | `CompareModal::194/209/225` → `api.readPresetParams` / `api.savePresetParams` | 可改可保存（逐字段结构保真写回） | 读不出来 → 空表 + `problem` | 写 `presets-mine` 各份自身 | `busy` / 无改动 / 最多 3 列 | 是 | 官方那份不进这张台子，用户不易理解 | `src/app/presets/CompareModal.tsx::194` |
| 右键菜单（行通用手势，含 Shift+F10） | 其余动作集中处 | 右键 | `entriesOf::1192`（按「哪张表 + 哪种来源」分支） | 按来源给出不同项 | — | 无 | 各项自带 `disabled` 与原因 | 是 | 5 种来源 × 2 张表，项多且语义重叠（CX-06） | `src/app/presets/PagePresets.tsx::1192` |
| ↳ 右键「复制链接」 | 复制下载链接 | 点击 | `sayNoContract::494` | — | 恒弹「这个动作还没有对应的实现」 | 无 | — | **否** | X-03 | `src/app/presets/PagePresets.tsx::494` |
| ↳ 右键「打开参数」/「选择要跟随的官方更新」 | 改参数 / 直进官方更新屏 | 点击 | `openParams::1175(syncFirst)` → `PresetParamsModal` | 参数模态框铺满内容区 | 前置不满足时灰掉并给原因 | 无 | 非 mine / 非 toml / 不可读 / 未标机型版本 | 是 | 参数编辑只能从行右键进 | `src/app/presets/PagePresets.tsx::1175` |
| ↳ 右键「置顶 / 取消置顶」 | 常用的排最前 | 点击 | `togglePin::915` → localStorage | 该表顺序变化，刷新保留 | localStorage 写不进 → 仅本次会话 | `mkp.A40.presets.pinned` | — | 是 | — | `src/app/presets/PagePresets.tsx::915` |
| ↳ 右键「在文件资源管理器中显示」 | 交给系统文件管理器 | 点击 | `runReveal::906` → `api.revealInFolder` | 系统窗口打开并选中该文件 | 文件被外面删了 → 找不到 | 无 | 仅 mine | 是 | — | `src-tauri/src/ipc/mine.rs::708` |
| ↳ 右键「在 BBS 预设查看器中打开」 | 跳到 BBS 页看这份 profile | 点击 | `bbsEntry::1080` → `App.tsx::80` setPendingBbs + setTab | 切到 BBS 页并按文件名选中 | 不是 bbs_profile/.json 时灰掉并说明 | 无 | kind/fileName/外壳条件 | 是 | 两页数据源不同，跳过去可能找不到 | `src/app/presets/PagePresets.tsx::1080` |
| ↳ 官方仓库行的「下载」（残支） | 下载官方仓库文件 | 点击 | `download::557` 末支 → `api.downloadFiles` | — | **两端都抛 NotImplemented** | 无 | — | **否** | X-09 | `src/api/bridge.ts::263` |

## 4. 参数编辑器（P-params，模态框内）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 分类胶囊 / 搜索 / 最近搜索 | 找到某一项 | 点击 / 输入 | `CategoryPills`、`SearchField::792`、`rememberSearch::378` | 过滤参数卡片 | — | 最近搜索写 localStorage | — | 是 | — | `src/app/params/PageParams.tsx::378` |
| 参数值编辑 / 单值还原 / 卡片展开 / 撤销重做 / Ctrl+S | 改值并随时回退 | 控件 / 键盘 | `u.edit → api.patchPresetDraft`；`u.revertToSaved`；`undo/redo`；`confirmSave::534` | 草稿即时改（结构保真：注释/键序/行尾不动） | 被 `showWhen` 挡住的项不可改（灰掉并说明） | `run/app-state.json#draft`；保存写回文件 | `blockedByOf` 非空 / 无改动 | 是 | 「保存修改」走哪条命令对用户不可见（RD-08） | `src/app/params/PageParams.tsx::621` |
| 参数行「用新值 / 保持我的」 | 逐项处理官方改动 | 点击 | `u.decideSync([key],'adopt'│'hold')` → `api.applyPresetParamDecisions` | adopt 改写对应项；hold 一字节不动；两者都推水位 | 官方正文不在本机时 adopt 如实报错 | `presets-mine` + `run/app-state.json#paramDecisions` | `syncBusy` | 是 | 「保持我的」会永久压掉这一版改动（RD-12） | `src-tauri/src/ipc/param_sync.rs::127` |
| 「选择要跟随的官方更新」弹窗 | 一屏勾选要采用的项 | 勾选 | `ParamSyncModal::88/95` → `onDecide` → `api.applyPresetParamDecisions` | 批量落决定，刷新待处理条数 | 应用失败原话 | 同上 | `busy` / 一项都没勾 | 是 | — | `src/app/params/ParamSyncModal.tsx::88` |
| 「修改历史」抽屉 | 看/回退本次编辑历史 | 点击 | `HistoryDrawer.tsx::138/150/116` → undo 栈 | 本会话内可还原 | 文件历史要接后端才有（自述"目前没有"） | 无（会话内） | `state==='undone'` | 部分 | 名字暗示文件历史，实为 undo 栈（X-15） | `src/app/params/HistoryDrawer.tsx::116` |

## 5. 校准页（P-calib）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 步骤条 tab | 换一步 | 点击 | `PageCalib::go::186`（脏则 `pending::171`） | 换视图；未保存先问一句 | — | 无 | — | 是 | — | `src/app/calib/PageCalib.tsx::186` |
| 预设 pill / 抽屉 | 换这一版的预设 | 选一份 | `pickPreset::215` → `activateCombo` → `api.applyActivePreset` | 指针改写 | — | `run/app-state.json` | — | 是 | 同首页（RD-07） | `src/app/calib/PageCalib.tsx::215` |
| 「打开模型」弹窗 | 打开校准板模型 | 点击 | 弹窗「从云端获取」→ `PageCalib::400` → `api.openModel` | 后端 `open_model` **只打日志** | 无提示 | 无 | 「从本地缓存打开」恒灰 | 部分 | X-04 / X-06 | `src-tauri/src/ipc/mod.rs::128` |
| 板上点格 / 读数输入 / 还原 / 清空 | 取 Z / XY 读数 | 点格 / 手输 | `CalibPlate onPick` → `pickZ/pickXY`；`CalibReadings::132/111` | 草稿更新（未落盘） | 没有可对照的基准时不可点 | 无（草稿在内存） | `!canPick` | 是 | 「暂时不可点」的条件不直观 | `src/app/calib/useCalibration.ts::181` |
| 「保存」 | 写进我那份预设 | 点击 | `commitAll::245` → `api.savePresetCalibration` | `presets-mine` 那份被结构保真改写 | 没有「我的一份」→ 拦下并提示先另存 | 写 `user/presets-mine/<rel>` | `!dirty ∥ !canSave` | 是 | 依赖链对用户不直观（RD-12 相关） | `src/app/calib/useCalibration.ts::245` |

## 6. BBS 预设页（P-bbs）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 预设列表抽屉 / 浮层↔并排 / 看全部↔跟 BBS 一样 / 深浅皮肤 | 换这一页的看与选 | 点击 | `PageBbs::227/253/302/311/378` | 形式与皮肤变化，刷新保留 | — | localStorage（4 键） | — | 是 | — | `src/app/bbs/PageBbs.tsx::227` |
| 「导入 JSON」/ 拖 .json 到页面 | 拿外来 profile 对照 | 选文件 / 拖拽 | `readFile::185` → `preset.importDoc` | 页面内读入并显示参数 | 非 json / 解析失败 → 整份拒 | 无（仅本页内存） | 仅 `.json` | 是 | 与预设页的「导入」完全两回事（CX-17） | `src/app/bbs/PageBbs.tsx::185` |
| 「重扫本机」/「载入产品配置…」 | 读本机 BBS 目录 / 读产品已下载配置 | 点击 / 下拉 | `data.rescan`（dev 端点）；`delivered.read::83` → `api.getRuntimeCatalog` / `getDownloadedFiles` / `readDownloadedText` | 有字节则读入对照 | 还没下载 → 显示「还没下载」 | 只读 | live 模式才有「重扫本机」 | 是 | 「重扫本机」走开发服务器端点，不是正式契约 | `src/app/bbs/useBbsDelivered.ts::83` |
| 行内控件改值 / 行内「重置」/ 顶部「全部还原」 | 在"可改动"模式下试改 | 控件 / 点击 | `BbsControl.tsx::89/121/139`、`BbsRow.tsx::89` → `preset.setValue/resetKey/revertAll` | 本页数值变化 | 展示模式 readOnly | **无 api —— 是否落盘未证实** | `readOnly` / `greyed` | 部分 | X-14（"可改动"但看不到落点） | `src/app/bbs/BbsControl.tsx::89` |

## 7. 设置页（P-settings）

| 入口 | 目的 | 输入 | 执行流程 | 成功 | 失败 | 持久化 | 禁用 | 可用 | 问题 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 打开页面（读数据源 + 查软件更新） | 显示当前源与有没有新版 | 进页面 | `read::66`、`checkUpdate::89` → `api.getPresetSource` / `getAppVersion` / `checkSoftwareUpdate` | 显示源与版本 | **catch 吞错 → 当「没查到」** | 只读 | — | 部分 | X-13 | `src/app/settings/PageSettings.tsx::66` |
| 「查看更新」/「在应用内下载」/「重新检查」 | 看说明 / 下载安装包 / 重查 | 点击 | `openReleasePage::113` → `api.openUrl`；`startDownload::122` → `api.startUpdate` | 浏览器打开下载页 / 开始下载 | **落到兜底 `NOT_IMPLEMENTED`** | `update/…` | `updateBusy`；按钮出现看 `hasUpdate/asset` | **否** | X-01 | `src-tauri/src/lib.rs::396` |
| 预设数据源：内置源单选 / 自定义地址「应用」 | 换预设数据来源 | 点选 / 输地址 | `pick::135` → `api.setPresetSource(next)` → `sourceMutated()`/`deliveryMutated()`；`apply::194` | 先探一次再落盘；换源后预设页立即按新源重读 | 地址不通 → 整次拒绝 | `run/app-state.json#presetSource` | `busy` / 地址为空 | 是 | 发布方的事做成用户设置（RD-09） | `src/app/settings/PageSettings.tsx::135` |

## 8. 这一层的结论

1. **不可用但可见的动作共 6 个**：A-04（更新环）、A-10（首页打开模型）、A-23（切片器复制）、A-35（复制链接）、A-40（官方仓库下载）、A-54（设置页软件更新链）。它们都能追到"命令没注册 / 没接线 / 只打日志 / 契约里没有"。
2. **部分可用或落点不明 6 个**：A-08（复制脚本路径）、A-11（浏览即启用）、A-45（历史抽屉）、A-51（BBS 重扫走 dev 端点）、A-52（BBS 可改动是否落盘）、A-53（设置页吞错）。
3. **重复入口**：首页「应用/下载并应用」与预设页「使用」是同一个指针写入的两个入口；参数页/首页/校准页的预设抽屉是同一件事的三个入口。
4. **必须理解内部实现才能用的操作**：看详情（血统/目录指纹/事件时间）、归档抽屉、"有新版"与"打开参数里的 N 项"之间的关系。
5. **主要任务要在多页之间来回**：拿一份预设（预设页）→ 启用（首页或预设页）→ 改参数（预设页右键）→ 校准（校准页）→ 跟官方更新（预设页 → 参数模态框）。
