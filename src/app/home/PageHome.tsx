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
import { api, errorText } from '../../api'
import { activeForSelection, selectionFromActive } from './activeSelection'
import { activateCombo, appStateMutated, useActivePreset } from '../state/appState'
import { deliveryMutated, useDeliveryRevision } from '../state/deliveryState'
import { uidOfFile, useCatalog } from './useCatalog'
import { useBundleFiles } from './useBundleFiles'
import PresetStack from './PresetStack'
import CalibPlate from '../calib/CalibPlate'
import PresetPickerDrawer from '../params/PresetPickerDrawer'
import HeroFade from './HeroFade'
import CardFrame from './CardFrame'
import CalibHead from '../calib/CalibHead'
import PlateZoom from '../calib/PlateZoom'
import { HomeCardStack, StepRail } from './HomeGuide'
import CopyAction from './CopyAction'
import { Btn } from '../ui/Controls'
import { Modal } from '../ui/Modal'
import type { Density } from '../../hooks/useDensity'
import { useDevDefaults } from './devDefaults'
import { pickArt } from './heroArt'
import { useArtLayers } from './useArtLayers'
import {
  AXIS_ROWS,
  NEED_COPY,
  NEED_PRESET,
  Z_LEGEND,
  Z_TIP,
  axisText,
  xyHitLabel,
  zHitLabel,
} from '../calib/calibAxes'
import { useCalibration } from '../calib/useCalibration'
import { usePreset } from '../calib/usePreset'
/* 第五步那张合影：**界面自带素材**（不进 Catalog / Delivery），
   从 `src/app/assets/hero/` 走 vite 资源管线（import 回来带内容哈希） */
import heroPile from '../assets/hero/hero_pile.webp'
import heroPile2x from '../assets/hero/hero_pile@2x.webp'

import p from './PageHome.module.css'

const EMPTY: Selection = { brand: null, model: null, variant: null }

/** 板子的配色跟应用的模式对齐；等应用做了深色模式，这里换成跟随主题的那个值 */
const PLATE_THEME = '浅色'

/** 两张校准卡在 sheets 里的位置：按页拦截换页要认它们 */
const CALIB_Z_INDEX = 2
const CALIB_XY_INDEX = 3
const CALIB_MODEL_INDEX = 4

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

/**
 * 后处理脚本里那段可执行文件路径。契约里没有它（真值在桌面壳那一侧），
 * 先沿用原来那串模板；`--Toml` 后面的路径跟着**当前那份文件**走（T10）。
 */
