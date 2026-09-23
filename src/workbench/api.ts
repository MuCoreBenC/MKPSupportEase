/**
 * IPC 的薄封装 —— **只包 invoke，不做任何合并、推算、兜底**（doc §6 最后一句）。
 *
 * 这里的类型是 Rust 那边 DTO 的**手抄镜像**。手抄的代价是可能抄错，
 * 所以每个结构上标了对应的 Rust 类型名，出问题时能一眼找到对面。
 * 不用代码生成是因为这套契约只有一个消费者、而且会跟着界面一起改 ——
 * 生成器要维护的东西比它省下的多。
 *
 * # 一条纪律：前端不算业务
 *
 * 状态、文案、来源、能不能改、批量会落到哪几列，全部由后端算完给。
 * 前端只做三件事：把值画出来、把手势翻成 `Patch`、把撤销栈记在会话内存里。
 *
 * 判据很具体：**这个文件里不该出现任何 `if` 判状态词、不该拼中文句子**。
 * 需要一句话的时候，那句话应该已经在返回值里了（`reason` / `explain` / `need`）。
 */
import { invoke } from '@tauri-apps/api/core'

/* ---------- 错误 ---------- */

/** `error::AppError`。与客户端共用同一套 —— trace id 也共用一条日志线 */
export interface AppError {
  code:
    | 'NOT_FOUND'
    | 'PERMISSION_DENIED'
    | 'INVALID_ARGUMENT'
    | 'CORRUPTED'
    | 'SHA_MISMATCH'
    | 'IO'
    | 'NOT_IMPLEMENTED'
    | 'INTERNAL'
  message: string
  traceId: string
  detail?: string
}

/** Tauri 把 `Err(AppError)` 原样序列化过来，所以 catch 到的就是它 */
export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e
}

/* ---------- 枚举。与 Rust 的变体一一对应 ---------- */

/** `domain::Level` —— 可写的两层 */
export type Level = 'machine' | 'version'
/** `domain::Origin` —— 有效值来自哪一层，比 Level 多一个出厂 */
export type Origin = 'factory' | 'machine' | 'version'
/** `wording::BuildState` */
export type BuildState = 'built' | 'stale' | 'neverBuilt' | 'noResources'
/** `wording::ArtifactState` */
export type ArtifactState = 'fresh' | 'stale' | 'missing'
/** `wording::SaveState` */
export type SaveState = 'saved' | 'dirty'
/** `wording::BbsAssign` */
export type BbsAssign = 'assigned' | 'optional' | 'archiveOnly'
/** `wording::BbsSource` */
export type BbsSource = 'own' | 'inheritedFromMachine'
/** `patch::Visibility` */
export type Visibility = 'menu' | 'archiveOnly'
/** `derive::CellKind` —— 三种，不是四种：「选中了升级成真控件」是前端的事 */
export type CellKind = 'notApplicable' | 'gcode' | 'value'
/** `upstream::ResourceType` */
export type ResourceType = 'bbsProfile' | 'mkpPreset' | 'image'
/** `registry::ValueType` */
export type ValueType = 'float' | 'int' | 'bool' | 'string'
/** `registry::UiComponent` */
export type UiComponent = 'number' | 'switch' | 'segmented' | 'select' | 'gcode'
/** `visibility::BlockScope` */
export type BlockScope = 'field' | 'section'
/** `preview::BulkKind` */
export type BulkKind = 'detaching' | 'changing' | 'noChange'

/* ---------- 启动 ---------- */

/** `app::Roots`。`upstream` 为 null = 定位不到（是状态，不是错误） */
export interface Roots {
  workbench: string
  dist: string
  upstream: string | null
}

/** `app::UpstreamInfo`。`minimumClient` 为 null = **上游未声明** */
export interface UpstreamInfo {
  registryUpdated: string
  manifestUpdated: string
  channel: string
  minimumClient: string | null
  latestRelease: string | null
  params: number
  machines: number
  deliverables: number
  fallbacks: number
}

/** `app::Boot`。**上游缺失时也是成功返回**，界面要能在那个状态下显示数据根 */
export interface Boot {
  roots: Roots
  problem: string | null
  detail: string | null
  info: UpstreamInfo | null
}

