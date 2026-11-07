/**
 * 「生成与发布」页 —— C14 版式移植（feat/b05-14b-c14-port，build 视角落地）。
 *
 * C13 的两页在这一页合成一页（作者：「发布中心和整包检查与生成有点重复了」），
 * 版式沿用左右分布（作者：「左边是那些事项、未完成的提示，右边才是主要操作」）：
 *
 *   左栏  检查        一份账（wb_preflight）：能不能生成 + 交出去会不会缺东西
 *   右栏  ① 生成      勾中版本按已保存的配方生成（原子整批，字节没变不重写）
 *         ② 发布      点它先开**发布闸**（第二刀，十五项逐项打勾，全绿才给往下走）
 *         ③ 基线      产物 vs 对照基线，人看过 diff 才同步（14.9）
 *         ④ 残留      交付目录里「不在本次交付集合内」的文件，清理走 .trash 回收
 *         ⑤ 回收站    删掉的生成快照（只读清单）
 *         ⑥ 子目录职责  workbench/ 五个子目录谁写谁读（14.7，后端给的一句话）
 *
 * # 闸门按钮（14.2）与后端硬闸的行为一致
 *
 * 生成 / 发布按钮的禁用读 `wb_preflight` 的阻断计数（每次写完外壳重取），
 * 后端在 `wb_generate` / `wb_publish` 开头**还会再查一遍**—— 前端只是把同一份
 * 判据提前画在按钮上，不是第二条闸。
 *
 * # 发布闸（第二刀）是**第三层**，但它也不判
 *
 * 点②的「发布」先进 [`PublishGateModal`]：十五项逐项打勾（`wb_publish_audit`）。
 * 那一份结果**整份来自 Rust**（`audit::publish_audit`，与 `cargo test` 判据同一个函数）——
 * 这一页**没有**第二套检查逻辑。三层先后是：按钮禁用（preflight）→ 发布闸（audit）
 * → `wb_publish` 内部那两道硬闸。后一层永远比前一层严，「全绿才发」这条规矩才不会
 * 变成一句标语。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的
 *
 *  - **包版本不摆格子**：上游整层删掉之后它没有来源（`PublishMeta` 里只有 channel
 *    是发布常量），照实留空，也不造一个输入框出来。
 *  - **最低客户端版本有来源了（第三刀），但它不是这一页的输入框**：它由结构规则表
 *    登记（`presets/structure-signatures.toml` → 当前结构签名 → 最低正式客户端版本），
 *    发布闸 ⑫ 会在发布**之前**把它摆出来，发布回执里再报一次。人在这里不该手填它 ——
 *    那是产品决定，落点在规则表里（单一来源）。
 *  - **检查只有一份账**：产品的 wb_preflight 是一份报告（引用 / 参数 / 唯一性 /
 *    孤儿 + 清单↔配方对齐），不像原型分 checkIssues / publishIssues 两份。
 *  - **行上没有「未保存」档**：BuildRow 不带 savedYet；未保存时按钮先走
 *    「保存再生成」（一个按钮两个动作，顺序规定死 —— 机型页同款）。
 *  - **没有内容指纹**：原型的 inputsHash 在产品侧没有对应物；产物的新旧由
 *    每一版自己的快照比对判（BuildRow.state），本页不重复一个全局哈希。
 *  - **恢复配方（wb_revert_preview）不在这页**：按版本走机型页的版本卡 ——
 *    恢复的是「某一版」的配方，不是整包的动作。
 *
 * # 交付文件摸得着（P6 起，2026-10-07 重定口径）
 *
 * ② 卡给的是**真实交付物**，名单与存在与否都来自后端那**同一份交付集合**
 * （`delivery_expected_set` —— 发布闸判残留用的就是它，界面不另拼一份"大概有这些"）：
 *
 *  - 产物 `mkp/presets/*.toml` 只留一枚概览 chip（`preset.toml × N 份`）——
 *    **逐份正文在「生成前确认」里看**（点「生成」先过那一屏）。作者 2026-10-07：
 *    「这上面这一个我觉得没必要了呀，因为我在生成的地方就可以查看了」，
 *    所以这一块不再长出一排「查看 TOML」。
 *  - 其余**不带版本概念**的附属文件（`content/*.json`、`catalog.json`、
 *    `manifest.json`、`source.json`、`release.json`）一份一行，标出谁写的
 *    （生成时重算 / 发布时定稿 / 软件发布链）与在不在盘上，点「查看」读**盘上原文**
 *    （`wb_delivery_file` 直读，不是重新渲染一份）。
 *
 * **旧的「说明书 JSON + 模拟云端」那一套 2026-10-04 退役**（`clientPackage.ts` /
 * `cloud.ts` / `compat.ts` / `fixtures/cloud-presets.json`）：它是一份假的云端数据结构，
 * 客户端根本不读，而且它的路径（`presets/mkp/…`）与真实发布契约（`catalog.path`）
 * 已经冲突 —— 留着只会让人误以为那种路径还合法。**没有迁移成兼容结构。**
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  BaselineDiffEntry,
  BookView,
  Boot,
  DeliveryStatus,
  FinalizePreview,
  GenerateReport,
  Issue,
  IssueReport,
  IssueWhere,
  MirrorSync,
  PreviewReport,
  PublishTxReport,
  ReleaseHistory,
  ReleaseLedger,
  VersionCell,
  VersionIdentity,
  Words,
} from '../api'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import { locateAnchor } from '../c14/locate'

import { releaseStageText as RELEASE_STAGE_TEXT } from '../c14/labels'
import {
  displayName,
  fmtSize,
  groupSummary,
  groupTodo,
  sortFiles,
  stateText,
  stateTone,
  type StateTone,
} from './deliveryStatus'
import GenerateDiffModal from './GenerateDiffModal'
import DeliveryFilesModal from './DeliveryFilesModal'
import FinalizeModal from './FinalizeModal'
import PublishGateModal from './PublishGateModal'
import ReleaseGateModal from './ReleaseGateModal'
import HistoryModal from './HistoryModal'
import GenHistoryModal from './GenHistoryModal'
import s from '../c14.module.css'

interface Props {
  boot: Boot
  book: BookView
  words: Words
  /** 外壳每次写完重取的检查报告 —— 闸门按钮与左栏都读它 */
  report: IssueReport | null
  /** 外壳的「后端状态变过了」计数：生成 / 撤销之后重取基线与残留 */
  tick: number
  onGoto: (view: string, focus?: GotoFocus) => void
  /** 先落盘再生成的那个「落盘」 */
  onSave: () => Promise<boolean>
  /**
   * 生成 / 发布**写了盘上产物**之后让外壳重取：整本（buildRows 的状态）+ 检查报告
   * + 交付读数（①卡那 9 行与「N 组有变化」）。三样缺一样，同一屏就会给出两个时点。
   */
  onArtifactsRefresh: () => void
  /**
   * 重取检查报告（外壳那份账）。修复动作（如内嵌目录重算）做完要立刻重取 ——
   * 不然待办还挂在列表上，人会以为没生效。
   */
  onReportRefresh: () => void
}

