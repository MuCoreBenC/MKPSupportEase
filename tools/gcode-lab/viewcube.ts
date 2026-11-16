/*
 * 视角立方体：六个面 + 八个角都能点，点哪就转到哪一面 / 哪一角。
 *
 * 为什么要单开一张 canvas：拾取要独占指针。画在主画布上就得和 OrbitControls 抢
 * 事件（它是按整张画布算的），一个 `pointerdown` 会被两边各处理一次。
 *
 * 朝向来源：主相机四元数的逆。立方体跟着它转，所以"顶面"永远是世界的 +Z ——
 * 于是拾取拿到的**局部法线就是世界方向**，不用再做一次坐标换算。
 *
 * 面贴图是现画的（canvas 2D）：标签、边框、3×3 的浅格 —— 那三格正好标出八个角的位置，
 * 点角的时候有个视觉落点。
 */

import * as THREE from 'three'

import { hex, palette } from './theme'

/** 六个面：局部法线 → 标签。顺序与 BoxGeometry 的材质数组一致 */
const FACES: { normal: THREE.Vector3; label: string }[] = [
  { normal: new THREE.Vector3(1, 0, 0), label: '右面' },
  { normal: new THREE.Vector3(-1, 0, 0), label: '左面' },
  { normal: new THREE.Vector3(0, 1, 0), label: '后面' },
  { normal: new THREE.Vector3(0, -1, 0), label: '前面' },
  { normal: new THREE.Vector3(0, 0, 1), label: '顶部' },
  { normal: new THREE.Vector3(0, 0, -1), label: '底部' },
]

const SIZE = 0.86
const CORNER_R = 0.09
/**
 * 正交相机的半高。比立方体本身大一圈（立方体的斜对角约 0.61 + 圆点半径），
 * 角上那颗提示点才不会被画布边缘切掉。
 */
const FRUSTUM = 0.92
/** 立方体相机的站位距离（正交，只影响近远裁面） */
const CAM_DIST = 4
export class ViewCube {
  private readonly canvas: HTMLCanvasElement
  private readonly renderer: THREE.WebGLRenderer
  private readonly scene = new THREE.Scene()
  private readonly camera: THREE.OrthographicCamera
  private readonly cube = new THREE.Group()
  /** 六个面（顺序与 `FACES` 一致，拾取按下标反查） */
  private readonly planes: THREE.Mesh[] = []
  private readonly corners: THREE.Mesh[] = []
  /** 重建时要一起释放的东西（棱线这类） */
  private readonly extras: THREE.Object3D[] = []
  /** 悬停高亮：六个面各自比本体大一点点的一块半透明面 */
  private readonly hoverPlanes: THREE.Mesh[] = []
  private readonly raycaster = new THREE.Raycaster()
  private readonly pointer = new THREE.Vector2()
  private hover: number | null = null
  private disposed = false

