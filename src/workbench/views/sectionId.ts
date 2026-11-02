/*
 * 新分组的 id 规则 —— **全仓一份**。
 *
 * 为什么单独一个文件：有两个入口会建分组，都必须是"同一种东西"：
 *
 *   · 参数定义框的「所属分组 → ＋」—— 建了跟着这条定义一起落盘；
 *   · 分组抽屉的「＋ 新增分组」—— 建完直接是空组，可以立刻删。
 *
 * 两个入口的规则（前缀 `custom_`、跳号挑下一个空位、后端 `ensure_section` 幂等）
 * 一旦各写一遍，改一处必然漏另一处。放这里两边都 import。
 *
 * 不写在 `ParamDefModal.tsx` 里还有个硬原因：组件文件额外导出函数会触发
 * `react-refresh/only-export-components` —— Fast Refresh 会把那个文件当成"模组"
 * 而不是"组件"，改它的时候整棵子树重挂。
 */

/** 新分组的 id 前缀。中文标签没法当 id（它会进 TOML 与客户端目录），所以名字归名字、id 归 id */
export const NEW_SECTION_PREFIX = 'custom_'

/**
 * 下一个可用的新分组 id。跳过已经占了的（盘上的 + 草稿里刚建的那个）。
 *
 * @param known 现在已经存在的分组（只需要有 `id`）
 */
export function nextSectionId(known: { id: string }[]): string {
  const used = new Set(known.map((x) => x.id))
  for (let n = 1; ; n += 1) {
    const id = `${NEW_SECTION_PREFIX}${n}`
    if (!used.has(id)) return id
  }
}
