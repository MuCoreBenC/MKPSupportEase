/*
 * 路径胶囊里那串字的算法（纯函数，与组件分开：组件文件只导出组件）。
 *
 * 只用得上两处：`app/shared/PathPill.tsx` 渲染，报告页 / 日志导出框拼落点。
 */

/**
 * 把"文件夹 + 基名"拼成一个完整路径。
 *
 * `folder` = 落点是一个**文件夹** ⇒ 尾巴补一个分隔符（"落进这个文件夹"）；
 * 落点是文件（zip）时不补。
 */
export function joinExportPath(dir: string, name: string, folder: boolean): string {
  const sep = dir.includes('\\') ? '\\' : '/'
  const base = `${dir.replace(/[\\/]+$/, '')}${sep}${name}`
  return folder ? `${base}${sep}` : base
}

/**
 * 胶囊里那串字拆两段：`<父目录...>` + `<尾两段>`。
 *
 * 分隔符按路径里用的那个认（Windows 收到的是 `\`，macOS 是 `/`）；
 * 段数少到没有"尾两段"可言时整条当尾段，不分。
 */
export function splitPillPath(full: string, folder: boolean): { head: string; tail: string } {
  const sep = full.includes('\\') ? '\\' : '/'
  const lead = full.startsWith(sep) ? sep : ''
  const parts = full
    .replace(/[\\/]+$/, '')
    .split(/[\\/]/)
    .filter((p) => p !== '')
  const tailOf = (list: string[]): string => list.join(sep) + (folder ? sep : '')
  if (parts.length <= 2) return { head: '', tail: tailOf(parts) }
  return { head: lead + parts.slice(0, -2).join(sep) + sep, tail: tailOf(parts.slice(-2)) }
}
