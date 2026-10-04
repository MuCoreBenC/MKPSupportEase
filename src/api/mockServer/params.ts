import type { CatalogParamDef, CatalogRegistry, ParamMeta, RecipeParam } from '../../api/contract'
import layoutJson from './data/layout_schema.json'
import registryJson from './data/param_registry.json'
import type { RawLayoutSchema, RawParam, RawParamRegistry } from './types'

/**
 * 参数视图：把注册表 + 布局表合成前端直接能渲染的一串 RecipeParam。
 *
 * 三条数据来源分工明确，别混：
 *
 * - **字段本身**（label / 单位 / 约束 / 控件 / 可选项）→ `param_registry.json` 的 `params`
 * - **分组名**（'空间偏移' / '圆盘动作控制'）→ `param_registry.json` 的 `tabs[].sections[]`
 *   （layout_schema 那边只有 section id，没有中文名）
 * - **顺序**（组内先后）→ `layout_schema.json` 的 `tabs[].sections[].items[]`
 *   （这是权威顺序；`param.layout.order` 只是兜底，实测有 0.5 这种插队值）
 *
 * 控件类型由数据说，不由页面猜 —— `uiComponent` 实测只有 5 种：
 * number 47 / switch 12 / segmented 11 / select 2 / gcode 2。
 */

const registry = registryJson as RawParamRegistry
const layout = layoutJson as RawLayoutSchema

/** 一个字段用哪种控件。映射表只此一份，别在别处再写一遍 */
const CONTROL: Record<string, RecipeParam['control']> = {
  number: 'number',
  switch: 'switch',
  segmented: 'choice',
  select: 'choice',
  gcode: 'text',
}

const paramByKey = new Map(registry.params.map((p) => [p.key, p]))

/** sectionId → 中文分组名 */
const sectionLabel = new Map<string, string>()
for (const tab of registry.tabs) {
  for (const s of tab.sections ?? []) {
    sectionLabel.set(s.id, s.label)
  }
}

/**
 * 按 layout_schema 的 items 顺序排好的 param.key。
 *
 * layout 里出现但注册表里没有的 key 会被跳过。实测两份快照对得上（74 条一条不漏），
 * 所以这里不再补一遍「注册表里有、layout 没排到」的兜底。
 */
const ORDERED_KEYS: string[] = []
const seenKeys = new Set<string>()
for (const tab of layout.tabs) {
  for (const section of tab.sections) {
    for (const item of section.items ?? []) {
      if (!paramByKey.has(item.paramKey) || seenKeys.has(item.paramKey)) continue
      ORDERED_KEYS.push(item.paramKey)
      seenKeys.add(item.paramKey)
    }
  }
}

/**
 * 值一律转成字符串。
 *
 * 契约里定的：单位换算与小数位数不该由界面再做一遍。布尔转 'on' / 'off' 是为了
 * 和 switch 控件的两个选项对齐 —— 界面拿到 'true' 还得再翻译一次就没意义了。
 */
function toText(value: unknown): string {
  if (value === undefined || value === null) return ''
  if (typeof value === 'boolean') return value ? 'on' : 'off'
  return String(value)
}

/** machineFilter 是逗号分隔的字符串，不是数组 */
function appliesTo(param: RawParam, machineId: string): boolean {
  if (!param.machineFilter) return true
  return param.machineFilter
    .split(',')
    .map((s) => s.trim())
    .includes(machineId)
}

function toRecipeParam(param: RawParam, machineId: string, versionId: string | null): RecipeParam {
  const base = toText(param.defaultValue)

  // 「版本覆盖」那一层的真数据就在 machineVariants 里，键是 '机型:版本'
  const overrideRaw =
    versionId !== null ? param.machineVariants?.[`${machineId}:${versionId}`] : undefined
  const hasOverride = overrideRaw !== undefined

  return {
    key: param.key,
    /* 与 `paramMeta()` 同源（注册表的 tomlKey）—— 形状与真后端 `RecipeParamDto` 一致 */
    tomlKey: param.tomlKey,
    label: param.label,
    desc: param.desc,
    group: sectionLabel.get(param.layout.sectionId) ?? param.layout.sectionId,
    unit: param.unit,
    control: CONTROL[param.uiComponent] ?? 'text',
    choices: param.choices,
    min: param.min,
    max: param.max,
    step: param.step,
    value: hasOverride ? toText(overrideRaw) : base,
    origin: hasOverride ? 'variant' : 'base',
    // 只有被这个版本盖过才带 —— 界面靠它显示「还原成基础配方的值」
    baseValue: hasOverride ? base : undefined,
  }
}

/**
 * 某一张配方的全部参数。`versionId` 传 `null` = 只看机型基底，那时每条都是 `origin: 'base'`。
 *
 * 默认排除 `deprecated` 与 `hidden`：实测 7 个字段已废弃，它们还在数据里是为了兼容旧文件，
 * 不是给人改的。被 `machineFilter` 排除的字段也不出现在这台机型上 ——
 * 那是后端的事实，界面不该自己在客户端筛。
 */
