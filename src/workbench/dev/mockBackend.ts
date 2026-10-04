/*
 * 测试用 IPC 桩 —— **只有显式 `?mock=1` 才装**（2026-10-02 定，见 main.tsx）。
 *
 * 它不是产品运行时的能力，是**探针的测试后端**：
 *
 *   · 正常人跑 `tauri:workbench:dev` / 生产构建 → **永远走真 Tauri IPC**，这段不装。
 *     以前是"dev 且没有 Tauri 就自动装"，于是浏览器里打开工作台看起来能跑，
 *     读的却是一份手写夹具 —— 与真盘上的 `workbench/bootstrap.json` 毫无关系，
 *     "我在终端改了配置界面没变"就是这么来的。
 *   · `scripts/probes/*.mjs` 跑在 vite preview + 浏览器里，本来就该显式说"我要桩"：
 *     启动时带上 `?mock=1`（探针自己加）。
 *
 * 为什么还需要它：参数台的验收判据是 C14 README 里那些**量过的数**
 * （行高 43/39、G-code 框 172×88、模态框 630×547、弃用四处落法……），
 * 量它们必须让页面带着数据渲染起来，而在浏览器里没有 Tauri `invoke`。
 * 这个桩把一份**手写夹具**（形状 = api.ts 的 DTO 手抄镜像）从
 * `window.__TAURI_INTERNALS__.invoke` 后面喂给界面，让验收可以在浏览器里做。
 *
 * 边界（重要）：
 *  - 这里**没有任何业务判定**搬进前端 —— 夹具里的弃用标记、blocked 句子、
 *    impact 等，都是照真后端的行为**手工预填/按同一规则现算的测试数据**，
 *    判据的唯一出处仍在 src-tauri；
 *  - **桩里的值与真盘无关**：`wb_set_bootstrap` 只写内存、不做 GitHub → raw 规范化
 *    （那是真后端 `dist::normalize_bootstrap_url` 的活，有单元测试钉着）——
 *    所以别拿浏览器里的行为当"配置生效了"的证据；
 *  - 写路径在这里是**内存里的假草稿**，落盘、快照、指纹这些一概没有。
 */

type Json = Record<string, unknown>

/* ---------- 词表（照抄 domain/wording.rs 的字面量） ---------- */

const WORDS: Json = {
  build: {
    built: { label: '已生成', explain: '磁盘里的生成物和当前配方一致，不用重新生成' },
    stale: { label: '待更新', explain: '磁盘里已经有上一版产物，配方改过了还没重新生成 —— 生成之后客户端才会拿到新的' },
    neverBuilt: { label: '未生成', explain: '还没生成过，客户端现在下载不到这一版' },
    noResources: { label: '配方文件缺失', explain: '这个版本没有配方文件 —— 这是异常，不是正常状态：版本一建出来就该带着配方文件。参数照样能看能改，但生成不了' },
  },
  artifact: {
    fresh: { label: '已生成', explain: '所有该有产物的版本都是最新的' },
    stale: { label: '待更新', explain: '有版本的配方改过了，产物还没跟上' },
    missing: { label: '未生成', explain: '一个产物都还没生成过' },
  },
  save: { saved: { label: '已保存', explain: null }, dirty: { label: '未保存', explain: null } },
  bbsAssign: {
    assigned: { label: '已分配', explain: '在某个套餐里，客户端装那个套餐就会拿到' },
    optional: { label: '可选', explain: '没进任何套餐，客户端能手动下' },
    archiveOnly: { label: '仅归档', explain: '客户端完全不知道这个文件存在' },
  },
  identity: {
    inBundle: { label: '进套餐', explain: '客户端首页按版本自动下载 —— 进套餐在套餐页管，资产库不直接设' },
    optional: { label: '可选', explain: '客户端预设页看得到，用户手动下载' },
    bundled: { label: '随包', explain: '构建期随程序包带进客户端，不下载不更新，页面上也不出现' },
    archiveOnly: { label: '仅归档', explain: '仓库里留着，客户端完全不消费' },
  },
  bbsSource: {
    own: { label: '本版本单独一份', explain: '这个版本自己挑的曲线清单' },
    inheritedFromMachine: { label: '跟随机型', explain: '用机型默认那份清单' },
  },
  origin: {
    factory: { label: '出厂', explain: '注册表里的默认值，全机型共用' },
    machine: { label: '机型', explain: '这台机器的基底，没自己写过的版本都跟着它' },
    version: { label: '版本', explain: '这个版本自己钉着的值' },
  },
  level: { machine: { label: '机型基底', explain: null }, version: { label: '版本', explain: null } },
  visibility: {
    menu: { label: '菜单可见', explain: '客户端的菜单里能看到、能下载' },
    archiveOnly: { label: '仅归档', explain: '文件在仓库里，但客户端看不到' },
  },
  bulkKind: {
    detaching: { label: '挂回继承', explain: null },
    changing: { label: '改值', explain: null },
    noChange: { label: '值不变', explain: null },
  },
  placeholder: {
    blank: '空',
    notApplicable: '不适用',
    undeclared: '未声明',
    unconfigured: '未配置',
    unsupported: '暂不支持',
  },
  disabled: {
    detachNothing: '这一层没有单独设过这一项，本来就是继承来的',
    detachReady: '删掉这一层的这一项，让它跟着上一层变',
    blockedByCondition: '上一项没打开，这一项现在不生效，所以不让改',
    notApplicable: '这台机型没有这一项，不是值为空',
    bulkRefusesGcode: 'G-code 不做批量：一段多行脚本被整体盖掉是不可逆的误操作，请逐列点开改',
    buildBlocked: '有阻断问题没解决，生成一定会出错',
    buildNothingToDo: '所有产物都和当前配方一致，没有要生成的',
    buildNoResources: '这一版没有可交付的产物，生成不出东西来',
    nothingToSave: '没有未保存的改动',
    nothingToUndo: '没有可以撤销的操作',
    notUndoable: '删除和生成记录不进撤销栈；删掉的版本在回收站里',
    deprecatedWriteBlocked: '已弃用，不能改（上游已标记）',
    deleteAssetInUse: '还有套餐装着它，或还有机型把它当图 / 图标用 —— 先解除引用',
    publishBlocked: '有阻断问题没解决，不许发布',
  },
  empty: {
    noIssues: '都过了 —— 没有阻断、没有待办、没有提示',
    trashEmpty: '回收站是空的',
    noDisabledFallback: '当前没有关掉的回退规则',
    matrixNoMatch: '没有匹配的参数',
    matrixNoCols: '先勾选要对照的列',
    matrixSearchSpansAllTabs: '搜索跨全部分类',
    selectBundle: '左边选一个套餐',
    selectAsset: '左边选一条文件，这里显示它被谁引用',
  },
  relate: { goFixIt: '去改那一项', showAnyway: '仍然展开看', foldBack: '收起来' },
  snapshot: {
    current: { label: '快照已跟上', explain: '停手之后已经写过一次崩溃快照，现在崩了也不丢' },
    pending: { label: '待落盘', explain: '刚改的还在内存里，停手 2 秒后会写一次快照' },
    failed: { label: '快照写不进去', explain: '快照写不进去，这会儿崩了会丢掉未保存的改动。改动本身没受影响' },
  },
  paramStatus: {
    factory: { label: '出厂默认', explain: '注册表里的默认值，这台机器没改过、这个版本也没改过' },
    machine: { label: '机型默认', explain: '来自这台机器的基底，这个版本自己没钉 —— 改基底它会跟着变' },
    version: { label: '本版修改', explain: '这个版本自己钉着的值，不跟随机型基底' },
    dirty: { label: '已修改', explain: '改了还没保存 —— 保存之后才会写进配方' },
  },
  paramDeprecated: {
    label: '已弃用',
    explain: '上游已标记这一项不再使用 —— 值照旧读得到（老配方里可能还写着它），但不要再改它',
  },
  paramDeprecatedChoice: {
    label: '已弃用',
    explain: '这一档放开的参数已经全部弃用 —— 选它不会带来任何还改得动的东西',
  },
  matrixRow: {
    notOwn: { label: '本机无此项', explain: '这一行是别的机型的参数，这台基准机型没有 —— 没有基准可比' },
    diff: { label: '差异', explain: '有勾选列的值与机型基底不同 —— 绿底的那几格就是' },
    same: { label: '一致', explain: '勾选列的值都与机型基底一致' },
  },
}

/* ---------- 参数夹具（照 ParamView / ChoiceView 的形状手写） ---------- */

interface FixtureParam {
  key: string
  label: string
  desc: string
  sectionId: string
  tabId: string
  order: number
  valueType: 'float' | 'int' | 'bool' | 'string'
  uiComponent: 'number' | 'switch' | 'segmented' | 'select' | 'gcode'
  defaultValue: unknown
  min: number | null
  max: number | null
  step: number | null
  unit: string | null
  choices: { label: string; value: string; deprecated: boolean }[]
  showWhen: { key: string; op: 'eq' | 'neq' | 'gt'; value: unknown } | null
  parentKey: string | null
  /** 空 = 不限机型（真源里是逗号串，这里按拆好的数组存） */
  machineFilter: string[]
  depth: number
  deprecated: boolean
}

const p = (over: Partial<FixtureParam> & { key: string; label: string }): FixtureParam => ({
  desc: '',
  sectionId: 'wipe',
  tabId: 'wiping',
  order: 1,
  valueType: 'float',
  uiComponent: 'number',
  defaultValue: 0,
  min: null,
  max: null,
  step: null,
  unit: null,
  choices: [],
  showWhen: null,
  parentKey: null,
  machineFilter: [],
  depth: 0,
  deprecated: false,
  ...over,
})

