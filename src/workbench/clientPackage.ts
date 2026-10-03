/*
 * 工作台 · **说明书生成**（`ClientDataPackage`）+「一次发布」的装配。
 *
 * # 这是「联动」那条链的缺环
 *
 * 一次发布 = 说明书 + N 份 `preset.toml`（客户端在「预设」页按份下载的那几份），
 * 两样同属一个 preset identity（T7.1）。本仓的后半样一直在 —— `wb_preview_toml` 就是生成
 * 产物那台渲染器；前半样没有产出者，于是「上传到云端」按不动。这一份文件补的就是它。
 * （2026-10-02 校准：客户端 C4 起不读 `STORAGE.cloud` 那一格、「同步」页也退役了；
 * 说明书的目标形态是"客户端自动获取"—— Bootstrap 那一刀。见 `cloud.ts` 头注释。）
 *
 * # 它读的是**后端报回来的保存态**
 *
 * 机型 / 版本 / 字段定义 / 各版本的有效值 / 产物名单，全部来自 `wb_*` 那几条**读**命令
 * （它们读的是盘上落盘的那一份）；这一层只做**装配**：挑字段、排顺序、摊平值。
 * 所以「没保存的改动不许进包」这条边界不是靠前端自觉 —— 是**拿不到**：草稿住在 Rust 那侧，
 * 读命令答的永远是已保存的那一份。
 *
 * # 四条刻意的规矩（与试验场 C15 的 `buildClientPackage` 逐条相同）
 *
 *   1. **摊平，不给继承**。包里只有「这个版本用什么值」（三层算完的有效值），
 *      没有 origin、没有机型基底这一层 —— 继承规则是后厨的账，客户端重算必定分叉。
 *   2. **字段集 = 各机型真实看得到的字段之并集**（`machineFilter` 让每台机型的字段集不同），
 *      **顺序走布局表**（不是字母序）—— 客户端照着它排分组。
 *   3. `tab` / `group` 给的是**中文名**；另外带上 `tabId` / `uiComponent` / `valueType` /
 *      `showWhen`（A40 补的四栏）。客户端不再拿「用哪种控件」猜「值是什么类型」，
 *      也不再自己写死「哪些参数属于哪个模式」。
 *   4. **文件清单只写真有的那几样**：MKP 那一份取产物名单（`BuildRow.mkpFile`，没有产物的
 *      版本这一格就是空的），切片器那几份按版本自己指的套餐（`recommendedBundle`）摊开。
 *
 * # 本该在哪一层（产品仓的纪律，登记在案）
 *
 * 「前端不算业务」是既定纪律（14b 把 C14 的 derive 整批搬进了 Rust），而**产物生成**
 * 本来就在 Rust（`wb_publish` / `wb_preview_toml` 那一侧）。这一份生成器同属那一类，
 * **归宿是 Rust**。本轮按 `C15-A40-PORT-PLAN.md` §6「仍待定」里那条本轮建议，
 * 先落在**这一处模块**里 —— 好让「工作台生成 → 上传」这一段当场走得通
 * （决策 #5：本轮就要求真的联通；客户端那半条链的现状见 `docs/PROJECT-AUDIT.md` ⑧）
 * —— 同时登记成债
 * （`tasks.md` 债 #1 / #8）。搬过去那天，界面一个字不动，换的是它们后面那个人。
 */
import type {
  ClientDataPackage,
  ClientFieldDef,
  ClientMachine,
  ClientDataMeta,
  FileRef,
  Release,
  ReleasePreset,
} from '../api/contract'
import { wb } from './api'
import type {
  AssetView,
  BuildRow,
  BundleView,
  Desk,
  MachineView,
  ParamView,
  RegistryView,
  Row,
  UiComponent,
  VersionView,
} from './api'

/** 说明书的结构版本。契约那一版就是 1 —— 兼容性清单把 ≠1 读成「下一代结构」（2.0.0） */
export const CLIENT_PKG_SCHEMA = 1 as const

