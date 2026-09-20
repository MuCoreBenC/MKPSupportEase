import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useDeckLayer } from './DeckLayerContext'
import Skeleton from './Skeleton'
import { presetCatalog } from '../../api/mock'
import { AXIS_ROWS, type Axes, type Axis, type AxisView } from '../calibAxes'
import { flipTo, layoutPos, type Pos } from '../flip'
import type { PresetState } from '../usePreset'
/*
 * 样式借用向导那一页的 CSS Module，不另抄一份。
 *
 * CSS Module 允许被多个文件 import —— 类名哈希只认"哪个 css 文件"，不认谁 import 它，
 * 所以这里拿到的 .axes / .axisInput 与向导页里的是**同一批类名**，两处长得一模一样。
 * 更要紧的是那一页里还有一批上下文选择器（`.calib[data-plate='xy'] > .axes`、
 * `[data-layer='exit'] .calibPreset` …）——把这几段 CSS 搬到新文件会让它们哈希不上、
 * 在向导里静悄悄失效，而那些规则正是落位抖动那一轮调出来的，不能动。
 */
import p from '../pages/PageHome.module.css'

/** 竖排 ⇄ 横排的 FLIP 动画时长。试验场里来自调参面板（默认 720ms），产品里取默认值 */
const GLIDE_MS = 720

/**
 * 最近一次横排的布局坐标。
 *
 * 往回折叠时，退出层是一个**新挂载**的节点（SlideDeck 故意不复用 DOM），
 * 它自己的 prev 是空的，量不到"从哪里来" —— 从这里取起点，读数才能沿原路退回竖排，
 * 而不是凭空淡掉。只有退出层读它（见 AxisBar 的 fromLastRow）。
 */
const LAST_ROW_POS: Pos[] = []

/**
 * 三轴读数条：两张校准卡的头部都渲染同一份，三轴一视同仁不做当前页高亮。
 * 读的是完整 draft —— 在 Z 页也能看见 X / Y 那边的未保存改动。
 * 旧值与圆点常驻占位（只切 visibility），改动不会让这一条变宽变高。
 *
 * 两种排布：`row` 是就位后的横排，`col` 是右侧露出卡那条缝里的竖排。
 * 切换时不淡入淡出，而是 FLIP 补间 —— 三个数从缝里的竖排原地展开成横排，往回也走同一条路。
 */
