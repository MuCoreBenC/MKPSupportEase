/*
 * 预设（去标题 + 列随类型变 + 常驻操作列 + 两种「生效」分开）。
 *
 * # 版面
 *
 * ```
 * ┌MKP 配置│切片器配置┐ ┌本地│云端┐ [搜索…] ┌● 已应用 A1.toml · A1 │ ▽ A1 ▾┐  ← MKP
 * 喷嘴 全部 0.2 0.4 0.6   层高 全部 0.08 0.10 … 更多                        ← 切片器
 * 名称            机型   版本    时间   来源   操作
 * A1.toml         A1    标准版  09-14  官方   已应用
 * ├────────────────────────────────────────────┤
 * 右键任意一行还有…        共 4 项 │ 仓库 20 · 本机 4 + 我的 3   ← 页脚
 * ```
 *
 * **融合 pill（G07-1 收编，A44 三轮定稿；2026-10-04 进产品）**：原来的「机型下拉
 * （`PresetPicker`）+ 已应用状态条 + 定位按钮」三件并成一颗 PresetStatusPill ——
 * 左拍「● 已应用 A1.toml · A1」是信息（带「已应用」两个字，作者第二轮点名保留），
 * **点它 = 定位**：清掉机型筛选、把正在生效那行滚到眼前闪一下，不写「定位」两个字
 * （作者第三轮）；右拍「▽ A1 ▾」是**机型筛选**（漏斗菜单，有「全部机型」一档）——
 * 机型是**筛选**，不是切换（作者第二轮纠正：「这个是个筛选的……不能再用抽屉了」）。
 * 三个事实（在生效的是谁 / 怎么找回它 / 看哪个机型）融进一颗件，机型信息从此只出现
 * 这一次 —— `PresetPicker` 下拉整个退场。切片器没有「已应用」，它的筛选以**独立漏斗**
 * 留在搜索左边（PresetStatusPill 的 named export），不硬凑成一颗。
 *
 * **「撤销应用」退役（作者 2026-10-04 裁决）**：不做取消应用 —— 总得有一套在生效。
 * 后端 `clear_active_preset` 命令与数据层的 `clearApply` 照旧在（能力不删），
 * 只是从此没有界面入口；pill 左拍上的「已应用」就是唯一的事实。
 *
 * **计数与台账的住处（2026-10-04 定稿）**：MKP 档恒住工具栏第二排（pill 右边，
 * 台账顶到右线）—— 工具栏恒两排：第一排 类型分段 / 位置分段(右) / 搜索，第二排
 * pill + 计数台账，不再随窗宽换摆法。页脚与右键提示同行；切片器的筛选排本来就满
 * （喷嘴 / 层高 chips），它的计数与台账任何档都住页脚。
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
 * 它右边那格台账：**「仓库」与「我的」两个数跟着当前类型档走**（MKP 档数 MKP 的、
 * 切片器档数切片器的；图标 / 模型不归这一页，哪个数里都没有它们）—— 全 catalog 的数字
 * 混进某一档的语境里只会让人对不上（作者 2026-10-02 点名「仓库 9」）。
 * 「本机」仍是官方副本的总数（老契约 `getLocalFiles` 的读数，id 集合分不出类型）。
 *
 * # 本地 / 云端是**两张互不相干的表**
 *
 *   本地表  你这台机器上有什么。官方下载下来的副本（`getLocalFiles()`）+ **用户线**
 *           （`getUserPresetFiles()` 扫 `presets-mine/` —— **云端没有它们**）
 *   云端表  菜单上有什么官方文件（已分配 + 可选）。仅归档的一处都不出现
 *
 * 一个官方文件下载之后两张表里都有，那是对的：云端表说「仓库里有这个东西」，
 * 本地表说「你机器上有这个东西」。两张表各回答一个问题，不合流。
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
 *   本机有哪些官方文件       `api.getLocalFiles()`                       **演示集合**（假后端没有文件系统）
 *   我自己的文件             `api.getUserPresetFiles()`                  真（**用户线**：扫 `presets-mine/`，盘当底账）
 *   我那一份的正文           `api.readUserPresetText()`                  真（只读；抽屉里看，读不出来照实说）
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
 *   查看详情                 上面那些字段的汇总                          **纯前端**，真的能用
 *   **下载 / 更新**（交付行） `api.downloadCatalogFile()` → 下载管道          真（落 `mkp/`；**过程水位**照说，
 *                            需更新时走**同一条管道** —— 旧份自动归档，没有第二个命令）
 *   **批量**（云端表那一行）   `api.downloadCatalogFiles()`（多份）            真（同一套机制；**逐份结局**，
 *                            没成的各占提示条一行。范围 = 机型 + 类型，不受搜索词影响；已下载的不进来）
 *   交付行的状态              `getDownloadedFiles` + `getStaleFiles`        真（三个读合起来才够四态：
 *                            + `getDeliveryTrust` —— 未下载 / 已下载 / 旧版本 / 内容异常，
 *                            见 `ReleaseFileState`）
 *   **修改 / 保存**（我的文件） `api.beginPresetEdit()` + `commitPresetDraft()` 真（改的是**临时文件** `run/app-state.json`：
 *                            `putPresetDraft` 边改边存；保存 = **写回我那一份自己**（不产生第二份）。
 *                            **原文件与下载区全程没被碰过** —— 判据逐字节盯着）
 *   **重命名 / 删除**（我的文件） `api.renameUserPreset()` / `deleteUserPreset()` 真（第十层：只动名字，
 *                            字节一个不动；使用中指针与该份草稿跟着改名。删=真删，没有垃圾桶、没有归档；
 *                            正在使用 / 还有草稿的不给删 —— 原因原话来自后端）
 *   **另存为一份新的**（我的文件） `api.copyUserPreset()`                   真（第十一层：我的文件 → 我的文件，
 *                            按字节复制、血统原样带过去；不覆盖、不自动改名；不碰使用中指针与草稿）
 *   **导入（第十二层）**      `FileImportProvider`（App 层）               真（通用导入入口 ——
 *                            **拖拽进窗口**；重名开改名那一格。工具栏的「导入文件…」
 *                            按钮已退役（作者 2026-10-04：几乎不需要导入），选择器能力照旧在 App 层）
 *   **下载**（官方行）        `api.downloadFiles()`                       抛未实现，界面照实说（不编假进度条）
 *   **在 Finder 中显示**（我的文件） `api.revealInFolder()`                  真（第十三层：打开系统文件管理器**并选中**；
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
import type { ActiveOrigin, ArchivedFile, FileRef } from '../../api'
import { longStatText } from '../store/package'
/* 归档抽屉的外壳：与参数页那个抽屉同一个（absolute 定位、遮罩只盖内容区） */
import Drawer from '../shared/Drawer'
/* 通用导入入口（第十二层，停在 App 层）：这一页消费它的 revision（拖拽导入照常生效） */
import { useFileImport } from '../import/useFileImport'
import { FieldLayer, FieldPopover } from '../../components/field'
import { ContextMenu, useContextMenu } from '../../components/menu'
import type { ContextMenuEntry } from '../../components/menu'
import type { Density } from '../../hooks/useDensity'
import { detectPlatform } from '../../hooks/usePlatform'
/* 下载过程与逐份结局的措辞（`shared/download.ts`）—— 同一件事一处文案 */
import { outcomeText, tickText } from '../shared/download'
import CompareModal from './CompareModal'
import PresetScopeBar from './PresetScopeBar'
import PresetStatusPill, { PresetMachineFilter } from './PresetStatusPill'
import PresetTable from './PresetTable'
import {
  ARCHIVE_DRAWER,
  ARCHIVE_TIME,
  ARCHIVE_WHY,
  EDIT_TEXT,
  MINE_COPY,
  MINE_DRAWER,
  MINE_RENAME,
  DOWNLOAD_WHY,
  mineCountOfAxis,
  treeCountOfAxis,
  NO_ASSET_WHY,
  RELEASE_SUSPECT_WHY,
  STATUS_TEXT,
  STATUS_WHY,
  UNSUPPORTED_TEXT,
  /* 「下载」那个词只有一处（`UPDATE_ACTION_TEXT`）—— 右键菜单与操作列那颗按钮不许各写一个 */
  UPDATE_ACTION_TEXT,
  isSuspectRelease,
  noContractText,
  releaseBatchText,
  sizeTextOf,
} from './presetTree'
import type {
  LocateTarget,
  PresetKindAxis,
  PresetScopeAxis,
  PresetLocalRow,
  PresetTableRow,
} from './presetTree'
import { usePresetData, usePresetPage } from './usePresetData'
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
 */
