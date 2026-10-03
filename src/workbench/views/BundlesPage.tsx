/**
 * 「套餐」页 —— C14 版式移植（feat/b05-14b-c14-port，menu 视角落地）。
 *
 * 两件事在这一页合起来：一份交付装什么（assetRefs），以及**谁在用它**
 * （版本 → 套餐）。指向模型是 C14 第十九轮定稿、作者 2026-09-30 拍板移植的
 * **一版一套**：每个版本自己指一份套餐（`recommendedBundle`），
 * 没指的回退机型 `defaultBundle`。
 *
 * # 套餐的唯一真源
 *
 * `presets/bundles.toml`（Task 13.6：发布 manifest v3 删掉了上游透传的 bundles，
 * 交付侧 `content/bundles.json` 由这里直出）。列表 / 内容 / 指向读它，
 * 内容编辑写它（`wb.setBundleRefs`，即时落盘）；改指向走 `wb.setVersionField`
 * （同样即时落盘）—— 两类都是清单编辑，不走参数草稿（同机型页的取舍）。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的
 *
 *  - **新建 / 复制 / 编辑 / 删除套餐没有接**：后端没有这些命令（新建的写入口
 *    在数据层、命令还没开；删除/改名要连带更新机型文件里的引用，模型没定）。
 *    对应的按钮与右键菜单**不渲染**，待裁决项记在 C14-PORT-PLAN §4。
 *  - **MKP 预设在资产库里**（作者 2026-10-03：「为什么资产库里面不放 mkp 预设」）：
 *    那一类 `type = 'mkPreset'`，套餐从资产库统一选（`assetRefs` 一份清单）。
 *    **文件在不在都能挂** —— 「有没有生成」是生成页四档状态的事，不拦选用。
 *    BBS 一侧的「至少一条」不因此放松（MKP 与 BBS 成套配发）。
 *  - **指向分两档报**：版本层（users，改指向动的是它）与机型默认
 *    （defaultFor，生成侧的回退）分开列 —— 混在一起的话「改套餐会动到谁」数不清。
 *  - **没有跨页撤销**：清单编辑即时落盘，Ctrl+Z 只管参数草稿（机型页同款文案）。
 *  - **行上的数是后端算的**：每行自己的指向数（C05 那个「读错 users」的坑
 *    在产品侧不存在 —— 判据根本不在前端）。
 */
