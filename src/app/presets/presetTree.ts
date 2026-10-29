/*
 * 预设页的数据形状与判定规则。
 *
 * 这个文件里**只有类型、常量和纯函数** —— 没有 state、没有 JSX、没有 api 调用。
 * 页面、卡片、行、首页都读它：「待下载」在一处写成「待下载」、另一处写成「云端」，
 * 界面就开始撒谎。
 *
 * # 轴与两张表（2026-10-09：位置那一轴回来了）
 *
 *   机型          → 页面顶部一个下拉（`PresetPicker`），含「全部机型」一档
 *   MKP / 切片器  → 一条分段控件（`PresetScopeBar` 左半），文件类型
 *   本地 / 云端   → 同一条分段控件（右半），**两种类型都有**
 *
 * 两张表各答一个用户真会问的问题，由 `localRows()` / `cloudRows()` 各自算，**不合流**：
 *
 * ```text
 *   本地表  我这台机器上真有的东西 —— 没有网也看得见、改得动、用得上
 *   云端表  云端现在提供什么 —— 我这边有没有、要不要拿
 * ```
 *
 * **MKP 档的两张表各是什么**（2026-10-09 改判，此前短暂合成过一张"资源库"表）：
 *
 *   本地  只有 `presets-local/` 里那几行（用户的本地文件）：能改、能使用、删了就没了。
 *         云端原件（下载区那份字节）与基准快照都是**内部数据**，不在这张表里出现
 *   云端  目录里登记的每一份预设一行。行上按 [`CloudLocalState`] 说"我这边怎么样"
 *         （未获取 / 本地已有 / 有新版），动作是「下载」/「更新」—— 都是把云端的取到本机，
 *         **不动「当前使用」**（使用只发生在本地表那几行上）
 *
 * **切片器档照旧**，它那两张表说的是另一件事：
 *
 *   本地表  本机有哪些文件。两种来源：云端下载下来的副本（`getLocalFiles()`）
 *           与用户自己放进预设目录的（`getLocalPresetFiles()` —— 本地文件，**云端没有它们**）
 *   云端表  菜单上有哪些云端文件（已分配 + 可选）。行尾标「已下载 / 未下载」，四态照旧
 *
 * **版本从筛选器降成了表里的一列。** 原因：筛选器的宽度随选项个数线性增长 ——
 * 6 个版本就摆不下，实测那一行还只画出了 3 个，剩下的直接看不见。而版本是
 * 「这个文件的属性」，机型才是「我这台机器关我什么事」的边界。所以行按**机型**汇总，
 * 一个文件一行，用到它的版本名收在 `versions: string[]` 里由表格画成一列（多个用 `·` 连，
 * **不写 `+N`** —— 作者原话「显示什么版本加 2 什么意思？」，那是省宽度省出来的黑话）。
 * 这一列不可点、不排序、不分组 —— 做成可点的就等于把筛选器换个位置放回来。
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
 * 为什么（云端那一档拿不到真值时）。**本地文件那一份是真值** —— 后端扫盘给的大小与时刻
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
  BasedOnFacts,
  FileKind,
  FileRef,
  FilesState,
  Machine,
  MachineVersion,
  MenuEntry,
  LocalState,
  CloudVersion,
  PresetFileInfo,
  LocalPresetFile,
  LocalPresetIdentity,
  VersionFiles,
} from '../../api'
import type { StringKey, Tr } from '../../i18n'

/**
 * 用户自己的一份文件（本地文件，`presets-local/`）。
 *
 * 名字直接用契约那一个，**不抄字段**：契约改了这里跟着改。这里**转出去**是因为
 * 这一页的输入形状（`PresetRowsInput`）也把它写在签名上 —— 调用方不必再去 `src/api` 取一次。
 */
export type { LocalPresetFile }

/** 取不到真值时的统一说法，与工作台那边用的是同一个词 */
export const UNKNOWN: StringKey = 'presets.tree.unknown'

/** 大小那一栏的 title。为什么是「未知」得说清，否则看的人只会觉得界面没做完 */

/** 没有这个值的那一种：用户自己放进预设目录的文件。**不替它编一个** */

/** 类型角标。MKP = 涂胶预设 toml，BBS = 切片器工艺 profile */

/**
 * 两条分段控件的词表。
 *
 * 左边那条是**文件类型**（`PresetFileInfo.kind`），和工作台「分配菜单」那一步
 * 的 MKP 段 / BBS 段说的是同一件事，所以用词要对得上。
 * 右边那条是**位置**，而位置不是筛选条件 —— 它切的是两张互不相干的表（见文件头）。
 */
export type PresetKindAxis = 'mkp' | 'slicer'
export type PresetScopeAxis = 'local' | 'cloud'

export const KIND_AXIS_TEXT: Record<PresetKindAxis, StringKey> = {
  mkp: 'presets.tree.kind.mkp',
  slicer: 'presets.tree.kind.slicer',
}

export const SCOPE_AXIS_TEXT: Record<PresetScopeAxis, StringKey> = {
  local: 'presets.tree.scope.local',
  cloud: 'presets.tree.scope.cloud',
}

/**
 * 本地表「状态」那一列的词。**两种类型两套词，因为「生效」是两件不同的事**：
 *
 *   MKP     生效 = 设为当前配置（`applyPreset`）      在用 / 未使用
 *   切片器   生效 = 复制到切片器目录（`copyToSlicer`） 已复制 / 未复制
 *
 * 曾经两种共用 `已应用 / 本地有` 那一套，等于说「把 profile 下到本机就生效了」——
 * 不成立：切片器只读它自己那个 profile 目录。
 *
 * ★ MKP 那一套 2026-10-10 从「已应用 / 未应用」改成「在用 / 未使用」（作者：
 *   「显示的是已应用，可文案却改了，改成使用没事，先把它都统一一下」）——
 *   动作词是那颗按钮上的「使用」，状态词就得跟它同族。旧词与按钮各说各的，
 *   同一屏上「已应用」和「使用」并排出现，读的人要自己判断这俩是不是一回事。
 *   切片器那一套（已复制 / 未复制）不动：它的动作本来就是「复制」。
 */
export const LIVE_TEXT: Record<PresetKindAxis, { on: StringKey; off: StringKey }> = {
  mkp: { on: 'presets.tree.live.mkp.on', off: 'presets.tree.live.mkp.off' },
  slicer: { on: 'presets.tree.live.slicer.on', off: 'presets.tree.live.slicer.off' },
}

/** 操作列上那个按钮的字。动作用**文字**不用图标 */
export const ACTION_TEXT: Record<PresetKindAxis, StringKey> = {
  mkp: 'presets.tree.action.mkp',
  slicer: 'presets.tree.action.slicer',
}


/**
 * 旧版本抽屉里那两个时间的标签与「未知」的写法。**两个时间各是各，永不互相顶替**：
 *
 *   云端发布  这一版在云端发布时的时刻 —— 跟着这一版字节走，反查版本出身；
 *             早于版本记忆的照实「未知（早于版本记忆）」，不拿"现在"顶
 *   替换时间  它被换下来那一刻（替换事件）—— 你动它的时刻，如实说，但只当下要信息
 */
export const ARCHIVE_TIME = {
  published: 'presets.tree.archiveTime.published',
  replaced: 'presets.tree.archiveTime.replaced',
  publishedUnknown: 'presets.tree.archiveTime.publishedUnknown',
  replacedUnknown: 'presets.tree.archiveTime.replacedUnknown',
} as const satisfies Record<string, StringKey>

/**
 * 来源：这个文件是云端的、用户自己的，还是**目录登记的交付预设**。
 *
 * 权限也挂在这一列上：云端副本不能改名（复制一份再改），用户自己的文件删了没有任何地方能找回来。
 */
export type PresetOrigin = 'asset' | 'local' | 'preset'

export const ORIGIN_TEXT: Record<PresetOrigin, StringKey> = {
  asset: 'presets.tree.origin.asset',
  local: 'presets.tree.origin.local',
  /*
   * ★ `preset` 在界面上**也叫「云端」**（2026-10-14 定的）—— 别再给它造词。
   *
   * 走过一段弯路：那一刀（术语改造）把 `official` 机械替换成 `cloud`，于是原本的
   * 「官方交付」变成「云端交付」，跟 `asset` 那一档的「云端」撞在一起 —— 作者当天就问
   * 「云端交付是什么」。当时的处置是改名「发布预设」，但作者看着真界面又说对了：
   * **「现在的发布预设和云端其实区别不大」**。
   *
   * 想清楚的那一层是：这两类**在界面上从不并排** —— `preset` 是「MKP 配置」（toml，
   * 目录登记的交付预设）、`asset` 是「切片器配置」（json，云端仓库里的文件副本），
   * 分属两个类型档，同一张表里只会出现其中一类。既然不并排，就没有"两个词挨着分不清"
   * 这回事，硬造「发布预设」只是让用户多学一个他看不出差别的新概念。
   *
   * 用户问"这份从哪来"，本来就有更准的一处答：来源列显示**数据源名**
   *（`Gitee` / `GitHub` / `自定义源`，见 [`sourceTextOf`]）。而这两类真正不同的地方
   *（受不受目录登记、有没有版本指纹与换代）是**程序内部怎么管**，写在悬停说明里就够了。
   *
   * ★ 名字为什么是这两个词（2026-10-14 作者定）：分类值只回答**这是哪一类业务对象**，
   * 不回答"从哪来"（三类都从云端来）—— `preset` = 云端发布的 MKP 预设 TOML、
   * `asset` = 云端提供的资源文件（切片器配置 / BBS JSON / 图标 / 模型）。
   */
  preset: 'presets.tree.origin.preset',
}

export const ORIGIN_WHY: Record<PresetOrigin, StringKey> = {
  asset: 'presets.tree.originWhy.asset',
  local: 'presets.tree.originWhy.local',
  preset: 'presets.tree.originWhy.preset',
}

/**
 * 数据源（设置页那一个）→ 来源列上**交付预设**那一行显示的字。
 *
 * 用户问的是「这文件从哪来」—— 答案是 **Gitee / GitHub / 自定义源**，不是一串目录指纹
 * （那串曾经过出现在来源 chip 上：`云端 · 08e55a3f…`，作者原话「我觉得这个没有意义」。
 * 指纹唯一还有意义的位置是展开详情里的「云端版本」—— 那是版本的身份证，不是来源）。
 *
 * 没配源（离线 / 随包）就退回「云端」—— 说不出是从哪一个源下的，
 * 但"它是从云端来的"这件事照样说得准。
 */
export function sourceTextOf(tr: Tr, sourceLabel: string | null): string {
  switch (sourceLabel) {
    case 'gitee':
      return 'Gitee'
    case 'github':
      return 'GitHub'
    case 'custom':
      return tr('presets.tree.source.custom')
    default:
      return tr(ORIGIN_TEXT.preset)
  }
}

/**
 * 来源列 / 展开详情「来源」一格的统一出口。
 *
 * 三种来源三种说法：
 *   云端      云端副本（仓库文件下到本机的那份）
 *   preset    交付预设：从**当前数据源**的目录下载的（`GitHub` / `Gitee` / `自定义源`）
 *   local      本地：再分三档 ——
 *             `复制自 <文件名>`（出处账记的用户文件复制，**可点击定位**到那一份）
 *             `复制自 <文件名>`（云端另存出来的，血统说的；定位到云端那一份云端）
 *             `导入`（出处账记的导入档）
 *             都没有 ⇒ 「本地」（手工放进目录的）
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
  tr: Tr,
  row: PresetTableRow,
  sourceLabel: string | null,
): { text: string; title: string; locate?: LocateTarget } {
  /*
   * **正向判别 `=== 'local'`**，不是两个反向 `!==`：PresetCloudRow 的 origin 本身是
   * 联合（'asset' | 'preset'），TS 的反向排除对"判别值是联合的成员"不彻底
   * （成员删不掉，后面的字段访问照样报错）。正向判别一次到位 —— 剩下的必是 local 行。
   */
  if (row.origin === 'local') {
    /* 出处账优先（用户文件 → 用户文件、导入），其次血统（云端另存出来的），再退「本地」 */
    if (row.provenance === 'copy' && row.copiedFromName) {
      return {
        text: tr('presets.tree.origin.copyFrom', { name: row.copiedFromName }),
        title: tr('presets.tree.origin.copyFromTitle', { name: row.copiedFromName }),
        locate: {
          scope: 'local',
          kind: row.kind === null ? null : 'mkp',
          /* 定位永远按「全部机型」那一档算行键（local 行的键里带机型筛选） */
          rowKey: `local::${row.copiedFrom}`,
          fileName: row.copiedFromName,
        },
      }
    }
    if (row.provenance === 'import') {
      return {
        text: tr('presets.tree.origin.imported'),
        title: tr('presets.tree.origin.importedTitle'),
      }
    }
    if (row.basedOnSource) {
      const sourceName = row.basedOnSource.split('/').pop() ?? row.basedOnSource
      return {
        text: tr('presets.tree.origin.copyFrom', { name: sourceName }),
        title: tr('presets.tree.origin.basedOnTitle', { basedOn: basedOnCellText(tr, row) }),
        locate: {
          scope: 'cloud',
          kind: 'mkp',
          rowKey: `preset-cloud:${sourceName}`,
          fileName: sourceName,
        },
      }
    }
    return { text: tr(ORIGIN_TEXT.local), title: tr(ORIGIN_WHY.local) }
  }
  if (row.origin === 'preset') {
    const source = sourceTextOf(tr, sourceLabel)
    return {
      text: source,
      title: tr('presets.tree.origin.presetTitle', {
        source,
        version: row.presetVersion ?? tr(UNKNOWN),
      }),
    }
  }
  return { text: tr(ORIGIN_TEXT.asset), title: tr(ORIGIN_WHY.asset) }
}

/** 云端表那一列：这个云端文件在不在本机。判据与本地表是同一个 `getLocalFiles()` */
export const CLOUD_STATE_TEXT = {
  downloaded: 'presets.tree.cloudState.downloaded',
  pending: 'presets.tree.cloudState.pending',
} as const satisfies Record<string, StringKey>

/*
 * ★ `VERSION_NEW_TEXT` / `VERSION_NEW_WHY` / `VERSION_TIME_WHY` 三条词随
 * 「逐版各占一行」一起退役了（2026-10-08 当天下午）：它们只服务**版本行**
 * （那枚「新版本」胶囊、以及"这一版自己的发布日 vs 目录代时间"的分别）。
 * 云端表回到「一个目录条目一行」之后，一行的状态只有那四档
 * （`UPDATE_STATE_TEXT` + `PRESET_STATE_WHY`），时间只有目录盖的戳 ——
 * 这三条词就没有一行会用到了。留着它们就是留一条没人走的路。
 */

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
 * 三档不一致的（old / tampered）**共有一条边界**：不许使用、不许改、不许复制，
 * 只能重新下载一份干净的 —— 见 [`isSuspectPreset`]。
 */
