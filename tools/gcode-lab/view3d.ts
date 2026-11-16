/*
 * 3D：打印本体（墙 / 填充 / 支撑 / 支撑面 / 熨烫面）+ 涂胶路径，按真实 Z 堆起来。
 *
 * 挣位置的只有一件事 —— **层间漂移**：同一条边这层抬了、下层没抬，2D 里看不出来，
 * 堆起来就是一道豁口。别处（判定为什么错）仍然看 2D。
 *
 * 坐标用 Z 朝上（相机 `up` 设成 +Z），与 G-code 一致，免得读图时还要在脑子里翻。
 * 涂胶块的高度取 `;MKP_BEGIN paint … z=`（层高），不是笔尖 Z —— 笔尖比层高多出一个
 * 机械偏移，拿它画会浮在模型上方好几个毫米。
 *
 * # 层滑块
 *
 * `parse.ts` 把顶点按 Z 升序排好并给出每层的段数前缀，所以「看到第 i 层」只是
 * `geometry.setDrawRange`，切层不重建几何 —— 拖着滑块走是即时的。
 *
 * # 相机手感
 *
 * - **缩放朝画面中心**（`zoomToCursor = false`）：滚轮不改 `target`，旋转中心稳定；
 *   最小视距 0.02mm，仍然能一路推到某个面、甚至穿进去。
 * - **平移走 OrbitControls 自带的 pan**：它把相机与 target 一起挪，转起来绕的是
 *   画面中心那一点。machine-motion 那边反过来（中心钉在机器上、平移走投影 skew）——
 *   那是「一台机器」该有的手感，这里是一堆互相独立的层。
 * - **Shift = 精调**：三个速度一起缩到 1/6，同时左右键换手（左键平移、右键旋转）。
 * - **近裁面跟着视距走**（视距的 2%）：贴到毫米级也不裁。
 *
 * # 重画
 *
 * 拖拽/缩放的中间帧由 `requestAnimationFrame` 驱动（开阻尼就必须每帧 `controls.update()`，
 * 只在事件里画一次会「拖了没反应」）。颜色全部取自 `theme.ts`，系统换深浅色时整层重建。
 */

import * as THREE from 'three'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'

import type { LabBlock, PrintKind, PrintModel } from '../../src/app/glueLab/parse'
import { hex, palette } from './theme'

/** Shift 档的速度倍率：与 machine-motion 同一档（滚轮一格约 0.9% 视距） */
const NAV_FINE = 1 / 6
/** 滚轮一格推拉多少：OrbitControls 默认 0.95（5%/格）在本仓的机位下推不动 */
const ZOOM_SPEED = 2.2
const MIN_DISTANCE = 0.01
const MAX_DISTANCE = 3000
/** 涂胶块在 3D 里抬起一点点，免得和它贴着的支撑面同深度打架 */
const GLUE_LIFT_MM = 0.02
/**
 * 正交那台相机的基准半高（mm）。正交的画幅 = 基准 / `zoom`，
 * 于是 `zoom` 与透视的视距可以互换（见 `applyOrthoZoom` / `distance`）。
 */
const ORTHO_BASE = 100

/** 层滑块的两种走线视图（切片器里那一颗钮） */
export type LayerMode = 'current' | 'all'

const IDENTITY = new THREE.Quaternion()
/** 爆炸把 Z 倍数拉到多少（作者要的「炸成 10~15」取中间） */
const EXPLODE_Z = 12
/** 「视角」按钮回到的默认机位：前面在左、右面在右、顶面在顶 */
const DEFAULT_DIR = new THREE.Vector3(0.85, -1, 0.75).normalize()
/** 视角立方体点一下之后转过去的时长（秒） */
const SNAP_SECONDS = 0.3
/**
 * 2D→3D 回程时切透视的进度。
 *
 * 放在中段而不是最后一帧：那时相机还在远处、模型还没完全显出来，
 * 而且切的一瞬间会把透视 FOV 反算成与正交同画幅，看不出跳变。
 */
const PERSPECTIVE_SWITCH_AT = 0.2
/**
 * 两条曲线错峰：
 *  - 视距（画幅）在切透视之后很快收尾：**先快后慢**（easeOutQuad）；
 *  - 方向 / 目标点几乎同时起步，但**先慢后快**、末尾再收住（late smoothstep）。
 * 于是前段主要是缩放、后段主要是移动，中间自然交接，不再「先冲一段再转」。
 */
const FLY_DIST_END = 0.72
const FLY_MOVE_START = 0.12
const clamp01 = (v: number): number => Math.min(1, Math.max(0, v))
/** 先快后慢：起步就有速度，收尾没有 */
const easeOutQuad = (v: number): number => 1 - (1 - clamp01(v)) ** 2
/** 先慢后快、末端也收住；速度峰在 2/3 处 */
const easeLate = (v: number): number => {
  const x = clamp01(v)
  return x * x * x * (4 - 3 * x)
}

export class View3d {
  private readonly canvas: HTMLCanvasElement
  private readonly renderer: THREE.WebGLRenderer
  private readonly scene = new THREE.Scene()
  private readonly camP: THREE.PerspectiveCamera
  private readonly camO: THREE.OrthographicCamera
  /** 在看的是不是正交那台 */
  private ortho = false
  private readonly controls: OrbitControls
  private readonly group = new THREE.Group()
  private grid: THREE.GridHelper
  private allLayer: THREE.LineSegments | null = null
  private curLayer: THREE.LineSegments | null = null
  private print: PrintModel | null = null
  /** `${对象 id}:${类别}` → 那一块线段；对象过滤只改可见性，不重建几何 */
  private readonly printLayers = new Map<
    string,
    { layer: THREE.LineSegments; object: number; kind: PrintKind }
  >()
  private blocks: LabBlock[] = []
  /** 每块累计的**段**数（与 `blocks` 同序，升序）；层滑块切涂胶栈用 */
  private blockEdges: number[] = [0]
  private index = 0
  private layerIndex = -1
  private layerMode: LayerMode = 'all'
  private exaggeration = 20
  private visible = false
  private framed = false
  private raf = 0
  /** 视角立方体点完之后的那一段补间 */
  private tween: { q: THREE.Quaternion; from: THREE.Vector3; t: number } | null = null
  private last = 0
  private readonly baseSpeed = { rotate: 1, pan: 1, zoom: 1 }
  private readonly shown = new Set<string>([
    'glue',
    'outer',
    'inner',
    'fill',
    'support',
    'supportFace',
    'ironing',
  ])
  /** 只看某一个模型（`null` = 全看） */
  private objectFilter: number | null = null
  /** 涂胶相对「这一层表面」的额外抬高（mm）：把两个 Z 偏移调成 0 之后的差值 */
  private glueZShift = 0
  /** 每帧报一次视距（顶栏那把缩放滑杆要跟着滚轮走） */
  onFrame: ((distance: number) => void) | null = null

