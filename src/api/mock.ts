import { NotImplementedError } from './errors'
import type { CalibModel, MkpApi, Preset } from './contract'
import {
  allMachines,
  allPresetFiles,
  appliedPreset,
  copyToSlicerIn,
  localFileIds,
  localUserFiles,
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

  async getLocalUserFiles() {
    /* 同样是一份手写的演示集合：用户自己放进预设目录的那些，云端没有它们 */
    return localUserFiles()
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
   * 新数据世界（第一圈）的浏览器演示。真机上 Rust 读的是释放进数据根的
   * catalog.json（真数据、真 SHA）；浏览器里没有数据根，给一份同形状的最小演示 ——
   * 数字是编的，形状不编：页面按什么结构读，真机上就读得到。
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
      ],
      files: [
        {
          kind: 'mkp_preset',
          fileName: 'A1-standard.toml',
          path: 'mkp/A1-standard.toml',
          machineId: 'A1',
          versionId: 'STANDARD',
          sha256: '0'.repeat(64),
          size: 2048,
        },
      ],
    }
  },
}
