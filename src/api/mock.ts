import { NotImplementedError } from './errors'
import type {
  ActiveOrigin,
  ActivePreset,
  ArchivedFile,
  Axes,
  CalibModel,
  MkpApi,
  CloudVersion,
  DownloadOutcome,
  DownloadTick,
  GuideTarget,
  OnDiskFile,
  ParamDecision,
  ParamDecisionKind,
  ParamEdit,
  ParamSyncEntry,
  PresetParamSync,
  PresetParamValues,
  PresetHistory,
  PresetSource,
  PinPref,
  SoftwareUpdate,
  ThemePref,
  LocalPresetFile,
  UpdateInfo,
  UpdateResult,
  UpdateState,
} from './contract'
import {
  allMachines,
  allPlates,
  allPresetFiles,
  bbsLocalTree,
  bbsPresetText,
  catalogRegistry,
  copyToSlicerIn,
  localFileIds,
  menuEntries,
  mockCatalogFiles,
  paramMeta,
  reportDetail,
  reportExportPaths,
  reportExportPlan,
  reportSummaries,
  resolveParams,
  resolveVersionFiles,
  slicerCopied,
  slicerPresetText,
} from './mockServer'

/**
 * mock 实现：同一套契约，浏览器预览时由它作答。
 *
 * **这个文件是接线处，不是数据层。** 真机上换的是 `src/api/bridge.ts`（Rust 侧注入的那一份），
 * 页面一行不动 —— 所以业务假数据不散在页面或 `src/app/constants/` 里。
 * 界面结构数据（页签、品牌/机型/版本三级选项）是另一回事，那些属于应用本身，在 `src/app/constants/`。
 *
 * 数据分两处，别混：
 *
 * - **本文件里**：校准页那几个方法（校准板清单、三轴偏移）用的那几份手写常量。
 * - **`src/api/mockServer/`**：预设页 / 参数页要读的那十二个方法，
 *   由 `data/*.json` 的六份上游快照解析而来。那一整个目录是搬过来的假后端，
 *   真机上由 Rust 侧接管，`mockServer/` 整个不再被引用。
 *
 * 刻意**不加延迟**。真实情况是「连接慢、下载快」，而这里连接这一步根本不存在 ——
 * 凭空塞一个 300ms 只会让每次选机型都闪一下骨架屏，那是假的慢，不是真的慢。
 * 保留 async 只为形状一致：调用方必须按"这是个 Promise"来写，将来换成 IPC 才不用改。
 */

/*
 * **v023 那份「按打印件版本索引的预设」夹具（`presetIndex`）随 `getPreset` 一起删了**
 * （2026-10-02 清扫）：三份全是 A1 mini 的、键来自已被替换掉的旧常量表；客户端换成
 * 文件体系之后页面就不再调它（见 `src/app/calib/usePreset.ts` 文件头），只剩这条死契约
 * 在引用。这里保留一笔记录，不再留夹具 —— 留一份没人读的假数据，下一次盘点又要猜它能不能删。
 */

/**
 * 校准板清单。
 *
 * 契约里没有对应的"预设目录"方法，而旧版校准页的预设下拉需要一份候选表 ——
 * 那时候这里以同步常量的形式导出过 `presetCatalog`。新版（A40）的预设候选走
 * `getPresetFiles()` / `getMachines()`，所以那份常量已经退场（见 `presetIndex` 上面那段）。
 */
const calibModels: CalibModel[] = [
  {
    id: 'z',
    name: 'Z 轴校准',
    desc: '校准喷嘴高度与第一层，先打这个',
    fileName: 'ZOffset_Calibration.3mf',
    size: '284 KB',
    ready: true,
  },
  {
    id: 'xy',
    name: 'XY 校准',
    desc: '校准平面内的偏移，Z 轴之后打',
    fileName: 'Precise_Calibration.3mf',
    size: '377 KB',
    ready: true,
  },
  {
    id: 'test-models',
    name: '支撑测试',
    desc: '校准完打这个看支撑效果',
    fileName: 'MKP_support_test_models.3mf',
    size: '3.2 MB',
    ready: true,
  },
]

/** 真机用户根里那一格的相对前缀（后端 `runtime::local` 的 `LOCAL_DIR` 同一个值） */
const LOCAL_PREFIX = 'presets-local/'

/**
 * 浏览器演示用的使用中指针（内存态，刷新即还原；真数据在 Rust 侧 run/ 状态文件里）。
 *
 * ★★ 2026-10-14（作者）：**初值必须是 `null`** —— 它只能由真实的「使用 / 应用」动作改。
 *
 * 原来这里写死指向某一份，后果有两条，都是"演示不像真机"：
 *   ① 用户点别的份之前，"在用"已经在某一份上了 —— 而真机首次打开是**没有**在用的；
 *   ② 探针复现路径不稳定（上一轮探针反复翻车就是这个：`mockActive` 被前面的动作改过，
 *      下一次跑就落在别的状态上）。**初值 null ⇒ 每次启动都是同一个起点**。
 */
let mockActive: ActivePreset | null = null

/** 浏览器演示用的数据源覆盖（内存态，刷新即还原；真数据在 Rust 侧 run/preset-source.json） */
let mockSource: PresetSource | null = null

/*
 * 浏览器演示用的主题档（**内存态，刷新即还原**；真数据在 Rust 侧 run/app-state.json）。
 * `null` = 还没选过 ⇒ 界面按"跟随系统"——与真机同一个口径，别在这里另立一套默认。
 */
let mockTheme: ThemePref | null = null

/*
 * 浏览器演示用的自动清理开关（**内存态，刷新即还原**；真数据在 Rust 侧 run/app-state.json）。
 * 演示里给"开着"——与真机同一个出厂默认（没选过 = 开），别在这里另立一套。
 */
let mockAutoClean = true

/*
 * 浏览器演示用的窗口两格（**内存态，刷新即还原**；真数据在 Rust 侧 run/app-state.json）。
 * 与真机同一个出厂默认：不置顶、不在窄窗。
 *
 * ★ 真窗口那半（`set_always_on_top` / `set_size` / `set_position`）在浏览器里**空转**——
 *   没有窗口可摆。但这两格照常记账：探针要验"点两下文字换了、盒子没变宽"。
 */
let mockPin: PinPref = 'off'
let mockCompact = false
/* 进小窗之前用户自己选的那一档（退出时还原）—— 与 Rust 侧 `pinBeforeCompact` 同一格 */
let mockPinBeforeCompact: PinPref | null = null

/*
 * 浏览器里的「用户目录」：**内存态**（刷新还原）—— 与 `mockActive` 同一套做法。
 *
 * 为什么让它在内存里真的能走通：编辑那条链（改 → 临时文件 → 另存）是这一层最需要被看见的
 * 东西；只抛"未实现"的话，浏览器里连编辑器长什么样、保存之后表格怎么变都验不了。
 * 真机上它是 `<appDataDir>/user/presets-local/`，盘就是底账。
 */
