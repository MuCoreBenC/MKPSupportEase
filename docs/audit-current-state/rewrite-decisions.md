# 交付物 D · 重写决策清单

> 每一项都是**可执行的重写需求输入**。裁断词表：**保留 / 合并 / 砍掉 / 降级为内部 / 重做 / 待验证**。
> 「受影响的 X」列的是本审计里的编号（`data/*.json` 可查）；证据一律 `路径::符号::行号`。
> **本文件里的"建议的新规则"都是建议，不是现状。**

目录：RD-01 命令注册面 · RD-02 本机状态唯一权威 · RD-03 状态词合并 · RD-04 动作词合并 · RD-05 内部概念内部化 · RD-06 状态改由后端给 · RD-07 查看与启用分离 · RD-08 单草稿链 · RD-09 数据源内部化 · RD-10 历史链合并 · RD-11 血统与归属瘦身 · RD-12 官方更新默认路径 · RD-13 页签只留有事可做 · RD-14 删除与指针解耦 · RD-15 错误不吞 · RD-16 写盘判据覆盖 src-tauri · RD-17 不做空实现 · RD-18 备注并入我的记录 · RD-19 数据层收敛 · RD-20 BBS 降为只读参考

---

## RD-01 命令注册面与构建面漂移

| 项 | 内容 |
| --- | --- |
| 当前问题 | 客户端构建缺 7 条 `ipc::update::*`；工作台构建缺 3 条 mine 命令。两份 `generate_handler!` 手写、按 feature 分叉，已经漂移（X-01 / X-08 / CX-10） |
| 当前行为 | `ipc::update::*` 只在 `#[cfg(feature="workbench")]` 那份清单里，而 `Cargo.toml default=[]`；标题栏 `UpdateIndicator` 与设置页在正式路径调用它们 → 落到桥兜底 `NOT_IMPLEMENTED` |
| 证据 | `src-tauri/src/lib.rs::139`、`src-tauri/src/lib.rs::396`、`src-tauri/Cargo.toml::45`、`src/app/components/UpdateIndicator.tsx::206` |
| 业务价值 | 软件自更新与"备注/归属"是真实功能，不能一半构建没有；"两份清单"这种手写重复本身就是缺陷源 |
| 裁断 | **重做** |
| 建议的新规则 | ①命令清单**由一处定义**（宏或数组），两个入口引用同一份；②`feature` 只决定"编译进不进"，不决定"注册到哪"；③加一条自动判据：**前端会调的命令必须都注册**，缺一条就失败（而不是落到兜底文案）；④启动时做一次自检 |
| 受影响 | 页 P-shell / P-settings；命令 `updateInfo` `startUpdate` `pauseUpdate` `resumeUpdate` `cancelUpdate` `installUpdate` `openUrl` `getPresetRemarks` `setPresetRemark` `setUserPresetMachineVersion`；文件 `src-tauri/src/lib.rs` |
| 验收标准 | 客户端构建下 7 条更新命令可调用（或界面不摆入口）；有一条判据同时检查两份清单一致 |
| 尚未确定 | 软件自更新是否进客户端重写范围（当前只发 Gitee、只支持 macOS `.app.zip`） |

## RD-02 「本机状态」只能有一个权威

| 项 | 内容 |
| --- | --- |
| 当前问题 | 同一事实有真值与桩两套：`get_local_files` 恒空、`get_slicer_copied` 恒空；官方件的真值在下载区；baseline 又算一份 `downloaded`（X-05 / X-07 / CF-01 / CF-02 / CF-08） |
| 当前行为 | 预设页「本机」计数与 `statusOf` 的"本地有"读恒空桩；`OfficialVersion.downloaded` 读 baseline；release 行读下载区三态 → 同一页两种事实 |
| 证据 | `src-tauri/src/ipc/presets.rs::812`、`::818`、`src-tauri/src/ipc/preset_baseline.rs::67`、`src/app/presets/presetTree.ts::448` |
| 业务价值 | 用户必须能相信"本机有没有这一份"——这是这个软件最基本的承诺 |
| 裁断 | **重做** |
| 建议的新规则 | ①本机状态**只有一个权威：磁盘**；②后端为每个文件直接给一个状态字段（缺 / 有且与官方一致 / 有但不一致），前端不再从多个读拼、也不再读桩；③baseline 只作为"官方旧值正文"的内部存储，**不参与"有没有"的判定** |
| 受影响 | 页 P-presets / P-home；实体 E-03 / E-09 / E-14；命令 `getLocalFiles` `getSlicerCopied` `getDownloadedFiles` `getStaleFiles` `getDeliveryTrust` `getOfficialVersions` |
| 验收标准 | 删掉本机文件后，所有显示"已下载/本机有"的地方在同一帧变"没有"；不存在第二个回答"有没有"的读 |
| 尚未确定 | 切片器那一档是否还在重写范围内（若在，它的"生效=复制进切片器目录"要跟着重新定义） |

