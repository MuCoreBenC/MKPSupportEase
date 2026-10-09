/*
 * SupportEase 现状地图 —— 单页离线应用（纯 TS，无框架、无外部依赖）。
 *
 * 数据来源：构建时由 build.mjs 把 docs/audit-current-state/audit-data.json 与其 parts
 * 内联成 window.__AUDIT__（与 audit-data.json 同一份数据，不另写一套）。
 *
 * 设计约束（来自任务要求）：
 *  - 独立 HTML，断网可开、可搜、可展开、可筛选；
 *  - 页面数据与 UI 分离（这里只读 window.__AUDIT__，不写死任何统计数字）；
 *  - 一切统计数字都由实际数据算出；
 *  - 源码引用只给「仓库相对路径::符号::行号」，不生成任何伪链接。
 */

/* 本文件是全局脚本（无 import/export），所以直接声明全局 Window 的形状 */
interface Window {
  __AUDIT__: AuditData
}

interface AnyRecord {
  id: string
  [k: string]: unknown
}

interface AuditData {
  schema: string
  meta: {
    title: string
    repo: string
    branch: string
    head: string
    headShort: string
    headDate: string
    headSubject: string
    auditDate: string
    workingTreeClean: boolean
    readOnly: boolean
    sourceOfTruth: string
    method: string[]
    evidenceLevels: Record<string, string>
    verdicts: Record<string, string>
    cliNote: string
  }
  repositoryPathNote: string
  workspace: Record<string, unknown>
  roots: Record<string, unknown>
  modules: AnyRecord[]
  notCoveredInModuleMap: string[]
  pages: AnyRecord[]
  actions: AnyRecord[]
  product: { summary: Record<string, unknown> }
  entities: AnyRecord[]
  stateMachines: AnyRecord[]
  lifecycle: AnyRecord[]
  readWriteMatrix: AnyRecord[]
  conflicts: AnyRecord[]
  domain: { rootsNote: Record<string, string>; pathRowTemplate: Record<string, string> }
  contract: {
    summary: Record<string, unknown>
    commands: AnyRecord[]
    dtos: AnyRecord[]
    boundaries: Record<string, unknown>
  }
  persistence: {
    atomicWrite: Record<string, unknown>
    writes: AnyRecord[]
    writeDisciplineGaps: AnyRecord[]
    carriers: AnyRecord[]
    localStorage: AnyRecord[]
    sessionState: Record<string, unknown>
    startupInit: Record<string, unknown>
    consistency: AnyRecord[]
    halfDoneRisks: AnyRecord[]
  }
  cloud: {
    source: Record<string, unknown>
    catalog: Record<string, unknown>
    downloadChain: AnyRecord[]
    publishChain: AnyRecord[]
    rules: AnyRecord[]
    devTestSurface: AnyRecord[]
  }
  problems: AnyRecord[]
  complexity: AnyRecord[]
  decisions: AnyRecord[]
  targetModel: Record<string, unknown>
  acceptance: AnyRecord[]
  roadmap: AnyRecord[]
  unverified: AnyRecord[]
  coverage: {
    covered: string[]
    notCovered: string[]
    gaps: AnyRecord[]
    methodLimitations: string[]
  }
}

const DATA: AuditData = window.__AUDIT__

/* ------------------------------------------------------------------ 工具 */

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  opts: { cls?: string; text?: string; title?: string; html?: string } = {},
): HTMLElementTagNameMap[K] {
  const n = document.createElement(tag)
  if (opts.cls) n.className = opts.cls
  if (opts.text !== undefined) n.textContent = opts.text
  if (opts.title !== undefined) n.title = opts.title
  if (opts.html !== undefined) n.innerHTML = opts.html
  return n
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/** 把一条记录压成一行可搜文本（搜索与筛选共用） */
function flatText(v: unknown, out: string[] = []): string[] {
  if (v === null || v === undefined) return out
  if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
    out.push(String(v))
    return out
  }
  if (Array.isArray(v)) {
    for (const x of v) flatText(x, out)
    return out
  }
  if (isPlainObject(v)) {
    for (const k of Object.keys(v)) {
      out.push(k)
      flatText((v as Record<string, unknown>)[k], out)
    }
  }
  return out
}

