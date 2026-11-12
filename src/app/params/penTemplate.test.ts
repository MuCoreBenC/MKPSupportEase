import { describe, expect, it } from 'vitest'

import {
  alignDiff,
  fieldOf,
  fmtNum,
  inlineSegs,
  placeholders,
  placeholderName,
  render,
  validate,
} from './penTemplate'

/** 与 Rust 侧 `pen_template::tests` 同一批骨架 —— 两边规则必须逐条对上。 */
const A1_MOUNT =
  'G92 E0\nG1 E-5 F1800\nG92 E0\nL801\nM204 S10000\nG1 X{wait} F42000\nG1 X{target} F{speed}\nG1 X{wait} F42000\nG1 F42000'

const P1_MOUNT =
  'L803\nG92 E0\nG1 E-10\nG92 E0\nL801\nM204 S10000\nG1 X{wait} Y{wait_y} F30000\nG1 X{target} F{speed}\nG1 X{wait} Y{wait_y} F30000\nG1 F30000'

const VARS = { wait: 256, wait_y: 7, target: 267, speed: 10000 }

describe('penTemplate', () => {
  it('整数不带小数点', () => {
    const out = render(A1_MOUNT, VARS)
    expect(out).toContain('G1 X256 F42000')
    expect(out).toContain('G1 X267 F10000')
    expect(fmtNum(256)).toBe('256')
  })

  it('A1 骨架不含 wait_y ⇒ 渲染结果里没有任何 Y', () => {
    expect(render(A1_MOUNT, VARS)).not.toContain('Y')
  })

  it('P1 骨架的两处 wait_y 都要替换', () => {
    const out = render(P1_MOUNT, VARS)
    expect(out.split('G1 X256 Y7 F30000').length - 1).toBe(2)
  })

  it('没有占位符的老骨架是恒等变换', () => {
    const raw = 'G92 E0\nG1 X1 F300'
    expect(render(raw, VARS)).toBe(raw)
  })

  it('未知占位符 / 没闭合 / 非法行首都要报出来', () => {
    expect(validate('G1 X{foo} F300')).toEqual({ kind: 'unknown', name: 'foo' })
    expect(validate('G1 X{wait F300')).toEqual({ kind: 'unclosed', at: 4 })
    expect(validate('G92 E0\nX100')).toEqual({ kind: 'malformed', line: 2, text: 'X100' })
  })

  it('注释与空行合法', () => {
    expect(validate('; 注释\n\nG1 X{wait} F300')).toBeNull()
  })

  it('按出现顺序列出占位符，重复只算一次', () => {
    expect(placeholders(A1_MOUNT)).toEqual(['wait', 'target', 'speed'])
    expect(placeholders(P1_MOUNT)).toEqual(['wait', 'wait_y', 'target', 'speed'])
  })

  it('占位符映射到落笔/收笔各自的字段', () => {
    expect(fieldOf('mount', 'wait')).toBe('toolhead.pen_mount_wait_x')
    expect(fieldOf('unmount', 'wait')).toBe('toolhead.pen_unmount_wait_x')
    expect(fieldOf('mount', 'target')).toBe('toolhead.pen_mount_x')
    expect(fieldOf('unmount', 'speed')).toBe('toolhead.pen_unmount_speed')
  })

  it('单个 token 的占位符识别', () => {
    expect(placeholderName('{wait}')).toBe('wait')
    expect(placeholderName('X{wait}')).toBeNull()
    expect(placeholderName('{nope}')).toBeNull()
  })
})

describe('alignDiff', () => {
  it('完全相同的两段：全是 same', () => {
    const rows = alignDiff('a\nb\nc', 'a\nb\nc')
    expect(rows.map((r) => r.kind)).toEqual(['same', 'same', 'same'])
  })

  it('删掉一行：只在左边出现，标 removed', () => {
    expect(alignDiff('a\nb\nc', 'a\nc')).toEqual([
      { left: 'a', right: 'a', kind: 'same' },
      { left: 'b', right: null, kind: 'removed' },
      { left: 'c', right: 'c', kind: 'same' },
    ])
  })

  it('新增一行：只在右边出现，标 added', () => {
    expect(alignDiff('a\nc', 'a\nb\nc')[1]).toEqual({ left: null, right: 'b', kind: 'added' })
  })

  it('一行改了半句：两栏并排成一条 changed，不是一删一加两条', () => {
    const rows = alignDiff('a\nG1 X1 F300;推到X8\nc', 'a\nG1 X1 F300;推到底\nc')
    expect(rows.map((r) => r.kind)).toEqual(['same', 'changed', 'same'])
    expect(rows[1].left).toBe('G1 X1 F300;推到X8')
    expect(rows[1].right).toBe('G1 X1 F300;推到底')
  })

  it('行尾有换行不算多一行', () => {
    expect(alignDiff('a\nb\n', 'a\nb').map((r) => r.kind)).toEqual(['same', 'same'])
  })

  it('配不上的多出来那几个：照旧单边成行', () => {
    const rows = alignDiff('a\nb\nc\nd', 'x\nd')
    expect(rows.map((r) => r.kind)).toEqual(['changed', 'removed', 'removed', 'same'])
    expect(rows[1]).toEqual({ left: 'b', right: null, kind: 'removed' })
  })
})

describe('inlineSegs', () => {
  it('只掐出分号后面变的那几个字', () => {
    const segs = inlineSegs('G1 X1 F300;推到X8,笔尖下落', 'G1 X1 F300;推到底')
    expect(segs.left).toEqual([
      { text: 'G1 X1 F300;推到', hit: false },
      { text: 'X8,笔尖下落', hit: true },
    ])
    expect(segs.right).toEqual([
      { text: 'G1 X1 F300;推到', hit: false },
      { text: '底', hit: true },
    ])
  })

  it('共同的尾巴不算变：向左离开 → 离开 只标「向左」', () => {
    const segs = inlineSegs('G1 X20 Y7 F30000;向左离开', 'G1 X20 Y7 F30000;离开')
    expect(segs.left).toEqual([
      { text: 'G1 X20 Y7 F30000;', hit: false },
      { text: '向左', hit: true },
      { text: '离开', hit: false },
    ])
    expect(segs.right).toEqual([
      { text: 'G1 X20 Y7 F30000;', hit: false },
      { text: '离开', hit: false },
    ])
  })

  it('两侧都能拼回原文', () => {
    const pair = ['G1 X1 F300;推到X8', 'G1 X2 F600;退到底']
    const segs = inlineSegs(pair[0], pair[1])
    expect(segs.left.map((s) => s.text).join('')).toBe(pair[0])
    expect(segs.right.map((s) => s.text).join('')).toBe(pair[1])
  })
})
