/*
 * 从 `*_meta.json` 里把这次实际生效的涂胶偏移读出来。
 *
 * 产物里的涂胶坐标 = 目标坐标 + 偏移，所以展示「偏移 0」只要把这两笔减掉。
 * 两笔 Z 是分开的：笔的机械高度（`penOffsetZ`）与落笔压进表面的补偿（`glueZOffset`），
 * 各自可以单独归零 —— 作者要看到「笔 Z 调成 0 会掉下去 4mm」这种对照。
 */

/** 一次运行实际生效的涂胶偏移（mm） */
export interface GlueOffsets {
  x: number
  y: number
  /** 笔尖相对喷嘴的机械高度 */
  toolZ: number
  /** 落笔时压进表面多少（负数是压进去） */
  glueZ: number
}

export const ZERO_OFFSETS: GlueOffsets = { x: 0, y: 0, toolZ: 0, glueZ: 0 }

function num(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null
}

function pick(src: unknown, key: string): number | null {
  if (typeof src !== 'object' || src === null) return null
  return num((src as Record<string, unknown>)[key])
}

/**
 * `meta.json` → 偏移。读不出来就是 `null`（界面按「未读到」处理，全部当 0）。
 *
 * 两个来源都认：`detail.glue.*` 是运行时真正用掉的数；`detail.snapshot.params.*`
 * 是那次运行的参数快照。前者优先 —— 它才是产物里已经加进去的那一笔。
 */
export function readGlueOffsets(meta: unknown): GlueOffsets | null {
  if (typeof meta !== 'object' || meta === null) return null
  const detail = (meta as Record<string, unknown>).detail
  const glue = typeof detail === 'object' && detail !== null ? (detail as Record<string, unknown>).glue : null
  const snap =
    typeof detail === 'object' && detail !== null
      ? ((detail as Record<string, unknown>).snapshot as Record<string, unknown> | undefined)
      : undefined
  const params = snap?.['params']

  const x = pick(glue, 'penOffsetX') ?? pick(params, 'toolhead.offset.x')
  const y = pick(glue, 'penOffsetY') ?? pick(params, 'toolhead.offset.y')
  const toolZ = pick(glue, 'penOffsetZ') ?? pick(params, 'toolhead.offset.z')
  const glueZ = pick(glue, 'glueZOffset') ?? pick(params, 'toolhead.glue_z_offset')
  if (x === null && y === null && toolZ === null && glueZ === null) return null
  return { x: x ?? 0, y: y ?? 0, toolZ: toolZ ?? 0, glueZ: glueZ ?? 0 }
}

/**
 * `meta.json` → 这次运行实际生效的「靠外几条」。
 *
 * 试验台的预测必须与产物用同一套参数，否则「预测 vs 实际」比的是两个口径。
 * 两处都认：`detail.snapshot.params.*` 是那次的参数快照（用户在「修改参数」里填的数），
 * `detail.snapshot.ir.Glue.EdgeLines` 是内核真正吃到的值。前者优先。
 */
export function readEdgeLines(meta: unknown): number | null {
  if (typeof meta !== 'object' || meta === null) return null
  const detail = (meta as Record<string, unknown>).detail
  if (typeof detail !== 'object' || detail === null) return null
  const snap = (detail as Record<string, unknown>).snapshot as Record<string, unknown> | undefined
  const fromParams = pick(snap?.['params'], 'wiping.glue_edge_lines')
  if (fromParams !== null) return fromParams
  const ir = snap?.['ir']
  const glue = typeof ir === 'object' && ir !== null ? (ir as Record<string, unknown>).Glue : null
  return pick(glue, 'EdgeLines')
}

/** 产物路径 → 同名元数据路径（`a/b/x.gcode` → `a/b/x_meta.json`） */
export function metaPathOf(gcodePath: string): string {
  return `${gcodePath.replace(/\.gcode$/i, '')}_meta.json`
}

/** 产物路径 → 同处理后原件路径（`a/b/x.gcode` → `a/b/x_original.gcode`） */
export function originalPathOf(gcodePath: string): string {
  return `${gcodePath.replace(/\.gcode$/i, '')}_original.gcode`
}
