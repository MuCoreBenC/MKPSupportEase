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
 *   ┌ 文件清单 ┐ ┌ 选中那份的 diff / 全文 ┐
 *   │ 有变化的在前 │ │ 整份摊开（对比 = 红绿底，完整 = 素底）│
 *   │ 无变化的折叠 │ │ │
 *   └──────────┘ └──────────────────────┘
 *
 * # 清单分两组：预设产物 / 目录与清单（作者 2026-10-07）
 *
 * 生成不只写 toml —— 它同时重算目录与清单（`content` 那三份 + `catalog.json`）。
 * 作者原话：「我希望到时候它还能显示一个会变化的 Json……也可以显示出来，也是在这个
 * 生成的这个模态框里面显示出来」「可以再多分一栏出来」。于是清单里多出一组
 * **目录与清单**（后端的 `PreviewReport.aux`），点开与产物同一种看法（完整 / 对比）。
 * 两组分开列：附属文件不带「版本」概念，混在产物里读不出谁是谁。
 *
 * # 「完整 / 对比」切换（作者 2026-10-02 加；**默认改成「对比」2026-10-07**）
 *
 * 作者原话：「diff 的效果我不喜欢……加一个切换吧，就看完整的不看对比的」。于是标题行
 * 右侧多一个两段切换（`Modal` 的 `headerExtra`）：
 *   · **对比（默认）** —— 上面的行级 diff。2026-10-07 作者改口：「默认显示对比里面的好吧」；
 *   · 完整 —— 选中那份的**新文件全文**，逐行带新文件行号，没有红绿底。
 * 行号口径也顺手修了：后端 `removed` 行记的是**旧文件行号**、其余记**新文件行号**，
 * 以前混排在同一列里（39、40 然后跳 41、42），人看不懂 —— 现在 `removed` 行行号
 * 槽留空（「−」符已经说明了它是删掉的），列里只剩一套「新文件第几行」的语义。
 *
 * 对比视图**不省略**（作者同日晚又改的口径：「对比的不要省略吧，还是就像这种一样
 * 正常的」—— 指的是编辑器里那种整份摊开的 diff）：所有行平铺，未变的也在，
 * 之前那套「… N 行未变 …」折叠删了。跟「完整」的差别只剩红绿底。
 *
 * 三档状态（后端 `DiffState`）：
 *   · 新增    磁盘上还没有这一份 —— 正文**全绿**，不折叠（没变化可言）
 *   · 修改    有，但这次算出来的不一样 —— 行级 diff，一删一增（删在前、增在后，与 git 同序）
 *   · 无变化  正文相同（只有头部时间戳那行会不同），不会重写。判据与真生成的
 *             「跳过」同一道（`same_payload`）：预演说「要写」而真生成跳过，
 *             「将写入 N 份」就是假的。
 *             **详情照给全文**（作者 2026-10-07：「就算它一模一样不会重写，我也希望
 *             看一个完整的，然后对比的时候才写这个文案」）—— 选中它：「完整」视图
 *             摊开整份素底正文，「对比」视图才写「这一份和磁盘上的一模一样，不会重写」
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
import { CodeLine } from '../c14/CodeText'
import { highlightLines, langOf } from '../c14/syntax'
import { deliveryStageText as STAGE_TEXT } from '../c14/labels'
import type { GenerateReport, PreviewDiffLine, PreviewFile, PreviewReport } from '../api'
import c from '../c14.module.css'
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

/** 详情区显示哪种：完整正文（默认）或与磁盘现存的行级对比 */
type DetailView = 'full' | 'diff'

