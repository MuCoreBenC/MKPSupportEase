/*
 * 开发用 IPC 桩（**只在 vite dev 且没有 Tauri IPC 时生效**）。
 *
 * 为什么要有它：参数台的验收判据是 C14 README 里那些**量过的数**
 * （行高 43/39、G-code 框 172×88、模态框 630×547、弃用四处落法……），
 * 量它们必须让页面带着数据渲染起来，而在浏览器里没有 Tauri `invoke`。
 * 这个桩把一份**手写夹具**（形状 = api.ts 的 DTO 手抄镜像）从
 * `window.__TAURI_INTERNALS__.invoke` 后面喂给界面，让验收可以在浏览器里做。
 *
 * 边界（重要）：
 *  - 这里**没有任何业务判定**搬进前端 —— 夹具里的弃用标记、blocked 句子、
 *    impact 等，都是照真后端的行为**手工预填/按同一规则现算的测试数据**，
 *    判据的唯一出处仍在 src-tauri；
 *  - 真机（tauri dev / 打包）下这段代码整个不安装，`import.meta.env.DEV`
 *    为 false 时连打包产物都不带它；
 *  - 写路径在这里是**内存里的假草稿**，落盘、快照、指纹这些一概没有。
 */

type Json = Record<string, unknown>

/* ---------- 词表（照抄 domain/wording.rs 的字面量） ---------- */

