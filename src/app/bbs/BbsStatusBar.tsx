/*
 * 状态条 —— 口径与提示的唯一落点。这一页不弹 alert，出什么事都写在这里。
 *
 * 每一格都在回答一个「凭什么」：
 *   改动 3/18      这份预设存了 18 项，其中 3 项与基准不同（判定见 sameValue）
 *   基准 412 项    链合并 ⊕ 出厂默认之后有多少个 key
 *   跟 BBS 一样    收起了多少行、置灰多少、整页隐藏多少（规则覆盖数在 title 里）
 *   继承 A → B → C 基准是怎么算出来的 —— 这是这一页最关键的口径，所以单独占一行、允许换行
 *   本机 BBS · …   清单是实时读本机目录的；读不到就是空态。**降级不是错误，别用错误色**
 */

import type { PanelStats } from './bbsPanel'
import type { BbsSourceMode, BbsSyncInfo, BbsViewMode } from './bbsTypes'
import s from './BbsStatusBar.module.css'

interface Props {
  title: string
  isSystem: boolean
  modifiedCount: number
  storedCount: number
  editedCount: number
  baselineCount: number
  viewMode: BbsViewMode
  stats: PanelStats
  rulesNote: string
  chain: string[]
  variantNote: string
  sourceMode: BbsSourceMode
  sourceRoot: string
  sourceNote: string
  userCount: number
  systemCount: number
  sync: BbsSyncInfo | null
  /** 搜参数那一格：`搜「support」· 命中 7 项（这一页 3）`。没在搜就是 null */
  search: string | null
  /** 一句话提示（链缺了 / 变体对不上 / support_style 被拉回默认…） */
  note: string
  /** 真问题（读文件失败、导入被拒） */
  error: string
}

export default function BbsStatusBar({
  title,
  isSystem,
  modifiedCount,
  storedCount,
  editedCount,
  baselineCount,
  viewMode,
  stats,
  rulesNote,
  chain,
  variantNote,
  sourceMode,
  sourceRoot,
  sourceNote,
  userCount,
  systemCount,
  sync,
  search,
  note,
  error,
}: Props) {
  const mode = viewMode === 'bbs'
    ? `跟 BBS 一样 · 收起 ${stats.hiddenByRule} 行`
      + (stats.greyed ? ` · 置灰 ${stats.greyed}` : '')
      + (stats.pagesHidden ? ` · 隐藏 ${stats.pagesHidden} 页` : '')
    : `全部参数 · ${stats.rows} 行`

  const source = sourceMode === 'live'
    ? `本机 BBS · 用户 ${userCount} / 系统 ${systemCount}`
    : `本机 BBS 目录没读到 · 用户 ${userCount} / 系统 ${systemCount}`

  const stamp = sync?.bbs_version
    ? `BBS ${sync.bbs_version}${sync.syncedAt ? ` · 元数据抄于 ${sync.syncedAt.slice(0, 10)}` : ''}`
    : '没有元数据印章（public/bbs/_sync.json 缺失）'

  return (
    <footer className={s.bar}>
      <span>预设 {title}</span>
      <span>
        {isSystem
          ? '改动 —（系统预设就是基准）'
          : `改动 ${modifiedCount} / ${storedCount} 项`}
      </span>
      {editedCount > 0 && <span>已改 {editedCount} 处（内存里，不写回 BBS）</span>}
      <span>基准 {baselineCount} 项</span>
      <span title={rulesNote}>{mode}</span>
      {search && <span className={s.search}>{search}</span>}
      <span title={sourceRoot + (sourceNote ? `（${sourceNote}）` : '')}>{source}</span>
      <span title="元数据（参数定义 / 版面 / 出厂默认）随本仓分发；清单才是实时读本机 BBS 目录的">{stamp}</span>

      <span className={s.chain}>
        {chain.length
          ? `继承 ${chain.join(' → ')}${variantNote}`
          : '继承 —（没找到父预设，基准只有出厂默认）'}
      </span>

      {note && <span className={s.note}>{note}</span>}
      {error && <span className={s.error}>{error}</span>}
    </footer>
  )
}
