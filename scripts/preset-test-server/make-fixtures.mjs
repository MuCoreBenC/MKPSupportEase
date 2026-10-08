#!/usr/bin/env node
/*
 * **从仓库里那份真交付根派生两代夹具**（2026-10-08）。
 *
 * ```bash
 * npm run preset-source:make           # 走 v1/v2 两代
 * ```
 *
 * # 为什么是"派生"而不是手写两份 JSON
 *
 * `catalog.json` 里除了文件清单，还有机型 / 资产 / 套餐 / 字段定义（参数页的整张表都从它摊）。
 * 手写一份**必然与真目录漂移** —— 漂了之后这个测试源就不再是"真形状"，测出来的结论也不作数。
 * 所以这一份的做法是：**读真的，只改那几处必须改的**：
 *
 *   ① `files` 只留两份 MKP 预设（测试够用，夹具也小）；
 *   ② 每份的 `path` 改成 `mkp/presets/<名字>`，配合 `source.json` 的 `filesRoot: "."`，
 *      文件就住在夹具根底下 —— 测试服务端的就是它自己的目录；
 *   ③ `structureSignature` / `minClientVersion` / 机型 / 注册表**一律照抄**：
 *      客户端"读不读得懂这一代"的判定（`structure::can_read`）才不会平白翻脸。
 *
 * 两代的差别：
 *
 *   v1  `# release_time: 2026-10-08`，`A1-fast.toml` 的 `offset_y = 26.3`
 *   v2  `# release_time: 2026-10-15`，`offset_y = 26.8`（**同一份预设的下一版**）
 *
 * 于是「本地下了 v1 → 云端有 v2 → 再下一份新的我的预设」这条链在本地就能整条走通。
 * 真正要换第三版时：改 `fixtures/vX` 里的 TOML，再 `npm run preset-source:sync`。
 *
 * 生成物**不进版本库**（`fixtures/.gitignore` 把它们挡住）：它们是派生产物，
 * 谁需要谁跑一次，比在仓库里维护两份会过期的 JSON 干净。
 */

import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '..', '..')
const DELIVERY = join(REPO, 'presets', 'delivery')

/** 夹具里放哪几份预设（真交付根里有的名字） */
const PICKED = ['A1-standard.toml', 'A1-fast.toml']

/** v2 相对 v1 要改的那几处：发布日 + 一个看得见的参数值 */
const V2_RELEASE_TIME = '2026-10-15 09:00:00'
const V1_RELEASE_TIME = '2026-10-08 09:00:00'
const V1_PARAM = /^offset_y\s*=\s*-?[\d.]+\s*(#.*)?$/m
const V2_PARAM_LINE = (comment) => `offset_y = 26.8${comment ?? ''}`
const V1_PARAM_LINE = (comment) => `offset_y = 26.3${comment ?? ''}`

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

const revisionOf = (catalog) =>
  createHash('sha256')
    .update(
      [
        'mkpse-test-catalog/revision',
        catalog.publishedAt ?? '',
        ...catalog.files
          .map((f) => `${f.fileName}=${f.sha256 ?? ''}:${f.size ?? ''}@${f.path}`)
          .sort(),
      ].join('\n'),
    )
    .digest('hex')
    .slice(0, 16)

/** 改一行值，**行尾注释原样留着**（与真机那条保真写回同一个口径） */
const withParam = (text, line) => {
  const hit = V1_PARAM.exec(text)
  if (hit === null) throw new Error('真预设里找不到 offset_y —— 夹具推导的前提变了，看 make-fixtures.mjs 顶上那段')
  const comment = (hit[1] ?? '').trimEnd()
  return text.replace(V1_PARAM, line(comment === '' ? '' : ` ${comment}`))
}

const withReleaseTime = (text, at) => {
  if (!/^# release_time:/m.test(text)) {
    throw new Error('真预设头上没有 # release_time —— 夹具推导的前提变了')
  }
  return text.replace(/^# release_time:.*$/m, `# release_time: ${at}`)
}

const realCatalog = JSON.parse(await readFile(join(DELIVERY, 'catalog.json'), 'utf8'))
const realSource = JSON.parse(await readFile(join(DELIVERY, 'source.json'), 'utf8'))

/** 真交付根里一份 MKP 预设的字节在哪儿（catalog.path 是相对 `presets/` 的） */
const readReal = async (name) => {
  const entry = realCatalog.files.find((f) => f.fileName === name && f.kind === 'mkp_preset')
  if (entry === undefined) throw new Error(`真目录里没有 ${name}`)
  return { entry, bytes: await readFile(join(REPO, 'presets', entry.path)) }
}

const filesV1 = new Map()
const filesV2 = new Map()
for (const name of PICKED) {
  const { entry, bytes } = await readReal(name)
  const text = bytes.toString('utf8')
  filesV1.set(name, {
    meta: entry,
    text: withParam(withReleaseTime(text, V1_RELEASE_TIME), V1_PARAM_LINE),
  })
  filesV2.set(name, { meta: entry, text: withParam(withReleaseTime(text, V2_RELEASE_TIME), V2_PARAM_LINE) })
}

const build = (version, files, publishedAt) => {
  const out = resolve(HERE, 'fixtures', version)
  return (async () => {
    /*
     * **原地覆盖，不删目录**：目录结构是固定的那六份文件，覆盖就是全量重写 ——
     * 也就不需要"先清空"这一步（清空还得处理"删不动"的各种环境差异）。
     */
    await mkdir(join(out, 'mkp', 'presets'), { recursive: true })

    const entries = []
    for (const [name, { meta, text }] of files) {
      const bytes = Buffer.from(text, 'utf8')
      await writeFile(join(out, 'mkp', 'presets', name), bytes)
      entries.push({
        kind: 'mkp_preset',
        fileName: name,
        /* filesRoot: "." ⇒ 落点就是夹具根底下的这一条 */
        path: `mkp/presets/${name}`,
        machineId: meta.machineId,
        versionId: meta.versionId,
        sha256: sha256(bytes),
        size: bytes.length,
      })
    }

    const catalog = {
      ...realCatalog,
      revision: '',
      publishedAt,
      files: entries,
    }
    catalog.revision = revisionOf(catalog)

    await writeFile(join(out, 'catalog.json'), `${JSON.stringify(catalog, null, 2)}\n`)
    /* source.json 只改一处：filesRoot 从 ".." 收成 "." —— 夹具自己就是一个交付根 */
    await writeFile(
      join(out, 'source.json'),
      `${JSON.stringify({ ...realSource, filesRoot: '.' }, null, 2)}\n`,
    )
    /* manifest / release 照抄：客户端会按 manifest 找 release.json，缺一个就少一条链 */
    for (const f of ['manifest.json', 'release.json']) {
      await writeFile(join(out, f), await readFile(join(DELIVERY, f)))
    }
    console.log(`[preset-source:make] ${version} → ${out}  revision=${catalog.revision}`)
  })()
}

const keep = resolve(HERE, 'fixtures', '.gitignore')
await mkdir(dirname(keep), { recursive: true })
await writeFile(keep, '# 派生产物：`npm run preset-source:make` 生成，不进版本库（会与真目录漂移）\n*\n!.gitignore\n')

await build('v1', filesV1, '2026-10-08T09:00:00Z')
await build('v2', filesV2, '2026-10-15T09:00:00Z')
console.log('[preset-source:make] 两代夹具就绪：v1（旧）/ v2（新）')
