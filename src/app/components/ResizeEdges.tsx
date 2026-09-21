import { winStartResize } from '../window'
import type { ResizeDirection } from '../window'
import s from './ResizeEdges.module.css'

/* 8 个方向与各自的 class。角在后面——DOM 顺序让角盖在边上面 */
const EDGES: { dir: ResizeDirection; cls: string }[] = [
  { dir: 'North', cls: s.n },
  { dir: 'South', cls: s.s },
  { dir: 'West', cls: s.w },
  { dir: 'East', cls: s.e },
  { dir: 'NorthWest', cls: s.nw },
  { dir: 'NorthEast', cls: s.ne },
  { dir: 'SouthWest', cls: s.sw },
  { dir: 'SouthEast', cls: s.se },
]

/**
 * 窗口内侧的一圈 resize 命中区。
 *
 * `decorations: false` 之后系统那条不可见 resize 边框在可见窗口**之外** 8px，
 * 必须把鼠标移出窗口才抓得到。前端改不了外圈（那要 Rust 侧接 `WM_NCHITTEST`），
 * 但在里侧补一圈之后，抓取带就跨在可见边界上——和系统其它软件的手感一致。
 */
export default function ResizeEdges() {
  return (
    <>
      {EDGES.map(({ dir, cls }) => (
        <div
          key={dir}
          className={`${s.edge} ${cls}`}
          onMouseDown={(e) => {
            if (e.button !== 0) return
            e.preventDefault()
            void winStartResize(dir)
          }}
        />
      ))}
    </>
  )
}