## RD-03 状态词从五套收成两轴

| 项 | 内容 |
| --- | --- |
| 当前问题 | 五套状态词并存，且都用内部概念命名（旧版本 / 内容异常 / 基于旧版 / 有新版）（CX-01 / CX-03） |
| 当前行为 | 官方件四态（`ReleaseFileState`）、更新三态（`ReleaseUpdateState`）、MKP 云端三态（`CloudLocalState`）、用户件三档（`BasedOn`）、可读性两档（`MineState`） |
| 证据 | `src/app/presets/presetTree.ts::467`、`::515`、`::568` |
| 业务价值 | 状态必须让用户一眼知道"我能不能用、要不要动手" |
| 裁断 | **合并** |
| 建议的新规则 | 只保留**两轴两个词**：①我这边有没有（没有 / 有 / 有但落后）②官方那边有没有新的（没有 / 有）。内部概念（baseline / 血统 / 目录指纹 / 归档 / 内容异常）不出现在主界面 |
| 受影响 | 页 P-presets；实体 E-06 / E-14；文件 `presetTree.ts`、`PresetTable.tsx` |
| 验收标准 | 用户两步内能判断"这份能不能用、要不要更新"；界面文本里不再出现 baseline / 血统 / 目录指纹 / 归档 四个词 |
| 尚未确定 | "内容异常"（字节被改过）要不要给用户看，还是下载时静默修复 |

## RD-04 动作词：只留「取回」与「启用」

| 项 | 内容 |
| --- | --- |
| 当前问题 | 「使用 / 应用」是同一件事的两个入口；「下载 / 更新」是同一条命令的两种文案；入口散在首页、预设页、参数页（CX-04 / CX-07） |
| 当前行为 | `applyActivePreset` 被首页「应用」与预设页「使用」同时使用；`fetchOfficialPreset` 同时承担"下载"与"更新" |
| 证据 | `src/app/presets/PagePresets.tsx::1046`、`src/app/home/PageHome.tsx::461`、`src-tauri/src/ipc/catalog.rs::434` |
| 业务价值 | 一个动作一个词、一个词只指一件事 —— 这是"看起来像正常软件"的最低要求 |
| 裁断 | **合并** |
| 建议的新规则 | 统一为两个动词：**「取回」**（把官方的取到本机，幂等，不动我的东西）与**「启用」**（把某一份设为当前使用，只写指针）。删掉「应用 / 使用它 / 下载并应用 / 更新」这一堆同义词 |
| 受影响 | 页 P-presets / P-home / P-params；实体 E-08；命令 `applyActivePreset` `fetchOfficialPreset` `downloadCatalogFile` `copyToSlicer` |
| 验收标准 | 全局搜索"应用 / 使用它 / 下载并应用"不再出现；每个动作词只对应一个命令 |
| 尚未确定 | 「启用」的入口放在哪一页（预设表 vs 首页） |

## RD-05 内部概念一律内部化

| 项 | 内容 |
| --- | --- |
| 当前问题 | 详情面板与归档抽屉暴露血统三行、目录指纹、事件时间、下载区路径；归档还给用户删（CX-03 / CX-09） |
| 当前行为 | 展开详情渲染 `based_on*`、"云端最新版本"（catalog revision）、"本机这份发布于"、"旧版本 N 份 → 抽屉可删" |
| 证据 | `src/app/presets/PresetTable.tsx::1001`、`src/app/presets/PagePresets.tsx::688` |
| 业务价值 | 这些概念对"用预设"不是必需的；它们的存在是为了内部追溯 |
| 裁断 | **降级为内部** |
| 建议的新规则 | 血统 / baseline / 归档 / 事件账 / 目录指纹全部内部化；界面只回答"这份是哪来的（官方 / 我改的 / 导入的）""什么时候变的""要不要更新"。**取消面向用户的归档管理与删除入口** |
| 受影响 | 页 P-presets；实体 E-04 / E-05 / E-09 / E-13 / E-15；命令 `getArchivedFiles` `deleteArchivedFile` `readArchivedText` |
| 验收标准 | 预设页与详情里不再出现内部术语；归档没有面向用户的删除入口 |
| 尚未确定 | 官方旧字节是否还需要保留（建议保留但纯内部，作为"认得出旧版"的证据） |

