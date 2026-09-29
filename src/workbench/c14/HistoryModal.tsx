/*
 * 未保存改动（外壳级，C14 移植）。
 *
 * 它原来长在参数台的工具行里，现在挂在**状态栏**上 —— 撤销/重做/未保存改动是
 * 全工作台的事（改机型、改可见性、改参数都算），不该是某一页的一个按钮。
 *
 * 作者点名要的那件事（「没显示什么值、改成什么值」）在真后端这里是一等公民：
 * `wb_diff_draft` 返回的每一行就带着 label / before / after —— 明细**由后端算好**，
 * 这个框只负责摆出来。前端不推「哪一步动了哪几格」，那正是「前端不算业务」要防的。
 */

import { useEffect, useState } from 'react'
import { wb, type DiffLine } from '../api'
import { isAppError } from '../api'
import ModalC14 from './ModalC14'
import s from '../c14.module.css'

interface Props {
  open: boolean
  onClose: () => void
}

export default function HistoryModal({ open, onClose }: Props) {
  const [lines, setLines] = useState<DiffLine[] | null>(null)
  const [err, setErr] = useState<string | null>(null)

  /* 每次打开现取一份 —— 草稿是后端的账，缓存它只会演出过期内容 */
  useEffect(() => {
    if (!open) return
    let alive = true
    setErr(null)
    wb
      .diffDraft()
      .then((d) => alive && setLines(d))
      .catch((e: unknown) => alive && setErr(isAppError(e) ? e.message : String(e)))
    return () => {
      alive = false
    }
  }, [open])

  return (
    <ModalC14
      open={open}
      title="未保存改动"
      subtitle="保存 = 写进仓库文件（保存本身也能撤销），放弃 = 整条一起丢"
      size="md"
      onClose={onClose}
    >
      <div className={s.history}>
        {err !== null ? (
          <div className={s.sum} data-tone="danger">
            {err}
          </div>
        ) : lines === null ? (
          <div className={s.sum}>正在读草稿…</div>
        ) : lines.length ? (
          lines.map((l, i) => (
            <div key={`${l.target}-${l.key ?? ''}-${i}`} className={s.hRow}>
              <span className={s.hWhat}>{l.label}</span>
              <span className={s.hFrom}>{l.before === '' ? '（空）' : l.before}</span>
              <span className={s.hArrow} aria-hidden>
                →
              </span>
              <span className={s.hTo}>{l.after === '' ? '（空）' : l.after}</span>
            </div>
          ))
        ) : (
          <div className={s.sum}>干净的，没有未保存改动</div>
        )}
      </div>
      <p className={s.note}>
        撤销与重做走同一条栈（状态栏那两个按钮，或 Ctrl+Z / Ctrl+Y）。
        「机型与版本」页的新增与删除是**即时落盘**的，不在这里 —— 那类操作没有草稿。
      </p>
    </ModalC14>
  )
}
