/*
 * 标题栏右侧那一格「有事在跑」的小胶囊 —— **软件更新**与**后处理后台跑**共用这一副身子。
 *
 * # 为什么会有这一份（2026-10-17 作者）
 *
 * 后处理那条原来是**压在页签下面的一条横条**（in-flow）：一出现就把整页往下推。
 * 作者：「出现这个会把页面往下我不喜欢」「要不然就复用更新的进度的，用不同的图标？
 * 不用圆环，带动画的图标，处理中的感觉就可以，进度就用百分比」+「更新的也可以不用圆环
 * 用下载的动态的图标」。
 *
 * ⇒ 两件事收进同一种身子：**图标 + 百分比**，住在标题栏、不占内容流。
 *   图标长什么样、动不动、什么语义，归各自的组件（`UpdateIndicator.tsx` /
 *   `postprocess/PostProcessChip.tsx`）；这里只管**壳**：排布、大小、悬停、禁用。
 *
 * ★ 动效一律 WAAPI（`element.animate()`），不用 CSS animation —— WKWebView 上
 *   CSS 动画不播（那条铁律见 `app/postprocess/HANDOVER.md` §4）。
 */

import type { ReactNode } from 'react'

import s from './TitlebarChip.module.css'

interface TitlebarChipProps {
  /** 左边那枚图标（各自的组件画，尺寸交给这里统一） */
  icon: ReactNode
  /** 百分比 / 状态（等宽数字，长短变化不抖） */
  label: ReactNode
  /** 悬停那一句话（说明它在干什么、点一下会怎样） */
  title: string
  ariaLabel: string
  onClick?: () => void
  disabled?: boolean
  /**
   * 图标那一枚的颜色：`accent` = 正在跑、`muted` = 停着、`danger` = 失败。
   * 只染图标，文字恒为次级文字色 —— 两颗并排时不至于抢成一排彩灯。
   */
  tone?: 'accent' | 'muted' | 'danger'
  /** 探针认它 */
  testId?: string
}

export default function TitlebarChip({
  icon,
  label,
  title,
  ariaLabel,
  onClick,
  disabled,
  tone = 'accent',
  testId,
}: TitlebarChipProps) {
  return (
    <button
      type="button"
      className={s.chip}
      data-tone={tone}
      data-testid={testId}
      title={title}
      aria-label={ariaLabel}
      onClick={onClick}
      disabled={disabled}
    >
      <span className={s.icon} aria-hidden="true">
        {icon}
      </span>
      <span className={s.label}>{label}</span>
    </button>
  )
}
