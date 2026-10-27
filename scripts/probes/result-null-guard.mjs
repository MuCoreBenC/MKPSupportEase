#!/usr/bin/env node
/*
 * 判据：**成功但结果读数是 `null` 时，结论屏不许崩**（2026-10-17）。
 *
 * # 它防的是哪一种崩
 *
 * `PostProcessFinished.result` 在 Rust 那边是 `Option<...>` ⇒ 线上传过来是**显式的
 * `null`**（不是"省略这个键"）。而 TS 契约一度只写 `result?: PostProcessResult`
 * （= `| undefined`），于是"判空"写成 `res !== undefined` 就是一路假安全 ——
 * `null` 能过那道闸，接着读 `res.penX` 当场抛
 * `TypeError: Cannot read properties of null (reading 'penX')`，
 * 那一屏整块白掉（React 没有边界兜它）。
 *
 * # 它验三件事
 *
 *   1. 结论屏**照常出现**（不白屏、不抛错）；
 *   2. 指标条**不出现**（四格不齐就不摆 —— 这是原本就有的设计）；
 *   3. 各格降级成 `—`，不是 0、不是空白。
 *
 * # 怎么跑
 *
 *   node scripts/probes/result-null-guard.mjs
 *
 * 它会**临时**把 `src/api/mock.ts` 的 `getPostProcessRun` 换成一份 `result: null`
 * 的快照，跑完**还原**（备份 + try/finally）。所以别在改 mock 的同时跑它。
 */

import fs from 'node:fs'
import { chromium } from 'playwright-core'

const MOCK = 'src/api/mock.ts'
const URL = process.env.PROBE_URL ?? 'http://localhost:5321/'

/* ---- ① 临时打补丁 ---- */
const backup = fs.readFileSync(MOCK, 'utf8')
const PATCHED = backup
  .replace(
    /  async getPostProcessRun\(\) \{\n    return null\n  \},/,
    `  async getPostProcessRun() {
    /* PROBE result-null-guard：成功但读数缺失 */
    return {
      presetName: 'A1-fast.toml',
      gcodeName: '19999.0.gcode',
      facts: null,
      question: null,
      progress: null,
      finished: {
        ok: true, cancelled: false, message: '处理完成（探针）', code: null, stage: null,
        exitCode: 0, elapsedMs: 840, output: '/tmp/x.gcode', warnings: [],
        result: null, calibChoice: undefined,
      },
      preview: false, questionEmitted: false,
    }
  },`,
  )
  /* GUI 档才会真弹结论屏 */
  .replace('let mockPostproc: PostprocPrefsWire = {}', "let mockPostproc: PostprocPrefsWire = { uiMode: 'gui' }")

if (PATCHED === backup) {
  console.error('没找到要打补丁的那两处 —— mock 结构改过了？')
  process.exit(2)
}

let browser
try {
  fs.writeFileSync(MOCK, PATCHED)
  /* 让 vite 重新编译（它看文件变化） */
  await new Promise((r) => setTimeout(r, 1200))

  browser = await chromium.launch({ channel: 'chrome' })
  const page = await browser.newPage({ viewport: { width: 1080, height: 760 } })
  const errs = []
  page.on('pageerror', (e) => errs.push(String(e)))
  page.on('console', (m) => {
    if (m.type() === 'error' && /penX|Cannot read|result/.test(m.text())) errs.push(m.text())
  })

  await page.goto(URL, { waitUntil: 'networkidle' })
  await page.waitForTimeout(1500)

  const got = await page.evaluate(() => {
    const modal = [...document.querySelectorAll('[role="dialog"]')].find((d) =>
      /后处理/.test(d.textContent ?? ''),
    )
    if (!modal) return { modal: false }
    const t = modal.innerText
    return {
      modal: true,
      hasMetricStrip: t.includes('笔偏移'),
      dashes: (t.match(/—/g) ?? []).length,
    }
  })

  const ok =
    got.modal === true &&
    got.hasMetricStrip === false &&
    got.dashes >= 4 &&
    errs.filter((e) => /penX/.test(e)).length === 0

  console.log(
    JSON.stringify(
      { ok, ...got, penXErrors: errs.filter((e) => /penX/.test(e)).length, errs },
      null,
      2,
    ),
  )
  process.exitCode = ok ? 0 : 1
} finally {
  /* ---- ② 无论成败都还原 ---- */
  fs.writeFileSync(MOCK, backup)
  if (browser) await browser.close()
}
