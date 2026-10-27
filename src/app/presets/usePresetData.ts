/*
 * 预设页的数据层（按机型汇总 + 已应用改读契约）。
 *
 * 两个 hook，分工是「拉数据」与「这一屏要显示什么」：
 *
 *   `usePresetData()`  拉齐几个读、建树。**首页也用它**（四步选择要整棵树），所以它的
 *                         返回形状不能随预设页的版面变 —— 这一轮只给它**加**了 `applied`
 *                         与 `pickMachine`，已有的字段一个都没删、没改名
 *   `usePresetPage()`  预设页自己的：两条分段控件（类型 / 位置）、搜索词、置顶集合，
 *                         以及算出来的**两张互不相干的表**
 *
 * # 拉哪些读
 *
 *   预设仓库清单        api.getPresetFiles()                      → 20 个（14 默认交付 / 6 可选）
 *   机型与版本          api.getMachines()                         → 6 机型 / 10 版本
 *   菜单三态            api.getMenu()                             → 14 已分配 / 6 可选 / 0 仅归档
 *   本机已有哪些文件    api.getLocalFiles()                       → **固定演示集合**，实测 3 个（2 MKP / 1 BBS）
 *   用户自己的文件      api.getLocalPresetFiles()                  → **本地文件**（扫 presets-local/），空是合法状态
 *   当前使用的那一条    AppState 的 activePreset 格（`run/app-state.json`，经唯一客户端订阅）→ **全局唯一**，null = 一套都还没应用
 *   已复制到切片器目录  api.getSlicerCopied()                     → **固定演示集合**，实测 1 个
 *   每个组合的文件      api.getVersionFiles(machineId, versionId) → 9 个组合各 2 个，A2L/STANDARD 是 incomplete
 *   与出厂不同 N 项     api.getMachineParams(machineId, versionId) 里 origin === 'variant' 的条数
 *
 * **三态显式**（loading / error / ready），不静默给空数组 —— 拉不到就说拉不到，
 * 给空数组的话界面会显示「共 0 个文件」，那是假信息。
 * 当前使用为 `null` **不是错误**（新装的机器就是这个状态），所以它不走 error 那支。
 *
 * # 两种类型的「生效」是两件不同的事
 *
 * ```
 * MKP     生效 = 设为当前使用        唯一底账 run/active-preset.json  状态 已应用 / 未应用
 * 切片器   生效 = 复制到切片器的目录   api.copyToSlicer()     状态 已复制 / 未复制
 * ```
 *
 * MKP 那个写走新世界的应用命令（IPC 落 `run/active-preset.json`），写完**重读底账**而不是自己在本地改状态：
 * 界面看到的「已应用」必须是底账答的，不是前端猜的 —— 那正是 A34 纠正过的那个错
 * （已应用不是派生量）。切片器照旧走契约（假后端只改内存，刷新还原）。
 *
 * # 为什么 20 多个请求一起发
 *
 * 「与出厂不同 N 项」是参数的事，契约里没有「给我每个版本改了几项」这个读。所以这里自己算：
 * 10 个组合 × 2 个读。假后端是同进程的同步数据，实测不需要分页或懒加载；真后端接上之后
 * 如果这里变慢，该加的是一个专门的读，不是在界面上打补丁。
 *
 * # 「已应用」不再由前端推
 *
 * 原来的判据是「当前机型 + 当前版本那个默认交付的 MKP 预设」—— 切一下机型就换一个，
 * 等于说这台机器同时应用着 6 套配置。作者的原话：**「已应用只有一个，所有机型所有版本
 * 始终只有一个已应用」**。所以它现在是**唯一底账答的一条独立事实**（新世界
 * `run/active-preset.json`）：整张表里最多一行带 ● 已应用，切机型也不会变出第二个。
 *
 * # 这一页与参数页各有一份「当前机型 / 版本」
 *
 * 外壳按 tab 切换页面（切走就卸载），两页之间没有共享状态，跨页联动也没做。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, errorText } from '../../api'
import { activePresetSnapshot, appStateMutated, useActivePreset } from '../state/appState'
import {
  deliveryMutated,
  useDeliveryRevision,
  usePresetPrefsRevision,
  useSourceRevision,
} from '../state/deliveryState'
import { draftMutated, useDraftRevision } from '../state/draftState'
import type {
  ActivePreset,
  CopyToSlicerOutcome,
  ArchivedFile,
  CommittedDraft,
  DownloadOutcome,
  DownloadTick,
  Machine,
  CloudVersion,
  OnDiskFile,
  ParamDecision,
  PresetParamSync,
  PresetFileInfo,
  PresetTextDraft,
  FetchCloudResult,
  LocalFileIdentity,
  LocalPresetIdentity,
} from '../../api'
import { isAppError } from '../../api/contract'
import { useLocale, useT } from '../../i18n'
import { localizeMachines } from '../shared/dataLabel'
import { STORAGE } from '../../api/storageKeys'
import { useSessionState } from '../shared/useSessionState'
import type { DeliveryPresetSource } from './presetTree'
import {
  applySlicerFilters,
  archivedIds,
  buildPresetTree,
  catalogKindToFileKind,
  cloudRows,
  localRows,
  machineNode,
  machinesInScope,
  slicerFilterValues,
} from './presetTree'
import type {
  PresetCloudRow,
  PresetKindAxis,
  PresetLocalRow,
  PresetRowsInput,
  PresetScopeAxis,
  PresetSort,
  PresetSortDir,
  PresetSortKey,
  PresetTableData,
  PresetTree,
  PresetFileState,
  LocalPresetFile,
  PresetVersionInput,
} from './presetTree'

export interface PresetData {
  loading: boolean
  /** null = 没出错。非 null 时页面要把这句话显示出来，不要装成空表 */
  error: string | null

  /**
   * **远端这一代目录本客户端读不懂** —— 后端在 `applyRemoteUpdate` 时返回了 `NOT_SUPPORTED`。
   *
   * ★ 它与 `error` **不是一回事**，页面也不许把它当整页错误：
   *   - `error` = 本机那份底账读坏了（该修的在这里）；
   *   - 这一条 = **远端发布了新结构，当前客户端还不具备读它的能力** ——
   *     本机什么都没坏、也没有任何东西失败，只是"你想给本地这份我读不了"。
   *
   * 用户要做的**唯一一件事**是去设置页更新客户端（不是"重试"、不是"再下一遍"）。
   * 因此它**不改目录、不落页级错误**（作者定的产品规则 B：列表照常，只在预设页出现一句提示）。
   */
  needsNewerClient: boolean

  /**
   * 交付预设：目录（catalog）登记的交付预设全量 + 下载区（`mkp/`）现况。
   *
   * 两份底账都住 Internal 根：清单在 `catalog.json` 的 files 域（发布方写），
   * 「已下载」在 `mkp/`（盘就是底账）。这一层只读不写；写走下面的 `downloadPreset` / `apply`。
   */
  delivery: PresetFeed

  /**
   * 归档区里躺着的**云端旧版本**（换版本时被换下来的那一份）。
   *
   * 与交付那一路同源：跟 `delivery` 一起读、同样**不挡首屏**，每次下载 / 更新之后跟着重读
   * （刚更新过的那一份会出现在这里）。**只读** —— 界面不删、不恢复、也不拿它当"用户修改历史"：
   * 归档是云端版本生命周期的一部分（见 `ArchivedFile` 的注释）。
   */
  archived: ArchivedFile[]

  /**
   * **云端版本账**（`api.getCloudVersions()`，2026-10-08）：交付预设登记过哪几版、
   * 哪几版这一机已经下过。云端表 MKP 档按它把每个预设摊成**多行版本**
   * （「已下载 / 新版本」）。
   *
   * ★ 它**不是"过时判定"**：不读用户预设的字节、不看血统，也不在任何一行上说"该更新了"。
   */
  cloudVersions: CloudVersion[]

  machines: Machine[]
  /** 当前机型。**空串 = 「全部机型」那一档**（预设页才用得到，首页永远是一台具体的） */
  machineId: string
  versionId: string
  machine: Machine | undefined
  /**
   * **当前数据源**（`getPresetSource()` 的 mode：`gitee` / `github` / `custom`）——
   * preset 行「来源」列显示 GitHub / Gitee / 自定义源的依据。null = 没配源 / 读取失败
   * （来源那一格照实退「云端」，不让整页因为这一格失败）。
   */
  sourceLabel: string | null
  /** 未过滤的整棵树（仅归档的文件建树时就剔掉了）。计数用它 */
  tree: PresetTree
  /**
   * 本机已有哪些文件（asset id）。
   * 假后端给的是**固定演示集合**（`src/api/mockServer` 的 `localFiles.ts`）。
   */
  localIds: string[]
  /**
   * **本地文件**：用户自己放的那一份（`presets-local/` 里扫出来的）。
   *
   * **云端没有它们**，所以它们只出现在本地表里、没有交付身份、也不参与套餐与菜单。
   * 认不出类别的那几份（`.json`）也在里面 —— 界面上任何类型档下都列，不藏。
   */
  local: LocalPresetFile[]
  /**
   * **身份账本**（a04）：哪几份本地文件跟云端有关系 —— 云端行三态与「更新」
   * 找要更新的是哪一份，都只看它（复制出来的副本不在里面）。
   */
  localIndex: LocalPresetIdentity[]
  /**
   * **有未保存参数改动的那一份**（相对用户根的路径；`null` = 现在没有）——
   * 预设列表行上那枚「未保存」小圆点的判据（参数页改过值、还没点保存）。
   * 来源是 `api.getPresetDraftSummary()`；参数页那边一动值就广播（`draftState`）。
   */
  draftPath: string | null
  /**
   * **备注覆盖账**（`api.getPresetRemarks()`，键 = 文件身份）：用户改过的副标题。
   * **只有本地表读它**（云端表只读工作台那句 —— 本地改动不许影响云端显示）：
   * 本地副标题 = 账上有的 → 那一版工作台写的 → 空着。
   */
  remarks: Record<string, string>
  /**
   * **改一份预设的备注**（本地副标题覆盖账）。**写什么就是什么 —— 包括空串**
   * （空 = 副标题留空，不回退）；`null` = 恢复默认（退回「工作台写的 → 空着」）。
   * 「更新不覆盖」靠账在，不在文件里。
   */
  setRemark: (key: string, remark: string | null) => Promise<void>
  /**
   * **改一份用户预设的归属**（机型 / 版本）：文件头那两行换成新值，
   * 回来重读本地文件。机型必须目录里登记的；**版本可自定义**（后端校验机型）。
   */
  setLocalMachineVersion: (path: string, machineId: string, versionId: string) => Promise<void>
  /**
   * 正在使用的**唯一那一条**：从底账（`run/active-preset.json`）读出来的，
   * **全表最多一份**，`null` = 一套都还没应用（**不是错误**）。
   *
   * 交付预设行应用后写着的都是它 —— 这是「使用中」的唯一来源
   * （旧契约 `getAppliedPreset` 与假后端那份演示值 2026-10-02 清扫时已删）。
   */
  active: ActivePreset | null
  /**
   * 哪些切片器 profile **已经复制到切片器的 profile 目录**了（asset id）。
   *
   * 和 `localIds` 是两件事：在本机 ≠ 切片器看得见它。切片器行的「已复制 / 未复制」只看这一个。
   * 假后端同样是固定演示集合（实测 1 个）。
   */
  slicerCopied: string[]
  /**
   * **重读本地文件 + 身份账本**（2026-10-20）。切进「云端」那一页时对着它刷新那三态。
   *
   * ★ 只读**本机**（扫盘 + 账本），**不发网络** —— 切页签不该顺手发一次网络请求。
   */
  refreshLocal: () => Promise<void>
  pick: (machineId: string, versionId: string) => void
  /**
   * 只换机型（预设页那个下拉）。版本跟着落到这台机型的第一个 —— 版本 id 在机型之间
   * 不通用（LITE 只有 P1S / X1C 有），留着上一台的 id 会指到一个不存在的组合。
   * 传空串 = 「全部机型」，那一档没有「当前版本」这回事。
   */
  pickMachine: (machineId: string) => void
  /**
   * 把某一份**本地文件**设为**当前使用的那一条**（身份 = **文件名 + 相对用户根的路径**）。
   * 云端的版本不能直接用 —— 要先用 `fetchCloud` 把它下载到本地。
   *
   * 走应用命令（`api.applyActivePreset(fileName, 'local', path)`，后端按闸校验后写
   * `run/active-preset.json`）**然后重读底账** —— 界面看到的永远是底账答的，
   * 与 `copy` 同一条规矩。失败照抛给调用方（页面用提示条说出来），**不在这里吞**。
   *
   * ★ 2026-10-13 修一个真 bug：这里原来只把 `path` 当唯一实参传下去，于是
   *   `fileName` 收成了 `presets-local/A1_MINI-fast.toml`、`origin` 缺省成 `preset` ——
   *   后端拿这串带前缀的名字去**交付目录**里找，报「目录里没有 presets-local/…」，
   *   本地表那颗「使用」**一份都点不动**。文件名与路径是两个参数，这里必须分开传。
   */
  apply: (fileName: string, path: string) => Promise<void>
  /**
   * **把交付预设那一份取到本机**（2026-10-09 改判）—— 云端表那两个动作（下载 / 更新）共用它。
   *
   * 它做两件幂等的事：云端当前版字节不在本机（或对不上）就取回来；本地本地文件
   * 不在就落一份（在了一个字节不动）。**它不改「当前使用」** —— 使用是本地表那颗按钮。
   *
   * 回来**两边一起重读**：投递面可能刚变（云端字节取回来了），本地文件可能也多了一份。
   * 返回值说清这次到底做成了什么（提示条照它说）。
   */
  /** `renameIfTaken`（a04）：撞名时用户点过「导入」再传 `true` —— 自动改名落一份新的 */
  fetchCloud: (fileName: string, renameIfTaken?: boolean) => Promise<FetchCloudResult>
  /**
   * **撤销应用** —— 把"当前使用的那一条"撤掉（`api.clearActivePreset()`），然后重读底账。
   *
   * 与 `apply` 同一条规矩（写底账 → 重读底账）；幂等：本来就没在应用也不报错
   * （后端 `clear_active` 对"文件不在"就是成功）。**它不碰任何文件** ——
   * 撤掉的是"哪一份在生效"这个指向，磁盘上的预设一个字节不动。
   */
  clearApply: () => Promise<void>
  /**
   * 把某个切片器 profile 复制进切片器目录，然后重新拉 `getSlicerCopied()`。同上。
   *
   * 回来给一句结果（`replaced`）：**那格同名就换掉**（2026-10-13 作者改判）——
   * 提示条按它说清这次是新放了一份，还是把原来那份替换了。
   */
  copy: (assetId: string) => Promise<CopyToSlicerOutcome>
  /**
   * 「下载」目录里登记的那一份：走新世界下载管道（`api.downloadCatalogFile(fileName)`），
   * 落进下载区 `mkp/`。盘上那一份对不上目录时（需更新）走的是**同一条路** ——
   * 再下一遍，旧份自动归档，没有单独的"更新"代码路径。
   *
   * `onTick` 给了就把过程说出来（一次调用一路水位，后端在下载过程中推）；不给就是
   * 原来那个"点了等结果"。**下载快慢由网络决定，界面上说真的字节数，不编进度条。**
   *
   * 与云端文件那颗「下载」（`api.downloadFiles()`，假后端必抛未实现）不同，
   * 这一条**真的能下**。失败照抛给页面说出来。
   */
  downloadPreset: (fileName: string, onTick?: (tick: DownloadTick) => void) => Promise<void>
  /**
   * **一次处理多份**（批量补齐 / 批量更新）。
   *
   * 就是上面那一条的复数版：同一个后端命令族（`api.downloadCatalogFiles`，它在 Rust 侧
   * 逐份跑**同一个** `deliver`、并发也在那边做），同一条水位，同一次"重读底账"。
   * **逐份结局按请求顺序返回**给页面说出来 —— 这里不聚合、不吞：一份失败就是那一条
   * `ok: false`，页面照实列出来（"批量失败"这种话是后端与页面都不许说的）。
   *
   * 传进来的这几份**已经由调用方按行上的状态挑过**（未下载 + 需更新，已下载的不进来）——
   * 这一层不重新判断谁该下。
   */
  downloadPresetBatch: (
    fileNames: string[],
    onTick?: (tick: DownloadTick) => void,
  ) => Promise<DownloadOutcome[]>

  /**
   * **打开正文编辑器（「编辑文件」）**：初稿 = 盘上那份文件**本身的字节** ——
   * 参数页未保存的改动一个字都不带进来（两条草稿分开存，见契约 `PresetTextDraft`）。
   * `reused` = 接着上次那半截改。
   */
  beginTextEdit: (fileName: string, path: string) => Promise<PresetTextDraft>
  /**
   * **这份文件的原文**（最初落到盘上时的那一份）——「恢复默认值」那颗退回的地方。
   * `null` = 没有原始备份（按钮禁用）。见契约里的同名接口。
   */
  readLocalOriginText: (path: string) => Promise<string | null>
  /** 正文的改动写进正文草稿（边改边存 —— 改到一半关掉也还在；盘上那份文件不碰） */
  putTextDraft: (text: string) => Promise<void>
  /**
   * 放弃这次正文编辑（丢正文草稿；盘上那份文件全程没被碰过，所以它天生安全）。
   *
   * ★ 2026-10-16：现在是「放弃并退出」的作废那一步在调用（丢草稿 → 重读盘上字节回填，
   *   见 `PagePresets` 的 `exitEdit`）—— 作者：「没有保存的话，退出编辑模式就是作废，
   *   那些改的都作废，保存了才有」。
   */
  discardTextDraft: () => Promise<void>
  /**
   * 保存正文：**写回它自己**（第八层，整份原子替换），然后丢掉正文草稿。
   * 写完**重读本地文件**（大小与时刻跟着变）—— 参数页那份草稿此时已被后端作废，
   * 所以还要广播一声草稿代次（列表上的「未保存」标记跟着收）。
   */
  commitTextDraft: () => Promise<CommittedDraft>

  /**
   * **一份本地文件现在的「云端更新」账**（逐参数三方：本地值 / 云端旧值 / 云端新值）。
   * 点「更新」时给 `fetchMissing: true` —— 允许为拿到云端当前版字节发一次请求
   * （★ 那是**唯一**为看差异发网络的两个入口之一，2026-10-20）。
   * 判定（哪几项待处理、哪几项是冲突）都在后端，前端只读这份账。
   *
   * `null` = 这一份不认云端（复制出来的那一份，且那个开关关着）。
   */
  presetSync: (path: string, fetchMissing?: boolean) => Promise<PresetParamSync | null>
  /**
   * **把逐参数的处理结果一次性写进本地文件**：`adopt` = 用云端新版值、`hold` = 保持本地值。
   * 结构保真地改那几项，写完**重读本地文件**。
   *
   * ★ 这是「更新」唯一的写入口：**没点确认之前一个字节都不写** ——
   * 无冲突的项自动 adopt、冲突的项按用户选择，全部决定凑齐了才调它一次。
   */
  applyParamDecisions: (path: string, decisions: ParamDecision[]) => Promise<void>
  /**
   * **重命名一份用户文件**（第十层）：只改名字，字节一个不动。回来**重读本地文件**
   * （列表立刻以磁盘为准），顺手重读使用中指针（它正指着这一份时会跟着改）。
   */
  rename: (path: string, newName: string) => Promise<LocalFileIdentity>
  /**
   * **另存为一份新的**（第十一层）：把我自己那一份按字节复制成同一格里另一份新的用户文件。
   * 回来**重读本地文件**（新文件要出现在表里）；**不重读使用中指针** —— 这一层不碰它
   * （原文件一个字节不动，复制出来的那份也不会自称"使用中"）。
   */
  copyAsNew: (path: string, newName: string) => Promise<LocalFileIdentity>
  /**
   * **删除一份用户文件**（第十层）：**真删除**（没有垃圾桶、没有归档）。回来重读本地文件。
   * 正在使用 / 有草稿不再拦（2026-10-06 一切皆可删）—— 后端把属于这一份的状态
   * 一并清掉；代价由页面的确认框讲清。
   */
  remove: (path: string) => Promise<void>
  /**
   * **删除本机那份交付预设文件**（2026-10-06 一切皆可删）：删了回「未下载」，
   * 随时可从云端重下。回来**重读云端那一侧**（那一行 + 归档清单一起刷新）。
   */
  removePreset: (fileName: string) => Promise<void>
  /**
   * **删除归档区一份旧版本**（2026-10-06 允许删，代价讲清：删了找不回）。
   * 回来重读云端那一侧（归档清单以盘为准）。
   */
  removeArchived: (path: string) => Promise<void>
  /**
   * **在文件管理器里显示**（第十三层）：打开 Finder / 资源管理器并选中这份用户文件。
   * **不重读任何东西** —— 它一个状态都不改（打开的是系统窗口，不是我们的界面）。
   */
  reveal: (path: string) => Promise<void>
  /**
   * 同上，另两个落点（2026-10-13）：**我们自己的下载区**（`assets/bbs/…`）与
   * **切片器目录里那份**（`user/<账号>/process/…`）。两个地方，两条命令，
   * 只给文件名 —— 路径由后端按目录登记拼。
   */
  revealDelivered: (fileName: string) => Promise<void>
  revealSlicer: (fileName: string) => Promise<void>
}

  /**
   * 「目录里登记的交付预设」这一路的现况。清单一份（catalog）、下载区一份 —— 两件事分开。
   * （「正在用的」不在这里：它只有一个出处 `run/active-preset.json`，见 `PresetData.active`。）
   */
