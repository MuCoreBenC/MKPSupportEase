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

/**
 * 一根轴的取值范围 —— **注册表口径**（`RecipeParam.min/max` 原样透传）。
 *
 * 校准页（读数输入框 / Z 板与 XY 板的点选增量）与参数页吃同一套限制：
 * `toolhead.offset.z` 注册表写的是 `[0, 20]`，所以校准里 z 也不许为负 ——
 * 点了负刻度 / 手输负数都夹回边界（`undefined` = 该方向不限）。
 */
export interface AxisLimit {
  min?: number
  max?: number
}

/** 三轴各自的取值范围（哪一轴没在注册表里登记就整项不设限） */
export interface AxisLimits {
  x: AxisLimit
  y: AxisLimit
  z: AxisLimit
}

/** 一份预设 = 某机型某打印件版本对应的那个 toml，外加它带来的偏移基准 */
/**
 * 一份预设的用量（`getPresetUsage` 的一行）。
 *
 * `count` = 被"应用"过几次；`lastUnix` 是最近一次（UTC epoch 秒），现在只留作以后
 * "最近用过"的排序尺，界面暂时不用。
 */
export interface PresetUsageEntry {
  /** `local:<用户根相对路径>` / `cloud:<内部根相对路径>` */
  key: string
  count: number
  lastUnix: number
}

export interface Preset {
  /**
   * **实际会用的那一份**的文件名（有「我的一份」时就是它）。
   *
   * 用它回答"我现在改/校准的是哪份文件"（校准页 pill、首页卡片的「正在使用」）。
   * 但**它不该回答"这个套餐配的是哪份预设"** —— 那件事看 `bundleName`。
   */
  name: string
  /**
   * **套餐（云端交付）里那份 MKP 预设**的文件名（2026-10-17 加）。
   *
   * 选机型页那一块「预设文件」显示的是它：第三级选的就是套餐，用户要看的就是
   * "这个套餐配的预设叫什么"，而不是他自己复制出来的那一份
   * （作者：「他为什么显示复制出来的，而不是显示云端下载的、套餐里面的」）。
   * 与 `name` 分开存，是为了让两句话各说各的 —— `name` 说"在用哪份"，它说"套餐是哪份"。
   */
  bundleName: string
  /** 套餐里那份的落点（与 `bundleName` 成对）—— 只当悬停提示用 */
  bundlePath: string
  /**
   * **落点**（用户线是相对用户根的 `presets-local/…`，云端线是相对内部根的
   * `delivery/mkp/presets/…`）—— 读正文、保存校准都把它交回后端，所以它**不是**
   * 本机绝对路径：后端只收相对落点（绝对路径一律被那道防穿越的闸拒掉）。
   * 界面上只当悬停提示与去重键用。
   */
  path: string
  axes: Axes
  /** 三轴的注册表范围（校准页消费；与参数页同一套 min/max，见 `AxisLimits`） */
  axisLimits: AxisLimits
  speed: number
  /**
   * **「我那一份」的落点** —— 校准保存写它（2026-10-08 资源库改判：偏移随「我的预设」走）。
   * `null` = 这个机型 / 版本还没有你的一份：校准页照旧显示云端默认值，**保存被拦**
   * （先另存一份自己的才有得存 —— 云端那一份是模板，永远不可变）。
   */
  local: { path: string; fileName: string } | null
}

/** 校准板模型（Z 板 / XY 板 / 支撑测试件） */
export interface CalibModel {
  id: string
  name: string
  desc: string
  /** 交付文件名（catalog 登记的那个）—— 「取回」走下载管道、「打开」找盘上那份，都认它 */
  fileName: string
  /** 本机那份的真实大小（'3.1 MB'）。`null` = 本机还没有这份文件 —— 不编假大小 */
  size: string | null
  /** 本机有没有这份文件（下载区或旧缓存里找得到），以磁盘为唯一权威 */
  ready: boolean
}

/* ---------- 后处理执行报告（报告页） ---------- */

/** 输入 / 输出文件的事实（大小、行数、SHA） */
export interface ReportFileFact {
  path?: string
  sizeBytes?: number
  sha256?: string
  lines?: number
}

/** 打印时间估算（`detail.printTime`） */
export interface ReportPrintTime {
  totalSeconds?: number
  toolMovementSeconds?: number
  prepOverheadSeconds?: number
  startupOverheadSeconds?: number
  segments?: number
  byType?: Record<string, { seconds?: number; count?: number; percent?: number }>
}

/** 管线一步的计时（`pipeline[]`） */
export interface ReportPipelineStep {
  id?: string
  elapsedMs?: number
  message?: string
  status?: string
  /**
   * 这一步自报的多行事实，`\n` 分行 —— 步骤名下面铺小字，排查时最值钱的一段。
   *
   * **只有老记录才有**（那时记录里存的就是句子）。新记录用 `detailLines`。
   */
  detail?: string
  /**
   * 同样的几行事实，按**字典键 + 原始参数**存（2026-10-26）。
   *
   * 记录里存事实不存句子：同一份 `_meta.json` 中英两界面各读各的。缺 = 老记录，
   * 回落 `detail`（当年那句中文，**不会复译**）。
   */
  detailLines?: { key: string; args?: Record<string, string> }[]
}

/** 一个轴对齐框（mm，机器坐标） */
export interface ReportBox4 {
  xMin?: number
  xMax?: number
  yMin?: number
  yMax?: number
}

/**
 * **模型俯视几何**（2026-10-27，`_meta.json` 顶层 `modelMap`）。
 *
 * 作者：「我希望把它模型的包围盒也显示出来，就是后处理的时候碰撞检测的那个显示的一样的，
 * 复用的」。所以这里**只有几何、没有塔、没有冲突** —— 塔由报告页按 IR 补，冲突是另一格。
 * 三档字段与 `ReportCollision` 里那三个同名键逐字同义（同一个内核出口算出来）：
 * 报告页擦料塔那一栏的俯视图拿它画"模型在板上占哪一块"。
 *
 * `modelBbox` 不在这里：画图只用 `modelBoxes` / `modelPolygons`（一个模型一块），
 * 整体框是给"挪模型"那条建议算方向的。
 */
export interface ReportModelMap {
  /** 可打印区（映射基准；与碰撞那一趟同一口径：gcode 的 `; printable_area` 优先） */
  printable?: ReportBox4
  /** 一个模型一个框（内核按 2mm 占用格连通块切） */
  modelBoxes?: ReportBox4[]
  /** 一个模型一块轮廓（凸包，≤24 边，机器坐标）—— 画的是它 */
  modelPolygons?: [number, number][][]
  /** 机型禁区（通常没有） */
  forbiddenZones?: [number, number][][]
}

/**
 * **碰撞那一趟的事实**（2026-10-18）。
 *
 * 内核算完检测就把这些一起带出来（旧版有"移动建议"，这一稿把它落进执行记录）：
 * 撞的是谁、建议挪谁、往哪挪多少、冲突点在哪、画俯视图要的几何。
 *
 * 建议那句（`adviceText`）是**内核按 `messages.toml` 渲染好的**，码在 `adviceCode` ——
 * 界面只管摆，不自己拼句子。
 */
export interface ReportCollision {
  /** `model` / `prime-tower` / `both` / `forbidden` / `wipe-tower-needed` */
  kind?: string
  /** 建议挪谁：`model` / `wipe-tower` */
  target?: string
  /** 方向（中文；算不出方向是空串 —— 那时看 `target` 走"挪塔"那句） */
  direction?: string
  requiredMoveX?: number
  requiredMoveY?: number
  conflictCount?: number
  /** 冲突点采样（mm，机器坐标；内核封顶 200 个，够画出分布） */
  conflictPoints?: [number, number][]
  /** **判据用的塔包围盒**（含外围结构扩展；左下角 = wiper_x / wiper_y）。
   *  ★ 键名是 `Bbox` 不是 `BBox`：Rust 侧 `tower_bbox` 走 camelCase 就长这样 ——
   *  第一稿写成 `towerBBox`，大小写对不上 ⇒ 俯视图整块退掉（2026-10-18 真机踩的）。 */
  towerBbox?: ReportBox4
  /** 模型挤出点的整体包围盒（禁区那一档没有 ⇒ 全 0）。**只给"挪模型"建议方向用**——
   *  画图请用下面的 `modelBoxes`（每个模型各一个框）。 */
  modelBbox?: ReportBox4
  /**
   * **每个模型各自的包围盒**（2026-10-18 作者：「做出来，分开的独立的包围盒」）。
   *
   * 内核按占用格的连通块切（`Occupancy::boxes`），两个模型中间隔着空 ⇒ 两个框。
   * 旧记录 / 没算这一档 ⇒ `undefined` 或空数组，界面退回 `modelBbox` 那一个框。
   */
  modelBoxes?: ReportBox4[]
  /**
   * **每个模型各自的轮廓**（凸包，≤24 边，机器坐标）。俯视图画的是它 ——
   * 矩形四个角会把模型旁边的空算进去，看着像早就撞上了。
   * 空 / 缺这一格（旧记录、稀疏块）⇒ 界面退回 `modelBoxes` 的矩形。
   */
  modelPolygons?: [number, number][][]
  /** 可打印区 */
  printable?: ReportBox4
  /** 画塔真实形状要的参数（与参数页同一套算法） */
  tower?: {
    outerStructure?: string
    bodyMm?: number
    ribWidth?: number
    ribExtraLength?: number
    ribFilletWall?: boolean
    sheathBaseExpand?: number
  } | null
  forbiddenZones?: [number, number][][]
  adviceCode?: string
  adviceText?: string
  /** **建议挪动的箭头**（俯视图画"往哪边、挪多少"）：起点 = 挪谁的中心（机器坐标）、
   *  位移 = **带符号** mm；位移为零 = 没有方向可画（挪不开那一档）。
   *  旧记录没有这两格 ⇒ 可缺（缺就不画箭头）。 */
  arrowFrom?: [number, number]
  arrowVec?: [number, number]
}

/** 涂胶组（`detail.glue`）—— 键名与旧版 `_meta.json` 的 `detail` 同名 */
export interface ReportGlueFacts {
  glueTotalLength?: number
  /** 换笔次数（旧版的叫法；内核里是 GlueCutterChanges） */
  gluePenSwitches?: number
  gluePassCount?: number
  /** mm/s */
  glueSpeed?: number
  penOffsetX?: number
  penOffsetY?: number
  penOffsetZ?: number
  glueZLiftHeight?: number
  glueZLiftHeightFirstLayers?: number
  glueInterModelLiftHeight?: number
  glueZOffset?: number
  primeTriggerLayers?: number
  primeTotalLength?: number
}

/** 擦料塔组（`detail.tower`） */
export interface ReportTowerFacts {
  towerLayers?: number
  towerHeight?: number
  towerExtrudeRatio?: number
  towerZLift?: number
  towerSafeZOffset?: number
  towerTravelSpeed?: number
  fastTowerMode?: string
}

/** 切片工艺组（`detail.slicer`） */
export interface ReportSlicerFacts {
  slicerVersion?: string
  machinePresetName?: string
  processPresetName?: string
  filamentType?: string
  nozzleDiameter?: number
  slicerIroningEnabled?: boolean
  ironingMode?: string
  ironingThreshold?: number
  treeSupport?: boolean
  useDiskWipe?: boolean
}

/**
 * 校准件身份证据链（`_meta.json` 的 `detail.calibration`）。
 *
 * 一份记录能独立回答「当时看的是哪个模型、两边各写了什么、怎么判的、最终用了哪一档」——
 * 用户把原始 3MF 删了也解释得清。
 */
export interface ReportCalibrationFacts {
  /** `origin.txt` 的原值（可能是 `.stl`，也可能为空） */
  originPath: string | null
  /** 仅当扩展名是 `.3mf` 时给出 —— 只有它才可能是身份来源 */
  model3mf: string | null
  /** 该路径**当时**是否真实存在（现在可能已被移走） */
  exists: boolean
  /** 3MF `MKPSE:Calibration` 的**原值**（未归一，可能是旧写法） */
  identity3mf: string | null
  /**
   * `from_3mf`（3MF 里有身份）/ `absent`（没有或读不到，原因见 `note`）。
   *
   * 只有这两种 —— 身份的唯一来源是 3MF 里那条 `MKPSE:Calibration`，
   * G-code 那一路载体已整条删除。
   */
  verdict: string
  /** 读不到的原因（人可读一条）；不是错误，只是解释 */
  note: string | null
  /** 实测挤出包围盒 `[xMin, xMax, yMin, yMax]`（mm）；经典分支拿不到 */
  measured: [number, number, number, number] | null
  /** 命中的档 id（`calibration_models.toml` 的 `[[models]] id`） */
  profileId: string | null
  /** 该档的尺寸容差（mm） */
  tolerance: number | null
  /** 该档声明的标称外形 `[宽, 高]` */
  footprint: [number, number] | null
  /** 最终采用的校准身份（`ZOffset` / `Precise` / `Rough` / `Repetition`） */
  mode: string | null
}

/** 一次执行的摘要（报告页列表行）。字段与 `_meta.json`（schema v2）一一对应 */
export interface ReportSummary {
  /** 记录 id = `_meta.json` 文件名去掉后缀，全树唯一 */
  id: string
  /** 哪一天的账（日期目录名） */
  day: string
  startedAt: string | null
  finishedAt: string | null
  elapsedMs: number | null
  presetName: string | null
  machineType: string | null
  /** 输入 G-code 的文件名 */
  gcodeName: string | null
  /**
   * **3MF 模型名**（`帕姆.3mf` → `帕姆`）—— 钩子在 G-code 还在切片器临时目录里时，
   * 从旁边的 `origin.txt` / 同名 zip / `_temp_1.config` 三级兜底取的（`_meta.json`
   * 顶层 `modelName`）。三级都没拿到、或老记录没有这一格 ⇒ `null`。
   *
   * 页面拿它拼「结论 → 文件」那一行：`帕姆（.63288.2.gcode）`；
   * `null` 时就只显示 G-code 名 —— **不编名字**。
   */
  modelName: string | null
  /** `false` 有两种：跑失败（有 error）与记录本身坏了 —— 都进列表，不许藏 */
  ok: boolean
  error: string | null
  warningCount: number
}

/**
 * **这一趟的参数快照**（2026-10-21，作者：「我当时切片的时候的 TOML 和 IR 识别到的
 * 对照一下看有没有错误的」）。
 *
 * 为什么必须落在记录里：用户切完往往接着改预设（真机就是这样把擦料塔从 20 改到 101 的），
 * 回头再读那份 TOML 已经读不到"当时那一盘用的是哪个值"了。
 */
export interface ReportSnapshot {
  /** 预设 TOML 里读到的全量参数（注册表 key → 控件看得懂的字符串），顺序稳定 */
  params: Record<string, string>
  /** **IR 终态原样**（内核 `Ir` 的 JSON，PascalCase 键）—— "程序实际认成了什么" */
  ir: Record<string, unknown>
  /**
   * **这一份预设里读不出来的参数**（中文名，相对它最初的原文镜像比）。
   *
   * 空 = 一项不缺。非空时报告页要说清"这一盘有 N 项用了默认值"—— 缺项不报错，
   * 但结果一定不是用户想的那样（2026-10-21 作者：「报告页面能看到这些对应的错误」）。
   * 旧记录没有这个键 ⇒ `undefined` ⇒ 那一行不摆。
   */
  missing?: string[]
}

