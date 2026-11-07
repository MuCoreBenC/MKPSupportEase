/*
 * **本次交付明细**（2026-10-19）。
 *
 * 作者的原话：「外面只显示简略的，点了才显示详细的」——因为预设 TOML 会越来越多，
 * 生成页那一块不可能把 9 份 TOML + 7 份 JSON 全铺开。
 *
 * # 形态：清单 + 详情两栏（不是“点查看跳只读全文”）
 *
 * 作者 2026-10-19 的第二轮意见：「点了查看看不到 diff 有什么变化」——
 * 所以左边选中一份，右边**直接就是与磁盘现存的行级 diff**（生成前确认同一套渲染，
 * 见 `DiffView`），顶部可切「完整」看全文；3MF 这类二进制没有文本 diff，
 * 右边说「目录登记多少 / 盘上多少」。
 *
 * # 状态：配方快照 + 字节登记
 *
 * 这份清单的状态来自后端 `DeliveryStatus`（`delivery_status` 已经把
 * `Book::build_state` 与 `catalog.json` 的登记合成一处）——前端不再二次推断。
 * 所以 `A1/FASTV3.3` 这种“配方改过、还没重新生成”的版本会显示「待更新」，
 * 与①生成卡同一句话。
 *
 * # 底部生成
 *
 * 「生成待更新项…」不要求先勾版本：`staleUids` 由页面按 `stale + neverBuilt` 算好，
 * 点了走既有的「预演 → 确认 → 写盘」流程，已生成的不进队列。
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import ModalC14 from '../c14/ModalC14'
import { deliveryStageText as STAGE_TEXT } from '../c14/labels'
import type {
  DeliveryDrift,
  DeliveryFile,
  DeliveryGroup,
  DeliveryStatus,
  PreviewFile,
  PreviewReport,
} from '../api'
import { DiffLines, DiffRuler } from './DiffView'
import type { DiffViewMode } from './DiffView'
import { measureRulerMarks } from './diffRuler'
import type { RulerMark } from './diffRuler'
import {
  displayName,
  fmtSize,
  groupSummary,
  groupTodo,
  sortFiles,
  stateText,
  stateTone,
  type StateTone,
} from './deliveryStatus'
import c from '../c14.module.css'
import s from './DeliveryFilesModal.module.css'
import { locateIntoView } from '../../components/scroll/locate'

interface Props {
  status: DeliveryStatus
  /** 只读预演（`wb_generate_preview`）：右侧 diff 的数据源；null = 正在算 / 失败 */
  preview: PreviewReport | null
  previewError: string | null
  /** 这次要进生成队列的版本 uid（空 = 没有可生成的，按钮禁用） */
  generateTargets: string[]
  /** 底部按钮文案（待更新项 / 重新生成目录） */
  generateLabel: string
  /** 关掉明细、跳进既有的生成确认流程 */
  onGenerate: () => void
  onClose: () => void
}

/** 与生成页同一套色调语义，只是这里用模态框自己的小徽标类名 */
function toneClass(tone: StateTone): string {
  if (tone === 'ok') return s.stateOk
  if (tone === 'todo') return s.statePending
  return s.stateMissing
}

/** 漂移那一行的人话：目录记多少、盘上多少，或者为什么取不到 */
function driftText(d: DeliveryDrift): string {
  switch (d.reason) {
    case 'missing':
      return '登记了，盘上取不到'
    case 'size':
      return `目录记 ${fmtSize(d.registeredSize)} · 盘上 ${fmtSize(d.actualSize)}`
    case 'bytes':
      return '大小相同，字节变了'
    case 'noExpectation':
      return '目录没登记哈希 / 大小'
  }
}

/** 选中项的 key（文件与漂移共用一个字符串空间，前缀避免撞名） */
const fileKey = (rel: string) => `f:${rel}`
const driftKey = (path: string) => `d:${path}`

/** 这一份在预演报告里的条目（TOML 用产物名匹配，附属 JSON 用 rel 匹配） */
function previewOf(rel: string, preview: PreviewReport | null): PreviewFile | null {
  if (preview === null) return null
  const direct = preview.files.find((f) => `mkp/presets/${f.fileName}` === rel)
  if (direct !== undefined) return direct
  return preview.aux.find((f) => f.fileName === rel || f.uid === rel) ?? null
}

