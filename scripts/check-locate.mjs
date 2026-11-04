#!/usr/bin/env node
/**
 * 判据 —— 「全仓的滚动定位只有一套；每一颗『去设置』都带落点」（2026-11-04）。
 *
 * # 它防的是哪一种坏
 *
 * 这一摊以前有三份实现，各写各的时长与长相：
 *
 *   · 设置页深链：`glideTo` + React `flash` state + `.field[data-flash]` 绿**描边圈** + 1.4s
 *   · 预设表定位：`glideIntoView` + `data-flash` in/out 两段 + 1.78s
 *   · 工作台定位：`glideIntoView` + 落位补测 + 两条 `@keyframes` + 1.8s
 *
 * 作者 2026-11-04 连着点名三处：「消失得太快了」「能不能是绿色背景，而不是这种圈起来的
 * 呀」「以后我还有什么需求，只要是滚动定位，大家都用一样的」。还有一处**静悄悄**的坏：
 * 后处理弹窗那颗「去设置」调用时没带落点，只把页签切过去、没滚到开关（截图里就是这样）。
 *
 * 这类坏法编译期抓不到、跑起来看着"也能用"，所以按本仓老规矩用静态扫描钉住。
 *
 * # 六道判据
 *
 *   ① 底层 `glide.ts` 只有 `components/scroll/locate.ts` 能 import（业务层不许自己滚）
 *   ② 定位件只有一处定义；工作台那份 `c14/locate.ts` 只能是转发
 *   ③ 绿底 / 时序只有一处定义：三套 `data-flash` / keyframes / outline 圈全退休
 *   ④ 落点字面量只在 `settings/targets.ts`；两个「去设置」入口都必须引用常量
 *   ⑤ `scrollIntoView` 只许留在「保持选中项可见」那 5 个白名单文件里
 *   ⑥ 这道闸自己挂在 `npm run lint` 末尾（CI 已在跑 lint ⇒ 自动生效、无需人工）
 *
 * ★ 纯静态、零依赖、不启 dev server、不写盘。红的时候把「为什么」写清楚，
 *   **不许**为了让它过而放宽判据（AGENTS.md §5）。
 */

import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, sep } from 'node:path'

const SRC = 'src'
const LOCATE = 'src/components/scroll/locate.ts'
const TARGETS = 'src/app/settings/targets.ts'
const WB_LOCATE = 'src/workbench/c14/locate.ts'
const REPORT = 'src/app/report/PageReport.tsx'
const MODAL = 'src/app/postprocess/PostProcessModal.tsx'
const SETTINGS_CSS = 'src/app/settings/PageSettings.module.css'

/** 「保持选中项可见」不是「定位到某处」—— 这 5 处是正当例外（见各文件注释） */
const NEAREST_ONLY = new Set([
  'src/workbench/views/DeliveryFilesModal.tsx',
  'src/workbench/views/GroupPicker.tsx',
  'src/components/field/SelectField.tsx',
  'src/workbench/components/field/SelectField.tsx',
  'src/workbench/c14/field/SelectField.tsx',
])

function norm(file) {
  return file.split(sep).join('/')
}

function walk(dir, only) {
  const out = []
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) out.push(...walk(full, only))
    else if (only(full)) out.push(norm(full))
  }
  return out
}

const TS = walk(SRC, (f) => /\.(ts|tsx)$/.test(f))
const CSS = walk(SRC, (f) => /\.css$/.test(f))

function read(file) {
  try {
    return readFileSync(file, 'utf8')
  } catch {
    return null
  }
}

/**
 * 去掉注释再判：这批文件里有大段历史注释，逐字记着 `data-flash` / `glideIntoView`
 * 这些名字，不剥掉会把「解释旧坑」误判成「又在用旧实现」。
 * 只剥整行 `//` 与块注释（含 JSX 里那类块注释），不动字符串里的 `//`。
 */
function code(file) {
  const text = read(file)
  if (text === null) return null
  return text
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/.*$/gm, '')
}

function lineOf(text, hit) {
  return text.slice(0, hit).split('\n').length
}

const problems = []
const note = (file, text, at, why) => {
  problems.push(`${file}:${lineOf(text, at)} → ${why}`)
}

/* ---------- ① 底层滚动实现只许定位件调用 ---------- */
for (const file of TS) {
  const text = code(file)
  if (text === null) continue
  const re = /import\s+[^;]*?from\s+['"][^'"]*components\/scroll\/glide['"]/g
  for (const m of text.matchAll(re)) {
    if (file !== LOCATE) {
      note(file, text, m.index, '业务层不许直接 import 底层 glide.ts —— 滚 + 落位 + 绿底是一整套，走 components/scroll/locate.ts')
    }
  }
}
const locateText = code(LOCATE)
if (locateText === null) {
  problems.push(`${LOCATE} 不存在 —— 共用定位件没了，这道判据无从判断`)
} else {
  if (!/export function locateIntoView\b/.test(locateText)) {
    problems.push(`${LOCATE} 里没有 locateIntoView`)
  }
  if (!/export function locateAnchor\b/.test(locateText)) {
    problems.push(`${LOCATE} 里没有 locateAnchor`)
  }
  if (!/const HOLD_MS = 6000\b/.test(locateText)) {
    problems.push(`${LOCATE} 的停留时长不是 6000ms —— 作者裁定「固定停约 6 秒淡出」`)
  }
  if (!/getPropertyValue\('--accent-soft'\)/.test(locateText)) {
    problems.push(`${LOCATE} 的绿底没有走 --accent-soft（主题 token 是唯一出处）`)
  }
}