const MKP_EXE = 'G:\\project\\mkp-ssr\\target\\debug\\mkp-ssr.exe'

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
  const deckRef = useRef<DeckHandle>(null)
  /* 分级揭示的淡入时长：面板里调（产品仓里是常量 FADE_MS） */
  const { fadeMs } = useDevDefaults()
  const [sel, setSel] = useState<Selection>(EMPTY)

  const [openModel, setOpenModel] = useState<string | null>(null)
  const [pending, setPending] = useState<{ from: number; to: number } | null>(null)
  /* 校准页的预设下拉要换一份、但本页有未保存草稿时，先把要换的那一份记下来等确认 */
  const [presetAsk, setPresetAsk] = useState<string | null>(null)
  /* 预设抽屉（与「校准」tab 同一颗）：pill 点开，从左缘出 */
  const [pickerOpen, setPickerOpen] = useState(false)
  // 保存 / 放弃之后自己发起的那一次跳转不该再被拦
  const bypass = useRef(false)

  /* ---------- 文件体系：三级清单与「文件」那一级都从这里来（T10） ---------- */
  const catalog = useCatalog()

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

  // v005 那个 Modal 不认 Esc，这里补上：Esc = 取消
  useEffect(() => {
    if (openModel === null && pending === null && presetAsk === null) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      setOpenModel(null)
      setPending(null)
      setPresetAsk(null)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [openModel, pending, presetAsk])

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

  // 再点一次已选项 = 取消该级；取消或改选都清空下游
  const pick = useCallback((level: keyof Selection, id: string) => {
    setSel((prev) => {
      const next = prev[level] === id ? null : id
      if (level === 'brand') return { brand: next, model: null, variant: null }
      if (level === 'model') return { ...prev, model: next, variant: null }
      return { ...prev, variant: next }
    })
  }, [])

  /* ---------- 三级的清单：数据源是契约（getMachines），不再是手编表 ---------- */
  const brandOptions = useMemo<Option[]>(() => {
    const out: Option[] = []
    for (const m of catalog.machines) {
      if (!out.some((o) => o.id === m.brand)) out.push({ id: m.brand, label: m.brand })
    }
    return out
  }, [catalog.machines])

  const modelOptions = useMemo<Option[]>(
    () => catalog.machines.map((m) => ({ id: m.id, label: m.display })),
    [catalog.machines],
  )

  const variantOptions = useMemo<Option[]>(() => {
    const machine = catalog.machines.find((m) => m.id === sel.model)
    return (machine?.versions ?? []).map((v) => ({
      id: v.id,
      label: v.name,
      badge: v.tag,
      badgeTone: TAG_TONE[v.tag ?? ''],
      note: v.description,
    }))
  }, [catalog.machines, sel.model])

  /* 「文件」这一级：仓库里的 9 份 MKP，每份带 机型/版本 倒查（校准页的预设下拉用它） */
  const presetOptions = useMemo(
    () =>
      catalog.presets.flatMap((f) => {
        const uid = uidOfFile(f)
        return uid === null ? [] : [{ uid, name: f.fileName }]
      }),
    [catalog.presets],
  )

  /* 当前那份文件的 id（`机型/版本`）—— 预设下拉的选中值，也是「换没换」的比较基准 */
  const currentUid =
    sel.model !== null && sel.variant !== null ? `${sel.model}/${sel.variant}` : null

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

  /* 换预设会把本页草稿作废（草稿是相对上一份预设点出来的增量），所以先问一次。
     只给「取消 / 放弃改动并换」两条路：换了预设 saved 会被新预设的值覆盖，
     这时候提供「保存并换」是骗人的 —— 存下去的数立刻就被顶掉了 */
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

  const brandName = sel.brand
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
   * 「应用 / 下载并应用 / 更新并应用 / 已应用」那颗按钮（作者 2026-10-06：第二页要一颗
   * **明确的**应用按钮，放在「下一步」旁边，宽度固定不随文字变）。
   *
   * ★ 它消费的是**整个套餐**（`useBundleFiles` = `getVersionFiles`：MKP + 配套 BBS），
   *   **不是**"一个 TOML 能不能应用" —— 单文件布尔值表达不了这个页面的状态。
   *   四态严格按顺序判、互斥（`docs/HOME-BUNDLE-DOWNLOAD.md` §2）：
   *
   *     底账正指着这个 combo        → 已应用（定格，点不了）
   *     套餐里缺任一份              → 下载并应用   ← 缺 + 漂同时存在时也是这一态
   *     无缺、但有漂的              → 更新并应用
   *     全齐且全新                  → 应用（一个字节都不重下）
   *
   *   判不了（套餐没拿到 / 清单还没回来）= `unknown`：卡住，**不许**滑进「应用」。
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

  const bundle = useBundleFiles(sel.model, sel.variant)
  /** 「应用」的目标：套餐里那一份 MKP（只有它能被应用；BBS 这一轮只落盘） */
  const presetFile = bundle.presetFileName

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

  const actionLabel = applying
    ? '应用中…'
    : comboApplied
      ? '已应用'
      : bundleState.kind === 'unknown'
        ? bundle.problems.length > 0
          ? '套餐未配置'
          : '读取中…'
        : bundleState.kind === 'missing'
          ? '下载并应用'
          : bundleState.kind === 'drift'
            ? '更新并应用'
            : '应用'
  const actionDisabled = comboApplied || applying || bundleState.kind === 'unknown'

  const applyCurrent = useCallback(async () => {
    if (applying || bundleState.kind === 'unknown' || presetFile === null) return
    setApplying(true)
    setApplyError(null)
    /* 预判：这一批里缺的与漂的（已去重）。空 = 全齐全新 —— 直接应用，一个字节都不重下 */
    const todo = bundleState.todo
    try {
      if (todo.length > 0) {
        /* 逐份看结局：**任何一份没成就停下、不应用** —— 套餐缺一份就是没齐（不编成功） */
        const outcomes = await api.downloadCatalogFiles(todo)
        const failed = outcomes.filter((o) => !o.ok)
        if (failed.length > 0) {
          throw new Error(
            `套餐没下全，先不应用：${failed.map((o) => `${o.fileName}（${o.message}）`).join('；')}`,
          )
        }
      }
      /*
       * 用的是**官方那一份本尊**（2026-10-08 资源库改判）：官方是模板，用户要用的就是它 ——
       * 「使用」在本机没有字节（或盘上那份与目录对不上）时**按需取回**，再写成当前使用
       * （`use_official_preset` 内部走的就是那条下载管道）。
       *
       * 原来那条"先补一份工作副本、再应用副本"的路已经退场：副本只能由「另存为我的预设」
       * 显式产生 —— 所以"用一下官方，本地就凭空多出一份"这回事不会再发生。
       *
       * 也**不必**再给"预判过期"留兜底：字节在不在这一层由后端自己解决，
       * 不存在"预判说齐了、其实没齐"那种错 —— 上面那次批量下载只负责套餐里**别的**文件
       * （切片器配置那一类）。
       */
      await api.useOfficialPreset(presetFile)
      appStateMutated()
    } catch (e) {
      setApplyError(errorText(e))
    } finally {
      setApplying(false)
      /* 投递面广播：本页重读那两份单子（订阅在上面），**别的页**（预设页本地表 /
         BBS 交付面）也收得到 —— 成了一半也是变了，所以放在 finally 里 */
      deliveryMutated()
    }
  }, [applying, bundleState, presetFile])

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
   * 应用了「我的文件」，这里就说我的文件（与预设页横幅同源）；没命中维持目录那份。
   * 覆盖只在目录那份就绪后发生：waiting / failed 的三态动画不动。
   */
  const presetReady = preset.status === 'ready'
  const displayName = activeForSel !== null && presetReady ? activeForSel.fileName : presetName
  const displayPath =
    activeForSel !== null && presetReady
      ? (activeForSel.path ?? presetInfo?.path ?? null)
      : (presetInfo?.path ?? null)
  const inUseNote =
    activeForSel === null || !presetReady
      ? undefined
      : activeForSel.origin === 'mine'
        ? '正在使用 · 我的文件'
        : '正在使用'
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
  const entryLabel = modelName ? '选择版本' : '选择机型'
  const artAlt = [brandName, modelName].filter(Boolean).join(' ') || '未选择机型'

  const sheets: Sheet[] = useMemo(
    () => [
      {
        id: 'machine',
        node: sel.model ? (
          /* 不给眉标题：机型代号（A1_MINI）与下面那行大标题（A1 mini）说的是同一件事，
             写两遍是复读。去掉之后 data-bare-top 生效，整块内容还往上抬了一截 */
          <CardFrame
            navs={[
              { label: '更换机型', arrow: true, onClick: () => deckRef.current?.jumpTo(1) },
            ]}
          >
            <div className={p.info} data-scroll>
              <div className={p.ident}>
                {modelName && <h1 className={p.name}>{modelName}</h1>}

                <button
                  type="button"
                  className={p.variant}
                  onClick={() => deckRef.current?.jumpTo(1)}
                  title="点击更换品牌 / 机型 / 版本"
                >
                  {variantName ?? entryLabel}
                  <span aria-hidden="true">›</span>
                </button>

                {ready && (
                  <>
                    <p className={p.path} title={displayPath ?? undefined}>
                      {displayName ?? '—'}
                    </p>
                    {/* 脚本里的 --Toml 跟着**正在使用的那一份**走（A3：底账命中时是
                        用户那份的路径，不再是目录底稿）；还没取到就先不摆这颗按钮 */}
                    {presetInfo && displayPath && (
                      <CopyAction
                        text={`"${MKP_EXE}" --Toml "${displayPath}" --Gcode`}
                        label="复制后处理脚本"
                      />
                    )}
                  </>
                )}
              </div>

              {ready && (
                <Rows
                  items={[
                    ['X 偏移', axisText(saved, 'x')],
                    ['Y 偏移', axisText(saved, 'y')],
                    ['Z 偏移', axisText(saved, 'z')],
                    ['涂胶速度', presetInfo ? `${presetInfo.speed} mm/s` : '—'],
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
                  <h1 className={p.heroTitle}>快速上手 MKP</h1>
                  <p className={p.heroLead}>选择机型，完成基础配置与校准。</p>
                  <button
                    type="button"
                    className={p.cta}
                    onMouseDown={noFocus}
                    onClick={() => deckRef.current?.jumpTo(1)}
                  >
                    现在开始
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
            navs={[{ label: '回主页', onClick: () => deckRef.current?.jumpTo(0) }]}
            actions={[
              /* 「应用 / 下载并应用 / 已应用」：宽度固定（文字换态不变），在「下一步」旁边。
                 浏览三级选择不算应用 —— 应用只发生在这颗按钮与各处抽屉的明确动作上 */
              ...(ready
                ? [
                    {
                      label: actionLabel,
                      primary: !actionDisabled,
                      fixed: true,
                      disabled: actionDisabled,
                      on: true,
                      onClick: () => {
                        void applyCurrent()
                      },
                    },
                  ]
                : []),
              {
                label: '下一步',
                arrow: true,
                primary: true,
                on: ready,
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
                  三轴只在"没有右侧露出卡"（density mini）时一起摆出来 */}
              {preset.status !== 'idle' && (
                <div className={p.pickerPreset}>
                  <PresetStack state={presetView} inUse={inUseNote} axes={density === 'mini' ? saved : undefined} />
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
            eyebrow="第三步 · Z 偏移校准"
            peekSafe
            tag={dirty ? '未保存' : savedNote ? '已保存' : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            /* 打开模型是这一步的主入口（作者 10-03：绿底给打开模型，预设 pill 退暗） */
            topAction={{
              label: '打开模型',
              accent: true,
              onClick: () => setOpenModel('Z 偏移校准板'),
            }}
            corner={
              /* 预设 pill 挂右上角（作者 10-03：内容列里那一行别占地方）——点开左抽屉；
                 没取到文件名时退成「选择预设」的灰态 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title="点击选择预设"
              >
                <span className={p.presetFileName}>{presetName ?? '选择预设'}</span>
                <span className={p.presetSwitch}>切换</span>
              </button>
            }
            navs={[{ label: '上一步', back: true, onClick: () => deckRef.current?.jumpTo(1) }]}
            actions={[
              { label: '放弃改动', on: dirty, onClick: clearAll },
              {
                label: '保存',
                on: dirty,
                primary: true,
                /* 没有「我的一份」（还没下载这份预设）时存不了：灰着并说清为什么 */
                disabled: !canSave,
                title: canSave ? undefined : NEED_COPY,
                onClick: commitAll,
              },
              {
                label: '下一步',
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
            ]}
          >
            {/* data-hint 与下面那行提示同源：板框里多一行，板子就得为它让出高度
                （没有它的话列会溢出、提示文字压在底部胶囊上） */}
            <div
              className={`${p.info} ${p.calib}`}
              data-hint={saved ? undefined : 'true'}
              data-scroll
            >
              {/* 预设入口已挪到卡片右上角（corner），内容列只留三轴读数 */}
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={axisView}
                activeAxis="z"
                presetUid={currentUid}
                presetOptions={presetOptions}
                onPickPreset={pickPreset}
                canEdit={canPick}
                showPreset={false}
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
                    theme={PLATE_THEME}
                    label="Z 偏移校准板"
                    onPick={pickZ}
                    picked={zSelected}
                    hitLabel={zHitLabel}
                  />
                </PlateZoom>
                {/* G06-8 攒下的两行文案，与「校准」tab 同一句原话（作者 10-03：首页也缺这个） */}
                <p className={p.legend}>{Z_LEGEND}</p>
                <p className={p.tip}>{Z_TIP}</p>
                {/* 没取到预设时点板子不产生读数，这一行是唯一的解释，必须留。
                    其余说明去掉了：点了哪一格、改成多少，上面三轴读数里的「旧 → 新」已经说完 */}
                {!saved && <p className={p.note}>{NEED_PRESET}</p>}
                {/* 有基准、但还没有「我的一份」：读数能用，改动却存不进任何地方 */}
                {saved && !canSave && <p className={p.note}>{NEED_COPY}</p>}
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'calib-xy',
        node: (
          <CardFrame
            eyebrow="第四步 · XY 偏移校准"
            peekSafe
            tag={dirty ? '未保存' : savedNote ? '已保存' : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            topAction={{
              label: '打开模型',
              accent: true,
              onClick: () => setOpenModel('XY 偏移校准板'),
            }}
            corner={
              /* 同 Z 步：预设 pill 挂右上角，点开左抽屉 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title="点击选择预设"
              >
                <span className={p.presetFileName}>{presetName ?? '选择预设'}</span>
                <span className={p.presetSwitch}>切换</span>
              </button>
            }
            navs={[
              {
                label: '上一步',
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_Z_INDEX),
              },
            ]}
            actions={[
              { label: '放弃改动', on: dirty, onClick: clearAll },
              {
                label: '保存',
                on: dirty,
                primary: true,
                /* 没有「我的一份」（还没下载这份预设）时存不了：灰着并说清为什么 */
                disabled: !canSave,
                title: canSave ? undefined : NEED_COPY,
                onClick: commitAll,
              },
              {
                label: '下一步',
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_MODEL_INDEX),
              },
            ]}
          >
            <div
              className={`${p.info} ${p.calib}`}
              data-plate="xy"
              data-hint={saved ? undefined : 'true'}
              data-scroll
            >
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={axisView}
                activeAxis="xy"
                presetUid={currentUid}
                presetOptions={presetOptions}
                onPickPreset={pickPreset}
                canEdit={canPick}
                showPreset={false}
                onType={typeAxis}
                onRevert={revertAxis}
                onReset={resetAxis}
              />

              <section className={p.step}>
                {/* 同 Z 步：去容器、去板名，手柄直角实白；XY 板内自带整段说明，不给字 */}
                <PlateZoom resetKey="xy" title="">
                  <CalibPlate
                    model="precise"
                    theme={PLATE_THEME}
                    label="XY 偏移校准板"
                    onPick={pickXY}
                    picked={xySelected}
                    hitLabel={xyHitLabel}
                  />
                </PlateZoom>
                {!saved && <p className={p.note}>{NEED_PRESET}</p>}
                {/* 有基准、但还没有「我的一份」：读数能用，改动却存不进任何地方 */}
                {saved && !canSave && <p className={p.note}>{NEED_COPY}</p>}
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'models',
        node: (
          <CardFrame
            eyebrow="第五步 · 测试模型"
            peekSafe
            navs={[
              {
                label: '上一步',
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
            ]}
            actions={[
              {
                label: '打开测试模型',
                primary: true,
                onClick: () => setOpenModel('测试模型'),
              },
              { label: '回主页', onClick: () => deckRef.current?.jumpTo(0) },
            ]}
          >
            <div className={`${p.info} ${p.models}`} data-scroll>
              {/* 不给页级大标题：眉标题那行已经写着「第五步 · 测试模型」，
                  再写一遍是复读（首页与选机型页去掉眉标题是同一个道理）。
                  这一行只说"接下来做什么"，不重复页名 */}
              <p className={p.modelsLead}>打印这套模型，检查校准结果。</p>
              <div className={p.modelsHero}>
                <img
                  className={p.modelsHeroImg}
                  src={heroPile}
                  srcSet={`${heroPile} 1x, ${heroPile2x} 2x`}
                  alt="测试模型"
                  draggable={false}
                />
              </div>
              <p className={p.modelsCaption}>
                鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景
              </p>
            </div>
          </CardFrame>
        ),
      },
    ],
    [
      artAlt,
      axisView,
      brandOptions,
      canPick,
      canSave,
      clearAll,
      commitAll,
      currentUid,
      density,
      dirty,
      displayName,
      displayPath,
      draft,
      drop,
      entryLabel,
      fadeMs,
      inUseNote,
      layers,
      modelName,
      modelOptions,
      pick,
      pickPreset,
      pickXY,
      pickZ,
      preset,
      presetInfo,
      presetName,
      presetOptions,
      presetView,
      ready,
      resetAxis,
      revertAxis,
      saved,
      savedNote,
      sel,
      actionDisabled,
      actionLabel,
      applyCurrent,
      applyError,
      bundle,
      settle,
      typeAxis,
      variantName,
      variantOptions,
      xySelected,
      zSelected,
    ],
  )

  return (
    <div className={p.page} data-density={density}>
      <SlideDeck ref={deckRef} sheets={sheets} density={density} canLeave={canLeave} />

      {/* 预设抽屉：从左缘出（右边常驻露出卡，没地方）。换预设的确认（有草稿先问一句）
          走 pickPreset，与下拉时代同一条路；抽屉挂在 .page（container 的布局包含块）上 */}
      <PresetPickerDrawer
        side="left"
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

      {openModel && (
        <Modal
          title="打开测试模型"
          onClose={() => setOpenModel(null)}
          foot={
            <>
              <Btn disabled>从本地缓存打开</Btn>
              <Btn variant="primary" onClick={() => setOpenModel(null)}>
                从云端获取
              </Btn>
            </>
          }
        >
          <p className={p.note}>
            {openModel} · 本地无缓存文件，即将从云端下载打开。点击后请耐心等待 3mf 打开。
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
                  leaveTo(pending.to)
                }}
              >
                放弃改动并离开
              </Btn>
              {canSave && (
                <Btn
                  variant="primary"
                  onClick={() => {
                    commitAll()
                    leaveTo(pending.to)
                  }}
                >
                  保存并离开
                </Btn>
              )}
            </>
          }
        >
          <p className={p.note}>
            偏移改动还没保存：
            {/* 三轴在哪一页都能改，所以这里列的是**全部**脏轴，不再按页筛。
                走到这个弹窗一定有草稿，也就一定有基准值；saved / draft 的判空只是给类型看 */}
            {saved && draft
              ? AXIS_ROWS.filter(([, k]) => dirtyAxes.includes(k))
                  .map(([label, k]) => `${label} ${saved[k].toFixed(2)} → ${draft[k].toFixed(2)}`)
                  .join('，')
              : ''}
            {canSave
              ? '。离开前要保存吗？'
              : '。这份还没有「你的一份」—— 先在预设页另存为我的预设才能存，现在只能放弃。'}
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
          <p className={p.note}>
            换成 {presetOptions.find((o) => o.uid === presetAsk)?.name ?? '另一份预设'} 之后，
            这一页点出来的偏移草稿会作废 —— 草稿是相对上一份预设点出来的增量，
            换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
