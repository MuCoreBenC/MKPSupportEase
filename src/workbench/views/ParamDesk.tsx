/**
 * 默认视角：**一个版本的分组列表**。
 *
 * 矩阵那一屏（`ParamsMatrix`）退成「同时看几台机器的同一项」那个对比工具。
 * 理由很实在：67 行 × 十几列的表格本来就不好看，也不方便改 ——
 * 一次只改一个版本的时候，一屏十几行、每行看得清值和来源，才是顺手的形状。
 *
 * # 三段
 *
 * ```
 * 顶部：机型:版本 · 共 N 项 · 搜索
 * 左栏：分组树（tab → section，带计数，**不随搜索变**）
 * 中栏：分组卡片 → 一行一项 → 点开就地改
 * ```
 *
 * # 刻意不做的几样（都是 mkppanel 里「当时想自由，现在觉得多余」的部分）
 *
 * - 拖拽排序：顺序是上游 `layout.order` 说的
 * - 增删 section / 参数：字段清单是上游的
 * - 改父子挂靠：父子关系是上游数据，我们只照它显示
 *
 * # 子项与「被关着」
 *
 * 子项挂在父项下面。父项把它们关掉时**整组收起来**，只留一句
 * 「『擦拭部件』选了圆盘擦拭，下面这 N 项现在不生效」——
 * 那句话点一下还能展开（值还在、会进产物），但默认不占地方。
 * 矩阵里只能靠每格一行灰字，那正是「信息过载」的来源。
 *
 * # 一次手势一次 IPC
 *
 * 改完不再单独去问一次这一页：`onApply` 带上 `refresh`，后端顺带把新的
 * `Desk` 带回来。上一稿一次编辑要走两趟 IPC、后端把整本书算两遍。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  isAppError,
  wb,
  type Desk,
  type DeskGroup,
  type DeskItem,
  type ParamView,
  type Patch,
  type Refresh,
  type RegistryView,
  type Row,
  type Words,
} from '../api'
import { FieldControl, Switch } from './FieldControl'

interface Props {
  machineId: string
  /** `null` = 机型基底那一层 */
  uid: string | null
  /** 这一层的中文名，顶部显示用 */
  where: string
  words: Words
  /** 后端状态变过了就 +1。**不是脏计数** —— 那个可以在状态真的变了时保持不变 */
  tick: number
  /** 唯一写入口。第三个参数让后端顺带把这一页带回来 */
  onApply: (label: string, patches: Patch[], refresh: Refresh) => Promise<{ desk: Desk | null }>
  /** 「在所有机型上看这一项」 */
  onCompare: (key: string) => void
}

