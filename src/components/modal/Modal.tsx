/*
 * 居中模态框（A34）。任何稿都能用，和 `components/field`、`components/menu` 同级别。
 *
 * # 为什么单独做一个件
 *
 * 作者的原话：「还有很多模态框还没有居中」。散着写的话每处都要自己算居中，
 * 而这个项目的外壳有个陷阱（见下），算错一次就偏一次。一套实现就只有一种对错。
 *
 * # 三条硬约定
 *
 * 1. **居中用 flex，不用 `top:50%` + `translate(-50%,-50%)`。**
 *    后者在奇数像素上会把框摆在 0.5px 处，文字渲染发虚；而且外壳 `.window` 本来就带
 *    transform，再叠一层只会更难查。flex 居中是整数对齐的。
 *
 * 2. **`position: absolute`，绝不用 `fixed`。**
 *    `src/App.module.css` 的 `.window` 有一个永不为 none 的 transform，它是 fixed 的
 *    包含块 —— #33 那两个「点了没反应」的浮层就栽在这上面：量的是视口坐标、
 *    被当成局部坐标用，偏移一百多像素之后再被 overflow 裁掉。
 *
 * 3. **遮罩挂在调用方给的那一层（内容区），不是整页。**
 *    底栏与窗口右下角那个 resize 手柄还得点得着 —— #31 特意调过。
 *    所以这个组件只画 `absolute inset:0` 的遮罩，挂在哪由宿主的定位祖先决定。
 *
 * # 什么时候用它，什么时候用抽屉
 *
 *   模态框    看完就关、或者要当场做个决定（查看详情 / 确认 / 三步向导）
 *   右抽屉    看一眼还想接着改（编辑历史那种，之前定过它要用右抽屉）
 *
 * 判据是「关掉之后还要不要回来」，不是「内容多不多」。
 */

import { useEffect, useRef, useState } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import Icon from '../Icon'
import { ModalHeadNoteSlotContext, ModalHeadSlotContext } from './modalHeadSlot'
import s from './Modal.module.css'

/*
 * **打开着的模态框**（后开的在最后）。Esc 只关**最上面那一层**（2026-10-21）。
 *
 * 作者：「我按 ESC 的时候，好像把整个模态框都退了 —— 你应该先退这一个，
 * 然后把外面那个显示出来」。原来每个 Modal 各挂一个 document keydown，
 * 两层同时开着（编辑文件 + 与默认值对比）时同一个 Esc 两个都收到，于是两层一起关。
 *
 * 栈只有这一处，判据只有一条：**不是栈顶就不响应**。
 * 栈顶关掉之后自己出栈，下一次 Esc 才轮到下面那一层 —— 正是"先退这一个、再退外面"。
 */
const openLayers: symbol[] = []

