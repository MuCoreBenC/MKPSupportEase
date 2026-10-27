/*
 * 机型多选器（2026-10-20 抽出来）。
 *
 * 原本它只长在「编辑定义」的**适用机型**那一格（模态框里的模态框：按品牌分组的
 * 勾选树、按住刷选）。这一轮「弃用机型」要的不是另一副界面 —— 作者的原话是
 * 「它其实也可以复用一个那个适用机型的那种，对吧？」。所以搬成共用件，
 * 三处共用同一副长相与同一套手势：
 *
 *   1. 参数级「适用机型」（machineFilter）
 *   2. 参数级「弃用机型」（deprecatedMachines）
 *   3. 每条可选项的「弃用机型」（choices[].deprecatedMachines）
 *
 * **它只管「挑哪几台」**：按钮长什么样、摘要怎么写由调用方决定（两处宽窄不同）。
 * 空选集的语义也由调用方定 —— 这一层只说 `[]`。
 *
 * Esc 只关这一层：`ModalC14` 自己按栈序处理，外面那框的草稿不动。
 */

import { useMemo, useRef } from 'react'

import type { MachineList } from '../api'
import ModalC14 from '../c14/ModalC14'
import { usePaintSelectC14 } from '../c14/usePaintSelect'
import c from '../c14.module.css'
import s from './MachinePicker.module.css'

interface Props {
  open: boolean
  /** 框标题，例如「选择弃用机型 · 擦料方式」 */
  title: string
  /** 标题下那句：**这个框选出来会怎样**（空选集的口径在这里说清） */
  subtitle: string
  /** 机型与品牌清单。`null` = 还没读到 —— 如实说一句，不给假清单 */
  machines: MachineList | null
  selected: string[]
  onChange: (next: string[]) => void
  onClose: () => void
}

/** 空选集的常量（钩子要一份稳定的输入，草稿还没起时用它） */
const NONE: string[] = []

export default function MachinePicker({
  open,
  title,
  subtitle,
  machines,
  selected,
  onChange,
  onClose,
}: Props) {
  const treeRef = useRef<HTMLDivElement>(null)

  const allIds = useMemo(() => (machines?.machines ?? []).map((m) => m.id), [machines])
  /* 钩子拿到的是 Set；每次渲染新建一份也行，但它只比对内容（见 hook 头注） */
  const picked = useMemo(() => new Set(selected.length > 0 ? selected : NONE), [selected])

  const changeRef = useRef(onChange)
  changeRef.current = onChange

  usePaintSelectC14({
    ref: treeRef,
    ids: allIds,
    selected: picked,
    onChange: (next) => changeRef.current([...next]),
    preview: (el, on) => {
      el.classList.toggle(String(c.pVrowOn), on)
      el.setAttribute('aria-checked', String(on))
      el.firstElementChild?.setAttribute('data-on', String(on))
    },
    enabled: open,
  })

  const byId = new Map((machines?.machines ?? []).map((m) => [m.id, m]))
  /* 品牌分组（多品牌时一组的树；没有品牌挂在谁下面的，兜进「未归品牌」别弄丢） */
  const brandedIds = new Set(machines?.brands.flatMap((b) => b.machines) ?? [])
  const orphanMachines = (machines?.machines ?? []).filter((m) => !brandedIds.has(m.id))

  const row = (id: string, display: string) => {
    const on = selected.includes(id)
    return (
      <button
        key={id}
        type="button"
        className={`${c.pVrow} ${on ? c.pVrowOn : ''}`}
        role="checkbox"
        aria-checked={on}
        /* 刷选引擎按它认行（usePaintSelectC14 的 attr） */
        data-sel={id}
        onClick={() => onChange(on ? selected.filter((x) => x !== id) : [...selected, id])}
      >
        <span className={c.pVmark} data-on={on} aria-hidden>
          <svg
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m2.6 6.3 2.4 2.4L9.5 3.9" />
          </svg>
        </span>
        <span className={c.pVtext}>
          <span className={c.pVname}>{display || id}</span>
          <span className={c.pVid}>{id}</span>
        </span>
      </button>
    )
  }

  return (
    <ModalC14
      open={open}
      title={title}
      subtitle={subtitle}
      size="md"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onClose}>
            完成（{selected.length} 台）
          </button>
        </>
      }
    >
      {machines === null ? (
        <p className={s.note}>机型清单还没读到 —— 关掉这个框再开一次就有了。</p>
      ) : (
        <div className={s.pickTree} ref={treeRef}>
          {machines.brands.map((b) => (
            <div key={b.id} className={c.pMg}>
              <div className={c.pMgName}>
                {b.name || b.id}
                <em>{b.machines.length}</em>
              </div>
              {b.machines.map((mid) =>
                row(mid, byId.get(mid)?.display ?? mid),
              )}
            </div>
          ))}
          {orphanMachines.length > 0 && (
            <div className={c.pMg}>
              <div className={c.pMgName}>
                未归品牌
                <em>{orphanMachines.length}</em>
              </div>
              {orphanMachines.map((m) => row(m.id, m.display))}
            </div>
          )}
        </div>
      )}
    </ModalC14>
  )
}


