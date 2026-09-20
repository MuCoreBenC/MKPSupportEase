import type { CSSProperties } from 'react'

import { useWindowSize } from '../../hooks/useWindowSize'
import { evalFill, evalNudge } from '../heroCurves'
import type { ArtLayer } from '../useArtLayers'

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
 * 尺寸与位置完全由窗口尺寸 + 固化曲线决定（见 `src/app/heroCurves.ts`）。
 * 换图时同时挂两层：新层淡入、旧层原地淡出，两层共用同一个定尺盒子，所以只有透明度在变。
 *
 * 与试验场那份的差别，都是刻意的：拖动微移、滚轮改尺寸、Ctrl+Z 撤销、吸附网格、坐标读数、
 * 槽位与大图的尺寸上报 —— 全部拿掉，它们是调参面板的输入输出，产品里没有面板。
 * 试验场的默认状态就是"锁定"（`editLock` 默认 true、网格 opacity 0），
 * 所以拿掉之后屏幕上的结果与那边逐像素一致。
 */
export default function HeroFade({ layers, onSettle, onDrop, alt }: HeroFadeProps) {
  const win = useWindowSize()

  // 什么都没选：槽位彻底空着
  if (layers.length === 0) return null

  const fill = evalFill(win.w, win.h)
  const nudge = evalNudge(win.w, win.h)
  const top = layers[layers.length - 1]

  return (
    <div className={s.box}>
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
