/*
 * 成员编辑框的草稿层：名字、最终成员序列、目标分组、待新建的分组与换位。
 *
 * 只管状态，不碰 DOM / IPC —— 与 `c14/sort/useSortDraft` 同一副骨架，
 * 状态转移抽成纯函数便于单测。确认之前一个字节都不写盘。
 */

import { useCallback, useRef, useState } from 'react'

import type { NewSection, SectionMoveStep } from '../api'

export interface MembersInitial {
  label: string
  /** 打开时这一组的成员（已按 `layout.order` 排好） */
  keys: string[]
  targetSection: string
}

export interface MembersDraft {
  label: string
  setLabel: (v: string) => void
  /** 最终成员序列 */
  keys: string[]
  targetSection: string
  /** 打开时就在这一组的成员 —— 右列据此分「已有」与「待归入」 */
  initialKeys: string[]
  /** 框里待新建的分组，确认才随同一次提交落盘 */
  newSections: NewSection[]
  addSection: (section: NewSection) => void
  /** 分组在自己页签里的净位移 */
  moves: SectionMoveStep[]
  moveSection: (id: string, dir: -1 | 1) => void
  has: (key: string) => boolean
  /** 勾上 / 取消一条（左列的候选行） */
  toggle: (key: string) => void
  /** 移出本组，或取消一条待归入 */
  remove: (key: string) => void
  /** 把已移出的那一条放回来 */
  putBack: (key: string) => void
  /** 换了目标分组：成员基准跟着换，之前的勾选作废 */
  retarget: (sectionId: string, keys: string[]) => void
  reset: () => void
  dirty: boolean
}

export function toggleMember(keys: string[], key: string): string[] {
  return keys.includes(key) ? keys.filter((k) => k !== key) : [...keys, key]
}

export function removeMember(keys: string[], key: string): string[] {
  return keys.filter((k) => k !== key)
}

export const sameKeys = (a: string[], b: string[]): boolean =>
  a.length === b.length && a.every((k, i) => k === b[i])

/** 能装参数的分组：组件卡与整块卡不进候选，客户端那边是固定长好的 */
export function usableSections<T extends { component: string | null; standalone: boolean }>(
  sections: T[],
): T[] {
  return sections.filter((x) => x.component === null && !x.standalone)
}

/** 把净位移摊回分组顺序：界面看到的位置与后端一格一格走完的结果同一份 */
export function applyMoves<T extends { id: string }>(
  rows: T[],
  moves: SectionMoveStep[],
): T[] {
  const out = [...rows]
  for (const m of moves) {
    const dir = Math.sign(m.steps)
    if (dir === 0) continue
    let at = out.findIndex((r) => r.id === m.sectionId)
    if (at < 0) continue
    for (let k = 0; k < Math.min(Math.abs(m.steps), out.length); k += 1) {
      const to = at + dir
      if (to < 0 || to >= out.length) break
      const [moved] = out.splice(at, 1)
      if (moved === undefined) break
      out.splice(to, 0, moved)
      at = to
    }
  }
  return out
}

export function useMembersDraft(initial: MembersInitial): MembersDraft {
  /* 打开那一刻的基准只认第一次 —— 「恢复」与脏判据都对着它 */
  const initialRef = useRef<MembersInitial>({
    ...initial,
    keys: [...initial.keys],
  })
  const [label, setLabel] = useState(initial.label)
  const [keys, setKeys] = useState<string[]>(() => [...initial.keys])
  const [initialKeys, setInitialKeys] = useState<string[]>(() => [...initial.keys])
  const [targetSection, setTargetSection] = useState(initial.targetSection)
  const [newSections, setNewSections] = useState<NewSection[]>([])
  const [moves, setMoves] = useState<SectionMoveStep[]>([])

  const toggle = useCallback(
    (key: string) => setKeys((prev) => toggleMember(prev, key)),
    [],
  )
  const remove = useCallback(
    (key: string) => setKeys((prev) => removeMember(prev, key)),
    [],
  )
  const putBack = useCallback(
    (key: string) =>
      setKeys((prev) => (prev.includes(key) ? prev : [...prev, key])),
    [],
  )
  const addSection = useCallback(
    (section: NewSection) => setNewSections((prev) => [...prev, section]),
    [],
  )
  const moveSection = useCallback((id: string, dir: -1 | 1) => {
    setMoves((prev) => {
      const hit = prev.find((m) => m.sectionId === id)
      if (hit === undefined) return [...prev, { sectionId: id, steps: dir }]
      return prev.map((m) =>
        m.sectionId === id ? { ...m, steps: m.steps + dir } : m,
      )
    })
  }, [])
  const retarget = useCallback((sectionId: string, next: string[]) => {
    setTargetSection(sectionId)
    setKeys([...next])
    setInitialKeys([...next])
  }, [])
  const reset = useCallback(() => {
    setLabel(initialRef.current.label)
    setKeys([...initialRef.current.keys])
    setInitialKeys([...initialRef.current.keys])
    setTargetSection(initialRef.current.targetSection)
    setNewSections([])
    setMoves([])
  }, [])

  return {
    label,
    setLabel,
    keys,
    targetSection,
    initialKeys,
    newSections,
    addSection,
    moves,
    moveSection,
    has: (key) => keys.includes(key),
    toggle,
    remove,
    putBack,
    retarget,
    reset,
    dirty:
      label !== initialRef.current.label ||
      targetSection !== initialRef.current.targetSection ||
      !sameKeys(keys, initialRef.current.keys) ||
      newSections.length > 0 ||
      moves.some((m) => m.steps !== 0),
  }
}
