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

/**
 * `derive::Badges`。
 *
 * 以前这里要分开数「一共几项」与「其中我们自己写了几项」（`baseOwn` / `overrideOwn`）：
 * 那时候每层各有两半。b04 Task 12 之后每层只有一张 `machineVariants` 拆出来的表，
 * 两个数是同一个 —— 于是只留 `items`
 */
export interface Badges {
  machines: number
  versions: number
  baseItems: number
  overrideItems: number
}

/** `derive::VersionNode`。`items` = 版本层这一版钉着几项值 */
export interface VersionNode {
  uid: string
  versionId: string
  name: string
  tag: string | null
  items: number
  build: BuildState
  bbsSource: BbsSource
  bbsCount: number
  recipeEmpty: boolean
  lastBuild: string | null
  orphanKeys: string[]
}

/** `derive::MachineNode`。`items` = 机型层这台钉着几项值 */
export interface MachineNode {
  id: string
  display: string
  icon: string | null
  items: number
  build: BuildState
  dimensionsMissing: boolean
  versions: VersionNode[]
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
  badges: Badges
  dirtyCount: number
  save: SaveState
  artifact: ArtifactState
  lastBuild: string | null
  buildRows: BuildRow[]
  notices: string[]
  /** `wording::SnapshotState` —— 崩溃快照跟上了没有。**与 `save` 是两件事** */
  snapshot: SnapshotState
}

/** `wording::SnapshotState` */
export type SnapshotState = 'current' | 'pending' | 'failed'

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
  /** 这一列那一层钉着几项值 */
  items: number
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
  /** 点开灰格子时显示的整句。**后端拼好的**，前端不组装 */
  blockedNote: string | null
  /** 「去改那一项」跳到哪个字段。`null` = 不给跳转按钮 */
  jumpTo: string | null
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
  /** 组的中文名。**与上一行不同就插一条分组表头** —— 分组是后端排出来的 */
  sectionLabel: string
  tabId: string | null
  /** 0 = 顶层，1 = 子项。只有两级 */
  depth: number
  parentKey: string | null
  parentLabel: string | null
  /** 「属于：X」 */
  parentNote: string | null
  /** 「受「X」控制」/「整组由「X」控制」。**与当前值无关**，灰之前就在 */
  controlNote: string | null
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

/* ---------- 配方台（默认视角） ---------- */

/** `derive::Desk` —— 一个版本的分组列表 */
export interface Desk {
  /** 左栏。**不随搜索变** */
  nav: DeskNavTab[]
  groups: DeskGroup[]
  /** 过滤前一共几项 */
  total: number
  note: string | null
  emptyReason: string | null
}

/** `derive::DeskNavTab` */
export interface DeskNavTab {
  id: string
  label: string
  count: number
  sections: DeskNavSection[]
}

/** `derive::DeskNavSection` */
export interface DeskNavSection {
  id: string
  label: string
  count: number
}

/** `derive::DeskGroup` */
export interface DeskGroup {
  sectionId: string
  label: string
  count: number
  /** 整组被 section 级条件关掉时的那一句。界面据此把整组收起来 */
  offNote: string | null
  items: DeskItem[]
}

/** `derive::DeskItem` —— 一项 + 挂在它下面的子项。**只有两级** */
export interface DeskItem {
  row: Row
  children: Row[]
  /** 这一项把自己下面那几个关掉了时的那一句 */
  offNote: string | null
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

/* 移动预览（`MovePreview` / `InheritedChange` / `LostKey`）整组删了（b04 Task 12）：
   「把一个版本搬到另一台机型」现在归「机型与版本」页，`Patch::MoveVersion`
   与 `wb_preview_move` 一起没有了。剩下的只读推演只有批量（`BulkPreview`）。 */

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
 * `kind` 是 serde 的内部标签，所以这里也用它做可辨识联合。
 *
 * b04 Task 12 之后**只剩这四种**：改名 / 归档 / 挑 BBS 与新建 / 克隆 / 移动 / 删除版本
 * 全部归「机型与版本」页即时落盘（REPORT §7），草稿里因此不再有结构手势 ——
 * **撤销栈只服务值编辑**
 */
export type Patch =
  | { kind: 'setValue'; level: Level; owner: string; key: string; value: unknown | null }
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
  /** 为 false 时界面**不给**撤销按钮 —— 生成记录不进撤销栈 */
  undoable: boolean
  /** 顺带带回来的那一页。**省掉 apply 之后再问一次**（一次手势一次派生） */
  desk: Desk | null
  matrix: Matrix | null
}

