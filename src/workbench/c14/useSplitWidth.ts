/*
 * 栏宽记忆（C14 useSplitWidth 原样移植）。
 *
 * 宽度偏好不该跟着组件卸载一起丢 —— 切页 / 刷新回来还是你拖的那个宽度
 * （满屏列宽跳回去比参数被重置更打眼）。所以落 localStorage，值一律 JSON 编码。
 *
 * 与原型的差别只有键名：那边带稿号 `mkp.c14.split`，这边是产品仓，
 * 起自己的命名空间 `mkp.wb.split`。
 */

import { useEffect, useState } from 'react'

const STORE_KEY = 'mkp.wb.split'

function readStore(): Record<string, number> {
  try {
    const raw = localStorage.getItem(STORE_KEY)
    const o: unknown = raw ? JSON.parse(raw) : null
    return o !== null && typeof o === 'object' ? (o as Record<string, number>) : {}
  } catch {
    return {}
  }
}

function writeStore(next: Record<string, number>) {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(next))
  } catch {
    /* 隐私模式 / 配额满：宽度记不住不算错误，无视 */
  }
}

const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), max)

/**
 * 一条栏宽的状态：初值读 localStorage（坏值 / 越界值一律退回或 clamp 回范围），
 * 变化时写回。调用方把 `w` 写进容器的 CSS 变量；拖动**过程中**分隔条直接覆写
 * 同一个变量，state 只在松手 / 键盘调整时变一次（理由见 SplitterC14 的文件头）。
 */
export function useSplitWidth(key: string, fallback: number, min: number, max: number) {
  const [w, setW] = useState(() => {
    const saved = readStore()[key]
    return clamp(typeof saved === 'number' && Number.isFinite(saved) ? saved : fallback, min, max)
  })

  useEffect(() => {
    writeStore({ ...readStore(), [key]: w })
  }, [key, w])

  return [w, setW] as const
}
