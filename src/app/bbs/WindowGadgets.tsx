/*
 * 工艺页工具条右端的两枚开关（2026-10-17，作者：「置顶」+「窄窗」）。
 *
 * # 为什么长在这里
 *
 * 作者原话：「一般是工艺这一页用得上，因为用户可以置顶，然后小窗对比」——
 * 他要的用法是**把本程序摆到切片器旁边**，一边看切片器的参数、一边看我们这份
 * （BBS 预设 / MKP 工艺）。所以开关放在"要看的那一页"上，而不是设置页：
 * 伸手就到，用完就关。
 *
 * # 两枚胶囊各自说什么
 *
 * · **置顶**（三档循环）：`不置顶 → 仅本页置顶 → 始终置顶 → 不置顶`。
 *   "仅本页"= 只有工艺页置顶 —— 切到别的页签自动解除，切回来又浮上去
 *   （落地在 `App.tsx` 那条 `[pin, tab]` 的 effect 里，这里只管切档）。
 * · **小窗**（开关）：宽 600（窗口最小宽）、高 = 屏幕可用高 90%、贴右缘，
 *   并**顺手置顶**（不置顶就会被切片器盖住）；再点一次「还原」回到
 *   **上一次手动摆放的位置与尺寸**，置顶也放回原来那一档。
 *   ★ 文案是作者 2026-10-17 亲口给的（原来叫「窄窗 / 恢复尺寸」，他不要）。
 *
 * # 不许有像素位移
 *
 * 作者 2026-10-17：「点击的时候好像有像素位移」—— 那是按下的 `translateY(1px)`
 * 与切换态的字重/内衬一起造成的。现在：**按下不移位**、档位只换颜色与文字，
 * `min-width` 按最长那档预留 ⇒ 点一万次盒子也一个像素不动。
 *
 * # 视觉口径
 *
 * 作者要「低调不显眼不占位」⇒ 平时只是一枚带发丝描边的文字胶囊（不是图标按钮，
 * 图标读不出"置顶"这两个字）；开启时换 accent 字 + 淡染底，仍**不加粗、不改宽**。
 * ★ 三档文字长度不同，`min-width` 按最长那档预留 ⇒ 切换时右边一个像素都不动
 * （全站铁律：状态不许改变盒子宽度）。
 *
 * ★ 窗口真动作全在 Rust（`ipc::window_layout`）：这里只发意图、只读 AppState 那两格。
 *   浏览器预览里后端是 mock（空转、照常记账），于是这一段在网页里也点得动。
 */
import { useState } from 'react'

import { api, errorText } from '../../api'
import type { PinPref } from '../../api/contract'
import { notify } from '../notify/notify'
import { appStateMutated, useCompactWindow, usePinPref } from '../state/appState'
import s from './WindowGadgets.module.css'

/** 三档的文字（直白 > 简短：作者要"一眼知道现在是哪一档"） */
const PIN_TEXT: Record<PinPref, string> = {
  off: '不置顶',
  page: '仅本页置顶',
  always: '始终置顶',
}

/** 点一下会变成哪一档（`title` 里说清，不靠猜） */
const PIN_NEXT: Record<PinPref, PinPref> = {
  off: 'page',
  page: 'always',
  always: 'off',
}

const pinTitle = (pin: PinPref) =>
  `窗口置顶：${PIN_TEXT[pin]} · 点一下改成「${PIN_TEXT[PIN_NEXT[pin]]}」`

export default function WindowGadgets() {
  const pin = usePinPref()
  const compact = useCompactWindow()
  const [busy, setBusy] = useState(false)

  /*
   * 两枚胶囊都不能连点：窗口动作是"先落盘再搬窗"，连点会让两次搬窗叠在一起。
   * 失败**不改文字**（界面上显示的永远是盘上那一档），只把原因讲成人话。
   */
  const guard = async (what: string, run: () => Promise<unknown>) => {
    if (busy) return
    setBusy(true)
    try {
      await run()
      /* 浏览器 mock 没有 app-state 事件通道 —— 写完兜底刷一次快照（桌面端只是多一次重读） */
      appStateMutated()
    } catch (e) {
      notify({ kind: 'bad', title: `${what}没改成`, text: errorText(e) })
    } finally {
      setBusy(false)
    }
  }

  /* 这一页就是工艺页 —— 点它的时候用户一定站在这里，所以 `onBbs` 恒 true */
  const cyclePin = () =>
    void guard('置顶', () => api.setPin(PIN_NEXT[pin], true))

  const toggleCompact = () =>
    void guard('窗口尺寸', () => api.setCompact(!compact))

  return (
    <div className={s.gadgets}>
      <button
        type="button"
        className={s.pill}
        data-on={pin !== 'off'}
        data-mode={pin}
        aria-label={`窗口置顶：${PIN_TEXT[pin]}`}
        title={pinTitle(pin)}
        onClick={cyclePin}
      >
        {PIN_TEXT[pin]}
      </button>

      <button
        type="button"
        className={s.pill}
        data-on={compact}
        aria-pressed={compact}
        aria-label={compact ? '还原窗口原本的位置与尺寸' : '把窗口收成小窗'}
        title={
          compact
            ? '还原：回到上一次手动摆放的位置与大小，置顶也回到原来那一档'
            : '小窗：600 宽、贴屏幕右侧、高度占屏幕可用区 90%，并自动置顶'
        }
        onClick={toggleCompact}
      >
        {compact ? '还原' : '小窗'}
      </button>
    </div>
  )
}
