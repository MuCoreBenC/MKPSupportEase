/*
 * 参数页的全部状态。
 *
 * # 与早先几版的区别只有一处：数据源
 *
 * 值模型、撤销重做、修改历史的语义照旧重建 —— 那套在之前的稿里已经调顺了，
 * 没有要改的理由。换掉的是数据：C4 之前字段表来自 localStorage 那格"说明书"
 * （`ClientDataPackage` —— 那套 2026-10-04 已随模拟云端一起退役），
 * 现在全部走**运行时 catalog**（总纲判据 4 的正面落点）：
 *
 *   字段定义 / 条件      api.getParamMeta()（catalog 的 definition）
 *   分类 / 分组 / 条数   catalog 的 registry 摊（`tabsOf`）
 *   值与来源层          api.getMachineParams(m, v)（三层取值，按 combo 拉）
 *   机型与版本          api.getMachines()
 *   这个版本的文件      catalog 的 files 域（挑 `mkp_preset` 那份）
 *
 * 手抄表和真注册表的总数都是 67，但最后两类的切分完全不同（手抄：涂胶 31 / 更多 4，
 * 真注册表：涂胶 18 / 更多 17）。总数对得上所以一直没人发现 —— 这里以真注册表为准。
 *
 * # 三层值
 *
 *   出厂     `RecipeParam.baseValue`（被版本盖过的那几条有）否则加载时 `origin === 'base'` 的值
 *     ↓ 已保存  保存动作把草稿并进来。**按「机型:版本」分开记**，切回去还看得见
 *       ↓ 草稿  未保存的改动
 *
 * 显示读 `draft ?? saved`。中间那一层不能省：行上那个「↩」chip 要退回的是**已保存值**，
 * 而「恢复默认值」退回的是**出厂值** —— 保存过一次之后这两个就不是一回事了。
 * 早先那一版的 chip 写的是出厂值，保存之后它就在撒谎（说退回 -0.9，磁盘上其实已经是 -0.89）。
 *
 * # 唯一写入口
 *
 * 所有改值动作都过 `apply(label, patches)`：一次手势 = 一条撤销 + 一条日志，
 * 不管它改了 1 项还是 67 项。三个动作各自改草稿的写法在加撤销时必漏一个。
 *
 * # 副作用不写在 setState 的 updater 里
 *
 * StrictMode 下 updater 会被调两次，在里面 push 另一个栈就会多推一条。
 * 所以撤销/重做都在回调体里读栈、算好了再一次性 set。
 *
 * # 加载失败不装空
 *
 * `loading` / `error` / ready 三态显式。拉不到就说拉不到 —— 静默给空数组的话界面会显示
 * 「共 0 条」，那是假信息。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, errorText } from '../../api'
import { msg, useLocale, useT, type StringKey, type Text, type Tr } from '../../i18n'
import {
  localizeMachines,
  localizeParamDef as localizeDef,
  localizeTab,
} from '../shared/dataLabel'
/* 「保存后的重切引导」共用开关（读盘上真值，决定存完弹不弹那一屏） */
import { guidesEnabled } from '../guide/guidesPref'
import { activateCombo, activateFile, activePresetSnapshot, useActivePreset } from '../state/appState'
import { draftMutated } from '../state/draftState'
import { deliveryMutated, useDeliveryRevision } from '../state/deliveryState'
import type {
  ActiveOrigin,
  CatalogParamDef,
  Machine,
  MachineVersion,
  ParamDecision,
  ParamMeta,
  ParamSection,
  ParamSyncEntry,
  ParamTab,
  Plate,
  PresetParamSync,
  RecipeParam,
  LocalPresetFile,
  RuntimeCatalog,
} from '../../api'
import type { FieldSchema } from '../../components/field'
/* 显示精度唯一出口（`mm` 两位、其余原样）—— 与 `FieldControl` / `NumberField` 同一套 */
import { displayDecimals, formatNum } from '../../components/field/value'
/* 关掉模态框再进来的会话缓存（草稿层 + 撤销重做 + 修改历史），见它头注 */
import {
  clearParamSession,
  paramTargetKey,
  readParamSession,
  writeParamSession,
} from './paramSessionCache'

/**
 * 参数编辑器的两种打开方式。
 *
 * - **不带 `target`**：整页参数台 —— 按「机型 · 版本」挑编辑目标（catalog 里那张
 *   `editTargetByCombo` 表说这个组合编辑谁），用户能在预设抽屉里换。
 * - **带 `target`**：右键某一份预设打开的那一个 —— **钉死这一份**
 *   （`presets-local/…` 里的那一个路径），机型 / 版本只用来决定字段表与分组，
 *   用户换不了目标。两次打开同一份预设得到的是同一件事，与「当前应用」无关。
 *
 * 两者**共用同一个 hook 与同一套组件**：外壳不同、编辑的对象来源不同，别的全一样。
 */
export interface UseParamsOptions {
  /** 钉住编辑目标（右键某一份预设进来时给） */
  target?: { path: string; fileName: string; machineId: string; versionId: string }
}

/** 一条字段定义。**刻意不带当前值** —— 值走 `valueOf()`，免得行数据里那份过期 */
export interface ParamDef {
  key: string
  label: string
  /** 英文显示名（双语注册表）。空 = 这条还没译；`localizeDef` 按界面语言挑 */
  labelEn?: string
  desc: string
  /** 英文说明（空 = 回落 `desc`） */
  descEn?: string
  /** 中文分组名（注册表的 section 名，'空间偏移' 这种） */
  group: string
  sectionId: string
  unit?: string
  /** 交给 FieldControl 的形状 */
  field: FieldSchema
  meta?: ParamMeta
  /**
   * 字段级弃用（2026-10-02，作者裁决）：该字段已退出正常编辑 / 产物生成，但仍是已知参数，
   * 所以参数页**照常显示**它的历史状态。
   *
   * **显示 ≠ 可编辑 ≠ 会进入新产物**：行照常列（红线 + 「已弃用」徽章），但控件只读、
   * 写值闸拒绝、也不再有新 TOML（`getMachineParams` 本就排除它）。
   *
   * 判据来源 = `meta.deprecated`（定义通道，也就是 `getParamMeta`）—— 与配方通道无关；
   * 配方通道不下发弃用字段，所以它的 `RecipeParam.deprecated` 正常恒为假。
   */
  deprecated: boolean
}

/** 一个字段在分类 / 分组结构里的位置。搜索结果要写清命中项属于哪一组 */
export interface Place {
  tabId: string
  tabLabel: string
  sectionId: string
  sectionLabel: string
}

/** 某个字段现在被哪个父字段的条件关着 */
export interface BlockedBy {
  key: string
  label: string
  /** 「等于 开启」这种，已经是可以直接显示的一句 */
  need: string
}

/**
 * 一条改动（操作记录底座 ④）。
 *
 * **成形那一刻就把显示要用的上下文快照下来**（`label` / `tab` / `section` / `unit`），
 * 而不是等到渲染时回查当前 combo 的字段定义 —— 回查有两个毛病：
 *
 *   ① 切了机型 / 版本之后，历史条目里的 key 在新 combo 里可能**根本不存在**，
 *      于是那一条就显示成裸 key（`toolhead.offset.x`），正是"历史没上下文"的根因；
 *   ② 定义会随目录（catalog）更新而变，而"当时我改的是哪一项、它叫什么"是**过去的事实**，
 *      不该被后来的改名改写。
 *
 * 值本身（`from` / `to`）也从一开始就是**文本**：它记录的是"当时框里写的是什么"，
 * 不是"现在按新定义应该显示成什么"。渲染成 ``` `-1 → -1.5` ``` 这种句子是界面的事。
 */
export interface HistoryItem {
  key: string
  from: string
  to: string
  /** 参数中文名（如 `X 轴偏移`）。查不到定义时退回 key —— 但那是异常，不是常态 */
  label: string
  /** 分类（页签名，如 `偏移`） */
  tab: string
  /** 分组（section 名，如 `空间偏移`） */
  section: string
  /** 单位（如 `mm`）；没有就是 `null` */
  unit: string | null
  /** 这一项最后那一次改动是「云端更新」带来的 —— 修改历史里挂「更新」徽章 */
  fromSync?: boolean
}

export interface HistoryEntry {
  id: number
  /** 给人看的序号，从 1 开始，不随撤销变动 */
  no: number
  action: string
  items: HistoryItem[]
  state: 'draft' | 'saved' | 'undone'
  /** 第几次保存把它落地的；`null` = 还没保存过 */
  batch: number | null
  /**
   * 发生那一刻（UTC epoch 秒）。**落盘之后才有的字段**（2026-10-17）：
   * 内存日志时代"什么时候改的"没处去，持久化之后它跟着这一条一起留着。
   * 2026-10-20 起界面上真的用它了：修改历史的段头右侧那一列时间就是段内最大值
   * （作者：「就在这个第几次保存后面显示一个时间呢？」）。
   */
  tsUnix?: number
  /**
   * 这一条是不是**「云端更新」落下来的**（2026-10-20）—— 修改历史里挂一枚「更新」徽章。
   *
   * 与普通改值分开记：两者在账本里长得一样（`改 X 轴偏移` / `0.1 → 0.7`），
   * 但用户想知道"这条是新版带来的，还是我自己调的"。判据只能存下来，不能事后从
   * action 文案里猜（文案会改，猜错就成了骗人的标注）。
   */
  fromSync?: boolean
}

export interface HistoryGroup {
  /** `null` = 未保存那一段 */
  batch: number | null
  title: string
  items: HistoryEntry[]
  /**
   * 这一段的**时间**（UTC epoch 秒；取段内条目 `tsUnix` 的最大值 = 这一批最后落笔那一刻）。
   * `null` = 未保存那一段（还没有"落盘时刻"可言）。
   */
  atUnix: number | null
}

/**
 * 一批改动 → 只留**净变化**（2026-10-16 作者的口径：「按保存批次的话……
 * 它只记录那个最终的」）。
 *
 * · 同一项在这一批里改了几次只看**头尾**：`70 → 75 → 80 → 90` 就是一条 `70 → 90`；
 * · 头尾相同的整条不出现：`-0.7 → -0.75 → -0.7` 等于没改（作者原话：
 *   「如果你看最后他从 -0.7 到最后还是 -0.7，那就不算，不用记录了」）；
 * · **撤回过的那几条跳过** —— 撤销已经把值退回去了，后面那条的 `from` 里已经算过它。
 *
 * 出去的顺序 = 各项**在这一批里第一次被改**的先后；`from` / `to` 取的是那句话
 * 当时的快照（`item.label` / `tab` / `section` / `unit` 用最近那一条）。
 */
export function netOf(entries: HistoryEntry[]): HistoryItem[] {
  const head = new Map<string, HistoryItem>()
  const tail = new Map<string, HistoryItem>()
  /** 这一项最后一次改动是不是云端更新带来的（跟随 `tail` 取最后一条） */
  const fromSync = new Map<string, boolean>()
  for (const e of entries) {
    if (e.state === 'undone') continue
    for (const it of e.items) {
      if (!head.has(it.key)) head.set(it.key, it)
      tail.set(it.key, it)
      fromSync.set(it.key, e.fromSync === true)
    }
  }
  const out: HistoryItem[] = []
  for (const [key, first] of head) {
    const last = tail.get(key)
    if (last === undefined || first.from === last.to) continue
    out.push({
      ...last,
      from: first.from,
      to: last.to,
      ...(fromSync.get(key) === true ? { fromSync: true } : {}),
    })
  }
  return out
}

/** 一处要写入的改动 */
export interface Patch {
  key: string
  /** `undefined` = 撤掉草稿（回到已保存的值） */
  value: string | undefined
}

/**
 * `apply` 的结果 —— 三种，缺一不可：
 *
 *   `applied`  真写进去了（撤销栈 / 历史各多一条）
 *   `noop`     目标值与现值**逐项相同**，没有可写的（不产生任何记录）
 *   `blocked`  写值闸拦住了（弃用字段 / 弃用选项），已经写好一句人话（`gateNote`）
 *
 * ★ 存在的理由：「历史 → 还原」点了要回话。原来 `apply` 不说话，点了命中 noop
 *   就静悄悄 —— 作者的原话是「怎么我点了还原没反应…是真的没还原还是没有交互效果」。
 */
export type ApplyResult = 'applied' | 'noop' | 'blocked'

/** 一条撤销记录（导出是给 `paramSessionCache` 存会话快照用的） */
export interface EditStep {
  patch: Record<string, string | undefined>
  before: Record<string, string | undefined>
  logId: number
  /**
   * 这一步**之前 / 之后**的云端表态表（2026-10-20）。整份快照，不是增量 ——
   * 撤销要把"已采用 / 已保持"一起退回去，否则值退了、徽章还挂着。
   * 旧会话快照里没有这两格（`undefined`）⇒ 退回空表，不按缺省值编。
   */
  syncBefore?: Record<string, SyncDraftKind>
  syncAfter?: Record<string, SyncDraftKind>
}

/** 草稿层对某一项云端更新的表态 */
export type SyncDraftKind = 'adopt' | 'hold'

/** `apply` 的可选交代（只有云端更新那一批用得上） */
export interface ApplyOptions {
  /**
   * 这一批是**「云端更新」落下来的** —— 修改历史给整条挂一枚「更新」徽章。
   * 值怎么变、怎么撤销与普通编辑完全一样，只有"来路"这一个标注不同。
   */
  fromSync?: boolean
  /**
   * 这一步要写进表态表的项（`null` = 撤掉那一项的表态）。
   *
   * 由调用方一次交代全：采用项即使这次 no-op（值本来就等于云端新值）也要记下来，
   * 保持项则根本不会进 `patch`。
   */
  sync?: Record<string, SyncDraftKind | null>
}

interface Catalog {
  machines: Machine[]
  metaByKey: Map<string, ParamMeta>
  /**
   * 打印板，按 id 索引（2026-10-02，③）。塔地图按机型的 `defaultPlateId` 从这里查板；
   * 查不到就退回「无板」（画布退回圆角矩形）。
   */
  platesById: Map<string, Plate>
  /**
   * **字段清单的权威**（definition 通道，`catalog.registry.params`）。
   *
   * 2026-10-02（作者裁决）：弃用字段「显示，但只读」—— 参数页的 defs 要以它为准来列字段，
   * 而不是只看配方通道（`getMachineParams` 会排除弃用字段）。它带 label / desc / choices /
   * uiComponent / valueType，正好够渲染一行；值本体仍走配方通道的 `baseByKey`。
   */
  registryParams: Map<string, CatalogParamDef>
  /** 页签与分组（页面级，不分机型；从 catalog 的 definition 摊） */
  tabs: ParamTab[]
  /** 每个组合的 MKP 文件名（`A1:STANDARD` → `A1-fastv3.3.toml`）—— 从 catalog 的 files 摊出来 */
  fileByCombo: Map<string, string>
  /**
   * 每个组合对应的**可编辑文件身份**（参数页底座 ③）。
   *
   * 参数页要改一份文件就得知道三件事：文件名、哪条线、用户线的落点 ——
   * `beginPresetEdit` 正是要这三个。云端线落点恒 `null`（由目录给），用户线给 `path`。
   *
   * 云端那一半从 catalog 的 files 摊（`kind === 'mkp_preset'`）；用户那一半从
   * `getLocalPresetFiles` 来（我那份认的是 `path`，机型/版本从文件名反推）。
   */
  editTargetByCombo: Map<string, { fileName: string; origin: ActiveOrigin; path: string | null }>
}

interface ComboData {
  machineId: string
  versionId: string
  tabs: ParamTab[]
  defs: ParamDef[]
  defByKey: Map<string, ParamDef>
  placeByKey: Map<string, Place>
  /** api 给的当前值 = 「文件里现在写着什么」的基准 */
  baseByKey: Map<string, string>
  /** 出厂值 */
  factoryByKey: Map<string, string>
  /** 这个 combo 的 MKP 文件名（`A1.toml`）。包里这个版本没有 MKP 文件时是 null */
  fileLabel: string | null
}