  /* ---- 爆炸图 ---- */
  private exploded = false
  /** 炸开这一段的补间：Z 倍数从多少到多少 */
  private explodeAnim: { from: number; to: number; t: number } | null = null
  /** 炸开之前的倍数与机位（收回去时还原） */
  private explodeHome: { z: number; dist: number; targetZ: number } | null = null
  /** 爆炸过程中把当前 Z 倍数报给顶栏（滑杆与数字要同步） */
  onExaggeration: ((z: number) => void) | null = null
  /** 进 2D 之前那个机位，回来时还原 */
  private homePose: {
    target: THREE.Vector3
    dir: THREE.Vector3
    dist: number
    up: THREE.Vector3
    zoom: number
    ortho: boolean
    fov: number
  } | null = null
  /**
   * 伪 2D 运镜：除了方向与距离，还插值 up / target / 正交 zoom，
   * 让最后一帧与 2D 的 `cx/cy/scale` 精确对上。
   */
  private flyAnim: {
    from: THREE.Vector3
    to: THREE.Vector3
    upFrom: THREE.Vector3
    upTo: THREE.Vector3
    targetFrom: THREE.Vector3
    targetTo: THREE.Vector3
    distFrom: number
    distTo: number
    zoomFrom: number
    zoomTo: number
    /** >=0 表示途中要换投影；值为切换进度 */
    switchAt: number
    switched: boolean
    /** 途中换到哪一台 */
    switchTo: 'ortho' | 'perspective' | null
    /** 正交画幅半高 → 终点画幅半高；换投影后用它们反算 FOV / zoom */
    halfFrom: number
    halfTo: number
    /** 'none' = 不动可见性（复位 / 换投影用） */
    fade: 'out' | 'in' | 'none'
    t: number
    done: () => void
  } | null = null
  /** 过渡期间 2D 只看当前那一块：3D 的当前块也要强制露出来 */
  private pseudo = false

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true })
    this.renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1))

    this.camP = new THREE.PerspectiveCamera(42, 1, 0.1, 5000)
    this.camP.up.set(0, 0, 1)
    this.camP.position.set(180, -180, 160)

    this.camO = new THREE.OrthographicCamera(
      -ORTHO_BASE,
      ORTHO_BASE,
      ORTHO_BASE,
      -ORTHO_BASE,
      0.1,
      100000,
    )
    this.camO.up.set(0, 0, 1)
    this.camO.position.copy(this.camP.position)

    this.controls = new OrbitControls(this.camP, canvas)
    this.controls.enableDamping = true
    this.controls.dampingFactor = 0.12
    this.controls.zoomToCursor = false
    this.controls.enablePan = true
    this.controls.zoomSpeed = ZOOM_SPEED
    this.controls.minDistance = MIN_DISTANCE
    this.controls.maxDistance = MAX_DISTANCE
    this.baseSpeed = {
      rotate: this.controls.rotateSpeed,
      pan: this.controls.panSpeed,
      zoom: this.controls.zoomSpeed,
    }

    this.grid = this.buildGrid()
    this.scene.add(this.grid)
    this.scene.add(this.group)

    this.bindNav()
  }

  setBlocks(blocks: LabBlock[], index: number, reframe = true): void {
    this.blocks = blocks
    this.rebuildGlue()
    this.select(index)
    this.applyRanges()
    if (!reframe) return
    this.framed = false
    if (this.visible) this.frame()
  }

  /** 打印本体：每类一层，开关按类独立；层滑块靠 `bounds` 切 */
  setPrint(print: PrintModel): void {
    this.print = print
    this.rebuildPrint()
    this.applyRanges()
    this.framed = false
    if (this.visible) this.frame()
  }

  setKindVisible(kind: string, visible: boolean): void {
    if (visible) this.shown.add(kind)
    else this.shown.delete(kind)
    this.syncVisibility()
  }

  /** 层滑块：`index < 0` = 全看 */
  setLayer(index: number, mode: LayerMode): void {
    this.layerIndex = index
    this.layerMode = mode
    this.applyRanges()
    this.syncVisibility()
  }

  select(index: number): void {
    this.index = index
    if (this.curLayer) {
      this.group.remove(this.curLayer)
      this.curLayer.geometry.dispose()
      ;(this.curLayer.material as THREE.Material).dispose()
      this.curLayer = null
    }
    const block = this.blocks[index]
    if (!block) return
    const layer = this.buildGlue([block], true, 1)
    this.curLayer = layer
    this.group.add(layer)
    this.group.scale.set(1, 1, this.exaggeration)
    this.syncVisibility()
  }

  setExaggeration(n: number): void {
    this.exaggeration = n
    this.group.scale.set(1, 1, n)
  }

  /**
   * 回到默认视角：朝向、目标点、距离一起补间。
   *
   * 默认机位是「前面在左、右面在右、顶面在顶」；不管中途被伪 2D 把 up 翻成 +Y，
   * 这里都把它插回 +Z，不会停在歪掉的取景上。
   */
  resetView(): void {
    this.group.updateMatrixWorld(true)
    const box = this.visibleBox()
    if (box.isEmpty()) return
    const center = box.getCenter(new THREE.Vector3())
    const dist = this.fitDistance(Math.max(1, box.getSize(new THREE.Vector3()).length() / 2))
    const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
    const distFrom = this.distance()
    const fromUp = this.cam.up.clone()
    this.flyAnim = {
      from: this.cam.position.clone().sub(this.controls.target).normalize(),
      to: DEFAULT_DIR.clone(),
      upFrom: fromUp,
      upTo: new THREE.Vector3(0, 0, 1),
      targetFrom: this.controls.target.clone(),
      targetTo: center,
      distFrom,
      distTo: dist,
      zoomFrom: this.ortho ? this.camO.zoom : 1,
      zoomTo: this.ortho ? ORTHO_BASE / Math.max(1e-9, dist * halfTan) : 1,
      switchAt: -1,
      switched: false,
      switchTo: null,
      halfFrom: distFrom * halfTan,
      halfTo: dist * halfTan,
      fade: 'none',
      t: 0,
      done: () => {},
    }
    this.controls.enabled = false
  }

  /**
   * 只换位置、不换朝向：绕**当前视线方向**重新对中、重新缩放。
   *
   * 与「视角」的区别：那一颗会把朝向也掰回默认的等轴测，这一颗只把你现在看的
   * 这个方向摆正 —— 转到满意的角度之后用它把模型放回画面里。
   */
  refit(instant = false): void {
    const dir = this.cam.position.clone().sub(this.controls.target)
    if (dir.lengthSq() < 1e-9) return
    dir.normalize()
    this.group.updateMatrixWorld(true)
    const box = this.visibleBox()
    if (box.isEmpty()) return
    const center = box.getCenter(new THREE.Vector3())
    const dist = this.fitDistance(Math.max(1, box.getSize(new THREE.Vector3()).length() / 2))
    if (instant) {
      this.controls.target.copy(center)
      this.cam.position.copy(center).addScaledVector(dir, dist)
      this.applyOrthoZoom()
      this.controls.update()
      return
    }
    const distFrom = this.distance()
    const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
    this.flyAnim = {
      from: dir,
      to: dir.clone(),
      upFrom: this.cam.up.clone(),
      upTo: this.cam.up.clone(),
      targetFrom: this.controls.target.clone(),
      targetTo: center,
      distFrom,
      distTo: dist,
      zoomFrom: this.ortho ? this.camO.zoom : 1,
      zoomTo: this.ortho ? ORTHO_BASE / Math.max(1e-9, dist * halfTan) : 1,
      switchAt: -1,
      switched: false,
      switchTo: null,
      halfFrom: distFrom * halfTan,
      halfTo: dist * halfTan,
      fade: 'none',
      t: 0,
      done: () => {},
    }
    this.controls.enabled = false
  }

  isOrtho(): boolean {
    return this.ortho
  }

  isExploded(): boolean {
    return this.exploded
  }

  /**
   * 切 2D 之前的运镜。
   *
   * 方向翻到正上方 (+Z)，但 **up 翻到 +Y** —— 这样屏幕右 = 世界 +X、屏幕上 = 世界 +Y，
   * 与 2D 的 `toScreen()` 同一朝向。之前 up 一直是 +Z，顶视时会绕着 Z 翻，方向就是反的。
   *
   * `view` 是 2D 当前的 `cx/cy/scale/w/h`：最后一帧的正交相机 target 与 zoom 由它反算，
   * 于是 3D 末帧与 2D 首帧是同一张画面，不需要淡入淡出。
   */
  flyToTop(view: { cx: number; cy: number; scale: number; w: number; h: number }, done: () => void): void {
    /*
     * 先快照再切正交：`setOrtho(true)` 会把 ortho / zoom 改掉，
     * 那时再记就记成「伪 2D 用的正交档」，回不去用户原本的透视机位。
     */
    const offset = this.cam.position.clone().sub(this.controls.target)
    if (offset.lengthSq() < 1e-9) {
      done()
      return
    }
    const dir = offset.clone().normalize()
    this.homePose = {
      target: this.controls.target.clone(),
      dir: dir.clone(),
      dist: offset.length(),
      up: this.cam.up.clone(),
      zoom: this.cam.zoom,
      ortho: this.ortho,
      fov: this.camP.fov,
    }
    if (!this.ortho) this.setOrtho(true, true)
    this.pseudo = true
    this.syncVisibility()
    const zoom = this.zoomForView(view)
    this.flyAnim = {
      from: dir,
      to: new THREE.Vector3(0, 0, 1),
      upFrom: this.cam.up.clone(),
      upTo: new THREE.Vector3(0, 1, 0),
      targetFrom: this.controls.target.clone(),
      targetTo: new THREE.Vector3(view.cx, view.cy, 0),
      distFrom: offset.length(),
      distTo: this.distanceForView(view),
      zoomFrom: this.camO.zoom,
      zoomTo: zoom,
      switchAt: -1,
      switched: false,
      switchTo: null,
      halfFrom: 0,
      halfTo: 0,
      fade: 'out',
      t: 0,
      done,
    }
    this.controls.enabled = false
  }

  /** 2D 的 px/mm → 正交 zoom：`screen_px = world_mm × scale` */
  private zoomForView(view: { scale: number; h: number }): number {
    return (2 * ORTHO_BASE * Math.min(400, Math.max(0.4, view.scale))) / Math.max(1, view.h)
  }

  /**
   * 2D 当前缩放对应的**等效 3D 视距**。
   *
   * 以前伪顶视把相机硬放在 1000mm，结果从 2D 切回 3D 时视距先从 2D 的值
   * 跳到 1000 再收到 home —— 看起来就是「先把视距拉得很大再缩小」。
   * 正交下距离不影响画幅，但切透视时它就是起点；这里按同一个 scale 反算，
   * 于是第一帧既同画面、又同视距。
   */
  private distanceForView(view: { scale: number; h: number }): number {
    const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
    return ORTHO_BASE / Math.max(1e-9, this.zoomForView(view) * halfTan)
  }

  /**
   * 瞬时把 3D 摆成「当前 2D」的伪顶视。
   *
   * 切回 3D 的那一帧必须接在 2D 最后一帧上：用户在 2D 里平移/缩放过，旧的 3D 顶视
   * 就不作数了。这里用同一个 `cx/cy/scale` 反算 target 与 zoom，且在同一 JS 任务里
   * 完成，浏览器没机会先画出旧相机。
   */
  snapToPseudo(view: { cx: number; cy: number; scale: number; w: number; h: number }): void {
    if (!this.ortho) this.setOrtho(true, true)
    const target = new THREE.Vector3(view.cx, view.cy, 0)
    const dist = this.distanceForView(view)
    this.controls.target.copy(target)
    this.camO.up.set(0, 1, 0)
    this.camO.position.copy(target).add(new THREE.Vector3(0, 0, dist))
    this.camO.lookAt(target)
    this.camO.zoom = this.zoomForView(view)
    this.camO.updateProjectionMatrix()
    this.controls.object = this.camO
    this.pseudo = true
    this.setFade(0)
    this.syncVisibility()
  }

  /** 从伪 2D 的顶视飞回进入 2D 前的机位，并把渐隐的东西带回来 */
  flyFromTop(
    view: { cx: number; cy: number; scale: number; w: number; h: number },
    done: () => void,
  ): void {
    const home = this.homePose
    /* 从没进过 2D（首帧就是 2D）：不要摆成伪顶视，按普通 3D 走 */
    if (home === null) {
      done()
      return
    }
    this.snapToPseudo(view)
    const homeTan = Math.tan((home.fov * Math.PI) / 360)
    this.flyAnim = {
      from: new THREE.Vector3(0, 0, 1),
      to: home.dir.clone(),
      upFrom: this.camO.up.clone(),
      upTo: home.up.clone(),
      targetFrom: this.controls.target.clone(),
      targetTo: home.target.clone(),
      distFrom: this.camO.position.distanceTo(this.controls.target),
      distTo: home.dist,
      zoomFrom: this.camO.zoom,
      zoomTo: home.ortho ? home.zoom : ORTHO_BASE / Math.max(1e-9, home.dist * homeTan),
      /* 透视档：中段切，别把投影切换堆在最后一帧 */
      switchAt: home.ortho ? -1 : PERSPECTIVE_SWITCH_AT,
      switched: false,
      switchTo: home.ortho ? null : 'perspective',
      halfFrom: ORTHO_BASE / Math.max(1e-9, this.camO.zoom),
      halfTo: home.dist * homeTan,
      fade: 'in',
      t: 0,
      done,
    }
    this.controls.enabled = false
  }

  /** 切正交：和 `enterPerspective` 反过来，画幅保持同一半高 */
  private enterOrtho(half: number): void {
    this.camO.position.copy(this.camP.position)
    this.camO.quaternion.copy(this.camP.quaternion)
    this.camO.up.copy(this.camP.up)
    this.camO.zoom = ORTHO_BASE / Math.max(1e-9, half)
    this.camO.updateProjectionMatrix()
    this.ortho = true
    this.controls.object = this.camO
  }

  /**
   * 切透视的那一次：把透视 FOV 反算成当前正交画幅，
   * 于是切的一帧在 target 平面上像素对得上；相机又还在远处，纵深差也看不出来。
   */
  private enterPerspective(half: number, dist: number): void {
    const fov = (2 * Math.atan(half / Math.max(1e-9, dist)) * 180) / Math.PI
    this.camP.fov = Math.min(120, Math.max(1, fov))
    this.camP.position.copy(this.camO.position)
    this.camP.quaternion.copy(this.camO.quaternion)
    this.camP.up.copy(this.camO.up)
    this.camP.zoom = 1
    this.camP.updateProjectionMatrix()
    this.ortho = false
    this.controls.object = this.camP
  }

  /** 记录每种线原本的不透明度，过渡时按比例渐隐，结束再原样恢复 */
  private setFade(k: number): void {
    const fade = (obj: THREE.Object3D | null): void => {
      if (obj === null) return
      const mat = (obj as THREE.Object3D & { material?: THREE.Material }).material as
        | (THREE.Material & { opacity: number })
        | undefined
      if (!mat) return
      const base = (obj.userData.baseOpacity as number | undefined) ?? mat.opacity
      obj.userData.baseOpacity = base
      mat.transparent = true
      mat.opacity = base * k
    }
    fade(this.grid)
    if (this.allLayer) fade(this.allLayer)
    for (const { layer } of this.printLayers.values()) fade(layer)
  }

  /**
   * 爆炸图：每层沿 Z 抬高，同时把镜头往后拉、顺着往上抬。
   *
   * 「往上」是必须的：炸开之后整摞往上长，镜头不跟着抬就会有一半跑出画面；
   * 距离也要跟着放大，否则层间距还是挤在一起，等于没炸开。
   */
  toggleExplode(): void {
    this.exploded = !this.exploded
    if (this.exploded) {
      /* 先把模型摆回画面正中，再开始炸 —— 不然是从画面外飞进来的 */
      this.refit(true)
      this.explodeHome = {
        z: this.exaggeration,
        dist: this.distance(),
        targetZ: this.controls.target.z,
      }
    }
    const home = this.explodeHome
    this.explodeAnim = {
      /* 收回去时从**当前**倍数起步，不要从 home 跳过去 */
      from: this.exaggeration,
      to: this.exploded ? EXPLODE_Z : (home?.z ?? this.exaggeration),
      t: 0,
    }
    this.controls.enabled = false
  }

  /**
   * 透视 / 正交。按钮走补间：中途换相机，前后画幅不变；
   * 内部切伪 2D 时传 `instant`，不要和运镜叠在一起。
   */
  setOrtho(on: boolean, instant = false): void {
    if (on === this.ortho) return
    if (instant) {
      const from = this.cam
      const d = this.distance()
      this.ortho = on
      const to = this.cam
      to.up.copy(from.up)
      to.quaternion.copy(from.quaternion)
      to.position.copy(from.position)
      if (on) {
        this.applyOrthoZoom()
      } else {
        const dir = to.position.clone().sub(this.controls.target)
        const len = dir.length()
        if (len > 1e-9) to.position.copy(this.controls.target).addScaledVector(dir.divideScalar(len), d)
      }
      this.controls.object = to
      this.controls.update()
      return
    }
    const half = this.ortho
      ? ORTHO_BASE / Math.max(1e-9, this.camO.zoom)
      : this.distance() * Math.tan((this.camP.fov * Math.PI) / 360)
    const dir = this.cam.position.clone().sub(this.controls.target).normalize()
    this.flyAnim = {
      from: dir,
      to: dir.clone(),
      upFrom: this.cam.up.clone(),
      upTo: this.cam.up.clone(),
      targetFrom: this.controls.target.clone(),
      targetTo: this.controls.target.clone(),
      distFrom: this.distance(),
      distTo: this.distance(),
      /* 换相机前后各自认自己的 zoom，不要在另一台的 tick 分支里被写坏 */
      zoomFrom: on ? ORTHO_BASE / Math.max(1e-9, half) : this.camO.zoom,
      zoomTo: on ? ORTHO_BASE / Math.max(1e-9, half) : this.camO.zoom,
      switchAt: 0.5,
      switched: false,
      switchTo: on ? 'ortho' : 'perspective',
      halfFrom: half,
      halfTo: half,
      fade: 'none',
      t: 0,
      done: () => {},
    }
    this.controls.enabled = false
  }

  /** 当前那台相机 */
  private get cam(): THREE.PerspectiveCamera | THREE.OrthographicCamera {
    return this.ortho ? this.camO : this.camP
  }

  /** 视距（正交档给的是**等效**视距：同样的 `zoom` 换成透视该站多远） */
  distance(): number {
    if (!this.ortho) return this.camP.position.distanceTo(this.controls.target)
    const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
    return ORTHO_BASE / Math.max(1e-9, this.camO.zoom * halfTan)
  }

  /** 视角立方体每帧要它：立方体的朝向 = 这个东西的逆 */
  cameraQuaternion(): THREE.Quaternion {
    return this.cam.quaternion.clone()
  }

  /** 转到某个方向（视角立方体点面 / 点角都走这里） */
  setDirection(dir: THREE.Vector3): void {
    const target = this.controls.target
    const from = this.cam.position.clone().sub(target)
    if (from.lengthSq() < 1e-9) return
    const to = dir.clone().normalize()
    /*
     * 侧面：屏幕上方是世界 +Z；顶 / 底：视线已平行于 +Z，改用 ±Y 当 up。
     * 于是点「前面 / 右面」是正对的正方形，点「顶部」是正俯视，
     * 不用再把方向扭 0.1° 去绕开 lookAt 退化。
     */
    const up =
      Math.abs(to.z) > 0.999 ? new THREE.Vector3(0, Math.sign(to.z), 0) : new THREE.Vector3(0, 0, 1)
    this.camP.up.copy(up)
    this.camO.up.copy(up)
    const q = new THREE.Quaternion().setFromUnitVectors(from.clone().normalize(), to)
    this.tween = { q, from, t: 0 }
    this.controls.enabled = false
  }

  /** 缩放滑杆：沿当前视线改视距，朝向与 target 不动 */
  setDistance(d: number): void {
    const want = Math.min(MAX_DISTANCE, Math.max(MIN_DISTANCE, d))
    if (this.ortho) {
      /* 正交没有「远近」，改的是画幅：等效视距 → zoom */
      const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
      this.camO.zoom = ORTHO_BASE / (want * halfTan)
      this.camO.updateProjectionMatrix()
      return
    }
    const dir = this.camP.position.clone().sub(this.controls.target)
    const len = dir.length()
    if (len < 1e-9) return
    this.camP.position.copy(this.controls.target).addScaledVector(dir.divideScalar(len), want)
    this.controls.update()
  }

  /** 系统换深浅色：网格与所有线段的颜色都要重建（顶点色烘在 geometry 里） */
  refreshTheme(): void {
    this.scene.remove(this.grid)
    this.grid.geometry.dispose()
    ;(this.grid.material as THREE.Material).dispose()
    this.grid = this.buildGrid()
    this.scene.add(this.grid)
    this.rebuildPrint()
    this.rebuildGlue()
    this.select(this.index)
    this.applyRanges()
  }

  /** 2D/3D 切换。第一次显示时再量尺寸 + 取景，隐藏时画布是 0×0 */
  setVisible(visible: boolean): void {
    this.visible = visible
    this.canvas.classList.toggle('hidden', !visible)
    if (!visible) {
      this.stop()
      return
    }
    this.resize()
    if (!this.framed) this.frame()
    this.start()
  }

  resize(): void {
    const rect = this.canvas.getBoundingClientRect()
    const w = Math.max(1, rect.width)
    const h = Math.max(1, rect.height)
    this.renderer.setSize(w, h, false)
    this.camP.aspect = w / h
    this.camP.updateProjectionMatrix()
    this.camO.left = -ORTHO_BASE * (w / h)
    this.camO.right = ORTHO_BASE * (w / h)
    this.camO.updateProjectionMatrix()
  }

  /* ---------------- 内部 ---------------- */

  private bindNav(): void {
    const apply = (shift: boolean): void => {
      const k = shift ? NAV_FINE : 1
      this.controls.rotateSpeed = this.baseSpeed.rotate * k
      this.controls.panSpeed = this.baseSpeed.pan * k
      this.controls.zoomSpeed = this.baseSpeed.zoom * k
      /*
       * Shift 换手：左键平移、右键旋转。
       * OrbitControls 自己在 `onPointerDown` 里看到 shiftKey 会把左右再对调一次，
       * 所以这里要先写成「对调之后正好」的那一套 —— 用的是公开的 `mouseButtons` 配置。
       */
      this.controls.mouseButtons.LEFT = shift ? THREE.MOUSE.PAN : THREE.MOUSE.ROTATE
      this.controls.mouseButtons.RIGHT = shift ? THREE.MOUSE.ROTATE : THREE.MOUSE.PAN
    }
    /* capture：OrbitControls 的监听没带 capture，先跑才能把键位换在它读之前 */
    this.canvas.addEventListener('pointerdown', (e) => apply(e.shiftKey), true)
    this.canvas.addEventListener('pointermove', (e) => apply(e.shiftKey), true)
    this.canvas.addEventListener('wheel', (e) => apply(e.shiftKey), {
      capture: true,
      passive: true,
    })
    this.canvas.addEventListener('contextmenu', (e) => e.preventDefault())
  }

  private buildGrid(): THREE.GridHelper {
    const p = palette()
    const color = hex(p.line, 0x232a33)
    const grid = new THREE.GridHelper(320, 32, color, color)
    grid.rotation.x = Math.PI / 2
    const mat = grid.material as THREE.Material
    mat.transparent = true
    mat.opacity = 0.6
    grid.userData.baseOpacity = 0.6
    return grid
  }

  /* ---- 涂胶 ---- */

  private buildGlue(
    blocks: readonly LabBlock[],
    colored: boolean,
    opacity: number,
  ): THREE.LineSegments {
    const p = palette()
    const normal = new THREE.Color(hex(p.teal, 0x2bb6a8))
    const raised = new THREE.Color(hex(p.amber, 0xf0a020))
    const dim = new THREE.Color(hex(p.dim, 0x8b949e))
    const positions: number[] = []
    const colors: number[] = []
    for (const block of blocks) {
      const anchor = blockLayerZ(block)
      /*
       * 落笔档最低的那个笔尖 Z 当基准，其余按**相对高度**画：
       * 于是「抬 0.5」就真的是 0.5mm，而不是被压成一条线或放大成 10mm。
       * 基准对齐到这一块自己的层高上 —— 涂胶本来就是落在那一层的表面上。
       */
      const base = block.normalZ ?? anchor
      for (const m of block.moves) {
        if (m.level === 'up') continue
        const z = anchor + (m.z - base) + this.glueZShift + GLUE_LIFT_MM
        positions.push(m.from.x, m.from.y, z, m.to.x, m.to.y, z)
        const c = colored ? (m.level === 'raised' ? raised : normal) : dim
        colors.push(c.r, c.g, c.b, c.r, c.g, c.b)
      }
    }
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3))
    geo.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3))
    /* 涂胶是这个试验台的主角：关掉深度测试，被模型挡住时也画在最上面 */
    const mat = new THREE.LineBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity,
      depthTest: !colored,
    })
    const layer = new THREE.LineSegments(geo, mat)
    if (colored) layer.renderOrder = 3
    return layer
  }

  private rebuildGlue(): void {
    if (this.allLayer) {
      this.group.remove(this.allLayer)
      this.allLayer.geometry.dispose()
      ;(this.allLayer.material as THREE.Material).dispose()
      this.allLayer = null
    }
    this.allLayer = this.buildGlue(this.blocks, false, 0.4)
    this.allLayer.userData.baseOpacity = 0.4
    this.group.add(this.allLayer)

    this.blockEdges = [0]
    let seg = 0
    for (const block of this.blocks) {
      for (const m of block.moves) if (m.level !== 'up') seg += 1
      this.blockEdges.push(seg)
    }
  }

  /* ---- 打印本体 ---- */

  private rebuildPrint(): void {
    for (const { layer } of this.printLayers.values()) {
      this.group.remove(layer)
      layer.geometry.dispose()
      ;(layer.material as THREE.Material).dispose()
    }
    this.printLayers.clear()
    if (this.print === null) return

    const p = palette()
    const tone: Record<PrintKind, { hex: number; opacity: number }> = {
      outer: { hex: hex(p.text, 0xe6edf3), opacity: 0.85 },
      inner: { hex: hex(p.dim, 0x8b949e), opacity: 0.55 },
      fill: { hex: hex(p.dim, 0x8b949e), opacity: 0.3 },
      support: { hex: hex(p.green, 0x3fb950), opacity: 0.45 },
      supportFace: { hex: hex(p.green, 0x3fb950), opacity: 0.85 },
      ironing: { hex: hex(p.violet, 0xa371f7), opacity: 0.9 },
      travel: { hex: hex(p.dim, 0x8b949e), opacity: 0.14 },
      other: { hex: hex(p.dim, 0x8b949e), opacity: 0.3 },
    }
    for (const obj of this.print.objects) {
      for (const [kind, data] of Object.entries(obj.kinds) as [PrintKind, Float32Array][]) {
        if (data.length === 0) continue
        const geo = new THREE.BufferGeometry()
        geo.setAttribute('position', new THREE.BufferAttribute(data, 3))
        const t = tone[kind]
        const mat = new THREE.LineBasicMaterial({
          color: t.hex,
          transparent: true,
          opacity: t.opacity,
        })
        const layer = new THREE.LineSegments(geo, mat)
        layer.userData.baseOpacity = t.opacity
        this.printLayers.set(`${obj.id}:${kind}`, { layer, object: obj.id, kind })
        this.group.add(layer)
      }
    }
  }

  /* ---- 层滑块 / 可见性 ---- */

  /** 把「看到第 i 层」翻成 `drawRange`（顶点数 = 段数 × 2） */
  private applyRanges(): void {
    const p = this.print
    if (p !== null && p.layers.length > 0) {
      for (const { layer, object, kind } of this.printLayers.values()) {
        const obj = p.objects.find((o) => o.id === object)
        if (obj === undefined) continue
        const edge = obj.bounds[kind]
        const total = layer.geometry.getAttribute('position').count
        if (this.layerIndex < 0) {
          layer.geometry.setDrawRange(0, total)
          continue
        }
        const i = Math.min(this.layerIndex, p.layers.length - 1)
        const from = this.layerMode === 'all' ? 0 : (edge[i] as number)
        const to = edge[i + 1] as number
        layer.geometry.setDrawRange(from * 2, (to - from) * 2)
      }
    }
    if (this.allLayer) {
      const total = this.allLayer.geometry.getAttribute('position').count
      const cut = this.glueCutSegments()
      this.allLayer.geometry.setDrawRange(0, cut === null ? total : cut * 2)
    }
  }

  /** 涂胶栈切到第几段为止（`null` = 不切） */
  private glueCutSegments(): number | null {
    if (this.layerIndex < 0 || this.print === null) return null
    const z = this.print.layers[Math.min(this.layerIndex, this.print.layers.length - 1)]
    if (z === undefined) return null
    let last = 0
    for (let i = 0; i < this.blocks.length; i += 1) {
      const block = this.blocks[i] as LabBlock
      if (blockLayerZ(block) <= z + 1e-9) last = i + 1
    }
    return this.blockEdges[last] ?? null
  }

  private syncVisibility(): void {
    const glueOn = this.shown.has('glue')
    const onlyLayer = this.layerMode === 'current' && this.layerIndex >= 0
    const glueObjectOk = this.glueInFilter()
    /* 只看某一个模型时，那一摞「所有层的涂胶」也收起来：它不属于任何一个模型 */
    if (this.allLayer) this.allLayer.visible = glueOn && !onlyLayer && this.objectFilter === null
    /* 当前那一块涂胶：对象过滤时按它的位置判 */
    if (this.curLayer) this.curLayer.visible = (glueOn && glueObjectOk) || this.pseudo
    for (const { layer, object, kind } of this.printLayers.values()) {
      layer.visible = this.shown.has(kind) && (this.objectFilter === null || this.objectFilter === object)
    }
  }

  /** 当前这一块涂胶在不在被选中的对象框里 */
  private glueInFilter(): boolean {
    if (this.objectFilter === null) return true
    const block = this.blocks[this.index]
    if (!block?.bbox) return false
    const box = this.objectBox(this.objectFilter)
    if (box === null) return true
    const cx = (block.bbox.minX + block.bbox.maxX) / 2
    const cy = (block.bbox.minY + block.bbox.maxY) / 2
    return cx >= box.minX && cx <= box.maxX && cy >= box.minY && cy <= box.maxY
  }

  /** 某个对象的平面范围（取它所有线段的包围盒） */
  private objectBox(id: number): { minX: number; minY: number; maxX: number; maxY: number } | null {
    const obj = this.print?.objects.find((o) => o.id === id)
    if (obj === undefined) return null
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const data of Object.values(obj.kinds) as Float32Array[]) {
      for (let i = 0; i < data.length; i += 3) {
        const x = data[i] as number
        const y = data[i + 1] as number
        if (x < minX) minX = x
        if (x > maxX) maxX = x
        if (y < minY) minY = y
        if (y > maxY) maxY = y
      }
    }
    return Number.isFinite(minX) ? { minX, minY, maxX, maxY } : null
  }

  /** 涂胶相对层表面的额外抬高（mm）：用户把两个 Z 偏移调成 0 之后的差值 */
  setGlueZShift(dz: number): void {
    this.glueZShift = dz
    this.rebuildGlue()
    this.select(this.index)
  }

  /** 只显示某一个模型（`null` = 全看） */
  setObjectFilter(id: number | null): void {
    this.objectFilter = id
    this.syncVisibility()
  }

  /* ---- 相机 ---- */

  /** 可见几何的外接框（关掉的层不算，免得画面被看不见的东西撑大） */
  private visibleBox(): THREE.Box3 {
    const box = new THREE.Box3()
    for (const child of this.group.children) {
      if (!child.visible) continue
      box.expandByObject(child)
    }
    return box
  }

  /** 半径这么大的一坨东西，透视要站多远才装得下（正交按等效视距换算） */
  private fitDistance(radius: number): number {
    const fov = (this.camP.fov * Math.PI) / 180
    return (radius / Math.sin(fov / 2)) * 1.05
  }

  /** 正交画幅跟到当前视距：`zoom = 基准 / (视距 × tan(fov/2))` */
  private applyOrthoZoom(): void {
    const d = this.camO.position.distanceTo(this.controls.target)
    const halfTan = Math.tan((this.camP.fov * Math.PI) / 360)
    this.camO.zoom = ORTHO_BASE / Math.max(1e-9, d * halfTan)
    this.camO.updateProjectionMatrix()
  }

  private frame(): void {
    this.group.updateMatrixWorld(true)
    const box = this.visibleBox()
    if (box.isEmpty()) return
    const center = box.getCenter(new THREE.Vector3())
    const radius = Math.max(1, box.getSize(new THREE.Vector3()).length() / 2)
    this.camP.up.set(0, 0, 1)
    this.camO.up.set(0, 0, 1)
    this.controls.target.copy(center)
    this.cam.position.copy(center).addScaledVector(DEFAULT_DIR, this.fitDistance(radius))
    this.applyOrthoZoom()
    this.controls.update()
    this.framed = true
  }

  private switchingToPerspective(a: NonNullable<View3d['flyAnim']>): boolean {
    return a.switchTo === 'perspective' && a.switchAt >= 0
  }

  private switchingToOrtho(a: NonNullable<View3d['flyAnim']>): boolean {
    return a.switchTo === 'ortho' && a.switchAt >= 0
  }

  /** 近裁面 = 视距的 2%（下界 0.01mm）：远景保住深度精度，贴到毫米级也不裁 */
  private syncNear(): void {
    /* 正交是平行投影，近远裁面不会卡到贴脸的面，别跟着视距动 */
    if (this.ortho) return
    const d = this.distance()
    const near = Math.max(0.01, d * 0.02)
    const far = Math.max(1000, d * 20)
    if (
      Math.abs(this.camP.near - near) > near * 0.05 ||
      Math.abs(this.camP.far - far) > far * 0.05
    ) {
      this.camP.near = near
      this.camP.far = far
      this.camP.updateProjectionMatrix()
    }
  }

  private readonly tick = (): void => {
    this.raf = requestAnimationFrame(this.tick)
    const now = performance.now()
    const dt = this.last === 0 ? 1 / 60 : Math.min(0.1, (now - this.last) / 1000)
    this.last = now

    let flyDone: (() => void) | null = null
    if (this.flyAnim !== null) {
      /*
       * 环绕飞行：方向 / up / target 一起插值。
       * 透视档不是等到最后才切：在中段相机还远、模型半透明时切，
       * 并把 FOV 反算成同一画幅，之后边飞边把 FOV 拉回原来的值。
       */
      const a = this.flyAnim
      a.t = Math.min(1, a.t + dt / 0.85)
      const k = 1 - (1 - a.t) ** 3
      /* 切透视点的**线性时间**：k 是缓出值，不能直接拿来排阶段 */
      const switchT = a.switchAt >= 0 ? 1 - Math.cbrt(1 - a.switchAt) : 0
      /*
       * 缩放与移动用两条**错峰曲线**：
       * 视距先快后慢（`easeOutQuad`），方向与目标点先慢后快、末尾收住（`easeLate`）。
       * 它们从切透视后几乎同时起步，速度峰不在一个时刻，所以看着是一条连贯的运镜，
       * 而不是「先把距离拉完、再开始转」那种两段式。
       */
      const kDist =
        a.switchAt >= 0
          ? easeOutQuad((a.t - switchT) / (FLY_DIST_END - switchT))
          : k
      const kMove = a.switchAt >= 0 ? easeLate((a.t - FLY_MOVE_START) / (1 - FLY_MOVE_START)) : k

      const dir = a.from.clone()
      if (a.from.dot(a.to) < -0.999) dir.x += 1e-3
      const q = new THREE.Quaternion().setFromUnitVectors(dir.normalize(), a.to)
      const cur = a.from.clone().applyQuaternion(IDENTITY.clone().slerp(q, kMove)).normalize()
      this.controls.target.lerpVectors(a.targetFrom, a.targetTo, kMove)

      /* 切透视之前位姿与画幅都不动：切的那帧仍然严丝合缝 */
      const hold = a.switchAt >= 0 && !a.switched
      const dist = hold ? a.distFrom : a.distFrom + (a.distTo - a.distFrom) * kDist

      if (a.switchAt >= 0 && !a.switched && k >= a.switchAt) {
        if (a.switchTo === 'ortho') this.enterOrtho(a.halfFrom)
        else this.enterPerspective(a.halfFrom, dist)
        a.switched = true
      }

      const cam = this.cam
      cam.position.copy(this.controls.target).addScaledVector(cur, dist)
      cam.up.copy(a.upFrom).lerp(a.upTo, kMove).normalize()
      cam.lookAt(this.controls.target)
      if (this.ortho) {
        this.camO.zoom = a.zoomFrom + (a.zoomTo - a.zoomFrom) * kDist
        this.camO.updateProjectionMatrix()
      } else {
        /* 保持画幅连续：half/dist 决定 FOV，终点正好回到 home.fov */
        const half = a.halfFrom + (a.halfTo - a.halfFrom) * kDist
        const fov = (2 * Math.atan(half / Math.max(1e-9, dist)) * 180) / Math.PI
        this.camP.fov = Math.min(120, Math.max(1, fov))
        this.camP.updateProjectionMatrix()
      }
      if (a.fade !== 'none') this.setFade(a.fade === 'out' ? 1 - k : k)
      if (a.t >= 1) {
        if (a.switchAt >= 0 && !a.switched) {
          if (a.switchTo === 'ortho') this.enterOrtho(a.halfTo)
          else this.enterPerspective(a.halfTo, a.distTo)
          a.switched = true
        }
        if (this.switchingToPerspective(a)) {
          const finalFov = (2 * Math.atan(a.halfTo / Math.max(1e-9, a.distTo)) * 180) / Math.PI
          this.camP.fov = finalFov
          this.camP.zoom = 1
          this.camP.updateProjectionMatrix()
        }
        if (this.switchingToOrtho(a)) {
          this.camO.zoom = ORTHO_BASE / Math.max(1e-9, a.halfTo)
          this.camO.updateProjectionMatrix()
        }
        if (a.fade === 'in') {
          this.pseudo = false
          this.syncVisibility()
        }
        this.flyAnim = null
        this.controls.enabled = true
        this.last = 0
        flyDone = a.done
      }
    } else if (this.explodeAnim !== null) {
      /* 爆炸 = 把 Z 倍数拉大，镜头每帧跟着重新装框；顶栏的滑杆由回调同步 */
      const a = this.explodeAnim
      const dir = this.cam.position.clone().sub(this.controls.target)
      const len = dir.length()
      if (len > 1e-9) dir.divideScalar(len)
      a.t = Math.min(1, a.t + dt / 1.4)
      const k = 1 - (1 - a.t) ** 3
      this.exaggeration = a.from + (a.to - a.from) * k
      this.group.scale.set(1, 1, this.exaggeration)
      this.onExaggeration?.(this.exaggeration)
      const box = this.visibleBox()
      if (!box.isEmpty() && len > 1e-9) {
        const center = box.getCenter(new THREE.Vector3())
        const radius = Math.max(1, box.getSize(new THREE.Vector3()).length() / 2)
        this.controls.target.copy(center)
        this.cam.position.copy(center).addScaledVector(dir, this.fitDistance(radius))
      }
      if (a.t >= 1) {
        this.explodeAnim = null
        this.controls.enabled = true
        this.last = 0
        this.controls.update()
      }
    } else if (this.tween === null) {
      this.controls.update()
    } else {
      /* 补间期间不调 controls.update()：它会按自己记的球坐标把相机位置盖回去 */
      const tw = this.tween
      tw.t = Math.min(1, tw.t + dt / SNAP_SECONDS)
      const k = 1 - (1 - tw.t) ** 3
      const q = IDENTITY.clone().slerp(tw.q, k)
      this.cam.position.copy(this.controls.target).add(tw.from.clone().applyQuaternion(q))
      this.cam.lookAt(this.controls.target)
      if (tw.t >= 1) {
        this.tween = null
        this.controls.enabled = true
        this.last = 0
        this.controls.update()
      }
    }

    this.syncNear()
    this.renderer.render(this.scene, this.cam)
    /* 先画完 3D 末帧再切画布：2D 首帧必须接在它后面，不能先隐藏 3D */
    if (flyDone !== null) flyDone()
    this.onFrame?.(this.distance())
  }

  private start(): void {
    if (this.raf !== 0) return
    this.raf = requestAnimationFrame(this.tick)
  }

  private stop(): void {
    if (this.raf === 0) return
    cancelAnimationFrame(this.raf)
    this.raf = 0
  }
}

/** 涂胶块画在哪一层：优先用 `;MKP_BEGIN paint … z=`，老产物没有就退回笔尖 Z */
export function blockLayerZ(block: LabBlock): number {
  return block.layerZ ?? block.normalZ ?? 0
}
