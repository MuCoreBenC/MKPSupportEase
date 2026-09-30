/*
 * 一行参数的右半边：四类控件。
 *
 * 分类与显示规则全部照上游（对着 BBS 截图逐行核出来的），四条最容易做错的：
 *
 *   1. **复选框用 BBS 的两张原件**，不自绘。`check_on` 是 #00AE42 实心方块 +
 *      白色折线勾，那个勾**本来就不居中**（(4.4,8.5) → (8.3,11.9) → (14.8,6.0)，
 *      右臂长、整体偏右上）。自绘过一次，形状和颜色都不对。图标缺了才回退自绘。
 *   2. **只有 coInt/coInts 才画上下箭头**（BBS 用 SpinCtrl）—— 墙层数、壳体层数有；
 *      壳体厚度、密度（coFloat/coPercent，用 TextCtrl）没有。
 *   3. **末尾 `.0` 抹掉**（链里存 "1.0"，BBS 经 double_to_string 显示 1）；
 *      **coPercent 的 % 不重复显示**（单位栏已经是 %），但 coFloatOrPercent 要留着 ——
 *      那个 % 是「按百分比解释」的标志，它的单位栏写「mm 或 %」。
 *   4. **带图案图标的枚举不画 ∨** —— 图标顶替了箭头那一格（顶面图案/底面图案/稀疏填充图案
 *      都没有 ∨，接缝位置、墙生成器有）。
 *
 * 枚举菜单为什么「只有一项也建」：`gui_type = f_enum_open` 是「能输数字也能选特殊值」的
 * 混合控件（`prime_tower_brim_width` 的 -1=Auto），按 >1 过滤会让这一行既不像数值框
 * 也点不开。
 */

import { useEffect, useRef, useState } from 'react'
import BbsIcon from './BbsIcon'
import { norm, truthy } from './bbsMerge'
import { optsFor } from './bbsSupportStyle'
import type { BbsParamMeta, BbsValue, BbsValues } from './bbsTypes'
import s from './BbsControl.module.css'

interface Props {
  paramKey: string
  meta: BbsParamMeta
  value: BbsValue | undefined
  /** 整份配置的值 —— support_style 的候选集要按 support_type 现算 */
  values: BbsValues
  variantIdx: number
  icons: Record<string, string>
  /** toggle_field 为假：行还在，控件置灰 */
  greyed: boolean
  /** 展示模式：连下拉都不给展开 */
  readOnly: boolean
  onChange: (key: string, value: string) => void
}

const isLongText = (str: string) => str.length > 24 || str.includes('\n')

export default function BbsControl({
  paramKey,
  meta,
  value,
  values,
  variantIdx,
  icons,
  greyed,
  readOnly,
  onChange,
}: Props) {
  const [open, setOpen] = useState(false)
  const boxRef = useRef<HTMLSpanElement>(null)

  /* 点别处就收起来。菜单是 absolute 挂在控件上的，不 portal —— 它跟着行一起滚 */
  useEffect(() => {
    if (!open) return
    const onDown = (ev: MouseEvent) => {
      if (!boxRef.current?.contains(ev.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [open])

  const raw = norm(value, variantIdx)
  const text = raw === undefined || raw === null ? '' : String(raw)

  /* ① 布尔 */
  if (meta.type === 'bool') {
    const on = truthy(value, variantIdx)
    const name = greyed
      ? (on ? 'check_on_disabled' : 'check_off_disabled')
      : (on ? 'check_on' : 'check_off')
    const svg = icons[name] ?? icons[on ? 'check_on' : 'check_off']
    const cls = [s.chk, on ? s.chkOn : '', svg ? '' : s.chkFallback].filter(Boolean).join(' ')
    return (
      <span className={s.val}>
        <button
          type="button"
          className={cls}
          disabled={readOnly || greyed}
          aria-pressed={on}
          aria-label={on ? '开' : '关'}
          onClick={() => onChange(paramKey, on ? '0' : '1')}
        >
          <BbsIcon name={svg ? name : null} icons={icons} className={s.chkIco} />
        </button>
      </span>
    )
  }

  /* ② 长文本（自定义 G-code 那些）：只显示首行摘要 */
  if (isLongText(text)) {
    return (
      <span className={s.val}>
        <span className={s.textline} title={text}>
          {text.split('\n')[0] || '（空）'}
        </span>
      </span>
    )
  }

  /* ③ 枚举 */
  const opts = optsFor(paramKey, meta, values, variantIdx)
  if (meta.type === 'sel' || opts.length) {
    const hit = opts.find((o) => o.value === text)
    const withIcon = Boolean(hit?.icon && icons[hit.icon])
    const canOpen = !readOnly && !greyed && opts.length >= 1
    return (
      <span className={s.val} ref={boxRef}>
        <button
          type="button"
          className={`${s.sel}${open ? ` ${s.selOpen}` : ''}`}
          title={text}
          disabled={!canOpen}
          onClick={() => setOpen((v) => !v)}
        >
          {withIcon
            ? <BbsIcon name={hit?.icon} icons={icons} className={s.eico} />
            : <BbsIcon name="drop_down" icons={icons} className={s.chev} fallback="∨" />}
          {/* 查不到就露原值，不猜 */}
          <span className={s.selText}>{hit ? (hit.label_zh ?? hit.value) : text}</span>
        </button>
        {open && (
          <span className={s.menu} role="listbox">
            {opts.map((o) => (
              <button
                key={o.value}
                type="button"
                role="option"
                aria-selected={o.value === text}
                className={`${s.menuItem}${o.value === text ? ` ${s.menuItemOn}` : ''}`}
                title={o.value}
                onClick={() => {
                  setOpen(false)
                  /* 写进去的是 enum_values 里的原始字符串（BBS json 里就是这个），不是中文 label */
                  onChange(paramKey, o.value)
                }}
              >
                {o.icon && icons[o.icon] && (
                  <BbsIcon name={o.icon} icons={icons} className={s.eico} />
                )}
                <span>{o.label_zh ?? o.value}</span>
              </button>
            ))}
          </span>
        )}
      </span>
    )
  }

  /* ④ 数值 / 短文本 */
  const co = meta.co ?? ''
  const isInt = /^coInts?$/.test(co)
  let shown = /^coPercents?$/.test(co) ? text.replace(/%$/, '') : text
  if (/^-?\d+\.\d+$/.test(shown)) shown = String(Number(shown))
  const unit = meta.unit?.zh || meta.unit?.en || ''

  return (
    <span className={s.val}>
      <span className={s.num}>
        {isInt && (
          <span className={s.arrows}>
            <BbsIcon name="spin_inc" icons={icons} />
            <BbsIcon name="spin_dec" icons={icons} />
          </span>
        )}
        {shown}
      </span>
      {unit && <span className={s.unit}>{unit}</span>}
    </span>
  )
}
