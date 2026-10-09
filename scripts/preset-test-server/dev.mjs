#!/usr/bin/env node
/*
 * **一键起"本地官方源 + Tauri dev"**（2026-10-08）。
 *
 * ```bash
 * npm run dev:test-update            # v1 夹具 + 1 在 8787 + tauri dev（源指向它）
 * node dev.mjs fixtures/v2           # 想直接看"新版"就指 v2
 * ```
 *
 * 三步，一步都不用手动：
 *
 *   ① 夹具不在就**先派生**（`make-fixtures.mjs`，从真交付根算 —— 不做第二份会过期的手抄件）；
 *   ② 起 `server.mjs`，轮询 `/source.json` **等它真的就绪**；
 *   ③ 以 `MKPSE_PRESET_SOURCE_URL` 启动 `tauri dev` —— 那个变量**只在 debug 构建里认**
 *      （见 `runtime/source.rs` 的 `debug_source_override`），生产构建一个字节都不受影响。
 *
 * 于是整条链是真的：客户端**不感知这是测试源**，它走 catalog / manifest / 寻址 /
 * SHA 校验 / 下载管道；换的只是"— 去哪儿取"。想换下一版：改夹具里的 TOML
 * 再 `npm run preset-source:sync`，然后重启这个命令（或直接点「检查更新」）。
 */

import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '..', '..')

const argv = process.argv.slice(2)
const ROOT = resolve(argv[0] ?? join(HERE, 'fixtures', 'v1'))
const PORT = Number(process.env.PRESET_TEST_PORT ?? 8787)
const ORIGIN = `http://127.0.0.1:${PORT}`

const runToEnd = (args) =>
  new Promise((done, fail) => {
    const p = spawn(process.execPath, args, { stdio: 'inherit', cwd: REPO })
    p.on('exit', (code) => (code === 0 ? done() : fail(new Error(`${args[0]} 退出码 ${code}`))))
  })

const waitFor = async (url, tries = 40) => {
  for (let i = 0; i < tries; i += 1) {
    const ok = await fetch(url).then(
      (r) => r.ok,
      () => false,
    )
    if (ok) return true
    await new Promise((r) => setTimeout(r, 150))
  }
  return false
}

if (!existsSync(join(ROOT, 'catalog.json'))) {
  console.log('[preset-test-server] 夹具不在，先派生一份……')
  await runToEnd([join(HERE, 'make-fixtures.mjs')])
}

const server = spawn(
  process.execPath,
  [join(HERE, 'server.mjs'), '--root', ROOT, '--port', String(PORT)],
  { stdio: 'inherit' },
)

const stopServer = () => {
  if (!server.killed) server.kill()
}

if (!(await waitFor(`${ORIGIN}/source.json`))) {
  stopServer()
  console.error(`[preset-test-server] 服务没起来（${ORIGIN}/source.json 一直取不到）`)
  process.exit(1)
}
console.log(`[preset-test-server] 就绪：${ORIGIN} —— 现在起 tauri dev，并把官方源指向它`)

const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx'
const dev = spawn(npx, ['tauri', 'dev'], {
  stdio: 'inherit',
  cwd: REPO,
  shell: process.platform === 'win32',
  env: { ...process.env, MKPSE_PRESET_SOURCE_URL: ORIGIN },
})

dev.on('exit', (code) => {
  stopServer()
  process.exit(code ?? 0)
})
for (const sig of ['SIGINT', 'SIGTERM']) {
  process.on(sig, () => {
    stopServer()
    dev.kill()
  })
}
