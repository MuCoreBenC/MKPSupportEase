/*
 * 钩子那一趟的那一屏 —— 切片器导出 G-code 时，**常驻的那扇窗**上弹出来的就是它。
 *
 * 形状照旧世代 `mkp-ssr` 的进度窗（窗口常驻、结果留到下一次顶掉它），所以：
 *
 *   1. **跑着的时候关不掉**：没有 ×、点遮罩也不关 —— 那一趟还在跑，藏起来只会让人以为
 *      出事了。要收起来只有一条路：按「后台运行」（那是明确说过的意思，而且收起来之后
 *      由外壳那条窄条兜着，见 `PostProcessStrip.tsx`）。
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

import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'

import { api, errorText } from '../../api'
import type { PostProcessFacts, PostProcessFinished, PostProcessResult } from '../../api/contract'
import { detectPlatform } from '../../hooks/usePlatform'
import { Btn } from '../ui/Controls'
import { Modal } from '../../components/modal'
import { useTitlebarDrag } from '../useTitlebarDrag'
import { percentOf, stepName } from './steps'
import type { PostProcessRunView } from './usePostProcessRun'
import s from './PostProcessModal.module.css'

/* 顶栏那一条的高度按平台来（与 `TopTabs` 同一对 token）：拖窗带要正好铺在它上面 */
const PLATFORM = detectPlatform()

export interface PostProcessModalProps {
  /** 那一趟的现况（`usePostProcessRun` 给的那一份；App 拿着它，后台那条窄条也要） */
  view: PostProcessRunView
  /** 「查看详情」：跨页跳转的出口由外壳给（页面 / 浮层不该伸手改页签） */
  onOpenReport: () => void
}

/* ---- 小工具 ---- */

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
        label: '耗材 / 喷嘴',
        value: `${facts?.filamentType ?? '—'} · ${mmText(facts?.nozzleDiameter ?? 0)} mm`,
      },
      { label: '层高', value: `${mmText(facts?.layerHeight ?? 0)} mm` },
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
  const rows: FactRow[] = []
  if (facts?.slicerPrintTimeSeconds !== undefined) {
    rows.push({ label: '预估打印时间', value: printTimeText(facts.slicerPrintTimeSeconds) })
  }
  rows.push({ label: '文件大小', value: facts === null ? '—' : sizeText(facts.fileSizeBytes) })
  if (facts?.glueSpeed !== undefined) {
    rows.push({ label: '涂胶速度', value: speedText(facts.glueSpeed) })
  }
  if (facts?.slicer !== undefined) rows.push({ label: '切片软件', value: facts.slicer, grey: true })
  return { title: '切片与预估', rows }
}

