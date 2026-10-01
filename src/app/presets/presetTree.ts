/*
 * 预设页的数据形状与判定规则。
 *
 * 这个文件里**只有类型、常量和纯函数** —— 没有 state、没有 JSX、没有 api 调用。
 * 页面、卡片、行、首页都读它：「待下载」在一处写成「待下载」、另一处写成「云端」，
 * 界面就开始撒谎。
 *
 * # 三个轴，版本**不在其中**（这一轮改的就是这一条）
 *
 *   机型          → 页面顶部一个下拉（`PresetPicker`），含「全部机型」一档
 *   MKP / 切片器  → 一条分段控件（`PresetScopeBar` 左半），文件类型
 *   本地 / 云端   → 一条分段控件（`PresetScopeBar` 右半），**两张互不相干的表**
 *
 * **版本从筛选器降成了表里的一列。** 原因：筛选器的宽度随选项个数线性增长 ——
 * 6 个版本就摆不下，实测那一行还只画出了 3 个，剩下的直接看不见。而版本是
 * 「这个文件的属性」，机型才是「我这台机器关我什么事」的边界。所以行按**机型**汇总，
 * 一个文件一行，用到它的版本名收在 `versions: string[]` 里由表格画成一列（多个用 `·` 连，
 * **不写 `+N`** —— 作者原话「显示什么版本加 2 什么意思？」，那是省宽度省出来的黑话）。
 * 这一列不可点、不排序、不分组 —— 做成可点的就等于把筛选器换个位置放回来。
 *
 * 曾经把「本地 / 云端」做成了同一个文件的行状态（一个角标 + 顶部一句汇总），那被否了。
 * 正确的模型是**两张表各回答一个问题**，谁也不管谁：
 *
 *   本地表  你这台机器上有哪些文件。两种来源：官方下载下来的副本（`getLocalFiles()`）
 *           与用户自己放进预设目录的（`getLocalUserFiles()`，**云端没有它们**）
 *   云端表  菜单上有哪些官方文件（已分配 + 可选）。行尾标「已下载 / 未下载」
 *
 * 所以一个官方文件下载之后**两张表里都有**，那是对的；用户自己的文件只在本地表里，
 * 云端表永远看不到它。两张表由 `localRows()` / `cloudRows()` 各自算，不合流。
 *
 * **树本身留着** —— 首页（`pages/PageHome`，这一轮不动）按 品牌→机型→版本→文件
 * 四步走，它要的就是这棵树；两张表也是从树上按机型汇总出来的。
 *
 * # 一个文件挂在哪几个版本下
 *
 * 两条规则，来自上游的两条链：
 *
 *   MKP 预设（`mkp_preset`）  按**机型 + 版本** —— `getVersionFiles(m, v)` 给的就是它
 *   BBS 工艺（`bbs_profile`） 按**机型** —— 同一台机型的每个版本都用得到
 *
 * 默认交付那一份从 `getVersionFiles` 来（那是「这个版本的最终资源」的唯一权威），
 * 可选的那 4 个 0.2mm profile 谁都没引用（`delivery === 'optional'` 的判据就是两个方向都没人引用），
 * 它们按 `machineIds` 挂到那台机型的**每个版本**下 —— 要手动下的人是在自己那台机器下面找。
 *
 * # 「仅归档」的文件一处都不出现
 *
 * 不在菜单上 = 客户端看不到也下不了。所以建树时就把 `MenuEntry.visibility`
 * 为 `archived` 的 asset **整个剔掉** —— 剔在这一层而不是在界面上过滤，是为了让首页与
 * 预设页两处都不可能漏（漏一处就等于把「藏起来的文件」摆到了用户面前）。
 *
 * # 大小与时间：**两种来源，靠 `statFrom` 分开**，前端一行都不编
 *
 * ```
 * 切片器（bbs）  `statFrom: 'file'`  真仓里那份真文件的字节数与上游 manifest 记的更新时间
 * 其余           `statFrom: 'demo'`  假后端按 path 的 FNV 哈希稳定推的演示值（同一个文件每次刷新都一样）
 * ```
 *
 * 真后端两档都要 `stat`。前端只搬，不算、不编、不补默认值：拿不到就写「未知」并在 title 里说清
 * 为什么（用户自己放进预设目录的文件就是这一种 —— 契约的 `LocalUserFile` 上没有这两个字段）。
 *
 * `FileRef.size`（字节数）仍恒为 undefined，它和 `sizeText` 不是同一个东西，别混。
 *
 * # 两种类型的「生效」是两件不同的事
 *
 * ```
 * MKP     生效 = 设为当前配置（唯一底账 STORAGE.clientActive）  状态 已应用 / 未应用
 * 切片器   生效 = 复制到切片器目录（copyToSlicer）       状态 已复制 / 未复制
 * ```
 *
 * 切片器 profile 光下到本机没用 —— 它得躺在切片器自己的 profile 目录里才生效。
 * 所以本地表那一列不能两种类型共用一套词（见 `LIVE_TEXT`）。
 */

import type {
  FileKind,
  FileRef,
  FilesState,
  LocalUserFile,
  Machine,
  MachineVersion,
  MenuEntry,
  PresetFileInfo,
  VersionFiles,
} from '../../api'
import type { ActiveEntry } from '../store/package'

/**
 * 用户自己放进预设目录的文件（`LocalUserFile`）。
 *
 * 名字直接用契约那一个，**不抄字段**：契约改了这里跟着改。这里**转出去**是因为
 * 这一页的输入形状（`PresetRowsInput`）也把它写在签名上 —— 调用方不必再去 `src/api` 取一次。
 */
export type { LocalUserFile }

/** 取不到真值时的统一说法，与工作台那边用的是同一个词 */
export const UNKNOWN = '未知'

/** 大小那一栏的 title。为什么是「未知」得说清，否则看的人只会觉得界面没做完 */
export const SIZE_WHY =
  '上游资产清单里 size 与 sha256 全是空值，真实大小要到发布打 manifest 时才算得出来。不显示 0 —— 那会让「不知道」长得像「空文件」'

/**
 * 「时间」那一格的 title（写成人话）。
 *
 * 原来那段是给开发者看的（假后端 / PresetFileInfo / FNV 哈希堆在一句里）——
 * 作者悬停时看到的就是它：「不知道是什么乱七八糟的信息」。
 * 事实不变：值是按 path 稳定推的演示数据，同一个文件每次刷新都一样；前端一行不编。
 */
export const DEMO_STAT_WHY =
  '演示数据：这个时间是按文件路径推算的固定值，不是文件的真实时间；同一个文件每次刷新显示的都是它'

/** 没有这个值的那一种：用户自己放进预设目录的文件。**不替它编一个** */
export const NO_STAT_WHY =
  '这个文件是你自己放进预设目录的，仓库里没有它的记录 —— 没有时间可显示，也不编一个'

