/*
 * 首页 —— 从 v029/pages/PageMachineV029 整份搬来的那一页（A31 上看到的就是它）。
 *
 * 这一轮的目的是**找回原样**，所以只做了三件事：
 *   1. 改名：文件名 / 组件名 / 引用的每一个件与 CSS Module 加 A44 后缀。
 *   2. 改 import：全部指向 A44 内部（`../calib/*`、`../ui/*`、`./*`），
 *      `src/calib/*.generated` 那份板子坐标是共享资产，照旧直接 import。
 *   3. 删掉 `src/dev/devStore` 那处调试钩子：`useDevState().fadeMs` 换成常量 FADE_MS。
 *
 * 数据（T10 起）：三级选择与「文件」那一级都消费**文件体系**（`useCatalog` 的
 * getMachines + getPresetFiles；取件与三轴走 `usePreset`）—— 手编表退场。
 * 作者点名的效果：「A1 就是 A1（A1.toml），P1S 的版本就是 LITE」。
 * 版面、动画、注释一行没动。
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from 'react'
import SlideDeck, { type DeckHandle, type Sheet } from './SlideDeck'
import MachinePicker, { type Option, type Selection } from './MachinePicker'
import { api, errorText, uiErrorOf } from '../../api'
import type { CalibModel, PostProcessCommand, UiError } from '../../api'
import { useLocale, useT, type StringKey } from '../../i18n'
import { bi } from '../shared/dataLabel'
import { activeForSelection, selectionFromActive } from './activeSelection'
import { notify } from '../notify/notify'
import { activateCombo, activateFile, appStateMutated, useActivePreset } from '../state/appState'
import { deliveryMutated, useDeliveryRevision } from '../state/deliveryState'
import { usePresetUsage } from '../state/usePresetUsage'
import { uidOfFile, useCatalog } from './useCatalog'
import { useBundleFiles } from './useBundleFiles'
import PresetStack from './PresetStack'
import CalibPlate from '../calib/CalibPlate'
import PresetPickerDrawer from '../params/PresetPickerDrawer'
import { useLocalPickerRows, useLocalPresetFiles } from '../presets/localPickerRows'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import { StaleCatalogHelp } from '../shared/StaleCatalogHelp'
import { isStaleCatalogError } from '../shared/staleCatalog'
import { STORAGE } from '../../api/storageKeys'
import HeroFade from './HeroFade'
import CardFrame from './CardFrame'
import CalibHead from '../calib/CalibHead'
import PlateZoom from '../calib/PlateZoom'
import { HomeCardStack, StepRail } from './HomeGuide'
import CopyAction from './CopyAction'
/* 取后处理命令走全站那一个出口（见 `loadCommand` 那段注释：只是反馈方式仍用页面底部气泡） */
import { postProcessCommand } from '../postprocess/copyScript'
import { Btn } from '../ui/Controls'
/* 模态框**只有一套**（2026-10-10 作者：两套"不行……强制去统一只能用一种"）——
   稿内那套 `ui/Modal` 退场，一律用通用居中框 */
import { HeadNote, Modal } from '../../components/modal'
import type { Density } from '../../hooks/useDensity'
import { useDevDefaults } from './devDefaults'
import { pickArt } from './heroArt'
import { useArtLayers } from './useArtLayers'
import {
  AXIS_ROWS,
  Z_LEGEND_KEY,
  Z_TIP_KEY,
  axisText,
  xyHitLabel,
  zHitLabel,
} from '../calib/calibAxes'
import { useCalibration } from '../calib/useCalibration'
import { usePreset } from '../calib/usePreset'
/* 第五步那张合影：**界面自带素材**（不进 Catalog / Delivery）——
   取图那一步（浅 / 深两张位图）收在 `heroPileSource` 里，见它的文件头 */
import { heroPileSource } from '../assets/hero/heroPile'

import { useIsDark } from '../state/appState'
import { slicerArgsOf, useSlicerAppPref, useSlicerPick, useSlicerPref } from '../shared/slicer'
import { SlicerPickView } from '../shared/SlicerPickView'
import p from './PageHome.module.css'

const EMPTY: Selection = { brand: null, model: null, variant: null }

/*
 * 板子的配色**跟着应用主题走**（2026-10-12 深色模式那一刀落地）——
 * 同校准页：那块 SVG 的配色是生成器里写死的两套，读不到 CSS 变量，要真值。
 */
const PLATE_THEME_OF = { dark: '深色', light: '浅色' } as { dark: string; light: string }

/** 第五步要打开的那份测试模型（后端 `getCalibModels` 认的 id） */
const TEST_MODEL_ID = 'test-models'

/** 两张校准卡在 sheets 里的位置：按页拦截换页要认它们 */
const CALIB_Z_INDEX = 2
const CALIB_XY_INDEX = 3
const CALIB_MODEL_INDEX = 4

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

/** 版本角标：契约给的是中文 tag，组件只收三档色调 —— 没配过的 tag 不给 tone（走 muted） */
const TAG_TONE: Record<string, Option['badgeTone']> = {
  推荐: 'rec',
  热门: 'hot',
  最新: 'new',
}

function Rows({ items }: { items: [string, string][] }) {
  return (
    <dl className={p.rows}>
      {items.map(([label, value]) => (
        <div className={p.row} key={label}>
          <dt className={p.label}>{label}</dt>
          <dd className={p.value}>{value}</dd>
        </div>
      ))}
    </dl>
  )
}

interface PageHomeProps {
  density: Density
}

