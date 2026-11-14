/*
 * **撤销的作用域**：当前这一屏「认哪几份」（2026-11-14）。
 *
 * 作者的原话：「最大的问题还是它可以撤销其他页面的，就比如说我点了我当前这个版本，
 * 我撤销它可以撤销到我其他版本的」。
 *
 * 撤销栈是全局一条（它记的是"我这一趟改了什么"），但**一次撤销只能落在当前这一份上**。
 * 判据两边：后端每条命令知道自己碰过哪些归属（`TopOp.owners`），页面这边知道自己
 * 在看谁 —— 这一格就是后者，一个模块级的当前值，跟 `draftState` / `deliveryState`
 * 同一套做法（页面上挂一个 `useScope`，App 那一层同步读）。
 *
 * # 为什么不做成 props
 *
 * 撤销的入口在外壳（顶栏、Cmd+Z、状态栏三处），而"我在看哪一版"在各页深处。
 * 一路传上去要在 App 里给四个页面各开一份 state，改一个页面就得动外壳 ——
 * 而作用域是**纯读取**的（不参与渲染），模块级一格最省。
 *
 * # 空的含义
 *
 * `[]` = 这一屏不认任何一份（或还没选）。后端约定：空作用域**不拦** ——
 * 拦了等于让"没得选"的那一屏永远撤不动。
 */

import { useEffect } from 'react'

/** 当前这一屏认的归属 token（`A1` / `A1:FAST261111` / `bundles` / `assets`） */
let current: string[] = []

/** 同步读（App 的撤销里用，不进渲染） */
export function scopeOf(): string[] {
  return current
}

/** 直接设（非组件路径 / 测试用） */
export function setScope(next: string[]): void {
  current = next.filter((s) => s !== '')
}

/**
 * 页面挂钩：挂载 / 依赖变化时把这一屏的作用域登记上，离开时清空。
 *
 * 传 `[]` 也照登（清空）—— "这一屏什么都不认" 与 "上一屏还在认" 是两件事，
 * 不清的话切到没选机型的页面还能撤上一屏的改动。
 */
export function useScope(tokens: string[]): void {
  const key = tokens.filter((s) => s !== '').join('|')
  useEffect(() => {
    setScope(key === '' ? [] : key.split('|'))
    return () => setScope([])
  }, [key])
}
