/*
 * 左侧抽屉：选哪一份预设。
 *
 * 上游那个是挂在预设名栏下面的下拉，这一稿改成常驻抽屉 —— 278 份系统预设 + 你自己的
 * 一堆用户预设，放在一个 70vh 的浮层里翻很难受，而这一页的主要动作就是「换一份看看」。
 *
 * 分组顺序照 BBS（PresetComboBoxes.cpp:1382-1387「BBS: move system to the end」）：
 * 导入的 → 用户配置 → 系统配置 → 其他机型 / 读不了的。
 *
 * 三种「不能点」要分清楚：
 *   - 坏文件：真的点不动（disabled），title 说为什么
 *   - 别的机型：**能点**（点了就切过去），只是显示得弱一点 —— 不然你会以为它坏了
 *   - 继承链中间层（fdm_process_*）：根本不进列表，BBS 的下拉里也没有
 */

import { useMemo } from 'react'
import BbsSearchBox from './BbsSearchBox'
import { MODEL_ALL, groupsFor, targetsOf } from './bbsSource'
import type { BbsDrawer } from './useBbsDrawer'
import type { BbsPresetItem } from './bbsTypes'
import s from './BbsPresetDrawer.module.css'

interface Props {
  presets: BbsPresetItem[]
  currentKey: string | null
  dirty: boolean
  target: string
  query: string
  /** 收起时整个不渲染（display:none） */
  open: boolean
  /**
   * 'side' 并排 · 'float' 浮层。
   * **切形态的按钮在顶部工具条上，不在这里** —— 浮层收起时抽屉整个不在了，
   * 按钮跟着消失就再也切不回并排（探针里真撞上了这一下）。
   */
  mode: 'side' | 'float'
  /** 拖宽那一套（热区的 props 与是否正在拖） */
  sizing: BbsDrawer
  onTarget: (next: string) => void
  onQuery: (next: string) => void
  onPick: (key: string) => void
}

export default function BbsPresetDrawer({
  presets,
  currentKey,
  dirty,
  target,
  query,
  open,
  mode,
  sizing,
  onTarget,
  onQuery,
  onPick,
}: Props) {
  const targets = useMemo(() => targetsOf(presets), [presets])
  const groups = useMemo(
    () => groupsFor(presets, { target, currentKey, dirty, query }),
    [presets, target, currentKey, dirty, query],
  )
  const liveCount = presets.filter((p) => !p.isAbstract).length

  /* 当前档位不在列表里（比如导入的文件认不出机型）→ 退到「全部」，别显示一个空列表 */
  const targetValue = targets.some((t) => t.key === target) ? target : MODEL_ALL

  return (
    <aside
      className={s.drawer}
      data-open={open}
      data-mode={mode}
      data-dragging={sizing.dragging ? 'true' : undefined}
      aria-label="预设列表"
    >
      <div className={s.head}>
        <select
          className={s.pick}
          value={targetValue}
          aria-label="按机型与喷嘴筛选"
          onChange={(e) => onTarget(e.target.value)}
        >
          <option value={MODEL_ALL}>全部（{liveCount}）</option>
          {targets.map((t) => (
            <option key={t.key} value={t.key}>
              {/* 「（全部喷嘴）」那一档缩进一格，看得出它是这台机的汇总。
                  缩进用全角空格的转义写法：<option> 里的普通空格会被折叠掉 */}
              {t.level === 'nozzle' ? `\u3000${t.label}（${t.count}）` : `${t.label}（${t.count}）`}
            </option>
          ))}
        </select>

        <BbsSearchBox
          value={query}
          placeholder="搜索预设名…"
          label="搜索预设名"
          variant="fill"
          onChange={onQuery}
        />
      </div>

      {/* data-role 是给验收探针用的稳定钩子（选择器不随 class 哈希变） */}
      <div className={s.list} data-role="preset-list">
        {!groups.length && (
          <p className={s.empty}>
            {query ? `没有名字含「${query}」的预设` : '这个机型下没有预设'}
          </p>
        )}
        {groups.map((g) => (
          <div key={g.title}>
            <div className={s.group} data-weak={g.weak ? 'true' : undefined}>
              {g.title}（{g.items.length}）
            </div>
            {g.items.map((it) => (
              <button
                key={it.key}
                type="button"
                className={s.item}
                data-on={it.current}
                data-off-model={it.offModel ? 'true' : undefined}
                disabled={it.disabled}
                title={it.disabled ? (it.reason ?? '') : `${it.printer} · ${it.nozzle} nozzle · ${it.file}`}
                onClick={() => onPick(it.key)}
              >
                <span className={s.tick}>{it.current ? '✓' : ''}</span>
                {it.star && <span className={s.star}>*</span>}
                <span className={s.name}>{it.label}</span>
              </button>
            ))}
          </div>
        ))}
      </div>

      {/*
        拖宽的热区：抽屉右边缘 6px 的透明带，**外观上什么都没有**，只有 cursor 变化
        （文件管理器那种「悬停就能拖」）。键盘聚焦时才显一条 1px 细线。
      */}
      <span className={s.grip} {...sizing.gripProps} />
    </aside>
  )
}
