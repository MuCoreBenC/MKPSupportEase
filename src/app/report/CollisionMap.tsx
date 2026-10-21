/*
 * **碰撞位置 · 热床俯视图**（2026-10-18）。
 *
 * 碰撞失败那一趟的记录里带着一份几何（`_meta.json` 的 `collision`），这一屏就是把它
 * 画出来：**打印板**（有机型板规格就画真板轮廓 —— 卡舌 / 把手 / 圆角，与参数页同一份
 * `Plate` 数据）+ 网格、**擦料塔**（按参数算出真实形状：塔身 / 斜肋 / 护套，与参数页
 * 同一套算法）、**模型整体包围盒**、**冲突点**（红），外加一条图例。
 *
 * 三条定死的口径：
 *   1. **只读**：拖拽 / 编辑一概没有 —— 这一屏是"事后复盘"，不是调参器（调参在参数页）。
 *   2. **判据框与真实形状一起画**：塔外面那圈虚线框是**碰撞判据按哪块算**的包围盒，
 *      里面那块是塔的真实长相。只画一个的话，用户会问"图上明明没碰到啊"。
 *   3. **没给就不摆**：几何缺一格（`printable` / `towerBbox`）就整块不画 ——
 *      宁可少一张图，不画一张编出来的图（与报告页"没给就不摆"同一条）。
 *
 * 坐标系（两套，换算各收在一处）：
 *   - **机器坐标**：事实里的 x/y（原点在热床左下角，y 向上）—— `bedPt()` 一个出口。
 *   - **板坐标**：`Plate` 的（原点在后缘左角，y 向下）—— 与参数页 `TowerMap` 同一条
 *     换算：`plateX = frame.x + 机器x`，`plateY = frame.y + (frame.h - 机器y)`。
 *   有板规格：整张图画在板坐标系里（网格 / 模型 / 塔都落在板上的真实位置）；
 *   没板规格（目录拉不到 / 机型没配板）：退回圆角矩形热床，事实照画 —— 不因为
 *   一层装饰缺席就把整块图吞掉。
 */
import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import TowerCoreSvg from '../params/TowerCoreSvg'
import { api } from '../../api'
import type { Machine, Plate, ReportBox4, ReportCollision } from '../../api'
import ZoomLayer from '../../components/zoom/ZoomLayer'
import s from './CollisionMap.module.css'

interface Props {
  collision: ReportCollision
  /** 报告行的机型（`A1_MINI` 这种规范形，与 catalog 的 machine.id 同一形状）—— 选板用 */
  machineType: string | null
  /**
   * **换一支箭头**（2026-10-18）：报告页那支默认画"建议挪**模型**"；挪塔那一屏要画
   * "塔从 A 到 B"——同一张画布，两件事必须分得开（作者：「这个示意图不对，因为他显示的
   * 箭头是移动模型，实际这个的是移动擦料塔」）。
   *
   * 给了就**按它如实画**（长度 = |vec|，镞尖正好落在目标处），不再做"探出空白"那套
   * 自适应；`vec` 为零 = 不画。
   */
  arrow?: { from: [number, number]; vec: [number, number] } | null
  /**
   * **一支都不画**（2026-10-18）：`arrow: null` 只是"没有覆盖"、会回落到事实里那支
   * 报告页箭头（挪模型）—— 挪塔那一屏的「挪后」那幅要的是**干净**，用这个。
   */
  hideArrow?: boolean
  /** **塔的目标位置虚框**（机器坐标）：箭头指过去时，落点得看得见（挪塔那一屏用） */
  ghost?: ReportBox4 | null
  /** 不摆下面那行图例（两幅并排的紧凑用法里，两条图例各占一行太吵） */
  hideLegend?: boolean
  /**
   * **按高度定尺**（px，2026-10-18）：给了就把画布高度钉死、宽度按板子纵横比反算
   * —— 两幅并排要比"同高"时用它。不给 = 原来的"宽度跟着容器走"。
   */
  height?: number
  /**
   * 小图按钮的无障碍名字（2026-10-21）。默认是碰撞俯视图；擦料塔那一栏借这张画布
   * 画"塔在哪"，说成"碰撞"会误导 —— 那边自己给一句。
   */
  label?: string
}

