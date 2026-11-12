/*
 * 「备份与恢复」那一框（2026-11-09）。
 *
 * 作者的口径一路收窄到这里，三刀：
 *   1. 「做模态框出来」—— 挑位置、看进度、看结果都进框，设置页不再显示结果；
 *   2. 「备份的时候会卡在这，做成异步的，读那个进度」—— 命令进线程池 + Channel 水位；
 *   3. 「备份和还原合并成同一个按钮，可选手动带上预设；去掉多余空行，文字极简」——
 *      一扇框、两个分页（`.seg`），备份那一页多一颗「连同预设一起备份」。
 *
 * # 外壳与语义照报告页的导出框（`app/report/ExportModal.tsx`）
 *
 * 同一个底座 `components/modal/Modal`，同一批约定：
 *
 * - **失败留在框里、不关框**：后端那句原话走共用件 `HeadNote`（标题右边那一格）——
 *   框身按内容撑高，往里面加一行红字会把下面的东西顶走；
 * - **成功也不关框**：「打开位置」翻在路径胶囊里，框留着让人去看一眼；
 * - **只有真在动 / 真成了才有进度那一块**（2026-11-09 第二刀）：不再拿空白行占位 ——
 *   作者指着恢复框里那条空进度说「多余的空行，去掉」。于是闲置时这一框只到胶囊为止。
 *
 * # 进度怎么读（`onTick`）
 *
 * `scan` = 清点 / 准备（`total = 0`，**不画百分比**）；`pack` = 真在搬
 * （备份按**字节**、恢复按**文件条数**）。单位由后端定，这一层只做除法。
 */

import { useEffect, useState } from 'react'
/*
 * 系统文件选择框：备份挑文件夹、恢复挑那份 zip —— 位置一律由用户点名。
 *
 * ★ 改名成 `openDialog` 不是因为口味：这一件的 props 里就有一个 `open`（这一扇开没开），
 *   同名会把系统框那颗函数遮在参数里 —— 界面上点「更改位置」时调的是一个布尔值。
 */
import { open as openDialog } from '@tauri-apps/plugin-dialog'

import { api, errorText, type BackupSummary, type BackupTick, type RestoreScope } from '../../api'
/* 自绘下拉共用件（#33）：项目里的下拉一律用它，原生 `<select>` 一个不留 */
import { SelectField } from '../../components/field'
import type { FieldOption } from '../../components/field'
import { HeadNote, Modal, PageToggle } from '../../components/modal'
/* 设置页那套圆钮开关：布尔项一律用它（与这一页其余几块同一条） */
import { Switch } from '../../components/switch'
import { msg, useT, type Text } from '../../i18n'
import { Btn } from '../ui/Controls'
import s from './BackupModal.module.css'

/** 恢复成功之后等多久再整窗重载（让"恢复完了"那句话来得及被看见） */
const RELOAD_MS = 900

/** 这一扇里的两页：备份 / 恢复 */
type Mode = 'backup' | 'restore'

/**
 * 字节数写成人话（`5.2 MB`）。
 *
 * 备份能到几百 MB（执行归档三件套），所以到 GB 也往上走 —— 一位小数够了。
 */
