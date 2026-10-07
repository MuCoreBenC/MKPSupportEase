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
/** `wording::AssetIdentity` —— 资产交付身份四态（作者 2026-10-03 定的模型） */
export type AssetIdentity = 'inBundle' | 'optional' | 'bundled' | 'archiveOnly'
/** `wording::BbsSource` */
export type BbsSource = 'own' | 'inheritedFromMachine'
/** `patch::Visibility` */
export type Visibility = 'menu' | 'archiveOnly'
/** `derive::CellKind` —— 三种，不是四种：「选中了升级成真控件」是前端的事 */
export type CellKind = 'notApplicable' | 'gcode' | 'value'
/** `registry::ValueType` */
export type ValueType = 'float' | 'int' | 'bool' | 'string'
/** `registry::UiComponent` —— `text` 是编辑器给的第六种（字符串清掉可选项后的自由文本） */
export type UiComponent = 'number' | 'switch' | 'segmented' | 'select' | 'gcode' | 'text'
/** `visibility::BlockScope` */
export type BlockScope = 'field' | 'section'
/** `preview::BulkKind` */
export type BulkKind = 'detaching' | 'changing' | 'noChange'

/* ---------- 启动 ---------- */

/** `workbench::Roots`。**唯一的数据根是 `presets/`**，没有第二候选、不 fallback */
export interface Roots {
  /** 开发源数据根（仓库里的 `workbench/`） */
  workbench: string
  /** 预设真相源（仓库里的 `presets/`） */
  presets: string
  /** 交付产物目录（仓库里的 `presets/delivery/`） */
  delivery: string
}

