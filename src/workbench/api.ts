/**
 * 工作台与 Rust 侧之间的唯一通道。
 *
 * 与客户端的 src/api/ 刻意分开：那一套有 mock / bridge 双实现（界面要能在浏览器里单独跑），
 * 工作台不需要 —— 它只在 Tauri 窗口里跑，没有"脱壳演示"的场景，多一层 mock 只会多一份
 * 会漂移的假数据。
 *
 * 类型是 Rust 那边 `serde(rename_all = "camelCase")` 的**手抄镜像**。没有编译器帮忙对照，
 * 所以每个结构上都标了对应的 Rust 类型名，改一边时另一边有迹可循。
 */
import { invoke } from '@tauri-apps/api/core'

/* ---------- 错误 ---------- */

/** 与 src-tauri/src/error.rs 的 AppError 对齐 */
export interface AppError {
  code: string
  message: string
  traceId: string
  detail?: string
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e
}

/** 把任意抛出物变成能显示给人看的一行字。traceId 附在后面 —— 出问题时要能按它去查日志 */
export function describeError(e: unknown): string {
  if (isAppError(e)) {
    return e.traceId && e.traceId !== '-' ? `${e.message}（${e.traceId}）` : e.message
  }
  return String(e)
}

/* ---------- 数据形状 ---------- */

export type ValueType = 'float' | 'int' | 'bool' | 'text' | 'gcode'

/** model.rs FieldDef */
export interface FieldDef {
  key: string
  label: string
  desc: string
  valueType: ValueType
  unit?: string
  min?: number
  max?: number
  step?: number
  section: string
  tomlKey: string
  order: number
}

/** model.rs Registry */
export interface Registry {
  schemaVersion: number
  fields: FieldDef[]
}

export type ParamValue = string | number | boolean
export type Params = Record<string, ParamValue>

/** model.rs Machine */
export interface Machine {
  id: string
  displayName: string
  base: Params
  defaultBbs: string[]
}

/** model.rs BbsBinding */
export type BbsBinding = { mode: 'inherit' } | { mode: 'own'; ids: string[] }

/** model.rs Version */
export interface Version {
  id: string
  displayName: string
  machineId: string
  overrides: Params
  bbs: BbsBinding
}

/** resolve.rs ValueOrigin */
export interface ValueOrigin {
  value: ParamValue
  origin: 'base' | 'override'
  /** 只有 origin === 'override' 时才有：清除覆盖后会回到这个值 */
  baseValue?: ParamValue
}

/** resolve.rs Effective */
export interface Effective {
  values: Record<string, ValueOrigin>
  /** 覆盖了、但当前机型基底没有这个字段。保留不丢，但不进产物 */
  orphans: string[]
  /** 覆盖里出现了字段定义里没有的 key */
  unknownKeys: string[]
  hash: string
}

/** mod.rs EffectiveView */
export interface EffectiveView {
  machine: Machine
  version: Version
  effective: Effective
  unconfigured: boolean
  machineHash: string
}

/** mod.rs MachineNode */
export interface MachineNode {
  id: string
  displayName: string
  baseFieldCount: number
  versions: { id: string; displayName: string }[]
}

/** mod.rs Roots */
export interface Roots {
  workbench: string
  dist: string
}

/** store.rs BootstrapReport */
export interface BootstrapReport {
  wroteRegistry: boolean
  wroteFallback: boolean
  hasMachines: boolean
}

/** store.rs Draft */
export interface Draft {
  baseHash: string
  overrides: Params
  savedAt: string
}

/** mod.rs DraftView */
export interface DraftView {
  draft: Draft | null
  /** 草稿是在当前这份配方上写的吗。false = 配方在写草稿之后被改过 */
  matchesCurrent: boolean
  currentHash: string
}

/** ops.rs MoveKind */
export type MoveKind =
  | 'inheritedChanged'
  | 'overrideKept'
  | 'becomesInapplicable'
  | 'becomesApplicable'

/** ops.rs MoveChange */
export interface MoveChange {
  key: string
  label: string
  kind: MoveKind
  from?: ParamValue
  to?: ParamValue
}

/** ops.rs MovePreview */
export interface MovePreview {
  fromMachine: string
  toMachine: string
  changes: MoveChange[]
  nameTaken: boolean
}

/** ops.rs TrashResult */
export interface TrashResult {
  trashedFile: string
  detachedPresets: string[]
}

/** store.rs TrashEntry */
export interface TrashEntry {
  file: string
  deletedStamp: string
  machineId: string
  versionId: string
}

