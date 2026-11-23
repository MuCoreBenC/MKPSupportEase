#!/usr/bin/env node
/**
 * **看「今天有多少人用过」** —— 读客户端那一次匿名心跳在计数服务上留下的数。
 *
 * ```text
 * npm run usage                             # 最近 14 天，一天一行
 * npm run usage -- --days 30                # 换窗口
 * npm run usage -- --html                   # 生成网页报告（最近 30 天）并打开
 * npm run usage -- --day 2026-11-23         # 单日，外加版本 / 平台分布
 * ```
 *
 * # 分桶规则（与 `src-tauri/src/runtime/usage_ping.rs` 必须一致）
 *
 * ```text
 * Referer: https://mkp-usage-v2.test/2026-11-23/0.0.18-preview.2/macos
 *                                    └── 日期 ──┘└──── 版本 ────┘└ 平台 ┘
 * ```
 *
 *   · `/<日期>/`                     当天总台次（客户端每天另外报这一条）
 *   · `/<日期>/<版本>/<平台>`          当天这个版本这个平台的台次
 *
 * ★ 日期必须在 path 里：不蒜子的桶按**主域名 + path** 记，**子域不参与**
 *   （`d11-23.xxx.test` 与 `d11-24.xxx.test` 是同一个桶，实测过）。
 *
 * 客户端一天最多报一次，所以"台次"就是"当天用过的机器数"。
 *
 * # 一个必须知道的偏差
 *
 * **看一次就 +1。** 这个接口没有只读的口子，本脚本每问一次就给那个桶加一次。
 * 所以脚本把"这台机器读了几个桶几次"记在 `.scratch/usage-reads.json`，显示时减掉。
 *
 * 副作用有两条：换一台机器看，之前那些读数是不知道的（会被算成真实台次）；
 * 减到 0 以下一律显示 0。
 */

import { execSync } from 'node:child_process'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'

const ENDPOINT = 'https://busuanzi.ibruce.info/busuanzi?jsonpCallback=cb'
/** 与客户端同一个域名后缀（只当分桶用的名字，不需要能解析） */
const HOST_SUFFIX = 'mkp-usage-v2.test'
const PLATFORMS = ['macos', 'windows', 'linux']
/** 两次请求之间歇多久：那是个人维护的免费服务，别打它 */
const GAP_MS = 150

const args = process.argv.slice(2)
const getArg = (name, fallback) => {
  const i = args.indexOf(`--${name}`)
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback
}

const html = args.includes('--html')
const days = Number(getArg('days', html ? '30' : '14'))
const onlyDay = getArg('day', null)

/** 本地日期 `YYYY-MM-DD`（与客户端同一口径：本地时区，不是 UTC） */
function isoDay(d) {
  const p = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

function dayList(n) {
  const out = []
  const now = new Date()
  for (let i = n - 1; i >= 0; i -= 1) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - i)
    out.push(isoDay(d))
  }
  return out
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

/**
 * 问一次某个桶，返回 `{ sitePv, pagePv }`（**已减掉本次这一下**）。
 * `path` 为空 = 问这个 origin 的根，拿到的 `sitePv` 就是当天总数。
 */
async function readBucket(day, path = '') {
  const referer = `https://${HOST_SUFFIX}/${day}/${path}`
  const res = await fetch(ENDPOINT, {
    headers: { Referer: referer, 'User-Agent': 'SupportEase-usage-script' },
    signal: AbortSignal.timeout(15000),
  })
  if (!res.ok) throw new Error(`计数服务回了 ${res.status}`)
  const text = await res.text()
  /* 响形是 `try{cb({"site_uv":..,"page_pv":..});}catch(e){}` —— 只取 cb(...) 里那个对象 */
  const m = text.match(/cb\((\{.*?\})\)/)
  if (!m) throw new Error(`认不出计数服务的响形：${text.slice(0, 80)}`)
  const body = JSON.parse(m[1])

  /* **读一次就 +1**，这个接口没有只读口子。所以把"这台机器读了几次"记在本地，
   * 显示时减掉 —— 否则翻看得越多、数字越大，趋势就是假的。 */
  const key = path || ''
  bumpRead(day, key)
  /* 只看 page_pv：它按 (主域名, path) 记，正是"这个桶"的台次。
     `site_pv` 是整条命名空间的累计，跟"哪天"无关，不拿它当日的数。 */
  return Math.max(0, Number(body.page_pv ?? 0) - readsOf(day, key))
}