/* ---------- 整本 ---------- */

/** `derive::Badges`。`items` 含上游给的那一半，`own` 只算我们写的（doc §3.6） */
export interface Badges {
  machines: number
  versions: number
  baseItems: number
  baseOwn: number
  overrideItems: number
  overrideOwn: number
}

/** `derive::VersionNode` */
export interface VersionNode {
  uid: string
  versionId: string
  name: string
  tag: string | null
  isNew: boolean
  archived: boolean
  own: number
  total: number
  build: BuildState
  bbsSource: BbsSource
  bbsCount: number
  recipeEmpty: boolean
  lastBuild: string | null
  orphanKeys: string[]
}

/** `derive::MachineNode` */
export interface MachineNode {
  id: string
  display: string
  icon: string | null
  own: number
  total: number
  build: BuildState
  dimensionsMissing: boolean
  versions: VersionNode[]
}

/** `derive::VersionBrief` */
export interface VersionBrief {
  uid: string
  machineId: string
  name: string
}

/** `derive::BuildRow` */
export interface BuildRow {
  uid: string
  machineId: string
  machine: string
  name: string
  state: BuildState
  reason: string
  buildable: boolean
  disabledReason: string | null
  mkpFile: string | null
  bbsCount: number
  lastBuild: string | null
}

/** `derive::BookView` */
export interface BookView {
  machines: MachineNode[]
  archived: VersionBrief[]
  badges: Badges
  dirtyCount: number
  save: SaveState
  artifact: ArtifactState
  lastBuild: string | null
  buildRows: BuildRow[]
  notices: string[]
}

/* ---------- 字段定义 ---------- */

/** `registry::SectionMeta` */
export interface SectionMeta {
  id: string
  label: string
  description: string | null
  order: number
}

/** `registry::TabMeta` —— 已过滤成「只留装参数的」 */
export interface TabMeta {
  id: string
  label: string
  order: number
  icon: string | null
  sections: SectionMeta[]
}

/** `app::ChoiceView` */
export interface ChoiceView {
  label: string
  value: unknown
  deprecated: boolean
}

/** `registry::ShowWhen` */
export interface ShowWhen {
  key: string
  op: 'eq' | 'neq' | 'gt'
  value: unknown
}

/** `app::ParamView` */
export interface ParamView {
  key: string
  label: string
  desc: string
  tomlKey: string
  sectionId: string
  tabId: string | null
  order: number
  valueType: ValueType
  uiComponent: UiComponent
  defaultValue: unknown
  /** 已格式化好的显示文本。**前端不再格式化一遍** */
  defaultText: string
  min: number | null
  max: number | null
  step: number | null
  unit: string | null
  choices: ChoiceView[]
  showWhen: ShowWhen | null
  parentKey: string | null
  /** 空 = 不限机型 */
  machineFilter: string[]
  deprecated: boolean
}

/** `app::RegistryView` */
export interface RegistryView {
  updated: string
  tabs: TabMeta[]
  params: ParamView[]
}

/* ---------- 矩阵 ---------- */

/** `derive::ColRef` —— 前端勾了什么。**顺序无所谓**，后端按配方本重排 */
export interface ColRef {
  machineId: string
  versionUid: string | null
}

/** `derive::Col` */
export interface Col {
  key: string
  machineId: string
  versionUid: string | null
  level: Level
  machine: string
  label: string
  own: number
}

/** `visibility::BlockedBy` —— `need` 是后端拼好的整句 */
export interface BlockedBy {
  key: string
  label: string
  need: string
  scope: BlockScope
}

/** `derive::Cell` */
export interface Cell {
  kind: CellKind
  text: string
  lines: number | null
  origin: Origin | null
  originLabel: string | null
  originExplain: string | null
  own: boolean
  dirty: boolean
  editable: boolean
  reason: string | null
  /** 根在前 */
  blocked: BlockedBy[]
  /** 原始值。受控控件用它，不能拿格式化过的文本回填 */
  raw: unknown
}

/** `derive::Row` */
export interface Row {
  key: string
  label: string
  desc: string
  unit: string | null
  sectionId: string
  tabId: string | null
  gcode: boolean
  deprecated: boolean
  cells: Cell[]
}

