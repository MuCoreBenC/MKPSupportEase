import s from './PagePlaceholder.module.css'

interface PagePlaceholderProps {
  /** 页签名，与 constants/tabs.ts 里那条一致 */
  title: string
  /** 这一页将来是干什么的，一句话 */
  hint: string
}

/**
 * 还没接进来的那几页。
 *
 * 写清"这一版没有"而不是留白或假装空状态 —— 空白页会让人以为是加载失败或数据为零，
 * 而这几页的真实情况是：界面在试验场里存在，但没随本轮骨架搬过来（见 doc §2.3 补记）。
 */
export default function PagePlaceholder({ title, hint }: PagePlaceholderProps) {
  return (
    <section className={s.wrap}>
      <h2 className={s.title}>{title}</h2>
      <p className={s.hint}>{hint}</p>
      <p className={s.note}>这一页本版未接入</p>
    </section>
  )
}
