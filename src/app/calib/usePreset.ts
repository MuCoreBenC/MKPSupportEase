/*
 * 取预设：选齐「机型 + 版本」后，组装那一份。
 *
 *   文件名 / 落点   有「本地那份」→ 它（`presets-local/…`，校准保存写它）；
 *                   没有 → 云端那份（`api.getVersionFiles` 里 `kind === 'mkp_preset'` 的条目）
 *   三轴 / 速度     有「本地那份」→ 读它的（`api.getLocalCopyFor`，用户自己校准出来的值）；
 *                   没有 → `api.getMachineParams` 的云端默认
 *                   （`toolhead.offset.x / y / z` + `toolhead.speed_limit`）
 *
 * **校准写在「我的预设」上**（2026-10-08 资源库改判）：交付预设那一份是模板、只读 ——
 * 要校准它就得先在资源库里「另存为本地预设」，之后读与写都落在那一份 TOML 上。
 * 「没有本地这份」不是错误：校准页照旧显示云端默认值，只是**保存会被拦**
 * （先存一份自己的才有得存）。
 *
 * 为什么走文件体系（作者的原话：「首页也应该是消费那个文件，而不是用硬编码」）：
 * 上一版走 `api.getPreset(variantId)`，那是一张手编表、三份全是 A1 mini 的 ——
 * 于是首页选「A1 / 标准版」也会显示成 `A1M.toml`，P1S 的版本干脆没有。
 * 现在 A1 就是 A1（`A1.toml`）：三级选择与这份文件来自同一套文件体系，自然对得上。
 *
 * 那个方法 **2026-10-02 清扫时已经从契约里删掉**（连同 Rust 的 `get_preset` 与假后端
 * 的夹具）：文件体系是唯一的路，这里也不会再出现第二份"按版本索引的表"。
 */

import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api'
import { isAppError } from '../../api/contract'
import type { AppError, Axes, Preset } from '../../api/contract'
import type { Selection } from '../home/MachinePicker'
import { useDeliveryRevision } from '../state/deliveryState'

export type { Preset }

/**
 * 预设的加载态。
 *
 * `idle` 是"还没选齐二级 / 后端没有这一份"，不是"空闲等着下载" —— 没选完就没有"哪一份预设"可谈。
 * 中间态只有 `waiting` 会真的停留：假后端是同进程的同步数据，真机是一次本地 IPC
 * （**不联网** —— 目录、参数、副本三样都在本机）。
 * `downloading` 这一档**取不到** —— 它原本只由试验场的调试面板扳出来。形状留着不动：
 * 读它的那几个组件是照搬过来的，不为了删一个分支去改它们。
 *
 * `failed` 多带一个 `error`：界面光说一句「读取失败」对排查毫无帮助，日志里那一条才有
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

/*
 * 跨页缓存（2026-10-05）：key = `机型/版本`。tab 切换会重挂，没有这份缓存时每次进
 * 校准页读数都要从「—」等一趟 IPC 才长出数字 —— 页脚在进场后当着用户的面再变一次。
 * 重挂先吃上一份 ready 结果渲染、照旧重拉（文件可能换过），回来后覆盖。
 * 只缓存 ready：waiting/failed 是过程态，缓存它们会把「失败」钉在页面上。
 */
const presetCache = new Map<string, Preset>()