/**
 * 本稿五种控件 → 契约那四档。
 *
 * 分段与下拉并成 `choice`（画成什么由宿主的 `form` 决定），G-code 落 `text` ——
 * 想细分就读 `uiComponent`（那一栏带的是原词）。
 */
const CLIENT_CONTROL: Record<UiComponent, ClientFieldDef['control']> = {
  number: 'number',
  switch: 'switch',
  segmented: 'choice',
  select: 'choice',
  gcode: 'text',
  text: 'text',
}

/** 交给客户端的值一律字符串 —— 契约里 `choices[].value` 与 `showWhen.value` 都是 string */
const asText = (v: unknown): string => String(v)

export const uidOf = (machineId: string, versionId: string): string => `${machineId}/${versionId}`

/**
 * 装配一份说明书要的全部输入。**每一项都是后端读命令的返回**，这一层不编数据。
 *
 * 单独抽出来是为了让 `buildClientPackage` 是个**纯函数**（同一份输入 → 同一份包），
 * 取数那半边（`collectInputs`）单独看得见。
 */
export interface PackageInputs {
  machines: MachineView[]
  registry: RegistryView
  bundles: BundleView[]
  assets: AssetView[]
  /** 有产物的版本 → 它的 MKP 文件名（`BuildRow.mkpFile`）。没产物的版本不在这张表里 */
  mkpFiles: Map<string, string>
  /** 每台机型的参数台：列是「基底 + 这台所有版本」，用它拿各版本的有效值 */
  desks: Map<string, Desk>
}

/**
 * 取装配输入。
 *
 * 六台机型就六次 `wb_desk` —— 这是「一次手势一次 IPC」的正常量级，但它确实是**一次
 * 全量读**：只在进页 / 后端写过之后跑（`tick`），不让它跟着每一次输入重算。
 */
export async function collectInputs(): Promise<PackageInputs> {
  const [list, registry, book, bundles, assets] = await Promise.all([
    wb.machines(),
    wb.registry(),
    wb.book(),
    wb.bundles(null),
    wb.assets(null, null, null, null, null, null),
  ])
  const desks = await Promise.all(list.machines.map((m) => wb.desk(m.id, null, null, '')))
  return {
    machines: list.machines,
    registry,
    bundles: bundles.bundles,
    assets: assets.assets,
    mkpFiles: new Map(
      book.buildRows
        .filter((r: BuildRow) => r.mkpFile !== null && r.mkpFile !== undefined)
        .map((r) => [r.uid, r.mkpFile as string]),
    ),
    desks: new Map(list.machines.map((m, i) => [m.id, desks[i]])),
  }
}

/** 库存里的一条切片器资源 → 给客户端的文件引用 */
function fileRefOfAsset(a: AssetView): FileRef | undefined {
  /* 图 / 图标 / 模型不进包 —— 那是界面上给人看的素材，不是要下载的东西
     （契约的 `FileKind` 也只有三类预设） */
  if (a.kind !== 'slicerProfile') return undefined
  return {
    kind: a.slicer === 'orca' ? 'orca_profile' : 'bbs_profile',
    /* 显示名（`name`）是给人看的（「A1：0.4 喷头 0.20 层高」），文件名要的是真文件 */
    fileName: a.path.split('/').pop() ?? a.name,
    path: `presets/${a.path}`,
  }
}

/** 一个版本的袋子里装什么：自己的 MKP（有产物才算）+ 套餐里摊开的切片器 */
function filesOf(inp: PackageInputs, m: MachineView, v: VersionView): FileRef[] {
  const out: FileRef[] = []
  const mkp = inp.mkpFiles.get(uidOf(m.id, v.id))
  if (mkp !== undefined) {
    out.push({ kind: 'mkp_preset', fileName: mkp, path: `presets/mkp/${mkp}` })
  }
  /* 一版一套：版本自己指的套餐。指了但找不到（悬空名）就是没有，不猜 */
  const want = (v.recommendedBundle ?? '').toLowerCase()
  const bundle = want === '' ? undefined : inp.bundles.find((b) => b.id.toLowerCase() === want)
  for (const ref of bundle?.assetRefs ?? []) {
    if (!ref.resolvable) continue
    const a = inp.assets.find((x) => x.id === ref.id)
    const f = a === undefined ? undefined : fileRefOfAsset(a)
    if (f !== undefined) out.push(f)
  }
  return out
}