export default function DeliveryFilesModal({
  status,
  preview,
  previewError,
  generateTargets,
  generateLabel,
  onGenerate,
  onClose,
}: Props) {
  /** 按后端分组归行，组内按真实差异排序：要动手的在前 */
  const rowsByGroup = useMemo(() => {
    const map = new Map<string, DeliveryFile[]>()
    for (const f of status.files) {
      const list = map.get(f.group) ?? []
      list.push(f)
      map.set(f.group, list)
    }
    for (const [key, list] of map) map.set(key, sortFiles(list))
    return map
  }, [status.files])
  /** 后端算好 allSame：有变化的组默认展开，全一致组默认折叠 */
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({})
  const isOpen = (g: DeliveryGroup) => collapsed[g.id] ?? !g.allSame
  const toggle = (id: string, open: boolean) =>
    setCollapsed((prev) => ({ ...prev, [id]: !open }))

  const jsonCount = status.files.filter((f) => f.rel.endsWith('.json')).length
  const pending = status.files.filter((f) => f.diff === 'modified' || f.diff === 'removed').length
  const missing = status.files.filter((f) => f.diff === 'added').length
  const unmanaged = status.files.filter((f) => f.diff === 'unmanaged').length

  /** 选中项：默认第一份真实待更新（没有就看第一条漂移，再没有就看第一份文件） */
  const defaultSel = useMemo(() => {
    const firstPending = status.files.find(
      (f) => f.diff === 'modified' || f.diff === 'removed' || f.diff === 'added',
    )
    if (firstPending !== undefined) return fileKey(firstPending.rel)
    if (status.drift.length > 0) return driftKey(status.drift[0].path)
    const first = status.files[0]
    return first === undefined ? null : fileKey(first.rel)
  }, [status])
  const [sel, setSel] = useState<string | null>(defaultSel)
  /** 选中行（打开时把默认选中项滚进左列可视区，免得"选中了却在屏幕外"） */
  const selRef = useRef<HTMLButtonElement | null>(null)
  useEffect(() => {
    selRef.current?.scrollIntoView({ block: 'nearest' })
  }, [sel])
  /** 明细刷新后原来选中的那份可能没了（生成完 / 目录重读）—— 回落到新的默认项 */
  useEffect(() => {
    const valid =
      sel === null
        ? false
        : sel.startsWith('f:')
          ? status.files.some((f) => fileKey(f.rel) === sel)
          : status.drift.some((d) => driftKey(d.path) === sel)
    if (!valid) setSel(defaultSel)
  }, [defaultSel, sel, status])
  const curFile = sel?.startsWith('f:')
    ? (status.files.find((f) => fileKey(f.rel) === sel) ?? null)
    : null
  const curDrift = sel?.startsWith('d:')
    ? (status.drift.find((d) => driftKey(d.path) === sel) ?? null)
    : null
  const curPreview = curFile === null ? null : previewOf(curFile.rel, preview)

  /** 右侧「对比 / 完整」—— 与生成前确认同一套语义 */
  const [view, setView] = useState<DiffViewMode>('diff')
  const diffRef = useRef<HTMLDivElement | null>(null)
  const [marks, setMarks] = useState<RulerMark[]>([])

  const showRuler =
    view === 'diff' &&
    curFile !== null &&
    curPreview !== null &&
    curPreview.state !== 'unchanged'

  useEffect(() => {
    if (!showRuler) {
      setMarks([])
      return
    }
    const el = diffRef.current
    if (el === null) {
      setMarks([])
      return
    }
    const measure = () => setMarks(measureRulerMarks(el))
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [showRuler, sel, curPreview])

  const jumpToRow = (row: number) => {
    const el = diffRef.current?.querySelector<HTMLElement>(`[data-row="${row}"]`)
    /* `behavior: 'smooth'` 在 WKWebView 上对元素级滚动容器等于瞬跳（本仓踩过），
       走共用定位件（底层演 transform）；这一处是"跳到 diff 行"，不铺绿底。 */
    if (el !== null && el !== undefined) locateIntoView(el, { flash: false })
  }

  /** 左侧一行（文件 / 漂移共用一种长相） */
  const row = (
    key: string,
    stateCls: string,
    stateLabel: string,
    name: string,
    meta: string,
  ) => (
    <button
      key={key}
      type="button"
      ref={sel === key ? selRef : undefined}
      className={sel === key ? `${s.item} ${s.itemOn}` : s.item}
      aria-current={sel === key}
      data-tip={name}
      onClick={() => setSel(key)}
    >
      <span className={s.itemName}>{name}</span>
      <span className={`${s.itemState} ${stateCls}`}>{stateLabel}</span>
      <span className={s.itemMeta}>{meta}</span>
    </button>
  )

  return (
    <ModalC14
      open
      size="xl"
      title="本次交付明细"
      subtitle={`${status.files.length} 个文件 · JSON ${jsonCount} 份${
        pending > 0 ? ` · ${pending} 份待更新 / 待删除` : ''
      }${missing > 0 ? ` · ${missing} 份未生成` : ''}${
        unmanaged > 0 ? ` · ${unmanaged} 份未纳管（不会删除）` : ''
      }${status.manifestStale ? ' · 发布凭据已失效' : ''}`}
      onClose={onClose}
      footer={
        <>
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
          <span className={c.grow} />
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            disabled={generateTargets.length === 0}
            data-tip={
              generateTargets.length === 0
                ? '当前没有待更新项，也没有要重算的目录'
                : '自动带上待更新与未生成的版本，先预演、确认后才写盘'
            }
            onClick={onGenerate}
          >
            {generateLabel}
          </button>
        </>
      }
    >
      {status.catalogError !== null && (
        <p className={s.error} role="alert">
          {status.catalogError}
        </p>
      )}
      {previewError !== null && (
        <p className={s.error} role="alert">
          预演没算出来：{previewError}
        </p>
      )}
      {status.manifestStale && (
        <div className={s.driftBox} role="status">
          <div className={s.kvRow}>
            <span>发布凭据</span>
            <b>已失效（{status.manifestReasons.length} 条）</b>
          </div>
          <p className={s.note}>
            生成或保存改变了清单描述的最终文件。重新计算摘要、大小与内容一致性由发布定稿负责。
          </p>
          {status.manifestReasons.slice(0, 3).map((reason) => (
            <div className={s.kvRow} key={reason}>
              <span>差异</span>
              <b>{reason}</b>
            </div>
          ))}
        </div>
      )}

      <div className={s.panes}>
        {/* 左：清单（按后端分组；有变化的组默认展开，全一致组折叠成带数字的摘要） */}
        <nav className={s.list} aria-label="本次交付文件">
          {status.drift.length > 0 && (
            <>
              <div className={s.groupHead}>资产字节漂移（{status.drift.length}）</div>
              {status.drift.map((d) =>
                row(
                  driftKey(d.path),
                  s.statePending,
                  '待重新生成',
                  d.path,
                  driftText(d),
                ),
              )}
            </>
          )}

          {status.groups.map((g) => {
            const files = rowsByGroup.get(g.id) ?? []
            const open = isOpen(g)
            const todo = groupTodo(g)
            return (
              <div key={g.id}>
                <button
                  type="button"
                  className={s.groupHead}
                  aria-expanded={open}
                  onClick={() => toggle(g.id, open)}
                >
                  <span>{open ? '▾' : '▸'}</span>
                  <span className={s.groupTitle}>{g.label}（{g.total}）</span>
                  {todo > 0 && <span className={s.statePending}>{todo} 项待处理</span>}
                  <span className={s.groupSum}>{groupSummary(g)}</span>
                </button>
                {open &&
                  files.map((f) =>
                    row(
                      fileKey(f.rel),
                      toneClass(stateTone(f)),
                      stateText(f),
                      displayName(f.rel),
                      f.exist ? fmtSize(f.size) : '—',
                    ),
                  )}
              </div>
            )
          })}
        </nav>

        {/* 右：选中那一份的 diff / 漂移说明 */}
        <div className={s.detail}>
          <div className={s.detailScroll}>
            {curDrift !== null ? (
              <>
                <header className={s.detailHead}>
                  <span className={`${s.detailName} ${c.mono}`}>{curDrift.path}</span>
                  <span className={s.detailMeta}>
                    <span className={s.statePending}>待重新生成</span>
                  </span>
                </header>
                <div className={s.driftBox}>
                  <div className={s.kvRow}>
                    <span>目录登记</span>
                    <b>{fmtSize(curDrift.registeredSize)}</b>
                  </div>
                  <div className={s.kvRow}>
                    <span>盘上实际</span>
                    <b>{fmtSize(curDrift.actualSize)}</b>
                  </div>
                  <div className={s.kvRow}>
                    <span>原因</span>
                    <b>{driftText(curDrift)}</b>
                  </div>
                  <p className={s.note}>
                    二进制载荷，没有文本 diff；生成会重算{' '}
                    <span className={c.mono}>catalog.json</span>。
                  </p>
                </div>
              </>
            ) : curFile !== null ? (
              <>
                <header className={s.detailHead}>
                  <span className={`${s.detailName} ${c.mono}`}>{curFile.rel}</span>
                  <span className={s.detailMeta}>
                    <span className={`${s.itemState} ${toneClass(stateTone(curFile))}`}>
                      {stateText(curFile)}
                    </span>
                    {curPreview !== null && curPreview.state !== 'unchanged' && (
                      <span className={s.counts}>
                        <span className={s.add}>+{curPreview.added}</span>
                        {curPreview.removed > 0 && (
                          <span className={s.del}>−{curPreview.removed}</span>
                        )}
                      </span>
                    )}
                    {curPreview !== null && (
                      <div className={s.viewToggle} role="group" aria-label="详情显示方式">
                        <button
                          type="button"
                          className={view === 'full' ? `${s.viewBtn} ${s.viewBtnOn}` : s.viewBtn}
                          aria-pressed={view === 'full'}
                          onClick={() => setView('full')}
                        >
                          完整
                        </button>
                        <button
                          type="button"
                          className={view === 'diff' ? `${s.viewBtn} ${s.viewBtnOn}` : s.viewBtn}
                          aria-pressed={view === 'diff'}
                          onClick={() => setView('diff')}
                        >
                          对比
                        </button>
                      </div>
                    )}
                  </span>
                </header>

                {curFile.diff === 'frozen' ? (
                  <p className={s.empty}>
                    这一份由{STAGE_TEXT[curFile.stage]}负责 —— 本次生成不会动它。
                  </p>
                ) : curFile.diff === 'removed' ? (
                  <p className={s.empty}>
                    当前源不再产出这一份。它在生成器受管清单内，确认生成后移入回收站（不是真删）；
                    发布定稿与版本交付的文件不受影响。
                  </p>
                ) : curFile.diff === 'unmanaged' ? (
                  <p className={s.empty}>
                    这一份不在生成器管理范围内，只展示、绝不删除。若它是你放进去的文件，会原样留在盘上。
                  </p>
                ) : curFile.diff === 'added' ? (
                  <p className={s.empty}>
                    盘上还没有这一份。确认生成后按当前源重算并写出，写完全量回读验证。
                  </p>
                ) : curPreview === null ? (
                  <p className={s.empty}>
                    {preview === null
                      ? '正在算这次生成会写什么……'
                      : '这一份这次不在生成队列里（已经生成过，本次不重写）。'}
                  </p>
                ) : view === 'diff' && curPreview.state === 'unchanged' ? (
                  <p className={s.empty}>
                    {curFile.pendingReason === 'registry'
                      ? '文件内容与重新渲染的一模一样，不会重写；只是目录登记（catalog.json）还停在旧哈希 —— 生成一次才会重算。'
                      : '这一份和磁盘上的一模一样，不会重写。'}
                  </p>
                ) : (
                  <DiffLines
                    lines={curPreview.lines}
                    mode={curPreview.state}
                    view={view}
                    fileName={curPreview.fileName}
                    innerRef={diffRef}
                  />
                )}
              </>
            ) : (
              <p className={s.empty}>左边点一份看它的变化</p>
            )}
          </div>
          {showRuler && <DiffRuler marks={marks} onJump={jumpToRow} />}
        </div>
      </div>
    </ModalC14>
  )
}
