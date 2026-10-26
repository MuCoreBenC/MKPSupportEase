/*
 * 预设清单的归一与分组 —— 译自 machine-motion/src/bbs/preset-source.js，逻辑一行没改。
 * 纯函数：不碰 DOM、不发请求。
 *
 * 本机那一批只有一处来源：serve 期端点 `/api/bbs/presets`（读你机器上 BBS 的目录，实时）。
 * 本仓不打包那 285 个预设快照（作者裁决 §6-2），所以没有「退到仓库快照」这一档 ——
 * 读不到本机目录就是空态，由页面照实说。
 *
 * ★ 2026-10-13 起清单里还多了一路：**我们自己的切片器配置**（预设页下载到本机的那几份
 *   `MKPProcess …json`，`scope = 'mkp'`）。它不是"本机 BBS 目录"里的东西，是交付面给的，
 *   由 `PageBbs` 读出来（见那儿读盘那一段）并进这份清单 —— 于是继承链、基准、改动判定全走
 *   同一条路（那几份的 `inherits` 指的就是 BBS 的系统预设）。
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

import type { StringKey, Tr } from '../../i18n'
import type {
  BbsListEntry,
  BbsListRaw,
  BbsMenuGroup,
  BbsMenuItem,
  BbsPresetItem,
  BbsTargetOption,
  BbsValue,
} from './bbsTypes'

/**
 * 「这一份为什么灰着」的那句话。
 *
 * 数据层只留**事实与键**（`reasonKey` + `reasonArgs`），取字在这里 —— 模块级函数
 * 拿不到 hook，也不该把某一刻的语言冻进数据里。
 */
export function reasonText(
  it: { reason?: string | null; reasonKey?: StringKey; reasonArgs?: Record<string, string> },
  tr: Tr,
): string {
  if (it.reasonKey !== undefined) return tr(it.reasonKey, it.reasonArgs)
  return it.reason ?? ''
}

export const GROUP_IMPORTED: StringKey = 'bbs.group.imported'
export const GROUP_USER: StringKey = 'bbs.group.user'
export const GROUP_SYSTEM: StringKey = 'bbs.group.system'
/* BBS 那一组叫「不支持的配置」（.po 里 Unsupported presets）。这一页语义稍宽：
   坏文件 + 别的机型的用户预设都落这里，所以叫「其他机型 / 读不了的」更准。 */
export const GROUP_OTHER: StringKey = 'bbs.group.other'
/* 我们自己的那一组（2026-10-13）：预设页「切片器配置」下载到本机的那几份
   `MKPProcess …json`。它排在最前 —— 这一页默认先给你看我们发的东西。
   组名与抽屉那颗胶囊（「MKP 工艺」）用同一个词：作者定名时特意避开"预设"二字
   （会跟 MKP 自己那份 toml 预设混起来），也避开"BBS"（内部黑话）。 */
export const GROUP_MKP: StringKey = 'bbs.group.mkp'
export const MODEL_ALL = 'all'

/**
 * catalog 的 `machineId` → BBS 里那台机的**全名**（`compatible_printers` 里的写法）。
 *
 * 只列目录里有 MKP 工艺档的那五台（见 `mockServer/catalogFiles.ts` / 真机
 * `catalog.json` 里那九条 `bbs_config`）。名字必须与 BBS 自己写的**逐字一致**
 * （是 `Bambu Lab X1 Carbon`，不是 `X1C`）—— 本页的机型筛选键就是这串名字，
 * 差一个字，换档之后目标档位就对不上（会被兜底成「全部」）。
 */
export const PRINTER_BY_MACHINE_ID: Record<string, string> = {
  A1: 'Bambu Lab A1',
  A1_MINI: 'Bambu Lab A1 mini',
  P1S: 'Bambu Lab P1S',
  P2S: 'Bambu Lab P2S',
  X1C: 'Bambu Lab X1 Carbon',
}

