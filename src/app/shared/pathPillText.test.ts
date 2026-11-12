/*
 * 路径胶囊那串字的判据（2026-11-12）。
 *
 * # 守的是什么
 *
 * 那条胶囊的读法是固定的：**前段自己收省略号、尾两段钉住不缩** ——
 * "往哪个文件夹 / 哪个包里导"永远看得见。两扇导出框（报告 / 日志）共用这一份算法，
 * 所以这里只钉纯函数，不起 DOM。
 */
import { describe, expect, it } from 'vitest'

import { joinExportPath, splitPillPath } from './pathPillText'

describe('joinExportPath', () => {
  it('把文件夹与基名拼起来，目录尾部多一个分隔符也认', () => {
    expect(joinExportPath('/Users/wzy/Desktop/', 'a.zip', false)).toBe('/Users/wzy/Desktop/a.zip')
    expect(joinExportPath('/Users/wzy/Desktop', 'a.zip', false)).toBe('/Users/wzy/Desktop/a.zip')
  })

  it('落点是文件夹时尾巴补一个分隔符（读起来是"落进这个文件夹"）', () => {
    expect(joinExportPath('/Users/wzy/Desktop', 'MKP-导出报告', true)).toBe(
      '/Users/wzy/Desktop/MKP-导出报告/',
    )
  })

  it('Windows 的反斜杠按原样拼，不混进正斜杠', () => {
    expect(joinExportPath('C:\\Users\\wzy\\Desktop', 'a.zip', false)).toBe(
      'C:\\Users\\wzy\\Desktop\\a.zip',
    )
  })
})

describe('splitPillPath', () => {
  it('深路径：前段吃省略号，尾两段钉住', () => {
    expect(splitPillPath('/Users/wzy/Desktop/MKP-导出报告/x.zip', false)).toEqual({
      head: '/Users/wzy/Desktop/',
      tail: 'MKP-导出报告/x.zip',
    })
  })

  it('文件夹落点：尾段自带分隔符', () => {
    expect(splitPillPath('/Users/wzy/Desktop', true)).toEqual({
      head: '/Users/',
      tail: 'wzy/Desktop/',
    })
  })

  it('段数不够"尾两段"时整条当尾段，不分', () => {
    expect(splitPillPath('C:\\b', false)).toEqual({ head: '', tail: 'C:\\b' })
  })

  it('Windows 路径按反斜杠拆', () => {
    expect(splitPillPath('C:\\Users\\wzy\\Desktop\\MKP-导出报告', true)).toEqual({
      head: 'C:\\Users\\wzy\\',
      tail: 'Desktop\\MKP-导出报告\\',
    })
  })
})