export function ParamDesk({ machineId, uid, where, words, tick, onApply, onCompare }: Props) {
  const [registry, setRegistry] = useState<RegistryView | null>(null)
  const [desk, setDesk] = useState<Desk | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [tab, setTab] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  /** 展开哪一项。**一次只开一个** —— 开一堆等于又回到信息过载 */
  const [open, setOpen] = useState<string | null>(null)
  /** 被父项关掉、但用户点了「仍然展开看」的那几组 */
  const [peek, setPeek] = useState<Set<string>>(new Set())
  const [gcode, setGcode] = useState<{ row: Row; param: ParamView } | null>(null)
  const scrollRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    void wb
      .registry()
      .then(setRegistry)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [])

  const refresh: Refresh = useMemo(
    () => ({ page: 'desk', machineId, uid, tab, query }),
    [machineId, uid, tab, query],
  )

  useEffect(() => {
    void wb
      .desk(machineId, uid, tab, query)
      .then(setDesk)
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [machineId, uid, tab, query, tick])

  const paramOf = useCallback(
    (key: string) => registry?.params.find((p) => p.key === key) ?? null,
    [registry],
  )

  /** 写一个值。**层由「现在在看哪一层」决定，前端不猜** */
  const setOne = useCallback(
    async (row: Row, next: unknown) => {
      const out = await onApply(
        `${where} · ${row.label}`,
        [
          {
            kind: 'setValue',
            level: uid ? 'version' : 'machine',
            owner: uid ?? machineId,
            key: row.key,
            value: next,
          },
        ],
        refresh,
      )
      if (out.desk) setDesk(out.desk)
    },
    [onApply, refresh, uid, machineId, where],
  )

  /** 挂回继承 = 删掉这一层的这个键。**不是写回上一层的值** */
  const detach = useCallback(
    async (row: Row) => {
      const out = await onApply(
        `${where} · ${row.label} 挂回继承`,
        [
          {
            kind: 'setValue',
            level: uid ? 'version' : 'machine',
            owner: uid ?? machineId,
            key: row.key,
            value: null,
          },
        ],
        refresh,
      )
      if (out.desk) setDesk(out.desk)
    },
    [onApply, refresh, uid, machineId, where],
  )

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      if (gcode) setGcode(null)
      else setOpen(null)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [gcode])

  const jumpToGroup = useCallback((sectionId: string) => {
    scrollRef.current
      ?.querySelector(`[data-group="${sectionId}"]`)
      ?.scrollIntoView({ block: 'start' })
  }, [])

  if (error) {
    return (
      <p className="wb-todo" data-tone="danger">
        {error}
      </p>
    )
  }

  return (
    <div className="wb-desk">
      <div className="wb-desk__bar">
        <span className="wb-desk__where">{where}</span>
        {desk && <span className="wb-mx__count">共 {desk.total} 项</span>}
        <input
          className="wb-input"
          type="search"
          placeholder="搜字段名 / key / tomlKey / 分类 / 说明"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {desk?.note && <p className="wb-mx__note">{desk.note}</p>}

      <div className="wb-desk__body">
        {/* 左栏：分组树。点一下只做定位，不做过滤 —— 过滤是搜索的事 */}
        <nav className="wb-desk__nav">
          <button
            type="button"
            className="wb-desk__navtab"
            data-on={tab === null ? 'yes' : undefined}
            onClick={() => setTab(null)}
          >
            全部
          </button>
          {desk?.nav.map((t) => (
            <div key={t.id} className="wb-desk__navgroup">
              <button
                type="button"
                className="wb-desk__navtab"
                data-on={tab === t.id ? 'yes' : undefined}
                onClick={() => setTab(t.id)}
              >
                {t.label}
                <i className="wb-desk__n">{t.count}</i>
              </button>
              {t.sections.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  className="wb-desk__navsec"
                  onClick={() => jumpToGroup(s.id)}
                >
                  {s.label}
                  <i className="wb-desk__n">{s.count}</i>
                </button>
              ))}
            </div>
          ))}
        </nav>

        {/* 中栏：分组卡片 */}
        <div className="wb-desk__list" ref={scrollRef}>
          {desk?.emptyReason ? (
            <p className="wb-todo">{desk.emptyReason}</p>
          ) : (
            desk?.groups.map((g) => (
              <GroupCard
                key={g.sectionId}
                group={g}
                words={words}
                open={open}
                peek={peek}
                paramOf={paramOf}
                onToggle={(key) => setOpen((k) => (k === key ? null : key))}
                onPeek={(id) =>
                  setPeek((s) => {
                    const next = new Set(s)
                    if (next.has(id)) next.delete(id)
                    else next.add(id)
                    return next
                  })
                }
                onCommit={(row, v) => void setOne(row, v)}
                onDetach={(row) => void detach(row)}
                onGcode={(row, param) => setGcode({ row, param })}
                onCompare={onCompare}
              />
            ))
          )}
        </div>
      </div>

      {gcode && (
        <>
          <div className="wb-backdrop" onClick={() => setGcode(null)} aria-hidden />
          <GcodePane
            row={gcode.row}
            param={gcode.param}
            where={where}
            onClose={() => setGcode(null)}
            onCommit={(text) => {
              void setOne(gcode.row, text)
              setGcode(null)
            }}
          />
        </>
      )}
    </div>
  )
}

