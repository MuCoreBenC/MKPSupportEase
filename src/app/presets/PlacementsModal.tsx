/*
 * 「查看放入情况」那个框（2026-11-10 · 作者：「右键里去查看，弹出一个模态框……
 * 就可以看到这个预设存在的文件夹有哪一些……就用徽章显示已放入 / 未放入」）。
 *
 * # 它答的是什么
 *
 * 一份切片器预设要**躺在切片器自己的账号目录里**才生效（`user/<账号>/process`），
 * 而一台机器上可以有好几个账号目录（数字 uid = 登录过的账号、`default` = 没登录时用的
 * 那一套、还有 BBL 这种）。一次「放入」写的是**每一个**，但这个框要让人**看得见**：
 * 哪几个里有、哪几个里没有。在系统文件管理器里删掉其中一份，这里就变回「未放入」——
 * 读的是盘，不是记忆。
 *
 * # 为什么按切片器分组
 *
 * 落点不止一族（2026-11-10 作者：「不只是 BBS，到时候 Orca 的也要放入……让我选择
 * BBS 还是哪些，让我自己看着去放入」）。后端把**这份预设自己那一档排在最前**，
 * 本机装了的另一个切片器跟在后面；别的切片器那几行带一句说明（字段表是另一套），
 * 放不放由用户自己按 —— 这一侧不替他决定，也不假装它一定读得出来。
 *
 * # 什么时候重读
 *
 * **不是轮询**：打开时读一次、放入之后读一次、窗口重新拿到焦点 / 从后台切回来再读一次
 * （与表格那一行用的是同一个事件）。用户去访达删掉一份再回来，这一格就得跟着变。
 *
 * # 为什么不做「已删除」
 *
 * 真值与"从没放过"在盘上长得一模一样，分出来要另记一本账（谁在哪天放过哪一份）。
 * 本仓的口径是「盘就是底账」，所以只有两态：`placed` 与 `!placed`。少一处就是没放全。
 *
 * # 分工
 *
 * 这个框只摆界面：数据与动作都由调用方给（`load` / `put` / `reveal`），
 * 与 `report/ExportModal` 同一个分工 —— 提示条、busy、表格那一行的状态都留在页面层。
 * 框里的失败走共用件 `HeadNote`（标题右边那一格），不往框身里塞带底色的段落。
 */
import { useCallback, useEffect, useState } from 'react'

import type {
  CopyToSlicerOutcome,
  SlicerPlacement,
  SlicerPlacements,
  SlicerTarget,
} from '../../api/contract'
import Icon from '../../components/Icon'
import { HeadNote, Modal } from '../../components/modal'
import { useT, type StringKey } from '../../i18n'
import { Btn } from '../ui/Controls'
import s from './PlacementsModal.module.css'

/** 分组标题用产品名（与设置页那个下拉同一份词，见 `shared/slicer.ts` 的 `SLICER_CHOICES`） */
const SLICER_LABEL: Record<SlicerTarget, StringKey> = {
  bambu: 'slicer.choices.bambu.label',
  orca: 'slicer.choices.orca.label',
}

interface PlacementsModalProps {
  open: boolean
  /** 问的是哪一份（后端只认文件名，路径由它按目录登记拼） */
  fileName: string
  /** 遮罩挂在哪一层（与页里其它框一样，盖住内容区而不是整窗） */
  host: HTMLElement | null
  onClose: () => void
  /** 放进去了之后通知页面重读「已放入」（表格那一行要跟着翻） */
  onPlaced: () => void
  load: (fileName: string) => Promise<SlicerPlacements>
  put: (fileName: string, slicer?: SlicerTarget, account?: string) => Promise<CopyToSlicerOutcome>
  reveal: (fileName: string, slicer?: SlicerTarget, account?: string) => Promise<void>
}

/** 一行按切片器归组时用的键（后端已经排好序，这里只按出现顺序收） */
function groupsOf(dirs: SlicerPlacement[]): { slicer: SlicerTarget; rows: SlicerPlacement[] }[] {
  const out: { slicer: SlicerTarget; rows: SlicerPlacement[] }[] = []
  for (const dir of dirs) {
    const last = out[out.length - 1]
    if (last !== undefined && last.slicer === dir.slicer) {
      last.rows.push(dir)
      continue
    }
    out.push({ slicer: dir.slicer, rows: [dir] })
  }
  return out
}

