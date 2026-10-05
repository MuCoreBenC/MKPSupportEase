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
 * 交互：**两个官方源摆成固定单选**（Gitee / GitHub），选上**立即生效**并回写
 * （作者 2026-10-05 推翻旧的"选了不写、另摆按钮"那套 —— 「点了怎么没有立即帮我
 * 切换成 Gitee」：单选本身就是明确的意图，再要一颗确认键反而让人怀疑没点上）。
 * 切换失败回到**生效的那一档**，原因先讲人话。自定义地址收在最后那一档，仍是两步：
 * 选上只展开输入框，按「应用」才写 —— 防错第一道是"用户没有输错地址的机会"，
 * 第二道在后端：**保存前先探一次**，取不到就整次拒绝（错地址留在设置里比"没配"
 * 更难查）；拒绝时同样先给一句看得懂的结论，技术细节收进折叠（普通用户看结论，
 * 开发才有得排查）。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { PresetSource, SoftwareUpdate } from '../../api/contract'
import { Btn } from '../ui/Controls'
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
  const [note, setNote] = useState<{ text: string; bad?: boolean; detail?: string } | null>(null)
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

  /*
   * 选官方源 = **立即生效**（作者 2026-10-05：选了没反应让人怀疑没点上）。
   * 乐观把单选切过去，后端落盘成功就用回传值校准；失败**回到生效的那一档**，
   * 结论讲成人话、技术细节进折叠。自定义仍只展开输入框，不在这里写。
   */
  const pick = async (next: Mode) => {
    setNote(null)
    if (next === 'custom') {
      setMode('custom')
      return
    }
    if (busy) return
    setBusy(true)
    setMode(next)
    try {
      const got = await api.setPresetSource(next)
      if (got === null) {
        /* 演示后端没有内置源（真机必有）：选内置 = 回到"没配"，如实照做 */
        setSource(null)
        setMode(next)
        return
      }
      setSource(got)
      setMode(got.mode as Mode)
      setNote({ text: `已切到：${got.label}` })
    } catch (e) {
      setMode((source?.mode as Mode | undefined) ?? 'github')
      setNote({
        text: `没换成，现在用的还是 ${source?.label ?? '默认源'}。`,
        bad: true,
        detail: errorText(e),
      })
    } finally {
      setBusy(false)
    }
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

  const canSet = draft.trim() !== ''

  /* 自定义地址的落盘（先探后写，后端做）。失败不回滚单选：输入框留着让人改地址重试 */
  const apply = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const got = await api.setPresetSource('custom', draft.trim())
      setSource(got)
      setNote({ text: '已应用：现在用的就是你指定的这个地址。' })
    } catch (e) {
      setNote({
        text: `这个地址取不到预设数据，没有改，现在用的还是 ${source?.label ?? '默认源'}。请检查地址（或直接选上面的官方源）。`,
        bad: true,
        detail: errorText(e),
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={s.page}>
      <header className={s.head}>
        <h1 className={s.title}>设置</h1>
        <p className={s.sub}>软件更新与应用设置。</p>
      </header>

      {/*
       * 更新卡：**状态在左、动作在右**，顶对齐一行。不再摆「软件更新」小标题
       * （作者 2026-10-05：绿色的状态大字本身就是这一块的标题，再写一遍没有信息量）。
       * 窄窗放不下时按钮组折到下一行（margin-left:auto 让它单独占行时仍靠右）。
       */}
      <section className={s.section} aria-label="软件更新">
        <div className={s.updateTop}>
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

            {update === null && !updateBusy && (
              <p className={s.fieldNote}>
                更新检查需要能连上发布地址（见下方「高级设置」）。连不上时如实说没查到，不冒充“已是最新”。
              </p>
            )}
          </div>

          <div className={s.secActions}>
            {/*
              ★ 「查看更新」用 `api.openUrl`，**不用 `<a target="_blank">`**：
              Tauri 的 webview 没开 opener 权限，`<a>` 点了**什么都不发生**
              （0.0.2 用户实测："点了没反应，没弹出浏览器"）。修法是走命令（第五刀）。
            */}
            {update?.hasUpdate === true && update.url !== undefined && (
              <Btn onClick={() => void openReleasePage(update.url as string)}>
                查看更新
              </Btn>
            )}
            {/* 有安装包（release.json 给了 asset）才摆"在应用内下载"；没有就只留上面那颗 */}
            {update?.hasUpdate === true && update.asset !== undefined && (
              <Btn variant="accent" disabled={updateBusy} onClick={() => void startDownload()}>
                在应用内下载
              </Btn>
            )}
            <Btn variant="primary" disabled={updateBusy} onClick={() => void checkUpdate()}>
              {updateBusy ? '正在检查……' : '重新检查'}
            </Btn>
          </div>
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

          {/* ★ 两个固定单选 = 防错第一道：用户没有输错地址的机会。选上**立即生效**。
              地址提示跟在**各自**那一档下面（原来两条 URL 挤在一处，看不出归属） */}
          {(source?.builtin ?? []).map((b) => (
            <div key={b.id} className={s.sourceOpt}>
              <label className={s.radio}>
                <input
                  type="radio"
                  name="preset-source"
                  checked={mode === b.id}
                  onChange={() => void pick(b.id as Mode)}
                  disabled={busy}
                />
                <span>
                  {b.label}
                  {b.id === source?.defaultMode ? '（默认）' : ''}
                </span>
              </label>
              <p className={s.hint}>{b.address}</p>
            </div>
          ))}

          {/* 收起的自定义：默认不展开（普通用户不需要看见输入框）；选上只展开，按「应用」才写 */}
          <label className={s.radio}>
            <input
              type="radio"
              name="preset-source"
              checked={mode === 'custom'}
              onChange={() => void pick('custom')}
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
                placeholder="https://…/presets/delivery/ 或 …/source.json"
                aria-label="数据源地址"
              />
              <Btn onClick={() => void apply()} disabled={busy || !canSet}>
                {busy ? '正在检查……' : '应用'}
              </Btn>
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
          <div className={s.noteBlock}>
            <p className={`${s.note} ${note.bad ? s.noteBad : ''}`}>{note.text}</p>
            {note.detail !== undefined && (
              /* 技术细节收进折叠：普通用户只看结论（作者 2026-10-05：
                 「出了一堆东西我也看不懂，你要给用户看得懂的东西」） */
              <details className={s.noteDetail}>
                <summary>技术详情（开发排查用）</summary>
                <p>{note.detail}</p>
              </details>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