/*
 * **真值**那两格的 title —— 切片器（bbs）这一档现在读的是仓库里那份真文件。
 *
 * 事实链：`scripts/sync-bbs-presets.mjs` 从上游 `mkpse-presets` 仓把 9 个 json **逐字节**
 * 拷进 `public/presets/bbs/…` 时，把磁盘上的字节数与上游 `manifest.json` 记的
 * `sha256 / size / updatedAt` 核了一遍；页面这两格读的就是那份快照
 * （`src/api/mockServer/data/bbs_files.json` 那份快照）。**不写「实时」** —— 它是快照，上游改了要重出快照。
 *
 * 为什么大小与时间要分开两句话：这两格各自都可能「有值但不是真的」，混着一句话说不清。
 */
export const FILE_SIZE_WHY =
  '真值：这是仓库里那份真文件的字节数（假后端的静态快照按上游清单核过，副本在 public/assets/bbs/ 下）'

export const FILE_TIME_WHY =
  '真值：这是上游 manifest 记的那份文件的内容更新时间 —— 不是本机 checkout 出来的时间'

/** 类型角标。MKP = 涂胶预设 toml，BBS = 切片器工艺 profile */
export const KIND_LABEL: Record<FileKind, string> = {
  mkp_preset: 'MKP',
  bbs_profile: 'BBS',
  orca_profile: 'ORCA',
}

export const KIND_NAME: Record<FileKind, string> = {
  mkp_preset: 'MKP 预设',
  bbs_profile: 'BBS 工艺',
  orca_profile: 'Orca 工艺',
}

/**
 * 两条分段控件的词表。
 *
 * 左边那条是**文件类型**（`PresetFileInfo.kind`），和工作台「分配菜单」那一步
 * 的 MKP 段 / BBS 段说的是同一件事，所以用词要对得上。
 * 右边那条是**位置**，而位置不是筛选条件 —— 它切的是两张互不相干的表（见文件头）。
 */
export type PresetKindAxis = 'mkp' | 'slicer'
export type PresetScopeAxis = 'local' | 'cloud'

export const KIND_AXIS_TEXT: Record<PresetKindAxis, string> = {
  mkp: 'MKP 配置',
  slicer: '切片器配置',
}

export const SCOPE_AXIS_TEXT: Record<PresetScopeAxis, string> = {
  local: '本地',
  cloud: '云端',
}

export const SCOPE_AXIS_WHY: Record<PresetScopeAxis, string> = {
  local: '本地：你这台机器上真有的文件 —— 官方下载下来的副本，加上你自己放进预设目录的（云端没有它们）',
  cloud: '云端：菜单上的官方文件（已分配 + 可选）。仅归档的一个都不出现 —— 不在菜单上 = 客户端看不到也下不了',
}

/**
 * 本地表「状态」那一列的词。**两种类型两套词，因为「生效」是两件不同的事**：
 *
 *   MKP     生效 = 设为当前配置（`applyPreset`）      已应用 / 未应用
 *   切片器   生效 = 复制到切片器目录（`copyToSlicer`） 已复制 / 未复制
 *
 * 曾经两种共用 `已应用 / 本地有` 那一套，等于说「把 profile 下到本机就生效了」——
 * 不成立：切片器只读它自己那个 profile 目录。
 */
export const LIVE_TEXT: Record<PresetKindAxis, { on: string; off: string }> = {
  mkp: { on: '已应用', off: '未应用' },
  slicer: { on: '已复制', off: '未复制' },
}

export const LIVE_WHY: Record<PresetKindAxis, { on: string; off: string }> = {
  mkp: {
    on: '已应用：本机底账（STORAGE.clientActive）说正在使用的就是它。**全局唯一** —— 换机型也不会变出第二个',
    off: '未应用：文件在本机，但当前生效的是别的那一套。点「应用」把它设成当前的',
  },
  slicer: {
    on: '已复制：getSlicerCopied() 说它已经躺在切片器的 profile 目录里了 —— 这才是切片器文件「生效」的方式',
    off: '未复制：文件在本机，但还没进切片器的 profile 目录，切片器看不见它。点「复制」把它送过去',
  },
}

/** 操作列上那个按钮的字。动作用**文字**不用图标 */
export const ACTION_TEXT: Record<PresetKindAxis, string> = {
  mkp: '应用',
  slicer: '复制',
}

/** 没有 asset id 的行为什么没有操作按钮。**不给一个点了会报错的按钮** */
export const NO_ASSET_WHY =
  '你自己放进预设目录的文件仓库里没有记录，没有 asset id —— 而契约的 applyPreset / copyToSlicer 只认 asset id。给一个点了必报错的按钮比不给糟'

/**
 * 交付身份的两种说法。**只有两种，没有第三种**（见契约 `PresetFileInfo.delivery`）：
 * 默认交付是钦定的最终资源、点下载后台自动下好；可选是仓库里有、没进默认集，用户自己手动下。
 * 「可选」是中性的一档 —— 不带 ⚠、不上警示色。
 */
export const DELIVERY_TEXT: Record<PresetFileInfo['delivery'], string> = {
  default: '默认交付',
  optional: '可选',
}

export const DELIVERY_WHY: Record<PresetFileInfo['delivery'], string> = {
  default: '默认交付：这个版本钦定的最终资源，点下载后台自动下好',
  optional: '可选：仓库里有、没进默认集，需要手动下载。不是错误，也不是孤儿',
}

/** 下载点了会抛 —— 这是设计，见 `PresetFileRow` 的文件头 */
export const DOWNLOAD_NOT_READY = '尚未实现'

export const DOWNLOAD_WHY =
  '下载要写盘，契约里 downloadFiles 在假后端上直接抛 NotImplementedError —— 这里不假装下载成功'

/**
 * 发布行的「下载」：这一条**是真的能下的**（`fetchPreset` 落到本机预设目录）——
 * 原来所有云端行共用官方那条「不假装下载成功」的说明，文案与行为不符，按行分流。
 */
export const RELEASE_DOWNLOAD_WHY =
  '从工作台发布里下载这一份到本机预设目录 —— 下载 ≠ 使用，生效要到本地表里点「应用」'

/**
 * 发布行「时间 / 大小」两格的 title：这两样都是**真值**（发布时刻 / 下载时刻 /
 * TOML 字节数），不该沿用官方行那句「演示数据」——按行分流，说清是哪一层的意思。
 */
export const RELEASE_TIME_WHY = {
  cloud: '这次发布的时刻',
  local: '下载到本机的时刻',
} as const

export const RELEASE_SIZE_WHY = '按这一份 TOML 的字节数算的'

/**
 * 来源：这个文件是官方的、用户自己的，还是**工作台发布下来的**。
 *
 * 权限也挂在这一列上：官方副本不能改名（复制一份再改），用户自己的文件删了没有任何地方能找回来。
 */
export type PresetOrigin = 'official' | 'mine' | 'release'

export const ORIGIN_TEXT: Record<PresetOrigin, string> = {
  official: '官方',
  mine: '我的',
  release: '工作台发布',
}

export const ORIGIN_WHY: Record<PresetOrigin, string> = {
  official: '官方：仓库里那份文件下载到本机的副本。不能改名（复制一份再改），删了可以从云端重新下载',
  mine: '我的：你自己放进预设目录的文件，云端没有它。可以改名，但删了没有任何地方能找回来',
  release: '工作台发布：工作台「生成与发布」发上来的 preset.toml。云端表只管下载，生效用本地那颗「应用」',
}

