/*
 * 设置页「数据与维护 / 清理与卸载」那两块。
 *
 * 这一份做三件（外加一行打开目录）：
 *   1. 「预设位置」= 打开 `presets-local/` 那个目录（**只是打开文件夹**，不读不写内容）
 *   2. 「清理数据」= 删掉本机的数据与偏好，**程序本体留着**（清完退出程序）
 *   3. 「卸载」= 删程序本体；连不连数据一起删，在卸载那一扇框里用开关点名
 *
 * 两条都是不可逆动作 ⇒ 版面上只有一颗按钮（清理 / 卸载），**真正动手在模态框里**
 * 那颗红的确认键上。「卸载时一并删除数据」那颗开关也住在卸载那一扇框里 ——
 * 它只对那一次卸载有效，摆在版面上会被读成一项常驻设置。
 *
 * 清完 / 卸完后端都会 `exit` —— 这一屏看不到"成功"，只有失败才会回到这里，
 * 所以失败那句住在框里（框还开着，人可以重试）。
 *
 * ★ 文案走字典。风险那一段里"您的数据会保留"是加粗的中间一段（三个键拼起来），
 *   它跟着框里那颗开关换档：界面上的字与后端那两个分支必须一一对上
 *   （关 = 只删本体 / 开 = 先清后删）。
 */

import { useState } from 'react'

import { msg, useText, type Text } from '../../i18n'

import { api, errorText } from '../../api'
import { clearClientPrefs } from '../../api/storageKeys'
import { useT } from '../../i18n'
import { Btn } from '../ui/Controls'
import Modal from '../../components/modal/Modal'
import { Tip } from '../../components/tooltip'
import { Switch } from '../../components/switch'
import RecoverySection from './RecoverySection'
import s from './PageSettings.module.css'

/** `host`：两扇框与「最近删除」共用（遮罩铺满标题栏以下那一块），原样传下去 */
export default function DataSection({ host }: { host: HTMLElement | null }) {
  const tr = useT()
  const say = useText()
  const [folderBusy, setFolderBusy] = useState(false)
  /** 成功时是**落点路径原样**（不翻），失败时是存键的那句话 —— 两种都住 [`Text`] */
  const [folderNote, setFolderNote] = useState<Text | null>(null)

  /* 清理数据：框开着 = 人已经在确认这一步上 */
  const [clearOpen, setClearOpen] = useState(false)
  const [clearBusy, setClearBusy] = useState(false)
  const [clearNote, setClearNote] = useState<Text | null>(null)

  /* 卸载：同一套；`cleanData` 那颗开关在框里，决定这一次删不删数据 */
  const [uninstallOpen, setUninstallOpen] = useState(false)
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

  const openClear = () => {
    setClearNote(null)
    setClearOpen(true)
  }

  /** 跑着的时候不许收：那一刻只有"等它删完 / 退出"这一条路 */
  const closeClear = () => {
    if (clearBusy) return
    setClearOpen(false)
    setClearNote(null)
  }

  const openUninstall = () => {
    setUninstallNote(null)
    setUninstallOpen(true)
  }

  const closeUninstall = () => {
    if (uninstallBusy) return
    setUninstallOpen(false)
    setUninstallNote(null)
    /* 开关只对刚那一次成立：关框就回到"只删本体"那一档 */
    setCleanData(false)
  }

  const doClear = () => {
    if (clearBusy) return
    setClearBusy(true)
    setClearNote(null)
    /*
     * 先把 WebView 自己那些偏好抹掉再叫后端：
     * 后端删完会 `exit`，那个 `.then` 基本走不到。
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
    if (uninstallBusy) return
    setUninstallBusy(true)
    setUninstallNote(null)
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

        {/* 版面这一句只说"这里有哪两件事、后果收在框里"—— 逐条后果进各自的框 */}
        <p className={s.dangerNote}>
          <span className={s.dangerNoteHead}>{tr('common.danger')}</span>
          {tr('settings.data.dangerNote')}
        </p>

        {/* ① 只清数据：程序本体留着 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={`${s.fieldTitle} ${s.dangerTitle}`}>
              {tr('settings.data.clear')}
              <Tip text={tr('settings.data.clearNote')} />
            </div>
          </div>
          <Btn variant="danger" onClick={openClear}>
            {tr('settings.data.clearAction')}
          </Btn>
        </div>

        {/* ② 卸载本体 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={`${s.fieldTitle} ${s.dangerTitle}`}>
              {tr('settings.data.uninstall')}
              <Tip text={tr('settings.data.uninstallNote')} />
            </div>
          </div>
          <Btn variant="danger" onClick={openUninstall}>
            {tr('settings.data.uninstallAction')}
          </Btn>
        </div>
      </section>

      {/* 清理数据那一扇：框本身就是那一下确认 */}
      <Modal
        open={clearOpen}
        title={tr('settings.data.clear')}
        size="md"
        host={host}
        hideClose={clearBusy}
        closeOnEsc={!clearBusy}
        onClose={closeClear}
        footer={
          <>
            <Btn disabled={clearBusy} onClick={closeClear}>
              {tr('common.cancel')}
            </Btn>
            <Btn variant="danger" disabled={clearBusy} onClick={doClear}>
              {clearBusy ? tr('settings.data.clearing') : tr('settings.data.clearConfirm')}
            </Btn>
          </>
        }
      >
        <div className={s.dangerBody}>
          <p className={s.dangerNote}>
            <span className={s.dangerNoteHead}>{tr('common.danger')}</span>
            {tr('settings.data.clearRisk')}
          </p>
          <p className={s.noteHint}>{tr('settings.data.clearNote')}</p>
          {clearNote !== null && <p className={`${s.note} ${s.noteBad}`}>{say(clearNote)}</p>}
        </div>
      </Modal>

      {/* 卸载那一扇：「一并删数据」那颗开关在框里，与上面那段风险句联动 */}
      <Modal
        open={uninstallOpen}
        title={tr('settings.data.uninstall')}
        size="md"
        host={host}
        hideClose={uninstallBusy}
        closeOnEsc={!uninstallBusy}
        onClose={closeUninstall}
        footer={
          <>
            <Btn disabled={uninstallBusy} onClick={closeUninstall}>
              {tr('common.cancel')}
            </Btn>
            <Btn variant="danger" disabled={uninstallBusy} onClick={doUninstall}>
              {uninstallBusy ? tr('settings.data.uninstalling') : tr('settings.data.uninstallConfirm')}
            </Btn>
          </>
        }
      >
        <div className={s.dangerBody}>
          <p className={s.dangerNote}>
            <span className={s.dangerNoteHead}>{tr('common.danger')}</span>
            {tr('settings.data.uninstallRiskPre')}
            <strong>
              {cleanData ? tr('settings.data.uninstallRiskWipe') : tr('settings.data.uninstallRiskKeep')}
            </strong>
            {cleanData
              ? tr('settings.data.uninstallRiskWipePost')
              : tr('settings.data.uninstallRiskPost')}
          </p>

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
              disabled={uninstallBusy}
              onChange={setCleanData}
            />
          </div>

          <p className={s.noteHint}>{tr('settings.data.uninstallNote')}</p>
          {uninstallNote !== null && <p className={`${s.note} ${s.noteBad}`}>{say(uninstallNote)}</p>}
        </div>
      </Modal>
    </>
  )
}