/** 参数台那一列摊成「key → 文本值」。`at < 0` = 这台机型的台上没有这个版本 */
function valuesOf(desk: Desk | undefined, versionUid: string): Record<string, string> {
  if (desk === undefined) return {}
  const at = desk.cols.findIndex((c) => c.versionUid === versionUid)
  if (at < 0) return {}
  const out: Record<string, string> = {}
  const take = (row: Row) => {
    const cell = row.cells[at]
    /* 空格子（这台机型没这一项）与空值都不进包 —— 客户端拿不到「空」和「没有」的区别 */
    if (cell === undefined || cell.kind === 'notApplicable') return
    if (cell.raw === null || cell.raw === undefined || cell.raw === '') return
    out[row.key] = asText(cell.raw)
  }
  for (const g of desk.groups) {
    for (const it of g.items) {
      take(it.row)
      for (const child of it.children) take(child)
    }
  }
  return out
}

/** 这一项在这台机型上存不存在（`machineFilter` 空 = 不限机型） */
const visibleOn = (p: ParamView, machineId: string): boolean =>
  p.machineFilter.length === 0 || p.machineFilter.includes(machineId)

/**
 * 给客户端的**显示元信息表**（`ClientFieldDef[]`）。
 *
 * 顺序 = 布局表（页 → 组 → 项，各按后端的 `order`），不是注册表的字母序 ——
 * 客户端照着它排分组，顺序错了界面上就是另一份东西。
 */
export function clientFieldsOf(inp: PackageInputs): ClientFieldDef[] {
  const union = new Set<string>()
  for (const m of inp.machines) {
    for (const p of inp.registry.params) if (visibleOn(p, m.id)) union.add(p.key)
  }

  const bySection = new Map<string, ParamView[]>()
  for (const p of inp.registry.params) {
    const list = bySection.get(p.sectionId)
    if (list === undefined) bySection.set(p.sectionId, [p])
    else list.push(p)
  }
  const byOrder = <T extends { order: number }>(a: T, b: T) => a.order - b.order

  const out: ClientFieldDef[] = []
  const seen = new Set<string>()
  for (const tab of [...inp.registry.tabs].sort(byOrder)) {
    for (const section of [...tab.sections].sort(byOrder)) {
      for (const p of [...(bySection.get(section.id) ?? [])].sort(byOrder)) {
        if (seen.has(p.key) || !union.has(p.key)) continue
        seen.add(p.key)
        out.push({
          key: p.key,
          label: p.label,
          /* 空说明不给这一栏 —— 空串会让客户端画出一行空的提示 */
          desc: p.desc === '' ? undefined : p.desc,
          unit: p.unit ?? undefined,
          control: CLIENT_CONTROL[p.uiComponent],
          uiComponent: p.uiComponent,
          valueType: p.valueType,
          showWhen:
            p.showWhen === null
              ? undefined
              : { key: p.showWhen.key, op: p.showWhen.op, value: asText(p.showWhen.value) },
          choices:
            p.choices.length === 0
              ? undefined
              : p.choices.map((c) => ({ value: asText(c.value), label: c.label })),
          min: p.min ?? undefined,
          max: p.max ?? undefined,
          step: p.step ?? undefined,
          group: section.label,
          tab: tab.label,
          tabId: tab.id,
        })
      }
    }
  }
  return out
}

/** 柜台上单卖的：身份「可选」（= 不入任何套餐、非随包）、又不是「仅归档」的那些 */
export function optionalFilesOf(inp: PackageInputs): FileRef[] {
  return inp.assets
    .filter((a) => a.kind === 'slicerProfile' && a.identity === 'optional')
    .map(fileRefOfAsset)
    .filter((f): f is FileRef => f !== undefined)
}