export default function PageHome({ density }: PageHomeProps) {
  /* 这一页所有用户能读到的字都走字典（2026-10-26） */
  const tr = useT()
  const locale = useLocale()
  /* 板子配色跟随应用主题（见文件头那条注释）：真值在 useIsDark() 里解析好了 */
  const isDark = useIsDark()
  const plateTheme = PLATE_THEME_OF[isDark ? 'dark' : 'light']
  /* 第五步那张合影（深浅各一张位图，见 heroPile.ts）：记忆化是为了 `sheets` 那个
     useMemo 的依赖稳定 —— 直接调的话每渲染一个新对象，卡片面就会跟着重算 */
  const pile = useMemo(() => heroPileSource(isDark), [isDark])
  const deckRef = useRef<DeckHandle>(null)

  /* 分级揭示的淡入时长：面板里调（产品仓里是常量 FADE_MS） */
  const { fadeMs } = useDevDefaults()
  const [sel, setSel] = useState<Selection>(EMPTY)

  const [openModel, setOpenModel] = useState<{ id: string; name: string } | null>(null)
  /* 「打开测试模型」弹窗的**真实状态**（2026-10-09）：本机有没有缓存由盘说了算 */
  const [modelInfo, setModelInfo] = useState<CalibModel | null>(null)
  const [modelBusy, setModelBusy] = useState(false)
  /* 打开模型那一步的错：留**整条**（人话 + 稳定码）—— 内容区那块"怎么办"按码出现
     （见 StaleCatalogHelp 抬头：目录过期要人重启，文件真坏要人重下，只有码分得清） */
  const [modelErr, setModelErr] = useState<UiError | null>(null)
  /* 打开模型用哪个切片软件：默认在设置页那格；模态框里可临时换（勾「仅这一次」就不写回） */
  const [defaultSlicer, setDefaultSlicer] = useSlicerPref()
  /* 设置页「其他应用…」指定的那个 —— 选着它时随 openModel 把路径带给壳（见 slicerArgsOf） */
  const [slicerApp] = useSlicerAppPref()
  const slicerPick = useSlicerPick(defaultSlicer, openModel !== null)
  const [pending, setPending] = useState<{ from: number; to: number } | null>(null)
  /* 预设抽屉要换一份、但本页有未保存草稿时，先把要换的那一份记下来等确认。
     （入口是抽屉的 `onUse` 里那个 `if (dirty)` —— 2026-10-14 之前还有"校准卡里的
     预设下拉"那一条，那条随 `PresetLine` 一起删了。） */
  const [presetAsk, setPresetAsk] = useState<string | null>(null)
  /* 预设抽屉（与「校准」tab 同一颗）：pill 点开，从左缘出 */
  const [pickerOpen, setPickerOpen] = useState(false)
  // 保存 / 放弃之后自己发起的那一次跳转不该再被拦
  const bypass = useRef(false)

  /* ---------- 文件体系：三级清单与「文件」那一级都从这里来（T10） ---------- */
  const catalog = useCatalog()
  /* 品牌的显示名（机型的 `brand` 存的是品牌显示名）—— 英文界面走 `nameEn` */
  const brandLabel = useCallback(
    (id: string): string => {
      const b = catalog.brands.find((x) => x.name === id)
      return b === undefined ? id : bi(b.name, b.nameEn ?? undefined, locale)
    },
    [catalog.brands, locale],
  )

  /*
   * T9/T10 联动（预设页 → 首页）：对准「正在使用的那一条」（AppState 的 activePreset 格）——
   * 预设页应用了哪一份，这里三级选择就反填成哪一台。active 的 machineId / versionId
   * 与选择器是**同一套 id**（不再有映射表）。
   * 底账从唯一客户端订阅（`useActivePreset`）：应用 / 撤销 / 删除一发生，
   * 这里同帧换基准 —— 不需要回页签对账（第一轮的补丁已拆，见 docs/APP-STATE.md）。
   */
  const activeEntry = useActivePreset()
  useEffect(() => {
    if (catalog.machines.length === 0) return
    const next = selectionFromActive(catalog.machines, activeEntry)
    if (next !== null) setSel(next)
  }, [activeEntry, catalog.machines])

  // ---------- 预设：选哪一份由 sel 定；取件、等待、失败三态都在 usePreset 里 ----------
  const preset = usePreset(sel)

  /*
   * A3：显示层与底账对齐。sel 已经按底账反填（上面那个 effect），activeForSel 拿底账
   * —— 显示层用它判断"正在使用的这份是否就是当前选中 combo"。具体取值在下面的
   * displayName / displayPath（要等 presetInfo / presetName 算完）。
   */
  const activeForSel = activeForSelection(activeEntry, sel.model, sel.variant)

  // ---------- 偏移：已保存的一份 + 板上点出来 / 手输出来的草稿 ----------
  /* 整块状态机搬进了 useCalibration —— 向导这几页与「校准」tab 用同一份实现。
     这里只解构成原来那些名字，JSX 与换页拦截一行没动 */
  const calib = useCalibration(preset)
  const {
    saved,
    draft,
    limits,
    dirty,
    dirtyAxes,
    savedNote,
    canPick,
    canSave,
    view: axisView,
    zSelected,
    xySelected,
    pickZ,
    pickXY,
    typeAxis,
    revertAxis,
    resetAxis,
    clearAll,
    commitAll,
  } = calib

  /* 「打开测试模型」弹窗：盘上有没有缓存查一次（下载区 + 旧缓存），失败照实说 */
  useEffect(() => {
    if (openModel === null) {
      setModelInfo(null)
      setModelErr(null)
      setModelBusy(false)
      return
    }
    let alive = true
    api.getCalibModels().then(
      (list) => {
        if (alive) setModelInfo(list.find((m) => m.id === openModel.id) ?? null)
      },
      (e: unknown) => {
        if (alive) setModelErr(uiErrorOf(e))
      },
    )
    return () => {
      alive = false
    }
  }, [openModel])

  /** 打开盘上那份（下载区或旧缓存）。没有缓存时按钮是灰的 —— 不给点了必报错的按钮 */
  const openCachedModel = () => {
    if (openModel === null) return
    setModelBusy(true)
    api.openModel(openModel.id, slicerArgsOf(slicerPick, setDefaultSlicer, slicerApp)).then(
      () => {
        setModelBusy(false)
        setOpenModel(null)
      },
      (e: unknown) => {
        setModelBusy(false)
        setModelErr(uiErrorOf(e))
      },
    )
  }

  /** 取回（走交付下载管道）再打开。两步都可能失败，失败就在弹窗里照实说，不吞 */
  const fetchAndOpenModel = () => {
    if (openModel === null || modelInfo === null) return
    setModelBusy(true)
    setModelErr(null)
    /* 参数在点击这一刻就定下（含"写回默认"那一步）—— 与下面下载那一步的成败无关 */
    const slicerArgs = slicerArgsOf(slicerPick, setDefaultSlicer, slicerApp)
    void (async () => {
      try {
        await api.downloadCatalogFile(modelInfo.fileName)
        await api.openModel(openModel.id, slicerArgs)
        setModelBusy(false)
        setOpenModel(null)
      } catch (e) {
        setModelBusy(false)
        setModelErr(uiErrorOf(e))
      }
    })()
  }

  /** 有未保存的改动就把换页拦下（不分页 —— 三轴在哪一页都能改），等弹窗里给答案 */
  const canLeave = useCallback(
    (from: number, to: number) => {
      if (bypass.current) {
        bypass.current = false
        return true
      }
      if (!dirty) return true
      setPending({ from, to })
      return false
    },
    [dirty],
  )

  const leaveTo = useCallback((target: number) => {
    bypass.current = true
    setPending(null)
    deckRef.current?.jumpTo(target)
  }, [])

  /**
   * 再点一次已选项 = 取消该级；取消或改选都清空下游。
   *
   * ★ 2026-10-26（作者：「我希望能真的取消在用的状态，现在还是在用的」）：
   *   取消**不只是**把这一级的选择清掉 —— 三级不齐就没有「正在使用」这回事，
   *   底账里那条也得真的撤掉（`clear_active_preset`，与预设页那颗「撤销应用」同一条）。
   *   只撤「底账正指着当前这台」的那一条：底账指着别处时（还在浏览），不动人家的账。
   */
  const pick = useCallback(
    (level: keyof Selection, id: string) => {
      const clearing = sel[level] === id
      setSel((prev) => {
        const next = prev[level] === id ? null : id
        if (level === 'brand') return { brand: next, model: null, variant: null }
        if (level === 'model') return { ...prev, model: next, variant: null }
        return { ...prev, variant: next }
      })
      if (clearing && activeForSel !== null) {
        void api
          .clearActivePreset()
          .then(() => appStateMutated())
          .catch((e: unknown) => {
            /* 撤不掉就如实说，不留一个「以为撤了」的界面 */
            notify({
              kind: 'bad',
              title: tr('home.unapply.failed'),
              text: errorText(e),
              ttl: 6000,
            })
          })
      }
    },
    [sel, activeForSel, tr],
  )

  /* ---------- 三级的清单：数据源是契约（getMachines），不再是手编表 ---------- */
  const brandOptions = useMemo<Option[]>(() => {
    const out: Option[] = []
    for (const m of catalog.machines) {
      if (!out.some((o) => o.id === m.brand)) out.push({ id: m.brand, label: brandLabel(m.brand) })
    }
    return out
  }, [catalog.machines, brandLabel])

  const modelOptions = useMemo<Option[]>(
    () => catalog.machines.map((m) => ({ id: m.id, label: m.display })),
    [catalog.machines],
  )

  const variantOptions = useMemo<Option[]>(() => {
    const machine = catalog.machines.find((m) => m.id === sel.model)
    return (machine?.versions ?? []).map((v) => ({
      id: v.id,
      label: v.name,
      badge: v.tag == null ? undefined : bi(v.tag, v.tagEn ?? undefined, locale),
      /* 色调仍按**原角标**查表（`TAG_TONE` 的键是中文那一册） */
      badgeTone: TAG_TONE[v.tag ?? ''],
      note: v.description,
    }))
  }, [catalog.machines, sel.model, locale])

  /* 「文件」这一级：仓库里的 9 份 MKP，每份带 机型/版本 倒查（校准页的预设下拉用它） */
  const presetOptions = useMemo(
    () =>
      catalog.presets.flatMap((f) => {
        const uid = uidOfFile(f)
        return uid === null ? [] : [{ uid, name: f.fileName }]
      }),
    [catalog.presets],
  )


  /*
   * 抽屉的行 = **本机真有**的预设（`presets-local/` 那一份扫描结果摊出来）。
   * ★ 与页面上的 `catalog`（目录）**不是同一份数据**，别互相替换。
   */
  const pickerRows = useLocalPickerRows(useLocalPresetFiles(), catalog.machines)

  /* 预设抽屉的宽度：拖内缘改宽 + 三态吸附（与参数页历史抽屉同一套，见 `shared/useDrawerWidth`） */
  const pickerDrag = useDrawerWidth({
    storageKey: STORAGE.clientPresetPickerW,
    side: 'left',
    snap: 320,
    min: 240,
    max: 560,
    label: tr('home.picker.list'),
  })

  /* 抽屉每一项自己的文件名（T11）—— 与「校准」tab 同一颗查询 */
  const fileOf = useCallback(
    (machineId: string, versionId: string) => {
      const hit = catalog.presets.find((f) =>
        f.usedByVersions.some((r) => r.machine === machineId && r.version === versionId),
      )
      return hit?.fileName ?? null
    },
    [catalog.presets],
  )

  /*
   * 抽屉里点一份 = 明确要用这份（与「改预设先问一句」同一入口）：落到 sel 之外，
   * 真写底账 —— 四个页面同账。首页三级的**浏览选择不算应用**（作者 2026-10-06：
   * 「不是说点了它就是应用」，应用走下面那颗明确的按钮）。
   * 守则见 activateCombo：底账已命中不动（不顶掉「本地文件」）、应用不了保持原账。
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

  /*
   * ★ 这里原来有一个 `pickPreset` ——"在校准卡里换预设、先把草稿问一次"，2026-10-14
   * 与 `PresetLine`（那个已删的预设下拉）一起删了：它的**唯一调用点**是 `CalibHead`
   * 的 `onPickPreset`，下拉撤掉之后再没人调它。
   *
   * ⚠️ 那个 `presetAsk` 确认框**留着** —— 它不是同一条链上的：抽屉换预设
   * （`onUse` 里 `if (dirty)` 那一支）还在用它。
   */

  const brandName = sel.brand === null ? null : brandLabel(sel.brand)
  const modelNode = catalog.machines.find((m) => m.id === sel.model)
  const modelName = modelNode?.display ?? null
  const variantName = modelNode?.versions.find((v) => v.id === sel.variant)?.name ?? null
  /* 大图：图位分层 品牌 → 机型 → 版本（版本缺则回落机型），URL 全部来自台账 path */
  const art = useMemo(
    () => pickArt(sel, modelNode, catalog.assets, catalog.brands),
    [sel, modelNode, catalog.assets, catalog.brands],
  )
  const { layers, settle, drop } = useArtLayers(art)

  // 三级齐全才有 toml / 偏移 / 脚本这些"某机型某版本"的产物
  const ready = Boolean(sel.brand && sel.model && sel.variant)

  /*
   * 「使用 / 下载并使用 / 更新并使用 / 正在使用」那颗按钮（作者 2026-10-06：第二页要一颗
   * **明确的**动作按钮，放在「下一步」旁边，宽度固定不随文字变）。
   *
   * ★ 它消费的是**整个套餐**（`useBundleFiles` = `getVersionFiles`：MKP + 配套 BBS），
   *   **不是**"一个 TOML 能不能用" —— 单文件布尔值表达不了这个页面的状态。
   *   四态严格按顺序判、互斥（`docs/HOME-BUNDLE-DOWNLOAD.md` §2）：
   *
   *     底账正指着这个 combo        → 正在使用（定格，点不了）
   *     套餐里缺任一份              → 下载并使用   ← 缺 + 漂同时存在时也是这一态
   *     无缺、但有漂的              → 更新并使用
   *     全齐且全新                  → 使用（一个字节都不重下）
   *
   *   判不了（套餐没拿到 / 清单还没回来）= `unknown`：卡住，**不许**滑进「使用」。
   *   盘的现状与预设页同一套账（下载清单 + 漂移清单）；失败说人话，不编成功。
   */
  const [applying, setApplying] = useState(false)
  const [applyError, setApplyError] = useState<string | null>(null)
  /*
   * 投递面代次（`deliveryState`）：**任何页面**把交付文件写下 / 移出本机之后 +1，
   * 这里重拉那两份单子 —— 页签常驻，本页不重挂载，不订阅就永远拿着首读那一份
   * （2026-10-07 真机：预设页下了 BBS，回首页按钮还停在旧状态）。
   * 本页自己的下载走 `applyCurrent` 里的 `deliveryMutated()`（同一条路，不再另设 tick）。
   */
  const deliveryRevision = useDeliveryRevision()
  /* 盘上那两份单子（下载区 / 漂移），按文件名查 */
  const [onDisk, setOnDisk] = useState<{ downloaded: Set<string>; stale: Set<string> } | null>(null)

  const comboKey =
    ready && sel.model !== null && sel.variant !== null ? `${sel.model}/${sel.variant}` : null
  const comboApplied =
    activeEntry !== null &&
    sel.model !== null &&
    sel.variant !== null &&
    activeEntry.machineId === sel.model &&
    activeEntry.versionId === sel.variant

  /*
   * **这一版下我自己的那些副本**（2026-10-17 晚，作者：「要都显示出来，套餐的始终和
   * 其他的分开」「这下面还有显示那些可以选的」）。
   *
   * 与抽屉同一份数据（`pickerRows` 已经按机型 → 版本摊好），这里只挑当前这一版的那几行；
   * 每条带**后处理次数**（学习账 `getPresetUsage`），那一格按次数降序摆 —— 只排这一格，
   * 预设页列表与抽屉**一个都不动**（作者明确要求）。
   */
  /*
   * 选中的副本**带钥匙**（属于哪一版）：换一版之后它自然不作数。
   * 原来靠一条 effect 事后 `setPickedCopy(null)` —— effect 在下一帧才跑，中间那一帧
   * 还牵着上一版的选中；带钥匙就没有那一帧。
   */
  const [picked, setPicked] = useState<{ key: string; fileName: string; path: string } | null>(null)
  const pickedCopy = picked !== null && picked.key === comboKey ? picked : null
  /*
   * 学习账（次数）：整本一次取回，按路径查。共用 `usePresetUsage`（预设页详情卡读的
   * 是同一本）—— 模块缓存让换一版时行尾那个 `N 次` 当帧就在，不必等一趟 IPC。
   * `comboKey` 只是刷新信号：这一页也是常驻的，挂载读一次之后就再也不会读了。
   */
  const usage = usePresetUsage(comboKey)

  const copyList = useMemo(() => {
    if (sel.model === null || sel.variant === null) return []
    return pickerRows
      .filter((r) => r.machineId === sel.model && r.versionId === sel.variant)
      .map((r) => ({
        fileName: r.fileName,
        path: r.ref,
        count: usage.get(`local:${r.ref}`) ?? 0,
      }))
  }, [pickerRows, sel.model, sel.variant, usage])

  /* 选中的那份正被底账用着（用户线按路径比） */
  const pickedActive =
    pickedCopy !== null &&
    activeEntry !== null &&
    activeEntry.origin === 'local' &&
    activeEntry.path === pickedCopy.path



  const bundle = useBundleFiles(sel.model, sel.variant)
  /** 「使用」的目标：套餐里那一份 MKP（只有它能被应用；BBS 这一轮只落盘） */
  const presetFile = bundle.presetFileName

  /*
   * **这一版套餐里那份预设的文件名 —— 同步就有**（目录里 `machineId/versionId` 的倒查，
   * 与 `getVersionFiles` 是同一件事实的两种读法）。
   *
   * 两个用处，都是"换一版那一帧不许拿旧事实":
   *   · `PresetStack.fallbackName` —— 名字决定「我的副本」筛几行（见那边的 prop 注释）；
   *   · 「默认」那一行的「正在使用」标签 —— `presetFile` 要等一趟 IPC，
   *     等它回来标签会先掉一下再加回来。
   */
  const defaultName =
    sel.model !== null && sel.variant !== null ? fileOf(sel.model, sel.variant) : null

  /*
   * 「默认」那一行是不是在用：底账指着**云端原件**，或指着 `presets-local/` 里**同名**
   * 那份（使用之后实际生效的是后者）。按**文件**判，不按 combo —— 同版本另一份在用着时
   * 这一行仍该能点（作者截图：「默认那个也要可以应用呀」）。
   */
  const defaultActive =
    comboApplied &&
    activeEntry !== null &&
    (activeEntry.origin === 'preset' ||
      (defaultName !== null && activeEntry.fileName === defaultName) ||
      activeEntry.fileName === presetFile)

  /* 盘上那两份单子：combo 换台时读一次；**投递面代次一变就重读**（谁写的都算） */
  useEffect(() => {
    if (comboKey === null) return
    let alive = true
    void Promise.all([
      api.getDownloadedFiles().catch(() => []),
      api.getStaleFiles().catch(() => []),
    ]).then(([downloaded, stale]) => {
      if (!alive) return
      setOnDisk({
        downloaded: new Set(downloaded.map((f) => f.fileName)),
        stale: new Set(stale.map((f) => f.fileName)),
      })
    })
    return () => {
      alive = false
    }
  }, [comboKey, deliveryRevision])

  /*
   * 套餐这一批文件在盘上是什么样 —— 三态与「待下清单」都从这一处算。
   *
   * ★ 「在盘上」= **已下载 ∪ 漂移**：两张单子都只收**盘上真有**的文件
   *   （`runtime::delivery::entries_in_status` 遍历目录登记再读盘：字节对得上进「下载区」，
   *   对不上进「漂移」），**漂 ≠ 缺** —— 漂的那份是"在盘上但字节旧/坏了"。
   *   把漂当成缺，就会把「更新并应用」说成「下载并应用」（探针逮到过）。
   * ★ 待下 = 缺 ∪ 漂，**按文件名去重**（作者 2026-10-06 的纪律①）：同一份既缺又漂
   *   （理论上不会，但别指望）只会下一次；实现上不许把两张单子各拼一段再连接。
   * ★ `unknown` 是"判不了"，不是"全齐"（纪律②）—— 不许拿它当 ready。
   */
  const bundleState = useMemo(() => {
    const files = bundle.files
    if (files === null || bundle.problems.length > 0 || onDisk === null) {
      return { kind: 'unknown' as const, todo: [] as string[], missing: 0, drift: 0 }
    }
    const todo = new Set<string>()
    let missing = 0
    let drift = 0
    for (const f of files) {
      const onDiskNow = onDisk.downloaded.has(f.fileName) || onDisk.stale.has(f.fileName)
      if (!onDiskNow) {
        missing += 1
        todo.add(f.fileName)
        continue
      }
      if (onDisk.stale.has(f.fileName)) {
        drift += 1
        todo.add(f.fileName)
      }
    }
    const kind = missing > 0 ? 'missing' : drift > 0 ? 'drift' : 'ready'
    return { kind, todo: [...todo], missing, drift }
  }, [bundle, onDisk])

  /*
   * 文案口径（作者 2026-10-17 第七轮）：「**下面的按钮从应用改成使用**」——
   * 全站这件事的动词是「使用」（抽屉里那颗也叫「使用」、行上的状态叫「正在使用」），
   * 这里原来叫「应用」，同一个动作两个词。
   */
  /*
   * ★ 存的是**字典键**而不是句子（2026-10-26）：下面 `actionText` 要拿它跟
   *   「正在使用」那一档比，比句子会在换语言时失效 —— 比键永远稳。
   */
  const actionLabelKey: StringKey = applying
    ? 'home.action.applying'
    : comboApplied
      ? 'home.action.inUse'
      : bundleState.kind === 'unknown'
        ? bundle.problems.length > 0
          ? 'home.action.planMissing'
          : 'home.action.loading'
        : bundleState.kind === 'missing'
          ? 'home.action.downloadUse'
          : bundleState.kind === 'drift'
            ? 'home.action.updateUse'
            : 'home.action.use'
  /* 主按钮的可用性在下面按「选中的那份文件」算（`actionOff`）——
     combo 级的旧判据（`comboApplied || …`）不再直接使用，留着这一行只会误导。 */

  const applyCurrent = useCallback(async () => {
    if (applying || bundleState.kind === 'unknown' || presetFile === null) return
    setApplying(true)
    setApplyError(null)
    /* 预判：这一批里缺的与漂的（已去重）。空 = 全齐全新 —— 直接应用，一个字节都不重下 */
    const todo = bundleState.todo
    /*
     * ★ 2026-10-13：这一趟**先浮一句再动手**（作者：「点这个下载并应用的时候，也可以出现
     * 刚才说的那个透明的气泡呀……如果还没有下载的时候，那个提示是和那个已经下载过、
     * 只应用的那个气泡里面显示的内容还不太一样，就区分一下」）。
     *
     * 三档分的正是"这一趟到底要做什么"，用词按作者定的口径（不出现"云端 / 本地这份"）：
     *   · 本机全齐（ready）→ 只应用；· 缺（missing）→ 先下载；· 漂（drift）→ 重新下载
     *
     * 吃的是页面底部那颗 `.scriptToast`（与「复制脚本」共用一颗）——**不占任何布局位置**，
     * 说完自己走（5s，见上面那个 effect）。
     */
    const pendingNote =
      bundleState.kind === 'ready'
        ? tr('home.note.ready')
        : bundleState.kind === 'drift'
          ? tr('home.note.drift')
          : tr('home.note.missing')
    setScriptNote(pendingNote)
    try {
      if (todo.length > 0) {
        /* 逐份看结局：**任何一份没成就停下、不应用** —— 套餐缺一份就是没齐（不编成功） */
        const outcomes = await api.downloadCatalogFiles(todo)
        const failed = outcomes.filter((o) => !o.ok)
        if (failed.length > 0) {
          throw new Error(
            tr('home.note.partial', {
              detail: failed.map((o) => `${o.fileName} (${o.message})`).join('; '),
            }),
          )
        }
      }
      /*
       * **两步，各是各的**（2026-10-09 改判）：
       *
       *   ① `fetchCloudPreset` 把云端这一版取到本机（本机已有当前版就一个字节不下载），
       *      并落一份**本地工作副本**（`presets-local/<原名>.toml`，已经有了就一个字节不动）；
       *   ② `applyActivePreset` 把**本地那份**设成当前使用。
       *
       * 为什么不合成一步：「取到本机」与「把哪一份设为生效」是两件事 —— 云端表那两颗
       * （下载 / 更新）只做前一件，使用只发生在本地表那几行上。首页这一颗按钮的语义是
       * 「让这一台机器用上这一版」，所以两步都走，顺序也是这个顺序。
       *
       * 于是"点一下使用就凭空多出一份"那回事仍然不会发生：副本要么本来就在（原样不动），
       * 要么这一趟就该有（本机一份都没有）。
       *
       * 也**不必**再给"预判过期"留兜底：字节在不在这一层由后端自己解决，
       * 不存在"预判说齐了、其实没齐"那种错 —— 上面那次批量下载只负责套餐里**别的**文件
       * （切片器配置那一类）。
       */
      const fetched = await api.fetchCloudPreset(presetFile)
      /* 本仓术语就是 `preset / local`（来源只有这两条线，见契约 ActiveOrigin） */
      await api.applyActivePreset(fetched.fileName, 'local', fetched.path)
      appStateMutated()
      /* 做完了换一句结果（与上面那句同源：这一趟是"下载并应用"还是"只应用"） */
      setScriptNote(
        bundleState.kind === 'ready'
          ? tr('home.note.used')
          : bundleState.kind === 'drift'
            ? tr('home.note.updated')
            : tr('home.note.downloaded'),
      )
    } catch (e) {
      const msg = errorText(e)
      /* 失败留痕两条路：列里那行红字（带 traceId、常驻）+ 底部气泡浮一句（点击必须有回声） */
      setApplyError(msg)
      setScriptNote(msg)
    } finally {
      setApplying(false)
      /* 投递面广播：本页重读那两份单子（订阅在上面），**别的页**（预设页本地表 /
         BBS 交付面）也收得到 —— 成了一半也是变了，所以放在 finally 里 */
      deliveryMutated()
    }
  }, [applying, bundleState, presetFile, tr])

  /*
   * **用我选中的那份副本**（2026-10-17 晚）。
   *
   * 与上面那颗按钮的区别只有一句：不下载、不落副本 —— 它本来就在 `presets-local/`
   * 里，直接设成正在使用的那一份（与抽屉里那颗「使用」同一个后端动作）。
   */
  const applyPickedCopy = useCallback(async () => {
    if (pickedCopy === null || applying) return
    setApplying(true)
    setApplyError(null)
    try {
      await api.applyActivePreset(pickedCopy.fileName, 'local', pickedCopy.path)
      appStateMutated()
      setScriptNote(tr('home.note.used'))
    } catch (e) {
      const msg = errorText(e)
      setApplyError(msg)
      setScriptNote(msg)
    } finally {
      setApplying(false)
    }
  }, [applying, pickedCopy, tr])

  /*
   * 那颗主按钮的**目标**与**文字**（2026-10-17 晚，两改）：
   *   选中了某份副本 → 使用它；选的是「默认」→ 走原来那套（缺就下载、有就直接用）。
   *
   * ★★ 「正在使用」必须按**选中的那份文件**判，不能按 combo：
   *   同一版本下 `A1-fast copy.toml` 在用着、而这里选的是「默认」那份 —— 按 combo 判会
   *   显示「正在使用」并把按钮禁掉（作者截图：「默认那个也要可以应用呀」）。
   */
  const targetActive = pickedCopy !== null ? pickedActive : defaultActive
  const actionTextKey: StringKey = applying
    ? 'home.action.applying'
    : targetActive
      ? 'home.action.inUse'
      : pickedCopy !== null
        ? 'home.action.use'
        : /* 默认那份没在用：`actionLabelKey` 里那句"正在使用"是 combo 级的，这里换成"使用" */
          actionLabelKey === 'home.action.inUse'
          ? 'home.action.use'
          : actionLabelKey
  const actionText = tr(actionTextKey)
  const actionOff =
    applying || targetActive || (pickedCopy === null && bundleState.kind === 'unknown')
  const runAction = pickedCopy === null ? applyCurrent : applyPickedCopy

  /** 已经拿到的那一份预设；还在等 / 失败时为 null —— 不回退到 mock 里的默认那份 */
  const presetInfo = preset.status === 'ready' ? preset.preset : null
  /** 文件名在等待之外的几态都是已知的（知道要取哪一份），单独取出来 */
  const presetName =
    preset.status === 'ready'
      ? preset.preset.name
      : preset.status === 'idle'
        ? null
        : preset.name

  /*
   * A3：正在使用的那份（底账）命中当前 combo 时，名字 / 路径**用底账的** ——
   * 应用了「本地文件」，这里就说本地文件（与预设页横幅同源）；没命中维持目录那份。
   * 覆盖只在目录那份就绪后发生：waiting / failed 的三态动画不动。
   */
  const presetReady = preset.status === 'ready'

  const displayName = activeForSel !== null && presetReady ? activeForSel.fileName : presetName
  const displayPath =
    activeForSel !== null && presetReady
      ? (activeForSel.path ?? presetInfo?.path ?? null)
      : (presetInfo?.path ?? null)
  /*
   * 「复制后处理脚本」那一整条命令 —— **由后端现拼**（`getPostProcessCommand`）：
   * 可执行物 = 本程序自己（壳那一侧的 `current_exe()`），预设 = 那台机器上真实的绝对
   * 落点（本地那份优先，其次是云端那份——**必须真在盘上**）。前端**不拼任何一段路径**：
   * 早先硬编码的是另一个仓库的 Windows exe，Mac 上复制出去的命令必然跑不通。
   *
   * ★ 按钮**常驻**（2026-10-10 作者裁定）：能不能给命令**不决定**摆不摆按钮 ——
   *   不能给时点它就是"抄不了"，原因照样说给人听。
   *
   * ★ 2026-10-13：进页面**不再预读一次命令**（原来那份 `copyCommand` 现况只喂给
   *   `CopyAction`，用来决定按钮下面那行小字说什么）。现在原因不再常驻在按钮下面 ——
   *   点了之后由页面底部那颗不占位的 `.scriptToast` 说（作者：「干嘛要放在这？都被
   *   截断了」），于是这份"缓存现况"整个退役：**点的时候现要一次**就够，前端不留旧路径。
   */

  /*
   * 点按钮那一刻**现要一次**：后端按当前平台与真实落点现拼，前端不缓存命令。
   *
   * ★ 2026-10-16：取命令走**全站那一个出口**（`postProcessCommand`）——
   *   与标题栏 `MKP` 字标、与预设页右键那一项同一个函数，只是**反馈方式不同**：
   *   ★ 首页这颗**不走通知系统**，仍用页面底部那颗不占位的气泡 —— 那是作者
   *     2026-10-13 的裁决（气泡挂在按钮下面会被卡片两层 overflow 裁掉一截：
   *     「干嘛要放在这？都被截断了」）。换位置是另一件事，本轮不动。
   */
  const loadCommand = useCallback(async (): Promise<PostProcessCommand> => {
    if (sel.model === null || sel.variant === null) {
      return { ready: false, reason: tr('home.reason.pickFirst') }
    }
    return postProcessCommand(sel.model, sel.variant)
  }, [sel.model, sel.variant, tr])

  /*
   * ★ 2026-10-13（作者：「可以加一下那个复制的……他要复制的是那个后处理脚本呀，就跟首页的
   * 是一样的呀……你改成复制脚本吧，反正位置足够」）：四个步骤的底部各摆一颗「复制脚本」——
   * 与第一页那颗（内容里的 `CopyAction`）是同一个东西：点一下向后端**现要**一条命令，
   * 拿到就进剪贴板（就地变「已复制」1.5s）；抄不了就把原因浮出来 ——
   * 用**不占位的悬浮气泡**（`.scriptToast`），不挤压任何东西。
   *
   * 门槛（作者给的两句话）：「选定了、有预设的时候才可以复制」+「没有应用的时候就不能复制」——
   * 显形要"选齐三级 + 这一份预设取得到"，未应用就灰着（title 说差哪一步）。
   */
  const [scriptCopied, setScriptCopied] = useState(false)
  const [scriptNote, setScriptNote] = useState<string | null>(null)
  const scriptTimer = useRef<number | null>(null)

  useEffect(
    () => () => {
      if (scriptTimer.current !== null) window.clearTimeout(scriptTimer.current)
    },
    [],
  )

  /*
   * 气泡自己走（5s）。
   *
   * ★ 2026-10-13：原来这里还有"鼠标停在上面时先不走"（当时图的是"文字能拖选，
   * 拖到一半消失等于没给这个能力"）。作者试过之后判掉了拖选本身
   * （「它的拖选好像是失败的……算了，不要了」）—— 现在气泡 `pointer-events: none`
   * 不吃指针，悬停判断也就永远触发不了，一并删掉。
   */
  useEffect(() => {
    if (scriptNote === null) return
    const t = window.setTimeout(() => setScriptNote(null), 5000)
    return () => window.clearTimeout(t)
  }, [scriptNote])

  const copyScript = useCallback(() => {
    void loadCommand().then(
      (cmd) => {
        if (!cmd.ready || cmd.command === undefined) {
          setScriptCopied(false)
          setScriptNote(cmd.reason ?? tr('home.script.notReady'))
          return
        }
        void navigator.clipboard.writeText(cmd.command).then(
          () => {
            setScriptNote(null)
            setScriptCopied(true)
            if (scriptTimer.current !== null) window.clearTimeout(scriptTimer.current)
            scriptTimer.current = window.setTimeout(() => setScriptCopied(false), 1500)
          },
          (e: unknown) => {
            setScriptCopied(false)
            setScriptNote(errorText(e))
          },
        )
      },
      (e: unknown) => {
        setScriptCopied(false)
        setScriptNote(errorText(e))
      },
    )
  }, [loadCommand, tr])

  /**
   * 四步共用那颗「复制脚本」（住 navs 左侧那一组 —— 作者：「它应该放到左侧吧」）。
   *
   * ★ 显形条件：**选齐三级 + 有预设 + 这一份已应用**。
   * 2026-10-13 作者：「这个灰色都没有灰色的感觉……用户根本不知道是禁用，他只觉得点不了。
   * 那没有的话，就干脆直接隐藏起来了，可用的时候才出现」——
   * 所以这里**不再有灰态**：差一步就整颗不出现，出现就是能点的。
   *
   * 点了之后后端说"这条命令现在给不了"（没落盘 / 预览里没有可执行物）不再靠灰按钮 + 小字，
   * 而是**浮一颗不占位的气泡**说原因（见 `copyScript` 与页脚那颗 `.scriptToast`）。
   */
  const scriptAction = useMemo(
    () => ({
      label: scriptCopied ? tr('home.script.copied') : tr('home.script.copy'),
      on: ready && presetReady && comboApplied,
      title: tr('home.script.copyTitle'),
      onClick: copyScript,
    }),
    [comboApplied, copyScript, presetReady, ready, scriptCopied, tr],
  )

  /*
   * 2026-10-13：「正在使用 · 我的文件」那个来源后缀去掉 —— 作者：「我的一份、官方一份」（现称交付预设）
   * 这些相关的都去掉，那是以前的旧版才那么设计的」。命中底账就说「正在使用」，
   * 不再分来源（云端底稿 / 自己那份），这句只回答"是不是正在用的那一份"。
   */
  const inUseNote =
    activeForSel === null || !presetReady ? undefined : tr('home.action.inUse')
  /* picker sheet 那块卡：名字 / 路径换成底账那份，其余（三轴数值等）保持原样。
     useMemo 包一层：sheets 那张 useMemo 拿它当依赖，引用得稳 */
  const presetView = useMemo(
    () =>
      activeForSel !== null && presetReady
        ? {
            ...preset,
            preset: {
              ...preset.preset,
              name: activeForSel.fileName,
              path: activeForSel.path ?? preset.preset.path,
            },
          }
        : preset,
    [activeForSel, presetReady, preset],
  )
  const entryLabel = modelName ? tr('home.entry.pickVersion') : tr('home.entry.pickMachine')
  const artAlt =
    [brandName, modelName].filter(Boolean).join(' ') || tr('home.entry.noMachine')

  const sheets: Sheet[] = useMemo(
    () => [
      {
        id: 'machine',
        node: sel.model ? (
          /* 不给眉标题：机型代号（A1_MINI）与下面那行大标题（A1 mini）说的是同一件事，
             写两遍是复读。去掉之后 data-bare-top 生效，整块内容还往上抬了一截 */
          <CardFrame
            navs={[
              {
                label: tr('home.entry.change'),
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(1),
              },
            ]}
          >
            <div className={p.info} data-scroll>
              <div className={p.ident}>
                {modelName && <h1 className={p.name}>{modelName}</h1>}

                <button
                  type="button"
                  className={p.variant}
                  onClick={() => deckRef.current?.jumpTo(1)}
                  title={tr('home.entry.changeTitle')}
                >
                  {variantName ?? entryLabel}
                  <span aria-hidden="true">›</span>
                </button>

                {ready && (
                  <>
                    <p className={p.path} title={displayPath ?? undefined}>
                      {displayName ?? '—'}
                    </p>
                    {/* 按钮**常驻**（2026-10-10）：这条命令能不能给由后端的两条真事实
                        说（本程序自己 + 预设真在盘上），点它才向后端现要命令。
                        抄不了时的原因由**页面底部那颗气泡**说 —— 不给 `onNote` 的话，
                        气泡就永远出不来（`CopyAction` 自己不再渲染任何提示，2026-10-13） */}
                    <CopyAction
                      label={tr('home.copy.action')}
                      load={loadCommand}
                      onNote={setScriptNote}
                    />
                  </>
                )}
              </div>

              {ready && (
                <Rows
                  items={[
                    [tr('calib.axis.x'), axisText(saved, 'x')],
                    [tr('calib.axis.y'), axisText(saved, 'y')],
                    [tr('calib.axis.z'), axisText(saved, 'z')],
                    [tr('home.rows.glueSpeed'), presetInfo ? `${presetInfo.speed} mm/s` : '—'],
                  ]}
                />
              )}
            </div>

            <div className={p.slot}>
              {/* density 不再传给它：A31 那个入参只喂调试面板的「按档覆盖」开关（默认关着），
                  面板删了它也就没有意义 —— 尺寸照旧由窗口与曲线算 */}
              <HeroFade layers={layers} onSettle={settle} onDrop={drop} alt={artAlt} />
            </div>
          </CardFrame>
        ) : (
          /* 还没选机型：欢迎版面。主 CTA 就在内容区，所以这一态左下角不放胶囊。
             也不放眉标题 —— 标题自己就带 MKP，眉标题再写一遍是复读 */
          <CardFrame navs={[]} peekSafe>
            <div className={p.welcome} data-scroll>
              <div className={p.welcomeTop}>
                <div className={p.welcomeCopy}>
                  <h1 className={p.heroTitle}>{tr('home.hero.title')}</h1>
                  <p className={p.heroLead}>{tr('home.hero.lead')}</p>
                  <button
                    type="button"
                    className={p.cta}
                    onMouseDown={noFocus}
                    onClick={() => deckRef.current?.jumpTo(1)}
                  >
                    {tr('home.hero.start')}
                    <span aria-hidden="true">→</span>
                  </button>
                </div>

                <div className={p.welcomeArt}>
                  <HomeCardStack />
                </div>
              </div>

              <StepRail
                pickIndex={1}
                zIndex={CALIB_Z_INDEX}
                xyIndex={CALIB_XY_INDEX}
                modelIndex={CALIB_MODEL_INDEX}
                onGo={(i) => deckRef.current?.jumpTo(i)}
              />
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'picker',
        node: (
          /* 同样不给眉标题：卡里三组标题（品牌 / 机型 / 打印件版本）已经说明这是在选机型。
             去掉后 data-bare-top 生效，「品牌」正好抬到原来眉标题那行字的高度 */
          <CardFrame
            style={{ '--fade-ms': `${fadeMs}ms` } as CSSProperties}
            peekSafe
            navs={[
              { label: tr('home.nav.home'), onClick: () => deckRef.current?.jumpTo(0) },
              /* 复制脚本住左侧那一组（作者：「它应该放到左侧吧」） */
              scriptAction,
            ]}
            actions={[
              /* 「应用 / 下载并应用 / 已应用」：宽度固定（文字换态不变）。
                 浏览三级选择不算应用 —— 应用只发生在这颗按钮与各处抽屉的明确动作上。
                 ★ 2026-10-13（作者：「换了打印件版本、还没应用的情况下，我就不希望出现
                 『下一步』这个图标，否则它就容易没点下载应用就下一步了，导致没有参数」）：
                 没应用时右组**只剩这一颗** —— 它是唯一的主行动（绿底），「下一步」不出现在
                 用户眼前；应用完成之后「下一步」才亮起来。 */
              ...(ready
                ? [
                    {
                      /* 选中了副本 ⇒ 目标就是那份副本（不下载、不落副本）；否则还是原口径 */
                      label: actionText,
                      primary: !actionOff,
                      fixed: true,
                      disabled: actionOff,
                      on: true,
                      onClick: () => {
                        void runAction()
                      },
                    },
                  ]
                : []),
              {
                /* `on` 里带 `comboApplied`：这一份真的应用了才亮。
                   用 on: false 而不是把这一项从数组里删掉 —— 胶囊照旧占位、只隐去，
                   于是它出现时一个像素都不推动前面那颗应用按钮（CardFrame 的老规矩）。 */
                label: tr('home.next'),
                arrow: true,
                primary: true,
                on: ready && comboApplied,
                onClick: () => deckRef.current?.jumpTo(CALIB_Z_INDEX),
              },
            ]}
          >
            <div className={`${p.info} ${p.picker}`} data-scroll>
              <MachinePicker
                sel={sel}
                onPick={pick}
                brands={brandOptions}
                models={modelOptions}
                variants={variantOptions}
              />

              {/* 预设文件那一块只在真的有一份预设可谈时才出现（选齐三级）。
                  原来是常驻的横杠占位，宽窗里就成了"左上角选了一行、中间凭空一块空占位"，
                  视觉锚点全落在没内容的地方。宁可出现时把列往下长一截。
                  （2026-10-13：小窗档那块三轴竖排退役 —— 作者「现在我觉得没必要了，
                  它不要出现这个偏移值」，见 PresetStack 文件头。） */}
              {preset.status !== 'idle' && (
                <div className={p.pickerPreset}>
                  <PresetStack
                    state={presetView}
                    /* 这一版套餐里那份预设的名字 —— 目录里同步就有，换一版当帧就在（见 prop 注释） */
                    fallbackName={defaultName}
                    inUse={inUseNote}
                    /* 这一版下我自己的副本（按后处理次数排）+ 选中态（见 PresetStack 头注释） */
                    copies={copyList}
                    /* 「正在使用」只挂在**这一版**的那份上：底账指着别版时不许在别版列表里乱标 */
                    activePath={
                      comboApplied && activeEntry !== null && activeEntry.origin === 'local'
                        ? (activeEntry.path ?? null)
                        : null
                    }
                    /* 第一行（默认那份）在用：底账指着云端原件，或指着本地同名那份 */
                    defaultActive={defaultActive}
                    pickedPath={pickedCopy?.path ?? null}
                    onPick={(c) =>
                      setPicked(
                        c === null || comboKey === null
                          ? null
                          : { key: comboKey, fileName: c.fileName, path: c.path },
                      )
                    }
                  />
                </div>
              )}

              {/* 应用失败要说话（不编成功）：errorText 带人话与 traceId */}
              {applyError !== null && <p className={p.applyError}>{applyError}</p>}

              {/* 套餐本身取不全（后端没这个组合 / 配置没配齐）—— 照实说，不假装"套餐没文件" */}
              {bundle.problems.length > 0 && (
                <p className={p.applyError}>{bundle.problems.join('；')}</p>
              )}
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'calib-z',
        node: (
          <CardFrame
            eyebrow={tr('home.calib.zEyebrow')}
            peekSafe
            tag={dirty ? tr('home.calib.unsaved') : savedNote ? tr('home.calib.saved') : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            /* 打开模型是这一步的主入口（作者 10-03：绿底给打开模型，预设 pill 退暗） */
            topAction={{
              label: tr('home.calib.openModel'),
              accent: true,
              onClick: () => setOpenModel({ id: 'z', name: tr('home.calib.zPlate') }),
            }}
            corner={
              /* 预设 pill 挂右上角（作者 10-03：内容列里那一行别占地方）——点开左抽屉；
                 没取到文件名时退成「选择预设」的灰态 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title={tr('home.calib.pickPresetTitle')}
              >
                <span className={p.presetFileName}>
                  {presetName ?? tr('home.calib.pickPreset')}
                </span>
                <span className={p.presetSwitch}>{tr('home.calib.switch')}</span>
              </button>
            }
            navs={[
              { label: tr('home.calib.back'), back: true, onClick: () => deckRef.current?.jumpTo(1) },
              scriptAction,
            ]}
            actions={[
              { label: tr('home.calib.discard'), on: dirty, onClick: clearAll },
              {
                label: tr('home.calib.save'),
                on: dirty,
                primary: true,
                /* 存不动时就灰着 —— 不再挂「本地这份 / 另存为本地预设」那套旧版提示
                   （2026-10-13 作者：这些字都去掉） */
                disabled: !canSave,
                onClick: commitAll,
              },
              {
                label: tr('home.next'),
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
            ]}
          >
            {/*
             * 2026-10-13（作者：「你把这一行字去掉，没必要提示给用户……就算没有预设
             * 空的时候，你也要把它位置调整好了，不要让它上下移动」）：
             * 板下那行「先回『选择机型』…」提示与它带来的 `data-hint` 让位一起退役 ——
             * 有预设 / 没预设从此同一副版面，板子不再因为提示出现与消失而变大小、挪位置。
             */}
            <div className={`${p.info} ${p.calib}`} data-scroll>
              {/* 预设入口已挪到卡片右上角（corner），内容列只留三轴读数 */}
              <CalibHead
                saved={saved}
                draft={draft}
                view={axisView}
                limits={limits}
                activeAxis="z"
                canEdit={canPick}
                onType={typeAxis}
                onRevert={revertAxis}
                onReset={resetAxis}
              />

              <section className={p.step}>
                {/* 板名撤了（作者 10-03：容器感去掉，与「校准」tab 同一语言）——
                    眉标题那行已写着「Z 偏移校准」，再挂一块板名是复读。
                    手柄留在原位（title="" 渲染零高 h3），只跟着收编的皮换直角实白 */}
                <PlateZoom resetKey="z" title="">
                  <CalibPlate
                    model="zoffset"
                    theme={plateTheme}
                    label={tr('home.calib.zPlate')}
                    onPick={pickZ}
                    picked={zSelected}
                    hitLabel={zHitLabel}
                  />
                </PlateZoom>
                {/* G06-8 攒下的两行文案，与「校准」tab 同一句原话（作者 10-03：首页也缺这个） */}
                <p className={p.legend}>{tr(Z_LEGEND_KEY)}</p>
                <p className={p.tip}>{tr(Z_TIP_KEY)}</p>
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'calib-xy',
        node: (
          <CardFrame
            eyebrow={tr('home.calib.xyEyebrow')}
            peekSafe
            tag={dirty ? tr('home.calib.unsaved') : savedNote ? tr('home.calib.saved') : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            topAction={{
              label: tr('home.calib.openModel'),
              accent: true,
              onClick: () => setOpenModel({ id: 'xy', name: tr('home.calib.xyPlate') }),
            }}
            corner={
              /* 同 Z 步：预设 pill 挂右上角，点开左抽屉 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title={tr('home.calib.pickPresetTitle')}
              >
                <span className={p.presetFileName}>
                  {presetName ?? tr('home.calib.pickPreset')}
                </span>
                <span className={p.presetSwitch}>{tr('home.calib.switch')}</span>
              </button>
            }
            navs={[
              {
                label: tr('home.calib.back'),
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_Z_INDEX),
              },
              scriptAction,
            ]}
            actions={[
              { label: tr('home.calib.discard'), on: dirty, onClick: clearAll },
              {
                label: tr('home.calib.save'),
                on: dirty,
                primary: true,
                /* 存不动时就灰着 —— 不再挂「本地这份 / 另存为本地预设」那套旧版提示
                   （2026-10-13 作者：这些字都去掉） */
                disabled: !canSave,
                onClick: commitAll,
              },
              {
                label: tr('home.next'),
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_MODEL_INDEX),
              },
            ]}
          >
            <div className={`${p.info} ${p.calib}`} data-plate="xy" data-scroll>
              <CalibHead
                saved={saved}
                draft={draft}
                view={axisView}
                limits={limits}
                activeAxis="xy"
                canEdit={canPick}
                onType={typeAxis}
                onRevert={revertAxis}
                onReset={resetAxis}
              />

              <section className={p.step}>
                {/* 同 Z 步：去容器、去板名，手柄直角实白；XY 板内自带整段说明，不给字 */}
                <PlateZoom resetKey="xy" title="">
                  <CalibPlate
                    model="precise"
                    theme={plateTheme}
                    label={tr('home.calib.xyPlate')}
                    onPick={pickXY}
                    picked={xySelected}
                    hitLabel={xyHitLabel}
                  />
                </PlateZoom>
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'models',
        node: (
          <CardFrame
            eyebrow={tr('home.calib.modelEyebrow')}
            peekSafe
            navs={[
              {
                label: tr('home.calib.back'),
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
              scriptAction,
            ]}
            actions={[
              {
                label: tr('home.calib.openTest'),
                primary: true,
                onClick: () =>
                  setOpenModel({ id: TEST_MODEL_ID, name: tr('home.calib.testModel') }),
              },
              { label: tr('home.nav.home'), onClick: () => deckRef.current?.jumpTo(0) },
            ]}
          >
            <div className={`${p.info} ${p.models}`} data-scroll>
              {/* 不给页级大标题：眉标题那行已经写着「第五步 · 测试模型」，
                  再写一遍是复读（首页与选机型页去掉眉标题是同一个道理）。
                  这一行只说"接下来做什么"，不重复页名 */}
              <p className={p.modelsLead}>{tr('home.calib.modelsLead')}</p>
              <div className={p.modelsHero}>
                {/* 深浅各一张（白件在深底上要暗一档）：取图见 heroPileSource */}
                <img
                  className={p.modelsHeroImg}
                  src={pile.src}
                  srcSet={pile.srcSet}
                  alt={tr('home.calib.testModel')}
                  draggable={false}
                />
              </div>
              <p className={p.modelsCaption}>{tr('home.calib.modelsFeature')}</p>
            </div>
          </CardFrame>
        ),
      },
    ],
    [
      activeEntry,
      artAlt,
      axisView,
      brandOptions,
      canPick,
      canSave,
      clearAll,
      comboApplied,
      commitAll,
      dirty,
      displayName,
      displayPath,
      draft,
      drop,
      entryLabel,
      fadeMs,
      inUseNote,
      layers,
      limits,
      loadCommand,
      modelName,
      modelOptions,
      pick,
      pickXY,
      pickZ,
      pile,
      plateTheme,
      preset,
      presetInfo,
      presetName,
      presetView,
      ready,
      resetAxis,
      revertAxis,
      saved,
      savedNote,
      scriptAction,
      sel,
      actionText,
      actionOff,
      runAction,
      copyList,
      comboKey,
      pickedCopy,
      defaultActive,
      defaultName,
      applyError,
      bundle,
      settle,
      typeAxis,
      variantName,
      variantOptions,
      xySelected,
      zSelected,
      /* 语言换了这张表要重算（里面全是 `tr(...)` 的结果） */
      tr,
    ],
  )

  return (
    <div className={p.page} data-density={density}>
      <SlideDeck ref={deckRef} sheets={sheets} density={density} canLeave={canLeave} />

      {/*
       * 「抄不了」的原因气泡 —— **绝对定位、不占任何布局位置，全页只有这一颗**：
       *   · 页脚那颗「复制脚本」（见 copyScript）点了给不了时；
       *   · 卡片里那颗「复制后处理脚本」（见上面的 `CopyAction`）点了给不了时 ——
       *     它自己不再渲染提示（挂在按钮下面会压住读数表、还被卡片裁，2026-10-13 作者退回）。
       *
       * 作者 2026-10-13：「你去用这个校准页面的这种提示啊，这种悬浮气泡不占任何位置的提示」
       * （上一版把它做成卡里的一行字，把板子和按钮都往上挤了）。说完自己走（5s，
       * 见上面那个 effect）；`pointer-events: none`（在 CSS 里）—— 不吃指针、不挡底下的按钮。
       */}
      {scriptNote !== null && (
        <div className={p.scriptToast} role="status">
          <i className={p.scriptToastDot} aria-hidden />
          <span>{scriptNote}</span>
        </div>
      )}

      {/* 预设抽屉：从左缘出（右边常驻露出卡，没地方）。有草稿时先问一句（`presetAsk` 那个确认框），
          确认后才换；抽屉挂在 .page（container 的布局包含块）上 */}
      <PresetPickerDrawer
        side="left"
        open={pickerOpen}
        /* ★ 2026-10-14：只列**本机真有**的（原来列目录那 12 个组合，点哪个都换不过去） */
        rows={pickerRows}
        /*
         * 在用那一份的**行键**（含文件名）。
         *
         * ★★ 2026-10-14 修：这里原来写 `presetInfo.name` —— 而 `presetInfo` 来自
         *   `usePreset`（读**目录那份 + 本机副本**），**不是底账**；同一屏上另外两处
         *   用的却是底账（`activeForSel` 显示名、1010 行那颗绿点）⇒ 两个源打架时
         *   抽屉勾的那一行跟顶上的名字对不上。**统一取底账**（与 `displayName` 同源）。
         */
        currentKey={
          sel.model !== null && sel.variant !== null && activeForSel !== null
            ? `${sel.model}/${sel.variant}/${activeForSel.fileName}`
            : null
        }
        onUse={(row) => {
          setPickerOpen(false)
          /*
           * ★★ 2026-10-14：走**按文件**那条路（`activateFile`），不是按 combo。
           * 原来把 `机型/版本` 交给 `pickPreset` —— 它开头"uid 与当前相同就退货"，
           * 而同一 combo 下的另一份 uid 完全相同 ⇒ 点了没反应。见 `activateFile` 的注释。
           * 有未保存改动时仍先问一句（原有那条分支）。
           */
          if (dirty) {
            setPresetAsk(`${row.machineId}/${row.versionId}`)
            return
          }
          if (row.machineId !== sel.model || row.versionId !== sel.variant) {
            const machine = catalog.machines.find((m) => m.id === row.machineId)
            const version = machine?.versions.find((v) => v.id === row.versionId)
            if (machine !== undefined && version !== undefined) {
              setSel({ brand: machine.brand, model: machine.id, variant: version.id })
            }
          }
          activateFile(row.fileName, row.ref)
        }}
        onClose={() => setPickerOpen(false)}
        drag={pickerDrag}
      />

      {openModel && (
        <Modal
          open
          size="sm"
          title={tr('home.calib.openModel')}
          onClose={() => setOpenModel(null)}
          footer={
            <>
              <Btn disabled={modelBusy || !modelInfo?.ready} onClick={openCachedModel}>
                {tr('home.openModel.cached')}
              </Btn>
              <Btn variant="primary" disabled={modelBusy || modelInfo === null} onClick={fetchAndOpenModel}>
                {tr('home.openModel.cloud')}
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
          {/* 错误进标题右边那一格（`HeadNote`，与校准页那颗同一套 —— 见那边那段长注释：
              一句话的错误不该住进框身，它会把框撑高、还和正常状态一个颜色） */}
          {modelErr !== null && <HeadNote text={modelErr.text} tone="error" />}
          <p className={p.note}>
            {modelBusy
              ? tr('home.openModel.working', { name: openModel.name })
              : modelInfo === null
                ? tr('home.openModel.checking', { name: openModel.name })
                : modelInfo.ready
                  ? tr('home.openModel.hasCache', {
                      name: openModel.name,
                      size: modelInfo.size ?? tr('home.openModel.sizeUnknown'),
                    })
                  : tr('home.openModel.noCache', { name: openModel.name })}
          </p>
          {/* ★ 2026-10-19（作者）：**内容区要给"怎么办"** —— 标题格只说"出了什么事"，
              下一步做什么住在这里。只在真能靠重启解决时才出现（判据是那个码，不是文案） */}
          {isStaleCatalogError(modelErr) && <StaleCatalogHelp />}
        </Modal>
      )}

      {pending !== null && (
        <Modal
          open
          size="sm"
          title={tr('home.modal.unsavedTitle')}
          onClose={() => setPending(null)}
          footer={
            <>
              <Btn onClick={() => setPending(null)}>{tr('home.modal.cancel')}</Btn>
              <Btn
                onClick={() => {
                  clearAll()
                  leaveTo(pending.to)
                }}
              >
                {tr('home.modal.discardLeave')}
              </Btn>
              {canSave && (
                <Btn
                  variant="primary"
                  onClick={() => {
                    commitAll()
                    leaveTo(pending.to)
                  }}
                >
                  {tr('home.modal.saveLeave')}
                </Btn>
              )}
            </>
          }
        >
          <p className={p.note}>
            {tr('home.modal.pendingNote')}
            {/* 三轴在哪一页都能改，所以这里列的是**全部**脏轴，不再按页筛。
                走到这个弹窗一定有草稿，也就一定有基准值；saved / draft 的判空只是给类型看 */}
            {saved && draft
              ? AXIS_ROWS.filter(([, k]) => dirtyAxes.includes(k))
                  .map(
                    ([labelKey, k]) =>
                      `${tr(labelKey)} ${saved[k].toFixed(2)} → ${draft[k].toFixed(2)}`,
                  )
                  .join(tr('home.modal.axisSeparator'))
              : ''}
            {canSave ? tr('home.modal.pendingAsk') : tr('home.modal.pendingLose')}
          </p>
        </Modal>
      )}

      {presetAsk !== null && (
        <Modal
          open
          size="sm"
          title={tr('home.modal.unsavedTitle')}
          onClose={() => setPresetAsk(null)}
          footer={
            <>
              <Btn onClick={() => setPresetAsk(null)}>{tr('home.modal.cancel')}</Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  clearAll()
                  applyPreset(presetAsk)
                  setPresetAsk(null)
                }}
              >
                {tr('home.modal.discardSwitch')}
              </Btn>
            </>
          }
        >
          <p className={p.note}>
            {tr('home.modal.presetAskNote', {
              name:
                presetOptions.find((o) => o.uid === presetAsk)?.name ??
                tr('home.modal.otherPreset'),
            })}
          </p>
        </Modal>
      )}

    </div>
  )
}
