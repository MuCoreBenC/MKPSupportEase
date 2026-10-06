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
 *   用户自己的文件      api.getUserPresetFiles()                  → **用户线**（扫 presets-mine/），空是合法状态
 *   当前使用的那一条    api.getActivePreset()（新世界底账 `run/active-preset.json`）→ **全局唯一**，null = 一套都还没应用
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

import { useCallback, useEffect, useMemo, useState } from 'react'
import { api, errorText } from '../../api'
import type {
  ActiveOrigin,
  ActivePreset,
  ArchivedFile,
  CommittedDraft,
  DownloadOutcome,
  DownloadTick,
  Machine,
  PresetDraft,
  UserFileIdentity,
} from '../../api'
import { isAppError } from '../../api/contract'
import { STORAGE } from '../../api/storageKeys'
import { useSessionState } from '../shared/useSessionState'
import type { ReleasePresetSource } from './presetTree'
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
  PresetTableData,
  PresetTree,
  ReleaseFileState,
  UserPresetFile,
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
   *     本机什么都没坏、也没有任何东西失败，只是"你想给我的这份我读不了"。
   *
   * 用户要做的**唯一一件事**是去设置页更新客户端（不是"重试"、不是"再下一遍"）。
   * 因此它**不改目录、不落页级错误**（作者定的产品规则 B：列表照常，只在预设页出现一句提示）。
   */
  needsNewerClient: boolean

  /**
   * 官方交付：目录（catalog）登记的交付预设全量 + 下载区（`mkp/`）现况。
   *
   * 两份底账都住 Internal 根：清单在 `catalog.json` 的 files 域（发布方写），
   * 「已下载」在 `mkp/`（盘就是底账）。这一层只读不写；写走下面的 `downloadRelease` / `apply`。
   */
  release: ReleaseState

  /**
   * 归档区里躺着的**官方旧版本**（换版本时被换下来的那一份）。
   *
   * 与交付那一路同源：跟 `release` 一起读、同样**不挡首屏**，每次下载 / 更新之后跟着重读
   * （刚更新过的那一份会出现在这里）。**只读** —— 界面不删、不恢复、也不拿它当"用户修改历史"：
   * 归档是官方版本生命周期的一部分（见 `ArchivedFile` 的注释）。
   */
  archived: ArchivedFile[]

  machines: Machine[]
  /** 当前机型。**空串 = 「全部机型」那一档**（预设页才用得到，首页永远是一台具体的） */
  machineId: string
  versionId: string
  machine: Machine | undefined
  /** 未过滤的整棵树（仅归档的文件建树时就剔掉了）。计数用它 */
  tree: PresetTree
  /**
   * 本机已有哪些文件（asset id）。
   * 假后端给的是**固定演示集合**（`src/api/mockServer` 的 `localFiles.ts`）。
   */
  localIds: string[]
  /**
   * **用户线**：用户自己放的那一份（`presets-mine/` 里扫出来的）。
   *
   * **云端没有它们**，所以它们只出现在本地表里、没有交付身份、也不参与套餐与菜单。
   * 认不出类别的那几份（`.json`）也在里面 —— 界面上任何类型档下都列，不藏。
   */
  mine: UserPresetFile[]
  /**
   * 正在使用的**唯一那一条**：从底账（`run/active-preset.json`）读出来的，
   * **全表最多一份**，`null` = 一套都还没应用（**不是错误**）。
   *
   * 官方交付行应用后写着的都是它 —— 这是「使用中」的唯一来源
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
  pick: (machineId: string, versionId: string) => void
  /**
   * 只换机型（预设页那个下拉）。版本跟着落到这台机型的第一个 —— 版本 id 在机型之间
   * 不通用（LITE 只有 P1S / X1C 有），留着上一台的 id 会指到一个不存在的组合。
   * 传空串 = 「全部机型」，那一档没有「当前版本」这回事。
   */
  pickMachine: (machineId: string) => void
  /**
   * 把某一份设为**当前使用的那一条**。**两条线共用这一个入口**（第七层）：
   *
   *   官方交付文件  `apply(fileName)`
   *   用户自己那份  `apply(fileName, 'mine', path)`（用户目录里可以分文件夹，所以认路径）
   *
   * 走新世界的应用命令（`api.applyActivePreset`，Rust 侧按各自的闸校验后写
   * `run/active-preset.json`）**然后重读底账** —— 界面看到的永远是底账答的，
   * 与 `copy` 同一条规矩。失败照抛给调用方（页面用提示条说出来），**不在这里吞**。
   */
  apply: (fileName: string, origin?: ActiveOrigin, path?: string) => Promise<void>
  /**
   * **撤销应用** —— 把"当前使用的那一条"撤掉（`api.clearActivePreset()`），然后重读底账。
   *
   * 与 `apply` 同一条规矩（写底账 → 重读底账）；幂等：本来就没在应用也不报错
   * （后端 `clear_active` 对"文件不在"就是成功）。**它不碰任何文件** ——
   * 撤掉的是"哪一份在生效"这个指向，磁盘上的预设一个字节不动。
   */
  clearApply: () => Promise<void>
  /** 把某个切片器 profile 复制进切片器目录，然后重新拉 `getSlicerCopied()`。同上 */
  copy: (assetId: string) => Promise<void>
  /**
   * 「下载」目录里登记的那一份：走新世界下载管道（`api.downloadCatalogFile(fileName)`），
   * 落进下载区 `mkp/`。盘上那一份对不上目录时（需更新）走的是**同一条路** ——
   * 再下一遍，旧份自动归档，没有单独的"更新"代码路径。
   *
   * `onTick` 给了就把过程说出来（一次调用一路水位，后端在下载过程中推）；不给就是
   * 原来那个"点了等结果"。**下载快慢由网络决定，界面上说真的字节数，不编进度条。**
   *
   * 与官方文件那颗「下载」（`api.downloadFiles()`，假后端必抛未实现）不同，
   * 这一条**真的能下**。失败照抛给页面说出来。
   */
  downloadRelease: (fileName: string, onTick?: (tick: DownloadTick) => void) => Promise<void>
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
  downloadReleaseBatch: (
    fileNames: string[],
    onTick?: (tick: DownloadTick) => void,
  ) => Promise<DownloadOutcome[]>

  /**
   * **开始改一份**预设：把正文复制进临时文件（原件一动不动）。
   * 两条线一个入口：官方线交文件名，用户线还要交路径（用户目录里可以自己分文件夹）。
   * `reused` = 接着上次那半截改。
   */
  beginEdit: (
    fileName: string,
    origin?: ActiveOrigin,
    path?: string,
  ) => Promise<PresetDraft>
  /** 把改动写进临时文件（界面边改边存 —— 改到一半关掉也还在） */
  putDraft: (text: string) => Promise<void>
  /** 放弃这次编辑（丢草稿；原件与下载区全程没被碰过，所以它天生安全） */
  discardDraft: () => Promise<void>
  /**
   * 存进用户根：官方线**另存**成一份新的、用户线**写回它自己**（第八层）。
   * 存完**重读用户线** —— 官方线那一份要跟着出现在本地表里，用户线要跟着变时刻与大小。
   */
  commitDraft: () => Promise<CommittedDraft>
  /**
   * **重命名一份用户文件**（第十层）：只改名字，字节一个不动。回来**重读用户线**
   * （列表立刻以磁盘为准），顺手重读使用中指针（它正指着这一份时会跟着改）。
   */
  rename: (path: string, newName: string) => Promise<UserFileIdentity>
  /**
   * **另存为一份新的**（第十一层）：把我自己那一份按字节复制成同一格里另一份新的用户文件。
   * 回来**重读用户线**（新文件要出现在表里）；**不重读使用中指针** —— 这一层不碰它
   * （原文件一个字节不动，复制出来的那份也不会自称"使用中"）。
   */
  copyAsNew: (path: string, newName: string) => Promise<UserFileIdentity>
  /**
   * **删除一份用户文件**（第十层）：**真删除**（没有垃圾桶、没有归档）。回来重读用户线。
   * 两道闸（正在使用的 / 还有没保存的草稿的）在后端 —— 失败照抛给页面说出来，不在这里吞。
   */
  remove: (path: string) => Promise<void>
  /**
   * **在文件管理器里显示**（第十三层）：打开 Finder / 资源管理器并选中这份用户文件。
   * **不重读任何东西** —— 它一个状态都不改（打开的是系统窗口，不是我们的界面）。
   */
  reveal: (path: string) => Promise<void>
}

  /**
   * 「目录里登记的交付预设」这一路的现况。清单一份（catalog）、下载区一份 —— 两件事分开。
   * （「正在用的」不在这里：它只有一个出处 `run/active-preset.json`，见 `PresetData.active`。）
   */
