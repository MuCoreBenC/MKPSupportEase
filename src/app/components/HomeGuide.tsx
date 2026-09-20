import s from './HomeGuide.module.css'

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

interface Step {
  no: string
  title: string
  hint: string
  /** 有对应页面才可点；没有的做成静态列，不做点了没反应的假按钮 */
  to?: number
}

export interface StepRailProps {
  /** 第一 / 三 / 四 / 五步各自的目标页序号 */
  pickIndex: number
  zIndex: number
  xyIndex: number
  modelIndex: number
  onGo: (index: number) => void
}

/**
 * 横排五步流程条：序号 + 标题 + 一句说明，列间用短连线接起来。
 * 第 3、4 步包在 `.pair` 里 —— 宽了它是 `display: contents`（五列），
 * 挤了它自己占一格、内部两格并排（序号都留着，中间一条竖虚线看得出是两步）。
 */
export function StepRail({ pickIndex, zIndex, xyIndex, modelIndex, onGo }: StepRailProps) {
  const steps: Step[] = [
    { no: '1', title: '选择设备', hint: '品牌 / 机型 / 打印件版本', to: pickIndex },
    { no: '2', title: '确认偏移值', hint: '核对群里的最新参考值' },
    { no: '3', title: '校准 Z 轴', hint: '打 Z 板判断哪格涂胶均匀', to: zIndex },
    { no: '4', title: '校准 XY 轴', hint: '打 XY 板判断哪条涂胶均匀', to: xyIndex },
    { no: '5', title: '打印测试模型', hint: '打一件确认最终效果', to: modelIndex },
  ]

  /** 一格的内容。连线由 data-link 决定（第一步没有），与 DOM 兄弟关系无关 */
  const cell = (step: Step) => {
    const body = (
      <>
        <span className={s.head}>
          <b className={s.no}>{step.no}</b>
          <span className={s.title}>{step.title}</span>
        </span>
        <span className={s.hint}>{step.hint}</span>
      </>
    )
    return step.to === undefined ? (
      <span className={s.cell}>{body}</span>
    ) : (
      <button
        type="button"
        className={s.cell}
        data-go="true"
        onMouseDown={noFocus}
        onClick={() => onGo(step.to as number)}
      >
        {body}
      </button>
    )
  }

  const [s1, s2, s3, s4, s5] = steps

  return (
    <ol className={s.rail}>
      <li className={s.item}>{cell(s1)}</li>
      <li className={s.item} data-link="true">
        {cell(s2)}
      </li>
      <li className={s.pair}>
        <span className={s.item} data-link="true">
          {cell(s3)}
        </span>
        <span className={s.item} data-link="true" data-tight="true">
          {cell(s4)}
        </span>
      </li>
      <li className={s.item} data-link="true">
        {cell(s5)}
      </li>
    </ol>
  )
}


/**
 * 三张待选的小卡：序号 + 这一级叫什么 + 一条空占位。
 *
 * 刻意不写任何具体值。这块插图出现在"还没选机型"的首页上，
 * 卡里一旦写着「拓竹」「A1 mini」，看的人会以为已经替他选好了 —— 它其实只是候选位。
 * 所以第三行是一条灰条：一眼就知道那里待填，也不用再解释一遍每一级是什么意思
 * （解释在第二页的选择器里，写两遍是复读）。
 */
const CARDS = [
  { no: '1', label: '品牌' },
  { no: '2', label: '机型' },
  { no: '3', label: '打印件版本' },
] as const

/**
 * 未选机型时的插图：三张小卡收成一条斜梯。
 *
 * v0.0.17 这里还有一只沿斜梯走下来的线条小猫。撤掉了 —— 造型没做到「在页面里生活的
 * 小角色」那个水准，停在简笔画阶段，留着不如不留。连带撤掉的还有它那套 hover 触发、
 * 位置轨迹动画和卡片下沉，以及为它在上方预留的约 61px 站位。
 */
export function HomeCardStack() {
  return (
    <div className={s.stage} role="img" aria-label="尚未选择机型">
      {CARDS.map((c, i) => (
        <span key={c.no} className={s.slot} data-slot={i + 1}>
          <span className={s.card}>
            <b className={s.cardNo}>
              {c.no} {c.label}
            </b>
            <span className={s.cardBlank} aria-hidden="true" />
          </span>
        </span>
      ))}
    </div>
  )
}



