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
  /**
   * 「这条命令现在还抄不了」的原因 —— **交给页面去说**。
   *
   * ★ 2026-10-13（作者：「不是啊，首页的那一个也是一样的，它是下面显示。你干嘛要放在这？
   * 都被截断了」）：原来这颗气泡挂在**按钮下面**。可按钮住在卡片的 `.info` 列里，
   * 而卡片 `.frame`（overflow: hidden）与 `.info`（overflow: hidden auto）两层都裁 ——
   * 气泡往下走就压住读数表，还要被切掉一截。
   * 现在这里只把话说出去，由**页面底部那颗不占位的 `.scriptToast`** 统一渲染
   * （与页脚那颗「复制脚本」同一颗、同一落位，见 PageHome 的 `scriptNote`）。
   */
  onNote: (note: string) => void
}

/**
 * 文案恒定的复制按钮：宽高永不跳 —— 尺寸只由原 label 撑出，
 * 「已复制」盖在原位直接显示（纯透明度，文字永不缩放/重采样，永远锐利）。
 *
 * 点击 → 向后端现要命令 → 拿到就进剪贴板 + 「✓ 已复制」1.5s；
 * 后端说还不能给（或读取失败）→ **不复制**，把原因显示在按钮下面。
 */
export default function CopyAction({ label, load, onNote }: CopyActionProps) {
  const [copied, setCopied] = useState(false)
  const [busy, setBusy] = useState(false)
  const timer = useRef<number | null>(null)

  useEffect(
    () => () => {
      if (timer.current !== null) window.clearTimeout(timer.current)
    },
    [],
  )

  const copy = async () => {
    if (busy) return
    setBusy(true)
    try {
      const cmd = await load()
      if (!cmd.ready || cmd.command === undefined) {
        setCopied(false)
        /* 抄不了：原因交给页面底部那颗气泡说（这里不摆任何常驻文案、也不占位） */
        onNote(cmd.reason ?? '这条命令现在还抄不了')
        return
      }
      await navigator.clipboard.writeText(cmd.command)
      setCopied(true)
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => setCopied(false), 1500)
    } catch (err: unknown) {
      /* 读取失败也得说人话（`errorText` 是错误唯一入口），不许静默"什么都没发生" */
      setCopied(false)
      onNote(errorText(err))
    } finally {
      setBusy(false)
    }
  }

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
    </div>
  )
}