const WORDS: Json = {
  build: {
    built: { label: '已生成', explain: '磁盘里的生成物和当前配方一致，不用重新生成' },
    stale: { label: '待重新生成', explain: '磁盘里的生成物跟当前配方不一致了 —— 生成之后客户端才会拿到新的' },
    neverBuilt: { label: '未生成', explain: '还没生成过，客户端现在下载不到这一版' },
    noResources: { label: '配方文件缺失', explain: '这个版本没有配方文件 —— 这是异常，不是正常状态：版本一建出来就该带着配方文件。参数照样能看能改，但生成不了' },
  },
  artifact: {
    fresh: { label: '已生成', explain: '所有该有产物的版本都是最新的' },
    stale: { label: '待重新生成', explain: '有版本的配方改过了，产物还没跟上' },
    missing: { label: '未生成', explain: '一个产物都还没生成过' },
  },
  save: { saved: { label: '已保存', explain: null }, dirty: { label: '未保存', explain: null } },
  bbsAssign: {
    assigned: { label: '已分配', explain: '在某个套餐里，客户端装那个套餐就会拿到' },
    optional: { label: '可选', explain: '没进任何套餐，客户端能手动下' },
    archiveOnly: { label: '仅归档', explain: '客户端完全不知道这个文件存在' },
  },
  bbsSource: {
    own: { label: '本版本单独一份', explain: '这个版本自己挑的曲线清单' },
    inheritedFromMachine: { label: '跟随机型', explain: '用机型默认那份清单' },
  },
  origin: {
    factory: { label: '出厂', explain: '注册表里的默认值，全机型共用' },
    machine: { label: '机型', explain: '这台机器的基底，没自己写过的版本都跟着它' },
    version: { label: '版本', explain: '这个版本自己钉着的值' },
  },
  level: { machine: { label: '机型基底', explain: null }, version: { label: '版本', explain: null } },
  visibility: {
    menu: { label: '菜单可见', explain: '客户端的菜单里能看到、能下载' },
    archiveOnly: { label: '仅归档', explain: '文件在仓库里，但客户端看不到' },
  },
  bulkKind: {
    detaching: { label: '挂回继承', explain: null },
    changing: { label: '改值', explain: null },
    noChange: { label: '值不变', explain: null },
  },
  placeholder: {
    blank: '空',
    notApplicable: '不适用',
    undeclared: '未声明',
    unconfigured: '未配置',
    unsupported: '暂不支持',
  },
  disabled: {
    detachNothing: '这一层没有单独设过这一项，本来就是继承来的',
    detachReady: '删掉这一层的这一项，让它跟着上一层变',
    blockedByCondition: '上一项没打开，这一项现在不生效，所以不让改',
    notApplicable: '这台机型没有这一项，不是值为空',
    bulkRefusesGcode: 'G-code 不做批量：一段多行脚本被整体盖掉是不可逆的误操作，请逐列点开改',
    buildBlocked: '有阻断问题没解决，生成一定会出错',
    buildNothingToDo: '所有产物都和当前配方一致，没有要生成的',
    buildNoResources: '这一版没有可交付的产物，生成不出东西来',
    nothingToSave: '没有未保存的改动',
    nothingToUndo: '没有可以撤销的操作',
    notUndoable: '删除和生成记录不进撤销栈；删掉的版本在回收站里',
    deprecatedWriteBlocked: '已弃用，不能改（上游已标记）',
  },
  empty: {
    noIssues: '都过了 —— 没有阻断、没有待办、没有提示',
    trashEmpty: '回收站是空的',
    noDisabledFallback: '当前没有关掉的回退规则',
    matrixNoMatch: '没有匹配的参数',
    matrixNoCols: '先勾选要对照的列',
    matrixSearchSpansAllTabs: '搜索跨全部分类',
  },
  relate: { goFixIt: '去改那一项', showAnyway: '仍然展开看' },
  snapshot: {
    current: { label: '快照已跟上', explain: '停手之后已经写过一次崩溃快照，现在崩了也不丢' },
    pending: { label: '待落盘', explain: '刚改的还在内存里，停手 2 秒后会写一次快照' },
    failed: { label: '快照写不进去', explain: '快照写不进去，这会儿崩了会丢掉未保存的改动。改动本身没受影响' },
  },
  paramStatus: {
    factory: { label: '出厂默认', explain: '注册表里的默认值，这台机器没改过、这个版本也没改过' },
    machine: { label: '机型默认', explain: '来自这台机器的基底，这个版本自己没钉 —— 改基底它会跟着变' },
    version: { label: '本版修改', explain: '这个版本自己钉着的值，不跟随机型基底' },
    dirty: { label: '已修改', explain: '改了还没保存 —— 保存之后才会写进配方' },
  },
  paramDeprecated: {
    label: '已弃用',
    explain: '上游已标记这一项不再使用 —— 值照旧读得到（老配方里可能还写着它），但不要再改它',
  },
  paramDeprecatedChoice: {
    label: '已弃用',
    explain: '这一档放开的参数已经全部弃用 —— 选它不会带来任何还改得动的东西',
  },
  matrixRow: {
    notOwn: { label: '本机无此项', explain: '这一行是别的机型的参数，这台基准机型没有 —— 没有基准可比' },
    diff: { label: '差异', explain: '有勾选列的值与机型基底不同 —— 绿底的那几格就是' },
    same: { label: '一致', explain: '勾选列的值都与机型基底一致' },
  },
}

/* ---------- 参数夹具（照 ParamView / ChoiceView 的形状手写） ---------- */

interface FixtureParam {
  key: string
  label: string
  desc: string
  sectionId: string
  tabId: string
  order: number
  valueType: 'float' | 'int' | 'bool' | 'string'
  uiComponent: 'number' | 'switch' | 'segmented' | 'select' | 'gcode'
  defaultValue: unknown
  min: number | null
  max: number | null
  step: number | null
  unit: string | null
  choices: { label: string; value: string; deprecated: boolean }[]
  showWhen: { key: string; op: 'eq' | 'neq' | 'gt'; value: unknown } | null
  parentKey: string | null
  depth: number
  deprecated: boolean
}

const p = (over: Partial<FixtureParam> & { key: string; label: string }): FixtureParam => ({
  desc: '',
  sectionId: 'wipe',
  tabId: 'wiping',
  order: 1,
  valueType: 'float',
  uiComponent: 'number',
  defaultValue: 0,
  min: null,
  max: null,
  step: null,
  unit: null,
  choices: [],
  showWhen: null,
  parentKey: null,
  depth: 0,
  deprecated: false,
  ...over,
})

