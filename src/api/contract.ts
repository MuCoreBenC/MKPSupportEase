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
  /**
   * 这个参数在 MKP 预设 TOML 里对应的**字段名**（`offset_x` / `speed_limit`…）。
   *
   * 与 `ParamMeta.tomlKey` 同值同源（`ParamDef.toml_key`）—— 两条通道都带着它，是因为
   * 用的地方不同：参数页拿这一份改值（字段级 patch 要它）。**一一对应**一个 TOML 字段
   * （2026-10-02 起 `offset = { x, y, z }` 已拆成 `offset_x/y/z`，不再有共享 tomlKey 的形状）。
   */
  tomlKey: string
  label: string
  /** 一句话说明，鼠标悬停时显示。注册表本来就有这份数据 */
  desc: string
  /** 分组名，来自注册表的 section —— 界面不自己造分类 */
  group: string
  unit?: string
  control: 'number' | 'switch' | 'choice' | 'text'
  /**
   * control === 'choice' 时的可选项。
   *
   * `deprecated`（选项级弃用，2026-10-02）：上游把某一档标成「正在退场」。**不是**字段级弃用
   * —— 参数本身还是活的，只是这一个取值不许再被写成新值（老文件里存着的照旧读得到、能还原）。
   * 客户端把它映射到共用的 `FieldOption.deprecated`（划线），不自己造第二个弃用信号。
   */
  choices?: { value: string; label: string; deprecated?: boolean }[]
  min?: number
  max?: number
  step?: number
  /** 当前值。**一律字符串** —— 单位换算与小数位数不该由界面再做一遍 */
  value: string
  origin: ParamOrigin
  /** 只有 origin === 'variant' 时有：基础配方里的那个值，用来显示「还原成」与对照 */
  baseValue?: string
  /**
   * 字段级弃用（2026-10-02）：该字段已退出正常编辑 / 产物生成，但**仍属于已知参数**，
   * 所以参数页继续展示它的历史状态。
   *
   * 显示 ≠ 可编辑 ≠ 会进入新产物 —— 参数定义存在 / 参数页显示 / 用户编辑禁止 /
   * 新 TOML 不生成。客户端靠 `ParamMeta.deprecated` 判（定义通道那边带），这一栏是
   * 配方通道的镜像：`getMachineParams` 照旧**排除**弃用字段，所以这里正常恒为 undefined。
   */
  deprecated?: boolean
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
   * **备注**（2026-10-07）：客户端预设列表的**副标题**用它 —— 不再显示
   * 「下载区 mkp · 路径」那种位置文案。可空（工作台可以不写；没写副标题就空着）。
   *
   * 两条消费链**互不干扰**（2026-10-07 三轮定案）：
   *   云端表  直接显示这一句（客户端只读，不叠加任何本地改动）；
   *   本地表  用户在**备注覆盖账**（`MkpApi.getPresetRemarks`）里改过的优先
   *           （「更新不覆盖，除非他删了重新下载」），没有覆盖才用它。
   */
  remark?: string | null
  /**
   * 这个版本用哪一套 bundle。**空字符串 = 这个版本还没配**（上游的 A2L 就是这样），
   * 不是出错 —— 界面该显示「未配置」。非空时保证 bundle 一定存在（后端启动时校验过）。
   */
  bundle: string
  /**
   * **这一版专属的外观图**（**资产 id**，不是文件名）：去 `RuntimeCatalog.assets` 里按 id 查
   * `path`。**空字符串 = 回落机型图**（`Machine.image`）—— 不是"这一版没图"。
   *
   * 图位分层（作者 2026-10-03）：品牌图 → 机型图 → 版本图，版本缺则回落机型。
   * 「标准版」与「快拆版」是同一台机器下两个独立的版本实体，各自有各自的图。
   */
  image: string
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

/**
 * 可打印区在板轮廓坐标里的位置（mm）。**归 `Plate` 所有**。
 *
 * 机型 `MachineDimensions.bedSize` 是涂胶 / 运动口径（A1 记的是 260×255），
 * 真可打印区是这里（256×256）。塔地图必须活在可打印区里。
 */
export interface PlateFrame {
  x: number
  y: number
  w: number
  h: number
}

/**
 * 一块打印板（2026-10-02）。**独立于机型的实体**：机型只引用 id（`plateIds`），
 * 几何住在这里，随 catalog 的 `plates` 域下发（`RuntimeCatalog.plates`）。
 *
 * 坐标是板件自身的毫米：原点在板的**后缘左角**（y 向下指前缘）。`path` 含卡舌与把手，
 * 孔洞子路径靠 evenodd 镂空；`bodyPath` 是不含卡舌 / 把手的板身。
 */
export interface Plate {
  /** 稳定主键（`single-latch-256` / `dual-latch-180`）。机型的 `plateIds` 指向它 */
  id: string
  /** 给人看的名字（`单卡舌 256`） */
  name: string
  /** 板件外轮廓宽（viewBox 宽） */
  w: number
  /** 板件外轮廓深（viewBox 高） */
  d: number
  frame: PlateFrame
  /** 板身路径（不含卡舌 / 把手） */
  bodyPath: string
  /** 完整外轮廓（含卡舌 / 把手） */
  path: string
}

