/*
 * 钩子那一趟的那一屏 —— 切片器导出 G-code 时，**常驻的那扇窗**上弹出来的就是它。
 *
 * 形状照进度窗一直以来的样子（窗口常驻、结果留到下一次顶掉它），所以：
 *
 *   1. **跑着的时候关不掉**：没有 ×、点遮罩也不关 —— 那一趟还在跑，藏起来只会让人以为
 *      出事了。要收起来只有一条路：按「后台运行」（那是明确说过的意思，而且收起来之后
 *      由标题栏那枚小胶囊兜着，见 `PostProcessChip.tsx`）。
 *   2. **窗口留着、下次复用**：钩子进程跑完就退（切片器一秒都不多等），但这一屏不自动关
 *      —— 用户关掉它之前，结论一直在（下一次切片由新一趟的开场顶掉）。
 *   3. **机型不匹配那一问等用户**：两颗按钮就是「继续跑 / 停下」，**没有默认** ——
 *      替用户决定"不匹配也照跑"是这一层最不该做的事（没答 / 超时 = 不跑，见 Rust 侧）。
 *   4. **跑完说清哪一档**：成功 / 已取消 / 失败长得不一样 —— 取消不是失败，
 *      把用户自己按的停止说成"处理失败"是一种冤枉。
 *
 * # 2026-10-12：这一屏换新版（`lab/mkp-postprocess` 58 轮定稿搬进来）
 *
 * 作者在试验场里把六扇窗逐像素定了稿（见 `lab/mkp-postprocess/HANDOVER.md`），
 * 这一段就是那份定稿的落地：**结构、配色、几何全照搬，一个像素不改**。
 *
 *   ① 跑着：绿 hero（大数左 / 步骤 + 倒计时右 / 凹槽进度条嵌底）+ 两组带题 facts，
 *      页脚「停止 / 后台运行」；
 *   ② 机型不匹配：红警告 hero（空心圈 + 标题 + 后端那一句问话）+ 两组带题 facts，
 *      页脚「强制继续 / 终止」—— 强调位给「终止」（第 9~11 轮定的）；
 *   ③ 成功：四格指标条（笔偏移 X/Y/Z + 涂胶速度）+ 两组带题 facts，
 *      页脚「关闭 / → 查看详情」；
 *   ④ 已停止：柔和内影那一版（`.smBase .smSunk`，浅色深色同一张卡），圈里画圆角方块；
 *   ⑤ 失败：照「机型不匹配」那一张红卡（`.failWarn`）—— 图标在板外、文案直接落在红卡上；
 *   ⑥ 结果那几扇也**没有 ×**（收窗走页脚「关闭」，与抽卡页签一致）。
 *
 * 数据口径：**真机给什么就摆什么，没给的不摆**（不编）——
 * 读数由内核在成功那一条里报上来（`PostProcessResult`，Rust 侧 `hook::result_facts`）；
 * 「预估打印时间」是**后补的**（有界面时那一步延后算，`postprocess-print-time` 到了才亮），
 * 所以成功那一刻它常常先空着；跑着 / 问句那两扇只有开场事实里有的那几格。
 */

import { useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent, ReactNode } from 'react'


import { api, errorText } from '../../api'
import type { PostProcessFacts, PostProcessFinished, PostProcessResult } from '../../api/contract'
import { detectPlatform } from '../../hooks/usePlatform'
import { Btn } from '../ui/Controls'
import { HeadNote, Modal, summarizeNotes } from '../../components/modal'
import { useTitlebarDrag } from '../useTitlebarDrag'
import { questionUiState } from './questionUi'
import { percentOf, stepName } from './steps'
import { SETTLE_MS, useSmoothPercent } from './useSmoothPercent'
import type { PostProcessRunView } from './usePostProcessRun'
import { RelocateMaps, RelocateNote } from './RelocatePanel'
import s from './PostProcessModal.module.css'

/* 顶栏那一条的高度按平台来（与 `TopTabs` 同一对 token）：拖窗带要正好铺在它上面 */
const PLATFORM = detectPlatform()

/**
 * 这一屏"整段演示"的最短时长（ms）—— **从窗口打开算**（`view.sinceMs`）。
 * 作者 2026-10-14 先要「起码 3 秒」，随即收回：「不用限制 3 秒，限制 2 秒或者 1.5，
 * 反正是要动画自然均匀」——**均匀比总时长重要**，这一条只是个下限兜底，
 * 不必为了凑时长把快跑拖成慢跑。1.5 秒 ≈ 观感上"看清了它走过去"。
 */
const MIN_SHOW_MS = 1500
/** 等条子走完的兜底（ms）：条子万一没报上来（`onSettled` 丢了），到点也得切，不许卡住 */
const SETTLE_MAX_MS = 8000

/**
 * 收尾那一下的"渐隐"时长（ms）—— 主件那个 WAAPI effect 用它（`el.animate(..., { duration: LEAD_FADE_MS })`）。
 *
 * 作者 2026-10-14：「感觉 100% 的时候消失得太突然了，没有渐隐渐现」——
 * 收尾窗口到点不再"啪"地换：hero 先淡出（这么久），淡完才真换结果态；
 * 结果块自己的淡入由 `.leadIn`（220ms）负责。
 */
const LEAD_FADE_MS = 160

export interface PostProcessModalProps {
  /** 那一趟的现况（`usePostProcessRun` 给的那一份；App 拿着它，后台那条窄条也要） */
  view: PostProcessRunView
  /** 「查看详情」：跨页跳转的出口由外壳给（页面 / 浮层不该伸手改页签） */
  onOpenReport: () => void
}

/* ---- 小工具 ---- */

/**
 * 双击的判定门槛。**不用系统的 `dblclick` 事件** —— 那个跟系统设置的"双击速度"走，
 * 有人调得慢一点，连击就被吞掉（作者 2026-10-15：「有时候连击感觉切换不了，会吞掉双击的」）。
 * 这里自己判：两次 `pointerup` 间隔 < `DOUBLE_TAP_MS`、且落点相距 < `DOUBLE_TAP_SLOP` 就算一次双击。
 */
const DOUBLE_TAP_MS = 600
const DOUBLE_TAP_SLOP = 24

/**
 * "弹上来"那套关键帧（先陷下去 → 弹过头一点 → 落位）—— 作者 2026-10-15：
 * 「我双击切换的时候，那个新的应该也弹上来的感觉」→「没有被按压下去跳动上来的」。
 * 与 `usePressFlash` 一个路子：**JS 驱动**（`element.animate()`），不走 CSS 级联。
 */
const POP_KEYS: Keyframe[] = [
  { transform: 'translateY(16px) scale(0.94)' },
  { transform: 'translateY(-3px) scale(1.018)', offset: 0.55 },
  { transform: 'translateY(0) scale(1)' },
]
/** 收尾切换的淡入（作者要的"渐隐渐现"） */
const FADE_KEYS: Keyframe[] = [{ opacity: 0 }, { opacity: 1 }]

/**
 * 按一下的交互（作者三轮说到位：「点击或者连击也有按压效果」+「按下不松手不抬起的都需要」
 * +「连击……会吞掉双击的」）。
 *
 * ★★ **全部用 JS 驱动（Web Animations API），不走 CSS `animation`/`transition`。**
 *
 * 为什么（2026-10-15 血的教训）：这一套先用 CSS 实现，作者真机（Tauri 的 WKWebView）上
 * **一个都看不见** —— 拆视频帧用像素差量过，切换是"一帧内跳变"（无任何渐变）；
 * 而同日排查确认：dev server 上的代码是新的、生产构建的 CSS 里 `@keyframes` 与
 * `animation:...!important` 都在、`tauri dev` 也重启过。诊断指向"这台机器上
 * CSS 动画这条管线就是不播"（`global.css` 里那条 `prefers-reduced-motion` 全站降级是头号嫌疑，
 * 哪怕系统开关看着是关的，WKWebView 自己读到的未必一样）。
 * 换 `element.animate()`：它是 JS 调用，**不过 CSS 级联**，那条降级规则管不到它。
 *
 * 三件事：
 *   ① **按住中**：`pointerdown` 起一个 130ms 的 WAAPI 动画压到 scale 0.972 + 暗 0.97，
 *      `fill: 'forwards'` 保持住；松手（`pointerup` / `cancel` / 离开）再起一个回到原状的。
 *   ② **弹一下**：`onClick` 起一个三段动画（1 → 0.975 → 1）。连击每次都重放（先 cancel 旧的）。
 *   ③ **双击**：自己数（`DOUBLE_TAP_MS` / `DOUBLE_TAP_SLOP`），`pointerup` 里判。
 *
 * 纯观感：不碰任何状态、不拦点击。
 */
