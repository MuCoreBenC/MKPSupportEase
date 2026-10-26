/*
 * BBS 预设查看器 —— 页面装配。
 *
 * 搬自 `machine-motion/entries/bbs-preset`（那一页是纯静态 HTML + 908 行原生 JS）。
 * 这里只负责装配与交互，算法全在旁边那几个文件里：
 *
 *   bbsMerge        继承链、合并、改动判定（纯函数，逐行译自上游）
 *   bbsSource       清单归一、机型/喷嘴识别、分组（同上）
 *   bbsToggleRules  86 条条件显隐规则（译自 ConfigManipulation.cpp）
 *   bbsPanel        版面 + 值 → 页/组/行（上游是边算边建 DOM，这里先算清再画）
 *   useBbsData      取数与缓存（本机目录端点 + `public/bbs/` 那四份元数据）
 *   useBbsDelivered 交付面：我们自己的切片器配置（预设页下载到本机的那几份）——
 *                   本页把它们读出来灌进清单，左抽屉那个「MKP」档就是它们
 *   useBbsPreset    选中那一份摊开来是什么样
 *
 * # 主题为什么自带一套变量
 *
 * 上游是一页独立的深色界面，两套皮肤（深/浅）的取值是照 BBS 的 StateColor 抄的。
 * 本项目的 tokens.css 只有浅色一套。把 BBS 那两套塞进全局 token 会污染别的页面，
 * 所以**整套 --bbs-* 变量都定义在这一页的根节点上**（见 PageBbs.module.css），
 * 切主题只切根节点上的 data-theme，一行 DOM 都不用重建。
 *
 * # 这一页不写盘
 *
 * 改值、重置、还原都只动内存里的副本，刷新即还原 —— 上游也是这样。
 * `tools/dev-server/bbsFs.mjs` 那个端点是**只读**的，没有写回 BBS 的口子。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { STORAGE } from '../../api/storageKeys'
import { useT } from '../../i18n'
import { useStickyState } from '../../hooks/useStickyState'
import { useIsDark } from '../state/appState'
import type { Density } from '../../hooks/useDensity'
import BbsGroup from './BbsGroup'
import BbsPresetDrawer from './BbsPresetDrawer'
import BbsRail from './BbsRail'
import BbsSearchBox from './BbsSearchBox'
import BbsTabs from './BbsTabs'
import { buildBbsPanel, resolveActivePage } from './bbsPanel'
import { normalizeQuery } from './bbsSearch'
import { MODEL_ALL } from './bbsSource'
import type { BbsPickSource } from './bbsSource'
import { coverage } from './bbsToggleRules'
import Icon from '../../components/Icon'
import { useSessionState } from '../shared/useSessionState'
import { useBbsData } from './useBbsData'
import type { BbsSlicerRow } from './useBbsData'
import { useBbsDelivered } from './useBbsDelivered'
import { useBbsDrawer } from './useBbsDrawer'
import { useBbsRail } from './useBbsRail'
import { useBbsPreset } from './useBbsPreset'
import WindowGadgets from './WindowGadgets'
import type { BbsViewMode } from './bbsTypes'
import s from './PageBbs.module.css'

interface Props {
  density: Density
  /** 预设页右键带过来的目标文件名；null = 直接从顶栏点进来的 */
  pending: { name: string; nonce: number } | null
  /**
   * 「MKP 工艺」档里那几份**还没下载**的，点一下该去哪儿（2026-10-13 作者 §7）：
   * 外壳给的动作 = 跳去预设页的「切片器配置 → 云端」把那一份取回来。
   *
   * `null` = 不留这个动作（那时列表里那几份照样灰着列，只是点了不做事）。
   */
  onNeedDownload: (() => void) | null
}

const RULES = coverage()
/** 浮层判据：窗口窄于这个就走浮层（与报告页 `PageReport` 的 880 同一个数） */
const NARROW_W = 880
/*
 * 顶条**一直是两行**（2026-10-13 作者裁决）：
 *
 *   第一行 = 工具条（☰ · 预设名 · 详情）
 *   第二行 = 页签 + 搜参数
 *
 * 中间试过"宽窗合一行"（`data-rows='one'`，阈值一路从 880 调到 640）—— 作者最终不要它：
 * 「要不然它那上面就默认就是两行算了吧，它一行的话总是变成两行，我感觉它缩小到一定程度
 * 就变成两行，我就觉得不太舒服。**默认就两行吧**。」
 * 所以那个分支连同 `MERGED_MIN` 一起删了 —— 留着就是一条永远走不到的路。
 */

