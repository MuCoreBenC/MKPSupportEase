/*
 * 预设（去标题 + 列随类型变 + 常驻操作列 + 两种「生效」分开）。
 *
 * # 版面
 *
 * ```
 * ┌MKP 配置│切片器配置┐ ┌本地│云端┐ [搜索…]                            ← 第一排（两档一致）
 * MKP   ┌● 已应用 A1.toml · A1 │ ▽ A1 ▾┐  共 4 项 │ 云端 9 · 本地 6    ← MKP 第二排
 * 切片器 ┌▽ 喷嘴 0.2 · 层高 0.12 │ ▽ A1 ▾┐  共 27 项 │ 仓库 27 · 本机 9 + 本地 5  ← 切片器第二排
 * 名称            机型   版本    时间   来源   操作
 * A1.toml         A1    标准版  09-14  云端   已应用
 * ├────────────────────────────────────────────┤
 * 右键任意一行还有…                                              ← 页脚（只剩右键提示）
 * ```
 *
 * **融合 pill（G07-1 收编，A44 三轮定稿；2026-10-04 进产品）**：原来的「机型下拉
 * （`PresetPicker`）+ 已应用状态条 + 定位按钮」三件并成一颗 PresetStatusPill ——
 * 左拍「● 已应用 A1.toml · A1」是信息（带「已应用」两个字，作者第二轮点名保留），
 * **点它 = 定位**：清掉机型筛选、把正在生效那行滚到眼前闪一下，不写「定位」两个字
 * （作者第三轮）；右拍「▽ A1 ▾」是**机型筛选**（漏斗菜单，有「全部机型」一档）——
 * 机型是**筛选**，不是切换（作者第二轮纠正：「这个是个筛选的……不能再用抽屉了」）。
 * 三个事实（在生效的是谁 / 怎么找回它 / 看哪个机型）融进一颗件，机型信息从此只出现
 * 这一次 —— `PresetPicker` 下拉整个退场。
 *
 * 切片器没有「已应用」，早先因此没做成 pill（机型筛单独住筛选区第一行）。
 * 作者 2026-10-10 裁决：「我希望做成 MKP 那样的」—— 切片器档也有了自己的那颗
 * （`PresetSlicerPill`）：左拍「▽ 喷嘴 0.2 · 层高 0.12」（点开同时给两组）、
 * 竖线、右拍机型筛选（同一个 `PresetMachineFilter`）。原先那三颗常驻下拉
 * （机型 / 喷嘴 / 层高；作者：「它 3 个筛选感觉很讨厌，特别讨厌」）随之退役；
 * 两档第二排从此同构：左一颗 pill、右边顶到右线的计数台账。
 *
 * **「撤销应用」恢复（作者 2026-10-10 裁决，推翻 2026-10-04 的那一条）**：正在使用的
 * 那一行，右键菜单里那一格**不再是灰掉的「在用」，而是可点的「取消使用」** ——
 * 作者：「它需要一个取消使用的按钮，而不是说这个灰色的在用」。走的还是照旧在的
 * `clear_active_preset`（`data.clearApply`）：撤下的只是"哪一份在生效"这个指向，
 * **磁盘上一个字节都不动**；撤完 pill 左拍回到「还没有使用任何预设」。
 *
 * **计数与台账的住处（2026-10-04 定稿）**：MKP 档恒住工具栏第二排（pill 右边，
 * 台账顶到右线）—— 工具栏恒两排：第一排 类型分段 / 位置分段(右) / 搜索，第二排
 * pill + 计数台账，不再随窗宽换摆法。页脚与右键提示同行；切片器的筛选排本来就满
 * （喷嘴 / 层高 chips），它的计数与台账任何档都住页脚。
 *
 * **「参数对比」的入口 = 右键两下（2026-10-10 作者规格）**：顶部那颗常驻按钮退役
 * （作者：「参数对比几个字，我们去掉了……答案是右键选择一个，然后再选择另外一个，
 * 就去对比」）—— 右键第一份点「参数对比」、再右键另一份点「与「X」对比」就开台，
 * 那两份直接选中（台子本身照旧：打开后还能勾第三份）。工具栏第二排从此只剩
 * pill + 台账两样，而且**这一排不折行**（`.tbRow`）：窄窗先收 pill 的左拍
 * （文件名省略号吸收），台账永远与 pill 同一行、顶到右线。pill 的宽**不封死**：
 * 随窗口宽窄变、也随文件名长短变（作者第三轮：「那个最大的长度宽度啊，它可以随着
 * 这个软件窗口的变化而变化，反正这个不能换行就是了」）；**右拍筛选一寸不让**。
 *
 * **页面里没有「预设」这两个字** —— 顶栏已经把「预设」高亮了，页面里再写一遍是重复。
 *
 * 上一版表格上方有**四条**横带（页头 / 机型版本 / 分段 / 提示），每条都 `flex-shrink: 0`，
 * 挤掉的全是表格 —— mini 档实测表格只剩 40px，表头自己就 34px，一行都露不出来。
 * 这一轮前三条压成一条工具条（窄档折两行），删掉的四样各有理由：
 *
 *   「预设」标题                 顶栏已经高亮了「预设」，重复
 *   副标题「本地的就是本地的…」  那是设计说明，不是数据。规则要靠界面本身说清
 *   「待下载 N 个」              它按机型算，而当前这张表是「MKP · 本地」—— 两个口径，
 *                                数字和眼前的表对不上，那就是噪音。云端表每行自己标
 *                                「未下载 / 已下载」，够了
 *   「演示：当成本地有」开关      作者说删
 *
 * 右上角那个数是**当前这张表的真计数**（筛后）：「共 N 项」。不是「本地 4 · 云端 9」那种
 * 两个口径并排 —— 眼前只有一张表，报第二张表的数只会让人去对一个看不见的东西。
 * 搜索框里原来还有一枚「命中 N 条」，和「共 N 项」是同一个数，删掉了：同一个数字写两遍，
 * 哪天算法改了就会有一处忘记跟。
 *
 * 它右边那格台账：**两个数跟着当前类型档走**（MKP 档数 MKP 的、切片器档数切片器的；
 * 图标 / 模型不归这一页，哪个数里都没有它们）—— 全 catalog 的数字混进某一档的语境里
 * 只会让人对不上（作者 2026-10-02 点名「仓库 9」）。MKP 档是「交付预设 · 我的」，
 * 切片器档是「仓库 · 本机 · 本地」。
 *
 * # 本地 / 云端是**两张互不相干的表**（两种类型都有这一对，2026-10-09 恢复）
 *
 *   本地表  我这台机器上真有的东西 —— 没有网也看得见、改得动、用得上。
 *           MKP：只有你自己那份本地文件（`presets-local/`）；切片器：云端副本 + 本地文件
 *   云端表  云端现在提供什么 —— MKP：目录里每一份预设一行，按血统说"我这边怎么样"
 *           （未获取 / 本地已有 / 有新版），动作是「下载」/「更新」（都**不改当前使用**）；
 *           切片器：菜单上的云端文件，行尾标「已下载 / 未下载」。仅归档的一处都不出现
 *
 * 两张表各回答一个问题，不合流。MKP 档里一个云端条目同时出现在两张表上不是重复：
 * 云端那一行说"云端有这个、我要不要拿"，本地那一行说"我手上这份能不能用"。
 *
 * # 两种类型的「生效」是两件不同的事
 *
 * ```
 * MKP     生效 = 设为当前配置       applyPreset()    已应用 / 未应用   操作 [应用]
 * 切片器   生效 = 复制到切片器目录   copyToSlicer()   已复制 / 未复制   操作 [复制]
 * ```
 *
 * 切片器 profile 光下到本机没用 —— 它得躺在切片器自己的 profile 目录里才生效。
 * 这两个写在**假后端只改内存，刷新页面还原**，所以提示条里把这一句说出来。
 *
 * # 这一页说的话，逐个交代来源
 *
 *   文件清单 / 路径 / 类型   `api.getPresetFiles()`                      真（20 个）
 *   套餐内 / 可单下          `PresetFileInfo.delivery`                   真（14 默认 / 6 可选）
 *   属于哪个机型版本         `api.getVersionFiles()` + `machineIds`      真（就是「版本」那一列）
 *   本机有哪些云端文件       `api.getLocalFiles()`                       **演示集合**（假后端没有文件系统）
 *   我自己的文件             `api.getLocalPresetFiles()`                  真（**本地文件**：扫 `presets-local/`，盘当底账）
 *   本地那份的正文           `api.readLocalPresetText()`                  真（只读；抽屉里看，读不出来照实说）
 *   已应用                   唯一底账 run/active-preset.json  真（**全局唯一**，落 Internal 根，刷新还在）
 *   已复制到切片器           `api.getSlicerCopied()`                     **演示集合**（起始 1 个）
 *   喷嘴 / 层高              `PresetFileInfo.{nozzle,layerHeight}`       真（**只有切片器有，所以只有切片器那张表里有这两列**）
 *   时间 / 大小              `PresetFileInfo.{modifiedText,sizeText}`    **假后端按 path 稳定推的演示值**，前端一行不编
 *                            （交付行的大小是**真值**：catalog 登记的那个字节数）
 *   与出厂不同 N 项          `getMachineParams` 里 origin === 'variant'  真（按已应用那个机型+版本算）
 *   暂不支持                 `VersionFiles.incomplete` + `missing[]`     真（只有 A2L）
 *   应用 / 复制              应用：唯一底账一个口（交付行合流，IPC 落 run/）
 *                            复制：`api.copyToSlicer()`，改假后端内存、刷新还原
 *   置顶                     localStorage `STORAGE.clientPresetsPinned`  **纯前端**，真的能用
 *   展开详情（点行）          上面那些字段的汇总                          **纯前端**，真的能用
 *   **下载 / 更新**（MKP 云端行） `api.fetchCloudPreset()`                真（取回云端当前版 + 落一份你的本地文件；
 *                            两个词是同一条路，差别只在本地有没有 / 换没换版。**不改当前使用**）
 *   **下载 / 更新**（交付行） `api.downloadCatalogFile()` → 下载管道          真（落 `mkp/`；**过程水位**照说，
 *                            需更新时走**同一条管道** —— 旧份自动归档，没有第二个命令）
 *   **批量**（云端表那一行）   **退役**（作者 2026-10-10：云端与本地一样，只逐份下载 / 更新）——
 *                            能力照旧在数据层（`fetchCloudPreset` / `downloadPresetBatch`），没有界面入口
 *   交付行的状态              `getDownloadedFiles` + `getStaleFiles`        真（三个读合起来才够四态：
 *                            + `getDeliveryTrust` —— 未下载 / 已下载 / 旧版本 / 内容异常，
 *                            见 `PresetFileState`）
 *   **编辑文件**（本地文件）  `api.beginTextEdit()` + `commitTextDraft()`  真（初稿 = **盘上那份文件的字节**：
 *                            `putTextDraft` 边改边存；保存 = **整份原子写回本地那份自己**（不产生第二份）。
 *                            **盘上那份直到点保存才被碰** —— 判据逐字节盯着；
 *                            参数页未保存的改动**不进来**（两条草稿分开存，2026-10-10））
 *   **重命名 / 删除**（本地文件） `api.renameLocalPreset()` / `deleteLocalPreset()` 真（第十层：只动名字，
 *                            字节一个不动；使用中指针与该份草稿跟着改名。删=真删，没有垃圾桶、没有归档；
 *                            正在使用 / 还有草稿的不给删 —— 原因原话来自后端）
 *   **复制**（本地文件）      `api.copyLocalPreset()`                        真（第十一层：本地文件 → 本地文件，
 *                            按字节复制、血统原样带过去；名字由程序起 `<原名> copy.toml`
 *                            （重名依次 `copy 2`…，见 `duplicateNameOf`）；不覆盖、不碰使用中指针与草稿）
 *   **导入（第十二层）**      `FileImportProvider`（App 层）               真（通用导入入口 ——
 *                            **拖拽进窗口**；重名开改名那一格。工具栏的「导入文件…」
 *                            按钮已退役（作者 2026-10-04：几乎不需要导入），选择器能力照旧在 App 层）
 *   **下载**（云端行）        `api.downloadFiles()`                       抛未实现，界面照实说（不编假进度条）
 *   **在 Finder 中显示**（本地文件） `api.revealInFolder()`                  真（第十三层：打开系统文件管理器**并选中**；
 *                            平台话术在 Windows 上是「在文件资源管理器中显示」。之后复制 / 压缩 / 发人随用户）
 *   **复制链接**
 *                            ——                                         **契约里连签名都没有**，就地说缺什么
 *
 * 数据与判定都在 `presetTree.ts`（纯函数）与 `usePresetData.ts`（三态加载 + 两张表），
 * 这个文件只管版面与动作。
 *
 * # 提示条那一格是约定
 *
 * 表格上方那一条：**以后所有提示与错误都走这个位置**，一次只显示一条，后来的替换前面的，
 * 带一个 × 手动关。不做自动消失 —— 「契约里还没有这个方法」这种话消失了就等于没说过。
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import { api, errorText } from '../../api'
import type { ArchivedFile, ParamDecision } from '../../api'
import { longStatText } from '../store/package'
/*
 * 这一页那几个框**统一用居中模态框**（作者 2026-10-10：「他全部都用这个右侧的抽屉，
 * 这完全不对，改成模态框」）。挂载点不变 —— 仍挂在 `.main` 里，遮罩只盖内容区，
 * 底栏与窗口右下角的 resize 手柄照旧点得着（Modal 自己那条约定）。
 */
import { Modal } from '../../components/modal'
/* 通用导入入口（第十二层，停在 App 层）：这一页消费它的 revision（拖拽导入照常生效） */
import { useFileImport } from '../import/useFileImport'
import { useDevPreview } from '../dev/useDevPreview'
/* 「正文草稿被别处作废」那一嗓子（参数页保存时发）：编辑器还开着就按新内容重开 */
import { lastTextDraftDroppedPath, useTextDraftDrop } from '../state/draftState'
import { FieldLayer } from '../../components/field'
import { ContextMenu, useContextMenu } from '../../components/menu'
import type { ContextMenuEntry } from '../../components/menu'
import type { Density } from '../../hooks/useDensity'
import { detectPlatform } from '../../hooks/usePlatform'
/* 下载过程与逐份结局的措辞（`shared/download.ts`）—— 同一件事一处文案 */
import { tickText } from '../shared/download'
/* 「复制后处理脚本」全站那一份（与标题栏 MKP 字标、首页那颗同一个出口）—— 见那个文件的文件头 */
import { copyPostProcessScript } from '../postprocess/copyScript'
/* 「右键某一份预设 → 打开它的参数」那一屏（2026-10-09）。它里面就是参数页本身 */
import PresetParamsModal from '../params/PresetParamsModal'
import CompareModal from './CompareModal'
import PresetScopeBar from './PresetScopeBar'
import PresetStatusPill, { PresetSlicerPill } from './PresetStatusPill'
import PresetTable from './PresetTable'
import {
  ARCHIVE_DRAWER,
  ARCHIVE_TIME,
  ARCHIVE_WHY,
  ASSET_USE_TEXT,
  /* 撞名那一格的文案（a04：不覆盖、不静默，问用户要不要导入 + 自动改名） */
  IMPORT_TAKEN_TEXT,
  /* MKP 云端行三态的按钮词与说明（与操作列那颗按钮同一条出处，2026-10-09） */
  CLOUD_LOCAL_ACTION,
  CLOUD_LOCAL_TEXT,
  CLOUD_LOCAL_WHY,
  EDIT_TEXT,
  LOCAL_COPY,
  LOCAL_DRAWER,
  LOCAL_NOT_PRESET_WHY,
  LOCAL_RENAME,
  duplicateNameOf,
  localUnreadableWhy,
  localCountOfAxis,
  treeCountOfAxis,
  PRESET_SUSPECT_WHY,
  STATUS_TEXT,
  UNSUPPORTED_TEXT,
  /* 「下载」那个词只有一处（`UPDATE_ACTION_TEXT`）—— 右键菜单与操作列那颗按钮不许各写一个 */
  UPDATE_ACTION_TEXT,
  isSuspectPreset,
  sizeTextOf,
  /* 重命名框：主名与后缀拆开（后缀灰色贴在行尾、不可改） */
  splitFileName,
} from './presetTree'
import type {
  LocateTarget,
  PresetKindAxis,
  PresetScopeAxis,
  PresetLocalRow,
  PresetTableRow,
} from './presetTree'
import { usePresetData, usePresetPage } from './usePresetData'
/* 正文的**语法着色**（自己写的词法，不引插件）—— 见那一份的文件头 */
import { tomlTokens } from './tomlPaint'
import s from './PagePresets.module.css'


