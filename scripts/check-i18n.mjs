/*
 * 字典闸门（2026-10-25「全站 i18n」第一刀）：`npm run check:i18n`
 *
 * # 查四件事
 *
 *   ① 两本字典**键集完全一致**（漏翻 / 多写都红）；
 *   ② 值**不许为空**；
 *   ③ `{占位符}` 集合两边一致（少一个变量 = 句子里永远留着 `{name}`）；
 *   ④ 英文表里**不许出现中文**（唯一白名单：语言名 `settings.locale.zh`，
 *      它本来就该用中文写自己）。
 *
 * # 为什么用正则、不 import
 *
 * 字典是 `.ts`（键类型给 TypeScript 用，`en.ts` 靠 `Record<StringKey, string>`
 * 在编译期就钉住漏键）。脚本里不启 TS 编译器，而是靠 `zh-CN.ts` 头注释里那条
 * **格式硬约定**：扁平键、一行一条 `'键': '值',`。正则只认这个形状，
 * 写歪了会被当成"少了这条键"—— 这本身就是一道提醒。
 *
 * 真值：`src/i18n/zh-CN.ts`（源语言）+ `src/i18n/en.ts`。
 */

import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const i18nDir = join(here, '..', 'src', 'i18n')

/** 语言名这一条本来就该用中文写（"简体中文"），不算漏翻 */
const CJK_ALLOWED = new Set(['settings.locale.zh'])

/**
 * 从字典源码里抠出 `键 → 值`。
 *
 * 值的字面量允许跨行（英文有长句），所以用 `[\s\S]` 走而不是逐行扫。
 * 键只认 `字母开头 + 字母数字点下划线`（`bbs.gadgets.pin.off` 这个形状）。
 */
function parseDict(source, file) {
  const re = /'([A-Za-z][\w.]*)':\s*'((?:[^'\\]|\\.)*)'/g
  const out = new Map()
  for (const m of source.matchAll(re)) {
    const [, key, value] = m
    if (out.has(key)) throw new Error(`${file}: 键重复 —— ${key}`)
    out.set(key, value)
  }
  if (out.size === 0) throw new Error(`${file}: 一条都没解析出来（格式是不是被改坏了？）`)
  return out
}

const placeholders = (text) => new Set([...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]))
const hasCjk = (text) => /[\u3400-\u9fff]/.test(text)

const problems = []

const zh = parseDict(await readFile(join(i18nDir, 'zh-CN.ts'), 'utf8'), 'zh-CN.ts')
const en = parseDict(await readFile(join(i18nDir, 'en.ts'), 'utf8'), 'en.ts')

/* ① 键集一致 */
for (const key of zh.keys()) if (!en.has(key)) problems.push(`en.ts 缺键：${key}`)
for (const key of en.keys()) if (!zh.has(key)) problems.push(`zh-CN.ts 缺键：${key}（en.ts 多出来的）`)

/* ② 空值 */
for (const [file, dict] of [
  ['zh-CN.ts', zh],
  ['en.ts', en],
]) {
  for (const [key, value] of dict) if (value.trim() === '') problems.push(`${file} 空值：${key}`)
}

/* ③ 占位符一致 + ④ 英文里不许有中文 */
for (const [key, zhValue] of zh) {
  const enValue = en.get(key)
  if (enValue === undefined) continue

  const a = [...placeholders(zhValue)].sort().join(',')
  const b = [...placeholders(enValue)].sort().join(',')
  if (a !== b) problems.push(`${key}: 占位符不一致（中文 {${a}} vs 英文 {${b}}）`)

  if (hasCjk(enValue) && !CJK_ALLOWED.has(key)) problems.push(`${en.ts} 里还有中文：${key} = ${enValue}`)
}

const total = zh.size
if (problems.length) {
  console.error(`✗ 字典校验没通过（${problems.length} 条）：`)
  for (const p of problems) console.error(`  · ${p}`)
  process.exitCode = 1
} else {
  console.log(`✓ 字典校验通过：${total} 条键，中文 / 英文键集与占位符完全一致`)
}
