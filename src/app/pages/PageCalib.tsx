import { useCallback, useEffect, useState } from 'react'
import CalibPlate from '../components/CalibPlate'
import { VIEW_BOX as Z_VIEW_BOX } from '../../calib/zoffset-calibration.generated'
import { VIEW_BOX as XY_VIEW_BOX } from '../../calib/precise-calibration.generated'
import CalibHead, { PresetLine } from '../components/CalibHead'
import PlateZoom from '../components/PlateZoom'
import { Btn } from '../ui/Controls'
import { Modal } from '../ui/Modal'
import { api } from '../../api'
import type { CalibModel } from '../../api/contract'
import { presetCatalog } from '../../api/mock'
import { brands } from '../constants/machines'
import { AXIS_ROWS, NEED_PRESET, xyHitLabel, zHitLabel } from '../calibAxes'
import { useCalibration } from '../useCalibration'
import { usePreset } from '../usePreset'
import type { Selection } from '../components/MachinePicker'
import s from './PageCalib.module.css'

/** 板子的配色跟应用的模式对齐；等应用做了深色模式，这里换成跟随主题的那个值 */
const PLATE_THEME = '浅色'

const EMPTY: Selection = { brand: null, model: null, variant: null }

type Step = 'z' | 'xy' | 'models'

/** 三步各自的页名、板子、以及对应的校准件 id（后端 calibModels 里的 id） */
const STEPS: { id: Step; label: string }[] = [
  { id: 'z', label: 'Z 偏移校准' },
  { id: 'xy', label: 'XY 偏移校准' },
  { id: 'models', label: '测试模型' },
]

/*
 * 三步各自的板子、页名、校准件 id 与命中读名。
 *
 * `mm` 取自产物自己的 VIEW_BOX（不是手抄的数）：PlateZoom 的 fill 模式拿它把
 * "框还剩多少宽高"换算成 px/mm。手抄过一次，XY 板抄成了 53.545 见方、实际是
 * 53.545 × 54.18，于是板子比框高 1.2%、上下各被裁掉 3px —— 探针量出来才发现。
 */
const PLATE = {
  z: {
    axis: 'z',
    model: 'zoffset',
    title: 'Z 偏移校准板',
    calibId: 'z',
    hitLabel: zHitLabel,
    mm: { w: Z_VIEW_BOX[2], h: Z_VIEW_BOX[3] },
  },
  xy: {
    axis: 'xy',
    model: 'precise',
    title: 'XY 偏移校准板',
    calibId: 'xy',
    hitLabel: xyHitLabel,
    mm: { w: XY_VIEW_BOX[2], h: XY_VIEW_BOX[3] },
  },
} as const

/**
 * 八个测试模型共用同一个 3mf，所以「打开」只有一个对象 —— 传一个固定 id，
 * 页面上也就只给一个按钮，别让人以为每个模型各有一份下载。
 */
const TEST_MODEL_ID = 'test-models'

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

/**
 * 「校准」tab。
 *
 * 与向导第三 / 四 / 五页是**同一套实现**：读数条与预设下拉是同一个 CalibHead，
 * 板子是同一个 CalibPlate，状态机是同一个 useCalibration，模型入口走同一个 api.openModel。
 * 区别只有版面 —— 这里是平铺一列、上面一条步骤条，不套 SlideDeck 的卡片翻页：
 * 从 tab 进来的人是"回来改一个数"，不该被按步骤推着走。
 *
 * 状态与向导那份不互通（各是一个 hook 实例）：在向导里点了一格没保存，
 * 不该让这一页的数也跟着变。
 */