export type PresetFileState = 'missing' | 'ok' | 'old' | 'tampered'

/** 内容存疑的那两档（旧版本 / 内容异常）—— 它们共用同一条边界 */
export function isSuspectPreset(state: PresetFileState | undefined): boolean {
  return state === 'old' || state === 'tampered'
}

/**
 * **云端 vs 盘上**的三态 —— preset 行「状态」与操作列的主词都从它出。
 *
 * 三个概念**不是** `PresetFileState`：那是"盘上那份认不认得出"（信任），
 * 这一个是"云端有没有比盘上新的货"（更新）。**云端有更新时主词就是「有更新」** ——
 * 不管盘上那份是认得出的旧版还是认不出的字节，用户要做的事是同一件：
 * 点「下载」换一份当前版回来（同一条下载管道）。
 *
 *   `missing`  未下载：目录里有、盘上没有
 *   `latest`   已下载：盘上字节与目录一致
 *   `update`   有更新：盘上有字节、但与目录登记的当前版不一致（旧版或认不出的都算）
 */
export type PresetUpdateState = 'missing' | 'latest' | 'update'

export function updateStateOf(state: PresetFileState): PresetUpdateState {
  switch (state) {
    case 'missing':
      return 'missing'
    case 'ok':
      return 'latest'
    default:
      return 'update'
  }
}

export const UPDATE_STATE_TEXT: Record<PresetUpdateState, StringKey> = {
  missing: 'presets.tree.updateState.missing',
  latest: 'presets.tree.updateState.latest',
  update: 'presets.tree.updateState.update',
}

/**
 * 「有更新」底下那行**认不出**的注记（只有 `tampered` 那一档给）。
 *
 * 2026-10-06 查明的实情：客户端的版本记忆只留得住"每一代被换下的目录"（归档链），
 * 在链建起来之前换过的版本谁都不记得 —— 所以"认不出"≠"被改过"，把「内容异常」顶在
 * 状态主词上是**吓唬人**（实测 7 份全部是正规旧版）。主词改说「有更新」，
 * 这句话作为第二行把实情讲全，动作给「下载」—— 对"旧版"和"真被动过"都是正解。
 */
export const PRESET_UNTRUSTED_NOTE: StringKey = 'presets.tree.untrustedNote'

/**
 * preset 行操作列那颗按钮：三态各一个字。
 *
 * ★ **「有更新」那一档叫「更新」**（2026-10-20 作者改判：「你怎么把它做成了下载按钮啊？
 *   我要更新按钮」）。2026-10-08 那天定的是"动作只有一个、就叫下载"，但用户看的是
 *   **状态**：明明写着「有新版」，按钮却叫「下载」，第一眼像是在重新下载一份。
 *   两档的词分开：
 *     `missing`（本机没有）→「下载」  第一次取回来
 *     `update` （本机是旧的）→「更新」换掉旧的
 *   **管道仍然只有一条**（`runtime/delivery.rs`：旧份自动归档），只是按钮说实话。
 */
/**
 * 云端那一行"我这边怎么样"那枚徽章怎么写（2026-10-27：**与本地行同一条规则**）。
 *
 * 作者原话：「两边都统一呀，不要有差异，一样的判断，一样的显示，云端旧的就不用显示更新」。
 * 所以这里不再自己判，直接搬认领者那一份的 `BasedOn`：
 *   newer     → 「云端较旧」（云端比我这份旧，不给动作）
 *   elsewhere → 「另一个源」
 *   其余照旧（current → 本地已有 / outdated → 有新版 / 没认领 → 未获取）
 */
export function cloudClaimBadge(row: {
  basedOn?: BasedOn
  localState?: CloudLocalState
}): { text: StringKey; state: string } | null {
  /*
   * 徽章只搬 [`cloudRowState`] 的结论（2026-10-29 收口）——
   * 以前这里自己写了一遍同样意思的 switch，详情头又直接读 `CLOUD_LOCAL_TEXT`，
   * 三处各自判一次，于是永远有一处没跟上。
   */
  const st = cloudRowState(row)
  if (st.text === null) return null
  return { text: st.text, state: row.localState ?? st.action }
}

export const UPDATE_ACTION_TEXT: Record<PresetUpdateState, StringKey> = {
  missing: 'presets.tree.updateAction.missing',
  latest: 'presets.tree.updateAction.latest',
  update: 'presets.tree.updateAction.update',
}

/**
 * 云端那一行「我这边怎么样」—— **MKP 档的判据**（2026-10-09）。
 *
 * # 它为什么不看下载区
 *
 * MKP 档里，云端原件（下载区那份字节）是**内部数据**：它是隐藏基准的来源，
 * 用户既看不到也改不了它。所以"盘上那份对不对"那四态（未下载 / 已下载 /
 * 旧版本 / 内容异常）在这一档**没有主语** —— 用户真正关心的是：
 *
 *   我有没有一份**基于它**的本地文件（`presets-local/` 里那个能改能用的）
 *   云端有没有换过版（换了 = 我那份的默认值来源旧了，有「有新版」）
 *
 * 判据只认**来源关系**（文件头血统里记的 `based_on`），不认文件名 ——
 * 导入的同名文件不会被误认成"本地已有"。
 *
 * **「有新版」的判据**（2026-10-10 改口）：只看**我那份跟不跟得上云端当前版** ——
 * 跟不上的就给「更新」（更新 = 把云端新版逐项处理到本地这份上，见 `PagePresets`
 * 的 `runUpdate`：没冲突的按新版采用、冲突的让用户选，确认后才写本地文件）。
 * 旧口径看的是"下载区字节取回来没有"，那会让用户根本点不到「更新」。
 *
 * 切片器档照旧用 `presetState`（那边的「本机」是真事实：profile 得躺在
 * 切片器自己的目录里才算数）。
 */
export type CloudLocalState = 'none' | 'has' | 'stale'

/** 状态那一列的词（MKP 云端行） */
export const CLOUD_LOCAL_TEXT: Record<CloudLocalState, StringKey> = {
  none: 'presets.tree.cloudLocal.none',
  has: 'presets.tree.cloudLocal.has',
  stale: 'presets.tree.cloudLocal.stale',
}

export const CLOUD_LOCAL_WHY: Record<CloudLocalState, StringKey> = {
  none: 'presets.tree.cloudLocalWhy.none',
  has: 'presets.tree.cloudLocalWhy.has',
  stale: 'presets.tree.cloudLocalWhy.stale',
}

/**
 * 云端行操作列那颗按钮的字。`null` = 这一档不给按钮 ——
 * 「本地已有」那一行点了没有意义（要改要用都在本地那张表上），
 * 摆一颗看起来能点、点了什么也不发生的按钮比不摆糟。
 */
export const CLOUD_LOCAL_ACTION: Record<CloudLocalState, StringKey | null> = {
  none: 'presets.tree.cloudLocalAction.none',
  has: null,
  stale: 'presets.tree.cloudLocalAction.stale',
}

/**
 * **云端那一行的唯一结论**（2026-10-29 收口）。
 *
 * # 为什么要有它
 *
 * 作者一天里连撞三次"界面自相矛盾"：徽章写「有新版」、详情头写「有新版」、按钮却是破折号；
 * 点了「更新」提示"已经是最新"，行上却还挂着「更新」。根因不是哪一处画错，而是**同一件事
 * 有三套并行话术**，各自算各自的：
 *
 * | 谁 | 原来读什么 | 缺什么 |
 * |---|---|---|
 * | 行首徽章 `cloudClaimBadge` | `localState`（3 档） | 「说不清 / 内容没变」都并进 `stale` |
 * | 详情头那枚 | `CLOUD_LOCAL_TEXT[localState]` 直接取 | 同上，且**绕过了**徽章那层 |
 * | 操作列 `CloudLocalAction` | `basedOn` + `localState` 两套混用 | 两条路各判一次 |
 *
 * 现在三者都从这里取：**结论（文案 + 动作 + 是否报警）只在这个函数里产生一次**，
 * 组件只负责画。以后再加一档判定，改这里一处即可。
 *
 * # `localState` 还剩什么用
 *
 * 只剩一个真问题：**我这儿有没有一份基于它的文件**（`none` = 没有 ⇒ 那是「下载」不是
 * 「更新」）。其余全部交给 `basedOn`（Rust 侧七档，唯一真值）。
 */
export type CloudRowAction = 'download' | 'update' | 'none'

export interface CloudRowState {
  /** 徽章 / 详情头那句话。`null` = 什么都不写（`newer` / `elsewhere` 故意不吭声） */
  text: StringKey | null
  /** 这一格给什么动作 */
  action: CloudRowAction
  /** 要不要用警告样式（目录自相矛盾是一句报警，不是一个动作） */
  warn: boolean
  /** 动作按钮的悬停说明（只有「说不清」那一档需要多解释一句） */
  hint: StringKey | null
}

/** 一句话定案。**任何要显示"云端这一行怎么样"的地方都必须走它。** */
export function cloudRowState(row: {
  basedOn?: BasedOn
  localState?: CloudLocalState
}): CloudRowState {
  const none = { text: null, action: 'none', warn: false, hint: null } as const
  if (row.localState === undefined) return { ...none }

  /* 我这儿一份都没有 ⇒ 「下载」。与 basedOn 无关：没有那份文件就谈不上基于哪一版。 */
  if (row.localState === 'none') {
    return {
      text: CLOUD_LOCAL_TEXT.none,
      action: 'download',
      warn: false,
      hint: null,
    }
  }

  switch (row.basedOn) {
    case 'current':
      return { text: CLOUD_LOCAL_TEXT.has, action: 'none', warn: false, hint: null }
    case 'outdated':
      return {
        text: CLOUD_LOCAL_TEXT.stale,
        action: 'update',
        warn: false,
        hint: null,
      }
    /*
     * **发布前进了，但这一份没变**：不说「有新版」，也不给动作 ——
     * 它确实没有需要更新的内容。
     */
    case 'unchanged':
      return {
        text: 'presets.tree.cloudLocal.unchanged',
        action: 'none',
        warn: false,
        hint: null,
      }
    /*
     * **说不清版本关系**（血统缺发布线 / 老目录没身份）：不声称有新版，
     * 但**必须留一条出路** —— 否则老副本永远没有建立新基线的入口。
     * 点它 = 以当前云端版建立新基线，`hint` 里把这句话说全。
     */
    case 'unverified':
      return {
        text: 'presets.tree.cloudLocal.unverified',
        action: 'update',
        warn: false,
        hint: 'presets.table.unverified.updateTitle',
      }
    /* **目录自相矛盾**：报警，不给动作（覆盖等于把错的东西写进用户文件） */
    case 'inconsistent':
      return {
        text: 'presets.tree.cloudLocal.inconsistent',
        action: 'none',
        warn: true,
        hint: null,
      }
    /* 云端更旧 / 不是同一条线：作者要「就当无事发生」—— 一个字都不写、不给动作 */
    case 'newer':
    case 'elsewhere':
      return { ...none }
    /*
     * 后端没给结论（老后端 / 探针数据）：退回 `localState` 那三档。
     * 这是**唯一的兜底**，也是唯一还会出现"说不清却写有新版"的地方 —— 收口在这儿，
     * 一眼能看出它什么时候会被用到。
     */
    default:
      return {
        text: CLOUD_LOCAL_TEXT[row.localState],
        action: row.localState === 'stale' ? 'update' : 'none',
        warn: false,
        hint: null,
      }
  }
}

/** 内容存疑那两档共有的那条边界（右键菜单禁用 / 详情里那句话都用它） */
export const PRESET_SUSPECT_WHY: StringKey = 'presets.tree.suspectWhy'

/** 「下载」那颗按钮在**盘上那份已经旧了**那一档的说明：它不是"删除重下"，旧份进归档，删除永远不是这件事的一部分 */

/**
 * **「编辑文件」**那条链的话（行菜单 / 展开详情 → 编辑器抽屉）。
 *
 * 2026-10-08 资源库改判起，编辑的对象只有**「我的预设」**（`presets-local/` 里那份；
 * 云端那一份是模板、只读 —— 要改它先在资源库里「另存为本地预设」）。
 * 初稿 = **盘上那份文件的字节**；编辑期间只动**正文草稿**，保存 = **写回它自己**
 * （同一个文件，不会多出一份；写回是整份原子替换）：
 *
 * ```text
 * presets-local/A1-standard.toml ──改这份──▶ 正文草稿 ──保存──▶ 写回它自己
 * ```
 *
 * ★ 2026-10-10（作者）：**参数页那份未保存的改动一个字都不进来** ——
 * 两条草稿分开存（契约 `PresetTextDraft`），编辑器看到的就是文件本身。
 *
 * 于是不会出现 `（已修改）2.toml` / `（再次修改）.toml` 这种越改越多的名字；
 * 它基于哪一版云端写在文件里（那三行），保存时**原样保留** —— 改的是参数，不是出处。
 * **不做**：改完之后"生效 / 应用"（那是"用户那份也能不能用"那条产品规则的活）。
 */
