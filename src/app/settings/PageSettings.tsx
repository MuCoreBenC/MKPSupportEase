/*
 * 设置页 —— 两块：**「软件更新」+「高级设置」**（2026-10-04 第三刀下半）。
 *
 * # 软件更新（第一块，普通用户唯一相关的）
 *
 * ★ **它是"有没有新版本 SupportEase"的唯一用户入口**（作者定死）。这一块与预设页的
 * 「读不懂」提示是**两条链**：这里答"软件有没有新版"，那里答"这批预设数据我读不读得懂"。
 * 两个事实**不许混成一句话**（禁区）。
 *
 * 信息源 = 仓库根的 `release.json`（`presets/` 之外，不是预设数据）：这是下半新开的一口，
 * 与 catalog 无关。检查**只在打开这一页时**发生（铁律 2：云端不参与首屏）。
 *
 * # 高级设置（第二块）
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
import type { PresetSource, SoftwareUpdate } from '../../api/contract'
import s from './PageSettings.module.css'

type Mode = 'builtin' | 'manual'

export default function PageSettings() {
  const [source, setSource] = useState<PresetSource | null>(null)
  const [loadErr, setLoadErr] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('builtin')
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: string; bad: boolean } | null>(null)
  /*
   * 软件更新这一块的状态。三态各有各的话：
   *   `loading`  还在问
   *   `null`     问不到（没配源 / 离线 / 远端没部署）—— 如实说"这次没查到"，不编"已是最新"
   *   `SoftwareUpdate`  查到了，按 `hasUpdate` 分两句
   */
  const [update, setUpdate] = useState<SoftwareUpdate | null>(null)
  const [updateBusy, setUpdateBusy] = useState(true)
  /* 问更新读到的那份「当前版本」：即使远端问不到也要能显示它（它来自构建期，一定拿得到） */
  const [appVersion, setAppVersion] = useState('')

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

  /*
   * 软件更新：**只在打开这一页时问一次**（铁律 2：更新检查不在启动 / 首屏路径）。
   * 两件事各问各的：版本号（本地、一定拿得到）与远端发布信息（可能问不到）。
   * 远端问不到 = `update` 留在 `null`，界面说"这次没查到" —— **不冒充"已是最新"**。
   */
  const checkUpdate = useCallback(async () => {
    setUpdateBusy(true)
    try {
      const [version, got] = await Promise.all([
        api.getAppVersion().catch(() => ''),
        api.checkSoftwareUpdate(),
      ])
      setAppVersion(version || got.currentVersion)
      setUpdate(got)
    } catch {
      /* 没配源 / 离线 / 远端还没发 release.json：都不该让设置页出问题 —— 如实留"没查到" */
      setUpdate(null)
      const v = await api.getAppVersion().catch(() => '')
      setAppVersion(v)
    } finally {
      setUpdateBusy(false)
    }
  }, [])

  useEffect(() => {
    void checkUpdate()
  }, [checkUpdate])

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
        <p className={s.sub}>软件更新与应用设置。</p>
      </header>

      <section className={s.section} aria-label="软件更新">
        <h2 className={s.secTitle}>软件更新</h2>

        <div className={s.updateRow}>
          <div className={s.updateHeadline}>
            {updateBusy ? (
              <span className={s.updateIdle}>正在检查……</span>
            ) : update?.hasUpdate === true ? (
              <span className={s.updateFresh}>有新版本 SupportEase</span>
            ) : update !== null ? (
              /* ★ 「已是最新」是**好消息**（作者 2026-10-05：字体要绿的）——
                 与「有新版本」同一个绿；「没查到」才是灰的（不是坏消息，是不知道）。 */
              <span className={s.updateLatest}>已是最新版本</span>
            ) : (
              <span className={s.updateIdle}>这次没能查到更新</span>
            )}
          </div>

          {/* 版本对照：当前 → 最新。当前版本一定拿得到（来自构建期） */}
          <p className={s.current}>
            当前版本 {appVersion || '—'}
            {update?.hasUpdate === true ? ` → 最新版本 ${update.latestVersion}` : ''}
          </p>

          {update?.hasUpdate === true && update.notes !== undefined && (
            <p className={s.fieldNote}>{update.notes}</p>
          )}

          <div className={s.row}>
            {update?.hasUpdate === true && update.url !== undefined && (
              <a className={s.btn} href={update.url} target="_blank" rel="noreferrer">
                查看更新
              </a>
            )}
            <button
              type="button"
              className={s.btn}
              onClick={() => void checkUpdate()}
              disabled={updateBusy}
            >
              {updateBusy ? '正在检查……' : '重新检查'}
            </button>
          </div>

          {update === null && !updateBusy && (
            <p className={s.fieldNote}>
              更新检查需要能连上发布地址（见下方「高级设置」）。连不上时如实说没查到，不冒充“已是最新”。
            </p>
          )}
        </div>
      </section>

      <section className={s.section} aria-label="高级设置">
        <h2 className={s.secTitle}>高级设置</h2>
        <p className={s.fieldNote}>以下仅供开发排查，普通使用不需要改动。</p>

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