export default function PageCalib() {
  const [step, setStep] = useState<Step>('z')
  const [sel, setSel] = useState<Selection>(EMPTY)

  const { state: preset } = usePreset(sel)
  const {
    saved,
    draft,
    view,
    dirty,
    dirtyAxes,
    savedNote,
    canPick,
    zSelected,
    xySelected,
    pickZ,
    pickXY,
    typeAxis,
    revertAxis,
    clearAll,
    commitAll,
  } = useCalibration(preset)

  /** 校准件清单：给板框标题用（板名以后端那份为准） */
  const [plates, setPlates] = useState<CalibModel[]>([])

  /* 开页就取一次、之后不再变的静态数据 */
  useEffect(() => {
    let alive = true
    api.getCalibModels().then(
      (list) => {
        if (alive) setPlates(list)
      },
      (err: unknown) => console.error('[calib] 取校准件清单失败', err),
    )
    return () => {
      alive = false
    }
  }, [])

  /** 要打开哪一个模型（弹窗里确认之后才真的让壳去开） */
  const [opening, setOpening] = useState<{ id: string; name: string } | null>(null)
  /** 有未保存改动时拦下的那次换步 */
  const [pending, setPending] = useState<Step | null>(null)
  /** 有未保存改动时拦下的那次换预设 */
  const [presetAsk, setPresetAsk] = useState<string | null>(null)

  // ui/Modal 不认 Esc，这里补上：Esc = 取消
  useEffect(() => {
    if (opening === null && pending === null && presetAsk === null) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      setOpening(null)
      setPending(null)
      setPresetAsk(null)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [opening, pending, presetAsk])

  /** 换步：有草稿先问一次（与向导的换页拦截同一条规则） */
  const go = useCallback(
    (next: Step) => {
      if (next === step) return
      if (dirty) {
        setPending(next)
        return
      }
      setStep(next)
    },
    [dirty, step],
  )

  /* 「选预设」= 反填三级选择：一份预设唯一对应一个打印件版本，机型从 预设目录那条自己带的 model 来 */
  const applyPreset = useCallback((variantId: string) => {
    const row = presetCatalog.find((it) => it.variant === variantId)
    if (!row) return
    setSel({ brand: brands[0].id, model: row.model, variant: variantId })
  }, [])

  /* 换预设会把草稿作废（草稿是相对上一份预设点出来的增量），所以先问一次。
     只给「取消 / 放弃改动并换」两条路：换了预设 saved 会被新预设的值覆盖，
     这时候提供「保存并换」是骗人的 —— 存下去的数立刻就被顶掉了 */
  const pickPreset = useCallback(
    (variantId: string) => {
      if (!variantId || variantId === sel.variant) return
      if (dirty) {
        setPresetAsk(variantId)
        return
      }
      applyPreset(variantId)
    },
    [applyPreset, dirty, sel.variant],
  )

  const plate = step === 'models' ? null : PLATE[step]
  const plateInfo = plate ? plates.find((m) => m.id === plate.calibId) : undefined

  /*
   * 这一步的动作组。校准两步里它与三轴读数同一行（读数在左、按钮在右），
   * 「测试模型」那一步没有读数，于是直接挂在页首那一行的右端。
   */
  const actions = plate ? (
    <div className={s.actions}>
      {/* 状态位在按钮左侧、常驻占位：它进出时不该把右边两个按钮顶来顶去 */}
      <span className={s.state} data-on={dirty || savedNote}>
        {dirty ? '未保存' : savedNote ? '已保存' : ''}
      </span>
      {/* 不写文件大小：与向导里那两个「打开模型」一字不差，
          大小对"要不要点"这件事没有影响，写上去只是让按钮忽宽忽窄 */}
      <Btn onClick={() => setOpening({ id: plate.calibId, name: plate.title })}>打开模型</Btn>
      <Btn variant="ghost" disabled={!dirty} onClick={clearAll}>
        放弃改动
      </Btn>
      <Btn variant="primary" disabled={!dirty} onClick={commitAll}>
        保存
      </Btn>
    </div>
  ) : (
    <div className={s.actions}>
      <Btn variant="primary" onClick={() => setOpening({ id: TEST_MODEL_ID, name: '测试模型' })}>
        打开测试模型
      </Btn>
    </div>
  )

  return (
    <div className={s.page}>
      {/*
       * 页首两行：
       *   第一行  步骤条（左） + 预设文件（中）
       *   第二行  三轴读数（左） + 这一步的动作（右，底边与读数对齐）
       *
       * 动作原来压在板框下方靠左，板子一高就被推到折叠线以下，要保存得先往下滚；
       * 读数原来自己一行、按钮又在上一行的右端，中间空一大块。合成这两行之后
       * 既不空、也不随内容高度漂移。
       */}
      <div className={s.top}>
        <div className={s.rail} role="tablist" aria-label="校准步骤">
          {STEPS.map((it) => (
            <button
              key={it.id}
              type="button"
              role="tab"
              aria-selected={it.id === step}
              className={s.tab}
              data-on={it.id === step}
              onMouseDown={noFocus}
              onClick={() => go(it.id)}
            >
              {it.label}
            </button>
          ))}
        </div>

        {plate && (
          /* 预设下拉挪到页首：原来它自己占一整行，而这一行右边本来空着 */
          <div className={s.presetSlot}>
            <PresetLine state={preset} value={sel.variant} onPick={pickPreset} />
          </div>
        )}

        {!plate && actions}
      </div>


      {plate ? (
        <>
          {/* 第二行：读数在左、动作在右。
              读数条自己是 width: fit-content + margin auto（向导那边要居中），
              所以外面套一个 fit-content 的壳，它就落在这一页的左基线上 */}
          <div className={s.headRow}>
            <div className={s.head}>
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={view}
                activeAxis={plate.axis}
                variant={sel.variant}
                onPickPreset={pickPreset}
                canEdit={canPick}
                showPreset={false}
                onType={typeAxis}
                onRevert={revertAxis}
              />
            </div>
            {actions}
          </div>

          <section className={s.plateBox}>
            {/* 板名与缩放手柄是同一行，由 PlateZoom 自己排；
                fill：框吃掉剩余高度、板子按框的剩余宽高连续算尺寸并居中 */}
            <PlateZoom resetKey={step} title={plateInfo?.name ?? plate.title} fill mm={plate.mm}>
              <CalibPlate
                model={plate.model}
                theme={PLATE_THEME}
                label={plate.title}
                onPick={step === 'z' ? pickZ : pickXY}
                picked={step === 'z' ? zSelected : xySelected}
                hitLabel={plate.hitLabel}
              />
            </PlateZoom>
            {/* 没取到预设时点板子不产生读数，这一行是唯一的解释，必须留 */}
            {!saved && <p className={s.note}>{NEED_PRESET}</p>}
          </section>
        </>
      ) : (
        <section className={s.models}>
          <p className={s.lead}>打印这套模型，检查校准结果。</p>
          {/*
           * 一张合影 + 一句话，与向导第五步同一套版面。
           *
           * 原来这里是八张走马灯大卡（编号、标签、缩略图、用时 / 耗材、每张一个按钮）——
           * 可八张卡打开的是同一个 3mf（TEST_MODEL_ID），走马灯页脚自己都这么写着，
           * 那"每张各有一份下载"的暗示是假的；这一屏要做的决定只有一个：要不要打开。
           */}
          <div className={s.hero}>
            <img
              className={s.heroImg}
              src="/models/hero_pile.webp"
              srcSet="/models/hero_pile.webp 1x, /models/hero_pile@2x.webp 2x"
              alt="测试模型"
              draggable={false}
            />
          </div>
          <p className={s.caption}>鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景</p>
        </section>
      )}


      {opening !== null && (
        <Modal
          title="打开模型"
          onClose={() => setOpening(null)}
          foot={
            <>
              <Btn disabled>从本地缓存打开</Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  /* 不等结果、不 catch：没接后端时 api 会抛并在控制台点名（见 src/api/index.ts） */
                  void api.openModel(opening.id)
                  setOpening(null)
                }}
              >
                从云端获取
              </Btn>
            </>
          }
        >
          <p className={s.note}>
            {opening.name} · 本地无缓存文件，即将从云端下载打开。点击后请耐心等待 3mf 打开。
          </p>
        </Modal>
      )}

      {pending !== null && (
        <Modal
          title="有未保存的改动"
          onClose={() => setPending(null)}
          foot={
            <>
              <Btn onClick={() => setPending(null)}>取消</Btn>
              <Btn
                onClick={() => {
                  clearAll()
                  setStep(pending)
                  setPending(null)
                }}
              >
                放弃改动并离开
              </Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  commitAll()
                  setStep(pending)
                  setPending(null)
                }}
              >
                保存并离开
              </Btn>
            </>
          }
        >
          <p className={s.note}>
            偏移改动还没保存：
            {saved && draft
              ? AXIS_ROWS.filter(([, k]) => dirtyAxes.includes(k))
                  .map(([label, k]) => `${label} ${saved[k].toFixed(2)} → ${draft[k].toFixed(2)}`)
                  .join('，')
              : ''}
            。离开前要保存吗？
          </p>
        </Modal>
      )}

      {presetAsk !== null && (
        <Modal
          title="有未保存的改动"
          onClose={() => setPresetAsk(null)}
          foot={
            <>
              <Btn onClick={() => setPresetAsk(null)}>取消</Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  clearAll()
                  applyPreset(presetAsk)
                  setPresetAsk(null)
                }}
              >
                放弃改动并换预设
              </Btn>
            </>
          }
        >
          <p className={s.note}>
            换成 {presetCatalog.find((it) => it.variant === presetAsk)?.name ?? '另一份预设'} 之后，点出来的偏移草稿会作废
            —— 草稿是相对上一份预设点出来的增量，换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
