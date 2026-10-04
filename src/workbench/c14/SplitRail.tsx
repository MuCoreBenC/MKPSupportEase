/*
 * `.split` 两栏页（左栏 + 详情）的**可拖分隔条**（作者 2026-10-03：
 * 「左侧的我希望能拖动他的宽度……要不然做成参数台那样可以拖动的」）。
 *
 * 手感、吸附、键盘、双击复位全部复用 `SplitterC14`（参数台那一份）——
 * 这里只把三件事接到 `.split` 上：
 *
 *  1. 左栏宽挂在容器的 `--split-w` 变量上（`.split` 的 grid 列读它）；
 *  2. 宽度按页各记一份（`useSplitWidth` 的 key 带页名，互不串）；
 *  3. 落点算在容器左栏的右缘（`.splitSplit`）。
 *
 * 用法（三行）：
 * ```tsx
 * const bodyRef = useRef<HTMLDivElement>(null)
 * const rail = useSplitRail('assets', bodyRef)
 * ...
 * <div className={s.split} ref={bodyRef} style={rail.style}> … {rail.handle} </div>
 * ```
 *
 * 拖动中分隔条直接覆写变量、state 只在松手时落一次（SplitterC14 的纪律）。
 */

import type { CSSProperties, ReactElement, RefObject } from 'react'

import SplitterC14 from './SplitterC14'
import { useSplitWidth } from './useSplitWidth'
import s from '../c14.module.css'

/** 默认 / 上下限。窄到 220 清单还读得下，宽到 560 详情仍有位置 */
export const SPLIT_DFLT = 300
export const SPLIT_MIN = 220
export const SPLIT_MAX = 560

export function useSplitRail(page: string, bodyRef: RefObject<HTMLElement | null>) {
  const [w, setW] = useSplitWidth(`split-${page}`, SPLIT_DFLT, SPLIT_MIN, SPLIT_MAX)
  return {
    w,
    /** 挂到 `.split` 容器上 —— grid 列读它 */
    style: { ['--split-w' as string]: `${w}px` } as CSSProperties,
    /** 放在容器 children 的最后（绝对定位，落在左栏右缘） */
    handle: (
      <SplitterC14
        bodyRef={bodyRef}
        side="rail"
        varName="--split-w"
        width={w}
        otherW={0}
        defaultValue={SPLIT_DFLT}
        min={SPLIT_MIN}
        max={SPLIT_MAX}
        onCommit={setW}
        label="左栏宽度"
        className={s.splitSplit}
      />
    ) as ReactElement,
  }
}
