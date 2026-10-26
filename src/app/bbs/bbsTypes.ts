import type { StringKey } from '../../i18n'

/*
 * BBS 那批 json 的类型。
 *
 * 全部照 `public/bbs/` 下实际文件的形状写，不是猜的 —— 那些文件是
 * `machine-motion/tools/bbs-extract.mjs` 从 Bambu Studio 的源码与安装目录提出来的，
 * 由 `npm run sync:bbs` 拷进来。
 *
 * 一条贯穿全局的约定：**值一律当「可能是数组」处理**。BBS 的多挤出头与挤出头变体
 * 就存成数组（X1C 是单喷嘴，但 `outer_wall_speed` 有 `["200","350"]` 两个值），
 * 取值必须过 `norm(v, idx)`。类型上也就不能把它写成 string。
 */

/** 一个参数的值。数字/布尔在 json 里大多是字符串，但两种都出现过，所以都留着 */
export type BbsScalar = string | number | boolean
export type BbsValue = BbsScalar | BbsScalar[]

/** key → 值。基准、用户覆盖、合并结果都是这个形状 */
export type BbsValues = Record<string, BbsValue>

/** 枚举的一个候选项。`icon` 是 icons.json 里的键名，没图标的是 '' 或缺 */
export interface BbsOpt {
  value: string
  label_en?: string
  label_zh?: string
  icon?: string | null
}

/** 控件大类。BBS 的 coXxx 原类型另存在 `co` 里，显示规则要看它 */
export type BbsParamType = 'num' | 'bool' | 'sel' | 'text'

/** registry.json 里一条参数的定义（702 条） */
export interface BbsParamMeta {
  label?: { zh?: string; en?: string }
  unit?: { zh?: string; en?: string }
  /** C++ 原类型：coFloat / coInt / coPercent / coFloatOrPercent … 数值显示规则看它 */
  co?: string
  type?: BbsParamType
  /** `f_enum_open` 这种混合控件（能输数字也能选特殊值） */
  gui_type?: string
  /** simple / advanced / develop */
  mode?: string
  category?: { zh?: string; en?: string }
  opts?: BbsOpt[]
  is_print_option?: boolean
}

export interface BbsRegistryFile {
  params: Record<string, BbsParamMeta>
  bbs_version?: string
}

export interface BbsLayoutGroup {
  name: string
  name_en?: string
  /** icons.json 里的键名，来自 Tab.cpp 的 new_optgroup 第二个参数 */
  icon?: string
  fields: string[]
}

export interface BbsLayoutTab {
  name: string
  name_en?: string
  icon?: string
  groups: BbsLayoutGroup[]
}

export interface BbsLayoutFile {
  tabs: BbsLayoutTab[]
  bbs_version?: string
}

export interface BbsDefaultsFile {
  values: BbsValues
  bbs_version?: string
}

export interface BbsIconsFile {
  /** 键名 → SVG 源码字符串 */
  svg: Record<string, string>
}

/** scripts/sync-bbs.mjs 写的那份。状态条要显示 bbs_version 与 syncedAt */
export interface BbsSyncInfo {
  from?: string
  bbs_version?: string | null
  source?: string | null
  syncedAt?: string
  files?: number
  bytes?: number
}

/**
 * 一份预设文件的内容。除了参数 key，还混着 name / inherits / from 这些身份字段
 * （`stripMeta` 会把它们剔掉，见 bbsMerge）。
 */
export interface BbsPresetDoc {
  name?: string
  inherits?: string
  instantiation?: string | boolean
  compatible_printers?: BbsValue
  print_extruder_variant?: BbsValue
  [key: string]: unknown
}

/**
 * 这一项从哪儿来。
 *
 *   `user` / `system`  本机 BBS 目录里的两份（`/api/bbs/presets` 读的）
 *   `imported`         手工拖进来/导入的那几份，内容直接带在身上
 *   `mkp`              **我们自己的**：预设页「切片器配置」下载到本机的那几份
 *                      `MKPProcess …json`（2026-10-13 加）—— 它不在 BBS 目录里，
 *                      是交付面给的东西，所以自成一档（抽屉那个「MKP」入口就是它）
 */
export type BbsScope = 'user' | 'system' | 'imported' | 'mkp'

/*
 * 清单里一项 / 一份清单的原始形态 —— **定义在 `src/api/contract.ts`**
 * （那两条 IPC 的返回形状：`BbsLocalEntry` / `BbsLocalTree`）。
 *
 * 这里只把名字转出去给页面用：名字照旧（`BbsListEntry` / `BbsListRaw`），
 * 于是 `bbsSource` 那些解析代码一行都不用改；而"形状只有一处"这件事也守住了 ——
 * 契约改了这里跟着改，不会出现两份能各自漂的定义。
 */
