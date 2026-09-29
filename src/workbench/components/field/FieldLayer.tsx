/*
 * 页面内浮层层（#33）。
 *
 * # 为什么需要它：`position: fixed` 在这个项目里是坏的
 *
 * 预览器的外壳 `.window` 上永远挂着一个 `transform`（`src/App.module.css@LINE[208]`，
 * 等比适配用的，即使 scale=1 也是一个 used transform）。这件事有两个后果：
 *
 *   1. `.window` 成了 `position: fixed` 的**包含块** —— fixed 弹层用 `getBoundingClientRect()`
 *      量到的是视口坐标，写进 `left/top` 却被当成 `.window` 的局部坐标，于是整体偏移
 *      约等于 `.window` 左上角在视口里的位置（默认预设下大约 116 / 80 px）。
 *   2. 偏出去的部分被 `.window { overflow: hidden }` 裁掉。
 *
 * #32 的枚举下拉与右键菜单是全仓库唯一两处 `position: fixed`，也正是唯一两处「点了没反应」。
 * 事件其实进来了（按钮拿到了焦点），只是浮层飞到看不见的地方去了。
 *
 * # 为什么不 portal 到 document.body
 *
 * 那样 fixed 立刻就对了（portal 节点不是 `.window` 的后代），但会撒谎：`fit` 档下外壳是缩放的，
 * body 上的浮层不跟着缩，字号与表格对不上；而且浮层会飘到外壳外面、盖住预览器工具栏。
 * 我们演示的是「一个产品界面」，它的浮层不该跑到产品窗口外面去。
 *
 * 所以：**浮层挂在页面自己里，坐标相对这一层算**。外壳怎么缩放、怎么平移都不影响。
 *
 * # 一次只开一个
 *
 * 层里同时只挂一个浮层：新的打开时旧的先关。密表里每行一个下拉 + 一颗 `⋯`，
 * 允许同时开两个只会让「点外面关」的判定变成一团乱麻。
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { FieldLayerCtx } from './fieldLayerContext'
import type { FieldLayerApi } from './fieldLayerContext'
import s from './FieldLayer.module.css'

export default function FieldLayer({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null)
  const [el, setEl] = useState<HTMLElement | null>(null)
  const openRef = useRef<(() => void) | null>(null)

  useEffect(() => {
    setEl(ref.current)
  }, [])

  const open = useCallback((close: () => void) => {
    const prev = openRef.current
    if (prev !== null && prev !== close) prev()
    openRef.current = close
    return () => {
      if (openRef.current === close) openRef.current = null
    }
  }, [])

  /* api 的身份要稳：浮层的 effect 依赖它，每帧换一个新对象会让浮层反复重挂 */
  const [api, setApi] = useState<FieldLayerApi>(() => ({ el: null, open }))
  useEffect(() => {
    setApi({ el, open })
  }, [el, open])

  return (
    <FieldLayerCtx.Provider value={api}>
      {children}
      <div ref={ref} className={s.layer} />
    </FieldLayerCtx.Provider>
  )
}