const LEVEL_LABEL: Record<string, string> = {
  confirmed: '已证实',
  partial: '部分证实',
  unverified: '待验证',
  conflict: '已发现冲突',
}

const VERDICT_LABEL: Record<string, string> = {
  keep: '保留',
  merge: '合并',
  drop: '砍掉',
  internalize: '降级为内部',
  redo: '重做',
  todo: '待验证',
}

/* --------------------------------------------------- 证据 / 字段渲染小件 */

function evidenceChips(list: unknown): HTMLElement {
  const box = el('div', { cls: 'ev' })
  box.append(el('span', { cls: 'evLabel', text: '证据' }))
  const arr = Array.isArray(list) ? list : list ? [list] : []
  if (arr.length === 0) {
    box.append(el('span', { cls: 'muted', text: '（无）' }))
    return box
  }
  for (const e of arr) {
    const s = String(e)
    // 只显示 路径::符号::行号；不做伪链接（仓库无可确定的公开浏览地址）
    box.append(el('code', { cls: 'evItem', text: s, title: s }))
  }
  return box
}

function levelBadge(level: unknown): HTMLElement | null {
  const key = typeof level === 'string' ? level : ''
  if (!key || !LEVEL_LABEL[key]) return null
  const b = el('span', { cls: `badge lv-${key}`, text: LEVEL_LABEL[key] })
  b.title = DATA.meta.evidenceLevels[key] ?? key
  return b
}

function verdictBadge(verdict: unknown): HTMLElement | null {
  const key = typeof verdict === 'string' ? verdict : ''
  if (!key || !VERDICT_LABEL[key]) return null
  const b = el('span', { cls: `badge vd-${key}`, text: VERDICT_LABEL[key] })
  b.title = DATA.meta.verdicts[key] ?? key
  return b
}

/* pairs 用宽松的 Array<Array<unknown>>：调用处直接写数组字面量时不会被元组类型卡住 */
function kvTable(pairs: Array<Array<unknown>>, opts: { skip?: string[] } = {}): HTMLElement {
  const skip = new Set(['id', 'evidence', 'level', 'verdict', ...(opts.skip ?? [])])
  const t = el('table', { cls: 'kv' })
  for (const pair of pairs) {
    const k = String(pair[0])
    const v = pair[1]
    if (skip.has(k)) continue
    if (v === undefined || v === null) continue
    const tr = el('tr')
    tr.append(el('th', { text: k }))
    tr.append(valueCell(v))
    t.append(tr)
  }
  return t
}

function valueCell(v: unknown): HTMLElement {
  const td = el('td')
  if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
    td.textContent = String(v)
    return td
  }
  if (Array.isArray(v)) {
    if (v.length === 0) {
      td.append(el('span', { cls: 'muted', text: '（空）' }))
      return td
    }
    // 字符串数组 → 一行 chips；对象数组 → 折叠列表
    if (v.every((x) => typeof x === 'string' || typeof x === 'number')) {
      for (const x of v) td.append(el('span', { cls: 'chip', text: String(x) }))
      return td
    }
    for (const x of v) {
      const d = el('details', { cls: 'inline' })
      const s = el('summary', { text: isPlainObject(x) && typeof x.id === 'string' ? String(x.id) : '一条' })
      d.append(s)
      if (isPlainObject(x)) d.append(kvTable(Object.entries(x), { skip: ['id'] }))
      else d.append(el('span', { text: String(x) }))
      td.append(d)
    }
    return td
  }
  if (isPlainObject(v)) {
    td.append(kvTable(Object.entries(v)))
    return td
  }
  td.textContent = '—'
  return td
}

