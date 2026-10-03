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
 * 默认只画床身轮廓 + 网格 + 多边形的边；**选中某块才显示顶点与坐标** ——
 * 一屏全是手柄与数字，反而看不清形状（旧面板的设计原则，照抄）。
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
  const [zoom, setZoom] = useState(1)
  const [busy, setBusy] = useState(false)
  /** 正在拖第几块的最后一个点（拖拽期间的中间态不进历史栈 —— 松手才推一次） */
  const dragRef = useRef<{ zone: number; point: number } | null>(null)
  const svgRef = useRef<SVGSVGElement>(null)
  /** 光标处的机器坐标（状态栏显示用） */
  const [cursor, setCursor] = useState<P | null>(null)

  /* 每次打开（或换了一台）都重新起草：取消掉的不该还留着 */
  useEffect(() => {
    setZones(cloneZones(machine?.zones ?? []))
    setPast([])
    setFuture([])
    setSel(machine?.zones.length ? 0 : null)
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

  /* —— 坐标换算（只在这个文件里写一次）—— */
  const toBed = useCallback(
    (e: ReactPointerEvent<SVGSVGElement | SVGPolygonElement>): P | null => {
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
  const onCanvasClick = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (dragRef.current !== null) return
    if (sel === null) return
    const at = toBed(e)
    if (at === null) return
    const next = zones.map((z, i) => (i === sel ? [...z, at] : z))
    commit(next)
  }

  const onVertexDown = (zi: number, pi: number) => (e: ReactPointerEvent<SVGCircleElement>) => {
    e.stopPropagation()
    setSel(zi)
    dragRef.current = { zone: zi, point: pi }
    // 拖拽起点压进历史栈（一次拖拽 = 一步撤销）
    setPast((p) => [...p.slice(-49), zones.map((z) => z.map((q) => [...q] as P))])
    setFuture([])
    svgRef.current?.setPointerCapture(e.pointerId)
  }

  const onCanvasMove = (e: ReactPointerEvent<SVGSVGElement>) => {
    const at = toBed(e)
    if (at === null) return
    setCursor([round1(at[0]), round1(at[1])])
    const d = dragRef.current
    if (d === null) return
    // 拖拽期间只改草稿（不进历史栈 —— 起点已经压过一次了）
    setZones((prev) =>
      prev.map((z, i) =>
        i === d.zone ? z.map((q, j) => (j === d.point ? [round1(at[0]), round1(at[1])] as P : q)) : z,
      ),
    )
  }

  const onCanvasUp = (e: ReactPointerEvent<SVGSVGElement>) => {
    dragRef.current = null
    if (svgRef.current?.hasPointerCapture(e.pointerId)) svgRef.current.releasePointerCapture(e.pointerId)
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
  }

  const removeZone = (i: number) => {
    commit(zones.filter((_, k) => k !== i))
    setSel((cur) => (cur === null ? null : cur === i ? null : cur > i ? cur - 1 : cur))
  }

  const duplicateZone = (i: number) => {
    const src = zones[i]
    if (src === undefined) return
    // 挪开一点点，免得叠在原件上看不见（床身太小时贴着边也不至于跑出去）
    const shift = Math.min(10, bed.width / 20)
    const copy: P[] = src.map(([x, y]) => [round1(Math.min(bed.width, x + shift)), round1(y)] as P)
    commit([...zones, copy])
    setSel(zones.length)
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
              onPointerMove={onCanvasMove}
              onPointerUp={onCanvasUp}
              onPointerLeave={() => setCursor(null)}
              onClick={onCanvasClick}
            >
              {/* 床身 */}
              <rect x={0} y={0} width={bed.width} height={bed.depth} className={s.bed} />
              {gridLines.map((g, i) => (
                <line key={i} {...g} className={s.grid} />
              ))}

              {zones.map((z, i) => (
                <g key={i}>
                  <polygon
                    points={pointsAttr(z)}
                    className={`${s.poly} ${i === sel ? s.polyOn : ''}`}
                    onClick={(e) => {
                      e.stopPropagation()
                      setSel(i)
                    }}
                  />
                  {/* 只画"点"这件事：顶点手柄与坐标只在选中的那一块上出现 */}
                  {i === sel &&
                    z.map(([x, y], j) => (
                      <g key={j}>
                        <circle
                          cx={x}
                          cy={bed.depth - y}
                          r={Math.max(2.2, Math.min(bed.width, bed.depth) / 90)}
                          className={s.handle}
                          onPointerDown={onVertexDown(i, j)}
                          onContextMenu={(e) => {
                            e.preventDefault()
                            removePoint(i, j)
                          }}
                        />
                        <text
                          x={x}
                          y={bed.depth - y - Math.max(4, Math.min(bed.width, bed.depth) / 42)}
                          className={s.handleText}
                          textAnchor="middle"
                        >
                          {x}, {y}
                        </text>
                      </g>
                    ))}
                </g>
              ))}
            </svg>
          </div>

          <p className={s.hint}>
            先在右边选一块 → 在画布上点一下加顶点、拖顶点挪位置（收 0.1mm）、右键顶点删掉它。
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
                onClick={() => setSel(i)}
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
              {/* 点开看坐标（每一格都能改；改完立刻画到画布上） */}
              {i === sel && (
                <div className={s.coords}>
                  {z.map(([x, y], j) => (
                    <div key={j} className={s.coordRow}>
                      <span className={s.coordIdx}>#{j + 1}</span>
                      <input
                        className={s.coordInput}
                        type="number"
                        step="any"
                        value={String(x)}
                        aria-label={`禁区 ${i + 1} 第 ${j + 1} 点 X`}
                        onChange={(e) => {
                          const nx = Number(e.target.value)
                          if (!Number.isFinite(nx)) return
                          setZones((prev) =>
                            prev.map((q, qi) =>
                              qi === i ? q.map((p, pi) => (pi === j ? [nx, p[1]] as P : p)) : q,
                            ),
                          )
                        }}
                      />
                      <input
                        className={s.coordInput}
                        type="number"
                        step="any"
                        value={String(y)}
                        aria-label={`禁区 ${i + 1} 第 ${j + 1} 点 Y`}
                        onChange={(e) => {
                          const ny = Number(e.target.value)
                          if (!Number.isFinite(ny)) return
                          setZones((prev) =>
                            prev.map((q, qi) =>
                              qi === i ? q.map((p, pi) => (pi === j ? [p[0], ny] as P : p)) : q,
                            ),
                          )
                        }}
                      />
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
