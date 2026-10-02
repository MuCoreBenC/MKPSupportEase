import { isAppError } from './contract'

/**
 * 「这个口子还没接」的那个错。
 *
 * 故意不做成"界面上一条友好提示"—— 撤掉 mock 之后，没接的接口应该一眼看得见：
 * 构造时就往控制台打一条 error，然后照常作为异常抛出去，让调用栈把现场留下来。
 * 谁吞掉它、谁在哪个按钮上没接后端，看控制台就知道。
 *
 * 与 `AppError.code === 'NOT_IMPLEMENTED'` 是**两件事**，别混：
 * - `NotImplementedError`：**前端**这一侧就知道没接（bridge 里那一格空着），当场抛，不过 IPC；
 * - `AppError(NOT_IMPLEMENTED)`：过了 IPC，Rust 侧回一句"这个命令还没做"。
 * 页面按前者判"这一块本版未接入"，按后者走错误条。
 */
export class NotImplementedError extends Error {
  constructor(method: string) {
    super(`[API] 未实现的接口: ${method}`)
    this.name = 'NotImplementedError'
    console.error(this.message)
  }
}

/**
 * 把 reject 出来的东西说成**一句人话**。界面展示错误的唯一入口。
 *
 * 各页原先各写各的 `e instanceof Error ? e.message : String(e)` ——
 * 而跨 IPC 的错误是 [`AppError`]（普通对象，**不是 `Error` 实例**），
 * 于是结构化错误在提示条上变成一句 `[object Object]`
 * （2026-10-02 预设页「下载失败：[object Object]」抓到的就是它）。
 *
 * 顺序就是"谁的话最该给人看"：
 *
 *   `AppError`  → `message`（契约里写死：这一句可以直接显示给用户，中文）
 *   `Error`     → `message`（前端自己的错）
 *   字符串      → 原样（边界外直接抛字符串的那种）
 *   其余        → 尽力拼一个能看的（JSON 化；连这也做不了才退回 `String`）
 *
 * **不展开 `code` 与 `traceId`**：那是日志对账用的编号，不是给用户看的话 ——
 * 它们露出来的地方是控制台与后端日志，不是提示条。
 */
export function errorText(v: unknown): string {
  if (isAppError(v)) return v.message
  if (v instanceof Error) return v.message
  if (typeof v === 'string') return v
  try {
    return JSON.stringify(v) ?? String(v)
  } catch {
    return String(v)
  }
}
