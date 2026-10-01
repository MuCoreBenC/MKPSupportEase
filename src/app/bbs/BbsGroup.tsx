/*
 * 一个参数组：组图标 + 组名 + 若干行。
 * 组标题左边那个小图标的 svg 名写在 Tab.cpp 的 new_optgroup 第二个参数里，图在 icons.json。
 */

import BbsIcon from './BbsIcon'
import BbsRow from './BbsRow'
import type { PanelGroup } from './bbsPanel'
import type { BbsValues } from './bbsTypes'
import s from './BbsGroup.module.css'

interface Props {
  group: PanelGroup
  values: BbsValues
  baseline: BbsValues
  modifiedKeys: Set<string>
  variantIdx: number
  icons: Record<string, string>
  readOnly: boolean
  /** 归一化后的搜索词，透传给行（标签里切命中那一段用） */
  query: string
  onChange: (key: string, value: string) => void
  onReset: (key: string) => void
}

export default function BbsGroup({
  group,
  values,
  baseline,
  modifiedKeys,
  variantIdx,
  icons,
  readOnly,
  query,
  onChange,
  onReset,
}: Props) {
  return (
    <section className={s.group}>
      <h3 className={s.head}>
        <BbsIcon name={group.icon} icons={icons} className={s.gico} />
        {group.name}
      </h3>
      {group.rows.map((row) => (
        <BbsRow
          key={row.key}
          paramKey={row.key}
          meta={row.meta}
          value={values[row.key]}
          values={values}
          baseline={baseline}
          variantIdx={variantIdx}
          icons={icons}
          modified={modifiedKeys.has(row.key)}
          greyed={row.greyed}
          readOnly={readOnly}
          hit={row.hit}
          query={query}
          onChange={onChange}
          onReset={onReset}
        />
      ))}
    </section>
  )
}
