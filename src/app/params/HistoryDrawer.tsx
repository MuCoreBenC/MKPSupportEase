/*
 * 修改历史抽屉。
 *
 * # 三处要点
 *
 *   1. 外壳换成 `Drawer`（`position: absolute`，见那个文件的文件头）
 *   2. 视图切换用共用件 `SegmentedField` —— 归一之后「分段选择器长什么样」
 *      只该有一处定义
 *   3. **每条改动自带上下文**（操作记录底座 ④）：参数名 / 分类 / 分组 / 单位都是
 *      `HistoryItem` 成形那一刻的快照，这里**不再回查当前 combo 的字段定义**——
 *      切了机型/版本、目录更新之后，历史条目照样说得清"当时改的是哪一项"。
 *
 * # 保留的两个主张
 *
 *   两种视图    **逐项**（未保存 / 第 N 次保存分段，一项一行）与**整批**（一段一行）——
 *               两者都**按保存组织**，没有"变一下记一条"的实时屏（2026-10-17 作者口径）。
 *               整批那一屏回答的是「这一次保存我整体都改了些什么、能不能一次退回去」；
 *               ★ 整批行**可以展开**（作者 2026-10-17）：展开后把这一段里每一项
 *                 摊出来（每项自己也有「还原」），于是"整段退"与"退其中一项"都在手边；
 *   段头 sticky  仓库有一条幽灵滚动条约定（global.css 把滚动条宽度设成 0），
 *               看不见滚动条，滚起来必须始终知道自己在哪一段
 *
 * # 两屏都只记净变化（2026-10-16 作者）
 *
 *   「它只记录那个最终的」—— 同一项一批里改了几次只记头尾（`70 → 75 → 80 → 90`
 *   就是一条 `70 → 90`），改回原值的不出现。聚合在 `useParams` 的 `netOf` 里做；
 *   这一层只是照收到的 `groups` 摆行，行上的「还原」退的是"这一段开始前"的值 ——
 *   **逐项退一项、整批退整段**（都走同一个 `onRevert`，见下面 `wholeEntry`）。
 */

import { useEffect, useRef, useState } from 'react'
import { SegmentedField } from '../../components/field'
import Drawer from '../shared/Drawer'
import type { DrawerWidth } from '../shared/useDrawerWidth'
/* 「进入编辑模式」那颗按钮说的就是外面同一句话（三个模态框 + 这里共用一个源） */
import { EDIT_TEXT } from '../presets/presetTree'
import type { HistoryEntry, HistoryGroup } from './useParams'
import s from './HistoryDrawer.module.css'

/**
 * 「还原」之后那句回话。**进场动一下**（作者要的"效果"）：
 * 160ms 从上方 3px 淡入 —— 不弹、不跳，只是让人看见"刚刚发生了一件事"。
 * ★ 动效走 WAAPI（WKWebView 不播 CSS animation）。
 */
function RevertNote({ note }: { note: { text: string; tone: 'ok' | 'muted' } }) {
  const ref = useRef<HTMLParagraphElement | null>(null)

  useEffect(() => {
    const el = ref.current
    if (el === null) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    const anim = el.animate(
      [
        { opacity: 0, transform: 'translateY(-3px)' },
        { opacity: 1, transform: 'translateY(0)' },
      ],
      { duration: 160, easing: 'ease-out' },
    )
    return () => anim.cancel()
  }, [note.text])

  return (
    <p ref={ref} className={s.revertNote} data-tone={note.tone} title={note.text}>
      {note.text}
    </p>
  )
}

