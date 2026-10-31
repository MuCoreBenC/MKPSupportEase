/*
 * 下载那两句话：**过程水位**与**逐份结局**。
 *
 * 为什么单独一个文件、且是共用的：下载过程两处说的是同一件事 —— 一次调用一路
 * Tauri Channel 水位（`DownloadTick`）。各写一套文案，迟早会有一处漏说
 * 「服务端没报总长度」那一档，或者一处改了动词另一处没改
 * （本仓库的规矩：组件文件只导出组件，话术与常量分开放，见 `shared/note.ts`）。
 * 原来这份文案是「同步页 + 预设页」各来一份再抽出来的；同步页 2026-10-02 退役，
 * 现在的消费者是预设页（单份下载 + 批量 + 更新都走这里）。
 *
 * 纪律：**没有总长度就说收了多少字节**，不为了凑一个百分比编一个分母 ——
 * 服务端不报 `Content-Length` 是常事。
 */

import type { DownloadOutcome, DownloadTick } from '../../api/contract'
import { msg, type Msg } from '../../i18n'

/**
 * 把一次下载的水位说成一句话。五态由后端给（`DownloadTick.stage`），这里只负责措辞。
 *
 * ★ 交出的是 [`Msg`]（键 + 变量）而不是成品句子：这几句会被**存进提示条的 state**，
 *   存成品的话换语言之后它还挂着旧语言（2026-10-29 作者截图）。
 */
export function tickText(t: DownloadTick): Msg {
  switch (t.stage) {
    case 'connecting':
      return msg('download.tick.connecting', { name: t.fileName })
    case 'transferring':
      return t.total !== null && t.total > 0
        ? msg('download.tick.transferringPct', {
            name: t.fileName,
            pct: Math.round((t.received / t.total) * 100),
            received: t.received,
            total: t.total,
          })
        : msg('download.tick.transferringBytes', { name: t.fileName, received: t.received })
    case 'done':
      return msg('download.tick.done', { name: t.fileName })
    case 'failed':
      return t.message !== undefined && t.message !== ''
        ? msg('download.tick.failedWhy', { name: t.fileName, message: t.message })
        : msg('download.tick.failed', { name: t.fileName })
  }
}

/**
 * 批量里**没成的那一份**说成人话。调用方只传失败的那些（成了的不占版面）。
 *
 * 后端约定失败必带原因（`FileOutcome` 的注释：成功了才是空的），所以空原因这一支
 * 是兜底 —— 它出现就说明后端漏说了，界面上宁可写"原因没说清"也不许写"失败"两个字了事。
 */
export function outcomeText(o: DownloadOutcome): Msg {
  return o.message === ''
    ? msg('download.outcome.failed', { name: o.fileName })
    : msg('download.outcome.failedWhy', { name: o.fileName, message: o.message })
}
