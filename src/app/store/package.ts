/*
 * 客户端的**同步**这一层
 *
 * 作者把现实世界的关系定死了：
 *
 *   `ClientDataPackage`（JSON）**不是用户要下载的东西** —— 客户端为了知道「有哪些版本 /
 *   参数怎么画 / 参数是什么类型 / 哪些条件下显示 / 当前发布值是什么 / 哪个版本可用」
 *   而**自动同步**下来的说明书。
 *
 *   **TOML 才是 preset artifact** —— 用户真正「获取 / 安装 / 使用」的是它。
 *
 * 所以这里有三件**互不混同**的事：
 *
 *   说明书（`STORAGE.clientPackage`）  进客户端 / 刷新时自动同步；界面只说「上次同步 / 已是最新」
 *   本机预设（`STORAGE.clientPresets`）用户点「获取预设」才拿到（云端那份 release 里的 TOML）
 *   当前使用（`STORAGE.clientActive`） 用户点「使用这一份」才切过去
 *
 * 于是下面这种状态完全合理，不是异常：
 *
 *   说明书 1.0.1 · 本机预设 1.0.1 · 使用中 1.0.0     ← 下载了，用户还没切
 *   说明书 1.0.1 · 本机预设 1.0.0 · 使用中 1.0.0     ← 说明书自动更新了，预设还没获取
 */

import type { ClientDataPackage, ReleasePreset } from '../../api'
import { STORAGE } from '../../api/storageKeys'

/** 云端的一次发布（与工作台 `src/workbench/cloud.ts` 同一个形状、同一格云端） */
export interface CloudEntry {
  id: string
  name: string
  version: string | null
  at: string | null
  from: 'static' | 'local'
  package: ClientDataPackage
  /** 这次发布的预设文件（TOML）。老的静态快照可能没有这一栏 —— 当空数组读 */
  presets: ReleasePreset[]
}

/*
 * 四格的键名都收在 `src/api/storageKeys.ts`：作者还没定这套名字，集中一处，
 * 将来改名是一处的事。值照旧 —— 与试验场的键同名，行为才对得上。
 */

const STATIC_URL = '/cloud/presets.json'

/* ---------- 云端 ---------- */

interface StaticFile {
  entries?: {
    id: string
    name: string
    version?: string | null
    at?: string | null
    package: ClientDataPackage
    presets?: ReleasePreset[]
  }[]
}

let staticCache: CloudEntry[] | null = null

export async function staticCloud(): Promise<CloudEntry[]> {
  if (staticCache !== null) return staticCache
  try {
    const res = await fetch(STATIC_URL)
    if (!res.ok) throw new Error(String(res.status))
    const file = (await res.json()) as StaticFile
    staticCache = (file.entries ?? []).map((e) => ({
      id: e.id,
      name: e.name,
      version: e.version ?? null,
      at: e.at ?? null,
      from: 'static' as const,
      package: e.package,
      presets: e.presets ?? [],
    }))
  } catch {
    staticCache = []
  }
  return staticCache
}

function localCloud(): CloudEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE.cloud)
    if (!raw) return []
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter((e): e is CloudEntry => {
        const o = e as Partial<CloudEntry>
        return typeof o?.id === 'string' && typeof o?.name === 'string' && o?.package !== undefined
      })
      .map((e) => ({ ...e, from: 'local' as const, presets: e.presets ?? [] }))
  } catch {
    return []
  }
}

/** 云端全部：静态在前、工作台刚上传的在后（**最后一条是最新的发布**） */
export async function listCloud(): Promise<CloudEntry[]> {
  return [...(await staticCloud()), ...localCloud()]
}

/** 云端最新的那一次发布 */
export function latestOf(cloud: CloudEntry[]): CloudEntry | null {
  return cloud.length === 0 ? null : cloud[cloud.length - 1]
}

/* ---------- ① 说明书（自动同步） ---------- */