/**
 * 来源列的一格：release 行要带上**包版本**（「工作台发布 · 1.0.0」），
 * 官方 / 我的照常走那两张静态表 —— 查表给不了动态的那截，所以收成一个小函数。
 */
export function originChip(row: {
  origin: PresetOrigin
  releaseVersion?: string | null
}): { text: string; title: string } {
  if (row.origin === 'release') {
    /* title 里带上全文 —— 窄档表里它会被省略号截断，悬停要能看到完整来源 */
    const text = row.releaseVersion
      ? `${ORIGIN_TEXT.release} · ${row.releaseVersion}`
      : ORIGIN_TEXT.release
    return { text, title: `${text} —— ${ORIGIN_WHY.release}` }
  }
  return { text: ORIGIN_TEXT[row.origin], title: ORIGIN_WHY[row.origin] }
}

/** 云端表那一列：这个官方文件在不在本机。判据与本地表是同一个 `getLocalFiles()` */
export const CLOUD_STATE_TEXT = {
  downloaded: '已下载',
  pending: '未下载',
}

export const CLOUD_STATE_WHY = {
  downloaded: '已下载：getLocalFiles() 说本机已经有这个文件了（假后端给的是固定演示集合）—— 它同时出现在本地表里，那是对的',
  pending: '未下载：仓库里有，你机器上还没有',
}

/**
 * 交付身份在云端表上的说法。
 *
 * 和 `DELIVERY_TEXT`（默认交付 / 可选）是同一个字段的两种措辞：那一套是仓库视角的身份，
 * 这一套是用户视角的「要不要我自己动手」。词不同是有意的 —— 用户端的表上说「套餐内」
 * 比说「默认交付」好懂，而工作台那边必须继续说「默认交付」（它编的就是那个集合）。
 */
export const DELIVERY_SCOPE_TEXT: Record<PresetFileInfo['delivery'], string> = {
  default: '套餐内',
  optional: '可单下',
}

/**
 * 右键菜单里那些**还没有后端**的动作。
 *
 * 分两种，界面上的说法也分两种：
 *
 *   契约里有签名   照调，让它抛 `NotImplementedError`，显示「尚未实现：<方法名>」
 *                 —— 现在只有 `downloadFiles` 是这一种
 *   契约里没签名   不发请求，就地说「契约里还没有这个方法：<要加的方法名>」
 *
 * 方法名是**给自己看的待办**，所以写的是将来要加在 `src/api/contract.ts` 里的那个名字。
 * 这一轮不许往契约里加方法（不在范围内），所以这里只是一句话，不是一个调用。
 */
export const MISSING_METHOD = {
  copy: 'copyLocalFile',
  rename: 'renameLocalFile',
  remove: 'deleteLocalFile',
  reveal: 'revealInFolder',
  link: 'getFileUrl',
}

/** 「尚未实现」与「契约里还没有这个方法」两句话的统一写法，免得各处各写一套 */
export const notImplementedText = (method: string): string => `尚未实现：${method}`

export const noContractText = (method: string): string => `契约里还没有这个方法：${method}`

// ——————————————————————————————————————————————————————————————
// 四档状态
// ——————————————————————————————————————————————————————————————

/**
 * 一行文件的状态。**直接用契约里那四档**（`FilesState`），不另造一套字面量 ——
 * 界面与后端说同一套词，真后端接上时这一层不用翻译。
 *
 *   `applied`      已应用    绿    无主操作
 *   `ready`        本地有    中性  应用
 *   `missing`      待下载    蓝    下载
 *   `unavailable`  暂不支持  灰    **无按钮**
 *
 * `unavailable` 是**机型级**的（这台机型下所有版本后端都没配资源，实测只有 A2L），
 * 所以它不会出现在某一行上：那种情况整页两张表都不画，只画一块
 * 「暂不支持该机型或版本」+ 后端给的原因。
 */
export type PresetStatus = FilesState

export const STATUS_TEXT: Record<PresetStatus, string> = {
  applied: '已应用',
  ready: '本地有',
  missing: '待下载',
  unavailable: '暂不支持',
}

export const STATUS_WHY: Record<PresetStatus, string> = {
  applied:
    '已应用：本机有这个文件，而且本机底账（STORAGE.clientActive）说正在使用的就是它。全局唯一 —— 换机型也不会变出第二个',
  ready: '本地有：getLocalFiles() 说本机已经有这个文件了（假后端给的是固定演示集合），还没应用',
  missing: '待下载：本机还没有这个文件',
  unavailable: '暂不支持该机型或版本：后端没有配这个机型的资源 —— 不是你这台机器的问题',
}

/** 「暂不支持」的正式说法。灰色、中性，和网络失败与「0 个文件」都要分得开 */
export const UNSUPPORTED_TEXT = '暂不支持该机型或版本'

// ——————————————————————————————————————————————————————————————
// 树：机型 → 版本 → 文件
//
// 预设页的本地表 / 云端表从它按机型汇总（machineFiles）；首页**不读它** ——
// 首页的三级选择吃 mock 手编表（旧注释「首页四步选择要整棵树」是没兑现的说法，已删）。
// ——————————————————————————————————————————————————————————————

/** 一行文件。**同一个文件可以在多个版本下各出现一行**，所以 rowKey 带上机型与版本 */
export interface PresetFileNode {
  rowKey: string
  /** 预设仓库里的 asset id；`getVersionFiles` 给了而仓库里查不到时没有这个字段 */
  id?: string
  fileName: string
  /** 相对预设仓库根 */
  path: string
  kind: FileKind
  delivery: PresetFileInfo['delivery']
  /** 恒为 undefined（见 SIZE_WHY）。留着是为了真值到了不用改形状 */
  size?: number
  /**
   * 大小与时间，**已经是可显示的字符串**（`4.2 KB` / `09-14`）。
   *
   * 来自 `PresetFileInfo.sizeText` / `.modifiedText`：切片器那一档是真值、其余是假后端
   * 按 path 稳定推的演示值 —— 哪一档看 `statFrom`（见文件头）。
   * 仓库里查不到这个文件（`info` 是 undefined）时两个都没有，界面写「未知」，**不编**。
   */
  sizeText?: string
  modifiedText?: string
  /** 上面两格的来源。`undefined` = 这个来源没有那两格 */
  statFrom?: 'file' | 'demo'
  /** 喷嘴直径。**只有切片器 profile 有** —— MKP 的 toml 没这回事 */
  nozzle?: string
  /** 层高，同上 */
  layerHeight?: string
  machineId: string
  versionId: string
}

export interface PresetVersionNode {
  machineId: string
  version: MachineVersion
  files: PresetFileNode[]
  /** `VersionFiles.incomplete`：该有的文件没配齐。界面要说「暂不支持」而不是「0 个文件」 */
  unavailable: boolean
  /** 没配齐的原因，一条一句，可直接显示 */
  missing: string[]
  /** 与出厂不同的条数 = `getMachineParams(m, v)` 里 origin === 'variant' 的条数 */
  changed: number
}

