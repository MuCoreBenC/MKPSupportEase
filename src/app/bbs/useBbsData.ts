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
 *     只有一处：serve 期端点 `/api/bbs/presets`（读本机 BBS 目录，实时；见
 *     `tools/dev-server/bbsFs.mjs`，dev 与 preview 都挂）。
 *     本仓不打包那 285 个预设快照（作者裁决 §6-2），所以**没有退档**：读不到就是
 *     `sourceMode: 'none'` 加一句理由，页面照实说，不拿一份陈旧快照充数。
 *
 *     随之拿掉的是「按静态路径取单份预设」那条路 —— 那条路上踩过 `encodeURIComponent`
 *     把 `@BBL` 编成 `%40` 的坑（sirv 按规范不解 reserved 字符）。现在单份预设一律走
 *     `/api/bbs/preset?scope=&uid=&file=`，参数交给 URLSearchParams 编，这类坑不复存在。
 *
 * # 为什么不走 src/api
 *
 * 那一层是「MKP 自己的业务数据」的契约。BBS 这批是**另一个仓库的提取产物**，
 * 形状由 Bambu Studio 决定、我们只读不写，且 278 份系统预设要按继承链**按需取**。
 * 把它塞进契约只会让契约多出一堆永远不会有后端实现的方法。
 *
 * # 文档缓存为什么用 ref + 计数器，而且键上带来源
 *
 * 按继承链取预设是「选一条链 → 并行拉 3~4 份 → 合并」。这些文档只是算基准的原料，
 * 放 state 里会让每拉到一份就整页重渲染一次。所以缓存住在 ref（Map），
 * 拉完一批之后 bump 一次 `docsVersion` —— 一条链一次重算，不是四次。
 *
 * 缓存键是 `${来源}:${重扫次数}:${预设名}` 而**不是**光一个预设名，换来源时也**不清缓存**。
 * 原因是踩过的一个坑：原来的写法是「换来源就 docs.clear()」，而 StrictMode 下取数的
 * effect 会跑两遍 —— 第二遍的 clear 落在「链已经拉完」之后，于是刚拿到的文档被清掉，
 * 页面报「读某某失败」。键上带来源之后，旧来源的条目自然读不到，不需要谁去清它，
 * 也就不存在这个时序。几百份小 json 留在内存里无所谓。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { normalizeList } from './bbsSource'
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
 * 单份预设的 URL。只有本机目录这一路：查询参数由端点用 URLSearchParams 解，
 * 文件名里的 `@BBL` / 空格 / 中文都交给它编，这里不自己拼路径。
 */
function presetUrl(item: BbsPresetItem): string {
  const q = new URLSearchParams({ scope: item.scope, file: item.file })
  if (item.uid) q.set('uid', item.uid)
  return `/api/bbs/preset?${q}`
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
  /** 并行拉一批（按名字），拉完只 bump 一次 */
  loadDocs: (names: string[]) => Promise<void>
  /** 缓存版本号，用来做 useMemo 的依赖 */
  docsVersion: number

  /** 导入一份 json：进清单顶部「导入的」那一组，返回它的 key */
  addImported: (fileName: string, doc: BbsPresetDoc) => string
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

  const [raw, setRaw] = useState<BbsListRaw>({})
  const [sourceMode, setSourceMode] = useState<BbsSourceMode>('none')
  const [sourceRoot, setSourceRoot] = useState('')
  const [sourceNote, setSourceNote] = useState('')
  const [imported, setImported] = useState<BbsPresetItem[]>([])
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

  /* 清单只有一处来源：serve 期的本机目录端点。scanNonce 变了就重来一遍（「重扫本机」） */
  useEffect(() => {
    let alive = true
    void (async () => {
      setLoading(true)
      let live: BbsListRaw | null = null
      try { live = await getJSON<BbsListRaw>('/api/bbs/presets') } catch { /* 没起服务器就是这条路 */ }
      if (!alive) return

      if (live?.available) {
        setRaw({ user: live.user ?? [], system: live.system ?? [] })
        setSourceMode('live')
        setSourceRoot(live.root ?? '')
        setSourceNote('')
      } else {
        /* 端点说得出理由就用它的（「没装 BBS」时最有用）；说不了就是没起服务器 */
        setRaw({})
        setSourceMode('none')
        setSourceRoot('')
        setSourceNote(live?.reason || '没起 dev / preview 服务器，`/api/bbs/presets` 取不到')
      }
      /* 换来源之后单份预设的 URL 也变了 —— 靠缓存键上的来源前缀区分，不清缓存（见文件头） */
      setDocsVersion((n) => n + 1)
      setLoading(false)
    })()
    return () => { alive = false }
  }, [scanNonce])

  const presets = useMemo(() => normalizeList(raw, imported), [raw, imported])

  /* 链解析按名字查：同名时系统预设优先（inherits 指向的就该是系统那一层） */
  const presetByName = useMemo(() => {
    const m = new Map<string, BbsPresetItem>()
    for (const p of presets) if (p.scope === 'system') m.set(p.name, p)
    for (const p of presets) if (!m.has(p.name)) m.set(p.name, p)
    return m
  }, [presets])

  /** 缓存键带来源与重扫次数：换来源不用清缓存，也就没有「清晚了」的时序 */
  const keyOf = useCallback(
    (name: string) => `${sourceMode}:${scanNonce}:${name}`,
    [sourceMode, scanNonce],
  )

  const getDoc = useCallback((name: string) => docs.current.get(keyOf(name)), [keyOf])

  const loadDocs = useCallback(async (names: string[]) => {
    const todo = names.filter((n) => !docs.current.has(keyOf(n)))
    if (!todo.length) return
    await Promise.all(todo.map(async (name) => {
      const item = presetByName.get(name)
      if (!item) return
      /* 导入进来的那几份内容就在身上，不用发请求 */
      if (item.doc) { docs.current.set(keyOf(name), item.doc); return }
      try { docs.current.set(keyOf(name), await getJSON<BbsPresetDoc>(presetUrl(item))) }
      catch { docs.current.set(keyOf(name), null) }
    }))
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
    rescan,
  }
}
