/*
 * 现状地图构建脚本。
 *
 * 输入：docs/audit-current-state/audit-data.json（索引）+ data/*.json（分主题数据）+
 *       map/app.ts（界面，纯 TS）+ map/app.css
 * 输出：docs/audit-current-state/index.html —— 独立、可离线打开的单文件应用；
 *       数据内联为 window.__AUDIT__（与 audit-data.json 同一份，不另写一套事实）。
 *
 * 做法（不引入任何新依赖）：
 *   1) 读索引与各分片，做结构校验（JSON 可解析 / id 唯一 / 证据串形状）；
 *   2) 用仓库自带的 typescript 把 app.ts 编成单文件 ES2020（无 import，直接可 <script> 执行）；
 *   3) 把 CSS、数据、JS 内联进 index.html；
 *   4) 打印统计与校验结果；有硬错误则非零退出。
 *
 * 重复构建：node docs/audit-current-state/map/build.mjs（幂等，覆盖 index.html）。
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const mapDir = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(mapDir, '..')            // docs/audit-current-state
const repoRoot = path.resolve(mapDir, '../../..')     // 仓库根
const outHtml = path.join(rootDir, 'index.html')

const errors = []
const warnings = []

function readJson(rel) {
  const p = path.join(rootDir, rel)
  if (!existsSync(p)) {
    errors.push(`缺少数据文件：${rel}`)
    return {}
  }
  try {
    return JSON.parse(readFileSync(p, 'utf8'))
  } catch (e) {
    errors.push(`JSON 解析失败：${rel} —— ${e.message}`)
    return {}
  }
}

const index = readJson('audit-data.json')
if (!index.meta) errors.push('audit-data.json 缺 meta')

const parts = {}
for (const rel of index.parts ?? []) {
  parts[rel] = readJson(rel)
}

const moduleMap = parts['data/module-map.json'] ?? {}
const product = parts['data/product.json'] ?? {}
const domain = parts['data/domain.json'] ?? {}
const contract = parts['data/contract.json'] ?? {}
const persistence = parts['data/persistence.json'] ?? {}
const cloud = parts['data/cloud.json'] ?? {}
const rewrite = parts['data/rewrite.json'] ?? {}
const coverage = parts['data/coverage.json'] ?? {}

/** 与 map/app.ts 里的 AuditData 形状一一对应 */
const data = {
  schema: index.schema,
  meta: index.meta,
  repositoryPathNote: index.repositoryPathNote,
  workspace: moduleMap.workspace ?? {},
  roots: moduleMap.roots ?? {},
  modules: moduleMap.modules ?? [],
  notCoveredInModuleMap: moduleMap.notCoveredInModuleMap ?? [],
  pages: product.pages ?? [],
  actions: product.actions ?? [],
  product: { summary: product.summary ?? {} },
  entities: domain.entities ?? [],
  stateMachines: domain.stateMachines ?? [],
  lifecycle: domain.lifecycle ?? [],
  readWriteMatrix: domain.readWriteMatrix ?? [],
  conflicts: domain.conflicts ?? [],
  domain: { rootsNote: domain.rootsNote ?? {}, pathRowTemplate: domain.pathRowTemplate ?? {} },
  contract: {
    summary: contract.summary ?? {},
    commands: contract.commands ?? [],
    dtos: contract.dtos ?? [],
    boundaries: contract.boundaries ?? {},
  },
  persistence: {
    atomicWrite: persistence.atomicWrite ?? {},
    writes: persistence.writes ?? [],
    writeDisciplineGaps: persistence.writeDisciplineGaps ?? [],
    carriers: persistence.carriers ?? [],
    localStorage: persistence.localStorage ?? [],
    sessionState: persistence.sessionState ?? {},
    startupInit: persistence.startupInit ?? {},
    consistency: persistence.consistency ?? [],
    halfDoneRisks: persistence.halfDoneRisks ?? [],
  },
  cloud: {
    source: cloud.source ?? {},
    catalog: cloud.catalog ?? {},
    downloadChain: cloud.downloadChain ?? [],
    publishChain: cloud.publishChain ?? [],
    rules: cloud.rules ?? [],
    devTestSurface: cloud.devTestSurface ?? [],
  },
  problems: rewrite.problems ?? [],
  complexity: rewrite.complexity ?? [],
  decisions: rewrite.decisions ?? [],
  targetModel: rewrite.targetModel ?? {},
  acceptance: rewrite.acceptance ?? [],
  roadmap: rewrite.roadmap ?? [],
  unverified: rewrite.unverified ?? [],
  coverage: {
    covered: coverage.covered ?? [],
    notCovered: coverage.notCovered ?? [],
    gaps: coverage.gaps ?? [],
    methodLimitations: coverage.methodLimitations ?? [],
  },
}

