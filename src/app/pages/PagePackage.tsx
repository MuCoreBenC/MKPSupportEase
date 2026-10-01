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
  /* 获取 / 使用之后重新读一遍本机状态（localStorage 不是响应式的） */
  const [tick, setTick] = useState(0)

  useEffect(() => {
    let alive = true
    setStatus('syncing')
    void (async () => {
      /* ① 说明书：进页面自己去看一眼云端，指纹不同就换掉 */
      const r = await autoSync()
      const list = await listCloud()
      if (!alive) return
      setCloud(list)
      setCur(r.local)
      setStatus(r.status)
    })()
    return () => {
      alive = false
    }
  }, [tick])

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