export interface SyncedPackage {
  entryId: string
  name: string
  version: string | null
  at: string | null
  /** 本机同步下来的时刻 */
  syncedAt: string
  package: ClientDataPackage
}

/**
 * 参数页那一侧的叫法：它要的就是「本机这份说明书」。名字留着不改，
 * 免得为了一个词去动那个大文件 —— 同一个东西，这里只有一个出处（`synced`）。
 */
export const downloaded = synced
export type Downloaded = SyncedPackage

export function synced(): SyncedPackage | null {
  try {
    const raw = localStorage.getItem(STORAGE.clientPackage)
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<SyncedPackage>
    if (parsed?.package === undefined) return null
    return parsed as SyncedPackage
  } catch {
    return null
  }
}

function writeSynced(e: CloudEntry): SyncedPackage {
  const next: SyncedPackage = {
    entryId: e.id,
    name: e.name,
    version: e.version,
    at: e.at,
    syncedAt: stamp(),
    package: e.package,
  }
  try {
    localStorage.setItem(STORAGE.clientPackage, JSON.stringify(next))
  } catch {
    /* 配额满 / 隐私模式：这一份留在内存里也能看，不算错误 */
  }
  return next
}

export type SyncStatus = 'synced' | 'upToDate' | 'empty'

export interface SyncResult {
  status: SyncStatus
  /** 同步后的本机说明书（没同步就还是原来那一份） */
  local: SyncedPackage | null
  /** 云端最新那一次 */
  latest: CloudEntry | null
  cloudCount: number
}

/**
 * **自动同步说明书**。
 *
 * 真实客户端不会让用户去点「下载 JSON」—— 进客户端 / 刷新时它自己看一眼云端发布记录，
 * 指纹不一样就把说明书换掉。这里照这个来：
 *
 *   · 云端空的 → `empty`（界面上说「云端暂时没有东西」）
 *   · 本机没有 / 指纹与云端最新那份不同 → **换掉本机说明书**，报 `synced`
 *   · 一样 → `upToDate`（一个字都不动）
 *
 * 判据是**内容指纹**（`inputsHash`），不是版本号 —— 版本号是人填的，可能忘了改。
 */
export async function autoSync(): Promise<SyncResult> {
  const cloud = await listCloud()
  const latest = latestOf(cloud)
  const local = synced()
  if (latest === null) return { status: 'empty', local, latest: null, cloudCount: 0 }
  if (local !== null && local.package.inputsHash === latest.package.inputsHash) {
    return { status: 'upToDate', local, latest, cloudCount: cloud.length }
  }
  return { status: 'synced', local: writeSynced(latest), latest, cloudCount: cloud.length }
}

/* ---------- ② 本机预设（用户「获取预设」才拿到） ---------- */

export interface LocalPreset {
  uid: string
  machineId: string
  versionId: string
  fileName: string
  /** 它是从哪一次发布拿下来的 */
  version: string | null
  /** TOML 正文 */
  content: string
  at: string
}

export type PresetMap = Record<string, LocalPreset>

export function presets(): PresetMap {
  try {
    const raw = localStorage.getItem(STORAGE.clientPresets)
    if (!raw) return {}
    const parsed = JSON.parse(raw) as unknown
    return parsed !== null && typeof parsed === 'object' ? (parsed as PresetMap) : {}
  } catch {
    return {}
  }
}

function writePresets(next: PresetMap) {
  try {
    localStorage.setItem(STORAGE.clientPresets, JSON.stringify(next))
  } catch {
    /* 同上 */
  }
}

/** 云端这一份发布里，这个版本对应的预设文件（没有就是那份 release 里没打包它） */
export function presetInCloud(e: CloudEntry, uid: string): ReleasePreset | null {
  const [mid, vid] = uid.split('/')
  return e.presets.find((p) => p.machineId === mid && p.versionId === vid) ?? null
}

