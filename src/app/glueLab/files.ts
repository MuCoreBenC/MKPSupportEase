/*
 * 拖入文件时的配对规则。
 *
 * 浏览器里的 `File` 没有磁盘路径：拖一个 gcode 时，JS 拿不到它旁边的 meta。
 * 所以这里按**文件名**配对 —— 同时拖、分两次拖、拖 zip，都是同一条规则：
 * `xxx.gcode` ↔ `xxx_meta.json`。纯函数只认名字，不碰 File / 解压，方便单测。
 */

/** 去掉路径与后缀（`a/b/x.gcode` → `x`） */
export function stemOf(name: string): string {
  const base = name.split(/[\\/]/).pop() ?? name
  return base.replace(/\.[^.]+$/, '')
}

/** `xxx.gcode` → `xxx_meta.json` */
export function metaNameOf(gcodeName: string): string {
  return `${stemOf(gcodeName)}_meta.json`
}

/** `xxx_meta.json` → `xxx.gcode`（不是 meta 名字就返回 null） */
export function gcodeNameOfMeta(metaName: string): string | null {
  const base = (metaName.split(/[\\/]/).pop() ?? metaName).toLowerCase()
  if (!base.endsWith('_meta.json')) return null
  return `${base.slice(0, -'_meta.json'.length)}.gcode`
}

/**
 * 在一堆名字里找与这个 gcode 配对的 meta；找不到返回 `null`。
 * 优先精确匹配 `xxx_meta.json`，其次忽略大小写。
 */
export function findMetaName(gcodeName: string, names: Iterable<string>): string | null {
  const want = metaNameOf(gcodeName).toLowerCase()
  let loose: string | null = null
  for (const name of names) {
    const base = name.split(/[\\/]/).pop() ?? name
    if (base.toLowerCase() === want) return name
    if (loose === null && base.toLowerCase().endsWith('_meta.json')) loose = name
  }
  /* 只有一个候选 meta 时也认 —— 用户常把两份文件改过大小写 */
  return names instanceof Array && names.filter((n) => /_meta\.json$/i.test(n)).length === 1
    ? loose
    : null
}

/** 分两次拖时，判断这份 meta 是不是在等那个 gcode */
export function pairsWith(gcodeName: string, metaName: string): boolean {
  const want = metaNameOf(gcodeName).toLowerCase()
  const got = (metaName.split(/[\\/]/).pop() ?? metaName).toLowerCase()
  return got === want
}
