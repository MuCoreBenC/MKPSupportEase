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
 * 交互：**两个官方源摆成固定单选**（Gitee / GitHub，作者 2026-10-05 拍），
 * 自定义地址**收在最后那一档**里 —— 用户没有输错地址的机会，这是防错的第一道；
 * 第二道在后端：**保存前先探一次**，取不到就整次拒绝（错地址留在设置里比"没配"更难查）。
 *
 * 写动作全部走按钮（不做"一选就写"的隐式动作）：选自定义填地址再按「应用」；
 * 已经是默认那一档时，按钮不出现（没有可撤的东西就不摆一颗点了没事干的按钮）。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { PresetSource, SoftwareUpdate } from '../../api/contract'
import s from './PageSettings.module.css'

/**
 * 选的是**哪一个源**（`github` / `gitee` / `custom`）—— 与后端 `SourceMode` 一一对应，
 * **id 是契约**。作者 2026-05 拍：两个官方源 + 收起的自定义。
 */
type Mode = 'github' | 'gitee' | 'custom'

export default function PageSettings() {
  const [source, setSource] = useState<PresetSource | null>(null)
  const [loadErr, setLoadErr] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('github')
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
      /* 初值跟着生效值走：选了自定义 = 自定义那一档；其余 = 出厂默认那个内置源 */
      setMode((got?.mode ?? 'github') as Mode)
      setDraft(got?.mode === 'custom' ? (got?.address ?? '') : '')
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

  /** 在系统浏览器里打开 Release 页（**走命令**，不走 `<a>`） */
  const openReleasePage = async (url: string) => {
    try {
      await api.openUrl(url)
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    }
  }

  /** 开始在应用内下载（标题栏那枚环会接手；这一页只负责发起） */
  const startDownload = async () => {
    try {
      await api.startUpdate()
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    }
  }

  const pick = (next: Mode) => {
    setMode(next)
    setNote(null)
  }

  /* 当前生效的那一句账：读不出来 / 没配 / 你指定的 / 内置源 —— 四种都要说得出 */
  const currentText =
    loadErr !== null
      ? `读不出来：${loadErr}`
      : source === null
        ? '还没配置 —— 下载与检查更新会先如实拒绝，并说明去哪儿配'
        : source.mode === 'custom'
          ? `你指定：${source.address === '' ? '（还没填地址）' : source.address}`
          : `${source.label}：${source.address}`

  /* 「恢复默认」可不可点：只有"现在不是出厂默认那一档"时才有的撤 */
  const canClear = source !== null && source.mode !== source.defaultMode
  const canSet = draft.trim() !== ''

  const apply = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      if (mode === 'custom') {
        // 自定义：**先探后落盘**（后端做），取不到会整次拒绝并说明为什么
        const got = await api.setPresetSource('custom', draft.trim())
        setSource(got)
        setNote({ text: `已应用：${got.address}`, bad: false })
        return
      }
      const got = await api.setPresetSource(mode)
      setSource(got)
      setNote({ text: `已切到：${got.label}（${got.address}）`, bad: false })
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
            {/*
              ★ 「查看更新」用 `api.openUrl`，**不用 `<a target="_blank">`**：
              Tauri 的 webview 没开 opener 权限，`<a>` 点了**什么都不发生**
              （0.0.2 用户实测："点了没反应，没弹出浏览器"）。修法是走命令（第五刀）。
            */}
            {update?.hasUpdate === true && update.url !== undefined && (
              <button
                type="button"
                className={s.btn}
                onClick={() => void openReleasePage(update.url as string)}
              >
                查看更新
              </button>
            )}
            {/* 有安装包（release.json 给了 asset）才摆"在应用内下载"；没有就只留上面那颗 */}
            {update?.hasUpdate === true && update.asset !== undefined && (
              <button
                type="button"
                className={s.btn}
                disabled={updateBusy}
                onClick={() => void startDownload()}
              >
                在应用内下载
              </button>
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
            两个都是官方源，选一个就行（国内选 Gitee 更稳）。自定义地址只在开发 / 排查时用。
          </p>

          {/* ★ 两个固定单选 = 防错第一道：用户没有输错地址的机会 */}
          {(source?.builtin ?? []).map((b) => (
            <label key={b.id} className={s.radio}>
              <input
                type="radio"
                name="preset-source"
                checked={mode === b.id}
                onChange={() => pick(b.id as Mode)}
                disabled={busy}
              />
              <span>
                {b.label}
                {b.id === source?.defaultMode ? '（默认）' : ''}
              </span>
            </label>
          ))}
          {(source?.builtin ?? []).map((b) => (
            <p key={`${b.id}-hint`} className={s.hint}>
              {b.address}
            </p>
          ))}
          {mode !== 'custom' && canClear && (
            <div className={s.row}>
              <button
                type="button"
                className={s.btn}
                onClick={async () => {
                  /* 撤掉选择 = 回到出厂默认那一档（写动作走按钮，不做"一选就写"） */
                  if (busy || source === null) return
                  setBusy(true)
                  setNote(null)
                  try {
                    const got = await api.setPresetSource(source.defaultMode)
                    setSource(got)
                    setMode(got.mode as Mode)
                    setNote({ text: `已回到默认：${got.label}`, bad: false })
                  } catch (e) {
                    setNote({ text: errorText(e), bad: true })
                  } finally {
                    setBusy(false)
                  }
                }}
                disabled={busy}
              >
                {busy ? '正在恢复……' : '恢复默认'}
              </button>
            </div>
          )}

          {/* 收起的自定义：默认不展开（普通用户不需要看见输入框） */}
          <label className={s.radio}>
            <input
              type="radio"
              name="preset-source"
              checked={mode === 'custom'}
              onChange={() => pick('custom')}
              disabled={busy}
            />
            <span>自定义地址（开发 / 排查）</span>
          </label>
          {mode === 'custom' && (
            <div className={s.row}>
              <input
                className={s.input}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder="https://…/presets/dist/ 或 …/source.json"
                aria-label="数据源地址"
              />
              <button
                type="button"
                className={s.btn}
                onClick={() => void apply()}
                disabled={busy || !canSet}
              >
                {busy ? '正在检查……' : '应用'}
              </button>
            </div>
          )}
          {mode === 'custom' && (
            <p className={s.hint}>
              数据源根与 source.json 地址都认；应用前会先探一次，取不到就不改。
            </p>
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
