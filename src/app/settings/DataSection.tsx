/*
 * 设置页「数据与维护 / 清理与卸载」那两块（2026-10-17 开，2026-11-01 加清理数据）。
 *
 * 作者的范围裁断（逐字）：
 *   「那个备份恢复就不用做了，清理卸载呀，那一些可以做，我们已经有自动清理了，
 *     那就应该只做卸载。预设位置其实也就是打开位置而已，打开我们那个程序那些文件的位置」
 *   —— 备份恢复已有（页面上面那一块），自动清理已有（执行记录那一格）。
 *
 * 2026-11-01 作者补上另一半：「还有不只是加一个卸载的，还有一个清理数据的，
 * 就是只清理不卸载，或者卸载的时候一起清理」。于是这一份做三件（外加一行打开目录）：
 *   1. 「预设位置」= 打开 `presets-local/` 那个目录（**只是打开文件夹**，不读不写内容）
 *   2. 「清理数据」= 删掉本机的数据与偏好，**程序本体留着**（清完退出程序）
 *   3. 「卸载」= 删程序本体；旁边那颗开关点名时才连数据一起删
 *
 * 两条都是不可逆动作，都走**两段式确认**（3 秒无操作就回退，与「清空历史」同一套）。
 * 清完 / 卸完后端都会 `exit` —— 这一屏看不到"成功"，只有失败才会回到这里。
 *
 * ★ 文案走字典（2026-10-25）。风险那一段里"您的数据会保留"原本是 `<strong>`，
 *   现在拆成前 / 中 / 后三个键，中间那段仍包在 `<strong>` 里（英文也一样加粗）。
 */

import { useEffect, useState } from 'react'

import { msg, useText, type Text } from '../../i18n'

import { api, errorText } from '../../api'
import { clearClientPrefs } from '../../api/storageKeys'
import { useT } from '../../i18n'
import { Btn } from '../ui/Controls'
import { Tip } from '../../components/tooltip'
import { Switch } from '../../components/switch'
import RecoverySection from './RecoverySection'
import s from './PageSettings.module.css'

/** 两段式确认的窗口：与「清空历史」那一套同一个时长（3 秒无操作就回退） */
const CONFIRM_MS = 3000

/**
 * 一颗两段式确认的按钮，管自己那 3 秒。
 *
 * 抽出来是因为这一块现在有**两颗**危险按钮（清理 / 卸载）：各写一份的话，
 * 「3 秒回退」这条行为就有两个实现，将来只改一处就又出一对不一致的按钮。
 */
function useArmed(): [boolean, () => boolean, () => void] {
  const [armed, setArmed] = useState(false)
  useEffect(() => {
    if (!armed) return
    const t = window.setTimeout(() => setArmed(false), CONFIRM_MS)
    return () => window.clearTimeout(t)
  }, [armed])
  /** 点一下：没举起就举起（返回 false = 还差一下）；已经举起就放下并放行（返回 true） */
  const hit = (): boolean => {
    if (!armed) {
      setArmed(true)
      return false
    }
    setArmed(false)
    return true
  }
  return [armed, hit, () => setArmed(false)]
}

/** `host`：只有「最近删除」那个模态框要（遮罩铺满标题栏以下那一块），原样传下去 */
export default function DataSection({ host }: { host: HTMLElement | null }) {
  const tr = useT()
  const say = useText()
  const [folderBusy, setFolderBusy] = useState(false)
  /** 成功时是**落点路径原样**（不翻），失败时是存键的那句话 —— 两种都住 [`Text`] */
  const [folderNote, setFolderNote] = useState<Text | null>(null)

  /* 清理数据：点了第一下变成「确认清理」，3 秒没人再点就自己退回 */
  const [clearArmed, clearHit, clearDisarm] = useArmed()
  const [clearBusy, setClearBusy] = useState(false)
  const [clearNote, setClearNote] = useState<Text | null>(null)

  /* 卸载：同一套两段式；`cleanData` 那颗开关决定删不删数据 */
  const [armed, uninstallHit, uninstallDisarm] = useArmed()
  const [uninstallBusy, setUninstallBusy] = useState(false)
  const [uninstallNote, setUninstallNote] = useState<Text | null>(null)
  const [cleanData, setCleanData] = useState(false)

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

  const doClear = () => {
    if (!clearHit()) {
      setClearNote(null)
      return
    }
    setClearBusy(true)
    /*
     * 先把 WebView 自己那些偏好抹掉再叫后端：
     * 后端删完会 `exit`，那个 `.then` 基本走不到（见文件头）。
     */
    clearClientPrefs()
    void api.clearAppData().then(
      /* 成功时后端已经 exit 了，这一行基本走不到；走到说明进程还活着 */
      () => setClearBusy(false),
      (e: unknown) => {
        setClearBusy(false)
        setClearNote(msg('settings.data.clearFailed', { detail: errorText(e) }))
      },
    )
  }

  const doUninstall = () => {
    if (!uninstallHit()) {
      setUninstallNote(null)
      return
    }
    setUninstallBusy(true)
    /* 与「清理数据」同一条顺序：先抹 WebView 的偏好，再交给后端（后端删完会 exit） */
    if (cleanData) clearClientPrefs()
    void api.uninstall(cleanData).then(
      /* 成功时后端已经 exit 了，这一行基本走不到；走到说明进程还活着 */
      () => setUninstallBusy(false),
      (e: unknown) => {
        setUninstallBusy(false)
        setUninstallNote(msg('settings.data.uninstallFailed', { detail: errorText(e) }))
      },
    )
  }

  /* 关掉"一键删数据"时把两颗按钮都收回原位：状态留在那儿会让下一次点得莫名其妙 */
  const pickCleanData = (next: boolean) => {
    setCleanData(next)
    clearDisarm()
    uninstallDisarm()
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
          {/*
           * 这句跟着下面那颗「卸载时一并删除数据」走：关着说"数据会保留"，
           * 开着就**不能再说保留** —— 界面上的字与后端那两个分支必须一一对上
           * （关 = 只删本体 / 开 = 先清后删）。
           */}
          {tr('settings.data.uninstallRiskPre')}
          <strong>
            {cleanData ? tr('settings.data.uninstallRiskWipe') : tr('settings.data.uninstallRiskKeep')}
          </strong>
          {cleanData
            ? tr('settings.data.uninstallRiskWipePost')
            : tr('settings.data.uninstallRiskPost')}
          {tr('settings.data.clearRisk')}
        </p>

        {/* ① 只清数据：程序本体留着 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={`${s.fieldTitle} ${s.dangerTitle}`}>
              {tr('settings.data.clear')}
              <Tip text={tr('settings.data.clearNote')} />
            </div>
          </div>
          <Btn variant="danger" disabled={clearBusy} onClick={doClear}>
            {clearBusy
              ? tr('settings.data.clearing')
              : clearArmed
                ? tr('settings.data.clearConfirm')
                : tr('settings.data.clearAction')}
          </Btn>
        </div>

        {clearNote !== null && <p className={`${s.note} ${s.noteBad}`}>{say(clearNote)}</p>}

        {/* ② 卸载时删不删数据 —— 紧挨着卸载那一行，它只对那一下有效 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.data.cleanOnUninstall')}
              <Tip text={tr('settings.data.cleanOnUninstallNote')} />
            </div>
          </div>
          <Switch
            checked={cleanData}
            ariaLabel={tr('settings.data.cleanOnUninstall')}
            onChange={pickCleanData}
          />
        </div>

        {/* ③ 卸载本体 */}
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