/** `derive::Matrix` */
export interface Matrix {
  cols: Col[]
  rows: Row[]
  totalRows: number
  note: string | null
  /** 空的时候写出为什么空，不留白 */
  emptyReason: string | null
}

/* ---------- 仓库盘点 / 回退 / 回收站 ---------- */

/** `derive::StockRow` */
export interface StockRow {
  id: string
  resourceType: ResourceType
  machineId: string | null
  fileName: string
  relativePath: string
  sha256: string
  size: number
  updatedAt: string
  nozzle: string | null
  layerHeight: string | null
  assign: BbsAssign
  /** **与 assign 正交**：可以已分配、同时不属于任何套餐 */
  inAnyBundle: boolean
  visibility: Visibility
}

/** `fallback::Rule` */
export interface FallbackRule {
  id: string
  category: 'default' | 'infer' | 'migration' | 'override' | 'recovery'
  trigger: string
  from: string
  to: string
  enabled: boolean
  severity: 'info' | 'warn'
  desc: string
  reportField: string
}

/** `app::FallbackGroup` */
export interface FallbackGroup {
  label: string
  rules: FallbackRule[]
}

/** `app::FallbackTable` */
export interface FallbackTable {
  version: number
  updated: string
  /** 带换行的长文，**原样显示** */
  guide: string
  groups: FallbackGroup[]
  disabled: string[]
  emptyHint: string
  readOnlyReason: string
}

/** `store::TrashEntry` */
export interface TrashEntry {
  file: string
  deletedStamp: string
  machineId: string
  versionId: string
}

/* ---------- 预览 ---------- */

/** `preview::InheritedChange` */
export interface InheritedChange {
  key: string
  label: string
  before: string
  after: string
  beforeOrigin: Origin
  afterOrigin: Origin
}

/** `preview::LostKey` */
export interface LostKey {
  key: string
  label: string
  /** 为真时这个值搬过去会变成再也进不了产物的孤儿 */
  hadOwnValue: boolean
}

/** `preview::MovePreview` —— 四组分开摆 */
export interface MovePreview {
  uid: string
  fromMachine: string
  toMachine: string
  allowed: boolean
  blockedReason: string | null
  kept: string[]
  inheritedChanges: InheritedChange[]
  gained: string[]
  lost: LostKey[]
}

/** `preview::BulkEffect` */
export interface BulkEffect {
  col: string
  machine: string
  label: string
  level: Level
  before: string
  after: string
  kind: BulkKind
}

/** `preview::BulkSkip` */
export interface BulkSkip {
  col: string
  machine: string
  label: string
  reason: string
  blocked: BlockedBy[]
}

/** `preview::BulkPreview`。`effects.length` 就是「N 列」那个 N */
export interface BulkPreview {
  key: string
  label: string
  allowed: boolean
  blockedReason: string | null
  effects: BulkEffect[]
  skipped: BulkSkip[]
}

/** `app::DiffLine` */
export interface DiffLine {
  target: string
  level: Level | null
  owner: string
  key: string | null
  label: string
  before: string
  after: string
  kind: string
}

/* ---------- 写 ---------- */

/**
 * `domain::Patch`。**`value: null` = 删键 = 挂回继承**，不是「值设成空」。
 *
 * `kind` 是 serde 的内部标签，所以这里也用它做可辨识联合
 */
export type Patch =
  | { kind: 'setValue'; level: Level; owner: string; key: string; value: unknown | null }
  | { kind: 'cloneVersion'; fromUid: string; name: string }
  | { kind: 'newVersion'; machineId: string; name: string }
  | { kind: 'renameVersion'; uid: string; name: string }
  | { kind: 'moveVersion'; uid: string; toMachineId: string }
  | { kind: 'archiveVersion'; uid: string }
  | { kind: 'restoreVersion'; uid: string }
  | { kind: 'purgeVersion'; uid: string }
  | { kind: 'setBbs'; uid: string; list: string[] | null }
  | { kind: 'setVisibility'; fileId: string; visibility: Visibility }
  | { kind: 'setBundle'; bundleId: string; presets: string[]; bbs: string[] }
  | {
      kind: 'markBuilt'
      uids: string[]
      stamp: string
      fingerprints: Record<string, string>
    }

