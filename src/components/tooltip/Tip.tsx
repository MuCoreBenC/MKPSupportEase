/*
 * 提示气泡的**触发物**（2026-10-30）
 *
 * 气泡本身由全站唯一的 `<TipLayer>` 画 —— 这里只做两件事：
 *   1. 渲染触发物（默认一颗 `?` 圆圈，也可以由调用方给 children）；
 *   2. 把要显示的句子写进 `data-tip`（TipLayer 靠它认领）。
 *
 * 所以气泡只有**一套**样式与避让逻辑（`TipLayer.module.css`），
 * 不会出现"这里用内联 tip、那里用全局 tip"两种观感。
 *
 * 用法：
 *   <Tip text="这句是什么" />                   → 一颗 `?`
 *   <Tip text="这句是什么"><span>任意触发物</span></Tip>
 *
 * ★ 不想加 `?` 圈、只想让**按钮自己**当触发物时，连这个组件都不用：
 *   给按钮写 `data-tip="…"` 就行（`Btn` 已经自动把 `title` 转成它）。
 */

import type { ReactNode } from 'react'

import s from './Tip.module.css'

export interface TipProps {
  /** 气泡里那句话（已经是翻译好的文本） */
  text: string
  /** 触发物；省略则渲染一颗 `?` 圆圈 */
  children?: ReactNode
  /** 读屏用：省略时用 text 本身 */
  ariaLabel?: string
}

export default function Tip({ text, children, ariaLabel }: TipProps) {
  return (
    <span className={s.wrap} data-tip={text}>
      {children ?? (
        <span className={s.dot} role="img" aria-label={ariaLabel ?? text} tabIndex={0}>
          ?
        </span>
      )}
    </span>
  )
}