export interface PresetFeed {
  /** 目录指纹（两端共用契约的 revision；旧世界的"包版本"没有对应物，指纹更诚实） */
  version: string | null
  /**
   * **发布时刻**（catalog 的 `publishedAt`，RFC3339 / UTC）—— 云端表「时间」列的来源，
   * 显示时由前端转本机时区。`null` = 这份目录没带（随包 bootstrap 目录、或旧版发布的
   * 目录没有这一格）—— 照实说「未知」，**不编**。
   *
   * ★ 名字就叫 `publishedAt`（2026-10-06）：它前一版叫 `at`，恒 `null` 且没人读，
   * 注释却写着"发布时刻" —— 那种"看起来接了线、其实没有"的字段就是「时间列永远是
   * 未知」那次误导的根源。改名之后它与目录里那一格同名，读的人不会认错来源。
   */
  publishedAt: string | null
  /** 目录里登记的交付文件（**预设页认的那两类**：MKP + 切片器；源头已按 kind 分好类别） */
  presets: DeliveryPresetSource[]
  /** 下载区（`mkp/`）里已有、**且与目录登记一致**的（`PresetFileState = ok`） */
  localPresets: DeliveryPresetSource[]
  /**
   * 下载区里有、**但与目录登记不一致**的（`PresetFileState = old / tampered`，
   * 即「旧版本 / 内容异常」）。
   *
   * 与 `localPresets` 是**两个读**：`getDownloadedFiles()` 只答"盘上有没有且对不对"，
   * `getStaleFiles()` 才答"盘上有一份但不能用"。少了这一个读，「旧版本 / 内容异常」就会
   * 被显示成「未下载」—— 用户点"下载"以为是第一次下，实际是在修一份坏档。
   *
   * 不一致的那几份**是哪一档**由第三个读（`getDeliveryTrust`）定：认得出是云端某一版旧版的
   * 是 `old`，哪儿都查不出的是 `tampered`（第三圈第 6 层）。
   */
  stale: DeliveryPresetSource[]
  /** 已下载的 uid 集合 */
  localUids: string[]
}