export function resolveParams(machineId: string, versionId: string | null): RecipeParam[] {
  const out: RecipeParam[] = []
  for (const key of ORDERED_KEYS) {
    const param = paramByKey.get(key)
    if (!param) continue
    if (param.deprecated || param.hidden) continue
    if (!appliesTo(param, machineId)) continue
    out.push(toRecipeParam(param, machineId, versionId))
  }
  return out
}

/** 校验 valueType 的字面量，越界就退回 'string'（解析器自己不抛，宁可给一个安全的档） */
const VALUE_TYPES = new Set(['float', 'int', 'bool', 'string'])

/**
 * 全部参数的元信息。含废弃与隐藏 —— 界面要能看到它们为什么不显示，
 * 所以这一份**不按 machineFilter / deprecated 过滤**（要过滤的是取值那一份）。
 *
 * `showWhen` 的 `op` 只认 `eq` / `neq` / `gt`（契约就是这么定的），越界的会被丢掉 ——
 * 实测 43 条全在这三档里，所以条数不变。
 */
export function paramMeta(): ParamMeta[] {
  return registry.params.map((p): ParamMeta => {
    const meta: ParamMeta = {
      key: p.key,
      tomlKey: p.tomlKey,
      jsonKey: p.jsonKey,
      configKey: p.configKey,
      section: p.section,
      sectionId: p.layout.sectionId,
      order: p.layout.order,
      scope: p.scope === 'machine_specific' ? 'machine_specific' : 'universal',
      valueType: VALUE_TYPES.has(p.valueType)
        ? (p.valueType as ParamMeta['valueType'])
        : 'string',
      uiComponent: p.uiComponent,
      unit: p.unit,
      tomlComment: p.tomlComment,
      mergeGroup: p.mergeGroup,
      pinned: p.pinned,
      deprecated: p.deprecated,
    }
    if (p.variantMode === 'shared' || p.variantMode === 'per_variant') {
      meta.variantMode = p.variantMode
    }
    // 上游是逗号分隔的字符串，不是数组
    if (p.machineFilter) {
      meta.machineFilter = p.machineFilter.split(',').map((s) => s.trim())
    }
    if (p.showWhen && (p.showWhen.op === 'eq' || p.showWhen.op === 'neq' || p.showWhen.op === 'gt')) {
      meta.showWhen = { key: p.showWhen.key, op: p.showWhen.op, value: toText(p.showWhen.value) }
    }
    return meta
  })
}

/**
 * catalog definition 的 registry 部分 —— 与 Rust `runtime::Catalog` 的 serde 形态同构。
 *
 * 浏览器演示的 `getRuntimeCatalog` 用它：参数页的页签/分组树从这份摊，与真机读
 * `catalog.json` 的路径**同一个形状**。数据就是同一批快照（74 条字段 + 页签元数据 +
 * 布局表），不另编一份 —— 快照改了这里跟着变，与真 catalog 的漂移只会来自快照本身过期。
 */
export function catalogRegistry(): CatalogRegistry {
  return {
    params: registry.params.map((p) => ({
      key: p.key,
      section: p.section,
      layout: { order: p.layout.order, sectionId: p.layout.sectionId },
      /* 定义侧的显示格子（2026-10-02，① deprecated 链路）：参数页的字段清单从这摊，
         弃用字段也在这里 —— 「显示，但只读」。 */
      label: p.label,
      desc: p.desc,
      unit: p.unit,
      uiComponent: p.uiComponent,
      valueType: VALUE_TYPES.has(p.valueType)
        ? (p.valueType as CatalogParamDef['valueType'])
        : 'string',
      choices: (p.choices ?? []).map((c) => ({
        value: toText(c.value),
        label: c.label,
        deprecated: c.deprecated === true ? true : undefined,
      })),
      min: p.min,
      max: p.max,
      step: p.step,
      deprecated: p.deprecated === true,
      machineFilter: p.machineFilter
        ? p.machineFilter.split(',').map((s) => s.trim()).filter((s) => s !== '')
        : [],
      showWhen:
        p.showWhen && (p.showWhen.op === 'eq' || p.showWhen.op === 'neq' || p.showWhen.op === 'gt')
          ? { key: p.showWhen.key, op: p.showWhen.op, value: toText(p.showWhen.value) }
          : undefined,
    })),
    tabs: registry.tabs.map((t) => ({
      id: t.id,
      label: t.label,
      order: t.order ?? Number.MAX_VALUE,
      icon: t.icon,
      sections: (t.sections ?? []).map((s) => ({
        id: s.id,
        label: s.label,
        order: s.order,
        description: s.description,
      })),
    })),
    layout: layout.tabs.map((t) => ({
      id: t.id,
      sections: (t.sections ?? []).map((s) => ({
        id: s.id,
        items: (s.items ?? []).map((i) => ({ id: i.id, paramKey: i.paramKey })),
      })),
    })),
  }
}
