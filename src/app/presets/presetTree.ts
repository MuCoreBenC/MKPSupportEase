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
 *           与用户自己放进预设目录的（`getUserPresetFiles()` —— 用户线，**云端没有它们**）
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
 * 为什么（官方那一档拿不到真值时）。**用户线那一份是真值** —— 后端扫盘给的大小与时刻
 * （`statFrom: 'file'` 那一档），不用编也不用写「未知」。
 *
 * `FileRef.size`（字节数）仍恒为 undefined，它和 `sizeText` 不是同一个东西，别混。
 *
 * # 两种类型的「生效」是两件不同的事
 *
 * ```
 * MKP     生效 = 设为当前配置（唯一底账 run/active-preset.json）  状态 已应用 / 未应用
 * 切片器   生效 = 复制到切片器目录（copyToSlicer）       状态 已复制 / 未复制
 * ```
 *
 * 切片器 profile 光下到本机没用 —— 它得躺在切片器自己的 profile 目录里才生效。
 * 所以本地表那一列不能两种类型共用一套词（见 `LIVE_TEXT`）。
 */

import type {
  ActivePreset,
  BasedOn,
  FileKind,
  FileRef,
  FilesState,
  Machine,
  MachineVersion,
  MenuEntry,
  MineState,
  PresetFileInfo,
  UserPresetFile,
  VersionFiles,
} from '../../api'

/**
 * 用户自己的一份文件（用户线，`presets-mine/`）。
 *
 * 名字直接用契约那一个，**不抄字段**：契约改了这里跟着改。这里**转出去**是因为
 * 这一页的输入形状（`PresetRowsInput`）也把它写在签名上 —— 调用方不必再去 `src/api` 取一次。
 */
export type { UserPresetFile }

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
 * **认不出是哪一类**时的说法（行上的 `kind === null`）。
 *
 * 什么时候会认不出：用户自己放的文件（用户线）。后端只按扩展名认 —— `.toml` 是 MKP 预设；
 * 切片器那两类都是 `.json`，**光看扩展名分不出是 bbs 还是 orca**，所以照实认不出。
 * 这一档在任何类型档下都列（不藏），认不出来也不编一个类别给它。
 */
export const KIND_UNKNOWN = '认不出是哪一类'

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
    on: '已应用：唯一底账（使用中指针，run/active-preset.json）说正在使用的就是它。**全局唯一** —— 换机型也不会变出第二个',
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

/**
 * 交付目录里没有记录的官方行为什么没有操作按钮（A2 人话化：发生了什么 + 能干什么）。
 *
 * 两种真实状态共用这一句：用户自己放进 `mkp/` 的来路不明文件；以及演示数据里
 * 旧资产库的行（新交付链里没有它的身份）。共同的事实是**目录里没有它** ——
 * 所以不再断言"是你自己放的"（对登记在案的文件那是假话），也不再说
 * asset id / 契约 / "点了必报错"那种开发者话。
 */
export const NO_ASSET_WHY =
  '这份文件不在 SupportEase 的交付目录里（不是从软件里下载的那一份），' +
  '所以不能直接应用，也不能直接另存。想用它：把它拖进窗口导入成「我的文件」，' +
  '或从云端重新下载官方版本'

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
 * 发布行的「下载」：这一条**是真的能下的**（`downloadCatalogFile` 落进下载区 `mkp/`）——
 * 原来所有云端行共用官方那条「不假装下载成功」的说明，文案与行为不符，按行分流。
 */
export const RELEASE_DOWNLOAD_WHY =
  '从目录登记的交付清单里下载这一份到下载区 mkp/ —— 下载 ≠ 使用，生效要到本地表里点「应用」'

/**
 * 「时间」拿不到时的那两句（**有值的时候 tooltip 直接给具体时间**，不解释实现 ——
 * `catalog.publishedAt` 这种内部名字不许出现在界面上；曾经过的弯路：tooltip 写成
 * 「这次发布的时刻 —— 发布侧落盘时写进目录的……」，用户悬停半天拿到的是一段实现说明）。
 *
 * 时间**两种来源分归两张表**（2026-10-06）：云端表是**云端更新时间**（发布侧盖进目录的戳）、
 * 本地表是**下载时间**（盘上那份的 mtime）；显示一律**按本机时区**（`parseStatDate`）。
 */
export const RELEASE_TIME_WHY = {
  cloudMissing: '云端没有记这次发布的时间 —— 随包目录不带时间，旧版发布的目录也没有',
  localMissing: '说不出这份字节是什么时候到的 —— 它认不出属于哪一版，这种字节不记时间，照实「未知」',
} as const

/**
 * 旧版本抽屉里那两个时间的标签与「未知」的写法。**两个时间各是各，永不互相顶替**：
 *
 *   云端发布  这一版在云端发布时的时刻 —— 跟着这一版字节走，反查版本出身；
 *             早于版本记忆的照实「未知（早于版本记忆）」，不拿"现在"顶
 *   替换时间  它被换下来那一刻（替换事件）—— 你动它的时刻，如实说，但只当下要信息
 */
export const ARCHIVE_TIME = {
  published: '云端发布',
  replaced: '替换时间',
  publishedUnknown: '未知（早于版本记忆）',
  replacedUnknown: '未知',
} as const

export const RELEASE_SIZE_WHY = '按这一份 TOML 的字节数算的'

/**
 * 来源：这个文件是官方的、用户自己的，还是**目录登记的交付预设**。
 *
 * 权限也挂在这一列上：官方副本不能改名（复制一份再改），用户自己的文件删了没有任何地方能找回来。
 */
export type PresetOrigin = 'official' | 'mine' | 'release'

export const ORIGIN_TEXT: Record<PresetOrigin, string> = {
  official: '官方',
  mine: '我的',
  release: '官方交付',
}

export const ORIGIN_WHY: Record<PresetOrigin, string> = {
  official: '官方：仓库里那份文件下载到本机的副本。不能改名（复制一份再改），删了可以从云端重新下载',
  mine: '我的：你自己放进预设目录的文件，云端没有它。可以改名，但删了没有任何地方能找回来',
  release: '官方交付：目录（catalog）里登记的交付预设，发布方写、客户端从下载区拿。云端表只管下载，生效用本地那颗「应用」',
}

/**
 * 数据源（设置页那一个）→ 来源列上「官方交付」行显示的字。
 *
 * 用户问的是「这文件从哪来」—— 答案是 **Gitee / GitHub / 自定义源**，不是一串目录指纹
 * （那串曾经过出现在来源 chip 上：`官方交付 · 08e55a3f…`，作者原话「我觉得这个没有意义」。
 * 指纹唯一还有意义的位置是展开详情里的「云端版本」—— 那是版本的身份证，不是来源）。
 * 没配源（离线 / 随包）就退回「官方」—— 照实说不出是哪一个源，不编。
 */
export function sourceTextOf(sourceLabel: string | null): string {
  switch (sourceLabel) {
    case 'gitee':
      return 'Gitee'
    case 'github':
      return 'GitHub'
    case 'custom':
      return '自定义源'
    default:
      return ORIGIN_TEXT.official
  }
}

/**
 * 来源列 / 展开详情「来源」一格的统一出口。
 *
 * 三种来源三种说法：
 *   官方      官方副本（仓库文件下到本机的那份）
 *   release   官方交付：从**当前数据源**的目录下载的（`GitHub` / `Gitee` / `自定义源`）
 *   mine      我的：再分三档 ——
 *             `复制自 <文件名>`（出处账记的用户文件复制，**可点击定位**到那一份）
 *             `复制自 <文件名>`（官方另存出来的，血统说的；定位到云端那一份官方）
 *             `导入`（出处账记的导入档）
 *             都没有 ⇒ 「我的」（手工放进目录的）
 *
 * `locate` 给了就是「这一格可以点」：点击后页面定位到来源那一行并高亮一下。
 */
export interface LocateTarget {
  scope: PresetScopeAxis
  /** null = 来源行与当前类型档相同（用户自己那份认不出类别的），定位时不动这一轴 */
  kind: PresetKindAxis | null
  /** 目标行键（按「全部机型」那一档算的 —— 定位时页面会把机型切回全部） */
  rowKey: string
  /** 目标文件名（定位失败时提示条用得上） */
  fileName: string
}

