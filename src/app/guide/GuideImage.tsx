/*
 * **图片引导**（2026-10-16 作者提的）—— 一张**系统窗口的截图** + 图上几块**框选**（带编号）
 * + 几步文字，装在模态框里。给"这一步得照着做、但那扇窗不在我们手里"的地方用。
 *
 * # 与 `GuideCard` 是两件事，别混（都叫"引导"）
 *
 *   `GuideCard`（屏幕引导）  贴到**切片器那扇活窗口**旁边，箭头指着控件，
 *                            位置由 AX 跟着窗口走（`ipc/guide.rs`）—— 用户**当场**照着点
 *   这里（图片引导）          拿一张**截图**讲步骤：对象是系统窗口（访达的「显示简介」
 *                            那种），我们开得出来、摆不了卡，只能给张图
 *
 * 两件事共用一个词是作者的话（「后续很多步骤要不要也用图片去引导用户呢」），
 * 所以名字里都带"引导"，靠前缀分（屏幕 / 图片）。
 *
 * # 作者对它的要求（2026-10-16）
 *
 * 「到时候不知道是模态框，还是浮窗，还是什么，功能试一下，到时候效果好的话，其他的引导
 * 都可以复用。效果不好，到时候再改」＋「千万不要做复杂」。
 * ⇒ 所以这个文件只有三样东西：**标注的类型 / 画标注的那个图 / 装它的那个模态框**。
 *
 * # 框选为什么必须是"图层"，不能把框画进图里
 *
 * 作者：「我为什么不自己去框选呢？……如果我自己框选，可能就算我非常精确，也会有一些像素的
 * 偏移。框选出来的框框大小可能不一致」—— 同一处引导要给**浅色 / 深色两张图**，
 * 两张各画一遍框必然对不齐。所以标注是**数据**（百分比坐标，见 [`GuideMark`]），
 * 两张图共用同一份，由这一个组件画上去 ⇒ 位置 / 粗细 / 圆角天然一致。
 *
 * # 坐标
 *
 * `left/top/width/height` 一律**百分比**（相对那张图的宽高）⇒ 图缩放时不跑位。
 * ⚠️ 百分比是相对**整张图**算的：作者给的是系统截图（自带窗口投影的边距），
 * 量坐标时把边距一起算进去。
 *
 * # 深浅两张怎么取
 *
 * `useIsDark()`（`state/appState` 那唯一一份解析好的真值）二选一，不自己判 CSS、
 * 也不自己 `matchMedia` —— 取图那一句收在各自的引导 spec 里（**谁要这张图就走那里取**）：
 * 引导图住**资产台账**（`presets/assets.toml` 的 `type = 'guide'`），spec 按**资产 id**
 * 从 catalog 算 URL（`assets/hero/heroPile.ts` 那种 import 写法只留给"跟着源码走"的界面素材）。
 */

import { useIsDark } from '../state/appState'
import { Btn } from '../ui/Controls'
import Modal from '../../components/modal/Modal'
import s from './GuideImage.module.css'

/** 图上的一块标注 */
export interface GuideMark {
  /** 框的位置与大小：**百分比**（相对整张图的宽高） */
  at: { left: number; top: number; width: number; height: number }
  /** 框角上那颗编号 —— 与右侧那几步文字对得上 */
  step: number
}

/** 一处图片引导的图（浅 / 深两张同裁切）＋ 标注 */
export interface GuideFigureSpec {
  /** 浅色模式那张 */
  light: string
  /** 深色模式那张（**与浅色那张同尺寸同裁切**，否则标注会飘） */
  dark: string
  /** 这张图在说什么（无障碍那句，也是图没加载出来时能读到的） */
  alt: string
  marks: GuideMark[]
}

/** 一步。`note` 是那步下面的小字（"不点它只对这一个文件生效"这种） */
export interface GuideStep {
  text: string
  note?: string
}

/** 图 + 框。**只画图与标注**，文案由下面的模态框摆 —— 图也可以单独用在别处 */
export function GuideFigure({ figure }: { figure: GuideFigureSpec }) {
  const isDark = useIsDark()
  return (
    <div className={s.frame}>
      <img className={s.shot} src={isDark ? figure.dark : figure.light} alt={figure.alt} />
      {figure.marks.map((m) => (
        <span
          key={m.step}
          className={s.mark}
          style={{
            left: `${m.at.left}%`,
            top: `${m.at.top}%`,
            width: `${m.at.width}%`,
            height: `${m.at.height}%`,
          }}
        >
          <span className={s.badge}>{m.step}</span>
        </span>
      ))}
    </div>
  )
}

/**
 * 图片引导那个框：一句状态 + 图（可省）+ 几步。
 *
 * `status` 是"刚才那一下发生了什么"（"已帮你打开那份文件的「显示简介」"；失败时就是
 * 那条错的原话）—— 与 `figure` 分开：图讲**怎么做**，状态讲**此刻在哪**。
 */
export default function GuideImageModal({
  open,
  title,
  status,
  figure,
  steps,
  onClose,
}: {
  open: boolean
  title: string
  status: string
  /** `null` = 这一处还没有图（图没到位的那些引导照常能用：只剩文字） */
  figure: GuideFigureSpec | null
  steps: GuideStep[]
  onClose: () => void
}) {
  return (
    <Modal
      open={open}
      title={title}
      size="lg"
      onClose={onClose}
      footer={
        <Btn variant="primary" onClick={onClose}>
          知道了
        </Btn>
      }
    >
      <p className={s.status}>{status}</p>
      {/* 图在左、步骤在右：图是照着做的那张，两步文字就摆在同一眼界里 */}
      <div className={s.split}>
        {figure !== null && <GuideFigure figure={figure} />}
        <ol className={s.steps}>
          {steps.map((step, i) => (
            <li key={i} className={s.step}>
              <span className={s.stepText}>{step.text}</span>
              {step.note !== undefined && <em className={s.stepNote}>{step.note}</em>}
            </li>
          ))}
        </ol>
      </div>
    </Modal>
  )
}