/** 一条执行记录的详情（展开那一屏） */
export interface ReportDetail {
  summary: ReportSummary
  inputFile: ReportFileFact | null
  outputFile: ReportFileFact | null
  /** 刮出来的切片器头部事实（机型 / 工艺预设名、熨平开关、3mf 名） */
  header: Record<string, unknown> | null
  /** 16 项统计（塔高 / 涂胶层 / 层数……）—— 键与 `_meta.json` 的 `detail.stats` 同名 */
  stats: Record<string, unknown> | null
  printTime: ReportPrintTime | null
  /**
   * 打印时间估算的状态：`computing` / `ready` / `failed`。
   *
   * 打印时间是**延后**算的（钩子写完盘就退，交给常驻进程），所以刚生成的记录
   * 可能还没有 `printTime` —— 这时看这个状态显示"正在估算…"，**不许拿 0 凑数**。
   */
  printTimeStatus: string | null
  /**
   * 估算出的打印秒数（`detail.estimatedPrintSeconds`）—— 「比切片器预估多多久」
   * 拿它与 `stats.originalPrintTime` 里解析出的秒数比。旧记录没有这一格。
   */
  estimatedPrintSeconds?: number | null
  /** 涂胶组（旧记录 / 失败记录没有 ⇒ `null`，整组不摆） */
  glue?: ReportGlueFacts | null
  /** 擦料塔组 */
  tower?: ReportTowerFacts | null
  /** 切片工艺组 */
  slicer?: ReportSlicerFacts | null
  /**
   * **校准件身份证据链**（2026-10-18，`_meta.json` 的 `detail.calibration`）。
   *
   * 作者原话：「每次处理拿到了什么、从哪里拿到、怎么判断、最终用了什么，都留下记录。」
   * 普通模型那一趟后端**不写这一格**；旧记录也没有 ⇒ `null` ⇒ 整块不摆。
   * 判空一律 `!= null`（Rust 的 `Option` 线上是显式 `null`）。
   */
  calibration?: ReportCalibrationFacts | null
  /**
   * **参数快照**（2026-10-21，`_meta.json` 的 `detail.snapshot`）。
   *
   * 切片那一刻的两份原文：预设 TOML 里读到的全量参数 + IR 终态。
   * 旧记录、失败 / 取消记录都没有这一格 ⇒ `null` ⇒ 整组不摆（不编）。
   */
  snapshot?: ReportSnapshot | null
  pipeline: ReportPipelineStep[]
  /**
   * **碰撞那一趟的事实**（2026-10-18，`_meta.json` 顶层 `collision`）。
   *
   * 只有 `E_GCODE_COLLISION_001` 那一趟会写：撞在哪、建议往哪挪多少、画俯视图的几何。
   * 别的记录（含成功记录与旧记录）一律 `null` ⇒ 报告页那两块整组不摆。
   */
  collision: ReportCollision | null
  /**
   * **模型俯视几何**（2026-10-27，`_meta.json` 顶层 `modelMap`）。
   *
   * 报告页擦料塔那一栏的俯视图拿它画"模型在板上占哪一块" —— 与碰撞那一屏同一份
   * `CollisionMap`、同一套几何（内核 `model_map_facts` 一个出口算出来）。
   *
   * 为什么单独一格：成功那趟 `collision` 恒为 `null`（撞了走不到成功出口），而要看
   * 的恰恰是成功那趟。旧记录 / 那趟扫不出模型点 ⇒ `null` ⇒ 图只画塔。
   */
  modelMap?: ReportModelMap | null
  /**
   * **挪塔记录**（2026-10-18，`_meta.json` 顶层 `relocation`）。
   *
   * 走到了"自动挪塔 / 自己挪模型"那一问才有；别的记录与旧记录一律 `null` ⇒ 整块不摆。
   * 与 `collision` 同一套判空口径（`!= null`）。
   */
  relocation?: RelocationRecord | null
  warnings: string[]
  /** 调用现场（exe + argv）—— 排查「用的哪份预设跑的」看它 */
  invocation: { exe?: string; args?: string[] } | null
  presetPath: string | null
  gcodePath: string | null
}

/**
 * 导出三件套里一份的**机器名**。
 *
 * 中文说明（「原始文件（改动前的备份）」那三句）住界面这一侧 —— 后端不写界面的话，
 * 与 `ErrorCode` 那条纪律同一条。
 */
export type ReportExportKind = 'original' | 'processed' | 'meta'

/** 导出计划里的一份文件（要落到用户目录里的那一份） */
export interface ReportExportFile {
  kind: ReportExportKind
  /** 落地时的文件名（`<基名>_original.gcode` 这种，**完整不省略** —— 框里要照实显示） */
  name: string
}

/**
 * 「导出执行报告」的计划：三件套叫什么、默认落到哪、哪几份归档里没有。
 *
 * 界面拿它画那个导出框（逐行文件名 + 路径胶囊），确认时才真写盘
 * （[`MkpApi.exportReportArchive`]）—— 两侧的基名由 Rust 侧同一个纯函数算，不会漂。
 */
export interface ReportExportPlan {
  /** 基名：`<模型名 或 归档自己的名字>_<切片时刻>`（`帕姆_20261011142431`） */
  base: string
  /**
   * 默认落点的**父目录**（`<用户根>/exports`）。界面显示时自己拼上 `<基名>/`，
   * 导出时交回去的也是这个父目录 —— 两处口径一致。
   */
  defaultDir: string
  /** 真在盘上、会跟着导出的那几份（顺序固定：原始 / 处理后 / 元数据） */
  files: ReportExportFile[]
  /** 归档里没有的那几份 —— 失败的那一次**不会有**「处理后的文件」，照实说，不假装齐 */
  missing: ReportExportKind[]
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
  /** 英文显示名（2026-10-26 双语注册表）。缺省 = 回落 `label` */
  labelEn?: string
  /** 一句话说明，鼠标悬停时显示。注册表本来就有这份数据 */
  desc: string
  /** 英文说明。缺省 = 回落 `desc` */
  descEn?: string
  /** 分组名，来自注册表的 section —— 界面不自己造分类 */
  group: string
  /** 英文分组名。缺省 = 回落 `group` */
  groupEn?: string
  unit?: string
  control: 'number' | 'switch' | 'choice' | 'text'
  /**
   * control === 'choice' 时的可选项。
   *
   * `deprecated`（选项级弃用，2026-10-02）：上游把某一档标成「正在退场」。**不是**字段级弃用
   * —— 参数本身还是活的，只是这一个取值不许再被写成新值（老文件里存着的照旧读得到、能还原）。
   * 客户端把它映射到共用的 `FieldOption.deprecated`（划线），不自己造第二个弃用信号。
   */
  choices?: {
    value: string
    label: string
    /** 英文显示名。缺省 = 回落 `label` */
    labelEn?: string
    deprecated?: boolean
    /** 只在这些机型上弃用（与 `deprecated` 叠加） */
    deprecatedMachines?: string[]
  }[]
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
  /** 英文显示名（2026-10-27 双语数据）。空 = 回落 `name` */
  nameEn?: string | null
  /** 角标，'推荐' / '热门' / '最新'；没有就不显示 */
  tag?: string
  /** 英文角标。空 = 回落 `tag` */
  tagEn?: string | null
  description?: string
  /** 英文说明。空 = 回落 `description` */
  descriptionEn?: string | null
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
  /** 英文名（空 = 回落 `label`） */
  labelEn?: string
  desc?: string
  /** 英文说明（空 = 回落 `desc`） */
  descEn?: string
  /** 这一组里有几条。**已按 machineFilter 与废弃过滤**，所以是这台机型真实看得到的条数 */
  count: number
}

