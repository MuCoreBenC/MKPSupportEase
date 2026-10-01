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


/**
 * 一个参数的值从哪来。
 *
 * `base` = 机型的基础配方（改它，这台机型的全部版本都会变）
 * `variant` = 这个版本自己盖过的值（只影响这一个版本）
 */
export type ParamOrigin = 'base' | 'variant'

/** 参数怎么显示、怎么改，全部由后端下发 —— 界面不写死任何键名与分组 */
export interface RecipeParam {
  /** 参数注册表里的稳定标识。TOML 路径会随分段调整而变，这个不会 */
  key: string
  label: string
  /** 一句话说明，鼠标悬停时显示。注册表本来就有这份数据 */
  desc: string
  /** 分组名，来自注册表的 section —— 界面不自己造分类 */
  group: string
  unit?: string
  control: 'number' | 'switch' | 'choice' | 'text'
  /** control === 'choice' 时的可选项 */
  choices?: { value: string; label: string }[]
  min?: number
  max?: number
  step?: number
  /** 当前值。**一律字符串** —— 单位换算与小数位数不该由界面再做一遍 */
  value: string
  origin: ParamOrigin
  /** 只有 origin === 'variant' 时有：基础配方里的那个值，用来显示「还原成」与对照 */
  baseValue?: string
}

/** 一台机型的一个版本（标准版 / 快拆版 / lite 版…） */
export interface MachineVersion {
  /** 'STANDARD' | 'FAST' | 'FASTV3.3' | 'LITE'，不是每台机型都有三档 */
  id: string
  /** 给人看的名字，'标准版' / '快拆版260628' */
  name: string
  /** 角标，'推荐' / '热门' / '最新'；没有就不显示 */
  tag?: string
  description?: string
  /**
   * 这个版本用哪一套 bundle。**空字符串 = 这个版本还没配**（上游的 A2L 就是这样），
   * 不是出错 —— 界面该显示「未配置」。非空时保证 bundle 一定存在（后端启动时校验过）。
   */
  bundle: string
}

export interface BedSize {
  width: number
  depth: number
}

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

export interface MachineFlags {
  /** G-code 里用来认机型的那行注释 */
  gcodeMarker: string
  hasSecondFan: boolean
}

/**
 * 机型尺寸。挂在机型上而不是版本上 —— 换快拆件不会改床身尺寸。
 *
 * 上游这里是 `map[string]any`，叶子字段连名字都不保证；这边逐个定死，
 * 少一个字段就编译不过，省得界面上出现 `undefined mm`。
 */
export interface MachineDimensions {
  bedSize: BedSize
  movementRange: MovementRange
  glueArea: GlueArea
  calibration: CalibrationPoints
  /** 边缘留白，mm */
  edgeZone: number
  flags: MachineFlags
}

export interface ZonePoint {
  x: number
  y: number
}

/** 禁区多边形。按需配置，不是每台机型都有 */
export interface ForbiddenZone {
  points: ZonePoint[]
}

export interface Machine {
  /** 'A1' / 'A1_MINI' / 'P1S'…，规范形 `^[A-Z][A-Z0-9_]*$` */
  id: string
  /** 给人看的名字，'A1 mini' */
  display: string
  /** 品牌显示名，'拓竹 (Bambu Lab)' */
  brand: string
  /** 机型图 / 图标的文件名，前端自己拼资源路径 */
  image: string
  icon: string
  /** 别名，用来认 G-code 里写的机型名（'A1MINI' / 'A1MC'…） */
  aliases: string[]
  versions: MachineVersion[]
  /**
   * `null` = **这台机型还没配尺寸**（上游的 A2L）。
   * 不给空对象也不给 0 —— 那会让「没配」和「配成 0」长得一样。
   */
  dimensions: MachineDimensions | null
  /** 没有禁区就是空数组。目前只有 P1S / P2S / X1C 有 */
  forbiddenZones: ForbiddenZone[]
}

export type FileKind = 'mkp_preset' | 'bbs_profile' | 'orca_profile'

