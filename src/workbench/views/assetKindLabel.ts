/**
 * 资产类型的界面标签 —— **只有这一份**。
 *
 * 资产库页与改名框都要在行上标"这是哪一类"（切片器改名会换文件名，图 / 图标不会，
 * 所以那一格在改名框里不是装饰）。两处各写一个 Map 的话，加第六类时必定漏一处。
 */
import type { AssetKind } from '../api'

export const KIND_LABEL: Record<AssetKind, string> = {
  image: '机型图',
  icon: '图标',
  model: '模型',
  slicerProfile: '切片器',
  mkPreset: 'MKP 预设',
  /* 引导图（2026-10-16）：图片引导那套里的系统窗口截图（浅 / 深各一张） */
  guide: '引导图',
}