export interface PresetMachineNode {
  machine: Machine
  versions: PresetVersionNode[]
  /** 这台机型下**去重后**的文件数（一个 BBS profile 被三个版本共用，只算一个） */
  fileCount: number
  /** 全部版本都没配齐、且一个文件都没有 */
  unavailable: boolean
  /** 各版本 missing 的并集 */
  missing: string[]
}

export interface PresetTree {
  machines: PresetMachineNode[]
  /** 预设仓库里一共几个文件（已剔掉仅归档的）—— 与树上出现几行是两回事 */
  totalFiles: number
}

/** 建树时每个「机型:版本」要喂进来的三样东西 */
export interface PresetVersionInput {
  machineId: string
  versionId: string
  /** `getVersionFiles` 的原样结果。`null` = 后端没有这个组合 */
  files: VersionFiles | null
  changed: number
}

export const atOf = (machineId: string, versionId: string): string => `${machineId}:${versionId}`

/** 一个文件被几个版本共用时行数会重复，所以去重一律按 path */
function uniquePaths(files: PresetFileNode[]): Set<string> {
  return new Set(files.map((f) => f.path))
}

function nodeOf(
  ref: Pick<FileRef, 'fileName' | 'path' | 'kind' | 'size'>,
  info: PresetFileInfo | undefined,
  machineId: string,
  versionId: string,
): PresetFileNode {
  return {
    rowKey: `${machineId}:${versionId}:${ref.path}`,
    id: info?.id,
    fileName: ref.fileName,
    path: ref.path,
    kind: ref.kind,
    /*
     * 交付身份只认仓库那一份。`getVersionFiles` 给出来的就是「这个版本的默认集」，
     * 所以查不到仓库记录时按 default 算 —— 那种情况说明仓库表与 bundle 表对不上，
     * 不该在这里悄悄改成「可选」（那会让一个真的默认文件显示成可选）。
     */
    delivery: info?.delivery ?? 'default',
    size: ref.size,
    /* 这两个只从契约搬。查不到仓库记录就是 undefined —— 界面写「未知」，不在这里补一个值 */
    sizeText: info?.sizeText,
    modifiedText: info?.modifiedText,
    statFrom: info?.statFrom,
    nozzle: info?.nozzle,
    layerHeight: info?.layerHeight,
    machineId,
    versionId,
  }
}

/**
 * 仅归档的 asset id 集合。
 *
 * 菜单拉不到时返回空集合 —— **宁可多显示也不少显示**？不：这里刚好相反，
 * 菜单拉不到时 `usePresetData` 会走 error 分支整页说拉不到，不会拿一棵没过滤的树去渲染。
 * 这个函数只负责把三态翻成一个集合。
 */
export function archivedIds(menu: MenuEntry[]): Set<string> {
  return new Set(menu.filter((e) => e.visibility === 'archived').map((e) => e.fileId))
}

/**
 * 把几个读的结果拼成一棵树。
 *
 * 纯函数：给同样的输入永远给同样的树，所以能直接在测试里喂假数据，也不会在 StrictMode 下双跑出岔。
 */
export function buildPresetTree(
  machines: Machine[],
  repo: PresetFileInfo[],
  inputs: PresetVersionInput[],
  /** 仅归档的 asset id。这些文件**一处都不出现** */
  archivedIds: Set<string> = new Set(),
): PresetTree {
  /* 仅归档的先从仓库清单里剔掉：后面两支都查这张表，剔一次就够 */
  const visible = repo.filter((f) => !archivedIds.has(f.id))
  const repoByPath = new Map(visible.map((f) => [f.path, f]))
  const repoByName = new Map(visible.map((f) => [f.fileName, f]))
  const archivedPaths = new Set(
    repo.filter((f) => archivedIds.has(f.id)).map((f) => f.path),
  )
  const inputAt = new Map(inputs.map((i) => [atOf(i.machineId, i.versionId), i]))

  const machineNodes = machines.map((machine): PresetMachineNode => {
    const versions = machine.versions.map((version): PresetVersionNode => {
      const input = inputAt.get(atOf(machine.id, version.id))
      const vf = input?.files ?? null

      const seen = new Set<string>()
      const files: PresetFileNode[] = []

      /* 1. 默认交付：这个版本的最终资源，来自 getVersionFiles */
      for (const ref of vf?.files ?? []) {
        if (seen.has(ref.path)) continue
        /* 仅归档的文件即使被套餐装着也不给客户端看（那条静默漏洞由工作台的发布检查报） */
        if (archivedPaths.has(ref.path)) continue
        seen.add(ref.path)
        files.push(
          nodeOf(ref, repoByPath.get(ref.path) ?? repoByName.get(ref.fileName), machine.id, version.id),
        )
      }

      /*
       * 2. 可选：仓库里有、没进任何默认集。BBS profile 是**按机型**的，
       *    所以挂到这台机型的每个版本下 —— 要手动下的人是在自己那台机器下面找。
       */
      for (const info of visible) {
        if (info.delivery !== 'optional') continue
        if (!info.machineIds.includes(machine.id)) continue
        if (seen.has(info.path)) continue
        seen.add(info.path)
        files.push(nodeOf(info, info, machine.id, version.id))
      }

      /* MKP 在前、切片器 profile 在后；同类按文件名 */
      files.sort((a, b) => {
        if (a.kind !== b.kind) return a.kind === 'mkp_preset' ? -1 : 1
        return a.fileName.localeCompare(b.fileName)
      })

      return {
        machineId: machine.id,
        version,
        files,
        unavailable: vf === null || vf.incomplete,
        missing:
          vf === null ? [`后端没有 ${atOf(machine.id, version.id)} 这个组合`] : vf.missing,
        changed: input?.changed ?? 0,
      }
    })

    return { machine, versions, ...countsOfMachine(versions) }
  })

  return { machines: machineNodes, totalFiles: visible.length }
}

/** 机型层的三个数 */
function countsOfMachine(versions: PresetVersionNode[]): {
  fileCount: number
  unavailable: boolean
  missing: string[]
} {
  const all = versions.flatMap((v) => v.files)
  const paths = uniquePaths(all)
  return {
    fileCount: paths.size,
    /* 「暂不支持」= 一个文件都没有，而且每个版本都没配齐。有文件就不算 */
    unavailable: paths.size === 0 && versions.every((v) => v.unavailable),
    missing: [...new Set(versions.filter((v) => v.unavailable).flatMap((v) => v.missing))],
  }
}

/** 树上的某一格。两个 id 都得对上，查不到返回 undefined（不拿别的顶） */
export function versionNode(
  tree: PresetTree,
  machineId: string,
  versionId: string,
): PresetVersionNode | undefined {
  return tree.machines
    .find((m) => m.machine.id === machineId)
    ?.versions.find((v) => v.version.id === versionId)
}

/**
 * 树上的某一台机型。
 *
 * 这一轮页面按机型汇总，所以要的是这个而不是某一格：「暂不支持」的判据从「当前这一个版本
 * 没配资源」改成了「**这台机型下所有版本都没配**」（`PresetMachineNode.unavailable`）。
 * `machineId` 是空串（「全部机型」）时返回 undefined —— 那一档不可能整机型没配。
 */