export interface Machine {
  /** 'A1' / 'A1_MINI' / 'P1S'…，规范形 `^[A-Z][A-Z0-9_]*$` */
  id: string
  /** 给人看的名字，'A1 mini' */
  display: string
  /** 品牌显示名，'拓竹 (Bambu Lab)' */
  brand: string
  /**
   * **资产 id**（不是文件名，也不是路径）：整机图。去 `RuntimeCatalog.assets` 里按 id 查
   * `path`，拼 `/assets/<path>` —— 客户端不再认识 `a1.webp` 这种具体文件（2026-10-03 第二刀）。
   * 空串 = 这台机型没有图（界面回落品牌 logo）
   */
  image: string
  /**
   * **第二个图位**：装了快拆件那张外观图（今天只有 A1 mini 有）。
   * 客户端在**选到版本这一级**显示它；空串/查不到则回落 `image`。
   * 这条关系 2026-10-03 之前只活在 `heroArt.ts` 的硬编码表里 —— 现在住机型文件
   */
  imageVariant: string
  icon: string
  /** 别名，用来认 G-code 里写的机型名（'A1MINI' / 'A1MC'…） */
  aliases: string[]
  /**
   * 这台机型能用的打印板 id（去 `RuntimeCatalog.plates` 里按 id 查板）。
   * 空数组 = 没有板规格（塔地图那一层不出）。**机型只持引用，不持几何**
   */
  plateIds: string[]
  /** 默认用哪一块板（塔地图按它选）；`null` = 没指定 */
  defaultPlateId: string | null
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
   * 文件落点 = **`catalog.path`**（唯一路径语义，2026-10-04）：
   * MKP 预设是 `dist/mkp/presets/A1-fastv3.3.toml`、切片器配置是 `assets/bbs/…`，
   * **同一串既是云端取哪、也是客户端放哪**。前端只把它当标识 / 去重键用，
   * 不拿它拼本机路径、也不拿它拼 URL。
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
 * **用户自己的一份文件**（用户线）。
 *
 * 与 `PresetFileInfo`（官方文件）是两种东西，不要混 —— 这是总纲 §1③「预设 TOML 的一生」
 * 在类型上的体现：
 *
 * ```text
 * 官方线                               用户线
 *   云端 → mkp/ → archive/              presets-mine/（另存出来的那一份）
 *   有 SHA、属于版本、不可变            云端没有它：没有 SHA、不属于任何版本、不参与套餐
 * ```
 *
 * 用户那份**从官方另存出来之后与云端脱钩**，可以自由改，**永远不回写官方原件**；
 * 反过来官方换版本也不会动它。
 *
 * **形状就是盘上的事实**：路径 / 文件名 / 大小 / 改动时刻 / 认得出的类别。
 * 没有「用户自己标的适用机型」这种字段 —— 今天没有任何地方能让用户去标它，
 * 留一个永远空的字段就是在编形状（需要它的那一步再加）。
 */
/**
 * 用户自己那份**基于的官方那一版**现在怎么样了（第七层）。
 *
 * 三档只回答一个问题：**它当初基于的那一版，和目录里现在这一版是不是同一份** ——
 * 不回答"这份用户文件好不好"（用户自己那份从来不是坏文件）。
 *
 * 判定用的是文件头血统里记的 `based_on_sha256`（建副本那一刻来源全文的摘要），
 * **不拿用户文件自己的字节去比**：用户改过的东西必然与官方不同，那样只会得出
 * "永远不一样"这种废话。
 */
export type BasedOn = 'current' | 'outdated' | 'unknown'

/**
 * 用户文件的**文件级**状态（第九层）：`ok` = 能读 + 是 UTF-8 + TOML 语法能解析；
 * `unreadable` = 读不出来（编码 / TOML 语法 / 指向用户根之外）。
 *
 * **它不回答"是不是一份合法 MKP Preset"**（有哪些字段、参数类型对不对 —— 那是 Preset 语义，
 * 客户端不复制那份 schema）；外部修改过但仍是能读的 TOML ⇒ 照常 `ok`，**不因 SHA 报警**。
 */
export type MineState = 'ok' | 'unreadable'

export interface UserPresetFile {
  /** 相对**用户根**的路径（`presets-mine/A1-fast.toml`）—— 读正文 / 应用时把它交回来 */
  path: string
  fileName: string
  size: number
  /** 最后改动时刻（UTF **epoch 秒**）。界面自己转人话：默认构建不引时间库 */
  modifiedUnix: number | null
  /**
   * 认得出是哪一类就给；**认不出是 `null`**。
   *
   * 后端只认 `.toml`（MKP 预设）—— 切片器那两类都是 `.json`，光看扩展名分不出
   * 是 bbs 还是 orca，所以照实认不出。界面上认不出的那一档**在任何类型档下都列**。
   */
  kind: FileKind | null
  /**
   * 第九层的**文件级**状态（见 [`MineState`]）。`unreadable` 的界面上说"文件无法读取"，
   * **不许应用 / 编辑**；不是预设候选（认不出是哪一类）时是 `null`
   */
  state: MineState | null
  /**
   * 用不了时后端给的一句人话原因（可直接显示，比如"TOML 语法不对（第 3 行第 1 列）"）；
   * 能用 / 不适用是 `null`
   */
  stateDetail: string | null
  /** 它基于的官方那一版在不在（见 [`BasedOn`]）。`outdated` 就是"官方换版了，你这份基于旧版" */
  basedOn: BasedOn
  /** 血统里记的来源（`dist/mkp/presets/A1-standard.toml`）。**没有血统是 `null`** */
  basedOnLabel: string | null
  /** 建副本那一刻来源文件头的版本号（给人看的）。不知道就是 `null` */
  basedOnRelease: string | null
  /** 来源那份**现在**对应哪台机型 / 哪个版本（认不出留 `null`，界面不猜） */
  basedOnMachineId: string | null
  basedOnVersionId: string | null
  /**
   * **这一份自己的归属**：文件头 `# machine:` / `# variant:` 两行（对着目录
   * 大小写无关归一化到版本 id）。头里没有 / 认不出 ⇒ 回落到来源那份
   * （`basedOnMachineId` / `basedOnVersionId`）。列表的机型 / 版本两列读它。
   *
   * 「改归属」（`setUserPresetMachineVersion`）改的就是文件头那两行 ——
   * 归属是**文件自己的属性**，随文件走。
   */
  machineId: string | null
  versionId: string | null
  /**
   * **出处账**记的来源：从用户自己的哪一份复制来的（相对用户根路径）。
   * 没记过 / 是导入的 / 来源已删除 ⇒ `null` —— 界面退回别的说法，不编。
   *
   * 为什么在账本里而不是文件头：另存是**按字节**复制（血统原样带过去），往文件头里
   * 加一行就得重写字节；导入的外部文件更不能动。账本只服务界面上「来源：复制自 X」
   * 一格，丢了就退回「我的」，什么都不坏。
   */
  copiedFrom: string | null
  /** 上面那条路径里的**文件名**（界面直接显示用）。没有同上 */
  copiedFromName: string | null
  /** 出处档：`copy`（复制自另一份用户文件）/ `import`（外部导入）。都没记 ⇒ `null` */
  provenance: 'copy' | 'import' | null
}

/**
 * **编辑中的那一份**（临时文件）。全局唯一 —— 同一时刻只改一份。
 *
 * 它是"临时编辑"这条链的第一步（总纲 §1③）。**两条线**的第一步：
 *
 * ```text
 * 官方线   dist/mkp/presets/A1-fast.toml ──改这份──▶ 临时文件 ──保存──▶ presets-mine/A1-fast（已修改）.toml
 * 用户线   presets-mine/A1-fast（已修改）.toml ──改这份──▶ 临时文件 ──保存──▶ 写回它自己（第八层）
 * ```
 *
 * 改的永远是**临时文件**（`run/draft-preset.json`，改到一半关掉也还在）；
 * 原件全程一动不动，官方那份只有"云端换版本"能替换它。
 */
export interface PresetDraft {
  /** 改的是哪一条线（官方交付文件 / 我自己那份）—— 保存按钮说什么由它决定 */
  origin: ActiveOrigin
  /** 从哪一份改出来的。官方线是 `mkp/` 里的文件名；用户线是给人看的文件名（落点看 `path`） */
  sourceFileName: string
  /** **用户线**的落点（相对用户根）：保存时写回这里。官方线是 `null`（落点由目录给） */
  path: string | null
  /** 正文：用户改到哪算哪 */
  text: string
  /** 最后改动时刻（UTC epoch 秒） */
  updatedUnix: number
  /** 这次打开是**接着上次改**（草稿本来就是这一份的），不是新建的 */
  reused: boolean
}

/** 另存完成的结果：用户文件落在哪、多大、是不是盖掉了上一次那份 */
export interface CommittedDraft {
  /** 相对**用户根**的路径（`presets-mine/A1-fast（已修改）.toml`） */
  path: string
  fileName: string
  size: number
  /** 盖掉了一份同名的用户文件（第二次保存就是这种） */
  replaced: boolean
}

/** 一份用户文件的新落点与名字（第十层改名 / 第十一层另存为一份新的，两处共用这一个形状） */
export interface UserFileIdentity {
  /** 落点（相对**用户根**，`presets-mine/…`） */
  path: string
  fileName: string
}

/** 落点检查的一档（第十二层）：`ready` 能收 / `collision` 重名 / `rejected` 收不了（带原因） */
export type ImportStage = 'ready' | 'collision' | 'rejected'

/** 一份外部文件的落点检查结果 */
export interface StagedImport {
  /** 用户给的那个外部路径（原样带回，只为对号入座；界面不显示它） */
  source: string
  /** 源文件名（落点的默认名字） */
  fileName: string
  state: ImportStage
  /** `rejected` 的原因（可直接显示）；别的档是 null */
  reason: string | null
}

/** 提交导入的一份：`newName` 只在"重名、用户改了名"时给 */
export interface ImportItem {
  source: string
  newName?: string
}

/** 导入的逐份结局（与下载同一副规矩：一份出错不拖累别人） */
export interface ImportOutcome {
  source: string
  ok: boolean
  /** 落进用户根之后的相对路径（`presets-mine/…`）；失败是空串 */
  path: string
  fileName: string
  /** 失败原因（可直接显示）；成功是空串 */
  message: string
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
 * 客户端要后端干的事。**这一份是产品仓的口径，不是试验场那份的照抄**：
 * 试验场把四个轨（客户端 / 工作台 / 原型 / 测试端）的方法并在一张表里（38 个），
 * 产品仓的用户端只用得到下面这些 —— 工作台那一套走自己的 `src/workbench/api.ts`（`wb_*`）。
 *
 * 命名规则：读用 get*，写用 save*，让壳去做的动作用动词（openModel / copyToSlicer）。
 *
 * 前四个是 v023 移植时就有的；「客户端接发布包」这一轮（P1）补的是后面十二个 ——
 * 预设页 / 参数页要读的东西（「同步」页 2026-10-02 退役，它当时读的
 * `getPresetSource` / `setPresetSource` 现在归设置页）。**本轮只有 mock 答得上来**，
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

/** 品牌（`presets/brands.toml` 那一行）。`logo` 是**资产 id**，不是文件名 */
export interface RuntimeCatalogBrand {
  /** `'Bambu Lab'` —— 稳定 id */
  id: string
  /** `'拓竹 (Bambu Lab)'` —— 给人看的显示名（机型上的 `brand` 就是它） */
  name: string
  /** 品牌图的资产 id；空/查不到 = 回落内置字标 */
  logo: string | null
}

export interface RuntimeCatalogMachine {
  id: string
  display: string
  brand: string
  versions: { id: string; name: string }[]
}

/**
 * 归档区里的一份**官方旧版本**（cloud 换版本时被换下来的那一份）。
 *
 * 归档是**官方版本生命周期**的一部分，**不是用户修改历史**：换版本时旧份进
 * `archive/`（保留最早一份，不覆盖、不删）；用户改出来的东西是另一条线
 * （另存成另一份文件），永远不回写官方原件。
 */
export interface ArchivedFile {
  /** 相对内部根的路径（`archive/dist/mkp/presets/A1-fast.toml`）—— 读正文时把它交回来 */
  path: string
  /** 文件名。与它对应的交付文件同名：换版本换的是字节，不是名字 */
  fileName: string
  size: number
  /** 这份旧字节的指纹 —— 替换事件与版本出身的对号键 */
  sha256: string
  /**
   * 这一版**在云端发布**的时刻（RFC3339）。**跟着这一版走，不跟着操作走**：
   * 查版本出身反查得到；链建立（2026-10-06）之前的版本查不到 = `null`
   * —— 界面照实「未知（早于版本记忆）」，**不拿"现在"或换下时刻顶**。
   */
  publishedAt: string | null
  /**
   * 被**换下来**的时刻（UTC **epoch 秒**）= `DeliveryReplaced.at`（事件账）。
   * 与 `publishedAt` 是两个时间、两件事：一个是"这一版什么时候发的"，
   * 一个是"它是什么时候被换下来的" —— 永不互相顶替。
   */
  replacedUnix: number | null
  /** 认得出是谁的旧版本就有；**认不出是 `null`**（目录里已经没有这一份了）—— 不猜 */
  machineId: string | null
  versionId: string | null
  kind: string | null
}

/**
 * 盘上这一份**认得出是哪一版吗**（第三圈第 6 层：官方文件的 SHA 报警）。
 *
 * **只答"本机这份是不是我们认可的官方内容"**，不掺"云端有没有更新"（后者是
 * [`checkRemoteUpdate`]，比的是目录指纹，与本机这一份的字节无关）。混成一句「需更新」，
 * 用户既不知道自己的文件是不是被改过，也不知道该不该等更新 —— 所以它是单独一条读。
 *
 * **只列有事的**：还没下载、和与目录逐字节一致的两种不出现（它们没有问题，
 * 列进来只会把真正要处理的那几份淹掉）。
 */
export interface DeliveryTrust {
  fileName: string
  /**
   * `old`      认得出它是官方的某一版旧版（归档里有它字节，或被归档的旧目录登记过）
   * `tampered` 目录、归档、旧目录都对不上 —— 这台机器上查不出它属于哪一版
   */
  verdict: 'old' | 'tampered'
  /**
   * `old` 且归档区里有它字节时给（`archive/dist/mkp/presets/A1-fast.toml`）—— 界面据此
   * 把那一版旧正文读出来给人对。被旧目录登记、归档里没字节的那种是 `null`
   */
  archivedPath: string | null
}

/**
 * 盘上交付区里的一份（`getDownloadedFiles` / `getStaleFiles` 的行形状）。
 *
 * 时间**全部来自事件**（2026-10-06 预设事件时间模型），mtime 不再上界面：
 *
 * - `downloadedUnix` / `replacedUnix` —— 这份字节是「下载」进本机的还是「替换」上去的
 *   （两个事件**至多一个有值**：同一份字节只有一种来路）。标签跟着事件走：
 *   有 `downloadedUnix` 就叫「下载时间」，有 `replacedUnix` 就叫「替换时间」。
 *   都没有 = 认不出出身的字节（这种字节不记账，不猜）—— 界面照实「未知」；
 * - `publishedAt` —— 这份字节属于哪一代目录、那一代**在云端发布**的时刻
 *   （RFC3339）。它跟着**这一版字节**走，云端以后怎么换代都不变；查不到 = `null`。
 */
export interface OnDiskFile {
  fileName: string
  downloadedUnix: number | null
  replacedUnix: number | null
  publishedAt: string | null
}

/**
 * catalog definition 里的**字段定义**（与 Rust `presetdata::ParamDef` 的 serde 形态对齐）。
 * 只声明消费面读的格子；JSON 里有更多字段（default_value / machine_variants …），
 * 见 `src-tauri/src/presetdata/registry.rs` —— 前端消费到哪一栏，声明就长到哪一栏。
 *
 * 2026-10-02（① deprecated 链路）：参数页的**字段清单改从 definition 取**（作者裁决：
 * 弃用字段「显示，但只读」）—— 所以这里补上渲染一行所需的显示格子（label / desc / unit /
 * uiComponent / valueType / choices / min / max / step）。这些 JSON 里本来就有，
 * 只是从前没声明。**这是 definition 通道**，与配方通道（`getMachineParams`，
 * 排除弃用字段）各司其职。
 */
export interface CatalogParamDef {
  key: string
  /** 数据域分区（= key 前缀）。**不是界面分组** —— 分组看 layout.sectionId */
  section: string
  /** 参数自己声明的界面归属（组内顺序 + 属于哪个分组） */
  layout: { order: number; sectionId: string }
  label: string
  desc: string
  unit?: string
  /** `number` / `switch` / `segmented` / `select` / `gcode` —— 控件形态的原词 */
  uiComponent: string
  valueType: 'float' | 'int' | 'bool' | 'string'
  /** 选项级弃用在 `Choice.deprecated` 上（那一档正在退场） */
  choices?: { value: string; label: string; deprecated?: boolean }[]
  min?: number
  max?: number
  step?: number
  deprecated?: boolean
  /** 只对这些机型生效；空数组 = 不限机型 */
  machineFilter?: string[]
  showWhen?: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }
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
/**
 * 资产登记（`presets/assets.toml` 的 definition 面，随 catalog 下发）。
 *
 * **客户端只认识 id，不认识文件**：机型写 `image = 'a1-image'`，这里给
 * `path = 'printers/a1.webp'`，界面拼 `/assets/<path>` 去取 ——
 * 改图/换图都在数据侧发生，TS 一行不动（2026-10-03 第二刀）。
 */
export interface CatalogAsset {
  /** 主键，**大小写不敏感**唯一 */
  id: string
  /** `'image' | 'icon' | 'model' | 'slicerProfile' | 'mkPreset'` */
  type: string
  /** 归属机型；不属于任何机型时不给这个字段 */
  machineId?: string
  /** 给人看的登记名 */
  name: string
  /** 相对资产根的路径。`mkPreset` 类留空（产物路径由命名规则算） */
  path: string
  /** 归属版本（`mkPreset` 类专有） */
  versionId?: string
  slicer?: string
  profile?: string
  /** `download` = 进交付集合按需下载；`bundled` = 随包不下载（构建期进客户端资源） */
  delivery: 'download' | 'bundled'
}

export interface RuntimeCatalog {
  catalogSchema: number
  /** 目录指纹：源或交付产物变了它就变 —— 将来「该不该同步」看它，不作完整性校验 */
  revision: string
  /**
   * **发布时刻**（RFC3339 / UTC）。发布侧落盘时写进目录（与 manifest 的 `updated`
   * 同一个戳），客户端云端表的「时间」列显示它 —— 显示时按本机时区转。
   *
   * **可选**：随包 bootstrap 目录刻意不带这一格（`gen-catalog` 的产物要逐字节可复现，
   * 而且"随包"没有发布事件）；旧版发布的目录也没有。缺了就是「没有可信的发布时刻」
   * —— 界面照实说「未知」，**不编**。
   */
  publishedAt?: string
  machines: RuntimeCatalogMachine[]
  /**
   * 品牌（含品牌图的**资产 id**）：机型与版本都没有图时回落到品牌图，再回落才是内置字标。
   * 2026-10-03 起品牌图正式进资产体系（`presets/brands.toml` 的 `logo` 引用资产 id）。
   *
   * 可选：盘上那份 catalog.json 可能还是旧版（那时 `assets` 也没有）。
   */
  brands?: RuntimeCatalogBrand[]
  files: RuntimeCatalogFile[]
  /**
   * 资产登记：界面按 id 查 path（见 [`CatalogAsset`]）。
   *
   * **可选**：客户端读的是盘上那份 catalog.json，它可能还是旧版本释放的
   * （那一版里没有这一栏）。缺了就是「图认不出文件在哪」⇒ 回落品牌 logo，不是错误。
   */
  assets?: CatalogAsset[]
  registry: CatalogRegistry
  /** 打印板（2026-10-02）：机型持引用（`Machine.plateIds`），几何住这里，按 id 查 */
  plates: Plate[]
}

/**
 * 使用中那一份**住在哪条线上**（第七层起）。
 *
 * ```text
 * official  官方线：目录（catalog）登记的交付文件（`mkp/…`）—— 只读，只有云端换版本能替换它
 * mine      用户线：用户自己那份（`presets-mine/…`）—— 用户可改，不属任何官方版本
 * ```
 *
 * **"只读"是文件归属的属性，不是"能不能被使用"的属性** —— 两条线都能成为使用中的那一份。
 */
export type ActiveOrigin = 'official' | 'mine'

/**
 * 使用中指针（新数据世界的第一个用户状态，全局唯一）。
 *
 * `intact` 是"盘上那份还是应用时刻的那份"。**两条线上它的意思不一样**：
 * 官方线正常恒 true（`mkp/` 是只读区）；用户线在用户自己又改了那份时是 false ——
 * 那是正常事（那份是他的），不是"这份配置坏了"。
 */
export interface ActivePreset {
  origin: ActiveOrigin
  fileName: string
  /** 用户线的落点（相对用户根）；官方线是 `null`（落点由目录给） */
  path: string | null
  sha256: string
  /** 认不出是哪台机型哪一版时是空串（用户那份没有血统、或目录里已经没有来源那份） */
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
  /**
   * 本客户端**读得懂这一代远端目录吗**（能力优先、版本兜底，见 `runtime::structure::can_read`）。
   *
   * `false` = 远端有这一代数据，但当前客户端不具备读它的能力 —— 此时**不该采用**它
   * （`applyRemoteUpdate` 会拒绝并抛 `NOT_SUPPORTED`）。用户要做的不是"再下一遍"，
   * 而是**去升级客户端**。★ 它是"能不能读"，不是"有没有新版本"（后者是 `upToDate`）。
   */
  readable: boolean
}

/**
 * **软件更新**状态（`release.json`）——与预设数据**完全两条链**。
 *
 * 它回答的是「有没有新版本的 SupportEase 这个软件」，与"预设数据能不能读"（`RemoteUpdateCheck`）
 * 是**两个不同的系统**，两个入口（设置页 / 预设页提示），**不混成一句话**。
 *
 * 信息源 = 仓库根的 `release.json`（`presets/` 外），**不属于** catalog / manifest。
 */
export interface SoftwareUpdate {
  /** 有没有比当前更新的正式版本 */
  hasUpdate: boolean
  /** 当前客户端版本（来自构建期，唯一真值） */
  currentVersion: string
  /** 最新正式版本（没有 release.json 时回落成 `currentVersion`） */
  latestVersion: string
  /** 更新说明；没有就是 `undefined` */
  notes?: string
  /** 去哪更新 / 看详情；没有就是 `undefined` */
  url?: string
  /**
   * **可下载的安装包**（`.app.zip`，2026-10-05 第五刀）。
   *
   * ★ `undefined` = 这一版**没给安装包** ⇒ 客户端退回"打开下载页"
   *   （那是 0.0.2 / 0.0.3 的行为，仍然成立）。**有它才摆"在应用内下载"按钮。**
   */
  asset?: ReleaseAsset
}

/** 发布出去的安装包（`release.json` 的可选 `asset` 格） */
export interface ReleaseAsset {
  name: string
  url: string
  /** 字节数；给了就照它收（对不上 = 这一份坏了，不装） */
  size: number
  /** 小写十六进制；空 = 不校验 */
  sha256: string
}

/**
 * 应用内更新的状态（`runtime::updater::UpdateState` 的线上形状）。
 *
 * **带 `state` 判别字段**（`tag` 序列化）—— 前端 `switch` 它，一处形状要认。
 */
export type UpdateState =
  | { state: 'idle' }
  | { state: 'downloading'; received: number; total: number }
  | { state: 'paused'; received: number; total: number }
  | { state: 'ready'; path: string; size: number }
  | { state: 'failed'; reason: string }
  | { state: 'cancelled' }

/** 上一次安装的结果（退出后那个脚本写的账，下次启动读） */
export interface UpdateResult {
  version: string
  ok: boolean
  reason: string
  at: string
}

/** 一次问全：状态 + 有无新版 + 资产 + 上次结果 */
export interface UpdateInfo {
  state: UpdateState
  hasUpdate: boolean
  currentVersion: string
  latestVersion: string
  notes?: string
  url?: string
  asset?: ReleaseAsset
  lastResult?: UpdateResult
}

/**
 * 一次下载的阶段。**只有这四个** —— 没有"校验中 / 落盘中"：
 * 那两步发生在管道内部，命令层拿不到它们的时机，报出来就成了编出来的进度。
 */
export type DownloadStage = 'connecting' | 'transferring' | 'done' | 'failed'

/**
 * 一次下载的水位。与 Rust 侧 `ipc::catalog::DownloadTick` 逐字段对齐
 * （两侧之间隔着 IPC，没有编译器，靠这里一份形状与那边的 serde rename 对上）。
 */
export interface DownloadTick {
  stage: DownloadStage
  fileName: string
  received: number
  /** 服务端没给长度时是 `null` —— 界面那时就别说百分比，说"已收多少字节" */
  total: number | null
  /** 只在 `failed` 上有值：失败原因，来自后端，前端不造句 */
  message?: string
}

/** 批量下载里每一份的结局。**一份出错不拖累别人**，所以按份返回，不是一次总的结果 */
export interface DownloadOutcome {
  fileName: string
  ok: boolean
  /** 失败原因同样来自后端 */
  message: string
}

/**
 * 一个**内置官方源**（作者 2026-10-05 拍"两个官方源"）。
 *
 * 界面摆两个固定单选就靠它 —— **用户没有输错地址的机会，这是防错的第一道**。
 */
export interface BuiltinPresetSource {
  /** `github` / `gitee` —— **id 是契约**（盘上存的就是它，别改） */
  id: string
  /** 给用户看的那一句（如「Gitee（国内直连）」） */
  label: string
  /** 这个源的 Bootstrap 地址（指向 `source.json`） */
  address: string
}

/**
 * 当前数据源（**用户选的是哪一个**，不是一个地址）
 *
 * - `mode`：`github` / `gitee` / `custom` —— 用户在三个里选，不手输；
 * - `address`：当前生效的入口地址（自定义时可能是"数据源根"，也可能是 `source.json`
 *   地址 —— **后端两种都认**，见 `runtime::source::CustomShape`）。
 */
export interface PresetSource {
  mode: string
  /** 模式名（给用户看的那一句） */
  label: string
  /** 当前生效的入口地址；空串 = 选了自定义但还没填 */
  address: string
  /** `true` = 用户自己填的（`custom`） */
  fromUser: boolean
  /** 两个内置源（没注入的那个不出现） */
  builtin: BuiltinPresetSource[]
  /** 出厂默认是哪一个（`github`）—— 「恢复默认」那一句要说清它是什么 */
  defaultMode: string
  /** 出厂默认那个源的地址 */
  builtinDefault: string | null
}

export interface MkpApi {
  /** 把校准好的三轴偏移写回配置。三轴一起写，不按页分 */
  saveOffsets(axes: Axes): Promise<void>