const PARAMS: FixtureParam[] = [
  p({ key: 'wiping.mode', label: '擦料方式', order: 1, valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'tower',
      choices: [{ label: '擦料塔', value: 'tower', deprecated: false }, { label: '圆盘擦拭', value: 'disk', deprecated: false }] }),
  p({ key: 'wiping.speed', label: '擦料塔速度与质量', order: 2, defaultValue: 45, min: 0, max: 100, step: 1, unit: 'mm/s',
      parentKey: 'wiping.mode', depth: 1,
      showWhen: { key: 'wiping.mode', op: 'eq', value: 'tower' } }),
  p({ key: 'wiping.legacy', label: '旧版擦料计数', order: 3, defaultValue: 7, deprecated: true }),
  p({ key: 'wiping.have_components', label: '启用擦料组', order: 4, valueType: 'bool', uiComponent: 'switch', defaultValue: false }),
  p({ key: 'frame.type', label: '外围结构', order: 5, sectionId: 'shell', valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'brim',
      choices: [
        { label: '斜肋外墙', value: 'brim', deprecated: false },
        /* 「护套」放开的 2 条（frame.shell.*）全被上游标了弃用 —— 推出来的选项级弃用 */
        { label: '护套', value: 'sheath', deprecated: true },
      ] }),
  p({ key: 'frame.shell.speed', label: '护套打印速度', order: 6, sectionId: 'shell', defaultValue: 40, min: 1, max: 200, step: 1,
      parentKey: 'frame.type', depth: 1, deprecated: true,
      showWhen: { key: 'frame.type', op: 'eq', value: 'sheath' } }),
  p({ key: 'frame.shell.wall', label: '护套壁宽', order: 7, sectionId: 'shell', defaultValue: 0.8, min: 0.1, max: 3, step: 0.1, unit: 'mm',
      parentKey: 'frame.type', depth: 1, deprecated: true,
      showWhen: { key: 'frame.type', op: 'eq', value: 'sheath' } }),
  p({ key: 'ironing.threshold', label: '熨烫覆盖阈值', order: 8, sectionId: 'ironing', tabId: 'ironing', valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'off',
      choices: [
        { label: '关闭', value: 'off', deprecated: false },
        { label: '稀疏一半', value: 'half', deprecated: false },
        { label: '稀疏90%', value: 'p90', deprecated: false },
      ] }),
  p({ key: 'toolhead.script', label: '装载胶箱 G-code', order: 9, sectionId: 'space', tabId: 'space',
      valueType: 'string', uiComponent: 'gcode', defaultValue: GCODE_SAMPLE() }),
]

function GCODE_SAMPLE(): string {
  return [
    'G92 E0',
    'M204 S800',
    'G1 X261 F10000',
    'L801 ;wipe tower start',
    'T0',
    'G1 E-5 F2400',
    'L803;此处使喷嘴降温',
    'S[AUTO] F42000',
    'M104 S0',
  ].join('\n')
}

const TABS = [
  { id: 'wiping', label: '擦料', order: 1, sections: [
    { id: 'wipe', label: '擦料方式', order: 1 },
    { id: 'shell', label: '外围结构', order: 2 },
  ] },
  { id: 'ironing', label: '熨烫', order: 2, sections: [{ id: 'ironing', label: '熨烫覆盖', order: 1 }] },
  { id: 'space', label: '偏移', order: 3, sections: [{ id: 'space', label: '空间偏移', order: 1 }] },
]

const SECTION_LABEL = new Map(TABS.flatMap((t) => t.sections.map((s) => [s.id, `${t.label} · ${s.label}`])))

/* ---------- 三层取值（夹具自己的值域；判定规则照后端文档，不算前端业务） ---------- */

