/*
 * 「校准」tab —— **HUD 台面**（G06-10 收编，10-03 作者定稿）。
 *
 * 没有页首行：控件贴四缘 —— 上左步骤条、上右预设 pill（点开右侧抽屉）、
 * 下左可编辑读数、下右动作组；板子占满其余全部。
 *
 *   Z 步   视口锁板子比例（136.219 × 34.019），说明两行贴板下（字号 cqw 跟板等比）
 *   XY 步  方板吃满剩余高居中（板内自带整段说明，不给字）
 *   测试模型  合影 contain 永不裁剪（作者：「无论怎么拉窗口都不应该被裁剪」），
 *          无背景（模糊铺底试过被否：「还是不要背景了」）
 *
 * 读数 = CalibReadings（点文字一样编辑 / Esc 还原）；预设 = pill + 右抽屉
 * （PresetPickerDrawer，与参数页同颗）。壳最小窗 600×500（App.tsx），
 * 下缘在任何宽度一行（容器查询两档收紧）。其余（弹窗 / 状态机）沿用 v029 那套。
 */
import { useCallback, useEffect, useMemo, useState } from 'react'
import CalibPlate from './CalibPlate'
import { VIEW_BOX as Z_VIEW_BOX } from '../../calib/zoffset-calibration.generated'
import { VIEW_BOX as XY_VIEW_BOX } from '../../calib/precise-calibration.generated'
import PlateZoom from './PlateZoom'
import PresetPickerDrawer from '../params/PresetPickerDrawer'
import { Btn } from '../ui/Controls'
import { Modal } from '../ui/Modal'
import { api } from '../../api'
import { activeForSelection, selectionFromActive } from '../home/activeSelection'
import { activateCombo, useActivePreset, useActivePresetReady } from '../state/appState'
import { uidOfFile, useCatalog } from '../home/useCatalog'
import { AXIS_ROWS, NEED_PRESET, Z_LEGEND, Z_TIP, xyHitLabel, zHitLabel } from './calibAxes'
import { useCalibration } from './useCalibration'
import { usePreset } from './usePreset'
import type { Selection } from '../home/MachinePicker'
import CalibReadings from './CalibReadings'
/* 测试模型那一屏的合影：**界面自带素材**（不进 Catalog / Delivery）。
   与首页第五步用的是同一张，从 `src/app/assets/hero/` import */
import heroPile from '../assets/hero/hero_pile.webp'
import heroPile2x from '../assets/hero/hero_pile@2x.webp'
import s from './PageCalib.module.css'

/** 板子的配色跟应用的模式对齐；等应用做了深色模式，这里换成跟随主题的那个值 */
const PLATE_THEME = '浅色'

const EMPTY: Selection = { brand: null, model: null, variant: null }

type Step = 'z' | 'xy' | 'models'

const STEPS: { id: Step; label: string }[] = [
  { id: 'z', label: 'Z 偏移校准' },
  { id: 'xy', label: 'XY 偏移校准' },
  { id: 'models', label: '测试模型' },
]

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

const TEST_MODEL_ID = 'test-models'

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