  /** 校准板清单 */
  getCalibModels(): Promise<CalibModel[]>

  /**
   * 让壳去打开一个模型文件（本地有缓存就直接开，没有就先下载）。
   * 前端不碰文件系统，也不关心它是下载还是命中缓存 —— 那是壳的事。
   */
  openModel(modelId: string): Promise<void>

  /* ——— 机型与文件（预设页 / 参数页要读的） ——— */

  /** 机型目录：品牌 → 机型 → 版本。客户端画三级选择用 */
  getMachines(): Promise<Machine[]>

  /**
   * 某个「机型:版本」下配了哪些文件。
   * `null` = 后端没有这个组合（不是空），与 `VersionFiles.files` 空数组是两件事。
   */
  getVersionFiles(machineId: string, versionId: string): Promise<VersionFiles | null>

  /** 本机预设目录里**已经有的**文件名（官方那一批，固定演示集合） */
  getLocalFiles(): Promise<string[]>

  /**
   * **用户自己的预设文件**（用户线，`<appDataDir>/user/presets-mine/`）。
   *
   * 盘就是底账（扫盘）：用户随时可能在 Finder 里改这个目录，所以没有账本可记。
   * 一份都没有 = 空数组，**不是错误**。
   *
   * 每一份都带上**血统**（如果它带着 `# based_on*` 三行）与它对应的判定
   * （[`BasedOn`]）—— 那是"官方换版了、你这份还是基于旧版"这件事的判据。
   */
  getUserPresetFiles(): Promise<UserPresetFile[]>

