/*
 * 三轴读数（可编辑）—— G06-8 轮从 AxisBar（v029 那套向导版）fork 出来的平面层版，
 * G06-10 起收编进 A44：「校准」tab 与向导就此分家 ——
 *
 *   向导（PageMachine）仍用 AxisBar（FLIP 补间 / 露出卡 / 退出层那套）；
 *   校准 tab 用这一颗：无 FLIP、读数是**可编辑**的——数字 + mm 住同一颗框、
 *   轴名 htmlFor 也能点聚焦，平时无边框（就是大字 + mm 的轻样子），hover / 聚焦
 *   才显一颗宽松的框（作者：「很小的框框，不够优雅」→ G06-8 定稿）。
 *
 * 留下的：打字缓冲（打到一半不被 toFixed 改写）、真相变了丢原文、Esc 还原、
 * Enter 失焦、改动绿点与「原 x.xx」chip、当前轴绿标 —— 一条不少。
 * 位置由调用方摆（HUD 台面 = 左下角；向导卡 = 横排）。
 *
 * 2026-10-11 第十一轮（作者：「这些很容易出问题……那个闪烁的效果不好看，还有，
 * 我没有进入编辑态就可以用滚轮或者箭头改变数值，很不好，我决定去掉双击编辑的」）：
 *
 *   双击编辑**整个撤掉** —— 回到**单击直进**：点数字（或轴名 / mm / 内衬）即聚焦进
 *   编辑态，下划线转**绿**（:focus-within），键盘 Tab 照旧。单击不再「闪两下」，
 *   `hintAt` / `unlocked` 两处状态一并删掉（形态仍是下划线：悬停灰线、聚焦绿线）。
 *
 *   滚轮 / 方向键**只在编辑态（聚焦）生效**：滚轮监听里补一道 `activeElement` 判断，
 *   没聚焦就放行走页面滚动、不 preventDefault、更不改数字 —— 手滑一下不会把已保存的
 *   值改掉（这正是作者要修的）。聚焦后滚轮仍按 0.05 步进；方向键/箭头自 2026-10-17 起
 *   也走 `step="0.05"`（原来 0.01，与滚轮两条口径）。
 */

import { useEffect, useRef, useState } from 'react'
import { useT } from '../../i18n'
import SkeletonA44 from '../home/Skeleton'
import type { AxisLimits } from '../../api/contract'
import { AXIS_ROWS, type Axes, type Axis, type AxisView } from './calibAxes'
import s from './CalibReadings.module.css'

/** 没传范围（不该发生，调用方都从 useCalibration 拿）时不设限 */
const NO_LIMITS: AxisLimits = { x: {}, y: {}, z: {} }