/** matrix.rs MatrixColumn */
export interface MatrixColumn {
  machineId: string
  versionId: string
  machineName: string
  versionName: string
  unconfigured: boolean
}

/** matrix.rs MatrixCell。origin 缺失 = 该机型不适用 */
export interface MatrixCell {
  value?: ParamValue
  origin?: 'base' | 'override'
}

/** matrix.rs MatrixRow */
export interface MatrixRow {
  key: string
  label: string
  section: string
  unit?: string
  cells: MatrixCell[]
}

/** matrix.rs Matrix */
export interface Matrix {
  columns: MatrixColumn[]
  rows: MatrixRow[]
}

/** matrix.rs BulkTarget */
export type BulkTarget =
  | { kind: 'versionOverride'; machineId: string; versionId: string }
  | { kind: 'machineBase'; machineId: string }

/** matrix.rs BulkKind */
export type BulkKind =
  | 'createsOverride'
  | 'updatesOverride'
  | 'updatesBase'
  | 'noChange'
  | 'notApplicable'

/** matrix.rs BulkEffect */
export interface BulkEffect {
  target: BulkTarget
  label: string
  before?: ParamValue
  after: ParamValue
  kind: BulkKind
}

/** store.rs GenFailure */
export interface GenFailure {
  at: string
  reason: string
}

/** state.rs GenState */
export type GenState = 'generated' | 'stale' | 'unconfigured'

/** state.rs VersionStatus */
export interface VersionStatus {
  machineId: string
  versionId: string
  machineName: string
  versionName: string
  presetId: string
  state: GenState
  lastGeneratedAt?: string
  lastFailure?: GenFailure
  minClientVersion?: string
  outputPresent: boolean
  outputMatches: boolean
  orphanCount: number
}

/** generate.rs GenOutcome */
export interface GenOutcome {
  machineId: string
  versionId: string
  presetId: string
  outputRel: string
  sha256: string
  size: number
  minClientVersion: string
  unsupported: { clientVersion: string; fieldKey?: string; reason: string }[]
  generatedAt: string
  unchanged: boolean
}

/** mod.rs GenReport */
export interface GenReport {
  generated: GenOutcome[]
  failed: { machineId: string; versionId: string; reason: string }[]
  unchangedCount: number
}

/** state.rs RestoreReport */
export interface RestoreReport {
  restoredOverrides: number
  baseDiffers: string[]
  snapshotGeneratedAt: string
}

/** bbs.rs BbsState / BbsFile */
export type BbsState = 'assigned' | 'optional' | 'archivedOnly'

export interface BbsFile {
  id: string
  size: number
  sha256: string
  parses: boolean
  usedBy: string[]
  state: BbsState
}

/** catalog.rs PresetEntry */
export interface PresetEntry {
  presetId: string
  machine: string
  version: string
  displayName: string
  resource: string
  sha256?: string
  size?: number
  minClientVersion?: string
  standalone: boolean
}

/** catalog.rs BbsEntry */
export interface BbsEntry {
  bbsId: string
  displayName: string
  resource: string
  sha256?: string
  size?: number
  minClientVersion?: string
  offering: 'assigned' | 'optional'
}

/** catalog.rs Catalog */
export interface Catalog {
  catalogSchemaVersion: number
  presets: PresetEntry[]
  bbs: BbsEntry[]
  generatedAt?: string
}

/** catalog.rs Bundle / Bundles */
export interface Bundle {
  bundleId: string
  displayName: string
  presets: string[]
  bbs: string[]
  minClientVersion?: string
}

export interface Bundles {
  bundles: Bundle[]
}

/** preflight.rs Finding / PreflightReport */
export interface Finding {
  severity: 'blocking' | 'warning'
  category: 'recipe' | 'reference' | 'compatibility' | 'inventory'
  message: string
  detail?: string
}

export interface PreflightReport {
  findings: Finding[]
  blocking: number
  warnings: number
  canPublish: boolean
}

/** publish.rs PublishReport */
export interface PublishReport {
  presetCount: number
  bbsCount: number
  bundleCount: number
  publishedAt: string
  files: string[]
}

/* ---------- 命令 ---------- */

