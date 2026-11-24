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
import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import TowerCoreSvg from '../params/TowerCoreSvg'
import { api } from '../../api'
import type { Machine, Plate, ReportBoundaryPiece, ReportBox4, ReportCollision } from '../../api'
import { useT } from '../../i18n'
import ZoomLayer from '../../components/zoom/ZoomLayer'
import { useSourceRevision } from '../state/deliveryState'
import { arrowPathOf, minArrowLength } from './arrowGeometry'
import { boundaryNumbering } from './boundaryNumbering'
import { rasterScreenRect } from './rasterRect'
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
  /**
   * **这一屏画不画塔**（2026-10-27，缺省画）。
   *
   * 擦料塔那一栏的俯视图要给的是"模型在板上占哪一块"（作者：「把它模型的包围盒也显示
   * 出来，就是后处理的时候碰撞检测的那个显示的一样的，复用的」）—— 同一块板、同一份
   * 模型几何，那里不摆塔、不摆判据框，图例里也没有塔这一条。
   */
  noTower?: boolean
  /** **塔的判据框不画**（真实塔走线已经在位图里，框只是旧版的辅助线） */
  hideTowerBox?: boolean
  /**
   * **越界那一屏的允许范围**（2026-11-19）：给了就把模型**超出这块**的部分再画一遍红色。
   *
   * 与 `collision.printable` 是两件事：那个是"板在哪"（映射基准），这个是"按哪块判"。
   * `_001` 用机型打印范围、`_002` 用涂胶范围 —— 两者可能比板大（A1 系列 X 有悬出量），
   * 所以不能拿它当画布基准。**不画那圈虚线框**（作者：大轮廓瞄着热床很怪）。
   */
  allowed?: ReportBox4 | null
  /**
   * **一块参考框**（机器坐标，2026-11-23）：校准件画布用它标"实测包围盒"。
   *
   * 与 `ghost` / `towerBox` 都不同：那两个是"塔按哪块判"，这个是记录里量的实物尺寸，
   * 只画一圈细虚线，不参与任何判据。
   */
  box?: ReportBox4 | null
  /**
   * **越界那一屏的每块模型**（2026-11-19）：轮廓 + 建议位移。
   *
   * 给了就按它画（每块一条凸包，**不按板裁** —— 板外那截正是要标红的东西），
   * 且画布会**扩到能看见溢出的那块**（否则红块被画布边裁掉，只剩贴边一条）。
   * 越界的块各一支白色箭头（中心 → 挪到位）。
   */
  pieces?: ReportBoundaryPiece[] | null
  /**
   * **笔尖相对喷嘴的偏移**（mm）：给了就多画一层"笔尖轮廓"。
   *
   * 与模型层重叠的那一段**不绘制**（裁剪掉）—— 两套虚线交错的话，图就废了。
   */
  pen?: [number, number] | null
  /**
   * **给越界块编号**（2026-11-19）：1、2、3… 与右侧那条逐条文案一一对应。
   * 只有弹窗说明页要（报告页不传 —— 那边没有逐条文字，编号只会是噪音）。
   */
  numbered?: boolean
  /**
   * `row`：图在左，图例与 `children` 摆右边（越界说明页，2026-11-19 作者：
   * 「你也不一定要把下面的内容放在下面，可以放在右侧，左右分布」）。
   *
   * 报告页仍是默认的 `stack`（图下面一行图例）—— 那边作者说「显示的很好」。
   */
  layout?: 'stack' | 'row'
  /** `row` 档右栏里、图例下面的那几句说明（没有就不摆那一格） */
  children?: ReactNode
}
/** 塔身固定画成 44px（与参数页同一个数 —— 两边比例一致，形状才是同一个形状） */
const TOWER_CORE_PX = 44
/** 参数页那张塔图的裙边圈数（同一套算法，同一个数） */
const BRIM_LOOP_COUNT = 5
/** 网格几等分（照参数页那张热床图的观感） */
const GRID = 6

