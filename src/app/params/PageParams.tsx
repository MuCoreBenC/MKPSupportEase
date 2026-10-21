/*
 * 修改参数 —— 用户端。
 *
 * # 数据全部来自**同步下来的那份包**，不再来自手抄表和跨稿 import
 *
 * 分类计数来自 `ParamTab.count`（6 个分类：偏移 5 / 擦料 22 / 风扇 3 / 涂胶 18 / 切换 2 / 更多 17 = 67），
 * 卡片标题来自 `ParamSection.label`（16 个真 section 名），
 * 可见性条件来自 `ParamMeta.showWhen`（支持 eq / neq / gt），
 * 字段的选项、范围、单位来自 `RecipeParam`。
 *
 * 手抄表的涂胶 31 / 更多 4 和真注册表的涂胶 18 / 更多 17 总数相同但切分不同。
 * 这里以真注册表为准。
 *
 * # 搜索跨全部分类
 *
 * 验证时踩过「只搜当前页签得到 0 条，而那个字段其实在另一类里」。
 * 这里的搜索去掉分类筛选，命中项显示所属分组名。支持按 label / key / tomlKey 三样匹配。
 *
 * # gcode 字段走 renderBlock
 *
 * 用户端给整块编辑器（工作台的「N 行 · 点开」是另一个取向）。
 * 实测 control 分布：42 number / 13 choice / 10 switch / 2 text（那 2 个 text 就是 gcode）。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { STORAGE } from '../../api/storageKeys'
import { useStickyState } from '../../hooks/useStickyState'
import type { Density } from '../../hooks/useDensity'
import { FieldLayer } from '../../components/field'
/*
 * 「修改历史」在模态框里会搬到标题栏 —— 那颗动作槽由通用模态框给（见 modalHeadSlot.ts）；
 * 保存回执（「已写回 xxx.toml」）走同一套机制的**回执槽**。
 */
import { useModalHeadNoteSlot, useModalHeadSlot } from '../../components/modal'
import CategoryPills from './CategoryPills'
import GcodeBlock from './GcodeBlock'
import HistoryDrawer from './HistoryDrawer'
import type { DrawerMode, HistoryView } from './HistoryDrawer'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import ParamCard from './ParamCard'
import type { TowerSlot } from './ParamCard'
import ResliceNotice from '../guide/ResliceNotice'
import ParamSyncModal from './ParamSyncModal'
import PresetPickerDrawer from './PresetPickerDrawer'
import { useLocalPickerRows } from '../presets/localPickerRows'
import SearchField from './SearchField'
import Icon from '../shell/icons'
import TowerMap from './TowerMap'
import type { TowerCoreProps } from './TowerCoreSvg'
import { invalidNumberText, useParams, valueText } from './useParams'
import type { HistoryEntry, ParamDef } from './useParams'
/* 两态框架那两颗按钮的词 —— 与「编辑文件」/「参数对比台」**同一个源**（作者要的"三处同一句话"） */
import { EDIT_TEXT } from '../presets/presetTree'
import s from './PageParams.module.css'

interface Props {
  density: Density
  /**
   * **嵌在模态框里**（右键某一份预设打开的那一档）。
   *
   * 只改版面，不改组件：去掉页面自己的大标题与预设切换 pill（外面那个框的标题就是
   * 这一份的名字），分类条那一排照旧。卡片、G-code、擦料塔画布、历史抽屉一个不动。
   */
  /*
   * ★ 活口（2026-10-14 标记）：**唯一的调用点 `PresetParamsModal` 恒传 `true`**
   * （参数页现在只以"嵌在预设弹窗里"这一档出现），所以下面那两处 `{!embedded && …}`
   * （"修改参数"整页标题那一套壳）**当前永不渲染**。
   *
   * 它们不是历史垃圾：`App.tsx` 的注释里"参数"本来就有**整页**那一档的身份
   * （从预设行右键「打开参数」进去的是同一份 `PageParams`）—— 哪天要给它一个独立入口，
   * 这两块就是现成的壳。
   */
  embedded?: boolean
  /** 钉住编辑目标（嵌模态框时由调用方点名：改的就是这一份） */
  target?: { path: string; fileName: string; machineId: string; versionId: string }
  /** 一进来就把「选择要跟随的更新」那一屏打开 */
  openSyncOnStart?: boolean
  /**
   * **编辑模式闸**（2026-10-16，作者：「不管是对比还是那个修改参数，他们的那个
   * 编辑模式布局几乎都要一样」）：true 时**打开先只读**（控件全体降成纯文本值，
   * 底栏只有「进入编辑模式」），点它才进入可改的那一态；false（默认）打开即可改。
   *
   * `PresetParamsModal`（当前唯一调用点）恒传 true —— 与参数对比台同一个框架：
   * 只读态 → 进编辑模式 → 撤销 / 重做 / 放弃并退出 / 保存（词在 `EDIT_TEXT`）。
   * 「放弃并退出」= **作废**这一次没保存的改动（回只读；见下面 `editMode` 那段）。
   */
  gateEdit?: boolean
}

interface ShownCard {
  id: string
  title: string
  subtitle?: string
  fields: ParamDef[]
}

/** 塔体固定 19.58mm —— 参考程序的原值；包围盒 = 塔体 + 外围结构最大扩展量 × 2 */
const TOWER_BODY_MM = 19.58
/** 塔身画布固定画成 44px，mm → px 的比例由它定（TowerCoreSvg 的既定尺寸） */
const TOWER_CORE_PX = 44

/** TOML 原文 → 数字。空着 / 不是数时给一个安全的默认，别让 NaN 进几何 */
function towerNumOf(raw: string, fallback: number): number {
  const n = Number(raw)
  return Number.isFinite(n) ? n : fallback
}

/**
 * 造「擦料塔位置与打印」卡的塔地图槽位（ParamCard 的 `tower` prop）。
 *
 * 数据全部现读包与值层，没有一处手抄：
 *   热床尺寸    `machine.dimensions`（包里带；没配尺寸时退回 256×256 / 留白 10mm）
 *   塔位置      `wiping.wiper_x / wiper_y` —— **包围盒左下角**，机器坐标 y 向上
 *   外围结构    `wiping.outer_structure` + rib_* / sheath_base_expand
 *   板轮廓      按机型的 `defaultPlateId` 从 catalog 的 plates 查（没配板就退回圆角矩形）
 *
 * 拖拽与轴输入的提交都走这一页的 `u.apply`：一次手势一条撤销、还原胶囊指已保存值。
 *
 * `disabled`（2026-10-16 编辑模式两态）：只读态里地图拖不动 —— 按下直接不响应。
 *
 * `onDrag`（2026-10-17）：拖动中的临时位置往上抛，让右侧 X/Y 两行**显示**跟着走
 * （只有显示跟着走，草稿不动 —— 一次拖动仍然只产生一条撤销）。
 */
function towerSlotOf(
  u: ReturnType<typeof useParams>,
  disabled: boolean,
  onDrag?: (pos: { x: number; y: number } | null) => void,
): TowerSlot | undefined {
  const dims = u.machine?.dimensions ?? null
  const xDef = u.defOf('wiping.wiper_x')
  const yDef = u.defOf('wiping.wiper_y')
  if (xDef === undefined || yDef === undefined) return undefined
  const width = dims !== null && dims.bedSize.width > 0 ? dims.bedSize.width : 256
  const depth = dims !== null && dims.bedSize.depth > 0 ? dims.bedSize.depth : 256
  const edgeZone = dims?.edgeZone ?? 10

  /*
   * 统一包围盒（参考程序同一条公式）：扩展量取三种外围结构里最大的 ——
   * 护套按底层膨胀量、斜肋按 45° 摆放的水平投影（额外长度 / √2 + 宽度 / 2√2）。
   * 三种模式下塔方块一样大，左下角坐标是同一个含义。
   */
  const outerRaw = u.valueOf('wiping.outer_structure')
  const outerVal = outerRaw === 'rib' || outerRaw === 'sheath' ? outerRaw : 'brim'
  const ribWidthMm = Math.max(0, towerNumOf(u.valueOf('wiping.rib_width'), 0))
  const ribExtraLengthMm = Math.max(0, towerNumOf(u.valueOf('wiping.rib_extra_length'), 0))
  const ribFilletWall = u.valueOf('wiping.rib_fillet_wall') === 'true'
  const sheathExpand = Math.max(0, towerNumOf(u.valueOf('wiping.sheath_base_expand'), 5))
  const ribExpand = ribExtraLengthMm / Math.SQRT2 + ribWidthMm / (2 * Math.SQRT2)
  const maxExpand = Math.max(sheathExpand, ribExpand)
  const vm = TOWER_CORE_PX / TOWER_BODY_MM

  const core: TowerCoreProps = {
    outerVal,
    isRibVisible: outerVal === 'rib' && ribWidthMm > 0,
    ribWidthMm,
    ribExtraLengthMm,
    ribFilletWall,
    vbOffset: -maxExpand * vm,
    vbSize: TOWER_CORE_PX + maxExpand * vm * 2,
    vm,
    brimLoopCount: 5,
  }

  /* 板按机型的默认板选（`defaultPlateId` → catalog.plates）—— 不按机型 id 硬编码；
     没配板就 undefined（画布退回圆角矩形，与参考程序对无板机型的处置一致） */
  const plate = u.plateOf()

  return {
    xKey: xDef.key,
    yKey: yDef.key,
    render: (xDef, yDef, side) => (
      <TowerMap
        bedW={width}
        bedD={depth}
        edgeZone={edgeZone}
        towerSize={TOWER_BODY_MM + maxExpand * 2}
        x={towerNumOf(u.valueOf(xDef.key), 0)}
        y={towerNumOf(u.valueOf(yDef.key), 0)}
        savedX={towerNumOf(u.savedValueOf(xDef.key), 0)}
        savedY={towerNumOf(u.savedValueOf(yDef.key), 0)}
        core={core}
        plate={plate}
        side={side}
        disabled={disabled}
        /* 拖动中的位置往上抛：页面拿它喂右侧 X/Y 两格的显示（草稿不动） */
        onDrag={onDrag}
        onCommit={(nx, ny) =>
          u.apply('移动擦料塔', [
            { key: xDef.key, value: String(nx) },
            { key: yDef.key, value: String(ny) },
          ])
        }
      />
    ),
  }
}

/** 双列：贪心按高度平衡，卡片高度用字段数近似 */
function splitDualColumns(cards: ShownCard[]): [ShownCard[], ShownCard[]] {
  const left: ShownCard[] = []
  const right: ShownCard[] = []
  let lh = 0
  let rh = 0
  for (const item of cards) {
    const h = item.fields.length + 1
    if (lh <= rh) {
      left.push(item)
      lh += h
    } else {
      right.push(item)
      rh += h
    }
  }
  return [left, right]
}