export interface ModalProps {
  open: boolean
  title: string
  /** 标题下面那行小字 */
  subtitle?: string
  /** 底部一条。没有就不画那一条边 */
  footer?: ReactNode
  /**
   * 框的宽度档。`sm` 看一眼就关的（详情）· `md` 要读的（确认 / 表单）· `lg` 向导 ·
   * `xl` **整块工作区**（参数编辑器那种"要占大半个窗口"的）·
   * `full` **撑满宿主整块**（遮罩不留内衬、框满、不圆角）—— 参数模态框要在顶栏页签以下
   * 整块内容区里铺开，就是这一档。
   * 不给自由数值 —— 五档已经够，给了数值各处就会长出十种宽度。
   */
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
  /**
   * 框身**不滚**、也不加内衬：交给里面那个自己管滚动的整体（`xl` 档的参数编辑器就是）。
   *
   * 默认那种（`overflow-y:auto` + 内衬）适合"一段内容"，不适合"一个自成一体的工作区"——
   * 后者有自己的头、可滚的中间、钉住的底栏，套进一个外层的滚动容器里会两头打架。
   */
  bodyFill?: boolean
  /**
   * 点遮罩关不关。**默认不关**（作者 2026-10-15：「所有页面的模态框都不应该点击背景就关掉，
   * 可以用 esc」）—— 遮罩是背景，不是取消键：手滑点一下就把框收了（有草稿的那种更糟）。
   * 关有两条明路：**Esc** 与页眉/页脚那颗关闭。传 `true` 才恢复"点背景关"。
   */
  closeOnScrim?: boolean
  /**
   * 遮罩挂在哪个 DOM 节点里（C13 交出 `.shellBody`：标题栏以下整层）。
   * 不给就**原地渲染** —— 遮罩盖住哪儿由最近的定位祖先决定（本组件原来的
   * 约定，A/B 轨都靠它）。给了就 `createPortal` 过去，遮罩与居中都跟着
   * 宿主走，与调用方写在哪一层无关。
   */
  host?: HTMLElement | null
  /**
   * 额外挂在框上的类名（可选）：给"这一屏要比同尺寸档更高一点"的场合用
   * （例如「选择要更新的参数」要一个固定高度，不能跟着 1 项内容塌成一小条）。
   * 只管框身尺寸 —— 宽度档仍由 `size` 决定。
   */
  boxClass?: string
  /**
   * **连标题栏那颗 × 都不摆**（2026-10-11 加，只有跑着那一屏用）。
   *
   * 与 `closeOnScrim={false}` 是配套的一对：那一刻"把这一屏收起来"只有一条明路
   * （后处理那屏的「后台运行」）。摆一颗点了不动的 × 比没有更糟 —— 用户会以为
   * 自己能关，点下去却什么都没发生。
   */
  hideClose?: boolean
  /**
   * **Esc 关不关**（默认关）。
   *
   * 与 `closeOnScrim` 同一类：有草稿语义 / 正在跑的框要传 `false` ——
   * 那种场合"收起来"必须是一条明说的动作（按钮），不能是一记顺手的 Esc。
   */
  closeOnEsc?: boolean
  /** 页眉那枚「关闭」的悬停说明 —— 关闭不总是「放弃」，有草稿语义的稿要写清 */
  closeTitle?: string
  /**
   * **框上面那一层**（可选）。还挂在同一个遮罩里（于是几何 = 内容区、居中不用自己算，
   * z-index 也天然在框之上），但**不摆头尾、不进焦点锁** —— 就是一个盖在框上的层。
   *
   * 用来做"从框里浮起来看大的东西"：图片引导点小图放大就是它（2026-10-16 作者：
   * 「我不希望再弹一个窗出来。我希望就在这个模态框里面实现这个图片的那种放大的感觉，
   *   它就是那种感觉可以点就悬浮出来的感觉……但是没那么突兀，它就是悬浮了出来，居中」）。
   * 这一层里怎么摆由调用方定（背景、留白、点击收起）。
   */
  overlay?: ReactNode
  onClose: () => void
  children: ReactNode
}

