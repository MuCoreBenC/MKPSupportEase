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
/* 内联自定义属性（`--scrollbar-pad`）要一个 `as CSSProperties` —— 见 `.editBody` 那一处 */
import type { CSSProperties } from 'react'
import { api, errorText } from '../../api'
import type { ArchivedFile, LocalPresetFile } from '../../api'
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
import { usePresetUsage } from '../state/usePresetUsage'
/* 设置页那一格「复制出来的也跟着云端更新」：本地行画不画「有新版」看它（2026-10-20） */
import { useCopiesFollowCloud } from '../state/deliveryState'
/* 「正文草稿被别处作废」那一嗓子（参数页保存时发）：编辑器还开着就按新内容重开 */
import { lastTextDraftDroppedPath, useTextDraftDrop } from '../state/draftState'
import { FieldLayer } from '../../components/field'
import { ContextMenu, useContextMenu } from '../../components/menu'
import type { ContextMenuEntry } from '../../components/menu'
import type { Density } from '../../hooks/useDensity'
import { detectPlatform } from '../../hooks/usePlatform'
import Icon from '../shell/icons'
/* 下载过程与逐份结局的措辞（`shared/download.ts`）—— 同一件事一处文案 */
import { tickText } from '../shared/download'
/* 「复制后处理脚本」全站那一份（与标题栏 MKP 字标、首页那颗同一个出口）—— 见那个文件的文件头 */
import { copyPostProcessScript } from '../postprocess/copyScript'
/* 全站通知（右上角那三格通知中心）："改动没保存"这类跨一步的话要说得比提示条响
   —— 没有 index，全站都按这个文件引（与 `copyScript` / `PageSettings` 一致）。
   `dismiss` 给下面那个"只留一条"的小封装用 */
import { dismiss, notify } from '../notify/notify'
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
  HEAD_LOCKED,
  LOCAL_COPY,
  LOCAL_DRAWER,
  LOCAL_NOT_PRESET_WHY,
  LOCAL_RENAME,
  PARAMS_TEXT,
  RESTORE_DEFAULT_NAME,
  defaultNameAvoiding,
  defaultNameOf,
  duplicateNameOf,
  isDetachedCopy,
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
import { splitHeader, tomlTokens } from './tomlPaint'
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

/*
 * 「有未保存的改动」通知停留时长（2026-10-16）：原来跟「复制后处理脚本」一档（5200），
 * 作者当晚看过说「这个通知太短暂了，时间可以延长一点」→ **9000**。
 * 它是一条"要去行动"的话（去保存），得留够读完再走一趟的工夫。
 *
 * ★ 现在它上面还挂着一颗动作按钮（`EDIT_TEXT.discard`）—— 9000 对一条"要去读"的话够，
 *   对一颗"要去点"的按钮偏紧（`notify.ts` 里带动作的那类默认是 0 = 不自己走）。
 *   先照作者定的 9000 不动：真机上看过嫌它走得快，改成 0 是一句话的事。
 */
const UNSAVED_NOTICE_MS = 9000

/* 「文件头改不了」那一句：一次短促的说明，不是"要去行动"的话 —— 说完就走 */
const HEAD_LOCKED_MS = 2600

/*
 * 撤销 / 重做历史**按 path 记在模块级**（2026-10-16 作者：「我这次再次进来，有没有那种
 * 可以做到还保留撤销重做的呀」）——组件 state 会随关闭 / 组件重挂丢，这里不会（不重启就一直在）。
 *
 * 每格：`undo`/`redo` 两个文本快照栈 + `head`（写下这一格时手上的正文）。
 * 重开时只认 `head === 读到的草稿正文` 的缓存；对不上（草稿被保存 / 被别处动过）就丢掉。
 * 作废、保存、参数页覆盖这几条路都不用显式清 —— 文本一换，`head` 自己就对不上了。
 */
const textHistCache = new Map<string, { undo: string[]; redo: string[]; head: string }>()

/**
 * 「有未保存的改动」**屏幕上只留一条** —— 关窗那条（"刚才…"）与重开那条（"上次…"）
 * 说的是同一件事；不动手的话两条会叠在一起（2026-10-16 截图里就是这样，像在重复念叨）。
 * 新的一条上来先把旧的那条 `dismiss` 掉。
 *
 * ★ 2026-10-16 下半场：这一条上多了一颗**放弃修改**（作者：「我希望这个时候那个通知的话，
 *   他可以给我就是选择那个放弃保存的那种……用户不知道退出编辑模式就是取消上一次的修改」）。
 *   于是它不再只是"一句通知"：改动落地（保存）或作废（退出编辑模式 / 放弃修改）之后
 *   必须**收掉**它，否则它会继续待在右上角说一件已经不存在的事 —— 三处都调
 *   `clearUnsavedNotice`。
 *
 * `onDiscard` 由调用方给（它要按"此刻模态框开没开"走两条不同的路，见 `discardNowRef`）。
 */
let unsavedNoticeId: string | null = null
function clearUnsavedNotice() {
  if (unsavedNoticeId === null) return
  dismiss(unsavedNoticeId)
  unsavedNoticeId = null
}

function notifyUnsaved(text: string, onDiscard: () => void) {
  clearUnsavedNotice()
  unsavedNoticeId = notify({
    kind: 'warn',
    title: '有未保存的改动',
    text,
    ttl: UNSAVED_NOTICE_MS,
    actions: [{ label: EDIT_TEXT.discard, onClick: onDiscard }],
  })
}

/**
 * 「文件头改不了」（`HEAD_LOCKED`）：试了一次没改成，说一句就走。
 *
 * 按住 `Backspace` 会连发（每一个键都判一次），所以**卡片已经在屏幕上就不再发一条**
 * —— 同一句话重复入队只会让它闪。ttl 到了把 id 落回 `null`，下一次再试还能说。
 */
let headLockedNoticeId: string | null = null
function notifyHeadLocked() {
  if (headLockedNoticeId !== null) return
  const id = notify({
    kind: 'warn',
    title: HEAD_LOCKED.title,
    text: HEAD_LOCKED.text,
    ttl: HEAD_LOCKED_MS,
  })
  headLockedNoticeId = id
  window.setTimeout(() => {
    if (headLockedNoticeId === id) headLockedNoticeId = null
  }, HEAD_LOCKED_MS + 200)
}

/**
 * **把文件头原样钉回去**：万一有一条路绕过了按键那道闸（输入法合成、拖放文本进来），
 * 头被动过的这一刻把它换回来、只留正文（`cur` = 改之前的文本、`next` = 改之后的）。
 *
 * 分界怎么重新认：先看正文第一行（`[toolhead]` 那种）在改后的文本里还在不在 ——
 * 在就**从它那儿切**（用户可能正好把头尾那个换行删了，"头 = 连续注释行"会连着把
 * 紧挨的 `[表]` 一起吞掉，那就把一个表头弄丢了）；找不到才退回按行切。
 */