const MAX_CONFIRM_ITEMS = 8
const DUAL_MIN_WIDTH = 1180

/** 「最近搜索」留几条 —— 再多就把下拉变成一份清单了 */
const HISTORY_MAX = 8
/** 存的键收在 `src/api/storageKeys.ts`（值仍是试验场那个键名），刷新之后还在 */
const HISTORY_KEY = STORAGE.clientParamsSearchHistory

/**
 * 命中项的全部**受控参数**（顺着 `showWhen.key` 往下走，孙子也算）。
 *
 * 词先说清（作者：「不要叫他父级吧，叫做什么好呢」）：
 * **条件参数** = 它的值决定别人能不能用；**受控参数** = 被那个条件管着的那些。
 * 关系叫**条件依赖**，行为叫**条件显隐**（配置界面里的通用说法，英文 conditional
 * parameter / conditional visibility）。用户面上不需要名词 —— 那句
 * 「需先让『支撑面熨烫』等于 自动」已经把因果说完了。
 *
 * 作者搜「支」，结果里只有「支撑面熨烫」自己 —— 它下面那条受控参数
 * （熨烫挤出阈值）名字里不带「支」，没出来。他要的是「搜索到条件参数也要显示受控参数」。
 *
 * 关系只有一条：`meta.showWhen.key` 就是条件参数（`useParams` 的 `findBlocked`
 * 也是顺着它往上走的）。这里反过来用一遍：条件参数 key → 受控 keys，从命中项往下逐个收。
 * `out` 同时当访问标记，注册表里真有环也转不住。
 *
 * **`defs` 传一张卡的**：于是走到卡外的路自动断掉 ——
 * 「擦料方式」那种模式开关一命中，就不会把跨卡的两片互斥分支全拖出来。
 */
function descendantsOf(defs: ParamDef[], roots: Set<string>): Set<string> {
  const childrenByParent = new Map<string, string[]>()
  for (const def of defs) {
    const parent = def.meta?.showWhen?.key
    if (parent === undefined) continue
    const list = childrenByParent.get(parent)
    if (list === undefined) childrenByParent.set(parent, [def.key])
    else list.push(def.key)
  }

  const out = new Set<string>()
  const queue = [...roots]
  while (queue.length > 0) {
    const at = queue.shift() as string
    for (const child of childrenByParent.get(at) ?? []) {
      if (out.has(child) || roots.has(child)) continue
      out.add(child)
      queue.push(child)
    }
  }
  return out
}

/**
 * 命中项的**条件参数**（顺着 `showWhen.key` 往上走，爷爷也算）—— 上一段的另一半。
 *
 * 作者：「需要显示父参数」。命中一个受控参数时（比如直接搜「熨烫挤出阈值」），
 * 结果里只有它自己，还挂着一句「需先让『支撑面熨烫』等于 自动」—— 而**要开的那一项
 * 不在眼前**，那是一句没有主语的话。把条件行一起带出来，它就与分类里那一屏长得一样了
 * （条件行 + 灰底子块）。
 *
 * 两处「就停」：
 *   · 走到 `defs` 里没有的条件参数就停 —— 卡外的不带（见 `descendantsOf`）；
 *     顺带也管住了「被 machineFilter 排除的字段」（这台机型上无从满足、也无从开启，
 *     与 `useParams` 的 `findBlocked` 同一条规则，否则界面会写
 *     「需先开启 某个本机型根本没有的项」）。
 */
function ancestorsOf(defs: ParamDef[], roots: Set<string>): Set<string> {
  const parentByKey = new Map<string, string>()
  const known = new Set<string>()
  for (const def of defs) {
    known.add(def.key)
    const parent = def.meta?.showWhen?.key
    if (parent !== undefined) parentByKey.set(def.key, parent)
  }

  const out = new Set<string>()
  const queue = [...roots]
  while (queue.length > 0) {
    const at = queue.shift() as string
    const parent = parentByKey.get(at)
    if (parent === undefined || !known.has(parent)) continue
    if (out.has(parent) || roots.has(parent)) continue
    out.add(parent)
    queue.push(parent)
  }
  return out
}

/**
 * 分类条那一行要同时装下「分类条 + 搜索框」需要多宽 —— 按档位三个**实测**常量。
 *
 * 每个数 = 分类条整块的自然宽（含右边那截当前分类名）+ 缝 + 搜索框槽宽 + 这一行的
 * 左右内衬 `2×(pad+10)` + 12px 余量：
 *
 *   mini           246 + 8  + 132 + 42 + 12 = 440   ← 搜索框 160 → 132
 *   compact        332 + 12 + 132 + 42 + 12 = 530
 *   wide / ultra   469 + 12 + 216 + 42 + 12 = 751
 *
 * 卡片区窄过它，搜索框就搬到头部标题行（那行本来就有空白）—— 作者：
 * 「我希望圈起来的能显示完整，也就是说这个搜索在特定窗口大小下可以移动到上面」。
 *
 * 实测只有 mini 档（窗口 < 640）会真的触发：窗口 460 及以下（卡片区 ≤ 458）搬上去，
 * 480 起（卡片区 478）留在分类条那一行。compact 与 wide 的卡片区下限（598 / 1038）
 * 都远大于各自的阈值，永不触发。
 *
 * # 为什么**不**跟着搜索框一起往下降
 *
 * 搜索框收窄了 28px（160 → 132），照公式三个常量本该同步降 28。但分类条那一侧
 * 同一轮改了退化规则（条外那截名字**不再**在 mini 档藏起来），mini 的自然宽
 * 从 246 变成约 315（246 + 名字那一段 69）。两边一升一降，真需要的地方反而更多，
 * 所以三个数**一个没动**：它们的含义是「什么时候该把这一行让给分类条」，
 * 降下来只会让分类条在更窄的地方被挤到横滚（实测过：卡片区 460 那一档会缺 17px）。
 * 实测扫描 400→1100 每 20px：分类条**一次都没有横滚**，就是靠这三个数兜住的。
 */
const BAR_NEED: Record<Density, number> = {
  ultra: 775,
  wide: 775,
  compact: 558,
  mini: 468,
}

/**
 * 量一个元素有多宽 —— 用**回调 ref**，不用 `useEffect + RefObject`。
 *
 * 为什么换写法（实测出来的，前后两版都栽在这上面）：
 * 原来是 `useEffect(..., [ref])` 里读 `ref.current` 再建 ResizeObserver。
 * 但这一页有一个早期 return：
 *
 *     if (u.loading) return <p>正在打开数据包…</p>
 *
 * 首次渲染走的是那一条，`.bodyWrap` 根本不在 DOM，effect 跑的时候 `ref.current === null`
 * 直接 return —— **RO 从来没被创建过**。等数据包读出来、元素真出现时，effect 的依赖
 * `[ref]`（ref 对象永远是同一个）没变，effect 不重跑，于是 RO 永远不会建立，
 * `width` 永远停在初始值 0 → `0 >= 1180` 恒假 → **双列永远不开**。
 *
 * 实测证据：`.bodyWrap` 的 `clientWidth = 1598`，而组件里的 `bodyWidth` state 是 `0`。
 *
 * 更早那一版的截图之所以是双列：它的参数数据来自手编表（同步，没有 loading 这一档），
 * 元素首帧就在 DOM 里，effect 一次就量到了。接真契约引入异步加载之后，
 * 顺手把双列关掉了，谁都没注意到。
 *
 * 回调 ref 由 React 在**元素每次挂载 / 卸载时**调用，晚出现也会被接上，
 * 不存在「跑太早、之后不再重试」这件事。
 */
function useWidth(): [(el: HTMLElement | null) => void, number] {
  const [width, setWidth] = useState(0)
  const roRef = useRef<ResizeObserver | null>(null)

  const attach = useCallback((el: HTMLElement | null) => {
    /* 换元素（或卸载）时先断开上一个，免得留着观察一个已经不在的节点 */
    roRef.current?.disconnect()
    roRef.current = null
    if (el === null) return
    setWidth(el.getBoundingClientRect().width)
    const ro = new ResizeObserver((entries) => setWidth(entries[0].contentRect.width))
    ro.observe(el)
    roRef.current = ro
  }, [])

  return [attach, width]
}