interface Props {
  density: Density
  /**
   * 右键「在 BBS 预设查看器中打开」的出口。外壳（App）给的 ——
   * 切 tab 的状态住在那一层，这一页只负责把目标文件名交出去。
   */
  onOpenBbs?: (name: string) => void
  /**
   * 「此预设需要更新版 SupportEase」那句提示里「去更新」的出口：跳到设置页的软件更新块。
   *
   * ★ 跳转的出口由外壳给（页签状态住在那儿），这一页只说"我要去设置"。
   *   注意这条链是**读不懂数据**（要升级客户端），与设置页那条"有没有新版本"是两条链 ——
   *   这里只把用户送过去，不替设置页说话。
   */
  onOpenSettings?: () => void
  /**
   * 从工艺页跳过来（2026-10-13）：那一档里"还没下载"的几份点一下就落到这儿 ——
   * **带上一点意图**：切到「切片器配置 → 云端」那一栏（下载在那儿），
   * 不然用户落在一张跟下载无关的表上还得自己找。
   *
   * 与 `PageBbs` 那一侧的 `pending`（预设页 → 工艺页）同一个形状：`nonce` 管"再点一次也要重新触发"。
   */
  pending?: { nonce: number } | null
}

/* 四档同一句（与参数页搜索框同一条规矩）：跨任何分界，框里的字都不换 */
const PLACEHOLDER: Record<Density, string> = {
  ultra: '搜索文件…',
  wide: '搜索文件…',
  compact: '搜索文件…',
  mini: '搜索文件…',
}

/*
 * 「在文件管理器里显示」在这一页的话术（第十三层）：作者写的是「在 Finder 中显示」，
 * Windows 上那是文件资源管理器 —— 标签按平台换，动作同一条。
 *
 * ★ 2026-10-10 作者嫌长（「文案太长了」）：Windows 上从「在文件资源管理器中显示」
 *   压成「显示所在文件夹」（动作没变，只是不再把那台程序的全名印在按钮上），
 *   macOS 跟着压成「在访达中显示」。
 */
const REVEAL_LABEL =
  detectPlatform() === 'windows' ? '显示所在文件夹' : '在访达中显示'

/**
 * 着色 token → 类名。**一处映射**：词法在 `tomlPaint.ts`、颜色在这一页的 CSS，
 * 中间就这一张表（加一种 token 只动这三个地方）。
 */
const TOKEN_CLASS = {
  comment: s.tCmt,
  table: s.tTable,
  key: s.tKey,
  string: s.tStr,
  number: s.tNum,
  bool: s.tBool,
  punct: s.tPunct,
} as const

/**
 * 页面上那一句话：做了什么 / 缺什么。`bad` 的那一种是「没接上」，不是「操作失败」。
 *
 * `lines` 是**一次动作里逐份的结局**（批量下 3 份、2 份没成）：主句说总数，
 * 明细一行一份。为什么不用一条长句拼起来：那种句子没人读得完，而且拼起来之后
 * 「哪一份坏了」这唯一有用的信息就淹了。
 */
interface Note {
  text: string
  bad: boolean
  lines?: string[]
}

