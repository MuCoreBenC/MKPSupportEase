/*
 * 设置页深色那一层星河：**云雾是原图，星是算出来的**。
 *
 * # 两层怎么分工
 *
 *   云雾（静态）  = `assets/hero/galaxy_cloud_dark.svg` —— 就是原来那张
 *                   `update_band_galaxy_dark.svg` **剥掉星点**之后剩下的三团光晕
 *                   + 一条柔化光带（同一个 viewBox、同一套渐变、同一层底部渐隐）。
 *                   中间那团「云雾一样的银河」在观感上是主角，逐字复刻、不做二次演绎。
 *   星点（动态）  = 这张 canvas 现算：同配色 / 同半径分布 / 同面积密度，
 *                   各自慢慢闪、慢慢漂，亮星带十字光芒，每隔一会儿划一颗流星。
 *
 * # 横向铺满、纵向长长地淡掉
 *
 * 原来那是一张 1200×470 的图挂在 `.page::before` 上：宽度铺满、高度固定 470，
 * 星只撒到 x 43..1172 —— 左右各一条**一颗星都没有的硬边**，470px 处又是第二道
 * 齐刷刷的边（作者 2026-11-10 的几次截图圈的就是这些）。
 *
 * 现在宽度按 `.page` 实际宽铺满（星撒满整宽），高度 `--galaxy-h` 一条，
 * 从 42% 处开始**线性收到 0**：雾和星用的是同一条曲线，所以下半页是"浅下去"，
 * 两层之间也不会有谁先断、谁后断的错位。
 *
 * # 为什么只有星用 canvas
 *
 * 每颗星要自己的闪烁相位与漂移周期，DOM 里就是几百个关键帧实例；一层铺满页面的画布
 * 每帧只画三四百个圆点，30fps 足够「慢慢闪」。云雾一张 CSS 背景就够，不占逐帧。
 */

import { useEffect, useRef } from 'react'

import { useIsDark } from '../state/appState'
import s from './GalaxyBackdrop.module.css'

/* 星带参考尺寸：与云雾图同一个 viewBox，密度好对 */
const REF_W = 1200
const REF_H = 470
/* 原图 247 颗 / (1200×470)；这里再疏一档（作者：像以前一样疏一点） */
const STAR_PER_AREA = (247 / (REF_W * REF_H)) * 0.85
/* 四色几乎等量（71 / 59 / 60 / 57） */
const COLORS = ['#cfe8ff', '#dff0ff', '#eafff3', '#ffe9cc']
/*
 * 往下淡掉的那一段：从 42% 开始线性收到 0，与 `.cloud` 那条 CSS mask 同一条曲线。
 * 改这里要回去改 `GalaxyBackdrop.module.css` 的 `mask-image`，两边一个数。
 */
const FADE_FROM = 0.42
const TAU = Math.PI * 2

interface Star {
  /** 归一化位置：撒满画布（含左右外扩的那几十像素） */
  u: number
  v: number
  /** 半径（参考单位，绘制时按宽度缩放） */
  r: number
  a: number
  color: string
  /** 闪烁：角速度 / 相位 / 振幅（占基础透明度的比例） */
  sp: number
  ph: number
  av: number
  /** 漂移：振幅（px） / 角速度 / 相位 */
  dx: number
  dy: number
  dsp: number
  dph: number
  cross: boolean
}

