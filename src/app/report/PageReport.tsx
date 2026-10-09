/*
 * 报告页 —— **后处理执行报告与历史**（2026-10-09 接入，占位退场）。
 *
 * # 账在哪
 *
 * 后处理由本程序自己跑（首页「复制后处理脚本」贴进切片器的后处理栏，导出时触发）。
 * 每跑一次落三件套：`<用户根>/gcode_history/<日期>/<名>_{_original.gcode, .gcode, _meta.json}`，
 * 本页读的就是那棵树（Rust 侧 `ipc/report.rs`，只读）。**页面自己不造一个字的数据**。
 *
 * # 三态分得开
 *
 *   还没有记录   空列表 → 「还没有执行记录」+ 怎么才有记录的一句话
 *   读失败       命令抛错 → 整块错误态（带原因），**不装成空列表**
 *   记录本身坏了 照进列表（红标 + 原因）—— 藏起它就等于替钩子瞒了一次事故
 *
 * # 动作只有两件
 *
 *   刷新   重读一遍磁盘（钩子随时可能又跑了一次）
 *   展开   看这一条的详情（统计 / 打印时间 / 管线计时 / 警告 / 调用现场）
 *
 * 「确认导出 / 清理缓存」那些**没有真实实现的动作不摆**（纪律：没有实现不给入口）。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { ReportDetail, ReportSummary } from '../../api'
import NoteBar from '../shared/NoteBar'
import type { Note } from '../shared/note'
import { Btn } from '../ui/Controls'
import s from './PageReport.module.css'

/** 统计里挑给用户看的几格（键与 `_meta.json` 的 `detail.stats` 同名） */
const STAT_CELLS: [string, string][] = [
  ['towerHeight', '塔高 mm'],
  ['glueLayerCount', '涂胶层'],
  ['totalLayerNumber', '总层数'],
  ['maxZHeight', '最高 Z mm'],
  ['originalPrintTime', '切片器预估'],
]

function elapsedText(ms: number | null): string {
  if (ms === null) return '—'
  if (ms < 1000) return `${ms} 毫秒`
  const sec = ms / 1000
  if (sec < 60) return `${sec.toFixed(1)} 秒`
  const m = Math.floor(sec / 60)
  return `${m} 分 ${Math.round(sec - m * 60)} 秒`
}

function printTimeText(sec: number | undefined): string {
  if (sec === undefined || !Number.isFinite(sec)) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.round((sec % 3600) / 60)
  return h > 0 ? `${h} 小时 ${m} 分` : `${m} 分`
}

function shaShort(sha: string | undefined): string {
  return sha === undefined ? '—' : sha.slice(0, 12)
}

/** 打印时间还没补上时的说法 —— 不许拿 0 凑数（延后估算的那几百毫秒里它是 null） */
function printTimeStatusText(status: string | null | undefined): string {
  if (status === 'computing') return '正在估算…'
  if (status === 'failed') return '估算失败'
  return '—'
}