/** 一个要下载/应用的具体文件 */
export interface FileRef {
  kind: FileKind
  /** 界面上显示的文件名 */
  fileName: string
  /**
   * 文件落点：MKP 预设相对**内部数据根**（'mkp/A1-fastv3.3.toml'，即下载区的登记落点）；
   * 切片器配置相对**预设仓库根**（'presets/bbs/…'，前端按它拼内置资源 URL）。
   * 前端只把它当标识/去重键用，不拿它拼本机路径。
   */
  path: string
  /**
   * `mkp_preset` 那一支带**真值**（运行时 catalog 的文件条目对交付产物真字节算的，
   * 下载校验拿它当期望值）；切片器那两支仍是 undefined —— 资产本体的登记与
   * 下载要等交付管道接管（总纲欠账 #3），没有就显示「未知」。
   */
  size?: number
  sha256?: string
}

/** 「这个版本要哪些文件」的答案 */
export interface VersionFiles {
  files: FileRef[]
  /** true = 该有的文件没配齐。界面要说「未配置」，不是「0 个文件」 */
  incomplete: boolean
  /** 没配齐的具体原因，一条一句，可直接显示 */
  missing: string[]
}

/**
 * 本机文件的现状。**和机型清单分开给** ——
 * 清单是静态的，状态会随下载变；混成一个字段会逼着每次下载完都重拉整张表。
 *
 * 四档的区别（B03 起改了字面量，见下）：
 *
 *   `unavailable`  后端就没配这个版本的文件（A2L）。**不是本机的问题**
 *   `missing`      配了，本地还没有
 *   `ready`        本地文件齐了，还没应用到切片器/配置
 *   `applied`      已经应用生效
 *
 * B03 的改动只有两处：`unconfigured` 改名成 `unavailable`（「没配」是后端侧的事实，
 * 用 un-configured 容易被读成「用户没配置」），`partial` 换成 `applied`。
 * `partial`（本地有一半文件）这一档在假后端里从来没被返回过，也没有任何页面判过它 ——
 * 留着只是给人一个可以编假进度的口子。反过来「下好了」与「应用了」是真的两件事，
 * 那一档缺了才会逼界面自己猜。
 */
export type FilesState = 'unavailable' | 'missing' | 'ready' | 'applied'

export interface ParamSection {
  /** `space_offset` / `disk_action` …，与 param 的 layout.sectionId 对应 */
  id: string
  /** 中文名，'空间偏移' / '圆盘动作控制'。这个名字只在注册表里有，布局表那边没有 */
  label: string
  desc?: string
  /** 这一组里有几条。**已按 machineFilter 与废弃过滤**，所以是这台机型真实看得到的条数 */
  count: number
}

export interface ParamTab {
  /** 'offset' | 'wiping' | 'fan' | 'glue' | 'gcode' | 'advanced' */
  id: string
  /** '偏移' | '擦料' | '风扇' | '涂胶' | '切换' | '更多' */
  label: string
  /** 这个分类下有几条（= 各 section 的 count 之和） */
  count: number
  sections: ParamSection[]
}

/**
 * 一个参数的全部元信息 —— 键名、作用域、约束、布局、可见性条件。
 *
 * `RecipeParam` 给的是「怎么显示、值是多少」，这里给的是「它到底是什么」。
 * 工作台的编辑抽屉和矩阵列头要这些，普通参数页不需要，所以分开拉。
 */
export interface ParamMeta {
  key: string
  /** TOML 里的键名 */
  tomlKey: string
  jsonKey: string
  /** 后端配置结构里的字段名 */
  configKey: string
  /** TOML 节名（`toolhead` / `wiping`），与 sectionId 不是一回事 */
  section: string
  /** 布局分组 id */
  sectionId: string
  /** 组内顺序。可以是小数 */
  order: number
  scope: 'universal' | 'machine_specific'
  variantMode?: 'shared' | 'per_variant'
  valueType: 'float' | 'int' | 'bool' | 'string'
  /** 'number' | 'switch' | 'segmented' | 'select' | 'gcode' —— 一共只有这五种 */
  uiComponent: string
  unit?: string
  /** 写进 TOML 的行尾注释 */
  tomlComment?: string
  mergeGroup?: string
  pinned?: boolean
  deprecated?: boolean
  /** 只对这些机型生效。上游是逗号字符串，这里已拆成数组；不限机型时不给这个字段 */
  machineFilter?: string[]
  /**
   * 可见性条件。**判断放前端** —— 它依赖当前草稿值，是交互态不是数据。
   * 只有一层，但被指向的字段自己也可能有条件，所以要顺着 key 往上递归。
   */
  showWhen?: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }
}

