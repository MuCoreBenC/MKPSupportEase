/*
 * 禁区编辑器（2026-10-03，作者照旧面板 mkppanel 的 ZoneEditorModal 移植）。
 *
 * 禁区 = 床身上「碰不得」的一块区域（擦嘴区 / 挡块 / 卡舌那几处），一串点围成的多边形。
 * 它住在 `presets/forbidden_zones/<机型>.toml`，一台机器一份（没有文件 = 没有禁区）。
 *
 * # 坐标系（**只在这里换算一次**）
 *
 * 机器坐标：x 向右、**y 向上**，原点是床身的**前左角**。
 * 画布（SVG）：y 向下。所以 `svgY = depth - y`。
 *
 * ★ 别照搬 `src/app/params/TowerMap.tsx` 那套：它的 `bedPointOf` 算的是**可打印区
 * （plate frame）**坐标，会多减一个 `frame` 偏移；禁区是**板 / 床身**口径，
 * 整个画布就是 `0 0 bedSize.width bedSize.depth`（旧面板同口径）。
 *
 * # 干净画布
 *
 * 默认只画床身轮廓 + 网格 + 多边形的边；悬停 / 选中只让**线段变色**，不冒出点。
 * 画布上**只有亮着的那一个点**：正在右边改它的坐标、正拖着它、或刚加上的那个 ——
 * 实心放大，头顶跟一块圆角坐标牌。右边坐标行用的是参数台同款 `NumberField`。
 *
 * # 加点只有两条路（2026-10-03 作者那刀：画布空白处点击**不再**加点）
 *
 * 旧版"点画布任意处 = 加顶点"有两个坑：想拖顶点松手时浏览器补发的 click 会
 * 落进加点逻辑（每拖一次多一个点）；随手一点床身就多出个野点。现在：
 * 点选中块的**边**插入顶点（插在点的那条边上），或列表里的「添加点」（加在
 * 收口边中点）。拖 / 删不变：拖顶点挪位置，右键顶点删掉。
 *
 * # 落盘
 *
 * 「确认保存」一次写整份（`wb.setMachineZones`）。清到 0 块 = **删掉那个文件**
 * （后端口径：清空是删文件，不是留一个空文件）—— 所以要先问一句。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent } from 'react'

import { isAppError, wb } from '../api'
import type { MachineView, ZonePolygon } from '../api'
import { NumberField } from '../components/field'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import s from './ZoneEditorModal.module.css'

interface Props {
  /** 正在编辑哪台（`null` = 关着） */
  machineId: string | null
  machine: MachineView | null
  onClose: () => void
  onSaved: (next: Awaited<ReturnType<typeof wb.setMachineZones>>) => void
}

/** 一块多边形最少几个点（少于 3 个围不出面 —— 后端也拦这一条） */
const MIN_POINTS = 3
const ZOOM_MIN = 0.5
const ZOOM_MAX = 8
/** 拖顶点收到 0.1mm：连续量不收小数，保存下来的文件里会出现一长串浮点尾巴 */
const round1 = (n: number) => Math.round(n * 10) / 10

type P = [number, number]

/** 深拷贝（草稿与外面那份互不影响） */
const cloneZones = (zs: ZonePolygon[]): P[][] => zs.map((z) => z.points.map((p) => [p[0], p[1]] as P))