interface Meteor {
  born: number
  life: number
  x: number
  y: number
  vx: number
  vy: number
  len: number
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

/** 数量按面积换算，位置铺满整宽，四色等权，半径落在原图区间上再收一档 */
function makeStars(w: number, h: number): Star[] {
  /* 种子固定：改窗口只是换算数量，星星不会整片重撒 */
  const rnd = mulberry32(0x5eed1a7)
  const n = Math.max(60, Math.round(STAR_PER_AREA * w * h))
  const out: Star[] = []
  for (let k = 0; k < n; k++) {
    const r = (0.4 + Math.pow(rnd(), 0.9) * 1.26) * 0.78
    out.push({
      u: rnd(),
      v: rnd() * 0.95,
      r,
      /* 原图是 0.19~0.77；作者嫌亮，压到七成 */
      a: R(rnd, 0.13, 0.54),
      color: COLORS[Math.floor(rnd() * COLORS.length)],
      sp: TAU / R(rnd, 4.5, 13.5),
      ph: rnd() * TAU,
      av: R(rnd, 0.16, 0.34),
      dx: R(rnd, 2, 7),
      dy: R(rnd, 2, 7),
      dsp: TAU / R(rnd, 30, 90),
      dph: rnd() * TAU,
      /* 十字光芒只长在亮星上，约 7%（原图 18/247） */
      cross: r > 1.05 && rnd() < 0.09,
    })
  }
  return out
}

/**
 * 往下淡掉：`FADE_FROM` 以下满亮，之后**长长地**线性收到 0。
 * 与 `.cloud` 那条 CSS mask 是同一个比例 —— 星河在下半页是"浅下去"，
 * 不是"到某一行齐刷刷没了"。
 */
function fadeAt(v: number): number {
  if (v < FADE_FROM) return 1
  return Math.max(0, (1 - v) / (1 - FADE_FROM))
}

/** 一帧：星星闪 + 慢慢漂 + 亮星十字 + 偶发流星 */
function paintStars(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  stars: Star[],
  t: number,
  meteors: Meteor[],
): void {
  /*
   * ★ 必须整张清。渐隐区里星星只是**变淡**、不是不画 —— 只清上半截的话，
   *   下面那半会一帧帧叠上去（越积越亮、十字越来越多），正是"中间下面反而亮了"。
   */
  ctx.clearRect(0, 0, w, h)
  const k = w / REF_W
  /* 整片天极慢地横着流一点（周期约 5 分钟），看不出在动、但一直是活的 */
  const sway = 6 * Math.sin(t * 0.02)

  for (const color of COLORS) {
    ctx.fillStyle = color
    for (const st of stars) {
      if (st.color !== color) continue
      const fade = fadeAt(st.v)
      if (fade <= 0) continue
      const tw = 1 + st.av * Math.sin(t * st.sp + st.ph)
      const x = st.u * w + sway + st.dx * Math.sin(t * st.dsp + st.dph)
      const y = st.v * h + st.dy * Math.cos(t * st.dsp + st.dph * 1.7)
      const r = Math.max(0.35, st.r * k)
      const a = Math.min(1, st.a * tw * fade)
      ctx.globalAlpha = a
      ctx.beginPath()
      ctx.arc(x, y, r, 0, TAU)
      ctx.fill()

      if (st.cross) {
        const arm = r * 5.5
        ctx.globalAlpha = a * 0.45
        ctx.strokeStyle = color
        ctx.lineWidth = Math.max(0.4, 0.7 * k)
        ctx.beginPath()
        ctx.moveTo(x - arm, y)
        ctx.lineTo(x + arm, y)
        ctx.moveTo(x, y - arm)
        ctx.lineTo(x, y + arm)
        ctx.stroke()
      }
    }
  }

  for (let i = meteors.length - 1; i >= 0; i--) {
    const m = meteors[i]
    const p = (t - m.born) / m.life
    if (p >= 1) {
      meteors.splice(i, 1)
      continue
    }
    const dt = t - m.born
    const speed = Math.hypot(m.vx, m.vy)
    const ux = m.vx / speed
    const uy = m.vy / speed
    const hx = m.x + m.vx * dt
    const hy = m.y + m.vy * dt
    const fade = Math.sin(Math.PI * p)
    /* 短的那些就是"小流星"：越短越浅，不抢星点的戏 */
    const bright = m.len / 200

    const g = ctx.createLinearGradient(hx, hy, hx - ux * m.len, hy - uy * m.len)
    g.addColorStop(0, `rgba(232, 250, 255, ${0.5 * bright * fade})`)
    g.addColorStop(0.3, `rgba(180, 232, 255, ${0.18 * bright * fade})`)
    g.addColorStop(1, 'rgba(180, 232, 255, 0)')
    ctx.strokeStyle = g
    ctx.lineWidth = 1.2
    ctx.globalAlpha = 1
    ctx.beginPath()
    ctx.moveTo(hx, hy)
    ctx.lineTo(hx - ux * m.len, hy - uy * m.len)
    ctx.stroke()

    ctx.fillStyle = `rgba(255, 255, 255, ${0.62 * bright * fade})`
    ctx.beginPath()
    ctx.arc(hx, hy, 1.1, 0, TAU)
    ctx.fill()
  }
  ctx.globalAlpha = 1
}

function spawnMeteor(now: number, w: number, h: number, rnd: () => number): Meteor {
  const dir = rnd() < 0.5 ? 1 : -1
  const speed = R(rnd, 420, 680)
  const ang = (R(rnd, 26, 48) * Math.PI) / 180
  return {
    born: now,
    life: R(rnd, 0.8, 1.3),
    x: dir > 0 ? R(rnd, 0.02, 0.42) * w : R(rnd, 0.58, 0.98) * w,
    y: R(rnd, -0.08, 0.22) * h,
    vx: Math.cos(ang) * speed * dir,
    vy: Math.sin(ang) * speed,
    /* 一多半是小流星（70~130），偶尔来一颗长的 */
    len: rnd() < 0.7 ? R(rnd, 70, 130) : R(rnd, 140, 220),
  }
}

export default function GalaxyBackdrop() {
  const isDark = useIsDark()
  const starsRef = useRef<HTMLCanvasElement | null>(null)

  useEffect(() => {
    if (!isDark) return
    const canvas = starsRef.current
    if (canvas === null) return
    const ctx = canvas.getContext('2d')
    if (ctx === null) return

    let cssW = 0
    let cssH = 0
    let list: Star[] = []
    let raf = 0
    let last = 0
    const meteors: Meteor[] = []
    const rnd = mulberry32(0x9e3779b9)
    let nextMeteorAt = 0

    const fit = () => {
      /* DPR 封顶 2：这张画布只画星点，再高的倍率看不出来 */
      const dpr = Math.min(window.devicePixelRatio || 1, 2)
      cssW = Math.max(1, canvas.clientWidth)
      cssH = Math.max(1, canvas.clientHeight)
      canvas.width = Math.max(1, Math.round(cssW * dpr))
      canvas.height = Math.max(1, Math.round(cssH * dpr))
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      list = makeStars(cssW, cssH)
    }

    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches

    const frame = (now: number) => {
      raf = window.requestAnimationFrame(frame)
      /* 30fps 够「慢慢闪」，也把逐帧开销压到一半 */
      if (now - last < 33) return
      last = now
      const t = now / 1000

      if (now >= nextMeteorAt) {
        meteors.push(spawnMeteor(t, cssW, cssH, rnd))
        nextMeteorAt = now + R(rnd, 10, 28) * 1000
      }
      paintStars(ctx, cssW, cssH, list, t, meteors)
    }

    fit()
    /* 先画一帧：浅色时画布虽然藏起来，切到深色不用再等第一帧。 */
    paintStars(ctx, cssW, cssH, list, 0, [])
    nextMeteorAt = performance.now() + 3500

    const ro = new ResizeObserver(fit)
    ro.observe(canvas)

    if (reduced || !isDark) {
      return () => ro.disconnect()
    }

    raf = window.requestAnimationFrame(frame)

    return () => {
      window.cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [isDark])

  if (!isDark) return null

  return (
    <div className={s.root} aria-hidden="true">
      {/* 云雾：原图的静态底（剥掉星点），一层 CSS 背景 —— 见文件头 */}
      <div className={s.cloud} />
      <canvas className={s.stars} ref={starsRef} />
    </div>
  )
}