export const EDIT_TEXT = {
  /*
   * 菜单项与详情里那颗按钮。2026-10-10 作者改判：原来叫「修改」，
   * 与旁边那颗「打开参数」（改的是**参数表**）分不清 —— 这一条改的是**文件正文**
   * （TOML），所以改叫「编辑文件」；弹窗标题跟着用同一个词。
   */
  cell: 'presets.edit.cell',
  open: 'presets.edit.open',
  title: 'presets.edit.title',
  /*
   * 2026-10-10 第二轮（作者：「它上面什么什么那个布局啊，什么的我都不满意」）：
   * 正文上方那一大段说明**从界面上撤了** —— 它是实现细节（临时文件 / 草稿 / 出处），
   * 不该占着编辑器的头。
   *
   * ★ 2026-10-10 第三轮：底栏左侧**只在出错时说话**。原来还有两句 ——
   *   「改动自动存草稿」与「接着上次改 —— 草稿已经在盘上」（作者圈着后一句：
   *   「那这个文案是什么意思？去掉它」）。它们都是**过程说明**：
   *   "草稿""盘上"是内部的词，用户既不知道指什么、也不需要被通知
   *   （要保存，右边那颗按钮就是）。留下的只有异常那一句（`draftFailed`）。
   *
   * ★ 2026-10-16：编辑器改成**两态**（与参数对比 / 修改参数弹窗同一套框架）——
   *   打开先只读（`enterMode` 那颗进编辑模式），进编辑模式后底栏是
   *   撤销 / 重做 / `exitMode` / `commit`。原来那颗「放弃」退休 ——
   *   **「退出编辑模式」就是作废**（作者 2026-10-16：「没有保存的话，退出编辑模式就是作废，
   *   那些改的都作废，保存了才有」）：丢草稿、内容回滚到盘上，回只读。
   *   想保留改动只有一条路 —— **保存**；直接关模态框则改动留在草稿里，下次打开
   *   直接进编辑模式并给一条"还没保存"的通知（见 `PagePresets` 的 `openEdit`）。
   */
  /** 只读态右下那颗：进编辑模式（三个模态框同一句话） */
  enterMode: 'presets.edit.enterMode',
  /*
   * 编辑态底栏那颗 = **作废这一次没保存的改动**、回只读。
   *
   * ★ 2026-10-16 作者（第二版）：「我希望这个退出编辑模式的文案可以改一下，就比如说改成
   *   一种『放弃更改并退出编辑模式』，但是这文案好像有点长」⇒ 定 **放弃并退出**。
   *   十个字会把底栏那颗字按钮挤得比「保存」还宽（一颗次要动作压过主按钮），
   *   而"放弃"两个字必须写在脸上 —— 他连着三条反馈说的都是同一件事：
   *   **用户不知道"退出编辑模式"就是作废**（所以才有通知上那颗「放弃修改」）。
   *   剩下那半句在 `exitTitle` 里说全。
   *
   * ★ 2026-10-21 作者又改回来：**「放弃」**。
   *   「我点那个放弃并退出的时候改一下吧，改成放弃，不要退出。因为我在外面那里编辑的时候，
   *    点这个按钮都没有退出模态框」（那颗按钮退的是**编辑模式**，不是模态框 ——
   *    按钮上写"退出"，用户就以为要关窗）。作废那半句仍留在 `exitTitle` 里说。
   */
  exitMode: 'presets.edit.exitMode',
  /** 上面那颗的 tooltip —— 三个模态框**同一句**（说的是同一件事） */
  exitTitle: 'presets.edit.exitTitle',
  /*
   * ★ 2026-10-16（作者：「我希望这个时候那个通知……他可以给我就是选择那个放弃保存的那种……
   *   因为我发现这个界面用户不知道退出编辑模式就是取消上一次的修改」）：
   * 「有未保存的改动」那条通知上多一颗**放弃修改** —— 它就是底栏那颗 `exitMode`
   * 的另一个入口（模态框已经关掉时就只是丢草稿）。
   * 叫「放弃修改」而不是「放弃保存」：**压根没有"尚未保存的保存动作"这回事**，
   * 作废的是这次改动；而且这个词与底栏那颗的说明是同一件（见 `PagePresets.notifyUnsaved`）。
   */
  discard: 'presets.edit.discard',
  commit: 'presets.edit.commit',
  /** 草稿自动落盘失败时那一行（安静地写在底栏里，不用提示条）。`{detail}` = 后端给的原因 */
  draftFailed: 'presets.edit.draftFailed',
  /** 写回之后那句（`path` 说清写回哪去了） */
  savedBack: 'presets.edit.savedBack',
} as const satisfies Record<string, StringKey>

/**
 * **文件头那几行不给改**（2026-10-16 作者：「这上面这一些灰色的，有没有可能说不让用户更改
 * 呀？」）：`# release_time` / `# machine` / `# variant` / 段落标签 / `# based_on*` 血统三行
 * —— 它们不是"用户的正文"，是程序写、程序读的事实（归属 + 血统）。
 * 分界在 `tomlPaint.ts` 的 `splitHeader`；拦下那一刻说一句就走（`PagePresets` 的
 * `notifyHeadLocked`）。
 */
export const HEAD_LOCKED = {
  title: 'presets.edit.headLocked.title',
  text: 'presets.edit.headLocked.text',
} as const satisfies Record<string, StringKey>

/** 「编辑」那颗按钮的说明 */

/**
 * **撞名那一格**（a04）：云端那份要落下来时，本机**已经有一份同名的**了。
 *
 * 与资源管理器"保留两者"同一条路子：**不覆盖、不静默** —— 问用户；
 * 同意就**自动改一个没被占的名字**（`A1-fast-2.toml`），两份都留着，
 * 原来那份一个字节都不动（口径见 `lab/preset-identity-and-update-spec.md` §6½）。
 */
export const IMPORT_TAKEN_TEXT = {
  title: 'presets.edit.taken.title',
  /** `name` = 占着名字的那份、`suggested` = 导入这份会自动改成什么 */
  note: 'presets.edit.taken.note',
  commit: 'presets.edit.taken.commit',
} as const satisfies Record<string, StringKey>

/**
 * **第十层**：重命名那一口框的话（删除的二次确认在菜单里，话术在菜单项上）。
 *
 * 只改名、不动字节：内容 / 那三行血统 / TOML 都不重写 —— 改完还是同一份预设。
 * 正在使用的那一份也能改名（使用中指针跟着走，不断）；有草稿的那一份也能改名
 * （草稿跟着走，「接着上次改」不接丢）。删除另说 —— 那两道闸在菜单与后端。
 *
 * ★ 2026-10-10 又一轮（作者：「只改名字，这些文案不需要」「他正在使用，也不需要」）：
 *   框里那两段说明（`note` 与「正在使用」那句）**整个撤了**；
 *   后缀也从输入框里拿出来 —— 它是**灰色的一枚贴在行尾、点不动**（见 `splitFileName`），
 *   用户只打主名，提交时原样拼回去。
 */
export const LOCAL_RENAME = {
  title: 'presets.edit.rename.title',
  commit: 'presets.edit.rename.commit',
} as const satisfies Record<string, StringKey>

/**
 * 拆出**主名与后缀**（`A1 涂胶.toml` → `A1 涂胶` + `.toml`）。
 *
 * 重命名框（后缀不可改，贴在输入行尾）与复制起名共用这一套拆法：
 * `lastIndexOf('.')` 且点不在开头（`> 0`）—— `.bashrc` 那种前导点不算后缀，
 * `Process_0.2mm.json` 取到的还是 `.json`。
 */
export function splitFileName(fileName: string): { stem: string; ext: string } {
  const dot = fileName.lastIndexOf('.')
  return dot > 0
    ? { stem: fileName.slice(0, dot), ext: fileName.slice(dot) }
    : { stem: fileName, ext: '' }
}

/** 「恢复默认名」四个字（菜单项与重命名框里那颗按钮共用一处，别各写一个） */
export const RESTORE_DEFAULT_NAME: StringKey = 'presets.edit.restoreDefaultName'

/**
 * 「修改参数」那个词（**全站用语**，2026-10-16 作者：别再动它）。
 *
 * 2026-10-20 起展开详情「快捷操作」里那颗也从「编辑」换成它（原来那颗打开的是
 * **改 TOML 正文**的「编辑文件」）—— 两处（右键菜单 / 快捷操作）共用这一份字，
 * 不许各写一个；改正文那一档仍叫「编辑文件」（[`EDIT_TEXT`]）。
 */
export const PARAMS_TEXT = {
  open: 'presets.edit.params.open',
  why: 'presets.edit.params.why',
} as const satisfies Record<string, StringKey>

/**
 * **这一行跟云端没关系**（2026-10-20）：复制 / 导入出来的，而设置页那一格
 * 「复制出来的也跟着云端更新」关着 ⇒ 不挂「有新版」、不给「更新」入口。
 *
 * 判据只有一个来源：后端那一格（`copiesFollow`）+ 出处账给出的 `provenance`。
 * 行上那几处（名称列徽章 / 操作列 / 详情快捷操作 / 右键菜单）**都读它**，
 * 不许各写一遍 —— 写两遍就会漂成"徽章不挂、菜单还让点"那种样子。
 */
export function isDetachedCopy(row: PresetTableRow, copiesFollow: boolean): boolean {
  if (copiesFollow || row.scope !== 'local') return false
  return row.provenance === 'copy' || row.provenance === 'import'
}

/**
 * **这一份的「默认名」**（2026-10-20 作者：「我希望他的这个名字可以恢复默认的名字」）。
 *
 * 判据只有一条 —— **它当初从哪来**，出处账那一头记着的文件名：
 *
 *   复制件（`provenance === 'copy'`）     ⇒ 来源那一份现在的文件名
 *                                          （`A1-fast copy 2.toml` → `A1-fast.toml`）
 *   导入的（`provenance === 'import'`）   ⇒ 没有默认名（`null`：它本来就不叫这个名字）
 *   从云端另存 / 下载出来的（血统指云端） ⇒ 云端那一份的文件名（`A1-standard.toml`）
 *
 * 三样都没有就 `null` —— **不猜**（拿当前名字当默认名是自欺：那等于"恢复"什么都不做）。
 *
 * 后缀跟着**来路**走：复制件的后缀与来源一致；血统那一档取血统路径的尾段。
 * 名字撞了怎么办由后端那道「不覆盖」闸回答（这里不替它改名）。
 */
export function defaultNameOf(row: PresetTableRow): string | null {
  if (row.origin !== 'local') return null
  if (row.provenance === 'import') return null
  if (row.provenance === 'copy') {
    return row.copiedFromName !== null && row.copiedFromName !== undefined
      ? row.copiedFromName
      : null
  }
  const source = row.basedOnSource
  if (source === null || source === undefined || source === '') return null
  const tail = source.split('/').pop()
  return tail !== undefined && tail !== '' ? tail : null
}

/**
 * **恢复默认名要落到哪个名字**：默认名没被占用就用它；**占了就按复制那一套依次让开**
 * （`A1-standard.toml` → `A1-standard 2.toml`…）。
 *
 * 为什么不报错让用户自己改：后端那道闸是"不覆盖"，用户点「恢复默认名」要的是
 * 「把它改回正常名字」这件事本身 —— 撞名时停下来问"要不要叫别的"是把活儿推回给他。
 * ★ 不叫 `copy`：它跟本来的默认名是一回事，只是重了一份（`copy` 这个词留给「复制」）。
 */
export function defaultNameAvoiding(defaultName: string, taken: ReadonlyArray<string>): string {
  const used = new Set(taken)
  if (!used.has(defaultName)) return defaultName
  const { stem, ext } = splitFileName(defaultName)
  for (let n = 2; n < 1000; n += 1) {
    const candidate = `${stem} ${n}${ext}`
    if (!used.has(candidate)) return candidate
  }
  return `${stem} ${Date.now()}${ext}`
}

/**
 * **第十一层**：复制出一份新的（2026-10-10 作者改判 —— 菜单里就叫「复制」）。
 *
 * 本地文件 → 本地文件（第十一层的边界）：**按字节复制**、那三行 `# based_on` 血统
 * **原样带过去**（不重算 —— 来源已经是用户文件）；原文件一个字节不动。
 *
 * ★ 名字**由程序自己起**（见 `duplicateNameOf`）：作者 2026-10-10 把原来那套
 *   「名字由用户明确指定」撤了 —— 「主要是它的文案太长了，我觉得要不然就叫复制，
 *   就自动复制出一份」。点一下就得一份，不再弹框问名字。
 * **一个状态都不碰**：不改「已应用」、草稿也留在原来那份上。
 */
export const LOCAL_COPY = {
  /** 菜单项上那两个字（作者点名的短文案） */
  cell: 'presets.edit.localCopy.cell',
  /** 副本名里那个词：`<原名> copy.toml`。**文件名片段，不进字典**（换语言不改文件名） */
  suffix: 'copy',
} as const satisfies { cell: StringKey; suffix: string }

/**
 * 复制出来的那一份叫什么：`<原名> copy.toml`；这个名字已经被占了就 `copy 2` / `copy 3`……
 *
 * 后缀保持原样（`.toml` 还是 `.toml`）—— 后端那套名字门槛要求一致。
 * `taken` 传当前本地文件里已有的文件名（页面手里就有），所以**不用猜、不会撞**。
 */
/**
 * 这一份是**谁的第几份副本**（2026-10-16 作者：「复制出的文件我希望放在原本的下面」）。
 *
 * # 为什么要它
 *
 * 副本叫 `<原名> copy.toml`（`duplicateNameOf`）。按文件名排时第 8 个字符是
 * `' '`(32) vs `'.'`(46) —— **空格比点小 ⇒ 副本会排到原件上面去**。
 * 用户要的是"复制出来的那一份就躺在原件下面"，所以排序不能只认整串文件名。
 *
 * # 怎么认
 *
 * 按 `duplicateNameOf` 起的名**从尾巴上一层一层**反解：`<base> copy<ext>` ⇒ 第 1 份副本；
 * `<base> copy 2<ext>` ⇒ 第 2 份；原件本身 ⇒ 空链。
 * ⇒ 先比 `base`（原件名），base 相同再比那条链 —— 于是
 *   `A1-fast.toml` < `A1-fast copy.toml` < `A1-fast copy 2.toml`，**三份挨在一起、原件在最前**。
 *
 * ★★ **剥到只剩原件名为止**（2026-10-20）。只剥一层是不够的：
 *   `A1-fastv3.3 copy copy.toml` 那一层剥完得到 base = `A1-fastv3.3 copy.toml`，
 *   与原件 `A1-fastv3.3.toml` **不是同一族** ⇒ 退化成整串文件名比较 ⇒
 *   副本跑到原件上面去（作者图里那一条）。副本的副本也是副本，得一直剥下去。
 *
 * # 那条链的方向：靠近原件的在**前**（外 → 内）
 *
 *   `A1 copy.toml`         ⇒ base `A1`  链 `[1]`
 *   `A1 copy 2.toml`       ⇒ base `A1`  链 `[2]`
 *   `A1 copy copy.toml`    ⇒ base `A1`  链 `[1, 1]`
 *   `A1 copy 2 copy.toml`  ⇒ base `A1`  链 `[2, 1]`（`copy 2` 的那一份副本）
 *
 * 剥是从右往左的，所以剥完要**倒过来** —— 倒过来之后按字典序比（短的在前），
 * 从某一层复制出来的那一份就**紧跟在它下面**（`[2]` 后面才是 `[2, 1]`）。
 *
 * ★ 认不出的（用户自己改名把 ` copy` 改掉了、或本来就叫这个名字）⇒ 空链、
 *   `base` = 整个文件名 —— 退化成"按文件名排"，不会把谁排丢。
 */
export interface CopyRank {
  /** 去掉**所有**副本后缀之后的主名（同一族的名字都一样） */
  base: string
  /**
   * 副本链，**靠近原件的在前**。空 = 原件。
   *
   * `[1]` = 原件的第 1 份副本；`[2, 1]` = 第 2 份副本再复制出来的那一份。
   */
  chain: number[]
}

