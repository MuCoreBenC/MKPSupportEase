/*
 * 修改历史抽屉。
 *
 * # 三处要点
 *
 *   1. 外壳换成 `Drawer`（`position: absolute`，见那个文件的文件头）
 *   2. 视图切换用共用件 `SegmentedField` —— 归一之后「分段选择器长什么样」
 *      只该有一处定义
 *   3. 字段名与值的翻译读 `useParams` 的 `defOf` / `valueText`，不再读手抄表
 *
 * # 保留的两个主张
 *
 *   两种视图    实时（按动作倒序）与按保存批次（未保存 / 第 N 次保存分段）——
 *               后者回答的是「我上次保存都改了些什么」
 *   段头 sticky  仓库有一条幽灵滚动条约定（global.css 把滚动条宽度设成 0），
 *               看不见滚动条，滚起来必须始终知道自己在哪一段
 */

import { SegmentedField } from '../../components/field'
import Drawer from '../shared/Drawer'
import type { DrawerWidth } from '../shared/useDrawerWidth'
import { valueText } from './useParams'
import type { HistoryEntry, HistoryGroup, ParamDef } from './useParams'
import s from './HistoryDrawer.module.css'

export type HistoryView = 'live' | 'batch'
export type DrawerMode = 'float' | 'pinned'

interface Props {
  open: boolean
  /** 正在编的那个文件名（没配预设文件时给机型 · 版本） */
  file: string
  mode: DrawerMode
  /** 窄窗口不允许固定 —— 固定了卡片区就只剩一条缝 */
  canPin: boolean
  /**
   * 悬浮态的受控宽度 + 左缘拖宽热区（作者：「左边能拖动拖动它的宽度，
   * 全局都是用同一套逻辑」）。只对 float 生效；pinned 维持 340px 固定。
   */
  width?: number
  drag?: DrawerWidth
  view: HistoryView
  history: HistoryEntry[]
  groups: HistoryGroup[]
  defOf: (key: string) => ParamDef | undefined
  onView: (v: HistoryView) => void
  onMode: (m: DrawerMode) => void
  onClose: () => void
  onRevert: (id: number) => void
}

const STATE_TEXT: Record<HistoryEntry['state'], string> = {
  draft: '未保存',
  saved: '已保存',
  undone: '已撤销',
}

const VIEWS = [
  { value: 'live', label: '实时' },
  { value: 'batch', label: '按保存批次' },
]

export default function HistoryDrawer({
  open,
  file,
  mode,
  canPin,
  width,
  drag,
  view,
  history,
  groups,
  defOf,
  onView,
  onMode,
  onClose,
  onRevert,
}: Props) {
  const describe = (entry: HistoryEntry) => {
    const first = entry.items[0]
    if (first === undefined) return { line: entry.action, extra: null as string | null }
    const def = defOf(first.key)
    return {
      line: `${def?.label ?? first.key} ： ${valueText(def, first.from)} → ${valueText(def, first.to)}`,
      extra: entry.items.length > 1 ? `同一动作还改了 ${entry.items.length - 1} 项` : null,
    }
  }

  const row = (entry: HistoryEntry) => {
    const { line, extra } = describe(entry)
    return (
      <li key={entry.id} className={entry.state === 'undone' ? s.itemOff : s.item}>
        <span className={s.no}>#{entry.no}</span>
        <span className={s.text}>
          <span className={s.line}>{line}</span>
          <span className={s.meta}>
            {entry.action}
            {extra !== null ? ` · ${extra}` : ''}
          </span>
        </span>
        <span className={entry.state === 'saved' ? s.badgeSaved : s.badge}>
          {STATE_TEXT[entry.state]}
        </span>
        {entry.state !== 'undone' && (
          <button type="button" className={s.revert} onClick={() => onRevert(entry.id)}>
            还原
          </button>
        )}
      </li>
    )
  }

  return (
    <Drawer
      open={open}
      title="修改历史"
      subtitle={`这次打开 ${file} 之后的改动`}
      mode={mode}
      width={mode === 'float' ? width : undefined}
      drag={mode === 'float' ? drag : undefined}
      onClose={onClose}
      actions={
        canPin ? (
          <button
            type="button"
            className={s.ghost}
            onClick={() => onMode(mode === 'pinned' ? 'float' : 'pinned')}
          >
            {mode === 'pinned' ? '改为悬浮' : '固定在右侧'}
          </button>
        ) : undefined
      }
      footer={
        <p className={s.foot}>这是本次会话的改动记录。文件的历史版本要接后端才有，目前没有。</p>
      }
    >
      <div className={s.views}>
        <span className={s.viewLabel}>视图</span>
        <SegmentedField
          variant="segmented"
          label="修改历史视图"
          options={VIEWS}
          value={view}
          onChange={(v) => onView(v as HistoryView)}
        />
      </div>

      {history.length === 0 ? (
        <p className={s.empty}>这个文件本次打开还没有改动</p>
      ) : (
        <div className={s.scroll}>
          {view === 'live' ? (
            <ul className={s.list}>{history.map(row)}</ul>
          ) : (
            groups.map((g) => (
              <section key={g.batch ?? 'draft'} className={s.group}>
                {/* 段头 sticky —— 看不见滚动条，靠它知道现在在哪一段 */}
                <h4 className={s.groupHead}>
                  <span>{g.title}</span>
                  <span className={s.count}>{g.items.length} 条</span>
                </h4>
                <ul className={s.list}>{g.items.map(row)}</ul>
              </section>
            ))
          )}
        </div>
      )}
    </Drawer>
  )
}
