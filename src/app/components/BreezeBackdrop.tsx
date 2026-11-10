import { useMemo } from 'react'

import { useIsDark } from '../state/appState'
import s from './BreezeBackdrop.module.css'

/*
 * 浅色窗底：一张离线算好的薄纱波浪图（CSS 背景，整层极慢循环平移）+ 少量光点与小柳叶。
 * 波浪不带任何 SVG 滤镜、也不逐帧重算路径；深色档整层不渲染。
 */
const W = 1200
const H = 700
const SEED = 2718
const PICK_INDEX = 5
const LEAF_DUR = [23, 29, 26]
const LEAF_DELAY = [0, 8, 15]

type Dot = { ox: number; oy: number; r: number; op: number }
type Leaf = { ox: number; oy: number; L: number; Wd: number; rot: number; op: number }

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

function makeDecor(): { dots: Dot[]; leaves: Leaf[] } {
  const rnd = mulberry32(SEED * 577 + PICK_INDEX * 7919 + 7)
  const dots: Dot[] = []
  const leaves: Leaf[] = []
  for (let k = 0; k < 7; k++) {
    dots.push({ ox: R(rnd, 70, 1130), oy: R(rnd, 70, 620), r: R(rnd, 2, 4.5), op: R(rnd, 0.7, 0.95) })
  }
  for (let k = 0; k < 3; k++) {
    leaves.push({
      ox: R(rnd, 120, 1080),
      oy: R(rnd, 110, 600),
      L: R(rnd, 66, 125),
      Wd: R(rnd, 24, 46),
      rot: R(rnd, -75, 75),
      op: R(rnd, 0.72, 1),
    })
  }
  return { dots, leaves }
}

function leafPath(L: number, Wd: number): string {
  return `M 0 0 C ${F(-Wd * 0.56)} ${F(-L * 0.28)}, ${F(-Wd * 0.36)} ${F(-L * 0.78)}, ${F(Wd * 0.18)} ${F(-L)} C ${F(Wd * 0.56)} ${F(-L * 0.78)}, ${F(Wd * 0.56)} ${F(-L * 0.28)}, 0 0 Z`
}

function BreezeDecor() {
  const { dots, leaves } = useMemo(makeDecor, [])
  return (
    <svg className={s.decor} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <defs>
        <linearGradient id="bk-leaf" x1="0" y1="1" x2="0" y2="0">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0" />
          <stop offset=".30" stopColor="#ffffff" stopOpacity="0.92" />
          <stop offset=".75" stopColor="#ffffff" stopOpacity="0.48" />
          <stop offset="1" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
      </defs>
      <g className={s.dots}>
        {dots.map((d, k) => (
          <circle key={k} cx={F(d.ox)} cy={F(d.oy)} r={d.r} fill="#ffffff" opacity={d.op} />
        ))}
      </g>
      {leaves.map((d, k) => (
        <g key={k} transform={`translate(${F(d.ox)} ${F(d.oy)})`} opacity={d.op}>
          <g
            className={s.leaf}
            style={{ animationDuration: `${LEAF_DUR[k]}s`, animationDelay: `-${LEAF_DELAY[k]}s` }}
          >
            <g transform={`rotate(${F(d.rot)})`}>
              <path
                d={leafPath(d.L, d.Wd)}
                fill="url(#bk-leaf)"
                stroke="#ffffff"
                strokeWidth="0.8"
                strokeOpacity="0.2"
              />
            </g>
          </g>
        </g>
      ))}
    </svg>
  )
}

export default function BreezeBackdrop() {
  const isDark = useIsDark()
  if (isDark) return null
  return (
    <div className={s.root} aria-hidden="true">
      <div className={s.waves} />
      <BreezeDecor />
    </div>
  )
}
