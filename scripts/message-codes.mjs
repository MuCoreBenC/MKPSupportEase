#!/usr/bin/env node
/*
 * 码表查询 —— **给 AI / 给排查的人**一个入口（2026-10-17，作者：
 * 「这样就方便我给 AI 去看啊，这样子，就算信息不全，也能通过错误码找到对应的」）。
 *
 * 用法：
 *   node scripts/message-codes.mjs                 # 列出整张码表（按码排序）
 *   node scripts/message-codes.mjs E_CAL_FAILED_001
 *   node scripts/message-codes.mjs W_LINE_CANCELLED_001
 *   node scripts/message-codes.mjs hook.cancelled  # 也可以拿**键**查
 *   node scripts/message-codes.mjs 校准            # 模糊：码 / 键 / 文案里含这几个字
 *
 * 它是**只读**的：直接解析 `crates/postprocess/messages.toml`（那份是 `include_str!`
 * 编进二进制的真值）。不重新实现解析规则 —— 只按"行首 `[` 开新条目"这一条切块，
 * 足够稳，也够简单。
 *
 * 前缀约定（同 `messages.toml` 表头）：
 *   E_ 故障 · W_ 停下/降级 · N_ 成功 · Q_ 提问
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const TABLE = path.join(here, '..', 'crates', 'postprocess', 'messages.toml')

const CLASS_DESC = {
  E: '故障 —— 系统判定这件事没成',
  W: '停下 / 降级 —— 不是故障：用户让它停的，或某一步降级了',
  N: '成功 —— 跑完了（不同结果各有一码）',
  Q: '提问 —— 钩子停下来问用户的那几句',
}

/** 把整张表切成 `[{ key, fields }]`。 */
function parseTable(text) {
  const entries = []
  let cur = null
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trimEnd()
    const head = line.match(/^\["([^"]+)"\]$/)
    if (head) {
      if (cur) entries.push(cur)
      cur = { key: head[1], fields: {} }
      continue
    }
    if (!cur) continue
    const kv = line.match(/^([a-z_]+)\s*=\s*"(.*)"$/)
    if (kv) cur.fields[kv[1]] = kv[2]
  }
  if (cur) entries.push(cur)
  return entries
}

/**
 * 码：条目自己声明的优先；否则 `E_` 打头的键本身就是码。
 * 与 `diag::messages::code_of` **同一条规则**（这里是只读镜像）。
 */
function codeOf(e) {
  if (e.fields.code) return e.fields.code
  return e.key.startsWith('E_') ? e.key : null
}

const entries = parseTable(fs.readFileSync(TABLE, 'utf8'))
const query = process.argv.slice(2).join(' ').trim()

if (!query) {
  /* ---- 整张表 ---- */
  const rows = entries
    .map((e) => ({ code: codeOf(e), key: e.key, text: e.fields.text ?? '' }))
    .sort((a, b) => (a.code ?? 'zzz').localeCompare(b.code ?? 'zzz'))

  const withCode = rows.filter((r) => r.code)
  const noCode = rows.filter((r) => !r.code)

  console.log(`码表：${withCode.length} 条有码，${noCode.length} 条无码（零件/框子）\n`)
  let lastClass = null
  for (const r of withCode) {
    const cls = r.code[0]
    if (cls !== lastClass) {
      lastClass = cls
      console.log(`── ${cls}_*  ${CLASS_DESC[cls] ?? '（未分类）'} ──`)
      console.log()
    }
    console.log(`${r.code.padEnd(26)} ${r.text}`)
  }
  if (noCode.length) {
    console.log('\n── 无码（零件条目，不是对用户说的话）──')
    for (const r of noCode) console.log(r.key)
  }
  console.log('\n查一条：node scripts/message-codes.mjs <码或键>')
  process.exit(0)
}

/* ---- 查一条（或模糊几句）---- */
const q = query.toLowerCase()
const hit = entries.filter((e) => {
  const code = codeOf(e) ?? ''
  return (
    e.key.toLowerCase() === q ||
    code.toLowerCase() === q ||
    e.key.toLowerCase().includes(q) ||
    code.toLowerCase().includes(q) ||
    (e.fields.text ?? '').toLowerCase().includes(q) ||
    (e.fields.text_en ?? '').toLowerCase().includes(q) ||
    (e.fields.slicer ?? '').toLowerCase().includes(q) ||
    (e.fields.slicer_en ?? '').toLowerCase().includes(q)
  )
})

if (hit.length === 0) {
  console.error(`没找到：${query}`)
  console.error('（前缀：E_ 故障 / W_ 停下 / N_ 成功 / Q_ 提问）')
  process.exit(1)
}

for (const e of hit) {
  const code = codeOf(e)
  const cls = code ? CLASS_DESC[code[0]] : null
  console.log('─'.repeat(64))
  console.log(code ? `${code}   ${cls ?? ''}` : '（无码）')
  console.log(`键      ${e.key}`)
  console.log(`本程序  ${e.fields.text ?? '—'}`)
  if (e.fields.text_en !== undefined) console.log(`English ${e.fields.text_en}`)
  if (e.fields.slicer !== undefined) console.log(`切片器  ${e.fields.slicer}`)
  if (e.fields.slicer_en !== undefined) console.log(`切片器英 ${e.fields.slicer_en}`)
  if (e.fields.params) console.log(`参数    ${e.fields.params}`)
  if (e.fields.note) console.log(`说明    ${e.fields.note}`)
}
console.log('─'.repeat(64))
