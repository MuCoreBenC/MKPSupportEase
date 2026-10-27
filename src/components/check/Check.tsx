/*
 * 勾选 / 单选共用件（把全站"勾和框"收成一套，2026-10-17）。
 *
 * 作者原话：「我发现我好多地方的按钮样子不一样，你觉得怎么统一一下，大小颜色那些
 * 变体无所谓，主要是**勾和框统一，圆角，勾的位置**之类的」。规格与来龙去脉见
 * `check.module.css` 头注释 —— 这一件就是那套规格的**唯一**出口。
 *
 * # 用法
 *
 * ```tsx
 * <Check checked={on} onChange={setOn}>自动清理执行记录</Check>
 * <Check kind="radio" name="theme" checked={isDark} onChange={() => pick('dark')} />
 * <Check aria-label="全选" indeterminate={part} checked={all} onChange={toggleAll} />
 * ```
 *
 * # 两条实现上的口径
 *
 * 1. **原生 `<input>` 留着**（`type` 按 `kind` 给，视觉上收成 1px 透明）：
 *    `name` 分组、方向键、Tab、读屏全部靠它 —— 换成 div + 手写 aria 是退步。
 *    所以 `kind="radio"` 时**必须给 `name`**，否则一组单选不会互斥（也不会被读屏
 *    当成一组播报）。
 * 2. **半选走原生 `indeterminate`**（复选才有）：它没有对应的 HTML attribute，
 *    只能在节点上现设，所以这里用 `ref` 写；`aria-checked="mixed"` 是画蛇添足
 *    （原生复选框自己就会这么播报）—— 不写。
 */

import type { ReactNode } from 'react'

import s from './check.module.css'

export interface CheckProps {
  /** 控件类型：复选方框 / 单选圈。默认复选。 */
  kind?: 'checkbox' | 'radio'
  checked: boolean
  /** 只对复选有意义：半选（画一道白色横杠）。默认 false。 */
  indeterminate?: boolean
  disabled?: boolean
  /** 单选必须给（一组互斥靠它）；复选给了也让读屏说出"这是一组"。 */
  name?: string
  /** 没有可见文字时（例：表格里那一列行勾选）用它给读屏一个名字。 */
  ariaLabel?: string
  /** 收到"勾/取消"的**下一个状态**（调用方负责把它写进自己的状态）。 */
  onChange: (next: boolean) => void
  /** 调用方微调间距 / 字号 / 文字色（结构不在这里改）。 */
  className?: string
  /** 框右边的文字；不给就是"只有框"（那就要给 `ariaLabel`）。 */
  children?: ReactNode
}

export default function Check({
  kind = 'checkbox',
  checked,
  indeterminate = false,
  disabled = false,
  name,
  ariaLabel,
  onChange,
  className,
  children,
}: CheckProps) {
  const isRadio = kind === 'radio'
  const cls = className === undefined ? s.choice : `${s.choice} ${className}`
  return (
    <label className={cls}>
      <input
        type={isRadio ? 'radio' : 'checkbox'}
        className={s.input}
        name={name}
        checked={checked}
        disabled={disabled}
        aria-label={ariaLabel}
        ref={(el) => {
          /* 半选只有原生 DOM 属性有（没有对应的 attribute），必须现设；
             非复选一律清零，免得复用同一个节点时留上一笔 */
          if (el !== null) el.indeterminate = !isRadio && indeterminate
        }}
        onChange={(e) => onChange(e.target.checked)}
      />
      {/* 标记由它画：`input:checked + .mark` 那几条选择器靠"紧邻兄弟"关系挂上来 */}
      <span className={`${s.mark} ${isRadio ? s.markRadio : s.markCheck}`} aria-hidden="true" />
      {children}
    </label>
  )
}