export function machineNode(
  tree: PresetTree,
  machineId: string,
): PresetMachineNode | undefined {
  if (machineId === '') return undefined
  return tree.machines.find((m) => m.machine.id === machineId)
}

/**
 * 要汇总哪几台机型。
 *
 * `machineId` 是空串就是「全部机型」—— 那一档把本机 / 云端全部文件都列出来，
 * 顺手补上了之前「在 A1 下看不到 A1 mini 的本机文件」那个缺口。
 */
export function machinesInScope(tree: PresetTree, machineId: string): PresetMachineNode[] {
  if (machineId === '') return tree.machines
  const hit = machineNode(tree, machineId)
  return hit === undefined ? [] : [hit]
}


// ——————————————————————————————————————————————————————————————
// 状态与搜索
// ——————————————————————————————————————————————————————————————

/**
 * 一行文件现在是四档里的哪一档。
 *
 * `localIds` 是 `api.getLocalFiles()` 的结果（假后端给的是固定演示集合）。
 * `active` 是**唯一底账**（`STORAGE.clientActive`）读出来的那一条：**全表最多一份**，
 * 换机型也不会变出第二个。判据**只认它** —— 假后端的 `getAppliedPreset` 界面不再读。
 *
 * 先判在不在本机，再判是不是正在使用的那一份 —— 「已应用」比「本地有」靠前，
 * 它是「本机有 + 正在用的就是它」。判据只剩这两条。
 */
export function statusOf(
  file: { id?: string },
  localIds: Set<string>,
  active: ActiveEntry | null,
): PresetStatus {
  const id = file.id
  if (id === undefined || !localIds.has(id)) return 'missing'
  if (active !== null && active.kind === 'official' && id === active.ref) return 'applied'
  return 'ready'
}


/**
 * 搜索：按文件名 / 路径。空串 = 不筛。
 *
 * 参数写成结构类型而不是 `PresetFileNode`，是因为用户自己的文件（`LocalUserFile`）
 * 与两张表的行都要用同一条判据 —— 三处各写一遍 `includes` 迟早会有一处忘了筛路径。
 */
export function fileMatchesQuery(
  file: { fileName: string; path: string },
  query: string,
): boolean {
  const q = query.trim().toLowerCase()
  if (q === '') return true
  return file.fileName.toLowerCase().includes(q) || file.path.toLowerCase().includes(q)
}

/** 取不到机型名 / 版本名时那一格写的短横。不是空格，也不是 0 */
export const DASH_ = '—'

/**
 * 「版本」那一格写什么。**没有 `+N`。**
 *
 * 作者原话：「显示什么版本加 2 什么意思？」—— `标准版 +2` 是上一轮为了省宽度编出来的黑话，
 * 看的人得先猜「+2」是版本数还是别的什么。所以：
 *
 *   一个版本   直接写版本名（`标准版`）
 *   多个版本   用 `·` 把**全部**版本名连起来（`标准版 · 快拆版260628`），宽度不够由 CSS 截断
 *   一个都没有 写 `—`（用户自己放进预设目录的文件不属于任何打印件版本）
 *
 * `title` 里永远是全的 —— 截断只影响看得见的宽度，不影响能不能查。
 */
export function versionsText(versions: string[]): { text: string; why: string } {
  if (versions.length === 0) {
    return {
      text: DASH_,
      why: '这个文件不属于任何打印件版本 —— 它是你自己放进预设目录的，仓库里没有它，也没有交付身份',
    }
  }
  const text = versions.join(' · ')
  return {
    text,
    why:
      versions.length === 1
        ? `只有「${text}」用到这个文件`
        : `${versions.length} 个版本共用这个文件：${text}`,
  }
}

// ——————————————————————————————————————————————————————————————
// 两张互不相干的表
//
// 本地表 = 本机磁盘上有什么（官方副本 + 用户自己的文件）
// 云端表 = 菜单上有什么官方文件（已分配 + 可选），行尾标已下载 / 未下载
//
// 一个官方文件下载之后两张表里都有 —— 两张表各回答一个问题，不合流（见文件头）。
// ——————————————————————————————————————————————————————————————

/** 两张表的行共有的部分。表格那 7 列全靠它 —— 组件不再回头查树 */
export interface PresetRowBase {
  /** 这张表里唯一。带上机型：**一个文件在一台机型下只有一行**（版本收进了 `versions`） */
  rowKey: string
  /**
   * 预设仓库里的 asset id。用户自己的文件与仓库里查不到的文件**没有**这个字段。
   *
   * 「已应用」的判据就是拿它和唯一底账（`STORAGE.clientActive`）里那一条的 ref 比 ——
   * 所以它必须在行上（release 行走 uid，见 `releaseUid`）。
   */
  assetId?: string
  /**
   * 置顶记的是这个键。用 asset id（用户自己的文件用它的本机 id），**不用 rowKey** ——
   * 置顶是「我常用这个文件」，换台机型再看还该在最前面。
   */
  pinKey: string
  fileName: string
  /** 相对预设仓库根。表格第二行的小字与「在文件夹中显示」都用它 */
  path: string
  kind: FileKind
  /** 这一行挂在哪台机型下。「全部机型」那一档下每一行各属于自己那台，所以它得在行上 */
  machineId: string
  /**
   * 机型的**显示名**（`A1 mini`），表格「机型」那一列。
   *
   * 为什么要这一列：机型下拉有「全部机型」一档，那一档下不写机型就分不出哪一行是哪台机器的。
   * 用户自己的文件按他标的适用机型写（多台用 `·` 连），一台都没标就是 `—`（那一枚
   * 「未标机型」的角标在名称列上）。
   */
  machineText: string
  /**
   * 用到这个文件的版本**显示名**（不是 id），按机型目录里的顺序。
   *
   * 表格画成一列：一个就写名字、多个用 `·` 把**全部**版本名写出来（宽度不够由 CSS 截断，
   * `title` 里是全的）、空数组写 `—`（用户自己放进预设目录的文件没有交付身份，
   * 也就不属于任何版本）。**没有 `+N` 这种写法** —— 见 `versionsText`。
   * **不做筛选、不做排序** —— 那就又变成一个轴了。
   */
  versions: string[]
  /** 喷嘴 / 层高：**只有切片器 profile 有**。MKP 的表里这两列整个不存在 */
  nozzle?: string
  layerHeight?: string
  /**
   * 大小与时间，来自契约（`PresetFileInfo.sizeText` / `.modifiedText`）。
   * **前端一行不编** —— 没有就是没有，界面写「未知」。
   *
   * 这两格的来源跟着 `statFrom` 走 —— 切片器那一档是**真值**（真仓文件的字节数与
   * 上游 manifest 记的更新时间），其余仍是演示推值。tooltip 分两句话，见 `PresetTable`。
   */
  sizeText?: string
  modifiedText?: string
  /** 上面那两格是哪来的。`undefined` = 这个来源没有这两格（自己的文件、release 行走别的口径） */
  statFrom?: 'file' | 'demo'
  /** 唯一底账（`STORAGE.clientActive`）说正在使用的就是这一份。**全页最多一行** */
  applied: boolean
  pinned: boolean
  /**
   * 工作台发布的那一份预设。uid 形如 `A1/STANDARD` —— 「下载」与「应用」
   * 都落在 `package` 的那两格（`STORAGE.clientPresets` / `STORAGE.clientActive`），
   * 契约的 `applyPreset` 只认仓库 asset id，release 行没有它，所以动作认这一个。
   */
  releaseUid?: string
  /** 它属于哪一次发布（包版本）。来源列那枚 chip 的动态那一截读它 */
  releaseVersion?: string | null
}


