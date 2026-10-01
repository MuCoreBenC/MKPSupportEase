/*
 * 试验场调试面板那几个可调值，在产品里固定成常量。
 *
 * 试验场的首页 / 校准页从它自己的 devStore 读三个数：动画时长（glideMs）、
 * 分级揭示的淡入时长（fadeMs）、热区显不显形（showHit）。那是试验场的调试面板 ——
 * 不属于产品，所以这里不 import 它。
 *
 * 这里的三个值就是面板的**默认值**，逐个对过：
 *
 *   GLIDE_MS  720    devStore 里 `read(K_GLIDE, 720)`
 *   FADE_MS   300    devStore 里 `read(K_FADE_MS, 300)`
 *   SHOW_HIT  false  devStore 里 `read(K_HIT, false)`
 *
 * 所以只要没在面板里手调过，观感与试验场一致。差别只是这三个数在这里调不动了 ——
 * 要调就改这个文件，而不是开一个产品里没有的面板。
 *
 * # A41 起三个调用点都走 `useDevDefaults()`
 *
 * A41 在试验场把这条线接回了面板（`useDevDefaultsA41` 订阅 devStore），SlideDeck /
 * PageHome / MachinePicker 三个调用点全改成从 hook 取值。产品仓的接法（A41 README
 * 预写的）：**hook 直接 return 下面三个常量**，别的一行不动 —— 调用点不知道值
 * 是从哪来的，试验场那份文件怎么改注释，这里照抄语义就行。
 */

/** 卡片推入 / 退回的时长（ms）。读数条的 FLIP 补间也按它分段，见 CalibHead */
export const GLIDE_MS = 720

/** 选机型那一页三组选项分级揭示的淡入时长（ms）。以 `--fade-ms` 下发给 CSS */
export const FADE_MS = 300

/** 翻页热区要不要显形。产品里永远是 false —— 它是调试用的可视化 */
export const SHOW_HIT = false

/** 面板那侧的形状，产品里就是三个常量（A41 起三个调用点统一从这里取） */
export function useDevDefaults(): { glideMs: number; fadeMs: number; showHit: boolean } {
  return { glideMs: GLIDE_MS, fadeMs: FADE_MS, showHit: SHOW_HIT }
}