const mockLocal: LocalPresetFile[] = [
  /*
   * ★★ 2026-10-14 与 catalog 那 9 份 MKP **对齐**（作者：「连本地表也照 catalog 铺」）。
   *
   * 改动前这里是一批**与目录毫无关系**的名字（`A1 涂胶.toml` 之类），而云端表列的是
   * `A1-standard.toml` / `A1-fast.toml` —— 于是浏览器演示里"我下过的那份"与"云端那一份"
   * 看不出是同一份，血统还写着早就过时的 `delivery/mkp/presets/…`。
   *
   * 现在：本地表 = 目录那 9 份的**一份镜像**（名字 / 机型 / 版本 / 血统与目录逐条对上），
   * 外加两份**用户自己造的**（`A1 涂胶.toml` = 另存改名、`Process_0.2mm.json` = 手工导入的
   * 认不出类别的那份）—— 镜像的那几份才是"云端下载来的"，那两份才是"我自己的"。
   *
   * 为什么"照铺"而不是"只改血统"：真机上 `presets-local/` 里**确实**躺着下载来的那几份
   * （下载 / 另存都落在这一格），演示要与真机同形 —— 数字（云端 9 · 本地 N）也才对得上。
   */
  {
    /*
     * 云端那份下载到本机（血统 = 它自己那一版，`current`）。
     * 这条是所有后续操作的基准：参数页 / 校准页都从它读。
     */
    path: 'presets-local/A1-standard.toml',
    fileName: 'A1-standard.toml',
    size: 2048,
    modifiedUnix: 1780000000,
    kind: 'mkp_preset',
    state: 'ok',
    stateDetail: null,
    basedOn: 'current',
    basedOnLabel: 'delivery/mkp/presets/A1-standard.toml',
    basedOnRelease: '2026-10-05 08:00:00',
    basedOnMachineId: 'A1',
    basedOnVersionId: 'STANDARD',
    machineId: 'A1',
    versionId: 'STANDARD',
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
  {
    /* 云端新版落下来的那一份（`current`，带 2026-10-15 那一版的发布时间） */
    path: 'presets-local/A1-fast.toml',
    fileName: 'A1-fast.toml',
    size: 2048,
    modifiedUnix: 1780900000,
    kind: 'mkp_preset',
    state: 'ok',
    stateDetail: null,
    basedOn: 'current',
    basedOnLabel: 'delivery/mkp/presets/A1-fast.toml',
    basedOnRelease: '2026-10-15 09:00:00',
    basedOnMachineId: 'A1',
    basedOnVersionId: 'FAST',
    machineId: 'A1',
    versionId: 'FAST',
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
  {
    /*
     * 用户自己那一份：**从 `A1-fast.toml` 另存改名的**（血统指向目录里真有的那份，
     * 名字是用户自己起的）—— 演示"另存为一份新的"之后长什么样。
     */
    path: 'presets-local/A1 涂胶.toml',
    fileName: 'A1 涂胶.toml',
    size: 2048,
    modifiedUnix: 1780500000,
    kind: 'mkp_preset',
    state: 'ok',
    stateDetail: null,
    basedOn: 'outdated',
    basedOnLabel: 'delivery/mkp/presets/A1-fast.toml',
    basedOnRelease: '2026-05-29 04:26:12',
    basedOnMachineId: 'A1',
    basedOnVersionId: 'FAST',
    machineId: 'A1',
    versionId: 'FAST',
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
  {
    /* 手工放进来的、认不出是哪一类的（`kind: null`：合法状态，详情里照实写"认不出"） */
    path: 'presets-local/Process_0.2mm.json',
    fileName: 'Process_0.2mm.json',
    size: 1024,
    modifiedUnix: 1780003600,
    kind: null,
    state: null,
    stateDetail: null,
    basedOn: 'unknown',
    basedOnLabel: null,
    basedOnRelease: null,
    basedOnMachineId: null,
    basedOnVersionId: null,
    machineId: null,
    versionId: null,
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
  {
    /*
     * 第九层的演示：**TOML 语法坏了** → 行上画「文件无法读取」，并且**不给**
     * 「应用」与「改这份」（真机上后端的文件级检查同样会把这两条拒掉）。
     * 注意它照常列在表里 —— 藏起来等于对用户说他没这份文件。
     */
    path: 'presets-local/坏了的涂胶.toml',
    fileName: '坏了的涂胶.toml',
    size: 1536,
    modifiedUnix: 1780007200,
    kind: 'mkp_preset',
    state: 'unreadable',
    stateDetail: 'TOML 语法不对（第 3 行第 1 列）',
    basedOn: 'unknown',
    basedOnLabel: null,
    basedOnRelease: null,
    basedOnMachineId: null,
    basedOnVersionId: null,
    machineId: null,
    versionId: null,
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
]
/** 正文库。键 = 相对用户根的路径。真机上每一份都能读；假后端里先把演示那份种上 */
const mockLocalText = new Map<string, string>()

/**
 * **备注覆盖账**（假后端版）：键 = 文件身份，值 = 用户改过的备注。
 * 与真机同语义（`runtime::remarks`）：`null` = 删键（恢复默认，回到工作台那句 /
 * 空着）；**空串也是覆盖**（存进去，副标题就空着）—— 只有删除才删键。
 */
const mockRemarks = new Map<string, string>()

/**
 * 改文件头的 `# machine:` / `# variant:` 两行（假后端版，与 Rust
 * `lineage::rewrite_machine_variant` 同一套规矩：原位换值，缺哪行补哪行，
 * 正文一个字节不动）。
 */
function rewriteMockMachineVariant(text: string, machineId: string, versionId: string): string {
  const isMachine = (l: string) => /^#\s*machine:/.test(l)
  const isVariant = (l: string) => /^#\s*variant:/.test(l)
  const hasMachine = text.split('\n').some(isMachine)
  const hasVariant = text.split('\n').some(isVariant)
  const out: string[] = []
  let machineDone = false
  let variantDone = false
  for (const line of text.split('\n')) {
    if (isMachine(line)) {
      out.push(`# machine: ${machineId}`)
      machineDone = true
      if (!hasVariant) {
        out.push(`# variant: ${versionId}`)
        variantDone = true
      }
      continue
    }
    if (isVariant(line)) {
      if (!hasMachine && !machineDone) {
        out.push(`# machine: ${machineId}`)
        machineDone = true
      }
      out.push(`# variant: ${versionId}`)
      variantDone = true
      continue
    }
    out.push(line)
  }
  if (!machineDone) out.push(`# machine: ${machineId}`)
  if (!variantDone) out.push(`# variant: ${versionId}`)
  return out.join('\n')
}

/**
 * 用户文件新名字的门槛（与 Rust 侧 `local::check_new_name` 同一套）：
 * 给一口人话原因，没有就是过。改名（第十层）与另存为一份新的（第十一层）共用这一处。
 */
function mockNameProblem(oldName: string, newName: string): string | null {
  const name = newName.trim()
  if (name === '') return '新名字不能是空的'
  if (name.includes('/') || name.includes('\\')) {
    return '新名字不能带路径 —— 这一层只改名字，不搬文件夹'
  }
  if (name === '.' || name === '..') return '这个名字不是一个文件名'
  const ext = (n: string): string => {
    const i = n.lastIndexOf('.')
    return i <= 0 ? '' : n.slice(i).toLowerCase()
  }
  if (ext(name) !== ext(oldName)) {
    return '后缀要保持原样 —— 改名 / 复制都不改它是哪一类（.toml 还是 .toml）'
  }
  return null
}

/** 外部路径取文件名（与 Rust 侧 `runtime::import::base_name` 同一件事） */
function mockBaseName(source: string): string {
  const parts = source.split(/[/\\]/)
  return parts[parts.length - 1] ?? source
}

/** 一个没被占的名字：`A1-fast.toml` → `A1-fast-2.toml` → `A1-fast-3.toml`…… */
function mockUntakenName(wanted: string): string {
  const dot = wanted.lastIndexOf('.')
  const stem = dot > 0 ? wanted.slice(0, dot) : wanted
  const ext = dot > 0 ? wanted.slice(dot) : ''
  for (let n = 2; n <= 999; n++) {
    const candidate = `${stem}-${n}${ext}`
    if (!mockLocal.some((f) => f.path === `presets-local/${candidate}`)) return candidate
  }
  return wanted
}
/**
 * **修改历史账本**（假后端版，2026-10-17）：键 = "哪条线 + 身份"，
 * 与真机 `runtime::history::key_of` 同形状（真机是 sha256、这里直接拼字符串 ——
 * 假后端不需要那一层），值 = 整本账。
 */
const mockHistory = new Map<string, PresetHistory>()

/**
 * **学习账**（2026-10-17）：真机在 `<appDataDir>/run/usage.json`（**不进备份**）。
 *
 * 计数点是**后处理那一趟**（钩子载入 IR 之后），不是"点了使用" ——
 * 浏览器里没有钩子进程，所以这一档在预览里**永远是空的**（如实为假，不铺假数据）。
 */
const mockUsage = new Map<string, { count: number; lastUnix: number }>()

/** 一本账认一份文件：同一条线 + 同一个身份 = 同一本（两份同名文件各有一本） */
function historyKeyOf(origin: string, id: string): string {
  return `${origin}:${id}`
}

/*
 * 「编辑文件」那条链的草稿（2026-10-10：与上面 `mockDraft` **分开存**）——
 * 两侧互相作废：任一侧保存都把另一侧丢掉（编辑器手上那份就成了孤儿）。
 */
let mockTextDraft: { fileName: string; path: string; text: string; updatedUnix: number } | null =
  null

/** 编辑中的那一份（真机上是 `run/draft-preset.json`）。`path` 只有用户线才有 */
let mockDraft: {
  origin: ActiveOrigin
  sourceFileName: string
  path: string | null
  text: string
  updatedUnix: number
} | null = null

/** 演示正文。真机上它是云端原件（`mkp/…`）的字节 —— 假后端没有文件系统，只能给一段 */
const MOCK_PRESET_TEXT =
  [
    '# 假后端的演示正文 —— 真机上这里是云端原件（mkp/…）的字节',
    '[toolhead]',
    'speed_limit = 70 # 速度上限(mm/s)',
    'offset_x = -1 # 笔尖偏移',
    'offset_y = 18.6 # 笔尖偏移',
    'offset_z = 4 # 笔尖偏移',
    '',
    '[wiping]',
    'mode = "tower"',
    '',
  ].join('\n')

/** 假后端给"导入进来的那份"种的演示正文：**没有血统** —— 外部文件没有 based_on，也不编造 */
const MOCK_IMPORTED_TEXT = '# 从外部导进来的（假后端演示正文）\n"涂胶宽度" = 1.0\n'

/*
 * 三行血统（真机上由 `runtime::lineage` 管：另存时写、写回时照抄）。
 * 假后端没有那一套，只做**文本形状上的同一件事**：编辑器里给正文、保存时把原来那三行抄回去。
 */
const lineageLinesOf = (text: string) =>
  text.split('\n').filter((line) => line.startsWith('# based_on'))
const withoutLineage = (text: string) =>
  text.split('\n').filter((line) => !line.startsWith('# based_on')).join('\n')

/* 演示那份 `.toml` 种一份正文（带血统，与它条目里 `basedOnLabel` 说的那份对上）。
   键带引号是因为它得**真的是 TOML** —— 它那条 state 说 `ok`，演示也要自洽 */
/* 血统那几行的落点写**真实目录形状**（`delivery/mkp/presets/…`），与 catalog 的 path 同形 ——
   老板本写的是 `delivery/mkp/presets/…`（那是更早一代的发布根） */
mockLocalText.set(
  'presets-local/A1-standard.toml',
  [
    '# 云端那份下载到本机（假后端演示正文）',
    '# machine: A1',
    '# variant: standard',
    '# based_on: delivery/mkp/presets/A1-standard.toml',
    '# based_on_release_time: 2026-10-05 08:00:00',
    `# based_on_sha256: ${'0'.repeat(64)}`,
    /* 键用**注册表里的真名字**（`[toolhead]` + `offset_y`）：对比台按注册表认参数，
       编一套假键的话那一列会全是"—"，等于把要验的东西绕开了 */
    '[toolhead]',
    'offset_y = 20.0 # 笔尖偏移',
    'speed_limit = 60 # 涂胶速度限制 (mm/s)',
    '',
  ].join('\n'),
)
mockLocalText.set(
  'presets-local/A1-fast.toml',
  [
    '# 云端新版落下来的那一份（假后端演示正文）',
    '# machine: A1',
    '# variant: fast',
    '# based_on: delivery/mkp/presets/A1-fast.toml',
    '# based_on_release_time: 2026-10-15 09:00:00',
    `# based_on_sha256: ${'1'.repeat(64)}`,
    '[toolhead]',
    'offset_y = 26.8 # 笔尖偏移',
    'speed_limit = 70 # 涂胶速度限制 (mm/s)',
    '',
  ].join('\n'),
)
/* 用户从 `A1-fast.toml` 另存改名出来的那一份：血统照抄来源，名字是用户起的 */
mockLocalText.set(
  'presets-local/A1 涂胶.toml',
  [
    '# 我另存出来的一份（假后端演示正文）',
    '# machine: A1',
    '# variant: fast',
    '# based_on: delivery/mkp/presets/A1-fast.toml',
    '# based_on_release_time: 2026-05-29 04:26:12',
    `# based_on_sha256: ${'1'.repeat(64)}`,
    '[toolhead]',
    'offset_y = 22.4 # 笔尖偏移',
    'speed_limit = 64 # 涂胶速度限制 (mm/s)',
    '',
  ].join('\n'),
)
/* 坏了那份也种一份正文：**看正文照旧读得出来**（用户要能看着它去修）——
   只有「应用 / 改这份」被拦（真机上是后端的文件级检查拦的） */
mockLocalText.set(
  'presets-local/坏了的涂胶.toml',
  ['# 坏了的演示（假后端演示正文）', '[toolhead]', 'offset_x = (1', ''].join('\n'),
)

/**
 * **原文镜像（假后端版）**：这份文件"最初落到盘上时"的字节 —— 把上面种好的那份抄一遍就是它。
 *
 * 与真机同语义（Rust `runtime::baseline` 的原文镜像一段）：下载 / 导入 / 复制 / 另存
 * 在**落盘那一刻**存一份、老文件在**第一次被我们改写**（保存 / 写回 / 校准 / 归属）
 * **之前**补上；「编辑文件」模态框那颗「恢复默认值」读的就是它。
 *
 * 往后每次"诞生一份新文件"（复制 / 导入 / 另存）都要在这张表里同步落一份 ——
 * 保存 / 改名这些**改写**动作则不碰它（改名只搬家）。
 */
const mockLocalOrigin = new Map<string, string>(mockLocalText)

/* ---------- 逐参数「云端更新」（2026-10-09）----------
 *
 * 真机上这件事由三份东西凑出来：
 *
 *   我那份文件      `presets-local/…`（盘上，用户唯一可见的那一份）
 *   云端旧值        隐藏 baseline（`<appDataDir>/baseline/<sha>.toml`，按内容摘要寻址）
 *   云端当前版      目录里那一版的字节（`<appDataDir>/<catalog.path>`）
 *
 * 假后端没有文件系统，用**同一形状的内存夹具**顶上（不是另编一套形状）：
 *
 *   基准区   `mockParamBaseline`：sha → 那一版云端正文
 *   当前版   `mockParamCurrent`：文件名 → 云端当前版（sha + 正文）
 *   决定账   `mockParamDecisions`：路径 → 参数 key → { sha, kind }（真机 run/app-state.json）
 *
 * 「云端当前版」那段正文比基准那版**真改了两个值**（`speed_limit` 70→65、
 * `offset_y` 18.6→19.0）—— 于是演示那份 `A1 涂胶.toml` 真有两条待处理。
 */

/** 与演示正文里那两行 `# based_on_sha256` 对上（`A1 涂胶.toml` / `…-2026-10-15.toml`） */
const SHA_BASED_OLD = '0'.repeat(64)
const SHA_BASED_NEW = '1'.repeat(64)

/** 云端**当前版**的演示正文（与 `MOCK_PRESET_TEXT` 差两项） */
const MOCK_PRESET_NEXT = [
  '# 假后端的演示正文 —— 云端当前版（比基准那版改了两个值）',
  '# release_time: 2026-10-15 09:00:00',
  '[toolhead]',
  'speed_limit = 65 # 速度上限(mm/s)',
  'offset_x = -1 # 笔尖偏移',
  'offset_y = 19.0 # 笔尖偏移',
  'offset_z = 4 # 笔尖偏移',
  '',
  '[wiping]',
  'mode = "tower"',
  '',
].join('\n')

const mockParamBaseline = new Map<string, string>([
  [SHA_BASED_OLD, MOCK_PRESET_TEXT],
  [SHA_BASED_NEW, MOCK_PRESET_NEXT],
])

const mockParamCurrent = new Map<string, { sha256: string; text: string }>([
  /* sha 与 `MOCK_PRESET_VERSIONS` 里 A1-fast 的「当前版」那一条是同一个值 */
  ['A1-fast.toml', { sha256: 'f2'.padEnd(64, '0'), text: MOCK_PRESET_NEXT }],
])

const mockParamDecisions = new Map<
  string,
  Map<string, { sha256: string; kind: ParamDecisionKind }>
>()

/** 文件头 `# <key>: <值>`（与真机 `runtime::lineage` 同一条解析规矩：键后必须是冒号） */
function mockHeaderOf(text: string, key: string): string | null {
  const re = new RegExp(`^#\\s*${key}\\s*:\\s*(.+)$`)
  for (const line of text.split('\n')) {
    const hit = re.exec(line.trim())
    if (hit !== null) {
      const value = hit[1]!.trim()
      if (value !== '') return value
    }
  }
  return null
}

/** 一份用户预设现在的云端更新账（真机：`ipc::param_sync::sync_of`） */
function mockSyncOf(path: string): PresetParamSync {
  const text = mockLocalText.get(path)
  if (text === undefined) throw new Error(`找不到 ${path}`)
  const fileName = path.split('/').pop() ?? path
  const basedOn = mockHeaderOf(text, 'based_on')
  const cloudFileName = basedOn === null ? null : (basedOn.split('/').pop() ?? null)
  const current = cloudFileName === null ? undefined : mockParamCurrent.get(cloudFileName)
  const basedSha = mockHeaderOf(text, 'based_on_sha256')
  const localValues = readMockParams(text)
  const currentValues = current === undefined ? undefined : readMockParams(current.text)
  const book = mockParamDecisions.get(path)

  const entries: ParamSyncEntry[] = []
  let pendingCount = 0
  for (const d of rawParamDefs()) {
    const decision = book?.get(d.key)
    const refSha = decision?.sha256 ?? basedSha
    const refText = refSha === null || refSha === undefined ? undefined : mockParamBaseline.get(refSha)
    const old = refText === undefined ? undefined : readMockParams(refText)[d.key]
    const neu = currentValues?.[d.key]
    const pending = old !== undefined && neu !== undefined && old !== neu
    if (pending) pendingCount += 1
    const localValue = localValues[d.key] ?? null
    entries.push({
      key: d.key,
      local: localValue,
      baselineOld: old ?? null,
      cloudNew: neu ?? null,
      pending,
      /* 冲突：云端改过它、**我也改过它**，两边的值还不一样（与 Rust 同一式子） */
      conflict:
        localValue !== null &&
        old !== undefined &&
        neu !== undefined &&
        old !== neu &&
        localValue !== old &&
        localValue !== neu,
      decided:
        decision !== undefined && current !== undefined && decision.sha256 === current.sha256
          ? decision.kind
          : null,
    })
  }

  return {
    path,
    fileName,
    machineId: mockHeaderOf(text, 'machine'),
    versionId: mockHeaderOf(text, 'variant'),
    cloudFileName,
    basedOnReleaseTime: mockHeaderOf(text, 'based_on_release_time'),
    currentReleaseTime: current === undefined ? null : mockHeaderOf(current.text, 'release_time'),
    cloudReady: current !== undefined,
    versionAdvanced: basedSha !== null && current !== undefined && basedSha !== current.sha256,
    pendingCount,
    entries,
  }
}

const nowSec = () => Math.floor(Date.now() / 1000)

/**
 * 演示口径的交付预设集合与它们的可信档（与 `getDownloadedFiles` / `getDeliveryTrust` /
 * `getStaleFiles` 同一套演示账）。下载即得工作副本那条链要按它判"这份能不能到你手里"。
 *
 * ★★ 2026-10-14：**从 `mockCatalogFiles()` 派生**，不再手抄一份。
 *   原来这里手写三条（standard/fast/mini-standard），而目录（`mockCatalogFiles`）后来
 *   加了 `A1mini-fast.toml` —— **同一份"目录"两本账**：预设页判"这份在不在目录里"用
 *   这张表（⇒ 新份被判"目录里没有"），下载那条用目录（⇒ 能下）—— 撕裂了。
 *   可信档（trust）是**演示**性质，单独一张小表挂上去；名单本身以目录为唯一出处。
 */
const MOCK_TRUST: Record<string, 'ok' | 'old' | 'tampered'> = {
  'A1-standard.toml': 'ok',
  'A1-fast.toml': 'old',
  'A1mini-standard.toml': 'tampered',
}

const MOCK_RELEASE_FILES = mockCatalogFiles()
  .filter((f) => f.kind === 'mkp_preset')
  .map((f) => ({
    fileName: f.fileName,
    machineId: f.machineId,
    versionId: f.versionId,
    /* 没挂演示档的那几份照实算 ok（新加的份默认"与目录一致"） */
    trust: MOCK_TRUST[f.fileName] ?? ('ok' as const),
  }))

const mockPresetFile = (fileName: string) =>
  MOCK_RELEASE_FILES.find((f) => f.fileName === fileName)

/**
 * 改演示正文里的三轴偏移（真机上是 `presetdata::patch` 按注册表定位：只换那个值，
 * 注释、键序、血统三行一个字节不动）。行尾注释保留。
 */
const patchMockOffsets = (text: string, axes: Axes): string =>
  text
    .split('\n')
    .map((line) => {
      const hit = /^(\s*offset_([xyz])\s*=\s*)(-?[\d.]+)(.*)$/.exec(line)
      if (hit === null) return line
      const value = hit[2] === 'x' ? axes.x : hit[2] === 'y' ? axes.y : axes.z
      return `${hit[1]}${value}${hit[4]}`
    })
    .join('\n')

/** 从演示正文里读一个数字（真机上是 `local::calibration_of` 按注册表读） */
const readMockNumber = (text: string, key: string): number | null => {
  const hit = new RegExp(`^\\s*${key}\\s*=\\s*(-?[\\d.]+)`, 'm').exec(text)
  if (hit === null) return null
  const n = Number(hit[1])
  return Number.isFinite(n) ? n : null
}

/** 从演示正文里读三轴偏移；缺一个轴就是 `null`（与真机同一口径，不拿半个基准充数） */
const readMockAxes = (text: string): Axes | null => {
  const x = readMockNumber(text, 'offset_x')
  const y = readMockNumber(text, 'offset_y')
  const z = readMockNumber(text, 'offset_z')
  return x === null || y === null || z === null ? null : { x, y, z }
}

/* ---------- baseline + 对比台（2026-10-08）----------
 *
 * 真机上 baseline 是**隐藏的内部存储**（按内容摘要寻址）、云端版本来自目录 + 版本链，
 * 假后端两样都没有 —— 用一份**固定的演示版本账**顶上，形状与真后端一致：
 * 每个演示预设给两版（旧的一版算「已下载」、新的一版算「新版本」）。
 *
 * 它**不是"过时判定"**：这里不读用户预设的字节、也不给任何一行挂"该更新了"。
 */
const MOCK_PRESET_VERSIONS: CloudVersion[] = [
  {
    fileName: 'A1-standard.toml',
    sha256: 'a1'.padEnd(64, '0'),
    releaseTime: '2026-09-30 05:46:00',
    publishedAt: '2026-10-06T05:46:00Z',
    downloaded: true,
    current: false,
  },
  {
    fileName: 'A1-standard.toml',
    sha256: 'a2'.padEnd(64, '0'),
    releaseTime: null,
    publishedAt: '2026-10-15T05:46:00Z',
    downloaded: false,
    current: true,
  },
  {
    fileName: 'A1-fast.toml',
    sha256: 'f1'.padEnd(64, '0'),
    releaseTime: '2026-05-29 04:26:12',
    publishedAt: null,
    downloaded: true,
    current: false,
  },
  {
    fileName: 'A1-fast.toml',
    sha256: 'f2'.padEnd(64, '0'),
    releaseTime: null,
    publishedAt: '2026-10-15T05:46:00Z',
    downloaded: false,
    current: true,
  },
]

/** 注册表里那三格（`CatalogParamDef` 的 TS 声明只长到消费面，假后端这里要 tomlKey） */
type RawParamDef = { key: string; section: string; tomlKey: string; valueType: string }

const rawParamDefs = (): RawParamDef[] => catalogRegistry().params as unknown as RawParamDef[]

/** 去掉行尾注释（引号里的 `#` 不算；演示数据够用） */
function stripMockComment(raw: string): string {
  let quote: string | null = null
  for (let i = 0; i < raw.length; i += 1) {
    const c = raw[i]!
    if (quote !== null) {
      if (c === quote) quote = null
      continue
    }
    if (c === '"' || c === "'") {
      quote = c
      continue
    }
    if (c === '#') return raw.slice(0, i)
  }
  return raw
}

/** 引号去掉（`"tower"` / `'tower'` → `tower`）；裸值原样 */
function unquoteMock(raw: string): string {
  const t = raw.trim()
  if (t.length >= 2 && ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'")))) {
    return t.slice(1, -1)
  }
  return t
}

/**
 * 极简 TOML 读取（**只为假后端**）：`[section]` + `key = value` →
 * `section\0key` 映射到**控件看得懂的值**（与真机 `presetdata::params` 同一口径）。
 */
function parseMockToml(text: string): Map<string, string> {
  const out = new Map<string, string>()
  const lines = text.split('\n')
  let section = ''
  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i]!.trim()
    if (trimmed === '' || trimmed.startsWith('#')) continue
    const sec = /^\[([^\]]+)\]$/.exec(trimmed)
    if (sec !== null) {
      section = sec[1]!
      continue
    }
    const kv = /^([\w"'-]+)\s*=\s*(.*)$/.exec(trimmed)
    if (kv === null) continue
    const key = unquoteMock(kv[1]!)
    const raw = kv[2]!.trim()
    if (raw.startsWith('"""')) {
      const body: string[] = [raw.slice(3)]
      while (!body[body.length - 1]!.includes('"""') && i + 1 < lines.length) {
        i += 1
        body.push(lines[i]!)
      }
      const joined = body.join('\n')
      const end = joined.indexOf('"""')
      out.set(`${section}\u0000${key}`, end < 0 ? joined : joined.slice(0, end))
      continue
    }
    out.set(`${section}\u0000${key}`, unquoteMock(stripMockComment(raw)))
  }
  return out
}

/** 按注册表把一份演示正文里的参数抽出来（真机：`presetdata::params::read_param_values`） */
function readMockParams(text: string): Record<string, string> {
  const sections = parseMockToml(text)
  const out: Record<string, string> = {}
  for (const d of rawParamDefs()) {
    const hit = sections.get(`${d.section}\u0000${d.tomlKey}`)
    if (hit !== undefined) out[d.key] = hit
  }
  return out
}

/** 只换那一行里的值（真机：`presetdata::patch` 保真写回）；行尾注释留着 */
function patchMockParam(text: string, def: RawParamDef, value: string): string {
  const literal = literalFor(value, def.valueType)
  const lines = text.split('\n')
  let section = ''
  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i]!.trim()
    const sec = /^\[([^\]]+)\]$/.exec(trimmed)
    if (sec !== null) {
      section = sec[1]!
      continue
    }
    if (section !== def.section) continue
    const hit = /^(\s*)([\w"'-]+)(\s*=\s*)(.*)$/.exec(lines[i]!)
    if (hit === null || unquoteMock(hit[2]!) !== def.tomlKey) continue
    /* 行尾注释留着（`# 速度上限(mm/s)` 这类）—— 只换值 */
    const comment = hit[4]!.slice(stripMockComment(hit[4]!).length).trimEnd()
    lines[i] = `${hit[1]}${hit[2]}${hit[3]}${literal}${comment === '' ? '' : ` ${comment}`}`
    return lines.join('\n')
  }
  throw new Error(`演示正文里没有 [${def.section}].${def.tomlKey} —— 这一项改不动`)
}

/**
 * 把一个控件值写成 TOML 字面量（假后端版）。
 *
 * 与真后端 `presetdata::patch::to_toml_value` **同一条规则**：形态由 `valueType` 定，
 * 不由"有没有 choices"定（`prime_enabled` 选项写 off/on，但 TOML 里是布尔）。
 */
function literalFor(raw: string, valueType: string): string {
  const t = raw.trim()
  switch (valueType) {
    case 'int':
    case 'float':
      return t
    case 'bool':
      return ['true', '1', 'on', 'yes'].includes(t.toLowerCase()) ? 'true' : 'false'
    default:
      /* 文本 / G-code：带换行就写多行字面量，否则普通串 */
      return t.includes('\n') ? `"""\n${t}"""` : `"${t.replace(/"/g, '\\"')}"`
  }
}

/*
 * 浏览器里的「下载区」：四份，**固定演示集合**（真机上是盘 `mkp/` + `assets/`，盘就是底账）。
 *
 * 为什么不是一份：预设页的交付行有四种状态（未下载 / 已下载 / 旧版本 / 内容异常），
 * 只给"未下载"一种的话，另外三种在浏览器里**根本画不出来** —— 而它们正是这一层
 * 最需要被看见的东西（"内容异常"尤其：那一档以前会被显示成"需更新"）。
 *
 * 前三份各占一档，**同一档里的两份不存在**：交付构造上每个 (机型, 版本) 只有一份 **MKP 产物**，
 * 再塞一份同版本的 MKP 条目就是**编形状**了。「未下载」那一档在浏览器里由云端行的「下载」
 * 按钮覆盖（同一套动作列），真机上则由"目录里登记了、下载区还没有"的那些行覆盖。
 *
 * 第四份是**套餐的另一半**（`A1_STANDARD` 这个 bundle 配的切片器配置），2026-10-06 补：
 * 首页那颗「下载并应用」消费的是**整个套餐**（MKP + 配套 BBS），少了这一条，
 * 浏览器里就画不出「全齐且全新 → 应用」那一态（永远显示「缺」）。它不违反上面那条 ——
 * 它是**另一种 kind**，不是同一 (机型, 版本) 的第二份 MKP。
 *
 * 哪一档都能看见（首页那颗按钮的四态就靠这几份编排）：
 *   A1/STANDARD       MKP 已下载 + BBS 已下载          → 「应用」
 *   A1/FAST           MKP 在盘上是旧的 + BBS 已下载     → 「更新并应用」
 *   A1_MINI/STANDARD  MKP 在盘上是旧的 + 配套 BBS 不在盘上 → 「下载并应用」（缺优先于漂）
 *   目录里没登记的机型（P1S…）                          → 「套餐未配置」
 *
 * ★ 两张单子是**互斥**的、都只收盘上真有的文件（漂移 = 盘上有但字节对不上），
 *   所以判「在不在盘上」要 `已下载 ∪ 漂移`（见 `PageHome` 的 `bundleState`）。
 *
 * 这四份都**不是真的能下**：点「下载」/「更新」/「重新下载」仍如实抛"浏览器里没有下载区"，
 * 见 `downloadCatalogFile`。
 *
 * `downloadedUnix / replacedUnix / publishedAt` 是固定演示值（真机上来自事件账与
 * 版本身份反查）：形状不编（`number | null` / `string | null`），数字编 ——
 * 界面按它们把「下载时间 / 替换时间 / 本机这份发布于」画出来。
 */
/*
 * ★★ 2026-10-14：下载区从**固定演示集合**改成**可变内存态** —— 数据可以是假的，
 * 但**逻辑必须是真的**（作者：「它数据是假，但是它那个逻辑那些要真的呀」）。
 *
 * 「下载 / 更新 / 删除」在浏览器里现在走的是与真机**同一条状态机**：
 * 下载会把一份加进来（还带水位事件）、删除会把一份拿走、重新下载会把 `stale` 洗成 `ok`。
 * 上一版这些都是写死的数组 ⇒ 界面上"下载成功了"，底账永远不变 —— 那不是演示，是撒谎。
 */
let mockDownloaded: OnDiskFile[] = [
  {
    fileName: 'A1-standard.toml',
    downloadedUnix: 1780000000,
    replacedUnix: null,
    publishedAt: '2026-10-06T05:46:00Z',
  },
  {
    /** 套餐的另一半：`A1_STANDARD` 的切片器配置（见上面那段说明） */
    fileName: 'MKPProcess A1 0.4 0.20.json',
    downloadedUnix: 1780000000,
    replacedUnix: null,
    publishedAt: '2026-10-06T05:46:00Z',
  },
]
let mockStale: OnDiskFile[] = [
  { fileName: 'A1-fast.toml', downloadedUnix: 1777000000, replacedUnix: null, publishedAt: null },
  {
    fileName: 'A1mini-standard.toml',
    downloadedUnix: null,
    replacedUnix: 1777000000,
    publishedAt: null,
  },
]

/** 下载区里 MKP toml 的**演示正文**（下载时种下，`readDownloadedText` 读它） */
const mockDeliveredText = new Map<string, string>()

/** 归档区（内存态）：`deleteArchivedFile` 真的会从这里拿走一条 */
let mockArchived: ArchivedFile[] = [
  {
    path: 'archive/delivery/mkp/presets/A1-fast.toml',
    fileName: 'A1-fast.toml',
    size: 2048,
    sha256: 'demo0000000000000000000000000000000000000000000000000000000000',
    publishedAt: '2026-10-06T05:46:00Z',
    replacedUnix: 1780000000,
    machineId: 'A1',
    versionId: 'FAST',
    kind: 'mkp_preset',
  },
]

/** 模拟网络节奏的一拍（真机的水位由下载进度推，这里按时间推 —— 形状一致） */
const tickWait = (ms: number) => new Promise((r) => setTimeout(r, ms))

/**
 * **把一份交付文件"下载"进内存下载区**（浏览器版 `runtime::delivery::deliver`）。
 *
 * 与真机同一条状态机：
 *   connecting → transferring（有总长度就报百分比）→ done；
 *   落盘 = 加进 `mockDownloaded`、把 `mockStale` 里同一份洗掉（重新下载 = 换成干净的）、
 *   MKP toml 顺手种一份演示正文（`readDownloadedText` 读它）。
 * `onTick` 不给就不发（与真机 Channel 可选同一个口径）。
 */
async function mockDeliver(
  fileName: string,
  onTick?: (tick: DownloadTick) => void,
): Promise<void> {
  const file = mockPresetFile(fileName)
  if (file === undefined) throw new Error(`目录里没有 ${fileName} 这一份`)
  /* 总长度从目录拿（真机是 Content-Length / 目录登记的 expected_size） */
  const total = mockCatalogFiles().find((f) => f.fileName === fileName)?.size ?? 4096
  const send = (stage: DownloadTick['stage'], received: number) =>
    onTick?.({ fileName, stage, received, total })

  send('connecting', 0)
  await tickWait(180)
  const steps = 3
  for (let i = 1; i <= steps; i++) {
    send('transferring', Math.round((total * i) / steps))
    await tickWait(160)
  }
  send('done', total)

  mockStale = mockStale.filter((f) => f.fileName !== fileName)
  mockDownloaded = [
    ...mockDownloaded.filter((f) => f.fileName !== fileName),
    {
      fileName,
      downloadedUnix: Math.floor(Date.now() / 1000),
      replacedUnix: null,
      publishedAt: '2026-10-06T05:46:00Z',
    },
  ]
  /* MKP toml 种一份演示正文（与 `fetchCloudPreset` 落本地那份同源）；别的类型没有正文可种 */
  if (fileName.toLowerCase().endsWith('.toml') && !mockDeliveredText.has(fileName)) {
    mockDeliveredText.set(
      fileName,
      [
        `# ${fileName}（假后端演示正文 —— 下载区那份云端原件）`,
        `# machine: ${file.machineId}`,
        `# variant: ${file.versionId.toLowerCase()}`,
        `[toolhead]`,
        `offset_y = 20.0`,
        '',
      ].join('\n'),
    )
  }
}

export const mockApi: MkpApi = {
  /*
   * 校准值写进「我的那一份」（2026-10-08 作者改判：不再是独立的 offsets.json）。
   * 落点带 path（`presets-local/…`），所以在浏览器里可以真的改那份演示正文 ——
   * 校准页保存后重进，`getLocalCopyFor` 读到的就是刚写的值（同一份内存文本）。
   * 真机上是 `local::save_preset_calibration`（按注册表定位、注释一个字节不动）。
   */
  async savePresetCalibration(path, axes) {
    const text = mockLocalText.get(path)
    if (text === undefined) {
      throw new Error(`${path} 读不出来 —— 先把这份另存为本地预设，再保存校准`)
    }
    mockLocalText.set(path, patchMockOffsets(text, axes))
    const at = mockLocal.findIndex((f) => f.path === path)
    if (at >= 0) mockLocal[at] = { ...mockLocal[at], modifiedUnix: nowSec() }
  },

  async getCalibModels() {
    return calibModels
  },

  async openModel(modelId, opts) {
    /* 浏览器里没有壳、真开不了 —— 把「用哪个打开」照实说出来，真开由桌面版接 */
    console.info(
      '[mock] openModel',
      modelId,
      'slicer =',
      opts?.slicer ?? 'system',
      'slicerApp =',
      opts?.slicerApp ?? '',
    )
  },

  /*
   * 「其他应用…」—— 浏览器里没有系统选择器，给一份"从选择器来的"演示应用
   * （与 `pickImportFiles` 同一路数：只把流程走通，不装文件系统）。
   * 真机上由 Rust `pick_slicer_app` 给真路径（macOS 是 `.app`、Windows 是 `.exe`）。
   */
  async pickSlicerApp() {
    const mac = /Mac/i.test(navigator.userAgent)
    return mac
      ? { name: 'PrusaSlicer', path: '/Applications/PrusaSlicer.app' }
      : { name: 'PrusaSlicer', path: 'C:\\Program Files\\Prusa3D\\PrusaSlicer\\prusa-slicer.exe' }
  },

  /*
   * 「跟随系统默认」到底指谁 —— 浏览器里**没有系统关联这回事**（没有 LaunchServices、
   * 也没有注册表），所以如实答 `null`：界面回落原来那句话术（"用 .3mf 关联的程序打开"）。
   * 与 `getPostProcessCommand` 同一条口径：这一格是要显示给用户看的**名字**，
   * 编一个像模像样的出来比空着更坏（真机上由 Rust `get_system_slicer_app` 答）。
   */
  async getSystemSlicerApp() {
    return null
  },

  /*
   * 「去系统里改默认应用」：浏览器里一个系统窗口都开不了（没有访达、没有「打开方式」框），
   * **照实拒** —— 与 `getFileUrl` / `checkRemoteUpdate` 同一条口径：这一步是桌面版的事。
   */
  async openDefaultAppSettings() {
    throw new NotImplementedError(
      'openDefaultAppSettings',
      '浏览器预览里打不开系统那一处 —— 改默认应用请用桌面版（SupportEase 应用）',
    )
  },

  /*
   * 「自动化」授权面板：浏览器里更没有系统设置可开 —— 照实拒（与上一条同口径）。
   */
  async openAutomationSettings() {
    throw new NotImplementedError(
      'openAutomationSettings',
      '浏览器预览里打不开系统设置 —— 授权请用桌面版（SupportEase 应用）',
    )
  },

  /*
   * 「复制后处理脚本」那一整条命令 —— 浏览器里没有本机可执行物、也没有盘上真实落点，
   * 如实答 `ready: false` + 原因（真机那份由壳现拼：`current_exe()` + 预设绝对落点）。
   * **不编一条假命令**：编出来的东西贴进切片器只会让人以为"复制成功了"，其实指不到东西。
   * 按钮照摆（那是界面规则）—— 点它就显示这一句原因。
   */
  async getPostProcessCommand() {
    return {
      ready: false,
      reason: '浏览器预览里没有本机的可执行物与预设文件 —— 命令由桌面版按真实路径现拼',
    }
  },

  /*
   * 钩子那一趟：浏览器里不可能有 —— 那一趟是**切片器带参数把本程序拉起来**的那一次
   * （`--Toml/--Gcode`），浏览器根本没有这个进程。所以快照如实答 `null`
   * （界面那屏模态框就不出现），另两条点不到、真被调也只报"没有在跑"。
   */
  async getPostProcessRun() {
    return null
  },

  async cancelPostProcess() {
    throw new NotImplementedError(
      'cancelPostProcess',
      '浏览器预览里没有在跑的后处理 —— 那一趟由切片器带参数拉起来（用桌面版试）',
    )
  },

  async answerPostProcessMismatch() {
    throw new NotImplementedError(
      'answerPostProcessMismatch',
      '浏览器预览里没有在跑的后处理 —— 那一趟由切片器带参数拉起来（用桌面版试）',
    )
  },

  /* ——— 「客户端接发布包」这一轮（P1）新增的十二个 ———
   *
   * 数据来自 `src/api/mockServer/`：那份假后端把 `data/*.json` 的六份上游快照
   * （机型目录 / 参数注册表 / 布局表 / 资产清单 / 套餐 / 切片器文件事实）
   * 合成下面这些形状。6 台机型 / 10 个版本 / 74 条参数 / 43 条带条件。
   *
   * **只答产品仓契约里有的那十二个** —— 工作台那一侧的方法（配方本、套餐定义、
   * 发布检查…）没有搬，契约里没有它们。真机上换成 Rust 侧实现时，换掉的是
   * `mockServer/` 这一整个目录，下面这十二行一行不动。
   */
  async getMachines() {
    return allMachines()
  },

  async getVersionFiles(machineId, versionId) {
    /* null 的语义是「后端没有这个组合」，不是「这个组合下没文件」 */
    return resolveVersionFiles(machineId, versionId)
  },

  async getLocalFiles() {
    /* 固定演示集合 —— 假后端没有文件系统，见 mockServer/localFiles.ts 文件头 */
    return localFileIds()
  },

  /* 用户线：两份**固定演示**（一份 `.toml` 认得出、一份 `.json` 认不出）+ 这份会话另存出来的 */
  async getLocalPresetFiles() {
    return mockLocal.map((f) => ({ ...f }))
  },

  /* 备注覆盖账（副标题）：整本给 / 改一条。空串也是覆盖（不回退）；null = 恢复默认 */
  async getPresetRemarks() {
    return Object.fromEntries(mockRemarks)
  },

  async setPresetRemark(key, remark) {
    const k = key.trim()
    if (k === '') throw new Error('备注要说是哪一份（缺文件身份）')
    if (remark === null) mockRemarks.delete(k)
    else mockRemarks.set(k, remark.trim())
  },

  /*
   * 改归属：改文件头的 `# machine:` / `# variant:` 两行（与真机同一个闸：
   * 机型必须目录里真有、**版本可自定义**；正文一个字节不动）。
   */
  async setLocalPresetMachineVersion(path, machineId, versionId) {
    const mid = machineId.trim()
    const vid = versionId.trim()
    if (mid === '' || vid === '') {
      throw new Error('机型与版本都要填 —— 归属写的是文件头的那两行，留空就认不出')
    }
    const hit = mockLocal.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const known = allMachines().some((m) => m.id === mid)
    if (!known) {
      throw new Error(`${mid} 不是目录里登记的机型 —— 归属的机型要选一台真的机器`)
    }
    const text = mockLocalText.get(path)
    if (text !== undefined) {
      mockLocalText.set(path, rewriteMockMachineVariant(text, mid, vid))
    }
    hit.machineId = mid
    hit.versionId = vid
  },

  /** 只有这份会话另存出来的那份有正文可读（真机上每一份都能读） */
  async readLocalPresetText(path) {
    const text = mockLocalText.get(path)
    if (text === undefined) {
      throw new NotImplementedError(
        'readLocalPresetText',
        '浏览器预览里读不到这一份的正文 —— 用桌面版就能看（每一份都读得到）',
      )
    }
    return text
  },

  /*
   * 临时编辑那条链：内存里真的走一遍（改的是临时文件，云端原件一动不动）。
   * 与真机同一个形状 —— 直道里的分岔只有一条：正文来自演示常量而不是 `mkp/` 里的字节。
   */
  async beginPresetEdit(fileName, origin = 'preset', path) {
    /* 两条线的钥匙：云端线认文件名，用户线认路径（用户目录里同名很正常） */
    if (mockDraft !== null && mockDraft.origin === origin && mockDraft.sourceFileName === fileName) {
      if (origin === 'preset' || mockDraft.path === (path ?? null)) {
        return { ...mockDraft, reused: true }
      }
    }
    if (origin === 'local') {
      const rel = path ?? ''
      /* 第九层：读不出来的那份**不给改**（与真机同一个入口闸；消息形状也对齐后端） */
      const entry = mockLocal.find((f) => f.path === rel)
      if (entry?.state === 'unreadable') {
        throw new Error(
          `${rel} 读不出来：${entry.stateDetail ?? 'TOML 语法不对'} —— 这一份现在不能应用、也不能改`,
        )
      }
      const raw = mockLocalText.get(rel)
      if (raw === undefined) {
        throw new NotImplementedError(
          'beginPresetEdit',
          `浏览器预览里读不到 ${rel} 的正文 —— 用桌面版就能改（每一份都读得到）`,
        )
      }
      /* 编辑器里给的是正文：那三行血统是程序的元数据，不是用户该改的内容 */
      mockDraft = {
        origin,
        sourceFileName: fileName,
        path: rel,
        text: withoutLineage(raw),
        updatedUnix: nowSec(),
      }
      return { ...mockDraft, reused: false }
    }
    mockDraft = {
      origin,
      sourceFileName: fileName,
      path: null,
      text: MOCK_PRESET_TEXT,
      updatedUnix: nowSec(),
    }
    return { ...mockDraft, reused: false }
  },

  async putPresetDraft(text) {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份')
    mockDraft = { ...mockDraft, text, updatedUnix: nowSec() }
  },

  /**
   * 按参数 key 改草稿里的一个值（参数页底座）。
   *
   * 假后端做的与真后端**同一件事**：查字段定义拿 `(section, toml_key)`，把那一行换掉。
   * 但**不做保真**（注释 / 键序）—— 那是 `toml_edit` 的活，浏览器里没有；
   * 这里按行替换已经很够探针用（探针量的是"改值这条路通不通"，不是格式保真）。
   * 真机上的保真由 `presetdata::patch` 的 7 条单测钉着。
   */
  async patchPresetDraft(paramKey, value) {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份，改不了参数')
    const def = paramMeta().find((p) => p.key === paramKey)
    if (def === undefined) throw new Error(`不认识这个参数：${paramKey}`)
    const { section, tomlKey, valueType } = def
    const literal = literalFor(value, valueType)
    /* 找 `[section]` 那一段，在段内替换 `<tomlKey> = …` 那一行 */
    const lines = mockDraft.text.split('\n')
    let inSection = false
    let done = false
    for (let i = 0; i < lines.length; i++) {
      const t = lines[i].trim()
      if (t.startsWith('[')) {
        inSection = t === `[${section}]`
        continue
      }
      if (inSection && t.startsWith(`${tomlKey} =`)) {
        const hash = lines[i].indexOf('#')
        const comment = hash >= 0 ? ` ${lines[i].slice(hash).trim()}` : ''
        lines[i] = `${tomlKey} = ${literal}${comment}`
        done = true
        break
      }
    }
    if (!done) {
      throw new Error(`草稿的 [${section}] 里没有 ${tomlKey}`)
    }
    mockDraft = { ...mockDraft, text: lines.join('\n'), updatedUnix: nowSec() }
  },

  async discardPresetDraft() {
    mockDraft = null
  },

  /*
   * **修改历史账本**（2026-10-17）：浏览器预览里用内存顶替 —— 一份文件一本（钥匙同真机：
   * 哪条线 + 身份），于是"改过 → 关掉 → 再打开还在"这条在预览里也走得通，
   * 只是**退出浏览器就没了**（真机那份在盘上）。
   */
  async getPresetHistory(origin, id) {
    return mockHistory.get(historyKeyOf(origin, id)) ?? { batches: 0, entries: [] }
  },

  async putPresetHistory(origin, id, history) {
    const key = historyKeyOf(origin, id)
    /* 空本 = 没有这一本（与后端"清空就删掉文件"同一条规矩） */
    if (history.entries.length === 0 && history.batches === 0) mockHistory.delete(key)
    else mockHistory.set(key, { batches: history.batches, entries: [...history.entries] })
  },

  async clearPresetHistory(origin, id) {
    mockHistory.delete(historyKeyOf(origin, id))
  },

  /* 学习账：真机读 `run/usage.json`；浏览器里读内存（上面 `mockUsage`） */
  async getPresetUsage() {
    return [...mockUsage.entries()].map(([key, e]) => ({
      key,
      count: e.count,
      lastUnix: e.lastUnix,
    }))
  },

  /*
   * **备份**：浏览器预览里**没有家当可打包**（没有数据根），所以照实说没有 ——
   * 不假装打了一份（那会让用户以为自己有备份）。
   */
  async createBackup() {
    throw new NotImplementedError(
      'createBackup',
      '浏览器预览里打不了备份（没有本机数据）—— 用桌面版就能打（设置页这一块）',
    )
  },

  async restoreBackup() {
    throw new NotImplementedError(
      'restoreBackup',
      '浏览器预览里恢复不了备份（没有本机数据）—— 用桌面版就能恢复（设置页这一块）',
    )
  },

  /** 云端线：另存成 `presets-local/<原名>（已修改）.toml`（与 Rust 侧 `local::edited_name` 同一条规则） */
  async commitPresetDraft() {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份，没得存')

    /*
     * 用户线（第八层）：**写回它自己** —— 同一个路径、同一份文件，不产生第二份；
     * 血统**照抄原来那三行**（出处没变）。与 Rust 侧 `local::save_back` 同一件事。
     */
    if (mockDraft.origin === 'local') {
      const rel = mockDraft.path ?? ''
      const old = mockLocalText.get(rel)
      if (old === undefined) {
        throw new NotImplementedError(
          'commitPresetDraft',
          `浏览器预览里读不到 ${rel} 的正文，写不回去 —— 用桌面版就能存`,
        )
      }
      const text = [...lineageLinesOf(old), withoutLineage(mockDraft.text)].join('\n')
      const fileName = rel.slice(rel.lastIndexOf('/') + 1)
      const at = mockLocal.findIndex((f) => f.path === rel)
      if (at >= 0) mockLocal[at] = { ...mockLocal[at], size: text.length, modifiedUnix: nowSec() }
      mockLocalText.set(rel, text)
      mockDraft = null
      return { path: rel, fileName, size: text.length, replaced: true }
    }

    const fileName = mockDraft.sourceFileName.replace(/\.toml$/i, '（已修改）.toml')
    const path = `presets-local/${fileName}`
    /* 与真机同形：写下去的正文 = 草稿 + 文件头三行血统（真机上那三行由 `lineage::make_copy` 生成） */
    const label = `delivery/mkp/presets/${mockDraft.sourceFileName}`
    const text = `# based_on: ${label}\n# based_on_sha256: ${'0'.repeat(64)}\n${mockDraft.text}`
    const size = text.length
    const replaced = mockLocal.some((f) => f.path === path)
    const entry: LocalPresetFile = {
      path,
      fileName,
      size,
      modifiedUnix: nowSec(),
      kind: 'mkp_preset',
      /* 刚存出来的那份：假后端不解析 TOML，按演示口径记"能读"（真机上是后端算的） */
      state: 'ok',
      stateDetail: null,
      /* 刚存出来的这份就是基于**当前**目录那一版（演示里那份恰好对得上目录） */
      basedOn: 'current',
      basedOnLabel: label,
      basedOnRelease: null,
      basedOnMachineId: 'A1',
      basedOnVersionId: 'STANDARD',
      /* 归属与来源那份一致（正文头里也是这两行） */
      machineId: 'A1',
      versionId: 'STANDARD',
      /* 云端另存出来的：出处走 based_on 血统（界面上显示"复制自云端 X"），账本不重复记 */
      copiedFrom: null,
      copiedFromName: null,
      provenance: null,
    }
    if (replaced) {
      mockLocal[mockLocal.findIndex((f) => f.path === path)] = entry
    } else {
      mockLocal.push(entry)
      /* 另存出来的新文件：落盘那一刻就是它的原文（覆盖别人的分支不碰 —— 那份早就有原文了） */
      mockLocalOrigin.set(path, text)
    }
    mockLocalText.set(path, text)
    mockDraft = null
    /* 另一侧作废：盘上字节换了，「编辑文件」手上那份正文就成了孤儿 */
    mockTextDraft = null
    return { path, fileName, size, replaced }
  },

  /* ---------- 「编辑文件」那条链（2026-10-10：与参数草稿分开存） ---------- */

  /** 「恢复默认值」（编辑文件那颗）读的原文 —— 见 `mockLocalOrigin` 的注释 */
  async readLocalOriginText(path) {
    return mockLocalOrigin.get(path) ?? null
  },

  /** 参数草稿的**摘要** —— 预设列表行上的「未保存」标记用。没有草稿就是 `null` */
  async getPresetDraftSummary() {
    if (mockDraft === null) return null
    const base = mockDraft.origin === 'local' ? mockLocalText.get(mockDraft.path ?? '') : undefined
    const clean = mockDraft.origin === 'preset' ? mockDraft.text === MOCK_PRESET_TEXT : base === mockDraft.text
    return {
      origin: mockDraft.origin,
      sourceFileName: mockDraft.sourceFileName,
      path: mockDraft.path,
      /* 「真改过才算」：比不出来（演示里读不到这一份）时保守算脏 */
      dirty: !clean,
      updatedUnix: mockDraft.updatedUnix,
    }
  },

  /** 开始改一份用户预设的正文。已经有同一份（同路径）的草稿就**接着改** */
  async beginTextEdit(fileName, path) {
    if (mockTextDraft !== null && mockTextDraft.path === path) {
      /* ★ `dirty`（2026-10-16）：与盘上字节**比过** —— "有草稿"≠"改过"
         （打开过又关掉也算有草稿）；比不了（演示里读不到这一份）保守算脏，
         与真机 `begin_text_edit` 同一句口径 */
      const raw = mockLocalText.get(path)
      return {
        ...mockTextDraft,
        reused: true,
        dirty: raw === undefined ? true : raw !== mockTextDraft.text,
      }
    }
    const raw = mockLocalText.get(path)
    if (raw === undefined) {
      throw new NotImplementedError(
        'beginTextEdit',
        `浏览器预览里读不到 ${path} 的正文 —— 用桌面版就能改（每一份都读得到）`,
      )
    }
    /* 编辑器里这份**带着它自己的血统三行**（在这一份上有效，保存时照写回） */
    mockTextDraft = { fileName, path, text: raw, updatedUnix: nowSec() }
    /* 新建的草稿 = 盘上字节本身，不脏 */
    return { ...mockTextDraft, reused: false, dirty: false }
  },

  async putTextDraft(text) {
    if (mockTextDraft === null) throw new Error('现在没有正在改的那一份正文')
    mockTextDraft = { ...mockTextDraft, text, updatedUnix: nowSec() }
  },

  /** 保存：正文原子写回同一份用户文件，然后丢草稿 + 作废另一侧 */
  async commitTextDraft() {
    if (mockTextDraft === null) throw new Error('现在没有正在改的那一份正文，没得存')
    const rel = mockTextDraft.path
    const at = mockLocal.findIndex((f) => f.path === rel)
    if (at < 0) throw new Error(`找不到 ${rel} —— 它可能已经被移走或删掉了`)
    mockLocalText.set(rel, mockTextDraft.text)
    mockLocal[at] = { ...mockLocal[at], size: mockTextDraft.text.length, modifiedUnix: nowSec() }
    const fileName = rel.slice(rel.lastIndexOf('/') + 1)
    const size = mockTextDraft.text.length
    mockTextDraft = null
    mockDraft = null
    return { path: rel, fileName, size, replaced: true }
  },

  async discardTextDraft() {
    mockTextDraft = null
  },

  /**
   * **身份账本**（派生视图）：只给**血统还指着目录里活着的那一版**的那几份 ——
   * 与后端同一条口径（不用持久化账本，"盘就是底账"）。
   */
  async getLocalPresetIndex() {
    return mockLocal
      /* 只有**血统还指着目录里活着的那一版**的那几份才进账 */
      .filter((f) => f.basedOn === 'current' || f.basedOn === 'outdated')
      .map((f) => ({
        id: f.path,
        path: f.path,
        fileName: f.fileName,
        cloudSource: mockBaseName(f.basedOnLabel ?? f.fileName),
      }))
  },

  /*
   * 第十层：用户文件管理（改名 / 删除）。与真机同一套规矩，在内存里走一遍 ——
   * 改名要看得见"使用中指针与草稿跟着走"，删除要看得见两道闸（正在使用的 / 还有草稿的）。
   */
  async renameLocalPreset(path, newName) {
    const hit = mockLocal.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const problem = mockNameProblem(hit.fileName, newName)
    if (problem !== null) throw new Error(problem)
    const name = newName.trim()
    const dir = path.slice(0, path.length - hit.fileName.length)
    const newPath = `${dir}${name}`
    if (newPath !== path && mockLocal.some((f) => f.path === newPath)) {
      throw new Error(`已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`)
    }
    if (newPath !== path) {
      /* 使用中指针跟着改（指纹原样 —— 字节没动）；草稿跟着改（「接着上次改」不接丢） */
      if (mockActive?.origin === 'local' && mockActive.path === path) {
        mockActive = { ...mockActive, path: newPath, fileName: name }
      }
      if (mockDraft !== null && mockDraft.origin === 'local' && mockDraft.path === path) {
        mockDraft = { ...mockDraft, path: newPath, sourceFileName: name }
      }
      hit.path = newPath
      hit.fileName = name
      const text = mockLocalText.get(path)
      if (text !== undefined) {
        mockLocalText.delete(path)
        mockLocalText.set(newPath, text)
      }
      /* 原文镜像跟着搬家（改名只动名字，不动字节 —— 真机 `move_origin_copy`） */
      const origin = mockLocalOrigin.get(path)
      if (origin !== undefined) {
        mockLocalOrigin.delete(path)
        mockLocalOrigin.set(newPath, origin)
      }
      /* 逐参数「云端更新」决定账按**路径**认这一份 —— 改名之后它得跟到新路径上
         （不跟的话，云端下一次发布会把处理过的项全翻出来） */
      const book = mockParamDecisions.get(path)
      if (book !== undefined) {
        mockParamDecisions.delete(path)
        mockParamDecisions.set(newPath, book)
      }
    }
    return { path: newPath, fileName: name }
  },

  /*
   * 第十一层：另存为一份新的（本地文件 → 本地文件）。与真机同一套规矩：
   * **字节复制**（正文与血统原样带过去）→ 新的一份从诞生起就是独立的；
   * 名字由用户自己起（不自动改名）：和原来一样 / 已存在同名都拒；
   * **一个状态都不碰** —— 使用中指针与草稿都还留在原来那份上。
   */
  async copyLocalPreset(path, newName) {
    const hit = mockLocal.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const problem = mockNameProblem(hit.fileName, newName)
    if (problem !== null) throw new Error(problem)
    const name = newName.trim()
    const dir = path.slice(0, path.length - hit.fileName.length)
    const newPath = `${dir}${name}`
    if (newPath === path) throw new Error('新名字和原来一样 —— 复制要起个不同的名字')
    if (mockLocal.some((f) => f.path === newPath)) {
      throw new Error(`已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`)
    }
    const text = mockLocalText.get(path)
    if (text !== undefined) {
      mockLocalText.set(newPath, text)
      /* 副本诞生那一刻的字节就是它的原文（真机在 `copy_as_new` 落盘后写镜像） */
      mockLocalOrigin.set(newPath, text)
    }
    mockLocal.push({
      ...hit,
      path: newPath,
      fileName: name,
      modifiedUnix: nowSec(),
      /* 与真机同形：复制出来的那份在出处账里记着从哪来（界面上「来源：复制自 X」） */
      copiedFrom: path,
      copiedFromName: hit.fileName,
      provenance: 'copy' as const,
    })
    return { path: newPath, fileName: name }
  },

  /*
   * 这台机型 / 这个版本，「本地那份」在哪（校准页：初值从它读、保存写它）。
   * 与真机同一套匹配（归属优先、回落血统）—— 演示数据里 `A1 涂胶.toml` 就是
   * 血统指向 `A1-fast` 的那一份，所以选 A1/快拆版能读到它。
   */
  async getLocalCopyFor(machineId, versionId) {
    const hit = mockLocal.find(
      (f) =>
        f.kind === 'mkp_preset' &&
        f.state === 'ok' &&
        (f.machineId ?? f.basedOnMachineId) === machineId &&
        (f.versionId ?? f.basedOnVersionId) === versionId,
    )
    if (hit === undefined) return null
    /* 浏览器里没有盘 —— 绝对路径那一件事不在这份数据里（真机上是
       `<appDataDir>/user/presets-local/…`，由 `getPostProcessCommand` 现拼）。 */
    const text = mockLocalText.get(hit.path)
    if (text === undefined) {
      return { fileName: hit.fileName, path: hit.path, axes: null, speed: null }
    }
    return {
      fileName: hit.fileName,
      path: hit.path,
      axes: readMockAxes(text),
      speed: readMockNumber(text, 'speed_limit'),
    }
  },

  /*
   * 第十二层：通用导入入口（假后端的演示）。
   * 真机上源是**真路径**（拖拽 / 系统选择器给的），复制的是它的字节；
   * 假后端里那些外部路径不是真文件 —— 给一份演示正文，其余规矩与真机一致：
   * 只收 .toml、重名不覆盖（改名由界面走）、**一个状态都不碰**。
   */
  async pickImportFiles() {
    /* 浏览器里没有系统选择器：给一份"从选择器来的"演示路径（不撞现有的那种） */
    return ['/（文件选择器演示）/从选择器导进来.toml']
  },
  async stageImport(sources) {
    const taken = new Set(mockLocal.map((f) => f.fileName))
    return sources.map((source) => {
      const fileName = mockBaseName(source)
      if (fileName === '') {
        return {
          source,
          fileName: source,
          state: 'rejected',
          reason: '这个路径里没有文件名 —— 它不是一份能导入的文件',
        }
      }
      if (!/\.toml$/i.test(fileName)) {
        return {
          source,
          fileName,
          state: 'rejected',
          reason:
            '现在只收 .toml 预设 —— 这种文件还没有认领它的导入器（ZIP / 备份包以后再说）',
        }
      }
      if (taken.has(fileName)) return { source, fileName, state: 'collision', reason: null }
      taken.add(fileName)
      return { source, fileName, state: 'ready', reason: null }
    })
  },
  async commitImport(items) {
    const outcomes: {
      source: string
      ok: boolean
      path: string
      fileName: string
      message: string
    }[] = []
    for (const item of items) {
      const fileName = mockBaseName(item.source)
      if (!/\.toml$/i.test(fileName)) {
        outcomes.push({
          source: item.source,
          ok: false,
          path: '',
          fileName,
          message: '现在只收 .toml 预设 —— 这种文件还没有认领它的导入器',
        })
        continue
      }
      let name = fileName
      if (item.newName !== undefined) {
        const problem = mockNameProblem(fileName, item.newName)
        if (problem !== null) {
          outcomes.push({ source: item.source, ok: false, path: '', fileName, message: problem })
          continue
        }
        name = item.newName.trim()
      }
      const path = `presets-local/${name}`
      if (mockLocal.some((f) => f.path === path)) {
        outcomes.push({
          source: item.source,
          ok: false,
          path: '',
          fileName: name,
          message: `已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`,
        })
        continue
      }
      mockLocalText.set(path, MOCK_IMPORTED_TEXT)
      /* 导入落盘那一刻的字节就是它的原文（真机在 `commit_import` 写镜像） */
      mockLocalOrigin.set(path, MOCK_IMPORTED_TEXT)
      mockLocal.push({
        path,
        fileName: name,
        size: MOCK_IMPORTED_TEXT.length,
        modifiedUnix: nowSec(),
        kind: 'mkp_preset',
        state: 'ok',
        stateDetail: null,
        basedOn: 'unknown',
        basedOnLabel: null,
        basedOnRelease: null,
        basedOnMachineId: null,
        basedOnVersionId: null,
        /* 导入的裸文件没有归属（文件头里没有那两行）—— 照实 null */
        machineId: null,
        versionId: null,
        /* 与真机同形：导入进来的在出处账里记一档「导入」 */
        copiedFrom: null,
        copiedFromName: null,
        provenance: 'import',
      })
      outcomes.push({ source: item.source, ok: true, path, fileName: name, message: '' })
    }
    return outcomes
  },

  /*
   * 第十三层：在文件管理器里显示。浏览器里没有 Finder / 资源管理器，假后端的"文件"
   * 也只是内存里一条记录 —— 前三步（只认本地文件 / 找得到 / 文件在不在）照真机走，
   * 最后一步如实说它在真机上的样子（不假装打开了）。
   */
  /*
   * 两个落点各自的「打开所在文件夹」（2026-10-13）：假后端没有文件系统，**照实说
   * 它在真机上的样子**，不假装打开了（假装的话浏览器里看着像成了）—— 与 `revealInFolder`
   * / `revealReportExport` 同一条口径。
   */
  async revealDeliveredFile(fileName) {
    throw new Error(
      `假后端没有文件系统 —— 真机上这一步会打开文件管理器，选中我们自己的那份「${fileName}」`,
    )
  },

  async revealSlicerFile(fileName) {
    throw new Error(
      `假后端没有文件系统 —— 真机上这一步会打开文件管理器，选中切片器目录里的「${fileName}」`,
    )
  },

  async revealInFolder(path) {
    if (!path.startsWith('presets-local/')) {
      throw new Error('只给「本地文件」显示 —— 云端那两份住程序自己管的目录')
    }
    if (!mockLocal.some((f) => f.path === path)) {
      throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了（列表以磁盘为准，刷新一下）`)
    }
    throw new Error(
      `假后端没有文件系统 —— 真机上这一步会打开 Finder / 资源管理器并选中「${path}」`,
    )
  },

  async deleteLocalPreset(path) {
    const i = mockLocal.findIndex((f) => f.path === path)
    if (i === -1) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    /* 与真机同语义（2026-10-06 一切皆可删）：属于这一份的状态一并清掉，不再拦 */
    if (mockActive?.origin === 'local' && mockActive.path === path) mockActive = null
    if (mockDraft !== null && mockDraft.origin === 'local' && mockDraft.path === path) {
      mockDraft = null
    }
    mockLocal.splice(i, 1)
    mockLocalText.delete(path)
    /* 原文镜像一起走（这份都没了，"它最初的样子"不该留着） */
    mockLocalOrigin.delete(path)
    /* 备注覆盖跟着删（删了重新下载 / 重新复制 = 回到工作台那句） */
    mockRemarks.delete(path)
    /* 逐参数「云端更新」决定账跟着删：文件都不在了，留着一本账只会是悬空的 */
    mockParamDecisions.delete(path)
  },

  async getSlicerCopied() {
    return slicerCopied()
  },

  async copyToSlicer(fileName) {
    /* 只改内存，刷新还原。传错类型会抛 —— 静默成功比报错难查得多。
       同一份复制第二遍就是「替换」（真机上切片器目录里那份被换成我们这份） */
    return { replaced: copyToSlicerIn(fileName) }
  },

  /*
   * 本机 BBS 目录（工艺页「本机工艺」那一档）：浏览器里没有盘，给一份**演示集合**
   * （6 份：链根 + 两个抽象中间层 + 三台机的标准档 + 一份用户档）。
   * 清单与正文都从同一份数据推出来 —— 不会出现"列了却读不出来"。见 `mockServer/bbsLocal.ts`。
   */
  async getBbsPresets() {
    return bbsLocalTree
  },

  async readBbsPreset(scope, uid, file) {
    const text = bbsPresetText(scope, uid, file)
    if (text === null) {
      throw new Error(`演示集合里没有这一份：${scope} / ${uid ?? '（无账号）'} / ${file}`)
    }
    return text
  },

  async getPresetFiles() {
    return allPresetFiles()
  },

  async getMenu() {
    return menuEntries()
  },

  async getParamMeta() {
    return paramMeta()
  },

  async getMachineParams(machineId, versionId) {
    /* 已按机器过滤掉 machineFilter 不适用的、并排除 deprecated / hidden 的字段 */
    return resolveParams(machineId, versionId)
  },

  /**
   * 「复制链接」要的是云端 URL，而云端地址在**真机的数据源设置**里 ——
   * 浏览器预览里没有它。**照实拒**，不编一个假 URL 让用户复制出去（那比报错糟得多）。
   */
  async getFileUrl() {
    throw new NotImplementedError(
      'getFileUrl',
      '浏览器预览里没有云端源地址 —— 复制链接请用桌面版（SupportEase 应用）',
    )
  },

  /**
   * 报告页的数据是 `mkp-ssr` 钩子落在本机的执行账 —— 浏览器里没有那棵树。
   * **照真机 2026-10-10 的记录原样做了一份静态夹具**（`mockServer/reports.ts`）——
   * 报告页很少有人看，先原原本本搬进预览里，以后再改。
   */
  async getReportList() {
    return reportSummaries()
  },

  async getReportDetail(id) {
    return reportDetail(id)
  },

  async getReportExportPlan(id) {
    return reportExportPlan(id)
  },

  /**
   * 演示导出：假后端没有文件系统 —— 把"会落到哪几个路径"原样报回来（页面据此说一句
   * 「已导出」）。真机上这一步是 [`crate::ipc::report::export_report_archive`] 的真复制，
   * 拒绝规则（不覆盖 / 不许指到归档自己）也都在那一侧。
   */
  async exportReportArchive(id, dir) {
    return reportExportPaths(id, dir)
  },

  /**
   * 「打开位置」（导出成功之后那一颗）。与 [`revealInFolder`] 同一条：假后端没有
   * 文件系统，**照实说它在真机上的样子**，不假装打开了（假装的话浏览器里看着像成了）。
   */
  async revealReportExport(_id, dir) {
    throw new Error(
      `假后端没有文件系统 —— 真机上这一步会打开文件管理器，停在 ${dir}\\<基名>\\`,
    )
  },

  /**
   * 「更改位置」那一格。浏览器里没有 `tauri-plugin-dialog` 那套系统选择器：
   * Chromium 有 File System Access API 就真弹一个，没有就退回一个"演示位置" ——
   * 与 [`pickImportFiles`] 同一路数（假后端不装文件系统，只把流程走通）。
   */
  async pickExportDir(current) {
    const pick = (
      window as Window & { showDirectoryPicker?: () => Promise<{ name: string }> }
    ).showDirectoryPicker
    if (pick === undefined) return `${current}/换过的位置`
    try {
      const handle = await pick()
      return `${current.replace(/[\\/]+$/, '')}/${handle.name}`
    } catch {
      /* 用户在系统框里取消：保持原来那个位置（不是错误） */
      return null
    }
  },

  /**
   * 新数据世界的浏览器演示。真机上 Rust 读的是释放进数据根的 catalog.json
   * （真数据、真 SHA、真 definition）；浏览器里没有数据根，给一份同形状的最小演示 ——
   * 数字是编的，形状不编：页面按什么结构读，真机上就读得到。
   * definition（registry）用同一批快照摊，参数页的页签/分组树在两种模式下同源。
   */
  async getRuntimeCatalog() {
    return {
      catalogSchema: 1,
      revision: 'mock000000000000',
      /*
       * 发布时刻：真机上由发布侧写进目录（云端表的「时间」列显示它）。
       * 演示给一个固定值 —— 形状不编，数字编。
       */
      publishedAt: '2026-10-05T08:00:00Z',
      machines: [
        {
          id: 'A1',
          display: 'A1',
          brand: '拓竹 (Bambu Lab)',
          versions: [
            { id: 'STANDARD', name: '标准版' },
            { id: 'FAST', name: '快拆版6月以前' },
          ],
        },
        {
          id: 'A1_MINI',
          display: 'A1 mini',
          brand: '拓竹 (Bambu Lab)',
          versions: [{ id: 'STANDARD', name: '标准版' }],
        },
      ],
      /* 目录登记的**唯一一份**演示数据：另一支消费者是「这个 combo 的套餐」（见该文件头） */
      files: mockCatalogFiles(),
      /*
       * 资产登记（真机那份来自 `presets/assets.toml`）。**图片这一档用真 id + 真 path**：
       * 首页大图按 id 查 path 再拼 `/assets/<path>`（2026-10-03 第二刀），
       * 而那几个文件在构建期真被装配进 dist（`tools/assets/plugin.mjs`）——
       * 所以浏览器演示里的大图是**真取到了**，不是画个占位。
       * 其余几档（MKP 产物 / 切片器配置）只登记、不在这里给文件。
       */
      assets: [
        /* 品牌字标（2026-10-03 品牌图正式进资产体系）：**不写 machineId** = 公共素材 */
        {
          id: 'bambu-lab-logo',
          type: 'image',
          name: 'Bambu Lab 字标',
          path: 'brands/bambu-lab-logo.svg',
          delivery: 'bundled',
        },
        {
          id: 'a1-image',
          type: 'image',
          machineId: 'A1',
          name: 'A1 外观图',
          path: 'printers/a1.webp',
          delivery: 'bundled',
        },
        {
          id: 'a1_mini-image',
          type: 'image',
          machineId: 'A1_MINI',
          name: 'A1 mini 外观图',
          path: 'printers/a1mini.webp',
          delivery: 'bundled',
        },
        {
          id: 'a1_mini-variant-image',
          type: 'image',
          machineId: 'A1_MINI',
          name: 'A1 mini 外观图（快拆版）',
          path: 'printers/a1mini-variant.webp',
          delivery: 'bundled',
        },
        {
          id: 'p1s-image',
          type: 'image',
          machineId: 'P1S',
          name: 'P1S 外观图',
          path: 'printers/p1s.webp',
          delivery: 'bundled',
        },
        /*
         * 引导图（2026-10-16）：与上面那几张同一条路 —— **真 id + 真 path**，
         * 文件也被装配进 dist（`delivery = 'bundled'`），所以浏览器演示里
         * 「去系统里改…」那张图片引导是**真取到图**的，不是画个占位。
         */
        {
          id: 'guide-open-with-light',
          type: 'guide',
          name: '「改 .3mf 默认打开方式」引导图（浅色）',
          /* ★ 2026-10-16 方案 A：交付的是**成品图**（框已烧进像素），原图留在仓库
             `guides/source/` 下自己留存（客户端只读这一条） */
          path: 'guides/open-with-light.rendered.webp',
          delivery: 'bundled',
        },
        {
          id: 'guide-open-with-dark',
          type: 'guide',
          name: '「改 .3mf 默认打开方式」引导图（深色）',
          path: 'guides/open-with-dark.rendered.webp',
          delivery: 'bundled',
        },
      ],
      registry: catalogRegistry(),
      /* 品牌（含品牌图的资产 id）—— 客户端在机型与版本都没图时回落到它 */
      brands: [{ id: 'Bambu Lab', name: '拓竹 (Bambu Lab)', logo: 'bambu-lab-logo' }],
      /*
       * **分组登记**（2026-10-16）—— 与 `presets/assets.toml` 的 `[[groups]]` 同形。
       * 引导图靠它挑"哪张是浅色那张"（从前写死两个资产 id）。
       * ★ 夹具要跟真台账同形：漏了组 ⇒ 那一处引导直接不出图（缺图一路往下落）。
       */
      groups: [
        {
          id: 'open-with',
          name: '「改 .3mf 默认打开方式」引导图',
          members: [
            { asset: 'guide-open-with-light', role: 'light' },
            { asset: 'guide-open-with-dark', role: 'dark' },
          ],
        },
      ],
      plates: allPlates(),
    }
  },

  /*
   * ★★ 2026-10-14：下载不再是"如实拒" —— **逻辑要真**（作者：「数据假的，
   * 但是它那个逻辑那些要真的呀」）。浏览器里走**同一套状态机**的内存版：
   * `mockDeliver` 发水位、落下载区、洗掉 stale；删除把那份拿走。
   * 界面上「下载 / 更新 / 删除」从此在浏览器里也是**真的会变**的。
   */
  async downloadCatalogFile(fileName, onTick) {
    await mockDeliver(fileName, onTick)
  },

  async deleteDeliveryFile(fileName) {
    const before = mockDownloaded.length
    mockDownloaded = mockDownloaded.filter((f) => f.fileName !== fileName)
    if (mockDownloaded.length === before) {
      throw new Error(`下载区里没有 ${fileName} —— 它可能已经被删掉了`)
    }
    mockDeliveredText.delete(fileName)
  },

  /** 归档区同样是内存态（`mockArchived`，见 `getArchivedFiles`）：删掉对应那条 */
  async deleteArchivedFile(path) {
    const before = mockArchived.length
    mockArchived = mockArchived.filter((a) => a.path !== path)
    if (mockArchived.length === before) {
      throw new Error(`归档区里没有 ${path} —— 它可能已经被删掉了`)
    }
  },

  async downloadCatalogFiles(fileNames, onTick) {
    /* 逐份走同一条 `mockDeliver`；**一份失败不拖累别人**（与真机同口径），按份给结局 */
    const outcomes: DownloadOutcome[] = []
    for (const fileName of fileNames) {
      try {
        await mockDeliver(fileName, onTick)
        outcomes.push({ fileName, ok: true, message: '' })
      } catch (e) {
        outcomes.push({ fileName, ok: false, message: String((e as Error)?.message ?? e) })
      }
    }
    return outcomes
  },

  /**
   * 浏览器里没有下载区，也就没有"已经下载的文件"可读 —— **一件一件照实说**：
   *
   * - **切片器配置那几份**（`bbs_config`）：有演示正文（`mockServer/slicerPreset.ts`）——
   *   工艺页左侧默认停在「MKP 工艺」那一档，那一档不给正文就是个空壳；
   * - 别的（MKP 的 toml 那些）：如实拒，不返回空串充数 —— 那是"云端原件"，
   *   造一份等于编造"云端发过这份字节"。
   */
  async readDownloadedText(fileName) {
    const demo = slicerPresetText(fileName)
    if (demo !== null) return demo
    throw new NotImplementedError(
      'readDownloadedText',
      '浏览器预览里没有下载区，读不到这份的正文 —— 用桌面版就能看',
    )
  },

  /*
   * 数据源（设置页那一格）：真机写 `run/preset-source.json`，浏览器里没有盘 ——
   * 这一档走**内存镜像**（与用户目录 / 使用中指针同一套口径：能走通的就真走，走不通的如实说）。
   * 演示口径：这个假后端**没有内置默认源**（真机的两个内置是构建期注进来的），
   * 所以「使用内置云端源」在这里 = 回到"没配"。
   */
  async getPresetSource() {
    return mockSource
  },

  async setPresetSource(mode, customUrl) {
    /* 校验与真机 `runtime::source::normalize_base_url` 同一套（连消息也照抄）：
       只认 http(s)、砍尾斜杠、空地址拒绝 */
    if (mode === 'custom') {
      const url = (customUrl ?? '').trim().replace(/\/+$/, '')
      if (url === '') throw new Error('数据源地址是空的')
      if (!url.startsWith('http://') && !url.startsWith('https://')) {
        throw new Error(`数据源地址只认 http:// 或 https://，填进来的是 ${url}`)
      }
      mockSource = {
        mode: 'custom',
        label: '自定义地址',
        address: url,
        fromUser: true,
        builtin: [],
        defaultMode: 'github',
        builtinDefault: null,
      }
      return mockSource
    }
    if (mode !== 'github' && mode !== 'gitee') {
      throw new Error(`认不出的源：${mode}（只能是 github / gitee / custom）`)
    }
    // 演示后端没有内置源：选了内置 = 回到"没配"（与上面那段注释同一口径）
    mockSource = null
    return mockSource as unknown as PresetSource
  },

  async clearPresetSource() {
    mockSource = null
    return null
  },

  /*
   * 主题（设置页「外观」那一格）：与数据源同一套口径 —— 内存镜像。
   * 认得出三个字面量才收（消息照抄真机那两条命令的说法），别的如实拒绝。
   */
  async getTheme() {
    return mockTheme
  },

  async setTheme(theme) {
    if (theme !== 'light' && theme !== 'dark' && theme !== 'system') {
      throw new Error(`认不出的主题：${theme}（只能是 light / dark / system）`)
    }
    mockTheme = theme
    return mockTheme
  },

  /*
   * 自动清理（2026-10-13）：与主题同一套口径 —— 内存镜像、回解析后的 boolean。
   * 演示后端不真清盘（浏览器里没有那棵 `gcode_history`），只把这一档记下来。
   */
  async getAutoCleanHistory() {
    return mockAutoClean
  },

  async setAutoCleanHistory(enabled) {
    mockAutoClean = enabled
    return mockAutoClean
  },

  /*
   * 主窗怎么摆（2026-10-17）：与主题同一套口径 —— 内存镜像、认档才收。
   * 窗口行为空转：浏览器里那扇窗不是我们能摆的，"置顶 / 窄窗"只在真机上有体感。
   */
  async getWindowPrefs() {
    return { pin: mockPin, compact: mockCompact }
  },

  /* 第二个参数（`onBbs`）在演示后端没有读者：浏览器里本来就没有"置顶"这件事 */
  async setPin(pin: PinPref) {
    if (pin !== 'off' && pin !== 'page' && pin !== 'always') {
      throw new Error(`认不出的置顶档：${pin}（只能是 off / page / always）`)
    }
    mockPin = pin
    return mockPin
  },

  async applyPin() {
    return mockPin === 'always'
  },

  /* 与真后端同一套口径（2026-10-17 作者第二轮）：**小窗一开就置顶，还原时把置顶放回去** */
  async setCompact(compact: boolean) {
    if (compact === mockCompact) return mockCompact
    if (compact) {
      mockPinBeforeCompact = mockPin
      mockPin = 'always'
    } else if (mockPin === 'always' && mockPinBeforeCompact !== null) {
      mockPin = mockPinBeforeCompact
      mockPinBeforeCompact = null
    }
    mockCompact = compact
    return mockCompact
  },

  /* —— baseline + 对比台（2026-10-08）：固定演示版本账 + 从演示正文里读写参数 ——
     真机上第一条读目录 + 版本链、第二条读隐藏 baseline、后两条读写用户那份文件本身。 */
  async getCloudVersions(fileName?: string | null) {
    const want = fileName?.trim()
    return MOCK_PRESET_VERSIONS.filter((v) => !want || v.fileName === want).map((v) => ({ ...v }))
  },

  async readPresetParams(path: string): Promise<PresetParamValues> {
    const text = mockLocalText.get(path)
    const fileName = path.split('/').pop() ?? path
    if (text === undefined) {
      return { path, fileName, values: {}, problem: `找不到 ${path}` }
    }
    return { path, fileName, values: readMockParams(text), problem: null }
  },

  async savePresetParams(path: string, edits: ParamEdit[]) {
    const text = mockLocalText.get(path)
    if (text === undefined) throw new Error(`找不到 ${path} —— 没有动别的地方`)
    let next = text
    for (const e of edits) {
      const def = rawParamDefs().find((d) => d.key === e.paramKey)
      if (def === undefined) throw new Error(`不认识这个参数：${e.paramKey}`)
      next = patchMockParam(next, def, e.value)
    }
    if (next === text) return
    mockLocalText.set(path, next)
    const at = mockLocal.findIndex((f) => f.path === path)
    if (at >= 0) mockLocal[at] = { ...mockLocal[at]!, size: next.length, modifiedUnix: nowSec() }
  },

  /* —— 逐参数「云端更新」（2026-10-09）：内存夹具里的三方账 + 采用·保持 ——
     真机上第一条读我那份文件 + 隐藏 baseline + 目录里那一版；第二条结构保真地写那几项。
     `fetchMissing` 在假后端里没有对应的动作（没有网络），形状照留。 */
  async getPresetParamSync(path: string, fetchMissing?: boolean) {
    /* 假后端没有网络可发：`fetchMissing` 在这里没有对应的动作，形状照留（真机上有） */
    void fetchMissing
    return mockSyncOf(path)
  },

  async applyPresetParamDecisions(path: string, decisions: ParamDecision[]) {
    const text = mockLocalText.get(path)
    if (text === undefined) throw new Error(`找不到 ${path}`)
    const before = mockSyncOf(path)
    const current =
      before.cloudFileName === null ? undefined : mockParamCurrent.get(before.cloudFileName)
    if (current === undefined) {
      throw new Error('云端当前版的正文还不在本机 —— 先把云端新版取回来，再处理这一项')
    }
    let next = text
    const book = mockParamDecisions.get(path) ?? new Map()
    for (const d of decisions) {
      if (d.kind === 'adopt') {
        const def = rawParamDefs().find((x) => x.key === d.paramKey)
        if (def === undefined) throw new Error(`不认识这个参数：${d.paramKey}`)
        const value = before.entries.find((e) => e.key === d.paramKey)?.cloudNew
        if (value === null || value === undefined) {
          throw new Error(`云端当前版里没有 ${d.paramKey} 的新值 —— 采用不了`)
        }
        next = patchMockParam(next, def, value)
      }
      book.set(d.paramKey, { sha256: current.sha256, kind: d.kind })
    }
    if (next !== text) {
      mockLocalText.set(path, next)
      const at = mockLocal.findIndex((f) => f.path === path)
      if (at >= 0) mockLocal[at] = { ...mockLocal[at]!, size: next.length, modifiedUnix: nowSec() }
    }
    mockParamDecisions.set(path, book)
    return mockSyncOf(path)
  },

  /* 盘就是底账 —— 浏览器没有盘，这里给的是**内存下载区**（可变，见 `mockDownloaded`）：
     两份对得上目录、一份对不上。三个读合起来才够预设页画三态，这是其中两个 */
  async getDownloadedFiles() {
    return [...mockDownloaded]
  },

  async getStaleFiles() {
    return [...mockStale]
  },

  /*
   * 第 6 层：认得出是哪一版吗。演示集合与上面两条对齐 ——
   *   `A1-fast.toml` 盘上那份**就是归档里那一版**（下面 `getArchivedFiles` 给的那条），
   *                所以它是「旧版本」，证据是那条归档路径；
   *   `A1mini-standard.toml` 与目录、归档都对不上 → 「内容异常」。
   * 真机上这两档都是算出来的（见 `runtime::delivery::trust_entries`），不记账本。
   */
  async getDeliveryTrust() {
    return [
      { fileName: 'A1-fast.toml', verdict: 'old' as const, archivedPath: 'archive/delivery/mkp/presets/A1-fast.toml' },
      { fileName: 'A1mini-standard.toml', verdict: 'tampered' as const, archivedPath: null },
    ]
  },

  /*
   * 归档区：浏览器里没有盘，给一条**固定演示** —— 让"这一份有旧版本"那一格画得出来。
   * 真机上它是扫 `archive/` 得到的（换版本时被换下来的那一份）。
   * 认人那三格（机型 / 版本 / kind）跟着演示的那条走 —— 形状不编，数字编。
   * 两个时间各是各：`publishedAt` 是这一版云端发布时刻、`replacedUnix` 是被换下的时刻。
   */
  async getArchivedFiles() {
    return [...mockArchived]
  },

  /** 读归档里的正文要真的盘（浏览器里没有）：与 downloadCatalogFile 同一条口径，不假装 */
  async readArchivedText() {
    throw new NotImplementedError(
      'readArchivedText',
      '浏览器预览里没有归档区，读不到这份旧版本的正文 —— 用桌面版（跑过一次更新）就能看',
    )
  },

  /* 使用中指针（新数据世界的第一个用户状态）：浏览器里记在内存，刷新即还原 */
  async getActivePreset() {
    return mockActive
  },

  /*
   * 两条线一个入口，浏览器里**两条线的处境不一样**，照实分开：
   *
   *   用户线（`presets-local/…`）  假后端有一份**内存里的用户目录**（上面那两份演示），
   *                             所以这一档能真的走一遍：记指针、界面立刻变「已应用」
   *   云端线（`mkp/…`）           浏览器里没有下载区、也没有那份字节，**校验无从谈起** ——
   *                             与 downloadCatalogFile 同一条口径：如实拒，不假装成功
   */
  async applyActivePreset(fileName, origin, path) {
    if (origin !== 'local') {
      throw new NotImplementedError(
        'applyActivePreset',
        `浏览器预览里没有 ${fileName} 的字节，应用云端预设要用桌面版（先在桌面版里下载一份）`,
      )
    }
    /*
     * ★★ 2026-10-14：按**真实后端的三条闸**校验（`ipc/catalog.rs::apply_active_preset`
     * 的 `ActiveOrigin::Local` 那一支），不能再"存下来就完事" —— 假后端里能过、
     * 真机被拒，就会把"路不对"误判成"界面点不动"（作者 2026-10-14 那次正是如此）。
     *
     *   ① 必须给路径（后端 `path.ok_or_else`：用自己那份要给出它在用户根里的路径）
     *   ② 只认用户根那一格（后端 `check_local_prefix`：`presets-local/` 开头 + 不许穿越）
     *   ③ 必须是一份 MKP 预设（后端 `kind_of`：`.toml`）
     * 加上原来的"文件得在、内容得读得出来"。
     */
    if (path === undefined || path === '') {
      throw new Error('用自己那份要给出它在用户根里的路径（presets-local/…）')
    }
    /* ② 前缀闸：与后端同一句话的形状（`只读用户自己的预设（要 … 开头，给的是 …）`） */
    if (!path.startsWith(LOCAL_PREFIX)) {
      throw new Error(
        `只读用户自己的预设（要 ${LOCAL_PREFIX}… 开头，给的是 ${path}）`,
      )
    }
    if (path.slice(LOCAL_PREFIX.length).split('/').includes('..')) {
      throw new Error(`路径里不许有 .. —— 给的路径是 ${path}`)
    }
    /* ③ 类型闸（后端 `kind_of`：只认 `.toml`；大小写不敏感，但用户文件里只认预设） */
    if (!fileName.toLowerCase().endsWith('.toml')) {
      throw new Error(
        `${fileName} 不是一份 MKP 预设（TOML）—— 用户文件里只有预设能被使用`,
      )
    }
    const hit = mockLocal.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`用户目录里没有 ${path}`)
    /* 第九层：读不出来的那份**不许应用**（真机入口闸会拒；消息形状对齐后端） */
    if (hit.state === 'unreadable') {
      throw new Error(
        `${hit.path} 读不出来：${hit.stateDetail ?? 'TOML 语法不对'} —— 这一份现在不能应用、也不能改`,
      )
    }
    mockActive = {
      origin: 'local',
      fileName: hit.fileName,
      path: hit.path,
      /* 指纹是应用那一刻的字节摘要；浏览器里没有真字节，用路径占位（形状不编） */
      sha256: `mock:${hit.path}`,
      machineId: hit.basedOnMachineId ?? '',
      versionId: hit.basedOnVersionId ?? '',
      intact: true,
    }
    return mockActive
  },

  async clearActivePreset() {
    mockActive = null
  },

  /*
   * 云端表那两个动作（2026-10-09 改判）：下载 / 更新 = 取回交付预设 + 落一份我的工作副本。
   * **不改「当前使用」** —— 使用是本地表那颗按钮的事。
   *
   * 浏览器里**没有下载区**，所以照实分两档：
   *   演示集合里已有（`MOCK_DOWNLOADED`）→ 当成"本机已有当前版"，
   *                                        然后落 `presets-local/<原名>`（没有的话）；
   *   没有                              → 取不回来，如实抛（与 `downloadCatalogFile` 同一口径）。
   *
   * 与真机同一个形状：用户只面对 `presets-local/` 里那一份，云端原件留在内部。
   */
  async fetchCloudPreset(fileName, renameIfTaken = false) {
    const hit = mockPresetFile(fileName)
    if (hit === undefined) throw new Error(`目录里没有 ${fileName} 这一份`)
    /*
     * ★★ 2026-10-14：原来这里有个闸 —— 演示集合里没有的那份**直接拒**。
     * 现在**逻辑要真**：目录里登记的任何一份都能取回（内存里模拟一次交付，
     * 与 `downloadCatalogFile` 同一条 `mockDeliver`，只是不给水位 —— 真机这条路也没水位）。
     */
    await mockDeliver(fileName)
    const path = `presets-local/${fileName}`
    const existing = mockLocal.find((f) => f.path === path)
    const created = existing === undefined
    if (created) {
      const label = `delivery/mkp/presets/${fileName}`
      const text = [
        `# machine: ${hit.machineId}`,
        `# variant: ${hit.versionId.toLowerCase()}`,
        `# based_on: ${label}`,
        `# based_on_release_time: 2026-10-06 03:34:17`,
        `# based_on_sha256: ${'0'.repeat(64)}`,
        MOCK_PRESET_TEXT,
      ].join('\n')
      mockLocalText.set(path, text)
      mockLocal.push({
        path,
        fileName,
        size: text.length,
        modifiedUnix: nowSec(),
        kind: 'mkp_preset',
        state: 'ok',
        stateDetail: null,
        basedOn: 'current',
        basedOnLabel: label,
        basedOnRelease: '2026-10-06 03:34:17',
        basedOnMachineId: hit.machineId,
        basedOnVersionId: hit.versionId,
        machineId: hit.machineId,
        versionId: hit.versionId,
        /* 它是从云端原件落下来的：出处账记着从哪来（界面上「来源」那一格） */
        copiedFrom: label,
        copiedFromName: fileName,
        provenance: 'copy' as const,
      })
    }
    /*
     * 撞名（2026-10-10）：本机已经有一份同名的 ——
     * 默认不动它（把"谁占了 + 会改成什么"交给界面问）；用户同意后的那次调用按
     * 没被占的名字再落一份新的，原来那份一个字节不动。
     */
    if (created) return { fetched: false, created, fileName, path }
    if (renameIfTaken) {
      const name = mockUntakenName(fileName)
      const newPath = `presets-local/${name}`
      const label = `delivery/mkp/presets/${fileName}`
      const text = [
        `# machine: ${hit.machineId}`,
        `# variant: ${hit.versionId.toLowerCase()}`,
        `# based_on: ${label}`,
        `# based_on_release_time: 2026-10-06 03:34:17`,
        `# based_on_sha256: ${'0'.repeat(64)}`,
        MOCK_PRESET_TEXT,
      ].join('\n')
      mockLocalText.set(newPath, text)
      mockLocal.push({
        path: newPath,
        fileName: name,
        size: text.length,
        modifiedUnix: nowSec(),
        kind: 'mkp_preset',
        state: 'ok',
        stateDetail: null,
        basedOn: 'current',
        basedOnLabel: label,
        basedOnRelease: '2026-10-06 03:34:17',
        basedOnMachineId: hit.machineId,
        basedOnVersionId: hit.versionId,
        machineId: hit.machineId,
        versionId: hit.versionId,
        copiedFrom: label,
        copiedFromName: fileName,
        provenance: 'copy' as const,
      })
      return { fetched: false, created: true, fileName: name, path: newPath }
    }
    return {
      fetched: false,
      created,
      fileName,
      path,
      takenBy: path,
      suggestedName: mockUntakenName(fileName),
    }
  },

  /* 浏览器里没有远端（真远端 = 工作台发布的 dist，或将来的云端）：如实说没有 */
  async checkRemoteUpdate() {
    throw new NotImplementedError(
      'checkRemoteUpdate',
      '浏览器预览里没有远端目录 —— 目录更新是桌面版自动做的事，这里不用管',
    )
  },

  async applyRemoteUpdate() {
    throw new NotImplementedError(
      'applyRemoteUpdate',
      '浏览器预览里没有远端目录 —— 目录更新是桌面版自动做的事，这里不用管',
    )
  },

  /*
   * 软件更新（release.json）：与预设数据是**两条链**。
   * 浏览器里演示"当前版本"与一个固定的假发布状态 —— 形状不编（字段与真后端同形），
   * 值是演示值。真机上这两个口子住 Rust 侧（`ipc::catalog`）。
   */
  async getAppVersion() {
    return MOCK_APP_VERSION
  },

  /*
   * 屏幕引导（2026-10-15）：**浏览器里没有这一档** —— 它要读切片器窗口的坐标，
   * 那是 macOS Accessibility 的活，浏览器里既没有切片器也没有那个 API。
   * 照实回"指不了" + 一句人话（界面因此显示原因，而不是画一个指错地方的箭头）。
   * 形状与真后端同形（`ready` + `reason`），值不编。
   */
  async locateBedTypeControl(): Promise<GuideTarget> {
    return {
      ready: false,
      reason: '开发台里没有屏幕引导：它要读切片器窗口的坐标（真机上才有）',
    }
  },

  /* 同上：浏览器里没有切片器窗口可指，照实回"指不了"，界面因此只显示原因 */
  async guideShow(): Promise<GuideTarget> {
    return this.locateBedTypeControl()
  },

  async guideClose() {
    /* 没开过也是成功 */
  },

  async checkSoftwareUpdate(): Promise<SoftwareUpdate> {
    /* 演示：比当前版本更新一档 —— 设置页因此显示「有新版本 SupportEase」。
       **想触发"有新版本"那套界面不用真发一版**：装 mock（`?mock=1`）就是这个形状。
       想演示「已是最新版本」，把 MOCK_NEW_VERSION 设成 MOCK_APP_VERSION */
    return {
      hasUpdate: MOCK_NEW_VERSION !== MOCK_APP_VERSION,
      currentVersion: MOCK_APP_VERSION,
      latestVersion: MOCK_NEW_VERSION,
      notes: '演示：更新检查是独立的一条链（release.json），与预设数据无关。',
      url: 'https://example.com/supportease/releases',
      /* 演示带安装包 ⇒ 界面会摆「在应用内下载」；删掉这一格就退回"打开下载页" */
      asset: {
        name: 'SupportEase_demo.app.zip',
        url: 'https://example.com/supportease/releases/download/demo/SupportEase_demo.app.zip',
        size: 7_260_510,
        sha256: '',
      },
    }
  },

  /* ---------- 应用内更新（第五刀）：内存里的假下载 ----------
     ★ 目的是**不用真发一版就能验界面**：标题栏的环、面板的暂停/继续/取消、
       完成后的「重启并安装」都在浏览器里点得到。真机上这些走 `ipc/update.rs`。 */

  async updateInfo(): Promise<UpdateInfo> {
    return {
      state: mockUpdate.state,
      hasUpdate: MOCK_NEW_VERSION !== MOCK_APP_VERSION,
      currentVersion: MOCK_APP_VERSION,
      latestVersion: MOCK_NEW_VERSION,
      notes: '演示：应用内更新的进度、暂停与重启安装。',
      url: 'https://example.com/supportease/releases',
      asset: {
        name: 'SupportEase_demo.app.zip',
        url: 'https://example.com/supportease/releases/download/demo/SupportEase_demo.app.zip',
        size: 7_260_510,
        sha256: '',
      },
      lastResult: mockUpdate.lastResult,
    }
  },

  async startUpdate() {
    if (mockUpdate.state.state === 'downloading' || mockUpdate.state.state === 'paused') return
    const total = 7_260_510
    let received = 0
    mockUpdate.timer = setInterval(() => {
      if (mockUpdate.state.state === 'paused') return
      received = Math.min(total, received + Math.round(total / 24))
      if (received >= total) {
        mockUpdate.stopTimer()
        mockUpdate.state = { state: 'ready', path: '/tmp/SupportEase.app', size: total }
        mockUpdate.lastResult = undefined
        return
      }
      mockUpdate.state = { state: 'downloading', received, total }
    }, 180)
  },

  async pauseUpdate() {
    if (mockUpdate.state.state !== 'downloading') return
    const { received, total } = mockUpdate.state
    mockUpdate.state = { state: 'paused', received, total }
  },

  async resumeUpdate() {
    if (mockUpdate.state.state !== 'paused') return
    const { received, total } = mockUpdate.state
    mockUpdate.state = { state: 'downloading', received, total }
  },

  async cancelUpdate() {
    mockUpdate.stopTimer()
    mockUpdate.state = { state: 'cancelled' }
  },

  async installUpdate() {
    // 真机上这一步会**退出进程**；演示里只把账记上（下次问 updateInfo 能看到）
    mockUpdate.lastResult = {
      version: MOCK_NEW_VERSION,
      ok: true,
      reason: '',
      at: new Date().toISOString(),
    }
    mockUpdate.state = { state: 'idle' }
  },

  async openUrl(url: string) {
    // 演示里不真的开浏览器；真机走 opener 插件
    mockUpdate.lastOpened = url
  },
}

/** 演示用的更新会话（内存态，与其它 mock 同一套口径） */
const mockUpdate = {
  state: { state: 'idle' } as UpdateState,
  lastResult: undefined as UpdateResult | undefined,
  lastOpened: undefined as string | undefined,
  timer: undefined as ReturnType<typeof setInterval> | undefined,
  stopTimer() {
    if (this.timer !== undefined) {
      clearInterval(this.timer)
      this.timer = undefined
    }
  },
}

/** mock 版的"当前版本"：与演示的"最新版本"配合出一个好看的对照 */
const MOCK_APP_VERSION: string = '0.0.1'

/**
 * 演示用的"最新版本"。★ **改成与 `MOCK_APP_VERSION` 相同就切到「已是最新版本」**
 * —— 想验哪一态改这一处，不用真发一版。
 */
const MOCK_NEW_VERSION: string = '0.0.2'