export interface ParamTab {
  /** 'offset' | 'wiping' | 'fan' | 'glue' | 'gcode' | 'advanced' */
  id: string
  /** '偏移' | '擦料' | '风扇' | '涂胶' | '切换' | '更多' */
  label: string
  /** 英文名（空 = 回落 `label`） */
  labelEn?: string
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
  /** 只在这些机型上弃用（与 `deprecated` 叠加）；不限机型时不给这个字段 */
  deprecatedMachines?: string[]
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
 * 与 `PresetFileInfo`（云端文件）是两种东西，不要混 —— 这是总纲 §1③「预设 TOML 的一生」
 * 在类型上的体现：
 *
 * ```text
 * 云端线                               用户线
 *   云端 → mkp/ → archive/              presets-local/（另存出来的那一份）
 *   有 SHA、属于版本、不可变            云端没有它：没有 SHA、不属于任何版本、不参与套餐
 * ```
 *
 * 用户那份**从云端另存出来之后与云端脱钩**，可以自由改，**永远不回写云端原件**；
 * 反过来云端换版本也不会动它。
 *
 * **形状就是盘上的事实**：路径 / 文件名 / 大小 / 改动时刻 / 认得出的类别。
 * 没有「用户自己标的适用机型」这种字段 —— 今天没有任何地方能让用户去标它，
 * 留一个永远空的字段就是在编形状（需要它的那一步再加）。
 */
/**
 * 用户自己那份**基于的云端那一版**现在怎么样了（第七层）。
 *
 * 五档只回答一个问题：**它当初基于的那一版，和目录里现在这一版是什么关系** ——
 * 不回答"这份用户文件好不好"（用户自己那份从来不是坏文件）。
 *
 * 判定顺序（2026-10-27 定，后端唯一出口 `runtime::local::based_on`）：
 * 先认「是不是同一个源」，再比内容，最后才谈谁更新。换源之后 SHA 不同**不再**
 * 自动等于「有新版」—— 那会把用户从新源往旧源上带。
 *
 * - `current`：同一个源，内容与云端那份一样
 * - `outdated`：同一个源，云端换版了（这份从旧版派生）
 * - `newer`：同一个源，但**云端那份更旧** —— 只显示「云端较旧」，不给任何动作
 * - `elsewhere`：**不是同一个源**（换平台 / 换分支 / 换目录，或旧副本没记源身份）
 * - `unknown`：说不清（没有血统、来源已不在目录里、摘要缺失）
 */
export type BasedOn = 'current' | 'outdated' | 'newer' | 'elsewhere' | 'unknown'

/**
 * 「基于哪一版」这个结论**摆事实**用的那几格（全可空）。
 *
 * 界面只负责把这几行原样摆出来 —— 判定全在 Rust 侧（`runtime::local::based_on`），
 * 前端不再自己拼第二套判据。
 */
export interface BasedOnFacts {
  /** 这份副本建的时候哪个源在生效（`github#owner/repo@ref`）。没记账 ⇒ `null` */
  lineageSource: string | null
  /** 血统里记的来源版本标记（文件头 `# release_time`，可能是版本号不是时刻） */
  lineageRelease: string | null
  /** 那个源最近一次发布目录的时刻 */
  lineagePublishedAt: string | null
  /** 当前生效的源身份 */
  cloudSource: string | null
  /** 当前这份目录的发布时刻（随包 bootstrap 目录没有） */
  cloudPublishedAt: string | null
}

/**
 * 用户文件的**文件级**状态（第九层）：`ok` = 能读 + 是 UTF-8 + TOML 语法能解析；
 * `unreadable` = 读不出来（编码 / TOML 语法 / 指向用户根之外）。
 *
 * **它不回答"是不是一份合法 MKP Preset"**（有哪些字段、参数类型对不对 —— 那是 Preset 语义，
 * 客户端不复制那份 schema）；外部修改过但仍是能读的 TOML ⇒ 照常 `ok`，**不因 SHA 报警**。
 */
export type LocalState = 'ok' | 'unreadable'

export interface LocalPresetFile {
  /** 相对**用户根**的路径（`presets-local/A1-fast.toml`）—— 读正文 / 应用时把它交回来 */
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
   * 第九层的**文件级**状态（见 [`LocalState`]）。`unreadable` 的界面上说"文件无法读取"，
   * **不许应用 / 编辑**；不是预设候选（认不出是哪一类）时是 `null`
   */
  state: LocalState | null
  /**
   * 用不了时后端给的一句人话原因（可直接显示，比如"TOML 语法不对（第 3 行第 1 列）"）；
   * 能用 / 不适用是 `null`
   */
  stateDetail: string | null
  /** 它基于的云端那一版在不在（见 [`BasedOn`]）。`outdated` 就是"云端换版了，你这份基于旧版" */
  basedOn: BasedOn
  /**
   * 上面那个结论**摆事实**的那几格（详情面板直接显示，前端不重算）。
   *
   * 全部可空：`null` = 不知道（血统源没记账 / 目录没发布时刻），**不编**。
   * 源身份是规范化过的 `平台#仓库@ref`（`github#owner/repo@main`、`custom#host@-`）。
   */
  basedOnFacts: BasedOnFacts
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
   * 「改归属」（`setLocalPresetMachineVersion`）改的就是文件头那两行 ——
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
   * 一格，丢了就退回「本地」，什么都不坏。
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
 * 云端线   dist/mkp/presets/A1-fast.toml ──改这份──▶ 临时文件 ──保存──▶ presets-local/A1-fast（已修改）.toml
 * 用户线   presets-local/A1-fast（已修改）.toml ──改这份──▶ 临时文件 ──保存──▶ 写回它自己（第八层）
 * ```
 *
 * 改的永远是**临时文件**（`run/draft-preset.json`，改到一半关掉也还在）；
 * 原件全程一动不动，云端那份只有"云端换版本"能替换它。
 */
export interface PresetDraft {
  /** 改的是哪一条线（交付预设文件 / 我自己那份）—— 保存按钮说什么由它决定 */
  origin: ActiveOrigin
  /** 从哪一份改出来的。云端线是 `mkp/` 里的文件名；用户线是给人看的文件名（落点看 `path`） */
  sourceFileName: string
  /** **用户线**的落点（相对用户根）：保存时写回这里。云端线是 `null`（落点由目录给） */
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
  /** 相对**用户根**的路径（`presets-local/A1-fast（已修改）.toml`） */
  path: string
  fileName: string
  size: number
  /** 盖掉了一份同名的用户文件（第二次保存就是这种） */
  replaced: boolean
}

/*
 * **修改历史账本**（2026-10-17）—— 参数弹窗「修改历史」抽屉的**持久化形态**。
 *
 * 作者口径：「修改历史是类似 git 的保存的那种形式吧，**不要清空，每次重新打开软件
 * 都存在**；所有的 toml 文件都需要存历史修改的，不管是导入的还是什么的」。
 *
 * 于是它不再是一份内存日志：一份文件一本账，落在内部根 `run/preset-history/<指纹>.json`
 * （后端 `runtime::history`），钥匙 = 哪条线 + 身份（用户线按相对路径 / 云端线按文件名）——
 * **导入的 toml 进来之后就是用户线的一份文件**，所以也天然有自己的一本。
 *
 * 前端仍然拿整本（读一次、整本写回）：一本账通常几十条、几 KB，
 * 而"撤销把上一条标成 undone"这类动作改的是**已有条目**，追加模型反而要多造一层协议。
 */
export interface PresetHistory {
  /** 这一份保存过几次（批号的水位，与条目的 `batch` 同一套数） */
  batches: number
  /**
   * 每个参数**「上一次保存之前的值」**（2026-10-21，作者：「保存之后还是那样显示，
   * 还需要一个我的旧值；不只是更新才有，所有的都能记录下来」）。
   * 跟账本走（一份文件一本、清空历史一并清）；旧账本没有这一格 ⇒ 按空读。
   */
  prevSaved: Record<string, string>
  entries: HistoryRecord[]
}

/** 打完 / 恢复完一份备份的回执 */
export interface BackupSummary {
  /** 备份文件自己的绝对路径（界面照原样显示：用户要知道东西落在哪了） */
  path: string
  /** 里面几个文件 */
  files: number
  /** 字节数（恢复时 = 那份 zip 的大小） */
  bytes: number
  /** 什么时候打的这份备份（UTC epoch 秒） */
  createdUnix: number
}

/** 账本里的一条改动（一次动作）。`state` 与 `HistoryEntry` 同一个枚举 */
export interface HistoryRecord {
  id: number
  no: number
  /** 人话的事件名（「改 Y 轴偏移」/「回退 第 1 次保存」） */
  action: string
  /** 发生那一刻（UTC epoch 秒）—— 持久化之后才有它 */
  tsUnix: number
  /** `draft` = 还没保存过；`saved` = 落在某次保存里；`undone` = 被撤销掉了 */
  state: 'draft' | 'saved' | 'undone'
  /** 第几次保存把它落地的；`null` = 还没保存过 */
  batch: number | null
  /**
   * 这一条是**「云端更新」落下来的**（2026-10-20）—— 修改历史里挂一枚「更新」徽章。
   * 旧账本里没有这一格 ⇒ 按 `false` 读（Rust 侧 `#[serde(default)]`）。
   */
  fromSync?: boolean
  items: HistoryRecordItem[]
}

/** 一条改动里的**一项**（一个参数） */
export interface HistoryRecordItem {
  key: string
  from: string
  to: string
  label: string
  tab: string
  section: string
  unit: string | null
}

/*
 * **正文编辑器**（「编辑文件」）那条链的草稿 —— 与上面 `PresetDraft`（参数页那条链）
 * **分开存**（2026-10-10 作者裁决）：两处同时改一份文件时互不干扰，也不互相作废。
 *
 * 唯一的例外：**参数页保存**（`commitPresetDraft`）会把**正文草稿作废**
 * （盘上字节换了，那份草稿就成了孤儿）—— 契约里写死，且只有这一个方向。
 */
export interface PresetTextDraft {
  /** 「改的是哪一份」的人话名（`A1-fast（已修改）.toml`） */
  fileName: string
  /**
   * 相对**用户根**的路径（`presets-local/A1-fast（已修改）.toml`）——
   * 编辑器只服务「本地预设」，所以这里恒有值
   */
  path: string
  /** 正文：用户改到哪算哪（**沿用所选用户线的路径**，与参数草稿共享同一个临时文件） */
  text: string
  /** 最后改动时刻（UTC epoch 秒） */
  updatedUnix: number
  /** 这次打开是**接着上次改**（草稿本来就是这一份的），不是新建的 */
  reused: boolean
  /**
   * 草稿与**盘上当前字节**比过：真有未保存的改动才算脏（2026-10-16 加）。
   *
   * 与 `reused` 是两个维度：`reused && !dirty` = "接着上次打开、但一字没改"
   * （打开过又关掉，草稿还在但内容没变）。界面拿它决定"重开时要不要直接进编辑模式
   * 并提醒还没保存"（作者 2026-10-16：「下次打开这个模态框……说刚才那些修改还没保存」）。
   *
   * ★ 比不了（那一份读不出来 / 被移走了）时**保守算脏** —— 宁可多提醒一句，不要漏报。
   */
  dirty: boolean
  /**
   * **这份正文现在的 TOML 语法错**（`null` = 能解析，2026-10-21）。
   *
   * 打开编辑器那一刻后端就校一遍：手滑敲坏一个键名 / 引号时，用户在这里**当场**看见
   * "第几行错了"，而不是等切片器拉起钩子才收到一句"预设解析失败"。
   * 文案指路：底栏「恢复默认值」退回最初那份（`baseline-local` 的原文镜像）。
   */
  problem: string | null
  /**
   * **相对最初那份少了哪些参数**（2026-10-21）：`key（中文名）`，按参数页顺序。
   *
   * 与 `problem` 是两件事：那个说"TOML 读不动"，这个说"读得动、但某一项不见了" ——
   * 粘行最典型的样子（`z_calibration_mode = "new"` 被粘进上一行行尾注释，
   * TOML 仍然合法，切片时那一项静默退回默认值）。空数组 = 一项都没缺；
   * 没有原文镜像时不检查（不编）。
   */
  missing: PresetTextMissing[]
  /** 与最初那份相比值变了的参数（空 = 一致；界面按"改了 N 项"说话） */
  changed: PresetTextChanged[]
}

/**
 * 少掉的一项参数（**给用户看的**）。
 *
 * `key` 只在排查 / 报告里有意义，界面上说的是 `label` 和"它现在在第几行" ——
 * 作者 2026-10-21：「用用户的那种面向用户的那种文案口吻去告诉他哪里错了，不要写得这么技术」。
 */
export interface PresetTextMissing {
  /** 参数主键（`wiping.z_calibration_mode`） */
  key: string
  /** 中文名（`Z校准模式`） */
  label: string
  /**
   * 这个键在正文里**出现在第几行**（1 起）。粘行时它就在"上一行"里，
   * 于是界面能说"第 102 行"而不是"某处不见了"。找不到就是 `null`。
   */
  line: number | null
}

/**
 * 与**最初那份**不一样的一项（2026-10-21 作者：「你可以让他和默认值对比一下，
 * 这样子用户很容易就知道改错了哪里」「没出问题的时候也可以和默认值对比」）。
 */
export interface PresetTextChanged {
  /** 参数主键（排查用；界面只说中文名） */
  key: string
  /** 中文名（`擦料塔X坐标`） */
  label: string
  /** 最初那份里写的值 */
  from: string
  /** 现在写的值 */
  to: string
}

/** 一份**还没有保存**的正文的校验结果（编辑器每停一下问一次，提示跟着字走） */
export interface PresetTextCheck {
  /** TOML 语法错（`null` = 能解析） */
  problem: string | null
  /** 相对最初那份少了哪些参数 */
  missing: PresetTextMissing[]
  /** 与最初那份相比值变了的参数（没变就是空数组，界面说"与最初一致"） */
  changed: PresetTextChanged[]
}

/*
 * 参数草稿的**摘要** —— 预设列表行上的「未保存」标记、浮动提示、"要不要重开"三处erground。
 * 只摘要不正文：`null` = 现在没有参数草稿（保存了 / 作废了 / 从没打开过）。
 */
export interface PresetDraftSummary {
  /** 改的是哪一条线（交付预设文件 / 我自己那份） */
  origin: ActiveOrigin
  /** 从哪一份改出来的。云端线是下载区里的文件名；用户线是给人看的文件名 */
  sourceFileName: string
  /** **用户线**的落点（相对用户根）；云端线是 `null`（落点由目录给） */
  path: string | null
  /** 「真改过才算」：与来源现在的字节比（只差血统处置 / 换行的差异不算改动） */
  dirty: boolean
  /** 最后改动时刻（UTC epoch 秒） */
  updatedUnix: number
}

/** 身份账本的一行：「我这边这一份」↔「云端那一版」 */
export interface LocalPresetIdentity {
  /** 生成一次、永不变（界面上不显示，只用来对账） */
  id: string
  /** 现在落在哪（相对用户根，如 `presets-local/A1-fast.toml`） */
  path: string
  fileName: string
  /** 从云端哪个条目下来的（目录里的 `fileName`） */
  cloudSource: string
}

/** 一份用户文件的新落点与名字（第十层改名 / 第十一层另存为一份新的，两处共用这一个形状） */
export interface LocalFileIdentity {
  /** 落点（相对**用户根**，`presets-local/…`） */
  path: string
  /** 文件名 —— **落盘的就是这一串** */
  fileName: string
  /**
   * 这个名字被程序**修过了**（剔掉了不合规的字符 / 名字太长被截到上限）——
   * 界面据此照实说一句"名字太长，已截断为 X"。
   *
   * ★ 名字不合规**不算失败**（2026-10-20 作者：「改名有限制…… 不要失败，
   * 可以自动截断显示出来被截断了」）：只有空的、后缀变了的那两档才拒。
   */
  adjusted: boolean
}

/**
 * 一台机型 / 一个版本对应的「本地那份」+ 它带的校准值（校准页的初值与保存落点）。
 *
 * **整个对象缺席**（`null`）= 这个机型 / 版本还没有你的一份 —— 校准页照旧显示
 * 云端默认值，**保存被拦**（先另存一份自己的才有得存：校准的对象是「本地预设」，
 * 云端那一份是模板、永远不可变）。
 */
export interface LocalCopyCalibration {
  fileName: string
  /** 落点（相对**用户根**，`presets-local/…`）—— 保存校准时把它交回来 */
  path: string
  /** 三轴偏移；缺一个轴就是 `null`（不拿半个基准充数） */
  axes: Axes | null
  /** 涂胶速度限速；读不出来是 `null` */
  speed: number | null
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
  /** 落进用户根之后的相对路径（`presets-local/…`）；失败是空串 */
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
  /** 英文显示名（2026-10-27 双语数据）。空 = 回落 `name` */
  nameEn?: string | null
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
 * 归档区里的一份**云端旧版本**（cloud 换版本时被换下来的那一份）。
 *
 * 归档是**云端版本生命周期**的一部分，**不是用户修改历史**：换版本时旧份进
 * `archive/`（保留最早一份，不覆盖、不删）；用户改出来的东西是另一条线
 * （另存成另一份文件），永远不回写云端原件。
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
 * 盘上这一份**认得出是哪一版吗**（第三圈第 6 层：云端文件的 SHA 报警）。
 *
 * **只答"本机这份是不是我们认可的云端内容"**，不掺"云端有没有更新"（后者是
 * [`checkRemoteUpdate`]，比的是目录指纹，与本机这一份的字节无关）。混成一句「需更新」，
 * 用户既不知道自己的文件是不是被改过，也不知道该不该等更新 —— 所以它是单独一条读。
 *
 * **只列有事的**：还没下载、和与目录逐字节一致的两种不出现（它们没有问题，
 * 列进来只会把真正要处理的那几份淹掉）。
 */
export interface DeliveryTrust {
  fileName: string
  /**
   * `old`      认得出它是云端的某一版旧版（归档里有它字节，或被归档的旧目录登记过）
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
  /**
   * 这个参数在 MKP 预设 TOML 里的**字段名**（`offset_x` / `speed_limit`…）。
   *
   * 真目录里本来就有它（`presetdata::ParamDef` 的 `toml_key`，serde 成 camelCase）——
   * 客户端的**写值**不读它（按注册表定位是 Rust 那边的事），
   * 只有**假后端的对比台**要靠它把演示正文对回参数键（真机上这一步在 Rust 里）。
   */
  tomlKey?: string
  /** 数据域分区（= key 前缀）。**不是界面分组** —— 分组看 layout.sectionId */
  section: string
  /** 参数自己声明的界面归属（组内顺序 + 属于哪个分组） */
  layout: { order: number; sectionId: string }
  label: string
  /** 英文显示名（2026-10-26 双语注册表）。缺省 = 回落 `label` */
  labelEn?: string
  desc: string
  /** 英文说明。缺省 = 回落 `desc` */
  descEn?: string
  unit?: string
  /** `number` / `switch` / `segmented` / `select` / `gcode` —— 控件形态的原词 */
  uiComponent: string
  valueType: 'float' | 'int' | 'bool' | 'string'
  /** 选项级弃用在 `Choice.deprecated`（全局）与 `Choice.deprecatedMachines`（点名机型）上 */
  choices?: {
    value: string
    label: string
    /** 英文显示名。缺省 = 回落 `label` */
    labelEn?: string
    deprecated?: boolean
    deprecatedMachines?: string[]
  }[]
  min?: number
  max?: number
  step?: number
  deprecated?: boolean
  /** 只在这些机型上弃用（与 `deprecated` 叠加） */
  deprecatedMachines?: string[]
  /** 只对这些机型生效；空数组 = 不限机型 */
  machineFilter?: string[]
  showWhen?: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }
}

/** 页签与分组的元数据（中文名、顺序的唯一权威；`layout_schema` 全文没有 label） */
export interface CatalogTabMeta {
  id: string
  label: string
  /** 英文显示名（缺省 = 回落 `label`） */
  labelEn?: string
  order: number
  icon?: string
  sections: {
    id: string
    label: string
    labelEn?: string
    order: number
    description?: string
    descriptionEn?: string
  }[]
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
  /** `'image' | 'icon' | 'model' | 'slicerProfile' | 'mkPreset' | 'guide'` */
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

/**
 * 分组里的一名成员（`presets/assets.toml` 的 `[[groups.members]]`）。
 *
 * `role` = 这张在这一组里演什么（`light` / `dark` / …，可空）。客户端按它挑图 ——
 * 「哪张是浅色那张」是**内容属性**，所以它写在登记里，不看文件名。
 */
export interface CatalogGroupMember {
  /** 指 [`CatalogAsset.id`]（**指 id，不指路径** —— 路径会改，id 不改） */
  asset: string
  role?: string
}

/**
 * **一组图**（2026-10-16）：共用同一份标注源稿的那几张。
 *
 * 与 `assets` 同一类东西 —— 都是台账登记、都进目录、都可云端下发 ⇒ 改组不用重编客户端。
 */
export interface CatalogGroup {
  /** 组 id（**同时就是那份标注源稿的文件名**） */
  id: string
  /** 组名（给人看的） */
  name: string
  members: CatalogGroupMember[]
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
  /**
   * **分组登记**（2026-10-16）：哪几张图共用同一份标注、每张在那个组里演什么角色。
   *
   * 客户端**按它挑图**（例：引导图按主题要 `role = 'light'` / `'dark'` 那一张）——
   * 从前那种"把两个资产 id 写死在源码里"的做法，一改名或一换图就得重编客户端。
   * 现在组是台账里的一份登记 ⇒ 换图、换成员、改组名都只是发一份新目录，**不用重编**。
   *
   * **可选**：盘上那份 catalog.json 可能还是旧版释放的（那一版里没有这一栏）。
   * 缺了就是「不知道哪张是哪张」⇒ 那一处引导不出图（缺图一路往下落，不白屏）。
   */
  groups?: CatalogGroup[]
  registry: CatalogRegistry
  /** 打印板（2026-10-02）：机型持引用（`Machine.plateIds`），几何住这里，按 id 查 */
  plates: Plate[]
}

/**
 * 使用中那一份**住在哪条线上**（第七层起）。
 *
 * ```text
 * preset  云端预设线：目录（catalog）登记的 MKP 预设（`delivery/mkp/presets/…`）——
 *         只读，只有云端换版本能替换它
 * local   用户线：用户自己那份（`presets-local/…`）—— 用户可改，不属任何云端版本
 * ```
 *
 * **"只读"是文件归属的属性，不是"能不能被使用"的属性** —— 两条线都能成为使用中的那一份。
 *
 * ★ 与列表行的分类（[`PresetOrigin`]：`asset` / `preset` / `local`）**不是同一套**：
 * 能成为"使用中"的只有 `preset` 与 `local` 两类，资源（`asset`）没有"使用中"这回事。
 *
 * ★ `'cloud'` 与 `'mine'` 是**盘上老档里的旧值**（术语改造 / C 阶段定名留下的），
 * 后端按只读别名读得进来，写回一律是 `'preset'` / `'local'`。
 */
export type ActiveOrigin = 'preset' | 'local'

/**
 * 使用中指针（新数据世界的第一个用户状态，全局唯一）。
 *
 * `intact` 是"盘上那份还是应用时刻的那份"。**两条线上它的意思不一样**：
 * 云端线正常恒 true（`mkp/` 是只读区）；用户线在用户自己又改了那份时是 false ——
 * 那是正常事（那份是他的），不是"这份配置坏了"。
 */
export interface ActivePreset {
  origin: ActiveOrigin
  fileName: string
  /** 用户线的落点（相对用户根）；云端线是 `null`（落点由目录给） */
  path: string | null
  sha256: string
  /** 认不出是哪台机型哪一版时是空串（用户那份没有血统、或目录里已经没有来源那份） */
  machineId: string
  versionId: string
  intact: boolean
}

/**
 * 「取回交付预设」的结果（[`fetchCloudPreset`]，2026-10-09 改判）。
 *
 * `fetched` 说这次**有没有真的取回来一份**云端原件（本机已经是当前版时是 `false`，
 * 一次网络都不发）；`created` 说这次**有没有新落一份我自己的工作副本**
 * （没有的话用的是已经在的那一份，那份一个字节都没动）。
 *
 * 两个都是"这次到底做了什么"的实情 —— 界面据此说清「取回来了没有」与
 * 「你之前那份还在不在」。`path` / `fileName` 是本地那份的落点：调用方要接着
 * 「使用」它（[`applyPreset`]）时就用这两个，不用自己拼路径。
 */
export interface FetchCloudResult {
  fetched: boolean
  created: boolean
  fileName: string
  /** 本地那份的落点（相对用户根：`presets-local/…`） */
  path: string
  /**
   * **同名已经有一份了**（2026-10-10）：这次**没落新的一份** —— 界面该问用户
   * "要不要导入（自动改名）"（不覆盖、也不静默丢掉）。
   *
   * 值是"占着这个名字的那一份的落点"。
   */
  takenBy?: string
  /**
   * 上面那一档里，用户同意导入之后**会自动改成什么名字**
   * （`A1-fast-2.toml` 这种）—— 弹框里直接显示给用户看，不用他自己猜。
   */
  suggestedName?: string
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
 * 一个**内置云端源**（作者 2026-10-05 拍"两个云端源"）。
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
  /**
   * **规范化后的源身份**（`github#owner/repo@main`，2026-10-27）。
   *
   * 只用来显示（云端详情里"这个云端是哪个源"）；判据在 Rust 侧，前端不拿它算新旧。
   * 认不出就是 `null`。
   */
  identity: string | null
}

/* ---------- 界面偏好（2026-10-12） ---------- */

/**
 * 界面主题档位 —— **id 是契约**（与 Rust `runtime::app_state::ThemePref` 一一对应，
 * 盘上 / 界面 / 前端都认这三个字面量）。
 *
 * `system` = 跟随系统深浅：程序不猜操作系统此刻是深是浅，界面读
 * `prefers-color-scheme` 解析（webview 里那个才是真值）。
 */
export type ThemePref = 'light' | 'dark' | 'system'

/* ---------- 界面语言（2026-10-25） ---------- */

/**
 * 界面语言档 —— **id 是契约**（与 Rust `runtime::app_state::LocalePref` 一一对应）。
 *
 * - `system`：跟随系统（出厂档）；
 * - `zh-CN`：简体中文；
 * - `en`：English。
 *
 * ★ 线上字面量带大写（`zh-CN`）—— Rust 那一侧是逐个 `serde(rename)`，
 *   不是 `rename_all`，改这里必须同时改那边。
 * ★ `null`（还没选过）与 `'system'`（明确点了跟随系统）分得开，与主题同一条口径。
 */
export type LocalePref = 'system' | 'zh-CN' | 'en'

/* ---------- 主窗怎么摆（2026-10-17） ---------- */

/**
 * 置顶档位 —— **id 是契约**（与 Rust `runtime::app_state::PinPref` 一一对应）。
 *
 * - `off`：不置顶；
 * - `page`：**只有工艺页置顶**（切到别的页签自动解除，切回来又浮上去）；
 * - `always`：任何页面都置顶。
 *
 * ★ 档位是"用户选的"，**当刻真有没有置顶**是算出来的（`page` 档不在工艺页时就不置顶），
 *   两件事不要混成一个开关。
 */
export type PinPref = 'off' | 'page' | 'always'

/** 窗口这一族的两格账（与 Rust `ipc::window_layout::WindowPrefs` 同形） */
export interface WindowPrefs {
  /** 置顶档位 */
  pin: PinPref
  /** 现在是不是窄窗态（600 宽、贴屏幕右缘、高占可用高 90%） */
  compact: boolean
}

/* ---------- 后处理偏好（2026-10-17） ---------- */

/** 「跑完这一趟界面怎么摆」：`silent` = 后台静默不起窗，`gui` = 独立窗口（出厂）。 */
export type PostprocUiMode = 'silent' | 'gui'

/**
 * 设置页里那几档的**解析后**值（`null` 已解、量程已夹）。
 *
 * 管线读的就是这一份（经 `runtime::postproc_prefs::apply_to_ir` 写进 `ir.prefs`）——
 * 界面只负责把它画出来、改它。
 */
export interface ResolvedPostprocPrefs {
  notifyOnFinish: boolean
  uiMode: PostprocUiMode
  glueBottomEnabled: boolean
  skipVibrationCalibration: boolean
  beginStageRetractEnabled: boolean
  beginStageRetractZLiftMm: number
  zCalibrationDwellSec: number
  xyCalibrationDwellSec: number
  bedGlueEnabled: boolean
  bedGlueAllModels: boolean
  /** **自动摆塔**（2026-10-18）：模型与塔相撞时问一句，选"自动挪塔"才改这一趟的塔位置 */
  autoTowerPosition: boolean
}

/**
 * 盘上那一格（`run/app-state.json` 的 `postproc`）。
 *
 * 每一项 `null`/缺省 = **这一项从没动过** ⇒ 用出厂档
 * （与"明确选了出厂档"分得开，见 `runtime::app_state::PostprocPrefs` 的注释）。
 * 写的时候传这个形状：改一项传全量，某项给 `null` = 那一项回到出厂档。
 */
export interface PostprocPrefsWire {
  notifyOnFinish?: boolean | null
  uiMode?: PostprocUiMode | null
  glueBottomEnabled?: boolean | null
  skipVibrationCalibration?: boolean | null
  beginStageRetractEnabled?: boolean | null
  beginStageRetractZLiftMm?: number | null
  zCalibrationDwellSec?: number | null
  xyCalibrationDwellSec?: number | null
  bedGlueEnabled?: boolean | null
  bedGlueAllModels?: boolean | null
  /** **自动摆塔**（2026-10-18）：`null` = 回到出厂档（关） */
  autoTowerPosition?: boolean | null
}

/** `getPostprocPrefs` 的回执：盘上原样 + 解析后 + 量程。 */
export interface PostprocPrefsView {
  raw: PostprocPrefsWire
  resolved: ResolvedPostprocPrefs
  /** 量程来自后端，界面**不另写一份** */
  limits: {
    zDwell: { min: number; max: number }
    xyDwell: { min: number; max: number }
    zLift: { min: number; max: number }
  }
}

/* ---------- 屏幕引导（2026-10-15） ---------- */

/** 屏幕坐标下的一个矩形（左上角 + 宽高，单位是**点**，与 Rust `ax::RectDto` 同形）。 */
export interface ScreenRect {
  x: number
  y: number
  width: number
  height: number
}

/**
 * 「板子类型」控件此刻在屏幕上的哪一块（与 Rust `ax::GuideTargetDto` 同形）。
 *
 * 形状沿用「能不给就不给、不给就说话」那一套（与 `PostProcessCommandDto` 同一条规矩）：
 * `ready` 决定能不能指，`ready: false` 时一定带一句人话 **reason**，
 * **绝不编一组坐标** —— 箭头指错位置比不指更糟。
 */
export interface GuideTarget {
  /** 两个条件都成立才为真：拿到切片器窗口、且窗口里找到了那个控件 */
  ready: boolean
  /** 切片器窗口的屏幕矩形（引导层拿它判"窗口动没动"） */
  window?: ScreenRect
  /** 「板子类型」控件的屏幕矩形 —— 箭头指它 */
  target?: ScreenRect
  /** 指不了时的一句人话（为什么 + 下一步做什么） */
  reason?: string
}

/* ---------- baseline + 对比台（2026-10-08） ---------- */

/**
 * 云端预设的一个版本（云端表按预设归组之后，组内的每一行）。
 *
 * ★ **这不是"过时判定"**：它说的是"云端登记过这一版、我这一版下过没有"，
 * 不是"你那份该更新了"。用户自己那份是完整、可继续使用的一份，系统不替他更新。
 */
export interface CloudVersion {
  /** 归属的云端预设文件名（`A1-fast.toml`）—— 界面按它归组 */
  fileName: string
  /** 内容摘要（版本的唯一身份）。**界面不显示** —— 用户没必要看见哈希 */
  sha256: string
  /** 云端**发布日期真值**（该版正文文件头的 `# release_time`）。`null` = 还没下载，拿不到 */
  releaseTime: string | null
  /** 目录代时间（`catalog.publishedAt`）—— 兜底，界面上标明是"目录发布时间" */
  publishedAt: string | null
  /** 这一版本机下过没有（= 隐藏 baseline 里有它） */
  downloaded: boolean
  /**
   * 这一版是**当前目录登记的那一版** —— 只有它的「下载」真能拿到（历史版本没法按摘要重下）。
   * 界面据此决定给不给「下载」：历史版本没下过也给不出一个真能成的动作。
   */
  current: boolean
}

/** 一次参数改动：参数 key + 目标值的字符串形式（形态由注册表决定，前端**不拼 TOML**） */
export interface ParamEdit {
  paramKey: string
  value: string
}

/** 一份用户预设的参数值（对比台里的一列） */
export interface PresetParamValues {
  /** 相对用户根的路径（`presets-local/A1-fast-2026-10-15.toml`） */
  path: string
  fileName: string
  /** 参数 key → 控件看得懂的值（`-1.5` / `true` / `standard` / 多行 G-code） */
  values: Record<string, string>
  /** 这一份读不出来时的一句人话（此时 `values` 是空表） */
  problem: string | null
}

/* ---------- 逐参数「云端更新」（2026-10-09） ---------- */

/**
 * 一个参数上"我做过的那次决定"。
 *
 * `adopt` = 采用云端新值（文件里那一项已经写成云端新值）；
 * `hold` = 明确保持本地值（文件里那一项一个字没动）。
 * 两者都把这一项的水位推到当时的云端版 —— 于是同一版不会再挂着，云端再改才会重新进待处理。
 */
export type ParamDecisionKind = 'adopt' | 'hold'

/** 一个参数上的四方账。`null` = 这一格没有值（**不是空串**：文件里没有这一项就是没有） */
export interface ParamSyncEntry {
  /** 参数 key（注册表的键，与 `RecipeParam.key` 同一套） */
  key: string
  /** 我这份文件里现在写着的值 */
  local: string | null
  /** 云端**旧值**：我上次处理到的那一版云端里这一项是什么 */
  baselineOld: string | null
  /** 云端**当前最新值**。云端当前版字节不在本机 ⇒ `null`（界面照实说"还没取回来"） */
  cloudNew: string | null
  /** 云端改过这一项、而我还没处理 */
  pending: boolean
  /**
   * **这一项是冲突**（2026-10-10）：云端新版改了它、**我也改过它**，
   * 两边的值还不一样。更新时这类项**必须让用户选**（使用新版值 / 保留本地值）；
   * 不是冲突的（云端改了、我没动）点「更新」就可以直接按新版采用。
   */
  conflict: boolean
  /** 我已经对**当前云端版**处理过这一项 */
  decided: ParamDecisionKind | null
}

/**
 * 一份用户预设的「云端更新」总账。
 *
 * 三方 = **我这份文件 / 云端旧值（我上次处理到的那一版） / 云端当前最新版**。
 * 用户看不见"旧值是哪一版"这种话 —— 界面上只说"云端旧值 → 云端新值"。
 */
export interface PresetParamSync {
  /** 相对用户根的路径（`presets-local/A1-fast.toml`） */
  path: string
  fileName: string
  /** 这份预设自己的归属（文件头两行归一化后的）。认不出是 `null` */
  machineId: string | null
  versionId: string | null
  /** 云端那一份的文件名（`A1-fast.toml`）。认不出是 `null` —— 那就比不出云端更新 */
  cloudFileName: string | null
  /** 我这份当初基于的那一版云端的发布时间（给人看的） */
  basedOnReleaseTime: string | null
  /** 云端当前版的发布时间（给人看的）。拿不到是 `null` */
  currentReleaseTime: string | null
  /** 云端当前版的字节**在本机可用**（可用就能逐项比） */
  cloudReady: boolean
  /** 云端换版了：当前版与我这份当初那一版不是同一份 */
  versionAdvanced: boolean
  /** 待处理条数（= `entries` 里 `pending` 的条数） */
  pendingCount: number
  /** 逐参数的四方账，顺序与参数定义一致 */
  entries: ParamSyncEntry[]
}

/** 一条要落下去的决定 */
export interface ParamDecision {
  paramKey: string
  kind: ParamDecisionKind
}

/* ——— 后处理钩子那一趟（切片器导出 G-code 时起的那个进程）———
 *
 * 几个形状与 Rust 侧 `hook_ui` 里的 `FactsPayload` / `ProgressPayload` /
 * `QuestionPayload` / `FinishedPayload` / `RunSnapshot` 一一对应
 * （两边没有编译器，靠这里的注释对齐）。
 */

/**
 * 开场那几个事实（那一屏左栏那张表）。
 *
 * **只报开跑那一刻就确定的那几格**：总层数要 pass1 刮出来、擦料塔类型要 support 步的
 * 判据定 —— 那两个不在这一条里（`Finished` 之前拿不到，编一个比空着糟）。
 */
export interface PostProcessFacts {
  /** G-code 自报的机型（`; printer_model` 那一档） */
  gcodeMachine: string
  /** 预设那份的机型（维度与打印边界按它算） */
  presetMachine: string
  /** 耗材类型（`PLA` / `PETG` …） */
  filamentType: string
  /** 输入 G-code 的字节数 */
  fileSizeBytes: number
  /** 喷嘴直径（mm） */
  nozzleDiameter: number
  /** 层高（mm）—— 常规层高那一档 */
  layerHeight: number
  /** 涂胶速度（mm/s）——「切片与预估」那一格（跑着的时候就摆）；旧版钩子不报就不摆 */
  glueSpeed?: number
  /**
   * 切片软件 —— 跑着的时候只有预设里那个**名字**（如 `"BambuStudio"`）；
   * 带版本号的完整串要等 pass1 扫输入头行，那是跑完的 `result.slicer`。
   */
  slicer?: string
  /**
   * **切片器自报的预估总时长**（秒）—— 输入头 `; total estimated time: 3h 22m 55s` 那一行。
   *
   * 「切片与预估」栏的「预估打印时间」用它（跑着的时候就摆）；跑完之后那一格换成内核
   * 重算的（`result.printTimeSeconds`）—— 两个数不一样是正常的：**这是切片器说的、
   * 那是内核说的**（报告页也是两个都摆）。头里没有这一行就不摆。
   */
  slicerPrintTimeSeconds?: number
}

/** 一条进度。**全局百分比不在这里** —— 阶段名与权重是界面的事（见 `app/postprocess/steps.ts`） */
export interface PostProcessProgress {
  /** 阶段 id（内核的稳定标识：`input` / `pass1`…） */
  step: string
  /** 步内比例 `0..1`；`null` = 这一步此刻给不出比例 */
  fractionInStep: number | null
  message: string
}

/** 要用户回答的一个问题 */
/** 问句里的一颗按钮（与 Rust `hook_ipc::QuestionChoice` 逐字对齐） */
export interface PostProcessChoice {
  /** 回给钩子的 id：`keep` / `stop` / `print` / `direct` … */
  id: string
  label: string
  /** 一行补充说明（后果 / 何时选它） */
  hint?: string
  /** `true` = 主按钮 */
  accent: boolean
}

export interface PostProcessQuestion {
  /**
   * 问题种类：`machine-mismatch` = 机型不匹配（给「继续 / 停下」两颗按钮）。
   *
   * ★ 字段名是 `questionKind` 而不是 `kind`（2026-10-17 真机事故）：线上的枚举标签
   * 已经叫 `kind`（`{"kind":"question",…}`），payload 再来一个同名键，解析会报
   * `duplicate field 'kind'` 并把这一条**整条丢掉** —— 界面上就是"没有选择框"。
   */
  questionKind: string
  text: string
  /**
   * **可选答项**（2026-10-17 泛化）：空 / 缺省 = 用默认那两颗按钮（机型那一问）；
   * 非空 = 照这张表画按钮，点哪颗就把它的 `id` 回给钩子。
   *
   * 文案由后端给（后端才知道每个选项对管线意味着什么），界面不按 id 猜。
   */
  choices?: PostProcessChoice[]
  /**
   * 没人作答时**什么时候自动按"不跑"收场**（unix 毫秒；`0` = 这一问不会自己超时）。
   *
   * 界面据此显示倒计时（「1 分 52 秒后自动终止」）。给绝对时刻、不给"还剩几秒"：
   * 补推那一趟也照着同一条数，不会重新从头开始数。
   */
  deadlineUnixMs?: number
  /**
   * **挪塔那一问附带的画布几何**（2026-10-18）：只有 `tower-relocation` 这一问有，
   * 别的一问没有这一格 ⇒ 缺省。界面照它画俯视图与坐标。
   */
  geo?: RelocateGeo | null
}

/**
 * 挪塔那一问的**画布几何 + 坐标**（机器坐标 mm；**不取整** —— 作者：「可以显示坐标，
 * 不用整数也可以的」）。
 */
export interface RelocateGeo {
  /** 碰撞那一整套事实（与报告页俯视图同一份结构 ⇒ 界面复用同一个画布） */
  collision?: ReportCollision | null
  /** **挪塔记录**（2026-10-18）：走到了那一问才有；旧记录读到 `null` ⇒ 整块不摆 */
  relocation?: RelocationRecord | null
  /** 自动摆塔算出来的塔**左下角**；`null` = 板子摆不下（那就没有"自动挪塔"这颗按钮） */
  candidate?: [number, number] | null
  /** 现在塔的左下角（`wiper_x` / `wiper_y`） */
  current: [number, number]
}

/**
 * **挪塔记录**（2026-10-18）：报告页 / 执行记录照着它说"塔怎么挪的"。
 *
 * 与 `_meta.json` 的 `relocation` 逐字段对齐（Rust `#[serde(rename_all = "camelCase")]`）。
 * 只有真的走到"挪塔那一问"时才有这一格；别的记录读到 `null` ⇒ 整块不摆。
 */
export interface RelocationRecord {
  /** 用户当时选了什么：自动挪塔 / 自己挪模型 / 没答（超时）/ 板上没位 */
  choice: 'relocate' | 'model' | 'timeout' | 'unavailable'
  /** 塔左下角，挪之前（mm，**不取整**） */
  from: [number, number]
  /** 塔左下角，挪之后；没挪 = `null` */
  to?: [number, number] | null
  /** 直线位移（mm）；没挪 = `null` */
  movedMm?: number | null
  /** 挪之前的冲突点数 */
  beforeConflictCount: number
  /** 挪完复检的冲突点数（没挪 = `null`） */
  afterConflictCount?: number | null
  /** 挪完复检过没过（`null` = 没挪） */
  verified?: boolean | null
  /** 这一趟用的塔高（mm）—— 斜肋塔的底座随它变大 */
  towerHeightMm: number
  /**
   * **挪之前那一份碰撞事实**（2026-10-18）：报告页拿它画「现在 / 挪后」两幅俯视图。
   * 成功那趟 `collision` 是 `null`（撞了就走不到成功出口），所以事实跟着记录走。
   */
  before?: ReportCollision | null
}

/** 那一趟的结论 */
export interface PostProcessFinished {
  ok: boolean
  /** 用户按了停止（或 Ctrl-C）—— 不是"失败"，说法不一样 */
  cancelled: boolean
  /** **原因**（人话）。稳定错误码已经从这句里剥走了，单独给 `code` ——
   * 整句码塞给用户，他读不到到底哪儿不对 */
  message: string
  /** 稳定错误码（`E_*_NNN`，报问题时带上它）。取消与没有码的几句是 `null` */
  code: string | null
  /** 停在哪一阶段（内核的阶段 id，界面转中文名）；还没出过进度时是 `null` */
  stage: string | null
  /** 给切片器的退出码（0 成功 / 1 失败或取消 / 2 输入·预设错） */
  exitCode: number
  elapsedMs: number
  /** 产物落点（成功时就是输入那一份 —— 原地覆盖） */
  output: string | null
  warnings: string[]
  /**
   * 校准件那一趟选了哪种跑法：`print` = 先打印再校准（模型原样）/ `direct` = 直接校准
   * （去掉打印件）。不是校准件 = 缺省。
   *
   * 两种**都是成功**，界面同一个成功屏，只有那一句结论文案按它分开说。
   */
  calibChoice?: 'print' | 'direct'
  /**
   * **结果读数**（那一屏「结果」几扇要摆的读数：指标条 + 擦料与胶水消耗那几格）。
   *
   * 内核在成功那一条里报上来（Rust 侧 `hook::result_facts`，值就是归档那份事实：
   * `detail.glue` / `detail.slicer` / `printTime`）；失败 / 取消没有这一段
   * （内核还没把那几个数算出来）。界面按"有哪几格摆哪几格"渲染。
   *
   * ★★ **必须写成 `| null`**（2026-10-17 实测踩到）：Rust 侧这一格是
   *   `Option<...>`，序列化出来是**显式的 `null`**、不是省略这个键。
   *   只写 `?`（= `| undefined`）的话，判空写成 `res !== undefined` 就是一路假安全 ——
   *   线上真的会传 `null` 进来，然后在 `res.penX` 上当场抛。
   */
  result?: PostProcessResult | null
}

/**
 * 结果读数（`PostProcessFinished.result`）—— 字段全是可选的：
 * 内核报到哪几格，界面就摆哪几格（没到的不摆，不编）。
 *
 * **口径：全是原始数值**（mm / mm·s⁻¹ / 秒）—— 文案是界面的事，与报告页同一个规矩
 * （那边读归档也是拿数值自己格式化）。换算只在 Rust 侧做过一次：涂胶速度的
 * mm/min → mm/s（`archive::glue_speed_of`，全仓唯一换算点）。
 */
export interface PostProcessResult {
  /** 笔偏移 X / Y / Z（mm）—— 预设里那一组（`detail.glue.penOffsetX` …） */
  penX?: number
  penY?: number
  penZ?: number
  /** 涂胶速度（mm/s，装出来是 `"70.00 mm/s"`） */
  glueSpeed?: number
  /**
   * 预估打印秒数。**有界面时它是后补的**：钩子必须立刻退出（切片器在等退出码），
   * 打印时间交给常驻进程算 —— 算完由 `postprocess-print-time` 那条事件补上这一格
   * （见 `usePostProcessRun`）。所以成功那一刻它常常先是 `undefined`。
   */
  printTimeSeconds?: number
  /** 涂胶总长（mm）—— 报告页「涂胶」那一组的头一格（同一口径，不换算成体积） */
  glueLengthMm?: number
  /** 切片软件（`"BambuStudio 02.08.02.60"` —— pass1 从输入头行刮到的完整串） */
  slicer?: string
}

/**
 * 延后的打印时间**补算完了**（事件 `postprocess-print-time`）。
 *
 * 只补那一格：值落进 `PostProcessFinished.result.printTimeSeconds`，
 * 界面上「预估打印时间」那一行由它亮起来（补算失败就不发这条 —— 那一格继续空着）。
 */
export interface PostProcessPrintTime {
  /** 预估打印秒数（与管线内同一口径：同一个 `printtime::estimate` + 同一组 Options） */
  totalSeconds: number
}

/**
 * 新一趟的开场（事件 `postprocess-started`）。
 *
 * 界面据此把上一趟的结论与"我关过了"一起清掉 —— 窗口是常驻的、结论会留到下一次顶掉它
 * （旧世代同一条），所以"新的一趟开始了"必须有个明确的开场信号。
 */
export interface PostProcessStarted {
  presetName: string
  gcodeName: string
  /** 这一趟是开发台演出来的 ⇒ 界面把真业务按钮禁掉、标「预览模式」（见 `PostProcessRun.preview`） */
  preview: boolean
}

/**
 * 钩子那一趟的**全量快照**：窗口起来时读一次，之后跟着事件走。
 *
 * 只靠事件是不够的：窗口起来时那一趟可能已经跑到一半、甚至已经跑完（早期的失败尤其快）。
 */
export interface PostProcessRun {
  presetName: string
  gcodeName: string
  /** 开场那几个事实（左栏那张表）；那一趟在报事实之前就结束了 ⇒ `null` */
  facts: PostProcessFacts | null
  question: PostProcessQuestion | null
  progress: PostProcessProgress | null
  finished: PostProcessFinished | null
  /**
   * 这一屏是**开发台演出来的**（不是真钩子跑出来的）。
   *
   * `true` 时那一屏把真业务按钮（停止 / 后台运行 / 强制继续 / 终止）**禁掉**
   * 并标上「预览模式」—— 那些按钮走的是真钩子回写线，预览里点了只会如实报
   * "现在没有在跑后处理"（作者 2026-10-14：预览就明确是预览，不许点出真业务）。
   */
  preview: boolean
  /**
   * **界面自己造出来的回放**（2026-10-27）：报告页拿一份归档记录摊成"当时那一趟"，
   * 点开看进度、耗时、明细与收尾。它同样不接真业务，但文案说「回放」不说「预览」。
   *
   * 只由前端置位，**后端永远不下发这个键**。
   */
  replay?: boolean
  /**
   * 快照里这份问句，事件**有没有送到过界面**（2026-10-17）。
   *
   * `false` = 读快照与订阅事件这两条路里，订阅慢了（钩子推问句和开始等答复之间没有
   * 间隙，先后全看运气）—— 界面得喊一声 `replayPostProcessQuestion` 补推一次，
   * 否则就是"一片空白地等"，直到钩子那侧超时按不跑处理（作者真机看到的就是这个）。
   */
  questionEmitted: boolean
}

/**
 * 首页「复制后处理脚本」那**一整条命令**的现况（`getPostProcessCommand` 的答案）。
 *
 * # 两个概念分开（2026-10-10 作者裁定）
 *
 * - **按钮常驻**：界面规则 —— 不拿 `ready` 决定摆不摆按钮，只拿它显示"现在能不能抄"；
 * - **命令能不能给**：`ready` 由两条**真实事实**决定（本程序自己 + 预设真在本机）。
 *   `ready: false` 时**没有命令、没有路径**，只有一句 `reason`（可直接显示）。
 *
 * 前端不拼任何一段路径、也不长期缓存命令：点一下向后端现要一次，状态一变再要一次
 * （下载 / 应用 / 换份都可能把指向换掉）。当前平台 / 当前实例 / 真实落点现拼 ——
 * macOS 用 macOS 的可执行物与落点，Windows 用 Windows 的，两个平台同一份代码。
 */
export interface PostProcessCommand {
  /** 两个事实都成立才有命令 */
  ready: boolean
  /** `ready` 时 = 整条命令（`"<exe>" --Toml "<toml>" --Gcode`）；否则没有 */
  command?: string
  /** `ready` 时 = `--Toml` 指的那份预设的**本机绝对路径**（给人看）；否则没有 */
  toml?: string
  /** 不能给时的一句人话（为什么 + 下一步做什么）；能给时没有 */
  reason?: string
}

/* ---------- 本机 BBS 目录（工艺页「本机工艺」那一档，2026-10-13） ---------- */

/**
 * 本机 BBS（Bambu Studio / OrcaSlicer…）工艺预设目录里的一项。
 *
 * 字段名与那批 json / 试验场那个只读端点**逐字一致**（`snake_case` 照旧）——
 * 页面那边已经按这套名字解析（`app/bbs/bbsSource.ts`），在生产契约里再翻译一次
 * 只会多一处能漂的东西。
 */
export interface BbsLocalEntry {
  /** `user`（自己另存的）/ `system`（出厂那批） */
  scope: 'user' | 'system'
  /** 账号目录名；系统层那一档是 `null` */
  uid: string | null
  file: string
  /** 文件名里写的那个名字（读不出来就是去掉 `.json` 的文件名） */
  name: string
  /** 父预设名（BBS 的继承链靠它） */
  inherits: string | null
  /** BBS 自己就有两种写法（字符串 `"false"` 与布尔 `false` 都出现过）—— 照抄，不归一 */
  instantiation: string | boolean | null
  compatible_printers: string[] | null
  from: string | null
  /** 修改时间（毫秒） */
  mtime: number
  size: number
  /** 这份 json 坏了（解析失败的原话）—— 也要列出来，让界面标灰，而不是悄悄少一项 */
  broken: string | null
}

/**
 * 「本机有没有 BBS、有哪几份」。
 *
 * **读不到不是错误**：`available: false` + 一句人话（`reason`）说的是事实 ——
 * 这台机器上没装、或者装完还没跑过一次。界面照实说，不静默退到一份陈旧快照
 * （那正是这一页不打包那 285 份预设快照的理由）。
 */
export interface BbsLocalTree {
  available: boolean
  /** 命中的是哪个应用（`BambuStudio` / `BambuStudioBeta`） */
  app: string | null
  /** 这次读的是哪棵树（系统层在就用它，否则用户层） */
  root: string | null
  user: BbsLocalEntry[]
  system: BbsLocalEntry[]
  reason: string | null
}

/**
 * 「复制进切片器」这一次干了什么（[`MkpApi.copyToSlicer`] 的返回值）。
 *
 * `replaced: true` = 切片器目录里原来已经有一份同名的，这一手把它**换成了**我们这份
 * （2026-10-13 作者改判：同名就覆盖）；`false` = 新建。提示条按它说话 ——
 * 「替换掉了你别的东西」与「新放了一份」是两件事，不合并成一句"已复制"。
 */
export interface CopyToSlicerOutcome {
  replaced: boolean
}

/* ---------- 切片软件（2026-10-13 第二刀：「其他应用…」从磁盘指定） ---------- */

/**
 * 打开模型用哪个切片软件（`app/shared/slicer.ts` 里那份 `SlicerId` 与它**逐字对齐**）。
 *
 * - `'system'` = 跟随系统关联（不指定应用，没选过时就是它）
 * - `'bambu'` / `'orca'` = 内置的两家
 * - `'custom'` = 用户**从磁盘指定**的那个 —— 路径随 `slicerApp` 一起给
 *   （见 [`MkpApi.openModel`]）
 */
export type SlicerId = 'system' | 'bambu' | 'orca' | 'custom'

/**
 * 一个切片软件：用户从磁盘挑的那个（[`MkpApi.pickSlicerApp`]），
 * 或系统当前关联的那个（[`MkpApi.getSystemSlicerApp`]）。
 *
 * `path` 是**可执行物**（macOS 是 `.app` 包、Windows 是 `.exe`）。`name` 在取它的
 * 那一刻就该取好（列目录时取 / 问系统时取）—— 界面直接拿来显示（下拉里那一条），
 * 不再让谁去解析路径。
 */
export interface SlicerApp {
  /** 显示名（如 `PrusaSlicer`） */
  name: string
  /** 可执行物绝对路径 */
  path: string
}

export interface MkpApi {
  /**
   * **把校准好的三轴偏移写进「我的预设」**（2026-10-08 资源库改判）。
   *
   * `path` 来自 [`getLocalCopyFor`]（或用户线列表）—— 必须是 `presets-local/…` 里
   * 真在的那一份。三个值一起写、不按页分；后端只改注册表派生的那三处值
   * （注释、键序、血统三行一个字节不动）。
   * 云端那一份是模板、永远不可变：还没有你的一份就先「另存为本地预设」。
   */
  savePresetCalibration(path: string, axes: Axes): Promise<void>