function GroupCard({
  group,
  words,
  open,
  peek,
  paramOf,
  onToggle,
  onPeek,
  onCommit,
  onDetach,
  onGcode,
  onCompare,
}: {
  group: DeskGroup
  words: Words
  open: string | null
  peek: Set<string>
  paramOf: (key: string) => ParamView | null
  onToggle: (key: string) => void
  onPeek: (id: string) => void
  onCommit: (row: Row, v: unknown) => void
  onDetach: (row: Row) => void
  onGcode: (row: Row, param: ParamView) => void
  onCompare: (key: string) => void
}) {
  const groupOff = Boolean(group.offNote)
  const shown = groupOff && !peek.has(group.sectionId)

  return (
    <section className="wb-card" data-group={group.sectionId}>
      <header className="wb-card__head">
        <span className="wb-card__title">{group.label}</span>
        <span className="wb-mx__count">{group.count} 项</span>
      </header>

      {/* 整组被 section 级条件关掉：收起来，只留一句 */}
      {group.offNote && (
        <p className="wb-desk__off">
          <span>{group.offNote}</span>
          <button type="button" className="wb-link" onClick={() => onPeek(group.sectionId)}>
            {peek.has(group.sectionId) ? '收起' : words.relate.showAnyway}
          </button>
        </p>
      )}

      {!shown &&
        group.items.map((it) => (
          <ItemRow
            key={it.row.key}
            item={it}
            words={words}
            open={open}
            peek={peek}
            paramOf={paramOf}
            onToggle={onToggle}
            onPeek={onPeek}
            onCommit={onCommit}
            onDetach={onDetach}
            onGcode={onGcode}
            onCompare={onCompare}
          />
        ))}
    </section>
  )
}

function ItemRow({
  item,
  words,
  open,
  peek,
  paramOf,
  onToggle,
  onPeek,
  onCommit,
  onDetach,
  onGcode,
  onCompare,
}: {
  item: DeskItem
  words: Words
  open: string | null
  peek: Set<string>
  paramOf: (key: string) => ParamView | null
  onToggle: (key: string) => void
  onPeek: (id: string) => void
  onCommit: (row: Row, v: unknown) => void
  onDetach: (row: Row) => void
  onGcode: (row: Row, param: ParamView) => void
  onCompare: (key: string) => void
}) {
  const hideKids = Boolean(item.offNote) && !peek.has(item.row.key)

  return (
    <>
      <ParamLine
        row={item.row}
        words={words}
        open={open === item.row.key}
        param={paramOf(item.row.key)}
        depth={0}
        onToggle={() => onToggle(item.row.key)}
        onCommit={(v) => onCommit(item.row, v)}
        onDetach={() => onDetach(item.row)}
        onGcode={(p) => onGcode(item.row, p)}
        onCompare={() => onCompare(item.row.key)}
      />

      {/* 父项把下面那几项关掉了：**收起来**，只留一句话 */}
      {item.offNote && (
        <p className="wb-desk__off" data-indent="yes">
          <span>{item.offNote}</span>
          <button type="button" className="wb-link" onClick={() => onPeek(item.row.key)}>
            {peek.has(item.row.key) ? '收起' : words.relate.showAnyway}
          </button>
        </p>
      )}

      {!hideKids &&
        item.children.map((c) => (
          <ParamLine
            key={c.key}
            row={c}
            words={words}
            open={open === c.key}
            param={paramOf(c.key)}
            depth={1}
            onToggle={() => onToggle(c.key)}
            onCommit={(v) => onCommit(c, v)}
            onDetach={() => onDetach(c)}
            onGcode={(p) => onGcode(c, p)}
            onCompare={() => onCompare(c.key)}
          />
        ))}
    </>
  )
}

/**
 * 一行。收起来时只有四样：**中文名 · 类型 · 当前值（带来源色）· 展开箭头**。
 *
 * 布尔项的开关**常驻**：改一个开关点一下就够，不用先展开
 */
