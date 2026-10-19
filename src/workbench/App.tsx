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
import { editQueue, settleAll } from './editQueue'
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
  type SandboxStatus,
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
import AnnotatePage from './views/AnnotatePage'
import BuildPage from './views/BuildPage'
import SettingsPage from './views/SettingsPage'
import DevPage from './views/DevPage'
import s from './c14.module.css'
import { pageActions } from './pageActions'

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
  /* 四角框 + 中间一块选中 = 在图上拖框 */
  {
    id: 'annotate',
    label: '标注台',
    icon: (
      <>
        <path d="M4 8.5V6.5A2.5 2.5 0 0 1 6.5 4h2M15.5 4h2A2.5 2.5 0 0 1 20 6.5v2M20 15.5v2a2.5 2.5 0 0 1-2.5 2.5h-2M8.5 20h-2A2.5 2.5 0 0 1 4 17.5v-2" />
        <rect x="9" y="9" width="6" height="6" rx="1" />
      </>
    ),
  },
  /* 盾牌 + 勾：检查 → 生成 → 发布按顺序走的一页（C14 合并了发布中心）。
     2026-10-19：作者把它挪到资产库 / 标注台之后、设置之前 —— 日常是"改数据"，
     生成发布是最后一站；跟设置那种低频页挨着更顺。 */
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
  /* 扳手 = 开发台（把看不见的那几屏做成按钮；只在 workbench 构建里存在） */
  {
    id: 'dev',
    label: '开发台',
    icon: (
      <>
        <path d="M14.7 6.3a3.5 3.5 0 0 0 4.6 4.6l-1.9-1.9 1.4-1.4 1.9 1.9a3.5 3.5 0 0 1-4.6-4.6z" />
        <path d="M14.7 6.3 4.5 16.5a1.5 1.5 0 0 0 2.1 2.1L17 8.4" />
      </>
    ),
  },
]

type NavId = NavItem['id']

/**
 * **走参数草稿的那几页**（页头那套「已保存 / 放弃 / 保存」的老语义属于它们）。
 *
 * 判据 = 这一页会不会往参数草稿里放东西（`wb.applyDraft` 那一路）：
 * 机型（先落盘再生成）、参数台、资产库（交付身份改了先进草稿）、生成与发布（同上）。
 * 其余：套餐页是即时落盘、设置页只读、开发台不落盘 ⇒ 页头不摆这一槽。
 * ★ 草稿非空时**无论在哪一页都摆**（见 `headOpsVisible` 的 ③）—— 别让改动存不下去。
 */