export interface ReleaseState {
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
  presets: ReleasePresetSource[]
  /** 下载区（`mkp/`）里已有、**且与目录登记一致**的（`ReleaseFileState = ok`） */
  localReleases: ReleasePresetSource[]
  /**
   * 下载区里有、**但与目录登记不一致**的（`ReleaseFileState = old / tampered`，
   * 即「旧版本 / 内容异常」）。
   *
   * 与 `localReleases` 是**两个读**：`getDownloadedFiles()` 只答"盘上有没有且对不对"，
   * `getStaleFiles()` 才答"盘上有一份但不能用"。少了这一个读，「旧版本 / 内容异常」就会
   * 被显示成「未下载」—— 用户点"下载"以为是第一次下，实际是在修一份坏档。
   *
   * 不一致的那几份**是哪一档**由第三个读（`getDeliveryTrust`）定：认得出是官方某一版旧版的
   * 是 `old`，哪儿都查不出的是 `tampered`（第三圈第 6 层）。
   */
  stale: ReleasePresetSource[]
  /** 已下载的 uid 集合 */
  localUids: string[]
}

const EMPTY_RELEASE: ReleaseState = {
  version: null,
  publishedAt: null,
  presets: [],
  localReleases: [],
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

/**
 * `importRevision` 是"外部导入进来过几批"的钥匙（第十二层）：变了就**整屏重读** ——
 * 导入落进 `presets-mine/` 之后，「我的文件」那张表要立刻以磁盘为准，不靠切页刷新。
 * 首页（`PageHome`）不给这个参数：它不看用户线那张表。
 */
export function usePresetData(importRevision = 0): PresetData {
  const [machines, setMachines] = useState<Machine[]>([])
  const [tree, setTree] = useState<PresetTree>({
    machines: [],
    fileCounts: { mkp_preset: 0, bbs_profile: 0, orca_profile: 0 },
  })
  const [localIds, setLocalIds] = useState<string[]>([])
  /* 远端这一代读不懂（NOT_SUPPORTED）：独立于 `error`，不挡列表、只开一句提示 */
  const [needsNewerClient, setNeedsNewerClient] = useState(false)
  /* 用户线：用户自己的预设（`presets-mine/`）。盘当底账 —— 首屏读一次；产生它的动作在下一层 */
  const [mine, setMine] = useState<UserPresetFile[]>([])
  const [active, setActive] = useState<ActivePreset | null>(null)
  const [slicerCopied, setSlicerCopied] = useState<string[]>([])
  const [ready, setReady] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [at, setAt] = useState<{ machineId: string; versionId: string } | null>(null)
  const [release, setRelease] = useState<ReleaseState>(EMPTY_RELEASE)
  /* 归档区（官方旧版本留档）。与 release 一起读、一起刷新（见 `readRelease`） */
  const [archived, setArchived] = useState<ArchivedFile[]>([])

  /**
   * 把官方交付这一路的现况读一遍。
   *
   * 全部来自新世界的底账：清单读 `api.getRuntimeCatalog()` 的 files 域，
   * 「盘上有没有、对不对」读 `api.getDownloadedFiles()` / `api.getStaleFiles()`
   * （`mkp/`，盘就是底账），「盘上那份认得出是哪一版吗」读 `api.getDeliveryTrust()`
   * （第三圈第 6 层），旧版本留档读 `api.getArchivedFiles()`。
   *
   * 下载 / 应用之后**重读一遍**而不是本地改状态 —— 与 `copy` 同一条规矩：
   * 界面上看到的必须是底账答的，不是前端猜的。
   */
  const readRelease = useCallback(async (): Promise<ReleaseState> => {
    /* 五个读：目录清单、盘上对得上的、盘上对不上的、**那些对不上的认得出是哪一版吗**、
       归档区里躺着的旧版本。中间三个合起来才是四档（见 `ReleaseFileState`）；
       归档是"更新过之后会变"的那一份，所以它跟着这一路一起读，而不是单开一次首屏读 */
    const [catalog, mine, drifted, trust, keep] = await Promise.all([
      api.getRuntimeCatalog(),
      api.getDownloadedFiles(),
      api.getStaleFiles(),
      api.getDeliveryTrust(),
      api.getArchivedFiles(),
    ])
    setArchived(keep)
    const downloaded = new Set(mine.map((f) => f.fileName))
    const driftedSet = new Set(drifted.map((f) => f.fileName))
    /*
     * 盘上那几份的**落盘时刻**（下载到本机的时刻 = 下载管道写盘那一刻的 mtime）。
     * 已下载与有更新的是两批不相交的文件，合成一张表查；没有的时刻是 `null`
     * —— 界面照实说「未知」，不编。
     */
    const onDiskAt = new Map<string, number | null>(
      [...mine, ...drifted].map((f) => [f.fileName, f.modifiedUnix]),
    )
    /* 判词按**文件名**查 —— 与下载 / 应用 / 读正文同一套口径（这套系统认的一直是 fileName） */
    const verdicts = new Map(trust.map((t) => [t.fileName, t.verdict]))
    /*
     * 四档**只在这里判一次**（两张表都读这一份结果，不许自己再判一次）。
     * 第三个读没给判词（两读之间有缝，理论上到不了）时按「内容异常」处理 ——
     * 那正是"认不出"的实情，而它的后果只是"得重下一份干净的"，宁可响。
     */
    const stateOf = (fileName: string): ReleaseFileState => {
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
    const listed: ReleasePresetSource[] = catalog.files.flatMap((f) => {
      const kind = catalogKindToFileKind(f.kind)
      if (kind === null) return []
      return [
        {
          uid: `${f.machineId}/${f.versionId}`,
          machineId: f.machineId,
          versionId: f.versionId,
          fileName: f.fileName,
          kind,
          size: f.size,
          releaseVersion: null,
          /* 盘上那份的落盘时刻（下载时刻）；云端语义的时间走目录的发布时刻（`publishedAt`），两回事 */
          modifiedUnix: onDiskAt.get(f.fileName) ?? null,
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
      localReleases: localList,
      stale: staleList,
      localUids: localList.map((p) => p.uid),
    }
  }, [])

  useEffect(() => {
    let alive = true

    const load = async () => {
      /* 用户线那一读与其他几个一起发：它不挡首屏（扫一个空目录几乎不花时间） */
      const [repo, list, menu, local, mine, copied] = await Promise.all([
        api.getPresetFiles(),
        api.getMachines(),
        api.getMenu(),
        api.getLocalFiles(),
        api.getUserPresetFiles(),
        api.getSlicerCopied(),
      ])
      setMine(mine)

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
      /* 唯一底账读一次 —— 下面默认落地那台机型也要用它，所以在这里拿 */
      const entry = await api.getActivePreset().catch(() => null)
      setMachines(list)
      /* 仅归档的文件在这里就被剔掉 —— 用户端一处都不该出现 */
      setTree(buildPresetTree(list, repo, inputs, archivedIds(menu)))
      setLocalIds(local)
      setActive(entry)
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
       * 官方交付那一路**不挡首屏**。
       *
       * 它是两条独立的读（catalog 清单 + 下载区），失败时 release 落成空态、页面照常渲染；
       * 这一页要先看的「本机那份注册表」答出来的表，没有理由让整页等它。
       *
       * **先画本地 catalog（下面这次 readRelease），再在后台检查 Bootstrap**（第十七刀）：
       * 顺序不能反 —— 用户一进预设页看到的必须是本地那份（离线也看得到），
       * 后台检查只是"路过时问一声远端有没有新版"，不问到就把页面挂住。
       */
      void readRelease()
        .then(async (next) => {
          if (!alive) return
          setRelease(next)
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
            const refreshed = await readRelease().catch(() => null)
            if (refreshed !== null && alive) setRelease(refreshed)
          }
        })
        .catch(() => {
          /* 官方交付那一路读不到（目录/下载区任一失败）就落空态，不挡首屏 —— 与三态纪律一致：
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
  }, [readRelease, importRevision])

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
   * 两个写。**先写底账，再重读底账**，中间不插一句前端自己的推断 ——
   * 底账在 Rust 侧（`mkp/` + `run/active-preset.json`），这一个来回是一次 IPC；
   * 界面上看到的必须是底账答的，不是前端猜的。
   *
   * 不 catch：失败要传到页面上说出来（没下载就应用、SHA 对不上这两种失败
   * 就是从这里冒上去的）。
   */
  const apply = useCallback(
    async (fileName: string, origin: ActiveOrigin = 'official', path?: string) => {
      setActive(await api.applyActivePreset(fileName, origin, path))
    },
    [],
  )

  const clearApply = useCallback(async () => {
    await api.clearActivePreset()
    setActive(await api.getActivePreset())
  }, [])

  const copy = useCallback(async (assetId: string) => {
    await api.copyToSlicer(assetId)
    setSlicerCopied(await api.getSlicerCopied())
  }, [])

  /*
   * 官方交付那一路的「下载」：走新世界下载管道，落进下载区 `mkp/`。
   * 不 catch：失败传给页面说出来，与 `apply` / `copy` 同一条规矩。（「应用」走上面那一个。）
   */
  const downloadRelease = useCallback(
    async (fileName: string, onTick?: (tick: DownloadTick) => void) => {
      await api.downloadCatalogFile(fileName, onTick)
      setRelease(await readRelease())
    },
    [readRelease],
  )

  /*
   * 临时编辑那条链：起手（复制官方正文进临时文件）、边改边存、放弃、另存成用户文件。
   *
   * 全是薄薄一层转发 —— 判定不在前端（哪一份能改、草稿在哪、存成什么名字，都由后端答）。
   * 只有**另存之后**多做一件事：重读用户线（本地表里那一半「我的文件」要跟着变）。
   */
  const beginEdit = useCallback(
    (fileName: string, origin: ActiveOrigin = 'official', path?: string) =>
      api.beginPresetEdit(fileName, origin, path),
    [],
  )

  const putDraft = useCallback((text: string) => api.putPresetDraft(text), [])

  const discardDraft = useCallback(() => api.discardPresetDraft(), [])

  const commitDraft = useCallback(async () => {
    const done = await api.commitPresetDraft()
    setMine(await api.getUserPresetFiles())
    return done
  }, [])

  /*
   * 第十层：两条用户文件管理。同一条路子 —— 写底账 → **重读底账**：
   * 改名之后使用中指针可能跟着改了名，所以顺手重读一遍（界面显示的永远是底账答的）；
   * 删除不碰使用中指针（正在使用的不给删），只重读用户线。
   */
  const rename = useCallback(async (path: string, newName: string) => {
    const done = await api.renameUserPreset(path, newName)
    setMine(await api.getUserPresetFiles())
    setActive(await api.getActivePreset().catch(() => null))
    return done
  }, [])

  const remove = useCallback(async (path: string) => {
    await api.deleteUserPreset(path)
    setMine(await api.getUserPresetFiles())
  }, [])

  /* 另存为一份新的（第十一层）：只重读用户线 —— 新的一份要出现在表里；使用中指针不归它管 */
  const copyAsNew = useCallback(async (path: string, newName: string) => {
    const done = await api.copyUserPreset(path, newName)
    setMine(await api.getUserPresetFiles())
    return done
  }, [])

  /* 在文件管理器里显示（第十三层）：纯外部动作 —— 不重读、不改任何状态 */
  const reveal = useCallback(async (path: string) => {
    await api.revealInFolder(path)
  }, [])

  /*
   * 批量：一次把多份交给后端，回来后**不管成没成先重读底账**（成功的那些已经落盘了），
   * 再把逐份结局原样交回页面。顺序 = 请求顺序（后端保证），页面按它列。
   */
  const downloadReleaseBatch = useCallback(
    async (fileNames: string[], onTick?: (tick: DownloadTick) => void) => {
      const outcomes = await api.downloadCatalogFiles(fileNames, onTick)
      setRelease(await readRelease())
      return outcomes
    },
    [readRelease],
  )

  const machineId = at?.machineId ?? ''
  const versionId = at?.versionId ?? ''

  return {
    loading: error === null && !ready,
    error,
    needsNewerClient,
    machines,
    machineId,
    versionId,
    machine: machines.find((m) => m.id === machineId),
    tree,
    localIds,
    mine,
    active,
    slicerCopied,
    pick,
    pickMachine,
    apply,
    clearApply,
    copy,
    release,
    archived,
    downloadRelease,
    downloadReleaseBatch,
    beginEdit,
    putDraft,
    discardDraft,
    commitDraft,
    rename,
    remove,
    copyAsNew,
    reveal,
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
 */
const PINNED_KEY = STORAGE.clientPresetsPinned

/**
 * 置顶是**纯前端的排序**，所以它刷新之后保留是对的：它记的是「我常用哪几个文件」，
 * 不是一份未保存的改动。读坏了就退化成空集合 —— 一个排序偏好不值得让页面起不来。
 */
function readPinned(): Set<string> {
  try {
    const raw = window.localStorage.getItem(PINNED_KEY)
    if (raw === null) return new Set()
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return new Set()
    return new Set(parsed.filter((v): v is string => typeof v === 'string'))
  } catch {
    /* 隐私模式会抛，手改坏了 JSON 也会抛。两种都退化成「这次没有置顶」 */
    return new Set()
  }
}

function writePinned(next: Set<string>): void {
  try {
    window.localStorage.setItem(PINNED_KEY, JSON.stringify([...next]))
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
  /** 右边那条分段控件：位置。它切的是两张**互不相干**的表，不是同一批数据的筛选 */
  scope: PresetScopeAxis
  setScope: (next: PresetScopeAxis) => void

  /** 本地表 —— 本机磁盘上有什么（官方副本 + 我的文件） */
  local: PresetTableData<PresetLocalRow>
  /** 云端表 —— 菜单上有什么官方文件 */
  cloud: PresetTableData<PresetCloudRow>

  /**
   * 批量要处理的那一批：**未下载 + 旧版本 + 内容异常**，已下载的不进来（不重复下）。
   *
   * 判定就是云端表行上那个 `releaseState`（唯一判据），这一层没长第二套状态判断；
   * 范围取**筛选前**的云端行 —— 按机型与类型（表的口径），**不受搜索词影响**。
   * 三档分开数：前两档是"补上官方那份"，第三档是"盘上那份我们不认"，用户要做的事不一样。
   */
  pending: {
    fileNames: string[]
    /** 其中「未下载」几份 */
    missing: number
    /** 其中「旧版本」几份（认得出是官方某一版旧版） */
    stale: number
    /** 其中「内容异常」几份（这台机器上查不出它属于哪一版） */
    tampered: number
    total: number
  }

  /** 置顶集合（`pinKey`）。落 localStorage，纯前端排序 */
  pinned: Set<string>
  togglePin: (pinKey: string) => void

  /**
   * 正在使用的**唯一那一条**（新世界底账 `run/active-preset.json`）。`null` = 一套都还没应用（**不是错误**）。
   *
   * 顶部状态条挂在它身上，**不跟着机型下拉走** —— 它是全局唯一的一条事实。
   */
  applied: ActivePreset | null
  /** 已应用那台机型的显示名。查不到就退回 id，不留空 */
  appliedMachineText: string
  /** 已应用那个文件的文件名（官方行查树、release 行查发布清单）。都查不到退回 ref */
  appliedFileName: string
  /**
   * 正在使用的这一份**是用户自己的那份**（不是官方交付的）。
   *
   * 状态条据此多写一小句说明 —— 两条线都能成为使用中的那一份（第七层），
   * 用户得看得出"现在跑的是我改的那一份"。
   */
  appliedIsMine: boolean

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
    layers: string[]
    /** 当前生效的值。空串 = 「全部」；选中的值不在可选集里（换了机型 / 表）时自动视为全部 */
    nozzle: string
    layer: string
    setNozzle: (next: string) => void
    setLayer: (next: string) => void
  }
}

export function usePresetPage(data: PresetData): PresetPage {
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

  const [pinned, setPinned] = useState<Set<string>>(readPinned)

  const togglePin = useCallback((pinKey: string) => {
    setPinned((now) => {
      /* 新建一个 Set 而不是改原来那个：React 靠引用变化才重画这张表 */
      const next = new Set(now)
      if (next.has(pinKey)) next.delete(pinKey)
      else next.add(pinKey)
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
      mine: data.mine,
      localIds: localSet,
      slicerCopiedIds: copiedSet,
      active: data.active,
      kind,
      query,
      pinned,
      releasePresets: data.release.presets,
      localReleases: data.release.localReleases,
      staleReleases: data.release.stale,
      releaseVersion: data.release.version,
      releaseAt: data.release.publishedAt,
    }),
    [
      data.active,
      data.machineId,
      data.release.publishedAt,
      data.release.localReleases,
      data.release.presets,
      data.release.stale,
      data.release.version,
      data.mine,
      copiedSet,
      kind,
      localSet,
      machines,
      pinned,
      query,
    ],
  )

  const localBase = useMemo(() => localRows(input), [input])
  const cloudBase = useMemo(() => cloudRows(input), [input])

  /*
   * 批量那一批。取**云端表筛前**的行，两个理由：
   *   ① 云端 = 目录登记的全集，本地只是它的子集（同一份文件在两张表里都有）——
   *      拿一边数就够了，合起来数会重复；
   *   ② **不受搜索词影响**：搜索是"我在找什么"，不该悄悄改变"按一下要动几份"。
   * 筛选（机型 / 类型）已经在 `cloudRows` 里做过了，所以这里只按状态挑。
   */
  const pending = useMemo(() => {
    const rows = cloudBase.rows.filter(
      (r) => r.origin === 'release' && r.releaseState !== undefined && r.releaseState !== 'ok',
    )
    return {
      fileNames: rows.map((r) => r.fileName),
      missing: rows.filter((r) => r.releaseState === 'missing').length,
      stale: rows.filter((r) => r.releaseState === 'old').length,
      tampered: rows.filter((r) => r.releaseState === 'tampered').length,
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
  const layerValue = kind === 'slicer' && slicerValues.layers.includes(layer) ? layer : ''

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
   * 文件名两个来源分别查：release 行在发布清单里、官方行在树里 —— 都查不到退回 ref 本身。
   */
  const appliedMachine = data.machines.find((m) => m.id === data.active?.machineId)
  const appliedFileName = useMemo((): string => {
    const entry = data.active
    if (entry === null) return ''
    /* 底账给的就是文件名；本地表 / 云端表都按它对（官方交付行与目录登记同名） */
    const hit = data.release.presets.find((p) => p.fileName === entry.fileName)
    return hit?.fileName ?? entry.fileName
  }, [data.active, data.release.presets])

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
    applied: data.active,
    appliedMachineText: appliedMachine?.display ?? data.active?.machineId ?? '',
    appliedFileName,
    appliedIsMine: data.active?.origin === 'mine',
    query,
    setQuery,
    searching: query.trim() !== '',
    slicerFilters: {
      nozzles: slicerValues.nozzles,
      layers: slicerValues.layers,
      nozzle: nozzleValue,
      layer: layerValue,
      setNozzle,
      setLayer,
    },
  }
}

