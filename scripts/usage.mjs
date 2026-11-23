#!/usr/bin/env node
/**
 * **看「今天有多少人用过」** —— 读客户端那一次匿名心跳在计数服务上留下的数。
 *
 * ```text
 * node scripts/usage.mjs                    # 最近 14 天，一天一行
 * node scripts/usage.mjs --days 30          # 换窗口
 * node scripts/usage.mjs --day 2026-11-23   # 单日，外加版本 / 平台分布
 * ```
 *
 * # 分桶规则（与 `src-tauri/src/runtime/usage_ping.rs` 必须一致）
 *
 * ```text
 * Referer: https://d2026-11-23.mkp-usage.test/0.0.18-preview.2/macos
 *          └────── 一天一个 origin ──────┘└──── 版本 ────┘└ 平台 ┘
 * ```
 *
 *   · `site_pv`（同一个 origin）  = 当天总台次
 *   · `page_pv`（origin + path）  = 当天这个版本这个平台的台次
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
const HOST_SUFFIX = 'mkp-usage.test'
const PLATFORMS = ['macos', 'windows', 'linux']
/** 两次请求之间歇多久：那是个人维护的免费服务，别打它 */
const GAP_MS = 150

const args = process.argv.slice(2)
const getArg = (name, fallback) => {
  const i = args.indexOf(`--${name}`)
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback
}

const days = Number(getArg('days', '14'))
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
  const referer = `https://d${day}.${HOST_SUFFIX}/${path}`
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
  return {
    sitePv: Math.max(0, Number(body.site_pv ?? 0) - totalReads(day)),
    pagePv: Math.max(0, Number(body.page_pv ?? 0) - readsOf(day, key)),
  }
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

function totalReads(day) {
  return Object.values(loadLedger()[day] ?? {}).reduce((s, n) => s + Number(n), 0)
}

function readsOf(day, key) {
  return Number(loadLedger()[day]?.[key] ?? 0)
}

function bumpRead(day, key) {
  const ledger = loadLedger()
  ledger[day] ??= {}
  ledger[day][key] = Number(ledger[day][key] ?? 0) + 1
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
  const { sitePv } = await readBucket(day)
  await sleep(GAP_MS)
  return sitePv
}

async function main() {
  if (onlyDay) {
    const total = await dayTotal(onlyDay)
    console.log(`\n  ${onlyDay} · ${total} 台次\n`)
    const rows = []
    for (const v of knownVersions()) {
      for (const p of PLATFORMS) {
        const { pagePv } = await readBucket(onlyDay, `${v}/${p}`)
        if (pagePv > 0) rows.push({ version: v, platform: p, n: pagePv })
        await sleep(GAP_MS)
      }
    }
    if (rows.length === 0) {
      console.log('  这一天没有分版本记录（可能是没有心跳，也可能版本号不在已知列表里）\n')
      return
    }
    rows.sort((a, b) => b.n - a.n)
    const w = Math.max(...rows.map((r) => r.version.length))
    for (const r of rows) {
      console.log(`  ${r.version.padEnd(w)}  ${r.platform.padEnd(8)} ${r.n}`)
    }
    console.log('')
    return
  }

  const list = dayList(days)
  const out = []
  for (const day of list) {
    out.push({ day, total: await dayTotal(day) })
  }
  console.log('')
  for (const r of out) {
    const bar = '█'.repeat(Math.min(40, r.total))
    console.log(`  ${r.day}  ${String(r.total).padStart(4)}  ${bar}`)
  }
  const last = out[out.length - 1]
  console.log('')
  console.log(`  最近 ${days} 天共 ${out.reduce((s, r) => s + r.total, 0)} 台次；今天 ${last.total}`)
  console.log('  想看某天的版本 / 平台分布：node scripts/usage.mjs --day ' + last.day)
  console.log('')
}

main().catch((e) => {
  console.error(`\n✗ 读不到：${e.message}\n`)
  console.error('  计数服务是个人维护的免费服务，挂了 / 被限流都可能；过一会儿再试。\n')
  process.exit(1)
})
