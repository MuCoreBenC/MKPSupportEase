/**
 * 「生成与发布」页 —— C14 版式移植（feat/b05-14b-c14-port，build 视角落地）。
 *
 * C13 的两页在这一页合成一页（作者：「发布中心和整包检查与生成有点重复了」），
 * 版式沿用左右分布（作者：「左边是那些事项、未完成的提示，右边才是主要操作」）：
 *
 *   左栏  检查        一份账（wb_preflight）：能不能生成 + 交出去会不会缺东西
 *   右栏  ① 生成      勾中版本按已保存的配方生成（原子整批，字节没变不重写）
 *         ② 客户端数据包   只读：包里有什么 + 兼容声明（渠道 / 最低客户端版本）
 *         ③ 发布      检查过了才亮 —— 后端还有一道硬闸（残留拦截在发布内部）
 *         ④ 基线      产物 vs 对照基线，人看过 diff 才同步（14.9）
 *         ⑤ 残留      交付目录里「不在本次交付集合内」的文件，清理走 .trash 回收
 *         ⑥ 回收站    删掉的生成快照（只读清单）
 *         ⑦ 子目录职责  workbench/ 五个子目录谁写谁读（14.7，后端给的一句话）
 *
 * # 闸门按钮（14.2）与后端硬闸的行为一致
 *
 * 生成 / 发布按钮的禁用读 `wb_preflight` 的阻断计数（每次写完外壳重取），
 * 后端在 `wb_generate` / `wb_publish` 开头**还会再查一遍**—— 前端只是把同一份
 * 判据提前画在按钮上，不是第二条闸。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的
 *
 *  - **包版本 / 最低客户端版本不是输入框**：上游整层删掉之后这两格没有来源 ——
 *    `wb_publish` 的 `PublishMeta` 里只有 channel 是发布常量，版本与最低客户端版本
 *    照实留空。「最低客户端版本未声明」在左栏是一条待办，那句话自己写着
 *    「这不是我们该填的空」—— 所以这里只读，不造一个输入框出来。
 *  - **检查只有一份账**：产品的 wb_preflight 是一份报告（引用 / 参数 / 唯一性 /
 *    孤儿 + 清单↔配方对齐），不像原型分 checkIssues / publishIssues 两份。
 *  - **行上没有「未保存」档**：BuildRow 不带 savedYet；未保存时按钮先走
 *    「保存再生成」（一个按钮两个动作，顺序规定死 —— 机型页同款）。
 *  - **没有内容指纹**：原型的 inputsHash 在产品侧没有对应物；产物的新旧由
 *    每一版自己的快照比对判（BuildRow.state），「② 卡」不重复一个全局哈希。
 *  - **恢复配方（wb_revert_preview）不在这页**：按版本走机型页的版本卡 ——
 *    恢复的是「某一版」的配方，不是整包的动作。
 *
 * # C15 增量：发布物摸得着、云端那一步真的通（P6）
 *
 * 原型这一稿往这页加了三件：② 变成真包（说明书 JSON）、③ 多一枚「上传到云端」、
 * 发布物看得见（说明书 + N 份 preset.toml，同属一次发布）。**两样产物**这条模型
 * 在本仓照样成立（合同在 `src/api/contract.ts` 的 `Release` / `ReleasePreset`）：
 *
 *   ② 说明书        `clientPackage.ts` 现装一份真包（机型 / 版本 / 字段 / 值 / 版本轴）
 *   ② 包版本        工作台不替发布定版本号（`wb_publish` 的 meta 里这一格留空），
 *                   这里只从 1.0.0 起步加三枚快捷，算出「下一版该填的数」给人参考
 *   ② 兼容性清单    客户端团队那张表（`src/workbench/compat.ts` 的 `CLIENT_COMPAT`），
 *                   外加按木桶原理算出来的「自动判断：x.y.z」—— 常显，悬停看它命中了哪几样
 *   ③ 发布物清单    产物名单与文件名来自 `BuildRow.mkpFile`（Rust 交付集合里那几份
 *                   `presets/mkp/*.toml`），TOML 正文走 `wb_preview_toml`（**真 Rust 渲染器**）
 *   ③ 查看 JSON     这次发布的那份**说明书原样**（`ClientDataPackage`）+ `wb_publish` 的返回
 *   ③ 上传到云端    整个 release（说明书 + 预设文件）写进 `STORAGE.cloud` ——
 *                   客户端「同步」页读的就是这一格，联动这条链到此闭合
 *
 * # 说明书生成器这本账
 *
 * 它是「前端算业务」那条纪律上的一个**已登记的例外**：产物生成本来在 Rust，本轮按
 * `C15-A40-PORT-PLAN.md` §6 那条建议先落在**一处前端模块**（`src/workbench/clientPackage.ts`）
 * 里，好让联动这条链当场走得通（决策 #5：本轮就要求真的联通），同时在 `tasks.md`
 * 债 #1 / #8 里登记「下一轮搬进 Rust」。搬过去那天，这一页一个字不动。
 */