/**
 * 输入指纹：这一份包**内容**的稳定摘要（FNV-1a，8 位十六进制）。
 *
 * 契约说它「只回答现在的输入和上次生成时是不是同一份」—— 那就把装配结果本身摘一遍：
 * 同一份输入出来的包逐字节相同，指纹自然相同；哪一格变了它跟着变。
 * 刻意**不带生成时刻**（`ClientDataPackage` 里根本没有 `generatedAt` 这一栏）：
 * 「这是第几版、什么时候发的」记在**云端目录项**上（谁上传谁记），不往包里塞。
 */
function inputsHashOf(content: Omit<ClientDataPackage, 'inputsHash'>): string {
  const blob = JSON.stringify(content)
  let h = 0x811c9dc5
  for (let i = 0; i < blob.length; i += 1) {
    h ^= blob.charCodeAt(i)
    h = Math.imul(h, 0x01000193) >>> 0
  }
  return (`00000000${h.toString(16)}`).slice(-8)
}

/**
 * **生成客户端数据包** —— 工作台这一侧的那份「成品 JSON」。
 *
 * `meta.minClientVersion` 照**上游 manifest 声明的那一格**原样带（未声明就是 null）。
 * 「自动判断」算出来的那个数是**建议**（见 `compat.ts` 的 `minClientOf`），
 * 用来告诉人「这一格该填多少」—— 本仓不替上游写它的 manifest，所以不往包里塞。
 */
export function buildClientPackage(
  inp: PackageInputs,
  meta: Pick<ClientDataMeta, 'minClientVersion'>,
): ClientDataPackage {
  const machines: ClientMachine[] = inp.machines.map((m) => ({
    id: m.id,
    /* 实测有机型的 `name` 是空串而 `display` 才是给人看的 */
    display: m.display || m.id,
    brand: m.brand,
    /*
     * 尺寸那一族**本仓的机型视图没带**（`MachineView` 只报「有没有配」`hasDimensions`）——
     * 如实给 null，不编一份尺寸出来。客户端读包里这一栏的地方只有一句计数
     * （`src/app/store/package.ts` 的汇总），所以这不算缺口；真要把尺寸带过去，
     * 得先让后端在 `wb_machines` 里补那一族（与本文件头那笔债同一笔）。
     */
    dimensions: null,
    versions: m.versions.map((v) => {
      const files = filesOf(inp, m, v)
      return {
        id: v.id,
        name: v.name,
        tag: v.tag === null || v.tag === '' ? undefined : v.tag,
        description: v.description === null || v.description === '' ? undefined : v.description,
        files,
        values: valuesOf(inp.desks.get(m.id), uidOf(m.id, v.id)),
        /* 一个文件都没有 = 这台机型/版本在资源侧还是占位（A2L 那种），客户端要显示它但不给下载 */
        unsupported: files.length === 0,
      }
    }),
  }))

  const content = {
    meta: { schemaVersion: CLIENT_PKG_SCHEMA, minClientVersion: meta.minClientVersion },
    machines,
    fields: clientFieldsOf(inp),
    optionalFiles: optionalFilesOf(inp),
  }
  return { ...content, inputsHash: inputsHashOf(content) }
}

/**
 * **一次发布**：说明书 + N 份预设文件。
 *
 * 预设文件不在这里拼 —— 逐份问 `wb_preview_toml`（**后端那台渲染器**），
 * 否则「看到的」和「发出去的」就不是同一份东西了。清单就是产物名单
 * （`BuildRow.mkpFile` 非空的那几个版本，与交付目录里 `presets/mkp/` 同一批）。
 */
export async function buildRelease(
  inp: PackageInputs,
  opts: { version: string; at: string } & Pick<ClientDataMeta, 'minClientVersion'>,
): Promise<Release> {
  const presets: ReleasePreset[] = await Promise.all(
    [...inp.mkpFiles].map(async ([uid, fileName]) => {
      const [machineId, versionId] = uid.split('/')
      return { machineId, versionId, fileName, content: await wb.previewToml(uid) }
    }),
  )
  return {
    version: opts.version,
    at: opts.at,
    package: buildClientPackage(inp, { minClientVersion: opts.minClientVersion }),
    presets,
  }
}