/*
 * 设置页的**卡片头**（2026-11-10，照作者给的参考稿）。
 *
 * 一张卡 = 一组设置，卡里第一行永远是"图标 + 名称 + 一句说明"：
 *
 *   ┌──────────────────────────────────────────┐
 *   │ [icon] 后处理                             │
 *   │        跑完怎么通知、用什么界面显示…       │
 *   │ ──────────────────────────────────────── │
 *   │ 处理完成发送系统通知              [开关]  │
 *   └──────────────────────────────────────────┘
 *
 * ★ 为什么单独一个组件：这一段在八个文件里各写一遍的话，图标尺寸 / 说明的灰阶 /
 *   `data-head` 这个给搜索用的标记迟早会漂 —— 它们必须是同一处定义。
 * ★ `data-head`：`PageSettings` 的搜索按"章节的直接子项"逐行藏，卡片头不是设置项，
 *   有它才不会因为搜索词不命中标题就把整张卡的抬头藏掉。
 * ★ `level`：`2` = 类目名（左目录里那一项），`3` = 类目里的小段名
 *   （校准与补偿 / 热床预涂胶 / 清理与卸载）—— 同一张卡头，字号差一档。
 */

import Icon, { type IconName } from '../../components/Icon'
import s from './PageSettings.module.css'

export function SectionHead({
  icon,
  title,
  desc,
  level = 2,
}: {
  icon: IconName
  title: string
  /** 一句"这组管什么"。没有就整行不出现（不许拿空字符串顶） */
  desc?: string
  level?: 2 | 3
}) {
  const Head = level === 2 ? 'h2' : 'h3'
  return (
    <div className={s.secHead} data-head>
      <span className={s.secIcon} aria-hidden="true">
        <Icon name={icon} size={15} />
      </span>
      <div className={s.secHeadText}>
        <Head className={level === 2 ? s.secTitle : s.subTitle}>{title}</Head>
        {desc !== undefined && desc !== '' && <p className={s.secDesc}>{desc}</p>}
      </div>
    </div>
  )
}

export default SectionHead
