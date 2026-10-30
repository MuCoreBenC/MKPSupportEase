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

/**
 * 语言名这几条本来就该用**它自己的文字**写（"简体中文" / "繁體中文"），不算漏翻。
 * 2026-10-30 加繁体时一并把 `zhTW` 放进来。
 */
const CJK_ALLOWED = new Set(['settings.locale.zh', 'settings.locale.zhTW'])

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
const zhTW = parseDict(await readFile(join(i18nDir, 'zh-TW.ts'), 'utf8'), 'zh-TW.ts')

/*
 * 除源语言之外每多一本，就多查一遍 —— 加语言只改这张表。
 *
 * `cjk: true` 的那本要做"不许夹中文"的检查（英文）；繁中当然全是中文，豁免。
 */
const OTHERS = [
  { file: 'en.ts', dict: en, cjk: true },
  { file: 'zh-TW.ts', dict: zhTW, cjk: false },
]

/* ① 键集一致 */
for (const { file, dict } of OTHERS) {
  for (const key of zh.keys()) if (!dict.has(key)) problems.push(`${file} 缺键：${key}`)
  for (const key of dict.keys()) if (!zh.has(key)) problems.push(`zh-CN.ts 缺键：${key}（${file} 多出来的）`)
}

/* ② 空值 */
for (const [file, dict] of [['zh-CN.ts', zh], ...OTHERS.map((o) => [o.file, o.dict])]) {
  for (const [key, value] of dict) if (value.trim() === '') problems.push(`${file} 空值：${key}`)
}

/* ③ 占位符一致 + ④ 英文里不许有中文 */
for (const { file, dict, cjk } of OTHERS) {
  for (const [key, zhValue] of zh) {
    const value = dict.get(key)
    if (value === undefined) continue

    const a = [...placeholders(zhValue)].sort().join(',')
    const b = [...placeholders(value)].sort().join(',')
    if (a !== b) problems.push(`${key}: 占位符不一致（中文 {${a}} vs ${file} {${b}}）`)

    if (cjk && hasCjk(value) && !CJK_ALLOWED.has(key)) {
      problems.push(`${file} 里还有中文：${key} = ${value}`)
    }
  }
}

const total = zh.size
if (problems.length) {
  console.error(`✗ 字典校验没通过（${problems.length} 条）：`)
  for (const p of problems) console.error(`  · ${p}`)
  process.exitCode = 1
} else {
  console.log(
    `✓ 字典校验通过：${total} 条键，中文 / 繁體中文 / 英文（${OTHERS.length + 1} 本）键集与占位符完全一致`,
  )
}
