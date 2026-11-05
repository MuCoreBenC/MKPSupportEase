/*
 * 分组清单：按页签分块，**点名字就改名、点行就切换、上下箭头挪位置**（2026-11-01 第四稿）。
 *
 * # 三稿是怎么收敛成这一稿的
 *
 *   ① 原生 `<select>` —— 挂不了按钮，页签只是死标签；
 *   ② 卡头就地改名 + 卡头箭头 —— 作者：「容易不小心点到的就是排序」；
 *   ③ 全部收进右侧抽屉，抽屉里还要再点铅笔才开编辑器 ——
 *      作者：「我是希望这个上面这个也可以点了去改名字」「选择是哪一个，它哪一个就亮一点」；
 *
 * 所以现在：**名字本身就是编辑入口**（点一下就地变输入框），整行仍然是一个可点的
 * 目标（点行的空白处 = 选中这一组），选中态铺一层背景 + 左边一道竖标。
 *
 * # 顺序与「模态框里那个组内顺序」同一条口径
 *
 * 抽屉里那颗 ↑↓ 与参数定义框里「组内顺序」是同一种东西：**只挪次序**，一次一位，
 * 到头就禁用。落不落盘由宿主决定（下面那节）。
 *
 * # 面板本身不落盘
 *
 * 只发四种手势：`onPick` / `onRename` / `onMove` / `onAdd`。写不写、写去哪，
 * 由调用方按自己的语义决定：
 *
 *   · 参数定义框里 —— 都是**草稿**，跟着这一笔定义一起交；
 *   · 分组抽屉里 —— 改一条应用一条（进草稿），真正落盘仍归右上角那颗「保存」。
 *
 * 改名时输入框是面板自己的局部状态：回车 / 失焦落一次，Esc 撤回。
 */
import { useEffect, useRef, useState } from 'react'

import type { SectionTarget } from '../api'
import s from './SectionList.module.css'

/** 一个页签下的分组（与后端 `layout_sections` 的顺序一致） */
export interface SectionGroup {
  tabId: string
  tabLabel: string
  sections: SectionTarget[]
}

interface Props {
  groups: SectionGroup[]
  /** 当前选中的分组 id（可能是还没落盘的那个新分组） */
  value: string | null
  /**
   * 画不画「选中」那一颗圆点、以及选中态那层底色。
   *
   * 下拉里必画（要表达"现在在哪一组"）；抽屉里也画 —— 作者 2026-11-01：
   * 「选择是哪一个，它哪一个就亮一点……顺便点击右侧的抽屉去切换，就跟在左侧
   * 原本那个点击切换的一样的效果」。抽屉里点圆点 = 换一组看。
   */
  selectable?: boolean
  onPick: (id: string) => void
  /** 改名字。**只收非空且真的变了**的名字（判在这里，不各写一遍） */
  onRename: (id: string, label: string) => void
  /** 上移 / 下移一位（-1 / +1）。到头了由这里按 `disabled` 挡住 */
  onMove?: (id: string, dir: -1 | 1) => void
  /** 在这个页签下新建一个分组 */
  onAdd?: (tabId: string, label: string) => void
  /** 这一行是不是还没落盘的新分组（显示「待保存」） */
  isPending?: (id: string) => boolean
  /** 面板底下的那一句（两处口径不同：下拉说"跟着这次保存"，抽屉说"已应用"） */
  hint?: string
}

/** 分组名字的上限 —— 与后端 `ensure_section` 同一把尺子 */
const LABEL_MAX = 24

/** 改名那颗笔（与参数台「编辑定义」同一支，12px 描边线） */
function Pencil() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
    </svg>
  )
}

