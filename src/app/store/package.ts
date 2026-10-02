/*
 * 同步层的时间显示工具。
 *
 * 原来这个文件是客户端同步层的三格底账（说明书 / 本机预设 / 使用中，全住 localStorage）——
 * 那套随 C4 收口整体退役：说明书的对应物是 catalog（`api.getRuntimeCatalog()`）、
 * 本机预设的对应物是下载区 `mkp/`（`api.getDownloadedFiles()`）、使用中的对应物是
 * `run/active-preset.json`（`api.getActivePreset()`）。存储全部回到程序管理的
 * Internal 根（`atomic_write` 纪律），WebView 的 localStorage 不再承载任何底账。
 *
 * 留下来的只有**时间那一格的格式化**：表格 / 展开面板上"官方行 / 发布行 / 老的本机值"
 * 三种来源的写法都在这里认。
 */

/**
 * 时间那一格的解析：三种来源的写法都在这里认。
 *
 *   官方行     `2026-08-26`（假后端按路径推的演示值，只有日期）
 *   发布行     ISO（云端 = 发布时刻；本机 = 下载时刻）
 *   老的本机值 `9/30 16:12`（老版 stamp 切掉了年份 —— 凑不出就不给年，不编）
 */
interface StatDate {
  y: string | null
  mo: string
  d: string
  hh: string | null
  mi: string | null
}
const pad2 = (s: string) => s.padStart(2, '0')

function parseStatDate(raw: string): StatDate | null {
  /* ISO（发布 / 下载时刻）：按**本地时区**取 —— 直接切字符串在跨日时会差一天 */
  if (/^\d{4}-\d{2}-\d{2}T/.test(raw)) {
    const t = new Date(raw)
    if (!Number.isNaN(t.getTime())) {
      return {
        y: String(t.getFullYear()),
        mo: pad2(String(t.getMonth() + 1)),
        d: pad2(String(t.getDate())),
        hh: pad2(String(t.getHours())),
        mi: pad2(String(t.getMinutes())),
      }
    }
  }
  /* 纯日期（演示值） */
  let m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(raw)
  if (m !== null) return { y: m[1], mo: m[2], d: m[3], hh: null, mi: null }
  /* 老的本机下载时刻 `9/30 16:12` */
  m = /^(\d{1,2})\/(\d{1,2})\s+(\d{1,2}):(\d{2})/.exec(raw)
  if (m !== null) return { y: null, mo: pad2(m[1]), d: pad2(m[2]), hh: pad2(m[3]), mi: m[4] }
  return null
}

/** 行上那一列：只写月-日，全表统一 `MM-DD`（补零、横杠）——不管官方 / 我的 / 发布 */
export function shortStatText(raw: string | undefined): string | undefined {
  if (raw === undefined) return undefined
  const t = parseStatDate(raw)
  return t === null ? raw : `${t.mo}-${t.d}`
}

/** 展开面板（预设页）：带年份；有时刻就缀 `HH:mm`（老值没有年份就只给月-日，不编） */
export function longStatText(raw: string | undefined): string | undefined {
  if (raw === undefined) return undefined
  const t = parseStatDate(raw)
  if (t === null) return raw
  const date = t.y === null ? `${t.mo}-${t.d}` : `${t.y}-${t.mo}-${t.d}`
  return t.hh === null ? date : `${date} ${t.hh}:${t.mi}`
}
