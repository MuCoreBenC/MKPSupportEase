/*
 * 「导出日志」那一框（2026-11-12）。
 *
 * # 它回答什么
 *
 * 用户遇到问题了要能干活：点开这一框，看见"会打包哪几份、做了哪些隐私处理"，
 * 确认之后落一个 zip 在桌面。成与不成**都留在框里**（与报告页那扇导出框同一条口径）：
 * 成功之后最要紧的下一步是"去那个文件夹里看看"，框一关路径就没了。
 *
 * # 三条纪律
 *
 * 1. **清单是后端给的**：份数、日期范围、大小全来自 `log_export_plan` 读的那棵目录，
 *    界面不自己算 —— 否则"框里写三份、包里只有两份"没有任何东西咬得住。
 * 2. **隐私处理写在框里**（作者点名要的）：用户有权知道交出去的那包里有什么、被处理过什么。
 * 3. **成了不关、不新增行**：落点胶囊那颗「更改位置」就地翻成「打开位置」，
 *    页脚左边多一句「已打包 N 份」—— 框高不变。
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { LogExport, LogPlan } from '../../api/contract'
import Modal from '../../components/modal/Modal'
import { useT, type StringKey } from '../../i18n'
import PathPill from '../shared/PathPill'
import { Btn } from '../ui/Controls'
import s from './LogExportModal.module.css'

/** 三条流的行（与后端 `stream` 一一对应）。摘要单独一行，它固定一份。 */
const STREAMS = ['app', 'ui', 'hook'] as const

/** 行名各自一条键 —— 不用模板拼键：字典是**编译期**的类型，拼出来的串它认不出 */
const ROW_KEY: Record<(typeof STREAMS)[number], StringKey> = {
  app: 'settings.logExport.row.app',
  ui: 'settings.logExport.row.ui',
  hook: 'settings.logExport.row.hook',
}

function fmtBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`
  return `${bytes} B`
}

export default function LogExportModal({
  open,
  host,
  onClose,
}: {
  open: boolean
  host: HTMLElement | null
  onClose: () => void
}) {
  const tr = useT()
  const [dir, setDir] = useState('')
  const [plan, setPlan] = useState<LogPlan | null>(null)
  const [planErr, setPlanErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [fail, setFail] = useState<string | null>(null)
  const [done, setDone] = useState<LogExport | null>(null)

  /* 每次打开都重读：日志是活的，上一次那几份不能代表这一次 */
  useEffect(() => {
    if (!open) return
    setDone(null)
    setFail(null)
    setPlanErr(null)
    setPlan(null)
    void api.defaultLogDir().then(setDir, (e: unknown) => setPlanErr(errorText(e)))
    void api.logExportPlan().then(setPlan, (e: unknown) => setPlanErr(errorText(e)))
  }, [open])

  /** 「更改位置」：取消（`null`）保持原来那个位置，不是错误 */
  const changeDir = () => {
    void api.pickExportDir(dir).then((next) => {
      if (next === null) return
      setDir(next)
      /* 换了位置就是"再导一份"：上一次那句"已打包"不再成立 */
      setDone(null)
    })
  }

  const openAt = () => {
    if (done === null) return
    setFail(null)
    api.revealLogExport(done.path).catch((e: unknown) => setFail(errorText(e)))
  }

  const confirm = () => {
    if (busy || dir === '') return
    setBusy(true)
    setFail(null)
    void api.exportLogs(dir).then(
      (out) => {
        setDone(out)
        setBusy(false)
      },
      (e: unknown) => {
        setFail(errorText(e))
        setBusy(false)
      },
    )
  }

  const countOf = (stream: (typeof STREAMS)[number]): number =>
    plan === null ? 0 : plan.files.filter((f) => f.stream === stream).length
  const bytesOf = (stream: (typeof STREAMS)[number]): number =>
    plan === null ? 0 : plan.files.reduce((sum, f) => (f.stream === stream ? sum + f.bytes : sum), 0)
  const empty = plan !== null && plan.files.length === 0

  return (
    <Modal
      open={open}
      title={tr('settings.logExport.title')}
      subtitle={tr('settings.logExport.desc')}
      size="md"
      host={host}
      hideClose={busy}
      closeOnEsc={!busy}
      onClose={onClose}
      footer={
        <>
          {done !== null && (
            <span className={s.footNote}>
              {tr('settings.logExport.done', { count: String(done.files) })}
            </span>
          )}
          <Btn disabled={busy} onClick={onClose}>
            {tr('common.cancel')}
          </Btn>
          <Btn
            variant="accent"
            disabled={busy || dir === '' || plan === null || empty}
            onClick={confirm}
          >
            {busy ? tr('settings.logExport.exporting') : tr('settings.logExport.confirm')}
          </Btn>
        </>
      }
    >
      <div className={s.body}>
        <section className={s.block} aria-label={tr('settings.logExport.plan')}>
          <h3 className={s.blockHead}>{tr('settings.logExport.plan')}</h3>
          {plan === null ? (
            <p className={s.note}>{tr('settings.logExport.loading')}</p>
          ) : (
            <ul className={s.rows}>
              {STREAMS.map((stream) => (
                <li key={stream} className={s.row}>
                  <span className={s.rowName}>{tr(ROW_KEY[stream])}</span>
                  <span className={s.rowMeta}>
                    {tr('settings.logExport.count', { count: String(countOf(stream)) })}
                    {countOf(stream) > 0 && ` · ${fmtBytes(bytesOf(stream))}`}
                  </span>
                </li>
              ))}
              <li className={s.row}>
                <span className={s.rowName}>{tr('settings.logExport.row.summary')}</span>
                <span className={s.rowMeta}>{tr('settings.logExport.single')}</span>
              </li>
            </ul>
          )}
          {plan !== null && plan.from !== null && plan.to !== null && (
            <p className={s.note}>
              {tr('settings.logExport.range', { from: plan.from, to: plan.to })}
            </p>
          )}
          <p className={s.note}>{tr('settings.logExport.retention')}</p>
          {empty && <p className={s.note}>{tr('settings.logExport.empty')}</p>}
          {planErr !== null && (
            <p className={`${s.note} ${s.noteBad}`}>
              {tr('settings.logExport.planFailed', { detail: planErr })}
            </p>
          )}
        </section>

        {/*
         * 隐私处理（作者点名要写在框里）：四条都是**已经做过的处理**，
         * 不是承诺 —— 打包那一刻逐行过 `obs::redact`。
         */}
        <section className={s.block} aria-label={tr('settings.logExport.privacy')}>
          <h3 className={s.blockHead}>{tr('settings.logExport.privacy')}</h3>
          <ul className={s.privacy}>
            <li>{tr('settings.logExport.privacyHome')}</li>
            <li>{tr('settings.logExport.privacyIp')}</li>
            <li>{tr('settings.logExport.privacyFiles')}</li>
            <li>{tr('settings.logExport.privacyKeep')}</li>
          </ul>
        </section>

        {fail !== null && (
          <p className={`${s.note} ${s.noteBad}`}>
            {tr('settings.logExport.exportFailed', { detail: fail })}
          </p>
        )}

        {/*
         * 保存位置钉在下面（与报告页那扇导出框同一个位置）：清单在上、落点在下。
         * 形状走共用件 `app/shared/PathPill` —— 两扇框里这一条必须是同一件东西。
         * 成了之后写的是**包里那个 zip 的名字**，那颗按钮就地翻成「打开位置」。
         */}
        <div className={s.foot}>
          {/* 位置还没读到 ⇒ 摆一副同高的空架子：框身不会先矮后高（与报告页那条量尺同一个理由） */}
          {dir === '' ? (
            <PathPill gauge />
          ) : (
            <PathPill
              path={done === null ? dir : done.path}
              folder={done === null}
              opened={done !== null}
              action={done === null ? tr('settings.logExport.change') : tr('settings.logExport.open')}
              onAction={done === null ? changeDir : openAt}
            />
          )}
        </div>
      </div>
    </Modal>
  )
}
