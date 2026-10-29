/*
 * 设置页「最近删除」（2026-10-29）。
 *
 * 删掉的预设不再真的消失：文件 + 原文镜像 + 来源账都在 `recovery/<id>/`。
 * 这里只做三件事：列出来、恢复、永久删 / 清空。恢复**不自动设为使用中**。
 *
 * 保留策略（后端 `recovery::cleanup`）：30 天或 20 份，先到先清。
 *
 * # 为什么列表搬进模态框（2026-10-29 作者）
 *
 * 「最近删除的不要这样显示，到时候很多会占位置，你用模态框」。设置页那一块现在只剩
 * 一行说明 + 一个「查看 N 项」，条目再多也不会把下面的「清理与卸载」「高级设置」
 * 顶到几屏之外 —— 设置页是"扫一眼找开关"的地方，不是列表页。
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { RecoveryEntry } from '../../api/contract'
import Modal from '../../components/modal/Modal'
import { msg, useText, useT, type Text } from '../../i18n'
import { deliveryMutated, useDeliveryRevision } from '../state/deliveryState'
import { Btn } from '../ui/Controls'
import s from './PageSettings.module.css'

const CONFIRM_MS = 3000

function when(epoch: number): string {
  try {
    return new Date(epoch * 1000).toLocaleString()
  } catch {
    return '—'
  }
}

/**
 * `host`：模态框的遮罩要铺满"标题栏以下那一整块"，框自己只有 `absolute inset:0`，
 * 盖多大取决于挂在谁里面（与 `PagePresets` 同一个做法，见那边 `pageEl` 的注释）。
 */
export default function RecoverySection({ host }: { host: HTMLElement | null }) {
  const tr = useT()
  const say = useText()
  /* 页面常驻挂：别的页删了预设，这里要跟着刷新（不能只在挂载时读一次） */
  const revision = useDeliveryRevision()
  const [entries, setEntries] = useState<RecoveryEntry[] | null>(null)
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState<string | null>(null)
  const [note, setNote] = useState<Text | null>(null)
  const [armedEmpty, setArmedEmpty] = useState(false)

  const load = () => {
    void api.listRecovery().then(
      (list) => setEntries(list),
      (e: unknown) => {
        setEntries([])
        setNote(msg('settings.recovery.failed', { detail: errorText(e) }))
      },
    )
  }

  useEffect(load, [revision])

  useEffect(() => {
    if (!armedEmpty) return
    const t = window.setTimeout(() => setArmedEmpty(false), CONFIRM_MS)
    return () => window.clearTimeout(t)
  }, [armedEmpty])

  /* 关框 = 把"确认清空"那一挂也收掉，免得下次打开还举着确认 */
  const close = () => {
    setOpen(false)
    setArmedEmpty(false)
    setNote(null)
  }

  const restore = (entry: RecoveryEntry) => {
    setBusy(entry.id)
    setNote(null)
    void api.restoreLocalPreset(entry.id).then(
      (path) => {
        setBusy(null)
        setNote(msg('settings.recovery.restored', { name: path.split('/').pop() ?? entry.fileName }))
        /* 预设页立刻能看到恢复的那一行（恢复不设 active，只是文件回来了） */
        deliveryMutated()
        load()
      },
      (e: unknown) => {
        setBusy(null)
        setNote(msg('settings.recovery.failed', { detail: errorText(e) }))
      },
    )
  }

  const purge = (entry: RecoveryEntry) => {
    setBusy(entry.id)
    setNote(null)
    void api.purgeRecovery(entry.id).then(
      () => {
        setBusy(null)
        load()
      },
      (e: unknown) => {
        setBusy(null)
        setNote(msg('settings.recovery.failed', { detail: errorText(e) }))
      },
    )
  }

  const empty = () => {
    if (!armedEmpty) {
      setArmedEmpty(true)
      return
    }
    setArmedEmpty(false)
    setBusy('empty')
    void api.emptyRecovery().then(
      (n) => {
        setBusy(null)
        setNote(msg('settings.recovery.emptied', { count: n }))
        deliveryMutated()
        load()
      },
      (e: unknown) => {
        setBusy(null)
        setNote(msg('settings.recovery.failed', { detail: errorText(e) }))
      },
    )
  }

  const count = entries?.length ?? 0

  return (
    <section className={s.section} aria-label={tr('settings.recovery.title')}>
      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <h2 className={s.secTitle}>{tr('settings.recovery.title')}</h2>
          <p className={s.fieldNote}>{tr('settings.recovery.note')}</p>
        </div>
        <Btn disabled={entries === null} onClick={() => setOpen(true)}>
          {count > 0
            ? tr('settings.recovery.openWithCount', { count: String(count) })
            : tr('settings.recovery.open')}
        </Btn>
      </div>

      <Modal
        open={open}
        title={tr('settings.recovery.title')}
        subtitle={tr('settings.recovery.subtitle')}
        size="md"
        host={host}
        onClose={close}
        footer={
          <>
            {count > 0 && (
              <Btn variant="danger" disabled={busy !== null} onClick={empty}>
                {busy === 'empty'
                  ? tr('settings.recovery.emptying')
                  : armedEmpty
                    ? tr('settings.recovery.emptyConfirm')
                    : tr('settings.recovery.empty')}
              </Btn>
            )}
            <Btn onClick={close}>{tr('common.close')}</Btn>
          </>
        }
      >
        {entries === null && <p className={s.noteHint}>{tr('settings.recovery.loading')}</p>}
        {entries !== null && entries.length === 0 && (
          <p className={s.noteHint}>{tr('settings.recovery.emptyList')}</p>
        )}

        {entries !== null && entries.length > 0 && (
          <ul className={s.recList}>
            {entries.map((entry) => (
              <li key={entry.id} className={s.recItem}>
                <div className={s.recText}>
                  <div className={s.recName}>{entry.fileName}</div>
                  {/* 时间在前：放不下时省略号吃掉的是**冗余的路径**（名字已经在上行），不是时间 */}
                  <div className={s.recMeta}>
                    {when(entry.deletedAt)} · {entry.path}
                    {entry.cloudFile !== null ? ` · ${entry.cloudFile}` : ''}
                  </div>
                </div>
                <div className={s.recBtns}>
                  <button
                    type="button"
                    className={s.recBtn}
                    disabled={busy !== null}
                    onClick={() => restore(entry)}
                  >
                    {busy === entry.id
                      ? tr('settings.recovery.restoring')
                      : tr('settings.recovery.restore')}
                  </button>
                  <button
                    type="button"
                    className={`${s.recBtn} ${s.recBtnDanger}`}
                    disabled={busy !== null}
                    onClick={() => purge(entry)}
                  >
                    {tr('settings.recovery.purge')}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}

        {note !== null && <p className={s.noteHint}>{say(note)}</p>}
      </Modal>
    </section>
  )
}
