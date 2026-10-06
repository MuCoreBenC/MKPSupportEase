/**
 * 工作台外壳 —— C14 版式（feat/b05-14b-c14-port，方案见 C14-PORT-PLAN.md）。
 *
 * 版式来自试验场 C14 第十八轮定稿：
 *
 *  1. **左侧一级导航按工作流排序**（机型是起点，参数第二；作者：「为什么维护啊？
 *     没必要分开」）—— 不分「维护」组，没有第二棵树。宽档可拖宽（64~320），
 *     窄档退成图标一列，mini 档横排 chips。
 *  2. **页面挂过就一直挂着**（`.pageSlot`，切走只是 `display: none`）—— 作者：
 *     「我改了套餐里选的某一个版本，去看别的界面再点回来，它又变成新的了，这不行」。
 *     非当前页的 element 用「冻结」缓存照原样复用（引用不变 → React 跳过那棵子树）。
 *  3. **模态框挂 `.shellBody`**（导航 + 页面 + 状态栏那一整层）—— 遮罩盖住的就是
 *     这个范围，标题栏（这里是 Tauri 原生窗口的系统栏）不在内。
 *  4. **撤销 / 重做 / 未保存改动在状态栏** —— 全工作台一条栈（作者：「它是一种
 *     全局的东西，要不就放在状态栏」）。
 *
 * # 外壳自己不做业务
 *
 * 状态、文案、能不能改全部由后端算好（`wb_book` / `wb_words` / `wb_preflight`）。
 * 外壳只做四件事：取数、把导航徽标算出来、拼撤销栈、分发到页面。
 *
 * # 与原型的差别（都是真后端决定的，不是视觉偷懒）
 *
 *  - 没有自绘标题栏 —— Tauri 原生窗口，标题栏归系统；
 *  - 没有「演示数据」徽标与「恢复默认」—— 每一笔写都是真写盘；
 *  - 导航徽标与「生成与发布」的读数来自 `wb_preflight`（每次写完重取一次），
 *    不是前端把检查重跑一遍。
 */
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import type { CSSProperties, ReactNode } from 'react'


import { FieldLayer } from './components/field'
import PageSkeleton from './components/Skeleton'
import { modalShortcutGate } from './components/modal'
import { useDensity } from './useDensity'
import {
  isAppError,
  wb,
  type BookView,
  type Boot,
  type IssueReport,
  type MetaApplied,
  type ParamMetaEdit,
  type Patch,
  type Refresh,
  type Words,
} from './api'
import { toasts } from './c14/toast'
import { OverlayHostCtx } from './c14/overlayHost'
import type { GotoFocus, PageId } from './c14/types'
import HistoryModal from './c14/HistoryModal'
import SplitterC14 from './c14/SplitterC14'
import { useSplitWidth } from './c14/useSplitWidth'
import MachinesPage from './views/MachinesPage'
import ParamsPage from './views/ParamsPage'
import BundlesPage from './views/BundlesPage'
import AssetsPage from './views/AssetsPage'
import BuildPage from './views/BuildPage'
import SettingsPage from './views/SettingsPage'
import s from './c14.module.css'

/**
 * 一级导航。前五页是 C14 定稿的顺序 —— C14 把矩阵并进了参数台的对照模式
 * （P3 已接），不再有独立的「对比」页。
 *
 * `settings` 排在末尾：它不是一个业务页 —— 预设根固定是 `<repo>/presets`，
 * 这一页只读地摆开数据根与子目录职责，导航角落那盏灯点它进来。
 */
interface NavItem {
  id: PageId
  label: string
  icon: ReactNode
  badge?: 'dirty' | 'build'
}

