#!/usr/bin/env node
/**
 * 资源寻址立法扫描（与旧 CI 里那段 grep 同口径）：
 * ① `join_url(` 只许住在 `runtime/resolver`；
 * ② 交付面 `presets/delivery/source.json` 禁止绝对 URL。
 */

import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative, resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const bad = []

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    const st = statSync(p)
    if (st.isDirectory()) walk(p)
    else if (p.endsWith('.rs')) scan(p)
  }
}

function scan(file) {
  const rel = relative(ROOT, file).replaceAll('\\', '/')
  if (rel.includes('runtime/resolver')) return
  readFileSync(file, 'utf8')
    .split('\n')
    .forEach((line, i) => {
      if (/join_url\s*\(/.test(line) && !/^\s*(\/\/|\/\/\/)/.test(line)) {
        bad.push(`${rel}:${i + 1} 路径拼接跑出了 runtime/resolver`)
      }
    })
}

walk(join(ROOT, 'src-tauri/src'))

const source = join(ROOT, 'presets/delivery/source.json')
try {
  if (/https?:\/\//.test(readFileSync(source, 'utf8'))) {
    bad.push('presets/delivery/source.json 含绝对 URL')
  }
} catch {
  /* 文件不存在由其它闸管；这里不重复报 */
}

if (bad.length) {
  for (const line of bad) console.error(`✗ ${line}`)
  process.exit(1)
}
console.log('✓ 寻址立法扫描通过')
