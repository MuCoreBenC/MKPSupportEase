/*
 * 预设抽屉：按机型分组列出**本机真有**的预设，一行 = 一份文件。
 *
 * 为什么是抽屉而不是两个下拉：作者的原话是「现在那个下拉菜单，我不喜欢。我喜欢改成抽屉的」。
 * 用户端早先那一版顶部就是「文件名 pill + 切换」，点了开一个文件抽屉 —— 这里回到那个形态。
 *
 * 两条实现上的硬约束：
 *   - `position: absolute` 挂在参数页的定位父级上，**不用 fixed**：
 *     外壳上的一层 transform 就是 fixed 的包含块（那两个点不开的浮层就栽在这上面）。
 *   - 切预设一律走调用方给的 `onUse` → `u.requestCombo`，它自带「有未保存改动先问一句」。
 *     抽屉自己**不判断脏数据**，免得两处各写一套确认。
 *
 * # ★★ 2026-10-14 改口径：这里**只列本机真有的**（作者裁定，三条规则）
 *
 * 改之前它列的是**目录（云端）**那一套：`machines` 来自 `catalog.machines`、
 * 文件名来自 `catalog.files[]` —— 于是本机只有一份 `A1_MINI-fastv3.3.toml`，
 * 抽屉里却把 A1 / A1 mini / P1S / P2S / X1C 的 12 个组合全列了出来，
 * 点任何一个都换不过去（校准要写回本机那份 TOML，压根没有）。
 *
 * 现在是三条规则：
 *   ① **点行什么都不做** —— 不切换、不动参数、不改正在编辑的文件（只把「选中」挪过去）；
 *   ② **点「使用」才真的切**（走调用方的 `onUse`，它复用预设页那颗「使用」的语义）；
 *   ③ **本机没有的机型 / 版本整组不出现**（`rows` 由调用方从本机扫描结果摊出来）。
 *
 * 为什么"点行不做"要占一条：这个抽屉在三页共用，而三页各有各的"当前"（参数页是编辑目标、
 * 校准页是基准组合、首页是浏览选择）—— 点行就切会让"看一眼"变成"动别人的状态"。
 * 选中与使用分开之后，三页的 onUse 各自接自己那条现成的路，抽屉不掺和。
 */

import { useEffect, useRef } from 'react'
import type { DrawerWidth } from '../shared/useDrawerWidth'
import s from './PresetPickerDrawer.module.css'

/** 抽屉里的一行 = **本机真有的一份预设文件** */
export interface PresetPickerRow {
  /** 归属机型（分组用） */
  machineId: string
  /** 机型显示名（分组标题） */
  machineDisplay: string
  /** 归属版本（与机型一起说明"这一份属于哪一版"） */
  versionId: string
  /** 版本显示名 —— 只用于悬停说明，**不再是行上的主文字**（2026-10-14 起行上是文件名） */
  versionName: string
  /** 这一份在盘上的文件名 —— **它就是行上的主文字**（2026-10-14 起） */
  fileName: string
  /**
   * 行键：`机型/版本/文件名`。
   *
   * 抽屉拿它与调用方给的"在用那一份"对比打勾。**必须含文件名**：
   * 一个组合下可能有好几份（下载来的一份 + 你复制出来的一份），
   * 只按 `机型/版本` 比会把两行一起打上（2026-10-14 作者截图那个 bug）。
   */
  key: string
  /**
   * 这一行的**动作身份**：用户线给路径、云端线给文件名 —— 与调用方 `onUse` 的入参同形。
   * （本机实有既有下载来的云端那份、也有用户自己那份，两者动作不同：一个「使用」、
   * 一个「使用」+ 可能带路径。）
   */
  ref: string
  /** 这一行是不是用户自己那份（用户线） */
  isLocal: boolean
}

interface Props {
  open: boolean
  /** 本机真有的预设（按机型 → 版本摊平；没本机文件的机型不进来） */
  rows: PresetPickerRow[]
  /**
   * 当前**在用**那一份的行键 —— 与 `PresetPickerRow.key` 同源（`机型/版本/文件名`）。
   *
   * ★★ 2026-10-14 修：原来是 `机型/版本`。一个组合里可以有好几份文件
   *   （下载来的一份 + 你复制出来的一份），按组合判 ⇒ **两行一起打勾**
   *   （作者截图：两份都显示 ✓ / 「使用」），而"当前用的是哪一份"在界面上
   *   本该唯一。改用行键（含文件名），一行也对不上第二行。
   */
  currentKey: string | null
  /**
   * 点「使用」—— 正式切换到这一份。**只有它会切**（点行不切，见文件头三条规则）。
   * 调用方接自己那条现成的路（参数页 `requestCombo`、校准页 `pickPreset`、首页 `pickPreset`）。
   */
  onUse: (row: PresetPickerRow) => void
  onClose: () => void
  /**
   * 从哪一缘出。默认右（参数页）；首页传 'left' —— 右边常驻露出卡，抽屉没地方，
   * 挂左缘、缝换到右边（与试验台同款）。
   */
  side?: 'left' | 'right'
  /** 拖宽那套（`shared/useDrawerWidth` 的返回值）—— 全仓抽屉同一套手感，见文件尾注释 */
  drag: DrawerWidth
}

