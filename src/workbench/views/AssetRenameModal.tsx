/*
 * **改名**（2026-11-19 作者）：
 *
 * > 「我希望可以改名，甚至就是批量的改名都可以……改了之后所有涉及到它的地方都要有，
 * >   而且跟其他页面一样，改的时候是进那个撤销和重做的，保存的时候才是落盘」
 *
 * # 改的是**显示名**，落在哪儿由类型决定（后端 `display_name` 一张表）
 *
 * | 类 | 显示名住在 | 文件 |
 * |---|---|---|
 * | 切片器 | 文件名去扩展名 | 改名（草稿里搬，保存才进真仓库） |
 * | 机型图 / 图标 / 模型 / 引导图 | 台账 `name` | 不动 |
 * | MKP 预设 | 版本名 | 这里不给改（去机型与版本页） |
 *
 * # 为什么先问一遍预览
 *
 * 「改完长什么样、哪一行拦得住」全在后端算（`wb_rename_assets_preview`）——
 * 前端不自己推文件名、也不自己判重名：那会变成第二套判据。与批量改值同一条纪律：
 * **先预览再落**，落下去是一笔结构编辑（一条撤销）。
 */
import { useEffect, useMemo, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type { AssetRenameInput, AssetRenamePlan, AssetRenameRow, AssetView } from '../api'
import ModalC14 from '../c14/ModalC14'
import s from '../c14.module.css'
import { KIND_LABEL } from './assetKindLabel'
import g from './AssetRenameModal.module.css'

interface Props {
  open: boolean
  /** 候选（批量那一档用：当前筛出来的这一批） */
  assets: AssetView[]
  /** 打开时选中的那几条（单条那一档 = 那一条） */
  initial: string[]
  /** 单条 = 一个输入框；批量 = 勾选 + 查找替换 */
  single: boolean
  onClose: () => void
  /** 落一次（失败把话抛出来，框不关） */
  onApply: (items: AssetRenameInput[]) => Promise<void>
}

export default function AssetRenameModal({
  open,
  assets,
  initial,
  single,
  onClose,
  onApply,
}: Props) {
  /** 批量那一档勾了谁（按 id，顺序不重要） */
  const [picked, setPicked] = useState<string[]>(initial)
  /** 单条那一档正在打的字（`id → 名字`） */
  const [manual, setManual] = useState<Record<string, string>>({})
  const [find, setFind] = useState('')
  const [repl, setRepl] = useState('')
  const [plan, setPlan] = useState<AssetRenamePlan | null>(null)
  /** 这份 `plan` 对应的是哪一串输入 —— 输入变了而预览还没回来时不许落 */
  const [planKey, setPlanKey] = useState('')
  const [planErr, setPlanErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /* 每次**打开**都把草稿铺成"这一批"（关掉再开不复用上一次的输入）。
     只在打开那一沿铺 —— 依赖里带上 assets / initial 是为了拿到最新那份，
     但每渲染都铺一遍会把正在打的字冲掉 */
  const wasOpen = useRef(false)
  useEffect(() => {
    if (open && !wasOpen.current) {
      setPicked(initial)
      setFind('')
      setRepl('')
      setPlan(null)
      setPlanKey('')
      setPlanErr(null)
      setManual(
        Object.fromEntries(
          assets.filter((a) => initial.includes(a.id)).map((a) => [a.id, a.display]),
        ),
      )
    }
    wasOpen.current = open
  }, [open, assets, initial])

  const proposed = (a: AssetView): string => {
    if (single) return manual[a.id] ?? a.display
    /* 查找为空 = 不动（不给"整批加前缀"这种隐含行为） */
    if (find === '') return a.display
    return a.display.split(find).join(repl)
  }

  const pickedAssets = useMemo(
    () =>
      single
        ? assets.filter((a) => initial.includes(a.id))
        : assets.filter((a) => picked.includes(a.id)),
    [assets, single, initial, picked],
  )

  const items = useMemo<AssetRenameInput[]>(
    () => pickedAssets.map((a) => ({ id: a.id, display: proposed(a) })),
    // proposed 是纯函数，依赖它用到的那几格
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [pickedAssets, single, manual, find, repl],
  )

  /* 输入的身份：预览回来时比一下，"回的是不是现在这一串" —— 打字比预览快时
     按钮不许亮（否则点下去落的是上一帧那个名字） */
  const itemsKey = items.map((i) => `${i.id}\u0000${i.display}`).join('\u0001')

  /* 预览：输入停下 120ms 才问一次（逐键问一遍等于每敲一下扫一次台账与两个目录） */
  useEffect(() => {
    if (!open) return
    if (items.length === 0) {
      setPlan(null)
      setPlanKey('')
      setPlanErr(null)
      return
    }
    const want = items.map((i) => `${i.id}\u0000${i.display}`).join('\u0001')
    let alive = true
    const timer = window.setTimeout(() => {
      wb.renameAssetsPreview(items)
        .then((p) => {
          if (!alive) return
          setPlan(p)
          setPlanKey(want)
          setPlanErr(null)
        })
        .catch((e) => {
          if (!alive) return
          setPlan(null)
          setPlanKey('')
          setPlanErr(isAppError(e) ? e.message : String(e))
        })
    }, 120)
    return () => {
      alive = false
      window.clearTimeout(timer)
    }
  }, [open, items])

  const fresh = plan !== null && planKey === itemsKey
  const changed = (plan?.rows ?? []).filter((r) => r.changed && r.problem === null)
  const canApply = fresh && (plan?.ok ?? false) && changed.length > 0 && !busy

  const submit = async () => {
    if (!canApply) return
    setBusy(true)
    try {
      await onApply(changed.map((r) => ({ id: r.id, display: r.new })))
      onClose()
    } catch {
      /* 失败的话页面已经弹过通知 —— 框留着，人改完再试 */
    } finally {
      setBusy(false)
    }
  }

  const rowOf = (id: string): AssetRenameRow | undefined =>
    plan?.rows.find((r) => r.id === id)

  const toggle = (id: string) =>
    setPicked((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))

  return (
    <ModalC14
      open={open}
      title={single ? '改名' : '批量改名'}
      subtitle={
        single
          ? undefined
          : '切片器改文件名，其余改台账名；保存后写入'
      }
      size="lg"
      onClose={onClose}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button
            type="button"
            className={s.btn}
            onClick={() => void submit()}
            disabled={!canApply}
          >
            {single ? '改名' : `改名 ${changed.length} 条`}
          </button>
        </>
      }
    >
      {single ? (
        <div className={g.wrap}>
          {pickedAssets.map((a) => {
            const row = rowOf(a.id)
            return (
              <label key={a.id} className={g.field}>
                <span className={g.label}>新名字</span>
                <input
                  className={s.inp}
                  autoFocus
                  value={manual[a.id] ?? a.display}
                  onChange={(e) => setManual({ ...manual, [a.id]: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void submit()
                  }}
                />
                <PreviewLine row={row} fallbackNew={proposed(a)} />
              </label>
            )
          })}
          {planErr !== null && <p className={s.note}>{planErr}</p>}
        </div>
      ) : (
        <div className={g.wrap}>
          <div className={g.bar}>
            <label className={g.field}>
              <span className={g.label}>查找</span>
              <input
                className={s.inp}
                value={find}
                onChange={(e) => setFind(e.target.value)}
                placeholder="名字里的一段"
              />
            </label>
            <label className={g.field}>
              <span className={g.label}>替换为</span>
              <input
                className={s.inp}
                value={repl}
                onChange={(e) => setRepl(e.target.value)}
              />
            </label>
          </div>
          <div className={g.bar}>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => setPicked(assets.map((a) => a.id))}
            >
              全选
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => setPicked([])}
            >
              全不选
            </button>
            <span className={g.hint}>
              已选 {pickedAssets.length} / {assets.length}
            </span>
          </div>
          <div className={g.list}>
            {assets.map((a) => {
              const on = picked.includes(a.id)
              const row = on ? rowOf(a.id) : undefined
              return (
                <div key={a.id} className={`${g.row} ${on ? '' : g.rowOff}`}>
                  <input
                    type="checkbox"
                    checked={on}
                    onChange={() => toggle(a.id)}
                    aria-label={`选择 ${a.display}`}
                  />
                  <span className={s.cardNote}>{KIND_LABEL[a.kind]}</span>
                  <span className={g.old}>{a.display}</span>
                  <span className={g.arrow}>→</span>
                  <span className={g.new}>
                    {on ? (row?.new ?? proposed(a)) : <span className={g.blank}>—</span>}
                  </span>
                  {on && <PreviewLine row={row} fallbackNew={proposed(a)} />}
                </div>
              )
            })}
          </div>
          {planErr !== null && <p className={s.note}>{planErr}</p>}
        </div>
      )}
    </ModalC14>
  )
}

/**
 * 一行预览的补充：**文件那一段**（切片器才有）+ 拦得住的原因。
 *
 * 文件那一段是这一页要说清的事 —— "改了名字"对切片器意味着盘上那份文件也跟着换名，
 * 而层高 / 喷嘴是从名字里读出来的（后端预览会把"层高被改掉"这类拦下来）。
 */
function PreviewLine({
  row,
  fallbackNew,
}: {
  row: AssetRenameRow | undefined
  fallbackNew: string
}) {
  /* 预览还没回来（或这一条不在这一批里）：只说"正在算"，别拿旧结论顶 */
  if (row === undefined) {
    return <span className={g.fileLine}>{fallbackNew === '' ? ' ' : '…'}</span>
  }
  const fileLine =
    row.oldPath !== null && row.newPath !== null
      ? `${row.oldPath.split('/').pop()} → ${row.newPath.split('/').pop()}`
      : null
  return (
    <>
      {row.problem !== null ? (
        <span className={g.problem}>{row.problem}</span>
      ) : (
        fileLine !== null && <span className={g.fileLine}>{fileLine}</span>
      )}
    </>
  )
}
