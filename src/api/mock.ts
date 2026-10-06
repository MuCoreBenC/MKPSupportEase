import { NotImplementedError } from './errors'
import type {
  ActiveOrigin,
  ActivePreset,
  CalibModel,
  MkpApi,
  OnDiskFile,
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
  { id: 'z', name: 'Z 轴校准', desc: '校准喷嘴高度与第一层，先打这个', size: '284 KB', ready: true },
  { id: 'xy', name: 'XY 校准', desc: '校准平面内的偏移，Z 轴之后打', size: '377 KB', ready: true },
  { id: 'sup', name: '支撑测试', desc: '校准完打这个看支撑效果', size: '3.2 MB', ready: true },
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
    /* 带血统、而且**基于旧版官方**（官方已经换到新版）：界面上要有「基于旧版官方」那一枚 */
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
    /* 演示里没走过复制/导入 —— 出处照实没有（真机上由出处账答） */
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
    copiedFrom: null,
    copiedFromName: null,
    provenance: null,
  },
]
/** 正文库。键 = 相对用户根的路径。真机上每一份都能读；假后端里先把演示那份种上 */
const mockMineText = new Map<string, string>()

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
    '# based_on: dist/mkp/presets/A1-fast.toml',
    '# based_on_release_time: 2026-05-29 04:26:12',
    `# based_on_sha256: ${'0'.repeat(64)}`,
    '"涂胶宽度" = 1.1',
    '"起始延时" = 0.4',
    '',
  ].join('\n'),
)
/* 坏了那份也种一份正文：**看正文照旧读得出来**（用户要能看着它去修）——
   只有「应用 / 改这份」被拦（真机上是后端的文件级检查拦的） */
mockMineText.set(
  'presets-mine/坏了的涂胶.toml',
  ['# 坏了的演示（假后端演示正文）', '[toolhead]', 'offset_x = (1', ''].join('\n'),
)

