/*
 * 「云端出了新版，这台机器上还是旧的」——**那一块"怎么办"**（判据在 `staleCatalog.ts`）。
 *
 * 2026-10-19（作者：「你显示给用户的，不只是那个错误标题，你要给一个解决办法，
 *   显示在下面内容区域」）：
 *
 * 交付校验失败那一句（`… 的内容对不上`）以前只有**标题格一行红字**（`HeadNote`），
 * 用户看见"对不上"却不知道下一步做什么。现在：标题格管"出了什么事"，
 * **内容区这一块管"怎么办"** —— 两件事不再挤在一句话里。
 *
 * 摆放位置由调用方定（都在模态框内容区那句状态下面）。
 */

import { useT } from '../../i18n'
import s from './StaleCatalogHelp.module.css'

/**
 * 那一块"怎么办"。
 *
 * 文案只说**用户要做的动作**（关掉再打开 → 回来再点一次），不解释成因、
 * 不摆 SHA / revision —— 那些是排查用的，住在后端 `detail` 与日志里。
 */
export function StaleCatalogHelp() {
  const tr = useT()
  return (
    <div className={s.help}>
      <span className={s.title}>{tr('catalog.stale.title')}</span>
      <span className={s.body}>{tr('catalog.stale.body')}</span>
    </div>
  )
}
