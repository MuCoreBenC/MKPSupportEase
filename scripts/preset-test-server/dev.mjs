#!/usr/bin/env node
/*
 * **一键起"本地官方源 + Tauri dev"**（2026-10-08）。
 *
 * ```bash
 * npm run dev:test-update            # 端仓库 presets + tauri dev（源指向它）
 * node dev.mjs --root <预设根>        # 想端别的地方
 * ```
 *
 * 两步，一步都不用手动：
 *
 *   ① 起 `server.mjs`（端**当前交付**，发布序号自动 +1），轮询 `/delivery/source.json`
 *      **等它真的就绪**；
 *   ② 以 `MKPSE_PRESET_SOURCE_URL` 启动 `tauri dev` —— 那个变量**只在 debug 构建里认**
 *      （见 `runtime/source.rs` 的 `debug_source_override`），生产构建一个字节都不受影响。
 *
 * 于是整条链是真的：客户端**不感知这是测试源**，它走 catalog / manifest / 寻址 /
 * SHA 校验 / 下载管道；换的只是"去哪儿取"。
 *
 * ★ 2026-10-30：**夹具退役**（作者：「我就不需要这些什么测试夹具一夹具二了」）。
 * 原来那种"从真交付根派生两代副本"的做法删了 —— 端真产物、序号在响应里 +1，
 * 一样能演"云端发了下一版"，而且不会和源漂移、也没有第二套目录。
 */

import { spawn } from 'node:child_process'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '..', '..')

const argv = process.argv.slice(2)
const argOf = (name) => {
  const at = argv.indexOf(name)
  return at >= 0 ? argv[at + 1] : undefined
}
/* 端哪一份：`--root` → 仓库 presets（与工作台按钮同一档默认） */
const ROOT = resolve(argOf('--root') ?? join(REPO, 'presets'))
const PORT = Number(process.env.PRESET_TEST_PORT ?? 8787)
const ORIGIN = `http://127.0.0.1:${PORT}`
/* 交付根在预设根底下的 `delivery/`：客户端填的是 `<地址>/delivery`（`filesRoot: ".."`） */
const DELIVERY_URL = `${ORIGIN}/delivery`

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

const server = spawn(
  process.execPath,
  [join(HERE, 'server.mjs'), '--root', ROOT, '--port', String(PORT), '--bump-serial'],
  { stdio: 'inherit' },
)

const stopServer = () => {
  if (!server.killed) server.kill()
}

if (!(await waitFor(`${DELIVERY_URL}/source.json`))) {
  stopServer()
  console.error(
    `[preset-test-server] 服务没起来（${DELIVERY_URL}/source.json 一直取不到）——` +
      `先在工作台点一次「生成」，或用 --root 指到有 delivery/ 的预设根`,
  )
  process.exit(1)
}
console.log(`[preset-test-server] 就绪：${DELIVERY_URL} —— 现在起 tauri dev，并把云端源指向它`)

const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx'
const dev = spawn(npx, ['tauri', 'dev'], {
  stdio: 'inherit',
  cwd: REPO,
  shell: process.platform === 'win32',
  env: { ...process.env, MKPSE_PRESET_SOURCE_URL: DELIVERY_URL },
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
