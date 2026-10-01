/*
 * 提示条 —— 报告页与设置页共用的那一格。
 *
 * # 这是一条约定，不是这一页的装饰
 *
 * 预设页先立的规矩，这里把它抽成共用件：
 *
 *   · **一次只显示一条**，后来的替换前面的（不堆栈、不排队）
 *   · **× 手动关，不自动消失** —— 「契约里还没有这个方法：getReport」这种话
 *     自己消失了就等于没说过
 *   · 两种色：中性 = 真的做了某件事；琥珀 = **没接上**（不是「操作失败」，所以不上错误红）
 *
 * # 为什么抽出来而不是各抄一份
 *
 * 报告页有 3 处要说话（刷新 / 确认导出 / 清理缓存），设置页有 4 处
 * （搜索切片器 / 检查更新 / 回退版本 / 导出诊断包）—— 两页七处同一个行为。
 * 各抄一份就是两套 CSS 加两个 `useState`，哪天改「自动消失」会有一处忘记跟。
 *
 * **预设页那一份原样留着，一行不动**：它的提示条嵌在自己的 `.main` 布局里
 * （`flex-shrink: 0` + 那一条底边线跟着表格），换成这个件要动那一页的版面，
 * 而预设页这次不动。
 */

import type { Note } from './note'
import s from './NoteBar.module.css'

interface Props {
  note: Note | null
  onClose: () => void
}

export default function NoteBar({ note, onClose }: Props) {
  if (note === null) return null

  return (
    <p className={note.bad ? s.noteBad : s.note} role="status">
      {note.text}
      <button type="button" className={s.noteClose} aria-label="关闭这条提示" onClick={onClose}>
        ×
      </button>
    </p>
  )
}
