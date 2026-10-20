/*
 * 参数定义编辑模态框（2026-10-03，作者：「弃用是谁决定的？我没办法改」）。
 *
 * 参数台右栏那一页「它是什么」此前是**只读的**：名称、单位、值类型、控件、
 * 范围、步进、出厂默认、属于、前置条件、弃用 —— 全住在
 * `presets/registry/param_registry.toml` 的 `[[params]]` 里，改一格要么手编
 * TOML、要么找人。这一框把它们搬进界面：**能选的不用打字**（值类型 / 控件 /
 * 前置条件的对象与档位 / 属于谁都是下拉），**能算的不用猜**（枚举的默认值得是
 * 选项之一、范围反了、int 挂小数 —— 当场不让存）。
 *
 * # 控件 × 值类型：只给画得出真的组合（2026-10-03 第二轮）
 *
 * 「控件」下拉不再摆全五种 —— 候选跟着值类型与可选项走（bool→开关、
 * float/int→步进框、string→分段/下拉/G-code/文本框），与后端
 * `registry::set_param_meta` 的校验同一张表。此前五种全给，选出「float 挂开关」
 * 这种组合：画出来是开关，点了写 true/false、归位成 NaN，行上永远不亮 ——
 * 作者的原话：「它实际上根本没有换一套」。
 *
 * # 可选项（choices）与适用机型（machineFilter）
 *
 * 字符串参数的可选项在这一组里编辑：值、显示名、**逐条弃用**（「护套弃不弃用
 * 谁说了算」就是这一格），增删随手。非字符串参数身上挂的几条是预设档（bool 的
 * 开/关叫法、数字的快捷档），界面不给编辑、保存时**原样带回** —— 不给编辑还
 * 顺带弄丢，那才是真Bug。适用机型是一排勾选：一个不勾 = 全部机型（与
 * `machineFilter` 空表同口径）。
 *
 * # 撤销 / 重做在框里（作者：「打开模态框撤销重做它还是撤销那外面的」）
 *
 * 框里的每一笔改动进**框自己的历史**（同格连续打字并成一步），页脚两颗按钮 +
 * Cmd+Z / Cmd+Shift+Z；Cmd+S = 确认保存。外壳的快捷键在框开着时由
 * `modalShortcutGate` 拦住，不会再穿透到遮罩后面去改看不见的草稿。
 * 保存本身走 `onCommit` 交给页面：落盘、把「改前/改后」整包压进外壳的撤销栈
 * （状态栏那颗「撤销」管得到它），然后关框。
 *
 * 校验的权威在后端（`registry::set_param_meta`）；这里先拦一道只是少挨一次错。
 */

import { useEffect, useRef, useState } from 'react'

import { isAppError } from '../api'
import type {
  MachineList,
  ParamMetaEdit,
  ParamView,
  SectionTarget,
  ShowOp,
  UiComponent,
  ValueType,
} from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import MachinePicker from '../components/MachinePicker'
import { machineSummary } from '../components/machineSummary'
import s from './ParamDefModal.module.css'

interface Props {
  /** 正在编辑哪条（`null` = 关着） */
  paramKey: string | null
  /** 那条参数现在的定义（来自注册表） */
  param: ParamView | null
  /** 全部参数 —— 「属于」「前置条件」的候选从这里来 */
  params: ParamView[]
  /**
   * 机型与品牌清单（「适用机型」「弃用机型」两个选择器的候选，打开定义框时现取）。
   * `null` = 还没读到 —— 选择器里如实说一句，不给假清单。
   */
  machineList: MachineList | null
  /**
   * 能装参数的分组（含暂时空掉的），按页签顺序 —— 「所属分组」下拉的候选。
   * 名单来自后端（`RegistryView.sections`）：**不从 params 反推**，
   * 否则把一个组搬空之后，那个组就再也搬不回去了。
   */
  sections: SectionTarget[]
  onClose: () => void
  /**
   * 确认保存：整包载荷交回页面 —— 那边负责调 `wb.setParamMeta` 落盘、把
   * 改前/改后压进外壳的撤销栈、关框。抛错 = 没存上，框留着。
   */
  onCommit: (edit: ParamMetaEdit) => Promise<void>
}

/** 值类型的中文注脚 —— 选项写原词（后端给什么就存什么），括号里说人话 */
const VT_LABEL: Record<ValueType, string> = {
  float: 'float · 小数',
  int: 'int · 整数',
  bool: 'bool · 开关',
  string: 'string · 文本',
}

const UC_LABEL: Record<UiComponent, string> = {
  number: 'number · 步进框',
  switch: 'switch · 开关',
  segmented: 'segmented · 分段',
  select: 'select · 下拉',
  gcode: 'gcode · 代码',
  text: 'text · 文本框',
}

const OP_LABEL: Record<ShowOp, string> = {
  eq: '等于',
  neq: '不等于',
  gt: '大于',
}

/**
 * 控件 × 值类型的合法候选。与后端 `registry::set_param_meta` 的校验同一张表 ——
 * 那边是权威，这里先拦一道只是少挨一次错。
 */
function controlCandidates(vt: ValueType, choiceCount: number): UiComponent[] {
  if (vt === 'bool') return ['switch']
  if (vt === 'float' || vt === 'int') return ['number']
  return choiceCount > 0 ? ['segmented', 'select'] : ['text', 'gcode']
}

/**
 * 界面上的草稿。数字三格（min / max / step）与出厂默认存**原文** ——
 * 打字的中间态（"0."、"-"）不是合法数字，但必须是合法草稿；空串 = 没填。
 * 可选项的值同样存原文（string 的枚举值本来就是字符串）。
 */
