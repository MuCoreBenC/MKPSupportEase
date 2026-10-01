/*
 * 5 页 Tab（质量 · 强度 · 速度 · 支撑 · 其他）。
 *
 * 「跟 BBS 一样」档下一行不剩的页**连 Tab 一起不出现** —— BBS 也这么做
 * （速度页在默认配置下就是这样）。判断在 bbsPanel，这里只按 rowCount 过滤。
 *
 * 搜索时那个命中数角标**不是装饰**：这一页一次只渲染当前那一页的行，
 * 别的页的命中在 DOM 里根本不存在 —— 角标是它们唯一的发声渠道。
 * 所以搜索中没命中的页要**压暗**，让「哪几页有货」一眼可见。
 */

import type { PanelPage } from './bbsPanel'
import s from './BbsTabs.module.css'

interface Props {
  pages: PanelPage[]
  active: string | null
  /** 正在搜索（用来决定要不要把没命中的页压暗） */
  searching: boolean
  onChange: (id: string) => void
}

export default function BbsTabs({ pages, active, searching, onChange }: Props) {
  const visible = pages.filter((p) => p.rowCount > 0)
  if (!visible.length) return null

  return (
    <nav className={s.tabs} aria-label="参数分页">
      {visible.map((p) => (
        <button
          key={p.id}
          type="button"
          className={s.tab}
          data-on={p.id === active}
          data-dim={searching && p.hitCount === 0 ? 'true' : undefined}
          aria-current={p.id === active ? 'page' : undefined}
          title={`${p.title} · ${p.rowCount} 行${searching ? ` · 命中 ${p.hitCount}` : ''}`}
          onClick={() => onChange(p.id)}
        >
          {p.title}
          {searching && p.hitCount > 0 && <span className={s.badge}>{p.hitCount}</span>}
        </button>
      ))}
    </nav>
  )
}
