#!/usr/bin/env node
/*
 * **重算一份夹具的指纹**（2026-10-08）。
 *
 * 夹具目录里的交付文件是手改的（改一个参数、把 `# release_time` 推到下周……），
 * 但目录说明书写着的 `sha256` / `size` / `revision` 是**对真字节算的** ——
 * 手工维护它们必然漂移，而漂移的表现是客户端**下载时报 SHA 不匹配**
 * （那条错看起来像"文件坏了"，其实只是说明书没跟上）。
 *
 * 所以：**改了夹具文件就跑一次这个**。
 *
 * ```bash
 * npm run preset-source:sync                                   # 默认 fixtures/v1
 * node sync-fixture.mjs --root fixtures/v2 --at 2026-10-15T00:00:00Z
 * ```
 *
 * 三件事：
 *   ① 逐份重算 `files[].sha256` / `files[].size`（按 `path` 读盘上的真字节）；
 *   ② 换一代 `publishedAt`（默认取当下 UTC；`--at` 可以钉死一个时刻）；
 *   ③ 按「这一代登记了什么」重算 `revision`（客户端"有没有新版"比的就是它）。
 *
 * 发布时刻为什么在这里换：客户端的「检查更新」比的是 `revision` 指纹，
 * **不换它就看不出"官方发新版了"**（那正是这个测试源要演示的第一件事）。
 */

import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))

const argv = process.argv.slice(2)
const argOf = (name) => {
  const at = argv.indexOf(name)
  return at >= 0 && argv[at + 1] !== undefined ? argv[at + 1] : undefined
}
const DIR = resolve(argOf('--root') ?? join(HERE, 'fixtures', 'v1'))
const AT = argOf('--at') ?? new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')

/** 指纹算法：只认「这一代登记了什么」——与具体时间无关的稳定摘要（16 位 hex） */
const revisionOf = (catalog) => {
  const lines = [
    'mkpse-test-catalog/revision',
    catalog.publishedAt ?? '',
    ...catalog.files
      .map((f) => `${f.fileName}=${f.sha256 ?? ''}:${f.size ?? ''}@${f.path}`)
      .sort(),
  ].join('\n')
  return createHash('sha256').update(lines).digest('hex').slice(0, 16)
}

const catalogPath = join(DIR, 'catalog.json')
const catalog = JSON.parse(await readFile(catalogPath, 'utf8'))

let changed = 0
for (const file of catalog.files) {
  const bytes = await readFile(join(DIR, file.path))
  const sha256 = createHash('sha256').update(bytes).digest('hex')
  if (file.sha256 !== sha256 || file.size !== bytes.length) changed += 1
  file.sha256 = sha256
  file.size = bytes.length
}
catalog.publishedAt = AT
catalog.revision = revisionOf(catalog)

/* 换行统一成 LF、缩进两个空格：夹具要能被 git 干净地 diff */
await writeFile(catalogPath, `${JSON.stringify(catalog, null, 2)}\n`)

console.log(`[preset-source:sync] ${DIR}`)
console.log(`[preset-source:sync] ${catalog.files.length} 份文件（${changed} 份指纹变了）`)
console.log(`[preset-source:sync] revision = ${catalog.revision}  publishedAt = ${catalog.publishedAt}`)
