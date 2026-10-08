/*
 * 三轴偏移的状态机：已保存的一份 + 草稿。
 *
 * 保存走 `api.savePresetCalibration` —— 写进**「我的预设」**那一份的 TOML
 * （2026-10-08 资源库改判：偏移随「我的预设」走，`index/offsets.json` 那个孤岛退役）。
 * 没有你的一份（还没在资源库里「另存为我的预设」）时保存被拦（按钮灰着 + 这里第二道）。
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from '../../api'
import { LADDER } from './plateLadder'
import {
  matchXY,
  matchZ,
  Z_SPOT,
  type Axes,
  type Axis,
  type AxisView,
} from './calibAxes'
import type { PresetState } from './usePreset'

/**
 * 三轴偏移的那一套状态机：已保存的一份 + 板上点出来 / 手输出来的草稿。
 *
 * 从向导页里整块搬出来的。搬的理由是「校准」tab 要用同一套 ——
 * 两处各自 `useCalibration(preset)`，**状态互不相通**（各是一个实例）：
 * 向导里点了一格没保存，不该让另一个页面上的数也跟着变。
 */
export interface Calibration {
  /** 已保存的那一份。null = 还没有基准（没选齐 / 正在取 / 失败） */
  saved: Axes | null
  /** 叠上草稿之后的那一份。没有基准时是 null */
  draft: Axes | null
  /** 数值位怎么显示：有值 / 正在取 / 没有 */
  view: AxisView
  dirty: boolean
  /** 哪几轴脏了 —— 拦截弹窗要把它们列清楚 */
  dirtyAxes: Axis[]
  /** 刚保存过（2.2 秒后自己消） */
  savedNote: boolean
  /** 有基准才点得动板子 */
  canPick: boolean
  /**
   * 有「我那一份」才存得动（2026-10-08 资源库改判：校准值写进它，不落别处）。
   * `false` = 这个机型 / 版本还没有你的一份 —— 保存按钮灰着，
   * 先去资源库把这份**另存为我的预设**。
   */
  canSave: boolean
  /** Z 板上高亮的格子 */
  zSelected: string[]
  /** XY 板上高亮的格子 */
  xySelected: string[]
  pickZ: (id: string) => void
  pickXY: (id: string) => void
  typeAxis: (axis: Axis, raw: string) => void
  revertAxis: (axis: Axis) => void
  /** 点「原 x.xx」：这一轴整个退回已保存值 —— 手输的与板上点出来的一起清 */
  resetAxis: (axis: Axis) => void
  clearAll: () => void
  commitAll: () => void
}

