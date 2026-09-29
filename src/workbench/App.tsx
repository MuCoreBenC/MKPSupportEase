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
import { useDensity } from './useDensity'
import {
  isAppError,
  wb,
  type BookView,
  type Boot,
  type IssueReport,
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
import { ParamDesk } from './views/ParamDesk'
import s from './c14.module.css'

/**
 * 一级导航。前五页是 C14 定稿的顺序；「对比」是过渡期保留项 —— C14 把矩阵并进了
 * 参数台，这一版要等 P2/P3 把参数台搬完才跟着并（P1 里参数台还是旧视角）。
 */
interface NavItem {
  id: PageId | 'compare'
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
  /* 过渡期保留（P3 并进参数台后删除） */
  { id: 'compare', label: '对比', icon: <path d="M8 4v16M16 4v16" /> },
]

type NavId = NavItem['id']

/** 导航宽度的三档（C14 第十七轮）：默认 236，可拖 64（图标档）~ 320（再宽只是挤正文） */
const NAVW = { dft: 236, min: 64, max: 320 }

/** 主选中：机型行（`uid` 为 null = 编基底）或版本行 —— 参数台的工作对象 */
interface Focus {
  machineId: string
  uid: string | null
}

interface UndoEntry {
  label: string
  patches: Patch[]
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
  /** 参数台的 remount 代号：`goto` 过来是「换现场」，要重挂去读新的主选中 */
  const [focusGen, setFocusGen] = useState(0)

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

  /* 首屏四步：boot（上游缺失也能显示数据根）→ 词表 → 整本 → 检查报告。
     ref 挡住 StrictMode 的第二遍（那三条命令全发两次） */
  const booted = useRef(false)
  useEffect(() => {
    if (booted.current) return
    booted.current = true
    void (async () => {
      try {
        const b = await wb.boot()
        setBoot(b)
        if (!b.info) return // 上游缺失：不启动业务
        const [w, bk] = await Promise.all([wb.words(), wb.book()])
        setWords(w)
        setBook(bk)
        // 默认主选中 = 第一台机型的基底：参数台不该在没有任何对象时空着
        setFocus((f) => f ?? { machineId: bk.machines[0]?.id ?? '', uid: null })
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
          const entry = { label, patches: out.inverse }
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

  const undo = useCallback(async () => {
    const top = undoStack[undoStack.length - 1]
    if (!top) return
    const out = await run(`撤销：${top.label}`, top.patches, 'redo')
    if (out) setUndoStack((st) => st.slice(0, -1))
  }, [undoStack, run])

  const redo = useCallback(async () => {
    const top = redoStack[redoStack.length - 1]
    if (!top) return
    const out = await run(`重做：${top.label}`, top.patches, 'undo')
    if (out) setRedoStack((st) => st.slice(0, -1))
  }, [redoStack, run])

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
    if (foc && next === 'params') {
      setFocus({ machineId: foc.machineId, uid: foc.uid })
      setFocusGen((g) => g + 1)
    }
    setPage(next)
  }, [])

  /* 撤销 / 重做 / 保存的键盘入口装在外壳上 —— 撤销不是某几个页面的小功能 */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
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
      case 'compare':
        return '过渡页 —— 对比矩阵在 P3 随参数台一起并进来（C14 把它长在参数台的对照栏）'
      case 'bundles':
        return 'P4 落地 —— 后端 wb_bundles / 套餐引用校验已就位'
      case 'build':
        return report
          ? `阻断 ${report.blocks} · 待办 ${report.todos} · 提示 ${report.hints}${report.hints > 0 && report.blocks === 0 && report.todos === 0 ? ' · 都过了' : ''}`
          : ''
      case 'assets':
        return 'P4 落地 —— 后端 wb_stock / wb_assets / wb_asset_usage 已就位'
    }
  }, [page, book, report])

  /* ---------- 页面渲染（挂载入口只有这一处） ---------- */

  const renderPage = (id: NavId): ReactNode => {
    if (!book || !words) return null
    switch (id) {
      case 'machines':
        return (
          <MachinesPage
            book={book}
            words={words}
            onGoto={goto}
            onApply={async (label, patches) => {
              await run(label, patches, 'undo')
            }}
            onSave={save}
            onBookRefresh={refreshBook}
          />
        )
      case 'params':
        return focus ? (
          <ParamDesk
            key={focusGen}
            machineId={focus.machineId}
            uid={focus.uid}
            where={
              focus.uid
                ? `主选中 ${focus.machineId} / ${book.machines
                    .find((m) => m.id === focus.machineId)
                    ?.versions.find((v) => v.uid === focus.uid)?.name ?? focus.uid}`
                : `主选中 ${focus.machineId} · ${words.level.machine.label}`
            }
            words={words}
            tick={tick}
            onApply={async (label, patches, refresh) => {
              const out = await run(label, patches, 'undo', refresh)
              return { desk: out?.desk ?? null }
            }}
            onCompare={() => setPage('compare')}
          />
        ) : (
          <p className="wb-todo">还没有主选中 —— 去机型与版本页挑一台机型或一个版本。</p>
        )
      case 'compare':
      case 'bundles':
      case 'assets':
      case 'build':
        return (
          <p className="wb-todo">
            {id === 'compare'
              ? '对比矩阵随 P3 回来。'
              : id === 'bundles'
                ? '套餐与菜单视角在 P4 落地（14.1 的 menu 半边）。'
                : id === 'assets'
                  ? '资产库视角在 P4 落地（14.6：资产只从库里挑）。'
                  : '生成与发布视角在 P5 落地（14.2 的闸门按钮含在内）。检查报告的读数已在页头。'}
          </p>
        )
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

      {/* 上游 / 数据根。灯 + 短句 + 附注；窄档只留灯（完整信息在 title 上） */}
      <div
        className={`${s.upstream} ${boot?.info ? '' : s.upstreamOff}`}
        title={
          boot
            ? `配方本 ${boot.roots.workbench}\n交付 ${boot.roots.dist}\n${
                boot.info
                  ? `上游已连接 · 注册表 ${boot.info.registryUpdated}`
                  : '上游未连接（可用环境变量 MKPSE_PRESETS_DIR 指过去）'
              }`
            : '正在读…'
        }
      >
        <span className={s.upDot} data-on={!!boot?.info} aria-hidden />
        <span className={s.upText}>
          <span className={s.upLine}>{boot?.info ? '已连接' : '未连接'}</span>
          <span className={s.upNote}>
            {boot?.info
              ? `上游 · 注册表 ${boot.info.registryUpdated}`
              : '本地副本 · MKPSE_PRESETS_DIR 可改'}
          </span>
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

                {boot?.problem && (
                  <div className="wb-banner" data-tone="danger" style={{ margin: '0 12px' }}>
                    {boot.problem}
                    {boot.detail && <> —— <span className="wb-mono">{boot.detail}</span></>}
                    <br />
                    没有上游工作台不启动业务。可以用环境变量{' '}
                    <span className="wb-mono">MKPSE_PRESETS_DIR</span> 指过去，改完点
                    <button type="button" className="wb-link" onClick={() => location.reload()}>
                      重新加载
                    </button>
                    。
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
