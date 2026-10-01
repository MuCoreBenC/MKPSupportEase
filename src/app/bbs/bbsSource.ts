/*
 * 预设清单的归一与分组 —— 译自 machine-motion/src/bbs/preset-source.js，逻辑一行没改。
 * 纯函数：不碰 DOM、不发请求。
 *
 * 来源只有一处：serve 期端点 `/api/bbs/presets`（读你机器上 BBS 的目录，实时）。
 * 本仓不打包那 285 个预设快照（作者裁决 §6-2），所以没有「退到仓库快照」这一档 ——
 * 读不到本机目录就是空态，由页面照实说。
 *
 * 分组与顺序照 BBS 自己的（PresetComboBoxes.cpp:1382-1387，那里注释写着
 * "BBS: move system to the end"）：用户配置在前、系统配置在后、不支持的配置垫底且置灰。
 * 组名取自 BBS 的 .po：User presets→用户配置、System presets→系统配置、
 * Unsupported presets→不支持的配置。
 *
 * 机型怎么认（这页最容易搞错的一点）：用户预设的名字里常常没有「@BBL XXX」
 * （`0.08mm MKP`、`MKPProcess A1 mini` 都是），所以不能只看名字 —— 要顺着 inherits
 * 往上走，落到系统预设那一层，名字里才有 @BBL。再不行才退 compatible_printers 原文。
 */

import type {
  BbsListEntry,
  BbsListRaw,
  BbsMenuGroup,
  BbsMenuItem,
  BbsPresetItem,
  BbsTargetOption,
  BbsValue,
} from './bbsTypes'

export const GROUP_IMPORTED = '导入的'
export const GROUP_USER = '用户配置'
export const GROUP_SYSTEM = '系统配置'
/* BBS 那一组叫「不支持的配置」（.po 里 Unsupported presets）。这一页语义稍宽：
   坏文件 + 别的机型的用户预设都落这里，所以叫「其他机型 / 读不了的」更准。 */
export const GROUP_OTHER = '其他机型 / 读不了的'
export const MODEL_ALL = 'all'

const norm1 = (v: BbsValue | null | undefined) => (Array.isArray(v) ? v[0] : v)

/* 从名字里抠 @BBL 后面那段（`0.20mm Standard @BBL A1M` → `A1M`，
   `0.06mm Fine @BBL A1M 0.2 nozzle` → `A1M`） */
export function modelFromName(name: string | undefined | null): string | null {
  const m = String(name || '').match(/@BBL\s+([A-Za-z0-9]+)/)
  return m ? m[1] : null
}

/* 从 compatible_printers 拆出打印机名与喷嘴直径。
 * `Bambu Lab A1 mini 0.4 nozzle` → { printer:'Bambu Lab A1 mini', nozzle:'0.4' }
 * BBL 的系统预设里，**名字不带 nozzle 的那一档就是 0.4**（实测 233 份里每份都带 compatible_printers，
 * 且没有一份跨多个喷嘴），所以拆不出直径时按 0.4 兜底。 */
export function printerOf(cp: BbsValue | null | undefined): string | null {
  const s = norm1(cp)
  return s ? String(s).replace(/\s+[\d.]+\s*nozzle$/i, '').trim() : null
}

export function nozzleOf(cp: BbsValue | null | undefined): string | null {
  const s = norm1(cp)
  if (!s) return null
  const m = String(s).match(/([\d.]+)\s*nozzle/i)
  return m ? m[1] : '0.4'
}

interface TargetGuess {
  model: string
  printer: string
  nozzle: string
}

/* 顺 inherits 往上找「这份预设属于哪台机、哪个喷嘴」；byName 是「预设名 → 项」的索引。
 * 用户预设**先跳到父预设**再认：自己的名字是人手起的，`0.12mm Fine @BBL A1Mmkp` 这种会被当成
 * 机型 `A1Mmkp`；它的 compatible_printers 也可能被改过。父预设是系统预设，两者都规范。 */
export function resolveTarget(
  item: BbsPresetItem,
  byName: Map<string, BbsPresetItem>,
  maxDepth = 12,
): TargetGuess {
  let cur: BbsPresetItem | undefined =
    (item.scope !== 'system' && item.inherits) ? byName.get(item.inherits) : item
  if (!cur) cur = item
  let depth = 0
  while (cur && depth++ < maxDepth) {
    const printer = printerOf(cur.compatible_printers)
    if (printer) {
      return {
        model: modelFromName(cur.name) || printer,
        printer,
        nozzle: nozzleOf(cur.compatible_printers) || '?',
      }
    }
    const hit = modelFromName(cur.name)
    if (hit) return { model: hit, printer: hit, nozzle: '?' }
    const next: BbsPresetItem | undefined = cur.inherits ? byName.get(cur.inherits) : undefined
    if (!next || next === cur) break
    cur = next
  }
  const own = modelFromName(item.name)
  return { model: own || 'unknown', printer: own || 'unknown', nozzle: '?' }
}

