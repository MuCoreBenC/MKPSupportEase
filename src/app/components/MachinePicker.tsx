import { useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { Badge } from '../ui/Controls'
import type { Option } from '../constants/machines'
import { brands } from '../constants/machines'
import { models } from '../constants/models'
import { variants } from '../constants/variants'
import s from './MachinePicker.module.css'

export interface Selection {
  brand: string | null
  model: string | null
  variant: string | null
}

interface MachinePickerProps {
  sel: Selection
  onPick: (level: keyof Selection, id: string) => void

  /** 卡片的眉标题。放进整块里一起渲染：它跟下面的组是一体的，
   *  与「品牌」的间距永远是这一块自己的 gap */
  head?: ReactNode
}

function Group({
  title,
  hint,
  items,
  value,
  on,
  onPick,
  collapsible = false,
}: {
  title: string
  /** 一句说明这一项是什么。省略即不渲染那一行 */
  hint?: string
  items: Option[]
  value: string | null
  /** 显不显形。不显形时这一组照样占位，位置不变 */
  on: boolean
  onPick: (id: string) => void
  /** 选项多的那几组：扁窗里默认只留已选的那一颗，其余折起来。
   *  展开与否是这一组自己的事，父级不需要知道 */
  collapsible?: boolean
}) {
  const [expanded, setExpanded] = useState(false)

  /* 没选之前折起来等于什么都看不到，所以「选过」是折叠的前提。
     取消选择（再点一次已选项）会让 value 变 null，这里自动回到全展开 */
  const collapsed = collapsible && !expanded && Boolean(value)

  return (
    <section className={s.group} data-on={on}>
      <header className={s.groupHead}>
        <h3 className={s.groupTitle}>{title}</h3>
        {hint && <span className={s.groupHint}>{hint}</span>}
      </header>
      <div className={s.options} data-collapsed={collapsed || undefined}>
        {items.map((o) => {
          const picked = o.id === value
          return (
            <button
              key={o.id}
              type="button"
              className={s.option}
              data-on={picked}
              aria-pressed={picked}
              title={picked ? '再点一次取消选择' : undefined}
              onClick={() => onPick(o.id)}
            >
              <span className={s.optionLabel}>{o.label}</span>
              {o.badge && <Badge tone={o.badgeTone ?? 'muted'}>{o.badge}</Badge>}
            </button>
          )
        })}

        {/* 展开指示器：宽窗里 CSS 把它收掉（display: none），所以那边点不到、也不占位 */}
        {collapsible && Boolean(value) && (
          <button
            type="button"
            className={s.toggle}
            data-open={expanded}
            aria-expanded={expanded}
            onClick={() => setExpanded((v) => !v)}
          >
            {expanded ? '收起' : `其余 ${items.length - 1} 项`}
          </button>
        )}
      </div>
    </section>
  )
}

/** 渐进披露的淡入时长。试验场里这个数来自调参面板，默认 300ms；产品里取默认值定为常量 */
const FADE_MS = 300

/** 品牌 → 机型 → 版本，竖向渐进披露：三行位置钉死，上游没选的那一行只是不显形 */
export default function MachinePicker({ sel, onPick, head }: MachinePickerProps) {
  // 可见性是纯派生值：三行都常驻在 DOM 里、位置由布局定死，切的只有 data-on。
  // 所以不需要任何 state / ref 去记「该给哪一组播动画」—— 过渡由 CSS transition 接管，
  // 外部（HeroFade 换图）引发的额外重渲染算出同一个 data-on，什么都不会发生。
  const vars = { '--fade-ms': `${FADE_MS}ms` } as CSSProperties

  return (
    <div className={s.stack} style={vars}>
      {head}

      {/* 品牌这一组不写说明。原来是「目前仅拓竹」—— 那是开发状态的说法，
          而且下面的选项本来就写着品牌名，再补一句限定词只是在强调「还没做完」 */}
      <Group
        title="品牌"
        items={brands}
        value={sel.brand}
        on
        onPick={(id) => onPick('brand', id)}
      />

      <Group
        title="机型"
        hint="决定校准板尺寸"
        items={models}
        value={sel.model}
        on={Boolean(sel.brand)}
        collapsible
        onPick={(id) => onPick('model', id)}
      />

      <Group
        title="打印件版本"
        hint="对应不同预设文件"
        items={variants}
        value={sel.variant}
        on={Boolean(sel.model)}
        collapsible
        onPick={(id) => onPick('variant', id)}
      />
    </div>
  )
}