/* ------------------------- 查看记录（只在本机） ------------------------- */

/**
 * `{ "2026-11-23": { "": 3, "0.0.18-preview.2/macos": 1 } }` —— 这台机器在某个桶上
 * 读过几次。`""` 是 origin 的根（当天总数）。
 *
 * 放 `.scratch/`（git 忽略）：它是**读的人**的记账，不是产品数据。
 */
const LEDGER = '.scratch/usage-reads.json'

function loadLedger() {
  try {
    return JSON.parse(readFileSync(LEDGER, 'utf8'))
  } catch {
    return {}
  }
}

/** 记账的键带上命名空间：换了计数域，旧读数不该继续跟着减 */
const scope = (day) => `${HOST_SUFFIX}|${day}`

function readsOf(day, key) {
  return Number(loadLedger()[scope(day)]?.[key] ?? 0)
}

function bumpRead(day, key) {
  const ledger = loadLedger()
  const k = scope(day)
  ledger[k] ??= {}
  ledger[k][key] = Number(ledger[k][key] ?? 0) + 1
  mkdirSync('.scratch', { recursive: true })
  writeFileSync(LEDGER, JSON.stringify(ledger, null, 2))
}

/** 本地已发布过的版本号（tag + 当前开发号），用来问版本分布 */
function knownVersions() {
  const set = new Set()
  try {
    const tags = execSync('git --no-pager tag --list "v*"', { encoding: 'utf8' })
    for (const t of tags.split('\n')) {
      const v = t.trim().replace(/^v/, '')
      if (v) set.add(v)
    }
  } catch {
    /* 不是 git 仓库就只问当前号 */
  }
  try {
    const pkg = JSON.parse(readFileSync('package.json', 'utf8'))
    if (pkg.version) set.add(pkg.version)
  } catch {
    /* 读不到就算了 */
  }
  return [...set]
}

async function dayTotal(day) {
  const n = await readBucket(day)
  await sleep(GAP_MS)
  return n
}

/** 一段日期的台次（一天一次请求） */
async function collectTrend(list) {
  const out = []
  for (const day of list) out.push({ day, total: await dayTotal(day) })
  return out
}

/** 某天的版本 / 平台分布（版本 × 平台逐个问，0 的不列） */
async function collectBreakdown(day) {
  const rows = []
  for (const v of knownVersions()) {
    for (const p of PLATFORMS) {
      const n = await readBucket(day, `${v}/${p}`)
      if (n > 0) rows.push({ version: v, platform: p, n })
      await sleep(GAP_MS)
    }
  }
  return rows.sort((a, b) => b.n - a.n)
}

function printBreakdown(day, total, rows) {
  console.log(`\n  ${day} · ${total} 台次\n`)
  if (rows.length === 0) {
    console.log('  这一天没有分版本记录（可能是没有心跳，也可能版本号不在已知列表里）\n')
    return
  }
  const w = Math.max(...rows.map((r) => r.version.length))
  for (const r of rows) {
    console.log(`  ${r.version.padEnd(w)}  ${r.platform.padEnd(8)} ${r.n}`)
  }
  console.log('')
}

function printTrend(list, rows) {
  console.log('')
  for (const r of rows) {
    const bar = '█'.repeat(Math.min(40, r.total))
    console.log(`  ${r.day}  ${String(r.total).padStart(4)}  ${bar}`)
  }
  const last = rows[rows.length - 1]
  console.log('')
  console.log(`  最近 ${list.length} 天共 ${rows.reduce((s, r) => s + r.total, 0)} 台次；今天 ${last.total}`)
  console.log(`  单日分布：node scripts/usage.mjs --day ${last.day}`)
  console.log('')
}

/* --------------------------- 网页版报告（本机文件） --------------------------- */

const esc = (s) =>
  String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])

