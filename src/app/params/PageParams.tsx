/*
 * 修改参数 —— 用户端。
 *
 * # 数据全部来自**同步下来的那份包**，不再来自手抄表和跨稿 import
 *
 * 分类计数来自 `ParamTab.count`（6 个分类：偏移 5 / 擦料 22 / 风扇 3 / 涂胶 18 / 切换 2 / 更多 17 = 67），
 * 卡片标题来自 `ParamSection.label`（16 个真 section 名），
 * 可见性条件来自 `ParamMeta.showWhen`（支持 eq / neq / gt），
 * 字段的选项、范围、单位来自 `RecipeParam`。
 *
 * 手抄表的涂胶 31 / 更多 4 和真注册表的涂胶 18 / 更多 17 总数相同但切分不同。
 * 这里以真注册表为准。
 *
 * # 搜索跨全部分类
 *
 * 验证时踩过「只搜当前页签得到 0 条，而那个字段其实在另一类里」。
 * 这里的搜索去掉分类筛选，命中项显示所属分组名。支持按 label / key / tomlKey 三样匹配。
 *
 * # gcode 字段走 renderBlock
 *
 * 用户端给整块编辑器（工作台的「N 行 · 点开」是另一个取向）。
 * 实测 control 分布：42 number / 13 choice / 10 switch / 2 text（那 2 个 text 就是 gcode）。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { STORAGE } from '../../api/storageKeys'
import { useStickyState } from '../../hooks/useStickyState'
import type { Density } from '../../hooks/useDensity'
import { FieldLayer } from '../../components/field'
import CategoryPills from './CategoryPills'
import GcodeBlock from './GcodeBlock'
import HistoryDrawer from './HistoryDrawer'
import type { DrawerMode, HistoryView } from './HistoryDrawer'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import ParamCard from './ParamCard'
import PresetPickerDrawer from './PresetPickerDrawer'
import SearchField from './SearchField'
import Icon from '../shell/icons'
import { useParams, valueText } from './useParams'
import type { ParamDef } from './useParams'
import s from './PageParams.module.css'

interface Props {
  density: Density
}

interface ShownCard {
  id: string
  title: string
  subtitle?: string
  fields: ParamDef[]
}

/** 双列：贪心按高度平衡，卡片高度用字段数近似 */
function splitDualColumns(cards: ShownCard[]): [ShownCard[], ShownCard[]] {
  const left: ShownCard[] = []
  const right: ShownCard[] = []
  let lh = 0
  let rh = 0
  for (const item of cards) {
    const h = item.fields.length + 1
    if (lh <= rh) {
      left.push(item)
      lh += h
    } else {
      right.push(item)
      rh += h
    }
  }
  return [left, right]
}

const MAX_CONFIRM_ITEMS = 8
const DUAL_MIN_WIDTH = 1180

/** 「最近搜索」留几条 —— 再多就把下拉变成一份清单了 */
const HISTORY_MAX = 8
/** 存的键收在 `src/api/storageKeys.ts`（值仍是试验场那个键名），刷新之后还在 */
const HISTORY_KEY = STORAGE.clientParamsSearchHistory

/**
 * 命中项的全部**受控参数**（顺着 `showWhen.key` 往下走，孙子也算）。
 *
 * 词先说清（作者：「不要叫他父级吧，叫做什么好呢」）：
 * **条件参数** = 它的值决定别人能不能用；**受控参数** = 被那个条件管着的那些。
 * 关系叫**条件依赖**，行为叫**条件显隐**（配置界面里的通用说法，英文 conditional
 * parameter / conditional visibility）。用户面上不需要名词 —— 那句
 * 「需先让『支撑面熨烫』等于 自动」已经把因果说完了。
 *
 * 作者搜「支」，结果里只有「支撑面熨烫」自己 —— 它下面那条受控参数
 * （熨烫挤出阈值）名字里不带「支」，没出来。他要的是「搜索到条件参数也要显示受控参数」。
 *
 * 关系只有一条：`meta.showWhen.key` 就是条件参数（`useParams` 的 `findBlocked`
 * 也是顺着它往上走的）。这里反过来用一遍：条件参数 key → 受控 keys，从命中项往下逐个收。
 * `out` 同时当访问标记，注册表里真有环也转不住。
 *
 * **`defs` 传一张卡的**：于是走到卡外的路自动断掉 ——
 * 「擦料方式」那种模式开关一命中，就不会把跨卡的两片互斥分支全拖出来。
 */
function descendantsOf(defs: ParamDef[], roots: Set<string>): Set<string> {
  const childrenByParent = new Map<string, string[]>()
  for (const def of defs) {
    const parent = def.meta?.showWhen?.key
    if (parent === undefined) continue
    const list = childrenByParent.get(parent)
    if (list === undefined) childrenByParent.set(parent, [def.key])
    else list.push(def.key)
  }

  const out = new Set<string>()
  const queue = [...roots]
  while (queue.length > 0) {
    const at = queue.shift() as string
    for (const child of childrenByParent.get(at) ?? []) {
      if (out.has(child) || roots.has(child)) continue
      out.add(child)
      queue.push(child)
    }
  }
  return out
}

/**
 * 命中项的**条件参数**（顺着 `showWhen.key` 往上走，爷爷也算）—— 上一段的另一半。
 *
 * 作者：「需要显示父参数」。命中一个受控参数时（比如直接搜「熨烫挤出阈值」），
 * 结果里只有它自己，还挂着一句「需先让『支撑面熨烫』等于 自动」—— 而**要开的那一项
 * 不在眼前**，那是一句没有主语的话。把条件行一起带出来，它就与分类里那一屏长得一样了
 * （条件行 + 灰底子块）。
 *
 * 两处「就停」：
 *   · 走到 `defs` 里没有的条件参数就停 —— 卡外的不带（见 `descendantsOf`）；
 *     顺带也管住了「被 machineFilter 排除的字段」（这台机型上无从满足、也无从开启，
 *     与 `useParams` 的 `findBlocked` 同一条规则，否则界面会写
 *     「需先开启 某个本机型根本没有的项」）。
 */
