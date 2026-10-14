/*
 * 开发台：**把那一格所在的页签切到前台**（作者 2026-10-14）。
 *
 * 为什么需要它：认领方（重命名 / 导出 / 导入那一格）住在各自的页签上，而页签是
 * "全挂载、只藏不卸"（`PageSlot hidden`）—— 框其实**已经画出来了**，只是它那一页
 * 被藏着，用户看不见。所以外壳要替它把那页签切过去。
 *
 * ★ 现在可能**同时摆着好几格**（异类并存），取**最后打开的那一格**所在的页签
 * （`nOnce` 最大的那个）—— 用户刚点的那个才是他想看的。
 *
 * 映射只写一行：哪个 slug 住哪一页。认领方自己不切页签（**跨页跳转的出口由外壳给**，
 * 与 `openBbs` / `openSettings` 同一条规矩）。
 *
 * 真机上收到这条事件的唯一来源是开发台的 `wb_dev_trigger` —— 正式包里没有，
 * 于是这个组件永远是"没反应"的空壳。
 */

import { useEffect } from 'react'

import { useDevPreviewAll } from './useDevPreview'

/** slug → 页签 id（页签 id 住 `App.tsx` 的 `tab` 那一格：`machine` / `preset` / `report` / …） */
const OWNER_TAB: Record<string, string> = {
  'import-collision': 'preset',
  'import-result': 'preset',
  rename: 'preset',
  'delete-confirm': 'preset',
  compare: 'preset',
  export: 'report',
  'float-toast': 'report',
}

export function DevPreviewTabs({ onTab }: { onTab: (tab: string) => void }) {
  const all = useDevPreviewAll()
  /* 异类并存时会有好几格 —— 取**最后点的那一格**（`useDevPreviewAll` 已按打开先后排好） */
  const latest = all.length === 0 ? null : all[all.length - 1]
  const scene = latest?.scene ?? null
  const nonce = latest?.nOnce ?? 0
  useEffect(() => {
    if (scene === null) return
    const tab = OWNER_TAB[scene]
    if (tab !== undefined) onTab(tab)
    /* 认 nonce：同一格再点一次也要再切过去（用户可能已经自己切走了） */
  }, [scene, nonce, onTab])
  return null
}
