/*
 * 产品资源区（`mkp/bbs/`）里的 BBS 配置 —— 第三圈第一刀的消费面。
 *
 * # 为什么单开一个 hook
 *
 * 这一页原来读的是**安装包里那份随包副本**（`public/bbs/` 的元数据、`public/assets/bbs/`
 * 的切片器配置）。那份已经退役：**外部资源归 catalog 管**，按需下载、落在 `mkp/bbs/` 下。
 * 于是这里有三件事必须分清楚，混在一起就会变成"看起来读不到、其实没下载"那种说不清的状态：
 *
 *   1. **目录里登记了哪些**（`listed`）—— 有没有下载都在，回答"世界上有什么"；
 *   2. **本机已经有哪些**（`downloaded`）—— 盘就是底账，不记账本；
 *   3. **读一份出来**（`read`）—— 走 IPC，前端不碰文件系统。
 *
 * **"一份都没下载"是合法状态，不是坏掉**：界面要说清去哪儿把它弄下来，
 * 而不是显示一个空列表让人以为产品本来就不带这个东西。
 */

import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api'
import type { BbsPresetDoc } from './bbsTypes'

/** catalog 里 BBS 配置那一类的 kind（与 Rust 侧 `runtime::catalog::kind::BBS_CONFIG` 同一个词） */
const KIND_BBS = 'bbs_config'

export interface BbsDeliveredItem {
  fileName: string
  /** 归属机型。**空串 = 这份不属于某台机器**（目录里的资产可以是公共的） */
  machineId: string
  size: number
}

export interface BbsDelivered {
  /** 目录里登记的那几份 BBS 配置 */
  listed: BbsDeliveredItem[]
  /** 盘上已经有的那几份（在、且 SHA 与目录一致才算） */
  downloaded: BbsDeliveredItem[]
  /** 读一份的正文。**没下载时后端会给理由，这里原样抛出去**，不自己造句 */
  read: (fileName: string) => Promise<BbsPresetDoc>
  /** 取数失败的说法。空串 = 正常 */
  note: string
}

export function useBbsDelivered(): BbsDelivered {
  const [listed, setListed] = useState<BbsDeliveredItem[]>([])
  const [downloaded, setDownloaded] = useState<BbsDeliveredItem[]>([])
  const [note, setNote] = useState('')

  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const [catalog, names] = await Promise.all([
          api.getRuntimeCatalog(),
          api.getDownloadedFiles(),
        ])
        if (!alive) return
        const all = catalog.files
          .filter((f) => f.kind === KIND_BBS)
          .map((f) => ({ fileName: f.fileName, machineId: f.machineId, size: f.size }));
        const have = new Set(names)
        setListed(all)
        setDownloaded(all.filter((f) => have.has(f.fileName)))
        setNote('')
      } catch (e) {
        if (!alive) return
        setNote((e as { message?: string }).message ?? '产品资源区读不出来')
      }
    })()
    return () => {
      alive = false
    }
  }, [])

  const read = useCallback(async (fileName: string) => {
    const text = await api.readDownloadedText(fileName)
    return JSON.parse(text) as BbsPresetDoc
  }, [])

  return { listed, downloaded, read, note }
}
