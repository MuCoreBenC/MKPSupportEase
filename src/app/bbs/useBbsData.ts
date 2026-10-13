/*
 * BBS 数据的取数层：元数据 + 预设清单 + 单份预设的缓存。
 *
 * # 两路数据
 *
 *   元数据（参数定义 / 版面 / 出厂默认 / 原件 SVG）
 *     `public/bbs/{registry,layout,defaults,icons}.json` —— Bambu Studio 源码提取器的产物，
 *     **随本仓分发**（来源与许可写在 `public/bbs/PROVENANCE.md`）。
 *     **不 import，运行时 fetch**：registry 一份 327 KB，进 bundle 不划算。
 *
 *   预设清单与内容
 *     走 **IPC**：`api.getBbsPresets()`（本机有哪几份）+ `api.readBbsPreset(scope, uid, file)`
 *     （单份正文，按继承链**按需取**）。真机上那两条由 `src-tauri/src/ipc/bbs.rs` 现读
 *     本机 BBS 目录（用户层 + 系统层）；浏览器预览里由 mock 给一份演示集合
 *     （`api/mockServer/bbsLocal.ts`）。
 *     本仓不打包那 285 个预设快照（作者裁决 §6-2），所以**没有退档**：读不到就是
 *     `sourceMode: 'none'` 加一句理由，页面照实说，不拿一份陈旧快照充数。
 *
 * # 为什么**现在**走 src/api 了（2026-10-13 改的口径）
 *
 * 原来这一层直连试验场那个 dev 端点（`tools/dev-server/bbsFs.mjs` 的
 * `/api/bbs/presets` + `/api/bbs/preset`）。那一版**只在 serve 期存在**，于是打包后的
 * 桌面版里「本机工艺」这一档永远是空的 —— 它没有 dev server。那一档要真能用只有 IPC
 * 一条路；而契约里那两条命令的返回形状与端点**逐字段一致**，所以这里换的只是"从哪儿取"，
 * 下面的解析与缓存一行没动。端点随之从产品仓退役（试验场那份留着给自己用）。
 *
 * 「按静态路径取单份预设」那条路早就拿掉了 —— 它踩过 `encodeURIComponent` 把 `@BBL`
 * 编成 `%40` 的坑（sirv 按规范不解 reserved 字符）。现在单份正文的参数由 IPC 那层收。
 *
 * # 文档缓存为什么用 ref + 计数器，而且键上带来源
 *
 * 按继承链取预设是「选一条链 → 并行拉 3~4 份 → 合并」。这些文档只是算基准的原料，
 * 放 state 里会让每拉到一份就整页重渲染一次。所以缓存住在 ref（Map），
 * 拉完一批之后 bump 一次 `docsVersion` —— 一条链一次重算，不是四次。
 *
 * 缓存键是 `${来源}:${预设名}` 而**不是**光一个预设名，换来源时也**不清缓存**。
 * 原因是踩过的一个坑：原来的写法是「换来源就 docs.clear()」，而 StrictMode 下取数的
 * effect 会跑两遍 —— 第二遍的 clear 落在「链已经拉完」之后，于是刚拿到的文档被清掉，
 * 页面报「读某某失败」。键上带来源之后，旧来源的条目自然读不到，不需要谁去清它，
 * 也就不存在这个时序。几百份小 json 留在内存里无所谓。
 *
 * ★ 键里**不再带重扫次数**（2026-10-13）。原来带，于是点一次「重扫本机」整条继承链的
 *   缓存全部作废 → 右侧参数区那几格（基准 705 项 / 改动 25 项 / 行数）先掉成空、再拉回来，
 *   数字当着人的面跳一下。作者：「他是把那些数字变了一下……他这个刷新不是重扫本机吗？
 *   那它为什么会动右侧的参数呢？它不应该刷新左侧的列表的感觉才对吗？」
 *   他说得对：重扫要的是**左侧那份清单**重新读一遍（刚在 BBS 里新建的预设出现在列表里），
 *   已经展开的这份预设和它的基准并没有变 —— 键上少一个重扫次数，右侧就一动不动。
 *   代价：改了**已经选中那一份**的内容之后重扫，右侧不会跟着变（换一份再换回来会）。
 *   这与这一页是"查看器"的口径一致（整页只读、值不落盘）。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, errorText } from '../../api'
import { PRINTER_BY_MACHINE_ID, normalizeList } from './bbsSource'
import type {
  BbsDefaultsFile,
  BbsIconsFile,
  BbsLayoutFile,
  BbsLayoutTab,
  BbsListRaw,
  BbsParamMeta,
  BbsPresetDoc,
  BbsPresetItem,
  BbsRegistryFile,
  BbsSourceMode,
  BbsSyncInfo,
  BbsValues,
} from './bbsTypes'

/** 静态资源的前缀。元数据就住在 `public/bbs/` 下 */
const BBS = '/bbs'

