/*
 * 大图淡入层。
 *
 * # 为什么比试验场那份少一半
 *
 * 试验场那份有一半是调试面板的编辑面：拖图写微移曲线、滚轮写尺寸曲线、Ctrl+Z 撤销 / 重做、
 * 网格吸附、拖动读数、以及往面板上报槽位与大图的实测尺寸。那些全部经试验场自己的 devStore，
 * 调试面板不属于产品，所以整块没有搬过来，只留下产品要的那一半：
 * **按曲线把图摆好 + 换图时两层交叉淡入**。
 *
 * 去掉的东西在试验场的默认状态下本来也不生效：面板的「编辑锁」默认是锁上的（editLock 默认
 * true），拖动 / 滚轮 / 双击复位都被那道锁挡着；网格与拖动读数默认不显形。
 * 所以**默认形态是一样的** —— 同一张图、同一个尺寸、同一个位置、同一段淡入。
 * 差别只有一处：鼠标悬停时不再有那句「已锁定：只看不写（面板里点「解锁编辑」才能调）」的提示，
 * 它说的是产品里没有的面板。`data-locked="true"` 照旧给着，光标就仍然不暗示「这图能拖」。
 *
 * # 尺寸与位置从哪来
 *
 * 那六条曲线写死在 `heroCurves` 里（值来自试验场的基线，见那边的文件头）。
 *
 * CSS Module 是逐字搬过来的，所以里面 `.grid` / `.readout` 两组规则现在没人用了，
 * 没有删 —— 保持与试验场那份一致。
 */

import type { CSSProperties } from 'react'

import type { ArtLayer } from './useArtLayers'
import { evalFill, evalNudge, useWinSize } from './heroCurves'

import s from './HeroFade.module.css'

const FADE_MS = 260

interface HeroFadeProps {
  /** 层状态由页面持有：见 useArtLayers，跨重挂不会重播淡入 */
  layers: ArtLayer[]
  onSettle: (id: number) => void
  onDrop: (id: number) => void
  alt: string
}

/**
 * 尺寸完全由窗口与曲线决定（自动适配），位置也是。
 * 换图时同时挂两层：新层淡入、旧层原地淡出，两层共用同一个定尺盒子，所以只有透明度在变。
 */
export default function HeroFade({ layers, onSettle, onDrop, alt }: HeroFadeProps) {
  const { size: win, ref: boxRef } = useWinSize()

  // 什么都没选：槽位彻底空着，一个节点都不留
  if (layers.length === 0) return null

  const fill = evalFill(win.w, win.h)
  const nudge = evalNudge(win.w, win.h)
  const top = layers[layers.length - 1]

  return (
    <div ref={boxRef} className={s.box}>
      <figure
        className={s.hero}
        style={
          {
            '--hero-fill': fill,
            '--nx': nudge.nudgeX,
            '--ny': nudge.nudgeY,
            '--fade-ms': `${FADE_MS}ms`,
          } as CSSProperties
        }
        /* 恒为锁定态：试验场那份的编辑锁默认就是锁上的，光标也就不该暗示「能拖」 */
        data-locked="true"
      >
        {layers.map((layer) => {
          const leaving = layer.id !== top.id
          return (
            <img
              key={layer.id}
              className={s.img}
              data-kind={layer.kind}
              data-fresh={layer.fresh}
              data-leaving={leaving}
              src={layer.src}
              alt={leaving ? '' : alt}
              aria-hidden={leaving || undefined}
              draggable={false}
              onAnimationEnd={() => (leaving ? onDrop(layer.id) : onSettle(layer.id))}
              onError={() => onDrop(layer.id)}
            />
          )
        })}
      </figure>
    </div>
  )
}