export interface PresetLocalRow extends PresetRowBase {
  scope: 'local'
  origin: PresetOrigin
  /** 官方副本才有交付身份。用户自己的文件与工作台发布的云端没有它，所以**没有**这个字段 */
  delivery?: PresetFileInfo['delivery']
  /**
   * 用到这个文件的版本 **id**（与 `versions` 显示名并排）。
   *
   * 「应用」这一步要把它写进底账（首页反填要的就是机型 id + 版本 id）——
   * 只有官方行有它：release 行走 uid、用户自己的文件不属于任何版本。
   */
  versionIds?: string[]
  /**
   * 用户自己的文件没标适用机型。
   *
   * 这种文件在**任何机型下都列出来**并在界面上说一句「未标机型」—— 替他猜一个机型
   * 会让一个本来通用的文件凭空绑到一台机器上，而按机型藏起来则等于这个文件永远看不见。
   */
  untagged: boolean
  /**
   * 这一行**生效了没有**。两种类型判据不同（见 `LIVE_TEXT`）：
   *
   *   MKP     是唯一底账（`STORAGE.clientActive`）里那一条 → 已应用 / 未应用
   *   切片器   `getSlicerCopied().includes(assetId)`   → 已复制 / 未复制
   *
   * 原来这里是四档 `status`，但本地表实际只有「生效 / 没生效」两档，而那四档里的
   * `ready`（本地有）对切片器是句废话 —— 在本机不等于切片器看得见它。
   */
  live: boolean
}

export interface PresetCloudRow extends PresetRowBase {
  scope: 'cloud'
  /**
   * 云端表上的东西：官方仓库的文件，或者**工作台发布的那一次 Release** 里的预设。
   * 用户自己的文件云端根本没有。
   */
  origin: 'official' | 'release'
  delivery: PresetFileInfo['delivery']
  /** 在不在本机。官方文件看 `getLocalFiles()`，release 行看 `STORAGE.clientPresets` —— 两套底账，行上不说谎 */
  downloaded: boolean
}

export type PresetTableRow = PresetLocalRow | PresetCloudRow

/** 一张表的结果。`total` 是**筛前**的条数 —— 空态要分清「被筛掉了」与「本来没有」 */
export interface PresetTableData<R> {
  rows: R[]
  total: number
}

/**
 * 两张表都要的那一堆输入。
 *
 * 写成一个对象而不是七个位置参数：调用处 `localRows(nodes, id, files, ids, applied, 'mkp', q, set)`
 * 这种写法，读的人得数到第五个才知道那个字符串是什么。
 */
export interface PresetRowsInput {
  /**
   * 要汇总哪几台机型（`machinesInScope()` 的结果）：选了一台就一台，
   * 「全部机型」就是全部。**按机型汇总，不按版本** —— 版本是行上的一列。
   */
  machines: PresetMachineNode[]
  /** 当前机型 id。空串 = 全部机型（用户自己的文件那一半按它筛，空串就不筛） */
  machineId: string
  /** `api.getLocalUserFiles()` 的结果。**云端没有这些文件**（假后端给的是固定演示集合） */
  userFiles: LocalUserFile[]
  /** `api.getLocalFiles()` 的结果（同样是固定演示集合） */
  localIds: Set<string>
  /**
   * `api.getSlicerCopied()` 的结果：**已经复制到切片器 profile 目录**的 asset id。
   *
   * 和 `localIds` 是两件事 —— 在本机不等于切片器看得见它。切片器行的「生效」只看这一个。
   */
  slicerCopiedIds: Set<string>
  /** 唯一底账（`STORAGE.clientActive`）里那一条。**全表最多一份**，null = 一套都还没应用 */
  active: ActiveEntry | null
  kind: PresetKindAxis
  query: string
  pinned: Set<string>
  /**
   * 工作台发布 —— 云端**最新一次** Release 摊开的预设文件。
   * `usePresetData` 从 `store/package` 的 `listCloud()` 取；空数组 = 云端还没有工作台的发布。
   */
  releasePresets: ReleasePresetSource[]
  /** 本机 preset 目录里已有的那些（`STORAGE.clientPresets` 的值）。本地表的 release 行就是它们 */
  localReleases: ReleasePresetSource[]
  /** 云端最新一次发布的包版本（来源列那枚 chip 用） */
  releaseVersion: string | null
}

/**
 * 一份工作台发布的预设，摊平成行要用的形状。
 * 数据来自 `store/package`（云端 `listCloud()` / 本机 `presets()`），那层不改。
 */
export interface ReleasePresetSource {
  /** `A1/STANDARD` 这种。下载 / 应用两个动作都认它 */
  uid: string
  machineId: string
  versionId: string
  fileName: string
  /** TOML 正文。大小列按它的字节数算，不编 */
  content: string
  /** 时间列：云端是发布时刻，本地是获取时刻 */
  at: string | null
  /** 它属于哪一次发布（包版本） */
  releaseVersion: string | null
}

/** 一台机型下的一个文件：代表那一份 + 用到它的版本名与 id */
interface MachineFile {
  machineId: string
  file: PresetFileNode
  /** 用到它的版本显示名，按机型目录里的顺序，已去重 */
  versions: string[]
  /** 与 versions 一一对应的版本 id（「应用」这一行时要写进底账，首页反填用） */
  versionIds: string[]
}

/**
 * 按机型把文件汇总成一行一个。
 *
 * 树是「机型 → 版本 → 文件」，同一个文件在三个版本下就有三个 `PresetFileNode`
 * （BBS 工艺按机型走，每个版本都用得到；可选的 0.2mm profile 也挂在每个版本下）。
 * 这里按 **path** 合并 —— 与 `countsOfMachine()` 的去重键是同一个，两处不会各说一套。
 *
 * 整机型没配资源的那种（A2L：`unavailable` 且一个文件都没有）自然贡献 0 行。
 */
function machineFiles(machines: PresetMachineNode[]): MachineFile[] {
  const out: MachineFile[] = []

  for (const m of machines) {
    /* path → 那一行。Map 保插入顺序，所以行的先后跟着版本顺序，不靠 sort 兜 */
    const at = new Map<string, MachineFile>()
    for (const v of m.versions) {
      for (const f of v.files) {
        const hit = at.get(f.path)
        if (hit === undefined) {
          at.set(f.path, {
            machineId: m.machine.id,
            file: f,
            versions: [v.version.name],
            versionIds: [v.version.id],
          })
        } else if (!hit.versions.includes(v.version.name)) {
          /* 同名版本（不同机型下都有「标准版」）在这一层不会撞：一次只汇总一台机型 */
          hit.versions.push(v.version.name)
          hit.versionIds.push(v.version.id)
        }
      }
    }
    out.push(...at.values())
  }

  return out
}