/** 条件三档的说法住字典（`params.op.*`）—— 这一层只存键，取字交给调用方传进来的 `tr` */
const OP_KEYS: Record<'eq' | 'neq' | 'gt', StringKey> = {
  eq: 'params.op.eq',
  neq: 'params.op.neq',
  gt: 'params.op.gt',
}

/* ---------- catalog → 这一页要的形状（唯一数据源 = catalog，总纲判据 4） ---------- */

/**
 * catalog 的 definition → 页签与分组（名字、顺序、条数全部来自 catalog，客户端不排一遍）。
 *
 * 只留**装参数**的 section：registry 里 27 个 section 有 11 个是设置页的 `component`
 * 占位（一个参数都没有），按 27 个建分类会多出 12 个永远为空的页签 ——
 * 与 Rust 侧 `ParamRegistry::param_tabs` 同一条过滤。参数归属按 **layout** 摊
 * （`layout_schema` 的 items 是权威：哪个参数落在哪个 section），中文名取 `[[tabs]]`。
 */
function tabsOf(registry: RuntimeCatalog['registry']): ParamTab[] {
  const paramKeys = new Set(registry.params.map((p) => p.key))
  const itemsBySection = new Map<string, number>()
  for (const tab of registry.layout) {
    for (const sec of tab.sections) {
      itemsBySection.set(
        sec.id,
        (itemsBySection.get(sec.id) ?? 0) + sec.items.filter((i) => paramKeys.has(i.paramKey)).length,
      )
    }
  }

  const out: { tab: ParamTab; order: number }[] = []
  for (const meta of registry.tabs) {
    const sections: ParamSection[] = meta.sections
      .map((s) => ({
        id: s.id,
        label: s.label,
        labelEn: s.labelEn,
        desc: s.description,
        descEn: s.descriptionEn,
        count: itemsBySection.get(s.id) ?? 0,
      }))
      .filter((s) => s.count > 0)
    if (sections.length === 0) continue
    out.push({
      tab: {
        id: meta.id,
        label: meta.label,
        labelEn: meta.labelEn,
        count: sections.reduce((n, s) => n + s.count, 0),
        sections,
      },
      order: meta.order,
    })
  }
  /* 页签先后以 [[tabs]] 的 order 为准；没声明 order 的排最后 */
  return out.sort((a, b) => a.order - b.order).map((x) => x.tab)
}

function testCondition(op: 'eq' | 'neq' | 'gt', actual: string, expected: string): boolean {
  if (op === 'eq') return actual === expected
  if (op === 'neq') return actual !== expected
  return Number(actual) > Number(expected)
}

/**
 * 布尔的写法归一。
 *
 * 假后端的 `toText()` 把 JSON 的 true/false 转成了 `'on'` / `'off'`（对齐旧程序的两段文案），
 * 而共用件的 `boolOf` 只认 `'true'` —— TOML 里那一行写的也是 `true`。
 * 所以在这里、也只在这里归一。不在共用件里多认一种写法：那等于默许数据层长出第二种形态。
 */
function normalizeRaw(raw: string | undefined, valueType: ParamMeta['valueType'] | undefined): string {
  if (raw === undefined) return ''
  if (valueType !== 'bool') return raw
  if (raw === 'on' || raw === 'true') return 'true'
  if (raw === 'off' || raw === 'false') return 'false'
  return raw
}

/**
 * 五种 uiComponent → 控件类型。
 *
 * 契约的 `RecipeParam.control` 没有 gcode 这一档，两个 G-code 字段是以 `'text'` 到达的
 * （实测 control 分布 42 number / 13 choice / 10 switch / 2 text，那 2 个 text 就是它们）。
 * 真身份在 `ParamMeta` 里：`uiComponent === 'gcode'`。
 */
function controlOf(
  control: 'number' | 'switch' | 'choice' | 'text',
  meta: ParamMeta | undefined,
): FieldSchema['control'] {
  if (meta?.uiComponent === 'gcode') return 'gcode'
  return control
}

/**
 * 定义侧的 `uiComponent` 原词 → 契约的四档 `control`。
 *
 * 只在**配方通道没给这条**（弃用字段）时才走这里 —— 那种情况没有 `RecipeParam.control`
 * 可用，只能从定义侧的 `uiComponent` 摊。映射与后端 `machine_params_dto` 同口径：
 * `segmented` / `select` 都并成 `choice`，`gcode` 落成 `gcode`（共用件自己画块）。
 */
function controlFromUi(ui: string): 'number' | 'switch' | 'choice' | 'text' {
  if (ui === 'switch') return 'switch'
  if (ui === 'segmented' || ui === 'select') return 'choice'
  if (ui === 'gcode') return 'text'
  return 'number'
}

/**
 * 单位字段的写法归一：**长得像区间的 unit 当成没有单位**。
 *
 * 真注册表里 `unit` 不是每次都写单位 —— 风扇速度那条写的是 `"unit": "0-255"`
 * （同一个对象的 min 0 / max 255 就在旁边），也就是 **min/max 被塞进了 unit 位**。
 * 照原样渲染出来是三处一样的话：
 *
 *   行里的步进器   `250 0-255`    —— 值后面跟着一个假单位
 *   行里的还原胶囊 `已保存的 255 0-255`
 *   保存确认       `255 0-255 → 250 0-255`
 *
 * 作者的原话：「不要显示区间 0-255 这种」。所以这里只认这一种脏数据，把它当**没有单位**：
 *
 *   · 范围一点没丢 —— 它在 min/max 上，行展开的「取值范围」写的是 `0 ~ 255`，
 *     步进器也照样按 0/255 夹值（`FieldControl` 已经把 min/max 递下去了）
 *   · 真单位（mm / mm/s / 秒 / %）一律照原样显示，**不在这里发明第二种规则** ——
 *     全仓库只有这一条 unit 是区间形状（实测 grep：`"unit": "<数字>-<数字>"` 命中 1 条）
 */
const RANGE_LIKE_UNIT = /^\s*\d+(?:\.\d+)?\s*[-~～—–]\s*\d+(?:\.\d+)?\s*$/

/**
 * 单位归一：`null` 与 `undefined` **同义 = 没有单位**。
 *
 * ★ 2026-10-17：「擦料塔挤出倍率」曾显示成 `1 null`。注册表里那一条根本没有 `unit`
 *   字段，但配方/定义通道过来是 JSON 的 `null` —— 而下游判的是 `!== undefined`，
 *   `null` 就那么一字不改地拼进了字符串。这不是脏数据，是**两种"没有"**。
 *   所以这里把 `null` 也吞掉，往后的类型只有 `string | undefined` 一种缺失形态。
 */
function unitOf(raw: string | null | undefined): string | undefined {
  if (raw == null) return undefined
  return RANGE_LIKE_UNIT.test(raw) ? undefined : raw
}

/**
 * 某个值该显示成什么字 —— 还原 chip、历史条目、保存确认、行内只读值全走这一套翻译。
 *
 * 两条规矩（2026-10-17）：
 *   1. **有单位才拼单位**（`null` 不算，见 `unitOf`）；
 *   2. **数字按显示精度渲染**（`formatNum`）：`mm` 固定两位（`-0.50` / `24.00` / `3.70`），
 *      其余单位原样、只修浮点尾巴。盘上的原文一个字都不改 —— 这里只管"给我看的字"。
 */
export function valueText(def: ParamDef | undefined, raw: string, tr: Tr): string {
  if (raw === '') return tr('params.value.empty')
  if (def === undefined) return raw
  if (def.field.control === 'switch') {
    return raw === 'true' ? tr('params.value.on') : tr('params.value.off')
  }
  if (def.field.control === 'choice') {
    return def.field.choices?.find((c) => c.value === raw)?.label ?? raw
  }
  if (def.field.control === 'gcode') {
    return tr('params.value.lines', { count: raw.split('\n').length })
  }
  /* 数字按显示精度走；非数字（`tower` / G-code 之类）`formatNum` 原样退回 */
  const text = formatNum(raw, displayDecimals(def.unit, def.field.step))
  return def.unit != null ? `${text} ${def.unit}` : text
}

const cellKey = (machineId: string, versionId: string, key: string) =>
  `${machineId}:${versionId}:${key}`

/**
 * 数字字段的**即时校验**：返回一句人话原因，`null` = 合法（非数字控件恒 `null`）。
 *
 * 两处消费：
 *   · 参数行 / 步进器边上那行红字（打字过程中就出，见 `NumberField` 的 hint ——
 *     它自己有一份本地判据，两边口径一致：空 / 不是数 / 超出 min~max）；
 *   · **保存闸**：后端 `commit_preset_draft` 的 patch 只换值、不做范围校验
 *     （`patch_preset_toml` 头注），所以"超范围不许保存"只能设在界面 ——
 *     保存确认弹层把超范围项列出来、灰掉「确认保存」，`save` 里再拦一道兜底。
 */
export function invalidNumberText(def: ParamDef, raw: string, tr: Tr): string | null {
  if (def.field.control !== 'number') return null
  const t = raw.trim()
  if (t === '') return tr('params.invalid.empty')
  const n = Number(t)
  if (!Number.isFinite(n)) return tr('params.invalid.number')
  const { min, max } = def.field
  if ((min !== undefined && n < min) || (max !== undefined && n > max)) {
    return tr('params.invalid.range', {
      min: min ?? tr('params.unbounded'),
      max: max ?? tr('params.unbounded'),
    })
  }
  return null
}

/** 现在（UTC epoch 秒）—— 历史条目落盘时刻用（与后端 `ts_unix` 同一口径） */
function nowSec(): number {
  return Math.floor(Date.now() / 1000)
}

/**
 * 「已应用的那一份 × 当前在看的那一份」。
 *
 * 只剩一个判据：**当前看的这份是不是已应用的那份** —— 顶部文件名 pill 用它定颜色
 * （是 → 绿；不是 → 琥珀）。
 *
 * 曾经它还要支撑一颗「回到已应用」的按钮（所以带着 `machineId` / `versionId` /
 * `canJump`）；作者 2026-10-20 把那整个功能砍了（「没有这个回到已应用的这个功能了」），
 * 那些字段跟着退役 —— 留着就是一份没人读的坐标，下一个人还得猜它为什么在。
 */
export interface ActiveUse {
  /** 当前看的这份就是已应用的那份 */
  onIt: boolean
}

export interface Params {
  loading: boolean
  /** null = 没出错。非 null 时页面要把这句话显示出来，不要装成空表（存键，渲染时再翻） */
  error: Text | null

  machines: Machine[]
  /**
   * 本机真有的 MKP 预设（`presets-local/` 扫描结果，**已按 combo 归属过**）
   * —— 预设抽屉要的就是它，与预设页「本地」那一面同源。
   */
  localPresets: LocalPresetFile[]
  /**
   * 本机文件面变了就重读 `localPresets`（别处下载 / 删除 / 另存之后的跨页刷新）。
   *
   * ★★ 2026-10-14 补：`localPresets` 原来只在装载 effect 里拉一次 ⇒ 在预设页下载完
   * 一份，切回参数页打开抽屉还是旧清单。**代次本身由本 hook 内部订阅**，
   * 这个字段只是把它回传给调用方（页面可以拿它当自己那一路重读的依赖）。
   */
  deliveryRevision: number
  machineId: string
  versionId: string
  machine: Machine | undefined
  version: MachineVersion | undefined
  /**
   * 当前 combo 的 MKP 文件名（`A1.toml`）—— 从说明书里版本的 files 清单来。
   * `null` = 这个版本没配 MKP 文件（如 A2L），页面自己退成「机型 · 版本」。
   */
  fileLabel: string | null
  /**
   * **正在使用的那份**的文件名（AppState 底账答的）。`null` = 还没用任何一份。
   * 顶部 pill 优先说它（底账命中当前 combo 时）—— pill 回答的是"正在用哪份"，
   * 与「正在编辑哪份」（`fileLabel`，确认框 / 历史标题用）是两个问题。
   */
  activeFileName: string | null
  /** 任一组合的 MKP 文件名 —— 抽屉里每一项都显示自己的 */
  fileOf: (machineId: string, versionId: string) => string | null
  /**
   * 「已应用的那一份」的现况 —— 顶部文件名 pill 用它定颜色与提示。
   *
   * `null` = 底账空（还没应用过任何一份）：没有可对比的对象，pill 保持原样、不出提示 ——
   * 提示要防的是「把不是已应用的那份当成自己的机器」，没有已应用的就没这个指向。
   */
  activeUse: ActiveUse | null

  /** 这台机型的分类与分组结构（6 个分类 / 16 个分组，条数已由后端算好） */
  tabs: ParamTab[]
  sectionsOf: (tabId: string) => ParamSection[]
  /** 这台机型这个版本真实看得到的字段（已排除废弃与不适用本机型的） */
  defs: ParamDef[]
  defsOfSection: (sectionId: string) => ParamDef[]
  defOf: (key: string) => ParamDef | undefined
  placeOf: (key: string) => Place | undefined
  /** 这台机型这个版本一共几条 —— 不是写死的 67 */
  total: number

  /** 显示值：draft ?? saved */
  valueOf: (key: string) => string
  /** 已保存的那一层。行上的「↩」chip 与保存确认都要它 */
  savedValueOf: (key: string) => string
  /** 这一项上一次保存之前的值（2026-10-21；没记录过 = null） */
  prevSavedOf: (key: string) => string | null
  /** 出厂值。「恢复默认值」与行详情要它 */
  factoryOf: (key: string) => string
  dirtyOf: (key: string) => boolean
  dirtyCount: number
  /** 哪些分类里有未保存的改动 —— 分类胶囊上那颗红点 */
  dirtyTabs: Set<string>
  /** 与出厂不同的条数，按当前机型实算 */
  changedCount: number