function keepHead(cur: string, next: string): string {
  const before = splitHeader(cur)
  if (next.startsWith(before.head)) return next
  const first = before.body.split('\n')[0]
  const at = first === '' ? -1 : next.indexOf(first)
  return before.head + (at >= 0 ? next.slice(at) : splitHeader(next).body)
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
  /*
   * 详情卡里「后处理 N 次」读的那本账（本机学习账 `run/usage.json`）。
   * 拿 `expandedKey` 当刷新信号：这一页是常驻的（六页签都在 DOM 里），只靠挂载读一次
   * 会一直显示旧的；每开一行重读一遍 —— 刚跑完一趟后处理回来，展开就看见新数。
   */
  const usage = usePresetUsage(expandedKey)
  /*
   * 设置页那一格「复制出来的也跟着云端更新」（2026-10-20，出厂关）：
   * 关着时复制 / 导入出来的那几份**不挂「有新版」** —— 作者：
   * 「我设置里面没开，它就不应该把那些复制出来的也显示……它就只显示那一个」。
   */
  const copiesFollowCloud = useCopiesFollowCloud()
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
   * **右键「打开参数」那一份**（2026-10-09）。`syncFirst` = 直接把「选择要更新的参数」
   * 那一屏打开（用户点的是那条直路：他已经知道有云端更新等着处理 ——
   * 所以那条路进页面时就把云端新版取回来了，见 `PageParams` 里那个 effect）。
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
  /**
   * **编辑模式**（2026-10-16，作者：「点进去之后，它一开始只是显示……点了之后才出现」）：
   * 打开先只读（`<pre>` 画着色，textarea 不出现），底栏只有「进入编辑模式」；
   * 进了编辑模式才有 撤销 / 重做 / 放弃并退出 / 保存。
   * 与参数对比台、修改参数弹窗是**同一套框架**（只读 → 进编辑 → 作废/保存），
   * 两颗按钮的词取自 `EDIT_TEXT`（`presetTree.ts`）。
   */
  const [editMode, setEditMode] = useState(false)
  /**
   * **撤销 / 重做**（本地文本快照栈）。粒度 = 一次「打字停顿」（就是草稿落盘那一拍，
   * 700ms）——不按每击键分格：撤销回退的是"一段字"，不是一个个字母。
   * 与参数对比台的本地栈同一个路数（那边编辑的是一份、这里一份正文）。
   */
  const [undoStack, setUndoStack] = useState<string[]>([])
  const [redoStack, setRedoStack] = useState<string[]>([])
  /**
   * 这回文本变化是**撤销 / 重做自己造的**（true）还是用户敲的（false）。
   * 落盘那一拍据此决定要不要往撤销栈推快照 —— 自己造的变化再推一条，
   * 撤销就会来回跳（撤销本身成了可撤销的一步）。
   */
  const histJumpRef = useRef(false)
  /** 上一次真正落到临时文件里的正文。用它判断"值不值得再存一次" */
  const savedTextRef = useRef<string>('')
  /**
   * **这份文件的原文**（它最初落到盘上时的那一份）——「恢复默认值」那颗退回的地方。
   * 打开编辑器时预读一次（`readLocalOriginText`）；`null` = 没有原始备份（按钮禁用，
   * 不猜、不编一个"默认"出来）。见 Rust `runtime::baseline` 的原文镜像一段。
   */
  const [originText, setOriginText] = useState<string | null>(null)
  /**
   * **打开那一刻的两样快照** —— 关窗时用它判"有没有没保存的改动"
   * （作者 2026-10-16：「我如果没保存的时候关闭了模态框也需要有通知」）：
   * `openDirtyRef` = 打开时后端答的"草稿已经和盘上不一样"；`openTextRef` = 打开时的正文。
   * 关窗那一下：`openDirty || text !== openText` ⇒ 有没保存的改动，喊一声。
   * `exitEdit`（作废）会把 `openDirtyRef` 落回 `false` —— 改动都作废了，关窗不该再喊。
   */
  const openTextRef = useRef('')
  const openDirtyRef = useRef(false)
  /**
   * 通知上那颗「放弃修改」**此刻该走哪条路**：模态框还开着 ⇒ 底栏那颗「放弃并退出」；
   * 已经关了 ⇒ 只把草稿丢掉。每次渲染后刷新（见 `exitEdit` 后面那一个 effect 里的说明）。
   */
  const discardNowRef = useRef<() => void>(() => {})

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

  /**
   * **行号槽**（`2026-10-16` 作者：「有没有说给这些东西加上行号呀？就左侧出现行号」）。
   *
   * 它自己不滚（CSS 里 `overflow: hidden`）—— `scrollTop` 只由 `syncScroll` 喂，
   * 所以永远和正文同一格（正文那两层谁是滚动容器都不影响它）。
   *
   * 为什么值得加：预设行"用不了"的那句原因就是按行报的
   * （`state_detail`：「TOML 语法不对（第 3 行第 1 列）」）—— 没有行号，用户得自己数。
   */
  const numsRef = useRef<HTMLDivElement>(null)
  const areaRef = useRef<HTMLTextAreaElement>(null)
  /*
   * 行号文本（一行一个数）。跟着**行数**走，不跟着内容走 —— 正文里打字不该重算这一串。
   * ★ 不折行是前提：一个逻辑行 = 屏幕上一行，行号才一一对得上（见 CSS 那条 ★★）。
   *   末行是"空行"（正文以 `\n` 结尾）时它也占一个号，与着色层画出那一行一致。
   */
  const numsText = useMemo(
    () => paintedLines.map((_, i) => i + 1).join('\n'),
    [paintedLines],
  )

  /**
   * **软折行开不开**（作者 2026-10-16 第三轮：「有没有按那个 ALT Z 去换行啊，
   * 苹果上面的好像是 option Z 吧」）。
   *
   * 口径是那条 ★★ 立下的：默认**不折**（小窗口里长行折了续行不像注释，行号也对不上号）；
   * 想看得全就自己按一下。编辑器界的老规矩（VS Code / 记事本都是这个键）。
   *
   * 不落盘：它是"怎么看"的偏好，跟窗口宽度一样属于当下这一眼 —— 关掉重开回到默认
   * （不折行）。真要记住它，将来接 `useSessionState` 一句话的事。
   */
  const [softWrap, setSoftWrap] = useState(false)
  const applySoftWrap = (next: boolean) => {
    setSoftWrap(next)
    setNote({ text: next ? '软折行：开（长行折着看）' : '软折行：关（长行横着滚）', bad: false })
  }

  /**
   * **⌥Z 要在整个模态框里都灵** —— 这是 2026-10-16 作者报的那个"按了没反应"。
   *
   * 原来只挂在 `textarea` 的 `onKeyDown` 上 ⇒ **只读态**（没有 textarea）与
   * "焦点在底栏按钮 / 正文以外"的时候，按下去什么都不发生。作者看到的正是这个：
   * 打开模态框先落在只读态，那会儿 ⌥Z 本来就该能用（说明行就在那儿写着）。
   *
   * 挂在 `document` 上、**只在编辑文件这个模态框开着时**接（`editing === null` 直接返回）；
   * 认 `altKey`（苹果上就是 option）+ `z`，且不带 ⌘/Ctrl —— **⌘Z 是撤销，绝不抢**。
   * 只读态在模态框里选中文字按 ⌥Z 也照切（那是"看"的偏好，不是编辑动作）。
   */
  useEffect(() => {
    if (editing === null) return
    const onKey = (e: KeyboardEvent) => {
      /*
       * ★★ 判 `e.code === 'KeyZ'`，**不看 `e.key`** —— 苹果上按住 option 再敲 Z，
       *    `e.key` 会变成 `'Ω'`（option 是字符选择键：⌥Z 打出来的就是 Ω），
       *    拿 `e.key.toLowerCase() === 'z'` 去比永远不成立 —— 作者截图里那个 "Ω"
       *    就是这么来的（按对键了、判错了）。`e.code` 是**物理键位**，与修饰键无关，
       *    VSCode 这类编辑器认的也是它。
       */
      if (!e.altKey || e.metaKey || e.ctrlKey) return
      if (e.code !== 'KeyZ') return
      e.preventDefault()
      e.stopPropagation()
      applySoftWrap(!softWrap)
    }
    /* 捕获阶段接：抢在别的 keydown 处理之前（它只认一个组合键，不碍别人） */
    document.addEventListener('keydown', onKey, true)
    return () => document.removeEventListener('keydown', onKey, true)
  }, [editing, softWrap])

  /**
   * **滚动同步**：正文滚了，另外两层跟着 —— 着色层（画字的）与行号槽。
   *
   * 编辑态由 `textarea` 当滚动容器；只读态由那层 `<pre>` 自己当（`data-ro` 才 `overflow: auto`）。
   * 两条路都调这一个函数。
   *
   * ★★ 行号那层**只搬纵向**（作者 2026-10-16 截图：横向一滚，行号跟着飘、正压在正文底下
   *   那串数字/字上）。它本来就不该横着动 —— 行号要永远钉在左边当刻度。
   *   横向只搬给着色层（两层的字必须逐像素对上）。
   */
  const syncScroll = (el: HTMLTextAreaElement | HTMLPreElement) => {
    const nums = numsRef.current
    if (nums !== null) nums.scrollTop = el.scrollTop
    const paint = paintRef.current
    if (paint !== null) {
      paint.scrollTop = el.scrollTop
      paint.scrollLeft = el.scrollLeft
    }
  }

  /**
   * **触控板两指左右划**（2026-10-16 作者第三轮：「现在编辑模式下，双指触摸板呀，
   * 左右拖动还是不行」）。
   *
   * 为什么原生不管用：`overflow: hidden scroll` 只在**这一层自己**是滚动容器时才有横向滚，
   * 而这里 `<textarea>` 铺在上面、`<pre>` 铺在下面 —— **两次滚动都打不到该动的那一层**：
   *   · 打在上面那层 textarea 上：它是 `wrap="off"`，横向**内容**不溢出（它只滚纵向）；
   *   · 打在下面那层 pre 上：`pointer-events: none`（它不接事件）。
   * 于是"看起来能滚、实际上不动" —— 这正是作者看到的现象。
   *
   * 解法（开源编辑器都是这一手：Monaco / CodeMirror / VSCode 的 `ScrollableElement`
   * 都是**自己接管 wheel**）：在这一层上接 `wheel`，把**带横向分量**的那一份
   * 按滚动容器自己的方式搬过去。
   *   · `deltaX`（触控板两指横划、⇧滚轮、鼠标横向滚轮）→ `scrollLeft`；
   *   · 只有在**真横着滚**的时候才 `preventDefault`（纵向仍交给浏览器，
   *     不然整页的纵向手感会被破坏 —— 而纵向本来就是原生滚动，不用接）。
   * `scrollLeft` 一改就派发 `scroll` 事件，`onScroll` 那条链照旧把另外两层带上。
   */
  /*
   * 这一次 wheel 攒下来的横向距离。**不直接写 `scrollLeft`** —— 触控板一划会连发
   * 十几个 wheel（每个几像素），逐个同步写会让合成器/渲染器被顶在同一个像素帧里，
   * 手上就是"不太流畅"。攒到下一帧再一次性搬过去。
   */
  const wheelDxRef = useRef(0)
  const wheelRafRef = useRef(0)

  const onWheel = (e: React.WheelEvent<HTMLElement>) => {
    /*
     * ★★ **纵向一律放行**（作者 2026-10-16：「怎么感觉没有那个预览模式下流畅啊」）：
     *   这一层上接 `wheel` 之后，**每次事件都要走一遍 React 的合成层** ——
     *   纵向如果也在这儿被"过一手"（哪怕只是判一下再返回），滚起来就比原生那条路钝。
     *   第一版就是 `deltaX` 与 `⇧+纵向` 都接，纵向的顺滑跟着被拖累了。
     * ⇒ 这里只认**纯横向那一种**（触控板两指横划 / 鼠标横向滚轮）；
     *   `⇧ + 滚轮` 是浏览器自己把 `deltaY` 改写成 `deltaX` 的，不用我们换。
     */
    const dx = e.deltaX
    if (dx === 0) return
    /*
     * **一帧一次**（`requestAnimationFrame` 合并）：触控板划一下连发十几个 wheel，
     * 每个都直接写 `scrollLeft` 会让浏览器在每个事件里做一次样式/绘制 —— 手上就是"不跟手"。
     * 攒着，到下一帧统一搬（同 Monaco / CodeMirror 那条 `ScrollableElement` 的路子）。
     */
    wheelDxRef.current += dx
    if (wheelRafRef.current !== 0) return
    wheelRafRef.current = window.requestAnimationFrame(() => {
      wheelRafRef.current = 0
      const d = wheelDxRef.current
      wheelDxRef.current = 0
      const cur = editMode ? areaRef.current : paintRef.current
      if (cur === null || d === 0) return
      const limit = cur.scrollWidth - cur.clientWidth
      cur.scrollLeft = Math.max(0, Math.min(limit, cur.scrollLeft + d))
      syncScroll(cur)
    })
  }

  /**
   * **横向滚动条多高，正文就得在底下让出多高** —— 2026-10-16 作者问「别人不是用触摸板的，
   * 是怎么左右滑动的？」，最终形态是**让系统画它自己那条**（与 VSCode 同一个长相：
   * macOS 上是覆盖式的，不滚看不见、一滚浮出来、零占位 —— 见 CSS 里那一段）。
   *
   * 但**别的平台**（Windows / Linux 的 WebView）那条是**占位**的：横向槽在框内，
   * 会盖住最后一行的尾巴。所以这里量一下 `offsetHeight - clientHeight`：
   *   · 覆盖式（macOS，含 Tauri 的 WKWebView）= 0 ⇒ `--scrollbar-pad` 为 0，什么都不变；
   *   · 占位式 = 十几像素 ⇒ 补到正文那两层的 `padding-bottom` 上（**只动内衬**，
   *     行高没被碰，行号那一列照旧与正文同一格）。
   *
   * `ResizeObserver` 认的是元素盒子的变化 —— 滚动条出现 / 消失会改 `clientHeight`，
   * 这条回调就会再走一遍（算出同一个值就不 setState，不会打转）。
   */
  const [scrollbarPad, setScrollbarPad] = useState(0)
  useEffect(() => {
    const el = editMode ? areaRef.current : paintRef.current
    if (el === null) return
    const read = () => {
      const h = el.offsetHeight - el.clientHeight
      setScrollbarPad((cur) => (cur === h ? cur : h))
    }
    read()
    const ro = new ResizeObserver(read)
    ro.observe(el)
    return () => ro.disconnect()
  }, [editMode, editing])

  /*
   * **文件头那一段在正文里的长度**（字符数）—— 它锁着，不许手改（见 `HEAD_LOCKED`）。
   * 分界规则在 `tomlPaint.ts` 的 `splitHeader`（开头连续的注释 / 空行，到第一个 `[表]` 为止）。
   *
   * ★ 跟着**当前文本**算，不记"打开时那一份的头"：`恢复默认值` 会把正文整份换成最初那份
   *   （头可能不一样），锁要跟着换过去。
   */
  const headLen = useMemo(
    () => (editing === null ? 0 : splitHeader(editing.text).head.length),
    [editing],
  )

  /**
   * **这次动作会不会动到文件头**（`from` = 这次要改写的区间在正文里的起点）——
   * 会就拦下并说一句（`HEAD_LOCKED`）。
   *
   * 判的是**将要被改写的区间**而不是光标位置：`Backspace` 没有选区时删的是光标
   * **左边**那一格，所以它的起点要减一（`Delete` 删的是右边那一格）。不改内容的键
   * （方向键 / Home / End / ⌘A / ⌘C / ⌘Z）在调用处就被筛掉了，压根不走这儿。
   */
  const headLockedAt = (e: { preventDefault: () => void }, from: number): void => {
    if (headLen === 0 || from >= headLen) return
    e.preventDefault()
    notifyHeadLocked()
  }

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
   * **把云端这一版落到「behind 这一份本地文件」上**（两条入口共用的一段）。
   *
   * 云端行那颗「更新」与本地行那颗「更新」都走这里 —— 区别只在"更新谁"由调用方定。
   *
   * 三步：
   *   ① 读逐参数三方账（`presetSync`，允许后端顺手把云端新版字节取回来）；
   *   ② **没有要改值的项** ⇒ 直接「应用 0 项」：值一个不动，云端的注释 / 新字段 /
   *      时间戳与血统跟着新版走 —— 「有新版」就此消失（原来这一步什么都不写，
   *      于是注释级换版永远挂着「有新版」）；
   *   ③ **有要改值的项** ⇒ 顺势打开「修改参数 → 选择要更新的参数」那一屏，
   *      由用户勾（采用云端新值）/ 不勾（保持我的值）后一次落下去。
   *
   * `behind` = 要更新的那一份本地文件；`row` = 发起它的那一行（忙的那一颗按钮认它）；
   * `fileName` = 它对应的云端文件名（文案用）。
   *
   * ★ 定义写在两个调用方**前面**（`const` 有暂时性死区：写在后面调用方拿不到）。
   */
  const applyCloudUpdate = async (
    behind: LocalPresetFile,
    row: PresetTableRow,
    fileName: string,
  ): Promise<void> => {
    setBusyKey(row.rowKey)
    /*
     * ② 读逐参数三方账。`fetchMissing: true` 允许后端**顺手把云端新版字节取回来**
     * （内部基准，**不动本地这份文件**）—— 所以这里不需要先「下载」一遍。
     *
     * ★ 这一处是"全应用为看差异发网络"的**两个入口之一**（另一个是参数页那颗按钮），
     * 因为用户到这一步已经点了「更新」（作者 2026-10-20：不点就别替他去取）。
     */
    const sync = await data.presetSync(behind.path, true)
    setBusyKey(null)
    /*
     * `null` = 这一份不认云端（复制出来的那一份）。账本已经把它滤掉了，
     * 正常走不到这里 —— 万一走到（数据刚被别处改过），也**不许当成"零项待处理"**，
     * 那样会让人以为云端这一版没动过任何一项。
     */
    if (sync === null) {
      setNote({
        text: `更新 ${fileName} 没成：你机器上没有跟这个云端条目对得上的那一份`,
        bad: true,
      })
      return
    }
    if (!sync.cloudReady) {
      setNote({
        text: `更新 ${fileName} 没成：云端新版的字节还没取回来（网络或数据源的问题）`,
        bad: true,
      })
      return
    }
    const pending = sync.entries.filter((e) => e.pending)
    /*
     * ③a **没有要改值的项** ⇒ 直接「应用 0 项」（= 把这一版收进来）：
     *    值一个不动，云端的注释 / 新字段 / 时间戳与血统跟着新版走 ——
     *    「有新版」就此消失（原来这一步什么都不写，于是它永远挂着）。
     */
    if (pending.length === 0) {
      await data.applyParamDecisions(behind.path, [])
      setNote({ text: `已更新 ${behind.fileName} —— 已经是云端最新版了`, bad: false })
      return
    }
    /*
     * ③b **有要改值的项** ⇒ 顺势打开「修改参数」并把「选择要更新的参数」那一屏带开
     *    （`syncFirst`）：那一屏里逐项勾，勾上的采用、没勾的保持 —— 用户看着定，
     *    不再替他静默采用任何一项。
     *
     * ★ 2026-10-20：那一屏的「应用」**只写进草稿**（一次撤销能整批退回来），
     *   真正落盘要用户在那张参数页上点「保存」—— 作者：「应该保存的时候才算真的用，
     *   跟编辑一样」。所以这里放人进去就够了，不再有"点一下当场改写文件"的路径。
     */
    const machineId = behind.machineId ?? behind.basedOnMachineId
    const versionId = behind.versionId ?? behind.basedOnVersionId
    if (machineId === null || versionId === null) {
      setNote({
        text: `更新 ${behind.fileName} 没成：认不出这一份属于哪台机型的哪一版`,
        bad: true,
      })
      return
    }
    setParamModal({
      target: { path: behind.path, fileName: behind.fileName, machineId, versionId },
      syncFirst: true,
    })
    setNote({ text: `${behind.fileName} 有 ${pending.length} 项云端改过的参数等你挑`, bad: false })
  }

  /**
   * **本地行那颗「更新」**（2026-10-20 作者：「这个下面应该加一个更新按钮，就是说有新的时候
   * 就出现一个更新按钮。我点了之后，它就跟云端一样更新了，就不用跳转到云端那边去更新」）。
   *
   * 与云端行那颗走**同一段落地逻辑**（[`applyCloudUpdate`]）—— 区别只在"更新谁"由这一行
   * 自己说了算：就是**这一份**（不再按云端文件名去账本里找"第一份落后的"）。
   * 于是同一云端条目下有好几份时，从哪个行点就更新哪个行。
   */
  const runUpdateLocal = (row: PresetLocalRow) => {
    if (row.basedOn !== 'outdated') return
    const local = data.local.find((f) => f.path === row.path)
    if (local === undefined) {
      setNote({ text: `${row.fileName} 已经不在本地了 —— 列表刚变过，再看一眼`, bad: true })
      return
    }
    const cloudName = row.basedOnCloud ?? row.fileName
    setNote({ text: `正在更新 ${row.fileName}…`, bad: false })
    void applyCloudUpdate(local, row, cloudName).catch((e: unknown) => {
      setBusyKey(null)
      setNote({ text: `更新失败：${errorText(e)}`, bad: true })
    })
  }

  /**
   * **云端行的「更新」**（2026-10-10 作者规格；2026-10-20 改过一遍走法）。
   *
   * 「更新谁」：**血统指向这个云端条目、还没跟上当前版的那一份本地文件**
   * （同一云端条目下我可能有好几份 —— 取列表里第一份落后的；复制件不在账本里，
   * 永远不会被当成"要更新的那一份"）。找到之后交给 [`applyCloudUpdate`] 落地。
   *
   * ★ 定义写在 [`applyCloudUpdate`] **后面**：`const` 有暂时性死区，反过来拿不到。
   */
  const runUpdate = (row: PresetTableRow) => {
    if (row.scope !== 'cloud') return
    const fileName = row.fileName
    void (async () => {
      setBusyKey(row.rowKey)
      setNote({ text: `正在更新 ${fileName}…`, bad: false })
      /*
       * ① 更新的是**本地那一份**：它跟这个云端条目的关系**只看身份账本**
       * （`cloudSource` 对上它），而且还没跟上当前版（`basedOn !== 'current'`）。
       * **先读一遍本地列表**：更新对象要在动手之前定下来。
       */
      const [localPresets, index] = await Promise.all([
        api.getLocalPresetFiles(),
        api.getLocalPresetIndex(),
      ])
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
      await applyCloudUpdate(behind, row, fileName)
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
        /*
         * 打开先只读（2026-10-16）——**除非**上一次的改动还没保存、也没作废掉
         * 就被关掉了（后端答 `reused && dirty`：草稿还在、且与盘上字节不一样）。
         * 作者 2026-10-16：「下次打开这个模态框，它里面还是显示编辑模式……
         * 好的提示、或者通知，就是说刚才那些修改还没保存、没生效」。
         */
        const unsaved = draft.reused && draft.dirty
        setEditMode(unsaved)
        /* 关窗时的"有没有没保存的改动"以这一刻为基准（见两个 ref 的注释） */
        openTextRef.current = draft.text
        openDirtyRef.current = unsaved
        /*
         * ★ 撤销 / 重做历史**按 path 记着**（作者 2026-10-16：「我这次再次进来，有没有那种
         * 可以做到还保留撤销重做的呀」）——关掉模态框再进来，栈还接着用。
         * 有效性只认一条：缓存里记的"上次离开时的正文"（`head`）与这次读到的草稿正文
         * 一模一样才认这份历史（正文被别处动过 ⇒ 旧快照对不上手，干脆丢掉重来）。
         */
        const kept = textHistCache.get(draft.path)
        if (kept !== undefined && kept.head === draft.text) {
          setUndoStack(kept.undo)
          setRedoStack(kept.redo)
        } else {
          if (kept !== undefined) textHistCache.delete(draft.path)
          setUndoStack([])
          setRedoStack([])
        }
        histJumpRef.current = false
        setEditing({
          fileName: draft.fileName,
          path: draft.path,
          text: draft.text,
          draftError: null,
        })
        /* 「恢复默认值」的原文预读（读到什么算什么；读不到 = 按钮禁用） */
        data.readLocalOriginText(draft.path).then(
          (text) => setOriginText(text),
          () => setOriginText(null),
        )
        if (unsaved) {
          /*
           * 提醒归提醒，改动本身一个字不动 —— 就挂在草稿里等他回来保存。
           * 后半句是 2026-10-16 作者要的"那一个提示"：**底栏那颗 = 作废**
           * （用户不知道这件事，于是不敢关、也不敢点）。通知自己那颗「放弃修改」
           * 就是这件事的另一个入口。
           */
          notifyUnsaved(
            '上次那些改动还没保存、还没生效 —— 要生效得先保存（底栏「放弃并退出」= 作废这次改动）',
            () => discardNowRef.current(),
          )
        }
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
   *
   * ★ 撤销栈的快照也挂在这一拍上（2026-10-16）：落盘时把**改动前的那份文本**
   * 推一格 —— 一次停顿 = 一格，撤销回退的是"一段字"而不是一个个字母。
   * 撤销 / 重做自己造的落盘（`histJumpRef`）**不推快照**，否则撤销本身会被记成一步。
   */
  useEffect(() => {
    if (editing === null || editing.text === savedTextRef.current) return
    const text = editing.text
    const before = savedTextRef.current
    const jump = histJumpRef.current
    const timer = window.setTimeout(() => {
      data.putTextDraft(text).then(
        () => {
          savedTextRef.current = text
          histJumpRef.current = false
          if (!jump) {
            setUndoStack((st) => [...st, before])
            setRedoStack([])
          }
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
   * 编辑中随时把两份栈镜像进模块级缓存（见 `textHistCache`）——关掉模态框那一下
   * 手上有"最后一眼"的完整状态；重开时按 `head` 认它。（这个 effect 只写模块级 Map，
   * 不 set state —— 没有回环。）
   */
  useEffect(() => {
    if (editing === null) return
    textHistCache.set(editing.path, { undo: undoStack, redo: redoStack, head: editing.text })
  }, [editing, undoStack, redoStack])

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
        /* 重开 = 换了一版内容，旧的撤销栈对不上手（一步都不能撤了） */
        setUndoStack([])
        setRedoStack([])
        setEditing((cur) => (cur === null ? cur : { ...cur, text: draft.text, draftError: null }))
        setNote({ text: '这份刚在「修改参数」里保存过 —— 编辑区已按最新内容重开', bad: false })
      },
      (e: unknown) => setNote({ text: `重开不了：${errorText(e)}`, bad: true }),
    )
  }, [data, editing, textDropRev])

  /*
   * **撤销 / 重做**（编辑模式的底栏，2026-10-16）。栈里存的是**整份文本快照**：
   * 一份预设几十 KB，几十格快照不痛不痒。两条都置 `histJumpRef` ——
   * 这次文本变化是"我们自己造的"，下一拍落盘不许再推快照。
   */
  const undoEdit = () => {
    if (editing === null) return
    const prev = undoStack[undoStack.length - 1]
    if (prev === undefined) return
    setUndoStack(undoStack.slice(0, -1))
    setRedoStack([...redoStack, editing.text])
    histJumpRef.current = true
    setEditing({ ...editing, text: prev })
  }

  const redoEdit = () => {
    if (editing === null) return
    const next = redoStack[redoStack.length - 1]
    if (next === undefined) return
    setRedoStack(redoStack.slice(0, -1))
    setUndoStack([...undoStack, editing.text])
    histJumpRef.current = true
    setEditing({ ...editing, text: next })
  }

  /**
   * 保存：**写回它自己**（第八层：同一个文件，不会多出一份）。
   *
   * 写的是**整份替换**（后端先写临时文件再换上去 —— 原子写入），
   * 而且只有这一下才碰盘：没点保存时，改到哪都只在正文草稿里。
   */
  const commitEdit = () => {
    if (editing === null) return
    /*
     * ★ 2026-10-16：先把手上的正文**补落一次草稿**再 commit ——
     * 边改边存是 700ms 防抖，"改完立刻点保存"时最后一笔可能还在路上；
     * 直接 commit 存的就是**上一个版本**（探针里踩到过：fill 完 600ms 就按保存，
     * 保存下来的内容少一行）。putTextDraft 本身幂等，先补这一次没有副作用。
     */
    const text = editing.text
    data
      .putTextDraft(text)
      .then(() => data.commitTextDraft())
      .then(
        (done) => {
          savedTextRef.current = text
          /* 保存 = 落定，一段新的开始：撤销历史（连模块级那份缓存）跟着清 */
          textHistCache.delete(editing.path)
          setUndoStack([])
          setRedoStack([])
          setEditing(null)
          /* 那一条"还没保存"的通知跟着过期（它已经是保存过的了） */
          clearUnsavedNotice()
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
   * **关窗**（Esc / ×；保存走 `commitEdit`，那条路不算"关"）——**只是关，一个字不丢**，
   * 但**没保存的改动要说一声**（作者 2026-10-16：「我如果没保存的时候关闭了模态框也需要
   * 有通知」）：改动留在草稿里、还没生效，下次打开会直接进编辑模式并再提醒一次
   * （两条通知：一条说"刚才"、一条说"上次"，同一件事的两个时刻）。
   */
  const closeEdit = () => {
    if (editing === null) return
    const unsaved = openDirtyRef.current || editing.text !== openTextRef.current
    setEditing(null)
    if (unsaved) {
      /* 这一条不用教底栏那颗（按钮已经不在屏幕上了）—— 那颗「放弃修改」就在卡上 */
      notifyUnsaved('刚才那些改动还没保存、还没生效 —— 要生效得先保存', () =>
        discardNowRef.current(),
      )
    }
  }

  /**
   * **底栏那颗（「放弃并退出」）= 作废**（作者 2026-10-16：「没有保存的话，退出编辑模式
   * 就是作废，那些改的都作废，保存了才有」）。
   *
   * 丢正文草稿 → 重读盘上字节回填 → 回只读。撤销栈跟着清：内容换了一版，
   * 旧快照对不上手（与"这份刚在修改参数里保存过"那一手同一条口径）。
   * 要留住改动只有一条路 —— **保存**。
   *
   * ★ 2026-10-16：通知上那颗「放弃修改」在**模态框开着**时走的就是这里
   * （作者要的正是这个 —— 用户不知道"退出编辑模式"就是作废，于是把它写进了按钮词）。
   */
  const exitEdit = () => {
    if (editing === null) return
    const there = editing
    /*
     * 先把 `savedTextRef` 顶到当前文本：挡掉"还有一笔没落盘"的判定 ——
     * 否则退出这一下若正逢 700ms 防抖的窗口，pending 的 put 可能与 discard 乱序到，
     * 草稿被写回、作废就白做了（下面的 `setEditing` 也会让那个 effect 自己清掉 timer）。
     */
    savedTextRef.current = there.text
    data
      .discardTextDraft()
      .then(() => data.beginTextEdit(there.fileName, there.path))
      .then(
        (fresh) => {
          savedTextRef.current = fresh.text
          /* 改动都作废了 —— 关窗时不该再喊"还没保存"（基准换成刚读回来的这份） */
          openTextRef.current = fresh.text
          openDirtyRef.current = false
          setUndoStack([])
          setRedoStack([])
          histJumpRef.current = false
          setEditing((cur) => (cur === null ? cur : { ...cur, text: fresh.text, draftError: null }))
          setEditMode(false)
          /* 那一条"还没保存"的通知跟着过期（它说的那件事已经作废了） */
          clearUnsavedNotice()
        },
        (e: unknown) => setNote({ text: `退回没成：${errorText(e)}`, bad: true }),
      )
  }

  /**
   * 通知上那颗**放弃修改**按下去走哪条路 —— 每次渲染后刷新。
   *
   * 为什么不能直接把它写进 `notifyUnsaved(...)` 的回调里：通知的动作闭包**定格在发出
   * 那一帧**（关窗那条是在 `editing` 已经变成 null 之后发的、重开那条是在 `setEditing`
   * 之后发的），抓状态会抓到旧值。所以只留一个"此刻该走的那条"在这里，动作只认它。
   *
   * 两条路：
   *   模态框还开着   ⇒ 就是底栏那颗「放弃并退出」（作废 + 回只读 + 按盘上字节重读）；
   *   已经关了       ⇒ 只把草稿丢掉 —— 盘上那份文件全程没被碰过，丢掉就是"回到保存过的样子"。
   */
  useEffect(() => {
    discardNowRef.current =
      editing === null
        ? () => {
            void data.discardTextDraft().then(
              () => {
                clearUnsavedNotice()
                setNote({ text: '已放弃上次那些改动 —— 那份文件还是保存过的样子', bad: false })
              },
              (e: unknown) => setNote({ text: `放弃没成：${errorText(e)}`, bad: true }),
            )
          }
        : exitEdit
  })

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
   * **恢复默认名**（2026-10-20 作者：「我希望他的这个名字可以恢复默认的名字」）。
   *
   * 默认名认的是**这一份当初从哪来**（[`defaultNameOf`]）：复制件 ⇒ 来源那一份现在的名字，
   * 从云端下来的 ⇒ 云端那份的名字。撞名时按 `A1-standard 2.toml` 依次让开，不报错
   * （用户要的是"改回正常名字"这件事本身）。
   *
   * 走的是**与重命名同一条路**（`data.rename`）：只换名字，内容与血统一个字节不动。
   */
  const runRestoreDefaultName = (row: PresetTableRow) => {
    menu.close()
    const want = defaultNameOf(row)
    if (want === null) {
      setNote({ text: `${row.fileName} 没有默认名可恢复（它本来就不叫这个名字）`, bad: true })
      return
    }
    const name = defaultNameAvoiding(
      want,
      data.local.filter((f) => f.path !== row.path).map((f) => f.fileName),
    )
    if (name === row.fileName) {
      setNote({ text: `${row.fileName} 已经是默认名了`, bad: false })
      return
    }
    setNote({ text: `正在恢复默认名：${row.fileName} → ${name}…`, bad: false })
    data.rename(row.path, name).then(
      (done) =>
        setNote({
          text: done.fileName === want
            ? `已恢复默认名：${row.fileName} → ${done.fileName}`
            : `已恢复默认名：${row.fileName} → ${done.fileName}（默认名已被占用，让开了编号）`,
          bad: false,
        }),
      (e: unknown) => setNote({ text: `没改成：${errorText(e)}`, bad: true }),
    )
  }

  /**
   * 重命名框里那颗「恢复默认名」：**不是直接改名** —— 它只把输入框填成默认名，
   * 用户还能看着改两笔再按「改名」。和菜单里那一项共用同一个默认名判据。
   */
  const fillDefaultName = () => {
    const row = renaming?.row
    if (row === undefined) return
    const want = defaultNameOf(row)
    if (want === null) {
      setRenaming((cur) => cur === null ? cur : { ...cur, error: '这一份没有默认名可恢复' })
      return
    }
    /* 后缀跟着默认名走（复制件的来源后缀不一定是 `.toml`，但这一档都是） */
    const { stem, ext } = splitFileName(want)
    setRenaming((cur) => (cur === null ? cur : { ...cur, name: stem, ext, error: null }))
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
          /*
           * 名字有上限（255 字节）：原名本来就长的时候 `<原名> copy.toml` 会超 ——
           ★ 也不让复制失败（作者 2026-10-20 那条裁定的另一半），截到上限并照实说一句。
           */
          text: done.adjusted
            ? `已复制出一份：${done.fileName}（名字到了上限，截短了一点）—— 原文件一个字节没动`
            : `已复制出一份：${done.fileName} —— 原文件一个字节没动`,
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
          /*
           * ★ 名字不合规**不再失败**（作者 2026-10-20：「改名有限制就显示出来……
           * 不要失败，可以自动截断」）：超长截到上限、不合规的字符剔掉，
           * 落盘的就是回执里那一串 —— 这里照实说"被截短了"，不说就等于骗他。
           */
          text: done.adjusted
            ? `已改名：${renaming.row.fileName} → ${done.fileName}（名字超过了上限，已截短）`
            : `已改名：${renaming.row.fileName} → ${done.fileName} —— 只换了名字，内容与血统一个字节没动`,
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
     *   （「选择要更新的参数」（当时叫「选择要跟随的云端更新」）2026-10-10 从菜单里去掉
     *     —— 参数表里照旧能处理更新）
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
       * **「复制后处理脚本」**：抄的是**这一行**那一份（作者排它第二）。
       *
       * # 寻址：点名这一行的落点（`row.path`），不再按 (机型, 版本) 找
       *
       * 2026-10-19 作者实测：右键 `A1-fast.toml`，抄到的却是"正在使用"的
       * `A1-fastv3.3 copy.toml` —— 因为从前只把 `(ownMachineId, ownVersionId)` 交给后端，
       * 后端按这组值在 `presets-local/` 里挑"我那一份"（优先挑底账里**正在使用**的那份）。
       * 同一版本下有几份副本时，这就必然可能落到别人身上。
       * ⇒ 现在把 `row.path`（相对用户根）一起交过去：后端只认这一份，**不再猜**。
       *
       * # 不再需要"归属"
       *
       * 命令的 `--Toml` 就是这一份的绝对落点，所以"文件头有没有 # machine:"不再决定
       * 能不能抄（从前缺归属就灰着 —— 那是按组合寻址时的限制）。
       * 仍然灰着的是**用不了的那几种**：认不出是 MKP 预设的 `.json`、读不出来的。
       * ★ 行上那个 `machineId` 是**筛选档**（这一行挂在哪个机型分栏下），与归属是两回事。
       */
      const scriptWhyNot =
        row.kind !== 'mkp_preset'
          ? LOCAL_NOT_PRESET_WHY
          : row.localState === 'unreadable'
            ? localUnreadableWhy(row.localStateDetail)
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
            /* ★ 关键：点名这一行（相对用户根）—— 后端据此只指这一份 */
            path: row.path,
            fileName: row.fileName,
          }),
      }

      /*
       * # 这一档的**顺序**（2026-10-19 作者定：「三段 + 两条分隔线」）
       *
       *   ① 用它    使用 / 取消使用 · 复制后处理脚本
       *   ② 改它    修改参数 · 编辑文件 · 参数对比 · 复制 · 重命名 · 显示所在文件夹 · 置顶
       *   ③ 删它    删除
       *
       * 作者原话：「首先使用排在第一个……后处理脚本放第二个，修改参数放第三个」，
       * 另有「其他的那一些没图标的就往后放」；看过三种排法后挑了这一版（分段 + 基本全图标）。
       *
       * ★ **图标**：除「编辑文件」「参数对比」外都挂（作者 2026-10-19：
       *   「有一些就是不应该加图标，就比如那个编辑文件参数对比就不需要」）——
       *   这两项都是"进另一屏去看 / 去改"的入口，一条文字已经说清了，挂图标反而抢眼睛。
       * ★ 两件都叫"复制"，**字形必须分得开**：「复制后处理脚本」= `copy`（两份纸）；
       *   「复制」（复制出一份文件）= `fileCopy`（一份文件 + 加号）。别把它们画成一个样。
       * ★ 「编辑文件」与「参数对比」的条件**完全相同**（本地 + MKP 预设 + 读得出），
       *   所以它们**同进同出**，第二段里不会出现"只剩一个"的空位。
       */
      return [
        useEntry,
        scriptEntry,
        /*
         * 第一条线：上面两项是"把这一份拿去用"，下面全是"改这一份 / 整理这张表"。
         * ★ 云端表与切片器档不加这条线（那两张表本来就短，线只会把菜单切碎）。
         */
        { separator: true },
        /*
         * 参数表那一屏（2026-10-09）：改的是参数本身。
         * ★ 「选择要更新的参数」（当时叫「选择要跟随的云端更新」）2026-10-10 按作者裁决
         *   **从菜单里去掉**了（原入口是 `openParams(row, true)` 那条直路）——
         *   参数表里照旧能处理云端更新，菜单只留一个「打开参数」。
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
          label: PARAMS_TEXT.open,
          disabled: paramsWhyNot(row),
          onSelect: () => openParams(row, false),
        },
        /*
         * 「编辑文件」：改的是**正文**。**不挂图标**（作者 2026-10-19 点名）——
         * 与下面「参数对比」同为"进另一屏"的入口，就让它俩并排空着。
         * `satisfies` 不能省：条件表达式的两个分支拿不到外层的上下文类型。
         */
        ...(row.origin === 'local' && row.kind === 'mkp_preset' && row.localState !== 'unreadable'
          ? ([{ id: 'edit', label: EDIT_TEXT.cell, onSelect: () => openEdit(row) }] satisfies ContextMenuEntry[])
          : []),
        /*
         * **「更新」**（2026-10-20）：与行上那颗、详情「快捷操作」那颗同一个动作 ——
         * 只在**有新版**时出现，点了就地更新（不切到云端表）。
         * 复制件跟云端无关（设置页那一格关着时）就不给这一项。
         */
        ...(row.basedOn === 'outdated' && !isDetachedCopy(row, copiesFollowCloud)
          ? ([
              {
                id: 'update',
                icon: 'download',
                label: UPDATE_ACTION_TEXT.update,
                onSelect: () => runUpdateLocal(row),
              },
            ] satisfies ContextMenuEntry[])
          : []),
        /* 参数对比的右键入口（2026-10-10）：两下选两份，见 `compareEntry`。**不挂图标** */
        ...(canCompare(row) ? [compareEntry(row)] : []),
        {
          id: 'copy',
          /* 「复制」（2026-10-10 作者：文案太长 → 短词，点了就自动出一份）。
             ★ 2026-10-19 起挂 `fileCopy`（一份文件 + 加号）—— 与上面「复制后处理脚本」
             的 `copy`（两份纸）分开：两行都叫"复制"，字形要一眼认得出来 */
          icon: 'fileCopy',
          label: LOCAL_COPY.cell,
          disabled: copyWhyNot(row),
          onSelect: () => runDuplicate(row),
        },
        {
          id: 'rename',
          /* 2026-10-19 挂 `pencil`（这枚 2026-10-16 撤过一次 —— 那时这一项不挂图标） */
          icon: 'pencil',
          label: '重命名',
          disabled: renameWhyNot(row),
          onSelect: () => openRename(row),
        },
        /*
         * **恢复默认名**（2026-10-20 作者：「我希望他的这个名字可以恢复默认的名字」）。
         * 只在**真有一个默认名、而且现在不叫它**的时候出现 —— 没来路的那份（导入的裸文件）
         * 这一项不摆出来（摆了点了只能报一句"没有默认名"，那是废话）。
         */
        ...(() => {
          const want = defaultNameOf(row)
          if (want === null || want === row.fileName) return []
          const entry: ContextMenuEntry = {
            id: 'restoreName',
            icon: 'pencil',
            label: RESTORE_DEFAULT_NAME,
            disabled: renameWhyNot(row),
            onSelect: () => runRestoreDefaultName(row),
          }
          return [entry]
        })(),
        {
          id: 'reveal',
          icon: 'folder',
          label: REVEAL_LABEL,
          disabled: revealWhyNot(row),
          onSelect: () => runReveal(row),
        },
        /*
         * **「置顶」的图标加回来了**（2026-10-16 作者：「置顶的还是加上图标吧」）——
         * 上一轮为了跟「在访达中显示」「删除」错开把它摘了，作者要回来。
         * 它俩同属"位置与整理"，排在第二段末尾。
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
                /*
                 * 展开详情「快捷操作」那颗「修改参数」（2026-10-20）——
                 * 与右键菜单里那一项同一个动作、同一个词（`PARAMS_TEXT`）。
                 */
                onParams={(tableRow) => openParams(tableRow, false)}
                usage={usage}
                onLive={runLive}
                /* MKP 本地行唯一的按钮（2026-10-09）：这一档只有你自己那份本地文件 */
                onUse={runUse}
                onDownload={download}
                /*
                 * 本地行那颗「更新」（2026-10-20）：「有新版」时出现 —— 点了就地更新，
                 * 不用切到云端表（作者：「就不用跳转到云端那边去更新」）。
                 */
                onUpdateLocal={(localRow) => {
                  void runUpdateLocal(localRow)
                }}
                /* 来源格「复制自 X」的定位落点；sourceLabel 决定交付预设行来源列显示 GitHub / Gitee */
                onLocate={locateRow}
                sourceLabel={data.sourceLabel}
                /* 备注（副标题覆盖账）与归属（复制出来的那份标机型 / 版本） */
                machines={data.machines}
                remarks={data.remarks}
                onSetRemark={runSetRemark}
                onSetAttribution={runSetAttribution}
                /*
                 * 设置页那一格「复制出来的也跟着云端更新」（2026-10-20，出厂关）：
                 * 关着时复制 / 导入出来的那几份**不挂「有新版」** —— 它们跟云端没关系。
                 * 这一格由 `usePresetPrefs` 读（后端唯一出口），表那边只照着画。
                 */
                copiesFollow={copiesFollowCloud}
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
           * **编辑器**（「编辑文件」那条链）。初稿 = **盘上那份文件的字节**，
           * 编辑期间只动正文草稿 —— 所以这里没有"保存到云端"这种动作：
           * 只有「放弃并退出」与「保存（写回我这份）」。参数页那份未保存的改动**不进来**
           * （2026-10-10 作者：「没有点保存的时候千万不要保存进来」，两条草稿分开存）。
           *
           * ★ 2026-10-16 两态（作者：「点进去之后，它一开始只是显示……」）：
           *   打开 = **只读**（着色正文 + 右下「进入编辑模式」），点进编辑模式才有
           *   撤销 / 重做 / 放弃并退出 / 保存。原来那颗「放弃」退休 ——
           *   那一颗**什么都不丢**：草稿在盘上，下次进来接着改。
           *
           * 2026-10-08 资源库改判起，编辑的对象只有**「我的预设」**（交付预设那一份是模板），
           * 所以这里只有一套说法：保存 = 写回它自己。
           * 关掉（Esc / ×）= **只关，不丢**（同上：草稿在盘上）。
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
             * 底栏（2026-10-16 两态，与参数对比 / 修改参数弹窗同一套框架）：
             *
             *   只读态   右下「进入编辑模式」（打开先只是显示 —— 图二那颗的位置）
             *   编辑态   撤销 / 重做 / 放弃并退出 / 保存
             *
             * 左边仍只在**出错**时说话 —— 那两句过程说明（"改动自动存草稿"等）
             * 2026-10-10 第三轮就撤了（"草稿""盘上"是内部的词，用户不需要被通知）；
             * 留下的只有**异常**：临时文件没写进去时必须看得见。
             */
            footer={
              editing !== null && (
                <div className={s.editorFoot}>
                  {/*
                   * **恢复默认值**（2026-10-16 作者：「就是左侧那个修改参数里面有按钮是恢复
                   * 默认值，那这个也可以有个按钮编辑文件的恢复默认值」）——与参数弹窗那颗
                   * **同一个位置**（底栏左下）、同一个词。退的是这份文件**最初落到盘上时**
                   * 那一份（`originText`，来自 `baseline-local/` 的原文镜像）。
                   * 算**一次改动**：按「撤销」能整个退回来；要落地还得按「保存」。
                   */}
                  {editMode && (
                    <button
                      type="button"
                      className={`${s.textBtn} ${s.editorRestore}`}
                      disabled={originText === null}
                      title={
                        originText === null
                          ? '这份还没有原始备份 —— 退回不了最初那份'
                          : '把正文退回它最初落到你盘上时的那一份 —— 算一次改动，按「撤销」能整个退回来'
                      }
                      onClick={() => {
                        if (originText === null) return
                        setEditing((cur) => (cur === null ? cur : { ...cur, text: originText }))
                        setNote({ text: '已恢复成最初那份 —— 这一步按「撤销」能退回来', bad: false })
                      }}
                    >
                      恢复默认值
                    </button>
                  )}
                  {editing.draftError !== null && (
                    <span className={s.editorStateBad}>
                      {EDIT_TEXT.draftFailed}
                      {editing.draftError}
                    </span>
                  )}
                  <span className={s.editorActs}>
                    {editMode ? (
                      <>
                        <button
                          type="button"
                          className={s.iconBtn}
                          disabled={undoStack.length === 0}
                          title="撤销（一次打字停顿回退一格）"
                          aria-label="撤销"
                          onClick={undoEdit}
                        >
                          <Icon name="undo" size={15} />
                        </button>
                        <button
                          type="button"
                          className={s.iconBtn}
                          disabled={redoStack.length === 0}
                          title="重做"
                          aria-label="重做"
                          onClick={redoEdit}
                        >
                          <Icon name="redo" size={15} />
                        </button>
                        <button
                          type="button"
                          className={s.textBtn}
                          /* 词与说明都取自 `EDIT_TEXT`（三个模态框同一句话，见那一份的长注释） */
                          title={EDIT_TEXT.exitTitle}
                          onClick={exitEdit}
                        >
                          {EDIT_TEXT.exitMode}
                        </button>
                        <button type="button" className={s.primary} onClick={commitEdit}>
                          {EDIT_TEXT.commit}
                        </button>
                      </>
                    ) : (
                      <button
                        type="button"
                        className={s.primary}
                        title="进编辑模式：正文就可以改了"
                        onClick={() => setEditMode(true)}
                      >
                        {EDIT_TEXT.enterMode}
                      </button>
                    )}
                  </span>
                </div>
              )
            }
            /* 关窗 = 只关不丢；没保存的改动会喊一声（见 `closeEdit`） */
            onClose={closeEdit}
          >
            {editing !== null && (
              /*
               * 正文区。**编辑态** = 着色的 <pre>（下）+ "文字透明"的 textarea（上）——
               * 自己写的语法高亮（词法在 `tomlPaint.ts`）。两层同字体 / 同内衬 / 同行高，
               * 逐像素对齐；滚动由 textarea 管，`onScroll` 搬给下面那层**与左边的行号槽**。
               * **只读态** = 就那层 <pre>（`data-ro`：自己滚、能选字，没有 textarea）。
               * **行号槽**（`.editNums`，2026-10-16 加）两态都在，只是编辑态它才真有用
               * （预设行"用不了"的原因按行报：「TOML 语法不对（第 3 行第 1 列）」）。
               * 这一块**占满整屏**（`bodyFill`：框身不滚，四周留白交给 `.editWrap`）。
               */
              <div className={s.editWrap}>
                {/*
                 * 正文那块 = **一个面板**（`.editBody`）：里面是绝对定位铺满的着色层 + textarea，
                 * 以及**左边那一列行号**（`.editNumsGutter`，也是绝对定位的一条刻度）。
                 *
                 * ★★★ 行号**不是**单独一块板子（2026-10-16 第三版，作者看过上一版之后：
                 *   「它又有这个圆角，然后又有这一个什么的」「左边的那个跟右侧的是分开的，
                 *   很割裂」）—— 它就在面板自己的左内衬里画着，面板那圈描边 / 圆角只有一套，
                 *   滚动也是同一个容器一起滚。上一版那"两块拼起来"的结构已拆掉。
                 */}
                <div
                  className={s.editBody}
                  /* 占位式滚动条的平台才非零（macOS 上是 0），见 `scrollbarPad` */
                  style={{ '--scrollbar-pad': `${scrollbarPad}px` } as CSSProperties}
                  /*
                   * 触控板两指左右划 / ⇧滚轮 —— 浏览器自己滚不到该动的那一层，
                   * 这里自己接管（见 `onWheel`）。
                   */
                  onWheel={onWheel}
                >
                  <div className={s.editNumsGutter} aria-hidden="true">
                    <div className={s.editNums} ref={numsRef} data-testid="edit-nums">
                      <pre className={s.editNumsInner}>{numsText}</pre>
                    </div>
                  </div>
                  <pre
                    className={s.editPaint}
                    ref={paintRef}
                    /* 编辑态时它是 textarea 的视觉副本（字透明），只读态时它就是内容本体 */
                    aria-hidden={editMode || undefined}
                    data-ro={editMode ? undefined : '1'}
                    /* 软折行（⌥Z 切）：折了就横着不再滚（`white-space` 也跟着），行号那一列随之收回 */
                    data-wrap={softWrap ? '1' : undefined}
                    /* 只读态它是那个滚动容器（`data-ro` 那条 CSS）—— 行号那一列跟着它走 */
                    onScroll={(e) => syncScroll(e.currentTarget)}
                  >
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
                  {editMode && (
                  <textarea
                    className={s.editArea}
                    ref={areaRef}
                    value={editing.text}
                    spellCheck={false}
                    aria-label="预设正文"
                    autoFocus
                    /*
                     * ★ `wrap="off"` = **不软折行**（与那两层 CSS 的 `white-space: pre` 是一件事）。
                     * 只写 CSS 也能让 Chrome/Safari 不折，但 `wrap` 属性才是这条规矩的本源
                     * （HTML 规范说 textarea 的换行由它管），两处都写死。
                     */
                    wrap={softWrap ? 'soft' : 'off'}
                    /* 软折行（⌥Z 切）：折了就横着不再滚（`white-space` 跟着换；行号那一列照旧在） */
                    data-wrap={softWrap ? '1' : undefined}
                    onScroll={(e) => syncScroll(e.currentTarget)}
                    /*
                     * **文件头那几行不给改**（`HEAD_LOCKED`）。
                     * 先把"不改内容"的键放过去：只有 `Backspace` / `Delete` / `Enter` /
                     * 不带 ⌘·Ctrl 的单字符才算要动正文（带修饰键的那些是 ⌘A / ⌘C / ⌘V
                     * 一类，粘贴与剪切各有 `onPaste` / `onCut`）。
                     */
                    onKeyDown={(e) => {
                      /* ⌥Z 不在这里管：它在模态框那一层统一接（见 `useEffect` 那段），
                         这里只管"文件头那几行不给改"这道闸 */
                      const el = e.currentTarget
                      const a = Math.min(el.selectionStart, el.selectionEnd)
                      const isDel = e.key === 'Backspace' || e.key === 'Delete'
                      const isType =
                        e.key === 'Enter' || (e.key.length === 1 && !e.metaKey && !e.ctrlKey)
                      if (!isDel && !isType) return
                      const from =
                        e.key === 'Backspace' && el.selectionStart === el.selectionEnd ? a - 1 : a
                      headLockedAt(e, from)
                    }}
                    onPaste={(e) => {
                      const el = e.currentTarget
                      headLockedAt(e, Math.min(el.selectionStart, el.selectionEnd))
                    }}
                    onCut={(e) => {
                      const el = e.currentTarget
                      headLockedAt(e, Math.min(el.selectionStart, el.selectionEnd))
                    }}
                    onChange={(e) => {
                      /* 用户敲的（不是撤销/重做造的）—— 下一拍落盘要进撤销栈 */
                      histJumpRef.current = false
                      const next = e.target.value
                      setEditing((cur) =>
                        cur === null ? cur : { ...cur, text: keepHead(cur.text, next) },
                      )
                    }}
                  />
                  )}
                </div>
                {/*
                 * **怎么看全 + 怎么能改** —— 一句常驻说明（作者 2026-10-16 那天连问了三件事：
                 * 横向怎么滚、有没有 ⌥Z、上面那几行为什么灰着）。这三件都**没有可见的入口**，
                 * 不写在这儿用户只能猜：横向滚动条平时看不见（只在滚的时候浮出来）、
                 * ⌥Z 是快捷键（更没有形）、文件头那几行锁着（看着像能改）。
                 *
                 * 常驻而不是 tooltip：⌥Z 这件事没说出口就等于不存在（作者原话就是问句）。
                 * ★ 词跟着做法走：横向**用「⇧滚轮 / 两指划」**（不是"⇧ 滚轮左右看"那种
                 *   把两件事混成一句），自动换行**用「⌥Z 切换自动换行」**（不写"软折行"——
                 *   那是我们内部的说法，作者问的就是"怎么换行"）。
                 * ⚠️ 它在 `.editBody` **外面**（面板那块的下一条）—— 进面板就是压在代码上。
                 */}
                <p className={s.editHint}>
                  文件头几行锁定不可改 · 两指左右划 / ⇧滚轮看长行 · ⌥Z 切换自动换行
                </p>
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
                {/*
                  **恢复默认名住底栏左侧**（2026-10-20 作者：「恢复默认名应该加到这个左下角呀，
                  做成那种幽灵按钮吧」）—— 原来它贴在输入框下面，把框撑高一格；
                  现在与「取消 / 改名」同一行、靠左，样式是幽灵按钮。
                  它只把输入框填成默认名，按「改名」才落盘。
                */}
                <button
                  type="button"
                  className={s.renameReset}
                  title={`填成默认名（${renaming?.row === undefined ? '' : (defaultNameOf(renaming.row) ?? '这一份没有默认名')}）`}
                  disabled={
                    renaming === null ||
                    renaming.busy ||
                    defaultNameOf(renaming.row) === null
                  }
                  onClick={fillDefaultName}
                >
                  {RESTORE_DEFAULT_NAME}
                </button>
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
                      : '右键任意一行还有使用 / 复制后处理脚本 / 修改参数 / 编辑文件 / 参数对比 / 复制 / 重命名 / 显示所在文件夹 / 置顶（没有鼠标就 Shift+F10 或菜单键）'
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