const NAV: NavItem[] = [
  /* 立方体 = 一台机器 */
  {
    id: 'machines',
    label: '机型与版本',
    icon: (
      <>
        <path d="M12 3 4 7.5v9L12 21l8-4.5v-9z" />
        <path d="M4 7.5 12 12l8-4.5" />
      </>
    ),
  },
  /* 两根带滑块的横杆 = 调参数 */
  {
    id: 'params',
    label: '参数台',
    badge: 'dirty',
    icon: (
      <>
        <path d="M4 8h16M4 16h16" />
        <circle cx="9" cy="8" r="2.2" />
        <circle cx="15" cy="16" r="2.2" />
      </>
    ),
  },
  /* 分格的箱子 = 一揽子资源 */
  {
    id: 'bundles',
    label: '套餐',
    icon: (
      <>
        <rect x="4" y="4" width="16" height="16" rx="1.5" />
        <path d="M4 10h16M10 4v16" />
      </>
    ),
  },
  /* 盾牌 + 勾：检查 → 生成 → 发布按顺序走的一页（C14 合并了发布中心） */
  {
    id: 'build',
    label: '生成与发布',
    badge: 'build',
    icon: (
      <>
        <path d="M12 3l8 3.5v6c0 4.2-3.4 7.4-8 8.5-4.6-1.1-8-4.3-8-8.5v-6z" />
        <path d="m9 12 2.2 2.2L15.5 10" />
      </>
    ),
  },
  /* 一张图 = 素材文件 */
  {
    id: 'assets',
    label: '资产库',
    icon: (
      <>
        <rect x="3.5" y="5" width="17" height="14" rx="1.5" />
        <path d="m3.5 15.5 4.2-4.2 3 3 3.8-3.8 6 5.5" />
      </>
    ),
  },
  /* 齿轮 = 只读的设置（数据根与子目录职责） */
  {
    id: 'settings',
    label: '设置',
    icon: (
      <>
        <circle cx="12" cy="12" r="3" />
        <path d="M12 3.5v2M12 18.5v2M3.5 12h2M18.5 12h2M6 6l1.4 1.4M16.6 16.6 18 18M18 6l-1.4 1.4M7.4 16.6 6 18" />
      </>
    ),
  },
]

type NavId = NavItem['id']

/** 导航宽度的三档（C14 第十七轮）：默认 236，可拖 64（图标档）~ 320（再宽只是挤正文） */
const NAVW = { dft: 236, min: 64, max: 320 }

/** 跨页定位载荷（C14 语义）：`key` 给套餐/资产页落到某一项（机型页 ④ 关联在用） */
interface Focus {
  machineId: string
  uid: string | null
  key?: string | null
}

/**
 * 撤销栈上的一条。**两种改动共用一个入口**（状态栏那两颗按钮 + Cmd+Z），所以
 * 条目自带「怎么倒回去」：
 *
 *   patches  值的改动 —— 反向 patch 交回 `wb.applyDraft`，后端算它自己的反向；
 *   meta     参数定义的改动（即时落盘那一路）—— **自带改前/改后两份整包**，
 *            撤销交改前、重做交改后，不用现算反向。label 写明「定义 · 某参数」，
 *            与值那条在栈里各说各的。
 */
type UndoEntry =
  | { kind: 'patches'; label: string; patches: Patch[] }
  | { kind: 'meta'; label: string; key: string; before: ParamMetaEdit; after: ParamMetaEdit }

/**
 * 状态条上的数据根怎么显示（2026-10-03，作者：「左下角的也是，没必要显示这个吧，这么长」）。
 *
 * 完整绝对路径放在 `title` 里（悬停能看到全部），行上只留**最后两段** ——
 * `…/projects/MKPSupportEase/presets` → `MKPSupportEase/presets`：够认出是哪份仓库，
 * 又不会把窄档的导航栏撑宽（实测撑宽之后那一块会溢出到内容区上，压着页脚）。
 */
const shortRoot = (p: string): string => {
  const parts = p.split(/[/\\]/).filter((s) => s !== '')
  return parts.length >= 2 ? `${parts[parts.length - 2]}/${parts[parts.length - 1]}` : p
}

