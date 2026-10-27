/*
 * 交付面（`assets/bbs/…`）里的 BBS 配置 —— 第三圈第一刀的消费面。
 *
 * # 为什么单开一个 hook
 *
 * 这一页原来读的是**安装包里那份随包副本**（`public/bbs/` 的元数据、`public/assets/bbs/`
 * 的切片器配置）。那份已经退役：**外部资源归 catalog 管**，按需下载、落在
 * `catalog.path` 说的那一格（`assets/bbs/…`，也是客户端的落点）。
 * 于是这里有三件事必须分清楚，混在一起就会变成"看起来读不到、其实没下载"那种说不清的状态：
 *
 *   1. **目录里登记了哪些**（`listed`）—— 有没有下载都在，回答"世界上有什么"；
 *   2. **本机已经有哪些**（`downloaded`）—— 盘就是底账，不记账本；
 *   3. **已经复制进切片器目录的有哪几份**（`copied`，2026-10-13 加）—— 那一批会在
 *      本机 BBS 目录里留下一份同名的（`copy_to_slicer` 就是复制过去），BBS 页据此
 *      把「本机工艺」档里那几份标成"我们发的"；
 *   4. **读一份出来**（`read`）—— 走 IPC，前端不碰文件系统。
 *
 * **"一份都没下载"是合法状态，不是坏掉**：界面要说清去哪儿把它弄下来，
 * 而不是显示一个空列表让人以为产品本来就不带这个东西。
 */

import { t } from '../../i18n'
import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api'
import { useDeliveryRevision } from '../state/deliveryState'
import type { BbsPresetDoc } from './bbsTypes'

/** catalog 里 BBS 配置那一类的 kind（与 Rust 侧 `runtime::catalog::kind::BBS_CONFIG` 同一个词） */
const KIND_BBS = 'bbs_config'

export interface BbsDeliveredItem {
  fileName: string
  /** 归属机型。**空串 = 这份不属于某台机器**（目录里的资产可以是公共的） */
  machineId: string
  /**
   * 目录里登记的落点（`assets/bbs/Process/0.4mm/…`）。
   *
   * 2026-10-13 起这一格有人用了：BBS 页按**目录登记**认机型与喷嘴（`machineId` 给机型、
   * 这一段里的 `Process/<喷嘴>mm/` 给喷嘴），不再顺继承链去猜 ——
   * 于是"这台机器上装没装那台机的系统预设"与识别结果无关。
   */
  path: string
  size: number
}

export interface BbsDelivered {
  /** 目录里登记的那几份 BBS 配置 */
  listed: BbsDeliveredItem[]
  /** 盘上已经有的那几份（在、且 SHA 与目录一致才算） */
  downloaded: BbsDeliveredItem[]
  /** 已经复制进切片器目录的那几份（**文件名**；与本机 BBS 目录里的同名对得上） */
  copied: string[]
  /** 读一份的正文。**没下载时后端会给理由，这里原样抛出去**，不自己造句 */
  read: (fileName: string) => Promise<BbsPresetDoc>
  /** 取数失败的说法。空串 = 正常 */
  note: string
}

export function useBbsDelivered(): BbsDelivered {
  const [listed, setListed] = useState<BbsDeliveredItem[]>([])
  const [downloaded, setDownloaded] = useState<BbsDeliveredItem[]>([])
  const [copied, setCopied] = useState<string[]>([])
  const [note, setNote] = useState('')
  /*
   * 「盘上有哪几份」是**投递面**的事：任何页面下载 / 删除之后代次 +1，这里跟着重读。
   * 页签常驻（`App.tsx` 2026-10-05 起不重挂载）—— 不订阅就永远拿着首读那一份
   * （2026-10-07 与首页 / 预设页同一条病，同一味药）。
   */
  const deliveryRevision = useDeliveryRevision()

  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const [catalog, files, copiedNow] = await Promise.all([
          api.getRuntimeCatalog(),
          api.getDownloadedFiles(),
          /* 「已经复制到切片器」是另一条命令（它在用户线那一族里读切片器目录）——
             复制完预设页会广播一次投递代次，这里跟着重读 */
          api.getSlicerCopied(),
        ])
        if (!alive) return
        const all = catalog.files
          .filter((f) => f.kind === KIND_BBS)
          .map((f) => ({
            fileName: f.fileName,
            machineId: f.machineId,
            path: f.path,
            size: f.size,
          }));
        const have = new Set(files.map((f) => f.fileName))
        setListed(all)
        setDownloaded(all.filter((f) => have.has(f.fileName)))
        setCopied(copiedNow)
        setNote('')
      } catch (e) {
        if (!alive) return
        setNote((e as { message?: string }).message ?? t('bbs.delivered.failed'))
      }
    })()
    return () => {
      alive = false
    }
  }, [deliveryRevision])

  const read = useCallback(async (fileName: string) => {
    const text = await api.readDownloadedText(fileName)
    return JSON.parse(text) as BbsPresetDoc
  }, [])

  return { listed, downloaded, copied, read, note }
}
