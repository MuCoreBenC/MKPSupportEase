/**
 * 「生成与发布」页 —— C14 版式移植（feat/b05-14b-c14-port，build 视角落地）。
 *
 * C13 的两页在这一页合成一页（作者：「发布中心和整包检查与生成有点重复了」），
 * 版式沿用左右分布（作者：「左边是那些事项、未完成的提示，右边才是主要操作」）：
 *
 *   左栏  检查        一份账（wb_preflight）：能不能生成 + 交出去会不会缺东西
 *   右栏  ① 生成      勾中版本按已保存的配方生成（原子整批，字节没变不重写）
 *         ② 客户端数据包   只读：包里有什么 + 兼容声明（渠道 / 最低客户端版本）
 *         ③ 发布      检查过了才亮 —— 后端还有一道硬闸（残留拦截在发布内部）
 *         ④ 基线      产物 vs 对照基线，人看过 diff 才同步（14.9）
 *         ⑤ 残留      交付目录里「不在本次交付集合内」的文件，清理走 .trash 回收
 *         ⑥ 回收站    删掉的生成快照（只读清单）
 *         ⑦ 子目录职责  workbench/ 五个子目录谁写谁读（14.7，后端给的一句话）
 *
 * # 闸门按钮（14.2）与后端硬闸的行为一致
 *
 * 生成 / 发布按钮的禁用读 `wb_preflight` 的阻断计数（每次写完外壳重取），
 * 后端在 `wb_generate` / `wb_publish` 开头**还会再查一遍**—— 前端只是把同一份
 * 判据提前画在按钮上，不是第二条闸。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的
 *
 *  - **包版本 / 最低客户端版本不是输入框**：发布沿用上游 compat 声明的
 *    channel / version / minimumClient（`PublishMeta` 从上游清单带过去）。
 *    「最低客户端版本未声明」在左栏是一条待办，那句话自己写着
 *    「这不是我们该填的空」—— 所以这里只读，不造一个输入框出来。
 *  - **检查只有一份账**：产品的 wb_preflight 是一份报告（引用 / 参数 / 唯一性 /
 *    孤儿 + 清单↔配方对齐），不像原型分 checkIssues / publishIssues 两份。
 *  - **行上没有「未保存」档**：BuildRow 不带 savedYet；未保存时按钮先走
 *    「保存再生成」（一个按钮两个动作，顺序规定死 —— 机型页同款）。
 *  - **没有内容指纹**：原型的 inputsHash 在产品侧没有对应物；产物的新旧由
 *    每一版自己的快照比对判（BuildRow.state），「② 卡」不重复一个全局哈希。
 *  - **恢复配方（wb_revert_preview）不在这页**：按版本走机型页的版本卡 ——
 *    恢复的是「某一版」的配方，不是整包的动作。
 */
import { Fragment, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  BaselineDiffEntry,
  BookView,
  Boot,
  Issue,
  IssueReport,
  PublishReport,
  Words,
} from '../api'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import s from '../c14.module.css'

interface Props {
  boot: Boot
  book: BookView
  words: Words
  /** 外壳每次写完重取的检查报告 —— 闸门按钮与左栏都读它 */
  report: IssueReport | null
  /** 外壳的「后端状态变过了」计数：生成 / 撤销之后重取基线与残留 */
  tick: number
  onGoto: (view: string, focus?: GotoFocus) => void
  /** 草稿写入口（生成记录 MarkBuilt 从这里过 —— 不可撤销，不进栈） */
  onApply: (label: string, patches: import('../api').Patch[]) => Promise<void>
  /** 先落盘再生成的那个「落盘」 */
  onSave: () => Promise<boolean>
  /** 生成之后让外壳重取整本（buildRows 的状态跟着走） */
  onBookRefresh: () => void
}

/** 检查项的「去处理」映射。产品的一份账里，不是每一条都有地方可去 */
function gotoOf(i: Issue): { view: string; focus?: GotoFocus } | null {
  if (i.id === 'bundle.orphan_files') {
    /* C14 第二十七轮的修正：孤儿文件去**资产库**并替人筛好「可选」——
       套餐页里根本没有那些文件，跳过去等于没指路 */
    return { view: 'assets', focus: { machineId: null, uid: null, key: 'optional' } }
  }
  switch (i.at.view) {
    case 'params':
    case 'fields':
      return { view: 'params', focus: { machineId: i.at.machineId ?? '', uid: i.at.uid, key: i.at.key } }
    case 'menu':
      return { view: 'bundles', focus: { machineId: null, uid: i.at.uid, key: null } }
    case 'stock':
      return { view: 'assets', focus: { machineId: null, uid: i.at.uid, key: null } }
    case 'fallback':
      return { view: 'assets', focus: { machineId: null, uid: null, key: null } }
    default:
      return null // build 视角的问题在本页就地滚过去，不跳页
  }
}