/** 一条记录的通用头部：标题 + 徽章 + 副标题 */
function recordHead(r: AnyRecord, title: string, sub?: string): HTMLElement {
  const h = el('div', { cls: 'rhead' })
  const line = el('div', { cls: 'rtitle' })
  line.append(el('span', { cls: 'rid', text: idOf(r) }))
  line.append(el('strong', { text: title }))
  const lv = levelBadge(r.level)
  if (lv) line.append(lv)
  const vd = verdictBadge(r.verdict)
  if (vd) line.append(vd)
  h.append(line)
  if (sub) h.append(el('div', { cls: 'rsub', text: sub }))
  return h
}

/* --------------------------------------------------------------- 记录渲染 */

/** 有些记录没有 id（生命周期步骤、链路步骤、localStorage 键…）——统一给一个稳定标识 */
function idOf(r: AnyRecord): string {
  const cand = r.id ?? r.key ?? r.step ?? r.flow ?? r.case ?? r.name ?? r.rule ?? r.data
  return cand === undefined ? '—' : String(cand)
}

function renderRecord(kind: string, r: AnyRecord): HTMLElement {
  const rid = idOf(r)
  const box = el('article', { cls: `rec k-${kind}` })
  box.dataset.kind = kind
  box.dataset.id = rid
  const level = typeof r.level === 'string' ? r.level : ''
  const verdict = typeof r.verdict === 'string' ? r.verdict : ''
  box.dataset.level = level
  box.dataset.verdict = verdict
  box.dataset.search = flatText(r).join(' ').toLowerCase() + ' ' + kind + ' ' + rid

  let title = rid
  let sub: string | undefined
  switch (kind) {
    case 'page':
      title = `${String(r.name)}（页签 ${String(r.tab)}）`
      sub = String(r.purpose)
      break
    case 'action':
      title = `${String(r.entry)}`
      sub = `${String(r.purpose)}（页面：${String(r.page)}）`
      break
    case 'entity':
      title = String(r.name)
      sub = String(r.meaning)
      break
    case 'stateMachine':
      title = String(r.entity)
      break
    case 'lifecycle':
      title = String(r.step)
      break
    case 'command':
      title = String(r.method)
      sub = `${String(r.rust)} @ ${String(r.rustFile)}`
      break
    case 'dto':
      title = `${String(r.name)}（字段字典）`
      sub = `TS ${String(r.ts)} ↔ Rust ${String(r.rust)}`
      break
    case 'problem':
      title = String(r.title)
      break
    case 'complexity':
      title = String(r.item)
      break
    case 'decision':
      title = String(r.problem)
      break
    case 'roadmap':
      title = `${String(r.phase)} — ${String(r.goal)}`
      break
    case 'acceptance':
      title = String(r.criterion)
      break
    case 'unverified':
      title = String(r.item)
      break
    case 'module':
      title = `${String(r.path)}（${String(r.role)}）`
      break
    case 'carrier':
      title = String(r.path)
      break
    case 'write':
      title = `${String(r.target)} ← ${String(r.file)}`
      break
    case 'rule':
      title = String(r.rule)
      break
    case 'conflict':
      title = String(r.title)
      break
    case 'matrix':
      title = String(r.data)
      break
    case 'record':
      title = r.key !== undefined
        ? `${String(r.key)}${r.meaning !== undefined ? ' —— ' + String(r.meaning) : ''}`
        : String(r.step ?? r.flow ?? r.case ?? r.item ?? r.what ?? r.desc ?? rid)
      break
    case 'gap':
      title = String(r.desc ?? r.gap ?? rid)
      break
    default:
      break
  }

  box.append(recordHead(r, title, sub))

  // DTO：字段字典单独渲染
  if (kind === 'dto' && Array.isArray(r.fields)) {
    box.append(el('p', { cls: 'note', text: `持久化：${String(r.persisted)}　用户可见：${String(r.userVisible)}` }))
    const list = el('div', { cls: 'fields' })
    for (const f of r.fields as AnyRecord[]) {
      const d = el('details', { cls: 'field' })
      const s = el('summary')
      s.append(el('code', { text: String(f.name) }))
      s.append(el('span', { cls: 'chip', text: String(f.type) }))
      if (f.required === true) s.append(el('span', { cls: 'chip req', text: '必填' }))
      if (f.userVisible === true) s.append(el('span', { cls: 'chip vis', text: '用户可见' }))
      if (f.derived === true) s.append(el('span', { cls: 'chip', text: '派生' }))
      d.append(s)
      d.append(kvTable(Object.entries(f), { skip: ['name', 'type'] }))
      list.append(d)
    }
    box.append(list)
    box.append(evidenceChips(r.evidence))
    return box
  }

  // 状态机：Mermaid 原文 + 状态/迁移表
  if (kind === 'stateMachine') {
    if (typeof r.mermaid === 'string') {
      const pre = el('pre', { cls: 'mermaid', text: r.mermaid })
      box.append(el('div', { cls: 'cap', text: 'Mermaid 状态图（原文；本页不渲染图形，避免引入外部依赖）' }))
      box.append(pre)
    }
    box.append(kvTable(Object.entries(r), { skip: ['mermaid', 'states', 'transitions'] }))
    if (Array.isArray(r.states)) {
      box.append(el('div', { cls: 'cap', text: '状态' }))
      const t = el('table', { cls: 'kv' })
      for (const st of r.states as AnyRecord[]) {
        const tr = el('tr')
        tr.append(el('th', { text: String(st.name) }))
        const td = el('td')
        td.append(el('div', { text: String(st.condition) }))
        td.append(evidenceChips(st.evidence))
        tr.append(td)
        t.append(tr)
      }
      box.append(t)
    }
    if (Array.isArray(r.transitions)) {
      box.append(el('div', { cls: 'cap', text: '迁移' }))
      const t = el('table', { cls: 'kv' })
      for (const tr0 of r.transitions as AnyRecord[]) {
        const tr = el('tr')
        tr.append(el('th', { text: `${String(tr0.from)} → ${String(tr0.to)}` }))
        const td = el('td')
        td.append(el('div', { text: `触发：${String(tr0.trigger)}` }))
        td.append(el('div', { text: `命令：${String(tr0.command)}` }))
        td.append(evidenceChips(tr0.evidence))
        tr.append(td)
        t.append(tr)
      }
      box.append(t)
    }
    box.append(evidenceChips(r.evidence))
    return box
  }

  // 决策：逐字段成表，方便当需求输入读
  if (kind === 'decision') {
    box.append(kvTable(Object.entries(r), { skip: ['affected'] }))
    const a = r.affected as Record<string, unknown> | undefined
    if (a) {
      box.append(el('div', { cls: 'cap', text: '影响面' }))
      box.append(kvTable(Object.entries(a)))
    }
    box.append(evidenceChips(r.evidence))
    return box
  }

  // 其余：通用键值表
  box.append(kvTable(Object.entries(r)))
  box.append(evidenceChips(r.evidence))
  return box
}

