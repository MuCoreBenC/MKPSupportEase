/*
 * 「测试模型」合影 —— 首页第五步那张与校准页「测试模型」那一格是**同一张**。
 *
 * # 两套主题 = 两张位图（2026-10-15）
 *
 * 这是白件 + 绿件的渲染合影。浅底那张是原作（作者：「浅色模式下的表现还可以」）；
 * 深底那张是**后期**出来的：中性（白件）像素按 **0.56** 压暗，绿色一字不动
 * （作者：「深色模式下太亮了…中间那个绿色的还可以，把那些白色的调整一下颜色」）。
 * 没有重出图的源，所以是像素级后期，不是重新渲染。
 *
 * ★ 为什么是"两张位图"，不是"一张图 + 深色下挂 `filter: brightness()`"：
 *   ① filter 作用于**整张图** —— 绿色会跟着一起暗，而这一刀要的正是"绿的不动"；
 *   ② 它做在合成之后的像素上，半透明描边那圈也会被压暗，边就脏了。
 *   "白件在白底 / 深底各该多亮"这件事只有位图分得开。
 *
 * ★ 谁要这张图就走这里取，别在页面里自己拼 `src` —— 深浅配对只有这一处。
 *   `isDark` 一律从 `useIsDark()` 拿（那是"档位 + 系统深浅"解析完的真值），
 *   别自己判 CSS 或 `matchMedia`。
 */

import heroPile from './hero_pile.webp'
import heroPile2x from './hero_pile@2x.webp'
import heroPileDark from './hero_pile_dark.webp'
import heroPileDark2x from './hero_pile_dark@2x.webp'

/**
 * 按当前深浅取那一张（`src` + `srcSet`，两张位图同尺寸同比例，切换不跳版）。
 */
export function heroPileSource(isDark: boolean): { src: string; srcSet: string } {
  const one = isDark ? heroPileDark : heroPile
  const two = isDark ? heroPileDark2x : heroPile2x
  return { src: one, srcSet: `${one} 1x, ${two} 2x` }
}
