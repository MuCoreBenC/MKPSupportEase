import type { CalibModel, MkpApi, Preset, TestModel } from './contract'

/**
 * mock 实现：同一套契约，数据写死在本文件里。
 *
 * **这个文件是临时实现，不是数据层。** Rust 接管时替换的是 `mock.ts` → `rust.ts` 这一个文件，
 * 页面一行不动 —— 所以业务假数据必须住在这里，不能散到页面或 `src/app/constants/` 去。
 * 界面结构数据（页签、品牌/机型/版本三级选项）是另一回事，那些属于应用本身，在 `src/app/constants/`。
 *
 * 数据来自试验场的 `src/mock/mkpFull.ts` 与 `src/mock/testModels.ts`，只取本仓库真正用到的三份：
 * 预设索引、校准板清单、测试模型清单。那两个文件（549 + 100 行，含大量其他页面的假数据）不搬。
 *
 * 刻意**不加延迟**。真实情况是「连接慢、下载快」，而这里连接这一步根本不存在 ——
 * 凭空塞一个 300ms 只会让每次选机型都闪一下骨架屏，那是假的慢，不是真的慢。
 * 保留 async 只为形状一致：调用方必须按"这是个 Promise"来写，将来换成 IPC 才不用改。
 */

/**
 * 按「打印件版本」索引的预设。键是 `src/app/constants/variants.ts` 里的 id。
 *
 * 三个版本给三份不同的文件与偏移 —— 这样「换版本 → 文件名和 XYZ 真的都变了」在界面上看得见。
 * 每份带一个 model（`constants/models.ts` 的 id）：校准页的预设下拉选了某一份之后，
 * 要能反填「机型 + 打印件版本」两级，光有 variant 不够。三份都是 A1 mini 那台的文件。
 */
const presetIndex: Record<string, Preset & { model: string }> = {
  std: {
    name: 'A1M.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1M.toml',
    model: 'a1mini',
    axes: { x: -0.6, y: 22.4, z: 3.8 },
    speed: 60,
  },
  'fast-old': {
    name: 'A1MF.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF.toml',
    model: 'a1mini',
    axes: { x: -0.8, y: 22.8, z: 3.9 },
    speed: 65,
  },
  'fast-260628': {
    name: 'A1MF_260628.toml',
    path: 'C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF_260628.toml',
    model: 'a1mini',
    axes: { x: -0.9, y: 23, z: 4 },
    speed: 70,
  },
}

/**
 * 预设目录 —— 界面上「有哪几份预设可选」。
 *
 * 契约里暂时没有对应的方法（五个 command 已经定死，见 doc §4），所以这里以同步常量的形式
 * 暴露给校准头的下拉与两个页面的反填逻辑。Rust 接管时这条会变成第六个 command，
 * 届时改的仍然只是本文件与调用点的三行，不是页面结构。
 */
export interface PresetCatalogEntry {
  /** `constants/variants.ts` 里的 id，同时是 getPreset 的键 */
  variant: string
  name: string
  path: string
  /** `constants/models.ts` 里的 id —— 选了预设要能反填「机型 + 版本」两级 */
  model: string
}

export const presetCatalog: PresetCatalogEntry[] = Object.entries(presetIndex).map(
  ([variant, row]) => ({ variant, name: row.name, path: row.path, model: row.model }),
)

const calibModels: CalibModel[] = [
  { id: 'z', name: 'Z 轴校准', desc: '校准喷嘴高度与第一层，先打这个', size: '284 KB', ready: true },
  { id: 'xy', name: 'XY 校准', desc: '校准平面内的偏移，Z 轴之后打', size: '377 KB', ready: true },
  { id: 'sup', name: '支撑测试', desc: '校准完打这个看支撑效果', size: '3.2 MB', ready: true },
]

/**
 * 测试模型清单。
 *
 * 文案与时长/用量逐字取自产品侧的 model_copy 真相源；缩略图是从那个 3mf 的
 * `Metadata/plate_N.png` 里提出来的切片器盘面预览（512×512 透明底），order 即盘号。
 * 例外是 3 号「鱼尾曲面测试」—— 换成了 Blender 渲出的 hero_fishtail.webp，
 * 同一张图也兼作走马灯右下角的装饰底图。
 *
 * 8 个模型共用同一个 3mf —— 「打开测试模型」按钮对哪张卡都是同一个文件。
 */
const testModels: TestModel[] = [
  {
    order: 1,
    title: '平面Z轴测试',
    subtitle: 'Z-Axis Calib',
    description: '基础平面涂胶验证，检验涂胶笔基础功能与 Z 轴偏移精度，确认笔尖出胶状态与高度定位是否正常。',
    image: '/models/plate_1.png',
    time: '8m',
    weight: '3.2g',
    tag: { label: '基础验证', color: '#EF4444' },
  },
  {
    order: 2,
    title: '半圆XY校准',
    subtitle: 'XY Calibration',
    description: '高精度测试 X/Y/Z 轴基础偏移量，精确检测校准状态与笔尖质量，验证半圆弧面的涂胶轨迹精度。',
    image: '/models/plate_2.png',
    time: '12m',
    weight: '4.5g',
    tag: { label: '尺寸校准', color: '#F97316' },
  },
  {
    order: 3,
    title: '鱼尾曲面测试',
    subtitle: 'Overhang Quality',
    description: '进阶测试涂胶均匀性与小面积涂胶处理能力，检验鱼尾结构悬垂面的涂胶覆盖质量与边缘一致性。',
    image: '/models/hero_fishtail.webp',
    time: '18m',
    weight: '6.2g',
    tag: { label: '曲面质量', color: '#EAB308' },
  },
  {
    order: 4,
    title: '综合测试阵列',
    subtitle: 'Multi Benchmark',
    description: '多模型组合阵列测试，全面评估涂胶系统的各项核心指标与参数配合，综合检验不同结构间的涂胶衔接。',
    image: '/models/plate_4.png',
    time: '1h 5m',
    weight: '21.7g',
    tag: { label: '综合评估', color: '#22C55E' },
  },
  {
    order: 5,
    title: '无支撑大平面',
    subtitle: 'No-Support Test',
    description: '单支撑面与大面积涂胶专项测试，验证无辅助支撑条件下大平面涂胶的均匀性、边缘覆盖与胶量控制。',
    image: '/models/plate_5.png',
    time: '47m',
    weight: '15.5g',
    tag: { label: '进阶组合', color: '#06B6D4' },
  },
  {
    order: 6,
    title: '极限综合测试',
    subtitle: 'Ultimate Clearance',
    description: '全结构整合的极限测试，深度检验整体涂胶效果与长时间作业稳定性，评估系统在复杂工况下的性能边界。',
    image: '/models/plate_6.png',
    time: '35m',
    weight: '11g',
    tag: { label: '极限测试', color: '#3B82F6' },
  },
  {
    order: 7,
    title: 'BMCU冲刷测试',
    subtitle: 'Flow Test',
    description: '一个较高、支撑面较多的模型可以充分测试使用 BMCU 时是否冲刷。',
    image: '/models/plate_7.png',
    time: '',
    weight: '',
    tag: { label: '冲刷测试', color: '#8B5CF6' },
    isNew: true,
  },
  {
    order: 8,
    title: '擦料塔测试',
    description: '',
    image: '/models/plate_8.png',
    time: '',
    weight: '',
    tag: { label: '料塔测试', color: '#3B82F6' },
    isNew: true,
  },
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

  async getTestModels() {
    return testModels
  },
}