/** 「去哪儿」的映射本体（检查项与交付问题共用一份，不各写一套） */
function gotoOfWhere(at: IssueWhere): { view: string; focus?: GotoFocus } | null {
  switch (at.view) {
    case 'params':
    case 'fields':
      return { view: 'params', focus: { machineId: at.machineId ?? '', uid: at.uid, key: at.key } }
    case 'menu':
      return { view: 'bundles', focus: { machineId: null, uid: at.uid, key: null } }
    case 'stock':
      return { view: 'assets', focus: { machineId: null, uid: at.uid, key: null } }
    case 'fallback':
      return { view: 'assets', focus: { machineId: null, uid: null, key: null } }
    default:
      return null // build 视角的问题在本页就地滚过去，不跳页
  }
}

/** 检查项的「去处理」映射。产品的一份账里，不是每一条都有地方可去 */
function gotoOf(i: Issue): { view: string; focus?: GotoFocus } | null {
  if (i.id === 'bundle.orphan_files') {
    /* C14 第二十七轮的修正：孤儿文件去**资产库**并替人筛好「可选」——
       套餐页里根本没有那些文件，跳过去等于没指路 */
    return { view: 'assets', focus: { machineId: null, uid: null, key: 'optional' } }
  }
  return gotoOfWhere(i.at)
}

/**
 * 后端记录的是 **UTC ISO 串**（`2026-10-02T16:30:09Z`，刻意跨时区一致），
 * 给人看要转**本机时区**（作者 2026-10-03：「应该用东八区的时间，或者电脑的时区」）。
 * 解析不动就原样回（老记录可能不是 ISO）。
 */
