/*
 * 单行文本（#33）。
 *
 * 两种形态，由 `clickToEdit` 决定：
 *
 *   常驻输入框（参数页一行一控件）—— 一行就一个控件，框一直在那儿最省一次点击
 *   点开才编辑（密表的格子）—— 一屏几十行，几十个常驻输入框会把表变成一片框；
 *                              平时只需要看，需要改的时候点一下
 *
 * 两种形态共用同一套提交约定：Enter 提交、Esc 放弃、失焦提交。
 * #32 的密表内联输入框就是这套，这一稿只是把它从行组件里搬出来变成共用件。
 */

import { useEffect, useRef, useState } from 'react'
import s from './TextField.module.css'

interface Props {
  value: string
  label: string
  disabled?: boolean
  invalid?: boolean
  /**
   * 编辑时就报的校验（C06）。收的是**「草稿 → 原因或 null」这个函数**，不是一个算好的
   * 字符串 —— 草稿在组件内部，父项看不到它，拿不到「正在打的这个值」就报不准。
   * 给了原因就在框下显示一行红字，并且**不许提交** —— 作者的要求是「不是改了它就可以，
   * 而是它是不是一套的」，要在输入的时候就说，不是保存之后才炸。
   * 失焦等于放弃（退回原值），要改就继续打字或回车。
   */
  validate?: (draft: string) => string | null
  /** 密表用：平时是一行字，点一下才变输入框 */
  clickToEdit?: boolean
  onChange: (next: string) => void
}

export default function TextField({
  value,
  label,
  disabled = false,
  invalid = false,
  validate,
  clickToEdit = false,
  onChange,
}: Props) {
  const [editing, setEditing] = useState(!clickToEdit)
  const [draft, setDraft] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setDraft(value)
  }, [value])

  /* 点开就选中全部：要改的多半是整段替换，不是在原文中间插字 */
  useEffect(() => {
    if (editing && clickToEdit) inputRef.current?.select()
  }, [clickToEdit, editing])

  /** 每次渲染按当前草稿算一遍 —— 父项不用管草稿，只提供规则 */
  const err = validate ? validate(draft) : null

  const commit = () => {
    if (clickToEdit) setEditing(false)
    /*
     * 有校验原因就不许提交（C06）：退回原值，改到不冲突为止。
     * 这不是「静默夹取」—— 红字一直摆在框下，人看得见为什么没进去。
     */
    if (err) {
      setDraft(value)
      return
    }
    if (draft !== value) onChange(draft)
  }

  const cancel = () => {
    setDraft(value)
    if (clickToEdit) setEditing(false)
  }

  if (clickToEdit && !editing) {
    return (
      <button
        type="button"
        className={s.view}
        data-invalid={invalid}
        aria-label={label}
        title={value === '' ? '空' : value}
        disabled={disabled}
        onClick={() => {
          setDraft(value)
          setEditing(true)
        }}
      >
        {value === '' ? <span className={s.empty}>空</span> : value}
      </button>
    )
  }

  return (
    <>
      <input
        ref={inputRef}
        className={s.input}
        data-invalid={invalid}
        data-error={err ? true : undefined}
        value={draft}
        aria-label={label}
        aria-invalid={invalid || err ? true : undefined}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') commit()
          else if (e.key === 'Escape') cancel()
        }}
      />
      {/*
        校验原因**只在编辑态显示** —— 平时那一行是纯文本，不该挂着一句红字。
        `clickToEdit` 收起之后错误也一起收起来，因为它描述的是「你正在打的这个值」。
      */}
      {err && <span className={s.err}>{err}</span>}
    </>
  )
}
