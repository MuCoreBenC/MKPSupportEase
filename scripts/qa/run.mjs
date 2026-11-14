/*
 * 质检跑道（2026-11-14）。
 *
 * 一条命令：起（或复用）工作台的 dev 服务 → 在真浏览器里点 → 出带截图的报告。
 *
 * ```bash
 * npm run qa                    # 全部 spec
 * npm run qa -- --only=order    # 只跑文件名含 order 的那几条
 * npm run qa -- --legacy        # 顺带把 scripts/probes 里那批老探针跑一遍（要先起客户端预览 4173）
 * npm run qa -- --url=http://localhost:5322   # 复用已在跑的服务
 * ```
 *
 * # 为什么打在 dev server 上，不是 preview
 *
 * 工作台**只认 dev**：`src/workbench/main.tsx` 里那句 `import.meta.env.DEV && ?mock=1`
 * 是假后端的开关，生产构建里它被摇掉（那是刻意的 —— 夹具不许冒充真后端）。
 * 所以质检跑的是 `vite --mode workbench`（5322），带 `?mock=1`。
 *
 * # 产物
 *
 * `tmp-shots/qa-<时间戳>/`：`report.html`（给人看）、`report.md`（给 AI / 留档）、
 * `shots/*.png`。整棵目录在 `.gitignore` 与产物边界闸的禁区里，不会进库。
 */

import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { chromium } from 'playwright-core'

import { makeReporter } from './lib.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = join(here, '..', '..')

const argOf = (name) => {
  const hit = process.argv.find((a) => a.startsWith(`--${name}=`))
  return hit === undefined ? null : hit.slice(name.length + 3)
}
const flag = (name) => process.argv.includes(`--${name}`)

const only = argOf('only')
const explicitUrl = argOf('url')
const withLegacy = flag('legacy')
const base = explicitUrl ?? 'http://localhost:5322'
const wbUrl = `${base.replace(/\/$/, '')}/workbench.html?mock=1`

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function up(url) {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(1500) })
    return res.ok
  } catch {
    return false
  }
}

/** 起 dev 服务（已经有人在跑就直接用 —— 5322 是 strictPort） */
async function ensureServer() {
  if (await up(`${base.replace(/\/$/, '')}/workbench.html`)) {
    console.log(`· 复用已在跑的服务 ${base}`)
    return null
  }
  console.log('· 起工作台 dev 服务（vite --mode workbench）…')
  const child = spawn('npm', ['run', 'dev:workbench'], {
    cwd: root,
    stdio: 'ignore',
    detached: true,
  })
  child.unref()
  for (let i = 0; i < 90; i += 1) {
    await sleep(1000)
    if (await up(`${base.replace(/\/$/, '')}/workbench.html`)) return child
  }
  throw new Error(`等不到 ${base} —— 手动起一次 npm run dev:workbench 看看它报什么`)
}

async function launch() {
  for (const channel of ['msedge', 'chrome']) {
    try {
      return await chromium.launch({ channel })
    } catch {
      /* 换下一个；都没有就用 playwright 自带的 chromium */
    }
  }
  return chromium.launch()
}

const stamp = new Date()
  .toISOString()
  .replace(/[:T]/g, '-')
  .slice(0, 19)
const outDir = join(root, 'tmp-shots', `qa-${stamp}`)
mkdirSync(outDir, { recursive: true })

const reporter = makeReporter({ outDir, title: '工作台质检报告' })

/** 一条 spec = 一个文件、默认导出 async (t, page) */
const specDir = join(here, 'specs')
const specFiles = existsSync(specDir)
  ? readdirSync(specDir)
      /* 下划线开头的是共用步骤，不是一条 spec */
      .filter((f) => f.endsWith('.mjs') && !f.startsWith('_'))
      .sort()
  : []
const picked = specFiles.filter((f) => only === null || f.includes(only))
if (picked.length === 0) {
  console.error(`✗ 没有可跑的 spec${only === null ? '' : `（--only=${only}）`}`)
  process.exit(2)
}

const child = await ensureServer()
const browser = await launch()
let failedAll = 0

try {
  for (const file of picked) {
    const name = file.replace(/\.mjs$/, '')
    const mod = await import(join(specDir, file))
    const run = mod.default
    reporter.section(mod.title ?? name)
    const page = await browser.newPage({ viewport: { width: 1600, height: 950 } })
    const errs = []
    page.on('pageerror', (e) => errs.push(`pageerror: ${e.message}`))
    page.on('console', (m) => {
      /* 404 是随包资产的探针期常态，不当红；其余 error 都算 */
      if (m.type() === 'error' && !m.text().includes('404')) errs.push(`console: ${m.text().slice(0, 300)}`)
    })
    try {
      await run({ check: (t, ok, d) => reporter.check(t, ok, d), shot: (p, l) => reporter.shot(p, l), note: (t) => reporter.note(t) }, page, wbUrl)
    } catch (e) {
      reporter.check(`${name} 跑到底`, false, String(e).split('\n')[0])
    } finally {
      reporter.errors(errs)
      await page.close()
    }
    const s = reporter.summary()
    failedAll = s.failed
  }

  /* ---------- 老探针（可选） ---------- */
  const legacy = []
  if (withLegacy) {
    const probeDir = join(root, 'scripts', 'probes')
    const probes = readdirSync(probeDir).filter((f) => f.endsWith('.mjs')).sort()
    const probeUrl = argOf('probe-url') ?? 'http://localhost:4173/'
    if (!(await up(probeUrl))) {
      legacy.push({ name: '老探针整批', ok: false, detail: `${probeUrl} 没起（先 npm run build && npx vite preview --port 4173）` })
    } else {
      for (const p of probes) {
        const res = await new Promise((resolve) => {
          const c = spawn('node', [join(probeDir, p), probeUrl], { cwd: root })
          let out = ''
          c.stdout.on('data', (d) => (out += d))
          c.stderr.on('data', (d) => (out += d))
          c.on('close', () => resolve(out))
        })
        const fails = res.split('\n').filter((l) => l.startsWith('FAIL'))
        legacy.push({
          name: p,
          ok: fails.length === 0 && res.trim() !== '',
          detail: fails.length === 0 ? '' : fails.join(' / ').slice(0, 300),
        })
      }
    }
  }

  const done = reporter.finish({ legacy })
  console.log('')
  console.log(`✓ 报告：${done.htmlPath}`)
  console.log(`  断言 ${done.total} 条 · 失败 ${done.failed} 条`)
  process.exitCode = done.failed === 0 && legacy.every((l) => l.ok) ? 0 : 1
} finally {
  await browser.close()
  if (child !== null && child.pid !== undefined) {
    try {
      process.kill(-child.pid)
    } catch {
      /* 已经退了 */
    }
  }
}