/** 一处「机型:版本」。倒查结果里到处要用它，所以单独一个名字 */
export interface VersionRef {
  machine: string
  version: string
}

/** 预设仓库里的一个文件 */
export interface PresetFileInfo {
  /** asset id，等宽显示 */
  id: string
  fileName: string
  /** 相对预设仓库根 */
  path: string
  kind: FileKind
  /** 'process' 之类；mkp_preset 为空 */
  category: string
  machineIds: string[]
  /** 只有 bbs_profile 有 —— MKP 的 .toml 是涂胶预设，本来就没有喷嘴层高这回事 */
  nozzle?: string
  layerHeight?: string
  /** 被哪些 bundle 装着 */
  inBundles: string[]
  /** 被哪些「机型:版本」当作 MKP 预设 */
  usedByVersions: VersionRef[]
  /**
   * 这个文件的**交付身份**。只有两种，没有第三种：
   *
   *   `default`   默认交付 —— 你为某个版本钦定的**最终资源**。用户点下载，后台自动下好。
   *   `optional`  可选 —— 仓库里有、你没放进默认集。**在用户端的预设列表里看得到，用户自己手动下。**
   *
   * 判据不变（有没有被任何 bundle 装着、有没有被任何版本当 MKP 预设），换的是说法：
   * 「没进默认集」是一个正常状态，不是「孤儿」也不是错误 —— 实测 4 个（全是 0.2mm 的
   * 工艺 profile）。所以它不带警示色、不进 `checkResources()` 的问题清单，
   * 但要在界面上显式可见（能筛、能看见数量），否则可选与默认就分不出来了。
   */
  delivery: 'default' | 'optional'
  /**
   * 文件大小，**已经格式化好的字符串**（`4.2 KB`）。
   *
   * 为什么叫 `sizeText` 而不是 `size`：`FileRef.size` 是 `number`（字节），而这一层给的是
   * 一个可直接显示的串。同名不同类型会让 `PresetFileInfo` 不再结构兼容 `Pick<FileRef, 'size'>`，
   * 实测直接把 A32 与 A34 两稿编译弄坏了 —— 冻结的稿不能因为新字段报错。
   *
   * **两种来源，用 `statFrom` 分开**（T16）：
   *
   *   `'file'`  bbs 这一档：真仓里那份文件的**真值**（`data/bbs_files.json` 的字节数与
   *            上游 manifest 记的更新时间，由 `scripts/sync-bbs-presets.mjs` 核对过 sha256）
   *   `'demo'`  其余：假后端按 `path` 稳定推出来的**演示值** —— 但同一个文件每次刷新都一样，
   *            不是随机数。真后端要 `stat`（上游 `assets_index.json` 里 `size` 全是 0）
   *
   * 为什么不干脆不给演示值：整个界面还有一半是演示数据，列表上少了「大小」会让版面看起来没做完。
   * 但**前端一行都不许自己编** —— 编在假后端，真后端接上时换掉的是一个函数，不是一堆界面代码。
   */
  sizeText?: string
  /** 修改时间（`09-14` / ISO）。同上，两种来源见 `statFrom` */
  modifiedText?: string
  /**
   * 上面那两格是哪来的。**缺省 = 不在这一档里**（自己的文件本来就没有这两个字段）。
   *
   * 这个记号的存在理由是「界面上不许说错话」：真值那两格的 tooltip 要说「来自仓库里那个
   * 真文件」，演示值那两格照旧说「演示数据」。混在一列里靠猜是不行的。
   */
  statFrom?: 'file' | 'demo'
}

