#!/usr/bin/env node
/*
 * 预设兼容矩阵：用**真实历史 tag** 的 `crates/preset` 读当前候选预设。
 *
 * 输出 `presets/delivery/compat-matrix.json`（发布闸读取）。
 *
 * 规则：
 *   - 客户端版本 >= minClientVersion：该客户端必须能解析（pass）；
 *   - 客户端版本 < minClientVersion：允许解析失败，但必须记成 `excluded`，
 *     不许把失败冒充 pass。
 *
 * 这些 tag 的源码只读；脚本用临时 worktree 编译探针，跑完清理。
 */

import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import {
  cpSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(fileURLToPath(new URL('..', import.meta.url)))
const PROBE_TEMPLATE = join(ROOT, 'tools', 'preset-compat-probe')
const CATALOG = join(ROOT, 'presets', 'delivery', 'catalog.json')
const PRESETS = join(ROOT, 'presets', 'delivery', 'mkp', 'presets')
const OUT = join(ROOT, 'presets', 'compat-matrix.json')
const COMPAT_CLIENTS = join(ROOT, 'presets', 'compat-clients.toml')
const KEEP = process.argv.includes('--keep')
/* 每趟独立目录：不删上一趟的 .scratch 现场（也避开环境里的批量删除保护）。 */
const WORK = join(ROOT, '.scratch', 'compat', `run-${Date.now()}`)

function run(cmd, args, opts = {}) {
  const res = spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    ...opts,
  })
  if (res.status !== 0) {
    throw new Error(
      `命令失败（${cmd} ${args.join(' ')}）：\n${res.stdout ?? ''}\n${res.stderr ?? ''}`,
    )
  }
  return res.stdout ?? ''
}

function parseClients() {
  const text = readFileSync(COMPAT_CLIENTS, 'utf8')
  const block = text.match(/clients\s*=\s*\[([\s\S]*?)\]/)
  if (!block) throw new Error(`${COMPAT_CLIENTS} 里没有 clients = [...]`)
  const tags = [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1])
  if (tags.length === 0) throw new Error('compat-clients.toml 里没有登记任何 tag')
  return tags
}

function appVersionOf(tag) {
  const text = run('git', ['show', `${tag}:src-tauri/Cargo.toml`])
  const m = text.match(/^version\s*=\s*"([^"]+)"/m)
  if (!m) throw new Error(`${tag} 里读不到 src-tauri/Cargo.toml 的 version`)
  return m[1]
}

function currentVersion() {
  const text = readFileSync(join(ROOT, 'src-tauri', 'Cargo.toml'), 'utf8')
  const m = text.match(/^version\s*=\s*"([^"]+)"/m)
  if (!m) throw new Error('读不到 src-tauri/Cargo.toml 的当前 version')
  return m[1]
}

function numsOf(version) {
  const core = version.trim().replace(/^v/, '').split('-')[0]
  const parts = core.split('.').map((p) => Number.parseInt(p, 10))
  if (parts.some((n) => !Number.isFinite(n))) throw new Error(`版本号认不出：${version}`)
  while (parts.length < 3) parts.push(0)
  return parts.slice(0, 3)
}

function versionAtLeast(actual, required) {
  const a = numsOf(actual)
  const b = numsOf(required)
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] > b[i]
  }
  return true
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

function parseProbeOutput(stdout) {
  const lines = stdout.trim().split(/\r?\n/).filter(Boolean)
  const last = lines.at(-1)
  if (!last) throw new Error('探针没有输出 JSON')
  return JSON.parse(last)
}