  /** 校准板清单 */
  getCalibModels(): Promise<CalibModel[]>

  /**
   * 让壳去打开一个模型文件（本地有缓存就直接开，没有就先下载）。
   * 前端不碰文件系统，也不关心它是下载还是命中缓存 —— 那是壳的事。
   *
   * `opts.slicer`（2026-10-13）：**用哪个切片软件打开**（`'system'` = 跟随系统关联，
   * 不指定应用）。不传 = 跟随系统 —— 旧调用一个都不用改；设置页「切片」那格是
   * 默认值，模态框里「仅这一次」覆盖的那一次由前端随调用带给壳。
   *
   * ★ `opts.slicerApp`（2026-10-13 第二刀）：`'custom'` 时**必须**带上 —— 那是用户
   *   从磁盘指定的那个应用的可执行物路径（`SlicerApp.path`）。壳读不到 webview 的
   *   localStorage，这个值只能由前端随调用递过去；不带就回落跟随系统。
   */
  openModel(modelId: string, opts?: { slicer?: SlicerId; slicerApp?: string }): Promise<void>

  /**
   * **从磁盘挑一个切片软件**（设置页「切片」那格选「其他应用…」时走它，2026-10-13 第二刀）。
   *
   * 作者原话：「也可以打开软件去指定呀…就像比如说 macOS 打开那个文件管理器的时候，
   * 就在 application」。做法按平台各自成熟的那一套：macOS 停在 `/Applications`
   * （扫 `/Applications/*.app` 太脆 —— 扫出来的东西跟着系统版本漂），Windows 用标准
   * 文件对话框挑 `.exe`。**走的是前端的对话框插件**（与 `pickExportDir` 同一条），
   * 不新增 Rust 命令 —— 挑一个文件是界面上的事，产品里它不进后端账本。
   *
   * **取消给 `null`，不是错误**（与 `pickExportDir` 同一条口径）—— 什么都不写。
   */
  pickSlicerApp(): Promise<SlicerApp | null>

