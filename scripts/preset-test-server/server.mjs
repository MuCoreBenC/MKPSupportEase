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
 * npm run preset-source:dev            # 起 v1（默认 127.0.0.1:8787）
 * node server.mjs --root fixtures/v2 --port 8787
 * ```
 *
 * # 端哪一份：`--root` → `PRESET_TEST_ROOT` → 夹具 v1
 *
 * 第二档那个环境变量是给**工作台那颗按钮**用的（2026-10-08）：它起的永远是
 * `npm run preset-source:dev` 这同一条命令（`package.json` 里那行是契约），
 * 换源换的是环境变量 —— 于是"换源"不必给这条命令另开一条 argv 路径。
 * 参数仍然优先（手敲命令时照旧 `--root` 说了算）。
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
/* 端哪一份：`--root` 参数 → `PRESET_TEST_ROOT`（工作台换源走这一条）→ 夹具 v1 */
const ROOT = resolve(argOf('--root', process.env.PRESET_TEST_ROOT ?? join(HERE, 'fixtures', 'v1')))

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
