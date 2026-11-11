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
  mountX: number
  /** `null` = 跟随驻点 */
  mountY: number | null
  unmountX: number
  /** `null` = 跟随驻点 */
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
    return list.filter(isPreset).slice(0, MAX)
  } catch (err) {
    console.error('[debugPresets] 读存档失败', err)
    return []
  }
}

function isPreset(v: unknown): v is DebugPreset {
  if (typeof v !== 'object' || v === null) return false
  const p = v as Record<string, unknown>
  return (
    typeof p.id === 'string' &&
    typeof p.at === 'number' &&
    typeof p.waitX === 'number' &&
    typeof p.waitY === 'number' &&
    typeof p.mountX === 'number' &&
    typeof p.unmountX === 'number' &&
    typeof p.speed === 'number' &&
    typeof p.moveY === 'boolean' &&
    (p.mountY === null || typeof p.mountY === 'number') &&
    (p.unmountY === null || typeof p.unmountY === 'number')
  )
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
  /** 落笔那一路：等待位 → 落笔位 */
  mountPath: string
  /** 收笔那一路：等待位 → 收笔位 */
  unmountPath: string
}

export type FactoryAnchorGroup = {
  /** 机型显示名（组标题，只在组头写一次） */
  machine: string
  /**
   * **开始阶段收笔**的位置（打印一开始先把笔收起来那一段）。
   * 来源：`crates/postprocess/src/pipeline/hooks.rs` 的 `BEGIN_STAGE_RETRACT_*`。
   */
  startRetract: string
  /** 组内每个版本一行 */
  rows: FactoryAnchorRow[]
}

export const FACTORY_ANCHORS: FactoryAnchorGroup[] = [
  {
    machine: 'A1',
    startRetract: '276',
    rows: [
      { version: '', mountPath: '256 → 267', unmountPath: '256 → 271' },
      { version: 'FAST', mountPath: '256 → 267', unmountPath: '256 → 271' },
      { version: 'FASTV3.3', mountPath: '256 → 267', unmountPath: '256 → 273' },
      { version: 'STANDARD', mountPath: '256 → 267', unmountPath: '256 → 271' },
    ],
  },
  {
    machine: 'A1_MINI',
    startRetract: '191',
    rows: [
      { version: '', mountPath: '168 → 185', unmountPath: '168 → 189' },
      { version: 'FAST', mountPath: '168 → 185', unmountPath: '168 → 189' },
      { version: 'FASTV3.3', mountPath: '168 → 185', unmountPath: '168 → 189' },
      { version: 'STANDARD', mountPath: '168 → 185', unmountPath: '168 → 189' },
    ],
  },
  {
    machine: 'P1S',
    startRetract: '256',
    rows: [
      { version: '', mountPath: '20 → 1', unmountPath: '246 → 256' },
      { version: 'LITE', mountPath: '20 → 1', unmountPath: '246 → 256' },
    ],
  },
  {
    machine: 'P2S',
    startRetract: '256',
    rows: [
      { version: '', mountPath: '20 → 1', unmountPath: '246 → 256' },
      { version: 'STANDARD', mountPath: '20 → 1', unmountPath: '246 → 256' },
    ],
  },
  {
    machine: 'X1C',
    startRetract: '256',
    rows: [
      { version: '', mountPath: '20 → 1', unmountPath: '246 → 256' },
      { version: 'LITE', mountPath: '20 → 1', unmountPath: '246 → 256' },
    ],
  },
]