  /** 点到了：给一个世界方向（相机应该站在那边） */
  onPick: ((dir: THREE.Vector3) => void) | null = null

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true })
    this.renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1))

    this.camera = new THREE.OrthographicCamera(-FRUSTUM, FRUSTUM, FRUSTUM, -FRUSTUM, 0.1, 10)
    this.camera.up.set(0, 0, 1)
    this.camera.position.set(2.4, -2.4, 2)
    this.camera.lookAt(0, 0, 0)

    this.scene.add(this.cube)
    this.build()
    this.bind()
  }

  /**
   * 每帧喂一次主相机的四元数。
   *
   * 立方体**自己不动**（局部 = 世界坐标），动的是它那台正交相机：位置与朝向都抄主相机。
   * 于是「点前面」之后看到的就是正对着的前面（一个正方形），不是斜着的 3/4 ——
   * 这里不要为了「三个面都点得着」去加一个固定机位的偏角，那是揣测，
   * 作者要的就是前面 = 前面。
   */
  update(cameraQuaternion: THREE.Quaternion): void {
    if (this.disposed) return
    this.camera.quaternion.copy(cameraQuaternion)
    this.camera.position.set(0, 0, CAM_DIST).applyQuaternion(cameraQuaternion)
    this.renderer.render(this.scene, this.camera)
  }

  /** 系统换深浅色：面贴图重画 */
  refreshTheme(): void {
    this.build()
  }

  resize(): void {
    const rect = this.canvas.getBoundingClientRect()
    const w = Math.max(1, rect.width)
    const h = Math.max(1, rect.height)
    this.renderer.setSize(w, h, false)
    this.camera.left = -FRUSTUM * (w / h)
    this.camera.right = FRUSTUM * (w / h)
    this.camera.top = FRUSTUM
    this.camera.bottom = -FRUSTUM
    this.camera.updateProjectionMatrix()
  }

  dispose(): void {
    this.disposed = true
    this.renderer.dispose()
  }

  /* ---------------- 内部 ---------------- */

  private build(): void {
    const drop = (obj: THREE.Object3D): void => {
      this.cube.remove(obj)
      const any = obj as unknown as { geometry?: THREE.BufferGeometry; material?: THREE.Material }
      any.geometry?.dispose()
      any.material?.dispose()
    }
    for (const plane of this.planes) drop(plane)
    for (const corner of this.corners) drop(corner)
    for (const extra of this.extras) drop(extra)
    for (const hover of this.hoverPlanes) drop(hover)
    this.planes.length = 0
    this.hoverPlanes.length = 0
    this.corners.length = 0
    this.extras.length = 0

    /*
     * 六个面各用一块 Plane，不用 BoxGeometry：后者的 UV 朝向按面而变
     * （侧面贴图的"上"是 +Y，而这个世界是 Z 朝上），文字会横躺 90°。
     * 自己摆 Plane + 指定 `up`，贴图的"上"就永远是你想要的那个方向。
     */
    const half = SIZE / 2
    FACES.forEach((f) => {
      const plane = new THREE.Mesh(
        new THREE.PlaneGeometry(SIZE, SIZE),
        new THREE.MeshBasicMaterial({ map: faceTexture(f.label) }),
      )
      plane.position.copy(f.normal).multiplyScalar(half + 0.001)
      plane.quaternion.setFromRotationMatrix(basisOf(f.normal))
      this.cube.add(plane)
      this.planes.push(plane)

      const glow = new THREE.Mesh(
        new THREE.PlaneGeometry(SIZE, SIZE),
        new THREE.MeshBasicMaterial({
          color: hex(palette().teal, 0x2bb6a8),
          transparent: true,
          opacity: 0.3,
          depthWrite: false,
        }),
      )
      glow.position.copy(f.normal).multiplyScalar(half + 0.004)
      glow.quaternion.copy(plane.quaternion)
      glow.visible = false
      this.cube.add(glow)
      this.hoverPlanes.push(glow)
    })

    /* 棱：让立方体在深色里也有个清楚的轮廓 */
    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(new THREE.BoxGeometry(SIZE, SIZE, SIZE)),
      new THREE.LineBasicMaterial({ color: hex(palette().dim, 0x8b949e), transparent: true, opacity: 0.55 }),
    )
    this.cube.add(edges)
    this.extras.push(edges)

    /* 八个角：透明但可拾取的小球，稍微凸出一点点，点角比点面更"跟手" */
    for (const sx of [-1, 1]) {
      for (const sy of [-1, 1]) {
        for (const sz of [-1, 1]) {
          const dot = new THREE.Mesh(
            new THREE.SphereGeometry(CORNER_R, 12, 8),
            new THREE.MeshBasicMaterial({
              color: hex(palette().teal, 0x2bb6a8),
              transparent: true,
              opacity: 0,
              depthWrite: false,
              /* 球心就在顶点上，球体大半埋在立方体里 —— 关掉深度测试才看得见 */
              depthTest: false,
            }),
          )
          dot.position.set(sx * half, sy * half, sz * half)
          dot.renderOrder = 4
          dot.userData.dir = new THREE.Vector3(sx, sy, sz)
          this.cube.add(dot)
          this.corners.push(dot)
        }
      }
    }
  }

  private bind(): void {
    this.canvas.addEventListener('pointermove', (e) => {
      this.hover = this.pick(e)
      this.canvas.style.cursor = this.hover === null ? 'default' : 'pointer'
      this.paint()
    })
    this.canvas.addEventListener('pointerleave', () => {
      this.hover = null
      this.paint()
    })
    this.canvas.addEventListener('click', (e) => {
      const hit = this.pick(e)
      if (hit === null) return
      const dir = this.dirOf(hit)
      if (dir !== null) this.onPick?.(dir)
      /* 点完视角变了，之前那个悬停高亮就不作数了 */
      this.hover = null
      this.paint()
    })
  }

  /**
   * 命中：0…5 是面（`FACES` 下标），6…13 是角（corners 下标 + 6）。
   *
   * 角**按屏幕距离判**，不用射线打小球：球心就在顶点上，从正对着它那个方向看时，
   * 它比三个面都近 —— 于是立方体正中（三条棱交汇处）点一下会被判成角，
   * 而作者点的明明是「前面」。改成：光标离某个**朝向镜头的**顶点不到 11px 才算角，
   * 其余一律算面。同一个位置点两次，结果就一样了。
   */
  private pick(e: MouseEvent): number | null {
    const rect = this.canvas.getBoundingClientRect()
    const px = e.clientX - rect.left
    const py = e.clientY - rect.top
    this.pointer.set((px / rect.width) * 2 - 1, -(py / rect.height) * 2 + 1)

    const toCam = this.camera.position.clone().normalize()
    let best = -1
    let bestD = 9
    this.corners.forEach((corner, i) => {
      /*
       * 朝向镜头那一半的顶点都算（含**最近的那个**：斜看时它正好在立方体正中，
       * 作者就是要能从那儿点进去）。背面的顶点会投影到面里，必须挡掉。
       */
      if (corner.position.dot(toCam) <= 0.05) return
      const p = corner.position.clone().project(this.camera)
      const sx = (p.x * 0.5 + 0.5) * rect.width
      const sy = (-p.y * 0.5 + 0.5) * rect.height
      const d = Math.hypot(sx - px, sy - py)
      if (d < bestD) {
        bestD = d
        best = i
      }
    })
    if (best >= 0) return 6 + best

    this.raycaster.setFromCamera(this.pointer, this.camera)
    const hit = this.raycaster.intersectObjects(this.planes, false)[0]
    return hit ? this.planes.indexOf(hit.object as THREE.Mesh) : null
  }

  /** 命中项 → 世界方向：和切片器 BBS 的视角立方体同一套规则 */
  private dirOf(hit: number): THREE.Vector3 | null {
    const obj = hit < 6 ? this.planes[hit] : this.corners[hit - 6]
    if (!obj) return null
    const p = obj.position.clone()
    /*
     * 角 = 等轴测：站在那个角上，前面在左、右面在右、顶面在顶。
     * 面 = 正对：点「前面」就是正前方，点「右面」就是正右方，不倾斜。
     */
    if (hit >= 6) p.z *= 0.75
    return p.normalize()
  }

  /** 悬停着色：面提亮，角点亮 */
  private paint(): void {
    this.hoverPlanes.forEach((glow, i) => {
      glow.visible = this.hover === i
    })
    this.corners.forEach((corner, i) => {
      const mat = corner.material as THREE.MeshBasicMaterial
      mat.opacity = this.hover === i + 6 ? 0.95 : 0
    })
    this.renderer.render(this.scene, this.camera)
  }
}

