import { useState, type CSSProperties } from 'react'
import * as precise from '../../calib/precise-calibration.generated'
import * as zoffset from '../../calib/zoffset-calibration.generated'
import s from './CalibPlate.module.css'

const PLATES = { zoffset, precise }

export type PlateModel = keyof typeof PLATES

interface CalibPlateProps {
  model: PlateModel
  /** 配色套名，省略则用工作台里存下的那一套 */
  theme?: string
  label: string
  /** 热区被点（含键盘 Enter / Space）；不传就是只读 */
  onPick?: (id: string) => void
  /** 当前选中的热区，多选就传多个 id */
  picked?: readonly string[]
  /** 给热区的可读名字，用于 aria-label */
  hitLabel?: (id: string) => string
}

export default function CalibPlate({
  model,
  theme,
  label,
  onPick,
  picked,
  hitLabel,
}: CalibPlateProps) {
  const plate = PLATES[model]
  const vars = plate.THEMES[theme ?? plate.ACTIVE_THEME] ?? plate.THEME
  const [x, y, w, h] = plate.VIEW_BOX
  const [hover, setHover] = useState<string | null>(null)
  const on = (id: string) => Boolean(picked?.includes(id))

  // 命中层：产物里的热区 bbox 四边各外扩 HIT_GROW —— 那些梯子只有 0.5mm 宽，
  // 靠轮廓本身根本点不中，导出的 SVG 也是另铺一层 .hit 矩形来解决的。
  const useRects = Boolean(onPick) && plate.HIT_SHAPE === 'rect'
  const grow = plate.HIT_GROW

  return (
    <div
      className={s.wrap}
      data-model={plate.MODEL}
      // --plate-mm 是这块板的 mm 宽度：乘上列上给的共用 px/mm，两块板才一样粗
      style={{ ...vars, '--plate-mm': w } as CSSProperties}
    >
      <svg
        className={s.plateSvg}
        viewBox={`${x} ${y} ${w} ${h}`}
        role={onPick ? 'group' : 'img'}
        aria-label={label}
        data-model={plate.MODEL}
        data-unit={plate.UNIT}
      >
        {plate.PATHS.map((path) => {
          // 用命中层时可见层一律不吃指针；退回老路时才让 path 自己可点
          const clickable = Boolean(onPick) && path.hit && !useRects
          return (
            <path
              key={path.id}
              className={s[path.role]}
              d={path.d}
              fillRule="evenodd"
              data-id={path.id}
              data-hit={path.hit}
              data-value={path.value}
              data-on={Boolean(onPick) && path.hit ? on(path.id) : undefined}
              data-hover={useRects && path.hit ? hover === path.id : undefined}
              role={clickable ? 'button' : undefined}
              tabIndex={clickable ? 0 : undefined}
              aria-pressed={clickable ? on(path.id) : undefined}
              aria-label={clickable ? (hitLabel?.(path.id) ?? path.label ?? path.id) : undefined}
              onClick={clickable ? () => onPick?.(path.id) : undefined}
              onKeyDown={
                clickable
                  ? (e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        onPick?.(path.id)
                      }
                    }
                  : undefined
              }
            />
          )
        })}
        {plate.TEXTS.map((t) => (
          <text
            key={t.id}
            className={s.glyph}
            x={t.x}
            y={t.y}
            fontFamily={plate.FONT_FAMILY}
            fontWeight={plate.FONT_WEIGHT}
            fontSize={t.fontSize}
            textAnchor={t.anchor}
            textLength={t.textLength}
            transform={t.rotate ? `rotate(${t.rotate} ${t.x} ${t.y})` : undefined}
            data-id={t.id}
          >
            {t.text}
          </text>
        ))}
        {useRects &&
          plate.HOTSPOTS.map((spot) => (
            <rect
              key={spot.id}
              className={s.hit}
              x={spot.bbox[0] - grow}
              y={spot.bbox[1] - grow}
              width={spot.bbox[2] - spot.bbox[0] + grow * 2}
              height={spot.bbox[3] - spot.bbox[1] + grow * 2}
              data-id={spot.id}
              data-hit="true"
              role="button"
              tabIndex={0}
              aria-pressed={on(spot.id)}
              aria-label={hitLabel?.(spot.id) ?? spot.id}
              onClick={() => onPick?.(spot.id)}
              // 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页跟着挪
              onMouseDown={(e) => e.preventDefault()}
              onMouseEnter={() => setHover(spot.id)}
              onMouseLeave={() => setHover((prev) => (prev === spot.id ? null : prev))}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  onPick?.(spot.id)
                }
              }}
            />
          ))}
      </svg>
    </div>
  )
}
