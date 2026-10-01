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
 *  - **MKP 组恒空**：MKP 预设不建资产条目（doc §12.5）—— 套餐的内容就是
 *    BBS 引用（doc §12.4），空组照实说，不装作有货。
 *  - **指向分两档报**：版本层（users，改指向动的是它）与机型默认
 *    （defaultFor，生成侧的回退）分开列 —— 混在一起的话「改套餐会动到谁」数不清。
 *  - **没有跨页撤销**：清单编辑即时落盘，Ctrl+Z 只管参数草稿（机型页同款文案）。
 *  - **行上的数是后端算的**：每行自己的指向数（C05 那个「读错 users」的坑
 *    在产品侧不存在 —— 判据根本不在前端）。
 */
import { useCallback, useEffect, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type { AssetList, BundleList, MachineList, Words } from '../api'
import { ContextMenu } from '../components/menu'
import type { ContextMenuEntry } from '../components/menu/types'
import { useContextMenu } from '../components/menu/useContextMenu'
import SelectField from '../c14/field/SelectField'
import { locateAnchor } from '../c14/locate'
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
  const [assignUid, setAssignUid] = useState('')
  /** 打开「套餐内容」框 —— 值是套餐 id。两页共用同一个组件 */
  const [resOpen, setResOpen] = useState<string | null>(null)
  const [pageErr, setPageErr] = useState<string | null>(null)
  const fMenu = useContextMenu<string>()
  const uMenu = useContextMenu<string>()

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

  /** 把版本指到这份套餐。即时落盘 */
  const assign = async (uid: string, bundleId: string) => {
    const [machineId, versionId] = uid.split('/')
    try {
      setMachines(await wb.setVersionField(machineId, versionId, 'recommendedBundle', bundleId))
      toasts.push(`已让 ${uid} 改用 ${bundleId}（真源关系是一版一套）`)
      setAssignUid('')
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
    <div className={s.split}>
      <div>
        <div className={s.topRow}>
          <input
            className={s.filter}
            value={filter}
            placeholder="筛套餐（id / 显示名）"
            aria-label="筛选套餐"
            onChange={(e) => setFilter(e.target.value)}
          />
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
              >
                <span className={s.rowName}>{b.id}</span>
                <span className={s.rowMeta}>{b.assetRefs.length} 个文件</span>
                {b.users.length > 0 ? (
                  <span className={s.rowMeta}>{b.users.length} 个版本</span>
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
                <div className={s.groupHead}>MKP 预设（{cur.assetRefs.filter((r) => r.kind !== 'slicerProfile').length}）</div>
                {/* MKP 预设不建资产条目（doc §12.5）—— 这一组在真数据上恒空，照实说 */}
                {cur.assetRefs
                  .filter((r) => r.kind !== 'slicerProfile')
                  .map((r) => (
                    <div key={r.id} className={s.fileRow} {...fMenu.triggerProps(r.id)}>
                      <span className={`${s.fileName} ${s.mono}`}>{r.name || r.id}</span>
                      <span className={s.tag}>MKP</span>
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
                    一个都没装 —— MKP 预设不进资产库登记（doc §12.5），套餐里也写不了它
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
                  <div className={s.assignSel}>
                    <SelectField
                      label="选择版本"
                      value={assignUid}
                      options={[
                        { value: '', label: '选择版本…' },
                        ...allUids.map((u) => {
                          const now = bundleOf(u.uid)
                          const same = now.trim().toLowerCase() === cur.id.toLowerCase()
                          return {
                            value: u.uid,
                            label: u.uid,
                            note: same ? '已经指着它' : now || '还没配（走机型默认）',
                          }
                        }),
                      ]}
                      onChange={setAssignUid}
                    />
                  </div>
                  <button
                    type="button"
                    className={s.btn}
                    disabled={
                      !assignUid ||
                      bundleOf(assignUid).trim().toLowerCase() === cur.id.toLowerCase()
                    }
                    title={
                      assignUid &&
                      bundleOf(assignUid).trim().toLowerCase() === cur.id.toLowerCase()
                        ? '它已经指着这个套餐了'
                        : undefined
                    }
                    onClick={() => void assign(assignUid, cur.id)}
                  >
                    改指向
                  </button>
                </div>
              </div>
              <p className={s.note}>
                真源关系是<strong>一版一套</strong>：每个版本指向自己的套餐，改了指向之后
                生成与发布跟着走（后端判据，不在前端）。
              </p>
            </div>
          </div>
        )}
      </div>

      <ContextMenu at={fMenu.at} entries={fileEntries} onClose={fMenu.close} />
      <ContextMenu at={uMenu.at} entries={userEntries} onClose={uMenu.close} />

      <BundleResourcesModal
        open={resOpen !== null}
        bundle={list.bundles.find((b) => b.id === resOpen) ?? null}
        assets={assets}
        onCancel={() => setResOpen(null)}
        onConfirm={(mkpIds, slicerIds) => {
          if (resOpen === null) return
          void setRefs(
            resOpen,
            [...mkpIds, ...slicerIds],
            `已把套餐 ${resOpen} 的清单更新（MKP ${mkpIds.length} · 切片器 ${slicerIds.length}）`,
          )
          setResOpen(null)
        }}
      />
    </div>
  )
}
