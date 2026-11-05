/*
 * 分组下拉（2026-11-02）。**编辑定义框里的所有下拉都用它**。
 *
 * # 它替掉了什么
 *
 * 「编辑定义」里原来有几处是原生 `<select>`：
 *
 *   · 值类型 `valueType`（float / int / bool / string）—— 5 项
 *   · 控件 `uiComponent`（number / slider / segmented / select / text / gcode）—— 6 项
 *   · 属于 `parentKey` —— **几十项**，一屏放不下，长得像一张表而不是一个下拉
 *   · 前置条件 `showWhen.key` —— 同样几十项
 *   · 档位 `showWhen.op`
 *
 * 系统画的弹层：改不了字号、改不了勾选样式、**分不了组**（`optgroup` 只是
 * 一行灰字，挂不了东西），`parentKey` 那种长名单还会直接顶到屏幕外。
 *
 * # 与「所属分组」同一副长相
 *
 * 作者：「这些下拉菜单都复用所属分组那样的，有层级的，分类的，自制的」。
 * 所以面板结构照 `SectionList`：**分组标题一行 + 组内若干行**，行有选中圆点、
 * 悬停整行亮、名字长就省略。触发器也照 `SectionPicker`：左边值、右边小箭头，
 * 值里可以带一截浅色的旁注（`by`）。
 *
 * 两者是同一套手艺的两次落地，**不是两次实现**：这里没有改名 / 上下移那些
 * 分组专有的动作，多出来的只有"分组标题"这一层。
 *
 * # 一条纪律：`disabled` 必须给理由
 *
 * 选项被禁时不发问号 —— 理由写在 `note` 里（行右侧那截小字），鼠标悬停也能看见。
 *
 * # 空值
 *
 * `value === ''` 表示"不指向谁"（顶层 / 无条件）。这类选项就是一张
 * **不带标题的组**（`GroupedOption` 的 `title` 为 `null`），排在最前面。
 */

import { useEffect, useRef, useState } from 'react'

import FieldPopover from '../c14/field/FieldPopover'
import s from './GroupPicker.module.css'

export interface GroupedOption {
  value: string
  /** 显示名（中文） */
  label: string
  /** 名字后面那截浅色旁注（键名 / 页签名 …）。没有就不画 */
  by?: string
  /** 行右侧的小字。**禁用的理由也写这儿**，不许只灰着不解释 */
  note?: string
  disabled?: boolean
  /**
   * 行尾常驻的上 / 下两颗箭头（2026-11-02，为「组内顺序」加的）。
   *
   * 作者：「我希望也是下拉菜单然后可以右侧常驻复用箭头，上下点击切换排序，
   * 就在下拉菜单里面操作」—— 于是排序不是"关掉面板去点另一处"，而是**在这个
   * 列表里直接调**。箭头**常显不藏**（藏着就没人知道能调），到头那颗灰掉。
   *
   * 给了这个字段的行：点箭头 = 挪位置，**不关面板、不算选中**（`stopPropagation`）；
   * 点行的其它地方照旧 = 选中。
   */
  move?: { up: boolean; down: boolean; onMove: (dir: -1 | 1) => void }
}

export interface GroupedGroup {
  /** 组的标题（「全部」「偏移」「擦料」…）。`null` = 不画标题，直接排选项 */
  title: string | null
  options: GroupedOption[]
}

interface Props {
  /** 无障碍名（「所属分组」「值类型」…） */
  label: string
  groups: GroupedGroup[]
  value: string
  onChange: (next: string) => void
  /** 认不出当前值时的兜底文字（比如注册表里查不到的那个键） */
  unknown?: string
  disabled?: boolean
  /** 面板宽度。长名单（`parentKey`）给宽一点，短枚举窄一点 */
  width?: number
}

/**
 * 触发器上那枚箭头：`up` 只表示"面板开着没"（开着就朝上）。
 *
 * ★ 它**只给触发器用**。行里那两颗上下箭头必须用 `MoveArrow` —— 这里用的是
 * `s.caretUp`，而那一类带着 `color: var(--text-2)` 与转动，混用会串（2026-11-02
 * 的 bug：行里两颗箭头都是朝下、且"上"那颗反而亮，就是它把触发器的类套上去了）。
 */
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

/**
 * 行尾那两颗排序箭头（2026-11-02）。与触发器那枚**不共用类**：
 * 颜色跟按钮走（`currentColor`），朝上由 `transform` 单独转，不掺触发器的开合态。
 */