  /**
   * **用户改过的备注**整本（副标题覆盖账）。**只有本地表的副标题读它** ——
   * 云端表只读工作台写的那句（本地改动不许影响云端显示，2026-10-07 作者定）。
   * 键 = 文件身份（官方交付行是 `catalog.path`，用户线是相对用户根的路径）；
   * 有 ⇒ 本地副标题用它，没有 ⇒ 用那一版工作台写的 `remark`（都没写就空着）。
   * 「更新不覆盖；删了重新下载才回到工作台那句」——删除文件时后端把键一起清掉。
   */
  getPresetRemarks(): Promise<Record<string, string>>

  /**
   * **改一份预设的备注**（本地副标题覆盖账）。**用户写什么就是什么 —— 包括空串**
   * （2026-10-07 作者改口：「可以空着，不要回退」；空 = 副标题就空着）。
   * `null` = 恢复默认（删掉覆盖，退回落入「工作台写的 → 空着」）。
   * 只动这一本账 —— 文件字节与目录全程不碰。
   */
  setPresetRemark(key: string, remark: string | null): Promise<void>

  /**
   * **改一份用户预设的归属**（机型 / 版本）：把文件头的 `# machine:` / `# variant:`
   * 两行换成新值（正文一个字节不动）。机型必须是目录里登记的；**版本可自定义**
   * （2026-10-07 作者：「版本也不一定是选择，加一个自定义」—— 界面遇到不认识的
   * 版本就照原文显示）。
   */
  setUserPresetMachineVersion(
    path: string,
    machineId: string,
    versionId: string,
  ): Promise<void>

