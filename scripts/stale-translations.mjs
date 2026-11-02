#!/usr/bin/env node
/*
 * 译文待办清单（2026-11-02）。
 *
 * 作者的口径：「我改了中文，但是其他没有改，这种能不能记录下来，让 AI 去改」。
 *
 * 工作台里中文一改，后端就把"跟着一起过期"的译文键名记进同一条 `[[params]]` 的
 * `staleTranslations`（`labelEn` / `descJa` 这种）。这个脚本把那份台账翻成人能读、
 * AI 能直接改的清单：一条参数一节，中文原文 + 每一册现在的样子 + 哪几格要重写。
 *
 * # 用法
 *
 *   node scripts/stale-translations.mjs            # 打印清单（有欠账时退出码 1）
 *   node scripts/stale-translations.mjs --json     # 给程序读
 *   node scripts/stale-translations.mjs --check    # 只判有没有欠账（CI 用）
 *
 * 它**只读**：改译文请改 `presets/registry/param_registry.toml` 的 `labelEn` 等键，
 * 改完把那一册从 `staleTranslations` 里摘掉（工作台里重写那一格再保存会自动摘）。
 */
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const REGISTRY = join(ROOT, 'presets', 'registry', 'param_registry.toml')

const LANGS = [
  { key: 'labelEn', desc: 'descEn', title: 'English' },
  { key: 'labelJa', desc: 'descJa', title: '日本語' },
  { key: 'labelTw', desc: 'descTw', title: '繁體中文' },
]

/** 只认**字符串**标量与字符串数组；`[[params]]` 里没有别的形态需要读 */
const unquote = (raw) => {
  const t = raw.trim()
  if (t.startsWith('"""') || t.startsWith("'''")) return t.slice(3, -3)
  if (
    (t.startsWith('"') && t.endsWith('"')) ||
    (t.startsWith("'") && t.endsWith("'"))
  ) {
    const body = t.slice(1, -1)
    return t.startsWith('"') ? JSON.parse(`"${body.replace(/"/g, '\\"')}"`) : body
  }
  return t
}

/**
 * 极简的 `[[params]]` 切块：只按行扫，不引 TOML 库。
 * 注册表这一份是「一参数一表数组」的扁平结构，键值都是一行 —— 够用，且少了依赖。
 */
function parseParams(text) {
  const out = []
  let cur = null
  let inArray = null
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (trimmed === '[[params]]') {
      cur = {}
      out.push(cur)
      inArray = null
      continue
    }
    if (cur === null) continue
    if (trimmed.startsWith('[')) {
      cur = trimmed.startsWith('[[params.') || trimmed.startsWith('[params.') ? cur : null
      if (cur === null) inArray = null
      continue
    }
    if (inArray !== null) {
      if (trimmed.endsWith(']')) {
        for (const piece of trimmed.replace(/\]$/, '').split(',')) {
          const v = piece.trim()
          if (v !== '') cur[inArray].push(unquote(v))
        }
        inArray = null
      } else {
        for (const piece of trimmed.split(',')) {
          const v = piece.trim()
          if (v !== '') cur[inArray].push(unquote(v))
        }
      }
      continue
    }
    const eq = trimmed.indexOf('=')
    if (eq <= 0) continue
    const k = trimmed.slice(0, eq).trim()
    const v = trimmed.slice(eq + 1).trim()
    if (v.startsWith('[') && !v.endsWith(']')) {
      cur[k] = []
      inArray = k
      continue
    }
    if (v.startsWith('[')) {
      cur[k] = v
        .replace(/^\[/, '')
        .replace(/\]$/, '')
        .split(',')
        .map((x) => x.trim())
        .filter((x) => x !== '')
        .map(unquote)
      continue
    }
    cur[k] = unquote(v)
  }
  return out
}

const text = readFileSync(REGISTRY, 'utf8')
const stale = []
for (const p of parseParams(text)) {
  const keys = Array.isArray(p.staleTranslations) ? p.staleTranslations : []
  if (keys.length === 0) continue
  stale.push({
    key: p.key ?? '(没有 key)',
    label: p.label ?? '',
    desc: p.desc ?? '',
    stale: keys,
    langs: LANGS.map((l) => ({
      title: l.title,
      labelKey: l.key,
      descKey: l.desc,
      label: p[l.key] ?? '',
      desc: p[l.desc] ?? '',
      staleLabel: keys.includes(l.key),
      staleDesc: keys.includes(l.desc),
    })),
  })
}

if (process.argv.includes('--json')) {
  process.stdout.write(`${JSON.stringify(stale, null, 2)}\n`)
  process.exit(stale.length > 0 ? 1 : 0)
}

if (process.argv.includes('--check')) {
  if (stale.length === 0) {
    console.log('译文台账是干净的：没有跟着中文一起过期的译本。')
    process.exit(0)
  }
  console.error(`有 ${stale.length} 条参数的译文过期了：`)
  for (const s of stale) console.error(`  ${s.key}  ← ${s.stale.join(', ')}`)
  console.error('清单看 `node scripts/stale-translations.mjs`。')
  process.exit(1)
}

if (stale.length === 0) {
  console.log('译文台账是干净的：没有跟着中文一起过期的译本。')
  process.exit(0)
}

console.log(`# 译文待办 · ${stale.length} 条\n`)
console.log(
  '这些参数的中文改过，下面的译本还是旧中文的译文。请按现行中文重写带 ⚠ 的那几格，' +
    '改 `presets/registry/param_registry.toml` 里同一个 `[[params]]` 块；重写之后把那一项从 ' +
    '`staleTranslations` 里删掉。\n',
)
for (const s of stale) {
  console.log(`## ${s.key}\n`)
  console.log(`- 中文名：${s.label}`)
  console.log(`- 中文说明：${s.desc || '(空)'}`)
  for (const l of s.langs) {
    const mark = (x) => (x ? ' ⚠ 过期' : '')
    console.log(
      `- ${l.title}：名 = ${l.label ? JSON.stringify(l.label) : '(空，回落中文)'}${mark(l.staleLabel)}` +
        `｜说明 = ${l.desc ? JSON.stringify(l.desc) : '(空，回落中文)'}${mark(l.staleDesc)}`,
    )
  }
  console.log('')
}
process.exit(1)
