/*
 * 参数页的全部状态。
 *
 * # 与早先几版的区别只有一处：数据源
 *
 * 值模型、撤销重做、修改历史的语义照旧重建 —— 那套在之前的稿里已经调顺了，
 * 没有要改的理由。换掉的是数据：C4 之前字段表来自 localStorage 那格"说明书"
 * （`ClientDataPackage`），现在全部走**运行时 catalog**（总纲判据 4 的正面落点）：
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

import { useCallback, useEffect, useMemo, useState } from 'react'
import { api, errorText } from '../../api'
import type {
  ActiveOrigin,
  ActivePreset,
  Machine,
  MachineVersion,
  ParamMeta,
  ParamSection,
  ParamTab,
  RecipeParam,
  RuntimeCatalog,
} from '../../api'
import type { FieldSchema } from '../../components/field'

/** 一条字段定义。**刻意不带当前值** —— 值走 `valueOf()`，免得行数据里那份过期 */
export interface ParamDef {
  key: string
  label: string
  desc: string
  /** 中文分组名（注册表的 section 名，'空间偏移' 这种） */
  group: string
  sectionId: string
  unit?: string
  /** 交给 FieldControl 的形状 */
  field: FieldSchema
  meta?: ParamMeta
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
}

export interface HistoryGroup {
  /** `null` = 未保存那一段 */
  batch: number | null
  title: string
  items: HistoryEntry[]
}

/** 一处要写入的改动 */
export interface Patch {
  key: string
  /** `undefined` = 撤掉草稿（回到已保存的值） */
  value: string | undefined
}

interface EditStep {
  patch: Record<string, string | undefined>
  before: Record<string, string | undefined>
  logId: number
}

interface Catalog {
  machines: Machine[]
  metaByKey: Map<string, ParamMeta>
  /** 页签与分组（页面级，不分机型；从 catalog 的 definition 摊） */
  tabs: ParamTab[]
  /** 每个组合的 MKP 文件名（`A1:STANDARD` → `A1-fastv3.3.toml`）—— 从 catalog 的 files 摊出来 */
  fileByCombo: Map<string, string>
  /**
   * 每个组合对应的**可编辑文件身份**（参数页底座 ③）。
   *
   * 参数页要改一份文件就得知道三件事：文件名、哪条线、用户线的落点 ——
   * `beginPresetEdit` 正是要这三个。官方线落点恒 `null`（由目录给），用户线给 `path`。
   *
   * 官方那一半从 catalog 的 files 摊（`kind === 'mkp_preset'`）；用户那一半从
   * `getUserPresetFiles` 来（我那份认的是 `path`，机型/版本从文件名反推）。
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

const OP_TEXT: Record<'eq' | 'neq' | 'gt', string> = { eq: '等于', neq: '不等于', gt: '大于' }

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
      .map((s) => ({ id: s.id, label: s.label, desc: s.description, count: itemsBySection.get(s.id) ?? 0 }))
      .filter((s) => s.count > 0)
    if (sections.length === 0) continue
    out.push({
      tab: {
        id: meta.id,
        label: meta.label,
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

function unitOf(raw: string | undefined): string | undefined {
  if (raw === undefined) return undefined
  return RANGE_LIKE_UNIT.test(raw) ? undefined : raw
}

/** 某个值该显示成什么字 —— 还原 chip、历史条目、保存确认都走这一套翻译 */
export function valueText(def: ParamDef | undefined, raw: string): string {
  if (raw === '') return '空'
  if (def === undefined) return raw
  if (def.field.control === 'switch') return raw === 'true' ? '开启' : '关闭'
  if (def.field.control === 'choice') {
    return def.field.choices?.find((c) => c.value === raw)?.label ?? raw
  }
  if (def.field.control === 'gcode') return `${raw.split('\n').length} 行`
  return def.unit !== undefined ? `${raw} ${def.unit}` : raw
}

const cellKey = (machineId: string, versionId: string, key: string) =>
  `${machineId}:${versionId}:${key}`

/**
 * 「已应用的那一份 × 当前在看的那一份」。
 *
 * 顶部文件名 pill 用它定颜色与指示：在看的就是已应用的 → 绿（原样）；看的不是
 * → 琥珀 + 副标题行尾给一句**可点的**「切换回已应用」。
 *
 * 作者把点名砍了（提示只说动作，不写目标是谁）：
 *
 *   「不用显示那么清楚，切换回已应用的后面不用东西，就『切换回已应用』就行了。
 *     为什么呢？因为有时候是用户他自己的，他自己的，你都不知道他是什么版本」
 *
 * 所以这里**不输出机型 / 版本名**——已应用的那份可能是用户自己的文件，没有可靠的
 * 「机型 · 版本」可标；要能切回只需要它的组合（machineId / versionId）。
 */
export interface ActiveUse {
  /** 当前看的这份就是已应用的那份 */
  onIt: boolean
  /** 已应用那份的组合 —— 切回动作要用 */
  machineId: string | null
  versionId: string | null
  /** 已应用那份在当前这份包里找得到（找不到就没有切回动作，提示只陈述、不可点） */
  canJump: boolean
}

export interface Params {
  loading: boolean
  /** null = 没出错。非 null 时页面要把这句话显示出来，不要装成空表 */
  error: string | null

