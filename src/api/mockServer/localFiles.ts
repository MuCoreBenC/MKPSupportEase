import type { SlicerTarget } from '../contract'
import { t } from '../../i18n'
import { allPresetFiles } from './resources'

/*
 * **这是演示数据。** 这个文件里的集合是手写的，不是查磁盘查出来的。
 *
 * # 为什么要有它
 *
 * 用户端的预设列表要说「这个文件在你机器上 / 还得下」。契约里给的是**逐文件**的集合
 * （`getLocalFiles`），不是按版本的一个总状态 ——
 * 后者会让「本地 / 云端」这个区分整个消失。
 *
 * # 真后端要做的事（这里一件都没做）
 *
 *   1. 按 `asset.relativePath` 去预设仓库根目录下查文件在不在
 *   2. 和 manifest 里的 sha256 对账（现在 `FileRef.sha256` 恒为空，所以对账是另一件事）
 *   3. 大小与修改时间也在那一步才有真值
 *
 * # 这份集合怎么挑的
 *
 * 目的只有一个：让界面上的**四档状态都能被看见**，而且是被真数据驱动的。
 *
 *   `a1_mkp_standard`               A1 / 标准版的 MKP —— 默认落地那一屏的「已应用」
 *   `a1_bbs_mkpprocess_a1_04_020`   A1 的 0.4mm 工艺 —— 本机有，而且**已复制到切片器**
 *   `a1_bbs_mkpprocess_a1_02_010`   A1 的 0.2mm 工艺 —— 本机有，但**没复制**
 *   `a1_mini_mkp_fastv3.3`          A1 mini / 快拆版260628 的 MKP —— 另一台机型上的「未应用」
 *
 * 剩下 16 个文件不在集合里，所以它们是「待下载」。
 * 「暂不支持」不靠这一层（它是 `VersionFiles.incomplete`，A2L 那一个）。
 *
 * **不往这份集合里编「下了一半」那种中间态**：契约里没有那一档。
 */

/**
 * 演示集合。**手写的四个 asset id。**
 *
 * 顺序无所谓，调用方只当集合用。改这里等于改「演示时本机有什么」，
 * 不会影响仓库清单（那是 `assets.json`，20 个文件那一份）。
 */
const DEMO_LOCAL_IDS: string[] = [
  'a1_mkp_standard',
  'a1_bbs_mkpprocess_a1_04_020',
  /*
   * 本机有、但**没有**复制到切片器目录的那一个。
   *
   * 不加的话「切片器 · 本地」那张表里只剩一行，而它恰好就是已复制的那个 ——
   * 于是「未复制 + [复制]」这一档在界面上永远看不到，那个按钮点不到也就验不了。
   * 演示数据的职责就是让每一档状态都能被看见。
   */
  'a1_bbs_mkpprocess_a1_02_010',
  'a1_mini_mkp_fastv3.3',
]

/**
 * 本机有哪些文件。
 *
 * 写错的 id **不过滤掉** —— 悄悄丢掉会让「演示集合里那个文件怎么不见了」变成一个查不出的问题。
 */
export function localFileIds(): string[] {
  return [...DEMO_LOCAL_IDS]
}

// ——————————————————————————————————————————————————————————————
// 用户自己的文件：**已经搬走了**
// ——————————————————————————————————————————————————————————————

/*
 * 这一支（旧 `getLocalUserFiles` 的演示集合）已随 2026-10-02 的换源退役：
 * 用户自己的文件是**用户线**，住 `<appDataDir>/user/presets-local/`，
 * 由 `runtime::local` + `ipc::local` 那两条读回答（盘就是底账，没有演示集合）。
 * 浏览器模式下的演示数据在 `src/api/mock.ts` 的 `getLocalPresetFiles` 里。
 *
 * 旧夹具同时退役的还有一个形状问题：它带 `machineIds`（用户自己标的适用机型），
 * 而今天没有任何地方能让用户去标 —— 留着一个永远空的字段就是在编形状。
 */
// ——————————————————————————————————————————————————————————————
// 正在生效的那一套（全局唯一）
// ——————————————————————————————————————————————————————————————

/*
 * 「正在生效的那一套」（v023 的 `appliedPreset` / `DEMO_APPLIED` 演示集合）随
 * 契约方法 `getAppliedPreset` 一起删了（2026-10-02 清扫）：
 * 现在「使用中」只有一个来源 —— `run/active-preset.json`（`mockApi.getActivePreset`），
 * 界面也不再读那份按 asset id 编的演示值。
 * 留一笔记录，不再留夹具（没人读的假数据，下次盘点就得再猜一次它能不能删）。
 */

// ——————————————————————————————————————————————————————————————
// 已放进切片器目录的
// ——————————————————————————————————————————————————————————————

/** 假后端里的两个切片器与它们各自的账号目录（真机上就是 `user/<账号>/process`） */
const DEMO_SLICER_ACCOUNTS: Record<SlicerTarget, string[]> = {
  bambu: ['2046724740', 'default'],
  /* Orca 那一档少一个账号：框里能看出"另一个切片器也有落点" */
  orca: ['default'],
}

/** `切片器/账号` → 那一格已经躺进了哪几份文件。**可写的内存集合。** */
const DEMO_SLICER_PLACED = new Map<string, Set<string>>([
  ['bambu/2046724740', new Set(['MKPProcess A1 0.4 0.20.json'])],
  ['bambu/default', new Set(['MKPProcess A1 0.4 0.20.json'])],
])