const MACHINES = [
  { id: 'A1', display: 'A1', versions: [
    { uid: 'A1/STANDARD', versionId: 'STANDARD', name: '标准版', tag: '推荐' },
    { uid: 'A1/FAST', versionId: 'FAST', name: '高速版', tag: null },
  ] },
]

/** 盘上那一份：机型基底与各版本钉着的键（save 之后这里跟着变） */
const atRest: { base: Record<string, Json>; over: Record<string, Json> } = {
  base: { A1: { 'wiping.speed': 45 } },
  over: {
    'A1/STANDARD': { 'wiping.speed': 70 },
    'A1/FAST': {},
  },
}

/** 草稿里还没保存的改动（level|owner|key → value；null = 删键） */
const pending = new Map<string, unknown>()

const splitKey = (raw: string) => {
  const [level, owner, ...rest] = raw.split('|')
  return { level: level as 'machine' | 'version', owner, key: rest.join('|') }
}

function pinned(level: 'machine' | 'version', owner: string, key: string): { has: boolean; value: unknown } {
  if (pending.has(`${level}|${owner}|${key}`)) {
    const v = pending.get(`${level}|${owner}|${key}`)
    return { has: v !== null, value: v }
  }
  const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
  if (table && key in table) return { has: true, value: table[key] }
  return { has: false, value: undefined }
}

function effective(machineId: string, uid: string | null, key: string) {
  if (uid !== null) {
    const hit = pinned('version', uid, key)
    if (hit.has) return { value: hit.value, origin: 'version', own: true }
  }
  const hit = pinned('machine', machineId, key)
  if (hit.has) return { value: hit.value, origin: 'machine', own: true }
  const pdef = PARAMS.find((x) => x.key === key)
  return pdef ? { value: pdef.defaultValue, origin: 'factory', own: false } : null
}

function valueText(pdef: FixtureParam, v: unknown): string {
  if (pdef.uiComponent === 'switch') return v === true || v === 'true' ? '开启' : '关闭'
  if (pdef.uiComponent === 'gcode') {
    const s = String(v ?? '')
    return s === '' ? '空' : `${s.split('\n').length} 行 · 点开`
  }
  const hit = pdef.choices.find((c) => String(c.value) === String(v))
  if (hit) return hit.label
  const text = v === '' || v === null || v === undefined ? '空' : String(v)
  return pdef.unit ? `${text} ${pdef.unit}` : text
}

/* ---------- Desk / Book / Registry 的现算（照 DTO 形状） ---------- */

function blockedOf(machineId: string, uid: string | null, pdef: FixtureParam): Json[] {
  const blocked: Json[] = []
  if (pdef.showWhen) {
    const dep = effective(machineId, uid, pdef.showWhen.key)
    if (String(dep?.value) !== String(pdef.showWhen.value)) {
      const depP = PARAMS.find((x) => x.key === pdef.showWhen?.key)
      blocked.push({
        key: pdef.showWhen.key,
        label: depP?.label ?? pdef.showWhen.key,
        need: `等于 ${depP ? valueText(depP, pdef.showWhen.value) : String(pdef.showWhen.value)}`,
        scope: 'field',
      })
    }
  }
  return blocked
}

