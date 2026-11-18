/*
 * **后处理弹框的预览清单**（`/?pp=list`，dev-only）。
 *
 * 浏览器里到不了真钩子那一趟，`src/api/mock.ts` 按 `/?pp=<名字>` 造一份假快照。
 * 这一页把 `ppScenes.ts` 那张表摊开：一行一屏，点进去看；序号也认（`/?pp=3`）。
 *
 * ★ 只在开发构建里挂（`main.tsx` 用 `import.meta.env.DEV` 分流 + 动态 import），
 *   正式包里的 `?pp=` 什么都不做。
 */
import type { CSSProperties } from 'react'

import s from './PpPreviewIndex.module.css'
import { PP_SCENES } from './ppScenes'

const link: CSSProperties = { color: 'var(--accent)', textDecoration: 'none' }
const dim: CSSProperties = { color: 'var(--text-3)', fontSize: 12 }
const code: CSSProperties = {
  ...dim,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
}

export default function PpPreviewIndex() {
  return (
    /* 整页不滚（`html/body` 都 `overflow: hidden`，见 `global.css`）⇒ 这一层自己滚 */
    <div className={s.scroll}>
      <div style={{ maxWidth: 760, margin: '0 auto', padding: '48px 24px' }}>
        <h1 style={{ margin: '0 0 6px', fontSize: 22, color: 'var(--text-1)' }}>后处理弹框预览</h1>
        <p style={{ ...dim, margin: '0 0 20px', lineHeight: 1.7 }}>
          点一行打开那一屏；序号也认 —— <code style={code}>/?pp=3</code>。
          深色 / 浅色跟设置里那一档走。
        </p>
        <ol
          style={{
            margin: 0,
            paddingLeft: 22,
            display: 'flex',
            flexDirection: 'column',
            gap: 10,
            color: 'var(--text-1)',
          }}
        >
          {PP_SCENES.map((s, i) => (
            <li key={s.id} style={{ lineHeight: 1.6 }}>
              <a href={`/?pp=${s.id}`} style={link}>
                {s.name}
              </a>
              <span style={{ ...dim, marginLeft: 10 }}>{s.note}</span>
              <div style={code}>{`/?pp=${i + 1} · /?pp=${s.id}`}</div>
            </li>
          ))}
        </ol>
      </div>
    </div>
  )
}
