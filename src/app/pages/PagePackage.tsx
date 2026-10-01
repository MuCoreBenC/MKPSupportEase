/*
 * **同步**页（客户端同步中心）。
 *
 * 这一页的账在 C4 换源后只剩一份：**catalog**（`<appDataDir>/catalog.json`，
 * 随安装包释放、升级时旧份归档）。旧世界那格"说明书"（`ClientDataPackage` 的
 * localStorage 自动同步）已随换源退役 —— 「说明书」就是 catalog，账在下面那区里：
 *
 *   catalog 的账    schema / 指纹 / 机型与版本数 / 登记的交付文件
 *   下载区 mkp/     盘就是底账；更新 = 对过时文件重跑下载管道，旧份自动归档
 *   使用中指针      run/active-preset.json，全局唯一
 *   远端目录        检查 / 应用更新（两端共用契约，比较 revision 指纹）
 *
 * 预设（TOML）的下载与应用整段在「预设」页 —— 作者：「同步里面应该只放那一些
 * JSON 文件……它里面不会去放这些每个版本的」。
 */

import { useCallback, useEffect, useState } from 'react'
import s from './PagePackage.module.css'
import { api } from '../../api'
import type { ActivePreset, RemoteUpdateCheck, RuntimeCatalog } from '../../api/contract'

/** 下载区现状（盘就是底账）+ 下载按钮的失败说明（第一圈浏览器里没有源，如实亮出来） */
type WorldState = {
  catalog: RuntimeCatalog
  downloaded: string[]
  stale: string[]
  active: ActivePreset | null
}

export default function PagePackage() {
  const [world, setWorld] = useState<WorldState | null>(null)
  const [downloadErr, setDownloadErr] = useState<string | null>(null)
  const [downloading, setDownloading] = useState(false)
  /* 远端目录检查结果——只点「检查更新」才查（那是显式动作，不进首屏） */
  const [remote, setRemote] = useState<RemoteUpdateCheck | null>(null)

  useEffect(() => {
    let alive = true
    void (async () => {
      const [catalog, downloaded, stale, active] = await Promise.all([
        api.getRuntimeCatalog().catch(() => null),
        api.getDownloadedFiles().catch(() => null),
        api.getStaleFiles().catch(() => null),
        api.getActivePreset().catch(() => null),
      ])
      if (!alive) return
      setWorld(
        catalog !== null && downloaded !== null && stale !== null
          ? { catalog, downloaded, stale, active }
          : null,
      )
    })()
    return () => {
      alive = false
    }
  }, [])

  const refreshWorld = useCallback(async () => {
    const [catalog, downloaded, stale, active] = await Promise.all([
      api.getRuntimeCatalog().catch(() => null),
      api.getDownloadedFiles().catch(() => null),
      api.getStaleFiles().catch(() => null),
      api.getActivePreset().catch(() => null),
    ])
    setWorld(
      catalog !== null && downloaded !== null && stale !== null
        ? { catalog, downloaded, stale, active }
        : null,
    )
  }, [])

  /* 走新管道拉一份进下载区。成功后重问盘（不记账本），失败把话说在页面上 */
  const tryDownload = useCallback(async () => {
    if (world === null || world.catalog.files.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      await api.downloadCatalogFile(world.catalog.files[0].fileName)
      await refreshWorld()
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '下载没成，原因没说清')
    } finally {
      setDownloading(false)
    }
  }, [world, downloading, refreshWorld])

  const tryApply = useCallback(async () => {
    if (world === null || world.catalog.files.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      await api.applyActivePreset(world.catalog.files[0].fileName)
      await refreshWorld()
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '使用没成，原因没说清')
    } finally {
      setDownloading(false)
    }
  }, [world, downloading, refreshWorld])

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
    if (world === null || world.stale.length === 0 || downloading) return
    setDownloading(true)
    setDownloadErr(null)
    try {
      for (const name of world.stale) {
        await api.downloadCatalogFile(name)
      }
      await refreshWorld()
    } catch (e) {
      setDownloadErr((e as { message?: string }).message ?? '更新没成，原因没说清')
      await refreshWorld()
    } finally {
      setDownloading(false)
    }
  }, [world, downloading, refreshWorld])

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

  const files = world?.catalog.files ?? []

  return (
    <div className={s.page}>
      <header className={s.head}>
        <div>
          <h1 className={s.title}>同步</h1>
          <p className={s.sub}>
            说明书（catalog：有哪些机型版本、参数怎么画、什么条件下显示）随安装包走，
            程序自动管它的版本 —— 你不用管它。真正要你动手的是**预设（TOML）**：
            去「预设」页的云端表下载，下载完在本地表点「应用」。
          </p>
        </div>
      </header>

      {world !== null && (
        <section className={s.section}>
          <div className={s.secTitle}>说明书与下载区（catalog + mkp/）</div>
          <div className={s.kv}>
            <span className={s.key}>catalog</span>
            <span className={s.val}>
              schema {world.catalog.catalogSchema} · 指纹 {world.catalog.revision}
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>机型</span>
            <span className={s.val}>
              {world.catalog.machines.length} 台 ·{' '}
              {world.catalog.machines.reduce((n, m) => n + m.versions.length, 0)} 个版本
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>文件</span>
            <span className={s.val}>
              {files.length} 份在目录里登记，落在下载区 mkp/ —— 用户没下载的，盘上就没有
            </span>
          </div>
          {files.length > 0 && (
            <p className={s.note}>
              第一份真实文件：<b>{files[0].fileName}</b>（SHA {files[0].sha256.slice(0, 12)}…，{files[0].size} 字节）
            </p>
          )}
          <div className={s.kv}>
            <span className={s.key}>已下载</span>
            <span className={s.val}>
              {world.downloaded.length} / {files.length} 份（文件在盘上且 SHA 对得上才算数）
            </span>
          </div>
          <div className={s.kv}>
            <span className={s.key}>有更新</span>
            <span className={s.val}>
              {world.stale.length === 0
                ? '没有，都是最新'
                : `${world.stale.length} 份可以更新（换新前旧份自动归档）`}
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
              {world.active === null
                ? '还没用任何一份'
                : `${world.active.fileName}（${world.active.machineId} / ${world.active.versionId}）${
                    world.active.intact ? '' : ' —— 文件已经不是应用时的那份'
                  }`}
            </span>
          </div>
          <button
            type="button"
            className={s.btn}
            onClick={tryDownload}
            disabled={downloading || files.length === 0}
          >
            {downloading ? '正在下载……' : '下载第一份（走新管道）'}
          </button>
          {world.active !== null ? (
            <button type="button" className={s.btn} onClick={tryClear} disabled={downloading}>
              撤销使用
            </button>
          ) : (
            world.downloaded.length > 0 && (
              <button
                type="button"
                className={s.btn}
                onClick={tryApply}
                disabled={downloading || files.length === 0}
              >
                使用第一份
              </button>
            )
          )}
          {world.stale.length > 0 && (
            <button type="button" className={s.btn} onClick={tryUpdate} disabled={downloading}>
              {downloading ? '正在更新……' : `更新这 ${world.stale.length} 份到最新`}
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

      {world === null && (
        <section className={s.section}>
          <div className={s.secTitle}>说明书与下载区</div>
          <p className={s.note}>读不到目录（catalog）—— 程序的数据根里没有它。</p>
        </section>
      )}
    </div>
  )
}