## RD-06 状态改由后端直接给

| 项 | 内容 |
| --- | --- |
| 当前问题 | 行状态由前端从 5 个读拼装（catalog + downloaded + stale + trust + localFiles + baseline + 血统）（CX-05） |
| 当前行为 | `presetTree.ts` 的 `cloudRows` / `localRows` / `statusOf` 承担全部状态推导与按钮判据 |
| 证据 | `src/app/presets/usePresetData.ts::480`、`src/app/presets/presetTree.ts::448` |
| 业务价值 | 状态判据只有一处，否则"改了后端忘了前端"就会撒谎 |
| 裁断 | **重做** |
| 建议的新规则 | 后端给"用户会问的每一个状态"的**单一字段与单一枚举**；列表接口直接返回行模型（含状态与可用动作）；前端不组合、不推断 |
| 受影响 | 页 P-presets；实体 E-03 / E-06；命令 `getDownloadedFiles` `getStaleFiles` `getDeliveryTrust` `getLocalFiles` `getUserPresetFiles` |
| 验收标准 | 前端没有第二处状态判据；新增一种状态只需改后端一处 |
| 尚未确定 | 列表接口是否合并成一个（含分页） |

## RD-07 「查看」与「启用」分离（去掉"浏览即切换"）

| 项 | 内容 |
| --- | --- |
| 当前问题 | 在首页/参数页的预设抽屉里选中一份，就等于改「当前使用」（X-18） |
| 当前行为 | `appState.ts::activateCombo::116` 在**浏览**时直接 `applyActivePreset` |
| 证据 | `src/app/state/appState.ts::116`、`src/app/home/PageHome.tsx::274` |
| 业务价值 | 看一眼参数不该改底账 |
| 裁断 | **重做** |
| 建议的新规则 | 点一份是**预览**（只改当前视图）；要启用必须显式点「启用」 |
| 受影响 | 页 P-home / P-params / P-calib；实体 E-08；命令 `applyActivePreset` |
| 验收标准 | 打开任何抽屉/列表浏览不会改变底账；底账只在显式「启用」后变化 |
| 尚未确定 | 预览态下"当前浏览的是哪一份"要不要在界面上有痕迹 |

## RD-08 一份文件的编辑只有一条链

| 项 | 内容 |
| --- | --- |
| 当前问题 | 正文编辑（`putPresetDraft`）与参数编辑（`patchPresetDraft`）共用 `run/app-state.json#draft`，但入口与语义是两套（CX-12） |
| 当前行为 | 预设页「改这份」走整份正文；参数模态框走单字段 patch；两者都写同一格草稿 |
| 证据 | `src-tauri/src/ipc/mine.rs::351`、`::374` |
| 业务价值 | 同一份文件的编辑应只有一条链、一种保存语义 |
| 裁断 | **合并** |
| 建议的新规则 | 统一为"一份文件的未保存改动"：无论改整段文本还是某个字段，都是同一份 working copy 的 diff；保存只有一件事（写回这一份） |
| 受影响 | 页 P-presets / P-params；实体 E-06 / E-07；命令 `beginPresetEdit` `putPresetDraft` `patchPresetDraft` `commitPresetDraft` `discardPresetDraft` `savePresetParams` |
| 验收标准 | 同一份文件在同一时刻只有一个未保存改动；保存只写一个文件、无第二份产物 |
| 尚未确定 | 直接编辑 TOML 正文是否保留为"高级入口" |

## RD-09 数据源内部化

| 项 | 内容 |
| --- | --- |
| 当前问题 | 「预设数据源」是发布方/构建期的事，却做成设置页里用户可点的单选（含自定义地址）（CX-11） |
| 当前行为 | 设置页列内置源（github/gitee）+ 自定义地址输入；改完立即重读；后端"先探一次网络再落盘" |
| 证据 | `src/app/settings/PageSettings.tsx::296`、`src-tauri/build.rs::28` |
| 业务价值 | 内网/自建源确实存在需求，但不能让普通用户面对"数据源"这个概念 |
| 裁断 | **降级为内部** |
| 建议的新规则 | 源来自构建期注入的清单；用户界面不出现「数据源」。保留一个隐藏的开发/排障入口（设置页最底部高级项，默认折叠） |
| 受影响 | 页 P-settings；实体 E-01；命令 `getPresetSource` `setPresetSource` `clearPresetSource` |
| 验收标准 | 普通用户在设置页看不到数据源选项；默认源由构建期决定 |
| 尚未确定 | 多源（境内/境外）要不要在首启时问一次 |