export default function PageCalib() {
  const [step, setStep] = useState<Step>('z')
  const [sel, setSel] = useState<Selection>(EMPTY)
  /* 「先回『选择机型』……」只在没有基准**成为事实**之后才许出现 ——
     挂载初期选择还在路上（目录/底账没回来），这时候那句提示是错的建议，
     而且数据一到它又得消失，页脚就这么闪一下。 */
  const [noSelection, setNoSelection] = useState(false)

  const catalog = useCatalog()

  /*
   * 对准「正在使用的那一条」（AppState 的 activePreset 格）—— 与首页同一套反填。
   * 底账从唯一客户端订阅：应用 / 撤销 / 删除一发生这里同帧换基准，
   * 不需要回页签对账（第一轮的 selCache + 回页签补丁已拆，见 docs/APP-STATE.md）。
   * 首读没落地前不动选择 —— 那时的 null 是"还没读到"，不是"没有已应用"。
   */
  const activeEntry = useActivePreset()
  const activeReady = useActivePresetReady()
  useEffect(() => {
    if (!activeReady || catalog.machines.length === 0) return
    const next = selectionFromActive(catalog.machines, activeEntry)
    if (next !== null) {
      setSel(next)
    } else {
      /* 底账里没有已应用：残留的选择一并撤掉，别拿旧基准冒充 */
      setSel((prev) => (prev.model === null ? prev : EMPTY))
      setNoSelection(true)
    }
  }, [activeReady, activeEntry, catalog.machines])

  /* 真选上了就撤掉「没有基准」那句 */
  useEffect(() => {
    if (sel.model !== null && sel.variant !== null) {
      setNoSelection(false)
    }
  }, [sel])

  const presetOptions = useMemo(
    () =>
      catalog.presets.flatMap((f) => {
        const uid = uidOfFile(f)
        return uid === null ? [] : [{ uid, name: f.fileName }]
      }),
    [catalog.presets],
  )
  const currentUid =
    sel.model !== null && sel.variant !== null ? `${sel.model}/${sel.variant}` : null

  const preset = usePreset(sel)
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
    resetAxis,
    clearAll,
    commitAll,
  } = useCalibration(preset)

  /* 预设抽屉（与 G06-8 同一颗） */
  const [pickerOpen, setPickerOpen] = useState(false)

  const fileOf = useCallback(
    (machineId: string, versionId: string) => {
      const hit = catalog.presets.find((f) =>
        f.usedByVersions.some((r) => r.machine === machineId && r.version === versionId),
      )
      return hit?.fileName ?? null
    },
    [catalog.presets],
  )

  const currentFileName = currentUid && sel.model !== null && sel.variant !== null
    ? fileOf(sel.model, sel.variant)
    : null

  /*
   * A3：底账正指着当前选中 combo 时，pill 说**底账那份**的名字 ——
   * 应用了「我的文件」就显示我的文件（与预设页横幅、首页同源），不再显示目录底稿。
   * （activeEntry 来自上面的 AppState 订阅。）
   */
  const activeForSel = activeForSelection(activeEntry, sel.model, sel.variant)
  const pillFileName = activeForSel?.fileName ?? currentFileName

  const [opening, setOpening] = useState<{ id: string; name: string } | null>(null)
  const [pending, setPending] = useState<Step | null>(null)
  const [presetAsk, setPresetAsk] = useState<string | null>(null)

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

  /*
   * 抽屉里点一份 = 明确要用这份：落到 sel 之外真写底账（与首页 / 参数页抽屉同一语义；
   * 首页三级浏览不算应用，那边走的是明确的「应用」按钮）。
   * 守则见 activateCombo：底账已命中不动（不顶掉「我的文件」）、应用不了保持原账。
   */
  const applyPreset = useCallback(
    (uid: string) => {
      const [machineId, versionId] = uid.split('/')
      const machine = catalog.machines.find((m) => m.id === machineId)
      const version = machine?.versions.find((v) => v.id === versionId)
      if (machine === undefined || version === undefined) return
      setSel({ brand: machine.brand, model: machine.id, variant: version.id })
      activateCombo(machine.id, version.id, fileOf, activeEntry)
    },
    [catalog.machines, fileOf, activeEntry],
  )

  const pickPreset = useCallback(
    (uid: string) => {
      if (!uid || uid === currentUid) return
      if (dirty) {
        setPresetAsk(uid)
        return
      }
      applyPreset(uid)
    },
    [applyPreset, currentUid, dirty],
  )

  const plate = step === 'models' ? null : PLATE[step]

  const actions = plate ? (
    <div className={s.actions}>
      <span className={s.state} data-on={dirty || savedNote}>
        {dirty ? '未保存' : savedNote ? '已保存' : ''}
      </span>
      {/* 打开模型是这一步的主入口（作者 10-03：绿底给打开模型，预设 pill 退暗） */}
      <Btn variant="accent" onClick={() => setOpening({ id: plate.calibId, name: plate.title })}>
        打开模型
      </Btn>
      <Btn variant="ghost" disabled={!dirty} onClick={clearAll}>
        放弃改动
      </Btn>
      <Btn variant="primary" disabled={!dirty} onClick={commitAll}>
        保存
      </Btn>
    </div>
  ) : null

  const plateView = plate && (
    <PlateZoom resetKey={step} title="" fill mm={plate.mm}>
      <CalibPlate
        model={plate.model}
        theme={PLATE_THEME}
        label={plate.title}
        onPick={step === 'z' ? pickZ : pickXY}
        picked={step === 'z' ? zSelected : xySelected}
        hitLabel={plate.hitLabel}
      />
    </PlateZoom>
  )

  return (
    <div className={s.root}>
      <div className={s.hud}>
        {/*
         * 上缘：步骤条贴左；预设 pill 贴右（「测试模型」那一步换成它的动作按钮）。
         */}
        <div className={s.hudTop}>
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
            <button
              type="button"
              className={pillFileName ? s.presetPill : `${s.presetPill} ${s.presetPillOff}`}
              onClick={() => setPickerOpen(true)}
              title="点击选择预设"
            >
              <span className={s.presetName}>{pillFileName ?? '选择预设'}</span>
              <span className={s.presetSwitch}>切换</span>
            </button>
          )}

          {!plate && (
            <Btn
              variant="primary"
              onClick={() => setOpening({ id: TEST_MODEL_ID, name: '测试模型' })}
            >
              打开测试模型
            </Btn>
          )}
        </div>

        {/*
         * 台面：板子占满其余全部。
         * Z 步视口锁板子比例（136.219 × 34.019）+ 说明两行紧贴板下；
         * XY 方板吃满剩余高居中（板内自带整段说明，不给字）。
         */}
        {plate ? (
          <div className={s.hudBoard} data-step={step}>
            {plateView}
            {step === 'z' && (
              <>
                <p className={s.legend}>{Z_LEGEND}</p>
                <p className={s.tip}>{Z_TIP}</p>
              </>
            )}
          </div>
        ) : (
          <div className={s.models}>
            {/*
             * 合影不裁剪（作者要求：怎么拉窗都完整显示），但也**不要背景**——
             * 模糊铺底的方案试过一版：清晰图自身的白底与模糊层之间总有一条看得见的
             * 边界（板内绿色晕开出画框），作者判了「还是不要背景了」—— 宁可两侧留白。
             * 不裁剪的实现：img 自己做 flex 项（flex:1 + min-height:0），
             * object-fit: contain 的盒子撑满中段，位图在盒内等比缩放，无百分比陷阱
             * （旧版 max-height:100% 在 grid 行里解不开约束，矮窗下被裁的老毛病根治）。
             */}
            <p className={s.lead}>打印这套模型，检查校准结果。</p>
            <img
              className={s.modelsImg}
              src={heroPile}
              srcSet={`${heroPile} 1x, ${heroPile2x} 2x`}
              alt="测试模型"
              draggable={false}
            />
            <p className={s.caption}>鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景</p>
          </div>
        )}

        {/*
         * 下缘：三轴读数（可编辑，复用 G06-8 那颗）贴左；动作组贴右。
         * 放不下整体折行（窄窗读数一行、按钮一行），不再各占一角硬碰。
         */}
        {plate && (
          <div className={s.hudBottom}>
            <CalibReadings
              saved={saved}
              draft={draft}
              view={view}
              activeAxis={plate.axis}
              canType={canPick}
              onType={typeAxis}
              onRevert={revertAxis}
              onReset={resetAxis}
            />
            {!saved && noSelection && <p className={s.note}>{NEED_PRESET}</p>}
            {actions}
          </div>
        )}
      </div>

      {/*
       * 预设抽屉：挂在 .root（定位父级）而不是会滚的 .hud。
       */}
      <PresetPickerDrawer
        open={pickerOpen}
        machines={catalog.machines.map((m) => ({
          id: m.id,
          display: m.display,
          versions: m.versions.map((v) => ({ id: v.id, name: v.name, tag: v.tag ?? null })),
        }))}
        machineId={sel.model ?? ''}
        versionId={sel.variant ?? ''}
        fileOf={fileOf}
        onPick={(m, v) => {
          setPickerOpen(false)
          pickPreset(`${m}/${v}`)
        }}
        onClose={() => setPickerOpen(false)}
      />

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
            换成 {presetOptions.find((o) => o.uid === presetAsk)?.name ?? '另一份预设'} 之后，
            点出来的偏移草稿会作废 —— 草稿是相对上一份预设点出来的增量，
            换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