function main() {
  const catalog = JSON.parse(readFileSync(CATALOG, 'utf8'))
  const minClient = catalog.minClientVersion
  if (typeof minClient !== 'string' || minClient.trim() === '') {
    throw new Error('当前 catalog 没有 minClientVersion —— 先补结构签名规则再跑矩阵')
  }

  const files = readdirSync(PRESETS)
    .filter((name) => name.toLowerCase().endsWith('.toml'))
    .sort()
  if (files.length === 0) throw new Error('交付预设目录为空')

  const tags = parseClients()
  mkdirSync(WORK, { recursive: true })

  const clients = []
  try {
    for (const tag of tags) {
      const version = appVersionOf(tag)
      const worktree = join(WORK, `src-${tag}`)
      run('git', ['worktree', 'prune'])
      run('git', ['worktree', 'add', '--detach', worktree, tag])

      const probe = join(WORK, `probe-${tag}`)
      cpSync(PROBE_TEMPLATE, probe, { recursive: true })
      const cargoPath = join(probe, 'Cargo.toml')
      const presetPath = join(worktree, 'crates', 'preset').replaceAll('\\', '/')
      writeFileSync(
        cargoPath,
        readFileSync(cargoPath, 'utf8').replaceAll('__PRESET_PATH__', presetPath),
      )

      const targets = files.map((name) => join(PRESETS, name))
      const stdout = run('cargo', [
        'run',
        '--quiet',
        '--manifest-path',
        join(probe, 'Cargo.toml'),
        '--',
        ...targets,
      ])
      const results = parseProbeOutput(stdout)
      clients.push({ tag, version, results })

      if (!KEEP) {
        run('git', ['worktree', 'remove', '--force', worktree])
        /* 探针的 cargo target 由环境决定；这里不递归删（批量删除可能被环境拒绝），
           交给 .scratch 的定期清理。 */
      }
    }

    /*
     * 当前构建也要作为基线：只测旧客户端不测新客户端，等于没证明
     * “新程序读得懂自己这一代”。
     */
    const currentProbe = join(WORK, 'probe-current')
    cpSync(PROBE_TEMPLATE, currentProbe, { recursive: true })
    const currentCargo = join(currentProbe, 'Cargo.toml')
    const currentPreset = join(ROOT, 'crates', 'preset').replaceAll('\\', '/')
    writeFileSync(
      currentCargo,
      readFileSync(currentCargo, 'utf8').replaceAll('__PRESET_PATH__', currentPreset),
    )
    const currentOut = run('cargo', [
      'run',
      '--quiet',
      '--manifest-path',
      currentCargo,
      '--',
      ...files.map((name) => join(PRESETS, name)),
    ])
    clients.push({
      tag: 'current',
      version: currentVersion(),
      results: parseProbeOutput(currentOut),
    })
  } finally {
    if (!KEEP) {
      try {
        rmSync(WORK, { recursive: true, force: true })
      } catch {
        console.error(`  临时目录保留在 ${WORK}（环境拒绝批量删除）`)
      }
    }
  }

  const outFiles = files.map((fileName) => {
    const bytes = readFileSync(join(PRESETS, fileName))
    const perClient = clients.map((client) => {
      const got = client.results.find((r) => r.file === fileName)
      const included = versionAtLeast(client.version, minClient)
      const parser = got?.result ?? 'missing'
      const result = included ? (parser === 'pass' ? 'pass' : 'fail') : parser === 'pass' ? 'pass' : 'excluded'
      return {
        tag: client.tag,
        version: client.version,
        result,
        parser,
        reason: got?.reason ?? '',
        detail: got?.detail ?? '',
      }
    })
    return {
      fileName,
      sha256: sha256(bytes),
      minClientVersion: minClient,
      clients: perClient,
    }
  })

  const violations = outFiles.flatMap((file) =>
    file.clients
      .filter((client) => client.result !== 'pass' && client.result !== 'excluded')
      .map((client) => `${file.fileName} @ ${client.tag}（${client.result}）`),
  )

  const report = {
    generatedAt: new Date().toISOString(),
    candidateRevision: catalog.revision,
    minClientVersion: minClient,
    clients: clients.map((c) => ({ tag: c.tag, version: c.version })),
    files: outFiles,
    ok: violations.length === 0,
    violations,
  }
  writeFileSync(OUT, `${JSON.stringify(report, null, 2)}\n`)

  console.log(
    `${report.ok ? '✓' : '✗'} 兼容矩阵：${files.length} 份预设 × ${clients.length} 个客户端；minClient=${minClient}`,
  )
  for (const file of outFiles) {
    const marks = file.clients.map((c) => `${c.tag}:${c.result}`).join(' ')
    console.log(`  ${file.fileName}  ${marks}`)
  }
  if (!report.ok) {
    console.error('\n阻断项：')
    for (const v of violations) console.error(`  · ${v}`)
    process.exit(1)
  }
}

main()
