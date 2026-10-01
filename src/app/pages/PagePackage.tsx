/*
 * **同步**页（客户端同步中心）。
 *
 * 作者把这一页的定位改掉了：它不再是「让我手动下载 JSON」，而是**客户端同步中心**。
 * 之后又收窄了一刀（作者：「同步里面应该只放那一些 JSON 文件……它里面不会去放
 * 这些每个版本的」）—— 预设（TOML）的下载与应用整段搬去「预设」页：
 *
 *   JSON（说明书）  进客户端 / 刷新时**自动同步** —— 用户看不见「下载 JSON」这个动作；
 *                    这一页只讲它的账（上次同步 / 包版本 / 布局 / 模式开关）
 *   TOML（预设）    云端表的「下载」+ 本地表的「应用」，都在「预设」页
 *
 * 搬自试验场那一份：除文件名 / 组件名 / import 按本仓目录去稿号外，正文一字未动。
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import s from './PagePackage.module.css'
import { api } from '../../api'
import type { ActivePreset, RemoteUpdateCheck, RuntimeCatalog } from '../../api/contract'
import {
  autoSync,
  latestOf,
  listCloud,
  longStatText,
  pkgSummary,
} from '../store/package'
import type { CloudEntry, SyncedPackage, SyncStatus } from '../store/package'

interface Props {
  /** 「去参数页看看它画出来的样子」 */
  onOpenParams: () => void
}

/** 说明书里的布局（页签 → 分组 → 条数）；顺序就是包里的顺序，客户端不重排 */
interface LayoutOfPkg {
  tab: string
  count: number
  sections: { group: string; count: number }[]
}

function layoutOf(pkg: SyncedPackage['package']): LayoutOfPkg[] {
  const out: LayoutOfPkg[] = []
  for (const f of pkg.fields) {
    let tab = out.find((t) => t.tab === f.tab)
    if (tab === undefined) {
      tab = { tab: f.tab, count: 0, sections: [] }
      out.push(tab)
    }
    tab.count += 1
    const sec = tab.sections.find((x) => x.group === f.group)
    if (sec === undefined) tab.sections.push({ group: f.group, count: 1 })
    else sec.count += 1
  }
  return out
}

/** 说明书里的模式开关（一个条件参数管了几条、分哪几支） */
interface SwitchOfPkg {
  key: string
  label: string
  branches: { need: string; count: number }[]
}

const OP: Record<'eq' | 'neq' | 'gt', string> = { eq: '=', neq: '≠', gt: '>' }

function switchesOf(pkg: SyncedPackage['package']): SwitchOfPkg[] {
  const labelOf = new Map(pkg.fields.map((f) => [f.key, f.label]))
  const byKey = new Map<string, Map<string, number>>()
  for (const f of pkg.fields) {
    const cond = f.showWhen
    if (cond === undefined) continue
    if (!byKey.has(cond.key)) byKey.set(cond.key, new Map())
    const branches = byKey.get(cond.key)
    if (branches === undefined) continue
    const need = `${OP[cond.op]} ${cond.value}`
    branches.set(need, (branches.get(need) ?? 0) + 1)
  }
  const out: SwitchOfPkg[] = []
  for (const [key, branches] of byKey) {
    const list = [...branches.entries()].map(([need, count]) => ({ need, count }))
    const total = list.reduce((n, b) => n + b.count, 0)
    if (list.length < 2 || total < 2) continue
    out.push({ key, label: labelOf.get(key) ?? key, branches: list.sort((a, b) => b.count - a.count) })
  }
  return out
}

