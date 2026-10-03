/*
 * 预设抽屉：按机型分组列出「版本」，一行 = 一个 `机型 + 版本` 组合。
 *
 * 为什么是抽屉而不是两个下拉：作者的原话是「现在那个下拉菜单，我不喜欢。我喜欢改成抽屉的」。
 * 用户端早先那一版顶部就是「文件名 pill + 切换」，点了开一个文件抽屉 —— 这里回到那个形态。
 *
 * 两条实现上的硬约束：
 *   - `position: absolute` 挂在参数页的定位父级上，**不用 fixed**：
 *     外壳上的一层 transform 就是 fixed 的包含块（那两个点不开的浮层就栽在这上面）。
 *   - 切预设一律走调用方给的 `onPick` → `u.requestCombo`，它自带「有未保存改动先问一句」。
 *     抽屉自己**不判断脏数据**，免得两处各写一套确认。
 *
 * 每一项的小字显示**它自己的文件名**（`A1.toml` / `P1.toml` …）——
 * 说明书里每个版本的 files 清单都在（`fileOf` 现查），抽屉这才配叫「选择文件」。
 * 上一版的"诚实的取舍"（只有当前项显示得出文件名）来自一个错误前提：
 * 以为文件清单只有当前 combo 拿得到。
 */

import { useEffect, useRef } from 'react'
import s from './PresetPickerDrawer.module.css'

interface VersionOption {
  id: string
  name: string
  tag?: string | null
}

interface MachineOption {
  id: string
  display: string
  versions: VersionOption[]
}

interface Props {
  open: boolean
  machines: MachineOption[]
  machineId: string
  versionId: string
  /** 任一组合的 MKP 文件名（来自说明书里的版本 files）—— 每一项查自己的 */
  fileOf: (machineId: string, versionId: string) => string | null
  onPick: (machineId: string, versionId: string) => void
  onClose: () => void
  /**
   * 从哪一缘出。默认右（参数页）；首页传 'left' —— 右边常驻露出卡，抽屉没地方，
   * 挂左缘、缝换到右边（与试验台同款）。
   */
  side?: 'left' | 'right'
}

export default function PresetPickerDrawer({
  open,
  machines,
  machineId,
  versionId,
  fileOf,
  onPick,
  onClose,
  side = 'right',
}: Props) {
  const panelRef = useRef<HTMLDivElement>(null)

  /* Esc 关闭。焦点回到 pill 由调用方在 onClose 里做（它才持有 pill 的 ref） */
  useEffect(() => {
    if (!open) return
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  /* 打开时把焦点移进抽屉，键盘用户才能直接上下翻 */
  useEffect(() => {
    if (!open) return
    panelRef.current?.querySelector<HTMLButtonElement>('button[data-current="true"], button')?.focus()
  }, [open])

  if (!open) return null

  return (
    <>
      {/* 遮罩：点一下关掉。它在抽屉之下（z-index 见 css） */}
      <button type="button" className={s.scrim} aria-label="关闭预设列表" onClick={onClose} />

      <div className={s.drawer} data-side={side} ref={panelRef} role="dialog" aria-label="选择预设">
        <div className={s.head}>
          <span className={s.title}>选择预设</span>
          <button type="button" className={s.close} onClick={onClose}>关闭</button>
        </div>

        <div className={s.list}>
          {machines.map((m) => (
            <div key={m.id} className={s.group}>
              <div className={s.groupHead}>
                {m.display}
                <span className={s.groupId}>{m.id}</span>
              </div>

              {m.versions.map((v) => {
                const current = m.id === machineId && v.id === versionId
                /* 这个组合的 MKP 文件名。没配文件的版本没有它，退成 tag / id */
                const file = fileOf(m.id, v.id)
                return (
                  <button
                    key={`${m.id}/${v.id}`}
                    type="button"
                    className={s.item}
                    data-current={current}
                    title={file ?? `${m.display} · ${v.name}`}
                    onClick={() => {
                      /* 当前项再点一次只关抽屉 —— 不触发 requestCombo，免得白问一句「要切吗」 */
                      if (current) { onClose(); return }
                      onPick(m.id, v.id)
                    }}
                  >
                    <span className={s.tick}>{current ? '✓' : ''}</span>
                    <span className={s.name}>{v.name}</span>
                    <span className={s.note}>{file ?? v.tag ?? v.id}</span>
                  </button>
                )
              })}
            </div>
          ))}
        </div>
      </div>
    </>
  )
}
