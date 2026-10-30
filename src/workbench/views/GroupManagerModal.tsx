/*
 * **分组管理器**（2026-10-16 作者）：
 *
 * > 「不能只用文件名去实现，必须有一个去管理它，像那个 catalog 那种的」
 * > 「我如果能自己分一下组，就更方便了。因为有一些事情说的没有自己去动手的更方便」
 *
 * # 它管的是什么
 *
 * **一组图 = 共用同一份标注源稿的那几张**。登记在台账 `presets/assets.toml` 的 `[[groups]]`
 * 里（跟资产一起入库、跟仓库走、编进 catalog），这里就是那份登记的管理界面。
 *
 * # 三条硬规矩（与后端 `Assets::check_groups` 一模一样，界面上先说清）
 *
 * 1. **同一组的成员必须在同一个目录** —— 源稿落点 = `<成员所在目录>/<组 id>.anno.json`；
 * 2. **一张资产至多属于一个组** —— 否则"它跟谁共用一份标注"有两种答案；
 * 3. **`id` 同时就是那份源稿的文件名** —— 所以它只在**新建时可填**（改它 = 那份标注要挪窝，
 *    旧文件会成孤儿；真需要就删了重建）。
 *
 * # 为什么编辑是"改完点保存"而不是"点一下存一次"
 *
 * 这一组是一个**整体**（名字 + 一张张成员），中途的半成品没有意义。所以草稿留在本地、
 * 一次交出去（后端也是整组一起校）。删组另说 —— 那是个独立决定，当场问一句就落。
 */
import { useEffect, useMemo, useState } from 'react'

import type { AssetView, GroupView } from '../api'
import ModalC14 from '../c14/ModalC14'
import s from '../c14.module.css'
import g from './GroupManagerModal.module.css'

/** 角色给几个常用的当提示（`light` / `dark`），但**允许随便写** —— 将来 macOS / Windows 同组要别的名 */
const ROLE_HINTS = ['light', 'dark']

/** 草稿里的一名成员（`display` 只为界面画得好看，落盘只交 `asset` + `role`） */
interface DraftMember {
  asset: string
  display: string
  role: string
}

interface Props {
  open: boolean
  groups: GroupView[]
  /** 全量资产（挑成员用） */
  assets: AssetView[]
  onClose: () => void
  /** 落一次盘（新建或改写）。失败就把话抛出来（调用方转通知） */
  onSave: (id: string, name: string, members: { asset: string; role: string }[]) => Promise<void>
  /** 删一个组。成员原地变回"未分组" */
  onRemove: (id: string) => Promise<void>
}

/**
 * 组名 → 建议的 id。
 *
 * ★ 只管**文件名里不能有的那些字符**（`/ \ : * ? " < > |` 与空白 ⇒ 全折成 `-`），
 *   **中文照留** —— 作者给东西起名就是中文（「不能只用文件名去实现」这句本身就是中文语境），
 *   逼他再想一个英文 id 是白添一步。后端那两道闸（`check_relative`）也只拦空 / 绝对路径 / `..`，
 *   中文文件名一路通。
 */