export default function BuildPage({ boot, book, words, report, tick, onGoto, onApply, onSave, onBookRefresh }: Props) {
  const rows = book.buildRows
  const [picked, setPicked] = useState<Record<string, boolean>>({})
  const [openRow, setOpenRow] = useState<string | null>(null)
  const [lastPublish, setLastPublish] = useState<PublishReport | null>(null)
  const [baseline, setBaseline] = useState<BaselineDiffEntry[] | null>(null)
  const [strays, setStrays] = useState<string[] | null>(null)
  const [trash, setTrash] = useState<Awaited<ReturnType<typeof wb.trash>> | null>(null)

  const blocked = (report?.blocks ?? 0) > 0
  const dirty = book.dirtyCount > 0
  const ids = Object.keys(picked).filter((k) => picked[k])

  /* 基线 / 残留 / 回收站：进页取一次，外壳每写一次（生成会改产物）重取 */
  useEffect(() => {
    void (async () => {
      try {
        const [b, st, t] = await Promise.all([wb.baselineDiff(), wb.distStrays(), wb.trash()])
        setBaseline(b)
        setStrays(st)
        setTrash(t)
      } catch (e) {
        toasts.push(isAppError(e) ? e.message : String(e))
      }
    })()
  }, [tick])

  /*
   * 「去处理」的定位（C14 第二十四轮）：滚动到目标模块 + 闪烁两秒。
   * 闪烁不走 React 状态 —— 瞬态 DOM 效果进状态只会逼全页重渲染；
   * classList 直改 + 1.8s 后摘掉。
   */
  const goTarget = (sel: string) => {
    const el = document.getElementById(sel)
    if (!el) return
    el.scrollIntoView({ block: 'center' })
    const isRow = sel.startsWith('t-build-') && sel !== 't-build'
    const cls = isRow ? s.rowFlash : s.flashIt
    el.classList.remove(s.flashIt, s.rowFlash)
    void (el as HTMLElement).offsetWidth /* 打断正在播的同款动画，从头再闪 */
    el.classList.add(cls)
    window.setTimeout(() => el.classList.remove(s.flashIt, s.rowFlash), 1800)
  }

  const issueGoto = (i: Issue) => {
    /* 本页的（build 视角）就地滚；先落盘的行没生成过，锚点就是 ① 卡 */
    if (i.at.view === 'build') {
      goTarget(i.at.uid ? `t-build-${i.at.uid}` : 't-build')
      return
    }
    const g = gotoOf(i)
    if (g) onGoto(g.view, g.focus)
  }

  /** 生成。勾中的行按当前已保存的配方整批生成 —— 后端原子：任一项算不出则整批不动 */
  const generate = async () => {
    if (!ids.length) return
    /* 草稿不干净先落盘 —— 生成读的是**已保存**的配方（一个按钮两个动作，顺序规定死） */
    if (dirty && !(await onSave())) return
    try {
      const rep = await wb.generate({ picked: ids })
      // 生成记录走唯一写入口落进草稿（不可撤销 —— 它是记录，不是编辑）
      await onApply(`生成记录：${rep.written.length + rep.unchanged.length} 份`, [rep.mark])
      const parts = [
        rep.written.length > 0 ? `写出 ${rep.written.length} 份` : null,
        rep.unchanged.length > 0 ? `${rep.unchanged.length} 份内容没变、跳过重写` : null,
        ...rep.skipped.map(([uid, why]) => `跳过 ${uid}（${why}）`),
      ].filter((x): x is string => x !== null)
      toasts.push(`已生成：${parts.join('；') || '没有可生成的项'}`)
      setPicked({})
      onBookRefresh()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 发布。后端还有两道硬闸：检查阻断 + 残留拦截（一个字节都不许在带残留时写出） */
  const publish = async () => {
    try {
      const rep = await wb.publish()
      setLastPublish(rep)
      toasts.push(`已发布 ${rep.files} 个文件到交付目录（最低客户端版本：${rep.minimumClient ?? '未声明'}）`)
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 同步对照基线（14.9 第②步）。前提：人看过上面的 diff 清单 */
  const syncBaseline = async () => {
    try {
      const n = await wb.syncBaseline()
      toasts.push(n > 0 ? `已同步 ${n} 份基线（内容相同的自动跳过）` : '基线本来就一致，没有要写的')
      setBaseline(await wb.baselineDiff())
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 清理交付残留。走 .trash/dist/<stamp>/ 回收，不直接删 */
  const cleanStrays = async () => {
    try {
      const n = await wb.cleanDistStrays()
      toasts.push(n > 0 ? `已把 ${n} 个残留移入回收站 —— 重新发布即可补齐` : '没有残留')
      setStrays(await wb.distStrays())
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  const baselineDirty = baseline?.filter((b) => b.status !== 'same').length ?? 0

  return (
    <div className={s.colsRev}>
      {/* —— 检查：左栏的事项清单 —— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <h2>检查</h2>
          <span className={s.cardNote}>
            {report
              ? `阻断 ${report.blocks} · 待办 ${report.todos} · 提示 ${report.hints}`
              : '正在读……'}
          </span>
        </div>
        <div className={s.cardBody}>
          {report && report.issues.length > 0 ? (
            report.issues.map((i) => (
              <div
                key={i.id}
                className={`${s.issue} ${i.severity === 'block' ? s.issueBlock : ''}`}
              >
                <span className={`${s.tag} ${s[`tagSev${i.severity[0].toUpperCase()}${i.severity.slice(1)}` as keyof typeof s] ?? ''}`}>
                  {i.severity === 'block' ? '阻断' : i.severity === 'todo' ? '待办' : '提示'}
                </span>
                <span className={s.issueTxt}>
                  <b>{i.title}</b>
                  <span>{i.detail}</span>
                </span>
                {/*
                  每一条都要说清去哪儿处理（issues.rs 的规矩）。最低客户端版本那一条
                  的说明自己写着「这不是我们该填的空」—— 它没有「去处理」，不摆按钮。
                */}
                {i.id !== 'compat.minimum_client' && (
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    onClick={() => issueGoto(i)}
                  >
                    去处理
                  </button>
                )}
              </div>
            ))
          ) : (
            <div className={s.emptyHint}>{report ? report.emptyHint : '正在读检查报告……'}</div>
          )}
        </div>
      </div>

      <div className={s.flow}>
        {/* —— ① 生成 —— */}
        <div id="t-build" className={s.card}>
          <div className={s.cardHead}>
            <h2>① 生成</h2>
            <span className={s.cardNote}>判据是快照比对，不看文件时间</span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              勾哪几个版本，就按它们<strong>当前已保存的配方</strong>整批生成。
              后端先全部算完、任一项算不出来则整批不动；字节没变的产物不重写。
            </p>
            {dirty && (
              <p className={s.note} style={{ marginTop: 0 }}>
                有<strong>未保存</strong>的改动 —— 点「生成」会先替你保存，成了才生成
                （生成读的是落盘的那份，不是屏幕上的这份）。
              </p>
            )}
            {rows.map((r) => (
              <Fragment key={r.uid}>
                <div
                  id={`t-build-${r.uid}`}
                  onClick={() => setOpenRow(openRow === r.uid ? null : r.uid)}
                  style={{ cursor: 'pointer' }}
                  className={s.row}
                >
                  <input
                    type="checkbox"
                    checked={!!picked[r.uid]}
                    disabled={!r.buildable}
                    title={r.disabledReason ?? undefined}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => setPicked({ ...picked, [r.uid]: e.target.checked })}
                    aria-label={`选择 ${r.uid}`}
                  />
                  <span className={`${s.mono} ${s.rowMeta}`}>{r.uid}</span>
                  <span className={s.rowName}>{r.name}</span>
                  <span
                    className={`${s.tag} ${STATE_TAG[r.state]}`}
                    title={words.build[r.state].explain ?? undefined}
                  >
                    {words.build[r.state].label}
                  </span>
                  <span className={s.grow} />
                  <span className={s.rowMeta}>{r.lastBuild ?? '—'}</span>
                </div>
                {/* 详情是行的**兄弟**不是行里右侧的一个格（C14 第二十七轮） */}
                {openRow === r.uid && (
                  <div className={s.rowDetail}>
                    <div>
                      <b>{words.build[r.state].label}</b> —— {words.build[r.state].explain ?? ''}
                    </div>
                    {r.reason && <div>{r.reason}</div>}
                    {r.disabledReason && <div>现在生成不了：{r.disabledReason}</div>}
                    <div>
                      产物：{r.mkpFile ?? '—'} · 配套切片器 {r.bbsCount} 份
                    </div>
                  </div>
                )}
              </Fragment>
            ))}
            <div className={s.bar} style={{ marginTop: 12 }}>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  const next: Record<string, boolean> = {}
                  for (const r of rows) {
                    if (r.buildable && (r.state === 'neverBuilt' || r.state === 'stale')) next[r.uid] = true
                  }
                  setPicked(next)
                }}
              >
                全选待生成
              </button>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={!ids.length || blocked}
                title={blocked ? words.disabled.buildBlocked : undefined}
                onClick={() => void generate()}
              >
                生成 {ids.length} 项
              </button>
            </div>
          </div>
        </div>

        {/* —— ② 客户端数据包 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>② 客户端数据包</h2>
            <span className={`${s.tag} ${lastPublish ? s.tagAccent : s.tagGhost}`}>
              {lastPublish ? `本会话已发布 ${lastPublish.files} 个文件` : '还没发布过'}
            </span>
          </div>
          <div className={s.cardBody}>
            {/*
              包版本 / 最低客户端版本在产品里沿用上游 compat 的声明（发布时从上游清单
              带过去），不是工作台的输入框 —— 「未声明」在左栏是一条待办，它的说明
              自己写着「这不是我们该填的空」。所以这张卡只读。
            */}
            <div className={s.kv}>
              <span className={s.kvKey}>渠道</span>
              <span className={s.kvVal}>{boot.info?.channel || '—'}</span>
              <span className={s.kvKey}>最低客户端版本</span>
              <span className={s.kvVal}>
                {boot.info?.minimumClient || (
                  <span className={s.kvDim}>未声明 —— 上游 manifest 没写（左栏有待办）</span>
                )}
              </span>
              <span className={s.kvKey}>发布目录</span>
              <span className={`${s.kvVal} ${s.mono}`}>{boot.roots.dist}</span>
            </div>
            <div className={s.group}>
              <div className={s.groupHead}>包里有什么</div>
              <div className={s.chips}>
                <span className={s.chip}>{book.badges.machines} 台机型</span>
                <span className={s.chip}>{book.badges.versions} 个版本</span>
                {boot.info && (
                  <>
                    <span className={s.chip}>{boot.info.params} 个字段</span>
                    <span className={s.chip}>{boot.info.deliverables} 个交付物</span>
                    <span className={s.chip}>{boot.info.fallbacks} 条回退规则</span>
                  </>
                )}
              </div>
              <p className={s.note}>
                客户端看不到「机型基底」这一层 —— 包里没有 origin、没有「这一项是谁给的」。
                继承规则是后厨的事，客户端重算就等于两边各留一份规则，必然分叉。
              </p>
            </div>
          </div>
        </div>

        {/* —— ③ 发布 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>③ 发布</h2>
            <span className={s.cardNote}>
              生成得出来 ≠ 可以发布 —— 这一步问的是「交出去客户端会不会缺东西」
            </span>
          </div>
          <div className={s.cardBody}>
            {blocked && (
              <div className={s.warn} style={{ marginTop: 0 }}>
                <div className={s.warnTitle}>有 {report?.blocks} 条阻断，不许发布</div>
                <div className={s.warnDetail}>左栏里标「阻断」的就是 —— 处理完这一步才会亮。</div>
              </div>
            )}
            <div className={s.bar} style={{ marginTop: blocked ? 12 : 0 }}>
              <span className={s.cardNote}>
                发布 = 把产物与清单一起定稿进交付目录；带残留时后端一个字节都不写。
              </span>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={blocked}
                title={blocked ? words.disabled.publishBlocked : undefined}
                onClick={() => void publish()}
              >
                发布
              </button>
            </div>
          </div>
        </div>

        {/* —— ④ 对照基线 —— */}
        <div id="t-baseline" className={s.card}>
          <div className={s.cardHead}>
            <h2>④ 对照基线</h2>
            <span className={s.cardNote}>
              {baseline ? `${baseline.length} 份产物 · ${baselineDirty} 份与基线不同` : '正在读……'}
            </span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              入库产物 vs 判据基线（14.9）。**人看过这份清单再点同步** —— 内容相同的自动跳过，
              只有真的变了才会写。
            </p>
            {baseline?.map((b) => (
              <div key={b.fileName} className={s.row}>
                <span className={`${s.mono} ${s.rowMeta}`}>{b.fileName}</span>
                <span
                  className={`${s.tag} ${
                    b.status === 'same' ? s.tagGhost : b.status === 'changed' ? s.tagDanger : s.tagBuildNone
                  }`}
                >
                  {b.status === 'same' ? '一致' : b.status === 'changed' ? '已变更' : '基线缺失'}
                </span>
                <span className={s.grow} />
                <span className={`${s.mono} ${s.rowMeta}`} title="产物内容哈希（前 16 位）">
                  {b.productSha}
                </span>
              </div>
            ))}
            <div className={s.bar} style={{ marginTop: 12 }}>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={!baseline || baselineDirty === 0}
                title={
                  baseline && baselineDirty === 0
                    ? '九份产物与基线逐字节相同，没有要写的'
                    : '只写上面标「已变更 / 基线缺失」的那几份'
                }
                onClick={() => void syncBaseline()}
              >
                同步基线{baselineDirty > 0 ? `（${baselineDirty}）` : ''}
              </button>
            </div>
          </div>
        </div>

        {/* —— ⑤ 交付残留 —— */}
        <div id="t-strays" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑤ 交付残留</h2>
            <span className={s.cardNote}>
              {strays ? `${strays.length} 个` : '正在读……'}
            </span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              「不在本次交付集合内」的文件 —— 残留会被消费端真的下载到，所以发布被它拦下。
            </p>
            {strays && strays.length > 0 ? (
              strays.map((f) => (
                <div key={f} className={s.row}>
                  <span className={`${s.mono} ${s.rowMeta}`}>{f}</span>
                </div>
              ))
            ) : (
              <div className={s.emptyHint}>
                {strays ? '没有残留 —— 交付目录与本次交付集合一致' : '正在读……'}
              </div>
            )}
            <div className={s.bar} style={{ marginTop: 12 }}>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={!strays || strays.length === 0}
                title="走 workbench/.trash/dist/ 回收（保留相对路径，可还原），不直接删"
                onClick={() => void cleanStrays()}
              >
                清理残留
              </button>
            </div>
          </div>
        </div>

        {/* —— ⑥ 回收站 —— */}
        <div id="t-trash" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑥ 回收站</h2>
            <span className={s.cardNote}>{trash ? `${trash.length} 条` : '正在读……'}</span>
          </div>
          <div className={s.cardBody}>
            {trash && trash.length > 0 ? (
              trash.map((t) => (
                <div key={t.file} className={s.row}>
                  <span className={`${s.mono} ${s.rowMeta}`}>{t.machineId}/{t.versionId}</span>
                  <span className={s.rowMeta}>{t.deletedStamp}</span>
                  <span className={s.grow} />
                  <span className={`${s.mono} ${s.rowMeta}`}>{t.file}</span>
                </div>
              ))
            ) : (
              <div className={s.emptyHint}>{words.empty.trashEmpty}</div>
            )}
            <p className={s.note} style={{ marginBottom: 0 }}>
              删除版本时它的生成快照进这里。**这一页只读** —— 还原与彻底删除的动作
              还没有命令（待裁决项），条目先列出来让人看得见。
            </p>
          </div>
        </div>

        {/* —— ⑦ 子目录职责（14.7） —— */}
        <div id="t-store-dirs" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑦ 子目录职责</h2>
            <span className={s.cardNote}>workbench/ 的五个子目录，谁写谁读一句话说清</span>
          </div>
          <div className={s.cardBody}>
            {boot.storeDirs.map((d) => (
              <div key={d.name} className={s.row}>
                <span className={`${s.mono} ${s.rowMeta}`}>{d.name}</span>
                <span className={s.rowName}>{d.role}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

/** 生成态四档的样式（与机型页共用同一张类名表） */
const STATE_TAG: Record<string, string> = {
  built: s.tagBuildBuilt,
  stale: s.tagBuildStale,
  neverBuilt: s.tagBuildNever,
  noResources: s.tagBuildNone,
}
