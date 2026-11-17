/*
 * 元数据读取：只钉「读得出来」和「读不出来时是 null」两件事。
 * 真实 meta 的字段路径来自 2026-11-16 那次真机运行的 `*_meta.json`。
 */

import { describe, expect, it } from 'vitest'

import { metaPathOf, readEdgeLines, readGlueOffsets } from './meta'

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

describe('readEdgeLines', () => {
  it('优先读参数快照里那次运行的数', () => {
    expect(
      readEdgeLines({
        detail: { snapshot: { params: { 'wiping.glue_edge_lines': 1 }, ir: { Glue: { EdgeLines: 4 } } } },
      }),
    ).toBe(1)
  })

  it('没有参数快照时回落到内核吃到的 IR 值', () => {
    expect(readEdgeLines({ detail: { snapshot: { ir: { Glue: { EdgeLines: 3 } } } } })).toBe(3)
  })

  it('0 是合法值（= 关掉边缘带），不能当「没读到」', () => {
    expect(readEdgeLines({ detail: { snapshot: { params: { 'wiping.glue_edge_lines': 0 } } } })).toBe(0)
  })

  it('读不到就是 null（滑杆保持现状，不拿默认值去猜产物）', () => {
    expect(readEdgeLines({})).toBeNull()
    expect(readEdgeLines(undefined)).toBeNull()
    expect(readEdgeLines({ detail: {} })).toBeNull()
  })
})

describe('metaPathOf', () => {
  it('把 .gcode 换成 _meta.json', () => {
    expect(metaPathOf('/a/b/x.gcode')).toBe('/a/b/x_meta.json')
    expect(metaPathOf('/a/b/x.GCODE')).toBe('/a/b/x_meta.json')
  })
})
