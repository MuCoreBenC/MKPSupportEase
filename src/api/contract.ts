/**
 * 前端与后端之间唯一的约定。
 *
 * 这个文件里只有类型，没有实现 —— 于是「接口长什么样」与「这一轮由谁来答」被彻底分开：
 * 现在答的是 mock（src/api/mock.ts），将来答的是桌面壳注入的那份（src/api/bridge.ts）。
 * 页面只认这里的签名，换实现不动页面。
 *
 * 命名规则：读用 get*，写用 save*，让壳去做的动作用动词（openModel）。
 */

/** 三轴偏移，单位 mm。x / y 是平面内的笔尖偏移，z 是笔尖高度 */
export interface Axes {
  x: number
  y: number
  z: number
}

/** 一份预设 = 某机型某打印件版本对应的那个 toml，外加它带来的偏移基准 */
export interface Preset {
  /** 文件名，界面上直接显示 */
  name: string
  /** 本机绝对路径，鼠标悬停时看 */
  path: string
  axes: Axes
  speed: number
}

/** 校准板模型（Z 板 / XY 板 / 支撑测试件） */
export interface CalibModel {
  id: string
  name: string
  desc: string
  /** 已经是给人看的字符串（'284 KB'），不是字节数 —— 单位换算不该由界面再做一遍 */
  size: string
  ready: boolean
}

/** 测试模型清单里的一件。走马灯与第五页共用这一份形状 */
export interface TestModel {
  /** 盘号，同时是卡片右上角的水印数字 */
  order: number
  title: string
  /** 英文名，大写小字号排在标题下面；最后两条产品侧就没给 */
  subtitle?: string
  description: string
  /** public 下的绝对路径 */
  image: string
  /** 打印时长；产品侧未给的留空字符串，渲染时整行隐藏 */
  time: string
  /** 耗材用量，同上 */
  weight: string
  tag: { label: string; color: string }
  isNew?: boolean
}

export interface MkpApi {
  /**
   * 取某个打印件版本对应的预设。
   *
   * 返回 `null` 是「没有这一份」（新增了机型版本但后端还没配预设），不是出错 ——
   * 调用方把它当"什么都没选"处理，不要拿半份数据糊弄。
   * 真出错（连不上、文件坏了）走 reject。
   */
  getPreset(variantId: string): Promise<Preset | null>

  /** 把校准好的三轴偏移写回配置。三轴一起写，不按页分 */
  saveOffsets(axes: Axes): Promise<void>

  /** 校准板清单 */
  getCalibModels(): Promise<CalibModel[]>

  /**
   * 让壳去打开一个模型文件（本地有缓存就直接开，没有就先下载）。
   * 前端不碰文件系统，也不关心它是下载还是命中缓存 —— 那是壳的事。
   */
  openModel(modelId: string): Promise<void>

  /** 测试模型清单 */
  getTestModels(): Promise<TestModel[]>
}

/** 方法名，报错时用来指出是哪个口子没接 */
export type MkpApiMethod = keyof MkpApi