export function AxisBar({
  saved,
  draft,
  orient,
  view,
  fromLastRow,
  activeAxis,
  canType = false,
  onType,
  onRevert,
}: {
  saved: Axes | null
  draft: Axes | null
  orient: 'col' | 'row'
  view: AxisView
  /** 允许拿 LAST_ROW_POS 当起点。只有退出层给 true */
  fromLastRow: boolean
  /** 这一页正在校哪一轴。给了就把另外那两（一）轴压小；不给则三轴一样大 */
  activeAxis?: 'z' | 'xy'
  /**
   * 这一份读数能不能直接打字改。false 就是三轴全只读（缝里的露出卡、退出层、还没取到预设）。
   * 形态不跟着变 —— 只读与可写都是同一个输入框，只是只读的那个不亮边框、不进 Tab 序。
   *
   * 不分轴：三轴在平面层上都能改，「保存」也早就不分页了（见 useCalibration 的 commitAll）。
   */
  canType?: boolean
  onType?: (axis: Axis, raw: string) => void
  onRevert?: (axis: Axis) => void
}) {
  const ref = useRef<HTMLDListElement>(null)
  const prev = useRef<{ orient: 'col' | 'row'; pos: Pos[] } | null>(null)
  /*
   * 正在打字的那一轴 + 它的原始输入串。
   *
   * 不能直接把 draft 格式化回输入框：打到 "0.5" 时受控值会被 toFixed(2) 改写成 "0.50"，
   * 光标跳到末尾、再按 5 就成了 "0.505"。所以聚焦期间显示这一份原文，失焦就交回 draft。
   */
  const [buf, setBuf] = useState<{ k: Axis; text: string } | null>(null)

  /*
   * 输入框里那份原文只在"它就是当前真相"时有效。
   *
   * 点板子会走 CalibPlate 的 onMouseDown preventDefault（它不想让 SVG 抢焦点）——
   * 副作用是输入框**不会失焦**，于是草稿已经被点选盖掉、框里却还显示着手输的那串数字。
   * 「放弃改动 / 保存」也一样：状态回退了，框里还留着旧原文。
   * 所以这里盯着 draft：一旦这一轴的真相与原文不再是同一个数，就把原文丢掉。
   * 只在原文能解析成数时判断 —— 打到中间态（`-`、`0.`）时别把人家正在敲的东西抢走。
   */
  useEffect(() => {
    if (!buf || !draft) return
    const n = Number(buf.text)
    if (buf.text.trim() === '' || !Number.isFinite(n)) return
    if (n.toFixed(2) !== draft[buf.k].toFixed(2)) setBuf(null)
  }, [buf, draft])

  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return

    const items = Array.from(el.children) as HTMLElement[]
    const pos = items.map(layoutPos)

    // 自己有上一帧就用自己的；退出层没有，用全局记下的那份横排坐标
    const before =
      prev.current ??
      (fromLastRow && orient === 'col' && LAST_ROW_POS.length === items.length
        ? { orient: 'row' as const, pos: LAST_ROW_POS.slice() }
        : null)

    prev.current = { orient, pos }
    if (orient === 'row') {
      LAST_ROW_POS.length = 0
      LAST_ROW_POS.push(...pos)
    }

    if (!before || before.orient === orient) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    /*
     * 时间窗要跟卡片自己的动画对齐，否则会看出是两件事：
     *
     * - 展开（col → row）：卡片是「0~60% 滑入、60~100% 白底长大」，位移贴前一段
     * - 收起（row → col）：卡片是「0~45% 白底缩小、45~100% 滑走」，位移贴后一段
     *
     * 之前收起也从 0 开始，于是读数先自己跑到左边、卡片才滑走 ——「先移过去再收起来」。
     */
    const folding = before.orient === 'row' && orient === 'col'
    const dur = GLIDE_MS * (folding ? 0.55 : 0.6)
    const delay = folding ? GLIDE_MS * 0.45 : 0

    items.forEach((item, i) => {
      const from = before.pos[i]
      if (from) flipTo(item, from, pos[i], dur, delay)
    })
  }, [draft, fromLastRow, orient, saved, view])

  return (
    <dl className={p.axes} ref={ref} data-orient={orient}>
      {AXIS_ROWS.map(([label, k]) => {
        const changed = view === 'value' && saved !== null && draft !== null && draft[k] !== saved[k]
        /* 非当前轴压小：Z 页压 X / Y，XY 页压 Z。两页都还看得见三个数，
           但"这一页在校哪个"不用读眉标题也知道 */
        const minor = activeAxis ? (activeAxis === 'z' ? k !== 'z' : k === 'z') : false
        return (
          <div className={p.axis} key={k} data-changed={changed} data-minor={minor}>
            <dt className={p.axisLabel}>
              {label}
              <i className={p.dot} data-on={changed} aria-hidden={!changed} />
              {/*
                原始值跟在标签右侧，不再占值行。
                常驻占位、只切 visibility：标签行的宽高在"改没改过"之间一个像素都不动，
                值行也就不会因为多出「旧 →」那一段而变宽（实测原来一点就 +60px，
                窄窗里把「mm」挤出卡片）。
              */}
              <span className={p.axisFrom} data-on={changed} aria-hidden={!changed}>
                原 {saved ? saved[k].toFixed(2) : '0.00'}
              </span>
            </dt>
            <dd className={p.axisValue}>
              {view === 'value' && draft ? (
                <>
                  {/*
                    有值就只有这一种形态 —— 输入框，三层都一样。
                    原来这里有两条互斥分支：能改的渲染 input、不能改的渲染 `<b>` 粗体文本，
                    于是同一个数在卡片滑进来的那一段是紧凑的粗体（内容宽 33.6-43.7），
                    落位换成平面层又变成固定 3.6em 的输入框（50.4-61.2）——「mm」跟着右移十几像素，
                    看起来就是"到位之后换了一种题"。

                    不能改时用 readOnly 而不是 disabled：disabled 会压成半透明（原来那条
                    `.axisInput:disabled { opacity: .5 }`），那又是第三种长相。
                    tabIndex -1 是第二层保险 —— 露出卡与退出层本来就被 SlideDeck 标了 inert。
                  */}
                  <input
                    className={p.axisInput}
                    type="number"
                    step="0.01"
                    min={-50}
                    max={50}
                    inputMode="decimal"
                    aria-label={`${label}（毫米）`}
                    readOnly={!canType}
                    tabIndex={canType ? undefined : -1}
                    value={buf?.k === k ? buf.text : draft[k].toFixed(2)}
                    onChange={
                      canType
                        ? (e) => {
                            setBuf({ k, text: e.target.value })
                            onType?.(k, e.target.value)
                          }
                        : undefined
                    }
                    onBlur={canType ? () => setBuf(null) : undefined}
                    onKeyDown={
                      canType
                        ? (e) => {
                            if (e.key === 'Enter') e.currentTarget.blur()
                            if (e.key === 'Escape') {
                              setBuf(null)
                              onRevert?.(k)
                              e.currentTarget.blur()
                            }
                          }
                        : undefined
                    }
                  />{' '}
                  mm
                </>
              ) : view === 'loading' ? (
                <Skeleton label="正在取预设" />
              ) : (
                <span className={p.axisBlank}>—</span>
              )}
            </dd>
          </div>
        )
      })}
    </dl>
  )
}

