/*
 * 错误弹框（2026-11-12）：一次失败要能讲清三件事 —— 哪一步、为什么、拿什么号去查。
 *
 * # 为什么不复用 toast
 *
 * 作者的原话：「如果失败的话，你必须给我弹出模态框或者通知，必须告诉我到底是怎么
 * 失败的……显示个错误码之类的」。toast 转眼就没，而这类失败**已经产生了现场**
 * （版本建好了、参数没抄过来），人得盯着那句原因去决定下一步。
 *
 * # 三行信息的来源
 *
 * ```text
 * 人话    AppError.message（字段级原因，写值失败时就是"哪个键、写的什么、该是什么"）
 * 码      messageCode（E_*）· code（NOT_FOUND / INVALID_ARGUMENT …）· req（日志号）
 * 详情    detail 原文 —— 可滚动、可选中、可一键复制
 * ```
 *
 * `req=` 由 `ipc::traced_with_req` 追加在 `detail` 末尾；这里只负责展示，不重拼。
 */

import { useEffect, useState } from 'react'
import type { AppError } from '../api'
import ModalC14 from './ModalC14'
import s from '../c14.module.css'

interface Props {
  open: boolean
  /** 哪一步失败（「复制配方」） */
  title: string
  /** 一句后果：这一下之后现场是什么样 */
  lead: string
  error: AppError | null
  onClose: () => void
}

/** 从 detail 里取出请求号。格式见 `ipc::traced_with_req`：末尾 `req=r00000336` */
function reqOf(detail: string | undefined): string | null {
  const hit = /req=([A-Za-z0-9]+)/.exec(detail ?? '')
  return hit?.[1] ?? null
}

export default function ErrorDialog({ open, title, lead, error, onClose }: Props) {
  const [label, setLabel] = useState('复制详情')

  /* 每次开框都把按钮文案复位 —— 上一次的「已复制」不该带到下一次 */
  useEffect(() => {
    if (open) setLabel('复制详情')
  }, [open])

  if (error === null) return null

  const req = reqOf(error.detail)
  const meta = [error.messageCode, error.code, req === null ? null : `req=${req}`]
    .filter((x): x is string => x !== null && x !== undefined && x !== '')
    .join(' · ')

  const raw = [
    title,
    lead,
    error.message,
    error.detail ?? '',
    `code=${error.code}`,
    error.messageCode === undefined ? '' : `messageCode=${error.messageCode}`,
    `traceId=${error.traceId}`,
  ]
    .filter((x) => x !== '')
    .join('\n')

  const copy = () => {
    /* 非安全上下文里 `clipboard` 可能是 undefined —— 不许把它变成一次未捕获的异常 */
    const writing = navigator.clipboard?.writeText(raw)
    if (writing === undefined) {
      setLabel('复制失败')
      return
    }
    writing
      .then(() => {
        setLabel('已复制')
        window.setTimeout(() => setLabel('复制详情'), 2000)
      })
      .catch(() => setLabel('复制失败'))
  }

  return (
    <ModalC14
      open={open}
      title={title}
      size="md"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={copy}>
            {label}
          </button>
          <button type="button" className={`${s.btn} ${s.btnPrimary}`} onClick={onClose}>
            关闭
          </button>
        </>
      }
    >
      <p className={s.cardNote} style={{ margin: '0 0 10px' }}>
        {lead}
      </p>
      <p className={s.errorMeta}>{meta}</p>
      <pre className={s.errorDetail}>
        {error.message}
        {error.detail === undefined ? '' : `\n${error.detail}`}
      </pre>
    </ModalC14>
  )
}