function cellOf(machineId: string, uid: string | null, pdef: FixtureParam): Json {
  const hit = effective(machineId, uid, pdef.key)
  const raw = hit?.value
  const blocked = blockedOf(machineId, uid, pdef)
  const deprecated = pdef.deprecated
  const editable = blocked.length === 0 && !deprecated
  const originExplain =
    hit ? ((WORDS.origin as Json)[hit.origin] as Json | undefined)?.explain ?? null : null
  return {
    kind: blocked.length > 0 ? 'notApplicable' : pdef.uiComponent === 'gcode' ? 'gcode' : 'value',
    text: valueText(pdef, raw),
    lines: pdef.uiComponent === 'gcode' ? String(raw ?? '').split('\n').length : null,
    origin: hit?.origin ?? null,
    originLabel: hit ? ({ factory: '出厂', machine: '机型', version: '版本' } as Record<string, string>)[hit.origin] : null,
    originExplain,
    own: hit?.own ?? false,
    dirty: pending.has(`${uid !== null ? 'version' : 'machine'}|${uid ?? machineId}|${pdef.key}`),
    editable,
    reason: blocked.length > 0 ? '上一项没打开，这一项现在不生效，所以不让改'
      : deprecated ? '上游已标记这一项不再使用 —— 值照旧读得到（老配方里可能还写着它），但不要再改它'
      : null,
    blocked,
    blockedNote: blocked.length > 0 ? `改不动：由「${(blocked[0] as Json).label}」控制，需${(blocked[0] as Json).need}` : null,
    blockedHint: blocked.length > 0 ? `要 ${(blocked[0] as Json).label} ${(blocked[0] as Json).need} 才可改` : null,
    jumpTo: blocked.length > 0 ? (blocked[0] as Json).key : null,
    raw: raw ?? null,
  }
}

/* origin explain 已在 cellOf 里从词表现取。 */

function impactOf(machineId: string, uid: string | null, key: string): Json {
  const label = (v: { name: string }) => `${machineId} / ${v.name}`
  const pins = (v: { uid: string }) => atRest.over[v.uid]?.[key] !== undefined
  const mine = MACHINES[0]!.versions
  if (uid !== null) {
    return {
      targets: mine.filter((v) => v.uid === uid).map((v) => label(v)),
      followers: mine.filter((v) => v.uid !== uid && !pins(v)).map((v) => label(v)),
    }
  }
  return { targets: mine.filter((v) => !pins(v)).map((v) => label(v)), followers: [] }
}

function rowOf(machineId: string, uid: string | null, pdef: FixtureParam): Json {
  const parent = pdef.parentKey ? PARAMS.find((x) => x.key === pdef.parentKey) : null
  return {
    key: pdef.key,
    label: pdef.label,
    desc: pdef.desc,
    unit: pdef.unit,
    sectionId: pdef.sectionId,
    sectionLabel: SECTION_LABEL.get(pdef.sectionId) ?? pdef.sectionId,
    tabId: pdef.tabId,
    depth: pdef.depth,
    parentKey: pdef.parentKey,
    parentLabel: parent?.label ?? null,
    parentNote: parent ? `属于：${parent.label}` : null,
    controlNote: pdef.showWhen ? `受「${PARAMS.find((x) => x.key === pdef.showWhen?.key)?.label}」控制` : null,
    gcode: pdef.uiComponent === 'gcode',
    deprecated: pdef.deprecated,
    impact: impactOf(machineId, uid, pdef.key),
    cells: [null, 'A1/STANDARD', 'A1/FAST'].map((u) => cellOf(machineId, u, pdef)),
  }
}

function buildDesk(machineId: string, uid: string | null, tab: string | null, query: string): Json {
  const cols = [
    { key: machineId, machineId, versionUid: null, level: 'machine', machine: 'A1', label: '机型基底', items: Object.keys(atRest.base[machineId] ?? {}).length },
    ...MACHINES[0]!.versions.map((v) => ({
      key: v.uid, machineId, versionUid: v.uid, level: 'version', machine: 'A1',
      label: v.name, items: Object.keys(atRest.over[v.uid] ?? {}).length,
    })),
  ]
  const cur = cols.findIndex((c) => (c.versionUid ?? null) === uid)
  const q = query.trim().toLowerCase()
  const params = PARAMS.filter((pdef) => {
    if (q) return `${pdef.label} ${pdef.key}`.toLowerCase().includes(q)
    return tab === null || pdef.tabId === tab
  })
  const groups: { sectionId: string; label: string; count: number; offNote: null; items: Json[] }[] = []
  for (const pdef of params) {
    const row = rowOf(machineId, uid, pdef)
    let group = groups[groups.length - 1]
    if (!group || group.sectionId !== pdef.sectionId) {
      group = { sectionId: pdef.sectionId, label: SECTION_LABEL.get(pdef.sectionId) ?? pdef.sectionId, count: 0, offNote: null, items: [] }
      groups.push(group)
    }
    group.count += 1
    if (pdef.parentKey !== null && !q) {
      const parentRow = group.items.find((it) => ((it as Json).row as Json).key === pdef.parentKey)
      if (parentRow) {
        ;((parentRow as Json).children as Json[]).push(row)
        continue
      }
    }
    group.items.push({ row, children: [], offNote: null })
  }
  const nav = TABS.map((t) => ({
    id: t.id,
    label: t.label,
    count: PARAMS.filter((pdef) => pdef.tabId === t.id).length,
    sections: t.sections.map((s) => ({
      id: s.id,
      label: s.label,
      count: PARAMS.filter((pdef) => pdef.sectionId === s.id).length,
    })),
  }))
  return { nav, cols, cur: cur >= 0 ? cur : 0, groups, total: PARAMS.length, note: q ? '搜索跨全部分类' : null, emptyReason: null }
}