interface ChoiceDraft {
  value: string
  label: string
  /** 全局弃用（所有机型都不许写这一档） */
  deprecated: boolean
  /** 只在这些机型上弃用（与上一格叠加，空 = 不按机型弃用） */
  deprecatedMachines: string[]
}

interface Draft {
  label: string
  desc: string
  unit: string
  valueType: ValueType
  uiComponent: UiComponent
  defaultText: string
  min: string
  max: string
  step: string
  /** 空串 = 不属于谁 */
  parentKey: string
  /** 空串 = 无前置条件 */
  whenKey: string
  whenOp: ShowOp
  whenValue: string
  /** **全局弃用**（所有机型、所有版本都不许写） */
  deprecated: boolean
  /** **只在这些机型上**弃用（与上一格叠加） */
  deprecatedMachines: string[]
  choices: ChoiceDraft[]
  /** 空 = 全部机型 */
  machineFilter: string[]
  /** 所属分组（`layout_schema` 的 section id）。同组 = 后端不动 */
  sectionId: string
  /** 组内顺序（`[params.layout].order`）—— 上下箭头改它，客户端按它排 */
  order: number
}

/** 草稿连同它自己的历史 —— 撤销/重做是框里的事，不出这个框 */
interface FormState {
  draft: Draft
  past: Draft[]
  future: Draft[]
  /** 同一个 tag 的连续改动（往一个框里打字）并成一步撤销 */
  lastTag: string | null
  lastAt: number
}

/**
 * 草稿的**深拷贝**。
 *
 * ★ 2026-10-20 修的真 Bug：原来 `edit` 只做 `{ ...prev.draft }`，而 `choices` 是
 * 数组、每条是对象 —— 浅拷贝之后 `d.choices[i].value = …` 改的是**历史里每一份
 * 草稿共用的那个对象**。于是「改个选项的值 → 按撤销」看起来毫无反应：撤销确实
 * 换回了上一份 draft，可那一份里的选项已经被同一笔改动改掉了。
 * （截图实测：把「默认」的值改成「原」，撤销后还是「原」。）
 *
 * 目录里的可变结构只有这两处：`choices[]`（含它自己的 `deprecatedMachines`）与
 * `machineFilter[]`。其余全是标量，浅拷即安全。
 */
function cloneDraft(d: Draft): Draft {
  return {
    ...d,
    choices: d.choices.map((c) => ({ ...c, deprecatedMachines: [...c.deprecatedMachines] })),
    machineFilter: [...d.machineFilter],
  }
}

const numOf = (t: string): number | null => {
  const s = t.trim()
  if (s === '') return null
  const n = Number(s)
  return Number.isFinite(n) ? n : null
}

const initOf = (p: ParamView): Draft => ({
  label: p.label,
  desc: p.desc,
  unit: p.unit ?? '',
  valueType: p.valueType,
  uiComponent: p.uiComponent,
  defaultText: String(p.defaultValue),
  min: p.min === null ? '' : String(p.min),
  max: p.max === null ? '' : String(p.max),
  step: p.step === null || p.step === undefined ? '' : String(p.step),
  parentKey: p.parentKey ?? '',
  whenKey: p.showWhen?.key ?? '',
  whenOp: p.showWhen?.op ?? 'eq',
  whenValue: p.showWhen === null ? '' : String(p.showWhen.value),
  deprecated: p.deprecated,
  deprecatedMachines: [...p.deprecatedMachines],
  choices: p.choices.map((c) => ({
    value: String(c.value),
    label: c.label,
    /* 回写只认**显式**两格；推定的弃用（`c.derived`）是显示用的，不许固化成数据 */
    deprecated: c.deprecated,
    deprecatedMachines: [...c.deprecatedMachines],
  })),
  machineFilter: [...p.machineFilter],
  sectionId: p.sectionId,
  order: p.order,
})

/** 草稿 → 后端载荷。归位口径与 `writeValue` 同一套：bool 归 bool、数字归数字 */
function payloadOf(d: Draft, param: ParamView, params: ParamView[]): ParamMetaEdit {
  let defaultValue: unknown
  if (d.valueType === 'bool') defaultValue = d.defaultText === 'true'
  else if (d.valueType === 'float' || d.valueType === 'int') defaultValue = numOf(d.defaultText) ?? 0
  else defaultValue = d.defaultText

  const whenParam = params.find((p) => p.key === d.whenKey) ?? null
  let whenValue: unknown = d.whenValue
  if (whenParam !== null) {
    if (whenParam.valueType === 'bool') whenValue = d.whenValue === 'true'
    else if ((whenParam.valueType === 'float' || whenParam.valueType === 'int') && d.whenValue !== '')
      whenValue = Number(d.whenValue)
  }

  return {
    label: d.label,
    desc: d.desc,
    unit: d.unit.trim() === '' ? null : d.unit.trim(),
    valueType: d.valueType,
    uiComponent: d.uiComponent,
    defaultValue,
    min: numOf(d.min),
    max: numOf(d.max),
    step: numOf(d.step),
    parentKey: d.parentKey === '' ? null : d.parentKey,
    showWhen:
      d.whenKey === ''
        ? null
        : { key: d.whenKey, op: d.whenOp, value: whenValue },
    deprecated: d.deprecated,
    deprecatedMachines: [...d.deprecatedMachines],
    /*
     * 可选项：字符串枚举交草稿里编的那份；其他类型**原样带回**（预设档不丢 ——
     * bool 的开/关叫法、数字的快捷档不在这一框的编辑范围里，但也不许弄丢）
     */
    choices:
      d.valueType === 'string'
        ? d.choices.map((c) => ({
            label: c.label,
            value: c.value,
            deprecated: c.deprecated,
            deprecatedMachines: [...c.deprecatedMachines],
            /* 推定是后端展示用的派生位，回写永远不认它 */
            derived: false,
          }))
        : param.choices,
    machineFilter: [...d.machineFilter],
    sectionId: d.sectionId,
    /* 顺序是**数据**（客户端按它排同一组），所以整包带上；后端见值没变就不动 */
    order: d.order,
  }
}