/**
 * **获取预设**：云端那份 TOML → 本机 preset 目录。
 *
 * 这是**用户意义上真正的「下载」** —— 它和说明书自动同步是两件事。
 * 返回 null = 云端这份 release 里没有这一份（没生成过就发布的话就会这样，界面上说清）。
 */
export function fetchPreset(e: CloudEntry, uid: string): LocalPreset | null {
  const p = presetInCloud(e, uid)
  if (p === null) return null
  const next: LocalPreset = {
    uid,
    machineId: p.machineId,
    versionId: p.versionId,
    fileName: p.fileName,
    version: e.version,
    content: p.content,
    at: stamp(),
  }
  writePresets({ ...presets(), [uid]: next })
  return next
}

/* ---------- ③ 当前使用（唯一底账） ---------- */

/**
 * 「当前使用的那一份」—— **全表唯一的底账**。
 *
 * 原来这里与假后端的 `api.getAppliedPreset()` 并存两套账（api 内存 / 本格 localStorage），
 * 一页上可能同时亮两行「已应用」。作者把话说死了（「始终是有一个已应用的」），所以：
 *
 *   · 这个键存**单个对象** —— 「唯一」由形状本身保证（写就是覆盖）
 *   · 界面（预设页 / 首页）读的、写的**只有它**；假后端那套 `applyPreset` /
 *     `getAppliedPreset` 退位成它自己的演示状态（真后端接上时再回来接）
 *   · `kind` 区分是哪一类行被应用：official 是官方仓库的文件，release 是工作台发布的
 *     那一份（uid 形如 `A1/STANDARD`）—— 同一份文件两种来源，只有被应用的那种才亮
 */
export interface ActiveEntry {
  kind: 'official' | 'release'
  /** official = 仓库 asset id；release = uid（`A1/STANDARD`） */
  ref: string
  /** 这份预设对应的机型 / 版本 —— 首页反填与状态条文案用它；不知道时为 null */
  machineId: string | null
  versionId: string | null
  /** release 条目：它属于哪次发布（包版本）；official 条目为 null */
  version: string | null
}

/**
 * 读唯一底账。**兼容早期写下的旧值**（`Record<uid,{version}>` 的 map 形状）——
 * 取第一条键当 release 条目读，作者浏览器里那条还会正常显示成「已应用 A1.toml · A1」。
 */
export function activeEntry(): ActiveEntry | null {
  try {
    const raw = localStorage.getItem(STORAGE.clientActive)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (parsed === null || typeof parsed !== 'object') return null
    const o = parsed as Record<string, unknown>
    /* 新形状：单条目（有 ref 字段） */
    if (typeof o.ref === 'string') {
      return {
        kind: o.kind === 'official' ? 'official' : 'release',
        ref: o.ref,
        machineId: typeof o.machineId === 'string' ? o.machineId : null,
        versionId: typeof o.versionId === 'string' ? o.versionId : null,
        version: typeof o.version === 'string' ? o.version : null,
      }
    }
    /* 旧形状：map —— 取第一条键（它一定是 uid）当 release 条目 */
    const first = Object.keys(o)[0]
    if (first === undefined) return null
    const [machineId, versionId] = first.split('/')
    const v = o[first] as { version?: unknown } | undefined
    return {
      kind: 'release',
      ref: first,
      machineId: machineId !== undefined && machineId !== '' ? machineId : null,
      versionId: versionId !== undefined && versionId !== '' ? versionId : null,
      version: typeof v?.version === 'string' ? v.version : null,
    }
  } catch {
    return null
  }
}

/**
 * 写唯一底账（覆盖 —— 这就是「应用」这个动作的落点）。
 *
 * release 条目守旧规矩：本机没有这一份就不动（`STORAGE.clientPresets` 是它的前提）。
 * official 条目直接写（它本来就来自表上「本机有」的那些行）。
 */
