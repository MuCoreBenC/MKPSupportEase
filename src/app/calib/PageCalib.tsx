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
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import CalibPlate from './CalibPlate'
import { VIEW_BOX as Z_VIEW_BOX } from '../../calib/zoffset-calibration.generated'
import { VIEW_BOX as XY_VIEW_BOX } from '../../calib/precise-calibration.generated'
import PlateZoom from './PlateZoom'
import PresetPickerDrawer from '../params/PresetPickerDrawer'
import type { PresetPickerRow } from '../params/PresetPickerDrawer'
import { useLocalPickerRows, useLocalPresetFiles } from '../presets/localPickerRows'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import { STORAGE } from '../../api/storageKeys'
import { Btn } from '../ui/Controls'
/* 模态框**只有一套**（2026-10-10 作者：两套"不行……强制去统一只能用一种"）——
   稿内那套 `ui/Modal` 退场，一律用通用居中框 */
import { Modal } from '../../components/modal'
import { api, errorText } from '../../api'
import type { CalibModel } from '../../api'
import { activeForSelection, selectionFromActive } from '../home/activeSelection'
import { activateFile, useActivePreset, useActivePresetReady } from '../state/appState'
import { useCatalog } from '../home/useCatalog'
import {
  AXIS_ROWS,
  NEED_PRESET_HOW,
  NEED_PRESET_TITLE,
  Z_LEGEND,
  Z_TIP,
  xyHitLabel,
  zHitLabel,
} from './calibAxes'
import { useCalibration } from './useCalibration'
import { usePreset } from './usePreset'
import type { Selection } from '../home/MachinePicker'
import CalibReadings from './CalibReadings'
/* 测试模型那一屏的合影：**界面自带素材**（不进 Catalog / Delivery）。
   与首页第五步用的是同一张，从 `src/app/assets/hero/` import */
import heroPile from '../assets/hero/hero_pile.webp'
import heroPile2x from '../assets/hero/hero_pile@2x.webp'
import { useIsDark } from '../state/appState'
import { slicerArgsOf, useSlicerAppPref, useSlicerPick, useSlicerPref } from '../shared/slicer'
import { SlicerPickView } from '../shared/SlicerPickView'
import s from './PageCalib.module.css'

/*
 * 板子的配色**跟着应用主题走**（2026-10-12 深色模式那一刀落地）。
 *
 * 底下那块 SVG 的配色是生成器里写死的两套（`src/calib/*.generated.ts` 的
 * `THEMES['浅色' | '深色']`），**读不到 CSS 变量** —— 所以这里要的是解析完的
 * 真值（`useIsDark()` = 用户那一档 + 系统深浅），不能只靠 `<html data-theme>`。
 */
const PLATE_THEME_OF = { dark: '深色', light: '浅色' } as { dark: string; light: string }

const EMPTY: Selection = { brand: null, model: null, variant: null }

type Step = 'z' | 'xy' | 'models'

/*
 * 步骤条文案（2026-10-10 作者：「改一下文案叫做 Z 校准，XY 校准」）——
 * 短一截是为了**窄窗不折行**：四字的「Z 偏移校准 / XY 偏移校准」在最小窗里
 * 会折成"Z 偏移 / 校准"两行（抽卡页 02 号的窄窗试出来的）。「测试模型」不长，不动。
 */