export function copyRankOf(fileName: string): CopyRank {
  const { stem, ext } = splitFileName(fileName)
  /* ★ `base` 一律**带后缀**：原件与它那几份副本必须落到**同一个** base 上才归得了一族；
     带上后缀也顺带把 `A1-fast.toml` 与 `A1-fast.json` 分成两族（后缀保持原样是硬规则）。 */
  const chain: number[] = []
  let base = stem
  for (;;) {
    /* 只认**行尾**那个 ` copy` / ` copy 2` —— 名字里本来就有的空格照旧是名字的一部分
       （`A1 涂胶.toml` 不能被误判成 `A1` 的副本） */
    const m = /^(.*?) copy(?: (\d+))?$/.exec(base)
    if (m === null) break
    const n = m[2] === undefined ? 1 : Number.parseInt(m[2], 10)
    /* 数字解析不出来（`copy abc`）就当它是名字的一部分，不是副本 ——
       已经剥掉的几层一起退回去，整串当名字（不许拿半个链去排序） */
    if (!Number.isFinite(n)) return { base: fileName, chain: [] }
    chain.push(n)
    base = m[1]
  }
  if (chain.length === 0) return { base: fileName, chain }
  /* 剥是从右往左的（先剥最里层）⇒ 倒过来，让靠近原件的那一层排在前面 */
  chain.reverse()
  return { base: `${base}${ext}`, chain }
}

export function duplicateNameOf(fileName: string, taken: ReadonlyArray<string>): string {
  const { stem: base, ext } = splitFileName(fileName)
  const used = new Set(taken)
  const first = `${base} ${LOCAL_COPY.suffix}${ext}`
  if (!used.has(first)) return first
  for (let n = 2; n < 1000; n += 1) {
    const candidate = `${base} ${LOCAL_COPY.suffix} ${n}${ext}`
    if (!used.has(candidate)) return candidate
  }
  /* 兜底（正常到不了）：名字里带时间戳，保证不会和已有的重名 */
  return `${base} ${LOCAL_COPY.suffix} ${Date.now()}${ext}`
}

/** 字节数写成人话（`4.2 KB`）。**一处** —— 表里的「大小」与归档抽屉里都用它 */
export function sizeTextOf(size: number): string {
  return size >= 1024 ? `${(size / 1024).toFixed(1)} KB` : `${size} B`
}

export const ARCHIVE_WHY: StringKey = 'presets.tree.archiveWhy'

/** 那一格上的字：几份 + 可以点开 */
export function archiveOpenText(tr: Tr, count: number): string {
  return tr('presets.tree.archiveOpen', { count })
}

/**
 * 本地文件那一份的抽屉（点「看正文」看的那个）。
 *
 * 它和归档那个抽屉共用外壳，但说的是另一条线：**用户自己的文件**。
 * 这一层只做「看得见、认得出、看得了」—— **改它 / 新建它 / 保存是下一层**，
 * 所以抽屉里没有任何写入按钮（连"另存为"都没有：那是第 5 步）。
 */
export const LOCAL_DRAWER = {
  title: 'presets.tree.localDrawer.title',
  /** 展开详情里那一格的名字与按钮 */
  cell: 'presets.tree.localDrawer.cell',
  open: 'presets.tree.localDrawer.open',
  reading: 'presets.tree.localDrawer.reading',
  note: 'presets.tree.localDrawer.note',
} as const satisfies Record<string, StringKey>

/** 「看正文」那颗按钮的说明（展开详情里的 title） */

/** 展开详情里那一格的名字 */
export const BASED_ON_KEY: StringKey = 'presets.tree.basedOnKey'

/** 展开详情里「后处理」那一格（本机学习账：这一份后处理成功跑过几次） */
export const USAGE_KEY: StringKey = 'presets.tree.usageKey'

/**
 * 「后处理」那一格写什么。
 *
 * 计数点在后处理**成功**那一趟（`runtime::usage`；机型不符、中途失败都不加），
 * 所以它答的是"真跑成过几次"，不是"点过几次使用"。0 次 = 还没跑成过 ——
 * 写「还没跑过」而不是「0 次」/「—」：前者像计数，后者像"不知道"，这句是事实。
 */
export function usageCellText(tr: Tr, count: number | undefined): string {
  return count === undefined || count <= 0
    ? tr('presets.tree.usage.none')
    : tr('presets.tree.usage.times', { count })
}

/** 那一格的 tooltip：把"怎么算一次"当场说清，省得用户拿它跟「使用」次数对不上 */
export const USAGE_HINT: StringKey = 'presets.tree.usageHint'

/**
 * 展开详情里「发布线」那一行写什么（2026-10-28）：`support-ease · 第 12 号`。
 *
 * **两格同生同死**（后端 `Catalog::parse` 拒收只有一格的目录），所以缺任何一个
 * 就返回 `null` —— 调用方据此**整行不渲染**。这里刻意不返回「未知」：
 * 老目录 / 老副本本来就没有这条信息，摆一行"未知"只会让人以为界面没做完。
 *
 * 本地行传血统的（我这份基于第几号），云端行传目录的（云端是第几号）。
 */
export function releaseLineText(
  tr: Tr,
  line: string | null | undefined,
  serial: number | null | undefined,
): string | null {
  if (line == null || line === '' || serial == null) return null
  return tr('presets.table.detail.ver.lineValue', { id: line, serial })
}

/** 「发布线」那一行的键名（一条句子只有一个出处，组件只搬） */
export const RELEASE_LINE_KEY: StringKey = 'presets.table.detail.ver.line'

/**
 * 展开详情里「基于」那一格写什么。**文案收在这一处**（组件只搬）——
 * 与 `versionsText` / `presetBatchText` 同一条规矩：一句话只有一个出处。
 */
export function basedOnCellText(
  tr: Tr,
  row: {
  basedOn?: BasedOn
    basedOnSource?: string | null
    basedOnCloud?: string | null
  },
): string {
  /* 说得清来源的名字就说它（`A1 · 标准版`），说不清就退回血统里那串原文 */
  const source = row.basedOnCloud ?? row.basedOnSource ?? ''
  switch (row.basedOn) {
    case 'current':
      return source === ''
        ? tr('presets.tree.basedOn.currentNoSource')
        : tr('presets.tree.basedOn.current', { source })
    case 'outdated':
      return source === ''
        ? tr('presets.tree.basedOn.outdatedNoSource')
        : tr('presets.tree.basedOn.outdated', { source })
    /*
     * 同一个源、但云端那份更旧（2026-10-27）：**不说"有新版"** —— 那会把人往回带。
     * 这句话只说事实："当前云端比我这份旧"，动作一个都不给（见 `PresetTable`）。
     */
    case 'newer':
      return source === ''
        ? tr('presets.tree.basedOn.newerNoSource')
        : tr('presets.tree.basedOn.newer', { source })
    /*
     * 不是同一个源：换平台 / 换分支 / 换目录，或老副本没记源身份。
     * 比不出新旧 —— 照实说"另有所在"，动作是「下载」而不是「更新」。
     */
    /*
     * 不是题目里的那个源、时间又比不出来（2026-10-27 起很少见）：
     * 只把血统里那串来源照抄出来，**不再加"不是当前这个云的"这种话** ——
     * 作者的原话是"不要显示另一个源"，那就连解释都不扯源。
     */
    case 'elsewhere':
      return source === ''
        ? tr('presets.tree.basedOn.noneNoSource')
        : tr('presets.tree.basedOn.elsewhereBare', { source })
    /*
     * **发布前进了，但这一份没变**（2026-10-28）：别的预设可能有新版，这份没有。
     * 说事实、不给动作 —— 不许因为它跟"这条线发过新版"沾边就说它有新版。
     */
    case 'unchanged':
      return source === ''
        ? tr('presets.tree.basedOn.unchangedNoSource')
        : tr('presets.tree.basedOn.unchanged', { source })
    /*
     * **说不清版本关系**（老副本缺发布线 / 老目录没身份）：照实说不知道，
     * 不冒充"已是最新"，也不冒充"有新版"。补基线要等用户明确「更新」那一趟。
     */
    case 'unverified':
      return source === ''
        ? tr('presets.tree.basedOn.unverifiedNoSource')
        : tr('presets.tree.basedOn.unverified', { source })
    /*
     * **目录自相矛盾**：同一条发布线的同一个序号，字节却变了 —— 那是数据坏了，
     * 不是"有新版"。这句话要说得像报警，因为它就是。
     */
    case 'inconsistent':
      return tr('presets.tree.basedOn.inconsistent', { source })
    default:
      return source === ''
        ? tr('presets.tree.basedOn.noneNoSource')
        : tr('presets.tree.basedOn.none', { source })
  }
}

/**
 * 用户那份**不能被应用**时的说明：认不出它是 MKP 预设。
 *
 * `.json` 那几份（bbs / orca 光看扩展名分不出）不是预设 —— 给一个点了必被拒的按钮，
 * 比不给糟（与 NO_ASSET_WHY 同一条口径）。
 */
export const LOCAL_NOT_PRESET_WHY: StringKey = 'presets.tree.localNotPresetWhy'

/**
 * **第九层**：读不出来的那一份（行上那一枚角标）。
 *
 * 与交付预设文件的「内容存疑」是**两回事**：那一档要 SHA（与目录登记逐字节一致才可信），
 * 本地文件只看**文件级**（能读 + UTF-8 + TOML 语法）—— 外部改过一轮但仍是能读的 TOML
 * 照常能用，**不因 SHA 报警**。"是不是一份合法 MKP Preset"（结构 / 参数）不在客户端判：
 * 那要真正的 Preset 解析能力，留给"应用 / 编辑"这类真正要解析的入口。
 */
export const LOCAL_UNREADABLE_TEXT: StringKey = 'presets.tree.localUnreadable'

/** 那一枚角标 / 灰掉的动作格的 title。`detail` 是后端给的人话原因（比如语法错在第几行） */
export function localUnreadableWhy(tr: Tr, detail: string | null | undefined): string {
  const text = tr(LOCAL_UNREADABLE_TEXT)
  return detail == null
    ? tr('presets.tree.localUnreadableWhy', { text })
    : tr('presets.tree.localUnreadableWhyDetail', { text, detail })
}

/** 归档抽屉里的那几句话 */
export const ARCHIVE_DRAWER = {
  title: 'presets.tree.archiveDrawer.title',
  /** 每条上那颗按钮 */
  open: 'presets.tree.archiveDrawer.open',
  reading: 'presets.tree.archiveDrawer.reading',
  bodyTitle: 'presets.tree.archiveDrawer.bodyTitle',
  /** 认不出机型版本时那一格写什么 */
  unknown: 'presets.tree.archiveDrawer.unknown',
} as const satisfies Record<string, StringKey>

/**
 * 批量那一行的字：**这一批里有什么，决定它是"下载"还是"更新"**。
 *
 * 计数由调用方从行上的 `presetState` 数出来，这里只负责措辞 —— 批量不自己判状态，
 * 它是"多份单文件操作的组合"，判据还是那一个。三档分开说（未下载 / 需更新 / 内容异常），
 * 因为用户要做的事不一样：前两种是补上云端那份，后一种是**盘上那份我们不认**。
 * `已下载的不进这一批`：这一层只解决"多份一起处理"，不解决"再下一遍已经对了的东西"。
 */
export function presetBatchText(
  tr: Tr,
  missing: number,
  stale: number,
  tampered: number,
): { count: string; label: string; why: string } {
  const total = missing + stale + tampered
  const label =
    missing === 0
      ? tr('presets.tree.batch.label.update', { count: total })
      : stale + tampered === 0
        ? tr('presets.tree.batch.label.download', { count: total })
        : tr('presets.tree.batch.label.both', { count: total })
  const parts = [
    missing > 0 ? tr('presets.tree.batch.count.missing', { count: missing }) : '',
    stale > 0 ? tr('presets.tree.batch.count.stale', { count: stale }) : '',
    tampered > 0 ? tr('presets.tree.batch.count.tampered', { count: tampered }) : '',
  ].filter((s) => s !== '')
  return {
    count: parts.join(' · '),
    label,
    why: tr('presets.tree.batch.why'),
  }
}

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

export const STATUS_TEXT: Record<PresetStatus, StringKey> = {
  applied: 'presets.tree.status.applied',
  ready: 'presets.tree.status.ready',
  missing: 'presets.tree.status.missing',
  unavailable: 'presets.tree.status.unavailable',
}

/** 「暂不支持」的正式说法。灰色、中性，和网络失败与「0 个文件」都要分得开 */
export const UNSUPPORTED_TEXT: StringKey = 'presets.tree.unsupported'

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
  tr: Tr,
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
          vf === null
            ? [tr('presets.tree.noCombo', { at: atOf(machine.id, version.id) })]
            : vf.missing,
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
 * 云端 MKP 行在树上没有 asset id（MKP 不进资产库，见 doc §12.5），它的「正在使用」
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
 * 参数写成结构类型而不是 `PresetFileNode`，是因为用户自己的文件（`LocalPresetFile`）
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
export function versionsText(tr: Tr, versions: string[]): { text: string; why: string } {
  if (versions.length === 0) {
    return { text: DASH_, why: tr('presets.tree.versions.noneWhy') }
  }
  const text = versions.join(' · ')
  return {
    text,
    why:
      versions.length === 1
        ? tr('presets.tree.versions.oneWhy', { text })
        : tr('presets.tree.versions.manyWhy', { count: versions.length, text }),
  }
}

// ——————————————————————————————————————————————————————————————
// 两张互不相干的表
//
// 本地表 = 本机磁盘上有什么（云端副本 + 用户自己的文件）
// 云端表 = 菜单上有什么云端文件（已分配 + 可选），行尾标已下载 / 未下载
//
// 一个云端文件下载之后两张表里都有 —— 两张表各回答一个问题，不合流（见文件头）。
// ——————————————————————————————————————————————————————————————

/** 两张表的行共有的部分。表格那 7 列全靠它 —— 组件不再回头查树 */
export interface PresetRowBase {
  /** 「版本来源」那几格（云端行只有后两格：源身份 + 目录发布时间） */
  basedOnFacts?: BasedOnFacts
  /** 我本地那一份**基于的云端版本时间**（血统 release_time；只有云端行有） */
  localReleaseText?: string | null