export default function GenerateDiffModal({ report, error, busy, done, onConfirm, onClose }: Props) {
  /** 清单里选中的那份（默认第一份有变化的） */
  const [sel, setSel] = useState<string | null>(null)
  /** 无变化那一组展开没有 */
  const [showUnchanged, setShowUnchanged] = useState(false)
  /** 附属文件里"无变化"那一组展开没有（与产物那组各管各的） */
  const [showAuxUnchanged, setShowAuxUnchanged] = useState(false)
  /** 详情视图：作者 2026-10-07 —— **默认看对比**，想看整份原文再切「完整」 */
  const [view, setView] = useState<DetailView>('diff')

  const files = useMemo(() => report?.files ?? [], [report])
  const aux = useMemo(() => report?.aux ?? [], [report])

  /*
   * 清单顺序：有变化的在前（作者「变的先显示」），无变化的折成最后一组。
   * 组内按后端给的顺序（= 生成顺序），不重排 —— 免得"看到的第一份"和"真写的第一份"对不上。
   */
  const changed = files.filter((f) => rankOf(f.state) === 0)
  const unchanged = files.filter((f) => rankOf(f.state) === 1)
  /*
   * 附属文件（目录与清单）再分两截：
   *   · `generate` —— 生成会一并重算的那几份，照常给「要不要写」的结论；
   *   · 其余（发布定稿 / 软件发布链）—— **本次生成不动**，只照实列出来看得见。
   */
  const auxGen = aux.filter((f) => f.stage === 'generate')
  const auxFrozen = aux.filter((f) => f.stage !== 'generate')
  const auxChanged = auxGen.filter((f) => rankOf(f.state) === 0)
  const auxUnchanged = auxGen.filter((f) => rankOf(f.state) === 1)

  /** 默认选中：第一份有变化的（产物优先，再轮到附属文件）；没有就第一份无变化的 */
  const current =
    [...files, ...aux].find((f) => f.fileName === sel) ??
    changed[0] ??
    auxChanged[0] ??
    unchanged[0] ??
    auxUnchanged[0] ??
    null

  /**
   * 会写盘 / 不会写盘的总份数（产物 + **生成会写的**附属文件）。
   * 「本次生成不动」的那几份不算 —— 把它们算进"将写入 N 份"就是虚报。
   */
  const willWrite = (report?.toWrite ?? 0) + auxChanged.length
  const stays = (report?.unchanged ?? 0) + auxUnchanged.length

  const total = files.length + aux.length

  /** 清单里的一行。产物与附属文件同一种长相 —— 两处各写一份迟早会长成两种 */
  const item = (f: PreviewFile) => {
    const frozen = f.stage !== 'generate'
    return (
      <button
        key={f.fileName}
        type="button"
        className={`${f.fileName === current?.fileName ? `${s.item} ${s.itemOn}` : s.item}${frozen ? ` ${s.itemFrozen}` : ''}`}
        aria-current={f.fileName === current?.fileName}
        title={f.fileName}
        onClick={() => setSel(f.fileName)}
      >
        <span className={s.itemName}>{f.fileName}</span>
        <span className={s.itemMeta}>
          {frozen ? (
            /* 生成不动它 —— 说「无变化」会让人以为"生成算过它"，那是假的 */
            <span className={s.stateFrozen}>{STAGE_TEXT[f.stage]}</span>
          ) : (
            <>
              <span className={s[`state_${f.state}`]}>{STATE_TEXT[f.state]}</span>
              {f.state !== 'unchanged' && (
                <span className={s.counts}>
                  <span className={s.add}>+{f.added}</span>
                  {f.removed > 0 && <span className={s.del}>−{f.removed}</span>}
                </span>
              )}
            </>
          )}
        </span>
      </button>
    )
  }

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
          <button type="button" className={`${c.btn} ${c.btnPrimary}`} onClick={onClose}>
            完成
          </button>
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
          : `将写入 ${willWrite} 份 · ${stays} 份无变化`

  return (
    <ModalC14
      open
      size="xl"
      title="生成前确认"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      headerExtra={
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
      }
      footer={
        <>
          <span className={s.footNote}>
            {willWrite > 0 ? (
              <>
                有变化的 <b>{willWrite}</b> 份会覆盖磁盘上同名文件
              </>
            ) : (
              <>没有哪一份会变 —— 点了也不会重写任何文件</>
            )}
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            onClick={onConfirm}
            disabled={!canConfirm}
          >
            {busy ? '生成中…' : '确认生成'}
          </button>
        </>
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
          {/* 左：文件清单（产物一组 / 目录与清单一组） */}
          <nav className={s.list} aria-label="要生成的文件">
            {files.length > 0 && <div className={s.listGroup}>预设产物</div>}
            {changed.map(item)}
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
                {showUnchanged && unchanged.map(item)}
              </>
            )}

            {auxGen.length > 0 && <div className={s.listGroup}>目录与清单</div>}
            {auxChanged.map(item)}
            {auxUnchanged.length > 0 && (
              <>
                <button
                  type="button"
                  className={s.groupToggle}
                  aria-expanded={showAuxUnchanged}
                  onClick={() => setShowAuxUnchanged((v) => !v)}
                >
                  <span className={s.caret} aria-hidden>
                    ▸
                  </span>
                  {auxUnchanged.length} 份无变化
                </button>
                {showAuxUnchanged && auxUnchanged.map(item)}
              </>
            )}

            {/*
              生成**不动**的那几份（manifest / source 归发布、release.json 归软件发布链）。
              作者 2026-10-07 问「那我那个生成的里面怎么没有」—— ②卡列全了七份，这里也列全，
              只是标明这次不碰它们。
            */}
            {auxFrozen.length > 0 && (
              <div className={s.listGroup}>发布时定稿（本次生成不动）</div>
            )}
            {auxFrozen.map(item)}

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
                    {current.stage !== 'generate' ? (
                      <span className={s.stateFrozen}>{STAGE_TEXT[current.stage]}</span>
                    ) : (
                      <>
                        <span className={s[`state_${current.state}`]}>{STATE_TEXT[current.state]}</span>
                        {current.state !== 'unchanged' && (
                          <span className={s.counts}>
                            <span className={s.add}>+{current.added}</span>
                            {current.removed > 0 && <span className={s.del}>−{current.removed}</span>}
                          </span>
                        )}
                      </>
                    )}
                  </span>
                </header>

                {/* 「对比」视图对着两份"没有 diff 可比"的东西各说一句自己的话；
                    「完整」视图一律摊开原文（含盘上现状） */}
                {view === 'diff' && current.stage !== 'generate' ? (
                  <p className={s.empty}>
                    这一份由{STAGE_TEXT[current.stage]}负责 —— 本次生成不会动它。
                  </p>
                ) : view === 'diff' && current.state === 'unchanged' ? (
                  <p className={s.empty}>这一份和磁盘上的一模一样，不会重写。</p>
                ) : (
                  <DiffLines
                    lines={current.lines}
                    mode={current.state}
                    view={view}
                    fileName={current.fileName}
                  />
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
 * 详情正文：`view === 'full'` 给新文件全文，`'diff'` 给整份摊开的对比（见文件头注释）。
 *
 * 行号口径：`removed` 行在后端记的是**旧文件**行号，其余记**新文件**行号 ——
 * 所以 `removed` 行的行号槽留空（「−」符已经说明它是删掉的），保证一列里
 * 只有一套「新文件第几行」的语义，不再出现 39、40 跳 41、42 的看不懂。
 *
 * **语法上色**（2026-10-07）：`fileName` 只为认文法（`.json` / `.toml`）——
 * 作者原话「不要都是黑色的字，变一下颜色，像 VSCode 里面的插件一样」。
 * 上色类名与只看盘上原文那个块共用一份（`c14.module.css` 的 `tok_*`）。
 */
function DiffLines({
  lines,
  mode,
  view,
  fileName,
}: {
  lines: PreviewDiffLine[]
  mode: PreviewFile['state']
  view: DetailView
  fileName: string
}) {
  /*
   * 完整视图的行 = 丢弃 `removed`（旧文件才有的行），剩下的 context / added
   * 就是新文件全文；后端按 diff 序输出（删在前、增在后），这里按新行号排回去。
   * 新文件行号唯一 → 可以直接当 key。
   */
  const fullRows = useMemo(
    () =>
      lines
        .filter((l) => l.kind !== 'removed')
        .slice()
        .sort((a, b) => a.no - b.no),
    [lines],
  )

  /*
   * 两套视图的 token 各自算：`full` 丢了 removed 行，行数与后端给的那一份对不上 ——
   * 共用一个 token 列表就会串行（一行的颜色贴到另一行上）。
   * 认不出文法（`null`）⇒ 原样渲染，一个字都不动。
   */
  const lang = langOf(fileName)
  const fullTokens = useMemo(
    () => (lang === null ? null : highlightLines(fullRows.map((l) => l.text), lang)),
    [lang, fullRows],
  )
  const diffTokens = useMemo(
    () => (lang === null ? null : highlightLines(lines.map((l) => l.text), lang)),
    [lang, lines],
  )

  if (view === 'full') {
    return (
      <div className={s.diff}>
        {fullRows.map((l, i) => (
          <div key={l.no} className={s.lineCtx}>
            <span className={s.gutter} aria-hidden />
            <span className={s.lineNo}>{l.no}</span>
            <code className={s.lineTxt}>
              <CodeLine line={l.text} tokens={fullTokens?.[i]} />
            </code>
          </div>
        ))}
      </div>
    )
  }

  /* 对比视图：**不省略** —— 后端给的每一行（含未变的 context）都平铺出来，
     与「完整」的差别只剩红绿底。行序就是后端的 diff 序（删在前、增在后，与 git 同序）。 */
  return (
    <div className={s.diff} data-mode={mode}>
      {lines.map((l, i) => (
        <div key={`${i}-${l.kind}-${l.no}`} className={l.kind === 'added' ? s.lineAdd : l.kind === 'removed' ? s.lineDel : s.lineCtx}>
          <span className={s.gutter} aria-hidden>
            {l.kind === 'added' ? '+' : l.kind === 'removed' ? '−' : ''}
          </span>
          <span className={s.lineNo}>{l.kind === 'removed' ? '' : l.no}</span>
          <code className={s.lineTxt}>
            <CodeLine line={l.text} tokens={diffTokens?.[i]} />
          </code>
        </div>
      ))}
    </div>
  )
}
