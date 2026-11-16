/*
 * 拖拽配对：只钉文件名规则。浏览器沙箱里的路径限制不是纯函数能测的，
 * 但「先拖一个再拖一个」「zip 里两份」最终都落到这几条规则上。
 */

import { describe, expect, it } from 'vitest'

import { findMetaName, gcodeNameOfMeta, metaNameOf, pairsWith, stemOf } from './files'

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