/**
 * **用户自己的文件**（A34）。
 *
 * 和 `PresetFileInfo` 是两种东西，不要混：
 *
 *   `PresetFileInfo`  官方仓库里的文件。有 asset id、有交付身份、云端有一份权威副本
 *   `LocalUserFile`   用户自己放进预设目录的文件。**云端没有它**，所以没有 asset id、
 *                     没有 `delivery`、也不参与套餐与菜单
 *
 * 这是「本地的就是本地的，云端的就是云端的」这条规则在类型上的体现：用户端的预设页有
 * 两张互不相干的表，本地表里能看到这种文件，云端表里永远看不到。
 *
 * 权限也因此不同：官方下载下来的副本**只读**（不能改名，删了能重新下回来），
 * 这种文件可以随便改名，但删了就没有任何地方能找回来。
 *
 * **假后端返回一份固定演示集合**，见 `src/server/resolve/localFiles.ts`。
 */
export interface LocalUserFile {
  /** 本机唯一。真后端用绝对路径的哈希，假后端直接写死一个 `user_` 前缀的串 */
  id: string
  fileName: string
  /** 相对预设仓库根 */
  path: string
  kind: FileKind
  /** 用户自己标的适用机型。可以是空数组 —— 他没标就是没标，不要替他猜 */
  machineIds: string[]
  /** 只有切片器 profile 才有。MKP 的涂胶预设没有喷嘴层高这回事 */
  nozzle?: string
  layerHeight?: string
}

/**
 * **正在生效的那一套预设。全局唯一。**
 *
 * 这是 A34 这一轮纠正的一个模型错误。原来前端自己推：「当前机型 + 当前版本那个默认交付的
 * MKP 预设」—— 于是切一下机型「已应用」就换一个，等于说这台机器同时应用着 6 套配置。
 * 物理上不成立：涂胶笔同一时间只跑一套。
 *
 * 所以「已应用」不是一个可以从别的数据算出来的派生量，它是**一条独立的事实**，
 * 只有后端知道（它读的是本机那份「当前配置」）。前端一律来问。
 *
 * `null` 不是错误 —— 新装的机器就是这个状态，界面要能把「还没有应用任何预设」
 * 和「加载失败」分开说。
 */
export interface AppliedPreset {
  /** 正在生效的那个 asset id */
  assetId: string
  path: string
  /** 它属于哪个机型 / 版本 —— 参数页要编的就是这一套 */
  machineId: string
  versionId: string
}

/**
 * 菜单的一条：这个文件对客户端公开到什么程度。
 *
 *   `bundled`   已分配 —— 在某个套餐里，客户端自动下
 *   `optional`  可选 —— 在菜单上，客户端看得到、可手动下。**逐瓶指定**，不是「同机型的都算」
 *   `archived`  仅归档 —— 不在菜单上，**客户端完全不知道它存在**
 *
 * 仓库里有 ≠ 客户端能拿到。不上菜单就下不了，这是这张表存在的全部理由。
 */
export interface MenuEntry {
  /** asset id，与 `PresetFileInfo.id` 同一套标识 */
  fileId: string
  visibility: 'bundled' | 'optional' | 'archived'
}

/**
 * 客户端数据包的兼容性声明。**值初始留空。**
 *
 * `minClientVersion` 的具体数字要客户端先给一份兼容性清单，工作台**不许瞎填一个版本号** ——
 * 填了就等于对外承诺「这份数据在 x.y.z 以上都能用」，而没人验证过。
 * 所以它初始就是 `null`，空着时由 `checkRecipe()` 报一条**待办**（不是阻断），
 * 由 `getPublishIssues()` 报一条**阻断**（发布检查那一组，见下）。
 *
 * B04 补上了 `saveClientDataMeta` —— 有了写方法，那条永远填不上的待办才填得上。
 */
export interface ClientDataMeta {
  schemaVersion: number
  /** 空 = 还没填。生成前会报一条待办，发布前是一条阻断 */
  minClientVersion: string | null
}

