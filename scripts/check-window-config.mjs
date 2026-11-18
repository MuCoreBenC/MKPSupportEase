#!/usr/bin/env node
/**
 * 平台窗口配置的「抄全」闸（2026-11-18）。
 *
 * # 防的是哪一种坏
 *
 * `tauri.windows.conf.json` 不是"补几条"就完事：CLI 用 `json_patch::merge`（RFC 7386）
 * 把它合进基础配置，而 **JSON 数组是整体替换、不逐项合并**。所以那个文件里的
 * `app.windows[0]` 只要有一个字段没抄，Windows 构建里这个字段就是**默认值** ——
 * 不报错、不警告、macOS 上还全对。
 *
 * 真实现场：它只写了 `decorations` 与 `additionalBrowserArgs`，于是 `minWidth` /
 * `minHeight` / `width` / `height` 在 Windows 上全丢了。表现就是作者看到的
 * 「macOS 有大小限制、Windows 拖到多小都行」。
 *
 * # 判据
 *
 * 基础配置里**每一扇窗**的每个字段（macOS 专属的那两个除外），Windows 那份必须
 * 原值抄一遍。Windows 专属字段（`decorations` / `additionalBrowserArgs`）额外允许。
 *
 * ★ 以后往 `tauri.conf.json` 的窗口里加字段，这里不用改 —— 它会直接要求 Windows
 *   那份跟着抄。真正只属于 macOS 的字段，才往 `MAC_ONLY` 里加。
 */

import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

/* ★ Windows 上只有 `fileURLToPath` 给的是 `G:\…`（见 `check-copy.mjs` 的同一段） */
const ROOT = fileURLToPath(new URL('..', import.meta.url))

/** 只对 macOS 有意义、Windows 上不该出现的字段（平台配置里出现反而会被读成"想设它"） */
const MAC_ONLY = new Set(['titleBarStyle', 'hiddenTitle'])

const read = (name) =>
  JSON.parse(readFileSync(join(ROOT, 'src-tauri', name), 'utf8')).app.windows

const base = read('tauri.conf.json')
const win = read('tauri.windows.conf.json')
const problems = []

if (base.length !== win.length) {
  problems.push(`窗口数不一样：tauri.conf.json ${base.length} 扇 / tauri.windows.conf.json ${win.length} 扇`)
}

base.forEach((b, i) => {
  const w = win[i]
  if (w === undefined) return /* 数量不等上面已经报了 */
  for (const [key, value] of Object.entries(b)) {
    if (MAC_ONLY.has(key)) continue
    if (!(key in w)) {
      problems.push(`第 ${i + 1} 扇窗缺 "${key}"（Windows 上会退回默认值）`)
      continue
    }
    if (JSON.stringify(w[key]) !== JSON.stringify(value)) {
      problems.push(
        `第 ${i + 1} 扇窗的 "${key}" 两边不一致：base ${JSON.stringify(value)} / windows ${JSON.stringify(w[key])}`,
      )
    }
  }
})

if (problems.length > 0) {
  console.error('\n✗ 平台窗口配置没抄全 —— Windows 上这些字段会变成默认值：\n')
  for (const p of problems) console.error(`  · ${p}`)
  console.error('\n  修法：把缺的字段按 `tauri.conf.json` 的值补进 `tauri.windows.conf.json`。')
  console.error('  别指望"只写不一样的那几条"—— 数组是整体替换（RFC 7386），不是逐项合并。\n')
  process.exit(1)
}

console.log(`✓ 平台窗口配置抄全了：${base.length} 扇窗，字段与基础配置一致`)
