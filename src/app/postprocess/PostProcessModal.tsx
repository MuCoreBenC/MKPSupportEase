/*
 * 钩子那一趟的那一屏 —— 切片器导出 G-code 时，本程序被带参数拉起来就弹它。
 *
 * 三条硬约定（照成熟版 `mkpsupporte` 的 `ProcessingModal`）：
 *
 *   1. **关不掉**：没有 ×、点遮罩也不关。这一屏的结束由那一趟说了算 —— 跑完窗口自己退，
 *      切片器才走得下去（切片器在等这个进程退出）。给一颗"关闭"就等于给一条
 *      "让切片器永远卡在 95%"的路。
 *   2. **机型不匹配那一问等用户**：两颗按钮就是「继续跑 / 停下」，**没有默认** ——
 *      替用户决定"不匹配也照跑"是这一层最不该做的事（没答 / 超时 = 不跑，见 Rust 侧）。
 *   3. **跑完说清哪一档**：成功 / 已取消 / 失败长得不一样 —— 取消不是失败，
 *      把用户自己按的停止说成"处理失败"是一种冤枉。
 */

import { useState } from 'react'

import { api, errorText } from '../../api'
import { Btn } from '../ui/Controls'
import { Progress } from '../ui/Modal'
import ui from '../ui/ui.module.css'
import { percentOf, stepName } from './steps'
import { usePostProcessRun } from './usePostProcessRun'
import s from './PostProcessModal.module.css'

export default function PostProcessModal() {
  const run = usePostProcessRun()
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  if (run === null) return null

  const finished = run.finished
  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)

  /** 两条命令（停止 / 答复）都要等后端，失败要说出来 —— 界面不许"点了没反应" */
  const act = (fn: () => Promise<void>) => {
    setBusy(true)
    setErr(null)
    void fn()
      .catch((e: unknown) => setErr(errorText(e)))
      .finally(() => setBusy(false))
  }

  return (
    <div className={ui.scrim}>
      <div className={ui.modal} role="dialog" aria-modal="true" aria-label="后处理">
        <header className={ui.modalHead}>
          <h2 className={ui.modalTitle}>后处理</h2>
          <span className={s.files} title={`${run.gcodeName} ｜ 预设 ${run.presetName}`}>
            {run.gcodeName}
          </span>
        </header>

        <div className={s.body}>
          {finished === null ? (
            <>
              <Progress percent={percent} />
              <p className={s.line}>
                {/* 有问句还挂着时别说"准备中"：那一刻**在等人**，进度条不动是应该的，
                    不说明白会像卡住 */}
                <span>
                  {run.question !== null
                    ? '等你决定'
                    : run.progress === null
                      ? '准备中'
                      : stepName(run.progress.step)}
                </span>
                <span className={s.pct}>{Math.round(percent)}%</span>
              </p>
              <p className={s.msg}>
                {run.progress?.message ?? '正在把这盘 G-code 交给后处理…'}
              </p>

              {run.question !== null && (
                <div className={s.ask}>
                  <p className={s.askText}>{run.question.text}</p>
                  <div className={s.askBtns}>
                    <Btn
                      variant="danger"
                      disabled={busy}
                      onClick={() => act(() => api.answerPostProcessMismatch(false))}
                    >
                      停下（不跑）
                    </Btn>
                    <Btn
                      variant="accent"
                      disabled={busy}
                      onClick={() => act(() => api.answerPostProcessMismatch(true))}
                    >
                      继续跑
                    </Btn>
                  </div>
                </div>
              )}
            </>
          ) : (
            <>
              <p
                className={
                  finished.ok ? s.ok : finished.cancelled ? s.cancelled : s.bad
                }
                role="status"
              >
                {finished.ok ? '✓ 处理完成' : finished.cancelled ? '已停止' : '✗ 处理失败'}
              </p>
              <p className={s.msg}>{finished.message}</p>
              {/* 失败时把**停在哪一步**说出来：只说"失败了"等于让人从头猜 */}
              {!finished.ok && !finished.cancelled && finished.stage !== null && (
                <p className={s.hint}>停在：{stepName(finished.stage)}</p>
              )}
              {finished.code !== null && (
                <p className={s.code}>
                  错误码 <code>{finished.code}</code> —— 报问题时带上它
                </p>
              )}
              {finished.warnings.map((w) => (
                <p className={s.warn} key={w}>
                  警告：{w}
                </p>
              ))}
              <p className={s.hint}>
                这一屏会自己关掉，让切片器继续（退出码 {finished.exitCode}）；想立刻放它走，
                直接把这扇窗关掉就行。
              </p>
            </>
          )}

          {err !== null && (
            <p className={s.bad} role="alert">
              {err}
            </p>
          )}
        </div>

        {finished === null && (
          <footer className={ui.modalFoot}>
            <Btn
              disabled={busy}
              title="停下这一趟；已经写盘之前的部分不会动，原文件不会被改坏"
              onClick={() => act(() => api.cancelPostProcess())}
            >
              停止
            </Btn>
          </footer>
        )}
      </div>
    </div>
  )
}