/** 第一条过不去的门。`null` = 可以存 */
function errorOf(d: Draft): string | null {
  if (d.label.trim() === '') return '名称不能是空的'
  for (const [name, t] of [
    ['最小', d.min],
    ['最大', d.max],
    ['步进', d.step],
  ] as const) {
    if (t.trim() !== '' && !Number.isFinite(Number(t))) return `${name}值得是个数字`
  }
  const min = numOf(d.min)
  const max = numOf(d.max)
  if (min !== null && max !== null && min > max) return '范围反了：最小比最大还大'
  if (d.step.trim() !== '' && (Number(d.step) <= 0 || !Number.isFinite(Number(d.step))))
    return '步进得是正数'
  if (d.valueType === 'float' || d.valueType === 'int') {
    if (d.defaultText.trim() === '' || !Number.isFinite(Number(d.defaultText)))
      return '出厂默认得是个数字'
    if (d.valueType === 'int' && !Number.isInteger(Number(d.defaultText)))
      return 'int 的出厂默认得是整数'
  }
  if (d.valueType === 'string') {
    const seen = new Set<string>()
    for (const [i, c] of d.choices.entries()) {
      const n = i + 1
      if (c.label.trim() === '') return `第 ${n} 条可选项没写名字`
      if (c.value.trim() === '') return `第 ${n} 条可选项（${c.label.trim()}）没有值`
      if (seen.has(c.value)) return `第 ${n} 条可选项（${c.label.trim()}）的值跟前面重复了`
      seen.add(c.value)
    }
    if (d.choices.length > 0 && !d.choices.some((c) => c.value === d.defaultText))
      return '出厂默认得是选项之一'
  }
  if (d.whenKey !== '') {
    if (d.whenValue.trim() === '') return '前置条件的值不能是空的'
  }
  return null
}

/** 同格连续打字并成一步（400ms）；历史封顶 100 步 —— 撤到底也就一秒的事 */
const COALESCE_MS = 400
const HISTORY_MAX = 100

/**
 * 机型选择器现在在挑**哪一格**（三处共用一个嵌套框，不各开一个）：
 *
 *   filter     参数级「适用机型」
 *   paramDep   参数级「弃用机型」
 *   choiceDep  某一条可选项的「弃用机型」
 */
type PickTarget =
  | { kind: 'filter' }
  | { kind: 'paramDep' }
  | { kind: 'choiceDep'; index: number }

