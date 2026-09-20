import { useEffect, useRef, useState } from 'react'
import * as THREE from 'three'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import s from './ExplodedHero.module.css'

/* ── deterministic pseudo-random (no Math.random) ─────────────── */
function hash(seed: string, i: number): number {
  let h = 0x811c9dc5
  const str = `${seed}_${i}`
  for (let k = 0; k < str.length; k++) {
    h ^= str.charCodeAt(k)
    h = Math.imul(h, 0x01000193)
  }
  return (h >>> 0) / 0xffffffff
}

/* ── config ────────────────────────────────────────────────────── */
const HERO_COLOR = 0x17924a
const BG_COLOR = 0xcfd3d6
const HERO_SCALE = 0.014      // mm → world units, hero slightly bigger
const BG_SCALE = 0.01         // mm → world units
const PIECE_COUNTS: Record<string, number> = { fishtail: 5, sphere: 5, plate: 4, block: 4 }
const SAFE = 0.04             // NDC margin
const GAP = 0.018             // separation gap in NDC
const COLS = 6
const ROWS = 4

export default function ExplodedHero() {
  const stageRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [glFailed, setGlFailed] = useState(false)

  useEffect(() => {
    const stage = stageRef.current
    const canvas = canvasRef.current
    if (!stage || !canvas) return

    /* ── renderer ──────────────────────────────────────────────── */
    let renderer: THREE.WebGLRenderer
    try {
      renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true })
    } catch {
      setGlFailed(true)
      return
    }
    if (!renderer.getContext()) { setGlFailed(true); return }

    renderer.setClearColor(0x000000, 0)
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2))
    renderer.outputColorSpace = THREE.SRGBColorSpace
    renderer.toneMapping = THREE.ACESFilmicToneMapping
    renderer.toneMappingExposure = 1.05

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 500)
    camera.position.set(0, 0.2, 5)
    camera.lookAt(0, 0, 0)

    /* ── lights ────────────────────────────────────────────────── */
    const key = new THREE.DirectionalLight(0xfff8f0, 2.2)
    key.position.set(3, 4, 5)
    scene.add(key)

    const fill = new THREE.DirectionalLight(0xd8e8f8, 0.8)
    fill.position.set(-4, -2, 3)
    scene.add(fill)

    const hemi = new THREE.HemisphereLight(0xe8ecf0, 0xf0f0f0, 0.6)
    scene.add(hemi)

    /* ── materials ─────────────────────────────────────────────── */
    const heroMat = new THREE.MeshStandardMaterial({
      color: HERO_COLOR,
      roughness: 0.72,
      metalness: 0,
    })
    const bgMat = new THREE.MeshStandardMaterial({
      color: BG_COLOR,
      roughness: 0.74,
      metalness: 0,
    })

    /* ── state for animation ───────────────────────────────────── */
    type Piece = { mesh: THREE.Object3D; baseY: number; phase: number; spin: number }
    const pieces: Piece[] = []
    let rafId = 0
    let visible = true
    const prefersReduced = matchMedia('(prefers-reduced-motion: reduce)').matches

    /* ── load models ───────────────────────────────────────────── */
    const loader = new GLTFLoader()
    loader.load(
      '/models/test_models.glb',
      (gltf) => {
        const geos: Record<string, THREE.BufferGeometry> = {}
        gltf.scene.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) {
            const m = child as THREE.Mesh
            geos[child.name] = m.geometry
          }
        })

        if (!geos.fishtail) { console.warn('ExplodedHero: fishtail geometry missing'); return }

        /* hero */
        const heroGeo = geos.fishtail
        const hero = new THREE.Mesh(heroGeo, heroMat)
        hero.scale.setScalar(HERO_SCALE)
        hero.rotation.set(0.25, -0.12, 1.12)  // similar to Blender pose
        scene.add(hero)
        pieces.push({ mesh: hero, baseY: 0, phase: 0, spin: 0.3 })

        /* background pieces */
        let idx = 0
        for (const [name, count] of Object.entries(PIECE_COUNTS)) {
          const geo = geos[name]
          if (!geo) continue
          const n = name === 'fishtail' ? count - 1 : count  // hero already placed
          for (let i = 0; i < n; i++) {
            const mesh = new THREE.Mesh(geo, bgMat)
            mesh.scale.setScalar(BG_SCALE)
            mesh.rotation.set(
              hash('rx', idx) * Math.PI * 2,
              hash('ry', idx) * Math.PI * 2,
              hash('rz', idx) * Math.PI * 2,
            )
            scene.add(mesh)
            pieces.push({
              mesh,
              baseY: 0,
              phase: hash('ph', idx) * Math.PI * 2,
              spin: (hash('sp', idx) - 0.5) * 0.6,
            })
            idx++
          }
        }

        /* ── layout: screen-space relaxation ───────────────────── */
        layoutPieces(camera, pieces, stage.clientWidth / stage.clientHeight)

        /* start render loop */
        if (!prefersReduced) startLoop()
        else { renderer.render(scene, camera) }
      },
      undefined,
      () => { setGlFailed(true) },
    )

    /* ── layout algorithm ──────────────────────────────────────── */
    function layoutPieces(
      cam: THREE.PerspectiveCamera,
      pcs: Piece[],
      aspect: number,
    ) {
      if (pcs.length < 2) return
      cam.aspect = aspect
      cam.updateProjectionMatrix()

      // hero stays at origin
      pcs[0].mesh.position.set(0, 0, 0)
      pcs[0].baseY = 0

      // compute screen-space bbox for a piece
      const box = new THREE.Box3()
      const v = new THREE.Vector3()
      function ndcRect(mesh: THREE.Object3D): [number, number, number, number] {
        box.setFromObject(mesh)
        const pts = [
          new THREE.Vector3(box.min.x, box.min.y, box.min.z),
          new THREE.Vector3(box.max.x, box.min.y, box.min.z),
          new THREE.Vector3(box.min.x, box.max.y, box.min.z),
          new THREE.Vector3(box.max.x, box.max.y, box.min.z),
          new THREE.Vector3(box.min.x, box.min.y, box.max.z),
          new THREE.Vector3(box.max.x, box.min.y, box.max.z),
          new THREE.Vector3(box.min.x, box.max.y, box.max.z),
          new THREE.Vector3(box.max.x, box.max.y, box.max.z),
        ]
        let xMin = Infinity, xMax = -Infinity, yMin = Infinity, yMax = -Infinity
        for (const p of pts) {
          v.copy(p).project(cam)
          const nx = (v.x + 1) / 2
          const ny = (v.y + 1) / 2
          if (nx < xMin) xMin = nx
          if (nx > xMax) xMax = nx
          if (ny < yMin) yMin = ny
          if (ny > yMax) yMax = ny
        }
        return [xMin, xMax, yMin, yMax]
      }

      // seed bg pieces onto jittered grid
      type Slot = { u: number; v: number; hw: number; hh: number }
      const slots: Slot[] = []

      // slot 0 = hero (fixed at center)
      const [hx0, hx1, hy0, hy1] = ndcRect(pcs[0].mesh)
      slots.push({ u: (hx0 + hx1) / 2, v: (hy0 + hy1) / 2, hw: (hx1 - hx0) / 2, hh: (hy1 - hy0) / 2 })

      const cells: [number, number][] = []
      for (let r = 0; r < ROWS; r++)
        for (let c = 0; c < COLS; c++)
          cells.push([c, r])

      for (let i = 1; i < pcs.length; i++) {
        const ci = (i - 1) % cells.length
        const [c, r] = cells[ci]
        const u = SAFE + (c + 0.5 + (hash('lu', i) - 0.5) * 0.4) * (1 - 2 * SAFE) / COLS
        const vv = SAFE + (r + 0.5 + (hash('lv', i) - 0.5) * 0.4) * (1 - 2 * SAFE) / ROWS

        // convert NDC to world position at z=0
        const wx = (u * 2 - 1) * cam.aspect * Math.tan(cam.fov * Math.PI / 360) * cam.position.z
        const wy = (vv * 2 - 1) * Math.tan(cam.fov * Math.PI / 360) * cam.position.z
        pcs[i].mesh.position.set(wx, wy, 0)
        pcs[i].baseY = wy

        const [x0, x1, y0, y1] = ndcRect(pcs[i].mesh)
        slots.push({ u: (x0 + x1) / 2, v: (y0 + y1) / 2, hw: (x1 - x0) / 2, hh: (y1 - y0) / 2 })
      }

      // relax: separate overlapping screen rects
      for (let iter = 0; iter < 400; iter++) {
        let moved = 0
        for (let i = 0; i < slots.length; i++) {
          for (let j = i + 1; j < slots.length; j++) {
            const a = slots[i], b = slots[j]
            const dx = b.u - a.u, dy = b.v - a.v
            const ox = (a.hw + b.hw + GAP) - Math.abs(dx)
            const oy = (a.hh + b.hh + GAP) - Math.abs(dy)
            if (ox > 0 && oy > 0) {
              if (i === 0) {
                // hero is fixed, push only j
                if (ox / (a.hw + b.hw + GAP) < oy / (a.hh + b.hh + GAP)) {
                  b.u += ox * (dx >= 0 ? 1 : -1)
                } else {
                  b.v += oy * (dy >= 0 ? 1 : -1)
                }
              } else if (ox / (a.hw + b.hw + GAP) < oy / (a.hh + b.hh + GAP)) {
                const sh = ox / 2 * (dx >= 0 ? 1 : -1)
                a.u -= sh; b.u += sh
              } else {
                const sv = oy / 2 * (dy >= 0 ? 1 : -1)
                a.v -= sv; b.v += sv
              }
              moved += ox + oy
            }
          }
        }
        // clamp inside safe frame (skip hero)
        for (let i = 1; i < slots.length; i++) {
          const sl = slots[i]
          sl.u = Math.max(SAFE + sl.hw, Math.min(1 - SAFE - sl.hw, sl.u))
          sl.v = Math.max(SAFE + sl.hh, Math.min(1 - SAFE - sl.hh, sl.v))
        }
        if (moved < 1e-4) break
      }

      // commit positions
      for (let i = 1; i < pcs.length; i++) {
        const sl = slots[i]
        const wx = (sl.u * 2 - 1) * cam.aspect * Math.tan(cam.fov * Math.PI / 360) * cam.position.z
        const wy = (sl.v * 2 - 1) * Math.tan(cam.fov * Math.PI / 360) * cam.position.z
        pcs[i].mesh.position.set(wx, wy, 0)
        pcs[i].baseY = wy
      }
    }

    /* ── animation loop ────────────────────────────────────────── */
    let lastT = 0
    function animate(t: number) {
      rafId = requestAnimationFrame(animate)
      const dt = Math.min((t - lastT) / 1000, 0.05)
      lastT = t

      for (const p of pieces) {
        p.mesh.rotation.y += dt * 0.04 * p.spin
        p.mesh.position.y = p.baseY + Math.sin(t * 0.0003 + p.phase) * 0.06
      }
      renderer.render(scene, camera)
    }

    function startLoop() {
      if (rafId) return
      lastT = performance.now()
      rafId = requestAnimationFrame(animate)
    }
    function stopLoop() {
      if (rafId) { cancelAnimationFrame(rafId); rafId = 0 }
    }

    /* ── resize ────────────────────────────────────────────────── */
    // 箭头函数而不是 function 声明：函数声明会被提升，TS 因此不保留上面
    // `if (!stage) return` 对 stage 的收窄，闭包里 stage 又变回可空
    const resize = () => {
      const w = stage.clientWidth, h = stage.clientHeight
      if (w === 0 || h === 0) return
      camera.aspect = w / h
      camera.updateProjectionMatrix()
      renderer.setSize(w, h, false)
      if (pieces.length > 1) layoutPieces(camera, pieces, w / h)
      renderer.render(scene, camera)
    }
    const ro = new ResizeObserver(resize)
    ro.observe(stage)
    resize()

    /* ── visibility ────────────────────────────────────────────── */
    const io = new IntersectionObserver(([e]) => {
      visible = e.isIntersecting
      if (visible && !prefersReduced) startLoop()
      else stopLoop()
    }, { threshold: 0.05 })
    io.observe(stage)

    function onVis() {
      if (document.hidden) stopLoop()
      else if (visible && !prefersReduced) startLoop()
    }
    document.addEventListener('visibilitychange', onVis)

    /* ── cleanup ───────────────────────────────────────────────── */
    return () => {
      stopLoop()
      ro.disconnect()
      io.disconnect()
      document.removeEventListener('visibilitychange', onVis)
      heroMat.dispose()
      bgMat.dispose()
      scene.traverse((child) => {
        if ((child as THREE.Mesh).isMesh) {
          (child as THREE.Mesh).geometry.dispose()
        }
      })
      renderer.dispose()
    }
  }, [])

  if (glFailed) {
    return (
      <img
        className={s.fallback}
        src="/models/hero_fishtail.webp"
        srcSet="/models/hero_fishtail.webp 1x, /models/hero_fishtail@2x.webp 2x"
        alt="校准测试模型"
      />
    )
  }

  return (
    <div ref={stageRef} className={s.stage}>
      <canvas ref={canvasRef} />
    </div>
  )
}