/** `app::ApplyResult` */
export interface ApplyResult {
  view: BookView
  /** 撤销这次操作要提交的 patches，**倒序**。为空时配合 `undoable=false` */
  inverse: Patch[]
  /** 为 false 时界面**不给**撤销按钮 —— 删除与生成记录不进撤销栈 */
  undoable: boolean
  notices: string[]
}

/** `app::SaveResult` */
export interface SaveResult {
  view: BookView
  /** 旧 uid → 新 uid。**新建与移动都会改 uid**，选中与勾选列要据此修 */
  remap: Record<string, string>
  notices: string[]
}

/* ---------- 状态词 ---------- */

/** `words::Word`。`explain` 写「改了会怎样」，不是「这个状态怎么算的」 */
export interface Word {
  label: string
  explain: string | null
}

/**
 * `words::Words` —— **状态词的唯一出处**。
 *
 * 键就是枚举序列化出来的那个值，所以 `words.build[node.build].label` 直接能用。
 * 前端**不许**再写一份 `{ stale: '待生成' }`：那样 Rust 改了措辞界面还是老词，
 * 而且没有任何东西会报错。
 */
export interface Words {
  build: Record<BuildState, Word>
  artifact: Record<ArtifactState, Word>
  save: Record<SaveState, Word>
  bbsAssign: Record<BbsAssign, Word>
  bbsSource: Record<BbsSource, Word>
  origin: Record<Origin, Word>
  level: Record<Level, Word>
  visibility: Record<Visibility, Word>
  bulkKind: Record<BulkKind, Word>
  placeholder: Record<
    'blank' | 'notApplicable' | 'undeclared' | 'unconfigured' | 'unsupported',
    string
  >
  disabled: Record<
    | 'detachNothing'
    | 'detachReady'
    | 'blockedByCondition'
    | 'notApplicable'
    | 'bulkRefusesGcode'
    | 'buildBlocked'
    | 'buildNothingToDo'
    | 'buildNoResources'
    | 'nothingToSave'
    | 'nothingToUndo'
    | 'notUndoable',
    string
  >
  empty: Record<
    | 'noIssues'
    | 'trashEmpty'
    | 'noDisabledFallback'
    | 'matrixNoMatch'
    | 'matrixNoCols'
    | 'matrixSearchSpansAllTabs',
    string
  >
}

/* ---------- 命令 ---------- */

export const wb = {
  open: () => invoke<void>('wb_open'),
  boot: () => invoke<Boot>('wb_boot'),
  reload: () => invoke<Boot>('wb_reload'),
  words: () => invoke<Words>('wb_words'),


  book: () => invoke<BookView>('wb_book'),
  registry: () => invoke<RegistryView>('wb_registry'),
  matrix: (cols: ColRef[], tab: string | null, query: string) =>
    invoke<Matrix>('wb_matrix', { cols, tab, query }),
  stock: () => invoke<StockRow[]>('wb_stock'),
  fallback: () => invoke<FallbackTable>('wb_fallback'),
  trash: () => invoke<TrashEntry[]>('wb_trash'),
  ui: () => invoke<Record<string, unknown>>('wb_ui'),
  saveUi: (ui: Record<string, unknown>) => invoke<void>('wb_save_ui', { ui }),

  previewMove: (uid: string, toMachineId: string) =>
    invoke<MovePreview>('wb_preview_move', { uid, toMachineId }),
  previewBulk: (key: string, value: unknown, cols: ColRef[]) =>
    invoke<BulkPreview>('wb_preview_bulk', { key, value, cols }),
  diffDraft: () => invoke<DiffLine[]>('wb_diff_draft'),

  /**
   * **唯一的写入口。** `label` 是给撤销按钮显示的一句话
   * （`撤销：A1 基底 · X 轴偏移`），所以它必须是人话，不是命令名
   */
  applyDraft: (label: string, patches: Patch[]) =>
    invoke<ApplyResult>('wb_apply_draft', { label, patches }),
  save: () => invoke<SaveResult>('wb_save'),
  discard: () => invoke<BookView>('wb_discard'),
}