/**
 * 手点一次「重扫本机」之后，那颗 ↻ 至少转这么久（毫秒）。
 *
 * 本机目录是同步读的，端点几十毫秒就回来了 —— 一转就停，人根本看不见，
 * 于是"点了一下没反应"。给一个最短时长，让那一下有落点。
 * 只对手点的重扫生效：首次进页面读清单不等，那时列表是空的，已经有
 * 「正在读清单…」顶着（再拖半秒只是让首屏慢）。
 */
const RESCAN_MIN_MS = 700

async function getJSON<T>(path: string): Promise<T> {
  const res = await fetch(path, { cache: 'no-store' })
  if (!res.ok) throw new Error(`HTTP ${res.status} ${path}`)
  /*
   * 路径对不上时 Vite 不给 404，而是**回退成 index.html 并返回 200** ——
   * 直接 res.json() 只会得到一句「Unexpected token '<'」，看不出是路径问题。
   * 所以先认一下 content-type，把原因说清楚。
   */
  const type = res.headers.get('content-type') ?? ''
  if (!type.includes('json')) {
    throw new Error(`${path} 返回的不是 JSON（content-type: ${type || '空'}），大概是路径对不上被回退成了页面`)
  }
  return res.json() as Promise<T>
}

/**
 * 单份预设的**正文**（原文读回来，解析在这儿）。
 *
 * 路径不在前端拼：`scope` / `uid` / `file` 三个字段就是 IPC 那层的边界，文件名里的
 * `@BBL` / 空格 / 中文都不必在这里转义（老路子上踩过 `%40` 的坑）。
 *
 * 只有本机目录那两档要现读 —— 导入的与我们自己的那几份内容就在身上，调用点先一步收下了。
 */
async function fetchDoc(item: BbsPresetItem): Promise<BbsPresetDoc> {
  if (item.scope !== 'user' && item.scope !== 'system') {
    throw new Error(`这一份不带正文：${item.scope} / ${item.file}`)
  }
  return JSON.parse(await api.readBbsPreset(item.scope, item.uid, item.file)) as BbsPresetDoc
}

/**
 * 一份"我们自己的切片器配置"进清单时要带的东西。
 *
 * `machineId` 与 `path` 来自**交付面的登记**（catalog 的 `files[]`），机型与喷嘴照它们认 ——
 * 不看 doc 里的继承链。作者 2026-10-13：「不用管它那个继承链，我那个工作台里面已经把它
 * 分好了，注册表里面就记录好了……根据那个 catalog 里面的登记去给它分类」。
 */
export interface BbsSlicerRow {
  fileName: string
  machineId: string
  /** 目录里的落点：`assets/bbs/Process/<喷嘴>mm/…`，喷嘴从这一段取 */
  path: string
  /**
   * 正文。**没有 = 这一份还没下载**（2026-10-13）。
   *
   * 没下载的那几份照样进清单：目录里登记着，机型与喷嘴也认得出（按登记拼），
   * 只是点不开 —— 它们灰着列在「MKP 工艺」组里，点了去预设页把它取回来。
   */
  doc?: BbsPresetDoc
}

/**
 * 我们自己的切片器配置（`MKPProcess …json`）→ 清单里的一项。
 *
 * `model / printer / nozzle / target / isAbstract` 由 `normalizeList` 里的 `resolveTarget`
 * 现算（与 `addImported` 同一套：这里填的是占位，进 `normalizeList` 之后会被覆盖）——
 * 但**认机型用的 `compatible_printers` 在这里就补上了**，见下面那段。
 */