export default function Modal({
  open,
  title,
  subtitle,
  footer,
  size = 'md',
  bodyFill = false,
  closeOnScrim = false,
  host = null,
  boxClass,
  closeTitle,
  hideClose = false,
  closeOnEsc = true,
  overlay,
  onClose,
  children,
}: ModalProps) {
  const boxRef = useRef<HTMLDivElement>(null)
  /*
   * 标题栏那颗**动作槽**的落点（`<span>` 元素）。框里的内容可以 portal 到这儿 ——
   * 见 `modalHeadSlot.ts`（「修改历史」就是这么搬到 × 旁边的）。
   */
  const [headSlot, setHeadSlot] = useState<HTMLSpanElement | null>(null)
  /*
   * 标题栏那颗**回执槽**（副标题右边）。框里想往头上塞一句状态（「已写回 xxx.toml」）
   * 就往这儿 portal —— 见 `modalHeadSlot.ts` 的头注。
   */
  const [headNoteSlot, setHeadNoteSlot] = useState<HTMLSpanElement | null>(null)
  /* 关掉之后焦点要还回去，不然键盘用户会掉到文档开头 */
  const returnTo = useRef<HTMLElement | null>(null)

  const closeRef = useRef(onClose)
  closeRef.current = onClose

  /* 这一层在 Esc 栈里的身份（见文件头 `openLayers`）：一个实例一个，重渲染不换 */
  const layerId = useRef(Symbol('modal-layer'))

  useEffect(() => {
    if (!open) {
      const back = returnTo.current
      returnTo.current = null
      /* 元素可能已经不在了（列表重画过），所以要判一下 */
      if (back !== null && document.body.contains(back)) back.focus()
      return
    }
    const active = document.activeElement
    returnTo.current = active instanceof HTMLElement ? active : null
    /* 焦点进框里 —— 打开了却还停在外面，Esc 之外的键都打到背后去了 */
    boxRef.current?.focus()
  }, [open])

  useEffect(() => {
    /* 开着就入栈 —— 就算这一层不吃 Esc（`closeOnEsc=false`），它也该**挡住**下面那层 */
    if (!open) return
    const id = layerId.current
    openLayers.push(id)
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== 'Escape') return
      /* 不是最上面那一层：让给上面那层（它关掉之后自己出栈，下一次才轮到这里） */
      if (openLayers[openLayers.length - 1] !== id) return
      if (!closeOnEsc) return
      closeRef.current()
    }
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('keydown', onKey)
      const at = openLayers.indexOf(id)
      if (at >= 0) openLayers.splice(at, 1)
    }
  }, [open, closeOnEsc])

  if (!open) return null

  /** Tab 锁在框内：到最后一个就回到第一个，反之同理 */
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'Tab') return
    const box = boxRef.current
    if (box === null) return
    const able = [
      ...box.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ]
    if (able.length === 0) return
    const first = able[0]
    const last = able[able.length - 1]
    if (first === undefined || last === undefined) return
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
      return
    }
    if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }

  /** 遮罩那一块。`host` 给了就 portal 过去 —— 其余行为（焦点、Esc、Tab 锁）照旧 */
  const scrim = (
    <div
      className={s.scrim}
      data-size={size}
      role="presentation"
      onMouseDown={(e) => {
        /* 只认落在遮罩自己身上的那一下 —— 从框里开始拖到外面松手不算 */
        if (closeOnScrim && e.target === e.currentTarget) onClose()
      }}
    >
      <div
        ref={boxRef}
        className={boxClass === undefined ? s.box : `${s.box} ${boxClass}`}
        data-size={size}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        <header className={s.head}>
          <span className={s.headText}>
            {/*
             * 标题那一行 = 标题 + **回执槽**（框里 portal 进来的那一句状态）。
             *
             * ★ 2026-10-17 晚：回执从「副标题右边」搬到**标题右边**（作者：
             *   「要不然利用起来标题旁边的空间」「文字长的话会导致换行」）。
             *   原因很实在：副标题那一行已经被文件名占着，回执一长就顶着文件名与
             *   右侧动作簇，窄窗里直接换行 / 压到按钮底下。标题右边那一整片本来是空的。
             *
             * 回执本身永远是**一行 + 省略号**（`white-space: nowrap` 在页面的 `.note` 上），
             * 这里只负责给它一块**能收缩**的地方：`flex: 1 1 auto` + `min-width: 0`。
             */}
            <span className={s.titleRow}>
              <strong className={s.title}>{title}</strong>
              <span ref={setHeadNoteSlot} className={s.headNote} />
            </span>
            {subtitle !== undefined && <span className={s.sub}>{subtitle}</span>}
          </span>
          {/* 标题栏右边：**动作槽**（框里想往头上塞按钮就往这儿 portal）+ 关闭 */}
          <span className={s.headActs}>
            <span ref={setHeadSlot} className={s.headSlot} />
            {/* `hideClose`：这一屏没有"关掉"这回事（要收起来走明路，见那个 prop） */}
            {!hideClose && (
              <button
                type="button"
                className={s.close}
                title={closeTitle ?? '关闭'}
                aria-label="关闭"
                onClick={onClose}
              >
                <Icon name="close" size={16} />
              </button>
            )}
          </span>
        </header>

        <div className={bodyFill ? `${s.body} ${s.bodyFill}` : s.body}>{children}</div>

        {footer !== undefined && <footer className={s.foot}>{footer}</footer>}
      </div>

      {/* 框上面那一层（`overlay`）—— 见 props 里那段：同一个遮罩、压在框之上 */}
      {overlay !== undefined && <div className={s.overlayLayer}>{overlay}</div>}
    </div>
  )

  /* 两个槽的 context 要罩住**框里的内容**（`children` 就在 `scrim` 里），才找得到落点 */
  const tree = (
    <ModalHeadSlotContext.Provider value={headSlot}>
      <ModalHeadNoteSlotContext.Provider value={headNoteSlot}>
        {scrim}
      </ModalHeadNoteSlotContext.Provider>
    </ModalHeadSlotContext.Provider>
  )
  return host !== null ? createPortal(tree, host) : tree
}
