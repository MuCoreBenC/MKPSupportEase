/*
 * 融合 pill（G07-1 收编，A44 三轮定稿；2026-10-04 从试验台收编进产品）。
 *
 * ```
 * ┌ ● 已应用 A1.toml · A1 │ ▽ A1 ▾ ┐
 *   └──────── 左拍 ───────┘ └ 右拍 ┘
 * ```
 *
 * # 这一颗件上是三个事实（作者的原话拆开的）
 *
 *   左拍（信息）  正在生效的是谁 —— 唯一底账 `run/active-preset.json` 答的那一条，
 *                 带「已应用」两个字（作者第二轮点名要保留）
 *   左拍（动作）  点它 = 定位 —— **清掉机型筛选** + 把正在生效的那一行滚到眼前闪一下。
 *                 不写「定位」两个字（作者第三轮：「不用显示定位两个字」），悬停有提示
 *   右拍（筛选）  看哪个机型的 —— 漏斗 + 当前值，点开菜单选机型。
 *                 **它是筛选，不是切换**（作者第二轮纠正：「这个是个筛选的……不能再用
 *                 抽屉了」）—— 所以这里没有抽屉、没有「切换」两个字，选完就地筛这张表
 *
 * 机型筛选从原来的 `PresetPicker` 下拉搬进来（那个组件整个退场）：
 * 机型写在已应用条上本来就是重复信息（作者第一轮：「这个信息你自己看都重复了」），
 * 融合之后机型只在这颗 pill 的右拍出现一次。
 *
 * # 菜单里有什么
 *
 * 「全部机型 N 台」排第一档（空串那一档，与原下拉同一个值），下面各台机型、
 * 行尾灰色是机型 id —— 与原下拉的选项一字不差，只是从 SelectField 换成了
 * FieldPopover 弹层（点开就地筛，筛完「共 N 项」自己变）。
 *
 * # 产品仓补的那一枚：我的文件
 *
 * 正在使用的可能是**用户自己那份**（第七层：两条线都能成为使用中的那一份）——
 * 这一枚不能省：用户得看得出"现在跑的不是官方那份"（它不跟着官方更新）。
 * 试验台的假数据没有用户线，所以 A44 原件上没有它；搬进产品时补上
 * （与旧状态条上那一枚同义，只是住处搬进了左拍）。
 *
 * # 切片器没有这一颗
 *
 * 切片器没有「已应用」这个概念（它的生效 = 复制到切片器目录，可以是多份），
 * 所以切片器那张表上没有 pill —— 机型筛选以**独立漏斗**的形式留在工具条上
 * （named export `PresetMachineFilter`，就是 pill 的右拍单拎出来）。
 *
 * # 定位为什么是「清筛选 + 滚到眼前」
 *
 * 机型筛选可能正好把正在生效的那一行筛没了（筛到 P2S 一行都不剩）——
 * 作者不愿意看到「筛完之后不知道哪套在生效」。点左拍把筛选清回「全部机型」，
 * 等重画完把那一行滚到视口中央闪一下（`PagePresets` 的 locateApplied，
 * 与收编前同一个动作，只是入口从「定位」按钮换成了左拍整块）。
 */

import { useRef, useState } from 'react'
import { FieldPopover } from '../../components/field'
import type { ActivePreset, Machine } from '../../api'
import s from './PresetStatusPill.module.css'

interface FilterProps {
  machines: Machine[]
  /** 当前机型。**空串 = 「全部机型」那一档**（与原 PresetPicker 同一个值） */
  machineId: string
  onPick: (machineId: string) => void
}

/** 漏斗（筛选的形状 —— 作者：「应该改成筛选的就是那个像一个漏斗一样的那个形状」） */
function FunnelIcon() {
  return (
    <svg className={s.funnel} viewBox="0 0 14 14" aria-hidden>
      <path
        d="M1.5 2h11L8.6 7.2v4.3L5.4 9.7V7.2L1.5 2z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinejoin="round"
      />
    </svg>
  )
}