/** `app::Boot`。**预设根定位不到时也是一个成功返回**，界面要显示问题与数据根 */
export interface Boot {
  roots: Roots
  /** **真正的开场失败**（今天只剩「presets/ 定位不到」这一种）。null = 业务能跑 */
  problem: string | null
  detail: string | null
  /** 工作台子目录的职责（14.7）—— 谁写谁读、能不能当编辑对象，后端一句话说清 */
  storeDirs: { name: string; role: string }[]
  /**
   * 官方源（Bootstrap）地址：**发布产物发到哪**（`workbench/bootstrap.json`，入库）。
   * null = 还没配 —— 客户端构建时就不会注入默认源（见 `src-tauri/build.rs`）。
   * 改了要**重新构建客户端**才生效（编译期注入）；`wb_set_bootstrap` 写它
   */
  bootstrapUrl: string | null
  /**
   * 第二官方源（Gitee 镜像，国内直连）：与 `bootstrapUrl` 指向**同一份交付**的另一个仓库。
   * null = 没配 —— 客户端构建不注入这一档，客户端界面也不出现它。各读各的，互不兜底
   */
  giteeBootstrapUrl: string | null
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

/** `registry::ShowOp` —— 实测只有三种 */
export type ShowOp = 'eq' | 'neq' | 'gt'

/** `registry::ShowWhen` */
export interface ShowWhen {
  key: string
  op: ShowOp
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

/**
 * `registry::ParamMetaEdit` —— 一次「参数定义」编辑的整包载荷（2026-10-03）。
 * 模态框一次保存改的可能不止一格，所以整包提交；`null` = **清空**那一格
 * （文件里删键），不是「不动」。
 */
export interface ParamMetaEdit {
  label: string
  desc: string
  unit: string | null
  valueType: ValueType
  uiComponent: UiComponent
  defaultValue: unknown
  min: number | null
  max: number | null
  step: number | null
  parentKey: string | null
  showWhen: ShowWhen | null
  deprecated: boolean
  /** 可选项整表 —— 字符串枚举在这里编辑；其他类型原样带回（预设档不丢） */
  choices: ChoiceView[]
  /** 适用机型。空 = 不限机型 */
  machineFilter: string[]
}

/**
 * 参数台「编辑定义」交到外壳的一条（2026-10-03）。外壳把它压进撤销栈：
 * 撤销交 `before`、重做交 `after`，两份都是整包载荷，不用现算反向。
 */
export interface MetaApplied {
  label: string
  key: string
  before: ParamMetaEdit
  after: ParamMetaEdit
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
  /** 行上的短提示（C14）：「要 X 才可改」。与 `blockedNote` 同源不同场合 */
  blockedHint: string | null
  /** 「去改那一项」跳到哪个字段。`null` = 不给跳转按钮 */
  jumpTo: string | null
  /** 对照模式（C14 第四轮）：这一格的值与基准机型基底**不同**（绿底）。基准列恒 false */
  differs: boolean
  /** 差异格的悬停句（「机型基底是 X」）。后端拼好的 */
  diffTip: string | null
  /** 原始值。受控控件用它，不能拿格式化过的文本回填 */
  raw: unknown
  /**
   * 这一层**盘上**钉着的值（草稿不算）。`null` = 这一层没钉着它 ——
   * 「恢复修改前的」要写 `null` 删键、挂回继承（批量抽屉用）
   */
  rest: unknown
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
  /**
   * 「改了影响谁」（C14 抽屉的作用域栏）。**配方台逐行给**；
   * 矩阵的行跨多台机型、答不出「哪一台」，是 null
   */
  impact: DeskImpact | null
  cells: Cell[]
}

/** `derive::DeskImpact` —— 版本层编辑 targets 只有自己，followers 是跟着基底的其它版本 */
export interface DeskImpact {
  /** 「A1 / 标准版」这种，直接可显示 */
  targets: string[]
  followers: string[]
}

/** `derive::Matrix` */
export interface Matrix {
  cols: Col[]
  rows: Row[]
  totalRows: number
  note: string | null
  /** 空的时候写出为什么空，不留白 */
  emptyReason: string | null
  /** 有任一勾选列与基准机型基底**不同**的行（C14 对照）。「仅显示差异」与状态列读它 */
  diffKeys: string[]
  /** 基准机型**没有**的参数行 —— 状态列写「本机无此项」 */
  notOwnKeys: string[]
}

/* ---------- 配方台（默认视角） ---------- */

/** `derive::Desk` —— 一个版本的分组列表（C14：列给全，右栏「各版本取值」直接用） */
export interface Desk {
  /** 左栏。**不随搜索变** */
  nav: DeskNavTab[]
  /**
   * 基底 + 这一机型所有版本，各一列。行的 `cells` 与它**一一对应** ——
   * 右栏「各版本取值」每层一行编辑控件，不用为选一个参数再问一次矩阵
   */
  cols: Col[]
  /** 请求的那一层在 `cols` 里的下标。正文那格 = `row.cells[cur]` */
  cur: number
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

/* ---------- 回收站 ---------- */

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
 * b04 Task 12 之后结构手势就归了「机型与版本」页即时落盘（REPORT §7）；
 * 2026-10-06 状态机修正又删掉了第四种 `markBuilt` —— 生成记录是**台账**
 * （`workbench/built.json`），由生成事务直接落盘，从来不进草稿。
 */
export type Patch =
  | { kind: 'setValue'; level: Level; owner: string; key: string; value: unknown | null }
  | { kind: 'setVisibility'; fileId: string; visibility: Visibility }
  | { kind: 'setBundle'; bundleId: string; presets: string[]; bbs: string[] }

/** `app::ApplyResult` */
export interface ApplyResult {
  view: BookView
  /** 撤销这次操作要提交的 patches，**倒序**。为空 = 没有可撤销的改动（不给撤销按钮） */
  inverse: Patch[]
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
  // 没有生成记录字段：台账（built.json）由生成事务直接落盘，生成完成 = 台账已是这一代
}

/** `build::DiffState` —— 点生成会怎样 */
export type DiffState = 'added' | 'modified' | 'unchanged'

/** `build::DiffLineKind` */
export type DiffLineKind = 'context' | 'added' | 'removed'

/** `build::DiffLine`（**行级文本 diff**，与上面 `app::DiffLine` 那条字段级差异不是一回事） */
export interface PreviewDiffLine {
  kind: DiffLineKind
  text: string
  /** 1 起（`removed` 记旧版行号，其余记新版） */
  no: number
}

/** `build::PreviewFile` —— 一份产物的预演 */
export interface PreviewFile {
  uid: string
  fileName: string
  state: DiffState
  /** `unchanged` 时是整份正文（全 `context`）—— 「完整」视图要看原文，「对比」视图才说"不会重写" */
  lines: PreviewDiffLine[]
  added: number
  removed: number
  /**
   * 谁写的：`generate` = 这次生成会重算；`publish` / `software` = **本次生成不动**
   * （manifest / source 归发布、release.json 归软件发布链）—— 界面据此说"不碰它"
   */
  stage: DeliveryStage
}

/** `build::PreviewReport` —— **只算不写**，生成前确认那一步 */
export interface PreviewReport {
  /** **产物**（`mkp/presets/*.toml`）—— 一份一个版本 */
  files: PreviewFile[]
  /**
   * **附属文件**（content 那三份 + `catalog.json`）：生成不只写 toml，它同时重算目录与清单。
   * `uid` / `fileName` 都是交付根相对的路径。单独一格 —— 发布闸数渲染产物时读的是 `files`
   */
  aux: PreviewFile[]
  skipped: [string, string][]
  /** 会写盘的**产物**份数（added + modified） */
  toWrite: number
  /** 不变的**产物**份数 */
  unchanged: number
  /** 非空 = 生成会被拒（与 generate 同一道闸），界面照它压按钮 */
  blocked: string | null
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
  /**
   * 这次发出去的目录登记的**最低正式客户端版本**（结构规则表里对当前结构签名那条）。
   *
   * `null` 只该出现在「这一代还没登记」时 —— 那种情况发布闸 ⑫ 会先拦住。
   * **不编一个版本号出来**（要登记就得有人去规则表里签字）
   */
  minimumClient: string | null
  todos: number
  hints: number
}

/* ---------- 发布闸（`app::audit`，第二刀） ---------- */

/**
 * `audit::AuditStatus` —— 一项的结论四态。
 *
 * ★ `skipped` **不许当 `pass` 画**：它说的是「这一项今天还没跑」。
 * 把"没实现"画成绿勾，比画成红叉更危险（作者定的口径）
 */
export type AuditStatus = 'pass' | 'fail' | 'warn' | 'skipped'

/** `audit::AuditSeverity` —— 分量。**只有 `blocker` 红了才拦发布** */
export type AuditSeverity = 'blocker' | 'warning'

/** `audit::AuditItem` —— 发布闸里的一行（一项检查） */
export interface AuditItem {
  /** 稳定 id（`docs/PUBLISH-ARCHITECTURE.md` §5.2）。界面按它 key 住，列表不会每次重排 */
  id: string
  name: string
  status: AuditStatus
  severity: AuditSeverity
  /** 一句话说清「红了是什么、在哪」。通过时也写（那时的结论） */
  details: string
  affectedFiles: string[]
  /** 界面上那颗「去修」的入口 */
  fixHint: string
}

/**
 * `audit::PublishAudit` —— 一次发布闸的全部结果。
 *
 * **这是"能不能往下走"的唯一答案来源**：`canPublish` 为假时，任何 commit / push / PR
 * 都不许发生。界面不许自己再算一遍（那正是这把刀要治的老病根）
 */
export interface PublishAudit {
  items: AuditItem[]
  filesAdded: number
  filesChanged: number
  filesRemoved: number
  /**
   * 这一代结构要求的**最低正式客户端版本**（`presets/structure-signatures.toml` 里登记的）。
   *
   * `null` = ⑫ 没给出答案（签名没登记 / 规则表读不出来）—— 那种情况 ⑫ 已经是 Blocker，
   * `canPublish` 已经是 false。**不编一个版本号出来**
   */
  minVersion: string | null
  canPublish: boolean
}

/* ---------- 发布事务（`app::publish_tx`，第三刀下半） ---------- */

/**
 * `publish_tx::PublishStage` —— 一次发布事务走到哪一步了。
 *
 * ★ 这是**工作台自己的发布状态模型**：GitHub 的 PR / Gitee 的 MR / 各自的 CI 方言
 * 在后端就被收敛成这一档（见 `ReviewState` / `ChecksSummary`）。前端**不认识任何平台方言**。
 */
export type PublishStage =
  | 'blockedAudit'
  | 'generated'
  | 'committed'
  | 'pushed'
  | 'reviewOpened'
  | 'statusRead'

/** `platform::ReviewState` —— PR/MR 的统一状态（方言已收敛） */
export type ReviewState = 'open' | 'merged' | 'closed' | 'unknown'

/** `platform::ChecksSummary` —— CI 汇总的统一档 */
export type ChecksSummary = 'pending' | 'passed' | 'failed' | 'none' | 'unknown'

/** `platform::RemoteReview` —— 一份 PR/MR 的平台无关快照 */
export interface RemoteReview {
  platform: string
  number: number
  url: string
  state: ReviewState
  checks: ChecksSummary
  title: string
  head: string
  base: string
}

/** `publish_tx::ChangeSummary` —— 这次发布改了几份（按目录里登记的指纹算） */
export interface ChangeSummary {
  added: number
  changed: number
  removed: number
  unchanged: number
}

/** `publish_tx::FileChange` —— 单份文件的指纹变化（`before` 空 = 新增；`after` 空 = 删除） */
export interface FileChange {
  path: string
  before: string | null
  after: string | null
}

/** `publish_tx::MirrorStatus` —— 「把主线同步到第二个官方源」那一步的结论 */
export type MirrorStatus = 'pushed' | 'upToDate' | 'skipped' | 'failed'

/**
 * `publish_tx::MirrorSync` —— 合并之后把主线推给**第二个官方源**的结果。
 *
 * ★ 它回答的是「**客户端读的那个源**跟上了没有」—— 与平台上的 PR 状态是两件事。
 * 一次发布只开一条 PR（代码主线仍只走 GitHub），合完由这一步把数据面铺到另一个源上。
 */
export interface MirrorSync {
  platform: string
  repositoryUrl: string
  branch: string
  status: MirrorStatus
  detail: string
}

/** `publish_tx::MergeOutcome` —— 「合并」这一步的完整结论（评审状态 + 镜像同步） */
export interface MergeOutcome {
  review: RemoteReview
  mirror: MirrorSync | null
}

/** `publish_tx::PublishTxReport` —— 一轮发布事务的结果（阶段的快照） */
export interface PublishTxReport {
  stage: PublishStage
  auditPassed: number
  auditFailed: number
  generated: number
  unchanged: number
  committedPaths: string[]
  review: RemoteReview | null
  branch: string | null
  /** 这次发布提交的短 sha（没有提交 = null，如实说） */
  commit: string | null
  files: number
  summary: string
  /** 本次发布时刻（= 写进目录 `publishedAt` 的那一个）。审计没过 = null */
  publishedAt?: string | null
  /** 本次发布的目录指纹（交付面 catalog 的 `revision`）。没走到生成 = null */
  revision?: string | null
  /** 这次改了几份（新增 / 修改 / 删除 / 未变化）。没走到生成 = null */
  changes?: ChangeSummary | null
  /** 逐份明细（只含新增 / 修改 / 删除那几份；未变化的不列） */
  changedFiles?: FileChange[]
}

/** `publish_tx::TxOptions` —— 发布事务的开关（一般用默认：一次点击走完全程） */
export interface TxOptions {
  /** 只审计 + 生成 + 定稿 + 报"会提交什么"，不 commit / push / 建 PR */
  dryRun?: boolean
  /** 提交推送之后要不要建 PR/MR（默认 true） */
  openReview?: boolean
  /** 目标分支（PR 的 base）；空 = main */
  base?: string
  /** 发到哪个平台（`github` / `gitee`）；不填 = 自动挑（见发布账户配置） */
  platform?: string | null
}

/**
 * `history::PublishRecord` —— 一条发布回执（`<appDataDir>/publish-history.json` 里的一条）。
 *
 * ★ 它是**写入那一刻的快照**：PR/MR 的状态之后会变。要看现在走到哪，拿编号去
 * `publishStatus` **手动刷新**（作者定死：状态是"看一看"，不做轮询）。
 */
export interface PublishRecord {
  at: string
  /**
   * 走到 / 停在哪一阶段。写入时是 `PublishStage` 的线上名；读的时候按字符串收 ——
   * 将来版本写了新阶段，不该让整份历史读不出来（界面自己回落成"认不出"）
   */
  stage: string
  branch: string | null
  commit: string | null
  review: RemoteReview | null
  files: number
  generated: number
  auditPassed: number
  auditFailed: number
  summary: string
  /**
   * 合并之后「把主线同步到第二个官方源」的结论。
   *
   * `null` = 这一条写下来的时候还没走到合并那一步。★ 它**不由刷新改写** ——
   * 刷新拿到的是平台上的 PR 状态，与"镜像跟没跟上"是两件事。
   */
  mirror?: MirrorSync | null
}

/** `history::PublishHistory` —— 回执日志（最新在前） */
export interface PublishHistory {
  historySchema: number
  records: PublishRecord[]
}

/* ---------- 第四刀：发布**软件版本**（与「发布预设」两条链，各有各的类型） ---------- */

/** `release_tx::ReleaseStage` 的线上名（camelCase，与后端 `wire_name()` 逐字一致） */
export type ReleaseStage =
  | 'blockedPreflight'
  | 'ready'
  | 'versionBumped'
  | 'committed'
  | 'pushed'
  | 'reviewOpened'
  | 'merged'
  | 'tagged'
  | 'built'
  | 'releaseCreated'
  | 'assetUploaded'
  | 'infoCommitted'
  | 'infoPushed'
  | 'infoReviewOpened'

/** `platform::RemoteRelease` —— 建好的 Release（网页地址是用户点开的那一页） */
export interface RemoteRelease {
  platform: string
  id: number
  tagName: string
  url: string
}

/** `release_tx::ArtifactInfo` —— 构建出来的安装包 */
export interface ArtifactInfo {
  name: string
  size: number
  path: string
}

/** `release_tx::ReleaseOptions` —— 发布软件版本的开关 */
export interface ReleaseOptions {
  /** 目标版本号（`x.y.z`，不带 v）；不填 = 沿用当前真值 */
  version?: string | null
  /** 一句话说明（进 Release 正文与 `release.json` 的 notes） */
  notes?: string
  /** PR 的 base；空 = main */
  base?: string
  /** 只预检，不动一个字节 */
  dryRun?: boolean
  /** 建了 PR 之后接着合并（人在界面上点过「确认发布」才有） */
  merge?: boolean
  /** 构建**本平台**的安装包（macOS → dmg / Windows → NSIS；默认开） */
  build?: boolean
  openReview?: boolean
}

/** `release_tx::PreflightItem` —— 闸里的一格 */
export interface PreflightItem {
  id: string
  label: string
  /** `pass` / `fail` */
  status: string
  detail: string
}

/** `release_tx::ReleasePreflight` —— 发布软件版本的闸（**只读**，点几次都没副作用） */
export interface ReleasePreflight {
  items: PreflightItem[]
  canRelease: boolean
  currentVersion: string
  tag: string
  branch: string
  hasAccount: boolean
}

/** `release_tx::ReleaseTxReport` —— 一轮「发布软件版本」的结果（阶段快照） */
export interface ReleaseTxReport {
  stage: ReleaseStage
  version: string
  tag: string
  branch: string | null
  commit: string | null
  review: RemoteReview | null
  release: RemoteRelease | null
  artifact: ArtifactInfo | null
  /** ② `release.json` 那一笔所在的分支 */
  infoBranch: string | null
  /** ② 的 PR —— **它由人合并**，合并完客户端才看得到新版本 */
  infoReview: RemoteReview | null
  committedPaths: string[]
  infoCommittedPaths: string[]
  blockedReasons: string[]
  summary: string
}

/** `release_history::ReleaseRecord` —— 一条软件版本发布回执 */
export interface ReleaseRecord {
  at: string
  stage: string
  version: string
  tag: string
  branch: string | null
  commit: string | null
  review: RemoteReview | null
  release: RemoteRelease | null
  artifact: ArtifactInfo | null
  infoBranch: string | null
  infoReview: RemoteReview | null
  summary: string
}

/** `release_history::ReleaseHistory` —— 软件版本发布历史（**最新在前**） */
export interface ReleaseHistory {
  historySchema: number
  records: ReleaseRecord[]
}

/** `credentials::CredentialStatus` —— **只有"有没有"+尾号，没有 Token 原值** */
export interface CredentialStatus {
  platform: string
  configured: boolean
  hint: string | null
}

/**
 * `publish_tx::PlatformAccountView` —— 一个平台在设置页里的完整视图。
 *
 * `repositoryUrl` / `username` 来自 `<appDataDir>/publish-account.json`（配置）；
 * `hasToken` / `tokenHint` 来自本机凭据文件 credentials.json（**只有真假 + 尾号，没有原值**）。
 */
export interface PlatformAccountView {
  platform: string
  repositoryUrl: string
  username: string
  hasToken: boolean
  tokenHint: string | null
}

/** `publish_tx::PublishAccount` —— 设置页「发布账户」那块读的现状（GitHub / Gitee 对称） */
export interface PublishAccount {
  /** 每个平台一格 */
  platforms: PlatformAccountView[]
  /** 当前工作目录的远端 URL（给用户对照"是不是这个仓库"） */
  remoteUrl: string | null
  /** 当前远端是否与已配置的某个平台一致（发布前的一致性提示） */
  remoteMatchesConfig: boolean
  /** 当前分支 */
  branch: string | null
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
  /** 资产交付身份四态（进套餐 / 可选 / 随包 / 仅归档） */
  identity: Record<AssetIdentity, Word>
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
    | 'publishBlocked'
    | 'buildNothingToDo'
    | 'buildNoResources'
    | 'nothingToSave'
    | 'nothingToUndo'
    | 'notUndoable'
    | 'deprecatedWriteBlocked'
    | 'deleteAssetInUse',
    string
  >
  empty: Record<
    | 'noIssues'
    | 'trashEmpty'
    | 'noDisabledFallback'
    | 'matrixNoMatch'
    | 'matrixNoCols'
    | 'matrixSearchSpansAllTabs'
    | 'selectBundle'
    | 'selectAsset',
    string
  >
  /**
   * 关联那一组里**不带变量**的那几句。
   * 带变量的整句在后端就拼好了（`Row.controlNote` / `Cell.blockedNote`）——
   * 前端不拿模板填空，模板一分两处迟早分岔
   */
  relate: Record<'goFixIt' | 'showAnyway' | 'foldBack', string>
  /** 崩溃快照三态。**与 `save` 不是一回事** */
  snapshot: Record<SnapshotState, Word>
  /** 参数台一行上的状态四档（C14）。dirty 压过 origin —— 改了还没保存是最要紧的事实 */
  paramStatus: Record<'factory' | 'machine' | 'version' | 'dirty', Word>
  /** 「已弃用」参数级那一枚（C14 §五）。判据来自上游注册表的 `deprecated` */
  paramDeprecated: Word
  /** 「已弃用」选项级那一枚。判据是后端推出来的（这一档放开的参数全弃用） */
  paramDeprecatedChoice: Word
  /** 对照矩阵状态列三档（C14 第四轮） */
  matrixRow: Record<'notOwn' | 'diff' | 'same', Word>
}

/* ---------- 命令 ---------- */

/* ---------- 机型与版本（`app::machines`） ---------- */

/** `machines::BrandView` —— 品牌从"机型下拉里的一个字符串"升成一等条目（2026-10-03） */
export interface BrandView {
  id: string
  /** 显示名（`拓竹 (Bambu Lab)`）。空 = 没填过，界面回落显示 id */
  name: string
  /** 品牌图 = **资产 id**（不是文件名）。`null` = 没配，消费侧回落内置字标 */
  logo: string | null
  /** **这个品牌下的机型**（反查，后端算）。机型 `brand` 字段写着它的 id */
  machines: string[]
}

/** `machines::VersionView` —— 版本卡上那几格 */
export interface VersionView {
  id: string
  name: string
  recommendedBundle: string | null
  tag: string | null
  description: string | null
  /**
   * **备注**（2026-10-07）：客户端预设列表的**副标题**显示的就是这一句。
   * `null` = 没写（客户端回落路径文本）。可空 —— 客户端「副标题不要位置文案」的正文。
   */
  remark: string | null
  /**
   * 这一版专属的外观图（**资产 id**）。`null` = 回落机型图（`MachineView.image`）——
   * 界面上要说明白那是回落，不是"没配"
   */
  image: string | null
  /** 参数正文已补（14.4）。false = 纯继承基底，界面标「参数源待补」，**不隐藏该版本** */
  hasRecipe: boolean
}

/*
 * 机型尺寸六组（`presetdata::Dimensions`）—— 字段名与外层契约 `src/api/contract.ts`
 * 的 `MachineDimensions` 一一对应（后端注释里写死的那条口径），中间没有翻译层。
 */

/** 床身尺寸，mm */
export interface BedSize {
  width: number
  depth: number
}

/** 喷头可达范围，mm */
export interface MovementRange {
  minX: number
  maxX: number
  minY: number
  maxY: number
  maxZ: number
}

/** 可涂胶范围 + 擦料点的 X 坐标 */
export interface GlueArea {
  glueMinX: number
  glueMaxX: number
  glueMinY: number
  glueMaxY: number
  wipeX: number
}

/** 校准时笔尖要走的那几个点 */
export interface CalibrationPoints {
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

/** 机型标记 */
export interface MachineFlags {
  /** G-code 里用来认机型的那行注释 */
  gcodeMarker: string
  hasSecondFan: boolean
}

/**
 * 一台机型的尺寸（六组）。
 *
 * ★ **六组在文件里是全有或全无**（后端 `load_dimensions` 的口径）：`[dimensions]`
 * 在，六个子表就都得在。所以这里六个字段都不是可选的 —— 界面上「没配尺寸」
 * 是整张表缺席（`MachineView.dimensions === null`），不是某几组缺。
 */
export interface MachineDimensions {
  bedSize: BedSize
  movementRange: MovementRange
  edgeZone: number
  glueArea: GlueArea
  calibration: CalibrationPoints
  flags: MachineFlags
}

/** 一块禁区（画布上的一个多边形）。机器坐标 mm：原点在床身前左角、y 向上 */
export interface ZonePolygon {
  /** `[x, y]` 点对。**点序是有环序的** —— 不许排序 */
  points: [number, number][]
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
  /** 有没有 `[dimensions]` —— 界面上要能看出「这台还没配尺寸」 */
  hasDimensions: boolean
  /** **`[dimensions]` 的逐格视图**（尺寸卡六组）。`null` = 这台没配尺寸 */
  dimensions: MachineDimensions | null
  /** 禁区块数。0 = 这台没有禁区文件 */
  zoneCount: number
  /** **禁区的原始点**（画布要用）。空数组与 `zoneCount === 0` 是同一件事 */
  zones: ZonePolygon[]
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
 *
 * `image`（整机图）2026-10-03 回到台账（第三刀的剥离已作废）：它用
 * `delivery = 'bundled'` 表达「不进云端交付、但工作台可管可换图」。
 */
export type AssetKind = 'image' | 'icon' | 'model' | 'slicerProfile' | 'mkPreset'

/** `assets::AssetView` —— 资产域①层的一条定义（`presets/assets.toml`） */
export interface AssetView {
  id: string
  kind: AssetKind
  /** 归属机型；不属于任何机型时是 null */
  machineId: string | null
  /** 归属版本（只 `mkPreset` 类有）—— 界面拿它跳到那一版 */
  versionId: string | null
  /** **显示名（一律真名）**：MKP 预设 = 版本名、切片器 = 文件名，其余 = 登记名。
   *  界面上显示这一格 */
  display: string
  /** 台账登记的名字（不是显示真源，详情卡里作对照） */
  name: string
  /**
   * 相对**资产源根**（`presets/assets/`）的一段 —— 工作台读的是源，与交付档位无关。
   * 它同时就是 `/assets/` 那条 URL 的后半截（`url` 由后端拼，前端只 encode）
   */
  path: string
  /** `/assets/<path>`。**用之前过 `assetUrl()`** —— 路径里可能有空格 */
  url: string
  /** 切片器（今天只有 `bbs`）与它下面的档位；只有 `slicerProfile` 才有 */
  slicer: string | null
  profile: string | null
  /** 文件在不在。还没搬的话是 false —— 那是状态，不是错 */
  present: boolean
  /** 切片器三根轴之二：喷嘴。后端从路径的 `0.4mm` 那段派生；只有切片器条目有 */
  nozzle: string | null
  /** 三根轴之三：层高。后端从文件名尾部派生（`… 0.10.json` → `0.10`） */
  layer: string | null
  /** **交付身份四态**（进套餐 / 可选 / 随包 / 仅归档；判定在后端一处：
   *  归档 > 随包 > 进套餐 > 可选） */
  identity: AssetIdentity
  /** 交付档位（作者 2026-10-03）：`download` = 客户端按需下载（默认）、
   *  `bundled` = **随包不下载**（整机图：工作台可管可换图，客户端不下载不更新） */
  delivery: 'download' | 'bundled'
  /** **生成状态**（只 `mkPreset` 类有，与生成页同一套判据）：没生成过 = 待生成；
   *  生成过、参数改完还没重新生成 = **待更新**。其余四类恒 null */
  buildState: 'built' | 'stale' | 'neverBuilt' | 'noResources' | null
}

/** `assets::AssetList` */
export interface AssetList {
  assets: AssetView[]
  /** 资产根的绝对路径 */
  root: string
  /** 喷嘴轴候选值。从全部切片器条目取，**不随筛选变** —— 选中一个值不能让别的选项消失 */
  nozzles: string[]
  /** 层高轴候选值。同上 */
  layers: string[]
  /** 过滤前一共几条 —— 页脚「筛出 X / Y 个」的 Y */
  total: number
  /** 全量里的「可选」条数（页脚读数，不随筛选变） */
  optionalCount: number
  /** 全量里的「仅归档」条数（页脚读数，不随筛选变） */
  archiveCount: number
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

/**
 * `assets::AssetInspectView` —— **资产检查面板**（第四刀，选中一条才问）。
 *
 * 它不并进 [`AssetView`]（列表里那一条）：最贵的两格（`sha256` / `bytes`）要读真实
 * 字节（模型 3.2 MB），而列表每次筛选 / 搜索词一变就重取 —— 把哈希算进列表就是
 * "每敲一个字读几 MB"。所以与 `assetUsage` 同一形状：**选中才问**。
 *
 * 两条线各说各的落点：普通资产 = 源文件（`presets/assets/<path>`）；`mkPreset` =
 * 生成产物（`fileName` 是产物名、`absPath` 是产物文件、`productPath` 相对仓库根）。
 * **文件不存在时 `absPath` 同时就是"期望路径"** —— 界面按 `exists` 换标题。
 */
export interface AssetInspectView {
  id: string
  /** 真实文件名（盘上那个名字）。普通资产 = `path` 的文件名；`mkPreset` = 产物名 */
  fileName: string
  /** 盘上绝对路径（普通 = 源文件；`mkPreset` = 产物文件）。**文件不存在时它是期望路径** */
  absPath: string
  /** 文件在不在这条链的期望位置 */
  exists: boolean
  /** 文件大小（字节）；不存在时 null */
  bytes: number | null
  /** **源文件字节**的 SHA-256（小写 hex，与交付侧同一算法）；不存在时 null */
  sha256: string | null
  /** 图片像素尺寸 / viewBox（webp / png / svg 读得出时给）；非图片或读不出 = null */
  width: number | null
  height: number | null
  /** 格式（小写扩展名：`webp` / `svg` / `3mf` / `json` / `toml`…） */
  format: string | null
  /** 产物**相对仓库根**的一段（仅 `mkPreset`）：`presets/delivery/mkp/presets/<产物名>` */
  productPath: string | null
}

/** `bundles::BundleRefView` —— 套餐里一个 `assetRef`，join 资产域后的解析结果（P4） */
export interface BundleRefView {
  id: string
  /** 资产类型。前端按它把 refs 分成 MKP / 切片器两组（MKP 预设 2026-10-03 进了资产库） */
  kind: AssetKind
  /** 能不能解析到一条真实资产。加载期解析不到是 error，真数据上恒 true */
  resolvable: boolean
  /** 是不是 BBS 预设。每条套餐至少一条 true（MKP 与 BBS 成套配发） */
  isBbs: boolean
  /** 资产域登记的名字。解析不到时是空串 */
  name: string
  /** 文件在不在。**MKP 预设按生成状态判**（产物不在资产根里） */
  present: boolean
  /** **生成状态**（只 `mkPreset` 类有，与生成页 / 资产库同一套判据），其余类恒 null */
  buildState: 'built' | 'stale' | 'neverBuilt' | 'noResources' | null
  /** 交付身份（在菜单 / 仅归档）。**含草稿态** —— 刚设还没保存的也看得见 */
  visibility: Visibility
}

/** `bundles::BundleUserView` —— 指向这份套餐的一个版本（一版一套的主指向） */
export interface BundleUserView {
  machineId: string
  versionId: string
}

/** `bundles::BundleView` —— 套餐域①层的一条定义（`presets/bundles.toml`，唯一真源） */
export interface BundleView {
  id: string
  display: string
  machineId: string
  assetRefs: BundleRefView[]
  /** 上一次改动日期（迁移照抄旧值；真改动由后端盖上当天） */
  updatedAt: string | null
  /** **一版一套**：`recommendedBundle` 指着这份套餐的版本 */
  users: BundleUserView[]
  /** `defaultBundle` 指着它的机型 —— 生成侧「版本没自己指」时回退的那一档 */
  defaultFor: string[]
}

/** `bundles::BundleList` */
export interface BundleList {
  bundles: BundleView[]
  /** 过滤前一共几份 —— 页脚「筛出 X / Y 个」的 Y */
  total: number
}

/**
 * 资产 URL。后端给的前缀只有一处（`/assets/`），这里只负责**编码一次** ——
 * 实测 BBS 文件名里有空格（`MKPProcess A1 0.2 0.10.json`）。
 */
export const assetUrl = (url: string) => encodeURI(url)

/** `catalog::VersionField` —— 版本身上可改的那几格。`id` 不在里面（改 ID = 删+加） */
export type VersionField =
  | 'name'
  | 'recommendedBundle'
  | 'tag'
  | 'description'
  | 'remark'
  | 'image'

/** `catalog::MachineField` —— 机型身上可改的那几格。`id` 不在里面（它是文件名） */
export type MachineField = 'display' | 'brand' | 'name' | 'image' | 'icon'

/** `delivery::DeliveryStage` —— 一份交付文件是谁写的 */
export type DeliveryStage = 'generate' | 'publish' | 'software'

/** `delivery::DeliveryFile` —— 交付集合里的一份（「发布预设」卡展出用） */
export interface DeliveryFile {
  /** 相对交付根的路径（`mkp/presets/A1-standard.toml`、`content/bundles.json`…） */
  rel: string
  /** 盘上有这一份没有 */
  exist: boolean
  /** 字节数（不存在 = 0） */
  size: number
  /** 谁写的：生成时重算 / 发布时定稿 / 软件发布链 */
  stage: DeliveryStage
}

/** `build::BaselineDiffEntry` —— 基线 diff 的一条（b05 Task 14.9）。两侧哈希前 16 位，不同就是变了 */
export interface BaselineDiffEntry {
  fileName: string
  status: 'same' | 'changed' | 'missingBaseline'
  productSha: string
  baselineSha: string | null
}

export const wb = {
  open: () => invoke<void>('wb_open'),
  boot: () => invoke<Boot>('wb_boot'),
  reload: () => invoke<Boot>('wb_reload'),
  /** 当前安装的 SupportEase 版本号（只读）—— 仅供「软件版本」展示位；不发版本 */
  appVersion: () => invoke<string>('wb_app_version'),
  /**
   * 写官方源（Bootstrap）**双源**；返回存下去的规范化形状（仓库地址 → 默认发布入口的
   * raw；blob 页转 raw；Gitee 是同一座桥）。`giteeUrl` 传 null / 空串 = 清除 Gitee 档。
   * 两格一笔写全 —— 这一笔就是配置的全部真值
   */
  setBootstrap: (url: string, giteeUrl: string | null) =>
    invoke<{ bootstrapUrl: string; giteeBootstrapUrl?: string | null }>('wb_set_bootstrap', {
      url,
      giteeUrl,
    }),
  words: () => invoke<Words>('wb_words'),

  book: () => invoke<BookView>('wb_book'),
  registry: () => invoke<RegistryView>('wb_registry'),
  /**
   * 改一条参数的**定义**（名称 / 说明 / 单位 / 值类型 / 控件 / 范围 / 步进 /
   * 出厂默认 / 属于 / 前置条件 / 弃用）。**即时落盘**（与机型尺寸那套一致），
   * 不走参数草稿 —— 定义与值在撤销语义上不是一件事。返回重读后的注册表
   */
  setParamMeta: (key: string, edit: ParamMetaEdit) =>
    invoke<RegistryView>('wb_set_param_meta', { key, edit }),
  matrix: (cols: ColRef[], tab: string | null, query: string, baseMachineId?: string | null) =>
    invoke<Matrix>('wb_matrix', { cols, tab, query, baseMachineId: baseMachineId ?? null }),
  trash: () => invoke<TrashEntry[]>('wb_trash'),
  ui: () => invoke<Record<string, unknown>>('wb_ui'),
  saveUi: (ui: Record<string, unknown>) => invoke<void>('wb_save_ui', { ui }),

  /**
   * 机型与版本清单。**这一页唯一的读入口**，不走 `wb_apply_draft` ——
   * 清单与参数值不共用状态机
   */
  machines: () => invoke<MachineList>('wb_machines'),

  /**
   * 资产库清单（P4）。条目来自 `presets/assets.toml`（资产域①层），
   * 类型 / 三根轴 / 交付身份 / 搜索在后端筛；选项表不随筛选变
   */
  assets: (
    kind: string | null,
    slicer: string | null,
    nozzle: string | null,
    layer: string | null,
    identity: string | null,
    query: string | null,
  ) => invoke<AssetList>('wb_assets', { kind, slicer, nozzle, layer, identity, query }),

  /** 删一条资产。反查守卫在后端：有人引用整次拒绝（界面把它转成拦截页） */
  removeAsset: (assetId: string) => invoke<AssetList>('wb_remove_asset', { assetId }),

  /** 改一条资产的交付档位（download ↔ bundled）。即时落盘；mkPreset 不许随包 */
  setAssetDelivery: (assetId: string, delivery: 'download' | 'bundled') =>
    invoke<AssetList>('wb_set_asset_delivery', { assetId, delivery }),

  /**
   * 套餐清单（P4）。条目来自 `presets/bundles.toml`（唯一真源），refs join 资产域、
   * 指向按一版一套分两档报；`query` 是 id / 显示名的子串筛选
   */
  bundles: (query: string | null) => invoke<BundleList>('wb_bundles', { query }),

  /**
   * 换一份套餐的文件清单（P4 套餐内容编辑）。**即时落盘**，不走参数草稿 ——
   * 悬空引用 / 「没有一条 BBS」在后端拦；MKP 预设也在 `assetIds` 里
   * （2026-10-03 进资产库，`type = 'mkPreset'`，**文件在不在都能选**）；
   * `updatedAt` 由那次写盖上当天
   */
  setBundleRefs: (bundleId: string, assetIds: string[]) =>
    invoke<BundleList>('wb_set_bundle_refs', { bundleId, assetIds }),

  /** 新建一条套餐。**至少一条 BBS**（成套配发），id 不许与现有撞（大小写不敏感） */
  addBundle: (id: string, machineId: string, display: string, assetIds: string[]) =>
    invoke<BundleList>('wb_add_bundle', { id, machineId, display, assetIds }),

  /** 编辑一条套餐：改 id 与/或显示名。**改 id 连带重指机型文件里的引用** */
  renameBundle: (bundleId: string, newId: string, display: string | null) =>
    invoke<BundleList>('wb_rename_bundle', { bundleId, newId, display }),

  /** 复制一条套餐：内容照抄、id 必须是新的；复制出来的那份没人指着 */
  copyBundle: (bundleId: string, newId: string, display: string | null) =>
    invoke<BundleList>('wb_copy_bundle', { bundleId, newId, display }),

  /** **把一批版本指到这份套餐**（多选 + 确认）。不限机型；已经指着它的跳过。
   *  即时落盘，界面在确认前先摆影响预览 */
  assignBundleVersions: (bundleId: string, uids: string[]) =>
    invoke<BundleList>('wb_assign_bundle_versions', { bundleId, uids }),

  /** 删一条套餐。被机型默认或版本指着时整次拒绝并点名（界面转拦截页） */
  removeBundle: (bundleId: string) => invoke<BundleList>('wb_remove_bundle', { bundleId }),

  /**
   * 「谁在用它」。**删资产之前先问这一条** —— 删掉一张还被机型引用着的图，
   * 界面上只表现为「那台机型的图没了」。删除守卫在数据层（`Presets::remove_asset`）
   */
  assetUsage: (assetId: string) => invoke<AssetUsageView>('wb_asset_usage', { assetId }),

  /**
   * **资产检查面板**的数据（第四刀）：真实文件名 / 绝对路径 / SHA-256 / 尺寸 / 格式 /
   * 大小 / 产物路径。只读；选中一条问一次（理由见 [`AssetInspectView`]）
   */
  assetInspect: (assetId: string) => invoke<AssetInspectView>('wb_asset_inspect', { assetId }),

  /**
   * 「在访达中显示」：打开系统文件管理器**并选中**这一条。
   * **前端只传资产 id** —— 路径由后端自己算（不给前端传任意路径的机会）；
   * 只读、只开窗口、不碰任何状态。文件不在时后端如实拒绝（附期望路径）
   */
  revealAsset: (assetId: string) => invoke<void>('wb_reveal_asset', { assetId }),

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
   * 改品牌的一格（`name` / `logo`）。**即时落盘**（同机型那一套：没有草稿、没有撤销）。
   * `logo = null` 是**清空**（删键，消费侧回落内置字标）；`name` 不许清空
   */
  setBrandField: (brandId: string, field: 'name' | 'logo', value: string | null) =>
    invoke<MachineList>('wb_set_brand_field', { brandId, field, value }),

  /** 新建一个品牌（id + 显示名；品牌图后配）。**即时落盘**，撞名（含仅大小写不同）当场拒 */
  addBrand: (id: string, name: string) => invoke<MachineList>('wb_add_brand', { id, name }),

  /**
   * **把一台机型挪到另一个品牌下**。只改机型文件的 `brand` 一格（品牌侧是反查）。
   * 目标品牌不存在时如实拒 —— 打错一个字会在盘上留下一个悬空的归属
   */
  moveMachineToBrand: (machineId: string, brandId: string) =>
    invoke<MachineList>('wb_move_machine_to_brand', { machineId, brandId }),

  /**
   * **写一台机型的整张 `[dimensions]`**（六组一起）。即时落盘。
   * 校验在后端：床身宽深必须为正、数字必须有限；全零的可选组由后端写零值（不删子表）
   */
  setMachineDimensions: (machineId: string, dimensions: MachineDimensions) =>
    invoke<MachineList>('wb_set_machine_dimensions', { machineId, dimensions }),

  /**
   * **写一台机型的禁区**。空数组 = 删掉 `forbidden_zones/<id>.toml`
   * （清空是删文件，不是留一个空文件）。每块 ≥ 3 点、块数 ≤ 32 由后端把关
   */
  setMachineZones: (machineId: string, zones: ZonePolygon[]) =>
    invoke<MachineList>('wb_set_machine_zones', { machineId, zones }),

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
  /** 生成前预演：**只算不写**，界面上「点生成 → 看 diff → 确认」的中间那一步 */
  generatePreview: (scope: BuildScope) =>
    invoke<PreviewReport>('wb_generate_preview', { scope }),
  generate: (scope: BuildScope) => invoke<GenerateReport>('wb_generate', { scope }),
  revertPreview: (uid: string) => invoke<RevertPreview>('wb_revert_preview', { uid }),
  /**
   * **发布闸**（第二刀）：十五项逐项结果。
   *
   * ★ 与工作台点【发布】是**同一个 Rust 核心**（`audit::publish_audit`）——
   * 界面只负责画，不负责判。`canPublish` 为假时**不许**往下走
   */
  publishAudit: () => invoke<PublishAudit>('wb_publish_audit'),
  /**
   * **发布事务**（第三刀下半）：唯一对外的发布动作。
   *
   * ★ 它内部串完 `审计 → 生成 → 定稿 → 本地 git → 平台 PR/MR` —— 前端**不再有**
   * 独立的「生成」「创建 PR」按钮。返回的是**阶段快照**（走到哪、停在哪、为什么）。
   * 不给 `opts` 就是默认全开（一次点击走完全程）。
   */
  publish: (opts?: TxOptions) => invoke<PublishTxReport>('wb_publish', { opts: opts ?? null }),
  /** 发布账户现状（只读）：每个平台的仓库地址 / 用户名 / 有无 Token */
  publishAccount: () => invoke<PublishAccount>('wb_publish_account'),
  /** 存一个平台的**发布目标**（仓库地址 + 用户名，进 publish-account.json；**不含 Token**） */
  setPublishAccount: (platform: string, repositoryUrl: string, username: string) =>
    invoke<PlatformAccountView>('wb_set_publish_account', { platform, repositoryUrl, username }),
  /** 存一个平台的 Token（**只进不出**：写凭据文件，返回里没有原值） */
  setPublishToken: (platform: string, token: string) =>
    invoke<PlatformAccountView>('wb_set_publish_token', { platform, token }),
  /**
   * **显式取出**一个平台的 Token 明文 —— 设置页那颗「眼睛」的唯一用途（作者 2026-10-07）。
   *
   * ★ 与 `publishAccount()` 那条状态面**分家**：`PlatformAccountView` 里从来没有 token 字段
   * （判据 `credentials_never_echo_the_token`），这里是另一条**只有人点眼睛才走**的出口。
   * 没配 / 读不出来 = `null`（不抛错）。
   */
  getPublishToken: (platform: string) =>
    invoke<string | null>('wb_get_publish_token', { platform }),
  /** 清一个平台的发布账户（配置 + 凭据一起清；幂等） */
  clearPublishAccount: (platform: string) =>
    invoke<PlatformAccountView>('wb_clear_publish_account', { platform }),
  /** **手动回读**一份 PR/MR 的状态（快照 + 手动刷新；不做后台轮询） */
  publishStatus: (number: number) => invoke<RemoteReview>('wb_publish_status', { number }),
  /**
   * **合并**一份 PR/MR（squash）—— 人在回执屏上**显式点过**才调。
   *
   * ★ 口径（作者 2026-10-04 拍）：一律 squash；**不强制等 CI** —— "CI 没跑完 / 已经红了"
   * 的二次确认在界面做，这里不重复设闸。合完返回**回读后的真状态**（应落到 `merged`）。
   */
  mergeReview: (number: number, platform?: string | null) =>
    invoke<MergeOutcome>('wb_merge_review', { number, platform: platform ?? null }),
  /** 发布历史（只读）：最近若干次「发布预设」事务的回执，**最新在前** */
  publishHistory: () => invoke<PublishHistory>('wb_publish_history'),
  /** 在系统浏览器里打开一个 **http(s)** 链接（回执屏的「查看 PR」；别的形状后端会拒） */
  openExternal: (url: string) => invoke<void>('wb_open_external', { url }),

  /**
   * **发布软件版本的闸**（第四刀）—— **只读**，一个字节都不写，点几次都没副作用。
   *
   * ★ 它与「发布预设」的闸（`publishAudit`）是**两道不同的闸**：这道看的是
   * 分支 / 工作区 / 版本号一致 / tag 有没有被占 / 平台支不支持软件 Release。
   */
  releasePreflight: (version?: string | null) =>
    invoke<ReleasePreflight>('wb_release_preflight', { version: version ?? null }),
  /**
   * **发布软件版本**（一次手势）—— 与 CLI（`src-tauri/src/bin/release.rs`）**同一个内核**。
   *
   * ★ 这一趟会**构建安装包并上传**（以分钟计），所以按钮要显示"发布中"，不是转圈就算了。
   */
  releaseSoftware: (opts: ReleaseOptions) =>
    invoke<ReleaseTxReport>('wb_release_software', { opts }),
  /** 软件版本发布历史（**与 `publishHistory` 不是同一本账** —— 两条链分开记） */
  releaseHistory: () => invoke<ReleaseHistory>('wb_release_history'),

  /**
   * 复制已有版本（b05 Task 14.3 / doc §4.3 第 2–5 步）：**只写版本定义** ——
   * `recommendedBundle` 抄模板；
   * `tag` / `description` 前端拿模板值预填。返回刷新后的清单
   */
  copyVersion: (machineId: string, templateVersionId: string, id: string, name: string, tag?: string, description?: string) =>
    invoke<MachineList>('wb_copy_version', {
      machineId, templateVersionId, id, name,
      tag: tag ?? null, description: description ?? null,
    }),

  /**
   * 复制参数正文（b05 Task 14.5 / doc §4.3 第 7 步）：取模板版本的**完整有效配方**
   * 钉成新版本的显式覆盖 —— **复制为独立版本，后续修改互不影响**。
   * 缺失的参数不伪造（模板有效配方里没有的键继续继承 defaults）。
   * 返回写入的键数。**前提**：新版本定义已存在（先 copyVersion）
   */
  copyRecipe: (machineId: string, templateVersionId: string, newVersionId: string) =>
    invoke<number>('wb_copy_recipe', { machineId, templateVersionId, newVersionId }),

  /**
   * 对照基线 diff（b05 Task 14.9 第①步，**只读**）：九份产物 vs 基线目录，
   * `status` ∈ `same | changed | missingBaseline`。**人看过这份清单再点同步**
   */
  baselineDiff: () => invoke<BaselineDiffEntry[]>('wb_baseline_diff'),
  /**
   * 同步对照基线（b05 Task 14.9 第②步，**显式写入动作**）：落点闸在
   * `preset::sync_baseline` 内部（只认真 fixtures 或系统临时目录）。
   * 内容相同的跳过，返回真正写入的份数
   */
  syncBaseline: () => invoke<number>('wb_sync_baseline'),

  /**
   * 交付目录的残留清单（b05 Task 13.4）：「不在本次交付集合内」的文件。
   * **发布被残留拦下时先看这一条** —— 残留会被消费端真的下载到
   */
  distStrays: () => invoke<string[]>('wb_dist_strays'),
  /**
   * 清理残留（b05 Task 13.5）：走 `workbench/.trash/delivery/` 回收（保留相对路径），
   * **不直接删**。清理完重新发布即可
   */
  cleanDistStrays: () => invoke<number>('wb_clean_dist_strays'),
  /**
   * **交付文件清单**（2026-10-07，只读）：本次交付集合里都有哪些文件、盘上有没有、
   * 谁写的（生成时重算 / 发布时定稿 / 软件发布链）。
   *
   * 名单就是发布闸判残留用的那份集合 —— 界面不另拼一份"大概有这些"
   */
  deliveryFiles: () => invoke<DeliveryFile[]>('wb_delivery_files'),
  /**
   * 看一份交付文件的**盘上原文**（只读）。只认交付集合里的路径；
   * 盘上还没有（还没生成 / 还没发布）时如实报错
   */
  deliveryFile: (rel: string) => invoke<string>('wb_delivery_file', { rel }),
}