  /** 这张表里唯一。带上机型：**一个文件在一台机型下只有一行**（版本收进了 `versions`） */
  rowKey: string
  /**
   * 预设仓库里的 asset id。用户自己的文件与仓库里查不到的文件**没有**这个字段。
   *
   * 云端行不再是「使用」的对象（使用中指针只认 MKP 交付文件）—— 字段留给右键菜单与筛选用，
   * 所以它必须在行上（preset 行走 uid，见 `presetUid`）。
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
   * 本机学习账里这一份的钥匙（`local:<相对用户根>` / `cloud:<相对内部根>`，见
   * `runtime::usage`）—— 详情面板那一格「后处理 N 次」读它。
   *
   * 为什么在行上算、不在界面里拿 `path` 现拼：两张表的 `path` 口径不同（本地表是
   * 相对用户根，云端表那几行是「云端 / 机型 / 版本」这种展示文案）。钥匙必须从**真正
   * 的落点**来 —— 哪儿知道路径，哪儿给钥匙。
   *
   * **没有这个字段 = 这一份不进这本账**：后处理只跑 MKP 预设（切片器配置不跑）。
   */
  usageKey?: string
  /**
   * **表格第二行的小字（副标题）= 备注**（2026-10-07 作者定：副标题不要位置文案）。
   *
   * 两张表两类来源、**互不干扰**（2026-10-07 三轮定案）：
   *   本地表  覆盖账（用户改过的）→ 那一版工作台写的 `remark` → **空着**
   *   云端表  **只**读那一版工作台写的 `remark` → **空着**（不读覆盖账、没有编辑格）
   * 都没有备注就空着（位置文案不上副标题；`path` 永远在 `title` 里，不丢）。
   * 唯一例外：本地交付行盘上不对劲（old / tampered）时状态注记压过备注（见 `localRows`）。
   */
  subtitle: string
  /**
   * 备注覆盖账的键（`null` = 这一行没有可改的备注）。**只有本地表的行有它**：
   * 本地交付行 = `catalog.path`、本地文件 = 相对用户根的路径；云端仓库行没有备注来源，
   * 云端表的行一律不给 —— 云端只读工作台那句（2026-10-07 作者定）。
   * 详情面板的备注编辑格只在它非 `null` 时出现。
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
  /** 上面那两格是哪来的。`undefined` = 这个来源没有这两格（自己的文件、preset 行走别的口径） */
  statFrom?: 'file' | 'demo'
  /** 唯一底账（使用中指针）说正在使用的就是这一份。**全页最多一行** */
  applied: boolean
  pinned: boolean
  /**
   * **在置顶序列里的位置**（`-1` = 这一份没置顶）。
   *
   * ★ 2026-10-16 作者：「后面点的置顶的应该置顶，多个置顶的按照先后顺序，后点击的置顶」
   *   ⇒ 光有 `pinned` 那个布尔不够 —— 它答不了"这几个置顶的谁在前"。
   *   存的是**置顶的先后**（`0` = 最早点的），排序时**倒过来读**（下标大的在前），
   *   于是最近一次点的排最上面。见 `usePresetData` 的 `PINNED_KEY` 与下面 `sortRows`。
   */
  pinOrder: number
  /**
   * **这一份有未保存的参数改动**（参数页改了值、还没点保存）。行上那枚小圆点读它；
   * 只有本地表的「本地文件」行可能为真（`input.draftPath` 对上路径的那些）。
   */
  unsaved: boolean
  /**
   * 目录登记的交付预设。uid 形如 `A1/STANDARD`（行键沿用）；「下载」与「使用」
   * 两个动作认 **fileName**（目录登记的文件名 = 使用中指针的口径），
   */
  presetUid?: string
  /** 它属于哪一次发布（包版本）。来源列那枚 chip 的动态那一截读它 */
  presetVersion?: string | null
  /**
   * 交付预设在**本机**的三态（见 `PresetFileState`）。**只有 preset 行有它** ——
   * 其余来源没有"从目录下载"这一回事。`undefined` = 这一行不是交付预设。
   */
  presetState?: PresetFileState
  /**
   * **这次发布的时刻**（catalog.publishedAt，ISO/UTC → 显示按本机时区）。
   * preset 行展开详情里「云端最新版发布于」一格的数据 —— **本地行也要有**：
   * "我盘上这份什么时候到的"（`modifiedText` + `arrivalBy`）与"云端最新版什么时候发的"
   * 是两个版本各自的时间，展开详情要同时给（云端表的 `modifiedText` 就是它，列上已经显示）。
   * null = 目录没盖戳（随包 / 旧版发布），照实「未知」。
   */
  publishedAt?: string | null
  /**
   * 这份字节是怎么**到位**的（2026-10-06 事件时间模型）：`downloaded` = 下载进来
   * （`DeliveryDownloaded`）、`replaced` = 替换上去（`DeliveryReplaced`）、null = 没有事件
   * （认不出出身的字节）。详情面板「下载时间 / 替换时间」的标签跟着它走 ——
   * **标签是事件的名字，不是 UI 自己挑的**。只有本地 preset 行有它。
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
  /** 云端副本才有交付身份。用户自己的文件与目录交付预设没有它，所以**没有**这个字段 */
  delivery?: PresetFileInfo['delivery']
  /**
   * 用到这个文件的版本 **id**（与 `versions` 显示名并排）。
   *
   * 「使用」这一步要把它写进底账（首页反填要的就是机型 id + 版本 id）——
   * 只有云端行有它：preset 行走 uid、用户自己的文件不属于任何版本。
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
   *           （**用户的与云端的共用这一条**：指针说 `origin` + 认 `path` / `fileName`）
   *   切片器   `getSlicerCopied().includes(assetId)`   → 已复制 / 未复制
   *
   * 原来这里是四档 `status`，但本地表实际只有「生效 / 没生效」两档，而那四档里的
   * `ready`（本地有）对切片器是句废话 —— 在本机不等于切片器看得见它。
   */
  live: boolean
  /**
   * **用户自己那份**基于云端哪一版（只有 `origin === 'local'` 的行有它）。
   *
   * 它是"文件头血统 + 目录现况"比出来的一个事实，不是"这份文件好不好"——
   * `outdated` 的那份照常能用能改，见 [`BASED_ON_WHY`]。
   */
  /**
   * 上面的结论**摆事实**的那几格（源身份 / 各自发布时刻；云端行只有后两格）。
   *
   * 展开详情里「版本来源」那几行原样搬它 —— 判定全在 Rust 侧，前端不重算，
   * 也不自己解释"谁更新"（那会变成第二套判据）。
   */
  basedOnFacts?: BasedOnFacts
  basedOn?: BasedOn
  /**
   * 血统里记的来源原文（`delivery/mkp/presets/A1-standard.toml`）。没有血统是 `null`。
   * **老文件里可能记着旧形状（`mkp/presets/…`）** —— 那种认不出就是 `unknown`，
   * 不回填、不猜（文件本身照旧能读能改能应用）。
   */
  basedOnSource?: string | null
  /** 来源那份现在对应哪台机型的哪一版（人话，已按名字查好）。认不出是 `null` */
  basedOnCloud?: string | null
  /**
   * **第九层**：这一份用户文件的**文件级**状态（只有 `local` 行有它）。
   *
   * `'unreadable'`（读不出来：编码 / TOML 语法 / 指向用户根之外）→ 名称列画
   * 「文件无法读取」，**「使用」与「改这份」都不给**（不给必被后端拒的按钮）；
   * `'ok'` 照常。认不出是哪一类的行没有这一档（`undefined`）。
   */
  localState?: LocalState
  /** 读不出来时后端给的那句人话原因（角标与动作格的 title 用它） */
  localStateDetail?: string | null
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
   * **这一份自己的归属**（只有 `local` 行有它）：文件头 `# machine:` / `# variant:`
   * 两行（认不出回落来源那份）。行上的 `machineId` 是**筛选档**（这一行挂在哪个
   * 机型分栏下），归属是**文件自己的** —— 详情面板的「改归属」读这两个。
   */
  ownMachineId?: string | null
  ownVersionId?: string | null
}

export interface PresetCloudRow extends PresetRowBase {
  /** 认领它的那份本地文件的判据（与本地行同一个 BasedOn；没认领 ⇒ undefined） */
  basedOn?: BasedOn