/** 对照矩阵（C14 第四轮）：基准机型判差异、行序跟基准走 —— 照真后端的规矩 */
function buildMatrix(
  cols: { machineId: string; versionUid: string | null }[],
  tab: string | null,
  query: string,
  baseMachineId: string | null,
): Json {
  const q = query.trim().toLowerCase()
  const colDefs = cols.map((c) => {
    const m = MACHINES.find((x) => x.id === c.machineId)
    const v = m?.versions.find((x) => x.uid === c.versionUid) ?? null
    const uid = c.versionUid
    return {
      key: uid ?? c.machineId,
      machineId: c.machineId,
      versionUid: uid,
      level: uid === null ? 'machine' : 'version',
      machine: m?.display ?? c.machineId,
      label: uid === null ? '机型基底' : (v?.name ?? uid),
      items: Object.keys(uid === null ? (atRest.base[c.machineId] ?? {}) : (atRest.over[uid] ?? {})).length,
    }
  })
  const params = PARAMS.filter((pdef) => {
    if (q) return `${pdef.label} ${pdef.key}`.toLowerCase().includes(q)
    return tab === null || pdef.tabId === tab
  })
  const baseTextOf = (key: string): string | null => {
    if (!baseMachineId) return null
    const pdef = PARAMS.find((x) => x.key === key)
    if (!pdef) return null
    const hit = effective(baseMachineId, null, key)
    return hit ? valueText(pdef, hit.value) : null
  }
  const rows: Json[] = []
  const diffKeys: string[] = []
  const notOwnKeys: string[] = []
  for (const pdef of params) {
    const row = rowOf(baseMachineId ?? 'A1', null, pdef)
    ;(row as Json).impact = null
    const cells = colDefs.map((c) => {
      const cell = cellOf(c.machineId, c.versionUid, pdef)
      const isBaseCol = c.machineId === baseMachineId && c.level === 'machine'
      const bt = baseTextOf(pdef.key)
      const differs = bt !== null && !isBaseCol && cell.text !== bt
      if (differs) {
        cell.differs = true
        cell.diffTip = `机型基底是 ${bt}`
      }
      return cell
    })
    if (cells.some((c) => c.differs === true)) diffKeys.push(pdef.key)
    rows.push(row)
    ;(row as Json).cells = cells
  }
  void notOwnKeys
  return {
    cols: colDefs,
    rows,
    totalRows: PARAMS.length,
    note: q ? '搜索跨全部分类' : null,
    emptyReason: null,
    diffKeys,
    notOwnKeys,
  }
}