/**
 * 抽屉看的是哪一边（2026-10-13）。
 *
 *   `mkp`  我们自己的 —— 预设页「切片器配置」下载到本机的那几份 MKPProcess 工艺预设
 *   `bbs`  本机 BBS 目录里的 —— 用户配置 + 系统配置
 *
 * 为什么只有两档（作者定的，不是三档）：本机的"用户/系统"本来就是同一个目录下的两批，
 * 分成两个入口只是把"这一份是本地还是 BBL 的"变成一次点击；真正要紧的分界是
 * 「我们自己发的那几份」与「本机 BBS 里原有的那几百份」。系统那一档因此不是缺了，
 * 是并进了 `bbs`。
 */
export type BbsPickSource = 'mkp' | 'bbs'

/**
 * 这一项属不属于当前那一档。列表过滤用、默认选中也用 —— **只此一处**，
 * 两处各写一份就会出现"列表里没有它、却被选成了当前项"。
 */
export function inPickSource(it: Pick<BbsPresetItem, 'scope'>, source: BbsPickSource): boolean {
  return source === 'mkp'
    ? it.scope === 'mkp'
    : (it.scope === 'user' || it.scope === 'system')
}

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
  /*
   * 该用**它自己**的 compatible_printers，还是顺 inherits 跳到父档去认？
   *
   *   `system` / `mkp` → 用自己。系统预设本来就是最规范的那一层；`mkp` 那份的这一串是
   *      **按 catalog 登记拼出来的**（见 `useBbsData` 的 `slicerItem`），比父档还准 ——
   *      这正是作者 2026-10-13 要的：「不用管它那个继承链……根据那个 catalog 里面的登记
   *      去给它分类」。原来的写法一律先跳父档，于是 A1 mini / P1S 那几份只要这台机器上
   *      没装对应机型的系统预设，机型就显示成 `unknown` —— 明明目录里写着归属机型。
   *
   *   `user` / `imported` → **先跳父档**（下面文件头那段的老口径，别动）：用户预设的名字是
   *      人手起的，那串 compatible_printers 也可能被用户改过；父档两个字段都规范。
   *      父档不在这台机器上时才退回它自己。
   */
  const parent = item.inherits ? byName.get(item.inherits) : undefined
  const useOwn = item.scope === 'system' || item.scope === 'mkp' || parent === undefined
  let cur: BbsPresetItem | undefined = useOwn ? item : parent
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
  { user = [], system = [] }: Partial<BbsListRaw> = {},
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
    /*
     * ★ **还没下载的那几份由 `slicerItem` 标好，这里不许拿通用算式盖掉它**（2026-10-13）：
     *   它的正文不在手上，选了也画不出参数 —— 那一档的默认选中必须跳过它
     *   （`pickInitial` 认的就是这个 `selectable`）。原来这一步是无条件重算的，
     *   于是灰着的那几份照样会被默认选中，右侧只剩出厂默认值。
     */
    it.selectable = it.notDownloaded ? false : (!it.broken && !it.isAbstract)
    /* 事实进 `reason`（内部判定照旧读它），话术进 `reasonKey` —— 见 `reasonText` */
    if (it.notDownloaded) {
      it.reason = '还没下载'
      it.reasonKey = 'bbs.reason.notDownloaded'
    } else if (it.broken) {
      it.reason = `文件解析失败：${it.broken}`
      it.reasonKey = 'bbs.reason.broken'
      it.reasonArgs = { error: it.broken }
    } else if (it.isAbstract) {
      it.reason = '继承链中间层'
      it.reasonKey = 'bbs.reason.abstract'
    } else {
      it.reason = null
    }
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
  /**
   * 看哪一边（2026-10-13）。`mkp` 时池子里只剩我们自己的那几份 —— 顺带解决作者
   * 抱怨的那件事：「这个下拉菜单它就不是说显示这么多的……切到了显示本机，它才显示这么多。
   * 你看这几百个哪里够人选？」。默认 `bbs`，老行为一字不改。
   */
  source?: BbsPickSource
}

