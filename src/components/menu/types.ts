/*
 * 右键菜单的形状（A34）。
 *
 * # 只有一层，没有子菜单
 *
 * 子菜单要处理悬停延迟、方向翻转、左右键 —— 三样东西换来的只是「菜单项多了还能塞」。
 * 一层 + 分隔线 + 危险项红字已经够用；真到塞不下那一天，该拆的是那个列表的动作，
 * 不是给菜单加一层。
 *
 * # 禁用一定带原因
 *
 * `disabled` 不是布尔，是**那句话**。灰一个项不说为什么，用户只会以为坏了 ——
 * 「云端文件不能改名，复制一份再改」这种话必须显示出来，所以类型上就不给「只灰不说」的余地。
 *
 * # 二次确认也在类型里
 *
 * 不可逆的动作（删除）要先问一句。问句放在数据里而不是交给调用方自己弹框，是为了让
 * 「哪些项危险」这件事在一个地方看得见；而且同一个动作对不同对象的话术不同
 * （云端文件删了能重下，用户自己的文件删了找不回来），所以 `detail` 是必要的第二句。
 *
 * # 图标是**可选**的，而且要求"挂的与不挂的交替"（2026-10-16 作者，两轮）
 *
 * 第一轮：「我希望加上图标……而且不要每个都加，给使用频率高的加」；
 * 第二轮（看到每项都挂上图标之后）：「这些不要图标，而且我希望是有图标的和没图标的交替」。
 *
 * ⇒ **不是"给常用的挂"就完了，是排布**：连着两项都挂图标看着像两排图标砖，
 *   交替开才有节奏、也才不至于让"图标"抢过"文字"。所以它是 `icon?` 而不是必填，
 *   而且**挂图标的那几项之间要隔一项不挂的**。
 *   ★ 这条只在**恒定出现**的项上能保证 —— 条件项（如 MKP 档的「编辑文件」、
 *     切片器档的「参数对比」）来不来要看那一行，跟着它们的邻居必须按"两条路都成立"
 *     来定挂不挂（本仓的做法：条件项一律不挂，它旁边那项一定是恒出现的）。
 * 那个空位仍占宽（见 `ContextMenu` 的 `.ico` 槽），否则挂图标与不挂的行左缘会参差。
 */

import type { IconName } from '../Icon'

/** 一个菜单项。`onSelect` 由调用方闭包好当前那一行，组件不碰业务数据 */
export interface ContextMenuItem {
  id: string
  label: string
  /**
   * 左边那枚图标。**省略 = 这一项确定不挂**（见文件头"挂的与不挂的交替"），
   * 不是"忘了挂"。名字取自 `components/Icon` 那张表，两边一处也不重复定义。
   */
  icon?: IconName
  /** **为什么不能点**。有值 = 禁用，这句话会显示在 title 里 */
  disabled?: string
  /** 红字。不可逆的动作才给 */
  danger?: boolean
  /** 有值就先问一句再执行 */
  confirm?: {
    /** 问句，如「删除 a1mini_调试版.toml？」 */
    question: string
    /** 后果。同一个动作对不同对象要说不同的话 */
    detail?: string
    /** 确认按钮上的字，默认「删除」 */
    ok?: string
  }
  onSelect: () => void
}

/** 分隔线。写成对象而不是 `null`，是为了让 `entries.map` 不用先过滤 */
export interface ContextMenuSeparator {
  separator: true
}

export type ContextMenuEntry = ContextMenuItem | ContextMenuSeparator

export function isSeparator(e: ContextMenuEntry): e is ContextMenuSeparator {
  return 'separator' in e
}

/** 打开的位置。**视口坐标** —— 换算成层内坐标的活在 ContextMenu 里做 */
export interface ContextMenuPoint {
  x: number
  y: number
}
