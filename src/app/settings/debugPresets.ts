/*
 * 调试参数的「存档」（2026-11-11 作者）。
 *
 * 「我点一下保存，它就记一条，这样子就方便我用不同的。」
 *
 * ★ 为什么住 localStorage 而不是偏好文件：调试参数是**临时状态**，
 *   按下「完成」之后它就不该再影响产物（`debug_mode_enabled` 关掉就整段不生效）。
 *   存进偏好文件反而会让「我上次试的那组数」变成长期生效的东西。
 *
 * ★ 上限 30 条（2026-11-11 作者：「记多一点比较好」）：草稿纸，超过就丢最老的。
 */

export type DebugPreset = {
  /** 存档 id（时间戳 + 随机尾巴，避免同一毫秒连点撞车） */
  id: string
  /** 存下来的时刻（ms） */
  at: number
  waitX: number
  waitY: number
  /** `null` = 跟随驻点位 */
  mountWaitX: number | null
  /** `null` = 跟随驻点位 */
  mountWaitY: number | null
  mountX: number
  /** `null` = 跟随落笔等待位 */
  mountY: number | null
  /** `null` = 跟随驻点位 */
  unmountWaitX: number | null
  /** `null` = 跟随驻点位 */
  unmountWaitY: number | null
  unmountX: number
  /** `null` = 跟随收笔等待位 */
  unmountY: number | null
  speed: number
  /** 动作到位后停几秒；`null` = 不停 */
  dwellSec: number | null
  moveY: boolean
}

const KEY = 'mkp:debug-presets:v1'
const MAX = 30

/** 读全部存档，新的在前。坏数据（手改过 localStorage）一律当空 */
export function loadPresets(): DebugPreset[] {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw === null) return []
    const list: unknown = JSON.parse(raw)
    if (!Array.isArray(list)) return []
    return list
      .flatMap((v) => {
        const p = normalize(v)
        return p === null ? [] : [p]
      })
      .slice(0, MAX)
  } catch (err) {
    console.error('[debugPresets] 读存档失败', err)
    return []
  }
}

/** `undefined` / `null` 都是"跟随"；别的形状记成 `undefined`（这一条作废） */
function optNum(v: unknown): number | null | undefined {
  if (v === undefined || v === null) return null
  return typeof v === 'number' ? v : undefined
}

/**
 * 一条存档归一成当前形状。
 *
 * 等待位那几格是后加的：早先存的记录里没有它们，读到就当「跟随驻点位」——
 * 不能因为多了四个键把作者手上的记录整批作废。
 */
function normalize(v: unknown): DebugPreset | null {
  if (typeof v !== 'object' || v === null) return null
  const p = v as Record<string, unknown>
  if (
    typeof p.id !== 'string' ||
    typeof p.at !== 'number' ||
    typeof p.waitX !== 'number' ||
    typeof p.waitY !== 'number' ||
    typeof p.mountX !== 'number' ||
    typeof p.unmountX !== 'number' ||
    typeof p.speed !== 'number' ||
    typeof p.moveY !== 'boolean'
  ) {
    return null
  }
  const mountWaitX = optNum(p.mountWaitX)
  const mountWaitY = optNum(p.mountWaitY)
  const unmountWaitX = optNum(p.unmountWaitX)
  const unmountWaitY = optNum(p.unmountWaitY)
  const mountY = optNum(p.mountY)
  const unmountY = optNum(p.unmountY)
  /* 等待秒数是后加的：早先的存档没有这个键 ⇒ 当「不停」，不整条作废 */
  const dwellSec = optNum(p.dwellSec)
  if (
    mountWaitX === undefined ||
    mountWaitY === undefined ||
    unmountWaitX === undefined ||
    unmountWaitY === undefined ||
    mountY === undefined ||
    unmountY === undefined ||
    dwellSec === undefined
  ) {
    return null
  }
  return {
    id: p.id,
    at: p.at,
    waitX: p.waitX,
    waitY: p.waitY,
    mountWaitX,
    mountWaitY,
    mountX: p.mountX,
    mountY,
    unmountWaitX,
    unmountWaitY,
    unmountX: p.unmountX,
    unmountY,
    speed: p.speed,
    dwellSec,
    moveY: p.moveY,
  }
}

function write(list: DebugPreset[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(list.slice(0, MAX)))
  } catch (err) {
    /* 配额满了 / 隐私模式：记一条就好，不打断用户手上的事 */
    console.error('[debugPresets] 写存档失败', err)
  }
}

/** 存一条（返回新列表，新的在前） */
export function savePreset(p: Omit<DebugPreset, 'id' | 'at'>): DebugPreset[] {
  const entry: DebugPreset = {
    ...p,
    id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`,
    at: Date.now(),
  }
  const list = [entry, ...loadPresets()]
  write(list)
  return list.slice(0, MAX)
}

/** 删一条 */
export function removePreset(id: string): DebugPreset[] {
  const list = loadPresets().filter((p) => p.id !== id)
  write(list)
  return list
}

/** 清空全部 */
export function clearPresets(): DebugPreset[] {
  try {
    localStorage.removeItem(KEY)
  } catch (err) {
    console.error('[debugPresets] 清空失败', err)
  }
  return []
}

/** 一行摘要：「驻点 20 / 267 · 10000」，列表里那一行用它 */
export function summarize(p: DebugPreset): string {
  const y = p.moveY ? `,${fmt(p.waitY)}` : ''
  return `${fmt(p.waitX)}${y} → ${fmt(p.mountX)} → ${fmt(p.unmountX)} · ${fmt(p.speed)}`
}

/** 与设置页数字框同一口径：整数不带小数点 */
export function fmt(v: number): string {
  return String(Number(v.toFixed(3)))
}
