/*
 * 元数据读取：只钉「读得出来」和「读不出来时是 null」两件事。
 * 真实 meta 的字段路径来自 2026-11-16 那次真机运行的 `*_meta.json`。
 */

import { describe, expect, it } from 'vitest'

import { metaPathOf, readGlueOffsets } from './meta'

const REAL = {
  detail: {
    glue: { penOffsetX: -0.9, penOffsetY: 23, penOffsetZ: 4, glueZOffset: -0.05 },
    snapshot: {
      params: { 'toolhead.offset.x': -0.9, 'toolhead.offset.y': 23, 'toolhead.offset.z': 4 },
    },
  },
}

describe('readGlueOffsets', () => {
  it('优先读运行时真正生效的那一份', () => {
    expect(readGlueOffsets(REAL)).toEqual({ x: -0.9, y: 23, toolZ: 4, glueZ: -0.05 })
  })

  it('只有参数快照时也能读（笔 Z 与胶 Z 各来自各自的键）', () => {
    const snapOnly = {
      detail: {
        snapshot: {
          params: {
            'toolhead.offset.x': -0.9,
            'toolhead.offset.y': 23,
            'toolhead.offset.z': 4,
            'toolhead.glue_z_offset': -0.05,
          },
        },
      },
    }
    expect(readGlueOffsets(snapOnly)).toEqual({ x: -0.9, y: 23, toolZ: 4, glueZ: -0.05 })
  })

  it('什么都没读到就是 null（界面按 0 处理，不许瞎猜）', () => {
    expect(readGlueOffsets({})).toBeNull()
    expect(readGlueOffsets(null)).toBeNull()
    expect(readGlueOffsets('x')).toBeNull()
  })
})

describe('metaPathOf', () => {
  it('把 .gcode 换成 _meta.json', () => {
    expect(metaPathOf('/a/b/x.gcode')).toBe('/a/b/x_meta.json')
    expect(metaPathOf('/a/b/x.GCODE')).toBe('/a/b/x_meta.json')
  })
})