/* ------------------------------------------------------------------ 分节 */

interface Section {
  id: string
  name: string
  build: () => Node[]
}

function groupBy<T extends AnyRecord>(label: string, rows: T[], kind: string): HTMLElement {
  const wrap = el('section', { cls: 'group' })
  const d = el('details', { cls: 'grp' })
  d.open = true
  const s = el('summary')
  s.append(el('strong', { text: label }))
  s.append(el('span', { cls: 'chip', text: `${rows.length} 条` }))
  d.append(s)
  const inner = el('div', { cls: 'grpBody' })
  for (const r of rows) inner.append(renderRecord(kind, r))
  d.append(inner)
  wrap.append(d)
  return wrap
}

function textList(items: string[], cls = 'textList'): HTMLElement {
  const ul = el('ul', { cls })
  for (const s of items) ul.append(el('li', { text: s }))
  return ul
}

function subSection(label: string, ...nodes: Node[]): HTMLElement {
  const s = el('section', { cls: 'sub' })
  s.append(el('h3', { text: label }))
  for (const n of nodes) s.append(n)
  return s
}

const SECTIONS: Section[] = [
  {
    id: 'overview',
    name: '概览',
    build: () => {
      const out: Node[] = []
      out.push(subSection('审计基线', kvTable(Object.entries({
        仓库: DATA.meta.repo,
        分支: DATA.meta.branch,
        HEAD: `${DATA.meta.headShort}（${DATA.meta.head}）`,
        提交时间: DATA.meta.headDate,
        提交说明: DATA.meta.headSubject,
        审计日期: DATA.meta.auditDate,
        工作区: DATA.meta.workingTreeClean ? '干净（无未提交改动）' : '有未提交改动',
        只读审计: DATA.meta.readOnly ? '是（未改任何产品代码）' : '否',
        事实来源: DATA.meta.sourceOfTruth,
      }))))
      out.push(subSection('规模（全部由数据实时统计）', kvTable([
        ['页签 / 页面', `${DATA.pages.length}`],
        ['用户可见动作', `${DATA.actions.length}`],
        ['模块', `${DATA.modules.length}`],
        ['领域实体', `${DATA.entities.length}`],
        ['状态机', `${DATA.stateMachines.length}`],
        ['生命周期步骤', `${DATA.lifecycle.length}`],
        ['契约方法', `${DATA.contract.commands.length}`],
        ['DTO 字段字典', `${DATA.contract.dtos.length} 个结构 / ${DATA.contract.dtos.reduce((n: number, d: AnyRecord) => n + (Array.isArray(d.fields) ? d.fields.length : 0), 0)} 个字段`],
        ['持久化载体', `${DATA.persistence.carriers.length}`],
        ['写盘入口', `${DATA.persistence.writes.length}`],
        ['已确认问题', `${DATA.problems.length}`],
        ['复杂性项', `${DATA.complexity.length}`],
        ['重写裁断', `${DATA.decisions.length}`],
        ['路线图阶段', `${DATA.roadmap.length}`],
        ['待验证事项', `${DATA.unverified.length}`],
      ])))
      out.push(subSection('方法', textList(DATA.meta.method)))
      out.push(subSection('证据等级口径', kvTable(Object.entries(DATA.meta.evidenceLevels))))
      out.push(subSection('裁断词表', kvTable(Object.entries(DATA.meta.verdicts))))
      out.push(subSection('覆盖与局限',
        el('div', { cls: 'cap', text: '已核实' }), textList(DATA.coverage.covered),
        el('div', { cls: 'cap', text: '未核实' }), textList(DATA.coverage.notCovered),
        el('div', { cls: 'cap', text: '证据缺口' }),
        ...DATA.coverage.gaps.map((g) => renderRecord('gap', g)),
        el('div', { cls: 'cap', text: '方法局限' }), textList(DATA.coverage.methodLimitations),
      ))
      return out
    },
  },
  {
    id: 'modules',
    name: '模块地图',
    build: () => {
      const byClass = new Map<string, AnyRecord[]>()
      for (const m of DATA.modules) {
        const k = String(m.pathClass)
        if (!byClass.has(k)) byClass.set(k, [])
        byClass.get(k)!.push(m)
      }
      const out: Node[] = []
      out.push(subSection('工作区', kvTable(Object.entries(DATA.workspace))))
      out.push(subSection('数据根', kvTable(Object.entries(DATA.roots))))
      for (const [k, rows] of byClass) {
        out.push(groupBy(`路径分类：${k}`, rows, 'module'))
      }
      out.push(subSection('模块地图未覆盖的部分', textList(DATA.notCoveredInModuleMap)))
      return out
    },
  },
  {
    id: 'product',
    name: '产品规格',
    build: () => [
      groupBy('页面', DATA.pages, 'page'),
      groupBy('动作（逐条）', DATA.actions, 'action'),
      subSection('动作小结', kvTable(Object.entries(DATA.product.summary))),
    ],
  },
  {
    id: 'domain',
    name: '领域与状态',
    build: () => [
      subSection('数据根与路径', kvTable(Object.entries(DATA.domain.rootsNote))),
      groupBy('实体档案', DATA.entities, 'entity'),
      groupBy('状态机', DATA.stateMachines, 'stateMachine'),
      groupBy('预设完整生命周期', DATA.lifecycle, 'lifecycle'),
      groupBy('数据权威与读写矩阵', DATA.readWriteMatrix, 'matrix'),
      groupBy('已确认的状态冲突', DATA.conflicts, 'conflict'),
    ],
  },
  {
    id: 'contract',
    name: '契约',
    build: () => [
      subSection('命令面小结', kvTable(Object.entries(DATA.contract.summary))),
      groupBy('IPC 命令清单', DATA.contract.commands, 'command'),
      groupBy('DTO 字段字典', DATA.contract.dtos, 'dto'),
      subSection('前后端职责边界', kvTable(Object.entries(DATA.contract.boundaries))),
    ],
  },
  {
    id: 'persistence',
    name: '持久化',
    build: () => [
      subSection('统一写盘出口', kvTable(Object.entries(DATA.persistence.atomicWrite))),
      groupBy('写盘入口清单', DATA.persistence.writes, 'write'),
      groupBy('写盘纪律缺口', DATA.persistence.writeDisciplineGaps, 'gap'),
      groupBy('持久化载体', DATA.persistence.carriers, 'carrier'),
      groupBy('localStorage（纯偏好）', DATA.persistence.localStorage, 'record'),
      subSection('会话态', kvTable(Object.entries(DATA.persistence.sessionState))),
      subSection('启动初始化', kvTable(Object.entries(DATA.persistence.startupInit))),
      groupBy('一致性审计', DATA.persistence.consistency, 'record'),
      groupBy('多步操作的半完成风险', DATA.persistence.halfDoneRisks, 'record'),
    ],
  },
  {
    id: 'cloud',
    name: '云端与发布',
    build: () => [
      subSection('官方源', kvTable(Object.entries(DATA.cloud.source))),
      subSection('目录', kvTable(Object.entries(DATA.cloud.catalog))),
      groupBy('下载与更新链路', DATA.cloud.downloadChain, 'record'),
      groupBy('发布链路', DATA.cloud.publishChain, 'record'),
      groupBy('产品规则逐条核实', DATA.cloud.rules, 'rule'),
      groupBy('开发 / 测试面', DATA.cloud.devTestSurface, 'record'),
    ],
  },
  {
    id: 'findings',
    name: '问题与裁断',
    build: () => [
      groupBy('已确认问题', DATA.problems, 'problem'),
      groupBy('复杂性来源', DATA.complexity, 'complexity'),
      groupBy('重写裁断', DATA.decisions, 'decision'),
    ],
  },
  {
    id: 'target',
    name: '目标模型与路线图',
    build: () => [
      subSection('目标模型（建议，非现状）', kvTable(Object.entries(DATA.targetModel))),
      groupBy('验收标准', DATA.acceptance, 'acceptance'),
      groupBy('重写路线图', DATA.roadmap, 'roadmap'),
    ],
  },
  {
    id: 'unverified',
    name: '待验证',
    build: () => [groupBy('待验证事项', DATA.unverified, 'unverified')],
  },
  {
    id: 'evidence',
    name: '证据索引',
    build: () => {
      const map = new Map<string, string[]>()
      /** 归属分节 id + 记录号 —— 于是点一下就能跳到「那一节的那一条」 */
      const walk = (sectionId: string, rows: AnyRecord[]) => {
        for (const r of rows) {
          const rid = idOf(r)
          const ev = r.evidence
          if (Array.isArray(ev)) {
            for (const e of ev) {
              const s = String(e)
              if (!map.has(s)) map.set(s, [])
              map.get(s)!.push(`${sectionId}/${rid}`)
            }
          }
        }
      }
      walk('modules', DATA.modules)
      walk('product', DATA.pages)
      walk('product', DATA.actions)
      walk('domain', DATA.entities)
      walk('domain', DATA.stateMachines)
      walk('domain', DATA.lifecycle)
      walk('domain', DATA.conflicts)
      walk('contract', DATA.contract.commands)
      walk('contract', DATA.contract.dtos)
      walk('persistence', DATA.persistence.carriers)
      walk('persistence', DATA.persistence.writes)
      walk('persistence', DATA.persistence.writeDisciplineGaps)
      walk('cloud', DATA.cloud.rules)
      walk('cloud', DATA.cloud.downloadChain)
      walk('cloud', DATA.cloud.publishChain)
      walk('findings', DATA.problems)
      walk('findings', DATA.complexity)
      walk('findings', DATA.decisions)
      walk('unverified', DATA.unverified)
      const entries = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))
      const out: Node[] = []
      out.push(el('p', { cls: 'note', text: `${entries.length} 处唯一证据引用（去重后）。${DATA.repositoryPathNote}` }))
      const t = el('table', { cls: 'kv' })
      for (const [ev, owners] of entries) {
        const tr = el('tr')
        tr.append(el('th', { text: ev }))
        const td = el('td')
        for (const o of owners) {
          const b = el('button', { cls: 'linkBtn', text: o })
          b.addEventListener('click', () => {
            location.hash = `#${o}`
          })
          td.append(b)
        }
        tr.append(td)
        t.append(tr)
      }
      out.push(t)
      return out
    },
  },
]

