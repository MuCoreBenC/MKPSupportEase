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
 * · **置顶**（三档循环）：`不置顶 → 本页置顶 → 始终置顶 → 不置顶`。
 *   按钮上只显示范围词（`关闭 / 本页 / 始终`），完整说法在 title 里。
 *   "本页"= 只有工艺页置顶 —— 切到别的页签自动解除，切回来又浮上去
 *   （落地在 `App.tsx` 那条 `[pin, tab]` 的 effect 里，这里只管切档）。
 * · **小窗**（开关）：宽 600（窗口最小宽）、高 = 屏幕可用高 90%、贴右缘，
 *   并**顺手置顶**（不置顶就会被切片器盖住）；再点一次「还原」回到
 *   **上一次手动摆放的位置与尺寸**，置顶也放回原来那一档。
 *   ★ 文案是作者 2026-10-17 亲口给的（原来叫「窄窗 / 恢复尺寸」，他不要）。
 *
 * # 文案从字典来（2026-10-25，全站 i18n 第一刀）
 *
 * 这个文件是**第一个迁到 `src/i18n` 的组件**：所有用户能读到的字（按钮文字 /
 * `title` / `aria-label` / 失败提示）都走 `t()`，这个文件里不再有中文字面量。
 * 按钮上放**短说法**、`title` / `aria-label` 放**完整说法**，两套都是字典键。
 *
 * # 不许有像素位移
 *
 * 作者 2026-10-17：「点击的时候好像有像素位移」—— 那是按下的 `translateY(1px)`
 * 与切换态的字重/内衬一起造成的。现在：**按下不移位**、档位只换颜色与文字，
 * 宽度由"隐形副本"撑到最长那档 ⇒ 点一万次盒子也一个像素不动。
 *
 * # 视觉口径
 *
 * 作者要「低调不显眼不占位」⇒ 平时只是一枚带发丝描边的文字胶囊（不是图标按钮，
 * 图标读不出"置顶"这两个字）；开启时换 accent 字 + 淡染底，仍**不加粗、不改宽**。
 *
 * ★ 宽度**不写死**：每一档的文字各渲染一份叠进同一个 grid 格（见 CSS 与下面那段
 *   `PIN_ORDER.map`），可见的只有当前档 ⇒ 盒子宽度 = **当前语言最长的那一档**。
 *   换语言（英文 `This page` 比「本页」长得多）自动跟着走，切档仍然不动。
 *
 * ★ 窗口真动作全在 Rust（`ipc::window_layout`）：这里只发意图、只读 AppState 那两格。
 *   浏览器预览里后端是 mock（空转、照常记账），于是这一段在网页里也点得动。
 */
import { useState } from 'react'

import { api, errorText } from '../../api'
import type { PinPref } from '../../api/contract'
import { msg, useT, type StringKey } from '../../i18n'
import { notify } from '../notify/notify'
import { appStateMutated, useCompactWindow, usePinPref } from '../state/appState'
import s from './WindowGadgets.module.css'

/**
 * 三档的**按钮上的字**（越短越好：作者要胶囊窄）。
 *
 * ★ 2026-10-25 五轮收窄，后三轮都是靠缩文案换宽度：
 *   「仅本页置顶」(5) → 「本页置顶」(4) → 「本页 / 始终」(2) → **全部 2 个字**。
 *   三档收成一条同类的轴：关闭 / 本页 / 始终（都在回答"置顶到哪一层"）。
 *   ★ 按钮上没"置顶"二字了，完整说法由 `PIN_LONG` 进 `title` 与 `aria-label`
 *     （鼠标停一下、读屏读一遍就知道说的是置顶）。
 *   ★ `off` 用「关闭」而不是「无」：单字在窄胶囊里太孤，
 *     且"关闭"读起来是个明确状态，配 title 的「不置顶」不会误解成"关掉这枚按钮"。
 */
const PIN_SHORT: Record<PinPref, StringKey> = {
  off: 'bbs.gadgets.pin.off',
  page: 'bbs.gadgets.pin.page',
  always: 'bbs.gadgets.pin.always',
}

/** 完整说法（只在 `title` / `aria-label` 里出现，不占按钮宽度） */
const PIN_LONG: Record<PinPref, StringKey> = {
  off: 'bbs.gadgets.pin.offLong',
  page: 'bbs.gadgets.pin.pageLong',
  always: 'bbs.gadgets.pin.alwaysLong',
}