/** 逐项 = 一项一行；整批 = 一次保存（或未保存那一段）一行 */
export type HistoryView = 'item' | 'batch'
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
  onView: (v: HistoryView) => void
  onMode: (m: DrawerMode) => void
  onClose: () => void
  /**
   * 「还原」那一条。**收整个 entry 而不是 id**（2026-10-16）：两屏的行都是
   * 聚合出来的（逐项 = 一项、`from` 是"这一段开始前"的值；整批 = 这一段的全部项），
   * 它们都没有 `log` 里的 id。整批那一份由 `wholeEntry` 在这里合成。
   */
  onRevert: (entry: HistoryEntry) => void
  /**
   * 「还原」之后那一句回话（2026-10-17）：成功 / 没变化 / 改不了。
   *
   * 作者的原话：「怎么我点了还原没反应…是真的没还原还是没有交互效果」——
   * 真相是**还原成功有变化、但命中 noop 时界面一个像素都不动**（目标值就是现值），
   * 而抽屉盖住了右边那列数值。所以这里必须回一句话，不能让人对着一颗按钮发愣。
   */
  note?: { text: string; tone: 'ok' | 'muted' } | null
  /**
   * **进入编辑模式**（2026-10-17，作者：「没有进入编辑模式就没有还原按钮，怎么让用户
   * 知道点击进入编辑模式…和外面的按钮复用同样的逻辑」）。
   *
   * 只读态里「还原」不画（它是写入口）—— 用户看到一排"已保存"却没有任何动作可做，
   * 也不知道门槛在哪（底栏那颗按钮还被抽屉盖在右边）。所以抽屉自己摆一颗，
   * **点的是页面同一个 `setEditMode(true)`**，不另开一套逻辑。
   */
  onEnterEdit?: () => void
  /**
   * **只读**（2026-10-16 的编辑模式两态）：历史照看，「还原」那颗不画 ——
   * 它是个写入口，只读态只负责"看得见改过什么"。
   */
  readonly?: boolean
  /**
   * **清空这份文件的修改历史**（2026-10-17 抽屉右上角那颗；不传就不画）。
   * 前端做二次确认：点一下变成「确认清空？」，再点才真清，3 秒不点自动复原。
   */
  onClearHistory?: () => void
}

const STATE_TEXT: Record<HistoryEntry['state'], string> = {
  draft: '未保存',
  saved: '已保存',
  undone: '已撤销',
}

