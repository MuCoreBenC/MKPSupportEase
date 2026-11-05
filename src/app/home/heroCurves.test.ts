/*
 * 大图曲线「一份数据、两处消费者」的门（2026-11-03）。
 *
 * # 它守的是什么
 *
 * 首屏骨架（`index.html` 的 `#boot .boot-hero`）必须比任何模块都早成形，所以它
 * **不能** import `heroCurves.ts` —— 那边内联了一份逐字镜像（数据 + 求值）。镜像
 * 有价值的前提是"它真的还是同一份"：改曲线只改 TS、忘了改 HTML，骨架上的机器就会
 * 与终态差一截 —— 正好是作者 2026-11-03 报的那个现象
 * （「骨架屏的机器位置和最终的不一样呀，你也按照我设置的曲线显示可以吗」）。
 *
 * 两道判据：
 *   1. 六个数组**逐个数字相等**（不是"差不多"）；
 *   2. 把那段内联脚本**真的跑一遍**（喂一个假 window / document），它在几档窗口上
 *      写出来的 `--boot-fill` / `--boot-nx` / `--boot-ny` 必须与 `evalFill` /
 *      `evalNudge` 一致 —— 这样连"公式抄错"也一并挡住。
 */
import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

import { HERO_CURVES, evalFill, evalNudge } from './heroCurves'

const html = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8')

const BEGIN = '/* @@HERO-CURVES-BEGIN@@ */'
const END = '/* @@HERO-CURVES-END@@ */'

function mirroredCurves(): unknown {
  const from = html.indexOf(BEGIN)
  const to = html.indexOf(END)
  expect(from, 'index.html 里找不到曲线镜像的起始标记').toBeGreaterThan(-1)
  expect(to, 'index.html 里找不到曲线镜像的结束标记').toBeGreaterThan(from)
  const body = html.slice(from + BEGIN.length, to)
  const open = body.indexOf('{')
  const close = body.lastIndexOf('}')
  expect(open).toBeGreaterThan(-1)
  expect(close).toBeGreaterThan(open)
  return JSON.parse(body.slice(open, close + 1))
}

/** 把 `<script id="boot-hero">` 那段拿出来，用一个假的 window / document 跑一遍 */
function runInlineScript(w: number, h: number): Record<string, string> {
  const open = html.indexOf('<script id="boot-hero">')
  expect(open, 'index.html 里找不到 `#boot-hero` 那段脚本').toBeGreaterThan(-1)
  const bodyStart = html.indexOf('>', open) + 1
  const bodyEnd = html.indexOf('</script>', bodyStart)
  const code = html.slice(bodyStart, bodyEnd)

  const vars: Record<string, string> = {}
  const el = {
    style: {
      setProperty(name: string, value: number) {
        vars[name] = String(value)
      },
    },
  }
  const fakeWindow = { innerWidth: w, innerHeight: h, addEventListener: () => {} }
  const fakeDocument = { getElementById: () => el }

  /* 跑的是**仓库里那段源码本身**（不是它的复制品）—— 这正是这道门要的 */
  new Function('window', 'document', code)(fakeWindow, fakeDocument)
  return vars
}

describe('骨架屏那份曲线镜像', () => {
  it('六个数组与 heroCurves.ts 逐数字一致', () => {
    expect(mirroredCurves()).toEqual(HERO_CURVES)
  })

  it('内联脚本算出来的三个量就是 evalFill / evalNudge 的结果', () => {
    const windows = [
      [600, 500],
      [900, 640],
      [1080, 768],
      [1440, 900],
    ]
    for (const [w, h] of windows) {
      const vars = runInlineScript(w, h)
      const fill = evalFill(w, h)
      const nudge = evalNudge(w, h)
      const at = `${w}×${h}`
      expect(Number(vars['--boot-fill']), at).toBeCloseTo(fill, 9)
      expect(Number(vars['--boot-nx']), at).toBeCloseTo(nudge.nudgeX, 9)
      expect(Number(vars['--boot-ny']), at).toBeCloseTo(nudge.nudgeY, 9)
      /* `.boot-model` 的基准 = `.page` 这一格，也就是整扇窗（`3.6cqw` 的容器） */
      expect(vars['--boot-page-w'], at).toBe(`${w}px`)
      /* 页边 / 信息列 / 图槽：四条都必须落地，而且是长度值 */
      for (const key of ['--boot-page-l', '--boot-page-r', '--boot-info-w', '--boot-slot-pad']) {
        expect(vars[key], `${at} ${key}`).toMatch(/^\d+(\.\d+)?px$/)
      }
    }
  })
})