const nowSec = () => Math.floor(Date.now() / 1000)

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
 * 浏览器里的「下载区」：三份，**固定演示集合**（真机上是盘 `mkp/`，盘就是底账）。
 *
 * 为什么不是一份：预设页的交付行有四种状态（未下载 / 已下载 / 旧版本 / 内容异常），
 * 只给"未下载"一种的话，另外三种在浏览器里**根本画不出来** —— 而它们正是这一层
 * 最需要被看见的东西（"内容异常"尤其：那一档以前会被显示成"需更新"）。
 *
 * 三份各占一档，**同一档里的两份不存在**：交付构造上每个 (机型, 版本) 只有一份产物，
 * 再塞一份同版本的条目就是**编形状**了。「未下载」那一档在浏览器里由官方行的「下载」
 * 按钮覆盖（同一套动作列），真机上则由"目录里登记了、下载区还没有"的那些行覆盖。
 *
 * 三份都**不是真的能下**：点「下载」/「更新」/「重新下载」仍如实抛"浏览器里没有下载区"，
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
   * 只记一条日志，不假装持久化。
   *
   * 想过在内存里留一份"已保存的偏移"让界面读回来，但那一份没有归属 ——
   * 契约里 saveOffsets 不带 variantId（写的是当前机器的配置，不是某份预设文件），
   * 于是在 A 预设上保存、切到 B 会看见 A 的数。宁可这一轮不假装持久化：
   * 界面自己有 saved 状态，看得见"存下去了"，真正的落盘等 Rust 侧。
   */
  async saveOffsets(axes) {
    console.info('[mock] saveOffsets', axes)
  },

  async getCalibModels() {
    return calibModels
  },

  async openModel(modelId) {
    console.info('[mock] openModel', modelId)
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

  /** 只有这份会话另存出来的那份有正文可读（真机上每一份都能读） */
  async readUserPresetText(path) {
    const text = mockMineText.get(path)
    if (text === undefined) {
      throw new NotImplementedError(
        'readUserPresetText：浏览器里只有这份会话另存出来的那份有正文',
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
          `beginPresetEdit：浏览器里只有那份演示正文能改（${rel} 没有正文）`,
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
        throw new NotImplementedError(`commitPresetDraft：${rel} 已经没有正文可写回`)
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
  },

  async getSlicerCopied() {
    return slicerCopied()
  },

  async copyToSlicer(assetId) {
    /* 只改内存，刷新还原。传错类型会抛 —— 静默成功比报错难查得多 */
    copyToSlicerIn(assetId)
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
   * 假后端对这个方法是**故意抛**的（浏览器里没有真网络），产品仓照同一条口径：
   * 不假装下载成功 —— 「下载点了没反应」比「点了说成功但盘上什么都没有」好查。
   */
  async downloadFiles() {
    throw new NotImplementedError('downloadFiles')
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
      files: [
        {
          kind: 'mkp_preset',
          fileName: 'A1-standard.toml',
          path: 'dist/mkp/presets/A1-standard.toml',
          machineId: 'A1',
          versionId: 'STANDARD',
          sha256: '0'.repeat(64),
          size: 2048,
        },
        {
          kind: 'mkp_preset',
          fileName: 'A1-fast.toml',
          path: 'dist/mkp/presets/A1-fast.toml',
          machineId: 'A1',
          versionId: 'FAST',
          sha256: '1'.repeat(64),
          size: 2048,
        },
        {
          /* 「内容异常」那一档的演示：盘上有它、但与目录对不上，而且哪儿都查不出它是哪一版 */
          kind: 'mkp_preset',
          fileName: 'A1mini-standard.toml',
          path: 'dist/mkp/presets/A1mini-standard.toml',
          machineId: 'A1_MINI',
          versionId: 'STANDARD',
          sha256: '2'.repeat(64),
          size: 2048,
        },
        {
          /*
           * 切片器那一类的交付文件（`bbs_config`）：预设页按 `kind` 把它分流进
           * 「切片器配置 → 云端」——MKP 档**不列它**（真机 catalog 里它们占 9 条，
           * 2026-10-02 作者截图里混进 MKP 表的就有它）。
           */
          kind: 'bbs_config',
          fileName: 'MKPProcess A1 0.4 0.20.json',
          path: 'assets/bbs/Process/0.4mm/MKPProcess A1 0.4 0.20.json',
          machineId: 'A1',
          versionId: '',
          sha256: '3'.repeat(64),
          size: 1332,
        },
        {
          /*
           * 图标（`icon`）：**不归预设页** —— 它是资源，由自己的资源体系消费。
           * 登记在 catalog 里只为钉住一条判据：「登记了」不等于「预设页要显示」
           * （同截图的 `a1.svg`）。
           */
          kind: 'icon',
          fileName: 'a1.svg',
          path: 'assets/icons/a1.svg',
          machineId: 'A1',
          versionId: '',
          sha256: '4'.repeat(64),
          size: 2400,
        },
      ],
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
    throw new NotImplementedError('downloadCatalogFile')
  },

  /** 浏览器里没有下载区：没有那份文件可删 —— 与 downloadCatalogFile 同一条口径 */
  async deleteDeliveryFile() {
    throw new NotImplementedError('deleteDeliveryFile：浏览器里没有下载区')
  },

  /** 浏览器里没有归档区：与 readArchivedText 同一条口径 */
  async deleteArchivedFile() {
    throw new NotImplementedError('deleteArchivedFile：浏览器里没有归档区')
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
    throw new NotImplementedError('readDownloadedText：浏览器里没有下载区')
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
    throw new NotImplementedError('readArchivedText：浏览器里没有归档区，先用真机跑一次更新')
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
        `applyActivePreset(${fileName})：浏览器里没有下载区，那份字节不在，先用真机下载一份`,
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

  /* 浏览器里没有远端（真远端 = 工作台发布的 dist，或将来的云端）：如实说没有 */
  async checkRemoteUpdate() {
    throw new NotImplementedError('checkRemoteUpdate：浏览器里没有远端目录')
  },

  async applyRemoteUpdate() {
    throw new NotImplementedError('applyRemoteUpdate：浏览器里没有远端目录')
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
