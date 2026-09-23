/**
 * 结构带 3：底部条（30px）。
 *
 * 左边「我在看什么」，右边「有没有没存的」。
 *
 * 与状态条的分工：状态条说的是**配方与产物的关系**（跨全店），
 * 这一条说的是**当前这一屏**。所以「未保存 N 处」这个数在两边都出现 ——
 * 但状态条上它是一个点加两个字（够不够新），这里是一个具体的数（有多少处要存）。
 * 这是刻意的重复：保存这件事在哪个屏都要看得见。
 */
import type { Badges, SaveState, SnapshotState, Words } from '../api'

interface Props {
  badges: Badges
  /** 「主选中 A1 / 标准版」这一句。由页面拼好传进来 —— 它知道选中的是什么 */
  focus: string
  save: SaveState
  dirtyCount: number
  /**
   * 崩溃快照跟上了没有。**与 `save` 是两条独立信息，不许合成一句**：
   * 「未保存」说的是仓库文件里还没有这些改动；
   * 「待落盘」说的是崩溃快照还没跟上 —— 草稿本身在内存里，是真相
   */
  snapshot: SnapshotState
  words: Words
}

export function StatusBar({ badges, focus, save, dirtyCount, snapshot, words }: Props) {
  const snap = words.snapshot[snapshot]
  return (
    <footer className="wb-foot">
      <span className="wb-foot__left">
        {badges.machines} 机型 · {badges.versions} 版本 · {focus}
      </span>
      <span className="wb-foot__right">
        <span className="wb-foot__counts">
          {words.level.machine.label} {badges.baseItems} 项 ·{' '}
          {words.level.version.label} {badges.overrideItems} 项
        </span>
        {/* 快照只在没跟上的时候才占位置：跟上了是常态，常态不值得一直说 */}
        {snapshot !== 'current' && (
          <span className="wb-foot__save" title={snap.explain ?? undefined}>
            <span
              className="wb-dot"
              data-state={snapshot === 'failed' ? 'danger' : 'warn'}
              aria-hidden
            />
            {snap.label}
          </span>
        )}
        <span className="wb-foot__save">
          <span
            className="wb-dot"
            data-state={save === 'dirty' ? 'warn' : 'ok'}
            aria-hidden
          />
          {words.save[save].label}
          {dirtyCount > 0 && <> {dirtyCount} 处</>}
        </span>
      </span>
    </footer>
  )
}
