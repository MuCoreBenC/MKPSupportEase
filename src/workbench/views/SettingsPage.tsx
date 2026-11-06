/**
 * 「设置」页 —— 数据根只读 + **一块可编辑的配置**（云端源）。
 *
 * 数据根（presets / workbench / dist）**不可配**：本仓的预设真相源固定是
 * `<repo>/presets`（没有第二候选、不 fallback），所以这一页把它们摆开，让人一眼看清
 * 工作台在读哪几个根、工作台子目录各自谁写谁读 —— 排查「读错了目录 / 数据长在哪」
 * 的那种问题用。上游根与回退规则那两块**已经整层删掉**（`HANDOFF.md`），照实不摆。
 *
 * 2026-10-02（第十七刀）起多了一块**可编辑**的：「云端源（Bootstrap）」——
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
 * 上面那一格是**发布到哪**（构建期注入）。这一格是**起一份本地假云端** ——
 * 一颗按钮替你敲 `npm run preset-source:dev`，把 `scripts/preset-test-server`
 * 起在 `127.0.0.1:8787` 上，于是不用为了验"云端发了新版本"真去发一版。
 *
 * ★ **它只起服务，不碰客户端**（作者 2026-10-08 裁决）。先前那一版连客户端
 * `tauri dev` 一起起（`npm run dev:test-update` + `MKPSE_PRESET_SOURCE_URL`）——
 * 作者的判断很直接：「这不就是单开一个服务吗？我自己输入这个地址就可以」。
 * 于是地址由人填一次：客户端的「设置 → 高级设置 → 预设数据源 → 自定义地址」，
 * **保存即生效**（见 `runtime/source.rs`）。那一格是客户端自己的运行时设置，
 * 归客户端设置页管，这里不碰。
 *
 * ★ **端口被占着时先把占用者摆出来**（2026-10-08 踩出来的那条）：8787 上已经有东西
 * （上一轮留下的服务、你手动起的 `preset-source:dev`），这一份就起不来。
 * 现在起之前先探那个端口，把"谁占着、PID 多少"列出来，并给一颗「停掉它」——
 * 问一句，而不是让人对着一句"退出码 1"去终端里猜。
 */
import { useCallback, useEffect, useState } from 'react'

import type { ThemePref } from '../../api/contract'
import { isAppError, wb } from '../api'
import type { Boot, DevSourceStatus, PortConflict, PublishAccount } from '../api'
/* 主题那一档与"现在到底深还是浅"都在 `theme.ts` —— 这一页不自己再落一次 `data-theme` */
import { setTheme, useResolvedDark, useThemePref } from '../theme'
import s from '../c14.module.css'

/** 外观三档（与客户端「设置 → 外观」同一套说法、同一个 id —— 后端那边的 ThemePref 钉着） */
const THEME_OPTS: { id: ThemePref; label: string; note: string }[] = [
  { id: 'system', label: '跟随系统', note: '系统切深色就跟着深色 —— 没选过的时候就是这一档' },
  { id: 'light', label: '浅色', note: '一直是浅色，不管系统怎么设' },
  { id: 'dark', label: '深色', note: '一直是深色，不管系统怎么设' },
]

interface Props {
  boot: Boot
}

