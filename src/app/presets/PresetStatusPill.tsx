/*
 * 融合 pill（G07-1 收编，A44 三轮定稿；2026-10-04 从试验台收编进产品）。
 *
 * ```
 * ┌ ● A1.toml · A1 │ ▽ A1 ▾ ┐
 *   └──── 左拍 ────┘ └ 右拍 ┘
 * ```
 *
 * # 这一颗件上是三个事实（作者的原话拆开的）
 *
 *   左拍（信息）  正在生效的是谁 —— 唯一底账 `run/active-preset.json` 答的那一条。
 *                 **前面那个字 2026-10-10 第三轮撤了**（先是「已应用」、后来「在用」；
 *                 作者：「节省一点字，不用那个在用」）—— 绿点 + 淡绿底已经说明
 *                 "这一份在生效"，字面重复
 *   左拍（动作）  点它 = 定位 —— **清掉机型筛选** + 把正在生效的那一行滚到眼前闪一下。
 *                 不写「定位」两个字（作者第三轮：「不用显示定位两个字」），悬停有提示
 *   右拍（筛选）  看哪个机型的 —— 漏斗 + 当前值，点开菜单选机型。
 *                 **它是筛选，不是切换**（作者第二轮纠正：「这个是个筛选的……不能再用
 *                 抽屉了」）—— 所以这里没有抽屉、没有「切换」两个字，选完就地筛这张表。
 *                 **窄窗里它一寸不让**（`.hold` 不收缩，见 CSS）—— 收缩全由左拍名字承担
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
 * # 产品仓补的那一枚：本地文件
 *
 * 正在使用的可能是**用户自己那份**（第七层：两边都能成为使用中的那一份）——
 * 这一枚不能省：用户得看得出"现在跑的不是云端那份"（它不跟着云端更新）。
 * 试验台的假数据没有本地文件，所以 A44 原件上没有它；搬进产品时补上
 * （与旧状态条上那一枚同义，只是住处搬进了左拍）。
 *
 * # 切片器那一颗
 *
 * 切片器没有「已应用」这个概念（它的生效 = 复制到切片器目录，可以是多份），
 * 所以它的 pill 左拍**不是信息而是筛选**（`PresetSlicerPill`，见下一节）；
 * 右拍照旧是同一个 `PresetMachineFilter` —— 机型筛选两档共用这一颗。
 *
 * # 定位为什么是「清筛选 + 滚到眼前」
 *
 * 机型筛选可能正好把正在生效的那一行筛没了（筛到 P2S 一行都不剩）——
 * 作者不愿意看到「筛完之后不知道哪套在生效」。点左拍把筛选清回「全部机型」，
 * 等重画完把那一行滚到视口中央闪一下（`PagePresets` 的 locateApplied，
 * 与收编前同一个动作，只是入口从「定位」按钮换成了左拍整块）。
 *
 * # 切片器那颗（`PresetSlicerPill`，2026-10-10）
 *
 * 作者裁决：「我希望做成 MKP 那样的」—— 切片器档第二排不再摆三颗常驻下拉
 * （机型 / 喷嘴 / 层高；作者：「它 3 个筛选感觉很讨厌，特别讨厌」），换成**同一颗 pill**：
 *
 * ```text
 * ┌ ▽ 喷嘴 0.2 · 层高 0.12 │ ▽ A1 ▾ ┐
 * └────── 左拍：喷嘴 × 层高 ──┘ └ 机型 ┘
 * ```
 *
 * 左拍点开是一张**两组的弹层**（喷嘴一行、层高一行，值用胶囊平铺）—— 作者要的
 * 「点开了之后它同时就显示喷嘴和层高」；右拍与 MKP 那颗一字不差（竖线隔开的机型筛选）。
 * 层高那一行画的是**全集**：当前喷嘴没有的档位灰着、点不动、不消失（作者 2026-10-10）。
 * 两颗 pill 共用这一份皮肤（`.pill` / `.divider` / 排在一行的 `.filterBtn`），
 * 所以两档第二排长得一样。
 *
 * 独立的 `PresetAxisFilter`（喷嘴 / 层高各一颗下拉）与 `inFilterRow` 那种白底形态
 * 随之退役 —— 切片器筛选从此只有这一种摆法。
 */

import { useRef, useState } from 'react'
import { FieldPopover } from '../../components/field'
import type { ActivePreset, Machine } from '../../api'
import s from './PresetStatusPill.module.css'

