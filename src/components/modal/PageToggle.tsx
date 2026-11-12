/*
 * **模态框左下角那颗翻页小钮**（「信息」/「说明」/「警告」）—— 唯一出口。
 *
 * 形状：细边胶囊 + 一枚圆 i，**比页脚正按钮小一档**（正按钮 32px 高）——
 * 它是个"翻过去看看"的小动作，不该与主动作抢同一档的分量。
 *
 * # 两条口径
 *
 * 1. **钮上写的是"点它会去哪一页"**，不是"现在在哪一页"（作者 2026-10-18 定的；
 *    早先还配过 `data-tip` 说全"切到哪儿"）。
 * 2. **靠左是外面的页脚管的**：页脚是右对齐的一行，这颗钮得由它外面那一块
 *    （`margin-right: auto`）顶到最左 —— 见两处调用点的 `footLead / footLeft`。
 *    这一颗自己**不带**那个 auto，否则同一个页脚里再放别的小字就会散。
 *
 * 那颗 i 是**画出来的 SVG**：字体里的 `ⓘ` 自带基线偏移，`align-items: center` 居中的
 * 是行盒不是字形，看上去会偏低（与全站"圈里记号一律 SVG"同一条）。
 */

import s from './PageToggle.module.css'

function InfoGlyph() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" fill="none" aria-hidden="true">
      <circle cx="6" cy="6" r="4.6" stroke="currentColor" strokeWidth="1.1" />
      <path d="M6 5.4v3" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" />
      <circle cx="6" cy="3.7" r="0.62" fill="currentColor" />
    </svg>
  )
}

export interface PageToggleProps {
  /** 钮面上的字 —— **去哪一页**（「信息」「说明」「警告」） */
  label: string
  /**
   * 读屏用的全句（「切换：显示这一趟的警告」）。
   *
   * 钮面只写得下一个词，光念那个词听不出"点了会发生什么" —— 这一格说全。
   * 不给就用 `label`（那一页的名字本身就是全部意思时）。
   */
  ariaLabel?: string
  /** 悬停说明（可选，与 `ariaLabel` 同一句也行） */
  tip?: string
  /** 两态开关那一类用法才给（导出框的「文件 ↔ 信息」）；翻页用不给 */
  pressed?: boolean
  onClick: () => void
}

export function PageToggle({ label, ariaLabel, tip, pressed, onClick }: PageToggleProps) {
  return (
    <button
      type="button"
      className={s.toggle}
      aria-label={ariaLabel ?? label}
      aria-pressed={pressed}
      data-tip={tip}
      onClick={onClick}
    >
      <InfoGlyph />
      {label}
    </button>
  )
}