  machines: Machine[]
  machineId: string
  versionId: string
  machine: Machine | undefined
  version: MachineVersion | undefined
  /**
   * 当前 combo 的 MKP 文件名（`A1.toml`）—— 从说明书里版本的 files 清单来。
   * `null` = 这个版本没配 MKP 文件（如 A2L），页面自己退成「机型 · 版本」。
   */
  fileLabel: string | null
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

  /** 唯一写入口。一次调用 = 一条撤销 + 一条日志 */
  apply: (label: string, patches: Patch[]) => void
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
  /** 保存后那份用户文件的路径（`presets-mine/…`），用来在页面提示"存到哪了" */
  saveResult: string | null
  /** 行上那个 chip：退回**已保存值**（= 撤掉草稿） */
  revertToSaved: (key: string) => void
  /** 底栏的「恢复默认值」：全部退回出厂值，算一次动作，撤销一下就全回来 */
  restoreDefaults: () => void
  save: () => void
  /** 保存成功后的那一行绿字；null = 不显示 */
  savedNote: string | null
  undo: () => void
  redo: () => void
  canUndo: boolean
  canRedo: boolean
  history: HistoryEntry[]
  historyByBatch: HistoryGroup[]
  /** 这个组合保存过几次 */
  savedBatches: number
  revertEntry: (id: number) => void

