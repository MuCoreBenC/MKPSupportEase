import type { ReactNode } from 'react'

import { t } from '../../i18n'
import s from './SyncBanner.module.css'

interface Props {
  /** 忽略这一条（只藏，不丢数据） */
  onDismiss: () => void
  children: ReactNode
}

/**
 * 云端更新那两条横幅的**共用外壳**。
 *
 * 之前 `PageParams` 里有两段几乎一样的 JSX（一条「有逐项差异」、一条「只知道有新版本」），
 * 连「忽略」按钮都要各写一遍 —— 那种结构改一次就得记得改两处。
 *
 * 这里只收外壳：底色、左边那条主色竖线、语义化的 `role="status"`、右侧的「忽略」。
 * 横幅说什么是调用方的事（两档文案不一样），所以正文走 `children`。
 */
export default function SyncBanner({ onDismiss, children }: Props) {
  return (
    <div className={s.bar} role="status">
      {children}
      <button type="button" className={s.dismiss} onClick={onDismiss}>
        {t('params.sync.dismiss')}
      </button>
    </div>
  )
}
