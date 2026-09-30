/*
 * 交付身份的角标。
 *
 * 只有两档，与工作台的预设管理页**同一条语义**，换的是尺度与色值：
 *
 *   默认交付  蓝 —— 这个版本钦定的最终资源，点下载后台自动下好
 *   可选      中性灰 —— 仓库里有、没进默认集，要手动下
 *
 * 「可选」**不上警示色**：没进默认集是一种身份，不是故障（工作台那边这里曾经用过 warn 色，
 * 标签错了）。蓝用 `--axis-y`，不引工作台那套 `--w-*`。
 */

import type { PresetFileInfo } from '../../api'
import { DELIVERY_TEXT, DELIVERY_WHY } from '../presets/presetTree'
import s from './OriginChip.module.css'

interface Props {
  delivery: PresetFileInfo['delivery']
}

export default function OriginChip({ delivery }: Props) {
  return (
    <span
      className={delivery === 'default' ? s.chipDefault : s.chipOptional}
      title={DELIVERY_WHY[delivery]}
    >
      {DELIVERY_TEXT[delivery]}
    </span>
  )
}