function ancestorsOf(defs: ParamDef[], roots: Set<string>): Set<string> {
  const parentByKey = new Map<string, string>()
  const known = new Set<string>()
  for (const def of defs) {
    known.add(def.key)
    const parent = def.meta?.showWhen?.key
    if (parent !== undefined) parentByKey.set(def.key, parent)
  }

  const out = new Set<string>()
  const queue = [...roots]
  while (queue.length > 0) {
    const at = queue.shift() as string
    const parent = parentByKey.get(at)
    if (parent === undefined || !known.has(parent)) continue
    if (out.has(parent) || roots.has(parent)) continue
    out.add(parent)
    queue.push(parent)
  }
  return out
}

/**
 * 分类条那一行要同时装下「分类条 + 搜索框」需要多宽 —— 按档位三个**实测**常量。
 *
 * 每个数 = 分类条整块的自然宽（含右边那截当前分类名）+ 缝 + 搜索框槽宽 + 这一行的
 * 左右内衬 `2×(pad+10)` + 12px 余量：
 *
 *   mini           246 + 8  + 132 + 42 + 12 = 440   ← 搜索框 160 → 132
 *   compact        332 + 12 + 132 + 42 + 12 = 530
 *   wide / ultra   469 + 12 + 216 + 42 + 12 = 751
 *
 * 卡片区窄过它，搜索框就搬到头部标题行（那行本来就有空白）—— 作者：
 * 「我希望圈起来的能显示完整，也就是说这个搜索在特定窗口大小下可以移动到上面」。
 *
 * 实测只有 mini 档（窗口 < 640）会真的触发：窗口 460 及以下（卡片区 ≤ 458）搬上去，
 * 480 起（卡片区 478）留在分类条那一行。compact 与 wide 的卡片区下限（598 / 1038）
 * 都远大于各自的阈值，永不触发。
 *
 * # 为什么**不**跟着搜索框一起往下降
 *
 * 搜索框收窄了 28px（160 → 132），照公式三个常量本该同步降 28。但分类条那一侧
 * 同一轮改了退化规则（条外那截名字**不再**在 mini 档藏起来），mini 的自然宽
 * 从 246 变成约 315（246 + 名字那一段 69）。两边一升一降，真需要的地方反而更多，
 * 所以三个数**一个没动**：它们的含义是「什么时候该把这一行让给分类条」，
 * 降下来只会让分类条在更窄的地方被挤到横滚（实测过：卡片区 460 那一档会缺 17px）。
 * 实测扫描 400→1100 每 20px：分类条**一次都没有横滚**，就是靠这三个数兜住的。
 */
const BAR_NEED: Record<Density, number> = {
  ultra: 775,
  wide: 775,
  compact: 558,
  mini: 468,
}

/**
 * 量一个元素有多宽 —— 用**回调 ref**，不用 `useEffect + RefObject`。
 *
 * 为什么换写法（实测出来的，前后两版都栽在这上面）：
 * 原来是 `useEffect(..., [ref])` 里读 `ref.current` 再建 ResizeObserver。
 * 但这一页有一个早期 return：
 *
 *     if (u.loading) return <p>正在打开数据包…</p>
 *
 * 首次渲染走的是那一条，`.bodyWrap` 根本不在 DOM，effect 跑的时候 `ref.current === null`
 * 直接 return —— **RO 从来没被创建过**。等数据包读出来、元素真出现时，effect 的依赖
 * `[ref]`（ref 对象永远是同一个）没变，effect 不重跑，于是 RO 永远不会建立，
 * `width` 永远停在初始值 0 → `0 >= 1180` 恒假 → **双列永远不开**。
 *
 * 实测证据：`.bodyWrap` 的 `clientWidth = 1598`，而组件里的 `bodyWidth` state 是 `0`。
 *
 * 更早那一版的截图之所以是双列：它的参数数据来自手编表（同步，没有 loading 这一档），
 * 元素首帧就在 DOM 里，effect 一次就量到了。接真契约引入异步加载之后，
 * 顺手把双列关掉了，谁都没注意到。
 *
 * 回调 ref 由 React 在**元素每次挂载 / 卸载时**调用，晚出现也会被接上，
 * 不存在「跑太早、之后不再重试」这件事。
 */
function useWidth(): [(el: HTMLElement | null) => void, number] {
  const [width, setWidth] = useState(0)
  const roRef = useRef<ResizeObserver | null>(null)

  const attach = useCallback((el: HTMLElement | null) => {
    /* 换元素（或卸载）时先断开上一个，免得留着观察一个已经不在的节点 */
    roRef.current?.disconnect()
    roRef.current = null
    if (el === null) return
    setWidth(el.getBoundingClientRect().width)
    const ro = new ResizeObserver((entries) => setWidth(entries[0].contentRect.width))
    ro.observe(el)
    roRef.current = ro
  }, [])

  return [attach, width]
}