function localStamp(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

/**
 * 一格版本身份：有值就等宽显示，读不到就把**原因**写在值的位置（不写「未知」）；
 * 有出处就把出处标在下面，有落点就给一颗「打开位置」—— 没有落点的不摆按钮。
 */
function IdentityCell({ label, cell }: { label: string; cell?: VersionCell }) {
  const text = cell?.value ?? `读不到${cell?.error === undefined ? '' : `：${cell.error}`}`
  const open = cell?.open
  return (
    <div className={s.identityCell}>
      <div className={s.identityTop}>
        <span className={s.identityLabel}>{label}</span>
        <span className={s.grow} />
        {open !== undefined && (
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            title={cell?.at === undefined ? open.label : `${open.label}：${cell.at}`}
            onClick={() => {
              /* 只传白名单键：具体路径由后端重算 */
              wb.openVersionLocation(open.key).catch((e) =>
                toasts.push(isAppError(e) ? e.message : String(e)),
              )
            }}
          >
            {open.label}
          </button>
        )}
      </div>
      <b className={cell?.value === undefined ? s.identityBad : s.mono} title={text}>
        {text}
      </b>
      {cell?.at !== undefined && (
        <span className={s.identityAt} title={cell.at}>
          {cell.at}
        </span>
      )}
    </div>
  )
}

/** 对账一行的人话：缺哪边就说哪边缺 */
function ledgerNote(row: ReleaseLedger['rows'][number]): { text: string; bad: boolean } {
  if (row.status === 'tagOnly') return { text: '没有回执', bad: true }
  if (row.status === 'recordOnly') return { text: '没有 tag', bad: true }
  return { text: '两边都有', bad: false }
}

export default function BuildPage({
  boot,
  book,
  words,
  report,
  tick,
  onGoto,
  onSave,
  onArtifactsRefresh,
  onReportRefresh,
}: Props) {
  const rows = book.buildRows

  /** 检查卡里展开着长描述的条目（2026-10-19：默认只留两行，按需展开） */
  const [expandedIssues, setExpandedIssues] = useState<Set<string>>(new Set())
  /** 「内嵌目录落后」那条待办的修复动作正在跑（按钮防连点） */
  const [regenBusy, setRegenBusy] = useState(false)
  const [lastPublish, setLastPublish] = useState<PublishTxReport | null>(null)
  /**
   * 发布闸开着没有（第二刀）。**点「发布」不再直接发** —— 先过这一道：
   * 十五项逐项打勾，全绿才给按「确认发布」。判定在后端（`wb_publish_audit`），
   * 这一页只负责把结果摆出来
   */
  const [gateOpen, setGateOpen] = useState(false)
  /**
   * 带着"上一次的发布结果"打开闸那屏（②卡的「查看发布结果」）。
   * 非 null ⇒ 直接进**发布回执**视图，不重跑十五项 —— 那是过去那一刻的快照。
   */
  const [gateReport, setGateReport] = useState<PublishTxReport | null>(null)
  /** 上一次合并之后「把主线同步到第二个云端源」的结论（重新打开回执时还要用） */
  const [gateMirror, setGateMirror] = useState<MirrorSync | null>(null)
  /** 发布历史面板（本地回执日志；每条一个手动刷新，不轮询） */
  const [historyOpen, setHistoryOpen] = useState(false)
  /**
   * 交付定稿检查（②，只读）。**随时可以开**：不看草稿脏不脏、不看工作区干不干净，
   * 也不写任何凭据 —— 它只回答"现在具不具备定稿条件、定稿会把 manifest 改成什么"。
   */
  const [finalizeOpen, setFinalizeOpen] = useState(false)
  const [finalizePreview, setFinalizePreview] = useState<FinalizePreview | null>(null)
  const [finalizeErr, setFinalizeErr] = useState<string | null>(null)
  const [finalizeBusy, setFinalizeBusy] = useState(false)
  /** 「更新发布凭据」正在写盘（只有这一颗按钮会写） */
  const [finalizeUpdating, setFinalizeUpdating] = useState(false)
  /** 生成记录面板（历次生成的回执；住应用数据目录，不轮询） */
  const [genHistoryOpen, setGenHistoryOpen] = useState(false)
  /*
   * **发布软件版本**的闸（第四刀）。点②卡的「发布软件版本」开它 —— 与上面那条
   * 「交付预设」的闸（`gateOpen`）是**两道不同的闸**，各有各的账。
   */
  const [releaseGateOpen, setReleaseGateOpen] = useState(false)
  /*
   * 软件版本的发布历史（2026-10-07 从发布框里挪到这 —— 作者：历史放在外面，不塞模态框）。
   * 本地回执日志（`release-history.json`），最新在前；与上面「交付预设」那本账
   * （`publish-history.json`）是**两本账**。进页取一次 + 关掉发布框取一次，不轮询。
   */
  const [releaseHistory, setReleaseHistory] = useState<ReleaseHistory | null>(null)
  const [releaseHistoryErr, setReleaseHistoryErr] = useState(false)
  const reloadReleaseHistory = useCallback(() => {
    setReleaseHistoryErr(false)
    wb.releaseHistory()
      .then(setReleaseHistory)
      .catch(() => setReleaseHistoryErr(true))
  }, [])
  /*
   * 版本身份四格 + git 对账（2026-11-07）：与发布历史同一个节奏 —— 进页取一次、
   * 点「刷新」重取。**两件事分两个请求**：四格永远读得出来（一格失败只写那一格），
   * 对账要跑 git，取不到 git 时它自己报错，不该把四格一起拖黑。
   */
  const [versionId, setVersionId] = useState<VersionIdentity | null>(null)
  const [ledger, setLedger] = useState<ReleaseLedger | null>(null)
  const [ledgerErr, setLedgerErr] = useState<string | null>(null)
  const [ledgerOpen, setLedgerOpen] = useState(false)
  const reloadVersionFacts = useCallback(() => {
    setLedgerErr(null)
    wb.versionIdentity()
      .then(setVersionId)
      .catch(() => setVersionId(null))
    wb.releaseLedger()
      .then(setLedger)
      .catch((e) => {
        setLedger(null)
        setLedgerErr(
          isAppError(e)
            ? `${e.message}${e.detail === undefined ? '' : ` —— ${e.detail}`}`
            : String(e),
        )
      })
  }, [])
  useEffect(() => {
    reloadReleaseHistory()
    reloadVersionFacts()
  }, [reloadReleaseHistory, reloadVersionFacts])
  const [baseline, setBaseline] = useState<BaselineDiffEntry[] | null>(null)
  const [strays, setStrays] = useState<string[] | null>(null)
  const [trash, setTrash] = useState<Awaited<ReturnType<typeof wb.trash>> | null>(null)
  /*
   * **交付文件清单**（2026-10-07）：除了 `mkp/presets/*.toml`，交付目录里还有一串
   * 附属文件（`content/*.json`、`catalog.json`、`manifest.json`、`source.json`）。
   * 名单来自后端（与残留审计同一份交付集合），这一页只负责摆出来、点开看原文。
   */
  const [deliveries, setDeliveries] = useState<DeliveryStatus | null>(null)
  /** 交付读数算不出来时明确报错，不伪装成"还在读" */
  const [deliveryError, setDeliveryError] = useState<string | null>(null)
  /** 交付分组折叠状态：缺省按后端 allSame 决定（有变化展开、全一致折叠） */
  const [groupOpenState, setGroupOpenState] = useState<Record<string, boolean>>({})
  /** 交付明细模态框开着没（页面外只摆汇总，作者 2026-10-19） */
  const [deliveryListOpen, setDeliveryListOpen] = useState(false)
  /**
   * 明细框右侧 diff 的**只读预演**（2026-10-19）。与生成确认框的 `genPreview`
   * 分开存：明细打开一次、确认框开关一次，两者不互相覆盖。
   */
  const [deliveryPreview, setDeliveryPreview] = useState<PreviewReport | null>(null)
  const [deliveryPreviewErr, setDeliveryPreviewErr] = useState<string | null>(null)
  /*
   * 当前安装的 SupportEase 版本号（只读展示）——「软件版本」块用它。
   * `null` = 还没取到 / 取不到（显示"未知"，**不阻塞页面**）。
   */
  const [appVersion, setAppVersion] = useState<string | null>(null)
  /*
   * 生成前确认（2026-10-02）：点「生成」先开这个框，看 diff 再确认。
   *
   *   preview === undefined  框没开
   *   preview === null       框开着，预演还没算回来（或算挂了 —— 看 previewErr）
   *   preview 是报告        算回来了，可以确认
   *   genDone 非 null        已经生成完，框切成结果页
   *
   * `genPicked` 记下"这一轮要生成哪些" —— 预演与确认必须同一批（草稿落盘会重取
   * book，`ids` 可能就变了，所以不能在确认那一刻再读一遍）。
   */
  const [genPicked, setGenPicked] = useState<string[] | null>(null)
  const [genPreview, setGenPreview] = useState<PreviewReport | null>(null)
  const [genPreviewErr, setGenPreviewErr] = useState<string | null>(null)
  const [genBusy, setGenBusy] = useState(false)
  const [genDone, setGenDone] = useState<GenerateReport | null>(null)

  /* 2026-10-31 统一编辑事务：生成/发布只在「已保存」那一版上跑。
     后端也有同一道闸（`require_clean`）—— 这里只是让按钮早点变灰、说清原因。 */
  const unsaved = book.dirtyCount > 0
  const blocked = (report?.blocks ?? 0) > 0
  const dirty = unsaved

  /** 没配方的版本不能生成；其余（含已生成）都能主动重走一遍预演 → 确认 */
  const pickable = (r: (typeof rows)[number]) => r.state !== 'noResources'

  /* 产物名 → 版本 uid：分组文件行上的「重新生成」与左侧「去处理」锚点都用它 */
  const uidByRel = new Map<string, string>()
  for (const r of rows) {
    if (r.mkpFile !== null) uidByRel.set(`mkp/presets/${r.mkpFile}`, r.uid)
  }
  const relByUid = new Map<string, string>()
  for (const [rel, uid] of uidByRel) relByUid.set(uid, rel)

  /* 交付读数：数量直接取后端分组摘要，页面不再逐份判"谁该更新" */
  const deliveryFiles = deliveries?.files ?? []
  const deliveryPending =
    deliveries?.groups.reduce((n, g) => n + g.modified + g.added + g.removed, 0) ?? 0

  /* 基线 / 残留 / 回收站：进页取一次，外壳每写一次（生成会改产物）重取 */
  useEffect(() => {
    void (async () => {
      try {
        const [b, st, t] = await Promise.all([wb.baselineDiff(), wb.distStrays(), wb.trash()])
        setBaseline(b)
        setStrays(st)
        setTrash(t)
      } catch (e) {
        toasts.push(isAppError(e) ? e.message : String(e))
      }
    })()
  }, [tick])

  /* 当前版本号：进页取一次就够（它构建期定死，不会变）；取不到显示"未知"，不报整页错 */
  useEffect(() => {
    void wb
      .appVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion(null))
  }, [])

  /* 交付读数：进页取一次；生成 / 发布之后（tick）重取 —— 那时盘上的存在与否、目录登记都会变。
     窗口 focus 那一路由外壳 +1 tick（2026-10-19）：外面改了 3MF，切回来这里就跟着重算。 */
  useEffect(() => {
    void wb
      .deliveryFiles()
      .then((s) => {
        setDeliveries(s)
        setDeliveryError(null)
      })
      .catch((e: unknown) => {
        setDeliveries(null)
        setDeliveryError(isAppError(e) ? e.message : String(e))
      })
  }, [tick])

  /*
   * 「去处理」的定位（C14 第二十四轮）：滚动到目标模块 + 闪烁两秒。
   * 本体在 `c14/locate.ts`（目标页也用同一份 —— 参数台的「去处理」落地按它滚 + 闪）。
   */
  const goTarget = (sel: string) => {
    const isRow = sel.startsWith('t-build-') && sel !== 't-build'
    const rel = isRow ? relByUid.get(sel.slice('t-build-'.length)) : undefined
    const group = deliveries?.files.find((f) => f.rel === rel)?.group
    /* 有产物行就展开它所在的组、定位到那一行；没有产物行（如暂无资源的版本）
       就落到①卡本身，不能让「去处理」点了没反应 */
    if (group !== undefined) {
      setGroupOpenState((prev) => ({ ...prev, [group]: true }))
      locateAnchor(sel, { row: true })
    } else {
      locateAnchor('t-build', {})
    }
  }

  const issueGoto = (i: Issue) => {
    /* 本页有格子可滚的那几条先就地滚过去 */
    /* 本页的（build 视角）就地滚；先落盘的行没生成过，锚点就是 ① 卡 */
    if (i.at.view === 'build') {
      goTarget(i.at.uid ? `t-build-${i.at.uid}` : 't-build')
      return
    }
    const g = gotoOf(i)
    if (g) onGoto(g.view, g.focus)
  }

  /**
   * 生成第一步：**先预演，不写盘**。勾中的行按当前已保存的配方算一遍，与磁盘上现存的
   * 逐份比，把报告喂给确认框。
   *
   * 作者 2026-10-02 定的规矩：点「生成」不许当场覆盖 —— 先看 diff、确认了才写。
   * 预演走 `wb_generate_preview`（后端只算不写），与真生成同一批 `todo`、同一道闸。
   */
  const openGenerate = async (list: string[]) => {
    if (!list.length) return
    /* 草稿不干净先落盘 —— 预演读的也是**已保存**的配方（与生成同一条顺序规矩） */
    if (dirty && !(await onSave())) return
    const picked = [...list]
    setGenPicked(picked)
    setGenPreview(null)
    setGenPreviewErr(null)
    setGenDone(null)
    try {
      setGenPreview(await wb.generatePreview({ picked }))
    } catch (e) {
      setGenPreviewErr(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * 待更新 / 未生成、可以进这次生成队列的版本（**不看用户勾选**，作者 2026-10-19）。
   *
   * 用后端 `pendingUids`，不再从 `BuildState` 推：它已经把「配方改过」与
   * 「重新渲染的字节变了」（模板/注释变化）都算进去了 —— 后者是①卡看不见的。
   */
  const pendingUids = deliveries?.pendingUids ?? []
  /**
   * 只剩目录漂移时（改了 3MF、所有产物都跟上了）也允许生成一次：
   * `generate_with` 即使产物全是 unchanged，也会重算 `content` 与 `catalog.json`。
   * 这时队列带全部可生成版本 —— 产物不会重写，目录会更新。
   */
  const needCatalog = deliveryPending > 0 || (deliveries?.drift.length ?? 0) > 0
  const generateTargets =
    pendingUids.length > 0
      ? pendingUids
      : needCatalog
        ? rows.filter((r) => pickable(r)).map((r) => r.uid)
        : []
  const generateLabel =
    deliveryPending > 0
      ? `生成本次待更新（${deliveryPending} 份）`
      : '重算目录与清单'

  /** 单文件「重新生成」：预设按自己的 uid，目录类按整批（后端事务一次重算全部目录） */
  const regenerateTargetsOf = (rel: string): string[] => {
    const uid = uidByRel.get(rel)
    if (uid !== undefined) return [uid]
    if (generateTargets.length > 0) return generateTargets
    return rows.filter((r) => pickable(r)).map((r) => r.uid)
  }

  /**
   * 打开「本次交付明细」（2026-10-19）。
   *
   * 先拉一次**只读预演**（全部可生成版本）—— 右侧 diff 用它，其中
   * `catalog.json` 那一份就能看到三份 3MF 的 sha / size 新旧行。
   * 预演与真生成同一台渲染器、同一批 todo（不另算一套）。
   */
  const openDeliveryList = async () => {
    setDeliveryListOpen(true)
    setDeliveryPreview(null)
    setDeliveryPreviewErr(null)
    if (dirty && !(await onSave())) {
      setDeliveryListOpen(false)
      return
    }
    const all = rows.filter((r) => pickable(r)).map((r) => r.uid)
    try {
      setDeliveryPreview(await wb.generatePreview({ picked: all }))
    } catch (e) {
      setDeliveryPreviewErr(isAppError(e) ? e.message : String(e))
    }
  }

  /** 明细框底部「生成待更新项…」：只带待更新 / 未生成，走既有的预演 → 确认 → 写盘 */
  const generateStale = async () => {
    setDeliveryListOpen(false)
    await openGenerate(generateTargets)
  }

  /**
   * 拉一次交付检查（只读）。
   *
   * 与 `openGenerate` / `openDeliveryList` 不同，它**不先保存草稿** —— 交付检查看的是
   * 盘上已生成的交付文件，与草稿无关；为了看一眼就把草稿写进仓库是副作用。
   */
  const loadFinalize = async () => {
    setFinalizePreview(null)
    setFinalizeErr(null)
    setFinalizeBusy(true)
    try {
      setFinalizePreview(await wb.finalizePreview())
    } catch (e) {
      setFinalizeErr(isAppError(e) ? e.message : String(e))
    } finally {
      setFinalizeBusy(false)
    }
  }

  const openFinalize = () => {
    setFinalizeOpen(true)
    void loadFinalize()
  }

  /**
   * 「更新发布凭据」：只重写 `manifest.json`（后端整笔校验，集合没收敛就拒绝）。
   *
   * 回执里带**更新后重新检查的那一份**，直接换屏 —— 不出现"回执说改了、列表还是红的"。
   */
  const updateCredentials = async () => {
    setFinalizeUpdating(true)
    try {
      const r = await wb.updatePublishCredentials()
      setFinalizePreview(r.preview)
      setFinalizeErr(null)
      toasts.push(
        r.written
          ? `已更新发布凭据：${r.changed} 条登记按盘上真字节重算（共 ${r.entries} 条）`
          : '发布凭据本来就是最新的，一个字节都没写',
      )
    } catch (e) {
      toasts.push(
        isAppError(e)
          ? `${e.message}${e.detail === undefined ? '' : ` —— ${e.detail}`}`
          : String(e),
      )
    } finally {
      setFinalizeUpdating(false)
    }
  }

  /** 生成第二步：确认。真写盘，然后把框切成结果页 */
  const confirmGenerate = async () => {
    if (genPicked === null) return
    setGenBusy(true)
    try {
      const rep = await wb.generate({ picked: genPicked })
      // 生成记录由**后端生成事务直接落进台账**（built.json）—— 前端不再回填草稿，
      // 生成完成 = 台账已是这一代（2026-10-06 状态机修正）。
      setGenDone(rep)
      /* 状态不在这里改成「成功」：`onArtifactsRefresh` 会让外壳 +1 tick，
         `wb_delivery_files` 那一趟 effect 按真实读数重算 ①卡 —— 别自己猜结论。 */
      onArtifactsRefresh()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      /* 真写挂了就把框关了 —— 结果页是给"成功"用的，别拿它兜错误 */
      closeGenerate()
    } finally {
      setGenBusy(false)
    }
  }

  /** 关掉确认框（取消 / 结果页「完成」都走这里） */
  const closeGenerate = () => {
    setGenPicked(null)
    setGenPreview(null)
    setGenPreviewErr(null)
    setGenDone(null)
  }

  /**
   * 真发布。**只有发布闸全绿、人按下「确认发布」才走得到这里**
   * （闸那句「任何一项 Blocker 红了就不许往下走」是这道口子的唯一入口）。
   *
   * 后端内部还有两道硬闸：检查阻断 + 残留拦截（一个字节都不许在带残留时写出）。
   * 这一层**不吞错**：抛出去让闸那屏把原因显示在原地，别把失败伪装成成功。
   */
  const publish = async () => {
    /*
     * 发布事务：审计 → 生成 → 定稿 → 本地 git → 平台 PR。
     * 这里**只发一次**、只把阶段快照摆出来 —— 前端不再串「生成 / 发布 / 建 PR」三个动作。
     *
     * ★ 返回值交回给闸那屏：它拿这份快照就地切成**发布回执**（阶段链 + PR 地址 + 合并）。
     * 这一屏也留一份 —— 关掉模态框之后，②卡还能「查看发布结果」再打开。
     */
    /* `confirmed` = 人刚点过这颗「确认发布」按钮（2026-11-05 硬约束）。
       内核默认 false，缺了会零副作用地拒绝 —— CLI / CI 那条路必须显式带 --confirm。 */
    const rep = await wb.publish({ confirmed: true })
    setLastPublish(rep)
    /* 发布改了交付目录 → 整本、报告、①卡交付读数一起重取（③ 基线 / ④ 残留也跟着 tick 走） */
    onArtifactsRefresh()
    toasts.push(rep.summary)
    return rep
  }

  /** 同步对照基线（14.9 第②步）。前提：人看过上面的 diff 清单 */
  const syncBaseline = async () => {
    try {
      const n = await wb.syncBaseline()
      toasts.push(n > 0 ? `已同步 ${n} 份基线（内容相同的自动跳过）` : '基线本来就一致，没有要写的')
      setBaseline(await wb.baselineDiff())
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 清理交付残留。走 .trash/delivery/<stamp>/ 回收，不直接删 */
  const cleanStrays = async () => {
    try {
      const n = await wb.cleanDistStrays()
      toasts.push(n > 0 ? `已把 ${n} 个残留移入回收站 —— 重新发布即可补齐` : '没有残留')
      setStrays(await wb.distStrays())
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /*
   * **内嵌目录重算**（2026-11-05，作者：「这条待办不该只剩终端那条路」）。
   * 按当前 `presets/` 重算 `src-tauri/src/runtime/catalog.generated.json`，
   * 完事立刻重取检查报告 —— 报告绿了、待办消失，才算真修好了。
   */
  const regenEmbeddedCatalog = async () => {
    setRegenBusy(true)
    try {
      const r = await wb.regenEmbeddedCatalog()
      toasts.push(
        r.changed
          ? `内嵌目录已重算（${r.files} 份文件 · revision ${r.revision}）—— 重新编译客户端后才随包生效`
          : '内嵌目录本来就是最新的，一个字节都没写',
      )
      onReportRefresh()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setRegenBusy(false)
    }
  }

  const drift = deliveries?.drift ?? []

  const baselineDirty = baseline?.filter((b) => b.status !== 'same').length ?? 0

  return (
    <div className={s.colsRev}>
      {/* —— 检查：左栏的事项清单 —— */}
      <div className={s.card}>
        <div className={s.cardHead}>
          <h2>检查</h2>
          <span className={s.cardNote}>
            {report
              ? `阻断 ${report.blocks} · 待办 ${report.todos} · 提示 ${report.hints}`
              : '正在读……'}
          </span>
        </div>
        <div className={s.cardBody}>
          {report && report.issues.length > 0 ? (
            report.issues.map((i) => {
              const long = i.detail.length > 120
              const open = expandedIssues.has(i.id)
              return (
                <div
                  key={i.id}
                  className={`${s.issue} ${i.severity === 'block' ? s.issueBlock : ''}`}
                >
                  <span className={`${s.tag} ${s[`tagSev${i.severity[0].toUpperCase()}${i.severity.slice(1)}` as keyof typeof s] ?? ''}`}>
                    {i.severity === 'block' ? '阻断' : i.severity === 'todo' ? '待办' : '提示'}
                  </span>
                  <span className={s.issueTxt}>
                    <b>{i.title}</b>
                    <span className={long && !open ? s.issueDetailClamp : undefined}>
                      {i.detail}
                    </span>
                  </span>
                  {long && (
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      onClick={() =>
                        setExpandedIssues((prev) => {
                          const next = new Set(prev)
                          if (next.has(i.id)) next.delete(i.id)
                          else next.add(i.id)
                          return next
                        })
                      }
                    >
                      {open ? '收起' : '展开'}
                    </button>
                  )}
                  {/* 「内嵌目录落后」自带修复动作：不是跳到别的页，而是就地重算那份文件 */}
                  {i.id === 'embedded.catalog_stale' && (
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnSm}`}
                      disabled={regenBusy}
                      title="按当前 presets/ 重算 src-tauri/src/runtime/catalog.generated.json（内容一样就不写）；重新编译客户端后才随安装包生效"
                      onClick={() => void regenEmbeddedCatalog()}
                    >
                      {regenBusy ? '重算中…' : '重新生成'}
                    </button>
                  )}
                  {/* 每一条都要说清去哪儿处理（issues.rs 的规矩），所以每一条都有这颗按钮 */}
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    onClick={() => issueGoto(i)}
                  >
                    去处理
                  </button>
                </div>
              )
            })
          ) : (
            <div className={s.emptyHint}>{report ? report.emptyHint : '正在读检查报告……'}</div>
          )}
        </div>
      </div>

      <div className={s.flow}>
        {/* —— ① 生成 —— */}
        <div id="t-build" className={s.card}>
          <div className={`${s.cardHead} ${s.buildHead}`}>
            <h2>① 生成</h2>
            {/* 查看 / 历史类放左侧，主操作独占右侧 */}
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => setGenHistoryOpen(true)}
              title="历次生成的回执：每一步做了什么、写了哪些文件、失败在哪（住应用数据目录，只留最近 50 份）"
            >
              生成记录
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              disabled={deliveries === null}
              onClick={() => void openDeliveryList()}
              title="看每一份文件的逐行 diff：MKP 预设 / 目录与清单 / 发布定稿"
            >
              交付明细
            </button>
            <span className={s.grow} />
            {deliveries === null ? (
              <span className={s.cardNote}>正在读交付目录……</span>
            ) : generateTargets.length > 0 ? (
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={blocked || unsaved}
                data-tip={
                  unsaved
                    ? '有改动还没保存 —— 先保存或放弃，再生成'
                    : blocked
                      ? words.disabled.buildBlocked
                      : `按当前源处理 ${deliveryPending} 份差异产物，先预演、确认后才写盘`
                }
                onClick={() => void openGenerate(generateTargets)}
              >
                {generateLabel}
              </button>
            ) : (
              <span className={s.cardNote}>没有需要生成的文件</span>
            )}
          </div>
          <div className={s.cardBody}>
            <div className={s.delStatusBar}>
              <span>预设改完 → 生成 → 交付 → 版本交付</span>
              {deliveries !== null && (
                <>
                  <span className={s.delSep} />
                  <span>
                    发布凭据：
                    {/* 可点开「交付定稿」看逐条原因与定稿后的变化（只读，不写任何凭据） */}
                    <button
                      type="button"
                      className={
                        deliveries.manifestStale
                          ? `${s.linkBtn} ${s.linkWarn}`
                          : s.linkBtn
                      }
                      title="打开「交付定稿」：逐条凭据检查 + 定稿会把 manifest 改成什么（只读）"
                      onClick={() => void openFinalize()}
                    >
                      {deliveries.manifestStale
                        ? `已失效（${deliveries.manifestReasons.length} 条）`
                        : '有效'}
                    </button>
                  </span>
                  <span className={s.delSep} />
                  <span>
                    {deliveries.groups.filter((g) => !g.allSame).length} 组有变化 / 共{' '}
                    {deliveries.groups.length} 组
                  </span>
                </>
              )}
            </div>
            {dirty && (
              <div className={s.warn} style={{ marginTop: 8 }}>
                <div className={s.warnDetail}>有改动还没保存 —— 生成前会先保存。</div>
              </div>
            )}
            {deliveryError !== null && (
              <div className={s.warn} style={{ marginTop: 8 }} role="alert">
                <div className={s.warnTitle}>交付状态算不出来</div>
                <div className={s.warnDetail}>
                  {deliveryError}（候选集重算或逐字节比较失败，不猜测状态）
                </div>
              </div>
            )}
            {deliveries === null ? (
              <div className={s.emptyHint}>正在读交付目录……</div>
            ) : (
              <div className={s.delGroups}>
                {deliveries.groups.map((g) => {
                  const open = groupOpenState[g.id] ?? !g.allSame
                  const todo = groupTodo(g)
                  const files = sortFiles(deliveryFiles.filter((f) => f.group === g.id))
                  return (
                    <section className={s.delGroup} key={g.id}>
                      <button
                        type="button"
                        className={s.delGroupHead}
                        aria-expanded={open}
                        onClick={() =>
                          setGroupOpenState((prev) => ({ ...prev, [g.id]: !open }))
                        }
                      >
                        <span className={s.delCaret}>{open ? '▾' : '▸'}</span>
                        <span className={s.delGroupTitle}>
                          {g.label}（{g.total}）
                        </span>
                        {todo > 0 && (
                          <span className={s.delGroupTodo}>{todo} 项待处理</span>
                        )}
                        <span className={s.delGroupSum}>{groupSummary(g)}</span>
                      </button>
                      {open && (
                        <div className={s.delGroupRows}>
                          {files.map((f) => {
                            const uid = uidByRel.get(f.rel)
                            const regen =
                              f.diff !== 'frozen' &&
                              f.diff !== 'removed' &&
                              f.diff !== 'unmanaged'
                            return (
                              <div
                                className={s.delFileRow}
                                id={uid === undefined ? undefined : `t-build-${uid}`}
                                key={f.rel}
                              >
                                <span className={`${s.mono} ${s.delFileName}`} title={f.rel}>
                                  {displayName(f.rel)}
                                </span>
                                <span className={toneClass(stateTone(f))}>
                                  {stateText(f)}
                                </span>
                                <span className={s.delFileSize}>
                                  {f.exist ? fmtSize(f.size) : '不在盘上'}
                                </span>
                                {regen && (
                                  <button
                                    type="button"
                                    className={s.delRowBtn}
                                    disabled={blocked || unsaved}
                                    title={
                                      uid === undefined
                                        ? '按当前源重算目录与清单（生成事务会一起核对预设）'
                                        : '只重新生成这一份预设，先预演、确认后才写盘'
                                    }
                                    onClick={() => void openGenerate(regenerateTargetsOf(f.rel))}
                                  >
                                    {uid === undefined ? '重算目录' : '重新生成'}
                                  </button>
                                )}
                              </div>
                            )
                          })}
                        </div>
                      )}
                    </section>
                  )
                })}
              </div>
            )}
          </div>
        </div>

        {/* —— ② 交付预设 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>② 交付预设</h2>
            <span className={s.cardNote}>提交到远端仓库，合并后客户端可取</span>
          </div>
          <div className={s.cardBody}>
            {blocked && (
              <div className={s.warn} style={{ marginTop: 0 }}>
                <div className={s.warnTitle}>有 {report?.blocks} 条阻断，不许发布</div>
                <div className={s.warnDetail}>左栏里标「阻断」的就是 —— 处理完这一步才会亮。</div>
              </div>
            )}
            <div className={s.bar} style={{ marginTop: blocked ? 12 : 0 }}>
              <span className={s.cardNote}>
                发布前先过闸（十五项）；有残留不写盘。
              </span>
              <span className={s.grow} />
              {/* 上一次发布的结果还在 —— 一次点击回到那份回执（不是每次都是空的新面板） */}
              {lastPublish !== null && (
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  title="看上一次发布的回执：走到哪一步、PR 多少号、合没合"
                  onClick={() => {
                    setGateReport(lastPublish)
                    setGateOpen(true)
                  }}
                >
                  查看发布结果
                </button>
              )}
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                title="历次发布的回执（本地记录；每条可手动刷新）"
                onClick={() => setHistoryOpen(true)}
              >
                发布历史
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={blocked || unsaved}
                data-tip={
                  unsaved
                    ? '有改动还没保存 —— 先保存或放弃，再发布'
                    : blocked
                      ? words.disabled.publishBlocked
                      : '先过发布闸：十五项逐项打勾，全绿才给发布'
                }
                onClick={() => {
                  setGateReport(null)
                  setGateOpen(true)
                }}
              >
                发布
              </button>
            </div>

            {/*
              **发布前状态**：完整的文件清单在①生成卡的分组列表里，这里只留
              发布独有的信息（资产漂移、目录登记、发布凭据），不重复数文件。
            */}
            <div className={s.group}>
              <div className={s.groupHead}>发布前状态</div>
              {deliveries === null ? (
                <div className={s.emptyHint}>正在读交付目录……</div>
              ) : (
                <>
                  {drift.length > 0 ? (
                    <p className={s.note} style={{ marginTop: 8 }}>
                      <span className={s.tagDanger}>{drift.length} 份资产字节变了</span>{' '}
                      <span className={s.mono}>catalog.json</span> 待重新生成
                      {deliveries.catalogModified !== null &&
                        `（目录上次重算 ${localStamp(deliveries.catalogModified)}）`}
                    </p>
                  ) : (
                    <p className={s.note} style={{ marginTop: 8 }}>
                      交付目录与盘上一致
                      {deliveries.catalogModified !== null &&
                        `（目录上次重算 ${localStamp(deliveries.catalogModified)}）`}
                    </p>
                  )}
                  {deliveries.catalogError !== null && (
                    <p className={s.note} style={{ color: 'var(--danger)' }} role="alert">
                      {deliveries.catalogError}
                    </p>
                  )}
                  <p className={s.note} style={{ marginTop: 0 }}>
                    生成重算目录；发布才写 manifest / source 并推远端。
                  </p>
                </>
              )}
              {lastPublish !== null && (
                <p className={s.note} style={{ marginTop: 4 }}>
                  上一次发布落下 {lastPublish.files} 个文件
                  {lastPublish.commit ? ` · ${lastPublish.commit}` : ''}
                  {lastPublish.review ? ` → PR #${lastPublish.review.number}` : ''}
                </p>
              )}
            </div>
          </div>
        </div>

        {/*
          —— 软件版本（只读）——
          ★ 与上面的「交付预设」是**两回事**（见 `docs/RELEASE-TRANSACTIONS.md`）：
            上面把预设/数据提交进仓库、合并后客户端就能取到；
            这一块说的是"发布一个新的 SupportEase 安装包"——**另一套事务，本轮不实现**。
          所以这里**只读展示**，不摆按钮（摆一个点不动的按钮比不摆更糟）。
        */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>软件版本</h2>
            <span className={s.cardNote}>发布安装包（与发预设是两回事）</span>
          </div>
          <div className={s.cardBody}>
            <div className={s.bar}>
              <span className={s.cardNote}>
                {/* 它只是"这个工作台二进制是哪一版"，不是"装的是哪一版" —— 后者在下面四格 */}
                本工作台 <span className={s.mono}>{appVersion ?? '读不到'}</span>
              </span>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary}`}
                onClick={() => setReleaseGateOpen(true)}
                title="先过一道闸（分支 / 工作区 / 版本号 / tag / 平台），全绿才发"
              >
                发布软件版本
              </button>
            </div>

            {/*
              版本身份四格（2026-11-07）：四个号不一样是常态，不是错误 ——
              改代码让"当前开发"往前走，打包让"本地已打包"变，打 tag 让"最新已发布"变，
              装包让"已安装"变。读不到就写明读不到，**不拿别的号顶**。
            */}
            {versionId === null ? (
              <div className={s.emptyHint}>正在读版本读数……</div>
            ) : (
              <div className={s.identityGrid}>
                <IdentityCell label="已安装（安装包）" cell={versionId.installed} />
                <IdentityCell label="本地已打包" cell={versionId.packaged} />
                <IdentityCell label="最新已发布（tag）" cell={versionId.published} />
                <IdentityCell label="当前开发" cell={versionId.developing} />
              </div>
            )}
            <p className={s.note}>
              「已安装」读的是<b>本机</b> <span className={s.mono}>/Applications</span> 下那个{' '}
              <span className={s.mono}>.app</span> 的 Info.plist —— 纯本地读取，不联网、不上传。
              预设更新不需要新安装包；改了程序本身才发。发完记得合并 release.json 的 PR。
            </p>

            {/*
              发布历史（2026-10-07 从发布框里挪到这）：本地回执日志，最新在前。
              ★ 这是「发布软件版本」那本账 —— 上面「交付预设」的回执在另一本里
                （②卡顶上那枚「本次发布落下…」chip），两本账不混。
              ★ 2026-11-07 起**两棵数据根一起读**（工作台 + 命令行），每条标来源。
            */}
            <div className={s.bar}>
              <span className={s.cardNote}>
                发布历史 ·{' '}
                {releaseHistoryErr
                  ? '读不到'
                  : releaseHistory === null
                    ? '正在读……'
                    : `${releaseHistory.records.length} 条 · 最新在前`}
              </span>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  reloadReleaseHistory()
                  reloadVersionFacts()
                }}
              >
                刷新
              </button>
            </div>
            {releaseHistoryErr ? (
              <div className={s.emptyHint}>读不到发布历史（后端刚才报错了）—— 点「刷新」再试</div>
            ) : releaseHistory === null ? (
              <div className={s.emptyHint}>正在读……</div>
            ) : releaseHistory.records.length === 0 ? (
              <div className={s.emptyHint}>还没有发过软件版本。</div>
            ) : (
              <div className={s.vstack}>
                {releaseHistory.records.map((r, i) => (
                  <div key={`${r.tag}-${i}`} className={s.vfield}>
                    <div className={s.vhead}>
                      <b className={s.mono}>{r.tag}</b>
                      {r.source !== undefined && (
                        <span className={s.tag} title={r.source === 'cli' ? '命令行发布写的那本账' : '本工作台写的那本账'}>
                          {r.source === 'cli' ? '命令行' : '工作台'}
                        </span>
                      )}
                      <span className={s.vkey}>{RELEASE_STAGE_TEXT[r.stage] ?? r.stage}</span>
                      <span className={s.grow} />
                      <span className={s.rowMeta}>{localStamp(r.at)}</span>
                    </div>
                    <span className={s.vhelp}>{r.summary}</span>
                  </div>
                ))}
              </div>
            )}

            {/*
              git 对账（2026-11-07）：历史只记"从这条入口发过的"，而 tag 才是冻结的
              唯一证据。表格把两边并起来，缺哪边就标哪边 —— **不从 git 编回执**。
            */}
            <button
              type="button"
              className={s.disclosure}
              aria-expanded={ledgerOpen}
              onClick={() => setLedgerOpen((v) => !v)}
            >
              {ledgerOpen ? '收起 git 对账' : '查看 git 对账'}
            </button>
            {ledgerOpen && (
              <div className={s.ledgerBox}>
                {ledgerErr !== null ? (
                  <div className={s.emptyHint}>读不到 git：{ledgerErr}</div>
                ) : ledger === null ? (
                  <div className={s.emptyHint}>正在读……</div>
                ) : (
                  <>
                    <p className={s.note} style={{ marginTop: 0 }}>
                      当前开发 <span className={s.mono}>{ledger.developing}</span>
                      {ledger.developingFrozen ? ' · 已冻结（有 tag）' : ' · 未冻结（还没有 tag）'} ·
                      最新 tag <span className={s.mono}>{ledger.latestTag ?? '—'}</span> · 回执{' '}
                      {ledger.receipts} 条
                    </p>
                    {ledger.rows.map((row) => {
                      const note = ledgerNote(row)
                      return (
                        <div key={`${row.version}-${row.tag ?? ''}`} className={s.ledgerRow}>
                          <b className={s.mono}>{row.version}</b>
                          <span className={s.rowMeta}>{row.at}</span>
                          <span className={`${s.mono} ${s.rowMeta}`}>{row.commit ?? '—'}</span>
                          <span className={s.grow} />
                          <span className={note.bad ? s.identityBad : s.rowMeta}>{note.text}</span>
                        </div>
                      )
                    })}
                  </>
                )}
              </div>
            )}
          </div>
        </div>

        {/* —— ③ 对照基线 —— */}
        <div id="t-baseline" className={s.card}>
          <div className={s.cardHead}>
            <h2>③ 对照基线</h2>
            <span className={s.cardNote}>
              {baseline ? `${baseline.length} 份产物 · ${baselineDirty} 份与基线不同` : '正在读……'}
            </span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              人看过再同步；内容相同的自动跳过。
            </p>
            {baseline?.map((b) => (
              <div key={b.fileName} className={s.row}>
                <span className={`${s.mono} ${s.rowMeta}`}>{b.fileName}</span>
                <span
                  className={`${s.tag} ${
                    b.status === 'same' ? s.tagGhost : b.status === 'changed' ? s.tagDanger : s.tagBuildNone
                  }`}
                >
                  {b.status === 'same' ? '一致' : b.status === 'changed' ? '已变更' : '基线缺失'}
                </span>
                <span className={s.grow} />
                <span className={`${s.mono} ${s.rowMeta}`} title="产物内容哈希（前 16 位）">
                  {b.productSha}
                </span>
              </div>
            ))}
            <div className={s.bar} style={{ marginTop: 12 }}>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={!baseline || baselineDirty === 0}
                data-tip={
                  baseline && baselineDirty === 0
                    ? '九份产物与基线逐字节相同，没有要写的'
                    : '只写上面标「已变更 / 基线缺失」的那几份'
                }
                onClick={() => void syncBaseline()}
              >
                同步基线{baselineDirty > 0 ? `（${baselineDirty}）` : ''}
              </button>
            </div>
          </div>
        </div>

        {/* —— ④ 交付残留 —— */}
        <div id="t-strays" className={s.card}>
          <div className={s.cardHead}>
            <h2>④ 交付残留</h2>
            <span className={s.cardNote}>
              {strays ? `${strays.length} 个` : '正在读……'}
            </span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              不在交付集合内的文件；有残留时发布会被拦下。
            </p>
            {strays && strays.length > 0 ? (
              strays.map((f) => (
                <div key={f} className={s.row}>
                  <span className={`${s.mono} ${s.rowMeta}`}>{f}</span>
                </div>
              ))
            ) : (
              <div className={s.emptyHint}>
                {strays ? '没有残留 —— 交付目录与本次交付集合一致' : '正在读……'}
              </div>
            )}
            <div className={s.bar} style={{ marginTop: 12 }}>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={!strays || strays.length === 0}
                title="走 workbench/.trash/delivery/ 回收（保留相对路径，可还原），不直接删"
                onClick={() => void cleanStrays()}
              >
                清理残留
              </button>
            </div>
          </div>
        </div>

        {/* —— ⑤ 回收站 —— */}
        <div id="t-trash" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑤ 回收站</h2>
            <span className={s.cardNote}>{trash ? `${trash.length} 条` : '正在读……'}</span>
          </div>
          <div className={s.cardBody}>
            {trash && trash.length > 0 ? (
              trash.map((t) => (
                <div key={t.file} className={s.row}>
                  <span className={`${s.mono} ${s.rowMeta}`}>{t.machineId}/{t.versionId}</span>
                  <span className={s.rowMeta}>{t.deletedStamp}</span>
                  <span className={s.grow} />
                  <span className={`${s.mono} ${s.rowMeta}`}>{t.file}</span>
                </div>
              ))
            ) : (
              <div className={s.emptyHint}>{words.empty.trashEmpty}</div>
            )}
            <p className={s.note} style={{ marginBottom: 0 }}>
              删除版本时的生成快照；只读。
            </p>
          </div>
        </div>

        {/* —— ⑥ 子目录职责（14.7） —— */}
        <div id="t-store-dirs" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑥ 子目录职责</h2>
            <span className={s.cardNote}>workbench/ 的五个子目录，谁写谁读一句话说清</span>
          </div>
          <div className={s.cardBody}>
            {boot.storeDirs.map((d) => (
              <div key={d.name} className={s.row}>
                <span className={`${s.mono} ${s.rowMeta}`}>{d.name}</span>
                <span className={s.rowName}>{d.role}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* 生成前确认（2026-10-02）：点「生成」不再当场覆盖 —— 先看 diff、确认了才写 */}
      {genPicked !== null && (
        <GenerateDiffModal
          report={genPreview}
          error={genPreviewErr}
          busy={genBusy}
          done={genDone}
          onConfirm={() => void confirmGenerate()}
          onClose={closeGenerate}
        />
      )}

      {/* 交付检查与修复（②）：能修的直接修（只写 manifest.json），不能修的说去哪修 */}
      {finalizeOpen && (
        <FinalizeModal
          preview={finalizePreview}
          error={finalizeErr}
          loading={finalizeBusy}
          busy={finalizeUpdating}
          onReload={() => void loadFinalize()}
          onUpdateCredentials={() => void updateCredentials()}
          onLocate={(at: IssueWhere) => {
            /* 关框再跳：定位是"去源的哪一版"，留在模态框里就看不清目标页 */
            const g = gotoOfWhere(at)
            if (g === null) return
            setFinalizeOpen(false)
            onGoto(g.view, g.focus)
          }}
          onCheckIdentity={() => {
            setFinalizeOpen(false)
            setGateReport(null)
            setGateOpen(true)
          }}
          onClose={() => setFinalizeOpen(false)}
          onPublish={() => {
            /* 只是进正式发布界面 —— 一行都不写，发布仍要在闸里由人确认 */
            setFinalizeOpen(false)
            setGateReport(null)
            setGateOpen(true)
          }}
        />
      )}

      {/* 发布闸 + 回执（第二 / 四刀）：点「发布」先过闸；发完就地留在回执上 */}
      {gateOpen && (
        <PublishGateModal
          onClose={() => {
            setGateOpen(false)
            setGateReport(null)
          }}
          onPublish={publish}
          initialReport={gateReport}
          initialMirror={gateMirror}
          onMerged={(r, m) => {
            /* 合并之后把②卡那份快照也更新掉 —— 免得「查看发布结果」还写着"等待合并" */
            setLastPublish((prev) => (prev === null ? prev : { ...prev, review: r }))
            /* ★ 镜像同步的结论一并留着：重新打开回执时「客户端」那一块还要靠它说话
               （它推不出来 —— 那是合并那一刻的事实） */
            setGateMirror(m)
          }}
        />
      )}

      {/* 发布历史：本地回执日志（`publish-history.json`），最新在前 */}
      {historyOpen && <HistoryModal onClose={() => setHistoryOpen(false)} />}
      {genHistoryOpen && <GenHistoryModal onClose={() => setGenHistoryOpen(false)} />}

      {/*
        发布软件版本的闸（第四刀）。点②卡的「发布软件版本」开它 —— 与上面那条
        「交付预设」的闸（`gateOpen`）是**两道闸**，各有各的账。
        ★ 关框时重取发布历史：发完的那一趟已经记进 `release-history.json`，
          页面这块要跟上（历史摆在这张卡上，不再塞在框里）。
      */}
      {releaseGateOpen && (
        <ReleaseGateModal
          onClose={() => {
            setReleaseGateOpen(false)
            reloadReleaseHistory()
          }}
          currentVersion={appVersion}
        />
      )}

      {/*
        交付明细（2026-10-19）：页面上只摆汇总与「待更新」提示，完整清单在这里。
        作者：「外面就应该显示一个简略的，我点了才显示详细的」——名单与状态都来自
        后端 `DeliveryStatus`，这一框不推第二份。
      */}
      {deliveryListOpen && deliveries !== null && (
        <DeliveryFilesModal
          status={deliveries}
          preview={deliveryPreview}
          previewError={deliveryPreviewErr}
          generateTargets={generateTargets}
          generateLabel={generateLabel}
          onGenerate={() => void generateStale()}
          onClose={() => setDeliveryListOpen(false)}
        />
      )}
    </div>
  )
}

/** 色调语义 → 本页的状态徽标类名 */
function toneClass(tone: StateTone): string {
  if (tone === 'ok') return `${s.delBadge} ${s.delBadgeOk}`
  if (tone === 'todo') return `${s.delBadge} ${s.delBadgeTodo}`
  return `${s.delBadge} ${s.delBadgeMuted}`
}