export default function PresetPickerDrawer({
  open,
  rows,
  currentKey,
  onUse,
  onClose,
  side = 'right',
  drag,
}: Props) {
  const panelRef = useRef<HTMLDivElement>(null)

  /* Esc 关闭。焦点回到 pill 由调用方在 onClose 里做（它才持有 pill 的 ref） */
  useEffect(() => {
    if (!open) return
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  /* 打开时把焦点移进抽屉，键盘用户才能直接上下翻 */
  useEffect(() => {
    if (!open) return
    panelRef.current?.querySelector<HTMLButtonElement>('button[data-current="true"], button')?.focus()
  }, [open])

  if (!open) return null

  /* 按机型分组（保持 rows 给的顺序 —— 调用方已按机型显示名 + 版本排好）。
     ★ 2026-10-17：**不要按"使用次数"重排这里**（作者：「不要动这些列表的」）——
     学习账只用来决定"默认展示哪一份"，不参与列表排序。 */
  const groups: { id: string; display: string; items: PresetPickerRow[] }[] = []
  for (const row of rows) {
    const last = groups[groups.length - 1]
    if (last !== undefined && last.id === row.machineId) last.items.push(row)
    else groups.push({ id: row.machineId, display: row.machineDisplay, items: [row] })
  }

  return (
    <>
      {/* 遮罩：点一下关掉。它在抽屉之下（z-index 见 css） */}
      <button type="button" className={s.scrim} aria-label="关闭预设列表" onClick={onClose} />

      <div
        className={s.drawer}
        data-side={side}
        data-dragging={drag.dragging}
        ref={panelRef}
        role="dialog"
        aria-label="选择预设"
        style={{ width: `${drag.width}px` }}
      >
        {/* 拖宽热区：贴边一条透明带（全仓抽屉同一套，见 `shared/useDrawerWidth`） */}
        <div className={s.grip} data-side={side} {...drag.gripProps} />

        <div className={s.head}>
          <span className={s.title}>选择预设</span>
          <button type="button" className={s.close} onClick={onClose}>关闭</button>
        </div>

        <div className={s.list}>
          {groups.length === 0 && (
            /* 本机一份都没有：说清楚"去哪儿弄一份"，不摆假选项 */
            <p className={s.empty}>
              这台机器上还没有预设文件 —— 去「预设」页下载一份，或把自己那份放进
              <code>presets-local/</code>。
            </p>
          )}

          {groups.map((g) => (
            <div key={g.id} className={s.group}>
              {/*
               * ★★ 2026-10-14（作者）：分组标题**只写给人看的那个名字**，不写内部 ID。
               *   原来右边还跟一枚等宽小字（`A1_MINI`）—— 那是我们的标识，用户认不出、
               *   也不需要认（作者原话：「那种我们给他定义的那个ID呀，还是什么就不用显示了」）。
               */}
              <div className={s.groupHead}>{g.display}</div>

              {g.items.map((row) => {
                const current = currentKey !== null && row.key === currentKey
                /*
                 * ★★ 2026-10-14（作者）：**行上主文字就是文件名**。
                 *
                 * 原来主文字写版本显示名（「快拆版260628」）、右边一枚等宽小字写文件名 ——
                 * 于是同一个版本下有几份文件时，几行**主文字一模一样**（作者：「他们两个
                 * 显示的那个名字都一样，我不是很喜欢」），得去读右边那截小字才分得清，
                 * 而那一截窄得装不下（「那个宽度不够」）。
                 *
                 * 现在：**文件名当主文字**（它本来就是"哪一份"的唯一答案，也是用户自己在
                 * 盘上看见的那个名字），版本名不再另占一格。两个问题一起消掉：
                 *   ① 名字不再重样（文件名在盘上就不允许重名）；
                 *   ② 窄的时候先保住的是最关键的那个名字。
                 * 用户自己导入的 / 没血统的那份也一样 —— 它照旧只有文件名，不需要额外兜底。
                 */
                return (
                  <div
                    key={row.key}
                    className={s.item}
                    data-current={current}
                    /*
                     * ★ 点整行**什么都不做**（规则①）：这里不是 button，也没有 onClick ——
                     *   选中态只由调用方给的 `currentKey` 决定。要切就走右边那颗「使用」。
                     */
                    title={`${row.fileName}（${row.machineDisplay} · ${row.versionName}）`}
                  >
                    {/*
                     * ★★ 2026-10-14（作者）：在用的那一份**不摆按钮**，与预设页本地表同形 ——
                     *   那一行写灰字「在用」（不是可点的「使用」）。
                     *   原来这一档也摆一颗「使用」，含义重复（它已经在用了，再点一下没意义），
                     *   而且与预设页两颗的样子不一致。
                     */}
                    <span className={s.name}>{row.fileName}</span>
                    {current ? (
                      <span className={s.inUse}>在用</span>
                    ) : (
                      <button
                        type="button"
                        className={s.use}
                        /* 规则②：只有它会切 */
                        onClick={() => onUse(row)}
                      >
                        使用
                      </button>
                    )}
                  </div>
                )
              })}
            </div>
          ))}
        </div>
      </div>
    </>
  )
}
