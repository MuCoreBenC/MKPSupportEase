/*
 * 工作台「去处理」定位 —— 2026-11-04 起只是共用件的**一层转发**。
 *
 * # 历史（别再抄回来）
 *
 * 这里原先自己写了一套：`glideIntoView` + `onDone` 里落位补测（`content-visibility`
 * 的行渲染晚一帧，不补就会"好像滚到了、又突然闪到别的地方"）+ 挂 `.flashIt` /
 * `.rowFlash` 两条 CSS `@keyframes`、1.8 秒后摘类。同时设置页用绿描边圈 1.4 秒、
 * 预设表用 `data-flash` 两段过渡 1.78 秒 —— 同一个动作三时长相。
 *
 * 作者 2026-11-04：「消失得太快了」「能不能是绿色背景，而不是这种圈起来的呀」
 * 「以后我还有什么需求，只要是滚动定位，大家都用一样的」。
 *
 * ⇒ 现在滚 / 补测 / 绿底 / 停 6 秒淡出**全在** `components/scroll/locate`；
 *   工作台这一层只保留 `locateAnchor` 这个名字与 `{ row }` 参数，免得
 *   `ParamSheetPage` / `BuildPage` / `BundlesPage` / `AssetsPage` 那几个调用点
 *   为了改个 import 路径白改一遍。`row` 现在也不再区分卡/行形态：两种都只铺绿底。
 *
 * ★ 新写的定位**直接 import 共用件**，不要再经这一层转发（它只是迁移期的别名）。
 */
export { locateAnchor } from '../../components/scroll/locate'