const DRAFT_PAGES: ReadonlySet<NavId> = new Set<NavId>(['machines', 'params', 'assets', 'build'])

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
  /**
   * 页头那一槽：**哪一页登记了就用它的**（例：标注台接管成"保存到仓库 / 放弃改动"）。
   * 没人登记（`null`）⇒ 回落到外壳自己那套（参数草稿）。见 `pageActions.ts` 头注。
   */
  const pageSlotRaw = useSyncExternalStore(pageActions.subscribe, pageActions.get)

  const [boot, setBoot] = useState<Boot | null>(null)
  /**
   * **测试模式（沙箱）**现在开着没有。外壳持有它、也照它换装（`data-sandbox` 整窗配色 +
   * 正文顶上那条横幅 + 状态栏那一枚）—— 作者 2026-10-08：「我不能迷迷糊糊的，不知道
   * 我在测试版还是正式版」。设置页那几张卡读的也是这一份，不各取各的。
   */
  const [sandbox, setSandbox] = useState<SandboxStatus | null>(null)
  const [words, setWords] = useState<Words | null>(null)
  const [book, setBook] = useState<BookView | null>(null)
  /**
   * 最新那一份整本（**只给回调读**）。
   *
   * `save` / `discard` 先 `await settleAll()` 把飞在空中的值写落完，再判
   * "草稿脏不脏" —— 而 settleAll 落地带来的 `setBook` 是异步的，这次调用的闭包
   * 里读不到它。`dirtyCount` 判错会让"刚点完步进器马上点保存"变成一次**静默丢弃**。
   */
  const bookRef = useRef<BookView | null>(null)
  bookRef.current = book
  const [report, setReport] = useState<IssueReport | null>(null)
  const [fatal, setFatal] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const [page, setPage] = useState<NavId>('machines')
  /**
   * 只认**当前页**那一槽：非激活页的元素被冻结、不会重渲染 ⇒ 它没机会注销自己，
   * 所以靠"页对不上就当没有"来失效（见 `pageActions.ts` 头注）。
   * ★ 这一份是**看得见的字**，不含动作；按钮点击一律走 `pageActions.run(…)`
   *   （拿最新那份闭包，见 `pageActions.ts` 头注"为什么 save/discard 不进快照"）。
   */
  const pageSlot = pageSlotRaw !== null && pageSlotRaw.page === page ? pageSlotRaw : null
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
        const [w, bk, sb] = await Promise.all([wb.words(), wb.book(), wb.sandboxStatus()])
        setWords(w)
        setBook(bk)
        setSandbox(sb)
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

  /**
   * **外面的文件改过了**（2026-10-19，作者：「我直接改了那些模型，它这里面能不能知道」）。
   *
   * 工作台读的是盘上的真字节，但读数只在写操作 / 进页时取 —— 作者在编辑器里改完
   * 3MF 再切回来，屏幕还是旧的。这里在窗口重新获得焦点时重取一遍「会变的那几样」：
   * 检查报告（交付目录对不对得上、导航徽标）+ `tick`（生成页交付清单 / 资产库列表）。
   *
   * 300ms 去抖：切窗常带一串 focus / blur（拖窗口、系统弹窗），不必每次都扫盘。
   * **不做轮询、不引入文件监视** —— 作者要的是"切回来就能看到"，不是秒级同步。
   */
  useEffect(() => {
    let t: number | null = null
    const onFocus = () => {
      if (t !== null) window.clearTimeout(t)
      t = window.setTimeout(() => {
        t = null
        refreshReport()
        setTick((n) => n + 1)
      }, 300)
    }
    window.addEventListener('focus', onFocus)
    return () => {
      window.removeEventListener('focus', onFocus)
      if (t !== null) window.clearTimeout(t)
    }
  }, [refreshReport])

  /**
   * 进这两页时也重取一次：它们是「盘上现在是什么样」的读数页
   * （生成与发布看交付目录，资产库看载荷字节）。
   */
  useEffect(() => {
    if (page !== 'build' && page !== 'assets') return
    refreshReport()
    setTick((n) => n + 1)
  }, [page, refreshReport])

  /**
   * 值写入那一路的检查报告**尾随去抖**（700ms）—— 见 `run` 的 `quiet`。
   *
   * `wb_preflight` 是这一串里最重的一条（parse recipes + 摸盘建 catalog）；
   * 连点步进器时每笔都跑它一次，正是"主线程被占死、数字不跟手"的主要来源。
   * 合到最后一笔之后跑一次就够：页头徽标最多迟 700ms（低频读数，可接受）。
   */
  const reportSoon = useRef<number | null>(null)
  const refreshReportSoon = useCallback(() => {
    if (reportSoon.current !== null) window.clearTimeout(reportSoon.current)
    reportSoon.current = window.setTimeout(() => {
      reportSoon.current = null
      refreshReport()
    }, 700)
  }, [refreshReport])
  useEffect(
    () => () => {
      if (reportSoon.current !== null) window.clearTimeout(reportSoon.current)
    },
    [],
  )

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
   * 测试模式换过（开关 / 重拷一份 / 清空）之后：**换装 + 按新根把整本重取一遍**。
   *
   * 后端切模式时已经把会话丢了（旧根那份 `Presets` / `Store` / 草稿全作废），所以
   * 这里必须重开一次会话读新根 —— 不重取的话，界面还摆着正式那一套机型与参数，
   * 而写盘已经落在沙箱里了：那正是"我到底在哪一边"的最坏一种错位。
   */
  const applySandbox = useCallback(
    (next: SandboxStatus) => {
      setSandbox(next)
      wb.reload().then(reloadBoot).catch(fail)
    },
    [reloadBoot, fail],
  )

  /**
   * 走唯一写入口。`where` 说这次结果往哪个栈压（正向压撤销、撤销压重做、重做压撤销）
   * —— 三种情形一条路径，分三份写「能不能重做」就会有三种写法。
   */
  const run = useCallback(
    async (
      label: string,
      patches: Patch[],
      where: 'undo' | 'redo',
      refresh?: Refresh,
      opts?: { quiet?: boolean },
    ) => {
      /*
       * `quiet` = 值写入那一路（见 `editQueue` 头注）。**不拨 `tick`、不置 `busy`**：
       *
       *  · 拨一次 `tick`，参数台会把 `wb_registry` / 整页 `wb_desk` / `wb_matrix`
       *    再各问一遍 —— 连点步进器时那正是"数字不跟手、停下才追账"的主因；
       *  · `busy` 一动整壳重渲染、页头那排按钮一起闪。
       *
       * 页要跟着走的那一份派生，由 `wb_apply_draft` **顺带返回**（`out.desk` /
       * `out.matrix`，后端按 `refresh` 现算）—— 不必事后再问一次。
       */
      const quiet = opts?.quiet === true
      if (!quiet) setBusy(true)
      try {
        const out = await wb.applyDraft(label, patches, refresh)
        setBook(out.view)
        if (!quiet) setTick((n) => n + 1)
        if (quiet) refreshReportSoon()
        else refreshReport()
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
        if (!quiet) setBusy(false)
      }
    },
    [fail, refreshReport, refreshReportSoon],
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
    /* 飞在空中的值写先落完 —— 否则撤销撤的是"落库前的那一半"，栈与页面会对不上 */
    await settleAll()
    /* 本地草稿层整层丢掉：撤销之后的真相在后端那份 desk 里（下一条 tick 会把它取回来） */
    editQueue.clear()
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
    /* 同 `undo`：先把在飞的写落完、把本地草稿层丢干净，再读栈 */
    await settleAll()
    editQueue.clear()
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
    if (busy) return false
    /* 飞在空中的值写先落完：`dirtyCount` 得是落完之后的账 ——
       否则"刚点完步进器马上点保存"会静默丢掉那一笔（读的是旧整本） */
    await settleAll()
    const cur = bookRef.current
    if (!cur || cur.dirtyCount === 0) return false
    /* 保存 = 提交点：本地草稿层整层丢掉，显示交回写进仓库的那份 */
    editQueue.clear()
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
  }, [busy, fail, refreshReport])

  const discard = useCallback(async () => {
    if (busy) return
    /* 同 `save`：先把在飞的写落完再判脏 —— 跳到一半的改动也得算进来 */
    await settleAll()
    const cur = bookRef.current
    if (!cur || cur.dirtyCount === 0) return
    /* 放弃 = 整条草稿丢：本地草稿层跟着一起丢 */
    editQueue.clear()
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
  }, [busy, fail, refreshReport])

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

  /**
   * 页头那一槽要不要显示（作者 2026-10-16：「有需要的页面就显示，不需要的页面就不显示」）。
   *
   * 三种"显示"的理由，任一成立即显示：
   *   ① 当前页**自己登记了**一槽（标注台）；
   *   ② 当前页**走参数草稿**（机型 / 参数 / 资产 / 生成 —— 那套老语义属于它们）；
   *   ③ 草稿**非空** —— 切到别的页也留一颗「保存」给它，别让改动存不下去。
   * 其余（设置页只读、开发台）不显示。
   */
  const headOpsVisible =
    pageSlot !== null || DRAFT_PAGES.has(page) || dirty
  /**
   * 「保存」那颗**现在点不点得动**（2026-10-17）：
   * - 页头被某一页接管（标注台）⇒ 听它的 `canSave`；
   * - 否则是外壳这套参数草稿 ⇒ 草稿非空、且没在保存中。
   */
  const saveOn =
    pageSlot !== null ? pageSlot.canSave === true && pageSlot.busy !== true : dirty && !busy
  /**
   * 点不动时**说出为什么**（悬停看得见）。禁用按钮恰恰是最需要 title 的地方 ——
   * 作者这一轮的病就是"点了没反应却不知道为什么"（其实是压根点不动）。
   */
  const saveWhy =
    pageSlot !== null
      ? '这一份没有要写回去的改动（和仓库里那份一样）'
      : (words?.disabled.nothingToSave ?? '没有未保存的改动')
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
      case 'annotate':
        return '在图上拖框 —— 导出的就是 marks（百分比坐标，浅深两张图共用一份）'
      case 'settings':
        return '只读 —— 数据根与工作台子目录的职责'
      case 'dev':
        return '把平时看不见的那几屏按按钮演出来（不读 G-code、不落记录）'
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
      case 'annotate':
        /* 这一页不读整本 / 词表，骨架基本见不到；给一具形状对得上的占位（见 renderPage 的门） */
        return <PageSkeleton layout="cols" cards={2} rows={6} label="标注台正在加载" />
      case 'settings':
        return <PageSkeleton layout="flow" cards={2} rows={4} label="设置正在加载" />
      case 'dev':
        return <PageSkeleton layout="flow" cards={2} rows={5} label="开发台正在加载" />
    }
  }

  const renderPage = (id: NavId): ReactNode => {
    /* 开发台不依赖整本 / 词表（它读的是后处理状态与场景清单）—— 先于数据闸渲染，
       不必跟着"整本还没到"一起给骨架 */
    if (id === 'dev') return <DevPage />
    /* 标注台同理：它读的是资产清单（一条独立命令）与一张图，等整本没有意义。
       ★ 它自己往页头登记那一槽（`usePageActions('annotate', …)`）—— 外壳按"页对不对得上"筛，
         非激活页即使被冻结复用也不会污染页头。 */
    if (id === 'annotate') return <AnnotatePage />
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
            onApply={async (label, patches, refresh, opts) => {
              const out = await run(label, patches, 'undo', refresh, opts)
              /* `ok` 让参数台那一层知道这笔到底写进去没有（失败时 overlay 要回落） */
              return { ok: out !== null, desk: out?.desk ?? null, matrix: out?.matrix ?? null }
            }}
            onMetaApplied={pushMeta}
            onSave={() => void save()}
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
            onApply={async (label, patches, opts) => {
              await run(label, patches, 'undo', undefined, opts)
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
        return <SettingsPage boot={boot} sandbox={sandbox} onSandbox={applySandbox} />
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
      /* 测试模式整窗换一层色（`c14tokens.css` 只改那几个变量，C14 的内衬全跟着走） */
      data-sandbox={sandbox?.enabled === true ? 'on' : 'off'}
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
                {/*
                  测试模式那条横幅：**常驻**，压在正文最上面。
                  它是三道明示里最显眼的一道（另外两道：整窗转琥珀、窗口标题的
                  【测试模式】前缀——后者在切走窗口、任务栏、截图里也看得见）。
                  `data-tone="warn"` 是现成的语义：这不是错误，是"要留意"。
                */}
                {sandbox?.enabled === true && (
                  <div
                    className="wb-banner"
                    data-tone="warn"
                    style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      gap: 12,
                      alignItems: 'baseline',
                    }}
                  >
                    <b>测试模式（沙箱）</b>
                    <span>
                      读写都在 <span className="wb-mono">{shortRoot(sandbox.sandboxRoot)}</span>{' '}
                      里（
                      {sandbox.ready
                        ? `${sandbox.files} 个文件 · ${(sandbox.bytes / 1024 / 1024).toFixed(1)} MB`
                        : '空的'}
                      ）—— <b>正式那份一个字节都不动</b>，发布与发版也是关着的。
                    </span>
                    {!sandbox.ready && (
                      <span style={{ color: 'var(--danger)' }}>
                        ☆ 沙箱是空的 —— 去设置页点「从正式重拷一份」再动手
                      </span>
                    )}
                    <span className={s.grow} />
                    <button
                      type="button"
                      className="wb-link"
                      onClick={() => setPage('settings')}
                    >
                      去设置页
                    </button>
                  </div>
                )}

                <div className={s.head}>
                  <div className={s.headT}>
                    <h1>{navLabel}</h1>
                    <span className={s.headSub}>{headSub}</span>
                  </div>
                  {/*
                    保存那一槽（C14 / 2026-10-16 作者）：
                    **上面这一个就是全站唯一的位置** —— 不再让哪一页在下面自己再摆一套。
                    三种情形：
                      ① 当前页**自己登记了**一槽（例：标注台 → 「保存到仓库 / 放弃改动」）⇒ 用它；
                      ② 走参数草稿的页（机型 / 参数 / 资产 / 生成）⇒ 用外壳这套（老样子）；
                      ③ 只读页与开发台 ⇒ **不显示**（作者：「有需要的页面就显示，不需要的页面就不显示」）。
                    ★ 草稿非空时**无论在哪一页都显示** —— 别让"改了参数切到设置页就存不了"。
                  */}
                  {headOpsVisible && (
                    <div className={s.headOps}>
                      {pageSlot !== null ? (
                        <>
                          <span
                            className={`${s.tag} ${s.tagGhost}`}
                            title={pageSlot.dirtyTitle ?? pageSlot.dirtyLabel}
                          >
                            {pageSlot.dirty ? pageSlot.dirtyLabel : pageSlot.savedLabel}
                          </span>
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm}`}
                            disabled={!pageSlot.canDiscard || pageSlot.busy === true}
                            title={pageSlot.discardTitle ?? '放弃这一次的改动'}
                            onClick={() => pageActions.run('discard')}
                          >
                            放弃
                          </button>
                          {/*
                           * ★★ **保存默认灰色、能保存才上主色**（2026-10-17 作者：「保存应该
                           *   默认灰色，可以保存才显示，用有颜色的」）。
                           *   旧写法是 `btnPrimary` **恒挂** + `disabled` ⇒ 走的是
                           *   `.btnPrimary:disabled`（底 = `--line-strong`、字仍白）：浅色下白字
                           *   压在 14% 灰上几乎看不见，看着像"还能点"，点下去当然没反应。
                           *   ⇒ 不能保存时**连 `btnPrimary` 都不挂**，回落普通 `.btn` 的禁用态
                           *     （灰字 + 淡描边，一眼是"点不了"）。
                           */}
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm} ${saveOn ? s.btnPrimary : ''}`}
                            disabled={!saveOn}
                            title={saveOn ? (pageSlot.saveTitle ?? '把这一份落盘') : saveWhy}
                            onClick={() => pageActions.run('save')}
                          >
                            {pageSlot.busy === true ? (pageSlot.savingLabel ?? '保存中…') : '保存'}
                          </button>
                        </>
                      ) : (
                        <>
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
                          {/* 同上：能保存才上主色，不能保存是普通灰按钮（见上面那条注释） */}
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm} ${saveOn ? s.btnPrimary : ''}`}
                            disabled={!saveOn}
                            title={saveOn ? (words?.save.dirty.explain ?? undefined) : saveWhy}
                            onClick={() => void save()}
                          >
                            保存
                          </button>
                        </>
                      )}
                    </div>
                  )}
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
              {/* 第三道明示（最小的一道，但它在状态栏上——做别的动作时眼角一直有它） */}
              {sandbox?.enabled === true && (
                <>
                  <span
                    className={s.sbSeg}
                    style={{ color: 'var(--accent-deep)', fontWeight: 600 }}
                    title={`测试模式（沙箱）：${sandbox.sandboxRoot}`}
                  >
                    【测试模式】
                  </span>
                  <span className={`${s.sbBar} ${s.sbMinor}`}>|</span>
                </>
              )}
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