/** 跑完：右栏是「擦料与胶水消耗」——结果读数有哪几格就摆哪几格 */
function doneGroup(finished: PostProcessFinished, facts: PostProcessFacts | null): FactGroup {
  const res = finished.result
  const rows: FactRow[] = []
  /* 打印时间**常常是后到的**（有界面时延后算）：先空着，`postprocess-print-time` 到了再补；
   * 失败 / 取消时退回开场那一格（切片器自报的）—— 有就摆，比空着强 */
  const printSec = res?.printTimeSeconds ?? facts?.slicerPrintTimeSeconds
  if (printSec !== undefined) {
    rows.push({ label: '预估打印时间', value: printTimeText(printSec) })
  }
  if (res?.glueLengthMm !== undefined) {
    rows.push({ label: '涂胶总长', value: mm2(res.glueLengthMm) })
  }
  /* 这一格真机现在就有：处理耗时 = finished.elapsedMs，大小 = 开场那几个事实里的字节数 */
  rows.push({
    label: '处理耗时 / 大小',
    value: `${(finished.elapsedMs / 1000).toFixed(2)} 秒 / ${
      facts === null ? '—' : sizeText(facts.fileSizeBytes)
    }`,
  })
  /* 切片软件：跑完给带版本的完整串；失败 / 取消时退回开场那个名字（总比空着强） */
  const slicer = res?.slicer ?? facts?.slicer
  if (slicer !== undefined) rows.push({ label: '切片软件', value: slicer, grey: true })
  return { title: '擦料与胶水消耗', rows }
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

function ProgressHero({ percent, step, eta }: { percent: number; step: string; eta: string | null }) {
  return (
    /* 不给 role="status"：这一块每秒都在动（百分比 / 倒计时），朗读器会被念个不停 */
    <div className={`${s.hero} ${s.heroSide}`}>
      <span className={s.heroNum}>{Math.round(percent)}%</span>
      <div className={s.heroMeta}>
        <span className={s.heroStep}>{step}</span>
        <span className={s.heroEta}>{eta ?? '正在估算…'}</span>
      </div>
      {/* 进度条是定稿皮（pb1「凹槽条」：绿底上凿出来的一道槽），不是普通白条 */}
      <div className={`${s.heroBar} ${s.heroBarGroove}`}>
        <span className={s.heroBarFill} style={{ width: `${Math.min(100, Math.max(0, percent))}%` }} />
      </div>
    </div>
  )
}

/* ---- ② 机型不匹配：红警告 hero（空心圈 + 标题 + 后端那一句问话） ---- */

function WarnHero({ text }: { text: string }) {
  return (
    <div className={`${s.warnHero} ${s.warnSoft}`} role="status">
      <span className={s.warnGlyph} aria-hidden="true">
        <span className={s.warnBadge}>!</span>
      </span>
      <div className={s.warnCopy}>
        <span className={s.warnTitle}>预设与机型不匹配</span>
        {/* 正文就是后端那一句问话（`question.text`）—— 不另编文案 */}
        <span className={s.warnLine}>{text}</span>
      </div>
    </div>
  )
}

/* ---- ③ 成功：四格指标条（笔偏移 X/Y/Z + 涂胶速度） ---- */

/** 四格齐（三轴偏移 + 涂胶速度）才摆 —— 缺格的时候这条空着比缺一角好 */
function metricsReady(
  res: PostProcessResult | undefined,
): res is Required<Pick<PostProcessResult, 'penX' | 'penY' | 'penZ' | 'glueSpeed'>> {
  return (
    res !== undefined &&
    typeof res.penX === 'number' &&
    typeof res.penY === 'number' &&
    typeof res.penZ === 'number' &&
    typeof res.glueSpeed === 'number'
  )
}

function MetricsStrip({
  res,
}: {
  res: Required<Pick<PostProcessResult, 'penX' | 'penY' | 'penZ' | 'glueSpeed'>>
}) {
  const cells: Array<[string, string, boolean?]> = [
    ['笔偏移 X', mm2(res.penX)],
    ['笔偏移 Y', mm2(res.penY)],
    ['笔偏移 Z', mm2(res.penZ)],
    ['涂胶速度', speedText(res.glueSpeed), true],
  ]
  return (
    <div className={`${s.strip} ${s.stripFinal}`} role="status">
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
  const [err, setErr] = useState<string | null>(null)
  /* 「预计还需」每秒重算一次（这个表只在跑着的时候挂着） */
  const [, setTick] = useState(0)
  /* 跑着的时候顶上那条拖窗带用它（与标题栏同一条动作，见文件头第 1 条下面那段） */
  const drag = useTitlebarDrag()

  const finished = run?.finished ?? null
  const running = run !== null && finished === null

  useEffect(() => {
    if (!running) return
    const id = window.setInterval(() => setTick((n) => n + 1), 1000)
    return () => window.clearInterval(id)
  }, [running])

  if (run === null || view.dismissed) return null

  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)
  const asking = running && run.question !== null
  const eta = running ? etaText(view.sinceMs, percent) : null
  const facts = run.facts

  /** 两条命令（停止 / 答复）都要等后端，失败要说出来 —— 界面不许"点了没反应" */
  const act = (fn: () => Promise<void>) => {
    setBusy(true)
    setErr(null)
    void fn()
      .catch((e: unknown) => setErr(errorText(e)))
      .finally(() => setBusy(false))
  }

  /*
   * 顶部那一块（跑着 hero / 问句警告 / 结果 hero）——四种长相收成一个变量，
   * 下面那句 facts 与警告行是它们**共用**的尾巴。
   */
  let lead: ReactNode = null
  if (asking && run.question !== null) {
    lead = <WarnHero text={run.question.text} />
  } else if (running) {
    lead = (
      <ProgressHero
        percent={percent}
        step={run.progress === null ? '准备中' : stepName(run.progress.step)}
        eta={eta}
      />
    )
  } else if (finished !== null) {
    if (finished.ok) {
      lead = metricsReady(finished.result) ? <MetricsStrip res={finished.result} /> : null
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
      title={running ? '后处理进度' : '后处理结果'}
      /* 文件名那一格原来是标题右边的 `s.files` —— 通用框有 `subtitle`，用它 */
      subtitle={run.gcodeName}
      /* 跑着的时候点遮罩不关（见文件头第 1 条）；要收起来请按「后台运行」 */
      closeOnScrim={!running}
      /* 跑着的时候连 × 都不摆 —— 那一刻没有"关掉"这个动作。
         结果那几扇也不摆（第 16~17 轮定的）：收窗走页脚「关闭」那一条明路 */
      hideClose
      /* Esc 也一样收进那条口径：跑着的时候"收起来"只有「后台运行」这一条明路 */
      closeOnEsc={!running}
      onClose={dismiss}
      footer={
        asking ? (
          /* 那一问的两颗按钮：**强制继续不强调、终止占强调位**（第 9~11 轮定的）——
             「终止」就是回答"不跑"那一条（与「停止」同一个落点，见契约里那条注释）。
             按钮皮肤是全站那一套（`.btn` 默认胶囊），这里不加任何 className */
          <>
            <Btn disabled={busy} onClick={() => act(() => api.answerPostProcessMismatch(true))}>
              强制继续
            </Btn>
            <Btn
              variant="accent"
              disabled={busy}
              onClick={() => act(() => api.answerPostProcessMismatch(false))}
            >
              终止
            </Btn>
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
        {facts !== null && (
          <FactsGroups
            groups={
              asking || running
                ? [baseGroup(facts), runGroup(facts)]
                : [baseGroup(facts), doneGroup(finished as PostProcessFinished, facts)]
            }
          />
        )}
        {finished !== null &&
          finished.warnings.map((w) => (
            <p className={s.warn} key={w}>
              警告：{w}
            </p>
          ))}
        {err !== null && (
          <p className={s.bad} role="alert">
            {err}
          </p>
        )}
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
   * 不跑的那一态（结论留在屏上）**退回普通模态框**：那时窗控键照旧点得着 ——
   * 看完结果顺手关窗是正常动作，不该拦。
   */
  return (
    <>
      <div className={s.layer} data-full={running}>
        {screen}
      </div>
      {running && (
        <div
          className={s.dragShield}
          data-platform={PLATFORM}
          aria-hidden="true"
          onMouseDown={drag.onMouseDown}
          onDoubleClick={drag.onDoubleClick}
        />
      )}
    </>
  )
}
