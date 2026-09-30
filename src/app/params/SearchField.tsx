/*
 * 常驻搜索框。
 *
 * 图标用用户端自己的内联 SVG；placeholder 说清能搜三样东西
 * （参数名 / 字段名 / TOML 键名）—— 这里的搜索真的会去比 `tomlKey`。
 *
 * 筛选本身在页面里做，而且**跨全部分类** —— 踩过「只搜当前页签得到 0 条，
 * 而那个字段其实在另一类里」。这个组件只负责收字。
 *
 * # 撤掉「命中 N 条」+ 点框内任何一处都进编辑态
 *
 * 作者两张截图指着框里那截绿字：「他这有时候太短了……命中 xxx 都可以不显示了吧」。
 * 量出来确实是它挤的：compact 档的框 132px，其中图标 14 + 内衬 20 + 缝 7 +
 * 清空键 18 + **「命中 14 条」约 75** —— 留给输入的那截只剩十几像素（截图上那个
 * 「支」几乎看不见）。撤掉之后同一只框留给输入约 91px。
 *
 * 那句计数也不是白撤：结果本来就在下面按卡列着，它回答的是「有几条」——
 * 而这个数在两种情况下还会骗人（上下带出来的行不算命中、一条都没命中时它又不出现）。
 * 一条都没命中仍有下面那句「没有匹配的参数，换个词试试」兜着。
 *
 * 另一半是点哪儿都能编辑：放大镜、框沿、右侧空白原来点了没反应，只有正好戳中
 * 文字那几个像素才进编辑态（作者的：「点击图标……也能变成编辑态」）。
 * 修法与 `NumberField` 的 `focusOnBoxClick` 一模一样（那一处的来由也是他提的）。
 *
 * # 点框就展开「最近搜索」
 *
 * 作者：「我点击搜索的时候没有展开搜索历史呀」—— 他一直在等这个东西。
 * （他那句「那我还需要搜索的记录」其实也是它，当时读成了「别把我打的字清掉」。）
 *
 * 历史词表由**页面**持有（落 localStorage，键收在 `src/api/storageKeys.ts`）：
 * 这个组件会被换掉 —— 窄窗时搜索框从分类条那一行搬进标题行，React 认的是两个不同的
 * 位置，状态留不住。
 *
 * 展开与收起一共五条路，各管一件事：
 *
 *   点框 / Ctrl+F 聚焦  开（`onMouseDown` 与 `onFocus` 都开 —— 有值时点框不换焦点）
 *   开始打字            关（`onChange`）—— 下拉压着结果列表是打扰
 *   Esc                 关，**不清空**；再按一次才清空（分两层，与页面那套一致）
 *   挑一条 / 点外面     关（点外面由 `FieldPopover` 管）
 *   离开框              把当时的词交给页面记一笔（`onCommit`）
 */

import { useRef, useState } from 'react'
import type { MutableRefObject } from 'react'
import type { Density } from '../../hooks/useDensity'
import FieldPopover from '../../components/field/FieldPopover'
import Icon from '../shell/icons'
import s from './SearchField.module.css'

interface Props {
  value: string
  density: Density
  onChange: (next: string) => void
  /**
   * 键盘入口（`Ctrl+F`）用：页面拿它把焦点放进这个框。
   *
   * 不给也行 —— 框自己那份 ref（下面 `ownRef`）照常工作。给了就把两个都接上，
   * 因为「清空」那颗按钮要自己那份才能把焦点放回框里。
   */
  inputRef?: MutableRefObject<HTMLInputElement | null>
  /** 最近搜过的词，新的在前。空数组 / 不给 = 没有下拉可展 */
  history?: string[]
  /** 离开框（blur）或按回车：把当时的词交给页面记一笔 */
  onCommit?: (term: string) => void
  /** 从下拉里挑了一条 */
  onPick?: (term: string) => void
  /** 把某一条从历史里删掉 */
  onForget?: (term: string) => void
}

/** placeholder 随宽度换文案：compact 档的框只有 132px，写全必然切在半个字上 */
const PLACEHOLDER: Record<Density, string> = {
  ultra: '搜索参数名称、字段名或 TOML 键名…',
  wide: '搜索参数名称或字段名…',
  compact: '搜索参数…',
  mini: '搜索…',
}