const EMPTY_PRESET_FEED: PresetFeed = {
  version: null,
  publishedAt: null,
  presets: [],
  localPresets: [],
  stale: [],
  localUids: [],
}

/**
 * 「本次运行已经检查过 Bootstrap 了吗」——**模块作用域**，所以它跟着这次 App 运行，
 * 不跟着这一页挂载。
 *
 * 这就是第十七刀那条判据的实现：**进入「预设」后台检查一次，本次运行只一次**。
 * 用户从 MKP 配置 → 切片器配置 → 搜索 来回切，页面反复挂载卸载，但这条只会走一次；
 * 关掉 App 再打开（页面重载）才重置。用会话态（`useSessionState`）不行 —— 那个是
 * 「切 tab 不失忆」，语义是"这一屏看到哪"，不是"这次运行做没做过一件事"。
 */
let checkedBootstrapThisRun = false

/** 后台检查一次的结果：换没换目录 + 远端这一代读不读得懂 */
interface BootstrapResult {
  /** 真的把本地目录换掉了（调用方据此重读那一路） */
  changed: boolean
  /** 远端有更新但**本客户端读不懂**（`NOT_SUPPORTED`）—— 列表照常，只多一句提示 */
  needsNewerClient: boolean
}

/**
 * 进入「预设」后的**后台检查**一次远端目录（第十七刀；第三刀下半补 readable / 读不懂那一路）。
 *
 * 判据（作者 2026-10-02，2026-10-04 补）：
 *
 *   - **只检查目录指纹**（`checkRemoteUpdate`，比的是 revision），不碰任何预设文件；
 *   - 有变化才 `applyRemoteUpdate` —— 换的是**本地那一份 catalog**（旧目录自动归档），
 *     **绝不自动下载预设文件**（用户点了"下载"才下）；
 *   - ★ 远端这一代**读不懂**时（`readable === false`，或 `applyRemoteUpdate` 抛回
 *     `NOT_SUPPORTED`）：**不换目录、不报错**，只把 `needsNewerClient` 立起来 ——
 *     预设页那句"此预设需要更新版 SupportEase"由它开（作者定的产品规则 B）；
 *   - 本次运行只做一次（`checkedBootstrapThisRun`）；
 *   - 其余失败**静默**：没内置源 / 没联网 / 远端还没部署都是开发期的正常状态，
 *     不许因此让预设页报错或弹提示（启动零网络那条纪律的延伸：这里只是"路过时问一声"）。
 */
async function checkBootstrapOnce(): Promise<BootstrapResult> {
  const idle: BootstrapResult = { changed: false, needsNewerClient: false }
  if (checkedBootstrapThisRun) return idle
  checkedBootstrapThisRun = true
  try {
    const check = await api.checkRemoteUpdate()
    if (check.upToDate) return idle
    /*
     * 先看能力再落盘：`readable === false` 时**根本不去 apply**（落盘前拦，
     * 后端那层也会再拦一道并回 NOT_SUPPORTED —— 两道闸说同一件事）。
     */
    if (!check.readable) return { changed: false, needsNewerClient: true }
    await api.applyRemoteUpdate()
    return { changed: true, needsNewerClient: false }
  } catch (e: unknown) {
    /*
     * `NOT_SUPPORTED` = 远端这一代读不懂（**不是失败**，是"我太旧了"）：
     * 不当页级错误、不改目录，只立起那句提示。
     */
    if (isAppError(e) && e.code === 'NOT_SUPPORTED') {
      return { changed: false, needsNewerClient: true }
    }
    /* 没配源 / 离线 / 远端没部署 / 坏档：都不该让预设页出问题 —— 静默略过 */
    return idle
  }
}

/** 建树前的四份输入（`usePresetData` 里存着它、按语言重出树） */
interface TreeSeed {
  list: Machine[]
  repo: PresetFileInfo[]
  inputs: PresetVersionInput[]
  archived: Set<string>
}

/**
 * `importRevision` 是"外部导入进来过几批"的钥匙（第十二层）：变了就**整屏重读** ——
 * 导入落进 `presets-local/` 之后，「本地文件」那张表要立刻以磁盘为准，不靠切页刷新。
 * 首页（`PageHome`）不给这个参数：它不看本地文件那张表。
 */