interface FilterOption {
  value: string
  label: string
  /** 行尾那截灰色小字（机型的 id；喷嘴 / 层高没有这一格） */
  meta?: string
}

/**
 * 筛选按钮的**通用件**（2026-10-10 从机型那一颗抽出来）：一颗「漏斗 + 当前值 + 箭头」
 * 的按钮，点开是一张**弹层菜单**。两处用它，都是 pill 的右拍（MKP 那颗、切片器那颗）。
 *
 * 喷嘴 / 层高那种「一颗按钮里两组值」的形态（切片器的左拍）**不走这里** ——
 * 它有自己的面板（`PresetSlicerPill` 里的 `AxisGroup`）。
 *
 * # 为什么是菜单不是 chips
 *
 * 值可能很多：一台机型有四个喷嘴、一个喷嘴有六档层高 —— 平铺成一排会把工具条
 * 越撑越宽（而且「层高全部」那一档永远列不全）。菜单宽度固定、里面能滚，
 * 多少个值都不改变这一排的形状。
 *
 * # 「筛上了」看得见
 *
 * `value !== ''`（不是「全部」那一档）时漏斗转绿 —— 见 `.filterBtn[data-filt]`。
 */
interface PresetFilterProps {
  /** 触发器上值前面那个小前缀（筛选区写「机型 / 喷嘴 / 层高」；pill 的右拍不写） */
  prefix?: string
  /** 触发器上「全部」那一档的字（短，恒「全部」） */
  allText: string
  /** 菜单第一档的字（长：「全部机型」/「全部喷嘴」/「全部层高」） */
  allMenuText: string
  /** 菜单第一档右侧的灰色补充（机型的「N 台」；喷嘴 / 层高没有） */
  allMeta?: string
  options: FilterOption[]
  value: string
  onPick: (value: string) => void
}

