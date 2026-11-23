/*
 * 「校准抬升高度」那行算式要用的两个底数（2026-11-19）：首层与笔尖偏移。
 *
 * 首层来自切片器（`; initial_layer_print_height`，常见 0.2），设置页在切片之前
 * **不知道**它，所以按缺省 0.2 举例；笔尖偏移读**正在使用的那一份**预设的
 * `toolhead.offset.z`，两条线各读各的（与校准页同一套判据）：
 *
 *   · 用户线（`presets-local/…`）→ `getLocalCopyFor`（后端优先挑底账在用的那份）
 *   · 云端线（交付预设）        → `getMachineParams` 的注册表值
 *
 * 读不到就**不摆算式**：宁可不显示，也不拿一个编出来的数。
 */

import { useEffect, useState } from 'react'

import { api } from '../../api'
import { FIRST_LAYER_MM } from './calibLift'
import type { CalibLiftBase } from './calibLift'
import { useActivePreset } from '../state/appState'
import { useSourceRevision } from '../state/deliveryState'

export function useCalibLiftBase(): CalibLiftBase | null {
  const active = useActivePreset()
  const machineId = active?.machineId ?? ''
  const versionId = active?.versionId ?? ''
  const origin = active?.origin ?? null
  const [base, setBase] = useState<CalibLiftBase | null>(null)
  /*
   * 这两个数一条来自 `getMachineParams`、一条来自 `getLocalCopyFor`，云端那条读的是
   * 内部根那份 catalog —— 换过数据源，同一个版本的笔尖偏移可能就不是这个数了。
   */
  const sourceRevision = useSourceRevision()

  useEffect(() => {
    if (machineId === '' || versionId === '' || origin === null) {
      setBase(null)
      return
    }
    let alive = true
    /* 换源代次只当"该重读"的开关 —— 值本身不参与这一趟计算 */
    void sourceRevision

    const penOffset =
      origin === 'local'
        ? api.getLocalCopyFor(machineId, versionId).then((copy) => copy?.axes?.z ?? null)
        : api.getMachineParams(machineId, versionId).then((rows) => {
            const row = rows.find((p) => p.key === 'toolhead.offset.z')
            const n = row === undefined ? Number.NaN : Number(row.value)
            return Number.isFinite(n) ? n : null
          })

    penOffset.then(
      (z) => {
        if (!alive) return
        setBase(
          z === null || !Number.isFinite(z) ? null : { firstLayer: FIRST_LAYER_MM, penOffset: z },
        )
      },
      () => {
        /* 读不到 = 这一行只有数字框，不编造算式 */
        if (alive) setBase(null)
      },
    )

    return () => {
      alive = false
    }
  }, [machineId, versionId, origin, sourceRevision])

  return base
}
