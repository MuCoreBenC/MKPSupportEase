/**
 * 发布闸（第二刀）—— 点【发布】之前先过这一道。
 *
 * # 它是什么
 *
 * 十五项检查逐项打勾，**全绿才允许往下走**（作者定的硬规矩：任何一项 Blocker 红了，
 * 绝对不进 commit / push / PR）。
 *
 * ★ 界面在这里**只负责画，不负责判**：整份结果来自 `wb_publish_audit`
 * （Rust 的 `audit::publish_audit` —— 与 `cargo test` 判据调的是同一个函数）。
 * 前端**不许**自己再实现一套检查：那正是这把刀要治的老病根 —— 两边各算一遍，
 * 各自看着都对，然后发布了不该发布的东西。
 *
 * # 布局：十五项平铺
 *
 * 作者的口径是「逐项打勾」，所以这一屏就是一张清单：一项一行，左边结论、右边那一项
 * 在说什么。**红的排最前**（要人先看），其余照 §5.2 的编号排。编号与顺序都来自后端
 * （`AuditItem.id` 就是 `PUBLISH-ARCHITECTURE.md` §5.2 那张表的 id）。
 *
 * `Skipped` 画成**虚线的灰**，不画成绿勾 —— 它说的是「这一项今天还没跑」。
 * 把"没实现"画成通过，比画成红叉更危险（作者定的分档口径）。
 *
 * # 底下那颗按钮
 *
 * 「确认发布」只在 `canPublish` 为真时亮，点下去走原来那条 `wb_publish`
 * （后端内部还有两道硬闸：检查阻断 + 残留拦截）。这一道闸不是把后端那道盖掉，
 * 是把它**提前摆到人面前**，并且把"哪儿还没好、去哪儿修"一次说全。
 *
 * **「创建 PR」还没接**（第三刀：minVersion + 结构签名 + PR 回读）—— 所以这里
 * 不摆一颗点不动的假按钮。今天这一屏管的是「把产物定稿」那一步。
 */

import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { AuditItem, AuditStatus, PublishAudit } from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import s from './PublishGateModal.module.css'

/** 四态各画成一个词。`pass` 是"跑过了、没问题"，`skipped` 是"今天没跑"—— 两件事 */
const STATUS_TEXT: Record<AuditStatus, string> = {
  pass: '通过',
  fail: '未通过',
  warn: '待留意',
  skipped: '未实现',
}

const STATUS_CLASS: Record<AuditStatus, string> = {
  pass: s.stPass,
  fail: s.stFail,
  warn: s.stWarn,
  skipped: s.stSkipped,
}

/** 一行底色：红的压过黄的，虚线留给"没跑" */
function rowClass(i: AuditItem): string {
  if (i.status === 'fail') return `${s.row} ${s.rowFail}`
  if (i.status === 'warn') return `${s.row} ${s.rowWarn}`
  if (i.status === 'skipped') return `${s.row} ${s.rowSkipped}`
  return s.row
}

/** 排序：红的、黄的在前（要人先看），其余照后端给的编号顺序 */
function rankOf(i: AuditItem): number {
  if (i.status === 'fail') return 0
  if (i.status === 'warn') return 1
  return 2
}

/** 一行里的文件清单：最多摊这么多个，多的折成一句 */
const FILE_CAP = 6

interface Props {
  onClose: () => void
  /** 闸全绿、人点了「确认发布」之后才走这里。**报错请抛出来**，这一屏会把它显示在原地 */
  onPublish: () => Promise<void>
}