  isVisible: (key: string) => boolean
  /** 现在是哪个父字段的条件没满足。可见时返回 null */
  blockedBy: (key: string) => BlockedBy | null
  /** 一个条件此刻满不满足 —— ParamCard 的分支摊开 / 折叠判据 */
  condOn: (cond: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }) => boolean
  /** 当前机型的默认打印板（塔地图选板）；没配板就是 undefined */
  plateOf: () => Plate | undefined

  /** 唯一写入口。一次调用 = 一条撤销 + 一条日志 */
  /**
   * 一次动作写进草稿。**返回结果**（2026-10-17 加）：
   *   `applied` 真改了 · `noop` 目标值就是现值（什么都不用做）· `blocked` 被写值闸拦住
   *
   * 「历史抽屉的还原」要拿它回话 —— 点了没反应到底是"本来就一样"还是"改不了"，
   * 不区分的话用户只会觉得按钮坏了。
   */
  apply: (label: string, patches: Patch[]) => ApplyResult
  edit: (key: string, value: string) => void

  /**
   * **正在编辑的那一份预设**（参数页底座 ③）—— 文件名 + 哪条线 + 用户线落点。
   * `null` = 这个 combo 没有可编辑的文件（没配 MKP / 目录里认不出）。
   *
   * 它与"当前应用"是**两个概念**：编辑目标跟着 combo 走，改它**不动** `active-preset`。
   */
  editingPreset: { fileName: string; origin: ActiveOrigin; path: string | null } | null
  /** 草稿没落盘时的原因（一行话）；正常是 `null` */
  draftError: string | null
  /**
   * **盘上这份草稿真改过**没有（后端摘要的 `dirty`，2026-10-19）——
   * 打开时页面据此**直接进编辑模式**（与「编辑文件」抽屉 `reused && dirty` 同一条口径）：
   * 作者：「我没保存的时候关闭它应该是记住着……它一样还是保留着那个编辑模式的」。
   */
  draftOnDisk: boolean
  /** 保存后那份用户文件的路径（`presets-local/…`），用来在页面提示"存到哪了" */
  saveResult: string | null
  /**
   * **保存成功之后要不要问一句"回切片器重切"**（2026-10-15）。
   *
   * 切片器那一份参数里根本没有 MKP 这些字段（见 `docs/INVESTIGATION-BBS-INVALIDATE.md`
   * §4），它不会知道我们改了什么 —— 于是它仍认为"这一盘是切好的"，不给重切。
   * 保存成功就问一次，用户自己决定要不要看引导；关掉就不再问（下一次保存再问）。
   */
  resliceAsk: boolean
  /** 关掉那一句提问 */
  dismissReslice: () => void
  /** 行上那个 chip：退回**已保存值**（= 撤掉草稿） */
  revertToSaved: (key: string) => void
  /**
   * 底栏的「恢复默认值」：全部退回**出厂值**（目录里那份 `baseValue`），
   * 算一次动作、可整体撤销（2026-10-08 资源库改判：不再问任何历史快照）。
   */
  restoreDefaults: () => void
  /**
   * 全部未保存的改动**一次退回已保存值**（原「取消」；2026-10-16 起 =
   * 「放弃并退出」做的事——作者：「退出编辑模式就是作废，那些改的都作废，
   * 保存了才有」）。
   *
   * 走的还是 `apply`（一次动作、可整体撤销、草稿 TOML 同步落盘），
   * 不是清栈——作废也是一次改动，按撤销能整个回来。
   */
  discardAll: () => void
  save: () => void
  /** 保存之后那一行回执（成功绿 / 部分失败琥珀 / 失败红）；null = 不显示。存键（`msg`） */
  savedNote: Text | null
  /** 上面那句的语气。**由产出方定** —— 页面不许从文案里猜（一翻语言就猜错） */
  savedTone: 'ok' | 'warn' | 'bad'
  /** 写值闸拦下一次改动时那句人话（弃用字段 / 弃用选项）；`null` = 不显示。存键（`msg`） */
  gateNote: Text | null
  undo: () => void
  redo: () => void
  canUndo: boolean
  canRedo: boolean
  history: HistoryEntry[]
  historyByBatch: HistoryGroup[]
  /**
   * 「修改历史 N」徽标上的数 = **净变化小项数**（2026-10-17 作者：「要显示小项的」）——
   * 两屏的分段数据里一项一条，加起来就是它。动作条数（`history.length`）会虚高：
   * 同一项改三次在动作里是三条、在这里只是一项。
   */
  historyItemCount: number
  /** 这个组合保存过几次 */
  savedBatches: number
  /** 「还原」历史里那一条（逐项 = 那一项的净变化；整批 = 那一整段的所有项） */
  revertEntry: (entry: HistoryEntry) => ApplyResult
  /**
   * **清空这一份的修改历史**（2026-10-17 抽屉右上那颗，二次确认在前端做）。
   * 记录与批号一起归零 —— 之后的保存从「第 1 次保存」重新数起。
   */
  clearHistory: () => void

  /** 切机型 / 切版本。有未保存改动时先问一句，不静默丢 */
  requestCombo: (machineId: string, versionId: string) => void
  /**
   * **换到抽屉里点的那一份**（按文件，不是按 combo）。
   *
   * 同一 combo 下有几份文件时，`requestCombo` 表达不了"要哪一份"（它开头就按 combo 退货）——
   * 这条把"哪一份文件"整条带下去。见 `requestFile` 的注释。
   */
  requestFile: (next: {
    machineId: string
    versionId: string
    fileName: string
    path: string
  }) => void
  /** 非 null = 正在等用户确认这次切换 */
  pendingCombo: { machineId: string; versionId: string } | null
  /** 同上，但那次切换是**按文件**的（含文件名与路径） */
  pendingFile: {
    machineId: string
    versionId: string
    fileName: string
    path: string
  } | null
  confirmPending: () => void
  confirmPendingFile: () => void
  cancelPending: () => void

  /**
   * **钉住模式**：这一次编辑的对象就是某一份具体的预设（右键进来的那一个）。
   * 整页参数台是 `false` —— 那时编辑目标跟着 combo 走。
   */
  pinned: boolean

  /* ---------- 云端更新（2026-10-09） ---------- */

  /**
   * 这一份的**云端更新账**（我 / 云端旧值 / 云端新值）。
   *
   * `null` = 现在还比不了（编辑目标不是「本地预设」、云端那一版认不出、或读坏了）——
   * 界面照实说，不编一个"零项待处理"出来。
   */
  sync: PresetParamSync | null
  /** 读 / 落这次云端更新账时的问题（一句人话）。正常是 `null` */
  syncError: string | null
  /** 云端当前版的字节还没到本机 —— 只能看到"有新版本"，看不到逐项差异 */
  syncNeedsFetch: boolean
  /** 正在落决定（采用 / 保持）—— 那一行的两颗按钮据此禁用 */
  syncBusy: boolean
  /**
   * 正在为看逐项差异**取回云端当前版的字节**（2026-10-20）。
   *
   * ★ 与 `syncBusy` 是两件事：这一档是"去取"（可能发网络），那一档是"在写"（动本地文件）。
   */
  syncFetching: boolean
  /** 某一项上的云端更新账（没有这一项就是 `undefined`） */
  syncOf: (key: string) => ParamSyncEntry | undefined
  /** 待处理条数（`sync === null` 时是 0） */
  pendingCount: number
  /**
   * 这一项在**草稿层**的处理状态（2026-10-20：决定不再当场写盘）：
   *
   *   `adopt`  草稿值已经等于云端新值（"用新值"了，保存才落盘）
   *   `hold`   明确保持原样（值一个字没动，只记一条待落决定）
   *   `null`   还没表态（行上照旧给那两颗按钮）
   *
   * 采用由**草稿值**判定（`draft[key] === cloudNew`）而不是第二套状态：撤销把值退回去，
   * 决定就跟着没了 —— 否则"撤销了值、账上还写着已采用"这种自相矛盾要另写一套同步。
   */
  syncDraftOf: (key: string) => 'adopt' | 'hold' | null
  /** 已表态但还没保存的项数（底栏据此让「保存」可点、状态栏据此说"N 项待保存"） */
  syncDirty: number
  /**
   * **弹窗「应用」**：勾上的采用、未勾的保持 —— 勾上的拼成**一次** `apply`，
   * 于是一次撤销能把整批退回来（作者：「就像批量修改，只算一次撤销一批」）。
   */
  applySyncDecisions: (decisions: ParamDecision[]) => void
  /** 行上「用新值」：采用这一项（只改草稿值，进撤销栈；保存才落盘） */
  adoptSync: (key: string) => void
  /** 行上「保持原样」：只记一条待落决定，值一个字不动 */
  holdSync: (key: string) => void
  /** 详情右栏「用回我的值」：撤掉这一项的采用 / 保持，退回已保存值 */
  useMyValue: (key: string) => void
  /**
   * **把云端当前版的字节取回来**（2026-10-20）—— 只有用户明确点了「更新 /
   * 选择要更新的参数」才走它。取回来才有逐项差异与那两颗按钮。
   *
   * 返回取回来之后的那一份总账；取失败 / 这一份不认云端 ⇒ `null`（问题进 `syncError`）。
   */
  fetchSync: () => Promise<PresetParamSync | null>
}

