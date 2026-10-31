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
 */

import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'

import { api, errorText } from '../../api'
import type { ReportExportKind, ReportExportPlan } from '../../api'
import { STORAGE } from '../../api/storageKeys'
import { HeadNote, Modal } from '../../components/modal'
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
 * 路径胶囊里的那串字：`<父目录><分隔符><基名><分隔符>` 拆两段 ——
 * 前段自己收省略号、**尾两段钉住不缩**（"往哪个盘哪个文件夹里导"永远看得见）。
 *
 * 分隔符按父目录里用的那个认（Windows 收到的是 `\`，macOS 是 `/`）；
 * 段数少到没有"尾两段"可言时整条当前段，不分。
 */
function pathPill(
  dir: string,
  base: string,
): { folder: string; full: string; head: string; tail: string } {
  const sep = dir.includes('\\') ? '\\' : '/'
  const folder = `${dir.replace(/[\\/]+$/, '')}${sep}${base}`
  const full = `${folder}${sep}`
  const lead = full.startsWith(sep) ? sep : ''
  const parts = full.split(/[\\/]/).filter((x) => x !== '')
  if (parts.length <= 2) return { folder, full, head: '', tail: full }
  return {
    folder,
    full,
    head: lead + parts.slice(0, -2).join(sep) + sep,
    tail: parts.slice(-2).join(sep) + sep,
  }
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
   * 框里那句话：`what` 决定前缀（导出 / 打开各说各的），`text` 是后端原话。
   * 两种都**留在框里、不关框**：用户就着这句话改位置 / 先把那份文件挪走，就地重来。
   */
  const [fail, setFail] = useState<{ what: 'export' | 'open'; text: string } | null>(null)
  /** 成了：出了几份（框留着，见文件头第 3 条）。`null` = 还没导 */
  const [done, setDone] = useState<number | null>(null)
  /* 上一个选过的保存位置；空串 = 还没选过 ⇒ 用后端给的默认值（`<用户根>/exports`） */
  const [picked, setPicked] = useStickyState<string>(STORAGE.clientReportExportDir, '')

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
  const path = plan === null || dir === '' ? null : pathPill(dir, plan.base)

  /** 「更改位置」：系统目录选择器。取消（`null`）保持原来那个位置，不是错误 */
  const changeDir = () => {
    void api.pickExportDir(dir).then(
      (next) => {
        if (next !== null) {
          setPicked(next)
          /* 换了个位置就是"再导一份"这件事重新开始：上一次那句"已导出"不再成立 */
          setDone(null)
        }
      },
      (e: unknown) => setFail({ what: 'export', text: errorText(e) }),
    )
  }

  const confirm = () => {
    if (id === null || plan === null || path === null || busy) return
    setBusy(true)
    setFail(null)
    api.exportReportArchive(id, dir).then(
      (written) => {
        setBusy(false)
        /* ★ 不关框、也不加行：成了这件事由图标 / 按钮 / 页脚那一句说（文件头第 3 条） */
        setDone(written.length)
      },
      (e: unknown) => {
        setBusy(false)
        setFail({ what: 'export', text: errorText(e) })
      },
    )
  }

  /** 「打开位置」：打开系统文件管理器，停在那次导出的那个目录（路径由后端自己算） */
  const openFolder = () => {
    if (id === null || done === null) return
    setFail(null)
    api.revealReportExport(id, dir).catch((e: unknown) => {
      setFail({ what: 'open', text: errorText(e) })
    })
  }

  const exported = done !== null

  return (
    <Modal
      open={open}
      size="md"
      title={tr('report.export.title')}
      onClose={onClose}
      footer={
        exported ? (
          /* 成的那一屏只有一条出口：看完就去干活（页脚左边那句是这一态的主语） */
          <>
            <span className={s.footNote}>{tr('report.export.done', { n: done })}</span>
            <Btn variant="accent" onClick={onClose}>
              {tr('report.export.finish')}
            </Btn>
          </>
        ) : (
          <>
            <Btn onClick={onClose}>{tr('report.export.cancel')}</Btn>
            <Btn variant="accent" disabled={busy || plan === null} onClick={confirm}>
              {tr('report.export.confirm')}
            </Btn>
          </>
        )
      }
    >
      <div className={s.body}>
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
          <ul className={s.fileList}>
            {GAUGE_ROWS.map((kind) => (
              <li key={kind} className={s.fileItem}>
                <span className={s.fileIcon} />
                <span className={s.fileMeta}>
                  {/* 空 span 没有行盒（高度 0），一格 `&nbsp;` 才撑得出与真行一样的高度 */}
                  <span className={s.fileName}>{NBSP}</span>
                  <span className={s.fileKind}>{NBSP}</span>
                </span>
              </li>
            ))}
          </ul>
          <div className={s.pathPill}>
            <span className={s.pathText}>
              <span className={s.pathTail}>{NBSP}</span>
            </span>
            <span className={s.pathEdit}>{NBSP}</span>
          </div>
        </div>

        {/* 真正的几块：与量尺同住一格（见 `ExportModal.module.css` 的 `.body`） */}
        <div className={s.live}>
          {/* 提示一律进标题右边那一格（`HeadNote`，2026-10-17 收口）：
              摆在框身里会把这个框撑高，下面那张文件清单跟着往下跳。
              ★ 2026-10-19：**只留这一处** —— 下面原来还给 `planErr` 摆了一份红字，
              同一条错同一个框里出现两遍（作者看见了那句重复）。 */}
          {planErr !== null && (
            <HeadNote text={tr('report.export.planErr', { err: planErr })} tone="error" />
          )}
          {fail !== null && (
            <HeadNote
              text={`${tr(fail.what === 'export' ? 'report.export.failExport' : 'report.export.failOpen')}${fail.text}`}
              tone="error"
            />
          )}
          {plan === null && planErr === null && (
            <p className={s.filling}>{tr('report.export.loading')}</p>
          )}

          {plan !== null && (
            <ul className={s.fileList}>
              {plan.files.map((f) => (
                <li key={f.kind} className={s.fileItem}>
                  {/* 写好了就地把纸换成勾：形状占同一格，框一行都不长 */}
                  <span className={s.fileIcon} data-done={exported} aria-hidden="true">
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
                      {exported ? CHECK_SHAPE : KIND_ICON[f.kind]}
                    </svg>
                  </span>
                  <span className={s.fileMeta}>
                    <span className={s.fileName}>{f.name}</span>
                    <span className={s.fileKind}>{tr(KIND_LABEL[f.kind])}</span>
                  </span>
                </li>
              ))}
            </ul>
          )}

          {plan !== null && plan.missing.length > 0 && (
            <p className={s.notFull}>
              {tr('report.export.missing', {
                names: plan.missing.map((k) => `「${tr(KIND_LABEL[k])}」`).join('、'),
              })}
            </p>
          )}

          {path !== null && (
            /*
             * 整条是**显示**不是按钮：只有右边那颗改位置 / 开位置（作者 2026-10-11）。
             * 路径正文因此还能拖选、误点一下也不会弹系统框。
             */
            <div className={s.pathPill} data-tip={path.full}>
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
                <span className={s.pathHead}>{path.head}</span>
                <span className={s.pathTail}>{path.tail}</span>
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
      </div>
    </Modal>
  )
}