  /**
   * **这台机器上 `.3mf` 现在关联的程序**（2026-10-16）。
   *
   * 「切片」那格与「打开模型」框里都有「跟随系统默认」这一档 —— 它是四档里**唯一一档
   * 名字里没有应用名**的：用户看着它没法知道双击会开哪个软件（作者原话：「我不知道
   * 它默认的是哪一个」）。这一条就是去把这个名字取回来，让界面摆在该档旁边。
   *
   * 问的是**系统真值**，两边各自问各自的那一套（macOS 走 LaunchServices、
   * Windows 走注册表，见 `src-tauri/src/ipc/slicer.rs` —— 都在壳那一侧：
   * 前端不扫盘、不猜安装位置）。
   *
   * **问不出来给 `null`**（这台机器没关联任何程序 / 这一次问不成）：界面回落原来那句话
   * （「用 .3mf 关联的程序打开」），**不编一个应用名顶上** —— 那句话是给用户看的。
   */
  getSystemSlicerApp(): Promise<SlicerApp | null>

  /**
   * **把用户送到"改 `.3mf` 默认应用"的那一处**（2026-10-16）。
   *
   * 我们**不替用户改**系统默认 —— 那是他自己的账，改不改、改成什么该由他在系统那一侧按。
   * 这一条只把他送过去：
   * - macOS：**系统设置里根本没有"文件关联"这一页**（查过：那 280 个设置扩展里没有一个
   *   管默认应用），唯一能改的是访达的「显示简介 → 打开方式 → 更改全部…」⇒ 我们**打开那份
   *   示例的「显示简介」窗口**。★ 要过一次「自动化 → 控制访达」的授权（第一次弹一张系统框，
   *   点「好」之后不再弹）；拒绝了就如实报错，并把手工那三步写在那句话里。
   *   ★ 只把文件夹定位出来**不算**（作者 2026-10-16：「我不是要打开那个文件夹呀，我是要像
   *   你那个打开显示简介一样呀」）。
   * - Windows：「你要如何打开这个文件？」那个框（勾上"始终"就是改默认）。
   *
   * 拿去当把手的是壳自己放的**一份空的示例 `.3mf`**（`<内部根>/open-with/`）——
   * 两个平台的 UI 都只是"对一份文件发问"，判据是扩展名，**不依赖本机下载过什么**。
   *
   * 回来的是**下一步提示**（界面照原样弹一条）—— "到了那儿还要点哪儿"是平台知识，
   * 由壳那一侧说清（`src-tauri/src/ipc/slicer.rs`），前端不复述、不翻译。
   */
  openDefaultAppSettings(): Promise<string>