export function useCalibration(preset: PresetState): Calibration {
  /* null = 还没有值。初值不能拿 mkpFull.offsets 顶 —— 那是上一台机器的数，
     一进来什么都没选就显示具体数字，看的人会以为已经替他读出来了。
     但**已就位**的预设（重挂时从跨页缓存里带来的那份 ready）就是当前真相，
     初值直接吃它 —— 不然首帧先画一次「—」、等 effect 来填，进场闪一下。 */
  const [saved, setSaved] = useState<Axes | null>(() =>
    preset.status === 'ready' ? preset.preset.axes : null,
  )
  const [zPick, setZPick] = useState<string | null>(null)
  const [xyPick, setXyPick] = useState<{ x: string | null; y: string | null }>({
    x: null,
    y: null,
  })
  /*
   * 手输的那一份草稿：绝对值，按轴记。
   *
   * 与"板上点出来的增量"是两条并行的来源，同一轴上**互斥、后来者赢**：手输就清掉该轴的
   * pick，点板子就清掉该轴的手输值。不这么定的话「点了格子又手输」会变成"delta 叠在手输值上"，
   * 屏幕上的数就说不清是从哪来的。
   */
  const [typed, setTyped] = useState<Partial<Record<Axis, number>>>({})
  const [savedNote, setSavedNote] = useState(false)

  /* 预设就位即新的真相：偏移换成它带来的那一份，草稿一并作废
     —— 草稿是"相对上一份预设点出来的增量"，换了预设就没有意义了。
     没就位（没选齐 / 正在取 / 失败）就把值清掉：宁可显示「—」，不拿旧数字冒充 */
  useEffect(() => {
    if (preset.status !== 'ready') {
      setSaved(null)
      setZPick(null)
      setXyPick({ x: null, y: null })
      setTyped({})
      return
    }
    setSaved(preset.preset.axes)
    setZPick(null)
    setXyPick({ x: null, y: null })
    setTyped({})
  }, [preset])

  /** 数值位怎么显示：有值 / 正在取 / 没有。整页只认这一个判断 */
  const view: AxisView =
    preset.status === 'ready' && saved
      ? 'value'
      : preset.status === 'waiting' || preset.status === 'downloading'
        ? 'loading'
        : 'blank'

  // 草稿有两条来源：板上点出来的增量、输入框里手输的绝对值（手输优先）
  const zDelta = zPick ? (Z_SPOT.get(zPick)?.value ?? 0) : 0
  const xDelta = xyPick.x ? (LADDER[xyPick.x]?.delta ?? 0) : 0
  const yDelta = xyPick.y ? (LADDER[xyPick.y]?.delta ?? 0) : 0
  const draft: Axes | null = useMemo(
    () =>
      saved
        ? {
            x: typed.x ?? saved.x + xDelta,
            y: typed.y ?? saved.y + yDelta,
            z: typed.z ?? saved.z + zDelta,
          }
        : null,
    [saved, typed.x, typed.y, typed.z, xDelta, yDelta, zDelta],
  )

  /*
   * 脏不脏不分页：三个轴在哪一页都能改，所以"这一页有没有草稿"这个问题没有意义。
   * 哪几轴脏了单独算一份，给拦截弹窗列清单用。
   */
  const dirtyAxes = useMemo<Axis[]>(() => {
    if (!saved || !draft) return []
    return (['x', 'y', 'z'] as Axis[]).filter((k) => draft[k] !== saved[k])
  }, [draft, saved])
  const dirty = dirtyAxes.length > 0

  /*
   * 板上的选中态：点出来的那一格优先，没有就拿"手输造成的改动量"去板上找对应刻度。
   * 于是打字改数之后板上那一格也会亮 —— 落在两格之间时一格都不亮（matchZ / matchXY 返回 null）。
   */
  const zSelected = useMemo(() => {
    if (zPick) return [zPick]
    if (!saved || !draft) return []
    const id = matchZ(Number((draft.z - saved.z).toFixed(2)))
    return id ? [id] : []
  }, [draft, saved, zPick])

  const xySelected = useMemo(() => {
    const out: string[] = []
    for (const axis of ['x', 'y'] as const) {
      const pickedId = xyPick[axis]
      if (pickedId) {
        out.push(pickedId)
        continue
      }
      if (!saved || !draft) continue
      const id = matchXY(axis, Number((draft[axis] - saved[axis]).toFixed(2)))
      if (id) out.push(id)
    }
    return out
  }, [draft, saved, xyPick])

  /** 清掉某几轴的手输值 */
  const dropTyped = useCallback((...axes: Axis[]) => {
    setTyped((t) => {
      if (axes.every((k) => t[k] === undefined)) return t
      const next = { ...t }
      axes.forEach((k) => delete next[k])
      return next
    })
  }, [])

  /** 放弃改动：板上点出来的与手输的一起清 —— 三轴随处可改，就不该只清本页那几轴 */
  const clearAll = useCallback(() => {
    setZPick(null)
    setXyPick({ x: null, y: null })
    setTyped({})
  }, [])

  /* 没有基准值就没有"加多少"可言：点板子不产生草稿，免得出现「— → +0.20」这种半截读数 */
  const canPick = saved !== null

  /* 有「我那一份」才存得动（见接口注释）—— 官方那份是模板，校准不落它 */
  const canSave = preset.status === 'ready' && preset.preset.mine !== null

  const pickZ = useCallback(
    (id: string) => {
      if (!canPick) return
      setZPick((prev) => (prev === id ? null : id))
      dropTyped('z')
    },
    [canPick, dropTyped],
  )

  const pickXY = useCallback(
    (id: string) => {
      if (!canPick) return
      const it = LADDER[id]
      if (!it) return
      setXyPick((prev) => ({ ...prev, [it.axis]: prev[it.axis] === id ? null : id }))
      dropTyped(it.axis as Axis)
    },
    [canPick, dropTyped],
  )

  /**
   * 手输一轴。解析不出数（空串、只有一个负号、只有小数点）就什么都不做 ——
   * 输入框里的原文由读数条自己留着，不弹提示、不回跳。
   */
  const typeAxis = useCallback(
    (k: Axis, raw: string) => {
      if (!canPick) return
      const v = Number(raw)
      if (raw.trim() === '' || !Number.isFinite(v)) return
      setTyped((t) => ({ ...t, [k]: Number(v.toFixed(2)) }))
      if (k === 'z') setZPick(null)
      else setXyPick((prev) => ({ ...prev, [k]: null }))
    },
    [canPick],
  )

  /** Esc：把这一轴退回"板上点出来 / 已保存"的值 */
  const revertAxis = useCallback((k: Axis) => dropTyped(k), [dropTyped])

  /* 点「原 x.xx」（作者 10-03：点了就变回原本的）：还原比 Esc 更彻底 ——
     手输值与板上点出来的选中一起清，草稿回到 saved。Esc 只退手输、留着点选，
     是"我在打字，退回点的那格"的语义；点 chip 是"这一轴我不要改了"。 */
  const resetAxis = useCallback(
    (k: Axis) => {
      dropTyped(k)
      if (k === 'z') setZPick(null)
      else setXyPick((prev) => ({ ...prev, [k]: null }))
    },
    [dropTyped],
  )

  /**
   * 保存：把**全部草稿轴**写进已保存那一份，不分页，同时写回**「我的预设」**那一份。
   *
   * 没有你的一份（还没在资源库里另存过）时不写：保存按钮本来就灰着，这里是第二道
   * —— 官方那一份不可变，校准值不落别处。
   * 落盘的结果不等、不 catch：没接后端时 api 会抛 NotImplementedError 并在控制台点名，
   * 要的就是"哪个口子没接"看得见（见 src/api/index.ts）。
   */
  const commitAll = useCallback(() => {
    if (!draft) return
    const copy = preset.status === 'ready' ? preset.preset.mine : null
    if (copy === null) return
    setSaved(draft)
    setZPick(null)
    setXyPick({ x: null, y: null })
    setTyped({})
    setSavedNote(true)
    void api.savePresetCalibration(copy.path, draft)
  }, [draft, preset])

  useEffect(() => {
    if (!savedNote) return
    const t = window.setTimeout(() => setSavedNote(false), 2200)
    return () => window.clearTimeout(t)
  }, [savedNote])

  return {
    saved,
    draft,
    view,
    dirty,
    dirtyAxes,
    savedNote,
    canPick,
    canSave,
    zSelected,
    xySelected,
    pickZ,
    pickXY,
    typeAxis,
    revertAxis,
    resetAxis,
    clearAll,
    commitAll,
  }
}
