/*
 * 界面动作记录（进 `ui.log`）。
 *
 * # 记什么
 *
 * 只记**有语义的动作**：按钮点击、切页、开关翻转、弹框开关、每次后端命令及结果。
 * hover / 滚动 / 鼠标移动一概不记 —— 那份量级会把真因埋掉，日志也就没法看了。
 *
 * # 三条硬约定
 *
 * 1. **绝不挡事**：入队是同步 O(1)，上报失败/后端没装全部吞掉。日志写不出去，
 *    界面照常用 —— 与 Rust 侧「日志装不上不阻断启动」同一个立场。
 * 2. **批量**：400 ms 或攒满 64 条发一次，一次点击打出来的七八个命令走同一个批。
 *    每次单发会在 Rust 侧变成几百次小 write，纯浪费。
 * 3. **不进参数值**：命令只记方法名，不记参数（参数可能是预设名、路径、甚至地址）。
 *
 * 浏览器（`npm run dev`）里没有 `invoke`：整份静默关掉，界面代码不用判断自己跑在哪。
 */
import { invoke } from '@tauri-apps/api/core'

import type { UiEvent } from '../../api/contract'

/** 队列上限：超了丢最旧的（日志不该因为写不出去而吃内存） */
const MAX_QUEUE = 200
/** 一批最多带几条 */
const BATCH = 64
/** 攒多久发一次 */
const FLUSH_MS = 400
/** 标签/细节的截断长度 */
const MAX_LABEL = 24
const MAX_DETAIL = 512

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const queue: UiEvent[] = []
let timer: number | null = null
let flushing = false
let dropped = 0
let currentPage = ''

/** 当前页签（切页时由外壳更新）。每一条事件都带上它，日志里才分得清"在哪一页点的" */
export function setUiPage(page: string): void {
  currentPage = page
}

function cut(text: string, max: number): string {
  const one = text.replace(/\s+/g, ' ').trim()
  return one.length <= max ? one : `${one.slice(0, max)}…`
}

function detailOf(detail: Record<string, unknown> | string | undefined): string | undefined {
  if (detail === undefined) return undefined
  if (typeof detail === 'string') return cut(detail, MAX_DETAIL)
  try {
    return cut(JSON.stringify(detail), MAX_DETAIL)
  } catch {
    return undefined
  }
}

/** 记一条。不 await、不抛错。 */
export function track(event: string, detail?: Record<string, unknown> | string): void {
  if (!inTauri) return
  if (queue.length >= MAX_QUEUE) {
    queue.shift()
    dropped += 1
  }
  queue.push({
    event: cut(event, 48),
    page: currentPage === '' ? undefined : currentPage,
    detail: detailOf(detail),
  })
  if (queue.length >= BATCH) {
    void flush()
    return
  }
  if (timer === null) {
    timer = window.setTimeout(() => {
      timer = null
      void flush()
    }, FLUSH_MS)
  }
}

/** 把队列发出去。失败一律吞掉（下一次还有机会，最坏也就是这段日志丢了）。 */
export async function flush(): Promise<void> {
  if (!inTauri || flushing || queue.length === 0) return
  flushing = true
  const lost = dropped
  dropped = 0
  const batch = queue.splice(0, BATCH)
  /* 丢过就留一条账：日志里少了多少条，是"这份日志完不完整"的唯一线索 */
  if (lost > 0) {
    batch.unshift({ event: 'ui.dropped', page: currentPage === '' ? undefined : currentPage, detail: `n=${lost}` })
  }
  try {
    await invoke('log_ui_events', { events: batch })
  } catch {
    dropped += batch.length
  } finally {
    flushing = false
  }
  /* 攒的比一批多：接着发，不必等下一次定时器 */
  if (queue.length > 0) void flush()
}

/** 这条元素在日志里的名字：优先 `data-track`，其次 `aria-label`，再次可见文案 */
function labelOf(el: HTMLElement): string {
  const marked = el.dataset.track
  if (marked !== undefined && marked !== '') return cut(marked, MAX_LABEL)
  const aria = el.getAttribute('aria-label')
  if (aria !== null && aria.trim() !== '') return cut(aria, MAX_LABEL)
  const text = el.textContent ?? ''
  if (text.trim() !== '') return cut(text, MAX_LABEL)
  const input = el as HTMLInputElement
  if (input.type !== undefined && input.type !== '') return cut(input.type, MAX_LABEL)
  return el.tagName.toLowerCase()
}

/* 认得出"这是一个可点的东西"的选择器。兜底要宽（任何按钮都算），但只认 click */
const CLICKABLE =
  '[data-track],button,a,[role="button"],[role="tab"],[role="switch"],label,summary,input,select,textarea'

/**
 * 装上全局点击捕获（捕获阶段：先于业务处理器拿到那一下，业务自己 stopPropagation 也拦不住）。
 *
 * 返回卸载函数（判据与热更新用；正式界面只装一次）。
 */
export function installUiTracking(): () => void {
  if (!inTauri || typeof document === 'undefined') return () => {}

  const onClick = (e: MouseEvent): void => {
    const target = e.target instanceof Element ? e.target : null
    if (target === null) return
    const el = target.closest(CLICKABLE)
    if (!(el instanceof HTMLElement)) return
    const section = el.closest('[data-sec]')?.getAttribute('id') ?? undefined
    const state =
      el.getAttribute('role') === 'switch' || el.getAttribute('role') === 'checkbox'
        ? el.getAttribute('aria-checked')
        : null
    track('click', {
      el: el.tagName.toLowerCase(),
      name: labelOf(el),
      section,
      ...(state === null ? {} : { checked: state }),
    })
  }

  const onHide = (): void => {
    if (document.visibilityState === 'hidden') void flush()
  }

  document.addEventListener('click', onClick, true)
  document.addEventListener('visibilitychange', onHide)
  window.addEventListener('pagehide', onHide)
  return () => {
    document.removeEventListener('click', onClick, true)
    document.removeEventListener('visibilitychange', onHide)
    window.removeEventListener('pagehide', onHide)
  }
}
