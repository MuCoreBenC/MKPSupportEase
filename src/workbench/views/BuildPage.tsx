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
import { Fragment, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  BaselineDiffEntry,
  BookView,
  Boot,
  DeliveryFile,
  GenerateReport,
  Issue,
  IssueReport,
  MirrorSync,
  PreviewReport,
  PublishTxReport,
  Words,
} from '../api'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import { locateAnchor } from '../c14/locate'
import ModalC14 from '../c14/ModalC14'
import CodeText from '../c14/CodeText'
import { deliveryStageText as STAGE_TEXT } from '../c14/labels'
import GenerateDiffModal from './GenerateDiffModal'
import PublishGateModal from './PublishGateModal'
import ReleaseGateModal from './ReleaseGateModal'
import HistoryModal from './HistoryModal'
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
  /** 生成之后让外壳重取整本（buildRows 的状态跟着走） */
  onBookRefresh: () => void
}

/** 检查项的「去处理」映射。产品的一份账里，不是每一条都有地方可去 */
function gotoOf(i: Issue): { view: string; focus?: GotoFocus } | null {
  if (i.id === 'bundle.orphan_files') {
    /* C14 第二十七轮的修正：孤儿文件去**资产库**并替人筛好「可选」——
       套餐页里根本没有那些文件，跳过去等于没指路 */
    return { view: 'assets', focus: { machineId: null, uid: null, key: 'optional' } }
  }
  switch (i.at.view) {
    case 'params':
    case 'fields':
      return { view: 'params', focus: { machineId: i.at.machineId ?? '', uid: i.at.uid, key: i.at.key } }
    case 'menu':
      return { view: 'bundles', focus: { machineId: null, uid: i.at.uid, key: null } }
    case 'stock':
      return { view: 'assets', focus: { machineId: null, uid: i.at.uid, key: null } }
    case 'fallback':
      return { view: 'assets', focus: { machineId: null, uid: null, key: null } }
    default:
      return null // build 视角的问题在本页就地滚过去，不跳页
  }
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

export default function BuildPage({ boot, book, words, report, tick, onGoto, onSave, onBookRefresh }: Props) {
  const rows = book.buildRows
  const [picked, setPicked] = useState<Record<string, boolean>>({})
  const [openRow, setOpenRow] = useState<string | null>(null)
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
  /** 上一次合并之后「把主线同步到第二个官方源」的结论（重新打开回执时还要用） */
  const [gateMirror, setGateMirror] = useState<MirrorSync | null>(null)
  /** 发布历史面板（本地回执日志；每条一个手动刷新，不轮询） */
  const [historyOpen, setHistoryOpen] = useState(false)
  /*
   * **发布软件版本**的闸（第四刀）。点②卡的「发布软件版本」开它 —— 与上面那条
   * 「发布预设」的闸（`gateOpen`）是**两道不同的闸**，各有各的账。
   */
  const [releaseGateOpen, setReleaseGateOpen] = useState(false)
  const [baseline, setBaseline] = useState<BaselineDiffEntry[] | null>(null)
  const [strays, setStrays] = useState<string[] | null>(null)
  const [trash, setTrash] = useState<Awaited<ReturnType<typeof wb.trash>> | null>(null)
  /*
   * **交付文件清单**（2026-10-07）：除了 `mkp/presets/*.toml`，交付目录里还有一串
   * 附属文件（`content/*.json`、`catalog.json`、`manifest.json`、`source.json`）。
   * 名单来自后端（与残留审计同一份交付集合），这一页只负责摆出来、点开看原文。
   */
  const [deliveries, setDeliveries] = useState<DeliveryFile[] | null>(null)
  /** 打开着的那一份交付文件（**盘上原文**，从 `wb_delivery_file` 直读） */
  const [deliveryOpen, setDeliveryOpen] = useState<{ rel: string; text: string } | null>(null)
  /** 正在取哪一份交付文件（同一时刻只会有一次） */
  const [deliveryBusy, setDeliveryBusy] = useState<string | null>(null)
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

  const blocked = (report?.blocks ?? 0) > 0
  const dirty = book.dirtyCount > 0
  const ids = Object.keys(picked).filter((k) => picked[k])

  /*
   * 「这一行能不能勾」= **不是「没有可生成的东西」那一档**（作者 2026-10-02）。
   *
   * 后端的 `row.buildable` 只覆盖 `stale | neverBuilt`（那是「要不要进**默认**生成队列」
   * 的口径）；而**已生成也能重生成** —— `planned_todos` 在 `Scope::Picked` 下本来就收
   * 任何 uid，内容没变就走 `unchanged`、不重写。作者要的是「就算没变化，我也想走一遍确认框」。
   *
   * 所以勾选只看 `NoResources`（这台压根没配方，生成出来是空的）。`buildable` 仍用于
   * 「全选待生成」那颗按钮 —— 那两个是不同的问题，别合成一个。
   */
  const pickable = (r: (typeof rows)[number]) => r.state !== 'noResources'

  /* 产物名单：**已经生成出来的那些版本**（与后端交付集合里的 `delivery/mkp/presets/` 同一批） */
  const artifacts = rows.flatMap((r) => (r.mkpFile === null ? [] : [{ uid: r.uid, fileName: r.mkpFile }]))

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

  /* 交付文件清单：进页取一次；生成 / 发布之后（tick）重取 —— 那时盘上的存在与否会变 */
  useEffect(() => {
    void wb
      .deliveryFiles()
      .then(setDeliveries)
      .catch(() => setDeliveries([]))
  }, [tick])

  /*
   * 「去处理」的定位（C14 第二十四轮）：滚动到目标模块 + 闪烁两秒。
   * 本体在 `c14/locate.ts`（目标页也用同一份 —— 参数台的「去处理」落地按它滚 + 闪）。
   */
  const goTarget = (sel: string) => {
    const isRow = sel.startsWith('t-build-') && sel !== 't-build'
    locateAnchor(sel, { row: isRow })
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
  const openGenerate = async () => {
    if (!ids.length) return
    /* 草稿不干净先落盘 —— 预演读的也是**已保存**的配方（与生成同一条顺序规矩） */
    if (dirty && !(await onSave())) return
    const picked = [...ids]
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

  /** 生成第二步：确认。真写盘，然后把框切成结果页 */
  const confirmGenerate = async () => {
    if (genPicked === null) return
    setGenBusy(true)
    try {
      const rep = await wb.generate({ picked: genPicked })
      // 生成记录由**后端生成事务直接落进台账**（built.json）—— 前端不再回填草稿，
      // 生成完成 = 台账已是这一代（2026-10-06 状态机修正）。
      setGenDone(rep)
      setPicked({})
      onBookRefresh()
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
    const rep = await wb.publish()
    setLastPublish(rep)
    /* 发布改了交付目录 → 让外壳重取（④ 残留与 ③ 基线跟着刷新） */
    onBookRefresh()
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

  /**
   * 看一份交付文件的正文。**直读盘上原文**（`wb_delivery_file`）——
   * 交付目录里现在就长这样，发布出去的就是它。
   */
  const openDelivery = async (rel: string) => {
    setDeliveryBusy(rel)
    try {
      setDeliveryOpen({ rel, text: await wb.deliveryFile(rel) })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setDeliveryBusy(null)
    }
  }

  /*
   * 「其他交付文件」= 交付集合里**不带版本概念**的那些（`mkp/presets/*.toml` 之外的）。
   * 产物那几份在上面的「发布物」组里按版本列，这里列目录 JSON / manifest / source。
   */
  const otherDeliveries = (deliveries ?? []).filter((d) => !d.rel.startsWith('mkp/presets/'))

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
            report.issues.map((i) => (
              <div
                key={i.id}
                className={`${s.issue} ${i.severity === 'block' ? s.issueBlock : ''}`}
              >
                <span className={`${s.tag} ${s[`tagSev${i.severity[0].toUpperCase()}${i.severity.slice(1)}` as keyof typeof s] ?? ''}`}>
                  {i.severity === 'block' ? '阻断' : i.severity === 'todo' ? '待办' : '提示'}
                </span>
                <span className={s.issueTxt}>
                  <b>{i.title}</b>
                  <span>{i.detail}</span>
                </span>
                {/* 每一条都要说清去哪儿处理（issues.rs 的规矩），所以每一条都有这颗按钮 */}
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  onClick={() => issueGoto(i)}
                >
                  去处理
                </button>
              </div>
            ))
          ) : (
            <div className={s.emptyHint}>{report ? report.emptyHint : '正在读检查报告……'}</div>
          )}
        </div>
      </div>

      <div className={s.flow}>
        {/* —— ① 生成 —— */}
        <div id="t-build" className={s.card}>
          <div className={s.cardHead}>
            <h2>① 生成</h2>
            <span className={s.cardNote}>判据是快照比对，不看文件时间</span>
          </div>
          <div className={s.cardBody}>
            <p className={s.note} style={{ marginTop: 0 }}>
              勾哪几个版本，就按它们<strong>当前已保存的配方</strong>整批生成。
              后端先全部算完、任一项算不出来则整批不动；字节没变的产物不重写。
            </p>
            {dirty && (
              <p className={s.note} style={{ marginTop: 0 }}>
                有<strong>未保存</strong>的改动 —— 点「生成」会先替你保存，成了才生成
                （生成读的是落盘的那份，不是屏幕上的这份）。
              </p>
            )}
            {rows.map((r) => (
              <Fragment key={r.uid}>
                <div
                  id={`t-build-${r.uid}`}
                  onClick={() => setOpenRow(openRow === r.uid ? null : r.uid)}
                  style={{ cursor: 'pointer' }}
                  className={s.row}
                >
                  {/*
                   * **已生成的行也能勾**（作者 2026-10-02）：后端本来就允许重生成 ——
                   * 内容没变就是 unchanged、不重写；而作者要的是"就算没变化我也想走一遍
                   * 确认框"。所以勾选只按 buildable 拦（机型没尺寸那种真不能生成），
                   * 不再看 state。
                   */}
                  <input
                    type="checkbox"
                    checked={!!picked[r.uid]}
                    disabled={!pickable(r)}
                    title={r.disabledReason ?? undefined}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => setPicked({ ...picked, [r.uid]: e.target.checked })}
                    aria-label={`选择 ${r.uid}`}
                  />
                  <span className={`${s.mono} ${s.rowMeta}`}>{r.uid}</span>
                  <span className={s.rowName}>{r.name}</span>
                  {/* 状态签贴右（作者 2026-10-03：以前紧跟名字，名字一长一短就歪歪扭扭） */}
                  <span className={s.grow} />
                  <span
                    className={`${s.tag} ${STATE_TAG[r.state]}`}
                    title={words.build[r.state].explain ?? undefined}
                  >
                    {words.build[r.state].label}
                  </span>
                  <span className={s.rowMeta}>{r.lastBuild ? localStamp(r.lastBuild) : '—'}</span>
                </div>
                {/* 详情是行的**兄弟**不是行里右侧的一个格（C14 第二十七轮） */}
                {openRow === r.uid && (
                  <div className={s.rowDetail}>
                    <div>
                      <b>{words.build[r.state].label}</b> —— {words.build[r.state].explain ?? ''}
                    </div>
                    {r.reason && <div>{r.reason}</div>}
                    {r.disabledReason && <div>现在生成不了：{r.disabledReason}</div>}
                    <div>
                      产物：{r.mkpFile ?? '—'} · 配套切片器 {r.bbsCount} 份
                    </div>
                  </div>
                )}
              </Fragment>
            ))}
            <div className={s.bar} style={{ marginTop: 12 }}>
              {/*
               * 两颗全选（作者 2026-10-02）：
               *   全选待生成  只勾 stale / neverBuilt —— 日常那一颗（`buildable` 的口径）
               *   全选        勾**所有能勾的**（含已生成）—— 要"重走一遍确认框"时用它
               * 两颗并存不是重复：一个问"哪些还没生成"，一个问"全部"。
              */}
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  const next: Record<string, boolean> = {}
                  for (const r of rows) {
                    if (r.buildable) next[r.uid] = true
                  }
                  setPicked(next)
                }}
              >
                全选待生成
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  const next: Record<string, boolean> = {}
                  for (const r of rows) {
                    if (pickable(r)) next[r.uid] = true
                  }
                  setPicked(next)
                }}
              >
                全选
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => setPicked({})}
              >
                全不选
              </button>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={!ids.length || blocked}
                title={blocked ? words.disabled.buildBlocked : undefined}
                onClick={() => void openGenerate()}
              >
                生成 {ids.length} 项
              </button>
            </div>
          </div>
        </div>

        {/* —— ② 发布预设 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>② 发布预设</h2>
            <span className={s.cardNote}>
              把本次预设与数据变更提交到远端仓库；合并后，客户端即可获取这些更新
            </span>
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
                发布 = 把产物与清单一起定稿进交付目录。点它是<strong>先开发布闸</strong>
                （十五项逐项打勾，全绿才给往下走）；带残留时后端一个字节都不写。
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
                disabled={blocked}
                title={
                  blocked
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
              **本次交付文件**（作者 2026-10-07 重定口径）：

              · **产物 toml 不在这里逐份列**了 —— 逐份正文在「生成前确认」那一屏里看得到
                （点「生成」先过那一屏），这一块不再长出一排「查看 TOML」；作者原话：
                「这一个我觉得没必要了呀，因为我在生成的地方就可以查看了」。
              · 这里列的是**不带版本概念**的附属文件（目录 JSON / manifest / source），
                名单来自后端那**同一份交付集合**（发布闸判残留用的就是它）——
                界面不另拼一份"大概有这些"。点「查看」读**盘上原文**。
            */}
            <div className={s.group}>
              <div className={s.groupHead}>本次交付文件</div>
              <div className={s.chips}>
                <span
                  className={s.chip}
                  title="已经生成出来的版本各一份（delivery/mkp/presets/<机型>-<版本>.toml）—— 逐份正文在「生成前确认」里看"
                >
                  {artifacts.length === 0
                    ? 'preset.toml — 未生成'
                    : `preset.toml × ${artifacts.length} 份`}
                </span>
                <span className={s.chip} title={lastPublish?.summary}>
                  {lastPublish
                    ? `本次发布落下 ${lastPublish.files} 个文件${lastPublish.commit ? ` · ${lastPublish.commit}` : ''}${lastPublish.review ? ` → PR #${lastPublish.review.number}` : ''}`
                    : '本会话还没发布过'}
                </span>
              </div>
              <p className={s.note}>
                下面这些**不带「版本」概念**，跟着生成 / 发布一起落进交付目录：
                <span className={s.mono}>content/*.json</span> 与{' '}
                <span className={s.mono}>catalog.json</span> 在<strong>生成时重算</strong>，
                <span className={s.mono}>manifest.json</span> /{' '}
                <span className={s.mono}>source.json</span> 由<strong>发布时定稿</strong>。
                所以改了备注这类不碰参数的数据，<span className={s.mono}>
                  mkp/presets/*.toml
                </span>{' '}
                一个字节都不动（生成时显示「无变化」），但{' '}
                <span className={s.mono}>catalog.json</span> 会跟着重算 ——
                客户端预设列表的副标题就是从它来的。
              </p>
              {deliveries === null ? (
                <div className={s.emptyHint}>正在读……</div>
              ) : (
                <div className={s.vstack}>
                  {otherDeliveries.map((d) => (
                    <div key={d.rel} className={s.vfield}>
                      <div className={s.vhead}>
                        <b className={s.mono}>{d.rel}</b>
                        <span className={s.vkey}>{STAGE_TEXT[d.stage]}</span>
                        <span className={s.grow} />
                        <span className={s.rowMeta}>
                          {d.stage === 'generate'
                            ? d.exist
                              ? `已生成 · ${fmtSize(d.size)}`
                              : '还没生成'
                            : d.exist
                              ? `已发布 · ${fmtSize(d.size)}`
                              : '还没发布'}
                        </span>
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          disabled={!d.exist || deliveryBusy === d.rel}
                          title={
                            d.exist
                              ? '看交付目录里这一份的盘上原文（只读）'
                              : '盘上还没有这一份 —— 先生成 / 发布再看'
                          }
                          onClick={() => void openDelivery(d.rel)}
                        >
                          {deliveryBusy === d.rel ? '正在取……' : '查看'}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/*
          —— 软件版本（只读）——
          ★ 与上面的「发布预设」是**两回事**（见 `docs/RELEASE-TRANSACTIONS.md`）：
            上面把预设/数据提交进仓库、合并后客户端就能取到；
            这一块说的是"发布一个新的 SupportEase 安装包"——**另一套事务，本轮不实现**。
          所以这里**只读展示**，不摆按钮（摆一个点不动的按钮比不摆更糟）。
        */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>软件版本</h2>
            <span className={s.cardNote}>用于发布新的 SupportEase 安装包（与上面发预设是两回事）</span>
          </div>
          <div className={s.cardBody}>
            <div className={s.bar}>
              <span className={s.cardNote}>
                当前：已安装 <span className={s.mono}>{appVersion ?? '未知'}</span>
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
            <p className={s.note}>
              预设更新**不需要**新的安装包 —— 走上面的「发布预设」即可。只有改动程序本身
              （Rust / React / 数据读取能力 / 客户端功能）才需要发布一个软件版本。
              发完还要**合并 release.json 那个 PR** —— 客户端才看得到新版本。
            </p>
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
              入库产物 vs 判据基线（14.9）。**人看过这份清单再点同步** —— 内容相同的自动跳过，
              只有真的变了才会写。
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
                title={
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
              「不在本次交付集合内」的文件 —— 残留会被消费端真的下载到，所以发布被它拦下。
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
              删除版本时它的生成快照进这里。**这一页只读** —— 还原与彻底删除的动作
              还没有命令（待裁决项），条目先列出来让人看得见。
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

      {/*
        发布软件版本的闸 → 回执 → 历史（第四刀）。
        ★ 与上面那个 `PublishGateModal` 是**两道闸**：这道发安装包，那道发预设。
      */}
      {releaseGateOpen && (
        <ReleaseGateModal
          onClose={() => setReleaseGateOpen(false)}
          currentVersion={appVersion}
        />
      )}

      {/* 交付文件：目录 JSON / manifest / source —— 正文是**盘上原文**（框给到最大那档） */}
      {deliveryOpen !== null && (
        <ModalC14
          open
          size="xl"
          title={`交付文件 ${deliveryOpen.rel}`}
          subtitle="交付目录里的盘上原文（只读）—— 客户端或云端取的就是这一份"
          onClose={() => setDeliveryOpen(null)}
        >
          <div className={s.vstack}>
            <p className={s.note}>
              正文是<span className={s.mono}>wb_delivery_file</span>直读的**盘上原文**
              —— 交付目录里现在就长这样，发布出去的就是它（盘上没有它时按钮是灰的）。
            </p>
            <CodeText
              fileName={deliveryOpen.rel}
              text={deliveryOpen.text}
              className={`${s.codeText} ${s.codeTextTall}`}
            />
          </div>
        </ModalC14>
      )}
    </div>
  )
}

/** 生成态四档的样式（与机型页共用同一张类名表） */
const STATE_TAG: Record<string, string> = {
  built: s.tagBuildBuilt,
  stale: s.tagBuildStale,
  neverBuilt: s.tagBuildNever,
  noResources: s.tagBuildNone,
}

/** 字节数给人看（交付文件那一行） */
function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}