export const wb = {
  roots: () => invoke<Roots>('wb_roots'),
  bootstrap: () => invoke<BootstrapReport>('wb_bootstrap'),
  registry: () => invoke<Registry>('wb_registry'),
  tree: () => invoke<MachineNode[]>('wb_tree'),
  effective: (machineId: string, versionId: string) =>
    invoke<EffectiveView>('wb_effective', { machineId, versionId }),
  saveVersion: (args: {
    machineId: string
    versionId: string
    displayName: string
    overrides: Params
    expectedHash: string
  }) => invoke<EffectiveView>('wb_save_version', args),
  saveMachineBase: (args: {
    machineId: string
    base: Params
    expectedMachineHash: string
  }) => invoke<Machine>('wb_save_machine_base', args),

  draft: (machineId: string, versionId: string) =>
    invoke<DraftView>('wb_draft', { machineId, versionId }),
  saveDraft: (args: {
    machineId: string
    versionId: string
    baseHash: string
    overrides: Params
  }) => invoke<void>('wb_save_draft', args),
  clearDraft: (machineId: string, versionId: string) =>
    invoke<void>('wb_clear_draft', { machineId, versionId }),

  baseDraft: (machineId: string) => invoke<DraftView>('wb_base_draft', { machineId }),
  saveBaseDraft: (args: { machineId: string; baseHash: string; overrides: Params }) =>
    invoke<void>('wb_save_base_draft', args),
  clearBaseDraft: (machineId: string) => invoke<void>('wb_clear_base_draft', { machineId }),

  createMachine: (id: string, displayName: string) =>
    invoke<Machine>('wb_create_machine', { id, displayName }),
  createVersion: (machineId: string, id: string, displayName: string) =>
    invoke<Version>('wb_create_version', { machineId, id, displayName }),
  cloneVersion: (args: {
    machineId: string
    versionId: string
    newId: string
    newDisplayName: string
  }) => invoke<Version>('wb_clone_version', args),
  renameVersion: (args: {
    machineId: string
    versionId: string
    newId: string
    newDisplayName: string
  }) => invoke<Version>('wb_rename_version', args),
  previewMove: (machineId: string, versionId: string, toMachineId: string) =>
    invoke<MovePreview>('wb_preview_move', { machineId, versionId, toMachineId }),
  moveVersion: (machineId: string, versionId: string, toMachineId: string) =>
    invoke<Version>('wb_move_version', { machineId, versionId, toMachineId }),
  trashVersion: (machineId: string, versionId: string) =>
    invoke<TrashResult>('wb_trash_version', { machineId, versionId }),
  trashList: () => invoke<TrashEntry[]>('wb_trash_list'),
  restoreFromTrash: (file: string) => invoke<Version>('wb_restore_from_trash', { file }),
  purgeFromTrash: (file: string) => invoke<void>('wb_purge_from_trash', { file }),

  matrix: (machineFilter: string[]) => invoke<Matrix>('wb_matrix', { machineFilter }),
  bulkPreview: (fieldKey: string, value: ParamValue, targets: BulkTarget[]) =>
    invoke<BulkEffect[]>('wb_bulk_preview', { fieldKey, value, targets }),
  bulkApply: (fieldKey: string, value: ParamValue, targets: BulkTarget[]) =>
    invoke<BulkEffect[]>('wb_bulk_apply', { fieldKey, value, targets }),

  viewState: <T>(name: string) => invoke<T | null>('wb_view_state', { name }),
  saveViewState: (name: string, value: unknown) =>
    invoke<void>('wb_save_view_state', { name, value }),

  status: () => invoke<VersionStatus[]>('wb_status'),
  generateStale: () => invoke<GenReport>('wb_generate_stale'),
  generateAll: () => invoke<GenReport>('wb_generate_all'),
  generateOne: (machineId: string, versionId: string) =>
    invoke<GenReport>('wb_generate_one', { machineId, versionId }),
  restoreRecipe: (machineId: string, versionId: string) =>
    invoke<RestoreReport>('wb_restore_recipe', { machineId, versionId }),

  bbsList: () => invoke<BbsFile[]>('wb_bbs_list'),
  bbsImport: (id: string, sourcePath: string) =>
    invoke<BbsFile>('wb_bbs_import', { id, sourcePath }),
  setMachineBbs: (machineId: string, ids: string[]) =>
    invoke<void>('wb_set_machine_bbs', { machineId, ids }),
  setVersionBbs: (machineId: string, versionId: string, binding: BbsBinding) =>
    invoke<void>('wb_set_version_bbs', { machineId, versionId, binding }),

  catalog: () => invoke<Catalog>('wb_catalog'),
  saveCatalog: (value: Catalog) => invoke<void>('wb_save_catalog', { value }),
  bundles: () => invoke<Bundles>('wb_bundles'),
  saveBundles: (value: Bundles) => invoke<void>('wb_save_bundles', { value }),

  preflight: () => invoke<PreflightReport>('wb_preflight'),
  publish: () => invoke<PublishReport>('wb_publish'),
}
