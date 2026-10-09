import { NotImplementedError } from './errors'
import type {
  ActiveOrigin,
  ActivePreset,
  Axes,
  CalibModel,
  MkpApi,
  OfficialVersion,
  OnDiskFile,
  ParamDecision,
  ParamDecisionKind,
  ParamEdit,
  ParamSyncEntry,
  PresetParamSync,
  PresetParamValues,
  PresetSource,
  SoftwareUpdate,
  UserPresetFile,
  UpdateInfo,
  UpdateResult,
  UpdateState,
} from './contract'
import {
  allMachines,
  allPlates,
  allPresetFiles,
  catalogRegistry,
  copyToSlicerIn,
  localFileIds,
  menuEntries,
  mockCatalogFiles,
  paramMeta,
  resolveParams,
  resolveVersionFiles,
  slicerCopied,
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

/** 浏览器演示用的使用中指针（内存态，刷新即还原；真数据在 Rust 侧 run/ 状态文件里） */
let mockActive: ActivePreset | null = null

/** 浏览器演示用的数据源覆盖（内存态，刷新即还原；真数据在 Rust 侧 run/preset-source.json） */
let mockSource: PresetSource | null = null

/*
 * 浏览器里的「用户目录」：**内存态**（刷新还原）—— 与 `mockActive` 同一套做法。
 *
 * 为什么让它在内存里真的能走通：编辑那条链（改 → 临时文件 → 另存）是这一层最需要被看见的
 * 东西；只抛"未实现"的话，浏览器里连编辑器长什么样、保存之后表格怎么变都验不了。
 * 真机上它是 `<appDataDir>/user/presets-mine/`，盘就是底账。
 */
const mockMine: UserPresetFile[] = [
  {
    /*
     * 带血统（基于旧版官方）。**2026-10-08 起界面上不再有「基于旧版官方」那一枚** ——
     * 用户那份不是"过时文件"，它是一份完整、可继续使用的预设；血统只在展开详情里
     * 答"从哪一版派生"。
     */
    path: 'presets-mine/我的 A1 涂胶.toml',
    fileName: '我的 A1 涂胶.toml',
    size: 2048,
    modifiedUnix: 1780000000,
    kind: 'mkp_preset',
    /* 第九层：能读 + TOML 语法过（演示正文是合法 TOML）→ 照常有「应用 / 改这份」 */
    state: 'ok',
    stateDetail: null,
    basedOn: 'outdated',
    basedOnLabel: 'dist/mkp/presets/A1-fast.toml',
    basedOnRelease: '2026-05-29 04:26:12',
    basedOnMachineId: 'A1',
    basedOnVersionId: 'FAST',
    /* 归属（文件头 # machine/# variant 两行）：这份演示正文里也种了这两行 */
    machineId: 'A1',
    versionId: 'FAST',
    /* 演示里没走过复制/导入 —— 出处照实没有（真机上由出处账答） */
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
  {
    /*
     * **第二份可读的「我的预设」**：对比台要能被看见就必须有两份能比的 ——
     * 只有一份时那个模态框只会说「至少选两份才能对比」。
     * 它对应的就是"官方发了 2026-10-15 那一版之后，下载生成的新的一份"。
     */
    path: 'presets-mine/我的 A1 涂胶-2026-10-15.toml',
    fileName: '我的 A1 涂胶-2026-10-15.toml',
    size: 2048,
    modifiedUnix: 1780900000,
    kind: 'mkp_preset',
    state: 'ok',
    stateDetail: null,
    basedOn: 'current',
    basedOnLabel: 'mkp/presets/A1-fast.toml',
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
    /* 没有血统（手工拷的 / 别的程序写出来的）—— 照实说不出新旧，这是合法状态 */
    path: 'presets-mine/Process_0.2mm.json',
    fileName: 'Process_0.2mm.json',
    size: 1024,
    modifiedUnix: 1780003600,
    kind: null,
    /* 认不出是哪一类 → 没有"能不能当预设用"这一档（与真机同形状） */
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
    path: 'presets-mine/坏了的涂胶.toml',
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
const mockMineText = new Map<string, string>()

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
 * 用户文件新名字的门槛（与 Rust 侧 `mine::check_new_name` 同一套）：
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
/** 编辑中的那一份（真机上是 `run/draft-preset.json`）。`path` 只有用户线才有 */
let mockDraft: {
  origin: ActiveOrigin
  sourceFileName: string
  path: string | null
  text: string
  updatedUnix: number
} | null = null

/** 演示正文。真机上它是官方原件（`mkp/…`）的字节 —— 假后端没有文件系统，只能给一段 */
const MOCK_OFFICIAL_TEXT =
  [
    '# 假后端的演示正文 —— 真机上这里是官方原件（mkp/…）的字节',
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
mockMineText.set(
  'presets-mine/我的 A1 涂胶.toml',
  [
    '# 我自己的这一份（假后端演示正文）',
    '# machine: A1',
    '# variant: fast',
    '# based_on: dist/mkp/presets/A1-fast.toml',
    '# based_on_release_time: 2026-05-29 04:26:12',
    `# based_on_sha256: ${'0'.repeat(64)}`,
    /* 键用**注册表里的真名字**（`[toolhead]` + `offset_y`）：对比台按注册表认参数，
       编一套假键的话那一列会全是"—"，等于把要验的东西绕开了 */
    '[toolhead]',
    'offset_y = 20.0 # 笔尖偏移',
    'speed_limit = 60 # 涂胶速度限制 (mm/s)',
    '',
  ].join('\n'),
)
mockMineText.set(
  'presets-mine/我的 A1 涂胶-2026-10-15.toml',
  [
    '# 我自己的这一份 · 基于 2026-10-15 那一版官方（假后端演示正文）',
    '# machine: A1',
    '# variant: fast',
    '# based_on: mkp/presets/A1-fast.toml',
    '# based_on_release_time: 2026-10-15 09:00:00',
    `# based_on_sha256: ${'1'.repeat(64)}`,
    '[toolhead]',
    'offset_y = 26.8 # 笔尖偏移',
    'speed_limit = 70 # 涂胶速度限制 (mm/s)',
    '',
  ].join('\n'),
)
/* 坏了那份也种一份正文：**看正文照旧读得出来**（用户要能看着它去修）——
   只有「应用 / 改这份」被拦（真机上是后端的文件级检查拦的） */
mockMineText.set(
  'presets-mine/坏了的涂胶.toml',
  ['# 坏了的演示（假后端演示正文）', '[toolhead]', 'offset_x = (1', ''].join('\n'),
)

/* ---------- 逐参数「官方更新」（2026-10-09）----------
 *
 * 真机上这件事由三份东西凑出来：
 *
 *   我那份文件      `presets-mine/…`（盘上，用户唯一可见的那一份）
 *   官方旧值        隐藏 baseline（`<appDataDir>/baseline/<sha>.toml`，按内容摘要寻址）
 *   官方当前版      目录里那一版的字节（`<appDataDir>/<catalog.path>`）
 *
 * 假后端没有文件系统，用**同一形状的内存夹具**顶上（不是另编一套形状）：
 *
 *   基准区   `mockParamBaseline`：sha → 那一版官方正文
 *   当前版   `mockParamCurrent`：文件名 → 官方当前版（sha + 正文）
 *   决定账   `mockParamDecisions`：路径 → 参数 key → { sha, kind }（真机 run/app-state.json）
 *
 * 「官方当前版」那段正文比基准那版**真改了两个值**（`speed_limit` 70→65、
 * `offset_y` 18.6→19.0）—— 于是演示那份 `我的 A1 涂胶.toml` 真有两条待处理。
 */

/** 与演示正文里那两行 `# based_on_sha256` 对上（`我的 A1 涂胶.toml` / `…-2026-10-15.toml`） */
const SHA_BASED_OLD = '0'.repeat(64)
const SHA_BASED_NEW = '1'.repeat(64)

/** 官方**当前版**的演示正文（与 `MOCK_OFFICIAL_TEXT` 差两项） */
const MOCK_OFFICIAL_NEXT = [
  '# 假后端的演示正文 —— 官方当前版（比基准那版改了两个值）',
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
  [SHA_BASED_OLD, MOCK_OFFICIAL_TEXT],
  [SHA_BASED_NEW, MOCK_OFFICIAL_NEXT],
])

const mockParamCurrent = new Map<string, { sha256: string; text: string }>([
  /* sha 与 `MOCK_OFFICIAL_VERSIONS` 里 A1-fast 的「当前版」那一条是同一个值 */
  ['A1-fast.toml', { sha256: 'f2'.padEnd(64, '0'), text: MOCK_OFFICIAL_NEXT }],
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

/** 一份用户预设现在的官方更新账（真机：`ipc::param_sync::sync_of`） */
function mockSyncOf(path: string): PresetParamSync {
  const text = mockMineText.get(path)
  if (text === undefined) throw new Error(`找不到 ${path}`)
  const fileName = path.split('/').pop() ?? path
  const basedOn = mockHeaderOf(text, 'based_on')
  const officialFileName = basedOn === null ? null : (basedOn.split('/').pop() ?? null)
  const current = officialFileName === null ? undefined : mockParamCurrent.get(officialFileName)
  const basedSha = mockHeaderOf(text, 'based_on_sha256')
  const mine = readMockParams(text)
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
    entries.push({
      key: d.key,
      mine: mine[d.key] ?? null,
      baselineOld: old ?? null,
      officialNew: neu ?? null,
      pending,
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
    officialFileName,
    basedOnReleaseTime: mockHeaderOf(text, 'based_on_release_time'),
    currentReleaseTime: current === undefined ? null : mockHeaderOf(current.text, 'release_time'),
    officialReady: current !== undefined,
    versionAdvanced: basedSha !== null && current !== undefined && basedSha !== current.sha256,
    pendingCount,
    entries,
  }
}

const nowSec = () => Math.floor(Date.now() / 1000)

/**
 * 演示口径的官方交付集合与它们的可信档（与 `getDownloadedFiles` / `getDeliveryTrust` /
 * `getStaleFiles` 同一套演示账）。下载即得工作副本那条链要按它判"这份能不能到你手里"。
 */
const MOCK_RELEASE_FILES = [
  { fileName: 'A1-standard.toml', machineId: 'A1', versionId: 'STANDARD', trust: 'ok' },
  { fileName: 'A1-fast.toml', machineId: 'A1', versionId: 'FAST', trust: 'old' },
  {
    fileName: 'A1mini-standard.toml',
    machineId: 'A1_MINI',
    versionId: 'STANDARD',
    trust: 'tampered',
  },
] as const

const mockReleaseFile = (fileName: string) =>
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

/** 从演示正文里读一个数字（真机上是 `mine::calibration_of` 按注册表读） */
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
 * 真机上 baseline 是**隐藏的内部存储**（按内容摘要寻址）、官方版本来自目录 + 版本链，
 * 假后端两样都没有 —— 用一份**固定的演示版本账**顶上，形状与真后端一致：
 * 每个演示预设给两版（旧的一版算「已下载」、新的一版算「新版本」）。
 *
 * 它**不是"过时判定"**：这里不读用户预设的字节、也不给任何一行挂"该更新了"。
 */
const MOCK_OFFICIAL_VERSIONS: OfficialVersion[] = [
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
 * 再塞一份同版本的 MKP 条目就是**编形状**了。「未下载」那一档在浏览器里由官方行的「下载」
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
const MOCK_DOWNLOADED: OnDiskFile[] = [
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
const MOCK_STALE: OnDiskFile[] = [
  { fileName: 'A1-fast.toml', downloadedUnix: 1777000000, replacedUnix: null, publishedAt: null },
  {
    fileName: 'A1mini-standard.toml',
    downloadedUnix: null,
    replacedUnix: 1777000000,
    publishedAt: null,
  },
]

export const mockApi: MkpApi = {
  /*
   * 校准值写进「我的那一份」（2026-10-08 作者改判：不再是独立的 offsets.json）。
   * 落点带 path（`presets-mine/…`），所以在浏览器里可以真的改那份演示正文 ——
   * 校准页保存后重进，`getUserCopyFor` 读到的就是刚写的值（同一份内存文本）。
   * 真机上是 `mine::save_preset_calibration`（按注册表定位、注释一个字节不动）。
   */
  async savePresetCalibration(path, axes) {
    const text = mockMineText.get(path)
    if (text === undefined) {
      throw new Error(`${path} 读不出来 —— 先把这份另存为我的预设，再保存校准`)
    }
    mockMineText.set(path, patchMockOffsets(text, axes))
    const at = mockMine.findIndex((f) => f.path === path)
    if (at >= 0) mockMine[at] = { ...mockMine[at], modifiedUnix: nowSec() }
  },

  async getCalibModels() {
    return calibModels
  },

  async openModel(modelId) {
    console.info('[mock] openModel', modelId)
  },

  /*
   * 「复制后处理脚本」里那段可执行物路径 —— 浏览器里没有本机可执行物，如实答 `null`
   * （真机那份是壳的 `current_exe()`：就是本程序自己）。不编一串假路径出来：
   * 编出来的命令贴进切片器只会让人以为"复制成功了"，其实指不到东西。
   * 界面拿到 `null` 就不摆那颗按钮 —— 与"这份预设不在本机"同一个处置。
   */
  async getPostProcessExe() {
    return null
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
  async getUserPresetFiles() {
    return mockMine.map((f) => ({ ...f }))
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
  async setUserPresetMachineVersion(path, machineId, versionId) {
    const mid = machineId.trim()
    const vid = versionId.trim()
    if (mid === '' || vid === '') {
      throw new Error('机型与版本都要填 —— 归属写的是文件头的那两行，留空就认不出')
    }
    const hit = mockMine.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const known = allMachines().some((m) => m.id === mid)
    if (!known) {
      throw new Error(`${mid} 不是目录里登记的机型 —— 归属的机型要选一台真的机器`)
    }
    const text = mockMineText.get(path)
    if (text !== undefined) {
      mockMineText.set(path, rewriteMockMachineVariant(text, mid, vid))
    }
    hit.machineId = mid
    hit.versionId = vid
  },

  /** 只有这份会话另存出来的那份有正文可读（真机上每一份都能读） */
  async readUserPresetText(path) {
    const text = mockMineText.get(path)
    if (text === undefined) {
      throw new NotImplementedError(
        'readUserPresetText',
        '浏览器预览里读不到这一份的正文 —— 用桌面版就能看（每一份都读得到）',
      )
    }
    return text
  },

  /*
   * 临时编辑那条链：内存里真的走一遍（改的是临时文件，官方原件一动不动）。
   * 与真机同一个形状 —— 直道里的分岔只有一条：正文来自演示常量而不是 `mkp/` 里的字节。
   */
  async beginPresetEdit(fileName, origin = 'official', path) {
    /* 两条线的钥匙：官方线认文件名，用户线认路径（用户目录里同名很正常） */
    if (mockDraft !== null && mockDraft.origin === origin && mockDraft.sourceFileName === fileName) {
      if (origin === 'official' || mockDraft.path === (path ?? null)) {
        return { ...mockDraft, reused: true }
      }
    }
    if (origin === 'mine') {
      const rel = path ?? ''
      /* 第九层：读不出来的那份**不给改**（与真机同一个入口闸；消息形状也对齐后端） */
      const entry = mockMine.find((f) => f.path === rel)
      if (entry?.state === 'unreadable') {
        throw new Error(
          `${rel} 读不出来：${entry.stateDetail ?? 'TOML 语法不对'} —— 这一份现在不能应用、也不能改`,
        )
      }
      const raw = mockMineText.get(rel)
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
      text: MOCK_OFFICIAL_TEXT,
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

  /** 官方线：另存成 `presets-mine/<原名>（已修改）.toml`（与 Rust 侧 `mine::edited_name` 同一条规则） */
  async commitPresetDraft() {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份，没得存')

    /*
     * 用户线（第八层）：**写回它自己** —— 同一个路径、同一份文件，不产生第二份；
     * 血统**照抄原来那三行**（出处没变）。与 Rust 侧 `mine::save_back` 同一件事。
     */
    if (mockDraft.origin === 'mine') {
      const rel = mockDraft.path ?? ''
      const old = mockMineText.get(rel)
      if (old === undefined) {
        throw new NotImplementedError(
          'commitPresetDraft',
          `浏览器预览里读不到 ${rel} 的正文，写不回去 —— 用桌面版就能存`,
        )
      }
      const text = [...lineageLinesOf(old), withoutLineage(mockDraft.text)].join('\n')
      const fileName = rel.slice(rel.lastIndexOf('/') + 1)
      const at = mockMine.findIndex((f) => f.path === rel)
      if (at >= 0) mockMine[at] = { ...mockMine[at], size: text.length, modifiedUnix: nowSec() }
      mockMineText.set(rel, text)
      mockDraft = null
      return { path: rel, fileName, size: text.length, replaced: true }
    }

    const fileName = mockDraft.sourceFileName.replace(/\.toml$/i, '（已修改）.toml')
    const path = `presets-mine/${fileName}`
    /* 与真机同形：写下去的正文 = 草稿 + 文件头三行血统（真机上那三行由 `lineage::make_copy` 生成） */
    const label = `dist/mkp/presets/${mockDraft.sourceFileName}`
    const text = `# based_on: ${label}\n# based_on_sha256: ${'0'.repeat(64)}\n${mockDraft.text}`
    const size = text.length
    const replaced = mockMine.some((f) => f.path === path)
    const entry: UserPresetFile = {
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
      /* 官方另存出来的：出处走 based_on 血统（界面上显示"复制自官方 X"），账本不重复记 */
      copiedFrom: null,
      copiedFromName: null,
      provenance: null,
    }
    if (replaced) {
      mockMine[mockMine.findIndex((f) => f.path === path)] = entry
    } else {
      mockMine.push(entry)
    }
    mockMineText.set(path, text)
    mockDraft = null
    return { path, fileName, size, replaced }
  },

  /*
   * 第十层：用户文件管理（改名 / 删除）。与真机同一套规矩，在内存里走一遍 ——
   * 改名要看得见"使用中指针与草稿跟着走"，删除要看得见两道闸（正在使用的 / 还有草稿的）。
   */
  async renameUserPreset(path, newName) {
    const hit = mockMine.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const problem = mockNameProblem(hit.fileName, newName)
    if (problem !== null) throw new Error(problem)
    const name = newName.trim()
    const dir = path.slice(0, path.length - hit.fileName.length)
    const newPath = `${dir}${name}`
    if (newPath !== path && mockMine.some((f) => f.path === newPath)) {
      throw new Error(`已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`)
    }
    if (newPath !== path) {
      /* 使用中指针跟着改（指纹原样 —— 字节没动）；草稿跟着改（「接着上次改」不接丢） */
      if (mockActive?.origin === 'mine' && mockActive.path === path) {
        mockActive = { ...mockActive, path: newPath, fileName: name }
      }
      if (mockDraft !== null && mockDraft.origin === 'mine' && mockDraft.path === path) {
        mockDraft = { ...mockDraft, path: newPath, sourceFileName: name }
      }
      hit.path = newPath
      hit.fileName = name
      const text = mockMineText.get(path)
      if (text !== undefined) {
        mockMineText.delete(path)
        mockMineText.set(newPath, text)
      }
      /* 逐参数「官方更新」决定账按**路径**认这一份 —— 改名之后它得跟到新路径上
         （不跟的话，官方下一次发布会把处理过的项全翻出来） */
      const book = mockParamDecisions.get(path)
      if (book !== undefined) {
        mockParamDecisions.delete(path)
        mockParamDecisions.set(newPath, book)
      }
    }
    return { path: newPath, fileName: name }
  },

  /*
   * 第十一层：另存为一份新的（我的文件 → 我的文件）。与真机同一套规矩：
   * **字节复制**（正文与血统原样带过去）→ 新的一份从诞生起就是独立的；
   * 名字由用户自己起（不自动改名）：和原来一样 / 已存在同名都拒；
   * **一个状态都不碰** —— 使用中指针与草稿都还留在原来那份上。
   */
  async copyUserPreset(path, newName) {
    const hit = mockMine.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    const problem = mockNameProblem(hit.fileName, newName)
    if (problem !== null) throw new Error(problem)
    const name = newName.trim()
    const dir = path.slice(0, path.length - hit.fileName.length)
    const newPath = `${dir}${name}`
    if (newPath === path) throw new Error('新名字和原来一样 —— 复制要起个不同的名字')
    if (mockMine.some((f) => f.path === newPath)) {
      throw new Error(`已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`)
    }
    const text = mockMineText.get(path)
    if (text !== undefined) mockMineText.set(newPath, text)
    mockMine.push({
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
   * 这台机型 / 这个版本，「我那一份」在哪（校准页：初值从它读、保存写它）。
   * 与真机同一套匹配（归属优先、回落血统）—— 演示数据里 `我的 A1 涂胶.toml` 就是
   * 血统指向 `A1-fast` 的那一份，所以选 A1/快拆版能读到它。
   */
  async getUserCopyFor(machineId, versionId) {
    const hit = mockMine.find(
      (f) =>
        f.kind === 'mkp_preset' &&
        f.state === 'ok' &&
        (f.machineId ?? f.basedOnMachineId) === machineId &&
        (f.versionId ?? f.basedOnVersionId) === versionId,
    )
    if (hit === undefined) return null
    /* 浏览器里没有盘 ⇒ 没有绝对路径（真机那份是 `<appDataDir>/user/presets-mine/…`）。
       首页「复制后处理脚本」按它决定摆不摆按钮 —— 那一侧不摆，与真机上"这份还没取回"
       同一个答案：不许编一个本机路径出来。 */
    const absPath = null
    const text = mockMineText.get(hit.path)
    if (text === undefined) {
      return { fileName: hit.fileName, path: hit.path, absPath, axes: null, speed: null }
    }
    return {
      fileName: hit.fileName,
      path: hit.path,
      absPath,
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
    const taken = new Set(mockMine.map((f) => f.fileName))
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
      const path = `presets-mine/${name}`
      if (mockMine.some((f) => f.path === path)) {
        outcomes.push({
          source: item.source,
          ok: false,
          path: '',
          fileName: name,
          message: `已经有一份叫 ${name} 的文件了 —— 换个名字（这里不覆盖）`,
        })
        continue
      }
      mockMineText.set(path, MOCK_IMPORTED_TEXT)
      mockMine.push({
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
   * 也只是内存里一条记录 —— 前三步（只认我的文件 / 找得到 / 文件在不在）照真机走，
   * 最后一步如实说它在真机上的样子（不假装打开了）。
   */
  async revealInFolder(path) {
    if (!path.startsWith('presets-mine/')) {
      throw new Error('只给「我的文件」显示 —— 官方那两份住程序自己管的目录')
    }
    if (!mockMine.some((f) => f.path === path)) {
      throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了（列表以磁盘为准，刷新一下）`)
    }
    throw new Error(
      `假后端没有文件系统 —— 真机上这一步会打开 Finder / 资源管理器并选中「${path}」`,
    )
  },

  async deleteUserPreset(path) {
    const i = mockMine.findIndex((f) => f.path === path)
    if (i === -1) throw new Error(`找不到 ${path} —— 它可能已经被移走或删掉了`)
    /* 与真机同语义（2026-10-06 一切皆可删）：属于这一份的状态一并清掉，不再拦 */
    if (mockActive?.origin === 'mine' && mockActive.path === path) mockActive = null
    if (mockDraft !== null && mockDraft.origin === 'mine' && mockDraft.path === path) {
      mockDraft = null
    }
    mockMine.splice(i, 1)
    mockMineText.delete(path)
    /* 备注覆盖跟着删（删了重新下载 / 重新复制 = 回到工作台那句） */
    mockRemarks.delete(path)
    /* 逐参数「官方更新」决定账跟着删：文件都不在了，留着一本账只会是悬空的 */
    mockParamDecisions.delete(path)
  },

  async getSlicerCopied() {
    return slicerCopied()
  },

  async copyToSlicer(fileName) {
    /* 只改内存，刷新还原。传错类型会抛 —— 静默成功比报错难查得多 */
    copyToSlicerIn(fileName)
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
   * 「复制链接」要的是官方 URL，而官方地址在**真机的数据源设置**里 ——
   * 浏览器预览里没有它。**照实拒**，不编一个假 URL 让用户复制出去（那比报错糟得多）。
   */
  async getFileUrl() {
    throw new NotImplementedError(
      'getFileUrl',
      '浏览器预览里没有官方源地址 —— 复制链接请用桌面版（SupportEase 应用）',
    )
  },

  /**
   * 报告页的数据是 `mkp-ssr` 钩子落在本机的执行账 —— 浏览器里没有那棵树。
   * **空列表就是"还没有执行记录"**（与真机同一个答案），不编演示记录充数。
   */
  async getReportList() {
    return []
  },

  async getReportDetail() {
    throw new NotImplementedError(
      'getReportDetail',
      '浏览器预览里没有执行记录 —— 后处理报告请用桌面版（SupportEase 应用）看',
    )
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
      ],
      registry: catalogRegistry(),
      /* 品牌（含品牌图的资产 id）—— 客户端在机型与版本都没图时回落到它 */
      brands: [{ id: 'Bambu Lab', name: '拓竹 (Bambu Lab)', logo: 'bambu-lab-logo' }],
      plates: allPlates(),
    }
  },

  /** 浏览器里没有下载区也没有源：与 downloadFiles 同一条口径，不假装下载成功 */
  async downloadCatalogFile() {
    throw new NotImplementedError(
      'downloadCatalogFile',
      '浏览器预览里没有下载区 —— 下载 / 更新要用桌面版（SupportEase 应用）',
    )
  },

  /** 浏览器里没有下载区：没有那份文件可删 —— 与 downloadCatalogFile 同一条口径 */
  async deleteDeliveryFile() {
    throw new NotImplementedError(
      'deleteDeliveryFile',
      '浏览器预览里没有下载区，删不了官方交付文件 —— 用桌面版再删',
    )
  },

  /** 浏览器里没有归档区：与 readArchivedText 同一条口径 */
  async deleteArchivedFile() {
    throw new NotImplementedError(
      'deleteArchivedFile',
      '浏览器预览里没有归档区，删不了这份旧版本 —— 用桌面版再删',
    )
  },

  async downloadCatalogFiles(fileNames) {
    /* 一份都没成就直说，不返回"假装成功"的空结局表 */
    return fileNames.map((fileName) => ({
      fileName,
      ok: false,
      message: '浏览器里没有下载区，也没有数据源地址',
    }))
  },

  /** 浏览器里没有下载区，也就没有"已经下载的文件"可读：如实拒，不返回空串充数 */
  async readDownloadedText() {
    throw new NotImplementedError(
      'readDownloadedText',
      '浏览器预览里没有下载区，读不到这份的正文 —— 用桌面版就能看',
    )
  },

  /*
   * 数据源（设置页那一格）：真机写 `run/preset-source.json`，浏览器里没有盘 ——
   * 这一档走**内存镜像**（与用户目录 / 使用中指针同一套口径：能走通的就真走，走不通的如实说）。
   * 演示口径：这个假后端**没有内置默认源**（真机的两个内置是构建期注进来的），
   * 所以「使用内置官方源」在这里 = 回到"没配"。
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

  /* —— baseline + 对比台（2026-10-08）：固定演示版本账 + 从演示正文里读写参数 ——
     真机上第一条读目录 + 版本链、第二条读隐藏 baseline、后两条读写用户那份文件本身。 */
  async getOfficialVersions(fileName?: string | null) {
    const want = fileName?.trim()
    return MOCK_OFFICIAL_VERSIONS.filter((v) => !want || v.fileName === want).map((v) => ({ ...v }))
  },

  async readPresetParams(path: string): Promise<PresetParamValues> {
    const text = mockMineText.get(path)
    const fileName = path.split('/').pop() ?? path
    if (text === undefined) {
      return { path, fileName, values: {}, problem: `找不到 ${path}` }
    }
    return { path, fileName, values: readMockParams(text), problem: null }
  },

  async savePresetParams(path: string, edits: ParamEdit[]) {
    const text = mockMineText.get(path)
    if (text === undefined) throw new Error(`找不到 ${path} —— 没有动别的地方`)
    let next = text
    for (const e of edits) {
      const def = rawParamDefs().find((d) => d.key === e.paramKey)
      if (def === undefined) throw new Error(`不认识这个参数：${e.paramKey}`)
      next = patchMockParam(next, def, e.value)
    }
    if (next === text) return
    mockMineText.set(path, next)
    const at = mockMine.findIndex((f) => f.path === path)
    if (at >= 0) mockMine[at] = { ...mockMine[at]!, size: next.length, modifiedUnix: nowSec() }
  },

  /* —— 逐参数「官方更新」（2026-10-09）：内存夹具里的三方账 + 采用·保持 ——
     真机上第一条读我那份文件 + 隐藏 baseline + 目录里那一版；第二条结构保真地写那几项。
     `fetchMissing` 在假后端里没有对应的动作（没有网络），形状照留。 */
  async getPresetParamSync(path: string, fetchMissing?: boolean) {
    /* 假后端没有网络可发：`fetchMissing` 在这里没有对应的动作，形状照留（真机上有） */
    void fetchMissing
    return mockSyncOf(path)
  },

  async applyPresetParamDecisions(path: string, decisions: ParamDecision[]) {
    const text = mockMineText.get(path)
    if (text === undefined) throw new Error(`找不到 ${path}`)
    const before = mockSyncOf(path)
    const current =
      before.officialFileName === null ? undefined : mockParamCurrent.get(before.officialFileName)
    if (current === undefined) {
      throw new Error('官方当前版的正文还不在本机 —— 先把官方新版取回来，再处理这一项')
    }
    let next = text
    const book = mockParamDecisions.get(path) ?? new Map()
    for (const d of decisions) {
      if (d.kind === 'adopt') {
        const def = rawParamDefs().find((x) => x.key === d.paramKey)
        if (def === undefined) throw new Error(`不认识这个参数：${d.paramKey}`)
        const value = before.entries.find((e) => e.key === d.paramKey)?.officialNew
        if (value === null || value === undefined) {
          throw new Error(`官方当前版里没有 ${d.paramKey} 的新值 —— 采用不了`)
        }
        next = patchMockParam(next, def, value)
      }
      book.set(d.paramKey, { sha256: current.sha256, kind: d.kind })
    }
    if (next !== text) {
      mockMineText.set(path, next)
      const at = mockMine.findIndex((f) => f.path === path)
      if (at >= 0) mockMine[at] = { ...mockMine[at]!, size: next.length, modifiedUnix: nowSec() }
    }
    mockParamDecisions.set(path, book)
    return mockSyncOf(path)
  },

  /* 盘就是底账 —— 浏览器没有盘，这里给的是**固定演示集合**（见 `MOCK_DOWNLOADED`）：
     两份对得上目录、一份对不上。三个读合起来才够预设页画三态，这是其中两个 */
  async getDownloadedFiles() {
    return [...MOCK_DOWNLOADED]
  },

  async getStaleFiles() {
    return [...MOCK_STALE]
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
      { fileName: 'A1-fast.toml', verdict: 'old' as const, archivedPath: 'archive/dist/mkp/presets/A1-fast.toml' },
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
    return [
      {
        path: 'archive/dist/mkp/presets/A1-fast.toml',
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
   *   用户线（`presets-mine/…`）  假后端有一份**内存里的用户目录**（上面那两份演示），
   *                             所以这一档能真的走一遍：记指针、界面立刻变「已应用」
   *   官方线（`mkp/…`）           浏览器里没有下载区、也没有那份字节，**校验无从谈起** ——
   *                             与 downloadCatalogFile 同一条口径：如实拒，不假装成功
   */
  async applyActivePreset(fileName, origin, path) {
    if (origin !== 'mine') {
      throw new NotImplementedError(
        'applyActivePreset',
        `浏览器预览里没有 ${fileName} 的字节，应用官方预设要用桌面版（先在桌面版里下载一份）`,
      )
    }
    const hit = mockMine.find((f) => f.path === path)
    if (hit === undefined) throw new Error(`用户目录里没有 ${path ?? '(没给路径)'}`)
    /* 第九层：读不出来的那份**不许应用**（真机入口闸会拒；消息形状对齐后端） */
    if (hit.state === 'unreadable') {
      throw new Error(
        `${hit.path} 读不出来：${hit.stateDetail ?? 'TOML 语法不对'} —— 这一份现在不能应用、也不能改`,
      )
    }
    mockActive = {
      origin: 'mine',
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
   * 云端表那两个动作（2026-10-09 改判）：下载 / 更新 = 取回官方 + 落一份我的工作副本。
   * **不改「当前使用」** —— 使用是本地表那颗按钮的事。
   *
   * 浏览器里**没有下载区**，所以照实分两档：
   *   演示集合里已有（`MOCK_DOWNLOADED`）→ 当成"本机已有当前版"，
   *                                        然后落 `presets-mine/<原名>`（没有的话）；
   *   没有                              → 取不回来，如实抛（与 `downloadCatalogFile` 同一口径）。
   *
   * 与真机同一个形状：用户只面对 `presets-mine/` 里那一份，官方原件留在内部。
   */
  async fetchOfficialPreset(fileName) {
    const hit = mockReleaseFile(fileName)
    if (hit === undefined) throw new Error(`目录里没有 ${fileName} 这一份`)
    if (!MOCK_DOWNLOADED.some((f) => f.fileName === fileName)) {
      throw new NotImplementedError(
        'fetchOfficialPreset',
        `浏览器预览里没有 ${fileName} 的字节，也没法从数据源取回来 —— 用桌面版点「下载」`,
      )
    }
    const path = `presets-mine/${fileName}`
    const existing = mockMine.find((f) => f.path === path)
    const created = existing === undefined
    if (created) {
      const label = `dist/mkp/presets/${fileName}`
      const text = [
        `# machine: ${hit.machineId}`,
        `# variant: ${hit.versionId.toLowerCase()}`,
        `# based_on: ${label}`,
        `# based_on_release_time: 2026-10-06 03:34:17`,
        `# based_on_sha256: ${'0'.repeat(64)}`,
        MOCK_OFFICIAL_TEXT,
      ].join('\n')
      mockMineText.set(path, text)
      mockMine.push({
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
        /* 它是从官方原件落下来的：出处账记着从哪来（界面上「来源」那一格） */
        copiedFrom: label,
        copiedFromName: fileName,
        provenance: 'copy' as const,
      })
    }
    return { fetched: false, created, fileName, path }
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
