import { NotImplementedError } from './errors'
import type { ActivePreset, CalibModel, MkpApi, Preset, UserPresetFile } from './contract'
import {
  allMachines,
  allPresetFiles,
  appliedPreset,
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
 * - **本文件里**：校准页那三个方法（预设索引、校准板清单、三轴偏移）用的那几份手写常量。
 * - **`src/api/mockServer/`**：预设页 / 参数页 / 同步页要读的那十二个方法，
 *   由 `data/*.json` 的六份上游快照解析而来。那一整个目录是搬过来的假后端，
 *   真机上由 Rust 侧接管，`mockServer/` 整个不再被引用。
 *
 * 刻意**不加延迟**。真实情况是「连接慢、下载快」，而这里连接这一步根本不存在 ——
 * 凭空塞一个 300ms 只会让每次选机型都闪一下骨架屏，那是假的慢，不是真的慢。
 * 保留 async 只为形状一致：调用方必须按"这是个 Promise"来写，将来换成 IPC 才不用改。
 */

/**
 * 按「打印件版本」索引的预设 —— `getPreset` 的夹具。
 *
 * **这一份是 v023 时代留下的**：那三份全是 A1 mini 的，键（`std` / `fast-old` /
 * `fast-260628`）来自已被替换掉的旧客户端常量表。每一份原来还带一个 `model` 字段，
 * 是给旧校准页反填「机型 + 版本」两级用的 —— 那页换掉之后没有消费者了，已经拿掉。
 *
 * 客户端换成 A40 那一套之后，**页面不再调 `getPreset` 了** —— 它们走文件体系
 * （`getMachines` / `getVersionFiles` / `getPresetFiles`），见 `src/app/calib/usePreset.ts`
 * 文件头那段说明。契约里的 `getPreset` 留着（`bridge` 那头对应 Rust 的 `get_preset`），
 * 所以这里继续给出一个像样的夹具，不删。
 *
 * 顺带记一笔：这个文件里原来还导出一份 `presetCatalog`，是给旧页面直接 import 的
 * （"假数据从 api 层漏进页面"的唯一一处）。旧页面删掉之后它就没有消费者了，已随 P1c 收尾删除 ——
 * 现在假数据的出口只剩 `mockApi` 这一张契约表。
 */
const presetIndex: Record<string, Preset> = {
  std: {
    name: 'A1M.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1M.toml',
    axes: { x: -0.6, y: 22.4, z: 3.8 },
    speed: 60,
  },
  'fast-old': {
    name: 'A1MF.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF.toml',
    axes: { x: -0.8, y: 22.8, z: 3.9 },
    speed: 65,
  },
  'fast-260628': {
    name: 'A1MF_260628.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF_260628.toml',
    axes: { x: -0.9, y: 23, z: 4 },
    speed: 70,
  },
}

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

/*
 * 浏览器里的「用户目录」：**内存态**（刷新还原）—— 与 `mockActive` 同一套做法。
 *
 * 为什么让它在内存里真的能走通：编辑那条链（改 → 临时文件 → 另存）是这一层最需要被看见的
 * 东西；只抛"未实现"的话，浏览器里连编辑器长什么样、保存之后表格怎么变都验不了。
 * 真机上它是 `~/Documents/SupportEase/presets-mine/`，盘就是底账。
 */
const mockMine: UserPresetFile[] = [
  {
    path: 'presets-mine/我的 A1 涂胶.toml',
    fileName: '我的 A1 涂胶.toml',
    size: 2048,
    modifiedUnix: 1780000000,
    kind: 'mkp_preset',
  },
  {
    path: 'presets-mine/Process_0.2mm.json',
    fileName: 'Process_0.2mm.json',
    size: 1024,
    modifiedUnix: 1780003600,
    kind: null,
  },
]
/** 正文库：只有**这份会话里另存出来的**才有（真机上每一份都能读）。键 = 相对用户根的路径 */
const mockMineText = new Map<string, string>()
/** 编辑中的那一份（真机上是 `run/draft-preset.json`） */
let mockDraft: { sourceFileName: string; text: string; updatedUnix: number } | null = null

/** 演示正文。真机上它是官方原件（`mkp/…`）的字节 —— 假后端没有文件系统，只能给一段 */
const MOCK_OFFICIAL_TEXT =
  '# 假后端的演示正文 —— 真机上这里是官方原件（mkp/…）的字节\n涂胶宽度 = 1.2\n起始延时 = 0.5\n'

const nowSec = () => Math.floor(Date.now() / 1000)

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
 */
const MOCK_DOWNLOADED = ['A1-standard.toml']
const MOCK_STALE = ['A1-fast.toml', 'A1mini-standard.toml']

export const mockApi: MkpApi = {
  async getPreset(variantId) {
    const row = presetIndex[variantId]
    if (!row) return null
    return { name: row.name, path: row.path, axes: row.axes, speed: row.speed }
  },

  /*
   * 只记一条日志，不回写 presetIndex。
   *
   * 想过在内存里留一份"已保存的偏移"让 getPreset 读回来，但那一份没有归属 ——
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
  async beginPresetEdit(fileName) {
    if (mockDraft !== null && mockDraft.sourceFileName === fileName) {
      return { ...mockDraft, reused: true }
    }
    mockDraft = { sourceFileName: fileName, text: MOCK_OFFICIAL_TEXT, updatedUnix: nowSec() }
    return { ...mockDraft, reused: false }
  },

  async putPresetDraft(text) {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份')
    mockDraft = { ...mockDraft, text, updatedUnix: nowSec() }
  },

  async discardPresetDraft() {
    mockDraft = null
  },

  /** 另存成 `presets-mine/<原名>（已修改）.toml` —— 与 Rust 侧 `mine::edited_name` 同一条规则 */
  async commitPresetDraft() {
    if (mockDraft === null) throw new Error('现在没有正在改的那一份，没得存')
    const fileName = mockDraft.sourceFileName.replace(/\.toml$/i, '（已修改）.toml')
    const path = `presets-mine/${fileName}`
    const size = mockDraft.text.length
    const replaced = mockMine.some((f) => f.path === path)
    if (replaced) {
      mockMine[mockMine.findIndex((f) => f.path === path)] = {
        path,
        fileName,
        size,
        modifiedUnix: nowSec(),
        kind: 'mkp_preset',
      }
    } else {
      mockMine.push({ path, fileName, size, modifiedUnix: nowSec(), kind: 'mkp_preset' })
    }
    mockMineText.set(path, mockDraft.text)
    mockDraft = null
    return { path, fileName, size, replaced }
  },

  async getAppliedPreset() {
    /* null = 一套都还没应用。这是合法状态，不是错误 */
    return appliedPreset()
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
          path: 'mkp/presets/A1-standard.toml',
          machineId: 'A1',
          versionId: 'STANDARD',
          sha256: '0'.repeat(64),
          size: 2048,
        },
        {
          kind: 'mkp_preset',
          fileName: 'A1-fast.toml',
          path: 'mkp/presets/A1-fast.toml',
          machineId: 'A1',
          versionId: 'FAST',
          sha256: '1'.repeat(64),
          size: 2048,
        },
        {
          /* 「内容异常」那一档的演示：盘上有它、但与目录对不上，而且哪儿都查不出它是哪一版 */
          kind: 'mkp_preset',
          fileName: 'A1mini-standard.toml',
          path: 'mkp/presets/A1mini-standard.toml',
          machineId: 'A1_MINI',
          versionId: 'STANDARD',
          sha256: '2'.repeat(64),
          size: 2048,
        },
      ],
      registry: catalogRegistry(),
    }
  },

  /** 浏览器里没有下载区也没有源：与 downloadFiles 同一条口径，不假装下载成功 */
  async downloadCatalogFile() {
    throw new NotImplementedError('downloadCatalogFile')
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

  /** 浏览器模式下数据源既读不到也配不了：如实答"没配"，页面据此把配置入口说清楚 */
  async getPresetSource() {
    return null
  },

  async setPresetSource() {
    throw new NotImplementedError('setPresetSource：浏览器模式的数据源只读，配不了')
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
      { fileName: 'A1-fast.toml', verdict: 'old' as const, archivedPath: 'archive/mkp/presets/A1-fast.toml' },
      { fileName: 'A1mini-standard.toml', verdict: 'tampered' as const, archivedPath: null },
    ]
  },

  /*
   * 归档区：浏览器里没有盘，给一条**固定演示** —— 让"这一份有旧版本"那一格画得出来。
   * 真机上它是扫 `archive/` 得到的（换版本时被换下来的那一份）。
   * 认人那三格（机型 / 版本 / kind）跟着演示的那条走 —— 形状不编，数字编。
   */
  async getArchivedFiles() {
    return [
      {
        path: 'archive/mkp/presets/A1-fast.toml',
        fileName: 'A1-fast.toml',
        size: 2048,
        modifiedUnix: 1780000000,
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

  /* 使用中指针（新数据世界的第一个用户状态）：浏览器里记在内存，刷新即还原。
     没有文件落地，所以 apply 只对已"模拟下载"的文件开——这里没有，恒拒，如实 */
  async getActivePreset() {
    return mockActive
  },

  async applyActivePreset(fileName) {
    throw new NotImplementedError(`applyActivePreset(${fileName})：浏览器里没有下载区，先用真机下载一份`)
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
}
