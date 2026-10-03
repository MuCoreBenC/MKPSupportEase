/**
 * 「资产库」页 —— C14 版式移植（feat/b05-14b-c14-port，stock 视角落地）。
 *
 * 这一页的核心不是列表，是**引用可见**：一条资产被哪些套餐装着、被哪些机型
 * 当图 / 图标用，改身份 / 删除之前必须看得见。反查是命令（`wb.assetUsage`），
 * 不是前端现算的。
 *
 * # 这一页的正源是资产域定义（`presets/assets.toml`）
 *
 * 不是 `wb_stock` 那份上游交付镜像 —— 套餐 `assetRefs` 与这里同一 id 空间，
 * 预览、归属、三轴筛选也都在资产域。C14 的资产页把它与盘点混在一份数据里，
 * 产品侧两份各有各的命令。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的
 *
 *  - **切片器三根轴在后端**（C14 第二十八轮：BBS/Orca · 喷嘴 · 层高）：
 *   `slicer` 是登记字段；喷嘴 / 层高由后端从路径与文件名派生（资产定义刻意
 *   不存它们 —— 不存第二份真相），筛选判据也在后端。
 *  - **删除走真反查**：没人引用才删得掉；有引用时那个框是**拦截页**，
 *    chips 指着谁在用、给出去哪解除的路（C05 的闭环）。
 *  - **「被这些版本当 MKP」不渲染**：产品模型里版本不直接引用资产
 *   （版本 → 套餐 → 资产是间接的，Task 9.4 的裁决），照实说，不画恒空的一行。
 *  - **导入没有接**：真后端这一步是把文件放进资源目录再登记，命令没有 ——
 *    按钮不渲染（待裁决项记在 C14-PORT-PLAN §4）。
 *  - **大小 / 修改时间不显示**：资产定义刻意不存它们（那是交付物的属性，
 *    发布时按真实字节算，Task 13.6）—— 原型里那格本来就标着「演示值」。
 */