/** 摊平后的一个机型 —— 客户端不做继承推导 */
export interface ClientMachine {
  id: string
  display: string
  brand: string
  dimensions: MachineDimensions | null
  versions: {
    id: string
    name: string
    tag?: string
    description?: string
    /** 这个版本的袋子里装什么。已按套餐摊平成文件清单 */
    files: FileRef[]
    /**
     * 这个版本的参数值 —— **已经三层算完的有效值**，键是参数注册表的稳定 key。
     *
     * 客户端**看不到机型基底这一层存在**：它拿到的就是「这个版本用什么值」，
     * 没有 origin、没有 baseValue、没有「哪一层给的」。那些是后厨的账。
     */
    values: Record<string, string>
    /** true = 暂不支持该机型或版本（配方本上有名字，资源一行没写） */
    unsupported: boolean
  }[]
}

/** 参数的**显示**元信息。不含继承规则、不含机型基底与版本覆盖 */
export interface ClientFieldDef {
  key: string
  label: string
  desc?: string
  unit?: string
  control: RecipeParam['control']
  /**
   * 控件形态的**原始**名字（A40 补）：`number` / `switch` / `segmented` / `select` / `gcode`。
   *
   * 为什么 `control` 之外还要这一栏：`control` 是**给画控件用的四档**
   * （number / switch / choice / text），分段与下拉都并成 `choice`、而 G-code 落成 `text`。
   * 客户端于是只能靠「只有 gcode 会落到 text」这个**巧合**反推 G-code —— 巧合不该是契约。
   * 这一栏把注册表的原词带出来，客户端要细分（分段 vs 下拉、G-code 块）就有据可依。
   */
  uiComponent: string
  /**
   * 值**本身**的类型（C15 / A40 补）。
   *
   * 与 `control` 不是一回事：`control` 回答「画什么控件」（number / switch / segmented /
   * select / gcode），这一栏回答「值是什么」（float / int / bool / string）。
   * 开关是 bool、下拉是 string —— 客户端要按类型校验、要显示「这是什么」，
   * 就不能拿控件去猜。
   */
  valueType: 'float' | 'int' | 'bool' | 'string'
  /**
   * 可见性条件（C15 / A40 补）：要 `key` 这个字段等于（或不等于 / 大于）`value` 才显示。
   *
   * 这一条原来只活在工作台里 —— 客户端拿到包却没有它，只能自己写死「哪些参数属于
   * 哪个模式」（模式开关：擦料方式 = 擦料塔 / 圆盘擦拭，选哪支显示哪支）。
   * 判据由客户端算（它依赖当前值），**数据由包里带** —— 客户端不再猜业务规则。
   */
  showWhen?: ParamMeta['showWhen']
  choices?: { value: string; label: string }[]
  min?: number
  max?: number
  step?: number
  /** 分组的中文名 */
  group: string
  /** 所属分类的中文名（`擦料`）。`tabId` 没有的老包靠它兜底 */
  tab: string
  /**
   * 所属分类的 **id**（T8 补，可选）：`offset` / `wiping` / `fan` / `glue` / `gcode` / `advanced`。
   *
   * 为什么 `tab` 之外还要这一栏：`tab` 是**中文名**，客户端拿它当 id 用就会踩 locale 的坑
   * （A40 分类条的图标表按英文 id 查，包里全是中文名，六个图标全塌成兜底那一个）。
   * id 稳定、名字可翻译 —— 老包没有这一栏时客户端退回 `tab` 照跑（只加字段，不改老语义）。
   */
  tabId?: string
}

/**
 * 一次发布的**另一个产物**：一份真正的预设文件（T7.1）。
 *
 * 作者把这件事说透了：「我们现在模拟的是『用户自己去下载一个 JSON 数据包』，但**真实客户端
 * 应该是自动同步/更新发布数据，用户真正下载、安装、使用的是 TOML 预设**」。
 *
 * 所以一次发布同时产生两样东西，**属于同一个 preset identity**：
 *
 *   `ClientDataPackage`  客户端说明书 —— 自动同步，用户看不见「下载 JSON」这个动作
 *   `ReleasePreset`      真正的 preset artifact —— 用户手动「获取预设」拿到它
 *
 * 不许出现「JSON 是 1.0.1、TOML 还是 1.0.0」这种原型层面的假链路。
 */
