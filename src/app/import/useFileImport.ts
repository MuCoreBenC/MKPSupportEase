/*
 * 通用导入入口的"把手"：上下文 + 消费者钩子。
 *
 * 拆出这一个文件的理由很具体：`FileImport.tsx` 里是组件，组件文件只导出组件
 * （Fast Refresh 的前提）；hook 与上下文住这里，两边都不吃亏。
 * 形状与职责见 `FileImport.tsx` 的头注释。
 */

import { createContext, useContext } from 'react'

/** 一次导入落定后，结果条上那一块 */
export interface ImportBannerState {
  text: string
  /** 有没成的就红一点 */
  bad: boolean
  /** 逐份明细（成功的也列 —— 落了哪个名字看得见） */
  lines: string[]
}

export interface ImportEntry {
  /** 打开系统文件选择器并导入（浏览器演示里给一份"选择器来的"演示路径） */
  pickFiles: () => void
  /** 导入成功过几批 —— 消费者拿它当"该重读用户线"的钥匙（只增不减） */
  revision: number
  /** 结果条（App 在标签栏下面渲染 `ImportBanner` 读它） */
  banner: ImportBannerState | null
  closeBanner: () => void
}

export const ImportContext = createContext<ImportEntry | null>(null)

/** 消费者（预设页是第一个）从这儿拿入口；没套 provider 是接线错误，直接报 */
export function useFileImport(): ImportEntry {
  const entry = useContext(ImportContext)
  if (entry === null) throw new Error('useFileImport 要在 FileImportProvider 里用')
  return entry
}
