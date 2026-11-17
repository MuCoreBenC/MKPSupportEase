/*
 * 拖拽配对：只钉文件名规则。浏览器沙箱里的路径限制不是纯函数能测的，
 * 但「先拖一个再拖一个」「zip 里两份」最终都落到这几条规则上。
 */

import { describe, expect, it } from 'vitest'

import {
  findMetaName,
  findOriginalName,
  gcodeNameOfMeta,
  gcodeNameOfOriginal,
  isOriginalName,
  metaNameOf,
  originalNameOf,
  pairsWith,
  pairsWithOriginal,
  stemOf,
} from './files'

describe('文件名配对', () => {
  it('带路径、大小写都能剥出主干', () => {
    expect(stemOf('a/b/x.gcode')).toBe('x')
    expect(metaNameOf('a/b/x.gcode')).toBe('x_meta.json')
    expect(gcodeNameOfMeta('a/b/x_meta.json')).toBe('x.gcode')
    expect(gcodeNameOfMeta('x.gcode')).toBeNull()
  })

  it('一堆文件里挑出同名 meta', () => {
    const names = ['other.gcode', 'other_meta.json', 'x.gcode', 'x_meta.json']
    expect(findMetaName('x.gcode', names)).toBe('x_meta.json')
    expect(findMetaName('nope.gcode', names)).toBeNull()
  })

  it('只有一份 meta 时容忍大小写差异', () => {
    expect(findMetaName('X.GCODE', ['X_META.JSON'])).toBe('X_META.JSON')
  })

  it('分两次拖：同名才算配对', () => {
    expect(pairsWith('x.gcode', 'x_meta.json')).toBe(true)
    expect(pairsWith('x.gcode', 'y_meta.json')).toBe(false)
  })
})

describe('后处理原件', () => {
  it('产物与原件互为对照，原件名不会叠两层后缀', () => {
    expect(isOriginalName('x_original.gcode')).toBe(true)
    expect(isOriginalName('a/b/X_ORIGINAL.GCODE')).toBe(true)
    expect(isOriginalName('x.gcode')).toBe(false)

    expect(originalNameOf('x.gcode')).toBe('x_original.gcode')
    expect(originalNameOf('a/b/x.gcode')).toBe('x_original.gcode')
    expect(originalNameOf('x_original.gcode')).toBe('x_original.gcode')

    expect(gcodeNameOfOriginal('a/x_original.gcode')).toBe('x.gcode')
    expect(gcodeNameOfOriginal('x.gcode')).toBeNull()
  })

  it('三份一起拖：原件只认同名产物，不会被当成产物', () => {
    const names = ['x_original.gcode', 'x.gcode', 'x_meta.json', 'y_original.gcode']
    expect(findOriginalName('x.gcode', names)).toBe('x_original.gcode')
    expect(findOriginalName('y.gcode', names)).toBe('y_original.gcode')
    expect(findOriginalName('z.gcode', names)).toBeNull()

    expect(pairsWithOriginal('x.gcode', 'x_original.gcode')).toBe(true)
    expect(pairsWithOriginal('x.gcode', 'y_original.gcode')).toBe(false)
  })
})