/* 菜单本体：「全部机型」第一档 + 各台机型（行尾灰色 id，与原下拉同一份选项） */
function MachineMenu({ machines, machineId, onPick }: FilterProps) {
  return (
    <div className={s.menu}>
      <button type="button" className={s.item} data-on={machineId === ''} onClick={() => onPick('')}>
        <span className={s.check} aria-hidden>
          ✓
        </span>
        全部机型
        <span className={s.mid}>{machines.length} 台</span>
      </button>
      {machines.map((m) => (
        <button
          key={m.id}
          type="button"
          className={s.item}
          data-on={machineId === m.id}
          onClick={() => onPick(m.id)}
        >
          <span className={s.check} aria-hidden>
            ✓
          </span>
          {m.display}
          <span className={s.mid}>{m.id}</span>
        </button>
      ))}
    </div>
  )
}

/**
 * 机型筛选（pill 的右拍，也是切片器工具条上的独立漏斗）。
 *
 * 筛上了（不是「全部机型」）漏斗转绿 —— 筛选在生效这件事要看得见；
 * 触发器上写着当前机型名，只有「全部」那一档例外（那一档本来就没有机型可写）。
 */
export function PresetMachineFilter({ machines, machineId, onPick }: FilterProps) {
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const current = machines.find((m) => m.id === machineId)

  return (
    <span className={s.hold}>
      <button
        type="button"
        ref={triggerRef}
        className={s.filterBtn}
        data-filt={machineId !== ''}
        aria-haspopup="true"
        aria-expanded={open}
        title="机型筛选（菜单里能选「全部机型」）"
        onClick={() => setOpen((v) => !v)}
      >
        <FunnelIcon />
        <b className={s.filterName}>{current?.display ?? '全部'}</b>
        <i className={s.caret} aria-hidden>
          ▼
        </i>
      </button>
      {open && (
        <FieldPopover
          anchor={triggerRef.current}
          align="end"
          onClose={() => setOpen(false)}
        >
          <MachineMenu
            machines={machines}
            machineId={machineId}
            onPick={(id) => {
              setOpen(false)
              onPick(id)
            }}
          />
        </FieldPopover>
      )}
    </span>
  )
}

interface Props extends FilterProps {
  /** 正在生效的唯一那一条（底账 `run/active-preset.json`）。`null` = 一套都还没应用（不是错误） */
  applied: ActivePreset | null
  appliedFileName: string
  appliedMachineText: string
  /** 正在使用的是**用户自己那份**（第七层）—— 左拍多一枚「我的文件」，A44 原件没有这一枚 */
  appliedIsMine: boolean
  /** 左拍的点击 = 定位（清筛选 + 滚到眼前闪一下）。没有已应用时左拍不可点 */
  onLocate: () => void
}

export default function PresetStatusPill({
  applied,
  appliedFileName,
  appliedMachineText,
  appliedIsMine,
  onLocate,
  machines,
  machineId,
  onPick,
}: Props) {
  return (
    <span className={s.pill}>
      {applied === null ? (
        /* 一套都还没应用：左拍退化成一句说明（不是错误色），右拍筛选照常 ——
           筛选不依赖「有没有应用过」，它只管这张表看哪个机型 */
        <span
          className={s.left}
          data-empty="true"
          title="在下面挑一套 MKP 配置，点「应用」把它设为当前的"
        >
          <i className={s.dotOff} aria-hidden />
          还没有应用任何预设
        </span>
      ) : (
        <button
          type="button"
          className={s.left}
          title="点一下：清掉机型筛选，把正在生效的那一行滚到眼前"
          onClick={onLocate}
        >
          <i className={s.dot} aria-hidden />
          <span className={s.label}>已应用</span>
          <b className={s.name}>{appliedFileName}</b>
          <span className={s.sep} aria-hidden>
            ·
          </span>
          <span className={s.mach}>{appliedMachineText}</span>
          {appliedIsMine && (
            <span
              className={s.mine}
              title="正在使用的是你自己那份（presets-mine/…）—— 官方怎么更新都不会动它"
            >
              我的文件
            </span>
          )}
        </button>
      )}
      <i className={s.divider} aria-hidden />
      <PresetMachineFilter machines={machines} machineId={machineId} onPick={onPick} />
    </span>
  )
}