export default function PagePackage({ onOpenParams }: Props) {
  const [cloud, setCloud] = useState<CloudEntry[]>([])
  const [cur, setCur] = useState<SyncedPackage | null>(null)
  const [status, setStatus] = useState<SyncStatus | 'syncing'>('syncing')
  /* 新数据世界（第一圈）：随包 catalog 读一条真实数据。与旧世界的同步**互不拖累** ——
     一边失败另一边照常显示，catch 成 null 由界面说「读不到」 */
  const [world, setWorld] = useState<RuntimeCatalog | null>(null)
  /* 下载区现状（盘就是底账）+ 下载按钮的失败说明（第一圈浏览器里没有源，如实亮出来） */
  const [downloaded, setDownloaded] = useState<string[] | null>(null)
  const [stale, setStale] = useState<string[] | null>(null)
  const [downloadErr, setDownloadErr] = useState<string | null>(null)
  const [downloading, setDownloading] = useState(false)
  /* 使用中指针（全局唯一；intact=false 说明盘上的文件漂了，照实说） */
  const [active, setActive] = useState<ActivePreset | null>(null)
  /* 远端目录检查结果——只点「检查更新」才查（那是显式动作，不进首屏） */
  const [remote, setRemote] = useState<RemoteUpdateCheck | null>(null)
  /* 获取 / 使用之后重新读一遍本机状态（localStorage 不是响应式的） */
  const [tick, setTick] = useState(0)

  useEffect(() => {
    let alive = true
    setStatus('syncing')
    void (async () => {
      /* ① 说明书：进页面自己去看一眼云端，指纹不同就换掉 */
      const r = await autoSync()
      const list = await listCloud()
      const w = await api.getRuntimeCatalog().catch(() => null)
      const d = await api.getDownloadedFiles().catch(() => null)
      const s = await api.getStaleFiles().catch(() => null)
      const a = await api.getActivePreset().catch(() => null)
      if (!alive) return
      setCloud(list)
      setCur(r.local)
      setStatus(r.status)
      setWorld(w)
      setDownloaded(d)
      setStale(s)
      setActive(a)
    })()
    return () => {
      alive = false
    }
  }, [tick])

  /* 走新管道拉一份进下载区。成功后重问盘（不记账本），失败把话说在页面上 */
  const tryDownload = useCallback(async () => {
    if (world === null || world.files.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      await api.downloadCatalogFile(world.files[0].fileName)
      setDownloaded(await api.getDownloadedFiles())
    } catch (e) {
      const msg = (e as { message?: string }).message
      setDownloadErr(msg ?? '下载没成，原因没说清')
    } finally {
      setDownloading(false)
    }
  }, [world, downloading])

  /* 使用 / 撤销 / 更新之后，新世界区块整个重读一遍（下载区 + 过时清单 + 指针） */
  const refreshWorld = useCallback(async () => {
    setDownloaded(await api.getDownloadedFiles().catch(() => null))
    setStale(await api.getStaleFiles().catch(() => null))
    setActive(await api.getActivePreset().catch(() => null))
  }, [])

  const tryApply = useCallback(async () => {
    if (world === null || world.files.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      setActive(await api.applyActivePreset(world.files[0].fileName))
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '使用没成，原因没说清')
    } finally {
      setDownloading(false)
    }
  }, [world, downloading])

  const tryClear = useCallback(async () => {
    setDownloadErr(null)
    try {
      await api.clearActivePreset()
      await refreshWorld()
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '撤销没成，原因没说清')
    }
  }, [refreshWorld])

  /* 更新 = 对每一份过时文件重跑一遍下载管道：旧份自动归档，没有单独的更新代码路径 */
  const tryUpdate = useCallback(async () => {
    if (stale === null || stale.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      for (const name of stale) {
        await api.downloadCatalogFile(name)
      }
      await refreshWorld()
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '更新没成，原因没说清')
      await refreshWorld()
    } finally {
      setDownloading(false)
    }
  }, [stale, downloading, refreshWorld])

  /* 检查更新：对远端目录比较指纹。显式动作，不进首屏、不自动跑 */
  const checkRemote = useCallback(async () => {
    setDownloadErr(null)
    try {
      setRemote(await api.checkRemoteUpdate())
    } catch (e) {
      setRemote(null)
      setDownloadErr((e as { message?: string }).message ?? '检查没成，原因没说清')
    }
  }, [])

  /* 应用远端目录：旧目录归档、新目录生效；之后 Stale 照常出现，用既有下载管道拉新 */
  const applyRemote = useCallback(async () => {
    if (downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      await api.applyRemoteUpdate()
      await refreshWorld()
      setRemote(await api.checkRemoteUpdate())
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '应用没成，原因没说清')
    } finally {
      setDownloading(false)
    }
  }, [downloading, refreshWorld])

  const latest = latestOf(cloud)
  const layout = useMemo(() => (cur === null ? [] : layoutOf(cur.package)), [cur])
  const switches = useMemo(() => (cur === null ? [] : switchesOf(cur.package)), [cur])

  const before = useCallback(() => setTick((x) => x + 1), [])

  const statusLine = (() => {
    if (status === 'syncing') return '正在同步说明书……'
    if (status === 'empty') return '云端暂时没有可同步的发布'
    return status === 'synced' ? '刚刚同步到最新' : '已是最新'
  })()

  return (
    <div className={s.page}>
      <header className={s.head}>
        <div>
          <h1 className={s.title}>同步</h1>
          <p className={s.sub}>
            说明书（有哪些机型版本、参数怎么画、什么类型、什么条件下显示）会**自动同步** ——
            你不用管它。真正要你动手的是**预设（TOML）**：去「预设」页的云端表下载，
            下载完在本地表点「应用」。
          </p>
        </div>
        <div className={s.states}>
          <div className={s.stateRow}>
            <span className={s.stateKey}>上次同步</span>
            {/* 本机时间戳统一显示（带年份；老值凑不出年份就只到月日，不编） */}
            <span className={s.stateVal}>{longStatText(cur?.syncedAt) ?? '—'}</span>
          </div>
          <div className={s.stateRow}>
            <span className={s.stateKey}>状态</span>
            <span className={s.stateVal}>{statusLine}</span>
          </div>
          <button type="button" className={s.btn} onClick={before}>
            重新检查
          </button>
        </div>
      </header>

      <section className={s.section}>
        <div className={s.secTitle}>总览</div>
        <div className={s.kv}>
          <span className={s.key}>说明书</span>
          <span className={s.val}>
            {cur === null ? '还没同步下来' : `已同步 ${cur.version ?? '（没记版本）'}`}
            {cur !== null && <span className={s.origin}>{cur.name}</span>}
          </span>
        </div>
        {cur !== null && <p className={s.note}>{pkgSummary(cur.package)}</p>}
        {latest !== null && (
          <p className={s.note}>
            云端最新那一次发布：<b>{latest.name}</b>
            {latest.version ? ` · 包版本 ${latest.version}` : ''}
            {latest.at ? ` · ${latest.at}` : ''} · 预设文件 {latest.presets.length} 份
          </p>
        )}
        {status === 'synced' && (
          <p className={`${s.note} ${s.staleNote}`}>刚把这台机器上的说明书换成了云端最新那一份。</p>
        )}
      </section>

      {/*
        新数据世界（第一圈骨架）：随包 catalog 的真实读数。
        与上面旧世界的演示同步摆在一起 —— 新旧交接的账，一眼看得见。
        文件列表未来长成下载入口（第一圈只展示，不发任何网络请求）。
      */}
      {world !== null && (
        <section className={s.section}>
          <div className={s.secTitle}>数据骨架（新）</div>
          <div className={s.kv}>
            <span className={s.key}>catalog</span>
            <span className={s.val}>
              schema {world.catalogSchema} · 指纹 {world.revision}
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>机型</span>
            <span className={s.val}>
              {world.machines.length} 台 ·{' '}
              {world.machines.reduce((n, m) => n + m.versions.length, 0)} 个版本
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>文件</span>
            <span className={s.val}>
              {world.files.length} 份在目录里登记，落在下载区 mkp/ —— 用户没下载的，盘上就没有
            </span>
          </div>
          {world.files.length > 0 && (
            <p className={s.note}>
              第一份真实文件：<b>{world.files[0].fileName}</b>（SHA{' '}
              {world.files[0].sha256.slice(0, 12)}…，{world.files[0].size} 字节）
            </p>
          )}
          <div className={s.kv}>
            <span className={s.key}>已下载</span>
            <span className={s.val}>
              {downloaded === null
                ? '—'
                : `${downloaded.length} / ${world.files.length} 份（文件在盘上且 SHA 对得上才算数）`}
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>有更新</span>
            <span className={s.val}>
              {stale === null
                ? '—'
                : stale.length === 0
                  ? '没有，都是最新'
                  : `${stale.length} 份可以更新（换新前旧份自动归档）`}
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>远端目录</span>
            <span className={s.val}>
              {remote === null
                ? '没查过'
                : remote.upToDate
                  ? '已是最新（指纹一致）'
                  : `有新目录 ${remote.remoteRevision}（本地 ${remote.localRevision}）`}
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>当前使用</span>
            <span className={s.val}>
              {active === null
                ? '还没用任何一份'
                : `${active.fileName}（${active.machineId} / ${active.versionId}）${
                    active.intact ? '' : ' —— 文件已经不是应用时的那份'
                  }`}
            </span>
          </div>
          <button
            type="button"
            className={s.btn}
            onClick={tryDownload}
            disabled={downloading || world.files.length === 0}
          >
            {downloading ? '正在下载……' : '下载第一份（走新管道）'}
          </button>
          {active !== null ? (
            <button type="button" className={s.btn} onClick={tryClear} disabled={downloading}>
              撤销使用
            </button>
          ) : (
            downloaded !== null &&
            downloaded.length > 0 && (
              <button
                type="button"
                className={s.btn}
                onClick={tryApply}
                disabled={downloading || world.files.length === 0}
              >
                使用第一份
              </button>
            )
          )}
          {stale !== null && stale.length > 0 && (
            <button type="button" className={s.btn} onClick={tryUpdate} disabled={downloading}>
              {downloading ? '正在更新……' : `更新这 ${stale.length} 份到最新`}
            </button>
          )}
          <button type="button" className={s.btn} onClick={checkRemote} disabled={downloading}>
            检查更新
          </button>
          {remote !== null && !remote.upToDate && (
            <button type="button" className={s.btn} onClick={applyRemote} disabled={downloading}>
              应用远端目录
            </button>
          )}
          {downloadErr !== null && (
            <p className={`${s.note} ${s.staleNote}`}>下载没成：{downloadErr}</p>
          )}
        </section>
      )}

      {/*
        这一页不再摆「每个版本」的 TOML —— 预设（下载 / 应用）整段搬去「预设」页
        （作者：「同步里面应该只放那一些 JSON 文件……它里面不会去放这些每个版本的」）。
        留下来的是说明书的账：总览（上面）+ 布局 + 模式开关（下面）。
      */}

      {cur !== null && (
        <section className={s.section}>
          <div className={s.secTitle}>
            这份说明书里有什么（参数页照它画）
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={onOpenParams}>
              去看参数页
            </button>
          </div>
          <div className={s.layout}>
            {layout.map((t) => (
              <div key={t.tab} className={s.layoutTab}>
                <div className={s.layoutTabHead}>
                  {t.tab}
                  <span className={s.count}>{t.count}</span>
                </div>
                {t.sections.map((sec) => (
                  <div key={sec.group} className={s.layoutSec}>
                    {sec.group}
                    <span className={s.count}>{sec.count}</span>
                  </div>
                ))}
              </div>
            ))}
          </div>
          {switches.length > 0 && (
            <p className={s.note}>
              模式开关（由说明书里的 showWhen 算出来）：
              {switches.map((sw) => `${sw.label}（${sw.branches.map((b) => `${b.need} ${b.count} 条`).join(' / ')}）`).join('；')}
            </p>
          )}
        </section>
      )}
    </div>
  )
}
