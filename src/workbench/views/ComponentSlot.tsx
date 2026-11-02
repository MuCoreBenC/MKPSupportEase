/*
 * 组件卡里的「画布占位」（工作台 2026-11-01 第三稿）。
 *
 * 作者两句话定了这块该长什么样：
 *   · 「客户端那边有个画布，这个工作台其实也应该要有」
 *   · 「他不用像客户端的一样真实啊，它就占个位置就可以了。它的宽度不用占据很多，
 *     主要是它的位置，它就应该在左侧」
 *
 * 所以这里**不画热床、不画比例**（工作台手里没有板件轮廓 `Plate`，也没有热床尺寸，
 * 硬画一张就是假数据）。就一块窄窄的虚线方块 + 一句话，说明"客户端那块画布在这一侧"。
 * 哪一侧由 `COMPONENT_SIDE` 说了算 —— 以后别的卡画布在右边，加一行就行。
 */
import s from './ComponentSlot.module.css'

export default function ComponentSlot({ component }: { component: string }) {
  return (
    <div className={s.slot} title="客户端在这一侧画的是可交互画布；工作台不还原它，只占住位置">
      <span className={s.badge}>画布</span>
      <span className={s.name}>{component}</span>
    </div>
  )
}
