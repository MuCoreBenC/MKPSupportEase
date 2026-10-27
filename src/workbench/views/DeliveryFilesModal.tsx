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
  DeliveryStatus,
  PreviewFile,
  PreviewReport,
} from '../api'
import { DiffLines, DiffRuler } from './DiffView'
import type { DiffViewMode } from './DiffView'
import { measureRulerMarks } from './diffRuler'
import type { RulerMark } from './diffRuler'
import c from '../c14.module.css'
import s from './DeliveryFilesModal.module.css'

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

function fmtSize(n: number | null): string {
  if (n === null) return '—'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

function baseName(rel: string): string {
  return rel.split('/').pop() ?? rel
}

/**
 * 一行交付文件的状态文案：按“谁写的”换词，别让发布定稿的那几份说「已生成」。
 *
 * **`pending` 要分三种原因说**（2026-10-20）：配方快照旧 / 目录登记没跟上 /
 * 重新渲染的字节不同。混成一句「待更新」时，同一屏上会出现「左边待更新、
 * 右边说一模一样不会重写」—— 那不是矛盾，是两把尺子。
 */
function stateText(f: DeliveryFile): string {
  if (f.state === 'missing') {
    return f.stage === 'publish' || f.stage === 'software' ? '还没发布' : '还没生成'
  }
  if (f.state === 'pending') {
    if (f.rel === 'catalog.json') return '待重新生成'
    if (f.stage === 'publish') return '待重新发布'
    if (f.pendingReason === 'registry') return '待重算目录'
    return '待更新'
  }
  return f.stage === 'publish' || f.stage === 'software' ? '已发布' : '已生成'
}

function stateClass(f: DeliveryFile): string {
  if (f.state === 'pending') return s.statePending
  if (f.state === 'missing') return s.stateMissing
  return s.stateOk
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

/** 清单的排序权重：待更新 / 未生成在前，已生成最后 */
function rankOf(state: DeliveryFile['state']): number {
  if (state === 'pending') return 0
  if (state === 'missing') return 1
  return 2
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
  const tomls = useMemo(
    () =>
      status.files
        .filter((f) => f.rel.startsWith('mkp/presets/'))
        .slice()
        .sort((a, b) => rankOf(a.state) - rankOf(b.state)),
    [status.files],
  )
  const generated = useMemo(
    () =>
      status.files.filter(
        (f) => !f.rel.startsWith('mkp/presets/') && f.stage === 'generate',
      ),
    [status.files],
  )
  const frozen = useMemo(
    () => status.files.filter((f) => f.stage !== 'generate'),
    [status.files],
  )
  const jsonCount = status.files.filter((f) => f.rel.endsWith('.json')).length
  const pending = status.files.filter((f) => f.state === 'pending').length
  const missing = status.files.filter((f) => f.state === 'missing').length

  /** 选中项：默认第一份待更新（没有就看第一条漂移，再没有就看第一份文件） */
  const defaultSel = useMemo(() => {
    const firstPending = status.files.find((f) => f.state === 'pending')
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
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' })
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
      title={name}
      onClick={() => setSel(key)}
    >
      <span className={`${s.itemState} ${stateCls}`}>{stateLabel}</span>
      <span className={s.itemName}>{name}</span>
      <span className={s.itemMeta}>{meta}</span>
    </button>
  )

  return (
    <ModalC14
      open
      size="xl"
      title="本次交付明细"
      subtitle={`${status.files.length} 个文件 · TOML ${tomls.length} 份 · JSON ${jsonCount} 份${
        pending > 0 ? ` · ${pending} 份待更新` : ''
      }${missing > 0 ? ` · ${missing} 份未生成 / 未发布` : ''}`}
      onClose={onClose}
      footer={
        <>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            disabled={generateTargets.length === 0}
            title={
              generateTargets.length === 0
                ? '没有待更新 / 未生成的版本，也没有要重算的目录'
                : '自动带上待更新与未生成的版本，先预演、确认后才写盘'
            }
            onClick={onGenerate}
          >
            {generateLabel}
          </button>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
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

      <div className={s.panes}>
        {/* 左：清单（漂移 / 预设产物 / 目录与清单 / 发布时定稿） */}
        <nav className={s.list} aria-label="本次交付文件">
          {status.drift.length > 0 && (
            <div className={s.groupHead}>资产字节漂移（{status.drift.length}）</div>
          )}
          {status.drift.map((d) =>
            row(
              driftKey(d.path),
              s.statePending,
              '待重新生成',
              d.path,
              driftText(d),
            ),
          )}

          {tomls.length > 0 && <div className={s.groupHead}>预设产物（{tomls.length}）</div>}
          {tomls.map((f) =>
            row(
              fileKey(f.rel),
              stateClass(f),
              stateText(f),
              baseName(f.rel) || f.rel,
              f.exist ? fmtSize(f.size) : '—',
            ),
          )}

          {generated.length > 0 && (
            <div className={s.groupHead}>目录与清单（{generated.length}）</div>
          )}
          {generated.map((f) =>
            row(
              fileKey(f.rel),
              stateClass(f),
              stateText(f),
              f.rel,
              f.exist ? fmtSize(f.size) : '—',
            ),
          )}

          {frozen.length > 0 && (
            <div className={s.groupHead}>发布时定稿（{frozen.length}）</div>
          )}
          {frozen.map((f) =>
            row(
              fileKey(f.rel),
              stateClass(f),
              stateText(f),
              f.rel,
              f.exist ? fmtSize(f.size) : '—',
            ),
          )}
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
                    <span className={stateClass(curFile)}>{stateText(curFile)}</span>
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

                {curFile.stage !== 'generate' ? (
                  <p className={s.empty}>
                    这一份由{STAGE_TEXT[curFile.stage]}负责 —— 本次生成不会动它。
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
