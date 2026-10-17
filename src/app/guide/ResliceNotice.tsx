/*
 * **保存成功之后那一句提问**（2026-10-15）：「参数存好了，切片器不会自己知道 ——
 * 要不要我指给你看重切一次？」
 *
 * # 为什么非问不可
 *
 * 切片器那份参数里没有 MKP 这些字段（XYZ 偏移、涂胶…），它**不会**因为我们改了
 * 就把上一次的切片作废 —— 于是「切片」按钮不给点，用户只觉得"改完没反应"。
 * 这条链我们改不动（没有外部入口，见 `docs/INVESTIGATION-BBS-INVALIDATE.md` §5/§7），
 * 能做的是**把那一步指给他看**。
 *
 * # 两个按钮 + 一个「不再显示」
 *
 * - 「查看引导」：把小卡片摆到切片器里「板子类型」旁边（真机才有；指不了时
 *   照实显示原因，**不画一个指错地方的箭头**）；
 * - 「关闭」：这次不问了；
 * - 「不再显示」（2026-10-17 作者）：勾上 = **以后保存不再弹这一屏**。它与设置页
 *   「引导」那一格**写同一个键**（`app/guide/guidesPref`）—— 一处关、处处关，
 *   想再看就去设置里解开。**参数页与校准页共用这一份**（同一个组件、同一个开关）。
 *
 * ★ 「不再显示」按**设置开关**的口径：勾上**立即写盘**（与主题 / 自动清理那几格
 *   同一条口径——开关本身就是明确意图）。这样带着勾点 × / Esc 收屏也一样算数，
 *   不会因为"没走那颗「关闭」"把这次的选择丢掉。
 *
 * ★★ 「查看引导」**不再自动关掉这一屏**（2026-10-15 作者："我觉得原本的模态框也
 * 没必要消失"）：卡片摆出去之后，用户要看的**是切片器那一屏**（我们在底下、切片器
 * 在最前），这一屏留着不挡事；而且它还得留着显示错误（"建不出引导窗口"那种）。
 * 上一次是 `.then` 里 `onClose()`，于是"点了引导、这屏没了、卡片也没出来"时
 * 用户手上什么都没了 —— 连失败原因都看不到。
 */

import { useState } from 'react'

import { api, errorText } from '../../api'
import { Modal } from '../../components/modal'
/* 勾选共用件（2026-10-17「统一全站的勾和框」）：原来是原生绿勾，换成全站同一套墨色 */
import { Check } from '../../components/check'
import { useGuidesPref } from './guidesPref'
import s from './ResliceNotice.module.css'

export default function ResliceNotice({
  open,
  onClose,
}: {
  open: boolean
  onClose: () => void
}) {
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  /* 这一趟摆出去成功了没有 —— 只用来给一句回执，不关这一屏 */
  const [shown, setShown] = useState(false)
  /*
   * 「不再显示」那个共用开关（与设置页「引导」同一格）。勾上 = 关，立即写盘：
   * 带着勾点 × / Esc 收屏也算数（见模块头）。
   */
  const [showGuides, setShowGuides] = useGuidesPref()

  const view = () => {
    setBusy(true)
    setErr(null)
    void api
      .guideShow()
      .then((t) => {
        if (t.ready) {
          /* 卡片摆出去了：给一句回执，**这一屏留着**（作者 2026-10-15） */
          setShown(true)
          return
        }
        /* 指不了：把后端的那句人话原样显示，不画箭头 */
        setShown(false)
        setErr(t.reason ?? '这一台上指不了引导')
      })
      .catch((e: unknown) => {
        setShown(false)
        setErr(errorText(e))
      })
      .finally(() => setBusy(false))
  }

  /*
   * ★★ 主次与引导卡片同一套（2026-10-15 作者："上面的是解释、应该作为副标题"）：
   * `subtitle` 放"为什么"，正文放**操作步骤**，按钮就是那一步的入口。
   */
  return (
    <Modal
      open={open}
      size="sm"
      title="参数已保存"
      subtitle="切片器不会自己知道参数变了 —— 重新切片后才生效"
      /* 收屏工具提示：是否再提示已交给下面那个共用开关决定，不再写死"还会再问" */
      closeTitle="关闭（以后是否再提示，看下面的「不再显示」或设置页）"
      onClose={onClose}
      footer={
        <>
          <button type="button" className={s.ghost} onClick={onClose} disabled={busy}>
            关闭
          </button>
          <button type="button" className={s.primary} onClick={view} disabled={busy}>
            {shown ? '重新引导' : '查看引导'}
          </button>
        </>
      }
    >
      <div className={s.body}>
        <p className={s.text}>
          请在 Bambu Studio 中<b>重新切片</b>，以生成应用最新参数的 G-code。
        </p>
        <p className={s.text}>
          如果点击「切片」后没有重新切片，或「切片」按钮呈灰色无法点击，请先
          <b>切换到另一块打印板，再选回原来的打印板</b>，然后重新切片。
        </p>
        {/*
         * 回执：引导卡片已经摆到切片器那边了。★ 提示用户"切片器这会儿在最前"，
         * 免得他以为点了没反应（卡片不在我们这扇窗里，看不见是正常的）。
         */}
        {shown && !err && <p className={s.done}>引导卡片已摆到切片器那边（它这会儿在最前）。</p>}
        {err && <p className={s.err}>{err}</p>}
        {/*
         * 「不再显示」（2026-10-17 作者）：与设置页「引导」那一格**同一个键** ——
         * 一处关、处处关（参数页与校准页共用）。勾上**立即写盘**（见模块头），
         * 于是带着勾点 × / Esc 也一样生效。**不勾回来**就随时能在设置里再打开。
         */}
        <Check
          className={s.check}
          checked={!showGuides}
          onChange={(v) => setShowGuides(!v)}
          disabled={busy}
        >
          以后不再显示（可在「设置 → 引导」里重新打开）
        </Check>
      </div>
    </Modal>
  )
}