export default function SettingsPage({ boot }: Props) {
  /* 云端源那两格：初值来自 boot；保存成功后本地回显（真值在 workbench/bootstrap.json，
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
   * 「本地测试源（开发）」的状态与那几颗按钮。
   *
   * ★ **跑着、或者有端口被占着的时候**每 2 秒问一次 —— 工作台的页面挂过就一直挂着
   * （`App.tsx` 的 `.pageSlot`：切走只是藏起来，不卸载），无条件轮询等于离开这一页
   * 之后还在空转 IPC。冲突那一路要轮询，是因为作者可能刚在别的终端把那个 dev 停了。
   */
  const [devSrc, setDevSrc] = useState<DevSourceStatus | null>(null)
  const [devBusy, setDevBusy] = useState(false)
  const [devNote, setDevNote] = useState<{ text: string; bad: boolean } | null>(null)
  /**
   * 现读一次状态。
   *
   * ★ 2026-11-06：没有"草稿选中项"了 —— 作者把那条选择撤了
   * （「任何东西都不用选，它只是把它预览出来」）。这一页只读盘上的事实。
   */
  const readDevSrc = useCallback(async () => {
    try {
      const st = await wb.devSourceStatus()
      setDevSrc(st)
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    }
  }, [])

  useEffect(() => {
    void readDevSrc()
  }, [readDevSrc])

  const devWatch = devSrc?.running === true || (devSrc?.conflicts.length ?? 0) > 0
  useEffect(() => {
    if (!devWatch) return
    const timer = setInterval(() => {
      wb.devSourceStatus().then(setDevSrc).catch(() => undefined)
    }, 2000)
    return () => clearInterval(timer)
  }, [devWatch])

  /** 「启动」= 起服务（端当前交付；本地发布那层后端自动叠）。已经在跑就是幂等 */
  const startDevSrc = async () => {
    if (devBusy) return
    setDevBusy(true)
    setDevNote(null)
    try {
      const next = await wb.devSourceStart()
      setDevSrc(next)
      setDevNote({ text: `已启动 —— ${next.url}`, bad: false })
    } catch (e) {
      /* 起不来时后端会把理由写进消息里（没生成 / 是谁占着那个端口），
         但状态也可能是旧的 —— 失败之后立刻重取一次，别让界面停在过期的读数上 */
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
      await readDevSrc()
    } finally {
      setDevBusy(false)
    }
  }

  const stopDevSrc = async () => {
    if (devBusy) return
    setDevBusy(true)
    setDevNote(null)
    try {
      const next = await wb.devSourceStop()
      setDevSrc(next)
      setDevNote({ text: next.note ?? '已停，服务端口也让出来了。', bad: false })
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setDevBusy(false)
    }
  }

  /** 字节数 → 人话（文件表那一列） */
  const sizeText = (v: number | null): string => {
    if (v === null) return '—'
    if (v >= 1024 * 1024) return `${(v / 1024 / 1024).toFixed(1)} MB`
    if (v >= 1024) return `${Math.round(v / 1024)} KB`
    return `${v} B`
  }

  /**
   * 停掉占着某个端口的那个进程（那颗「停掉它」）。
   *
   * 把**当时看到的 pid** 一起交回后端 —— 它会重新问一次"这个端口现在是谁的"，
   * 对不上就拒绝动手（PID 会被复用）。
   */
  /** 「打开交付目录」：路径用后端 DTO 里那一串（界面不自己拼） */
  const openDevRoot = async () => {
    const root = devSrc?.sourceRoot
    if (root === undefined || root === '') return
    try {
      await wb.openPath(root)
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    }
  }

  const clearConflict = async (c: PortConflict) => {
    if (devBusy) return
    setDevBusy(true)
    setDevNote(null)
    try {
      const next = await wb.devSourceClearConflict(c.port, c.pid)
      setDevSrc(next)
      const left = next.conflicts.length
      setDevNote({
        text:
          left === 0
            ? `${next.note ?? '占用者已停'} —— 现在可以点「启动」了。`
            : `${next.note ?? '停是停了'}，但还有 ${left} 条占用没处理。`,
        bad: left > 0,
      })
    } catch (e) {
      setDevNote({ text: isAppError(e) ? e.message : String(e), bad: true })
      await readDevSrc()
    } finally {
      setDevBusy(false)
    }
  }

  /* 端口占用那几条（起服务之前要人处理掉的） */
  const devConflicts = devSrc?.conflicts ?? []

  /*
   * 外观（深浅）：**选哪一档**由 `theme.ts` 拿着（它同时管着 `<html data-theme>`），
   * 这一页只负责"点一下 → 交后端落盘 → 回来那一档是什么就显示什么"。
   *
   * ★ 真值一律取**后端答回来的那一份**（`setTheme` 的返回值）—— 界面不自己记住
   *   "我以为是深色"，否则后端拒了而界面已经翻过去，就是"界面说了不算"。
   */
  const themePref = useThemePref()
  const isDark = useResolvedDark()
  const [themeBusy, setThemeBusy] = useState(false)
  const [themeNote, setThemeNote] = useState<string | null>(null)

  const pickTheme = async (next: ThemePref) => {
    if (themeBusy) return
    setThemeBusy(true)
    setThemeNote(null)
    try {
      await setTheme(next)
    } catch (e) {
      setThemeNote(isAppError(e) ? e.message : e instanceof Error ? e.message : String(e))
    } finally {
      setThemeBusy(false)
    }
  }

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
      {/* ——— 外观：深浅两套配色（2026-10-16，与客户端同一套取值） ——— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>外观</b>
          <span className={s.cardNote}>
            深浅两套配色 —— 现在这一窗是{isDark ? '深色' : '浅色'}
          </span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              工作台与客户端<b>各记各的一档</b>（两个应用的数据根是分开的，谁也不去读对方那棵树）
              —— 这儿改的是<b>工作台这一窗</b>；客户端那扇窗在它自己的「设置 → 外观」里。
            </p>
            {THEME_OPTS.map((o) => (
              <label
                key={o.id}
                className={s.vrow}
                style={{ alignItems: 'flex-start', cursor: 'pointer' }}
              >
                <input
                  type="radio"
                  name="wb-theme"
                  value={o.id}
                  /* 「没选过」= 跟随系统那一档（`null` 与 `system` 是一个意思） */
                  checked={(themePref ?? 'system') === o.id}
                  disabled={themeBusy}
                  onChange={() => void pickTheme(o.id)}
                />
                <span className={s.grow}>
                  <b>{o.label}</b>
                  <span className={s.vhelp} style={{ display: 'block' }}>
                    {o.note}
                  </span>
                </span>
              </label>
            ))}
            {themeNote !== null && (
              <p className={s.vhelp} style={{ color: 'var(--danger)' }}>
                {themeNote}
              </p>
            )}
          </div>
        </div>
      </div>

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
                        data-tip={revealed[p] !== undefined ? '收起明文' : '显示明文（可复制）'}
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
          <b>云端源（Bootstrap）</b>
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
                aria-label="云端源（GitHub 主源）地址"
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
                aria-label="云端源（Gitee 镜像）地址"
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

      {/* ——— 本地测试源（开发）：一颗按钮 = npm run preset-source:dev（只起服务） ——— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>本地测试源</b>
          <span className={s.cardNote}>只读 —— 下面每一格都是盘上与运行时的真实读数</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>
                {devSrc === null ? '读取中……' : devSrc.running ? '运行中' : '没在跑'}
                {devSrc?.running === true && devSrc.pid !== null && (
                  <span className={s.vkey}>· PID {devSrc.pid}</span>
                )}
              </b>
              <span className={s.mono}>{devSrc?.url ?? 'http://127.0.0.1:8787'}</span>
            </div>

            <div className={s.vrow}>
              {devSrc?.running === true ? (
                <button
                  type="button"
                  className={s.btn}
                  disabled={devBusy}
                  onClick={() => void stopDevSrc()}
                >
                  停止
                </button>
              ) : (
                <button
                  type="button"
                  className={`${s.btn} ${s.btnPrimary}`}
                  disabled={devBusy || devSrc === null || devConflicts.length > 0}
                  onClick={() => void startDevSrc()}
                >
                  启动
                </button>
              )}
              <button
                type="button"
                className={s.btn}
                disabled={devBusy}
                onClick={() => void readDevSrc()}
              >
                刷新状态
              </button>
              <button
                type="button"
                className={s.btn}
                disabled={devSrc === null}
                onClick={() => void openDevRoot()}
              >
                打开交付目录
              </button>
              <span className={s.grow} />
              {devNote !== null && (
                <span
                  className={s.vhelp}
                  style={devNote.bad ? { color: 'var(--danger)' } : undefined}
                >
                  {devNote.text}
                </span>
              )}
            </div>

            {/* ——— 版本：三格并排（每格都是它自己的真实身份） ——— */}
            <div className={s.vhead}>
              <b>版本</b>
              <span className={s.vkey}>开发中 / 已发布 / 包内自报，各说各的</span>
            </div>

            <div className={s.vrow}>
              <label className={s.vlabel}>开发中</label>
              <span className={`${s.vstatic} ${s.mono}`}>{devSrc?.devVersion ?? '—'}</span>
              <span className={s.grow} />
            </div>
            <div className={s.vrow}>
              <label className={s.vlabel}>已发布</label>
              <span className={`${s.vstatic} ${s.mono}`}>
                {devSrc?.publishedVersion ?? '（还没发过）'}
              </span>
              <span className={s.grow} />
            </div>
            <div className={s.vrow}>
              <label className={s.vlabel}>包内自报</label>
              <span className={`${s.vstatic} ${s.mono}`}>
                {devSrc?.buildVersion ?? '（还没编过）'}
              </span>
              <span className={s.grow} />
            </div>

            {(devSrc?.issues.length ?? 0) > 0 ? (
              <div className={s.vfield}>
                {devSrc?.issues.map((issue) => (
                  <p key={issue} className={s.vhelp} style={{ color: 'var(--danger)' }}>
                    {issue}
                  </p>
                ))}
              </div>
            ) : (
              <p className={s.vhelp} style={{ color: 'var(--ok, #1a7f37)' }}>
                三处一致
              </p>
            )}

            {/* ——— 盘上有什么：文件 / 大小 / 状态（不摆假行） ——— */}
            <div className={s.vhead}>
              <b>交付文件</b>
              <span className={`${s.vkey} ${s.mono}`}>{devSrc?.sourceRoot ?? '—'}</span>
            </div>

            {(devSrc?.files ?? []).map((f) => (
              <div className={s.vrow} key={f.name}>
                <span
                  className={s.mono}
                  style={{ flex: '0 0 auto', minWidth: 240, opacity: f.exists ? 1 : 0.45 }}
                >
                  {f.name}
                </span>
                <span className={`${s.vstatic} ${s.mono}`}>{sizeText(f.size)}</span>
                <span className={s.grow} />
                <span
                  className={s.vhelp}
                  style={{ color: f.exists ? 'var(--ok, #1a7f37)' : 'var(--muted, #8c959f)' }}
                >
                  {f.exists ? '存在' : '不存在'}
                  {f.version !== null ? ` · ${f.version}` : ''}
                </span>
              </div>
            ))}

            <details>
              <summary className={s.vhelp}>路径与命令</summary>
              <p className={`${s.vhelp} ${s.mono}`}>交付根：{devSrc?.sourceRoot ?? '—'}</p>
              <p className={`${s.vhelp} ${s.mono}`}>仓库根：{devSrc?.repoRoot ?? '—'}</p>
              <p className={`${s.vhelp} ${s.mono}`}>
                命令：{devSrc?.command ?? 'npm run preset-source:dev'}
              </p>
            </details>

            {devConflicts.length > 0 && (
              <div className={s.vfield}>
                <div className={s.vhead}>
                  <b>端口被占着</b>
                  <span className={s.vkey}>先处理掉，再点「启动」</span>
                </div>
                {devConflicts.map((c) => (
                  <div className={s.vrow} key={c.port}>
                    <span>{c.text}</span>
                    <span className={s.grow} />
                    <button
                      type="button"
                      className={s.btn}
                      disabled={devBusy || c.pid === null}
                      data-tip={
                        c.pid === null
                          ? '认不出占着它的进程 —— 请在那个终端里自己停掉它'
                          : '把它停掉（后端会先确认这个端口还是它的，再动手）'
                      }
                      onClick={() => void clearConflict(c)}
                    >
                      停掉它
                    </button>
                  </div>
                ))}
                <p className={s.vhelp}>
                  多半是上一轮留下的服务，或者你手动起的
                  <span className={s.mono}>preset-source:dev</span>
                  —— 同一端口上只能有一个。停掉它，再点「启动」。
                </p>
              </div>
            )}

            <p className={s.vhelp}>
              ★ 它<b>只起服务</b>，不碰你的客户端 —— 地址自己填一次：
              <b>客户端</b>的「设置 → 高级设置 → 预设数据源 → 自定义地址」，把上面那一串贴进去
              （保存即生效，不用重启客户端）。
            </p>

            <p className={s.vhelp}>
              想连客户端的 <span className={s.mono}>tauri dev</span> 一起起（它会自动把源指过来，
              不用手填地址）：那在命令行里 ——{' '}
              <span className={s.mono}>npm run dev:test-update</span>
              。那条路靠 <span className={s.mono}>MKPSE_PRESET_SOURCE_URL</span> 注入，
              所以它必须把客户端 dev 一起管起来；这里这颗按钮不干那件事。
            </p>

            <p className={s.vhelp}>
              ★ <b>端口被占着的话，上面会先把它摆出来</b>（谁占着、PID 多少），
              点那颗「停掉它」就行 —— 后端会先确认那个端口还是它的才动手（PID 会被复用）。
              不在跑的时候这里每 2 秒问一次，所以你在别的终端把它停了，界面也会跟上。
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