/**
 * 一个面的姿态：局部 +Z = 法线（朝外），局部 +Y = 贴图的"上"。
 * 自己拼基向量而不用 `lookAt` —— `lookAt` 还要看父级世界矩阵，这里一眼能读清。
 */
function basisOf(normal: THREE.Vector3): THREE.Matrix4 {
  const z = normal.clone()
  const x = new THREE.Vector3().crossVectors(upHint(z), z).normalize()
  const y = new THREE.Vector3().crossVectors(z, x)
  return new THREE.Matrix4().makeBasis(x, y, z)
}

/**
 * 这一面摆好之后，贴图的"上"指向哪：侧面朝 +Z（世界的上），顶面朝 +Y、底面朝 -Y。
 *
 * 顶面用 +Y 而不是 -Y：立方体几乎总是从固定 3/4 机位看，顶面的远边在 +Y 那一侧 ——
 * 文字朝它才对得上；朝 -Y 会看着是倒的。
 */
function upHint(normal: THREE.Vector3): THREE.Vector3 {
  if (Math.abs(normal.z) > 0.5) return new THREE.Vector3(0, normal.z > 0 ? 1 : -1, 0)
  return new THREE.Vector3(0, 0, 1)
}

/**
 * 一张面贴图：浅底、细边、3×3 的浅格、正中一行字。
 *
 * 上/下两面（±Z）的文字要跟着转 90° —— 从上看下去，画面的"上"是 -Y，
 * 不转的话「顶部」会横躺在那里。
 */
function faceTexture(label: string): THREE.CanvasTexture {
  const p = palette()
  const canvas = document.createElement('canvas')
  canvas.width = 256
  canvas.height = 256
  const ctx = canvas.getContext('2d')
  if (ctx) {
    const bg = p.cube
    const text = p.text
    ctx.fillStyle = bg
    ctx.fillRect(0, 0, 256, 256)

    ctx.strokeStyle = p.dim
    ctx.globalAlpha = 0.7
    ctx.lineWidth = 5
    ctx.strokeRect(2.5, 2.5, 251, 251)
    ctx.globalAlpha = 1

    ctx.strokeStyle = p.dim
    ctx.globalAlpha = 0.22
    ctx.lineWidth = 2
    ctx.beginPath()
    for (const t of [85, 170]) {
      ctx.moveTo(t, 0)
      ctx.lineTo(t, 256)
      ctx.moveTo(0, t)
      ctx.lineTo(256, t)
    }
    ctx.stroke()
    ctx.globalAlpha = 1

    ctx.translate(128, 128)
    ctx.fillStyle = text
    ctx.font = '600 54px -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(label, 0, 0)
  }
  const tex = new THREE.CanvasTexture(canvas)
  tex.colorSpace = THREE.SRGBColorSpace
  tex.anisotropy = 2
  return tex
}