  /**
   * **「自动化」授权被拒后的反悔入口**（2026-10-17）：macOS 拒过一次就永远不会再弹
   * 授权框（TCC 没有"再问一次"的 API）—— 用户反悔的唯一正路是系统设置 →
   * 隐私与安全性 → 自动化 里把「访达」勾回来。这一条用系统给的直达 URL 把
   * **那一页**打开，省得他自己翻。依旧**不替他勾**。
   *
   * 前端**只在 `PERMISSION_DENIED` 那条错之后**摆这颗按钮 —— 授权没被拒过，
   * 那一页对他没有信息量。Windows / 其余平台没有"授权"这回事（`NOT_IMPLEMENTED`）。
   */
  openAutomationSettings(): Promise<void>

  /**
   * 首页「复制后处理脚本」要的**那一整条命令**（**当前平台、当前实例、真实落点**现拼）。
   *
   * 两段真值都从这一侧来：可执行物 = 本程序自己（`current_exe()`：开发态是
   * `target/debug/…`，装好了就是安装位置）；预设 = 那台机器上真实的绝对落点
   * （本地那份优先，其次是云端那份——**必须真在盘上**）。前端不拼任何一段路径：
   * 早先首页硬编码的是**另一个仓库**的 Windows exe，Mac 上复制出去的命令必然跑不通。
   *
   * 给不出来（还没下载 / 目录里没有这个组合 / 拿不到自己的路径）时不抛错：
   * `ready: false` + `reason` —— 界面照摆按钮、照说原因，**不抄一条假命令**
   * （贴进切片器就是 `Error code: 2`）。
   *
   * `path`（可选）= **被点名的那一份**在用户根里的相对落点（`presets-local/…`）。
   * 预设页右键某一行要抄的就是**那一行**：同一机型版本下可能有好几份副本，
   * 只按 `(machineId, versionId)` 寻址会落到「正在使用」的那份上（2026-10-19 修）。
   * 首页 / 标题栏字标不传它 —— 它们指的是"当前使用的那一套"，由后端按归属挑。
   */
  getPostProcessCommand(
    machineId: string,
    versionId: string,
    path?: string,
  ): Promise<PostProcessCommand>

  /* ——— 后处理钩子那一趟：看它 / 停它 / 答它那一问（切在 `app/postprocess/`）——— */

  /**
   * 钩子那一趟的全量快照。`null` = **现在没有在跑后处理**（普通模式，从桌面图标起来的）
   * —— 不是错误：界面据此不摆那一屏模态框。
   */
  getPostProcessRun(): Promise<PostProcessRun | null>

  /**
   * 界面那颗「停止」：请求取消。
   *
   * 取消落在内核的**步骤边界**上（写盘前还有最后一道），所以按下去之后原文件不会被写坏；
   * 顺带把"等你回答机型那一问"的等待也放开（按停止 = 不跑）。
   */
  cancelPostProcess(): Promise<void>

  /** 回答「机型不匹配还跑不跑」：`keep = true` 继续，`false` 停下 */
  /**
   * 回答屏幕上那一问（机型 / 校准方式 …）：`questionKind` 对得上那一问、`id` 是按钮 id
   * （`keep` / `stop` / `print` / `direct`）。对不上 = 那一屏已经过期，后端会拒绝。
   */
  answerPostProcess(questionKind: string, id: string): Promise<void>

  /**
   * 把快照里那份问句**再推一次事件**（只给首帧竞态收尾用）。
   *
   * 见 `PostProcessRun.questionEmitted`：界面读完快照发现"有一问、但自己从没收到过事件"
   * 时喊一次。不动任何状态 —— 问句还在后端手里，钩子那边还在等。
   */
  replayPostProcessQuestion(): Promise<void>

  /**
   * **这一趟整份重推一遍**（2026-10-18，首帧竞态的另一半）。
   *
   * 界面读完快照、订阅也挂上之后仍然什么都没看见 ⇒ `postprocess-started` 落在
   * 「读快照」与「订阅挂上」那条缝里了（抬窗在 `Hello` 里、还排在 emit 前面，
   * 所以"窗口被抬起来了"不代表事件送到了）。喊一声把开场 / 事实 / 进度 / 那一问 /
   * 结论按序重推。不动任何状态。
   */
  replayPostProcessRun(): Promise<void>

  /* ——— 机型与文件（预设页 / 参数页要读的） ——— */

  /** 机型目录：品牌 → 机型 → 版本。客户端画三级选择用 */
  getMachines(): Promise<Machine[]>

  /**
   * 某个「机型:版本」下配了哪些文件。
   * `null` = 后端没有这个组合（不是空），与 `VersionFiles.files` 空数组是两件事。
   */
  getVersionFiles(machineId: string, versionId: string): Promise<VersionFiles | null>

  /** 本机预设目录里**已经有的**文件名（云端那一批，固定演示集合） */
  getLocalFiles(): Promise<string[]>

  /**
   * **用户自己的预设文件**（用户线，`<appDataDir>/user/presets-local/`）。
   *
   * 盘就是底账（扫盘）：用户随时可能在 Finder 里改这个目录，所以没有账本可记。
   * 一份都没有 = 空数组，**不是错误**。
   *
   * 每一份都带上**血统**（如果它带着 `# based_on*` 三行）与它对应的判定
   * （[`BasedOn`]）—— 那是"云端换版了、你这份还是基于旧版"这件事的判据。
   */
  getLocalPresetFiles(): Promise<LocalPresetFile[]>

  /**
   * **用户改过的备注**整本（副标题覆盖账）。**只有本地表的副标题读它** ——
   * 云端表只读工作台写的那句（本地改动不许影响云端显示，2026-10-07 作者定）。
   * 键 = 文件身份（交付预设行是 `catalog.path`，用户线是相对用户根的路径）；
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
  setLocalPresetMachineVersion(
    path: string,
    machineId: string,
    versionId: string,
  ): Promise<void>

  /**
   * 读用户自己那份的正文。**只认 `presets-local/`**（入参是 [`getLocalPresetFiles`] 给的路径），
   * 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来。
   */
  readLocalPresetText(path: string): Promise<string>

  /**
   * **开始改一份预设**：把正文复制进临时文件，**原件一动不动**。
   *
   * 两条线走同一个入口（`origin` 缺省 `preset`，与 [`applyActivePreset`] 同一形状）：
   * 云端线认**文件名**（目录的键），用户线认**路径**（用户目录里可以自己分文件夹）。
   *
   * 前置都在后端拦：只改 MKP 预设（TOML）；云端线还要求盘上这份与目录逐字节一致
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
   * 别的行，只把那一处换掉。两条都只动草稿正文，云端原件与下载区全程不碰。
   *
   * `paramKey` 是**注册表主键**（如 `toolhead.offset.x`），不是 TOML 字段名 ——
   * 定位与取值形态（数字 / 布尔 / 字符串）归后端按字段定义决定，前端不拼 TOML 字面量。
   */
  patchPresetDraft(paramKey: string, value: string): Promise<void>

  /**
   * **一次手势里的多项改动，一次落地**（2026-10-21）。
   *
   * 拖一下擦料塔会同时改 X 与 Y。分两条 `patchPresetDraft` 发的话，后端每条都是
   * 「读草稿 → 只改自己那一项 → 写回」，两条交错时后写的那条基于旧正文，把前一条冲掉
   * —— 真机踩过：拖到 (101, 85)，保存后重进只剩 Y=85、X 还是出厂 20
   * （修改历史里两项都记着，因为内存里是对的，盘上丢了一半）。
   *
   * 所以**一次手势 = 一条命令**：读一次正文、叠完所有项、写一次。
   * 后端是"任何一项失败整批不落"，与 `apply` 一次一条撤销同一条语义。
   */
  patchPresetDraftBatch(edits: ParamEdit[]): Promise<void>

  /** 放弃这次编辑：丢掉临时文件（幂等；云端原件与下载区全程没被碰过，所以它天生安全） */
  discardPresetDraft(): Promise<void>

  /**
   * **草稿里改过的那几项参数**（`param_key → 草稿现在的值`，控件形态；没有草稿 = 空表）。
   *
   * 给**冷启动**接回草稿层用（2026-10-19）。后端那份草稿活在 `run/app-state.json`，
   * 而界面那一层（`useParams` 的 `draft`）是进程内存 —— 关掉软件再打开，
   * `beginPresetEdit` 虽然答 `reused`（还是这一份的草稿），但"改了哪几项"没有别的来源：
   * 不补这一条，参数弹窗就会"盘上有草稿、摘要说脏，界面一个改动都看不见"。
   *
   * 判据是**参数值**，不是字节：注释 / 空行 / 血统那几行不算改动。
   */
  readPresetDraftChanges(): Promise<Record<string, string>>

  /**
   * **读这一份的修改历史**（一份文件一本；没改过 = 空本，不是错误）。
   * `origin` 与 `beginPresetEdit` 同一套：用户线认 `path`、云端线认 `fileName`。
   */
  getPresetHistory(origin: ActiveOrigin, id: string): Promise<PresetHistory>

  /**
   * **整本写回**这一份的修改历史（改一条 / 撤销一条 / 保存一次之后）。
   * 给的就是界面眼里整本账（含批号水位），后端照写 —— 空本会被删掉文件。
   */
  putPresetHistory(origin: ActiveOrigin, id: string, history: PresetHistory): Promise<void>

  /** **清空**这一份的修改历史（抽屉右上那颗；二次确认在界面那侧做） */
  clearPresetHistory(origin: ActiveOrigin, id: string): Promise<void>

  /**
   * **「哪份预设用得最多」的本机学习账**（2026-10-17）。
   *
   * 写在"应用预设"那一步（后端 `apply_active_preset` 成功 +1），所以这里只有读。
   * 界面拿它给选择器抽屉排序：**同一版本里用得多的排前面**。
   *
   * ★ 这份数据住在 `<appDataDir>/run/usage.json`，**不进备份**（换设备就从零学）。
   * ★ 引擎：`local:<用户根相对路径>` / `cloud:<内部根相对路径>` —— 与行上的 `ref` 同口径。
   */
  getPresetUsage(): Promise<PresetUsageEntry[]>

  /**
   * **打一份备份**（设置页）：把这台机器的家当收进一个 zip，落在 `destDir` 里。
   *
   * 备份里有的：应用设置（`run/app-state.json`）、目录（`catalog.json`）、
   * **全部用户预设 toml（含导入的）**、**修改历史**、云端基准与原文镜像、执行归档。
   * 备份里没有的：云端下载区原件 / 换代归档 / 日志 / 导出产物 —— 那些都能重新拿到，
   * 备份要的是"丢了就再也回不来的东西"。
   *
   * 文件名由后端定（`SupportEase-备份-<时刻>.zip`），于是同一天打几份也不会互相盖掉。
   */
  createBackup(destDir: string): Promise<BackupSummary>

