import { useCallback, useEffect, useState } from 'react'

import { api } from '../../api'
import type { PostprocPrefsView, PostprocPrefsWire } from '../../api/contract'

/*
 * 设置页「后处理模式 / 校准与补偿 / 热床预涂胶」那三块的读写口（2026-10-17）。
 *
 * # 为什么要有这一层
 *
 * 那三块加起来十个控件，全住在 `run/app-state.json` 的**同一格**（`postproc`），
 * 而写命令是**整格覆盖**（`setPostprocPrefs` 收一整份）。如果每个控件各自
 * `await api.setPostprocPrefs(...)`，就会出两类事：
 *   · 传半份 = 没传到的那些项被清回出厂档（用户会看到"改一个开关，别的跟着跳回去"）；
 *   · 连点两下 = 两个请求乱序落地，最后盘上是先点的那一份。
 * 所以：**读写都走这里**，页面只调 `set({ 改的那一项 })`。
 *
 * # 三条约定
 *
 * 1. **模块级缓存 + 广播**：一份数据、多个读者（设置页那几个开关，以及 `App` 那侧的
 *    `silent` —— 后处理那一屏弹不弹认的就是它）。写成功必须让**所有**读者一起换过来，
 *    否则"设置页改成 GUI、外壳还按静默摆"（2026-10-27 作者真机踩到：切成 GUI 之后
 *    每一趟仍旧只缩在右上角，模态框再也不出来）。
 *    ★ 只读一次 IPC：缓存里没有才真读，且**同时在飞的那一次读**共用（App 与设置页
 *      前后脚挂载时不会问两遍）。
 * 2. **乐观更新 + 失败回滚**：开关点下去要**立刻**翻，不等人；写失败就翻回去，
 *    并把原因交给调用方（页面上摆一句 `.note`）。不许静默 —— 一个"点了没反应"的
 *    开关比没有这个开关更糟。
 * 3. **落盘后以盘上那份为准**：写成功后拿后端回的那一份替换缓存（不是"以为写成了"）。
 *    这一格只有设置页一个地方改，没有"另一个窗口同时改它"那种场景 ——
 *    所以**不订阅** `appStateMutated`（那是"多作者"才要的东西，这里订阅只会多一条无用的重读）；
 *    但**同一个进程里的读者**必须靠下面那个 `listeners` 同步，见约定 1。
 */

/** 缓存里那一份（`null` = 还没读过） */
let view: PostprocPrefsView | null = null
/** 读失败的原因（`null` = 没失败；"还没读完"不是失败） */
let loadError: string | null = null
/** 正在飞的那一次写（连点时排成队，不并发） */
let pending: Promise<void> = Promise.resolve()
/** 正在飞的那一次读（多读者共用，见头注约定 1） */
let reading: Promise<void> | null = null
/** 所有读者（`usePostprocPrefs` 的每一份实例） */
const listeners = new Set<() => void>()

function notify() {
  for (const l of listeners) l()
}

/** 换掉缓存那一份并叫醒所有读者 */
function publish(next: PostprocPrefsView) {
  view = next
  notify()
}

function fail(why: string | null) {
  loadError = why
  notify()
}

export type PostprocPrefsPatch = Partial<PostprocPrefsWire>

export interface PostprocPrefsApi {
  /** 解析后的值（`null` = 还没读回来，调用方画骨架/什么都不画） */
  prefs: PostprocPrefsView | null
  /** 读失败的原因（只有真失败才有；"还没读完"不是失败） */
  loadError: string | null
  /** 改一项（或几项）：乐观更新，失败自动回滚并把原因交给 `onError` */
  set: (patch: PostprocPrefsPatch, onError?: (why: string) => void) => Promise<void>
}

/** 把 `{ a: x }` 叠到盘上那一份上 —— **整格覆盖**要的是"全量"，这一步就是"全量"的来处。 */
function merge(raw: PostprocPrefsWire, patch: PostprocPrefsPatch): PostprocPrefsWire {
  return { ...raw, ...patch }
}

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

export function usePostprocPrefs(): PostprocPrefsApi {
  const [snap, setSnap] = useState<PostprocPrefsView | null>(view)
  const [err, setErr] = useState<string | null>(loadError)

  const read = useCallback(() => {
    if (reading !== null) return reading
    reading = api.getPostprocPrefs().then(
      (got) => {
        loadError = null
        publish(got)
      },
      (e: unknown) => {
        /* 读不到**不编一份出来**：页面按"还没读到"画，并把原因说出来 */
        fail(errorText(e))
      },
    )
    return reading.finally(() => {
      reading = null
    })
  }, [])

  useEffect(() => {
    const l = () => {
      setSnap(view)
      setErr(loadError)
    }
    listeners.add(l)
    /* 挂载时对一次账：上面两个 `useState` 的初值取的是**这次渲染开头**那份，
       期间别的读者可能已经换过（App 与设置页同一帧挂载就是这样）。 */
    l()
    /* 缓存里没有就真读一次；有就直接用（模块级缓存，见头注约定 1） */
    if (view === null) void read()
    return () => {
      listeners.delete(l)
    }
  }, [read])

  const set = useCallback(
    async (patch: PostprocPrefsPatch, onError?: (why: string) => void) => {
      const before = view
      if (before === null) return

      /* ① 先按"改成功后应该长什么样"画出来（乐观） */
      const wire = merge(before.raw, patch)
      const optimistic: PostprocPrefsView = {
        ...before,
        raw: wire,
        resolved: {
          notifyOnFinish: wire.notifyOnFinish ?? before.resolved.notifyOnFinish,
          uiMode: wire.uiMode ?? before.resolved.uiMode,
          glueBottomEnabled: wire.glueBottomEnabled ?? before.resolved.glueBottomEnabled,
          skipVibrationCalibration:
            wire.skipVibrationCalibration ?? before.resolved.skipVibrationCalibration,
          beginStageRetractEnabled:
            wire.beginStageRetractEnabled ?? before.resolved.beginStageRetractEnabled,
          beginStageRetractZLiftMm:
            wire.beginStageRetractZLiftMm ?? before.resolved.beginStageRetractZLiftMm,
          zCalibrationDwellSec: wire.zCalibrationDwellSec ?? before.resolved.zCalibrationDwellSec,
          xyCalibrationDwellSec:
            wire.xyCalibrationDwellSec ?? before.resolved.xyCalibrationDwellSec,
          bedGlueEnabled: wire.bedGlueEnabled ?? before.resolved.bedGlueEnabled,
          bedGlueAllModels: wire.bedGlueAllModels ?? before.resolved.bedGlueAllModels,
          autoTowerPosition: wire.autoTowerPosition ?? before.resolved.autoTowerPosition,
        },
      }
      publish(optimistic)

      /* ② 排队落盘（连点时按点的先后落地，不并发） */
      pending = pending.then(async () => {
        try {
          await api.setPostprocPrefs(merge(view?.raw ?? wire, patch))
        } catch (e: unknown) {
          /* ③ 失败：退回改之前那一份，并把原因交出去（不静默） */
          publish(before)
          onError?.(errorText(e))
        }
      })
      await pending
    },
    [],
  )

  return { prefs: snap, loadError: err, set }
}
