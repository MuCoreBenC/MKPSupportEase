/**
 * `data/*.json` 的逐字形状。
 *
 * 这些类型只在 `src/api/mockServer/` 内部用，不出边界 —— 对外的形状在 `src/api/contract.ts`。
 * 分开是有意的：上游的 JSON 长什么样是它的事，我们对前端的承诺是另一件事，
 * 中间那层翻译就是 `resolve` 那几个文件存在的理由。上游字段改名只会打到这个文件。
 *
 * 命名一律 `Raw*`，看见 Raw 就知道「这是别人的形状，别直接端给前端」。
 */

// —— machine_catalog.json ——

export interface RawBrand {
  id: string
  name: string
  logo: string
}

export interface RawVersion {
  /** **这一版专属的外观图**（资产 id）。缺省 = 回落机型图 —— 演示数据里今天都没配 */
  image?: string
  id: string
  name: string
  /** MKP 预设的文件名。**空字符串 = 没配**（A2L 就是空的） */
  presetFile: string
  /** bundle id。空字符串 = 没配 */
  recommendedBundle: string
  tag: string
  description: string
}

export interface RawModel {
  id: string
  /** 上游六台机型这个字段全是空串，显示一律用 display */
  name: string
  display: string
  /** 机型级默认 bundle。版本级的 recommendedBundle 优先 */
  defaultBundle: string
  externalAliases: string[]
  /**
   * **资产 id**（不是文件名）：与真后端 `machines_dto` 同一口径 —— 界面拿它去
   * `RuntimeCatalog.assets[]` 里查 `path`（2026-10-03 第二刀起）。空串 = 这台没有图
   */
  image: string
  /** 第二个图位（快拆版外观图）。缺省 = 没有 */
  imageVariant?: string
  icon: string
  versions: RawVersion[]
}

export interface RawBedSize {
  width: number
  depth: number
}

export interface RawMovementRange {
  minX: number
  maxX: number
  minY: number
  maxY: number
  maxZ: number
}

export interface RawGlueArea {
  glueMinX: number
  glueMaxX: number
  glueMinY: number
  glueMaxY: number
  wipeX: number
}

export interface RawCalibration {
  lShapeBaseX: number
  lShapeBaseY: number
  xLineX: number
  xLineY: number
  xLineYEnd: number
  yLineX: number
  yLineXEnd: number
  yLineY: number
  zStartX: number
  zStartY: number
}

export interface RawFlags {
  gcodeMarker: string
  hasSecondFan: boolean
}

/**
 * 上游 Go 侧这是 `map[string]any`，叶子字段没有任何约束。
 * 实测五台机型（A1 / A1_MINI / P1S / P2S / X1C）的字段集完全一致，所以这里定死。
 * 哪天上游多给一个字段，这里会静默忽略；少给一个，界面上会露出 `undefined`。
 */
export interface RawDimensions {
  bedSize: RawBedSize
  movementRange: RawMovementRange
  glueArea: RawGlueArea
  calibration: RawCalibration
  edgeZone: number
  flags: RawFlags
}

export interface RawZonePoint {
  x: number
  y: number
}

export interface RawZonePolygon {
  points: RawZonePoint[]
}

export interface RawCatalog {
  brands: RawBrand[]
  /** 按品牌显示名分组。上游只有 'Bambu Lab' 一个键 */
  models: Record<string, RawModel[]>
  /** 只覆盖已配置的机型，A2L 不在里面 */
  dimensions: Record<string, RawDimensions>
  /** 只覆盖 P1S / P2S / X1C */
  forbiddenZones: Record<string, RawZonePolygon[]>
}

// —— bundles.json ——

export interface RawBundle {
  id: string
  display: string
  /**
   * 上游标注「这个 bundle 属于哪台机型」。
   * 我们不用它 —— 选中关系由机型侧的 bundle 字段单向决定，
   * 两边互指而没人对账正是上游最容易出错的地方。
   */
  machineId: string
  /** asset id，**上游刻意只放 bbs_profile / orca_profile，不放 mkp_preset** */
  assetRefs: string[]
}

// —— assets.json（由上游 source/assets/*.toml 汇总而来） ——

export interface RawAsset {
  id: string
  resourceType: string
  category: string
  machineId: string
  fileName: string
  /** 相对预设仓库根的真实路径 */
  relativePath: string
}

export interface RawAssetsFile {
  assets: RawAsset[]
}

// —— param_registry.json ——

export interface RawParamLayout {
  /** 组内顺序。可以是小数（实测有 0.5） */
  order: number
  /** 指向 layout_schema 里的 section id */
  sectionId: string
}