/**
 * `app::Refresh` —— 写完顺带刷哪一页。
 *
 * 注意 `page` 是判别字段（Rust 那边 `tag = "page"`）
 */
export type Refresh =
  | { page: 'desk'; machineId: string; uid?: string | null; tab?: string | null; query?: string }
  | { page: 'matrix'; cols: ColRef[]; tab?: string | null; query?: string }

/** `app::SaveResult` */
export interface SaveResult {
  view: BookView
  /**
   * 旧 uid → 新 uid。b04 Task 12 之后**永远是空的** —— 会改 uid 的那两条
   * （新建、移动版本）都归「机型与版本」页了。留着是为了不动这个返回值的形状
   */
  remap: Record<string, string>
}

/* ---------- 校验三档 ---------- */

/** `issues::Severity`。分档的判据是「挡不挡生成」，不是「严不严重」 */
export type Severity = 'block' | 'todo' | 'hint'

/** `issues::View` —— 去哪儿处理 */
export type IssueView = 'params' | 'menu' | 'build' | 'fields' | 'stock' | 'fallback'

/** `issues::Where`。一条说不清去哪儿的问题**等于没报** */
export interface IssueWhere {
  view: IssueView
  machineId: string | null
  uid: string | null
  key: string | null
}

/** `issues::Issue` */
export interface Issue {
  id: string
  severity: Severity
  title: string
  /** 「怎么办」。不是把标题换个说法重复一遍 */
  detail: string
  at: IssueWhere
}

/** `issues::Report`。`blocks > 0` = 生成该全禁用 */
export interface IssueReport {
  issues: Issue[]
  blocks: number
  todos: number
  hints: number
  /** 零问题时的那一句。**不留白** */
  emptyHint: string
}

/* ---------- 生成 / 恢复 / 发布 ---------- */

/** `build::Scope` —— 三个生成入口 */
export type BuildScope = 'stale' | 'all' | { picked: string[] }

/** `build::GenerateReport` */
export interface GenerateReport {
  stamp: string
  written: string[]
  /** 算出来和现在的文件一模一样，所以没重写。**要显式说出来** ——
   *  不说的话「点了生成但文件时间没变」看起来像失败了 */
  unchanged: string[]
  skipped: [string, string][]
  /** 生成记录要走 `applyDraft` 落进草稿 */
  mark: Patch
}

/** `build::RevertChange` */
export interface RevertChange {
  key: string
  label: string
  before: string
  after: string
}

/** `build::RevertPreview` —— **只算不写**，patches 交给 applyDraft */
export interface RevertPreview {
  uid: string
  allowed: boolean
  blockedReason: string | null
  patches: Patch[]
  changes: RevertChange[]
  /** 原本继承来的、会从此脱钩的那几项。**不许闷着改** */
  detaching: string[]
}

/** `build::PublishReport` */
export interface PublishReport {
  stamp: string
  root: string
  files: number
  /** null = 上游未声明。**不编一个版本号出来** */
  minimumClient: string | null
  todos: number
  hints: number
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
  /**
   * 关联那一组里**不带变量**的那几句。
   * 带变量的整句在后端就拼好了（`Row.controlNote` / `Cell.blockedNote`）——
   * 前端不拿模板填空，模板一分两处迟早分岔
   */
  relate: Record<'goFixIt' | 'showAnyway', string>
  /** 崩溃快照三态。**与 `save` 不是一回事** */
  snapshot: Record<SnapshotState, Word>
}

/* ---------- 命令 ---------- */

/* ---------- 机型与版本（`app::machines`） ---------- */

/** `machines::BrandView` */
export interface BrandView {
  id: string
  name: string
  logo: string | null
}

/** `machines::VersionView` —— 版本卡上那六格 */
export interface VersionView {
  id: string
  name: string
  presetFile: string | null
  recommendedBundle: string | null
  tag: string | null
  description: string | null
}

/** `machines::MachineView` */
export interface MachineView {
  id: string
  /** 人看的名字。实测有机型的 `name` 是空串而 `display` 才是给人看的 */
  display: string
  name: string
  brand: string
  defaultBundle: string | null
  externalAliases: string[]
  image: string | null
  icon: string | null
  /** 有没有 `[dimensions]`。A2L 实测没有 */
  hasDimensions: boolean
  /** 禁区块数。0 = 这台没有禁区文件 */
  zoneCount: number
  versions: VersionView[]
  /** 它自己那个 toml 文件名（`A1.toml`）。**给人看的**，让「我在改哪个文件」不用猜 */
  file: string
}