  /**
   * **从一份备份恢复**（设置页）：**全量覆盖** —— 备份里有的一律覆盖，
   * 被管的那几棵树里备份没有的删掉（= 回到"打那份备份那一刻"）。
   *
   * ★ 不做选择性恢复：同名文件"选哪一份"没有天然答案，成熟方案要么整机快照
   * （靠多份备份按时间选拿细粒度），要么细粒度同步（要求每个文件有稳定 id 与合并语义）。
   * 我们走前者。
   *
   * 恢复完界面要**整窗重载**（盘上的数据换过了，内存里的都不算数）。
   */
  restoreBackup(zipPath: string): Promise<BackupSummary>

  /**
   * **把这一份存进用户根**，然后丢掉草稿。存到哪由**这份草稿改的是哪一份**决定：
   * 云端线**另存**成 `presets-local/<原名>（已修改）<后缀>`（原件全程不动）；
   * 用户线**写回它自己** —— 同一个路径、同一份文件，不产生第二份（第八层）。
   *
   * 云端线写下去的正文 = 草稿 + **文件头三行血统**（`# based_on` / `# based_on_release_time` /
   * `# based_on_sha256`）—— 于是这份文件**拷到哪台电脑上都说得清自己从哪来、基于哪一版**。
   * 那是"文件本身的信息"（随文件走），所以**不**另写进 `run/`（第七层作者定的原则）；
   * 用户线写回时那三行**照抄原来那三行**（出处没变）。
   *
   * 不碰云端原件、不碰下载区、也**不碰使用中指针**（生效走 [`applyActivePreset`]）。
   * 云端线再存一次就是覆盖它自己（`replaced` 说出来这次是不是盖掉了上一次那份）；
   * 用户线写的就是原来那一份所在的位置（`replaced` 恒为 true）。
   */
  commitPresetDraft(): Promise<CommittedDraft>

  /**
   * **参数草稿的摘要**（预设列表行上的「未保存」标记、浮动提示、"要不要重开"三处共用）。
   *
   * **只摘要不正文**、**没有 null 之外的空值**：现在没有草稿就是 `null`。
   * 改入口还是 [`putPresetDraft`] / [`patchPresetDraft`] / [`commitPresetDraft`] /
   * [`discardPresetDraft`] 四条 —— 这条只是给"列表上要不要标一下"用的。
   */
  getPresetDraftSummary(): Promise<PresetDraftSummary | null>

  /**
   * **开始改一份用户预设的正文**（预设页「编辑文件」）。与上面参数页那条链（`beginPresetEdit`）
   * 不是一回事 —— 这里返回的正文**带着这一份的血统三行**（只在这份文件上有效），
   * 且要带着 **这份文件的 sha256**（保存时比一下有没有被别人抢先改）。
   */
  beginTextEdit(fileName: string, path: string): Promise<PresetTextDraft>

  /**
   * **这份用户文件的原文**（它最初落到盘上时的那一份）——「编辑文件」模态框那颗
   * **「恢复默认值」**读的就是它（2026-10-16 作者：「编辑文件这个也可以有个按钮……
   * 恢复默认值……对于用户导进来的，可能需要你去做一种备份」）。
   *
   * 来源 = `baseline-local/` 的路径镜像：下载 / 导入 / 复制 / 另存的那几份在**落盘时**
   * 存过；老文件在**第一次被我们改写**（编辑器保存 / 参数写回 / 校准 / 归属机型）之前补上。
   * 都没有 ⇒ `null` —— 界面把那颗按钮禁用（不猜、不编一个"默认"出来）。
   */
  readLocalOriginText(path: string): Promise<string | null>

  /**
   * 把改动写进临时文件（编辑框边改边存）。**只动正文**，[`beginTextEdit`] 那一刻的其它不变。
   * 调用前必须有正在改的那一份正文（页面先调 [`beginTextEdit`]）。
   */
  putTextDraft(text: string): Promise<void>

  /**
   * **保存正文**：把正文草稿**原子地**写回它自己 —— 一次替换**整份文件**
   * （先写临时文件、再换上去）。写完丢掉正文草稿，并把参数页那份草稿一并作废
   * （它基于的旧字节已经不在）。
   */
  commitTextDraft(): Promise<CommittedDraft>

  /**
   * **实时校验一份还没保存的正文**（2026-10-21，作者：「它是实时的吗？……我希望它
   * 那种是比较实时的检测」）。
   *
   * 编辑框每停一下打一次：红条跟着字走 —— 改回合法、把被粘的行拆开，红条立刻消失。
   * `path` 用来找它最初的原文镜像（缺参数是相对那份比的）。
   */
  checkPresetText(path: string, text: string): Promise<PresetTextCheck>

  /** 放弃这次编辑：丢掉临时文件。**天然幂等**（有没有草稿都不会报错） */
  discardTextDraft(): Promise<void>

  /**
   * **身份账本**：「我这边这一份」和「云端那一版」的对应关系。
   *
   * 预设页判「下载区里这一份我有没有一份」的唯一依据 —— 也是「更新」找**要更新的是哪一份**的依据。
   * **只有云端的那一版活着的时候才进**（那一版已被移出版次的、**外来 / 自己写的**一律不进）。
   */
  getLocalPresetIndex(): Promise<LocalPresetIdentity[]>

  /**
   * **重命名一份用户文件**（第十层）：只改名字，**字节一个不动** —— 内容、那三行血统、
   * TOML 都不重写；改完还是同一份 Preset。只换名字不换目录（`presets-local/` 那一格内）；
   * 新名字不许带路径分隔符、不许空、**后缀保持原样**（改名不改它是哪一类）；
   * 落点已经有东西就拒绝（**不覆盖**）。
   *
   * 后端还会把两本状态账跟着改：**正指着它的使用中指针**（路径与文件名换成新的，指纹原样）
   * 与**这一份的草稿**（用户线认路径，「接着上次改」不接丢）。失败原话冒上来。
   */
  renameLocalPreset(path: string, newName: string): Promise<LocalFileIdentity>

  /**
   * **另存为一份新的**（第十一层）：把我自己那一份**按字节**复制成同一格里另一份新的用户文件。
   *
   * **本地文件 → 本地文件**：原文件一个字节不动；内容与那三行 `based_on*` 血统
   * **原样带过去**（来源已经是用户文件，不重算血统 —— 重算会把"从哪一版云端派生"说错）。
   * 新名字过同一套门槛、落点已有东西就拒绝（**不覆盖、也不自动改名** —— 名字由用户自己换）。
   *
   * **一个状态都不碰**：不改使用中指针、不迁移草稿、不建草稿、不进 archive ——
   * 新文件从诞生起就是独立的一份（之后能独立编辑 / 改名 / 删除 / 应用）。
   */
  copyLocalPreset(path: string, newName: string): Promise<LocalFileIdentity>

  /**
   * **这台机型 / 这个版本，本地那份在哪**（校准页：初值从它读、保存写它）。
   *
   * 找法（有先后）：底账正用着的那一份（origin=local 且归属匹配）> 用户目录里
   * 第一份匹配的。归属 = 文件头 `# machine:` / `# variant:` 归一化后回落血统
   * （与用户线列表同一套口径）。
   *
   * 一份都没有 / 都读不出来 ⇒ `null`：校准页照旧显示云端默认值，保存被拦。
   */
  getLocalCopyFor(machineId: string, versionId: string): Promise<LocalCopyCalibration | null>

  /* ---------- 第十二层：通用文件导入入口（Preset 只是第一个消费者）---------- */

  /**
   * **导入第一段：看落点**（拖拽与文件选择器都走这里）。只检查、不动盘；
   * 重名（`collision`）只如实说，**不自动改名** —— 名字由用户在界面上改。
   * `rejected` 带原因（现在只收 `.toml` 预设；ZIP / 备份包还没有认领它的导入器）。
   */
  stageImport(sources: string[]): Promise<StagedImport[]>

  /**
   * **导入第二段：真的复制进 `presets-local/`**。逐份独立（一份出错不拖累别人）：
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
   * 只认「本地文件」（照旧过用户根那两道闸）；**读不出来的那份也能显示**
   * （文件管理同族：它只是一份文件，打开文件夹不吃内容）；**不改任何状态**
   * （使用中指针 / 草稿 / archive 一个都不碰）。文件被外面删了会如实报找不到。
   */
  revealInFolder(path: string): Promise<void>

  /**
   * 打开**我们自己的落点**（下载区 `assets/bbs/…` 里那一份）所在的文件夹，并选中它。
   *
   * 与 [`revealInFolder`] 是**两个地方**，别混：那一条只认 `presets-local/`（你的文件），
   * 这一条认**目录登记**（交付区里我们发的那一份）。只收文件名，路径不由前端拼；
   * 盘上还没有这份 → 照实报告"先下载"（不打开空文件夹）。
   */
  revealDeliveredFile(fileName: string): Promise<void>

  /**
   * 打开**切片器目录里那份**（`user/<账号>/process/…`）所在的文件夹，并选中它。
   *
   * 那是「复制」过去的落点（切片器的生效就是那一手），与我们自己的下载区**不是一处** ——
   * 用户在 BBS 里改的也是那一份，所以「去文件夹里看看」得能分别打开两处。
   * 还没复制过去 → 照实报告"先复制"。
   */
  revealSlicerFile(fileName: string): Promise<void>

  /**
   * **删除一份用户文件**（第十层）：**真删除** —— 没有垃圾桶、也没有归档
   * （`archive/` 是云端版本生命周期的一部分；用户自己删自己的文件就结束）。
   *
   * 两道硬闸在后端：**正在使用的不许删**（删了「使用中」就指向一份不存在的文件）、
   * **还有没保存的草稿的不许删**（删了草稿就永远存不回去）。失败原话冒上来。
   */
  deleteLocalPreset(path: string): Promise<void>

  /**
   * 已经复制到切片器目录的那些（**文件名**，与下载 / 应用 / 读正文同一个取用口径）。
   * 盘就是底账：切片器的用户配置目录里真有这份文件才算「已复制」。
   */
  getSlicerCopied(): Promise<string[]>

  /**
   * 把一份切片器配置复制进切片器自己的用户配置目录 —— **切片器的「生效」就是这一手**
   * （MKP 预设的生效是「启用」，两回事）。
   *
   * 真机上会落盘。两条真实的失败照实抛：不是切片器配置 / 本机还没有那份字节（先「下载」）。
   *
   * ★ **目标已有同名就换掉它**（2026-10-13 作者改判：「有相同的就覆盖吧」）——
   *   原来是拒的，理由是"那份可能被用户在切片器里改过"。覆盖走原子替换，切片器正开着
   *   也不会读到半个；返回值说清这次是**新建**还是**替换**（提示条按它说话）。
   */
  copyToSlicer(fileName: string): Promise<CopyToSlicerOutcome>

  /**
   * 本机 BBS 目录里现在有哪几份工艺预设（用户层 + 系统层，一次读全）—— 工艺页左边
   * 那个「本机工艺」档的清单。
   *
   * 为什么只能现读：那份清单住在**用户的切片器里**（他刚在 BBS 里另存一份，回来
   * 重扫就该看见），而本仓不打包那 285 份预设快照（作者裁决 §6-2）。
   *
   * 读不到不是错误：`available: false` + `reason` 是**事实**（没装 / 没跑过），
   * 界面照实说 —— 那与"读失败"是两件事。
   */
  getBbsPresets(): Promise<BbsLocalTree>

  /**
   * 读其中一份的**原文**（JSON 文本）。解析交给前端：坏文件要由前端照实报错，
   * 不在这里被吞掉。
   *
   * 路径不由前端拼：只能给 `scope` + `uid` + `file` —— `file` 必须是**单个 `.json`
   * 文件名**（不接受任何路径），`uid` 必须在扫到的账号目录里。
   */
  readBbsPreset(scope: 'user' | 'system', uid: string | null, file: string): Promise<string>

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
   * 一份目录文件的**云端下载链接**（「复制链接」复制的就是它）。
   *
   * 寻址与下载管道同一个出口算 —— 复制到的链接就是下载时取的那个地址。
   * 没配数据源时照实报错，不拼一个猜的 URL。
   */
  getFileUrl(fileName: string): Promise<string>

  /**
   * **后处理执行报告与历史**（报告页）：`mkp-ssr` 钩子每跑一次落一份 `_meta.json`，
   * 这里把它们读成摘要列表（新在前）。空列表 = 还没有执行记录（不是错误）；
   * 读不出来的那几条**照进列表**（`ok=false` + 原因），不许消失。
   */
  getReportList(): Promise<ReportSummary[]>

  /** 一条执行记录的详情（展开那一屏）。`id` 是 [`ReportSummary::id`] */
  getReportDetail(id: string): Promise<ReportDetail>

  /**
   * 「导出执行报告」的计划（三件套的文件名 + 默认落点 + 缺哪几份）。
   * 点开导出框时读一次；`id` 不认识 ⇒ `NOT_FOUND`。
   */
  getReportExportPlan(id: string): Promise<ReportExportPlan>

  /**
   * **把这一条的三件套导出去**（报告页唯一的写口）：复制到 `dir/<基名>/` 下，一字不改。
   *
   * `dir` 是**父目录**（默认值来自 [`MkpApi.getReportExportPlan`]），必须是绝对路径 ——
   * 相对路径会落到进程的当前目录，那不是用户点过的任何一个地方。
   * 返回真正落盘的路径。三种拒绝都是照实抛，不吞：
   * 目标里已经有同名的那一份（**整批不落**，不覆盖）、保存位置就是归档自己的目录、
   * 这条记录一份文件都不在盘上。
   */
  exportReportArchive(id: string, dir: string): Promise<string[]>

  /**
   * **打开这次导出的那个文件夹**（导出框上那颗「打开位置」）。
   *
   * 导出成功之后用户要做的事只剩一件：去那个文件夹里看一眼 —— 所以那一刻
   * 「更改位置」换成它。`dir` 与 [`MkpApi.exportReportArchive`] 收到的是**同一个**
   * 父目录，基名由后端算；**只有那个目录真在**才开（还没导出过就如实报找不到），
   * 也绝不把一个文件交给系统（那是"启动它"，不是"打开位置"）。
   */
  revealReportExport(id: string, dir: string): Promise<void>

  /**
   * 选一个目录（导出框那颗「更改位置」）。走系统目录选择器，`current` 是打开时停在的
   * 那个目录。**取消给 `null`，不是错误**。
   */
  pickExportDir(current: string): Promise<string | null>

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
   * 云端地址由构建方注入（Bootstrap 那一刀），这一格留的是开发 / 排查的后门。
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
   * 当前主题偏好。`null` = **还没选过**（界面按"跟随系统"）。
   *
   * 与 `'system'` 意思不同：那是用户明确点了跟随系统 —— 界面上两者长得一样
   * （都跟随系统），但盘上分得开，将来改出厂默认档时还能重新解释。
   */
  getTheme(): Promise<ThemePref | null>

  /**
   * 换主题档位：**立即生效并落盘**（写 `run/app-state.json` 的 `theme` 格）。
   * 写完后端广播一次 AppState 变更 —— 界面不用"切完手动刷新"。
   */
  setTheme(theme: ThemePref): Promise<ThemePref>

  /**
   * 把**主窗的原生背景色**报给原生（2026-10-22）—— 参数是 CSS 颜色（`#rrggbb`）。
   *
   * 治的是 **macOS 上拖拽改窗口大小（live resize）时，右侧/底部露出一条白边**：
   *
   * - 这是 WebKit 的已知呈现 bug（`WebKit#323132` / `tauri#14288`）：窗口 bounds 先变，
   *   WebContent 进程的图层树后提交，中间那几帧盖不满新 bounds 的边；
   *   Tauri 官方已表态这个场景 wontfix，报告里那批"强推 frame / 保留内容"的手段实测都无效。
   * - 唯一可行的是**把露出的那层染成同色**。露的是 WKWebView 自己的白
   *   （`drawsBackground` 默认 true），所以必须走 `set_background_color`
   *   （它同时关掉 webview 的白底、设 `underPageBackgroundColor`、设窗口底色）。
   *
   * ★ 颜色**不存第二个源**：调用方读 `--bg-2` 传上来，真值仍然只有 `tokens.css`。
   * 失败不该挡任何事（外观兜底），调用方吞掉即可。
   */
  setWindowBackdrop(color: string): Promise<void>