/* 端点拿到的原始形态 → 一份清单。
 * key 是这一项的稳定标识（同名预设可能出现在多个账号目录下，所以带 uid 与文件名）。
 * `extra` 是导入进来的那几份：它们不在任何目录里，内容直接带在身上。 */
export function normalizeList(
  { user = [], system = [] }: BbsListRaw = {},
  extra: BbsPresetItem[] = [],
): BbsPresetItem[] {
  const seed = [
    ...user.map((e) => ({ entry: e, scope: 'user' as const })),
    ...system.map((e) => ({ entry: e, scope: 'system' as const })),
  ]

  const raw: BbsPresetItem[] = [
    ...extra,
    ...seed.map(({ entry, scope }: { entry: BbsListEntry; scope: 'user' | 'system' }) => ({
      key: `${scope}:${entry.uid ?? ''}:${entry.file}`,
      name: entry.name || String(entry.file || '').replace(/\.json$/i, ''),
      scope,
      uid: entry.uid ?? null,
      file: entry.file,
      inherits: entry.inherits || null,
      instantiation: entry.instantiation ?? null,
      compatible_printers: entry.compatible_printers ?? null,
      broken: entry.broken || null,
      mtime: entry.mtime ?? 0,
      /* 下面四个由 resolveTarget 现填，先占位 */
      model: 'unknown',
      printer: 'unknown',
      nozzle: '?',
      target: 'unknown|?',
      isAbstract: false,
      selectable: true,
      reason: null,
    })),
  ]

  const byName = new Map<string, BbsPresetItem>()
  for (const it of raw) if (!byName.has(it.name)) byName.set(it.name, it)

  for (const it of raw) {
    const t = resolveTarget(it, byName)
    it.model = t.model                      // 名字里的简称（A1M），只用于显示
    it.printer = t.printer                  // compatible_printers 里的全名（Bambu Lab A1 mini）
    it.nozzle = t.nozzle                    // '0.4' / '?'
    it.target = `${t.printer}|${t.nozzle}`  // 筛选用的键
    /* instantiation:"false" 的是继承链中间层（fdm_process_*），BBS 下拉里也不列它们 */
    it.isAbstract = String(it.instantiation) === 'false'
    it.selectable = !it.broken && !it.isAbstract
    it.reason = it.broken ? `文件解析失败：${it.broken}` : (it.isAbstract ? '继承链中间层' : null)
  }
  return raw
}

/* 同名冲突有两种：同一账号下两个文件的 name 字段相同（`MKPProcess A1 mini.json` 与
 * `MKPProcess A1 mini 0.4 0.20.json` 都叫 MKPProcess A1 mini），以及同名预设分布在多个
 * 账号目录里。两种都要能在下拉里分清点的是哪一份，所以后缀用文件名（必要时再带 uid）。 */
export function labelOf(item: BbsPresetItem, items: BbsPresetItem[]): string {
  const same = items.filter((x) => x.name === item.name && x.scope === item.scope)
  if (same.length <= 1) return item.name
  const stem = String(item.file || '').replace(/\.json$/i, '')
  /* 文件名通常是「预设名 + 一点后缀」，只把多出来那截拿来区分，别把整个名字重复一遍 */
  const tail = stem.startsWith(item.name) ? stem.slice(item.name.length).trim() : stem
  const uids = new Set(same.map((x) => x.uid))
  const parts: string[] = []
  if (tail) parts.push(tail)
  if (uids.size > 1 && item.uid) parts.push(item.uid)
  return parts.length ? `${item.name}（${parts.join(' · ')}）` : item.name
}

export function modelsOf(items: BbsPresetItem[]): { model: string; count: number }[] {
  const count = new Map<string, number>()
  for (const it of items) {
    if (it.isAbstract) continue
    count.set(it.model, (count.get(it.model) || 0) + 1)
  }
  return [...count.entries()]
    .map(([model, n]) => ({ model, count: n }))
    .sort((a, b) => (b.count - a.count) || a.model.localeCompare(b.model))
}

/* 筛选档位：三层。`all` / `打印机|*`（该机型全部喷嘴）/ `打印机|0.4`（某个喷嘴）
 * 之所以要分喷嘴：BBS 的下拉只列当前喷嘴那一档（A1 mini 的 29 份系统预设里 0.4 只有 10 份），
 * 不分的话你会在列表里看到一堆装不上的预设。 */
