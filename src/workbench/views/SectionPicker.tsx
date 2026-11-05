/*
 * 「所属分组」的自绘下拉（2026-11-01）。
 *
 * # 为什么不再用原生 `<select>`
 *
 * 原生下拉的每一行只能是一个选项：分组名（页签）只能当 `optgroup` 的死标签，
 * 想在页签那一行挂一颗「＋ 新增分组」根本没有地方放。系统画的弹层也改不了
 * 字号与勾选样式 —— 与 `c14/field/SelectField` 的取舍同一条。
 *
 * # 面板是 `SectionList`，与分组抽屉同一份
 *
 * 这里只负责「触发器 + 浮层」这层壳：按页签分块、行内改名/上下移/新建，全在
 * `SectionList` 里 —— 同一块面板两处用，行为没有第二种实现。
 *
 * # 一切仍只是草稿
 *
 * 面板发出来的手势只改 `ParamDefModal` 的草稿（`sectionId` / `newSection` /
 * 顺序），**确认保存那一趟**才跟参数定义一起落盘：建了又取消，数据里不会留一个
 * 谁也删不掉（这里没有删分组的入口）的空组。所以新建的行先带「待保存」。
 *
 * # id 不是名字
 *
 * 分组 id 会进两份 TOML、还会进客户端目录，只收 a-z / 0-9 / `_`；中文标签没法
 * 直接当 id，所以这里只让人写**名字**，id 取 `custom_N`（由调用方算，见 `nextSectionId`）。
 */
import { useState } from 'react'

import FieldPopover from '../c14/field/FieldPopover'
import SectionList from './SectionList'
import type { SectionGroup } from './SectionList'
import s from './SectionPicker.module.css'

export type { SectionGroup }

interface Props {
  groups: SectionGroup[]
  /** 当前选中的分组 id（可能是还没落盘的那个新分组） */
  value: string
  onPick: (id: string) => void
  /** 在某个页签下新建一个分组（只进草稿；名字已经过调用方校验） */
  onAdd: (tabId: string, label: string) => void
  /** 这一行是不是草稿里的新分组（显示「待保存」） */
  isPending: (id: string) => boolean
  /** 行内改名（只进草稿） */
  onRename?: (id: string, label: string) => void
  /** 组间换位（只进草稿的顺序） */
  onMove?: (id: string, dir: -1 | 1) => void
}

function Chevron({ up }: { up: boolean }) {
  return (
    <svg
      className={up ? s.caretUp : s.caret}
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

export default function SectionPicker({
  groups,
  value,
  onPick,
  onAdd,
  isPending,
  onRename,
  onMove,
}: Props) {
  const [open, setOpen] = useState(false)
  /** 触发器。浮层要拿它量位置，所以得进 state（首帧 ref 还是 null） */
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null)

  const flat = groups.flatMap((g) => g.sections)
  const current = flat.find((x) => x.id === value) ?? null
  const currentTab = current === null ? undefined : groups.find((g) => g.tabId === current.tabId)

  /*
   * 关面板**不抢焦点**（2026-11-02 作者：「这个点击了就自动收回，闪一下」）。
   *
   * 原来这里在关的同时 `anchor.focus()`：点完一行，焦点从面板里那颗按钮被拽回
   * 触发器 —— 两次焦点切换在同一个事件里完成，界面就是"闪一下"。
   * 空手点遮罩那种关法本来就不该动焦点（光标落在哪儿是用户的事），
   * 所以只有「刚从这里选完 / 刚在这里建完」才把焦点还给触发器（见下面 `onPick`）。
   */
  const close = () => setOpen(false)

  /**
   * 选完一组：收起面板并把焦点还给触发器 —— 这是**用户明确完成了一次选择**，
   * 光标回到刚才操作的那一格才对得上。
   *
   * 收得晚一格（`setTimeout 0`）：同一拍里收起会让面板在 `click` 之前就消失，
   * 选中态与标题一起闪；等到这一拍结束再收，人看到的是一行"亮了一下"然后收起。
   */
  const pickAndClose = (id: string) => {
    onPick(id)
    setOpen(false)
    window.setTimeout(() => anchor?.focus(), 0)
  }

  return (
    <span className={s.wrap}>
      <button
        ref={setAnchor}
        type="button"
        className={s.trigger}
        aria-label="所属分组 (sectionId)"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => (open ? close() : setOpen(true))}
      >
        <span className={s.text}>
          {current === null ? (
            '选一个分组'
          ) : (
            <>
              <span className={s.tabOf}>{currentTab?.tabLabel ?? current.tabId}</span>
              <span className={s.dot}>·</span>
              {current.label}
              {isPending(current.id) && <span className={s.badge}>待保存</span>}
            </>
          )}
        </span>
        <Chevron up={open} />
      </button>

      {open && (
        <FieldPopover anchor={anchor} onClose={close}>
          <div className={s.panel}>
            <SectionList
              groups={groups}
              value={value}
              selectable
              onPick={pickAndClose}
              onRename={onRename ?? (() => undefined)}
              onMove={onMove}
              onAdd={onAdd}
              isPending={isPending}
              hint="新分组跟着这次保存一起落盘 —— 取消保存的话它不会留下。"
            />
          </div>
        </FieldPopover>
      )}
    </span>
  )
}