function usePressFlash<T extends HTMLElement>(onDoubleTap?: () => void) {
  const ref = useRef<T>(null)
  /* 回调用 ref 承接：事件处理器本身随渲染更新，但"上一次落点"要跨渲染留着 */
  const doubleTapRef = useRef(onDoubleTap)
  doubleTapRef.current = onDoubleTap
  const lastUpRef = useRef({ at: 0, x: 0, y: 0 })
  /** 进行中的"按住"动画（松手时要把它 cancel 掉，否则 forwards 会一直压着） */
  const holdRef = useRef<Animation | null>(null)

  const animate = (keyframes: Keyframe[], options: KeyframeAnimationOptions): Animation | null => {
    const el = ref.current
    /* 老引擎没有 WAAPI 就什么都不做（宁可没动效，也不能抛错挡了点击） */
    if (el === null || typeof el.animate !== 'function') return null
    return el.animate(keyframes, options)
  }

  const onPointerDown = () => {
    holdRef.current?.cancel()
    holdRef.current = animate(
      [
        { transform: 'scale(1)', filter: 'none' },
        { transform: 'scale(0.972)', filter: 'brightness(0.97)' },
      ],
      { duration: 130, easing: 'ease-out', fill: 'forwards' },
    )
  }
  /** 结束"按住中"（松手 / 取消 / 指针跑出去）—— 三个事件共用 */
  const endPress = () => {
    if (holdRef.current === null) return
    holdRef.current.cancel()
    holdRef.current = null
    animate(
      [
        { transform: 'scale(0.972)', filter: 'brightness(0.97)' },
        { transform: 'scale(1)', filter: 'none' },
      ],
      { duration: 110, easing: 'ease-out' },
    )
  }
  const onPointerUp = (e: ReactPointerEvent<HTMLElement>) => {
    endPress()
    if (doubleTapRef.current === undefined) return
    const now = performance.now()
    const last = lastUpRef.current
    const near = Math.hypot(e.clientX - last.x, e.clientY - last.y) < DOUBLE_TAP_SLOP
    if (now - last.at < DOUBLE_TAP_MS && near) {
      /* 用掉这一次：三击不会再触发第二次双击 */
      lastUpRef.current = { at: 0, x: 0, y: 0 }
      doubleTapRef.current()
      return
    }
    lastUpRef.current = { at: now, x: e.clientX, y: e.clientY }
  }
  const onPointerCancel = endPress
  const onPointerLeave = endPress
  /** 弹一下：挂在 `onClick` 上（不是 pointerdown —— 那样按着不放会先弹一次，重复） */
  const onClick = () => {
    animate(
      [
        { transform: 'scale(1)', filter: 'none' },
        { transform: 'scale(0.975)', filter: 'brightness(0.97)', offset: 0.35 },
        { transform: 'scale(1)', filter: 'none' },
      ],
      { duration: 190, easing: 'ease' },
    )
  }
  return { ref, onPointerDown, onPointerUp, onPointerCancel, onPointerLeave, onClick }
}