function slicerItem({ fileName, machineId, path, doc }: BbsSlicerRow): BbsPresetItem {
  /* 没下载的那几份没有正文：名字退回文件名（那一份的档名与文件名本来就同族） */
  const name = String(doc?.name || fileName.replace(/\.json$/i, ''))
  /*
   * 按目录登记拼出 BBS 自己会写的那种 `compatible_printers`（`Bambu Lab A1 mini 0.4 nozzle`）：
   * `machineId` 给机型、落点里的 `Process/<喷嘴>mm/` 给喷嘴。
   *
   * 有了它，`resolveTarget` 就不必顺 `inherits` 跳父档 —— 那几份原本不带这个字段，
   * 于是"本机装没装那台机的系统预设"决定它认不认得出来（症状：下拉里出现 `unknown`）。
   *
   * 认不出（目录里没登记这一台、或路径形状不对）就退回 doc 自己的 / null：
   * 那时走回继承链那条老路，认不出照实显示 `unknown`，不硬编一个机型。
   */
  const printer = PRINTER_BY_MACHINE_ID[machineId]
  const nozzle = /\/Process\/([\d.]+)mm\//.exec(path)?.[1]
  const compatible = printer !== undefined && nozzle !== undefined
    ? `${printer} ${nozzle} nozzle`
    : (doc?.compatible_printers ?? null)
  /* 正文不在手上 = 还没下载。这一份**照样进清单**（见 BbsSlicerRow 那段），
     只是不可选：点了是"去预设页取回来" */
  const missing = doc === undefined
  return {
    key: `mkp::${fileName}::${name}`,
    name,
    scope: 'mkp',
    uid: null,
    file: fileName,
    inherits: typeof doc?.inherits === 'string' ? doc.inherits : null,
    instantiation: doc?.instantiation ?? null,
    compatible_printers: compatible,
    broken: null,
    mtime: 0,
    model: 'unknown',
    printer: 'unknown',
    nozzle: '?',
    target: 'unknown|?',
    isAbstract: false,
    selectable: !missing,
    reason: missing ? '还没下载' : null,
    notDownloaded: missing,
    doc,
  }
}

export interface BbsData {
  loading: boolean
  /** 元数据都没拿到（`public/bbs/*.json` 缺了）—— 这是空态，不是崩溃 */
  fatal: string | null
  registry: Record<string, BbsParamMeta>
  layout: BbsLayoutTab[]
  defaults: BbsValues
  icons: Record<string, string>
  sync: BbsSyncInfo | null

  sourceMode: BbsSourceMode
  sourceRoot: string
  /** 读不到本机目录的理由（`sourceMode === 'none'` 时那句给人看的话）。空串 = 读到了 */
  sourceNote: string
  presets: BbsPresetItem[]
  presetByName: Map<string, BbsPresetItem>

  /** 缓存里的文档。没拉过返回 undefined，拉过但失败返回 null */
  getDoc: (name: string) => BbsPresetDoc | null | undefined
  /**
   * 并行拉一批（按名字），拉完只 bump 一次。
   * `force` = 连已经在缓存里的也重新读一遍（「重扫本机」用）。
   */
  loadDocs: (names: string[], force?: boolean) => Promise<void>
  /** 缓存版本号，用来做 useMemo 的依赖 */
  docsVersion: number

  /** 导入一份 json：进清单顶部「导入的」那一组，返回它的 key */
  addImported: (fileName: string, doc: BbsPresetDoc) => string
  /**
   * 把我们自己的切片器配置（预设页下载到本机的那几份）**整批**摆进清单，
   * `scope = 'mkp'`。传空数组 = 一份都没有（盘上被删了也跟着消失）。
   *
   * 为什么是整批而不是一份一份 `add`：调用方给的 `downloaded` 本来就是
   * "盘上现在有哪几份"的完整底账（契约里的 `getDownloadedFiles` 同一口径）。
   */
  setSlicerPresets: (rows: BbsSlicerRow[]) => void
  /** 重扫本机 BBS（只有 live 才有意义） */
  rescan: () => void
}

