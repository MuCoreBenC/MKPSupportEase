/*
 * **屏幕引导卡片**（2026-10-15）：贴着切片器里「板子类型」（就是"打印板"那个下拉）
 * 摆的一张小卡片，箭头指过去，告诉他怎么让 BBS 愿意重新切片。
 *
 * # 它只指，不代劳
 *
 * 点控件、换板型，从头到尾是用户自己做的 —— 这一层**不模拟点击、不改切片器任何
 * 参数**（边界写在 `ipc/guide.rs` 模块头）。卡片也**刻意做小**：它活着的时候，
 * 用户照样能回头去点切片器。
 *
 * # 它自己会挪 / 会消失（2026-10-15）
 *
 * 挪窗口与收放都在 Rust 侧做（AX 通知驱动，不是定时器，见 `ax_watch.rs`）——
 * 这一层只做一件事：**左右翻边时换箭头方向**。
 *
 * Rust 那边拖动窗口用 `set_position`（不重建，重建会闪），所以 URL 上的 `side`
 * 在窗口活着的期间是**初始值**；真的翻边了由 `guide-side` 事件推过来。
 *
 * # 文案（作者原文，2026-10-15 第三版 —— 结构按他说的"反过来"）
 *
 * 作者原话：「这个文案感觉应该把下面那些作为重点，因为那些是操作步骤，
 * 上面的反而是解释应该作为副标题」⇒ **操作步骤当正文（大、强调），
 * "为什么"降成副标题（小、灰）**。别自作主张改回去。
 */

import { useEffect, useMemo, useState } from 'react'

import { api, errorText, listen } from '../../api'
import { useT } from '../../i18n'
import s from './GuideCard.module.css'

/** 与 `ipc::guide::GUIDE_SIDE_EVENT` 同一个名字（那边是唯一真值） */
const SIDE_EVENT = 'guide-side'

export default function GuideCard() {
  const tr = useT()
  const q = useMemo(() => new URLSearchParams(window.location.search), [])
  /* 卡片被摆在目标的哪一侧 ⇒ 箭头指向哪边（Rust 那边算的，这里不猜） */
  const initial = q.get('side') === 'left' ? 'left' : 'right'
  const [side, setSide] = useState<'left' | 'right'>(initial)

  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  /*
   * 切片器被拖到屏幕另一头时会**翻边**（右边放不下就翻到左边，箭头跟着反向）。
   * 窗口不重建 ⇒ 只能靠这条事件知道。★ 订阅是异步的，effect 里要 `cancelled`
   * 兜住"还没订上就被卸载"那一档。
   */
  useEffect(() => {
    let cancelled = false
    let off: (() => void) | undefined
    void listen<string>(SIDE_EVENT, (payload) => {
      if (payload === 'left' || payload === 'right') setSide(payload)
    }).then((un) => {
      if (cancelled) un()
      else off = un
    })
    return () => {
      cancelled = true
      off?.()
    }
  }, [])

  const close = () => {
    setBusy(true)
    setErr(null)
    void api
      .guideClose()
      .catch((e: unknown) => setErr(errorText(e)))
      .finally(() => setBusy(false))
  }

  /*
   * 箭头**不是卡片的孩子** —— 它是窗口这一层的：卡片只占窗口的一段，旁边那条竖条是
   * 留给箭头的（理由见 `ipc/guide.rs` 的 `ARROW_PAD` 与 `GuideCard.module.css` 的头注释）。
   * 挂在卡片里的话，`left: -12px` 那种"伸出去"的写法会落在窗口边界外 ——
   * 透明窗口画不出边界之外的东西，整条箭头会直接消失。
   */
  return (
    <div className={`${s.root} ${side === 'left' ? s.left : s.right}`}>
      <span className={s.arrow} aria-hidden />

      <div className={s.card}>
        <div className={s.body}>
          {/* 副标题：一句"为什么"，小字灰字 —— 解释不该抢操作步骤的地方 */}
          <p
            className={s.lead}
            title={tr('guide.card.leadTitle')}
          >
            {tr('guide.card.lead')}
          </p>

          {/* ★ 正文 = 操作步骤。★ 每一段都写成**一整行**：JSX 会把段落中间的换行
              折成一个空格，中文里那个空格会跟着标点一起显示出来（"打印板， 再选回…"）。 */}
          <p className={s.text}>
            {tr('guide.body1.pre')}
            {tr('guide.body1.strong')}
            {tr('guide.body1.post')}
          </p>
          <p className={s.text}>
            {tr('guide.body2.pre')}
            {tr('guide.body2.strong')}
            {tr('guide.body2.post')}
          </p>
          {err && <p className={s.err}>{err}</p>}
        </div>

        <div className={s.actions}>
          <button type="button" className={s.primary} onClick={close} disabled={busy}>
            {tr('guide.ok')}
          </button>
        </div>
      </div>
    </div>
  )
}
