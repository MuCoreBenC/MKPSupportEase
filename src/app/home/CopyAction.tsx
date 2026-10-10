/*
 * 首页那颗「复制后处理脚本」。
 *
 * # 两个概念分开（2026-10-10 作者裁定）
 *
 * - **按钮常驻**：显不显示**不由**"命令现在给不给得出来"决定 —— 那是界面规则。
 * - **命令能不能给**：后端的两条真实事实说了算（本程序自己 + 预设真在盘上）；
 *   给不出来时把 `reason` 显示在按钮下面（"先下载并应用"那条路），**不抄假命令**。
 *
 * ★ 点的时候**向后端现要一次命令**（`load`）：下载 / 应用 / 换份都发生在别处，
 *   任何缓存的路径都可能过期 —— 前端不拼路径、也不长期缓存命令，只负责展示与复制。
 * ★ 命令按**当前平台 / 当前实例 / 真实落点**现拼：macOS 用 macOS 的可执行物与落点，
 *   Windows 用 Windows 的，同一份代码、各自的真值（没有写死的路径）。
 */

import { useEffect, useRef, useState } from 'react'
import { errorText } from '../../api'
import type { PostProcessCommand } from '../../api'
import s from './CopyAction.module.css'

interface CopyActionProps {
  label: string
  /** 现取现抄：点一下向后端要一次（后端按当前平台与真实落点现拼） */
  load: () => Promise<PostProcessCommand>
  /** 后端刚答的现况：不能抄时这里带着原因（按钮照摆，原因在下面那行） */
  state: PostProcessCommand | null
}

/**
 * 文案恒定的复制按钮：宽高永不跳 —— 尺寸只由原 label 撑出，
 * 「已复制」盖在原位直接显示（纯透明度，文字永不缩放/重采样，永远锐利）。
 *
 * 点击 → 向后端现要命令 → 拿到就进剪贴板 + 「✓ 已复制」1.5s；
 * 后端说还不能给（或读取失败）→ **不复制**，把原因显示在按钮下面。
 */
export default function CopyAction({ label, load, state }: CopyActionProps) {
  const [copied, setCopied] = useState(false)
  const [busy, setBusy] = useState(false)
  /* 点下去那一刻后端给的"还不给"——比 `state` 新；下一份现况回来就让它让位 */
  const [refused, setRefused] = useState<string | null>(null)
  const timer = useRef<number | null>(null)

  useEffect(
    () => () => {
      if (timer.current !== null) window.clearTimeout(timer.current)
    },
    [],
  )

  /* 现况换了一份：旧的"还不给"说完了就让位（该说的由新的 `state.reason` 说） */
  useEffect(() => {
    setRefused(null)
  }, [state])

  const copy = async () => {
    if (busy) return
    setBusy(true)
    try {
      const cmd = await load()
      if (!cmd.ready || cmd.command === undefined) {
        setCopied(false)
        setRefused(cmd.reason ?? '这条命令现在还抄不了')
        return
      }
      await navigator.clipboard.writeText(cmd.command)
      setRefused(null)
      setCopied(true)
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => setCopied(false), 1500)
    } catch (err: unknown) {
      /* 读取失败也得说人话（`errorText` 是错误唯一入口），不许静默"什么都没发生" */
      setCopied(false)
      setRefused(errorText(err))
    } finally {
      setBusy(false)
    }
  }

  /* 提示 = 点下去后端刚说的那条 > 现况里带着的原因；能抄时两边都没有 */
  const hint = refused ?? (state !== null && !state.ready ? (state.reason ?? null) : null)

  return (
    <div className={s.wrap}>
      <button type="button" className={s.btn} data-copied={copied} onClick={copy}>
        <span className={s.label}>{label}</span>
        <span className={s.done} aria-live="polite">
          <svg className={s.tick} viewBox="0 0 12 12" width="11" height="11" aria-hidden="true">
            <path d="M2.2 6.6 5 9.4 9.8 3.2" pathLength={100} />
          </svg>
          已复制
        </span>
      </button>
      {hint !== null && <p className={s.hint}>{hint}</p>}
    </div>
  )
}
