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
 *
 * # 放大看大的那一层（2026-10-18 起）
 *
 * `ZoomView` 抽成了**通用放大层** `components/zoom/ZoomLayer`（内容泛化成任意 ReactNode，
 * 不止 `<img>`）—— 这里只负责把 `GuideFigure` 装进去、经 `Modal` 的 `overlay` 槽挂上。
 * 交互定式（FLIP + WAAPI / wheel 三分流 / Esc 先收这层）全住在那边，别在这再写一份。
 */

import { useEffect, useRef, useState } from 'react'
import type { CSSProperties, ReactNode, Ref } from 'react'

import { useT } from '../../i18n'
import { useIsDark } from '../state/appState'
import { Btn } from '../ui/Controls'
import Modal from '../../components/modal/Modal'
import ZoomLayer from '../../components/zoom/ZoomLayer'
import s from './GuideImage.module.css'

/**
 * 一条标注**画出来什么样**（全部可选；一个都不写 = 今天的样子：圆角 8 / 线宽 2 / 编号 18）。
 *
 * 这几项由工作台「标注台」编辑、随标注源稿一起存（`presets/assets/guides/*.anno.json`），
 * 描到客户端的 CSS 变量上（`--mark-radius` / `--mark-stroke` / `--badge-size`）。
 *
 * ★ **颜色不在这儿**：颜色必须跟主题走（浅 / 深两套 token），写进标注就会和主题系统打架。
 */
export interface GuideMarkStyle {
  /** 框的圆角（px） */
  radius?: number
  /** 框的线宽（px）—— 画在 `box-shadow` 上，所以能小于 1px 也能量着缩放 */
  stroke?: number
  /** 左上角那颗编号圆点的直径（px） */
  badge?: number
}

/** 图上的一块标注 */
export interface GuideMark {
  /** 框的位置与大小：**百分比**（相对整张图的宽高） */
  at: { left: number; top: number; width: number; height: number }
  /** 框角上那颗编号 —— 与右侧那几步文字对得上 */
  step: number
  /** 画出来什么样（不写 = 站里的缺省；见 [`GuideMarkStyle`]） */
  style?: GuideMarkStyle
}

/** 一条标注的样式 → 三个 CSS 变量（写在**这条标注自己身上** —— 每条可以不一样） */
function markVars(style: GuideMarkStyle | undefined): CSSProperties {
  if (style === undefined) return {}
  const v: Record<string, string> = {}
  if (style.radius !== undefined) v['--mark-radius'] = `${style.radius}px`
  if (style.stroke !== undefined) v['--mark-stroke'] = `${style.stroke}px`
  if (style.badge !== undefined) v['--badge-size'] = `${style.badge}px`
  return v as CSSProperties
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
  /**
   * **这张图是不是"框已经烧进像素"的成品图**（2026-10-16 方案 A）。
   *
   * 是 ⇒ **不再叠画 `marks`** —— 框已经是图的一部分了，再画一层就是双份
   * （而且位置会因百分比取整差一点点，看着像重影）。
   *
   * 不写 / `false` ⇒ 老规矩：图是原图，框由这里按 `marks` 现画
   * （成员资产还是原图时仍然成立 —— 我们不是"一刀全切"，是"图里有没有"说了算）。
   */
  rendered?: boolean
}

/** 一步。`note` 是那步下面的小字（"不点它只对这一个文件生效"这种） */
export interface GuideStep {
  text: string
  note?: string
}

/**
 * 图 + 框。**只画图与标注**，文案由下面的模态框摆 —— 图也可以单独用在别处。
 *
 * `big` 是"放大看"那一档的尺寸（`.shot` 与 `.zoomShot` 两个上限，见 CSS 那段）：
 * 同一个组件、同一份标注，只是换了两条边界的最大值 —— 大图上的框与小图上的框
 * 是同一套百分比画出来的，所以对得上。
 *
 * `onBroken`：**字节取不到**（地址在、404；云端失效那种）时报给上层。
 * 上层据此把这一处当成"没有图"——**整块不画**，而不是留一个裂图框（作者 2026-10-16：
 * 「如果云端失效的时候，这个图片显示不出来是怎么样子的……要不然就留空吧」）。
 */