  /**
   * 读用户自己那份的正文。**只认 `presets-mine/`**（入参是 [`getUserPresetFiles`] 给的路径），
   * 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来。
   */
  readUserPresetText(path: string): Promise<string>

  /**
   * **开始改一份预设**：把正文复制进临时文件，**原件一动不动**。
   *
   * 两条线走同一个入口（`origin` 缺省 `official`，与 [`applyActivePreset`] 同一形状）：
   * 官方线认**文件名**（目录的键），用户线认**路径**（用户目录里可以自己分文件夹）。
   *
   * 前置都在后端拦：只改 MKP 预设（TOML）；官方线还要求盘上这份与目录逐字节一致
   * （第六层：旧版本 / 被改过的不许改）；用户线只要求盘上真有 —— **它不查 SHA**，
   * 用户那份本来就是允许改的。
   *
   * 已经有同一份的草稿 → **接着改**（`reused: true`），不覆盖用户的改动。
   */
  beginPresetEdit(fileName: string, origin?: ActiveOrigin, path?: string): Promise<PresetDraft>

  /** 把改动写进临时文件（界面边改边存）。**只动正文** —— 来源与那一刻的指纹不动 */
  putPresetDraft(text: string): Promise<void>

  /**
   * **按参数 key 改草稿里的一个值**（参数页底座：字段级写回）。
   *
   * 与 [`putPresetDraft`] 的分工：那条是"把界面上那一整份正文写进去"（编辑器逐字改的场景）；
   * 这条是"我只改这一个字段"（参数页用控件改值的场景）—— 后者**不碰**注释、键序、
   * 别的行，只把那一处换掉。两条都只动草稿正文，官方原件与下载区全程不碰。
   *
   * `paramKey` 是**注册表主键**（如 `toolhead.offset.x`），不是 TOML 字段名 ——
   * 定位与取值形态（数字 / 布尔 / 字符串）归后端按字段定义决定，前端不拼 TOML 字面量。
   */
  patchPresetDraft(paramKey: string, value: string): Promise<void>

