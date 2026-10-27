/*
 * 勾选 / 单选共用件的出口。
 *
 * 宿主一律从这里 import（`import { Check } from '../../components/check'`）——
 * 别直接指向 `Check.tsx`，将来换实现才只用改这一处。
 *
 * 这一件只解决"全站的勾和框长一个样"；页面自己的行框感（整行可点的内边距、
 * 悬停提亮、主文案 + 小字说明两行）**留在各页**，用 `className` 传进来。
 */

export { default as Check } from './Check'
export type { CheckProps } from './Check'
