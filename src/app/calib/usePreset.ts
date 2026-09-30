/*
 * 取预设：选齐「机型 + 版本」后，从**文件体系**里组装那一份。
 *
 *   文件名 / 路径   `api.getVersionFiles(machine, version)` 里 `kind === 'mkp_preset'` 的那份
 *   三轴 / 速度     `api.getMachineParams(machine, version)` 的
 *                   `toolhead.offset.x / y / z` + `toolhead.speed_limit`
 *
 * 为什么走文件体系（作者的原话：「首页也应该是消费那个文件，而不是用硬编码」）：
 * 上一版走 `api.getPreset(variantId)`，那是一张手编表、三份全是 A1 mini 的 ——
 * 于是首页选「A1 / 标准版」也会显示成 `A1M.toml`，P1S 的版本干脆没有。
 * 现在 A1 就是 A1（`A1.toml`）：三级选择与这份文件来自同一套文件体系，自然对得上。
 *
 * 契约的 `api.getPreset` 留着不动（别处还在用），这里不再调它。
 */

import { useEffect, useState } from 'react'
import { api } from '../../api'
import { isAppError } from '../../api/contract'
import type { AppError, Preset } from '../../api/contract'
import type { Selection } from '../home/MachinePicker'

export type { Preset }

/**
 * 预设的加载态。
 *
 * `idle` 是"还没选齐二级 / 后端没有这一份"，不是"空闲等着下载" —— 没选完就没有"哪一份预设"可谈。
 * 中间态只有 `waiting` 会真的停留：假后端是同进程的同步数据，真后端连接慢时这一步会亮骨架。
 * `downloading` 这一档**取不到** —— 它原本只由试验场的调试面板扳出来。形状留着不动：
 * 读它的那几个组件是照搬过来的，不为了删一个分支去改它们。
 *
 * `failed` 多带一个 `error`：界面光说一句「连接失败」对排查毫无帮助，日志里那一条才有
 * 上下文，两者之间唯一的桥是 `traceId` —— 所以错误对象要一路带到 `PresetStack` 上
 * （见 `components/TraceTag`）。
 */
export type PresetState =
  | { status: 'idle' }
  | { status: 'waiting'; name: string }
  | { status: 'downloading'; name: string }
  | { status: 'failed'; name: string; error?: AppError }
  | { status: 'ready'; preset: Preset }

/**
 * 文件名在拿到响应之前是不知道的。
 *
 * 等待期间宁可显示一个中性占位，也不猜一个名字出来（PresetStack 在 waiting 态本来就渲染骨架屏，
 * 这个串只在「强制 downloading / failed 且还没拿到响应」这一种情况下露脸）。
 */
const PENDING_NAME = '预设文件'

/** 三轴与速度那四个字段 —— 参数注册表里的真名字，别在别处再写一遍 */
const OFFSET_KEYS = {
  x: 'toolhead.offset.x',
  y: 'toolhead.offset.y',
  z: 'toolhead.offset.z',
} as const
const SPEED_KEY = 'toolhead.speed_limit'

export function usePreset(sel: Selection): PresetState {
  const [state, setState] = useState<PresetState>({ status: 'idle' })

  const machine = sel.model
  const version = sel.variant
  const complete = Boolean(sel.brand && machine && version)

  useEffect(() => {
    if (!complete || machine === null || version === null) {
      setState({ status: 'idle' })
      return
    }

    /* 选择在请求回来之前又改了：旧那一份的结果直接丢掉，不许它盖掉新的 */
    let alive = true
    setState({ status: 'waiting', name: PENDING_NAME })

    Promise.all([
      api.getVersionFiles(machine, version),
      api.getMachineParams(machine, version),
    ]).then(
      ([files, params]) => {
        if (!alive) return

        /* 这个版本没有配 MKP 预设文件（如 A2L）：当没选，别拿半份数据糊弄 */
        const file = files?.files.find((f) => f.kind === 'mkp_preset')
        if (file === undefined) {
          setState({ status: 'idle' })
          return
        }

        const valueOf = (key: string): number | null => {
          const row = params.find((p) => p.key === key)
          if (row === undefined) return null
          const n = Number(row.value)
          return Number.isFinite(n) ? n : null
        }
        const x = valueOf(OFFSET_KEYS.x)
        const y = valueOf(OFFSET_KEYS.y)
        const z = valueOf(OFFSET_KEYS.z)
        const speed = valueOf(SPEED_KEY)

        /* 四个数缺一个就当取不到：读数条宁可说「取不到」，也不拿半个基准开始校准 */
        if (x === null || y === null || z === null || speed === null) {
          setState({ status: 'failed', name: PENDING_NAME })
          return
        }

        setState({
          status: 'ready',
          preset: { name: file.fileName, path: file.path, axes: { x, y, z }, speed },
        })
      },
      (err: unknown) => {
        if (!alive) return
        console.error('[preset] 取预设失败', err)
        /* 带着 AppError 一起进状态：界面要能显示 traceId，否则日志里那条查不到人 */
        setState({
          status: 'failed',
          name: PENDING_NAME,
          error: isAppError(err) ? err : undefined,
        })
      },
    )

    return () => {
      alive = false
    }
  }, [complete, machine, version])

  /* 直接把 state 交出去。
     身份必须稳定：useCalibration 的「预设换了就作废草稿」那条 effect 依赖的是这个对象本身，
     每次渲染新造一个会让它反复清空草稿。useState 持有的对象只在 setState 时才换，所以够稳。 */
  return state
}
