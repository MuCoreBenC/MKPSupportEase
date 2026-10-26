/*
 * 圆钮开关共用件（2026-10-17，作者：「你要做一下符合我的风格的开关，我不希望用它那种，
 * 而且你得做好复用的开关」）。
 *
 * # 为什么另起一个，而不是用参数表那个
 *
 * 参数表那件是 `components/field/SegmentedField` 的 `variant="switch"` —— 它是
 * **「关闭 | 开启」两段胶囊**，住在密集的参数行里，尺寸与话术都是为"表"设计的。
 * 设置页要的是"一行说明 + 右边一颗开关"，两段胶囊在那儿会被读成"一个小选择器"，
 * 而且它自带文字段，与右边对齐的控件列放不到一起。
 *
 * 两件**并存**，各管各的：参数表照旧用分段式（那里的值本来就是"两档枚举"），
 * 设置页一律用这一颗。**不互相替代**，也不为对方改样子。
 *
 * # 外观
 *
 * 36×20 的槽 + 16 的圆钮。关：`--surface-sunken` 底配 `--line` 发丝边；
 * 开：`--accent` 实心（深浅两套都是那支印刷机绿）+ 白钮贴右。
 * 颜色**全走 token**，没有一处写死色值 —— 深色那套是 `#2f9e5b`，压暗一档但仍是实心绿。
 *
 * ★★ **不加 `transition` / `animation`**：本项目跑在 WKWebView 里，那边不播 CSS 动画
 * （`docs` 里那条铁律），写了也只是"点了之后等一帧直接跳"。切换就是换色，
 * 一步到位，看起来反而是准的。
 *
 * # 语义
 *
 * 真 `<button type="button" role="switch" aria-checked>`：
 *   · 读屏会说"开关，已开启/已关闭"；
 *   · 空格与回车走原生 button 行为（不需要自己接键盘）；
 *   · **受控**：状态归页面（`checked` + `onChange(next)`），这里一个 state 都不存 ——
 *     与 `Check` 同一套约定，于是"界面上显示的"永远是"调用方手里那份"。
 *
 * 没有可见文字时给 `ariaLabel`（设置页那一行有标题，用 `ariaLabel` 把标题带过来）。
 */

import s from './switch.module.css'

export interface SwitchProps {
  /** 当前状态（受控） */
  checked: boolean
  /** 收到的是**下一个状态**（调用方负责写进自己的状态） */
  onChange: (next: boolean) => void
  disabled?: boolean
  /** 开关旁边没有可见文字时给读屏一个名字（设置页传那一行的标题） */
  ariaLabel: string
  /** 调用方只微调外距（结构不在这里改） */
  className?: string
}

export default function Switch({
  checked,
  onChange,
  disabled = false,
  ariaLabel,
  className,
}: SwitchProps) {
  const cls = [
    s.track,
    checked ? s.on : '',
    disabled ? s.disabled : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      className={cls}
      onClick={() => onChange(!checked)}
    >
      <span className={s.knob} aria-hidden="true" />
    </button>
  )
}
