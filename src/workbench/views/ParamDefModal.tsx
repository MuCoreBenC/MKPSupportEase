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
 * Cmd+Z / Cmd+Shift+Z；Cmd+S = 确认（只进草稿，落盘归右上角「保存」）。外壳的快捷键在框开着时由
 * `modalShortcutGate` 拦住，不会再穿透到遮罩后面去改看不见的草稿。
 * 保存本身走 `onCommit` 交给页面：落盘、把「改前/改后」整包压进外壳的撤销栈
 * （状态栏那颗「撤销」管得到它），然后关框。
 *
 * # 分组下拉是自己画的（2026-11-01）
 *
 * 作者：「我希望在这里能加上新增的按钮」—— 原生 `<select>` 里没地方挂按钮，
 * 页签也只能当 `optgroup` 的死标签，所以「所属分组」换成 `SectionPicker`：
 * 每个页签那一行右边一颗「＋ 新增分组」。新建的那一组**先只进草稿**，保存那一趟
 * 才跟定义一起落盘（`payloadOf` 的 `newSection`）—— 建了又取消，数据里不会多出
 * 一个空组（这里没有删分组的入口，留下的就再也删不掉了）。
 *
 * 校验的权威在后端（`registry::set_param_meta`）；这里先拦一道只是少挨一次错。
 */

import { useEffect, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  MachineList,
  NewSection,
  ParamMetaEdit,
  ParamView,
  RegistryView,
  SectionTarget,
  ShowOp,
  UiComponent,
  ValueType,
} from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import MachinePicker from '../components/MachinePicker'
import { machineSummary } from '../components/machineSummary'
import GroupPicker from './GroupPicker'
import type { GroupedGroup, GroupedOption } from './GroupPicker'
import SectionPicker from './SectionPicker'
import type { SectionGroup } from './SectionPicker'
/* 新分组的 id 规则：与分组抽屉的「＋ 新增分组」共用同一份实现，见那个文件的头注 */
import { nextSectionId } from './sectionId'
/* 上移 / 下移那两颗箭头：客户端共享那一颗（同一颗 chevronDown，朝上的翻 180°），
   不再排「▲ / ▼」字形 —— 字形的墨迹高度跟着字体走，按钮里看着忽高忽低 */
import Icon from '../../app/shell/icons'
import ui from '../../app/ui/ui.module.css'
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
  /**
   * 译文台账被清过了（逃生通道那一颗）。页面拿回新注册表去顶掉旧的、刷开「未保存
   * 改动」并重取派生数据 —— 这一笔同样是草稿，右上角「保存」才落盘。
   */
  onStaleCleared: (next: RegistryView, key: string) => void
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
 * 注册表里那三册的编辑格（2026-10-31）。
 *
 * 作者口径：「它是注册表，中英文日文就该一起存里面」—— 所以这一框不是另开一张
 * 翻译表，而是**同一个 `[[params]]` 块**里的 `labelEn/labelJa/labelTw` 与
 * `descEn/descJa/descTw`。中文那两格在上面（必填），这三组可空：
 * 空 = 那一册没译，客户端按语言回落中文。
 *
 * ★ 2026-11-02：这三组**各占一整行**（名称 / 说明各一行），与「名称 / 说明」那两格
 * 同宽。原来两列并排时，每格固定 `flex: 0 0 128px` 的标签一占，输入框只剩几十像素，
 * 英文句子只看得见头三个词 —— 作者的截图就是这一屏（「其他的语言不完整」）。
 * 格子里的语言名单独成一行（`langHead`），标签本身退成短名（名称 / 说明）。
 */
const LANG_FIELDS = [
  { title: 'English', labelKey: 'labelEn', descKey: 'descEn' },
  { title: '日本語', labelKey: 'labelJa', descKey: 'descJa' },
  { title: '繁體中文', labelKey: 'labelTw', descKey: 'descTw' },
] as const

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
  /** 另外三册（2026-10-31）—— 这一框不编它们，但**原样带回**，保存不许丢 */
  labelEn: string
  labelJa: string
  labelTw: string
  /** 全局弃用（所有机型都不许写这一档） */
  deprecated: boolean
  /** 只在这些机型上弃用（与上一格叠加，空 = 不按机型弃用） */
  deprecatedMachines: string[]
}