/*
 * 板规格按机型查一次就够 —— 整个会话缓存（参数页/校准页也是这么缓 catalog 的）。
 *
 * ★ 缓存**认数据源代次**：这两读读的是内部根那份 catalog，换了源就换一条发布线的
 *   机型与板规格；不带代次的话，报告页的板轮廓 / 可打印区一直按旧源画，要重启才换。
 */
type PlateWorld = { machines: Machine[]; plates: Plate[] } | null
let plateWorld: { rev: number; promise: Promise<PlateWorld> } | null = null
function plateWorldOnce(rev: number) {
  if (plateWorld !== null && plateWorld.rev === rev) return plateWorld.promise
  plateWorld = {
    rev,
    promise: Promise.all([api.getMachines(), api.getRuntimeCatalog()])
      .then(([machines, world]) => ({
        /* 机型的 `defaultPlateId` 住旧世界的 Machine 里（新世界的 catalog 那份没有） */
        machines,
        plates: world.plates ?? [],
      }))
      .catch((err) => {
        /* 拉不到就不画板（退回圆角矩形）—— 装饰缺席不拦着事实 */
        console.error('[collision-map] 目录拉取失败', err)
        return null
      }),
  }
  return plateWorld.promise
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
  label,
  noTower = false,
  hideTowerBox = false,
  allowed = null,
  box = null,
  pieces = null,
  pen = null,
  numbered = false,
  layout = 'stack',
  children,
}: Props) {
  const tr = useT()
  const mapLabel = label ?? tr('report.map.label')
  /*
   * 塔框：**只有要画塔时才算**（2026-10-27）。`noTower` 那一档（模型俯视图）本来就没有
   * `towerBbox`，下面"没塔就不画整块"的闸按它放行。
   */
  const tower = noTower ? null : boxOf(collision.towerBbox)
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
  /* 换过数据源 ⇒ 按新源那份目录重查这块机型板（见 `plateWorldOnce` 的注释） */
  const sourceRevision = useSourceRevision()
  useEffect(() => {
    let alive = true
    if (machineType === null) return undefined
    void plateWorldOnce(sourceRevision).then((w) => {
      if (!alive || w === null) return
      const m = w.machines.find((x) => x.id === machineType)
      const p = m?.defaultPlateId != null ? w.plates.find((x) => x.id === m.defaultPlateId) : undefined
      /* 新源里这台机型没登记板 ⇒ 也回「没板规格」，不让旧源的板继续挂着 */
      setPlate(p ?? null)
    })
    return () => {
      alive = false
    }
  }, [machineType, sourceRevision])

  /* 点图 = 浮起来看大的（通用放大层 `components/zoom/ZoomLayer`，内容是这份 SVG 俯视图）。
     报告页不在模态框里 ⇒ 放大层 portal 到 body。`fromRef` 记点的那一下小图在哪（FLIP First） */
  const [zoom, setZoom] = useState(false)
  const fromRef = useRef<DOMRect | null>(null)
  const mapBtnRef = useRef<HTMLButtonElement>(null)
  /* 越界那一屏的裁剪路径 id：同一屏有行内与放大层两个实例，id 必须各不相同
     （React 的 `useId` 带冒号，SVG 的 `url(#…)` 里用不了，先去掉） */
  const outsideClipId = `collision-outside-${useId().replace(/:/g, '')}`
  /* 笔尖层的裁剪路径：整块视窗减去模型轮廓（重叠的那一段不画） */
  const penClipId = `collision-pen-${useId().replace(/:/g, '')}`

  /*
   * **可打印区**（映射的基准）：
   *   · 事实里给了就用它（碰撞 / 挪塔那两屏的几何自带，且与板 frame 同一块区域）；
   *   · 没给（擦料塔那栏只有塔坐标）就用**板规格里的可打印区**，原点即机器坐标原点。
   *
   * ★ 不许拿机型的 `movementRange` 顶替（2026-10-21 真机踩到）：A1 mini 是
   *   `-10..180`（喷嘴能悬出床 10mm），比板上那块 180 宽的可打印区大一圈 ——
   *   `bedPt` 只平移不缩放，塔会被整体往右推 10mm，判据框直接压出板外。
   */
  const bed =
    boxOf(collision.printable) ??
    (plate !== null ? { x: 0, y: 0, w: plate.frame.w, h: plate.frame.h } : null)

  /* 热床都没有（老记录 / 几何缺 / 板规格也查不到）⇒ 整块不画，交给上一层的空白态。
     要画塔的屏还多一道"没有塔框就不画"；`noTower` 那一档（模型俯视图）按前一条判。 */
  if (bed === null || (tower === null && !noTower)) return null

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
   * **允许范围**（越界那一屏，笔尖坐标）→ 画布矩形：红层按它裁（不画框，见 props 注）。
   * 模型画的是喷嘴坐标，所以范围要先**减去笔偏移**再映射 —— 这样"红的那部分"
   * 仍然是"笔尖会越界的那部分模型"，与消息里那个数、箭头长度三者对得上。
   */
  const penX = pen?.[0] ?? 0
  const penY = pen?.[1] ?? 0
  const allowedBox = boxOf(allowed ?? undefined)
  const allowedRect =
    allowedBox !== null
      ? (() => {
          const [ax, ay] = bedPt(allowedBox.x - penX, allowedBox.y - penY + allowedBox.h)
          return { x: ax, y: ay, w: allowedBox.w, h: allowedBox.h }
        })()
      : null

  /*
   * **画布要扩到能看见溢出的那块**（2026-11-19）：越界模型的轮廓在板外，画布仍按板定
   * 就会被切掉（作者看到的就是"贴边一条红"）。给了 `pieces` 才扩 —— 否则 `pad` 全 0，
   * 坐标与从前一字不差（碰撞 / 擦料塔那几屏不受影响）。
   * ★ 笔尖那一层（`bbox + pen`）也要框进来，否则它被画布边切掉一半。
   */
  const pieceRects = (pieces ?? [])
    .flatMap((p) => {
      const b = boxOf(p.bbox ?? undefined)
      if (b === null) return []
      const rects = [b]
      if (penX !== 0 || penY !== 0) {
        rects.push({ x: b.x + penX, y: b.y + penY, w: b.w, h: b.h })
      }
      return rects
    })
    .map((b) => {
      const [lx, ty] = bedPt(b.x, b.y + b.h)
      return { x: lx, y: ty, w: b.w, h: b.h }
    })
  /* 真溢出的那一侧才外扩，并留一点余量给描边与箭镞（没溢出的方向一像素都不加） */
  const GUTTER = 6
  const pad =
    pieceRects.length > 0
      ? (() => {
          const minX = Math.min(frame.x, ...pieceRects.map((r) => r.x))
          const maxX = Math.max(frame.x + frame.w, ...pieceRects.map((r) => r.x + r.w))
          const minY = Math.min(frame.y, ...pieceRects.map((r) => r.y))
          const maxY = Math.max(frame.y + frame.h, ...pieceRects.map((r) => r.y + r.h))
          const grow = (raw: number) => (raw > 0 ? raw + GUTTER : 0)
          return {
            l: grow(frame.x - minX),
            t: grow(frame.y - minY),
            r: grow(maxX - (frame.x + frame.w)),
            b: grow(maxY - (frame.y + frame.h)),
          }
        })()
      : { l: 0, t: 0, r: 0, b: 0 }
  /* 画布坐标不变，只把视窗往外挪（`viewBox` 的 x/y 为负） */
  const view = { x0: -pad.l, y0: -pad.t, w: canvasW + pad.l + pad.r, h: canvasH + pad.t + pad.b }
  const pct = (v: number, total: number) => `${(v / total) * 100}%`

  /*
   * 模型轮廓（2026-10-18 作者：「能不能真的做成俯视图，多边形的，可以用 12 边形，
   * 或者 24 边形」）—— 内核按占用格连通块切好、交给的是凸包（≤24 边）。
   * 空 / 旧记录 ⇒ 下面退回 `modelRects` 的矩形。
   */
  /*
   * 切片器自己的擦料塔（2026-11-04）：**只画包围盒**。
   * 形状随盘变（又长又宽 / 很小都有），所以不套 MKP 塔那套固定长相，
   * 就是一个按实测走线并集算出来的矩形。
   */
  const slicerTower = boxOf(collision.slicerTowerBbox)

  /* 越界那一屏给的是**不裁剪**的轮廓（板外那截也在）—— 有它就用它 */
  const boundaryPolys = (pieces ?? [])
    .map((p) =>
      (p.polygon ?? [])
        .filter((q): q is [number, number] => Array.isArray(q) && q.length === 2)
        .map(([x, y]) => bedPt(x, y)),
    )
    .filter((poly) => poly.length >= 3)
  /*
   * **笔尖那一层**（2026-11-19 作者：「用不同的虚线去显示模型的和我的那个喷嘴的」）：
   * 同一批多边形整体平移一个笔偏移。与模型重叠的那一段不画（见下面的裁剪路径），
   * 否则两套虚线交错，反而看不出哪条是哪条。
   */
  const penPolys =
    boundaryPolys.length > 0 && (penX !== 0 || penY !== 0)
      ? (pieces ?? [])
          .map((p) =>
            (p.polygon ?? [])
              .filter((q): q is [number, number] => Array.isArray(q) && q.length === 2)
              .map(([x, y]) => bedPt(x + penX, y + penY)),
          )
          .filter((poly) => poly.length >= 3)
      : []
  /*
   * **走线投影**：机器坐标 → 板坐标（与 `bedPt` 同一套 y 翻转）。
   * 图是预先渲染好的 PNG，这里只算它该占哪一块。
   */
  const raster = collision.modelRaster ?? null
  const rasterRect = raster == null ? null : rasterScreenRect(raster, bedPt)

  /* 真实外形优先；旧记录没有这一格 ⇒ 退回凸包（与从前一致） */
  const modelPolys =
    boundaryPolys.length > 0
      ? boundaryPolys
      : (collision.modelOutlines?.length ? collision.modelOutlines : (collision.modelPolygons ?? []))
          .map((poly) =>
            poly
              .filter((p): p is [number, number] => Array.isArray(p) && p.length === 2)
              .map(([x, y]) => bedPt(x, y)),
          )
          .filter((poly) => poly.length >= 3)

  /**
   * 同一批模型几何画两遍：一遍正常色，一遍（越界那一屏）裁到"允许范围之外"再染红。
   * 走得通是因为两份只差 `className` 与裁剪 —— 不许为红色另算一套几何。
   */
  const modelShapes = (cls: string) =>
    modelPolys.length > 0
      ? modelPolys.map((poly, i) => (
          <polygon
            key={`${cls}-p${i}`}
            className={cls}
            points={poly.map(([x, y]) => `${x},${y}`).join(' ')}
          />
        ))
      : modelRects.map((b, i) => {
          const [lx, ty] = bedPt(b.x, b.y + b.h)
          return <rect key={`${cls}-r${i}`} className={cls} x={lx} y={ty} width={b.w} height={b.h} />
        })

  /* `noTower` 时 `tower` 恒为 `null`（上面就这么定的），这一串只给要画塔的那两屏用 */
  const towerShape = noTower ? null : (collision.tower ?? null)
  const bodyMm = towerShape?.bodyMm ?? tower?.w ?? 0
  /*
   * 塔图那张 svg 自己带"外围结构往外扩多少"的画布：扩展量取**判据框与塔身之差的一半**
   * —— 于是画出来的外沿正好压在虚线判据框上（两件事是同一件事实的两个说法）。
   */
  const expand = Math.max(
    0,
    (Math.min(tower?.w ?? bodyMm, tower?.h ?? bodyMm) - bodyMm) / 2,
  )
  const vm = TOWER_CORE_PX / (bodyMm > 0 ? bodyMm : 1)
  /*
   * viewBox 比判据框再多 2px 的描边余量（2026-10-21 作者：「我这一个塔不要被裁剪掉」）：
   * brim / 护套那几圈正好画到 viewBox 边缘，stroke 有一半落在框外 —— 不留这圈
   * 就会被 SVG 自己裁掉（截图里绿边缺一截、塔身像被削了角）。
   */
  const VB_GUARD_PX = 2
  const vbOffset = -expand * vm - VB_GUARD_PX
  const vbSize = TOWER_CORE_PX + expand * vm * 2 + VB_GUARD_PX * 2

  /* 冲突点：落进可打印区才画（框外的点画出来只会误导 —— 那不是撞的地方） */
  const points = (collision.conflictPoints ?? []).filter(
    ([x, y]) => x >= bed.x && x <= bed.x + bed.w && y >= bed.y && y <= bed.y + bed.h,
  )
  const dotR = Math.max(0.8, bed.w / 240)

  /* 塔的左下角在板坐标里（`noTower` 没有塔 ⇒ 不参与，下面那两处都带闸） */
  const [towerLeftPlate, towerTopPlate] =
    tower !== null ? bedPt(tower.x, tower.y + tower.h) : [0, 0]

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
  /**
   * 一支**如实**的箭头：起点 → 目标（长度 = |vec|）。挪塔那一屏与越界那一屏共用。
   *
   * `minLen`：越界那一屏给一个最小可见长度（位移可能只有 3mm，真长画出来看不见），
   * 挪塔那屏不传 —— 那边是"塔要挪多远"，长度必须如实。
   */
  const exactArrow = (
    from: [number, number],
    vec: [number, number],
    minLen = 0,
  ): { path: string; transform: string } | null => {
    if (vec[0] === 0 && vec[1] === 0) return null
    const [ox, oy] = bedPt(from[0], from[1])
    const [gx, gy] = bedPt(from[0] + vec[0], from[1] + vec[1])
    const len = Math.max(Math.hypot(gx - ox, gy - oy), minLen)
    const ang = Math.atan2(gy - oy, gx - ox)
    const path = arrowPathOf(len, canvasW)
    if (path === null) return null
    return {
      path,
      transform: `translate(${ox} ${oy}) rotate(${(ang * 180) / Math.PI})`,
    }
  }

  /*
   * **越界那一屏**：每块越界的模型一支箭头（中心 → 挪到位的位置）。
   * 长度 = 建议位移；短于「最小可见长度」时按最小值画（真实距离在右侧那句里）。
   */
  const boundaryArrows = (pieces ?? [])
    .map((p) => {
      const s = p.shift
      const b = boxOf(p.bbox ?? undefined)
      if (s === undefined || b === null) return null
      return exactArrow([b.x + b.w / 2, b.y + b.h / 2], s, minArrowLength(canvasW))
    })
    .filter((a): a is { path: string; transform: string } => a !== null)

  /* 序号徽标：与右栏那几条文字共用 `boundaryNumbering` 的排序（编号只有一个来源） */
  const badges = numbered
    ? boundaryNumbering(pieces ?? [])
        .map(({ n, piece }) => {
          const b = boxOf(piece.bbox ?? undefined)
          if (b === null) return null
          const [cx, cy] = bedPt(b.x + b.w / 2, b.y + b.h / 2)
          return { n, x: cx, y: cy }
        })
        .filter((v): v is { n: number; x: number; y: number } => v !== null)
    : []

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
      for (const b of [
        model,
        tower !== null ? { x: tower.x, y: tower.y, w: tower.w, h: tower.h } : null,
      ]) {
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
    arrow !== null || boundaryArrows.length > 0 || badges.length > 0 ? (
      <svg
        className={s.svg}
        style={{ zIndex: 3, pointerEvents: 'none' }}
        viewBox={`${view.x0} ${view.y0} ${view.w} ${view.h}`}
        preserveAspectRatio="xMidYMid meet"
        aria-hidden="true"
      >
        {arrow !== null && (
          <path className={s.arrow} d={arrow.path} transform={arrow.transform} />
        )}
        {boundaryArrows.map((a, i) => (
          <path key={`ba${i}`} className={s.arrowHot} d={a.path} transform={a.transform} />
        ))}
      </svg>
    ) : null

  /*
   * **序号徽标**（2026-11-20）：不再用 SVG `<text>` —— 放大看的时候数字与圆圈会分家
   * （作者原话：「这个数字 1 它也不是那个 SVG，好像是实时化的…放大这个图片会导致位置偏移」）。
   * 改成**图容器里的绝对定位元素**，位置按画布百分比算，缩放由同一层 `transform` 带着走；
   * 尺寸用 `--zoom-k` 反着除（放大层里也保持 ~20px，读得清又不糊住模型）。
   */
  /* 行内那份缩小一号（2026-11-23 作者：「小图的时候那个序号太大了……放大之后的刚刚好」），
     放大层原样 —— 只改这一处。 */
  const badgeNodes = (zoomed: boolean) =>
    badges.map((b) => (
      <span
        key={`bd${b.n}`}
        className={zoomed ? s.badge : `${s.badge} ${s.badgeSm}`}
        style={{
          left: pct(b.x - view.x0, view.w),
          top: pct(b.y - view.y0, view.h),
        }}
      >
        {b.n}
      </span>
    ))

  const factSvg = (
    <svg
      className={s.svg}
      viewBox={`${view.x0} ${view.y0} ${view.w} ${view.h}`}
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

      {/* **走线投影**（预先渲染好的 PNG）：垫在板子与轮廓之间 —— 打开记录只贴一张图，
          不读 G-code。旧记录没有这一格 ⇒ 不画。 */}
      {raster !== null && rasterRect !== null && (
        <image
          className={s.raster}
          href={`data:image/png;base64,${raster.pngBase64}`}
          x={rasterRect.x}
          y={rasterRect.y}
          width={rasterRect.w}
          height={rasterRect.h}
          preserveAspectRatio="none"
        />
      )}

      {/* 参考框（校准件的实测包围盒）：只圈位置，不参与任何判据 */}
      {box != null &&
        box.xMin !== undefined &&
        box.xMax !== undefined &&
        box.yMin !== undefined &&
        box.yMax !== undefined && (
          <rect
            className={s.guide}
            x={bedPt(box.xMin, box.yMax)[0]}
            y={bedPt(box.xMin, box.yMax)[1]}
            width={box.xMax - box.xMin}
            height={box.yMax - box.yMin}
          />
        )}

      {/* 模型轮廓（另一种颜色 + 虚线：一眼和塔分开）。
          ★ **一个模型一块轮廓**，不是矩形（2026-10-18 作者：「图片的都是矩形的看着好像
            碰撞了一样，能不能真的做成俯视图，多边形的」）——矩形四个角会把模型旁边的空
            算进去。内核给的是凸包（≤24 边）；旧记录没有这一格 ⇒ 退回矩形。 */}
      {modelShapes(s.model)}

      {/* **笔尖那一层**（2026-11-19 作者：「用不同的虚线去显示模型的和我的那个喷嘴的」）：
          同一批多边形整体平移一个笔偏移；与模型重叠的那一段裁掉不画 —— 两套虚线交错
          就分不清哪条是哪条了。只有越界那一屏给 `pen`，别的屏这一层不存在。 */}
      {penPolys.length > 0 && (
        <>
          <defs>
            <clipPath id={penClipId} clipPathUnits="userSpaceOnUse">
              <path
                clipRule="evenodd"
                d={
                  `M${view.x0},${view.y0}H${view.x0 + view.w}V${view.y0 + view.h}H${view.x0}Z ` +
                  modelPolys
                    .map((poly) => `M${poly.map(([x, y]) => `${x},${y}`).join('L')}Z`)
                    .join(' ')
                }
              />
            </clipPath>
          </defs>
          <g clipPath={`url(#${penClipId})`}>
            {penPolys.map((poly, i) => (
              <polygon
                key={`pen${i}`}
                className={s.pen}
                points={poly.map(([x, y]) => `${x},${y}`).join(' ')}
              />
            ))}
          </g>
        </>
      )}

      {/* **越界那一屏**（2026-11-19）：模型凸出允许范围的那一块染红。
          裁剪路径 = 整个视窗减去允许范围（evenodd 两个子路径）——
          于是"红的那片"与消息里那句"超出多少"是同一件事，不用另算一套。
          ★ 不画允许范围的框（作者 2026-11-19：「不要再有一个大轮廓瞄着热床」）。 */}
      {allowedRect !== null && (
        <>
          <defs>
            <clipPath id={outsideClipId} clipPathUnits="userSpaceOnUse">
              <path
                clipRule="evenodd"
                d={
                  `M${view.x0},${view.y0}H${view.x0 + view.w}V${view.y0 + view.h}H${view.x0}Z ` +
                  `M${allowedRect.x},${allowedRect.y}h${allowedRect.w}v${allowedRect.h}h${-allowedRect.w}Z`
                }
              />
            </clipPath>
          </defs>
          <g clipPath={`url(#${outsideClipId})`}>{modelShapes(s.overflow)}</g>
        </>
      )}

      {/* 判据框：碰撞按这一块算（虚线，压在塔的真实形状外沿上）。
          `noTower`（模型俯视图）没有塔 ⇒ 这一层不画。 */}
      {tower !== null && !hideTowerBox && (
        <rect
          className={s.towerBox}
          x={towerLeftPlate}
          y={towerTopPlate}
          width={tower.w}
          height={tower.h}
        />
      )}

      {/* 切片器自己的擦料塔：一个实时包围盒（绿色，与 MKP 塔的红色判据框分开） */}
      {slicerTower !== null && (
        <rect
          className={s.slicerTower}
          x={bedPt(slicerTower.x, slicerTower.y + slicerTower.h)[0]}
          y={bedPt(slicerTower.x, slicerTower.y + slicerTower.h)[1]}
          width={slicerTower.w}
          height={slicerTower.h}
        />
      )}

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
            /* 视窗被越界几何撑大时按新的比例铺（否则那块被压扁） */
            aspectRatio: `${view.w} / ${view.h}`,
            /* `height` 给了就**按高度定尺**（宽度反算）—— 两幅并排要比同高时用。
               ★ 放大档不认它（2026-10-18 真机）：放大层给的是"确定宽 + 高度上限"，
                 行内那个 104/132px 若照带进来，放大的仍是一张缩略图 ——
                 于是大壳子里躺着一张小图，FLIP 又从小图飞到大壳子（位置和大小都不对）。 */
            ...(height !== undefined && !zoomed
              ? { height: `${height}px`, width: 'auto' }
              : null),
          }}
        >
          {/* 板画在自己的坐标系里，只按视窗里的占比定位 —— 视窗被越界几何撑大时它缩进去，
              溢出那块就落在板外那块空白上（不缩的话会被画布边切掉）。 */}
          <div
            style={{
              position: 'absolute',
              left: pct(0 - view.x0, view.w),
              top: pct(0 - view.y0, view.h),
              width: pct(plate.w, view.w),
              height: pct(plate.d, view.h),
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
            {/* 塔的真实形状（绝对定位盖在判据框上，百分比按板坐标算） */}
            {towerShape !== null && tower !== null && (
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
                  /* 缩略图里塔只有 20px 上下，字按 0.34 只有 7px（糊）—— 小图放大一档，
                     进放大层再回到正常比例（2026-10-21 作者：「字不见了」） */
                  labelScale={zoomed ? 0.34 : 0.42}
                  className={s.core}
                />
              </div>
            )}
          </div>
          {factSvg}
          {arrowSvg}
          {badgeNodes(zoomed)}
        </div>
      ) : (
        /* 没板规格：退回圆角矩形热床（事实照画 —— 不因为装饰缺席吞掉整块图） */
        <div
          className={s.map}
          style={{
            /* 视窗被越界几何撑大时按新的比例铺 */
            aspectRatio: `${view.w} / ${view.h}`,
            /* 同 `.mapPlate`：放大档按宽度铺满（行内才用固定高度） */
            ...(height !== undefined && !zoomed
              ? { height: `${height}px`, width: 'auto' }
              : null),
          }}
        >
          {factSvg}
          {towerShape !== null && tower !== null && (
            <div
              className={s.tower}
              style={{
                /* 百分比按**视窗**算（视窗被越界几何撑大时它会缩） */
                left: pct(bedPt(tower.x, tower.y + tower.h)[0] - view.x0, view.w),
                top: pct(bedPt(tower.x, tower.y + tower.h)[1] - view.y0, view.h),
                width: pct(tower.w, view.w),
                height: pct(tower.h, view.h),
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
                /* 缩略图里塔只有 20px 上下，字按 0.34 只有 7px（糊）—— 小图放大一档，
                   进放大层再回到正常比例（2026-10-21 作者：「字不见了」） */
                labelScale={zoomed ? 0.34 : 0.42}
                className={s.core}
              />
            </div>
          )}
          {arrowSvg}
          {badgeNodes(zoomed)}
        </div>
      )

  return (
    <div className={s.wrap} data-layout={layout}>
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
        aria-label={mapLabel}
      >
        {renderMap(false)}
      </button>

      {/* 图例：三个色块 + 名字（不标数字 —— 数字在「建议」那组里）。
          挪塔那一屏两幅并排，图例各占一行太吵 ⇒ `hideLegend`；
          `noTower` 那一档没画塔、也没有冲突点 ⇒ 只剩"模型轮廓"一条（空的那几条不摆）。 */}
      {!hideLegend && (
      <ul className={s.legend}>
        {!noTower && (
          <li>
            <span className={s.swTower} aria-hidden="true" />
            {tr('report.map.legendTower')}
          </li>
        )}
        {slicerTower !== null && (
          <li>
            <span className={s.swSlicerTower} aria-hidden="true" />
            {tr('report.map.legendSlicerTower')}
          </li>
        )}
        {allowedRect !== null && (
          <li>
            <span className={s.swOverflow} aria-hidden="true" />
            {tr('report.map.legendOverflow')}
          </li>
        )}
        {box !== null && (
          <li>
            <span className={s.swGuide} aria-hidden="true" />
            {tr('report.map.legendBox')}
          </li>
        )}
        <li>
          <span className={s.swModel} aria-hidden="true" />
          {tr('report.map.legendModel')}
        </li>
        {penPolys.length > 0 && (
          <li>
            <span className={s.swPen} aria-hidden="true" />
            {tr('report.map.legendPen')}
          </li>
        )}
        {!noTower && (
          <li>
            <span className={s.swHit} aria-hidden="true" />
            {tr('report.map.legendHit')}
            {collision.conflictCount !== undefined && collision.conflictCount > 0 && (
              <span className={s.legendNum}>
                {tr('report.map.hitCount', { n: collision.conflictCount })}
              </span>
            )}
          </li>
        )}
      </ul>
      )}

      {/* 横排档的右栏：图例下面那几句说明（图被挤成薄片就是因为它们原来住在图底下） */}
      {children !== undefined && <div className={s.side}>{children}</div>}

      {/* 点开看大：报告页不在模态框 ⇒ portal 到 body；点任意处 / Esc 收起，滚轮 / 捏合缩放 */}
      {zoom &&
        createPortal(
          <ZoomLayer
            from={fromRef.current ?? new DOMRect(0, 0, 0, 0)}
            returnTo={() =>
              mapBtnRef.current?.firstElementChild?.getBoundingClientRect() ?? null
            }
            onGone={() => setZoom(false)}
            label={tr('report.map.zoomClose')}
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