export function usePresetData(importRevision = 0): PresetData {
  const tr = useT()
  const locale = useLocale()
  const [machines, setMachines] = useState<Machine[]>([])
  /*
   * 建树那几份读的**原样结果**（不是树本身）。
   *
   * 存原样是因为树里有一句给人看的字（`后端没有 … 这个组合`）：语言一变，那句话要跟着换，
   * 而"重出一棵树"绝不能变成"重发那 20 多个请求" —— 页面是常驻挂的（`App` 的页槽只切
   * `data-hidden`），切语言时预设页并没有卸载，不重算树就会留着上一种语言的那句。
   */
  const [treeSeed, setTreeSeed] = useState<TreeSeed | null>(null)
  const tree: PresetTree = useMemo(
    () =>
      treeSeed === null
        ? { machines: [], fileCounts: { mkp_preset: 0, bbs_profile: 0, orca_profile: 0 } }
        : buildPresetTree(
            tr,
            localizeMachines(treeSeed.list, locale),
            treeSeed.repo,
            treeSeed.inputs,
            treeSeed.archived,
          ),
    [treeSeed, tr, locale],
  )
  const [localIds, setLocalIds] = useState<string[]>([])
  /* 远端这一代读不懂（NOT_SUPPORTED）：独立于 `error`，不挡列表、只开一句提示 */
  const [needsNewerClient, setNeedsNewerClient] = useState(false)
  /* 本地文件：用户自己的预设（`presets-local/`）。盘当底账 —— 首屏读一次；产生它的动作在下一层 */
  const [localPresets, setLocalPresets] = useState<LocalPresetFile[]>([])
  /* 使用中指针：AppState 唯一客户端订阅 —— 本页是写方之一，但快照同样只从客户端来 */
  const active = useActivePreset()
  const [slicerCopied, setSlicerCopied] = useState<string[]>([])
  const [ready, setReady] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [at, setAt] = useState<{ machineId: string; versionId: string } | null>(null)
  const [delivery, setDelivery] = useState<PresetFeed>(EMPTY_PRESET_FEED)
  /* 归档区（云端旧版本留档）。与 delivery 一起读、一起刷新（见 `readPresetFeed`） */
  const [archived, setArchived] = useState<ArchivedFile[]>([])
  /*
   * **云端版本账**（2026-10-08）：交付预设登记过哪几版、哪几版这一机已下过。
   * 与 delivery 一路读（它俩本来就同源：都是"云端登记了什么"），空数组 = 读不到 = 退回
   * "一个预设一行"的老画法。
   */
  const [cloudVersions, setCloudVersions] = useState<CloudVersion[]>([])
  /* 当前数据源（preset 行「来源」列的字）。读失败照实 null，不挡任何表 */
  const [sourceLabel, setSourceLabel] = useState<string | null>(null)
  /*
   * **备注覆盖账**（副标题）：用户改过的备注（键 = 文件身份）。只读一次 +
   * 写后重读整本 —— 与 delivery / local 同一条「界面读底账」的规矩。
   */
  const [remarks, setRemarks] = useState<Record<string, string>>({})
  /*
   * **有未保存参数改动的那一份**（路径）：预设列表行上那枚「未保存」圆点读它。
   * 首屏读一次 + 参数页那边一动值就广播重读（`draftState`）—— 页签常驻，
   * 不做这件事的话，用户在参数页改完切回来，标记还停在旧样子上。
   */
  const [draftPath, setDraftPath] = useState<string | null>(null)
  /*
   * **身份账本**（a04）：哪几份本地文件跟云端有关系（只有"从云端下载"建条目）。
   * 云端行那三态（未获取 / 本地已有 / 有新版）与「更新」找要更新的是哪一份，
   * 都只看它 —— 复制出来的副本、拖回来的、导入的都不在里面（它们不跟云端）。
   */
  const [localIndex, setLocalIndex] = useState<LocalPresetIdentity[]>([])
  /** 重读本地文件 + 身份账本（两个得一起读：判据是"账本里那一份文件"） */
  const readLocalPresets = useCallback(async () => {
    const [files, index] = await Promise.all([api.getLocalPresetFiles(), api.getLocalPresetIndex()])
    setLocalPresets(files)
    setLocalIndex(index)
    return files
  }, [])

  /**
   * 把交付预设这一路的现况读一遍。
   *
   * 全部来自新世界的底账：清单读 `api.getRuntimeCatalog()` 的 files 域，
   * 「盘上有没有、对不对」读 `api.getDownloadedFiles()` / `api.getStaleFiles()`
   * （`mkp/`，盘就是底账），「盘上那份认得出是哪一版吗」读 `api.getDeliveryTrust()`
   * （第三圈第 6 层），旧版本留档读 `api.getArchivedFiles()`。
   *
   * 下载 / 应用之后**重读一遍**而不是本地改状态 —— 与 `copy` 同一条规矩：
   * 界面上看到的必须是底账答的，不是前端猜的。
   */
  const readPresetFeed = useCallback(async (): Promise<PresetFeed> => {
    /*
     * 六个读：目录清单、盘上对得上的、盘上对不上的、**那些对不上的认得出是哪一版吗**、
     * 归档区里躺着的旧版本；外加 `getPresetFiles` —— 目录（catalog）里**没有**喷嘴 /
     * 层高，那两格只有它算得出来（按 `fileName` 对同一份文件，见
     * `DeliveryPresetSource.nozzle`）。中间三个合起来才是四档（见 `PresetFileState`）；
     * 归档是"更新过之后会变"的那一份，所以它跟着这一路一起读，而不是单开一次首屏读。
     */
    const [catalog, local, drifted, trust, keep, repo, versions] = await Promise.all([
      api.getRuntimeCatalog(),
      api.getDownloadedFiles(),
      api.getStaleFiles(),
      api.getDeliveryTrust(),
      api.getArchivedFiles(),
      api.getPresetFiles(),
      /* 云端版本账（2026-10-08）：云端表 MKP 档按它把每个预设摊成多行版本 */
      api.getCloudVersions(),
    ])
    setArchived(keep)
    setCloudVersions(versions)
    const downloaded = new Set(local.map((f) => f.fileName))
    const driftedSet = new Set(drifted.map((f) => f.fileName))
    /*
     * 盘上那几份的**事件时间**（2026-10-06 预设事件时间模型）：下载 / 替换两个事件
     * + 这份字节属于哪一代目录（`publishedAt`，版本出身反查）。已下载与有更新的是
     * 两批不相交的文件，合成一张表查；没有的事件是 `null` —— 界面照实说「未知」，不编。
     */
    const onDisk = new Map<string, OnDiskFile>([...local, ...drifted].map((f) => [f.fileName, f]))
    /* 判词按**文件名**查 —— 与下载 / 应用 / 读正文同一套口径（这套系统认的一直是 fileName） */
    const verdicts = new Map(trust.map((t) => [t.fileName, t.verdict]))
    /*
     * 四档**只在这里判一次**（两张表都读这一份结果，不许自己再判一次）。
     * 第三个读没给判词（两读之间有缝，理论上到不了）时按「内容异常」处理 ——
     * 那正是"认不出"的实情，而它的后果只是"得重下一份干净的"，宁可响。
     */
    const stateOf = (fileName: string): PresetFileState => {
      if (downloaded.has(fileName)) return 'ok'
      if (!driftedSet.has(fileName)) return 'missing'
      return verdicts.get(fileName) === 'old' ? 'old' : 'tampered'
    }
    /*
     * **分类判据是 catalog 的 `kind`**，不是扩展名、也不是"目录里只有预设"那个旧假设：
     * 预设页认的两类（MKP / 切片器）留下、带上类别；图标 / 模型等资源**不进这一页**
     * —— 它们由自己的资源体系消费。2026-10-02 作者截图里 `a1.svg` 和
     * `MKPProcess ….json` 混在「MKP 配置」表里，就是这一层没看 `kind` 造成的。
     */
    /* 喷嘴 / 层高的搬运表：`getPresetFiles` 的 `fileName` 与 catalog 的 `fileName`
       是同一把钥匙（都是文件的本名）；目录里没有这两格，这里只搬、不重算 */
    const axesByName = new Map(repo.map((r) => [r.fileName, r]))
    const listed: DeliveryPresetSource[] = catalog.files.flatMap((f) => {
      const kind = catalogKindToFileKind(f.kind)
      if (kind === null) return []
      return [
        {
          uid: `${f.machineId}/${f.versionId}`,
          machineId: f.machineId,
          versionId: f.versionId,
          fileName: f.fileName,
          kind,
          /* 目录登记的落点：备注覆盖账的键（「改了备注，更新不覆盖」认的就是它） */
          path: f.path,
          size: f.size,
          /* 喷嘴 / 层高（切片器 profile 才有）：从 `getPresetFiles` 的同一份文件上搬 */
          nozzle: axesByName.get(f.fileName)?.nozzle,
          layerHeight: axesByName.get(f.fileName)?.layerHeight,
          presetVersion: null,
          /* 事件时间（见 `DeliveryPresetSource` 三格的注释）；云端语义的时间走目录的发布时刻，两回事 */
          downloadedUnix: onDisk.get(f.fileName)?.downloadedUnix ?? null,
          replacedUnix: onDisk.get(f.fileName)?.replacedUnix ?? null,
          deliveryPublishedAt: onDisk.get(f.fileName)?.publishedAt ?? null,
          state: stateOf(f.fileName),
        },
      ]
    })
    /* 判据用 fileName：盘就是底账，盘上认的文件名 = 目录登记的文件名（不是 id、不是路径） */
    const localList = listed.filter((p) => p.state === 'ok')
    const staleList = listed.filter((p) => p.state === 'old' || p.state === 'tampered')
    return {
      version: catalog.revision,
      /* 发布时刻跟着目录走：随包 / 旧版目录没有这一格 → null（界面照实说未知） */
      publishedAt: catalog.publishedAt ?? null,
      presets: listed,
      localPresets: localList,
      stale: staleList,
      localUids: localList.map((p) => p.uid),
    }
  }, [])

  /*
   * **「读一遍 + 写快照」的唯一出口，带代次守卫（后发者优先）**。
   *
   * 快照有三个读源：首屏那条链、换目录后的重读、投递面代次触发的重读 —— 它们可能
   * **并发**（下载完的广播正好撞上首屏那条还在路上）。没有守卫时，**先发起、后返回**
   * 的旧读会把快照写回去（新数据闪一下又变回旧的）。
   *
   * 守卫只用一句话表达：**只有"最后发起的那次读"允许写快照**（`seq` 对上才写）。
   * 它盖住所有并发组合，不需要每条读源各写一套 cancel 逻辑。
   * 读失败照抛（调用方决定怎么说）——失败不写快照，保持上一份。
   */
  const readSeqRef = useRef(0)
  const readPresetFeedInto = useCallback(async (): Promise<void> => {
    const seq = ++readSeqRef.current
    const next = await readPresetFeed()
    if (seq !== readSeqRef.current) return
    setDelivery(next)
  }, [readPresetFeed])

  useEffect(() => {
    let alive = true

    const load = async () => {
      /* 本地文件那一读与其他几个一起发：它不挡首屏（扫一个空目录几乎不花时间） */
      const [repo, list, menu, local, localPresets, index, copied, marks, draft] = await Promise.all([
        api.getPresetFiles(),
        api.getMachines(),
        api.getMenu(),
        api.getLocalFiles(),
        api.getLocalPresetFiles(),
        api.getLocalPresetIndex(),
        api.getSlicerCopied(),
        api.getPresetRemarks(),
        api.getPresetDraftSummary(),
      ])
      setLocalPresets(localPresets)
      setLocalIndex(index)
      setRemarks(marks)
      setDraftPath(draft !== null && draft.dirty ? (draft.path ?? null) : null)

      /* 10 个组合 × 2 个读一起发。顺序无所谓，结果按 machine:version 对回去 */
      const combos = list.flatMap((m) => m.versions.map((v) => ({ machineId: m.id, versionId: v.id })))
      const inputs = await Promise.all(
        combos.map(async ({ machineId, versionId }): Promise<PresetVersionInput> => {
          const [files, params] = await Promise.all([
            api.getVersionFiles(machineId, versionId),
            api.getMachineParams(machineId, versionId),
          ])
          return {
            machineId,
            versionId,
            files,
            changed: params.filter((p) => p.origin === 'variant').length,
          }
        }),
      )

      if (!alive) return
      /* 唯一底账的快照（客户端的一次读取）—— 下面默认落地那台机型也要用它 */
      const entry = await activePresetSnapshot()
      setMachines(list)
      /* 仅归档的文件在这里就被剔掉 —— 用户端一处都不该出现 */
      setTreeSeed({ list, repo, inputs, archived: archivedIds(menu) })
      setLocalIds(local)
      setSlicerCopied(copied)

      /*
       * 默认落在**正在使用那台机型**：用户打开这一页最先想确认的是「我这台机器现在跑的是哪一套」。
       * 没有（或它指的机型 / 版本已经不在目录里）就退回第一台 —— 不硬留一个空选择。
       */
      const liveMachine = list.find((m) => m.id === entry?.machineId)
      const home = liveMachine ?? list[0]
      const homeVersion =
        (liveMachine === undefined
          ? undefined
          : liveMachine.versions.find((v) => v.id === entry?.versionId)) ?? home?.versions[0]
      if (home !== undefined && homeVersion !== undefined) {
        setAt({ machineId: home.id, versionId: homeVersion.id })
      }
      setReady(true)

      /*
       * 交付预设那一路**不挡首屏**。
       *
       * 它是两条独立的读（catalog 清单 + 下载区），失败时 delivery 落成空态、页面照常渲染；
       * 这一页要先看的「本机那份注册表」答出来的表，没有理由让整页等它。
       *
       * **先画本地 catalog（下面这次 readPresetFeed），再在后台检查 Bootstrap**（第十七刀）：
       * 顺序不能反 —— 用户一进预设页看到的必须是本地那份（离线也看得到），
       * 后台检查只是"路过时问一声远端有没有新版"，不问到就把页面挂住。
       */
      void readPresetFeedInto()
        .then(async () => {
          if (!alive) return
          /* 数据源与 delivery 一路同读：它只服务「来源」那一格的字，失败照实 null */
          const source = await api.getPresetSource().catch(() => null)
          if (alive) setSourceLabel(source?.mode ?? null)
          /*
           * 后台检查一次（本次运行只一次，见 `checkBootstrapOnce`）。
           * 有新版才把本地 catalog 换掉，**随后重读那一路**（目录换了，'ok / old / tampered'
           * 的分档要跟着以新目录为准）。它不返回脏数据：换的是盘上的 catalog，读的是同一套契约。
           */
          const boot = await checkBootstrapOnce()
          if (!alive) return
          /* 远端读不懂：立起那句提示（列表照常，不动 error、不动目录） */
          setNeedsNewerClient(boot.needsNewerClient)
          if (boot.changed) {
            /* 目录换过了，重读一遍（同一条出口，代次守卫管着先后） */
            await readPresetFeedInto().catch(() => {})
            /*
             * ★★ **身份账本也跟着重读**（2026-10-20）。
             *
             * 云端那一表每行的三态（未获取 / 本地已有 / 有新版）是
             * "本地文件 × 身份账本"算出来的，而账本**按当前目录**判
             * （血统指的那一版还在目录里才算数）。原来这里只重读"机型树 + 交付"，
             * 账本还拿着换版之前那一套 ⇒ 用户第一次切进云端页看到的是旧判定，
             * 直到别处碰巧重读一次才对上（作者图 7：「一开始的时候都没看到有更新按钮，
             * 第二次点开才有」）。
             *
             * 这一路只读本机（扫盘 + 账本），**不发网络** —— 那一趟网络在
             * `checkBootstrapOnce` 里已经付过了。
             */
            await readLocalPresets().catch(() => {})
          }
        })
        .catch(() => {
          /* 交付预设那一路读不到（目录/下载区任一失败）就落空态，不挡首屏 —— 与三态纪律一致：
             页面主体（注册表那几张表）已 ready，这里不升格成整页错误 */
        })
    }

    load().catch((e: unknown) => {
      if (!alive) return
      setError(errorText(e))
    })

    return () => {
      alive = false
    }
  }, [readPresetFeedInto, readLocalPresets, importRevision])

  /*
   * **跨页再同步**（2026-10-07 真机两刀挣来的）：
   *
   * 外壳的页签是**常驻 + 切显示**（`App.tsx`，2026-10-05 起），本页**不再重挂载**，
   * 只在挂载时读一次 —— 别的页面写进投递面的东西这里永远看不见
   * （首页「下载并应用」下了 X1 套餐，切回本页本地表却没有那两行）。
   *
   * 订阅的是**投递面代次**（`deliveryState`）：凡写投递面处，写完就广播，谁写的都算。
   * （第一版拿「使用中指针」当信号 —— 它只覆盖"下载并应用"这一类，覆盖不了
   * "只下载不应用"，所以这一刀换成直说。）
   * 本页自己的四个写出口（下载 / 批量 / 删本机 / 删归档）也只广播、不另行显式重读 ——
   * 单一触发路径。首屏那条链自己会读，这里从 ready 之后才开始听（首帧多读一次，接受）。
   */
  const deliveryRevision = useDeliveryRevision()
  useEffect(() => {
    if (!ready) return
    /* 走那条带守卫的出口（`readPresetFeedInto`）：这次重读与首屏 / 换目录后的重读
       并发时，晚发起的那次才算数 —— 旧读晚回来不许覆盖新快照 */
    void readPresetFeedInto().catch(() => {
      /* 读不到就保持旧数据 —— 与首屏同一条纪律：不把这一路升格成整页错误 */
    })
    /*
     * **本地文件也要重读**：投递面变了 = 云端那一份的字节可能刚被取回来（「使用」按需取回、
     * 「另存为本地预设」也会先取一份），而 `presets-local/` own 目录里也可能刚多出一份
     * —— 资源库要跟着出现那一行（只重读云端那一侧的话，用户会看不到自己刚存的那一份）。
     */
    void readLocalPresets().catch(() => {
      /* 读不到就保持旧列表 —— 与上面同一条纪律 */
    })
  }, [deliveryRevision, ready, readPresetFeedInto, readLocalPresets])

  /*
   * **设置页那一格偏好改过了**（2026-10-20）：「复制出来的也跟着云端更新」开 / 关，
   * 直接改变**身份账本**的出口（那一格是账本判"复制件认不认云端"的唯一依据），
   * 于是云端那一表每行的三态与那颗「更新」跟着变。
   *
   * 这里只重读**本地文件 + 身份账本**（都是本机扫盘，**不发网络**）—— 目录那一路
   * 没动过，为一句偏好去重读它是白跑一趟（更别说那一路里可能有网络）。
   */
  const presetPrefsRevision = usePresetPrefsRevision()
  useEffect(() => {
    if (presetPrefsRevision === 0) return
    void readLocalPresets().catch(() => {
      /* 读不到就保持旧列表 —— 与首屏同一条纪律 */
    })
  }, [presetPrefsRevision, readLocalPresets])

  /*
   * **草稿代次也听**（`draftState`）：参数页改了值 / 保存了 / 取消了（含预设页上
   * 那个「修改参数」模态框）、或正文编辑保存作废了参数草稿 —— 列表行上那枚
   * 「未保存」小圆点当场跟着亮 / 灭。只读一眼轻摘要，不重读别的。
   */
  const draftRevision = useDraftRevision()
  useEffect(() => {
    if (!ready) return
    void api
      .getPresetDraftSummary()
      /* `dirty` 这一道**必须**在（2026-10-19）：`begin_preset_edit` 一打开就写出草稿文件，
         "只打开过、一个字没改"也会有一份摘要 —— 不看不脏，那枚橙点会当场误亮。
         契约与头注的口径都是"**真改过才算**"（`PresetDraftSummary.dirty`）。 */
      .then((d) => setDraftPath(d !== null && d.dirty ? (d.path ?? null) : null))
      .catch(() => {
        /* 读不到就保持上一眼看到的 —— 一个标记不值得把列表带崩 */
      })
  }, [draftRevision, ready])

  /*
   * **换了数据源 —— 立刻按新地址重来一遍**（2026-10-08 作者要求）。
   *
   * 验收标准原话：「改完地址 → 保存 → 立即刷新数据源状态 → 下一次读取 / 检查就使用新地址」，
   * **不允许要求重启**。后端那侧本来就是每次现读（`resolve_source` 无进程内缓存），
   * 所以这里要补的只有两件前端的事：
   *
   *   ① 把「本次运行只检查一次」那道刹**复位**，允许后台**再检查一次**目录
   *      （不然用户改完地址，页面还停在旧源的目录与经济状态上）；
   *   ② 重读来源标签与交付那一路，并把新目录带来的变化照常应用 + 重读。
   *
   * 只在**真的换过**之后跑（代次 0 = 还没有人换过）。
   */
  const sourceRevision = useSourceRevision()
  useEffect(() => {
    if (sourceRevision === 0) return
    let alive = true
    checkedBootstrapThisRun = false
    void (async () => {
      const source = await api.getPresetSource().catch(() => null)
      if (!alive) return
      setSourceLabel(source?.mode ?? null)
      const boot = await checkBootstrapOnce().catch(() => null)
      if (!alive || boot === null) return
      setNeedsNewerClient(boot.needsNewerClient)
      /* 无论换没换目录都重读一遍：新源的自述与它登记的文件可能全都不同 */
      await readPresetFeedInto().catch(() => {})
    })()
    return () => {
      alive = false
    }
  }, [sourceRevision, readPresetFeedInto])

  /*
   * **对外只说"重读本地那一侧"**（2026-10-20）：切进「云端」那一页时要它刷新那三态，
   * 但那一侧不需要文件列表（它自己会进快照），所以这里把返回值收掉。
   */
  const refreshLocal = useCallback(async (): Promise<void> => {
    await readLocalPresets()
  }, [readLocalPresets])

  const pick = useCallback((machineId: string, versionId: string) => {
    setAt({ machineId, versionId })
  }, [])

  const pickMachine = useCallback(
    (next: string) => {
      const first = machines.find((m) => m.id === next)?.versions[0]
      setAt({ machineId: next, versionId: first?.id ?? '' })
    },
    [machines],
  )

  /*
   * 写。**写命令成功后 AppState 客户端重读整份并广播**（`appStateMutated`）——
   * 本页横幅与订阅了底账的另外三页同帧换账，中间不插一句前端自己的推断；
   * 界面上看到的必须是底账答的，不是前端猜的。
   *
   * 不 catch：失败要传到页面上说出来（没下载就应用、SHA 对不上这两种失败
   * 就是从这里冒上去的）。
   */
  const apply = useCallback(async (fileName: string, path: string) => {
    /* 用户线：**文件名与路径分开传**（后端认 origin=local + path 那道闸，见接口注释） */
    await api.applyActivePreset(fileName, 'local', path)
    appStateMutated()
  }, [])

  const clearApply = useCallback(async () => {
    await api.clearActivePreset()
    appStateMutated()
  }, [])

  /*
   * 切片器的「生效」= 复制进切片器自己的用户配置目录。**键是文件名**
   * （2026-10-09 起，与下载 / 应用 / 读正文同一个取用口径），真机落盘、假后端改内存。
   *
   * **目标已有同名就换掉它**（2026-10-13 作者改判：「有相同的就覆盖吧」）——
   * 后端原子替换，回来的 `replaced` 说这次是新放还是替换，提示条按它说话。
   */
  const copy = useCallback(async (fileName: string) => {
    const outcome = await api.copyToSlicer(fileName)
    setSlicerCopied(await api.getSlicerCopied())
    return outcome
  }, [])

  /*
   * 交付预设那一路的「下载」：走新世界下载管道，落进下载区 `mkp/`。
   * 不 catch：失败传给页面说出来，与 `apply` / `copy` 同一条规矩。（「应用」走上面那一个。）
   *
   * 写完**只广播**（`deliveryMutated()`）：本页与别的页都从订阅里重读那一路 ——
   * 这里是"写下 / 移出投递面"的四个出口之一，不再各自显式重读一次（单一触发路径）。
   */
  const downloadPreset = useCallback(
    async (fileName: string, onTick?: (tick: DownloadTick) => void) => {
      try {
        await api.downloadCatalogFile(fileName, onTick)
      } finally {
        /* 成没成都广播：`deliver` 是"先归档旧份、再换新"，失败也可能动过盘 ——
           宁可多读一次，不赌"失败 = 没变"（与首页 `applyCurrent` 同一条） */
        deliveryMutated()
      }
    },
    [],
  )

  /*
   * 云端表那条路（2026-10-09 改判）：下载 / 更新 = 取回交付预设 + 落一份我的本地文件。
   *
   * 它**不 catch**（失败传给页面说出来，与 `apply` / `copy` 同一条规矩）；
   * 成功之后**两边一起重读**：云端字节可能刚被取回来（投递面变了），
   * 本地文件可能也多了一份（`created`）—— 两个广播都要发。
   * **不广播 app-state**：这条命令一个应用状态都不碰（不设当前使用、不碰草稿）。
   */
  const fetchCloud = useCallback(
    async (fileName: string, renameIfTaken = false): Promise<FetchCloudResult> => {
      const done = await api.fetchCloudPreset(fileName, renameIfTaken)
      deliveryMutated()
      await readLocalPresets()
      return done
    },
    [readLocalPresets],
  )

  /*
   * **「编辑文件」那条链**：起手（读盘上那份文件的字节）、边改边存、放弃、写回它自己。
   *
   * 全是薄薄一层转发 —— 判定不在前端（哪一份能改、初稿是什么、草稿在哪，都由后端答）。
   * 只有两处多做一件事：
   *   · **写回之后**重读本地文件（本地表那一行的大小与时刻跟着变）；
   *   · 写回会把**参数页那份草稿作废**（盘上字节换了）→ 广播一声草稿代次，
   *     列表行上的「未保存」标记跟着收。
   */
  const beginTextEdit = useCallback(
    (fileName: string, path: string) => api.beginTextEdit(fileName, path),
    [],
  )

  const readLocalOriginText = useCallback((path: string) => api.readLocalOriginText(path), [])

  const putTextDraft = useCallback((text: string) => api.putTextDraft(text), [])

  const discardTextDraft = useCallback(() => api.discardTextDraft(), [])

  const commitTextDraft = useCallback(async () => {
    const done = await api.commitTextDraft()
    await readLocalPresets()
    draftMutated()
    return done
  }, [readLocalPresets])

  /*
   * **「云端更新」那条链**（2026-10-10）：读差异账 + 把用户的处理结果**一次性**写进本地文件。
   *
   * 两件薄转发 —— 判定（哪几项待处理、哪几项是冲突）与写入（结构保真地改那几项）都在后端。
   * 前端不拼 TOML、不猜冲突；「更新」那个流程（无冲突自动采用、冲突让用户选）见 `PagePresets`。
   */
  const presetSync = useCallback(
    (path: string, fetchMissing = false) => api.getPresetParamSync(path, fetchMissing),
    [],
  )

  const applyParamDecisions = useCallback(
    async (path: string, decisions: ParamDecision[]) => {
      /*
       * `decisions` 可以是空的（2026-10-20「应用 0 项」）—— 那一样是写：
       * 值一个不动，云端的注释 / 新字段 / 时间戳与血统推进（「有新版」就此收场）。
       */
      await api.applyPresetParamDecisions(path, decisions)
      await readLocalPresets()
      /* 盘上那份刚被改过（大小与时刻变了）—— 列表标记顺手对一眼 */
      draftMutated()
      /* 决定账（状态账）也动了 —— 云端更新那一段的读法跟着刷新 */
      appStateMutated()
    },
    [readLocalPresets],
  )

  /*
   * 第十层：用户文件管理。同一条路子 —— 写完 AppState 客户端重读广播：
   * 改名之后使用中指针可能跟着改了名（`repoint_local`），删的若是使用中那份，
   * 后端会把指针一并撤下 —— 都靠重读对表（曾在这里漏过的两个"删除后横幅
   * 还说正在使用"的滞留 bug，就是靠这一句消灭的）。
   *
   * ★★ 2026-10-14 补 `deliveryMutated()`：这两条**同时**是"本机文件面变了"
   * （改名换的是落点、删除直接少一份），只发 `appStateMutated()` 不够 —— 读文件面
   * 的地方（预设抽屉 `localPickerRows`、首页 / 校准页的 `usePreset`、BBS 交付面）
   * 收不到信号 ⇒ 抽屉还列着已经删掉的那一份。作者 2026-10-14 实测到这一条
   *（「很多事件都没有广播，导致那一边没有及时的实时的变化」）。
   * 同族的 `removePreset` / `removeArchived` 本来就发了，是这两条漏了。
   */
  const rename = useCallback(async (path: string, newName: string) => {
    const done = await api.renameLocalPreset(path, newName)
    await readLocalPresets()
    deliveryMutated()
    appStateMutated()
    return done
  }, [readLocalPresets])

  const remove = useCallback(async (path: string) => {
    await api.deleteLocalPreset(path)
    await readLocalPresets()
    deliveryMutated()
    appStateMutated()
  }, [readLocalPresets])

  /*
   * 删除本机那份交付预设文件（2026-10-06 一切皆可删）：重读云端那一侧 ——
   * 那一行回「未下载」，归档清单跟着刷新（`readPresetFeed` 连归档一起读）。
   * 若删的是使用中那份，后端会撤使用中指针 —— `appStateMutated` 让横幅同帧回「未应用」。
   */
  const removePreset = useCallback(async (fileName: string) => {
    await api.deleteDeliveryFile(fileName)
    /* 投递面广播（本页跟着重读，清单以盘为准）；删的是使用中那份时后端已撤指针，
       那条走 AppState 的广播（下面这句） */
    deliveryMutated()
    appStateMutated()
  }, [])

  /* 删除归档区一份旧版本（代价讲清在确认框）：投递面广播，清单以盘为准 */
  const removeArchived = useCallback(async (path: string) => {
    await api.deleteArchivedFile(path)
    deliveryMutated()
  }, [])

  /*
   * 另存为一份新的（第十一层）：只重读本地文件 —— 新的一份要出现在表里；使用中指针不归它管。
   *
   * ★★ 2026-10-14 补 `deliveryMutated()`：**盘上多了一份**，这就是"本机文件面变了"。
   *   原来只 `readLocalPresets()`（本页自己重读）⇒ 别的页收不到信号：
   *   校准页 / 首页的预设抽屉还列着复制之前的那几行（作者 2026-10-14 实测：
   *   「我复制出了一个新的结果，我在列表里面没看到」——预设页表里有、抽屉里没有）。
   *   与 `rename` / `remove` 同一个漏，上次只修了那两个，漏了这个。
   */
  const copyAsNew = useCallback(async (path: string, newName: string) => {
    const done = await api.copyLocalPreset(path, newName)
    await readLocalPresets()
    deliveryMutated()
    return done
  }, [readLocalPresets])

  /* 在文件管理器里显示（第十三层）：纯外部动作 —— 不重读、不改任何状态 */
  const reveal = useCallback(async (path: string) => {
    await api.revealInFolder(path)
  }, [])

  /* 同上，另两个落点（2026-10-13）：我们自己的下载区、切片器目录里那份 */
  const revealDelivered = useCallback(async (fileName: string) => {
    await api.revealDeliveredFile(fileName)
  }, [])

  const revealSlicer = useCallback(async (fileName: string) => {
    await api.revealSlicerFile(fileName)
  }, [])

  /*
   * **改一份预设的备注**（2026-10-07 副标题覆盖账；只有本地表读它）：写完重读整本账 ——
   * 副标题是账答的，不是前端改自己那份拷贝。写什么就是什么（空也存）；
   * `null` = 恢复默认（删掉覆盖，退回落入「工作台写的 → 空着」）。
   *
   * ★ 2026-10-14 补 `deliveryMutated()`：**副标题就是本地表那一列**，
   *   别的页（BBS 交付面读同一本账）也该跟着刷。
   */
  const setRemark = useCallback(async (key: string, remark: string | null) => {
    await api.setPresetRemark(key, remark)
    setRemarks(await api.getPresetRemarks())
    deliveryMutated()
  }, [])

  /*
   * **改一份用户预设的归属**（机型 / 版本）：写完重读本地文件 ——
   * 归属是文件头里那两行，重读之后列表的机型 / 版本两列以文件为准。
   * 机型必须目录里登记的；版本可自定义（不认识的照原文显示）。
   *
   * ★★ 2026-10-14 补 `deliveryMutated()`：改的是**文件头那两行**（真写盘），
   *   而归属正是预设抽屉**分组的依据**（`localPickerRows` 按 `basedOn*`/`machineId` 摊组）
   *   ⇒ 不发这条，改完归属切到校准页，抽屉里那份还挂在旧的机型组下。
   */
  const setLocalMachineVersion = useCallback(
    async (path: string, machineId: string, versionId: string) => {
      await api.setLocalPresetMachineVersion(path, machineId, versionId)
      await readLocalPresets()
      deliveryMutated()
    },
    [readLocalPresets],
  )

  /*
   * 批量：一次把多份交给后端，回来后**不管成没成先广播**（成功的那些已经落盘了），
   * 再把逐份结局原样交回页面。顺序 = 请求顺序（后端保证），页面按它列。
   */
  const downloadPresetBatch = useCallback(
    async (fileNames: string[], onTick?: (tick: DownloadTick) => void) => {
      const outcomes = await api.downloadCatalogFiles(fileNames, onTick)
      deliveryMutated()
      return outcomes
    },
    [],
  )

  const machineId = at?.machineId ?? ''
  const versionId = at?.versionId ?? ''
  /* 版本显示名/说明按语言摊开（id、套餐、图位不动）；树那边同一份摊法 */
  const machinesShown = useMemo(() => localizeMachines(machines, locale), [machines, locale])

  return {
    loading: error === null && !ready,
    error,
    needsNewerClient,
    machines: machinesShown,
    machineId,
    versionId,
    machine: machinesShown.find((m) => m.id === machineId),
    sourceLabel,
    tree,
    localIds,
    local: localPresets,
    localIndex,
    draftPath,
    active,
    slicerCopied,
    refreshLocal,
    pick,
    pickMachine,
    apply,
    fetchCloud,
    clearApply,
    copy,
    delivery,
    archived,
    cloudVersions,
    downloadPreset,
    downloadPresetBatch,
    beginTextEdit,
    readLocalOriginText,
    putTextDraft,
    discardTextDraft,
    commitTextDraft,
    presetSync,
    applyParamDecisions,
    rename,
    remove,
    removePreset,
    removeArchived,
    copyAsNew,
    reveal,
    revealDelivered,
    revealSlicer,
    /** 备注覆盖账（副标题）与它的写入口；归属的写入口见 `setLocalMachineVersion` */
    remarks,
    setRemark,
    setLocalMachineVersion,
  }
}


