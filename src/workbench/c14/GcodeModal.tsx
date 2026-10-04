/*
 * 多行 G-code 的模态框（C14 §八移植）。
 *
 * 作者：「你应该给我一个按钮才能打开模态框，不要点击就打开」。
 * 所以入口只有**行上那枚按钮**（`.pGbig`，压在框的右上角）：点框本身只是把
 * 光标放进那几行里就地改，不弹东西。模态框是「看全 + 一次读十几行」的地方 ——
 * 172px 宽的框里读 `G1 X261 F10000` 这种长行要靠横向滚，读不了。
 *
 * 三件事跟共用件的 `Modal` 对齐：
 *
 *   1. **不点遮罩关**（`closeOnScrim={false}`）：框里是正在改的东西，手滑点一下
 *      外面就关掉，观感像「刚写的东西没了」（草稿其实还在，但人不会这么读）。
 *      Esc 与**页眉那一枚**「关闭」在 —— 页脚不再放第二枚（作者：
 *      「他现在有两个关闭，我觉得有点多余，只留一个关闭吧」）。
 *   2. **挂载点由宿主给**：经 `ModalC14` 落到外壳交出的 `.shellBody` ——
 *      遮罩盖住导航 + 页面 + 状态栏那一整层。
 *   3. **写值仍然只有一处**：这里不自己算，走宿主给的 `write`（与行上、
 *      与右栏各层同一条路 —— 已弃用那道闸也在那儿）。
 *
 * # 三个出口各管什么（第十九轮）
 *
 *   关闭（页眉那枚）  只是收框 —— 改动**留在草稿里**，行上继续显示，
 *                     那枚「已修改」标签说的就是它；保存才落盘。
 *   取消（页脚）      把这一层写回**进框之前**的值再收框；没改过就是灰的。
 *   保存（页脚）      把工作台的未保存改动落盘（外壳那条保存路径）。
 *
 * 标题底下那行小字写**在哪一层**（`A1 / 标准版`、`A1 基底`）：同一个参数
 * 各版本值不一样，不写清楚就会改错一层。
 */

import { useState } from 'react'

import type { Cell, ParamView } from '../api'
import { toasts } from './toast'
import ModalC14 from './ModalC14'
import GcodeEditor from './GcodeEditor'
import s from '../c14.module.css'

interface Props {
  param: ParamView
  cell: Cell
  /** 「A1 / 标准版」或「A1 基底」—— 标题下面那一行 */
  layerLabel: string
  disabled?: boolean
  /** 外壳的草稿脏不脏（保存按钮用） */
  dirty: boolean
  /** 写这一层的值。**唯一入口** —— 弃用闸在那条路上 */
  write: (next: string) => void
  /** 外壳的保存（落盘） */
  onSave: () => void
  onClose: () => void
}

export default function GcodeModal({
  param,
  cell,
  layerLabel,
  disabled = false,
  dirty,
  write,
  onSave,
  onClose,
}: Props) {
  const before = String(cell.raw ?? '')
  /*
   * 行数按**有内容的行**算：结尾的空行只是编辑过程的临时态（编辑器失焦会
   * 把它摘掉，见 `GcodeEditor.settle`），存草稿的历史值里可能还带着，
   * 拿它当「一行」标题就会写「10 行」而正文只有 9 行。
   */
  const rows = before === '' ? 0 : before.replace(/\n+$/, '').split('\n').length

  /*
   * 「取消」回到的原点：**进框那一刻**的值（C14 第十九轮，作者：
   * 「保存旁边的应该就是取消」）。挂载时拍一份快照 —— 挂过就重新挂的
   * 条件渲染，每次打开都是新的一次挂载，快照天然就是当次的。
   */
  const [openValue] = useState(before)

  /** 取消 = 把这一层写回快照（一次直接的写，比 N 步撤销清楚），然后关框 */
  const cancel = () => {
    if (String(cell.raw ?? '') !== openValue) {
      write(openValue)
      toasts.push('已取消这一层的改动 —— 回到打开框之前的值')
    }
    onClose()
  }

  return (
    <ModalC14
      open
      size="lg"
      title={param.label}
      subtitle={`${layerLabel} · ${rows} 行${disabled ? ' · 这一层改不动' : ''}`}
      closeOnScrim={false}
      /* 这里的每一笔写都进外壳的草稿栈 —— Cmd+Z 得照常够得着它们（页脚那句就是它） */
      shellShortcuts
      onClose={onClose}
      closeTitle="改动留在草稿里（没有保存）—— 想丢弃这次改动，用下面的「取消」"
      footer={
        <>
          <span className={s.pGmNote}>
            改哪一行都直接落进草稿，Ctrl+Z 可撤回；<b>[AUTO]</b> 是变量，生成时才展开
          </span>
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            disabled={String(cell.raw ?? '') === openValue}
            title={
              String(cell.raw ?? '') === openValue
                ? '这一层还没改过 —— 没有可取消的'
                : '把这一层改回打开框之前的值，然后关框'
            }
            onClick={cancel}
          >
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
            disabled={!dirty}
            title={dirty ? '把工作台里的未保存改动落盘' : '没有未保存的改动'}
            onClick={onSave}
          >
            保存
          </button>
        </>
      }
    >
      <GcodeEditor
        value={before}
        onChange={write}
        disabled={disabled}
        gutter
        size="modal"
        label={`${param.label}（${layerLabel}）`}
      />
    </ModalC14>
  )
}