/** 那一格（账号目录）的演示路径（浏览器里没有磁盘，形状照真机给） */
function demoProcessPath(slicer: SlicerTarget, account: string): string {
  const app = slicer === 'bambu' ? 'BambuStudio' : 'OrcaSlicer'
  return `~/Library/Application Support/${app}/user/${account}/process`
}

/** 这份预设归哪个切片器（与真后端 `slicer_of_kind` 同一口径） */
const DEMO_SLICER_OF_KIND: Record<string, SlicerTarget> = { bbs_config: 'bambu' }

/**
 * 已放进切片器的**文件名**。返回副本，调用方改不到这份状态。
 *
 * 与真机同一个口径：**自己那个切片器**的每一个账号目录里都有才算「已放入」
 * （别的切片器那一档是手动放的，不算进"这一份生效了没有"）。
 */
export function slicerCopied(): string[] {
  return allPresetFiles()
    .filter((file) => {
      const own = DEMO_SLICER_OF_KIND[file.kind]
      if (own === undefined) return false
      return DEMO_SLICER_ACCOUNTS[own].every((account) =>
        DEMO_SLICER_PLACED.get(`${own}/${account}`)?.has(file.fileName) === true,
      )
    })
    .map((file) => file.fileName)
}

/**
 * 这份在**每个切片器的每个账号目录**里的实况（与真后端 `get_slicer_placements`
 * 同一个形状：自己那一档在前，本机装了的另一个切片器跟在后面）。
 */
export function slicerPlacements(fileName: string): {
  available: boolean
  reason: string | null
  dirs: { slicer: SlicerTarget; account: string; path: string; placed: boolean }[]
} {
  const hit = allPresetFiles().find((f) => f.fileName === fileName)
  if (hit === undefined) throw new Error(t('mock.slicer.noSuchFile', { file: fileName }))
  const own = DEMO_SLICER_OF_KIND[hit.kind]
  if (own === undefined) throw new Error(t('mock.slicer.notMKP', { file: hit.fileName }))
  const order: SlicerTarget[] = own === 'bambu' ? ['bambu', 'orca'] : ['orca', 'bambu']
  return {
    available: true,
    reason: null,
    dirs: order.flatMap((slicer) =>
      DEMO_SLICER_ACCOUNTS[slicer].map((account) => ({
        slicer,
        account,
        path: demoProcessPath(slicer, account),
        placed: DEMO_SLICER_PLACED.get(`${slicer}/${account}`)?.has(fileName) === true,
      })),
    ),
  }
}

/**
 * 把这份从**它自己那个切片器**的每个账号目录里删掉（删除确认框里那个勾）。
 * 返回删掉了几份；本来就不在 = 0。别的切片器与云端一概不碰。
 */
export function removeSlicerCopiesIn(fileName: string): number {
  const hit = allPresetFiles().find((f) => f.fileName === fileName)
  if (hit === undefined) return 0
  const own = DEMO_SLICER_OF_KIND[hit.kind]
  if (own === undefined) return 0
  let removed = 0
  for (const account of DEMO_SLICER_ACCOUNTS[own]) {
    const cell = `${own}/${account}`
    const placed = DEMO_SLICER_PLACED.get(cell)
    if (placed?.delete(fileName) === true) removed += 1
  }
  return removed
}

/**
 * 把某个切片器 profile 放进切片器目录（`slicer` 不给 = 它自己那一档；`account` 给了就
 * 只放那一个账号）—— **只改内存，刷新还原。**
 *
 * 真后端在这里的真实失败：不是切片器配置、本机没有那份字节、认不出的切片器 / 账号目录；
 * **目标已有同名就换掉它**（2026-10-13 作者改判「有相同的就覆盖吧」）。假后端没有磁盘，
 * 所以只校验"类型对不上"与"不认识的切片器 / 账号"—— 静默成功的话，界面上会出现
 * 「放入一个 MKP 预设」这种说不通的状态。
 */
export function copyToSlicerIn(
  fileName: string,
  slicer?: SlicerTarget,
  account?: string,
): { replaced: boolean; targets: number } {
  const hit = allPresetFiles().find((f) => f.fileName === fileName)
  if (hit === undefined) throw new Error(t('mock.slicer.noSuchFile', { file: fileName }))
  const own = DEMO_SLICER_OF_KIND[hit.kind]
  if (own === undefined) throw new Error(t('mock.slicer.notMKP', { file: hit.fileName }))
  const target = slicer ?? own
  const accounts = DEMO_SLICER_ACCOUNTS[target]
  if (accounts === undefined) {
    throw new Error(t('mock.slicer.unknownSlicer', { slicer: target }))
  }
  const wanted = account === undefined ? accounts : [account]
  let replaced = false
  for (const wanted_account of wanted) {
    if (!accounts.includes(wanted_account)) {
      throw new Error(t('mock.slicer.unknownAccount', { account: wanted_account }))
    }
    const cell = `${target}/${wanted_account}`
    const placed = DEMO_SLICER_PLACED.get(cell) ?? new Set<string>()
    if (placed.has(fileName)) replaced = true
    placed.add(fileName)
    DEMO_SLICER_PLACED.set(cell, placed)
  }
  return { replaced, targets: wanted.length }
}