export default function PlacementsModal({
  open,
  fileName,
  host,
  onClose,
  onPlaced,
  load,
  put,
  reveal,
}: PlacementsModalProps) {
  const tr = useT()
  /*
   * `data` 与 `forName` 一起存：只存数据分不清它是**哪一份**的（换一行再开，
   * 上一份的账号清单会先闪一下）。
   */
  const [data, setData] = useState<{ fileName: string; rows: SlicerPlacements } | null>(null)
  /** 标题右边那一格那句话（读不出来 / 没放成 / 放成了） */
  const [note, setNote] = useState<{ text: string; tone: 'ok' | 'error' } | null>(null)
  /** 正在放的那一格（`切片器/账号`）；`'*'` = 页脚那颗「全部放入」 */
  const [busy, setBusy] = useState<string | null>(null)

  const reread = useCallback(
    async (name: string) => {
      setData({ fileName: name, rows: await load(name) })
    },
    [load],
  )

  /*
   * 读一次，三个时机：打开 / 换一份、窗口重新拿到焦点、从后台切回来。
   * **不是轮询** —— 没有定时器，只在"用户刚在别处动过"的时机问一次盘。
   */
  useEffect(() => {
    if (!open) return
    let alive = true
    setNote(null)
    setData(null)
    const read = () => {
      void load(fileName).then(
        (rows) => {
          if (alive) setData({ fileName, rows })
        },
        (e: unknown) => {
          /* 读不出来时框身给一句中性的话，原话进标题右边那一格 */
          if (!alive) return
          setData({ fileName, rows: { available: false, reason: null, dirs: [] } })
          setNote({ text: e instanceof Error ? e.message : String(e), tone: 'error' })
        },
      )
    }
    const onVisible = () => {
      if (document.visibilityState === 'visible') read()
    }
    read()
    window.addEventListener('focus', read)
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      alive = false
      window.removeEventListener('focus', read)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [open, fileName, load])

  const rows = data !== null && data.fileName === fileName ? data.rows : null
  const dirs = rows?.dirs ?? []
  const groups = groupsOf(dirs)
  /*
   * 页脚那颗「全部放入」只看**这份预设自己那一档**：
   *
   * - 表格那一行的「已放入」判的也是这一档（别的切片器不是它的生效条件）；
   * - 另一个切片器那几行是"用户自己看着放"的，不替他一次性塞进去 ——
   *   一颗写着"全部"的按钮把 Orca 也填了，等于把"自己看着"这件事偷偷做掉。
   */
  const ownRows = groups[0]?.rows ?? []
  const ownMissing = ownRows.filter((dir) => !dir.placed).length

  const runPut = async (slicer?: SlicerTarget, account?: string) => {
    if (busy !== null) return
    setNote(null)
    setBusy(slicer === undefined ? '*' : `${slicer}/${account ?? ''}`)
    try {
      const outcome = await put(fileName, slicer, account)
      await reread(fileName)
      onPlaced()
      /* 放成了就说一句（这一格会自己从「未放入」翻成「已放入」，那句话是补一句"落了几个"） */
      setNote({
        text:
          account === undefined
            ? tr('presets.placements.putDoneAll', { count: outcome.targets })
            : tr('presets.placements.putDone', { account }),
        tone: 'ok',
      })
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : String(e), tone: 'error' })
      /* 只放成一部分时盘已经变了 —— 重新读一次，让人看见到底哪几个没成 */
      await reread(fileName).catch(() => undefined)
    } finally {
      setBusy(null)
    }
  }

  const runReveal = async (slicer: SlicerTarget, account: string) => {
    setNote(null)
    try {
      await reveal(fileName, slicer, account)
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : String(e), tone: 'error' })
    }
  }

  const body =
    rows === null ? (
      <p className={s.hint}>{tr('presets.placements.loading')}</p>
    ) : !rows.available ? (
      <p className={s.hint}>{rows.reason ?? tr('presets.placements.unavailable')}</p>
    ) : (
      <div className={s.groups}>
        {groups.map((group, index) => (
          <section key={group.slicer} className={s.group}>
            {/*
             * 组标题 = 产品名。**第一组是这份预设自己那一档**（后端排的序），
             * 后面那些是"本机还装了别的切片器"—— 那几行带一句说明，放不放用户自己按。
             */}
            <h3 className={s.groupHead}>
              <span>{tr(SLICER_LABEL[group.slicer])}</span>
              {index > 0 && (
                <span className={s.groupNote}>{tr('presets.placements.otherSlicerNote')}</span>
              )}
            </h3>
            <ul className={s.list}>
              {group.rows.map((dir) => (
                <li key={dir.account} className={s.row} data-placed={dir.placed || undefined}>
                  <span className={s.rowIcon} aria-hidden="true">
                    <Icon name={dir.placed ? 'check' : 'folder'} size={16} />
                  </span>
                  <span className={s.rowMeta}>
                    <span className={s.rowName}>
                      <span className={s.account}>{dir.account}</span>
                      {dir.account === 'default' && (
                        <span className={s.tag}>{tr('presets.placements.defaultNote')}</span>
                      )}
                    </span>
                    <span className={s.rowPath} data-tip={dir.path}>
                      {dir.path}
                    </span>
                  </span>
                  <span className={s.badge} data-on={dir.placed || undefined}>
                    {tr(dir.placed ? 'presets.placements.placed' : 'presets.placements.notPlaced')}
                  </span>
                  <span className={s.rowActs}>
                    {!dir.placed && (
                      <Btn
                        variant="ghost"
                        disabled={busy !== null}
                        onClick={() => void runPut(dir.slicer, dir.account)}
                      >
                        {tr('presets.placements.put')}
                      </Btn>
                    )}
                    <button
                      type="button"
                      className={s.iconBtn}
                      aria-label={tr('presets.placements.reveal')}
                      data-tip={tr('presets.placements.reveal')}
                      onClick={() => void runReveal(dir.slicer, dir.account)}
                    >
                      <Icon name="external" size={14} />
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    )

  return (
    <Modal
      open={open}
      size="md"
      title={tr('presets.placements.title', { name: fileName })}
      host={host}
      onClose={onClose}
      footer={
        <>
          <span className={s.footLeft}>
            {ownMissing > 0 && (
              <Btn
                variant="primary"
                disabled={busy !== null}
                onClick={() => void runPut()}
                title={tr('presets.placements.putAllNote')}
              >
                {tr('presets.placements.putAll')}
              </Btn>
            )}
          </span>
          <Btn onClick={onClose}>{tr('presets.placements.close')}</Btn>
        </>
      }
    >
      {note !== null && <HeadNote text={note.text} tone={note.tone} />}
      {body}
    </Modal>
  )
}