/** 左边那条分段控件。MKP 以外的（bbs / orca）都算「切片器配置」 */
function matchesKind(axis: PresetKindAxis, kind: FileKind): boolean {
  return axis === 'mkp' ? kind === 'mkp_preset' : kind !== 'mkp_preset'
}

/**
 * 置顶的排最前，其余按文件名。
 *
 * 置顶是**纯前端的排序**（落 localStorage，见 `usePresetData` 的 PINNED_KEY），
 * 不是「设为当前」—— 应用是另一件事，右键菜单里没有它。
 */
function sortRows<R extends PresetRowBase>(rows: R[]): R[] {
  return [...rows].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
    return a.fileName.localeCompare(b.fileName)
  })
}

/**
 * 机型 id → 显示名。
 *
 * 两张表的「机型」那一列都查它。`machines` 是当前这一档的机型（选了一台就一台，
 * 「全部机型」就是全部），所以用户自己的文件标了一台**不在这一档里**的机型时查不到 ——
 * 那时退回 id，不留空：一个 id 也比什么都不写有用。
 */
function machineNames(machines: PresetMachineNode[]): Map<string, string> {
  return new Map(machines.map((m) => [m.machine.id, m.machine.display]))
}

/**
 * 版本 id → 显示名（release 行的「版本」列）。
 *
 * release 里存的只有 versionId（`STANDARD`），表格那一列要的是人话（「标准版」）——
 * 名字只认当前这一档机型目录里有的；查不到（机型被筛掉了之类）退回 id，不留空。
 */
function versionNameLookup(
  machines: PresetMachineNode[],
): (machineId: string, versionId: string) => string {
  const byMachine = new Map<string, Map<string, string>>()
  for (const m of machines) {
    byMachine.set(
      m.machine.id,
      new Map(m.versions.map((v) => [v.version.id, v.version.name])),
    )
  }
  return (machineId, versionId) => byMachine.get(machineId)?.get(versionId) ?? versionId
}