export function GuideFigure({
  figure,
  big = false,
  innerRef,
  onBroken,
}: {
  figure: GuideFigureSpec
  big?: boolean
  /** 放大那一层拿它做 FLIP（量"原来在哪 / 现在在哪"）—— 别处不用给 */
  innerRef?: Ref<HTMLDivElement>
  /** 图取不到时喊一声（不重试、不写 console —— 失败就是失败，留空） */
  onBroken?: () => void
}) {
  const isDark = useIsDark()
  return (
    <div className={s.frame} ref={innerRef}>
      <img
        className={big ? `${s.shot} ${s.zoomShot}` : s.shot}
        src={isDark ? figure.dark : figure.light}
        alt={figure.alt}
        onError={onBroken}
        /* ★ 禁掉原生拖图（2026-10-17）：放大层里按住一拖，WebKit 起的是"拖走这张图"，
           pointer 序列当场断掉 ⇒ 平移就废了（与工作台标注台同一条病） */
        draggable={false}
      />
      {/*
       * ★ **成品图不再叠画框**（2026-10-16 方案 A）：`figure.rendered` 为真时，
       * 框早就烧进像素里了 —— 再画一层就是双份（百分比取整还会差一点点，像重影）。
       * 原图（没重渲过的成员资产）时照旧按 `marks` 现画。
       */}
      {!figure.rendered &&
        figure.marks.map((m) => (
          <span
            key={m.step}
            className={s.mark}
            style={{
              left: `${m.at.left}%`,
              top: `${m.at.top}%`,
              width: `${m.at.width}%`,
              height: `${m.at.height}%`,
              /* 样式随标注走（每条可以不一样）；不写 = CSS 里的缺省值 */
              ...markVars(m.style),
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
  statusExtra,
  onClose,
}: {
  open: boolean
  title: string
  status: string
  /** `null` = 这一处还没有图（图没到位的那些引导照常能用：只剩文字） */
  figure: GuideFigureSpec | null
  steps: GuideStep[]
  /**
   * **状态句底下的一个槽**（2026-10-17）：失败的那几条话往往带着"你还得再做一件事"
   * （例：自动化授权被拒 ⇒ 去系统设置把「访达」勾回来）—— 这类"接着做"的入口
   * 由调用方给（按钮 / 链接），这里只负责摆在该摆的位置。`undefined` = 不摆。
   */
  statusExtra?: ReactNode
  onClose: () => void
}) {
  /* 点小图 = 在这个模态框里浮起来看大的（放大层住在 `components/zoom/ZoomLayer`）。
     `fromRef` 记的是**点的那一下小图在哪** —— FLIP 的 First，得在开之前量 */
  const [zoom, setZoom] = useState(false)
  const tr = useT()
  const fromRef = useRef<DOMRect | null>(null)
  const figBtnRef = useRef<HTMLButtonElement>(null)

  /**
   * **图取不到**（台账里查不到那条资产 ⇒ `figure === null`；地址在、字节 404 ⇒ 这里）。
   * 两条路落到同一处：**整块图不画**，右列文字自己占满整幅宽
   * （作者 2026-10-16：「要不然就留空吧，不要让它显示」——不留裂图、不留灰块、不写一个字）。
   */
  const [imgBroken, setImgBroken] = useState(false)
  /* 换一处引导 / 切深浅（换了一张 URL）都要复位 —— 否则上一张坏了会把这一张也藏起来 */
  useEffect(() => {
    setImgBroken(false)
  }, [figure?.light, figure?.dark, open])
  /* 放大那一层里才坏掉的话，也把它收掉（回到"无图"那一态，文字照常） */
  useEffect(() => {
    if (imgBroken) setZoom(false)
  }, [imgBroken])

  const usable = figure !== null && !imgBroken ? figure : null

  /* 模态框一关，放大那一层也跟着收（不然下次打开还是放大的） */
  useEffect(() => {
    if (!open) setZoom(false)
  }, [open])

  return (
    <Modal
      open={open}
      title={title}
      size="lg"
      onClose={onClose}
      overlay={
        zoom && usable !== null ? (
          <ZoomLayer
            from={fromRef.current ?? new DOMRect(0, 0, 0, 0)}
            returnTo={() => figBtnRef.current?.getBoundingClientRect() ?? null}
            onGone={() => setZoom(false)}
            label={tr('guide.zoomClose')}
          >
            <GuideFigure figure={usable} big onBroken={() => setImgBroken(true)} />
          </ZoomLayer>
        ) : undefined
      }
      footer={
        <Btn variant="primary" onClick={onClose}>
          {tr('guide.ok')}
        </Btn>
      }
    >
      {/*
        * 图在左、**文字全在右**（2026-10-16 作者：「这上面这段话能不能移到右侧呀，
        * 就是说左侧显示图片，右侧显示文字」）—— 状态那句原来横在顶上占满整幅宽，
        * 图被压到第二行去，视线得来回跳；现在右侧一列从上到下就是
        * 「此刻在哪（status）→ 怎么做（steps）」，图在左边同一眼界里。
        */}
      <div className={s.split}>
        {usable !== null && (
          /* 图可点（小窗里它只剩 ~200px 宽，用户要能点开看大的）—— 皮扒干净，只留"能点"。
             ★ 不摆"点一下放大"那种提示（作者 2026-10-16：不要出现那个提示）；能点的意思
             由光标（`cursor: zoom-in`）与那一下过渡自己说。 */
          <button
            ref={figBtnRef}
            type="button"
            className={s.figBtn}
            onClick={() => {
              fromRef.current = figBtnRef.current?.getBoundingClientRect() ?? null
              setZoom(true)
            }}
            aria-label={tr('guide.zoomAria', { alt: usable.alt })}
          >
            <GuideFigure figure={usable} onBroken={() => setImgBroken(true)} />
          </button>
        )}
        <div className={s.side}>
          <p className={s.status}>{status}</p>
          {statusExtra}
          <ol className={s.steps}>
            {steps.map((step, i) => (
              <li key={i} className={s.step}>
                <span className={s.stepText}>{step.text}</span>
                {step.note !== undefined && <em className={s.stepNote}>{step.note}</em>}
              </li>
            ))}
          </ol>
        </div>
      </div>
    </Modal>
  )
}
