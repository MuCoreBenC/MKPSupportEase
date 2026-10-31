/*
 * 设置页「数据与维护 / 清理与卸载」那两块（2026-10-17）。
 *
 * 作者的范围裁断（逐字）：
 *   「那个备份恢复就不用做了，清理卸载呀，那一些可以做，我们已经有自动清理了，
 *     那就应该只做卸载。预设位置其实也就是打开位置而已，打开我们那个程序那些文件的位置」
 *
 * 所以这一份只做两件：
 *   1. 「预设位置」= 打开 `presets-local/` 那个目录（**只是打开文件夹**，不读不写内容）
 *   2. 「卸载」= 删程序本体，**用户数据一个不碰**
 *
 * 不做：导出数据 / 导入数据 / 分类清理（备份恢复已有、自动清理已有）。
 * 那两块**故意不摆占位** —— 摆一个点了没用的按钮比不摆更糟。
 *
 * ★ 文案走字典（2026-10-25）。风险那一段里"您的数据会保留"原本是 `<strong>`，
 *   现在拆成前 / 中 / 后三个键，中间那段仍包在 `<strong>` 里（英文也一样加粗）。
 */

import { useEffect, useState } from 'react'

import { msg, useText, type Text } from '../../i18n'

import { api, errorText } from '../../api'
import { useT } from '../../i18n'
import { Btn } from '../ui/Controls'
import { Tip } from '../../components/tooltip'
import RecoverySection from './RecoverySection'
import s from './PageSettings.module.css'

/** 两段式确认的窗口：与「清空历史」那一套同一个时长（3 秒无操作就回退） */
const CONFIRM_MS = 3000

/** `host`：只有「最近删除」那个模态框要（遮罩铺满标题栏以下那一块），原样传下去 */
export default function DataSection({ host }: { host: HTMLElement | null }) {
  const tr = useT()
  const say = useText()
  const [folderBusy, setFolderBusy] = useState(false)
  /** 成功时是**落点路径原样**（不翻），失败时是存键的那句话 —— 两种都住 [`Text`] */
  const [folderNote, setFolderNote] = useState<Text | null>(null)

  /* 卸载：点了第一下变成「确认卸载」，3 秒没人再点就自己退回 */
  const [armed, setArmed] = useState(false)
  const [uninstallBusy, setUninstallBusy] = useState(false)
  const [uninstallNote, setUninstallNote] = useState<Text | null>(null)

  useEffect(() => {
    if (!armed) return
    const t = window.setTimeout(() => setArmed(false), CONFIRM_MS)
    return () => window.clearTimeout(t)
  }, [armed])

  const openFolder = () => {
    setFolderBusy(true)
    setFolderNote(null)
    void api.openPresetFolder().then(
      (path) => {
        setFolderNote(path)
        setFolderBusy(false)
      },
      (e: unknown) => {
        setFolderNote(msg('settings.data.openFailed', { detail: errorText(e) }))
        setFolderBusy(false)
      },
    )
  }

  const doUninstall = () => {
    if (!armed) {
      setArmed(true)
      setUninstallNote(null)
      return
    }
    setArmed(false)
    setUninstallBusy(true)
    void api.uninstall().then(
      /* 成功时后端已经 exit 了，这一行基本走不到；走到说明进程还活着 */
      () => setUninstallBusy(false),
      (e: unknown) => {
        setUninstallBusy(false)
        setUninstallNote(msg('settings.data.uninstallFailed', { detail: errorText(e) }))
      },
    )
  }

  return (
    <>
      <section className={s.section} aria-label={tr('settings.data.title')}>
        <h2 className={s.secTitle}>{tr('settings.data.title')}</h2>

        {/* 次级行：只是"打开个目录"的附属动作 */}
        <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.data.folder')}
              <Tip text={tr('settings.data.folderNote')} />
            </div>
            {/* 这一句是**运行结果/错误**，不是说明 —— 留在版面上（`.current` 是常显那档） */}
            {folderNote !== null && <p className={s.current}>{say(folderNote)}</p>}
          </div>
          <Btn disabled={folderBusy} onClick={openFolder}>
            {folderBusy ? tr('settings.data.opening') : tr('settings.data.open')}
          </Btn>
        </div>

        <p className={s.noteHint}>{tr('settings.data.privacy')}</p>
      </section>

      {/* 「最近删除」是独立一章（产品里它自己一个 section）：套一层带 id 的壳，
          左目录才跳得到、搜索才筛得到 */}
      <div id="recovery" data-sec>
        <RecoverySection host={host} />
      </div>

      <section className={`${s.section} ${s.dangerBlock}`} aria-label={tr('settings.data.uninstallTitle')}>
        <h2 className={`${s.secTitle} ${s.dangerTitle}`}>{tr('settings.data.uninstallTitle')}</h2>

        <p className={s.dangerNote}>
          <span className={s.dangerNoteHead}>{tr('common.danger')}</span>
          {tr('settings.data.uninstallRiskPre')}
          <strong>{tr('settings.data.uninstallRiskKeep')}</strong>
          {tr('settings.data.uninstallRiskPost')}
        </p>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={`${s.fieldTitle} ${s.dangerTitle}`}>
              {tr('settings.data.uninstall')}
              <Tip text={tr('settings.data.uninstallNote')} />
            </div>
          </div>
          <Btn variant="danger" disabled={uninstallBusy} onClick={doUninstall}>
            {uninstallBusy
              ? tr('settings.data.uninstalling')
              : armed
                ? tr('settings.data.uninstallConfirm')
                : tr('settings.data.uninstallAction')}
          </Btn>
        </div>

        {uninstallNote !== null && <p className={`${s.note} ${s.noteBad}`}>{say(uninstallNote)}</p>}
      </section>
    </>
  )
}