/** 批量预览（C14 批量 + 产品纪律「先看后写」）：每一列的前后值与跳过原因 */
function previewBulkOf(key: string, value: unknown, cols: { machineId: string; versionUid: string | null }[]): Json {
  const pdef = PARAMS.find((x) => x.key === key)
  if (!pdef) {
    return { key, label: key, allowed: false, blockedReason: '字段定义里没有这一项', effects: [], skipped: [] }
  }
  const out: Json = {
    key,
    label: pdef.label,
    allowed: pdef.uiComponent !== 'gcode',
    blockedReason: pdef.uiComponent === 'gcode' ? 'G-code 不做批量' : null,
    effects: [] as Json[],
    skipped: [] as Json[],
  }
  if (!out.allowed) return out
  for (const c of cols) {
    const m = MACHINES.find((x) => x.id === c.machineId)
    const v = m?.versions.find((x) => x.uid === c.versionUid) ?? null
    const label = v ? v.name : '机型基底'
    const blocked = blockedOf(c.machineId, c.versionUid, pdef)
    if (blocked.length > 0) {
      ;(out.skipped as Json[]).push({
        col: c.versionUid ?? c.machineId, machine: m?.display ?? c.machineId, label,
        reason: '上一项没打开，这一项现在不生效，所以不让改', blocked,
      })
      continue
    }
    const hit = effective(c.machineId, c.versionUid, key)
    const hadOwn = hit?.origin === (c.versionUid === null ? 'machine' : 'version')
    const kind = hit && String(hit.value) === String(value) ? 'noChange' : hadOwn ? 'changing' : 'detaching'
    ;(out.effects as Json[]).push({
      col: c.versionUid ?? c.machineId, machine: m?.display ?? c.machineId, label,
      level: c.versionUid === null ? 'machine' : 'version',
      before: hit ? valueText(pdef, hit.value) : '不适用',
      after: valueText(pdef, value),
      kind,
    })
  }
  return out
}

function buildBook(): Json {
  const dirtyCount = pending.size
  return {
    machines: MACHINES.map((m) => ({
      id: m.id, display: m.display, icon: null, items: Object.keys(atRest.base[m.id] ?? {}).length,
      build: 'built', dimensionsMissing: false,
      versions: m.versions.map((v) => ({
        uid: v.uid, versionId: v.versionId, name: v.name, tag: v.tag,
        items: Object.keys(atRest.over[v.uid] ?? {}).length,
        build: 'stale', bbsSource: 'own', bbsCount: 2, recipeEmpty: false,
        lastBuild: '2026-09-20 10:00:00', orphanKeys: [],
      })),
    })),
    badges: { machines: 1, versions: 2, baseItems: 2, overrideItems: 1 },
    dirtyCount,
    save: dirtyCount > 0 ? 'dirty' : 'saved',
    artifact: 'stale',
    lastBuild: '2026-09-20 10:00:00',
    buildRows: [],
    notices: [],
    snapshot: dirtyCount > 0 ? 'pending' : 'current',
  }
}

function buildRegistry(): Json {
  return {
    updated: '2026-09-01 08:00:00',
    tabs: TABS,
    params: PARAMS.map((pdef) => ({
      key: pdef.key, label: pdef.label, desc: pdef.desc, tomlKey: pdef.key,
      sectionId: pdef.sectionId, tabId: pdef.tabId, order: pdef.order,
      valueType: pdef.valueType, uiComponent: pdef.uiComponent,
      defaultValue: pdef.defaultValue, defaultText: valueText(pdef, pdef.defaultValue),
      min: pdef.min, max: pdef.max, step: pdef.step, unit: pdef.unit,
      choices: pdef.choices, showWhen: pdef.showWhen, parentKey: pdef.parentKey,
      machineFilter: [], deprecated: pdef.deprecated,
    })),
  }
}

/* ---------- 命令分发 ---------- */