export interface ReleasePreset {
  machineId: string
  versionId: string
  /** 本机落盘的文件名（注册表里那一栏 `presetFile`，比如 `A1.toml`） */
  fileName: string
  /** TOML 正文 */
  content: string
}

/** 一次发布 = 说明书 + 若干份预设文件 */
export interface Release {
  /** 包版本（工作台发布时填的那个三段数字） */
  version: string | null
  at: string | null
  package: ClientDataPackage
  presets: ReleasePreset[]
}

export interface ClientDataPackage {
  meta: ClientDataMeta
  machines: ClientMachine[]
  fields: ClientFieldDef[]
  /** 柜台上单卖的：菜单里 optional 的那些 */
  optionalFiles: FileRef[]
  /**
   * 输入指纹，用来判「已过期」。
   *
   * 把配方本 + 菜单 + 套餐 + 字段定义 + 兼容声明排序后 JSON 化再取的**稳定结构化摘要**，
   * **不是文件哈希**，也不作完整性校验 —— 它只回答「现在的输入和上次生成时是不是同一份」。
   *
   * 刻意**没有 `generatedAt`**：假后端里没有可信的时间源（`Date.now()` 在这一层没意义，
   * 产物也不落盘）。「上次生成」由调用方自己记。
   */
  inputsHash: string
}

/**
 * 客户端要后端干的事。**这一份是产品仓的口径，不是试验场那份的照抄**：
 * 试验场把四个轨（客户端 / 工作台 / 原型 / 测试端）的方法并在一张表里（38 个），
 * 产品仓的用户端只用得到下面这些 —— 工作台那一套走自己的 `src/workbench/api.ts`（`wb_*`）。
 *
 * 命名规则：读用 get*，写用 save*，让壳去做的动作用动词（openModel / copyToSlicer）。
 *
 * 前四个是 v023 移植时就有的；「客户端接发布包」这一轮（P1）补的是后面十二个 ——
 * 预设页 / 参数页 / 同步页三页要读的东西。**本轮只有 mock 答得上来**，
 * 真机上没接的那几个由 bridge 抛 NotImplementedError（`src/api/errors.ts`），
 * 界面上是一块「未接入」空态，不是白屏。
 */

/* ——— 新数据世界（第一圈）：随包 catalog —— ——— */

/**
 * 运行时目录里的**一份交付文件**。`path` 是相对内部数据根的落点（下载区 `mkp/`）——
 * 下载它就该落到那；`sha256` / `size` 是发布时对产物真字节算的，将来下载完拿它校验。
 */
export interface RuntimeCatalogFile {
  kind: string
  fileName: string
  path: string
  machineId: string
  versionId: string
  sha256: string
  size: number
}

export interface RuntimeCatalogMachine {
  id: string
  display: string
  brand: string
  versions: { id: string; name: string }[]
}

/**
 * catalog definition 里的**字段定义**（与 Rust `presetdata::ParamDef` 的 serde 形态对齐）。
 * 只声明消费面读的格子；JSON 里有更多字段（default_value / machine_variants …），
 * 见 `src-tauri/src/presetdata/registry.rs` —— 前端消费到哪一栏，声明就长到哪一栏。
 */
export interface CatalogParamDef {
  key: string
  /** 数据域分区（= key 前缀）。**不是界面分组** —— 分组看 layout.sectionId */
  section: string
  /** 参数自己声明的界面归属（组内顺序 + 属于哪个分组） */
  layout: { order: number; sectionId: string }
  deprecated?: boolean
  machineFilter?: string[]
}

/** 页签与分组的元数据（中文名、顺序的唯一权威；`layout_schema` 全文没有 label） */
export interface CatalogTabMeta {
  id: string
  label: string
  order: number
  icon?: string
  sections: { id: string; label: string; order: number; description?: string }[]
}