export default function CalibReadings({
  saved,
  draft,
  view,
  limits = NO_LIMITS,
  activeAxis,
  canType = false,
  onType,
  onRevert,
  onReset,
}: {
  saved: Axes | null
  draft: Axes | null
  view: AxisView
  /** 三轴的注册表范围（输入框 min/max 与滚轮步进夹取都按它；2026-10-20 起不吃 -50/50 硬编码） */
  limits?: AxisLimits
  /** 这一页正在校哪一轴：当前轴的轴名转绿，另外两轴保持灰（三个数一样大） */
  activeAxis?: 'z' | 'xy'
  /** 取到预设才有基准可改；只读时框不变灰（ disabled 会是第三种长相），只是不亮 */
  canType?: boolean
  onType?: (axis: Axis, raw: string) => void
  onRevert?: (axis: Axis) => void
  /** 点「原 x.xx」：这一轴整个退回已保存值（手输 + 板上点选一起清） */
  onReset?: (axis: Axis) => void
}) {
  const tr = useT()
  /*
   * 正在打字的那一轴 + 它的原始输入串。
   *
   * 不能直接把 draft 格式化回输入框：打到 "0.5" 时受控值会被 toFixed(2) 改写成 "0.50"，
   * 光标跳到末尾、再按 5 就成了 "0.505"。所以聚焦期间显示这一份原文，失焦就交回 draft。
   */
  const [buf, setBuf] = useState<{ k: Axis; text: string } | null>(null)
  const rootRef = useRef<HTMLDListElement>(null)

  /*
   * 输入框里那份原文只在"它就是当前真相"时有效。
   * 点板子 / 保存 / 放弃之后真相变了，框里不能还留着手输的旧串 ——
   * 只在原文能解析成数时判断（打到中间态 `-`、`0.` 时别抢走正在敲的东西）。
   */
  useEffect(() => {
    if (!buf || !draft) return
    const n = Number(buf.text)
    if (buf.text.trim() === '' || !Number.isFinite(n)) return
    if (n.toFixed(2) !== draft[buf.k].toFixed(2)) setBuf(null)
  }, [buf, draft])

  /*
   * 滚轮步进 0.05（作者 10-03：原生 number 滚轮按 step 走 0.01，太细）。
   * React 的 onWheel 挂的是被动监听，preventDefault 压不住浏览器原生的滚轮步进 ——
   * 手动挂非被动监听拦掉原生那一发，自己按 0.05 加减。
   * ★ 2026-10-17：`step` 也改成 0.05（见 `data-step` 那一行）—— 滚轮 / 箭头 / 键盘
   *   三者同一档，不再出现"滚一下 0.05、按一下 0.01"。
   *
   * **只在这一格聚焦（编辑态）时才动手**：没聚焦就直接 return，让滚轮照常滚页面 ——
   * 否则鼠标划过就把已保存的值改了（作者第十一轮要修的正是这个）。
   */
  useEffect(() => {
    const root = rootRef.current
    if (!root || !canType || !onType) return
    const onWheel = (e: WheelEvent) => {
      const input = e.target instanceof HTMLInputElement ? e.target : null
      if (!input || input.readOnly || input.type !== 'number') return
      if (document.activeElement !== input) return
      const k = input.dataset.axis
      if (k !== 'x' && k !== 'y' && k !== 'z') return
      e.preventDefault()
      const cur = Number(input.value)
      if (!Number.isFinite(cur)) return
      /* 夹取按注册表范围（原来硬编码 ±50，与注册表的 z ∈ [0,20] 打架） */
      const l = limits[k]
      const next = Math.min(l.max ?? Infinity, Math.max(l.min ?? -Infinity, cur + (e.deltaY < 0 ? 0.05 : -0.05)))
      onType(k, next.toFixed(2))
    }
    root.addEventListener('wheel', onWheel, { passive: false })
    return () => root.removeEventListener('wheel', onWheel)
  }, [canType, onType, limits])

  return (
    <dl className={s.readings} ref={rootRef}>
      {AXIS_ROWS.map(([label, k]) => {
        const changed = view === 'value' && saved !== null && draft !== null && draft[k] !== saved[k]
        const hasBase = view === 'value' && saved !== null
        const minor = activeAxis ? (activeAxis === 'z' ? k !== 'z' : k === 'z') : false
        const id = `calib-axis-${k}`
        const revertible = changed && canType
        return (
          <div className={s.axis} key={k} data-changed={changed} data-minor={minor}>
            <dt className={s.axisLabel}>
              <label htmlFor={id}>{label}</label>
              <i className={s.dot} data-on={changed} aria-hidden={!changed} />
              {/*
                「原 x.xx」：有基准就常显（只在改动后才冒出来，等于校准页没有这个角标
                —— 作者 10-03），改过的那一轴变成还原入口：点一下整轴退回已保存值。
                没改动 / 只读时 disabled —— 没有可还原的东西，也不该有按钮的手感。
              */}
              <button
                type="button"
                className={s.axisFrom}
                data-on={hasBase}
                data-act={changed ? 'revert' : undefined}
                disabled={!revertible}
               
                onClick={revertible ? () => onReset?.(k) : undefined}
              >
                {tr('calib.original', { value: saved ? saved[k].toFixed(2) : '0.00' })}
              </button>
            </dt>
            <dd className={s.axisValue}>
              {view === 'value' && draft ? (
                <label className={s.field} data-readonly={!canType}>
                  <input
                    id={id}
                    className={s.fieldInput}
                    type="number"
                    step="0.05"
                    min={limits[k].min}
                    max={limits[k].max}
                    inputMode="decimal"
                    aria-label={tr('calib.axisAria', { label })}
                    data-axis={k}
                    readOnly={!canType}
                    tabIndex={canType ? undefined : -1}
                    value={buf?.k === k ? buf.text : draft[k].toFixed(2)}
                    onChange={
                      canType
                        ? (e) => {
                            setBuf({ k, text: e.target.value })
                            onType?.(k, e.target.value)
                          }
                        : undefined
                    }
                    onBlur={canType ? () => setBuf(null) : undefined}
                    onKeyDown={
                      canType
                        ? (e) => {
                            if (e.key === 'Enter') e.currentTarget.blur()
                            if (e.key === 'Escape') {
                              setBuf(null)
                              onRevert?.(k)
                              e.currentTarget.blur()
                            }
                          }
                        : undefined
                    }
                  />
                  <span className={s.unit}>mm</span>
                </label>
              ) : view === 'loading' ? (
                <SkeletonA44 label={tr('calib.loadingPreset')} />
              ) : (
                <span className={s.axisBlank}>—</span>
              )}
            </dd>
          </div>
        )
      })}
    </dl>
  )
}
