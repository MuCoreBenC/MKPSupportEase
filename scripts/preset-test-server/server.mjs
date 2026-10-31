#!/usr/bin/env node
/*
 * **本地官方源测试服务**（2026-10-08）。
 *
 * # 它解决的那件事
 *
 * 「我只是想测更新功能，为什么每次都要真的发布到云端？」
 *
 * 这个服务把一份**已发布形状的交付根**（`source.json` + `catalog.json` + `manifest.json`
 * + `release.json` + 交付文件）用普通 HTTP 端出去，客户端**不知道它是测试源** ——
 * 它还是走 catalog / manifest / 寻址规则 / SHA 校验 / 下载管道那一条真链。
 * 生产也还是同一个客户端逻辑，换的只是源地址（见 `dev.mjs` 注入的
 * `MKPSE_PRESET_SOURCE_URL`，只在 debug 构建里生效）。
 *
 * # 用法
 *
 * ```bash
 * npm run preset-source:dev            # 端仓库里的 presets（默认 127.0.0.1:8787）
 * node server.mjs --root <某预设根> --port 8787
 * ```
 *
 * # 端哪一份：`--root` → `PRESET_TEST_ROOT` → 仓库 `presets/`
 *
 * 第二档那个环境变量是给**工作台那颗按钮**用的（2026-10-08）：它起的永远是
 * `npm run preset-source:dev` 这同一条命令（`package.json` 里那行是契约），
 * 换源换的是环境变量 —— 于是"换源"不必给这条命令另开一条 argv 路径。
 * 参数仍然优先（手敲命令时照旧 `--root` 说了算）。
 *
 * # 发布序号自动 +1（2026-10-30）
 *
 * `PRESET_TEST_BUMP_SERIAL=1`（或 `--bump-serial`）时，返回的 `catalog.json` 里
 * `publishSerial` 往前拨一格、`publishedAt` 换成此刻 —— **只改响应，盘上一个字节不动**。
 *
 * 为什么要拨：客户端手上那份目录和这份源同号、内容却不同 ⇒ 逐文件判据给
 * `Inconsistent`（"那一代目录不再唯一"），看着像坏了。拨一格就成了正常的
 * "云端发了下一版"，和真发布一模一样。工作台那颗「启动」按钮默认带上这个开关。
 *
 * 起好之后可以把它填进「设置 → 高级设置 → 预设数据源 → 自定义地址」，
 * 或者用 `npm run dev:test-update` 让 dev 脚本自动注入。
 *
 * ★ **不引任何依赖**（`node:http` + `node:fs`）：它是个测试脚手架，不该进 package.json
 * 的依赖树，也不该被 `check-zero-network.mjs`（那份只扫 `src-tauri/src` 与 `src/api`）
 * 牵连 —— 客户端正式代码里一个网络符号都不多。
 */

import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join, normalize, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))

const argv = process.argv.slice(2)
const argOf = (name, fallback) => {
  const at = argv.indexOf(name)
  return at >= 0 && argv[at + 1] !== undefined ? argv[at + 1] : fallback
}

const PORT = Number(argOf('--port', process.env.PRESET_TEST_PORT ?? 8787))
const HOST = argOf('--host', '127.0.0.1')
/* 端哪一份：`--root` 参数 → `PRESET_TEST_ROOT`（工作台换源走这一条）→ 仓库里的 presets */
const ROOT = resolve(
  argOf('--root', process.env.PRESET_TEST_ROOT ?? join(HERE, '..', '..', 'presets')),
)
/* 发布序号 +1：见文件头那段 —— 只改响应，不改盘上的字节 */
const BUMP = argv.includes('--bump-serial') || process.env.PRESET_TEST_BUMP_SERIAL === '1'

const TYPES = {
  '.json': 'application/json; charset=utf-8',
  '.toml': 'text/plain; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.svg': 'image/svg+xml',
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? '/', `http://${HOST}:${PORT}`)
  const rel = decodeURIComponent(url.pathname).replace(/^\/+/, '')
  /* 目录访问试 `source.json`：写地址时少写一截也能用（与客户端那两条读法同一套宽容） */
  const target = resolve(ROOT, normalize(rel === '' || rel.endsWith('/') ? `${rel}source.json` : rel))
  /* 防穿越：只端出 ROOT 底下的东西 */
  if (target !== ROOT && !target.startsWith(ROOT + sep)) {
    res.writeHead(403, { 'content-type': 'text/plain; charset=utf-8' }).end('只能读测试根底下的文件')
    return
  }
  readFile(target).then(
    (buf) => {
      /* 序号 +1：只碰 `catalog.json` 这一份响应的两格，其余字节原样 */
      if (BUMP && target.endsWith(`${sep}catalog.json`)) {
        try {
          const catalog = JSON.parse(buf.toString('utf8'))
          catalog.publishSerial = Number(catalog.publishSerial ?? 1) + 1
          catalog.publishedAt = new Date().toISOString()
          buf = Buffer.from(JSON.stringify(catalog, null, 2), 'utf8')
        } catch {
          /* 读不出 JSON 就照原样端出去 —— 客户端那边会如实说"目录读不出来"，
             比在这里编一份坏目录强 */
        }
      }
      res
        .writeHead(200, {
          'content-type': TYPES[extname(target)] ?? 'application/octet-stream',
          'content-length': buf.length,
          /* 测试源要能"改了立刻看见"：别让任何一层缓存住旧字节 */
          'cache-control': 'no-store',
        })
        .end(buf)
    },
    () => {
      res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' }).end(`没有这个资源：${rel}`)
    },
  )
})

server.listen(PORT, HOST, () => {
  console.log(`[preset-test-server] 根：${ROOT}`)
  console.log(`[preset-test-server] 地址：http://${HOST}:${PORT}`)
  console.log('[preset-test-server] 填进「设置 → 高级设置 → 预设数据源 → 自定义地址」，')
  console.log('[preset-test-server] 或 `npm run dev:test-update` 自动注入（debug 构建才认）')
})