/* ------------------------------------------------------------ 结构校验 */

const idSets = {
  modules: data.modules,
  pages: data.pages,
  actions: data.actions,
  entities: data.entities,
  stateMachines: data.stateMachines,
  commands: data.contract.commands,
  dtos: data.contract.dtos,
  writes: data.persistence.writes,
  carriers: data.persistence.carriers,
  conflicts: data.conflicts,
  problems: data.problems,
  complexity: data.complexity,
  decisions: data.decisions,
  acceptance: data.acceptance,
  roadmap: data.roadmap,
  unverified: data.unverified,
}

let evidenceTotal = 0
let evidenceOdd = 0
let fieldTotal = 0

function checkEvidence(list) {
  if (!Array.isArray(list)) return
  for (const e of list) {
    evidenceTotal++
    if (typeof e !== 'string' || !e.includes('::')) evidenceOdd++
  }
}

for (const [coll, rows] of Object.entries(idSets)) {
  const seen = new Set()
  for (const r of rows) {
    const id = r.id ?? r.key ?? r.step ?? r.flow ?? r.rule ?? r.data
    if (id === undefined) {
      warnings.push(`${coll}: 有一条缺 id（将用占位标识）`)
      continue
    }
    if (seen.has(id)) errors.push(`${coll}: id 重复 ${id}`)
    seen.add(id)
    checkEvidence(r.evidence)
  }
}

for (const r of data.lifecycle) checkEvidence(r.evidence)
for (const r of data.cloud.downloadChain) checkEvidence(r.evidence)
for (const r of data.cloud.publishChain) checkEvidence(r.evidence)
for (const r of data.cloud.rules) checkEvidence(r.evidence)
for (const r of data.persistence.consistency) checkEvidence(r.evidence)
for (const r of data.persistence.writeDisciplineGaps) checkEvidence(r.evidence)

for (const d of data.contract.dtos) {
  if (Array.isArray(d.fields)) {
    fieldTotal += d.fields.length
    for (const f of d.fields) {
      if (!f || typeof f.name !== 'string') errors.push(`DTO ${d.id}: 字段缺 name`)
    }
  } else {
    warnings.push(`DTO ${d.id}: 没有 fields`)
  }
}

// 交叉引用：只在「引用就是纯编号」时才校验（正文里提到的编号不作为判据）
const problemIds = new Set(data.problems.map((p) => p.id))
const complexityIds = new Set(data.complexity.map((p) => p.id))
const refRe = /^(X|CX|RD|E|SM|C|D|PS|W|A|P|M|CF|CO|WD|DT|U|G|R|AC|RM)-\d+$/
function checkRefs(owner, list) {
  if (!Array.isArray(list)) return
  for (const ref of list) {
    const s = String(ref)
    if (!refRe.test(s)) continue
    const ok = problemIds.has(s) || complexityIds.has(s) ||
      data.entities.some((x) => x.id === s) || data.contract.commands.some((x) => x.id === s) ||
      data.contract.dtos.some((x) => x.id === s) || data.persistence.carriers.some((x) => x.id === s) ||
      data.persistence.writes.some((x) => x.id === s) || data.pages.some((x) => x.id === s) ||
      data.actions.some((x) => x.id === s) || data.conflicts.some((x) => x.id === s) ||
      data.modules.some((x) => x.id === s) || data.unverified.some((x) => x.id === s)
    if (!ok) warnings.push(`${owner} 引用了不存在的编号 ${s}`)
  }
}
for (const a of data.actions) checkRefs(`动作 ${a.id}`, a.problems)
for (const d of data.decisions) checkRefs(`决策 ${d.id}`, d.decisions)

/* ---------------------------------------------- 证据可核对性（文件必须真在） */

/** 证据串形如 `路径::符号::行号`；也可能是一个目录/通配/逗号分隔的多项 */
function evidencePaths(ev) {
  const head = String(ev).split('::')[0].trim()
  return head.split(',').map((s) => s.trim()).filter(Boolean)
}

