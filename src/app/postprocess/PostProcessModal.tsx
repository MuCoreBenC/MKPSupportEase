/*
 * 钩子那一趟的那一屏 —— 切片器导出 G-code 时，**常驻的那扇窗**上弹出来的就是它。
 *
 * 形状照旧世代 `mkp-ssr` 的进度窗（窗口常驻、结果留到下一次顶掉它），所以：
 *
 *   1. **跑着的时候关不掉**：没有 ×、点遮罩也不关 —— 那一趟还在跑，藏起来只会让人以为
 *      出事了。要收起来只有一条路：按「后台运行」（那是明确说过的意思，而且收起来之后
 *      由外壳那条窄条兜着，见 `PostProcessStrip.tsx`）。
 *   2. **窗口留着、下次复用**：钩子进程跑完就退（切片器一秒都不多等），但这一屏不自动关
 *      —— 用户关掉它之前，结论一直在（下一次切片由新一趟的开场顶掉）。
 *   3. **机型不匹配那一问等用户**：两颗按钮就是「继续跑 / 停下」，**没有默认** ——
 *      替用户决定"不匹配也照跑"是这一层最不该做的事（没答 / 超时 = 不跑，见 Rust 侧）。
 *   4. **跑完说清哪一档**：成功 / 已取消 / 失败长得不一样 —— 取消不是失败，
 *      把用户自己按的停止说成"处理失败"是一种冤枉。
 *
 * # 2026-10-11：左栏那张表（照旧世代那一屏）
 *
 * 顶上先摆**这一趟在处理什么**：G-code 机型 / 预设机型 / 耗材 / 文件大小 / 喷嘴 · 层高 ——
 * 事实由钩子一开场报一条（`hook_ipc::FactsPayload`）。**只摆开跑那一刻就确定的那几格**：
 * 总层数要 pass1 才刮出来、擦料塔类型要 support 步的判据才定，这一版先不摆（不编、也不空占一行）。
 *
 * 旧世代那一屏右边还有一块路径预览，我们这一版**没有**：内核不存塔 / 涂胶的路径点集
 * （要画得先从 G-code 抠，或者让内核把几何交出来），那是下一步的事。
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import { Btn } from '../ui/Controls'
import { Progress } from '../ui/Progress'
/* 模态框只有一套（2026-10-10）：稿内那套 `ui/Modal` + 它的 `.scrim/.modal` 退役 */
import { Modal } from '../../components/modal'
import { percentOf, stepName } from './steps'
import type { PostProcessRunView } from './usePostProcessRun'
import s from './PostProcessModal.module.css'

export interface PostProcessModalProps {
  /** 那一趟的现况（`usePostProcessRun` 给的那一份；App 拿着它，后台那条窄条也要） */
  view: PostProcessRunView
  /** 「看报告」：跨页跳转的出口由外壳给（页面 / 浮层不该伸手改页签） */
  onOpenReport: () => void
}