function suggestId(name: string): string {
  return name
    .trim()
    .replace(/[\\/:*?"<>|\s]+/g, '-')
    .replace(/^[.-]+/, '')
    .replace(/[.-]+$/, '')
    .slice(0, 60)
}

/** 这张图在哪个目录（与后端判"同一目录"同一把尺） */
const dirOf = (path: string) => path.split('/').slice(0, -1).join('/')

export default function GroupManagerModal({ open, groups, assets, onClose, onSave, onRemove }: Props) {
  /** 正在编辑哪一组；`null` = 新建 */
  const [selId, setSelId] = useState<string | null>(groups[0]?.id ?? null)
  const [draftId, setDraftId] = useState('')
  const [draftName, setDraftName] = useState('')
  const [draftMembers, setDraftMembers] = useState<DraftMember[]>([])
  const [adding, setAdding] = useState(false)
  const [confirmDel, setConfirmDel] = useState(false)
  const [busy, setBusy] = useState(false)
  const isNew = selId === null

  /* 换选中（或外面数据变了）就把草稿重新铺一遍 —— 草稿是这一组的镜像，不是另一份真相 */
  useEffect(() => {
    if (!open) return
    const g0 = groups.find((x) => x.id === selId) ?? null
    if (g0 === null && selId !== null) {
      /* 选中的那组没了（刚被删掉）⇒ 退到第一组 */
      setSelId(groups[0]?.id ?? null)
      return
    }
    if (g0 === null) {
      setDraftId('')
      setDraftName('')
      setDraftMembers([])
      return
    }
    setDraftId(g0.id)
    setDraftName(g0.name)
    setDraftMembers(
      g0.members.map((m) => ({ asset: m.asset, display: m.display, role: m.role })),
    )
  }, [open, groups, selId])

  /** 已经属于**别的**组的资产 id（小写）—— 规矩 2，不许拉进来 */
  const takenElsewhere = useMemo(() => {
    const mine = new Set(draftMembers.map((m) => m.asset.toLowerCase()))
    const out = new Set<string>()
    for (const grp of groups) {
      if (grp.id === selId) continue
      for (const m of grp.members) out.add(m.asset.toLowerCase())
    }
    for (const x of mine) out.add(x)
    return out
  }, [groups, selId, draftMembers])

  /**
   * 能拉进来的候选。**先按"同一个目录"收窄**（规矩 1）：
   * 组里已经有成员就只列那个目录的；还是空的就全列（第一条自己定目录）。
   */
  const candidates = useMemo(() => {
    const wantDir = draftMembers.length > 0 ? dirOf(assets.find((a) => a.id === draftMembers[0].asset)?.path ?? '') : null
    return assets
      .filter((a) => !takenElsewhere.has(a.id.toLowerCase()))
      .filter((a) => wantDir === null || dirOf(a.path) === wantDir)
      .sort((x, y) => x.display.localeCompare(y.display))
  }, [assets, draftMembers, takenElsewhere])

  const dirHint =
    draftMembers.length > 0
      ? `这一组在 ${dirOf(assets.find((a) => a.id === draftMembers[0].asset)?.path ?? '') || '（根目录）'}/`
      : '这一组还没有成员 —— 先加一张，这一组的目录就定下来了'

  const save = async () => {
    const id = isNew ? draftId.trim() : selId
    if (id === null || id === '') return
    setBusy(true)
    try {
      await onSave(
        id,
        draftName,
        draftMembers.map((m) => ({ asset: m.asset, role: m.role })),
      )
      if (isNew) setSelId(id)
    } finally {
      setBusy(false)
    }
  }

  return (
    <ModalC14
      open={open}
      title="管理分组"
      subtitle="一组 = 共用同一份标注的那几张图；登记在台账 `[[groups]]`，跟着仓库走"
      size="lg"
      onClose={onClose}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onClose}>
            关闭
          </button>
        </>
      }
    >
      <div className={g.wrap}>
        {/* 左：组列表 */}
        <div className={g.left}>
          <div className={g.leftHead}>
            <span className={s.dim}>分组 · {groups.length}</span>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => {
                setSelId(null)
                setDraftId('')
                setDraftName('')
                setDraftMembers([])
                setAdding(false)
                setConfirmDel(false)
              }}
            >
              + 新建分组
            </button>
          </div>
          <div className={g.leftList}>
            {groups.length === 0 && <div className={s.emptyHint}>还没有分组</div>}
            {groups.map((grp) => (
              <button
                key={grp.id}
                type="button"
                className={`${s.row} ${selId === grp.id ? s.rowOn : ''}`}
                onClick={() => {
                  setSelId(grp.id)
                  setAdding(false)
                  setConfirmDel(false)
                }}
              >
                <span className={s.rowName}>{grp.name}</span>
                <span className={s.rowMeta}>{grp.members.length} 张 · {grp.id}</span>
              </button>
            ))}
          </div>
        </div>

        {/* 右：这一组的编辑区 */}
        <div className={g.right}>
          <label className={g.field}>
            <span className={g.fieldLabel}>组名</span>
            <input
              className={s.inp}
              value={draftName}
              placeholder="例：「改 .3mf 默认打开方式」引导图"
              onChange={(e) => {
                setDraftName(e.target.value)
                if (isNew && draftId === '') setDraftId(suggestId(e.target.value))
              }}
            />
          </label>
          <label className={g.field}>
            <span className={g.fieldLabel}>
              组 id（**就是那份标注源稿的文件名**：{draftId || '…'}.anno.json）
            </span>
            <input
              className={s.inp}
              value={draftId}
              disabled={!isNew}
              placeholder="例：open-with"
              onChange={(e) => setDraftId(e.target.value)}
            />
            <span className={s.dim}>
              {isNew
                ? '文件名里不能有的字符会自动折成 `-`，中文可以。建好之后不再改 —— 改它等于让那份标注挪窝'
                : '建好之后不再改（改它等于让那份标注挪窝，旧文件会成孤儿）'}
            </span>
          </label>

          <div className={g.memHead}>
            <span className={s.dim}>
              成员 · {draftMembers.length} 张 —— {dirHint}
            </span>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              disabled={candidates.length === 0}
              onClick={() => setAdding((v) => !v)}
            >
              {adding ? '收起' : '+ 添加资产…'}
            </button>
          </div>

          {adding && (
            <div className={g.addBox}>
              {candidates.length === 0 && (
                <div className={s.emptyHint}>
                  没有能加的了（同目录、且还没进别的组的都在这儿了）
                </div>
              )}
              {candidates.map((a) => (
                <button
                  key={a.id}
                  type="button"
                  className={s.row}
                  onClick={() => {
                    setDraftMembers((ms) => [
                      ...ms,
                      { asset: a.id, display: a.display, role: '' },
                    ])
                    setAdding(false)
                  }}
                  data-tip={a.path}
                >
                  <span className={s.rowName}>{a.display}</span>
                  <span className={s.rowMeta}>{KIND_TEXT(a.kind)}</span>
                </button>
              ))}
            </div>
          )}

          <div className={g.memList}>
            {draftMembers.length === 0 && (
              <div className={s.emptyHint}>这一组还没有成员 —— 点上面「添加资产…」拉几张进来</div>
            )}
            {draftMembers.map((m, i) => (
              <div key={m.asset} className={g.memRow}>
                <span className={s.mono}>{m.display}</span>
                <input
                  className={`${s.inp} ${g.role}`}
                  value={m.role}
                  list="grp-roles"
                  placeholder="角色（可空）"
                  title="这张在这一组里演什么 —— 客户端按它挑图（浅色用 light、深色用 dark）"
                  onChange={(e) =>
                    setDraftMembers((ms) =>
                      ms.map((x, j) => (j === i ? { ...x, role: e.target.value } : x)),
                    )
                  }
                />
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  title="从这一组移出去（它不会删，只是变回未分组）"
                  onClick={() => setDraftMembers((ms) => ms.filter((_, j) => j !== i))}
                >
                  移出
                </button>
              </div>
            ))}
            <datalist id="grp-roles">
              {ROLE_HINTS.map((r) => (
                <option key={r} value={r} />
              ))}
            </datalist>
          </div>

          <div className={g.ops}>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={busy || draftName.trim() === '' || draftId.trim() === '' || draftMembers.length === 0}
              onClick={() => void save()}
            >
              {isNew ? '建立这一组（写进台账）' : '保存这一组（写进台账）'}
            </button>
            {!isNew && (
              <>
                <span className={s.grow} />
                {confirmDel ? (
                  <>
                    <span className={s.dim}>删了之后成员变回"未分组"，图不会动</span>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      onClick={() => setConfirmDel(false)}
                    >
                      取消
                    </button>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      disabled={busy}
                      onClick={() => {
                        void (async () => {
                          setBusy(true)
                          try {
                            await onRemove(selId)
                          } finally {
                            setBusy(false)
                            setConfirmDel(false)
                          }
                        })()
                      }}
                    >
                      确认删除
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    onClick={() => setConfirmDel(true)}
                  >
                    删除这个分组
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </ModalC14>
  )
}

/** 类型给人看的词（与资产库那一排筛选按钮同一套口径） */
function KIND_TEXT(kind: AssetView['kind']): string {
  switch (kind) {
    case 'image':
      return '机型图'
    case 'icon':
      return '图标'
    case 'guide':
      return '引导图'
    case 'model':
      return '模型'
    case 'slicerProfile':
      return '切片器'
    case 'mkPreset':
      return 'MKP 预设'
    default:
      return kind
  }
}
