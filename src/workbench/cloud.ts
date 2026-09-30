/*
 * 工作台 · **模拟云端**（自试验场 C15 的 `store/cloud.ts` 移植）
 *
 * 真实的线是一条管道，不是两个人读同一个文件（作者原话）：
 *
 *   工作台（TOML 编辑）→ 生成 JSON → P 文件夹 → 同步 → 云端 preset 文件夹 → 客户端下载
 *
 * 这份文件负责最后那两格：云端上现在有哪些包、上传一份上去。它只有两个来源：
 *
 *   静态快照  `public/cloud/presets.json` —— 等于「云端已经有别人发过的几份」，只读。
 *             **改判后已随仓分发**（债 #6：作者要「静态快照 ＋ 我刚上传的」两条都要，
 *             且该快照与我们的假后端同源同一批上游 JSON）。读不到一样退空 ——
 *             代码本来就是这么写的（`catch` 退空数组，不编数据）。
 *   我刚上传的 `localStorage[STORAGE.cloud]` —— 演示用。真后端是写文件 + 推云端，
 *             这里换成 localStorage，**同一个键让客户端也读得到** —— 联动就靠这一格。
 *             上传的是**整个 release**（说明书 + N 份 TOML），由 `clientPackage.ts` 现装，
 *             入口是 `BuildPage` 的「上传到云端」。
 *
 * 两件事刻意分开：
 *
 *   · **包体（`ClientDataPackage`）本身不带版本号与时间** —— 真契约里它只有 `inputsHash`，
 *     刻意的：假后端没有可信的时间源。所以「这是第几版、什么时候发的」记在**云端目录项**上
 *     （谁上传谁记），不往包里塞。
 *   · 键名集中在 `src/api/storageKeys.ts`，且**不带端名**：这不是工作台或客户端的偏好，
 *     是**那条管道**本身（工作台上传、客户端下载的是同一个地方）。带上端名就变成
 *     「只有这一端看得见」，联动就断了。
 */

import { STORAGE } from '../api/storageKeys'
import type { ClientDataPackage, ReleasePreset } from '../api/contract'

/**
 * 云端目录里的一项 = **一次发布**。
 *
 * 两样东西同属一个 preset identity，一起放在这儿：
 *
 *   `package`  说明书（`ClientDataPackage`）—— 客户端**自动同步**它，用户看不见这个动作
 *   `presets`  真正的预设文件（TOML）—— 客户端要**用户点「获取预设」**才拿得到
 *
 * 所以列表里那「一份」不是「一个 JSON 文件」，是「一次发布」。
 */
export interface CloudEntry {
  id: string
  /** 名字（给人看的，比如「官方预设」/「A1 标准版 1.0.1」） */
  name: string
  /** 包版本。静态快照里有；本地那份由上传时填 */
  version: string | null
  /** 放上云端的时刻（静态快照写死的文本 / 本地那份由上传时记） */
  at: string | null
  /** 来源：静态快照 / 我刚上传的 */
  from: 'static' | 'local'
  package: ClientDataPackage
  /** 这次发布的预设文件（TOML）。老的静态快照里可能没有这一栏 —— 当空数组读 */
  presets: ReleasePreset[]
}

/** 静态云端（只读）：`public/cloud/presets.json` 里的一组快照 */
const STATIC_URL = '/cloud/presets.json'

interface StaticFile {
  note?: string
  entries?: {
    id: string
    name: string
    version?: string | null
    at?: string | null
    package: ClientDataPackage
    /** T7.2 起带上；早先落的那一份没有这一栏，当空数组读 */
    presets?: ReleasePreset[]
  }[]
}

let staticCache: CloudEntry[] | null = null

/** 读静态云端。读不到（产品仓没搬这份文件）就当云端空的 —— 不编数据 */
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

/** 读我上传的那几份（坏了当没有 —— 解析不出 JSON 就是脏数据） */
export function localCloud(): CloudEntry[] {
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
      .map((e) => ({ ...e, from: 'local' as const }))
  } catch {
    return []
  }
}

/** 云端全部：静态在前，我刚上传的在后（列表里看得出哪个是新的） */
export async function listCloud(): Promise<CloudEntry[]> {
  return [...(await staticCloud()), ...localCloud()]
}

/** 上传一份到云端（本地这份）。同 id 覆盖 */
export function uploadToCloud(entry: Omit<CloudEntry, 'from'>): CloudEntry {
  const next: CloudEntry = { ...entry, from: 'local' }
  const rest = localCloud().filter((e) => e.id !== entry.id)
  try {
    localStorage.setItem(STORAGE.cloud, JSON.stringify([...rest, next]))
  } catch {
    /* 配额满 / 隐私模式：演示用的云端写不进去不算错误，界面照旧显示这份 */
  }
  return next
}

/** 从云端删一份（只删本地那份；静态快照删不动） */
export function removeFromCloud(id: string): void {
  try {
    localStorage.setItem(STORAGE.cloud, JSON.stringify(localCloud().filter((e) => e.id !== id)))
  } catch {
    /* 同上 */
  }
}

/** 一条目录项的一句话摘要（列表里显示用，不再各处拼） */
export function cloudSummary(e: CloudEntry): string {
  const versions = e.package.machines.reduce((n, m) => n + m.versions.length, 0)
  const conds = e.package.fields.filter((f) => f.showWhen !== undefined).length
  /* 两样产物分开数：说明书是按字段算的，预设文件是按份数的 */
  return `${e.package.machines.length} 台机型 · ${versions} 个版本 · 说明书 ${e.package.fields.length} 个字段（带条件 ${conds}）· 预设文件 ${e.presets.length} 份 · 结构 ${e.package.meta.schemaVersion}`
}