import { Fragment, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  BaselineDiffEntry,
  BookView,
  Boot,
  Issue,
  IssueReport,
  PublishReport,
  Words,
} from '../api'
/* 说明书的形状走**客户端的契约**（只借类型，编译期擦除）—— 工作台不因此把客户端的
   api / bridge / mock 拖进自己的 bundle，见 `src/workbench/compat.ts` 的同一条注 */
import type { ClientDataPackage } from '../../api/contract'
import { toasts } from '../c14/toast'
import type { GotoFocus } from '../c14/types'
import { locateAnchor } from '../c14/locate'
import ModalC14 from '../c14/ModalC14'
import { CLIENT_COMPAT, minClientOf, verdictTextOf } from '../compat'
import { buildClientPackage, buildRelease, collectInputs } from '../clientPackage'
import { cloudSummary, listCloud, removeFromCloud, uploadToCloud } from '../cloud'
import type { CloudEntry } from '../cloud'
import s from '../c14.module.css'

/** 发布渠道常量 —— 与后端 `app::build::PUBLISH_CHANNEL` 同一个值（上游整层删掉后的唯一来源） */
const PUBLISH_CHANNEL = 'stable'

interface Props {
  boot: Boot
  book: BookView
  words: Words
  /** 外壳每次写完重取的检查报告 —— 闸门按钮与左栏都读它 */
  report: IssueReport | null
  /** 外壳的「后端状态变过了」计数：生成 / 撤销之后重取基线与残留 */
  tick: number
  onGoto: (view: string, focus?: GotoFocus) => void
  /** 草稿写入口（生成记录 MarkBuilt 从这里过 —— 不可撤销，不进栈） */
  onApply: (label: string, patches: import('../api').Patch[]) => Promise<void>
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

/* ---------- 版本号的小工具（照原型）：从 1.0.0 起，只认 x.y.z 三段数字 ---------- */

type Triple = [number, number, number]

/** 在第 `at` 段加一格，后面几段归零（`1.4.7` +0.1.0 → `1.5.0`） */
const bumpVer = (t: Triple, at: 0 | 1 | 2): string =>
  t.map((x, i) => (i === at ? x + 1 : i > at ? 0 : x)).join('.')

/**
 * 「什么时候发的」那一刻，给人看的写法（与静态快照里那份 `at` 同款：`2026-09-28 10:12`）。
 *
 * 时刻记在**云端目录项**上、不往包里塞（`ClientDataPackage` 里根本没有这一栏）——
 * 假后端没有可信的时间源，「这是第几版、什么时候发的」由**上传的人**记。
 */
function stampNow(): string {
  const d = new Date()
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

export default function BuildPage({ boot, book, words, report, tick, onGoto, onApply, onSave, onBookRefresh }: Props) {
  const rows = book.buildRows
  const [picked, setPicked] = useState<Record<string, boolean>>({})
  const [openRow, setOpenRow] = useState<string | null>(null)
  const [lastPublish, setLastPublish] = useState<PublishReport | null>(null)
  const [baseline, setBaseline] = useState<BaselineDiffEntry[] | null>(null)
  const [strays, setStrays] = useState<string[] | null>(null)
  const [trash, setTrash] = useState<Awaited<ReturnType<typeof wb.trash>> | null>(null)
  /** 展开看的那个「这次发布的记录」（查看 JSON） */
  const [jsonOpen, setJsonOpen] = useState(false)
  /** 打开着的那一份产物 TOML（正文从 `wb_preview_toml` 现取） */
  const [tomlOpen, setTomlOpen] = useState<{ uid: string; fileName: string; text: string } | null>(null)
  /** 正在取哪一份产物的正文（同一时刻只会有一次） */
  const [tomlBusy, setTomlBusy] = useState<string | null>(null)
  /** 云端那一格（静态快照 + 我刚传的） */
  const [cloud, setCloud] = useState<CloudEntry[]>([])
  /** 包版本的「下一版该填什么」——`null` = 还没手动点过，显示建议值 */
  const [ver, setVer] = useState<string | null>(null)
  /** 现装出来的那一份说明书（`clientPackage.ts`）—— ② 卡与「上传到云端」都读它 */
  const [pkg, setPkg] = useState<ClientDataPackage | null>(null)
  /** 装说明书时读命令挂了。有它就是「还没装出来」，不编一份顶上 */
  const [pkgErr, setPkgErr] = useState<string | null>(null)
  /** 正在上传（装说明书的读命令有六次 `wb_desk`，按钮要压得住重复点） */
  const [upBusy, setUpBusy] = useState(false)

  const blocked = (report?.blocks ?? 0) > 0
  const dirty = book.dirtyCount > 0
  const ids = Object.keys(picked).filter((k) => picked[k])

  /*
   * 包版本（C15）：建议值从 1.0.0 起步（第一版不做加法 —— 「从 1.0.0 加一格」是那三枚快捷的事）。
   *
   * 工作台不替发布定版本号：`wb_publish` 的 meta 里这一格留空（上游整层删掉后没有来源），
   * 发布的返回值里也没有版本号。所以这里算出的是「下一版该填什么数」，
   * 不是一个能回头改发布的输入框。
   */
  const suggestVer = ver ?? '1.0.0'

  /* 产物名单：**已经生成出来的那些版本**（与后端交付集合里的 `presets/mkp/` 同一批） */
  const artifacts = rows.flatMap((r) => (r.mkpFile === null ? [] : [{ uid: r.uid, fileName: r.mkpFile }]))

  /* 云端那一格：进页取一次。真云端在本轮不存在，见 cloud.ts 的文件头 */
  useEffect(() => {
    void listCloud().then(setCloud)
  }, [])

  /*
   * 说明书：进页装一次，后端每写一次（`tick`）重装。
   *
   * 它读的是**落盘那一份**（全部来自 `wb_*` 那几条读命令）—— 所以机型 / 版本 / 配方
   * 改完并保存之后，这里出来的就是新的；屏幕上的未保存改动进不来（草稿在 Rust 那侧）。
   */
  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const next = buildClientPackage(await collectInputs(), {
          minClientVersion: null,
        })
        if (!alive) return
        setPkg(next)
        setPkgErr(null)
      } catch (e) {
        if (!alive) return
        setPkg(null)
        setPkgErr(isAppError(e) ? e.message : String(e))
      }
    })()
    return () => { alive = false }
  }, [tick])

  /* 兼容性清单的结论：拿这一份包去扫那张表（木桶原理），常显那个数、悬停看全账 */
  const compat = pkg === null ? null : minClientOf(pkg)
  const condCount = pkg === null ? 0 : pkg.fields.filter((f) => f.showWhen !== undefined).length
  const pkgVersions = pkg === null ? 0 : pkg.machines.reduce((n, m) => n + m.versions.length, 0)

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

  /*
   * 「去处理」的定位（C14 第二十四轮）：滚动到目标模块 + 闪烁两秒。
   * 本体在 `c14/locate.ts`（目标页也用同一份 —— 参数台的「去处理」落地按它滚 + 闪）。
   */
  const goTarget = (sel: string) => {
    const isRow = sel.startsWith('t-build-') && sel !== 't-build'
    locateAnchor(sel, { row: isRow })
  }

  const issueGoto = (i: Issue) => {
    /*
     * 本页有格子可滚的那几条先就地滚过去。「最低客户端版本未声明」那一条的格子
     * 在 ② 卡里（C15 起那一栏有 id）—— 跳到 ① 生成上等于没指路。
     */
    if (i.id.includes('min_client') || i.id.includes('minimum_client')) {
      goTarget('t-min-client')
      return
    }
    /* 本页的（build 视角）就地滚；先落盘的行没生成过，锚点就是 ① 卡 */
    if (i.at.view === 'build') {
      goTarget(i.at.uid ? `t-build-${i.at.uid}` : 't-build')
      return
    }
    const g = gotoOf(i)
    if (g) onGoto(g.view, g.focus)
  }

  /** 生成。勾中的行按当前已保存的配方整批生成 —— 后端原子：任一项算不出则整批不动 */
  const generate = async () => {
    if (!ids.length) return
    /* 草稿不干净先落盘 —— 生成读的是**已保存**的配方（一个按钮两个动作，顺序规定死） */
    if (dirty && !(await onSave())) return
    try {
      const rep = await wb.generate({ picked: ids })
      // 生成记录走唯一写入口落进草稿（不可撤销 —— 它是记录，不是编辑）
      await onApply(`生成记录：${rep.written.length + rep.unchanged.length} 份`, [rep.mark])
      const parts = [
        rep.written.length > 0 ? `写出 ${rep.written.length} 份` : null,
        rep.unchanged.length > 0 ? `${rep.unchanged.length} 份内容没变、跳过重写` : null,
        ...rep.skipped.map(([uid, why]) => `跳过 ${uid}（${why}）`),
      ].filter((x): x is string => x !== null)
      toasts.push(`已生成：${parts.join('；') || '没有可生成的项'}`)
      setPicked({})
      onBookRefresh()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 发布。后端还有两道硬闸：检查阻断 + 残留拦截（一个字节都不许在带残留时写出） */
  const publish = async () => {
    try {
      const rep = await wb.publish()
      setLastPublish(rep)
      toasts.push(`已发布 ${rep.files} 个文件到交付目录（最低客户端版本：${rep.minimumClient ?? '未声明'}）`)
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
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

  /** 清理交付残留。走 .trash/dist/<stamp>/ 回收，不直接删 */
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
   * 看一份产物的正文。**正文来自后端**（`wb_preview_toml` 就是生成时那台渲染器）——
   * 前端不拼一份「大概是这样」的 TOML 出来：那样看到的和发布的就不是同一份东西。
   */
  const openToml = async (uid: string, fileName: string) => {
    setTomlBusy(uid)
    try {
      setTomlOpen({ uid, fileName, text: await wb.previewToml(uid) })
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setTomlBusy(null)
    }
  }

  /** 从云端删一份（只删本地那份；静态快照删不动） */
  const dropCloud = (e: CloudEntry) => {
    removeFromCloud(e.id)
    void listCloud().then(setCloud)
    toasts.push(`已从云端删掉 ${e.name}（模拟）`)
  }

  /**
   * 上传到云端（模拟）。**整个 release 一起上去** —— 说明书 + 那几份 `preset.toml`，
   * 同属一个 preset identity（「JSON 是 1.0.1、TOML 还是 1.0.0」这种假链路不许出现）。
   *
   * 落点是 `STORAGE.cloud`：客户端「同步」页读的是**同一格**，联动这条链就靠它。
   * 包版本取上面那一格算出来的数（上游声明 +0.0.1 起步 / 手点快捷），
   * 上传的那一刻由这一侧记进目录项。
   */
  const upload = async () => {
    if (upBusy) return
    setUpBusy(true)
    try {
      const inp = await collectInputs()
      const release = await buildRelease(inp, {
        version: suggestVer,
        at: stampNow(),
        minClientVersion: null,
      })
      uploadToCloud({
        id: `workbench-${suggestVer}`,
        name: `工作台发布 ${suggestVer}`,
        version: suggestVer,
        at: release.at,
        package: release.package,
        presets: release.presets,
      })
      /* 顺手把 ② 卡那份换成刚上传的这一份 —— 二者同源，不该长得不一样 */
      setPkg(release.package)
      setCloud(await listCloud())
      toasts.push(
        `已上传云端：说明书 ${release.package.fields.length} 个字段 + ${release.presets.length} 份预设文件`
        + '（客户端「同步」页读的就是这一格）',
      )
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setUpBusy(false)
    }
  }

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
                {/*
                  每一条都要说清去哪儿处理（issues.rs 的规矩）。最低客户端版本那一条
                  的说明自己写着「这不是我们该填的空」—— 它没有「去处理」，不摆按钮。
                */}
                {i.id !== 'compat.minimum_client' && (
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    onClick={() => issueGoto(i)}
                  >
                    去处理
                  </button>
                )}
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
                  <input
                    type="checkbox"
                    checked={!!picked[r.uid]}
                    disabled={!r.buildable}
                    title={r.disabledReason ?? undefined}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => setPicked({ ...picked, [r.uid]: e.target.checked })}
                    aria-label={`选择 ${r.uid}`}
                  />
                  <span className={`${s.mono} ${s.rowMeta}`}>{r.uid}</span>
                  <span className={s.rowName}>{r.name}</span>
                  <span
                    className={`${s.tag} ${STATE_TAG[r.state]}`}
                    title={words.build[r.state].explain ?? undefined}
                  >
                    {words.build[r.state].label}
                  </span>
                  <span className={s.grow} />
                  <span className={s.rowMeta}>{r.lastBuild ?? '—'}</span>
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
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  const next: Record<string, boolean> = {}
                  for (const r of rows) {
                    if (r.buildable && (r.state === 'neverBuilt' || r.state === 'stale')) next[r.uid] = true
                  }
                  setPicked(next)
                }}
              >
                全选待生成
              </button>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={!ids.length || blocked}
                title={blocked ? words.disabled.buildBlocked : undefined}
                onClick={() => void generate()}
              >
                生成 {ids.length} 项
              </button>
            </div>
          </div>
        </div>

        {/* —— ② 客户端数据包 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>② 客户端数据包</h2>
            <span className={`${s.tag} ${lastPublish ? s.tagAccent : s.tagGhost}`}>
              {lastPublish ? `本会话已发布 ${lastPublish.files} 个文件` : '还没发布过'}
            </span>
          </div>
          <div className={s.cardBody}>
            {/*
              三个版本轴（C15 把原型 ② 卡那一排搬过来，按本仓的账重排）：
              上游整层删掉之后，包版本与最低客户端版本都没有来源 —— `wb_publish` 的
              `PublishMeta` 里只剩 channel 是发布常量，其余照实留空。「未声明」在左栏
              是一条待办，它那句话自己写着「这不是我们该填的空」。数据结构版本
              （schemaVersion）本仓没有这一格。
            */}
            <div className={s.kv}>
              <span className={s.kvKey}>渠道</span>
              <span className={s.kvVal}>{PUBLISH_CHANNEL}</span>
              <span className={s.kvKey}>发布目录</span>
              <span className={`${s.kvVal} ${s.mono}`}>{boot.roots.dist}</span>
            </div>

            <div className={s.group}>
              <div className={s.groupHead}>三个版本，各管一件事</div>
              <div className={s.vstack}>
                <div id="t-pkg-ver" className={s.vfield}>
                  <div className={s.vhead}>
                    <b>包版本</b>
                    <span className={s.vkey}>latestRelease</span>
                    <span className={s.grow} />
                    <div className={s.vrow}>
                      {(
                        [
                          [2, '+0.0.1'],
                          [1, '+0.1.0'],
                          [0, '+1.0.0'],
                        ] as const
                      ).map(([at, label]) => (
                        <button
                          key={label}
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          title="从 1.0.0 往上加一格"
                          onClick={() => setVer(bumpVer([1, 0, 0], at))}
                        >
                          {label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <p className={s.vhelp}>
                    这次发出去的包是第几版 —— 客户端拿它对比自己上次拉到几，决定要不要重新下载。
                  </p>
                  <div className={s.vrow}>
                    <span className={s.vstatic}>{suggestVer}</span>
                    <span className={s.cardNote}>工作台不读上游声明 —— 从 1.0.0 起算</span>
                  </div>
                  <p className={s.vhelp}>
                    上面那三枚算的是<strong>下一版该填什么数</strong>，给人参考。本轮工作台不替
                    发布定版本号（`wb_publish` 的 meta 里这一格留空），所以这里不是一个能回头
                    改发布结果的输入框。
                  </p>
                </div>

                <div id="t-min-client" className={s.vfield}>
                  <div className={s.vhead}>
                    <b>最低客户端版本</b>
                    <span className={s.vkey}>minimumClient</span>
                    <span className={s.grow} />
                    <span
                      className={`${s.tag} ${s.tagGhost}`}
                      title={compat === null ? undefined : verdictTextOf(compat)}
                    >
                      自动判断：{compat === null ? '—' : compat.version}
                    </span>
                  </div>
                  <p className={s.vhelp}>
                    能用这份数据的<strong>最老</strong>客户端 App。低于它的客户端会被提示升级
                    —— 数据不拦，只是提醒。
                  </p>
                  <div className={s.vrow}>
                    <span className={s.vstatic}>
                      <span className={s.kvDim}>未声明</span>
                    </span>
                    <span className={s.cardNote}>
                      上游整层删掉之后没有来源 —— 照实留空（左栏有待办）
                    </span>
                  </div>
                  {/*
                    「自动判断」的**依据**先摆出来（客户端团队维护的那张表就在
                    `src/workbench/compat.ts` 里，逐行原样读，不在界面上重抄一份）。
                    按不了的原因写在表下面 —— 原型那颗按钮要的是**一份说明书**，
                    本仓后端还没有产出它的命令（见文件头那段账）。
                  */}
                  <p className={s.vhelp}>客户端兼容性清单 —— 包里用到哪一样，门槛就抬到哪一档：</p>
                  <div className={s.vstack}>
                    {CLIENT_COMPAT.map((f) => (
                      <div key={f.id} className={s.vrow}>
                        <span className={`${s.vkey} ${s.mono}`}>{f.since}</span>
                        <span>{f.label}</span>
                      </div>
                    ))}
                  </div>
                  {/*
                    常显的结论 —— 算法不许是黑盒：给数字，悬停看到它命中了哪几样
                    （全账在 `verdictTextOf` 那一句里）。它扫的是③ 卡里现装的那份说明书。
                  */}
                  <div className={s.vrow}>
                    <span className={s.vstatic} title={compat === null ? undefined : verdictTextOf(compat)}>
                      {compat === null ? '—' : compat.version}
                    </span>
                    <span className={s.cardNote}>
                      {compat === null
                        ? (pkgErr ?? '正在装说明书……')
                        : `命中 ${compat.hits.length} 样特性，取最高的那一档（悬停看全账）`}
                    </span>
                  </div>
                  <p className={s.vhelp}>
                    「自动判断」拿这一页现装的说明书去扫<span className={s.mono}> src/workbench/compat.ts </span>
                    那张表，按木桶原理取命中特性里最高的那一档。它是**建议**：本仓这一格填的是
                    上游 manifest 的声明，工作台不替上游写 manifest —— 上游未声明时，
                    上面这个数就是该往那儿填的。
                  </p>
                </div>
              </div>
            </div>

            <div className={s.group}>
              <div className={s.groupHead}>包里有什么</div>
              <div className={s.chips}>
                {/* 这几个数**从现装的这份包上数**，不从别处抄 —— 卡里报的就是要传出去的那份东西 */}
                {pkg !== null ? (
                  <>
                    <span className={s.chip}>{pkg.machines.length} 台机型</span>
                    <span className={s.chip}>{pkgVersions} 个版本</span>
                    <span className={s.chip}>{pkg.fields.length} 个字段</span>
                    <span className={s.chip}>带条件 {condCount}</span>
                    <span className={s.chip}>可选文件 {pkg.optionalFiles.length}</span>
                    <span className={`${s.chip} ${s.mono}`} title="输入指纹（FNV-1a）——客户端拿它判「要不要重新同步」">
                      #{pkg.inputsHash}
                    </span>
                  </>
                ) : (
                  <span className={s.chip}>{pkgErr ?? '正在装说明书……'}</span>
                )}
              </div>
              <p className={s.note}>
                客户端看不到「机型基底」这一层 —— 包里没有 origin、没有「这一项是谁给的」。
                继承规则是后厨的事，客户端重算就等于两边各留一份规则，必然分叉。
              </p>
            </div>
          </div>
        </div>

        {/* —— ③ 发布 —— */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>③ 发布</h2>
            <span className={s.cardNote}>
              生成得出来 ≠ 可以发布 —— 这一步问的是「交出去客户端会不会缺东西」
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
                发布 = 把产物与清单一起定稿进交付目录；带残留时后端一个字节都不写。
              </span>
              <span className={s.grow} />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={blocked}
                title={blocked ? words.disabled.publishBlocked : undefined}
                onClick={() => void publish()}
              >
                发布
              </button>
            </div>

            {/*
              本次发布的**发布物**（C15）。作者要工作台也看得见这两样：
              一次发布 = 说明书（客户端自动同步）+ N 份 preset.toml（用户「获取预设」拿到的那份），
              同属一个 preset identity。本仓这前半样还没有产出者 —— 见文件头那段账，
              界面上照实写出来，不拿产物名单冒充它。
            */}
            <div className={s.group}>
              <div className={s.groupHead}>
                本次发布的发布物
                <span className={s.grow} />
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  title="看这次发布后端真报回来的那份记录（只读）"
                  onClick={() => setJsonOpen(true)}
                >
                  查看 JSON
                </button>
              </div>
              <div className={s.chips}>
                {/*
                  说明书那一枚报的是**现装的这份包**（② 卡同一份，`clientPackage.ts`）——
                  一次发布的两样产物同属一个 preset identity，数要对得上。
                */}
                <span
                  className={s.chip}
                  title="ClientDataPackage —— 客户端自动同步的那一份（由 src/workbench/clientPackage.ts 现装）"
                >
                  {pkg === null
                    ? `说明书 — ${pkgErr ?? '正在装……'}`
                    : `说明书 × 1 · ${pkg.fields.length} 字段（带条件 ${condCount}）`}
                </span>
                <span className={s.chip} title="已经生成出来的版本各一份：presets/mkp/<机型>-<版本>.toml">
                  {artifacts.length === 0
                    ? 'preset.toml — 未生成'
                    : `preset.toml × ${artifacts.length} 份`}
                </span>
                <span className={s.chip}>包版本（建议）{suggestVer}</span>
                <span className={s.chip}>
                  {lastPublish
                    ? `本次发布落下 ${lastPublish.files} 个文件 · ${lastPublish.stamp}`
                    : '本会话还没发布过'}
                </span>
              </div>
              {artifacts.length > 0 && (
                <div className={s.vstack}>
                  {artifacts.slice(0, 3).map((a) => (
                    <div key={a.uid} className={s.vfield}>
                      <div className={s.vhead}>
                        <b className={s.mono}>{a.fileName}</b>
                        <span className={s.vkey}>{a.uid}</span>
                        <span className={s.grow} />
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          disabled={tomlBusy === a.uid}
                          title="看这一份预设文件长什么样（正文由后端渲染器现出，只读）"
                          onClick={() => void openToml(a.uid, a.fileName)}
                        >
                          {tomlBusy === a.uid ? '正在取……' : '查看 TOML'}
                        </button>
                      </div>
                    </div>
                  ))}
                  {artifacts.length > 3 && (
                    <p className={s.note}>
                      还有 {artifacts.length - 3} 份 —— 「查看 JSON」里有全名单。
                    </p>
                  )}
                </div>
              )}
              <p className={s.note}>
                说明书是给客户端<strong>自动同步</strong>的；
                <strong>预设文件才是用户点「获取预设」时下载的那一份</strong>。两样东西
                <strong>一起</strong>上传（下面那一枚整份 release 上去）—— 所以不会出现
                「JSON 是一版、TOML 是另一版」。产物名单与 TOML 正文都从后端来
                （<span className={s.mono}>wb_preview_toml</span> 就是生成时那台渲染器），
                说明书由 <span className={s.mono}>src/workbench/clientPackage.ts</span> 现装
                —— 那是「前端算业务」边上的一个已登记例外，见本页页头那一段。
              </p>
            </div>

            {/*
              云端 preset 文件夹（模拟）。**真实的线是一条管道**：工作台生成 → 上传云端 →
              客户端去那一格下载（键 `mkp.cloud.presets`，客户端读同一个键）。
              本仓没有真云端 —— 这一格就是那个模拟（见 `src/workbench/cloud.ts`）。
            */}
            <div className={s.group}>
              <div className={s.groupHead}>
                云端 preset 文件夹（模拟）
                <span className={s.grow} />
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  disabled={upBusy || pkg === null}
                  title={pkg === null
                    ? (pkgErr ?? '说明书还在装，等它出来再传')
                    : `把整份 release 传上去：说明书 + ${artifacts.length} 份 preset.toml。客户端「同步」页读的就是这一格`}
                  onClick={() => void upload()}
                >
                  {upBusy ? '正在上传……' : '上传到云端'}
                </button>
              </div>
              {cloud.length === 0 ? (
                <p className={s.note}>
                  这一格现在是空的：静态快照（public/cloud/presets.json）读不到，本机也没有
                  传过东西 —— 读不到就当空的，不编几份出来。
                </p>
              ) : (
                <div className={s.vstack}>
                  {cloud.map((e) => (
                    <div key={`${e.from}-${e.id}`} className={s.vfield}>
                      <div className={s.vhead}>
                        <b>{e.name}</b>
                        <span className={s.vkey}>{e.from === 'static' ? '云端已有' : '我刚传的'}</span>
                        <span className={s.grow} />
                        <span className={s.cardNote}>
                          {e.version ? `包版本 ${e.version}` : '（没记版本）'}
                          {e.at ? ` · ${e.at}` : ''}
                        </span>
                        {e.from === 'local' && (
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm}`}
                            title="从云端删掉这一份（静态快照删不动）"
                            onClick={() => dropCloud(e)}
                          >
                            删除
                          </button>
                        )}
                      </div>
                      <p className={s.vhelp}>{cloudSummary(e)}</p>
                    </div>
                  ))}
                </div>
              )}
              <p className={s.note}>
                上面这几份是<strong>读</strong>出来的：静态那一份来自 public/cloud/presets.json
                （别人发过的种子，来源标「云端已有」），「我刚传的」那几份来自 localStorage 的
                <span className={s.mono}> mkp.cloud.presets </span>—— 客户端读的也是这同一格。
              </p>
              <p className={s.note}>
                「上传到云端」把<strong>整份 release</strong> 写进下面这一格：说明书
                （<span className={s.mono}>ClientDataPackage</span>）+ 那几份 preset.toml
                （正文走 <span className={s.mono}>wb_preview_toml</span>）—— 一次发布的两样产物
                同属一个 preset identity，所以不许分批传。包版本取上面那一格算出来的数，
                上传的时刻由这一侧记进目录项。
              </p>
              <p className={s.note}>
                <strong>客户端读的是同一格</strong>（<span className={s.mono}>src/app/store/package.ts</span>
                里的 <span className={s.mono}>STORAGE.cloud</span>）：客户端「同步」页进页就把这份
                说明书自动同步下来（指纹一样就一个字不动），预设页的云端表随之多出几行
                「工作台发布 · x.y.z」—— 在那儿<strong>下载</strong>到本机预设目录，再到本地表点
                <strong>应用</strong>生效。这条链就是「工作台生成 → 上传 → 客户端同步 → 下载 → 应用」。
              </p>
            </div>
          </div>
        </div>

        {/* —— ④ 对照基线 —— */}
        <div id="t-baseline" className={s.card}>
          <div className={s.cardHead}>
            <h2>④ 对照基线</h2>
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

        {/* —— ⑤ 交付残留 —— */}
        <div id="t-strays" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑤ 交付残留</h2>
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
                title="走 workbench/.trash/dist/ 回收（保留相对路径，可还原），不直接删"
                onClick={() => void cleanStrays()}
              >
                清理残留
              </button>
            </div>
          </div>
        </div>

        {/* —— ⑥ 回收站 —— */}
        <div id="t-trash" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑥ 回收站</h2>
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

        {/* —— ⑦ 子目录职责（14.7） —— */}
        <div id="t-store-dirs" className={s.card}>
          <div className={s.cardHead}>
            <h2>⑦ 子目录职责</h2>
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

      {/*
        这次发布的那份**说明书原样**（C15 的「查看 JSON」）。
        它不是另算一份：就是 `upload` 传出去的那个 `release.package` ——
        所以「看到的」和「客户端同步下来的」逐字节相同。
      */}
      <ModalC14
        open={jsonOpen}
        size="lg"
        title="这次发布的那份说明书（原样）"
        subtitle={
          pkg === null
            ? (pkgErr ?? '还在装……')
            : `${pkg.machines.length} 台机型 · ${pkgVersions} 个版本 · ${pkg.fields.length} 个字段 · 指纹 #${pkg.inputsHash}`
        }
        onClose={() => setJsonOpen(false)}
      >
        <div className={s.vstack}>
          <p className={s.note}>
            这一格就是「上传到云端」传出去的那一份（<span className={s.mono}>CloudEntry.package</span>），
            客户端「同步」页自动拉走的就是它。<span className={s.mono}>release</span> 那一栏把
            这一次发布的<strong>两样产物</strong>合起来看：说明书 + N 份 preset.toml
            —— 同属一个 preset identity，不会出现「JSON 是一版、TOML 是另一版」。
          </p>
          <textarea
            className={s.pkgJson}
            readOnly
            spellCheck={false}
            rows={20}
            aria-label="本次发布的说明书 JSON"
            value={JSON.stringify(
              {
                release: {
                  version: suggestVer,
                  channel: PUBLISH_CHANNEL,
                  /* 上游整层删掉之后没有来源；「自动判断」的建议见 ② 卡 */
                  minimumClient: null,
                  presets: artifacts.map((a) => a.fileName),
                },
                clientPackage: pkg,
                /* `wb_publish` 的返回原样；还没发布过就是 null */
                published: lastPublish,
              },
              null,
              1,
            )}
          />
        </div>
      </ModalC14>

      {/* 另一半发布物：用户真正下载的那一份 —— 正文来自后端的渲染器 */}
      {tomlOpen !== null && (
        <ModalC14
          open
          size="lg"
          title={`预设文件 ${tomlOpen.fileName}`}
          subtitle={`${tomlOpen.uid} —— 这才是用户「获取预设」拿到的那一份`}
          onClose={() => setTomlOpen(null)}
        >
          <div className={s.vstack}>
            <p className={s.note}>
              正文由后端的生成器现出（wb_preview_toml 就是生成产物那台渲染器）——
              不是前端拼一份「大概长这样」。客户端那条链上，说明书是<strong>自动同步</strong>的，
              这一份要用户点「获取预设」。
            </p>
            <textarea
              className={s.pkgJson}
              readOnly
              spellCheck={false}
              rows={18}
              aria-label="预设文件 TOML"
              value={tomlOpen.text}
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