export default function ParamDefModal({
  paramKey,
  param,
  params,
  machineList,
  sections,
  onClose,
  onCommit,
}: Props) {
  const [form, setForm] = useState<FormState | null>(null)
  const [busy, setBusy] = useState(false)
  /** 机型选择器开着吗、在挑哪一格（模态框里的模态框，与套餐选版同一副长相） */
  const [pick, setPick] = useState<PickTarget | null>(null)
  /** 「全局弃用」二次确认（保存前拦一道，说清影响面） */
  const [confirmText, setConfirmText] = useState<string[] | null>(null)

  /*
   * 每次打开（或换了参数）都从「当前那份定义」重新起草 —— 上次取消的不该还留着。
   * 只跟着「换了哪条」走：框开着的时候注册表刷新（toast 上的撤销、外壳的重取）
   * 不许把正在打的草稿冲掉 —— 草稿是打开那一刻的快照，保存时整包交回、后端再验。
   */
  useEffect(() => {
    setForm(param === null ? null : { draft: initOf(param), past: [], future: [], lastTag: null, lastAt: 0 })
    setBusy(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 只按 paramKey 重开，见上
  }, [paramKey])

  /* 撤销 / 重做 / 保存走 ref —— 键盘监听只在开框时装一次，不必跟着每次打字重挂 */
  const undoRef = useRef<() => void>(() => {})
  const redoRef = useRef<() => void>(() => {})
  const saveRef = useRef<() => void>(() => {})

  /* 框里的快捷键。外壳那套在框开着时被 `modalShortcutGate` 拦住 —— 两不干扰 */
  useEffect(() => {
    if (paramKey === null) return
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      const k = e.key.toLowerCase()
      if (k === 'z' && !e.shiftKey) {
        e.preventDefault()
        undoRef.current()
      } else if ((k === 'z' && e.shiftKey) || k === 'y') {
        e.preventDefault()
        redoRef.current()
      } else if (k === 's') {
        e.preventDefault()
        saveRef.current()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [paramKey])

  if (paramKey === null || param === null || form === null) return null

  const draft = form.draft

  /**
   * 改一格。tag 相同且连续（400ms 内）的改动并成一步撤销； redo 链作废。
   *
   * **必须深拷**（见 [`cloneDraft`]）：浅拷的话 `fn` 里改嵌套对象会顺手把历史
   * 里每一份草稿一起改掉，撤销就成了空动作。
   */
  const edit = (tag: string, fn: (d: Draft) => void) =>
    setForm((prev) => {
      if (prev === null) return prev
      const next = cloneDraft(prev.draft)
      fn(next)
      const now = Date.now()
      const merge = prev.lastTag === tag && now - prev.lastAt < COALESCE_MS
      return {
        draft: next,
        past: merge ? prev.past : [...prev.past, prev.draft].slice(-HISTORY_MAX),
        future: [],
        lastTag: tag,
        lastAt: now,
      }
    })

  const undoForm = () =>
    setForm((prev) => {
      if (prev === null || prev.past.length === 0) return prev
      const past = prev.past.slice(0, -1)
      const draft = prev.past[prev.past.length - 1]!
      return { draft, past, future: [prev.draft, ...prev.future], lastTag: null, lastAt: 0 }
    })
  undoRef.current = undoForm

  const redoForm = () =>
    setForm((prev) => {
      if (prev === null || prev.future.length === 0) return prev
      const [draft, ...future] = prev.future
      return {
        draft,
        past: [...prev.past, prev.draft].slice(-HISTORY_MAX),
        future,
        lastTag: null,
        lastAt: 0,
      }
    })
  redoRef.current = redoForm

  /** 「属于」的候选：不是自己、也不是任何人的父项（层级只有两级，后端同判） */
  const parentsWithKids = new Set(
    params.filter((p) => p.parentKey !== null).map((p) => p.parentKey as string),
  )
  const parentCandidates = params.filter((p) => p.key !== param.key && !parentsWithKids.has(p.key))
  /** 「前置条件」的候选：任何不是自己的参数（实际上多半是父项或模式开关） */
  const whenCandidates = params.filter((p) => p.key !== param.key)
  const whenParam = whenCandidates.find((p) => p.key === draft.whenKey) ?? null

  /*
   * 「所属分组」的候选按页签分组（`optgroup`）。候选来自后端那份完整清单 ——
   * 含暂时空掉的组：把一个组搬空之后还得能搬回去。
   */
  const sectionGroups = (() => {
    const out: { tabId: string; tabLabel: string; sections: SectionTarget[] }[] = []
    for (const sec of sections) {
      const hit = out.find((g) => g.tabId === sec.tabId)
      if (hit === undefined) {
        out.push({ tabId: sec.tabId, tabLabel: sec.tabLabel, sections: [sec] })
      } else {
        hit.sections.push(sec)
      }
    }
    return out
  })()
  const hereSection = sections.find((x) => x.id === draft.sectionId) ?? null

  /*
   * 组内顺序（2026-10-20 作者：「客户端到时候也要消费到这个排序」）——
   * 它不是界面状态：同一把尺子还决定客户端那一列从上到下的先后，所以存进
   * `[params.layout].order`，并让后端把 `layout_schema` 的 items 一起重排。
   *
   * 排序时用**草稿里的 order**（不是注册表那份）：连按两次箭头，第二次要基于刚挪过的位置。
   */
  const siblings = [
    ...params
      .filter((p) => p.sectionId === draft.sectionId && p.key !== param.key)
      .map((p) => ({ key: p.key, order: p.order })),
    /* 自己那一条用**草稿**里的顺序：判的是「马上要存成什么样」，不是盘上那份 */
    { key: param.key, order: draft.order },
  ].sort((a, b) => a.order - b.order || a.key.localeCompare(b.key))
  const myAt = siblings.findIndex((p) => p.key === param.key)

  /** 上/下移一位：取两个邻居之间的中点（到头就往外扩 1），不动别人的 order */
  const moveOrder = (dir: -1 | 1) => {
    if (myAt < 0) return
    const other = dir < 0 ? siblings[myAt - 1] : siblings[myAt + 1]
    if (other === undefined) return
    const outer = dir < 0 ? siblings[myAt - 2] : siblings[myAt + 2]
    edit('order', (d) => {
      d.order =
        outer === undefined
          ? other.order + dir /* 到头：往外扩一格 */
          : (other.order + outer.order) / 2
    })
  }

  /** 换分组：顺序跟着落到目标组末尾（与后端「不给顺序就放末尾」同一条口径） */
  const onSection = (id: string) =>
    edit('sectionId', (d) => {
      d.sectionId = id
      let max: number | null = null
      for (const p of params) {
        if (p.sectionId !== id || p.key === param.key) continue
        max = max === null ? p.order : Math.max(max, p.order)
      }
      d.order = max === null ? 1 : max + 1
    })

  /** 可选项上/下移一位：数组顺序就是分段控件的先后，后端整表照写 */
  const moveChoice = (i: number, dir: -1 | 1) =>
    edit('choices-move', (d) => {
      const j = i + dir
      if (j < 0 || j >= d.choices.length) return
      const [moved] = d.choices.splice(i, 1)
      if (moved !== undefined) d.choices.splice(j, 0, moved)
    })

  const onValueType = (vt: ValueType) =>
    edit('valueType', (d) => {
      d.valueType = vt
      /* 值类型一换，控件候选跟着换 —— 原来的控件配不上就落到新类型的头一个 */
      const cand = controlCandidates(vt, d.choices.length)
      if (!cand.includes(d.uiComponent)) d.uiComponent = cand[0]!
      const looksNum = d.defaultText.trim() !== '' && Number.isFinite(Number(d.defaultText))
      if (vt === 'float' || vt === 'int') {
        if (!looksNum) d.defaultText = '0'
      } else if (vt === 'bool') {
        d.defaultText = d.defaultText === 'true' ? 'true' : 'false'
      } else {
        const first = d.choices.find((c) => !c.deprecated) ?? d.choices[0]
        d.defaultText = first ? first.value : ''
      }
    })

  const onWhenKey = (key: string) =>
    edit('whenKey', (d) => {
      d.whenKey = key
      const target = params.find((p) => p.key === key) ?? null
      if (target === null) {
        d.whenValue = ''
        return
      }
      if (target.valueType === 'bool') d.whenValue = 'true'
      else {
        const first = target.choices.find((c) => !c.deprecated) ?? target.choices[0]
        d.whenValue = first ? String(first.value) : ''
      }
    })

  const err = errorOf(draft)

  /** 这次保存会把「全局弃用」从关变成开吗 —— 是的话，先让人看清影响面 */
  const deprecatedConfirm = (): string[] | null => {
    if (!param.deprecated && draft.deprecated) {
      const kids = params.filter(
        (p) => p.parentKey === param.key && !p.deprecated && !p.deprecatedMachines.length,
      )
      return [
        `「${draft.label || param.label}」会在所有机型、所有版本上变成只读：值照旧读得到，但谁都改不了。`,
        kids.length > 0
          ? `它下面挂着的 ${kids.length} 条子参数（${kids.map((p) => p.label).join('、')}）也会跟着进不去。`
          : '',
      ].filter((x) => x !== '')
    }
    const lines: string[] = []
    for (let i = 0; i < draft.choices.length; i += 1) {
      const c = draft.choices[i]!
      const before = param.choices.find((x) => String(x.value) === c.value)
      if (before?.deprecated === true || !c.deprecated) continue
      /* 被这一档放开、而自己还没弃用的参数 —— 「选它就等于通向一片改不动的子树」 */
      const gated = params.filter((p) => {
        const sw = p.showWhen
        return (
          sw !== null &&
          sw.key === param.key &&
          sw.op === 'eq' &&
          String(sw.value) === c.value &&
          !p.deprecated &&
          p.deprecatedMachines.length === 0
        )
      })
      lines.push(
        `「${draft.label || param.label}」的「${c.label || c.value}」会在所有机型、所有版本上变成只读。` +
          (gated.length > 0
            ? `被它放开的 ${gated.length} 条参数（${gated.map((p) => p.label).join('、')}）也会一起变只读。`
            : ''),
      )
    }
    return lines.length > 0 ? lines : null
  }

  const doSave = async (): Promise<void> => {
    if (paramKey === null || param === null || form === null || busy) return
    const problem = errorOf(form.draft)
    if (problem !== null) {
      toasts.push(problem)
      return
    }
    setBusy(true)
    try {
      await onCommit(payloadOf(form.draft, param, params))
      /* 成功后页面关框 —— busy 不用回位，组件随框一起卸 */
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      setBusy(false)
    }
  }

  /** 保存按钮：先看有没有「新开的全局弃用」，有就先摆确认框 */
  const save = (): void => {
    if (paramKey === null || param === null || form === null || busy) return
    const problem = errorOf(form.draft)
    if (problem !== null) {
      toasts.push(problem)
      return
    }
    const lines = deprecatedConfirm()
    if (lines !== null) {
      setConfirmText(lines)
      return
    }
    void doSave()
  }
  saveRef.current = save

  /** 选择器当前挑的是哪一份清单 */
  const pickedMachines: string[] =
    pick === null
      ? []
      : pick.kind === 'filter'
        ? draft.machineFilter
        : pick.kind === 'paramDep'
          ? draft.deprecatedMachines
          : (draft.choices[pick.index]?.deprecatedMachines ?? [])

  /** 选完写回对应那一格（三处共用同一个框，只有落点不同） */
  const applyPicked = (next: string[]): void => {
    if (pick === null) return
    if (pick.kind === 'filter') {
      edit('machineFilter', (d) => {
        d.machineFilter = next
      })
    } else if (pick.kind === 'paramDep') {
      edit('deprecatedMachines', (d) => {
        d.deprecatedMachines = next
      })
    } else {
      edit(`choice.depMachines.${pick.index}`, (d) => {
        const row = d.choices[pick.index]
        if (row !== undefined) row.deprecatedMachines = next
      })
    }
  }

  /* 控件候选跟着值类型与可选项走 —— 「改成了下拉却不下拉」就是从这里堵死的 */
  const controlOptions = controlCandidates(draft.valueType, draft.choices.length)

  /* 出厂默认的控件跟着值类型走 —— 「值的类型肯定是选的」（作者），不是文本框糊弄 */
  const defaultField = (() => {
    if (draft.valueType === 'bool') {
      return (
        <select
          className={s.sel}
          value={draft.defaultText}
          aria-label="出厂默认 (defaultValue)"
          onChange={(e) => edit('defaultText', (d) => (d.defaultText = e.target.value))}
        >
          <option value="true">开启</option>
          <option value="false">关闭</option>
        </select>
      )
    }
    if (draft.valueType === 'float' || draft.valueType === 'int') {
      return (
        <input
          className={s.txt}
          value={draft.defaultText}
          inputMode="decimal"
          aria-label="出厂默认 (defaultValue)"
          onChange={(e) => edit('defaultText', (d) => (d.defaultText = e.target.value))}
        />
      )
    }
    if (draft.choices.length > 0) {
      return (
        <select
          className={s.sel}
          value={draft.defaultText}
          aria-label="出厂默认 (defaultValue)"
          onChange={(e) => edit('defaultText', (d) => (d.defaultText = e.target.value))}
        >
          {draft.choices.map((c) => (
            <option key={c.value} value={c.value}>
              {c.label}
              {c.deprecated ? '（已弃用）' : ''}
            </option>
          ))}
        </select>
      )
    }
    return (
      <input
        className={s.txt}
        value={draft.defaultText}
        aria-label="出厂默认 (defaultValue)"
        onChange={(e) => edit('defaultText', (d) => (d.defaultText = e.target.value))}
      />
    )
  })()

  /* 前置条件的值控件跟着被指向的参数走 */
  const whenValueField = (() => {
    if (whenParam === null) return null
    if (whenParam.valueType === 'bool') {
      return (
        <select
          className={s.sel}
          value={draft.whenValue}
          aria-label="前置条件的值 (showWhen.value)"
          onChange={(e) => edit('whenValue', (d) => (d.whenValue = e.target.value))}
        >
          <option value="true">开启</option>
          <option value="false">关闭</option>
        </select>
      )
    }
    if (whenParam.choices.length > 0) {
      return (
        <select
          className={s.sel}
          value={draft.whenValue}
          aria-label="前置条件的值 (showWhen.value)"
          onChange={(e) => edit('whenValue', (d) => (d.whenValue = e.target.value))}
        >
          {whenParam.choices.map((c) => (
            <option key={String(c.value)} value={String(c.value)}>
              {c.label}
              {c.deprecated ? '（已弃用）' : ''}
            </option>
          ))}
        </select>
      )
    }
    return (
      <input
        className={s.txt}
        value={draft.whenValue}
        inputMode={whenParam.valueType === 'float' || whenParam.valueType === 'int' ? 'decimal' : undefined}
        aria-label="前置条件的值 (showWhen.value)"
        onChange={(e) => edit('whenValue', (d) => (d.whenValue = e.target.value))}
      />
    )
  })()

  return (
    <ModalC14
      open
      title={`编辑定义 · ${param.label}`}
      subtitle={`写回 presets/registry/param_registry.toml 的 [[params]] —— 确认保存才落盘；Cmd+Z 撤框里的改动；注册键 ${param.key}`}
      size="lg"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          {err !== null && <span className={s.warn}>{err}</span>}
          <span className={s.grow} />
          <button
            type="button"
            className={s.btn}
            disabled={form.past.length === 0}
            title="撤回框里的上一步改动（Cmd+Z）"
            onClick={undoForm}
          >
            撤销
          </button>
          <button
            type="button"
            className={s.btn}
            disabled={form.future.length === 0}
            title="把刚撤回的改动再摆回来（Cmd+Shift+Z）"
            onClick={redoForm}
          >
            重做
          </button>
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={busy || err !== null}
            title={err ?? undefined}
            onClick={() => void save()}
          >
            {busy ? '保存中…' : '确认保存'}
          </button>
        </>
      }
    >
      <div className={s.group}>
        <div className={s.grid}>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              名称<em className={s.fieldKey}>label</em>
            </span>
            <input
              className={s.txt}
              value={draft.label}
              aria-label="名称 (label)"
              onChange={(e) => edit('label', (d) => (d.label = e.target.value))}
            />
          </label>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              说明<em className={s.fieldKey}>desc</em>
            </span>
            <input
              className={s.txt}
              value={draft.desc}
              aria-label="说明 (desc)"
              onChange={(e) => edit('desc', (d) => (d.desc = e.target.value))}
            />
          </label>
        </div>
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>类型与外观</div>
        <div className={s.grid}>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              值类型<em className={s.fieldKey}>valueType</em>
            </span>
            <select
              className={s.sel}
              value={draft.valueType}
              aria-label="值类型 (valueType)"
              onChange={(e) => onValueType(e.target.value as ValueType)}
            >
              {(Object.keys(VT_LABEL) as ValueType[]).map((vt) => (
                <option key={vt} value={vt}>
                  {VT_LABEL[vt]}
                </option>
              ))}
            </select>
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              控件<em className={s.fieldKey}>uiComponent</em>
            </span>
            <select
              className={s.sel}
              value={draft.uiComponent}
              aria-label="控件 (uiComponent)"
              onChange={(e) => edit('uiComponent', (d) => (d.uiComponent = e.target.value as UiComponent))}
            >
              {controlOptions.map((uc) => (
                <option key={uc} value={uc}>
                  {UC_LABEL[uc]}
                </option>
              ))}
            </select>
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              单位<em className={s.fieldKey}>unit</em>
            </span>
            <input
              className={s.txt}
              value={draft.unit}
              placeholder="空 = 无单位"
              aria-label="单位 (unit)"
              onChange={(e) => edit('unit', (d) => (d.unit = e.target.value))}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              步进<em className={s.fieldKey}>step</em>
            </span>
            <input
              className={s.txt}
              value={draft.step}
              placeholder="空 = 不限"
              inputMode="decimal"
              aria-label="步进 (step)"
              onChange={(e) => edit('step', (d) => (d.step = e.target.value))}
            />
          </label>
        </div>
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>范围与出厂默认</div>
        <div className={s.grid}>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              最小<em className={s.fieldKey}>min</em>
            </span>
            <input
              className={s.txt}
              value={draft.min}
              placeholder="空 = 不限"
              inputMode="decimal"
              aria-label="最小 (min)"
              onChange={(e) => edit('min', (d) => (d.min = e.target.value))}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              最大<em className={s.fieldKey}>max</em>
            </span>
            <input
              className={s.txt}
              value={draft.max}
              placeholder="空 = 不限"
              inputMode="decimal"
              aria-label="最大 (max)"
              onChange={(e) => edit('max', (d) => (d.max = e.target.value))}
            />
          </label>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              出厂默认<em className={s.fieldKey}>defaultValue</em>
            </span>
            {defaultField}
          </label>
        </div>
      </div>

      {/*
        可选项（字符串枚举才有这一组）。值 / 显示名 / 逐条弃用 / 删 ——
        「护套弃不弃用」就是右边那颗勾；删到没有时控件自动落回文本框
      */}
      {draft.valueType === 'string' && (
        <div className={s.group}>
          <div className={s.groupHead}>
            可选项<em className={s.fieldKey}>choices</em>
          </div>
          {draft.choices.map((c, i) => (
            <div className={s.choiceRow} key={i}>
              <input
                className={s.txt}
                value={c.value}
                placeholder="值（原词）"
                aria-label={`第 ${i + 1} 条可选项的值`}
                onChange={(e) =>
                  edit(`choice.value.${i}`, (d) => {
                    d.choices[i]!.value = e.target.value
                  })
                }
              />
              <input
                className={s.txt}
                value={c.label}
                placeholder="显示名"
                aria-label={`第 ${i + 1} 条可选项的显示名`}
                onChange={(e) =>
                  edit(`choice.label.${i}`, (d) => {
                    d.choices[i]!.label = e.target.value
                  })
                }
              />
              <span className={s.choiceMove}>
                <button
                  type="button"
                  className={s.orderBtn}
                  disabled={i === 0}
                  aria-label={`第 ${i + 1} 条可选项上移`}
                  title="上移一位（分段按钮的先后）"
                  onClick={() => moveChoice(i, -1)}
                >
                  ▲
                </button>
                <button
                  type="button"
                  className={s.orderBtn}
                  disabled={i >= draft.choices.length - 1}
                  aria-label={`第 ${i + 1} 条可选项下移`}
                  title="下移一位（分段按钮的先后）"
                  onClick={() => moveChoice(i, 1)}
                >
                  ▼
                </button>
              </span>
              <label className={s.choiceDep} title="所有机型、所有版本都不许再写这一档">
                <input
                  type="checkbox"
                  className={s.check}
                  checked={c.deprecated}
                  aria-label={`第 ${i + 1} 条可选项全局弃用`}
                  onChange={(e) =>
                    edit(`choice.dep.${i}`, (d) => {
                      d.choices[i]!.deprecated = e.target.checked
                    })
                  }
                />
                全局弃用
              </label>
              <button
                type="button"
                className={s.choiceMachines}
                aria-haspopup="dialog"
                title="只在这些机型上弃用这一档（与全局那一勾叠加）"
                onClick={() => setPick({ kind: 'choiceDep', index: i })}
              >
                {c.deprecatedMachines.length === 0 ? '机型…' : `机型 ${c.deprecatedMachines.length}`}
              </button>
              <button
                type="button"
                className={s.choiceDel}
                title="删掉这条可选项"
                aria-label={`删除第 ${i + 1} 条可选项`}
                onClick={() =>
                  edit('choices-remove', (d) => {
                    d.choices.splice(i, 1)
                    /* 删空了还挂着分段/下拉就是假控件 —— 落回文本框（后端同一条门） */
                    if (d.choices.length === 0 && (d.uiComponent === 'segmented' || d.uiComponent === 'select'))
                      d.uiComponent = 'text'
                  })
                }
              >
                ×
              </button>
            </div>
          ))}
          <div className={s.addChoice}>
            <button
              type="button"
              className={s.btn}
              onClick={() =>
                edit('choices-add', (d) => {
                  d.choices.push({
                    value: '',
                    label: '',
                    deprecated: false,
                    deprecatedMachines: [],
                  })
                  /* 从无到有的第一条 —— 文本框就该换成枚举的控件了 */
                  if (d.choices.length === 1 && d.uiComponent === 'text') d.uiComponent = 'segmented'
                })
              }
            >
              ＋ 加一条选项
            </button>
          </div>
        </div>
      )}

      <div className={s.group}>
        <div className={s.groupHead}>
          归属与条件<em className={s.fieldKey}>parentKey · showWhen</em>
        </div>
        <div className={s.grid}>
          {/*
            所属分组（2026-10-20，作者：「这个里面根本就没有看到它属于哪个 tab」）。
            **搬到别的页签 = 两份文件一起改**：注册表的 `[params.layout].sectionId`
            与 `layout_schema.toml` 的 items。后端在同一个事务里写，前端只管选。
          */}
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              所属分组<em className={s.fieldKey}>sectionId</em>
            </span>
            <select
              className={s.sel}
              value={draft.sectionId}
              aria-label="所属分组 (sectionId)"
              onChange={(e) => onSection(e.target.value)}
            >
              {sectionGroups.map((g) => (
                <optgroup key={g.tabId} label={g.tabLabel}>
                  {g.sections.map((sec) => (
                    <option key={sec.id} value={sec.id}>
                      {sec.label}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </label>
          {/*
            组内顺序：客户端按同一个 `layout.order` 排同一组里的参数 ——
            所以这不是「界面上的小偏好」，而是数据。后端保存时会把 items 一起重排。
          */}
          <label className={s.field}>
            <span className={s.fieldLabel}>
              组内顺序<em className={s.fieldKey}>layout.order</em>
            </span>
            <span className={s.orderRow}>
              <button
                type="button"
                className={s.orderBtn}
                disabled={myAt <= 0}
                aria-label="上移一位"
                title="上移一位"
                onClick={() => moveOrder(-1)}
              >
                ▲
              </button>
              <span className={s.orderText}>
                {myAt < 0 ? '—' : `第 ${myAt + 1} / ${siblings.length} 位`}
              </span>
              <button
                type="button"
                className={s.orderBtn}
                disabled={myAt < 0 || myAt >= siblings.length - 1}
                aria-label="下移一位"
                title="下移一位"
                onClick={() => moveOrder(1)}
              >
                ▼
              </button>
            </span>
          </label>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              属于<em className={s.fieldKey}>parentKey</em>
            </span>
            <select
              className={s.sel}
              value={draft.parentKey}
              aria-label="属于 (parentKey)"
              onChange={(e) => edit('parentKey', (d) => (d.parentKey = e.target.value))}
            >
              <option value="">不属于谁（顶层）</option>
              {parentCandidates.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}（{p.key}）
                </option>
              ))}
            </select>
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              前置条件<em className={s.fieldKey}>showWhen.key</em>
            </span>
            <select
              className={s.sel}
              value={draft.whenKey}
              aria-label="前置条件指向的参数 (showWhen.key)"
              onChange={(e) => onWhenKey(e.target.value)}
            >
              <option value="">无条件</option>
              {whenCandidates.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}（{p.key}）
                </option>
              ))}
            </select>
          </label>
          {whenParam !== null && (
            <>
              <label className={s.field}>
                <span className={s.fieldLabel}>
                  档位<em className={s.fieldKey}>showWhen.op</em>
                </span>
                <select
                  className={s.sel}
                  value={draft.whenOp}
                  aria-label="前置条件的档位 (showWhen.op)"
                  onChange={(e) => edit('whenOp', (d) => (d.whenOp = e.target.value as ShowOp))}
                >
                  {(Object.keys(OP_LABEL) as ShowOp[]).map((op) => (
                    <option key={op} value={op}>
                      {OP_LABEL[op]}
                    </option>
                  ))}
                </select>
              </label>
              <label className={s.field}>
                <span className={s.fieldLabel}>
                  值<em className={s.fieldKey}>showWhen.value</em>
                </span>
                {whenValueField}
              </label>
            </>
          )}
        </div>
        <p className={s.note}>
          {hereSection === null
            ? '这一组在布局里找不到了 —— 保存前先在上面选一个分组。'
            : `现在在：${hereSection.tabLabel} · ${hereSection.label}（保存后按新分组摆放）`}
        </p>
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>
          适用机型<em className={s.fieldKey}>machineFilter</em>
        </div>
        {/*
          机型多、品牌多的时候一横排勾选框摆不下（作者的实测意见）——
          收进一枚按钮，点开才是模态框里的模态框：按品牌分组的树状多选，
          与套餐「选择版本」同一副长相，按住还能刷选。
        */}
        <button
          type="button"
          className={s.pickerBtn}
          aria-haspopup="dialog"
          title="点开挑机型 —— 可多选，按住拖过去整批勾上/摘掉"
          onClick={() => setPick({ kind: 'filter' })}
        >
          <span className={s.pickerText}>{machineSummary(draft.machineFilter, machineList)}</span>
          <span className={s.pickerCaret} aria-hidden>
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
              <path d="m4 6 4 4 4-4" />
            </svg>
          </span>
        </button>
        <p className={s.note}>一个都不选 = 全部机型。摘掉某台机型的话，它身上已写的值会变成孤儿（检查页有提示）。</p>
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>
          状态<em className={s.fieldKey}>deprecated · deprecatedMachines</em>
        </div>
        <div className={s.grid}>
          <label className={s.field} title="勾上 = 所有机型、所有版本都不许再改它">
            <span className={s.fieldLabel}>
              全局弃用<em className={s.fieldKey}>deprecated</em>
            </span>
            <input
              className={s.check}
              type="checkbox"
              checked={draft.deprecated}
              aria-label="全局弃用 (deprecated)"
              onChange={(e) => edit('deprecated', (d) => (d.deprecated = e.target.checked))}
            />
          </label>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              弃用机型<em className={s.fieldKey}>deprecatedMachines</em>
            </span>
            <button
              type="button"
              className={s.pickerBtn}
              aria-haspopup="dialog"
              title="只在这些机型上不许改（与「全局弃用」叠加：全局勾了就不用再挑）"
              onClick={() => setPick({ kind: 'paramDep' })}
            >
              <span className={s.pickerText}>
                {draft.deprecated
                  ? '全局已弃用（这里只是留档）'
                  : machineSummary(draft.deprecatedMachines, machineList)}
              </span>
              <span className={s.pickerCaret} aria-hidden>
                <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
                  <path d="m4 6 4 4 4-4" />
                </svg>
              </span>
            </button>
          </label>
        </div>
        <p className={s.note}>
          两格叠加：勾上「全局弃用」= 所有机型都不许写；只挑机型 = 只这些机型不许写，别的机型照旧能改。
        </p>
      </div>

      <p className={s.hint}>
        改的是「这条参数是什么」，不是某一台机型上的值。值类型变了的话，已有的机型/版本
        覆盖值不会跟着迁移；非字符串参数身上挂的可选项是预设档（bool 的开/关叫法、数字的
        快捷档），这里不给编辑、保存时原样保留。
      </p>

      {/* 三处共用一个机型选择器（适用机型 / 参数级弃用机型 / 每条选项的弃用机型） */}
      <MachinePicker
        open={pick !== null}
        title={
          pick === null
            ? ''
            : pick.kind === 'filter'
              ? `选择适用机型 · ${param.label}`
              : pick.kind === 'paramDep'
                ? `选择弃用机型 · ${param.label}`
                : `选择弃用机型 · ${param.label} 的「${draft.choices[pick.index]?.label ?? ''}」`
        }
        subtitle={
          pick !== null && pick.kind === 'filter'
            ? '点一下勾/摘；按住拖过去整批刷。一个都不选 = 全部机型'
            : '点一下勾/摘；按住拖过去整批刷。一个都不选 = 不限机型'
        }
        machines={machineList}
        selected={pickedMachines}
        onChange={applyPicked}
        onClose={() => setPick(null)}
      />

      {/*
        全局弃用的确认（2026-10-20）。作者要的不是"再点一次保存"，
        而是**看清这一勾影响到谁**：所有机型、所有版本，以及被这一档放开、
        而自己还没弃用的子参数 —— 它们会跟着一起变成只读。
      */}
      <ModalC14
        open={confirmText !== null}
        title={`确认全局弃用 · ${param.label}`}
        subtitle="这一步影响的是定义，不是某一台机型上的值 —— 确认后立即写回注册表"
        size="md"
        closeOnScrim={false}
        onClose={() => setConfirmText(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setConfirmText(null)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnDanger}`}
              onClick={() => {
                setConfirmText(null)
                void doSave()
              }}
            >
              确认弃用
            </button>
          </>
        }
      >
        <ul className={s.confirmList}>
          {(confirmText ?? []).map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
        <p className={s.note}>
          已有的值不会丢：客户端照旧读得到，只是参数台上这一项（或这一档）不能再被改。
        </p>
      </ModalC14>
    </ModalC14>
  )
}
