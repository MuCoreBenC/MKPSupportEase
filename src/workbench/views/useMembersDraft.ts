/*
 * 成员编辑框的草稿层：名字、最终成员序列、目标分组、分组最终次序与待新建的分组。
 *
 * 只管状态，不碰 DOM / IPC —— 与 `c14/sort/useSortDraft` 同一副骨架，
 * 状态转移抽成纯函数便于单测。确认之前一个字节都不写盘。
 *
 * 名字那一格有两种归属（`nameFollowsTarget`）：
 *
 *   · 页签层 —— 它写的是这个页签的名，换目标分组与它无关；
 *   · 分组层 —— 它写的是**当前目标那个分组**的名，换目标它就跟着换。
 *
 * 打开时那一组不是这份草稿的身份，目标组才是：后端 `GroupScope::Section`
 * 要求 `id == sectionId`（`ui.presetdata.registry.351`），两边必须同一个。
 */

import { useCallback, useRef, useState } from 'react'

import type { NewSection } from '../api'

export interface MembersInitial {
  label: string
  /** 打开时这一组的成员（已按 `layout.order` 排好） */
  keys: string[]
  targetSection: string
  /** 打开时该页签下分组的最终次序 */
  groupOrder: string[]
  /** 已有分组现在的名（id → 名） */
  names: Record<string, string>
  /** 名字那一格跟着目标组走（分组层），还是就是页签名（页签层） */
  nameFollowsTarget: boolean
}

export interface MembersDraft {
  label: string
  setLabel: (v: string) => void
  /** 每个分组现在的名（含还没落盘的新组）—— 左列块标题与右列行名都读它 */
  names: Record<string, string>
  /** 最终成员序列 */
  keys: string[]
  targetSection: string
  /** 打开时就在这一组的成员 —— 右列据此分「已有」与「待归入」 */
  initialKeys: string[]
  /** 分组最终次序：列表、后端、mock 共用这一份 */
  groupOrder: string[]
  /** 框里待新建的分组，确认才随同一次提交落盘 */
  newSections: NewSection[]
  addSection: (section: NewSection) => void
  removeNewSection: (id: string) => void
  /** 列表给的是「移到第 to 位」，不是挪几格 */
  moveGroup: (from: number, to: number) => void
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

/** 把第 from 行挪到第 to 行；越界或同位置都原样退回，不产生半截状态 */
export function moveInOrder(order: string[], from: number, to: number): string[] {
  if (from === to || from < 0 || to < 0 || from >= order.length || to >= order.length) {
    return order
  }
  const next = [...order]
  const [moved] = next.splice(from, 1)
  if (moved === undefined) return order
  next.splice(to, 0, moved)
  return next
}

export const sameKeys = (a: string[], b: string[]): boolean =>
  a.length === b.length && a.every((k, i) => k === b[i])

/** 能装参数的分组：组件卡与整块卡不进候选，客户端那边是固定长好的 */
export function usableSections<T extends { component: string | null; standalone: boolean }>(
  sections: T[],
): T[] {
  return sections.filter((x) => x.component === null && !x.standalone)
}

export function useMembersDraft(initial: MembersInitial): MembersDraft {
  /* 打开那一刻的基准只认第一次 —— 「恢复」与脏判据都对着它 */
  const initialRef = useRef<MembersInitial>({
    ...initial,
    keys: [...initial.keys],
    groupOrder: [...initial.groupOrder],
    names: { ...initial.names },
  })
  const [label, setLabelState] = useState(initial.label)
  const [names, setNames] = useState<Record<string, string>>(() => ({ ...initial.names }))
  const [keys, setKeys] = useState<string[]>(() => [...initial.keys])
  const [initialKeys, setInitialKeys] = useState<string[]>(() => [...initial.keys])
  const [targetSection, setTargetSection] = useState(initial.targetSection)
  const [groupOrder, setGroupOrder] = useState<string[]>(() => [...initial.groupOrder])
  const [newSections, setNewSections] = useState<NewSection[]>([])

  const follows = initial.nameFollowsTarget
  const shownLabel = follows ? (names[targetSection] ?? '') : label

  const setLabel = useCallback(
    (v: string) => {
      if (!follows) {
        setLabelState(v)
        return
      }
      setNames((prev) => ({ ...prev, [targetSection]: v }))
    },
    [follows, targetSection],
  )
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
  const addSection = useCallback((section: NewSection) => {
    setNewSections((prev) => [...prev, section])
    setNames((prev) => ({ ...prev, [section.id]: section.label }))
    setGroupOrder((prev) =>
      prev.includes(section.id) ? prev : [...prev, section.id],
    )
  }, [])
  const removeNewSection = useCallback((id: string) => {
    setNewSections((prev) => prev.filter((x) => x.id !== id))
    setGroupOrder((prev) => prev.filter((x) => x !== id))
    setNames((prev) => {
      const next = { ...prev }
      delete next[id]
      return next
    })
  }, [])
  const moveGroup = useCallback(
    (from: number, to: number) =>
      setGroupOrder((prev) => moveInOrder(prev, from, to)),
    [],
  )
  const retarget = useCallback((sectionId: string, next: string[]) => {
    setTargetSection(sectionId)
    setKeys([...next])
    setInitialKeys([...next])
  }, [])
  const reset = useCallback(() => {
    setLabelState(initialRef.current.label)
    setNames({ ...initialRef.current.names })
    setKeys([...initialRef.current.keys])
    setInitialKeys([...initialRef.current.keys])
    setTargetSection(initialRef.current.targetSection)
    setGroupOrder([...initialRef.current.groupOrder])
    setNewSections([])
  }, [])

  return {
    label: shownLabel,
    setLabel,
    names,
    keys,
    targetSection,
    initialKeys,
    groupOrder,
    newSections,
    addSection,
    removeNewSection,
    moveGroup,
    has: (key) => keys.includes(key),
    toggle,
    remove,
    putBack,
    retarget,
    reset,
    dirty:
      (follows
        ? (names[targetSection] ?? '') !== (initialRef.current.names[targetSection] ?? '')
        : label !== initialRef.current.label) ||
      targetSection !== initialRef.current.targetSection ||
      !sameKeys(keys, initialRef.current.keys) ||
      !sameKeys(groupOrder, initialRef.current.groupOrder) ||
      newSections.length > 0,
  }
}
