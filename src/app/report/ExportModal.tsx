/*
 * 「导出执行报告」那一框（2026-10-11 · 照 `demo2/HANDOFF-EXPORT-3MF-NAME.md` §B）。
 *
 * 三件套一起导 —— **原始文件（改动前的备份） / 处理后文件 / JSON 元数据** ——
 * 就是归档里那三份原样复制出去，不多导网页报告与截图（那些以后可能做，不在这张清单里）。
 *
 * # 谁说了算
 *
 * 文件名与默认落点**都是后端的**（[`api.getReportExportPlan`]）：命名规则只许有一处
 * （`ipc/report.rs` 那个纯函数），界面照单显示。否则"框里写着一个名、盘上落另一个名"
 * 这种错没有任何东西咬得住。
 *
 * # 三条纪律
 *
 * 1. **不在框里造词**：归档里缺的那几份照实说（失败的那一次不会有"处理后的文件"），
 *    不摆一行"缺"的空行、也不假装三件套都齐；
 * 2. **失败留在框里**：导出没成（不覆盖 / 目录建不出来 / 一份都不在盘上）→ 后端那句
 *    原话**摆在标题右边那一格**、框不关 —— 用户就着这句话改位置或先把那一份挪走，
 *    不用从头来一遍；
 *
 *    ★ 2026-10-17 收口：原来那句话是框身里一块带底色的 `<p className={s.bad}>`，
 *    每出现一次就把文件清单往下顶一截。作者定死「所有提示都进标题右边那一格」⇒
 *    现在走共用件 `HeadNote`（`components/modal`）。**代价是长错误被省略号截掉**
 *    （全文在 `title` 悬停可读），这是作者在"绝对统一 vs 框身可读全文"之间选的后者优先 —— 
 *    别再自作主张把它挪回框身；
 * 3. **成了也不关、也不长高**（作者 2026-10-11 两次点名）：
 *    · 导完最要紧的下一步是"去那个文件夹里看看"，框一关路径就没了 ⇒ 框留着；
 *    · 但**不许因此多出一条东西**：原来那句绿色横条把框顶高了、看着像闪了一下 ⇒
 *      现在就着**已有的那几行**说成功 —— 每份文件的小图标收成一个绿勾、
 *      保存位置那颗按钮从「更改位置」翻成「打开位置」、页脚左边多一句
 *      「已导出 N 个文件」。一条新行都不加，框的大小一动不动。
 *
 *    ★ 2026-10-31 作者第三次点同一件事，这次是**打开的那一下**：「一打开那个高度
 *    不是固定的 —— 开始比较矮，过半秒一秒才变高」。两条根因、两处修：
 *    · 计划原来只在 `useEffect` 里灌进 state ⇒ 首帧画的是"正在读导出清单…"。
 *      现在 `plan` 是渲染期算出来的（预取命中就直接用），首帧即终态；
 *    · `.body` 里垫了一副看不见的**量尺**（与终态同构的空架子）⇒ 就算真要现场读，
 *      框身高度也不动。两处都写在 `ExportModal.module.css` 的 `.body` 那条下面。
 *
 * # 保存位置记着上一次（`STORAGE.clientReportExportDir`）
 *
 * 纯前端偏好，不是底账：它只决定框打开时路径胶囊里先写哪一串。目录不在了照样是
 * 后端说了算（会照实报"建不出保存位置"），所以记住一个失效的路径不会骗人。
 *
 * 胶囊**整条不改位置**：只有右边那颗按钮弹系统框 —— 路径正文因此还能拖选，
 * 误点一下也不会跳出一个系统目录框。
 *
 * # 左下角那颗「信息」与「导出时启用压缩」（2026-11-03 · 作者当天两次改版）
 *
 * 第一版把压缩开关摆进了「信息」那一页，作者看图后的口径是：
 *
 * > 我希望还是这样的布局（压缩开关在文件清单底下）……这样就有位置在文件页面也显示这个开关了
 *
 * ⇒ **压缩开关留在第一页**，位置是文件清单下面、保存位置胶囊上面；
 *   「信息」那一页只剩一件事：**归档里那几份此刻在盘上哪儿**
 *   （可复制路径、可**不导出直接定位** —— 在访达 / 资源管理器里选中它）。
 *
 * 两条硬约束（作者两次点名，别再动）：
 *
 * 1. **底下那一簇钉在框身下沿**（`.pageFoot` 的 `margin-top: auto`）：
 *    失败那一次只有两行文件、还多一句"归档里缺哪几份"，可**保存位置胶囊的位置
 *    与三行时一模一样** —— 「失败的下面的也是固定在下面的位置的」。
 *    靠的是量尺：它照三行 + 这一簇的高度撑住框身，清单短了也不塌。
 * 2. **翻页不改变尺寸**：两页同时挂着、叠在同一格里（`.page`），藏起来的那页
 *    `visibility: hidden` 仍占格 —— 与后处理挪塔那一问的 `.relocateSlot` 同一个做法。
 *
 * 压缩开关不是"我行我素的显示项"：默认开、用户关过一次就记住
 * （`STORAGE.clientReportExportZip`），`confirm` 真的按这一格决定"打成一个 zip"
 * 还是"散着复制三份"。
 *
 * ★ 2026-11-04 去掉了两条多余的发丝线（作者圈出来的）：清单**顶上那条**与
 *   **最后一行下面那条**。行与行之间那几条留着 —— 它们是"一份一份"的分隔，
 *   首尾两条只是把清单框起来，念出来是"这里还有个表格"，而它并不是。
 *
 * # 翻到哪一页，只在「这一次打开软件」里记着（2026-11-04）
 *
 * 作者：「点了信息，下次打开别的那个文件的导出就还是显示信息……只是这一次
 * 打开软件的时候记住」。
 *
 * ⇒ 左下角那颗翻页钮改过的那一页，是**本次运行**的默认页：这一趟里再打开任何
 *   一条记录的导出框，都从用户最后翻到的那一页起；软件关掉再开回到「文件」。
 *   落点是模块级变量（`sessionTab`），**不写 localStorage、不写 app-state** ——
 *   它和旁边那颗压缩开关不是一回事：那一格是跨重启的偏好，这一格是临时习惯。
 */

import { useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { createPortal } from 'react-dom'

import { api, errorText, uiErrorOf } from '../../api'
import type { ReportExportFile, ReportExportKind, ReportExportPlan } from '../../api'
import { STORAGE } from '../../api/storageKeys'
import Icon from '../../components/Icon'
import { HeadNote, Modal, useModalHeadNoteSlot } from '../../components/modal'
import { Switch } from '../../components/switch'
import { useStickyState } from '../../hooks/useStickyState'
import { useT, type StringKey } from '../../i18n'
import { Btn } from '../ui/Controls'
import s from './ExportModal.module.css'

/** 三件套的说明（键）。后端只给机器名（`kind`），**界面的话住界面这一侧** */
const KIND_LABEL: Record<ReportExportKind, StringKey> = {
  original: 'report.export.kindOriginal',
  processed: 'report.export.kindProcessed',
  meta: 'report.export.kindMeta',
}

/**
 * 「量尺」要摆几行 —— **跟着三件套的份数走**（以后加一份就自动多一行，不会漏）。
 * 见下面 `.gauge`：它撑起的是"终态该有多高"，不是内容。
 */
const GAUGE_ROWS = Object.keys(KIND_LABEL)

/** 撑开行盒的那一格空白（`&nbsp;`）：空 span 没有行盒、高度是 0，量尺就白摆了 */
const NBSP = '\u00a0'

/** 「位置里已经有这一份」的稳定码（后端 `export_exists` 挂的，见 `ipc/report.rs`） */
const EXPORT_EXISTS_CODE = 'E_REPORT_EXPORT_EXISTS_001'

/**
 * 成功之后按钮僵住的时长（毫秒）。
 *
 * 作者 2026-11-04 被这件事故到过：连点几次「确认导出」，其中一下刚好落在**刚翻脸的那颗**
 * 上 —— 框当场关了。现在那颗不翻了（一直是「确认导出」），但"刚成功"的那一下仍要僵一会儿，
 * 让手指头停下来。900ms 是"看得出在忙、又不觉得卡"的那一档。
 */
const SETTLE_MS = 900

/** 框身两页：`files` = 三件套清单；`info` = 同一份清单 + 归档路径与两颗按钮 */
type ExportTab = 'files' | 'info'

/**
 * 本次运行里用户最后翻到的那一页（默认「文件」）。
 *
 * ★ 为什么不放在组件 state 里：这个框每次开 / 关都要把内务清干净，页面切走再回来
 *   还可能重挂 —— state 会跟着重来。模块级变量活到这份前端 bundle 被卸载为止，
 *   正好是「这一次打开软件」：不落任何盘，关掉软件再开自然回到 `files`。
 */
let sessionTab: ExportTab = 'files'

/**
 * 忙的时候那颗转圈（**WAAPI，不是 CSS 动画**）。
 *
 * 项目铁律：WKWebView 不播 CSS 关键帧动效 —— 写 `@keyframes` 在真机上就是一张静止的图，
 * 作者会看到"没有动画，好像卡住一样"。转圈住在这里、由 `element.animate` 驱动。
 */
function Spinner() {
  const ref = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    const el = ref.current
    if (el === null || typeof el.animate !== 'function') return
    const spin = el.animate([{ transform: 'rotate(0deg)' }, { transform: 'rotate(360deg)' }], {
      duration: 700,
      iterations: Number.POSITIVE_INFINITY,
    })
    return () => spin.cancel()
  }, [])
  return <span ref={ref} className={s.spin} aria-hidden="true" />
}

/** 折角的那张纸（形状与报告页详情里那套同一网格：24×24、currentColor） */
const PAPER_SHAPE = (
  <>
    <path d="M7 3h7l4.5 4.5V20a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 20V4.5A1.5 1.5 0 0 1 7 3z" />
    <path d="M13.5 3v5h5" />
  </>
)

/**
 * 每份前面的小图标。**三行是同一族的三种文件**：两份 G-code 都是那张纸，
 * 元数据那张纸里多一对花括号（`{ }`）。
 *
 * ★ 2026-10-11 改：原来那枚是"一对大括号 + 中间一根竖线"（v13 抽卡里照搬的），
 *   15px 下读起来像 `(()` —— 作者指着它说改一下。现在三行共用同一个纸张轮廓，
 *   只有元数据多一对小括号：同族、但一眼分得出。
 */
const KIND_ICON: Record<ReportExportKind, ReactNode> = {
  original: PAPER_SHAPE,
  processed: PAPER_SHAPE,
  meta: (
    <>
      {PAPER_SHAPE}
      {/* 花括号（lucide `file-json` 那一对，左右对称落在纸的中间偏下） */}
      <path d="M10 12a1 1 0 0 0-1 1v1a1 1 0 0 1-1 1 1 1 0 0 1 1 1v1a1 1 0 0 0 1 1" />
      <path d="M14 18a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1 1 1 0 0 1-1-1v-1a1 1 0 0 0-1-1" />
    </>
  ),
}

/** 写好了的那个勾（与报告页详情里「结论」那颗同形：圈里一个对号） */
const CHECK_SHAPE = (
  <>
    <circle cx="12" cy="12" r="8.5" />
    <path d="M8.5 12.5l2.5 2.5 4.5-5" />
  </>
)

