import type { CSSProperties, ReactNode } from 'react'
import s from './CardFrame.module.css'

/** 一颗左下角胶囊。`on` 省略即常显（导航胶囊就是常显的） */
export interface Capsule {
  label: string
  onClick: () => void
  /** 绿底主按钮。只管样式，不管位置 */
  primary?: boolean
  /** label 后面跟一个 › */
  arrow?: boolean
  /** 往回走的那颗：箭头改成 ‹ 并放在 label 前面 */
  back?: boolean
  /** 状态胶囊：false 时不显形（占不占位由 CSS 决定，这里是追加式，所以不占位） */
  on?: boolean
}

interface CardFrameProps {
  /**
   * 眉标题那一行。它由外壳定位（`position: absolute`），不参与内容列布局。
   * 省略即整行不渲染 —— 因为是绝对定位，去掉它内容一个像素都不动。
   */
  eyebrow?: ReactNode
  /** 标题右侧的徽标（未保存 / 已保存）。绝对定位、不占位 */
  tag?: string
  tagTone?: 'accent' | 'muted'
  /**
   * 眉标题那一行右侧的一颗动作（校准两页的「打开模型」走这里）。
   *
   * 由 CardFrame 自己画，页面只给文字与回调：它得和 22px 的行盒一样高，
   * 不然眉标题的 y 会跟着变 —— 那条"全站同坐标"就破了（见 .eyebrow 的注释）。
   */
  topAction?: { label: string; onClick: () => void }
  /** 导航胶囊，恒显、占最前面几格：第一格全站同坐标。上一步在左、下一步在右 */
  navs: Capsule[]
  /** 随状态出现的胶囊，追加在导航右侧，出现时不推动任何已有元素 */
  actions?: Capsule[]
  /**
   * 给右侧露出的那张卡让位：内容盒的右边缘停在卡片左边缘之前。
   * 按需开启 —— 只有内容真的顶到那一条的卡才传，其余卡几何一个像素都不动。
   */
  peekSafe?: boolean
  children: ReactNode
  style?: CSSProperties
}

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

function Pill({ label, onClick, primary, arrow, back, on = true }: Capsule) {
  return (
    <button
      type="button"
      className={s.pill}
      data-on={on}
      data-primary={primary ? 'true' : undefined}
      tabIndex={on ? 0 : -1}
      onMouseDown={noFocus}
      onClick={onClick}
    >
      {back && <span aria-hidden="true">‹</span>}
      {label}
      {arrow && <span aria-hidden="true">›</span>}
    </button>
  )
}

/**
 * 卡片外壳：眉标题 / 徽标 / 左下角胶囊都由这里定位，页面只管内容。
 * 页面不得自己写这三样的定位 —— 那是"每页一套坐标"的来源。
 */
export default function CardFrame({
  eyebrow,
  tag,
  tagTone = 'accent',
  topAction,
  navs,
  actions,
  peekSafe,
  children,
  style,
}: CardFrameProps) {
  return (
    <div
      className={s.frame}
      data-peek-safe={peekSafe ? 'true' : undefined}
      /* 没有眉标题就不给眉标题留那条带子。带子是为它预留的，它不在，
         留着就是白占一百多像素 —— 内容被压低，下面的东西还容易被裁 */
      data-bare-top={eyebrow || tag ? undefined : 'true'}
      /* 左下角一颗胶囊都没有时，同理不给胶囊留那条带子 */
      data-bare-bottom={navs.length || actions?.length ? undefined : 'true'}
      style={style}
    >
      {(eyebrow || tag) && (
        <p className={s.eyebrow}>
          {eyebrow}
          <i />
          {topAction && (
            <button
              type="button"
              className={s.topBtn}
              onMouseDown={noFocus}
              onClick={topAction.onClick}
            >
              {topAction.label}
            </button>
          )}
          {tag && (
            <span className={s.tag} data-tone={tagTone}>
              {tag}
            </span>
          )}
        </p>
      )}

      {children}

      <div className={s.pills}>
        <div className={s.pillGroup}>
          {navs.map((n) => (
            <Pill key={n.label} {...n} />
          ))}
        </div>
        <div className={s.pillGroup}>
          {actions?.map((a) => (
            <Pill key={a.label} {...a} />
          ))}
        </div>
      </div>
    </div>
  )
}