// ——————————————————————————————————————————————————————————————
// 置顶
// ——————————————————————————————————————————————————————————————

/**
 * 置顶的 localStorage 键。**页面的私有偏好**，所以收在 `src/api/storageKeys.ts`
 * （作者还没定这套名字，集中一处，将来改名是一处的事），值沿用原来的、一个字不改。
 *
 * 存的是一串 `pinKey`（asset id，用户自己的文件是它的本机 id），**JSON 编码** ——
 * 这个仓库里所有落 localStorage 的值都走 `JSON.stringify`，一处写裸字符串，
 * 以后想把它读成结构化数据就得先猜编码。
 *
 * # ★★ 是一个**有序数组**，不是一个集合（2026-10-16 作者：「后面点的置顶的应该置顶，
 *   多个置顶的按照先后顺序，后点击的置顶」）
 *
 * 顺序的含义 = **置顶的先后**：`[0]` 是**最早**置顶的那份，末尾是**最近**一次点的。
 * 排序时按这个下标**倒过来**读 ⇒ 最近置顶的排最前（见 `presetTree.sortRows`）。
 *
 * ★ 为什么存"先→后"、显示时倒过来，而不是存"后→前"：
 *   这一键从第一版起就存的是一串 pinKey，那时是个 `Set`，**迭代顺序 = 插入顺序**
 *   （先点的在前）。语义改成"后点的在前"之后，**存法不变、只是读的时候倒过来**，
 *   老用户盘上那份数据一步就对，不需要迁移、不需要给存储加版本号。
 *   （反过来做 —— 存的时候 unshift —— 会让老数据的顺序整个反过来。）
 */