/** TOML 正文的字节数 → 人话。与假后端 sizeText 的口径一致（KB 一位小数） */
function tomlSizeText(content: string): string {
  const bytes = new TextEncoder().encode(content).length
  return bytes >= 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${bytes} B`
}

/**
 * 本地表：**本机磁盘上真有的文件**。
 *
 * 两种来源拼在一起，用 `origin` 分开：
 *
 *   官方副本  当前这些机型的文件里，`getLocalFiles()` 说已经下到本机的那些
 *   我的文件  `getLocalUserFiles()` 给的，**仓库里查不到**，所以没有交付身份、也不属于任何版本
 *
 * 官方那一半按**机型**汇总（一个文件一行，用到它的版本收进 `versions`）；
 * 我的那一半按用户自己标的适用机型筛，**没标的在任何机型下都列**（见 `untagged`）。
 *
 * 「生效」两种类型两套判据（见 `PresetLocalRow.live`）：MKP 看唯一底账
 * （`STORAGE.clientActive`）里那一条，切片器看已复制到切片器目录的那个集合。
 */
export function localRows(input: PresetRowsInput): PresetTableData<PresetLocalRow> {
  const {
    machines,
    machineId,
    userFiles,
    localIds,
    slicerCopiedIds,
    active,
    kind,
    query,
    pinned,
    localReleases,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)

  const official = machineFiles(machines)
    .filter((e) => matchesKind(kind, e.file.kind))
    /* 四档里的 missing 不属于本地表 —— 判据只有 statusOf 这一条 */
    .map((e) => ({ e, status: statusOf(e.file, localIds, active) }))
    .filter(({ status }) => status !== 'missing')
    .map(({ e, status }): PresetLocalRow => {
      const f = e.file
      const pinKey = f.id ?? f.path
      const applied = status === 'applied'
      return {
        rowKey: `${e.machineId}:${f.path}`,
        assetId: f.id,
        pinKey,
        fileName: f.fileName,
        path: f.path,
        kind: f.kind,
        machineId: e.machineId,
        machineText: names.get(e.machineId) ?? e.machineId,
        versions: e.versions,
        versionIds: e.versionIds,
        nozzle: f.nozzle,
        layerHeight: f.layerHeight,
        sizeText: f.sizeText,
        modifiedText: f.modifiedText,
        statFrom: f.statFrom,
        applied,
        pinned: pinned.has(pinKey),
        scope: 'local',
        origin: 'official',
        delivery: f.delivery,
        untagged: false,
        /* MKP 的生效是「被应用」，切片器的生效是「被复制进切片器目录」—— 两回事 */
        live: kind === 'mkp' ? applied : f.id !== undefined && slicerCopiedIds.has(f.id),
      }
    })

  const mine = userFiles
    .filter((f) => matchesKind(kind, f.kind))
    /* 「全部机型」（空串）不筛机型；没标机型的在任何一档下都列 */
    .filter((f) => machineId === '' || f.machineIds.length === 0 || f.machineIds.includes(machineId))
    .map((f): PresetLocalRow => ({
      /* rowKey 带上当前那一档机型：换机型时这一行要当成新的一行重画（焦点与菜单都跟着行走） */
      rowKey: `mine:${machineId}:${f.id}`,
      /* 仓库里没有它，所以没有 assetId —— 「已应用」也就不可能落在它身上 */
      pinKey: f.id,
      fileName: f.fileName,
      path: f.path,
      kind: f.kind,
      machineId,
      /* 他标了几台就写几台；一台都没标写「—」，名称列上那枚「未标机型」说的是同一件事 */
      machineText:
        f.machineIds.length === 0
          ? DASH_
          : f.machineIds.map((id) => names.get(id) ?? id).join(' · '),
      /* 用户自己的文件不属于任何版本。表格那一列写「—」，不替他猜一个 */
      versions: [],
      nozzle: f.nozzle,
      layerHeight: f.layerHeight,
      /* 契约的 LocalUserFile 上没有大小与时间 —— 留 undefined，界面写「未知」 */
      applied: false,
      pinned: pinned.has(f.id),
      scope: 'local',
      origin: 'mine',
      untagged: f.machineIds.length === 0,
      /* 没有 asset id 就没法「应用 / 复制」（契约只认 asset id），所以永远是没生效那一档 */
      live: false,
    }))

  /*
   * 工作台发布的那些：下载之后它们就躺在本机 preset 目录（`STORAGE.clientPresets`），
   * 本地表说的就是「本机磁盘上真有的文件」—— 所以这一半**只收下载过的**。
   * 「生效」认唯一底账 `STORAGE.clientActive` 里那一条（与官方行合流，不再分两套）。
   */
  const release = localReleases
    .filter(() => matchesKind(kind, 'mkp_preset'))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .map((p): PresetLocalRow => {
      const live = active !== null && active.kind === 'release' && active.ref === p.uid
      return {
        rowKey: `release:${machineId}:${p.uid}`,
        /* 仓库里没有它，没有 assetId —— 「应用」认 releaseUid（页面里分流） */
        pinKey: `release:${p.uid}`,
        fileName: p.fileName,
        path: `本机预设目录 · ${p.uid}`,
        kind: 'mkp_preset',
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        sizeText: tomlSizeText(p.content),
        modifiedText: p.at ?? undefined,
        applied: live,
        pinned: pinned.has(`release:${p.uid}`),
        scope: 'local',
        origin: 'release',
        untagged: false,
        releaseUid: p.uid,
        releaseVersion: p.releaseVersion,
        live,
      }
    })

  const all = [...official, ...mine, ...release]
  return {
    rows: sortRows(all.filter((r) => fileMatchesQuery(r, query))),
    total: all.length,
  }
}

/**
 * 云端表：**菜单上的官方文件**（已分配 + 可选）。
 *
 * 取的是树上这些机型的文件 —— 仅归档的文件建树时就整个剔掉了（`archivedIds`），
 * 所以这里不需要再过滤一遍，也**不该**再过滤一遍：多一处过滤就多一处可能漏。
 *
 * 行尾两件事分开说：`downloaded` 是「你机器上有没有」，`delivery` 是「要不要你自己动手」。
 * `applied` 是第三件事，判据只有一条 asset id 相等 —— 所以一个还没下的文件也可能是
 * 「正在配着的那一份」，那时页脚会说「配着 X（还没下）」。
 */
export function cloudRows(input: PresetRowsInput): PresetTableData<PresetCloudRow> {
  const {
    machines,
    machineId,
    localIds,
    active,
    kind,
    query,
    pinned,
    releasePresets,
    localReleases,
    releaseVersion,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const localReleaseIds = new Set(localReleases.map((p) => p.uid))

  /*
   * 凡文件名在**云端最新发布**里打包过的，官方行不再列出 —— 发布行接管它
   * （作者：「这种官方是不是不用再存在了」）。两行本来就同源：官方行是仓库身份、
   * 「下载」是死按钮（downloadFiles 未实现）；发布行才是真能下的那一行。
   * 没有发布（列表为空）时官方行照列，行为不退化。
   * 本地表不动 —— 那里「官方」说的是"你机器上的官方副本"，是本机事实。
   */
  const releasedNames = new Set(releasePresets.map((p) => p.fileName))
  const official = machineFiles(machines)
    .filter((e) => matchesKind(kind, e.file.kind))
    .filter((e) => !releasedNames.has(e.file.fileName))
    .map((e): PresetCloudRow => {
      const f = e.file
      const pinKey = f.id ?? f.path
      return {
        rowKey: `${e.machineId}:${f.path}`,
        assetId: f.id,
        pinKey,
        fileName: f.fileName,
        path: f.path,
        kind: f.kind,
        machineId: e.machineId,
        machineText: names.get(e.machineId) ?? e.machineId,
        versions: e.versions,
        nozzle: f.nozzle,
        layerHeight: f.layerHeight,
        sizeText: f.sizeText,
        modifiedText: f.modifiedText,
        statFrom: f.statFrom,
        applied:
          active !== null && active.kind === 'official' && f.id !== undefined && f.id === active.ref,
        pinned: pinned.has(pinKey),
        scope: 'cloud',
        origin: 'official',
        delivery: f.delivery,
        downloaded: f.id !== undefined && localIds.has(f.id),
      }
    })

  /*
   * 工作台发布的那一份 Release —— 云端表上最新的几行。
   * 与官方文件同列一个「来源」chip 区分；「已下载」看 `STORAGE.clientPresets`，
   * 「正在生效」看唯一底账 `STORAGE.clientActive`。release 里全是 MKP 的 toml，切片器档自然一行都不出。
   */
  const release = releasePresets
    .filter(() => matchesKind(kind, 'mkp_preset'))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .map((p): PresetCloudRow => {
      const live = active !== null && active.kind === 'release' && active.ref === p.uid
      return {
        rowKey: `release-cloud:${p.uid}`,
        pinKey: `release:${p.uid}`,
        fileName: p.fileName,
        path: `工作台发布 / ${p.machineId} / ${p.versionId}`,
        kind: 'mkp_preset',
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        sizeText: tomlSizeText(p.content),
        modifiedText: p.at ?? undefined,
        applied: live,
        pinned: pinned.has(`release:${p.uid}`),
        scope: 'cloud',
        origin: 'release',
        releaseUid: p.uid,
        releaseVersion,
        delivery: 'default',
        downloaded: localReleaseIds.has(p.uid),
      }
    })

  const all = [...official, ...release]
  return {
    rows: sortRows(all.filter((r) => fileMatchesQuery(r, query))),
    total: all.length,
  }
}


/** 版本标签（推荐 / 热门 / 最新）。没有就不显示，不补一个「无」 */
export const tagOf = (version: MachineVersion): string | null => version.tag ?? null


// ——————————————————————————————————————————————————————————————
// 切片器的喷嘴 / 层高筛选
//
// 作者：切片器配置「希望能按照喷嘴，层高筛选」，但「下拉菜单感觉不太适合，
// 因为层高很多」—— 所以是**一排轻量 chips**，放不下的值收进「更多」。
// MKP 没有这两个字段，也没有应用之外的状态筛选：那片位置由「已应用状态条」占。
// ——————————————————————————————————————————————————————————————

/**
 * 从一组行里提取喷嘴 / 层高的可选值（去重 + 数值升序）。
 *
 * 可选项要从**没筛过**的行里取 —— 从筛过的行取的话，选中 0.2 之后再点 0.4，
 * 0.2 的 chip 就消失了。排序按数值不按字符串：'0.10' 的字符串序会插到 '0.2' 前面。
 */
export function slicerFilterValues(
  rows: ReadonlyArray<{ nozzle?: string; layerHeight?: string }>,
): { nozzles: string[]; layers: string[] } {
  const byNum = (a: string, b: string): number => {
    const da = Number.parseFloat(a)
    const db = Number.parseFloat(b)
    if (Number.isFinite(da) && Number.isFinite(db) && da !== db) return da - db
    return a.localeCompare(b)
  }
  return {
    nozzles: [...new Set(rows.map((r) => r.nozzle).filter((v): v is string => v !== undefined))].sort(byNum),
    layers: [...new Set(rows.map((r) => r.layerHeight).filter((v): v is string => v !== undefined))].sort(byNum),
  }
}

/**
 * 把喷嘴 / 层高筛选应用到行上。空串 = 「全部」那一档，不筛。
 *
 * 不塞进 `localRows` / `cloudRows`：那两个纯函数管「这张表有什么」，
 * 喷嘴 / 层高是**这一屏**的筛选 —— 页面先拿一遍没筛的行（可选项的来源），
 * 再用这一个函数出筛后的表，两件事各走各的。
 */
export function applySlicerFilters<R extends { nozzle?: string; layerHeight?: string }>(
  rows: R[],
  nozzle: string,
  layer: string,
): R[] {
  return rows.filter(
    (r) => (nozzle === '' || r.nozzle === nozzle) && (layer === '' || r.layerHeight === layer),
  )
}