export function targetsOf(items: BbsPresetItem[]): BbsTargetOption[] {
  const byPrinter = new Map<string, Map<string, number>>()
  for (const it of items) {
    if (it.isAbstract) continue
    if (!byPrinter.has(it.printer)) byPrinter.set(it.printer, new Map())
    const noz = byPrinter.get(it.printer)!
    noz.set(it.nozzle, (noz.get(it.nozzle) || 0) + 1)
  }
  const out: BbsTargetOption[] = []
  const printers = [...byPrinter.entries()]
    .map(([printer, noz]) => ({ printer, noz, total: [...noz.values()].reduce((a, b) => a + b, 0) }))
    .sort((a, b) => (b.total - a.total) || a.printer.localeCompare(b.printer))
  for (const p of printers) {
    out.push({ key: `${p.printer}|*`, label: `${p.printer}（全部喷嘴）`, count: p.total, level: 'printer' })
    for (const [nozzle, count] of [...p.noz.entries()].sort((a, b) => Number(a[0]) - Number(b[0]))) {
      out.push({ key: `${p.printer}|${nozzle}`, label: `${p.printer} · ${nozzle}`, count, level: 'nozzle' })
    }
  }
  return out
}

export interface GroupsOptions {
  target?: string
  currentKey?: string | null
  dirty?: boolean
  /** 抽屉里那个输入框：只过滤预设名，不过滤参数 */
  query?: string
}

/* 分组：
 *   target 给具体档位 → 系统组只留该档（两百多份全铺出来没法看）；
 *                       别的档位的用户预设进「其他机型」组，弱化但仍可点
 *   target = 'all'    → 全留，其他组里只剩坏文件
 */
export function groupsFor(
  items: BbsPresetItem[],
  { target = MODEL_ALL, currentKey = null, dirty = false, query = '' }: GroupsOptions = {},
): BbsMenuGroup[] {
  const byName = (a: BbsMenuItem, b: BbsMenuItem) => a.label.localeCompare(b.label, 'zh-Hans-CN')
  const live = items.filter((it) => !it.isAbstract)
  const wanted = (it: BbsPresetItem) => {
    if (target === MODEL_ALL) return true
    const [printer, nozzle] = String(target).split('|')
    return it.printer === printer && (nozzle === '*' || it.nozzle === nozzle)
  }
  /* 搜索只认名字与文件名。**当前选中的那一项永远留着** —— 搜索把它过滤掉的话，
     列表里就没有任何一项是高亮的，人会以为自己看的是另一份预设。 */
  const q = query.trim().toLowerCase()
  const hitQuery = (it: BbsPresetItem) =>
    !q || it.key === currentKey
    || it.name.toLowerCase().includes(q)
    || String(it.file || '').toLowerCase().includes(q)

  const wrap = (it: BbsPresetItem): BbsMenuItem => ({
    key: it.key,
    label: labelOf(it, items),
    scope: it.scope,
    model: it.model,
    printer: it.printer,
    nozzle: it.nozzle,
    target: it.target,
    file: it.file,
    uid: it.uid,
    current: it.key === currentKey,
    /* BBS 里改动未保存的预设，名字前面带 * */
    star: it.key === currentKey && dirty,
    /* 坏文件才真的点不动；只是「不属于当前档位」的仍可点（点了就切过去），
       但要显示得弱一点 —— 不然你会以为它坏了。 */
    disabled: !it.selectable,
    offModel: it.selectable && !wanted(it),
    reason: it.reason || (wanted(it) ? null : `属于 ${it.printer} · ${it.nozzle}`),
  })

  const pool = live.filter(hitQuery)
  /* 导入的那几份不参与机型筛选：你刚拖进来的文件，无论属于哪台机都得看得见 */
  const importedItems = pool.filter((it) => it.scope === 'imported').map(wrap).sort(byName)
  const userItems = pool.filter((it) => it.scope === 'user' && it.selectable && wanted(it)).map(wrap).sort(byName)
  const systemItems = pool.filter((it) => it.scope === 'system' && it.selectable && wanted(it)).map(wrap).sort(byName)
  /* 「其他 / 不支持」只收两种：坏文件，以及**你自己的**别的机型的预设（那些你会想看见）。
     别的机型的系统预设直接不列 —— BBS 里也只加载当前机型那一批，全塞进来会是两百多条噪音。 */
  const restItems = pool
    .filter((it) => it.scope !== 'imported' && (!it.selectable || (it.scope === 'user' && !wanted(it))))
    .map(wrap)
    .sort(byName)

  const groups: BbsMenuGroup[] = []
  if (importedItems.length) groups.push({ title: GROUP_IMPORTED, items: importedItems })
  if (userItems.length) groups.push({ title: GROUP_USER, items: userItems })
  if (systemItems.length) groups.push({ title: GROUP_SYSTEM, items: systemItems })
  if (restItems.length) groups.push({ title: GROUP_OTHER, items: restItems, weak: true })
  return groups
}
