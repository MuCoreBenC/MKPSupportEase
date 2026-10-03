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
import type { ParamMetaEdit, ParamView, ShowOp, UiComponent, ValueType } from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import s from './ParamDefModal.module.css'

interface Props {
  /** 正在编辑哪条（`null` = 关着） */
  paramKey: string | null
  /** 那条参数现在的定义（来自注册表） */
  param: ParamView | null
  /** 全部参数 —— 「属于」「前置条件」的候选从这里来 */
  params: ParamView[]
  /** 机型清单 —— 「适用机型」勾选框的候选（来自整本） */
  machines: { id: string; display: string }[]
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
  deprecated: boolean
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
  deprecated: boolean
  choices: ChoiceDraft[]
  /** 空 = 全部机型 */
  machineFilter: string[]
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
  choices: p.choices.map((c) => ({ value: String(c.value), label: c.label, deprecated: c.deprecated })),
  machineFilter: [...p.machineFilter],
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
    /*
     * 可选项：字符串枚举交草稿里编的那份；其他类型**原样带回**（预设档不丢 ——
     * bool 的开/关叫法、数字的快捷档不在这一框的编辑范围里，但也不许弄丢）
     */
    choices:
      d.valueType === 'string'
        ? d.choices.map((c) => ({ label: c.label, value: c.value, deprecated: c.deprecated }))
        : param.choices,
    machineFilter: [...d.machineFilter],
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

export default function ParamDefModal({ paramKey, param, params, machines, onClose, onCommit }: Props) {
  const [form, setForm] = useState<FormState | null>(null)
  const [busy, setBusy] = useState(false)

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

  /** 改一格。tag 相同且连续（400ms 内）的改动并成一步撤销； redo 链作废 */
  const edit = (tag: string, fn: (d: Draft) => void) =>
    setForm((prev) => {
      if (prev === null) return prev
      const next = { ...prev.draft }
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

  const save = async (): Promise<void> => {
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
  saveRef.current = () => void save()

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
      subtitle={`写回 presets/registry/param_registry.toml 的 [[params]] —— 即时落盘；Cmd+Z 撤框里的改动；注册键 ${param.key}`}
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
              <label className={s.choiceDep}>
                <input
                  type="checkbox"
                  className={s.check}
                  checked={c.deprecated}
                  aria-label={`第 ${i + 1} 条可选项已弃用`}
                  onChange={(e) =>
                    edit(`choice.dep.${i}`, (d) => {
                      d.choices[i]!.deprecated = e.target.checked
                    })
                  }
                />
                弃用
              </label>
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
                  d.choices.push({ value: '', label: '', deprecated: false })
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
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>
          适用机型<em className={s.fieldKey}>machineFilter</em>
        </div>
        <div className={s.machines}>
          {machines.map((m) => {
            const on = draft.machineFilter.includes(m.id)
            return (
              <label key={m.id} className={s.machineChip}>
                <input
                  type="checkbox"
                  className={s.check}
                  checked={on}
                  aria-label={`适用于 ${m.display}`}
                  onChange={() =>
                    edit('machineFilter', (d) => {
                      d.machineFilter = on
                        ? d.machineFilter.filter((x) => x !== m.id)
                        : [...d.machineFilter, m.id]
                    })
                  }
                />
                {m.display}
                <span className={s.machineId}>{m.id}</span>
              </label>
            )
          })}
        </div>
        <p className={s.note}>一个都不勾 = 全部机型。摘掉某台机型的话，它身上已写的值会变成孤儿（检查页有提示）。</p>
      </div>

      <div className={s.group}>
        <div className={s.groupHead}>状态</div>
        <div className={s.grid}>
          <label className={s.field}>
            <span className={s.fieldLabel}>
              已弃用<em className={s.fieldKey}>deprecated</em>
            </span>
            <input
              className={s.check}
              type="checkbox"
              checked={draft.deprecated}
              aria-label="已弃用 (deprecated)"
              onChange={(e) => edit('deprecated', (d) => (d.deprecated = e.target.checked))}
            />
          </label>
        </div>
      </div>

      <p className={s.hint}>
        改的是「这条参数是什么」，不是某一台机型上的值。值类型变了的话，已有的机型/版本
        覆盖值不会跟着迁移；非字符串参数身上挂的可选项是预设档（bool 的开/关叫法、数字的
        快捷档），这里不给编辑、保存时原样保留。
      </p>
    </ModalC14>
  )
}