  /** 切机型 / 切版本。有未保存改动时先问一句，不静默丢 */
  requestCombo: (machineId: string, versionId: string) => void
  /** 非 null = 正在等用户确认这次切换 */
  pendingCombo: { machineId: string; versionId: string } | null
  confirmPending: () => void
  cancelPending: () => void
}

export function useParams(): Params {
  const [catalog, setCatalog] = useState<Catalog | null>(null)
  const [combo, setCombo] = useState<ComboData | null>(null)
  const [error, setError] = useState<string | null>(null)
  /** 当前选的组合。catalog 到了才知道选谁，所以初值是 null */
  const [pick, setPick] = useState<{ machineId: string; versionId: string } | null>(null)
  const [pendingCombo, setPendingCombo] = useState<{ machineId: string; versionId: string } | null>(
    null,
  )

  /** 已保存的那一层。键是 `机型:版本:字段`，所以切回去还看得见上次存的 */
  const [saved, setSaved] = useState<Map<string, string>>(() => new Map())
  /** 每个组合保存过几次 */
  const [batchesByCombo, setBatchesByCombo] = useState<Map<string, number>>(() => new Map())

  const [draft, setDraft] = useState<Record<string, string>>({})
  const [past, setPast] = useState<EditStep[]>([])
  const [future, setFuture] = useState<EditStep[]>([])
  const [log, setLog] = useState<HistoryEntry[]>([])
  const [seq, setSeq] = useState(0)
  const [savedNote, setSavedNote] = useState<string | null>(null)

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
   * `origin` 决定改的是官方线还是用户线：官方线保存会**另存**成"我那份"，用户线**写回它自己**。
   */
  const [editingPreset, setEditingPreset] = useState<{
    fileName: string
    origin: ActiveOrigin
    path: string | null
  } | null>(null)
  /** 编辑目标的草稿有没有真的落到盘上（后端 `run/draft-preset.json`） */
  const [draftError, setDraftError] = useState<string | null>(null)
  /** 保存后那份用户文件的路径（`presets-mine/…`），用来在页面提示"存到哪了" */
  const [saveResult, setSaveResult] = useState<string | null>(null)

  /**
   * **保存**：把草稿提交成"我的预设"（官方线另存 / 用户线写回）。
   *
   * 真值落在用户根（后端 `commit_preset_draft`），本地那一层只是"这一屏看得到的改动"。
   * 没有编辑目标时如实说，不假装保存了。
   *
   * 定义在这里（早于 `save`）是为了让 `save` 在渲染期就能引用到它。
   */
  const commitDraft = useCallback(() => {
    setEditingPreset((cur) => {
      if (cur === null) {
        setSavedNote('这份没有可保存的目标（没配 MKP 文件或还没选）')
        return cur
      }
      void api.commitPresetDraft().then(
        (done) => {
          setDraftError(null)
          setSavedNote(
            cur.origin === 'mine'
              ? `已写回 ${done.fileName}`
              : done.replaced
                ? `已保存成 ${done.fileName}（覆盖了上一份）`
                : `已另存为 ${done.fileName}`,
          )
          setSaveResult(done.path)
        },
        (e: unknown) => setSavedNote(`保存失败：${errorText(e)}`),
      )
      return cur
    })
  }, [])

  /*
   * **改一个值就写进草稿**（字段级 patch，参数页底座 ③，不重生成整份）。
   *
   * 每次改动直接调一次 `patch_preset_draft` —— 不做防抖：参数页的控件是
   * 步进器 / 下拉 / 开关（一次点击 = 一次确定的值），不像文本编辑器要按字节流。
   * 失败只记进 `draftError`（页面显示一行），**不回滚界面值** —— 用户看到的仍是他刚改的；
   * "真机上草稿没写进去"这件事要看得见，而不是悄悄弹回去。
   *
   * 定义在这里（而不是挨着它用的那几个回调）是为了让 `apply` 能引用到它：
   * `useCallback` 依赖的是**运行时**的绑定，而下面的 `apply` 在渲染期就建好了。
   */
  const patchDraft = useCallback(
    (key: string, value: string) => {
      void api.patchPresetDraft(key, value).then(
        () => setDraftError(null),
        (e: unknown) => setDraftError(errorText(e)),
      )
    },
    [],
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
        setError('读不到目录（catalog）—— 参数页的全部数据都从它出，先确认安装包完整。')
        return
      }
      const metaByKey = new Map<string, ParamMeta>(meta.map((m) => [m.key, m]))
      /* MKP 文件名按「机型:版本」建索引 —— 目录里登记的交付文件就是那一份 */
      const fileByCombo = new Map<string, string>()
      for (const f of world.files) {
        if (f.kind === 'mkp_preset') fileByCombo.set(`${f.machineId}:${f.versionId}`, f.fileName)
      }
      /*
       * 可编辑的文件身份（参数页底座 ③）：官方线从目录的 files 摊；用户线从
       * `getUserPresetFiles` 来 —— **同一个 combo 用户线优先**（用户自己那份就是他实际在用的）。
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
            origin: 'official',
            path: null,
          })
        }
      }
      const mine = await api.getUserPresetFiles().catch(() => [])
      if (!alive) return
      for (const f of mine) {
        /*
         * 我那份的机型/版本从**血统**认（`basedOn*` 指向官方来源那一版）；认不出就跳过 ——
         * 认不出的那份在预设页照常列，只是不对应任何 combo，所以不进这张表（别硬塞一格）。
         */
        const { basedOnMachineId: mid, basedOnVersionId: vid } = f
        if (mid !== null && vid !== null && f.kind === 'mkp_preset') {
          editTargetByCombo.set(`${mid}:${vid}`, {
            fileName: f.fileName,
            origin: 'mine',
            path: f.path,
          })
        }
      }
      setCatalog({
        machines: list,
        metaByKey,
        tabs: tabsOf(world.registry),
        fileByCombo,
        editTargetByCombo,
      })

