/*
 * 3D 层堆叠：把每一块涂胶按它自己那一档 Z 摆起来，Z 方向可放大。
 *
 * 挣位置的只有一件事 —— **层间漂移**：同一条边这层抬了、下层没抬，2D 里看不出来，
 * 堆起来就是一道豁口。别处（判定为什么错）仍然看 2D。
 *
 * 坐标用 Z 朝上（相机 `up` 设成 +Z），与 G-code 一致，免得读图时还要在脑子里翻。
 *
 * # 相机手感
 *
 * 照 machine-motion 那一套（那边调过好几轮，别再自己发明一遍）：
 *
 * - **旋转中心钉在模型上**：`zoomToCursor` 关掉 —— 开着的话滚几次中心就跑到模型旁边的
 *   空地上，转起来像绕着屏幕中心甩；
 * - **平移会把中心一起带走**：右键拖拽走 OrbitControls 自带的 pan（改 `target`），
 *   所以拖完再转，转的是新位置上的那个点；
 * - **Shift = 精调**：三个速度一起缩到 1/6，同时把左右键换手（左键平移、右键旋转）；
 * - **近裁面跟着视距走**（视距的 2%）：可以一路推进去看某一层的豁口，不会在 1mm 处被裁掉。
 *
 * # 重画
 *
 * 拖拽/缩放的中间帧由 `requestAnimationFrame` 驱动（开阻尼就必须每帧 `controls.update()`，
 * 只在事件里画一次会「拖了没反应」）。颜色全部取自 `theme.ts`，系统换深浅色时整层重建。
 */

import * as THREE from 'three'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'

import type { LabBlock } from '../../src/app/glueLab/parse'
import { hex, palette } from './theme'

/** Shift 档的速度倍率：与 machine-motion 同一档（滚轮一格约 0.9% 视距） */
const NAV_FINE = 1 / 6

export class View3d {
  private readonly canvas: HTMLCanvasElement
  private readonly renderer: THREE.WebGLRenderer
  private readonly scene = new THREE.Scene()
  private readonly camera: THREE.PerspectiveCamera
  private readonly controls: OrbitControls
  private readonly group = new THREE.Group()
  private grid: THREE.GridHelper
  private allLayer: THREE.LineSegments | null = null
  private curLayer: THREE.LineSegments | null = null
  private blocks: LabBlock[] = []
  private index = 0
  private baseZ = 0
  private exaggeration = 20
  private visible = false
  private framed = false
  private raf = 0
  private readonly baseSpeed = { rotate: 1, pan: 1, zoom: 1 }

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true })
    this.renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1))

    this.camera = new THREE.PerspectiveCamera(42, 1, 0.1, 5000)
    this.camera.up.set(0, 0, 1)
    this.camera.position.set(180, -180, 160)

    this.controls = new OrbitControls(this.camera, canvas)
    this.controls.enableDamping = true
    this.controls.dampingFactor = 0.1
    this.controls.zoomToCursor = false
    this.controls.screenSpacePanning = true
    this.controls.minDistance = 0.05
    this.controls.maxDistance = 4000
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

  setBlocks(blocks: LabBlock[], index: number): void {
    this.blocks = blocks
    const zs = blocks
      .map((b) => b.normalZ)
      .filter((z): z is number => z !== null)
      .sort((a, b) => a - b)
    this.baseZ = zs.length > 0 ? (zs[0] as number) : 0
    this.rebuildAll()
    this.select(index)
    this.framed = false
    if (this.visible) this.frame()
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
    const layer = this.build([block], true, 1)
    this.curLayer = layer
    this.group.add(layer)
    this.group.scale.set(1, 1, this.exaggeration)
  }

  setExaggeration(n: number): void {
    this.exaggeration = n
    this.group.scale.set(1, 1, n)
  }

  /** 系统换深浅色：网格与所有线段的颜色都要重建（顶点色烘在 geometry 里） */
  refreshTheme(): void {
    this.scene.remove(this.grid)
    this.grid.geometry.dispose()
    ;(this.grid.material as THREE.Material).dispose()
    this.grid = this.buildGrid()
    this.scene.add(this.grid)
    this.rebuildAll()
    this.select(this.index)
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
    this.camera.aspect = w / h
    this.camera.updateProjectionMatrix()
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
       * 所以这里要先写成「对调之后正好」的那一套 —— 用的是公开的 `mouseButtons` 配置，
       * 没有去改库里的东西。
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
    return grid
  }

  private build(
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
      for (const m of block.moves) {
        if (m.level === 'up') continue
        const z = Math.max(0, m.z - this.baseZ)
        positions.push(m.from.x, m.from.y, z, m.to.x, m.to.y, z)
        const c = colored ? (m.level === 'raised' ? raised : normal) : dim
        colors.push(c.r, c.g, c.b, c.r, c.g, c.b)
      }
    }
    const geo = new THREE.BufferGeometry()
    geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3))
    geo.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3))
    const mat = new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity })
    return new THREE.LineSegments(geo, mat)
  }

  private rebuildAll(): void {
    if (this.allLayer) {
      this.group.remove(this.allLayer)
      this.allLayer.geometry.dispose()
      ;(this.allLayer.material as THREE.Material).dispose()
      this.allLayer = null
    }
    this.allLayer = this.build(this.blocks, false, 0.4)
    this.group.add(this.allLayer)
  }

  private frame(): void {
    const box = new THREE.Box3().setFromObject(this.group)
    if (box.isEmpty()) return
    const center = box.getCenter(new THREE.Vector3())
    const radius = Math.max(1, box.getSize(new THREE.Vector3()).length() / 2)
    const fov = (this.camera.fov * Math.PI) / 180
    const dist = (radius / Math.sin(fov / 2)) * 1.05
    const dir = new THREE.Vector3(0.85, -1, 0.75).normalize()
    this.controls.target.copy(center)
    this.camera.position.copy(center).addScaledVector(dir, dist)
    this.controls.update()
    this.framed = true
  }

  /** 近裁面 = 视距的 2%（下界 0.02mm）：远景保住深度精度，贴到毫米级也不裁 */
  private syncNear(): void {
    const d = this.camera.position.distanceTo(this.controls.target)
    const near = Math.max(0.02, d * 0.02)
    const far = Math.max(1000, d * 20)
    if (
      Math.abs(this.camera.near - near) > near * 0.05 ||
      Math.abs(this.camera.far - far) > far * 0.05
    ) {
      this.camera.near = near
      this.camera.far = far
      this.camera.updateProjectionMatrix()
    }
  }

  private readonly tick = (): void => {
    this.raf = requestAnimationFrame(this.tick)
    this.syncNear()
    this.controls.update()
    this.renderer.render(this.scene, this.camera)
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
