/*
 * 一级分类：图标胶囊组。
 *
 * 改了两处：
 *   1. 外部 `components/Icon` 换成用户端自己的 `icons`
 *   2. **计数读 `ParamTab.count`，不自己数** —— 后端那份已经按 machineFilter 与废弃过滤过，
 *      前端再数一遍只会在两者不一致时说不清谁对。原样照抄旧版那个 `counts` 入参
 *      等于把别的稿的假设也复制进来。
 *
 * 红点 = 这一类里有未保存的改动。旧程序就是这么提示的，比在分类名后面写数字安静。
 */

import type { CSSProperties, RefObject } from 'react'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { ParamTab } from '../../api'
import Icon from '../shell/icons'
import type { IconName } from '../shell/icons'
import s from './CategoryPills.module.css'

interface Props {
  /** 真注册表的六个分类。顺序、名字、条数全部来自它 */
  tabs: ParamTab[]
  current: string
  /** 哪些分类里有未保存的改动 */
  dirty: Set<string>
  onSelect: (id: string) => void
}

/** 分类 id → 图标。查不到就退回一个通用图标，不画空方块 */
const TAB_ICONS: Record<string, IconName> = {
  offset: 'catOffset',
  wiping: 'catWiping',
  fan: 'catFan',
  glue: 'catGlue',
  gcode: 'catSwitch',
  advanced: 'catMore',
}

/**
 * 分类**中文名** → 图标（兜底）：老包（`tabId` 还没进契约时同步下来的那份）
 * 页签 id 是中文名，按 id 查表六个全落空 —— 图标全塌成同一个。
 * 新包有 `tabId` 走上面那张表；这一条只为了让旧包照跑。
 */
const TAB_ICONS_BY_LABEL: Record<string, IconName> = {
  偏移: 'catOffset',
  擦料: 'catWiping',
  风扇: 'catFan',
  涂胶: 'catGlue',
  切换: 'catSwitch',
  更多: 'catMore',
}

const iconOf = (id: string, label: string): IconName =>
  TAB_ICONS[id] ?? TAB_ICONS_BY_LABEL[label] ?? 'catMore'

/**
 * 这一行真正留给分类条多少宽 —— **按 DOM 量，不按密度档猜**。
 *
 * 算法：取**行所在的那一列**的内容宽，减掉这一行自己的左右内衬，
 * 再减掉「同样站在这行里的搜索框 + 缝」。搜索框窄窗时会搬到头部那一行
 * （`PageParams` 的 `searchInHead`），那一刻它就不在这行了 —— 于是这一块的
 * 可用宽自动变大，正好解释为什么 400 窗口的余额反而比某些更宽的窗口更宽。
 * 这条逻辑以前是写死在 `[data-density]` 两档里的，于是 720 那一行明明空着
 * 338px，条数照样被藏了。
 *
 * **为什么不量这一行自己**：`.bar` 是 `flex-shrink: 0`，内容一宽它就跟着变宽，
 * 于是「量到更宽 → 画得更大 → 又量到更宽」—— 第一批实现就是这么转起来的
 * （页面直接卡住，`page.reload` 连 load 事件都等不到）。上一层 `.left` 是
 * `flex: 1; min-width: 0`，宽度只由外层给的自由空间决定，不被内容顶开，
 * 所以拿它当基准。
 *
 * 为什么 effect 不写依赖数组：搜索框换不换行是拆 / 建子节点，`ResizeObserver`
 * **不会响**（这一行的宽度没变），所以每次渲染都补量一遍。
 * `setAvail` 值没变时 React 自己会跳过重渲染，不会转起来。
 */