/** `machines::MachineList` */
export interface MachineList {
  brands: BrandView[]
  machines: MachineView[]
  /** 数据根的绝对路径 */
  root: string
}

/* ---------- 资产库（`app::assets` / `presets::assets`） ---------- */

/**
 * `presets::AssetKind` —— 资产类型集合（闸 G-3：切片器维度**开放**，模型保留）。
 *
 * 四个取值而不是三个：`image` 与 `icon` 是两种消费方式（一个是机型图、一个是矢量标记）。
 * **没有 `mkpPreset`** —— 那份路径由命名规则算出，不建条目（doc §12.5）。
 */
export type AssetKind = 'image' | 'icon' | 'model' | 'slicerProfile'

/** `assets::AssetView` —— 资产域①层的一条定义（`presets/assets.toml`） */
export interface AssetView {
  id: string
  kind: AssetKind
  /** 归属机型；不属于任何机型时是 null */
  machineId: string | null
  name: string
  /** 相对资产根（`public/assets/`）的一段 */
  path: string
  /** `/assets/<path>`。**用之前过 `assetUrl()`** —— 路径里可能有空格 */
  url: string
  /** 切片器（今天只有 `bbs`）与它下面的档位；只有 `slicerProfile` 才有 */
  slicer: string | null
  profile: string | null
  /** 文件在不在。**Task 9 之前普遍 false** —— 那是还没搬，不是错 */
  present: boolean
}

/** `assets::AssetList` */
export interface AssetList {
  assets: AssetView[]
  /** 资产根的绝对路径 */
  root: string
}

/** `assets::AssetUsageView` —— 「谁在用它」（Task 9.4，套餐那一档 Task 10） */
export interface AssetUsageView {
  id: string
  /** 直接引用它的机型 */
  machines: string[]
  /** 引用它的套餐（`assetRefs` 写着这个 id 的）。**归属不是引用** ——
   *  `p1s-icon` 归 P1S，但借它当图标的是另外两台 */
  bundles: string[]
}

/** `bundles::BundleRefView` —— 套餐里一个 `assetRef` 的解析状态（b05 Task 10） */
export interface BundleRefView {
  id: string
  /** 能不能解析到一条真实资产。加载期解析不到是 error，真数据上恒 true */
  resolvable: boolean
  /** 是不是 BBS 预设。每条套餐至少一条 true（MKP 与 BBS 成套配发） */
  isBbs: boolean
}

/** `bundles::BundleView` —— 套餐域①层的一条定义（`presets/bundles.toml`） */
export interface BundleView {
  id: string
  display: string
  machineId: string
  assetRefs: BundleRefView[]
  /** 上一次改动日期（迁移照抄旧值，不写「搬运日」） */
  updatedAt: string | null
}

/** `bundles::BundleList` */
export interface BundleList {
  bundles: BundleView[]
}

/**
 * 资产 URL。后端给的前缀只有一处（`/assets/`），这里只负责**编码一次** ——
 * 实测 BBS 文件名里有空格（`MKPProcess A1 0.2 0.10.json`）。
 */
export const assetUrl = (url: string) => encodeURI(url)

/** `catalog::VersionField` —— 版本身上可改的那几格。`id` 不在里面（改 ID = 删+加） */
export type VersionField = 'name' | 'presetFile' | 'recommendedBundle' | 'tag' | 'description'

