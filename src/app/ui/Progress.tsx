/*
 * 细进度条（后处理那一屏在用）。
 *
 * 原来是 `ui/Modal.tsx` 里的一个 export —— 2026-10-10 那套**稿内模态框退役**
 * （作者：「为什么会有两个模态框不行？我们要统一只能用一种」），它没有别的地方可去，
 * 就在 `ui/` 下自立一格；样式仍是 `ui.module.css` 的 `.bar` / `.barFill`。
 */

import s from './ui.module.css'

export function Progress({ percent }: { percent: number }) {
  return (
    <span className={s.bar}>
      <span className={s.barFill} style={{ width: `${Math.min(100, Math.max(0, percent))}%` }} />
    </span>
  )
}
