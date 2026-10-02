/*
 * 生成前确认（生成 diff 模态框）。
 *
 * # 为什么要它
 *
 * `wb_generate` 是**直接原子替换**的：勾中、点一下、磁盘上那份就被覆盖了，人没有
 * 任何机会先看一眼。作者原话：「如果它原本已经存在了呢，我不希望他直接就这样子点了
 * 生成就生成」。
 *
 * 于是中间插一步：点「生成」先跑 `wb_generate_preview`（**只算不写**，后端那条命令
 * 一个字节都不落），把这次要写的产物与磁盘上现存的逐份比，给出行级 diff。看过、确认，
 * 才真写。
 *
 * # 形态：清单 + 详情两栏（不是把 N 份竖着堆）
 *
 * 作者点名：「多很多个文件的话，那就不应该把所有文件都这样子竖着显示出来，而是应该
 * 用一个列表……点了列表就可以看仔细看每一个单独的」。
 *
 *   ┌ 文件清单 ┐ ┌ 选中那份的行级 diff ┐
 *   │ 有变化的在前 │ │ 变的行展开，其余「… N 行未变 …」可点开 │
 *   │ 无变化的折叠 │ │ │
 *   └──────────┘ └──────────────────────┘
 *
 * 三档状态（后端 `DiffState`）：
 *   · 新增    磁盘上还没有这一份 —— 正文**全绿**，不折叠（没变化可言）
 *   · 修改    有，但这次算出来的不一样 —— 行级 diff，一删一增（删在前、增在后，与 git 同序）
 *   · 无变化  逐字节相同，不会重写 —— 只在清单里占一行，详情写「没有变化」
 *
 * # 确认之后：框不关，换成结果页
 *
 * 作者：「模态框内直接换成结果页（写了几份 / 几份未变），你点「完成」再关」。
 * 结果数据就是 `wb_generate` 的返回值（`written` / `unchanged` / `skipped`）——
 * **本来就是它返回的**，不需要第二套口径。
 *
 * # 一个字节都不提前写
 *
 * 打开框 = 调一次预演（只读）。点「取消」= 什么都不发生。只有点「确认生成」才会走
 * `wb_generate`。所以「看一眼再决定」是真的没有副作用。
 */

import { useMemo, useState } from 'react'
import ModalC14 from '../c14/ModalC14'
import type { GenerateReport, PreviewDiffLine, PreviewFile, PreviewReport } from '../api'
import s from './GenerateDiffModal.module.css'

interface Props {
  /** 预演报告；null = 还没预演出来（正在算） */
  report: PreviewReport | null
  /** 预演失败（读命令报错）—— 有它就不给确认 */
  error: string | null
  /** 正在真写 */
  busy: boolean
  /** 生成结果；非 null = 已经生成完，框切成结果页 */
  done: GenerateReport | null
  onConfirm: () => void
  onClose: () => void
}

const STATE_TEXT: Record<PreviewFile['state'], string> = {
  added: '新增',
  modified: '修改',
  unchanged: '无变化',
}

/** 一份文件在清单里的排序权重：有变化（修改 / 新增）在前，无变化在最后 */
function rankOf(state: PreviewFile['state']): number {
  return state === 'unchanged' ? 1 : 0
}