export type {
  BbsLocalEntry as BbsListEntry,
  BbsLocalTree as BbsListRaw,
} from '../../api/contract'

/** normalizeList 的产物：清单里一项，带认出来的机型与可选性 */
export interface BbsPresetItem {
  /** 稳定标识：同名预设可能分布在多个账号目录下，所以带 uid 与文件名 */
  key: string
  name: string
  scope: BbsScope
  uid: string | null
  file: string
  inherits: string | null
  instantiation: string | boolean | null
  compatible_printers: BbsValue | null
  broken: string | null
  mtime: number
  /** 名字里的简称（A1M），只用于显示 */
  model: string
  /** compatible_printers 里的全名（Bambu Lab A1 mini） */
  printer: string
  /** '0.4' / '?' */
  nozzle: string
  /** 筛选用的键：`printer|nozzle` */
  target: string
  /** instantiation:"false" 的继承链中间层，BBS 下拉里也不列 */
  isAbstract: boolean
  selectable: boolean
  /** 灰着的**字典键**与它的参数（见 `reasonText`）：数据层只说事实，取字在渲染处 */
  reasonKey?: StringKey
  reasonArgs?: Record<string, string>
  reason: string | null
  /**
   * **我们自己的那一份还没下载**（2026-10-13）：目录里登记着、盘上还没有。
   *
   * 它照样进清单（灰着列在 MKP 组里，机型与喷嘴按目录登记认），但**不可选** ——
   * 点了不是切预设，是去预设页把它取回来。原来这几份一份都不列，
   * 于是"我们其实发过它"这件事在界面上完全看不见。
   */
  notDownloaded?: boolean
  /** 内容直接带在身上的那一份（导入的 / 我们自己的切片器配置）—— 不用再 fetch */
  doc?: BbsPresetDoc
}

/** 抽屉里一项的显示形态 */
export interface BbsMenuItem {
  key: string
  label: string
  scope: BbsScope
  model: string
  printer: string
  nozzle: string
  target: string
  file: string
  uid: string | null
  current: boolean
  /** BBS 里改动未保存的预设，名字前面带 * */
  star: boolean
  disabled: boolean
  offModel: boolean
  reason: string | null
  /**
   * 还没下载的我们那几份：**可点，但点了是"去预设页取"**（不是切预设）——
   * 所以它不走 `disabled`（那是"真点不动"），由抽屉另给一个动作。
   */
  notDownloaded: boolean
  /**
   * 「这一份为什么灰着 / 为什么不在这一档」的**字典键**（2026-10-26）。
   * 数据层只说事实，取字在渲染处：这一层拿不到 `tr`，也不该拿。
   */
  reasonKey?: StringKey
  reasonArgs?: Record<string, string>
}

export interface BbsMenuGroup {
  /**
   * 组名的**字典键**，不是字面量（2026-10-26）：组名是界面句子，随语言变。
   * 数据侧只存键，取字在渲染处（`tr(titleKey)`）。
   */
  titleKey: StringKey
  items: BbsMenuItem[]
  weak?: boolean
}

/** 机型 + 喷嘴的筛选档位 */
export interface BbsTargetOption {
  key: string
  label: string
  count: number
  level: 'printer' | 'nozzle'
}

/** 条件显隐的结果。`line` 为假整行收起、`field` 为假留行置灰 —— 两者不是一回事 */
export interface BbsToggle {
  line: boolean
  field: boolean
}

/** 规则里读得到但工艺预设里没有的那 5 类东西 */
export interface BbsEnv {
  is_BBL_printer: boolean
  gcode_flavor: string
  is_global_config: boolean
  printer_model: string
  wrapping: boolean
}

/**
 * 数据来源：`live` = serve 期端点实时读本机 BBS 目录（唯一的清单来源）。
 * `none` = 读不到本机目录（没装 BBS / 没起 dev 或 preview 服务器），页面出空态。
 *
 * **没有 `snapshot` 这一档了**：本仓不打包那 285 个预设快照（作者裁决 §6-2），
 * 所以「退到仓库里那份陈旧快照」这条路被刻意拿掉 —— 宁可如实说读不到。
 */
export type BbsSourceMode = 'live' | 'none'

/** 两档显隐：全部参数 / 跟 BBS 一样 */
export type BbsViewMode = 'all' | 'bbs'