function Chevron({ up }: { up: boolean }) {
  return (
    <svg
      className={up ? s.caretUp : s.caret}
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

export default function SectionList({
  groups,
  value,
  selectable = false,
  onPick,
  onRename,
  onMove,
  onAdd,
  isPending,
  hint,
}: Props) {
  /** 正在改名的那一组（`null` = 没在改） */
  const [editing, setEditing] = useState<string | null>(null)
  const [text, setText] = useState('')
  /** 正在哪个页签下写新名字（`null` = 没在写） */
  const [adding, setAdding] = useState<string | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const addRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (editing !== null) inputRef.current?.select()
  }, [editing])
  useEffect(() => {
    if (adding !== null) addRef.current?.focus()
  }, [adding])

  const beginRename = (sec: SectionTarget) => {
    setEditing(sec.id)
    setText(sec.label)
  }

  /** 落一次改名：空名 / 没改都当没发生（不弹错，也不写盘） */
  const commitRename = () => {
    const id = editing
    const next = text.trim()
    setEditing(null)
    if (id === null || next === '') return
    const sec = groups.flatMap((g) => g.sections).find((x) => x.id === id)
    if (sec === undefined || sec.label === next) return
    onRename(id, next)
  }

  const submitAdd = (tabId: string) => {
    const label = text.trim()
    setAdding(null)
    if (label === '' || onAdd === undefined) return
    onAdd(tabId, label)
  }

  return (
    <div className={s.panel}>
      {groups.map((g) => (
        <div key={g.tabId} className={s.group}>
          <div className={s.tabRow}>
            <span className={s.tabLabel}>{g.tabLabel}</span>
            {onAdd !== undefined && (
              <button
                type="button"
                className={s.addBtn}
                title={`在「${g.tabLabel}」下新建一个分组`}
                onClick={() => {
                  setAdding(adding === g.tabId ? null : g.tabId)
                  setText('')
                }}
              >
                ＋ 新增分组
              </button>
            )}
          </div>

          {adding === g.tabId && (
            <div className={s.addRow}>
              <input
                ref={addRef}
                className={s.addInput}
                value={text}
                maxLength={LABEL_MAX}
                placeholder="新分组名称"
                aria-label="新分组名称"
                onChange={(e) => setText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') submitAdd(g.tabId)
                  else if (e.key === 'Escape') setAdding(null)
                }}
              />
              <button
                type="button"
                className={s.addOk}
                disabled={text.trim() === ''}
                onClick={() => submitAdd(g.tabId)}
              >
                添加
              </button>
              <button type="button" className={s.addCancel} onClick={() => setAdding(null)}>
                取消
              </button>
            </div>
          )}

          {g.sections.map((sec, i) => {
            const on = selectable && sec.id === value
            const pending = isPending?.(sec.id) === true
            if (editing === sec.id) {
              return (
                <div key={sec.id} className={s.editRow} data-current={on ? 'true' : undefined}>
                  <input
                    ref={inputRef}
                    className={s.editInput}
                    value={text}
                    maxLength={LABEL_MAX}
                    aria-label="分组名"
                    onChange={(e) => setText(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') commitRename()
                      else if (e.key === 'Escape') setEditing(null)
                    }}
                    onBlur={commitRename}
                  />
                </div>
              )
            }
            return (
              <div
                key={sec.id}
                className={on ? s.secOn : s.secRow}
                data-current={on ? 'true' : undefined}
              >
                {/*
                  * **整行 = 切换**（2026-11-01 第五稿，作者：「点击了就是切换，而不是说
                  * 点击的是改名字。你把那个改名字加一支笔的图标放在右侧，点击了笔才是
                  * 改名字」）。名字不再是编辑入口 —— 一颗按钮把整行吃掉，点到哪儿都
                  * 是"选这一组"，改名只认右边那颗笔。
                  */}
                <button
                  type="button"
                  className={s.sec}
                  aria-current={on ? 'true' : undefined}
                  onClick={() => onPick(sec.id)}
                >
                  {selectable && <span className={s.dot} aria-hidden />}
                  <span className={s.secLabel}>{sec.label}</span>
                  {sec.component !== null && (
                    <span className={s.badge} title="这一组是整块组件卡（客户端那一页是画出来的），参数搬不进来">
                      组件卡
                    </span>
                  )}
                  {pending && <span className={s.badge}>待保存</span>}
                </button>
                {/* 组件卡与普通分组同一套操作：笔改名、箭头换位（作者 2026-11-01：
                    「组件卡跟别的一样呀，只是它的样子不一样而已」） */}
                <span className={s.ops}>
                  <button
                    type="button"
                    className={s.op}
                    title="改名字（回车确定 · Esc 撤回）"
                    aria-label={`${sec.label} 改名`}
                    onClick={() => beginRename(sec)}
                  >
                    <Pencil />
                  </button>
                  {onMove !== undefined && (
                    <>
                      <button
                        type="button"
                        className={s.op}
                        disabled={i === 0}
                        title={i === 0 ? '已经在最前面了' : '往上挪一位'}
                        aria-label={`${sec.label} 上移`}
                        onClick={() => onMove(sec.id, -1)}
                      >
                        <Chevron up />
                      </button>
                      <button
                        type="button"
                        className={s.op}
                        disabled={i === g.sections.length - 1}
                        title={i === g.sections.length - 1 ? '已经在最后面了' : '往下挪一位'}
                        aria-label={`${sec.label} 下移`}
                        onClick={() => onMove(sec.id, 1)}
                      >
                        <Chevron up={false} />
                      </button>
                    </>
                  )}
                </span>
              </div>
            )
          })}
        </div>
      ))}
      {hint !== undefined && <p className={s.hint}>{hint}</p>}
    </div>
  )
}