export function WorkbenchApp() {
  const rootRef = useRef<HTMLDivElement>(null)
  const density = useDensity(rootRef)
  const toastList = useSyncExternalStore(toasts.subscribe, toasts.get)

  const [boot, setBoot] = useState<Boot | null>(null)
  const [words, setWords] = useState<Words | null>(null)
  const [book, setBook] = useState<BookView | null>(null)
  const [report, setReport] = useState<IssueReport | null>(null)
  const [fatal, setFatal] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const [page, setPage] = useState<NavId>('machines')
  const [focus, setFocus] = useState<Focus | null>(null)
  /** 定位页的 remount 代号：`goto` 过来是「换现场」，要重挂去读新的主选中 */
  const [focusGen, setFocusGen] = useState(0)
  /** 套餐 / 资产页的跨页预选（C14 的 GotoFocus：机型页 ④ 关联、套餐页反查都在用） */
  const [bundleSel, setBundleSel] = useState<string | null>(null)
  const [assetSel, setAssetSel] = useState<string | null>(null)
  /** 资产页身份筛选的预选（检查报告的孤儿文件跳过来替人筛好「可选」） */
  const [assetAssign, setAssetAssign] = useState<string | null>(null)

  /** 撤销 / 重做栈。会话内存，关窗就没 —— 草稿本身还在盘上 */
  const [undoStack, setUndoStack] = useState<UndoEntry[]>([])
  const [redoStack, setRedoStack] = useState<UndoEntry[]>([])

  const [navW, setNavW] = useSplitWidth('nav', NAVW.dft, NAVW.min, NAVW.max)
  const [historyOpen, setHistoryOpen] = useState(false)

  /** 模态框遮罩宿主（`.shellBody`）与当前页滚动区（页头按钮靠它对齐 `--sbw`） */
  const [overlayEl, setOverlayEl] = useState<HTMLDivElement | null>(null)
  const [slotEl, setSlotEl] = useState<HTMLDivElement | null>(null)

  /** 「后端状态变过了」计数器：每写一次 +1，页面据此重取（理由见旧版的教训注释） */
  const [tick, setTick] = useState(0)

  /* 页面常驻（C14 第十八轮）：第一次去才挂载，切走只藏起来 */
  const [visited, setVisited] = useState<NavId[]>([page])
  useEffect(() => {
    setVisited((v) => (v.includes(page) ? v : [...v, page]))
  }, [page])
  const held = useRef<Partial<Record<NavId, ReactNode>>>({})

  const fail = useCallback((e: unknown) => {
    setFatal(isAppError(e) ? `${e.message}${e.detail ? ` —— ${e.detail}` : ''}` : String(e))
  }, [])

  /* 首屏四步：boot（预设根定位不到也能显示数据根）→ 词表 → 整本 → 检查报告。
     **预设根定位不到不再是一道门**：六页业务照常，只是依赖预设的读数留空，
     问题横幅与「设置」页负责把数据根说清。ref 挡住 StrictMode 的第二遍（那三条命令全发两次） */
  const booted = useRef(false)
  useEffect(() => {
    if (booted.current) return
    booted.current = true
    void (async () => {
      try {
        const b = await wb.boot()
        setBoot(b)
        const [w, bk] = await Promise.all([wb.words(), wb.book()])
        setWords(w)
        setBook(bk)
        // 参数台默认谁都不选（C14 第十七轮）：空态留白 + 文案，
        // 「还没选」这个状态必须存在 —— 选中由左树那一下点击或 goto 产生
        setReport(await wb.preflight())
      } catch (e) {
        fail(e)
      }
    })()
  }, [fail])

  /** 检查报告重取。每次写完调一次 —— 一次手势一次 IPC，徽标与生成页读数跟着走 */
  const refreshReport = useCallback(() => {
    wb.preflight()
      .then(setReport)
      .catch(() => undefined)
  }, [])

  /** 整本重取。机型页的结构性写（建 / 删 / 复制版本）之后徽章的机型版本数要跟上 */
  const refreshBook = useCallback(() => {
    wb.book()
      .then(setBook)
      .catch(() => undefined)
    refreshReport()
  }, [refreshReport])

  /**
   * `presets/` 改过之后（「重新加载」= 后端重开一次会话）：换 `Boot`，并把
   * 整本与检查报告重取一遍 —— 预设数据变了会改这两处读数
   */
  const reloadBoot = useCallback(
    (b: Boot) => {
      setBoot(b)
      refreshBook()
    },
    [refreshBook],
  )

  /**
   * 走唯一写入口。`where` 说这次结果往哪个栈压（正向压撤销、撤销压重做、重做压撤销）
   * —— 三种情形一条路径，分三份写「能不能重做」就会有三种写法。
   */
  const run = useCallback(
    async (label: string, patches: Patch[], where: 'undo' | 'redo', refresh?: Refresh) => {
      setBusy(true)
      try {
        const out = await wb.applyDraft(label, patches, refresh)
        setBook(out.view)
        setTick((n) => n + 1)
        refreshReport()
        // 不可撤销的手势（生成记录）不进栈，否则栈里会有一条按不动的
        if (out.inverse.length > 0) {
          const entry: UndoEntry = { kind: 'patches', label, patches: out.inverse }
          if (where === 'undo') setUndoStack((st) => [...st, entry])
          else setRedoStack((st) => [...st, entry])
        }
        return out
      } catch (e) {
        fail(e)
        return null
      } finally {
        setBusy(false)
      }
    },
    [fail, refreshReport],
  )

  /*
   * meta 条目（参数定义）的执行：把条目里指定的那份整包交回去，成了就把这条
   * 压进对面那摞（撤销压重做、重做压撤销）。定义是即时落盘 —— 一次调用就是
   * 一次成败，失败就原地不动。
   */
  const runMeta = useCallback(
    async (
      entry: Extract<UndoEntry, { kind: 'meta' }>,
      dir: 'before' | 'after',
      toStack: 'undo' | 'redo',
    ): Promise<boolean> => {
      setBusy(true)
      try {
        await wb.setParamMeta(entry.key, entry[dir])
        /* 注册表变了：参数台按 tick 重取（注册表 / 配方台 / 矩阵三处派生跟着走） */
        setTick((n) => n + 1)
        refreshReport()
        if (toStack === 'undo') setUndoStack((st) => [...st, entry])
        else setRedoStack((st) => [...st, entry])
        return true
      } catch (e) {
        fail(e)
        return false
      } finally {
        setBusy(false)
      }
    },
    [fail, refreshReport],
  )

  /** 参数台「编辑定义」保存成功后交上来的一条：压进撤销栈（改前/改后都在手上） */
  const pushMeta = useCallback((m: MetaApplied) => {
    setUndoStack((st) => [...st, { kind: 'meta', ...m }])
    setRedoStack([])
  }, [])

  const undo = useCallback(async () => {
    const top = undoStack[undoStack.length - 1]
    if (!top) return
    if (top.kind === 'meta') {
      if (await runMeta(top, 'before', 'redo')) setUndoStack((st) => st.slice(0, -1))
      return
    }
    const out = await run(`撤销：${top.label}`, top.patches, 'redo')
    if (out) setUndoStack((st) => st.slice(0, -1))
  }, [undoStack, run, runMeta])

  const redo = useCallback(async () => {
    const top = redoStack[redoStack.length - 1]
    if (!top) return
    if (top.kind === 'meta') {
      if (await runMeta(top, 'after', 'undo')) setRedoStack((st) => st.slice(0, -1))
      return
    }
    const out = await run(`重做：${top.label}`, top.patches, 'undo')
    if (out) setRedoStack((st) => st.slice(0, -1))
  }, [redoStack, run, runMeta])

  const save = useCallback(async (): Promise<boolean> => {
    if (!book || book.dirtyCount === 0 || busy) return false
    setBusy(true)
    try {
      const out = await wb.save()
      setBook(out.view)
      setUndoStack([])
      setRedoStack([])
      setTick((n) => n + 1)
      refreshReport()
      toasts.push('已保存 —— 草稿写进仓库文件了')
      return true
    } catch (e) {
      fail(e)
      return false
    } finally {
      setBusy(false)
    }
  }, [book, busy, fail, refreshReport])

  const discard = useCallback(async () => {
    if (!book || book.dirtyCount === 0 || busy) return
    setBusy(true)
    try {
      setBook(await wb.discard())
      setUndoStack([])
      setRedoStack([])
      setTick((n) => n + 1)
      refreshReport()
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }, [book, busy, fail, refreshReport])

  /** 跨页定位（C14 语义）：跳过去并落上主选中；那一页重挂一次去读它 */
  const goto = useCallback((view: string, foc?: GotoFocus) => {
    const next = view as NavId
    if (foc) {
      if (next === 'params') {
        /* 参数台的定位一定有机型语境；套餐/资产那两路才会带 null 过来。
           `key` 一起带过去 —— 参数台落地后按它滚到那一项 + 闪一下 */
        setFocus({ machineId: foc.machineId ?? '', uid: foc.uid, key: foc.key ?? null })
        setFocusGen((g) => g + 1)
      } else if (next === 'bundles') {
        setBundleSel(foc.key ?? foc.uid ?? null)
        setFocusGen((g) => g + 1)
      } else if (next === 'assets') {
        /* key = 'optional' 是检查报告「孤儿文件」的暗号：替人筛好身份，不是选中某条 */
        setAssetAssign(foc.key === 'optional' ? 'optional' : null)
        setAssetSel(foc.key === 'optional' ? null : (foc.key ?? null))
        setFocusGen((g) => g + 1)
      }
    }
    setPage(next)
  }, [])

  /* 撤销 / 重做 / 保存的键盘入口装在外壳上 —— 撤销不是某几个页面的小功能。
     模态框开着（且没让路）就不穿透：框里的事框里自己管（G-code 框让路），
     不然 Cmd+Z 改的是遮罩后面看不见的草稿（作者的实测） */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      if (modalShortcutGate.blocking()) return
      const k = e.key.toLowerCase()
      if (k === 'z' && !e.shiftKey) {
        e.preventDefault()
        void undo()
      } else if ((k === 'z' && e.shiftKey) || k === 'y') {
        e.preventDefault()
        void redo()
      } else if (k === 's') {
        e.preventDefault()
        void save()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [undo, redo, save])

  /* 草稿不干净时，刷新前用浏览器原生弹窗问一嘴（自绘模态框弹出来时刷新已经走了） */
  const dirty = (book?.dirtyCount ?? 0) > 0
  useEffect(() => {
    if (!dirty) return
    const guard = (e: BeforeUnloadEvent) => {
      e.preventDefault()
      e.returnValue = ''
    }
    window.addEventListener('beforeunload', guard)
    return () => window.removeEventListener('beforeunload', guard)
  }, [dirty])

  /* 页头按钮与页面内容的右缘对齐：滚动条占的宽度量一次写进变量，不让页头去猜 */
  useEffect(() => {
    const el = slotEl
    const root = rootRef.current
    if (!el || !root) return
    const read = () => {
      const next = `${el.offsetWidth - el.clientWidth}px`
      if (root.style.getPropertyValue('--sbw') !== next) root.style.setProperty('--sbw', next)
    }
    read()
    const ro = new ResizeObserver(read)
    ro.observe(el)
    return () => ro.disconnect()
  }, [slotEl])

  /* ---------- 导航徽标（C14 两级：只有阻断用红，待办/未保存是灰） ---------- */

  const buildBadge = useMemo(() => {
    if (!report) return { n: 0, block: false }
    const n = report.issues.filter((i) => i.severity !== 'hint' && i.at.view === 'build').length
    return { n, block: report.blocks > 0 }
  }, [report])

  const badgeOf = (b?: NavItem['badge']): { n: number; block: boolean } => {
    if (b === 'dirty') return { n: dirty ? 1 : 0, block: false }
    if (b === 'build') return buildBadge
    return { n: 0, block: false }
  }

  /* ---------- 页头副标题 ---------- */

  const headSub = useMemo(() => {
    if (!book) return ''
    switch (page) {
      case 'machines':
        return `${book.badges.machines} 台机型 · ${book.badges.versions} 个版本`
      case 'params':
        return '改了先进草稿，保存才落盘'
      case 'bundles':
        return '套餐 = 交付的真源（bundles.toml）—— 装什么、谁在用、改指向，都在这一页'
      case 'build':
        return report
          ? `阻断 ${report.blocks} · 待办 ${report.todos} · 提示 ${report.hints}${report.hints > 0 && report.blocks === 0 && report.todos === 0 ? ' · 都过了' : ''}`
          : ''
      case 'assets':
        return '资产域定义 + 引用反查 —— 交付身份改了先进草稿'
      case 'settings':
        return '只读 —— 数据根与工作台子目录的职责'
    }
  }, [page, book, report])

  /* ---------- 页面渲染（挂载入口只有这一处） ---------- */

  /*
   * 整本 / 词表还没到 —— **别返回 null**。
   *
   * 之前这里 `return null`，于是点导航要等 `wb_book` + `wb_words` 回来才有画面，
   * 作者的验收是「必须立马显示，就是那个反馈」。改成**按页给一具骨架**：
   * 壳（页头 / 卡片框 / 行槽）立刻出来，数据一到整体换成真内容。
   *
   * 每页的形状给个大致对得上的（机型页两栏 / 生成页一叠卡）—— 骨架是占位，不是预览图。
   */
  const skeletonFor = (id: NavId): ReactNode => {
    switch (id) {
      case 'machines':
        return <PageSkeleton layout="cols" cards={2} rows={7} label="机型与版本正在加载" />
      case 'params':
        return <PageSkeleton layout="cols" cards={2} rows={8} label="参数台正在加载" />
      case 'bundles':
        return <PageSkeleton layout="cols" cards={2} rows={6} label="套餐正在加载" />
      case 'build':
        return <PageSkeleton layout="flow" cards={3} rows={5} label="生成与发布正在加载" />
      case 'assets':
        return <PageSkeleton layout="cols" cards={2} rows={6} label="资产库正在加载" />
      case 'settings':
        return <PageSkeleton layout="flow" cards={2} rows={4} label="设置正在加载" />
    }
  }

  const renderPage = (id: NavId): ReactNode => {
    /* 数据没到先给骨架（不再黑屏）—— 见 skeletonFor 的注 */
    if (!book || !words) return skeletonFor(id)
    switch (id) {
      case 'machines':
        return (
          <MachinesPage
            book={book}
            words={words}
            onGoto={goto}
            onSave={save}
            onBookRefresh={refreshBook}
          />
        )
      case 'params':
        return (
          <ParamsPage
            key={focusGen}
            book={book}
            words={words}
            initialFocus={focus}
            tick={tick}
            dirty={dirty}
            onApply={async (label, patches, refresh) => {
              const out = await run(label, patches, 'undo', refresh)
              return { desk: out?.desk ?? null, matrix: out?.matrix ?? null }
            }}
            onMetaApplied={pushMeta}
            onSave={() => void save()}
            onDiscard={() => void discard()}
            onUndo={() => void undo()}
            onGoto={goto}
          />
        )
      case 'bundles':
        return (
          <BundlesPage
            key={focusGen}
            words={words}
            tick={tick}
            initialSel={bundleSel}
            onGoto={goto}
          />
        )
      case 'assets':
        return (
          <AssetsPage
            key={focusGen}
            words={words}
            tick={tick}
            initialSel={assetSel}
            initialAssign={assetAssign}
            onGoto={goto}
            onApply={async (label, patches) => {
              await run(label, patches, 'undo')
            }}
          />
        )
      case 'build':
        /* `boot` 比整本晚到的那一瞬也给骨架（别在整本已到之后又黑一下） */
        if (!boot) return skeletonFor('build')
        return (
          <BuildPage
            boot={boot}
            book={book}
            words={words}
            report={report}
            tick={tick}
            onGoto={goto}
            onSave={save}
            onBookRefresh={refreshBook}
          />
        )
      case 'settings':
        if (!boot) return skeletonFor('settings')
        return <SettingsPage boot={boot} />
    }
  }
  const nowNode = renderPage(page)
  useEffect(() => {
    held.current[page] = nowNode
  })

  const navLabel = NAV.find((i) => i.id === page)?.label ?? ''

  const nav = (
    <nav className={s.nav} aria-label="一级导航">
      <div className={s.navList}>
        {NAV.map((it) => {
          const bd = badgeOf(it.badge)
          return (
            <button
              key={it.id}
              type="button"
              className={`${s.navItem} ${page === it.id ? s.navItemOn : ''}`}
              title={it.label}
              onClick={() => setPage(it.id)}
            >
              <span className={s.navIcon} aria-hidden>
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={1.6}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  {it.icon}
                </svg>
              </span>
              <span className={s.navLabel}>{it.label}</span>
              {bd.n > 0 && (
                <span
                  className={`${s.navBadge} ${bd.block ? '' : s.navBadgeSoft}`}
                  title={bd.block ? '有阻断没处理完' : '有待处理的项，但不挡生成'}
                >
                  {bd.n}
                </span>
              )}
            </button>
          )
        })}
      </div>

      {/*
        唯一的预设真相源。灯 + 短句 + 附注；窄档只留灯（完整信息在 title 上）。
        **点它进设置页** —— 那里只读地摆开三个数据根与子目录职责（没有可改的表单：
        预设根固定是 `<repo>/presets`）
      */}
      <div
        className={s.upstream}
        role="button"
        tabIndex={0}
        title={
          boot
            ? `预设源 ${boot.roots.presets}\n配方本 ${boot.roots.workbench}\n交付 ${boot.roots.delivery}\n点击查看设置`
            : '正在读…'
        }
        onClick={() => setPage('settings')}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            setPage('settings')
          }
        }}
      >
        <span className={s.upDot} data-on={!!boot} aria-hidden />
        <span className={s.upText}>
          <span className={s.upLine}>{boot ? '预设源' : '读取中'}</span>
          {/* 行上只留仓库名 + 目录名（见 `shortRoot`）；完整路径在整块的 title 上 */}
          <span className={s.upNote}>{boot ? shortRoot(boot.roots.presets) : '正在读…'}</span>
        </span>
      </div>

      {density !== 'compact' && density !== 'mini' && (
        <SplitterC14
          bodyRef={rootRef}
          side="nav"
          varName="--nav-w"
          width={navW}
          otherW={0}
          defaultValue={NAVW.dft}
          min={NAVW.min}
          max={NAVW.max}
          onCommit={setNavW}
          label="导航栏"
          className={s.pSplitNav}
        />
      )}
    </nav>
  )

  return (
    <div
      ref={rootRef}
      className={s.shell}
      data-wb
      data-no-chrome="yes"
      data-page={page}
      data-density={density}
      style={{ '--nav-w': `${navW}px` } as CSSProperties}
    >
      {fatal && (
        <div
          className="wb-banner"
          data-tone="danger"
          style={{ position: 'absolute', top: 8, left: 12, right: 12, zIndex: 60 }}
        >
          {fatal}
        </div>
      )}

      <OverlayHostCtx value={overlayEl}>
        <FieldLayer>
          <div className={s.shellBody} ref={setOverlayEl}>
            <div className={s.body}>
              {/* mini 档的页导航：侧栏整个藏掉之后，六个页面在这里还有入口 */}
              {density === 'mini' && (
                <nav className={s.mnav} aria-label="页面切换">
                  {NAV.map((it) => {
                    const bd = badgeOf(it.badge)
                    return (
                      <button
                        key={it.id}
                        type="button"
                        className={`${s.mnavItem} ${page === it.id ? s.mnavItemOn : ''}`}
                        title={it.label}
                        onClick={() => setPage(it.id)}
                      >
                        <span className={s.navIcon} aria-hidden>
                          <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={1.6}
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          >
                            {it.icon}
                          </svg>
                        </span>
                        <span className={s.mnavLabel}>{it.label}</span>
                        {bd.n > 0 && (
                          <span
                            className={`${s.navBadge} ${bd.block ? '' : s.navBadgeSoft}`}
                            title={bd.block ? '有阻断没处理完' : '有待处理的项，但不挡生成'}
                          >
                            {bd.n}
                          </span>
                        )}
                      </button>
                    )
                  })}
                </nav>
              )}
              {nav}

              <div className={s.main}>
                <div className={s.head}>
                  <div className={s.headT}>
                    <h1>{navLabel}</h1>
                    <span className={s.headSub}>{headSub}</span>
                  </div>
                  {/*
                    保存三件常驻（C14）：改参数会让这里亮起来；「机型与版本」页的
                    新增与删除是即时落盘的，不经过这三个按钮 —— 徽章只反映草稿。
                  */}
                  <div className={s.headOps}>
                    <span
                      className={`${s.tag} ${s.tagGhost}`}
                      title={
                        dirty
                          ? `有 ${book?.dirtyCount ?? 0} 处改动没落盘 —— 状态栏的「未保存改动」能看明细`
                          : (words?.save.saved.explain ?? '草稿和磁盘那一份一致')
                      }
                    >
                      {words ? words.save[dirty ? 'dirty' : 'saved'].label : '—'}
                    </span>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      disabled={!dirty || busy}
                      title={dirty ? undefined : (words?.disabled.nothingToSave ?? '草稿是空的')}
                      onClick={() => void discard()}
                    >
                      放弃
                    </button>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                      disabled={!dirty || busy}
                      title={dirty ? undefined : (words?.disabled.nothingToSave ?? '草稿是空的')}
                      onClick={() => void save()}
                    >
                      保存
                    </button>
                  </div>
                </div>

                {/*
                  这一条只剩**真正的开场失败**（今天只有「presets/ 定位不到」一种）。
                  预设根固定是 `<repo>/presets`，没有可改的路径表单 —— 把目录补齐后
                  点「重新加载」重开一次会话即可，不用重启应用；「设置页」进去只读地
                  看三个数据根到底指哪
                */}
                {boot?.problem && (
                  <div className="wb-banner" data-tone="danger" style={{ margin: '0 12px' }}>
                    {boot.problem}
                    {boot.detail && <> —— <span className="wb-mono">{boot.detail}</span></>}
                    <br />
                    确认仓库里的
                    <span className="wb-mono">presets/</span>
                    已就位，然后点
                    <button
                      type="button"
                      className="wb-link"
                      onClick={() => {
                        wb.reload().then(reloadBoot).catch(fail)
                      }}
                    >
                      重新加载
                    </button>
                    ，或去
                    <button type="button" className="wb-link" onClick={() => setPage('settings')}>
                      设置页
                    </button>
                    看数据根。
                  </div>
                )}

                {/* 正文 = 一叠 pageSlot：去过的页一直挂着，切走的只是藏起来 */}
                <div className={s.content}>
                  {visited.map((id) => {
                    const active = id === page
                    return (
                      <div
                        key={id}
                        className={s.pageSlot}
                        hidden={!active}
                        ref={active ? setSlotEl : undefined}
                      >
                        {active ? nowNode : held.current[id]}
                      </div>
                    )
                  })}
                </div>
              </div>
            </div>

            {/* 状态栏：三段读数 + 动作组（C14 版式；读数全部来自后端） */}
            <div className={s.sb}>
              <span className={s.sbSeg}>
                <span className={`${s.dot} ${dirty ? s.dotDirty : ''}`} />
                {words ? words.save[dirty ? 'dirty' : 'saved'].label : '—'}
              </span>
              <span className={`${s.sbBar} ${s.sbMinor}`}>|</span>
              <span
                className={`${s.sbSeg} ${s.sbMinor}`}
                title={(words && book ? words.artifact[book.artifact].explain : undefined) ?? undefined}
              >
                {words && book ? words.artifact[book.artifact].label : '—'}
              </span>
              <span className={`${s.sbBar} ${s.sbMinor}`}>|</span>
              <span
                className={`${s.sbSeg} ${s.sbMinor}`}
                title={(words && book ? words.snapshot[book.snapshot].explain : undefined) ?? undefined}
              >
                {words && book ? words.snapshot[book.snapshot].label : '—'}
              </span>
              <span className={s.sbOps}>
                <button
                  type="button"
                  className={s.sbBtn}
                  disabled={undoStack.length === 0 || busy}
                  title={
                    undoStack.length > 0
                      ? `撤销：${undoStack[undoStack.length - 1].label}`
                      : (words?.disabled.nothingToUndo ?? '没有可撤销的改动')
                  }
                  onClick={() => void undo()}
                >
                  撤销
                </button>
                <button
                  type="button"
                  className={s.sbBtn}
                  disabled={redoStack.length === 0 || busy}
                  title={
                    redoStack.length > 0
                      ? `重做：${redoStack[redoStack.length - 1].label}`
                      : (words?.disabled.nothingToUndo ?? '没有可重做的改动')
                  }
                  onClick={() => void redo()}
                >
                  重做
                </button>
                <button
                  type="button"
                  className={`${s.sbBtn} ${dirty ? s.sbBtnOn : ''}`}
                  title="看看这几处都改了什么 —— 每一格从什么值改成什么值"
                  onClick={() => setHistoryOpen(true)}
                >
                  未保存改动（{book?.dirtyCount ?? 0}）
                </button>
              </span>
            </div>

            <HistoryModal open={historyOpen} onClose={() => setHistoryOpen(false)} />

            {/* Toast：右下角，压在模态遮罩之上（遮罩开着时「已撤销」这类提示还得看得见） */}
            <div
              style={{
                position: 'absolute',
                right: 14,
                bottom: 42,
                zIndex: 50,
                display: 'flex',
                flexDirection: 'column',
                gap: 8,
                alignItems: 'flex-end',
              }}
            >
              {toastList.map((t) => (
                <div
                  key={t.id}
                  className={`${s.card} ${s.cardBody}`}
                  style={{ padding: '8px 12px', fontSize: 12, boxShadow: 'var(--shadow)' }}
                >
                  {t.text}
                  {t.action && (
                    <button
                      type="button"
                      className={s.toastAct}
                      onClick={() => {
                        t.action?.run()
                        toasts.close(t.id)
                      }}
                    >
                      {t.action.label}
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>
        </FieldLayer>
      </OverlayHostCtx>
    </div>
  )
}