const PINNED_KEY = STORAGE.clientPresetsPinned

/**
 * 置顶是**纯前端的排序**，所以它刷新之后保留是对的：它记的是「我常用哪几个文件」，
 * 不是一份未保存的改动。读坏了就退化成空 —— 一个排序偏好不值得让页面起不来。
 */
function readPinned(): string[] {
  try {
    const raw = window.localStorage.getItem(PINNED_KEY)
    if (raw === null) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed.filter((v): v is string => typeof v === 'string')
  } catch {
    /* 隐私模式会抛，手改坏了 JSON 也会抛。两种都退化成「这次没有置顶」 */
    return []
  }
}

function writePinned(next: string[]): void {
  try {
    window.localStorage.setItem(PINNED_KEY, JSON.stringify(next))
  } catch {
    /* 写不进去就只在这次会话里有效，不影响别的功能 */
  }
}

// ——————————————————————————————————————————————————————————————
// 这一屏要显示什么
// ——————————————————————————————————————————————————————————————

export interface PresetPage {
  /**
   * 机型级「暂不支持」：**这台机型下所有版本后端都没配资源**（实测只有 A2L）。两张表都不画。
   *
   * 判据从「当前这一个版本没配」改成了机型级 —— 版本选择器没了，按版本说话就没有主语了。
   * 「全部机型」那一档永远是 false：那一档下至少有别的机型的文件。
   */
  unsupported: boolean
  /** 没配齐的原因，后端原文（这台机型各版本 missing 的并集） */
  missing: string[]