function useAvailWidth(wrapRef: RefObject<HTMLDivElement>): number | null {
  const [avail, setAvail] = useState<number | null>(null)
  const roRef = useRef<ResizeObserver | null>(null)

  const measure = useCallback(() => {
    const row = wrapRef.current?.parentElement
    const host = row?.parentElement
    if (row === null || row === undefined || host === null || host === undefined) return

    const rowCs = getComputedStyle(row)
    const hostCs = getComputedStyle(host)
    const vpad = (cs: CSSStyleDeclaration) => [
      Number.parseFloat(cs.paddingLeft) || 0,
      Number.parseFloat(cs.paddingRight) || 0,
    ]

    const [hostL, hostR] = vpad(hostCs)
    const [rowL, rowR] = vpad(rowCs)
    /* 这一行的内容宽 = 列的内容宽 − 这一行自己的内衬（含边框也没漏：offsetWidth − clientWidth） */
    const hostInner = (host instanceof HTMLElement ? host.clientWidth : 0) - hostL - hostR
    const rowChrome =
      (row instanceof HTMLElement ? row.offsetWidth : 0) -
      ((row instanceof HTMLElement ? row.clientWidth : 0) - rowL - rowR)

    const gap = Number.parseFloat(rowCs.columnGap || rowCs.gap || '0') || 0
    /*
     * 预留的是**这一行里除我以外还有谁**的宽度（含它们之间的缝）。
     *
     * 第一版写的是「预留搜索框，它搬到头部也算」：那会造出一段反直觉的暴跌 ——
     * 实测卡片区 438 时可用宽 396，到 478 反而只剩 268（搜索框回到行内），
     * 于是**窗口更宽、条数反而被收掉**，而且那一段还放不下（分类条要横滚 11px）。
     *
     * 后来头部搜索框是 `flex: 1 1 48px`，它自己会把多出来的宽度吃光，
     * 于是 400–460 四档的可用宽**卡在 260 不动** —— 又是一种「不跟着窗口长」。
     *
     * 现在直接问这一行：搜一遍 `.bar` 里除自己以外的孩子（搜索框、`mini` 档搬下来的
     * 「修改历史」），把它们的宽度加起来。谁在这一行就预留谁，一行的成员换了也自动跟上。
     */
    const others = [...row.children].filter((el) => el !== wrapRef.current)
    const taken =
      others.reduce(
        (sum, el) => sum + (el instanceof HTMLElement ? el.getBoundingClientRect().width : 0),
        0,
      ) + gap * others.length

    setAvail((prev) => {
      const next = Math.max(0, Math.round(hostInner - rowChrome - taken))
      return prev !== null && Math.abs(prev - next) < 0.5 ? prev : next
    })
  }, [wrapRef])

  useEffect(() => {
    measure()
    const row = wrapRef.current?.parentElement
    if (row === null || row === undefined) return
    roRef.current?.disconnect()
    const ro = new ResizeObserver(measure)
    ro.observe(row)
    roRef.current = ro
    return () => {
      ro.disconnect()
      roRef.current = null
    }
  })

  return avail
}

export default function CategoryPills({ tabs, current, dirty, onSelect }: Props) {
  const at = tabs.findIndex((t) => t.id === current)
  const label = tabs[at]?.label ?? ''
  const wrapRef = useRef<HTMLDivElement>(null)
  const avail = useAvailWidth(wrapRef)

  const move = (dir: 1 | -1) => {
    if (tabs.length === 0) return
    const next = tabs[(at + dir + tabs.length) % tabs.length]
    if (next !== undefined) onSelect(next.id)
  }

  return (
    <div
      className={s.wrap}
      ref={wrapRef}
      style={avail === null ? undefined : ({ '--avail': `${avail}px` } as CSSProperties)}
    >
      <div
        className={s.group}
        role="tablist"
        aria-label="参数分类"
        onKeyDown={(e) => {
          if (e.key === 'ArrowRight') {
            e.preventDefault()
            move(1)
          } else if (e.key === 'ArrowLeft') {
            e.preventDefault()
            move(-1)
          }
        }}
      >
        {tabs.map((t) => {
          const on = t.id === current
          return (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={on}
              aria-current={on ? 'true' : undefined}
              tabIndex={on ? 0 : -1}
              className={on ? s.pillOn : s.pill}
              title={`${t.label} · ${t.count} 项`}
              onClick={() => onSelect(t.id)}
            >
              <Icon name={iconOf(t.id, t.label)} size={20} className={s.icon} />
              {/* 条数在最窄的一档会被 CSS 收掉 —— 名字（右边那截）优先级更高，留着 */}
              <span className={s.num}>{t.count}</span>
              {dirty.has(t.id) && <i className={s.dot} aria-label="这一类有未保存的改动" />}
            </button>
          )
        })}
      </div>

      <span className={s.sep} aria-hidden />
      {/*
       * 条外只留**分类名**，不再跟「N 项」（作者圈着那个「3 项」：
       * 「不要这个数字，占位置了，左侧 tab 显示不完整」）。
       *
       * 两个数字说的是同一件事，而条外这个更占地方：实测 400 窗口那一行只剩
       * 0.8px 余量，名字后面挂上「22 项」就宽出 5px —— 分类条是唯一能让的一件
       * （`overflow-x: auto`），只能横着滚，最后一个胶囊被啃掉一角。条数本来就在
       * 每个胶囊里（够宽时）和它的 tooltip 里（任何宽度），条外再写一遍是重复。
       */}
      <strong className={s.current}>{label}</strong>
    </div>
  )
}