/* ------------------------------------------------------------------ 外壳 */

function buildNav(): { nav: HTMLElement; show: (id: string, focus?: string) => void } {
  const nav = el('nav', { cls: 'nav' })
  const main = document.getElementById('main')!
  const buttons = new Map<string, HTMLButtonElement>()
  const sections = new Map<string, HTMLElement>()
  /*
   * **一次性把十一个分节都建出来**，之后只切 display。
   * 为什么不做懒加载：全局搜索是「跨页面 / 动作 / 实体 / 字段 / 命令 / 路径 / 问题 / 裁断」的，
   * 懒加载会让没打开过的分节搜不到 —— 那就不是全局搜索了。
   */
  for (const s of SECTIONS) {
    const b = el('button', { cls: 'navBtn', text: s.name })
    b.addEventListener('click', () => show(s.id))
    nav.append(b)
    buttons.set(s.id, b)
    const wrap = el('div', { cls: 'section' })
    wrap.dataset.section = s.id
    wrap.style.display = 'none'
    for (const n of s.build()) wrap.append(n)
    sections.set(s.id, wrap)
    main.append(wrap)
  }

  function show(id: string, focus?: string) {
    for (const [k, w] of sections) w.style.display = k === id ? '' : 'none'
    for (const [k, b] of buttons) b.classList.toggle('on', k === id)
    if (focus) {
      const rec = document.querySelector(`[data-id="${CSS.escape(focus)}"]`)
      if (rec) {
        ;(rec as HTMLElement).scrollIntoView({ block: 'center' })
        rec.classList.add('flash')
        setTimeout(() => rec.classList.remove('flash'), 1600)
      }
    } else {
      window.scrollTo({ top: 0 })
    }
  }

  window.addEventListener('hashchange', () => applyHash())
  /*
   * 哈希两种形态都认：
   *   #分节/记录号      （证据索引点出来的、以及页内互跳用的规范形态）
   *   #kind:记录号      （容错：按记录号反查它所在的分节）
   */
  function applyHash() {
    const h = decodeURIComponent(location.hash.replace(/^#/, ''))
    if (!h) return show('overview')
    const slash = h.indexOf('/')
    if (slash > 0) {
      const id = h.slice(0, slash)
      const focus = h.slice(slash + 1)
      if (SECTIONS.some((s) => s.id === id)) return show(id, focus)
    }
    if (SECTIONS.some((s) => s.id === h)) return show(h)
    const rid = h.includes(':') ? h.slice(h.indexOf(':') + 1) : h
    const node = rid ? document.querySelector(`[data-id="${CSS.escape(rid)}"]`) : null
    if (node !== null) {
      const sec = node.closest('.section')?.getAttribute('data-section') ?? 'overview'
      return show(sec, rid)
    }
    show('overview')
  }
  ;(window as unknown as { __applyHash: () => void }).__applyHash = applyHash
  return { nav, show }
}

function wireFilters() {
  const q = document.getElementById('q') as HTMLInputElement
  const lv = document.getElementById('lv') as HTMLSelectElement
  const vd = document.getElementById('vd') as HTMLSelectElement
  const count = document.getElementById('count')!
  const clear = document.getElementById('clear')!

  function apply() {
    const needle = q.value.trim().toLowerCase()
    const lvf = lv.value
    const vdf = vd.value
    let shown = 0
    let total = 0
    for (const rec of document.querySelectorAll<HTMLElement>('.rec')) {
      total++
      const okQ = needle === '' || (rec.dataset.search ?? '').includes(needle)
      const okL = lvf === '' || rec.dataset.level === lvf
      const okV = vdf === '' || rec.dataset.verdict === vdf
      const ok = okQ && okL && okV
      rec.style.display = ok ? '' : 'none'
      if (ok) shown++
    }
    // 组内命中的折叠组自动打开；全空的组隐藏
    for (const grp of document.querySelectorAll<HTMLDetailsElement>('.grp')) {
      const body = grp.querySelector('.grpBody')!
      const vis = [...body.querySelectorAll<HTMLElement>('.rec')].filter((r) => r.style.display !== 'none')
      grp.style.display = vis.length === 0 ? 'none' : ''
      if (needle !== '' || lvf !== '' || vdf !== '') grp.open = vis.length > 0
    }
    for (const sec of document.querySelectorAll<HTMLElement>('.sub')) {
      const any = [...sec.querySelectorAll<HTMLElement>('.rec')].some((r) => r.style.display !== 'none')
      const hasRec = sec.querySelector('.rec') !== null
      sec.style.display = !hasRec || any ? '' : 'none'
    }
    count.textContent = `${shown} / ${total} 条命中`
  }

  q.addEventListener('input', apply)
  lv.addEventListener('change', apply)
  vd.addEventListener('change', apply)
  clear.addEventListener('click', () => {
    q.value = ''
    lv.value = ''
    vd.value = ''
    apply()
  })
  ;(window as unknown as { __applyFilters: () => void }).__applyFilters = apply
}

function boot() {
  const meta = DATA.meta
  document.title = meta.title
  const head = document.getElementById('baseline')!
  head.append(kvTable([
    ['仓库', meta.repo],
    ['分支', meta.branch],
    ['HEAD', `${meta.headShort} / ${meta.head}`],
    ['提交时间', meta.headDate],
    ['审计日期', meta.auditDate],
  ], { skip: [] }))

  const { nav } = buildNav()
  document.getElementById('navHost')!.append(nav)
  wireFilters()
  ;(window as unknown as { __applyHash: () => void }).__applyHash()
  ;(window as unknown as { __applyFilters: () => void }).__applyFilters()

  const stats = {
    pages: DATA.pages.length,
    actions: DATA.actions.length,
    modules: DATA.modules.length,
    entities: DATA.entities.length,
    commands: DATA.contract.commands.length,
    problems: DATA.problems.length,
    decisions: DATA.decisions.length,
    unverified: DATA.unverified.length,
  }
  const s = document.getElementById('stats')!
  for (const [k, v] of Object.entries(stats)) {
    const b = el('span', { cls: 'stat' })
    b.append(el('strong', { text: String(v) }))
    b.append(el('span', { text: k }))
    s.append(b)
  }
}

boot()