## RD-10 三条追溯链合并成一条

| 项 | 内容 |
| --- | --- |
| 当前问题 | 归档 / 版本链 / 事件账三条持久化链都只为追溯，其中版本链与事件账**没有删除入口**（CX-09 / CX-08） |
| 当前行为 | `archive/<path>`（可删）、`archive/catalogs/<rev>.json`（只追加）、`preset_events.json`（只追加）；三者在 sha 上重叠 |
| 证据 | `src-tauri/src/runtime/preset_events.rs::104`、`src-tauri/src/runtime/release.rs::48`、`src-tauri/src/runtime/delivery.rs::474` |
| 业务价值 | 「这份字节是哪一版官方」与「什么时候变的」确实是判据（识别旧版、显示时间） |
| 裁断 | **合并** |
| 建议的新规则 | 只保留**一份**"官方版本记录"（建议：按 sha 索引的『我见过哪些官方版 + 何时到达 + 正文快照（可选）』）；目录链与归档降为它的实现细节；不再有面向用户的归档管理 |
| 受影响 | 实体 E-04 / E-05 / E-13；命令 `getArchivedFiles` `deleteArchivedFile` `getOfficialVersions` |
| 验收标准 | 只有一个模块负责回答"这一版的来路与时刻"；用户界面无归档管理 |
| 尚未确定 | 事件时间是否还要精确到时刻，还是只要"本次 / 上次" |

## RD-11 血统与归属瘦身

| 项 | 内容 |
| --- | --- |
| 当前问题 | 文件头承载三行血统 + 两行归属；改归属要重写文件（E-15 / E-16 / CX-18） |
| 当前行为 | `# based_on / # based_on_release_time / # based_on_sha256` 与 `# machine: / # variant:` 都写在预设文件里 |
| 证据 | `src-tauri/src/runtime/lineage.rs::63`、`::339` |
| 业务价值 | "血统随文件走"的优点是真的：拷到别的机器也说得清来源 |
| 裁断 | **保留（但瘦身）** |
| 建议的新规则 | 保留"随文件走"，但压缩到**一行**（如 `# from: <官方文件名>@<sha 短>`）；归属从文件头移到应用侧记录（**改归属不该重写文件**） |
| 受影响 | 实体 E-15 / E-16；命令 `setUserPresetMachineVersion` `beginPresetEdit` `commitPresetDraft` |
| 验收标准 | 改归属不重写预设文件；换机器打开仍知道来源 |
| 尚未确定 | 归属属于"文件"还是"我的收藏库" |

## RD-12 官方更新的默认路径简化

| 项 | 内容 |
| --- | --- |
| 当前问题 | 逐参数官方更新是一条重流程：baseline + 决策账 + 三方对照 + 逐项 adopt/hold + 批量复核弹窗（E-09 / E-10） |
| 当前行为 | 打开参数 → 看到"官方更新待选择 N 项" → 逐项「用新值 / 保持我的」，或进「选择要跟随的官方更新」一屏勾选 |
| 证据 | `src-tauri/src/runtime/param_sync.rs::96`、`src/app/params/ParamSyncModal.tsx::88` |
| 业务价值 | 「官方改了几项、我要不要跟」是真实需求；**用户手改过的项不能被顶掉** |
| 裁断 | **保留能力 + 重做默认路径** |
| 建议的新规则 | 默认路径简化成两句话：①官方改过 N 项 → 一键「全部跟随」/「全部保持」；②逐项列表只在用户点「看看改了哪几项」时展开。决策记录从"水位摘要"改为"**按项记下我的选择**"，不再需要 baseline 才能算 |
| 受影响 | 页 P-params；实体 E-09 / E-10；命令 `getPresetParamSync` `applyPresetParamDecisions` |
| 验收标准 | 不打开逐项列表也能完成"全部跟随/全部保持"；手改过的项在任何默认路径下不会被覆盖 |
| 尚未确定 | 「全部保持」是否要提供撤销 |

## RD-13 页签只留有事可做的

