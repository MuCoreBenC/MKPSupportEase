/*
 * 成员编辑框的草稿层：名字、最终成员序列、目标分组。
 *
 * 只管状态，不碰 DOM / IPC —— 与 `c14/sort/useSortDraft` 同一副骨架，
 * 状态转移抽成纯函数便于单测。确认之前一个字节都不写盘。
 */

import { useCallback, useRef, useState } from 'react'

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
  has: (key: string) => boolean
  /** 勾上 / 取消一条（左列的候选行） */
  toggle: (key: string) => void
  /** 移出本组，或取消一条待归入 */
  remove: (key: string) => void
  /** 把已移出的那一条放回来 */
  putBack: (key: string) => void
  /** 页签层换了目标分组：成员基准跟着换，之前的勾选作废 */
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
  }, [])

  return {
    label,
    setLabel,
    keys,
    targetSection,
    initialKeys,
    has: (key) => keys.includes(key),
    toggle,
    remove,
    putBack,
    retarget,
    reset,
    dirty:
      label !== initialRef.current.label ||
      targetSection !== initialRef.current.targetSection ||
      !sameKeys(keys, initialRef.current.keys),
  }
}