export default function PublishGateModal({ onClose, onPublish }: Props) {
  /** 闸的结果；null = 还没算回来 */
  const [audit, setAudit] = useState<PublishAudit | null>(null)
  /** 算挂了 / 发布挂了 —— 都在这一行说，不另开一个提示框 */
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /**
   * 跑一遍闸。**只读** —— 后端那条命令一个字节都不写，所以点几次都不会有副作用
   * （「重新检查」那颗按钮就靠这条）。
   */
  const run = useCallback(async () => {
    setError(null)
    try {
      setAudit(await wb.publishAudit())
    } catch (e) {
      setAudit(null)
      setError(isAppError(e) ? e.message : String(e))
    }
  }, [])

  useEffect(() => {
    void run()
  }, [run])

  const publish = async () => {
    setBusy(true)
    setError(null)
    try {
      await onPublish()
      onClose()
    } catch (e) {
      /* 发布挂了就把闸留在原地 —— 结果页是给"成功"用的，别拿它兜错误 */
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const items = audit === null ? [] : [...audit.items].sort((a, b) => rankOf(a) - rankOf(b))
  const blockers = audit?.items.filter((i) => i.severity === 'blocker' && i.status === 'fail').length ?? 0
  const warnings = audit?.items.filter((i) => i.status === 'warn').length ?? 0

  const subtitle =
    audit === null
      ? error === null
        ? '正在跑十五项检查……'
        : '没跑起来'
      : audit.canPublish
        ? `十五项全过${warnings > 0 ? ` · ${warnings} 项待留意` : ''}`
        : `${blockers} 项拦住发布`

  return (
    <ModalC14
      open
      size="lg"
      title="发布闸"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.footNote}>
            {audit === null
              ? '闸还没跑完 —— 跑完才谈得上发不发'
              : audit.canPublish
                ? '任何一项 Blocker 红了就不许往下走 —— 现在是全绿'
                : '有 Blocker 未通过：修完点「重新检查」'}
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button type="button" className={c.btn} onClick={() => void run()} disabled={busy}>
            重新检查
          </button>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            disabled={!audit?.canPublish || busy}
            title={audit?.canPublish ? undefined : '闸没全绿 —— 这颗按钮不亮'}
            onClick={() => void publish()}
          >
            {busy ? '发布中…' : '确认发布'}
          </button>
        </>
      }
    >
      {error !== null && (
        <p className={s.error} role="alert">
          {error}
        </p>
      )}

      {audit !== null && (
        <>
          {/* 这一批会让 presets/ 怎么变 —— 数来自后端那一格（只数 presets/ 下面的） */}
          <p className={s.counts}>
            本次发布会让 <span className={c.mono}>presets/</span> 新增{' '}
            <b>{audit.filesAdded}</b> · 改动 <b>{audit.filesChanged}</b> · 删除{' '}
            <b>{audit.filesRemoved}</b> 个文件
            {audit.minVersion !== null && <> · 最低客户端版本 {audit.minVersion}</>}
          </p>

          <div className={s.list} role="list">
            {items.map((i) => (
              <div key={i.id} className={rowClass(i)} role="listitem">
                <span className={`${s.st} ${STATUS_CLASS[i.status]}`}>
                  {STATUS_TEXT[i.status]}
                  {i.severity === 'blocker' && i.status === 'fail' && (
                    <span className={s.sev} title="Blocker：红了就不许发布">
                      拦发布
                    </span>
                  )}
                </span>
                <div className={s.body}>
                  <span className={s.head}>
                    <b className={s.name}>{i.name}</b>
                    <span className={`${c.mono} ${s.id}`}>{i.id}</span>
                  </span>
                  <span className={s.details}>{i.details}</span>
                  {i.affectedFiles.length > 0 && (
                    <span className={s.files}>
                      {i.affectedFiles.slice(0, FILE_CAP).map((f) => (
                        <code key={f} className={`${c.mono} ${s.file}`}>
                          {f}
                        </code>
                      ))}
                      {i.affectedFiles.length > FILE_CAP && (
                        <span className={s.more}>…等 {i.affectedFiles.length} 处</span>
                      )}
                    </span>
                  )}
                  {i.status !== 'pass' && <span className={s.fix}>去修：{i.fixHint}</span>}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {audit === null && error === null && <p className={s.pending}>正在跑十五项检查……</p>}
    </ModalC14>
  )
}
