/*
 * 提示条的类型与话术（`NoteBar` 的邻居）。
 *
 * 为什么和组件分开放：那个文件只导出组件 —— 组件文件里混着导出常量或函数，
 * Vite 的热更新就没法只替换组件（eslint 的 react-refresh 规则会直接报出来）。
 * 预设页那一份也是这么分的（版面在 `presets/PagePresets.tsx`，话术在 `presets/presetTree.ts`）。
 */

import type { Text } from '../../i18n'

/**
 * 页面上那一句话：做了什么 / 缺什么。`bad` 的那一种是「没接上」，不是「操作失败」。
 *
 * `text` 是 [`Text`]（`msg(键, 变量)` 或原样句子）：这句话会一直躺在页脚直到下一次动作，
 * 换语言时它得跟着换字 ⇒ **存键，不存 `t()` 的结果**（渲染处用 `useText()`）。
 */
export interface Note {
  text: Text
  bad: boolean
}

/*
 * ★ 2026-10-14：这里原来有一个 `missingMethodText(method)` ——"缺某个契约方法时的统一话术"。
 * 删了：**没有任何调用点**（客户端侧再没有 `[API] 未实现的接口` 那种兜底 —— 那类兜底
 * 现在只在 `api/mock.ts` 里抛错，不再往界面上拼话术）。
 *
 * 这个文件现在只导出 `Note` 这一个类型，而它还是**该活着**：
 * `shared/NoteBar.tsx` 与 `report/PageReport.tsx` 都 `import type { Note }`。
 */