export function useBbsData(): BbsData {
  const [loading, setLoading] = useState(true)
  const [fatal, setFatal] = useState<string | null>(null)

  const [registry, setRegistry] = useState<Record<string, BbsParamMeta>>({})
  const [layout, setLayout] = useState<BbsLayoutTab[]>([])
  const [defaults, setDefaults] = useState<BbsValues>({})
  const [icons, setIcons] = useState<Record<string, string>>({})
  const [sync, setSync] = useState<BbsSyncInfo | null>(null)

  /* 只留那两批（本机目录的两层）—— 契约那份是**整棵树的报告**（含 available / reason），
     读到的树才有值，读不到就是一个空壳，所以这一格按 Partial 收 */
  const [raw, setRaw] = useState<Partial<BbsListRaw>>({})
  const [sourceMode, setSourceMode] = useState<BbsSourceMode>('none')
  const [sourceRoot, setSourceRoot] = useState('')
  const [sourceNote, setSourceNote] = useState('')
  const [imported, setImported] = useState<BbsPresetItem[]>([])
  /*
   * 我们自己的切片器配置（2026-10-13）。与 `imported` 同属"内容带在身上"的那一类，
   * 区别只在来源与那一档入口：由页面把盘上已下载的那几份读出来整批灌进来（见 `setSlicerPresets`）。
   */
  const [slicer, setSlicer] = useState<BbsPresetItem[]>([])
  const [scanNonce, setScanNonce] = useState(0)

  const docs = useRef(new Map<string, BbsPresetDoc | null>())
  const [docsVersion, setDocsVersion] = useState(0)

  /* 元数据只取一次：它不随来源变，也不随重扫变 */
  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const [reg, lay, def, ico, syn] = await Promise.all([
          getJSON<BbsRegistryFile>(`${BBS}/registry.json`),
          getJSON<BbsLayoutFile>(`${BBS}/layout.json`),
          getJSON<BbsDefaultsFile>(`${BBS}/defaults.json`),
          /* 图标缺了只是没图标，不该拦住整页 */
          getJSON<BbsIconsFile>(`${BBS}/icons.json`).catch(() => ({ svg: {} })),
          getJSON<BbsSyncInfo>(`${BBS}/_sync.json`).catch(() => null),
        ])
        if (!alive) return
        setRegistry(reg.params ?? {})
        setLayout(lay.tabs ?? [])
        setDefaults(def.values ?? {})
        setIcons(ico.svg ?? {})
        setSync(syn)
      } catch (err) {
        if (!alive) return
        setFatal(
          `没读到 public/bbs 下的 BBS 元数据（${String(err)}）。`
          + '那四份 json（registry / layout / defaults / icons）随本仓分发，'
          + '少了说明产物不全，见 public/bbs/PROVENANCE.md。',
        )
      }
    })()
    return () => { alive = false }
  }, [])

  /* 清单只有一处来源：本机 BBS 目录（IPC 现读）。scanNonce 变了就重来一遍（「重扫本机」） */
  useEffect(() => {
    let alive = true
    void (async () => {
      setLoading(true)
      const started = performance.now()
      let live: BbsListRaw | null = null
      let why = ''
      try {
        live = await api.getBbsPresets()
      } catch (err) {
        /* 这一条真出错时（没装 BBS 不是错误，那种情况走 available:false）照实说 */
        why = `读本机 BBS 目录失败：${errorText(err)}`
      }
      if (!alive) return

      /* 手点的那一次重扫：把那只 ↻ 补到最短时长（理由见 RESCAN_MIN_MS） */
      if (scanNonce > 0) {
        const left = RESCAN_MIN_MS - (performance.now() - started)
        if (left > 0) await new Promise((r) => setTimeout(r, left))
        if (!alive) return
      }

      if (live?.available) {
        setRaw({ user: live.user ?? [], system: live.system ?? [] })
        setSourceMode('live')
        setSourceRoot(live.root ?? '')
        setSourceNote('')
      } else {
        /* 命令说得出理由就用它的（「没装 BBS」时最有用）；说不出来就是这一次真读失败了 */
        setRaw({})
        setSourceMode('none')
        setSourceRoot('')
        setSourceNote(live?.reason || why || '读不到本机 BBS 目录')
      }
      /* 换来源之后单份预设的 URL 也变了 —— 靠缓存键上的来源前缀区分，不清缓存（见文件头）。
         重扫时这一句照样执行，但文档缓存没失效（键里没有重扫次数了），重算出来的数一模一样，
         右侧参数区不会动 —— 这正是「重扫只刷左侧列表」要的。 */
      setDocsVersion((n) => n + 1)
      setLoading(false)
    })()
    return () => { alive = false }
  }, [scanNonce])

  /* 三路合一：本机目录（raw）+ 我们自己的切片器配置 + 手工导入的那几份。
     后面两路都排在本机那批之前 —— 清单顺序与抽屉里分组顺序是两回事（分组在 bbsSource） */
  const presets = useMemo(
    () => normalizeList(raw, [...slicer, ...imported]),
    [raw, slicer, imported],
  )

  /* 链解析按名字查：同名时系统预设优先（inherits 指向的就该是系统那一层） */
  const presetByName = useMemo(() => {
    const m = new Map<string, BbsPresetItem>()
    for (const p of presets) if (p.scope === 'system') m.set(p.name, p)
    for (const p of presets) if (!m.has(p.name)) m.set(p.name, p)
    return m
  }, [presets])

  /**
   * 缓存键只带**来源**（不带重扫次数 —— 理由见文件头那段：带了会让重扫去动右侧的数字）。
   * 换来源不用清缓存，也就没有「清晚了」的时序。
   */
  const keyOf = useCallback(
    (name: string) => `${sourceMode}:${name}`,
    [sourceMode],
  )

  const getDoc = useCallback((name: string) => docs.current.get(keyOf(name)), [keyOf])

  const loadDocs = useCallback(async (names: string[], force = false) => {
    /*
     * `force` = 不管缓存里有没有，都重新读一遍（「重扫本机」用：你刚在 BBS 里改完保存的
     * 那一份要能读到新值）。
     *
     * 关键在**怎么更新缓存**：新读到的先攒在临时 Map 里，全部到位之后才一次写进去、
     * 一次 bump —— 于是右侧那几格数字不会经过"基准 0 项"那种中间态。作者要的是
     * "数字滚一下落到真值"，不是"先变成另一个数再变回来"。
     */
    const todo = force ? names : names.filter((n) => !docs.current.has(keyOf(n)))
    if (!todo.length) return
    const fresh = new Map<string, BbsPresetDoc | null>()
    await Promise.all(todo.map(async (name) => {
      const item = presetByName.get(name)
      if (!item) return
      /* 导入进来的那几份内容就在身上，不用现读 */
      if (item.doc) { fresh.set(keyOf(name), item.doc); return }
      try { fresh.set(keyOf(name), await fetchDoc(item)) }
      catch { fresh.set(keyOf(name), null) }
    }))
    for (const [key, doc] of fresh) docs.current.set(key, doc)
    setDocsVersion((n) => n + 1)
  }, [presetByName, keyOf])

  const addImported = useCallback((fileName: string, doc: BbsPresetDoc) => {
    const name = String(doc.name || fileName.replace(/\.json$/i, ''))
    const key = `imported::${fileName}::${name}`
    const item: BbsPresetItem = {
      key,
      name,
      scope: 'imported',
      uid: null,
      file: fileName,
      inherits: typeof doc.inherits === 'string' ? doc.inherits : null,
      instantiation: null,
      compatible_printers: doc.compatible_printers ?? null,
      broken: null,
      mtime: 0,
      model: 'unknown',
      printer: 'unknown',
      nozzle: '?',
      target: 'unknown|?',
      isAbstract: false,
      selectable: true,
      reason: null,
      doc,
    }
    docs.current.set(keyOf(name), doc)
    setDocsVersion((n) => n + 1)
    /* 同名再导入一次就换掉旧的那份：同一个文件改了再拖进来是常态 */
    setImported((prev) => [item, ...prev.filter((x) => x.key !== key)])
    return key
  }, [keyOf])

  /**
   * 整批换掉"我们自己的切片器配置"。
   *
   * 调用方（`PageBbs` 里读盘那一段）读的是盘上**现在**有哪几份，所以是整批换而不是
   * 一份份往里加：删掉一份之后它得跟着消失。
   */
  const setSlicerPresets = useCallback((rows: BbsSlicerRow[]) => {
    setSlicer(rows.map((row) => slicerItem(row)))
  }, [])

  /**
   * 重扫本机：**只重新读左侧那份清单**。
   *
   * 它不会让右侧参数区动一下 —— 已经加载的文档缓存不受影响（键里没有重扫次数，见 `keyOf`），
   * 所以基准 / 改动 / 行数在重扫前后是同一个数。那只 ↻ 至少转 `RESCAN_MIN_MS`，
   * 那一下才有落点。
   */
  const rescan = useCallback(() => setScanNonce((n) => n + 1), [])

  return {
    loading,
    fatal,
    registry,
    layout,
    defaults,
    icons,
    sync,
    sourceMode,
    sourceRoot,
    sourceNote,
    presets,
    presetByName,
    getDoc,
    loadDocs,
    docsVersion,
    addImported,
    setSlicerPresets,
    rescan,
  }
}
