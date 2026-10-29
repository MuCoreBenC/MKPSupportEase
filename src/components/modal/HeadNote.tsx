/*
 * **模态框里"一句提示"的唯一出口**（2026-10-17，作者第三次点名之后就收口了）。
 *
 * 作者原话（三句话拼起来的口径）：
 *   · 「这个文字提示不应该加在这个地方，应该加在标题后面」
 *   · 「因为你加在这的话，会导致我的模态框高度宽度变化，你能理解吗？」
 *   · 「把所有这些提示的，不管是什么的，都用同一套组件，同一套逻辑复用在标题后面显示」
 *
 * # 为什么必须搬进标题
 *
 * 框身（`.body`）是**按内容撑高**的：往里加一行字，框就长一截，下面那张事实表 / 参数列表
 * 跟着往下跳。回执槽（`ModalHeadNoteSlotContext`）那一格是
 * `flex: 1 1 auto; min-width: 0` + `nowrap + ellipsis` —— **字数不影响框的任何尺寸**，
 * 长了只是省略号（全文在 `title` 上）。
 *
 * # 三档只是颜色
 *
 *   `error` 红 —— 这件事没成（命令失败、读不出来）
 *   `note`  中性 —— 成了，这句只是补充（校准那趟的跑法、内核警告的汇总）
 *   `ok`    绿 —— 成了，而且这句是"好消息"
 *
 * 三档**共用同一个落点**：`modalHeadSlot.ts` 头注释里那条
 * 「三条状态共用这一格，只换颜色 —— 别为失败另开一处」就是这一条。
 *
 * # 不在模态框里的时候
 *
 * `slot === null`（这一屏还没进框 / 被单独渲染）⇒ 落回**原地**（`.fallback`），
 * 样子跟着 tone 走。同一份代码两种住处，调用方不用分情况。
 */

import { createPortal } from 'react-dom'
import { useText, type Text } from '../../i18n'
import { useModalHeadNoteSlot } from './modalHeadSlot'
import s from './HeadNote.module.css'

export type HeadNoteTone = 'error' | 'note' | 'ok'

/**
 * 标题右边那一格提示。
 *
 * `text` 收 [`Text`]：**这里摊开**（`useText`），所以调用方可以直接给 `msg(键, 变量)` 存着，
 * 换语言时这一格自己跟着翻；后端原话那种翻不了的照旧给字符串。
 */
export function HeadNote({ text, tone }: { text: Text; tone: HeadNoteTone }) {
  const slot = useModalHeadNoteSlot()
  const say = useText()
  const line = say(text)

  if (slot === null) {
    /* 没有槽 = 这一屏不在框里：照旧渲染在原处（不许"什么都没画"） */
    return (
      <p className={`${s.fallback} ${s[tone]}`} role={tone === 'error' ? 'alert' : undefined} title={line}>
        {line}
      </p>
    )
  }

  return createPortal(
    <span className={`${s.head} ${s[tone]}`} role={tone === 'error' ? 'alert' : undefined} title={line}>
      {line}
    </span>,
    slot,
  )
}