      /*
       * 默认落在**正在用的那一份**：使用中指针（`run/active-preset.json`）说应用了哪台哪个版本，
       * 就从目录里找它；找不到（没应用过 / 目录里没这台）才退回第一台 ——
       * 与预设页「默认落在已应用那台」同一个理由。
       * 原来写死"第一台"，应用了 P1S 再进这一页还是 A1（作者：「怎么一直是 a1」）。
       */
      const live = await api.getActivePreset().catch(() => null)
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
  }, [])

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
                fileLabel: catalog.fileByCombo.get(`${machineId}:${versionId}`) ?? null,
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

        for (const p of params) {
          const meta = catalog.metaByKey.get(p.key)
          const sectionId = meta?.sectionId ?? p.group
          const def: ParamDef = {
            key: p.key,
            label: p.label,
            desc: p.desc,
            group: p.group,
            sectionId,
            /* 区间的 unit（风扇速度的 `0-255`）在这里就落掉，见 unitOf */
            unit: unitOf(p.unit),
            meta,
            field: {
              key: p.key,
              label: p.label,
              desc: p.desc,
              control: controlOf(p.control, meta),
              unit: unitOf(p.unit),
              min: p.min,
              max: p.max,
              step: p.step,
              choices: p.choices,
            },
          }
          defs.push(def)
          defByKey.set(p.key, def)
          baseByKey.set(p.key, normalizeRaw(p.value, meta?.valueType))
          /* 出厂值：被版本盖过的那几条带 baseValue，没盖过的 value 本身就是出厂值 */
          factoryByKey.set(
            p.key,
            normalizeRaw(p.origin === 'variant' ? p.baseValue : p.value, meta?.valueType),
          )

          const tab = tabs.find((t) => t.sections.some((sec) => sec.id === sectionId))
          const section = tab?.sections.find((sec) => sec.id === sectionId)
          if (tab !== undefined && section !== undefined) {
            placeByKey.set(p.key, {
              tabId: tab.id,
              tabLabel: tab.label,
              sectionId: section.id,
              sectionLabel: section.label,
            })
          }
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

  const defOf = useCallback((key: string) => combo?.defByKey.get(key), [combo])
  const placeOf = useCallback((key: string) => combo?.placeByKey.get(key), [combo])

  const savedValueOf = useCallback(
    (key: string) =>
      saved.get(cellKey(machineId, versionId, key)) ?? combo?.baseByKey.get(key) ?? '',
    [combo, machineId, saved, versionId],
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
            need: `${OP_TEXT[cond.op]} ${valueText(parent, expected)}`,
          }
        }
        at = cond.key
      }
      return null
    },
    [catalog, defOf, valueOf],
  )

  const isVisible = useCallback((key: string) => findBlocked(key) === null, [findBlocked])

  /**
   * 唯一写入口。
   *
   * 「改成与已保存值相同」= 撤掉草稿，而不是记一条「改成一样」的假改动；
   * 值没变的项不占一格撤销。
   */
  const apply = useCallback(
    (label: string, patches: Patch[]) => {
      const patch: Record<string, string | undefined> = {}
      const before: Record<string, string | undefined> = {}
      const items: HistoryItem[] = []

      for (const p of patches) {
        const savedValue = savedValueOf(p.key)
        const target = p.value !== undefined && p.value === savedValue ? undefined : p.value
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
          from: current ?? savedValue,
          to: target ?? savedValue,
          label: def?.label ?? p.key,
          tab: place?.tabLabel ?? '',
          section: place?.sectionLabel ?? def?.group ?? '',
          unit: def?.unit ?? null,
        })
      }

      if (items.length === 0) return

      const id = seq + 1
      const next = { ...draft }
      for (const [key, value] of Object.entries(patch)) {
        if (value === undefined) delete next[key]
        else next[key] = value
      }

      setSeq(id)
      setDraft(next)
      setPast([...past, { patch, before, logId: id }])
      setFuture([]) // 新动作把「重做」那一支剪掉，撤销栈的通例
      setLog([...log, { id, no: id, action: label, items, state: 'draft', batch: null }])
      setSavedNote(null)

      /*
       * **落进草稿 TOML**（参数页底座 ③）：这一屏的内存改动同步写进 `run/draft-preset.json`
       * 的正文，于是切页 / 关掉软件再回来都还在。`value === undefined`（撤回保存值）
       * 用「已保存值」回写 —— 草稿里该是这个值，不是"没有这一项"。
       *
       * 一次 `apply` 可能带多项（如"恢复默认值"几十项）→ 逐项 patch。
       * 后端只改那一处、不重生成整份，所以多次调用是叠加的，不会互相冲掉。
       */
      for (const [key, value] of Object.entries(patch)) {
        patchDraft(key, value ?? savedValueOf(key))
      }
    },
    [defOf, draft, log, past, patchDraft, placeOf, savedValueOf, seq],
  )

  const edit = useCallback(
    (key: string, value: string) => {
      apply(`改 ${defOf(key)?.label ?? key}`, [{ key, value }])
    },
    [apply, defOf],
  )

  /** 行上那个 chip：退回已保存值 */
  const revertToSaved = useCallback(
    (key: string) => {
      apply(`还原 ${defOf(key)?.label ?? key}`, [{ key, value: savedValueOf(key) }])
    },
    [apply, defOf, savedValueOf],
  )

  /** 整份恢复出厂值：一次动作，撤销一下就全回来 */
  const restoreDefaults = useCallback(() => {
    if (combo === null) return
    apply(
      '恢复默认值',
      combo.defs.map((d) => ({ key: d.key, value: combo.factoryByKey.get(d.key) ?? '' })),
    )
  }, [apply, combo])

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
    setPast(past.slice(0, -1))
    setFuture([...future, step])
    setLog(log.map((e) => (e.id === step.logId ? { ...e, state: 'undone' } : e)))
    setSavedNote(null)
    /* 撤销也要落进草稿 TOML（否则关掉再回来，"撤了的那一步"又回来了） */
    for (const [key, value] of Object.entries(step.before)) {
      patchDraft(key, value ?? savedValueOf(key))
    }
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
    setFuture(future.slice(0, -1))
    setPast([...past, step])
    setLog(log.map((e) => (e.id === step.logId ? { ...e, state: 'draft' } : e)))
    setSavedNote(null)
    /* 重做同样要落进草稿 TOML */
    for (const [key, value] of Object.entries(step.patch)) {
      patchDraft(key, value ?? savedValueOf(key))
    }
  }, [draft, future, log, past, patchDraft, savedValueOf])

  const at = `${machineId}:${versionId}`
  const savedBatches = batchesByCombo.get(at) ?? 0

  /**
   * 保存：草稿并进已保存层、清撤销栈、日志里那批草稿条目转成「已保存」。
   *
   * 清撤销栈是因为保存是提交点 —— 「保存完再撤销」会撤出一个已保存状态的假草稿。
   * 日志不清：它回答的是「这次打开之后我改过什么」，清掉就成了骗人的表现。
   */
  const save = useCallback(() => {
    if (combo === null) return
    /*
     * **先落盘**：把草稿提交成"我的预设"（官方线另存 / 用户线写回，后端 `commit_preset_draft`）。
     * 本地这一层只是"这一屏看得到的已保存值"，真值在用户根 —— 两句状态由 `commitDraft` 写。
     */
    commitDraft()
    const nextSaved = new Map(saved)
    for (const [key, value] of Object.entries(draft)) {
      nextSaved.set(cellKey(combo.machineId, combo.versionId, key), value)
    }
    const batch = savedBatches + 1
    setSaved(nextSaved)
    setBatchesByCombo(new Map(batchesByCombo).set(at, batch))
    setDraft({})
    setPast([])
    setFuture([])
    setLog(log.map((e) => (e.state === 'draft' ? { ...e, state: 'saved', batch } : e)))
  }, [at, batchesByCombo, commitDraft, combo, draft, log, saved, savedBatches])

  /** 历史面板里的「还原」：把那一条改动的值退回去，本身也算一次新编辑（可以再撤销） */
  const revertEntry = useCallback(
    (id: number) => {
      const entry = log.find((e) => e.id === id)
      if (entry === undefined || entry.state === 'undone') return
      apply(
        `回退 ${entry.action}`,
        entry.items.map((item) => ({ key: item.key, value: item.from })),
      )
    },
    [apply, log],
  )

  /** 换组合：草稿、栈、日志一起作废（面板上写的是「这次打开之后」）。已保存的那一层留着 */
  const switchTo = useCallback((next: { machineId: string; versionId: string }) => {
    setPick(next)
    setDraft({})
    setPast([])
    setFuture([])
    setLog([])
    setSeq(0)
    setSavedNote(null)
  }, [])

  const dirtyKeys = useMemo(() => Object.keys(draft), [draft])

  const requestCombo = useCallback(
    (nextMachine: string, nextVersion: string) => {
      if (nextMachine === machineId && nextVersion === versionId) return
      /* 有未保存改动就先问一句 —— 切过去草稿就没了，不能静默丢 */
      if (dirtyKeys.length > 0) {
        setPendingCombo({ machineId: nextMachine, versionId: nextVersion })
        return
      }
      switchTo({ machineId: nextMachine, versionId: nextVersion })
    },
    [dirtyKeys.length, machineId, switchTo, versionId],
  )

  const confirmPending = useCallback(() => {
    if (pendingCombo === null) return
    switchTo(pendingCombo)
    setPendingCombo(null)
  }, [pendingCombo, switchTo])

  const cancelPending = useCallback(() => setPendingCombo(null), [])

  const dirtyTabs = useMemo(() => {
    const set = new Set<string>()
    for (const key of dirtyKeys) {
      const place = combo?.placeByKey.get(key)
      if (place !== undefined) set.add(place.tabId)
    }
    return set
  }, [combo, dirtyKeys])

  /** 与出厂不同的条数 —— 按当前机型实算，不是写死的 67 */
  const changedCount = useMemo(() => {
    if (combo === null) return 0
    return combo.defs.filter((d) => valueOf(d.key) !== (combo.factoryByKey.get(d.key) ?? '')).length
  }, [combo, valueOf])

  const sectionsOf = useCallback(
    (tabId: string) => combo?.tabs.find((t) => t.id === tabId)?.sections ?? [],
    [combo],
  )

  const defsOfSection = useCallback(
    (sectionId: string) => combo?.defs.filter((d) => d.sectionId === sectionId) ?? [],
    [combo],
  )

  const history = useMemo(() => [...log].reverse(), [log])

  /**
   * 「按保存批次」视图：未保存的一段在最上，然后是第 N 次、第 N-1 次…… 保存。
   * 撤销过的条目**留在原来那一段里**（仍标「已撤销」划掉）—— 挪走或删掉都会让
   * 「我那次保存到底改了什么」说不清。
   */
  const historyByBatch = useMemo<HistoryGroup[]>(() => {
    const groups: HistoryGroup[] = []
    const drafts = log.filter((e) => e.batch === null)
    if (drafts.length > 0) {
      groups.push({ batch: null, title: '未保存', items: [...drafts].reverse() })
    }
    for (let b = savedBatches; b >= 1; b -= 1) {
      const items = log.filter((e) => e.batch === b)
      if (items.length > 0) {
        groups.push({ batch: b, title: `第 ${b} 次保存`, items: [...items].reverse() })
      }
    }
    return groups
  }, [log, savedBatches])

  const machine = catalog?.machines.find((m) => m.id === machineId)
  const version = machine?.versions.find((v) => v.id === versionId)

  const fileOf = useCallback(
    (m: string, v: string) => catalog?.fileByCombo.get(`${m}:${v}`) ?? null,
    [catalog],
  )

  /*
   * 「在看的是不是已应用的那份」：唯一底账 × 当前 combo。
   * 底账走 IPC（`run/active-preset.json`），所以在依赖变化时读一次 state
   * （开页 / 切组合）——与旧版"每次读一次 localStorage"同一个节奏，不是每次 render。
   * 依赖带 machineId / versionId：切组合时 pick 先行、颜色立刻跟上，
   * 不等字段重拉完（`combo` 那一层只管字段，不影响颜色）。
   *
   * `canJump`：已应用那份在当前目录里找得到才有「切换回」动作 —— 找不到时切过去会落到
   * 一个空壳 combo（字段在、值全空），那是假信息，所以宁可不给动作、只陈述。
   */
  const [active, setActive] = useState<ActivePreset | null>(null)
  useEffect(() => {
    let alive = true
    void api
      .getActivePreset()
      .then((a) => {
        if (alive) setActive(a)
      })
      .catch(() => {
        if (alive) setActive(null)
      })
    return () => {
      alive = false
    }
  }, [machineId, versionId])

  const activeUse = useMemo<ActiveUse | null>(() => {
    if (active === null) return null
    const liveMachine = catalog?.machines.find((m) => m.id === active.machineId)
    const liveVersion = liveMachine?.versions.find((v) => v.id === active.versionId)
    return {
      onIt: active.machineId === machineId && active.versionId === versionId,
      machineId: active.machineId,
      versionId: active.versionId,
      canJump: liveMachine !== undefined && liveVersion !== undefined,
    }
  }, [active, catalog, machineId, versionId])

  /*
   * **编辑目标**：当前 combo 对应的那一份文件（参数页底座 ③）。
   *
   * 默认由"正在应用的那一份"决定（`catalog.editTargetByCombo` 里那张表按 combo 查）；
   * 用户切机型/版本 = 换目标，**不动 active-preset** —— 编辑与应用解耦。
   * combo 对应不到任何文件（如 A2L 没配 MKP）就是 `null`：页面照实说"这份没法改"。
   */
  const editingTarget = useMemo(() => {
    if (catalog === null || machineId === '' || versionId === '') return null
    return catalog.editTargetByCombo.get(`${machineId}:${versionId}`) ?? null
  }, [catalog, machineId, versionId])

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
        },
        (e: unknown) => {
          if (!alive) return
          setEditingPreset(null)
          setDraftError(errorText(e))
        },
      )
    return () => {
      alive = false
    }
  }, [editingTarget])

  return {
    loading: error === null && (catalog === null || combo === null),
    error,
    machines: catalog?.machines ?? [],
    machineId,
    versionId,
    machine,
    version,
    fileLabel: combo?.fileLabel ?? null,
    fileOf,
    activeUse,
    tabs: combo?.tabs ?? [],
    sectionsOf,
    defs: combo?.defs ?? [],
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
    apply,
    edit,
    editingPreset,
    draftError,
    saveResult,
    revertToSaved,
    restoreDefaults,
    save,
    savedNote,
    undo,
    redo,
    canUndo: past.length > 0,
    canRedo: future.length > 0,
    history,
    historyByBatch,
    savedBatches,
    revertEntry,
    requestCombo,
    pendingCombo,
    confirmPending,
    cancelPending,
  }
}
