import { useEffect, useState } from 'react'

import { api } from '../../api'

/*
 * 本机学习账 —— 「哪份预设后处理跑得最多」。
 *
 * 盘上是 `<appDataDir>/run/usage.json`（`runtime::usage`），钥匙两种：
 *   `local:<相对用户根>`   `presets-local/A1-fast.toml`
 *   `cloud:<相对内部根>`   `delivery/mkp/presets/A1-fast.toml`
 * 计数点在后处理**成功**那一趟（机型不符 / 中途失败都不加），所以它说的是
 * "这份真正跑成过几次"，不是"点过几次使用"。
 *
 * 它**只用来显示**，不许拿来重排任何列表（作者：「不要动这些列表的」）。
 *
 * 读法两条：
 *   · 整本一次性取回（`get_preset_usage` 给的是全书，不是按份查）—— 所以模块级
 *     缓存一份就够，换页/换版不必等一次 IPC 才"长出"数字；
 *   · `revision` 变一次重读一次（首页传当前版本组合、预设页传展开的那一行）：
 *     这个数是**随时会变的本机事实**（用户刚跑完一趟后处理），挂载时读一次就再也不
 *     读的话，常在屏上的页面会一直显示旧数字。
 */
let book: Map<string, number> | null = null

export function usePresetUsage(revision: unknown = null): Map<string, number> {
  const [usage, setUsage] = useState<Map<string, number>>(() => book ?? new Map())

  useEffect(() => {
    let alive = true
    void api.getPresetUsage().then(
      (list) => {
        book = new Map(list.map((e) => [e.key, e.count]))
        if (alive) setUsage(book)
      },
      () => {
        /* 读不到 = 全 0：这只是"用得多不多"，不是任何判据，不弹错 */
      },
    )
    return () => {
      alive = false
    }
  }, [revision])

  return usage
}
