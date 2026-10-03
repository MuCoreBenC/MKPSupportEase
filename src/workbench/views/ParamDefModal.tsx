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
 * # 与外壳的关系
 *
 * 即时落盘（`wb.setParamMeta`），不进参数草稿栈 —— 与机型尺寸 / 禁区那套一致：
 * 一次提交 = 一次写盘 = 回一份重读后的注册表。所以是「取消 / 确认保存」两颗
 * 按钮，不是改一格写一格（十几个格子一起动，中间态不该漂在文件上）。
 *
 * 校验的权威在后端（`registry::set_param_meta`）；这里先拦一道只是少挨一次错。
 */

import { useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { ParamMetaEdit, ParamView, RegistryView, ShowOp, ValueType } from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import s from './ParamDefModal.module.css'

interface Props {
  /** 正在编辑哪条（`null` = 关着） */
  paramKey: string | null
  /** 那条参数现在的定义（来自注册表，落盘成功后由新的那份顶替） */
  param: ParamView | null
  /** 全部参数 —— 「属于」「前置条件」的候选从这里来 */
  params: ParamView[]
  onClose: () => void
  /** 写盘成功后把重读的注册表交回页面 */
  onSaved: (next: RegistryView) => void
}

/** 值类型的中文注脚 —— 选项写原词（后端给什么就存什么），括号里说人话 */
const VT_LABEL: Record<ValueType, string> = {
  float: 'float · 小数',
  int: 'int · 整数',
  bool: 'bool · 开关',
  string: 'string · 文本',
}

const UC_LABEL: Record<ParamMetaEdit['uiComponent'], string> = {
  number: 'number · 步进框',
  switch: 'switch · 开关',
  segmented: 'segmented · 分段',
  select: 'select · 下拉',
  gcode: 'gcode · 代码',
}

const OP_LABEL: Record<ShowOp, string> = {
  eq: '等于',
  neq: '不等于',
  gt: '大于',
}

/**
 * 界面上的草稿。数字三格（min / max / step）与出厂默认存**原文** ——
 * 打字的中间态（"0."、"-"）不是合法数字，但必须是合法草稿；空串 = 没填。
 */
interface Draft {
  label: string
  desc: string
  unit: string
  valueType: ValueType
  uiComponent: ParamMetaEdit['uiComponent']
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
})

/** 草稿 → 后端载荷。归位口径与 `writeValue` 同一套：bool 归 bool、数字归数字 */
function payloadOf(d: Draft, params: ParamView[]): ParamMetaEdit {
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
  }
}

/** 第一条过不去的门。`null` = 可以存 */
function errorOf(d: Draft, param: ParamView): string | null {
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
  if (d.valueType === 'string' && param.choices.length > 0) {
    const ok = param.choices.some((c) => String(c.value) === d.defaultText)
    if (!ok) return '枚举的出厂默认得是选项之一'
  }
  if (d.whenKey !== '') {
    if (d.whenValue.trim() === '') return '前置条件的值不能是空的'
  }
  return null
}