/** 文件大小：MB 两位小数（与报告页那份口径一致） */
function sizeText(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/** 毫米这两位：内核里 0 = 预设没写这一格 ⇒ 摆 `—`，不摆 `0.00` */
function mmText(v: number): string {
  return v > 0 ? v.toFixed(2) : '—'
}

/** 毫米（两位小数）：笔偏移 / 涂胶总长那几格 */
function mm2(v: number): string {
  return `${v.toFixed(2)} mm`
}

/** 涂胶速度（mm/s 两位小数）—— 换算在 Rust 侧做过一次（`archive::glue_speed_of`） */
function speedText(v: number): string {
  return `${v.toFixed(2)} mm/s`
}

/**
 * 预估打印时间 —— **与报告页同一个讲法**（`PageReport` 的 `printTimeText`）：
 * 同一个数在这两处不许出现两种说法（那边读归档里的秒数，这边读事件里的秒数）。
 */
function printTimeText(sec: number): string {
  if (!Number.isFinite(sec)) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.round((sec % 3600) / 60)
  return h > 0 ? `${h} 小时 ${m} 分` : `${m} 分`
}

/**
 * 「预计还需」：拿**已经跑了多久**按全局完成度外推。
 *
 * 起点只有"这一趟的开场事件被这一侧看见过"时才准（窗口开在半路 ⇒ 没有起点 ⇒ 不摆这一格，
 * 编一个比空着糟）；完成度太低时外推值会离谱（1% 时乘以 99），所以低于 5% 也不摆。
 */
function etaText(sinceMs: number | null, percent: number): string | null {
  if (sinceMs === null || percent < 5) return null
  const left = ((Date.now() - sinceMs) * (100 - percent)) / percent
  if (!Number.isFinite(left) || left <= 0) return null
  const sec = Math.round(left / 1000)
  if (sec < 60) return `预计还需 ${sec} 秒`
  const m = Math.floor(sec / 60)
  return `预计还需 ${m} 分 ${sec - m * 60} 秒`
}

/* ---- 两组带题 facts（定稿的读数区：题名行 = 色点 + 题 + 发丝线） ---- */

interface FactRow {
  label: string
  value: string
  /** 灰字行（切片软件这类「看一眼就行」的读数） */
  grey?: boolean
}

interface FactGroup {
  title: string
  rows: FactRow[]
}

/** 左栏「机型与切片基础」—— 六态共用（这一栏几格真机全有） */
function baseGroup(facts: PostProcessFacts | null): FactGroup {
  return {
    title: '机型与切片基础',
    rows: [
      { label: 'G-code 机型', value: facts?.gcodeMachine ?? '—' },
      { label: '预设机型', value: facts?.presetMachine ?? '—' },
      {
        /* 喷嘴那一格：`0` 是"没有值"的哨兵（内核缺值一律 0）—— 别把它当 0.00 mm 摆出去
           （作者 2026-10-14：「它怎么就不显示那一个喷嘴层高」）。真值现在由内核
           从输入头 `; nozzle_diameter = ` 认（`pass1::adopt_nozzle_and_layer_height`）。 */
        label: '耗材 / 喷嘴',
        value:
          facts === null
            ? '—'
            : `${facts.filamentType} · ${
                facts.nozzleDiameter > 0 ? `${mmText(facts.nozzleDiameter)} mm` : '—'
              }`,
      },
      { label: '层高', value: facts !== null && facts.layerHeight > 0 ? `${mmText(facts.layerHeight)} mm` : '—' },
    ],
  }
}

/**
 * 跑着 / 问句：右栏是「切片与预估」。
 *
 * 「预估打印时间」这一刻给的是**切片器自报的**（输入头 `; total estimated time` 那一行）——
 * 内核重算的那个要跑完才有（而且有界面时还延后）；涂胶速度 / 切片软件开跑就确定。
 * 缺哪格不摆哪格。
 */
function runGroup(facts: PostProcessFacts | null): FactGroup {
  const rows: FactRow[] = [
    {
      label: '预估打印时间',
      value: facts?.slicerPrintTimeSeconds === undefined ? '—' : printTimeText(facts.slicerPrintTimeSeconds),
    },
    { label: '文件大小', value: facts === null ? '—' : sizeText(facts.fileSizeBytes) },
    { label: '涂胶速度', value: facts?.glueSpeed === undefined ? '—' : speedText(facts.glueSpeed) },
    { label: '切片软件', value: facts?.slicer ?? '—', grey: true },
  ]
  return { title: '切片与预估', rows }
}

/** 跑完：右栏是「擦料与胶水消耗」——结果读数有哪几格就摆哪几格 */
function doneGroup(finished: PostProcessFinished, facts: PostProcessFacts | null): FactGroup {
  const res = finished.result
  const rows: FactRow[] = []
  /* 打印时间**常常是后到的**（有界面时延后算）：先空着，`postprocess-print-time` 到了再补；
   * 失败 / 取消时退回开场那一格（切片器自报的）—— 有就摆，比空着强 */
  const printSec = res?.printTimeSeconds ?? facts?.slicerPrintTimeSeconds
  rows.push({
    label: '预估打印时间',
    value: printSec === undefined ? '—' : printTimeText(printSec),
  })
  rows.push({
    label: '涂胶总长',
    value: res?.glueLengthMm === undefined ? '—' : mm2(res.glueLengthMm),
  })
  /* 这一格真机现在就有：处理耗时 = finished.elapsedMs，大小 = 开场那几个事实里的字节数 */
  rows.push({
    label: '处理耗时 / 大小',
    value: `${(finished.elapsedMs / 1000).toFixed(2)} 秒 / ${
      facts === null ? '—' : sizeText(facts.fileSizeBytes)
    }`,
  })
  /* 切片软件：跑完给带版本的完整串；失败 / 取消时退回开场那个名字（总比空着强） */
  const slicer = res?.slicer ?? facts?.slicer
  rows.push({ label: '切片软件', value: slicer ?? '—', grey: true })
  return { title: '擦料与胶水消耗', rows }
}

/**
 * 这一屏的**提示**一律走共用件 [`HeadNote`]（`components/modal`）—— 落点是标题右边
 * 那一格（回执槽），不是框身。
 *
 * 作者 2026-10-17 三次点名之后收的口径：
 *   · 「这种错误一行的能不能放到标题右侧」
 *   · 「这个文字提示不应该加在这个地方，应该加在标题后面……会导致我的模态框高度宽度变化」
 *   · 「把所有这些提示的，不管是什么的，都用同一套组件，同一套逻辑复用在标题后面显示」
 *
 * 原来这一屏自己抄了一份 `HeadNote` + `.headErr/.headNote/.headOk` 三档样式，
 * 现在那三档住共用件里（`components/modal/HeadNote.module.css`）—— 别在这里再抄一遍。
 *
 * ★ 必须**在 `Modal` 里面渲染**：槽是 `Modal` 自己给的（`modalHeadSlot.ts`），
 * 在外面拿到的永远是 `null`。
 */

/**
 * 内核那几条警告压成**一行**（标题右边那一格只放得下一行）。
 *
 * 多条那半截的写法归共用件（`summarizeNotes`）；这里只加"警告："这层帽子 ——
 * 因为报告页那些"读不出来"之类的提示也是同一格，帽子里得说清是哪一类事。
 */
function warningsLine(list: string[]): string {
  const one = summarizeNotes(list)
  return list.length === 1 ? `警告：${one}` : `警告 ${one}`
}

function FactsGroups({ groups }: { groups: FactGroup[] }) {
  return (
    <div className={s.groups}>
      {groups.map((g) => (
        <div className={s.group} key={g.title}>
          <div className={s.groupHead}>
            <span className={s.groupDot} aria-hidden="true" />
            <span className={s.groupTitle}>{g.title}</span>
          </div>
          <div className={s.facts}>
            {g.rows.map((r) => (
              <div className={r.grey === true ? `${s.factRow} ${s.factGrey}` : s.factRow} key={r.label}>
                <span className={s.factLabel}>{r.label}</span>
                <span className={s.factValue}>{r.value}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

/* ---- ① 跑着：绿 hero（大数左 / 步骤 + 倒计时右 / 凹槽条嵌底） ---- */

function ProgressHero({
  percent,
  step,
  stepId,
  sizeBytes,
  eta,
  done,
  onSettled,
  onDoubleClick,
}: {
  /** 事件给的**真实**百分比（不是显示值） */
  percent: number
  /** 给人看的中文步骤名 */
  step: string
  /** 稳定步骤 id（跨步衔接用：上一步的整步节奏是下一步的先验，见 hook 头注释） */
  stepId: string | null
  /** G-code 字节数 —— 跨运行学习按 MB 归一要用（见 hook 头注释） */
  sizeBytes: number | null
  eta: string | null
  /** 跑完没有 —— 置位后条子把最后几个点走完再交给结果态（见 `SETTLE_MS`） */
  done: boolean
  /** 条子真的走到 100 了（收尾窗口据此切结果态，不再用固定时长去猜） */
  onSettled: () => void
  /** 双击这一块（只有"翻过来看 100%"那一态才接 —— 用来翻回结果态） */
  onDoubleClick?: () => void
}) {
  /*
   * ★ 显示值由 rAF **命令式**直写 DOM（不经过 React）：
   *   - 每帧 setState 会被 React 调度器合并/延后 ⇒ 真机上数字与条子一跳一跳（作者录屏）；
   *   - 这里只在 rAF 里改 textContent 与 style.width 两个属性，零 diff、零重渲。
   * JSX 里**故意**留空内容与 0 宽：React 重渲（倒计时每秒 tick）后由 hook 的
   * 无依赖 effect 把当前值补写回去。
   */
  const { numRef, fillRef } = useSmoothPercent(percent, done, stepId, sizeBytes, onSettled)
  /*
   * 按一下弹一下（等待时解压用）+ 双击翻回结果 —— 见 `usePressFlash`。
   * **双击是自己判的**（不用系统 `dblclick`：那个跟系统"双击速度"走，连击会被吞）。
   */
  const press = usePressFlash<HTMLDivElement>(onDoubleClick)
  return (
    /*
     * 不给 role="status"：这一块每秒都在动（百分比 / 倒计时），朗读器会被念个不停。
     * 交互（2026-10-14 作者）：**单击 / 连击回弹**（解压用，纯观感）；
     * **双击**只在"翻过来看 100%"那一态接（用来翻回结果态），跑着时不接。
     */
    <div
      ref={press.ref}
      className={`${s.hero} ${s.heroSide}`}
      onPointerDown={press.onPointerDown}
      onPointerUp={press.onPointerUp}
      onPointerCancel={press.onPointerCancel}
      onPointerLeave={press.onPointerLeave}
      onClick={press.onClick}
      title={onDoubleClick === undefined ? undefined : '双击：回到结果'}
    >
      <span className={s.heroNum} ref={numRef} />
      <div className={s.heroMeta}>
        <span className={s.heroStep}>{step}</span>
        <span className={s.heroEta}>{done ? '正在收尾…' : (eta ?? '正在估算…')}</span>
      </div>
      {/* 进度条是定稿皮（pb1「凹槽条」：绿底上凿出来的一道槽），不是普通白条 */}
      <div className={`${s.heroBar} ${s.heroBarGroove}`}>
        <span className={s.heroBarFill} ref={fillRef} style={{ width: '0%' }} />
      </div>
    </div>
  )
}

/* ---- ② 机型不匹配：红警告 hero（空心圈 + 标题 + 后端那一句问话） ---- */

/**
 * 「1 分 52 秒后自动终止」——没人答就自动按"不跑"收场。
 *
 * 作者 2026-10-17：「文案里面出现自动停止的倒计时」。后端给的是**绝对时刻**
 * （`question.deadlineUnixMs`），所以补推一次也不会从头数。
 * 每秒重算一次靠上面那条 `running` 的 interval（`setTick`）——这一屏跑着的时候本来就每秒重渲。
 */
/**
 * 校准那趟的**那一句结论**（`print` / `direct` 两种跑法各一句）。
 *
 * ★ 与 Rust 侧 `hook::calib_done_line` 读的是**同一组键**（`messages.toml` 的
 * `hook.calib_done_print` / `hook.calib_done_direct`）—— 报告页那一侧也读它，
 * 于是界面与报告不会各说各的。这里这份是它的前端镜像：那一行住在标题右边
 * （`HeadNote`），要的是"立刻能画"的短句，不再回一趟后端。
 *
 * ★★ 两边**必须逐字一致**；改文案时 `messages.toml` 与这里一起改。
 */
function calibDoneLine(choice: 'print' | 'direct'): string {
  return choice === 'direct'
    ? '已去掉打印件，只保留校准路径 —— 处理完成'
    : '打印件保留，只加了涂胶与擦料 —— 处理完成'
}

function countdownText(deadlineUnixMs: number | undefined): string | null {
  /* 老一侧（没这个字段）/ 预览那一档（0）= 不摆倒计时，不许摆出个 `NaN 秒` */
  if (deadlineUnixMs === undefined || !Number.isFinite(deadlineUnixMs) || deadlineUnixMs <= 0) {
    return null
  }
  const left = deadlineUnixMs - Date.now()
  if (!Number.isFinite(left)) return null
  if (left <= 0) return '正在自动终止…'
  const sec = Math.ceil(left / 1000)
  if (sec < 60) return `${sec} 秒后自动终止`
  const m = Math.floor(sec / 60)
  return `${m} 分 ${sec - m * 60} 秒后自动终止`
}

function WarnHero({ text, deadlineUnixMs }: { text: string; deadlineUnixMs?: number }) {
  const left = countdownText(deadlineUnixMs)
  return (
    <div className={`${s.warnHero} ${s.warnSoft}`} role="status">
      <span className={s.warnGlyph} aria-hidden="true">
        <span className={s.warnBadge}>!</span>
      </span>
      <div className={s.warnCopy}>
        {/*
          这一叠（标题 + 问话 + 倒计时）在框内滚：hero 与其它三扇一样**定死 104px**
          （作者 2026-10-17：「内容在里面滚动」），内容多的时候不许把框撑高。

          ★ 文案只留三行（作者：「你就用很简单的文案，简短几行的」）：
            标题（是什么事）+ 后端那一句（哪两台机对不上）+ 倒计时（不管会怎样）。
          原来那两句"维度按预设算 / 怎么答"是内核口径，作者原话：「文案不好，什么纬度？」
        */}
        <div className={s.warnScroll}>
          <span className={s.warnTitle}>预设与机型不匹配 · 等你决定</span>
          {/* 正文就是后端那一句问话（`question.text`）—— 不另编文案 */}
          <span className={s.warnLine}>{text}</span>
          {left !== null && <span className={s.warnHint}>{left}</span>}
        </div>
      </div>
    </div>
  )
}

/**
 * **那一问的 hero**（2026-10-17 泛化）：按 `questionKind` 分两种长相，**同一口 104px 的框、
 * 同一套 `.warnScroll` 内滚**（作者：「hero 高度定死，内容在里面滚动」）。
 *
 * - `machine-mismatch` = 警告橙（`.warnSoft`）：这是"结果可能出错"，要拦一下；
 * - `calibration-mode` = 中性绿（`.askSoft`）：**不是出错**，是"这一盘怎么跑"的流程选择，
 *   拿警告色去说它会让用户以为出了问题（作者：「这不是警告」）。
 *
 * ★ 种类串必须与后端常量**逐字一致**（`hook_ipc::QKIND_*`）。这里原来写的是
 * `calibration-print`，而后端发的是 `calibration-mode` —— 对不上就落到兜底那条，
 * 校准那一问被套上"预设与机型不匹配 · 等你决定"的橙框（作者 2026-10-17 截图）。
 *
 * 两种都只摆三行：标题 + 后端那一句 + 倒计时。文案不在这里编（正文就是 `question.text`）。
 */
function AskHero({
  questionKind,
  text,
  deadlineUnixMs,
}: {
  questionKind: string
  text: string
  deadlineUnixMs?: number
}) {
  if (questionKind === 'calibration-mode') {
    const left = countdownText(deadlineUnixMs)
    return (
      <div className={`${s.warnHero} ${s.askSoft}`} role="status">
        <span className={s.warnGlyph} aria-hidden="true">
          {/* 圈里那枚记号是**画出来的 SVG**，不是文字符号 —— 与 `.resBadge` 那条同一条理由：
              字体里的 `✦` 自带基线偏移（`align-items: center` 居中的是行盒、不是字形），
              看上去会偏低。SVG 占满整个盒子，几何上就是正的（作者 2026-10-17 截图）。 */}
          <span className={`${s.warnBadge} ${s.askBadge}`}>
            <SparkMark />
          </span>
        </span>
        <div className={s.warnCopy}>
          <div className={s.warnScroll}>
            <span className={s.warnTitle}>这是校准件 · 选怎么跑</span>
            <span className={s.warnLine}>{text}</span>
            {left !== null && <span className={s.warnHint}>{left}</span>}
          </div>
        </div>
      </div>
    )
  }
  return <WarnHero text={text} deadlineUnixMs={deadlineUnixMs} />
}

/* ---- ③ 成功：四格指标条（笔偏移 X/Y/Z + 涂胶速度） ---- */

/**
 * 四格齐（三轴偏移 + 涂胶速度）才摆 —— 缺格的时候这条空着比缺一角好。
 *
 * ★★ 第一道闸是 `res == null`，**不是** `res !== undefined`（2026-10-17 实测踩到）：
 *   类型上这一格写的是 `PostProcessResult | undefined`，但**线上真的会出现 `null`**
 *   （后端 `Option` 序列化出来就是 `null`，不存在的格一律 `null` 而不是省略键）。
 *   `null !== undefined` 为真 ⇒ 往下读 `res.penX` 当场抛
 *   `TypeError: Cannot read properties of null (reading 'penX')`，
 *   那一屏整块白掉（React 没有边界兜它）。
 *
 *   `== null` 同时盖住 `null` 与 `undefined`，正好是这里要的两种"没有读数"。
 */
function metricsReady(
  res: PostProcessResult | null | undefined,
): res is Required<Pick<PostProcessResult, 'penX' | 'penY' | 'penZ' | 'glueSpeed'>> {
  return (
    res != null &&
    typeof res.penX === 'number' &&
    typeof res.penY === 'number' &&
    typeof res.penZ === 'number' &&
    typeof res.glueSpeed === 'number'
  )
}

function MetricsStrip({
  res,
  onDoubleTap,
}: {
  res: Required<Pick<PostProcessResult, 'penX' | 'penY' | 'penZ' | 'glueSpeed'>>
  /** 双击这一条：翻回 100% 那个 hero（"看一眼刚才走过去的那根条"） */
  onDoubleTap: () => void
}) {
  const cells: Array<[string, string, boolean?]> = [
    ['笔偏移 X', mm2(res.penX)],
    ['笔偏移 Y', mm2(res.penY)],
    ['笔偏移 Z', mm2(res.penZ)],
    ['涂胶速度', speedText(res.glueSpeed), true],
  ]
  /* 按一下弹一下（等结果时也能解压）+ 双击翻回 hero —— 见 `usePressFlash` */
  const press = usePressFlash<HTMLDivElement>(onDoubleTap)
  return (
    /*
     * ⚠️ 这里**只有按压这一个 animation**：淡入（`.leadIn`）/弹上来（`.popIn`）由调用处外面包的那一层负责 ——
     * 同一个元素上挂两个 `animation` 会互相覆盖（`animation` 是同一个属性），
     * 四格当初就是这么"点了没效果"的（见 CSS 里 `.press` 那条说明）。
     */
    <div
      ref={press.ref}
      className={`${s.strip} ${s.stripFinal}`}
      role="status"
      onPointerDown={press.onPointerDown}
      onPointerUp={press.onPointerUp}
      onPointerCancel={press.onPointerCancel}
      onPointerLeave={press.onPointerLeave}
      onClick={press.onClick}
      title="双击：回到刚才的进度"
    >
      {cells.map(([label, value, hot]) => (
        <span className={s.stripCell} key={label}>
          <span className={s.stripLabel}>{label}</span>
          <span className={hot === true ? `${s.stripValue} ${s.stripHot}` : s.stripValue}>{value}</span>
        </span>
      ))}
    </div>
  )
}

/* ---- ④⑤ 结果 hero（已停止 / 失败 —— 与 ①② 同一套骨架：图标 + 文案） ---- */

/** 圈里那个记号：已停止 = 圆角方块（**画成 SVG** —— 文字符号在圈里对不齐） */
function SquareMark() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <rect x="5.6" y="5.6" width="8.8" height="8.8" rx="1.8" fill="currentcolor" />
    </svg>
  )
}

/**
 * 四角星（「这是校准件 · 选怎么跑」那一枚记号）。
 *
 * 为什么不写 `✦` 文本：字体里的星号自带基线偏移，`align-items: center` 居中的是
 * **行盒**而不是字形，作者看到的那颗星"略微偏低"。SVG 用几何坐标画，正中心就是正中心。
 *
 * 形状 = 四条内凹的边（每边一个控制点拉向圆心），比正菱形更像"闪光"，与 `✦` 同一种感觉。
 */
/**
 * 翻页那颗钮上的记号：一个 i（"信息"）—— 画成 SVG，与全站「圈里记号一律 SVG」同一条。
 */
function InfoGlyph() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" fill="none" aria-hidden="true">
      <circle cx="6" cy="6" r="4.6" stroke="currentColor" strokeWidth="1.1" />
      <path d="M6 5.4v3" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" />
      <circle cx="6" cy="3.7" r="0.62" fill="currentColor" />
    </svg>
  )
}

function SparkMark() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path
        d="M10 2.2 C10.7 6.4 13.6 9.3 17.8 10 C13.6 10.7 10.7 13.6 10 17.8 C9.3 13.6 6.4 10.7 2.2 10 C6.4 9.3 9.3 6.4 10 2.2 Z"
        fill="currentcolor"
      />
    </svg>
  )
}

/**
 * 已停止（§4 定稿 = sm4「柔和内影」）：**同一张气泡卡**，只换面 / 线 / 影，
 * 图标**进板**（板是 [圈 + 文案列] 的一行）。
 *
 * 失败（§5 定稿 = `.failWarn`）：照「机型不匹配」那一张红卡 —— 图标在板外、
 * 文案直接落在红卡上；那枚叹号是 24px 粗体**文字**（与机型不匹配那一枚逐字一样）。
 */
function ResultHero({
  cancelled,
  title,
  lines,
}: {
  cancelled: boolean
  title: string
  lines: string[]
}) {
  const text = (
    <div className={s.resText}>
      <span className={s.resTitle}>{title}</span>
      {lines.map((l) => (
        <span className={s.resLine} key={l}>
          {l}
        </span>
      ))}
    </div>
  )
  const badge = cancelled ? (
    <span className={s.resBadge} aria-hidden="true">
      <SquareMark />
    </span>
  ) : (
    <span className={s.resBadge} aria-hidden="true">
      !
    </span>
  )

  if (cancelled) {
    /* 图标进板：板里是 [圈 + 文案列] 一行 */
    return (
      <div className={`${s.resHero} ${s.resStop} ${s.smBase} ${s.smSunk}`} role="status">
        <div className={s.resCopy}>
          <span className={s.resGlyph}>{badge}</span>
          {text}
        </div>
      </div>
    )
  }
  return (
    <div className={`${s.resHero} ${s.resBad} ${s.failWarn}`} role="status">
      <span className={s.resGlyph}>{badge}</span>
      <div className={s.resCopy}>{text}</div>
    </div>
  )
}

/* ---- 主件 ---- */

export default function PostProcessModal({ view, onOpenReport }: PostProcessModalProps) {
  const { run, dismiss } = view
  const [busy, setBusy] = useState(false)
  /*
   * **已经点过答复、钩子还没把问句收掉**（2026-10-18 作者：「点击把塔挪到不撞的位置之后
   * 出来问题了……这个时候应该显示原本的 hero 进度条那些，但是现在是按钮在一闪一闪的，
   * 还能一直点击，这不对」）。
   *
   * 病根：`answer_post_process` 是**投递即返回**（把答复写进那条回写线就 Ok 了），而钩子
   * 那边要等它自己接着跑完剩下的步骤、到 `Finished` 才把 `question` 清掉 —— 中间这十几秒
   * 问句一直挂着，于是问句那一屏（含按钮）也一直挂着；而 `busy` 在 IPC 回来的那一下就被
   * 清成 false ⇒ 按钮复活 ⇒ 再点一下又闪一下。
   *
   * 所以这里记的不只是 id，还有**这一问的身份**（kind + 截止毫秒）：问句换了人（钩子又问了
   * 第二问）、或者问句被收掉，effect 才松手；这期间问句那一屏整个让位给 hero。
   */
  const [answering, setAnswering] = useState<{
    kind: string
    deadline: number
    /** 点下去那一刻进度条停在的百分比（见下面 `shownPercent`） */
    from: number
  } | null>(null)
  const [err, setErr] = useState<string | null>(null)
  /* 「预计还需」每秒重算一次（这个表只在跑着的时候挂着） */
  const [, setTick] = useState(0)
  /* 跑着的时候顶上那条拖窗带用它（与标题栏同一条动作，见文件头第 1 条下面那段） */
  const drag = useTitlebarDrag()

  /*
   * ★★ 2026-10-19：**撤掉两处开发台钩子**（作者：「把预览全部取消吧」）。
   *
   * 以前这里有 `/dev/preview.tsx` 的 `useDevPreviewNonce(...)` 两个分支：
   *
   * | 撤掉的戳 | 以前演的是什么 | 现在怎么看到 |
   * | --- | --- | --- |
   * | `postprocess-answer-stale` | 那一问还挂着、钩子已经没了时点一下，红字落在标题右边 | **真跑就能遇到**：矩阵 `collision` 档不答、等它超时后再点，红字会真出现 —— 而且现在报错那一屏有「停止并关闭」（见 `questionUi.ts`） |
   * | `postprocess-backgrounded` | 跑着的时候点「后台运行」 | 真跑起来点那颗按钮 —— 真的会收走，标题栏那枚胶囊是真的 |
   *
   * 两处都是"界面本地动作"，`dismiss` 与 `setErr` 本身都是产品行为，**没动**；
   * 撤的只是"由开发台替你点"这两条入口。`devTrigger` 收到这两个 slug 时
   * 客户端还会把 slug 推给前端（`hook_ui` 那条 `tell_ui` 分支），但没有组件再接它 ——
   * 无害，且留着能说明"这一格曾经存在过"。
   */

  const finished = run?.finished ?? null
  const running = run !== null && finished === null
  /*
   * ★★ 正在等用户作答（2026-10-17）。**要不要问，看的是"问句还在不在"**：
   * 钩子那边推完问句就在原地等，直到有人答 / 按停止 / 超时 —— 这一整段时间界面上
   * 都必须是"等你回答"那一屏，不能还挂着"准备中 6%"（作者真机看到的就是那个，
   * 然后两分钟后被静默终止）。
   */
  const asking = running && run.question !== null
  /*
   * 问句那一屏只在"还没答"的时候摆。答完到钩子收掉问句之间，这一屏让位给 hero
   * （不然按钮会一直立在那儿给人点 —— 见上面 `answering` 那段）。
   */
  /**
   * 摆不摆问句屏 / 要不要给逃生按钮 —— 判据住在 `questionUi.ts`（纯函数）。
   *
   * ★ 2026-10-19：**报错时问句屏要回来**。以前 `err` 一出就把按钮收走，
   * 于是"答晚了一步"变成死局（既没按钮也关不掉框）—— 作者真机踩到的就是这个。
   */
  const { showAsk, offerStopAndClose } = questionUiState({
    running,
    asking,
    answering: answering !== null,
    err,
  })

  useEffect(() => {
    if (answering === null) return
    const q = run?.question ?? null
    const same =
      q !== null &&
      q.questionKind === answering.kind &&
      q.deadlineUnixMs === answering.deadline
    /* 问句换了人 / 已经收掉 ⇒ 松手。
       ★ 报错**不再**松手（2026-10-19）：一松手按钮就闪回来、而那一答其实没成功，
       人接着点接着错。留着 `answering` 让「停止并关闭」这条逃生通道明确
       —— 见 `questionUi.ts` 那段。 */
    if (!same) {
      setAnswering(null)
      return
    }
    /* 兜底：那一问的自动终止是 118 秒，走完必然换屏；这里只是防"永远" */
    const t = window.setTimeout(() => setAnswering(null), 130_000)
    return () => window.clearTimeout(t)
  }, [answering, run?.question])
  /*
   * 收尾窗口：跑完那一刻**先让条子把最后几个点走完**（95% → 100%），再切结果态。
   * 作者 2026-10-14：「最后 95% 的时候就直接结束消失了，出现了结果页面」—— 太突兀。
   * 同日晚又定了一条：「我希望他处理快，但是动画慢，起码要 3 秒」——
   * **处理（内核）多快就多快，这一屏的演示不许一闪而过**。落地成四条：
   *   ① 条子由 `useSmoothPercent` 自己报告"我走到 100 了"（`onSettled`）——
   *      不再用固定 420ms 去猜（小文件里条子可能还差半条街，猜出来就是"突然完成"）；
   *   ② 从**窗口打开**算起，整段至少 `MIN_SHOW_MS`；
   *   ③ 到 100 之后再停留 `SETTLE_MS` —— 让人看清那个 100%；
   *   ④ 失败 / 停止**不适用**（那种时候该尽快把结果给人看，别拿动画拖人）；
   *   ⑤ 兜底 `SETTLE_MAX_MS`：条子万一没报上来，到点也得切（不许卡在这一屏）。
   */
  const [barDone, setBarDone] = useState(false)
  /** 收尾窗口到点（条子走完 + 展示下限满足）⇒ **开始淡出**（见下面那条 effect） */
  const [settled, setSettled] = useState(false)
  /** 淡出完 ⇒ **真换**结果态（作者「没有渐隐渐现」那一条：不再"啪"地一下换） */
  const [swapped, setSwapped] = useState(false)
  /** 结果态里双击四格 ⇒ 翻回 100% 的 hero（"看一眼刚才那根条"）；hero 上双击再翻回来 */
  const [peek, setPeek] = useState(false)
  /**
   * 刚从"翻过来看 100%"翻回结果 —— 那一下的结果块要**弹上来**（作者：
   * 「我双击切换的时候，那个新的应该也弹上来的感觉」）。收尾那一路仍是纯淡入（`.leadIn`）。
   */
  const [flipBack, setFlipBack] = useState(false)
  /*
   * 这两格标量是下面那条 effect 的依赖 —— **不许直接把 `finished` 对象写进依赖**：
   * 「预估打印时间」是后补的，补进来时 `usePostProcessRun` 会把同一个结论换成新对象，
   * 拿对象当键就会把它当成"结果变了"（2026-10-18 作者：「最后这个还会闪一下」）。
   */
  const finishedNull = finished === null
  const finishedOk = finished !== null && finished.ok
  useEffect(() => {
    if (finishedNull) {
      setBarDone(false)
      setSettled(false)
      setSwapped(false)
      setPeek(false)
      setFlipBack(false)
      return
    }
    if (!finishedOk) {
      const id = window.setTimeout(() => setSettled(true), SETTLE_MS)
      return () => window.clearTimeout(id)
    }
    if (!barDone) {
      const id = window.setTimeout(() => setSettled(true), SETTLE_MAX_MS)
      return () => window.clearTimeout(id)
    }
    /* `sinceMs` 缺省（老记录）时按"刚打开"算 —— 最坏也只是把下限走满，不会一闪而过 */
    const elapsed = Date.now() - (view.sinceMs ?? Date.now())
    const wait = Math.max(MIN_SHOW_MS - elapsed, 0) + SETTLE_MS
    const id = window.setTimeout(() => setSettled(true), wait)
    return () => window.clearTimeout(id)
  }, [finishedNull, finishedOk, barDone, view.sinceMs])

  /*
   * 渐隐渐现（作者：「感觉 100% 的时候消失得太突然了，没有渐隐渐现」）：
   * hero 先淡出 `LEAD_FADE_MS`（WAAPI，见下面那个 effect），走完才真换（结果块自己淡入）。
   * **失败 / 取消不走这套** —— 那种时候该尽快把结果给人看（作者："失败那些停止那些不用做"）。
   */
  useEffect(() => {
    if (!settled) {
      setSwapped(false)
      return
    }
    if (finished?.ok !== true) {
      setSwapped(true)
      return
    }
    const id = window.setTimeout(() => setSwapped(true), LEAD_FADE_MS)
    return () => window.clearTimeout(id)
  }, [settled, finished?.ok])
  /**
   * 顶上那一块现在该按"跑着"画：跑着 / 收尾窗口（含淡出中）/ "翻回来看 100%" 都要它，
   * 换完（`swapped`）且没翻过来才交给结果态。
   */
  const showProgress = running || peek || !swapped

  /*
   * 出场动画 —— **JS 驱动**（`element.animate()`），与 `usePressFlash` 同一个理由：
   * 作者真机上这套 CSS `animation` 一个都不播（拆帧量过：切换是"一帧内跳变"），
   * 而 `element.animate()` 是 JS 调用、不过 CSS 级联，那条全站降级规则管不到它。
   *   · hero：从"翻过来看 100%"翻上来时**弹上来**；收尾时**淡出**（然后 `swapped` 换成结果）。
   *   · 结果块：收尾切过来**淡入**；从"翻过来看"翻回来时**弹上来**。
   */
  const heroBoxRef = useRef<HTMLDivElement>(null)
  const leadBoxRef = useRef<HTMLDivElement>(null)
  /** 收尾成功那一瞬：hero 正在渐隐（失败 / 取消不渐隐 —— 那种时候该尽快把结果给人看） */
  const fadingOut = settled && !swapped && finished?.ok === true
  useEffect(() => {
    const el = heroBoxRef.current
    if (el === null || typeof el.animate !== 'function') return
    /*
     * ★ 先把上一轮留下的动画清掉 —— 尤其"收尾淡出"那个 `fill: 'forwards'`：
     * 它把这一层按在 opacity 0 上，翻面回来复用时就成了"这一块是空的"
     * （作者 2026-10-15 的截图）。清掉 = 回到元素自身样式，再按本轮状态重新起。
     */
    el.getAnimations().forEach((a) => a.cancel())
    if (!showProgress) return
    if (fadingOut) {
      el.animate([{ opacity: 1 }, { opacity: 0 }], {
        duration: LEAD_FADE_MS,
        easing: 'ease',
        fill: 'forwards',
      })
      return
    }
    if (peek) el.animate(POP_KEYS, { duration: 240, easing: 'ease-out' })
  }, [showProgress, fadingOut, peek])
  useEffect(() => {
    const el = leadBoxRef.current
    if (el === null || typeof el.animate !== 'function') return
    el.getAnimations().forEach((a) => a.cancel())
    if (flipBack) {
      el.animate(POP_KEYS, { duration: 240, easing: 'ease-out' })
      return
    }
    const fade = el.animate(FADE_KEYS, { duration: 220, easing: 'ease' })
    /* 安全阀：万一这动画没跑完（真机上见过"停在起始帧"那类），到点强制回到自身样式（= 可见） */
    window.setTimeout(() => {
      if (fade.playState !== 'finished') fade.cancel()
    }, 400)
    /*
     * ★★ **不许把 `finished` 整个对象写进依赖**（2026-10-18 作者：「最后这个还会闪一下，
     * 不是这个碰撞的问题，没碰撞他也会闪一下」）。
     *
     * 「预估打印时间」是**后补的**：钩子跑完先给一份结论（那一格空着），常驻进程算完再推
     * `postprocess-print-time`，`usePostProcessRun` 那里会 `{...prev.finished, result:{...}}`
     * —— **同一个结论换了个新对象**。依赖写 `finished` 就会当成"结果变了"重跑这条 effect：
     * 先把正在跑的淡入 `cancel()` 掉，再从 0 重新淡入一遍 ⇒ 屏上就是那一下闪。
     * 真正该重播的时机只有两个：`showProgress` 翻过来（`swapped`）与翻面回来（`flipBack`）。
     */
  }, [showProgress, flipBack])

  /**
   * 翻面（**轮播感**）——作者 2026-10-15：「切换的时候上一个往上消失，下一个从下弹出来；
   * 现在感觉消失的没有只有出现的」。所以旧的**先往上滑出去**（WAAPI 200ms），滑完才真翻；
   * 新块由上面那两个 effect 从下弹进来。**不许"先切再弹"**——那样旧的就是瞬间消失。
   * 快速连击：`cancel()` 会打断上一轮滑出，直接翻（干净利落，不排队）。
   */
  /**
   * 翻面（**轮播感**，作者 2026-10-15：「上一个往上消失，下一个从下弹出来」+
   * 同日第二轮：「速度有点慢，我点击快的时候他也要快，要很流畅的翻页的感觉」）：
   *   · **旧块先往上滑出去**（WAAPI 130ms），滑完才真翻；新块由上面那两个 effect 从下弹进来。
   *   · **快速连击**：上一轮还在滑出就直接翻（不等那 130ms）—— 点得快它也要快，不许排队。
   *   · `gen`（轮次号）：被打断的旧轮次回来后一律忽略，免得"旧轮次的收尾"把新一轮顶掉。
   */
  const flipGenRef = useRef(0)
  const flipBusyRef = useRef(false)
  const startFlip = (to: 'hero' | 'result') => {
    const gen = flipGenRef.current + 1
    flipGenRef.current = gen
    const apply = () => {
      if (flipGenRef.current !== gen) return /* 过期轮次：忽略 */
      flipBusyRef.current = false
      if (to === 'hero') {
        setPeek(true)
      } else {
        setPeek(false)
        setFlipBack(true)
      }
    }
    /* 旧块：翻到 hero 时它是结果块；翻回结果时它是 hero 那一层 */
    const el = to === 'hero' ? leadBoxRef.current : heroBoxRef.current
    /* 快连击（上一轮还在滑出）或没有可动的元素 ⇒ **立刻翻**，不等 */
    if (flipBusyRef.current || el === null || typeof el.animate !== 'function') {
      if (el !== null && typeof el.animate === 'function') {
        el.getAnimations().forEach((a) => a.cancel())
      }
      apply()
      return
    }
    flipBusyRef.current = true
    el.getAnimations().forEach((a) => a.cancel())
    const out = el.animate(
      [
        { transform: 'translateY(0)', opacity: 1 },
        { transform: 'translateY(-20px)', opacity: 0 },
      ],
      /* fill: forwards —— 滑完**停在**"透明+上移"上，等 `apply` 把这一整块卸掉；
         不加的话动画一结束元素会瞬间回闪到原样（那一下就是"乱"的一半） */
      { duration: 130, easing: 'ease-in', fill: 'forwards' },
    )
    const done = () => {
      flipBusyRef.current = false
      apply()
    }
    out.finished.then(done).catch(done)
    /* 安全阀：动画万一没跑完也照翻，不许卡在旧块上 */
    window.setTimeout(done, 260)
  }

  /**
   * 这一屏是**开发台演出来的**（不是真钩子跑出来的）。
   *
   * 那几颗真业务按钮（停止 / 后台运行 / 强制继续 / 终止）走的是真钩子回写线，
   * 预览里没有那条线 —— 点了只会如实报"现在没有在跑后处理"（作者 2026-10-14：
   * 预览就明确是预览，不许点出真业务）。所以预览里把它们**禁掉**，并标一句「预览模式」。
   */
  const preview = run?.preview === true

  useEffect(() => {
    if (!running) return
    const id = window.setInterval(() => setTick((n) => n + 1), 1000)
    return () => window.clearInterval(id)
  }, [running])

  /*
   * **挪塔那一问的两页**（2026-10-18）：上面是示意图，下面那块位置在
   * 「挪塔说明」与「原始两栏信息」之间翻页 —— 左下角那颗很小的按钮切。
   * 两页占**同一块高度**（`.relocateSlot` 定高），所以模态框不跟着变大小。
   */
  const [relocateTab, setRelocateTab] = useState<'note' | 'facts'>('note')
  const questionKind = run?.question?.questionKind
  useEffect(() => {
    /* 换一问（或这一问结束）就翻回"说明"那页 —— 别把上一问的翻页状态带过来 */
    setRelocateTab('note')
  }, [questionKind])

  if (run === null || view.dismissed) return null

  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)
  /*
   * **条子不往回跳**（2026-10-18 作者：「我点击之后它又从 0% 开始了，没有从中断的
   * 百分比继续」）。
   *
   * 那一问是**管线开跑之前**的预检（钩子在 `process_with_ir` 之前先算一遍擦料塔会不会撞，
   * 见 `hook.rs` 那段"闸门必须与管线同一套判据"），所以答复之后主线是从 0% 起跑的 ——
   * 数值本身没错，可**看上去就是"又重来了一遍"**。这里把答复那一刻的百分比记成下限：
   * 新一段真实进度追上来之前，条子停在那儿不倒车；追上之后就跟着真实值走（下限只是
   * `max`，不是伪造）。
   */
  const shownPercent = answering === null ? percent : Math.max(percent, answering.from)
  const eta = running ? etaText(view.sinceMs, percent) : null
  const facts = run.facts

  /** 挪塔那一问、且几何真带出来了 —— 这一屏的版式与别处不同（见上面的翻页状态） */
  const isRelocate =
    showAsk &&
    run.question !== null &&
    run.question.questionKind === 'tower-relocation' &&
    run.question.geo?.collision != null

  /** 两条命令（停止 / 答复）都要等后端，失败要说出来 —— 界面不许"点了没反应" */
  const act = (fn: () => Promise<void>) => {
    setBusy(true)
    setErr(null)
    /* 返回 promise：调用方要接一句"这颗按钮正在办"（`answering`），见下面的选项按钮 */
    return fn()
      .catch((e: unknown) => setErr(errorText(e)))
      .finally(() => setBusy(false))
  }

  /**
   * **逃生：先请求停止、再收窗**（2026-10-19，作者：「出错的时候加一个停止并关闭的按钮，
   * 这样就不用管多的操作了」）。
   *
   * `cancelPostProcess` 在这一刻**可能失败**（钩子已经退了 ⇒ 后端如实回
   * 「这一趟已经结束了，没有正在跑的处理」）。那句错在这里**故意吞掉**：
   * 这颗按钮的语义是"把这一屏收场"，不是"保证停成功"——
   * 吞掉之后 `dismiss()`，人回到主界面，不用再跟红字纠缠。
   */
  const stopAndClose = () => {
    void api
      .cancelPostProcess()
      .catch(() => {
        /* 钩子已经不在了 —— 正是这一颗按钮要处理的那种局面，不往界面上抛 */
      })
      .finally(() => dismiss())
  }


  /*
   * 顶部那一块（跑着 hero / 问句警告 / 结果 hero）——四种长相收成一个变量，
   * 下面那句 facts 与警告行是它们**共用**的尾巴。
   */
  let lead: ReactNode = null
  if (showAsk && run.question !== null) {
    /*
     * **挪塔那一问**（2026-10-18）：撞了要看图才敢决定 ⇒ 上摆放两幅俯视图（不套外圈
     * 容器、高度跟 hero 那一带对齐），文案挪到下面"原本两栏事实"的位置（可翻页，
     * 见下面的 `relocateTab`）。几何没带出来（旧钩子 / 解析丢格）就退回三行字。
     */
    lead = isRelocate ? (
      <RelocateMaps question={run.question} machineType={run.facts?.presetMachine ?? null} />
    ) : (
      <AskHero
        questionKind={run.question.questionKind}
        text={run.question.text}
        deadlineUnixMs={run.question.deadlineUnixMs}
      />
    )
  } else if (showProgress) {
    lead = (
      /*
       * ⚠️ `key` 是必须的（2026-10-15 作者："上一个还没消失，下一个就进来了 / 乱了"）：
       * 这一层与结果块那一层在 JSX 里是**同一个位置的条件渲染**，不给 key 时 React 会
       * **复用同一个 div** —— 于是"旧块滑出"与"新块弹入"作用在**同一个元素**上互相打断
       * （滑一半突然换内容）。key 不同 ⇒ React 强制"卸载旧的、新建新的"，各演各的。
       */
      <div ref={heroBoxRef} key="heroBox">
        <ProgressHero
          /* 收尾窗口里目标直接给 100：让条子走完那最后几个点，而不是停在 95% 被切走 */
          percent={running ? shownPercent : 100}
          step={run.progress === null ? '准备中' : stepName(run.progress.step)}
          stepId={run.progress?.step ?? null}
          sizeBytes={run.facts?.fileSizeBytes ?? null}
          eta={eta}
          done={!running}
          /* 条子真的走到 100 了 —— 收尾窗口据此决定何时切结果态（见 above 那段注释） */
          onSettled={() => setBarDone(true)}
          /* 双击：只在"翻过来看 100%"那一态接（翻回结果态）；跑着的时候不接 */
          onDoubleClick={peek ? () => startFlip('result') : undefined}
        />
      </div>
    )
  } else if (finished !== null) {
    if (finished.ok) {
      /* 双击四格 ⇒ 翻回 100% 的 hero（作者：「双击之后显示 100% 那个 hero」） */
      lead = metricsReady(finished.result) ? (
        /* 出场（淡入 / 弹上来）由主件那个 effect 用 WAAPI 驱动 —— 同上；
           `key` 与 hero 那一层必须不同（见那边注释） */
        <div ref={leadBoxRef} key="leadBox">
          <MetricsStrip res={finished.result} onDoubleTap={() => startFlip('hero')} />
          {/*
            校准件那一趟：**两种跑法都是成功**，同一屏，只有这一句按选择分开说
            （作者 2026-10-17：「两边一样的成功，但是文案可以区别一下」）。

            ★ 这一句**摆在标题右边那一格**（`HeadNote`），不是框身里 —— 作者：
            「这一个文字提示不应该加在这个地方，应该加在标题后面。因为你加在这的话，
            会导致我的模态框高度宽度变化」。文案住 `messages.toml`（`hook.calib_done_*`），
            报告读同一组键 —— 这里只挑键。
          */}
          {finished.calibChoice !== undefined && (
            <HeadNote text={calibDoneLine(finished.calibChoice)} tone="note" />
          )}
        </div>
      ) : null
    } else {
      const lines = [
        /* 「已停止 —— 」那半句前缀不用再摆：标题那行就是「已停止」 */
        finished.message.replace(/^已停止 —— /, ''),
      ]
      const codes: string[] = []
      if (finished.code !== null) codes.push(`错误码 ${finished.code}`)
      if (finished.exitCode !== 0) codes.push(`退出码 ${finished.exitCode}`)
      /* 第 46 轮定的口径：失败**不说"停在哪一步"** —— 只说"失败了"等于让人从头猜，
       * 但那一行用户点名不要；错误码与退出码并成一行，跟切片器那个框对得上号 */
      if (codes.length > 0) lines.push(codes.join(' · '))
      lead = (
        <ResultHero
          cancelled={finished.cancelled}
          title={finished.cancelled ? '已停止' : '处理失败'}
          lines={lines}
        />
      )
    }
  }

  const screen = (
    <Modal
      open
      size="md"
      title={showProgress ? '后处理进度' : '后处理结果'}
      /* 文件名那一格原来是标题右边的 `s.files` —— 通用框有 `subtitle`，用它。
      预览里前面加「预览 ·」—— 一眼看出这不是真跑（页脚那枚小字之外的第二处） */
      subtitle={preview ? `预览 · ${run.gcodeName}` : run.gcodeName}
      /* 点遮罩不关（全站口径 2026-10-15；跑着的时候更是如此）—— 收起来请按 Esc 或「后台运行」 */
      /* 跑着的时候连 × 都不摆 —— 那一刻没有"关掉"这个动作。
         结果那几扇也不摆（第 16~17 轮定的）：收窗走页脚「关闭」那一条明路 */
      hideClose
      /* Esc 也一样收进那条口径：跑着的时候"收起来"只有「后台运行」这一条明路 */
      closeOnEsc={!running}
      onClose={dismiss}
      footer={
        preview ? (
          /* 预览模式：真业务按钮**一个都不摆**（它们走真钩子回写线，这里没有那条线）。
             只留「关闭」—— 关掉预览、回工作台选下一屏（作者 2026-10-14 裁定）。 */
          <>
            <span className={s.previewTag}>预览模式 · 不接真实后处理</span>
            <Btn variant="accent" onClick={dismiss}>
              关闭预览
            </Btn>
          </>
        ) : offerStopAndClose ? (
          /*
           * **逃生通道**（2026-10-19，作者：「出错的时候加一个停止并关闭的按钮，
           * 这样就不用管多的操作了」）。只摆这一颗 —— 报错之后那些正常按钮
           * （强制继续 / 终止 / 停止）都已经没意义，多摆一颗只会让人以为还能继续答。
           */
          <>
            <span className={s.previewTag}>这一趟对不上了</span>
            <Btn variant="accent" onClick={stopAndClose}>
              停止并关闭
            </Btn>
          </>
        ) : showAsk && run.question !== null ? (
          /*
           * 那一问的按钮（2026-10-17 泛化）：**问句自带选项表就照它画**，没有就还是
           * 机型那一问的老两颗（强制继续不强调 / 终止占强调位 —— 第 9~11 轮定的）。
           * 按钮皮肤是全站那一套（`.btn` 默认胶囊），这里不加任何 className。
           */
          <>
            {/*
             * 挪塔那一问：右下是动作按钮，**左下这片空白**摆一颗很小的翻页钮
             * （作者 2026-10-18：「我点击很小的切换信息按钮，这个按钮可以放在左下角空白的位置」）。
             * `margin-right: auto` 把它顶到最左，动作按钮仍靠右。
             */}
            {isRelocate && (
              <button
                type="button"
                className={s.slotToggle}
                onClick={() => setRelocateTab((t) => (t === 'note' ? 'facts' : 'note'))}
                aria-label={
                  relocateTab === 'note' ? '切换：显示原始信息' : '切换：显示挪塔说明与坐标'
                }
                title={relocateTab === 'note' ? '查看原始信息' : '回到挪塔说明'}
              >
                <InfoGlyph />
                {relocateTab === 'note' ? '信息' : '说明'}
              </button>
            )}
          {run.question.choices !== undefined && run.question.choices.length > 0 ? (
            /* `data-choices` = 「问句自带选项」那一档：页脚**允许折行**（见 `.footChoices`）。
               ★ 原来的样子（作者 2026-10-17 截图）：两颗长按钮都不许收缩（全局
               `button { white-space: nowrap }`），中号框装不下 ⇒ 第二颗被推出框外看不见。
               `hint` 只是补充说明，**不再塞进按钮里**：按钮的字 = `label`，
               后果交给 `title`（悬停可读），按钮本身短了才排得下。 */
            <span className={s.footChoices} data-choices>
              {/*
               * ★★ **主按钮恒在右**（2026-10-18 作者：「一般来说有颜色的都在右侧的…
               *   左边的按钮优先级低，右边的高，有背景颜色的」）。
               * 后端给的是"主在前"（主 = `accent`），这里**稳定排序**把 accent 挪到最后
               * ⇒ 绿按钮永远是最右那颗，别处（校准件那一问）同一份规则。
               */}
              {[...run.question.choices]
                .sort((a, b) => Number(a.accent === true) - Number(b.accent === true))
                .map((c) => (
                /* 外观 = 全站那颗 `Btn`（深浅两套都在 `ui.module.css` 里，
                   深色下 accent 是实心绿白字）；这里只**借**一个布局类让它能折行收缩。
                   `hint` 挂 `title`：后果说明不该挤在按钮上（按钮越长越排不下）。 */
                <Btn
                  key={c.id}
                  variant={c.accent ? 'accent' : 'default'}
                  className={s.choiceBtn}
                  disabled={busy}
                  title={c.hint}
                  onClick={() => {
                    setAnswering({
                      kind: run.question?.questionKind ?? '',
                      deadline: run.question?.deadlineUnixMs ?? 0,
                      from: percent,
                    })
                    void act(() =>
                      api.answerPostProcess(run.question?.questionKind ?? '', c.id),
                    )
                  }}
                >
                  {c.label}
                </Btn>
              ))}
            </span>
          ) : (
            <>
              <Btn
                disabled={busy}
                onClick={() =>
                  act(() => api.answerPostProcess(run.question?.questionKind ?? '', 'keep'))
                }
              >
                强制继续
              </Btn>
              <Btn
                variant="accent"
                disabled={busy}
                onClick={() =>
                  act(() => api.answerPostProcess(run.question?.questionKind ?? '', 'stop'))
                }
              >
                终止
              </Btn>
            </>
          )}
          </>
        ) : running ? (
          <>
            <Btn disabled={busy} onClick={() => act(() => api.cancelPostProcess())}>
              停止
            </Btn>
            <Btn variant="accent" onClick={dismiss}>
              后台运行
            </Btn>
          </>
        ) : (
          <>
            <Btn onClick={dismiss}>关闭</Btn>
            <Btn
              variant="accent"
              onClick={() => {
                dismiss()
                onOpenReport()
              }}
            >
              → 查看详情
            </Btn>
          </>
        )
      }
    >
      <div className={s.body}>
        {lead}
        {/*
         * 事实表**从开场就在**（2026-10-14 作者截图：「下面的内容那些字段它可以显示出来呀，
         * 只是参数你留空或者用 -」）。
         *
         * 原来是 `facts !== null && …`：开场那一瞬钩子的 `Hello` 先到、`facts` 还没到，
         * 整块两栏事实都不渲染 ⇒ 框里一条孤零零的 hero，等事实到齐才"啪"地长开 ——
         * 那一下就是作者说的闪（框身已经定高，但框内空一块更难看）。
         * 口径反过来：**框一打开就是完整的**，没到手的值一律 `—`（两组里每一格都有固定标签，
         * 所以不会"少一格"，只会有"暂时是 —"），值到了原地换掉。
         */}
        {isRelocate && run.question !== null ? (
          /*
           * 挪塔那一问：这一格在「说明」与「原始两栏信息」之间**翻页**（左下角那颗小钮切）。
           *
           * ★★ **两页都挂着、叠在同一格里**（2026-10-18 第三次修）：前一版给槽写死一个
           *   高度，可"四行两栏"的真实高度随主题/字体变 ⇒ 要么底下留一大片空、要么
           *   撑高框。叠起来之后槽高 = **较高的那一页**（就是两栏事实），于是：
           *     · 信息页零留空（它就是定高的那一页）；
           *     · 翻页不跳（另一页被 `visibility: hidden` 藏住，但**仍占格**）。
           */
          <div className={s.relocateSlot}>
            <div
              className={`${s.relocatePage} ${relocateTab === 'note' ? '' : s.relocatePageOff}`}
            >
              <RelocateNote
                question={run.question}
                countdown={countdownText(run.question.deadlineUnixMs)}
              />
            </div>
            <div
              className={`${s.relocatePage} ${relocateTab === 'facts' ? '' : s.relocatePageOff}`}
              aria-hidden={relocateTab !== 'facts'}
            >
              <FactsGroups groups={[baseGroup(facts), runGroup(facts)]} />
            </div>
          </div>
        ) : (
          <FactsGroups
            groups={
              asking || running
                ? [baseGroup(facts), runGroup(facts)]
                : [baseGroup(facts), doneGroup(finished as PostProcessFinished, facts)]
            }
          />
        )}
        {/*
          内核那几条警告（"这一盘没有支撑"之类）：
            · **标题右边那一格**（`HeadNote`）= 汇总一句话「警告 N 条 · 首条…」，
              作者 2026-10-17：「把所有这些提示的，不管是什么的，都用同一套组件、同一套逻辑
              复用在标题后面显示」；
            · 框身这张列表 = **明细**（每条一行，要读全就得在这儿）。
          两者分工不同，不是重复：格子只回答"有没有事、什 么事"，
          明细回答"具体哪几条"。明细的条数不影响格子（格子只有一行、省略号）。 */}
        {finished !== null && finished.warnings.length > 0 && (
          <HeadNote text={warningsLine(finished.warnings)} tone="note" />
        )}
        {finished !== null &&
          finished.warnings.map((w) => (
            <p className={s.warn} key={w}>
              警告：{w}
            </p>
          ))}
        {err !== null && <HeadNote text={err} tone="error" />}
      </div>
    </Modal>
  )

  /*
   * 跑着的那一态：**整扇窗都归这一屏**（连顶栏那三颗窗控键）。
   *
   * 那三颗被 TopTabs 自己抬到了 `z-index: 94`（它要盖住外壳顶部那条 1px 高光，
   * 也要压过窗口内侧那圈 resize 命中条 —— 见 `TopTabs.module.css` 的 `.controls`），
   * 而通用模态框的遮罩是 `z-index: 40` —— 于是"模态框开着、右上角那颗关闭照样点得着"
   * （作者 2026-10-11 点着的就是那一下）。这一层跑着的时候抬到 92（窗控键之上、
   * 拖窗带之下），整扇窗的**内容**都归它。
   *
   * 顶上那条拖窗带是这一层的**兄弟**（不在里面）：它要压在所有东西之上 ——
   * 包括那三颗窗控键（94）与窗口内侧那圈 resize 命中条（93）里落在标题栏上的那几条。
   * 于是跑着的时候，整条标题栏就是窗口的把手：拖窗、双击最大化照旧，那三颗键点不着。
   * 拖窗是窗口的基本动作，不该被一个正跑着的框锁死。
   *
   * **窗口大小仍然改得动**（作者 2026-10-11 明确要的）：左右两条边（标题栏以下）、
   * 下边一条与下面两个角那几块命中条（93）都在拖窗带之外，跑着的时候照旧能拖 ——
   * 让给拖窗带的只有**标题栏那一格里的几块**（北边一条、上面两个角，以及左右两条边
   * 顶上那 38px）：拖窗带必须整条盖住窗控键才管用，而那几块本来就在标题栏上。
   *
   * 不跑的那一态（结论留在屏上）**层高不降**（2026-10-18：结果框要压过参数弹窗，
   * 它是这一屏的头等事），但仍低于窗控键（94）—— 看完结果顺手关窗照旧。
   */
  return (
    <>
      <div className={s.layer}>
        {screen}
      </div>
      {running && (
        <div
          className={s.dragShield}
          data-platform={PLATFORM}
          /* 探针认它：标题栏被这一层接管时，**它就是窗口把手**（拖窗照旧）
             —— `scripts/probes/pp-modal-height.mjs` 的命中测试认这两个之一 */
          data-drag-handle="true"
          aria-hidden="true"
          onMouseDown={drag.onMouseDown}
          onDoubleClick={drag.onDoubleClick}
        />
      )}
    </>
  )
}
