/*
 * 通知停泊区（把 `useNotices()` 里那几条画出来）。
 *
 * 挂在 `App.tsx` 根部**一次**即可。用它内部走 `createPortal(…, document.body)`
 * —— 见 `.module.css` 里那段：标题栏有 `mask-image`，任何住在它里面的浮层都会被裁。
 */

import { createPortal } from 'react-dom'
import { useT, useText } from '../../i18n'
import { dismiss, useNotices, type Notice } from './notify'
import s from './NotifyHost.module.css'

function NoticeCard({ n }: { n: Notice }) {
  const tr = useT()
  const say = useText()
  return (
    <div className={s.card} data-kind={n.kind} role="status" aria-live="polite">
      <span className={s.dot} aria-hidden="true" />

      <div className={s.body}>
        <div className={s.title}>{say(n.title)}</div>
        {n.text !== undefined && <div className={s.text}>{say(n.text)}</div>}

        {n.progress !== undefined && (
          <div className={s.progressTrack}>
            {/* 进度条走 scaleX（见 CSS 注释）；`0~1` 夹一下，防止后端给越界值 */}
            <div
              className={s.progressBar}
              style={{ transform: `scaleX(${Math.max(0, Math.min(1, n.progress))})` }}
            />
          </div>
        )}

        {n.actions !== undefined && n.actions.length > 0 && (
          <div className={s.actions}>
            {n.actions.map((a, i) => (
              <button
                key={i}
                type="button"
                className={a.primary === true ? `${s.btn} ${s.btnPrimary}` : s.btn}
                disabled={a.disabled === true}
                onClick={a.onClick}
              >
                {say(a.label)}
              </button>
            ))}
          </div>
        )}
      </div>

      {n.closable !== false && (
        <button type="button" className={s.close} aria-label={tr('common.close')} onClick={() => dismiss(n.id)}>
          ×
        </button>
      )}
    </div>
  )
}

export default function NotifyHost() {
  const notices = useNotices()
  if (notices.length === 0) return null

  return createPortal(
    <div className={s.host}>
      {notices.map((n) => (
        <NoticeCard key={n.id} n={n} />
      ))}
    </div>,
    document.body,
  )
}
