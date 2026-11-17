#!/usr/bin/env node
/*
 * 译文待办清单（2026-11-02，2026-11-14 扩到机型版本）。
 *
 * 作者的口径：「我改了中文，但是其他没有改，这种能不能记录下来，让 AI 去改」。
 *
 * 两处台账，同一套判据：
 *
 *   presets/registry/param_registry.toml  `[[params]]` 的 `staleTranslations`
 *   presets/machines/*.toml               `[[versions]]` 的 `staleTranslations`
 *
 * 工作台里中文一改，后端就把"跟着一起过期"的译文键名记进去（`labelEn` / `tagJa` 这种）。
 * 这个脚本把那份台账翻成人能读、AI 能直接改的清单：一条一节，
 * **现行中文 + 每一册现在的样子 + 哪几格要重写**。
 *
 * # 用法
 *
 *   node scripts/stale-translations.mjs            # 打印清单（有欠账时退出码 1）
 *   node scripts/stale-translations.mjs --json     # 给程序读
 *   node scripts/stale-translations.mjs --check    # 只判有没有欠账（CI 用）
 *
 * 它**只读**：改译文请改对应的 TOML 键，改完把那一册从 `staleTranslations` 里摘掉
 * （工作台里重写那一格再保存会自动摘）。
 */
import { readdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const REGISTRY = join(ROOT, 'presets', 'registry', 'param_registry.toml')
const MACHINES = join(ROOT, 'presets', 'machines')

/** 只认**字符串**标量与字符串数组；两份 TOML 里没有别的形态需要读 */
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
 * 极简切块：按行扫，`header`（`[[params]]` / `[[versions]]`）开新块。
 * 这两份都是「一实体一表数组」的扁平结构，键值都是一行 —— 够用，且少了依赖。
 */
function parseBlocks(text, header) {
  const out = []
  let cur = null
  let inArray = null
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (trimmed === header) {
      cur = {}
      out.push(cur)
      inArray = null
      continue
    }
    if (cur === null) continue
    if (trimmed.startsWith('[')) {
      /* 子表（`[params.machineVariants]`）还挂在这个块上；换别的块就收手 */
      const keep = trimmed.startsWith('[params.') || trimmed.startsWith('[versions.')
      cur = keep ? cur : null
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

const str = (v) => (typeof v === 'string' ? v : '')

/* ---------- ① 参数注册表 ---------- */
const paramStale = []
for (const p of parseBlocks(readFileSync(REGISTRY, 'utf8'), '[[params]]')) {
  const keys = Array.isArray(p.staleTranslations) ? p.staleTranslations : []
  if (keys.length === 0) continue
  paramStale.push({
    kind: 'param',
    where: str(p.key),
    file: 'presets/registry/param_registry.toml',
    zh: { name: str(p.label), note: str(p.desc) },
    stale: keys,
    langs: [
      { title: 'English', name: str(p.labelEn), note: str(p.descEn), nameKey: 'labelEn', noteKey: 'descEn' },
      { title: '日本語', name: str(p.labelJa), note: str(p.descJa), nameKey: 'labelJa', noteKey: 'descJa' },
      { title: '繁體中文', name: str(p.labelTw), note: str(p.descTw), nameKey: 'labelTw', noteKey: 'descTw' },
    ],
  })
}

/* ---------- ② 机型版本（2026-11-14） ---------- */
const STALE_SLOTS = ['name', 'tag', 'description']
const versionStale = []
for (const f of readdirSync(MACHINES).filter((x) => x.endsWith('.toml'))) {
  const mid = f.replace(/\.toml$/, '')
  for (const v of parseBlocks(readFileSync(join(MACHINES, f), 'utf8'), '[[versions]]')) {
    const keys = Array.isArray(v.staleTranslations) ? v.staleTranslations : []
    if (keys.length === 0) continue
    versionStale.push({
      kind: 'version',
      where: `${mid}/${str(v.id)}`,
      file: `presets/machines/${f}`,
      zh: { name: str(v.name), tag: str(v.tag), note: str(v.description) },
      stale: keys,
      langs: [
        ['En', 'English'],
        ['Tw', '繁體中文'],
        ['Ja', '日本語'],
      ].map(([sfx, title]) => ({
        title,
        name: str(v[`name${sfx}`]),
        tag: str(v[`tag${sfx}`]),
        note: str(v[`description${sfx}`]),
        nameKey: `name${sfx}`,
        tagKey: `tag${sfx}`,
        noteKey: `description${sfx}`,
      })),
    })
  }
}

const stale = [...paramStale, ...versionStale]

if (process.argv.includes('--json')) {
  process.stdout.write(`${JSON.stringify(stale, null, 2)}\n`)
  process.exit(stale.length > 0 ? 1 : 0)
}

if (process.argv.includes('--check')) {
  if (stale.length === 0) {
    console.log('译文台账是干净的：没有跟着中文一起过期的译本。')
    process.exit(0)
  }
  console.error(`有 ${stale.length} 处译文过期了：`)
  for (const s of stale) console.error(`  ${s.where}  ← ${s.stale.join(', ')}`)
  console.error('清单看 `node scripts/stale-translations.mjs`。')
  process.exit(1)
}

if (stale.length === 0) {
  console.log('译文台账是干净的：没有跟着中文一起过期的译本。')
  process.exit(0)
}

const count = (k) => stale.filter((s) => s.kind === k).length
console.log(`# 译文待办 · ${stale.length} 处（参数 ${count('param')} / 机型版本 ${count('version')}）\n`)
console.log(
  '这些条目的中文改过，下面的译本还是旧中文的译文。请按**现行中文**重写带 ⚠ 的那几格，' +
    '改完把那一项从 `staleTranslations` 里删掉（工作台里重写那一格再保存会自动摘）。\n',
)
for (const s of stale) {
  console.log(`## ${s.kind === 'param' ? '参数' : '版本'} · ${s.where}\n`)
  console.log(`- 文件：${s.file}`)
  console.log(`- 中文名：${s.zh.name}`)
  if (s.zh.tag !== undefined) console.log(`- 中文标签：${s.zh.tag || '(空)'}`)
  console.log(`- 中文说明：${s.zh.note || '(空)'}`)
  for (const l of s.langs) {
    const mark = (key) => (s.stale.includes(key) ? ' ⚠ 过期' : '')
    const shown = (v) => (v ? JSON.stringify(v) : '(空，回落中文)')
    let line = `- ${l.title}：名 = ${shown(l.name)}${mark(l.nameKey)}`
    if (l.tagKey !== undefined) line += `｜标签 = ${shown(l.tag)}${mark(l.tagKey)}`
    line += `｜说明 = ${shown(l.note)}${mark(l.noteKey)}`
    console.log(line)
  }
  console.log('')
}
process.exit(1)
