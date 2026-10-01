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
 * 「官方文件不能改名，复制一份再改」这种话必须显示出来，所以类型上就不给「只灰不说」的余地。
 *
 * # 二次确认也在类型里
 *
 * 不可逆的动作（删除）要先问一句。问句放在数据里而不是交给调用方自己弹框，是为了让
 * 「哪些项危险」这件事在一个地方看得见；而且同一个动作对不同对象的话术不同
 * （官方文件删了能重下，用户自己的文件删了找不回来），所以 `detail` 是必要的第二句。
 */

/** 一个菜单项。`onSelect` 由调用方闭包好当前那一行，组件不碰业务数据 */
export interface ContextMenuItem {
  id: string
  label: string
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
