/*
 * 路径胶囊：导出类框里"保存位置"那一行（2026-11-12 抽成共用件）。
 *
 * 原来只有报告页的「导出执行报告」有它，日志导出那扇框另写了一遍 —— 两处形状不一样，
 * 同一件事在两扇框里长得不同。现在收成一件：**整条是显示、只有右边那颗能点**
 * （路径正文还能拖选，误点一下不会弹系统框）。
 *
 * 三条口径：
 *
 * 1. **前段自己收省略号、尾两段钉住不缩** —— "往哪个文件夹 / 哪个包里"永远看得见；
 * 2. **只显示、不点**：整条不是按钮，改位置只有右边那一颗；
 * 3. **成了就地换字**（`opened`）：同一格、同一尺寸，配色换强调色 —— 那一刻最该点的是它。
 */

import css from './PathPill.module.css'
import { splitPillPath } from './pathPillText'

/** 尾部那一段占位用的空白：空 span 没有行盒，量尺会白摆 */
const NBSP = '\u00a0'

interface PathPillStyle {
  /** 这一趟已经有东西落在那个位置了 ⇒ 那颗按钮换强调色 */
  opened?: boolean
  /** `path` 是个文件夹 ⇒ 尾巴补分隔符 */
  folder?: boolean
}

/** 两种形态：真胶囊（要路径与动作）与量尺（什么都不要）。缺字段就是编译错误 */
export type PathPillProps =
  | ({ gauge: true } & PathPillStyle)
  | ({
      gauge?: false
      /** 完整路径（文件夹或文件都行）。空串不画 —— 调用方自己判 */
      path: string
      /** 右侧那颗按钮的字（「更改位置」/「打开位置」…） */
      action: string
      onAction: () => void
    } & PathPillStyle)

export default function PathPill(props: PathPillProps) {
  /* 量尺：只占高度（不画、不读屏、不吃鼠标），撑住框身不先矮后高 */
  if (props.gauge === true) {
    return (
      <div className={css.pill} aria-hidden="true">
        <span className={css.text}>
          <span className={css.tail}>{NBSP}</span>
        </span>
        <span className={css.edit}>{NBSP}</span>
      </div>
    )
  }

  const { path, action, onAction, opened = false, folder = false } = props
  const { head, tail } = splitPillPath(path, folder)
  return (
    <div className={css.pill} data-tip={path}>
      <svg
        className={css.icon}
        width={15}
        height={15}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.7}
        aria-hidden="true"
      >
        <path d="M3.5 6.5A2 2 0 0 1 5.5 4.5h4l2 2.5h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
      </svg>
      <span className={css.text}>
        {head !== '' && <span className={css.head}>{head}</span>}
        <span className={css.tail}>{tail}</span>
      </span>
      <button
        type="button"
        className={css.edit}
        data-open={opened ? 'true' : 'false'}
        onClick={onAction}
      >
        {action}
      </button>
    </div>
  )
}