export default function ZoneEditorModal({ machineId, machine, onClose, onSaved }: Props) {
  const bed = machine?.dimensions?.bedSize ?? { width: 256, depth: 256 }

  /* —— 草稿 + 历史栈（编辑器内部，**不进外壳草稿栈**）—— */
  const [zones, setZones] = useState<P[][]>(() => cloneZones(machine?.zones ?? []))
  const [past, setPast] = useState<P[][][]>([])
  const [future, setFuture] = useState<P[][][]>([])
  const [sel, setSel] = useState<number | null>(null)
  /** 亮着的那一个点：正在右边改它坐标、正拖着它、或刚加上的那个（选中块里的下标）。
   *  画布上**只有这一个**点 —— 悬停只让线段变色，不冒出别的点 */
  const [activePt, setActivePt] = useState<number | null>(null)
  const [zoom, setZoom] = useState(1)
  const [busy, setBusy] = useState(false)
  /**
   * 正在拖第几块的哪个点。`before` 是拖拽起点的快照：第一次真挪动才压进历史栈
   * （点一下不挪 = 不留无意义的一步撤销）；`orig` 用来判断"真挪动了"。
   */
  const dragRef = useRef<{ zone: number; point: number; orig: P; before: P[][]; pushed: boolean } | null>(null)
  const svgRef = useRef<SVGSVGElement>(null)
  /** 光标处的机器坐标（状态栏显示用） */
  const [cursor, setCursor] = useState<P | null>(null)

  /* 每次打开（或换了一台）都重新起草：取消掉的不该还留着 */
  useEffect(() => {
    setZones(cloneZones(machine?.zones ?? []))
    setPast([])
    setFuture([])
    setSel(machine?.zones.length ? 0 : null)
    setActivePt(null)
    setZoom(1)
    setBusy(false)
    setCursor(null)
  }, [machineId, machine])

  /** 推进一步历史（改动前调；`setZones` 的下一帧就是新状态） */
  const commit = useCallback(
    (next: P[][]) => {
      setPast((p) => [...p.slice(-49), zones])
      setFuture([])
      setZones(next)
    },
    [zones],
  )

  const undo = useCallback(() => {
    setPast((p) => {
      if (!p.length) return p
      const prev = p[p.length - 1]
      setFuture((f) => [zones, ...f])
      setZones(prev)
      setSel((cur) => (cur !== null && cur >= prev.length ? prev.length - 1 : cur))
      return p.slice(0, -1)
    })
  }, [zones])

  const redo = useCallback(() => {
    setFuture((f) => {
      if (!f.length) return f
      const next = f[0]
      setPast((p) => [...p, zones])
      setZones(next)
      setSel((cur) => (cur !== null && cur >= next.length ? next.length - 1 : cur))
      return f.slice(1)
    })
  }, [zones])

  /* 快捷键：⌘Z / ⇧⌘Z（与旧面板同一套；只在框开着时挂） */
  useEffect(() => {
    if (machineId === null) return
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey
      if (mod && e.key.toLowerCase() === 'z') {
        e.preventDefault()
        if (e.shiftKey) redo()
        else undo()
      }
      if (e.key === 'Delete' || e.key === 'Backspace') {
        // 删选中的那一块（不是删顶点 —— 顶点用右键 / 列表里的坐标行删）
        if (sel !== null && document.activeElement?.tagName !== 'INPUT') {
          e.preventDefault()
          commit(zones.filter((_, i) => i !== sel))
          setSel(null)
        }
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [machineId, undo, redo, commit, zones, sel])

  /* —— 坐标换算（只在这个文件里写一次；pointer / click 事件都带 clientXY）—— */
  const toBed = useCallback(
    (e: { clientX: number; clientY: number }): P | null => {
      const el = svgRef.current
      if (el === null) return null
      const r = el.getBoundingClientRect()
      const x = ((e.clientX - r.left) / r.width) * bed.width
      const y = bed.depth - ((e.clientY - r.top) / r.height) * bed.depth
      return [Math.max(0, Math.min(bed.width, x)), Math.max(0, Math.min(bed.depth, y))]
    },
    [bed.width, bed.depth],
  )

  const pointsAttr = (pts: P[]) => pts.map(([x, y]) => `${x},${bed.depth - y}`).join(' ')
  const flat = (zs: P[][]) => zs.map((points) => ({ points }))

  /* —— 画布操作 —— */
  /** 点边插入顶点：插在被点的那条边中间（比"画布上哪儿都能加"不容易出野点） */
  const insertPoint = (zi: number, after: number, at: P) => {
    const np: P = [round1(at[0]), round1(at[1])]
    commit(zones.map((q, i) => (i === zi ? [...q.slice(0, after + 1), np, ...q.slice(after + 1)] : q)))
    setActivePt(after + 1)
  }

  const onVertexDown = (zi: number, pi: number) => (e: ReactPointerEvent<SVGCircleElement>) => {
    e.stopPropagation()
    setSel(zi)
    setActivePt(pi)
    dragRef.current = {
      zone: zi,
      point: pi,
      orig: zones[zi][pi],
      before: zones.map((z) => z.map((q) => [...q] as P)),
      pushed: false,
    }
    setFuture([])
    svgRef.current?.setPointerCapture(e.pointerId)
  }

  const onCanvasMove = (e: ReactPointerEvent<SVGSVGElement>) => {
    const at = toBed(e)
    if (at === null) return
    setCursor([round1(at[0]), round1(at[1])])
    const d = dragRef.current
    if (d === null) return
    // 第一次真挪动了才把起点快照压进历史栈（一次拖拽 = 一步撤销）
    const snapped: P = [round1(at[0]), round1(at[1])]
    if (!d.pushed && (snapped[0] !== d.orig[0] || snapped[1] !== d.orig[1])) {
      d.pushed = true
      setPast((p) => [...p.slice(-49), d.before])
      setFuture([])
    }
    // 拖拽期间只改草稿（不进历史栈 —— 起点已经压过一次了）
    setZones((prev) =>
      prev.map((z, i) =>
        i === d.zone ? z.map((q, j) => (j === d.point ? snapped : q)) : z,
      ),
    )
  }

  const onCanvasUp = (e: ReactPointerEvent<SVGSVGElement>) => {
    const d = dragRef.current
    dragRef.current = null
    if (svgRef.current?.hasPointerCapture(e.pointerId)) svgRef.current.releasePointerCapture(e.pointerId)
    // 松手时光标若已不在那个顶点上，把它脚下的坐标牌收掉
    if (d !== null && activePt !== null) {
      const at = toBed(e)
      const pt = zones[d.zone]?.[d.point]
      const near = at !== null && pt !== undefined && Math.hypot(at[0] - pt[0], at[1] - pt[1]) < 4
      if (!near) setActivePt(null)
    }
  }

  /* —— 列表操作 —— */
  const addZone = () => {
    // 新块给一个床身中央的小方块（不然"添加"之后画布上什么都没有，人会以为坏了）
    const w = Math.min(40, bed.width / 4)
    const h = Math.min(40, bed.depth / 4)
    const cx = (bed.width - w) / 2
    const cy = (bed.depth - h) / 2
    const fresh: P[] = [
      [round1(cx), round1(cy)],
      [round1(cx + w), round1(cy)],
      [round1(cx + w), round1(cy + h)],
      [round1(cx), round1(cy + h)],
    ]
    commit([...zones, fresh])
    setSel(zones.length)
    setActivePt(null)
  }

  const removeZone = (i: number) => {
    commit(zones.filter((_, k) => k !== i))
    setSel((cur) => (cur === null ? null : cur === i ? null : cur > i ? cur - 1 : cur))
    setActivePt(null)
  }

  const duplicateZone = (i: number) => {
    const src = zones[i]
    if (src === undefined) return
    // 挪开一点点，免得叠在原件上看不见（床身太小时贴着边也不至于跑出去）
    const shift = Math.min(10, bed.width / 20)
    const copy: P[] = src.map(([x, y]) => [round1(Math.min(bed.width, x + shift)), round1(y)] as P)
    commit([...zones, copy])
    setSel(zones.length)
    setActivePt(null)
  }

  /** 列表里的「添加点」：加在末点→首点那条收口边的中点（跟点边插入同一口径） */
  const addPoint = (zi: number) => {
    const z = zones[zi]
    if (z === undefined || z.length < 2) return
    const a = z[z.length - 1]
    const b = z[0]
    commit(zones.map((q, i) => (i === zi ? [...q, [round1((a[0] + b[0]) / 2), round1((a[1] + b[1]) / 2)] as P] : q)))
    setActivePt(z.length)
  }

  const removePoint = (zi: number, pi: number) => {
    const z = zones[zi]
    if (z === undefined) return
    // 只剩 3 个点时删掉任何一个这块就不成形了 —— 直接问一句整块要不要删
    if (z.length <= MIN_POINTS) {
      toasts.push(`这一块只有 ${MIN_POINTS} 个点了 —— 再删就不成形，整块删掉用「删除选中」`)
      return
    }
    commit(zones.map((q, i) => (i === zi ? q.filter((_, j) => j !== pi) : q)))
  }

  const save = async () => {
    if (machineId === null) return
    const tooSmall = zones.findIndex((z) => z.length < MIN_POINTS)
    if (tooSmall >= 0) {
      toasts.push(`第 ${tooSmall + 1} 块只有 ${zones[tooSmall].length} 个点 —— 少于 ${MIN_POINTS} 个围不出面`)
      setSel(tooSmall)
      return
    }
    if (zones.length === 0 && (machine?.zoneCount ?? 0) > 0) {
      const yes = window.confirm(
        `这会删掉 ${machineId} 的禁区文件（${machine?.file.replace('.toml', '')} 那份整个没了）—— 继续？`,
      )
      if (!yes) return
    }
    setBusy(true)
    try {
      onSaved(await wb.setMachineZones(machineId, flat(zones)))
      toasts.push(zones.length ? `已写回 ${zones.length} 块禁区` : '已清空禁区（那个文件被删掉了）')
      onClose()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      setBusy(false)
    }
  }

  const viewBox = `${0} ${0} ${bed.width} ${bed.depth}`
  /** 网格：床身四分之一的整毫米线（256 → 64 一格），太大太小都退化成不画 */
  const gridLines = useMemo(() => {
    const step = Math.round(Math.min(bed.width, bed.depth) / 4)
    if (!(step > 0) || step < 4) return []
    const out: { x1: number; y1: number; x2: number; y2: number }[] = []
    for (let x = step; x < bed.width; x += step) out.push({ x1: x, y1: 0, x2: x, y2: bed.depth })
    for (let y = step; y < bed.depth; y += step) out.push({ x1: 0, y1: y, x2: bed.width, y2: y })
    return out
  }, [bed.width, bed.depth])

  /* 鼠标悬停时能看出"点了会加在哪"（选中某块时才有意义） */
  const cursorHint = cursor !== null && sel !== null ? `${cursor[0]}, ${cursor[1]}` : ''

  /* 顶点手柄半径（随床身大小缩放） */
  const handleR = Math.max(2.2, Math.min(bed.width, bed.depth) / 90)

  return (
    <ModalC14
      open={machineId !== null}
      title={`禁区 · ${machine?.display || machineId || ''}`}
      subtitle={`床身 ${bed.width} × ${bed.depth} mm · 机器坐标原点在左前角、y 向上 —— 写回 presets/forbidden_zones/${machineId ?? ''}.toml`}
      size="lg"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.status}>
            {zones.length} 个禁区
            {past.length > 0 && ` · 可撤销 ${past.length} 步`}
            {cursorHint && ` · 光标 ${cursorHint}`}
          </span>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={busy}
            onClick={() => void save()}
          >
            {busy ? '保存中…' : '确认保存'}
          </button>
        </>
      }
    >
      <div className={s.wrap}>
        {/* —— 左：画布 —— */}
        <div className={s.canvasPane}>
          <div className={s.tools}>
            <button type="button" className={s.tool} onClick={addZone} title="加一块新禁区（给一个床身中央的方块起手）">
              添加禁区
            </button>
            <button
              type="button"
              className={s.tool}
              disabled={sel === null}
              title={sel === null ? '先在右边选一块' : '删掉选中的那一块'}
              onClick={() => sel !== null && removeZone(sel)}
            >
              删除选中
            </button>
            <button
              type="button"
              className={s.tool}
              disabled={sel === null}
              title={sel === null ? '先在右边选一块' : '复制选中的那一块'}
              onClick={() => sel !== null && duplicateZone(sel)}
            >
              复制
            </button>
            <span className={s.toolGap} />
            <button type="button" className={s.tool} disabled={!past.length} onClick={undo} title="撤销（⌘Z）">
              撤销
            </button>
            <button type="button" className={s.tool} disabled={!future.length} onClick={redo} title="重做（⇧⌘Z）">
              重做
            </button>
            <button
              type="button"
              className={s.tool}
              disabled={!zones.length}
              title="一次清空全部（保存时才真的删文件）"
              onClick={() => {
                commit([])
                setSel(null)
              }}
            >
              清空
            </button>
            <span className={s.toolGap} />
            <button type="button" className={s.tool} onClick={() => setZoom((z) => Math.max(ZOOM_MIN, z / 1.25))} title="缩小">
              −
            </button>
            <span className={s.zoom}>{Math.round(zoom * 100)}%</span>
            <button type="button" className={s.tool} onClick={() => setZoom((z) => Math.min(ZOOM_MAX, z * 1.25))} title="放大">
              +
            </button>
            <button type="button" className={s.tool} onClick={() => setZoom(1)} title="回到 100%">
              适配
            </button>
          </div>

          <div className={s.canvasBox}>
            <svg
              ref={svgRef}
              className={s.canvas}
              viewBox={viewBox}
              preserveAspectRatio="xMidYMid meet"
              style={{ transform: `scale(${zoom})` }}
              onPointerDown={() => {
                /* 按在手柄上那颗会 stopPropagation，走不到这里 —— 所以按到别处
                   （床身 / 块面）就是"不编辑那个点了"，把亮着的点收掉 */
                if (dragRef.current === null) setActivePt(null)
              }}
              onPointerMove={onCanvasMove}
              onPointerUp={onCanvasUp}
              onPointerLeave={() => setCursor(null)}
            >
              {/* 床身 */}
              <rect x={0} y={0} width={bed.width} height={bed.depth} className={s.bed} />
              {gridLines.map((g, i) => (
                <line key={i} {...g} className={s.grid} />
              ))}

              {zones.map((z, i) => {
                return (
                  <g key={i} className={s.zoneG}>
                    <polygon
                      points={pointsAttr(z)}
                      className={`${s.poly} ${i === sel ? s.polyOn : ''}`}
                      onClick={() => {
                        setSel(i)
                        setActivePt(null)
                      }}
                    />
                    {/* 选中那块的"点边加顶点"热区：看不见，但一直能点（压在面上、手柄下） */}
                    {i === sel &&
                      z.length > 0 &&
                      z.map(([x, y], j) => {
                        const [nx, ny] = z[(j + 1) % z.length]
                        return (
                          <line
                            key={j}
                            x1={x}
                            y1={bed.depth - y}
                            x2={nx}
                            y2={bed.depth - ny}
                            className={s.edgeHit}
                            strokeWidth={Math.max(2.5, Math.min(bed.width, bed.depth) / 36)}
                            onClick={(e) => {
                              e.stopPropagation()
                              const at = toBed(e)
                              if (at !== null) insertPoint(i, j, at)
                            }}
                          />
                        )
                      })}
                    {/* 画布上只有亮着的那一个点（正在改它的坐标 / 正拖着 / 刚加上的） */}
                    {i === sel &&
                      z.map(([x, y], j) => {
                        if (j !== activePt) return null
                        const py = bed.depth - y
                        const label = `${x}, ${y}`
                        const boxW = label.length * 4.7 + 7
                        const boxH = 11
                        /* 牌子默认顶在点上方；贴着床身上沿时翻到点下方 */
                        const boxY = py - 4 - boxH >= 0.6 ? py - 4 - boxH : py + 4
                        const boxX = Math.max(0.6, Math.min(bed.width - boxW - 0.6, x - boxW / 2))
                        return (
                          <g key={j}>
                            <circle
                              cx={x}
                              cy={py}
                              r={handleR * 1.6}
                              className={`${s.handle} ${s.handleOn}`}
                              onPointerDown={onVertexDown(i, j)}
                              onContextMenu={(e) => {
                                e.preventDefault()
                                removePoint(i, j)
                              }}
                            />
                            <rect className={s.tagBox} x={boxX} y={boxY} width={boxW} height={boxH} rx={2} />
                            <text x={boxX + boxW / 2} y={boxY + 8} className={s.handleText} textAnchor="middle">
                              {label}
                            </text>
                          </g>
                        )
                      })}
                  </g>
                )
              })}
            </svg>
          </div>

          <p className={s.hint}>
            先在右边选一块 → 点块的边插入顶点；点某行的坐标框，画布上亮出那个点 —— 拖它挪位置（收
            0.1mm）、右键删掉它；列表里也有「添加点」。
            {sel === null && ' 现在没有选中的块 —— 点右边任意一块，或先「添加禁区」。'}
          </p>
        </div>

        {/* —— 右：禁区列表 —— */}
        <div className={s.listPane}>
          <div className={s.listHead}>禁区列表（共 {zones.length} 个）</div>
          {!zones.length && (
            <div className={s.empty}>
              这台机器还没有禁区。
              <br />
              点左上「添加禁区」起一块，再到画布上拖出形状。
            </div>
          )}
          {zones.map((z, i) => (
            <div key={i} className={`${s.item} ${i === sel ? s.itemOn : ''}`}>
              <button
                type="button"
                className={s.itemHead}
                onClick={() => {
                  setSel(i)
                  setActivePt(null)
                }}
                title="选中这块（画布上高亮，才能编辑顶点）"
              >
                <span className={s.itemName}>禁区 {i + 1}</span>
                <span className={s.itemMeta}>({z.length} 点)</span>
                {z.length < MIN_POINTS && <span className={s.itemBad}>不足 3 点</span>}
              </button>
              <div className={s.itemActions}>
                <button type="button" className={s.mini} onClick={() => duplicateZone(i)} title="复制这块">
                  复制
                </button>
                <button type="button" className={`${s.mini} ${s.miniDanger}`} onClick={() => removeZone(i)} title="删掉这块">
                  删除
                </button>
              </div>
              {/* 点开看坐标：参数台同款的步进框（箭头 / 滚轮 / 键入都行）。
                  正在改哪个点，画布上哪个点亮起 —— 用 onFocus/onBlur 冒泡挂在这行上 */}
              {i === sel && (
                <div className={s.coords}>
                  {z.map(([x, y], j) => (
                    <div
                      key={j}
                      className={s.coordRow}
                      onFocus={() => setActivePt(j)}
                      onBlur={() => {
                        /* 拖那个点之前输入框会先失焦 —— 这一下别把亮着的点掐灭 */
                        if (dragRef.current === null) setActivePt((cur) => (cur === j ? null : cur))
                      }}
                    >
                      <span className={s.coordIdx}>#{j + 1}</span>
                      <span className={s.coordNum}>
                        <NumberField
                          value={x}
                          label={`禁区 ${i + 1} 第 ${j + 1} 点 X`}
                          unit="mm"
                          min={0}
                          max={bed.width}
                          step={0.1}
                          decimals={1}
                          size="sm"
                          focusOnBoxClick
                          onChange={(nx) =>
                            setZones((prev) =>
                              prev.map((q, qi) =>
                                qi === i ? q.map((p, pi) => (pi === j ? [nx, p[1]] as P : p)) : q,
                              ),
                            )
                          }
                        />
                      </span>
                      <span className={s.coordNum}>
                        <NumberField
                          value={y}
                          label={`禁区 ${i + 1} 第 ${j + 1} 点 Y`}
                          unit="mm"
                          min={0}
                          max={bed.depth}
                          step={0.1}
                          decimals={1}
                          size="sm"
                          focusOnBoxClick
                          onChange={(ny) =>
                            setZones((prev) =>
                              prev.map((q, qi) =>
                                qi === i ? q.map((p, pi) => (pi === j ? [p[0], ny] as P : p)) : q,
                              ),
                            )
                          }
                        />
                      </span>
                      <button
                        type="button"
                        className={s.coordDel}
                        title={`删掉第 ${j + 1} 个顶点`}
                        onClick={() => removePoint(i, j)}
                      >
                        ×
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    className={s.addPoint}
                    title="加在末点→首点那条收口边的中点（画布上也可以直接点边插入）"
                    onClick={() => addPoint(i)}
                  >
                    + 添加点
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>

      {zones.length === 0 && (machine?.zoneCount ?? 0) > 0 && (
        <p className={s.warn}>
          这一保存会把 {machineId} 的禁区文件删掉（本来有 {machine?.zoneCount} 块）。
        </p>
      )}
    </ModalC14>
  )
}