export default function SearchField({
  value,
  density,
  onChange,
  inputRef,
  history,
  onCommit,
  onPick,
  onForget,
}: Props) {
  /* 类型里带上 `| null` 才拿到可写的那个 ref 形态（`useRef<T>(null)` 给的是只读的） */
  const ownRef = useRef<HTMLInputElement | null>(null)
  /** 下拉的锚点：整个框（点图标、点框沿都算在框里，浮层的「点外面」判据要用它） */
  const fieldRef = useRef<HTMLDivElement | null>(null)
  const [histOpen, setHistOpen] = useState(false)

  const hasHistory = history !== undefined && history.length > 0
  const openHist = () => {
    if (hasHistory) setHistOpen(true)
  }

  /* 自己有那份，外面要的那份也给一份（两个 ref 指向同一个 input） */
  const attach = (el: HTMLInputElement | null) => {
    ownRef.current = el
    if (inputRef !== undefined) inputRef.current = el
  }

  return (
    <div
      ref={fieldRef}
      className={s.field}
      /*
       * 点框内任何一处（放大镜、原来的计数位置、框沿、右侧空白）都进编辑态。
       * 与 NumberField 的 focusOnBoxClick 同一套写法，「清空」那颗按钮交给它自己
       * （它清完本来就会把焦点放回来）。事件挂在 mousedown 上，理由也同那边：
       * click 太晚 —— 浏览器先把焦点挪走，再抢回来会闪一下。
       */
      onMouseDown={(e) => {
        const t = e.target as HTMLElement
        if (t.closest('button') !== null) return
        const input = ownRef.current
        if (input === null || t === input) {
          openHist()
          return
        }
        /* 必须 preventDefault：否则浏览器的默认行为接着把焦点挪到别处，刚抢来的焦点当场丢 */
        e.preventDefault()
        input.focus()
        openHist()
      }}
    >
      <Icon name="search" size={14} className={s.icon} />
      <input
        ref={attach}
        className={s.input}
        value={value}
        placeholder={PLACEHOLDER[density]}
        aria-label="搜索参数"
        aria-keyshortcuts="Control+F"
        title="搜索参数（Ctrl+F）"
        onFocus={openHist}
        onBlur={() => onCommit?.(value)}
        onChange={(e) => {
          /* 一开始打字就把下拉收起来，别压着结果 */
          setHistOpen(false)
          onChange(e.target.value)
        }}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            /* 分两层：先把展开的历史收起来，再按才清空（清空仍不失焦） */
            if (histOpen) {
              e.preventDefault()
              setHistOpen(false)
              return
            }
            if (value !== '') {
              e.preventDefault()
              onChange('')
            }
            return
          }
          if (e.key === 'Enter') {
            /* 回车 = 「这次就这么搜了」：记一笔 */
            e.preventDefault()
            onCommit?.(value)
            setHistOpen(false)
          }
        }}
      />
      {value !== '' && (
        <button
          type="button"
          className={s.clear}
          aria-label="清空搜索"
          onClick={() => {
            onChange('')
            ownRef.current?.focus()
          }}
        >
          {/* 用它而不是文字「×」：那个字形在字身框里本来就靠下，作者一眼看出来了 */}
          <Icon name="close" size={11} />
        </button>
      )}

      {/*
        最近搜索。挂在 FieldLayer 的浮层里（`FieldPopover`），所以窄窗那个
        `max-width: 160px` 的槽裁不到它，位置也不用自己算。
      */}
      {histOpen && hasHistory && (
        <FieldPopover anchor={fieldRef.current} onClose={() => setHistOpen(false)}>
          <ul className={s.hist}>
            <li className={s.histHead} aria-hidden>
              最近搜索
            </li>
            {history.map((term) => (
              <li key={term} className={s.histRow}>
                <button
                  type="button"
                  className={s.histTerm}
                  /*
                   * 按下不夺焦点：否则框先 blur，把手上这半截词记进历史，
                   * 再轮到这次「挑一条」—— 那就凭空多了一条没搜过的记录。
                   */
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => {
                    onPick?.(term)
                    setHistOpen(false)
                  }}
                >
                  {term}
                </button>
                <button
                  type="button"
                  className={s.histForget}
                  aria-label={`忘掉「${term}」`}
                  title="忘掉这一条"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => onForget?.(term)}
                >
                  <Icon name="close" size={11} />
                </button>
              </li>
            ))}
          </ul>
        </FieldPopover>
      )}
    </div>
  )
}