function sizeText(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

/**
 * 路径拆成"头 + 尾"两段 —— 中间省略用（与导出框那颗胶囊同一个读法）：
 * 头自己收省略号、尾两段钉住不缩，"往哪个文件夹里落"永远看得见。
 */
function splitPath(full: string): { head: string; tail: string } {
  const sep = full.includes('\\') ? '\\' : '/'
  const lead = full.startsWith(sep) ? sep : ''
  const parts = full.split(/[\\/]/).filter((x) => x !== '')
  if (parts.length <= 2) return { head: '', tail: full }
  return { head: lead + parts.slice(0, -2).join(sep) + sep, tail: parts.slice(-2).join(sep) }
}

/** 文件夹那枚轮廓（与全站图标同一网格：24×24、currentColor） */
const FOLDER_SHAPE = (
  <path d="M3.5 6.5A2 2 0 0 1 5.5 4.5h4l2 2.5h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
)

export interface BackupModalProps {
  open: boolean
  /**
   * 遮罩挂在哪（设置页交的是这一页的根 `pageEl`）——
   * 与这一页另外两扇框（清理数据 / 卸载）同一个做法。
   */
  host?: HTMLElement | null
  onClose: () => void
}

export default function BackupModal({ open, host = null, onClose }: BackupModalProps) {
  const tr = useT()

  const [mode, setMode] = useState<Mode>('backup')
  /** 备份落点（文件夹）。`null` = 还在问后端默认的桌面位置 */
  const [dir, setDir] = useState<string | null>(null)
  /** 「连同预设一起备份」 */
  const [includePresets, setIncludePresets] = useState(true)
  /** 恢复的那份 zip。`null` = 还没挑 */
  const [zip, setZip] = useState<string | null>(null)
  /** 恢复范围（`all` / `presets` / `settings`）—— 只动选中的那一块 */
  const [scope, setScope] = useState<RestoreScope>('all')
  const [busy, setBusy] = useState(false)
  /** 最近一跳（见文件头「进度怎么读」） */
  const [tick, setTick] = useState<BackupTick | null>(null)
  /** 后端原话走标题右边那一格 —— 只放失败 */
  const [fail, setFail] = useState<Text | null>(null)
  /** 成了的那一份回执（路径 / 份数 / 字节） */
  const [done, setDone] = useState<BackupSummary | null>(null)

  /* 开 / 关都把内务清干净：下一趟的第一帧不许带着上一次的结果或一条红字 */
  useEffect(() => {
    setMode('backup')
    setDir(null)
    setIncludePresets(true)
    setZip(null)
    setScope('all')
    setBusy(false)
    setTick(null)
    setFail(null)
    setDone(null)
  }, [open])

  /*
   * 默认落点按需问（只有真进「备份」那一页才问）：恢复那一页不需要桌面，
   * 也就不该为了打开一扇框去碰一下文件系统。
   */
  useEffect(() => {
    if (!open || mode !== 'backup' || dir !== null) return
    let alive = true
    api.defaultBackupDir().then(
      (got) => {
        if (alive) setDir(got)
      },
      (e: unknown) => {
        if (alive) setFail(msg('settings.backupModal.dirFailed', { detail: errorText(e) }))
      },
    )
    return () => {
      alive = false
    }
  }, [open, mode, dir])

  /** 换页：跑着的时候不许换；换过去就把上一页的回执清掉（那是上一页的事） */
  const switchMode = (next: Mode) => {
    if (busy || next === mode) return
    setMode(next)
    setTick(null)
    setFail(null)
    setDone(null)
  }

  /** 备份的「更改位置」：系统目录选择器。取消（`null`）保持原来那个位置，不是错误 */
  const chooseDir = () => {
    if (busy) return
    setFail(null)
    void openDialog({
      directory: true,
      title: tr('settings.backup.pickDir'),
      defaultPath: dir ?? undefined,
    }).then(
      (picked) => {
        const at = Array.isArray(picked) ? picked[0] : picked
        if (typeof at === 'string' && at !== '') setDir(at)
      },
      (e: unknown) => {
        setFail(msg('settings.backupModal.dirFailed', { detail: errorText(e) }))
      },
    )
  }

  /** 恢复的「选择备份文件 / 更换文件」：取消不算错，保持原来那份 */
  const chooseZip = () => {
    if (busy) return
    setFail(null)
    void openDialog({
      directory: false,
      multiple: false,
      title: tr('settings.restore.pickTitle'),
      filters: [{ name: tr('settings.restore.pickFilter'), extensions: ['zip'] }],
    }).then(
      (picked) => {
        const at = Array.isArray(picked) ? picked[0] : picked
        if (typeof at === 'string' && at !== '') setZip(at)
      },
      (e: unknown) => {
        setFail(msg('settings.restoreModal.pickFailed', { detail: errorText(e) }))
      },
    )
  }

  const startBackup = () => {
    if (busy || dir === null) return
    setBusy(true)
    setFail(null)
    setDone(null)
    setTick(null)
    api.createBackup(dir, includePresets, (t) => setTick(t)).then(
      (sum) => {
        setBusy(false)
        setDone(sum)
      },
      (e: unknown) => {
        setBusy(false)
        setTick(null)
        setFail(msg('settings.backupModal.failed', { detail: errorText(e) }))
      },
    )
  }

  const confirmRestore = () => {
    if (busy || zip === null) return
    setBusy(true)
    setFail(null)
    setDone(null)
    setTick(null)
    api.restoreBackup(zip, scope, (t) => setTick(t)).then(
      (sum) => {
        setBusy(false)
        setDone(sum)
        /* 盘上的数据换过了，内存里的都不算数 ⇒ 整窗重载 */
        window.setTimeout(() => window.location.reload(), RELOAD_MS)
      },
      (e: unknown) => {
        setBusy(false)
        setTick(null)
        setFail(msg('settings.restoreModal.failed', { detail: errorText(e) }))
      },
    )
  }

  /** 「打开位置」：在系统文件管理器里选中那份 zip（后端只认自家给过的那串路径） */
  const reveal = () => {
    const at = done?.path
    if (at === undefined) return
    setFail(null)
    api.revealBackup(at).catch((e: unknown) => {
      setFail(msg('settings.backupModal.revealFailed', { detail: errorText(e) }))
    })
  }

  const backup = mode === 'backup'

  /*
   * 恢复范围三档（下拉里的选项，带一行小字说明）—— 只影响"动不动那一块"：
   * 选中范围内仍是整棵换掉，范围外一个都不碰。
   */
  const scopeOptions: FieldOption[] = [
    {
      value: 'all',
      label: tr('settings.backupModal.scopeAll'),
      note: tr('settings.backupModal.scopeAllNote'),
    },
    {
      value: 'presets',
      label: tr('settings.backupModal.scopePresets'),
      note: tr('settings.backupModal.scopePresetsNote'),
    },
    {
      value: 'settings',
      label: tr('settings.backupModal.scopeSettings'),
      note: tr('settings.backupModal.scopeSettingsNote'),
    },
  ]

  /*
   * 百分比：只认后端给的 `done / total`。清点阶段 `total = 0` ⇒ 不画条（"还不知道"，
   * 不是 0%）；成了之后不再画条（那时说的是"这件事完了"）。
   */
  const pct =
    tick !== null && tick.total > 0
      ? Math.min(100, Math.round((tick.done / tick.total) * 100))
      : null

  const stageLine =
    done !== null
      ? backup
        ? tr('settings.backupModal.done', {
            files: done.files,
            source: sizeText(done.sourceBytes),
            size: sizeText(done.bytes),
          })
        : tr('settings.restoreModal.done')
      : tick?.stage === 'pack'
        ? `${tr(backup ? 'settings.backupModal.pack' : 'settings.restoreModal.pack')} · ${pct ?? 0}%`
        : tr(backup ? 'settings.backupModal.scan' : 'settings.restoreModal.scan')

  /** 路径胶囊里那一串：备份 = 落点文件夹（打完后是那个 zip）；恢复 = 挑好的那份 */
  const shownPath = backup ? (done !== null ? done.path : (dir ?? '')) : (zip ?? '')
  const cut = splitPath(shownPath)

  /** 胶囊右边那一颗：备份打完后翻成「打开位置」；恢复是「更换文件」 */
  const pillAct = backup ? (
    <button
      type="button"
      className={s.pillAct}
      data-open={done !== null}
      disabled={busy}
      onClick={done !== null ? reveal : chooseDir}
    >
      {done !== null ? tr('settings.backupModal.open') : tr('settings.backupModal.change')}
    </button>
  ) : (
    <button
      type="button"
      className={s.pillAct}
      disabled={busy || done !== null}
      onClick={chooseZip}
    >
      {tr('settings.restoreModal.change')}
    </button>
  )

  /*
   * 进度那一块：**有才出现**（跑着 / 成了），不再拿空白行占位。
   * 条只在"知道分母、且真在搬"时画；当前文件那一行只在跑着时画。
   */
  const progress =
    busy || done !== null ? (
      <div className={s.progress}>
        <p className={s.stage} data-done={done !== null}>
          {stageLine}
        </p>
        {busy && pct !== null && (
          <div
            className={s.bar}
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={pct}
          >
            <span className={s.fill} style={{ width: `${pct}%` }} />
          </div>
        )}
        {busy && tick !== null && tick.file !== '' && (
          <p className={s.detail} data-tip={tick.file}>
            {tick.file}
          </p>
        )}
      </div>
    ) : null

  return (
    <Modal
      open={open}
      size="md"
      host={host}
      title={tr('settings.backupModal.title')}
      onClose={onClose}
      /* 跑着的时候不许收（正在跑的框"收起来"必须是一条明说的动作，不能是一记 Esc） */
      closeOnEsc={!busy}
      hideClose={busy}
      footer={
        done !== null ? (
          <Btn variant="accent" onClick={onClose}>
            {tr('settings.backupModal.finish')}
          </Btn>
        ) : (
          <>
            {/*
             * 左下角那颗**翻页小胶囊**（共用件 `PageToggle`，与导出框 / 后处理同一处位置）：
             * 钮面写"点它去哪一页"（备份页写「恢复」，反之写「备份」）；跑着的时候不摆 ——
             * 半路换页会让下面那块与正在跑的活对不上。
             */}
            {!busy && (
              <span className={s.footLeft}>
                <PageToggle
                  label={tr(
                    backup ? 'settings.backupModal.tabRestore' : 'settings.backupModal.tabBackup',
                  )}
                  ariaLabel={tr('settings.backupModal.switchTo', {
                    page: tr(
                      backup ? 'settings.backupModal.tabRestore' : 'settings.backupModal.tabBackup',
                    ),
                  })}
                  onClick={() => switchMode(backup ? 'restore' : 'backup')}
                />
              </span>
            )}
            <Btn onClick={onClose} disabled={busy}>
              {tr(backup ? 'settings.backupModal.cancel' : 'settings.restoreModal.cancel')}
            </Btn>
            {backup ? (
              <Btn
                variant="accent"
                onClick={startBackup}
                disabled={busy || dir === null}
              >
                {busy ? tr('settings.backupModal.busy') : tr('settings.backupModal.start')}
              </Btn>
            ) : (
              <Btn
                variant="danger"
                onClick={confirmRestore}
                disabled={busy || zip === null}
              >
                {busy ? tr('settings.restoreModal.busy') : tr('settings.restoreModal.confirm')}
              </Btn>
            )}
          </>
        )
      }
    >
      <div className={s.body}>
        {fail !== null && <HeadNote text={fail} tone="error" />}

        {/*
         * 「连同预设一起备份」—— 位置与形制照报告页导出框的那条（`zipRow`）：
         * **带标题的选项在上、路径在下**（作者 2026-11-09：「这种开关呀，有标题什么的
         * 应该放在上面，像那个导出一样。这个路径是在下面的」）。
         * 左边两行（题 + 一句小字），右边一颗共用件开关。
         */}
        {backup && (
          <div className={s.switchRow}>
            <span className={s.switchText}>
              <span className={s.switchTitle}>{tr('settings.backupModal.includePresets')}</span>
              <span className={s.switchHint}>
                {tr('settings.backupModal.includePresetsHint')}
              </span>
            </span>
            <Switch
              checked={includePresets}
              onChange={setIncludePresets}
              disabled={busy || done !== null}
              ariaLabel={tr('settings.backupModal.includePresets')}
            />
          </div>
        )}

        {/*
         * 恢复页那一行：与备份页的开关行**同一副骨架**（左两行字、右控件），
         * 控件是共用下拉（三档）。两页因此**等高**（行高由文字块撑着，下拉 34px 更矮）。
         *
         * ★ 第二行就是那句红字（「只动选中的那一部分，覆盖后无法撤销」）——
         *   原来是摆在文件行下面独立一条，但那样恢复页会比备份页高出一行；
         *   作者这轮的硬要求就是"两页等高"，所以把它并进这一行的说明位（内容一字没少）。
         */}
        {!backup && (
          <div className={s.switchRow}>
            <span className={s.switchText}>
              <span className={s.switchTitle}>{tr('settings.backupModal.scope')}</span>
              <span className={`${s.switchHint} ${s.hintDanger}`}>
                {tr('settings.backupModal.scopeHint')}
              </span>
            </span>
            <span className={s.scopePick}>
              <SelectField
                label={tr('settings.backupModal.scope')}
                options={scopeOptions}
                value={scope}
                disabled={busy || done !== null}
                onChange={(v) => setScope(v as RestoreScope)}
              />
            </span>
          </div>
        )}

        {backup || zip !== null ? (
          /* 路径胶囊：整条是**显示**，只有右边那颗按钮能点（路径正文还能拖选） */
          <div className={s.pill} data-tip={shownPath === '' ? undefined : shownPath}>
            <svg
              className={s.pillIcon}
              width={15}
              height={15}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.7}
              aria-hidden="true"
            >
              {FOLDER_SHAPE}
            </svg>
            <span className={s.pillText}>
              <span className={s.pillHead} data-tip={shownPath === '' ? undefined : shownPath}>
                {shownPath === '' ? tr('settings.backupModal.dirReading') : cut.head}
              </span>
              <span className={s.pillTail}>{cut.tail}</span>
            </span>
            {pillAct}
          </div>
        ) : (
          /* 恢复那一页还没挑文件：整条是"去挑"的入口 */
          <button type="button" className={s.pickRow} onClick={chooseZip} disabled={busy}>
            <svg
              className={s.pillIcon}
              width={15}
              height={15}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.7}
              aria-hidden="true"
            >
              {FOLDER_SHAPE}
            </svg>
            <span className={s.pickText}>{tr('settings.restoreModal.pick')}</span>
          </button>
        )}

        {progress}
      </div>
    </Modal>
  )
}