const PARAMS: FixtureParam[] = [
  p({ key: 'wiping.mode', label: '擦料方式', order: 1, valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'tower',
      choices: [{ label: '擦料塔', value: 'tower', deprecated: false }, { label: '圆盘擦拭', value: 'disk', deprecated: false }] }),
  p({ key: 'wiping.speed', label: '擦料塔速度与质量', order: 2, defaultValue: 45, min: 0, max: 100, step: 1, unit: 'mm/s',
      parentKey: 'wiping.mode', depth: 1,
      showWhen: { key: 'wiping.mode', op: 'eq', value: 'tower' } }),
  p({ key: 'wiping.legacy', label: '旧版擦料计数', order: 3, defaultValue: 7, deprecated: true }),
  p({ key: 'wiping.have_components', label: '启用擦料组', order: 4, valueType: 'bool', uiComponent: 'switch', defaultValue: false }),
  p({ key: 'frame.type', label: '外围结构', order: 5, sectionId: 'shell', valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'brim',
      choices: [
        { label: '斜肋外墙', value: 'brim', deprecated: false },
        /* 「护套」放开的 2 条（frame.shell.*）全被上游标了弃用 —— 推出来的选项级弃用 */
        { label: '护套', value: 'sheath', deprecated: true },
      ] }),
  p({ key: 'frame.shell.speed', label: '护套打印速度', order: 6, sectionId: 'shell', defaultValue: 40, min: 1, max: 200, step: 1,
      parentKey: 'frame.type', depth: 1, deprecated: true,
      showWhen: { key: 'frame.type', op: 'eq', value: 'sheath' } }),
  p({ key: 'frame.shell.wall', label: '护套壁宽', order: 7, sectionId: 'shell', defaultValue: 0.8, min: 0.1, max: 3, step: 0.1, unit: 'mm',
      parentKey: 'frame.type', depth: 1, deprecated: true,
      showWhen: { key: 'frame.type', op: 'eq', value: 'sheath' } }),
  /*
   * 照真数据摆：`wiping.ironing_coverage_threshold` 是 **float + 百分比输入框**
   * （unit % · 0~100 · 步进 1），身上**不带** choices —— 真源里那三条 0/50/90
   * 没有任何消费方，却被一路误读成「只能三选一」，已按「`choices` 是取值域、
   * 只对 string 开」这道判据从 `presets/registry/param_registry.toml` 删掉。
   */
  p({ key: 'ironing.threshold', label: '熨烫覆盖阈值', order: 8, sectionId: 'ironing', tabId: 'ironing',
      defaultValue: 10, min: 0, max: 100, step: 1, unit: '%' }),
  p({ key: 'toolhead.script', label: '装载胶箱 G-code', order: 9, sectionId: 'space', tabId: 'space',
      valueType: 'string', uiComponent: 'gcode', defaultValue: GCODE_SAMPLE() }),
  /*
   * —— C15 A2「整卡收起」的两张演示卡 ——
   *
   * 真数据里两种关法都存在，各摆一张（照 `presets/layout_schema.toml` 与
   * `presets/registry/param_registry.toml` 的实况）：
   *
   *   tower_position   门槛写在**布局表**上（section 级 showWhen，真数据里就是那一段
   *                    `tower_position` 指 `wiping.have_wiping_components = tower`）
   *                    → 整组被 section 级条件关掉，后端给整句话（`offNote`）
   *   tower_structure  门槛写在**参数**上（字段级 showWhen，真数据里那 9 条塔参数就是）
   *                    → 后端不给整组的话，收起时摆第一行那句 `blockedHint`
   *
   * 切到「圆盘擦拭」（`wiping.mode = disk`）这两张卡就整张收起来 —— 正是作者
   * 「切换到圆盘擦拭的时候那些擦料塔的都隐藏」那句话要的形状。
   */
  p({ key: 'wiping.wiper_x', label: '擦料塔位置 X', sectionId: 'tower_position', order: 10, defaultValue: -1, unit: 'mm' }),
  p({ key: 'wiping.wiper_y', label: '擦料塔位置 Y', sectionId: 'tower_position', order: 11, defaultValue: 18.6, unit: 'mm' }),
  p({ key: 'wiping.tower_first_layer_flow', label: '擦料塔首层流量', sectionId: 'tower_structure', order: 12, defaultValue: 1, unit: 'x',
      showWhen: { key: 'wiping.mode', op: 'eq', value: 'tower' } }),
  p({ key: 'wiping.outer_structure', label: '外围结构', sectionId: 'tower_structure', order: 13, valueType: 'string', uiComponent: 'segmented',
      defaultValue: 'brim',
      choices: [
        { label: 'brim', value: 'brim', deprecated: false },
        { label: '斜肋外墙', value: 'rib', deprecated: false },
      ],
      showWhen: { key: 'wiping.mode', op: 'eq', value: 'tower' } }),
]

/**
 * **section 级条件**（照 `presets/layout_schema.toml` 的 `[tabs.sections.showWhen]` 手抄一条）。
 *
 * 真后端把这一档读进 `Registry::section_show_when`，命中时给 `Cell.blocked` 添一条
 * `scope: 'section'` —— `derive.rs` 的 `group_off_note` 就靠它认出「整组被 section 级
 * 条件关掉」，把那句话写进 `DeskGroup.offNote`（界面据此把整组收起来）。
 * 这里只抄形状，不抄数据：条件指向的还是夹具那个模式开关。
 */
const SECTION_SHOW_WHEN: Record<string, { key: string; op: 'eq' | 'neq' | 'gt'; value: unknown }> = {
  tower_position: { key: 'wiping.mode', op: 'eq', value: 'tower' },
}


function GCODE_SAMPLE(): string {
  return [
    'G92 E0',
    'M204 S800',
    'G1 X261 F10000',
    'L801 ;wipe tower start',
    'T0',
    'G1 E-5 F2400',
    'L803;此处使喷嘴降温',
    'S[AUTO] F42000',
    'M104 S0',
  ].join('\n')
}

const TABS = [
  { id: 'wiping', label: '擦料', order: 1, sections: [
    { id: 'wipe', label: '擦料方式', order: 1 },
    { id: 'shell', label: '外围结构', order: 2 },
    /* C15 A2 的两张演示卡（见 PARAMS 里那一段注释） */
    { id: 'tower_position', label: '擦料塔位置与打印', order: 3 },
    { id: 'tower_structure', label: '塔结构加强', order: 4 },
  ] },
  { id: 'ironing', label: '熨烫', order: 2, sections: [{ id: 'ironing', label: '熨烫覆盖', order: 1 }] },
  { id: 'space', label: '偏移', order: 3, sections: [{ id: 'space', label: '空间偏移', order: 1 }] },
]

const SECTION_LABEL = new Map(TABS.flatMap((t) => t.sections.map((s) => [s.id, `${t.label} · ${s.label}`])))

/* ---------- 三层取值（夹具自己的值域；判定规则照后端文档，不算前端业务） ---------- */

const MACHINES = [
  { id: 'A1', display: 'A1', versions: [
    { uid: 'A1/STANDARD', versionId: 'STANDARD', name: '标准版', tag: '推荐' },
    { uid: 'A1/FAST', versionId: 'FAST', name: '高速版', tag: null },
  ] },
]

/** 盘上那一份：机型基底与各版本钉着的键（save 之后这里跟着变） */
const atRest: { base: Record<string, Json>; over: Record<string, Json> } = {
  base: { A1: { 'wiping.speed': 45 } },
  over: {
    'A1/STANDARD': { 'wiping.speed': 70 },
    'A1/FAST': {},
  },
}

/** 草稿里还没保存的改动（level|owner|key → value；null = 删键） */
const pending = new Map<string, unknown>()

/* ---------- 套餐 / 资产 / 回退夹具（P4 三页的形状镜像；判定照后端文档现算） ---------- */

/**
 * 品牌夹具（2026-10-03：品牌升成一等条目 —— 机型与版本页可编辑显示名与品牌图）。
 * 与真数据同形：机型的 `brand` 字段写着品牌的 **id**（`Bambu Lab` 这个字符串既是 id
 * 也是真数据里那份 id），`logo` 是**资产 id**（不是文件名）。
 */
const BRANDS: { id: string; name: string; logo: string | null }[] = [
  { id: 'Bambu Lab', name: '拓竹 (Bambu Lab)', logo: 'bambu-lab-logo' },
]

/** 机型清单（含品牌反查 —— 与真机同一条口径，前端不复算关系） */
const machineListOf = () => ({
  brands: BRANDS.map((b) => ({
    ...b,
    machines: MACHINE_VIEWS.filter((m) => m.brand.toLowerCase() === b.id.toLowerCase()).map((m) => m.id),
  })),
  machines: MACHINE_VIEWS,
  root: 'C:\\dev\\MKPSupportEase\\presets',
})

/**
 * 尺寸六组夹具（与真数据 A1 逐格同形 —— 探针要拿它跟界面上的读数对）。
 *
 * 六组**一个不少**：文件里 `[dimensions]` 是全有或全无，这里照同一口径造。
 */
const A1_DIMENSIONS = {
  bedSize: { width: 260, depth: 255 },
  movementRange: { minX: -40, maxX: 260, minY: 0, maxY: 255, maxZ: 999 },
  edgeZone: 10,
  glueArea: { glueMinX: -40, glueMaxX: 260, glueMinY: 0, glueMaxY: 255, wipeX: 252 },
  calibration: {
    lShapeBaseX: 68.21, lShapeBaseY: 126.373,
    xLineX: 114.523, xLineY: 104.83, xLineYEnd: 114.83,
    yLineX: 104.53, yLineXEnd: 114.53, yLineY: 114.83,
    zStartX: 68.21, zStartY: 126.373,
  },
  flags: { gcodeMarker: ';===== machine: A1', hasSecondFan: false },
}

/** P1S 的尺寸（床身 256×256 —— 禁区画布的 viewBox 就是它） */
const P1S_DIMENSIONS = {
  bedSize: { width: 256, depth: 256 },
  movementRange: { minX: 0, maxX: 255, minY: 0, maxY: 265, maxZ: 999 },
  edgeZone: 10,
  glueArea: { glueMinX: 0, glueMaxX: 255, glueMinY: 0, glueMaxY: 265, wipeX: 20 },
  calibration: {
    lShapeBaseX: 68.21, lShapeBaseY: 126.373,
    xLineX: 114.523, xLineY: 104.83, xLineYEnd: 114.83,
    yLineX: 104.53, yLineXEnd: 114.53, yLineY: 112.83,
    zStartX: 68.21, zStartY: 126.373,
  },
  flags: { gcodeMarker: ';===== machine: P1', hasSecondFan: true },
}

/** P1S 的两块禁区（点序与真数据 `forbidden_zones/P1S.toml` 同形：6 点 + 4 点） */
const P1S_ZONES: { points: [number, number][] }[] = [
  {
    points: [
      [0, 0], [240, 0], [240, 14], [238, 14], [238, 5], [0, 5],
    ],
  },
  {
    points: [
      [0, 0], [18, 0], [18, 28], [0, 28],
    ],
  },
]

/** 机型夹具带上一版一套的指向（MachinesPage 也要用） */
const MACHINE_VIEWS: {
  id: string; display: string; name: string; brand: string;
  defaultBundle: string | null; externalAliases: string[]; image: string | null;
  icon: string | null; hasDimensions: boolean;
  dimensions: typeof A1_DIMENSIONS | null;
  zoneCount: number; zones: { points: [number, number][] }[]; file: string;
  versions: { id: string; name: string; recommendedBundle: string | null; tag: string | null; description: string | null; image: string | null; hasRecipe: boolean }[];
}[] = [
  {
    id: 'A1', display: 'A1', name: 'A1', brand: 'Bambu Lab',
    defaultBundle: 'A1_default', externalAliases: ['A1C'], image: null, icon: 'a1-icon',
    hasDimensions: true, dimensions: A1_DIMENSIONS, zoneCount: 0, zones: [], file: 'A1.toml',
    versions: [
      // `image` = 这一版专属的外观图（资产 id）。`null` = 回落机型图（第三刀的默认）
      { id: 'STANDARD', name: '标准版', recommendedBundle: 'A1_default', tag: '推荐', description: null, image: null, hasRecipe: true },
      { id: 'FAST', name: '高速版', recommendedBundle: 'A1_FAST', tag: null, description: null, image: 'a1_mini-variant-image', hasRecipe: true },
    ],
  },
  {
    // 第二台（有禁区）—— 禁区编辑器与「移到品牌…」都要有第二条才量得出来
    id: 'P1S', display: 'P1S', name: 'P1S', brand: 'Bambu Lab',
    defaultBundle: 'P1S_default', externalAliases: ['P1'], image: 'p1s-image', icon: null,
    hasDimensions: true, dimensions: P1S_DIMENSIONS, zoneCount: 2, zones: P1S_ZONES, file: 'P1S.toml',
    versions: [
      { id: 'STANDARD', name: '标准版', recommendedBundle: 'P1S_default', tag: '推荐', description: null, image: null, hasRecipe: true },
    ],
  },
  {
    // 占位机型（没尺寸）—— 「尺寸卡显示说明而不是报错」那一档
    id: 'A2L', display: 'A2L', name: 'A2L', brand: 'Bambu Lab',
    defaultBundle: null, externalAliases: [], image: null, icon: null,
    hasDimensions: false, dimensions: null, zoneCount: 0, zones: [], file: 'A2L.toml',
    versions: [
      { id: 'STANDARD', name: '标准版', recommendedBundle: null, tag: null, description: null, image: null, hasRecipe: false },
    ],
  },
]