export default function PageParams({ density }: Props) {
  const u = useParams()

  const [categoryId, setCategoryId] = useState('')
  const [query, setQuery] = useState('')
  const [historyOpen, setHistoryOpen] = useState(false)
  const [historyMode, setHistoryMode] = useState<DrawerMode>('float')
  const [historyView, setHistoryView] = useState<HistoryView>('live')
  const [expandedDetailKey, setExpandedDetailKey] = useState<string | null>(null)
  const [askRestore, setAskRestore] = useState(false)
  const [askSave, setAskSave] = useState(false)
  const [attachBody, bodyWidth] = useWidth()
  /** 预设抽屉的开合 + pill 的 ref（关抽屉后焦点要还回去） */
  const [pickerOpen, setPickerOpen] = useState(false)
  const pillRef = useRef<HTMLButtonElement>(null)
  /** 搜索框的 ref —— `Ctrl+F` 把焦点放进去（槽在头部还是分类条那一行由布局定） */
  const searchRef = useRef<HTMLInputElement | null>(null)

  const closePicker = useCallback(() => {
    setPickerOpen(false)
    pillRef.current?.focus()
  }, [])

  /**
   * 「最近搜索」（作者：「我点击搜索的时候没有展开搜索历史呀」）。
   * 放在页面这一层而不是搜索框里：那个框窄窗时会从分类条那一行搬进标题行，
   * React 认的是两个不同的位置 —— 状态留在框里就会被搬丢。
   */
  const [searchHistory, setSearchHistory] = useStickyState<string[]>(HISTORY_KEY, [])

  /** 记一笔：去重（不分大小写）、置顶、封顶 HISTORY_MAX */
  const rememberSearch = useCallback(
    (term: string) => {
      const t = term.trim()
      if (t === '') return
      setSearchHistory((prev) =>
        [t, ...prev.filter((x) => x.toLowerCase() !== t.toLowerCase())].slice(0, HISTORY_MAX),
      )
    },
    [setSearchHistory],
  )

  const forgetSearch = useCallback(
    (term: string) => setSearchHistory((prev) => prev.filter((x) => x !== term)),
    [setSearchHistory],
  )

  /* 当 tabs 到了但 categoryId 还没选（或选的那一项不在 tabs 里），落到第一个 */
  useEffect(() => {
    if (u.tabs.length > 0 && !u.tabs.some((t) => t.id === categoryId)) {
      setCategoryId(u.tabs[0].id)
    }
  }, [categoryId, u.tabs])

  const searching = query.trim() !== ''
  const canPin = density === 'ultra' || density === 'wide'
  const pinned = historyOpen && historyMode === 'pinned' && canPin
  /*
   * 悬浮抽屉的宽度状态：拖左缘改宽、三态吸附回 380、双击/Home 复位、
   * localStorage 记住 —— 全稿同一套（BBS 预设抽屉同源，见 shared/useDrawerWidth）。
   */
  const historyDrag = useDrawerWidth({
    storageKey: STORAGE.clientParamsHistoryDrawerW,
    side: 'right',
    snap: 380,
    min: 280,
    max: 560,
    label: '修改历史',
  })

  /* 当前分类里的 section 列表 */
  const currentTab = u.tabs.find((t) => t.id === categoryId)

  /*
   * 搜索：跨全部分类的就地筛选。
   * 按 label / key / tomlKey 三样匹配（只搜 label 会漏掉用英文名搜的场景）。
   * 命中项显示所属分组名。一条都不命中的 section 整张不出现。
   *
   * 命中项**上下都带** —— 受控参数（`descendantsOf`）与条件参数（`ancestorsOf`）。
   * 一开始他搜「支」只看见「支撑面熨烫」自己，它下面那条「熨烫挤出阈值」不见了；
   * 后来又反过来：「直接搜一个参数时，要能看见它的条件参数」——
   * 否则那句「需先让『支撑面熨烫』等于 自动」说的是一个不在眼前的东西。
   * 带出来的行按 `split()` 摆成条件行 + 灰底子块，与分类里那一屏长得一样。
   *
   * 后来不再数「命中 N 条」（作者：「命中 xxx 都可以不显示了吧」）——
   * 那个数字要数清「被带出来的不算」，而结果本身就在下面按卡列着，没必要再报一遍。
   *
   * # **只在同一张卡里带**（作者：「擦料方式这种很大的父级的怎么办」）
   *
   * 早先的写法是拿 `u.defs`（全机型全分类）去收上下家，于是「擦料方式」这种
   * **模式开关**一命中，它底下那两片互斥分支全被拖出来 —— 实测搜「擦料方式」：
   * **5 张卡 22 行**（擦料塔位置与打印 4 + 安全与高级参数 7 + 塔结构加强 6 +
   * 圆盘动作控制 4），而真正命中的只有它自己。他自己那台程序不是这么办的：
   * 「切换到圆盘擦拭的时候那些擦料塔的都隐藏，切换到擦料塔的时候圆盘擦拭的都隐藏」——
   * 即只看当前那一支。
   *
   * 判据落成一句**同一张卡**：`descendantsOf` / `ancestorsOf` 改喂**这张卡自己的
   * `defsOfSection(sec.id)`**，于是要走的路只认卡里的键 —— 跨卡的那一片自动断在这里
   * （`ancestorsOf` 里本来就有一条「条件参数不在 known 里就停」）。这与灰底子块的判据
   * 天然对齐：受控参数能嵌到条件行下面，前提本来就是它们在**同一张卡**里（见 `split()`）。
   *
   * 改后实测：搜「擦料方式」**1 张卡 1 行**（它自己）；搜「塔体打印速度」**1 张卡 2 行**
   * （同卡的条件参数「擦料塔速度与质量」+ 它自己），擦料方式那张卡不再跟出来。
   * 上面那几个例子不受影响 —— 它们的条件关系本来就在同一张卡里。
   */
  const shown = useMemo<ShownCard[]>(() => {
    if (!searching) {
      if (currentTab === undefined) return []
      return currentTab.sections.map((sec) => ({
        id: `${currentTab.id}-${sec.id}`,
        title: sec.label,
        fields: u.defsOfSection(sec.id),
      }))
    }

    const q = query.trim().toLowerCase()
    /*
     * 匹配面（补了两个，作者：「可是我搜索圆盘搜不出」）：
     *   参数名 / 字段名 / TOML 键名  ← 原来就有的三样
     *   **选项名**（choices 的 label） ← 「圆盘」是「擦料方式」的一个选项，不是任何参数名
     *   **分组名**（section 的 label） ← 「圆盘动作控制」是一整张卡的名字
     * 只按前两样搜，「圆盘」在这份注册表里一个字都匹配不到 —— 而它明明写在两个地方。
     * 代价说在明处：搜「开启 / 关闭」这类通用选项词会把带这个选项的都列出来。
     */
    const isMatch = (d: ParamDef) =>
      d.label.toLowerCase().includes(q) ||
      d.key.toLowerCase().includes(q) ||
      (d.meta?.tomlKey ?? '').toLowerCase().includes(q) ||
      (d.field.choices ?? []).some((c) => c.label.toLowerCase().includes(q))

    const cards: ShownCard[] = []
    for (const tab of u.tabs) {
      for (const sec of tab.sections) {
        const defs = u.defsOfSection(sec.id)
        const hit = new Set(defs.filter(isMatch).map((d) => d.key))
        /*
         * 分组名命中 = 这一整张卡都算命中（他搜的是这张卡的名字）。
         * **不匹配分类名**（涂胶 / 擦料）：那等于「搜分类 = 整类都出来」，
         * 反而把这个过滤变成摆设 —— 分类本来就在分类条上、也印在每张卡的副标题里。
         */
        const wholeCard = sec.label.toLowerCase().includes(q)
        if (hit.size === 0 && !wholeCard) continue
        /* 上下都带，但**只认这张卡里**的路（见上面那段） */
        const carried = new Set([...descendantsOf(defs, hit), ...ancestorsOf(defs, hit)])
        const fields = wholeCard
          ? defs
          : defs.filter((d) => hit.has(d.key) || carried.has(d.key))
        cards.push({
          id: `${tab.id}-${sec.id}`,
          title: sec.label,
          subtitle: tab.label,
          fields,
        })
      }
    }
    return cards
  }, [currentTab, query, searching, u])

  const dualColumn = bodyWidth >= DUAL_MIN_WIDTH && shown.length > 1

  /**
   * 搜索框放哪一行：够宽时待在分类条那一行的右端，不够时搬到头部标题行。
   * `bodyWidth` 量的是卡片区（`.bodyWrap`，见 useWidth），阈值与实测边界见 BAR_NEED。
   *
   * **mini 档一律在上面** —— 作者要的小尺寸摆法是「搜索在上面、
   * 修改历史在下面」，所以这一档不再看阈值，免得 480 与 460 两种宽度算法不一。
   */
  const searchInHead = density === 'mini' || bodyWidth < BAR_NEED[density]

  /** 「修改历史」在 mini 档跟着搜索框换到下一行（分类条那一行）的右端 */
  const historyInBar = density === 'mini'

  /** 保存确认弹层要显示的改动列表。from = 已保存值，to = 草稿值 */
  const changedEntries = useMemo(() => {
    const entries: { key: string; label: string; from: string; to: string }[] = []
    for (const def of u.defs) {
      if (!u.dirtyOf(def.key)) continue
      entries.push({
        key: def.key,
        label: def.label,
        from: valueText(def, u.savedValueOf(def.key)),
        to: valueText(def, u.valueOf(def.key)),
      })
    }
    return entries
  }, [u])

  const confirmSave = () => {
    u.save()
    setAskSave(false)
  }

  const [leftCol, rightCol] = useMemo(
    () => (dualColumn ? splitDualColumns(shown) : [shown, [] as ShownCard[]]),
    [dualColumn, shown],
  )

  /* 文件名从说明书里来：包里每个版本带着自己的文件清单（A1/STANDARD → A1.toml）；
     这个版本没配 MKP 文件（如 A2L）才退成「机型 · 版本」 */
  const fileLabel = u.fileLabel ?? `${u.machineId} · ${u.versionId}`

  /*
   * 看的那份**不是**已应用的那份 —— pill 不画绿、换琥珀，副标题行尾给一句
   * **可点的**「切换回已应用」。作者把点名砍了：「不用显示那么清楚……
   * 因为有时候是用户他自己的，你都不知道他是什么版本」——所以提示不含机型 / 版本。
   * 底账空（activeUse === null）时不提示：没有已应用的就没有可对比的对象（见 useParams）。
   */
  const au = u.activeUse
  const inactive = au !== null && !au.onIt

  /*
   * 「切换回已应用」的动作。跳之前**现读一次底账**，不依赖渲染期的收窄 ——
   * 有未保存改动时 `requestCombo` 自会先问一句（那条路是现成的，不在这里另写）。
   */
  const backToActive = () => {
    const back = u.activeUse
    if (back === null || back.machineId === null || back.versionId === null) return
    u.requestCombo(back.machineId, back.versionId)
  }

  /*
   * 底栏那句状态文案。mini 档只留前半句 —— 实测「保存后会覆盖 A1.toml」这半截
   * 占 153px，加上左边三个按钮正好把底栏顶成两行（作者：「下面改不改都多一行」）。
   * 半句话没丢：保存确认弹层里本来就在说（「以下 N 处改动将写入 A1.toml」），
   * 而且这句挂了 title，悬停看全。
   */
  const stateText =
    u.dirtyCount === 0
      ? density === 'mini'
        ? '没有改动'
        : '没有未保存的改动'
      : density === 'mini'
        ? `${u.dirtyCount} 处未保存`
        : `${u.dirtyCount} 处未保存 · 保存后会覆盖 ${fileLabel}`

  const closeHistory = () => setHistoryOpen(false)

  const renderCards = (cards: ShownCard[]) =>
    cards.map(({ id, title, subtitle, fields }) => (
      <ParamCard
        key={id}
        title={title}
        subtitle={subtitle}
        fields={fields}
        mode={searching ? 'search' : 'normal'}
        /* 高亮只看词：不搜索时是空串，`Highlight` 原样返回 */
        query={query}
        valueOf={u.valueOf}
        savedValueOf={u.savedValueOf}
        factoryOf={u.factoryOf}
        dirtyOf={u.dirtyOf}
        blockedByOf={u.blockedBy}
        expandedKey={expandedDetailKey}
        onToggleExpand={(key) => setExpandedDetailKey((prev) => (prev === key ? null : key))}
        onEdit={u.edit}
        onRevertToSaved={u.revertToSaved}
        renderBlock={(def) => (
          <GcodeBlock
            def={def}
            value={u.valueOf(def.key)}
            dirty={u.dirtyOf(def.key)}
            disabled={u.blockedBy(def.key) !== null}
            savedValue={u.savedValueOf(def.key)}
            onEdit={u.edit}
            onRevertToSaved={u.revertToSaved}
          />
        )}
      />
    ))

  /*
   * 键盘入口（撤销 / 重做 / 保存 / 搜索 / Esc）。
   *
   *   Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y   撤销 / 重做
   *   Ctrl+S                          保存修改 —— 与点那颗按钮**同一条路**：先开确认弹层
   *   Ctrl+F                          焦点进搜索框（搜什么、跨全部分类，都是搜索框自己的事）
   *   Esc                             关掉当前那一层弹层
   *
   * 早先撤销 / 重做两颗按钮换成图标时留过一句「或者省略，只用快捷键」，但快捷键
   * 当时并不存在（整份稿查过：没有一个 `keydown` 认 Ctrl+Z）—— 于是「省略」这条路走不通，
   * 只能留着图标。现在补上。
   *
   * **装在这一页，不装外壳。** 只有参数台有撤销栈（`useParams` 那一份是页面级的：
   * 首页 / 预设页 / 校准页没有可撤的东西），装在外壳上就成了「在别的页按了没反应」。
   * 工作台那一套装在外壳，是因为它的撤销栈本来就是全局一份（store）。
   *
   * 几条细节：
   *   · 焦点在输入框里也认，并且 `preventDefault`：参数台的账比浏览器给 `<input>` 的
   *     原生文本撤销更贴题 —— 改的是参数，退就该退回参数。
   *     （实测过这一条：改完 5 之后焦点还在那个输入框里按 Ctrl+Z，值回到 -1 而不是
   *     「只把框里的字退回去」。）
   *   · `e.altKey` 时不认：`Ctrl+Alt+Z` 在部分键盘布局上是 AltGr+Z（欧洲键盘打 @ 那种）。
   *   · **弹层（保存确认 / 恢复默认值 / 切机型确认）开着时，认得的这几个键一律吃掉**
   *     （`preventDefault` + 什么都不做）。不能只是「不办我的事」：那样按键会落到浏览器
   *     手里，焦点要是在某个输入框上，它会去改那个框里的字 ——
   *     弹层背后悄悄变一个值，比什么都不发生更糟。关掉弹层再动手，账才干净。
   *   · `Ctrl+S` 不直落保存：那是鼠标走的路（先看清楚要写哪几处，再按「确认保存」），
   *     键盘不该有另一条更快的路。没有未保存改动时它什么都不做（那颗按钮此刻也是灰的）。
   *   · Esc 用**捕获阶段**：抽屉与预设选择器的 Esc 挂在 `document` 上（冒泡），
   *     弹层开着时这个键得先在这里被拦下来，否则弹层与抽屉会一起关。
   *
   * 依赖写解构出来的具体值（不写整个 `u`）：`undo` / `redo` 的 `useCallback` 依赖里有
   * draft / past / future / log，每改一次身份就换一次，于是每改一次重挂一遍监听
   * （一次 remove + 一次 add，可以忽略）—— 好处是**不会拿到旧的闭包**，
   * 撤销栈读的正是这两个回调里的那份快照。写成员表达式 `u.undo` 则会让 eslint 的
   * react-hooks/exhaustive-deps 认不出（它报「缺 u」），写成 `[u]` 又变成每帧重挂。
   *
   * 没有加载态守卫：数据包还没读到 `past` 是空的，`undo()` 自己会 `return`（同一条规则
   * 管着重做），加一层 `if (loading) return` 只会让「守卫用的还是挂载时那个值」这种
   * 陈旧判断混进来。
   */
  const { undo, redo, pendingCombo, cancelPending, dirtyCount } = u
  useEffect(() => {
    const sheetOpen = askSave || askRestore || pendingCombo !== null
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (!sheetOpen) return /* 没弹层就让下去：抽屉 / 预设选择器自己会关 */
        e.preventDefault()
        e.stopPropagation()
        if (askSave) setAskSave(false)
        else if (askRestore) setAskRestore(false)
        else cancelPending()
        return
      }
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return
      const k = e.key.toLowerCase()
      if (k !== 's' && k !== 'f' && k !== 'z' && k !== 'y') return
      /* 认得出是这一页的键，就先吃掉 —— 弹层开着也一样（理由见上面注释） */
      e.preventDefault()
      if (sheetOpen) return
      if (k === 's') {
        if (dirtyCount > 0) setAskSave(true)
        return
      }
      if (k === 'f') {
        searchRef.current?.focus()
        return
      }
      if (k === 'y' || e.shiftKey) redo()
      else undo()
    }
    /* 捕获阶段：Esc 要抢在 `document` 上那些抽屉监听之前（见注释） */
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [undo, redo, pendingCombo, cancelPending, dirtyCount, askSave, askRestore])

  if (u.loading) {
    return (
      <div className={s.page} data-density={density}>
        <p className={s.loading}>正在打开数据包…</p>
      </div>
    )
  }

  /*
   * 读不到目录（catalog）时的样子。
   *
   * 这一页画什么（页签、分组、每条的控件与类型、什么条件下显示）全部来自目录 ——
   * 读不到它就**没有页面可言**，所以这里不是「加载失败」那种红字（那是「本该有却没有」），
   * 而是一个空态：一句人话 + 后端给的原因。
   * 原来这里还有一颗「去「同步」页获取一份」—— 那一页 2026-10-02 退役（数据源配置
   * 降级成设置页里的开发后门），这条跳转没有去处，删掉。那句「加载失败：」是照更早
   * 那一版的壳抄的，而那一版走 `api.*`（客户端内置，不会没有），语义根本不一样。
   */
  if (u.error !== null) {
    return (
      <div className={s.page} data-density={density}>
        <div className={s.emptyBox}>
          <h2 className={s.emptyTitle}>读不到参数目录</h2>
          <p className={s.emptyText}>
            参数页画什么（页签、分组、每条的控件与类型、什么条件下显示）全部来自随安装包
            走的那份目录（catalog）—— 读不到它时，这一页没有东西可画，也不编一份假的。
          </p>
          <p className={s.emptyText}>{u.error}</p>
        </div>
      </div>
    )
  }

  /*
   * 「修改历史」那颗按钮：标题行（宽档）与分类条那一行（mini 档）两处共用同一颗。
   *
   * **它是开关**（作者对着开着抽屉的截图：「这个时候点击了修改历史应该可以
   * 折叠起来」）。原来是 `setHistoryOpen(true)` —— 抽屉已经开着时点它什么都不发生，
   * 而那颗按钮（`aria-expanded=true` 时还带绿框）看起来明明是个「开着」的状态，
   * 点下去没反应就是骗人。现在点一下就收，`aria-expanded` 同时是样式钩子。
   *
   * 悬浮态的抽屉会盖住这颗按钮（抽屉从右缘铺过来），那一路本来就不靠它收 ——
   * 点抽屉外面、抽屉自己的 ×、Esc 三条都还在；这里修的是**固定在右侧**那一路：
   * 抽屉占住右侧一条时，这颗按钮就在内容区的右上角，点得到也看得见。
   */
  const historyButton = (
    <button
      type="button"
      className={s.historyBtn}
      aria-haspopup="dialog"
      aria-expanded={historyOpen}
      title={historyOpen ? '收起修改历史' : '修改历史'}
      onClick={() => setHistoryOpen((v) => !v)}
    >
      修改历史
      {u.history.length > 0 && <span className={s.detailCount}>{u.history.length}</span>}
    </button>
  )

  /*
   * 搜索框也是两处共用（分类条那一行右端 / mini 档搬到标题行，见 searchInHead）——
   * 加上「最近搜索」那几个 prop 之后，再各写一份就该分叉了。
   */
  const searchField = (
    <SearchField
      value={query}
      density={density}
      inputRef={searchRef}
      history={searchHistory}
      onChange={setQuery}
      onCommit={rememberSearch}
      onPick={(term) => {
        setQuery(term)
        rememberSearch(term)
      }}
      onForget={forgetSearch}
    />
  )

  return (
    <div className={s.page} data-density={density}>
      <FieldLayer>
        <div className={s.main}>
          <div className={s.left}>
            <header className={s.head}>
              <div className={s.headTop}>
                <h2 className={s.pageTitle}>修改参数</h2>

                {/*
                 * 预设选择器：一个文件名 pill + 「切换」小字，点开抽屉。
                 * 更早那一版用的是两个共用件下拉（机型 / 版本），作者的原话是
                 * 「那个下拉菜单，我不喜欢，我喜欢改成抽屉的」。换成 pill 之后顶部只剩
                 * 一件东西，标题那一行也清爽了。
                 *
                 * 换 combo 仍走 `u.requestCombo` —— 未保存改动的确认弹层在那一层，
                 * 抽屉自己不判断脏数据。
                 */}
                <button
                  type="button"
                  className={inactive ? `${s.presetPill} ${s.presetPillOff}` : s.presetPill}
                  ref={pillRef}
                  title={
                    inactive
                      ? `当前预设 ${fileLabel}（未应用）· 点击切换`
                      : `当前预设 ${fileLabel} · 点击切换`
                  }
                  onClick={() => setPickerOpen((v) => !v)}
                >
                  <span className={s.presetName}>{fileLabel}</span>
                  <span className={s.presetSwitch}>切换</span>
                </button>

                {/*
                 * 窄窗时搜索框搬到这一行（见 searchInHead）—— 挤在文件名
                 * pill 右边、宽度跟着剩下的空间收，让分类条那一行只剩分类条。
                 */}
                {searchInHead && (
                  <div className={s.searchSlot} data-row="head">
                    {searchField}
                  </div>
                )}

                {/*
                 * 修改历史搬到**右上角**（作者：「右上角不用出现那个卡片了，
                 * 太占位置了」）—— 文件概要卡整个撤掉：机型/版本
                 * 副标题那一行本来就在说，历史抽屉里还有全部账。按钮 margin-left:auto
                 * 顶到最右，与参考布局一致。
                 *
                 * **mini 档不放在这一行**，下一行（分类条那一行）的右端才是它的位置
                 * —— 作者对着自己画的示意图说的原话：「小尺寸这时候搜索在上面，
                 * 修改历史在下面」。抽成 `historyButton` 一个变量，两处共用同一颗。
                 */}
                {!historyInBar && historyButton}
              </div>
              {/*
                提示挂在副标题行尾，不放 pill 那一行：窄档（mini 400px）标题 / pill /
                搜索三件正好铺满，实测没有余量再塞一句话（见 .headTop 的 mini 注释）。
                这一行本来就是「当前在看哪台」的说明，给「切换回已应用」这个动作正合适；
                它**能点** —— 名字都很像，一键回到已应用的那份，不靠认名字；
                也**不点名**（已应用那份可能是用户自己的文件，没有可靠的机型 / 版本可标，
                见 useParams 的 ActiveUse）。已应用那份不在包里就没有动作，只陈述。
              */}
              <p className={s.sub}>
                {u.machine?.display ?? '—'} · {u.version?.name ?? '—'} · 精细调整参数，优化涂胶表现
                {au !== null &&
                  !au.onIt &&
                  (au.canJump ? (
                    <button
                      type="button"
                      className={s.usingNote}
                      title="点击切换回已应用的那一份"
                      onClick={backToActive}
                    >
                      切换回已应用
                    </button>
                  ) : (
                    <span className={s.usingNote} title="已应用那份不在当前数据包里，这里切不过去">
                      已应用的是另一份
                    </span>
                  ))}
              </p>
            </header>

            <div className={s.bar}>
              <CategoryPills
                tabs={u.tabs}
                current={categoryId}
                dirty={u.dirtyTabs}
                /*
                 * 点分类**不清搜索**（作者：「点击了 tab 之后，搜索就被清空了，
                 * 那我还需要搜索的记录」）。原来这里写的是 `setQuery(''); setCategoryId(id)`
                 * —— 搜到一半换个分类看，刚打的字就没了。
                 *
                 * 搜索在着的这一段时间里，结果仍然是**跨分类**的（只搜当前
                 * 页签会漏掉「东西其实在另一类里」的那些）。所以这时点分类只换选中的那一枚
                 * 与条外那截分类名，列表不动；把搜索清掉，落到的就是他刚点的那一类。
                 */
                onSelect={setCategoryId}
              />
              {/* 搜索框在这一行的右端；窄窗时它已经搬到上面那行了（见 searchInHead） */}
              {!searchInHead && (
                <div className={s.searchSlot} data-row="bar">
                  {searchField}
                </div>
              )}

              {/* mini 档的「修改历史」搬到这里（见 historyInBar）：搜索在上面，它在下面 */}
              {historyInBar && historyButton}
            </div>

            {u.savedNote !== null && <p className={s.ok}>{u.savedNote}</p>}
            {/*
              草稿那一行的状态（参数页底座）：改的值**当场写进草稿 TOML**，
              所以关掉软件再回来还在。写失败要看得见 —— 不能只有界面上改了、盘上没改。
              没有可编辑目标时（这个 combo 没配 MKP）也说一句，别让人以为改了会保存。
            */}
            {u.draftError !== null && (
              <p className={s.warn}>草稿没写进磁盘：{u.draftError}</p>
            )}
            {u.draftError === null && u.editingPreset === null && u.fileLabel === null && (
              <p className={s.warn}>
                这个版本没配 MKP 预设文件 —— 改动不会保存在任何文件里（只在这一屏）
              </p>
            )}

            {/*
              测量层与分列容器**分开的两层**：
                .bodyWrap  永远存在，只量宽度（回调 ref 接在这里）
                .scroll / .dual  会随 dualColumn 整个换掉

              两层都必要，各治一个毛病：
                · 分开 → 分列切换时被换掉的是里面那个，测量层不受影响
                · 回调 ref（见 useWidth 的注释）→ 元素在 loading 之后才出现也能接上，
                  这才是「双列一直没开」的真根因（RO 压根没被创建过）
            */}
            <div className={s.bodyWrap} ref={attachBody}>
              <div className={dualColumn ? s.dual : s.scroll}>
                {dualColumn ? (
                  <>
                    <div className={s.col}>{renderCards(leftCol)}</div>
                    <div className={s.col}>{renderCards(rightCol)}</div>
                  </>
                ) : (
                  <>
                    {renderCards(shown)}
                    {searching && shown.length === 0 && (
                      <p className={s.empty}>没有匹配的参数，换个词试试</p>
                    )}
                  </>
                )}
              </div>
            </div>
          </div>

          {pinned && (
            <HistoryDrawer
              open
              file={fileLabel}
              mode="pinned"
              canPin={canPin}
              view={historyView}
              history={u.history}
              groups={u.historyByBatch}
              onView={setHistoryView}
              onMode={setHistoryMode}
              onClose={closeHistory}
              onRevert={u.revertEntry}
            />
          )}
        </div>

        <footer className={s.foot}>
          <div className={s.footLeft}>
            {/*
             * 撤销 / 重做改图标（作者：「撤销重做用图标吧，或者省略？只用快捷键」）。
             * 选图标不选「省略」：当时**这一页还没有绑 Ctrl+Z / Ctrl+Y**（整份稿查过一遍），
             * 删了就等于撤销这条路彻底看不见了。图标 28×28，两颗共省下约 44px ——
             * 正好是 mini 档底栏「多一行」缺的那一截。
             *
             * 快捷键后来补上了（见上面那个 keydown effect），所以 tooltip 里现在
             * **写得**出快捷键了 —— 那句话当时留的备注就是「没配就不写，免得骗人」。
             */}
            <button
              type="button"
              className={s.iconBtn}
              disabled={!u.canUndo}
              onClick={u.undo}
              title="撤销（Ctrl+Z）"
              aria-label="撤销"
              aria-keyshortcuts="Control+Z"
            >
              <Icon name="undo" size={15} />
            </button>
            <button
              type="button"
              className={s.iconBtn}
              disabled={!u.canRedo}
              onClick={u.redo}
              title="重做（Ctrl+Y）"
              aria-label="重做"
              aria-keyshortcuts="Control+Y"
            >
              <Icon name="redo" size={15} />
            </button>
            <span className={s.sep} aria-hidden />
            <button type="button" className={s.textBtn} onClick={() => setAskRestore(true)}>
              恢复默认值
            </button>
          </div>

          <div className={s.footRight}>
            {/*
             * mini 档只留「N 处未保存」—— 「保存后会覆盖 xxx.toml」那半句实测 153px，
             * 加上左边那三个按钮正好把底栏顶成两行（作者：「下面改不改都多一行」）。
             * 半句话不丢：它本来就在保存确认弹层里说（「以下 N 处改动将写入 A1.toml」），
             * 而且这里挂了 title，悬停还能看全。
             */}
            <span className={s.state} title={stateText}>
              {stateText}
            </span>

            <button
              type="button"
              className={s.primary}
              disabled={u.dirtyCount === 0}
              title="保存修改（Ctrl+S）"
              aria-keyshortcuts="Control+S"
              onClick={() => setAskSave(true)}
            >
              保存修改
            </button>
          </div>
        </footer>

        {/* 切机型确认：有未保存改动时先问 */}
        {u.pendingCombo !== null && (
          <div className={s.scrim} role="presentation" onClick={u.cancelPending}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="丢弃改动"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>
                切换到 {u.pendingCombo.machineId} · {u.pendingCombo.versionId}？
              </h3>
              <p className={s.sheetText}>
                当前有 <b>{u.dirtyCount} 处未保存的改动</b>，切过去就丢了（修改历史也会清空）。
              </p>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={u.cancelPending}>
                  取消
                </button>
                <button type="button" className={s.danger} onClick={u.confirmPending}>
                  丢弃并切换
                </button>
              </div>
            </div>
          </div>
        )}

        {askRestore && (
          <div className={s.scrim} role="presentation" onClick={() => setAskRestore(false)}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="恢复默认值"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>恢复默认值</h3>
              <p className={s.sheetText}>
                会把全部参数改回出厂设置。它算<b>一次</b>改动，
                <b>按「撤销」可以整个退回来</b>；要落地还得按「保存修改」。
              </p>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={() => setAskRestore(false)}>
                  取消
                </button>
                <button
                  type="button"
                  className={s.danger}
                  onClick={() => {
                    u.restoreDefaults()
                    setAskRestore(false)
                  }}
                >
                  恢复默认值
                </button>
              </div>
            </div>
          </div>
        )}

        {askSave && (
          <div className={s.scrim} role="presentation" onClick={() => setAskSave(false)}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="保存确认"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>确认保存？</h3>
              <p className={s.sheetText}>
                以下 <b>{changedEntries.length}</b> 处改动将写入 <b>{fileLabel}</b>：
              </p>
              <ul className={s.changeList}>
                {changedEntries.slice(0, MAX_CONFIRM_ITEMS).map((e) => (
                  <li key={e.key} className={s.changeItem}>
                    <span className={s.changeLabel}>{e.label}</span>
                    <span className={s.changeArrow}>
                      {e.from} → {e.to}
                    </span>
                  </li>
                ))}
                {changedEntries.length > MAX_CONFIRM_ITEMS && (
                  <li className={s.changeMore}>
                    …… 还有 {changedEntries.length - MAX_CONFIRM_ITEMS} 项
                  </li>
                )}
              </ul>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={() => setAskSave(false)}>
                  取消
                </button>
                <button type="button" className={s.primary} onClick={confirmSave}>
                  确认保存
                </button>
              </div>
            </div>
          </div>
        )}
        {/*
          预设抽屉。挂在 FieldLayer 里、`.main` 之后 —— absolute 的定位父级是 `.main`，
          放在最后只为了 z-index 顺序（它要盖在卡片与底部操作条之上）。
        */}
        <PresetPickerDrawer
          open={pickerOpen}
          machines={u.machines}
          machineId={u.machineId}
          versionId={u.versionId}
          fileOf={u.fileOf}
          onPick={(m, v) => {
            /* 切 combo 走现成的那条路：未保存改动的确认弹层在 requestCombo 里 */
            u.requestCombo(m, v)
            closePicker()
          }}
          onClose={closePicker}
        />
      </FieldLayer>

      {/* 修改历史（悬浮态）挂在**页面根**（作者：「它是一个假的抽屉……
          应该紧贴右边」，而且它和底栏之间的断层也不要）—— 遮罩 inset:0 相对 .page
          通铺：贴住窗口右缘、通高盖过底栏，不再有「抽屉到 footer 为止」的截断。
          pinned 态仍留在 .main 里占位让内容让位。 */}
      {historyOpen && !pinned && (
        <HistoryDrawer
          open
          file={fileLabel}
          mode="float"
          canPin={canPin}
          width={historyDrag.width}
          drag={historyDrag}
          view={historyView}
          history={u.history}
          groups={u.historyByBatch}
          onView={setHistoryView}
          onMode={setHistoryMode}
          onClose={closeHistory}
          onRevert={u.revertEntry}
        />
      )}
    </div>
  )
}