/** 文件大小：MB 两位小数（与报告页那份口径一致） */
function sizeText(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/** 毫米这两位：内核里 0 = 预设没写这一格 ⇒ 摆 `—`，不摆 `0.00` */
function mmText(v: number): string {
  return v > 0 ? v.toFixed(2) : '—'
}

/**
 * 「预计还需」：拿**已经跑了多久**按全局完成度外推。
 *
 * 起点只有"这一趟的开场事件被这一侧看见过"时才准（窗口开在半路 ⇒ 没有起点 ⇒ 不摆这一格，
 * 编一个比空着糟）；完成度太低时外推值会离谱（1% 时乘以 99），所以低于 5% 也不摆。
 */
function etaText(sinceMs: number | null, percent: number): string | null {
  if (sinceMs === null || percent < 5) return null
  const left = ((Date.now() - sinceMs) * (100 - percent)) / percent
  if (!Number.isFinite(left) || left <= 0) return null
  const sec = Math.round(left / 1000)
  if (sec < 60) return `预计还需 ${sec} 秒`
  const m = Math.floor(sec / 60)
  return `预计还需 ${m} 分 ${sec - m * 60} 秒`
}

export default function PostProcessModal({ view, onOpenReport }: PostProcessModalProps) {
  const { run, dismiss } = view
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  /* 「预计还需」每秒重算一次（这个表只在跑着的时候挂着） */
  const [, setTick] = useState(0)

  const finished = run?.finished ?? null
  const running = run !== null && finished === null

  useEffect(() => {
    if (!running) return
    const id = window.setInterval(() => setTick((n) => n + 1), 1000)
    return () => window.clearInterval(id)
  }, [running])

  if (run === null || view.dismissed) return null

  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)
  const eta = running ? etaText(view.sinceMs, percent) : null
  const facts = run.facts

  /** 两条命令（停止 / 答复）都要等后端，失败要说出来 —— 界面不许"点了没反应" */
  const act = (fn: () => Promise<void>) => {
    setBusy(true)
    setErr(null)
    void fn()
      .catch((e: unknown) => setErr(errorText(e)))
      .finally(() => setBusy(false))
  }

  return (
    <Modal
      open
      size="md"
      title={running ? '后处理进度' : '后处理结果'}
      /* 文件名那一格原来是标题右边的 `s.files` —— 通用框有 `subtitle`，用它 */
      subtitle={run.gcodeName}
      /* 跑着的时候点遮罩不关（见文件头第 1 条）；要收起来请按「后台运行」 */
      closeOnScrim={!running}
      onClose={dismiss}
      footer={
        running ? (
          <>
            <Btn disabled={busy} onClick={() => act(() => api.cancelPostProcess())}>
              停止
            </Btn>
            <Btn variant="accent" onClick={dismiss}>
              后台运行
            </Btn>
          </>
        ) : (
          <>
            <Btn
              onClick={() => {
                dismiss()
                onOpenReport()
              }}
            >
              看报告
            </Btn>
            <Btn variant="accent" onClick={dismiss}>
              关闭
            </Btn>
          </>
        )
      }
    >
      <div className={s.body}>
        {/*
          这一趟在处理什么（左栏那张表）。**一格都没报过就整块不摆**（老记录 / 极早失败），
          不做成一张空表。
        */}
        {facts !== null && (
          <dl className={s.facts}>
            <div className={s.factRow}>
              <dt>G-code 机型</dt>
              <dd>{facts.gcodeMachine}</dd>
            </div>
            <div className={s.factRow}>
              <dt>预设机型</dt>
              <dd>{facts.presetMachine}</dd>
            </div>
            <div className={s.factRow}>
              <dt>耗材类型</dt>
              <dd>{facts.filamentType}</dd>
            </div>
            <div className={s.factRow}>
              <dt>文件大小</dt>
              <dd>{sizeText(facts.fileSizeBytes)}</dd>
            </div>
            <div className={s.factRow}>
              <dt>喷嘴 / 层高</dt>
              <dd>
                {mmText(facts.nozzleDiameter)} / {mmText(facts.layerHeight)} mm
              </dd>
            </div>
          </dl>
        )}

        {running ? (
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
              <span className={s.pct}>
                {eta !== null && <span className={s.eta}>{eta}</span>}
                {Math.round(percent)}%
              </span>
            </p>
            <p className={s.msg}>{run.progress?.message ?? '正在把这盘 G-code 交给后处理…'}</p>

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
          finished !== null && (
            <>
              <p
                className={finished.ok ? s.ok : finished.cancelled ? s.cancelled : s.bad}
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
              {/*
                退出码：切片器那个报错框里**一定有**它（"… failed. Error code: N"）。
                而我们在 stderr 上写的那一整句：**macOS 上它摆在同一个框的 `Output:` 段里，
                Windows 上那个框根本不接脚本输出**（BBS 源码里那一支连管道都没建 ——
                见 `docs/HOOK-ERRORS-AND-MESSAGES.md`）。把这一格摆出来，两个框才对得上号
                （作者 2026-10-11 的真实困惑：那边只说 code 1，这边才知道是超边界还是碰撞）。
              */}
              {finished.exitCode !== 0 && (
                <p className={s.code}>
                  退出码 <code>{finished.exitCode}</code> —— 切片器那个「Error code」就是它
                </p>
              )}
              {finished.warnings.map((w) => (
                <p className={s.warn} key={w}>
                  警告：{w}
                </p>
              ))}
              <p className={s.hint}>
                窗口留着，下次切片接着用；这一屏收起来也不影响记录 —— 报告页里有那一条。
              </p>
            </>
          )
        )}

        {err !== null && (
          <p className={s.bad} role="alert">
            {err}
          </p>
        )}
      </div>
    </Modal>
  )
}