export function installMockBackend() {
  const invoke = (cmd: string, args?: Json) => {
    const machineId = 'A1'
    switch (cmd) {
      case 'wb_boot':
        return Promise.resolve({
          roots: { workbench: 'C:\\dev\\workbench', dist: 'C:\\dev\\dist', upstream: 'C:\\dev\\upstream' },
          problem: null, detail: null,
          info: { registryUpdated: '2026-09-01', manifestUpdated: '2026-09-01', channel: 'dev',
                  minimumClient: null, latestRelease: null, params: PARAMS.length, machines: 1, deliverables: 9, fallbacks: 12 },
        })
      case 'wb_words':
        return Promise.resolve(WORDS)
      case 'wb_book':
        return Promise.resolve(buildBook())
      case 'wb_registry':
        return Promise.resolve(buildRegistry())
      case 'wb_desk':
        return Promise.resolve(buildDesk(machineId, (args?.uid as string | null) ?? null, (args?.tab as string | null) ?? null, (args?.query as string) ?? ''))
      case 'wb_matrix':
        return Promise.resolve(
          buildMatrix(
            (args?.cols as { machineId: string; versionUid: string | null }[]) ?? [],
            (args?.tab as string | null) ?? null,
            (args?.query as string) ?? '',
            (args?.baseMachineId as string | null) ?? null,
          ),
        )
      case 'wb_preview_bulk':
        return Promise.resolve(
          previewBulkOf(
            args?.key as string,
            args?.value,
            (args?.cols as { machineId: string; versionUid: string | null }[]) ?? [],
          ),
        )
      case 'wb_preflight':
        return Promise.resolve({ issues: [], blocks: 0, todos: 0, hints: 0, emptyHint: '都过了 —— 没有阻断、没有待办、没有提示' })
      case 'wb_machines':
        return Promise.resolve({ brands: [], machines: [], root: 'C:\\dev\\workbench' })
      case 'wb_apply_draft': {
        const patches = (args?.patches as Json[]) ?? []
        const inverse: Json[] = []
        const atRestOf = (level: 'machine' | 'version', owner: string, key: string) => {
          const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
          return table && key in table ? table[key] : null
        }
        for (const patch of patches) {
          if (patch.kind !== 'setValue') continue
          const { level, owner, key, value } = patch as never as { level: 'machine' | 'version'; owner: string; key: string; value: unknown }
          const vk = `${level}|${owner}|${key}`
          const before = pending.has(vk) ? pending.get(vk) : atRestOf(level, owner, key)
          if (before === value) continue
          inverse.push({ kind: 'setValue', level, owner, key, value: before })
          // 改回了盘上那个值 = 从草稿里拿掉（照后端 apply_one 的规矩）
          if (atRestOf(level, owner, key) === value) pending.delete(vk)
          else pending.set(vk, value)
        }
        inverse.reverse()
        const refresh = args?.refresh as Json | null
        const desk = refresh && refresh.page === 'desk'
          ? buildDesk(machineId, (refresh.uid as string | null) ?? null, (refresh.tab as string | null) ?? null, (refresh.query as string) ?? '')
          : null
        return Promise.resolve({ view: buildBook(), inverse, undoable: true, desk, matrix: null })
      }
      case 'wb_save': {
        for (const [vk, value] of pending) {
          const { level, owner, key } = splitKey(vk)
          const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
          if (table) {
            if (value === null) delete table[key]
            else table[key] = value
          }
        }
        pending.clear()
        return Promise.resolve({ view: buildBook(), remap: {} })
      }
      case 'wb_discard':
        pending.clear()
        return Promise.resolve(buildBook())
      case 'wb_diff_draft':
        return Promise.resolve([])
      case 'wb_ui':
        return Promise.resolve({})
      case 'wb_save_ui':
        return Promise.resolve(null)
      case 'wb_reload':
        return Promise.resolve({
          roots: { workbench: 'C:\\dev\\workbench', dist: 'C:\\dev\\dist', upstream: 'C:\\dev\\upstream' },
          problem: null, detail: null, info: null,
        })
      default:
        return Promise.reject({ code: 'NOT_IMPLEMENTED', message: `开发桩没有实现 ${cmd}`, traceId: 'mock' })
    }
  }
  ;(window as unknown as Json).__TAURI_INTERNALS__ = { invoke }
  console.info('[workbench] dev mock backend installed —— 没有 Tauri，写路径只落在内存里')
}