export function usePreset(sel: Selection): PresetState {
  const machine = sel.model
  const version = sel.variant
  const complete = Boolean(sel.brand && machine && version)

  /* 本机文件面代次：别处下载 / 另存 / 删除之后，这一支要重读（见 effect 尾部注释） */
  const deliveryRevision = useDeliveryRevision()

  /*
   * ★★ 状态**跟着"哪一版"走**（2026-10-17 第六轮）。
   *
   * 原来是裸 `useState`：换一版时 effect 里才 `setState(waiting)`，于是**中间有一帧**
   * 是"旧的 ready + 新的选择" —— 选机型页那一格就拿着旧名字配新副本画出来，看起来
   * 是"先多出来、再变少、再改名"（作者连拍照到的就是它）。
   *
   * 现在状态带钥匙：渲染时发现钥匙对不上，**当帧**就换成新版的（缓存命中直接 ready），
   * 不会出现混合态。`useMemo` 保证对象身份稳定（`useCalibration` 那条 effect 依赖它）。
   */
  const key = complete && machine !== null && version !== null ? `${machine}/${version}` : null
  const [entry, setEntry] = useState<{ key: string; state: PresetState } | null>(null)
  const state = useMemo<PresetState>(() => {
    if (key === null) return { status: 'idle' }
    if (entry !== null && entry.key === key) return entry.state
    const hit = presetCache.get(key)
    return hit !== undefined ? { status: 'ready', preset: hit } : { status: 'waiting', name: PENDING_NAME }
  }, [entry, key])

  useEffect(() => {
    if (!complete || key === null || machine === null || version === null) return

    /* 选择在请求回来之前又改了：旧那一份的结果直接丢掉，不许它盖掉新的 */
    let alive = true
    /* 缓存里有上一份就站在它上面刷新，不清成 waiting —— 重挂时读数不许闪「—」 */
    if (!presetCache.has(key)) {
      setEntry({ key, state: { status: 'waiting', name: PENDING_NAME } })
    }

    Promise.all([
      api.getVersionFiles(machine, version),
      api.getMachineParams(machine, version),
      /* 「我那一份」在不在（在就用它的校准值 —— 2026-10-08：校准写在「我的预设」上） */
      api.getLocalCopyFor(machine, version),
    ]).then(
      ([files, params, copy]) => {
        if (!alive) return

        /* 这个版本没有配 MKP 预设文件（如 A2L）：当没选，别拿半份数据糊弄 */
        const file = files?.files.find((f) => f.kind === 'mkp_preset')
        if (file === undefined) {
          presetCache.delete(key)
          setEntry({ key, state: { status: 'idle' } })
          return
        }

        const valueOf = (key: string): number | null => {
          const row = params.find((p) => p.key === key)
          if (row === undefined) return null
          const n = Number(row.value)
          return Number.isFinite(n) ? n : null
        }
        /* 云端默认那一套：你那份里读不出来的那几格回落到它 */
        const x = valueOf(OFFSET_KEYS.x)
        const y = valueOf(OFFSET_KEYS.y)
        const z = valueOf(OFFSET_KEYS.z)
        const presetAxes: Axes | null =
          x !== null && y !== null && z !== null ? { x, y, z } : null
        const cloudSpeed = valueOf(SPEED_KEY)

        /* 本地那份优先：它的三轴是**用户自己校准出来的**（云端那套只是出厂默认） */
        const axes = copy?.axes ?? presetAxes
        const speed = copy?.speed ?? cloudSpeed

        /* 四个数缺一个就当取不到：读数条宁可说「取不到」，也不拿半个基准开始校准 */
        if (axes === null || speed === null) {
          setEntry({ key, state: { status: 'failed', name: PENDING_NAME } })
          return
        }

        const preset: Preset = {
          /* 「实际会用的那一份」：有副本就是副本（校准写它） */
          name: copy?.fileName ?? file.fileName,
          /* 套餐里那份 —— 选机型页的「预设文件」只认它（见 contract 里那段） */
          bundleName: file.fileName,
          bundlePath: file.path,
          path: copy?.path ?? file.path,
          axes,
          speed,
          /* 保存校准写它；null = 还没有本地这份（先另存为本地预设才有得存）
             ★ 本仓术语是 `preset / local`（来源只有这两条线，见 contract 的 ActiveOrigin） */
          local: copy === null ? null : { path: copy.path, fileName: copy.fileName },
        }
        presetCache.set(key, preset)
        setEntry({ key, state: { status: 'ready', preset } })
      },
      (err: unknown) => {
        if (!alive) return
        console.error('[preset] 取预设失败', err)
        /* 带着 AppError 一起进状态：界面要能显示 traceId，否则日志里那条查不到人 */
        setEntry({
          key,
          state: {
            status: 'failed',
            name: PENDING_NAME,
            error: isAppError(err) ? err : undefined,
          },
        })
      },
    )

    return () => {
      alive = false
    }
    /*
     * ★★ 2026-10-14 补代次依赖（原来是 `[complete, machine, version]`）：
     * 这一支读了 `getLocalCopyFor`（"我那份在不在"）—— 而那份文件由**别的页**写
     * （预设页下载 / 另存 / 改归属）。不跟着投递面代次重读的话，首页与校准页会拿
     * 一个陈旧的 `local`（比如你刚在预设页另存了一份，这里还以为"还没有本地这份"，
     * 校准就没地方写）。同一条口径见 `../state/deliveryState.ts` 文件头。
     */
  }, [complete, machine, version, key, deliveryRevision])

  /* 直接把 state 交出去。
     身份必须稳定：useCalibration 的「预设换了就作废草稿」那条 effect 依赖的是这个对象本身，
     每次渲染新造一个会让它反复清空草稿。`useMemo` + `setEntry` 只在"哪一版"或它那一份状态
     真的变了时才换引用（同键重读回来的新对象会换 —— 那是新事实，本来该换）。 */
  return state
}
