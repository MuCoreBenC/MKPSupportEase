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
import type { Badges, SaveState, Words } from '../api'

interface Props {
  badges: Badges
  /** 「主选中 A1 / 标准版」这一句。由页面拼好传进来 —— 它知道选中的是什么 */
  focus: string
  save: SaveState
  dirtyCount: number
  words: Words
}

export function StatusBar({ badges, focus, save, dirtyCount, words }: Props) {
  return (
    <footer className="wb-foot">
      <span className="wb-foot__left">
        {badges.machines} 机型 · {badges.versions} 版本 · {focus}
      </span>
      <span className="wb-foot__right">
        <span className="wb-foot__counts">
          {words.level.machine.label} {badges.baseItems} 项（自有 {badges.baseOwn}） ·{' '}
          {words.level.version.label} {badges.overrideItems} 项（自有 {badges.overrideOwn}）
        </span>
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