/*
 * 标签列宽度的连续曲线（2026-10-13）。
 *
 * 原来这是两条按密度档切的 CSS 规则（compact 180 / mini 44%）⇒ 窗口拖过
 * Compact/Wide 那道线（作者说的"1080"）时整列值**横着跳一下**。
 * 作者：「我变化尺寸的时候，它这个参数还会移动，一瞬间的，好像在 1080 的时候跳了，
 * 突然从左边跳到右边，说不上的感觉，不是很喜欢。」
 *
 * 所以照 tokens.css 那条口径（"量"连续、只有"结构"才按档）改成页面宽的线性函数：
 *   1400 及以上 → 236（现在是基线那一档）
 *   880（浮层那道线）→ 170（跟原来 compact 的 180 差不多，差的那 10px 是留给"撤销位 + 控件"）
 * 两端之间线性；比 880 还窄就不再缩（再缩值列就贴到标签上了）。
 */
const LABEL_MAX = 236
const LABEL_MIN = 170
const LABEL_FULL = 1400
const LABEL_SLOPE = (LABEL_MAX - LABEL_MIN) / (LABEL_FULL - 880)
const labelWidth = (pageW: number) => Math.round(
  Math.max(LABEL_MIN, Math.min(LABEL_MAX, LABEL_MAX - (LABEL_FULL - pageW) * LABEL_SLOPE)),
)
/* 规则说明那句在 2026-10-26 搬进字典（`bbs.rulesNote`）：模块级常量会被求值一次冻住，
   现在由组件里用 `t(..., {rules, keys})` 现取 */

