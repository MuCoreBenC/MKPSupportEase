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
  if (
    mountWaitX === undefined ||
    mountWaitY === undefined ||
    unmountWaitX === undefined ||
    unmountWaitY === undefined ||
    mountY === undefined ||
    unmountY === undefined
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

/**
 * 出厂点位参照表（2026-11-11 作者：「显示出它那个默认的那个点位，方便我去改」）。
 *
 * ★ 这是**快照**，不是运行时真相：真正的值住在
 *   `presets/delivery/mkp/presets/<机型>-<档>.toml` 的 `pen_mount_*` / `pen_unmount_*`。
 *   改那几个文件时**顺手同步这里**，否则调试窗会显示一个过时的参照。
 *   （做成一整条 IPC 要新开注册表读口，而这里只需要"看一眼"，先把参照摆出来。）
 *
 * ★ 2026-11-11 更正：落笔 / 收笔**不只是按机型分**，同一个机型的不同版本也不同
 *   （`A1:FASTV3.3` 的收笔位是 273，其余 A1 是 271）。所以这张表按
 *   `param_registry.toml` 里 `pen_mount_*` / `pen_unmount_*` 的 `machineVariants`
 *   **逐版本**列出，不再按机型归并。
 *
 *   开始阶段收笔仍按机型（那是 `hooks.rs` 的常量，不随版本变）。
 */
export type FactoryAnchorRow = {
  /** 版本（`FAST` / `FASTV3.3` …）；空串 = 该机型的裸键（没有版本那一档） */
  version: string
  /** 落笔等待位 X。调试窗只有一个驻点，套用这一行时驻点取它 */
  mountWaitX: number
  /** 落笔位 X */
  mountX: number
  /** 收笔等待位 X（P 系列与落笔等待位不同，调试窗只放得下一个驻点） */
  unmountWaitX: number
  /** 收笔位 X */
  unmountX: number
}

export type FactoryAnchorGroup = {
  /** 机型显示名（组标题，只在组头写一次） */
  machine: string
  /**
   * **开始阶段收笔**的整条路径（打印一开始先把笔收起来那一段）。
   *
   * 来源：`crates/postprocess/src/pipeline/hooks.rs` 的 `BEGIN_STAGE_RETRACT_*`。
   * A 系列两步就走完；P 系列要绕角擦一下（先压到 X256，再降到 Y200，再退回 X250），
   * 中间每一段都写出来 —— 少写一段就复现不了。
   */
  startPath: string
  /** 组内每个版本一行 */
  rows: FactoryAnchorRow[]
}

export const FACTORY_ANCHORS: FactoryAnchorGroup[] = [
  {
    machine: 'A1',
    startPath: '276 → 20',
    rows: [
      { version: '', mountWaitX: 256, mountX: 267, unmountWaitX: 256, unmountX: 271 },
      { version: 'FAST', mountWaitX: 256, mountX: 267, unmountWaitX: 256, unmountX: 271 },
      { version: 'FASTV3.3', mountWaitX: 256, mountX: 267, unmountWaitX: 256, unmountX: 273 },
      { version: 'STANDARD', mountWaitX: 256, mountX: 267, unmountWaitX: 256, unmountX: 271 },
    ],
  },
  {
    machine: 'A1_MINI',
    startPath: '191 → 20',
    rows: [
      { version: '', mountWaitX: 168, mountX: 185, unmountWaitX: 168, unmountX: 189 },
      { version: 'FAST', mountWaitX: 168, mountX: 185, unmountWaitX: 168, unmountX: 189 },
      { version: 'FASTV3.3', mountWaitX: 168, mountX: 185, unmountWaitX: 168, unmountX: 189 },
      { version: 'STANDARD', mountWaitX: 168, mountX: 185, unmountWaitX: 168, unmountX: 189 },
    ],
  },
  {
    machine: 'P1S',
    startPath: '250,216 → 256 → 215 → 200 → 250',
    rows: [
      { version: '', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
      { version: 'LITE', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
    ],
  },
  {
    machine: 'P2S',
    startPath: '250,216 → 256 → 215 → 200 → 250',
    rows: [
      { version: '', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
      { version: 'STANDARD', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
    ],
  },
  {
    machine: 'X1C',
    startPath: '250,216 → 256 → 215 → 200 → 250',
    rows: [
      { version: '', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
      { version: 'LITE', mountWaitX: 20, mountX: 1, unmountWaitX: 246, unmountX: 256 },
    ],
  },
]
