/**
 * 一个字段的真控件。
 *
 * 74 行 × 最多 16 列全做成受控输入 = 1000 多个 state、1000 多次 re-render，
 * 而且同屏一片输入框根本没法读。所以文本框/下拉**默认纯文本，点中了才升级成控件**。
 *
 * # 开关是例外：它常驻
 *
 * 开关**没有本地 state**（值来自 `cell.raw`，点一下就提交），
 * 所以上面那个顾虑对它不成立。让它常驻的好处是改一个布尔项只要一下 ——
 * 原来要先点格子选中、再点一下切换，而中间那一下还会触发整表位移。
 *
 * 误触的代价是点一下「撤销」（一次手势一条撤销早就在了）。
 *
 * # 三条纪律
 *
 * 1. **回填用 `raw` 不用格式化过的文本。** 格子上显示的是「圆盘擦拭」，
 *    而控件要的是 `"disk"` —— 拿显示文本回填会把值改成一个上游不认识的字符串。
 * 2. **失焦/回车才提交**，不是每敲一个字符提交一次。每字符一次会让撤销栈里
 *    塞满「改了 1」「改了 12」「改了 123」。
 * 3. **所有控件与文本行同高**（`--w-cell-h`）：高度一变，后面所有行就上下跳。
 */
import { useEffect, useState } from 'react'

import type { ParamView } from '../api'

/**
 * 常驻开关。**两个状态尺寸完全一样** —— 原来这里是一个写着「开」/「关」的文字按钮，
 * 两个字宽度还不同，切一下这一列就变宽。
 *
 * 被上级条件关着时渲染出来但**不翻转**：点击交给调用方去解释「是谁把它关了」
 */
export function Switch({
  on,
  disabledReason,
  onToggle,
  onBlocked,
}: {
  on: boolean
  /** 非空 = 现在改不动，这一句就是为什么 */
  disabledReason?: string | null
  onToggle: (next: boolean) => void
  /** 改不动的时候点了它 */
  onBlocked?: () => void
}) {
  const blocked = Boolean(disabledReason)
  return (
    <button
      type="button"
      className="wb-sw"
      role="switch"
      aria-checked={on}
      aria-disabled={blocked || undefined}
      title={disabledReason ?? undefined}
      onClick={() => (blocked ? onBlocked?.() : onToggle(!on))}
    >
      <span className="wb-sw__knob" />
    </button>
  )
}

interface Props {
  param: ParamView
  /** 原始值，不是显示文本 */
  value: unknown
  form: 'cell' | 'row'
  autoFocus?: boolean
  /** 提交一个新值。**由调用方决定写到哪一层** */
  onCommit: (next: unknown) => void
  /** 放弃这次编辑（Esc） */
  onCancel: () => void
}

export function FieldControl({ param, value, form, autoFocus, onCommit, onCancel }: Props) {
  const [text, setText] = useState(() => asText(value))

  // 外部值变了（比如撤销）要跟着走，否则控件里还留着上一次的输入
  useEffect(() => setText(asText(value)), [value])

  if (param.uiComponent === 'switch') {
    return <Switch on={value === true} onToggle={onCommit} />
  }

  if (param.choices.length > 0) {
    // segmented 与 select 都用 <select>：segmented 的分段按钮在 16 列宽度下摆不开，
    // 而「摆不开就横向溢出」比「换成下拉」更糟
    return (
      <select
        className="wb-ctl"
        data-form={form}
        data-kind="select"
        value={String(value ?? '')}
        autoFocus={autoFocus}
        onChange={(e) => {
          const hit = param.choices.find((c) => String(c.value) === e.target.value)
          onCommit(hit ? hit.value : e.target.value)
        }}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
      >
        {param.choices.map((c) => (
          <option key={String(c.value)} value={String(c.value)}>
            {c.label}
            {c.deprecated && '（已废弃）'}
          </option>
        ))}
      </select>
    )
  }

  const numeric = param.valueType === 'float' || param.valueType === 'int'
  return (
    <input
      className="wb-ctl"
      data-form={form}
      data-kind={numeric ? 'number' : 'text'}
      type={numeric ? 'number' : 'text'}
      value={text}
      min={param.min ?? undefined}
      max={param.max ?? undefined}
      step={param.step ?? undefined}
      autoFocus={autoFocus}
      onChange={(e) => setText(e.target.value)}
      /* 失焦 / 回车才提交。每敲一个字符提交一次会让撤销栈塞满半截值 */
      onBlur={() => commit(text)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit(text)
        if (e.key === 'Escape') onCancel()
      }}
    />
  )

  function commit(raw: string) {
    if (!numeric) {
      onCommit(raw)
      return
    }
    const n = Number(raw)
    // 空串或解析不出数字时**不提交**，而不是提交 0 —— 提交 0 是在替用户编一个值
    if (raw.trim() === '' || Number.isNaN(n)) {
      onCancel()
      return
    }
    onCommit(n)
  }
}

function asText(v: unknown): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'string') return v
  return String(v)
}