/** 参数摆放（`layout_schema`：哪个参数落在哪个 section） */
export interface CatalogLayoutTab {
  id: string
  sections: { id: string; items: { id: string; paramKey: string }[] }[]
}

/**
 * catalog 的 definition 注册表部分（字段定义 + 页签元数据 + 参数摆放）。
 * 参数页的页签/分组树从它摊 —— **首屏唯一数据源 = catalog**（总纲判据 4）。
 */
export interface CatalogRegistry {
  params: CatalogParamDef[]
  tabs: CatalogTabMeta[]
  layout: CatalogLayoutTab[]
}

/**
 * 新数据世界的说明书（`<appDataDir>/catalog.json`，随安装包释放）。
 * 形状与 Rust 侧 `runtime::catalog::Catalog` 一一对应，两边没有编译器，
 * 对齐靠 `docs/DATA-ARCHITECTURE.md` 与判据测试。
 *
 * 第二圈加厚后 Rust 侧还序列化 definition（brands / 机型的完整字段 / assets /
 * bundles / registry）。前端声明随消费面长出来：本轮先长 `registry`
 * （参数页的页签/分组树从它摊），其余几域消费时再声明。
 */
export interface RuntimeCatalog {
  catalogSchema: number
  /** 目录指纹：源或交付产物变了它就变 —— 将来「该不该同步」看它，不作完整性校验 */
  revision: string
  machines: RuntimeCatalogMachine[]
  files: RuntimeCatalogFile[]
  registry: CatalogRegistry
}

/**
 * 使用中指针（新数据世界的第一个用户状态，全局唯一）。
 * `intact` 是"盘上那份还是应用时刻的那份"——`mkp/` 是只读区，正常恒 true；
 * false 说明字节漂了（被手动动过 / 文件没了），界面要照实说。
 */
export interface ActivePreset {
  fileName: string
  sha256: string
  machineId: string
  versionId: string
  intact: boolean
}

/**
 * 远端目录检查结果（两端共用契约：比较的是 revision 指纹，不逐项 diff）。
 * 开发期远端 = 工作台发布的 dist/catalog.json；真云端来了只换来源，这个形状不动。
 */
export interface RemoteUpdateCheck {
  upToDate: boolean
  localRevision: string
  remoteRevision: string
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

  /* ——— 机型与文件（预设页 / 同步页要读的） ——— */

  /** 机型目录：品牌 → 机型 → 版本。客户端画三级选择用 */
  getMachines(): Promise<Machine[]>

  /**
   * 某个「机型:版本」下配了哪些文件。
   * `null` = 后端没有这个组合（不是空），与 `VersionFiles.files` 空数组是两件事。
   */
  getVersionFiles(machineId: string, versionId: string): Promise<VersionFiles | null>

  /** 本机预设目录里**已经有的**文件名（官方那一批，固定演示集合） */
  getLocalFiles(): Promise<string[]>

  /** 用户自己放进预设目录的文件 */
  getLocalUserFiles(): Promise<LocalUserFile[]>

  /** 正在生效的那一套。**全局唯一**，null = 一套都还没应用（不是错误） */
  getAppliedPreset(): Promise<AppliedPreset | null>

  /** 已经复制到切片器目录的那些（切片器文件的「生效」与 MKP 不是一回事） */
  getSlicerCopied(): Promise<string[]>

  /** 把一份切片器配置复制到切片器目录。写方法：真机上会落盘 */
  copyToSlicer(assetId: string): Promise<void>

  /** 预设仓库的清单（含交付身份 / 大小 / 修改时间） */
  getPresetFiles(): Promise<PresetFileInfo[]>

  /** 菜单表：每个文件对客户端公开到什么程度（bundled / optional / archived） */
  getMenu(): Promise<MenuEntry[]>

  /** 参数注册表（名字 / 类型 / 控件 / 选项 / 范围 / 条件 / 单位） */
  getParamMeta(): Promise<ParamMeta[]>