export function useParams(opts: UseParamsOptions = {}): Params {
  /* 这一屏的话术入口：所有会显示给用户的句子都从这里出（语言一换整套跟着换） */
  const tr = useT()
  /* 数据（参数名/说明/分组）那一册 —— 双语注册表按它挑 */
  const locale = useLocale()
  /*
   * 钉住的那一份（右键预设进来）。摊成几个原始值再用 —— 对象每次渲染都是新的，
   * 拿它当 effect 依赖会让所有效果每帧重跑一遍。
   */
  const pinnedPath = opts.target?.path ?? null
  const pinnedFileName = opts.target?.fileName ?? null
  const pinnedMachine = opts.target?.machineId ?? null
  const pinnedVersion = opts.target?.versionId ?? null
  const pinned = pinnedPath !== null && pinnedFileName !== null
  /*
   * 挂载那一刻的钉住状态：catalog 那条装载效果只跑一次，它要按**当时**是不是钉住模式
   * 决定"要不要落到正在用的那一份" —— 拿 state 当依赖会让它重跑一遍（那是另一件事）。
   */
  const pinnedAtMount = useRef(pinned).current

  const [catalog, setCatalog] = useState<Catalog | null>(null)
  /*
   * 本机真有的预设（抽屉要的那一份）。
   *
   * ★ 单独一格、**跟着投递面代次重读**（而不是塞进 `catalog`）：`catalog` 那一份是目录，
   *   一变就得重算全部 combo / defs / 网络请求；而"我盘上多了份文件"只影响抽屉那一列。
   *   两件事分开重读，代价小得多（参数页是这一仓最重的一页）。
   */
  const [localPresets, setLocalPresets] = useState<LocalPresetFile[]>([])
  const deliveryRevision = useDeliveryRevision()
  const [combo, setCombo] = useState<ComboData | null>(null)
  const [error, setError] = useState<Text | null>(null)
  /** 当前选的组合。catalog 到了才知道选谁，所以初值是 null */
  const [pick, setPick] = useState<{ machineId: string; versionId: string } | null>(null)
  const [pendingCombo, setPendingCombo] = useState<{ machineId: string; versionId: string } | null>(
    null,
  )
  /* 等确认的那次"按文件切换"（与 pendingCombo 分开：那个只有 combo，这个带文件名） */
  const [pendingFile, setPendingFile] = useState<{
    machineId: string
    versionId: string
    fileName: string
    path: string
  } | null>(null)

  /** 已保存的那一层。键是 `机型:版本:字段`，所以切回去还看得见上次存的 */
  const [saved, setSaved] = useState<Map<string, string>>(() => new Map())
  /** 每个组合保存过几次 */
  const [batchesByCombo, setBatchesByCombo] = useState<Map<string, number>>(() => new Map())
  /*
   * **「上一次保存之前的值」**（2026-10-21，作者：「保存之后还是那样显示，
   * 还需要一个我的旧值；不只是更新才有，所有的都能记录下来」）。
   * 一份文件一页（钥匙 = historyKeyText），保存时把"保存前的已保存值"记下；
   * 跟着修改历史账本一起落盘（`PresetHistory.prevSaved`），清空历史一并清。
   */
  const [prevSavedByFile, setPrevSavedByFile] = useState<
    Record<string, Record<string, string>>
  >({})

  const [draft, setDraft] = useState<Record<string, string>>({})
  const [past, setPast] = useState<EditStep[]>([])
  const [future, setFuture] = useState<EditStep[]>([])
  const [log, setLog] = useState<HistoryEntry[]>([])
  const [seq, setSeq] = useState(0)
  const [savedNote, setSavedNote] = useState<Text | null>(null)
  /*
   * 那一句回执的**语气**（`ok` / `warn` / `bad`）。从前是页面拿 `startsWith('保存失败')`
   * 反推颜色 —— 文案进了字典就再也猜不准（英文里没有那个开头），所以由产出方定。
   */
  const [savedTone, setSavedTone] = useState<'ok' | 'warn' | 'bad'>('ok')

  /*
   * **正在编辑的那一份预设**（参数页底座 ③；作者 2026-10-02 定的语义）。
   *
   * 两个概念要分清，它们**解耦**：
   *
   *   当前应用（`active-preset.json`）—— 决定参数页**默认打开哪一份**，仅此而已
   *   当前编辑（这里）              —— 参数页实际正在改的那一份
   *
   * 用户切机型 / 版本只换**编辑目标**，**不等于应用它**；改 `P1S` 不会动 `active-preset`。
   * 没有应用任何预设、目录里也没有对应文件时它是 `null` —— 那时没有默认目标（页面照实说）。
   *
   * `origin` 决定改的是云端线还是用户线：云端线保存会**另存**成"我那份"，用户线**写回它自己**。
   */
  const [editingPreset, setEditingPreset] = useState<{
    fileName: string
    origin: ActiveOrigin
    path: string | null
  } | null>(null)
  /**
   * 编辑目标的**当前值**（渲染期同步一份）。
   *
   * 保存这条链要读它（`commitDraft`），但不能再把它塞进 `setEditingPreset` 的 updater
   * 里去拿 —— updater 在 StrictMode 下会跑两遍，副作用（真写盘）就跟着做两遍。
   * ref 只在渲染期赋值，没有这个毛病。
   */
  const editingPresetRef = useRef(editingPreset)
  editingPresetRef.current = editingPreset
  /** 编辑目标的草稿有没有真的落到盘上（后端 `run/draft-preset.json`） */
  const [draftError, setDraftError] = useState<string | null>(null)
  /**
   * 盘上这份草稿**真改过**没有（后端摘要的 `dirty`）—— 2026-10-19。
   *
   * 页面拿它决定"重开时要不要直接进编辑模式"：与兄弟链「编辑文件」抽屉
   * （`PagePresets.tsx` 的 `unsaved = draft.reused && draft.dirty`）同一条口径，
   * 也是作者 2026-10-16 的原话「下次打开这个模态框，它里面还是显示编辑模式……好的提示、
   * 或者通知，就是说刚才那些修改还没保存、没生效」。
   */
  const [draftOnDisk, setDraftOnDisk] = useState(false)
  /** 当前编辑目标的记号（会话缓存的钥匙）；`beginPresetEdit` 回来才落 */
  const sessionKeyRef = useRef<string | null>(null)
  /**
   * 这个编辑目标的**草稿层已经接回来过**（同一个钥匙只接一次）。
   *
   * 目录刷新（`catalog` 换了引用）会让下面那个 `beginPresetEdit` 的 effect 重跑：
   * 每次都往 `draft` 里灌一遍，会把用户这一屏刚改的值按旧快照盖回去、顺手清掉撤销栈。
   * 接回来本身也不是"每次都要"的动作 —— 只在换目标 / 冷启动那一次有意义。
   */
  const draftRestoredRef = useRef<string | null>(null)
  /** 离开（模态框关闭）时要写进会话缓存的那一份 —— 用 ref 免去闭包过期 */
  const sessionRef = useRef<{
    draft: Record<string, string>
    past: EditStep[]
    future: EditStep[]
  } | null>(null)
  /** 「未保存圆点」那条广播的尾随计时器（见 `broadcastDraftSoon`） */
  const draftSoonRef = useRef<number | null>(null)

  /**
   * 广播"草稿变了"（预设列表行上那枚橙色「未保存」小圆点按它重读轻摘要）。
   *
   * ★ 2026-10-19 补修：`state/draftState.ts` 的头注一直写着"发：参数页（`useParams`：
   * 改值 / 保存 / 取消 / 采用云端新值）"，可**实际调用者只有预设页那两处（正文链）** ——
   * 参数链一次都不发，于是那枚圆点只有等整表重读才可能亮（作者："那个橙色的点也不显示了"）。
   * 这里把它接上。
   *
   * 尾随 150ms 合并：连改几处只让对面读一次轻摘要（`get_preset_draft_summary`）。
   */
  const broadcastDraftSoon = useCallback(() => {
    if (draftSoonRef.current !== null) window.clearTimeout(draftSoonRef.current)
    draftSoonRef.current = window.setTimeout(() => {
      draftSoonRef.current = null
      draftMutated()
    }, 150)
  }, [])
  /** 保存 / 作废这种"一锤子"的时刻：当场发，别让圆点晚灭 */
  const broadcastDraftNow = useCallback(() => {
    if (draftSoonRef.current !== null) {
      window.clearTimeout(draftSoonRef.current)
      draftSoonRef.current = null
    }
    draftMutated()
  }, [])
  useEffect(
    () => () => {
      if (draftSoonRef.current !== null) window.clearTimeout(draftSoonRef.current)
    },
    [],
  )
  /** 保存后那份用户文件的路径（`presets-local/…`），用来在页面提示"存到哪了" */
  const [saveResult, setSaveResult] = useState<string | null>(null)
  /** 保存成功 ⇒ 问一句"要不要回切片器重切"（理由见返回类型那一格） */
  const [resliceAsk, setResliceAsk] = useState(false)
  /**
   * 写值闸拦下一次改动时那句人话（2026-10-02）。`null` = 没在提示。
   *
   * 与 `savedNote`（保存成功的绿字）分开：这是**拒绝**，语气与位置都不同。
   */
  const [gateNote, setGateNote] = useState<Text | null>(null)

  /*
   * **我那份文件里真实写着的参数值**（`presets-local/…` 的字节，`readPresetParams` 读的）。
   *
   * 为什么需要这一层：`getMachineParams` 给的是**目录里的配方**（机型·版本的有效值），
   * 而"本地值"是**我那份文件里写着什么** —— 两者在用户改过之后就分岔了。
   * 编辑对象是「本地预设」时，已保存那一层以文件为准；云端线 / 认不出那份才退回配方。
   * 文件里没有这一项就**不进表**（下一层回落），所以"这份没写它"与"它等于 0"分得开。
   */
  const [fileValues, setFileValues] = useState<Map<string, string>>(() => new Map())
  /** 重读那一层的计数器（采用云端新值之后文件变了，要再读一次） */
  const [fileReload, setFileReload] = useState(0)

  /* ---------- 云端更新（三方账） ---------- */
  const [sync, setSync] = useState<PresetParamSync | null>(null)
  const [syncError, setSyncError] = useState<string | null>(null)
  const [syncBusy, setSyncBusy] = useState(false)
  /** 正在为看逐项差异**取回云端当前版的字节**（2026-10-20）—— 那一颗按钮据此转圈 */
  const [syncFetching, setSyncFetching] = useState(false)
  /**
   * **草稿层已经表过态的云端更新**（2026-10-20）：`key → 'adopt' | 'hold'`。
   *
   * 为什么是一张显式的表，而不是"从草稿值反推"：
   *
   *   · 保持原样**没有值可改**，反推不出来；
   *   · 采用也可能一次 no-op（云端新值恰好等于你现在的值），照样要记下"这一版处理过了"；
   *   · 反推要靠 `cloudNew` 比字符串，云端改一次格式（`0.7` / `0.70`）就会认错。
   *
   * 它跟着 `apply` 走：改值顺手删掉这一项的表态、撤销 / 重做整份快照还原 ——
   * 于是"撤销了值、账上还写着已采用"这种自相矛盾不会出现。**不落盘**：
   * 真账（水位）在保存那一步交给后端，这里只是"还没保存的那一笔"。
   */
  const [syncDecisions, setSyncDecisions] = useState<Record<string, 'adopt' | 'hold'>>({})

  /**
   * **「恢复默认值」的基准** —— **已退场**（2026-10-08 资源库改判）。
   *
   * 那一版是"退回这份预设当初那版云端的默认值"（藏在内部 baseline 里）。
   * 现在整个功能不要了：**有问题就让用户删掉、重新取一份** —— 参数表上的「恢复默认值」
   * 退回**出厂值**（目录里那份 `baseValue`），不再去问任何历史快照。
   *
   * 于是这一条读（`getPresetDefaults`）与状态一起下线：少一次请求、少一个"我该退到哪一版"
   * 的隐性概念。
   */

  /**
   * **保存**：把草稿提交成"本地预设"（云端线另存 / 用户线写回）。
   *
   * 真值落在用户根（后端 `commit_preset_draft`），本地那一层只是"这一屏看得到的改动"。
   * 没有编辑目标时如实说，不假装保存了。
   *
   * 定义在这里（早于 `save`）是为了让 `save` 在渲染期就能引用到它。
   */
  const commitDraft = useCallback(
    (decisions: ParamDecision[] = [], path: string | null = null) => {
      const cur = editingPresetRef.current
      if (cur === null) {
        setSavedTone('bad')
        setSavedNote(msg('params.saved.noTarget'))
        return
      }
      void api.commitPresetDraft().then(
        (done) => {
          setDraftError(null)
          setSavedTone('ok')
          setSavedNote(
            cur.origin === 'local'
              ? msg('params.saved.writtenBack', { file: done.fileName })
              : done.replaced
                ? msg('params.saved.overwrote', { file: done.fileName })
                : msg('params.saved.savedAs', { file: done.fileName }),
          )
          setSaveResult(done.path)
          /*
           * 保存 = 提交点：草稿被后端清掉了（契约），所以
           *   ① 会话缓存**当场作废**（再进来该是一台干净的台；见 `paramSessionCache` 头注）；
           *   ② 广播一声"草稿变了"—— 预设列表行上那枚橙色圆点**立刻灭**（不能等 150ms 尾随）。
           */
          if (sessionKeyRef.current !== null) clearParamSession(sessionKeyRef.current)
          broadcastDraftNow()
          /*
           * 落盘成功 ⇒ 也广播一声"文件面变了"（2026-10-20）：校准页（usePreset →
           * getLocalCopyFor）按 delivery 代次重读这份 `presets-local/…` —— 不然
           * 参数存完校准页还显示打开时的旧值（作者实测的那条 bug）。
           */
          deliveryMutated()
          /* 存完了 ⇒ 这一份盘上不再有"改过没保存"的草稿（重开不该再自动进编辑模式） */
          setDraftOnDisk(false)

          /*
           * ★ **存完要重开草稿**（2026-10-17 真机那两条红字的正解）。
           *
           * 后端契约是「存完丢草稿」（`commit_preset_draft` 成功后 `clear_draft`）——
           * 后端没错、不改；可这一屏还在开着，用户接着改一个值就打到一条**没有草稿**
           * 的链上：`patch_preset_draft` 报「现在没有正在改的那一份，改不了参数」，
           * 再点保存报「…没得存」（`editingTarget` 没变 ⇒ 重开草稿那个 effect 不会重跑）。
           *
           * 重开的对象 = **刚落盘的那一份**（云端线另存出来的现在就在用户根，
           * 用户线写的还是它自己），于是"保存 → 接着改 → 再保存"是一条不断的链。
           */
          const reopen = () => {
            const next: { fileName: string; origin: ActiveOrigin; path: string | null } = {
              fileName: done.fileName,
              origin: 'local',
              path: done.path,
            }
            setEditingPreset(next)
            /*
             * 存完之后这一份的身份可能换了（云端线另存出来的那一份现在是**用户线**的了）
             * ⇒ 历史账本跟着换到新钥匙上：**先认下新钥匙**，别让"读那一本"的 effect
             * 再去读一次（那会把刚保存的这批记录从界面上冲掉）。
             */
            const nextKey = `local:${next.path ?? next.fileName}`
            lastHistoryKeyRef.current = nextKey
            setHistoryReady({ key: nextKey, writable: true })
            void api.beginPresetEdit(next.fileName, next.origin, next.path ?? undefined).then(
              () => setDraftError(null),
              (err: unknown) => setDraftError(errorText(err)),
            )
            /* 存下了 ⇒ 问一次"要不要回切片器重切" —— 切片器自己不会知道参数变了。
               ★ 只有共用开关开着（`guidesEnabled()`）才问：用户在提示里勾过「不再显示」
               或去设置页关过这一格，就不再打扰（那个开关与校准页共用同一个键）。 */
            if (guidesEnabled()) setResliceAsk(true)
          }

          /*
           * **云端更新的表态在这里落盘**（2026-10-20）。
           *
           * 顺序不能反：先把草稿正文写回（adopt 的值已经在里面了），后端再按"盘上现值"
           * 重写骨架（云端新版的注释 / 新字段跟着进来）、把水位推到当前版并记账。
           * 反过来的话，`apply_preset_param_decisions` 读到的是**旧正文** ——
           * 用户这一屏手改的其它值会被它顶回旧值。
           *
           * 失败要分得开：值已经存好了，只是"云端更新处理过没有"这一笔没落上 ——
           * 如实说，不回滚、不假装成功。表态表**留着**，让用户再点一次保存重试。
           */
          if (decisions.length > 0 && path !== null) {
            setSyncBusy(true)
            void api.applyPresetParamDecisions(path, decisions).then(
              (v) => {
                setSyncBusy(false)
                setSync(v)
                setSyncDecisions({})
                setFileReload((n) => n + 1)
                deliveryMutated()
                reopen()
              },
              (e: unknown) => {
                setSyncBusy(false)
                setSavedTone('warn')
                setSavedNote(
                  msg('params.saved.syncFailed', {
                    file: done.fileName,
                    detail: errorText(e),
                  }),
                )
                reopen()
              },
            )
            return
          }
          reopen()
        },
        (e: unknown) => {
          setSavedTone('bad')
          setSavedNote(msg('params.saved.failed', { detail: errorText(e) }))
        },
      )
    },
    [broadcastDraftNow],
  )

  /*
   * **一次手势的改动整批发进草稿**（字段级 patch，参数页底座 ③，不重生成整份）。
   *
   * ★ 2026-10-21 修的那个 bug：原来这里收单个 key、逐条发 `patch_preset_draft`，
   *   而一次手势可能带多项（拖一下擦料塔 = X + Y）。后端每条命令都是
   *   「读草稿 → 只改自己那一项 → 写回」，两条并发时后写的基于旧正文，
   *   把前一条冲掉 —— 真机上塔拖到 (101, 85)，保存后重进只剩 Y=85、X 还是 20，
   *   而修改历史里两项都记着（内存里对、盘上丢一半）。
   *   现在一次手势 = 一条 `patch_preset_draft_batch` = 一次写盘。
   *
   * 不做防抖：参数页的控件是步进器 / 下拉 / 开关（一次点击 = 一次确定的值），
   * 不像文本编辑器要按字节流。
   * 失败只记进 `draftError`（页面显示一行），**不回滚界面值** —— 用户看到的仍是他刚改的；
   * "真机上草稿没写进去"这件事要看得见，而不是悄悄弹回去。
   *
   * 定义在这里（而不是挨着它用的那几个回调）是为了让 `apply` 能引用到它：
   * `useCallback` 依赖的是**运行时**的绑定，而下面的 `apply` 在渲染期就建好了。
   */
  const patchDraft = useCallback(
    (patch: Record<string, string>) => {
      const edits = Object.entries(patch).map(([paramKey, value]) => ({ paramKey, value }))
      if (edits.length === 0) return
      void api.patchPresetDraftBatch(edits).then(
        () => {
          setDraftError(null)
          /* 草稿真变过了 ⇒ 广播一声，预设列表行上那枚橙点当场亮（见 `broadcastDraftSoon`） */
          broadcastDraftSoon()
        },
        (e: unknown) => {
          setDraftError(errorText(e))
          /* 失败也发（`draftState` 头注的规矩：上一次成功写的值还挂着，重读一次没有代价） */
          broadcastDraftSoon()
        },
      )
    },
    [broadcastDraftSoon],
  )

  /*
   * 这一页的**唯一数据源 = catalog**（`<appDataDir>/catalog.json`，随安装包释放；
   * 旧世界那格 localStorage 说明书已随 C4 退役）。页面上每一个字（页签名、分组名、
   * 条数、类型、选项、条件、值）都只有一个出处：
   *
   *   机型与版本        api.getMachines()（catalog 兜底）
   *   字段定义 / 条件    api.getParamMeta()（catalog 的 definition）
   *   页签与分组        api.getRuntimeCatalog() 的 registry（中文名与顺序的唯一权威）
   *   值                api.getMachineParams(m, v)（三层取值，按 combo 拉）
   *   MKP 文件名        catalog 的 files 域
   *
   * 读不到目录就**不编一份假的**，直说数据源在哪。
   */
  /*
   * 本机预设那一格**跟着投递面代次重读**（2026-10-14 补）。
   *
   * 为什么不并进上面那个装载 effect：那一份重跑要重算 combo / defs / 拉网络；
   * 而"我盘上多了份文件"只该重读这一列。两个 effect、各自一条依赖，各读各的。
   */
  useEffect(() => {
    let alive = true
    void api.getLocalPresetFiles().then(
      (list) => {
        if (alive) setLocalPresets(list)
      },
      () => {
        /* 拉不到就空着（抽屉说"本机还没有"）—— 不编，也不挡页面 */
      },
    )
    return () => {
      alive = false
    }
  }, [deliveryRevision])

  useEffect(() => {
    let alive = true
    void (async () => {
      const [list, meta, world] = await Promise.all([
        api.getMachines(),
        api.getParamMeta(),
        api.getRuntimeCatalog().catch(() => null),
      ])
      if (!alive) return
      if (world === null) {
        setError(msg('params.error.noCatalog'))
        return
      }
      const metaByKey = new Map<string, ParamMeta>(meta.map((m) => [m.key, m]))
      /* 字段清单的权威（definition 通道）—— 弃用字段也在这里，参数页靠它列全 */
      const registryParams = new Map<string, CatalogParamDef>(
        world.registry.params.map((p) => [p.key, p]),
      )
      /* 打印板按 id 索引（塔地图按机型 defaultPlateId 查） */
      const platesById = new Map<string, Plate>((world.plates ?? []).map((p) => [p.id, p]))
      /* MKP 文件名按「机型:版本」建索引 —— 目录里登记的交付文件就是那一份 */
      const fileByCombo = new Map<string, string>()
      for (const f of world.files) {
        if (f.kind === 'mkp_preset') fileByCombo.set(`${f.machineId}:${f.versionId}`, f.fileName)
      }
      /*
       * 可编辑的文件身份（参数页底座 ③）：云端线从目录的 files 摊；用户线从
       * `getLocalPresetFiles` 来 —— **同一个 combo 用户线优先**（用户自己那份就是他实际在用的）。
       * 用户文件没有机型/版本元数据，靠 `sourceOf` 的血统 / 文件名反推（与预设页同一套）。
       */
      const editTargetByCombo = new Map<
        string,
        { fileName: string; origin: ActiveOrigin; path: string | null }
      >()
      for (const f of world.files) {
        if (f.kind === 'mkp_preset') {
          editTargetByCombo.set(`${f.machineId}:${f.versionId}`, {
            fileName: f.fileName,
            origin: 'preset',
            path: null,
          })
        }
      }
      const localPresets = await api.getLocalPresetFiles().catch(() => [])
      if (!alive) return
      setLocalPresets(localPresets)
      for (const f of localPresets) {
        /*
         * 我那份的机型/版本从**血统**认（`basedOn*` 指向云端来源那一版）；认不出就跳过 ——
         * 认不出的那份在预设页照常列，只是不对应任何 combo，所以不进这张表（别硬塞一格）。
         */
        const { basedOnMachineId: mid, basedOnVersionId: vid } = f
        if (mid !== null && vid !== null && f.kind === 'mkp_preset') {
          editTargetByCombo.set(`${mid}:${vid}`, {
            fileName: f.fileName,
            origin: 'local',
            path: f.path,
          })
        }
      }
      setCatalog({
        machines: list,
        metaByKey,
        registryParams,
        platesById,
        tabs: tabsOf(world.registry),
        fileByCombo,
        editTargetByCombo,
      })

      /*
       * 默认落在**正在用的那一份**：AppState 的 activePreset 格说应用了哪台哪个版本，
       * 就从目录里找它；找不到（没应用过 / 目录里没这台）才退回第一台 ——
       * 与预设页「默认落在已应用那台」同一个理由。
       * 原来写死"第一台"，应用了 P1S 再进这一页还是 A1（作者：「怎么一直是 a1」）。
       * 快照走唯一客户端的一次读取（`activePresetSnapshot`），不在这里单独读底账。
       */
      /* 钉住模式：目标由调用方点名，别让"正在用的那一份"把它顶掉 */
      if (pinnedAtMount) return
      const live = await activePresetSnapshot()
      if (!alive) return
      const liveMachine = list.find((m) => m.id === live?.machineId)
      const liveVersion = liveMachine?.versions.find((v) => v.id === live?.versionId)
      const home = liveMachine ?? list[0]
      const homeVersion = liveVersion ?? home?.versions[0]
      if (home !== undefined && homeVersion !== undefined) {
        setPick({ machineId: home.id, versionId: homeVersion.id })
      }
    })().catch((e: unknown) => {
      if (alive) setError(errorText(e))
    })
    return () => {
      alive = false
    }
  }, [pinnedAtMount, tr])

  /*
   * 换组合就**重拉**布局、参数、文件清单 —— 不吃缓存。
   *
   * 条数与可见性都跟着机型变（8 个字段带 machineFilter、10 个是 machine_specific），
   * 只换值不换结构就会出现「计数还是上一台机型的」那种假信息。
   */
  useEffect(() => {
    if (catalog === null || pick === null) return
    let alive = true
    setCombo(null)

    const { machineId, versionId } = pick
    /*
     * 布局、字段、值全部从 catalog 的命令摊出来。
     * 文件名从目录的 files 域来：早先那一版把 files 写死成 null（理由是「那一栏一处都不读」），
     * 结果 pill / 底栏 / 保存确认 / 历史标题 / 「已保存到 X」五处全都读不到文件名，
     * 只剩「机型 · 版本」可显示 —— 而目录本来就登记着每个版本的交付文件。
     */
    const data =
      catalog === null
        ? null
        : api
            .getMachineParams(machineId, versionId)
            .then(
              (params): { tabs: ParamTab[]; params: RecipeParam[]; fileLabel: string | null } => ({
                tabs: catalog.tabs,
                params,
                /*
                 * A3：标签跟着**编辑目标**走（同一张表，同 combo 本地文件优先）——
                 * 编辑的是「本地文件」时，pill / 确认框 / 历史标题都说本地文件名，
                 * 不再说目录里那份云端底稿（那是 fileByCombo 的答案，两件事）。
                 */
                fileLabel:
                  catalog.editTargetByCombo.get(`${machineId}:${versionId}`)?.fileName ??
                  catalog.fileByCombo.get(`${machineId}:${versionId}`) ??
                  null,
              }),
            )
    Promise.resolve(data)
      .then((res) => {
        if (!alive || res === null) return
        const { tabs, params, fileLabel } = res

        const placeByKey = new Map<string, Place>()
        /* 全局唯一的 section 序号：同一个 index 在两个分类里各有一个会让 defs 的顺序交错 */
        const sectionOrder = new Map<string, number>()
        let sectionAt = 0
        tabs.forEach((tab) => {
          tab.sections.forEach((section) => {
            sectionOrder.set(section.id, sectionAt)
            sectionAt += 1
          })
        })

        const defs: ParamDef[] = []
        const defByKey = new Map<string, ParamDef>()
        const baseByKey = new Map<string, string>()
        const factoryByKey = new Map<string, string>()

        /* 把某个 key 登记进分类 / 分组索引 */
        const place = (key: string, sectionId: string): void => {
          const tab = tabs.find((t) => t.sections.some((sec) => sec.id === sectionId))
          const section = tab?.sections.find((sec) => sec.id === sectionId)
          if (tab !== undefined && section !== undefined) {
            placeByKey.set(key, {
              tabId: tab.id,
              tabLabel: tab.label,
              sectionId: section.id,
              sectionLabel: section.label,
            })
          }
        }

        /*
         * **字段清单以 definition 通道为准**（2026-10-02，作者裁决）。
         *
         * 配方通道（`getMachineParams`）按既有语义**排除**弃用字段、也不下发它们的值 ——
         * 这一点**不改**（显示 ≠ 会进入新产物）。但参数页要列全：弃用不是删除，值还在、
         * 读得到，「藏起来的后果是用户以为这个参数不存在，然后去别处找」。
         *
         * 于是这一轮遍历 `catalog.registry.params`（定义侧）来建 defs：
         *   · 属于本机型的（`machineFilter` 空 = 不限；非空且不含本机 → 不列）
         *   · 若配方通道给了这条（活字段），label / desc / 值以配方为准（那是这个 combo 的实况）
         *   · 若配方通道**没有**这条（弃用字段被排除），label / desc / 选项 / 控制形态
         *     从定义侧拿，值本体拿不到 —— `baseByKey` 不填，展示值走 `savedValueOf` 兜底
         *     （草稿 TOML 正文里若有就读到，没有就空），**不伪造一个数**。
         *
         * 判据：**参数定义存在 / 参数页显示 / 用户编辑禁止 / 新 TOML 不生成** —— 四件事分开。
         */
        const recipeByKey = new Map(params.map((p) => [p.key, p]))
        const defOfKeys: string[] = []
        for (const def of catalog.registryParams.values()) {
          /* 机型过滤：定义侧限定了机型、且不含本机 → 这台机型根本不认识它，不列 */
          if (def.machineFilter !== undefined && def.machineFilter.length > 0) {
            if (!def.machineFilter.includes(machineId)) continue
          }
          defOfKeys.push(def.key)
        }

        for (const key of defOfKeys) {
          const rd = catalog.registryParams.get(key)
          if (rd === undefined) continue
          const p = recipeByKey.get(key)
          const meta = catalog.metaByKey.get(key)
          const sectionId = meta?.sectionId ?? rd.layout.sectionId
          /*
           * 活字段（配方带了）用配方的显示信息与值；弃用字段（配方没带）退到定义侧。
           * `unit` 一律过 `unitOf`（风扇速度那条把 min/max 塞进了 unit 位）。
           */
          /* 两册都留：显示哪一册由 `localizeDef` 在读的时候决定（见那里的注释） */
          const labelEn = p?.labelEn ?? rd.labelEn
          const descEn = p?.descEn ?? rd.descEn
          const label = p?.label ?? rd.label
          const desc = p?.desc ?? rd.desc
          /*
           * 弃用**按这台机型**判：定义侧可以只点名一部分机型（`deprecatedMachines`），
           * 那种情况在这台机器上照旧能改。全局 `deprecated` 仍然压过一切。
           */
          const isDeprecated =
            p === undefined ||
            p.deprecated === true ||
            meta?.deprecated === true ||
            rd.deprecatedMachines?.includes(machineId) === true
          const def: ParamDef = {
            key,
            label,
            labelEn,
            desc: desc,
            descEn,
            group: p?.group ?? rd.layout.sectionId,
            sectionId,
            unit: unitOf(p?.unit ?? rd.unit),
            meta,
            deprecated: isDeprecated,
            field: {
              key,
              label,
              desc,
              control: controlOf(p?.control ?? controlFromUi(rd.uiComponent), meta),
              unit: unitOf(p?.unit ?? rd.unit),
              min: p?.min ?? rd.min,
              max: p?.max ?? rd.max,
              step: p?.step ?? rd.step,
              /*
               * 选项级弃用映射到共用件的 `FieldOption.deprecated`（划线）。**不新造第二个信号**
               * （FieldOption.deprecated 早就在，SegmentedField / SelectField 已经在画）。
               */
              choices: (p?.choices ?? rd.choices)?.map((c) => ({
                value: c.value,
                label: c.label,
                labelEn: c.labelEn,
                /* 全局弃用 ∪ 点名这台机型 —— 与后端 `Choice::deprecated_for` 同一条 */
                deprecated:
                  c.deprecated === true || c.deprecatedMachines?.includes(machineId) === true
                    ? true
                    : undefined,
              })),
            },
          }
          defs.push(def)
          defByKey.set(key, def)
          /* 值本体只对活字段有（弃用字段配方通道不下发 → 不填，展示走兜底） */
          if (p !== undefined) {
            baseByKey.set(key, normalizeRaw(p.value, meta?.valueType))
            /* 出厂值：被版本盖过的那几条带 baseValue，没盖过的 value 本身就是出厂值 */
            factoryByKey.set(
              key,
              normalizeRaw(p.origin === 'variant' ? p.baseValue : p.value, meta?.valueType),
            )
          }
          place(key, sectionId)
        }

        /* 组内顺序读注册表的 order（可以是小数），没有就保持后端给的顺序 */
        defs.sort((a, b) => {
          const sa = sectionOrder.get(a.sectionId) ?? 0
          const sb = sectionOrder.get(b.sectionId) ?? 0
          if (sa !== sb) return sa - sb
          return (a.meta?.order ?? 0) - (b.meta?.order ?? 0)
        })

        setCombo({
          machineId,
          versionId,
          tabs,
          defs,
          defByKey,
          placeByKey,
          baseByKey,
          factoryByKey,
          fileLabel,
        })
      })
      .catch((e: unknown) => {
        if (!alive) return
        setError(errorText(e))
      })

    return () => {
      alive = false
    }
  }, [catalog, pick])

  const machineId = combo?.machineId ?? pick?.machineId ?? ''
  const versionId = combo?.versionId ?? pick?.versionId ?? ''

  /* 读定义的两个出口都按当前语言摊（页面与卡片只认这两个，不碰 `combo.defByKey`） */
  const defOf = useCallback(
    (key: string) => {
      const d = combo?.defByKey.get(key)
      return d === undefined ? undefined : localizeDef(d, locale)
    },
    [combo, locale],
  )
  /*
   * 分类 / 分组名也按当前语言摊开（id 一个字不动，只换给眼睛看的那两格）。
   *
   * `placeByKey` 里存的是注册表原文（中文），而历史条目会把这两个名字**快照**进账本 ——
   * 照原样取，英文界面上「偏移 / 空间偏移 · Z 轴偏移」就整片中文。历史行的显示走
   * [`historyByBatch`] 的回落逻辑：这一项还在当前定义里就用这里的本语言名，
   * 不在了才用快照（那时快照是唯一说得清"当时改的是哪一项"的东西）。
   */
  const placeOf = useCallback(
    (key: string) => {
      const p = combo?.placeByKey.get(key)
      if (p === undefined) return undefined
      const tab = combo?.tabs.find((t) => t.id === p.tabId)
      if (tab === undefined) return p
      const localized = localizeTab(tab, locale)
      const section = localized.sections.find((s) => s.id === p.sectionId)
      return {
        ...p,
        tabLabel: localized.label,
        sectionLabel: section?.label ?? p.sectionLabel,
      }
    },
    [combo, locale],
  )

  /**
   * **这一轮编辑的是哪一份**（参数页底座 ③）。
   *
   * 默认由"当前组合对应哪一份"决定（`catalog.editTargetByCombo`：同一组合用户线优先）；
   * 钉住模式（右键某一行的参数模态框）由调用方点名，**不看**那张表 ——
   * 用户点的是哪一份就编哪一份，与「当前应用」、与 combo 都不再有关系。
   */
  const editingTarget = useMemo(() => {
    if (pinnedPath !== null && pinnedFileName !== null) {
      return { fileName: pinnedFileName, origin: 'local' as ActiveOrigin, path: pinnedPath }
    }
    if (catalog === null || machineId === '' || versionId === '') return null
    return catalog.editTargetByCombo.get(`${machineId}:${versionId}`) ?? null
  }, [catalog, machineId, pinnedFileName, pinnedPath, versionId])

  /** 这一轮编辑的对象是**本地那份**（有路径）—— 云端线 / 认不出则不是 */
  const editingLocal = editingTarget !== null && editingTarget.origin === 'local'
  const localPath = editingLocal ? (editingTarget?.path ?? null) : null

  /**
   * 已保存那一层的取值顺序：
   *   ① 这一屏保存过的（`saved`，按「机型:版本:字段」记）
   *   ② **我那份文件里真实写着的**（编辑对象是「本地预设」时才有这一档）
   *   ③ 目录里的配方值（`getMachineParams`）—— 云端线 / 文件里没写这一项时的底
   *
   * ② 不能省：用户改过之后，配方给的就不再是"本地值"了 —— 少了它，
   * 参数行会把云端默认显示成"我已保存的"，而「本地修改」那个徽章也就没了依据。
   */
  const savedValueOf = useCallback(
    (key: string) =>
      saved.get(cellKey(machineId, versionId, key)) ??
      (editingLocal ? fileValues.get(key) : undefined) ??
      combo?.baseByKey.get(key) ??
      '',
    [combo, editingLocal, fileValues, machineId, saved, versionId],
  )

  const valueOf = useCallback(
    (key: string) => draft[key] ?? savedValueOf(key),
    [draft, savedValueOf],
  )

  const factoryOf = useCallback((key: string) => combo?.factoryByKey.get(key) ?? '', [combo])

  const dirtyOf = useCallback((key: string) => draft[key] !== undefined, [draft])

  /** 顺着 showWhen.key 往上找第一个不成立的条件。带 seen 防环（注册表写错也不死循环） */
  const findBlocked = useCallback(
    (key: string): BlockedBy | null => {
      if (catalog === null) return null
      const seen = new Set<string>()
      let at = key
      while (!seen.has(at)) {
        seen.add(at)
        const cond = catalog.metaByKey.get(at)?.showWhen
        if (cond === undefined) return null
        const parent = defOf(cond.key)
        /*
         * 父字段不在这台机型的字段表里（被 machineFilter 排除了）= 这条件在这台机器上无从满足，
         * 也无从开启。那就不算「被关着」—— 否则界面会写「需先开启 某个本机型根本没有的项」。
         */
        if (parent === undefined) return null
        const expected = normalizeRaw(cond.value, parent.meta?.valueType)
        if (!testCondition(cond.op, valueOf(cond.key), expected)) {
          return {
            key: cond.key,
            label: parent?.label ?? cond.key,
            need: `${tr(OP_KEYS[cond.op])} ${valueText(parent, expected, tr)}`,
          }
        }
        at = cond.key
      }
      return null
    },
    [catalog, defOf, valueOf, tr],
  )

  const isVisible = useCallback((key: string) => findBlocked(key) === null, [findBlocked])

  /**
   * 一个条件**现在满不满足** —— ParamCard 的分支摊开 / 折叠判据（A43 移植）。
   *
   * 与 `findBlocked` 同一跳：条件参数被 `machineFilter` 排除（这台机型没有它）时返回
   * **true**（不适用 = 不算被关着），否则照 `testCondition` 真值判。
   */
  const condOn = useCallback(
    (cond: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }): boolean => {
      const parent = defOf(cond.key)
      if (parent === undefined) return true
      const expected = normalizeRaw(cond.value, parent.meta?.valueType)
      return testCondition(cond.op, valueOf(cond.key), expected)
    },
    [defOf, valueOf],
  )

  /**
   * 当前机型的**默认打印板**（塔地图选板用，2026-10-02）。
   *
   * 按 `machine.defaultPlateId` 从 catalog 的 plates 里查；没配板 / 查不到就 `undefined`
   * （塔地图那一层不出，画布退回圆角矩形）—— **不保留「按机型 id 兜底」那条老路**
   * （无板就是无板）。
   */
  const plateOf = useCallback((): Plate | undefined => {
    if (catalog === null) return undefined
    const m = catalog.machines.find((x) => x.id === machineId)
    const id = m?.defaultPlateId ?? null
    if (id === null) return undefined
    return catalog.platesById.get(id)
  }, [catalog, machineId])

  /**
   * 唯一写入口。
   *
   * 「改成与已保存值相同」= 撤掉草稿，而不是记一条「改成一样」的假改动；
   * 值没变的项不占一格撤销。
   *
   * ★ 云端更新的「用新值」走的也是这里（2026-10-20）：它只是一次普通的批量改值，
   *   于是撤销 / 重做 / 放弃并退出全都天然成立 —— 不再另开一条绕过草稿的写盘路。
   *   被改掉的那几项顺手撤掉「保持原样」的标记（值已经动了，那条待落决定就不再成立）。
   */
  const apply = useCallback(
    (label: string, patches: Patch[], opts: ApplyOptions = {}): ApplyResult => {
      /*
       * **写值闸**（2026-10-02，作者裁决）：弃用字段 / 弃用选项不许写新值。
       *
       * 闸设在**唯一出口**而不是各控件 —— 以后新增入口（折叠条切换、地图、预设、批量）
       * 不必各写各的规矩。两个口子：
       *
       *   · 目标值**就是已保存值**时放行 —— 老配方里本来就写着「护套」的那份文件，
       *     还原 chip 要能退回去（值还在、读得到；拦它等于把用户锁在自己的文件外）。
       *   · 整个动作**原子拒绝**，不做「弃用的那几条跳过、其余照写」的半套 ——
       *     半套会让一次点击产生看不懂的半截效果。
       */
      const dep = patches.find(
        (p) =>
          p.value !== undefined &&
          p.value !== savedValueOf(p.key) &&
          (defOf(p.key)?.deprecated === true ||
            defOf(p.key)?.field.choices?.some((c) => c.deprecated === true && c.value === p.value)),
      )
      if (dep !== undefined) {
        const def = defOf(dep.key)
        const choice = def?.field.choices?.find(
          (c) => c.deprecated === true && c.value === dep.value,
        )
        setGateNote(
          choice !== undefined
            ? msg('params.gate.deprecatedChoice', {
              label: def?.label ?? dep.key,
              choice: choice.label,
            })
            : msg('params.gate.deprecated', { label: def?.label ?? dep.key }),
        )
        return 'blocked'
      }

      const patch: Record<string, string | undefined> = {}
      const before: Record<string, string | undefined> = {}
      const items: HistoryItem[] = []

      for (const p of patches) {
        const savedValue = savedValueOf(p.key)
        /*
         * **浮点尾闸**（2026-10-21，作者：「文件里面记录的就是两位小数的才对」）：
         * 任何来路的值（步进器算数 / 云端 JSON 的 `3.8000000000000003` / 旧账原文）
         * 进草稿、进历史之前先收一次 IEEE754 的尾巴（修尾**不补零** ——
         * `3.8` 不许变成 `3.80`，那会无谓制造一处"看起来改过了"）。
         * 后端 `patch_preset_toml` 的写盘出口还有同一道闸兜底。
         */
        const wanted = p.value === undefined ? undefined : formatNum(p.value, null)
        const target = wanted !== undefined && wanted === savedValue ? undefined : wanted
        const current = draft[p.key]
        if (current === target) continue
        patch[p.key] = target
        before[p.key] = current
        /*
         * 上下文**当场快照**（操作记录底座 ④）：参数名 / 分类 / 分组 / 单位。
         * 之后切机型、目录更新都不改写这一条 —— 它记的是"当时我改的是哪一项、它叫什么"。
         */
        const def = defOf(p.key)
        const place = placeOf(p.key)
        items.push({
          key: p.key,
          from: formatNum(current ?? savedValue, null),
          to: formatNum(target ?? savedValue, null),
          label: def?.label ?? p.key,
          tab: place?.tabLabel ?? '',
          section: place?.sectionLabel ?? def?.group ?? '',
          unit: def?.unit ?? null,
        })
      }

      /*
       * 这一步之后的**云端表态表**：改过值的项先撤掉表态（值都动了，那条判断不再成立），
       * 再按 `opts.sync` 写回这一批明确交代的项 —— 采用即使 no-op（值本来就等于云端新值）
       * 也要记下来，保持则根本不会出现在 `patch` 里。
       */
      const nextSync: Record<string, SyncDraftKind> = { ...syncDecisions }
      for (const key of Object.keys(patch)) delete nextSync[key]
      for (const [key, kind] of Object.entries(opts.sync ?? {})) {
        if (kind === null) delete nextSync[key]
        else nextSync[key] = kind
      }

      /* 逐项都等于现值 ⇒ 没有值要写（历史「还原」最常见的一下）：如实回答，不装作写了。
         但表态表可能变了（点「保持原样」/ 采用一个相同的值）—— 那一格照写，不吞掉。 */
      if (items.length === 0) {
        if (opts.sync !== undefined) setSyncDecisions(nextSync)
        return 'noop'
      }

      const id = seq + 1
      const next = { ...draft }
      for (const [key, value] of Object.entries(patch)) {
        if (value === undefined) delete next[key]
        else next[key] = value
      }

      setSeq(id)
      setDraft(next)
      setSyncDecisions(nextSync)
      setPast([
        ...past,
        { patch, before, logId: id, syncBefore: syncDecisions, syncAfter: nextSync },
      ])
      setFuture([]) // 新动作把「重做」那一支剪掉，撤销栈的通例
      setLog([
        ...log,
        {
          id,
          no: id,
          action: label,
          items,
          state: 'draft',
          batch: null,
          tsUnix: nowSec(),
          ...(opts.fromSync === true ? { fromSync: true } : {}),
        },
      ])
      setSavedNote(null)
      /* 这次写成功 → 上一次的拦截提示收掉 */
      setGateNote(null)

      /*
       * **落进草稿 TOML**（参数页底座 ③）：这一屏的内存改动同步写进 `run/draft-preset.json`
       * 的正文，于是切页 / 关掉软件再回来都还在。`value === undefined`（撤回保存值）
       * 用「已保存值」回写 —— 草稿里该是这个值，不是"没有这一项"。
       *
       * 一次 `apply` 可能带多项（如"恢复默认值"几十项）→ **一条批量命令一次落地**。
       * 后端只改那几处、不重生成整份；分成 N 条发会互相冲掉（见 `patchDraft` 头注里那个 bug）。
       */
    const toDraft: Record<string, string> = {}
    for (const [key, value] of Object.entries(patch)) {
      toDraft[key] = value ?? savedValueOf(key)
    }
    patchDraft(toDraft)
    return 'applied'
    },
    [defOf, draft, log, past, patchDraft, placeOf, savedValueOf, seq, syncDecisions],
  )

  const edit = useCallback(
    (key: string, value: string) => {
      apply(tr('params.act.change', { label: defOf(key)?.label ?? key }), [{ key, value }])
    },
    [apply, defOf, tr],
  )

  /** 行上那个 chip：退回已保存值 */
  const revertToSaved = useCallback(
    (key: string) => {
      apply(tr('params.act.revert', { label: defOf(key)?.label ?? key }), [
        { key, value: savedValueOf(key) },
      ])
    },
    [apply, defOf, savedValueOf, tr],
  )

  /**
   * 整份恢复默认值：一次动作，撤销一下就全回来。
   *
   * **退回目录里那份出厂值**（2026-10-08 资源库改判）：那套"退回这份预设当初那版交付预设的
   * 默认值"整个退场了 —— 有问题就让用户删掉、重新取一份，参数页不再养一份历史快照，
   * 也就没有"基准里缺哪一项"那一串回落。
   */
  const restoreDefaults = useCallback(() => {
    if (combo === null) return
    apply(
      tr('params.act.restoreDefaults'),
      combo.defs.map((d) => ({
        key: d.key,
        value: combo.factoryByKey.get(d.key) ?? '',
      })),
    )
  }, [apply, combo, tr])

  /* 撤销 / 重做在**回调体里**读栈算完再 set —— StrictMode 会把 updater 跑两次 */
  const undo = useCallback(() => {
    const step = past[past.length - 1]
    if (step === undefined) return
    const next = { ...draft }
    for (const [key, value] of Object.entries(step.before)) {
      if (value === undefined) delete next[key]
      else next[key] = value
    }
    setDraft(next)
    /* 云端表态跟着退回去：采用撤掉了，那一项就该回到"待选择"，而不是还挂着「已采用」 */
    setSyncDecisions(step.syncBefore ?? {})
    setPast(past.slice(0, -1))
    setFuture([...future, step])
    setLog(log.map((e) => (e.id === step.logId ? { ...e, state: 'undone' } : e)))
    setSavedNote(null)
    /* 撤销也要落进草稿 TOML（否则关掉再回来，"撤了的那一步"又回来了）—— 一次批量 */
    const toDraft: Record<string, string> = {}
    for (const [key, value] of Object.entries(step.before)) {
      toDraft[key] = value ?? savedValueOf(key)
    }
    patchDraft(toDraft)
  }, [draft, future, log, past, patchDraft, savedValueOf])

  const redo = useCallback(() => {
    const step = future[future.length - 1]
    if (step === undefined) return
    const next = { ...draft }
    for (const [key, value] of Object.entries(step.patch)) {
      if (value === undefined) delete next[key]
      else next[key] = value
    }
    setDraft(next)
    setSyncDecisions(step.syncAfter ?? {})
    setFuture(future.slice(0, -1))
    setPast([...past, step])
    setLog(log.map((e) => (e.id === step.logId ? { ...e, state: 'draft' } : e)))
    setSavedNote(null)
    /* 重做同样要落进草稿 TOML —— 同样一次批量 */
    const toDraft: Record<string, string> = {}
    for (const [key, value] of Object.entries(step.patch)) {
      toDraft[key] = value ?? savedValueOf(key)
    }
    patchDraft(toDraft)
  }, [draft, future, log, past, patchDraft, savedValueOf])

  const at = `${machineId}:${versionId}`
  const savedBatches = batchesByCombo.get(at) ?? 0

  /* --------------------------------------------------------------------------
   * 修改历史**落在盘上**（2026-10-17 作者口径：「类似 git 的保存那种形式，不要清空，
   * 每次重新打开软件都存在」）。
   *
   * 一本账认一份文件：钥匙 = 哪条线 + 身份（用户线按相对路径、云端线按文件名）——
   * 于是**导入的 toml 也有自己的一本**（它进来之后就是用户线的一份文件），
   * 而用户目录里两份同名文件不会串账。
   * ------------------------------------------------------------------------ */
  const historyOrigin: ActiveOrigin | null = editingPreset?.origin ?? null
  const historyId: string | null =
    editingPreset === null
      ? null
      : editingPreset.origin === 'local'
        ? (editingPreset.path ?? editingPreset.fileName)
        : editingPreset.fileName
  const historyKeyText: string | null =
    historyOrigin === null || historyId === null ? null : `${historyOrigin}:${historyId}`

  /** 当前这一份文件的「上一次保存之前的值」表（参数 key → 保存前的已保存值） */
  const prevSaved: Record<string, string> = useMemo(
    () => (historyKeyText === null ? {} : (prevSavedByFile[historyKeyText] ?? {})),
    [historyKeyText, prevSavedByFile],
  )

  /** 这一项上一次保存之前是什么（没记录过 = null，比如从来没保存过它的改动） */
  const prevSavedOf = useCallback(
    (key: string): string | null => prevSaved[key] ?? null,
    [prevSaved],
  )

  /** 读完了哪一本（`writable: false` = 读不出来 ⇒ **一个字都不许写回去**，免得把盘上那本冲了） */
  const [historyReady, setHistoryReady] = useState<{ key: string; writable: boolean } | null>(null)
  /** 上一次读的是哪一本（同 key 不重读 —— 重读会把刚改的冲掉） */
  const lastHistoryKeyRef = useRef<string | null>(null)

  /* 读：换了一份文件（或第一次打开这一份）就读它那一本 */
  useEffect(() => {
    if (historyOrigin === null || historyId === null || historyKeyText === null) return
    if (lastHistoryKeyRef.current === historyKeyText) return
    lastHistoryKeyRef.current = historyKeyText
    let alive = true
    void api
      .getPresetHistory(historyOrigin, historyId)
      .then((book) => {
        if (!alive) return
        setLog(
          book.entries.map((e) => ({
            id: e.id,
            no: e.no,
            action: e.action,
            state: e.state,
            batch: e.batch,
            tsUnix: e.tsUnix,
            ...(e.fromSync === true ? { fromSync: true } : {}),
            items: e.items.map((i) => ({
              key: i.key,
              /* 读回也过一次修尾闸：旧账本里已经有 `3.8000000000000003` 那种值（2026-10-21），
                 显示、历史「还原」、下次整本写回都从这儿干净 */
              from: formatNum(i.from, null),
              to: formatNum(i.to, null),
              label: i.label,
              tab: i.tab,
              section: i.section,
              unit: i.unit,
            })),
          })),
        )
        setPrevSavedByFile((p) => ({ ...p, [historyKeyText]: book.prevSaved ?? {} }))
        setSeq(book.entries.reduce((n, e) => Math.max(n, e.id), 0))
        setBatchesByCombo((prev) => new Map(prev).set(at, book.batches))
        setHistoryReady({ key: historyKeyText, writable: true })
      })
      .catch(() => {
        /* 读不出来 = 不写（静默）：界面照样能改值，只是这一本暂时不落到盘上 */
        if (alive) setHistoryReady({ key: historyKeyText, writable: false })
      })
    return () => {
      alive = false
    }
  }, [at, historyId, historyKeyText, historyOrigin])

  /* 写：改一条 / 撤销一条 / 保存一次之后整本写回（600ms 合批，连着敲几下只写一次） */
  useEffect(() => {
    if (historyOrigin === null || historyId === null || historyKeyText === null) return
    if (historyReady === null || historyReady.key !== historyKeyText || !historyReady.writable) return
    const timer = window.setTimeout(() => {
      void api
        .putPresetHistory(historyOrigin, historyId, {
          batches: savedBatches,
          prevSaved,
          entries: log.map((e) => ({
            id: e.id,
            no: e.no,
            action: e.action,
            state: e.state,
            batch: e.batch,
            tsUnix: e.tsUnix ?? nowSec(),
            fromSync: e.fromSync === true,
            items: e.items,
          })),
        })
        .catch(() => {
          /* 写不进去不挡着改值 —— 历史是"记着"，不是"能不能改"的闸 */
        })
    }, 600)
    return () => window.clearTimeout(timer)
  }, [historyId, historyKeyText, historyOrigin, historyReady, log, prevSaved, savedBatches])

  /**
   * 保存：草稿并进已保存层、清撤销栈、日志里那批草稿条目转成「已保存」。
   *
   * 清撤销栈是因为保存是提交点 —— 「保存完再撤销」会撤出一个已保存状态的假草稿。
   * 日志不清：它回答的是「这次打开之后我改过什么」，清掉就成了骗人的表现。
   */
  const save = useCallback(() => {
    if (combo === null) return
    /*
     * **范围闸**（2026-10-20）：超出注册表 min/max 的草稿不许落盘 —— 后端的 patch
     * 不做范围校验，这一道只能设在界面。正常路径到不了这里：保存确认弹层会先列出
     * 超范围项、把「确认保存」灰掉（见 PageParams）；这里是 Ctrl+S / 直调的兜底。
     */
    const badKeys = Object.keys(draft).filter((key) => {
      const def = defOf(key)
      return def !== undefined && invalidNumberText(def, draft[key], tr) !== null
    })
    if (badKeys.length > 0) {
      setSavedNote(
        msg('params.save.invalidKeys', {
          keys: badKeys.map((k) => defOf(k)?.label ?? k).join(tr('params.listSep')),
        }),
      )
      return
    }
    /*
     * **这一批云端更新的表态**（2026-10-20）：只交还"还在待处理、而且表过态"的那几项。
     * 没表态的项不能捎带（那等于替用户做了决定），保存完它仍然挂着。
     *
     * ★ 表态表只活在内存里（关掉这一屏就没了）；草稿却会跟到下一次打开。
     *   所以这里补一条**从草稿反推**：某一项还没表态，可草稿里的值已经就是云端新值
     *   ——那等于"用过了"，只是那一笔表态在关窗时丢了。不补的话值已经改了、
     *   水位却没推进，云端更新会一直挂着，逼用户再点一次「用新值」（还是 noop）。
     *   保持原样推不出来（没有值线索），丢了就丢了 —— 那一版下次再问一遍，不算错。
     */
    const decisions: ParamDecision[] = []
    for (const e of sync?.entries ?? []) {
      if (!e.pending) continue
      const kind =
        syncDecisions[e.key] ??
        (e.cloudNew !== null && draft[e.key] === e.cloudNew ? 'adopt' : undefined)
      if (kind !== undefined) decisions.push({ paramKey: e.key, kind })
    }
    /*
     * **先落盘**：把草稿提交成"本地预设"（云端线另存 / 用户线写回，后端 `commit_preset_draft`）。
     * 本地这一层只是"这一屏看得到的已保存值"，真值在用户根 —— 两句状态由 `commitDraft` 写。
     * 云端表态跟着这一趟走（`commitDraft` 里先提交正文、再落决定）。
     */
    commitDraft(decisions, localPath)
    const nextSaved = new Map(saved)
    /*
     * **记下「上一次保存之前的值」**（2026-10-21）：这一批真的动了值的项，
     * 把保存前的已保存值记进账（跟历史账本一起落盘）。保存后行上、
     * 云端对照栏里都还能摆出"我的旧值 → 新值"，不只是还没保存时才看得到。
     */
    const nextPrev: Record<string, string> = { ...prevSaved }
    for (const [key, value] of Object.entries(draft)) {
      nextSaved.set(cellKey(combo.machineId, combo.versionId, key), value)
      const before = savedValueOf(key)
      if (value !== before) nextPrev[key] = formatNum(before, null)
    }
    if (historyKeyText !== null)
      setPrevSavedByFile((p) => ({ ...p, [historyKeyText]: nextPrev }))
    const batch = savedBatches + 1
    setSaved(nextSaved)
    setBatchesByCombo(new Map(batchesByCombo).set(at, batch))
    setDraft({})
    setPast([])
    setFuture([])
    setLog(log.map((e) => (e.state === 'draft' ? { ...e, state: 'saved', batch } : e)))
  }, [
    at,
    batchesByCombo,
    commitDraft,
    combo,
    defOf,
    draft,
    historyKeyText,
    localPath,
    log,
    prevSaved,
    saved,
    savedValueOf,
    savedBatches,
    sync,
    syncDecisions,
    tr,
  ])

  /**
   * 历史面板里的「还原」：把那一条改动的值退回去，本身也算一次新编辑（可以再撤销）。
   *
   * ★ 收整个 `entry` 而不是 id（2026-10-16）：抽屉两屏的行都是聚合出来的
   *   （逐项 = 一项、`from` 是"这一段开始前的"；整批 = 这一段的全部项），
   *   它们都不在 `log` 里、没有自己的 id —— 退货的判据只能是它身上那份 `items`。
   *   整批交进来的是一整段 ⇒ 一次 `apply` 退完、**撤销一次全回来**。
   */
  const revertEntry = useCallback(
    (entry: HistoryEntry): ApplyResult => {
      if (entry.state === 'undone' || entry.items.length === 0) return 'noop'
      /*
       * 「还原」= 把这一段开始前的值**再写一遍**（一次动作、可撤销、进历史）。
       * 返回 `apply` 的三种结果：抽屉拿它回话 —— 命中 `noop` 很常见
       * （这几天你已经手动改回那个值了），那时界面上不会有一丝变化。
       */
      return apply(
        tr('params.act.undo', { action: entry.action }),
        entry.items.map((item) => ({ key: item.key, value: item.from })),
      )
    },
    [apply, tr],
  )

  /*
   * 「在看的是不是已应用的那份」：AppState 的 activePreset 格（唯一底账 × 当前 combo）。
   * 从唯一客户端订阅（`../state/appState.ts`）——应用 / 撤销 / 删除一发生，
   * 这里的 active 同帧换账，不持有本地副本、不按 combo 变化重读
   * （第一轮的本地 state + effect 补丁已拆，见 docs/APP-STATE.md）。
   *
   * 声明位置在 `switchTo` **之前** —— switchTo 的写回拿它当守卫，依赖数组在渲染期
   * 求值，声明挪晚了就是 TDZ 白屏（2026-10-06 真机踩过：Cannot access 'active'
   * before initialization）。
   */
  const active = useActivePreset()

  /** 换组合：草稿、栈、日志一起作废（面板上写的是「这次打开之后」）。已保存的那一层留着 */
  /*
   * 换 combo = 切换 —— **写回只住在这个显式动作处理器里**（抽屉选一份 / 确认切换，
   * 都是用户点出来的，一次动作一次写）。2026-10-06 教训（作者：「左右左右地闪」）：
   * 这条写回曾经是个 **effect**（依赖底账快照、每次底账变都重跑）—— effect 写
   * 自己所依赖的共享状态就是构造反馈环：别处一应用，这里把旧 combo 写回底账、
   * follow 再把 combo 拉向新底账，两个 effect 互相拉扯，1 秒内打出十几次 apply，
   * 底账在两份预设之间来回震荡。**纪律由此钉死：effect 一律只读；写底账只能发生在
   * 显式动作的处理器里**（首页按钮 / 各处抽屉的选择回调同理）。
   * 守则见 activateCombo：底账已命中不动（不顶掉「本地文件」）、应用不了保持原账。
   */
  const switchTo = useCallback(
    (next: { machineId: string; versionId: string }) => {
      setPick(next)
      setDraft({})
      setPast([])
      setFuture([])
      setLog([])
      setSeq(0)
      setSavedNote(null)
      /* 换目标 = 另一份文件，上一份的云端表态留在这边没有意义（还会认错文件） */
      setSyncDecisions({})
      activateCombo(
        next.machineId,
        next.versionId,
        (m, v) => catalog?.fileByCombo.get(`${m}:${v}`) ?? null,
        active,
      )
    },
    [active, catalog],
  )

  const dirtyKeys = useMemo(() => Object.keys(draft), [draft])

  /**
   * 底栏「取消」（2026-10-16，作者：「我的修改参数还缺少取消按钮，风格要一样呀」）：
   * 全部未保存的改动**一次退回已保存值**。`apply` 的既有语义正好接住——
   * 「改成与已保存值相同 = 撤掉草稿」——所以这只是一次普通的批量 apply：
   * 一次撤销能整个回来、草稿 TOML 逐键回写、日志记一条「取消改动」。
   *
   * ★ **内存里一项都没有时兜底**（2026-10-19）：那一颗按"内存里那几项"回写，可冷启动
   *   （关掉软件再打开）时草稿层是空的 —— 盘上那份草稿却可能还是脏的（橙点亮着、
   *   自动进了编辑模式）。这时回写没有对象，旧版直接 `return`，于是「放弃并退出」按下去
   *   什么都不发生，草稿也永远丢不掉。没有可回写的项 ⇒ **直接把盘上那份草稿丢掉**，
   *   那正是这一颗字面上的意思。
   */
  const discardAll = useCallback(() => {
    if (dirtyKeys.length === 0) {
      /* 「放弃并退出」= 这一屏的表态一个都不算数（云端更新仍然挂着） */
      setSyncDecisions({})
      if (!draftOnDisk) return
      void api.discardPresetDraft().then(
        () => {
          setDraftOnDisk(false)
          setSavedTone('ok')
          setSavedNote(msg('params.saved.discarded'))
          broadcastDraftNow()
        },
        (e: unknown) => setDraftError(errorText(e)),
      )
      return
    }
    apply(
      tr('params.act.discardAll'),
      dirtyKeys.map((key) => ({ key, value: savedValueOf(key) })),
    )
    /*
     * 清表态要放在 `apply` **之后**：`apply` 自己会按"这一步之后的表态表"覆盖一次
     * （它是从闭包里那份旧表算的），先清会被它当成没清 —— 只剩"保持原样"那几项会漏回来。
     */
    setSyncDecisions({})
    /* 回写干净了 ⇒ 盘上那份草稿也不再算脏（重开这一屏不该再自动进编辑模式） */
    setDraftOnDisk(false)
    /* 「取消」是"一锤子"：当场广播 —— 全退回已保存值之后那枚橙点该立刻灭 */
    broadcastDraftNow()
  }, [apply, broadcastDraftNow, dirtyKeys, draftOnDisk, savedValueOf, tr])

  const requestCombo = useCallback(
    (nextMachine: string, nextVersion: string) => {
      /* 钉住模式（右键某一份预设）：编辑的是那一份，换目标这件事整个不存在 */
      if (pinned) return
      if (nextMachine === machineId && nextVersion === versionId) return
      /* 有未保存改动就先问一句 —— 切过去草稿就没了，不能静默丢 */
      if (dirtyKeys.length > 0) {
        setPendingCombo({ machineId: nextMachine, versionId: nextVersion })
        return
      }
      switchTo({ machineId: nextMachine, versionId: nextVersion })
    },
    [dirtyKeys.length, machineId, pinned, switchTo, versionId],
  )

  /**
   * **从抽屉里点「使用」换到某一份**（2026-10-14）。
   *
   * 与 `requestCombo` 的区别：那个按 **combo** 换，而抽屉列的是**本机文件** ——
   * 同一 combo 下可能有好几份（下载来一份 + 你复制出来一份），按 combo 那条路
   * 开头就 `nextMachine === machineId && nextVersion === versionId` 退货 ⇒
   * 点另一份没反应（作者 2026-10-14 实测）。
   *
   * 这里做两件事，缺一不可：
   *   ① **换编辑目标**（与 `switchTo` 同一套：清草稿、重置历史 —— 换了基准那些增量没意义），
   *      同一 combo 时**也要**做（这正是"换一份"与"换一版"的差别）；
   *   ② 把它变成「正在使用」（`activateFile`，**按文件**，不再让后端去查目录）。
   *
   * 未保存改动仍然先问一句（`pendingFile` 那条弹层）。
   */
  const requestFile = useCallback(
    (next: { machineId: string; versionId: string; fileName: string; path: string }) => {
      /* 钉住模式（右键某一份预设）：编辑的是那一份，换目标这件事整个不存在 */
      if (pinned) return
      if (dirtyKeys.length > 0) {
        setPendingFile(next)
        return
      }
      switchTo({ machineId: next.machineId, versionId: next.versionId })
      activateFile(next.fileName, next.path)
    },
    [dirtyKeys.length, pinned, switchTo],
  )

  const confirmPending = useCallback(() => {
    if (pendingCombo === null) return
    switchTo(pendingCombo)
    setPendingCombo(null)
  }, [pendingCombo, switchTo])

  const confirmPendingFile = useCallback(() => {
    if (pendingFile === null) return
    const next = pendingFile
    setPendingFile(null)
    switchTo({ machineId: next.machineId, versionId: next.versionId })
    activateFile(next.fileName, next.path)
  }, [pendingFile, switchTo])

  const cancelPending = useCallback(() => {
    setPendingCombo(null)
    setPendingFile(null)
  }, [])

  const dirtyTabs = useMemo(() => {
    const set = new Set<string>()
    for (const key of dirtyKeys) {
      const place = combo?.placeByKey.get(key)
      if (place !== undefined) set.add(place.tabId)
    }
    return set
  }, [combo, dirtyKeys])

  /*
   * 界面语言一变，这两个目录就按新语言摊一次 —— 数据本身（`combo`）一个字不动，
   * 所以机型/版本、草稿、撤销栈全都不受影响。
   */
  const localizedDefs = useMemo(
    () => (combo?.defs ?? []).map((d) => localizeDef(d, locale)),
    [combo, locale],
  )
  const localizedTabs = useMemo(
    () => (combo?.tabs ?? []).map((t) => localizeTab(t, locale)),
    [combo, locale],
  )

  /** 与出厂不同的条数 —— 按当前机型实算，不是写死的 67 */
  const changedCount = useMemo(() => {
    if (combo === null) return 0
    return combo.defs.filter((d) => valueOf(d.key) !== (combo.factoryByKey.get(d.key) ?? '')).length
  }, [combo, valueOf])

  const sectionsOf = useCallback(
    (tabId: string) => localizedTabs.find((t) => t.id === tabId)?.sections ?? [],
    [localizedTabs],
  )

  const defsOfSection = useCallback(
    (sectionId: string) => localizedDefs.filter((d) => d.sectionId === sectionId),
    [localizedDefs],
  )

  const history = useMemo(() => [...log].reverse(), [log])

  /**
   * 抽屉两屏共用的**分段数据**（2026-10-17：逐项 / 整批都读它，只是摆法不同）：
   * 未保存的一段在最上，然后是第 N 次、第 N-1 次…… 保存。
   *
   * ★ 2026-10-16 作者口径（见 `netOf`）：**这一批只看净变化** —— 同一项改了几次
   * 只记头尾（`70 → 75 → 80 → 90` 就是一条 `70 → 90`），改回原值的不出现。
   * 于是每一段里**一项一行**（不再是"一个动作一行"），行上的「还原」退的是
   * 这一项在**这一段开始前**的值。
   *
   * 净变化为空的那一段（这一批的改动全被改回来了）**整段不摆** —— 摆一个
   * 「0 条」的空段只是噪声；批号本身照真号走，不会因为跳过一段就串号。
   */
  const historyByBatch = useMemo<HistoryGroup[]>(() => {
    const groups: HistoryGroup[] = []
    /* 段内序号：跨段连续（与旧版"全局递增"同一种读法，不因分段而重号） */
    let no = 0
    let id = 0
    const push = (batch: number | null, title: string, src: HistoryEntry[]) => {
      /*
       * 快照里的名字是**写那一刻的语言**（旧账里存的就是中文）。历史行照当前语言
       * 重说一遍：`defOf` / `placeOf` 都是摊过的，这一项还在当前定义里就换成本语言名；
       * 不在（换了机型/版本）才回落快照 —— 那时快照是唯一说得清"当时改的是哪一项"的东西。
       */
      const items = netOf(src).map((it): HistoryItem => {
        const def = defOf(it.key)
        if (def === undefined) return it
        const place = placeOf(it.key)
        return {
          ...it,
          label: def.label,
          tab: place?.tabLabel ?? it.tab,
          section: place?.sectionLabel ?? it.section,
          unit: def.unit ?? it.unit,
        }
      })
      if (items.length === 0) return
      /*
       * 段头右侧那一列时间：取这一段里**最后落笔**的 `tsUnix`（保存那一刻就在它附近）。
       * 未保存那一段没有"落盘时刻"可言，给 `null`（界面不摆时间）。
       */
      const atUnix =
        batch === null
          ? null
          : src.reduce<number>((n, e) => Math.max(n, e.tsUnix ?? 0), 0) || null
      groups.push({
        batch,
        title,
        atUnix,
        items: items.map((it) => {
          id -= 1
          no += 1
          return {
            id,
            no,
            action: tr('params.act.change', { label: it.label }),
            items: [it],
            /* 未保存那一段标「未保存」，落过盘的标「已保存」 —— 与旧版同一套词 */
            state: batch === null ? 'draft' : 'saved',
            batch,
            ...(it.fromSync === true ? { fromSync: true } : {}),
          }
        }),
      })
    }
    push(null, tr('params.history.draft'), log.filter((e) => e.batch === null))
    for (let b = savedBatches; b >= 1; b -= 1) {
      push(b, tr('params.history.batchTitle', { n: b }), log.filter((e) => e.batch === b))
    }
    return groups
  }, [defOf, log, placeOf, savedBatches, tr])

  const machine = catalog?.machines.find((m) => m.id === machineId)
  const version = machine?.versions.find((v) => v.id === versionId)
  /* 抽屉那几行要显示版本名（`localPickerRows`），按语言摊开后再给出去 */
  const machinesShown = useMemo(
    () => localizeMachines(catalog?.machines ?? [], locale),
    [catalog, locale],
  )

  const fileOf = useCallback(
    (m: string, v: string) => catalog?.fileByCombo.get(`${m}:${v}`) ?? null,
    [catalog],
  )

  /*
   * 「正在使用」与当前 combo 的关系（只剩 onIt：pill 画绿还是琥珀）。
   * **不再被回页签弹回**（2026-10-06 架构决策）：第一轮的"回页签把 combo 拉回底账"
   * 是同步补丁，它把「正在使用」与「正在编辑」绑死了。现在 active 只喂"正在使用"
   * 的显示，用户自选的 combo（编辑目标）独立存在 —— 离开再回来，
   * 接着编辑的就是自己选的那一份。
   */
  const activeUse = useMemo<ActiveUse | null>(() => {
    if (active === null) return null
    return { onIt: active.machineId === machineId && active.versionId === versionId }
  }, [active, machineId, versionId])

  /*
   * 「应用的状态」跟到参数页（作者 2026-10-06：点了应用，所有其他页就是应用的状态）——
   * **只读的单向跟随**：底账的 combo 值真变了（别处显式应用成功）→ 这边把编辑目标
   * 挪过去。effect 不写任何共享状态（写回只在 `switchTo` 那个显式处理器里，
   * 见那里的教训注释）；底账值没变（只是快照换了新对象）时这里直接返回，
   * 不会拽着用户在别处选的 combo 乱跳。挂载落地的那一次 pick 本来就是
   * 底账那份，自然不动。
   */
  useEffect(() => {
    /* 钉住模式（右键某一份预设进来）：目标由调用方点名，底账怎么变都不该拽着这一屏乱跳 */
    if (pinned) return
    if (active === null || catalog === null) return
    if (active.machineId === machineId && active.versionId === versionId) return
    const machine = catalog.machines.find((m) => m.id === active.machineId)
    const version = machine?.versions.find((v) => v.id === active.versionId)
    if (machine !== undefined && version !== undefined) {
      setPick({ machineId: machine.id, versionId: version.id })
    }
  }, [active, catalog, machineId, pinned, versionId])

  /*
   * 钉住模式：机型 / 版本只用来决定**字段表与分组**（这张卡画哪些参数、怎么分组），
   * 编辑目标另算（见 `editingTarget`）。放在这里而不是初值里：catalog 是异步到的，
   * 初值那一次可能还没有这台机型。
   */
  useEffect(() => {
    if (pinnedMachine === null || pinnedVersion === null) return
    setPick({ machineId: pinnedMachine, versionId: pinnedVersion })
  }, [pinnedMachine, pinnedVersion])

  /*
   * **编辑目标**：当前 combo 对应的那一份文件（参数页底座 ③）。
   *
   * 默认由"正在应用的那一份"决定（`catalog.editTargetByCombo` 里那张表按 combo 查）；
   * 用户切机型/版本 = 换目标，**不动 active-preset** —— 编辑与应用解耦。
   * combo 对应不到任何文件（如 A2L 没配 MKP）就是 `null`：页面照实说"这份没法改"。
   */
  /*
   * combo 一换就**重开草稿**（`beginPresetEdit`）：把那份文件的正文读进草稿链，
   * 于是"改到一半关掉再回来"接着改（后端按"同一份"认草稿，`reused`）。
   *
   * 切 combo 时**先把上一条的未保存改动丢掉** —— 与页面现有"切组合清草稿"同一语义
   * （作者："面板上写的是'这次打开之后'"）。保存过的已经落用户根，不受影响。
   */
  useEffect(() => {
    if (editingTarget === null) {
      setEditingPreset(null)
      setDraftOnDisk(false)
      return
    }
    let alive = true
    setDraftError(null)
    void api
      .beginPresetEdit(editingTarget.fileName, editingTarget.origin, editingTarget.path ?? undefined)
      .then(
        (d) => {
          if (!alive) return
          setEditingPreset({
            fileName: d.sourceFileName,
            origin: d.origin,
            path: d.path,
          })
          const key = paramTargetKey(d.origin, d.path, d.sourceFileName)
          sessionKeyRef.current = key
          /*
           * ★ **会话缓存**（2026-10-19）：`reused` = 后端按"同一份"认到了上次那份草稿。
           * 认到了才把"这一屏的内存状态"接回来 —— 草稿层（改了没保存的那几项）、
           * 撤销 / 重做栈、修改历史。作者：「我没保存的时候关闭它应该是记住着，然后下次
           * 我再进来的时候，它一样还是保留着那个编辑模式的」。
           *
           * 认不到（这趟是新建的 / 已被别处作废，正文链保存会把参数草稿作废）就整条丢掉 ——
           * 与 `PagePresets` 的 `textHistCache`「对不上就丢掉」同一条纪律。
           */
          if (d.reused) {
            /* 同一个目标已经接回来过了（目录刷新让 effect 重跑）：别拿旧快照盖掉这一屏的改动 */
            if (draftRestoredRef.current !== key) {
              const kept = readParamSession(key)
              if (kept !== null) {
                draftRestoredRef.current = key
                setDraft(kept.draft)
                setPast(kept.past)
                setFuture(kept.future)
              } else {
                /*
                 * ★ **冷启动接回草稿层**（2026-10-19 作者实测的那条）。
                 *
                 * 会话缓存是**进程内存**，关掉软件就没了；盘上那份草稿（`run/app-state.json`）
                 * 却还在，摘要也如实说脏 —— 行上那枚橙点亮着。这一屏的 `draft` 从 `{}` 起，
                 * `valueOf` 于是全落回盘上那份文件的旧值：改了什么一个都看不见、
                 * 底栏写着「没有未保存的改动」（保存灰着）、「放弃并退出」也退不掉
                 * （那一颗只按内存里那几项回写，内存里一项都没有）。
                 *
                 * 后端按参数值把"草稿 vs 来源"的差异算好了（`readPresetDraftChanges`），
                 * 这里接上就是那次没保存的改动。撤销栈接不回来（那是操作序列，后端不存）——
                 * 这一层只能恢复"改成了什么"，恢复不了"一步步怎么改的"。
                 */
                void api.readPresetDraftChanges().then(
                  (changes) => {
                    if (!alive) return
                    draftRestoredRef.current = key
                    setDraft(changes)
                    setPast([])
                    setFuture([])
                    /* 接回来的是"已经改过"的状态 ⇒ 让列表那枚橙点按后端摘要再对一次 */
                    broadcastDraftSoon()
                  },
                  (e: unknown) => {
                    if (alive) setDraftError(errorText(e))
                  },
                )
              }
            }
          } else {
            clearParamSession(key)
            /*
             * 这一趟是**新建**草稿：正文就是盘上那份，这一屏的 `draft` 天然对得上
             * （空表 = 没有未保存的改动）。记下"这个目标不需要接"，免得目录刷新让
             * effect 重跑时又去后端要一次差异、把这一屏的撤销栈清掉。
             */
            draftRestoredRef.current = key
          }
          /* 草稿刚被新建 / 替换过 ⇒ 让预设列表那枚橙色圆点重读一眼轻摘要 */
          broadcastDraftSoon()
          /*
           * **重开要不要直接进编辑模式**：`beginPresetEdit` 的 DTO 只有 `reused`，
           * "脏不脏"在**摘要**那条路（`get_preset_draft_summary` 的 `dirty`）——
           * 与兄弟链 `PagePresets` 的 `unsaved = draft.reused && draft.dirty` 同一口径
           * （作者 2026-10-16：「下次打开这个模态框，它里面还是显示编辑模式……」）。
           */
          void api.getPresetDraftSummary().then(
            (s) => {
              if (!alive) return
              setDraftOnDisk(
                d.reused === true &&
                  s !== null &&
                  s.dirty &&
                  s.origin === d.origin &&
                  (d.origin === 'local'
                    ? s.path === d.path
                    : s.sourceFileName === d.sourceFileName),
              )
            },
            () => {
              if (alive) setDraftOnDisk(false)
            },
          )
        },
        (e: unknown) => {
          if (!alive) return
          setEditingPreset(null)
          setDraftError(errorText(e))
          setDraftOnDisk(false)
        },
      )
    return () => {
      alive = false
    }
  }, [editingTarget, broadcastDraftSoon])

  /*
   * **离开时把这一屏的内存状态落进会话缓存**（2026-10-19）。
   *
   * "离开"= 这个 hook 卸载（模态框被关掉）。渲染期把当前那几样记进 ref（免去闭包过期），
   * 卸载那一下按当时的钥匙写进去；再进来时由上面那个 effect 按 `reused` 认回来。
   */
  sessionRef.current = { draft, past, future }
  useEffect(
    () => () => {
      const key = sessionKeyRef.current
      const s = sessionRef.current
      if (key !== null && s !== null) writeParamSession(key, s)
    },
    [],
  )

  /*
   * **我那份文件里真实写着的参数值**（只在编辑对象是「本地预设」时读）。
   *
   * 读不出来**不报错**：那是"这一屏退回配方值显示"（下一层兜底），不是整页失败 ——
   * 用户看的是参数，不该为一次读盘失败丢掉整张表。
   * 采用云端新值之后这里要重读一次（值已经落进文件了，见 `decideSync`）。
   * ★ 跟着 `deliveryRevision` 走（2026-10-20）：校准页 `save_preset_calibration`
   *   直接写同一份文件、成功就广播（`deliveryMutated`）—— 不跟的话这一屏的
   *   fileValues 还停在打开时的旧值，校准存完参数行照旧显示改前的数。
   */
  useEffect(() => {
    if (localPath === null) {
      setFileValues(new Map())
      return
    }
    let alive = true
    void api.readPresetParams(localPath).then(
      (v) => {
        if (alive) setFileValues(new Map(Object.entries(v.values)))
      },
      () => {
        if (alive) setFileValues(new Map())
      },
    )
    return () => {
      alive = false
    }
  }, [localPath, fileReload, deliveryRevision])

  /*
   * **云端更新账**（三方：我 / 云端旧值 / 云端新值）。
   *
   * ★ `fetchMissing: false`（2026-10-20 作者裁定）：**打开这一份不发网络**。
   * 原来这里给 `true`，于是"我都没点更新，本地怎么知道有参数变化的" —— 那些逐项差异
   * 是打开弹窗时替用户取回来的（还有些项会因为后来的「更新」被悄悄改写）。
   * 现在这一读只回答"云端有没有新版"（`versionAdvanced`），逐项差异等用户
   * 明确点了「更新 / 选择要更新的参数」再取（见 [`fetchSync`]）。
   *
   * 后端没有云端字节时 `pendingCount` 天然是 0（`param_sync::diff` 缺新值不报待处理），
   * 所以界面不会编出"待选择 N 项"。
   */
  useEffect(() => {
    /* 换了文件：上一份的云端表态不带过来（它认的是那一份的参数与那一版云端） */
    setSyncDecisions({})
    if (localPath === null) {
      setSync(null)
      setSyncError(null)
      return
    }
    let alive = true
    setSyncError(null)
    void api.getPresetParamSync(localPath, false).then(
      (v) => {
        if (alive) setSync(v)
      },
      (e: unknown) => {
        if (alive) {
          setSync(null)
          setSyncError(errorText(e))
        }
      },
    )
    return () => {
      alive = false
    }
  }, [localPath])

  /**
   * **把云端当前版的字节取回来**（2026-10-20）—— 全应用**唯一**会为看差异发网络的那一处。
   *
   * 触发它的只有两个用户动作：预设页云端行的「更新」、参数页那颗「选择要更新的参数」。
   * 取回之后才有逐项差异（旧值 → 新值）与「采用 / 保持」那两颗按钮。
   * 取不回来不算失败：界面照实说"还没取回来看不到差异"。
   *
   * `null` = 这一份不认云端（复制出来的那一份）。
   */
  const fetchSync = useCallback(async (): Promise<PresetParamSync | null> => {
    if (localPath === null) return null
    setSyncFetching(true)
    setSyncError(null)
    try {
      const v = await api.getPresetParamSync(localPath, true)
      setSync(v)
      return v
    } catch (e: unknown) {
      setSyncError(errorText(e))
      return null
    } finally {
      setSyncFetching(false)
    }
  }, [localPath])

  /** 逐项的账，按 key 索引（行上直接取；没有这一项就是 `undefined`） */
  const syncByKey = useMemo(
    () => new Map((sync?.entries ?? []).map((e) => [e.key, e] as const)),
    [sync],
  )
  const syncOf = useCallback((key: string) => syncByKey.get(key), [syncByKey])

  /**
   * 这一项在**草稿层**表过什么态（`null` = 还没表态；非待处理项一律 `null`）。
   *
   * 判据只看那张表态表，不再从草稿值反推 —— 理由见 `syncDecisions` 的注释。
   */
  const syncDraftOf = useCallback(
    (key: string): SyncDraftKind | null => {
      const entry = syncByKey.get(key)
      if (entry === undefined || !entry.pending) return null
      return syncDecisions[key] ?? null
    },
    [syncByKey, syncDecisions],
  )

  /**
   * 只改表态、不动值的那些动作（「保持原样」）走这里 —— 不占一格撤销：
   * 它没有值变化，撤销栈里多一格"什么都没变"的步骤只会让人多按一次撤销。
   * 真改了值的动作一律走 `apply`，由它把表态和值一起记进同一步。
   */
  const markSync = useCallback((key: string, kind: SyncDraftKind | null) => {
    setSyncDecisions((prev) => {
      if (kind === null) {
        if (!(key in prev)) return prev
        const next = { ...prev }
        delete next[key]
        return next
      }
      if (prev[key] === kind) return prev
      return { ...prev, [key]: kind }
    })
  }, [])

  /** 行上「用新值」：把这一项写成云端新值（一次普通改值，进撤销栈；保存才落盘） */
  const adoptSync = useCallback(
    (key: string) => {
      const cloudNew = syncByKey.get(key)?.cloudNew ?? null
      if (cloudNew === null) return
      apply(tr('params.act.adoptSync'), [{ key, value: cloudNew }], {
        fromSync: true,
        sync: { [key]: 'adopt' },
      })
    },
    [apply, syncByKey, tr],
  )

  /** 行上「保持原样」：只记一条待落决定，值一个字不动 */
  const holdSync = useCallback((key: string) => markSync(key, 'hold'), [markSync])

  /** 详情右栏「用回我的值」：撤掉这一项的采用 / 保持，退回已保存值 */
  const useMyValue = useCallback(
    (key: string) => {
      apply(
        tr('params.act.useMine', { label: defOf(key)?.label ?? key }),
        [{ key, value: savedValueOf(key) }],
        { sync: { [key]: null } },
      )
    },
    [apply, defOf, savedValueOf, tr],
  )

  /**
   * **弹窗「应用」**（2026-10-20）：勾上的采用、没勾的保持 —— 全进草稿，保存才落盘。
   *
   * 勾上的那些拼成**一次** `apply`：一次撤销能把整批退回来（作者：「就像批量修改，
   * 只算一次撤销一批」）。一项都没勾时 `apply` 会命中 `noop`，但表态表照写 ——
   * 「保持原样」本来就没有值可改。
   */
  const applySyncDecisions = useCallback(
    (decisions: ParamDecision[]) => {
      const patches: Patch[] = []
      const sync: Record<string, SyncDraftKind | null> = {}
      for (const d of decisions) {
        if (d.kind === 'adopt') {
          sync[d.paramKey] = 'adopt'
          const cloudNew = syncByKey.get(d.paramKey)?.cloudNew ?? null
          if (cloudNew !== null) patches.push({ key: d.paramKey, value: cloudNew })
        } else {
          sync[d.paramKey] = 'hold'
        }
      }
      apply(patches.length > 0 ? tr('params.act.adoptSync') : tr('params.act.holdSync'), patches, {
        fromSync: patches.length > 0,
        sync,
      })
    },
    [apply, syncByKey, tr],
  )

  /** 已表态、还没保存的项数（底栏 / 横幅那句「N 项待保存」） */
  const syncDirty = useMemo(() => {
    let n = 0
    for (const key of Object.keys(syncDecisions)) {
      if (syncDraftOf(key) !== null) n += 1
    }
    return n
  }, [syncDecisions, syncDraftOf])

  return {
    loading: error === null && (catalog === null || combo === null),
    error,
    machines: machinesShown,
    /* 本机真有的（抽屉列的）；`machines` 那份目录数据仍然来自 catalog */
    localPresets,
    deliveryRevision,
    machineId,
    versionId,
    machine,
    version,
    fileLabel: combo?.fileLabel ?? null,
    activeFileName: active?.fileName ?? null,
    fileOf,
    activeUse,
    tabs: localizedTabs,
    sectionsOf,
    defs: localizedDefs,
    defsOfSection,
    defOf,
    placeOf,
    total: combo?.defs.length ?? 0,
    valueOf,
    savedValueOf,
    factoryOf,
    dirtyOf,
    dirtyCount: dirtyKeys.length,
    dirtyTabs,
    changedCount,
    isVisible,
    blockedBy: findBlocked,
    condOn,
    plateOf,
    apply,
    edit,
    editingPreset,
    draftError,
    draftOnDisk,
    saveResult,
    resliceAsk,
    /* `useCallback` 包一下：这个数组会被页面塞进依赖，每次渲染都新建会白重渲 */
    dismissReslice: useCallback(() => setResliceAsk(false), []),
    revertToSaved,
    restoreDefaults,
    discardAll,
    save,
    savedNote,
    savedTone,
    gateNote,
    undo,
    redo,
    canUndo: past.length > 0,
    canRedo: future.length > 0,
    history,
    historyByBatch,
    historyItemCount: historyByBatch.reduce((n, g) => n + g.items.length, 0),
    clearHistory: useCallback(() => {
      setLog([])
      setSeq(0)
      setBatchesByCombo((prev) => new Map(prev).set(at, 0))
      /* 「上一次保存之前的值」跟着账本走 —— 清历史一并清 */
      if (historyKeyText !== null) setPrevSavedByFile((p) => ({ ...p, [historyKeyText]: {} }))
      /* 盘上那本也要清（作者口径：历史是一直留着的，只有这一颗按钮能清掉它） */
      if (historyOrigin !== null && historyId !== null) {
        void api.clearPresetHistory(historyOrigin, historyId).catch(() => {
          /* 清不掉就照界面上的"已清空"走 —— 下一次写回会再试一次 */
        })
      }
    }, [at, historyId, historyKeyText, historyOrigin]),
    savedBatches,
    revertEntry,
    requestCombo,
    requestFile,
    pendingCombo,
    pendingFile,
    confirmPending,
    confirmPendingFile,
    cancelPending,
    pinned,
    sync,
    syncError,
    syncNeedsFetch:
      sync !== null && sync.versionAdvanced && sync.cloudFileName !== null && !sync.cloudReady,
    syncBusy,
    syncFetching,
    syncOf,
    pendingCount: sync?.pendingCount ?? 0,
    syncDraftOf,
    syncDirty,
    applySyncDecisions,
    adoptSync,
    holdSync,
    useMyValue,
    prevSavedOf,
    fetchSync,
  }
}
