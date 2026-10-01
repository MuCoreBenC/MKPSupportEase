/*
 * 把搜索命中的那截字加个底。
 *
 * 作者：「我希望搜索出来的可以高亮那个字，就是加上背景的那种，如果是数字也可以」，
 * 第二张截图是搜「t」得到「X 轴偏移」那一行展开之后的样子 —— 唯一命中处在
 * 「字段名 offset」里，不展开根本看不出这一行为什么被搜出来。
 *
 * 谁调它：行名（`ParamRow`）、展开后的每一格（说明 / 取值范围 / 字段名 /
 * 已保存值 / 数据包里的值 / 作用域）、卡片标题与那几个分类小字（`ParamCard`）。
 * 「命中就加底」这一条覆盖他说的两种情形：文字（`offset` 里的 `t`）与数字
 * （值里的 `-1`、范围里的 `50`）走的是同一段代码，不分开判类型。
 *
 * 三条规矩：
 *   1. 比对**不分大小写** —— 与搜索的匹配规则同一套（页面那边也是 `toLowerCase`
 *      之后 `includes`）；命中处照原文返回，不改字。
 *   2. 没词 / 没命中 → 原样返回字符串，**不多包一层**：DOM 干净，
 *      `textContent` 也不会被拆成一串小片段。
 *   3. 同一个词出现多次，每一处都加底（`while` 往后走，不只看第一处）。
 */

import type { ReactNode } from 'react'
import s from './highlight.module.css'

interface Props {
  /** 原文 */
  text: string
  /** 搜索词；空串 = 不高亮 */
  query: string
}

export default function Highlight({ text, query }: Props) {
  const q = query.trim().toLowerCase()
  if (q === '') return <>{text}</>

  const hay = text.toLowerCase()
  if (!hay.includes(q)) return <>{text}</>

  const parts: ReactNode[] = []
  let at = 0
  let found = hay.indexOf(q, at)
  while (found !== -1) {
    if (found > at) parts.push(text.slice(at, found))
    /* key 用起始下标：同一个词在同一段里出现多次时它是唯一的 */
    parts.push(
      <mark key={found} className={s.mark}>
        {text.slice(found, found + q.length)}
      </mark>,
    )
    at = found + q.length
    found = hay.indexOf(q, at)
  }
  parts.push(text.slice(at))
  return <>{parts}</>
}
