import { t } from '../i18n'
import { isAppError } from './contract'

/**
 * 「这个口子还没接」的那个错。
 *
 * 故意分两副面孔（A2 人话化）：
 *
 * - **给界面看的**（`message`）：人话 —— 说了「发生了什么」与「能干什么」
 *   （`hint` 给的时候用它；没给就老实说这一步这一版还没有）。原来那句
 *   "[API] 未实现的接口: downloadCatalogFile" 会原样出现在提示条上，
 *   测试报告的原话是"像是误入了开发者的聊天记录" —— 方法名 / API 字样
 *   不再进提示条；
 * - **给控制台看的**（`console.error`）：技术形式（方法名 + hint 原文）——
 *   撤掉 mock 之后，没接的接口要一眼看得见：谁在哪个按钮上没接后端，看控制台。
 *
 * 与 `AppError.code === 'NOT_IMPLEMENTED'` 是**两件事**，别混：
 * - `NotImplementedError`：**前端**这一侧就知道没接（bridge 里那一格空着），当场抛，不过 IPC；
 * - `AppError(NOT_IMPLEMENTED)`：过了 IPC，Rust 侧回一句"这个命令还没做"。
 * 页面按前者判"这一块本版未接入"，按后者走错误条。
 */
export class NotImplementedError extends Error {
  constructor(
    method: string,
    /** 给界面看的那句人话（发生了什么 + 能干什么）。缺省兜底也尽量不说黑话 */
    hint?: string,
  ) {
    super(hint ?? t('api.error.notImplementedHint'))
    this.name = 'NotImplementedError'
    console.error(`[API] 未实现的接口: ${method}${hint === undefined ? '' : ` —— ${hint}`}`)
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
 * **不展开 `traceId`**：那是日志对账用的编号，不是给用户看的话 —— 它露出来的地方是
 * 控制台与后端日志，不是提示条。
 *
 * ★★ 2026-10-19（作者：「不用错误码吗」）：`AppError` 的稳定码收在 `message` 的末尾
 *   （`…（E_DELIVERY_STALE_001）`）—— 由后端 `AppError::text` 拼好，这一层**不再带一次**
 *   （两份都带就成了「（码）（码）」）。`messageCode` 那一格留给需要单独拿码的人。
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

/** 界面拿得住的一条错：给人看的那句（已带码）+ 稳定码（`E_*_NNN`，可以没有）。 */
export interface UiError {
  /** `errorText(v)` 那一句 —— 直接显示，不要在上面再加工 */
  text: string
  /** 稳定码。缺 = 这一条后端还没盖码（那时别猜它是哪一种） */
  code: string | null
}

/**
 * `errorText` 那句 + 稳定码，一次取齐。
 *
 * 2026-10-19：界面要**按码决定给不给"怎么办"**（同一句"内容对不上"，
 * 目录过期要人重启、文件真坏要人重下 —— 只有码分得清），于是除了那句人话
 * 还得把码一起留住。
 */
export function uiErrorOf(v: unknown): UiError {
  return { text: errorText(v), code: isAppError(v) ? (v.messageCode ?? null) : null }
}
