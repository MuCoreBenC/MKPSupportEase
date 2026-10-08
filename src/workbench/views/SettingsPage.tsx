/**
 * 「设置」页 —— 数据根只读 + **一块可编辑的配置**（官方源）。
 *
 * 数据根（presets / workbench / dist）**不可配**：本仓的预设真相源固定是
 * `<repo>/presets`（没有第二候选、不 fallback），所以这一页把它们摆开，让人一眼看清
 * 工作台在读哪几个根、工作台子目录各自谁写谁读 —— 排查「读错了目录 / 数据长在哪」
 * 的那种问题用。上游根与回退规则那两块**已经整层删掉**（`HANDOFF.md`），照实不摆。
 *
 * 2026-10-02（第十七刀）起多了一块**可编辑**的：「官方源（Bootstrap）」——
 * 发布产物发到哪。它是**工作台唯一一处"发布到哪"**（入库的
 * `workbench/bootstrap.json`）；客户端构建时由 `build.rs` 读它注入默认源
 * （`npm run tauri dev` 也吃它）。
 *
 * 输入口径（作者 2026-10-02 定死的产品契约）：**只填仓库地址就够** ——
 * GitHub 仓库地址 / `.git` 克隆地址都会被后端补成 `main/presets/delivery/source.json`
 * 的 raw 直链；blob 页按人指的转；raw / 自建源原样。用户不必知道
 * `raw.githubusercontent.com` / `blob` / `presets/delivery` / `source.json` 里的任何一个。
 *
 * # 「重新读取」（2026-10-02）
 *
 * 这一格读的是 `workbench/bootstrap.json`（**随仓库入库的文件**），它可能被**界面之外**
 * 的东西改掉：你手工编辑、另一个工具写、别人提交后你 pull、CI 生成。那之后界面上的值
 * 就旧了 —— 而"界面上看到的是不是磁盘上那份"必须答得出来（否则证明不了任何事）。
 * 所以给一颗「重新读取」：它调 `wb.reload()`（**重开一次后端会话、从磁盘重读**），
 * 拿回来的 `bootstrapUrl` 重新填进输入框与下面那行「当前」。
 *
 * **它同时是"配置生效了没有"的判据**：点一下看到的就是**磁盘真值**。
 * （注意区分：这里读的是**配置**；Bootstrap 被客户端吃进二进制是**构建期**的事 ——
 * 改完要重新构建客户端，所以保存成功那句话会提"重启 dev / 重打正式包"。）
 *
 * # 「本地测试源（开发）」（2026-10-08）
 *
 * 上面那一格是**发布到哪**（构建期注入）。这一格是**运行时把源换成什么** ——
 * 一颗按钮替你敲 `npm run dev:test-update`：起本地假云端在 `127.0.0.1:8787`，
 * 再以 `MKPSE_PRESET_SOURCE_URL` 把客户端 dev 一起起起来，于是不用为了验"官方
 * 发了新版本"真去发一版。
 *
 * ★ 它**不碰客户端那一格「预设数据源」**：那是客户端自己的运行时设置
 * （住客户端的 `appDataDir/run/app-state.json`，两个应用的 appDataDir 都不一样），
 * 归客户端设置页管 —— 这里换的是**环境变量**那条路，只在 debug 构建里认。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { Boot, DevSourceStatus, PublishAccount } from '../api'
import s from '../c14.module.css'

export default function SettingsPage({ boot }: { boot: Boot }) {
  /* 官方源那两格：初值来自 boot；保存成功后本地回显（真值在 workbench/bootstrap.json，
     下次 wb_boot 会带回同一份）。GitHub 是主源；Gitee 是镜像（空 = 没配/清除） */
  const [bootstrap, setBootstrap] = useState(boot.bootstrapUrl ?? '')
  const [saved, setSaved] = useState<string | null>(boot.bootstrapUrl)
  const [giteeBootstrap, setGiteeBootstrap] = useState(boot.giteeBootstrapUrl ?? '')
  const [giteeSaved, setGiteeSaved] = useState<string | null>(boot.giteeBootstrapUrl)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: string; bad: boolean } | null>(null)

  /*
   * **发布账户**（第三刀下半）：GitHub / Gitee 对称，每平台三格 —— 仓库地址 / 用户名（配置）
   * + Token（秘密，住本机凭据文件 credentials.json，0600）。
   *
   * ★ 状态面**不含 Token 原值** —— 只有 `hasToken` 与尾号提示（`PlatformAccountView`）；
   * 要看 / 要复制明文，点那一格右侧的**眼睛**（显式调 `getPublishToken`，作者 2026-10-07 加）。
   * ★ 配置与秘密分离：仓库地址 / 用户名进 `publish-account.json`，Token 进凭据文件。
   */
  const [account, setAccount] = useState<PublishAccount | null>(null)
  const [acctBusy, setAcctBusy] = useState(false)
  /* 每平台的输入草稿：仓库地址 / 用户名 / Token（Token 存完即清空，不留界面） */
  const [draft, setDraft] = useState<
    Record<string, { repositoryUrl: string; username: string; token: string }>
  >({})
  /* 哪几个平台的 Token 正在编辑（已配置时默认显示掩码点，点进去才变输入框） */
  const [editingToken, setEditingToken] = useState<Record<string, boolean>>({})
  /* 哪几个平台**眼睛睁着**（platform → 明文；键在 = 显示中）。保存 / 清除 / 重读一律收回 */
  const [revealed, setRevealed] = useState<Record<string, string>>({})
  const [revealBusy, setRevealBusy] = useState<string | null>(null)
  const [acctNote, setAcctNote] = useState<{ text: string; bad: boolean } | null>(null)

  const loadAccount = useCallback(async () => {
    try {
      const next = await wb.publishAccount()
      setAccount(next)
      // 用磁盘真值初始化草稿（Token 从不回填）；重新读取 = 退出 Token 编辑态
      const d: Record<string, { repositoryUrl: string; username: string; token: string }> = {}
      for (const p of next.platforms) {
        d[p.platform] = { repositoryUrl: p.repositoryUrl, username: p.username, token: '' }
      }
      setDraft(d)
      setEditingToken({})
      // ★ 重读 = 收回明文：眼睛睁着的那几格一律回到掩码（明文不留在界面上）
      setRevealed({})
    } catch (e) {
      setAcctNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    }
  }, [])

  useEffect(() => {
    void loadAccount()
  }, [loadAccount])

  const setDraftField = (
    platform: string,
    field: 'repositoryUrl' | 'username' | 'token',
    v: string,
  ) => {
    setDraft((prev) => {
      const cur = prev[platform] ?? { repositoryUrl: '', username: '', token: '' }
      return { ...prev, [platform]: { ...cur, [field]: v } }
    })
  }

  /** 保存一个平台：先写目标（仓库地址 + 用户名），Token 非空再写凭据文件。 */
  const saveAccount = async (platform: string) => {
    const d = draft[platform]
    if (d === undefined) return
    if (d.repositoryUrl.trim() === '' || d.username.trim() === '') {
      setAcctNote({ text: '仓库地址与用户名都要填', bad: true })
      return
    }
    setAcctBusy(true)
    setAcctNote(null)
    try {
      await wb.setPublishAccount(platform, d.repositoryUrl.trim(), d.username.trim())
      const token = d.token.trim()
      if (token !== '') {
        await wb.setPublishToken(platform, token)
      }
      // ★ Token 存完**立刻清空输入框** —— 不留在界面状态里
      setDraft((prev) => ({
        ...prev,
        [platform]: { ...prev[platform], token: '' },
      }))
      setAcctNote({
        text: `已保存 ${platform} 发布账户${token !== '' ? '（Token 存进本机凭据文件）' : ''}`,
        bad: false,
      })
      await loadAccount()
    } catch (e) {
      setAcctNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setAcctBusy(false)
    }
  }

  const clearAccount = async (platform: string) => {
    setAcctBusy(true)
    setAcctNote(null)
    try {
      await wb.clearPublishAccount(platform)
      setAcctNote({ text: `已清除 ${platform} 发布账户（配置 + 凭据）`, bad: false })
      await loadAccount()
    } catch (e) {
      setAcctNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setAcctBusy(false)
    }
  }

  const PLATFORMS = ['github', 'gitee'] as const
  const PLATFORM_LABEL: Record<string, string> = { github: 'GitHub', gitee: 'Gitee' }

  /**
   * 眼睛：显示 / 收起一个平台的 Token 明文（作者 2026-10-07 —— 不要密码，点眼睛直接看、能复制）。
   *
   * ★ 明文只活在**这一刻的界面状态**里：眼睛收起、保存、清除、从磁盘重读都会收回。
   */
  const toggleReveal = async (platform: string) => {
    if (revealed[platform] !== undefined) {
      setRevealed((prev) => {
        const next = { ...prev }
        delete next[platform]
        return next
      })
      return
    }
    setRevealBusy(platform)
    setAcctNote(null)
    try {
      const token = await wb.getPublishToken(platform)
      if (token === null || token === '') {
        setAcctNote({
          text: `${PLATFORM_LABEL[platform]} 这会儿取不到 Token（可能已被清掉）`,
          bad: true,
        })
      } else {
        setRevealed((prev) => ({ ...prev, [platform]: token }))
      }
    } catch (e) {
      setAcctNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setRevealBusy(null)
    }
  }

  /** 把「眼睛睁着」那一格的明文送进剪贴板（复制失败就说清楚，不静默）。 */
  const copyToken = async (platform: string) => {
    const token = revealed[platform]
    if (token === undefined) return
    try {
      await navigator.clipboard.writeText(token)
      setAcctNote({ text: `已复制 ${PLATFORM_LABEL[platform]} Token`, bad: false })
    } catch {
      setAcctNote({ text: '剪贴板用不了 —— 请手动选中明文再复制', bad: true })
    }
  }

  /*
   * 「本地测试源（开发）」的状态与那两颗按钮。
   *
   * ★ **只在跑着的时候**每 2 秒问一次 —— 工作台的页面挂过就一直挂着
   * （`App.tsx` 的 `.pageSlot`：切走只是藏起来，不卸载），无条件轮询等于
   * 离开这一页之后还在空转 IPC。不跑的时候它自己退了只是少一次刷新。
   */
  const [devSrc, setDevSrc] = useState<DevSourceStatus | null>(null)
  const [devBusy, setDevBusy] = useState(false)
  const [devNote, setDevNote] = useState<{ text: string; bad: boolean } | null>(null)

  const readDevSrc = useCallback(async () => {
    try {
      setDevSrc(await wb.devSourceStatus())
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    }
  }, [])

  useEffect(() => {
    void readDevSrc()
  }, [readDevSrc])

  useEffect(() => {
    if (devSrc?.running !== true) return
    const timer = setInterval(() => {
      wb.devSourceStatus().then(setDevSrc).catch(() => undefined)
    }, 2000)
    return () => clearInterval(timer)
  }, [devSrc?.running])

  /** 一颗按钮翻面：没在跑就起，跑着就（连它起的客户端 dev 一起）停 */
  const toggleDevSrc = async () => {
    if (devBusy || devSrc === null) return
    const stopping = devSrc.running
    setDevBusy(true)
    setDevNote(null)
    try {
      const next = stopping ? await wb.devSourceStop() : await wb.devSourceStart()
      setDevSrc(next)
      setDevNote(
        next.running
          ? {
              text: '已启动 —— 客户端 dev 也一起起了，日志打在起工作台的那个终端里。',
              bad: false,
            }
          : { text: next.note ?? '已停，服务端口也让出来了。', bad: false },
      )
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setDevBusy(false)
    }
  }

  /* 那一行结论：有动作反馈用动作的，否则用后端报的"为什么不在跑" */
  const devLine =
    devNote?.text ??
    (devSrc !== null && !devSrc.running ? (devSrc.note ?? null) : null)
  const devBad = devNote?.bad === true

  const save = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      /* 两格一笔写全：gitee 空着就是清除（null），不是"不动它" —— */
      const stored = await wb.setBootstrap(
        bootstrap.trim(),
        giteeBootstrap.trim() === '' ? null : giteeBootstrap.trim(),
      )
      setSaved(stored.bootstrapUrl)
      setBootstrap(stored.bootstrapUrl)
      setGiteeSaved(stored.giteeBootstrapUrl ?? null)
      setGiteeBootstrap(stored.giteeBootstrapUrl ?? '')
      setNote({ text: '已保存（重启 dev / 重打正式包后客户端才吃得到）', bad: false })
    } catch (e) {
      setNote({
        text: isAppError(e) ? e.message : e instanceof Error ? e.message : String(e),
        bad: true,
      })
    } finally {
      setBusy(false)
    }
  }

  /**
   * 从**磁盘**重读（`wb.reload()` 重开一次后端会话）。
   * 拿回来的就是 `workbench/bootstrap.json` 的真值 —— 不管它是谁写的、什么时候改的。
   * 界面上那些还没保存的输入会被磁盘那份**覆盖**（那正是"看到真相"的含义）。
   */
  const reread = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const fresh = await wb.reload()
      setSaved(fresh.bootstrapUrl)
      setBootstrap(fresh.bootstrapUrl ?? '')
      setGiteeSaved(fresh.giteeBootstrapUrl)
      setGiteeBootstrap(fresh.giteeBootstrapUrl ?? '')
      setNote({
        text:
          fresh.bootstrapUrl === null
            ? '已从磁盘重读：还没配'
            : fresh.giteeBootstrapUrl === null
              ? '已从磁盘重读（Gitee 镜像没配）'
              : '已从磁盘重读',
        bad: false,
      })
    } catch (e) {
      setNote({
        text: isAppError(e) ? e.message : e instanceof Error ? e.message : String(e),
        bad: true,
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={s.flow} style={{ padding: 12 }}>
      {/* ——— 发布账户（第三刀下半）：GitHub / Gitee 对称，各三格 ——— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>发布账户</b>
          <span className={s.cardNote}>
            发布到哪个仓库、以谁的身份 —— SupportEase 自己管理凭据，不用本机的 gh / git 登录态
          </span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              发布时会自动提交、推送并建 PR/MR。**平台与仓库由这里配置**（不靠猜远端）；
              Token 存在**本机凭据文件**里（仅本人可读），**绝不写进配置文件**；
              要看 / 要复制明文，点那一格右侧的**眼睛**（作者 2026-10-07）。
            </p>
            <p className={s.vhelp}>
              当前仓库：
              {account === null ? (
                '读取中……'
              ) : account.remoteUrl === null ? (
                '读不到 git remote（这台机器上取不到 git / 不是仓库）'
              ) : (
                <>
                  <span className={s.mono}>{account.remoteUrl}</span>
                  {account.remoteMatchesConfig
                    ? ' —— 与已配置的发布账户一致 ✓'
                    : ' —— 与已配置的发布账户不一致，发布前请确认'}
                  {account.branch !== null && ` · 分支 ${account.branch}`}
                </>
              )}
            </p>
          </div>

          {PLATFORMS.map((p) => {
            const view = account?.platforms.find((v) => v.platform === p)
            const d = draft[p] ?? { repositoryUrl: '', username: '', token: '' }
            return (
              <div className={s.vfield} key={p}>
                <div className={s.vhead}>
                  <b>{PLATFORM_LABEL[p]}</b>
                  <span className={s.vkey}>
                    {view?.hasToken === true
                      ? `Token 已配置（${view.tokenHint ?? '已存'}）`
                      : 'Token 未配置'}
                  </span>
                </div>

                <div className={s.vrow}>
                  <label className={s.vlabel}>仓库地址</label>
                  <input
                    className={s.inp}
                    type="text"
                    value={d.repositoryUrl}
                    onChange={(e) => setDraftField(p, 'repositoryUrl', e.target.value)}
                    placeholder={`https://${p === 'github' ? 'github.com' : 'gitee.com'}/用户名/仓库.git`}
                    aria-label={`${PLATFORM_LABEL[p]} 仓库地址`}
                  />
                </div>

                <div className={s.vrow}>
                  <label className={s.vlabel}>用户名</label>
                  <input
                    className={s.inp}
                    type="text"
                    value={d.username}
                    onChange={(e) => setDraftField(p, 'username', e.target.value)}
                    placeholder="推送 / 建 PR 用的账号名"
                    aria-label={`${PLATFORM_LABEL[p]} 用户名`}
                  />
                </div>

                <div className={s.vrow}>
                  <label className={s.vlabel}>Token</label>
                  {/*
                    Token 框的三种形态（作者 2026-10-04 立、2026-10-07 改）：
                    · **已配置且不在编辑** → 显示一串假的掩码点（`••••••••`），**不是空的** ——
                      让人一眼看到"这里有值、已经填好了"，而不是怀疑自己是不是没存上。
                    · **点进去要改** → 输入框接受新的 Token；已配置的用掩码点当**占位**，
                      进来不再显得是空的（存完又回到掩码态）。
                    · **眼睛**（右侧那颗）→ 显示 / 收起**明文**，明文旁边多一颗【复制】
                      （作者 2026-10-07：不要密码，点眼睛直接看）。
                    ★ 用 `type="text"`（不是 password）：password 会带浏览器自带的"小眼睛"，
                      作者不要它。掩码是我们自己画的字符，不需要浏览器帮我们遮。
                  */}
                  {view?.hasToken === true && editingToken[p] !== true ? (
                    revealed[p] !== undefined ? (
                      /* 眼睛睁着：明文（只读、点一下整段选中）—— 右边跟一颗【复制】 */
                      <input
                        className={`${s.inp} ${s.mono}`}
                        type="text"
                        readOnly
                        value={revealed[p]}
                        onFocus={(e) => e.currentTarget.select()}
                        aria-label={`${PLATFORM_LABEL[p]} Token 明文`}
                      />
                    ) : (
                      <button
                        type="button"
                        className={`${s.inp} ${s.tokenMask}`}
                        onClick={() => setEditingToken((prev) => ({ ...prev, [p]: true }))}
                        aria-label={`${PLATFORM_LABEL[p]} Token 已配置，点击可修改`}
                        title="已配置。点击可填入新的 Token（不改则保持原值）"
                      >
                        ••••••••••••
                      </button>
                    )
                  ) : (
                    <input
                      className={s.inp}
                      type="text"
                      autoComplete="off"
                      spellCheck={false}
                      value={d.token}
                      onFocus={() => setEditingToken((prev) => ({ ...prev, [p]: true }))}
                      onChange={(e) => setDraftField(p, 'token', e.target.value)}
                      onBlur={() => {
                        /* 已配置的 Token，如果点进来什么也没填就退出 —— 回到掩码态，
                           不让人对着一格空输入框怀疑"是不是被我清掉了" */
                        if (view?.hasToken === true && d.token === '') {
                          setEditingToken((prev) => ({ ...prev, [p]: false }))
                        }
                      }}
                      /* 已配置时用掩码点当占位：点进来**不显得空**（作者 2026-10-07） */
                      placeholder={
                        view?.hasToken === true ? '••••••••••••' : '粘贴 Personal Access Token'
                      }
                      aria-label={`${PLATFORM_LABEL[p]} Token`}
                    />
                  )}
                  {/* 眼睛（作者 2026-10-07）：显示 / 收起明文；明文可整段选中，也可一键复制。
                      编辑态不摆它 —— 那时候在填新的，没有"看旧的"这回事。 */}
                  {view?.hasToken === true && editingToken[p] !== true && (
                    <>
                      <button
                        type="button"
                        className={s.tokenEye}
                        disabled={revealBusy === p}
                        onClick={() => void toggleReveal(p)}
                        aria-label={`${PLATFORM_LABEL[p]} Token 明文查看`}
                        title={revealed[p] !== undefined ? '收起明文' : '显示明文（可复制）'}
                      >
                        <svg viewBox="0 0 20 20" aria-hidden>
                          <path
                            d="M2 10s3-5 8-5 8 5 8 5-3 5-8 5-8-5-8-5z"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.5"
                          />
                          {revealed[p] !== undefined ? (
                            <path
                              d="M3.5 3.5 16.5 16.5"
                              fill="none"
                              stroke="currentColor"
                              strokeWidth="1.5"
                            />
                          ) : (
                            <circle cx="10" cy="10" r="2.2" fill="currentColor" />
                          )}
                        </svg>
                      </button>
                      {revealed[p] !== undefined && (
                        <button type="button" className={s.btn} onClick={() => void copyToken(p)}>
                          复制
                        </button>
                      )}
                    </>
                  )}
                </div>

                <div className={s.vrow}>
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnPrimary}`}
                    disabled={
                      acctBusy || d.repositoryUrl.trim() === '' || d.username.trim() === ''
                    }
                    onClick={() => void saveAccount(p)}
                  >
                    保存
                  </button>
                  {(view?.hasToken === true ||
                    (view?.repositoryUrl ?? '') !== '' ||
                    (view?.username ?? '') !== '') && (
                    <button
                      type="button"
                      className={s.btn}
                      disabled={acctBusy}
                      onClick={() => void clearAccount(p)}
                    >
                      清除
                    </button>
                  )}
                </div>
              </div>
            )
          })}

          {acctNote && (
            <p className={s.vhelp} style={acctNote.bad ? { color: 'var(--danger)' } : undefined}>
              {acctNote.text}
            </p>
          )}
        </div>
      </div>

      <div className={s.card}>
        <div className={s.cardHead}>
          <b>官方源（Bootstrap）</b>
          <span className={s.cardNote}>发布产物发到哪 —— 工作台唯一一处；客户端构建时编进去</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              发布出去的 <span className={s.mono}>presets/delivery/</span> 推到哪里 —— 客户端拿它那口
              <span className={s.mono}> source.json </span>找回目录与文件。
              <b>填仓库地址就够</b>（仓库地址或 <span className={s.mono}>.git</span> 克隆地址）——
              我们会自动补成发布入口的 raw 直链（GitHub / Gitee 是同一座桥）。
              GitHub 是<b>主源</b>；Gitee 是<b>镜像</b>（国内直连），空着 = 不提供这一档。
              也收：指向 <span className={s.mono}>source.json</span> 的 blob / raw 直链、
              自建源（<span className={s.mono}>http://…</span> 原样）。
              入库（<span className={s.mono}>workbench/bootstrap.json</span>）：换机器、CI 拿的都是同一份。
            </p>
            <div className={s.vrow}>
              <label className={s.vlabel} htmlFor="bootstrap-gh">
                GitHub
              </label>
              <input
                id="bootstrap-gh"
                className={s.inp}
                value={bootstrap}
                onChange={(e) => setBootstrap(e.target.value)}
                /* 中性示例（`<owner>/<repo>`），**不是"默认值"** ——
                   空着就是"还没配"，别让占位符看起来像已经填好了 */
                placeholder="https://github.com/<owner>/<repo>（主源）"
                aria-label="官方源（GitHub 主源）地址"
              />
            </div>
            <div className={s.vrow}>
              <label className={s.vlabel} htmlFor="bootstrap-gitee">
                Gitee
              </label>
              <input
                id="bootstrap-gitee"
                className={s.inp}
                value={giteeBootstrap}
                onChange={(e) => setGiteeBootstrap(e.target.value)}
                placeholder="https://gitee.com/<owner>/<repo>（镜像，空 = 不提供）"
                aria-label="官方源（Gitee 镜像）地址"
              />
            </div>
            <div className={s.vrow}>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary}`}
                disabled={
                  busy ||
                  bootstrap.trim() === '' ||
                  (bootstrap.trim() === (saved ?? '') &&
                    giteeBootstrap.trim() === (giteeSaved ?? ''))
                }
                onClick={() => void save()}
              >
                {busy ? '保存中……' : '保存'}
              </button>
              {/* 从磁盘重读 —— 界面之外改过这份配置时，用它看真相（见页头） */}
              <button
                type="button"
                className={s.btn}
                disabled={busy}
                title="丢掉输入框里没保存的内容，重新从 workbench/bootstrap.json 读一遍"
                onClick={() => void reread()}
              >
                重新读取
              </button>
            </div>
            <p className={s.vhelp}>
              当前（磁盘 <span className={s.mono}>workbench/bootstrap.json</span> 里的那份）：
              {saved === null ? (
                'GitHub 还没配 —— 客户端构建时不会注入默认源（下载会如实说「没配」）'
              ) : (
                <span className={s.mono}>{saved}</span>
              )}
              ；Gitee：
              {giteeSaved === null ? (
                '没配（客户端不出现 Gitee 档）'
              ) : (
                <span className={s.mono}>{giteeSaved}</span>
              )}
            </p>
            <p className={s.vhelp}>
              两个「Gitee」是两回事：这里的 Gitee 是<b>预设数据从哪读</b>（客户端里的镜像源档）；
              上面「发布账户」的 Gitee 是<b>软件版本 Release 发到哪</b>（安装包附件）。
            </p>
            {note && (
              <p className={s.vhelp} style={note.bad ? { color: 'var(--danger)' } : undefined}>
                {note.text}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* ——— 本地测试源（开发）：一颗按钮 = npm run dev:test-update ——— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>本地测试源（开发）</b>
          <span className={s.cardNote}>
            起本地假云端 + 带 MKPSE_PRESET_SOURCE_URL 起客户端 dev —— 不用为了验"官方发了新版本"真去发一版
          </span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              点「启动」就是替你在这个仓库里敲
              <span className={s.mono}>{devSrc?.command ?? 'npm run dev:test-update'}</span>
              ：夹具不在先派生 → 起 <span className={s.mono}>scripts/preset-test-server</span>
              在下面这个地址上 → 再把客户端 <span className={s.mono}>tauri dev</span>
              一起起起来，官方源就指向它。
            </p>
            <p className={s.vhelp}>
              客户端<b>不感知这是测试源</b> —— 它仍走 catalog / manifest / 寻址 / SHA 校验 /
              下载那一条真链，换的只是"去哪儿取"；生产构建里这段代码整个不存在。日志打在
              <b>起工作台的那个终端</b>里 —— 这个按钮只是替你敲了那条命令，不另造一套日志窗口。
            </p>

            <div className={s.vhead}>
              <b>状态</b>
              <span className={s.vkey}>
                {devSrc === null
                  ? '读取中……'
                  : devSrc.running
                    ? `运行中（PID ${devSrc.pid ?? '—'}）`
                    : '没在跑'}
              </span>
            </div>

            <div className={s.vrow}>
              <label className={s.vlabel}>服务地址</label>
              <span className={`${s.vstatic} ${s.mono}`}>
                {devSrc?.url ?? 'http://127.0.0.1:8787'}
              </span>
            </div>

            <div className={s.vrow}>
              <button
                type="button"
                className={`${s.btn} ${devSrc?.running === true ? '' : s.btnPrimary}`}
                disabled={devBusy || devSrc === null}
                title={
                  devSrc?.running === true
                    ? '连同它起的客户端 dev 一起停（整棵进程树），并把服务端口让出来'
                    : undefined
                }
                onClick={() => void toggleDevSrc()}
              >
                {devSrc?.running === true ? '停止' : '启动'}
              </button>
              <button
                type="button"
                className={s.btn}
                disabled={devBusy}
                title="现问一次后端 —— 状态是现问子进程得来的，不靠界面自己记"
                onClick={() => void readDevSrc()}
              >
                刷新状态
              </button>
            </div>

            {devLine !== null && (
              <p className={s.vhelp} style={devBad ? { color: 'var(--danger)' } : undefined}>
                {devLine}
              </p>
            )}

            <p className={s.vhelp}>
              只想起服务、不起客户端 dev：用{' '}
              <span className={s.mono}>npm run preset-source:dev</span>
              ，再把上面这个地址填进<b>客户端</b>的「设置 → 高级设置 → 预设数据源 → 自定义地址」
              （保存即生效）。两边各管各的 —— 这一格换的是环境变量那条路，客户端那一格归客户端设置页管。
            </p>

            <p className={s.vhelp}>
              ★ <b>客户端 dev 已经在跑时，先把它停掉再点</b>：那一份没法被重新指源
              （源是它启动时的环境变量），而新起的这份会撞在同一个 vite 端口上 ——
              终端里会说 <span className={s.mono}>Port 5321 is in use</span>，
              这里的状态随后会如实说它退出了。
            </p>
          </div>
        </div>
      </div>

      <div className={s.card}>
        <div className={s.cardHead}>
          <b>数据根</b>
          <span className={s.cardNote}>唯一真相源 —— 没有第二候选、不 fallback</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>预设源</b>
              <span className={s.vkey}>presets</span>
            </div>
            <p className={s.vhelp}>
              工作台读写的唯一预设根。改它要改的是仓库结构，不是这里的某一格。
            </p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.presets}</span>
            </div>
          </div>

          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>配方本</b>
              <span className={s.vkey}>workbench</span>
            </div>
            <p className={s.vhelp}>开发源数据、草稿与回收站住的地方。</p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.workbench}</span>
            </div>
          </div>

          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>交付产物</b>
              <span className={s.vkey}>dist</span>
            </div>
            <p className={s.vhelp}>
              「生成与发布」写出的那一批：人维护 <span className={s.mono}>presets/*.toml</span>，
              机器生成 <span className={s.mono}>presets/delivery/*</span>。
            </p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.delivery}</span>
            </div>
          </div>
        </div>
      </div>

      <div className={s.card}>
        <div className={s.cardHead}>
          <b>工作台子目录</b>
          <span className={s.cardNote}>谁写谁读，由后端一句话说清</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vstack}>
            {boot.storeDirs.map((d) => (
              <div key={d.name} className={s.vrow}>
                <span className={`${s.vkey} ${s.mono}`}>{d.name}</span>
                <span>{d.role}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {boot.problem && (
        <div className={s.card}>
          <div className={s.cardHead}>
            <b>开场问题</b>
          </div>
          <div className={s.cardBody}>
            <p className={s.vhelp}>
              {boot.problem}
              {boot.detail && (
                <>
                  {' —— '}
                  <span className={s.mono}>{boot.detail}</span>
                </>
              )}
            </p>
          </div>
        </div>
      )}
    </div>
  )
}