/** 塔身固定画成 44px（与参数页同一个数 —— 两边比例一致，形状才是同一个形状） */
const TOWER_CORE_PX = 44
/** 参数页那张塔图的裙边圈数（同一套算法，同一个数） */
const BRIM_LOOP_COUNT = 5
/** 网格几等分（照参数页那张热床图的观感） */
const GRID = 6

/* 板规格按机型查一次就够 —— 整个会话缓存（参数页/校准页也是这么缓 catalog 的） */
type PlateWorld = { machines: Machine[]; plates: Plate[] } | null
let plateWorld: Promise<PlateWorld> | null = null
function plateWorldOnce() {
  plateWorld ??= Promise.all([api.getMachines(), api.getRuntimeCatalog()])
    .then(([machines, world]) => ({
      /* 机型的 `defaultPlateId` 住旧世界的 Machine 里（新世界的 catalog 那份没有） */
      machines,
      plates: world.plates ?? [],
    }))
    .catch((err) => {
      /* 拉不到就不画板（退回圆角矩形）—— 装饰缺席不拦着事实 */
      console.error('[collision-map] 目录拉取失败', err)
      return null
    })
  return plateWorld
}

/** 一个框的四至（缺一格就当没有 —— 不许拿 0 顶替） */
function boxOf(b: ReportBox4 | undefined): { x: number; y: number; w: number; h: number } | null {
  if (b === undefined) return null
  const { xMin, xMax, yMin, yMax } = b
  if (xMin === undefined || xMax === undefined || yMin === undefined || yMax === undefined) {
    return null
  }
  const w = xMax - xMin
  const h = yMax - yMin
  if (!(w > 0) || !(h > 0)) return null
  return { x: xMin, y: yMin, w, h }
}

