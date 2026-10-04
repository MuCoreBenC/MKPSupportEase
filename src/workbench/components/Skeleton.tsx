/*
 * 骨架屏（2026-10-02，作者：「所有页面都应该优先显示出来……等待的时候，你可以用骨架屏」）。
 *
 * # 为什么要有它
 *
 * 之前 `App.renderPage` 在 `!book || !words` 时**返回 null** —— 整本（`wb_book`）没回来
 * 之前，点了导航**什么都不显示**。作者的原话是「点了等等一段时间它才显示，这是不对的，
 * 它必须立马显示，就是那个反馈」。
 *
 * 骨架屏解决的就是这一句：**页面外壳（页头 / 卡片框 / 行槽）立刻出来**，数据区先摆灰条，
 * 数据一到就换成真内容。用户看见的是「页面在、正在填」，不是「点了没反应」。
 *
 * # 用法
 *
 *   <PageSkeleton cards={3} rows={6} />     页骨架（几张卡、每张几行）
 *   <Line w="60%" />                        一条灰条（卡内局部用）
 *
 * 灰条自己会呼吸（CSS 动画），不靠 JS 计时 —— 页面卡不卡都看得见"在加载"。
 * **不画假数据**：灰条就是灰条，不预先填字（那是另一种撒谎）。
 */

import type { CSSProperties } from 'react'
import s from './Skeleton.module.css'

/** 一条灰条。`w` 是宽度（默认 100%），`h` 是高度（默认 12px） */
export function Line({ w = '100%', h = 12 }: { w?: string; h?: number }) {
  return <span className={s.line} style={{ width: w, height: `${h}px` }} aria-hidden />
}

/** 一条行槽：左边一枚方块（勾选框/头像位）+ 两条长短不一的灰条 */
export function Row() {
  return (
    <div className={s.row} aria-hidden>
      <span className={s.square} />
      <Line w="18%" />
      <Line w="34%" />
      <span className={s.spacer} />
      <Line w="12%" />
    </div>
  )
}

interface PageSkeletonProps {
  /** 画几张卡（默认 2） */
  cards?: number
  /** 每张卡画几行（默认 5；`0` = 只画卡头不画行） */
  rows?: number
  /** 卡片排布：`flow` 竖着一叠（生成页那种）· `cols` 左右两栏（机型页那种） */
  layout?: 'flow' | 'cols'
  /** 页头那一行要不要画（默认画：标题 + 副标题那两条） */
  head?: boolean
  /** 无障碍名字 —— 探针/读屏用来确认"这是加载态" */
  label?: string
}

/**
 * 页骨架。把页面**大致的形状**先摆出来（几张卡、卡里几行），数据到了整体换掉。
 * 形状不必与真页一模一样 —— 它是"这里马上会有东西"的占位，不是预览图。
 */
export default function PageSkeleton({
  cards = 2,
  rows = 5,
  layout = 'flow',
  head = true,
  label = '正在加载',
}: PageSkeletonProps) {
  const style: CSSProperties = {} /* 预留：以后要按 density 调内距的话挂这儿 */
  return (
    <div className={s.page} data-skeleton={layout} aria-busy="true" aria-label={label} style={style}>
      {head && (
        <div className={s.head}>
          <Line w="180px" h={16} />
          <Line w="260px" h={11} />
        </div>
      )}
      <div className={layout === 'cols' ? s.cols : s.flow}>
        {Array.from({ length: cards }).map((_, i) => (
          <section className={s.card} key={i}>
            <div className={s.cardHead}>
              <Line w="120px" h={13} />
              <span className={s.spacer} />
              <Line w="90px" h={11} />
            </div>
            {rows > 0 && (
              <div className={s.cardBody}>
                {Array.from({ length: rows }).map((__, j) => (
                  <Row key={j} />
                ))}
              </div>
            )}
          </section>
        ))}
      </div>
    </div>
  )
}