  /** 左边那条分段控件：文件类型 */
  kind: PresetKindAxis
  setKind: (next: PresetKindAxis) => void
  /**
   * 右边那条分段控件：位置。**两种类型都有它**（2026-10-09 起）—— 那两张表互不相干、
   * 各答各的问题：
   *
   *   本地  我这台机器上真有的东西（MKP 档 = 你自己那份本地文件；切片器档 = 本机真有的文件）
   *   云端  云端现在提供什么（MKP 档三态：未获取 / 本地已有 / 有新版；切片器档标「已下载 / 未下载」）
   */
  scope: PresetScopeAxis
  setScope: (next: PresetScopeAxis) => void

  /**
   * 「本地」这一张表 —— **我电脑里真有的东西**。
   *
   * MKP 档 = 你自己那份本地文件（`presets-local/`，只有它）；切片器档 = 本机磁盘上
   * 真有的文件（云端副本 + 本地文件）。两种类型在这一档的分法见 [`localRows`]。
   */
  local: PresetTableData<PresetLocalRow>
  /**
   * 「云端」这一张表 —— **云端现在提供什么**。
   *
   * MKP 档的行按 [`CloudLocalState`] 说"我这边怎么样"（未获取 / 本地已有 / 有新版）；
   * 切片器档照旧按 `presetState` 说"下过没有 / 要不要重下"。见 [`cloudRows`]。
   */
  cloud: PresetTableData<PresetCloudRow>

  /**
   * 批量要处理的那一批：**未下载 + 旧版本 + 内容异常**，已下载的不进来（不重复下）。
   *
   * 判定就是云端表行上那个 `presetState`（唯一判据），这一层没长第二套状态判断；
   * 范围取**筛选前**的云端行 —— 按机型与类型（表的口径），**不受搜索词影响**。
   * 三档分开数：前两档是"补上云端那份"，第三档是"盘上那份我们不认"，用户要做的事不一样。
   */
  pending: {
    fileNames: string[]
    /** 其中「未下载」几份 */
    missing: number
    /** 其中「旧版本」几份（认得出是云端某一版旧版） */
    stale: number
    /** 其中「内容异常」几份（这台机器上查不出它属于哪一版） */
    tampered: number
    total: number
  }

  /**
   * 置顶的**先后顺序**（一串 `pinKey`，`[0]` 是最早置顶的、末尾是最近一次点的）。
   * 落 localStorage，纯前端排序。★ 不是集合 —— 顺序是有意义的，见 `PINNED_KEY` 那段。
   */
  pinned: string[]
  togglePin: (pinKey: string) => void

  /**
   * 表头那一列怎么排（2026-10-16 作者：每列都能点）。落会话 —— 与搜索词同一条理由
   * （切页对一眼回来不该复位）。★ 置顶**不归它管**：置顶永远压在最上面。
   */
  sort: PresetSort
  /**
   * 点表头。**名称列是三循环**（默认 → a-z → 倒序 → 默认），别的列在升 / 降之间换；
   * 换列则**从升序重新起**（★ 时间那一列除外 —— 先看最新的更符合直觉，
   * 所以换到时间列时默认**降序**）。
   */
  toggleSort: (key: PresetSortKey) => void

  /**
   * 正在使用的**唯一那一条**（新世界底账 `run/active-preset.json`）。`null` = 一套都还没应用（**不是错误**）。
   *
   * 顶部状态条挂在它身上，**不跟着机型下拉走** —— 它是全局唯一的一条事实。
   */
  applied: ActivePreset | null
  /** 已应用那台机型的显示名。查不到就退回 id，不留空 */
  appliedMachineText: string
  /** 已应用那个文件的文件名（云端行查树、preset 行查发布清单）。都查不到退回 ref */
  appliedFileName: string
  /**
   * 正在使用的这一份**是用户自己的那份**（不是交付预设的）。
   *
   * 状态条据此多写一小句说明 —— 两边都能成为使用中的那一份（第七层），
   * 用户得看得出"现在跑的是我改的那一份"。
   */
  appliedIsLocal: boolean

  query: string
  setQuery: (next: string) => void
  searching: boolean

  /**
   * 切片器的喷嘴 / 层高筛选。MKP 没有这两个字段 ——
   * MKP 那一排的位置由「已应用状态条」占，这里的状态只在切片器表上生效。
   */
  slicerFilters: {
    /** 可选值：从**没筛过**的当前表里取（筛过的行会让选项一个个消失），数值升序 */
    nozzles: string[]
    /**
     * 层高**全集**（所有喷嘴的并集，数值升序）。
     *
     * ★ 2026-10-10 作者改判：选了喷嘴之后，层高那一组**不是把别的收走**，
     *   而是**全都在、不支持的那些灰着点不动**（「我希望我选了它，筛选出来的不是
     *   把别的消失……别的变成那种灰色不让点击的」）。所以面板画的是这一份全集，
     *   能不能点看 `layerOptions`。
     */
    allLayers: string[]
    /**
     * 层高**可用集** —— 跟着喷嘴收敛（选了 0.2 就只有 0.06–0.14）。
     *
     * 它管两件事：面板里哪些胶囊可点（其余灰掉），以及当前选中的层高在新喷嘴下
     * 还成不成立（不成则自动回「全部」，见 `layerValue`）。页面不重算，只管画。
     */
    layerOptions: string[]
    /** 当前生效的值。空串 = 「全部」；选中的值不在可选集里（换了机型 / 表）时自动视为全部 */
    nozzle: string
    layer: string
    setNozzle: (next: string) => void
    setLayer: (next: string) => void
  }
}