const STEPS: { id: Step; label: string }[] = [
  { id: 'z', label: 'Z 校准' },
  { id: 'xy', label: 'XY 校准' },
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

/*
 * ---------- 药丸宽度的弹簧动画（2026-10-11 第八轮） ----------
 *
 * 作者：「还是一瞬间就变宽变窄了，没有 iOS 那种弹跳、果冻的效果」。
 *
 * CSS 做不出这个效果，两轮都栽在同一个点上：
 *   - `width: fit-content` 不是可插值长度 → `transition: width` 无效；
 *   - `transition: max-width` 是空转（两态都是 100%，没有变化量）；
 *   - `width: calc-size(fit-content, size)` 理论上可插值，实测补间没跑起来，
 *     而且它只能做**单调**补间 —— 而"果冻"要的是 **overshoot（冲过头再回弹）**。
 *
 * 所以走 JS：把自然宽**量成 px**，再解一条**弹簧轨迹**交给 Web Animations API。
 *
 * 灵动岛那个"果冻"感其实是两件事叠出来的（少了任何一个都死板）：
 *   ① **弹簧**：走 stiffness/damping 的阻尼谐振子，带 ≈4.6% 的 overshoot；
 *   ② **squash & stretch**：横向拉得快时纵向压一点、回弹时鼓一点 ——
 *      只有宽度弹性、没有这个挤压，看着就是"死板地变宽"。
 * 用 `width` 做动画会触发布局，但药丸只有几十像素宽的子树，代价可忽略。
 */

/*
 * 弹簧用 **stiffness / damping / mass** 描述，而不是 duration —— 这是灵动岛
 * 那种"物理感"的来源（oksr/react-dynamic-island 的设计说明里也是这么定的：
 * "Springs are parameterized by stiffness and damping rather than duration"）。
 * 好处是**可被中断**：连续点格子时能从当前位置与速度接着走，不会跳回起点。
 *
 * 阻尼比 ζ = damping / (2·√(stiffness·mass)) ≈ 0.71 —— 欠阻尼但不过分，
 * 理论 overshoot ≈ 4.6%，正是灵动岛那种"轻轻弹一下就停"的量。
 * （ζ 再小就晃、再大就没有那一下弹；0.7 附近是它的甜点。）
 */
const SPRING = { stiffness: 600, damping: 32, mass: 1 }

/*
 * 积分步长取 **1/120**（比显示的 60fps 更细）—— 半隐式欧拉在 1/60 下有明显的
 * 数值阻尼，实测把 4.3% 的 overshoot 吃到只剩 1.1%，弹跳几乎没了；
 * 细化到 1/120 才回到物理值。多出来的帧只进关键帧数组，不做任何额外计算。
 */
const DT = 1 / 120

/** 静止判定：位置与速度都足够小就收工 */
const REST_X = 0.0005
const REST_V = 0.0005

/** 帧数上限（120 帧 = 1s），防数值不收敛时兜底 */
const MAX_FRAMES = 120

/**
 * 挤压幅度：横向速度带来的纵向形变（**squash & stretch**，果冻感的真正来源）。
 * 变宽时纵向压一点、回弹时纵向鼓一点 —— 灵动岛展开就是这一下。
 */
const SQUASH = 0.04

/** 变化小于这个像素就不值得播动画（字体渲染的亚像素抖动） */
const MIN_DELTA = 0.5

/** 阻尼谐振子的一个采样点。x 是相对进度（0→1），v 是速度 */
interface SpringSample {
  t: number
  x: number
  v: number
}

/** 数值积分求解弹簧（半隐式欧拉），返回从静止到静止的整条轨迹 */
function solveSpring(): SpringSample[] {
  const { stiffness: k, damping: c, mass: m } = SPRING
  const out: SpringSample[] = [{ t: 0, x: 0, v: 0 }]
  let x = 0
  let v = 0
  let t = 0
  for (let i = 0; i < MAX_FRAMES; i++) {
    const a = (-k * (x - 1) - c * v) / m
    v += a * DT
    x += v * DT
    t += DT
    out.push({ t, x, v })
    if (Math.abs(x - 1) < REST_X && Math.abs(v) < REST_V) break
  }
  /* 尾巴钉死：保证最后一帧正好落在 x=1、v=0，避免停在一个抖动的中间值上 */
  const last = out[out.length - 1]
  if (last.x !== 1 || last.v !== 0) out.push({ t: last.t + DT, x: 1, v: 0 })
  return out
}

/**
 * 播一段灵动岛式的宽度弹性：宽度走弹簧轨迹，同时按**横向速度**给纵向一点挤压。
 */
function islandWidth(el: HTMLElement, from: number, to: number): Animation {
  const path = solveSpring()
  const delta = to - from
  const total = path[path.length - 1].t

  /*
   * 正负速度**分别**归一化：回弹那一段的速度只有峰值的一成左右，
   * 若按同一个峰值归一化，回弹时几乎不鼓（实测只到 1.002），看着还是"单向压扁"。
   * 各自归一之后两个方向都吃满 SQUASH，才有真正的"伸缩"果冻感。
   */
  let peakOut = 0 // 变宽方向的最大速度
  let peakBack = 0 // 回弹方向的最大速度（正值）
  for (const p of path) {
    if (p.v > peakOut) peakOut = p.v
    if (-p.v > peakBack) peakBack = -p.v
  }

  const frames = path.map((p) => {
    /* 归一化到 [-1, 1]：+1 = 正在全速变宽，-1 = 正在全速回弹 */
    const s = p.v >= 0 ? p.v / (peakOut || 1) : p.v / (peakBack || 1)
    return {
      offset: p.t / total,
      width: `${from + delta * p.x}px`,
      /* 变宽 → 纵向压；回弹 → 纵向鼓 */
      transform: `scaleY(${1 - SQUASH * s})`,
    }
  })

  return el.animate(frames, { duration: total * 1000, fill: 'forwards' })
}

interface PageCalibProps {
  /**
   * ★ 实验场专用（产品里没有这个口子）：平铺画廊要"三步各摆一格" —— 给一个起始步骤
   * （见 `gallery/cells.tsx`；搬回产品时删掉）。
   */
  /*
   * ★ 活口（2026-10-14 标记）：**当前没有任何调用点传它**（`App.tsx` 那里直接
   * `<PageCalib />`），于是下面 `useState(initialStep ?? 'z')` 恒落在 `'z'`。
   * 留着它是"跳进来就直接开在某一轴"那个口（例如从别处点「去校准 XY」），
   * 要删之前先确认这个需求确实没有了。
   */
  initialStep?: Step
}

export default function PageCalib({ initialStep }: PageCalibProps) {
  /* 板子配色跟随应用主题（见文件头那条注释）：真值在 useIsDark() 里解析好了 */
  const isDark = useIsDark()
  const plateTheme = PLATE_THEME_OF[isDark ? 'dark' : 'light']
  const [step, setStep] = useState<Step>(initialStep ?? 'z')
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
    canSave,
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

  /*
   * 抽屉的行 = **本机真有**的预设（`presets-local/` 那一份扫描结果摊出来）。
   * ★ 与页面上的 `catalog`（目录）**不是同一份数据**，别互相替换。
   */
  const pickerRows = useLocalPickerRows(useLocalPresetFiles(), catalog.machines)

  /* 预设抽屉的宽度：拖内缘改宽 + 三态吸附（与参数页历史抽屉同一套，见 `shared/useDrawerWidth`） */
  const pickerDrag = useDrawerWidth({
    storageKey: STORAGE.clientPresetPickerW,
    side: 'right',
    snap: 320,
    min: 240,
    max: 560,
    label: '预设列表',
  })

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
   * 应用了「本地文件」就显示本地文件（与预设页横幅、首页同源），不再显示目录底稿。
   * （activeEntry 来自上面的 AppState 订阅。）
   */
  const activeForSel = activeForSelection(activeEntry, sel.model, sel.variant)
  const pillFileName = activeForSel?.fileName ?? currentFileName

  const [opening, setOpening] = useState<{ id: string; name: string } | null>(null)
  /* 打开模型用哪个切片软件：默认在设置页那格；模态框里可临时换（勾「仅这一次」就不写回） */
  const [defaultSlicer, setDefaultSlicer] = useSlicerPref()
  /* 设置页「其他应用…」指定的那个 —— 选着它时随 openModel 把路径带给壳（见 slicerArgsOf） */
  const [slicerApp] = useSlicerAppPref()
  const slicerPick = useSlicerPick(defaultSlicer, opening !== null)
  const [pending, setPending] = useState<Step | null>(null)
  /** 待确认"要不要丢掉草稿再换"的那一行（null = 没在问） */
  const [presetAsk, setPresetAsk] = useState<PresetPickerRow | null>(null)

  /* 药丸宽度弹簧用的三样东西（见下面那段 `useLayoutEffect`） */
  const barRef = useRef<HTMLDivElement | null>(null)
  const barText = useRef('') // 上一次的文案，用来判断"内容变了没"
  const barWidth = useRef<number | null>(null) // 当前受控宽度（px）；null = 还没量过
  const barAnim = useRef<Animation | null>(null) // 正在播的宽度动画

  /*
   * 「打开模型」那一个模态框的**真实状态**（2026-10-09）：本机有没有缓存由**盘**说了算
   * （`getCalibModels` 查下载区与旧缓存），不再是写死的「本地无缓存，即将从云端下载」。
   */
  const [modelInfo, setModelInfo] = useState<CalibModel | null>(null)
  const [modelBusy, setModelBusy] = useState(false)
  const [modelErr, setModelErr] = useState<string | null>(null)

  useEffect(() => {
    if (opening === null) {
      setModelInfo(null)
      setModelErr(null)
      setModelBusy(false)
      return
    }
    let alive = true
    api.getCalibModels().then(
      (list) => {
        if (alive) setModelInfo(list.find((m) => m.id === opening.id) ?? null)
      },
      (e: unknown) => {
        if (alive) setModelErr(errorText(e))
      },
    )
    return () => {
      alive = false
    }
  }, [opening])

  /** 打开盘上那份（下载区或旧缓存）。没有缓存时按钮是灰的 —— 不给点了必报错的按钮 */
  const openCachedModel = () => {
    if (opening === null) return
    setModelBusy(true)
    api.openModel(opening.id, slicerArgsOf(slicerPick, setDefaultSlicer, slicerApp)).then(
      () => {
        setModelBusy(false)
        setOpening(null)
      },
      (e: unknown) => {
        setModelBusy(false)
        setModelErr(errorText(e))
      },
    )
  }

  /** 取回（走交付下载管道）再打开。两步都可能失败，失败就在弹窗里照实说，不吞 */
  const fetchAndOpenModel = () => {
    if (opening === null || modelInfo === null) return
    setModelBusy(true)
    setModelErr(null)
    /* 参数在点击这一刻就定下（含"写回默认"那一步）—— 与下面下载那一步的成败无关 */
    const slicerArgs = slicerArgsOf(slicerPick, setDefaultSlicer, slicerApp)
    void (async () => {
      try {
        await api.downloadCatalogFile(modelInfo.fileName)
        await api.openModel(opening.id, slicerArgs)
        setModelBusy(false)
        setOpening(null)
      } catch (e) {
        setModelBusy(false)
        setModelErr(errorText(e))
      }
    })()
  }

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
   * 守则见 activateCombo：底账已命中不动（不顶掉「本地文件」）、应用不了保持原账。
   */
  /* 占位：applyPreset 已并入 applyRow（按文件那条路），这里不再单独留一份 */

  /**
   * **换到抽屉里的那一份**（按文件，不是按 combo）。
   *
   * ★★ 2026-10-14：原先是 `pickPreset(uid)`，开头 `uid === currentUid` 就退货 ——
   *   而同一 combo 下的另一份（复制出来的那份）uid 完全相同 ⇒ 点了没反应。
   *   现在整条按"哪一份文件"走：`row.fileName` + `row.ref`。
   *   有未保存改动仍然先问一句（草稿是相对上一份的增量，换了基准就没意义）。
   */
  const applyRow = useCallback(
    (row: PresetPickerRow) => {
      if (dirty) {
        setPresetAsk(row)
        return
      }
      /* 换了机型 / 版本要先把浏览位置挪过去（与旧路径同一个动作） */
      if (row.machineId !== sel.model || row.versionId !== sel.variant) {
        const machine = catalog.machines.find((m) => m.id === row.machineId)
        const version = machine?.versions.find((v) => v.id === row.versionId)
        if (machine !== undefined && version !== undefined) {
          setSel({ brand: machine.brand, model: machine.id, variant: version.id })
        }
      }
      activateFile(row.fileName, row.ref)
    },
    [catalog.machines, dirty, sel.model, sel.variant],
  )

  const plate = step === 'models' ? null : PLATE[step]

  /*
   * 下缘的常驻件（a05 第十四轮，作者从抽卡页 14 号挑的）：气泡里只留
   * **主入口「打开模型」** —— 改动是临时状态，交给气泡外那条 `changeBar`（见下）；
   * 原来那套「未保存 + 放弃改动 + 保存」三件套从气泡里撤走（窄窗不再折行，
   * 下缘永远只有"读数 + 打开模型"这一行）。
   */
  const actions = plate ? (
    <span className={s.actions}>
      {/* 打开模型是这一步的主入口（作者 10-03：绿底给打开模型，预设 pill 退暗） */}
      <Btn variant="accent" onClick={() => setOpening({ id: plate.calibId, name: plate.title })}>
        打开模型
      </Btn>
    </span>
  ) : null

  /*
   * 气泡下面那条「改动提示条」（a05 第十四轮）——
   * 有未保存的改动 / 刚保存过时出现：把**改了哪几轴**写清楚（`Z 4.00 → 4.01`，
   * 比"未保存"三个字有信息量），右端是「放弃 / 保存」；保存成功后它变成
   * 安静的一句回执，2.2 秒自己走（`savedNote` 那套照旧）。
   *
   * 2026-10-10（作者）：**没选机型时的那句提示也走这条**（原来只是气泡里一行灰字，
   * 存在感不够）。提示条不一定是黑底 —— `data-tone` 分档：
   * 不写 = 黑（改动这种"要你行动"的事）、`hint` = 中性底 + 琥珀点（"还差一步"的提示）。
   */
  const changeBar = plate ? (
    dirty || savedNote ? (
      <div ref={barRef} className={s.changeBar} data-saved={!dirty}>
        {dirty ? (
          <>
            <span>未保存的改动</span>
            <span className={s.changeDot}>·</span>
            {/*
             * 各轴之间不用「，」也不挤成一行（2026-10-11 第五轮，作者：
             * 「文案改一下，不要用 / 分开或者其他的分开」）—— 这里是**一格一个**，
             * 用胶囊自己的内衬和竖线分家：胶囊变宽变窄由 `.changeBar` 的
             * `width` 过渡动画接着（见 CSS）。
             */}
            <span className={s.changeList}>
              {saved && draft
                ? AXIS_ROWS.filter(([, k]) => draft[k] !== saved[k]).map(([, k]) => (
                    <span key={k} className={s.changeAxis}>
                      <span className={s.changeAxisName}>{k.toUpperCase()}</span>
                      <span className={s.changeFrom}>{saved[k].toFixed(2)}</span>
                      <span className={s.changeArrow}>→</span>
                      <span className={s.changeTo}>{draft[k].toFixed(2)}</span>
                    </span>
                  ))
                : null}
            </span>
            <span className={s.changeSp} />
            <button type="button" className={s.changeGhost} onClick={clearAll}>
              放弃
            </button>
            <button type="button" className={s.changeSave} disabled={!canSave} onClick={commitAll}>
              保存
            </button>
          </>
        ) : (
          <>
            <span>已保存</span>
            <span className={s.changeDot}>·</span>
            {/* 2026-10-13：原来是「改动已写进『本地预设』」—— 「本地预设」那套说法
                整条退役（作者：「本地这份、云端一份这些相关的都去掉」），只说结果 */}
            <span className={s.changeList}>改动已经写进预设文件</span>
          </>
        )}
      </div>
    ) : noSelection ? (
      /*
       * 2026-10-10（作者）：这条与上面那条**同一个模子** —— 主句 + · + 补充 +
       * 右端动作，只是颜色换成中性 + 开头一颗琥珀点（见 CSS 的 `data-tone='hint'`）。
       * 「选择预设」开的还是右上角那颗 pill 的抽屉：选完这一条自己就没了。
       */
      <div ref={barRef} className={s.changeBar} data-tone="hint">
        <span className={s.hintDot} aria-hidden />
        <span>{NEED_PRESET_TITLE}</span>
        <span className={s.changeDot}>·</span>
        <span className={s.changeList}>{NEED_PRESET_HOW}</span>
        <span className={s.changeSp} />
        <button type="button" className={s.changeAction} onClick={() => setPickerOpen(true)}>
          选择预设
        </button>
      </div>
    ) : null
  ) : null

  /*
   * 药丸的**宽度弹簧**（2026-10-11 第八轮）—— 见文件头 `springWidth()` 那段注释。
   *
   * 做法：每次渲染后量出药丸的**自然内容宽**（px），与上次记下的宽度比较；
   * 只要文字变了、宽度就会变，于是播一段带 overshoot 的弹簧关键帧。
   *
   * ★ 量宽必须用**离屏克隆**，不能就地 `style.width='max-content'` ——
   *   上一版就是就地量，量完把宽度设成某个 px 值（比如比自然宽小 1px）的那一帧，
   *   `flex-wrap` 就把「未保存的改动」折成了两行（作者截图抓到的）。
   *   克隆三件套：**不进文档流**（absolute）、`visibility: hidden`（看不见）、
   *   `pointer-events: none`；量完立刻 remove。
   *   克隆上把几何与外观**全写成内联**（除了 width）—— 这样它不依赖任何 CSS 类，
   *   也就不会受 `.changeBar` 那些响应式规则影响，量出来的就是真自然宽。
   *
   * 用 `textContent` 当变化信号而不是依赖数组：文案的任何一处改动都算（轴数增减、
   * 数值变化、"已保存"回执），不用逐个把状态列进依赖，也不会漏。
   * 读 `textContent` 不触发强制布局，所以每次渲染跑一次很便宜。
   *
   * 首次出现不播宽度动画（没有 `from`）—— 出现时只淡入、宽度直接是终值，
   * 符合作者「不要出现才移动」那条要求。
   */
  useLayoutEffect(() => {
    const el = barRef.current
    if (el === null) {
      /* 药丸不在了：清掉记忆，下次出现当作首次 */
      barText.current = ''
      barWidth.current = null
      barAnim.current?.cancel()
      barAnim.current = null
      return
    }

    const text = el.textContent ?? ''
    /* 文案没变 → 宽度也不会变，直接收工 */
    if (text === barText.current) return
    barText.current = text

    /* 量自然宽：离屏克隆（见上面注释，绝不碰可见那个的宽度） */
    const cs = getComputedStyle(el)
    const rect = el.getBoundingClientRect()
    const clone = el.cloneNode(true) as HTMLElement
    /* 把几何 / 外观全写成内联，克隆就不再依赖任何 CSS 类与容器查询 —— 量到的是真自然宽 */
    const GEOM = [
      'display', 'alignItems', 'flexWrap', 'gap', 'font', 'letterSpacing', 'color',
      'padding', 'border', 'borderRadius', 'background', 'boxShadow',
    ] as const
    for (const prop of GEOM) {
      const v = cs[prop as keyof CSSStyleDeclaration]
      if (typeof v === 'string') clone.style[prop as 'font'] = v
    }
    clone.style.position = 'absolute'
    clone.style.left = '-99999px'
    clone.style.top = '0'
    clone.style.margin = '0'
    clone.style.width = 'max-content'
    clone.style.maxWidth = 'none'
    clone.style.visibility = 'hidden'
    clone.style.pointerEvents = 'none'
    clone.style.animation = 'none'

    document.body.appendChild(clone)
    const natural = clone.getBoundingClientRect().width
    clone.remove()

    /* 顶到台面的内衬（对应 CSS 的 `max-width: 100%`，窄窗不顶出页边） */
    const host = el.parentElement
    const hostCs = host === null ? null : getComputedStyle(host)
    const avail =
      host === null || hostCs === null
        ? natural
        : host.clientWidth -
          (parseFloat(hostCs.paddingLeft) || 0) -
          (parseFloat(hostCs.paddingRight) || 0)
    const to = Math.max(Math.min(natural, avail), 0)
    const from = barWidth.current

    if (from === null || rect.width === 0 || Math.abs(to - from) < MIN_DELTA) {
      /* 首次出现 / 宽度没变：直接落到终值，不播动画 */
      el.style.width = `${to}px`
      barWidth.current = to
      return
    }

    /* 起点用屏上**此刻**的宽度（不是记忆值）—— 中途改别的轴时不会跳回旧值 */
    el.style.width = `${rect.width}px`

    const reduce =
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches
    if (reduce) {
      el.style.width = `${to}px`
      barWidth.current = to
      return
    }

    const anim = islandWidth(el, rect.width, to)
    barAnim.current = anim
    barWidth.current = to
    /* 动画收尾后把宽度钉成终值，再撤掉动画 —— 否则 fill:forwards 会一直压着 inline width */
    void anim.finished.then(
      () => {
        if (barAnim.current !== anim) return /* 已经被下一次动画顶掉了 */
        el.style.width = `${to}px`
        /* 挤压是 transform，收尾要清掉 —— 否则 fill:forwards 会把 scaleY 一直留着 */
        el.style.transform = ''
        anim.cancel()
        barAnim.current = null
      },
      () => {
        /* cancel 会 reject：什么都不用做 */
      },
    )
  })

  const plateView = plate && (
    <PlateZoom resetKey={step} fill mm={plate.mm}>
      <CalibPlate
        model={plate.model}
        theme={plateTheme}
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
         * 上缘（2026-10-10 改版：预设**融进步骤条** —— 作者从抽卡页挑的 02 号）——
         * 一条胶囊轨道里，前三个是步骤、一条细竖线之后是当前预设（点它开抽屉）；
         * 「切换」两个字退役（作者：「也不需要切换两个字」）。
         * 「测试模型」那一步没有预设可挂：轨道照旧三段，右端是它的动作按钮。
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

          {/*
           * 预设从轨道里**搬出来，飘到上缘右端**（作者：「没对齐」）——
           * 原来它是轨道第四段，左边「测试模型」、右边「A1-fastv3.3.toml」，
           * 两个胶囊左右参差、谁也没跟谁对齐；现在步骤条只住三个步骤、钉在左缘，
           * 预设这颗钉在右缘，整条上缘两端各一个锚点，和它的下缘（读数左 /
           * 动作右）同一条刻度。
           */}
          {plate && (
            <button
              type="button"
              className={s.presetSeg}
              data-off={pillFileName === null}
              title={pillFileName ?? '选择预设'}
              onMouseDown={noFocus}
              onClick={() => setPickerOpen(true)}
            >
              <span className={s.presetName}>{pillFileName ?? '选择预设'}</span>
            </button>
          )}
        </div>

        {/*
         * 台面：板子占满其余全部。
         * Z 步视口锁板子比例（136.219 × 34.019）+ 说明两行紧贴板下；
         * XY 方板吃满剩余高居中（板内自带整段说明，不给字）。
         */}
        {plate ? (
          <div className={s.hudBoard} data-step={step}>
            {/*
             * 悬浮药丸（2026-10-11 第四轮，作者：「圈起来的要贴着下对齐……
             * svg 可以上移，缩小一点的，提前预留下面位置给悬浮药丸」）——
             *
             *   1. 台面**常驻**在底缘留一条（`.hudBoard` 的 `padding-bottom` 恒等于
             *      `--shiftbar-h`，见 `.root`）；板子算尺寸时就已减掉它，
             *      出现/消失**一个像素都不动**（旧版是"出现时才让位"，缩身与浮条
             *      各走各的动画 —— 那就是"一出现就缩放 + 移动"）；
             *   2. Z 步整块板**顶对齐**（`.hudBoard[data-step='z']` 的 `flex-start`）：
             *      板与说明上移，让出来的空当全落到台面底端，正是药丸的位置；
             *   3. 药丸是文档流里的一行 + `margin-top: auto` → **贴住台面下缘**，
             *      离下方气泡永远是那条约 14px 的常驻间隙（不跑板子上去）。
             *
             * 具体几何全在 `.root` / `.hudBoard` / `.changeBar` 三处（见 CSS）。
             */}
            {changeBar}
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
             *
             * ★ a05（抽卡 M3 落地，作者：「上面的打印这套模型，检查校准结果。文字就不需
             * 要了」）：那句说明**去掉**、图注搬进下缘气泡 —— 这一屏就只剩图，
             * 文字与动作全在下缘，三个 tab 的下缘从此长得一样。
             */}
            <img
              className={s.modelsImg}
              src={heroPile}
              srcSet={`${heroPile} 1x, ${heroPile2x} 2x`}
              alt="测试模型"
              draggable={false}
            />
          </div>
        )}

        {/*
         * 下缘：三轴读数（可编辑，复用 G06-8 那颗）贴左；动作组贴右。
         * 放不下整体折行（窄窗读数一行、按钮一行），不再各占一角硬碰。
         * 改动提示条不在这里 —— 已上移进 `.hudBoard` 的预留格（见上面那段注释）。
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
            {actions}
          </div>
        )}
        {/*
         * 测试模型步的下缘（a05，抽卡 M3 落地）：**同一颗气泡**（`.hudBottom` 同款）——
         * 左边图注、右边「打开模型」（绿边那颗，与另两屏同款）。
         * 这一屏没有读数、没有改动，气泡就这两件；三屏的下缘从此长得一样。
         */}
        {!plate && (
          <div className={s.hudBottom}>
            <p className={s.caption}>鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景</p>
            <span className={s.actions}>
              <Btn
                variant="accent"
                onClick={() => setOpening({ id: TEST_MODEL_ID, name: '测试模型' })}
              >
                打开模型
              </Btn>
            </span>
          </div>
        )}
      </div>

      {/*
       * 预设抽屉：挂在 .root（定位父级）而不是会滚的 .hud。
       */}
      <PresetPickerDrawer
        open={pickerOpen}
        /* ★ 2026-10-14：只列**本机真有**的（原来列目录那 12 个组合，点哪个都换不过去） */
        rows={pickerRows}
        /*
         * 在用那一份的**行键**（含文件名）。
         *
         * ★★ 2026-10-14 修（作者截图：抽屉勾在 `…copy.toml`，顶部 pill 却写
         *   `A1_MINI-fastv3.3.toml`）：这里原来写 `preset.preset.name` —— 而 `preset`
         *   来自 `usePreset(sel)`，它读的是**目录那份 + 本机副本**，**不是底账**。
         *   于是同一屏上"当前是哪一份"出现了**两个不同源的判据**（pill 读底账、
         *   抽屉读 usePreset），两个源一对不上就打架 —— 正是"同一件事别用两个
         *   不同源的判据"那条铁律（2026-10-11 报告页列表点不动的同一个根）。
         *   现在两处都取**底账**（`activeForSel.fileName`，与上面 pill 同源）。
         */
        currentKey={
          sel.model !== null && sel.variant !== null && activeForSel !== null
            ? `${sel.model}/${sel.variant}/${activeForSel.fileName}`
            : null
        }
        onUse={(row) => {
          setPickerOpen(false)
          /* ★ 走 applyRow（按文件那条路）—— 见它的注释 */
          applyRow(row)
        }}
        onClose={() => setPickerOpen(false)}
        drag={pickerDrag}
      />

      {opening !== null && (
        <Modal
          open
          size="sm"
          title="打开模型"
          onClose={() => setOpening(null)}
          footer={
            <>
              <Btn disabled={modelBusy || !modelInfo?.ready} onClick={openCachedModel}>
                从缓存打开
              </Btn>
              <Btn variant="primary" disabled={modelBusy || modelInfo === null} onClick={fetchAndOpenModel}>
                从云端获取
              </Btn>
            </>
          }
        >
          {/* 用哪个打开（默认在设置页「切片」那格；勾「仅这一次」就不写回） */}
          <SlicerPickView
            value={slicerPick.value}
            once={slicerPick.once}
            onChange={slicerPick.setValue}
            onOnceChange={slicerPick.setOnce}
          />
          <p className={s.note}>
            {modelErr !== null
              ? modelErr
              : modelBusy
                ? `${opening.name} · 正在处理，请稍候…`
                : modelInfo === null
                  ? `${opening.name} · 正在看本机有没有缓存…`
                  : modelInfo.ready
                    ? `${opening.name} · 本机已有缓存（${modelInfo.size ?? '大小未知'}），可以直接打开`
                    : `${opening.name} · 本机还没有缓存文件，「从云端获取」会先取回再打开（请耐心等待 3mf 打开）`}
          </p>
        </Modal>
      )}

      {pending !== null && (
        <Modal
          open
          size="sm"
          title="有未保存的改动"
          onClose={() => setPending(null)}
          footer={
            <>
              <Btn onClick={() => setPending(null)}>取消</Btn>
              <Btn
                onClick={() => {
                  clearAll()
                  setStep(pending)
                  setPending(null)
                }}
              >
                不保存并离开
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
          open
          size="sm"
          title="有未保存的改动"
          onClose={() => setPresetAsk(null)}
          footer={
            <>
              <Btn onClick={() => setPresetAsk(null)}>取消</Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  clearAll()
                  /* ★ 与抽屉那颗「使用」走**同一条**路（按文件，不是按 combo） */
                  applyRow(presetAsk)
                  setPresetAsk(null)
                }}
              >
                不保存并切换
              </Btn>
            </>
          }
        >
          <p className={s.note}>
            换成 {presetAsk.fileName} 之后，点出来的偏移草稿会作废 ——
            草稿是相对上一份预设点出来的增量，换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