  scope: 'cloud'
  /**
   * 云端表上的东西：云端仓库的文件（切片器档），或者**目录登记的交付预设**（MKP 档）。
   * 用户自己的文件云端根本没有。
   */
  origin: 'asset' | 'preset'
  delivery: PresetFileInfo['delivery']
  /** 在不在本机。云端文件看 `getLocalFiles()`，preset 行看下载区（`mkp/`，盘就是底账）—— 两套底账，行上不说谎 */
  downloaded: boolean
  /**
   * **下过的旧版有几代**（版本账里 `downloaded && !current` 的条数）。
   *
   * 2026-10-08：云端表回到「一个目录条目一行」之后，那几代旧版的**唯一**痕迹就是它 ——
   * 旧版本身以用户自己的名字躺在本地表里（`presets-local/`，名字带日期），
   * 展开这一行还能在「旧版本」那一格看到具体是谁。`0` / `undefined` = 没有。
   *
   * **MKP 档不给它**（2026-10-09）：那一档的"下过没有"按 [`CloudLocalState`] 说
   * （我有没有一份基于它的本地文件），下载区里的旧份是内部数据，不进用户世界。
   */
  olderVersions?: number
  /**
   * 「我这边怎么样」—— **只有 MKP 档有它**（见 [`CloudLocalState`]）。
   * 切片器档不出这个字段：那一档照旧按 `presetState` 四态说话。
   */
  localState?: CloudLocalState
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
   * `api.getLocalPresetFiles()` 的结果：**本地文件**（`presets-local/` 里那些）。
   * 云端没有它们 —— 所以它们只在这张本地表里出现，也永远不进云端表。
   */
  local: LocalPresetFile[]
  /**
   * **身份账本**（`api.getLocalPresetIndex()`，a04 新增）：哪几份本地文件跟云端有关系。
   *
   * 云端行那三态（未获取 / 本地已有 / 有新版）的"认亲"**只看它** ——
   * **不看文件头那三行血统**：血统复制也会照抄（副本会跟着一起认领云端条目），
   * 而账本条目的**只有"从云端下载"建**的一条（复制 / 拖回 / 导入 / 恢复都建不了）。
   */
  localIndex: LocalPresetIdentity[]
  /**
   * **有未保存参数改动的那一份**（相对用户根的路径；`null` = 现在没有）——
   * 行上那枚「未保存」小圆点按它挂（只有本地表的「本地文件」行可能命中）。
   * 来源是 `api.getPresetDraftSummary()`，见 `usePresetData` 的 `draftPath`。
   */
  draftPath: string | null
  /** `api.getLocalFiles()` 的结果（同样是固定演示集合） */
  localIds: Set<string>
  /**
   * `api.getSlicerCopied()` 的结果：**已经复制到切片器 profile 目录**的**文件名**
   * （2026-10-09 起，与下载 / 应用 / 读正文同一个取用口径）。
   *
   * 和 `localIds` 是两件事 —— 在本机不等于切片器看得见它。切片器行的「生效」只看这一个。
   */
  slicerCopiedIds: Set<string>
  /** 唯一底账（新世界 `run/active-preset.json`）里那一条。**全表最多一份**，null = 一套都还没应用 */
  active: ActivePreset | null
  kind: PresetKindAxis
  query: string
  /**
   * 置顶的**先后顺序**（一串 `pinKey`，`[0]` = 最早置顶的）。★ 不是集合 —— 顺序有意义，
   * 多个置顶之间谁在前由它决定（作者 2026-10-16）。见 `usePresetData` 的 `PINNED_KEY`。
   */
  pinned: string[]
  /** 表头那一列排的（作者 2026-10-16：每列都能点）。置顶永远压在最上面，不归它管 */
  sort: PresetSort
  /**
   * **目录（catalog）里登记的交付文件**（预设页认的那两类：MKP + 切片器）——
   * 旧世界"云端最新一次发布"的新世界对应物：发布方写进目录，消费方从这里看全量。
   * `usePresetData` 从 `api.getRuntimeCatalog()` 取，**已在源头按 kind 分好类别**
   * （图标 / 模型不进这个数组）；空数组 = 目录里没有登记预设页的文件。
   */
  deliveryPresets: DeliveryPresetSource[]
  /**
   * **云端版本账**（`api.getCloudVersions()`，2026-10-08）：交付预设登记过哪几版、
   * 哪几版这一机已经下过。云端表 MKP 档按它把每个预设摊成**多行版本**。
   *
   * 空数组 = 版本账读不到（假后端没这一路 / 随包目录）—— 退回"一个预设一行"的老画法，
   * 行为不退化。
   */
  cloudVersions: CloudVersion[]
  /**
   * **云端条目 → 我本地那一份的修改时刻**（epoch 秒），云端详情里摆「本地版本时间」用。
   *
   * 键是云端文件名（认亲只认血统 based_on 的末段，与 claims 同一套）；
   * 我这儿没有那一份就没有这个键 —— 就写「还没下载」，不编。
   */
  localTimes: Map<string, { at: number; release: string | null }>
  /** 下载区（`mkp/`）里有、**且与目录登记一致**的那些（`PresetFileState = ok`） */
  localPresets: DeliveryPresetSource[]
  /**
   * 下载区里有、**但与目录登记不一致**的那些（`PresetFileState = stale`）。
   *
   * 和 `localPresets` 分开传：本地表要按"盘上有没有"画（两者都画），
   * 云端表要按三态标（一个 ok 一个需更新）。合成一个数组的话，两处都要再拆一次。
   */
  stalePresets: DeliveryPresetSource[]
  /** 目录指纹前 16 位（来源列那枚 chip 用）。null = 没读到目录 */
  presetVersion: string | null
  /**
   * **当前目录的发布身份**（`catalog.catalogId` + `catalog.publishSerial`）——
   * 详情面板「发布线」那一行显示"云端是第几号"。
   *
   * **两格一组**：后端保证同生同死（只有一格的目录过不了解析），所以要么都有、
   * 要么都是 `null`（老目录 / 随包旧版 ⇒ 那一行不渲染，不摆"未知"）。
   */
  catalogLine: string | null
  catalogSerial: number | null
  /**
   * **备注覆盖账**（`api.getPresetRemarks()`）：用户改过的副标题。**只有本地表读它** ——
   * 云端表只读工作台写的那句（2026-10-07 作者定：本地的改动不许影响云端显示）。
   * 键 = 文件身份（交付预设行 `catalog.path` / 本地文件相对用户根路径）；
   * 有 ⇒ 本地副标题用它，没有 ⇒ 用那一版工作台写的 `remark`。
   */
  remarks: Record<string, string>
  /**
   * **这次发布的时刻**（catalog 的 `publishedAt`，RFC3339 / UTC）—— 云端表 preset 行
   * 「时间」列的来源，显示时前端转本机时区。null = 目录没带（随包 / 旧版发布），
   * 那一列照实「未知」。
   */
  releaseAt: string | null
  /**
   * **当前数据源**（设置页那一个的 mode：`gitee` / `github` / `custom`）——
   * preset 行「来源」列显示 GitHub / Gitee / 自定义源 的依据。null = 没配源
   * （离线 / 随包），来源照旧退「云端」。
   */
  sourceLabel: string | null
  /**
   * 当前源的**规范化身份**（`github#owner/repo@main`）—— 云端详情里
   * 「云端源」那一行照它显示。`null` = 没配源 / 认不出（就写「未知」）。
   * **判据不读它**：那是 Rust 侧 `based_on` 的事。
   */
  sourceIdentity: string | null
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
 * 三种话对应 [`PresetFileState`] 的三种"盘上有东西"：一致 / 旧版本 / 内容异常。
 * 它不改任何判定（判定全在 `state` 上）—— 只是让用户在这一行上就能看出该不该信它。
 */
function presetPathText(tr: Tr, p: DeliveryPresetSource): string {
  const at = tr('presets.tree.path.delivery', { uid: p.uid })
  const note =
    p.state === 'old'
      ? tr('presets.tree.path.old')
      : p.state === 'tampered'
        ? tr('presets.tree.path.tampered')
        : ''
  return `${at}${note}`
}

export interface DeliveryPresetSource {
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
  /**
   * 喷嘴 / 层高 —— **切片器 profile 才有**（MKP 的 `.toml` 没有这回事，字段不给）。
   *
   * 目录（catalog）里**没有**这两格，它们由 `getPresetFiles()` 对**同一份文件**
   * （按 `fileName` 对上）算好给出来 —— 算法只有后端一处（`ipc/presets.rs::slicer_axes`），
   * 前端只搬，**不在 TS 里从路径重算**（2026-10-07：preset 行从前不带这两格，
   * 切片器档的喷嘴 / 层高整列都是「—」）。
   */
  nozzle?: string
  layerHeight?: string
  /** 目录登记的字节数（真值） */
  size: number
  /** 它属于哪次发布（新世界目录没有版本号概念，恒 null；chip 只写「云端」） */
  presetVersion: string | null
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
   * 盘上那一份现在是什么（见 [`PresetFileState`]）。
   *
   * **算它的地方只有一处**：`usePresetData.readPresetFeed` 把三个读（目录清单 / 下载区 /
   * 认得出是哪一版吗）合成这一个字段。两张表都读它，**不许自己再判一次** ——
   * 两处各判一次，迟早有一处忘了跟（"需更新"以前就是那样把坏档说成"未下载"的）。
   */
  state: PresetFileState
}

/** 一台机型下的一个文件：代表那一份 + 用到它的版本名与 id */
interface MachineFile {
  machineId: string
  file: PresetFileNode
  /** 用到它的版本显示名，按机型目录里的顺序，已去重 */
  versions: string[]
  /** 与 versions 一一对应的版本 id（「使用」这一行时要写进底账，首页反填用） */
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
 * （bbs_config）混进"MKP 配置 → 云端"表里 —— 因为 preset 行当年默认
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
 * 台账那一格「本地 N」：用户自己的文件里**属于当前档**的个数。
 *
 * 判据与本地表 local 行的过滤是同一条（`localRows` 里那 `.filter`）：**认不出类别的
 * （`.json`）两档都算** —— 它在任何档下都列在表里，计数也要跟着列，藏起来就等于
 * 对它说"你没这份文件"。
 */
export function localCountOfAxis(local: LocalPresetFile[], axis: PresetKindAxis): number {
  return local.filter((f) => f.kind === null || matchesKind(axis, f.kind)).length
}

/*
 * # 表头排序（2026-10-16 作者：「表头加上排序吧……名称排序，机型排序，所有的都排序，时间也排序」）
 *
 * 每一列都能点，点同一列在升 / 降之间换。**置顶永远压在最上面**（那是另一件事的判据，
 * 不参与列排序）—— 列排序只决定"同一堆里谁在前"。
 *
 * # 名称列是**三循环**，别的列仍是两态（2026-10-20 作者：「名称那个应该是三循环的
 * 点击切换排序，就是默认，a-z，倒序」）
 *
 *   family（默认）→ asc（a-z）→ desc（z-a）→ 回到 family
 *
 * `family` = **族优先**：原件当锚，它那几份副本紧跟着（见 [`copyRankOf`]），
 * 副本序号不随方向翻面；asc / desc 是**纯文件名文本序**（那一刻副本按文本排到原件
 * 上面，正是用户点 a-z 要的结果）。别的列没有"族"这回事 —— 它们只有升 / 降两态。
 */

/** 能拿來排的列。★ 与表头那几格一一对应，加一列要同时改两处 */
export type PresetSortKey = 'name' | 'machine' | 'version' | 'nozzle' | 'layer' | 'time'

/**
 * 名称列那三档；别的列只在 `asc` / `desc` 之间换 —— 它们拿到 `family` 时按 `asc` 处理
 * （见 [`sortRows`] 里 `dirOf` 那一句：那一档对它们没有意义，不值得分叉）。
 */
export type PresetSortDir = 'family' | 'asc' | 'desc'

export interface PresetSort {
  key: PresetSortKey
  dir: PresetSortDir
}

/**
 * 取某一列的排序值。`null` = 这一行没有这一格（切片器档没有版本、MKP 档没有喷嘴层高、
 * 时间认不出来）—— **空值恒排最后**，不该因为点了降序就跳到最前面。
 *
 * ★ `time` 就用**这一列显示的那串**（`modifiedText`）：
 *   本地行 / 交付行是 `toISOString()`、云端行是 RFC3339 —— ISO 串的字典序**就是**时间序，
 *   不用再解析一遍。★ 切片器档那格的 `modifiedText` 直接来自契约，契约允许
 *   「`09-14` / ISO」两种（那一档本来就是演示值，`statFrom:'demo'`）⇒ 那一档退化为
 *   **按文本排**（同一年内是对的，跨年不行）。要它准，得让契约把真值给出来 ——
 *   那是改契约，本轮不做。
 */
function sortValueOf(row: PresetRowBase, key: PresetSortKey): string | null {
  switch (key) {
    case 'name':
      return row.fileName
    case 'machine':
      /* ★ `—`（未标机型）就是"没有"的写法 —— 与空值同一个待遇，排最后。
         不处理的话 `—` 按文本比会跑到所有机型前面（U+2014 < 字母），
         而用户按机型排序是想看"按机型分堆"，最该沉底的是"分不出机型"的那几份 */
      return row.machineText === DASH_ ? null : row.machineText
    case 'version':
      return row.versions.length === 0 ? null : row.versions.join(' · ')
    case 'nozzle':
      return row.nozzle ?? null
    case 'layer':
      return row.layerHeight ?? null
    case 'time':
      return row.modifiedText ?? null
  }
}

/** 喷嘴 / 层高是 `0.4` 这种**数字串** —— 按数值比（`0.2 < 0.4 < 0.6`），按文本比会错 */
function byNumber(key: PresetSortKey): boolean {
  return key === 'nozzle' || key === 'layer'
}

/**
 * 置顶的排最前（**后点的更前**），其余按表头那一列。
 *
 * 置顶是**纯前端的排序**（落 localStorage，见 `usePresetData` 的 PINNED_KEY），
 * 不是「设为当前」—— 应用是另一件事，右键菜单里没有它。
 *
 * ★ 2026-10-16 作者：「后面点的置顶的应该置顶，多个置顶的按照先后顺序，后点击的置顶」
 *   ⇒ `pinOrder` 是**置顶的先后**（0 = 最早），所以这里**倒着比**：下标大的排前面。
 *     未置顶那份是 `-1`，减出来的差恒为正 ⇒ 永远落在所有置顶的后面 —— 一个式子把两件事都办了。
 *
 * ★ 同一族（`A1-fast.toml` 与它的 `copy` / `copy 2`）**原件在最前**，见 `copyRankOf`。
 */
function sortRows<R extends PresetRowBase>(rows: R[], sort: PresetSort): R[] {
  /*
   * 名称列三档（2026-10-20）：`family` 与 `asc` 都当升序，只有 `desc` 翻面 ——
   * `family` 若落到别的列（它们只有两态）上，也是升序，不用分叉。
   */
  const dir: 1 | -1 = sort.dir === 'desc' ? -1 : 1
  const family = sort.key === 'name' && sort.dir === 'family'
  const num = byNumber(sort.key)
  const rank = new Map<string, CopyRank>()
  const rankOf = (fileName: string): CopyRank => {
    const hit = rank.get(fileName)
    if (hit !== undefined) return hit
    const r = copyRankOf(fileName)
    rank.set(fileName, r)
    return r
  }

  /** 副本链：逐层比，**短的在前**（`[1]` 在 `[1, 1]` 前面） */
  const byChain = (a: number[], b: number[]): number => {
    const len = Math.min(a.length, b.length)
    for (let i = 0; i < len; i += 1) {
      const d = (a[i] ?? 0) - (b[i] ?? 0)
      if (d !== 0) return d
    }
    return a.length - b.length
  }

  /*
   * **族优先**（`dir` 只作用在"族"上）。
   *
   * 先按"族"（原件名）比，同族再按副本链 ⇒ 复制出来的那一份就躺在原件下面
   * （`A1-fast copy.toml` 若按整串文件名比会跑到 `A1-fast.toml` **上面**，因为空格比点小）。
   * ★★ 副本链**不乘 `dir`**："原件在前、副本在后"是一条**稳定的**规矩，
   *    不该因为用户点了降序就把 `copy 2` 翻到原件上面去。
   */
  const byName = (a: R, b: R, d: 1 | -1): number => {
    const ra = rankOf(a.fileName)
    const rb = rankOf(b.fileName)
    if (ra.base !== rb.base) return ra.base.localeCompare(rb.base) * d
    const c = byChain(ra.chain, rb.chain)
    if (c !== 0) return c
    return a.fileName.localeCompare(b.fileName) * d
  }

  return [...rows].sort((a, b) => {
    if (a.pinOrder !== b.pinOrder) return b.pinOrder - a.pinOrder

    /* 名字那一列：默认档族优先；a-z / z-a 那两档是**纯文件名文本序** */
    if (sort.key === 'name') {
      if (!family) return a.fileName.localeCompare(b.fileName) * dir
      return byName(a, b, dir)
    }

    const va = sortValueOf(a, sort.key)
    const vb = sortValueOf(b, sort.key)
    /* 空值恒排最后（不管方向） */
    if (va === null && vb === null) return byName(a, b, 1)
    if (va === null) return 1
    if (vb === null) return -1
    const c = num ? Number.parseFloat(va) - Number.parseFloat(vb) : va.localeCompare(vb)
    /* 数字解析不出来就退回按文本比，别排出个 NaN 把顺序弄乱 */
    const r = Number.isNaN(c) ? va.localeCompare(vb) : c
    if (r !== 0) return r * dir
    /*
     * ★ 这一列**比不出先后**（同一机型、同一天改的…）⇒ 回退到**按名字**、且**恒升序**。
     *   没有这一句，同值的那几份的顺序就取决于数组原本的构造顺序 —— 用户看着就是"乱的"。
     */
    return byName(a, b, 1)
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
 * 版本 id → 显示名（preset 行的「版本」列）。
 *
 * delivery 里存的只有 versionId（`STANDARD`），表格那一列要的是人话（「标准版」）——
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
 * 一份**本地文件**自己说属于哪个「机型 · 版本」：文件头 `# machine:` / `# variant:`
 * 两行（后端已归一化）；头里没有 / 认不出就**回落到血统**那份。都没有 = `null`
 * （导入的裸文件 —— 在列表里照常列，只是不对应任何 combo）。
 */
export function ownComboOf(f: LocalPresetFile): { machineId: string; versionId: string } | null {
  const machineId = f.machineId ?? f.basedOnMachineId
  const versionId = f.versionId ?? f.basedOnVersionId
  return machineId === null || versionId === null ? null : { machineId, versionId }
}

/**
 * **本地文件按 `机型:版本` 索引**（只收 MKP 预设；同一个 combo 有多份时取第一份）——
 * 参数页的编辑目标、首页 / 校准页"切 combo 顺手应用"都读它。
 *
 * 云端那些不在这张表里：它们只能查看 / 下载，编辑与使用都发生在本地文件上。
 */
export function localByCombo(local: LocalPresetFile[]): Map<string, LocalPresetFile> {
  const out = new Map<string, LocalPresetFile>()
  for (const f of local) {
    if (f.kind !== 'mkp_preset') continue
    const combo = ownComboOf(f)
    if (combo === null) continue
    const key = `${combo.machineId}:${combo.versionId}`
    if (!out.has(key)) out.set(key, f)
  }
  return out
}

/**
 * **「本地预设」那一半的行**（`presets-local/` 扫出来的那些）—— 本地表与资源库列表
 * 共用的**唯一一条映射**（两个列表的这一半不许各写一份）。
 *
 * 与云端行最本质的区别（总纲 §1③）：**云端没有它们** —— 没有交付身份、不属于任何版本、
 * 也没有 SHA 可比。所以它们只活在用户侧的表里，云端表永远看不到。
 *
 * 两条"不知道就别筛掉"的口径（藏起来等于对用户说他没这份文件）：
 *   - **认不出类别的**（`.json` 分不出 bbs 还是 orca）在任何类型档下都列；
 *   - **机型**这一档**真的筛**：归属（文件头 `# machine:` / `# variant:` 那两行，2026-10-07
 *     起就有，还能「改归属」）说得出属于哪台机的，就只在那台机下出现；
 *     **归属认不出的**（导入的裸文件）才在任何机型档下列，机上那格写「—」、
 *     名称列挂「未标机型」。这与云端那半边的 `untagged` 是同一条口径。
 *
 * ★ 机型筛这一段的历史：注释原来写的是「这一层根本没有来源 → 任何机型档下都列」，
 * 那句话在「改归属」落地那天就过期了，而**没人跟着改筛**，于是筛 A1 mini 时 A1 / XC
 * 的文件照旧满屏（作者 2026-10-08 实测）。现在按归属筛（见最后那个 `.filter`）。
 */
function localHalf(input: PresetRowsInput): PresetLocalRow[] {
  const {
    machines,
    machineId,
    local: localPresetFiles,
    draftPath,
    kind,
    active,
    pinned,
    remarks,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const versionRemark = versionRemarkLookup(machines)
  /*
   * 副标题：**账上有覆盖（包括空串）就用用户的**（2026-10-07 作者改口：
   * 「可以空着，不要回退」—— 用户写空副标题就空）；没有覆盖才走
   * 「工作台写的 → 空着」回落。「恢复默认」= 删掉覆盖。
   */
  const remarkOr = (key: string | null, fallback: string): string =>
    key !== null && remarks[key] !== undefined ? remarks[key] : fallback
  return localPresetFiles
    .filter((f) => f.kind === null || matchesKind(kind, f.kind))
    .map((f): PresetLocalRow => {
      /*
       * 本地文件**也能被使用**（第七层）：只有认得出是 MKP 预设（`.toml`）的才行 ——
       * `.json` 那几份（bbs / orca 分不出）不是预设，使用它们无从谈起。
       * 「在用」认的是**唯一底账**里的那一条：路径就是这一份（底账只指向本地文件）。
       */
      const canApply = f.kind === 'mkp_preset'
      const live = canApply && active?.path === f.path
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
        rowKey: `local:${machineId}:${f.path}`,
        /* 没有 asset id，也没有别的身份 —— **路径就是它的身份**（用户随时可能改名，认路径最稳） */
        pinKey: f.path,
        fileName: f.fileName,
        path: f.path,
        /* 我自己的文件落在用户根里 ⇒ `local:`（认不出是哪一类的 `.json` 不进这本账） */
        usageKey: canApply ? `local:${f.path}` : undefined,
        /* 副标题 = 备注覆盖账里的 → 归属那一版的备注 → 空着（路径在 title 里，不丢） */
        subtitle: remarkOr(
          f.path,
          ownMachine !== null && ownVersion !== null
            ? (versionRemark(ownMachine, ownVersion) ?? '')
            : '',
        ),
        remarkKey: f.path,
        /* 认不出是哪一类就照实留 `null`：展开详情里写"认不出"，不替他认成 MKP */
        kind: f.kind,
        machineId,
        /* **归属**说它属于哪台 —— 不再是「—」；归属都没有（导入的裸文件）才写「—」 */
        machineText: ownMachine === null ? DASH_ : (names.get(ownMachine) ?? ownMachine),
        /* 归属那一版的显示名。都没有就不写，不替他猜一个 */
        versions: ownMachine !== null && ownVersion !== null ? [versionName(ownMachine, ownVersion)] : [],
        /* **真值**：盘上那份的大小与改动时刻（本地文件也盘当底账）—— 与切片器那一档同一档来源 */
        sizeText: sizeTextOf(f.size),
        modifiedText:
          f.modifiedUnix === null ? undefined : new Date(f.modifiedUnix * 1000).toISOString(),
        statFrom: 'file',
        applied: live,
        pinned: pinned.includes(f.path),
        pinOrder: pinned.indexOf(f.path),
        /* 参数页改了值还没点保存的那一份 —— 行上那枚「未保存」小圆点 */
        unsaved: f.path === draftPath,
        scope: 'local',
        origin: 'local',
        /* 归属都没有（导入的裸文件）才挂「未标机型」—— 有归属的那份现在自己说得出 */
        untagged: ownMachine === null,
        live,
        /* 血统：它当初基于云端哪一版、那一版现在还在不在（第七层） */
        basedOn: f.basedOn,
        basedOnFacts: f.basedOnFacts,
        basedOnSource: f.basedOnLabel,
        basedOnCloud: source,

        /* 详情面板「改归属」读它（机型 / 版本两个下拉的数据源是行上这份归属） */
        ownMachineId: ownMachine,
        ownVersionId: ownVersion,
        /*
         * 第九层：文件级状态。**读不出来的照样列出来**（藏起来等于说他没这份文件），
         * 只是不给「使用 / 改这份」—— 与 `.json` 那份"不给必报错的按钮"同一条口径。
         * 认不出是哪一类的没有这一档（`null` → `undefined`）。
         */
        localState: f.state ?? undefined,
        localStateDetail: f.stateDetail,
        /* 出处账：复制自哪一份 / 是不是导入的（「来源」那一格的数据，见 `originCellOf`） */
        provenance: f.provenance ?? null,
        copiedFrom: f.copiedFrom,
        copiedFromName: f.copiedFromName,
      }
    })
    /*
     * 机型这一档**真的筛**（2026-10-08 修）：归属说得出属于哪台机的，就只在那一档下出现；
     * 归属认不出的（`ownMachineId === null`，导入的裸文件）在任何机型档下列 ——
     * 藏起来等于说他没这份文件。`machineId === ''` 是「全部机型」那一档。
     */
    .filter(
      (row) =>
        row.ownMachineId === null || machineId === '' || row.ownMachineId === machineId,
    )
}

/**
 * **本地表**（两种类型都算）：本机磁盘上真有的东西。
 *
 *   MKP     只列 `presets-local/` 里那几行（你自己那份本地文件，见下面 `localOnly`）——
 *           云端原件是内部数据，不进这张表
 *   切片器   云端副本（`getLocalFiles()` 说已经下到本机的那些）+ 本地文件
 *           （`getLocalPresetFiles()` 扫出来的；认不出类别的 `.json` 在任何类型档下都列）
 *
 * 云端那一半按**机型**汇总（一个文件一行，用到它的版本收进 `versions`）；
 * 本地那一半是**本地文件**（两张表共用 [`localHalf`]，这一半永远一致）。
 *
 * 「生效」两种类型两套判据（见 `PresetLocalRow.live`）：MKP 看唯一底账
 * （使用中指针）里那一条，切片器看已复制到切片器目录的那个集合。
 */
export function localRows(tr: Tr, input: PresetRowsInput): PresetTableData<PresetLocalRow> {
  const {
    machines,
    machineId,
    localIds,
    slicerCopiedIds,
    active,
    kind,
    query,
    pinned,
    sort,
    localPresets,
    stalePresets,
    presetVersion,
    releaseAt,
    remarks,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const versionRemark = versionRemarkLookup(machines)
  /*
   * 副标题：**账上有覆盖（包括空串）就用用户的**（2026-10-07 作者改口：
   * 「可以空着，不要回退」—— 用户写空副标题就空）；没有覆盖才走
   * 「工作台写的 → 空着」回落。「恢复默认」= 删掉覆盖。
   */
  const remarkOr = (key: string | null, fallback: string): string =>
    key !== null && remarks[key] !== undefined ? remarks[key] : fallback

  /*
   * **MKP 档的本地表只列"我的本地文件"**（`localOnly`，2026-10-09 定死）。
   *
   * 云端那一半（下载区里的云端副本）不进这张表：那是内部数据 —— 它是隐藏基准的来源，
   * 用户既看不到也改不了它。把"云端原件在本机"当成"本机有这一份预设"摆给用户看，
   * 正是上一版让人误以为"本地有两份"的根子。
   *
   * 切片器档照旧（本机真有的那些 + 云端菜单上的那些，两张表各答各的）。
   */
  const localOnly = kind === 'mkp'

  const assetRows = (localOnly ? [] : machineFiles(machines))
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
        /* 仓库里那份落在内部根 ⇒ `cloud:`（只有 MKP 预设跑后处理） */
        usageKey: f.kind === 'mkp_preset' ? `cloud:${f.path}` : undefined,
        /* 云端仓库行没有备注的来源也没有覆盖（remarkKey null）—— 副标题空着（路径在 title 里） */
        subtitle: '',
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
        pinned: pinned.includes(pinKey),
        pinOrder: pinned.indexOf(pinKey),
        /* 云端行不是「本地文件」—— 参数页那份未保存草稿不会有它的份 */
        unsaved: false,
        scope: 'local',
        origin: 'asset',
        delivery: f.delivery,
        untagged: false,
        /* MKP 的生效是「被应用」，切片器的生效是「被复制进切片器目录」—— 两回事 */
        live: kind === 'mkp' ? applied : slicerCopiedIds.has(f.fileName),
      }
    })

  const local = localHalf(input)

  /*
   * 目录里登记的交付预设（**切片器档**）：下载之后它们就躺在下载区，
   * 本地说的就是「本机磁盘上真有的文件」—— 所以这一半**收盘上真有的那些**：
   * 与目录一致的、以及不一致的（旧版本 / 内容异常）。四档全在盘上，
   * 藏起后三种就等于对用户说"你机器上没有它"，而修复它的入口也就没了。
   * MKP 档**不列**（那一档只列 `presets-local/` 那几行，见上面 `localOnly`）；
   * 「生效」认唯一底账（`run/app-state.json` 的 activePreset）里那一条（合流，不分两套）。
   */
  const onDisk = localOnly ? [] : [...localPresets, ...stalePresets]

  const presetRows = onDisk
    .filter((p) => matchesKind(kind, p.kind))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .map((p): PresetLocalRow => {
      const state = p.state
      /*
       * MKP 交付行的「生效」= 被启用（使用中指针认的一直是 MKP 交付文件）；
       * 切片器那一类的「生效」= 已复制进切片器目录（2026-10-09 接通）—— 两回事，别合流。
       */
      const live =
        p.kind === 'mkp_preset'
          ? active !== null && active.fileName === p.fileName
          : slicerCopiedIds.has(p.fileName)
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
        rowKey: `preset:${machineId}:${p.fileName}`,
        /* 仓库里没有它，没有 assetId —— 「使用」认 fileName（页面里分流） */
        pinKey: `preset:${p.fileName}`,
        fileName: p.fileName,
        /* 下载区里那份也落在内部根 ⇒ `cloud:`（只有 MKP 预设跑后处理） */
        usageKey: p.kind === 'mkp_preset' ? `cloud:${p.path}` : undefined,
        /* 第二行小字（`path` 仍是盘上落点，「在文件夹中显示」与 title 用它）：
           副标题 = 备注覆盖账里的 → 那一版工作台写的 → 空着；
           盘上那份不对劲时那件事**必须**还写在原地 —— 覆盖账压不住状态注记 */
        path: presetPathText(tr, p),
        subtitle:
          /* 盘上那份不对劲（旧版本 / 内容异常）时那件事**必须**写在副标题原地 ——
             状态注记压过备注；一致的那份才走「覆盖账 → 工作台备注 → 空着」回落 */
          p.state === 'ok'
            ? remarkOr(p.path, versionRemark(p.machineId, p.versionId) ?? '')
            : presetPathText(tr, p),
        remarkKey: p.path,
        kind: p.kind,
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        /* 喷嘴 / 层高：切片器 profile 才有（按 fileName 从 `getPresetFiles` 对过来的） */
        nozzle: p.nozzle,
        layerHeight: p.layerHeight,
        sizeText: sizeTextOf(p.size),
        /*
         * 到位时刻：这份字节是「下载」进来的还是「替换」上去的（两个事件至多一个有值，
         * 见 `DeliveryPresetSource`）—— 标签跟着事件走，UI 不自己挑。epoch 秒 → ISO
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
        pinned: pinned.includes(`preset:${p.fileName}`),
        pinOrder: pinned.indexOf(`preset:${p.fileName}`),
        /* 交付行也不是「本地文件」—— 未保存的那份草稿只会落在 `presets-local/` */
        unsaved: false,
        scope: 'local',
        origin: 'preset',
        untagged: false,
        presetUid: p.uid,
        presetVersion,
        presetState: state,
        /* 发布时刻：展开详情「云端更新」一格（本地行的下载时间在 `modifiedText` 上） */
        publishedAt: releaseAt,
        live,
      }
    })

  const all = [...assetRows, ...local, ...presetRows]
  return {
    rows: sortRows(all.filter((r) => fileMatchesQuery(r, query)), sort),
    total: all.length,
  }
}

/**
 * **本地表 / 云端表共用的那一颗按钮**：把这一份设为当前生效（`applyActivePreset`）。
 *
 * 两边（云端那一侧与本地文件）**同一个词**：用户看到的那一份就是"这一份预设"，
 * 不必先分清它住哪条线。云端原件不进用户世界，所以今天能走到这里的基本都是
 * `presets-local/` 里那几行 —— 词照旧只有这一个。
 */
export const ASSET_USE_TEXT = {
  on: 'presets.tree.assetUse.on',
  off: 'presets.tree.assetUse.off',
  /**
   * **取消使用**（2026-10-10 作者裁决恢复的入口：「它需要一个取消使用的按钮，
   * 而不是说这个灰色的在用」）。撤下的只是"哪一份在生效"这个指向
   * （唯一底账 `run/active-preset.json`）—— **磁盘上一个字节都不动**。
   */
  cancel: 'presets.tree.assetUse.cancel',
} as const satisfies Record<string, StringKey>

/**
 * **这一行生效了没有**（展开详情「状态」那一格）。
 *
 * 与 [`ASSET_USE_TEXT`] 分开是有意的：那个是**按钮上的字**（点了会发生什么），
 * 这个是**状态的字**（现在是什么样）—— `off` 那半句要是也写「使用」，
 * 展开详情里就会读到「状态：使用」这种把动作当状态的句子。
 *
 * 2026-10-10 起两边的 `on` 都是「在用」（两个字，按钮那两颗因此同宽），
 * `off` 一边是动作、一边是状态，仍然分开写。
 */
export const ASSET_STATE_TEXT = {
  on: 'presets.tree.assetState.on',
  off: 'presets.tree.assetState.off',
} as const satisfies Record<string, StringKey>

/**
 * 云端表：**菜单上的云端文件**（已分配 + 可选）。
 *
 * 取的是树上这些机型的文件 —— 仅归档的文件建树时就整个剔掉了（`archivedIds`），
 * 所以这里不需要再过滤一遍，也**不该**再过滤一遍：多一处过滤就多一处可能漏。
 *
 * 行尾两件事分开说：`downloaded` 是「你机器上有没有」，`delivery` 是「要不要你自己动手」。
 * `applied` 是第三件事，判据只有一条 asset id 相等 —— 所以一个还没下的文件也可能是
 * 「正在配着的那一份」，那时页脚会说「配着 X（还没下）」。
 */
/*
 * ★ `versionDateText` 随「逐版各占一行」一起退役了（2026-10-08 当天下午）：
 * 它算的是**版本行**那一格时间（已下载的取文件头 `release_time`，没下的取目录代时间）。
 * 云端表回到「一个目录条目一行」之后，那一格时间是目录这次发布的时刻（`releaseAt`）——
 * 见 `cloudRows` 里 `modifiedText` 的说明。留着它就是留一条没人走的路。
 */

export function cloudRows(tr: Tr, input: PresetRowsInput): PresetTableData<PresetCloudRow> {
  const {
    machines,
    machineId,
    localIds,
    active,
    kind,
    query,
    pinned,
    sort,
    deliveryPresets,
    cloudVersions,
    localTimes,
    presetVersion,
    releaseAt,
    catalogLine,
    catalogSerial,
    sourceIdentity,
  } = input
  const names = machineNames(machines)
  const versionName = versionNameLookup(machines)
  const versionRemark = versionRemarkLookup(machines)

  /*
   * **"我这边有没有一份基于它的本地文件"**（MKP 档，2026-10-09）。
   *
   * 认亲只认**来源关系**：我那份文件头血统里的 `based_on` 末段就是云端那个文件名
   * （`delivery/mkp/presets/A1-fast.toml` → `A1-fast.toml`）。**不按文件名认** ——
   * 用户导入一份同名文件不会被当成"本地已有"（自己拉进来的东西没有来源关系，
   * 它在本列表里就是"未获取"，而他那份照常在本地表里列着）。
   *
   * 一个云端条目可能被我好几份认领（「另存为一份新的」造出来的副本血统照抄）。
   * 这一行只回答"有没有、新不新"，所以取其中**最好的那一个**：
   * 只要有一份跟得上当前版就算「本地已有」。
   *
   * `unknown`（认得出来源名字、但比对不了摘要）**当"不敢说它跟得上"** ——
   * 报「有新版」比报「本地已有」安全：点一下"更新"是幂等的（我那份一个字节不动），
   * 而默默说"跟得上"却让用户错过云端改动，是这一轮要根治的那类毛病。
   */
  /*
   * ★ 2026-10-11（a04）：认亲**改成只认身份账本**（不再看血统）。
   *
   *   为什么改：血统那三行**复制也会照抄** —— 于是"另存为一份新的"造出来的副本
   *   会跟着一起认领云端条目，云端的"有新版 / 更新"就落到副本身上了（那些副本
   *   其实跟云端没关系）。账本一条一份、**只有下载建**，正好是"这一份跟云端有关吗"
   *   这件事的唯一凭据（口径见 `lab/preset-identity-and-update-spec.md` §6）。
   *
   *   血统照旧留在文件头 —— 它管的是"这一份从哪一版派生"（展开详情那一格），
   *   与"跟不跟云端"是两件事。
   */
  const claims = new Map<string, BasedOn>()
  if (kind === 'mkp') {
    const indexByPath = new Map(input.localIndex.map((e) => [e.path, e]))
    for (const f of input.local) {
      if (f.kind !== 'mkp_preset') continue
      const entry = indexByPath.get(f.path)
      /* 账本里没有它 → 不认领（复制 / 拖回 / 导入 / 恢复的那些都在这一档） */
      if (entry === undefined) continue
      /*
       * 认领者里**只要有一份不是 current**，这一行就还有「更新」要做
       * （更新是"把云端新版处理到本地这份上"，陈旧的认领者需要它）——
       * 只有所有认领者都跟得上当前版时才翻成「本地已有」。
       */
      const prev = claims.get(entry.cloudSource)
      if (f.basedOn === 'current' && prev !== undefined) continue
      claims.set(entry.cloudSource, f.basedOn)
    }
  }
  /**
   * 云端那一行「我这边怎么样」。切片器档没有这一档（它照旧按 `presetState` 说）。
   *
   * 判据只有一条（2026-10-10）：**我那份的认领里有没有"跟不上云端当前版"的**。
   *
   * ```text
   * none   这个云端条目我这儿一份都没有   → 「下载」（取回 + 落一份本地的）
   * stale  有一份（或几份）基于旧版 / 认不出 → 「更新」（逐项处理差异，见 PagePresets 的 runUpdate）
   * has    所有认领者都跟得上             → 不给动作（要改要用都在本地表那一行上）
   * ```
   */
  const localStateOf = (fileName: string): CloudLocalState | undefined => {
    if (kind !== 'mkp') return undefined
    const claim = claims.get(fileName)
    if (claim === undefined) return 'none'
    if (claim === 'current') return 'has'
    /*
     * 我那份基于的**不是**云端现在这一版 → 给「更新」。
     *
     * ★ 2026-10-10 改判：判据不再看"下载区字节取回来没有" —— 「更新」现在是
     * **把云端新版逐项处理到本地这份上**（没冲突的按新版采用、冲突的让用户选），
     * 字节在不在本机不影响这件事要不要做。旧口径（取回字节就翻「本地已有」）
     * 会让用户根本点不到「更新」，新版永远落不到他这份文件上。
     */
    return 'stale'
  }

  /*
   * 凡文件名在**云端最新发布**里打包过的，云端行不再列出 —— 发布行接管它
   * （作者：「这种云端是不是不用再存在了」）。两行本来就同源：云端行是仓库身份、
   * 「下载」是死按钮（downloadFiles 未实现）；发布行才是真能下的那一行。
   * 没有发布（列表为空）时云端行照列，行为不退化。
   * 本地表不动 —— 那里「云端」说的是"你机器上的云端副本"，是本机事实。
   */
  const deliveredNames = new Set(deliveryPresets.map((p) => p.fileName))
  const assetRows = machineFiles(machines)
    .filter((e) => matchesKind(kind, e.file.kind))
    .filter((e) => !deliveredNames.has(e.file.fileName))
    .map((e): PresetCloudRow => {
      const f = e.file
      const pinKey = f.id ?? f.path
      return {
        rowKey: `${e.machineId}:${f.path}`,
        assetId: f.id,
        pinKey,
        fileName: f.fileName,
        path: f.path,
        /* 仓库里那份落在内部根 ⇒ `cloud:`（只有 MKP 预设跑后处理） */
        usageKey: f.kind === 'mkp_preset' ? `cloud:${f.path}` : undefined,
        /* 云端仓库行没有备注的来源（工作台只给交付预设写 remark）—— 副标题空着（路径在 title 里） */
        subtitle: '',
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
        /* 云端行没有「正在使用」这一说 —— 使用中指针只认 MKP 交付文件，剩下的云端行全是切片器档 */
        applied: false,
        pinned: pinned.includes(pinKey),
        pinOrder: pinned.indexOf(pinKey),
        /* 未保存的草稿只服务本地表的「本地文件」行，云端行恒 false */
        unsaved: false,
        scope: 'cloud',
        origin: 'asset',
        delivery: f.delivery,
        downloaded: f.id !== undefined && localIds.has(f.id),
      }
    })

  /*
   * 目录里登记的交付文件（catalog 的 files 域）—— 云端表上它们那几行。
   * 与云端文件同列一个「来源」chip 区分；「已下载」看下载区（`mkp/`，盘就是底账），
   * 「正在生效」看唯一底账（`run/active-preset.json`）。**按 `kind` 分流**：
   * MKP 档只列 MKP 预设、切片器档只列 BBS / Orca 的配置 —— 不靠扩展名猜；
   * 图标 / 模型在数据层就被 `catalogKindToFileKind` 挡掉了（不归这一页）。
   */
  const presetRows = deliveryPresets
    .filter((p) => matchesKind(kind, p.kind))
    .filter((p) => machineId === '' || p.machineId === machineId)
    .flatMap((p): PresetCloudRow[] => {
      const live = p.kind === 'mkp_preset' && active !== null && active.fileName === p.fileName
      /* 四档全在源上算好了（见 `DeliveryPresetSource.state`）—— 这里只搬，不判 */
      const state = p.state
      /*
       * **一个目录条目一行**（2026-10-08 作者改判）。
       *
       * 上午那一版是"同一个预设的每一代云端版本各占一行"（版本账 `cloudVersions`
       * 按 `downloaded || current` 滤，`versions.map` 铺开）。作者在真机上看到的后果是：
       * `A1_MINI-fast.toml` **三行同名** —— 名称 / 机型 / 版本三列逐字一样，只有时间列
       * 不同（还有两行都是 08 号），他原话「我觉得每一个版本就只应该显示一个」
       * 「他为什么不沿用本地的名字呢……真正值得下载的才显示云端的名字」。
       *
       * 于是回到一行：这一行答的还是那三件事（云上有没有我这份 / 我该不该下 /
       * 我这份对不对），状态与动作全部由 `p.presetState` 出（四档在源上算好）。
       *
       * **下过的旧版去哪了**：它们本来就是**用户的东西**，以用户自己的名字躺在本地表里
       * （`A1_MINI-fast-2026-10-06.toml` 这种，`presets-local/`），展开这一行还能看到
       * 归档里那几份旧版（`.facts` 的「旧版本 N」）—— 云端表不必再替它们各占一行。
       * 列表上只留一个数（`olderVersions`），把"下过几代旧的"这件事说出来。
       */
      const olderVersions =
        kind === 'mkp'
          ? undefined
          : cloudVersions.filter(
              (v) => v.fileName === p.fileName && v.downloaded && !v.current,
            ).length
      const row = {
        /* 行键 / 置顶键都认 fileName —— 理由见 `localRows` 里那一段（别拿 uid 当键） */
        rowKey: `preset-cloud:${p.fileName}`,
        pinKey: `preset:${p.fileName}`,
        fileName: p.fileName,
        /* 这一行的 `path` 是展示文案（下面那行）—— 真正的落点在 `p.path`，钥匙从它来 */
        usageKey: p.kind === 'mkp_preset' ? `cloud:${p.path}` : undefined,
        path: tr('presets.tree.path.cloud', { machine: p.machineId, version: p.versionId }),
        /*
         * 副标题 = **只读工作台写的那句**（这一版发布时带进目录的 `remark`）。
         * 云端表不读本地覆盖账（2026-10-07 作者三轮：本地的改动不许影响云端显示）——
         * 要写自己的备注，下载后在本地表里改。工作台没写就**空着**
         * （不回落「云端 / …」那种位置文案；落点仍在 `title` 里）。
         */
        subtitle: versionRemark(p.machineId, p.versionId) ?? '',
        remarkKey: null,
        kind: p.kind,
        machineId: p.machineId,
        machineText: names.get(p.machineId) ?? p.machineId,
        versions: [versionName(p.machineId, p.versionId)],
        /* 喷嘴 / 层高：切片器 profile 才有（按 fileName 从 `getPresetFiles` 对过来的） */
        nozzle: p.nozzle,
        layerHeight: p.layerHeight,
        sizeText: sizeTextOf(p.size),
        /*
         * 云端表的时间 = **这次发布的时刻**（`releaseAt`，发布侧盖进目录的戳）——
         * 同一次发布的每一行是同一个值；目录没带（随包 / 旧版发布）就是 undefined
         * （「未知」+ `PRESET_TIME_WHY.cloudMissing` 那句话）。
         */
        modifiedText: releaseAt ?? undefined,
        applied: live,
        pinned: pinned.includes(`preset:${p.fileName}`),
        pinOrder: pinned.indexOf(`preset:${p.fileName}`),
        /* 同上一处：云端行没有「未保存」这一档 */
        unsaved: false,
        scope: 'cloud',
        origin: 'preset',
        presetUid: p.uid,
        presetVersion,
        delivery: 'default',
        /* `downloaded` 仍问"与目录一致的那一份在不在本机"（切片器那一档也用它） */
        downloaded: state === 'ok',
        presetState: state,
        publishedAt: releaseAt,
        /*
         * 我本地那一份**基于的云端版本时间**（认亲认出来的）—— 云端详情里和
         * 「云端发布时间」同量纲对照。★ 不能拿文件修改时间：那是我哪天动过它，
         * 与版本新旧无关（作者 2026-10-27 就是被那个数带偏了）。
         */
        localReleaseText: localTimes.get(p.fileName)?.release ?? null,
        /*
         * 云端详情里那一行「云端源」（2026-10-27）。
         *
         * ★ 云端也要摆**时间**（作者第二次截图：详情里只有「更新」按钮、没有时间）。
         *   与本地行同一套「源 + 时间」两格，读的人不用去别处凑。
         *   与「来源：自定义源」不重复：那一格说的是**模式**（自定义 / GitHub / Gitee），
         *   这一格是**规范化身份**（`gitee#owner/repo@ref`）—— 键名也分开写。
         */
        basedOnFacts: {
          /*
           * 云端行没有"我基于哪一版"这回事 —— 那是**本地行**的语义（我手上这份从哪拷的）。
           * 这里照实留空，不拿目录的身份去冒充血统。
           */
          lineageSource: null,
          lineageRelease: null,
          lineagePublishedAt: null,
          cloudSource: sourceIdentity,
          cloudPublishedAt: releaseAt,
          lineageCatalog: null,
          lineageSerial: null,
          /*
           * **这一份目录的发布身份**（2026-10-28）：详情里「发布线 · 第几号」那一行。
           * 两格同生同死；老目录 / 随包旧版都是 null ⇒ 那一行不渲染。
           */
          cloudCatalog: catalogLine,
          cloudSerial: catalogSerial,
        },
        olderVersions,
        /* MKP 档的那一问（切片器档是 `undefined`，它照旧读 `presetState`） */
        /* 同一个判据：云端行也用后端那一格 BasedOn（与本地行同一条规则） */
        basedOn: claims.get(p.fileName),
        localState: localStateOf(p.fileName),
      } satisfies PresetCloudRow
      return [row]
    })

  const all = [...assetRows, ...presetRows]
  return {
    rows: sortRows(all.filter((r) => fileMatchesQuery(r, query)), sort),
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
 * ★ 层高**按喷嘴分组**（2026-10-10）：一个喷嘴只配它自己那一组层高 ——
 *   0.2 喷嘴没有 0.30、0.8 喷嘴没有 0.10，而 0.24 在 0.4 / 0.6 / 0.8 三个喷嘴下
 *   各是各的文件。所以层高**不是**一张全局表：
 *
 *     layers          所有喷嘴的层高并集（喷嘴那一档是「全部」时用它）
 *     layersByNozzle  每个喷嘴自己的层高（选了喷嘴就用它那一份）
 *
 *   规矩在 `usePresetPage`：选了喷嘴 → 层高只列这个喷嘴的；喷嘴是「全部」→ 列并集。
 *   切喷嘴时当前层高不在新集合里就自动回「全部」，不然会筛出一张永远空着的表。
 *
 * 可选项要从**没筛过**的行里取 —— 从筛过的行取的话，选中 0.2 之后再点 0.4，
 * 0.2 的选项就消失了。排序按数值不按字符串：'0.10' 的字符串序会插到 '0.2' 前面。
 */
export function slicerFilterValues(
  rows: ReadonlyArray<{ nozzle?: string; layerHeight?: string }>,
): { nozzles: string[]; layers: string[]; layersByNozzle: Record<string, string[]> } {
  const byNum = (a: string, b: string): number => {
    const da = Number.parseFloat(a)
    const db = Number.parseFloat(b)
    if (Number.isFinite(da) && Number.isFinite(db) && da !== db) return da - db
    return a.localeCompare(b)
  }
  const sorted = (values: ReadonlyArray<string | undefined>): string[] =>
    [...new Set(values.filter((v): v is string => v !== undefined))].sort(byNum)
  const nozzles = sorted(rows.map((r) => r.nozzle))
  const layersByNozzle: Record<string, string[]> = {}
  for (const n of nozzles) {
    layersByNozzle[n] = sorted(rows.filter((r) => r.nozzle === n).map((r) => r.layerHeight))
  }
  return {
    nozzles,
    layers: sorted(rows.map((r) => r.layerHeight)),
    layersByNozzle,
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