export default function PageBbs({ density, pending, onNeedDownload }: Props) {
  const tr = useT()
  const data = useBbsData()
  /*
   * 看哪一边（2026-10-13）：`mkp` = 我们自己的切片器配置，`bbs` = 本机 BBS 目录里的。
   * 默认停在 **MKP** —— 作者：「最优先的是显示我们自己的 MKP 的……显示系统的不是最优先的」。
   * 与抽屉开合、筛选档位同一档记忆（只记这一次会话）。
   */
  const [source, setSource] = useSessionState<BbsPickSource>('bbs.source', 'mkp')
  const preset = useBbsPreset(data, source)
  /* 交付面（`assets/bbs/…`）：第三圈起，这一页的产品配置从下载区来，不再是随包副本 */
  const delivered = useBbsDelivered()

  /*
   * 我们自己的切片器配置 → 清单（2026-10-13）。
   *
   * 读的是"预设页下载到本机的那几份"（`downloaded`；真机走 IPC，浏览器预览里给的是
   * 演示正文，见 `mockServer/slicerPreset.ts`）。整批交给清单（`scope = 'mkp'`），
   * 左抽屉那个「MKP」档就是它们 —— 于是继承链、基准、改动判定全走与 BBS 预设同一条路。
   *
   * ★ 同一天第二轮：**目录里登记着、盘上还没有的那几份也进清单**（`listed` 里减掉
   *   `downloaded`）—— 它们没有正文，灰着列在 MKP 组里，点了去预设页取回来
   *   （作者 §7：「把未下载的 MKP 工艺也列出来（灰着、点了去预设页下载）」）。
   *   一份读不出来就跳过这一份：**一份读不到不该拦着别的几份**（预览里 MKP 的 toml 就是抛的）。
   */
  const setSlicerPresets = data.setSlicerPresets
  /* 先解构再进依赖：整个 `delivered` 每帧都是新对象，把它放进依赖会让这一段每渲染
     都重读一遍盘（一次读九份，不能这么花） */
  const mkpListed = delivered.listed
  const mkpFiles = delivered.downloaded
  const readMkp = delivered.read
  useEffect(() => {
    let alive = true
    void (async () => {
      const have = new Set(mkpFiles.map((f) => f.fileName))
      const rows: BbsSlicerRow[] = []
      for (const f of mkpListed) {
        if (!have.has(f.fileName)) {
          /* 还没下载：只带目录登记（机型与喷嘴），没有正文 */
          rows.push({ fileName: f.fileName, machineId: f.machineId, path: f.path })
          continue
        }
        try {
          const doc = await readMkp(f.fileName)
          if (!alive) return
          /* 机型与落点（目录登记）一起带下去：认机型用它，不看 doc 里的继承链 */
          rows.push({ fileName: f.fileName, machineId: f.machineId, path: f.path, doc })
        } catch {
          if (!alive) return
        }
      }
      if (alive) setSlicerPresets(rows)
    })()
    return () => { alive = false }
  }, [mkpFiles, mkpListed, readMkp, setSlicerPresets])

  /* 已经复制进切片器目录的那几份（文件名）：本机工艺档里出现同名的那一份要标出来 ——
     `copy_to_slicer` 之后两边各有一份内容相同的（作者 §7） */
  const copiedFiles = delivered.copied
  const fromUs = useMemo(() => new Set(copiedFiles), [copiedFiles])

  /* 落 localStorage 的偏好：只看档位（主题那一格 2026-10-12 退役，见下）。键名收在 `src/api/storageKeys.ts` */
  const [viewMode, setViewMode] = useStickyState<BbsViewMode>(STORAGE.clientBbsView, 'all')
  /*
   * ★ 2026-10-12（深色模式那一刀）：**这一页的深浅跟着应用主题走**。
   *
   * 以前它自带一颗「深色 / 明亮」开关（落 `clientBbsTheme`）＋ 地址参数 `?theme=`，
   * 于是同一屏上可能出现两套深浅（应用浅色、这页深色）。现在真值只有一个：
   * `useIsDark()` —— 用户选的那一档加上系统深浅（见 `app/state/appState.ts`）。
   *
   * 皮还是 BBS 自己那两套（照上游抄的 `--bbs-*`，见 `PageBbs.module.css` 模块头）——
   * 那是"复刻 BBS 界面"这一页该有的样子；改的只是**由谁来决定用哪一套**。
   */
  const light = !useIsDark()
  /* 只记「这次会话看到哪」的：筛选档位、搜索词、当前页、抽屉开合 */
  const [target, setTarget] = useSessionState('bbs.target', MODEL_ALL)
  const [query, setQuery] = useSessionState('bbs.query', '')
  const [page, setPage] = useSessionState<string | null>('bbs.page', null)
  const [drawerOpen, setDrawerOpen] = useSessionState('bbs.drawer', true)
  /*
   * 搜参数的词。**换预设、换档位都保留** —— 「同一个参数在另一台机器上是多少」
   * 是这一页最常见的用法，每次换预设都被清空会很烦。
   */
  const [paramQuery, setParamQuery] = useSessionState('bbs.paramQuery', '')

  /*
   * 抽屉的两种形态：**由密度档 + 页面实际宽度自动决定**（2026-10-13 作者口径）。
   *
   * 只看宽度会漏：mini 档内容缩得最小、抽屉常驻会把参数挤没了 —— mini 一律浮层；
   * 但 compact 档在 900×640 这种正常窗口里**就该并排**（作者实测裁断：这个尺寸
   * 不用悬浮）。所以并排 = 页面实际 ≥ 880px（与报告页滑盖同一个数）且非 mini 档。
   * 量**页面自身**而不是 `window.innerWidth`：外壳是模拟窗口，浏览器窗口再宽、
   * 模拟窗只有 600~900 宽时页面拿到的也就那么宽 —— 报告页量的也是容器。
   * 手动开关退役：换档、换窗口大小都自动换形态。
   */
  const pageRef = useRef<HTMLDivElement>(null)
  /*
   * 量出来的页面宽度（0 = 还没量到）。两个用处：
   *   · `narrow`（< 880）→ 左抽屉改浮层（与报告页滑盖同一个数）；
   *   · `--bbs-label-w` → 标签列宽度的连续曲线（见上面 `labelWidth`）。
   */
  const [pageW, setPageW] = useState(0)
  useEffect(() => {
    const el = pageRef.current
    if (!el) return
    const ro = new ResizeObserver(() => setPageW(el.clientWidth))
    ro.observe(el)
    setPageW(el.clientWidth)
    return () => ro.disconnect()
  }, [])
  const narrow = pageW < NARROW_W
  /*
   * 右侧「详情」栏的开合（声明挪在左抽屉形态之前 —— 下面浮不浮的判据要读它）：
   * **默认收起**（2026-10-17 作者：「默认左侧展开，右侧收起」）。
   * 口径沿革见下面 `openRail` 那一段注释。
   */
  const [railOpen, setRailOpen] = useSessionState('bbs.rail', false)
  /*
   * 左抽屉浮不浮（2026-10-17 作者口径）：**浮只在「右栏开着」的前提下发生** ——
   * 「右侧的没展开的时候，左侧没必要悬浮」「小尺寸的时候，右侧收起的时候，左侧的不要悬浮」。
   * 右栏收着时整幅宽都是参数区的，左边并排就够了。
   * mini 档 / 页面窄于 880 仍会改浮层（老判据不动），但都以右栏开着为前提。
   */
  const drawerMode: 'side' | 'float' =
    railOpen && (density === 'mini' || narrow) ? 'float' : 'side'
  /** 宽度、拖动状态、热区那一套 props */
  const sizing = useBbsDrawer()

  /*
   * ============ 右侧「详情」栏（2026-10-13 那一档：底条整条退场） ============
   *
   * 原来贴在窗口下缘那条状态条（统计 / 继承链 / 说明）搬到了右边一栏。作者点头的那一版
   * 是抽卡里的 04 —— lab/bbs-page（那一页 04 号卡就是这一版的可点原型）。
   *
   * 两件事与左抽屉共用同一套口径，别各写一份：
   *   · 宽度：`useBbsRail` → 全仓共用的 `shared/useDrawerWidth`（拖宽 + 三态吸附 + 键盘 +
   *     记住宽度）。作者：「本地项目其他页面也在用，不要自己复用」—— 即别另起一份；
   *   · 开合：工具条上那颗「详情」（与左边 ☰ 对称），只记这一次会话。
   *     ★ 声明挪到了上面（左抽屉浮不浮的判据要读它），**默认收起**（2026-10-17）。
   *
   * ★ 它**不再是浮层**（2026-10-13 作者：「右侧的那个详情呀，我不想让它悬浮起来，
   *   还是不要悬浮起来了」）。左抽屉照旧会浮（窄窗里它盖住参数区、点一下就收），
   *   而这一栏在任何宽度下都并排站着 —— 所以 BbsRail 里那半套浮层样式也一并删了，
   *   要恢复就是给它加回 `position: absolute` 与按宽度给的 mode。
   */
  const railSizing = useBbsRail()
  /*
   * 两条侧栏的开关：**各开各的，互不牵连**。
   *
   * 上一版在浮层态做过一次"互斥"（开一条就收另一条）—— 作者 2026-10-13 打回来了：
   * 「我发现不能同时展开两个了」。他看得没错：两条都开着是**正常用法**
   * （左边挑预设、右边看详情），造成"悬浮感"的不是它们同时在，是那层灰（见 `.scrim`）。
   *
   * ★ 2026-10-13 又一轮：详情这一栏不再由"一颗按钮来回切" —— 开是页签行右端那颗 📄
   *   图标，关是栏自己右上角那颗叉（见 BbsRail 的 `onClose`）。栏开着的时候那颗图标
   *   干脆不画：开着的栏本身就是"详情"，再留一颗高亮的图标是在把同一件事说两遍。
   */
  const openRail = () => setRailOpen(true)
  const closeRail = () => setRailOpen(false)
  const toggleDrawer = () => setDrawerOpen(!drawerOpen)

  /*
   * 左抽屉那只 ↻：「重扫本机」。两件事一起做（2026-10-13）：
   *
   *   · **清单**重读（`data.rescan`）—— 刚在 BBS 里新建 / 删掉的预设跟着进出列表，
   *     左侧列表也会重铺一遍（那一下的动效见 BbsPresetDrawer 的 `.list[data-refreshing]`）；
   *   · **当前这一份**的链重读（`preset.refreshCurrent`）—— 你改完保存的那一份，
   *     右边跟着变；它是"拉完才换"，所以不会经过"基准掉成 0 再回来"那种中间态。
   *
   * 重读期间详情栏那几格数字会滚起来（老虎机，见 `BbsRail` 的 `refreshing`）——
   * 于是"在刷新"与"数据变了"是两回事。
   */
  const rescanAll = () => {
    data.rescan()
    void preset.refreshCurrent()
  }

  /*
   * 重扫进行中：清单在读（`data.loading`）或当前这一份的链在读（`preset.refreshing`）。
   *
   * 三处表现共用这一个量 —— 它们是同一件事的三种说法，别各传各的：
   *   · 左抽屉：头部下沿一条扫动的细线 + 列表项重铺（`BbsPresetDrawer`）；
   *   · 参数区：整块重铺（下面 `data-refreshing`）；
   *   · 详情栏：那几格数字滚起来（`BbsRail` 的 `refreshing`）。
   */
  const refreshing = data.loading || preset.refreshing

  /*
   * **整页只读**（2026-10-09 用户裁断）：这一页是「看 BBS 里是什么值」的查看器。
   * 原来那个「可改动」档砍掉了 —— 改了不落盘、看不到落点，是个假模式（审计 A-52 / X-14）。
   */
  /*
   * ★ 活口（2026-10-14 标记）：恒真常量，**看着像死代码，其实是"产品档位"的开关位**。
   * 唯一消费点是下面传给 `BbsControl` 的 `readOnly` —— 那一支对应的是"可编辑"那半套
   * （已按 2026-10-09 的裁决砍掉：改了不落盘、看不到落点，是个假模式）。
   * 留着它是为了"哪天真要恢复可编辑"时有一个明确的位置，而不是散落的注释。
   */
  const readOnly = true
  const [localNote, setLocalNote] = useState('')

  /* 选中项换了：筛选档位跟着它走，否则你会在一个筛不到它的列表里找不到当前那一项 */
  useEffect(() => {
    if (preset.current?.target) setTarget(preset.current.target)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preset.currentKey])

  /*
   * 预设页右键带过来的目标。按**文件名**在清单里找（两页的数据源不是同一份，
   * 不能互相塞对象）。找不到不弹错、不跳走 —— 停在默认那一份，在状态条说一句。
   */
  useEffect(() => {
    if (!pending || !data.presets.length) return
    const want = pending.name.toLowerCase()
    const hit = data.presets.find(
      (p) => p.file.toLowerCase() === want || p.name.toLowerCase() === want.replace(/\.json$/, ''),
    )
    if (hit?.selectable) {
      preset.pick(hit.key)
      setLocalNote('')
    } else {
      setLocalNote(
        hit
          ? tr('bbs.pending.notSelectable', {
              name: pending.name,
              reason: hit.reason ?? '',
            })
          : tr('bbs.pending.missing', { name: pending.name }),
      )
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending?.nonce, data.presets])

  /* 一份都没选中（清单空着 / 还没挑）。抽成布尔是为了当 useMemo 的依赖：
     直接写 `preset.current` 会被 react-hooks 判成「可变值当依赖」 */
  const hasCurrent = preset.current !== null

  const panel = useMemo(
    () => buildBbsPanel(
      /*
       * 版面**照常传**，没选中时走骨架档（2026-10-13）。
       *
       * 原来的写法是 `hasCurrent ? data.layout : []` —— 于是"没选中"就等于一整片空白，
       * 只有一句空态文案。作者要的不是那个：「如果没选择的时候，我希望这些字段参数都是在的，
       * 只不过它不显示参数……那个才叫骨架屏，因为它这个骨架是定好的，
       * 这些字段顺序都是固定好摆在这的，不会变化的。」
       *
       * 骨架档铺的是**真版面**（layout 里的组与字段，顺序原样），值全空着 ——
       * 所以它不是"我另画的一具假骨架"，就是这一页本身少了值。
       */
      data.layout,
      data.registry, preset.values, preset.user, preset.toggles, viewMode,
      paramQuery, preset.variant.index, !hasCurrent,
    ),
    [hasCurrent, data.layout, data.registry, preset.values, preset.user, preset.toggles, viewMode,
      paramQuery, preset.variant.index],
  )

  /** 归一化后的搜索词。空串 = 没在搜 */
  const q = useMemo(() => normalizeQuery(paramQuery), [paramQuery])
  const searching = q.length > 0

  /* 当前页被隐藏了（切档 / 换预设）就落到第一个还有内容的页，否则会看到一片空白 */
  const activeId = resolveActivePage(panel.pages, page)
  const active = panel.pages.find((p) => p.id === activeId) ?? null

  const modifiedSet = useMemo(() => new Set(preset.modifiedKeys), [preset.modifiedKeys])

  /*
   * 无头探针的落点，照上游的 `globalThis.__bbs`（bbs-preset.js:901）。
   * 只在 dev 挂，build 里这段被摇掉 —— 它是给 playwright 脚本读数字用的，
   * 不是产品功能，所以界面上没有任何东西依赖它。
   */
  useEffect(() => {
    if (!import.meta.env.DEV) return
    Object.assign(globalThis, {
      __bbs: {
        sourceMode: data.sourceMode,
        presets: data.presets.length,
        currentKey: preset.currentKey,
        chain: preset.chainNames,
        modified: preset.modifiedKeys,
        unknown: preset.unknownKeys,
        baseline: Object.keys(preset.baseline).length,
        stats: panel.stats,
        note: preset.note,
        error: preset.error,
      },
    })
  })

  const userCount = data.presets.filter((p) => p.scope === 'user' || p.scope === 'imported').length
  const systemCount = data.presets.filter((p) => p.scope === 'system' && !p.isAbstract).length
  /* 头里那个「 · 」不再带着了 —— 详情栏把它单摆一行（见 BbsRail 的继承那一段） */
  const variantNote = preset.variant.reason === 'ok'
    ? tr('bbs.variant', {
        want: preset.variant.want ?? '',
        index: preset.variant.index + 1,
      })
    : ''

  /*
   * 抽屉开合。**不再按密度档分叉** —— 上一轮给窄档写的「横过来铺在参数区上面」
   * （`.body { flex-direction: column }`）是个错：抽屉只占左边那么宽，右边一片空白，
   * 参数被推到抽屉下面、又被 `overflow: hidden` 裁掉，等于看不见。
   * 现在并排永远是并排，窄窗口靠抽屉的 `max-width: 60%` 收窄自己。
   */
  const showDrawer = drawerOpen
  /* `toggleDrawer` 挪到上面去了（浮层态要与右侧栏互斥，见那一处的注释） */

  /* 浮层态：选完就走、Esc 关、点参数区也关 —— 浮层的意思就是用完收起 */
  const pickPreset = useCallback((key: string) => {
    preset.pick(key)
    if (drawerMode === 'float') setDrawerOpen(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preset.pick, drawerMode, setDrawerOpen])

  /* Esc 收起浮层。只有左抽屉会浮（右侧那栏并排站着了，见上面那段注释） */
  useEffect(() => {
    if (drawerMode !== 'float' || !drawerOpen) return
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') setDrawerOpen(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [drawerMode, drawerOpen, setDrawerOpen])

  /*
   * 切形态不再有按钮 —— 形态由窗口宽度自动决定（见上面的 drawerMode），
   * 想收起就点「预设列表」。
   */

  if (data.fatal) {
    return (
      <div className={s.page} data-density={density} data-theme={light ? 'light' : 'dark'}>
        <p className={s.blank}>{data.fatal}</p>
      </div>
    )
  }

  return (
    <div
      ref={pageRef}
      className={s.page}
      data-density={density}
      data-theme={light ? 'light' : 'dark'}
      /*
       * 三条尺寸都以 CSS 变量下发：
       *   · 两条侧栏的宽度 —— 各自的拖宽 hook 给的（`width: var(--bbs-*-w)`）；
       *   · 标签列宽度 —— 上面 `labelWidth()` 按页面宽算的连续值（还没量到时不给，
       *     让它落回 CSS 里那个 236 兜底）。
       */
      style={{
        '--bbs-drawer-w': `${sizing.width}px`,
        '--bbs-rail-w': `${railSizing.width}px`,
        ...(pageW > 0 ? { '--bbs-label-w': `${labelWidth(pageW)}px` } : {}),
      } as React.CSSProperties}
    >
      {/*
        ============ 工具条（2026-10-13 第二轮：只留"常做"的五件事） ============
        作者看完落地那一版：「上面的内容太拥挤了……这个被截断了，显示不完整。
        搜索框、跟 BBS 一样、重扫本机、载入产品配置这些功能，我不知道哪些需要。
        你得站在用户角度看下他到底需要什么。预设列表干嘛占这么大空间？用三条横杠
        不就可以了吗？详情不一定要用小气泡，用一些图标……我希望放在右边。
        重扫本机应该放在预设列表里。这个功能（跟 BBS 一样）我觉得应该放到详情里面。」

        于是这一行只剩：☰ 列表 · 预设名 · 页签 · 搜参数 · 详情（图标）。

        搬走的三个都不是"看参数"时要用的东西：

          · **重扫本机** → 左抽屉（它是"刷新这个列表"，与机型筛选是同一层的事）；
          · **载入随包配置**（原来叫「载入产品配置」，作者："我不知道是干嘛用的"）
            → 也进左抽屉：它换的是"这份清单从哪来"；
          · **跟 BBS 一样 / 全部参数** → 右侧「详情」栏的一行（它与那一栏里的
            「行数 · 收起 N 行」本来就在说同一件事）。

        图标用现成的 `shared-ui/Icon`（`menu` / `refresh` / `doc`），不自己画。

        ★ 第三轮（同日）这两样又换了位：**搜参数搬上第一行（贴预设名右边）、
        详情图标搬下第二行（贴页签行最右端）** —— 搜参数是"看参数"时一直在敲的东西，
        该挨着预设名；详情管的是"右边那一栏开不开"，与页签同一行更近（作者圈的就是那儿）。
        同时那颗图标**栏开着时不画**了：开着的栏本身就是"详情"，再留一颗高亮的图标
        等于同一件事说两遍 —— 收栏改由栏自己右上角那颗叉来管（BbsRail 的 `onClose`）。
      */}
      {/* 工具条：只有「列表 · 名字 · 搜参数」三件 —— 一直是这一条，页签行永远是第二行 */}
      <div className={s.bar}>
        <button
          type="button"
          className={s.iconBtn}
          data-on={showDrawer}
          onClick={toggleDrawer}
          title={tr('bbs.presetListTitle')}
          aria-label={tr('bbs.presetList')}
        >
          <Icon name="menu" size={16} />
        </button>

        {/*
          形态切换按钮已退役（2026-10-13）：并排 / 浮层由窗口宽度自动决定
          （窄于 NARROW_W 就浮层），与报告页的滑盖同一个判据。
        */}

        <span className={s.name} title={preset.current?.file ?? ''}>{preset.title}</span>

        {/*
          搜参数（2026-10-13 第三轮从页签行搬上来）：它是"看参数"时一直在敲的东西，
          贴着预设名比贴在最右端顺手 —— 这一行本来只有 ☰ 与名字，右边的空位正好给它。
          宽度照旧是 `fixed` 那一档 200px（窄窗 140），见 BbsSearchBox.module.css。
        */}
        <BbsSearchBox
          value={paramQuery}
          placeholder={tr('bbs.searchPlaceholder')}
          label={tr('bbs.searchLabel')}
          onChange={setParamQuery}
        />

        {/*
          窗口的两枚开关（2026-10-17，作者：「置顶」「窄窗」）—— 挂在这一行最右端。
          为什么是这一页：他的用法是"置顶之后贴到切片器旁边看参数"，工艺页正是那一页；
          为什么是文字胶囊：这两件事图标读不出来（见 `WindowGadgets` 文件头）。
        */}
        <WindowGadgets />

        {/*
          「导入 JSON」已退役（2026-10-13 作者口径）：这一页只看真源 ——
          本机 BBS 目录（live），以及产品资源区下载来的配置；手工拖文件进来
          属于没实现的入口。等 MKPSupportEase 那边接上后，这里只剩真数据。
        */}


        {/* 2026-10-12：这一页那颗「深色 / 明亮」开关**退役** —— 深浅跟应用主题走
            （真值在 `useIsDark()`，见上面那条注释；设置页里那三档才是唯一的入口）。 */}
      </div>

      <div className={s.body}>
        <BbsPresetDrawer
          presets={data.presets}
          currentKey={preset.currentKey}
          dirty={preset.dirty}
          target={target}
          query={query}
          /* 看哪一边（2026-10-13）：默认 MKP —— 列表、机型下拉、空态都跟着它走 */
          source={source}
          onSource={setSource}
          open={showDrawer}
          mode={drawerMode}
          /* 重扫进行中（清单在读、或当前这一份的链在读）——
             抽屉据此扫那条细线、重铺列表、把 ↻ 提成 accent 色。它**不是**参数区的状态 */
          loading={refreshing}
          sizing={sizing}
          onTarget={setTarget}
          onQuery={setQuery}
          onPick={pickPreset}
          /* 「重扫本机」从工具条搬进来了（见工具条那段注释）：它是刷新这个列表。
             这一颗同时把右侧那一份也重读一遍 —— 见上面 `rescanAll` */
          rescan={data.sourceMode === 'live' ? rescanAll : null}
          /* 我们自己的切片器配置：目录里几份、本机下到几份 —— 只用来给 MKP 档的空态说话 */
          slicer={{ listed: delivered.listed.length, ready: delivered.downloaded.length }}
          /* 已经复制进切片器的那几份：本机目录里同名的标「我们发的」 */
          copied={fromUs}
          /* 还没下载的那几份点一下去哪儿（外壳给的动作，见 Props） */
          onNeedDownload={onNeedDownload}
        />

        {/* 浮层态的遮罩：点一下收起抽屉。它在抽屉之下、参数之上（z-index 15 vs 20） */}
        {drawerMode === 'float' && drawerOpen && (
          <button
            type="button"
            className={s.scrim}
            aria-label="关闭预设列表"
            onClick={() => setDrawerOpen(false)}
          />
        )}

        <div className={s.main}>
          {/* 第二行：页签 + 详情（搜参数第三轮搬上第一行，见工具条那段注释） */}
          <div className={s.tabsRow}>
            <BbsTabs
              pages={panel.pages}
              active={activeId}
              searching={searching}
              onChange={setPage}
            />

            {/*
              右侧「详情」栏那颗开关（第三轮从工具条搬下来 —— 作者圈的就是这个位置：
              页签行最右端，正对着右边那一栏）。

              **栏开着的时候它不画**：开着的栏本身就是详情，图标留着只像"还能再开一次"；
              收栏交给栏自己右上角那颗叉（BbsRail 的 `onClose`）。
              栏收起来时它替那一栏站岗，**有失败就在右上角挂一颗红点** —— 那条红字不能
              跟着栏一起消失（抽卡里定过的一条：错误不上锁）。
            */}
            {!railOpen && (
              <button
                type="button"
                className={s.iconBtn}
                onClick={openRail}
                title={preset.error !== ''
                  ? `详情：统计 · 继承链 · 说明（有一条错误：${preset.error}）`
                  : '详情：统计 · 继承链 · 说明'}
                aria-label="详情"
              >
                <Icon name="doc" size={16} />
                {preset.error !== '' && <span className={s.iconDot} />}
              </button>
            )}
          </div>

          {/* 命中全在别的页：列出来，**点了才切**（你说的不自动跳） */}
          {searching && panel.stats.hits > 0 && active?.hitCount === 0 && (
            <p className={s.hintBar}>
              这一页没有命中，命中在：
              {panel.pages.filter((p) => p.hitCount > 0).map((p) => (
                <button
                  key={p.id}
                  type="button"
                  className={s.hintLink}
                  onClick={() => setPage(p.id)}
                >
                  {p.title} {p.hitCount}
                </button>
              ))}
            </p>
          )}

          {/* 一个都没命中：照常显示全部行，不清空 —— 清空了你就没法确认「是真没有」 */}
          {searching && panel.stats.hits === 0 && (
            <p className={s.hintBar}>
              没有匹配「{paramQuery.trim()}」的参数
              {panel.stats.hiddenHits > 0
                && `（有 ${panel.stats.hiddenHits} 个在「跟 BBS 一样」收起的行里）`}
            </p>
          )}


          <div
            className={s.content}
            data-role="params"
            /* 重扫时整块重铺一遍（见 css 的 `.content[data-refreshing]`）——
               这一份的内容是真的重读过的，这一步是把它说出来 */
            data-refreshing={refreshing ? 'true' : undefined}
          >
            {/*
              读清单时**参数区一个字都不动**（2026-10-13）。
              这一轮的 loading 是"在读预设清单"，那是**左抽屉**的事（作者：「它应该是刷新左侧的列表，
              而不是说刷新它这一个右侧的参数吧」）—— 所以提示挂在抽屉里（见 BbsPresetDrawer 的
              `loading` 那一格），这里继续画手上已有的东西：有预设画它的行，没预设画骨架行。
            */}
            {/*
              没有本机目录就是这里的空态：说清「为什么空」，不拿陈旧快照充数。
              ★ **只在「本机 BBS」那一档说**（2026-10-13）：MKP 档下我们自己的那几份照样
              列得出来 —— 它们的正文就在手上，少的只是算基准用的父档（那时详情栏会照实
              说一句"父预设不在清单里"）。在这儿拦一道就等于把那一档也说成空的。
            */}
            {data.sourceMode === 'none' && source === 'bbs' && (
              <p className={s.blank}>
                没读到本机 BBS 预设目录：{data.sourceNote}
                <br />
                这一页列的是你本机 Bambu Studio 里的工艺预设 —— 本版不打包预设快照。
                <br />
                装好 Bambu Studio（或它的 Beta），本页就能读到它的工艺预设。
              </p>
            )}
            {(data.sourceMode === 'live' || source === 'mkp') && !active && (
              <p className={s.blank}>
                这一档下没有可显示的参数。切回「全部参数」看看。
              </p>
            )}
            {/*
              行一直画：有预设画真值；清单读到了但一份都没选，走骨架档 —— 铺的是 layout 里的
              组与字段、值空着（见上面 buildBbsPanel 那段注释）。
              **唯独"压根没有数据源"那一档不画**：那时上面已经有一句"没读到本机目录"，
              再铺一版面空字段只会自相矛盾。MKP 档不属于那一档（正文在手上，见上）。
            */}
            {(data.sourceMode !== 'none' || source === 'mkp') && active?.groups.map((g) => (
              <BbsGroup
                key={g.name}
                group={g}
                values={preset.values}
                baseline={preset.baseline}
                modifiedKeys={modifiedSet}
                variantIdx={preset.variant.index}
                icons={data.icons}
                readOnly={readOnly}
                query={q}
                onChange={preset.setValue}
                onReset={preset.resetKey}
              />
            ))}
          </div>
        </div>

        {/* 右侧「详情」栏：原来那条贴在窗口下缘的状态条搬到了这里（底条整条退场）。
            它**没有浮层态**了（作者：不悬浮），所以那一道遮罩也不用画。
            「收」有两处入口，各管一半：栏自己右上角那颗叉（`onClose`，栏开着时用），
            以及页签行右端那颗详情图标（栏收起来时才画，见上面那段） */}
        {railOpen && (
          <BbsRail
            title={preset.title}
            isSystem={preset.scope === 'system'}
            modifiedCount={preset.modifiedKeys.length}
            storedCount={Object.keys(preset.user).length}
            editedCount={preset.editedKeys.size}
            baselineCount={Object.keys(preset.baseline).length}
            viewMode={viewMode}
            /* 「跟 BBS 一样 / 全部参数」原来是工具条上那颗按钮，搬进这一栏 ——
               它与下面那格「行数 · 收起 N 行」本来就在说同一件事 */
            onViewMode={(next) => setViewMode(next)}
            stats={panel.stats}
            rulesNote={tr('bbs.rulesNote', { rules: RULES.rules, keys: RULES.keys })}
            chain={preset.chainNames}
            variantNote={variantNote}
            sourceMode={data.sourceMode}
            sourceRoot={data.sourceRoot}
            sourceNote={data.sourceNote}
            userCount={userCount}
            systemCount={systemCount}
            sync={data.sync}
            search={searching
              ? `搜「${paramQuery.trim()}」· 命中 ${panel.stats.hits} 项（这一页 ${active?.hitCount ?? 0}）`
              : null}
            note={[
              preset.note,
              localNote,
              preset.importError,
              /* 被规则收起的行里也有命中 —— 闷着不说会让人以为搜索漏了 */
              searching && panel.stats.hiddenHits > 0
                ? `另有 ${panel.stats.hiddenHits} 个命中在「跟 BBS 一样」收起的行里，切到「全部参数」能看到`
                : '',
            ].filter(Boolean).join(' · ')}
            error={preset.error}
            hasPreset={hasCurrent}
            fromMkp={preset.scope === 'mkp'}
            /* 重扫期间那几格数字滚起来（同一个 `refreshing`，见上面那段注释） */
            refreshing={refreshing}
            onClose={closeRail}
            sizing={railSizing}
          />
        )}
      </div>
    </div>
  )
}