/**
 * 页脚那颗「信息」上的记号：圈里一个 i。
 *
 * 画成 SVG 而不是写 `ⓘ`：字体里那个符号自带基线偏移，`align-items: center` 居中的是
 * 行盒不是字形，看上去会偏低（与后处理那一颗同一条理由、同一个字形，见
 * `PostProcessModal.tsx` 的 `InfoGlyph`）。
 */
/**
 * 文件名拆成"头 + 尾"两段 —— 为的是**中间省略**（2026-11-04 作者：「文件名字太长中间应该省略」，
 * 「反正不要换行高度不要变化」）。
 *
 * 头那段自己收省略号、尾那段钉住不缩：宽的时候两段拼起来就是原名（一个字符不少），
 * 挤的时候头先被吃掉、尾巴（时间戳 + 扩展名）永远在 —— 与访达里那串
 * `伊埃斯-bambu-2026...e_2026103115900` 是同一种读法。
 *
 * 为什么不在 JS 里按字数硬截：这里的字号是 `clamp()` 跟着窗口缩放的，
 * 按字数算等于替某个窗口宽度写死一次。CSS 只需要知道"哪一段可以缩"。
 */
function middleName(name: string): { head: string; tail: string } {
  /* 尾巴留 22 个字符：够放 `<时间戳>_original.gcode` 这一截 */
  const KEEP = 22
  return name.length <= KEEP ? { head: name, tail: '' } : { head: name.slice(0, -KEEP), tail: name.slice(-KEEP) }
}

/**
 * 标题右边那枚「处理失败」徽章（作者 2026-11-04：「失败的可以加上徽章」）。
 *
 * 走的是模态框**同一个回执槽**（`useModalHeadNoteSlot`）——那格本来就是"标题右边、
 * 永远一行、字数不影响框尺寸"的地方（见 `HeadNote.tsx` 头注）。徽章是个更短的形态：
 * 失败那一次它说明"这三行里有一条不在盘上"，逐行再由灰掉划线的那一行自己交代。
 */
function HeadBadge({ label }: { label: string }) {
  const slot = useModalHeadNoteSlot()
  if (slot === null) return null
  return createPortal(<span className={s.badge}>{label}</span>, slot)
}

function InfoGlyph() {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" fill="none" aria-hidden="true">
      <circle cx="6" cy="6" r="4.6" stroke="currentColor" strokeWidth="1.1" />
      <path d="M6 5.4v3" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" />
      <circle cx="6" cy="3.7" r="0.62" fill="currentColor" />
    </svg>
  )
}

/**
 * 路径胶囊里的那串字：`<父目录><分隔符><基名><分隔符>` 拆两段 ——
 * 前段自己收省略号、**尾两段钉住不缩**（"往哪个盘哪个文件夹里导"永远看得见）。
 *
 * 分隔符按父目录里用的那个认（Windows 收到的是 `\`，macOS 是 `/`）；
 * 段数少到没有"尾两段"可言时整条当前段，不分。
 *
 * `zipped` = 压缩导出：落点是一个 `<基名>.zip` **文件**，尾巴上不再带分隔符 ——
 * 胶囊里显示的就是"会落在哪个包"，读起来与散着导出那一档同一种语法。
 */
function pathPill(
  dir: string,
  base: string,
  zipped: boolean,
): { folder: string; full: string; head: string; tail: string } {
  const sep = dir.includes('\\') ? '\\' : '/'
  const name = zipped ? `${base}.zip` : base
  const folder = `${dir.replace(/[\\/]+$/, '')}${sep}${name}`
  const full = zipped ? folder : `${folder}${sep}`
  const lead = full.startsWith(sep) ? sep : ''
  const parts = full.split(/[\\/]/).filter((x) => x !== '')
  if (parts.length <= 2) return { folder, full, head: '', tail: full }
  return {
    folder,
    full,
    head: lead + parts.slice(0, -2).join(sep) + sep,
    tail: parts.slice(-2).join(sep) + (zipped ? '' : sep),
  }
}

/**
 * 归档路径拆成「目录 + 文件名」两段（信息页那一行用）。
 *
 * 与保存位置胶囊同一条理由：**尾那个文件名钉住不缩**，长的部分由前面的目录自己收省略号。
 * 只收成一条"从左边截断的路径"会把文件名截没 —— 那一行要回答的正是"这是哪个文件"。
 */
function splitPath(full: string): { dir: string; base: string } {
  const at = Math.max(full.lastIndexOf('/'), full.lastIndexOf('\\'))
  return at < 0 ? { dir: '', base: full } : { dir: full.slice(0, at + 1), base: full.slice(at + 1) }
}

export interface ExportModalProps {
  open: boolean
  /** 要导出的那一条记录（`null` = 列表里还没有选中的，那时光标根本打不开） */
  id: string | null
  /**
   * 页面替它**预先读好的**计划（切记录时就读了）。
   *
   * 有它开框就是终态高度：现场去读的话，框会先摆一行"正在读导出清单…"、等计划
   * 回来再长高 —— 那一下的跳动看着就像整页闪了一下（作者 2026-10-11 点的）。
   * `id` 对不上就不用（那一份是上一条记录的）。
   */
  preload: { id: string; plan: ReportExportPlan } | null
  onClose: () => void
}

