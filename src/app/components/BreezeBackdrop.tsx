import { useEffect, useMemo, useState, type CSSProperties } from 'react'

import lightBgUrl from '../assets/hero/breeze-tuned-base.svg'
import darkCloudUrl from '../assets/hero/galaxy_cloud_dark.svg'
import { useIsDark } from '../state/appState'
import s from './BreezeBackdrop.module.css'

/*
 * 浅色窗底：调板确认的「我的方案」。
 *
 * 底色 + 原版流动线条 + 色阶抖动离线做成 `breeze-tuned-base.svg`；
 * 光点和叶片每次出现时随机生成一批位置，保留轻摆 / 明暗变化。
 * 深色档整层不渲染。
 */
const W = 1080
const H = 608
const LEAF_COUNT = 14
const DOT_COUNT = 7
const LEAF_STRENGTH = 0.47
const DOT_STRENGTH = 0.36
const DOT_SIZE = 0.58
const DOT_GLOW = 0.8
const LEAF_SIZE = 1.18
const LEAF_SPEED = 0.65
const LEAF_SWAY = 1.74
const LEAF_BOB = 1.32
const LEAF_TINT = 0.77

type Dot = { ox: number; oy: number; r: number; op: number }
type Leaf = {
  ox: number
  oy: number
  L: number
  Wd: number
  rot: number
  op: number
  scale: number
  duration: number
  delay: number
  sway: number
  bob: number
  direction: number
}

function mulberry32(a: number): () => number {
  return () => {
    a |= 0
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
const R = (rnd: () => number, a: number, b: number) => a + (b - a) * rnd()
const F = (n: number) => n.toFixed(1)
const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v))

function mixHex(from: string, to: string, t: number): string {
  const parse = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16))
  const a = parse(from)
  const b = parse(to)
  return `#${a.map((v, i) => Math.round(v + (b[i] - v) * t).toString(16).padStart(2, '0')).join('')}`
}

const LEAF_SOFT = mixHex('#b9e4d4', '#ffffff', LEAF_TINT)
const randomSeed = () => Math.floor(Math.random() * 0xffffffff)

function makeDecor(seed: { leaf: number; dot: number }): { dots: Dot[]; leaves: Leaf[] } {
  const dotRnd = mulberry32(seed.dot * 97 + 31)
  const leafRnd = mulberry32(seed.leaf * 97 + 19)
  const dots: Dot[] = []
  const leaves: Leaf[] = []
  const cx = W * 0.55
  const cy = H * 0.38
  const rx = W * 0.60 / 2
  const ry = H * 0.60 / 2

  for (let k = 0; k < DOT_COUNT; k++) {
    dots.push({
      ox: clamp(cx + (dotRnd() * 2 - 1) * rx, 18, W - 18),
      oy: clamp(cy + (dotRnd() * 2 - 1) * ry, 18, H - 18),
      r: R(dotRnd, 1.2, 3.6) * DOT_SIZE,
      op: R(dotRnd, 0.72, 0.98),
    })
  }

  for (let k = 0; k < LEAF_COUNT; k++) {
    const duration = R(leafRnd, 15, 30) / Math.max(0.4, LEAF_SPEED)
    leaves.push({
      ox: R(leafRnd, W * 0.06, W * 0.94),
      oy: R(leafRnd, H * 0.08, H * 0.92),
      L: R(leafRnd, 46, 132),
      Wd: R(leafRnd, 14, 49),
      rot: R(leafRnd, -72, 72),
      op: R(leafRnd, 0.60, 0.92),
      scale: R(leafRnd, 0.38, 0.92) * LEAF_SIZE,
      duration,
      delay: leafRnd() * duration,
      sway: R(leafRnd, 10, 30) * LEAF_SWAY,
      bob: R(leafRnd, 4, 14) * LEAF_BOB,
      direction: leafRnd() < 0.5 ? -1 : 1,
    })
  }
  return { dots, leaves }
}

function leafPath(L: number, Wd: number): string {
  return `M 0 0 C ${F(-Wd * 0.56)} ${F(-L * 0.28)}, ${F(-Wd * 0.36)} ${F(-L * 0.78)}, ${F(Wd * 0.18)} ${F(-L)} C ${F(Wd * 0.56)} ${F(-L * 0.78)}, ${F(Wd * 0.56)} ${F(-L * 0.28)}, 0 0 Z`
}

function BreezeDecor() {
  const [seeds] = useState(() => ({ leaf: randomSeed(), dot: randomSeed() }))
  const { dots, leaves } = useMemo(() => makeDecor(seeds), [seeds])
  return (
    <svg className={s.decor} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden="true">
      <defs>
        <radialGradient id="bk-dot-glow" cx=".5" cy=".5" r=".5" colorInterpolation="sRGB">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.9" />
          <stop offset=".35" stopColor="#ffffff" stopOpacity="0.28" />
          <stop offset="1" stopColor="#ffffff" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="bk-leaf" x1="0" y1="1" x2=".18" y2="0" colorInterpolation="sRGB">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.16" />
          <stop offset=".32" stopColor={LEAF_SOFT} stopOpacity="0.86" />
          <stop offset=".62" stopColor="#ffffff" stopOpacity="0.94" />
          <stop offset=".84" stopColor={LEAF_SOFT} stopOpacity="0.56" />
          <stop offset="1" stopColor="#ffffff" stopOpacity="0.18" />
        </linearGradient>
      </defs>
      <g className={s.dots} opacity={DOT_STRENGTH}>
        {dots.map((d, k) => (
          <g key={k} transform={`translate(${F(d.ox)} ${F(d.oy)})`}>
            <circle r={F(d.r * (1.1 + DOT_GLOW * 3.6))} fill="url(#bk-dot-glow)" opacity=".78" />
            <circle r={F(d.r)} fill="#ffffff" opacity={d.op} />
          </g>
        ))}
      </g>
      <g opacity={LEAF_STRENGTH}>
        {leaves.map((d, k) => {
          const style = {
            '--leaf-duration': `${F(d.duration)}s`,
            '--leaf-delay': `-${F(d.delay)}s`,
            '--leaf-sway-x': `${F(d.sway * d.direction)}px`,
            '--leaf-sway-y': `${F(-d.bob)}px`,
          } as CSSProperties
          const path = leafPath(d.L, d.Wd)
          return (
            <g key={k} transform={`translate(${F(d.ox)} ${F(d.oy)})`} opacity={d.op}>
              <g className={s.leaf} style={style}>
                <g transform={`rotate(${F(d.rot)}) scale(${F(d.scale)})`}>
                  <path d={path} fill="#8fc9b3" opacity=".12" transform="translate(1.6 2)" />
                  <path d={path} fill="url(#bk-leaf)" opacity=".86" stroke="#84bca6" strokeWidth="1" strokeOpacity=".4" />
                  <path d={path} fill="#ffffff" className={s.leafShimmer} opacity=".24" />
                </g>
              </g>
            </g>
          )
        })}
      </g>
    </svg>
  )
}

export default function BreezeBackdrop() {
  const isDark = useIsDark()

  /*
   * 深浅背景都在启动时预热：浅色 SVG 常驻在 DOM 里（深色时只把整层藏掉），
   * 深色云雾也提前 decode。首次切主题不再临时发请求、不再等图片解码。
   */
  useEffect(() => {
    [lightBgUrl, darkCloudUrl].forEach((url) => {
      const img = new Image()
      img.src = url
      void img.decode?.().catch(() => undefined)
    })
  }, [])

  if (isDark) return null

  return (
    <div className={s.root} aria-hidden="true">
      <div className={s.waves} />
      <BreezeDecor />
    </div>
  )
}