export function originCellOf(
  row: PresetTableRow,
  sourceLabel: string | null,
): { text: string; title: string; locate?: LocateTarget } {
  /*
   * **正向判别 `=== 'mine'`**，不是两个反向 `!==`：PresetCloudRow 的 origin 本身是
   * 联合（'official' | 'release'），TS 的反向排除对"判别值是联合的成员"不彻底
   * （成员删不掉，后面的字段访问照样报错）。正向判别一次到位 —— 剩下的必是 mine 行。
   */
  if (row.origin === 'mine') {
    /* 出处账优先（用户文件 → 用户文件、导入），其次血统（官方另存出来的），再退「我的」 */
    if (row.provenance === 'copy' && row.copiedFromName) {
      return {
        text: `复制自 ${row.copiedFromName}`,
        title: `复制自 ${row.copiedFromName} —— 点击定位到那一份（你现在这份是它的副本，两边各改各的）`,
        locate: {
          scope: 'local',
          kind: row.kind === null ? null : 'mkp',
          /* 定位永远按「全部机型」那一档算行键（mine 行的键里带机型筛选） */
          rowKey: `mine::${row.copiedFrom}`,
          fileName: row.copiedFromName,
        },
      }
    }
    if (row.provenance === 'import') {
      return {
        text: '导入',
        title: '导入：这个文件是你从外面导进来的（拖拽 / 文件选择器）—— 云端没有它，删了没有地方能找回来',
      }
    }
    if (row.basedOnSource) {
      const sourceName = row.basedOnSource.split('/').pop() ?? row.basedOnSource
      return {
        text: `复制自 ${sourceName}`,
        title: `${basedOnCellText(row)} —— 点击定位到官方那一份（你这份是从它另存/修改出来的）`,
        locate: {
          scope: 'cloud',
          kind: 'mkp',
          rowKey: `release-cloud:${sourceName}`,
          fileName: sourceName,
        },
      }
    }
    return { text: ORIGIN_TEXT.mine, title: ORIGIN_WHY.mine }
  }
  if (row.origin === 'release') {
    const source = sourceTextOf(sourceLabel)
    return {
      text: source,
      title: `官方交付：发布方写进目录、从 ${source} 的预设数据源下载（目录指纹 ${row.releaseVersion ?? '未知'}）`,
    }
  }
  return { text: ORIGIN_TEXT.official, title: ORIGIN_WHY.official }
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
 * **盘上那一份是什么**（catalog 登记的交付预设才有的那一档）。
 *
 * 三个读的组合，**不是前端猜的**：
 *
 *   `api.getDownloadedFiles()`  盘上在、且字节与目录登记的一致 → `ok`
 *   `api.getStaleFiles()`       盘上在、但字节与目录不一致     → 再问下面这一条
 *   `api.getDeliveryTrust()`    那不一致的字节**认得出是哪一版吗** → `old` / `tampered`
 *   三个都不含它                                              → `missing`（还没下过，合法状态）
 *
 * 为什么后两档必须分开（第三圈第 6 层）：盘上那份可能是**我们发过的旧版本**
 * （云端换了新版），也可能是**被改过 / 来路不明**的字节 —— 前者的正文我们认得了、
 * 说得出来源；后者这台机器上查不出它属于哪一版。只说一句「需更新」是把"本机内容可不可信"
 * 和"云端有没有新版"混成一句：用户既不知道自己的文件是不是被改过，也不知道该不该等更新。
 *
 * 三档不一致的（old / tampered）**共有一条边界**：不许应用、不许改、不许复制，
 * 只能重新下载一份干净的 —— 见 [`isSuspectRelease`]。
 */
export type ReleaseFileState = 'missing' | 'ok' | 'old' | 'tampered'

export const RELEASE_STATE_TEXT: Record<ReleaseFileState, string> = {
  missing: '未下载',
  ok: '已下载',
  old: '旧版本',
  tampered: '内容异常',
}

export const RELEASE_STATE_WHY: Record<ReleaseFileState, string> = {
  missing: '未下载：目录里登记了它，你机器上还没有',
  ok: '已下载：下载区 mkp/ 里有它，字节与目录登记的一致。下载 ≠ 使用，生效要到本地表里点「应用」',
  old:
    '旧版本：盘上这一份的字节与归档里那一版**逐字节相同** —— 它是我们发过的某一版旧版（云端已经换了新的）。' +
    '正文认得出来，可以点开旧版本那一格对照；装到机器上的动作请用「更新」换成当前版本',
  tampered:
    '内容异常：盘上这一份的字节既不是目录登记的当前版本，也不是我们发过的任何一版 —— ' +
    '这台机器上查不出它属于哪一版（被改过 / 来路不明）。不许应用、不许改、不许复制，先「更新」换一份干净的',
}

/** 内容存疑的那两档（旧版本 / 内容异常）—— 它们共用同一条边界 */
export function isSuspectRelease(state: ReleaseFileState | undefined): boolean {
  return state === 'old' || state === 'tampered'
}

/**
 * **云端 vs 盘上**的三态 —— release 行「状态」与操作列的主词都从它出。
 *
 * 三个概念**不是** `ReleaseFileState`：那是"盘上那份认不认得出"（信任），
 * 这一个是"云端有没有比盘上新的货"（更新）。**云端有更新时主词就是「有更新」** ——
 * 不管盘上那份是认得出的旧版还是认不出的字节，用户要做的事是同一件：
 * 点「更新」换成当前版（同一条下载管道）。
 *
 *   `missing`  未下载：目录里有、盘上没有
 *   `latest`   已下载：盘上字节与目录一致
 *   `update`   有更新：盘上有字节、但与目录登记的当前版不一致（旧版或认不出的都算）
 */
export type ReleaseUpdateState = 'missing' | 'latest' | 'update'

export function updateStateOf(state: ReleaseFileState): ReleaseUpdateState {
  switch (state) {
    case 'missing':
      return 'missing'
    case 'ok':
      return 'latest'
    default:
      return 'update'
  }
}

export const UPDATE_STATE_TEXT: Record<ReleaseUpdateState, string> = {
  missing: '未下载',
  latest: '已下载',
  update: '有更新',
}

/**
 * 「有更新」底下那行**认不出**的注记（只有 `tampered` 那一档给）。
 *
 * 2026-10-06 查明的实情：客户端的版本记忆只留得住"每一代被换下的目录"（归档链），
 * 在链建起来之前换过的版本谁都不记得 —— 所以"认不出"≠"被改过"，把「内容异常」顶在
 * 状态主词上是**吓唬人**（实测 7 份全部是正规旧版）。主词改说「有更新」，
 * 这句话作为第二行把实情讲全，动作给「更新」—— 对"旧版"和"真被动过"都是正解。
 */
export const RELEASE_UNTRUSTED_NOTE =
  '本机这份认不出是官方哪一版 —— 官方连发几版时，更早的版本指纹在客户端留不全，' +
  '所以认不出不等于被改过（没人动过它的话，多半就是旧版）。点「更新」换成当前版即可'

/** release 行操作列那颗按钮：更新三态各一个字。**没有「重新下载」** —— 修坏档与换新版是同一条管道、同一个动作 */
export const UPDATE_ACTION_TEXT: Record<ReleaseUpdateState, string> = {
  missing: '下载',
  latest: '已下载',
  update: '更新',
}

/** 内容存疑那两档共有的那条边界（右键菜单禁用 / 详情里那句话都用它） */
export const RELEASE_SUSPECT_WHY =
  '这一份的字节不是目录登记的当前版本 —— 内容存疑，所以不许应用、不许改、不许复制。先「更新」换一份干净的回来'

/** 「更新」那颗按钮的说明：它不是"删除重下"，旧份进归档，删除永远不是更新的一部分 */
export const RELEASE_UPDATE_WHY =
  '更新：对盘上这一份再跑一遍下载管道 —— 旧份先归档（archive/）再换新，删除永远不是更新的一部分'

/**
 * 「更新」那颗按钮在**认不出**那一档的说明。
 *
 * 它不是第二套机制：与「下载 / 更新」是**同一条管道**（再下一遍，落点还是目录说的那一个）。
 * 按钮统一叫「更新」（云端有更新就说更新），这里要讲清的是"盘上那份我们不认"——
 * 用户多半不知道文件被谁动过，也不知道更新会不会把他改的东西冲掉。
 */
export const RELEASE_REPAIR_WHY =
  '更新：盘上这一份我们认不出是官方哪一版（不是目录登记的当前版本，已知的旧版本指纹也都对不上）—— ' +
  '再下一份干净的换上，落点与校验与「下载」是同一条管道；旧份进归档，不删'

/**
 * **临时编辑**那条链的话（展开详情里的「修改」→ 编辑器抽屉）。
 *
 * 这一层只做"改 → 另存成用户那一份"这条链本身：官方原件全程不动（只有云端换版本能替换它），
 * 用户改的是临时文件，保存 = 另存成 `presets-mine/<原名>（已修改）<后缀>`。
 * **不做**：改完之后"生效 / 应用"（那是"用户那份也能不能用"那条产品规则的活）、
 * 与官方更新的并存处理（第 7 步）。
 */
export const EDIT_TEXT = {
  cell: '修改',
  open: '改这份',
  title: '改这一份',
  note:
    '改的是**临时文件**：官方原件一动不动（只有云端换版本能替换它），改到一半关掉也还在。' +
    '点「保存为用户文件」才另存成你自己那一份 —— 那份从此与云端脱钩：' +
    '官方怎么更新都不动它，改它也永远不回写官方原件。',
  reused: '上次改到一半的那一份，接着改',
  discard: '放弃这次编辑',
  commit: '保存为用户文件',
  /** 草稿自动落盘失败时那一行（安静地写在抽屉里，不用提示条） */
  draftFailed: '草稿没存上：',
  /** 存到哪去了那句（`replaced` 时另说） */
  saved: (name: string, path: string) => `已保存成你自己那一份：${name}（${path}）`,
  savedAgain: (name: string) => `已保存：${name} —— 盖掉了上一次那份（你改的一直是同一份）`,
} as const

/** 「改这份」那颗按钮的说明（官方线） */
export const EDIT_WHY =
  '修改：把这一份的正文复制进临时文件再改 —— 官方原件不会被改动（只有云端换版本能替换它）'

/**
 * **用户线**那份的「改这份」（第八层）：同一个编辑器抽屉、**同一条临时文件链**，
 * 只有两句话不一样 —— 保存的不是"另存成一份新的"，而是**写回你自己那一份**：
 *
 * ```text
 * 我的 A1-standard（已修改）.toml ──改这份──▶ 临时文件 ──保存──▶ 写回它自己
 * ```
 *
 * 于是不会出现 `（已修改）2.toml` / `（再次修改）.toml` 这种越改越多的名字；
 * 它基于哪一版官方写在文件里（那三行），保存时**原样保留** —— 改的是参数，不是出处。
 */
export const MINE_EDIT_TEXT = {
  cell: '修改',
  open: '改这份',
  title: '改我自己这份',
  note:
    '改的是**临时文件**：保存时才写回你自己那一份（**同一个文件**，不会多出一份）。' +
    '它基于哪一版官方写在文件里 —— 保存时原样保留，你改的是参数、不是出处。',
  commit: '保存回我这份',
  /** 写回之后那句（`path` 说清写回哪去了） */
  savedBack: (name: string, path: string) => `已保存回我自己那一份：${name}（${path}）`,
} as const

/** 用户线「改这份」那颗按钮的说明 */
export const MINE_EDIT_WHY =
  '修改：把我自己这份的正文复制进临时文件再改 —— 保存时写回同一个文件（不会多出一份）'

/**
 * **第十层**：重命名那一口抽屉的话（删除的二次确认在菜单里，话术在菜单项上）。
 *
 * 只改名、不动字节：内容 / 那三行血统 / TOML 都不重写 —— 改完还是同一份预设。
 * 正在使用的那一份也能改名（使用中指针跟着走，不断）；有草稿的那一份也能改名
 * （草稿跟着走，「接着上次改」不接丢）。删除另说 —— 那两道闸在菜单与后端。
 */
export const MINE_RENAME = {
  title: '重命名',
  note:
    '只改名字：文件内容、那三行血统、TOML 都**不动** —— 改完还是同一份预设，只是换了个叫法。' +
    '后缀要保持原样（.toml 还是 .toml）；这里只改名字，不搬文件夹。',
  liveNote: '它正在使用 —— 「已应用」那条底账会跟着改名，不会断。',
  commit: '改名',
} as const

/**
 * **第十一层**：另存为一份新的那一口抽屉的话。
 *
 * 我的文件 → 我的文件（与第八层"官方 → 我的文件"那条另存分开）：**按字节复制**、
 * 那三行 `# based_on` 血统**原样带过去**（不重算 —— 来源已经是用户文件）；原文件一个字节不动。
 * 名字由用户自己起 —— **不预填、不自动改名**（作者：目标存在就拒绝，让他自己换名字）；
 * **一个状态都不碰**：不改「已应用」、草稿也留在原来那份上。
 */
export const MINE_COPY = {
  title: '另存为一份新的',
  note:
    '把我这份**按字节**复制成同一格里另一份新的用户文件 —— 原文件一个字节不动，' +
    '新的那份从诞生起就是独立的一份（之后能自己编辑 / 改名 / 删除 / 应用）。' +
    '那三行 `# based_on` 血统原样带过去（来源已经是用户文件，不重算）。' +
    '名字由你来起：不能和原来一样、后缀保持原样；已经有同名文件了会被拒（不覆盖，也不会自动改名）。',
  commit: '另存为',
} as const

/**
 * **官方交付行的那口另存抽屉**（UX 场景测试 A1 的正路，作者 2026-10-06 定案）：
 * 官方 → 我的文件。可信字节（与「改这份」同一条闸）**按字节复制**，血统三行
 * **新写指向**来源交付文件（官方原件没有血统头）；官方原件一个字节不动、
 * 一个状态都不碰。名字由用户自己起 —— 不预填、不覆盖、不自动改名（与我的行同一套）。
 */
export const RELEASE_COPY = {
  title: '另存为一份新的',
  note:
    '把官方这份**按字节**复制成你自己的一份（presets-mine/）—— 官方原件一个字节不动，' +
    '身世（机型 / 版本 / 来源）跟着它走；新的那份从诞生起就是独立的一份' +
    '（之后能自己编辑 / 改名 / 删除 / 应用）。' +
    '名字由你来起：不能和官方那份一样、后缀保持原样；已经有同名文件了会被拒（不覆盖，也不会自动改名）。',
  commit: '另存为',
} as const

/** 字节数写成人话（`4.2 KB`）。**一处** —— 表里的「大小」与归档抽屉里都用它 */
export function sizeTextOf(size: number): string {
  return size >= 1024 ? `${(size / 1024).toFixed(1)} KB` : `${size} B`
}

/**
 * 归档那一格（展开详情里的「旧版本」）。
 *
 * `archive/` 是**官方版本生命周期**的一部分：云端换版本时，旧的那一份被换下来放进归档
 * （保留最早一份，不覆盖）。它**不是用户修改历史** —— 用户改出来的东西是另一条线
 * （另存成另一份文件），永远不回写官方原件。
 *
 * 这一层做**看得见 / 认得出 / 看得了 / 删得掉**（2026-10-06 起允许删，代价讲清：
 * 云端只有最新版，删了找不回）；仍然不提供"恢复"与"用这份旧版本"
 * （归档管理不在这一层，见 HANDOFF §3.5 的七步顺序）。
 */
export const ARCHIVE_KEY = '旧版本'

export const ARCHIVE_WHY =
  '归档：云端换版本时，旧的那一份被换下来留在这里（保留最早一份，不覆盖）。' +
  '它属于官方文件的生命周期，不是你的修改历史 —— 你改出来的东西是另一份文件（另存），永远不回写官方原件。' +
  '这份留档可以删：但云端只有最新版，删了就找不回（版本链与事件账不受影响）。' +
  '仍然没有「恢复」、也没有「用这份旧版本」。'

/** 那一格上的字：几份 + 可以点开 */
export function archiveOpenText(count: number): string {
  return `${count} 份（点开看）`
}

/**
 * 用户线那一份的抽屉（点「看正文」看的那个）。
 *
 * 它和归档那个抽屉共用外壳，但说的是另一条线：**用户自己的文件**。
 * 这一层只做「看得见、认得出、看得了」—— **改它 / 新建它 / 保存是下一层**，
 * 所以抽屉里没有任何写入按钮（连"另存为"都没有：那是第 5 步）。
 */
export const MINE_DRAWER = {
  title: '我自己的这一份',
  /** 展开详情里那一格的名字与按钮 */
  cell: '正文',
  open: '看正文',
  reading: '正在读…',
  note:
    '这是**你自己的文件**（住 ~/Documents/SupportEase/presets-mine）：云端没有它，' +
    '所以没有 SHA、不属于任何版本、也不参与套餐。官方换版本不会动它；' +
    '改它（同一格里的「改这份」）写回的是它自己，永远不回写官方原件。',
} as const

/** 「看正文」那颗按钮的说明（展开详情里的 title） */
export const MINE_BODY_WHY = `读这一份的正文：它就在你自己的目录里，读它不需要校验（它本来就没有官方 SHA）。`

/**
 * 用户那份**基于官方哪一版**（第七层）。三档说的是**同一件事**：
 * 它当初基于的那一版，和目录里现在这一版是不是同一份 ——
 * **不判这份文件好不好**（用户自己那份从来不是坏文件）。
 *
 * `outdated` 就是「官方：v2，我的：基于 v1」那件事在界面上的落点。
 */
export const BASED_ON_TEXT: Record<BasedOn, string> = {
  current: '基于当前版',
  outdated: '基于旧版官方',
  unknown: '来源说不清',
}

export const BASED_ON_WHY: Record<BasedOn, string> = {
  current: '这份文件头里记着它是从目录里**现在**这一版官方拷出来改的（血统摘要与目录登记的一致）—— 官方没换过版',
  outdated:
    '官方已经换新版了，而这份还是从**旧版**官方派生出来的。**它照常能用、能改**，' +
    '只是不会跟着官方更新 —— 它从来不是官方那一份（要不要把改动挪到新版上，是另一件事）',
  unknown:
    '说不清从哪一版改的：要么这份文件没有血统（手工拷的 / 别的程序写出来的），' +
    '要么它记的来源已经不在目录里了（换源或下线）。**这不影响它是一份正常的用户预设**',
}

/** 展开详情里那一格的名字 */
export const BASED_ON_KEY = '基于'

/**
 * 展开详情里「基于」那一格写什么。**文案收在这一处**（组件只搬）——
 * 与 `versionsText` / `releaseBatchText` 同一条规矩：一句话只有一个出处。
 */
export function basedOnCellText(row: {
  basedOn?: BasedOn
  basedOnSource?: string | null
  basedOnOfficial?: string | null
}): string {
  /* 说得清来源的名字就说它（`A1 · 标准版`），说不清就退回血统里那串原文 */
  const source = row.basedOnOfficial ?? row.basedOnSource ?? ''
  switch (row.basedOn) {
    case 'current':
      return source === '' ? '目录里现在这一版官方' : `${source} · 官方当前版`
    case 'outdated':
      return source === '' ? '旧版官方（官方已换新版）' : `${source} · 官方已换新版`
    default:
      return source === ''
        ? '没有血统（说不清从哪一版改的）'
        : `${source}（目录里已经没有它了）`
  }
}

/**
 * 「应用」用户自己那份的说明。
 *
 * 与官方那份**同一个入口、同一条底账**（`run/active-preset.json`）——
 * 「只读」是文件归属的属性，不是"能不能被使用"的属性（第七层作者定）。
 * 区别只有闸不一样：官方那份要 SHA 与目录对得上；用户那份要落在 `presets-mine/` 那一格里。
 */
export const MINE_APPLY_WHY =
  '应用：把这一份设成正在使用的配置（与官方那份同一条底账 run/active-preset.json）。' +
  '改它不会影响官方那份；官方换版本也不会动你的这份'

/**
 * 用户那份**不能被应用**时的说明：认不出它是 MKP 预设。
 *
 * `.json` 那几份（bbs / orca 光看扩展名分不出）不是预设 —— 给一个点了必被拒的按钮，
 * 比不给糟（与 NO_ASSET_WHY 同一条口径）。
 */
export const MINE_NOT_PRESET_WHY =
  '这一份认不出来是 MKP 预设（切片器那两类都是 .json，光看扩展名分不出是 bbs 还是 orca）—— ' +
  '能被使用的只有 TOML 预设'

/**
 * **第九层**：读不出来的那一份（行上那一枚角标）。
 *
 * 与官方线的「内容存疑」是**两回事**：官方线要 SHA（与目录登记逐字节一致才可信），
 * 用户线只看**文件级**（能读 + UTF-8 + TOML 语法）—— 外部改过一轮但仍是能读的 TOML
 * 照常能用，**不因 SHA 报警**。"是不是一份合法 MKP Preset"（结构 / 参数）不在客户端判：
 * 那要真正的 Preset 解析能力，留给"应用 / 编辑"这类真正要解析的入口。
 */
export const MINE_UNREADABLE_TEXT = '文件无法读取'

/** 那一枚角标 / 灰掉的动作格的 title。`detail` 是后端给的人话原因（比如语法错在第几行） */
export function mineUnreadableWhy(detail: string | null | undefined): string {
  return (
    `${MINE_UNREADABLE_TEXT}${detail == null ? '' : `：${detail}`} —— ` +
    '这一份现在不能应用、也不能改（先把它改回一份能读的 TOML，或者删掉它）。' +
    '它还是你自己的文件：程序只如实说读不出来，不动它'
  )
}

/** 归档抽屉里的那几句话 */
export const ARCHIVE_DRAWER = {
  title: '旧版本',
  /** 每条上那颗按钮 */
  open: '看正文',
  reading: '正在读…',
  bodyTitle: '正文',
  /** 认不出机型版本时那一格写什么 */
  unknown: '认不出是哪台机型的哪一版（目录里已经没有这一份了）',
} as const

/**
 * 批量那一行的字：**这一批里有什么，决定它是"下载"还是"更新"**。
 *
 * 计数由调用方从行上的 `releaseState` 数出来，这里只负责措辞 —— 批量不自己判状态，
 * 它是"多份单文件操作的组合"，判据还是那一个。三档分开说（未下载 / 需更新 / 内容异常），
 * 因为用户要做的事不一样：前两种是补上官方那份，后一种是**盘上那份我们不认**。
 * `已下载的不进这一批`：这一层只解决"多份一起处理"，不解决"再下一遍已经对了的东西"。
 */
export function releaseBatchText(
  missing: number,
  stale: number,
  tampered: number,
): { count: string; label: string; why: string } {
  const total = missing + stale + tampered
  const label =
    missing === 0
      ? `更新 ${total} 份`
      : stale + tampered === 0
        ? `下载 ${total} 份`
        : `下载并更新 ${total} 份`
  const parts = [
    missing > 0 ? `未下载 ${missing} 份` : '',
    stale > 0 ? `需更新 ${stale} 份` : '',
    tampered > 0 ? `内容异常 ${tampered} 份` : '',
  ].filter((s) => s !== '')
  return {
    count: parts.join(' · '),
    label,
    why:
      '这一批＝把多份单文件操作合成一次：全程走同一条下载管道（没有第二套），逐份给结局 —— ' +
      '哪一份没成会单独列出来，绝不压成一句"批量失败"或"批量成功"。' +
      '已下载的那几份不在这一批里（不重复下）。' +
      '范围是**当前机型 + 这一档类型**，不受搜索词影响（搜索是"我在找什么"，不该改变"按一下要动几份"）。',
  }
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
 * 右键菜单里那些**还没有后端**的动作（A2 人话化后的现状）：
 *
 *   契约里有签名   照调，让它抛 `NotImplementedError`（自带人话 hint），界面原样显示
 *                 —— 官方仓库文件的「下载」（`downloadFiles`）是这一种
 *   契约里没签名   不发请求，就地说「这个动作还没有对应的实现」（[`noContractText`]）
 *                 —— 现在只剩「复制链接」一件（要加的话是 `getFileUrl`）
 *
 * **用户文件那五件都不在这一档了**：重命名 / 删除是第十层（`renameUserPreset` /
 * `deleteUserPreset`）、另存为一份新的（我的 → 我的）是第十一层（`copyUserPreset`）、
 * 官方 → 我的文件的另存是 `copyReleaseAsNew`、在文件管理器里显示是第十三层
 * （`revealInFolder`）。原「MISSING_METHOD」待办表已并入这段注释 —— 只剩一件，
 * 不值得一张表。
 */

/**
 * 「界面上还没有对应实现的动作」那一句话（A2 人话化）：发生了什么 + 能干什么。
 * 技术形式（方法名 / 契约名）不进提示条 —— 那是控制台与日志的事。
 * 目前界面上唯一到不了契约的动作是「复制链接」（要加的话是 `getFileUrl`）。
 */
export const noContractText = (): string =>
  '这个动作还没有对应的实现 —— 先用能用的那几步把事情办完'

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
    '已应用：本机有这个文件，而且唯一底账（使用中指针）说正在使用的就是它。全局唯一 —— 换机型也不会变出第二个',
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
  /**
   * 仓库里**每一类**各有几个文件（按 path 去重；已剔掉仅归档的）。
   *
   * 台账那一格「仓库 N」读它 —— **数字跟着当前类型档走**：
   * MKP 档数 MKP 的、切片器档数切片器的，不把别的类型的数混进来
   * （作者 2026-10-02：「'仓库 9' 这种全 catalog 数字不应该混在当前类型的业务语境里」）。
   *
   * 原来那一格是 `totalFiles`（只数 repo 一支 = 切片器资产）：在 MKP 档下显示的是
   * 切片器的数 —— 树是两个来源合成的（`getVersionFiles` 的 MKP + `getPresetFiles`
   * 的切片器），只数一支答不上"仓库里有什么"。
   */
  fileCounts: Record<FileKind, number>
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

  /*
   * 「仓库里每一类几个」：把树上全部文件按 kind 数一遍，**按 path 去重**
   * （同一个 BBS 被三个版本共用只算一个 —— 与 `countsOfMachine` 同一条去重键）。
   * 两个来源合起来数才是完整的：MKP 那一支来自 `getVersionFiles`（不在 `visible` 里），
   * 切片器那一支来自 repo —— 只数一支就是旧 `totalFiles` 的病。
   */
  const fileCounts: Record<FileKind, number> = { mkp_preset: 0, bbs_profile: 0, orca_profile: 0 }
  const counted = new Set<string>()
  for (const m of machineNodes) {
    for (const v of m.versions) {
      for (const f of v.files) {
        if (counted.has(f.path)) continue
        counted.add(f.path)
        fileCounts[f.kind] += 1
      }
    }
  }

  return { machines: machineNodes, fileCounts }
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
 * `active` 是**唯一底账**（AppState 的 activePreset 格，`run/app-state.json`，经唯一客户端订阅）：
 * **全表最多一份**，换机型也不会变出第二个。判据**只认它**。
 *
 * 先判在不在本机，再判是不是正在使用的那一份 —— 「已应用」比「本地有」靠前，
 * 它是「本机有 + 正在用的就是它」。判据只剩这两条。
 * 官方 MKP 行在树上没有 asset id（MKP 不进资产库，见 doc §12.5），它的「正在使用」
 * 按文件名对 —— 目录里登记的交付文件与使用中指针说的是同一份文件名。
 */
export function statusOf(
  file: { id?: string; kind: FileKind; fileName: string },
  localIds: Set<string>,
  active: ActivePreset | null,
): PresetStatus {
  const id = file.id
  if (id === undefined || !localIds.has(id)) return 'missing'
  if (active !== null && file.kind === 'mkp_preset' && file.fileName === active.fileName) {
    return 'applied'
  }
  return 'ready'
}


/**
 * 搜索：按文件名 / 路径。空串 = 不筛。
 *
 * 参数写成结构类型而不是 `PresetFileNode`，是因为用户自己的文件（`UserPresetFile`）
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
   * 官方行不再是「应用」的对象（使用中指针只认 MKP 交付文件）—— 字段留给右键菜单与筛选用，
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
  /**
   * **表格第二行的小字（副标题）**（2026-10-07 作者定：副标题不要位置文案，
   * 改显示**备注**）：备注覆盖账里有 ⇒ 用户的；否则那一版工作台写的 `remark`；
     连版本备注都没有 ⇒ 回落 `path`（路径永远在 `title` 里，不丢）。
   */
  subtitle: string
  /**
   * 备注覆盖账的键（`null` = 这一行没有可改的备注 —— 官方仓库行）。
   * 官方交付行 = `catalog.path`、用户线 = 相对用户根的路径；「改备注」「删了重新下载
   * 就回到工作台那句」都认它。详情面板的备注编辑格只在它非 `null` 时出现。
   */
  remarkKey: string | null
  /**
   * 哪一类。**`null` = 认不出**（用户自己的 `.json`：bbs 与 orca 都是 json，光看扩展名分不出）。
   *
   * 认不出的行**在任何类型档下都列**（藏起来等于说他没这份文件），
   * 展开详情里那一格写「认不出是哪一类」—— 不替他认成 MKP 预设。
   */
  kind: FileKind | null
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
  /** 唯一底账（使用中指针）说正在使用的就是这一份。**全页最多一行** */
  applied: boolean
  pinned: boolean
  /**
   * 目录登记的交付预设。uid 形如 `A1/STANDARD`（行键沿用）；「下载」与「应用」
   * 两个动作认 **fileName**（目录登记的文件名 = 使用中指针的口径），
   */
  releaseUid?: string
  /** 它属于哪一次发布（包版本）。来源列那枚 chip 的动态那一截读它 */
  releaseVersion?: string | null
  /**
   * 交付预设在**本机**的三态（见 `ReleaseFileState`）。**只有 release 行有它** ——
   * 其余来源没有"从目录下载"这一回事。`undefined` = 这一行不是交付预设。
   */
  releaseState?: ReleaseFileState
  /**
   * **这次发布的时刻**（catalog.publishedAt，ISO/UTC → 显示按本机时区）。
   * release 行展开详情里「云端最新版发布于」一格的数据 —— **本地行也要有**：
   * "我盘上这份什么时候到的"（`modifiedText` + `arrivalBy`）与"云端最新版什么时候发的"
   * 是两个版本各自的时间，展开详情要同时给（云端表的 `modifiedText` 就是它，列上已经显示）。
   * null = 目录没盖戳（随包 / 旧版发布），照实「未知」。
   */
  publishedAt?: string | null
  /**
   * 这份字节是怎么**到位**的（2026-10-06 事件时间模型）：`downloaded` = 下载进来
   * （`DeliveryDownloaded`）、`replaced` = 替换上去（`DeliveryReplaced`）、null = 没有事件
   * （认不出出身的字节）。详情面板「下载时间 / 替换时间」的标签跟着它走 ——
   * **标签是事件的名字，不是 UI 自己挑的**。只有本地 release 行有它。
   */
  arrivalBy?: 'downloaded' | 'replaced' | null
  /**
   * **本机这份**属于哪一代目录、那一代在云端发布的时刻（RFC3339；版本出身反查）。
   * 它跟着这一版字节走，云端以后怎么换代都不变 —— 与 `publishedAt`（云端**最新**版）
   * 是两个版本各自的时间。认不出出身 = null（那一格整个不显示，不占「未知」位）。
   */
  ownPublishedAt?: string | null
}


export interface PresetLocalRow extends PresetRowBase {
  scope: 'local'
  origin: PresetOrigin
  /** 官方副本才有交付身份。用户自己的文件与目录交付预设没有它，所以**没有**这个字段 */
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
   *   MKP     是唯一底账（使用中指针）里那一条 → 已应用 / 未应用
   *           （**用户的与官方的共用这一条**：指针说 `origin` + 认 `path` / `fileName`）
   *   切片器   `getSlicerCopied().includes(assetId)`   → 已复制 / 未复制
   *
   * 原来这里是四档 `status`，但本地表实际只有「生效 / 没生效」两档，而那四档里的
   * `ready`（本地有）对切片器是句废话 —— 在本机不等于切片器看得见它。
   */
  live: boolean
  /**
   * **用户自己那份**基于官方哪一版（只有 `origin === 'mine'` 的行有它）。
   *
   * 它是"文件头血统 + 目录现况"比出来的一个事实，不是"这份文件好不好"——
   * `outdated` 的那份照常能用能改，见 [`BASED_ON_WHY`]。
   */
  basedOn?: BasedOn
  /**
   * 血统里记的来源原文（`delivery/mkp/presets/A1-standard.toml`）。没有血统是 `null`。
   * **老文件里可能记着旧形状（`mkp/presets/…`）** —— 那种认不出就是 `unknown`，
   * 不回填、不猜（文件本身照旧能读能改能应用）。
   */
  basedOnSource?: string | null
  /** 来源那份现在对应哪台机型的哪一版（人话，已按名字查好）。认不出是 `null` */
  basedOnOfficial?: string | null
  /**
   * **第九层**：这一份用户文件的**文件级**状态（只有 `mine` 行有它）。
   *
   * `'unreadable'`（读不出来：编码 / TOML 语法 / 指向用户根之外）→ 名称列画
   * 「文件无法读取」，**「应用」与「改这份」都不给**（不给必被后端拒的按钮）；
   * `'ok'` 照常。认不出是哪一类的行没有这一档（`undefined`）。
   */
  mineState?: MineState
  /** 读不出来时后端给的那句人话原因（角标与动作格的 title 用它） */
  mineStateDetail?: string | null
  /**
   * **出处账**：这一份是复制来的还是导入来的（`copy` / `import`）。没记过是 `null`。
   * 与 `copiedFrom` / `copiedFromName` 一起构成「来源：复制自 X / 导入」那一格的数据。
   */
  provenance?: 'copy' | 'import' | null
  /** 出处账记的来源路径（相对用户根）。没有是 `null` */
  copiedFrom?: string | null
  /** 出处账记的来源**文件名**（界面直接显示）。没有是 `null` */
  copiedFromName?: string | null
  /**
   * **这一份自己的归属**（只有 `mine` 行有它）：文件头 `# machine:` / `# variant:`
   * 两行（认不出回落来源那份）。行上的 `machineId` 是**筛选档**（这一行挂在哪个
   * 机型分栏下），归属是**文件自己的** —— 详情面板的「改归属」读这两个。
   */
  ownMachineId?: string | null
  ownVersionId?: string | null
}

export interface PresetCloudRow extends PresetRowBase {
  scope: 'cloud'
  /**
   * 云端表上的东西：官方仓库的文件（切片器档），或者**目录登记的交付预设**（MKP 档）。
   * 用户自己的文件云端根本没有。
   */
  origin: 'official' | 'release'
  delivery: PresetFileInfo['delivery']
  /** 在不在本机。官方文件看 `getLocalFiles()`，release 行看下载区（`mkp/`，盘就是底账）—— 两套底账，行上不说谎 */
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
  /**
   * `api.getUserPresetFiles()` 的结果：**用户线**（`presets-mine/` 里那些）。
   * 云端没有它们 —— 所以它们只在这张本地表里出现，也永远不进云端表。
   */
  mine: UserPresetFile[]
  /** `api.getLocalFiles()` 的结果（同样是固定演示集合） */
  localIds: Set<string>
  /**
   * `api.getSlicerCopied()` 的结果：**已经复制到切片器 profile 目录**的 asset id。
   *
   * 和 `localIds` 是两件事 —— 在本机不等于切片器看得见它。切片器行的「生效」只看这一个。
   */
  slicerCopiedIds: Set<string>
  /** 唯一底账（新世界 `run/active-preset.json`）里那一条。**全表最多一份**，null = 一套都还没应用 */
  active: ActivePreset | null
  kind: PresetKindAxis
  query: string
  pinned: Set<string>
  /**
   * **目录（catalog）里登记的交付文件**（预设页认的那两类：MKP + 切片器）——
   * 旧世界"云端最新一次 Release"的新世界对应物：发布方写进目录，消费方从这里看全量。
   * `usePresetData` 从 `api.getRuntimeCatalog()` 取，**已在源头按 kind 分好类别**
   * （图标 / 模型不进这个数组）；空数组 = 目录里没有登记预设页的文件。
   */
  releasePresets: ReleasePresetSource[]
  /** 下载区（`mkp/`）里有、**且与目录登记一致**的那些（`ReleaseFileState = ok`） */
  localReleases: ReleasePresetSource[]
  /**
   * 下载区里有、**但与目录登记不一致**的那些（`ReleaseFileState = stale`）。
   *
   * 和 `localReleases` 分开传：本地表要按"盘上有没有"画（两者都画），
   * 云端表要按三态标（一个 ok 一个需更新）。合成一个数组的话，两处都要再拆一次。
   */
  staleReleases: ReleasePresetSource[]
  /** 目录指纹前 16 位（来源列那枚 chip 用）。null = 没读到目录 */
  releaseVersion: string | null
  /**
   * **备注覆盖账**（`api.getPresetRemarks()`）：用户改过的副标题。
   * 键 = 文件身份（官方交付行 `catalog.path` / 用户线相对用户根路径）；
   * 有 ⇒ 副标题用它，没有 ⇒ 用那一版工作台写的 `remark`。
   */
  remarks: Record<string, string>
  /**
   * **这次发布的时刻**（catalog 的 `publishedAt`，RFC3339 / UTC）—— 云端表 release 行
   * 「时间」列的来源，显示时前端转本机时区。null = 目录没带（随包 / 旧版发布），
   * 那一列照实「未知」。
   */
  releaseAt: string | null
  /**
   * **当前数据源**（设置页那一个的 mode：`gitee` / `github` / `custom`）——
   * release 行「来源」列显示 GitHub / Gitee / 自定义源 的依据。null = 没配源
   * （离线 / 随包），来源照旧退「官方」。
   */
  sourceLabel: string | null
}

/**
 * 目录里登记的一份交付预设，摊平成行要用的形状。
 *
 * 数据来自新世界两端共用契约（`api.getRuntimeCatalog()` 的 `files` 域）——
 * 大小是**发布时对产物真字节算的真值**（catalog 登记的）。时间有两样、
 * 各归各的表（2026-10-06 起，此前"目录没有可信时间源"整列都是「未知」）：
 *
 *   事件时间         **这份字节怎么到的**（下载 / 替换，事件账）—— 本地表用
 *   `releaseAt`     **这次发布的时刻**（catalog.publishedAt，发布侧盖的戳）—— 云端表用
 *
 * 都没有（随包目录没盖戳 / 认不出出身的字节）就照实「未知」，**不编**。
 */
/**
 * 交付行第二行那串小字：把人引到盘上的落点，**顺带把"盘上那份不对劲"这件事写在原地**。
 *
 * 三种话对应 [`ReleaseFileState`] 的三种"盘上有东西"：一致 / 旧版本 / 内容异常。
 * 它不改任何判定（判定全在 `state` 上）—— 只是让用户在这一行上就能看出该不该信它。
 */
function releasePathText(p: ReleasePresetSource): string {
  const at = `下载区 mkp · ${p.uid}`
  const note =
    p.state === 'old' ? '（旧版本）' : p.state === 'tampered' ? '（内容异常）' : ''
  return `${at}${note}`
}

export interface ReleasePresetSource {
  /** `A1/STANDARD` 这种。行键沿用；下载 / 应用两个动作认 `fileName` */
  uid: string
  machineId: string
  versionId: string
  fileName: string
  /**
   * 这一份归预设页的哪一类（catalog 的 `kind` 经 [`catalogKindToFileKind`] 映射）。
   *
   * 老形状里没有它 —— 于是"catalog 登记的只有预设"这个**没写下来的前提**被当成了
   * 事实：catalog 长出新种类（BBS 配置 / 图标 / 模型）之后，MKP 档把 `.json`
   * 和 `.svg` 全列了出来（2026-10-02 作者截图）。现在类别跟着数据走，两档按它分流。
   */
  kind: FileKind
  /** **catalog 登记的落点**（`delivery/mkp/presets/A1-standard.toml`）—— 备注覆盖账的键 */
  path: string
  /** 目录登记的字节数（真值） */
  size: number
  /** 它属于哪次发布（新世界目录没有版本号概念，恒 null；chip 只写「官方交付」） */
  releaseVersion: string | null
  /**
   * **事件时间**（2026-10-06 预设事件时间模型），三格各是各、互不顶替：
   *
   * - `downloadedUnix` —— 这份字节是「下载」进本机的（`DeliveryDownloaded.at`）；
   * - `replacedUnix` —— 这份字节是「替换」上去的（`DeliveryReplaced.at`，
   *   即上一次「更新」换上去的那一刻）。两个事件**至多一个有值**；
   * - `deliveryPublishedAt` —— 这份字节属于哪一代目录、那一代**在云端发布**的时刻
   *   （RFC3339）。**跟着这一版字节走**，云端以后怎么换代都不变。
   *
   * 都没有 / null = 认不出出身的字节或目录没盖戳 —— 照实「未知」，**不编**。
   * mtime 从此只归文件系统，不再上界面。
   */
  downloadedUnix: number | null
  replacedUnix: number | null
  deliveryPublishedAt: string | null
  /**
   * 盘上那一份现在是什么（见 [`ReleaseFileState`]）。
   *
   * **算它的地方只有一处**：`usePresetData.readRelease` 把三个读（目录清单 / 下载区 /
   * 认得出是哪一版吗）合成这一个字段。两张表都读它，**不许自己再判一次** ——
   * 两处各判一次，迟早有一处忘了跟（"需更新"以前就是那样把坏档说成"未下载"的）。
   */
  state: ReleaseFileState
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
 * **catalog 里的 `kind` → 预设页的行类别**。两档分流的唯一判据，不靠扩展名猜。
 *
 * catalog 的 `kind` 是发布方对**资源种类**的登记（`runtime::catalog::kind` 那四个），
 * 预设页的 `FileKind` 是"这一行显示在哪一档"的类别 —— 中间这一层两端的词表不同，
 * 映射只写在这里一处：
 *
 *   `mkp_preset`   → `mkp_preset`   MKP 配置那一档
 *   `bbs_config`   → `bbs_profile`  切片器配置那一档（BBS 是切片器的一种）
 *   `orca_config`  → `orca_profile` 同上（Orca 的配置。今天还没有这种文件，位置先留着）
 *   `icon` / `model` / 其余 → `null`
 *
 * **返回 `null` = 不归预设页**：图标是界面素材、模型是校准资源，各自由自己的资源体系
 * 消费。这里不列它们，而不是"认不出"——认不出是用户文件那一档的事
 * （`FileKind | null` 里的 `null`，两处含义不同，别混）。
 *
 * 这就是 2026-10-02 那张截图的修法：`a1.svg`（icon）与 `MKPProcess ….json`
 * （bbs_config）混进"MKP 配置 → 云端"表里 —— 因为 release 行当年默认
 * "catalog 里登记的只有预设"，一个 kind 都没看。
 */
export function catalogKindToFileKind(kind: string): FileKind | null {
  switch (kind) {
    case 'mkp_preset':
      return 'mkp_preset'
    case 'bbs_config':
      return 'bbs_profile'
    case 'orca_config':
      return 'orca_profile'
    default:
      return null
  }
}

/**
 * 台账那一格「仓库 N」：**当前类型档**在仓库里一共有几个文件（全机型）。
 *
 * 判据与两张表同一套（`matchesKind`）：MKP 档数 MKP 的，切片器档数切片器的
 * （bbs + orca）。图标 / 模型既不算进来也不出现在表里 —— 它们不归这一页。
 */
export function treeCountOfAxis(tree: PresetTree, axis: PresetKindAxis): number {
  const kinds: FileKind[] = axis === 'mkp' ? ['mkp_preset'] : ['bbs_profile', 'orca_profile']
  return kinds.reduce((n, k) => n + tree.fileCounts[k], 0)
}

/**
 * 台账那一格「我的 N」：用户自己的文件里**属于当前档**的个数。
 *
 * 判据与本地表 mine 行的过滤是同一条（`localRows` 里那 `.filter`）：**认不出类别的
 * （`.json`）两档都算** —— 它在任何档下都列在表里，计数也要跟着列，藏起来就等于
 * 对它说"你没这份文件"。
 */
export function mineCountOfAxis(mine: UserPresetFile[], axis: PresetKindAxis): number {
  return mine.filter((f) => f.kind === null || matchesKind(axis, f.kind)).length
}

/**
 * 切片器那一类的**交付行**（catalog 登记、能下载）在本地表里为什么没有操作按钮。
 *
 * 它不能被「应用」（使用中指针只认 MKP 预设）；「复制到切片器目录」那条路只认
 * 资产库里的 asset id（切片器那一侧的写还没接，见 `docs/PROJECT-AUDIT.md` ③）——
 * 原来这里画的是「复制」，点了**静静没反应**。给一个点了没反应的按钮，与
 * "点了必报错"同罪：不给。
 */
export const SLICER_RELEASE_WHY =
  '切片器配置不参与「应用」——它的生效要把它复制进切片器自己的目录，那条写动作还没接（见 PROJECT-AUDIT ③）。' +
  '这一份已经在本地了，没有可点的动作；要重新下一份干净的，到云端表那一行点「下载 / 更新」'

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

/**
 * 版本 id → **备注**（副标题的第二优先级）。
 *
 * 与 `versionNameLookup` 同一棵树、同一个道理：只认当前这一档机型目录里有的，
 * 查不到（没写 / 机型被筛掉了）返回 `undefined` —— 上层再往 `path` 回落，不编。
 */
function versionRemarkLookup(
  machines: PresetMachineNode[],
): (machineId: string, versionId: string) => string | undefined {
  const byMachine = new Map<string, Map<string, string | undefined>>()
  for (const m of machines) {
    byMachine.set(
      m.machine.id,
      new Map(m.versions.map((v) => [v.version.id, v.version.remark ?? undefined])),
    )
  }
  return (machineId, versionId) => byMachine.get(machineId)?.get(versionId)
}

/**
 * 本地表：**本机磁盘上真有的文件**。
 *
 * 两种来源拼在一起，用 `origin` 分开：
 *
 *   官方副本  当前这些机型的文件里，`getLocalFiles()` 说已经下到本机的那些
 *   我的文件  `getUserPresetFiles()` 扫出来的（**用户线**：`presets-mine/`）—— 云端没有它，
 *             所以没有交付身份、不属于任何版本、也没有 SHA 可比
 *
 * 官方那一半按**机型**汇总（一个文件一行，用到它的版本收进 `versions`）；
 * 我的那一半是**用户线**（`presets-mine/` 扫出来的）：机型这一层没有来源，
 * 所以任何机型档下都列（机上写「—」，名称列挂「未标机型」，见 `untagged`）。
 *
 * 「生效」两种类型两套判据（见 `PresetLocalRow.live`）：MKP 看唯一底账
 * （使用中指针）里那一条，切片器看已复制到切片器目录的那个集合。
 */
export function localRows(input: PresetRowsInput): PresetTableData<PresetLocalRow> {
  const {
    machines,
    machineId,
    mine: mineFiles,
    localIds,
    slicerCopiedIds,
    active,
    kind,
    query,
    pinned,
    localReleases,
    staleReleases,
    releaseVersion,
    releaseAt,
    remarks,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const versionRemark = versionRemarkLookup(machines)
  /*
   * 副标题：**账上有覆盖（包括空串）就用用户的**（2026-10-07 作者改口：
   * 「可以空着，不要回退」—— 用户写空副标题就空）；没有覆盖才走
   * 「工作台写的 → 路径文本」回落。「恢复默认」= 删掉覆盖。
   */
  const remarkOr = (key: string | null, fallback: string): string =>
    key !== null && remarks[key] !== undefined ? remarks[key] : fallback

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
        /* 官方仓库行没有备注覆盖（ remarkKey null）—— 副标题照旧路径文本 */
        subtitle: f.path,
        remarkKey: null,
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

  /*
   * **用户线**：用户自己放进 `presets-mine/` 的那些（`<appDataDir>/user/`）。
   *
   * 与官方行最本质的区别（总纲 §1③）：**云端没有它们** —— 没有交付身份、不属于任何版本、
   * 也没有 SHA 可比。所以它们只活在这一张本地表里，云端表永远看不到。
   *
   * 两条"不知道就别筛掉"的口径（藏起来等于对用户说他没这份文件）：
   *   - **认不出类别的**（`.json` 分不出 bbs 还是 orca）在任何类型档下都列；
   *   - **机型**这一层根本没有来源（今天没有地方让用户标它）→ 任何机型档下都列，
   *     机上那格写「—」、名称列上挂「未标机型」—— 这与官方那半边的 `untagged` 是同一条口径。
   */
  const mine = mineFiles
    .filter((f) => f.kind === null || matchesKind(kind, f.kind))
    .map((f): PresetLocalRow => {
      /*
       * 用户自己那份**也能被应用**（第七层）：只有认得出是 MKP 预设（`.toml`）的才行 ——
       * `.json` 那几份（bbs / orca 分不出）不是预设，应用它们无从谈起。
       * 「生效」认的是**唯一底账**里的那一条：`origin` 是用户线、而且路径就是这一条。
       */
      const canApply = f.kind === 'mkp_preset'
      const live = canApply && active?.origin === 'mine' && active.path === f.path
      /* 血统里的来源，现在对应哪台机型的哪一版（人话）。认不出就不写 */
      const source =
        f.basedOnMachineId === null || f.basedOnVersionId === null
          ? null
          : `${names.get(f.basedOnMachineId) ?? f.basedOnMachineId} · ${versionName(
              f.basedOnMachineId,
              f.basedOnVersionId,
            )}`
      /*
       * **自己的归属**（2026-10-07 作者要的：复制出来的那份也要显示机型 / 版本，
       * 而且能改）：文件头 `# machine:` / `# variant:` 两行（后端已归一化，
       * 头里没有 / 认不出回落血统那份）。副标题的版本备注也按它查。
       */
      const ownMachine = f.machineId ?? f.basedOnMachineId
      const ownVersion = f.versionId ?? f.basedOnVersionId
      return {
        /* rowKey 带上当前那一档机型：换机型时这一行要当成新的一行重画（焦点与菜单都跟着行走） */
        rowKey: `mine:${machineId}:${f.path}`,
        /* 没有 asset id，也没有别的身份 —— **路径就是它的身份**（用户随时可能改名，认路径最稳） */
        pinKey: f.path,
        fileName: f.fileName,
        path: f.path,
        /* 副标题 = 备注覆盖账里的 → 归属那一版的备注 → 路径（presets-mine/…） */
        subtitle: remarkOr(
          f.path,
          ownMachine !== null && ownVersion !== null
            ? (versionRemark(ownMachine, ownVersion) ?? f.path)
            : f.path,
        ),
        remarkKey: f.path,
        /* 认不出是哪一类就照实留 `null`：展开详情里写"认不出"，不替他认成 MKP */
        kind: f.kind,
        machineId,
        /* **归属**说它属于哪台 —— 不再是「—」；归属都没有（导入的裸文件）才写「—」 */
        machineText: ownMachine === null ? DASH_ : (names.get(ownMachine) ?? ownMachine),
        /* 归属那一版的显示名。都没有就不写，不替他猜一个 */
        versions: ownMachine !== null && ownVersion !== null ? [versionName(ownMachine, ownVersion)] : [],
        /* **真值**：盘上那份的大小与改动时刻（用户线也盘当底账）—— 与切片器那一档同一档来源 */
        sizeText: sizeTextOf(f.size),
        modifiedText:
          f.modifiedUnix === null ? undefined : new Date(f.modifiedUnix * 1000).toISOString(),
        statFrom: 'file',
        applied: live,
        pinned: pinned.has(f.path),
        scope: 'local',
        origin: 'mine',
        /* 归属都没有（导入的裸文件）才挂「未标机型」—— 有归属的那份现在自己说得出 */
        untagged: ownMachine === null,
        live,
        /* 血统：它当初基于官方哪一版、那一版现在还在不在（第七层） */
        basedOn: f.basedOn,
        basedOnSource: f.basedOnLabel,
        basedOnOfficial: source,
        /* 详情面板「改归属」读它（机型 / 版本两个下拉的数据源是行上这份归属） */
        ownMachineId: ownMachine,
        ownVersionId: ownVersion,
        /*
         * 第九层：文件级状态。**读不出来的照样列出来**（藏起来等于说他没这份文件），
         * 只是不给「应用 / 改这份」—— 与 `.json` 那份"不给必报错的按钮"同一条口径。
         * 认不出是哪一类的没有这一档（`null` → `undefined`）。
         */
        mineState: f.state ?? undefined,
        mineStateDetail: f.stateDetail,
        /* 出处账：复制自哪一份 / 是不是导入的（「来源」那一格的数据，见 `originCellOf`） */
        provenance: f.provenance ?? null,
        copiedFrom: f.copiedFrom,
        copiedFromName: f.copiedFromName,
      }
    })

  /*
   * 目录里登记的交付预设：下载之后它们就躺在下载区（`mkp/`，盘就是底账），
   * 本地说的就是「本机磁盘上真有的文件」—— 所以这一半**收盘上真有的那些**：
   * 与目录一致的、以及不一致的（旧版本 / 内容异常）。四档全在盘上，
   * 藏起后三种就等于对用户说"你机器上没有它"，而修复它的入口也就没了。
   * 「生效」认唯一底账（新世界 `run/active-preset.json`）里那一条（与官方行合流，不分两套）。
   */
  const onDisk = [...localReleases, ...staleReleases]

  const release = onDisk
    .filter((p) => matchesKind(kind, p.kind))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .map((p): PresetLocalRow => {
      const state = p.state
      /* 交付行只有 MKP 那一类能被「应用」（使用中指针认的一直是 MKP 交付文件）——
         切片器那一类的交付行没有"生效"这回事，`live` 恒 false */
      const live = p.kind === 'mkp_preset' && active !== null && active.fileName === p.fileName
      return {
        /*
         * 行键与置顶键都用 **fileName**，不用 `uid`（`机型/版本`）。
         *
         * 交付构造上每个 (机型, 版本) 只有一份产物，所以 uid 现在也唯一 —— 但那是
         * **没写下来的前提**（`CatalogFile` 里没有这条约束）。拿它当 React 键，撞了之后的
         * 症状是「另一张表里冒出一行幽灵」：实测过，同一 uid 两份文件时，切一次轴
         * 云端那一行会漏进本地表。而 `fileName` 是这套系统里**明写的**取用口径 ——
         * 下载 / 应用 / 读正文全认它（`download_runtime_file` 就是按名字在目录里找）。
         */
        rowKey: `release:${machineId}:${p.fileName}`,
        /* 仓库里没有它，没有 assetId —— 「应用」认 fileName（页面里分流） */
        pinKey: `release:${p.fileName}`,
        fileName: p.fileName,
        /* 第二行小字（`path` 仍是盘上落点，「在文件夹中显示」与 title 用它）：
           副标题 = 备注覆盖账里的 → 那一版工作台写的 → 落点文本；
           盘上那份不对劲时那件事**必须**还写在原地 —— 覆盖账压不住状态注记 */
        path: releasePathText(p),
        subtitle:
          /* 盘上那份不对劲（旧版本 / 内容异常）时那件事**必须**写在副标题原地 ——
             状态注记压过备注；一致的那份才走「覆盖账 → 工作台备注 → 落点」回落 */
          p.state === 'ok'
            ? remarkOr(p.path, versionRemark(p.machineId, p.versionId) ?? releasePathText(p))
            : releasePathText(p),
        remarkKey: p.path,
        kind: p.kind,
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        sizeText: sizeTextOf(p.size),
        /*
         * 到位时刻：这份字节是「下载」进来的还是「替换」上去的（两个事件至多一个有值，
         * 见 `ReleasePresetSource`）—— 标签跟着事件走，UI 不自己挑。epoch 秒 → ISO
         * 交 `parseStatDate` 按本机时区显示。没有事件（认不出出身的字节）就是
         * undefined（「未知」）—— 不拿 mtime 顶（硬规则③）。
         */
        modifiedText:
          (p.downloadedUnix ?? p.replacedUnix) === null
            ? undefined
            : new Date((p.downloadedUnix ?? p.replacedUnix)! * 1000).toISOString(),
        arrivalBy:
          p.downloadedUnix !== null
            ? ('downloaded' as const)
            : p.replacedUnix !== null
              ? ('replaced' as const)
              : null,
        ownPublishedAt: p.deliveryPublishedAt,
        applied: live,
        pinned: pinned.has(`release:${p.fileName}`),
        scope: 'local',
        origin: 'release',
        untagged: false,
        releaseUid: p.uid,
        releaseVersion,
        releaseState: state,
        /* 发布时刻：展开详情「云端更新」一格（本地行的下载时间在 `modifiedText` 上） */
        publishedAt: releaseAt,
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
    releaseVersion,
    releaseAt,
    remarks,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const versionRemark = versionRemarkLookup(machines)
  /* 与本地表同一条回落 —— 同一份覆盖账，两张表不各说一套（空覆盖也是覆盖，不回退） */
  const remarkOr = (key: string | null, fallback: string): string =>
    key !== null && remarks[key] !== undefined ? remarks[key] : fallback

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
        subtitle: f.path,
        remarkKey: null,
        kind: f.kind,
        machineId: e.machineId,
        machineText: names.get(e.machineId) ?? e.machineId,
        versions: e.versions,
        nozzle: f.nozzle,
        layerHeight: f.layerHeight,
        sizeText: f.sizeText,
        modifiedText: f.modifiedText,
        statFrom: f.statFrom,
        /* 官方行没有「正在使用」这一说 —— 使用中指针只认 MKP 交付文件，剩下的官方行全是切片器档 */
        applied: false,
        pinned: pinned.has(pinKey),
        scope: 'cloud',
        origin: 'official',
        delivery: f.delivery,
        downloaded: f.id !== undefined && localIds.has(f.id),
      }
    })

  /*
   * 目录里登记的交付文件（catalog 的 files 域）—— 云端表上它们那几行。
   * 与官方文件同列一个「来源」chip 区分；「已下载」看下载区（`mkp/`，盘就是底账），
   * 「正在生效」看唯一底账（`run/active-preset.json`）。**按 `kind` 分流**：
   * MKP 档只列 MKP 预设、切片器档只列 BBS / Orca 的配置 —— 不靠扩展名猜；
   * 图标 / 模型在数据层就被 `catalogKindToFileKind` 挡掉了（不归这一页）。
   */
  const release = releasePresets
    .filter((p) => matchesKind(kind, p.kind))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .map((p): PresetCloudRow => {
      const live = p.kind === 'mkp_preset' && active !== null && active.fileName === p.fileName
      /* 四档全在源上算好了（见 `ReleasePresetSource.state`）—— 这里只搬，不判 */
      const state = p.state
      return {
        /* 行键 / 置顶键都认 fileName —— 理由见 `localRows` 里那一段（别拿 uid 当键） */
        rowKey: `release-cloud:${p.fileName}`,
        pinKey: `release:${p.fileName}`,
        fileName: p.fileName,
        path: `官方交付 / ${p.machineId} / ${p.versionId}`,
        /* 副标题同样走备注（与本地表同一份覆盖账；盘上不对劲的状态只在本地表说） */
        subtitle: remarkOr(
          p.path,
          versionRemark(p.machineId, p.versionId) ?? `官方交付 / ${p.machineId} / ${p.versionId}`,
        ),
        remarkKey: p.path,
        kind: p.kind,
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        sizeText: sizeTextOf(p.size),
        /*
         * 云端表的时间 = **这次发布的时刻**（`releaseAt`，发布侧盖进目录的戳）——
         * 同一次发布的每一行是同一个值；目录没带（随包 / 旧版发布）就是 undefined
         * （「未知」+ `RELEASE_TIME_WHY.cloudMissing` 那句话）。
         */
        modifiedText: releaseAt ?? undefined,
        applied: live,
        pinned: pinned.has(`release:${p.fileName}`),
        scope: 'cloud',
        origin: 'release',
        releaseUid: p.uid,
        releaseVersion,
        delivery: 'default',
        /* `downloaded` 仍问"与目录一致的那一份在不在本机"（切片器那一档也用它） */
        downloaded: state === 'ok',
        releaseState: state,
        publishedAt: releaseAt,
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
