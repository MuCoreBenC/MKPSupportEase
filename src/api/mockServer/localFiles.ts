import type { AppliedPreset, LocalUserFile } from '../../api/contract'
import { allPresetFiles } from './resources'

/*
 * **这是演示数据。** 这个文件里的集合是手写的，不是查磁盘查出来的。
 *
 * # 为什么要有它
 *
 * 用户端的预设列表要说「这个文件在你机器上 / 还得下」。契约里给的是**逐文件**的集合
 * （`getLocalFiles` / `getLocalUserFiles`），不是按版本的一个总状态 ——
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
// 用户自己的文件
// ——————————————————————————————————————————————————————————————

/**
 * **云端没有的那些文件。** 同样是手写的演示数据。
 *
 * 和上面那份集合的区别不是「另外几个 id」，而是**另一种东西**：上面那些是官方 asset
 * 已经下到本机，云端有权威副本；这三个是用户自己放进预设目录的，仓库里查不到，
 * 所以它们没有 asset id、没有交付身份，也永远不出现在云端那张表里。
 *
 * 挑这三个的目的：让本地表里「官方 / 我的」两种来源同时可见，并且能看出权限差别 ——
 * 官方那几行的「重命名」是灰的，这三行不是。
 *
 * 真后端要做的：遍历预设目录，把**不在 manifest 里**的文件都归到这一类。
 * 「适用机型」真后端只能靠文件名或文件内容猜，所以这里故意留一个 `machineIds: []`
 * 的例子 —— 用户没标就是没标，界面要能显示「未标机型」而不是替他猜一个。
 */
const DEMO_USER_FILES: LocalUserFile[] = [
  {
    id: 'user_a1mini_debug',
    fileName: 'a1mini_调试版.toml',
    path: 'user/a1mini_调试版.toml',
    kind: 'mkp_preset',
    machineIds: ['A1_MINI'],
  },
  {
    id: 'user_p1s_slow',
    fileName: 'p1s_慢速加胶.toml',
    path: 'user/p1s_慢速加胶.toml',
    kind: 'mkp_preset',
    machineIds: ['P1S'],
  },
  {
    /*
     * 这一个换成了**真仓那份文件的副本**：名字、喷嘴层高、内容都来自
     * `presets/bbs/Process/0.4mm/MKPProcess A1 mini 0.4 0.20.json`。
     *
     * 位置仍在 `user/` 下：演示要的就是「你把它放进自己预设目录」这一种 ——
     * 官方路径 `presets/bbs/…` 下那份是另一条记录（云端那张表里）。
     */
    id: 'user_mkpprocess_a1mini_04_020',
    fileName: 'MKPProcess A1 mini 0.4 0.20.json',
    path: 'user/MKPProcess A1 mini 0.4 0.20.json',
    kind: 'bbs_profile',
    /* 故意空着：用户没标适用机型。文件名里写着 A1 mini，但「没标」是他自己的状态，不替他猜 */
    machineIds: [],
    /* 这两个从文件名读得出来（真后端也只能这么猜），照实填 */
    nozzle: '0.4',
    layerHeight: '0.20',
  },
]

/** 用户自己的文件。返回副本，调用方改不到这份演示数据 */
export function localUserFiles(): LocalUserFile[] {
  return DEMO_USER_FILES.map((f) => ({ ...f, machineIds: [...f.machineIds] }))
}

// ——————————————————————————————————————————————————————————————
// 正在生效的那一套（全局唯一）
// ——————————————————————————————————————————————————————————————

/**
 * **正在生效的那一套预设。整个程序只有一个。**
 *
 * 前端自己推「当前机型 + 当前版本那个默认交付的 MKP 预设」是不行的：切一下机型就换一个，
 * 等于说这台机器同时应用着 6 套配置。涂胶笔同一时间只跑一套，所以这是一条**独立的事实**，
 * 归后端答。
 *
 * 演示值挑的是 `a1_mkp_standard`（A1 / 标准版），理由是它同时也在上面那份「本机已有」
 * 集合里 —— 这样默认落地那一屏能看到「已应用」，而不是「配着它但还没下」。
 *
 * 真后端要读本机那份当前配置（MKP 自己的 state 文件），**不是**猜哪个是默认交付的。
 * 想演示「一套都还没应用」把这里改成 `null` 即可，界面上会走另一条文案。
 */
const DEMO_APPLIED: { assetId: string; machineId: string; versionId: string } | null = {
  assetId: 'a1_mkp_standard',
  machineId: 'A1',
  versionId: 'STANDARD',
}

/**
 * 正在生效的那一套。`null` = 一套都还没应用（**不是错误**）。
 *
 * 机型 / 版本从这个 asset 的倒查结果推 —— 查不到就当没应用：
 * 编一个空壳出来只会让界面显示一个不存在的文件。
 */
export function appliedPreset(): AppliedPreset | null {
  if (DEMO_APPLIED === null) return null
  const hit = allPresetFiles().find((f) => f.id === DEMO_APPLIED.assetId)
  if (hit === undefined) return null
  const ref = hit.usedByVersions[0]
  return {
    assetId: hit.id,
    path: hit.path,
    machineId: ref?.machine ?? hit.machineIds[0] ?? '',
    versionId: ref?.version ?? '',
  }
}

// ——————————————————————————————————————————————————————————————
// 已复制到切片器目录的
// ——————————————————————————————————————————————————————————————

/**
 * 哪些切片器 profile 已经躺进切片器自己的 profile 目录了。**可写的内存集合。**
 *
 * 切片器文件光下到本机**没用** —— 它得在切片器的目录里才生效。所以两种类型的「生效」
 * 是两件事：
 *
 *   MKP     生效 = 设为当前配置       状态 已应用 / 未应用
 *   切片器   生效 = 复制到切片器目录    状态 已复制 / 未复制
 *
 * 演示集合挑 `a1_bbs_mkpprocess_a1_04_020`（它同时也在「本机已有」里）——
 * 这样默认那一屏能同时看到「已复制」和「未复制」两种行。
 */
const DEMO_SLICER_COPIED = new Set<string>(['a1_bbs_mkpprocess_a1_04_020'])

/** 已复制到切片器目录的 asset id。返回副本，调用方改不到这份状态 */
export function slicerCopied(): string[] {
  return [...DEMO_SLICER_COPIED]
}

/**
 * 复制某个切片器 profile 到切片器目录。**只改内存，刷新还原。**
 *
 * 真后端在这里会有两种真实的失败：切片器路径没配、目标已存在。那时该抛，界面照抛。
 * 假后端没有磁盘，所以只校验「它得是切片器文件」——
 * 传错类型静默成功的话，界面上会出现「已应用一个 json」这种说不通的状态。
 */
export function copyToSlicerIn(assetId: string): void {
  const hit = allPresetFiles().find((f) => f.id === assetId)
  if (hit === undefined) throw new Error(`仓库里没有这个文件：${assetId}`)
  if (hit.kind === 'mkp_preset') {
    throw new Error(`${hit.fileName} 是 MKP 预设，不用复制到切片器 —— 它走「应用」`)
  }
  DEMO_SLICER_COPIED.add(assetId)
}
