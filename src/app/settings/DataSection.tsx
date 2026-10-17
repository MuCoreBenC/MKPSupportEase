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
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import { Btn } from '../ui/Controls'
import s from './PageSettings.module.css'

/** 两段式确认的窗口：与「清空历史」那一套同一个时长（3 秒无操作就回退） */
const CONFIRM_MS = 3000

export default function DataSection() {
  const [folderBusy, setFolderBusy] = useState(false)
  const [folderNote, setFolderNote] = useState<string | null>(null)

  /* 卸载：点了第一下变成「确认卸载」，3 秒没人再点就自己退回 */
  const [armed, setArmed] = useState(false)
  const [uninstallBusy, setUninstallBusy] = useState(false)
  const [uninstallNote, setUninstallNote] = useState<string | null>(null)

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
        setFolderNote(`打不开：${errorText(e)}`)
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
        setUninstallNote(`没卸成：${errorText(e)}`)
      },
    )
  }

  return (
    <>
      <section className={s.section} aria-label="数据与维护">
        <h2 className={s.secTitle}>数据与维护</h2>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>预设位置</div>
            <p className={s.fieldNote}>打开预设配置文件所在目录</p>
            {folderNote !== null && <p className={s.fieldNote}>{folderNote}</p>}
          </div>
          <Btn disabled={folderBusy} onClick={openFolder}>
            {folderBusy ? '正在打开…' : '打开'}
          </Btn>
        </div>

        <p className={s.noteHint}>
          MKP SupportEase 不会读取、保存、导出或修改您的 3MF 文件
        </p>
      </section>

      <section className={`${s.section} ${s.dangerBlock}`} aria-label="清理与卸载">
        <h2 className={`${s.secTitle} ${s.dangerTitle}`}>清理与卸载</h2>

        <p className={s.dangerNote}>
          <span className={s.dangerNoteHead}>危险操作</span>
          「卸载应用」会退出程序并删除程序本体，<strong>您的数据会保留</strong> ——
          预设、执行记录、备份都留在原来的位置，重装回来还在。
        </p>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={`${s.fieldTitle} ${s.dangerTitle}`}>卸载应用</div>
            <p className={s.fieldNote}>
              删掉程序本体之后就打不开了。切片器里那条后处理命令也要自己清掉，
              否则下次切片会报"脚本不存在"。
            </p>
          </div>
          <Btn variant="danger" disabled={uninstallBusy} onClick={doUninstall}>
            {uninstallBusy ? '正在卸载…' : armed ? '确认卸载' : '卸载'}
          </Btn>
        </div>

        {uninstallNote !== null && <p className={`${s.note} ${s.noteBad}`}>{uninstallNote}</p>}
      </section>
    </>
  )
}