| 项 | 内容 |
| --- | --- |
| 当前问题 | 6 个页签里「报告」是占位；「参数」已于 2026-10-09 退场；「同步」早已退役（CX-14 / X-16） |
| 当前行为 | `App.tsx` 给每个页签一个 `PageSlot`；报告挂 `PagePlaceholder` |
| 证据 | `src/app/App.tsx::126`、`src/app/constants/tabs.ts` |
| 业务价值 | 页签是用户的心智地图，空格子会让人以为功能缺失（或找不到东西） |
| 裁断 | **砍掉** |
| 建议的新规则 | 页签只保留**真的有事可做**的页；功能没做完就不上导航 |
| 受影响 | 页 P-report；文件 `src/app/constants/tabs.ts` |
| 验收标准 | 每个页签都有真实数据与动作 |
| 尚未确定 | 报告 / 后处理这块要不要进新版本（见 `U-11`） |

## RD-14 删除与「当前使用」解耦

| 项 | 内容 |
| --- | --- |
| 当前问题 | 删文件会连带撤指针、丢草稿、删备注，确认框只用一句话交代（X-18 邻近问题 / 动作 A-30） |
| 当前行为 | `deleteUserPreset` / `deleteDeliveryFile` 在删除时一并清理 `activePreset` / `draft` / `remarks`（+ 决策账） |
| 证据 | `src-tauri/src/ipc/mine.rs::662`、`src-tauri/src/ipc/catalog.rs::746` |
| 业务价值 | 不留悬空指针是对的；但"删一份文件"不该悄悄改变"当前在用什么" |
| 裁断 | **重做** |
| 建议的新规则 | 删**正在使用**的那一份必须**显式**问"还要一并停用吗"；删其他份不影响指针；草稿属于哪一份，删除时明确告知会被丢弃 |
| 受影响 | 页 P-presets；实体 E-06 / E-07 / E-08；命令 `deleteUserPreset` `deleteDeliveryFile` |
| 验收标准 | 删一份不在使用的文件不改底账；删在使用的文件必须显式确认停用 |
| 尚未确定 | 「停用」之后要不要自动启用另一份（还是允许"没有启用任何一份"） |

## RD-15 错误不吞

| 项 | 内容 |
| --- | --- |
| 当前问题 | 前端 31 处 catch，多处把失败降级成空值 → 无法区分"没有"与"失败"（X-13） |
| 当前行为 | 设置页版本读取失败当空串；首页下载区/过时列表失败当空数组；`usePresetData` 多处 `.catch(() => {})` |
| 证据 | `src/app/settings/PageSettings.tsx::93`、`src/app/home/PageHome.tsx::361` |
| 业务价值 | 用户需要知道是不是网络/后端出错 |
| 裁断 | **重做** |
| 建议的新规则 | 三态显式（loading / error / ready）贯彻到每个读（这是仓库已有的纪律，只是有处违反）；"失败"与"空"永远长得不一样 |
| 受影响 | 页 P-shell / P-home / P-presets / P-settings；命令 `getPresetSource` `getAppVersion` `checkSoftwareUpdate` `getDownloadedFiles` `getStaleFiles` |
| 验收标准 | 任何读失败都有可见提示，不会被渲染成空数据 |
| 尚未确定 | — |

## RD-16 写盘判据覆盖 `src-tauri`

| 项 | 内容 |
| --- | --- |
| 当前问题 | 写盘纪律的源码扫描只扫两个 crate；clippy 不管删除/改名/复制（X-17） |
| 当前行为 | `write_discipline_scan.rs` 的 `FORBIDDEN` 清单不覆盖 `src-tauri`；`clippy.toml` 只管内容写 |
| 证据 | `crates/preset/tests/write_discipline_scan.rs::162`、`clippy.toml::39` |
| 业务价值 | 删除 / 改名 / 复制是最危险的操作，需要自动兜底 |
| 裁断 | **重做** |
| 建议的新规则 | 把 `src-tauri` 纳入同一条写盘扫描判据；**新增写盘入口必须登记**（删除/改名/复制也算），未登记的写法直接判据失败 |
| 受影响 | 文件 `crates/preset/tests/write_discipline_scan.rs`、`clippy.toml` |
| 验收标准 | `src-tauri` 的每一条写/删/改名都在白名单里 |
| 尚未确定 | 判据放在哪个 crate（建议独立成"架构判据"测试，不挂在 preset crate 下） |

## RD-17 不做空实现