let evidenceMissing = []
function checkEvidenceFiles(list) {
  if (!Array.isArray(list)) return
  for (const ev of list) {
    for (const p of evidencePaths(ev)) {
      const clean = p.replace(/^\.\//, '')
      if (clean.includes('*')) {
        // 通配：只要求通配之前的目录存在
        const dir = clean.slice(0, clean.indexOf('*')).replace(/\/$/, '')
        if (dir && !existsSync(path.join(repoRoot, dir))) evidenceMissing.push(`${ev} → 目录不存在 ${dir}`)
        continue
      }
      const abs = path.join(repoRoot, clean)
      if (!existsSync(abs)) evidenceMissing.push(`${ev} → 文件不存在 ${clean}`)
    }
  }
}
const evidenceSources = [
  data.modules, data.pages, data.actions, data.entities, data.stateMachines, data.lifecycle,
  data.conflicts, data.contract.commands, data.contract.dtos,
  data.persistence.writes, data.persistence.writeDisciplineGaps, data.persistence.carriers,
  data.persistence.localStorage, data.persistence.consistency, data.persistence.halfDoneRisks,
  data.cloud.rules, data.cloud.downloadChain, data.cloud.publishChain, data.cloud.devTestSurface,
  data.problems, data.complexity, data.decisions, data.unverified,
]
for (const rows of evidenceSources) for (const r of rows) checkEvidenceFiles(r.evidence)
for (const r of data.persistence.consistency) checkEvidenceFiles(r.evidence)

/* ------------------------------------------- 正文与数据的一致性（机械核对） */

const EXPECTED_DOCS = [
  'README.md', '01-product-spec.md', '02-domain-state.md', '03-contract.md',
  '04-persistence.md', '05-cloud-publish.md', '06-complexity-findings.md',
  'rewrite-decisions.md', 'rewrite-roadmap.md',
]
for (const f of EXPECTED_DOCS) {
  if (!existsSync(path.join(rootDir, f))) errors.push(`缺少交付物正文：${f}`)
}

const counts = {
  modules: data.modules.length,
  pages: data.pages.length,
  actions: data.actions.length,
  entities: data.entities.length,
  stateMachines: data.stateMachines.length,
  lifecycle: data.lifecycle.length,
  commands: data.contract.commands.length,
  dtos: data.contract.dtos.length,
  dtoFields: fieldTotal,
  problems: data.problems.length,
  complexity: data.complexity.length,
  decisions: data.decisions.length,
  acceptance: data.acceptance.length,
  roadmap: data.roadmap.length,
  unverified: data.unverified.length,
  carriers: data.persistence.carriers.length,
  writes: data.persistence.writes.length,
}

/* README 里那行一致性数字必须与数据一致（避免正文与数据漂移） */
const readmePath = path.join(rootDir, 'README.md')
let readmeOk = false
if (existsSync(readmePath)) {
  const md = readFileSync(readmePath, 'utf8')
  const expect =
    `模块 ${counts.modules} / 页面 ${counts.pages} / 动作 ${counts.actions} / 实体 ${counts.entities} / ` +
    `状态机 ${counts.stateMachines} / 生命周期 ${counts.lifecycle} / 命令 ${counts.commands} / ` +
    `DTO ${counts.dtos} 结构 ${counts.dtoFields} 字段 / 载体 ${counts.carriers} / 写盘 ${counts.writes} / ` +
    `问题 ${counts.problems} / 复杂性 ${counts.complexity} / 裁断 ${counts.decisions} / ` +
    `验收 ${counts.acceptance} / 路线图 ${counts.roadmap} / 待验证 ${counts.unverified}`
  readmeOk = md.includes(expect)
  if (!readmeOk) errors.push(`README 的一致性数字与数据不一致。期望正文里出现：${expect}`)
}

/* 证据可核对性：引用的源文件/目录必须真实存在 */
const missingEvidence = [...new Set(evidenceMissing)]
if (missingEvidence.length) {
  errors.push(`证据指向的文件/目录有 ${missingEvidence.length} 处找不到（见下方清单）`)
  for (const m of missingEvidence.slice(0, 40)) console.error('  - ' + m)
}

/* ----------------------------------------------------------- 编译 app.ts */

const tscCandidates = [
  path.join(repoRoot, 'node_modules', 'typescript', 'bin', 'tsc'),
]
let tscBin = tscCandidates.find((p) => existsSync(p))
let tscRes
if (tscBin) {
  tscRes = spawnSync(process.execPath, [tscBin, '-p', path.join(mapDir, 'tsconfig.json')], {
    stdio: 'inherit',
  })
} else {
  warnings.push('仓库 node_modules 里找不到 typescript，回退到 npx tsc')
  tscRes = spawnSync('npx', ['tsc', '-p', path.join(mapDir, 'tsconfig.json')], {
    stdio: 'inherit',
    shell: true,
  })
}
if (tscRes.status !== 0) {
  errors.push(`tsc 编译失败（退出码 ${tscRes.status}）——不写出 index.html，避免用旧页面冒充新成果`)
}

const jsPath = path.join(mapDir, 'out', 'app.js')
const cssPath = path.join(mapDir, 'app.css')

let appJs = ''
let appCss = ''
if (errors.length === 0) {
  if (!existsSync(jsPath)) errors.push(`编译产物不存在：${path.relative(repoRoot, jsPath)}`)
  else appJs = readFileSync(jsPath, 'utf8')
  if (!existsSync(cssPath)) errors.push(`样式文件不存在：${path.relative(repoRoot, cssPath)}`)
  else appCss = readFileSync(cssPath, 'utf8')
}

/* ------------------------------------------------------------- 写 HTML */

function esc(s) {
  // 内联 JSON 时避免把 </script> 提前闭合
  return String(s).replace(/<\/script>/gi, '<\\/script>')
}

const html = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${data.meta?.title ?? 'SupportEase 现状地图'}</title>
<style>
${appCss}
</style>
</head>
<body>
<header class="top">
  <div class="topRow">
    <div>
      <h1>${data.meta?.title ?? 'SupportEase 现状地图'}</h1>
      <div class="muted" style="font-size:12px">只读审计 · 数据与 audit-data.json 同一份 · 源码引用为「路径::符号::行号」（仓库无可确定的公开浏览地址，故不生成链接）</div>
    </div>
    <div id="baseline"></div>
  </div>
  <div id="stats"></div>
  <div class="toolbar">
    <input id="q" type="search" placeholder="全局搜索：页面 / 动作 / 实体 / 字段 / 命令 / 路径 / 问题 / 裁断…" />
    <select id="lv">
      <option value="">全部证据等级</option>
      <option value="confirmed">已证实</option>
      <option value="partial">部分证实</option>
      <option value="unverified">待验证</option>
      <option value="conflict">已发现冲突</option>
    </select>
    <select id="vd">
      <option value="">全部裁断</option>
      <option value="keep">保留</option>
      <option value="merge">合并</option>
      <option value="drop">砍掉</option>
      <option value="internalize">降级为内部</option>
      <option value="redo">重做</option>
      <option value="todo">待验证</option>
    </select>
    <button id="clear">清除</button>
    <span id="count"></span>
  </div>
</header>

<div class="layout">
  <div id="navHost"></div>
  <main id="main"></main>
</div>

<footer class="foot">
  ${data.meta?.cliNote ?? ''}
</footer>

<script>window.__AUDIT__ = ${esc(JSON.stringify(data))};</script>
<script>
${appJs}
</script>
</body>
</html>
`

if (errors.length === 0) {
  mkdirSync(path.dirname(outHtml), { recursive: true })
  writeFileSync(outHtml, html, 'utf8')
}

/* --------------------------------------------------------------- 汇报 */

if (readmeOk) console.log('[audit-map] README 一致性数字与数据一致 ✅')
if (missingEvidence.length === 0) console.log('[audit-map] 证据可核对：所有引用的源文件/目录都真实存在 ✅')

console.log('[audit-map] 统计：', JSON.stringify(counts, null, 0))
console.log(`[audit-map] 证据引用 ${evidenceTotal} 处（形状异常 ${evidenceOdd} 处；异常项多为「非 路径::符号」的说明性引用）`)
if (warnings.length) {
  console.log(`[audit-map] 警告 ${warnings.length} 条：`)
  for (const w of warnings.slice(0, 40)) console.log('  - ' + w)
}
if (errors.length) {
  console.error(`[audit-map] 错误 ${errors.length} 条（未写出 index.html）：`)
  for (const e of errors.slice(0, 40)) console.error('  - ' + e)
  process.exit(1)
}
console.log(`[audit-map] 已写出 ${path.relative(repoRoot, outHtml).replace(/\\\\/g, '/')}（${(html.length / 1024).toFixed(1)} KB，单文件、无外部依赖）`)