  /**
   * 后处理记录的**自动清理**开着吗（2026-10-13）。
   *
   * 回的是**解析后的 `boolean`**：盘上那一格"没选过"是 `null`（真机上等于开着），
   * 而"没选过怎么解释"是后端的事 —— 界面不自己拿 null 去猜默认值
   * （猜出来的那一档迟早与真正执行清理的那一侧漂开）。
   */
  getAutoCleanHistory(): Promise<boolean>

  /**
   * 开关自动清理：**立即生效并落盘**（写 `run/app-state.json` 的 `autoCleanHistory` 格）。
   *
   * 打开 = 下一次检查（启动 / 后处理跑完）就按双限瘦身；关掉 = 旧记录从此不再被清
   * （**已经清掉的不会回来**）。清理全程静默，界面上不会有任何提示。
   */
  setAutoCleanHistory(enabled: boolean): Promise<boolean>

  /**
   * **复制出来的那一份跟不跟云端**（2026-10-20；出厂 = 关）。
   *
   * 回的是**解析后的 `boolean`**：盘上那一格"没选过"是 `null`，"没选过怎么解释"
   * 是后端的事 —— 界面不自己拿 null 去猜（与自动清理那一条同一条口径）。
   *
   * 关 ⇒ 复制件是独立的一份：预设页云端行不因它显示「有新版 / 更新」，
   * 参数页打开它时整段「云端更新」不画。
   */
  getCopiesFollowCloud(): Promise<boolean>

  /**
   * 开关「复制出来的也跟着云端更新」：**立即生效并落盘**
   * （写 `run/app-state.json` 的 `presets.copiesFollowCloud` 格）。
   *
   * 开 / 关会**立刻改变**云端行的三态与那颗「更新」—— 那一态由身份账本算，
   * 而账本的出口正看这一格，所以写完后端广播一次 AppState 变更。
   */
  setCopiesFollowCloud(enabled: boolean): Promise<boolean>

  /**
   * 设置页「后处理模式 / 校准与补偿 / 热床预涂胶」那几块的当前值（2026-10-17）。
   *
   * 回的是**解析后**的值（盘上 `null` 已经解成出厂档、量程已经夹住）——
   * 界面不自己拿 null 去猜默认值，与自动清理那一条同一条口径。
   *
   * `raw` 是盘上原样那一格：界面用它判断"这一项动过没有"。
   * `limits` 是后端那边量的量程，界面**不另写一份**在 TS 里。
   */
  getPostprocPrefs(): Promise<PostprocPrefsView>

  /**
   * 写后处理偏好：**整格覆盖**（界面手里就有全量，改一项传全量）。
   *
   * 传 `null`（或省略项）进来 = 那一项回到出厂档 —— 这是"整格覆盖"才能表达的。
   */
  setPostprocPrefs(prefs: PostprocPrefsWire): Promise<{ ok: boolean }>

  /**
   * **重置后处理状态**（2026-10-17）：治"钩子还在等答复、界面已经走了"留下的那一屏。
   *
   * 先让钩子那头别再等，再清界面这一侧的共享状态。回一句话 ——
   * 设置页原样摆出来，于是"点了没反应"与"本来就没东西"分得开。
   */
  resetPostProcessState(): Promise<string>

  /**
   * 打开**用户预设目录**（`presets-local/`）并把窗口喊到前台（2026-10-17）。
   *
   * 回的是那个目录的绝对路径 —— 界面上那一行可以把路径显示出来（用户要知道"东西放哪"）。
   * 目录不存在时后端会**先建一个空的**：新建的机器上它本来就该在，报"找不到"只会让人以为坏了。
   */
  openPresetFolder(): Promise<string>

  /**
   * **卸载应用**（2026-10-17）：删程序本体，然后退出。
   *
   * ★ 不可逆，且**用户数据一个不碰**（预设 / 执行记录 / 备份原样留着）。
   * ★ 后端会先把 `.app` 删掉再 `exit`：删完这一下进程就没了。
   *   Windows 上交给一个 detach 的 `cmd` 等我们退出后再删（正在跑的 exe 删不掉）。
   * ★ 失败**原样报错**（detail 里带 `E_SYS_UNINSTALL_001`）——
   *   静默失败会让用户以为卸了，结果切片器下次还在调它。
   *
   * 界面必须**两段式确认**之后再调它（见设置页那个 3 秒回退的按钮）。
   */
  uninstall(): Promise<void>

  /**
   * 当前界面语言偏好（2026-10-25）。`null` = **还没选过** ⇒ 按"跟随系统"。
   *
   * ★ 只有文案跟着它换；**错误码 `code` 永远不变**（每句话必有码那条纪律）。
   */
  getLocale(): Promise<LocalePref | null>

  /**
   * 换界面语言：`system` / `zh-CN` / `en`。
   *
   * 写成功后后端广播一次 AppState，前端字典据此换一套文案 —— 不需要重启。
   * 认不出的档整次拒绝（不静默回落成中文）。
   */
  setLocale(locale: LocalePref): Promise<LocalePref>

  /**
   * 窗口这一族的两格账（2026-10-17）：置顶档位 + 在不在窄窗。
   *
   * 一次读齐是有意的 —— 它们同住 `run/app-state.json`、同一次广播，
   * 分两次读只会让"刚点完窄窗、置顶那格还是旧的"这种时序缝隙变多。
   */
  getWindowPrefs(): Promise<WindowPrefs>

  /**
   * 换置顶档位：**先落盘、再设窗口标志**，成功后广播一次。
   *
   * `onBbs` = 现在这一页是不是工艺页（`page` 档要用它算"该不该真置顶"）。
   * 页签的真值在前端，后端不复制一份。
   */
  setPin(pin: PinPref, onBbs: boolean): Promise<PinPref>

  /**
   * 切页之后重算一次"该不该置顶"（**不写盘、不广播**）。
   *
   * 为什么与 `setPin` 分开：切页是高频动作、且档位没变 —— 走写盘那条会把前端
   * 绕回自己的 effect 里。这一条只把已经选好的档落到窗口上。
   */
  applyPin(onBbs: boolean): Promise<boolean>

  /**
   * 进 / 出窄窗：宽 600、高 = 当前屏可用高 90%、贴右缘。
   *
   * 退出时回到**上一次手动摆放的位置与尺寸**（用户自己拖过 / 拉过的最后一次）；
   * 从没摆放过就是出厂尺寸居中。失败时后端把 `compact` 那格回滚 ——
   * 盘上不会出现"说着在窄窗、窗口其实没动"的状态。
   */
  setCompact(compact: boolean): Promise<boolean>

  /**
   * 「板子类型」控件此刻在屏幕上的哪一块（**只读**，2026-10-15）。
   *
   * 改了参数（XYZ 偏移、涂胶…）之后，切片器**不会**认为要重切 —— 它那份参数里
   * 根本没有这些字段（见 `docs/INVESTIGATION-BBS-INVALIDATE.md`）。我们也没有
   * 外部入口去让它重算（那份文档 §5/§7）。能做的是**把那一步指给用户看**：
   * 在他自己的切片器窗口上画个箭头，让他换一下板子类型、再换回来。
   *
   * 这一条只回答"在哪"，**不模拟点击、不改切片器任何参数**。
   * `ready: false` 时一定带 `reason`（没权限 / 切片器没在跑 / 界面换了），
   * **不给一组假的坐标** —— 指错位置比不指更糟。
   */
  locateBedTypeControl(): Promise<GuideTarget>

  /**
   * 把引导卡片摆出去（贴着切片器里「板子类型」那个控件）。
   *
   * **只有一步**（2026-10-15 作者定）：卡片上把"换成另一个、再换回原来那个"整件事
   * 一次说清就够 —— 起作用的是"换成不同值"那一次（切片器源码里那个 `if (old != new)`），
   * 换回来只是把用户的板型还原。原来那个「下一步」要**关掉窗口重建**，中间那一瞬
   * 卡片没了，比不引导还差。
   *
   * 卡片只占一小块、贴着目标放 —— 它活着的时候用户照样能去点切片器。
   * 指不了时与 [`locateBedTypeControl`] 同一条规矩：`ready: false` + `reason`。
   */
  guideShow(): Promise<GuideTarget>

  /**
   * 收起引导卡片。**没开着也算成功** —— 这是一句"我现在不需要它了"，
   * 不是"去关一个一定存在的窗口"。
   */
  guideClose(): Promise<void>

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
   * 归档区里有什么：云端文件换版本时**被换下来的那些旧版本**。
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
   * **删除本机下载区里那份交付预设文件**（作者裁决 2026-10-06：一切皆可删）。
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
   * 「使用这一份」。**两条线共用这一个入口**（第七层）：交付预设文件与用户自己那份
   * 都是真的 Preset —— "只读"是文件归属的属性，不是"能不能被使用"的属性。
   *
   * ```text
   * cloud  目录里有这一份 + 盘上字节与目录登记的当前版本逐字节一致
   *           （没下载 / 被改过 / 是旧版本 —— 都 reject，不会应用半份）
   * local      落点必须在 presets-local/ 那一格里 + 盘上真有这一份 + 是一份 TOML 预设
   * ```
   *
   * `origin` 缺省是 `cloud`（老调用点不用改）；`path` 只有用户线要给
   * （用户目录里可以自己分文件夹，所以认的是路径，不是文件名）。
   */
    applyActivePreset(
      fileName: string,
      origin?: ActiveOrigin,
      path?: string,
    ): Promise<ActivePreset>

  /**
   * **把云端这一份预设取到本机** —— 云端表那两个动作（「下载」/「更新」）共用的那一条。
   *
   * 它做的两件事，都幂等：
   *
   * ```text
   *   ① 云端当前版的字节   没有（或盘上那份与目录对不上）→ 取回（同一条下载管道，旧份进 archive）
   *   ② 本地工作副本       presets-local/<原名>.toml 还没有 → 落一份（云端原文 + 血统三行）
   *                       已经在了 → **一个字节都不动**（我改过的东西不许被云端原件顶掉）
   * ```
   *
   * **不碰「当前使用」**：下载 / 更新是"把云端的取到我机器上"，使用是"把哪一份设为生效"——
   * 后者由本地表那颗按钮走 {@link applyPreset}。云端原件留在下载区，是内部数据。
   */
  /**
   * `renameIfTaken`（2026-10-10）：本机**已经有一份同名的**时怎么办 ——
   * 默认（不传 / `false`）**不动它**，返回 `takenBy` + `suggestedName`，由界面问用户；
   * 用户同意导入时传 `true`，这次**自动改一个没被占的名字**落一份新的
   * （原来那份一个字节不动；新那份的身份记的来源仍是云端这个条目）。
   */
  fetchCloudPreset(
    fileName: string,
    renameIfTaken?: boolean,
  ): Promise<FetchCloudResult>

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

  /* ---------- baseline + 对比台（2026-10-08） ---------- */

  /**
   * **云端版本列表**：这个云端预设云端登记过哪几版、哪几版这一机已经下过。
   *
   * 两条判据：云端登记过哪几版 = 当前目录 + 版本链（按内容摘要去重）；
   * 下过没有 = 隐藏 baseline 里有没有那一版的摘要。
   *
   * ★ **它不是"过时判定"**：不读用户预设的字节、不看血统，也**不会**说"你那份该更新了"。
   * `fileName` 给了就只返回那一个预设的版本。
   */
  getCloudVersions(fileName?: string | null): Promise<CloudVersion[]>

  /**
   * **读一份用户预设的参数**（对比台里的一列）。**只认 `presets-local/`**。
   *
   * 读不出来**不抛错**：返回空表 + `problem` 一句人话 —— 那一列如实说"这份读不出来"，
   * 而不是把整个对话框打掉。
   */
  readPresetParams(path: string): Promise<PresetParamValues>

  /**
   * **把改过的几项写回它自己那一份**（同一路径，**不产生第二份**）。
   *
   * 结构保真：逐项只换那个值（注释 / 键序 / inline table / 多行字面量 / 行尾一个字节不动），
   * 任何一项失败**整批不落**；空改动 / 算出来与原文一样**不写盘**。
   * baseline 与云端当前版永远不被写。
   */
  savePresetParams(path: string, edits: ParamEdit[]): Promise<void>

  /* ---------- 逐参数「云端更新」（2026-10-09） ---------- */

  /**
   * **这一份预设的云端更新账**：逐项给出「我 / 云端旧值 / 云端新值」与待处理标记。
   *
   * 云端那一版默认值住在**隐藏 baseline** 里（用户看不见）；"我上次处理到哪一版"
   * 记在逐参数决定账里 —— 所以云端连着发几版也能一次算清（比对的是
   * 「我处理到的那一版 → 当前版」，不需要云端提供中间版本）。
   *
   * `fetchMissing` = 允许为了拿到**云端当前版**的正文发一次网络请求。
   * ★ 2026-10-20：**打开某一份也不给 `true`** —— 作者裁定「没点更新之前不该知道
   * 逐项差异」。只有用户明确点了「更新 / 选择要更新的参数」那一刻才发这一次；
   * 其余时候 `cloudReady: false`，界面只说"云端有新版本"，不说改了哪几项。
   * 列表那种一屏几十行的读一律 `false`（启动零网络）。
   * 拿不到云端当前版正文时**不报错**：`cloudReady: false`，逐项新值给 `null`。
   *
   * **`null` = 这一份不认云端**（复制出来的那一份，且上面那个开关关着）——
   * 界面据此整段「云端更新」都不画，而不是画一个"零项待处理"
   * （那会让用户以为云端这一版没动过任何一项）。
   */
  getPresetParamSync(path: string, fetchMissing?: boolean): Promise<PresetParamSync | null>

  /**
   * **落一批决定**（采用 / 保持），回来的是落完之后的那一份总账。
   *
   * 采用会**结构保真地**改文件里那几项（注释 / 键序 / 行尾一个字节不动），
   * 保持一个字节不动；两者都把这一项的水位推到云端当前版。
   * 云端当前版正文不在本机时，采用会如实报错（不许"记了水位、值却没写"）。
   */
  applyPresetParamDecisions(
    path: string,
    decisions: ParamDecision[],
  ): Promise<PresetParamSync>
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
  /**
   * **稳定错误码**（`E_*_NNN` / `W_*_NNN`）—— 与 `code` 那个英文类别不是一回事。
   *
   * 2026-10-19（作者：「不用错误码吗」）：作者定的"每一条用户能读到的句子都必须有码"，
   * 以前只落在后处理/钩子那条链上，客户端的交付、读写、更新这些没码 —— 用户把错贴给 AI
   * 时无码可贴。现在码由后端**拼在 message 里**（`AppError::text`），这一格是给
   * 需要单独拿码的人（面包屑、日志对账）用的。
   *
   * 缺失 = 这一条还没迁（那时 message 里也没有码）。
   */
  messageCode?: string
  detail?: string
}

/** 运行时判断 reject 出来的东西是不是本结构。不是的话由 bridge 兜底成 INTERNAL */
export function isAppError(v: unknown): v is AppError {
  if (typeof v !== 'object' || v === null) return false
  const e = v as Record<string, unknown>
  return typeof e.code === 'string' && typeof e.message === 'string' && typeof e.traceId === 'string'
}