  /** 放弃这次编辑：丢掉临时文件（幂等；官方原件与下载区全程没被碰过，所以它天生安全） */
  discardPresetDraft(): Promise<void>

  /**
   * **把这一份存进用户根**，然后丢掉草稿。存到哪由**这份草稿改的是哪一份**决定：
   * 官方线**另存**成 `presets-mine/<原名>（已修改）<后缀>`（原件全程不动）；
   * 用户线**写回它自己** —— 同一个路径、同一份文件，不产生第二份（第八层）。
   *
   * 官方线写下去的正文 = 草稿 + **文件头三行血统**（`# based_on` / `# based_on_release_time` /
   * `# based_on_sha256`）—— 于是这份文件**拷到哪台电脑上都说得清自己从哪来、基于哪一版**。
   * 那是"文件本身的信息"（随文件走），所以**不**另写进 `run/`（第七层作者定的原则）；
   * 用户线写回时那三行**照抄原来那三行**（出处没变）。
   *
   * 不碰官方原件、不碰下载区、也**不碰使用中指针**（生效走 [`applyActivePreset`]）。
   * 官方线再存一次就是覆盖它自己（`replaced` 说出来这次是不是盖掉了上一次那份）；
   * 用户线写的就是原来那一份所在的位置（`replaced` 恒为 true）。
   */
  commitPresetDraft(): Promise<CommittedDraft>

  /**
   * **重命名一份用户文件**（第十层）：只改名字，**字节一个不动** —— 内容、那三行血统、
   * TOML 都不重写；改完还是同一份 Preset。只换名字不换目录（`presets-mine/` 那一格内）；
   * 新名字不许带路径分隔符、不许空、**后缀保持原样**（改名不改它是哪一类）；
   * 落点已经有东西就拒绝（**不覆盖**）。
   *
   * 后端还会把两本状态账跟着改：**正指着它的使用中指针**（路径与文件名换成新的，指纹原样）
   * 与**这一份的草稿**（用户线认路径，「接着上次改」不接丢）。失败原话冒上来。
   */
  renameUserPreset(path: string, newName: string): Promise<UserFileIdentity>

  /**
   * **另存为一份新的**（第十一层）：把我自己那一份**按字节**复制成同一格里另一份新的用户文件。
   *
   * 与第八层"官方 → 我的文件"那条另存分开：这一层是**我的文件 → 我的文件** ——
   * 原文件一个字节不动；内容与那三行 `based_on*` 血统**原样带过去**（来源已经是用户文件，
   * 不重算血统 —— 重算会把"从哪一版官方派生"说错）。新名字过同一套门槛、落点已有东西就拒绝
   * （**不覆盖、也不自动改名** —— 名字由用户自己换）。
   *
   * **一个状态都不碰**：不改使用中指针、不迁移草稿、不建草稿、不进 archive ——
   * 新文件从诞生起就是独立的一份（之后能独立编辑 / 改名 / 删除 / 应用）。
   */
  copyUserPreset(path: string, newName: string): Promise<UserFileIdentity>

