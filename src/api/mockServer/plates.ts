import type { Plate } from '../../api/contract'

/**
 * 浏览器演示用的打印板表 + 机型引用。
 *
 * **这是真数据的镜像，不是第二份真相**：真机读 `presets/plates/*.toml`（定义侧）
 * 与 `presets/machines/*.toml` 的 `plateIds` / `defaultPlateId`；浏览器没有文件系统，
 * 这里把同两块的几何与同一套机型映射写成常量，让塔地图在演示里也能画出底衬。
 *
 * 漂移只会来自「真 plates/*.toml 改了这里没跟着改」——所以放同一个文件、
 * 头一句就点明同源；两台机型的映射（A1/P1S/P2S/X1C → 单卡舌，A1_MINI → 双卡舌）
 * 与真数据逐条对齐。
 */

/** 与 `presets/plates/single-latch-256.toml` 逐字节同源 */
const SINGLE_LATCH_256: Plate = {
  id: 'single-latch-256',
  name: '单卡舌 256',
  w: 258,
  d: 276,
  frame: { x: 1, y: 8.5, w: 256, h: 256 },
  bodyPath:
    'M7.5,8.0L250.5,8.0A7.5,7.5 0 0 1 258.0,15.5L258.0,266.0L6.0,266.0A6.0,6.0 0 0 1 0.0,260.0L0.0,15.5A7.5,7.5 0 0 1 7.5,8.0Z',
  path: 'M95.1,7.8L102.7,0.9A1.2,1.2 0 0 1 103.5,0.6L154.5,0.6A1.2,1.2 0 0 1 155.3,0.9L162.9,7.8A0.6,0.6 0 0 1 163.3,8.0L250.5,8.0A7.5,7.5 0 0 1 258.0,15.5L258.0,268.0A8.0,8.0 0 0 1 250.0,276.0L83.0,276.0A1.2,1.2 0 0 1 82.2,275.7L72.3,266.3A1.2,1.2 0 0 1 71.5,266.0L6.0,266.0A6.0,6.0 0 0 1 0.0,260.0L0.0,15.5A7.5,7.5 0 0 1 7.5,8.0L94.7,8.0A0.6,0.6 0 0 1 95.1,7.8ZM107.0,6.2L151.0,6.2L151.0,8.0L107.0,8.0L107.0,6.2ZM224.0,266.1L231.4,266.1L237.7,272.8L224.0,272.8L224.0,266.1ZM236.0,266.1L249.0,266.1L249.0,272.8L241.9,272.8L236.0,266.1Z',
}

/** 与 `presets/plates/dual-latch-180.toml` 逐字节同源 */
const DUAL_LATCH_180: Plate = {
  id: 'dual-latch-180',
  name: '双卡舌 180',
  w: 184,
  d: 197.1,
  frame: { x: 2, y: 8, w: 180, h: 180 },
  bodyPath:
    'M2.0,6.0L182.0,6.0A2.0,2.0 0 0 1 184.0,8.0L184.0,190.0L2.0,190.0A2.0,2.0 0 0 1 0.0,188.0L0.0,8.0A2.0,2.0 0 0 1 2.0,6.0Z',
  path: 'M33.8,5.9L39.9,0.6A0.8,0.8 0 0 1 40.4,0.4L61.6,0.4A0.8,0.8 0 0 1 62.1,0.6L69.4,5.9A0.5,0.5 0 0 1 69.7,6.0L114.2,6.0A0.5,0.5 0 0 1 114.5,5.9L121.1,0.6A0.8,0.8 0 0 1 121.6,0.4L143.6,0.4A0.8,0.8 0 0 1 144.1,0.6L150.1,5.9A0.5,0.5 0 0 1 150.4,6.0L182.0,6.0A2.0,2.0 0 0 1 184.0,8.0L184.0,195.0L182.1,196.9A0.5,0.5 0 0 1 181.8,197.1L59.1,197.1A1.0,1.0 0 0 1 58.4,196.8L51.0,190.3A1.0,1.0 0 0 1 50.3,190.0L2.0,190.0A2.0,2.0 0 0 1 0.0,188.0L0.0,8.0A2.0,2.0 0 0 1 2.0,6.0L33.5,6.0A0.5,0.5 0 0 1 33.8,5.9ZM59.7,192.2L60.3,192.2A1.2,1.2 0 0 1 61.5,193.4L61.5,194.1A1.2,1.2 0 0 1 60.3,195.3L59.7,195.3A1.2,1.2 0 0 1 58.5,194.1L58.5,193.4A1.2,1.2 0 0 1 59.7,192.2ZM164.3,191.2L170.2,191.2L173.2,195.3L164.3,195.3L164.3,191.2ZM174.1,191.2L181.8,191.2L181.8,195.3L176.1,195.3L174.1,191.2Z',
}

/** 机型 id → 板引用（与 `presets/machines/*.toml` 同源）。A2L 没配板 */
const REFS: Record<string, { plateIds: string[]; defaultPlateId: string | null }> = {
  A1: { plateIds: ['single-latch-256'], defaultPlateId: 'single-latch-256' },
  P1S: { plateIds: ['single-latch-256'], defaultPlateId: 'single-latch-256' },
  P2S: { plateIds: ['single-latch-256'], defaultPlateId: 'single-latch-256' },
  X1C: { plateIds: ['single-latch-256'], defaultPlateId: 'single-latch-256' },
  A1_MINI: { plateIds: ['dual-latch-180'], defaultPlateId: 'dual-latch-180' },
}

/** catalog 的 plates 域（按 id 排序，与真 catalog 一致） */
export function allPlates(): Plate[] {
  return [DUAL_LATCH_180, SINGLE_LATCH_256]
}

export function plateRefsOf(machineId: string): {
  plateIds: string[]
  defaultPlateId: string | null
} {
  return REFS[machineId] ?? { plateIds: [], defaultPlateId: null }
}