const REVEAL_LABEL =
  detectPlatform() === 'windows' ? '在文件资源管理器中显示' : '在 Finder 中显示'

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

export default function PagePresets({ density, onOpenBbs, onOpenSettings }: Props) {
  /*
   * 通用导入入口（第十二层）停在 App 层；这一页拿 `revision`
   * （导入落进 `presets-mine/` 之后整屏重读，「我的文件」立刻以磁盘为准）。
   * 导入的入口是**拖拽**（把文件拖进窗口，App 层接）—— 工具栏上的「导入文件…」
   * 按钮已退役（作者 2026-10-04：几乎不需要导入）；选择器能力（pickFiles）
   * 照旧住在 App 层，只是这里不再有触发它的界面。
   */
  const imp = useFileImport()
  const data = usePresetData(imp.revision)
  const page = usePresetPage(data)
  /*
   * **对比台的候选**：读得出来的「我的预设」（MKP 预设）。
   * 读不出来的那份不给 —— 比一份读不懂的文件只会得到一行"读不出来"；
   * 别的类型（`.json`）也不给：这一台子比的是参数。`useMemo` 是必需的（对比台按它重置选择）。
   */
  const compareFiles = useMemo(
    () => data.mine.filter((f) => f.kind === 'mkp_preset' && f.state !== 'unreadable'),
    [data.mine],
  )
  const rootRef = useRef<HTMLDivElement>(null)
  /** 「定位」的闪烁层：盖在被定位那一行上的普通 div（见 s.locateFlash 的注释） */
  const flashRef = useRef<HTMLDivElement>(null)

  /* 一个列表一个菜单。两张表同时只显示一张，所以一份就够 */
  const menu = useContextMenu<PresetTableRow>()
  /*
   * 展开详情的那一行（rowKey）。行上点击与右键菜单的「查看详情」共享同一个状态，
   * 同一时刻只开一行 —— 与参数页的 expandedKey 同构。
   */
  const [expandedKey, setExpandedKey] = useState<string | null>(null)
  const [note, setNote] = useState<Note | null>(null)
  /*
   * **参数对比台**（2026-10-08）：一个**独立常驻工具** —— 不挂在"官方版本"或"更新"上，
   * 也不因为有没有新版而变化。它只比**用户自己的本地预设**（`presets-mine/`）。
   */
  const [compareOpen, setCompareOpen] = useState(false)
  /*
   * 正在做动作的那一行（rowKey）。只用来把那一颗按钮禁掉 —— 假后端是同进程的内存写，
   * 这一下快到看不见；真后端上「应用」要写盘，连点两次就会发两个写。
   * 不做全表遮罩：一行的动作不该把整张表锁住。
   */
  const [busyKey, setBusyKey] = useState<string | null>(null)
  /*
   * 批量在进行中。它只锁那一颗批次按钮 —— 行级的动作**不跟着锁**：
   * 一根水管里同时下两份不同的文件本来就是允许的（后端幂等、落点互不相同），
   * 锁整张表只会让人以为页面卡了。
   */
  const [batchBusy, setBatchBusy] = useState(false)

  /*
   * **只读正文抽屉**在看哪一份。`null` = 关着。
   *
   * 两条线共用一个抽屉，因为它俩要的是同一件事：取出一份文本、只读地看。
   *   `archive` 官方线的旧版本（先列出一份文件的那几个旧版本，再挑一份看）
   *   `mine`    用户线的那一份（就一份，打开直接读）
   *
   * 正文单独一个状态：读它要问后端（前端不碰文件系统），而且**会失败** ——
   * 失败要如实说，不许显示一段空正文假装它是空的。
   */
  const [viewer, setViewer] = useState<
    | { kind: 'archive'; fileName: string; rows: ArchivedFile[] }
    | { kind: 'mine'; fileName: string; path: string }
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
   * **编辑器**（临时编辑那条链）。`null` = 没在改。
   *
   * 正文是**临时文件里那份**（后端给的），不是官方原件的 —— 改谁也不动 `mkp/`。
   * `draftError` 是"草稿自动落盘失败了"那一行：它安静地待在抽屉里，不占提示条
   * （用户可能还在打字，提示条会闪）。
   */
  const [editing, setEditing] = useState<{
    /** 改的是哪一条线 —— 保存按钮说什么、保存之后那句话说什么都由它决定 */
    origin: ActiveOrigin
    sourceFileName: string
    /** 用户线的落点（`null` = 官方线，落点由目录给） */
    path: string | null
    text: string
    reused: boolean
    draftError: string | null
  } | null>(null)
  /** 上一次真正落到临时文件里的正文。用它判断"值不值得再存一次" */
  const savedTextRef = useRef<string>('')

  /*
   * **起名字抽屉**（第十层改名 / 第十一层另存为一份新的，共用一个）：`null` = 关着。
   * `kind` 决定说哪一套话、按哪颗按钮、成功怎么说 —— 形状一模一样。
   * 改名预填现在这个名字；另存为**不预填**（作者：名字由用户明确指定，不做自动起名）。
   * 名字的门槛（空 / 路径 / 后缀 / 不覆盖）全在后端，这里不重复判断；
   * 失败原话留在抽屉里（不弹提示条，别把用户刚打的字顶掉）。
   */
  const [naming, setNaming] = useState<{
    kind: 'rename' | 'copy'
    row: PresetTableRow
    name: string
    busy: boolean
    error: string | null
  } | null>(null)

  /* 「更多」层高的下拉锚点与开关 —— 层高值多，chips 一排放不下时收进这里 */
  const [moreOpen, setMoreOpen] = useState(false)
  const moreRef = useRef<HTMLButtonElement>(null)

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
   * 行键按「全部机型」那一档算（mine 行的键里带机型筛选，见 `originCellOf`）；
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
   * 契约里**连签名都没有**的那一件事（复制链接）——
   * 用户文件那四件都已经接上了：重命名与删除在第十层、另存为一份新的在第十一层、
   * 在文件管理器里显示在第十三层，都不在这里。
   *
   * 不发请求 —— 没有可发的方法。就地说清「还没有对应的实现」（A2 人话化：
   * 发生了什么 + 能干什么；缺的是哪个方法记在 `MISSING_METHOD` 里，给开发对账用），
   * 而假装成功（弹个「已删除」然后什么都没发生）比说不出话糟得多。
   */
  const sayNoContract = (row: PresetTableRow) => {
    setNote({ text: `${noContractText()}（${row.fileName}）`, bad: true })
  }

  /**
   * 下载。两条路：
   *
   *   目录登记的交付预设（`releaseUid` 在）  → `downloadRelease(fileName)`，**真的能下** ——
   *     走新世界下载管道落进下载区 `mkp/`，本地表跟着多出一行
   *   官方仓库的文件                        → 契约里有签名，所以**照调**。
   *     假后端一定抛 `NotImplementedError`（自带人话 hint，A2），界面接住原样显示
   *     —— 不许整页白屏，也不许静默吞掉（吞掉就等于把「哪个口子没接」藏起来）
   */
  const download = (row: PresetTableRow) => {
    if (row.releaseUid !== undefined) {
      /*
       * 盘上那份不对劲的两档（旧版本 / 认不出）走的是**同一条下载管道**（再下一遍，
       * 旧份自动归档）—— 动词统一叫「更新」（与按钮一致：云端有更新就该说更新，
       * 「重新下载」那种吓唬人的说法不再出现在动作上）；认不出的实情在结果那句话里说。
       * 别在这里分支去找"另一个命令"：没有那个命令。
       */
      const updating = row.releaseState === 'tampered' || row.releaseState === 'old'
      const verb = updating ? '更新' : '下载'
      setBusyKey(row.rowKey)
      setNote({ text: `正在${verb} ${row.fileName}…`, bad: false })
      /* 过程如实说：一次调用一路水位，后端推到哪说到哪 —— 不编一个分母，也不转空圈 */
      data
        .downloadRelease(row.fileName, (t) => setNote({ text: tickText(t), bad: t.stage === 'failed' }))
        .then(
          () => {
            setBusyKey(null)
            setNote({
              text: updating
                ? row.releaseState === 'tampered'
                  ? `已更新 ${row.fileName} —— 盘上那份认不出的，现在换成了目录登记的当前版本`
                  : `已更新 ${row.fileName} —— 旧的那一份进了归档（archive/），没有删`
                : `已下载 ${row.fileName} 到本机预设目录 —— 本地表里现在有它了`,
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
    if (row.kind === null) {
      /* 认不出类别的东西（用户自己那份 `.json`）不进官方下载那条路：它本来也不在仓库里 */
      setNote({ text: `${row.fileName}：认不出它是哪一类，走不了「下载」这条路`, bad: true })
      return
    }
    const ref: FileRef = { kind: row.kind, fileName: row.fileName, path: row.path }
    setBusyKey(row.rowKey)
    setNote({ text: `正在请壳下载 ${row.fileName}…`, bad: false })
    api.downloadFiles([ref]).then(
      () => {
        /* 真后端接上以后走这一支。这一轮到不了这里 —— 假后端一定抛 */
        setBusyKey(null)
        setNote({ text: `已交给外壳下载 ${row.fileName}`, bad: false })
      },
      (e: unknown) => {
        setBusyKey(null)
        /* 错误话术统一走 errorText：NotImplementedError 自带人话 hint（A2），
           不再按异常类型在前端拼"尚未实现：downloadFiles"那种术语 */
        setNote({ text: `下载失败：${errorText(e)}`, bad: true })
      },
    )
  }

  /**
   * 批量：把「未下载 + 需更新」的那些**一次交给后端** —— 多份单文件操作的组合，
   * 不是第二套机制（走同一个 `downloadCatalogFiles`，Rust 侧逐份跑同一个 `deliver`）。
   *
   * 结局**逐份**收：全成 / 有名有姓地列出没成的。**命令级失败是另一档**（比如没配数据源）：
   * 那时一份都没发出去，不能说成"全都失败了" —— 两句话不一样，用户要做的也不一样。
   */
  const runBatch = () => {
    const { fileNames } = page.pending
    if (fileNames.length === 0 || batchBusy) return
    const { label } = releaseBatchText(
      page.pending.missing,
      page.pending.stale,
      page.pending.tampered,
    )
    setBatchBusy(true)
    setNote({ text: `正在${label}…`, bad: false })
    data
      .downloadReleaseBatch(fileNames, (t) =>
        setNote({ text: tickText(t), bad: t.stage === 'failed' }),
      )
      .then(
        (outcomes) => {
          const bad = outcomes.filter((o) => !o.ok)
          const done = outcomes.length - bad.length
          setNote(
            bad.length === 0
              ? { text: `${label}完成：${done} 份都落进下载区了`, bad: false }
              : {
                  text: `${label}：${outcomes.length} 份里 ${done} 份成了、${bad.length} 份没成 —— 没成的那几份在下面；本机那几份保持原样`,
                  bad: true,
                  lines: bad.map(outcomeText),
                },
          )
        },
        (e: unknown) => {
          setNote({
            text: `这一批没能发出去（一份都没下）：${errorText(e)}`,
            bad: true,
          })
        },
      )
      .finally(() => setBatchBusy(false))
  }

  /**
   * 读一份正文（只读）。**两条线共用这一个口子**，只是问的命令不同：
   * 归档区那份走 `readArchivedText`，用户自己那份走 `readUserPresetText`。
   * 失败照实说（与下载 / 应用同一条规矩：不吞、不装）。
   */
  const readBody = (path: string, which: 'archive' | 'user') => {
    setBody({ path, text: null, error: null, loading: true })
    const ask = which === 'user' ? api.readUserPresetText(path) : api.readArchivedText(path)
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
   * 打开用户线那一份的正文（「看正文」）。就一份文件，打开直接读 —— 没有列表这一层。
   *
   * **这一层只能看**：改它要先经过「改这份」那条链（那会另存出一份自己的）。
   */
  const openMine = (row: PresetTableRow) => {
    menu.close()
    setEditing(null)
    setViewer({ kind: 'mine', fileName: row.fileName, path: row.path })
    readBody(row.path, 'user')
  }

  /**
   * **开始改这一份**：让后端把正文复制进临时文件，然后把编辑器打开。
   *
   * 2026-10-08 起**编辑的对象只有用户的工作副本**（本地表 MKP 档只列它）——
   * 官方基线不进用户世界，也没有"改官方那份"这条入口。前置条件全在后端拦
   * （认得出是 TOML + 读得出来），这里不重复判断。同一份的草稿还在的话后端会返回它
   * （`reused`），于是"改到一半关掉再回来"接着改。
   */
  const openEdit = (row: PresetTableRow) => {
    menu.close()
    setViewer(null)
    data.beginEdit(row.fileName, 'mine', row.path).then(
      (draft) => {
        savedTextRef.current = draft.text
        setEditing({
          origin: draft.origin,
          sourceFileName: draft.sourceFileName,
          path: draft.path,
          text: draft.text,
          reused: draft.reused,
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
   * 落盘的是**临时文件** —— 官方原件一动不动，这也是"改到一半关掉还在"的来源。
   * 失败写进抽屉里那一行（不弹提示条：用户还在打字，别拿一条会闪的条子打断他）。
   */
  useEffect(() => {
    if (editing === null || editing.text === savedTextRef.current) return
    const text = editing.text
    const timer = window.setTimeout(() => {
      data.putDraft(text).then(
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

  /** 放弃这次编辑：丢草稿（原文件与下载区全程没被碰过，所以它天生安全） */
  const discardEdit = () => {
    data.discardDraft().then(
      () => {
        setEditing(null)
        setNote({ text: '已放弃这次编辑 —— 你的那份从头到尾没有被改过', bad: false })
      },
      (e: unknown) =>
        setNote({ text: `放弃不了：${errorText(e)}`, bad: true }),
    )
  }

  /** 保存：**写回它自己**（第八层：同一个文件，不会多出一份） */
  const commitEdit = () => {
    data.commitDraft().then(
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
    setNaming({ kind: 'rename', row, name: row.fileName, busy: false, error: null })
  }

  /**
   * **另存为一份新的**（第十一层）：我的文件 → 我的文件，按字节复制。
   *
   * 名字**不预填** —— 作者定死：用户明确指定目标名字，目标存在就拒绝、让他自己换；
   * 不做"复制后自动改名"这种智能行为。别的规矩（血统原样带过去 / 不覆盖 / 不碰状态）
   * 都在后端。
   */
  const openCopyAs = (row: PresetTableRow) => {
    menu.close()
    setViewer(null)
    setEditing(null)
    setNaming({ kind: 'copy', row, name: '', busy: false, error: null })
  }

  /** 起名字抽屉那一颗按钮（改名 / 另存为共用这一条提交路） */
  const submitNaming = () => {
    if (naming === null || naming.busy) return
    const name = naming.name.trim()
    if (name === '') {
      setNaming({ ...naming, error: '新名字不能是空的' })
      return
    }
    setNaming({ ...naming, busy: true, error: null })
    const ask =
      naming.kind === 'rename'
        ? data.rename(naming.row.path, name)
        : data.copyAsNew(naming.row.path, name)
    ask.then(
      (done) => {
        setNaming(null)
        setNote({
          text:
            naming.kind === 'rename'
              ? `已改名：${naming.row.fileName} → ${done.fileName} —— 只换了名字，内容与血统一个字节没动`
              : `已另存为一份新的：${done.fileName}（${done.path}）—— 原文件一个字节没动，血统原样带过去了`,
          bad: false,
        })
      },
      (e: unknown) =>
        setNaming((cur) =>
          cur === null
            ? cur
            : { ...cur, busy: false, error: errorText(e) },
        ),
    )
  }

  /**
   * **删除**（2026-10-06 一切皆可删）：我的文件走用户线（**真删除**）；
   * 切片器交付行走交付线（删了回「未下载」，随时可从云端重下）。
   * 二次确认长在菜单里（`danger` + `confirm`）；正在使用 / 有草稿不再拦 ——
   * 后端把属于这一份的状态一并清掉（确认框讲清了）。
   */
  const runRemove = (row: PresetTableRow) => {
    setNote({ text: `正在删除 ${row.fileName}…`, bad: false })
    const done =
      row.origin === 'release' ? data.removeRelease(row.fileName) : data.remove(row.path)
    done.then(
      () =>
        setNote({
          text:
            row.origin === 'release'
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

  /** 置顶是纯前端的排序，真的能用 —— 落 localStorage，刷新还在 */
  const togglePin = (row: PresetTableRow) => {
    page.togglePin(row.pinKey)
    setNote({
      text: row.pinned
        ? `已取消置顶 ${row.fileName}（只影响这张表的顺序）`
        : `已置顶 ${row.fileName}（只影响这张表的顺序，不是「设为当前」）`,
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
   * 回来用户线已重读 —— 列表的机型 / 版本两列以文件为准。
   */
  const runSetAttribution = (row: PresetTableRow, machineId: string, versionId: string) => {
    const m = data.machines.find((x) => x.id === machineId)
    const v = m?.versions.find((x) => x.id === versionId)
    return data.setMineMachineVersion(row.path, machineId, versionId).then(
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
   * 真后端还会多两种（切片器路径没配、目标已存在）。不吞、不假装成功。
   */
  const runLive = (row: PresetLocalRow) => {
    /* 切片器：照旧走契约那一个写（假后端内存，刷新还原），成功要发提示条 */
    if (page.kind !== 'mkp') {
      const slicerId = row.assetId
      if (slicerId === undefined) return /* 到不了这里：切片器行必有 asset id */
      setBusyKey(row.rowKey)
      data.copy(slicerId).then(
        () => {
          setBusyKey(null)
          setNote({
            text: `已复制 ${row.fileName} 到切片器目录（假后端只改内存，刷新会还原）`,
            bad: false,
          })
        },
        (e: unknown) => {
          setBusyKey(null)
          setNote({ text: `复制失败：${errorText(e)}`, bad: true })
        },
      )
      return
    }

    /*
     * **用户自己那份也能应用**（第七层）：同一个写口、同一条底账 ——
     * 只是落点由用户根给（用户目录里可以自己分文件夹，所以交的是**路径**）。
     * 后端那一道闸是"在 presets-mine/ 那一格里 + 盘上真有 + 是 TOML 预设"。
     */
    if (row.origin === 'mine') {
      setBusyKey(row.rowKey)
      data.apply(row.fileName, 'mine', row.path).then(
        () => setBusyKey(null),
        (e: unknown) => {
          setBusyKey(null)
          setNote({ text: `应用失败：${errorText(e)}`, bad: true })
        },
      )
      return
    }

    /*
     * 官方交付行：「应用」只有一个写 —— 唯一底账（新世界 `run/active-preset.json`）。
     * 动作认 **fileName**（目录登记的文件名 = 使用中指针的口径 = 首页反填要读的那一条）。
     * Rust 侧会校验"已下载且 SHA 与目录登记的当前版本对得上"，
     * 没下载 / 字节漂了 / 是旧版本都应用不成，错误原样冒给提示条。
     */
    if (row.releaseUid === undefined) {
      /*
       * 兜底（正常到不了）：A2 修缝后，没有交付身份的官方 MKP 行在表格那一层
       * 就不给「应用」按钮了（`PresetTable` 的灰杠 + `NO_ASSET_WHY` 人话原因）。
       * 这一格留着防行形状再变化时静默出错 —— 文案同样是人话（发生了什么 + 能干什么）。
       */
      setNote({ text: `${row.fileName}：${NO_ASSET_WHY}`, bad: true })
      return
    }
    setBusyKey(row.rowKey)
    data.apply(row.fileName).then(
      () => setBusyKey(null),
      (e: unknown) => {
        setBusyKey(null)
        setNote({ text: `应用失败：${errorText(e)}`, bad: true })
      },
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

  const bbsEntry = (row: PresetTableRow): ContextMenuEntry => ({
    id: 'bbs',
    label: '在 BBS 预设查看器中打开',
    disabled: bbsWhyNot(row),
    onSelect: () => onOpenBbs?.(row.fileName),
  })

  /**
   * 「重命名」为什么不能点（第十层）。只有**我的文件**能改 ——
   * 灰一个项不说为什么，用户只会以为坏了。
   */
  const renameWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'mine'
      ? undefined
      : '只有「我的文件」能改名（只动名字、字节一个不动）—— 官方那份不归你改名'

  /**
   * 「另存为一份新的」为什么不能点（第十一层）：
   *
   * - **我的文件**：本来就开放（我的 → 我的，按字节复制、血统原样带过去）；
   * - 其余（切片器交付行 / 官方仓库文件）：不是"我的文件"，另行处置 ——
   *   切片器自有「复制到切片器目录」那条路。
   */
  const copyWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'mine'
      ? undefined
      : '只有「我的文件」能另存为一份新的 —— 官方那份不归你复制'

  /**
   * 「在 Finder 中显示」为什么不能点（第十三层）：只有**我的文件**在本机有个"家"——
   * 官方那份住在程序自己管的区域，或者根本还没下载（仓库表）。
   */
  const revealWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'mine'
      ? undefined
      : '住在程序自己管理的区域 —— 能这样打开的是「我的文件」（你自己的目录里的那份）'

  /**
   * 「删除」对哪几行给。**一切皆可删**（作者裁决 2026-10-06，此前拦得太死）：
   *
   * - 我的文件：真删；**正在使用的那份也给删** —— 后端把使用中指针一并撤下
   *   （悬空的「使用中」比「没在用」糟），有草稿的连草稿一起丢，确认框讲清；
   * - 切片器交付行：删了回到「未下载」，随时可从云端重新下载；
   * - 官方仓库文件（切片器仓库行）的本地副本不给删（它们走资源那一套命令）；
   * - 云端表没有删除 —— 那不是"不让"，是"不能"：客户端删不了仓库里的东西
   *   （云端表的菜单本来就不含这一项，不经过这里）。
   */
  const removeWhyNot = (row: PresetTableRow): string | undefined =>
    row.origin === 'official'
      ? '官方仓库文件不在这里删 —— 能删的是你自己那份与目录登记的交付文件'
      : undefined

  /** 删除确认框的第二行：**代价跟着行的来源走** —— 能重下的说能重下，真删的说真删 */
  const removeConfirmDetail = (row: PresetTableRow): string => {
    if (row.origin === 'release') {
      return '它回到「未下载」，随时可以从云端重新下载（字节有目录 SHA 锚定，不会丢什么）。' +
        '它正在被使用的话，使用中会一并撤下。'
    }
    const live = 'live' in row && row.live
    return live
      ? '这份正在使用中，删除会一并撤下使用；有没保存的草稿也一并丢弃。' +
          '删了就没了 —— 程序没有垃圾桶、也没有归档（删掉就是真删掉）。'
      : '这是你自己的文件，删了就没了 —— 程序没有垃圾桶、也没有归档（删掉就是真删掉）。'
  }

  const entriesOf = (row: PresetTableRow | null): ContextMenuEntry[] => {
    if (row === null) return []

    /*
     * 云端表：**仓库里的东西删不掉**（客户端管不到云端）——但**本机那一份可以删**。
     *
     * ★ 2026-10-08 补上这一项。作者的实测是：「我在本地删除，我在云端看到的还是显示已下载」
     * —— 因为他删的是**自己那份副本**（`presets-mine/`），而那一行说的「已下载」指的是
     * **下载区里那一份**（另一个地方的东西，`deleteDeliveryFile` 早就有，只是一直没挂出来）。
     * 两件事都说得通，缺的是"把 `已下载` 撤销掉"的那颗按钮：现在它在这儿 ——
     * 删掉本机这份，那一行回到「未下载」，随时能再下一份（云端不受影响）。
     */
    if (row.scope === 'cloud') {
      /* 只有**交付行且真下过**才给这一项：官方仓库行本机那份不归这一页管 */
      const hasLocalCopy = row.origin === 'release' && row.downloaded
      return [
        {
          id: 'download',
          /* 一个动作一个词（作者 2026-10-08：「那颗按钮就叫下载吧」）；"为什么"是状态列那格的事 */
          label: UPDATE_ACTION_TEXT.missing,
          onSelect: () => download(row),
        },
        ...(hasLocalCopy
          ? ([
              { separator: true },
              {
                id: 'removeLocal',
                label: '删除本机这份',
                danger: true,
                confirm: {
                  question: `删掉本机那份 ${row.fileName}？`,
                  detail: removeConfirmDetail(row),
                },
                onSelect: () => runRemove(row),
              },
            ] satisfies ContextMenuEntry[])
          : []),
        {
          id: 'link',
          label: '复制链接',
          onSelect: () => sayNoContract(row),
        },
        { id: 'detail', label: '查看详情', onSelect: () => setExpandedKey((k) => (k === row.rowKey ? null : row.rowKey)) },
        bbsEntry(row),
      ]
    }

    /*
     * 临时编辑的入口：**只有用户的工作副本**（本地表 MKP 档只列它；
     * 认不出是哪一类的（`.json`）不给 —— 这一层只改 TOML 预设；
     * **第九层读不出来的**也不给（改的入口同样过文件级检查，不给必被拒的项）。
     */
    const canEdit =
      row.origin === 'mine' && row.kind === 'mkp_preset' && row.mineState !== 'unreadable'
    /*
     * 内容存疑的那两档（旧版本 / 内容异常）：**不许复制** ——
     * 与"不许应用、不许改"同一条边界（第三圈第 6 层）：盘上那份的字节我们不认，
     * 不能让它换个名字继续活着。禁用一定带原因 —— 灰一个项不说为什么，用户只会以为坏了。
     */
    const suspect = isSuspectRelease(row.releaseState)

    return [
      { id: 'pin', label: row.pinned ? '取消置顶' : '置顶', onSelect: () => togglePin(row) },
      ...(canEdit
        ? [{ id: 'edit', label: EDIT_TEXT.cell, onSelect: () => openEdit(row) }]
        : []),
      {
        id: 'copy',
        label: '另存为一份新的',
        /* 我的文件 → 我的文件（第十一层，按字节复制、血统原样带过去）；
           其余行按 `copyWhyNot` 说明为什么不行（内容存疑那一句优先） */
        disabled: suspect ? RELEASE_SUSPECT_WHY : copyWhyNot(row),
        onSelect: () => openCopyAs(row),
      },
      {
        id: 'rename',
        label: '重命名',
        /* 第十层：只有「我的文件」能改名（只动名字、字节一个不动） */
        disabled: renameWhyNot(row),
        onSelect: () => openRename(row),
      },
      {
        id: 'reveal',
        label: REVEAL_LABEL,
        /* 第十三层：只有「我的文件」能这样打开（打开的是系统文件管理器，不是我们的界面） */
        disabled: revealWhyNot(row),
        onSelect: () => runReveal(row),
      },
      { id: 'detail', label: '查看详情', onSelect: () => setExpandedKey((k) => (k === row.rowKey ? null : row.rowKey)) },
      bbsEntry(row),
      { separator: true },
      {
        id: 'remove',
        label: '删除',
        danger: true,
        /* 我的文件真删；切片器交付行删了可重下；正在使用的那份删掉时后端会一并撤下使用
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
  /* 批量那一行的字：这一批里有什么，决定它是「下载」「更新」还是「下载并更新」 */
  const batch = releaseBatchText(
    page.pending.missing,
    page.pending.stale,
    page.pending.tampered,
  )
  /* 这一份在归档里有几个旧版本。按文件名对（与下载 / 应用同一套口径） */
  const archiveCountOf = (fileName: string): number =>
    data.archived.filter((a) => a.fileName === fileName).length

  /* 层高 chips 一排放不下的值收进「更多」（先摆 6 个） */
  const layerMain = page.slicerFilters.layers.slice(0, 6)
  const layerMore = page.slicerFilters.layers.slice(6)

  /*
   * 搜索框。两种类型共用这一个节点，只是排位不同：MKP 在 pill 左边、切片器在漏斗右边
   * （G07-1 定稿的排位）。宽度**固定 180**（mini 收 130，见 CSS）——「弹性 180–320」
   * 那一版被作者否掉（2026-10-04：「不要搞这么宽」，小的感觉不在宽度）。
   * 真正的解在**高度**：36px，故意比 32px 的分段控件高 4px —— 分段是实心灰槽、
   * 边界对比强，搜索是白底细边框，标称同高视觉上仍矮一截（作者：「高度上看着
   * 比切换器小」）。
   */
  const searchNode = (
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
   * 「共 N 项」+ 仓库台账（G07-1 的 4 号：住页脚，与右键提示同行；
   * compact / mini 的 MKP 档由工具栏渲染这一份，页脚那份不画 —— 见 .foot 的说明）。
   * **包成一个不拆分的整体** —— 窄窗折行时两样一起走，台账的竖线永远不会落单
   * （作者圈过那半截孤线）。
   *
   * 台账那两个可数的数**跟着当前类型档走**（作者 2026-10-02：「'仓库 9' 这种
   * 全 catalog 数字不应该混在当前类型的业务语境里」）：仓库数的是这一档类型
   * （MKP / 切片器）在全机型下的文件数、我的数的是这一档下用户文件的个数 ——
   * 图标 / 模型不归这一页，哪个数里都不含它们。
   * **MKP 档没有「本机」这一格**（2026-10-08）：官方基线不进用户世界，用户只有他那一份 ——
   * 台账就是「仓库（云端有几个）· 我的（你有几个）」。切片器档照旧带「本机」
   * （官方副本的总数，老契约 getLocalFiles 的读数）。
   */
  const mineCount = mineCountOfAxis(data.mine, page.kind)
  const metaNode = (
    <span className={s.meta}>
      <span
        className={s.counts}
        title={`当前这张表（${page.scope === 'local' ? '本地' : '云端'}）在这一档机型、类型与搜索词之下有几行。筛前 ${table.total} 项`}
      >
        共 {table.rows.length} 项
      </span>
      <span
        className={s.ledger}
        title={
          page.kind === 'mkp'
            ? `仓库：这一档类型在全机型下一共几个官方文件（已剔掉仅归档的）· 我的：你自己的文件里属于这一档的几个（getUserPresetFiles，扫 presets-mine；云端没有它们）。官方基线不进用户世界（2026-10-08）。${DOWNLOAD_WHY}`
            : `仓库：这一档类型在全机型下一共几个官方文件（已剔掉仅归档的）· 本机：已有几个官方副本（getLocalFiles，演示集合）· 我的：你自己的文件里属于这一档的几个（getUserPresetFiles，扫 presets-mine；认不出类别的两档都算；云端没有它们）。${DOWNLOAD_WHY}`
        }
      >
        仓库 {treeCountOfAxis(data.tree, page.kind)} ·{' '}
        {page.kind === 'mkp' ? (
          <>我的 {mineCount}</>
        ) : (
          <>
            本机 {data.localIds.length} + 我的 {mineCount}
          </>
        )}
      </span>
    </span>
  )

  return (
    <div ref={rootRef} className={s.page} data-density={density}>
      <FieldLayer>
        {/*
         * 工具栏区（G07-1 收编；2026-10-04 作者再裁决：**恒两排**）——
         *
         *   MKP     第一排：类型分段 位置分段(右) 搜索
         *           第二排：pill + 计数台账（台账顶到右线）—— 全档一样，不再随窗宽并排
         *   切片器  第一排：类型分段 位置分段(右) 漏斗 搜索；第二排：喷嘴 chips；第三排：层高 chips
         *
         * pill = PresetStatusPill：左拍「● 已应用 …」点击 = 定位（清筛选 + 闪行），
         * 右拍 = 机型筛选漏斗。切片器没有「已应用」，它的筛选是独立漏斗
         * （PresetMachineFilter，就是 pill 的右拍单拎出来），排在搜索左边。
         * 计数与台账：MKP 档**恒住 pill 右边**（作者 2026-10-04：以两排稿为准；
         * 此前宽档住页脚、小窗回 pill 右边，一页两种摆法），页脚只留右键提示；
         * 切片器的筛选排本来就满（喷嘴 / 层高 chips），它的计数与台账照旧住页脚。
         * **一个控件都不藏进「更多」**：藏起来等于让人猜。
         * 换行不靠窗宽碰运气：.tbBreak（flex-basis 100% 的零高断行）把 pill / chips
         * 钉在各自的排上，窄窗只是让第一排自己折。
         */}
        <div className={s.toolbar} data-kind={page.kind}>
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
              <PresetStatusPill
                applied={page.applied}
                appliedFileName={page.appliedFileName}
                appliedMachineText={page.appliedMachineText}
                appliedIsMine={page.appliedIsMine}
                onLocate={locateApplied}
                machines={data.machines}
                machineId={data.machineId}
                onPick={pickMachine}
              />
              <button
                type="button"
                className={s.chip}
                title="对比台：选 2~3 份「我的预设」，同一项并排看、可以直接改。只在你自己那几份之间比 —— 官方那份（隐藏基线）不进这张台子"
                onClick={() => setCompareOpen(true)}
              >
                参数对比
              </button>
              {metaNode}
            </>
          ) : (
            <>
              <PresetMachineFilter
                machines={data.machines}
                machineId={data.machineId}
                onPick={pickMachine}
              />
              {searchNode}
              <span className={s.tbBreak} aria-hidden />
              <div className={`${s.filterGroup} ${s.groupNozzle}`}>
                <span className={s.filterLabel}>喷嘴</span>
                <span className={s.chips}>
                  <button
                    type="button"
                    className={s.chip}
                    data-on={page.slicerFilters.nozzle === ''}
                    onClick={() => page.slicerFilters.setNozzle('')}
                  >
                    全部
                  </button>
                  {page.slicerFilters.nozzles.map((v) => (
                    <button
                      key={v}
                      type="button"
                      className={s.chip}
                      data-on={page.slicerFilters.nozzle === v}
                      onClick={() => page.slicerFilters.setNozzle(v)}
                    >
                      {v}
                    </button>
                  ))}
                </span>
              </div>

              <span className={`${s.tbBreak} ${s.tbBreak2}`} aria-hidden />

              <div className={`${s.filterGroup} ${s.groupLayer}`}>
                <span className={s.filterLabel}>层高</span>
                <span className={s.chips}>
                  <button
                    type="button"
                    className={s.chip}
                    data-on={page.slicerFilters.layer === ''}
                    onClick={() => page.slicerFilters.setLayer('')}
                  >
                    全部
                  </button>
                  {layerMain.map((v) => (
                    <button
                      key={v}
                      type="button"
                      className={s.chip}
                      data-on={page.slicerFilters.layer === v}
                      onClick={() => page.slicerFilters.setLayer(v)}
                    >
                      {v}
                    </button>
                  ))}
                  {layerMore.length > 0 && (
                    <>
                      <button
                        ref={moreRef}
                        type="button"
                        className={s.chip}
                        data-on={layerMore.includes(page.slicerFilters.layer)}
                        onClick={() => setMoreOpen((v) => !v)}
                      >
                        更多 {moreOpen ? '⌃' : '⌄'}
                      </button>
                      {moreOpen && (
                        <FieldPopover anchor={moreRef.current} onClose={() => setMoreOpen(false)}>
                          <ul className={s.moreList}>
                            {layerMore.map((v) => (
                              <li key={v}>
                                <button
                                  type="button"
                                  className={s.moreItem}
                                  data-on={page.slicerFilters.layer === v}
                                  onClick={() => {
                                    page.slicerFilters.setLayer(v)
                                    setMoreOpen(false)
                                  }}
                                >
                                  {v}
                                </button>
                              </li>
                            ))}
                          </ul>
                        </FieldPopover>
                      )}
                    </>
                  )}
                </span>
              </div>
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
                  <span className={s.noneBadge} title={STATUS_WHY.unavailable}>
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
               * 批量那一行：只在**云端表**、且这一批非空时出现。
               * 为什么在云端表：那里说的是"目录里有什么、你缺什么"，批量正是补这里；
               * 本地表说的是"我机器上有什么"（需更新那几份已经各带一颗「更新」）。
               * 为什么敢占一行高度：它的数**与眼前这张表同口径**（当前机型 + 这一档类型），
               * 而且带着动作 —— 不是那种"别处还有个数字"的噪音（那种以前删过）。
               */}
              {page.scope === 'cloud' && page.pending.total > 0 && (
                <div className={s.batch}>
                  <span className={s.batchText} title={batch.why}>
                    {batch.count}
                  </span>
                  <button
                    type="button"
                    className={s.batchBtn}
                    disabled={batchBusy}
                    title={batch.why}
                    onClick={runBatch}
                  >
                    {batchBusy ? '处理中…' : batch.label}
                  </button>
                </div>
              )}

              {/*
                「远端这一代读不懂」那一句（第三刀下半，作者定的产品规则 B）。
                ★ 与 `note` 分开：`note` 是"刚做的事怎么样了"，这一条是"你想给我的这份我读不了"。
                  **列表照常**（没有整表标红、没有逐行"不兼容"标签），只在这一格多一句提示。
                文案照抄作者原话，按钮只有一个去处：设置页的软件更新块。
              */}
              {data.needsNewerClient && (
                <div className={s.noteBad} role="status">
                  <p className={s.noteMain}>
                    <span>
                      <strong>此预设需要更新版 SupportEase</strong>
                      <br />
                      当前客户端版本过旧，暂不支持此预设文件。
                    </span>
                    {onOpenSettings !== undefined && (
                      <button
                        type="button"
                        className={s.noteAction}
                        onClick={onOpenSettings}
                      >
                        去更新
                      </button>
                    )}
                  </p>
                </div>
              )}

              {note !== null && (
                <div className={note.bad ? s.noteBad : s.note} role="status">
                  <p className={s.noteMain}>
                    {note.text}
                    <button
                      type="button"
                      className={s.noteClose}
                      aria-label="关闭这条提示"
                      onClick={() => setNote(null)}
                    >
                      ×
                    </button>
                  </p>
                  {/* 一次动作里逐份的结局：一行一份（批量里"哪一份坏了"是唯一有用的信息） */}
                  {(note.lines ?? []).map((line) => (
                    <p key={line} className={s.noteItem}>
                      {line}
                    </p>
                  ))}
                </div>
              )}

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
                /* 点同一行收起、点别行换行 —— 与右键菜单「查看详情」和参数页同一条规则 */
                onToggleExpand={(rowKey) => setExpandedKey((k) => (k === rowKey ? null : rowKey))}
                /*
                 * 操作列常驻：本地表那一颗按钮的字随类型变（应用 / 复制），云端表是下载。
                 * 右键菜单**照旧全在**，操作列只是把最常用的那一个摆到明面上。
                 */
                busyKey={busyKey}
                archiveCountOf={archiveCountOf}
                onOpenArchive={openArchive}
                onOpenMine={openMine}
                onEdit={openEdit}
                onLive={runLive}
                onDownload={download}
                /* 来源格「复制自 X」的定位落点；sourceLabel 决定官方交付行来源列显示 GitHub / Gitee */
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

          {/* 查看详情不再是模态框 —— 点行展开下方的内容 */}

          {/*
           * **只读正文抽屉**：两条线共用这一个外壳。
           *
           * 挂在 `.main` 里（与参数页同一条规矩）：遮罩只盖内容区，底栏与窗口右下的
           * resize 手柄还点得到。
           *
           *   官方线（`archive`）列出旧版本、挑一份看；**删除 / 恢复 / 用这份旧版本都没有**
           *   用户线（`mine`）  就一份，打开直接读；**改它 / 另存 / 保存都没有**（下一层的事）
           *
           * 两边的共同点：**只读**，读不出来照实说。
           */}
          <Drawer
            open={viewer !== null}
            title={viewer?.kind === 'mine' ? MINE_DRAWER.title : ARCHIVE_DRAWER.title}
            subtitle={
              viewer === null
                ? undefined
                : viewer.kind === 'mine'
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
                              <span className={s.archName} title={a.path}>
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
                            <p className={s.archWho} title={a.path}>
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
                                title={
                                  confirmingArchive === a.path
                                    ? '再点一下确认删除 —— 这一版删了就找不回（云端只有最新版）'
                                    : '删除这份旧版本（删了就找不回，云端只有最新版）'
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
                        {viewer.kind === 'mine' ? MINE_DRAWER.reading : ARCHIVE_DRAWER.reading}
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
                  {viewer.kind === 'mine' ? MINE_DRAWER.note : ARCHIVE_WHY}
                </p>
              </div>
            )}
          </Drawer>

          {/*
           * **参数对比台**（2026-10-08）：一个独立常驻工具，只在**用户自己的本地预设**
           * 之间比。挂在 `.main` 里（与抽屉同一条规矩）：遮罩只盖内容区。
           * **官方那份（隐藏 baseline）不进这张台子** —— 它是系统内部的东西，不是给用户比的。
           */}
          <CompareModal
            open={compareOpen}
            files={compareFiles}
            onClose={() => setCompareOpen(false)}
            onSaved={(text) => setNote({ text, bad: false })}
          />

          {/*
           * **编辑器抽屉**（临时编辑那条链）。改的是**临时文件**里的正文 ——
           * 所以这里没有"保存到官方"这种动作：只有「放弃」与「保存（写回我这份）」。
           *
           * 2026-10-08 起编辑的对象只有**用户的工作副本**（本地表 MKP 档只列它），
           * 所以这里只有一套说法：保存 = 写回它自己。
           *
           * 关掉（Esc / 点遮罩）= **只关，不丢**：草稿在盘上，回头点「改这份」接着改。
           * 真正丢掉草稿只有一个入口：footer 里那颗「放弃这次编辑」。
           */}
          <Drawer
            open={editing !== null}
            title={EDIT_TEXT.title}
            subtitle={editing === null ? undefined : (editing.path ?? editing.sourceFileName)}
            footer={
              <>
                <button type="button" className={s.editGhost} onClick={discardEdit}>
                  {EDIT_TEXT.discard}
                </button>
                <button type="button" className={s.editPrimary} onClick={commitEdit}>
                  {EDIT_TEXT.commit}
                </button>
              </>
            }
            onClose={() => setEditing(null)}
          >
            {editing !== null && (
              <div className={s.edit}>
                <p className={s.editNote}>{EDIT_TEXT.note}</p>
                {editing.reused && <p className={s.editReused}>{EDIT_TEXT.reused}</p>}
                <textarea
                  className={s.editArea}
                  value={editing.text}
                  spellCheck={false}
                  aria-label="预设正文"
                  onChange={(e) =>
                    setEditing((cur) => (cur === null ? cur : { ...cur, text: e.target.value }))
                  }
                />
                {editing.draftError !== null && (
                  <p className={s.editErr}>
                    {EDIT_TEXT.draftFailed}
                    {editing.draftError}
                  </p>
                )}
              </div>
            )}
          </Drawer>

          {/*
           * **起名字抽屉**（第十层改名 / 第十一层另存为一份新的，共用一个）：
           * 两个动作都只传一个名字，字节与状态全在后端管 —— 这里没有"保存内容"这回事。
           *
           * 名字的门槛全在后端（不许空 / 不许带路径 / 后缀保持原样 / 不覆盖 / 不自动改名），
           * 这里只把用户输入的字带过去；失败原话**留在抽屉里**（别把他刚打的字盖掉）。
           * 改名预填现在这个名字（改名后使用中指针与草稿在后端跟着走）；
           * 另存为**不预填**（名字由用户明确指定）。
           */}
          <Drawer
            open={naming !== null}
            title={naming?.kind === 'copy' ? MINE_COPY.title : MINE_RENAME.title}
            subtitle={naming?.row.path}
            footer={
              <>
                <button type="button" className={s.editGhost} onClick={() => setNaming(null)}>
                  取消
                </button>
                <button
                  type="button"
                  className={s.editPrimary}
                  disabled={naming?.busy === true}
                  onClick={submitNaming}
                >
                  {naming?.kind === 'copy' ? MINE_COPY.commit : MINE_RENAME.commit}
                </button>
              </>
            }
            onClose={() => setNaming(null)}
          >
            {naming !== null && (
              <div className={s.edit}>
                <p className={s.editNote}>
                  {naming.kind === 'copy' ? MINE_COPY.note : MINE_RENAME.note}
                </p>
                {naming.kind === 'rename' && naming.row.scope === 'local' && naming.row.live && (
                  <p className={s.editReused}>{MINE_RENAME.liveNote}</p>
                )}
                <input
                  className={s.renameInput}
                  value={naming.name}
                  spellCheck={false}
                  aria-label="新的文件名"
                  placeholder={naming.kind === 'copy' ? '新文件名' : undefined}
                  autoFocus
                  onChange={(e) =>
                    setNaming((cur) =>
                      cur === null ? cur : { ...cur, name: e.target.value, error: null },
                    )
                  }
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') submitNaming()
                  }}
                />
                {naming.error !== null && (
                  <p className={s.editErr}>
                    {naming.kind === 'copy' ? '没另存成：' : '没改成：'}
                    {naming.error}
                  </p>
                )}
              </div>
            )}
          </Drawer>
        </div>

        {/*
         * 页脚（G07-1 的 4 号收编；2026-10-04 起 MKP 档的计数与台账恒住工具栏第二排，
         * 见工具栏的说明）——页脚这一条：右键提示在左；**只有切片器**把
         * 「共 N 项 │ 仓库…」放在这里（它的筛选排满，pill 也不在）。
         * 「暂不支持」时没有表可右键，提示不画（计数与台账照旧 —— 那是全局事实）。
         */}
        <div className={s.foot}>
          {!page.unsupported && (
            <span className={s.footHint}>
              右键任意一行还有置顶 / 重命名 / 删除 / 查看详情（没有鼠标就 Shift+F10 或菜单键）
            </span>
          )}
          {page.kind === 'slicer' && metaNode}
        </div>

        <ContextMenu at={menu.at} entries={entriesOf(menu.target)} onClose={menu.close} />
      </FieldLayer>

      {/* 「定位」的闪烁层：盖在被定位那一行上的普通 div，位置尺寸由 locateApplied
          现量现设。为什么不闪行本身 —— 见 PagePresets.module.css 的 .locateFlash。 */}
      <div ref={flashRef} className={s.locateFlash} />
    </div>
  )
}