export interface RawChoice {
  value: string
  label: string
  /** 选项级弃用（实测 1 条：`wiping.outer_structure` 的 `sheath` = 护套） */
  deprecated?: boolean
}

/**
 * 可见性条件。**只有一层，不是链** —— 但被指向的那个字段自己也可能有 showWhen，
 * 所以实际判断要顺着 key 往上递归。实测 43 个字段有这个字段，op 取 'eq' | 'neq' | 'gt'。
 */
export interface RawShowWhen {
  key: string
  op: string
  value: unknown
}

export interface RawParam {
  key: string
  label: string
  desc: string
  unit?: string
  tomlKey: string
  jsonKey: string
  configKey: string
  /** 'float' | 'int' | 'bool' | 'string' */
  valueType: string
  /** 'number' | 'switch' | 'segmented' | 'select' | 'gcode' —— 一共只有这五种 */
  uiComponent: string
  defaultValue: unknown
  /** segmented / select 的可选项，实测 13 个字段有 */
  choices?: RawChoice[]
  min?: number
  max?: number
  step?: number
  /** 'universal' | 'machine_specific' */
  scope: string
  /** 'shared' | 'per_variant'，多数条目没有这个字段 */
  variantMode?: string
  /**
   * 各机型变体的实际取值，键是 `'A1:FASTV3.3'` 这种 `机型:版本`。
   * 实测只有 5 个 machine_specific 字段有 —— 这就是「版本覆盖」那一层的数据来源。
   */
  machineVariants?: Record<string, unknown>
  machineMinVariants?: Record<string, number>
  machineMaxVariants?: Record<string, number>
  /** **逗号分隔的字符串**，不是数组：`'A1,A1_MINI,A2L,P1S,P2S,X1C'` */
  machineFilter?: string
  /** TOML 里的节名，和 layout 的 sectionId 不是一回事 */
  section: string
  layout: RawParamLayout
  showWhen?: RawShowWhen
  /** 依赖的父字段，和 showWhen.key 一般相同 */
  parentKey?: string
  tomlComment?: string
  mergeGroup?: string
  pinned?: boolean
  /** 实测 7 个字段已废弃，界面默认不显示 */
  deprecated?: boolean
  hidden?: boolean
}

/**
 * 注册表侧的 section 元信息。**section 的中文名只在这里有**，
 * layout_schema 那边只有 id 和 items —— 所以分组名要从注册表取，顺序从 layout 取。
 */
export interface RawRegistrySection {
  id: string
  label: string
  description?: string
  order: number
}

export interface RawRegistryTab {
  id: string
  label: string
  icon?: string
  order?: number
  sections?: RawRegistrySection[]
}

export interface RawParamRegistry {
  params: RawParam[]
  tabs: RawRegistryTab[]
  updated: string
}

// —— layout_schema.json ——

export interface RawLayoutItem {
  id: string
  /** 指向 param_registry 里的 param.key */
  paramKey: string
}

export interface RawLayoutSection {
  id: string
  /** section 内的字段与顺序。这是排序的权威来源，比 param.layout.order 更直接 */
  items?: RawLayoutItem[]
}

export interface RawLayoutTab {
  id: string
  sections: RawLayoutSection[]
}

export interface RawLayoutSchema {
  tabs: RawLayoutTab[]
}

// —— preset_registry.json ——

export interface RawPresetEntry {
  fileName: string
  relativePath: string
  /** 实测 9 条全是 'bbs_profile' */
  resourceType: string
  category: string
  machineIds: string[]
  displayName: string
  nozzle: string
  layerHeight: string
}

export interface RawPresetRegistry {
  entries: RawPresetEntry[]
}

// —— bbs_files.json ——

/**
 * bbs 这一档的文件事实：字节数与内容更新时间。
 *
 * 上游 `assets_index.json` 里 `size` 全是 0、`sha256` 全是空串，所以那一份给不出真值。
 * 这一份是从上游预设仓同步真文件时顺手算出来的，**只覆盖 `presets/bbs`**。
 */
export interface RawBbsFile {
  /** 与 `assets.json` 同一套标识 */
  assetId: string
  relativePath: string
  fileName: string
  bytes: number
  sha256: string
  /** 上游记的内容更新时间（**不是**本机 checkout 的 mtime —— 那个每个 clone 都不一样） */
  updatedAt: string
}

export interface RawBbsFilesFile {
  _note: string
  source: {
    repo: string
    commit: string | null
    presetsDir: string
    manifest: string
    syncedAt: string
    copiedTo: string
  }
  files: RawBbsFile[]
}