/** `catalog::MachineField` —— 机型身上可改的那几格。`id` 不在里面（它是文件名） */
export type MachineField = 'display' | 'brand' | 'name' | 'image' | 'icon'

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

  /**
   * 机型与版本清单。**这一页唯一的读入口**，不走 `wb_apply_draft` ——
   * 清单与参数值不共用状态机
   */
  machines: () => invoke<MachineList>('wb_machines'),

  /**
   * 资产库清单（**只读**）。条目来自 `presets/assets.toml`，文件在 `public/assets/` 下。
   * 现在普遍 `present: false` —— 条目与文件一起在 b05 Task 9 落地
   */
  assets: () => invoke<AssetList>('wb_assets'),

  /**
   * 套餐清单（**只读**，b05 Task 10）。条目来自 `presets/bundles.toml`，
   * `defaultBundle` / `recommendedBundle` 引用的就是这里的 `id`
   */
  bundles: () => invoke<BundleList>('wb_bundles'),

  /**
   * 「谁在用它」。**删资产之前先问这一条** —— 删掉一张还被机型引用着的图，
   * 界面上只表现为「那台机型的图没了」。删除守卫在数据层（`Presets::remove_asset`）
   */
  assetUsage: (assetId: string) => invoke<AssetUsageView>('wb_asset_usage', { assetId }),

  /**
   * 加一台机型 = **新建一个 `presets/machines/{ID}.toml`**。
   * 已存在的文件**绝不覆盖**（后端用 `create_new` 原子地占路径）
   */
  addMachine: (id: string, brand: string, display: string) =>
    invoke<MachineList>('wb_add_machine', { id, brand, display }),

  /**
   * 改版本的一格。`value = null` = **清空**，而清空在文件里是**删掉那一行**，
   * 不是写 `tag = ''` —— 后者读成「填过，填了个空」，和「还没填」是两件事
   */
  setVersionField: (
    machineId: string,
    versionId: string,
    field: VersionField,
    value: string | null,
  ) => invoke<MachineList>('wb_set_version_field', { machineId, versionId, field, value }),

  /** 改机型自己的一格。`display` / `brand` 不许清空 */
  setMachineField: (machineId: string, field: MachineField, value: string | null) =>
    invoke<MachineList>('wb_set_machine_field', { machineId, field, value }),

  /**
   * 删这个版本会让哪些字段留下孤儿引用。**删之前先问这一条。**
   * 返回的是字段 key（`wiping.wiper_x` 这种）—— 那是用户能据以行动的单位
   */
  versionOrphans: (machineId: string, versionId: string) =>
    invoke<string[]>('wb_version_orphans', { machineId, versionId }),

  /** 删一个版本。**不可逆**，所以界面上是两步确认 */
  removeVersion: (machineId: string, versionId: string) =>
    invoke<MachineList>('wb_remove_version', { machineId, versionId }),

  /**
   * 加一个版本。**立刻落盘，没有草稿也没有撤销** ——
   * 逆操作是「删掉那个版本」，所以不为它建一套中间态。
   * 返回的是**重读盘之后**的清单，界面看到的就是落盘的结果
   */
  addVersion: (machineId: string, id: string, name: string) =>
    invoke<MachineList>('wb_add_version', { machineId, id, name }),

  /** 默认视角：一个版本的分组列表 */
  desk: (machineId: string, uid: string | null, tab: string | null, query: string) =>
    invoke<Desk>('wb_desk', { machineId, uid, tab, query }),
  previewBulk: (key: string, value: unknown, cols: ColRef[]) =>
    invoke<BulkPreview>('wb_preview_bulk', { key, value, cols }),
  diffDraft: () => invoke<DiffLine[]>('wb_diff_draft'),

  /**
   * **唯一的写入口。** `label` 是给撤销按钮显示的一句话
   * （`撤销：A1 基底 · X 轴偏移`），所以它必须是人话，不是命令名。
   *
   * `refresh` 说「顺带把哪一页给我」：给了就不用在这之后再问一次 ——
   * 一次手势一次 IPC、后端一次派生
   */
  applyDraft: (label: string, patches: Patch[], refresh?: Refresh) =>
    invoke<ApplyResult>('wb_apply_draft', { label, patches, refresh: refresh ?? null }),
  save: () => invoke<SaveResult>('wb_save'),
  discard: () => invoke<BookView>('wb_discard'),

  preflight: () => invoke<IssueReport>('wb_preflight'),
  previewToml: (uid: string) => invoke<string>('wb_preview_toml', { uid }),
  generate: (scope: BuildScope) => invoke<GenerateReport>('wb_generate', { scope }),
  revertPreview: (uid: string) => invoke<RevertPreview>('wb_revert_preview', { uid }),
  publish: () => invoke<PublishReport>('wb_publish'),

  /**
   * 交付目录的残留清单（b05 Task 13.4）：「不在本次交付集合内」的文件。
   * **发布被残留拦下时先看这一条** —— 残留会被消费端真的下载到
   */
  distStrays: () => invoke<string[]>('wb_dist_strays'),
  /**
   * 清理残留（b05 Task 13.5）：走 `workbench/.trash/dist/` 回收（保留相对路径），
   * **不直接删**。清理完重新发布即可
   */
  cleanDistStrays: () => invoke<number>('wb_clean_dist_strays'),
}
