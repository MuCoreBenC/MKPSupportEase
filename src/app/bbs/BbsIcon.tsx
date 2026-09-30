/*
 * icons.json 里那些 SVG 的唯一注入口。
 *
 * # 为什么是 dangerouslySetInnerHTML
 *
 * 上游把 BBS 的原件（`check_on/off`、`undo`、`drop_down`、`spin_inc/dec`、`param_*`）
 * 提成了「键名 → SVG 源码字符串」的一张表。要把字符串变成真 DOM，React 里只有这一条路。
 *
 * 风险面就这一句话能说清：**这些字符串来自本仓库自己的资产文件**
 * （`public/bbs/icons.json`，由 `npm run sync:bbs` 从 machine-motion 拷来），
 * 不是用户输入、也不来自网络。用户导入的那份 json 只贡献参数值，不贡献 SVG ——
 * 它走的是 `values`，永远到不了这里。
 *
 * 集中成一个组件而不是每处 `dangerouslySetInnerHTML`：将来要改注入方式（比如换成
 * 预先解析成 React 元素），只动这一个文件；也方便一眼数出「这一页有几处注入」。
 *
 * # 颜色一个都不设
 *
 * BBS 的 param_*.svg 是双色的（轮廓 + 填充块），上游提取时已经把可换色的那几个源色
 * 映射成 `var(--ico-*)`。这里给 color/fill/stroke 只会把双色压平成一团粗灰 ——
 * 那就是「图标看着太粗」的成因。所以本组件只管尺寸。
 */

interface Props {
  /** icons.json 里的键名 */
  name: string | null | undefined
  icons: Record<string, string>
  className?: string
  /** 缺图标时画什么。不给就什么都不画（不占位） */
  fallback?: string
}

export default function BbsIcon({ name, icons, className, fallback }: Props) {
  const svg = name ? icons[name] : undefined
  if (!svg) {
    /* 图标缺了不该拦住整页，也不该留个空框让人以为这里有东西没加载出来 */
    return fallback ? <span className={className}>{fallback}</span> : null
  }
  return <span className={className} dangerouslySetInnerHTML={{ __html: svg }} />
}