export function activatePreset(entry: ActiveEntry): void {
  if (entry.kind === 'release' && presets()[entry.ref] === undefined) return
  try {
    localStorage.setItem(STORAGE.clientActive, JSON.stringify(entry))
  } catch {
    /* 写不进去就只在这次会话里有效 —— 与另一格同一个态度 */
  }
}

/** 一句话摘要（列表里显示用） */
export function pkgSummary(pkg: ClientDataPackage): string {
  const versions = pkg.machines.reduce((n, m) => n + m.versions.length, 0)
  const conds = pkg.fields.filter((f) => f.showWhen !== undefined).length
  return `${pkg.machines.length} 台机型 · ${versions} 个版本 · 说明书 ${pkg.fields.length} 个字段（带条件 ${conds}）· 结构 ${pkg.meta.schemaVersion}`
}

/**
 * 本机时刻（说明书同步 / 预设下载）—— 落盘的统一写法。
 *
 * 原来是 `toLocaleString('zh-CN').slice(5,16)` → `9/30 16:12`（斜杠、没有年份、
 * 不补零），与别处的时间对不上（作者：「你那个时间格式能不能统一一下」）。
 * 现在落 **ISO**；显示由 `shortStatText` / `longStatText` 统一格式化，
 * 老值（斜杠那种）仍能被它们读出来。
 */
function stamp(): string {
  return new Date().toISOString()
}

/**
 * 时间那一格的解析：三种来源的写法都在这里认。
 *
 *   官方行     `2026-08-26`（假后端按路径推的演示值，只有日期）
 *   发布行     ISO（云端 = 发布时刻；本机 = 下载时刻）
 *   老的本机值 `9/30 16:12`（老版 stamp 切掉了年份 —— 凑不出就不给年，不编）
 */
interface StatDate {
  y: string | null
  mo: string
  d: string
  hh: string | null
  mi: string | null
}
const pad2 = (s: string) => s.padStart(2, '0')

function parseStatDate(raw: string): StatDate | null {
  /* ISO（发布 / 下载时刻）：按**本地时区**取 —— 直接切字符串在跨日时会差一天 */
  if (/^\d{4}-\d{2}-\d{2}T/.test(raw)) {
    const t = new Date(raw)
    if (!Number.isNaN(t.getTime())) {
      return {
        y: String(t.getFullYear()),
        mo: pad2(String(t.getMonth() + 1)),
        d: pad2(String(t.getDate())),
        hh: pad2(String(t.getHours())),
        mi: pad2(String(t.getMinutes())),
      }
    }
  }
  /* 纯日期（演示值） */
  let m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(raw)
  if (m !== null) return { y: m[1], mo: m[2], d: m[3], hh: null, mi: null }
  /* 老的本机下载时刻 `9/30 16:12` */
  m = /^(\d{1,2})\/(\d{1,2})\s+(\d{1,2}):(\d{2})/.exec(raw)
  if (m !== null) return { y: null, mo: pad2(m[1]), d: pad2(m[2]), hh: pad2(m[3]), mi: m[4] }
  return null
}

/** 行上那一列：只写月-日，全表统一 `MM-DD`（补零、横杠）——不管官方 / 我的 / 发布 */
export function shortStatText(raw: string | undefined): string | undefined {
  if (raw === undefined) return undefined
  const t = parseStatDate(raw)
  return t === null ? raw : `${t.mo}-${t.d}`
}

/** 展开面板 / 同步页：带年份；有时刻就缀 `HH:mm`（老值没有年份就只给月-日，不编） */
export function longStatText(raw: string | undefined): string | undefined {
  if (raw === undefined) return undefined
  const t = parseStatDate(raw)
  if (t === null) return raw
  const date = t.y === null ? `${t.mo}-${t.d}` : `${t.y}-${t.mo}-${t.d}`
  return t.hh === null ? date : `${date} ${t.hh}:${t.mi}`
}