export default function PageReport() {
  /* null = 还在读；读失败是另一格（err），不与空列表混 */
  const [rows, setRows] = useState<ReportSummary[] | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  const [openId, setOpenId] = useState<string | null>(null)
  const [detail, setDetail] = useState<ReportDetail | null>(null)
  const [detailErr, setDetailErr] = useState<string | null>(null)

  const read = useCallback(() => {
    setErr(null)
    setRows(null)
    api.getReportList().then(
      (list) => setRows(list),
      (e: unknown) => {
        setRows([])
        setErr(errorText(e))
      },
    )
  }, [])

  useEffect(() => {
    read()
  }, [read])

  /* 展开一条：详情按需读（列表只带摘要，展开才读整份 meta） */
  const toggle = (row: ReportSummary) => {
    if (openId === row.id) {
      setOpenId(null)
      setDetail(null)
      setDetailErr(null)
      return
    }
    setOpenId(row.id)
    setDetail(null)
    setDetailErr(null)
    api.getReportDetail(row.id).then(
      (got) => setDetail(got),
      (e: unknown) => setDetailErr(errorText(e)),
    )
  }

  return (
    <section className={s.page}>
      <header className={s.head}>
        <h2 className={s.title}>后处理执行报告与历史</h2>
        <p className={s.sub}>
          后处理每跑一次（切片器导出时按「复制后处理脚本」那条命令跑），这里多一条记录。
        </p>
        <div className={s.headActions}>
          <Btn
            onClick={() => {
              read()
              setNote({ text: '正在重读执行记录…', bad: false })
            }}
          >
            刷新
          </Btn>
        </div>
      </header>

      <NoteBar note={note} onClose={() => setNote(null)} />

      {err !== null ? (
        <p className={s.err} role="alert">
          执行记录读不出来：{err}
        </p>
      ) : rows === null ? (
        <p className={s.empty}>正在读执行记录…</p>
      ) : rows.length === 0 ? (
        <p className={s.empty}>
          还没有执行记录 —— 把首页「复制后处理脚本」贴进切片器的后处理脚本栏，
          导出打印文件时会自动生成第一条。
        </p>
      ) : (
        <ul className={s.list}>
          {rows.map((row) => (
            <li key={row.id} className={s.row} data-open={openId === row.id}>
              <button type="button" className={s.rowHead} onClick={() => toggle(row)}>
                <span className={row.ok ? s.ok : s.bad}>{row.ok ? '✓ 成功' : '✗ 失败'}</span>
                <span className={s.when}>{row.startedAt ?? row.day}</span>
                <span className={s.file} title={row.gcodeName ?? ''}>
                  {row.gcodeName ?? '（记录里没写文件名）'}
                </span>
                <span className={s.meta}>
                  {[row.presetName, row.machineType, elapsedText(row.elapsedMs)]
                    .filter((x): x is string => x !== null && x !== undefined)
                    .join(' · ')}
                  {row.warningCount > 0 ? ` · ⚠ ${row.warningCount}` : ''}
                </span>
              </button>

              {!row.ok && row.error !== null && <p className={s.rowErr}>{row.error}</p>}

              {openId === row.id && (
                <div className={s.detail}>
                  {detailErr !== null ? (
                    <p className={s.err} role="alert">
                      详情读不出来：{detailErr}
                    </p>
                  ) : detail === null ? (
                    <p className={s.empty}>正在读这一条的详情…</p>
                  ) : (
                    <ReportDetailView detail={detail} />
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function ReportDetailView({ detail }: { detail: ReportDetail }) {
  const stats = detail.stats ?? {}
  return (
    <div className={s.detailBody}>
      {detail.warnings.length > 0 && (
        <div className={s.block}>
          <h3 className={s.blockTitle}>警告（{detail.warnings.length}）</h3>
          <ul className={s.warnList}>
            {detail.warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}

      <div className={s.block}>
        <h3 className={s.blockTitle}>统计</h3>
        <dl className={s.cells}>
          <div className={s.cell}>
            <dt>预计打印时间</dt>
            <dd>
              {detail.printTime?.totalSeconds === undefined
                ? printTimeStatusText(detail.printTimeStatus)
                : printTimeText(detail.printTime?.totalSeconds)}
            </dd>
          </div>
          {STAT_CELLS.map(([key, label]) => (
            <div className={s.cell} key={key}>
              <dt>{label}</dt>
              <dd>{stats[key] === undefined ? '—' : String(stats[key])}</dd>
            </div>
          ))}
          {detail.printTime?.segments !== undefined && (
            <div className={s.cell}>
              <dt>耗时段数</dt>
              <dd>{detail.printTime.segments}</dd>
            </div>
          )}
        </dl>
      </div>

      {detail.pipeline.length > 0 && (
        <div className={s.block}>
          <h3 className={s.blockTitle}>管线计时</h3>
          <ol className={s.steps}>
            {detail.pipeline.map((step, i) => (
              <li key={i}>
                <span className={s.stepName}>{step.id ?? `第 ${i + 1} 步`}</span>
                <span className={s.stepMsg}>{step.message ?? ''}</span>
                <span className={s.stepMs}>{elapsedText(step.elapsedMs ?? null)}</span>
              </li>
            ))}
          </ol>
        </div>
      )}

      <div className={s.block}>
        <h3 className={s.blockTitle}>文件</h3>
        <dl className={s.facts}>
          <dt>输入</dt>
          <dd>
            {detail.inputFile?.path ?? detail.gcodePath ?? '—'}
            {detail.inputFile?.sizeBytes !== undefined &&
              ` · ${(detail.inputFile.sizeBytes / 1024 / 1024).toFixed(1)} MB`}
            {detail.inputFile?.lines !== undefined && ` · ${detail.inputFile.lines} 行`}
            {` · sha ${shaShort(detail.inputFile?.sha256)}`}
          </dd>
          <dt>输出</dt>
          <dd>
            {detail.outputFile?.path ?? '—'}
            {detail.outputFile?.sizeBytes !== undefined &&
              ` · ${(detail.outputFile.sizeBytes / 1024 / 1024).toFixed(1)} MB`}
            {` · sha ${shaShort(detail.outputFile?.sha256)}`}
          </dd>
          <dt>预设</dt>
          <dd>{detail.presetPath ?? detail.summary.presetName ?? '—'}</dd>
          {detail.invocation?.exe !== undefined && (
            <>
              <dt>调用</dt>
              <dd className={s.mono}>
                {detail.invocation.exe}
                {detail.invocation.args !== undefined ? ` ${detail.invocation.args.join(' ')}` : ''}
              </dd>
            </>
          )}
        </dl>
      </div>
    </div>
  )
}