export default function PageParams({
  density,
  embedded = false,
  target,
  openSyncOnStart = false,
  gateEdit = false,
}: Props) {
  /* 标题栏两个借位槽（只有挂在模态框里才有值）—— hook 必须在所有 early return 之前调 */
  const headSlot = useModalHeadSlot()
  const headNoteSlot = useModalHeadNoteSlot()
  const u = useParams(target === undefined ? undefined : { target })

  /*
   * **编辑模式**（2026-10-16，作者：「进入编辑模式的那个按钮位置也统一一下」）。
   *
   * `gateEdit` 时打开先只读 —— 控件全体降成纯文本值、底栏只有「进入编辑模式」
   * （与参数对比台、编辑文件两个模态框同一个框架）；点它进编辑模式，底栏换成
   * 撤销 / 重做 / 放弃并退出 / 保存。退出 = **作废**（作者 2026-10-16：「没有保存的话，
   * 退出编辑模式就是作废，那些改的都作废，保存了才有」）—— 走 `u.discardAll()`：
   * 全部未保存的改动一次退回已保存值；要留住改动只有「保存」一条路。
   *
   * ★ 例外：`openSyncOnStart`（右键「选择要更新的参数」那条直路）**直接进编辑模式**
   * —— 用户是带着"来处理更新"的意图点进来的，再让他先点一下「进入编辑模式」
   * 是白拦一道（那屏里的采用 / 保持本来就是写操作）。
   */
  const [editMode, setEditMode] = useState(gateEdit !== true || openSyncOnStart)
  /*
   * **带着没保存的草稿重开 → 直接进编辑模式**（2026-10-19）。
   *
   * `useParams.draftOnDisk` = 后端摘要说"盘上这份草稿真跟那份文件不一样"。
   * 作者 2026-10-19：「我没保存的时候关闭它应该是记住着，然后下次我再进来的时候，
   * 它一样还是保留着那个编辑模式的」。与兄弟链「编辑文件」抽屉的
   * `unsaved = draft.reused && draft.dirty` 同一条口径（那边在 `PagePresets` 里
   * 直接 `setEditMode(unsaved)`）。
   *
   * **只自动进一次**：用户点了「放弃并退出」（作废）之后就不再替他进 —— 那时他要的是只读。
   */
  const autoEditConsumedRef = useRef(false)
  useEffect(() => {
    if (autoEditConsumedRef.current) return
    if (gateEdit !== true || u.draftOnDisk !== true) return
    autoEditConsumedRef.current = true
    setEditMode(true)
  }, [gateEdit, u.draftOnDisk])
  /** 只读 = 有闸且还没进编辑模式 */
  const ro = gateEdit && !editMode

  const [categoryId, setCategoryId] = useState('')
  const [query, setQuery] = useState('')
  const [historyOpen, setHistoryOpen] = useState(false)
  const [historyMode, setHistoryMode] = useState<DrawerMode>('float')
  /* 默认「逐项」（一项一行）；另一档「整批」是一次保存一行、一次退整段 */
  const [historyView, setHistoryView] = useState<HistoryView>('item')
  const [expandedDetailKey, setExpandedDetailKey] = useState<string | null>(null)
  const [askRestore, setAskRestore] = useState(false)
  const [askSave, setAskSave] = useState(false)
  const [attachBody, bodyWidth] = useWidth()
  /**
   * 「选择要更新的参数」那一屏的开合。**文案 2026-10-20 改过**：原来叫「选择要跟随的
   * 更新」（"跟随"讲的是机制，用户点它时想的是"哪几项要改成云端的新值"）。
   * `openSyncOnStart` 是右键菜单那条直路（＝他已经知道有更新等着处理）。
   */
  const [syncOpen, setSyncOpen] = useState(openSyncOnStart)
  /**
   * 「选择要更新的参数」那一屏的宿主层（2026-10-20）。
   *
   * 点 `.page` 的**父节点**（在「修改参数」模态框里就是 `.paramsLayer`）——
   * 遮罩挂在那儿才铺满整块内容区、不被框的圆角裁掉；`fallback` 是独立页那一档
   * （没有父层时退回 `.page` 自己，与原来是同一处）。
   */
  const [syncLayer, setSyncLayer] = useState<HTMLElement | null>(null)
  const mountSyncLayer = useCallback((el: HTMLDivElement | null) => {
    /* `el === null` 时**不改**：卸载那一刻换宿主会凭空重挂一次遮罩 */
    if (el !== null) setSyncLayer(el.parentElement ?? el)
  }, [])
  /*
   * 走那条直路进来 ⇒ **进来就把云端新版取回来**（2026-10-20）。
   *
   * 用户是带着"来处理更新"的意图点进来的，他自己已经传递了"我愿意等这一次网络"；
   * 而普通打开参数页是不取的（那时只说"云端发了新版"）。
   */
  useEffect(() => {
    if (openSyncOnStart !== true) return
    void u.fetchSync()
    /* 只在进来时跑这一次（`u` 每次渲染都是新的，跟它走会反复取） */
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openSyncOnStart])
  /** 预设抽屉的开合 + pill 的 ref（关抽屉后焦点要还回去） */
  const [pickerOpen, setPickerOpen] = useState(false)
  const pillRef = useRef<HTMLButtonElement>(null)
  /** 搜索框的 ref —— `Ctrl+F` 把焦点放进去（槽在头部还是分类条那一行由布局定） */
  const searchRef = useRef<HTMLInputElement | null>(null)
  /**
   * mini 档搜索**是否收起**（作者：「这个搜索框还是不要这样子，它多占了位置」）——
   * 这一档输入框常驻就是整整一行（上面还带着头部内衬），收起之后只剩一颗放大镜
   * 图标（28px），点开才在分类条那一行铺开输入框（见 `searchCollapsed`）。
   */
  const [searchOpen, setSearchOpen] = useState(false)
  /**
   * mini 档那颗放大镜是**用户点的**吗？
   *
   * ★ 只有点它才给焦点（作者：「我本来没有点它的话，我变小了还会自己聚焦上去，
   *   这是不对的」）—— 原来那个 effect 只看「展开没展开」，于是窗口一跨到 mini 档、
   *   或从 mini 档回来，输入框都会自己抢焦点。现在用一个一次性的意图标记，
   *   把"聚焦"这个动作绑死在**点击**这一个入口上。
   */
  const wantSearchFocus = useRef(false)

  const closePicker = useCallback(() => {
    setPickerOpen(false)
    pillRef.current?.focus()
  }, [])

  /*
   * 抽屉的行 = **本机真有**的预设（`presets-local/` 那一份扫描结果摊出来）。
   * ★ 与页面上的 `u.machines`（目录）**不是同一份数据**，别互相替换：
   *   目录那一套是"云端提供什么"（供的是说明书），抽屉要的是"我盘上有什么"。
   */
  const pickerRows = useLocalPickerRows(u.localPresets, u.machines)

  /* 预设抽屉的宽度：拖内缘改宽 + 三态吸附（`shared/useDrawerWidth`，与参数页历史抽屉同一套） */
  const pickerDrag = useDrawerWidth({
    storageKey: STORAGE.clientPresetPickerW,
    side: 'right',
    snap: 320,
    min: 240,
    max: 560,
    label: '预设列表',
  })

  /**
   * 「最近搜索」（作者：「我点击搜索的时候没有展开搜索历史呀」）。
   * 放在页面这一层而不是搜索框里：那个框窄窗时会从分类条那一行搬进标题行，
   * React 认的是两个不同的位置 —— 状态留在框里就会被搬丢。
   */
  const [searchHistory, setSearchHistory] = useStickyState<string[]>(HISTORY_KEY, [])

  /** 记一笔：去重（不分大小写）、置顶、封顶 HISTORY_MAX */
  const rememberSearch = useCallback(
    (term: string) => {
      const t = term.trim()
      if (t === '') return
      setSearchHistory((prev) =>
        [t, ...prev.filter((x) => x.toLowerCase() !== t.toLowerCase())].slice(0, HISTORY_MAX),
      )
    },
    [setSearchHistory],
  )

  const forgetSearch = useCallback(
    (term: string) => setSearchHistory((prev) => prev.filter((x) => x !== term)),
    [setSearchHistory],
  )

  /* 当 tabs 到了但 categoryId 还没选（或选的那一项不在 tabs 里），落到第一个 */
  useEffect(() => {
    if (u.tabs.length > 0 && !u.tabs.some((t) => t.id === categoryId)) {
      setCategoryId(u.tabs[0].id)
    }
  }, [categoryId, u.tabs])

  const searching = query.trim() !== ''
  const canPin = density === 'ultra' || density === 'wide'
  const pinned = historyOpen && historyMode === 'pinned' && canPin
  /*
   * 悬浮抽屉的宽度状态：拖左缘改宽、三态吸附回 380、双击/Home 复位、
   * localStorage 记住 —— 全稿同一套（BBS 预设抽屉同源，见 shared/useDrawerWidth）。
   */
  const historyDrag = useDrawerWidth({
    storageKey: STORAGE.clientParamsHistoryDrawerW,
    side: 'right',
    snap: 380,
    min: 280,
    max: 560,
    label: '修改历史',
  })

  /* 当前分类里的 section 列表 */
  const currentTab = u.tabs.find((t) => t.id === categoryId)

  /*
   * 搜索：跨全部分类的就地筛选。
   * 按 label / key / tomlKey 三样匹配（只搜 label 会漏掉用英文名搜的场景）。
   * 命中项显示所属分组名。一条都不命中的 section 整张不出现。
   *
   * 命中项**上下都带** —— 受控参数（`descendantsOf`）与条件参数（`ancestorsOf`）。
   * 一开始他搜「支」只看见「支撑面熨烫」自己，它下面那条「熨烫挤出阈值」不见了；
   * 后来又反过来：「直接搜一个参数时，要能看见它的条件参数」——
   * 否则那句「需先让『支撑面熨烫』等于 自动」说的是一个不在眼前的东西。
   * 带出来的行按 `split()` 摆成条件行 + 灰底子块，与分类里那一屏长得一样。
   *
   * 后来不再数「命中 N 条」（作者：「命中 xxx 都可以不显示了吧」）——
   * 那个数字要数清「被带出来的不算」，而结果本身就在下面按卡列着，没必要再报一遍。
   *
   * # **只在同一张卡里带**（作者：「擦料方式这种很大的父级的怎么办」）
   *
   * 早先的写法是拿 `u.defs`（全机型全分类）去收上下家，于是「擦料方式」这种
   * **模式开关**一命中，它底下那两片互斥分支全被拖出来 —— 实测搜「擦料方式」：
   * **5 张卡 22 行**（擦料塔位置与打印 4 + 安全与高级参数 7 + 塔结构加强 6 +
   * 圆盘动作控制 4），而真正命中的只有它自己。他自己那台程序不是这么办的：
   * 「切换到圆盘擦拭的时候那些擦料塔的都隐藏，切换到擦料塔的时候圆盘擦拭的都隐藏」——
   * 即只看当前那一支。
   *
   * 判据落成一句**同一张卡**：`descendantsOf` / `ancestorsOf` 改喂**这张卡自己的
   * `defsOfSection(sec.id)`**，于是要走的路只认卡里的键 —— 跨卡的那一片自动断在这里
   * （`ancestorsOf` 里本来就有一条「条件参数不在 known 里就停」）。这与灰底子块的判据
   * 天然对齐：受控参数能嵌到条件行下面，前提本来就是它们在**同一张卡**里（见 `split()`）。
   *
   * 改后实测：搜「擦料方式」**1 张卡 1 行**（它自己）；搜「塔体打印速度」**1 张卡 2 行**
   * （同卡的条件参数「擦料塔速度与质量」+ 它自己），擦料方式那张卡不再跟出来。
   * 上面那几个例子不受影响 —— 它们的条件关系本来就在同一张卡里。
   */
  const shown = useMemo<ShownCard[]>(() => {
    if (!searching) {
      if (currentTab === undefined) return []
      return currentTab.sections.map((sec) => ({
        id: `${currentTab.id}-${sec.id}`,
        title: sec.label,
        /* `hidden` 的参数不出行：它们是某张卡的**槽位**或「当前用哪套」，
           界面上的样子是那张卡标题栏里的一条下拉（见 GcodeBlock） */
        fields: u.defsOfSection(sec.id).filter((d) => d.meta?.hidden !== true),
      }))
    }

    const q = query.trim().toLowerCase()
    /*
     * 匹配面（补了两个，作者：「可是我搜索圆盘搜不出」）：
     *   参数名 / 字段名 / TOML 键名  ← 原来就有的三样
     *   **选项名**（choices 的 label） ← 「圆盘」是「擦料方式」的一个选项，不是任何参数名
     *   **分组名**（section 的 label） ← 「圆盘动作控制」是一整张卡的名字
     * 只按前两样搜，「圆盘」在这份注册表里一个字都匹配不到 —— 而它明明写在两个地方。
     * 代价说在明处：搜「开启 / 关闭」这类通用选项词会把带这个选项的都列出来。
     */
    const isMatch = (d: ParamDef) =>
      d.label.toLowerCase().includes(q) ||
      d.key.toLowerCase().includes(q) ||
      (d.meta?.tomlKey ?? '').toLowerCase().includes(q) ||
      (d.field.choices ?? []).some((c) => c.label.toLowerCase().includes(q))

    const cards: ShownCard[] = []
    for (const tab of u.tabs) {
      for (const sec of tab.sections) {
        /* 同非搜索态：槽位 / 选择器不参与搜索，也不成行 */
        const defs = u.defsOfSection(sec.id).filter((d) => d.meta?.hidden !== true)
        const hit = new Set(defs.filter(isMatch).map((d) => d.key))
        /*
         * 分组名命中 = 这一整张卡都算命中（他搜的是这张卡的名字）。
         * **不匹配分类名**（涂胶 / 擦料）：那等于「搜分类 = 整类都出来」，
         * 反而把这个过滤变成摆设 —— 分类本来就在分类条上、也印在每张卡的副标题里。
         */
        const wholeCard = sec.label.toLowerCase().includes(q)
        if (hit.size === 0 && !wholeCard) continue
        /* 上下都带，但**只认这张卡里**的路（见上面那段） */
        const carried = new Set([...descendantsOf(defs, hit), ...ancestorsOf(defs, hit)])
        const fields = wholeCard
          ? defs
          : defs.filter((d) => hit.has(d.key) || carried.has(d.key))
        cards.push({
          id: `${tab.id}-${sec.id}`,
          title: sec.label,
          subtitle: tab.label,
          fields,
        })
      }
    }
    return cards
  }, [currentTab, query, searching, u])

  const dualColumn = bodyWidth >= DUAL_MIN_WIDTH && shown.length > 1

  /**
   * 搜索框放哪一行：够宽时待在分类条那一行的右端，不够时搬到头部标题行。
   * `bodyWidth` 量的是卡片区（`.bodyWrap`，见 useWidth），阈值与实测边界见 BAR_NEED。
   *
   * **mini 档不再看这个阈值**：那一档搜索收成一颗放大镜钉在分类条那一行
   * （`searchCollapsed`），点了才铺开 —— 于是头部不外借任何东西，
   * `[data-row='head']` 那一套在 mini 档不再出现。
   */
  const searchInHead = density !== 'mini' && bodyWidth < BAR_NEED[density]

  /**
   * mini 档搜索**是否收起**（作者：「这个搜索框还是不要这样子，它多占了位置」）——
   * 这一档它常驻就是整整一行（上面还带着头部内衬），收起之后只剩一颗放大镜图标
   * （28px），点开才在分类条那一行铺开输入框。
   */
  const searchCollapsed = density === 'mini' && !searchOpen

  /*
   * mini 档点开那颗放大镜之后，焦点直接进输入框 —— 展开与聚焦是同一个动作的两半，
   * 还要再点一下才输字就烦人了（`SearchField` 认 `inputRef`）。
   * 它必须待在**所有 early return 之前**（hooks 的规矩），所以放在这一层。
   */
  useEffect(() => {
    if (!wantSearchFocus.current) return
    wantSearchFocus.current = false
    if (searchCollapsed) return
    searchRef.current?.focus()
  }, [searchCollapsed])

  /** 保存确认弹层要显示的改动列表。from = 已保存值，to = 草稿值 */
  const changedEntries = useMemo(() => {
    const entries: { key: string; label: string; from: string; to: string; bad: string | null }[] = []
    for (const def of u.defs) {
      if (!u.dirtyOf(def.key)) continue
      entries.push({
        key: def.key,
        label: def.label,
        from: valueText(def, u.savedValueOf(def.key)),
        to: valueText(def, u.valueOf(def.key)),
        /* 即时校验的同一份判据（见 invalidNumberText）：超范围的项不许保存 */
        bad: invalidNumberText(def, u.valueOf(def.key)),
      })
    }
    return entries
  }, [u])
  /* 超出取值范围的改动 —— 弹层里列出来、把「确认保存」灰掉（作者 2026-10-20） */
  const invalidEntries = changedEntries.filter((e) => e.bad !== null)

  const confirmSave = () => {
    u.save()
    setAskSave(false)
  }

  const [leftCol, rightCol] = useMemo(
    () => (dualColumn ? splitDualColumns(shown) : [shown, [] as ShownCard[]]),
    [dualColumn, shown],
  )

  /* 文件名从说明书里来：包里每个版本带着自己的文件清单（A1/STANDARD → A1.toml）；
     这个版本没配 MKP 文件（如 A2L）才退成「机型 · 版本」 */
  const fileLabel = u.fileLabel ?? `${u.machineId} · ${u.versionId}`
  /*
   * pill 说的是「**正在用哪份**」：底账命中当前 combo → 底账那份（作者 2026-10-06：
   * 应用的是云端件，pill 却说「（已修改）」，那是把编辑目标当成了正在用）；
   * 没命中（正在编辑另一份）→ 退回编辑目标。确认框 / 历史标题仍用 fileLabel
   * —— 它们回答的是「写回哪份 / 改的哪份」，与 pill 是两个问题。
   */
  const pillLabel =
    u.activeUse?.onIt === true && u.activeFileName !== null ? u.activeFileName : fileLabel

  /*
   * 看的那份**不是**已应用的那份 —— pill 不画绿、换琥珀（作者：「我切换到不是正在用的时候，
   * 就不要用绿色，用琥珀色」）。只改这一颗 pill 的颜色，不再另给"切回去"的按钮：
   * 作者 2026-10-20 裁定「没有这个回到已应用的这个功能了」—— 现在都从预设行的右键
   * 进「修改参数」，切哪一份也由那条右键决定，标题栏再摆一颗只会跟它抢动作。
   *
   * 底账空（activeUse === null）时不比较：没有已应用的就没有可对比的对象（见 useParams）。
   */
  const inactive = u.activeUse !== null && !u.activeUse.onIt

  /*
   * 底栏那句状态文案。**落点跟着编辑目标说**（A1 统一口径）：编辑的是云端底稿，
   * 保存 = 另存成你自己的一份（`presets-local/…（已修改）.toml`，原件不动）——
   * 原来那句"保存后会覆盖 A1.toml"与实际落盘相反，是测试报告里"用户不敢赌"的根源；
   * 编辑的是本地文件，保存 = 写回它自己。mini 档只留前半句 —— 实测整句占 153px，
   * 加上左边三个按钮正好把底栏顶成两行（作者：「下面改不改都多一行」）。
   * 完整的落点说明在保存确认弹层里，这里挂了 title，悬停看全。
   */
  const savingToLocal = u.editingPreset?.origin === 'local'
  /** 云端更新里**还没表态**的项数（0 = 都选好了，只差保存） */
  const syncUndecided = Math.max(0, u.pendingCount - u.syncDirty)
  /*
   * 底栏那句状态：值改动与**云端更新的表态**分开数（2026-10-20）。
   * 只点了「用新值 / 保持原样」而没手改任何值时，`dirtyCount` 是 0 —— 但那一笔
   * 同样没保存，底栏必须说出来，否则「保存」亮着而左边写着"没有未保存的改动"。
   */
  const stateText =
    u.dirtyCount === 0 && u.syncDirty === 0
      ? density === 'mini'
        ? '没有改动'
        : '没有未保存的改动'
      : u.dirtyCount === 0
        ? density === 'mini'
          ? `${u.syncDirty} 项更新待保存`
          : `${u.syncDirty} 项云端更新待保存 · 保存才生效`
        : density === 'mini'
          ? `${u.dirtyCount} 处未保存`
          : savingToLocal
            ? `${u.dirtyCount} 处未保存 · 保存会写回 ${fileLabel}`
            : `${u.dirtyCount} 处未保存 · 保存会另存成你自己的一份`

  const closeHistory = () => setHistoryOpen(false)

  /**
   * **「历史 → 还原」点完的那一句回话**（2026-10-17）。
   *
   * 作者：「怎么我点了还原没反应…是真的没还原还是没有交互效果」。
   * 查下来按钮是好的：还原成功会写出草稿、底栏变「N 处未保存」、抽屉顶上多一条
   * 「未保存」；而**命中 noop 时（目标值就是现值）界面一个像素都不动** —— 那正是
   * 他看到的那一下。而抽屉正好盖住右边那列数值，连成功也不容易看见。
   *
   * 所以这里按 `apply` 的三种结果各回一句话（几秒后自己消失）：
   *   applied  绿：`已还原：X 轴偏移 → -0.45 mm`
   *   blocked  灰：被上游标记弃用，改不了
   *   noop     灰：现在就是那个值，不用还原
   */
  const [revertNote, setRevertNote] = useState<{ text: string; tone: 'ok' | 'muted' } | null>(null)
  const revertNoteTimer = useRef<number | null>(null)
  useEffect(
    () => () => {
      if (revertNoteTimer.current !== null) window.clearTimeout(revertNoteTimer.current)
    },
    [],
  )
  const onRevertHistory = useCallback(
    (entry: HistoryEntry) => {
      const result = u.revertEntry(entry)
      const first = entry.items[0]
      const many = entry.items.length > 1
      const target = first === undefined ? '' : valueText(u.defOf(first.key), first.from)
      const label = first?.label ?? '这一项'

      const next =
        result === 'applied'
          ? {
              text: many ? `已还原 ${entry.items.length} 项（含 ${label} → ${target}）` : `已还原：${label} → ${target}`,
              tone: 'ok' as const,
            }
          : result === 'blocked'
            ? { text: `没有还原：${label} 已被上游标记弃用，改不了`, tone: 'muted' as const }
            : {
                text: many ? '没有变化：这几项现在就是当时那个值' : `没有变化：${label} 现在就是 ${target}`,
                tone: 'muted' as const,
              }

      setRevertNote(next)
      if (revertNoteTimer.current !== null) window.clearTimeout(revertNoteTimer.current)
      revertNoteTimer.current = window.setTimeout(() => setRevertNote(null), 3200)
    },
    [u],
  )

  /**
   * 拖动擦料塔时的临时位置（`null` = 没在拖）。
   *
   * 它**只喂显示**：右侧 X/Y 两格在拖动期间显示这个数，于是"塔在走、数字也在走"；
   * 松手后 `TowerMap` 提交一次（一条撤销），这个 state 立刻清掉、交回草稿值。
   * 草稿 / 撤销栈 / 历史清单全程不碰这里 —— 作者要的是"不记中间值"。
   */
  const [towerDrag, setTowerDrag] = useState<{ x: number; y: number } | null>(null)
  /* 松手时 TowerMap 回调一次 `null` ⇒ 这里清空，两格立刻交回草稿值 */
  const onTowerDrag = useCallback((pos: { x: number; y: number } | null) => setTowerDrag(pos), [])

  /** 擦料塔地图的供数（见 towerSlotOf）：坐标字段不在 / 没配尺寸时是 undefined，坐标行照旧 */
  const tower = towerSlotOf(u, ro, onTowerDrag)
  /**
   * 拖动期间那一卡里 X/Y 两行该显示什么：**临时值优先**。
   *
   * `ParamCard` 只认 `fields` 里的 key → 值；这里不改字段，只在渲染前把这两条的
   * `def.key` 换成同一份"假 def + 临时值"代价太大，所以走 `ParamCard` 新增的
   * `overrides` 通道（值 → 显示用的原文）。
   */
  const towerDragTexts =
    towerDrag === null || tower === undefined
      ? undefined
      : { [tower.xKey]: String(towerDrag.x), [tower.yKey]: String(towerDrag.y) }

  const renderCards = (cards: ShownCard[]) =>
    cards.map(({ id, title, subtitle, fields }) => (
      <ParamCard
        key={id}
        title={title}
        subtitle={subtitle}
        fields={fields}
        mode={searching ? 'search' : 'normal'}
        /* 高亮只看词：不搜索时是空串，`Highlight` 原样返回 */
        query={query}
        valueOf={u.valueOf}
        savedValueOf={u.savedValueOf}
        prevSavedOf={u.prevSavedOf}
        factoryOf={u.factoryOf}
        dirtyOf={u.dirtyOf}
        blockedByOf={u.blockedBy}
        /* 分支摊开 / 折叠的判据（A43 移植） */
        condOn={u.condOn}
        /* 擦料塔地图槽位：坐标两字段都在这张卡时才生效（见 ParamCard） */
        tower={tower}
        /* 拖动擦料塔时 X/Y 两格显示的临时值（只喂显示，草稿不动 —— 见 Props.valueOverrides） */
        valueOverrides={towerDragTexts}
        expandedKey={expandedDetailKey}
        onToggleExpand={(key) => setExpandedDetailKey((prev) => (prev === key ? null : key))}
        onEdit={u.edit}
        onRevertToSaved={u.revertToSaved}
        /* 只读态（编辑模式之外）：控件降成纯文本值、分支卡头点不动 */
        readonly={ro}
        /*
         * 云端更新那一段（2026-10-09）。`sync` 为 null（编辑对象不是「我的预设」/
         * 认不出是哪一版云端）时不传 —— 行上一律不画，与以前长得一模一样。
         * 只读态两个动作也不传（ParamRow：「不传就画不出那两颗按钮」）——
         * 「云端已更新 · 云端 x → y」那两行说明照常显示。
         */
        syncOf={u.sync === null ? undefined : u.syncOf}
        /* 三态由草稿层那张表态表细分（待选择 / 已采用 / 已保持）—— 只读也能看，只是不给按钮 */
        syncDraftOf={u.syncDraftOf}
        onAdoptSync={ro ? undefined : u.adoptSync}
        onHoldSync={ro ? undefined : u.holdSync}
        onUseMyValue={ro ? undefined : u.useMyValue}
        syncBusy={u.syncBusy}
        renderBlock={(def) => {
          /*
           * 多套方案（2026-10-21）：卡片还是那一张，读写落在**当前选中那套**上。
           * 没有槽位（或本机没配）时 `group` 为 null，走的完全是老样子。
           */
          const group = u.slotsOf(def.key)
          const slotKey = group?.activeKey ?? def.key
          return (
            <GcodeBlock
              def={def}
              editKey={slotKey}
              slots={group?.options}
              activePosition={group?.options.find((o) => o.key === slotKey)?.position}
              onSelectSlot={(position) => {
                if (group === null) return
                u.edit(group.selectorKey, position)
              }}
              value={u.valueOf(slotKey)}
              dirty={u.dirtyOf(slotKey)}
              disabled={u.blockedBy(slotKey) !== null}
              savedValue={u.savedValueOf(slotKey)}
              onEdit={u.edit}
              onRevertToSaved={u.revertToSaved}
            />
          )
        }}
      />
    ))

  /*
   * 键盘入口（撤销 / 重做 / 保存 / 搜索 / Esc）。
   *
   *   Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y   撤销 / 重做
   *   Ctrl+S                          保存修改 —— 与点那颗按钮**同一条路**：先开确认弹层
   *   Ctrl+F                          焦点进搜索框（搜什么、跨全部分类，都是搜索框自己的事）
   *   Esc                             关掉当前那一层弹层
   *
   * 早先撤销 / 重做两颗按钮换成图标时留过一句「或者省略，只用快捷键」，但快捷键
   * 当时并不存在（整份稿查过：没有一个 `keydown` 认 Ctrl+Z）—— 于是「省略」这条路走不通，
   * 只能留着图标。现在补上。
   *
   * **装在这一页，不装外壳。** 只有参数台有撤销栈（`useParams` 那一份是页面级的：
   * 首页 / 预设页 / 校准页没有可撤的东西），装在外壳上就成了「在别的页按了没反应」。
   * 工作台那一套装在外壳，是因为它的撤销栈本来就是全局一份（store）。
   *
   * 几条细节：
   *   · 焦点在输入框里也认，并且 `preventDefault`：参数台的账比浏览器给 `<input>` 的
   *     原生文本撤销更贴题 —— 改的是参数，退就该退回参数。
   *     （实测过这一条：改完 5 之后焦点还在那个输入框里按 Ctrl+Z，值回到 -1 而不是
   *     「只把框里的字退回去」。）
   *   · `e.altKey` 时不认：`Ctrl+Alt+Z` 在部分键盘布局上是 AltGr+Z（欧洲键盘打 @ 那种）。
   *   · **弹层（保存确认 / 恢复默认值 / 切机型确认）开着时，认得的这几个键一律吃掉**
   *     （`preventDefault` + 什么都不做）。不能只是「不办本地事」：那样按键会落到浏览器
   *     手里，焦点要是在某个输入框上，它会去改那个框里的字 ——
   *     弹层背后悄悄变一个值，比什么都不发生更糟。关掉弹层再动手，账才干净。
   *   · `Ctrl+S` 不直落保存：那是鼠标走的路（先看清楚要写哪几处，再按「确认保存」），
   *     键盘不该有另一条更快的路。没有未保存改动时它什么都不做（那颗按钮此刻也是灰的）。
   *   · Esc 用**捕获阶段**：抽屉与预设选择器的 Esc 挂在 `document` 上（冒泡），
   *     弹层开着时这个键得先在这里被拦下来，否则弹层与抽屉会一起关。
   *
   * 依赖写解构出来的具体值（不写整个 `u`）：`undo` / `redo` 的 `useCallback` 依赖里有
   * draft / past / future / log，每改一次身份就换一次，于是每改一次重挂一遍监听
   * （一次 remove + 一次 add，可以忽略）—— 好处是**不会拿到旧的闭包**，
   * 撤销栈读的正是这两个回调里的那份快照。写成员表达式 `u.undo` 则会让 eslint 的
   * react-hooks/exhaustive-deps 认不出（它报「缺 u」），写成 `[u]` 又变成每帧重挂。
   *
   * 没有加载态守卫：数据包还没读到 `past` 是空的，`undo()` 自己会 `return`（同一条规则
   * 管着重做），加一层 `if (loading) return` 只会让「守卫用的还是挂载时那个值」这种
   * 陈旧判断混进来。
   */
  const { undo, redo, pendingCombo, cancelPending, dirtyCount, syncDirty, save } = u
  useEffect(() => {
    const sheetOpen = askSave || askRestore || pendingCombo !== null
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (!sheetOpen) return /* 没弹层就让下去：抽屉 / 预设选择器自己会关 */
        e.preventDefault()
        e.stopPropagation()
        if (askSave) setAskSave(false)
        else if (askRestore) setAskRestore(false)
        else cancelPending()
        return
      }
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return
      const k = e.key.toLowerCase()
      if (k !== 's' && k !== 'f' && k !== 'z' && k !== 'y') return
      /* 认得出是这一页的键，就先吃掉 —— 弹层开着也一样（理由见上面注释） */
      e.preventDefault()
      if (sheetOpen) return
      if (k === 'f') {
        /* 搜索是"看"的一部分，只读态照认 */
        searchRef.current?.focus()
        return
      }
      /* 只读态（编辑模式之外）：s / z / y 到此为止 —— 没有可保存、可撤销的东西 */
      if (ro) return
      if (k === 's') {
        /* 只表了态、没改值 ⇒ 没有可确认的值改动，直接存（弹层列的是"改了什么值"） */
        if (dirtyCount > 0) setAskSave(true)
        else if (syncDirty > 0) save()
        return
      }
      if (k === 'y' || e.shiftKey) redo()
      else undo()
    }
    /* 捕获阶段：Esc 要抢在 `document` 上那些抽屉监听之前（见注释） */
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [undo, redo, pendingCombo, cancelPending, dirtyCount, syncDirty, save, askSave, askRestore, ro])

  if (u.loading) {
    return (
      <div className={s.page} data-density={density}>
        <p className={s.loading}>正在打开数据包…</p>
      </div>
    )
  }

  /*
   * 读不到目录（catalog）时的样子。
   *
   * 这一页画什么（页签、分组、每条的控件与类型、什么条件下显示）全部来自目录 ——
   * 读不到它就**没有页面可言**，所以这里不是「加载失败」那种红字（那是「本该有却没有」），
   * 而是一个空态：一句人话 + 后端给的原因。
   * 原来这里还有一颗「去「同步」页获取一份」—— 那一页 2026-10-02 退役（数据源配置
   * 降级成设置页里的开发后门），这条跳转没有去处，删掉。那句「加载失败：」是照更早
   * 那一版的壳抄的，而那一版走 `api.*`（客户端内置，不会没有），语义根本不一样。
   */
  if (u.error !== null) {
    return (
      <div className={s.page} data-density={density}>
        <div className={s.emptyBox}>
          <h2 className={s.emptyTitle}>读不到参数目录</h2>
          <p className={s.emptyText}>
            参数页画什么（页签、分组、每条的控件与类型、什么条件下显示）全部来自随安装包
            走的那份目录（catalog）—— 读不到它时，这一页没有东西可画，也不编一份假的。
          </p>
          <p className={s.emptyText}>{u.error}</p>
        </div>
      </div>
    )
  }

  /*
   * 「修改历史」那颗按钮：它进**标题栏动作槽**（`×` 左边，见下面的 `headActions`）——
   * 嵌在模态框里时 portal 到那颗槽、单独当页用时落在头部那一行的右端。
   *
   * **它是开关**（作者对着开着抽屉的截图：「这个时候点击了修改历史应该可以
   * 折叠起来」）。原来是 `setHistoryOpen(true)` —— 抽屉已经开着时点它什么都不发生，
   * 而那颗按钮（`aria-expanded=true` 时还带绿框）看起来明明是个「开着」的状态，
   * 点下去没反应就是骗人。现在点一下就收，`aria-expanded` 同时是样式钩子。
   *
   * 悬浮态的抽屉会盖住这颗按钮（抽屉从右缘铺过来），那一路本来就不靠它收 ——
   * 点抽屉外面、抽屉自己的 ×、Esc 三条都还在；这里修的是**固定在右侧**那一路：
   * 抽屉占住右侧一条时，这颗按钮就在内容区的右上角，点得到也看得见。
   */
  const historyButton = (
    <button
      type="button"
      className={s.historyBtn}
      aria-haspopup="dialog"
      aria-expanded={historyOpen}
      title={historyOpen ? '收起修改历史' : '修改历史'}
      onClick={() => setHistoryOpen((v) => !v)}
    >
      修改历史
      {u.historyItemCount > 0 && <span className={s.detailCount}>{u.historyItemCount}</span>}
    </button>
  )

  /*
   * ★ **离开就折回去**（作者：「我希望它自己折回去，就是我离开了之后，它又继续
   *   自己变小，不要一直留在这个地方」）—— mini 档那颗放大镜展开出来的输入框一失焦
   *   （或按回车）就收回成图标：它是**临时借来的位置**，不该常驻。
   *   `onCommit` 正是「离开框 / 回车」那一下（见 SearchField 的文件头）。
   *   搜过的字照旧留着（`query` 不变）—— 收回的是位置，不是筛选。
   */
  const onSearchCommit = (term: string) => {
    rememberSearch(term)
    if (density === 'mini') setSearchOpen(false)
  }

  /*
   * 搜索框也是两处共用（分类条那一行右端 / 不够宽时搬到标题行，见 searchInHead）——
   * 加上「最近搜索」那几个 prop 之后，再各写一份就该分叉了。
   */
  const searchField = (
    <SearchField
      value={query}
      density={density}
      inputRef={searchRef}
      history={searchHistory}
      onChange={setQuery}
      onCommit={onSearchCommit}
      onPick={(term) => {
        setQuery(term)
        rememberSearch(term)
      }}
      onForget={forgetSearch}
    />
  )

  /*
   * ★ **标题栏动作槽里那一簇**（照 a05）：这里只剩「修改历史」一颗。
   *
   * 原来与它并排的「回到已应用」**已退役**（作者 2026-10-20：「没有这个回到已应用
   * 的这个功能了」）：那一份现在是从预设行右键「修改参数」进来的，编辑对象由那一下
   * 定死，标题栏再多一颗"跳到已应用那份"只会跟右键抢动作。
   *
   * 撤销 / 重做 / 保存那些属于**这一页的编辑框架**（MKP 那一套：只读 → 进编辑模式 →
   * 撤销 / 重做 / 放弃并退出 / 保存，见 `editMode` 与底栏），不在这一簇里 ——
   * 「修改历史」是参数这一页自己的出口，才在标题栏。
   */
  const headActions = historyButton

  /*
   * 有动作槽（住在模态框里 / `embedded`）→ portal 进去，与 `×` 同一排；
   * 没有槽（单独当页用）→ 原样摆在头部那一行右端（`headActionsInline` 的 `margin-left: auto`）。
   */
  const headActionsNode =
    headSlot === null ? (
      <span className={s.headActionsInline}>{headActions}</span>
    ) : (
      createPortal(headActions, headSlot)
    )

  /*
   * **标题栏那一句状态 —— 成功 / 警告 / 失败共用一格**（2026-10-17 两改）。
   *
   * 第一改（上午）：保存回执从内容区顶上搬进模态框框架 —— 它原来每出现一次就把
   * 参数列表往下顶一行。第二改（晚上，作者看截图）：搬过去之后**回执太长会换行**、
   * 还压到右侧动作簇；而「弃用不能改」那种警告还留在内容区里没跟过来。
   * 原话：「还有一些失败、警告的没改」「不管是成功的还是失败的，我觉得都应该复用
   * 同样的位置和显示、和提示」「要不然利用起来标题旁边的空间」。
   *
   * 于是三条并成一条，位置 = **标题右边那一格**（`Modal` 的 `.titleRow`）：
   *
   *   优先级  `draftError`（草稿没落盘，最要紧）> `gateNote`（动手被闸拦住）> `savedNote`
   *   语气    `bad` 红 / `warn` 琥珀 / `ok` 绿 —— 位置与长相一样，只换颜色
   *   长相    **永远一行**，长就省略号（`.note` 的 nowrap + ellipsis），`title` 里给全文
   *
   * 单独当页用（没有槽）时同一份落回内容区顶上 —— 一套代码两种住处。
   */
  const noteBad =
    u.savedNote !== null &&
    (u.savedNote.startsWith('保存失败') ||
      u.savedNote.startsWith('这份没有可保存') ||
      /* 「应用 0 项」那一颗没落上（2026-10-21）—— 同一格、同一套红字 */
      u.savedNote.startsWith('收下云端这一版没成'))
  const headNote = ((): { text: string; tone: 'ok' | 'warn' | 'bad' } | null => {
    if (u.draftError !== null) return { text: `草稿没写进磁盘：${u.draftError}`, tone: 'bad' }
    if (u.gateNote !== null) return { text: u.gateNote, tone: 'warn' }
    if (u.savedNote === null) return null
    return { text: u.savedNote, tone: noteBad ? 'bad' : 'ok' }
  })()
  const headNoteNode =
    headNote === null ? null : (
      <span className={s.note} data-tone={headNote.tone} title={headNote.text}>
        {headNote.text}
      </span>
    )

  return (
    <div ref={mountSyncLayer} className={s.page} data-density={density}>
      {/* 那一句状态 → 模态框标题栏（标题右边）；没有槽时它渲染在内容区顶上（见下面那段） */}
      {headNoteSlot !== null && headNoteNode !== null
        ? createPortal(headNoteNode, headNoteSlot)
        : null}
      <FieldLayer>
        <div className={s.main}>
          <div className={s.left}>
            <header className={s.head}>
              <div className={s.headTop}>
                {/* 嵌在模态框里时不画页面大标题 —— 外面那个框的标题就是这一份的名字 */}
                {!embedded && <h2 className={s.pageTitle}>修改参数</h2>}

                {/*
                 * 预设选择器：一个文件名 pill + 「切换」小字，点开抽屉。
                 * 更早那一版用的是两个共用件下拉（机型 / 版本），作者的原话是
                 * 「那个下拉菜单，我不喜欢，我喜欢改成抽屉的」。换成 pill 之后顶部只剩
                 * 一件东西，标题那一行也清爽了。
                 *
                 * 换 combo 仍走 `u.requestCombo` —— 未保存改动的确认弹层在那一层，
                 * 抽屉自己不判断脏数据。
                 *
                 * **嵌模态框时它整个不画**：那时编辑对象已经定死（用户右键的就是那一份），
                 * 摆一个点了没反应的 pill 只会让人以为能换。
                 */}
                {!embedded && (
                  <button
                    type="button"
                    className={inactive ? `${s.presetPill} ${s.presetPillOff}` : s.presetPill}
                    ref={pillRef}
                    title={
                      inactive
                        ? `当前预设 ${pillLabel}（未应用）· 点击切换`
                        : `当前预设 ${pillLabel} · 点击切换`
                    }
                    onClick={() => setPickerOpen((v) => !v)}
                  >
                    <span className={s.presetName}>{pillLabel}</span>
                    <span className={s.presetSwitch}>切换</span>
                  </button>
                )}

                {/*
                 * 窄窗时搜索框搬到这一行（见 searchInHead）—— 挤在文件名
                 * pill 右边、宽度跟着剩下的空间收，让分类条那一行只剩分类条。
                 */}
                {searchInHead && (
                  <div className={s.searchSlot} data-row="head">
                    {searchField}
                  </div>
                )}

                {/*
                 * 右上角那一簇（照 a05）：现在只有**修改历史** —— 见 `headActions`。
                 * 在模态框里它 portal 到标题栏那颗动作槽（`×` 左边，`margin-left:auto`
                 * 顶到最右）；单独当页用时落在这一行右端。
                 *
                 * 原来这一簇还有一颗「回到已应用」，2026-10-20 整颗退役（作者：
                 * 「没有这个回到已应用的这个功能了」）—— 进哪一份由预设行右键决定。
                 */}
                {headActionsNode}
              </div>
            </header>

            <div className={s.bar}>
              <CategoryPills
                tabs={u.tabs}
                current={categoryId}
                dirty={u.dirtyTabs}
                /*
                 * 点分类**不清搜索**（作者：「点击了 tab 之后，搜索就被清空了，
                 * 那我还需要搜索的记录」）。原来这里写的是 `setQuery(''); setCategoryId(id)`
                 * —— 搜到一半换个分类看，刚打的字就没了。
                 *
                 * 搜索在着的这一段时间里，结果仍然是**跨分类**的（只搜当前
                 * 页签会漏掉「东西其实在另一类里」的那些）。所以这时点分类只换选中的那一枚
                 * 与条外那截分类名，列表不动；把搜索清掉，落到的就是他刚点的那一类。
                 */
                onSelect={setCategoryId}
              />
              {/*
               * 搜索框在这一行的右端；不够宽时它搬到上面那行（见 searchInHead）。
               *
               * ★ mini 档**收成一颗放大镜**（作者：「这个搜索框还是不要这样子，
               *   它多占了位置」）—— 那一档它常驻就是整整一行；收起时只占 28px，
               *   点开才在分类条这一行的右端铺成输入框（`:focus` 由上面那个 effect 给）。
               */}
              {!searchInHead &&
                (searchCollapsed ? (
                  <button
                    type="button"
                    className={s.searchIcon}
                    /* 收起来的时候筛选**还在生效**（`query` 没清）—— 这件事要说出来，
                       否则"怎么只剩三条"会没头没脑 */
                    data-on={query !== ''}
                    aria-label="搜索参数"
                    aria-expanded={false}
                    onClick={() => {
                      wantSearchFocus.current = true
                      setSearchOpen(true)
                    }}
                  >
                    <Icon name="search" size={15} />
                  </button>
                ) : (
                  <div className={s.searchSlot} data-row="bar">
                    {searchField}
                  </div>
                ))}
            </div>

            {/*
              云端更新（2026-10-09）：云端改过的项**不直接顶掉本地值**，
              它们先在这里等着 —— 一条，动作只有一个去处（那一个弹层）。

              ★ **两档**（2026-10-20）：
                · 取回了云端字节 ⇒ 「待选择 N 项」+ 逐项差异（点它直接进那一屏）；
                · 还没取回 ⇒ 只说"云端发了新版"这个**事实**，差异要用户点了才去取。
                  不自动取的理由：自动取会让"没点更新，本地怎么知道参数变了"成立，
                  而取回之后的「更新」会把我没改过的项悄悄改成云端新值。

              ★ 又说清**选了几项**（同日）：表态只进草稿，不点保存一个字节都不写 ——
                横幅必须让人知道"选完了还要保存"，否则他会以为已经生效了。
            */}
            {u.pendingCount > 0 && (
              <div className={s.syncBar} role="status">
                <span className={s.syncBarText}>
                  {syncUndecided === 0 ? (
                    <>
                      云端更新已选 <b>{u.syncDirty}</b> 项 —— 点「保存」才写进这份文件
                      {u.sync?.currentReleaseTime != null
                        ? `（云端 ${u.sync.currentReleaseTime}）`
                        : ''}
                    </>
                  ) : (
                    <>
                      云端更新待选择 <b>{syncUndecided}</b> 项
                      {u.sync?.currentReleaseTime != null
                        ? `（云端 ${u.sync.currentReleaseTime}）`
                        : ''}
                      {u.syncDirty > 0 && (
                        <span className={s.syncBarHint}>
                          （另有 {u.syncDirty} 项已选好、等保存）
                        </span>
                      )}
                    </>
                  )}
                  {syncUndecided > 0 && (
                    <span className={s.syncBarHint}>
                      —— 你改过的值不会被顶掉，勾选采用 / 不勾保持
                    </span>
                  )}
                </span>
                <button
                  type="button"
                  className={s.syncBarBtn}
                  /*
                   * 只读态点它 = 顺手进编辑模式再开面板 —— "处理更新"本来就是写操作，
                   * 用不着让用户先去找底栏那颗「进入编辑模式」。
                   */
                  onClick={() => {
                    if (ro) setEditMode(true)
                    setSyncOpen(true)
                  }}
                >
                  选择要更新的参数
                </button>
              </div>
            )}
            {/*
              「云端发了新版，但一项要改值的都没有 / 正文还没取回来」—— 一条**陈述**，不是错误。

              ★ 两档（2026-10-21 修）：
                · `syncNeedsFetch`：字节还没取回来，逐项差异要比了才知道 —— 点右边去取；
                · 取回来发现 **0 项要改值**（云端只改了注释 / 新字段）⇒ 那时
                  `syncNeedsFetch` 已经是 false，原来这一条**整块消失**，用户再没有入口
                  把这一版收掉（作者实测：「有新版」一直挂着）。所以这一档留着一条
                  「收下这一版」—— 与「修改参数」里那一屏的 0 项按钮同一件事。
            */}
            {u.pendingCount === 0 &&
              u.sync !== null &&
              u.sync.versionAdvanced &&
              u.sync.cloudFileName !== null && (
                <div className={s.syncBar} role="status">
                  {u.syncNeedsFetch ? (
                    <>
                      <span className={s.syncBarText}>
                        云端发了新版
                        {u.sync.basedOnReleaseTime != null
                          ? `（你现在这一份基于 ${u.sync.basedOnReleaseTime}）`
                          : ''}
                        <span className={s.syncBarHint}>—— 点右边先看看它改了哪几项</span>
                      </span>
                      <button
                        type="button"
                        className={s.syncBarBtn}
                        disabled={u.syncFetching}
                        onClick={() => {
                          if (ro) setEditMode(true)
                          /* 这一下才是"拿云端新版字节"那个动作：取回来再开那一屏 */
                          void u.fetchSync().then(() => setSyncOpen(true))
                        }}
                      >
                        {u.syncFetching ? '正在取回…' : '选择要更新的参数'}
                      </button>
                    </>
                  ) : (
                    <>
                      <span className={s.syncBarText}>
                        云端发了新版
                        {u.sync.currentReleaseTime != null
                          ? `（${u.sync.currentReleaseTime}）`
                          : ''}
                        <span className={s.syncBarHint}>
                          —— 这一版没有要改值的参数，只是注释 / 新字段变了
                        </span>
                      </span>
                      <button
                        type="button"
                        className={s.syncBarBtn}
                        disabled={u.syncBusy}
                        onClick={() => {
                          if (ro) setEditMode(true)
                          /* 值一个不动，注释 / 新字段 / 时间戳与血统推到当前版：「有新版」收场 */
                          void u.digestSyncRelease()
                        }}
                      >
                        {u.syncBusy ? '正在收下…' : '收下这一版'}
                      </button>
                    </>
                  )}
                </div>
              )}
            {u.syncError !== null && <p className={s.warn}>云端更新这一条读不出来：{u.syncError}</p>}

            {/*
              那一句状态：嵌入模态框时它住在**标题右边**（见 `headNoteNode` 那段）；
              单独当页用（没有槽）时才落回这里 —— 同一份节点、同一套语气。
            */}
            {headNoteSlot === null && headNote !== null && (
              <p className={s.noteLine} data-tone={headNote.tone} title={headNote.text}>
                {headNote.text}
              </p>
            )}
            {u.draftError === null && u.editingPreset === null && u.fileLabel === null && (
              <p className={s.warn}>
                这个版本没配 MKP 预设文件 —— 改动不会保存在任何文件里（只在这一屏）
              </p>
            )}

            {/*
              测量层与分列容器**分开的两层**：
                .bodyWrap  永远存在，只量宽度（回调 ref 接在这里）
                .scroll / .dual  会随 dualColumn 整个换掉

              两层都必要，各治一个毛病：
                · 分开 → 分列切换时被换掉的是里面那个，测量层不受影响
                · 回调 ref（见 useWidth 的注释）→ 元素在 loading 之后才出现也能接上，
                  这才是「双列一直没开」的真根因（RO 压根没被创建过）
            */}
            <div className={s.bodyWrap} ref={attachBody}>
              <div className={dualColumn ? s.dual : s.scroll}>
                {dualColumn ? (
                  <>
                    <div className={s.col}>{renderCards(leftCol)}</div>
                    <div className={s.col}>{renderCards(rightCol)}</div>
                  </>
                ) : (
                  <>
                    {renderCards(shown)}
                    {searching && shown.length === 0 && (
                      <p className={s.empty}>没有匹配的参数，换个词试试</p>
                    )}
                  </>
                )}
              </div>
            </div>
          </div>

          {pinned && (
            <HistoryDrawer
              open
              file={fileLabel}
              mode="pinned"
              canPin={canPin}
              view={historyView}
              history={u.history}
              groups={u.historyByBatch}
              onView={setHistoryView}
              onMode={setHistoryMode}
              onClose={closeHistory}
              onRevert={onRevertHistory}
              /* 只读态抽屉里那颗「进入编辑模式」= 底栏同一颗（同一个 setEditMode） */
              onEnterEdit={() => setEditMode(true)}
              onClearHistory={u.clearHistory}
              note={revertNote}
              /* 只读态：历史照看，「还原」那颗不画（写入口） */
              readonly={ro}
            />
          )}
        </div>

        {/*
         * 底栏（2026-10-16 两态，与参数对比台 / 编辑文件模态框同一套框架）：
         *
         *   只读态   左状态 · 右下「进入编辑模式」
         *   编辑态   左「恢复默认值 + 状态」· 右下 撤销 / 重做 / 放弃并退出 / 保存
         *
         * ★ 撤销 / 重做从**左下**搬到右下（与对比台同一位置）——作者：
         *   「我发现它有的撤销重做位置都不一样……你把他们的文案也统一一下」。
         * ★ 「取消」退休（那一颗统一收编：回只读并作废）；
         *   「保存修改」的按钮字收短成「保存」（三个模态框一个词）。
         */}
        <footer className={s.foot}>
          {ro ? (
            <>
              <div className={s.footLeft}>
                {/*
                 * mini 档只留「N 处未保存」—— 「保存会写回 xxx」那半句实测 153px，
                 * 会把底栏顶成两行（作者：「下面改不改都多一行」）。
                 * 半句话不丢：它本来就在保存确认弹层里说（「以下 N 处改动将写入 A1.toml」），
                 * 而且这里挂了 title，悬停还能看全。
                 */}
                <span className={s.state} title={stateText}>
                  {stateText}
                </span>
              </div>
              <div className={s.footRight}>
                <button
                  type="button"
                  className={s.primary}
                  title="进编辑模式：参数就可以改了"
                  onClick={() => setEditMode(true)}
                >
                  {EDIT_TEXT.enterMode}
                </button>
              </div>
            </>
          ) : (
            <>
              <div className={s.footLeft}>
                {/*
                 * 「恢复默认值」留在左下（右下的保存组离它远一点 —— 它一按下去
                 * 全部参数回出厂值，虽然在撤销栈里、也不该挨着保存键）。
                 */}
                <button type="button" className={s.textBtn} onClick={() => setAskRestore(true)}>
                  恢复默认值
                </button>
                <span className={s.state} title={stateText}>
                  {stateText}
                </span>
              </div>

              <div className={s.footRight}>
                {/*
                 * 撤销 / 重做改图标（作者：「撤销重做用图标吧，或者省略？只用快捷键」）。
                 * 选图标不选「省略」：当时**这一页还没有绑 Ctrl+Z / Ctrl+Y**（整份稿查过一遍），
                 * 删了就等于撤销这条路彻底看不见了。图标 28×28，两颗共省下约 44px。
                 * 快捷键后来补上了（见上面那个 keydown effect），tooltip 里写得出快捷键。
                 */}
                <button
                  type="button"
                  className={s.iconBtn}
                  disabled={!u.canUndo}
                  onClick={u.undo}
                  title="撤销（Ctrl+Z）"
                  aria-label="撤销"
                  aria-keyshortcuts="Control+Z"
                >
                  <Icon name="undo" size={15} />
                </button>
                <button
                  type="button"
                  className={s.iconBtn}
                  disabled={!u.canRedo}
                  onClick={u.redo}
                  title="重做（Ctrl+Y）"
                  aria-label="重做"
                  aria-keyshortcuts="Control+Y"
                >
                  <Icon name="redo" size={15} />
                </button>
                <button
                  type="button"
                  className={s.textBtn}
                  title={EDIT_TEXT.exitTitle}
                  onClick={() => {
                    /*
                     * **那一颗 = 作废**（作者 2026-10-16：「没有保存的话，退出编辑模式
                     * 就是作废，那些改的都作废，保存了才有」）——`discardAll` 正是这件事：
                     * 全部未保存的改动一次退回已保存值（草稿条目删掉、草稿 TOML 逐键回写）。
                     * 它本身是"一次动作"，重进编辑模式后按撤销还能整个拉回来。
                     */
                    u.discardAll()
                    setEditMode(false)
                  }}
                >
                  {EDIT_TEXT.exitMode}
                </button>
                <button
                  type="button"
                  className={s.primary}
                  disabled={u.dirtyCount === 0 && u.syncDirty === 0}
                  title="保存（Ctrl+S）"
                  aria-keyshortcuts="Control+S"
                  onClick={() => (u.dirtyCount === 0 ? u.save() : setAskSave(true))}
                >
                  保存
                </button>
              </div>
            </>
          )}
        </footer>

        {/* 切机型确认：有未保存改动时先问 */}
        {u.pendingCombo !== null && (
          <div className={s.scrim} role="presentation" onClick={u.cancelPending}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="丢弃改动"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>
                切换到 {u.pendingCombo.machineId} · {u.pendingCombo.versionId}？
              </h3>
              <p className={s.sheetText}>
                当前有 <b>{u.dirtyCount} 处未保存的改动</b>，切过去就丢了（修改历史也会清空）。
              </p>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={u.cancelPending}>
                  取消
                </button>
                <button type="button" className={s.danger} onClick={u.confirmPending}>
                  丢弃并切换
                </button>
              </div>
            </div>
          </div>
        )}

        {/*
         * 换一份预设的确认（按文件那条路，2026-10-14）。
         * 与上面"切机型"那格分开：那个按 combo 说，这个说得清是**哪一份文件**
         *（同一 combo 下有几份时，只有这一格答得出用户点的是哪份）。
         */}
        {u.pendingFile !== null && (
          <div className={s.scrim} role="presentation" onClick={u.cancelPending}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="丢弃改动"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>切换到 {u.pendingFile.fileName}？</h3>
              <p className={s.sheetText}>
                当前有 <b>{u.dirtyCount} 处未保存的改动</b>，切过去就丢了（修改历史也会清空）。
              </p>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={u.cancelPending}>
                  取消
                </button>
                <button type="button" className={s.danger} onClick={u.confirmPendingFile}>
                  丢弃并切换
                </button>
              </div>
            </div>
          </div>
        )}

        {askRestore && (
          <div className={s.scrim} role="presentation" onClick={() => setAskRestore(false)}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="恢复默认值"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>恢复默认值</h3>
              <p className={s.sheetText}>
                {/*
                 * 退回的是**目录里的出厂值**（2026-10-08 资源库改判）：那套"退到这份预设
                 * 当初那版云端的默认值"整个退场了。后果一样安全 —— 一次可整体撤销的改动，
                 * 都要按「保存修改」才落地。
                 */}
                会把全部参数改回出厂设置。
                它算<b>一次</b>改动，<b>按「撤销」可以整个退回来</b>；要落地还得按「保存修改」。
              </p>
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={() => setAskRestore(false)}>
                  取消
                </button>
                <button
                  type="button"
                  className={s.danger}
                  onClick={() => {
                    u.restoreDefaults()
                    setAskRestore(false)
                  }}
                >
                  恢复默认值
                </button>
              </div>
            </div>
          </div>
        )}

        {askSave && (
          <div className={s.scrim} role="presentation" onClick={() => setAskSave(false)}>
            <div
              className={s.sheet}
              role="dialog"
              aria-label="保存确认"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className={s.sheetTitle}>确认保存？</h3>
              {/*
                落点按**编辑目标**说（A1 统一口径，与 Rust 侧 `commit_preset_draft` 的两条
                落点一一对应）：云端底稿 → 另存成你自己的一份（原件一个字不动）；
                本地文件 → 写回它自己。原来那句"将写入 A1xxx.toml"说的是云端原件，
                实际却落 presets-local/ —— 测试报告里"我不敢赌"的那句话就是它。
              */}
              <p className={s.sheetText}>
                {u.editingPreset?.origin === 'local' ? (
                  <>
                    以下 <b>{changedEntries.length}</b> 处改动会写回你自己那一份{' '}
                    <b>{fileLabel}</b>：
                  </>
                ) : /\.[a-z0-9]+$/i.test(fileLabel) ? (
                  <>
                    以下 <b>{changedEntries.length}</b> 处改动会另存成你自己的一份：
                    presets-local/<b>{fileLabel.replace(/\.([a-z0-9]+)$/i, '（已修改）.$1')}</b>
                    （云端原件一个字不动）：
                  </>
                ) : (
                  <>
                    以下 <b>{changedEntries.length}</b> 处改动会另存成你自己的一份
                    （presets-local/，云端原件一个字不动）：
                  </>
                )}
              </p>
              <ul className={s.changeList}>
                {changedEntries.slice(0, MAX_CONFIRM_ITEMS).map((e) => (
                  <li key={e.key} className={s.changeItem}>
                    <span className={s.changeLabel}>{e.label}</span>
                    <span className={s.changeArrow}>
                      {e.from} → {e.to}
                    </span>
                  </li>
                ))}
                {changedEntries.length > MAX_CONFIRM_ITEMS && (
                  <li className={s.changeMore}>
                    …… 还有 {changedEntries.length - MAX_CONFIRM_ITEMS} 项
                  </li>
                )}
              </ul>
              {/*
                **范围闸的提示**（2026-10-20，作者：「超出范围了还允许我保存」）：
                有超范围项就把这一块摆出来、把「确认保存」灰掉 —— 后端的 patch 不做
                范围校验，落盘前唯一的闸在这里（`save` 里还有一道兜底）。
              */}
              {invalidEntries.length > 0 && (
                <div className={s.sheetWarn} role="alert">
                  <p className={s.sheetWarnTitle}>
                    有 <b>{invalidEntries.length}</b> 处超出取值范围，不能保存：
                  </p>
                  <ul className={s.sheetWarnList}>
                    {invalidEntries.map((e) => (
                      <li key={e.key}>
                        {e.label} —— {e.bad}（当前 {e.to}）
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              <div className={s.sheetFoot}>
                <button type="button" className={s.ghost} onClick={() => setAskSave(false)}>
                  取消
                </button>
                <button
                  type="button"
                  className={s.primary}
                  disabled={invalidEntries.length > 0}
                  title={invalidEntries.length > 0 ? '先把超范围的值改回范围内再保存' : undefined}
                  onClick={confirmSave}
                >
                  确认保存
                </button>
              </div>
            </div>
          </div>
        )}
        {/*
          「选择要更新的参数」那一屏（原「选择要跟随的更新」）。

          ★★ **挂到参数模态框的宿主层，不再嵌在它的内容里**（2026-10-20 作者：
             「我不希望它嵌套在这个修改参数的模态框里面，它能不能在外面呢？」）。

             `.page` 本身在「修改参数」模态框里就是框里的一块，所以原来它的遮罩
             只能盖住那个框、看起来像框里又套了一层。现在是 **portal 到宿主那一层**
             （`.paramsLayer`，`z-index: 70`）的**兄弟节点**：遮罩铺满整个内容区、
             它成为独立的一屏，参数框压在它下面；处理完 / 暂不修改回到参数框。

             ★ 为什么 portal 到这一层而不是 `document.body`：Modal 的遮罩是
               `position: absolute`（`Modal.tsx` 硬约定②，绝不用 fixed）。`body`
               不是定位祖先 ⇒ 遮罩会被定位到**文档最底下**，而外壳 `.shell` 是
               `overflow: hidden` —— 那就是"点了没任何反应"（2026-10-20 踩过）。
               这一层是定位祖先，遮罩正好铺满内容区。
        */}
        {syncLayer !== null &&
          createPortal(
            <ParamSyncModal
              open={syncOpen}
              sync={u.sync}
              defOf={u.defOf}
              draftOf={u.syncDraftOf}
              onDecide={u.applySyncDecisions}
              /* 0 项 = 没有草稿可写 ⇒ 这一颗当场落盘（「有新版」随之收掉） */
              onDigestZero={u.digestSyncRelease}
              digesting={u.syncBusy}
              onClose={() => setSyncOpen(false)}
            />,
            syncLayer,
          )}

        {/*
         预设抽屉。挂在 FieldLayer 里、`.main` 之后 —— absolute 的定位父级是 `.main`，
         放在最后只为了 z-index 顺序（它要盖在卡片与底部操作条之上）。
        */}
        <PresetPickerDrawer
          open={pickerOpen}
          /* ★ 2026-10-14：只列**本机真有**的（原来列目录那 12 个组合，点哪个都换不过去） */
          rows={pickerRows}
          /*
           * 在用那一份的**行键**（含文件名）—— 同组合里有几份文件时只打中真在用的那行。
           *
           * ★★ 2026-10-14 修：这里原来写 `u.fileLabel` —— 那是「**正在编辑**哪份」，
           *   而这一格问的是「**正在用**哪份」（两个问题，`useParams` 的注释里写着）。
           *   一个 combo 下有好几份时，编辑目标可能是别的一份 ⇒ 勾错行。
           *   改用 `u.activeFileName`（**AppState 底账答的**，与三页同一个源）。
           */
          currentKey={
            u.activeFileName !== null && u.machineId !== ''
              ? `${u.machineId}/${u.versionId}/${u.activeFileName}`
              : null
          }
          onUse={(row) => {
            /*
             * ★★ 2026-10-14：走**按文件**那条路 —— 抽屉列的是本机文件，同一 combo
             * 下可能有好几份，`requestCombo` 那种按 combo 的走法表达不了"要哪一份"
             * （它开头就按 combo 退货 ⇒ 点另一份没反应）。
             * 未保存改动的确认弹层在 `requestFile` 里（`pendingFile`）。
             */
            u.requestFile({
              machineId: row.machineId,
              versionId: row.versionId,
              fileName: row.fileName,
              path: row.ref,
            })
            closePicker()
          }}
          onClose={closePicker}
          drag={pickerDrag}
        />
      </FieldLayer>

      {/* 修改历史（悬浮态）挂在**页面根**（作者：「它是一个假的抽屉……
          应该紧贴右边」，而且它和底栏之间的断层也不要）—— 遮罩 inset:0 相对 .page
          通铺：贴住窗口右缘、通高盖过底栏，不再有「抽屉到 footer 为止」的截断。
          pinned 态仍留在 .main 里占位让内容让位。 */}
      {historyOpen && !pinned && (
        <HistoryDrawer
          open
          file={fileLabel}
          mode="float"
          canPin={canPin}
          width={historyDrag.width}
          drag={historyDrag}
          view={historyView}
          history={u.history}
          groups={u.historyByBatch}
          onView={setHistoryView}
          onMode={setHistoryMode}
          onClose={closeHistory}
          onRevert={onRevertHistory}
          /* 只读态抽屉里那颗「进入编辑模式」= 底栏同一颗（同一个 setEditMode） */
          onEnterEdit={() => setEditMode(true)}
          onClearHistory={u.clearHistory}
          note={revertNote}
          /* 只读态：历史照看，「还原」那颗不画（写入口） */
          readonly={ro}
        />
      )}

      {/* 保存成功之后那一句提问（2026-10-15）：切片器不会自己知道参数变了，
          要不要把"回它那边换一下板子类型"指给用户看。关掉只是这一次的决定。 */}
      {u.resliceAsk && <ResliceNotice open onClose={u.dismissReslice} />}
    </div>
  )
}
