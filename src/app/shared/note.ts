/*
 * 提示条的类型与话术（`NoteBar` 的邻居）。
 *
 * 为什么和组件分开放：那个文件只导出组件 —— 组件文件里混着导出常量或函数，
 * Vite 的热更新就没法只替换组件（eslint 的 react-refresh 规则会直接报出来）。
 * 预设页那一份也是这么分的（版面在 `presets/PagePresets.tsx`，话术在 `presets/presetTree.ts`）。
 */

/** 页面上那一句话：做了什么 / 缺什么。`bad` 的那一种是「没接上」，不是「操作失败」 */
export interface Note {
  text: string
  bad: boolean
}

/**
 * 缺某个契约方法时统一的话术。**方法名要原样写出来** ——
 * 它是往后要加进契约的那个东西的名字，说成「暂不可用」等于把这条线索丢掉。
 */
export function missingMethodText(method: string): string {
  return `契约里还没有这个方法：${method}`
}