export default function GenerateDiffModal({ report, error, busy, done, onConfirm, onClose }: Props) {
  /** 清单里选中的那份（默认第一份有变化的） */
  const [sel, setSel] = useState<string | null>(null)
  /** 无变化那一组展开没有 */
  const [showUnchanged, setShowUnchanged] = useState(false)

  const files = useMemo(() => report?.files ?? [], [report])

  /*
   * 清单顺序：有变化的在前（作者「变的先显示」），无变化的折成最后一组。
   * 组内按后端给的顺序（= 生成顺序），不重排 —— 免得"看到的第一份"和"真写的第一份"对不上。
   */
  const changed = files.filter((f) => rankOf(f.state) === 0)
  const unchanged = files.filter((f) => rankOf(f.state) === 1)

  /** 默认选中：第一份有变化的；没有就第一份无变化的 */
  const current = files.find((f) => f.fileName === sel) ?? changed[0] ?? unchanged[0] ?? null

  const total = files.length

  /* ---------- 结果页（已经生成完） ---------- */
  if (done !== null) {
    return (
      <ModalC14
        open
        size="md"
        title="生成完成"
        subtitle={`写出 ${done.written.length} 份 · ${done.unchanged.length} 份内容没变、跳过重写`}
        closeOnScrim={false}
        onClose={onClose}
        footer={
          <div className={s.foot}>
            <button type="button" className={s.btnPrimary} onClick={onClose}>
              完成
            </button>
          </div>
        }
      >
        <div className={s.result}>
          <p className={s.summary}>
            {done.written.length > 0 ? (
              <>
                真写了盘：<b>{done.written.length} 份</b>
              </>
            ) : (
              <>没有任何一份被重写（内容都和磁盘上的一致）</>
            )}
            {done.unchanged.length > 0 && <> · 内容没变、跳过：{done.unchanged.length} 份</>}
          </p>

          {done.written.length > 0 && (
            <section className={s.resultGroup}>
              <h4 className={s.resultHead}>已写出</h4>
              <ul className={s.resultList}>
                {done.written.map((uid) => (
                  <li key={uid} className={s.resultItem}>
                    <span className={s.dotWritten} aria-hidden />
                    {uid}
                  </li>
                ))}
              </ul>
            </section>
          )}

          {done.unchanged.length > 0 && (
            <section className={s.resultGroup}>
              <h4 className={s.resultHead}>内容没变（没重写）</h4>
              <ul className={s.resultList}>
                {done.unchanged.map((uid) => (
                  <li key={uid} className={s.resultItem}>
                    <span className={s.dotUnchanged} aria-hidden />
                    {uid}
                  </li>
                ))}
              </ul>
            </section>
          )}

          {done.skipped.length > 0 && (
            <section className={s.resultGroup}>
              <h4 className={s.resultHead}>跳过</h4>
              <ul className={s.resultList}>
                {done.skipped.map(([uid, why]) => (
                  <li key={uid} className={s.resultItem}>
                    <span className={s.dotSkipped} aria-hidden />
                    {uid} —— {why}
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>
      </ModalC14>
    )
  }

  /* ---------- 预演 / 确认页 ---------- */
  const blocked = report?.blocked ?? null
  const canConfirm = report !== null && error === null && blocked === null && !busy

  const subtitle =
    report === null
      ? '正在算出这次会写什么…'
      : blocked !== null
        ? '有阻断，不能生成'
        : total === 0
          ? '没有可生成的项'
          : `将写入 ${report.toWrite} 份 · ${unchanged.length} 份无变化`

  return (
    <ModalC14
      open
      size="lg"
      title="生成前确认"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <div className={s.foot}>
          <span className={s.footNote}>
            {changed.length > 0 ? (
              <>
                有变化的 <b>{changed.length}</b> 份会覆盖磁盘上同名文件
              </>
            ) : (
              <>没有哪一份会变 —— 点了也不会重写任何文件</>
            )}
          </span>
          <button type="button" className={s.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button type="button" className={s.btnPrimary} onClick={onConfirm} disabled={!canConfirm}>
            {busy ? '生成中…' : '确认生成'}
          </button>
        </div>
      }
    >
      {error !== null && (
        <p className={s.error} role="alert">
          预演没算出来：{error}
        </p>
      )}
      {blocked !== null && (
        <p className={s.error} role="alert">
          有阻断，生成会被拒：{blocked}
        </p>
      )}

      {report !== null && blocked === null && (
        <div className={s.panes}>
          {/* 左：文件清单 */}
          <nav className={s.list} aria-label="要生成的文件">
            {changed.map((f) => (
              <button
                key={f.fileName}
                type="button"
                className={f.fileName === current?.fileName ? `${s.item} ${s.itemOn}` : s.item}
                aria-current={f.fileName === current?.fileName}
                onClick={() => setSel(f.fileName)}
              >
                <span className={s.itemName}>{f.fileName}</span>
                <span className={s.itemMeta}>
                  <span className={s[`state_${f.state}`]}>{STATE_TEXT[f.state]}</span>
                  {f.state !== 'unchanged' && (
                    <span className={s.counts}>
                      <span className={s.add}>+{f.added}</span>
                      {f.removed > 0 && <span className={s.del}>−{f.removed}</span>}
                    </span>
                  )}
                </span>
              </button>
            ))}

            {unchanged.length > 0 && (
              <>
                <button
                  type="button"
                  className={s.groupToggle}
                  aria-expanded={showUnchanged}
                  onClick={() => setShowUnchanged((v) => !v)}
                >
                  <span className={s.caret} aria-hidden>
                    ▸
                  </span>
                  {unchanged.length} 份无变化
                </button>
                {showUnchanged &&
                  unchanged.map((f) => (
                    <button
                      key={f.fileName}
                      type="button"
                      className={f.fileName === current?.fileName ? `${s.item} ${s.itemOn}` : s.item}
                      aria-current={f.fileName === current?.fileName}
                      onClick={() => setSel(f.fileName)}
                    >
                      <span className={s.itemName}>{f.fileName}</span>
                      <span className={s.itemMeta}>
                        <span className={s.state_unchanged}>无变化</span>
                      </span>
                    </button>
                  ))}
              </>
            )}

            {total === 0 && <p className={s.empty}>没有可生成的项</p>}
          </nav>

          {/* 右：选中那份的详情 */}
          <div className={s.detail}>
            {current === null ? (
              <p className={s.empty}>左边点一份看它的差异</p>
            ) : (
              <>
                <header className={s.detailHead}>
                  <span className={s.detailName}>{current.fileName}</span>
                  <span className={s.detailMeta}>
                    <span className={s[`state_${current.state}`]}>{STATE_TEXT[current.state]}</span>
                    {current.state !== 'unchanged' && (
                      <span className={s.counts}>
                        <span className={s.add}>+{current.added}</span>
                        {current.removed > 0 && <span className={s.del}>−{current.removed}</span>}
                      </span>
                    )}
                  </span>
                </header>

                {current.state === 'unchanged' ? (
                  <p className={s.empty}>这一份和磁盘上的一模一样，不会重写。</p>
                ) : (
                  <DiffLines lines={current.lines} mode={current.state} />
                )}
              </>
            )}
          </div>
        </div>
      )}

      {report === null && error === null && <p className={s.empty}>正在算…</p>}
    </ModalC14>
  )
}

/**
 * 行级 diff 的展示：**变的行展开，其余折成「… N 行未变 …」**（可点开）。
 *
 * 这就是作者要的「变的先显示、其他省略、可以点开看」——在**一份文件内部**的粒度。
 * 折叠只折连续未变的段；段头写行数，点一下就摊开那一整段。
 */
function DiffLines({ lines, mode }: { lines: PreviewDiffLine[]; mode: PreviewFile['state'] }) {
  /* 把连续的同种行切成段：context 段可折叠，added / removed 段永远摊开 */
  const blocks = useMemo(() => {
    const out: { kind: PreviewDiffLine['kind']; lines: PreviewDiffLine[] }[] = []
    for (const l of lines) {
      const last = out[out.length - 1]
      if (last !== undefined && last.kind === l.kind) last.lines.push(l)
      else out.push({ kind: l.kind, lines: [l] })
    }
    return out
  }, [lines])

  /* 默认展开「未变段」的头尾各一小截？——不，作者要的是「变的先显示、其他省略」：
     默认**全折**，只留一行计数可点开。新增（mode==='added'）不折 —— 没有"未变"可言。 */
  const [open, setOpen] = useState<Record<number, boolean>>({})

  return (
    <div className={s.diff} data-mode={mode}>
      {blocks.map((b, i) => {
        if (b.kind === 'context') {
          const isOpen = open[i] ?? false
          return (
            <div key={i} className={s.foldBlock}>
              <button
                type="button"
                className={s.foldRow}
                aria-expanded={isOpen}
                onClick={() => setOpen((v) => ({ ...v, [i]: !isOpen }))}
              >
                <span className={s.caret} aria-hidden>
                  {isOpen ? '▾' : '▸'}
                </span>
                … {b.lines.length} 行未变 …
              </button>
              {isOpen &&
                b.lines.map((l) => (
                  <div key={l.no} className={s.lineCtx}>
                    <span className={s.gutter} aria-hidden />
                    <span className={s.lineNo}>{l.no}</span>
                    <code className={s.lineTxt}>{l.text === '' ? '\u00a0' : l.text}</code>
                  </div>
                ))}
            </div>
          )
        }
        return b.lines.map((l) => (
          <div key={`${i}-${l.no}`} className={l.kind === 'added' ? s.lineAdd : s.lineDel}>
            <span className={s.gutter} aria-hidden>
              {l.kind === 'added' ? '+' : '−'}
            </span>
            <span className={s.lineNo}>{l.no}</span>
            <code className={s.lineTxt}>{l.text === '' ? '\u00a0' : l.text}</code>
          </div>
        ))
      })}
    </div>
  )
}
