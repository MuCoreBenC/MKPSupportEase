/*
 * 首页与校准页到处在用 Btn 与 Badge，所以连着搬。
 *
 * ★ 2026-10-14：`Compare` / `Revert` / `Field` / `NumberField` / `Segmented` 五个导出**删了**
 * （它们从搬进来那天起就没人 import —— 注释原来写着"搬迁不做裁剪"，但那是**没人用的代码**，
 * 留着只会让人以为界面上还有这些件）。
 * 连带删掉的是 `ui.module.css` 里只服务它们的那些类（`.seg` / `.field*` / `.num*` /
 * `.compare*` / `.revert` / `.help` / `.table` / `.rowAction` / `.inlineProgress` /
 * `.bar*` / `.empty` / `.stepRail` / `.step*` / `.scaleBar` / `.tick` / `.grid` / `.cell` / `.modal`）。
 *
 * 同时删掉的还有 `Progress.tsx`（整文件，没人 import）与 `shared/OriginChip.tsx`（整文件 + 它的 css）。
 */

import type { ReactNode } from 'react'
import s from './ui.module.css'

export function Badge({
  children,
  tone = 'muted',
}: {
  children: ReactNode
  tone?: 'new' | 'rec' | 'hot' | 'muted' | 'danger'
}) {
  return (
    <span className={s.badge} data-tone={tone}>
      {children}
    </span>
  )
}

export function Compare({ from, to }: { from: string; to: string }) {
  const changed = from !== to
  return (
    <span className={s.compare}>
      {from}
      <span aria-hidden="true">→</span>
      <b className={changed ? s.compareNew : undefined}>{to}</b>
    </span>
  )
}

export function Revert({ original, onClick }: { original: string; onClick: () => void }) {
  return (
    <button type="button" className={s.revert} onClick={onClick}>
      <span aria-hidden="true">↰</span>
      {original}
    </button>
  )
}

export function Field({
  label,
  paramKey,
  help,
  children,
  extra,
}: {
  label: string
  paramKey?: string
  help?: string
  children: ReactNode
  extra?: ReactNode
}) {
  return (
    <div className={s.field}>
      <div className={s.fieldMain}>
        <span className={s.fieldLabel}>
          {label}
          {help && (
            <i className={s.help}>
              ?
            </i>
          )}
          {extra}
        </span>
        {paramKey && <span className={s.fieldKey}>{paramKey}</span>}
      </div>
      {children}
    </div>
  )
}

export function NumberField({
  value,
  unit,
  onChange,
}: {
  value: string
  unit?: string
  onChange: (v: string) => void
}) {
  return (
    <span className={s.num}>
      <input
        className={s.numInput}
        type="text"
        inputMode="decimal"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <span className={s.unit}>{unit ?? ''}</span>
    </span>
  )
}

export function Btn({
  children,
  variant = 'default',
  size,
  disabled,
  onClick,
  className,
  title,
}: {
  children: ReactNode
  variant?: 'default' | 'accent' | 'primary' | 'ghost' | 'danger'
  /**
   * 尺寸（不换皮）。
   *
   * `sm` = 页头那种一行小胶囊：24px 高 / 左右 6px / 12px 字 —— 与工艺页工具条右端
   * 那两枚开关（`WindowGadgets` 的 `.pill`）同一组数（见 `ui.module.css` 的 `.btnSm`）。
   * 只在"顶条"这种地方用；框里、页脚里那些正经动作仍是不传（默认那一档）。
   */
  size?: 'sm'
  disabled?: boolean
  onClick?: () => void
  /**
   * **额外**的类（外观仍是 `variant` 那一套）。
   *
   * 2026-10-17 加：只该由布局方用来表达"这一颗允许折行 / 收缩"。
   * 后处理那一问的两颗长按钮就是这么排的（`PostProcessModal` 的 `.choiceBtn`）——
   * 全局 `button { white-space: nowrap }` 让它们既不能缩也不能折，中号框里第二颗直接被
   * 挤出框外。**别再为这种需求另抄一套按钮皮**（抄出来的那套没有深色分支，
   * 深色下主按钮就不是全站那颗实心绿了）。
   */
  className?: string
  /** 悬停提示（`hint` / 全文这类"看得见但占地方"的补充说明挂这儿） */
  title?: string
}) {
  const cls = [
    s.btn,
    size === 'sm' ? s.btnSm : '',
    variant === 'accent' ? s.btnAccent : '',
    variant === 'primary' ? s.btnPrimary : '',
    variant === 'ghost' ? s.btnGhost : '',
    variant === 'danger' ? s.btnDanger : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ')
  return (
    <button type="button" className={cls} disabled={disabled} onClick={onClick} title={title}>
      {children}
    </button>
  )
}