  /**
   * **把官方交付那份直接另存成你自己的一份**（官方 → 我的文件；UX 场景测试 A1 的正路）。
   *
   * 在此之前官方行的「另存为一份新的」是灰的，要绕「改这份」→ 保存才能复制 ——
   * 可"改了再保存"与"不改直接复制"落的是同一种东西，绕一道编辑流程不合直觉。
   *
   * 来源是**官方交付行**，闸在 Rust 侧（`mine::copy_release_as_new`）：
   * 目录里得有它、得是 MKP 预设、**字节必须与目录一致**（与「改这份」同一条边界 ——
   * 旧版本 / 内容异常禁令不变）。血统三行**新写指向**来源交付文件（官方原件没有
   * 血统头，不是照抄）；出处账不记（血统已经答了"从哪来"）。名字过同一套门槛、
   * 不覆盖、不自动改名；**一个状态都不碰**（不改使用中指针、不建草稿、不进 archive）。
   */
  copyReleaseAsNew(fileName: string, newName: string): Promise<UserFileIdentity>

  /* ---------- 第十二层：通用文件导入入口（Preset 只是第一个消费者）---------- */

  /**
   * **导入第一段：看落点**（拖拽与文件选择器都走这里）。只检查、不动盘；
   * 重名（`collision`）只如实说，**不自动改名** —— 名字由用户在界面上改。
   * `rejected` 带原因（现在只收 `.toml` 预设；ZIP / 备份包还没有认领它的导入器）。
   */
  stageImport(sources: string[]): Promise<StagedImport[]>

  /**
   * **导入第二段：真的复制进 `presets-mine/`**。逐份独立（一份出错不拖累别人）：
   * 源文件只读；**不覆盖**（`newName` 走改名那套名字门槛）；**内容按字节复制、不校验 TOML**
   * （能不能当 Preset 用是后面 Preset 语义入口的事 —— 导入不是"安装 Preset"；
   * 有血统三行原样带过去，没有也不编造）；**不碰任何状态**（不改使用中指针、
   * 不迁移 / 不建草稿、不进 archive）。
   */
  commitImport(items: ImportItem[]): Promise<ImportOutcome[]>

  /**
   * 打开系统文件选择器（多选）。用户取消 = 空数组（不是错误）。
   * 这是"通用入口"的一半：拖拽那一半住在 App 层（`FileImportProvider`）。
   */
  pickImportFiles(): Promise<string[]>

  /* ---------- 第十三层：文件外部管理 ---------- */

  /**
   * **在文件管理器里显示**：打开 Finder（Windows 上是文件资源管理器）并**选中**这份
   * 用户文件 —— 之后复制 / 压缩 / 发给别人 / 备份都随用户，**不经过 SupportEase 的业务逻辑**
   * （"文件外部管理"的含义就这一句；不另造一套"分享 / 导出"）。
   *
   * 只认「我的文件」（照旧过用户根那两道闸）；**读不出来的那份也能显示**
   * （文件管理同族：它只是一份文件，打开文件夹不吃内容）；**不改任何状态**
   * （使用中指针 / 草稿 / archive 一个都不碰）。文件被外面删了会如实报找不到。
   */
  revealInFolder(path: string): Promise<void>

  /**
   * **删除一份用户文件**（第十层）：**真删除** —— 没有垃圾桶、也没有归档
   * （`archive/` 是官方版本生命周期的一部分；用户自己删自己的文件就结束）。
   *
   * 两道硬闸在后端：**正在使用的不许删**（删了「使用中」就指向一份不存在的文件）、
   * **还有没保存的草稿的不许删**（删了草稿就永远存不回去）。失败原话冒上来。
   */
  deleteUserPreset(path: string): Promise<void>

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
   * 新数据世界的下载管道：把 catalog 里登记的一份从数据源拉进下载区 `mkp/`。
   * SHA 或大小对不上就整个拒绝——坏字节不落盘。
   *
   * `onTick` 可选：给了就看过程（一次调用一路事件），不给就是原来那个"点了等结果"。
   * 数据源地址不在 catalog 里，见 [`getPresetSource`]。
   */
  downloadCatalogFile(
    fileName: string,
    onTick?: (tick: DownloadTick) => void,
  ): Promise<void>

  /**
   * 批量下载（并发在 Rust 侧做）。**一份失败不拖累别人** —— 按份给结局，
   * 界面因此能说清楚"八份成了、两份因为什么没成"，而不是一句抹平的"下载失败"。
   */
  downloadCatalogFiles(
    fileNames: string[],
    onTick?: (tick: DownloadTick) => void,
  ): Promise<DownloadOutcome[]>

  /**
   * 读一份**已经下载**的交付文件的正文（第三圈起，资产也从这里读）。
   *
   * 前端不碰文件系统，所以要有这一条；只认 catalog 登记过的落点，
   * 没下载就是 NOT_FOUND —— 那时界面该说"还没下载"，而不是渲染一个空壳。
   */
  readDownloadedText(fileName: string): Promise<string>

  /**
   * 当前数据源。`null` = 还没配（既没填过、也没有出厂默认值）——
   * 这时下载与检查更新都会拒绝执行并说明去哪儿配。
   *
   * 读者是**设置页**（「高级设置 → 预设数据源」）—— 普通用户不需要来这里：
   * 官方地址由构建方注入（Bootstrap 那一刀），这一格留的是开发 / 排查的后门。
   */
  getPresetSource(): Promise<PresetSource | null>

  /**
   * 换数据源：**先探一次，通了才落盘**（作者 2026-10-05：用户容易输错地址）。
   *
   * - 内置两个源（`mode = 'github' | 'gitee'`）：地址是程序自带的，不联网不探；
   * - `mode = 'custom'`：**数据源根**与 **`source.json` 地址两种都认**（用户复制来的
   *   通常就是后者），取不到就整次拒绝 —— 错地址留在设置里比"没配"更难查。
   */
  setPresetSource(mode: string, customUrl?: string | null): Promise<PresetSource>

  /**
   * 撤掉用户覆盖（回到内置默认 / 没配）：删掉这台机器上的那份设置，幂等。
   *
   * **"回到内置"只能靠删** —— [`setPresetSource`] 拒空地址（写空 = 第三种状态）；
   * 返回撤完之后生效的值（有内置给内置，没有就是 `null`），界面直接换账。
   */
  clearPresetSource(): Promise<PresetSource | null>