type FixtureRef = { id: string; kind: string; slicer: string | null; profile: string | null; name: string; path: string; delivery?: 'download' | 'bundled' }

/** 资产域夹具。喷嘴 / 层高不存（doc §12.5 同一条），从路径与文件名现算。
 *  **image 类回来了**（2026-10-03，撤销第三刀的剥离）：整机图在台账里，用
 *  `delivery: 'bundled'` 表达「随包不下载」—— 台账可管可换图，客户端不下载。 */
const ASSETS: FixtureRef[] = [
  /* 品牌字标（2026-10-03 品牌图正式进台账）：**公共素材**，不写 machineId */
  { id: 'bambu-lab-logo', kind: 'image', slicer: null, profile: null, name: 'Bambu Lab 字标', path: 'brands/bambu-lab-logo.svg', delivery: 'bundled' },
  { id: 'a1-image', kind: 'image', slicer: null, profile: null, name: 'A1 外观图', path: 'printers/a1.webp', delivery: 'bundled' },
  { id: 'a1_mini-image', kind: 'image', slicer: null, profile: null, name: 'A1 mini 外观图', path: 'printers/a1mini.webp', delivery: 'bundled' },
  { id: 'p1s-image', kind: 'image', slicer: null, profile: null, name: 'P1S 外观图', path: 'printers/p1s.webp', delivery: 'bundled' },
  { id: 'a1-icon', kind: 'icon', slicer: null, profile: null, name: 'A1 图标', path: 'icons/a1.svg' },
  { id: 'a1-standard', kind: 'mkPreset', slicer: null, profile: null, name: 'A1 标准版预设', path: '' },
  { id: 'a1-fast', kind: 'mkPreset', slicer: null, profile: null, name: 'A1 高速版预设', path: '' },
  { id: 'p1s-icon', kind: 'icon', slicer: null, profile: null, name: 'P1S 图标', path: 'icons/p1s.svg' },
  { id: 'mkp-support-models', kind: 'model', slicer: null, profile: null, name: '支撑测试模型', path: 'models/support-test.3mf' },
  { id: 'a1-bbs-02-010', kind: 'slicerProfile', slicer: 'bbs', profile: 'process', name: 'A1：0.2 喷头 0.10 层高', path: 'bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json' },
  { id: 'a1-bbs-04-020', kind: 'slicerProfile', slicer: 'bbs', profile: 'process', name: 'A1：0.4 喷头 0.20 层高', path: 'bbs/Process/0.4mm/MKPProcess A1 0.4 0.20.json' },
  { id: 'a1-mini-bbs-02-010', kind: 'slicerProfile', slicer: 'bbs', profile: 'process', name: 'A1 mini：0.2 喷头 0.10 层高', path: 'bbs/Process/0.2mm/MKPProcess A1 mini 0.2 0.10.json' },
  { id: 'p1s-bbs-04-024', kind: 'slicerProfile', slicer: 'bbs', profile: 'process', name: 'P1S：0.4 喷头 0.24 层高', path: 'bbs/Process/0.4mm/MKPProcess P1S 0.4 0.24.json' },
  { id: 'a1-orca-02-010', kind: 'slicerProfile', slicer: 'orca', profile: 'process', name: 'A1：Orca 0.2 喷头 0.10 层高（可选）', path: 'orca/Process/0.2mm/OrcaProcess A1 0.2 0.10.json' },
]

/** 套餐域夹具。真源关系是**一版一套**：A1 两版各指一份（A1_default / A1_FAST）。
 *  `presets` = 套餐挂的 MKP 预设（**版本 uid 直引，文件可不存在** —— 作者 2026-10-03） */
const BUNDLES: { id: string; display: string; machineId: string; assetRefs: string[]; updatedAt: string | null }[] = [
  { id: 'A1_default', display: '官方推荐', machineId: 'A1', assetRefs: ['a1-standard', 'a1-bbs-04-020'], updatedAt: '2026-07-12' },
  { id: 'A1_FAST', display: '高速版工艺', machineId: 'A1', assetRefs: ['a1-fast', 'a1-bbs-02-010', 'a1-orca-02-010'], updatedAt: '2026-07-12' },
  { id: 'P1S_default', display: '官方推荐', machineId: 'P1S', assetRefs: ['p1s-bbs-04-024'], updatedAt: '2026-07-12' },
]

/** 可见性（含草稿态）：fileId → 'archiveOnly'。写路径与 save 都落这里 */
const pendingVis = new Map<string, string>()

/** 切片器三根轴之二三 —— 判据与后端 slicer_axes 同一条（路径 mm 段 + 文件名尾数） */
function slicerAxes(path: string): { nozzle: string | null; layer: string | null } {
  const numeric = (s: string) => s !== '' && /\d/.test(s) && /^[0-9.]+$/.test(s)
  const nozzle = path.split('/').find((seg) => seg.endsWith('mm'))?.slice(0, -2) ?? null
  const file = path.split('/').pop() ?? path
  const stem = /\.json$/i.test(file) ? file.replace(/\.json$/i, '') : file
  const tail = stem.split(' ').pop() ?? null
  return { nozzle: nozzle && numeric(nozzle) ? nozzle : null, layer: tail && numeric(tail) ? tail : null }
}

const inBundleSet = () =>
  new Set(BUNDLES.flatMap((b) => b.assetRefs.map((r) => r.toLowerCase())))

function assetListOf(kind: string | null, slicer: string | null, nozzle: string | null, layer: string | null, identity: string | null, query: string | null) {
  const bundled = inBundleSet()
  const rows = ASSETS.map((a) => {
    const axes = slicerAxes(a.path)
    const vis = pendingVis.get(a.id) ?? 'menu'
    const delivery = a.delivery ?? 'download'
    // 四态身份，判定与真机同一优先级：归档 > 随包 > 进套餐 > 可选
    const idt =
      vis === 'archiveOnly' ? 'archiveOnly'
      : delivery === 'bundled' ? 'bundled'
      : bundled.has(a.id.toLowerCase()) ? 'inBundle'
      : 'optional'
    return {
      id: a.id, kind: a.kind, machineId: a.id.startsWith('a1-') && a.kind !== 'model' ? 'A1' : a.id.startsWith('p1s-') ? 'P1S' : null,
      versionId: a.kind === 'mkPreset' ? 'STANDARD' : null,
      // 显示名一律真名：MKP 预设 = 版本名、切片器 = 文件名（与后端同一口径）
      display:
        a.kind === 'mkPreset'
          ? (MACHINE_VIEWS.find((m) => m.id === 'A1')?.versions.find((v) => v.id === 'STANDARD')?.name ?? a.name)
          : a.kind === 'slicerProfile'
            ? (a.path.split('/').pop() ?? a.path).replace(/\.json$/i, '')
            : a.name,
      name: a.name, path: a.path, url: `/assets/${a.path}`, slicer: a.slicer, profile: a.profile,
      // MKP 预存在资产库里恒有登记；「有没有生成」由生成页那套判据说（演示：一份已生成、
      // 一份待生成）—— 作者 2026-10-03：「文件在不在都能选，徽章说生成到哪一步了」
      present: a.kind === 'mkPreset' ? a.id === 'a1-standard' : true,
      buildState: a.kind === 'mkPreset' ? (a.id === 'a1-standard' ? 'built' : 'neverBuilt') : null,
      nozzle: axes.nozzle, layer: axes.layer, identity: idt,
      delivery,
    }
  })
  const q = (query ?? '').trim().toLowerCase()
  const all = rows
  const filtered = rows.filter((a) =>
    (kind === null || a.kind === kind) &&
    (slicer === null || a.slicer === slicer) &&
    (nozzle === null || a.nozzle === nozzle) &&
    (layer === null || a.layer === layer) &&
    (identity === null || a.identity === identity) &&
    (q === '' || a.name.toLowerCase().includes(q) || a.id.toLowerCase().includes(q)),
  )
  const numeric = (xs: string[]) => [...new Set(xs)].sort((x, y) => parseFloat(x) - parseFloat(y))
  return {
    assets: filtered,
    root: 'C:\\dev\\presets\\assets',
    nozzles: numeric(all.filter((a) => a.kind === 'slicerProfile' && a.nozzle).map((a) => a.nozzle as string)),
    layers: numeric(all.filter((a) => a.kind === 'slicerProfile' && a.layer).map((a) => a.layer as string)),
    total: all.length,
    optionalCount: all.filter((a) => a.identity === 'optional').length,
    archiveCount: all.filter((a) => a.identity === 'archiveOnly').length,
  }
}

/** 演示用的"源文件字节的 SHA-256"：真机上由后端读字节算 —— 这里只要形如 64 位 hex */
function mockSha(id: string): string {
  const hex = '0123456789abcdef'
  let out = ''
  for (let i = 0; i < 64; i += 1) out += hex[(id.charCodeAt(i % id.length) + i) % 16]
  return out
}

/**
 * `wb_asset_inspect` 的演示读数（第四刀）—— 与真机同形：
 * 普通资产指向**源文件**（`presets/assets/<path>`）、mkPreset 指向**产物**
 * （`presets/dist/mkp/presets/A1-standard.toml`，一份已生成、一份还没）。
 * 文件不在的那一条给期望路径 + 空读数（照真机的口径）。
 */