interface Draft {
  label: string
  labelEn: string
  labelJa: string
  labelTw: string
  desc: string
  descEn: string
  descJa: string
  descTw: string
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
  /**
   * 草稿里新建的分组（`sectionId` 指向它时才有意义）。
   *
   * **只进草稿**：保存那一趟才跟定义一起落盘 ——「建了又取消」不该在数据里留下
   * 一个删不掉的空组。改了主意选别的组的话，这份也随之作废（见 `onSection`），
   * 真交出去时只有 `sectionId` 指着它的那一份（见 `payloadOf`）
   */
  newSection: NewSection | null
  /**
   * 「所属分组」面板里就地改过的组名（id → 新名字）。**只活在这一框的草稿里**：
   * 盘上那份改名走参数台卡片抽屉那条命令（一次写完两份文件、撤销栈认得出），
   * 这里改的是"这份候选清单看起来叫什么"，确认保存后至少这一框不会骗人。
   */
  sectionLabelEdits: Record<string, string>
  /** 分组候选清单的次序（上下箭头的效果）。空 = 用注册表给的次序 */
  sectionOrder: string[]
  /** 组内顺序（`[params.layout].order`）—— 上下箭头改它，客户端按它排 */
  order: number
}

/** 在一份 id 次序表里把 `id` 挪一位；表里还没有它就什么都不做 */
function moveInList(order: string[], id: string, dir: -1 | 1): string[] {
  const at = order.indexOf(id)
  const to = at + dir
  if (at < 0 || to < 0 || to >= order.length) return order
  const next = [...order]
  const [moved] = next.splice(at, 1)
  if (moved !== undefined) next.splice(to, 0, moved)
  return next
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
    /* 新分组也是可变的（名字能被改），历史里那几份各存各的 */
    newSection: d.newSection === null ? null : { ...d.newSection },
    /* 改名表与次序表同理：浅拷会让撤销改到历史里那一份 */
    sectionLabelEdits: { ...d.sectionLabelEdits },
    sectionOrder: [...d.sectionOrder],
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
  labelEn: p.labelEn,
  labelJa: p.labelJa,
  labelTw: p.labelTw,
  desc: p.desc,
  descEn: p.descEn,
  descJa: p.descJa,
  descTw: p.descTw,
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
    labelEn: c.labelEn,
    labelJa: c.labelJa,
    labelTw: c.labelTw,
    /* 回写只认**显式**两格；推定的弃用（`c.derived`）是显示用的，不许固化成数据 */
    deprecated: c.deprecated,
    deprecatedMachines: [...c.deprecatedMachines],
  })),
  machineFilter: [...p.machineFilter],
  sectionId: p.sectionId,
  newSection: null,
  sectionLabelEdits: {},
  sectionOrder: [],
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

  /* 新建的那一组只有在「选中的就是它」时才算这一笔 —— 见下面 `newSection` 那段 */
  const pending = d.newSection !== null && d.newSection.id === d.sectionId ? d.newSection : null

  /*
   * 另外三册**整包带回**（2026-11-02 修）：此前这两处只交了中文两格，`labelEn`
   * 等六格压根没进载荷 ⇒ 在这个框里打了英文 / 日文 / 繁中，点「确认」再看一眼
   * 还是旧内容（后端 `None` = 不动）。空串照旧是「清掉那一册」，与后端同一套语义。
   */
  return {
    label: d.label,
    labelEn: d.labelEn.trim(),
    labelJa: d.labelJa.trim(),
    labelTw: d.labelTw.trim(),
    desc: d.desc,
    descEn: d.descEn.trim(),
    descJa: d.descJa.trim(),
    descTw: d.descTw.trim(),
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
            labelEn: c.labelEn,
            labelJa: c.labelJa,
            labelTw: c.labelTw,
            value: c.value,
            deprecated: c.deprecated,
            deprecatedMachines: [...c.deprecatedMachines],
            /* 推定是后端展示用的派生位，回写永远不认它 */
            derived: false,
          }))
        : param.choices,
    machineFilter: [...d.machineFilter],
    /*
     * 草稿里新建的那个分组：**只有真的选中它**才交出去。写了名字又改选别的组，
     * 那些字一个都不落盘 —— 数据里不留空组（这里也没有删分组的入口）。
     */
    sectionId: pending === null ? d.sectionId : null,
    newSection: pending,
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
  onStaleCleared,
}: Props) {
  const [form, setForm] = useState<FormState | null>(null)
  const [busy, setBusy] = useState(false)
  /** 机型选择器开着吗、在挑哪一格（模态框里的模态框，与套餐选版同一副长相） */
  const [pick, setPick] = useState<PickTarget | null>(null)
  /**
   * 「其他语言」是否展开（2026-11-02 作者：「默认可以折叠起来」）。
   *
   * **默认收起**，但一旦有译文过期就**自动展开一次** —— 那几格必须让人看见，
   * 收着等于把"还有旧译文"藏起来。之后开合完全听用户的（`touched` 记这一笔，
   * 免得他刚手动收起、下一次渲染又被过期提示弹开）。
   */
  const [langOpen, setLangOpen] = useState(false)
  const langTouched = useRef(false)
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
    /* 换一条参数 = 换一份草稿，「其他语言」回到收起态（上面那个自动展开会重新判一次） */
    setLangOpen(false)
    langTouched.current = false
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

  /*
   * 有译文过期且用户没手动开合过 ⇒ 展开一次（见 `langOpen` 那段的解释）。
   *
   * ★ 这条 effect 必须住在下面那个 `return null` **之前** —— 钩子的调用顺序
   * 不许随渲染变化（`react-hooks/rules-of-hooks` 钉着）。
   */
  useEffect(() => {
    if (!langTouched.current && (param?.staleTranslations.length ?? 0) > 0) setLangOpen(true)
  }, [param])

  if (paramKey === null || param === null || form === null) return null

  const draft = form.draft

  /*
   * 哪几格译文过期了。**认打开那一刻的定义**（`param`，即盘上/草稿里已经记下的
   * 台账），不认草稿里正在打的字 —— 这一框只负责显示"欠着哪几笔"，
   * 摘标记是后端在保存时按"值真的变了"做的。
   */
  const staleList = param.staleTranslations

  /**
   * 逃生通道：这几格其实已经由别的路译好了（直接改文件 / 按待办清单整批改），
   * 那就在后端点名清掉。**不校验译文有没有真的改过** —— 后端那条老判据
   * （`set_param_meta` 里"值变了才摘"）一个字没动，撤销/重做照样幂等；
   * 这里是另一条显式的路，按的这颗就是那句"我确认已经改好了"。
   */
  const clearStale = async (): Promise<void> => {
    if (paramKey === null || busy) return
    setBusy(true)
    try {
      onStaleCleared(await wb.clearStaleTranslations(paramKey, [...staleList]), paramKey)
      toasts.push(`已清掉 ${staleList.length} 个译文标记 —— 记得右上角「保存」才写文件`)
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

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
   * 「所属分组」的候选按页签分块。候选来自后端那份完整清单 ——
   * 含暂时空掉的组：把一个组搬空之后还得能搬回去。
   *
   * 草稿里新建的那一组**也在这里**（挂到它那个页签的末尾、带「待保存」）：
   * 它不是盘上的数据，但选择器得先能显示、能选中它，否则「刚建完就选不中」。
   */
  const sectionGroups: SectionGroup[] = (() => {
    /*
     * 「所属分组」只留**能装参数**的分组：`component` 整块组件卡不进这一列
     * （后端 `move_param` 拒收，`ui.presetdata.registry.329`）。唯一的例外是
     * **这一条参数现在就住在里面** —— 那一行得留着，否则触发器与「现在在」会
     * 说"这一组在布局里找不到了"。
     */
    let list: SectionTarget[] = sections
      .filter((sec) => sec.component === null || sec.id === draft.sectionId)
      .map((sec) => {
        const renamed = draft.sectionLabelEdits[sec.id]
        return renamed === undefined ? sec : { ...sec, label: renamed }
      })
    if (draft.sectionOrder.length > 0) {
      const at = new Map(draft.sectionOrder.map((id, i) => [id, i] as const))
      list = [...list].sort((a, b) => (at.get(a.id) ?? 1e9) - (at.get(b.id) ?? 1e9))
    }
    if (draft.newSection !== null) {
      const tab = sections.find((x) => x.tabId === draft.newSection?.tabId)
      list.push({
        id: draft.newSection.id,
        label: draft.newSection.label,
        tabId: draft.newSection.tabId,
        tabLabel: tab?.tabLabel ?? draft.newSection.tabId,
        /* 新分组一定是普通分组（组件卡不是这条路上建得出来的） */
        component: null,
        standalone: false,
      })
    }
    const out: SectionGroup[] = []
    for (const sec of list) {
      const hit = out.find((g) => g.tabId === sec.tabId)
      if (hit === undefined) {
        out.push({ tabId: sec.tabId, tabLabel: sec.tabLabel, sections: [sec] })
      } else {
        hit.sections.push(sec)
      }
    }
    return out
  })()
  /**
   * 把一列参数**按分组分类**（2026-11-02 作者：「有层级的，分类的，自制的」）。
   *
   * 原来「属于」和「前置条件」是原生 `<select>` 摊成一个几十项的长名单 ——
   * 一屏放不下、还得靠 `（键名）` 那串英文才认得出是谁。现在按**分组**分块，
   * 块标题用分组名（「偏移」「擦料」「涂胶」…），行的旁注放键名（浅色、靠右）。
   *
   * 分块的顺序跟 `sectionGroups` 走（页签 → 分组），所以两个下拉的块序与
   * 「所属分组」一致；查不到分组的（理论上不该有）兜进最后一块，**别弄丢选项**。
   */
  const groupParams = (
    list: ParamView[],
    empty: GroupedOption,
  ): GroupedGroup[] => {
    const out: GroupedGroup[] = [{ title: null, options: [empty] }]
    const bySection = new Map<string, ParamView[]>()
    for (const p of list) {
      const arr = bySection.get(p.sectionId)
      if (arr === undefined) bySection.set(p.sectionId, [p])
      else arr.push(p)
    }
    const seen = new Set<string>()
    for (const g of sectionGroups) {
      for (const sec of g.sections) {
        const hits = bySection.get(sec.id)
        if (hits === undefined) continue
        seen.add(sec.id)
        out.push({
          /* 块标题带上页签：两组同名（不同页签下都有「偏移」）时不会混 */
          title: `${g.tabLabel} · ${sec.label}`,
          options: [...hits]
            .sort((a, b) => a.order - b.order)
            .map((p) => ({ value: p.key, label: p.label, by: p.key })),
        })
      }
    }
    const rest = list.filter((p) => !seen.has(p.sectionId))
    if (rest.length > 0) {
      out.push({
        title: '未归分组',
        options: [...rest]
          .sort((a, b) => a.label.localeCompare(b.label))
          .map((p) => ({ value: p.key, label: p.label, by: p.key })),
      })
    }
    return out
  }

  const parentGroups = groupParams(parentCandidates, { value: '', label: '不属于谁（顶层）' })
  const whenGroups = groupParams(whenCandidates, { value: '', label: '无条件' })

  const hereSection =
    sectionGroups.flatMap((g) => g.sections).find((x) => x.id === draft.sectionId) ?? null
  const herePending = draft.newSection !== null && draft.newSection.id === draft.sectionId

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

  /**
   * 「组内顺序」下拉里的那张表（2026-11-02）：**这一组的所有参数按当下顺序排**，
   * 每行右边常驻上下箭头（`GroupedOption.move`），在下拉里直接调序。
   *
   * 三条约束：
   *
   *   · 行序 = `siblings`（已按草稿里的 order 排好），**不另算一遍** ——
   *     两处各排一次迟早会不一致，而这一框的全部意义就是"看到的就是要存的"；
   *   · 箭头只对**当前这一条**有效：`moveOrder` 改的是草稿里 `d.order`，
   *     挪别人需要先选中它、再开这一框 —— 那是另一件事，这里不做；
   *     所以别的行箭头置灰并给出理由（别让人点了没反应）；
   *   · 触发器上写「第 N / 共 M 位」，收起也不丢位置信息。
   */
  const orderGroups: GroupedGroup[] = [
    {
      title: null,
      options: siblings.map((sib, i) => {
        const isSelf = sib.key === param.key
        const p = params.find((x) => x.key === sib.key)
        return {
          value: sib.key,
          label: p?.label ?? sib.key,
          by: `第 ${i + 1} / ${siblings.length} 位`,
          ...(isSelf
            ? {
                move: {
                  up: myAt > 0,
                  down: myAt >= 0 && myAt < siblings.length - 1,
                  onMove: moveOrder,
                },
              }
            : {
                /* 挪别人得先选它 —— 这里点了也不该"看起来能动" */
                move: { up: false, down: false, onMove: () => undefined },
                note: '只能挪这一条',
              }),
        }
      }),
    },
  ]

  /**
   * 换分组：顺序跟着落到目标组末尾（与后端「不给顺序就放末尾」同一条口径）。
   *
   * 改选了别的组 ⇒ 草稿里那个新分组**作废**（下拉里随之消失）：那一行写着「待保存」，
   * 留着却不会落盘才是撒谎。
   */
  const onSection = (id: string) =>
    edit('sectionId', (d) => {
      if (d.newSection !== null && d.newSection.id !== id) d.newSection = null
      d.sectionId = id
      let max: number | null = null
      for (const p of params) {
        if (p.sectionId !== id || p.key === param.key) continue
        max = max === null ? p.order : Math.max(max, p.order)
      }
      d.order = max === null ? 1 : max + 1
    })

  /**
   * 在某个页签下新建分组（分组下拉里那颗「＋」）。
   *
   * **只进草稿**：保存那一趟才跟定义一起落盘（`payloadOf` 的 `newSection`），
   * 所以建了又取消 = 数据里什么也没留下。id 在这里就定死（`custom_N`）：
   * 撤销 / 重做会把同一笔载荷原样重放，后端按「确保这个分组在」处理。
   * 顺序落到 1 —— 新组里眼下只有这一条。
   */
  const onAddSection = (tabId: string, label: string) =>
    edit('newSection', (d) => {
      const id = nextSectionId(d.newSection === null ? sections : [...sections, d.newSection])
      d.newSection = { id, tabId, label }
      d.sectionId = id
      d.order = 1
    })

  /**
   * 在「所属分组」面板里就地改名（2026-11-01）。
   *
   * 改的是**盘上已有的**那一组 ⇒ 只改草稿里这份候选清单（它就是面板的数据源），
   * 连同载荷一起在确认保存时才交出去；改的是**这一笔刚建的**新分组 ⇒ 直接把
   * 草稿里那个 `newSection.label` 换掉（那一份本来就没落盘，不必另起一笔）。
   */
  const onRenameSection = (id: string, label: string) =>
    edit('section-rename', (d) => {
      /* 草稿里的改名表：值 = 新名字（盘上那份不动；新分组另有一处，见上） */
      d.sectionLabelEdits = { ...d.sectionLabelEdits, [id]: label }
      if (d.newSection !== null && d.newSection.id === id) {
        d.newSection = { ...d.newSection, label }
      }
    })

  /**
   * 组间换位（面板里那两颗 ↑↓）。
   *
   * 这一框里只挪**草稿那份候选清单的次序**（`sectionOrder`）—— 它决定保存后
   * 「所属分组」里读到的先后。真要把一组的次序写进注册表，用参数台卡片抽屉里的
   * 那颗：那里是一条独立命令、一次写完两份文件，撤销栈也认得出它在改什么。
   *
   * 还只在草稿里的新分组没有盘上的位置可挪（它排在它那个页签的末尾）。
   */
  const onMoveSection = (id: string, dir: -1 | 1) =>
    edit('section-move', (d) => {
      if (d.newSection !== null && d.newSection.id === id) return
      d.sectionOrder = moveInList(d.sectionOrder, id, dir)
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
        <GroupPicker
          label="出厂默认"
          groups={[
            {
              title: null,
              options: [
                { value: 'true', label: '开启' },
                { value: 'false', label: '关闭' },
              ],
            },
          ]}
          value={draft.defaultText}
          onChange={(v) => edit('defaultText', (d) => (d.defaultText = v))}
          width={200}
        />
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
        <GroupPicker
          label="出厂默认"
          groups={[
            {
              title: null,
              options: draft.choices.map((c) => ({
                value: c.value,
                label: c.label,
                note: c.deprecated ? '已弃用' : undefined,
              })),
            },
          ]}
          value={draft.defaultText}
          onChange={(v) => edit('defaultText', (d) => (d.defaultText = v))}
          width={260}
        />
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
        <GroupPicker
          label="前置条件的值"
          groups={[
            {
              title: null,
              options: [
                { value: 'true', label: '开启' },
                { value: 'false', label: '关闭' },
              ],
            },
          ]}
          value={draft.whenValue}
          onChange={(v) => edit('whenValue', (d) => (d.whenValue = v))}
          width={200}
        />
      )
    }
    if (whenParam.choices.length > 0) {
      return (
        <GroupPicker
          label="前置条件的值"
          groups={[
            {
              title: null,
              options: whenParam.choices.map((c) => ({
                value: String(c.value),
                label: c.label,
                note: c.deprecated ? '已弃用' : undefined,
              })),
            },
          ]}
          value={draft.whenValue}
          onChange={(v) => edit('whenValue', (d) => (d.whenValue = v))}
          width={260}
        />
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
      /*
       * 2026-11-01 口径（作者）：**只有右上角那颗「保存」才写文件**，这里这颗只是
       * 「确认」——把框里的改动收进草稿。所以副标题不再写"写回…才落盘"，
       * 按钮也不叫「确认保存」：它确实不保存。
       */
      subtitle={`改的是 presets/registry/param_registry.toml 的 [[params]] —— 确认只进草稿，右上角「保存」才写文件；Cmd+Z 撤框里的改动；注册键 ${param.key}`}
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
            data-tip={err ?? undefined}
            onClick={() => void save()}
          >
            {busy ? '应用中…' : '确认'}
          </button>
        </>
      }
    >
      <div className={s.group}>
        <div className={s.grid}>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              名称
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
              说明
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

      {/*
        其他语言（2026-11-02 作者：「其他语言默认可以折叠起来」）。
        **默认收起**：三册译文是"偶尔才改"的东西，常驻展开会把下面的类型 / 范围 /
        归属全推出去，改一个步进要先滚过九格输入框。
        标题行本身就是开关（整行可点，不只是一枚小箭头）；过期提示在**收起时也留着**，
        否则"有几册要重写"这件事就被藏起来了。
      */}
      <div className={s.group}>
        <button
          type="button"
          className={s.groupToggle}
          aria-expanded={langOpen}
          onClick={() => {
            langTouched.current = true
            setLangOpen((o) => !o)
          }}
        >
          <Icon name="chevronDown" size={12} className={langOpen ? s.toggleOpen : s.toggle} />
          <span>其他语言</span>
          <span className={s.toggleHint}>
            {staleList.length > 0 ? `${staleList.length} 格待重写` : `${LANG_FIELDS.length} 册译文`}
          </span>
        </button>
        {/*
          过期提示（2026-11-02）：中文改过、那一册没跟着重写 ⇒ 后端把键名记在
          `staleTranslations` 里。这里只是**摊开事实**（哪一册还用着旧中文的译文），
          不给"一键机翻"那种假按钮 —— 重写译文（值变了）那一笔保存才摘掉标记。
        */}
        {staleList.length > 0 && (
          <p className={s.staleNote}>
            {`中文改过之后，这几格还是旧译：${staleList.join(' · ')} —— 重写它们（内容变了）再确认，标记就摘掉。`}
            {/*
              逃生通道（2026-11-02）：译文可能是从别的路写的（直接改文件、或按
              `scripts/stale-translations.mjs` 的清单整批改），那种"改"后端看不见，
              标记就永远摘不掉。这颗**点名清**当前这批，判据不在前端重算。
            */}
            <button type="button" className={s.staleClear} onClick={() => void clearStale()}>
              已经改好过（清掉这些标记）
            </button>
          </p>
        )}
        {langOpen && (
          <div className={s.langGrid}>
            {/* 一册一块：语言名一行，名称 / 说明各自占满一行（两列并排会把输入框挤成条） */}
            {LANG_FIELDS.map((f) => (
              <div className={s.langBlock} key={f.labelKey}>
                <div className={s.langHead}>{f.title}</div>
                <div className={s.grid}>
                  <label className={`${s.field} ${s.wide}`}>
                    <span className={s.fieldLabel}>名称</span>
                    <input
                      className={staleList.includes(f.labelKey) ? `${s.txt} ${s.txtStale}` : s.txt}
                      value={draft[f.labelKey]}
                      aria-label={`${f.title} 名称 (${f.labelKey})`}
                      onChange={(e) => edit(f.labelKey, (d) => (d[f.labelKey] = e.target.value))}
                    />
                  </label>
                  <label className={`${s.field} ${s.wide}`}>
                    <span className={s.fieldLabel}>说明</span>
                    <input
                      className={staleList.includes(f.descKey) ? `${s.txt} ${s.txtStale}` : s.txt}
                      value={draft[f.descKey]}
                      aria-label={`${f.title} 说明 (${f.descKey})`}
                      onChange={(e) => edit(f.descKey, (d) => (d[f.descKey] = e.target.value))}
                    />
                  </label>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>类型与外观</div>
        <div className={s.grid}>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              值类型
            </span>
            <GroupPicker
              label="值类型"
              groups={[
                {
                  title: null,
                  options: (Object.keys(VT_LABEL) as ValueType[]).map((vt) => ({
                    value: vt,
                    label: VT_LABEL[vt],
                  })),
                },
              ]}
              value={draft.valueType}
              onChange={(v) => onValueType(v as ValueType)}
              width={220}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              控件
            </span>
            <GroupPicker
              label="控件"
              groups={[
                {
                  title: null,
                  options: controlOptions.map((uc) => ({ value: uc, label: UC_LABEL[uc] })),
                },
              ]}
              value={draft.uiComponent}
              onChange={(v) => edit('uiComponent', (d) => (d.uiComponent = v as UiComponent))}
              width={220}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              单位
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
              步进
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
              最小
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
              最大
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
              出厂默认
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
            可选项
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
                  <Icon name="chevronDown" size={12} className={ui.caretUp} />
                </button>
                <button
                  type="button"
                  className={s.orderBtn}
                  disabled={i >= draft.choices.length - 1}
                  aria-label={`第 ${i + 1} 条可选项下移`}
                  title="下移一位（分段按钮的先后）"
                  onClick={() => moveChoice(i, 1)}
                >
                  <Icon name="chevronDown" size={12} />
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
                    labelEn: '',
                    labelJa: '',
                    labelTw: '',
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
          归属与条件
        </div>
        <div className={s.grid}>
          {/*
            所属分组（2026-10-20，作者：「这个里面根本就没有看到它属于哪个 tab」）。
            **搬到别的页签 = 两份文件一起改**：注册表的 `[params.layout].sectionId`
            与 `layout_schema.toml` 的 items。后端在同一个事务里写，前端只管选。

            2026-11-01 换成自绘下拉（`SectionPicker`）：原生 `<select>` 里没地方
            挂「＋ 新增分组」，而页签只能当 `optgroup` 的死标签。新增的那一组先
            只进草稿 —— 建了又取消的话，数据里不会多出一个删不掉的空组。
          */}
          <div className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              所属分组
            </span>
            <SectionPicker
              groups={sectionGroups}
              value={draft.sectionId}
              onPick={onSection}
              onAdd={onAddSection}
              isPending={(id) => draft.newSection !== null && draft.newSection.id === id}
              onRename={onRenameSection}
              onMove={onMoveSection}
            />
          </div>
          {/*
            组内顺序：客户端按同一个 `layout.order` 排同一组里的参数 ——
            所以这不是「界面上的小偏好」，而是数据。后端保存时会把 items 一起重排。

            ★ 2026-11-02 作者：「我希望也是下拉菜单然后可以右侧常驻复用箭头，
            上下点击切换排序，就在下拉菜单里面操作」——
            原来是一格「↑ 第 2 / 3 位 ↓」的小控件，箭头只能挪**自己**一位；
            现在是一座下拉：**整组参数按当下的顺序排在里面，每行右边常驻上下箭头**，
            在下拉里直接调。触发器仍然显示「第 N / 共 M 位」，收起后照样读得出位置。

            选中行 = 这一条参数自己；点别的行**不改选中**（这一框只编辑这一条），
            点箭头只挪位置、不关面板 —— 连着调几位不用反复开合。
          */}
          <label className={s.field}>
            <span className={s.fieldLabel}>
              组内顺序
            </span>
            <GroupPicker
              label="组内顺序"
              groups={orderGroups}
              value={param.key}
              onChange={() => undefined}
              width={320}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              属于
            </span>
            <GroupPicker
              label="属于"
              groups={parentGroups}
              value={draft.parentKey}
              onChange={(v) => edit('parentKey', (d) => (d.parentKey = v))}
              width={420}
            />
          </label>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              前置条件
            </span>
            <GroupPicker
              label="前置条件"
              groups={whenGroups}
              value={draft.whenKey}
              onChange={onWhenKey}
              width={420}
            />
          </label>
          {whenParam !== null && (
            <>
              <label className={s.field}>
                <span className={s.fieldLabel}>
                  档位
                </span>
                <GroupPicker
                  label="档位"
                  groups={[
                    {
                      title: null,
                      options: (Object.keys(OP_LABEL) as ShowOp[]).map((op) => ({
                        value: op,
                        label: OP_LABEL[op],
                      })),
                    },
                  ]}
                  value={draft.whenOp}
                  onChange={(v) => edit('whenOp', (d) => (d.whenOp = v as ShowOp))}
                  width={200}
                />
              </label>
              <label className={s.field}>
                <span className={s.fieldLabel}>
                  值
                </span>
                {whenValueField}
              </label>
            </>
          )}
        </div>
        {/*
          「现在在」这一句只在**需要解释**的时候出现（2026-11-02 作者：「这些都重复了」）。
          正常情形里上面那格「所属分组」的触发器已经把「页签 · 分组」写得清清楚楚，
          再补一句「现在在：偏移 · 空间偏移」就是把同一个信息说两遍。
          只有两种情形值得说：① 新分组还没落盘；② 这一组在布局里查不到（不能选而不说）。
        */}
        {(herePending || hereSection === null) && (
          <p className={s.note}>
            {herePending && draft.newSection !== null
              ? `这一组还没落盘：保存时一并建立，取消保存就不会留下。`
              : '这一组在布局里找不到了 —— 保存前先在上面选一个分组。'}
          </p>
        )}
      </div>

      {/*
        适用机型 + 状态（2026-11-02 作者：「这些也不是同一行，而且全局弃用的应该在后面吧」）。

        原来「适用机型」自成一组、按钮按住整行宽；「状态」组里「全局弃用」那颗
        孤零零占左列、标签在左上角 —— 一行里左半边空着，右半边一个勾，看着不像
        同一张表。现在并成**一组**、同一个网格：

          第 1 行  适用机型 ｜ [全部机型（5 台全勾）]      〔跨到最后一列〕
          第 2 行  弃用机型 ｜ [全部机型（不限）]          〔跨到最后一列〕
          第 3 行  全局弃用 ｜ ☐                          〔左列，短字段就该短〕

        「全局弃用」放到最后：它是一句**总闸**（所有机型、所有版本都不许再改），
        摆在具体挑哪几台之前会让人先去勾它。顺序 = 先看细的，最后才是那个一勾全封的。
      */}
      <div className={s.group}>
        <div className={s.groupHead}>适用机型</div>
        <div className={s.grid}>
          {/*
            机型多、品牌多的时候一横排勾选框摆不下（作者的实测意见）——
            收进一枚按钮，点开才是模态框里的模态框：按品牌分组的树状多选，
            与套餐「选择版本」同一副长相，按住还能刷选。
          */}
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>适用机型</span>
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
          </label>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>弃用机型</span>
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
          <label className={s.field} title="勾上 = 所有机型、所有版本都不许再改它">
            <span className={s.fieldLabel}>全局弃用</span>
            <input
              className={s.check}
              type="checkbox"
              checked={draft.deprecated}
              aria-label="全局弃用 (deprecated)"
              onChange={(e) => edit('deprecated', (d) => (d.deprecated = e.target.checked))}
            />
          </label>
        </div>
        <p className={s.note}>
          摘掉某台机型，它身上已写的值会变成孤儿（检查页有提示）。
          两格叠加：勾上「全局弃用」= 所有机型都不许写；只挑「弃用机型」= 只这些机型不许写。
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