export function usePresetPage(data: PresetData): PresetPage {
  /* 两张表要措辞（行上的「下载区 mkp · …」那种），所以这一层也要取当前语言 */
  const tr = useT()
  /*
   * 搜索词与两条分段控件都记在会话里：切到参数页对一眼再回来，不该复位成「MKP / 本地」
   * 并把搜索词清空（见 useSessionState 的理由）。默认落在 MKP + 本地 —— 用户打开这一页
   * 最常问的是「我这台机器上现在有什么」。
   */
  const [query, setQuery] = useSessionState('presets.query', '')
  const [kind, setKind] = useSessionState<PresetKindAxis>('presets.kind', 'mkp')
  const [scope, setScope] = useSessionState<PresetScopeAxis>('presets.scope', 'local')
  /* 切片器的喷嘴 / 层高筛选。空串 = 「全部」。落会话：切页对一眼回来不该复位 */
  const [nozzle, setNozzle] = useSessionState('presets.nozzle', '')
  const [layer, setLayer] = useSessionState('presets.layer', '')

  const [pinned, setPinned] = useState<string[]>(readPinned)

  /*
   * **切进「云端」那一页时，把那三态的账对一眼**（2026-10-20 作者要求"两条都兜住"）。
   *
   * 上面那条（目录换版后重读）收拾的是"懒加载那一趟换过目录"；这一条收拾的是
   * **用户切进来的时机**：账本与本地文件都可能在那之后又被别处动过（别页另存了、
   * 参数页落了决定），而首屏那一条链只在页面第一次挂载时跑。
   *
   * 只读**本地文件 + 身份账本**（`readLocalPresets`）—— 都是本机扫盘，**不发网络**：
   * 用户切页签不该顺手发一次网络请求（"启动零网络"那条纪律的另一半）。
   */
  /* 先取出来再进依赖数组（直接写 `data.refreshLocal` 会让那条规则认不出它是稳定的） */
  const refreshLocal = data.refreshLocal
  useEffect(() => {
    if (scope !== 'cloud') return
    void refreshLocal().catch(() => {
      /* 读不到就保持旧列表 —— 与首屏同一条纪律 */
    })
  }, [scope, refreshLocal])

  /*
   * 表头排序（2026-10-16）。
   *
   * ★ 2026-10-20 名称列改**三循环**（作者：「名称那个应该是三循环的点击切换排序，
   *   就是默认，a-z，倒序」）：默认 = 族优先（`family`，复制出来的躺在原件下面），
   *   点一下 = a-z，再点 = 倒序，第三下回到默认。别的列仍是两态（升 / 降）。
   * ★ 换到「时间」那一列时默认**降序**（最新的在最上），其余从升序起 ——
   *   按时间asc 看到的是"最老的在前"，没人是要这个。
   */
  const [sort, setSort] = useSessionState<PresetSort>('presets.sort', {
    key: 'name',
    dir: 'family',
  })
  /* ★ `useSessionState` 的 setter 只收值（不收 updater），所以这里读当前 `sort` 现算 */
  const toggleSort = useCallback(
    (key: PresetSortKey) => {
      const next = ((): PresetSortDir => {
        if (sort.key !== key) return key === 'time' ? 'desc' : 'asc'
        /* 同一列再点：名称列走三循环，别的列只在升 / 降之间换 */
        if (key !== 'name') return sort.dir === 'asc' ? 'desc' : 'asc'
        if (sort.dir === 'family') return 'asc'
        return sort.dir === 'asc' ? 'desc' : 'family'
      })()
      setSort({ key, dir: next })
    },
    [sort, setSort],
  )

  const togglePin = useCallback((pinKey: string) => {
    setPinned((now) => {
      /* 新建一个数组而不是改原来那个：React 靠引用变化才重画这张表 */
      const next = now.includes(pinKey)
        ? /* 取消置顶：删掉它，其余的相对顺序不动 */
          now.filter((k) => k !== pinKey)
        : /* 置顶：**追加到末尾** —— 末尾 = 最近一次点的（显示时倒过来读，它就在最前） */
          [...now, pinKey]
      writePinned(next)
      return next
    })
  }, [])

  const localSet = useMemo(() => new Set(data.localIds), [data.localIds])
  /* 已复制到切片器目录的那一份集合。和 localSet 是两件事，别合并 */
  const copiedSet = useMemo(() => new Set(data.slicerCopied), [data.slicerCopied])

  /* 要汇总哪几台机型：一台，或者「全部机型」那一档的全部 */
  const machines = useMemo(
    () => machinesInScope(data.tree, data.machineId),
    [data.machineId, data.tree],
  )

  /* 两张表的输入是同一份 —— 拼一次，两个纯函数各取所需 */
  const input = useMemo(
    (): PresetRowsInput => ({
      machines,
      machineId: data.machineId,
      local: data.local,
      localIndex: data.localIndex,
      draftPath: data.draftPath,
      localIds: localSet,
      slicerCopiedIds: copiedSet,
      active: data.active,
      kind,
      query,
      pinned,
      sort,
      deliveryPresets: data.delivery.presets,
      cloudVersions: data.cloudVersions,
      localPresets: data.delivery.localPresets,
      stalePresets: data.delivery.stale,
      presetVersion: data.delivery.version,
      releaseAt: data.delivery.publishedAt,
      sourceLabel: data.sourceLabel,
      remarks: data.remarks,
    }),
    /* `data` 一个就够（`data.xxx` 都在它下面 —— 单列出来 eslint 会报"多余"） */
    [data, copiedSet, kind, localSet, machines, pinned, query, sort],
  )

  /*
   * **两张表，两种类型都算**（2026-10-09 起；MKP 档此前短暂合成过一张资源库表）。
   * 各自的判据在 [`localRows`] / [`cloudRows`] 里，这一层只负责算出来。
   */
  const localBase = useMemo(() => localRows(tr, input), [tr, input])
  const cloudBase: PresetTableData<PresetCloudRow> = useMemo(
    () => cloudRows(tr, input),
    [tr, input],
  )

  /*
   * 批量那一批。取**云端表筛前**的行，两个理由：
   *   ① 云端 = 目录登记的全集，本地只是它的子集（同一份文件在两张表里都有）——
   *      拿一边数就够了，合起来数会重复；
   *   ② **不受搜索词影响**：搜索是"我在找什么"，不该悄悄改变"按一下要动几份"。
   * 筛选（机型 / 类型）已经在 `cloudRows` 里做过了，所以这里只按状态挑。
   *
   * **两种类型两套状态词**（同一件事的两种口径，各自都是源上算好的）：
   *
   *   MKP     看 `localState`：未获取（要"下载"）/ 有新版（要"更新"）
   *   切片器   看 `presetState`：未下载 / 旧版本 / 内容异常（三档都是"要处理"）
   */
  const pending = useMemo(() => {
    const rows = cloudBase.rows.filter((r) =>
      r.localState !== undefined
        ? r.localState === 'none' || r.localState === 'stale'
        : r.origin === 'preset' && r.presetState !== undefined && r.presetState !== 'ok',
    )
    return {
      fileNames: rows.map((r) => r.fileName),
      missing: rows.filter((r) =>
        r.localState !== undefined ? r.localState === 'none' : r.presetState === 'missing',
      ).length,
      stale: rows.filter((r) =>
        r.localState !== undefined ? r.localState === 'stale' : r.presetState === 'old',
      ).length,
      tampered: rows.filter((r) => r.presetState === 'tampered').length,
      total: rows.length,
    }
  }, [cloudBase.rows])

  /*
   * 切片器的喷嘴 / 层高：可选项从**没筛过**的当前表里取（筛过的行会让选项一个个消失），
   * 然后再把筛选应用到行上出最终的表。MKP 没有这两个字段 —— 值集恒为空，筛选恒为「全部」。
   * 选中的值不在可选集里（换了机型 / 表）时自动视为「全部」，不然会筛出一张永远空着的表。
   */
  const slicerValues = useMemo(
    () =>
      slicerFilterValues(
        kind === 'slicer' ? (scope === 'local' ? localBase.rows : cloudBase.rows) : [],
      ),
    [kind, scope, localBase, cloudBase],
  )
  const nozzleValue = kind === 'slicer' && slicerValues.nozzles.includes(nozzle) ? nozzle : ''
  /*
   * 层高的可选值**跟着喷嘴走**（2026-10-10）：选了喷嘴就只列这个喷嘴的层高；
   * 喷嘴那一档是「全部」时列所有喷嘴的并集。切喷嘴时当前层高不在新集合里就自动
   * 回「全部」—— 不这么做的话，0.2 喷嘴下留着 0.30 会筛出一张永远空着的表。
   */
  const layerOptions =
    nozzleValue === ''
      ? slicerValues.layers
      : (slicerValues.layersByNozzle[nozzleValue] ?? [])
  const layerValue = kind === 'slicer' && layerOptions.includes(layer) ? layer : ''

  const local = useMemo(
    () => ({ ...localBase, rows: applySlicerFilters(localBase.rows, nozzleValue, layerValue) }),
    [localBase, nozzleValue, layerValue],
  )
  const cloud = useMemo(
    () => ({ ...cloudBase, rows: applySlicerFilters(cloudBase.rows, nozzleValue, layerValue) }),
    [cloudBase, nozzleValue, layerValue],
  )

  /* 机型级「暂不支持」：这台机型一个文件都没有、而且每个版本都没配齐 */
  const machineAt = useMemo(
    () => machineNode(data.tree, data.machineId),
    [data.machineId, data.tree],
  )

  /*
   * 顶部状态条那句话全按**正在使用的那一条**取，不按当前筛的机型。
   * 文件名两个来源分别查：preset 行在发布清单里、云端行在树里 —— 都查不到退回 ref 本身。
   */
  const appliedMachine = data.machines.find((m) => m.id === data.active?.machineId)
  const appliedFileName = useMemo((): string => {
    const entry = data.active
    if (entry === null) return ''
    /* 底账给的就是文件名；本地表 / 云端表都按它对（交付预设行与目录登记同名） */
    const hit = data.delivery.presets.find((p) => p.fileName === entry.fileName)
    return hit?.fileName ?? entry.fileName
  }, [data.active, data.delivery.presets])

  return {
    unsupported: machineAt !== undefined && machineAt.unavailable,
    missing: machineAt?.missing ?? [],
    kind,
    setKind,
    scope,
    setScope,
    local,
    cloud,
    pending,
    pinned,
    togglePin,
    sort,
    toggleSort,
    applied: data.active,
    appliedMachineText: appliedMachine?.display ?? data.active?.machineId ?? '',
    appliedFileName,
    /* 使用中的永远是**本地文件**（即「本地文件」）—— 有底账就是它 */
    appliedIsLocal: data.active !== null,
    query,
    setQuery,
    searching: query.trim() !== '',
    slicerFilters: {
      nozzles: slicerValues.nozzles,
      allLayers: slicerValues.layers,
      layerOptions,
      nozzle: nozzleValue,
      layer: layerValue,
      setNozzle,
      setLayer,
    },
  }
}