export function PresetFilter({
  prefix,
  allText,
  allMenuText,
  allMeta,
  options,
  value,
  onPick,
}: PresetFilterProps) {
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const current = options.find((o) => o.value === value)

  return (
    <span className={s.hold}>
      <button
        type="button"
        ref={triggerRef}
        className={s.filterBtn}
        data-filt={value !== ''}
        aria-haspopup="true"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <FunnelIcon />
        {prefix !== undefined && <span className={s.filterPrefix}>{prefix}</span>}
        <b className={s.filterName}>{current?.label ?? allText}</b>
        {/* 箭头是 CSS 画的两条边（见 .caret）：不排「▼」字形，免得它自带一套基线 */}
        <i className={s.caret} aria-hidden />
      </button>
      {open && (
        <FieldPopover anchor={triggerRef.current} align="end" onClose={() => setOpen(false)}>
          <div className={s.menu}>
            {/* 第一档恒是「全部」（那一档的值是空串，与原来的下拉同一个值） */}
            <button
              type="button"
              className={s.item}
              data-on={value === ''}
              onClick={() => {
                setOpen(false)
                onPick('')
              }}
            >
              <span className={s.check} aria-hidden>
                ✓
              </span>
              {allMenuText}
              {allMeta !== undefined && <span className={s.mid}>{allMeta}</span>}
            </button>
            {options.map((o) => (
              <button
                key={o.value}
                type="button"
                className={s.item}
                data-on={value === o.value}
                onClick={() => {
                  setOpen(false)
                  onPick(o.value)
                }}
              >
                <span className={s.check} aria-hidden>
                  ✓
                </span>
                {o.label}
                {o.meta !== undefined && <span className={s.mid}>{o.meta}</span>}
              </button>
            ))}
          </div>
        </FieldPopover>
      )}
    </span>
  )
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

interface FilterProps {
  machines: Machine[]
  /** 当前机型。**空串 = 「全部机型」那一档**（与原 PresetPicker 同一个值） */
  machineId: string
  onPick: (machineId: string) => void
  /** 触发器上值前面那个小前缀（pill 的右拍不写） */
  prefix?: string
}

/**
 * 机型筛选：`PresetFilter` 的机型版（选项是各台机型、行尾带 id）。
 *
 * 两处用它 —— **两颗 pill 的右拍**（MKP 那颗、切片器那颗），一处不差。
 * 筛上了（不是「全部机型」）漏斗转绿：筛选在生效这件事要看得见。
 */
export function PresetMachineFilter({ machines, machineId, onPick, prefix }: FilterProps) {
  return (
    <PresetFilter
      prefix={prefix}
      allText="全部"
      allMenuText="全部机型"
      allMeta={`${machines.length} 台`}
      options={machines.map((m) => ({ value: m.id, label: m.display, meta: m.id }))}
      value={machineId}
      onPick={onPick}
    />
  )
}

interface AxisGroupProps {
  /** 组标题（「喷嘴」/「层高」） */
  label: string
  /**
   * **画出来的全集** —— 数据层给的：喷嘴是当前表里真有文件的那些，层高是
   * **所有喷嘴的并集**（`slicerFilters.allLayers`）。这里一个都不重算。
   */
  values: string[]
  /**
   * **能点的子集**（不给 = 全都可点）。层高那一组传 `layerOptions`：
   * 当前喷嘴没有的档位**留在原位、灰着、点不动**。
   *
   * ★ 2026-10-10 作者改判：「我希望我选了它，筛选出来的不是把别的消失……
   *   别的变成那种灰色不让点击的」。所以**留而不删** —— 那些档位本身是信息
   *   （"这个喷嘴没这档"），收走之后用户只会以为界面上从来没这个东西。
   */
  enabled?: string[]
  value: string
  onPick: (value: string) => void
}

/**
 * 弹层里的一组值（喷嘴或层高）：一行标题 + 一排胶囊（「全部」恒排第一）。
 *
 * 为什么是**平铺胶囊**而不是下拉：一个喷嘴最多六档层高、一台机型四个喷嘴 ——
 * 都放得下，而且**两组的选项同时看得见**（作者要的「点开了之后它同时就显示
 * 喷嘴和层高」）。下拉一次只能展开一组，这正是不想要的那种。
 *
 * 选中后弹层**不关闭**（与 `PresetFilter` 的单选不同：那里选完就走）：
 * 喷嘴与层高是一对 —— 挑完喷嘴，层高那一组立刻按新喷嘴重算谁可点，接着挑，连选是常态。
 */
function AxisGroup({ label, values, enabled, value, onPick }: AxisGroupProps) {
  return (
    <div className={s.axisGroup}>
      <span className={s.axisLabel}>{label}</span>
      <span className={s.axisValues}>
        <button
          type="button"
          className={s.axisChip}
          data-on={value === ''}
          onClick={() => onPick('')}
        >
          全部
        </button>
        {values.map((v) => {
          /* 灰态：这一档当前喷嘴没有 —— 留在原地不给点（见 `enabled` 的说明） */
          const off = enabled !== undefined && !enabled.includes(v)
          return (
            <button
              key={v}
              type="button"
              className={s.axisChip}
              data-on={value === v}
              data-off={off}
              disabled={off}
              /* 灰一个项不说原因，用户只会以为坏了 —— 悬停给一句人话 */
              title={off ? `当前喷嘴没有「${label} ${v}」这一档` : undefined}
              onClick={() => onPick(v)}
            >
              {v}
            </button>
          )
        })}
      </span>
    </div>
  )
}

interface SlicerPillProps {
  /* 右拍那颗机型筛选的三个入参，与 `PresetMachineFilter` 同一套 */
  machines: Machine[]
  machineId: string
  onPickMachine: (machineId: string) => void
  /* 左拍：喷嘴 × 层高（两份集合都由数据层算好） */
  nozzles: string[]
  nozzle: string
  onPickNozzle: (value: string) => void
  /** 层高**全集**（画出来；这一档当前喷嘴没有的话就灰着） */
  layers: string[]
  /** 层高**可用集**（跟着喷嘴收敛；面板按它决定谁灰、谁可点） */
  layerOptions: string[]
  layer: string
  onPickLayer: (value: string) => void
}

/**
 * **切片器那颗 pill**（2026-10-10 作者裁决，见文件头「切片器那颗」一节）：
 * 与 MKP 那颗同一套皮肤、同一条竖线、同一个机型筛选在右拍 —— 差别只在左拍是什么。
 *
 *   MKP 左拍      「● 已应用 文件名 · 机型」  信息，点击 = 定位
 *   切片器左拍    「▽ 喷嘴 0.2 · 层高 0.12」  筛选，点击 = 展开两组的弹层
 *
 * 整颗的底色跟「筛没筛」走（`data-filt`）：一颗都没筛 = 中性 sunken（与 MKP
 * 「还没有使用任何预设」那一档同一种底），筛上 = 淡绿 —— 「长得一样」包括这条信号。
 */
export function PresetSlicerPill({
  machines,
  machineId,
  onPickMachine,
  nozzles,
  nozzle,
  onPickNozzle,
  layers,
  layerOptions,
  layer,
  onPickLayer,
}: SlicerPillProps) {
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const filtered = nozzle !== '' || layer !== ''

  /** 左拍上的一小节（「喷嘴 0.2」）：没筛就只留标签，筛上才长出值 */
  const axisItem = (label: string, value: string) => (
    <span className={s.axisItem}>
      <span className={s.axisPrefix}>{label}</span>
      {value !== '' && <b className={s.axisVal}>{value}</b>}
    </span>
  )

  return (
    <span className={s.pill} data-filt={filtered}>
      <button
        type="button"
        ref={triggerRef}
        className={s.filterBtn}
        /* 左拍形态（左圆角、撑满 pill 高、透明底）—— 见 .pill .filterBtn[data-side='left'] */
        data-side="left"
        data-filt={filtered}
        aria-haspopup="true"
        aria-expanded={open}
        /* 窄窗里左拍的值会被省略号吃掉，悬停看全 */
        title={`喷嘴 ${nozzle === '' ? '全部' : nozzle} · 层高 ${layer === '' ? '全部' : layer}`}
        onClick={() => setOpen((v) => !v)}
      >
        <FunnelIcon />
        <span className={s.axisText}>
          {axisItem('喷嘴', nozzle)}
          <i className={s.sep} aria-hidden />
          {axisItem('层高', layer)}
        </span>
        <i className={s.caret} aria-hidden />
      </button>
      <i className={s.divider} aria-hidden />
      <PresetMachineFilter machines={machines} machineId={machineId} onPick={onPickMachine} />
      {open && (
        <FieldPopover anchor={triggerRef.current} align="start" onClose={() => setOpen(false)}>
          <div className={s.axisPanel}>
            <AxisGroup label="喷嘴" values={nozzles} value={nozzle} onPick={onPickNozzle} />
            {/* 层高画全集、按当前喷嘴给「能点的子集」—— 别的灰着不消失（见 AxisGroup 的 enabled） */}
            <AxisGroup
              label="层高"
              values={layers}
              enabled={layerOptions}
              value={layer}
              onPick={onPickLayer}
            />
          </div>
        </FieldPopover>
      )}
    </span>
  )
}

interface Props {
  /** 正在生效的唯一那一条（底账 `run/active-preset.json`）。`null` = 一套都还没应用（不是错误） */
  applied: ActivePreset | null
  appliedFileName: string
  appliedMachineText: string
  /** 左拍的点击 = 定位（清筛选 + 滚到眼前）。没有已应用时左拍不可点 */
  onLocate: () => void
  /* 右拍那一颗（机型筛选）的三个入参，与 `PresetMachineFilter` 同一套 */
  machines: Machine[]
  machineId: string
  onPick: (machineId: string) => void
}

export default function PresetStatusPill({
  applied,
  appliedFileName,
  appliedMachineText,
  onLocate,
  machines,
  machineId,
  onPick,
}: Props) {
  return (
    <span className={s.pill} data-empty={applied === null}>
      {applied === null ? (
        /* 一套都还没应用：左拍退化成一句说明（不是错误色），右拍筛选照常 ——
           筛选不依赖「有没有应用过」，它只管这张表看哪个机型 */
        <span
          className={s.left}
          data-empty="true"
        >
          <i className={s.dotOff} aria-hidden />
          还没有使用任何预设
        </span>
      ) : (
        <button
          type="button"
          className={s.left}
          /* 名字在窄窗里会被省略号吃掉 —— 悬停看全名；「点击定位」这件事也在这句话里 */
          title={`点击定位：${appliedFileName}`}
          onClick={onLocate}
        >
          <i className={s.dot} aria-hidden />
          <b className={s.name}>{appliedFileName}</b>
          {/* 分隔点也是画的（见 .sep）：中点字符「·」自带一套基线 */}
          <span className={s.sep} aria-hidden />
          <span className={s.mach}>{appliedMachineText}</span>
          {/* a03：去掉「本地文件」徽章（这一页本来就只有自己的文件）。
              a04 第三轮：「在用」两个字也撤了（作者：「节省一点字，不用那个在用」）——
              绿点 + 淡绿底已经说明"这一份在生效"，字面重复 */}
        </button>
      )}
      <i className={s.divider} aria-hidden />
      <PresetMachineFilter machines={machines} machineId={machineId} onPick={onPick} />
    </span>
  )
}
