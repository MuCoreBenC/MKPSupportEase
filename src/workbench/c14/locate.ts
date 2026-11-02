/*
 * 「去处理」的定位 —— 滚到目标锚点 + 闪一下。
 *
 * 滚动走 `glideIntoView`（全仓那一套"演 transform"的做法，见
 * `components/scroll/glide`）—— 2026-11-01 之前这里写的是 `scrollIntoView({ block:'center' })`
 * 即时跳：作者的要求是"全仓的滚动定位都用那一条"，所以定位也带上过程；
 * 目标所在的滚动容器没有"唯一内容层"时它会自己退回瞬时，不会演出半屏空白。
 * 闪烁是给元素挂一个动画类（`.flashIt` 卡 / `.rowFlash` 行），1.8s 后摘掉。
 *
 * 瞬态 DOM 效果**不进 React 状态**：进状态只会逼整页重渲染；classList 直改，
 * 打断正在播的同款动画（读一次 offsetWidth）后从头再闪。
 *
 * # 2026-11-01 第二轮：滚完**再对一次位置**
 *
 * 作者报「点比较下面的，滚动不到位；好像滚到了、又突然闪到别的地方」。两个来源：
 *
 *   ① **滚到不等于停下**：上面几条 effect 里还有"滚到顶部"（切 tab / 换目标）、
 *      重取派生（`desk` 换了一整个对象）在跑，谁后到谁说话 —— 中途它会把位置拨回去。
 *   ② **高度不是一开始就准**：参数行带着 `content-visibility: auto`，没渲染过的行只按
 *      `contain-intrinsic-size` 估个高。停在半路时目标行其实还没被渲染、量到的是估值 ——
 *      真渲染出来高度一换，它就从中间挪走了一截。
 *
 * 所以收场之后：再量一次、没对准就补到终点（瞬时），然后才闪。补位只做一次，不做动画
 * —— 那种"再演一遍"最容易被人看成"闪了一下"。
 */
import s from '../c14.module.css'
import { glideIntoView, scrollParentOf } from '../../components/scroll/glide'

/** 闪烁类摘掉的延时（与 keyframes 的时长一致） */
const FLASH_MS = 1800
/** 落位补测的容差（px）：小于它就别再动，免得每帧抖一像素 */
const SNAP_TOL = 2
/** 落位补测最多重试几帧（`content-visibility` 的行渲染出来通常 1–2 帧内） */
const SNAP_TRIES = 4

/** 现在把 `el` 居中时，滚动容器该在哪个位置（瞬时算，不写） */
function centerTopOf(el: HTMLElement): { box: HTMLElement; top: number } | null {
  const box = scrollParentOf(el)
  if (box === null) return null
  const at = box.scrollTop + el.getBoundingClientRect().top - box.getBoundingClientRect().top
  const room = Math.max(0, box.scrollHeight - box.clientHeight)
  const raw = at - (box.clientHeight - el.offsetHeight) / 2
  return { box, top: Math.min(Math.max(raw, 0), room) }
}

/**
 * 收场补位：演完之后再量一次"该在哪"，对不上就直接落过去（不做第二段动画）。
 *
 * 为什么要**隔一帧再量**：演的时候内容整块挂着 `translate3d`，收场的同一帧里
 * 布局还没按"没有 transform"重算完；而且刚被滚进视口的那几行（`content-visibility`
 * 的）是在这一帧之后才渲染出来的。
 *
 * @param tries 还剩几次（每次都是一帧）
 */
function settle(el: HTMLElement, tries: number): void {
  if (tries <= 0) return
  if (!el.isConnected) return
  const want = centerTopOf(el)
  if (want === null) return
  const diff = want.top - want.box.scrollTop
  if (Math.abs(diff) <= SNAP_TOL) return
  /* 只补一次真的落过去；剩下的帧留给"落过去之后行渲染出来又把高度改了" */
  want.box.scrollTop = want.top
  requestAnimationFrame(() => settle(el, tries - 1))
}

/**
 * 按锚点 id 定位：滚到视口中央 + 闪一下。
 *
 * `row` 用行上那条动画（只铺底色），默认用卡上那条（底色 + 描边）。
 * 找不到元素就什么都不做 —— 目标项可能在某台机型/某一版上不存在，那不是错误。
 */
export function locateAnchor(id: string, opts?: { row?: boolean }): void {
  const el = document.getElementById(id)
  if (!el) return
  const cls = opts?.row ? s.rowFlash : s.flashIt

  glideIntoView(el, {
    align: 'center',
    onDone: () => {
      settle(el, SNAP_TRIES)
      el.classList.remove(s.flashIt, s.rowFlash)
      void (el as HTMLElement).offsetWidth /* 打断正在播的同款动画，从头再闪 */
      el.classList.add(cls)
      window.setTimeout(() => el.classList.remove(s.flashIt, s.rowFlash), FLASH_MS)
    },
  })
}