  /**
   * 已经下载到下载区的文件。盘就是底账：文件在且 SHA 对得上才算数，不查缓存。
   *
   * 时间三格全部来自**事件**（见 [`OnDiskFile`]）：`downloadedUnix`（下载事件）、
   * `replacedUnix`（替换事件）、`publishedAt`（这一版字节在云端发布的时刻）。
   */
  getDownloadedFiles(): Promise<OnDiskFile[]>

  /**
   * 有更新的文件：盘上在、但字节与目录不一致（目录更新带来新版本，或文件被动过）。
   * "更新"就是对这些再跑一遍 downloadCatalogFile——旧份自动归档。
   *
   * 时间口径与 [`getDownloadedFiles`] 相同（事件账）。
   *
   * **它只说"不一致"，不说"因为什么"** —— 分成哪两种（旧版本 / 查不出它是哪一版）
   * 看 [`getDeliveryTrust`]。
   */
  getStaleFiles(): Promise<OnDiskFile[]>

  /**
   * 盘上这几份交付预设**认得出是哪一版吗**（第三圈第 6 层）。**只列有事的**
   * （没下载 / 与目录一致的两种不出现）。
   *
   * 与 [`getStaleFiles`] 是同一个事实的两层：那一条说"与目录不一致"，这一条说
   * "那不一致的字节是不是我们发过的某一版"。**它与"云端有没有更新"也是两件事**。
   */
  getDeliveryTrust(): Promise<DeliveryTrust[]>

  /**
   * 归档区里有什么：官方文件换版本时**被换下来的那些旧版本**。
   *
   * 目录里已经没有的那几份，`machineId / versionId / kind` 如实给 `null`。
   * 删除见 [`deleteArchivedFile`]（作者裁决 2026-10-06：允许删）。
   */
  getArchivedFiles(): Promise<ArchivedFile[]>

  /**
   * 读归档区里某一份旧版本的正文（旧版 TOML）。
   * **只认归档区**：入参是 [`getArchivedFiles`] 给的那个相对路径；不是 UTF-8 就如实报错。
   */
  readArchivedText(path: string): Promise<string>

  /**
   * **删除本机下载区里那份官方交付文件**（作者裁决 2026-10-06：一切皆可删）。
   *
   * 删了它回到「未下载」，随时可以从云端重新下载（字节有目录 SHA 锚定，零数据损失）。
   * 它正在被使用 / 还有没保存的草稿：后端把使用中指针一并撤下、草稿一并丢弃
   * —— 确认框必须讲清这一步。幂等：本来就不在也照实成功。
   */
  deleteDeliveryFile(fileName: string): Promise<void>

  /**
   * **删除归档区里的一份旧版本**（作者裁决 2026-10-06：允许删）。
   *
   * 入参是 [`getArchivedFiles`] 给的那个相对路径。**删了就找不回**
   * （云端只有最新版）—— 确认框必须讲清这个代价。
   * 版本链与事件账不动：那是历史事实，不是这份文件的附属。
   */
  deleteArchivedFile(path: string): Promise<void>

  /**
   * 当前使用的是哪一份（全局唯一）。null = 还没用任何一份，是合法状态不是错误。
   */
  getActivePreset(): Promise<ActivePreset | null>

  /**
   * 「使用这一份」。**两条线共用这一个入口**（第七层）：官方交付文件与用户自己那份
   * 都是真的 Preset —— "只读"是文件归属的属性，不是"能不能被使用"的属性。
   *
   * ```text
   * official  目录里有这一份 + 盘上字节与目录登记的当前版本逐字节一致
   *           （没下载 / 被改过 / 是旧版本 —— 都 reject，不会应用半份）
   * mine      落点必须在 presets-mine/ 那一格里 + 盘上真有这一份 + 是一份 TOML 预设
   * ```
   *
   * `origin` 缺省是 `official`（老调用点不用改）；`path` 只有用户线要给
   * （用户目录里可以自己分文件夹，所以认的是路径，不是文件名）。
   */
    applyActivePreset(
      fileName: string,
      origin?: ActiveOrigin,
      path?: string,
    ): Promise<ActivePreset>

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
   *
   * ★ 若远端这一代**读不懂**（`RemoteUpdateCheck.readable === false`），本命令
   * **不落盘、不归档、不采用**，返回 `NOT_SUPPORTED` —— "下载前拦"的落点。
   */
  applyRemoteUpdate(): Promise<void>

  /**
   * 当前客户端版本号（来自构建期 `CARGO_PKG_VERSION`，唯一真值）。
   *
   * 设置页「软件更新」块要显示"当前版本 → 最新版本"，这是它拿"当前版本"的唯一口子。
   */
  getAppVersion(): Promise<string>

  /**
   * 检查**软件**更新（`release.json`）—— 与预设数据链**完全分开**。
   *
   * 只读、不在启动/首屏路径上（设置页打开时才调；铁律：云端不参与首屏）。
   * 信息源不可达时**如实拒绝**（`IO` / `NOT_FOUND`），由调用方决定说还是略过。
   */
  checkSoftwareUpdate(): Promise<SoftwareUpdate>

  /* ---------- 应用内更新（第五刀） ---------- */

  /**
   * 一次问全（状态 + 有无新版 + 资产 + 上次安装结果）。
   *
   * **不在首屏路径**：标题栏那枚图标在**用户开始下载之后**才订阅事件，
   * 平时只在切到设置页时问一次。
   */
  updateInfo(): Promise<UpdateInfo>

  /** 开始下载安装包（**秒回**；进度靠 `software-update-progress` 事件） */
  startUpdate(): Promise<void>
  pauseUpdate(): Promise<void>
  resumeUpdate(): Promise<void>
  cancelUpdate(): Promise<void>
  /**
   * 装上去并重启：**本进程会退出**（正在运行的 `.app` 换不掉）。
   * 替换成没成，**下次启动**由 `updateInfo().lastResult` 说清。
   */
  installUpdate(): Promise<void>

  /**
   * 在系统默认程序里打开一个 **http(s)** 链接。
   *
   * ★ 前端**不要**用 `<a target="_blank">`：Tauri 的 webview 没开 opener 权限，
   *   点了**什么都不发生**（0.0.2 的「查看更新」就是那样"点了没反应"的）。
   */
  openUrl(url: string): Promise<void>
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
   * 客户端**读不懂**这一代数据：文件没坏、也不是下载失败，是能力不足。
   *
   * 与 `CORRUPTED` / `INTERNAL` 严格分开：那两个是"数据/程序出了问题"，这一档是
   * "这份数据是新结构，当前客户端还不具备读它的能力" —— 用户要做的是**去升级客户端**。
   */
  | 'NOT_SUPPORTED'

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