import { useCallback, useEffect, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type { AssetList, BundleList, BundleView, MachineList, Words } from '../api'
import { ContextMenu } from '../components/menu'
import type { ContextMenuEntry } from '../components/menu/types'
import { useContextMenu } from '../components/menu/useContextMenu'
import ModalC14 from '../c14/ModalC14'
import SelectField from '../c14/field/SelectField'
import { locateAnchor } from '../c14/locate'
import { useSplitRail } from '../c14/SplitRail'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import BundleResourcesModal from './BundleResourcesModal'
import s from '../c14.module.css'

interface Props {
  words: Words
  /** 外壳的「后端状态变过了」计数：撤销 / 重做 / 保存之后 +1，页面据此重取 */
  tick: number
  /** 跨页定位带来的预选（机型页 ④ 关联的「选择套餐…」跳过来就是用这个） */
  initialSel?: string | null
  onGoto: (view: string, focus?: GotoFocus) => void
}

export default function BundlesPage({ words, tick, initialSel, onGoto }: Props) {
  const [list, setList] = useState<BundleList | null>(null)
  const [machines, setMachines] = useState<MachineList | null>(null)
  const [assets, setAssets] = useState<AssetList | null>(null)
  const [sel, setSel] = useState<string | null>(initialSel ?? null)
  const [filter, setFilter] = useState('')
  const [assignPicks, setAssignPicks] = useState<string[]>([])
  /** 「选择版本」的多选框（作者 2026-10-03：多选；不限机型） */
  const [assignPickOpen, setAssignPickOpen] = useState(false)
  /** 点了「确认指向」之后的**影响预览**框：摆清哪些版本会被改、原来指着谁 */
  const [assignConfirm, setAssignConfirm] = useState(false)
  /** 打开「套餐内容」框 —— 值是套餐 id。两页共用同一个组件 */
  const [resOpen, setResOpen] = useState<string | null>(null)
  const [pageErr, setPageErr] = useState<string | null>(null)
  /* —— 套餐 CRUD（作者 2026-10-03：C15 的能力，产品侧真写）—— */
  const [creating, setCreating] = useState(false)
  const [newId, setNewId] = useState('')
  const [newMachine, setNewMachine] = useState('')
  const [newDisplay, setNewDisplay] = useState('')
  const [editing, setEditing] = useState(false)
  const [editId, setEditId] = useState('')
  const [editDisplay, setEditDisplay] = useState('')
  const [copying, setCopying] = useState(false)
  const [copyId, setCopyId] = useState('')
  const [deleting, setDeleting] = useState(false)
  /** 「选内容」的框在新建流程里复用同一组件 —— bundle 传合成对象（空清单） */
  const [newResOpen, setNewResOpen] = useState(false)
  const [newRefs, setNewRefs] = useState<string[]>([])
  const bMenu = useContextMenu<string>()
  const fMenu = useContextMenu<string>()
  const uMenu = useContextMenu<string>()
  /** 左栏宽度可拖（作者 2026-10-03，同参数台） */
  const bodyRef = useRef<HTMLDivElement | null>(null)
  const rail = useSplitRail('bundles', bodyRef)

  const say = useCallback((e: unknown) => {
    toasts.push(isAppError(e) ? e.message : String(e))
  }, [])

  const load = useCallback((q: string) => {
    void (async () => {
      try {
        const [b, m, a] = await Promise.all([
          wb.bundles(q || null),
          wb.machines(),
          /* 套餐内容框的候选池（切片器页签）—— 没有它「改套餐内容」就没得选 */
          assets ? Promise.resolve(assets) : wb.assets(null, null, null, null, null, null),
        ])
        setList(b)
        setMachines(m)
        setAssets(a)
      } catch (e) {
        setPageErr(isAppError(e) ? e.message : String(e))
      }
    })()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  useEffect(() => {
    load('')
  }, [load])
  /* 筛选是命令参数（后端筛），防抖一拍再发；外壳的 tick（撤销/保存）也重取一遍 */
  useEffect(() => {
    const t = setTimeout(() => load(filter), 200)
    return () => clearTimeout(t)
  }, [filter, load, tick])

  const cur = list?.bundles.find((b) => b.id === sel)

  /* 预选的套餐可能刚被删了，或者根本没带预选 —— 不在清单里就回落到第一个（C14 同一条） */
  useEffect(() => {
    if (!list) return
    if (sel && list.bundles.some((b) => b.id === sel)) return
    setSel(list.bundles[0]?.id ?? null)
  }, [list, sel])

  /*
   * 「去处理」跳过来时：等清单读过、那一行确实在，滚过去 + 闪一下（见 `c14/locate.ts`）。
   * 只做一次 —— 之后用户在本页换选择不该被拽回去。
   */
  const locatedRef = useRef(false)
  useEffect(() => {
    if (locatedRef.current) return
    if (!initialSel || !list || !list.bundles.some((b) => b.id === initialSel)) return
    locatedRef.current = true
    locateAnchor(`t-bundle-${initialSel}`, { row: true })
  }, [initialSel, list])

  /** 全部版本的 uid（改指向的下拉用）。uid 与机型页同形：`机型/版本` */
  const allUids = (machines?.machines ?? []).flatMap((m) =>
    m.versions.map((v) => ({ uid: `${m.id}/${v.id}`, machineId: m.id, versionId: v.id, bundle: v.recommendedBundle ?? '' })),
  )
  const bundleOf = (uid: string): string =>
    allUids.find((x) => x.uid === uid)?.bundle ?? ''

  /** 解除一个指向 = 清掉版本自己的 `recommendedBundle`（回到机型默认）。即时落盘 */
  const unassign = async (machineId: string, versionId: string) => {
    try {
      setMachines(await wb.setVersionField(machineId, versionId, 'recommendedBundle', null))
      toasts.push(
        `已取消 ${machineId}/${versionId} 自己的指向 —— 它回到机型默认（清单改动即时落盘）`,
      )
      load(filter)
    } catch (e) {
      say(e)
    }
  }

  /** 把选中的版本指到这份套餐。**即时落盘**（清单编辑，不进参数草稿） */
  const assign = async (bundleId: string, uids: string[]) => {
    try {
      setList(await wb.assignBundleVersions(bundleId, uids))
      toasts.push(`已让 ${uids.join('、')} 改用 ${bundleId}（清单改动即时落盘）`)
      setAssignPicks([])
      setAssignConfirm(false)
      setMachines(await wb.machines())
      load(filter)
    } catch (e) {
      say(e)
    }
  }

  /** 换整份清单（移出一条 / 模态框确认都走它）。后端校验不过会整次拒绝 */
  const setRefs = async (bundleId: string, ids: string[], done: string) => {
    try {
      setList(await wb.setBundleRefs(bundleId, ids))
      toasts.push(done)
    } catch (e) {
      say(e)
    }
  }

  /* —— 套餐 CRUD 的动作（全部即时落盘；后端校验不过整次拒绝） —— */
  const run = useCallback(
    async (doIt: () => Promise<BundleList>, done: string) => {
      try {
        setList(await doIt())
        toasts.push(done)
      } catch (e) {
        say(e)
      }
    },
    [say],
  )

  const create = () => {
    if (!newId.trim() || !newMachine) return
    void run(
      () => wb.addBundle(newId.trim(), newMachine, newDisplay.trim() || '官方推荐', newRefs),
      `已新建套餐 ${newId.trim()}（${newRefs.length} 份文件）`,
    )
    setCreating(false)
    setNewRefs([])
    setNewId('')
    setNewMachine('')
    setNewDisplay('')
  }

  const saveEdit = () => {
    if (!cur) return
    const id = editId.trim()
    const display = editDisplay.trim()
    if (id === cur.id && display === cur.display) return
    void run(
      () =>
        wb.renameBundle(
          cur.id,
          id,
          display === cur.display ? null : display,
        ),
      `套餐 ${cur.id} 已改为 ${id}（机型文件里的引用一并重指）`,
    )
    setSel(id)
    setEditing(false)
  }

  const doCopy = () => {
    if (!cur || !copyId.trim()) return
    void run(() => wb.copyBundle(cur.id, copyId.trim(), null), `已复制为 ${copyId.trim()}（没人指着它）`)
    setSel(copyId.trim())
    setCopying(false)
  }

  /** 删除。被指着时**先在前端拦截**（把话说完），后端仍会再拦一次 */
  const doDelete = () => {
    if (!cur) return
    const holders = [
      ...cur.defaultFor.map((m) => `机型 ${m} 的 defaultBundle`),
      ...cur.users.map((u) => `版本 ${u.machineId}/${u.versionId}`),
    ]
    if (holders.length) {
      toasts.push(`套餐 ${cur.id} 还被引用着，不能删 —— ${holders.join('、')}。先把指向取消或改指别的套餐`)
      setDeleting(false)
      return
    }
    void run(() => wb.removeBundle(cur.id), `已删除 ${cur.id}`)
    setDeleting(false)
    setSel(null)
  }

  /* —— 套餐行的右键菜单：复制 / 编辑 / 删除（C05 的三处菜单之一） —— */
  const bundleEntries: ContextMenuEntry[] = bMenu.target
    ? (() => {
        const id = bMenu.target
        const open = (fn: () => void) => () => {
          setSel(id)
          fn()
        }
        return [
          { id: 'copy', label: '复制套餐…', onSelect: open(() => { setCopyId(''); setCopying(true) }) },
          { id: 'edit', label: '编辑…', onSelect: open(() => {
            const b = list?.bundles.find((x) => x.id === id)
            setEditId(b?.id ?? id)
            setEditDisplay(b?.display ?? '')
            setEditing(true)
          }) },
          { separator: true },
          { id: 'delete', label: '删除套餐…', danger: true, onSelect: open(() => setDeleting(true)) },
        ]
      })()
    : []

  /* —— 文件行的右键菜单：移出 / 去资产库看 —— */
  const fileEntries: ContextMenuEntry[] = fMenu.target
    ? (() => {
        const assetId = fMenu.target
        const ref = cur?.assetRefs.find((r) => r.id === assetId)
        return [
          {
            id: 'remove',
            label: '移出套餐',
            onSelect: () => {
              if (!cur) return
              void setRefs(
                cur.id,
                cur.assetRefs.filter((r) => r.id !== assetId).map((r) => r.id),
                `已把 ${assetId} 移出 ${cur.id}`,
              )
            },
          },
          {
            id: 'see',
            label: '在资产库里看…',
            onSelect: () => onGoto('assets', { machineId: null, uid: assetId, key: null }),
          },
          { separator: true },
          ...(ref
            ? [
                {
                  id: 'vis',
                  label: ref.visibility === 'menu' ? '改成仅归档' : '改回上菜单',
                  onSelect: () => onGoto('assets', { machineId: null, uid: assetId, key: 'visibility' }),
                } satisfies ContextMenuEntry,
              ]
            : []),
        ]
      })()
    : []

  /* —— 指向 chip 的右键菜单：去机型页 / 取消关联 —— */
  const userEntries: ContextMenuEntry[] = uMenu.target
    ? (() => {
        const uid = uMenu.target
        const [machineId, versionId] = uid.split('/')
        return [
          {
            id: 'see',
            label: '去机型页看这个版本…',
            onSelect: () => onGoto('machines', { machineId, uid, key: null }),
          },
          { separator: true },
          { id: 'unassign', label: '取消关联', danger: true, onSelect: () => void unassign(machineId, versionId) },
        ]
      })()
    : []

  if (!list || !machines) {
    if (pageErr) return <p className="wb-todo">{pageErr}</p>
    return <p className="wb-todo">正在读套餐定义……</p>
  }

  const listed = list.bundles
  const fileTotal = listed.reduce((n, b) => n + b.assetRefs.length, 0)

  return (
    <div className={s.split} ref={bodyRef} style={rail.style}>
      <div>
        <div className={s.topRow}>
          <input
            className={s.filter}
            value={filter}
            placeholder="筛套餐（id / 显示名）"
            aria-label="筛选套餐"
            onChange={(e) => setFilter(e.target.value)}
          />
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            onClick={() => {
              setNewId('')
              setNewMachine(machines.machines[0]?.id ?? '')
              setNewDisplay('')
              setNewRefs([])
              setCreating(true)
            }}
          >
            新建套餐
          </button>
        </div>
        <div className={s.list}>
          {listed.map((b) => {
            const bad = b.assetRefs.some((r) => r.visibility === 'archiveOnly')
            return (
              <button
                key={b.id}
                type="button"
                /* 「去处理」定位的锚点（locateAnchor 按它滚 + 闪） */
                id={`t-bundle-${b.id}`}
                className={`${s.row} ${sel === b.id ? s.rowOn : ''}`}
                onClick={() => setSel(b.id)}
                {...bMenu.triggerProps(b.id)}
              >
                <span className={s.rowName}>{b.id}</span>
                <span className={s.rowMeta}>{b.assetRefs.length} 个文件</span>
                {/* 版本数标色（作者 2026-10-03）：0 / 1 个是常态（绿），多个说明
                    好几台机型/版本共用这一份 —— 那是要提醒看一眼的事（红） */}
                {b.users.length > 1 ? (
                  <span className={`${s.tag} ${s.tagDanger}`}>{b.users.length} 个版本</span>
                ) : b.users.length === 1 ? (
                  <span className={s.rowMeta}>1 个版本</span>
                ) : (
                  /* 没人用不是错误 —— 刚建好还没挂上去就是这样 */
                  <span className={`${s.tag} ${s.tagGhost}`}>没人用</span>
                )}
                {bad && <span className={`${s.tag} ${s.tagDanger}`}>含归档文件</span>}
              </button>
            )
          })}
        </div>
        {!listed.length && (
          <div className={s.sum}>
            没有匹配「{filter}」的套餐{' '}
            <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setFilter('')}>
              清空筛选
            </button>
          </div>
        )}
        <div className={s.sum}>
          {listed.length === list.total
            ? `${list.total} 个套餐 · 装了 ${fileTotal} 个文件`
            : `筛出 ${listed.length} / ${list.total} 个`}
        </div>
      </div>

      <div className={s.detail}>
        {!cur ? (
          <div className={s.empty}>
            <h2>选一个套餐</h2>
            <p>{words.empty.selectBundle}</p>
          </div>
        ) : (
          <div className={s.card}>
            <div className={s.cardHead}>
              <h2>{cur.id}</h2>
              <span className={s.cardNote}>{cur.display}</span>
              <button type="button" className={s.btn} onClick={() => { setEditId(cur.id); setEditDisplay(cur.display); setEditing(true) }}>
                编辑
              </button>
              <button type="button" className={s.btn} onClick={() => { setCopyId(''); setCopying(true) }}>
                复制
              </button>
              <button type="button" className={s.btn} onClick={() => setDeleting(true)}>
                删除
              </button>
            </div>
            <div className={s.cardBody}>
              {cur.defaultFor.length > 0 && (
                <p className={s.note} style={{ marginTop: 0 }}>
                  机型默认跟着它的：<strong>{cur.defaultFor.join('、')}</strong>
                  （版本没自己指套餐时，生成侧回退到这份）
                </p>
              )}
              {cur.assetRefs.some((r) => r.visibility === 'archiveOnly') && (
                <div className={s.warn}>
                  <div className={s.warnTitle}>装了「仅归档」的文件</div>
                  <div className={s.warnDetail}>
                    客户端看得到这个套餐却拿不到这份文件，而且<strong>不会报错</strong> ——
                    只是少一个东西。去资产库把它的交付身份改回来。
                  </div>
                </div>
              )}

              <div className={s.group}>
                <div className={s.groupHead}>
                  MKP 预设（{cur.assetRefs.filter((r) => r.kind === 'mkPreset').length}）
                </div>
                {/* 资产库里的 `mkPreset` 类（2026-10-03 按作者裁决进的台账）——
                    **文件在不在都能挂**，「有没有生成」是生成页的状态，不拦选用 */}
                {cur.assetRefs
                  .filter((r) => r.kind === 'mkPreset')
                  .map((r) => (
                    <div key={r.id} className={s.fileRow} {...fMenu.triggerProps(r.id)}>
                      <span className={`${s.fileName} ${s.mono}`}>{r.name || r.id}</span>
                      <span className={s.tag}>MKP</span>
                      {/* 生成状态徽章（与资产库 / 生成页同一套词与判据）：
                          「有没有生成」是生成页的状态，不拦挂载 */}
                      {r.buildState && (
                        <span
                          className={s.tag}
                          title={words.build[r.buildState]?.explain ?? '这一版的产物还没生成'}
                        >
                          {words.build[r.buildState]?.label ?? r.buildState}
                        </span>
                      )}
                      <button
                        type="button"
                        className={`${s.btn} ${s.btnSm}`}
                        onClick={() =>
                          void setRefs(
                            cur.id,
                            cur.assetRefs.filter((x) => x.id !== r.id).map((x) => x.id),
                            `已把 ${r.id} 移出 ${cur.id}`,
                          )
                        }
                      >
                        移出
                      </button>
                    </div>
                  ))}
                {/* 旧数据里 assetRefs 可能挂着图标 / 模型类资产 —— 照实显示 */}
                {cur.assetRefs
                  .filter((r) => r.kind !== 'slicerProfile' && r.kind !== 'mkPreset')
                  .map((r) => (
                    <div key={r.id} className={s.fileRow} {...fMenu.triggerProps(r.id)}>
                      <span className={`${s.fileName} ${s.mono}`}>{r.name || r.id}</span>
                      <span className={s.tag}>{r.kind}</span>
                      {r.visibility === 'archiveOnly' && (
                        <span className={`${s.tag} ${s.tagDanger}`}>仅归档</span>
                      )}
                      <button
                        type="button"
                        className={`${s.btn} ${s.btnSm}`}
                        onClick={() =>
                          void setRefs(
                            cur.id,
                            cur.assetRefs.filter((x) => x.id !== r.id).map((x) => x.id),
                            `已把 ${r.id} 移出 ${cur.id}`,
                          )
                        }
                      >
                        移出
                      </button>
                    </div>
                  ))}
                {!cur.assetRefs.some((r) => r.kind !== 'slicerProfile') && (
                  <p className={s.note} style={{ margin: 0 }}>
                    一个都没装 —— 点「改套餐内容…」从资产库挂 MKP 预设
                    （文件还没生成也能先挂，生成之后文件自动补上）
                  </p>
                )}
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>切片器文件（{cur.assetRefs.filter((r) => r.kind === 'slicerProfile').length}）</div>
                {cur.assetRefs
                  .filter((r) => r.kind === 'slicerProfile')
                  .map((r) => (
                    <div key={r.id} className={s.fileRow} {...fMenu.triggerProps(r.id)}>
                      <span className={`${s.fileName} ${s.mono}`}>{r.name || r.id}</span>
                      <span className={s.tag}>BBS</span>
                      {r.visibility === 'archiveOnly' && (
                        <span className={`${s.tag} ${s.tagDanger}`}>仅归档</span>
                      )}
                      <button
                        type="button"
                        className={`${s.btn} ${s.btnSm}`}
                        onClick={() =>
                          void setRefs(
                            cur.id,
                            cur.assetRefs.filter((x) => x.id !== r.id).map((x) => x.id),
                            `已把 ${r.id} 移出 ${cur.id}`,
                          )
                        }
                      >
                        移出
                      </button>
                    </div>
                  ))}
                {!cur.assetRefs.some((r) => r.kind === 'slicerProfile') && (
                  <p className={s.note} style={{ margin: 0 }}>
                    一个都没装
                  </p>
                )}
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>套餐里的文件</div>
                <div className={s.addRow}>
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    onClick={() => setResOpen(cur.id)}
                  >
                    改套餐内容…
                  </button>
                  <span className={s.cardNote}>
                    一个框里两个页签（MKP 预设 / 切片器）——
                    和机型页那颗「改套餐内容…」打开的是<strong>同一个框</strong>
                  </span>
                </div>
                <p className={s.note}>
                  改动即时落盘（updatedAt 会盖上今天）；存一条 BBS 都没有的套餐存不进去
                  —— MKP 与 BBS 必须成套配发。
                </p>
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>被哪些版本指向（{cur.users.length}）</div>
                <div className={s.chips}>
                  {cur.users.length ? (
                    cur.users.map((u) => {
                      const uid = `${u.machineId}/${u.versionId}`
                      return (
                        <span key={uid} className={s.chip} {...uMenu.triggerProps(uid)}>
                          <button
                            type="button"
                            className={s.chipMain}
                            title="去机型页看这个版本"
                            onClick={() => onGoto('machines', { machineId: u.machineId, uid, key: null })}
                          >
                            {uid}
                          </button>
                          <button
                            type="button"
                            className={s.chipX}
                            title={`取消 ${uid} 自己的指向`}
                            aria-label={`取消 ${uid} 自己的指向`}
                            onClick={() => void unassign(u.machineId, u.versionId)}
                          >
                            {/*
                              叉用 SVG 不用文字「×」（C14 第十九轮）—— 字形坐在基线上，
                              在 16px 的格子里天生偏下；描边图形 place-items 居中就是正中
                            */}
                            <svg
                              viewBox="0 0 8 8"
                              width="8"
                              height="8"
                              fill="none"
                              stroke="currentColor"
                              strokeWidth="1.2"
                              strokeLinecap="round"
                              aria-hidden
                            >
                              <path d="m1.6 1.6 4.8 4.8M6.4 1.6 1.6 6.4" />
                            </svg>
                          </button>
                        </span>
                      )
                    })
                  ) : (
                    <span className={s.cardNote}>
                      没有版本用它 —— 合法中间状态（大家都走机型默认），不是错误
                    </span>
                  )}
                </div>
                {cur.users.length > 0 && (
                  <p className={s.note}>
                    点 chip 跳到那个版本，点 <strong>×</strong> 取消它自己的指向
                    （它会回到机型默认）。右键 chip 也是这两项。
                  </p>
                )}
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>把版本指到这个套餐</div>
                <div className={s.addRow}>
                  <button
                    type="button"
                    className={s.btn}
                    onClick={() => setAssignPickOpen(true)}
                  >
                    选择版本…（{assignPicks.length} 已选）
                  </button>
                  <span className={s.cardNote}>
                    <strong>一个套餐可以被多个版本指</strong>（不限机型）；
                    <strong>一个版本只指一个套餐</strong> —— 一版一套
                  </span>
                </div>
                {assignPicks.length > 0 && (
                  <div className={s.chips}>
                    {assignPicks.map((uid) => {
                      const now = bundleOf(uid)
                      return (
                        <span key={uid} className={s.chip}>
                          {uid}
                          <span className={s.cardNote}>
                            {now.trim().toLowerCase() === cur.id.toLowerCase()
                              ? '已经指着它'
                              : now || '还没配（走机型默认）'}
                          </span>
                          <button
                            type="button"
                            className={s.chipX}
                            aria-label={`从这次选择里去掉 ${uid}`}
                            onClick={() => setAssignPicks(assignPicks.filter((x) => x !== uid))}
                          >
                            ✕
                          </button>
                        </span>
                      )
                    })}
                  </div>
                )}
                <div className={s.addRow} style={{ marginTop: 8 }}>
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnPrimary}`}
                    disabled={assignPicks.length === 0}
                    title={assignPicks.length === 0 ? '先在上面挑版本' : undefined}
                    onClick={() => setAssignConfirm(true)}
                  >
                    确认指向
                  </button>
                  <span className={s.cardNote}>
                    点了先看<strong>影响预览</strong>（哪些版本会被改、原来指着谁），确认之后才落盘
                  </span>
                </div>
              </div>
              <p className={s.note}>
                真源关系是<strong>一版一套</strong>：每个版本指向自己的套餐，改了指向之后
                那一版的生成与发布跟着走（后端判据，不在前端）。
              </p>
            </div>
          </div>
        )}
      </div>

      <ContextMenu at={bMenu.at} entries={bundleEntries} onClose={bMenu.close} />
      <ContextMenu at={fMenu.at} entries={fileEntries} onClose={fMenu.close} />
      <ContextMenu at={uMenu.at} entries={userEntries} onClose={uMenu.close} />

      <BundleResourcesModal
        open={resOpen !== null}
        bundle={list.bundles.find((b) => b.id === resOpen) ?? null}
        assets={assets}
        onCancel={() => setResOpen(null)}
        onConfirm={(assetIds) => {
          if (resOpen === null) return
          void setRefs(resOpen, assetIds, `已把套餐 ${resOpen} 的清单更新（${assetIds.length} 份文件）`)
          setResOpen(null)
        }}
      />

      {/* —— 新建套餐：id / 机型 / 显示名 + 内容（至少一条 BBS，后端闸）—— */}
      <ModalC14
        open={creating}
        title="新建套餐"
        subtitle="一份交付装什么（MKP 预设 + 切片器），装好再建 —— 内容以后也能改"
        size="md"
        onClose={() => setCreating(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setCreating(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!newId.trim() || !newMachine}
              title={!newId.trim() ? '先填 id' : !newMachine ? '先选归属机型' : undefined}
              onClick={create}
            >
              创建
            </button>
          </>
        }
      >
        <div className={s.addRow}>
          <SelectField
            label="归属机型"
            value={newMachine}
            options={[
              { value: '', label: '选择机型…' },
              ...machines.machines.map((m) => ({ value: m.id, label: m.display || m.id })),
            ]}
            onChange={setNewMachine}
          />
        </div>
        <div className={s.addRow}>
          <input
            className={s.filter}
            value={newId}
            placeholder="id（如 A1_NEW，不许空白或 /）"
            aria-label="新套餐 id"
            onChange={(e) => setNewId(e.target.value)}
          />
          <input
            className={s.filter}
            value={newDisplay}
            placeholder="显示名（默认：官方推荐）"
            aria-label="新套餐显示名"
            onChange={(e) => setNewDisplay(e.target.value)}
          />
        </div>
        <div className={s.addRow}>
          <button type="button" className={s.btn} onClick={() => setNewResOpen(true)}>
            选内容…（{newRefs.length} 份）
          </button>
          <span className={s.cardNote}>至少一条切片器预设 —— MKP 与 BBS 必须成套配发</span>
        </div>
        <BundleResourcesModal
          open={newResOpen}
          bundle={syntheticBundle(newId, newMachine, newDisplay, newRefs)}
          assets={assets}
          onCancel={() => setNewResOpen(false)}
          onConfirm={(assetIds) => {
            setNewRefs(assetIds)
            setNewResOpen(false)
          }}
        />
      </ModalC14>

      {/* —— 编辑：id 与/或显示名；改 id 连带重指机型文件（后端做） —— */}
      <ModalC14
        open={editing}
        title={`编辑套餐 · ${cur?.id ?? ''}`}
        subtitle="改 id 会把机型文件里的 defaultBundle / recommendedBundle 一并重指"
        size="md"
        onClose={() => setEditing(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setEditing(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={editId.trim() === cur?.id && editDisplay.trim() === cur?.display}
              title={editId.trim() === cur?.id && editDisplay.trim() === cur?.display ? '两个值都没动' : undefined}
              onClick={saveEdit}
            >
              保存
            </button>
          </>
        }
      >
        <div className={s.addRow}>
          <input
            className={s.filter}
            value={editId}
            placeholder="id"
            aria-label="套餐 id"
            onChange={(e) => setEditId(e.target.value)}
          />
          <input
            className={s.filter}
            value={editDisplay}
            placeholder="显示名"
            aria-label="套餐显示名"
            onChange={(e) => setEditDisplay(e.target.value)}
          />
        </div>
      </ModalC14>

      {/* —— 复制：内容照抄，id 必须是新的；复制出来的那份没人指着 —— */}
      <ModalC14
        open={copying}
        title={`复制套餐 · ${cur?.id ?? ''}`}
        subtitle="内容与归属照抄；复制出来的那份不被任何版本指着 —— 指向要人显式改"
        size="md"
        onClose={() => setCopying(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setCopying(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!copyId.trim()}
              onClick={doCopy}
            >
              复制
            </button>
          </>
        }
      >
        <div className={s.addRow}>
          <input
            className={s.filter}
            value={copyId}
            placeholder="新套餐 id"
            aria-label="新套餐 id"
            onChange={(e) => setCopyId(e.target.value)}
          />
        </div>
      </ModalC14>

      {/* —— 选择版本（多选；不限机型） —— */}
      <ModalC14
        open={assignPickOpen}
        title={`选择版本 · ${cur?.id ?? ''}`}
        subtitle="可以多选；一个套餐可以被多个版本指，一个版本只指一个套餐"
        size="md"
        onClose={() => setAssignPickOpen(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setAssignPickOpen(false)}>
              完成（{assignPicks.length} 已选）
            </button>
          </>
        }
      >
        <div className={s.bunList}>
          {allUids.map((u) => {
            const on = assignPicks.includes(u.uid)
            const now = u.bundle
            return (
              <label key={u.uid} className={`${s.row} ${s.rowPick}`} style={{ cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={on}
                  onChange={() =>
                    setAssignPicks(on ? assignPicks.filter((x) => x !== u.uid) : [...assignPicks, u.uid])
                  }
                />
                <span className={`${s.mono} ${s.rowMeta}`}>{u.uid}</span>
                <span className={s.rowMeta}>
                  {cur && now.trim().toLowerCase() === cur.id.toLowerCase()
                    ? '已经指着它'
                    : now || '还没配（走机型默认）'}
                </span>
                {cur && cur.machineId !== u.machineId && (
                  <span className={`${s.tag} ${s.tagGhost}`}>归属 {cur.machineId} 的套餐</span>
                )}
              </label>
            )
          })}
        </div>
      </ModalC14>

      {/* —— 影响预览：确认之前把「谁会被改」摆出来 —— */}
      <ModalC14
        open={assignConfirm}
        title={`确认指向 · ${cur?.id ?? ''}`}
        subtitle="一个套餐可以被多个版本指 —— 改了指向之后，这些版本的生成与发布都会跟着走"
        size="md"
        onClose={() => setAssignConfirm(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setAssignConfirm(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              onClick={() => cur && void assign(cur.id, assignPicks)}
            >
              确认
            </button>
          </>
        }
      >
        <div className={s.kv}>
          <span className={s.kvKey}>会改动</span>
          <span className={s.kvVal}>
            {assignPicks.length} 个版本的指向
            {(() => {
              const noop = assignPicks.filter(
                (uid) => bundleOf(uid).trim().toLowerCase() === cur?.id.toLowerCase(),
              )
              return noop.length > 0 ? `（其中 ${noop.length} 个已经指着它，不算改动）` : ''
            })()}
          </span>
        </div>
        <div className={s.chips} style={{ marginTop: 8 }}>
          {assignPicks.map((uid) => (
            <span key={uid} className={s.chip}>
              {uid}：{bundleOf(uid) || '（未配）'} → {cur?.id}
            </span>
          ))}
        </div>
        <p className={s.note}>
          确认之后<strong>即时落盘</strong>（没有草稿、没有撤销）；要撤就把这些版本改指回原来的套餐。
        </p>
      </ModalC14>

      {/* —— 删除：被机型默认或版本指着时是拦截页（说清是谁、去哪解除） —— */}
      <ModalC14
        open={deleting}
        title={`删除套餐 · ${cur?.id ?? ''}`}
        subtitle={
          cur && (cur.users.length || cur.defaultFor.length)
            ? undefined
            : '没人指着它 —— 删掉之后这份登记就没了（文件本体还在资产库）'
        }
        size="md"
        onClose={() => setDeleting(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setDeleting(false)}>
              {cur && (cur.users.length || cur.defaultFor.length) ? '知道了' : '取消'}
            </button>
            {cur && !cur.users.length && !cur.defaultFor.length && (
              <button type="button" className={`${s.btn} ${s.btnDanger}`} onClick={doDelete}>
                删除
              </button>
            )}
          </>
        }
      >
        {cur && (cur.users.length || cur.defaultFor.length) ? (
          <div className={s.warn}>
            <div className={s.warnTitle}>还有地方指着它，不能删</div>
            <div className={s.warnDetail}>
              静默删掉会让那些版本生成时一条切片器预设都拿不到。先把指向逐个取消
              （chip 上的 ×，版本回到机型默认），或者把它们改指别的套餐。
            </div>
            <div className={s.chips}>
              {cur.defaultFor.map((m) => (
                <span key={m} className={s.chip}>
                  机型 {m}
                </span>
              ))}
              {cur.users.map((u) => (
                <span key={`${u.machineId}/${u.versionId}`} className={s.chip}>
                  {u.machineId}/{u.versionId}
                </span>
              ))}
            </div>
          </div>
        ) : (
          <p className={s.note} style={{ margin: 0 }}>
            删掉之后这份登记就没了。它装的文件本体还在资产库里，不受影响。
          </p>
        )}
      </ModalC14>
      {rail.handle}
    </div>
  )
}

/** 新建流程里「选内容」用的合成套餐（空清单起步） */
function syntheticBundle(
  id: string,
  machineId: string,
  display: string,
  refs: string[],
): BundleView {
  return {
    id: id.trim() || '(新套餐)',
    display: display.trim() || '官方推荐',
    machineId,
    assetRefs: refs.map((r) => {
      /* 勾选态只要 id —— 其余字段是详情卡用的，给中性值 */
      return {
        id: r,
        kind: 'slicerProfile' as const,
        resolvable: true,
        isBbs: true,
        name: r,
        present: true,
        buildState: null,
        visibility: 'menu' as const,
      }
    }),
    updatedAt: null,
    users: [],
    defaultFor: [],
  }
}
