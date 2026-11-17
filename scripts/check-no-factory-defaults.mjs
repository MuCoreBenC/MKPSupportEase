#!/usr/bin/env node
/*
 * **客户端不许消费出厂值**（AGENTS.md §1.6）。
 *
 * 注册表 / 目录里的 `default` / `baseValue` 是工作台的共用口径 —— 里面可能是
 * 只为对齐结构而写的占位零。客户端拿它当「恢复默认值」的目标，会把用户的
 * X / Y 偏移改成 0。恢复只认 `baseline-local/`（最近一次与云端某一版逐字节
 * 一致落盘的原样）。
 *
 * 扫 `src/`，命中即红；扫描清单与 AGENTS.md 那一条同源。
 */
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const SRC = join(ROOT, 'src')
const BAD = ['factoryByKey', 'factoryValue', 'baseValue', 'params.row.dt.package']

const hits = []

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) {
      walk(full)
      continue
    }
    if (!/\.(ts|tsx|css)$/.test(name)) continue
    const lines = readFileSync(full, 'utf8').split('\n')
    lines.forEach((line, i) => {
      for (const pat of BAD) {
        if (line.includes(pat)) hits.push(`${relative(ROOT, full)}:${i + 1}  ${pat}`)
      }
    })
  }
}

walk(SRC)

if (hits.length > 0) {
  console.error('✗ 客户端在消费出厂值（工作台内部口径，含占位零）。')
  console.error('  「恢复默认值」只认 baseline-local：readPresetBaselineValues / readLocalOriginText。')
  for (const hit of hits) console.error(`  · ${hit}`)
  process.exit(1)
}

console.log('✓ 客户端没有消费出厂值')