  /**
   * 某一版参数的值与元数据。`versionId` 传 `null` = 只看机型基底。
   * 客户端**不做继承推导** —— 这一份是后端摊平好的结果。
   */
  getMachineParams(machineId: string, versionId: string | null): Promise<RecipeParam[]>

  /**
   * 下载选中的文件（官方那一批）。
   * 试验场的假后端对这个方法是**故意抛**的（那里没有真网络），真机上是 Rust 的活。
   */
  downloadFiles(refs: FileRef[]): Promise<void>

  /**
   * 新数据世界的目录（第一圈骨架）。读运行时释放进数据根的那份 catalog.json，
   * 零网络 —— 与 getMachines（旧世界解析 TOML 树）并存，收口后由它接班。
   */
  getRuntimeCatalog(): Promise<RuntimeCatalog>

  /**
   * 新数据世界的下载管道（第一圈骨架）：把 catalog 里登记的一份拉进下载区 mkp/。
   * SHA 或大小对不上就整个拒绝——坏字节不落盘。第一圈只有开发源，
   * 真云端来了换管道里的源实现，这个口子不动。
   */
  downloadCatalogFile(fileName: string): Promise<void>

  /**
   * 已经下载到下载区的文件名。盘就是底账：文件在且 SHA 对得上才算数，不查缓存。
   */
  getDownloadedFiles(): Promise<string[]>

  /**
   * 有更新的文件名：盘上在、但字节与目录不一致（目录更新带来新版本，或文件被动过）。
   * "更新"就是对这些再跑一遍 downloadCatalogFile——旧份自动归档。
   */
  getStaleFiles(): Promise<string[]>

  /**
   * 当前使用的是哪一份（全局唯一）。null = 还没用任何一份，是合法状态不是错误。
   */
  getActivePreset(): Promise<ActivePreset | null>

  /**
   * 「使用这一份」：把下载区里某份登记过的文件记成使用中。
   * 文件没下载 / 盘上字节与目录对不上 → reject，不会应用半份。
   */
  applyActivePreset(fileName: string): Promise<ActivePreset>

  /**
   * 撤销使用。幂等：本来就没在用也不报错。
   */
  clearActivePreset(): Promise<void>

  /**
   * 检查更新：对远端目录比较指纹（本地 vs 远端的 revision）。
   */
  checkRemoteUpdate(): Promise<RemoteUpdateCheck>

  /**
   * 应用远端目录：旧目录归档、新目录生效。之后照常走「有更新」→ 下载，
   * 没有第三条更新路径。
   */
  applyRemoteUpdate(): Promise<void>
}

/** 方法名，报错时用来指出是哪个口子没接 */
export type MkpApiMethod = keyof MkpApi

/* ---------- 错误模型：与 src-tauri/src/error.rs 逐字段对齐 ---------- */

/**
 * 错误分类。值与 Rust 侧的 `ErrorCode`（serde SCREAMING_SNAKE_CASE）一一对应，
 * 那边有单元测试钉住字段名与取值 —— 两份声明之间没有编译器，只能靠测试对齐。
 */
export type ErrorCode =
  | 'NOT_FOUND'
  | 'PERMISSION_DENIED'
  | 'INVALID_ARGUMENT'
  | 'CORRUPTED'
  | 'SHA_MISMATCH'
  | 'IO'
  | 'NOT_IMPLEMENTED'
  | 'INTERNAL'

/**
 * 跨 IPC 边界的错误。
 *
 * `message` 可以直接显示给用户（中文）；`detail` 是给开发看的技术细节；
 * `traceId` 是这次调用在日志里的编号 —— 界面要把它露出来，否则日志查不到人。
 */
export interface AppError {
  code: ErrorCode
  message: string
  traceId: string
  detail?: string
}

/** 运行时判断 reject 出来的东西是不是本结构。不是的话由 bridge 兜底成 INTERNAL */
export function isAppError(v: unknown): v is AppError {
  if (typeof v !== 'object' || v === null) return false
  const e = v as Record<string, unknown>
  return typeof e.code === 'string' && typeof e.message === 'string' && typeof e.traceId === 'string'
}
