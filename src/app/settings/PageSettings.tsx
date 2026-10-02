/*
 * 设置页 —— 最小版本：**只有「高级设置 → 预设数据源」这一节**（2026-10-02）。
 *
 * 为什么数据源住这里（作者裁决，「同步」页退役那一刀）：
 *  - 普通用户**完全不需要**知道"数据源"这个概念 —— 官方地址由工作台配置、
 *    构建时注入客户端（Bootstrap 那一刀）；catalog 随安装包走、更新是内部机制。
 *  - 原来那一整页（数据源 / catalog 的调试账 / 下载第一份 / 检查更新）是开发验证面板，
 *    不是产品页面 —— 整页退役（`src/app/pages/` 随它清空）。
 *  - 这里留下的是**开发 / 排查的后门**：临时把下载指向本地 `python3 -m http.server`
 *    或别的源。默认状态就是"没被碰过"（内置默认 / 没配），普通用户什么都不用动。
 *
 * 交互：两个单选只管**选哪条路**，写动作全部走按钮（不做"一选就写"的隐式动作）——
 * 「手动指定」填地址再按「应用」；「使用内置官方源」按「恢复内置默认」撤掉覆盖
 * （只有存在覆盖时才出现这颗按钮 —— 没有可撤的东西就不摆一颗点了没事干的按钮）。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { PresetSource } from '../../api/contract'
import s from './PageSettings.module.css'

type Mode = 'builtin' | 'manual'

export default function PageSettings() {
  const [source, setSource] = useState<PresetSource | null>(null)
  const [loadErr, setLoadErr] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('builtin')
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: string; bad: boolean } | null>(null)

  const read = useCallback(async () => {
    setLoadErr(null)
    try {
      const got = await api.getPresetSource()
      setSource(got)
      /* 初值跟着生效值走：有覆盖 = 手动那一档；内置 / 没配 = 内置那一档 */
      setMode(got?.fromUser === true ? 'manual' : 'builtin')
      setDraft(got?.baseUrl ?? '')
    } catch (e) {
      setSource(null)
      setLoadErr(errorText(e))
    }
  }, [])

  useEffect(() => {
    void read()
  }, [read])

  const pick = (next: Mode) => {
    setMode(next)
    setNote(null)
  }

  /* 当前生效的那一句账：读不出来 / 没配 / 你指定 / 内置默认 —— 四种都要说得出 */
  const currentText =
    loadErr !== null
      ? `读不出来：${loadErr}`
      : source === null
        ? '还没配置 —— 下载与检查更新会先如实拒绝，并说明去哪儿配'
        : source.fromUser
          ? `你指定：${source.baseUrl}`
          : `内置默认：${source.baseUrl}`

  /* 「使用内置官方源」那格的副文案：内置是什么，没有就如实说没有 */
  const builtinText =
    source?.builtin != null
      ? `内置地址：${source.builtin}`
      : '这个构建没有内置官方源（选它 = 回到「没配」）'

  /* 「恢复内置默认」可不可点：只有"存在用户覆盖"时才有的撤 */
  const canClear = source !== null && source.fromUser
  const canSet = draft.trim() !== ''

  const apply = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      if (mode === 'builtin') {
        const got = await api.clearPresetSource()
        setSource(got)
        setDraft(got?.baseUrl ?? '')
        setNote({
          text:
            got === null
              ? '已撤掉你填的地址 —— 这个构建没有内置官方源，现在就是「没配」'
              : `已回到内置默认：${got.baseUrl}`,
          bad: false,
        })
      } else {
        const got = await api.setPresetSource(draft.trim())
        setSource(got)
        setDraft(got.baseUrl)
        setNote({ text: `已应用：${got.baseUrl}`, bad: false })
      }
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={s.page} data-density="roomy">
      <header className={s.head}>
        <h1 className={s.title}>设置</h1>
        <p className={s.sub}>
          应用设置、诊断与版本。本版只有「高级设置」这一节 —— 普通使用不用来这里。
        </p>
      </header>

      <section className={s.section} aria-label="高级设置">
        <h2 className={s.secTitle}>高级设置</h2>

        <div className={s.field}>
          <div className={s.fieldTitle}>预设数据源</div>
          <p className={s.fieldNote}>
            默认使用内置官方源。仅用于开发测试或排查问题时，临时指定其他数据源。
          </p>

          <label className={s.radio}>
            <input
              type="radio"
              name="preset-source"
              checked={mode === 'builtin'}
              onChange={() => pick('builtin')}
              disabled={busy}
            />
            <span>使用内置官方源</span>
          </label>
          <p className={s.hint}>{builtinText}</p>
          {mode === 'builtin' && canClear && (
            <div className={s.row}>
              <button type="button" className={s.btn} onClick={() => void apply()} disabled={busy}>
                {busy ? '正在恢复……' : '恢复内置默认'}
              </button>
            </div>
          )}

          <label className={s.radio}>
            <input
              type="radio"
              name="preset-source"
              checked={mode === 'manual'}
              onChange={() => pick('manual')}
              disabled={busy}
            />
            <span>手动指定（开发 / 排查）</span>
          </label>
          {mode === 'manual' && (
            <div className={s.row}>
              <input
                className={s.input}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder="https://…"
                aria-label="数据源地址"
              />
              <button
                type="button"
                className={s.btn}
                onClick={() => void apply()}
                disabled={busy || !canSet}
              >
                {busy ? '正在应用……' : '应用'}
              </button>
            </div>
          )}
        </div>

        <p className={s.current}>当前：{currentText}</p>
        {note !== null && (
          <p className={`${s.note} ${note.bad ? s.noteBad : ''}`}>{note.text}</p>
        )}
      </section>
    </div>
  )
}
