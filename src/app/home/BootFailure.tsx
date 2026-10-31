/*
 * 首屏数据读不出来时的那一屏（2026-10-31）。
 *
 * # 它不是什么
 *
 * 它**不是**启动画面 —— 启动阶段的画面就是 `index.html` 那具骨架，从头到尾只有它
 * （作者：「别说 MKP 了，就直接用骨架」）。这一屏只在"读完了但没读到"时出现：
 * 那一刻骨架已经收掉（继续摆着就成了骗人），必须有人说清现在是什么状况。
 *
 * # 为什么要它
 *
 * 首屏要按「底账 + 机型目录」决定显示正式版面还是欢迎页。读失败时目录是空的，
 * 如果放行，首页会把"没读到"渲染成"用户从来没选过机型"—— 那正是这次要消灭的
 * 错误中间态。所以失败停在这里：一句话 + 「重试」，重试把骨架放回来重新读。
 *
 * ★ 文案只有一句、不摆技术细节：原因在日志里，用户要的是"能不能再试一次"。
 */

import { useT } from '../../i18n'
import { Btn } from '../ui/Controls'
import b from './BootFailure.module.css'

export interface BootFailureProps {
  /** 重读首屏数据（`useCatalog().reload`）；重试期间骨架会重新盖回来 */
  onRetry: () => void
}

export default function BootFailure({ onRetry }: BootFailureProps) {
  const tr = useT()
  return (
    <div className={b.fail} role="alert">
      <p className={b.note}>{tr('app.boot.failed')}</p>
      <Btn variant="primary" onClick={onRetry}>
        {tr('app.boot.retry')}
      </Btn>
    </div>
  )
}