export default function CollisionMap({
  collision,
  machineType,
  arrow: arrowOverride = null,
  hideArrow = false,
  ghost = null,
  hideLegend = false,
  height,
  label = '放大查看碰撞俯视图',
}: Props) {
  const bed = boxOf(collision.printable)
  const tower = boxOf(collision.towerBbox)
  const model = boxOf(collision.modelBbox)
  /*
   * 画图用的模型框：**一个模型一个**（内核按占用格连通块切好的 `modelBoxes`）。
   * 旧记录没有这一格 ⇒ 退回整体那一个框（`modelBbox`）——至少还能看出"东西大概在哪"。
   * ★ `model` 本身留着：箭头"探出模型框"那段逻辑仍按**整体**算（挪模型是一条整体建议）。
   */
  const modelRects = (() => {
    const each = (collision.modelBoxes ?? [])
      .map(boxOf)
      .filter((b): b is { x: number; y: number; w: number; h: number } => b !== null)
    if (each.length > 0) return each
    return model !== null ? [model] : []
  })()

  const [plate, setPlate] = useState<Plate | null>(null)
  useEffect(() => {
    let alive = true
    if (machineType === null) return undefined
    void plateWorldOnce().then((w) => {
      if (!alive || w === null) return
      const m = w.machines.find((x) => x.id === machineType)
      const p = m?.defaultPlateId != null ? w.plates.find((x) => x.id === m.defaultPlateId) : undefined
      if (p !== undefined) setPlate(p)
    })
    return () => {
      alive = false
    }
  }, [machineType])

  /* 点图 = 浮起来看大的（通用放大层 `components/zoom/ZoomLayer`，内容是这份 SVG 俯视图）。
     报告页不在模态框里 ⇒ 放大层 portal 到 body。`fromRef` 记点的那一下小图在哪（FLIP First） */
  const [zoom, setZoom] = useState(false)
  const fromRef = useRef<DOMRect | null>(null)
  const mapBtnRef = useRef<HTMLButtonElement>(null)

  /* 热床都没有（老记录 / 几何缺）⇒ 整块不画，交给上一层的空白态 */
  if (bed === null || tower === null) return null

  /* 有板：画布是板坐标系（可打印区在板上的 frame 是机器坐标的落点）；
     没板：画布就是可打印区本身。两套换算都收在 bedPt 里 */
  const frame = plate !== null ? plate.frame : { x: 0, y: 0, w: bed.w, h: bed.h }
  const canvasW = plate !== null ? plate.w : bed.w
  const canvasH = plate !== null ? plate.d : bed.h
  const bedPt = (x: number, y: number): [number, number] => [
    frame.x + (x - bed.x),
    frame.y + (frame.h - (y - bed.y)),
  ]

  /*
   * 模型轮廓（2026-10-18 作者：「能不能真的做成俯视图，多边形的，可以用 12 边形，
   * 或者 24 边形」）—— 内核按占用格连通块切好、交给的是凸包（≤24 边）。
   * 空 / 旧记录 ⇒ 下面退回 `modelRects` 的矩形。
   */
  const modelPolys = (collision.modelPolygons ?? [])
    .map((poly) =>
      poly
        .filter((p): p is [number, number] => Array.isArray(p) && p.length === 2)
        .map(([x, y]) => bedPt(x, y)),
    )
    .filter((poly) => poly.length >= 3)

  const towerShape = collision.tower ?? null
  const bodyMm = towerShape?.bodyMm ?? tower.w
  /*
   * 塔图那张 svg 自己带"外围结构往外扩多少"的画布：扩展量取**判据框与塔身之差的一半**
   * —— 于是画出来的外沿正好压在虚线判据框上（两件事是同一件事实的两个说法）。
   */
  const expand = Math.max(0, (Math.min(tower.w, tower.h) - bodyMm) / 2)
  const vm = TOWER_CORE_PX / (bodyMm > 0 ? bodyMm : 1)
  const vbOffset = -expand * vm
  const vbSize = TOWER_CORE_PX + expand * vm * 2

  /* 冲突点：落进可打印区才画（框外的点画出来只会误导 —— 那不是撞的地方） */
  const points = (collision.conflictPoints ?? []).filter(
    ([x, y]) => x >= bed.x && x <= bed.x + bed.w && y >= bed.y && y <= bed.y + bed.h,
  )
  const dotR = Math.max(0.8, bed.w / 240)

  const [towerLeftPlate, towerTopPlate] = bedPt(tower.x, tower.y + tower.h)

  /*
   * **建议挪动的箭头**（2026-10-18 作者定的样子：**白**、粗、和塔的暖色分开；
   * 「箭身可以叠在其他图形，但是箭头尽量在空白位置」＋「智能的伸长缩短」）。
   *
   * 算法：距离不必如实画（挪多少在建议文字里），方向如实 —— 从"挪谁"的中心沿
   * 建议方向做射线，把模型框 / 塔框（都垫一圈，给箭镞留位）沿射线"探出去"，
   * 箭镞落在探出口再往外一点：起点压在框里没关系（箭身可以叠），**箭镞必须在
   * 空白处**，压在框上的箭头没有信息量。长度同时夹在 [最小可见长度, 板内边界]
   * 之间 —— 短了看不清，长了戳出板。旧记录没这两格 / 挪不开那一档 ⇒ 不画。
   */
  const arrowFrom = arrowOverride?.from ?? collision.arrowFrom
  const arrowVec = arrowOverride?.vec ?? collision.arrowVec
  let arrow: { path: string; transform: string } | null = null
  if (
    !hideArrow &&
    arrowFrom != null &&
    arrowVec != null &&
    (arrowVec[0] !== 0 || arrowVec[1] !== 0)
  ) {
    const [ox, oy] = bedPt(arrowFrom[0], arrowFrom[1])
    const [gx, gy] = bedPt(arrowFrom[0] + arrowVec[0], arrowFrom[1] + arrowVec[1])
    const head = Math.max(5, canvasW * 0.05)
    const w0 = Math.max(3, canvasW * 0.02)
    const w1 = w0 * 0.55
    const hw = head * 1.15
    let len: number
    let ang: number
    if (arrowOverride != null) {
      /* **如实一支**（挪塔那一屏）：起点 → 目标中心的真实位移，镞尖正好落在目标上 */
      len = Math.hypot(gx - ox, gy - oy)
      ang = Math.atan2(gy - oy, gx - ox)
    } else {
      const du = { x: Math.sign(arrowVec[0]), y: -Math.sign(arrowVec[1]) }
      const n = Math.hypot(du.x, du.y)
      du.x /= n
      du.y /= n
      const pad = canvasW * 0.045
      const boxes: Array<[number, number, number, number]> = []
      for (const b of [model, { x: tower.x, y: tower.y, w: tower.w, h: tower.h }]) {
        if (b === null) continue
        const [ax, ay] = bedPt(b.x, b.y + b.h)
        const [bx, by] = bedPt(b.x + b.w, b.y)
        boxes.push([
          Math.min(ax, bx) - pad,
          Math.min(ay, by) - pad,
          Math.max(ax, bx) + pad,
          Math.max(ay, by) + pad,
        ])
      }
      /* 射线从原点出这个框（沿方向）走多远 —— 原点在框里时就是"探出头"的距离 */
      const exitOf = (b: [number, number, number, number]) => {
        const tx =
          du.x > 0 ? (b[2] - ox) / du.x : du.x < 0 ? (b[0] - ox) / du.x : Number.POSITIVE_INFINITY
        const ty =
          du.y > 0 ? (b[3] - oy) / du.y : du.y < 0 ? (b[1] - oy) / du.y : Number.POSITIVE_INFINITY
        return Math.max(0, Math.min(tx, ty))
      }
      const clearLen = boxes.length > 0 ? Math.max(...boxes.map(exitOf)) : 0
      const margin = head * 0.7
      const tMax = Math.max(
        0,
        Math.min(
          du.x > 0 ? (canvasW - margin - ox) / du.x : du.x < 0 ? (margin - ox) / du.x : Infinity,
          du.y > 0 ? (canvasH - margin - oy) / du.y : du.y < 0 ? (margin - oy) / du.y : Infinity,
        ),
      )
      len = Math.max(head * 1.5, Math.min(Math.max(clearLen, head * 1.8), tMax))
      ang = Math.atan2(du.y, du.x)
    }
    if (len > head * 1.4) {
      const body = len - head
      arrow = {
        path:
          `M 0 ${-w0 / 2} L ${body} ${-w1 / 2} L ${body} ${-hw / 2} L ${len} 0` +
          ` L ${body} ${hw / 2} L ${body} ${w1 / 2} L 0 ${w0 / 2} Z`,
        transform: `translate(${ox} ${oy}) rotate(${(ang * 180) / Math.PI})`,
      }
    }
  }

  /* 箭头单独一层（z-index 压过塔图 —— 上一版被塔压住，作者：「现在箭头被压住了」） */
  const arrowSvg =
    arrow !== null ? (
      <svg
        className={s.svg}
        style={{ zIndex: 3, pointerEvents: 'none' }}
        viewBox={`0 0 ${canvasW} ${canvasH}`}
        preserveAspectRatio="xMidYMid meet"
        aria-hidden="true"
      >
        <path className={s.arrow} d={arrow.path} transform={arrow.transform} />
      </svg>
    ) : null

  const factSvg = (
    <svg
      className={s.svg}
      viewBox={`0 0 ${canvasW} ${canvasH}`}
      preserveAspectRatio="xMidYMid meet"
      aria-hidden="true"
    >
      {/* 没板规格时才有"热床矩形"这一层 —— 有板时板轮廓就是床 */}
      {plate === null && (
        <rect
          className={s.bed}
          x={0}
          y={0}
          width={canvasW}
          height={canvasH}
          rx={Math.min(canvasW, canvasH) / 40}
        />
      )}

      {/* 淡网格：给"挪多少"一个尺度感（6 等分，与参数页同一观感；贴边的线不画） */}
      <g className={s.grid}>
        {Array.from({ length: GRID - 1 }, (_, i) => {
          const x = frame.x + ((i + 1) * frame.w) / GRID
          return <line key={`v${i}`} x1={x} y1={frame.y} x2={x} y2={frame.y + frame.h} />
        })}
        {Array.from({ length: GRID - 1 }, (_, i) => {
          const y = frame.y + ((i + 1) * frame.h) / GRID
          return <line key={`h${i}`} x1={frame.x} y1={y} x2={frame.x + frame.w} y2={y} />
        })}
      </g>

      {/* 机型额外禁区（通常没有；有就画 —— "塔落在禁区里"那一档靠它说清） */}
      {(collision.forbiddenZones ?? []).map((zone, i) => {
        if (zone.length < 3) return null
        return (
          <polygon
            key={`z${i}`}
            className={s.zone}
            points={zone.map(([x, y]) => bedPt(x, y).join(',')).join(' ')}
          />
        )
      })}

      {/* 模型轮廓（另一种颜色 + 虚线：一眼和塔分开）。
          ★ **一个模型一块轮廓**，不是矩形（2026-10-18 作者：「图片的都是矩形的看着好像
            碰撞了一样，能不能真的做成俯视图，多边形的」）——矩形四个角会把模型旁边的空
            算进去。内核给的是凸包（≤24 边）；旧记录没有这一格 ⇒ 退回矩形。 */}
      {modelPolys.length > 0
        ? modelPolys.map((poly, i) => (
            <polygon
              key={`mp${i}`}
              className={s.model}
              points={poly.map(([x, y]) => `${x},${y}`).join(' ')}
            />
          ))
        : modelRects.map((b, i) => {
            const [lx, ty] = bedPt(b.x, b.y + b.h)
            return (
              <rect key={`m${i}`} className={s.model} x={lx} y={ty} width={b.w} height={b.h} />
            )
          })}

      {/* 判据框：碰撞按这一块算（虚线，压在塔的真实形状外沿上） */}
      <rect
        className={s.towerBox}
        x={towerLeftPlate}
        y={towerTopPlate}
        width={tower.w}
        height={tower.h}
      />

      {/* **塔的目标位置虚框**（挪塔那一屏）：箭头指过去时落点看得见 */}
      {ghost != null &&
        ghost.xMin !== undefined &&
        ghost.xMax !== undefined &&
        ghost.yMin !== undefined &&
        ghost.yMax !== undefined && (
          <rect
            className={s.ghost}
            x={bedPt(ghost.xMin, ghost.yMax)[0]}
            y={bedPt(ghost.xMin, ghost.yMax)[1]}
            width={ghost.xMax - ghost.xMin}
            height={ghost.yMax - ghost.yMin}
          />
        )}

      {/* 冲突点 */}
      <g className={s.points}>
        {points.map(([x, y], i) => {
          const [cx, cy] = bedPt(x, y)
          return <circle key={i} cx={cx} cy={cy} r={dotR} />
        })}
      </g>

    </svg>
  )

  /* 俯视图本体：报告页里缩着显示（可点开进放大层），放大层里是**同一份**照大档画。
     ★ 放大档没有"另一套配色"（2026-10-18 作者：「你看这些的颜色就很好呀……
     不要动这些的，要改的是放大的」）—— 就是行内那份。唯一多出来的东西是
     `zoomed` 那一块**不透明的板形底**：行内本来就压在页面底上，放大档背后是
     深色遮罩，所以得自己垫一块，而且**裁剪成完整板轮廓**（含卡舌 / 把手孔）。 */
  const renderMap = (zoomed: boolean) =>
    plate !== null ? (
        /* 有板规格：板轮廓（含卡舌 / 把手）+ 余量灰圈 + 事实层，全在一张板坐标 svg 里 */
        <div
          className={s.mapPlate}
          style={{
            aspectRatio: `${plate.w} / ${plate.d}`,
            /* `height` 给了就**按高度定尺**（宽度反算）—— 两幅并排要比同高时用。
               ★ 放大档不认它（2026-10-18 真机）：放大层给的是"确定宽 + 高度上限"，
                 行内那个 104/132px 若照带进来，放大的仍是一张缩略图 ——
                 于是大壳子里躺着一张小图，FLIP 又从小图飞到大壳子（位置和大小都不对）。 */
            ...(height !== undefined && !zoomed
              ? { height: `${height}px`, width: 'auto' }
              : null),
          }}
        >
          <svg
            className={s.plate}
            viewBox={`0 0 ${plate.w} ${plate.d}`}
            preserveAspectRatio="xMidYMid meet"
            aria-hidden="true"
          >
            {/* 不透明底**布尔裁剪成完整板轮廓**（作者：「这个不透明的背景需要裁剪布尔成
                板子一样的大小包括舌」）—— 裁 `plate.path`（含卡舌的整块外轮廓，evenodd
                带上把手的孔），不裁 bodyPath（第一稿裁板身 ⇒ 卡舌缺口漏了底）。
                色是页面底 `--bg-1`（跟主题走，浅 #f7f9fa / 深 #1c1c1c），不是板自己的绿。
                ★ `fill` 走 CSS 类：presentation attribute 里的 `var()` 不生效。 */}
            {zoomed && (
              <>
                <clipPath id="collision-bed-clip">
                  <path d={plate.path} clipRule="evenodd" />
                </clipPath>
                <rect
                  className={s.bedSolid}
                  x={0}
                  y={0}
                  width={plate.w}
                  height={plate.d}
                  clipPath="url(#collision-bed-clip)"
                />
              </>
            )}
            <path d={plate.path} fillRule="evenodd" />
            {/* 板缘 → 可打印区之间的余量圈（与参数页同一画法：evenodd(板身, frame)） */}
            <path
              className={s.ring}
              fillRule="evenodd"
              d={`${plate.bodyPath} M${frame.x},${frame.y}h${frame.w}v${frame.h}h${-frame.w}Z`}
            />
          </svg>
          {factSvg}
          {/* 塔的真实形状（绝对定位盖在判据框上，百分比按板坐标算） */}
          {towerShape !== null && (
            <div
              className={s.tower}
              style={{
                left: `${(towerLeftPlate / plate.w) * 100}%`,
                top: `${(towerTopPlate / plate.d) * 100}%`,
                width: `${(tower.w / plate.w) * 100}%`,
                height: `${(tower.h / plate.d) * 100}%`,
              }}
            >
              <TowerCoreSvg
                outerVal={
                  towerShape.outerStructure === 'rib' || towerShape.outerStructure === 'sheath'
                    ? towerShape.outerStructure
                    : 'brim'
                }
                isRibVisible={towerShape.outerStructure === 'rib' && (towerShape.ribWidth ?? 0) > 0}
                ribWidthMm={towerShape.ribWidth ?? 0}
                ribExtraLengthMm={towerShape.ribExtraLength ?? 0}
                ribFilletWall={towerShape.ribFilletWall ?? false}
                vbOffset={vbOffset}
                vbSize={vbSize}
                vm={vm}
                brimLoopCount={BRIM_LOOP_COUNT}
                className={s.core}
              />
            </div>
          )}
          {arrowSvg}
        </div>
      ) : (
        /* 没板规格：退回圆角矩形热床（事实照画 —— 不因为装饰缺席吞掉整块图） */
        <div
          className={s.map}
          style={{
            aspectRatio: `${bed.w} / ${bed.h}`,
            /* 同 `.mapPlate`：放大档按宽度铺满（行内才用固定高度） */
            ...(height !== undefined && !zoomed
              ? { height: `${height}px`, width: 'auto' }
              : null),
          }}
        >
          {factSvg}
          {towerShape !== null && (
            <div
              className={s.tower}
              style={{
                left: `${(bedPt(tower.x, tower.y + tower.h)[0] / canvasW) * 100}%`,
                top: `${(bedPt(tower.x, tower.y + tower.h)[1] / canvasH) * 100}%`,
                width: `${(tower.w / canvasW) * 100}%`,
                height: `${(tower.h / canvasH) * 100}%`,
              }}
            >
              <TowerCoreSvg
                outerVal={
                  towerShape.outerStructure === 'rib' || towerShape.outerStructure === 'sheath'
                    ? towerShape.outerStructure
                    : 'brim'
                }
                isRibVisible={towerShape.outerStructure === 'rib' && (towerShape.ribWidth ?? 0) > 0}
                ribWidthMm={towerShape.ribWidth ?? 0}
                ribExtraLengthMm={towerShape.ribExtraLength ?? 0}
                ribFilletWall={towerShape.ribFilletWall ?? false}
                vbOffset={vbOffset}
                vbSize={vbSize}
                vm={vm}
                brimLoopCount={BRIM_LOOP_COUNT}
                className={s.core}
              />
            </div>
          )}
          {arrowSvg}
        </div>
      )

  return (
    <div className={s.wrap}>
      {/* 小图可点（能点的意思只说给光标听，`cursor: zoom-in`）—— 皮扒干净，只留"能点" */}
      <button
        ref={mapBtnRef}
        type="button"
        className={s.mapBtn}
        onClick={() => {
          /* ★ 量**按钮里那张图**（firstElementChild），不量按钮自己 —— 按钮在 flex 列里
             被拉伸到整栏宽（2026-10-18 真机：收起时朝按钮中心＝图的右下飞过去） */
          fromRef.current =
            mapBtnRef.current?.firstElementChild?.getBoundingClientRect() ?? null
          setZoom(true)
        }}
        aria-label={label}
      >
        {renderMap(false)}
      </button>

      {/* 图例：三个色块 + 名字（不标数字 —— 数字在「建议」那组里）。
          挪塔那一屏两幅并排，图例各占一行太吵 ⇒ `hideLegend` */}
      {!hideLegend && (
      <ul className={s.legend}>
        <li>
          <span className={s.swTower} aria-hidden="true" />
          擦料塔
        </li>
        <li>
          <span className={s.swModel} aria-hidden="true" />
          模型轮廓
        </li>
        <li>
          <span className={s.swHit} aria-hidden="true" />
          冲突点
          {collision.conflictCount !== undefined && collision.conflictCount > 0 && (
            <span className={s.legendNum}>{collision.conflictCount} 个</span>
          )}
        </li>
      </ul>
      )}

      {/* 点开看大：报告页不在模态框 ⇒ portal 到 body；点任意处 / Esc 收起，滚轮 / 捏合缩放 */}
      {zoom &&
        createPortal(
          <ZoomLayer
            from={fromRef.current ?? new DOMRect(0, 0, 0, 0)}
            returnTo={() =>
              mapBtnRef.current?.firstElementChild?.getBoundingClientRect() ?? null
            }
            onGone={() => setZoom(false)}
            label="收起放大俯视图"
          >
            <div
              className={s.zoomed}
              style={{
                /* 壳给**确定宽**：`.mapPlate` 的 `width:100%` 在自动宽的祖先链里会解析塌掉
                   （塌回"没有在流内容"⇒ 缩回原尺寸，"放大了个寂寞"）。宽按画布纵横比
                   折算高度上限 —— SVG 本身无损，给多大画多大 */
                width: `min(86vw, calc(78vh * ${(canvasW / canvasH).toFixed(4)}), 960px)`,
              }}
            >
              {renderMap(true)}
            </div>
          </ZoomLayer>,
          document.body,
        )}
    </div>
  )
}