/**
 * 校准页上方那一行「这是哪一份预设」—— v0.0.20 起可以直接在这里换。
 *
 * 不是只读文本了：一个整行宽的下拉，选项就是预设目录里的三份预设（见 src/api/mock.ts）。
 * 选了之后由页面反填「机型 + 打印件版本」，所以在校准页换预设等于把前面那两级也改了。
 * 名字未知时下拉停在那条占位项上，位置固定、不跳版。
 */
const PRESET_OPTIONS = presetCatalog.map(({ variant, name, path }) => ({ variant, name, path }))

/**
 * 「预设文件」那一行。
 *
 * 导出给「校准」tab 用 —— 那一页把它挪到了页首那一行（步骤条右边那块空地），
 * 所以要能脱离 CalibHead 单独渲染。向导里仍由 CalibHead 自己带着它。
 */
export function PresetLine({
  state,
  value,
  onPick,
}: {
  state: PresetState
  /** 当前选中的打印件版本 id —— 一份预设对应一个版本 */
  value: string | null
  onPick: (variantId: string) => void
}) {
  const path = state.status === 'ready' ? state.preset.path : undefined

  return (
    <div className={p.presetLine} title={path}>
      <span className={p.presetLineLabel}>预设文件</span>
      {state.status === 'waiting' ? (
        <Skeleton width="9em" label="正在连接云端" />
      ) : (
        <select
          className={p.presetSelect}
          value={value ?? ''}
          aria-label="预设文件"
          onChange={(e) => onPick(e.target.value)}
        >
          <option value="" disabled>
            — 选择一份预设 —
          </option>
          {PRESET_OPTIONS.map((o) => (
            <option key={o.variant} value={o.variant}>
              {o.name}
            </option>
          ))}
        </select>
      )}
    </div>
  )
}

/**
 * 校准页的头部：一行预设下拉 + 三轴读数条。向导的两张校准卡与「校准」tab 都用这一个。
 *
 * 在向导里它同时被渲染在当前页和"下一页的露出卡 / 正在退回的卡"上，几处要长得不一样：
 *
 * - 当前页：一行预设文件名 + 横排读数条，居中，跟板子一起看
 * - 露出卡 / 退出卡：缝里只有版面 20% 宽，横排 470px 塞不进去 —— 只留竖排的三个数，
 *   文件名不跟过去（它在缝里会折成三四行）
 *
 * 推入一开始（phase === 'in'）就切回横排，位移交给 AxisBar 里的 FLIP 补 ——
 * 落位与当前页的静态版面完全重合，卡片层被换成平面层那一帧不会跳。
 * 往回折叠走的是同一条路径的反向：退出层拿 LAST_ROW_POS 当起点。
 *
 * 不在 deck 里时（「校准」tab 那一页）useDeckLayer 给的默认值是 plane/idle，
 * 于是直接走"当前页"那一支：横排、可打字，没有缝、没有退出层。
 */
export default function CalibHead({
  saved,
  draft,
  preset,
  view,
  activeAxis,
  variant,
  onPickPreset,
  canEdit,
  showPreset = true,
  onType,
  onRevert,
}: {
  saved: Axes | null
  draft: Axes | null
  preset: PresetState
  view: AxisView
  activeAxis: 'z' | 'xy'
  variant: string | null
  onPickPreset: (variantId: string) => void
  /** 取到预设才有基准可改（与"点板子不产生读数"同一条规则） */
  canEdit: boolean
  /**
   * 要不要带上「预设文件」那一行。
   * 「校准」tab 传 false —— 它把预设下拉挪到页首那一行自己渲染（见 PageCalibV023）。
   * 向导不传：那边的预设行还参与卡片层的淡入淡出与缝里的藏显，不能拆出去。
   */
  showPreset?: boolean
  onType: (axis: Axis, raw: string) => void
  onRevert: (axis: Axis) => void
}) {
  const { layer, phase } = useDeckLayer()
  const offPlane = layer !== 'plane'
  const peek = offPlane && phase !== 'in'

  /* 能不能打字只看"在不在当前平面层 + 有没有基准"，与轴无关：
     三轴随处可改，「保存」也不分页（见 useCalibration 的 commitAll）。
     缝里的露出卡与退出层是只读的同一个输入框 —— 焦点不该跑到那条 20% 宽的缝里的卡上，
     但形态不变，落位时不会"换一种题"。 */
  const canType = !offPlane && canEdit

  return (
    <>
      {/* 三层都渲染，显不显形交给 CSS：缝里直接藏（脱离文档流，不影响 FLIP 终点），
          退回时淡出（直接卸载会"啪"一下没了） */}
      {showPreset && (
        <div className={p.calibPreset}>
          <PresetLine state={preset} value={variant} onPick={onPickPreset} />
        </div>
      )}
      <AxisBar
        saved={saved}
        draft={draft}
        view={view}
        orient={peek ? 'col' : 'row'}
        fromLastRow={layer === 'exit'}
        activeAxis={activeAxis}
        canType={canType}
        onType={onType}
        onRevert={onRevert}
      />
    </>
  )
}