export default function PagePresets({ density, onOpenBbs, onOpenSettings, pending }: Props) {
  /*
   * 通用导入入口（第十二层）停在 App 层；这一页拿 `revision`
   * （导入落进 `presets-local/` 之后整屏重读，「本地文件」立刻以磁盘为准）。
   * 导入的入口是**拖拽**（把文件拖进窗口，App 层接）—— 工具栏上的「导入文件…」
   * 按钮已退役（作者 2026-10-04：几乎不需要导入）；选择器能力（pickFiles）
   * 照旧住在 App 层，只是这里不再有触发它的界面。
   */
  const imp = useFileImport()
  const data = usePresetData(imp.revision)
  const page = usePresetPage(data)

  /*
   * 工艺页跳过来那一下（见 `Props.pending`）：落到「切片器配置 → 云端」——
   * 那是"还没下载"的那几份被取回来的地方。顺手清掉机型筛选与搜索词，
   * 与 `locateRow` 同一个理由：**目标行被筛没就等于没把人送到位**。
   * 依赖用 `pending?.nonce`（与 BBS 页那一侧同一条口径）：同一个意图再点一次也要重新触发。
   */
  const presetsIntent = pending?.nonce ?? null
  /*
   * 依赖**只认那个 nonce**：`page` 那几个 setter 每帧都是新对象，把它们放进依赖就会
   * 变成"每次渲染都把档位按回切片器那一栏"—— 用户切回去的手会被按回来。
   */
  useEffect(() => {
    if (presetsIntent === null) return
    page.setKind('slicer')
    page.setScope('cloud')
    page.setQuery('')
    data.pickMachine('')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [presetsIntent])
  /*
   * **对比台的候选**：读得出来的「本地预设」（MKP 预设）。
   * 读不出来的那份不给 —— 比一份读不懂的文件只会得到一行"读不出来"；
   * 别的类型（`.json`）也不给：这一台子比的是参数。`useMemo` 是必需的（对比台按它重置选择）。
   */
  const compareFiles = useMemo(
    () => data.local.filter((f) => f.kind === 'mkp_preset' && f.state !== 'unreadable'),
    [data.local],
  )
  /* 可写：`mountPage` 这个 callback ref 要往回填（`useRef<T>(null)` 的 current 是只读的） */
  const rootRef = useRef<HTMLDivElement | null>(null)
  /*
   * **模态框的遮罩要盖住整页**（2026-10-10 作者：「遮罩覆盖了，除了最上面的标题、
   * 就是有红绿灯的那个标题除外，其他所有地方遮罩」）。
   *
   * 框自己不知道该怎么盖 —— 它只有 `absolute inset: 0`，盖多大取决于挂在谁里面。
   * 这一页的根（`.page`）正好就是"顶栏页签以下那一整块"（工具栏 + 内容 + 页脚），
   * 所以把这一层的元素交给 `Modal` 的 `host`，遮罩 portal 过去就刚好铺满那一块，
   * 而带红绿灯的标题栏在它上面、不被盖。
   */
  const [pageEl, setPageEl] = useState<HTMLDivElement | null>(null)
  const mountPage = (el: HTMLDivElement | null) => {
    rootRef.current = el
    setPageEl(el)
  }
  /** 「定位」的闪烁层：盖在被定位那一行上的普通 div（见 s.locateFlash 的注释） */
  const flashRef = useRef<HTMLDivElement>(null)

  /* 一个列表一个菜单。两张表同时只显示一张，所以一份就够 */
  const menu = useContextMenu<PresetTableRow>()
  /*
   * 展开详情的那一行（rowKey）。**点行就是展开** —— 右键菜单里那个「查看详情」
   * 2026-10-10 删掉了（作者：「既然我点击了它就是展开，那就没必要查看详情了呀，
   * 这些功能重复了」）。同一时刻只开一行 —— 与参数页的 expandedKey 同构。
   */
  const [expandedKey, setExpandedKey] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  /*
   * **参数对比台**（2026-10-08）：一个**独立常驻工具** —— 不挂在"云端版本"或"更新"上，
   * 也不因为有没有新版而变化。它只比**用户自己的本地预设**（`presets-local/`）。
   *
   * ★ 2026-10-10：**入口从顶部那颗常驻按钮搬进右键菜单**（作者：「参数对比几个字，
   *   我们去掉了……答案是右键选择一个，然后再选择另外一个，就去对比」）——
   *   右键第一份点「参数对比」记下它（`comparePick`），再右键另一份点
   *   「与「X」对比」就开台；这两份由 `comparePreselect` 直接选中。
   *   （台子本身照旧：打开后还能自己勾，最多三份。）
   */
  const [compareOpen, setCompareOpen] = useState(false)
  /** 打开对比台时要预选的两份（path）；`[]` = 台子按默认规则选前两份 */
  const [comparePreselect, setComparePreselect] = useState<string[]>([])
  /**
   * **对比的第一份**：右键点了「参数对比」的那一行。`null` = 还没选。
   * 记的是**路径**（改名不动它）；那份真被删了的话，开台前拿候选核一遍、忽略它。
   */
  const [comparePick, setComparePick] = useState<{ path: string; fileName: string } | null>(null)

  /* ---------------- 开发台（只在开发构建里收得到事件；见 `../dev/useDevPreview`） ----------------
   * 认领「重命名」与「参数对比台」两格。只摆出来给看，不动真业务
   * （框里的输入 / 按钮照旧走原来的 handler）。 */
  const devRename = useDevPreview('rename')
  useEffect(() => {
    if (!devRename.on) return
    /* 用一份**本机真有的**本地行（有就用真名，没有才摆一个只够画框的假 row）——
       真行的话"取消 / 确定"照旧能走真路，假 row 那一条被下面的守卫挡住 */
    const real = data.local[0]
    const row = (real ?? {
      kind: 'local',
      scope: 'local',
      origin: 'local',
      fileName: 'A1_MINI-fast.toml',
      path: 'presets-local/A1_MINI-fast.toml',
    }) as unknown as PresetTableRow
    setRenaming({
      row,
      name: splitFileName(row.fileName).stem,
      ext: splitFileName(row.fileName).ext,
      busy: false,
      error: null,
    })
    return () => setRenaming(null)
  }, [devRename.on, data.local])
  const devCompare = useDevPreview('compare')
  useEffect(() => {
    if (devCompare.on) setCompareOpen(true)
  }, [devCompare.on])
  /* 删除确认的认领：把右键菜单开在一条**能删的行**上、并一开就停到「删除」那一项的确认层
     （见 `ContextMenu` 的 `startConfirming`）。挑不到能删的行就什么都不做。
     ★ 拿 `page.local.rows`（= 下面的 `table`）—— 它在上面的 early return 之前就有了。 */
  const devDelete = useDevPreview('delete-confirm')
  useEffect(() => {
    if (!devDelete.on) return
    const row = page.local.rows.find((r) => removeWhyNot(r) === undefined)
    if (row === undefined) return
    menu.open(row, { x: 420, y: 260 })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [devDelete.on, page.local.rows])
  /*
   * **右键「打开参数」那一份**（2026-10-09）。`syncFirst` = 直接把「选择要跟随的更新」
   * 那一屏打开（用户点的是那条直路：他已经知道有云端更新等着处理）。
   */
  const [paramModal, setParamModal] = useState<{
    target: { path: string; fileName: string; machineId: string; versionId: string }
    syncFirst: boolean
  } | null>(null)
  /*
   * 正在做动作的那一行（rowKey）。只用来把那一颗按钮禁掉 —— 假后端是同进程的内存写，
   * 这一下快到看不见；真后端上「应用」要写盘，连点两次就会发两个写。
   * 不做全表遮罩：一行的动作不该把整张表锁住。
   */
  const [busyKey, setBusyKey] = useState<string | null>(null)
  /*
   * 批量入口 2026-10-10 退役（作者：「这个可以去掉的……跟本地的一样就可以」）：
   * 云端表的呈现与本地表一致，只在行上逐份下载 / 更新，不再有整批按钮。
   * 能力照旧在数据层（`fetchCloud` / `downloadPresetBatch`），只是没有界面入口。
   */

  /*
   * **只读正文抽屉**在看哪一份。`null` = 关着。
   *
   * 两边共用一个抽屉，因为它俩要的是同一件事：取出一份文本、只读地看。
   *   `archive` 云端那一侧的旧版本（先列出一份文件的那几个旧版本，再挑一份看）
   *   `local`    本地文件的那一份（就一份，打开直接读）
   *
   * 正文单独一个状态：读它要问后端（前端不碰文件系统），而且**会失败** ——
   * 失败要如实说，不许显示一段空正文假装它是空的。
   */
  const [viewer, setViewer] = useState<
    | { kind: 'archive'; fileName: string; rows: ArchivedFile[] }
    | { kind: 'local'; fileName: string; path: string }
    | null
  >(null)
  /*
   * 归档删除的**两段式确认**（抽屉里没有菜单那套 confirm 机制）：
   * 记"哪一份的删除按钮已经点过第一下"。点到别的份 / 关抽屉都退回。
   */
  const [confirmingArchive, setConfirmingArchive] = useState<string | null>(null)
  const [body, setBody] = useState<{
    path: string
    text: string | null
    error: string | null
    loading: boolean
  } | null>(null)

  /*
   * **编辑器**（「编辑文件」那条链）。`null` = 没在改。
   *
   * 初稿是**盘上那份文件的字节**（参数页未保存的改动不在里面 —— 两条草稿分开存，
   * 见契约 `PresetTextDraft`）；编辑期间只动**正文草稿**，保存那一刻才整份写回。
   * `draftError` 是"草稿自动落盘失败了"那一行：它安静地待在抽屉里，不占提示条
   * （用户可能还在打字，提示条会闪）。
   */
  const [editing, setEditing] = useState<{
    /** 改的是哪一份（文件名）—— 副标题与提示读它 */
    fileName: string
    /** 盘上那份的落点（相对用户根）—— 保存提示里"写回哪去了"读它 */
    path: string
    text: string
    draftError: string | null
  } | null>(null)
  /** 上一次真正落到临时文件里的正文。用它判断"值不值得再存一次" */
  const savedTextRef = useRef<string>('')

  /*
   * **撞名那一格**（a04）：云端那份要落下来时，本机已经有一份同名的了 ——
   * 不覆盖、也不静默丢掉，问用户"要不要导入"；同意就**自动改一个没被占的名字**
   * 落一份新的（原来那份一个字节不动）。`null` = 没有在问。
   */
  const [takenAsk, setTakenAsk] = useState<{
    fileName: string
    existingPath: string
    suggestedName: string
    busy: boolean
  } | null>(null)

  /*
   * ★ 2026-10-11（a04）：**「更新」不再弹确认层**。
   *
   *   点「更新」= 我没改过的项静默跟上新版；**我改过的项一个字节不动** ——
   *   它们留给「修改参数」那条杠（云端改了 N 项 —— 你改过的不会被顶掉）逐项处理：
   *   行内「用新值 / 保持本地」、或杠上的「逐项挑选」。
   *
   *   为什么不弹框问（作者在 lab 原型里定稿的口径，见 `lab/preset-identity-and-update-spec.md` §4）：
   *   更新是"把云端的东西拿下来"这一件事，不该在点它的时候打断用户做决定；
   *   而"我改过的那几项要不要动"是**改参数**的时候才该回答的问题 —— 所以那件事在参数页。
   *   a03 的那一格（`PresetUpdateModal`）在 a04 已删。
   */
  /** 正文下面那一层**着色**（`<pre>`）：滚动时由 textarea 把它一起搬 */
  const paintRef = useRef<HTMLPreElement>(null)
  /*
   * 正文的着色结果（行 → token）。只在正文变了才重算 —— 每敲一个字就重新分词，
   * 对几十行的预设来说便宜得很，但没必要每帧都做。
   */
  const paintedLines = useMemo(
    () => (editing === null ? [] : tomlTokens(editing.text)),
    [editing],
  )

  /*
   * **重命名那一口框**（第十层）。2026-10-10 起这里只剩它一件事 ——
   * 「另存为一份新的」变成了右键菜单里的**「复制」**：名字由程序起
   * （`duplicateNameOf`），点一下就得一份，不再弹框问名字。
   *
   * ★ 2026-10-10 又一轮（作者：「它不要显示这个后缀，后缀我不希望它能改……
   *   把后缀灰色放到右侧，吸附在右边」）：输入框里**只装主名**（`name`），
   *   后缀（`ext`，含点）单独拆出来、灰色贴在行尾 —— 提交时原样拼回去。
   *   框里那两段说明也撤了（作者：「只改名字，这些文案不需要」「他正在使用，
   *   也不需要」）—— 这一格只剩「打名字 + 确定」。
   *
   * 名字的门槛（空 / 路径 / 后缀 / 不覆盖）仍全在后端，这里不重复判断；
   * 失败原话留在这口框里（不弹提示条，别把用户刚打的字顶掉）。
   */
  const [renaming, setRenaming] = useState<{
    row: PresetTableRow
    /** **主名**（不含后缀）—— 那一枚后缀单独在 `ext` 里、不可改 */
    name: string
    /** 后缀（含点，如 `.toml`；拆不出来就是空串）。提交时原样拼回去 */
    ext: string
    busy: boolean
    error: string | null
  } | null>(null)



  /*
   * 「定位」的闪光本体：把目标行滚到视口中央，然后盖上 overlay 亮一下再退光。
   * 两个入口共用（状态条的「定位已应用」与来源格的「复制自 X → 定位」）。
   *
   * 闪烁本体是 `s.locateFlash` 那块**盖在行上的普通 div**：现量现设位置，
   * Web Animations API 淡出（1.8s、先停在 55%）后隐藏。**不许**给行本身挂 class
   * 跑 keyframes —— 这张折叠边框表在 tr/td 背景上跑动画，适配缩放下合成层缓存
   * 会留旧帧，行里就多出一条若隐若现的白带（作者：「有时候窗口比较矮就没有，
   * 比较高就出现」）。div 的终态是 display:none，缓存与否无关紧要。
   * 闪的 1.8s 里用户要是滚动了页面，这块 div 不跟着走 —— 一次 1.8 秒的瞬态效果，接受。
   */
  const flashRow = (row: HTMLElement) => {
    const root = rootRef.current
    const overlay = flashRef.current
    if (!root || !overlay) return
    /* 即时滚（不用 smooth）：无头/低帧率环境下 smooth 可能一帧都不跑，等于没滚 */
    row.scrollIntoView({ block: 'center' })
    const rr = row.getBoundingClientRect()
    const pr = root.getBoundingClientRect()
    overlay.style.left = `${rr.left - pr.left}px`
    overlay.style.width = `${rr.width}px`
    overlay.style.top = `${rr.top - pr.top}px`
    overlay.style.height = `${rr.height}px`
    overlay.style.display = 'block'
    overlay.getAnimations().forEach((a) => a.cancel())
    overlay
      .animate(
        [
          { opacity: 1 },
          { opacity: 1, offset: 0.55 },
          { opacity: 0 },
        ],
        { duration: 1800, easing: 'ease-out' },
      )
      .onfinish = () => {
        overlay.style.display = 'none'
      }
  }

  /*
   * 状态条上的「定位」。
   *
   * 全局只有一个「已应用」，机型筛选可能正好把它筛没了 —— 作者不愿意看到
   * 「筛完之后不知道哪套在生效」。点它清掉机型筛选（已应用一定在本地 MKP 表里），
   * 等重画完把那一行滚到视口中央。它替代了原来页脚那个「已应用 … →」按钮：
   * 客户端不知道「基底 / 出厂」这些工作台的概念，页脚那句话整个搬走了。
   */
  const locateApplied = () => {
    menu.close()
    page.setKind('mkp')
    page.setScope('local')
    data.pickMachine('')
    window.setTimeout(() => {
      const row = rootRef.current?.querySelector('tr[data-live="true"]') as HTMLElement | null
      if (row !== null) flashRow(row)
    }, 60)
  }

  /*
   * **来源定位**（「复制自 X」那一格点过来）：切到来源所在的表（local / cloud、
   * 必要时换类型档）、清掉机型筛选与搜索词（不然目标行可能被筛没）、展开那一行、
   * 滚过去亮一下 —— 用户要的是"看见我从哪复制来的"，缺一步都到不了那个效果。
   *
   * 行键按「全部机型」那一档算（local 行的键里带机型筛选，见 `originCellOf`）；
   * 类型轴只在目标说得出类型时才切（认不出类别的那份两张表里都有，不动当前档）。
   * 60ms 与 `locateApplied` 同一个数：等 React 把换轴 + 清筛选的重画落完盘再找行。
   */
  const locateRow = (target: LocateTarget) => {
    menu.close()
    page.setScope(target.scope)
    if (target.kind !== null) page.setKind(target.kind)
    data.pickMachine('')
    page.setQuery('')
    setExpandedKey(target.rowKey)
    window.setTimeout(() => {
      const row = rootRef.current?.querySelector(
        `tr[data-rowkey="${CSS.escape(target.rowKey)}"]`,
      ) as HTMLElement | null
      if (row !== null) flashRow(row)
    }, 60)
  }

  /**
   * 云端那份 MKP 条目在**本地表**里那一行的行键（还没下载 ⇒ `null`）。
   *
   * 云端行的键认 `fileName`、本地文件行的键认**路径**，中间那道转换只有 `localIndex`
   * 给得出（`cloudSource` 就是"从云端哪条下来的"）—— 这正是云端行那三态认亲的同一本账。
   * 行键按「全部机型」那一档算，因为 `goLocal` 会把机型筛选清掉。
   */
  const localFileRowKey = (fileName: string): string | null => {
    const hit = data.localIndex.find((e) => e.cloudSource === fileName)
    return hit === undefined ? null : `local::${hit.path}`
  }

  /*
   * **云端行 → 本地表**（2026-10-16 作者：「云端的右键菜单需要一个切换到本地的，
   * 懒得点那个本地的切换」）。
   *
   * ★ **只是切过去，不展开详情**（作者当天补的第二句：「还有切换到本地的，
   *   不要展开详细」）—— 上一版顺手 `setExpandedKey`，结果切过去那一行是炸开的，
   *   一屏里多出一大块他没要看的详情。所以下面**没有** `setExpandedKey`，
   *   只滚过去 + 闪一下：那一下闪光已经足够回答"是这一行"。
   *
   * 另两件事，与上面「来源定位」同一套手势：
   *   ① 切到本地表；
   *   ② 清掉机型筛选与搜索词 —— 不清的话目标行可能正好被筛掉，切过去看见一张空表，
   *      那比不切更让人糊涂。
   *
   * 滚过去那一行要先认出它，账本与云端行那三态是**同一本**（`localIndex`）：
   * MKP 档的本机副本是「本地文件」行，键形如 `local::<路径>`（`localFileRowKey`）；
   * 切片器档那一行在本地表里与云端**同形**（都是 `<机型>:<路径>`），原样带过去。
   * **认不出就不滚**（还没下载、正好不在列表里）—— 列表照常在那儿，不猜一个行键。
   */
  const goLocal = (row: PresetTableRow) => {
    menu.close()
    page.setScope('local')
    data.pickMachine('')
    page.setQuery('')
    const rowKey = page.kind === 'mkp' ? localFileRowKey(row.fileName) : `${row.machineId}:${row.path}`
    if (rowKey === null) return
    window.setTimeout(() => {
      const hit = rootRef.current?.querySelector(
        `tr[data-rowkey="${CSS.escape(rowKey)}"]`,
      ) as HTMLElement | null
      if (hit !== null) flashRow(hit)
    }, 60)
  }

  /*
   * 切机型 / 分段之前先关菜单：它指向的那一行可能已经不在了，
   * 停在一个不存在的行上的菜单，点哪一项都是在对空气动手。
   */
  const pickMachine = (machineId: string) => {
    menu.close()
    data.pickMachine(machineId)
  }


  const setKind = (next: PresetKindAxis) => {
    menu.close()
    page.setKind(next)
  }

  const setScope = (next: PresetScopeAxis) => {
    menu.close()
    page.setScope(next)
  }

  // ——————————————————————————————————————————————————————————
  // 菜单里的动作
  // ——————————————————————————————————————————————————————————

  /**
   * **复制链接**（2026-10-09 接通）：交付预设下载链接由后端按**当前数据源**算 ——
   * 与下载管道同一个寻址出口，复制到的链接就是下载时取的那个地址。
   * 拿到就进剪贴板；没配数据源 / 算不出来照实说，不编一个假 URL。
   */
  const copyLink = (row: PresetTableRow) => {
    api.getFileUrl(row.fileName).then(
      (url) => {
        void navigator.clipboard.writeText(url).then(
          () => setNote({ text: `已复制 ${row.fileName} 的下载链接`, bad: false }),
          (e: unknown) => setNote({ text: `链接拿到了，但进剪贴板没成功：${errorText(e)}`, bad: true }),
        )
      },
      (e: unknown) => setNote({ text: `取不到下载链接：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 云端行的「更新」：**把云端新版处理到本地这份上**（2026-10-10 作者规格）。
   *
   * 三步，**不弹任何框**：
   *   ① 取回云端当前版字节（本地那份一个字节不动 —— 只是取来比较）；
   *   ② 读逐参数三方账（`presetSync`）：云端改了哪些、其中哪些**我也改过**（= 冲突）；
   *   ③ **我没改过的项静默跟上**（一次性写进本地文件）；**我改过的项一个字节不动** ——
   *      留给「修改参数」逐项处理（那边的杠与行内按钮就是为这件事做的）。
   */
  const runUpdate = (row: PresetTableRow) => {
    if (row.scope !== 'cloud') return
    const fileName = row.fileName
    setBusyKey(row.rowKey)
    setNote({ text: `正在更新 ${fileName}…`, bad: false })
    void (async () => {
      /*
       * ① 更新的是**本地那一份**：血统指向这个云端条目、而且还没跟上当前版的
       * （同一云端条目下我可能有好几份 —— 取列表里第一份落后的）。
       * **先读一遍本地列表**：更新对象要在动手之前定下来。
       */
      const [localPresets, index] = await Promise.all([
        api.getLocalPresetFiles(),
        api.getLocalPresetIndex(),
      ])
      /*
       * 更新的是**本地那一份**：它跟这个云端条目的关系**只看身份账本**
       * （`cloudSource` 对上它），而且还没跟上当前版（`basedOn !== 'current'`）。
       * 同一云端条目下我可能有好几份（比如下载过两次、各改各的）—— 取列表里第一份落后的。
       * **复制出来的副本不在账本里**，所以永远不会被当成"要更新的那一份"。
       */
      const indexByPath = new Map(index.map((e) => [e.path, e]))
      const behind =
        localPresets.find((f) => {
          const entry = indexByPath.get(f.path)
          return (
            f.kind === 'mkp_preset' &&
            entry !== undefined &&
            entry.cloudSource === fileName &&
            f.basedOn !== 'current'
          )
        }) ?? null
      if (behind === null) {
        setBusyKey(null)
        setNote({ text: `云端 ${fileName} 这一版没有要处理的（你机器上那几份都跟得上）`, bad: false })
        return
      }
      /*
       * ② 读逐参数三方账。`fetchMissing: true` 允许后端**顺手把云端新版字节取回来**
       * （内部基准，**不动本地这份文件**）—— 所以这里不需要先「下载」一遍。
       */
      const sync = await data.presetSync(behind.path, true)
      setBusyKey(null)
      if (!sync.cloudReady) {
        setNote({
          text: `更新 ${fileName} 没成：云端新版的字节还没取回来（网络或数据源的问题）`,
          bad: true,
        })
        return
      }
      const pending = sync.entries.filter((e) => e.pending)
      const conflicts = pending.filter((e) => e.conflict)
      const auto: ParamDecision[] = pending
        .filter((e) => !e.conflict)
        .map((e) => ({ paramKey: e.key, kind: 'adopt' }))
      if (pending.length === 0) {
        setNote({ text: `已更新 ${behind.fileName} —— 云端这一版没有要处理的参数差异`, bad: false })
        return
      }
      /*
       * ③ 写盘：**只写"我没改过"的那些项**（静默跟上新版）；我改过的一项都不写 ——
       *    它们留给「修改参数」逐项处理（那边看得到"云端 X → Y"与「用新值 / 保持本地」）。
       *    没有可跟的项时**一个字节都不动**（不必白写一遍）。
       */
      if (auto.length > 0) await data.applyParamDecisions(behind.path, auto)
      const parts: string[] = []
      if (auto.length > 0) parts.push(`${auto.length} 项跟上了新版`)
      if (conflicts.length > 0) parts.push(`${conflicts.length} 项你改过 —— 去「修改参数」处理`)
      setNote({ text: `已更新 ${behind.fileName} —— ${parts.join('；')}`, bad: false })
    })().catch((e: unknown) => {
      setBusyKey(null)
      setNote({ text: `更新失败：${errorText(e)}`, bad: true })
    })
  }

  /**
   * 撞名那一格点「导入」：再来一次，这次允许**自动改名** ——
   * 落一份新的（`A1-fast-2.toml`），原来那份一个字节不动，两份都留着。
   */
  const confirmTakenImport = () => {
    if (takenAsk === null) return
    const ask = takenAsk
    setTakenAsk({ ...ask, busy: true })
    data.fetchCloud(ask.fileName, true).then(
      (done) => {
        setTakenAsk(null)
        setNote({
          text: `已导入 ${done.fileName} —— 原来那份没动，两份都留着（本地表里现在有它）`,
          bad: false,
        })
      },
      (e: unknown) => {
        setTakenAsk((cur) => (cur === null ? cur : { ...cur, busy: false }))
        setNote({ text: `导入没成：${errorText(e)}`, bad: true })
      },
    )
  }

  /**
   * 云端表那颗按钮。**两条路，都是"把云端的东西落到我机器上"**（2026-10-10 按「本地 / 云端」改口径）：
   *
   *   MKP 云端行「下载」（本地还没有）  → `fetchCloud`：取回云端当前版 + 落一份**本地文件**；
   *   MKP 云端行「更新」（本地已有、云端有新版的）→ `runUpdate`：取回新版 + 逐项比较，
   *       没冲突的按新版采用、冲突的让用户选，**确认后才一次性写回本地文件**；
   *   切片器 / 交付预设（`presetUid` 在） → `downloadPreset(fileName)`：落下载区 `mkp/`；
   *   云端仓库的文件                 → 契约里有签名，所以**照调**。
   *     假后端一定抛 `NotImplementedError`（自带人话 hint，A2），界面接住原样显示
   *     —— 不许整页白屏，也不许静默吞掉（吞掉就等于把「哪个口子没接」藏起来）
   */
  const download = (row: PresetTableRow) => {
    if (row.scope === 'cloud' && row.localState !== undefined) {
      /* 「更新」≠ 只把云端取回来：要把差异处理到本地这份上（见 runUpdate） */
      if (row.localState === 'stale') {
        runUpdate(row)
        return
      }
      /* 「下载」：本地还没有这一份 —— 取回云端当前版并落一份本地的 */
      setBusyKey(row.rowKey)
      setNote({ text: `正在下载 ${row.fileName}…`, bad: false })
      data.fetchCloud(row.fileName).then(
        (done) => {
          setBusyKey(null)
          /*
           * ★ a04：**同名已经有一份了** —— 不覆盖、也不静默：问用户
           * （同意就自动改一个没被占的名字落一份新的，原来那份一个字节不动）。
           */
          if (done.takenBy !== undefined) {
            setTakenAsk({
              fileName: row.fileName,
              existingPath: done.takenBy,
              suggestedName: done.suggestedName ?? row.fileName,
              busy: false,
            })
            return
          }
          setNote({
            /* 一句大白话就够（2026-10-13 作者：**技术细节不必让用户知道**）——
               "云端字节进了下载区、另落一份你自己那份"是我们内部的两件事，
               用户要的是"这下我能用它了" */
            text: `已下载 ${done.fileName} —— 本地表里现在有它了`,
            bad: false,
          })
        },
        (e: unknown) => {
          setBusyKey(null)
          /* 断网 / 取不回来照实说：不假装已经可以使用（它确实还不能用） */
          setNote({ text: `下载失败：${errorText(e)}`, bad: true })
        },
      )
      return
    }
    if (row.presetUid !== undefined) {
      /*
       * 盘上那份不对劲的两档（旧版本 / 认不出）走的是**同一条下载管道**（再下一遍，
       * 旧份自动归档）—— 动词统一叫「更新」（与按钮一致：云端有更新就该说更新，
       * 「重新下载」那种吓唬人的说法不再出现在动作上）；认不出的实情在结果那句话里说。
       * 别在这里分支去找"另一个命令"：没有那个命令。
       */
      const updating = row.presetState === 'tampered' || row.presetState === 'old'
      const verb = updating ? '更新' : '下载'
      setBusyKey(row.rowKey)
      setNote({ text: `正在${verb} ${row.fileName}…`, bad: false })
      /* 过程如实说：一次调用一路水位，后端推到哪说到哪 —— 不编一个分母，也不转空圈 */
      data
        .downloadPreset(row.fileName, (t) => setNote({ text: tickText(t), bad: t.stage === 'failed' }))
        .then(
          () => {
            setBusyKey(null)
            setNote({
              text: updating
                ? row.presetState === 'tampered'
                  ? `已更新 ${row.fileName} —— 盘上那份认不出的，现在换成了目录登记的当前版本`
                  : `已更新 ${row.fileName} —— 旧的那一份进了归档（archive/），没有删`
                /*
                 * 大白话（2026-10-13 作者：**技术细节不必让用户知道**）——
                 * 落点路径不印在提示条上（要看文件在哪，行上右键有「显示我们那份的文件夹」）；
                 * **"要进切片器还得点复制"这条指引留着**：那是用户真要做的下一步。
                 */
                : `已下载 ${row.fileName} —— 本地表里现在有它了；要进切片器再点「复制」`,
              bad: false,
            })
          },
          (e: unknown) => {
            setBusyKey(null)
            setNote({ text: `${verb}失败：${errorText(e)}`, bad: true })
          },
        )
      return
    }
    /*
     * 到这里只剩「云端仓库文件」那一类（没有 MKP 的 `localState`、也没有交付身份）。
     * 它的「下载」残支 2026-10-09 按用户裁断**连入口一起撤了**（`downloadFiles`
     * 从契约里删掉）—— 界面上这类行不再给下载按钮，走到这里说明按钮闸漏了。
     */
    setNote({ text: `${row.fileName}：这一份不走「下载」—— 只有登记过的交付文件能取`, bad: true })
  }

  /**
   * 读一份正文（只读）。**两边共用这一个口子**，只是问的命令不同：
   * 归档区那份走 `readArchivedText`，用户自己那份走 `readLocalPresetText`。
   * 失败照实说（与下载 / 应用同一条规矩：不吞、不装）。
   */
  const readBody = (path: string, which: 'archive' | 'user') => {
    setBody({ path, text: null, error: null, loading: true })
    const ask = which === 'user' ? api.readLocalPresetText(path) : api.readArchivedText(path)
    ask.then(
      (text) => setBody({ path, text, error: null, loading: false }),
      (e: unknown) =>
        setBody({
          path,
          text: null,
          error: errorText(e),
          loading: false,
        }),
    )
  }

  /**
   * 打开「旧版本」抽屉：按**文件名**把归档里那一份的旧版本挑出来。
   *
   * 判据用文件名 —— 与下载 / 应用 / 读正文同一套口径（归档里那份与交付文件同名，
   * 换版本换的是字节不是名字）。
   */
  const openArchive = (row: PresetTableRow) => {
    menu.close()
    /* 两个抽屉互斥（草稿已经在盘上，"接着改"能回来，所以关掉编辑器不丢东西） */
    setEditing(null)
    setBody(null)
    setViewer({
      kind: 'archive',
      fileName: row.fileName,
      rows: data.archived.filter((a) => a.fileName === row.fileName),
    })
  }

  /**
   * 打开本地文件那一份的正文（「看正文」）。就一份文件，打开直接读 —— 没有列表这一层。
   *
   * **这一层只能看**：改它要先经过「改这份」那条链（那会另存出一份自己的）。
   */
  const openLocal = (row: PresetTableRow) => {
    menu.close()
    setEditing(null)
    setViewer({ kind: 'local', fileName: row.fileName, path: row.path })
    readBody(row.path, 'user')
  }

  /**
   * **开始改这一份**：让后端把**盘上那份文件的字节**读出来当初稿，然后把编辑器打开。
   *
   * **编辑的对象只有「本地预设」**（`presets-local/` 里那份）—— 云端那一份是模板、只读，
   * 要改它先在云端「下载」一份自己的。
   *
   * ★ 2026-10-10 起**参数页那份草稿不进来**（作者裁定）：这边看到的就是文件本身，
   * 参数页改了几处没保存也不影响它 —— 两条草稿分开存（契约 `PresetTextDraft`）。
   * 前置条件全在后端拦（读得出那份文件），这里不重复判断。同一份的**正文草稿**还在的
   * 话后端会返回它（`reused`），于是"改到一半关掉再回来"接着改。
   */
  const openEdit = (row: PresetTableRow) => {
    menu.close()
    setViewer(null)
    data.beginTextEdit(row.fileName, row.path).then(
      (draft) => {
        savedTextRef.current = draft.text
        setEditing({
          fileName: draft.fileName,
          path: draft.path,
          text: draft.text,
          draftError: null,
        })
      },
      (e: unknown) => {
        setNote({
          text: `改不了 ${row.fileName}：${errorText(e)}`,
          bad: true,
        })
      },
    )
  }

  /*
   * 边改边存：停下 700ms 才落一次（在打字的中间态里反复写盘没有意义）。
   * 落盘的是**正文草稿** —— 盘上那份文件一动不动，这也是"改到一半关掉还在"的来源。
   * 失败写进抽屉里那一行（不弹提示条：用户还在打字，别拿一条会闪的条子打断他）。
   */
  useEffect(() => {
    if (editing === null || editing.text === savedTextRef.current) return
    const text = editing.text
    const timer = window.setTimeout(() => {
      data.putTextDraft(text).then(
        () => {
          savedTextRef.current = text
        },
        (e: unknown) =>
          setEditing((cur) =>
            cur === null
              ? cur
              : { ...cur, draftError: errorText(e) },
          ),
      )
    }, 700)
    return () => window.clearTimeout(timer)
  }, [data, editing])

  /*
   * **这份刚在「修改参数」里保存过 —— 编辑区按盘上最新内容重开**（2026-10-10）。
   *
   * 参数页保存会把后端那份**正文草稿**作废（盘上字节换了，契约里写死的）。
   * 编辑器如果还开着，手上的草稿就成了孤儿：再改一个字撞「没有正在改的那一份正文」，
   * 点保存也存不上（作者实测到过）。收到这一声就把编辑区按**新的文件内容**重开 ——
   * 这边没保存的改动就此作罢（提示条说明；与参数页那边"被别人保存后自己收拾"
   * 是同一件事的两面）。
   */
  const textDropRev = useTextDraftDrop()
  const seenTextDropRef = useRef(0)
  useEffect(() => {
    if (textDropRev === seenTextDropRef.current) return
    seenTextDropRef.current = textDropRev
    const dropped = lastTextDraftDroppedPath()
    if (dropped === null) return
    if (editing === null || editing.path !== dropped) return
    void data.beginTextEdit(editing.fileName, dropped).then(
      (draft) => {
        savedTextRef.current = draft.text
        setEditing((cur) => (cur === null ? cur : { ...cur, text: draft.text, draftError: null }))
        setNote({ text: '这份刚在「修改参数」里保存过 —— 编辑区已按最新内容重开', bad: false })
      },
      (e: unknown) => setNote({ text: `重开不了：${errorText(e)}`, bad: true }),
    )
  }, [data, editing, textDropRev])

  /** 放弃这次编辑：丢正文草稿（盘上那份文件全程没被碰过，所以它天生安全） */
  const discardEdit = () => {
    data.discardTextDraft().then(
      () => {
        setEditing(null)
        setNote({ text: '已放弃这次编辑 —— 你的那份从头到尾没有被改过', bad: false })
      },
      (e: unknown) =>
        setNote({ text: `放弃不了：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 保存：**写回它自己**（第八层：同一个文件，不会多出一份）。
   *
   * 写的是**整份替换**（后端先写临时文件再换上去 —— 原子写入），
   * 而且只有这一下才碰盘：没点保存时，改到哪都只在正文草稿里。
   */
  const commitEdit = () => {
    data.commitTextDraft().then(
      (done) => {
        setEditing(null)
        setNote({ text: EDIT_TEXT.savedBack(done.fileName, done.path), bad: false })
      },
      (e: unknown) =>
        setNote({
          text: `没存上：${errorText(e)}`,
          bad: true,
        }),
    )
  }

  /**
   * **重命名我自己那一份**（第十层）：只改名字，**字节一个不动**。
   *
   * 菜单里点进来先开这一口抽屉（初值 = 现在的文件名），确定才交给后端 ——
   * 名字的门槛与"使用中指针、这一份的草稿跟着改名"都在那里（页面不重复判断）。
   */
  const openRename = (row: PresetTableRow) => {
    menu.close()
    setViewer(null)
    setEditing(null)
    /* 初值 = 现在的主名（后缀拆出去、贴在行尾 —— 它不可改，见 `renaming` 的注释） */
    const { stem, ext } = splitFileName(row.fileName)
    setRenaming({ row, name: stem, ext, busy: false, error: null })
  }

  /**
   * **复制出一份新的**（第十一层，2026-10-10 改判）：我的文件 → 我的文件，按字节复制。
   *
   * 名字**由程序起**（`duplicateNameOf`：`<原名> copy.toml`，重名依次 `copy 2`…）——
   * 作者 2026-10-10：「主要是它的文案太长了……要不然就叫复制，就自动复制出一份」，
   * 所以这一层**不再弹框问名字**（原来那口"起名字抽屉"的 copy 分支整个退场）。
   * 别的规矩没变：血统原样带过去、不覆盖、不碰使用中指针与草稿。
   */
  const runDuplicate = (row: PresetTableRow) => {
    menu.close()
    const name = duplicateNameOf(
      row.fileName,
      data.local.map((f) => f.fileName),
    )
    setNote({ text: `正在复制 ${row.fileName}…`, bad: false })
    data.copyAsNew(row.path, name).then(
      (done) =>
        setNote({
          text: `已复制出一份：${done.fileName} —— 原文件一个字节没动`,
          bad: false,
        }),
      (e: unknown) => setNote({ text: `没复制成：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 重命名那一口框里那颗按钮。
   *
   * 提交的是**主名 + 拆出来那一枚后缀**（原样拼回去）—— 输入框里只有主名，
   * 后缀从头到尾没得改。防一手"粘贴进来的名字自带同一个后缀"
   * （`x.toml` 贴进输入框再拼一次会得到 `x.toml.toml`）：先剥掉再拼。
   */
  const submitRename = () => {
    /* 开发台预览：这一格也不是真的在改名字 —— 「确定」只把它关掉 */
    if (devRename.on) {
      devRename.close()
      setRenaming(null)
      return
    }
    if (renaming === null || renaming.busy) return
    let stem = renaming.name.trim()
    if (stem === '') {
      setRenaming({ ...renaming, error: '新名字不能是空的' })
      return
    }
    if (renaming.ext !== '' && stem.toLowerCase().endsWith(renaming.ext.toLowerCase())) {
      stem = stem.slice(0, stem.length - renaming.ext.length).trim()
      if (stem === '') {
        setRenaming({ ...renaming, error: '新名字不能是空的' })
        return
      }
    }
    const name = `${stem}${renaming.ext}`
    setRenaming({ ...renaming, busy: true, error: null })
    data.rename(renaming.row.path, name).then(
      (done) => {
        setRenaming(null)
        setNote({
          text: `已改名：${renaming.row.fileName} → ${done.fileName} —— 只换了名字，内容与血统一个字节没动`,
          bad: false,
        })
      },
      (e: unknown) =>
        setRenaming((cur) =>
          cur === null ? cur : { ...cur, busy: false, error: errorText(e) },
        ),
    )
  }

  /**
   * **删除**（2026-10-06 一切皆可删）：我的文件走本地文件（**真删除**）；
   * 切片器交付行走交付线（删了回「未下载」，随时可从云端重下）。
   * 二次确认长在菜单里（`danger` + `confirm`）；正在使用 / 有草稿不再拦 ——
   * 后端把属于这一份的状态一并清掉（确认框讲清了）。
   */
  const runRemove = (row: PresetTableRow) => {
    setNote({ text: `正在删除 ${row.fileName}…`, bad: false })
    const done =
      row.origin === 'preset' ? data.removePreset(row.fileName) : data.remove(row.path)
    done.then(
      () =>
        setNote({
          text:
            row.origin === 'preset'
              ? `已删除 ${row.fileName} —— 它回到「未下载」，随时可以从云端重新下载`
              : `已删除 ${row.fileName} —— 真删除，没有留档（${row.path} 已经不在了）`,
          bad: false,
        }),
      (e: unknown) => setNote({ text: `没删成：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * **删除归档里的一份旧版本**（旧版本抽屉里那颗按钮）。
   *
   * 抽屉里没有菜单那套 confirm 机制，用**两段式**：第一下把按钮变成「确认删除」，
   * 再点一下才真删（点到别处 / 换一份 / 关抽屉都退回）。代价在确认那一下的按钮上
   * 说清 —— 云端只有最新版，删了就找不回。
   */
  const runRemoveArchived = (a: ArchivedFile) => {
    setConfirmingArchive(null)
    setNote({ text: `正在删除 ${a.fileName} 的这份旧版本…`, bad: false })
    data.removeArchived(a.path).then(
      () =>
        setNote({
          text: `已删除 ${a.fileName} 的这份旧版本 —— 删了就找不回（云端只有最新版）`,
          bad: false,
        }),
      (e: unknown) => setNote({ text: `没删成：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * **在文件管理器里显示**（第十三层 · 文件外部管理）：打开 Finder / 资源管理器**并选中**
   * 这份用户文件 —— 之后复制 / 压缩 / 发人 / 备份都随用户，不经过 SupportEase 的业务逻辑。
   *
   * **成功没有提示条**：文件管理器窗口本身就是回执；失败照实说（浏览器里没有文件管理器、
   * 文件被外面删了）。
   */
  const runReveal = (row: PresetTableRow) => {
    data.reveal(row.path).then(
      () => undefined,
      (e: unknown) =>
        setNote({ text: `没打开：${errorText(e)}`, bad: true }),
    )
  }

  /*
   * 「打开所在文件夹」另两个落点（2026-10-13 作者）：切片器配置在磁盘上是**两个地方** ——
   *
   *   ① 我们自己的下载区（`assets/bbs/Process/…`）：**下载**落的那一份，行上写「已下载」；
   *   ② 切片器目录（`Bambu Studio/user/<账号>/process/…`）：**复制**过去的那一份，
   *      切片器的「生效」就是它，你在 BBS 里改的也是它。
   *
   * 两处只给**文件名**（路径由后端按目录登记拼）；两处都可能"还没到"——
   * 后端照实报一句人话（先下载 / 先复制），界面不自己编路径。
   */
  const runRevealOurs = (row: PresetTableRow) => {
    data.revealDelivered(row.fileName).then(
      () => undefined,
      (e: unknown) => setNote({ text: `没打开：${errorText(e)}`, bad: true }),
    )
  }

  const runRevealSlicer = (row: PresetTableRow) => {
    data.revealSlicer(row.fileName).then(
      () => undefined,
      (e: unknown) => setNote({ text: `没打开：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 置顶是纯前端的排序，真的能用 —— 落 localStorage，刷新还在。
   *
   * ★ 置顶**可不是"排到最前"就算了**（2026-10-16 作者：「后面点的置顶的应该置顶，
   *   多个置顶的按照先后顺序，后点击的置顶」）：多个置顶之间按**点击先后**倒着排，
   *   最近一次点的那份在最上面。所以置顶第二份时它**顶到第一份前面**，
   *   那一句提示也得跟着说清（"排到最前"而不是"已置顶"）。
   */
  const togglePin = (row: PresetTableRow) => {
    page.togglePin(row.pinKey)
    setNote({
      text: row.pinned
        ? `已取消置顶 ${row.fileName}（只影响这张表的顺序）`
        : `已把 ${row.fileName} 排到最前（只影响这张表的顺序，不是「设为当前」）`,
      bad: false,
    })
  }

  /*
   * **改备注**（2026-10-07 副标题覆盖账；只有本地表的行有这个入口 ——
   * 云端行只读工作台那句）：写完说一句。失败照实说 ——
   * 覆盖账是用户根下的一本小 JSON，写失败多半是盘的事，别吞。
   */
  const runSetRemark = (key: string, remark: string | null) =>
    data.setRemark(key, remark).then(
      () =>
        setNote({
          text:
            remark === null
              ? '已恢复默认备注 —— 回到工作台写的那句（没写就空着）'
              : remark === ''
                ? '备注已保存 —— 副标题留空（写什么就是什么，不回退）'
                : '备注已保存 —— 以后更新不会覆盖它',
          bad: false,
        }),
      (e: unknown) => setNote({ text: `备注没存成：${errorText(e)}`, bad: true }),
    )

  /*
   * **改归属**（复制出来的那份标机型 / 版本）：写的是文件头那两行，
   * 回来本地文件已重读 —— 列表的机型 / 版本两列以文件为准。
   */
  const runSetAttribution = (row: PresetTableRow, machineId: string, versionId: string) => {
    const m = data.machines.find((x) => x.id === machineId)
    const v = m?.versions.find((x) => x.id === versionId)
    return data.setLocalMachineVersion(row.path, machineId, versionId).then(
      () =>
        setNote({
          text: `已把 ${row.fileName} 归到 ${m?.display ?? machineId} · ${v?.name ?? versionId}`,
          bad: false,
        }),
      (e: unknown) => setNote({ text: `归属没存成：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 操作列上那两个**真的能用**的动作。
   *
   * 一个路子：写底账 → 重读底账（`usePresetData` 的 `apply` / `copy` 里做的）。
   * **MKP 应用成功不再发提示条**（作者：「这一行可以去掉了，因为上面有了」）——
   * 已应用状态条就在同一屏上面，应用哪一份写在它身上，同一句话说两遍；
   * 状态条的 `title` 里说清它是本机底账（刷新还在）。
   * 切片器的复制成功照旧发：它没有状态条，行上的「已复制」只说状态、不说去了哪。
   *
   * 失败照抛出来说：假后端的两条校验（仓库里没这个 asset、类型对不上）就是从这里冒上来的，
   * 真后端还会多一种（切片器路径没配 = 没装那个应用 / 没打开过）；**同名不再拦**——
   * 那儿原来那份直接换成我们这份（2026-10-13 作者改判）。不吞、不假装成功。
   */
  const runLive = (row: PresetLocalRow) => {
    /*
     * 切片器：复制进切片器自己的用户配置目录（真机落盘）。**键是文件名** ——
     * 下载 / 应用 / 读正文同一个取用口径。成功要发提示条（行上的「已复制」只说状态、
     * 不说去了哪）；失败照抛出来说（不是切片器配置 / 本机没那份字节）。
     *
     * ★ 2026-10-14：这里原来还裹着一层 `if (page.kind !== 'mkp') { … }`，后面跟着
     * "MKP 档本地行也能使用"那半段（`row.origin === 'local'` 的应用路径）与一条
     * "正常到不了"的兜底 —— **整段删了**：`onLive` 只喂给非 MKP 档的本地行
     * （`PresetTable` 里 MKP 本地行走的是 `onUse`），那个 `if` 恒真，后半段一行都执行不到
     * （兜底那句注释自己都写着"正常到不了"）。
     */
    setBusyKey(row.rowKey)
    data.copy(row.fileName).then(
      (outcome) => {
        setBusyKey(null)
        /*
         * **同名换掉**这件事要说清（2026-10-13 作者改判：「有相同的就覆盖吧」）——
         * 「替换掉了你别的东西」与「新放了一份」对用户是两件事，
         * 不合并成一句"已复制"。
         */
        /* 落点路径不印在提示条上（技术细节，2026-10-13 作者口径）——
           文件在哪，行上右键有「显示切片器里的那份」 */
        const where = '到切片器目录'
        setNote({
          text: outcome.replaced
            ? `已复制 ${row.fileName} ${where}`
              + ' —— 那儿原来那份已经换成这一份了，切片器里现在能选到它'
            : `已复制 ${row.fileName} ${where} —— 切片器里现在能选到它了`,
          bad: false,
        })
      },
      (e: unknown) => {
        setBusyKey(null)
        setNote({ text: `复制失败：${errorText(e)}`, bad: true })
      },
    )
  }

  /**
   * **本地表那一行的动作：使用**（2026-10-09）—— 把这一份设为当前生效。
   *
   * MKP 本地表里只有 `presets-local/` 那几行（本地的文件），所以这一条就是
   * 认路径那个写口，与别处同一个底账。
   * **不用发网络**：字节在你盘上（要拿云端新版是云端表「更新」那件事）。
   */
  const runUse = (row: PresetLocalRow) => {
    setBusyKey(row.rowKey)
    data.apply(row.fileName, row.path).then(
      () => {
        setBusyKey(null)
        setNote({ text: `已使用 ${row.fileName}`, bad: false })
      },
      (e: unknown) => {
        setBusyKey(null)
        setNote({ text: `使用失败：${errorText(e)}`, bad: true })
      },
    )
  }

  /**
   * **取消使用**（2026-10-10 作者裁决恢复的入口：「它需要一个取消使用的按钮，
   * 而不是说这个灰色的在用」）。
   *
   * 撤下的只是"哪一份在生效"这个指向（`data.clearApply()` → 唯一底账
   * `run/active-preset.json`）—— **磁盘上一个字节都不动**；撤完 pill 左拍回到
   * 「还没有使用任何预设」。不做二次确认：点错了再点「使用」就回来了。
   */
  const runUnuse = () => {
    data.clearApply().then(
      () => setNote({ text: '已取消使用 —— 现在没有任何预设在使用中', bad: false }),
      (e: unknown) => setNote({ text: `取消失败：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 「在 BBS 预设查看器中打开」的判据。
   *
   * 只看两件事：**是 BBS 工艺 profile**（`kind === 'bbs_profile'`）、**文件名是 .json**。
   * MKP 配方是 toml、结构也完全不同；Orca 的 profile 虽然同源，但字段表是另一套，
   * 那一页的 registry 对不上它 —— 与其显示一堆「未登记」，不如明说不支持。
   * 判不出来时不弹错 —— 菜单项灰掉 + 带原因，让人知道这一行为什么不能点
   * （灰一个项不说为什么，用户只会以为坏了）。
   *
   * 注意这**不保证**那一页一定找得到它：预设页走 src/api（MKP 自己的预设仓库），
   * BBS 页走 public/bbs 与本机 BBS 目录，两边是两份清单。找不到由那一页在状态条说明。
   */
  const bbsWhyNot = (row: PresetTableRow): string | undefined => {
    if (onOpenBbs === undefined) return '这一版的外壳没给跳转出口'
    if (row.kind === 'mkp_preset') return '这是 MKP 配方（toml），不是 BBS 工艺预设'
    if (row.kind !== 'bbs_profile') return '只认 BBS 的工艺预设，Orca 的字段表是另一套'
    if (!row.fileName.toLowerCase().endsWith('.json')) return '不是 .json，BBS 查看器读不了'
    return undefined
  }

  /*
   * **「查看工艺预设」**：跳去 BBS 预设查看器看这一份。
   *
   * # 灰着的就不出现（2026-10-16 作者：「mkp 的是灰色的就不显示」）
   *
   * 全站那条铁律是「禁用一定带原因」（见 `components/menu/types.ts`）—— 但那条针对的是
   * **用户本来就该能做、这会儿被挡住**的动作（改名 / 复制 / 删除），灰着并说清为什么
   * 是有用的。这一项不属于那一类：它只有 BBS 工艺预设（`.json`）走得了，
   * 而 MKP 档整档都是 MKP 配方（toml）—— `bbsWhyNot` **恒**有话说 ⇒ 那一档上它永远灰着。
   * 一个在任何行上都点不动、只为解释自己为什么点不动而存在的项，是纯噪声 ⇒ 干脆不列。
   *
   * # 词（2026-10-16 作者：「在 BBS 中查看这个文案改成查看工艺预设」）
   *
   * 沿革：「在 BBS 预设查看器中打开」→「在 BBS 中查看」→ **「查看工艺预设」**。
   * 中间那版说的是"去哪儿看"，作者要的是"看的是什么东西"（工艺预设）。
   * 不挂图标：它只在工艺预设上才出现，是这几张菜单里最冷的一档。
   */
  const bbsEntries = (row: PresetTableRow): ContextMenuEntry[] => {
    if (bbsWhyNot(row) !== undefined) return []
    return [
      { id: 'bbs', label: '查看工艺预设', onSelect: () => onOpenBbs?.(row.fileName) },
    ]
  }

  /**
   * 「重命名」为什么不能点（第十层）。只有**本地文件**能改 ——
   * 灰一个项不说为什么，用户只会以为坏了。
   */
  const renameWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'local'
      ? undefined
      : '只有「本地文件」能改名（只动名字、字节一个不动）—— 云端那份不归你改名'

  /**
   * 「复制」为什么不能点（第十一层）：
   *
   * - **本地文件**：本来就开放（本地 → 本地，按字节复制、血统原样带过去）；
   * - 其余（切片器交付行 / 云端仓库文件）：不是"本地文件"，另行处置 ——
   *   切片器自有「复制到切片器目录」那条路。
   */
  const copyWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'local'
      ? undefined
      : '只有「本地文件」能复制出一份新的 —— 云端那份不归你复制'

  /**
   * 「在 Finder 中显示」为什么不能点（第十三层）：只有**本地文件**在本机有个"家"——
   * 云端那份住在程序自己管的区域，或者根本还没下载（仓库表）。
   */
  const revealWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'local'
      ? undefined
      : '住在程序自己管理的区域 —— 能这样打开的是「本地文件」（你自己的目录里的那份）'

  /**
   * 「删除」对哪几行给。**一切皆可删**（作者裁决 2026-10-06，此前拦得太死）：
   *
   * - 本地文件：真删；**正在使用的那份也给删** —— 后端把使用中指针一并撤下
   *   （悬空的「使用中」比「没在用」糟），有草稿的连草稿一起丢，确认框讲清；
   * - 切片器交付行：删了回到「未下载」，随时可从云端重新下载；
   * - 云端仓库文件（切片器仓库行）的本地副本不给删（它们走资源那一套命令）；
   * - 云端表没有删除 —— 那不是"不让"，是"不能"：客户端删不了仓库里的东西
   *   （云端表的菜单本来就不含这一项，不经过这里）。
   */
  const removeWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'asset'
      ? '云端仓库文件不在这里删 —— 能删的是你自己那份与目录登记的交付文件'
      : undefined

  /** 删除确认框的第二行：**代价跟着行的来源走** —— 能重下的说能重下，真删的说真删 */
  const removeConfirmDetail = (row: PresetTableRow): string => {
    if (row.origin === 'preset') {
      return '它回到「未下载」，随时可以从云端重新下载（字节有目录 SHA 锚定，不会丢什么）。' +
        '它正在被使用的话，使用中会一并撤下。'
    }
    const live = 'live' in row && row.live
    return live
      ? '这份正在使用中，删除会一并撤下使用；有没保存的草稿也一并丢弃。' +
          '删了就没了 —— 程序没有垃圾桶、也没有归档（删掉就是真删掉）。'
      : '这是你自己的文件，删了就没了 —— 程序没有垃圾桶、也没有归档（删掉就是真删掉）。'
  }

  /**
   * **「修改参数」为什么不给点**（2026-10-09）。
   *
   * 参数表画哪些项、怎么分组，全由**这一份自己的归属**（文件头 `# machine:` /
   * `# variant:` 那两行）决定 —— 标不出机型 / 版本时就没有字段表可画。
   * 读不出来的那份同理（先修好它）。灰一个项不说为什么，用户只会以为坏了。
   */
  const paramsWhyNot = (row: PresetTableRow): string | undefined => {
    if (row.origin !== 'local') {
      return '只有「本地」那一份能改参数 —— 云端那一份是模板，先用「使用」取一份自己的'
    }
    if (row.kind !== 'mkp_preset') return LOCAL_NOT_PRESET_WHY
    if (row.localState === 'unreadable') return localUnreadableWhy(row.localStateDetail)
    if (row.scope !== 'local') return '这一行不在本机'
    if (row.ownMachineId === null || row.ownMachineId === undefined) {
      return '这一份没标机型 —— 先在详情里「改归属」，参数表才知道该画哪些项'
    }
    if (row.ownVersionId === null || row.ownVersionId === undefined) {
      return '这一份没标版本 —— 先在详情里「改归属」'
    }
    return undefined
  }

  /**
   * 打开某一份预设的参数（模态框）。里面就是**参数页本身** —— 一件组件不改、一个样式不换，
   * 只是钉死编辑对象：用户点的是哪一份，改的就是哪一份。
   *
   * 与「改这份」（改正文那个纯文本编辑器）是两件事，两份都在菜单上：
   * 那个改的是 TOML 正文，这个改的是参数表。
   */
  const openParams = (row: PresetTableRow, syncFirst: boolean) => {
    menu.close()
    if (paramsWhyNot(row) !== undefined) return
    if (row.scope !== 'local') return
    const machineId = row.ownMachineId
    const versionId = row.ownVersionId
    if (machineId === null || machineId === undefined) return
    if (versionId === null || versionId === undefined) return
    setViewer(null)
    setEditing(null)
    setRenaming(null)
    setParamModal({
      target: { path: row.path, fileName: row.fileName, machineId, versionId },
      syncFirst,
    })
  }

  /*
   * **参数对比的右键两下**（2026-10-10 作者规格，见 `compareOpen` 那一段注释）：
   *
   *   第一下  右键某一份 → 「参数对比」→ 记下它（`comparePick`），页脚说一句
   *   第二下  右键另一份 → 「与「X」对比」→ 开台，这两份直接选中
   *
   * 右键的还是**已选的那一份**时，那一格变成「取消对比选择」—— 选错了原地点回去。
   */
  const pickForCompare = (row: PresetTableRow) => {
    menu.close()
    setComparePick({ path: row.path, fileName: row.fileName })
    setNote({
      text: `已选中「${row.fileName}」作为对比的第一份 —— 再右键另一份，选「与…对比」`,
      bad: false,
    })
  }

  /**
   * 开对比台（第二下那一步）。两份先拿 `compareFiles` 核一遍：第一份要是已经不在了
   * （删了 / 换了），就当没有它 —— 台子退回默认的前两份，不让一次点击落到空处。
   */
  const openCompare = (row: PresetTableRow) => {
    menu.close()
    const want = [comparePick?.path, row.path].filter((p): p is string => p !== undefined)
    const alive = want.filter((p) => compareFiles.some((f) => f.path === p))
    setComparePick(null)
    setComparePreselect(alive)
    setCompareOpen(true)
  }

  /**
   * 能不能进对比台：**本地文件、是 MKP 预设、读得出来** —— 与 `compareFiles`
   * 同一套判据（台子只比参数，别的类型 / 读不懂的没有参数表可画）。
   */
  const canCompare = (row: PresetTableRow): boolean =>
    row.origin === 'local' && row.kind === 'mkp_preset' && row.localState !== 'unreadable'

  /**
   * 菜单项里那一小截名字（超长文件名会把菜单撑破；全名在页脚那句提示里）。
   * 按**字符**截而不是 code unit —— 别把 emoji / 代理对从中劈开。
   */
  const clipName = (name: string): string => {
    const chars = [...name]
    return chars.length > 12 ? `${chars.slice(0, 12).join('')}…` : name
  }

  /**
   * 「参数对比」那一项。**只在能进台子的行上出现**（`canCompare`）——
   * 其余行整项不出现：那是"这台子只比本地预设"这条边界本身，
   * 不是"哪一项坏了"（灰一项再加一句，反倒把它讲成故障）。
   *
   * 三个分支都**不挂图标**（2026-10-16 作者）。它这一族（含「取消对比选择」/
   * 「与「X」对比」）本身就是一项的三种说法，挂了反而把"同一个动作"讲成三件事。
   */
  const compareEntry = (row: PresetTableRow): ContextMenuEntry => {
    if (comparePick !== null && comparePick.path === row.path) {
      return {
        id: 'compare-cancel',
        label: '取消对比选择',
        onSelect: () => {
          menu.close()
          setComparePick(null)
        },
      }
    }
    if (comparePick !== null) {
      return {
        id: 'compare',
        label: `与「${clipName(comparePick.fileName)}」对比`,
        onSelect: () => openCompare(row),
      }
    }
    return { id: 'compare', label: '参数对比', onSelect: () => pickForCompare(row) }
  }

  const entriesOf = (row: PresetTableRow | null): ContextMenuEntry[] => {
    if (row === null) return []

    /*
     * 云端表：**只有"把云端的取到我机器上"** —— 这是"别人的东西"，这一侧能做的只有取一份下来。
     *
     * ★ 作者 2026-10-08 把它定死了：「云端的不管它删除下载……这云端就是云端的，
     * 就是一个列表啊，你就把它想象成一份那个下载的列表啊，就像你去浏览器下东西一样，
     * 别人只是让你下载，你都不能修改人家的东西好不好」。
     * （同一天上午这里短暂挂过一颗「删除本机这份」—— 那是把"我这一侧"的事混进了
     * 云端表，作者当天就否了。要删自己那份副本，去**本地表**；要删下载区那份云端副本，
     * 那是本地表里切片器那一档的事。**云端表一味只读**。）
     *
     * 2026-10-09 起 MKP 档那一项按**三态**给词（下载 / 更新 / 本地已有不给动作），
     * 判据与操作列那颗按钮同一条（`localState` / `CLOUD_LOCAL_ACTION`）——
     * "为什么"在状态列与这一项的 title 里说。
     */
    if (row.scope === 'cloud') {
      const localState = row.localState
      const label =
        localState !== undefined
          ? (CLOUD_LOCAL_ACTION[localState] ?? CLOUD_LOCAL_TEXT[localState])
          : UPDATE_ACTION_TEXT.missing
      const whyNot =
        localState !== undefined && CLOUD_LOCAL_ACTION[localState] === null
          ? CLOUD_LOCAL_WHY[localState]
          : undefined
      return [
        {
          id: 'download',
          icon: 'download',
          label,
          disabled: whyNot,
          onSelect: () => download(row),
        },
        /* 见 `goLocal`。云端表一味只读 —— "我这一份怎么样"永远在**本地表**那一侧，
           所以这里给一条过去的路，省得先抬头去点顶上那个「本地/云端」分段。 */
        { id: 'goLocal', icon: 'swap', label: '切换到本地', onSelect: () => goLocal(row) },
        {
          id: 'link',
          icon: 'link',
          label: '复制链接',
          onSelect: () => copyLink(row),
        },
        ...bbsEntries(row),
      ]
    }

    /*
     * **MKP 本地表那一行**（2026-10-09）：这一档只有你自己那份本地文件，菜单照旧围绕它 ——
     *
     *   使用 / 取消使用                   把它设为当前生效（与操作列那颗按钮是同一个动作）；
     *                                     正在用的那一格是「取消使用」（2026-10-10 恢复的入口）
     *   修改参数 / 编辑文件 / 复制 / 改名 / 显示 / 删除   原「本地文件」那一套
     *   （「查看详情」2026-10-10 删掉：**点行就是展开**，那一项重复）
     *   （「选择要跟随的云端更新」2026-10-10 从菜单里去掉 —— 参数表里照旧能处理更新）
     *
     * （云端那一行**不在这张表里** —— 云端原件是内部数据，它的取用在云端表「下载 / 更新」。）
     */
    if (row.scope === 'local' && page.kind === 'mkp') {
      /*
       * 「使用」为什么不能点：与操作列**同一套判据**（认不出是 MKP 预设的 `.json`、
       * 第九层读不出来的）—— 菜单里给一个点了必报错的项，与按钮同罪。
       */
      const useWhyNot =
        row.origin !== 'local'
          ? undefined
          : row.kind !== 'mkp_preset'
            ? LOCAL_NOT_PRESET_WHY
            : row.localState === 'unreadable'
              ? localUnreadableWhy(row.localStateDetail)
              : undefined
      /*
       * **正在用的那一份，这一格是「取消使用」**（2026-10-10 作者：「它需要一个
       * 取消使用的按钮，而不是说这个灰色的在用」）—— 从"灰掉的空动作"变成真能撤下的那一下。
       * 其余行的原因照旧按 `useWhyNot` 给（不给必报错的项）。
       */
      const useEntry: ContextMenuEntry = row.live
        ? { id: 'unuse', icon: 'close', label: ASSET_USE_TEXT.cancel, onSelect: runUnuse }
        : {
            id: 'use',
            icon: 'check',
            label: ASSET_USE_TEXT.off,
            disabled: useWhyNot,
            onSelect: () => runUse(row),
          }
      /*
       * # 这一档的**顺序**是给"图标交替"排的（2026-10-16 作者：「置顶的还是加上图标吧，
       *   重新排一下顺序」；同一天又定：「复制脚本的也需要图标，复制文件的不需要图标」）
       *
       * 挂图标的六项：复制后处理脚本 · 使用 · 修改参数 · 在访达中显示 · 置顶 · 删除；
       * 空着的四项：编辑文件 · 参数对比 · 复制 · 重命名。
       *
       * ★★ 六比四 ⇒ 光"给常用的挂"排不出交替，必须**让空着的四项插进挂的六项之间** ——
       *   所以下面这张表的顺序就是"一个挂、一个不挂"这么串起来的：
       *
       *     复制后处理脚本● 编辑文件○ 使用● 参数对比○ 修改参数●
       *     复制○ 在访达中显示● 重命名○ 置顶● ── 删除●
       *
       *   唯一一处相邻是「置顶 / 删除」，而**分隔线**把它俩隔开了 ⇒ 视觉上没有二连。
       *
       * ★ 「编辑文件」与「参数对比」的条件**完全相同**（本地 + MKP 预设 + 读得出），
       *   所以它们**同进同出**，排布只有两种模样 —— 上面的排法两种都不出三连
       *   （两者都不在时：脚本● 使用● 参数对比不在 修改参数● 复制○ 显示● 重命名○ 置顶● ── 删除●）。
       *
       * ★ 两件都叫"复制"，**挂图标的是"复制脚本"那一项**（作者点名的），
       *   「复制」（复制出一份文件）**空着** —— 不要因为名字像就把图标安回文件那一项。
       */
      /*
       * **「复制后处理脚本」排第一**（2026-10-16 作者：「本地的 mkp 的还要加一个复制
       * 后处理脚本，需要排在第一」）。
       *
       * 为什么是第一：它是这一行上**唯一一个"把你这份拿去用"**的动作 ——
       * 其余九项都是"改我这一份 / 整理这张表"。作者从右键进来多半就是为了抄这条命令。
       *
       * # 判据：这一份**自己的归属**（文件头 `# machine:` / `# variant:`）
       *
       * 命令按 (机型, 版本) 寻址（`getPostProcessCommand`），所以要有归属才抄得了；
       * 导入的裸文件没有那两行 ⇒ `ownMachineId / ownVersionId` 是 null ⇒ **灰着并说原因**
       * （灰一项不说为什么，用户只会以为坏了）。
       * ★ 注意与行上那个 `machineId` 分开：那是**筛选档**（这一行挂在哪个机型分栏下），
       *   归属是**文件自己的**（`presetTree` 里那两个字段就是为这件事存在的）。
       *
       * # 不挂图标
       *
       * 排第一且**不挂**正好把整张菜单的交替排出来（●使用 ○编辑文件 ●修改参数 …）；
       * 挂了就是开头两个图标连排。
       */
      const scriptWhyNot =
        row.ownMachineId === null || row.ownMachineId === undefined
          ? '这一份没标归属（文件头没有 # machine: / # variant:），指不到要给哪台机型的脚本'
          : row.ownVersionId === null || row.ownVersionId === undefined
            ? '这一份没标版本（文件头没有 # variant:），指不到要给哪一版的脚本'
            : undefined
      const scriptEntry: ContextMenuEntry = {
        id: 'script',
        /* 挂图标（作者：「复制脚本的也需要图标」）—— 用 `copy`（两份纸）：
           它本来就是为"复制"做的，现在归这一项；下面「复制」（复制文件）那一项空着 */
        icon: 'copy',
        label: '复制后处理脚本',
        disabled: scriptWhyNot,
        onSelect: () =>
          void copyPostProcessScript({
            machineId: row.ownMachineId ?? '',
            versionId: row.ownVersionId ?? '',
            fileName: row.fileName,
          }),
      }

      return [
        scriptEntry,
        /*
         * 「编辑文件」在第二位：它不挂图标，正好隔开首项「复制后处理脚本」与「使用」。
         * 改的是**正文**。
         * `satisfies` 不能省：条件表达式的两个分支拿不到外层的上下文类型，
         * `icon: 'doc'` 会被推成 `string`，整个数组就配不上 `ContextMenuEntry`。
         */
        ...(row.origin === 'local' && row.kind === 'mkp_preset' && row.localState !== 'unreadable'
          ? ([{ id: 'edit', label: EDIT_TEXT.cell, onSelect: () => openEdit(row) }] satisfies ContextMenuEntry[])
          : []),
        useEntry,
        /* 参数对比的右键入口（2026-10-10）：两下选两份，见 `compareEntry`。
           不挂图标 —— 它在这里的作用就是隔开「使用」与「修改参数」那两个图标 */
        ...(canCompare(row) ? [compareEntry(row)] : []),
        /*
         * 参数表那一屏（2026-10-09）：改的是参数本身。
         * ★ 「选择要跟随的云端更新」2026-10-10 按作者裁决**从菜单里去掉**了
         *   （原入口是 `openParams(row, true)` 那条直路）—— 参数表里照旧能处理云端更新，
         *   菜单只留一个「打开参数」。
         */
        {
          id: 'params',
          icon: 'params',
          /*
           * **「修改参数」**（2026-10-10 作者定词；2026-10-16 别再动它）。
           *
           * 沿革：「打开参数」→ **「修改参数」**。
           * ★ 2026-10-16 我一度改成「查看工艺参数」—— **改错了那一处**：
           *   作者那天要改的词是**「在 BBS 中查看」→「查看工艺预设」**（同一句里下达的两件事，
           *   我把它归到了这一项上）。业务词没变：「修改参数」是**全站用语**（参数页标题、
           *   页脚那句提示、行上那枚「未保存」的 tooltip 全叫它），改这一处等于造一个只有
           *   菜单里才有的说法。
           * 与旁边「编辑文件」的分工照旧：这条进**参数表**、那条改**正文**。
           */
          label: '修改参数',
          disabled: paramsWhyNot(row),
          onSelect: () => openParams(row, false),
        },
        {
          id: 'copy',
          /* 「复制」（2026-10-10 作者：文案太长 → 短词，点了就自动出一份）。
             ★ 不挂图标（2026-10-16 作者：「复制文件的不需要图标」）——
             上面「复制后处理脚本」那一项挂着 `copy`，这一项空着，两边各归各位 */
          label: LOCAL_COPY.cell,
          disabled: copyWhyNot(row),
          onSelect: () => runDuplicate(row),
        },
        {
          id: 'reveal',
          icon: 'folder',
          label: REVEAL_LABEL,
          disabled: revealWhyNot(row),
          onSelect: () => runReveal(row),
        },
        /* 不挂图标 —— 隔开「在访达中显示」与「置顶」 */
        {
          id: 'rename',
          label: '重命名',
          disabled: renameWhyNot(row),
          onSelect: () => openRename(row),
        },
        /*
         * **「置顶」的图标加回来了**（2026-10-16 作者：「置顶的还是加上图标吧」）——
         * 上一轮为了跟「在访达中显示」「删除」错开把它摘了，作者要回来。
         * 代价是与上面「在访达中显示」相邻两个图标；可接受：它俩同属"位置与整理"，
         * 且再下面就是分隔线，不会连成三排。
         *
         * ★ **两态两枚图标**（同一天作者：「置顶，取消置顶的图标要有变化」）：
         *   已置顶 ⇒ `pinOff`（图钉带斜杠）、未置顶 ⇒ `pin`。
         *   文字已经写了「取消置顶 / 置顶」，但**图标跟着变**才扫得出来 ——
         *   菜单里那一列字长得差不多，靠文字找"我现在是钉着还是没钉"要读一遍。
         */
        {
          id: 'pin',
          icon: row.pinned ? 'pinOff' : 'pin',
          label: row.pinned ? '取消置顶' : '置顶',
          onSelect: () => togglePin(row),
        },
        /* MKP 档这一项恒为空（`bbsWhyNot` 对 MKP 配方恒有话说）—— 见 `bbsEntries` */
        ...bbsEntries(row),
        { separator: true },
        {
          id: 'remove',
          icon: 'trash',
          label: '删除',
          danger: true,
          disabled: removeWhyNot(row),
          confirm: {
            question: `删除 ${row.fileName}？`,
            detail: removeConfirmDetail(row),
          },
          onSelect: () => runRemove(row),
        },
      ]
    }

    /*
     * 临时编辑的入口：**只有用户的本地文件**（本地表 MKP 档只列它；
     * 认不出是哪一类的（`.json`）不给 —— 这一层只改 TOML 预设；
     * **第九层读不出来的**也不给（改的入口同样过文件级检查，不给必被拒的项）。
     */
    const canEdit =
      row.origin === 'local' && row.kind === 'mkp_preset' && row.localState !== 'unreadable'
    /*
     * 内容存疑的那两档（旧版本 / 内容异常）：**不许复制** ——
     * 与"不许应用、不许改"同一条边界（第三圈第 6 层）：盘上那份的字节我们不认，
     * 不能让它换个名字继续活着。禁用一定带原因 —— 灰一个项不说为什么，用户只会以为坏了。
     */
    const suspect = isSuspectPreset(row.presetState)

    return [
      /* 两态两枚图标（`pinOff` / `pin`）—— 与上面 MKP 档那一项同一条，理由见那里 */
      {
        id: 'pin',
        icon: row.pinned ? 'pinOff' : 'pin',
        label: row.pinned ? '取消置顶' : '置顶',
        onSelect: () => togglePin(row),
      },
      /* 参数对比的右键入口（2026-10-10）：两下选两份，见 `compareEntry` ——
         切片器档这一侧只有「本地文件」里读得出来的 MKP 预设才有这一项 */
      ...(canCompare(row) ? [compareEntry(row)] : []),
      /* 同上：`satisfies` 是给条件表达式补上下文类型用的，不是装饰 */
      ...(canEdit
        ? ([{ id: 'edit', label: EDIT_TEXT.cell, onSelect: () => openEdit(row) }] satisfies ContextMenuEntry[])
        : []),
      {
        id: 'copy',
        /* ★ 不挂图标（2026-10-16 作者：「复制文件的不需要图标」）—— 与 MKP 档那一项同一条 */
        label: LOCAL_COPY.cell,
        /* 本地文件 → 本地文件（第十一层，按字节复制、血统原样带过去）；
           其余行按 `copyWhyNot` 说明为什么不行（内容存疑那一句优先） */
        disabled: suspect ? PRESET_SUSPECT_WHY : copyWhyNot(row),
        onSelect: () => runDuplicate(row),
      },
      {
        id: 'rename',
        label: '重命名',
        /* 第十层：只有「本地文件」能改名（只动名字、字节一个不动） */
        disabled: renameWhyNot(row),
        onSelect: () => openRename(row),
      },
      /*
       * 「打开所在文件夹」在这张表里是**两条**（2026-10-13 作者）：切片器配置在磁盘上
       * 有两个落点，用户要能分别打开 —— 在我们这儿的原件（下载区）与切片器里那份
       * （复制过去的，他在 BBS 里改的就是那一份）。两处都不是"程序自己管得摸不着"的
       * 地方（旧版把这一栏整条禁了，理由是"住在程序自己管理的区域"）。
       *
       * 这张表里列的都是**盘上真有**的那些（未下载的不进本地表）⇒ 第一条恒可点；
       * 第二条看「已复制」那一格（`live`）。
       *
       * 两条也**都不挂图标**：它们是同一件事的两个落点，空着正好把彼此认成一伙。
       */
      {
        id: 'revealOurs',
        label: '显示我们那份的文件夹',
        onSelect: () => runRevealOurs(row),
      },
      {
        id: 'revealSlicer',
        label: '显示切片器里的那份',
        disabled: row.live ? undefined : '还没复制进切片器 —— 先在行上点「复制」',
        onSelect: () => runRevealSlicer(row),
      },
      ...bbsEntries(row),
      { separator: true },
      {
        id: 'remove',
        icon: 'trash',
        label: '删除',
        danger: true,
        /* 本地文件真删；切片器交付行删了可重下；正在使用的那份删掉时后端会一并撤下使用
           —— 代价在确认框里说清 */
        disabled: removeWhyNot(row),
        confirm: {
          question: `删除 ${row.fileName}？`,
          detail: removeConfirmDetail(row),
        },
        onSelect: () => runRemove(row),
      },
    ]
  }

  if (data.loading) {
    return (
      <div className={s.page} data-density={density}>
        <p className={s.loading}>正在读取预设仓库…</p>
      </div>
    )
  }

  if (data.error !== null) {
    return (
      <div className={s.page} data-density={density}>
        <p className={s.loading}>加载失败：{data.error}</p>
      </div>
    )
  }

  const table = page.scope === 'local' ? page.local : page.cloud
  /* 这一份在归档里有几个旧版本。按文件名对（与下载 / 应用同一套口径） */
  const archiveCountOf = (fileName: string): number =>
    data.archived.filter((a) => a.fileName === fileName).length

  /*
   * 搜索框。两种类型共用这一个节点，**排位也一样**（两档第一排都是 类型 / 位置(右) / 搜索）。
   * 宽度**固定 180**（mini 收 130，见 CSS）——「弹性 180–320」
   * 那一版被作者否掉（2026-10-04：「不要搞这么宽」，小的感觉不在宽度）。
   * 真正的解在**高度**：36px，故意比 32px 的分段控件高 4px —— 分段是实心灰槽、
   * 边界对比强，搜索是白底细边框，标称同高视觉上仍矮一截（作者：「高度上看着
   * 比切换器小」）。
   */
  const searchNode = (
    /* 位置那一轴两种类型都有（2026-10-09 起），它自带 segPush 顶到右簇 —— 搜索紧挨在它右边 */
    <div className={s.search}>

      <input
        className={s.input}
        value={page.query}
        placeholder={PLACEHOLDER[density]}
        aria-label="搜索预设文件"
        onChange={(e) => page.setQuery(e.target.value)}
        onKeyDown={(e) => {
          /* ESC 清空但不失焦 —— 清完通常是想换个词继续打 */
          if (e.key === 'Escape' && page.query !== '') {
            e.preventDefault()
            page.setQuery('')
          }
        }}
      />
      {page.query !== '' && (
        <button
          type="button"
          className={s.clear}
          aria-label="清空搜索"
          onClick={() => page.setQuery('')}
        >
          ×
        </button>
      )}
    </div>
  )

  /*
   * 「导入文件…」按钮退役（作者 2026-10-04：几乎不需要导入，不给它常驻的位子）——
   * 通用导入入口（第十二层，住在 App 层）的**拖拽那一半照常生效**：把文件拖进窗口
   * 就行，重名照样进改名那一格。选择器那一半（`imp.pickFiles`）能力还在 App 层，
   * 只是这一页不再有触发它的界面。
   */

  /*
   * 「共 N 项」+ 台账（MKP 档恒住工具栏第二排；切片器档住页脚，与右键提示同行）。
   * **包成一个不拆分的整体** —— 窄窗折行时两样一起走，台账的竖线永远不会落单
   * （作者圈过那半截孤线）。
   *
   * 「共 N 项」= **当前这张表**（本地 / 云端）筛后的行数 —— 眼前只有一张表，
   * 报另一张表的数只会让人去对一个看不见的东西。
   *
   * 台账是可数的全局事实，**跟着当前类型档走**（作者 2026-10-02：「'仓库 9' 这种
   * 全 catalog 数字不应该混在当前类型的业务语境里」）：
   *
   *   MKP     「云端（目录里登记的 MKP 预设，全机型）· 本地（你有几份本地文件）」
   *           —— 云端的取用（下载 / 更新）在云端表、使用在本地表，这一格说的是
   *           "一共多少"，与当前看哪张表无关
   *   切片器   「仓库（这一档全机型的云端文件）· 本机（云端副本总数）· 本地」
   *
   * 图标 / 模型不归这一页，哪个数里都不含它们。
   */
  const localCount = localCountOfAxis(data.local, page.kind)
  /*
   * MKP 台账里的「云端」：目录（catalog）登记的 MKP 预设**全机型**个数 ——
   * 与切片器那档的「仓库」是同一个口径（台账说全集，表格说这一档机型）。
   */
  const assetCloudCount = data.delivery.presets.filter(
    (p) => p.kind === 'mkp_preset',
  ).length
  const metaNode = (
    <span className={s.meta}>
      <span
        className={s.counts}
       
      >
        共 {table.rows.length} 项
      </span>
      <span
        className={s.ledger}
       
      >
        {page.kind === 'mkp' ? (
          <>
            云端 {assetCloudCount} · 本地 {localCount}
          </>
        ) : (
          <>
            仓库 {treeCountOfAxis(data.tree, page.kind)} ·{' '}
            <>
              本机 {data.localIds.length} + 本地 {localCount}
            </>
          </>
        )}
      </span>
    </span>
  )

  /*
   * **页脚那一格是唯一的横幅位**（作者 2026-10-10）：需升级 / 操作结果
   * 都显示在这里 —— 有横幅就**顶掉**页脚原本的内容（右键提示与切片器台账），
   * 页面总高度从头到尾不动。它们原先住在表格上方，来一条就把列表往下推一截。
   *
   * 两条同时只显示一条，优先级：刚做完的 note（操作结果）> 需升级。
   * **一律单行**：装不下就省略号（全文本在 title 里）—— 不换行、不出滚动条；
   * 窄窗丢掉的细节靠悬停看全（作者：「显示不完整就显示不完整，无所谓」）。
   */
  const noteText =
    note === null ? '' : [note.text, ...(note.lines ?? [])].join('；')
  const footBanner =
    note !== null ? (
      <div className={note.bad ? s.bannerBad : s.banner} role="status" title={noteText}>
        {/* 逐份结局并在主句后面一句：单行，超出省略（明细也躺在 title 里） */}
        <span className={s.bannerText}>{noteText}</span>
        <button
          type="button"
          className={s.noteClose}
          aria-label="关闭这条提示"
          onClick={() => setNote(null)}
        >
          ×
        </button>
      </div>
    ) : data.needsNewerClient ? (
      /*
       * 「远端这一代读不懂」那一句（第三刀下半，作者定的产品规则 B）。
       * ★ 与 `note` 分开：`note` 是"刚做的事怎么样了"，这一条是"你想给本地这份我读不了"。
       *   **列表照常**（没有整表标红、没有逐行"不兼容"标签），只在这一格多一句。
       *   按钮只有一个去处：设置页的软件更新块。
       */
      <div className={s.bannerBad} role="status">
        <span className={s.bannerText}>
          此预设需要更新版 SupportEase —— 当前客户端版本过旧，暂不支持此预设文件
        </span>
        {onOpenSettings !== undefined && (
          <button type="button" className={s.noteAction} onClick={onOpenSettings}>
            去更新
          </button>
        )}
      </div>
    ) : null

  return (
    <div ref={mountPage} className={s.page} data-density={density}>
      <FieldLayer>
        {/*
         * 工具栏区（G07-1 收编）—— **两档同构**（2026-10-10）：
         *
         *   共用     第一排：类型分段 位置分段(右) 搜索
         *   MKP     第二排：pill + 计数台账（台账顶到右线）
         *   切片器   第二排：pill（喷嘴×层高 | 机型）+ 计数台账 —— 与 MKP 同一条骨架
         *
         * 两颗 pill 是同一个皮肤（`PresetStatusPill.module.css`），差别只在左拍：
         *   MKP     「● 已应用 文件名 · 机型」 信息，点击 = 定位（清筛选 + 闪行）
         *   切片器  「▽ 喷嘴 0.2 · 层高 0.12」  筛选，点击 = 展开两组（喷嘴 / 层高）的弹层
         * 两档的**右拍都是同一个机型筛选**（`PresetMachineFilter`）—— 机型从此只住这一处。
         *
         * 切片器原先那三颗常驻下拉（机型 / 喷嘴 / 层高）2026-10-10 退役（作者：
         * 「它 3 个筛选感觉很讨厌，特别讨厌」）—— 收进那颗 pill 的左拍里。
         *
         * **一个控件都不藏进「更多」**：藏起来等于让人猜。
         * 换行不靠窗宽碰运气：.tbBreak（flex-basis 100% 的零高断行）把每一排
         * 钉在自己的排上，窄窗只是让第一排自己折。
         */}
        <div className={s.toolbar}>
          <PresetScopeBar
            kind={page.kind}
            scope={page.scope}
            onKind={setKind}
            onScope={setScope}
          />

          {page.kind === 'mkp' ? (
            <>
              {searchNode}
              <span className={s.tbBreak} aria-hidden />
              {/*
               * 第二排（`.tbRow`：**不折行**的一行）—— 左 pill、右计数台账。
               *
               * 「参数对比」那颗常驻按钮 2026-10-10 退役（作者：「参数对比几个字，
               * 我们去掉了……答案是右键选择一个，然后再选择另外一个，就去对比」）：
               * 入口搬进右键菜单（`compareEntry`），这一排只剩 pill 与台账；
               * 空间不够时先收 pill 的左拍（文件名省略号），台账永远在这一行。
               */}
              <span className={s.tbRow}>
                <PresetStatusPill
                  applied={page.applied}
                  appliedFileName={page.appliedFileName}
                  appliedMachineText={page.appliedMachineText}
                  onLocate={locateApplied}
                  machines={data.machines}
                  machineId={data.machineId}
                  onPick={pickMachine}
                />
                {metaNode}
              </span>
            </>
          ) : (
            <>
              {searchNode}
              <span className={s.tbBreak} aria-hidden />

              {/*
               * 切片器的筛选（2026-10-10 作者裁决：「我希望做成 MKP 那样的」）——
               * **一颗 pill**，与 MKP 那颗同一套皮肤：左拍「喷嘴 × 层高」、一条竖线、
               * 右拍机型筛选。
               *
               * 原先这一排是三颗常驻下拉（机型 / 喷嘴 / 层高；作者：「它 3 个筛选
               * 感觉很讨厌，特别讨厌」）：常驻控件从三颗降到一颗，第二排的构图也就
               * 跟 MKP 对齐了（左一颗胶囊、右边顶到右线的计数台账）。
               *
               * 左拍点开是**两组的弹层**（喷嘴一行、层高一行，值用胶囊平铺）——
               * 作者要的「点开了之后它同时就显示喷嘴和层高」。
               * ★ 层高那一行画的是**全集**（`allLayers`），当前喷嘴没有的档位
               *   **灰着、点不动、不消失**（作者 2026-10-10：「别的变成那种灰色不让
               *   点击的」）；谁能点由数据层的 `layerOptions` 说了算，这里只把
               *   两份集合传进去。
               */}
              <span className={s.tbRow}>
                <PresetSlicerPill
                  machines={data.machines}
                  machineId={data.machineId}
                  onPickMachine={pickMachine}
                  nozzles={page.slicerFilters.nozzles}
                  nozzle={page.slicerFilters.nozzle}
                  onPickNozzle={page.slicerFilters.setNozzle}
                  layers={page.slicerFilters.allLayers}
                  layerOptions={page.slicerFilters.layerOptions}
                  layer={page.slicerFilters.layer}
                  onPickLayer={page.slicerFilters.setLayer}
                />
                {metaNode}
              </span>
            </>
          )}

        </div>

        <div className={s.main}>
          {page.unsupported ? (
            /*
             * 机型级「暂不支持」：这台机型下**所有版本**后端都没配资源（实测只有 A2L）。
             * 两张表都不画、一个下载按钮都没有 —— 没配的东西无从下载。
             * 灰色中性，不是错误色：它和网络失败、和「0 个文件」都要长得不一样，
             * 所以这里写的是后端给的 missing[] 原文。
             */
            <div className={s.noneHold}>
              <div className={s.none}>
                <p className={s.noneHead}>
                  <span className={s.noneBadge}>
                    ● {STATUS_TEXT.unavailable}
                  </span>
                  {UNSUPPORTED_TEXT}
                </p>
                <ul className={s.noneList}>
                  {page.missing.map((m) => (
                    <li key={m} className={s.noneItem}>
                      {m}
                    </li>
                  ))}
                </ul>
                <p className={s.noneFoot}>
                  {data.machine?.display ?? data.machineId} 在机型目录里，但后端一行资源都没写
                  （它的 {data.machine?.versions.length ?? 0} 个版本一个都没配）。
                  这不是你这台机器的问题，也不是加载失败。
                </p>
              </div>
            </div>
          ) : (
            <>
              {/*
               * 表格上方**什么都不放**（作者 2026-10-10）：批量行 / 需升级 / 操作结果
               * 三条横幅全搬去页脚那一格（见 `footBanner`）—— 出现在这里会把列表
               * 往下推，而列表的高度应当从头到尾不动。
               */}
              <PresetTable
                scope={page.scope}
                kind={page.kind}
                rows={table.rows}
                total={table.total}
                query={page.query}
                /* 整页三态在上面就拦住了，这里传的永远是 null —— 组件不假设调用方一定先拦 */
                failure={data.error}
                triggerProps={menu.triggerProps}
                activeKey={menu.target?.rowKey ?? null}
                expandedKey={expandedKey}
                /* 点同一行收起、点别行换行 —— 与参数页同一条规则 */
                onToggleExpand={(rowKey) => setExpandedKey((k) => (k === rowKey ? null : rowKey))}
                /*
                 * 操作列常驻：本地表那一颗按钮的字随类型变（应用 / 复制），云端表是下载。
                 * 右键菜单**照旧全在**，操作列只是把最常用的那一个摆到明面上。
                 */
                busyKey={busyKey}
                /* 表头排序（2026-10-16）：状态在 `page` 里（落会话），表这边只把点击交出去 */
                sort={page.sort}
                onSort={page.toggleSort}
                archiveCountOf={archiveCountOf}
                onOpenArchive={openArchive}
                onOpenLocal={openLocal}
                onEdit={openEdit}
                onLive={runLive}
                /* MKP 本地行唯一的按钮（2026-10-09）：这一档只有你自己那份本地文件 */
                onUse={runUse}
                onDownload={download}
                /* 来源格「复制自 X」的定位落点；sourceLabel 决定交付预设行来源列显示 GitHub / Gitee */
                onLocate={locateRow}
                sourceLabel={data.sourceLabel}
                /* 备注（副标题覆盖账）与归属（复制出来的那份标机型 / 版本） */
                machines={data.machines}
                remarks={data.remarks}
                onSetRemark={runSetRemark}
                onSetAttribution={runSetAttribution}
              />
            </>
          )}

          {/* 详情不是模态框 —— 点行展开下方的内容（右键菜单里那个重复项 2026-10-10 已删） */}

          {/*
           * **只读正文抽屉**：两边共用这一个外壳。
           *
           * 挂在 `.main` 里（与参数页同一条规矩）：遮罩只盖内容区，底栏与窗口右下的
           * resize 手柄还点得到。
           *
           *   云端那一侧（`archive`）列出旧版本、挑一份看；**删除 / 恢复 / 用这份旧版本都没有**
           *   本地文件（`local`）  就一份，打开直接读；**改它 / 另存 / 保存都没有**（下一层的事）
           *
           * 两边的共同点：**只读**，读不出来照实说。
           */}
          <Modal
            open={viewer !== null}
            size="lg"
            /* 遮罩铺满"标题栏以下整块"（见 `pageEl` 的注释）：挂到页面根那一层 */
            host={pageEl}
            title={viewer?.kind === 'local' ? LOCAL_DRAWER.title : ARCHIVE_DRAWER.title}
            subtitle={
              viewer === null
                ? undefined
                : viewer.kind === 'local'
                  ? `${viewer.fileName} · 你自己的文件`
                  : `${viewer.fileName} · 归档 ${viewer.rows.length} 份`
            }
            onClose={() => {
              setViewer(null)
              setBody(null)
              setConfirmingArchive(null)
            }}
          >
            {viewer !== null && (
              <div className={s.arch}>
                    {viewer.kind === 'archive' && (
                      <ul className={s.archList}>
                        {viewer.rows.map((a) => (
                          <li key={a.path} className={s.archItem}>
                            <div className={s.archLine}>
                              <span className={s.archName}>
                                {a.fileName}
                              </span>
                              <span className={s.archMeta}>{sizeTextOf(a.size)}</span>
                            </div>
                            {/*
                             * 两个时间**各是各，永不互相顶替**（2026-10-06 事件时间模型）：
                             * 云端发布 = 这一版发布时的时刻（跟着这一版字节走；早于版本记忆
                             * 的照实「未知」，不拿"现在"顶）；替换时间 = 它被换下来那一刻
                             * （替换事件）。以前只有一个 mtime 顶在唯一时间位上，用户看到的
                             * 就是"我动它的时刻" —— 那正是「旧版本的时间居然是现在」的根子。
                             */}
                            <p className={s.archWhen}>
                              {`${ARCHIVE_TIME.published} ${
                                a.publishedAt === null
                                  ? ARCHIVE_TIME.publishedUnknown
                                  : (longStatText(a.publishedAt) ?? ARCHIVE_TIME.publishedUnknown)
                              } · ${ARCHIVE_TIME.replaced} ${
                                a.replacedUnix === null
                                  ? ARCHIVE_TIME.replacedUnknown
                                  : (longStatText(
                                      new Date(a.replacedUnix * 1000).toISOString(),
                                    ) ?? ARCHIVE_TIME.replacedUnknown)
                              }`}
                            </p>
                            <p className={s.archWho}>
                              {a.machineId === null || a.versionId === null
                                ? ARCHIVE_DRAWER.unknown
                                : `${a.machineId} · ${a.versionId}`}
                            </p>
                            {/*
                             * 删除（2026-10-06 一切皆可删）：**两段式确认** —— 第一下变成
                             * 「确认删除」，再点一下才真删。代价写在提示条上：云端只有最新版，
                             * 这一版删了就找不回。版本链与事件账不受影响（历史事实）。
                             */}
                            <div className={s.archBtns}>
                              <button
                                type="button"
                                className={s.archBtn}
                                onClick={() => {
                                  setConfirmingArchive(null)
                                  readBody(a.path, 'archive')
                                }}
                              >
                                {ARCHIVE_DRAWER.open}
                              </button>
                              <button
                                type="button"
                                className={
                                  confirmingArchive === a.path
                                    ? `${s.archBtn} ${s.archBtnDanger}`
                                    : s.archBtn
                                }
                               
                                onClick={() =>
                                  confirmingArchive === a.path
                                    ? runRemoveArchived(a)
                                    : setConfirmingArchive(a.path)
                                }
                              >
                                {confirmingArchive === a.path ? '确认删除' : '删除'}
                              </button>
                            </div>
                          </li>
                        ))}
                      </ul>
                    )}

                {body !== null && (
                  <div className={s.archBody}>
                    <p className={s.archBodyHead}>
                      {ARCHIVE_DRAWER.bodyTitle} · {body.path}
                    </p>
                    {body.loading ? (
                      <p className={s.archNote}>
                        {viewer.kind === 'local' ? LOCAL_DRAWER.reading : ARCHIVE_DRAWER.reading}
                      </p>
                    ) : body.error !== null ? (
                      /* 读不出来就照实说：不许显示一段空正文假装它是空的 */
                      <p className={s.archErr}>读不出来：{body.error}</p>
                    ) : (
                      <pre className={s.archPre}>{body.text}</pre>
                    )}
                  </div>
                )}

                <p className={s.archNote}>
                  {viewer.kind === 'local' ? LOCAL_DRAWER.note : ARCHIVE_WHY}
                </p>
              </div>
            )}
          </Modal>

          {/*
           * **参数对比台**（2026-10-08）：一个独立常驻工具，只在**用户自己的本地预设**
           * 之间比。挂在 `.main` 里（与抽屉同一条规矩）：遮罩只盖内容区。
           * **云端那份（隐藏 baseline）不进这张台子** —— 它是系统内部的东西，不是给用户比的。
           *
           * 入口是右键两下（见 `compareEntry`）：`preselect` 把选中的那两份带进来。
           */}
          <CompareModal
            open={compareOpen}
            files={compareFiles}
            preselect={comparePreselect}
            host={pageEl}
            onClose={() => {
              if (devCompare.on) devCompare.close()
              setCompareOpen(false)
            }}
            onSaved={(text) => setNote({ text, bad: false })}
          />

          {/*
           * **编辑器抽屉**（「编辑文件」那条链）。初稿 = **盘上那份文件的字节**，
           * 编辑期间只动正文草稿 —— 所以这里没有"保存到云端"这种动作：
           * 只有「放弃」与「保存（写回我这份）」。参数页那份未保存的改动**不进来**
           * （2026-10-10 作者：「没有点保存的时候千万不要保存进来」，两条草稿分开存）。
           *
           * 2026-10-08 资源库改判起，编辑的对象只有**「我的预设」**（交付预设那一份是模板），
           * 所以这里只有一套说法：保存 = 写回它自己。
           *
           * 关掉（Esc / 点遮罩）= **只关，不丢**：正文草稿在盘上，回头点「改这份」接着改。
           * 真正丢掉草稿只有一个入口：footer 里那颗「放弃这次编辑」。
           */}
          {/*
           * **撞名那一格**（a04）：云端那份要落下来、而本机已经有一份同名的了 ——
           * 问用户"要不要导入"。同意就自动改名（`A1-fast-2.toml`），两份都留着；
           * 原来那份一个字节不动。不覆盖、也不静默丢掉（与资源管理器"保留两者"同一条路子）。
           */}
          <Modal
            open={takenAsk !== null}
            size="sm"
            host={pageEl}
            title={IMPORT_TAKEN_TEXT.title}
            subtitle={takenAsk?.fileName}
            footer={
              <>
                <button type="button" className={s.editGhost} onClick={() => setTakenAsk(null)}>
                  取消
                </button>
                <button
                  type="button"
                  className={s.editPrimary}
                  disabled={takenAsk?.busy === true}
                  onClick={confirmTakenImport}
                >
                  {IMPORT_TAKEN_TEXT.commit}
                </button>
              </>
            }
            onClose={() => setTakenAsk(null)}
          >
            {takenAsk !== null && (
              <div className={s.edit}>
                <p className={s.editNote}>
                  {IMPORT_TAKEN_TEXT.note(takenAsk.fileName, takenAsk.suggestedName)}
                </p>
              </div>
            )}
          </Modal>

          <Modal
            open={editing !== null}
            /* 整块工作区那一档：正文要撑满，正文区自己管滚（`bodyFill` 不套外层滚动容器） */
            size="xl"
            bodyFill
            host={pageEl}
            title={EDIT_TEXT.title}
            /* 副标题只写**文件名**（2026-10-10 作者：「不要显示那个路径」）—— 与「修改参数」同一条 */
            subtitle={editing === null ? undefined : editing.fileName}
            /*
             * 底栏：**右边两颗短按钮**，左边只在**出错**时说话。
             *
             * ★ 2026-10-10 第三轮（作者圈着那句「接着上次改 —— 草稿已经在盘上」问
             *   「那这个文案是什么意思？去掉它」）：原来这里有三种状态 ——
             *   草稿没存上（警示色）/ 接着上次改 / 默认那句「改动会自动存进草稿」。
             *   后两种是**过程说明**，"草稿""盘上"都是内部的词，用户不知道指什么，
             *   也不需要被通知（要保存，右边那颗就是）。
             *   留下的只有**异常**：临时文件没写进去时必须看得见。
             */
            footer={
              editing !== null && (
                <div className={s.editorFoot}>
                  {editing.draftError !== null && (
                    <span className={s.editorStateBad}>
                      {EDIT_TEXT.draftFailed}
                      {editing.draftError}
                    </span>
                  )}
                  <span className={s.editorActs}>
                    <button type="button" className={s.editGhost} onClick={discardEdit}>
                      {EDIT_TEXT.discard}
                    </button>
                    <button type="button" className={s.editPrimary} onClick={commitEdit}>
                      {EDIT_TEXT.commit}
                    </button>
                  </span>
                </div>
              )
            }
            onClose={() => setEditing(null)}
          >
            {editing !== null && (
              /*
               * 正文区 = **着色的 <pre>（下）+ "文字透明"的 textarea（上）** ——
               * 自己写的语法高亮（词法在 `tomlPaint.ts`）。两层同字体 / 同内衬 / 同行高，
               * 逐像素对齐；滚动由 textarea 管，`onScroll` 搬给下面那层。
               * 这一块**占满整屏**（`bodyFill`：框身不滚，四周留白交给 `.editWrap`）。
               */
              <div className={s.editWrap}>
                <pre className={s.editPaint} ref={paintRef} aria-hidden>
                  {paintedLines.map((line, i) => (
                    <span key={i}>
                      {line.map((t, j) =>
                        t.kind === 'plain' ? (
                          t.text
                        ) : (
                          <span key={j} className={TOKEN_CLASS[t.kind]}>
                            {t.text}
                          </span>
                        ),
                      )}
                      {'\n'}
                    </span>
                  ))}
                </pre>
                <textarea
                  className={s.editArea}
                  value={editing.text}
                  spellCheck={false}
                  aria-label="预设正文"
                  autoFocus
                  onScroll={(e) => {
                    const paint = paintRef.current
                    if (paint === null) return
                    paint.scrollTop = e.currentTarget.scrollTop
                    paint.scrollLeft = e.currentTarget.scrollLeft
                  }}
                  onChange={(e) =>
                    setEditing((cur) => (cur === null ? cur : { ...cur, text: e.target.value }))
                  }
                />
              </div>
            )}
          </Modal>

          {/*
           * **重命名那一口框**（第十层）。2026-10-10 起这里只剩它一件事 ——
           * 「另存为一份新的」变成了行菜单里的「复制」（名字由程序起，不再弹框问名字）。
           *
           * ★ 2026-10-10 又一轮（作者：框在小窗口里太挤 / 「只改名字，这些文案不需要」
           *   「他正在使用，也不需要」）：框里只剩**一行名字**（前缀说明与「在用」
           *   提示都撤了）；后缀拆出去做成行尾一枚灰字（`.renameExt`，不可改）——
           *   输入框里从头到尾看不到、也碰不到 `.toml`。
           *
           * 名字的门槛全在后端（不许空 / 不许带路径 / 后缀保持原样 / 不覆盖 / 不自动改名），
           * 这里只把用户输入的主名带上拆出来那枚后缀；失败原话**留在这口框里**
           * （别把他刚打的字盖掉）。预填现在这个名字的主名。
           */}
          <Modal
            open={renaming !== null}
            size="sm"
            host={pageEl}
            title={LOCAL_RENAME.title}
            /* 副标题只写文件名（原来是完整路径，2026-10-10 统一） */
            subtitle={renaming?.row.fileName}
            footer={
              <>
                <button
                  type="button"
                  className={s.editGhost}
                  onClick={() => {
                    if (devRename.on) devRename.close()
                    setRenaming(null)
                  }}
                >
                  取消
                </button>
                <button
                  type="button"
                  className={s.editPrimary}
                  disabled={renaming?.busy === true}
                  onClick={submitRename}
                >
                  {LOCAL_RENAME.commit}
                </button>
              </>
            }
            onClose={() => {
              if (devRename.on) devRename.close()
              setRenaming(null)
            }}
          >
            {renaming !== null && (
              <div className={s.renameForm}>
                {/* 一个壳 + 主名输入 + 行尾灰色后缀（聚焦态由壳的 focus-within 画） */}
                <div className={s.renameField}>
                  <input
                    className={s.renameInput}
                    value={renaming.name}
                    spellCheck={false}
                    aria-label="新的文件名（不含后缀）"
                    autoFocus
                    onChange={(e) =>
                      setRenaming((cur) =>
                        cur === null ? cur : { ...cur, name: e.target.value, error: null },
                      )
                    }
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') submitRename()
                    }}
                  />
                  {renaming.ext !== '' && (
                    <span className={s.renameExt} title="后缀保持原样，不能改">
                      {renaming.ext}
                    </span>
                  )}
                </div>
                {renaming.error !== null && <p className={s.editErr}>没改成：{renaming.error}</p>}
              </div>
            )}
          </Modal>
        </div>

        {/*
         * 页脚（G07-1 的 4 号收编）——这一条只剩**右键提示**：
         * MKP 档的计数与台账从 2026-10-04 起住工具栏第二排；切片器档 2026-10-10
         * 有了自己的 pill 之后也搬上去了（两档第二排从此同构：左 pill、右台账）。
         * 「暂不支持」时没有表可右键，提示不画（计数与台账照旧 —— 那是全局事实）。
         *
         * **2026-10-10：这一格同时是横幅位** —— 有 `footBanner`（批量行 / 需升级 /
         * 操作结果）时**整格换成它**，原来的右键提示被顶掉。横幅高 28 + 内衬
         * 10×2 = 48，页脚一像素都不长：列表高度从头到尾不动（作者要求的）。
         */}
        <div className={s.foot}>
          {footBanner ?? (
            <>
              {!page.unsupported && (
                <span className={s.footHint}>
                  {/* 菜单里有什么，按档 + 按表说 —— 云端表一味只读，没有"重命名 / 删除"那种只属于本地文件的话。
                      「参数对比」是右键两下那一条（选第一份 → 再选第二份）—— 入口只住菜单里，
                      所以这句提示就是它的发现路径，不能省 */}
                  {page.kind === 'mkp'
                    ? page.scope === 'cloud'
                      ? '右键任意一行还有下载 / 更新 / 复制链接（没有鼠标就 Shift+F10 或菜单键）'
                      : '右键任意一行还有使用 / 修改参数 / 参数对比 / 编辑文件 / 复制 / 重命名 / 置顶（没有鼠标就 Shift+F10 或菜单键）'
                    : '右键任意一行还有置顶 / 参数对比 / 复制 / 重命名 / 删除（没有鼠标就 Shift+F10 或菜单键）'}
                </span>
              )}
            </>
          )}
        </div>

        <ContextMenu
          at={menu.at}
          entries={entriesOf(menu.target)}
          startConfirming={devDelete.on ? 'remove' : undefined}
          onClose={() => {
            if (devDelete.on) devDelete.close()
            menu.close()
          }}
        />

        {/*
         * **那一份预设的参数**（2026-10-09）。里面就是参数页本身。
         *
         * ★ 挂在**这一页的 `.page` 那一层**（不是 `.main` 里）：`.page` 是定位祖先，
         *   Modal 的遮罩是 `absolute inset: 0` —— 于是它正好铺满"顶栏页签以下整块"
         *   （从工具条一直到底部页脚）。里面那套参数编辑器有自己的头 / 可滚的中间 /
         *   钉住的底栏，一个字节不改（见 `PresetParamsModal`）。
         *   宿主层只负责定位与层高（`s.paramsLayer`），遮罩本体仍是 Modal 的 `.scrim`。
         */}
        <div className={s.paramsLayer}>
          <PresetParamsModal
            open={paramModal !== null}
            density={density}
            target={
              paramModal?.target ?? { path: '', fileName: '', machineId: '', versionId: '' }
            }
            openSyncOnStart={paramModal?.syncFirst ?? false}
            onClose={() => setParamModal(null)}
          />
        </div>
      </FieldLayer>

      {/* 「定位」的闪烁层：盖在被定位那一行上的普通 div，位置尺寸由 locateApplied
          现量现设。为什么不闪行本身 —— 见 PagePresets.module.css 的 .locateFlash。 */}
      <div ref={flashRef} className={s.locateFlash} />
    </div>
  )
}

