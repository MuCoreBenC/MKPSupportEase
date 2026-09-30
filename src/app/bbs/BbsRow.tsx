/*
 * 一行参数：标签 + 重置 ↺ + 控件 + 单位 + 变体角标。
 *
 * 两处必须照上游的细节：
 *   - **未改动的行也要留一个等宽空位**（`.resetSlot`），否则改动行与未改动行的值列错开。
 *   - 重置箭头是 BBS 的 `undo.svg`（Tab.cpp:313），颜色 #F1754E **写死在 svg 里**，
 *     不吃 CSS 的 color —— hover 只能用 filter 提亮。
 *
 * 标签的 title 里带 key 与 mode：这一页是拿来对账的，看到一行想知道它在 json 里叫什么。
 */

import BbsControl from './BbsControl'
import BbsIcon from './BbsIcon'
import { norm } from './bbsMerge'
import { hitNote, sliceLabel } from './bbsSearch'
import type { BbsHit } from './bbsSearch'
import type { BbsParamMeta, BbsValue, BbsValues } from './bbsTypes'
import s from './BbsRow.module.css'

interface Props {
  paramKey: string
  meta: BbsParamMeta
  value: BbsValue | undefined
  values: BbsValues
  baseline: BbsValues
  variantIdx: number
  icons: Record<string, string>
  modified: boolean
  greyed: boolean
  readOnly: boolean
  /** 搜索命中在哪一栏；null = 没在搜或没命中 */
  hit: BbsHit | null
  /** 归一化后的搜索词（小写），只用来在标签里切出命中那一段 */
  query: string
  onChange: (key: string, value: string) => void
  onReset: (key: string) => void
}

export default function BbsRow({
  paramKey,
  meta,
  value,
  values,
  baseline,
  variantIdx,
  icons,
  modified,
  greyed,
  readOnly,
  hit,
  query,
  onChange,
  onReset,
}: Props) {
  const label = meta.label?.zh || meta.label?.en || paramKey
  const title = `${paramKey}${meta.mode ? `  [${meta.mode}]` : ''}`
    + `${greyed ? '  · BBS 里此项当前不可用' : ''}`
    + `${hit ? `  · ${hitNote(hit, paramKey, meta)}` : ''}`
  const base = norm(baseline[paramKey], variantIdx)

  /* 只有「命中在标签上」才有可圈的字。命中在 key 或值上时标签一个字都不动 */
  const parts = hit?.where === 'label' ? sliceLabel(label, query) : null

  const cls = [s.row, modified ? s.modified : '', greyed ? s.greyed : ''].filter(Boolean).join(' ')

  return (
    <div className={cls} data-key={paramKey} data-hit={hit ? 'true' : undefined}>
      <span className={s.lbl} title={title}>
        {parts
          ? <>{parts[0]}<mark className={s.mark}>{parts[1]}</mark>{parts[2]}</>
          : label}
      </span>

      {/*
        ↺ **只按「改过没改过」决定画不画**，展示模式下画但点不动。
        理由：展示模式是这一页的默认态，按「可编辑才画」写的话，默认就看不见这个箭头 ——
        而它除了「能重置」还担着第二个职责：告诉你**基准值是多少**（悬停那句话）。
        颜色不用管：undo.svg 的 #F1754E 写死在 svg 里（BBS 的 Tab.cpp:313），
        CSS 的 color 对它不生效，所以「禁用但仍是橙的」是免费的。
      */}
      {modified ? (
        <button
          type="button"
          className={s.reset}
          disabled={readOnly}
          title={`恢复默认值 ${base === undefined ? '（基准缺）' : String(base)}`
            + (readOnly ? ' · 展示模式下不可点，切到「可改动」' : '')}
          aria-label={`恢复 ${label} 的默认值`}
          onClick={() => onReset(paramKey)}
        >
          <BbsIcon name="undo" icons={icons} fallback="↺" />
        </button>
      ) : (
        <span className={s.resetSlot} />
      )}

      <BbsControl
        paramKey={paramKey}
        meta={meta}
        value={value}
        values={values}
        variantIdx={variantIdx}
        icons={icons}
        greyed={greyed}
        readOnly={readOnly}
        onChange={onChange}
      />

      {Array.isArray(value) && value.length > 1 && (
        <span
          className={s.variant}
          title={`按挤出头变体取第 ${variantIdx + 1} 个值；完整值 ${JSON.stringify(value)}`}
        >
          [{variantIdx + 1}/{value.length}]
        </span>
      )}
    </div>
  )
}