| 项 | 内容 |
| --- | --- |
| 当前问题 | `open_model` 只打日志；`copyToSlicer` / `downloadFiles` / 「复制链接」未接线；首页「打开模型」是空动作（X-02 / X-03 / X-04 / X-06 / X-09） |
| 当前行为 | 命令层存在"返回成功但什么也没做"的实现；界面上摆着点了必然失败/无反应的入口 |
| 证据 | `src-tauri/src/ipc/mod.rs::128`、`src/api/bridge.ts::262`、`src/app/home/PageHome.tsx::995` |
| 业务价值 | "没做的事不要摆成入口"是仓库本来就写的纪律 |
| 裁断 | **砍掉** |
| 建议的新规则 | 凡"点了必然失败或什么都不做"的入口，重写时要么做实、要么不摆；命令层不做空实现（**宁可不注册**） |
| 受影响 | 页 P-home / P-calib / P-presets；命令 `openModel` `copyToSlicer` `downloadFiles` `getCalibModels` |
| 验收标准 | 界面里不存在"点了没反应"的动作；命令层没有只打日志的实现 |
| 尚未确定 | 校准模型下载要不要真的做（若做，需要一个真实的下载与打开链路） |

## RD-18 备注并入「我的那一份」

| 项 | 内容 |
| --- | --- |
| 当前问题 | 备注覆盖账是独立持久化 + 独立命令 + 详情编辑格，只为副标题服务（CX-13） |
| 当前行为 | `user/preset-remarks.json` 以"文件身份"为键；本地表读它，云端表不读 |
| 证据 | `src-tauri/src/runtime/remarks.rs::33` |
| 业务价值 | 用户想给一份预设写句自己的话，是合理的小需求 |
| 裁断 | **降级为内部** |
| 建议的新规则 | 改备注 = 改"我的那一份"的属性（随我的记录走），不再单独一本账；官方行的备注**只读**、不提供编辑 |
| 受影响 | 页 P-presets；实体 E-06 / E-11；命令 `getPresetRemarks` `setPresetRemark` |
| 验收标准 | 只有一本记录存"我自己给这份写的话" |
| 尚未确定 | 官方行的备注是否需要在界面上显示（建议：不显示，或只在详情次要位置） |

## RD-19 应用数据层收敛

| 项 | 内容 |
| --- | --- |
| 当前问题 | 首页 / 预设 / 参数 / 校准各拉各的同一批数据（`usePresetData`、`usePreset`、`useCatalog`、`useParams`）（CX-19 类问题） |
| 当前行为 | 四个 hook 各自读 catalog 与盘上状态，各持有快照 |
| 证据 | `src/app/presets/usePresetData.ts::435`、`src/app/home/useCatalog.ts::67`、`src/app/params/useParams.ts::690` |
| 业务价值 | 同一事实不该有三份会漂移的快照 |
| 裁断 | **合并** |
| 建议的新规则 | 一层「应用数据层」（一个 store + 一个后端读），各页从它取；跨页一致性由它保证 |
| 受影响 | 页 P-home / P-presets / P-calib / P-params；实体 E-02；命令 `getRuntimeCatalog` `getMachines` `getVersionFiles` `getPresetFiles` |
| 验收标准 | 同一事实在一个会话里只有一个快照 |
| 尚未确定 | 是否引入前端状态库（当前是自建 `useSyncExternalStore`） |

## RD-20 BBS 页降为只读参考查看器

| 项 | 内容 |
| --- | --- |
| 当前问题 | BBS 页与预设页是两套世界：数据源不同（public/dev 端点 vs `src/api`）、有"导入"但只在内存、有"可改动"模式但落点不明（CX-17 / X-14 / 动作 A-50/A-52） |
| 当前行为 | `useBbsData` 走 `fetch /bbs/*.json` 与 dev 端点；`preset.setValue` 只改本页状态 |
| 证据 | `src/app/bbs/useBbsData.ts`、`src/app/bbs/BbsControl.tsx::89` |
| 业务价值 | "看别人的 profile 长什么样"是查看器需求，与"管理我的预设"不同 |
| 裁断 | **降级为内部** |
| 建议的新规则 | 定位成**只读参考查看器**（可载入、可对照、不落盘）；**去掉「可改动」这一档**；数据统一走后端契约（不再走 dev 端点） |
| 受影响 | 页 P-bbs；命令 `readDownloadedText` `getRuntimeCatalog` |
| 验收标准 | 这一页没有任何写动作；数据来源与其它页一致（都走后端契约） |
| 尚未确定 | 本地 BBS 目录扫描是否保留 |