import { useCallback, useEffect, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type { AssetInspectView, AssetList, AssetView, Words } from '../api'
import { ContextMenu } from '../components/menu'
import type { ContextMenuEntry } from '../components/menu/types'
import { useContextMenu } from '../components/menu/useContextMenu'
import ModalC14 from '../c14/ModalC14'
import { locateAnchor } from '../c14/locate'
import { useSplitRail } from '../c14/SplitRail'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import s from '../c14.module.css'

interface Props {
  words: Words
  /** 外壳的「后端状态变过了」计数：撤销 / 重做 / 保存之后 +1，页面据此重取 */
  tick: number
  /** 跨页跳转带来的预选（套餐页文件行的「在资产库里看…」） */
  initialSel?: string | null
  /** 身份筛选的预选（检查报告的「孤儿文件」跳过来就替人筛好「可选」，C14 第二十七轮） */
  initialAssign?: string | null
  onGoto: (view: string, focus?: GotoFocus) => void
  /** 草稿写入口（改交付身份走它 —— 可见性是唯一进参数草稿的动作，带撤销） */
  onApply: (label: string, patches: import('../api').Patch[]) => Promise<void>
}

const KIND_LABEL: Record<string, string> = {
  image: '机型图',
  icon: '图标',
  model: '模型',
  slicerProfile: '切片器',
  mkPreset: 'MKP 预设',
}

/** 字节数给人看的写法（检查面板「大小」那一格）：B / KB / MB 一位小数 */
function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

/** 筛选轴的类型化包装 —— 全部走命令参数，本地不复算。
 *  `image`（整机图）**回到台账**：作者 2026-10-03 改判 —— 第三刀把它挪进客户端
 *  源码是错的（不会编程的用户改不了图）。它仍然**不进云端交付**，但必须
 *  在工作台看得见、选得着 */
type Kind = 'all' | 'image' | 'icon' | 'model' | 'slicerProfile' | 'mkPreset'
/** 交付身份四态（作者 2026-10-03 定的模型）：进套餐 / 可选 / 随包 / 仅归档 */
type Assign = 'all' | 'inBundle' | 'optional' | 'bundled' | 'archiveOnly'

export default function AssetsPage({ words, tick, initialSel, initialAssign, onGoto, onApply }: Props) {
  const [list, setList] = useState<AssetList | null>(null)
  const [usage, setUsage] = useState<Awaited<ReturnType<typeof wb.assetUsage>> | null>(null)
  /** 检查面板（第四刀）：盘上那个文件的样子 —— 选中一条才问（重字段要读真实字节） */
  const [inspect, setInspect] = useState<AssetInspectView | null>(null)
  const [sel, setSel] = useState<string | null>(initialSel ?? null)
  const [kind, setKind] = useState<Kind>('all')
  const [assign, setAssign] = useState<Assign>((initialAssign as Assign) ?? 'all')
  /** 切片器的三根轴（C14 第二十八轮）：只在 kind = 切片器时生效，换走后留着不丢（粘性） */
  const [slicer, setSlicer] = useState<'' | 'bbs' | 'orca'>('')
  const [nozzle, setNozzle] = useState('')
  const [layer, setLayer] = useState('')
  const [q, setQ] = useState('')
  const [confirm, setConfirm] = useState<'archive' | 'delete' | null>(null)
  const [pageErr, setPageErr] = useState<string | null>(null)
  const aMenu = useContextMenu<string>()
  /** 左栏宽度可拖（作者 2026-10-03，同参数台） */
  const bodyRef = useRef<HTMLDivElement | null>(null)
  const rail = useSplitRail('assets', bodyRef)

  const load = useCallback(
    (k: Kind, a: Assign, sl: string, nz: string, ly: string, query: string) => {
      void (async () => {
        try {
          const l = await wb.assets(
            k === 'all' ? null : k,
            k === 'slicerProfile' && sl ? sl : null,
            k === 'slicerProfile' && nz ? nz : null,
            k === 'slicerProfile' && ly ? ly : null,
            a === 'all' ? null : a,
            query || null,
          )
          setList(l)
        } catch (e) {
          setPageErr(isAppError(e) ? e.message : String(e))
        }
      })()
    },
    [],
  )
  useEffect(() => {
    load(kind, assign, slicer, nozzle, layer, q)
  }, [load, kind, assign, slicer, nozzle, layer, q, tick])

  /** 选中一条就问一次反查 —— 谁在用它必须看得见（详情卡与删除拦截页都读它） */
  useEffect(() => {
    if (!sel) {
      setUsage(null)
      return
    }
    let alive = true
    wb.assetUsage(sel)
      .then((u) => {
        if (alive) setUsage(u)
      })
      .catch(() => undefined)
    return () => {
      alive = false
    }
  }, [sel, list])

  /** 检查面板读数（第四刀）。**只依赖选中** —— 搜索框逐键重取清单时不再重读字节
   *  （重的是 SHA-256 / 3.2 MB 的模型）；换选中才重新问 */
  useEffect(() => {
    if (!sel) {
      setInspect(null)
      return
    }
    let alive = true
    setInspect(null)
    wb.assetInspect(sel)
      .then((v) => {
        if (alive) setInspect(v)
      })
      .catch(() => undefined)
    return () => {
      alive = false
    }
  }, [sel])

  const cur: AssetView | undefined = list?.assets.find((a) => a.id === sel)
  /** 读数对着当前选中吗（换选中 / 读失败时那一块说「正在读……」，不摆旧数据） */
  const insp = inspect !== null && cur !== undefined && inspect.id === cur.id ? inspect : null

  /* 预选的条目可能刚被删了，或者没带预选 —— 回落到第一个（C14 同一条）。
     只在没选中时自动挑，别跟人手点的 selection 打架 */
  useEffect(() => {
    if (!list || sel) return
    setSel(list.assets[0]?.id ?? null)
  }, [list, sel])

  /*
   * 「去处理」跳过来时：等清单读过、那一行确实在，滚过去 + 闪一下（见 `c14/locate.ts`）。
   * 孤儿文件那一路带的是「替人筛好可选」而不是选中某条（`initialSel` 为空），不定位。
   */
  const locatedRef = useRef(false)
  useEffect(() => {
    if (locatedRef.current) return
    if (!initialSel || !list || !list.assets.some((a) => a.id === initialSel)) return
    locatedRef.current = true
    locateAnchor(`t-asset-${initialSel}`, { row: true })
  }, [initialSel, list])
  /** 反查结果对着当前选中吗 —— 右键「删除…」会先换选中再开框，旧结果不能拿来说话 */
  const usageReady = usage !== null && cur !== undefined && usage.id === cur.id
  const inUse =
    usageReady && (usage?.machines.length ?? 0) + (usage?.bundles.length ?? 0) > 0

  /** 写可见性（草稿入口 + 刷新）。入口闸在 setVis：有人引用的「仅归档」先过确认框 */
  const applyVis = async (id: string, v: 'menu' | 'archiveOnly') => {
    await onApply(`把 ${id} 设为${v === 'menu' ? '可选' : '仅归档'}`, [
      { kind: 'setVisibility', fileId: id, visibility: v },
    ])
    toasts.push(`已把 ${id} 设为${v === 'menu' ? '可选' : '仅归档'}（改了先进草稿，保存才落盘）`)
    /* 可见性连着四态与页脚计数 —— 重新读一遍（草稿态在会话里，重取就看得见） */
    load(kind, assign, slicer, nozzle, layer, q)
  }

  /** 改交付档位（download ↔ bundled）。即时落盘（清单编辑，不走参数草稿） */
  const applyDelivery = async (id: string, v: 'download' | 'bundled') => {
    try {
      setList(await wb.setAssetDelivery(id, v))
      toasts.push(
        v === 'bundled'
          ? `已把 ${id} 设为随包（构建期打进客户端，客户端不再单独下载）`
          : `已把 ${id} 改回按需下载（客户端预设页看得到、可手动下）`,
      )
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 改交付身份。唯一走参数草稿的动作：进外壳那条撤销栈，「仅归档」有拦截确认 */
  const setVis = (id: string, v: 'menu' | 'archiveOnly') => {
    if (v === 'archiveOnly' && inUse) {
      setConfirm('archive')
      return
    }
    void applyVis(id, v)
  }

  /** 删除。反查在数据层；有引用的话后端整次拒绝 —— 界面先自己拦一道，把话说完 */
  const doDelete = async (id: string) => {
    try {
      const next = await wb.removeAsset(id)
      setList(next)
      toasts.push(`已删除 ${id}（登记条目没了，文件本体还在资产目录里）`)
      setConfirm(null)
      setSel(null)
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      setConfirm(null)
    }
  }

  /*
   * 资产行右键（C06）。不引入新动作：删除、改可见性详情卡上都已经有按钮，
   * 右键只是熟手的第二条路，文案也照抄按钮上的字。
   */
  const assetEntries: ContextMenuEntry[] = aMenu.target
    ? (() => {
        const id = aMenu.target
        const row = list?.assets.find((a) => a.id === id)
        return [
          {
            id: 'vis',
            label: row?.identity === 'archiveOnly' ? '改回可选' : '改成仅归档…',
            onSelect: () => {
              setSel(id)
              void setVis(id, row?.identity === 'archiveOnly' ? 'menu' : 'archiveOnly')
            },
          },
          { separator: true },
          {
            id: 'delete',
            label: '删除…',
            danger: true,
            /* 不灰掉、也不静默 —— 点下去弹的是拦截页或确认框，它会说清楚谁在用 */
            onSelect: () => {
              setSel(id)
              setConfirm('delete')
            },
          },
        ]
      })()
    : []

  if (!list) {
    if (pageErr) return <p className="wb-todo">{pageErr}</p>
    return <p className="wb-todo">正在读资产定义……</p>
  }

  const listeds = list.assets

  return (
    <div className={s.split} ref={bodyRef} style={rail.style}>
      <div>
        <div className={s.topRow}>
          <input
            className={s.inp}
            placeholder="搜索名称 / id"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>
        {/*
         * 筛选行**显式分两排**（C14 第二十八轮，作者：「右边的全部身份要放到下面」）——
         * 上排 = 我在看哪类文件；下排 = 交付身份。选中「切片器」再亮出三根轴。
         */}
        <div className={s.bar}>
          {(
            [
              ['all', '全部'],
              ['image', '机型图'],
              ['icon', '图标'],
              ['model', '模型'],
              ['slicerProfile', '切片器'],
              ['mkPreset', 'MKP 预设'],
            ] as const
          ).map(([k, label]) => (
            <button
              key={k}
              type="button"
              className={`${s.btn} ${s.btnSm} ${kind === k ? s.btnOn : ''}`}
              onClick={() => setKind(k)}
            >
              {label}
            </button>
          ))}
        </div>
        <div className={s.bar}>
          {(
            [
              ['all', '全部身份'],
              ['inBundle', words.identity.inBundle.label],
              ['optional', words.identity.optional.label],
              ['bundled', words.identity.bundled.label],
              ['archiveOnly', words.identity.archiveOnly.label],
            ] as const
          ).map(([k, label]) => (
            <button
              key={k}
              type="button"
              className={`${s.btn} ${s.btnSm} ${assign === k ? s.btnOn : ''}`}
              onClick={() => setAssign(k)}
            >
              {label}
            </button>
          ))}
        </div>
        {kind === 'slicerProfile' && (
          <div className={s.bar}>
            {/*
             * 每根轴是一个**不可拆的组**（inline-flex）：左栏窄，折行只许发生在
             * 组与组之间，「喷嘴」仨字不能和自己的选项被拆到两行。
             */}
            <span className={s.axisGroup}>
              <span className={s.barLabel}>切片器</span>
              {(
                [
                  ['', '全部'],
                  ['bbs', 'BBS'],
                  ['orca', 'Orca'],
                ] as const
              ).map(([v, label]) => (
                <button
                  key={v || 'all'}
                  type="button"
                  className={`${s.btn} ${s.btnSm} ${slicer === v ? s.btnOn : ''}`}
                  onClick={() => setSlicer(v)}
                >
                  {label}
                </button>
              ))}
            </span>
            <span className={s.axisGroup}>
              <span className={s.barLabel}>喷嘴</span>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm} ${nozzle === '' ? s.btnOn : ''}`}
                onClick={() => setNozzle('')}
              >
                全部
              </button>
              {list.nozzles.map((v) => (
                <button
                  key={v}
                  type="button"
                  className={`${s.btn} ${s.btnSm} ${nozzle === v ? s.btnOn : ''}`}
                  onClick={() => setNozzle(v)}
                >
                  {v}
                </button>
              ))}
            </span>
            <span className={s.axisGroup}>
              <span className={s.barLabel}>层高</span>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm} ${layer === '' ? s.btnOn : ''}`}
                onClick={() => setLayer('')}
              >
                全部
              </button>
              {list.layers.map((v) => (
                <button
                  key={v}
                  type="button"
                  className={`${s.btn} ${s.btnSm} ${layer === v ? s.btnOn : ''}`}
                  onClick={() => setLayer(v)}
                >
                  {v}
                </button>
              ))}
            </span>
          </div>
        )}
        <div className={s.list}>
          {listeds.map((a) => (
            <button
              key={a.id}
              type="button"
              /* 「去处理」定位的锚点（locateAnchor 按它滚 + 闪） */
              id={`t-asset-${a.id}`}
              className={`${s.row} ${sel === a.id ? s.rowOn : ''}`}
              onClick={() => setSel(a.id)}
              {...aMenu.triggerProps(a.id)}
            >
              <span className={`${s.rowName} ${s.mono}`}>{a.display}</span>
              <span className={s.tag}>{KIND_LABEL[a.kind]}</span>
              {/* 同名版本会撞（三台机型都有「标准版」）—— 行上补一个机型小签 */}
              {a.machineId && a.kind === 'mkPreset' && (
                <span className={`${s.tag} ${s.tagGhost}`}>{a.machineId}</span>
              )}
              {a.buildState !== null && (
                /* 生成状态（作者 2026-10-03）：没生成过 = 待生成；参数改了没重新生成 = 待更新。
                   词与判据都取生成页那一套 —— 这里只是把同一件事显示在资产库里 */
                <span
                  className={s.tag}
                  title={words.build[a.buildState]?.explain ?? '这一版的产物还没生成'}
                >
                  {words.build[a.buildState]?.label ?? a.buildState}
                </span>
              )}
              {/* 交付身份四态一行一个词（进套餐 / 可选 / 随包 / 仅归档）——
                  不再「已分配/可选/仅归档 + 随包」两套轴各说各的 */}
              <span
                className={s.tag}
                title={words.identity[a.identity].explain ?? undefined}
              >
                {words.identity[a.identity].label}
              </span>
            </button>
          ))}
        </div>
        <div className={s.sum}>
          {listeds.length} / {list.total} 个条目 · 可选 {list.optionalCount} · 仅归档{' '}
          {list.archiveCount}
        </div>
        {!listeds.length && (
          <div className={s.sum}>
            没有匹配的文件{' '}
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => {
                setQ('')
                setKind('all')
                setAssign('all')
                setSlicer('')
                setNozzle('')
                setLayer('')
              }}
            >
              清空筛选
            </button>
          </div>
        )}
      </div>

      <div className={s.detail}>
        {!cur ? (
          <div className={s.empty}>
            <h2>选一个文件</h2>
            <p>{words.empty.selectAsset}</p>
          </div>
        ) : (
          <div className={s.card}>
            <div className={s.cardHead}>
              <h2>{cur.display}</h2>
              <span className={s.cardNote}>
                {cur.kind === 'slicerProfile'
                  ? `切片器 · ${(cur.slicer ?? 'bbs') === 'orca' ? 'Orca' : 'BBS'}${cur.profile ? ` · ${cur.profile}` : ''}`
                  : KIND_LABEL[cur.kind]}
              </span>
              {/*
               * 「在访达中显示」（第四刀）：**前端只传资产 id**，路径由后端算
               * （参照客户端 `reveal_in_folder` 的纪律：只读、只开窗口、不碰状态）。
               * 文件不在时不给一个必被拒的按钮 —— 灰掉并把原因写在 title 里
               */}
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={insp === null || !insp.exists}
                title={
                  insp === null
                    ? '正在读文件信息……'
                    : insp.exists
                      ? '打开系统文件管理器并选中这个文件'
                      : '文件不在，没什么可显示的（期望路径在「资产检查」里）'
                }
                onClick={() => {
                  void (async () => {
                    try {
                      await wb.revealAsset(cur.id)
                    } catch (e) {
                      toasts.push(isAppError(e) ? e.message : String(e))
                    }
                  })()
                }}
              >
                在访达中显示
              </button>
              <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setConfirm('delete')}>
                删除
              </button>
            </div>
            <div className={s.cardBody}>
              <div className={s.kv}>
                <span className={s.kvKey}>id</span>
                <span className={`${s.kvVal} ${s.mono}`}>{cur.id}</span>
                {/* 名字的真源摆出来：MKP 预设 = 版本名（去版本页改）；切片器 = 文件名 */}
                {cur.kind === 'mkPreset' && (
                  <>
                    <span className={s.kvKey}>名字来自</span>
                    <span className={s.kvVal}>
                      版本名称
                      <button
                        type="button"
                        className={`${s.btn} ${s.btnSm}`}
                        onClick={() =>
                          onGoto('machines', {
                            machineId: cur.machineId,
                            uid: `${cur.machineId}/${cur.versionId ?? ''}`,
                            key: null,
                          })
                        }
                        title="去「机型与版本」页改这一版的名字（资产库显示的是同一个值）"
                      >
                        在版本页改
                      </button>
                      {cur.name !== cur.display && (
                        <span className={s.cardNote}>台账登记名：{cur.name}</span>
                      )}
                    </span>
                  </>
                )}
                <span className={s.kvKey}>台账路径</span>
                <span className={`${s.kvVal} ${s.mono}`}>
                  {cur.path || <span className={s.kvDim}>（生成产物，没有源 path —— 看下面的「产物路径」）</span>}
                </span>
                <span className={s.kvKey}>适用机型</span>
                <span className={s.kvVal}>
                  {cur.machineId ? (
                    <span className={s.chip}>{cur.machineId}</span>
                  ) : (
                    <span className={s.kvDim}>未标机型</span>
                  )}
                </span>
                {cur.kind === 'slicerProfile' && (
                  <>
                    <span className={s.kvKey}>切片器轴</span>
                    <span className={s.kvVal}>
                      {(cur.slicer ?? 'bbs') === 'orca' ? 'Orca' : 'BBS'} · 喷嘴{' '}
                      {cur.nozzle ?? '—'} · 层高 {cur.layer ?? '—'}
                      <span className={s.cardNote}>（从路径与文件名读出）</span>
                    </span>
                  </>
                )}
                <span className={s.kvKey}>文件</span>
                <span className={s.kvVal}>
                  {!cur.present ? (
                    <span className={s.kvDim}>登记了，文件还没搬进来</span>
                  ) : cur.kind === 'image' || cur.kind === 'icon' ? (
                    <img
                      src={encodeURI(cur.url)}
                      alt={cur.display}
                      loading="lazy"
                      style={{ maxWidth: 200, maxHeight: 120, display: 'block', borderRadius: 4 }}
                    />
                  ) : (
                    <span className={`${s.kvVal} ${s.mono}`}>{cur.path.split('/').pop()}</span>
                  )}
                </span>
              </div>

              <div className={s.refs} style={{ marginTop: 12 }}>
                <div className={s.refRow}>
                  <b>被这些套餐装着</b>
                  <span className={s.chips}>
                    {usage && usage.bundles.length ? (
                      usage.bundles.map((b) => (
                        /* 反查要「走得到」，不只是「看得见」 */
                        <button
                          key={b}
                          type="button"
                          className={s.chip}
                          title="去套餐页看它"
                          onClick={() => onGoto('bundles', { machineId: null, uid: b, key: null })}
                        >
                          {b}
                        </button>
                      ))
                    ) : (
                      <span className={s.kvDim}>没有套餐装它</span>
                    )}
                  </span>
                </div>
                <div className={s.refRow}>
                  <b>被这些机型引用着</b>
                  <span className={s.chips}>
                    {usage && usage.machines.length ? (
                      usage.machines.map((m) => (
                        <button
                          key={m}
                          type="button"
                          className={s.chip}
                          title="去机型页看它"
                          onClick={() => onGoto('machines', { machineId: m, uid: null, key: null })}
                        >
                          {m}
                        </button>
                      ))
                    ) : (
                      <span className={s.kvDim}>没有机型用它</span>
                    )}
                  </span>
                </div>
                <p className={s.note} style={{ margin: 0 }}>
                  版本不直接引用资产 —— 它经由套餐拿到（一版一套），所以「被谁当 MKP 用」
                  在套餐那一份上看。
                </p>
              </div>

              {/*
               * —— 资产检查（第四刀，作者锁定的字段清单）——
               * 详情卡从「登记表」变成「资产检查面板」：盘上那个文件到底是什么样，
               * 它自己说。这几格的读数单独一条命令、**选中才问**（SHA-256 与大小要读
               * 真实字节，模型 3.2 MB —— 并进列表就是每敲一个字读几 MB）。
               */}
              <div className={s.group}>
                <div className={s.groupHead}>资产检查</div>
                {insp === null ? (
                  <p className={s.note} style={{ marginTop: 0 }}>
                    正在读盘上那个文件……
                  </p>
                ) : (
                  <div className={s.kv}>
                    <span className={s.kvKey}>真实文件名</span>
                    <span className={`${s.kvVal} ${s.mono}`}>
                      {insp.fileName}
                      {cur.kind === 'mkPreset' && (
                        <span className={s.cardNote}>（产物名，命名规则算出）</span>
                      )}
                    </span>
                    <span className={s.kvKey}>{insp.exists ? '源文件路径' : '期望路径'}</span>
                    <span className={`${s.kvVal} ${s.mono}`}>
                      {insp.absPath}
                      {!insp.exists && (
                        <span className={s.cardNote}>（文件不在，这就是它该在的地方）</span>
                      )}
                    </span>
                    {insp.productPath !== null && (
                      <>
                        <span className={s.kvKey}>产物路径</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {insp.productPath}
                          <span className={s.cardNote}>（相对仓库根）</span>
                        </span>
                      </>
                    )}
                    <span className={s.kvKey}>格式</span>
                    <span className={`${s.kvVal} ${s.mono}`}>
                      {insp.format ?? <span className={s.kvDim}>没有扩展名</span>}
                    </span>
                    <span className={s.kvKey}>尺寸</span>
                    <span className={s.kvVal}>
                      {insp.width !== null && insp.height !== null ? (
                        <span className={s.mono}>
                          {insp.width} × {insp.height}
                        </span>
                      ) : (
                        <span className={s.kvDim}>不是图片 / 读不出</span>
                      )}
                    </span>
                    <span className={s.kvKey}>大小</span>
                    <span className={s.kvVal}>
                      {insp.bytes !== null ? formatBytes(insp.bytes) : <span className={s.kvDim}>—</span>}
                    </span>
                    <span className={s.kvKey}>SHA-256</span>
                    <span className={`${s.kvVal} ${s.mono}`}>
                      {insp.sha256 ?? <span className={s.kvDim}>—</span>}
                    </span>
                    <span className={s.kvKey}>状态</span>
                    <span className={s.kvVal}>
                      {cur.kind === 'mkPreset' && cur.buildState !== null
                        ? `${words.build[cur.buildState]?.label ?? cur.buildState} · 产物文件${insp.exists ? '在' : '不在'}`
                        : `登记着 · 文件${insp.exists ? '在' : '不在'}`}
                    </span>
                  </div>
                )}
                <p className={s.note}>
                  这几格是盘上那个文件的读数（SHA-256 与大小要读真实字节，所以选中才问一次）。
                  台账里不存这些 —— 不存第二份真相。
                </p>
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>
                  交付身份 ·{' '}
                  <span className={s.cardNote} style={{ display: 'inline' }}>
                    {words.identity[cur.identity].label}
                  </span>
                </div>
                <p className={s.note} style={{ marginTop: 0 }}>
                  {words.identity[cur.identity].explain}
                </p>
                <div className={s.bar}>
                  {/*
                   * 三个动作**恒定给出、按当前态高亮/禁用**（作者 2026-10-03：
                   * 「怎么不同文件不同，我觉得这样也不应该限制」—— 按钮对每一类
                   * 都一样；能不能成由后端判（例如 MKP 预设的产物在交付根，构建期
                   * 没有复制路径 → 那一格点了会如实被拒，而不是界面上先不给）。
                   */}
                  {cur.identity === 'inBundle' && (
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      onClick={() => onGoto('bundles', { machineId: null, uid: usage?.bundles[0] ?? null, key: null })}
                      disabled={!usage?.bundles.length}
                      title={usage?.bundles.length ? undefined : '反查还没回来'}
                    >
                      去套餐页看谁装着它
                    </button>
                  )}
                  <button
                    type="button"
                    className={`${s.btn} ${cur.identity === 'bundled' ? s.btnOn : ''}`}
                    disabled={cur.identity === 'bundled'}
                    onClick={() => void applyDelivery(cur.id, 'bundled')}
                  >
                    设为随包
                  </button>
                  <button
                    type="button"
                    className={`${s.btn} ${cur.identity === 'archiveOnly' ? s.btnOn : ''}`}
                    disabled={cur.identity === 'archiveOnly'}
                    onClick={() => void setVis(cur.id, 'archiveOnly')}
                  >
                    设为仅归档
                  </button>
                  <button
                    type="button"
                    className={`${s.btn} ${cur.identity === 'optional' || cur.identity === 'inBundle' ? s.btnOn : ''}`}
                    disabled={cur.identity !== 'archiveOnly'}
                    onClick={() => {
                      /* 随包 → 可选 = 改回按需下载（动交付档位）；仅归档 → 可选 = 解除归档 */
                      if (cur.delivery === 'bundled') void applyDelivery(cur.id, 'download')
                      else void setVis(cur.id, 'menu')
                    }}
                    title={
                      cur.delivery === 'bundled'
                        ? '改回按需下载：客户端预设页看得到、可手动下'
                        : '解除归档：回到可选'
                    }
                  >
                    {cur.delivery === 'bundled' ? '改回按需下载' : '改回可选'}
                  </button>
                </div>
                <p className={s.note}>
                  「仅归档」= 客户端完全不知道这个文件存在；「随包」= 构建期把文件带进客户端，
                  不下载不更新、页面也不出现；「进套餐」在套餐页管 —— 那是「哪一版配发什么」的事。
                </p>
              </div>
            </div>
          </div>
        )}
      </div>

      <ContextMenu at={aMenu.at} entries={assetEntries} onClose={aMenu.close} />

      {/* 反查没回来之前不亮「删除」—— 旧结果不能拿来说话；回来之后：
          有引用时它是拦截页（说清楚谁在用、去哪解除），没引用才是确认框 */}
      <ModalC14
        open={confirm !== null}
        title={confirm === 'delete' ? '删除资产' : '设为仅归档'}
        subtitle={confirm === 'archive' ? '客户端从此完全看不到它' : undefined}
        size="md"
        onClose={() => setConfirm(null)}
        footer={
          confirm === 'delete' && !usageReady ? (
            <>
              <span className={s.grow} />
              <button type="button" className={s.btn} onClick={() => setConfirm(null)}>
                取消
              </button>
            </>
          ) : confirm === 'delete' && inUse ? (
            <>
              <span className={s.grow} />
              <button type="button" className={s.btn} onClick={() => setConfirm(null)}>
                知道了
              </button>
            </>
          ) : (
            <>
              <span className={s.grow} />
              <button type="button" className={s.btn} onClick={() => setConfirm(null)}>
                取消
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnDanger}`}
                onClick={() => {
                  if (!confirm || !cur) return
                  /* 确认框里直走写路径 —— 这里的人已经看过「静默漏洞」那段话了，
                     再过一遍入口闸只会把弹窗打回给自己（循环） */
                  if (confirm === 'archive') {
                    void applyVis(cur.id, 'archiveOnly')
                  } else {
                    void doDelete(cur.id)
                  }
                  setConfirm(null)
                }}
              >
                {confirm === 'delete' ? '删除' : '设为仅归档'}
              </button>
            </>
          )
        }
      >
        {confirm === 'delete' && !usageReady ? (
          <p className={s.note} style={{ margin: 0 }}>
            正在反查谁在用它……
          </p>
        ) : confirm === 'delete' && inUse ? (
          <div className={s.warn}>
            <div className={s.warnTitle}>还有地方在用，不能删</div>
            <div className={s.warnDetail}>{words.disabled.deleteAssetInUse}</div>
            <div className={s.chips}>
              {usage?.bundles.map((b) => (
                <span key={b} className={s.chip}>
                  套餐 {b}
                </span>
              ))}
              {usage?.machines.map((m) => (
                <span key={m} className={s.chip}>
                  机型 {m}
                </span>
              ))}
            </div>
            {/* 闭环的最后一环：不光说「不能删」，还给出去哪解除的路 */}
            {usage && usage.bundles.length > 0 && (
              <div className={s.bar} style={{ marginBottom: 0 }}>
                <button
                  type="button"
                  className={s.btn}
                  onClick={() => {
                    setConfirm(null)
                    onGoto('bundles', { machineId: null, uid: usage.bundles[0], key: null })
                  }}
                >
                  去套餐页，把文件从 {usage.bundles[0]} 移出
                </button>
              </div>
            )}
            {usage && usage.machines.length > 0 && (
              <p className={s.note} style={{ margin: 0 }}>
                机型引用在机型文件的 image / icon 字段上 —— 去机型与版本页能改到它。
              </p>
            )}
          </div>
        ) : (
          <p className={s.note} style={{ margin: 0 }}>
            {confirm === 'archive'
              ? '设为「仅归档」后客户端完全看不到它。而它现在还在套餐里 —— 那正是那条静默漏洞：客户看得到套餐、拿不到文件、还不报错。'
              : '没有套餐装它，也没有机型引用它。删掉之后这份登记就没了（文件本体留在资产目录里，发布也不会再带上它）。'}
          </p>
        )}
      </ModalC14>
      {rail.handle}
    </div>
  )
}
