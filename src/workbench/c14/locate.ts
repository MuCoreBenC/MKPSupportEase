/*
 * 「去处理」的定位 —— 滚到目标锚点 + 闪一下。
 *
 * 判据与行为与 C14/C15 那一版一致：滚动用**即时**（`block: 'center'`，不用 smooth ——
 * 低帧率 / 无头环境下 smooth 可能一帧都不跑，等于没滚）；闪烁是给元素挂一个
 * 动画类（`.flashIt` 卡 / `.rowFlash` 行），1.8s 后摘掉。
 *
 * 瞬态 DOM 效果**不进 React 状态**：进状态只会逼整页重渲染；classList 直改，
 * 打断正在播的同款动画（读一次 offsetWidth）后从头再闪。
 */
import s from '../c14.module.css'

/** 闪烁类摘掉的延时（与 keyframes 的时长一致） */
const FLASH_MS = 1800

/**
 * 按锚点 id 定位：滚到视口中央 + 闪一下。
 *
 * `row` 用行上那条动画（只铺底色），默认用卡上那条（底色 + 描边）。
 * 找不到元素就什么都不做 —— 目标项可能在某台机型/某一版上不存在，那不是错误。
 */
export function locateAnchor(id: string, opts?: { row?: boolean }): void {
  const el = document.getElementById(id)
  if (!el) return
  el.scrollIntoView({ block: 'center' })
  const cls = opts?.row ? s.rowFlash : s.flashIt
  el.classList.remove(s.flashIt, s.rowFlash)
  void (el as HTMLElement).offsetWidth /* 打断正在播的同款动画，从头再闪 */
  el.classList.add(cls)
  window.setTimeout(() => el.classList.remove(s.flashIt, s.rowFlash), FLASH_MS)
}