const VIEWS = [
  { value: 'item', label: '逐项' },
  { value: 'batch', label: '整批' },
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
  onView,
  onMode,
  onClose,
  onRevert,
  note = null,
  onEnterEdit,
  readonly = false,
  onClearHistory,
}: Props) {
  /** 整批那一屏**展开了哪几段**（一段一个键：`draft` / `b<N>`）。只是这一屏的看法定 */
  const [openBatches, setOpenBatches] = useState<ReadonlySet<string>>(() => new Set())
  /**
   * 「清空历史」的**二次确认**（作者要求）：点第一下只是武装，再点才清。
   * 3 秒没点第二下就自己退回去，不让人对着一颗红按钮发呆。
   */
  const [armClear, setArmClear] = useState(false)
  const toggleBatch = (key: string) =>
    setOpenBatches((prev) => {
      const next = new Set(prev)
      if (!next.delete(key)) next.add(key)
      return next
    })

  /*
   * 一条改动 → 两句：
   *   上句 **上下文**：`分类 / 分组 · 参数名`（就是"哪个分类 → 哪个参数"，历史缺的那点信息）
   *   下句 **值**：`旧 → 新`（带单位；单位也来自快照，不回查定义）
   * 两项以上时上句改成"共 N 项"，避免拿第一项冒充整条动作。
   */
  const describe = (entry: HistoryEntry) => {
    const first = entry.items[0]
    if (first === undefined) return { ctx: entry.action, line: entry.action }
    const many = entry.items.length > 1
    const where = [first.tab, first.section].filter((x) => x !== '').join(' / ')
    const ctx = many
      ? `${first.label} 等 ${entry.items.length} 项`
      : where === ''
        ? first.label
        : `${where} · ${first.label}`
    const unit = first.unit ?? ''
    const line = many
      ? `共 ${entry.items.length} 项（含 ${first.label} ${first.from} → ${first.to}）`
      : `${first.from}${unit} → ${first.to}${unit}`
    return { ctx, line }
  }

  /** 一行的**内容**（不含外层 `li`）—— 整批那一屏的行要被 `<li>` 包两层（行 + 展开的明细） */
  const rowInner = (
    entry: HistoryEntry,
    head?: string,
    meta?: string,
    noText?: string,
    caret?: { open: boolean; label: string; onToggle: () => void },
  ) => {
    const { ctx, line } = describe(entry)
    return (
      <>
        {caret !== undefined && (
          <button
            type="button"
            className={s.caret}
            aria-expanded={caret.open}
            aria-label={`${caret.open ? '收起' : '展开'} ${caret.label}的明细`}
            onClick={caret.onToggle}
          >
            {caret.open ? '▾' : '▸'}
          </button>
        )}
        <span className={s.no}>{noText ?? `#${entry.no}`}</span>
        <span className={s.text}>
          <span className={s.line}>{head ?? ctx}</span>
          <span className={s.meta}>{meta ?? `${line} · ${entry.action}`}</span>
        </span>
        <span className={entry.state === 'saved' ? s.badgeSaved : s.badge}>
          {STATE_TEXT[entry.state]}
        </span>
        {entry.state !== 'undone' && !readonly && (
          <button type="button" className={s.revert} onClick={() => onRevert(entry)}>
            还原
          </button>
        )}
      </>
    )
  }

  /** 逐项那一屏的行：一项一行，行上「还原」退这一项 */
  const itemRow = (entry: HistoryEntry) => (
    <li key={entry.id} className={entry.state === 'undone' ? s.itemOff : s.item}>
      {rowInner(entry)}
    </li>
  )

  /*
   * **整批那一屏：一段合成一条 entry**（2026-10-17 作者口径）。
   *
   * 段里每一项的 `from` 就是"这一段开始前"的值（见 `netOf`），于是"一次退整段"
   * = 把这些项**一起**交给 `onRevert`：它走 `apply`，**一个动作、撤销一次全回来**，
   * 草稿 TOML 也跟着逐键回写 —— 不需要后端新增任何东西。
   */
  const wholeEntry = (g: HistoryGroup): HistoryEntry => ({
    /* 段号当 id/序号：整批那一屏不占逐项的号 */
    id: g.batch ?? 0,
    no: g.batch ?? 0,
    /* `revertEntry` 会写成「回退 <action>」，所以这里只放段名本身 */
    action: g.title,
    items: g.items.flatMap((e) => e.items),
    state: g.batch === null ? 'draft' : 'saved',
    batch: g.batch,
  })

  /*
   * 整批的一段 = **可展开的一块**（作者 2026-10-17：「整批的要可以展开看详细的」）：
   * 行本身一次退整段；展开后把这一段的每一项摊出来（每项自己也有「还原」，
   * 退的是"这一段开始前"那一个值）—— 与逐项那一屏同源，只是挂在这一段下面。
   * 展开态是这一屏自己的 UI 状态，谁展开过不写进任何账。
   */
  const batchBlock = (g: HistoryGroup) => {
    const key = g.batch === null ? 'draft' : `b${g.batch}`
    const openBatch = openBatches.has(key)
    const entry = wholeEntry(g)
    const { line } = describe(entry)
    return (
      <li key={key} className={s.block}>
        <div className={s.item}>
          {rowInner(
            entry,
            g.title,
            line,
            /* 未保存那一段没有批号，摆一道横杠而不是 `#0` */
            g.batch === null ? '—' : `#${g.batch}`,
            { open: openBatch, label: g.title, onToggle: () => toggleBatch(key) },
          )}
        </div>
        {openBatch && (
          <ul className={s.subList}>
            {g.items.map((e, i) => (
              <li key={e.id} className={`${e.state === 'undone' ? s.itemOff : s.item} ${s.subItem}`}>
                {rowInner(
                  e,
                  undefined,
                  undefined,
                  /*
                   * 段内子项的号 = **段号-段内序号**（2026-10-17 作者：「能不能用 1-1 这种」）——
                   * 跨段连续的 `#4 #5` 读不出"它在哪一段"，`2-1` 一眼就读出来。
                   */
                  `${g.batch ?? '未'}-${i + 1}`,
                )}
              </li>
            ))}
          </ul>
        )}
      </li>
    )
  }

  return (
    <Drawer
      open={open}
      title="修改历史"
      /* 历史跟着**这份文件**走（2026-10-17），不是跟着这一次打开 —— 说"这次打开之后"就成了骗人的表现 */
      subtitle={`${file} 的改动记录（一直留着）`}
      mode={mode}
      width={mode === 'float' ? width : undefined}
      drag={mode === 'float' ? drag : undefined}
      onClose={onClose}
      actions={
        <>
          {onClearHistory !== undefined && history.length > 0 && (
            <button
              type="button"
              className={armClear ? s.clearArm : s.ghost}
              title={armClear ? '再点一下就真的清掉了' : '清空这份文件的修改历史'}
              onClick={() => {
                if (armClear) {
                  setArmClear(false)
                  setOpenBatches(new Set())
                  onClearHistory()
                } else {
                  setArmClear(true)
                  window.setTimeout(() => setArmClear(false), 3000)
                }
              }}
            >
              {armClear ? '确认清空？' : '清空历史'}
            </button>
          )}
          {canPin && (
            <button
              type="button"
              className={s.ghost}
              onClick={() => onMode(mode === 'pinned' ? 'float' : 'pinned')}
            >
              {mode === 'pinned' ? '改为悬浮' : '固定在右侧'}
            </button>
          )}
        </>
      }
      footer={
        <p className={s.foot}>
          这些改动跟着这份文件留着 —— 关掉软件、重开都还在；只有「清空历史」会清掉它。
        </p>
      }
    >
      {/* 「还原」的回话：一句话、占一行，几秒后自己消失（状态由页面管） */}
      {note !== null && <RevertNote note={note} />}

      <div className={s.views}>
        <span className={s.viewLabel}>视图</span>
        <SegmentedField
          variant="segmented"
          label="修改历史视图"
          options={VIEWS}
          value={view}
          onChange={(v) => onView(v as HistoryView)}
        />
        {/*
          只读态：把人领到「进入编辑模式」那一态（点的是页面同一颗逻辑）。
          没有它，这一屏就只有一排"已保存"，还原藏在编辑模式后面，而底栏那颗按钮
          正好被抽屉盖着 —— 作者：「怎么让用户知道点击进入编辑模式」。
        */}
        {readonly && onEnterEdit !== undefined && (
          <button
            type="button"
            className={s.enterEdit}
            title="进入编辑模式之后，这里的每一条都能还原"
            onClick={onEnterEdit}
          >
            {EDIT_TEXT.enterMode}
          </button>
        )}
      </div>

      {/* 只读态把门槛说清楚（一句话，进编辑模式后自己消失） */}
      {readonly && onEnterEdit !== undefined && history.length > 0 && (
        <p className={s.readonlyHint}>当前只读：进入编辑模式后，每一条都能还原</p>
      )}

      {history.length === 0 ? (
        <p className={s.empty}>这个文件本次打开还没有改动</p>
      ) : (
        <div className={s.scroll}>
          {groups.length === 0 ? (
            /* 净变化全为零（改动都被改回原值了）—— 说一句，不摆一个「0 条」的空段 */
            <p className={s.empty}>改动都改回了原值 —— 按保存批次看，这里没有净变化</p>
          ) : view === 'item' ? (
            groups.map((g) => (
              <section key={g.batch ?? 'draft'} className={s.group}>
                {/* 段头 sticky —— 看不见滚动条，靠它知道现在在哪一段 */}
                <h4 className={s.groupHead}>
                  <span>{g.title}</span>
                  <span className={s.count}>{g.items.length} 条</span>
                </h4>
                <ul className={s.list}>{g.items.map(itemRow)}</ul>
              </section>
            ))
          ) : (
            /* 整批：**一段一行**（段名 + 这一段一共几项 + 一次退整段），行可展开看明细 */
            <ul className={s.list}>{groups.map(batchBlock)}</ul>
          )}
        </div>
      )}
    </Drawer>
  )
}