export default function ExportModal({ open, id, preload, onClose }: ExportModalProps) {
  const tr = useT()
  /* 现场读回来的计划。**`id` 一起存** —— 光有计划分不清它是哪一条记录的 */
  const [fetched, setFetched] = useState<{ id: string; plan: ReportExportPlan } | null>(null)
  const [planErr, setPlanErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  /**
   * 框里那句话：`what` 决定前缀（导出 / 打开 / 定位各说各的），`text` 是后端原话。
   * 三种都**留在框里、不关框**：用户就着这句话改位置 / 先把那份文件挪走，就地重来。
   */
  const [fail, setFail] = useState<{ what: 'export' | 'open' | 'reveal'; text: string } | null>(
    null,
  )
  /** 成了：出了几份（框留着，见文件头第 3 条）。`null` = 还没导 */
  const [done, setDone] = useState<number | null>(null)
  /* 上一个选过的保存位置；空串 = 还没选过 ⇒ 用后端给的默认值（`<桌面>/MKP-导出报告`） */
  const [picked, setPicked] = useStickyState<string>(STORAGE.clientReportExportDir, '')
  /**
   * 框身现在摆哪一页：`files` = 三件套清单 + 保存位置；`info` = 同一份清单 +
   * 归档路径与两颗按钮。两页**同时挂着**（见下面 `.page`），所以翻页不改变框的尺寸。
   *
   * 初值取 `sessionTab`：本次运行里用户翻过页，下一趟开框首帧就得是那一页
   * （等 effect 再改会闪一帧）。
   */
  const [tab, setTab] = useState<ExportTab>(sessionTab)

  /** 翻页 = 改这一屏 + 记下本次运行的习惯（**只记这一下，不改别的内务**） */
  const chooseTab = (next: ExportTab) => {
    sessionTab = next
    setTab(next)
  }
  /** 「导出时启用压缩」：默认开，用户关过一次就记住（见 `STORAGE.clientReportExportZip`） */
  const [zip, setZip] = useStickyState<boolean>(STORAGE.clientReportExportZip, true)
  /** 刚复制过哪一份的路径（1.4 秒后自己熄掉，那颗小按钮上就一句「已复制」） */
  const [copied, setCopied] = useState<ReportExportKind | null>(null)
  /**
   * 刚成功的那一下**先僵一会儿**（2026-11-04 作者的原话：连点几次导出，「突然成功就点击到了
   * 完成按钮，模态框就关闭了，糟糕的体验」）。
   *
   * 老版面成功之后右下角那颗会翻成「完成」—— 连点的人第 N 下正好落在它身上，框就没了。
   * 现在那颗**不翻了**（一直是「确认导出」），但"刚成功那一瞬"仍然不该接受点击：
   * 这一格让页脚右侧先僵 `SETTLE_MS`，按钮上写「正在导出…」。
   */
  const [settling, setSettling] = useState(false)
  /**
   * 已经有一份在位置上了（`exports/` 里撞名）—— **只问"要不要覆盖"，不动手**。
   *
   * 作者 2026-11-04 问的是「位置已有怎么办？是显示一个打开文件夹的按钮还是干脆重新覆盖一次」。
   * 这里选**问一句**，两头都能走：
   *
   * - 「重新导出」⇒ 把这批盖掉（盘上不会留半个文件 / 半个包）；
   * - 「打开位置」⇒ 先去那儿看看那份是什么（可能是同一趟导出的，也可能用户自己改过）。
   *
   * 不直接静默覆盖：那份可能是用户自己动过的（唯一一份），一声不响地盖掉等于删数据；
   * 也不只给"打开位置"：那样每导一次撞名都要自己去删一次，太磨人。
   */
  const [overwrite, setOverwrite] = useState(false)

  /*
   * 开框读一次计划：文件名 / 默认落点 / 缺哪几份都是后端算的（页面预取过就直接用）。
   *
   * ★ 预取命中时**这里一个字节都不用写**：`plan` 是下面那条渲染期派生出来的，
   *   打开的第一帧就已经是终态（见那条注释）。原来在这里 `setPlan`，要等本 effect
   *   跑完才长高 —— 那一帧矮框就是作者 2026-10-31 点的那一下。
   */
  useEffect(() => {
    if (!open || id === null) return
    setPlanErr(null)
    setFail(null)
    setDone(null)
    /* 开框照本次运行里用户最后翻到的那一页起（没翻过 = 「文件清单」） */
    setTab(sessionTab)
    setCopied(null)
    /* 撞名那条问句属于**上一次那一下**，开框时一律收起来 */
    setOverwrite(false)

    if (preload !== null && preload.id === id) return

    let alive = true
    api.getReportExportPlan(id).then(
      (got) => {
        if (alive) setFetched({ id, plan: got })
      },
      (e: unknown) => {
        if (alive) setPlanErr(errorText(e))
      },
    )
    return () => {
      alive = false
    }
  }, [open, id, preload])

  /*
   * 关掉的这一下就把内务清干净，别等下次打开再 `set` —— 那会让"打开的第一帧"
   * 带着上一条的「已导出 N 个文件」/ 错误 / 灰按钮，再在第二帧变回去。
   */
  useEffect(() => {
    if (open) return
    setPlanErr(null)
    setFail(null)
    setDone(null)
    setBusy(false)
    setFetched(null)
    /* 关框也不把它掰回「文件」：下次开框的首帧要就是用户上次那一页 */
    setTab(sessionTab)
    setCopied(null)
    setSettling(false)
    setOverwrite(false)
  }, [open])

  /**
   * 框里要摊开的那份计划 —— **首帧就要拿到**。
   *
   * 页面替这条记录预取过（`preload`），这里在**渲染期**直接用它，不再等 effect
   * 灌进 state：`useEffect` 是首帧画完之后才跑的，所以原来打开的第一帧一定画成
   * "正在读导出清单…"那一行（框矮一截），下一帧才长到终态 —— 框是居中撑开的，
   * 上下一起外扩，比"内容变多"更扎眼（作者：「一打开那个高度不是固定的」）。
   *
   * 优先级：现场读回来的（同一 `id`）> 页面预取的（同一 `id`）> 没有。
   */
  const plan =
    fetched !== null && fetched.id === id
      ? fetched.plan
      : preload !== null && preload.id === id
        ? preload.plan
        : null

  const dir = picked === '' ? (plan?.defaultDir ?? '') : picked
  /* 压缩开着时落点是一个 `<基名>.zip`（胶囊里就得这么写，别写成"某个文件夹"） */
  const path = plan === null || dir === '' ? null : pathPill(dir, plan.base, zip)

  /**
   * 「这一次导出去的是哪一档」—— 成功之后要记住，不能跟着开关飘（作者 2026-11-04）：
   *
   * 导完再拨一下压缩开关，胶囊不该从 `.zip` 变回目录 —— 那句话说的是**已经发生在盘上
   * 的事**。所以 `done` 一记下来就把当时的落点与数量一起留住；拨开关只是"下一趟"的意向
   * （页脚那句「已导出…」也照这一份说，不跟着开关改口径）。
   */
  /*
   * `packed` = 这一趟**真装进包里 / 真复制出去**的份数（计划里 `exists` 的那几份）：
   * 压缩那一档的页脚要说"含 N 个文件"，而 `exportReportArchive` 压缩时只回一条 zip 路径，
   * 拿它数份数会说成"含 1 个文件"（作者一眼就能看出不对）。
   */
  const [last, setLast] = useState<{ zip: boolean; path: string; packed: number } | null>(null)

  /**
   * 拨开关 = 「下一趟要什么样」（作者 2026-11-04 第一件事）——
   * 与「更改位置」同一条：上一趟那句"已导出"不再成立，确认按钮回到可点。
   * 否则用户关个压缩、想再导一份散的，按钮却是「完成」，只能关框重来。
   */
  const chooseZip = (next: boolean) => {
    setZip(next)
    setDone(null)
    setLast(null)
    /* 撞名那条问句也一起收：换了落点，它问的那一份已经不对应了 */
    setOverwrite(false)
  }

  /**
   * 成功之后**僵一下**再让人接着点（见 `settling`）。
   *
   * 用 `window.setTimeout` 而不是 `useEffect`：这里要的是"一次成功对应一次僵住"，
   * 拉个 effect 反倒要再记一层状态。卸载时那个 timer 会自然失效（只 `set` 一个
   * 已经没人看的 state），不值得再挂清理。
   */
  const settle = () => {
    setSettling(true)
    window.setTimeout(() => setSettling(false), SETTLE_MS)
  }

  /** 「更改位置」：系统目录选择器。取消（`null`）保持原来那个位置，不是错误 */
  const changeDir = () => {
    void api.pickExportDir(dir).then(
      (next) => {
        if (next !== null) {
          setPicked(next)
          /* 换了个位置就是"再导一份"这件事重新开始：上一次那句"已导出"不再成立 */
          setDone(null)
          setLast(null)
          /* 换了个位置，撞名那条问句问的是**上一个位置**的事 */
          setOverwrite(false)
        }
      },
      (e: unknown) => setFail({ what: 'export', text: errorText(e) }),
    )
  }

  const confirm = () => {
    if (id === null || plan === null || path === null || busy || settling) return
    setBusy(true)
    setFail(null)
    /* 这一趟的落点先扣下来：成功之后开关再怎么拨，胶囊与页脚那句都照这一份说 */
    const at = path.full
    /* 真会进包 / 真会复制出去的份数（缺的那一份不算） */
    const packed = plan.files.filter((f) => f.exists).length
    api.exportReportArchive(id, dir, zip, overwrite).then(
      (written) => {
        setBusy(false)
        setOverwrite(false)
        /* ★ 不关框、也不加行：成了这件事由图标 / 按钮 / 页脚那一句说（文件头第 3 条） */
        setDone(written.length)
        setLast({ zip, path: at, packed })
        /* 刚下来这一下先僵一会儿：连着点的人那第 N 下不许再落到任何一颗按钮上 */
        settle()
      },
      (e: unknown) => {
        setBusy(false)
        /*
         * 这一趟没成 ⇒ 把上一趟的"已导出"收掉：绿勾、页脚那句、胶囊翻脸说的都是
         * **上一次**的事，而现在屏幕上摆着一条红字 —— 两件事一起看就是自相矛盾。
         * （不这么做的话，撞名那一屏会出现"导出失败"配"已导出 2 个文件"配绿勾。）
         */
        setDone(null)
        setLast(null)
        const err = uiErrorOf(e)
        setFail({ what: 'export', text: err.text })
        /*
         * 撞名（那三份 / 那个包已经在位置上）⇒ 把「重新导出」摆出来。
         * **只认码**（`E_REPORT_EXPORT_EXISTS_001`）—— 不认句子：文案会翻译、
         * 后端也随时可能改措辞，拿字面去匹配是最脆的那种写法。
         */
        if (err.code === EXPORT_EXISTS_CODE) setOverwrite(true)
      },
    )
  }

  /**
   * 打开文件管理器，停在这个落点上（路径由后端自己算）。
   *
   * 两处都用它：成功之后胶囊里那颗「打开位置」、撞名那一屏页脚那颗（那份是别人 /
   * 上一趟留下的，`done` 还是 `null` —— 所以不能拿 `done` 当判据）。
   */
  const revealTarget = () => {
    if (id === null) return
    setFail(null)
    api.revealReportExport(id, dir).catch((e: unknown) => {
      setFail({ what: 'open', text: errorText(e) })
    })
  }

  /** 「打开位置」（成功之后）：只有真导过才开 —— 没导过那儿什么都没有 */
  const openFolder = () => {
    if (done === null) return
    revealTarget()
  }

  /** 撞名那一屏页脚那颗「打开位置」：要去的是**位置上已有的那一份**，与导没导过无关 */
  const openExisting = revealTarget

  /**
   * 信息页那颗「复制路径」：复制的是**归档里那一份的绝对路径**。
   *
   * 剪贴板失败（WKWebView 的权限 / 隐私模式）**不弹错**：路径正文本身就能拖选，
   * 那才是兜底；为一句"没复制上"再顶一条红字不值得（与 `TraceTag` 同一条口径）。
   */
  const copyPath = (kind: ReportExportKind, text: string) => {
    void navigator.clipboard?.writeText(text).then(
      () => {
        setCopied(kind)
        window.setTimeout(() => setCopied((now) => (now === kind ? null : now)), 1400)
      },
      () => {
        /* 复制不了就算了 —— 路径就在那一行里，拖选照样能抄 */
      },
    )
  }

  /**
   * 信息页那颗「定位」：**不导出**，直接让系统文件管理器选中归档里的那一份。
   *
   * 路径由后端自己算（前端只给 `id + kind`）—— 与「打开位置」同一条纪律：
   * 界面永远拿不到"随便打开一个路径"的能力。
   */
  const revealSource = (kind: ReportExportKind) => {
    if (id === null) return
    setFail(null)
    api.revealReportSource(id, kind).catch((e: unknown) => {
      setFail({ what: 'reveal', text: errorText(e) })
    })
  }

  const exported = done !== null

  /** 成功之后胶囊上写哪个落点：这一趟真导出去的那个（开关后来的改动不回溯） */
  const shownPath = exported && last !== null ? last.path : (path?.full ?? '')
  /** 胶囊里那两段字：成功态按"真落点"拆，其余按"待导出的落点"拆 */
  const shown: { dir: string; tail: string } =
    exported && last !== null
      ? { dir: splitPath(last.path).dir, tail: splitPath(last.path).base }
      : { dir: path?.head ?? '', tail: path?.tail ?? '' }

  /**
   * **一行文件** —— 两页共用这一个（作者 2026-11-04 第三件事：信息页要和文件页一样，
   * 只多两颗按钮）。两页因此逐行同构：同一枚图标、同一个中间省略、同一个高度。
   *
   * `where` = 信息页那一版：第二行右边多一截"归档里在哪儿"，再右边多两颗按钮。
   *
   * 缺的那一份（`exists === false`，处理失败的那一次没有「处理后的文件」）：
   * 行照出、灰掉、名字划线（作者：「第三个也显示出来然后灰色划线意思没有这个文件」），
   * 复制路径仍然可用（"它本该在这儿"），定位那颗禁用（盘上没有它）。
   */
  const rowOf = (f: ReportExportFile, where: boolean) => {
    const cut = middleName(f.name)
    /**
     * 盘上没有这一份 ⇒ **两颗都点不了**（作者 2026-11-04：「失败的复制按钮也是灰色的吧？
     * 根本没有这个文件」）。原来只禁了「定位」，复制还能按 —— 抄一串指向不存在文件的路径，
     * 抄到手也没用，反而让人以为那儿真有东西。
     */
    const live = f.exists
    return (
      <li key={f.kind} className={s.fileItem} data-missing={!live || undefined}>
        {/* 写好了就地把纸换成勾：形状占同一格，框一行都不长（缺的那行永远是纸） */}
        <span
          className={s.fileIcon}
          data-done={exported && live}
          data-missing={!live || undefined}
          aria-hidden="true"
        >
          <svg
            width={15}
            height={15}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.7}
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            {exported && live ? CHECK_SHAPE : KIND_ICON[f.kind]}
          </svg>
        </span>
        <span className={s.fileMeta}>
          {/* 名字只占一行：宽了两段拼回原名，挤了中间省略（`middleName`） */}
          <span className={s.fileName} title={f.name}>
            <span className={s.nameHead}>{cut.head}</span>
            <span className={s.nameTail}>{cut.tail}</span>
          </span>
          <span className={s.fileSub}>
            <span className={s.fileKind}>{tr(KIND_LABEL[f.kind])}</span>
            {where && (
              <span className={s.fileWhere} title={f.path}>
                {f.path}
              </span>
            )}
          </span>
        </span>
        {where && (
          <>
            <button
              type="button"
              className={s.srcAct}
              data-done={copied === f.kind}
              disabled={!live}
              aria-label={tr('report.export.copyPath')}
              data-tip={copied === f.kind ? tr('report.export.copied') : tr('report.export.copyPath')}
              onClick={() => copyPath(f.kind, f.path)}
            >
              <Icon name="copy" size={14} />
            </button>
            <button
              type="button"
              className={s.srcAct}
              aria-label={tr('report.export.reveal')}
              data-tip={tr('report.export.reveal')}
              disabled={!live}
              onClick={() => revealSource(f.kind)}
            >
              <Icon name="external" size={14} />
            </button>
          </>
        )}
      </li>
    )
  }

  /**
   * **底下那一簇** —— 两页**共用同一个节点**（作者 2026-11-04 第三件事：
   * 信息页要和文件页一样）。压缩开关 + 保存位置胶囊，`margin-top: auto` 钉在框身下沿。
   *
   * 一个变量喂给两页，两页就不可能长得不一样 —— 这正是他要的那条。
   */
  const footNode =
    plan === null ? null : (
      <div className={s.pageFoot}>
        {/*
         * 「导出时启用压缩」：默认开、用户关过一次就记住（`STORAGE.clientReportExportZip`）。
         * 它决定 `confirm` 真往盘上写什么：一个 `<基名>.zip`，还是散着复制三份。
         */}
        <div className={s.zipRow}>
          <span className={s.zipText}>
            <span className={s.zipTitle}>{tr('report.export.zip')}</span>
            <span className={s.zipHint}>{tr('report.export.zipHint')}</span>
          </span>
          {/* 开关是共用件（`components/switch`）：受控，状态住在上面那一格（`zip`）。
              拨一下 = 「下一趟要什么样」⇒ 走 `chooseZip`（成功态退回"待导出"）。 */}
          <Switch checked={zip} onChange={chooseZip} ariaLabel={tr('report.export.zip')} />
        </div>

        {path !== null && (
          /*
           * 整条是**显示**不是按钮：只有右边那颗改位置 / 开位置（作者 2026-10-11）。
           * 路径正文因此还能拖选、误点一下也不会弹系统框。
           */
          <div className={s.pathPill} data-tip={shownPath}>
            <svg
              className={s.pathIcon}
              width={15}
              height={15}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.7}
              aria-hidden="true"
            >
              <path d="M3.5 6.5A2 2 0 0 1 5.5 4.5h4l2 2.5h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
            </svg>
            <span className={s.pathText}>
              <span className={s.pathHead}>{shown.dir}</span>
              <span className={s.pathTail}>{shown.tail}</span>
            </span>
            {/*
             * ★ 成了之后这一颗**就地翻成「打开位置」**（作者：都导出成功了，
             *   位置已经定了，那一刻要的是"去那儿看看"）—— 同一格、同一尺寸，
             *   框的宽窄一动不动。
             */}
            <button
              type="button"
              className={s.pathEdit}
              data-open={exported}
              onClick={exported ? openFolder : changeDir}
            >
              {exported ? tr('report.export.open') : tr('report.export.change')}
            </button>
          </div>
        )}
      </div>
    )

  return (
    <Modal
      open={open}
      size="md"
      title={tr('report.export.title')}
      onClose={onClose}
      footer={
        <>
          {/*
           * 左下角那一簇：**翻页那颗「信息」** 恒定在最左（`margin-right: auto`），
           * 成了之后「已导出 N 个文件」跟在它右边 —— 两态页脚高度不变（按钮撑着）。
           */}
          <span className={s.footLeft}>
            <button
              type="button"
              className={s.tabToggle}
              aria-pressed={tab === 'info'}
              onClick={() => chooseTab(tab === 'files' ? 'info' : 'files')}
            >
              <InfoGlyph />
              {tab === 'files' ? tr('report.export.info') : tr('report.export.files')}
            </button>
            {exported && (
              <span className={s.footNote}>
                {/*
                 * 口径照**这一趟真导成的那一档**说（`last`），不跟开关飘：
                 * 压缩那一档说"一个包，含 N 个文件"（包里其实是三份，别只说 1 个）；
                 * 散着导出的那一档才数份数 —— 而且数的是**真写出去的份数**（`done`）。
                 */}
                {last?.zip === true
                  ? tr('report.export.doneZip', { n: last.packed })
                  : tr('report.export.done', { n: done })}
              </span>
            )}
          </span>
          {/*
           * ★ 2026-11-04 第二轮（作者：「成功导出之后的已导出了之后我希望确认导出按钮灰色吧，
           *   切换了开关才变亮」）—— **同一份文件不必再导一次**：
           *
           *   · 导成功之后（且没拨开关 / 没换位置）：这颗灰着，页脚左边那句「已导出…」
           *     就是它为什么灰；
           *   · 拨一下压缩开关、或换个保存位置 ⇒ 落点变了、要导的是另一份 ⇒ 重新亮起来；
           *   · 撞名那一屏（`overwrite`）它写「重新导出」，是用户**明确选择**覆盖，照样可点。
           *
           * 灰着而不是消失：位置不变、仍是这一屏的主动作，用户知道"已经导过了"。
           */}
          {/*
           * 「取消」也一起僵：刚成功那一瞬整条出口都别接点击（作者被"手一滑点到完成、
           * 框关了"坑过）。关框还有右上角那颗 ×。
           */}
          {/* 撞名时这一颗让位给「打开位置」（同一格，页脚宽度不变），见下 */}
          {!(overwrite && !exported) && (
            <Btn onClick={onClose} disabled={busy || settling}>
              {tr('report.export.cancel')}
            </Btn>
          )}
          {/*
           * 撞名那一屏：第二颗从「取消」翻成「打开位置」（2026-11-04 作者问「位置已有怎么办」）。
           *
           * 先去那儿看看那份是什么 —— 可能是同一趟导出的，也可能他自己改过。
           *
           * ★ 不许改变页脚宽度：原来这颗是**新加**的一颗，一出现页脚就长一截、框宽跟着变
           *   （作者一眼就看出来了）。现在它**顶替**「取消」那一格 —— 两颗按钮的个数、
           *   顺序、位置都不变，只是中间那颗换了字。关框仍有右上角的 ×。
           */}
          {overwrite && !exported && (
            /* 同一格、同一顺序（第二位）：宽度与撞名前逐像素一致 */
            <Btn onClick={openExisting} disabled={busy || settling}>
              {tr('report.export.open')}
            </Btn>
          )}
          <Btn
            variant="accent"
            /* 导过了就不再接受这一下（除撞名那一屏：那是"重新导出"的选择） */
            disabled={busy || settling || plan === null || (exported && !overwrite)}
            className={busy || settling ? s.btnBusy : undefined}
            onClick={confirm}
          >
            {busy || settling ? (
              /*
               * 刚下来这 900ms 与正在导用的是同一句「导出中」：
               * 两处都不许说"已导出"（那一刻页脚已经在说，且盘上确实写好了），
               * 也不许说"正在导出…"（那是在撒谎，文件已经落下了）——
               * 统一成最朴素的一句「导出中」，宽度也与「确认导出」齐平。
               */
              <>
                <Spinner />
                {tr('report.export.busy')}
              </>
            ) : overwrite ? (
              tr('report.export.overwrite')
            ) : (
              tr('report.export.confirm')
            )}
          </Btn>
        </>
      }
    >
      <div className={s.body}>
        {/*
         * 「处理失败」徽章（2026-11-04）—— 挂在**框身最外层**而不是某一页里：
         * 它 portal 到标题右边那一格，翻到信息页时也不该消失（两页说的是同一件事）。
         * 判据只有一条：计划里有哪一份 `exists === false`。
         */}
        {plan !== null && plan.files.some((f) => !f.exists) && (
          <HeadBadge label={tr('report.export.failed')} />
        )}

        {/*
         * 量尺（2026-10-31）：一副**与终态同构、看不见**的空架子，永远垫在内容底下。
         * 它撑出的是"三行文件 + 保存位置胶囊"该有的高度，于是这一块的高度与
         * 「有没有读到计划」无关 —— 打开的那一刻就是终态，一秒之后也不会再长。
         *
         * 为什么不是写死 `min-height`：这里的字号是 `clamp()` 跟着窗口走的
         * （`--fs-body` 12~14、`--fs-label` 10~12.5），写死像素只会在某个窗口
         * 宽度上刚好，别的宽度上照样差几像素。叠一层真 DOM 就分毫不差。
         *
         * 它不画、不读屏、不吃鼠标（`.gauge` 那三条），也不导出 —— 只是布局。
         */}
        <div className={s.gauge} aria-hidden="true">
          <div className={s.pageTop}>
            <ul className={s.fileList}>
              {GAUGE_ROWS.map((kind) => (
                <li key={kind} className={s.fileItem}>
                  <span className={s.fileIcon} />
                  <span className={s.fileMeta}>
                    {/* 空 span 没有行盒（高度 0），一格 `&nbsp;` 才撑得出与真行一样的高度。
                        结构照真行摆（名字两段 + 第二行），多出来的那截路径/按钮不参与高度 */}
                    <span className={s.fileName}>
                      <span className={s.nameHead}>{NBSP}</span>
                    </span>
                    <span className={s.fileSub}>
                      <span className={s.fileKind}>{NBSP}</span>
                    </span>
                  </span>
                </li>
              ))}
            </ul>
          </div>
          {/* 底下那一簇也照摆一副：压缩开关两行字 + 那颗 36×20 的钮，再是胶囊 */}
          <div className={s.pageFoot}>
            <div className={s.zipRow}>
              <span className={s.zipText}>
                <span className={s.zipTitle}>{NBSP}</span>
                <span className={s.zipHint}>{NBSP}</span>
              </span>
              <span className={s.zipKnob} />
            </div>
            <div className={s.pathPill}>
              <span className={s.pathText}>
                <span className={s.pathTail}>{NBSP}</span>
              </span>
              <span className={s.pathEdit}>{NBSP}</span>
            </div>
          </div>
        </div>

        {/*
         * 第一页：三件套清单 + 保存位置（与量尺同住一格，见 `ExportModal.module.css`）。
         * 翻到「信息」时它只是**藏起来**（`visibility: hidden`）—— 仍占格，于是两页同高。
         */}
        <div
          className={`${s.page} ${tab === 'files' ? '' : s.pageOff}`}
          aria-hidden={tab !== 'files'}
        >
          {/* 上面那一截：清单（或"正在读…"）。底下那一簇靠 `margin-top: auto` 钉在框身下沿 */}
          <div className={s.pageTop}>
            {/* 提示一律进标题右边那一格（`HeadNote`，2026-10-17 收口）：
                摆在框身里会把这个框撑高，下面那张文件清单跟着往下跳。
                ★ 2026-10-19：**只留这一处** —— 下面原来还给 `planErr` 摆了一份红字，
                同一条错同一个框里出现两遍（作者看见了那句重复）。 */}
            {planErr !== null && (
              <HeadNote text={tr('report.export.planErr', { err: planErr })} tone="error" />
            )}
            {fail !== null && (
              <HeadNote
                text={`${tr(
                  fail.what === 'export'
                    ? 'report.export.failExport'
                    : fail.what === 'open'
                      ? 'report.export.failOpen'
                      : 'report.export.failReveal',
                )}${fail.text}`}
                tone="error"
              />
            )}
            {plan === null && planErr === null && (
              <p className={s.filling}>{tr('report.export.loading')}</p>
            )}

            {plan !== null && (
              <ul className={s.fileList}>{plan.files.map((f) => rowOf(f, false))}</ul>
            )}
          </div>

          {/* 底下那一簇：两页共用同一个节点（`footNode`），所以两页连"已导出 / 打开位置"都一致 */}
          {footNode}
        </div>

        {/*
         * 第二页：信息（2026-11-03）。作者 2026-11-04 第三件事的定版：
         *
         * > 信息页面也是一样的，不要现在这样，要和文件页面一样的，就是多了按钮
         *
         * ⇒ **与文件页同一个骨架**：左边同一枚图标（成功换成同一个绿勾）、同一个文件名、
         *   同一个种类小字；多出来的只是右边那两颗（复制路径 / 定位）。
         *   原来那一版是"一列路径"，看着像另一张表 —— 两页之间只剩文件名是对得上的。
         *
         * 底下那一簇**一模一样**（压缩开关 + 保存位置胶囊），也**一模一样的成功态**：
         * 导过之后这一页同样显示"已导出 / 打开位置"（作者第二件事：「已经导出过就可以
         * 切换到信息页面，显示已经导出过」）。
         */}
        <div
          className={`${s.page} ${tab === 'info' ? '' : s.pageOff}`}
          aria-hidden={tab !== 'info'}
        >
          <div className={s.pageTop}>
            {/* 与文件页同一个 `rowOf`，只多两处：第二行接一截归档路径、右边两颗按钮 */}
            {plan !== null && (
              <ul className={s.fileList}>{plan.files.map((f) => rowOf(f, true))}</ul>
            )}
          </div>

          {/* 底下那一簇：与文件页**是同一个节点**（`footNode`）—— 连"已导出 / 打开位置"都一致 */}
          {footNode}
        </div>
      </div>
    </Modal>
  )
}