function ParamLine({
  row,
  words,
  open,
  param,
  depth,
  onToggle,
  onCommit,
  onDetach,
  onGcode,
  onCompare,
}: {
  row: Row
  words: Words
  open: boolean
  param: ParamView | null
  depth: number
  onToggle: () => void
  onCommit: (v: unknown) => void
  onDetach: () => void
  onGcode: (param: ParamView) => void
  onCompare: () => void
}) {
  const cell = row.cells[0]
  if (!cell) return null

  const na = cell.kind === 'notApplicable'
  const isSwitch = param?.uiComponent === 'switch'

  return (
    <div className="wb-line" data-depth={depth || undefined} data-dirty={cell.dirty ? 'yes' : undefined}>
      <button type="button" className="wb-line__head" onClick={onToggle} disabled={na}>
        <span className="wb-line__label">
          {row.label}
          {row.deprecated && <i className="wb-mx__dep">已废弃</i>}
        </span>
        {param && <span className="wb-line__type">{param.valueType}</span>}
      </button>

      <span className="wb-line__value">
        {na ? (
          <span className="wb-line__na" title={cell.reason ?? undefined}>
            {cell.text}
          </span>
        ) : cell.kind === 'gcode' ? (
          <button
            type="button"
            className="wb-mx__gcode"
            onClick={() => param && onGcode(param)}
            title="点开看全文并编辑 —— G-code 不在行里改"
          >
            {cell.text}
          </button>
        ) : isSwitch ? (
          <Switch
            on={cell.raw === true}
            disabledReason={cell.editable ? null : cell.blockedNote}
            onToggle={onCommit}
          />
        ) : (
          <span className="wb-line__text">{cell.text}</span>
        )}
        {cell.origin && (
          <span
            className="wb-mx__origin"
            data-origin={cell.origin}
            title={`${cell.originLabel} · ${cell.originExplain}`}
          >
            {words.origin[cell.origin].label}
          </span>
        )}
      </span>

      {open && !na && (
        <div className="wb-line__open">
          {/* 就地改。开关那一行上面已经有常驻控件了，不再放第二个 */}
          {cell.kind === 'value' && !isSwitch && param && (
            <div className="wb-line__edit">
              <span className="wb-line__k">当前值</span>
              <FieldControl
                param={param}
                value={cell.raw}
                form="row"
                autoFocus
                onCommit={onCommit}
                onCancel={onToggle}
              />
              {param.unit && <span className="wb-line__unit">{param.unit}</span>}
              <button
                type="button"
                className="wb-mx__detach"
                disabled={!cell.own}
                onClick={onDetach}
                title={cell.own ? words.disabled.detachReady : words.disabled.detachNothing}
              >
                ⤺
              </button>
            </div>
          )}

          {row.desc && <p className="wb-line__desc">{row.desc}</p>}

          {/* 改不动的时候说清是谁关的。整句由后端给 */}
          {cell.blockedNote && <p className="wb-line__blocked">{cell.blockedNote}</p>}

          <details className="wb-line__meta">
            <summary>高级元数据</summary>
            <dl>
              <dt>键名</dt>
              <dd>{row.key}</dd>
              {param && (
                <>
                  <dt>TOML 键</dt>
                  <dd>{param.tomlKey}</dd>
                  <dt>出厂默认</dt>
                  <dd>{param.defaultText}</dd>
                </>
              )}
              <dt>来源</dt>
              <dd>{cell.originExplain ?? words.placeholder.blank}</dd>
              {row.parentNote && (
                <>
                  <dt>归属</dt>
                  <dd>{row.parentNote}</dd>
                </>
              )}
              {row.controlNote && (
                <>
                  <dt>关联</dt>
                  <dd>{row.controlNote}</dd>
                </>
              )}
            </dl>
          </details>

          <button type="button" className="wb-link" onClick={onCompare}>
            在所有机型上看这一项 →
          </button>
        </div>
      )}
    </div>
  )
}

/** G-code 不在行里改：一段几十行的脚本挤进一行既看不清也撑坏行高 */
function GcodePane({
  row,
  param,
  where,
  onClose,
  onCommit,
}: {
  row: Row
  param: ParamView
  where: string
  onClose: () => void
  onCommit: (text: string) => void
}) {
  const cell = row.cells[0]
  const [text, setText] = useState(typeof cell?.raw === 'string' ? cell.raw : '')

  return (
    <aside className="wb-drawer" role="dialog" aria-label={`${row.label} 的 G-code`}>
      <header className="wb-drawer__head">
        <span>
          {row.label} · {where}
        </span>
        <button type="button" className="wb-link" onClick={onClose}>
          关闭
        </button>
      </header>
      {param.desc && <p className="wb-drawer__desc">{param.desc}</p>}
      <textarea
        className="wb-drawer__ta"
        value={text}
        spellCheck={false}
        onChange={(e) => setText(e.target.value)}
      />
      <footer className="wb-drawer__foot">
        <span className="wb-mx__count">{text.split('\n').length} 行</span>
        <button type="button" className="wb-btn" onClick={onClose}>
          取消
        </button>
        <button
          type="button"
          className="wb-btn"
          data-tone="primary"
          disabled={!cell?.editable}
          title={cell?.editable ? undefined : (cell?.blockedNote ?? undefined)}
          onClick={() => onCommit(text)}
        >
          保存这一段
        </button>
      </footer>
    </aside>
  )
}
