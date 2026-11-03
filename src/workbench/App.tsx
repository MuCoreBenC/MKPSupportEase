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


/* 气泡层：全站唯一一份（客户端 `src/app/App.tsx` 早就挂了，工作台一直漏着） */
import { TipLayer } from '../components/tooltip'
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
  type Patch,
  type Refresh,
  type TxnState,
  type Words,
} from './api'
import { toasts } from './c14/toast'
import { OverlayHostCtx } from './c14/overlayHost'
import type { GotoFocus, PageId } from './c14/types'
import HistoryModal from './c14/HistoryModal'
import SplitterC14 from './c14/SplitterC14'
import { useSplitWidth } from './c14/useSplitWidth'
import MachinesPage from './views/MachinesPage'
import ParamSheetPage from './sheet/ParamSheetPage'
import { sheetWrites } from './sheet/writeQueue'
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
  /*
   * 两根带滑块的横杆 = 调参数。
   *
   * **只有这一个参数台**（2026-11-03 作者：「可以去掉一个参数台了，因为我发现他们
   * 一模一样了」）。此前并存过「参数台」与「参数台（新）」两项：新版是旧页 UI 的
   * 整份复制 + 新内核（本地草稿 + 单飞写队列），版式一字不差，**留的是新版**。
   * 旧页 `views/ParamsPage.tsx` 已删 —— 它那套 `editQueue` 乐观层正是"连点步进器
   * 打出一串 apply_draft"的来源，新版按值判脏、单飞发送，没有第二个真值。
   */
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
const DRAFT_PAGES: ReadonlySet<NavId> = new Set<NavId>([
  'machines',
  'params',
  'assets',
  'build',
])

/** 导航宽度的三档（C14 第十七轮）：默认 236，可拖 64（图标档）~ 320（再宽只是挤正文） */
const NAVW = { dft: 236, min: 64, max: 320, storeKey: 'mkp.wb.nav', iconW: 64 }

/**
 * 导航的三档：`auto`（跟着窗口走，默认）/ `text`（明确要名字，窗口再窄也展开）/
 * `icon`（明确只要图标）。**用户的点击一旦发生就记住**，不再被窗口宽度改回去。
 */
type NavMode = 'auto' | 'text' | 'icon'

