/*
 * Toast（C14 原样移植）。
 *
 * 纯 UI 状态，不进任何业务 state —— 它不参与后端数据，也不该触发页面重取。
 * 动作挂**一个**（用来做「撤销」）；挂了动作的停留 5 秒（比普通的 3 秒长一点，
 * 给看清和抬手的时间）。
 *
 * 与原型的差别只有文案约定：原型写真操作也要缀「（演示）」because 它不落盘；
 * 这里写的是真盘，不加后缀 —— 成功文案说什么就是什么。
 */

export interface ToastAction {
  label: string
  run: () => void
}

export interface ToastItem {
  id: number
  text: string
  action?: ToastAction
}

let items: ToastItem[] = []
let seq = 0
const listeners = new Set<() => void>()

function emit() {
  for (const fn of listeners) fn()
}

function drop(id: number) {
  items = items.filter((t) => t.id !== id)
  emit()
}

export const toasts = {
  subscribe(fn: () => void): () => void {
    listeners.add(fn)
    return () => listeners.delete(fn)
  },
  get(): ToastItem[] {
    return items
  },
  push(text: string, action?: ToastAction): void {
    const id = ++seq
    items = [...items, { id, text, action }].slice(-3)
    emit()
    setTimeout(() => drop(id), action ? 5000 : 3000)
  },
  /** 点完动作就把这条收走 —— 留着一个已经用过的「撤销」会让人再点一次 */
  close(id: number): void {
    drop(id)
  },
}
