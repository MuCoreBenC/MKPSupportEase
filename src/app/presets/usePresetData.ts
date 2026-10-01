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
 *   用户自己的文件      api.getLocalUserFiles()                   → **固定演示集合**，实测 3 个（云端没有它们）
 *   当前使用的那一条    `store/package.activeEntry()`（`STORAGE.clientActive`）→ **全局唯一**，null = 一套都还没应用
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
 * MKP     生效 = 设为当前使用        唯一底账 STORAGE.clientActive  状态 已应用 / 未应用
 * 切片器   生效 = 复制到切片器的目录   api.copyToSlicer()     状态 已复制 / 未复制
 * ```
 *
 * MKP 那个写落在本机（localStorage），写完**重读底账**而不是自己在本地改状态：
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
 * 始终只有一个已应用」**。所以它现在是**唯一底账答的一条独立事实**（`STORAGE.clientActive`，
 * 单条目）：整张表里最多一行带 ● 已应用，切机型也不会变出第二个。
 *
 * # 这一页与参数页各有一份「当前机型 / 版本」
 *
 * 外壳按 tab 切换页面（切走就卸载），两页之间没有共享状态，跨页联动也没做。
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from '../../api'
import type { Machine } from '../../api'
import { STORAGE } from '../../api/storageKeys'
import { useSessionState } from '../shared/useSessionState'
import {
  activeEntry,
  activatePreset,
  fetchPreset,
  latestOf,
  listCloud,
  presets as localReleasesOf,
} from '../store/package'
import type { ActiveEntry } from '../store/package'
import type { ReleasePresetSource } from './presetTree'
import {
  applySlicerFilters,
  archivedIds,
  buildPresetTree,
  cloudRows,
  localRows,
  machineNode,
  machinesInScope,
  slicerFilterValues,
} from './presetTree'
import type {
  LocalUserFile,
  PresetCloudRow,
  PresetKindAxis,
  PresetLocalRow,
  PresetRowsInput,
  PresetScopeAxis,
  PresetTableData,
  PresetTree,
  PresetVersionInput,
} from './presetTree'

export interface PresetData {
  loading: boolean
  /** null = 没出错。非 null 时页面要把这句话显示出来，不要装成空表 */
  error: string | null

  /**
   * 工作台发布：云端最新一次 Release + 本机 preset 目录的现况。
   *
   * 三格底账都在 `store/package`（`STORAGE.cloud` / `STORAGE.clientPresets` /
   * `STORAGE.clientActive`），这一层只读不写；写走下面的 `downloadRelease` / `apply`。
   */
  release: ReleaseState

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
   * 用户自己放进预设目录的文件（第二个读）。
   *
   * **云端没有它们**，所以它们只出现在本地表里、没有交付身份、也不参与套餐与菜单。
   * 同样是固定演示集合（实测 3 个：2 个 MKP / 1 个切片器，其中一个故意没标适用机型）。
   */
  userFiles: LocalUserFile[]
  /**
   * 正在使用的**唯一那一条**：从 `STORAGE.clientActive` 读出来的，**全表最多一份**，
   * `null` = 一套都还没应用（**不是错误**）。
   *
   * 官方行 / release 行应用后写着的都是它 —— 假后端的 `getAppliedPreset()` 界面不再读。
   */
  active: ActiveEntry | null
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
   * 把某一份设为当前使用的**那一条**（官方行 / release 行共用这一个写）。
   *
   * 写 `STORAGE.clientActive`（唯一底账）**然后重读底账** —— 界面看到的永远是底账答的，
   * 与 `copy` 同一条规矩。失败照抛给调用方（页面用提示条说出来），**不在这里吞**。
   */
  apply: (entry: ActiveEntry) => Promise<void>
  /** 把某个切片器 profile 复制进切片器目录，然后重新拉 `getSlicerCopied()`。同上 */
  copy: (assetId: string) => Promise<void>
  /**
   * 「下载」工作台发布的那一份：云端 Release 里的 TOML → 本机 preset 目录。
   *
   * 与官方文件那颗「下载」（`api.downloadFiles()`，假后端必抛未实现）不同，
   * 这一条走 `package.fetchPreset`，**真的能下**。失败照抛给页面说出来。
   */
  downloadRelease: (uid: string) => Promise<void>
}

/**
 * 工作台发布这一路的现况。云端一份、本机一份 —— 两件事分开。
 * （「正在用的」不在这里：它只有一个出处 `STORAGE.clientActive`，见 `PresetData.active`。）
 */
export interface ReleaseState {
  /** 云端最新一次发布的包版本（`null` = 云端还没有工作台的发布） */
  version: string | null
  /** 云端最新一次发布的时刻 */
  at: string | null
  /** 云端这一次发布会里的预设文件 */
  presets: ReleasePresetSource[]
  /** 本机 preset 目录里已有的（`STORAGE.clientPresets` 的值，本地表的 release 行就是它们） */
  localReleases: ReleasePresetSource[]
  /** 本机已下载的 uid 集合 */
  localUids: string[]
}

const EMPTY_RELEASE: ReleaseState = {
  version: null,
  at: null,
  presets: [],
  localReleases: [],
  localUids: [],
}

export function usePresetData(): PresetData {
  const [machines, setMachines] = useState<Machine[]>([])
  const [tree, setTree] = useState<PresetTree>({ machines: [], totalFiles: 0 })
  const [localIds, setLocalIds] = useState<string[]>([])
  const [userFiles, setUserFiles] = useState<LocalUserFile[]>([])
  const [active, setActive] = useState<ActiveEntry | null>(null)
  const [slicerCopied, setSlicerCopied] = useState<string[]>([])
  const [ready, setReady] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [at, setAt] = useState<{ machineId: string; versionId: string } | null>(null)
  const [release, setRelease] = useState<ReleaseState>(EMPTY_RELEASE)

  /**
   * 把工作台发布这一路的现况读一遍。
   *
   * 全部来自 localStorage + 静态云端（`package` 那层），与 api 那几个读互不相干；
   * 下载 / 应用之后**重读一遍**而不是本地改状态 —— 与上面 `apply` / `copy` 同一条规矩：
   * 界面上看到的必须是底账答的，不是前端猜的。
   */
  const readRelease = useCallback(async (): Promise<ReleaseState> => {
    const cloud = await listCloud()
    const latest = latestOf(cloud)
    const mine = localReleasesOf()
    const cloudPresets: ReleasePresetSource[] = (latest?.presets ?? []).map((p) => ({
      uid: `${p.machineId}/${p.versionId}`,
      machineId: p.machineId,
      versionId: p.versionId,
      fileName: p.fileName,
      content: p.content,
      at: latest?.at ?? null,
      releaseVersion: latest?.version ?? null,
    }))
    const localList: ReleasePresetSource[] = Object.entries(mine).map(([uid, p]) => ({
      uid,
      machineId: p.machineId,
      versionId: p.versionId,
      fileName: p.fileName,
      content: p.content,
      at: p.at,
      releaseVersion: p.version,
    }))
    return {
      version: latest?.version ?? null,
      at: latest?.at ?? null,
      presets: cloudPresets,
      localReleases: localList,
      localUids: Object.keys(mine),
    }
  }, [])

  useEffect(() => {
    let alive = true

    const load = async () => {
      const [repo, list, menu, local, mine, copied] = await Promise.all([
        api.getPresetFiles(),
        api.getMachines(),
        api.getMenu(),
        api.getLocalFiles(),
        api.getLocalUserFiles(),
        api.getSlicerCopied(),
      ])

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
      const entry = activeEntry()
      setRelease(await readRelease())
      setMachines(list)
      /* 仅归档的文件在这里就被剔掉 —— 用户端一处都不该出现 */
      setTree(buildPresetTree(list, repo, inputs, archivedIds(menu)))
      setLocalIds(local)
      setUserFiles(mine)
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
    }

    load().catch((e: unknown) => {
      if (!alive) return
      setError(e instanceof Error ? e.message : String(e))
    })

    return () => {
      alive = false
    }
  }, [readRelease])

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
   * MKP 那份底账在本机（localStorage），这一个来回是即时的；真后端接上时这里该显示的
   * 「正在应用…」由页面的提示条负责，不在这一层编。
   *
   * 不 catch：失败要传到页面上说出来（切片器路径没配、目标已存在这两种真后端的失败
   * 就是从这里冒上去的）。
   */
  const apply = useCallback(async (entry: ActiveEntry) => {
    activatePreset(entry) /* release 条目本机没有这一份时它自己不动 —— 底账的规矩，不在这里猜 */
    setActive(activeEntry())
  }, [])

  const copy = useCallback(async (assetId: string) => {
    await api.copyToSlicer(assetId)
    setSlicerCopied(await api.getSlicerCopied())
  }, [])

  /*
   * 工作台发布那一路的「下载」。不 catch：失败传给页面说出来，
   * 与 `apply` / `copy` 同一条规矩。（「应用」走上面那一个 `apply`。）
   */
  const downloadRelease = useCallback(
    async (uid: string) => {
      const cloud = await listCloud()
      const latest = latestOf(cloud)
      if (latest === null) throw new Error('云端暂时没有可下载的发布')
      const got = fetchPreset(latest, uid)
      if (got === null) throw new Error('云端这一次发布里没有打包这一份')
      setRelease(await readRelease())
    },
    [readRelease],
  )

  const machineId = at?.machineId ?? ''
  const versionId = at?.versionId ?? ''

  return {
    loading: error === null && !ready,
    error,
    machines,
    machineId,
    versionId,
    machine: machines.find((m) => m.id === machineId),
    tree,
    localIds,
    userFiles,
    active,
    slicerCopied,
    pick,
    pickMachine,
    apply,
    copy,
    release,
    downloadRelease,
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

  /** 置顶集合（`pinKey`）。落 localStorage，纯前端排序 */
  pinned: Set<string>
  togglePin: (pinKey: string) => void

  /**
   * 正在使用的**唯一那一条**（`STORAGE.clientActive`）。`null` = 一套都还没应用（**不是错误**）。
   *
   * 顶部状态条挂在它身上，**不跟着机型下拉走** —— 它是全局唯一的一条事实。
   */
  applied: ActiveEntry | null
  /** 已应用那台机型的显示名。查不到就退回 id，不留空 */
  appliedMachineText: string
  /** 已应用那个文件的文件名（官方行查树、release 行查发布清单）。都查不到退回 ref */
  appliedFileName: string

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
      userFiles: data.userFiles,
      localIds: localSet,
      slicerCopiedIds: copiedSet,
      active: data.active,
      kind,
      query,
      pinned,
      releasePresets: data.release.presets,
      localReleases: data.release.localReleases,
      releaseVersion: data.release.version,
    }),
    [
      data.active,
      data.machineId,
      data.release.localReleases,
      data.release.presets,
      data.release.version,
      data.userFiles,
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
    if (entry.kind === 'release') {
      const hit =
        data.release.localReleases.find((p) => p.uid === entry.ref) ??
        data.release.presets.find((p) => p.uid === entry.ref)
      return hit?.fileName ?? entry.ref
    }
    const node = entry.machineId === null ? undefined : machineNode(data.tree, entry.machineId)
    const file = node?.versions.flatMap((v) => v.files).find((f) => f.id === entry.ref)
    return file?.fileName ?? entry.ref
  }, [data.active, data.release.localReleases, data.release.presets, data.tree])

  return {
    unsupported: machineAt !== undefined && machineAt.unavailable,
    missing: machineAt?.missing ?? [],
    kind,
    setKind,
    scope,
    setScope,
    local,
    cloud,
    pinned,
    togglePin,
    applied: data.active,
    appliedMachineText: appliedMachine?.display ?? data.active?.machineId ?? '',
    appliedFileName,
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