/** 跨页定位载荷（C14 语义）：`key` 给套餐/资产页落到某一项（机型页 ④ 关联在用） */
interface Focus {
  machineId: string
  uid: string | null
  key?: string | null
}

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
  /**
   * 统一编辑事务的状态（2026-10-31）。`dirtyCount` 覆盖全部编辑类型；
   * `canUndo/canRedo` 决定状态栏那两颗按钮点不点得动。
   */
  const [txn, setTxn] = useState<TxnState | null>(null)
  /**
   * **「未保存 N」只有一个源**（2026-11-02）。
   *
   * `book.dirtyCount` 与 `txn.dirtyCount` 是两个字段、两次 setState，谁晚一拍
   * 界面就会出现「页头写 0、状态栏写 1」，或者保存完了还挂着数字。
   * 事务状态是唯一真值 —— 每次拿到它都把整本那个数覆盖掉。
   */
  const applyTxn = useCallback((st: TxnState) => {
    setTxn(st)
    setBook((b) =>
      b === null || b.dirtyCount === st.dirtyCount ? b : { ...b, dirtyCount: st.dirtyCount },
    )
  }, [])
  /**
   * 事务状态**只读内存**（后端那份 `Txn` 的条数），几乎不花代价。
   *
   * 为什么不靠各页面上报：工作台有二十多条写命令，页面各自拿自己的返回值刷新，
   * 谁漏一次「上报」就是状态栏少一个数。这里按固定节拍对一下账 —— 页面怎么改，
   * 「未保存改动 / 能不能撤销」都不会落后。
   */
  useEffect(() => {
    let alive = true
    const pull = () => {
      wb.txnState()
        .then((st) => {
          if (alive) applyTxn(st)
        })
        .catch(() => undefined)
    }
    pull()
    const timer = window.setInterval(pull, 800)
    return () => {
      alive = false
      window.clearInterval(timer)
    }
  }, [applyTxn])
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

  const [navW, setNavW] = useSplitWidth('nav', NAVW.dft, NAVW.min, NAVW.max)
  /*
   * **导航折叠**（2026-11-03 作者）：「不管窗口宽度大小，我都能主动把侧边栏折起来」，
   * 而且要记住 —— 以前它只跟着窗口宽度走（< 640 自动变图标列），宽窗口里没有入口。
   *
   * 实现上**不新造一套形态**：宽度给到 64，`.nav` 的容器查询（≤ 88px）自己把
   * 名字换图标、徽标压角 —— 与「拖窄」「窄档」是同一份 CSS，改一处三处都跟着对。
   * `navW` 不动，展开时回到用户拖过的那个宽度。
   */
  const [navMode, setNavMode] = useState<NavMode>(() => {
    try {
      const v = localStorage.getItem(NAVW.storeKey)
      return v === 'icon' || v === 'text' ? v : 'auto'
    } catch {
      return 'auto'
    }
  })
  useEffect(() => {
    try {
      localStorage.setItem(NAVW.storeKey, navMode)
    } catch {
      /* 隐私模式 / 配额满：记不住不算错误 */
    }
  }, [navMode])
  /*
   * **现在是不是图标档**（一个判据，三处用：宽度、把手方向、拖线给不给）。
   *
   *   显式收起                  ⇒ 图标
   *   自动档 + 窗口窄（compact） ⇒ 图标（这就是原来"窄了自动变图标"那档）
   *   自动档 + 被拖到 ≤ 88       ⇒ 图标（容器查询已经把它画成图标了，把手得跟着翻）
   *
   * `text` 档**压过窗口窄**：作者 2026-11-03「窗口比较小的时候，也要能把它展开」。
   */
  const navIcon =
    navMode === 'icon' ||
    (navMode === 'auto' && density === 'compact') ||
    /* 拖到最窄：容器查询已经把它画成图标列了 —— 不论哪一档，把手都得跟着翻 */
    navW <= NAVW.min + 24
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
    /* 事务状态与整本同源：谁改了书，谁就得把「未保存改动 / 可撤销」一并刷新 */
    wb.txnState()
      .then(setTxn)
      .catch(() => undefined)
    refreshReport()
  }, [refreshReport])

  /**
   * 盘上的 `presets/` 被外部改过没有（2026-10-20，作者：「像你这种外部改的，
   * 我这个软件完全没有看到改的呀」）。
   *
   * 会话把预设**只读一次**（`wb_reload` 才重读），所以外部改完切回来，界面还是旧的。
   * 这里只**发现并提示**，不自动重载：正开着的草稿是按旧定义起稿的，悄悄换掉
   * 定义再存回去，会把 `sectionId` / 弃用这些格写错。
   */
  const [sourceStale, setSourceStale] = useState(false)
  const checkSourceStale = useCallback(() => {
    wb.presetsChanged()
      .then(setSourceStale)
      .catch(() => undefined)
  }, [])

  /** 当前页面的 ref：focus 那条监听只装一次，不能跟着每次切页重挂 */
  const pageRef = useRef(page)
  pageRef.current = page

  /**
   * **外面的文件改过了**（2026-10-19，作者：「我直接改了那些模型，它这里面能不能知道」）。
   *
   * 工作台读的是盘上的真字节，但读数只在写操作 / 进页时取 —— 作者在编辑器里改完
   * 3MF 再切回来，屏幕还是旧的。这里在窗口重新获得焦点时重取一遍「会变的那几样」：
   * 检查报告（交付目录对不对得上、导航徽标）+ `tick`（生成页交付清单 / 资产库列表）。
   *
   * ★ 2026-10-20 补上 `wb_book()`：生成与发布页那 9 行状态来自 `book.buildRows`，
   *   它以前**不在**这条刷新链里 —— 于是外面显示「已生成」、明细显示「待更新」，
   *   要重启才对齐。现在停在生成页时一起重取（别的页仍然只取报告，省一次全本推导）。
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
        if (pageRef.current === 'build') refreshBook()
        else refreshReport()
        checkSourceStale()
        setTick((n) => n + 1)
      }, 300)
    }
    window.addEventListener('focus', onFocus)
    return () => {
      window.removeEventListener('focus', onFocus)
      if (t !== null) window.clearTimeout(t)
    }
  }, [checkSourceStale, refreshBook, refreshReport])

  /**
   * 进这两页时也重取一次：它们是「盘上现在是什么样」的读数页
   * （生成与发布看交付目录，资产库看载荷字节）。
   *
   * 生成页还要**整本**：那 9 行的 `BuildState` 不住在交付清单里。
   */
  useEffect(() => {
    if (page === 'build') {
      refreshBook()
      checkSourceStale()
      setTick((n) => n + 1)
      return
    }
    if (page === 'assets') {
      refreshReport()
      checkSourceStale()
      setTick((n) => n + 1)
    }
  }, [checkSourceStale, page, refreshBook, refreshReport])

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

  /**
   * `presets/` 改过之后（「重新加载」= 后端重开一次会话）：换 `Boot`，并把
   * 整本与检查报告重取一遍 —— 预设数据变了会改这两处读数
   */
  const reloadBoot = useCallback(
    (b: Boot) => {
      setBoot(b)
      setSourceStale(false)
      refreshBook()
    },
    [refreshBook],
  )

  /**
   * 走唯一写入口。`where` 说这次结果往哪个栈压（正向压撤销、撤销压重做、重做压撤销）
   * —— 三种情形一条路径，分三份写「能不能重做」就会有三种写法。
   */
  const run = useCallback(
    async (
      label: string,
      patches: Patch[],
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
        /*
         * 2026-10-31：撤销不再由前端记账 —— 后端已把这次编辑记成一条事务。
         *
         * ★ **返回的那份 view 里 `dirtyCount` 慢一笔**（2026-11-02）：事务是在命令
         * 业务体**跑完之后**才记的（`run_ctx` 拍 before、调 `f`、再 `record`），
         * 所以 `view_of` 算账时这一笔还没进栈。页头于是出现「未保存（0）」而状态栏
         * 写着「未保存改动（1）」—— 同一屏两个数。
         *
         * 账以 `txn` 为准（它就是那唯一一份），拿到之后回填给 `book`，
         * 别让两个数各说各话。
         */
        wb.txnState()
          .then(applyTxn)
          .catch(() => undefined)
        if (!quiet) setTick((n) => n + 1)
        if (quiet) refreshReportSoon()
        else refreshReport()
        return out
      } catch (e) {
        fail(e)
        return null
      } finally {
        if (!quiet) setBusy(false)
      }
    },
    [applyTxn, fail, refreshReport, refreshReportSoon],
  )

  /**
   * 编辑落进草稿之后，把**服务器侧那条栈**的状态拉一次。
   *
   * 2026-10-31：撤销不再由前端记账 —— 后端把每次结构编辑也记成一条事务
   * （`EDIT_COMMANDS` 里登记的命令），这里只需要把事务状态刷一下。
   * 参数台两种编辑都要它：改定义（`pushMeta`）与改分组（`onEditApplied`）。
   */
  const refreshTxn = useCallback(() => {
    wb.txnState()
      .then(setTxn)
      .catch(() => undefined)
  }, [])

  /** 参数台「编辑定义」保存成功后交上来的一条 */
  const pushMeta = useCallback(
    (m: MetaApplied) => {
      void m
      refreshTxn()
    },
    [refreshTxn],
  )

  /**
   * 撤销 / 重做：**服务器侧一条栈**（2026-10-31 统一编辑事务）。
   *
   * 以前这里是「把反向 patch 再发一次」——值编辑能这么玩，结构编辑（新增版本、
   * 删套餐）没有反向 patch。现在一律交回后端：它按影子树里那条 op 的 `before`
   * 写回去，字段 / 尺寸 / 结构 / 参数值走的是同一条路径。
   */
  /* 两个都返回 `boolean`（真撤成了没有）：未保存改动框里的「撤销这一笔」要据此
     决定要不要重取它自己那份 diff —— 没撤成还重取一次，等于把同一份账读两遍。 */
  const undo = useCallback(async (): Promise<boolean> => {
    /* 飞在空中的值写先落完 —— 否则撤销撤的是"落完之前的那一半" */
    await settleAll()
    /* ★ 参数台 v2 那一条队列也得先排空（2026-11-03）。它不走 `editQueue`：
       还在队列里的那几笔**还没进事务栈**，直接撤会撤到更早的一笔，而它们随后
       照样落进影子树 —— 用户看到的就是「撤了，一秒后又自己弹回来」。
       `drop()` 只清草稿，救不了已经在路上的那一条。 */
    await sheetWrites.settle().catch(() => undefined)
    editQueue.clear()
    setBusy(true)
    try {
      const st = await wb.txnUndo()
      applyTxn(st)
      /* 后端那一笔被撤掉了，参数台 v2 手里的草稿就不再是真话 —— 让它丢掉 */
      sheetWrites.drop()
      await refreshBook()
      /*
       * ★ **必须让参数台那一屏重取自己的 desk**（2026-11-03 作者：按 Cmd+Z 撤不掉）。
       *
       * `refreshBook()` 只刷外壳那份 `book`（整本 / 事务状态），而参数台 v2 的每一格
       * 读的是它自己 `wb.desk(...)` 拿回来的那一份。`drop()` 又把本地草稿清了 ⇒
       * 界面回落到**撤销之前**的 desk，看起来就是"撤了等于没撤"，而且接着一碰输入框
       * 就把那个旧数又写回去（作者日志里 `3.95 → 3.9` 紧跟在一笔撤销后面）。
       * `discard` 一直是对的（它 `setTick`），撤销 / 重做漏了这一格。
       */
      setTick((n) => n + 1)
      refreshReport()
      return true
    } catch (e) {
      fail(e)
      return false
    } finally {
      setBusy(false)
    }
  }, [applyTxn, fail, refreshBook, refreshReport])

  const redo = useCallback(async (): Promise<boolean> => {
    await settleAll()
    /* 同 `undo`：队列里没落的写先发完，否则它们会落在重做之后 */
    await sheetWrites.settle().catch(() => undefined)
    editQueue.clear()
    setBusy(true)
    try {
      const st = await wb.txnRedo()
      applyTxn(st)
      sheetWrites.drop()
      await refreshBook()
      /* 同 `undo`：重做也改了影子树，参数台那一屏必须重取 desk */
      setTick((n) => n + 1)
      refreshReport()
      return true
    } catch (e) {
      fail(e)
      return false
    } finally {
      setBusy(false)
    }
  }, [applyTxn, fail, refreshBook, refreshReport])

  const save = useCallback(async (): Promise<boolean> => {
    if (busy) return false
    /* 飞在空中的值写先落完：`dirtyCount` 得是落完之后的账 ——
       否则"刚点完步进器马上点保存"会静默丢掉那一笔（读的是旧整本） */
    await settleAll()
    /* ★ 参数台 v2 那一页的写入队列也要排空（它不走 `editQueue`）。
       排不空就**拒绝保存**：带着未确认的编辑去落盘等于把那几笔抹掉。 */
    try {
      await sheetWrites.settle()
    } catch (e) {
      fail(e)
      return false
    }
    /* 现问一次（别信上一次轮询的那份）：刚改完就按 Cmd+S 时它可能还没刷新 */
    const fresh = await wb.txnState()
    applyTxn(fresh)
    const cur = bookRef.current
    if (!fresh.dirty && (cur?.dirtyCount ?? 0) === 0) return false
    /* 保存 = 提交点：本地草稿层整层丢掉，显示交回写进仓库的那份 */
    editQueue.clear()
    setBusy(true)
    try {
      const out = await wb.save()
      setBook(out.view)
      /* 保存成功 ⇒ 参数台 v2 的本地草稿作废（它们已经成了新的保存值） */
      sheetWrites.drop()
      applyTxn(await wb.txnState())
      setTick((n) => n + 1)
      refreshReport()
      toasts.push('已保存 —— 改动已写进仓库文件')
      return true
    } catch (e) {
      fail(e)
      return false
    } finally {
      setBusy(false)
    }
  }, [applyTxn, busy, fail, refreshReport])

  const discard = useCallback(async () => {
    if (busy) return
    /* 同 `save`：先把在飞的写落完再判脏 —— 跳到一半的改动也得算进来 */
    await settleAll()
    /* 参数台 v2 的队列：放弃不因"没发完"而拒绝，但也别让它们落在 `wb_discard` 之后 */
    await sheetWrites.settle().catch(() => undefined)
    sheetWrites.drop()
    const fresh = await wb.txnState()
    applyTxn(fresh)
    const cur = bookRef.current
    if (!fresh.dirty && (cur?.dirtyCount ?? 0) === 0) return
    /* 放弃 = 整条草稿丢：影子树整个拉回磁盘原状 */
    editQueue.clear()
    setBusy(true)
    try {
      setBook(await wb.discard())
      applyTxn(await wb.txnState())
      setTick((n) => n + 1)
      refreshReport()
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }, [applyTxn, busy, fail, refreshReport])

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
        /* ★ 当前页若接管了页头那一槽，**Cmd+S 必须跟它走**（2026-11-02）。
           参数台 v2 保存前要先排空写入队列；绕过页面直接调外壳的 `save()` 会
           带着还没落完的编辑去落盘 —— 那一笔会落在 `wb_save` 之后，
           表现就是"点了保存，右上角还写着未保存"。 */
        if (pageActions.viewFor(page) !== null) pageActions.run('save')
        else void save()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [undo, redo, save, page])

  /* 草稿不干净时，刷新前用浏览器原生弹窗问一嘴（自绘模态框弹出来时刷新已经走了） */
  /* 脏 = 整本里记的（值草稿 + 事务）+ 事务自己那份。两处都看，切页瞬间也不会闪「已保存」 */
  const dirty = (book?.dirtyCount ?? 0) > 0 || txn?.dirty === true

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
        return '保存值由后端按三层算好 —— 徽章只在「现在」与它不同时出现'
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
        /* 参数台只剩这一个（2026-11-03）。它是旧页 UI 的整份复制 + 新内核
           「本地草稿 + 单飞写队列」；保存前的排空经 `sheetWrites` 挂到外壳（见 `save`） */
        return (
          <ParamSheetPage
            key={focusGen}
            book={book}
            words={words}
            initialFocus={focus}
            tick={tick}
            dirty={dirty}
            onApply={async (label, patches, refresh, opts) => {
              const out = await run(label, patches, refresh, opts)
              return { ok: out !== null, desk: out?.desk ?? null, matrix: out?.matrix ?? null }
            }}
            onMetaApplied={pushMeta}
            onEditApplied={refreshTxn}
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
              await run(label, patches, undefined, opts)
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
      {/*
        折叠把手（2026-11-03 作者）：「不管窗口宽度大小，我都能主动折叠 / 展开」。
        **方向跟的是"现在到底是不是图标档"**，不是那个开关变量 —— 以前把栏拖到最窄，
        屏幕上已经变成图标列了，箭头还指向左（作者报的第一条）。
      */}
      {density !== 'mini' && (
        <button
          type="button"
          className={s.navCollapse}
          data-icon={navIcon}
          aria-expanded={!navIcon}
          data-tip={
            navIcon
              ? '展开侧边栏（窗口小也能展开）'
              : '收起侧边栏 —— 只留图标（记着我这次的选择）'
          }
          onClick={() => {
            if (navIcon) {
              /* 展开：栏被拖到 ≤ 88 时一并回到默认宽，否则"展开了"看不出变化 */
              if (navW <= NAVW.min + 24) setNavW(NAVW.dft)
              setNavMode('text')
            } else {
              setNavMode('icon')
            }
          }}
        >
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
          >
            <path d="m15 18-6-6 6-6" />
          </svg>
        </button>
      )}
      <div className={s.navList}>
        {NAV.map((it) => {
          const bd = badgeOf(it.badge)
          return (
            <button
              key={it.id}
              type="button"
              className={`${s.navItem} ${page === it.id ? s.navItemOn : ''}`}
              data-tip={it.label}
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
                  data-tip={bd.block ? '有阻断没处理完' : '有待处理的项，但不挡生成'}
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
        data-tip={
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

      {!navIcon && density !== 'mini' && (
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
      /* `text` 档要压过窄档那套图标规则（见 css 里 `.shell[data-nav='text']` 那一段） */
      data-nav={navIcon ? 'icon' : 'text'}
      /* 测试模式整窗换一层色（`c14tokens.css` 只改那几个变量，C14 的内衬全跟着走） */
      style={
        {
          '--nav-w': `${navIcon ? NAVW.iconW : navW}px`,
          '--nav-w-text': `${navW}px`,
        } as CSSProperties
      }
    >
      {/*
        * 严重错误横幅（z 60，盖在页头之上 ⇒ **会挡住保存按钮**）。
        *
        * ★ 必须有一颗「关闭」（2026-11-02 作者：「它永远不消失，除非我重新打开软件，
        *   不能点穿」）。`fatal` 以前**只设不清**：任何一次性错误都会变成一块永久
        *   挡路的板子。那些"其实没出错"的情形（点了没有改动的放弃）现在已在后端
        *   改成幂等，但**真出错时人也得能把它关掉**继续干活。
        */}
      {fatal && (
        <div
          className="wb-banner"
          data-tone="danger"
          style={{ position: 'absolute', top: 8, left: 12, right: 12, zIndex: 60 }}
        >
          <button
            type="button"
            className="wb-link"
            style={{ float: 'right', marginLeft: 12 }}
            onClick={() => setFatal(null)}
          >
            关闭
          </button>
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
                        data-tip={it.label}
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
                            data-tip={bd.block ? '有阻断没处理完' : '有待处理的项，但不挡生成'}
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
                            data-tip={pageSlot.dirtyTitle ?? pageSlot.dirtyLabel}
                          >
                            {pageSlot.dirty ? pageSlot.dirtyLabel : pageSlot.savedLabel}
                          </span>
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm}`}
                            disabled={!pageSlot.canDiscard || pageSlot.busy === true}
                            data-tip={pageSlot.discardTitle ?? '放弃这一次的改动'}
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
                            data-tip={saveOn ? (pageSlot.saveTitle ?? '把这一份落盘') : saveWhy}
                            onClick={() => pageActions.run('save')}
                          >
                            {pageSlot.busy === true ? (pageSlot.savingLabel ?? '保存中…') : '保存'}
                          </button>
                        </>
                      ) : (
                        /*
                         * **一颗按钮**（2026-11-02 作者：「外面的按钮就换一下文案，变成一个，
                         * 就是查看改动这种的吧」）。
                         *
                         * 原来这里是「未保存 · 放弃 · 保存」三颗：改完想知道自己改了什么，
                         * 得先记住去状态栏找「未保存改动」；看过之后又得回到页头再按保存。
                         * 现在三件事收进同一个框：**看 diff → 在框里放弃或保存**。
                         *
                         * 落盘边界没有变（框里的「保存」调的就是下面这条 `save()`）——
                         * 变的是入口只有一个，且"看"排在"决定"之前。
                         */
                        <button
                          type="button"
                          /*
                           * 常年是**普通灰按钮**：它现在只负责"看"，不负责落盘 ——
                           * 真正的保存落在框里那颗（能保存才上主色）。页头这颗上主色
                           * 会让人以为"点它就是保存"，而它其实只是打开一个框。
                           */
                          className={`${s.btn} ${s.btnSm}`}
                          data-tip={
                            dirty
                              ? `看看这 ${book?.dirtyCount ?? 0} 处改动 —— 逐行 diff，看完在框里放弃或保存`
                              : (words?.save.saved.explain ?? '草稿和磁盘那一份一致')
                          }
                          onClick={() => setHistoryOpen(true)}
                        >
                          {words ? words.save[dirty ? 'dirty' : 'saved'].label : '—'}
                          {dirty ? `（${book?.dirtyCount ?? 0}）` : ''} · 查看改动
                        </button>
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

                {/*
                  数据被外部改过（2026-10-20）。会话只读一次 `presets/`，所以要有一条路
                  让人知道"屏幕这份不是盘上那份"—— 重载仍然是人的动作，见 checkSourceStale。
                */}
                {sourceStale && (
                  <div className="wb-banner" data-tone="warn" style={{ margin: '0 12px' }}>
                    盘上的
                    <span className="wb-mono"> presets/ </span>
                    在这个窗口外面改过了 —— 现在显示的仍是打开时读进来的那一份（参数名、可选项、
                    分组都可能是旧的）。点
                    <button
                      type="button"
                      className="wb-link"
                      onClick={() => {
                        /* 有未保存草稿时后端会拒绝（W_WB_RELOAD_DRAFTS_001）——
                           把那条话说出来，让人自己决定，不静默丢东西。 */
                        wb.reload()
                          .then(reloadBoot)
                          .catch((e) => {
                            if (isAppError(e) && e.messageCode === 'W_WB_RELOAD_DRAFTS_001') {
                              if (window.confirm(`${e.message}\n\n确定放弃这些未保存的改动，按磁盘上的 presets/ 重新加载吗？`)) {
                                wb.reload(true).then(reloadBoot).catch(fail)
                              }
                              return
                            }
                            fail(e)
                          })
                      }}
                    >
                      重新加载
                    </button>
                    读进来，不用重启。{' '}
                    <strong>有未保存的改动时会先问你一句</strong>，不会悄悄丢掉。
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
                data-tip={(words && book ? words.artifact[book.artifact].explain : undefined) ?? undefined}
              >
                {words && book ? words.artifact[book.artifact].label : '—'}
              </span>
              <span className={`${s.sbBar} ${s.sbMinor}`}>|</span>
              <span
                className={`${s.sbSeg} ${s.sbMinor}`}
                data-tip={(words && book ? words.snapshot[book.snapshot].explain : undefined) ?? undefined}
              >
                {words && book ? words.snapshot[book.snapshot].label : '—'}
              </span>
              <span className={s.sbOps}>
                <button
                  type="button"
                  className={s.sbBtn}
                  disabled={txn?.canUndo !== true || busy}
                  data-tip={
                    txn !== null && txn.ops.length > 0
                      ? `撤销：${txn.ops[txn.ops.length - 1].label}`
                      : (words?.disabled.nothingToUndo ?? '没有可撤销的改动')
                  }
                  onClick={() => void undo()}
                >
                  撤销
                </button>
                <button
                  type="button"
                  className={s.sbBtn}
                  disabled={txn?.canRedo !== true || busy}
                  data-tip={
                    txn?.canRedo === true
                      ? '重做上一条撤销'
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
              {/*
                * **刚跑完那一笔的请求号**（2026-11-02）。
                *
                * 一次点击会打七八行日志（写队列 → IPC → 值进影子树 → 参数定义已改 →
                * 派生 → 事务），每行都带同一个 `req=`。把它摊在状态栏上，作者复制去
                * 日志里 grep 就是完整一条线 —— 不用再拿时间戳一个个猜。
                *
                * 取走即清（后端 `take_receipt`）：同一个号不会在一屏里重复出现，
                * 所以这里不用做"多久之后隐藏"。
                */}
              {txn?.receipt != null && (
                <>
                  <span className={`${s.sbBar} ${s.sbMinor}`}>|</span>
                  <span
                    className={`${s.sbSeg} ${s.sbMinor}`}
                    data-tip={
                      `日志里搜 ${txn.receipt.req} 就是这个动作的全部行` +
                      `（${txn.receipt.command}${txn.receipt.ok ? '' : '，失败了'}）`
                    }
                    style={{ fontFamily: 'ui-monospace, monospace', userSelect: 'text' }}
                  >
                    {txn.receipt.req}
                  </span>
                </>
              )}
            </div>

            {/*
              框里给「放弃改动 / 保存」：这两颗调的就是外壳这条 `save()` / `discard()`，
              落盘边界仍然只有一处。保存成功后把框关掉（改完了、也没什么可看的了）；
              放弃同理 —— 留在框里只会显示一堆已经不存在的变化。
            */}
            <HistoryModal
              open={historyOpen}
              onClose={() => setHistoryOpen(false)}
              canSave={saveOn}
              canDiscard={dirty && !busy}
              busy={busy}
              onDiscard={() => {
                setHistoryOpen(false)
                void discard()
              }}
              onSave={() => {
                void save().then((ok) => {
                  if (ok) setHistoryOpen(false)
                })
              }}
              /* 撤销/重做走外壳这一条 —— 框里发完还要让外壳重取整本，状态栏才不会说谎 */
              canUndo={txn?.canUndo === true}
              canRedo={txn?.canRedo === true}
              onUndo={undo}
              onRedo={redo}
            />

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

        {/*
         * **气泡层**（2026-11-03）。工作台以前**没有挂** `<TipLayer>` —— 它只装在
         * 客户端 `src/app/App.tsx` 里，于是工作台里所有 `data-tip` 都是**死属性**：
         * 悬停什么都不出（作者：「我悬停了也没有出现字呀」）。
         * 挂上之后沿用全站那一套：跟光标、出框平移、`z-index 96`、只认 `data-tip`。
         */}
        <TipLayer />
      </OverlayHostCtx>
    </div>
  )
}