export default function ParamDefModal({ paramKey, param, params, onClose, onSaved }: Props) {
  const [draft, setDraft] = useState<Draft | null>(null)
  const [busy, setBusy] = useState(false)

  /* 每次打开（或换了参数）都从「当前那份定义」重新起草 —— 上次取消的不该还留着 */
  useEffect(() => {
    setDraft(param === null ? null : initOf(param))
    setBusy(false)
  }, [paramKey, param])

  if (paramKey === null || param === null || draft === null) return null

  const edit = (fn: (d: Draft) => void) =>
    setDraft((prev) => {
      if (prev === null) return prev
      const next = { ...prev }
      fn(next)
      return next
    })

  /** 「属于」的候选：不是自己、也不是任何人的父项（层级只有两级，后端同判） */
  const parentsWithKids = new Set(
    params.filter((p) => p.parentKey !== null).map((p) => p.parentKey as string),
  )
  const parentCandidates = params.filter((p) => p.key !== param.key && !parentsWithKids.has(p.key))
  /** 「前置条件」的候选：任何不是自己的参数（实际上多半是父项或模式开关） */
  const whenCandidates = params.filter((p) => p.key !== param.key)
  const whenParam = whenCandidates.find((p) => p.key === draft.whenKey) ?? null

  const onValueType = (vt: ValueType) =>
    edit((d) => {
      d.valueType = vt
      const looksNum = d.defaultText.trim() !== '' && Number.isFinite(Number(d.defaultText))
      if (vt === 'float' || vt === 'int') {
        if (!looksNum) d.defaultText = '0'
      } else if (vt === 'bool') {
        d.defaultText = d.defaultText === 'true' ? 'true' : 'false'
      } else {
        const first = param.choices.find((c) => !c.deprecated) ?? param.choices[0]
        d.defaultText = first ? String(first.value) : ''
      }
    })

  const onWhenKey = (key: string) =>
    edit((d) => {
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

  const err = errorOf(draft, param)

  const save = async () => {
    setBusy(true)
    try {
      onSaved(await wb.setParamMeta(paramKey, payloadOf(draft, params)))
      toasts.push(`已写回 param_registry.toml · ${param.label}`)
      onClose()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      setBusy(false)
    }
  }

  /* 出厂默认的控件跟着值类型走 —— 「值的类型肯定是选的」（作者），不是文本框糊弄 */
  const defaultField = (() => {
    if (draft.valueType === 'bool') {
      return (
        <select
          className={s.sel}
          value={draft.defaultText}
          aria-label="出厂默认 (defaultValue)"
          onChange={(e) => edit((d) => (d.defaultText = e.target.value))}
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
          onChange={(e) => edit((d) => (d.defaultText = e.target.value))}
        />
      )
    }
    if (param.choices.length > 0) {
      return (
        <select
          className={s.sel}
          value={draft.defaultText}
          aria-label="出厂默认 (defaultValue)"
          onChange={(e) => edit((d) => (d.defaultText = e.target.value))}
        >
          {param.choices.map((c) => (
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
        value={draft.defaultText}
        aria-label="出厂默认 (defaultValue)"
        onChange={(e) => edit((d) => (d.defaultText = e.target.value))}
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
          onChange={(e) => edit((d) => (d.whenValue = e.target.value))}
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
          onChange={(e) => edit((d) => (d.whenValue = e.target.value))}
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
        onChange={(e) => edit((d) => (d.whenValue = e.target.value))}
      />
    )
  })()

  return (
    <ModalC14
      open
      title={`编辑定义 · ${param.label}`}
      subtitle={`写回 presets/registry/param_registry.toml 的 [[params]] —— 即时落盘，没有草稿；注册键 ${param.key}`}
      size="lg"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          {err !== null && <span className={s.warn}>{err}</span>}
          <span className={s.grow} />
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
              onChange={(e) => edit((d) => (d.label = e.target.value))}
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
              onChange={(e) => edit((d) => (d.desc = e.target.value))}
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
              onChange={(e) => edit((d) => (d.uiComponent = e.target.value as ParamMetaEdit['uiComponent']))}
            >
              {(Object.keys(UC_LABEL) as ParamMetaEdit['uiComponent'][]).map((uc) => (
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
              onChange={(e) => edit((d) => (d.unit = e.target.value))}
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
              onChange={(e) => edit((d) => (d.step = e.target.value))}
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
              onChange={(e) => edit((d) => (d.min = e.target.value))}
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
              onChange={(e) => edit((d) => (d.max = e.target.value))}
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

      <div className={s.group}>
        <div className={s.groupHead}>归属与条件</div>
        <div className={s.grid}>
          <label className={`${s.field} ${s.wide}`}>
            <span className={s.fieldLabel}>
              属于<em className={s.fieldKey}>parentKey</em>
            </span>
            <select
              className={s.sel}
              value={draft.parentKey}
              aria-label="属于 (parentKey)"
              onChange={(e) => edit((d) => (d.parentKey = e.target.value))}
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
                  onChange={(e) => edit((d) => (d.whenOp = e.target.value as ShowOp))}
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
              onChange={(e) => edit((d) => (d.deprecated = e.target.checked))}
            />
          </label>
        </div>
      </div>

      <p className={s.hint}>
        改的是「这条参数是什么」，不是某一台机型上的值。值类型变了的话，已有的机型/版本
        覆盖值不会跟着迁移；可选项（choices）这一格不在这里改 —— 那是另一张表。
      </p>
    </ModalC14>
  )
}