/* 分组：
 *   target 给具体档位 → 系统组只留该档（两百多份全铺出来没法看）；
 *                       别的档位的用户预设进「其他机型」组，弱化但仍可点
 *   target = 'all'    → 全留，其他组里只剩坏文件
 *   source 决定池子里有谁（见 `inPickSource`），分组规则本身不变
 */
export function groupsFor(
  items: BbsPresetItem[],
  {
    target = MODEL_ALL,
    currentKey = null,
    dirty = false,
    query = '',
    source = 'bbs',
  }: GroupsOptions = {},
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
    /*
     * 坏文件才真的点不动；只是「不属于当前档位」的仍可点（点了就切过去），
     * 但要显示得弱一点 —— 不然你会以为它坏了。
     * ★ 2026-10-13：**还没下载的那几份不进 disabled** —— 它们可点，
     *   点了是另一种动作（去预设页取回来，见 `notDownloaded`）。
     */
    disabled: !it.selectable && !it.notDownloaded,
    offModel: it.selectable && !wanted(it),
    reason: it.reason || (wanted(it) ? null : `属于 ${it.printer} · ${it.nozzle}`),
    reasonKey: it.reasonKey
      ?? (wanted(it) ? undefined : ('bbs.reason.otherTarget' as StringKey)),
    reasonArgs: it.reasonKey !== undefined
      ? it.reasonArgs
      : wanted(it)
        ? undefined
        : { printer: it.printer, nozzle: it.nozzle },
    notDownloaded: Boolean(it.notDownloaded),
  })

  /* 池子先按这一档收一次：两侧不会同时出现在列表里（那是两个入口） */
  const pool = live.filter(hitQuery).filter((it) => inPickSource(it, source))
  /* 我们自己的那几份（2026-10-13）：筛法与用户预设同一套 —— 当前机型的排在组里，
     别的机型的落进「其他机型」组弱化但可点（那是你会想看见的，不该直接消失）。
     ★ **还没下载的那几份也排在这一组里**（灰着、点了去预设页取）：列出来才知道我们发过它 */
  const mkpItems = pool
    .filter((it) => it.scope === 'mkp' && (it.selectable || it.notDownloaded) && wanted(it))
    .map(wrap)
    .sort(byName)
  /* 导入的那几份不参与机型筛选：你刚拖进来的文件，无论属于哪台机都得看得见 */
  const importedItems = pool.filter((it) => it.scope === 'imported').map(wrap).sort(byName)
  const userItems = pool.filter((it) => it.scope === 'user' && it.selectable && wanted(it)).map(wrap).sort(byName)
  const systemItems = pool.filter((it) => it.scope === 'system' && it.selectable && wanted(it)).map(wrap).sort(byName)
  /* 「其他 / 不支持」只收三种：坏文件、**你自己的**别的机型的预设、以及我们自己的别的机型的
     切片器配置（后两类你会想看见）。别的机型的系统预设直接不列 —— BBS 里也只加载当前机型
     那一批，全塞进来会是两百多条噪音。 */
  /* 已经在上面那两组里排过的不再进来（一组一份）—— 还没下载的我们那几份正落在这一条上 */
  const placed = new Set([...mkpItems, ...importedItems, ...userItems, ...systemItems].map((it) => it.key))
  const restItems = pool
    .filter((it) => it.scope !== 'imported' && !placed.has(it.key)
      && (!it.selectable || ((it.scope === 'user' || it.scope === 'mkp') && !wanted(it))))
    .map(wrap)
    .sort(byName)

  const groups: BbsMenuGroup[] = []
  /* 我们自己那一组排最前 —— 这一页默认先给你看我们发的东西 */
  if (mkpItems.length) groups.push({ titleKey: GROUP_MKP, items: mkpItems })
  if (importedItems.length) groups.push({ titleKey: GROUP_IMPORTED, items: importedItems })
  if (userItems.length) groups.push({ titleKey: GROUP_USER, items: userItems })
  if (systemItems.length) groups.push({ titleKey: GROUP_SYSTEM, items: systemItems })
  if (restItems.length) groups.push({ titleKey: GROUP_OTHER, items: restItems, weak: true })
  return groups
}