/* ---------- ② 定位件只有一处定义；工作台那份是转发 ---------- */
for (const file of TS) {
  const text = code(file)
  if (text === null || file === LOCATE) continue
  if (/export function (locateIntoView|locateAnchor)\b/.test(text)) {
    note(file, text, 0, '又定义了一份定位件 —— 全仓只允许 components/scroll/locate.ts 一处')
  }
}
const wb = code(WB_LOCATE)
if (wb === null || !/export \{ locateAnchor \} from '[^']*components\/scroll\/locate'/.test(wb)) {
  problems.push(`${WB_LOCATE} 不是共用件的薄转发 —— 工作台不许再有第二套定位`)
}

/* ---------- ③ 三套旧高亮全退休：绿底 / 时序只有一处 ---------- */
const STALE_CSS = [
  { re: /\[data-flash[^\]]*\]/, why: 'data-flash 规则还在 —— 绿底与时序归共用件，页面/表格不许再各写一份' },
  { re: /\.flashIt\b/, why: 'flashIt 还在 —— 工作台那两条 @keyframes 已退休' },
  { re: /\.rowFlash\b/, why: 'rowFlash 还在 —— 工作台那两条 @keyframes 已退休' },
  { re: /@keyframes\s+c14-flash-/, why: 'c14-flash-* 关键帧还在 —— 定位高亮统一走 WAAPI' },
]
for (const file of CSS) {
  const text = code(file)
  if (text === null) continue
  for (const { re, why } of STALE_CSS) {
    const m = re.exec(text)
    if (m !== null) note(file, text, m.index, why)
  }
}
/* 设置页那圈 outline 不许回来（作者：「不是这种圈起来的呀」） */
const settingsCss = code(SETTINGS_CSS)
if (settingsCss !== null && /outline-offset/.test(settingsCss)) {
  note(SETTINGS_CSS, settingsCss, settingsCss.indexOf('outline-offset'), '设置页又出现描边式高亮 —— 定位要的是整行绿色背景')
}
/* 旧的三段时序不许散回业务文件 */
for (const file of [REPORT, MODAL, 'src/app/presets/PagePresets.tsx', 'src/app/settings/PageSettings.tsx', WB_LOCATE]) {
  const text = code(file)
  if (text === null) continue
  const m = /\b(FLASH_MS|FLASH_IN_MS|FLASH_HOLD_MS|FLASH_OUT_MS)\b/.exec(text)
  if (m !== null) note(file, text, m.index, '又在业务层写定位高亮的时长 —— 停留/淡出只在共用件里定义')
}

/* ---------- ④ 落点字面量只在 targets.ts；两个「去设置」入口都引用常量 ---------- */
for (const file of TS) {
  const text = code(file)
  if (text === null || file === TARGETS) continue
  const m = /['"`]postproc-tower['"`]/.exec(text)
  if (m !== null) {
    note(file, text, m.index, '落点字面量不许散在业务文件里 —— 从 settings/targets.ts 引 SETTINGS_TARGET_TOWER')
  }
}
const targets = code(TARGETS)
if (targets === null || !/export const SETTINGS_TARGET_TOWER = 'postproc-tower'/.test(targets)) {
  problems.push(`${TARGETS} 里没有唯一的落点常量`)
}
for (const file of [REPORT, MODAL]) {
  const text = code(file)
  if (text === null) continue
  if (!/SETTINGS_TARGET_TOWER/.test(text)) {
    note(file, text, 0, '「去设置」没带落点常量 —— 会只切页签、不滚到开关（2026-11-04 截图里那个坏法）')
  }
  const bare = /onOpenSettings\(\s*\)/.exec(text)
  if (bare !== null) {
    note(file, text, bare.index, '「去设置」空参数 —— 漏带落点，设置页不会滚')
  }
}

/* ---------- ⑤ scrollIntoView 只许留在「保持选中项可见」白名单 ---------- */
for (const file of TS) {
  const text = code(file)
  if (text === null) continue
  const m = /\.scrollIntoView\(/.exec(text)
  if (m === null) continue
  if (!NEAREST_ONLY.has(file)) {
    note(file, text, m.index, '滚动定位不许用 scrollIntoView —— 走 locateAnchor / locateIntoView')
  } else if (!/block:\s*'nearest'/.test(text)) {
    note(file, text, m.index, '这处例外只该是「保持选中项可见」（block: nearest）')
  }
}

/* ---------- ⑥ 这道闸自己挂在 lint 上 ---------- */
const pkg = read('package.json')
if (pkg === null) {
  problems.push('读不到 package.json —— 无法确认这道闸接在 lint 上')
} else {
  let lint = ''
  try {
    lint = JSON.parse(pkg).scripts?.lint ?? ''
  } catch {
    problems.push('package.json 解析不了 —— 无法确认这道闸接在 lint 上')
  }
  if (!lint.includes('check-locate.mjs')) {
    problems.push('package.json 的 lint 没接 check-locate.mjs —— 这道闸会变成没人跑的死文件')
  }
}

/* ---------- 收场 ---------- */
if (problems.length > 0) {
  console.error(`✗ 滚动定位判据没过（${problems.length} 条）：`)
  for (const p of problems) console.error(`  · ${p}`)
  console.error('')
  console.error('  一套定位 = components/scroll/locate.ts（滚 / 落位 / 绿底 / 6 秒淡出）+ 一处落点常量。')
  console.error('  改对了再跑；**不要**把这条判据放宽来"让 CI 过"——那等于把闸门拆了。')
  process.exitCode = 1
} else {
  console.log('✓ 滚动定位：底层只由共用件调用、绿底/时长/落点各一处、去设置都带落点、例外白名单干净')
}
