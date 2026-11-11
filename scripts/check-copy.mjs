#!/usr/bin/env node
/**
 * 文案闸：拦住「次要标签 / 说明句」那几种固定坏写法。
 *
 * 判据与写法见 `docs/COPY-STYLE.md`；这里只做**机械可判**的那几条，
 * 语义问题（这句该不该存在）仍要人看。
 *
 * # 只查描述位
 *
 * `.note` / `.hint` / `.tip` / `.title` / `.desc` / `.subtitle` 这些键是「控件旁边那句小字」，
 * 也正是最容易长出「破折号补解释、括号补机制」的地方。按钮名、状态词不在这里管。
 *
 * # 为什么有 baseline
 *
 * 存量文案里这类写法很多（1893 条里挑出来的），一次全改不现实。baseline 记下**已经欠着的**
 * 那些键：闸只拦「新长出来的」。要还账就把键从 baseline 删掉再改文案；
 * 确实要留（老包袱），`node scripts/check-copy.mjs --update` 重新记一遍。
 *
 * 用法：
 *   node scripts/check-copy.mjs            # 只报新增
 *   node scripts/check-copy.mjs --update   # 把当前全部违规写进 baseline
 */

import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = new URL('..', import.meta.url).pathname
const DICT = join(ROOT, 'src/i18n/zh-CN.ts')
const BASELINE = join(ROOT, 'scripts/copy-baseline.json')

/** 描述位：控件名旁边、分组标题下面那一句小字 */
const DESC_KEY = /\.(note|hint|tip|title|desc|subtitle|Note|Hint|Tip|Title|Desc|Subtitle)$/

/** 每条规则：命中即报，message 直接给人看 */
const RULES = [
  {
    id: 'dash',
    test: (v) => v.includes('——'),
    message: '破折号后面在补解释：能删就删，两件事分两句说',
  },
  {
    id: 'paren',
    test: (v) => {
      const m = v.match(/（([^）]*)）/g) ?? []
      return m.some((s) => s.length > 12)
    },
    message: '括号补丁：括号里只留单位、范围、必填标记，解释进 tooltip 或删',
  },
  {
    id: 'long',
    test: (v) => v.length > 40,
    message: '说明超过 40 字：一句说完，说不完就说明布局没交代清楚',
  },
  {
    id: 'meta',
    test: (v) => /如实|不冒充|不会丢什么|一个字都没动|请耐心等待|请稍候/.test(v),
    message: '安抚 / 元话术：删掉，用户只想知道会发生什么',
  },
  {
    id: 'legacy',
    test: (v) => /这一份|那一份|底账|基线|补齐字段|来源发布线|判定/.test(v),
    message: '内部说法上了界面：换成用户词（见 COPY-STYLE §5）',
  },
]

function readZh() {
  const lines = readFileSync(DICT, 'utf8').split('\n')
  const out = new Map()
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^\s*'([^']+)':\s*(.*)$/)
    if (!m) continue
    const key = m[1]
    const rest = m[2]
    if (rest.startsWith("'")) {
      const v = rest.match(/^'(.*)',?\s*$/)
      if (v) out.set(key, v[1])
    } else if (rest === '') {
      const v = (lines[i + 1] ?? '').match(/^\s*'(.*)',?\s*$/)
      if (v) out.set(key, v[1])
    }
  }
  return out
}

const zh = readZh()
const descKeys = [...zh.keys()].filter((k) => DESC_KEY.test(k))
const found = new Map() // key -> [{id, message, value}]
for (const [key, value] of zh) {
  if (!DESC_KEY.test(key)) continue
  const hits = RULES.filter((r) => r.test(value)).map((r) => ({ id: r.id, message: r.message }))
  if (hits.length > 0) found.set(key, { value, hits })
}

const update = process.argv.includes('--update')
if (update) {
  const keys = [...found.keys()].sort()
  writeFileSync(BASELINE, JSON.stringify({ note: 'check-copy 的欠账清单；只增不改旧的等于没还', keys }, null, 2) + '\n')
  console.log(`✓ 已把 ${keys.length} 条写进 ${BASELINE.replace(ROOT, '')}`)
  process.exit(0)
}

let baseline = new Set()
try {
  baseline = new Set(JSON.parse(readFileSync(BASELINE, 'utf8')).keys ?? [])
} catch {
  console.log('  提示：没有 baseline，先跑一次 node scripts/check-copy.mjs --update')
}

const fresh = [...found.entries()].filter(([key]) => !baseline.has(key))
const gone = [...baseline].filter((key) => !found.has(key))

if (fresh.length === 0) {
  console.log(`✓ 文案闸通过：描述位 ${descKeys.length} 条里，新增 0 条` + (gone.length ? `（有 ${gone.length} 条旧账已修好，可跑 --update 清掉）` : ''))
  process.exit(0)
}

console.error(`\n✗ 文案闸：新增 ${fresh.length} 条「说明句」坏写法\n`)
for (const [key, { value, hits }] of fresh.slice(0, 20)) {
  console.error(`  · ${key}`)
  console.error(`    ${value}`)
  for (const h of hits) console.error(`    → [${h.id}] ${h.message}`)
}
if (fresh.length > 20) console.error(`  … 还有 ${fresh.length - 20} 条`)
console.error('\n  写法见 docs/COPY-STYLE.md。确实要留（老包袱）跑：')
console.error('    node scripts/check-copy.mjs --update\n')
process.exit(1)