function assetInspectOf(a: FixtureRef) {
  const toWin = (p: string) => p.replace(/\//g, '\\')
  if (a.kind === 'mkPreset') {
    const versionId = a.id.endsWith('-fast') ? 'FAST' : 'STANDARD'
    const fileName = `A1-${versionId.toLowerCase()}.toml`
    // 演示：一份产物已生成、一份还没 —— 与列表里的 buildState 同一套演示事实
    const exists = a.id === 'a1-standard'
    const productPath = `presets/dist/mkp/presets/${fileName}`
    return {
      id: a.id, fileName, absPath: `C:\\dev\\${toWin(productPath)}`, exists,
      bytes: exists ? 4312 : null, sha256: exists ? mockSha(a.id) : null,
      width: null, height: null, format: 'toml', productPath,
    }
  }
  const fileName = a.path.split('/').pop() ?? a.path
  const format = fileName.includes('.') ? (fileName.split('.').pop() ?? '').toLowerCase() : null
  const image = a.kind === 'image'
  const icon = a.kind === 'icon'
  return {
    id: a.id, fileName, absPath: `C:\\dev\\presets\\assets\\${toWin(a.path)}`, exists: true,
    bytes: image ? 95232 : icon ? 1824 : a.kind === 'model' ? 3355443 : 12048,
    sha256: mockSha(a.id), format,
    width: image ? 1024 : icon ? 24 : null,
    height: image ? 768 : icon ? 24 : null,
    productPath: null,
  }
}

function bundleListOf(query: string | null) {
  const q = (query ?? '').trim().toLowerCase()
  const versions = MACHINE_VIEWS.flatMap((m) => m.versions.map((v) => ({ machineId: m.id, defaultBundle: m.defaultBundle, ...v })))
  const listed = BUNDLES.filter((b) => q === '' || b.id.toLowerCase().includes(q) || b.display.toLowerCase().includes(q))
  return {
    bundles: listed.map((b) => ({
      id: b.id, display: b.display, machineId: b.machineId, updatedAt: b.updatedAt,
      assetRefs: b.assetRefs.map((r) => {
        const a = ASSETS.find((x) => x.id === r)
        return {
          id: r, kind: a?.kind ?? 'slicerProfile', resolvable: a !== undefined,
          isBbs: a?.kind === 'slicerProfile' && a.slicer === 'bbs',
          name: a?.name ?? '',
          // MKP 预设的「在不在」按生成状态判（与真机同口径）；演示：一份已生成、一份待生成
          present: a !== undefined && (a.kind !== 'mkPreset' || a.id === 'a1-standard'),
          buildState: a?.kind === 'mkPreset' ? (a.id === 'a1-standard' ? 'built' : 'neverBuilt') : null,
          visibility: pendingVis.get(r) ?? 'menu',
        }
      }),
      users: versions.filter((v) => (v.recommendedBundle ?? '').toLowerCase() === b.id.toLowerCase())
        .map((v) => ({ machineId: v.machineId, versionId: v.id })),
      defaultFor: MACHINE_VIEWS.filter((m) => (m.defaultBundle ?? '').toLowerCase() === b.id.toLowerCase()).map((m) => m.id),
    })),
    total: BUNDLES.length,
  }
}

/** 官方源（Bootstrap）：`wb_set_bootstrap` 写它（浏览器里存内存）。真机写 workbench/bootstrap.json */
let mockBootstrap: string | null = null

/** `app::Boot` 的桩。**唯一的数据根是 presets/**（没有第二候选、不 fallback） */
function mockBoot(): Json {
  return {
    // 与真机同一个形状：`<仓库>/presets`（状态条上只摆最后两段，见 App.shortRoot）
    roots: {
      workbench: 'C:\\dev\\MKPSupportEase\\workbench',
      presets: 'C:\\dev\\MKPSupportEase\\presets',
      dist: 'C:\\dev\\MKPSupportEase\\presets\\dist',
    },
    problem: null,
    detail: null,
    storeDirs: STORE_DIRS,
    bootstrapUrl: mockBootstrap,
  }
}

const STORE_DIRS = [
  { name: 'machines', role: '机型配方的真源（machineVariants 按机型一份）。机型页读写它' },
  { name: 'bbs', role: '零读写 —— BBS 预设今天不经工作台，占位' },
  { name: '.draft', role: '草稿与界面状态（book.json / ui.json）。只存「改了什么」，懒写' },
  { name: '.trash', role: '回收站：删除版本的生成快照、交付残留回收，都在这里' },
  { name: '.snapshots', role: '生成快照：生成时写入，恢复配方时只读 —— 不当编辑对象' },
]

/** 生成记录（MarkBuilt 的内存账）；wb_generate 往里记 */
const builtRecords = new Set<string>(['A1/STANDARD'])

const BASELINE = [
  { fileName: 'A1_standard_v3.3.toml', status: 'same', productSha: 'f444aeaf1a2b3c4d', baselineSha: 'f444aeaf1a2b3c4d' },
  { fileName: 'A1_fast_v3.3.toml', status: 'same', productSha: '9c8b7a6f5e4d3c2b', baselineSha: '9c8b7a6f5e4d3c2b' },
  { fileName: 'P1S_standard_v3.3.toml', status: 'changed', productSha: '0112233445566778', baselineSha: '9988776655443322' },
]

const TRASH = [
  { file: '20260928-T-10-12-45__A1__OLDVER', deletedStamp: '20260928-T-10-12-45', machineId: 'A1', versionId: 'OLDVER' },
]

/** dist 面的残留（`wb_dist_strays` 读它、「清理残留」清它）—— 桩里可变，好让发布闸跟着动 */
let mockStrays: string[] = ['mkp/presets/old_file.toml']

/** 发布闸一行（`audit::AuditItem` 的桩形状） */
interface MockAuditItem {
  id: string
  name: string
  status: string
  severity: string
  details: string
  affectedFiles: string[]
  fixHint: string
}

/** 演示用的结构代次与签名（真值由 Rust 的 `runtime::structure` 从类型算出来） */
const MOCK_EPOCH = 1
const MOCK_SIGNATURE = 'cb1080919d39b2bd'

/** 十五项的编号与名字照 `docs/PUBLISH-ARCHITECTURE.md` §5.2（id 是契约） */
const AUDIT_ROWS: [string, string][] = [
  ['sources/complete', '源数据完整'],
  ['refs/resolve', '引用都能落地'],
  ['assets/exist', 'A 类资产的文件真在'],
  ['assets/path-unique', 'catalog.path 唯一且不越界'],
  ['presets/renderable', 'B 类预设能成功渲染'],
  ['presets/rendered', '渲染产物真在交付目录里'],
  ['catalog/matches-files', '登记的每一条都取得到'],
  ['catalog/sha-size', 'SHA / 大小对真字节算且一致'],
  ['catalog/no-phantoms', '没有幽灵条目'],
  ['dist/no-strays', '交付目录里没有残留'],
  ['bundles/closure', '套餐引用闭包完整'],
  ['version/structure', '结构代次与最低客户端版本'],
  ['source/correct', 'Bootstrap（source.json）正确'],
  ['manifest/correct', 'manifest 与交付集合一致'],
  ['git/clean', 'Git 工作区干净'],
]

/**
 * 发布闸（第二刀）的桩。
 *
 * ★ **判定不在这一层**：真判据是 Rust 的 `audit::publish_audit`（界面与 `cargo test`
 * 调的是同一个函数）。这一份只是让浏览器里那个框能验收 —— 所以结论按桩自己那份数据给，
 * 至少 `dist/no-strays` 要跟着 [`mockStrays`] 走，别让两项桩互相打脸。
 */
function mockAudit(): Json {
  const items: MockAuditItem[] = AUDIT_ROWS.map(([id, name]) => {
    if (id === 'version/structure') {
      // 第三刀之后这一项**真跑了**（不再是 Skipped）—— 桩照它的结论形态给一份
      return {
        id,
        name,
        status: 'pass',
        severity: 'blocker',
        details: `结构代次 ${MOCK_EPOCH} · 签名 ${MOCK_SIGNATURE} · 最低正式客户端版本 0.0.1（演示桩）`,
        affectedFiles: [],
        fixHint: '演示桩：这一项的真判据在 Rust（`runtime::structure` + 规则表）',
      }
    }
    if (id === 'dist/no-strays' && mockStrays.length > 0) {
      return {
        id,
        name,
        status: 'fail',
        severity: 'blocker',
        details: `${mockStrays.length} 个残留文件`,
        affectedFiles: [...mockStrays],
        fixHint: '点「清理残留」（进 workbench/.trash/dist/<时间戳>/，可还原）',
      }
    }
    return {
      id,
      name,
      status: 'pass',
      severity: 'blocker',
      details: '演示桩：这一项按「全过」给',
      affectedFiles: [],
      fixHint: '演示桩没有真判据 —— 真机由 Rust 的 audit::publish_audit 说了算',
    }
  })
  return {
    items,
    filesAdded: 12,
    filesChanged: 3,
    filesRemoved: 0,
    minVersion: '0.0.1',
    canPublish: !items.some((i) => i.severity === 'blocker' && i.status === 'fail'),
  }
}

const splitKey = (raw: string) => {
  const [level, owner, ...rest] = raw.split('|')
  return { level: level as 'machine' | 'version', owner, key: rest.join('|') }
}

function pinned(level: 'machine' | 'version', owner: string, key: string): { has: boolean; value: unknown } {
  if (pending.has(`${level}|${owner}|${key}`)) {
    const v = pending.get(`${level}|${owner}|${key}`)
    return { has: v !== null, value: v }
  }
  const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
  if (table && key in table) return { has: true, value: table[key] }
  return { has: false, value: undefined }
}

function effective(machineId: string, uid: string | null, key: string) {
  if (uid !== null) {
    const hit = pinned('version', uid, key)
    if (hit.has) return { value: hit.value, origin: 'version', own: true }
  }
  const hit = pinned('machine', machineId, key)
  if (hit.has) return { value: hit.value, origin: 'machine', own: true }
  const pdef = PARAMS.find((x) => x.key === key)
  return pdef ? { value: pdef.defaultValue, origin: 'factory', own: false } : null
}

function valueText(pdef: FixtureParam, v: unknown): string {
  if (pdef.uiComponent === 'switch') return v === true || v === 'true' ? '开启' : '关闭'
  if (pdef.uiComponent === 'gcode') {
    const s = String(v ?? '')
    return s === '' ? '空' : `${s.split('\n').length} 行 · 点开`
  }
  const hit = pdef.choices.find((c) => String(c.value) === String(v))
  if (hit) return hit.label
  const text = v === '' || v === null || v === undefined ? '空' : String(v)
  return pdef.unit ? `${text} ${pdef.unit}` : text
}

/* ---------- Desk / Book / Registry 的现算（照 DTO 形状） ---------- */

/**
 * 这一项被哪些条件关着。**根在前** —— 与后端的 `Gate::blocked` 同一个口径：
 * 段（section）级的条件排在字段自己的条件前面（那是更靠上的根）。
 *
 * 真后端 `visibility.rs` 的 `walk` 一段不落：字段自己的 `showWhen` +
 * 它所在 section 的 `showWhen`（`Registry::section_show_when`），
 * 命中时给 `scope` 标 `field` / `section` —— 后者是「整组收起」那句
 * （`derive.rs` 的 `group_off_note`）唯一的判据。
 */
function blockedOf(machineId: string, uid: string | null, pdef: FixtureParam): Json[] {
  const blocked: Json[] = []
  const describe = (sw: { key: string; op: string; value: unknown }, scope: string): Json => {
    const depP = PARAMS.find((x) => x.key === sw.key)
    return {
      key: sw.key,
      label: depP?.label ?? sw.key,
      need: `${sw.op === 'eq' ? '等于' : sw.op === 'neq' ? '不等于' : '大于'} ${depP ? valueText(depP, sw.value) : String(sw.value)}`,
      scope,
    }
  }
  const holds = (sw: { key: string; value: unknown }): boolean => {
    const dep = effective(machineId, uid, sw.key)
    return String(dep?.value) === String(sw.value)
  }
  const section = SECTION_SHOW_WHEN[pdef.sectionId]
  if (section && !holds(section)) blocked.push(describe(section, 'section'))
  if (pdef.showWhen && !holds(pdef.showWhen)) blocked.push(describe(pdef.showWhen, 'field'))
  return blocked
}

function cellOf(machineId: string, uid: string | null, pdef: FixtureParam): Json {
  const hit = effective(machineId, uid, pdef.key)
  const raw = hit?.value
  const blocked = blockedOf(machineId, uid, pdef)
  const deprecated = pdef.deprecated
  const editable = blocked.length === 0 && !deprecated
  const originExplain =
    hit ? ((WORDS.origin as Json)[hit.origin] as Json | undefined)?.explain ?? null : null
  return {
    kind: blocked.length > 0 ? 'notApplicable' : pdef.uiComponent === 'gcode' ? 'gcode' : 'value',
    text: valueText(pdef, raw),
    lines: pdef.uiComponent === 'gcode' ? String(raw ?? '').split('\n').length : null,
    origin: hit?.origin ?? null,
    originLabel: hit ? ({ factory: '出厂', machine: '机型', version: '版本' } as Record<string, string>)[hit.origin] : null,
    originExplain,
    own: hit?.own ?? false,
    dirty: pending.has(`${uid !== null ? 'version' : 'machine'}|${uid ?? machineId}|${pdef.key}`),
    editable,
    reason: blocked.length > 0 ? '上一项没打开，这一项现在不生效，所以不让改'
      : deprecated ? '上游已标记这一项不再使用 —— 值照旧读得到（老配方里可能还写着它），但不要再改它'
      : null,
    blocked,
    blockedNote: blocked.length > 0 ? `改不动：由「${(blocked[0] as Json).label}」控制，需${(blocked[0] as Json).need}` : null,
    blockedHint: blocked.length > 0 ? `要 ${(blocked[0] as Json).label} ${(blocked[0] as Json).need} 才可改` : null,
    jumpTo: blocked.length > 0 ? (blocked[0] as Json).key : null,
    raw: raw ?? null,
    /* 盘上钉着的那个值（草稿不算）—— 「恢复修改前的」据此写回，没钉着就写 null 删键 */
    rest: atRestOf(uid !== null ? 'version' : 'machine', uid ?? machineId, pdef.key),
  }
}

/** 这一层在 `atRest` 表里钉着的值（`pending` 一概不看） */
function atRestOf(level: 'machine' | 'version', owner: string, key: string): unknown {
  const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
  return table && key in table ? table[key] : null
}

/* origin explain 已在 cellOf 里从词表现取。 */

function impactOf(machineId: string, uid: string | null, key: string): Json {
  const label = (v: { name: string }) => `${machineId} / ${v.name}`
  const pins = (v: { uid: string }) => atRest.over[v.uid]?.[key] !== undefined
  const mine = MACHINES[0]!.versions
  if (uid !== null) {
    return {
      targets: mine.filter((v) => v.uid === uid).map((v) => label(v)),
      followers: mine.filter((v) => v.uid !== uid && !pins(v)).map((v) => label(v)),
    }
  }
  return { targets: mine.filter((v) => !pins(v)).map((v) => label(v)), followers: [] }
}

function rowOf(machineId: string, uid: string | null, pdef: FixtureParam): Json {
  const parent = pdef.parentKey ? PARAMS.find((x) => x.key === pdef.parentKey) : null
  return {
    key: pdef.key,
    label: pdef.label,
    desc: pdef.desc,
    unit: pdef.unit,
    sectionId: pdef.sectionId,
    sectionLabel: SECTION_LABEL.get(pdef.sectionId) ?? pdef.sectionId,
    tabId: pdef.tabId,
    depth: pdef.depth,
    parentKey: pdef.parentKey,
    parentLabel: parent?.label ?? null,
    parentNote: parent ? `属于：${parent.label}` : null,
    controlNote: pdef.showWhen ? `受「${PARAMS.find((x) => x.key === pdef.showWhen?.key)?.label}」控制` : null,
    gcode: pdef.uiComponent === 'gcode',
    deprecated: pdef.deprecated,
    impact: impactOf(machineId, uid, pdef.key),
    cells: [null, 'A1/STANDARD', 'A1/FAST'].map((u) => cellOf(machineId, u, pdef)),
  }
}

function buildDesk(machineId: string, uid: string | null, tab: string | null, query: string): Json {
  const cols = [
    { key: machineId, machineId, versionUid: null, level: 'machine', machine: 'A1', label: '机型基底', items: Object.keys(atRest.base[machineId] ?? {}).length },
    ...MACHINES[0]!.versions.map((v) => ({
      key: v.uid, machineId, versionUid: v.uid, level: 'version', machine: 'A1',
      label: v.name, items: Object.keys(atRest.over[v.uid] ?? {}).length,
    })),
  ]
  const cur = cols.findIndex((c) => (c.versionUid ?? null) === uid)
  const q = query.trim().toLowerCase()
  const params = PARAMS.filter((pdef) => {
    if (q) return `${pdef.label} ${pdef.key}`.toLowerCase().includes(q)
    return tab === null || pdef.tabId === tab
  })
  const groups: { sectionId: string; label: string; count: number; offNote: string | null; items: Json[] }[] = []
  for (const pdef of params) {
    const row = rowOf(machineId, uid, pdef)
    let group = groups[groups.length - 1]
    if (!group || group.sectionId !== pdef.sectionId) {
      group = { sectionId: pdef.sectionId, label: SECTION_LABEL.get(pdef.sectionId) ?? pdef.sectionId, count: 0, offNote: null, items: [] }
      groups.push(group)
    }
    group.count += 1
    if (pdef.parentKey !== null && !q) {
      const parentRow = group.items.find((it) => ((it as Json).row as Json).key === pdef.parentKey)
      if (parentRow) {
        ;((parentRow as Json).children as Json[]).push(row)
        continue
      }
    }
    group.items.push({ row, children: [], offNote: null })
  }
  const nav = TABS.map((t) => ({
    id: t.id,
    label: t.label,
    count: PARAMS.filter((pdef) => pdef.tabId === t.id).length,
    sections: t.sections.map((s) => ({
      id: s.id,
      label: s.label,
      count: PARAMS.filter((pdef) => pdef.sectionId === s.id).length,
    })),
  }))
  const curIdx = cur >= 0 ? cur : 0
  /* 整组被 section 级条件关掉的那一句（见 groupOffNoteOf） */
  for (const g of groups) g.offNote = groupOffNoteOf(g, curIdx)
  return { nav, cols, cur: curIdx, groups, total: PARAMS.length, note: q ? '搜索跨全部分类' : null, emptyReason: null }
}

/**
 * **整组被 section 级条件关掉了吗**（照 `derive.rs` 的 `group_off_note` 逐条搬）：
 * 组里每一项的当前格子都有一条 `scope: 'section'` 的 blocked，**且是同一个 key** ——
 * 是的话就回那一句（界面据此把整组收起来），不是就 null。
 *
 * 措辞照抄 `wording.rs` 的 `relate::group_off`，**连它那个空值也一起抄**：
 * 真后端传的是 `group_off(label, "", count)`（`derive.rs`），所以那句话在界面与
 * 真机上一样读作「「X」选了，这一组 N 项现在不生效」。夹具不替后端把话说圆 ——
 * 说圆了，浏览器里验收的就不是真后端会给的那一句了。
 */
function groupOffNoteOf(g: Json, cur: number): string | null {
  let who: { key: string; label: string } | null = null
  for (const it of g.items as Json[]) {
    const cell = ((it.row as Json).cells as Json[])[cur] as Json | undefined
    if (!cell) return null
    const hit = (cell.blocked as Json[]).find((b) => b.scope === 'section')
    if (!hit) return null
    const key = String(hit.key)
    const label = String(hit.label)
    if (who === null) who = { key, label }
    else if (who.key !== key) return null
  }
  if (who === null) return null
  return `「${who.label}」选了，这一组 ${String(g.count)} 项现在不生效`
}

/** 对照矩阵（C14 第四轮）：基准机型判差异、行序跟基准走 —— 照真后端的规矩 */
function buildMatrix(
  cols: { machineId: string; versionUid: string | null }[],
  tab: string | null,
  query: string,
  baseMachineId: string | null,
): Json {
  const q = query.trim().toLowerCase()
  const colDefs = cols.map((c) => {
    const m = MACHINES.find((x) => x.id === c.machineId)
    const v = m?.versions.find((x) => x.uid === c.versionUid) ?? null
    const uid = c.versionUid
    return {
      key: uid ?? c.machineId,
      machineId: c.machineId,
      versionUid: uid,
      level: uid === null ? 'machine' : 'version',
      machine: m?.display ?? c.machineId,
      label: uid === null ? '机型基底' : (v?.name ?? uid),
      items: Object.keys(uid === null ? (atRest.base[c.machineId] ?? {}) : (atRest.over[uid] ?? {})).length,
    }
  })
  const params = PARAMS.filter((pdef) => {
    if (q) return `${pdef.label} ${pdef.key}`.toLowerCase().includes(q)
    return tab === null || pdef.tabId === tab
  })
  const baseTextOf = (key: string): string | null => {
    if (!baseMachineId) return null
    const pdef = PARAMS.find((x) => x.key === key)
    if (!pdef) return null
    const hit = effective(baseMachineId, null, key)
    return hit ? valueText(pdef, hit.value) : null
  }
  const rows: Json[] = []
  const diffKeys: string[] = []
  const notOwnKeys: string[] = []
  for (const pdef of params) {
    const row = rowOf(baseMachineId ?? 'A1', null, pdef)
    ;(row as Json).impact = null
    const cells = colDefs.map((c) => {
      const cell = cellOf(c.machineId, c.versionUid, pdef)
      const isBaseCol = c.machineId === baseMachineId && c.level === 'machine'
      const bt = baseTextOf(pdef.key)
      const differs = bt !== null && !isBaseCol && cell.text !== bt
      if (differs) {
        cell.differs = true
        cell.diffTip = `机型基底是 ${bt}`
      }
      return cell
    })
    if (cells.some((c) => c.differs === true)) diffKeys.push(pdef.key)
    rows.push(row)
    ;(row as Json).cells = cells
  }
  void notOwnKeys
  return {
    cols: colDefs,
    rows,
    totalRows: PARAMS.length,
    note: q ? '搜索跨全部分类' : null,
    emptyReason: null,
    diffKeys,
    notOwnKeys,
  }
}

/** 批量预览（C14 批量 + 产品纪律「先看后写」）：每一列的前后值与跳过原因 */
function previewBulkOf(key: string, value: unknown, cols: { machineId: string; versionUid: string | null }[]): Json {
  const pdef = PARAMS.find((x) => x.key === key)
  if (!pdef) {
    return { key, label: key, allowed: false, blockedReason: '字段定义里没有这一项', effects: [], skipped: [] }
  }
  const out: Json = {
    key,
    label: pdef.label,
    allowed: pdef.uiComponent !== 'gcode',
    blockedReason: pdef.uiComponent === 'gcode' ? 'G-code 不做批量' : null,
    effects: [] as Json[],
    skipped: [] as Json[],
  }
  if (!out.allowed) return out
  for (const c of cols) {
    const m = MACHINES.find((x) => x.id === c.machineId)
    const v = m?.versions.find((x) => x.uid === c.versionUid) ?? null
    const label = v ? v.name : '机型基底'
    const blocked = blockedOf(c.machineId, c.versionUid, pdef)
    if (blocked.length > 0) {
      ;(out.skipped as Json[]).push({
        col: c.versionUid ?? c.machineId, machine: m?.display ?? c.machineId, label,
        reason: '上一项没打开，这一项现在不生效，所以不让改', blocked,
      })
      continue
    }
    const hit = effective(c.machineId, c.versionUid, key)
    const hadOwn = hit?.origin === (c.versionUid === null ? 'machine' : 'version')
    const kind = hit && String(hit.value) === String(value) ? 'noChange' : hadOwn ? 'changing' : 'detaching'
    ;(out.effects as Json[]).push({
      col: c.versionUid ?? c.machineId, machine: m?.display ?? c.machineId, label,
      level: c.versionUid === null ? 'machine' : 'version',
      before: hit ? valueText(pdef, hit.value) : '不适用',
      after: valueText(pdef, value),
      kind,
    })
  }
  return out
}

function buildBook(): Json {
  const dirtyCount = pending.size + pendingVis.size
  return {
    machines: MACHINES.map((m) => ({
      id: m.id, display: m.display, icon: null, items: Object.keys(atRest.base[m.id] ?? {}).length,
      build: 'built', dimensionsMissing: false,
      versions: m.versions.map((v) => ({
        uid: v.uid, versionId: v.versionId, name: v.name, tag: v.tag,
        items: Object.keys(atRest.over[v.uid] ?? {}).length,
        build: 'stale', bbsSource: 'own', bbsCount: 2, recipeEmpty: false,
        lastBuild: '2026-09-20 10:00:00', orphanKeys: [],
      })),
    })),
    badges: { machines: 1, versions: 2, baseItems: 2, overrideItems: 1 },
    dirtyCount,
    save: dirtyCount > 0 ? 'dirty' : 'saved',
    artifact: 'stale',
    lastBuild: '2026-09-20 10:00:00',
    buildRows: MACHINE_VIEWS.flatMap((m) =>
      m.versions.map((v) => {
        const uid = `${m.id}/${v.id}`
        const built = builtRecords.has(uid)
        return {
          uid,
          machineId: m.id,
          machine: m.id,
          name: v.name,
          state: built ? 'built' : 'neverBuilt',
          reason: '',
          buildable: m.hasDimensions,
          disabledReason: m.hasDimensions ? null : '这台机型还没配尺寸（占位），不参与交付',
          mkpFile: built ? `MKPProcess_${m.id}_${v.id}.toml` : null,
          bbsCount: 1,
          bbsSource: 'own',
          lastBuild: built ? '2026-09-30 12:00:00（演示）' : null,
        }
      }),
    ),
    notices: [],
    snapshot: dirtyCount > 0 ? 'pending' : 'current',
  }
}

function buildRegistry(): Json {
  return {
    updated: '2026-09-01 08:00:00',
    tabs: TABS,
    params: PARAMS.map((pdef) => ({
      key: pdef.key, label: pdef.label, desc: pdef.desc, tomlKey: pdef.key,
      sectionId: pdef.sectionId, tabId: pdef.tabId, order: pdef.order,
      valueType: pdef.valueType, uiComponent: pdef.uiComponent,
      defaultValue: pdef.defaultValue, defaultText: valueText(pdef, pdef.defaultValue),
      min: pdef.min, max: pdef.max, step: pdef.step, unit: pdef.unit,
      choices: pdef.choices, showWhen: pdef.showWhen, parentKey: pdef.parentKey,
      machineFilter: pdef.machineFilter ?? [], deprecated: pdef.deprecated,
    })),
  }
}

/* ---------- 命令分发 ---------- */

export function installMockBackend() {
  const invoke = (cmd: string, args?: Json) => {
    const machineId = 'A1'
    switch (cmd) {
      case 'wb_boot':
        return Promise.resolve(mockBoot())
      /* 「软件版本」展示位：桩里报与真机同源的那个演示版本号 */
      case 'wb_app_version':
        return Promise.resolve('0.0.1')
      case 'wb_words':
        return Promise.resolve(WORDS)
      case 'wb_book':
        return Promise.resolve(buildBook())
      case 'wb_registry':
        return Promise.resolve(buildRegistry())
      case 'wb_desk':
        return Promise.resolve(buildDesk(machineId, (args?.uid as string | null) ?? null, (args?.tab as string | null) ?? null, (args?.query as string) ?? ''))
      case 'wb_matrix':
        return Promise.resolve(
          buildMatrix(
            (args?.cols as { machineId: string; versionUid: string | null }[]) ?? [],
            (args?.tab as string | null) ?? null,
            (args?.query as string) ?? '',
            (args?.baseMachineId as string | null) ?? null,
          ),
        )
      case 'wb_preview_bulk':
        return Promise.resolve(
          previewBulkOf(
            args?.key as string,
            args?.value,
            (args?.cols as { machineId: string; versionUid: string | null }[]) ?? [],
          ),
        )
      case 'wb_preflight':
        return Promise.resolve({
          issues: [
            {
              id: 'bundle.orphan_files',
              severity: 'hint',
              title: '有 2 个文件没进任何套餐',
              detail: 'a1-mini-bbs-02-010、a1-orca-02-010 —— 客户看得到它们，只是没有套餐推荐。仓库里放一个不分配给谁的 profile 是正常的交付身份，不是待修的事。',
              at: { view: 'menu', machineId: null, uid: null, key: null },
            },
          ],
          blocks: 0,
          todos: 1,
          hints: 1,
          emptyHint: '都过了 —— 没有阻断、没有待办、没有提示',
        })
      case 'wb_machines':
        return Promise.resolve(machineListOf())
      case 'wb_set_brand_field': {
        const brandId = args?.brandId as string
        const field = args?.field as 'name' | 'logo'
        const value = (args?.value as string | null) ?? null
        const b = BRANDS.find((x) => x.id.toLowerCase() === brandId.trim().toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `没有品牌 ${brandId}`, traceId: 'mock' })
        if (field === 'name') {
          if (!value || !value.trim()) {
            return Promise.reject({ code: 'INVALID', message: '显示名 不能清空', traceId: 'mock' })
          }
          b.name = value.trim()
        } else {
          b.logo = value && value.trim() !== '' ? value.trim() : null
        }
        return Promise.resolve(machineListOf())
      }
      case 'wb_add_brand': {
        const id = (args?.id as string).trim()
        const name = (args?.name as string).trim()
        if (id === '') return Promise.reject({ code: 'INVALID', message: '品牌 id 不能为空', traceId: 'mock' })
        if (name === '') return Promise.reject({ code: 'INVALID', message: '显示名不能为空', traceId: 'mock' })
        if (BRANDS.some((x) => x.id.toLowerCase() === id.toLowerCase())) {
          return Promise.reject({ code: 'INVALID', message: `已经有一个叫 ${id} 的品牌`, traceId: 'mock' })
        }
        BRANDS.push({ id, name, logo: null })
        return Promise.resolve(machineListOf())
      }
      case 'wb_move_machine_to_brand': {
        const machineId = args?.machineId as string
        const brandId = (args?.brandId as string).trim()
        const m = MACHINE_VIEWS.find((x) => x.id === machineId)
        if (!m) return Promise.reject({ code: 'NOT_FOUND', message: `没有机型 ${machineId}`, traceId: 'mock' })
        // 目标品牌必须真的存在 —— 与真机同一条校验（打错一个字会留下悬空归属）
        const b = BRANDS.find((x) => x.id.toLowerCase() === brandId.toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `没有品牌 ${brandId}`, traceId: 'mock' })
        m.brand = b.id
        return Promise.resolve(machineListOf())
      }
      case 'wb_set_machine_dimensions': {
        const machineId = args?.machineId as string
        const dims = args?.dimensions as typeof A1_DIMENSIONS
        const m = MACHINE_VIEWS.find((x) => x.id === machineId)
        if (!m) return Promise.reject({ code: 'NOT_FOUND', message: `没有机型 ${machineId}`, traceId: 'mock' })
        if (!(dims?.bedSize?.width > 0) || !(dims?.bedSize?.depth > 0)) {
          return Promise.reject({ code: 'INVALID', message: '床身宽与深都必须大于 0 —— 一台没有可打印面积的机器画不出床身图', traceId: 'mock' })
        }
        m.dimensions = dims
        m.hasDimensions = true
        return Promise.resolve(machineListOf())
      }
      case 'wb_set_param_meta': {
        /* 演示桩：同名的那条改掉就回。真机的校验（类型门 / 两级层级 / 枚举默认）
           在 registry::set_param_meta —— 桩里不做第二套 */
        const key = args?.key as string
        const e = args?.edit as Partial<FixtureParam>
        const target = PARAMS.find((x) => x.key === key)
        if (!target) return Promise.reject({ code: 'NOT_FOUND', message: `字段定义里没有 ${key}`, traceId: 'mock' })
        Object.assign(target, e)
        return Promise.resolve(buildRegistry())
      }
      case 'wb_set_machine_zones': {
        const machineId = args?.machineId as string
        const zones = (args?.zones as { points: [number, number][] }[]) ?? []
        const m = MACHINE_VIEWS.find((x) => x.id === machineId)
        if (!m) return Promise.reject({ code: 'NOT_FOUND', message: `没有机型 ${machineId}`, traceId: 'mock' })
        for (const [i, z] of zones.entries()) {
          if (z.points.length < 3) {
            return Promise.reject({ code: 'INVALID', message: `第 ${i + 1} 块禁区只有 ${z.points.length} 个点 —— 少于 3 个围不出面`, traceId: 'mock' })
          }
        }
        // 空数组 = 删掉禁区文件（真机口径：清空是删文件，不是留个空文件）
        m.zones = zones
        m.zoneCount = zones.length
        return Promise.resolve(machineListOf())
      }
      case 'wb_bundles':
        return Promise.resolve(bundleListOf((args?.query as string | null) ?? null))
      case 'wb_set_bundle_refs': {
        const bundleId = args?.bundleId as string
        const ids = (args?.assetIds as string[]) ?? []
        const b = BUNDLES.find((x) => x.id.toLowerCase() === bundleId.toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `查无此套餐：${bundleId}`, traceId: 'mock' })
        if (!ids.some((id) => ASSETS.find((x) => x.id === id)?.kind === 'slicerProfile')) {
          return Promise.reject({ code: 'INVALID', message: `套餐 ${bundleId} 的 assetRefs 里没有一条 BBS 预设`, traceId: 'mock' })
        }
        b.assetRefs = ids
        b.updatedAt = '今天（演示）'
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_add_bundle': {
        const id = (args?.id as string)?.trim() ?? ''
        const machineId = args?.machineId as string
        const display = args?.display as string
        const ids = (args?.assetIds as string[]) ?? []
        if (!id || /\s|[/]/.test(id)) return Promise.reject({ code: 'INVALID', message: '套餐 id 不能为空、不能含空白或 /', traceId: 'mock' })
        if (BUNDLES.some((x) => x.id.toLowerCase() === id.toLowerCase())) {
          return Promise.reject({ code: 'INVALID', message: `套餐 id 已经存在：${id}`, traceId: 'mock' })
        }
        if (!ids.some((x) => ASSETS.find((a) => a.id === x)?.kind === 'slicerProfile')) {
          return Promise.reject({ code: 'INVALID', message: '套餐的 assetRefs 里没有一条 BBS 预设', traceId: 'mock' })
        }
        BUNDLES.push({ id, display: display || '官方推荐', machineId, assetRefs: ids, updatedAt: '今天（演示）' })
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_rename_bundle': {
        const bundleId = args?.bundleId as string
        const newId = (args?.newId as string)?.trim() ?? ''
        const display = args?.display as string | null
        const b = BUNDLES.find((x) => x.id.toLowerCase() === bundleId.toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `查无此套餐：${bundleId}`, traceId: 'mock' })
        if (!newId || /\s|[/]/.test(newId)) return Promise.reject({ code: 'INVALID', message: '套餐 id 不能为空、不能含空白或 /', traceId: 'mock' })
        if (BUNDLES.some((x) => x.id.toLowerCase() === newId.toLowerCase() && x !== b)) {
          return Promise.reject({ code: 'INVALID', message: `套餐 id 已经存在：${newId}`, traceId: 'mock' })
        }
        b.id = newId
        if (display) b.display = display
        b.updatedAt = '今天（演示）'
        // 演示桩里的机型指向也跟着重指（真机由 Presets::rename_bundle 连带改机型文件）
        for (const m of MACHINE_VIEWS) {
          if (m.defaultBundle?.toLowerCase() === bundleId.toLowerCase()) m.defaultBundle = newId
          for (const v of m.versions) {
            if ((v.recommendedBundle ?? '').toLowerCase() === bundleId.toLowerCase()) v.recommendedBundle = newId
          }
        }
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_copy_bundle': {
        const bundleId = args?.bundleId as string
        const newId = (args?.newId as string)?.trim() ?? ''
        const display = args?.display as string | null
        const b = BUNDLES.find((x) => x.id.toLowerCase() === bundleId.toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `查无此套餐：${bundleId}`, traceId: 'mock' })
        if (BUNDLES.some((x) => x.id.toLowerCase() === newId.toLowerCase())) {
          return Promise.reject({ code: 'INVALID', message: `套餐 id 已经存在：${newId}`, traceId: 'mock' })
        }
        BUNDLES.push({ id: newId, display: display || b.display, machineId: b.machineId, assetRefs: [...b.assetRefs], updatedAt: '今天（演示）' })
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_assign_bundle_versions': {
        const bundleId = args?.bundleId as string
        const uids = (args?.uids as string[]) ?? []
        const b = BUNDLES.find((x) => x.id.toLowerCase() === bundleId.toLowerCase())
        if (!b) return Promise.reject({ code: 'NOT_FOUND', message: `查无此套餐：${bundleId}`, traceId: 'mock' })
        for (const uid of uids) {
          const [mid, vid] = uid.split('/')
          const m = MACHINE_VIEWS.find((x) => x.id.toLowerCase() === mid?.toLowerCase())
          const v = m?.versions.find((x) => x.id.toLowerCase() === vid?.toLowerCase())
          if (!m || !v) {
            return Promise.reject({ code: 'NOT_FOUND', message: `没有这个版本：${uid}`, traceId: 'mock' })
          }
          v.recommendedBundle = bundleId
        }
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_remove_bundle': {
        const bundleId = args?.bundleId as string
        const versions = MACHINE_VIEWS.flatMap((m) => m.versions.map((v) => ({ m, v })))
        const holders = [
          ...MACHINE_VIEWS.filter((m) => (m.defaultBundle ?? '').toLowerCase() === bundleId.toLowerCase()).map((m) => `机型 ${m.id} 的 defaultBundle`),
          ...versions.filter(({ v }) => (v.recommendedBundle ?? '').toLowerCase() === bundleId.toLowerCase()).map(({ m, v }) => `版本 ${m.id}/${v.id}`),
        ]
        if (holders.length) {
          return Promise.reject({ code: 'INVALID', message: `套餐 ${bundleId} 还被引用着，不能删`, detail: holders.join('、'), traceId: 'mock' })
        }
        const at = BUNDLES.findIndex((x) => x.id.toLowerCase() === bundleId.toLowerCase())
        if (at >= 0) BUNDLES.splice(at, 1)
        return Promise.resolve(bundleListOf(null))
      }
      case 'wb_assets':
        return Promise.resolve(
          assetListOf(
            (args?.kind as string | null) ?? null,
            (args?.slicer as string | null) ?? null,
            (args?.nozzle as string | null) ?? null,
            (args?.layer as string | null) ?? null,
            (args?.identity as string | null) ?? null,
            (args?.query as string | null) ?? null,
          ),
        )
      case 'wb_set_asset_delivery': {
        const assetId = args?.assetId as string
        const delivery = args?.delivery as 'download' | 'bundled'
        const a = ASSETS.find((x) => x.id === assetId)
        if (!a) return Promise.reject({ code: 'NOT_FOUND', message: `没有资产 ${assetId}`, traceId: 'mock' })
        if (a.kind === 'mkPreset' && delivery === 'bundled') {
          return Promise.reject({ code: 'INVALID', message: `资产 ${assetId} 是 MKP 预设登记，不能设成随包`, traceId: 'mock' })
        }
        a.delivery = delivery
        return Promise.resolve(assetListOf(null, null, null, null, null, null))
      }
      case 'wb_remove_asset': {
        const assetId = args?.assetId as string
        const used = BUNDLES.some((b) => b.assetRefs.includes(assetId)) || assetId === 'a1-icon'
        if (used) {
          return Promise.reject({ code: 'INVALID', message: `资产 ${assetId} 还被引用着，不能删`, traceId: 'mock' })
        }
        const at = ASSETS.findIndex((a) => a.id === assetId)
        if (at >= 0) ASSETS.splice(at, 1)
        return Promise.resolve(assetListOf(null, null, null, null, null, null))
      }
      case 'wb_generate_preview': {
        /*
         * 生成前预演（2026-10-02）。真机由 Rust 的 `build::preview_one` 逐份与磁盘比；
         * 这里按 `builtRecords`（"这台之前生成过没有"）造一份同形的报告，好让确认框在
         * 浏览器里能验收。**演示数据不冒充真渲染器**：正文头一行写着这是开发桩。
         *
         *   · 没生成过 → added（正文全绿）
         *   · 生成过   → unchanged（只占清单一行）
         */
        const scope = args?.scope as string | { picked: string[] }
        const rows = buildBook().buildRows as { uid: string; buildable: boolean; state: string }[]
        const picked =
          typeof scope === 'string'
            ? rows.filter((r) => r.buildable && (scope === 'all' || r.state === 'stale')).map((r) => r.uid)
            : (scope?.picked ?? [])
        const files: unknown[] = []
        const skipped: [string, string][] = []
        let toWrite = 0
        let unchangedN = 0
        for (const uid of picked) {
          const row = rows.find((r) => r.uid === uid)
          if (!row || !row.buildable) {
            skipped.push([uid, '这台机型还没配尺寸（占位），不参与交付'])
            continue
          }
          const fileName = `${uid.replace('/', '-')}.toml`
          if (builtRecords.has(uid)) {
            unchangedN += 1
            files.push({ uid, fileName, state: 'unchanged', lines: [], added: 0, removed: 0 })
          } else {
            toWrite += 1
            const text = [
              '# 开发桩渲染的演示产物 —— 真产物由 Rust 的 build::render() 出',
              `# machine: ${uid.split('/')[0]}`,
              '',
              '[demo]',
            ]
            files.push({
              uid,
              fileName,
              state: 'added',
              lines: text.map((t, i) => ({ kind: 'added', text: t, no: i + 1 })),
              added: text.length,
              removed: 0,
            })
          }
        }
        return Promise.resolve({ files, skipped, toWrite, unchanged: unchangedN, blocked: null })
      }
      case 'wb_generate': {
        const scope = args?.scope as string | { picked: string[] }
        const rows = buildBook().buildRows as { uid: string; buildable: boolean; state: string }[]
        const picked =
          typeof scope === 'string'
            ? rows.filter((r) => r.buildable && (scope === 'all' || r.state === 'stale')).map((r) => r.uid)
            : (scope?.picked ?? [])
        const written: string[] = []
        const unchanged: string[] = []
        const skipped: [string, string][] = []
        for (const uid of picked) {
          const row = rows.find((r) => r.uid === uid)
          if (!row || !row.buildable) {
            skipped.push([uid, '这台机型还没配尺寸（占位），不参与交付'])
            continue
          }
          if (builtRecords.has(uid)) unchanged.push(uid)
          else written.push(uid)
          builtRecords.add(uid)
        }
        return Promise.resolve({
          stamp: '2026-09-30 12:00:00（演示）',
          written,
          unchanged,
          skipped,
          mark: {
            kind: 'markBuilt',
            uids: [...written, ...unchanged],
            stamp: '2026-09-30 12:00:00（演示）',
            fingerprints: {},
          },
        })
      }
      case 'wb_publish_audit':
        return Promise.resolve(mockAudit())
      /*
       * 发布事务（第三刀下半）：一次调用走完审计 → 生成 → 定稿 → 本地 git → 平台 PR。
       * 桩走向**成功那一路**（演示"一条龙"是什么样）。真机由 Rust 的
       * `publish_tx::run` 跑；平台方言在那个模块里被收敛成统一的 ReviewState / ChecksSummary。
       */
      case 'wb_publish':
        return Promise.resolve({
          stage: 'reviewOpened',
          auditPassed: 15,
          auditFailed: 0,
          generated: 9,
          unchanged: 0,
          committedPaths: ['presets/dist/', 'presets/structure-signatures.toml', 'presets/assets.toml'],
          review: {
            platform: 'github',
            number: 128,
            url: 'https://github.com/MuCoreBenC/MKPSupportEase/pull/128',
            state: 'open',
            checks: 'pending',
            title: '发布：交付产物 21 份',
            head: 'publish/0.0.2',
            base: 'main',
          },
          branch: 'publish/0.0.2',
          commit: '08ec040',
          files: 21,
          summary: '已生成 9 份、定稿 21 份产物。已提交并推送到 `publish/0.0.2`。已建 PR !128。',
        })
      /* 发布账户现状（演示）：GitHub 配好了、Gitee 还没配（尾号提示不是原值） */
      case 'wb_publish_account':
        return Promise.resolve({
          platforms: [
            {
              platform: 'github',
              repositoryUrl: 'git@github.com:MuCoreBenC/MKPSupportEase.git',
              username: 'MuCoreBenC',
              hasToken: true,
              tokenHint: '…d4e5',
            },
            { platform: 'gitee', repositoryUrl: '', username: '', hasToken: false, tokenHint: null },
          ],
          remoteUrl: 'git@github.com:MuCoreBenC/MKPSupportEase.git',
          remoteMatchesConfig: true,
          branch: 'publish/0.0.2',
        })
      case 'wb_set_publish_account': {
        const platform = String(args?.platform ?? 'github')
        return Promise.resolve({
          platform,
          repositoryUrl: String(args?.repositoryUrl ?? ''),
          username: String(args?.username ?? ''),
          hasToken: false,
          tokenHint: null,
        })
      }
      case 'wb_set_publish_token': {
        const platform = String(args?.platform ?? 'github')
        return Promise.resolve({
          platform,
          repositoryUrl: '',
          username: '',
          hasToken: true,
          tokenHint: '…ab12',
        })
      }
      case 'wb_clear_publish_account': {
        const platform = String(args?.platform ?? 'github')
        return Promise.resolve({
          platform,
          repositoryUrl: '',
          username: '',
          hasToken: false,
          tokenHint: null,
        })
      }
      case 'wb_publish_status': {
        // 演示脚本：#128 已经合了；别的编号回读一律"CI 已经跑完" —— 刷新看得出变化
        const n = Number(args?.number ?? 128)
        return Promise.resolve({
          platform: 'github',
          number: n,
          url: `https://github.com/MuCoreBenC/MKPSupportEase/pull/${n}`,
          state: n === 128 ? 'merged' : 'open',
          checks: 'passed',
          title: '发布：交付产物 21 份',
          head: 'publish/0.0.2',
          base: 'main',
        })
      }
      /* 合并（演示）：一律 squash，回读后落到 merged —— 与真机同一条口径 */
      case 'wb_merge_review':
        return Promise.resolve({
          platform: 'github',
          number: Number(args?.number ?? 128),
          url: `https://github.com/MuCoreBenC/MKPSupportEase/pull/${Number(args?.number ?? 128)}`,
          state: 'merged',
          checks: 'passed',
          title: '发布：交付产物 21 份',
          head: 'publish/0.0.2',
          base: 'main',
        })
      /*
       * 发布历史（演示）：三条 —— 已合并 / 等待合并 / 停在发布检查。
       * 真机读 `<appDataDir>/publish-history.json`（`history::load`）。
       */
      case 'wb_publish_history':
        return Promise.resolve({
          historySchema: 1,
          records: [
            {
              at: '2026-10-04T14:02:00+08:00',
              stage: 'statusRead',
              branch: 'publish/0.0.2',
              commit: '08ec040',
              review: {
                platform: 'github',
                number: 128,
                url: 'https://github.com/MuCoreBenC/MKPSupportEase/pull/128',
                state: 'merged',
                checks: 'passed',
                title: '发布：交付产物 21 份',
                head: 'publish/0.0.2',
                base: 'main',
              },
              files: 21,
              generated: 0,
              auditPassed: 15,
              auditFailed: 0,
              summary: '已生成 0 份、定稿 21 份产物。已提交并推送到 `publish/0.0.2`。已建 PR !128。',
            },
            {
              at: '2026-10-04T11:20:00+08:00',
              stage: 'reviewOpened',
              branch: 'feat/demo',
              commit: '1a2b3c4',
              review: {
                platform: 'github',
                number: 127,
                url: 'https://github.com/MuCoreBenC/MKPSupportEase/pull/127',
                state: 'open',
                checks: 'pending',
                title: '发布：交付产物 18 份',
                head: 'feat/demo',
                base: 'main',
              },
              files: 18,
              generated: 9,
              auditPassed: 15,
              auditFailed: 0,
              summary: '已生成 9 份、定稿 18 份产物。已提交并推送到 `feat/demo`。已建 PR !127。',
            },
            {
              at: '2026-10-03T18:44:00+08:00',
              stage: 'blockedAudit',
              branch: null,
              commit: null,
              review: null,
              files: 0,
              generated: 0,
              auditPassed: 14,
              auditFailed: 1,
              summary: '发布闸没全绿 —— 一个字节都没写。先照「去修」把红项处理掉',
            },
          ],
        })
      /* 打开系统浏览器（演示）：桩里只记一笔，不真开 */
      case 'wb_open_external':
        console.info('[mock] openExternal', args?.url)
        return Promise.resolve(undefined)
      case 'wb_preview_toml': {
        /*
         * 单独看一份产物的正文。**真产物由 Rust 的 `build::render()` 出**（段名取
         * `param.section`、共享 tomlKey 的参数合成内联表、注释按 `tomlComment`…）。
         * 这里只把「哪一版、哪些值」按 TOML 的样子摊平，好让「查看 TOML」这个入口
         * 在浏览器里能验收；正文头一行就写着这是开发桩，不冒充真渲染器。
         */
        const uid = String(args?.uid ?? '')
        const [mid, vid] = uid.split('/')
        const m = MACHINES.find((x) => x.id === mid)
        if (!m || !m.versions.some((v) => v.uid === uid)) {
          return Promise.reject({ code: 'NOT_FOUND', message: `查无此版本：${uid}`, traceId: 'mock' })
        }
        const out = [
          '# 开发桩渲染的演示产物 —— 真产物由 Rust 的 build::render() 出',
          `# machine: ${mid}`,
          `# variant: ${(vid ?? '').toLowerCase()}`,
          '',
          '[demo]',
        ]
        for (const pdef of PARAMS) {
          const hit = effective(mid, uid, pdef.key)
          if (!hit) continue
          out.push(`${pdef.key.split('.').pop()} = ${JSON.stringify(String(hit.value))}`)
        }
        return Promise.resolve(`${out.join('\n')}\n`)
      }
      case 'wb_baseline_diff':
        return Promise.resolve(BASELINE)
      case 'wb_sync_baseline': {
        const n = BASELINE.filter((b) => b.status !== 'same').length
        for (const b of BASELINE) {
          b.baselineSha = b.productSha
          b.status = 'same'
        }
        return Promise.resolve(n)
      }
      case 'wb_dist_strays':
        return Promise.resolve([...mockStrays])  // dist 面的残留（dist 相对）
      case 'wb_clean_dist_strays': {
        const n = mockStrays.length
        mockStrays = []
        return Promise.resolve(n)
      }
      case 'wb_trash':
        return Promise.resolve(TRASH)
      case 'wb_asset_usage': {
        const assetId = args?.assetId as string
        const machines = MACHINE_VIEWS.filter(
          (m) => m.image === assetId || m.icon === assetId,
        ).map((m) => m.id)
        const bundles = BUNDLES.filter((b) => b.assetRefs.includes(assetId)).map((b) => b.id)
        return Promise.resolve({ id: assetId, machines, bundles })
      }
      case 'wb_asset_inspect': {
        const assetId = args?.assetId as string
        const a = ASSETS.find((x) => x.id === assetId)
        if (!a) return Promise.reject({ code: 'NOT_FOUND', message: `没有资产 ${assetId}`, traceId: 'mock' })
        return Promise.resolve(assetInspectOf(a))
      }
      case 'wb_reveal_asset': {
        // 演示后端没有系统文件管理器 —— 如实说（真机上这一条会打开它并选中文件）。
        // 探针断言的就是「演示里点它要说实话」，不是静默成功
        return Promise.reject({
          code: 'NOT_IMPLEMENTED',
          message: '浏览器演示里没有系统文件管理器 —— 真机上会打开它并选中这个文件',
          traceId: 'mock',
        })
      }
      case 'wb_apply_draft': {
        const patches = (args?.patches as Json[]) ?? []
        const inverse: Json[] = []
        const atRestOf = (level: 'machine' | 'version', owner: string, key: string) => {
          const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
          return table && key in table ? table[key] : null
        }
        for (const patch of patches) {
          if (patch.kind === 'setVisibility') {
            const { fileId, visibility } = patch as never as { fileId: string; visibility: string }
            const before = pendingVis.get(fileId) ?? 'menu'
            if (before === visibility) continue
            inverse.push({ kind: 'setVisibility', fileId, visibility: before })
            // 改回「在菜单」= 拿掉草稿（与后端 draft 的口径一致：Menu 是缺省）
            if (visibility === 'menu') pendingVis.delete(fileId)
            else pendingVis.set(fileId, visibility)
            continue
          }
          if (patch.kind !== 'setValue') continue
          const { level, owner, key, value } = patch as never as { level: 'machine' | 'version'; owner: string; key: string; value: unknown }
          const vk = `${level}|${owner}|${key}`
          const before = pending.has(vk) ? pending.get(vk) : atRestOf(level, owner, key)
          if (before === value) continue
          inverse.push({ kind: 'setValue', level, owner, key, value: before })
          // 改回了盘上那个值 = 从草稿里拿掉（照后端 apply_one 的规矩）
          if (atRestOf(level, owner, key) === value) pending.delete(vk)
          else pending.set(vk, value)
        }
        inverse.reverse()
        const refresh = args?.refresh as Json | null
        const desk = refresh && refresh.page === 'desk'
          ? buildDesk(machineId, (refresh.uid as string | null) ?? null, (refresh.tab as string | null) ?? null, (refresh.query as string) ?? '')
          : null
        return Promise.resolve({ view: buildBook(), inverse, undoable: true, desk, matrix: null })
      }
      case 'wb_save': {
        for (const [vk, value] of pending) {
          const { level, owner, key } = splitKey(vk)
          const table = level === 'machine' ? atRest.base[owner] : atRest.over[owner]
          if (table) {
            if (value === null) delete table[key]
            else table[key] = value
          }
        }
        pending.clear()
        return Promise.resolve({ view: buildBook(), remap: {} })
      }
      case 'wb_discard':
        pending.clear()
        return Promise.resolve(buildBook())
      case 'wb_diff_draft':
        return Promise.resolve([])
      case 'wb_ui':
        return Promise.resolve({})
      case 'wb_save_ui':
        return Promise.resolve(null)
      case 'wb_set_bootstrap': {
        const url = String(args?.url ?? '').trim()
        if (url === '') {
          return Promise.reject({ code: 'INVALID', message: 'Bootstrap 地址是空的', traceId: 'mock' })
        }
        /* 演示桩不做 GitHub blob → raw 的规范化（那是真后端 `dist::normalize_bootstrap_url`
           的活）—— 存原样；真机存下去的是转好的 raw 直链 */
        mockBootstrap = url
        return Promise.resolve(url)
      }
      case 'wb_reload':
        return Promise.resolve(mockBoot())
      default:
        return Promise.reject({ code: 'NOT_IMPLEMENTED', message: `开发桩没有实现 ${cmd}`, traceId: 'mock' })
    }
  }
  ;(window as unknown as Json).__TAURI_INTERNALS__ = { invoke }
  console.info('[workbench] dev mock backend installed —— 没有 Tauri，写路径只落在内存里')
}

