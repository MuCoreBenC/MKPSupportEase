import { describe, expect, it } from 'vitest'

import type { ReportCalibrationFacts, ReportTowerFacts } from '../../api'
import { showsTowerGroup } from './towerVisibility'

const tower: ReportTowerFacts = { towerLayers: 38, towerHeight: 6.46 }

const calibration: ReportCalibrationFacts = {
  originPath: null,
  model3mf: null,
  exists: false,
  identity3mf: 'Combined',
  verdict: 'from_3mf',
  note: null,
  measured: [69.979, 189.49, 98.144, 113.416],
  profileId: 'zoffset-ext',
  tolerance: 0.5,
  footprint: [119.58, 15.272],
  mode: 'ZOffset',
}

describe('擦料塔这一组摆不摆', () => {
  it('普通成功记录：有塔、没有校准件身份 ⇒ 摆', () => {
    expect(showsTowerGroup({ tower, calibration: null })).toBe(true)
    expect(showsTowerGroup({ tower, calibration: undefined })).toBe(true)
  })

  it('校准件那一趟：两格都有 ⇒ 不摆（这一条就是上次的回归点）', () => {
    expect(showsTowerGroup({ tower, calibration })).toBe(false)
  })

  it('失败记录：后端不写塔那一格 ⇒ 不摆', () => {
    expect(showsTowerGroup({ tower: null, calibration: null })).toBe(false)
  })

  it('旧记录：两格都缺 ⇒ 不摆，也不抛', () => {
    expect(() => showsTowerGroup({})).not.toThrow()
    expect(showsTowerGroup({})).toBe(false)
  })
})
