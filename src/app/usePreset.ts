import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { Preset } from '../api/contract'
import type { Selection } from './components/MachinePicker'

export type { Preset }

/**
 * 预设的加载态。
 *
 * `idle` 是"还没选齐三级 / 后端没有这一份"，不是"空闲等着下载" —— 没选完就没有"哪一份预设"可谈。
 * 中间态只有 `waiting` 会真的停留：连接慢、下载快（文件几十 KB），
 * `downloading` 现实里一闪而过，留着只是为了能在调试面板里停住看样式。
 */
export type PresetState =
  | { status: 'idle' }
  | { status: 'waiting'; name: string }
  | { status: 'downloading'; name: string }
  | { status: 'failed'; name: string }
  | { status: 'ready'; preset: Preset }

/**
 * 文件名在拿到响应之前是不知道的。
 *
 * 旧版能直接从 mock 表里查出名字（那张表就在前端），走了接口之后这个前提没了 ——
 * 等待期间宁可显示一个中性占位，也不猜一个名字出来（PresetStack 在 waiting 态本来就渲染骨架屏，
 * 这个串只在「调试面板强制 downloading / failed 且还没拿到响应」这一种情况下露脸）。
 */
const PENDING_NAME = '预设文件'

/**
 * 取预设：请求走 src/api，状态机在这里。
 *
 * 试验场那份还叠了一层 `devStore.presetPhase` —— 调试面板可以强制把界面按 waiting /
 * downloading / failed 显示，用来看样式。产品里没有面板，那层覆盖整个去掉，
 * 状态只由真实请求决定；`PresetState` 的形状没变，所以 PresetStack、CalibHead、useCalibration
 * 里的判断都不用动。
 *
 * `retry` 是新加的：失败态那颗「重试」按钮在试验场里调的是面板开关（只改显示、不重发），
 * 这里让它真的再发一次请求。
 */
export function usePreset(sel: Selection): { state: PresetState; retry: () => void } {
  const [state, setState] = useState<PresetState>({ status: 'idle' })
  /** 重试计数：变一次就重跑下面那条 effect */
  const [attempt, setAttempt] = useState(0)
  const retry = useCallback(() => setAttempt((n) => n + 1), [])

  const variant = sel.variant
  const complete = Boolean(sel.brand && sel.model && variant)

  useEffect(() => {
    if (!complete || !variant) {
      setState({ status: 'idle' })
      return
    }

    /* 选择在请求回来之前又改了：旧那一份的结果直接丢掉，不许它盖掉新的 */
    let alive = true
    setState({ status: 'waiting', name: PENDING_NAME })

    api.getPreset(variant).then(
      (preset) => {
        if (!alive) return
        // 后端没有这一份（新增了机型版本但没配预设）：当没选，别拿半份数据糊弄
        setState(preset ? { status: 'ready', preset } : { status: 'idle' })
      },
      (err: unknown) => {
        if (!alive) return
        console.error('[preset] 取预设失败', err)
        setState({ status: 'failed', name: PENDING_NAME })
      },
    )

    return () => {
      alive = false
    }
  }, [complete, variant, attempt])

  /* state 只在真的变化时才换新对象（setState 的语义），retry 恒定，
     所以 useCalibration 那条「预设换了就作废草稿」的 effect 依赖仍然稳定 */
  return { state, retry }
}