/** 三档的固定顺序（渲染隐形副本时用：顺序稳定，React 不会因切档重建节点） */
const PIN_ORDER: readonly PinPref[] = ['off', 'page', 'always']

/** 点一下会变成哪一档（`title` 里说清，不靠猜） */
const PIN_NEXT: Record<PinPref, PinPref> = {
  off: 'page',
  page: 'always',
  always: 'off',
}

/** 「小窗」那枚的两档（`on` = 现在是小窗态，`off` = 可还原） */
const COMPACT_ORDER: readonly ('on' | 'off')[] = ['on', 'off']

const COMPACT_TEXT: Record<'on' | 'off', StringKey> = {
  on: 'bbs.gadgets.compact.on',
  off: 'bbs.gadgets.compact.off',
}

const COMPACT_ARIA: Record<'on' | 'off', StringKey> = {
  on: 'bbs.gadgets.compact.ariaOn',
  off: 'bbs.gadgets.compact.ariaOff',
}

const COMPACT_TITLE: Record<'on' | 'off', StringKey> = {
  on: 'bbs.gadgets.compact.titleOn',
  off: 'bbs.gadgets.compact.titleOff',
}

export default function WindowGadgets() {
  const pin = usePinPref()
  const compact = useCompactWindow()
  const [busy, setBusy] = useState(false)
  const t = useT()

  /* 小窗那枚现在处于哪一档（决定哪份文字可见、aria/title 用哪条） */
  const compactSide: 'on' | 'off' = compact ? 'off' : 'on'

  /*
   * 两枚胶囊都不能连点：窗口动作是"先落盘再搬窗"，连点会让两次搬窗叠在一起。
   * 失败**不改文字**（界面上显示的永远是盘上那一档），只把原因讲成人话。
   */
  const guard = async (failTitle: StringKey, run: () => Promise<unknown>) => {
    if (busy) return
    setBusy(true)
    try {
      await run()
      /* 浏览器 mock 没有 app-state 事件通道 —— 写完兜底刷一次快照（桌面端只是多一次重读） */
      appStateMutated()
    } catch (e) {
      notify({ kind: 'bad', title: msg(failTitle), text: errorText(e) })
    } finally {
      setBusy(false)
    }
  }

  /* 这一页就是工艺页 —— 点它的时候用户一定站在这里，所以 `onBbs` 恒 true */
  const cyclePin = () =>
    void guard('bbs.gadgets.pin.failed', () => api.setPin(PIN_NEXT[pin], true))

  const toggleCompact = () =>
    void guard('bbs.gadgets.compact.failed', () => api.setCompact(!compact))

  return (
    <div className={s.gadgets}>
      <button
        type="button"
        className={s.pill}
        data-on={pin !== 'off'}
        data-mode={pin}
        aria-label={t('bbs.gadgets.pin.aria', { now: t(PIN_LONG[pin]) })}
        data-tip={t('bbs.gadgets.pin.title', {
          now: t(PIN_LONG[pin]),
          next: t(PIN_LONG[PIN_NEXT[pin]]),
        })}
        onClick={cyclePin}
      >
        {/* 三档各渲染一份叠在同一格：只有当前档可见，其余隐形但占宽
            ⇒ 盒子宽度 = 当前语言里最长的那一档，切档一个像素不动，
              换语言（英文 / 以后的日文）宽度自动跟着走，不写死任何像素 */}
        {PIN_ORDER.map((k) => (
          <span key={k} aria-hidden={k === pin ? undefined : true}>
            {t(PIN_SHORT[k])}
          </span>
        ))}
      </button>

      <button
        type="button"
        className={s.pill}
        data-on={compact}
        aria-pressed={compact}
        aria-label={t(COMPACT_ARIA[compactSide])}
        data-tip={t(COMPACT_TITLE[compactSide])}
        onClick={toggleCompact}
      >
        {/* 同一套撑宽法：两档文字各一份，宽的一档决定盒子宽度 */}
        {COMPACT_ORDER.map((k) => (
          <span key={k} aria-hidden={k === compactSide ? undefined : true}>
            {t(COMPACT_TEXT[k])}
          </span>
        ))}
      </button>
    </div>
  )
}