function MoveArrow({ up }: { up: boolean }) {
  return (
    <svg
      className={s.moveIcon}
      data-up={up ? 'true' : undefined}
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

/** 拍平之后当前那一项 —— 触发器要拿它画「名字 + 旁注」 */
function findOption(groups: GroupedGroup[], value: string): GroupedOption | null {
  for (const g of groups) {
    const hit = g.options.find((o) => o.value === value)
    if (hit !== undefined) return hit
  }
  return null
}

export default function GroupPicker({
  label,
  groups,
  value,
  onChange,
  unknown,
  disabled = false,
  width = 320,
}: Props) {
  const [open, setOpen] = useState(false)
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null)
  const closeRef = useRef<HTMLButtonElement | null>(null)

  const current = findOption(groups, value)

  /*
   * 关面板不抢焦点（与 `SectionPicker` 同一条，2026-11-02）：
   * 收起时同时 `focus()` 会让面板消失与焦点回跳挤在同一拍，界面闪一下。
   */
  const close = () => setOpen(false)

  /*
   * 选完收起并把焦点还给触发器 —— 但这要**晚一拍**：
   * 同一拍里收起会让面板在 `click` 之前消失，选中态与触发器文字一起跳。
   */
  const pick = (next: string) => {
    onChange(next)
    setOpen(false)
    window.setTimeout(() => closeRef.current?.focus(), 0)
  }

  /* 面板开着时把选中的那一行滚进视野 —— `parentKey` 那种几十项的名单，不看位置会开在最上面 */
  useEffect(() => {
    if (!open) return
    const box = document.getElementById(`gp-${label}`)
    box?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
  }, [open, label])

  return (
    <span className={s.wrap}>
      <button
        ref={(el) => {
          setAnchor(el)
          closeRef.current = el
        }}
        type="button"
        className={disabled ? s.triggerOff : s.trigger}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => (open ? close() : setOpen(true))}
      >
        <span className={s.text}>
          {current === null ? (
            (unknown ?? '选一个')
          ) : (
            <>
              {current.label}
              {current.by !== undefined && <span className={s.by}>{current.by}</span>}
            </>
          )}
        </span>
        <Chevron up={open} />
      </button>

      {open && (
        <FieldPopover anchor={anchor} matchWidth onClose={close}>
          <div className={s.panel} id={`gp-${label}`} style={{ width }} role="listbox" aria-label={label}>
            {groups.map((g, gi) => (
              <div key={`${g.title ?? ''}-${gi}`} className={s.group}>
                {g.title !== null && <div className={s.groupTitle}>{g.title}</div>}
                {g.options.map((o) => {
                  const on = o.value === value
                  /*
                   * 有 `move` 的行不能是 `<button>` 包整行 —— 里面还有两颗按钮，
                   * 按钮套按钮是非法结构（React 会告警、读屏器也读不出来）。
                   * 这一支改成 `<div role="option">` + 内部一枚"选中"按钮 + 两枚箭头。
                   */
                  if (o.move !== undefined) {
                    return (
                      <div
                        key={o.value}
                        role="option"
                        aria-selected={on}
                        className={on ? s.rowOn : s.row}
                      >
                        <button
                          type="button"
                          className={s.pickArea}
                          aria-label={`选 ${o.label}`}
                          onClick={() => pick(o.value)}
                        >
                          <span className={s.dot} aria-hidden />
                          <span className={s.label}>{o.label}</span>
                          {o.by !== undefined && <span className={s.by}>{o.by}</span>}
                        </button>
                        {/* 箭头常驻：调序就在这里做，不关面板也不改选中 */}
                        <span className={s.moves}>
                          <button
                            type="button"
                            className={s.moveBtn}
                            disabled={!o.move.up}
                            title={o.move.up ? '往上挪一位' : '已经在最前面了'}
                            aria-label={`${o.label} 上移`}
                            onClick={(e) => {
                              e.stopPropagation()
                              o.move?.onMove(-1)
                            }}
                          >
                            <MoveArrow up />
                          </button>
                          <button
                            type="button"
                            className={s.moveBtn}
                            disabled={!o.move.down}
                            title={o.move.down ? '往下挪一位' : '已经在最后面了'}
                            aria-label={`${o.label} 下移`}
                            onClick={(e) => {
                              e.stopPropagation()
                              o.move?.onMove(1)
                            }}
                          >
                            <MoveArrow up={false} />
                          </button>
                        </span>
                      </div>
                    )
                  }
                  return (
                    <button
                      key={o.value}
                      type="button"
                      role="option"
                      aria-selected={on}
                      disabled={o.disabled === true}
                      className={on ? s.rowOn : s.row}
                      title={o.disabled === true ? o.note : undefined}
                      onClick={() => pick(o.value)}
                    >
                      <span className={s.dot} aria-hidden />
                      <span className={s.label}>{o.label}</span>
                      {o.by !== undefined && <span className={s.by}>{o.by}</span>}
                      {o.note !== undefined && <span className={s.note}>{o.note}</span>}
                    </button>
                  )
                })}
              </div>
            ))}
          </div>
        </FieldPopover>
      )}
    </span>
  )
}
