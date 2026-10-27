#!/usr/bin/env node
/*
 * 一键重做应用图标（macOS + Windows 两套）。
 *
 * # 为什么这么绕
 *
 * Tauri 的 `tauri icon <png>` 是**一套图出两个平台**的 —— 它拿同一张母版同时生成
 * macOS 的 `icon.icns`/`icon.png`/`128x128*.png` 与 Windows 的
 * `icon.ico`/`Square*.png`/`StoreLogo.png`。可这两个平台要的**不是同一张图**
 * （见 `gen-icons.py` 里 `WIN_ART_SCALE` 那段）：
 *   · macOS 会被 AppKit 裁成 squircle ⇒ 图形要留白多一点（60%）；
 *   · Windows 方角满幅 ⇒ 图形要顶满一点（73%），否则"小一圈"。
 *
 * ★★★ 第一版脚本以为"两次跑各写各的文件、互不覆盖"——**错了**：
 * `tauri icon` **每次都写全量**（两个平台的产物都重写）。所以顺序无关紧要，
 * **后跑的那次会把先跑的整个盖掉**。
 * ⇒ 正解是「各跑一次，然后从各自的产物里**按文件名挑出属于自己那套的**回填」。
 *
 * 跑完的账目（谁归谁）：
 *   macOS  ：icon.icns · icon.png · 16x16.png · 32x32.png · 64x64.png
 *            128x128.png · 128x128@2x.png · icon.icns
 *   Windows：icon.ico · Square*.png · StoreLogo.png
 *   ★ `32x32.png` / `64x64.png` 两边都可能瞅 —— 在 macOS 上它们主要给 Dock/菜单栏，
 *     本项目的 `tauri.conf.json` 只声明了 32 / 128 / 128@2x / icns / ico，
 *     所以这里把 32 / 64 也归 macOS（Windows 走 ico 里内嵌的那几档）。
 *
 * 需要 `cairosvg`（`gen-icons.py` 会提示怎么装）。
 */

import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)))
const ICONS = join(ROOT, 'src-tauri', 'icons')
const TMP = join(ROOT, 'src-tauri', 'icons', '.tmp-gen')

/** macOS 那套的文件名 */
const MAC_FILES = [
  'icon.icns',
  'icon.png',
  '16x16.png',
  '32x32.png',
  '64x64.png',
  '128x128.png',
  '128x128@2x.png',
]

/** Windows 那套的文件名（Square 系列用前缀匹配） */
const WIN_FILES = ['icon.ico', 'StoreLogo.png']
const WIN_PREFIX = 'Square'

function python() {
  const venv = '/tmp/iconenv/bin/python'
  return existsSync(venv) ? venv : 'python3'
}

function run(cmd, args) {
  console.log(`\n$ ${cmd} ${args.join(' ')}`)
  execFileSync(cmd, args, { cwd: ROOT, stdio: 'inherit' })
}

/** 把当前 icons/ 里符合 `pick` 的文件搬到 `dest` */
function harvest(dest, pick) {
  mkdirSync(dest, { recursive: true })
  const all = execFileSync('ls', [ICONS], { encoding: 'utf8' }).split('\n').filter(Boolean)
  const got = all.filter((f) => pick(f) && !f.startsWith('.'))
  for (const f of got) copyFileSync(join(ICONS, f), join(dest, f))
  console.log(`  收下 ${got.length} 个：${got.join(', ')}`)
  return got
}

// 0) 清干净（含上一轮留下的临时目录）
rmSync(TMP, { recursive: true, force: true })

// 1) 出两个母版
run(python(), ['scripts/gen-icons.py'])

// 2) 跑 macOS 母版 → 收下 macOS 那套
run('npx', ['tauri', 'icon', join(ICONS, 'source', 'icon-1024.png')])
console.log('[1/2] macOS 产物：')
const macGot = harvest(join(TMP, 'mac'), (f) => MAC_FILES.includes(f))

// 3) 跑 Windows 母版 → 收下 Windows 那套（这次会盖掉上一步的全量产物，但我们有备份）
run('npx', ['tauri', 'icon', join(ICONS, 'source', 'icon-1024-win.png')])
console.log('[2/2] Windows 产物：')
const winGot = harvest(
  join(TMP, 'win'),
  (f) => WIN_FILES.includes(f) || f.startsWith(WIN_PREFIX),
)

// 4) 回填：两套各回各的
for (const f of macGot) copyFileSync(join(TMP, 'mac', f), join(ICONS, f))
for (const f of winGot) copyFileSync(join(TMP, 'win', f), join(ICONS, f))
console.log(`\n回填完成：macOS ${macGot.length} 个 · Windows ${winGot.length} 个`)

// 5) 清临时目录与不做的平台
rmSync(TMP, { recursive: true, force: true })
for (const d of ['android', 'ios']) {
  const p = join(ICONS, d)
  if (existsSync(p)) {
    rmSync(p, { recursive: true, force: true })
    console.log(`已删除无关平台目录：${d}/`)
  }
}

console.log('\n图标已生成：macOS = squircle（图形 60%）· Windows = 方角满幅（图形 73%）')