function renderHtml(rows, latest) {
  const max = Math.max(1, ...rows.map((r) => r.total))
  const sum = rows.reduce((s, r) => s + r.total, 0)
  const bars = rows
    .map((r) => {
      const h = Math.round((r.total / max) * 160)
      return `<div class="col" title="${r.day} · ${r.total} 台次">
        <div class="n">${r.total || ''}</div>
        <div class="bar" style="height:${r.total === 0 ? 2 : h}px"></div>
        <div class="d">${r.day.slice(5)}</div>
      </div>`
    })
    .join('')
  const bd = latest.rows.length
    ? latest.rows
        .map((r) => `<tr><td>${esc(r.version)}</td><td>${esc(r.platform)}</td><td class="num">${r.n}</td></tr>`)
        .join('')
    : '<tr><td colspan="3" class="empty">今天还没有心跳记录</td></tr>'

  return `<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8">
<title>SupportEase 使用统计</title>
<style>
  :root { color-scheme: light dark; --fg:#1d1f23; --dim:#6b7280; --line:#e5e7eb; --bar:#3b82f6; --bg:#fff }
  @media (prefers-color-scheme: dark) { :root { --fg:#e8eaed; --dim:#9aa0a6; --line:#2a2d33; --bar:#60a5fa; --bg:#16181c } }
  * { box-sizing: border-box }
  body { margin:0; padding:32px; background:var(--bg); color:var(--fg);
         font:14px/1.55 -apple-system, "PingFang SC", system-ui, sans-serif }
  h1 { font-size:18px; margin:0 0 4px }
  .sub { color:var(--dim); font-size:12px; margin-bottom:28px }
  .kpi { display:flex; gap:36px; margin-bottom:28px }
  .kpi div { font-size:12px; color:var(--dim) }
  .kpi b { display:block; font-size:26px; font-weight:600; color:var(--fg); line-height:1.2 }
  .chart { display:flex; align-items:flex-end; gap:6px; height:210px;
           border-bottom:1px solid var(--line); padding-bottom:6px }
  .col { flex:1; display:flex; flex-direction:column; justify-content:flex-end; align-items:center; gap:2px }
  .col .n { font-size:10px; color:var(--dim) }
  .col .bar { width:100%; max-width:34px; background:var(--bar); border-radius:2px 2px 0 0; opacity:.85 }
  .col .d { font-size:10px; color:var(--dim); margin-top:4px; white-space:nowrap }
  h2 { font-size:13px; font-weight:600; margin:32px 0 8px }
  table { border-collapse:collapse; min-width:320px }
  th, td { text-align:left; padding:6px 12px; border-bottom:1px solid var(--line); font-size:13px }
  th { color:var(--dim); font-weight:500; font-size:12px }
  td.num { text-align:right; font-variant-numeric:tabular-nums }
  .empty { color:var(--dim) }
  footer { margin-top:32px; color:var(--dim); font-size:11px }
</style></head><body>
<h1>SupportEase 使用统计</h1>
<div class="sub">每台机器每天最多报一次 · 生成于 ${esc(new Date().toLocaleString('zh-CN'))}</div>
<div class="kpi">
  <div>最近 ${rows.length} 天台次<b>${sum}</b></div>
  <div>今天 <b>${latest.day}</b></div>
  <div>今天台次<b>${rows[rows.length - 1].total}</b></div>
</div>
<div class="chart">${bars}</div>
<h2>今天 · 版本 / 平台</h2>
<table><thead><tr><th>版本</th><th>平台</th><th class="num">台次</th></tr></thead>
<tbody>${bd}</tbody></table>
<footer>「台次」= 当天报过心跳的机器数；离线、或关掉「在线增强」的机器不会出现在这里。</footer>
</body></html>`
}

async function main() {
  if (onlyDay) {
    const total = await dayTotal(onlyDay)
    const rows = await collectBreakdown(onlyDay)
    printBreakdown(onlyDay, total, rows)
    return
  }

  const list = dayList(days)
  const rows = await collectTrend(list)
  if (!html) {
    printTrend(list, rows)
    return
  }
  const day = rows[rows.length - 1].day
  const bd = await collectBreakdown(day)
  mkdirSync('.scratch', { recursive: true })
  const file = '.scratch/usage-report.html'
  writeFileSync(file, renderHtml(rows, { day, rows: bd }))
  console.log(`\n  报告：${process.cwd()}/${file}\n`)
  try {
    execSync(`open "${file}"`)
  } catch {
    /* 没有 open（非 macOS）就只给路径 */
  }
}

main().catch((e) => {
  console.error(`\n✗ 读不到：${e.message}\n`)
  console.error('  计数服务是个人维护的免费服务，挂了 / 被限流都可能；过一会儿再试。\n')
  process.exit(1)
})
