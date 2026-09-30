/*
 * BBS 预设的链解析、合并与差异判定 —— 译自 machine-motion/src/bbs/preset-merge.js，
 * 逻辑一行没改，只加了类型。纯函数：不碰 DOM、不发请求。
 *
 * 要解决的事：BBS 的用户预设是**增量**的 —— 一份用户工艺预设只记它相对 `inherits`
 * 那条系统预设改了哪几项。而系统预设自己**也是增量的**，一层层往上继承到
 * `fdm_process_common`。所以基准只能顺着 `inherits` 现算，不能写死某一条链：
 *
 *     0.20mm Standard @BBL A1M  ← fdm_process_single_0.20 ← fdm_process_single_common ← fdm_process_common
 *     0.20mm Standard @BBL H2D  ← fdm_process_dual_0.20_nozzle_0.4 ← fdm_process_dual_common ← fdm_process_common
 *
 * 实测各机型的 0.20mm Standard 之间差 1~48 项（A1 只差 1，H2C 差 48），写死一条链换机型就全错。
 * 链里没覆盖到的参数（三百多个里有一半）来自 BBS 程序内置的出厂默认，由 `public/bbs/defaults.json`
 * 兜在最底层，那份是上游从 PrintConfig.cpp 直接提的。
 *
 * 判定「改动」的口径与 BBS 自己一致：**逐 key 比值**（BBS 是 `Preset::dirty_options()` →
 * `deep_diff`，拿编辑态完整配置对父预设完整配置比），不是「key 出现过就算改」—— 用户预设里
 * 完全可能存着一个与默认相同的值（改过又改回去），那种在 BBS 界面上是不标橙的。
 */

import type {
  BbsParamMeta,
  BbsPresetDoc,
  BbsScalar,
  BbsValue,
  BbsValues,
} from './bbsTypes'

/* 预设的身份信息，不是参数，任何环节都不参与合并与比对。
 * `print_extruder_id` / `print_extruder_variant` 也在里面：它们是预设与挤出头的绑定，
 * BBS 工艺面板上没有这两行，但 variantIndex() 会单独读 variant 来决定数组取第几个值。 */
export const META_KEYS = new Set([
  'inherits', 'name', 'from', 'instantiation', 'description', 'version',
  'setting_id', 'type', 'metadata', 'print_settings_id', 'filament_id',
  'print_extruder_id', 'print_extruder_variant',
  'compatible_printers', 'compatible_printers_condition', 'compatible_models',
  '_meta', '_note', 'bbs_version', 'source',
])

/* 数组值有两种来源，都按下标取：
 *   - 多挤出头（H2D 真双头）
 *   - 挤出头变体（X1C 是单喷嘴，但 outer_wall_speed 有 ["200","350"] 两个值，
 *     对应 Direct Drive / Bowden 之类的变体）
 * 下标由 variantIndex() 给；越界就回落到第 0 个，不要让页面显示 undefined。 */
export function norm(v: BbsValue | undefined, idx = 0): BbsScalar | undefined {
  if (Array.isArray(v)) return v[idx] ?? v[0]
  return v
}

/* 布尔的宽松判定：'1'/'true'/'yes'/'on'/非零数字 都算开。
 * BBS 里同一个布尔在不同层可能写成 '1' 或 'true'，严格比会判成两个值。 */
export function truthy(v: BbsValue | undefined, idx = 0): boolean {
  const raw = norm(v, idx)
  if (raw === true) return true
  if (raw === false || raw === undefined || raw === null) return false
  const s = String(raw).trim().toLowerCase()
  if (s === '' || s === '0' || s === 'false' || s === 'off' || s === 'no' || s === 'nil') return false
  if (s === 'true' || s === 'yes' || s === 'on' || s === '1') return true
  const n = Number(s)
  return Number.isFinite(n) ? n !== 0 : true
}

/* 剔掉 META、统一小写 key、丢掉 null/''。
 * 空值要丢：一层继承里写个空串不代表「把这个参数设成空」，留着会把下游基准洗空。 */
export function stripMeta(doc: BbsPresetDoc | null | undefined): BbsValues {
  const out: BbsValues = {}
  if (!doc || typeof doc !== 'object' || Array.isArray(doc)) return out
  for (const k of Object.keys(doc)) {
    if (META_KEYS.has(k)) continue
    const v = doc[k]
    if (v === null || v === undefined || v === '') continue
    if (Array.isArray(v) && v.length === 0) continue
    out[k.toLowerCase()] = v as BbsValue
  }
  return out
}

export interface ChainResult {
  chain: BbsPresetDoc[]
  missing: string[]
  looped: boolean
}

/* 顺着 inherits 往上走，直到某层没有 inherits。
 * `getDoc(name)` 由调用方给（页面是「索引查名 → fetch → 缓存」，node 核对时直接读盘）。
 * seen 是环形 inherits 的保险 —— 上游数据出问题时宁可截断也不要把页面挂死。 */
export function resolveChain(
  name: string | null,
  getDoc: (name: string) => BbsPresetDoc | null | undefined,
  maxDepth = 12,
): ChainResult {
  const chain: BbsPresetDoc[] = []
  const seen = new Set<string>()
  const missing: string[] = []
  let looped = false
  let curName: string | null = name
  while (curName) {
    if (seen.has(curName)) { looped = true; break }
    seen.add(curName)
    const doc = getDoc(curName)
    if (!doc) { missing.push(curName); break }
    chain.unshift(doc)
    if (chain.length >= maxDepth) break
    curName = doc.inherits || null
  }
  return { chain, missing, looped }
}

/* 链合并：后覆盖前。docs 按「链底 → 链顶」的顺序给。 */
export function mergeChain(docs: (BbsPresetDoc | null | undefined)[]): BbsValues {
  const merged: BbsValues = {}
  for (const doc of docs) Object.assign(merged, stripMeta(doc))
  return merged
}

export interface VariantPick {
  index: number
  reason: 'ok' | 'no-variant' | 'no-list' | 'not-listed'
  want?: string
  list?: BbsScalar[]
}

/* 用户预设说了自己用哪个挤出头变体，就按那个变体在系统预设里的位置取值。
 * 对不上（变体名没登记 / 系统预设没这个字段）就用 0，并把原因交给调用方去说。 */
export function variantIndex(
  userDoc: BbsPresetDoc | null | undefined,
  chainTop: BbsPresetDoc | null | undefined,
): VariantPick {
  const want = norm(userDoc?.print_extruder_variant)
  const list = chainTop?.print_extruder_variant
  if (!want) return { index: 0, reason: 'no-variant' }
  if (!Array.isArray(list)) return { index: 0, reason: 'no-list' }
  const i = list.indexOf(want)
  return i < 0
    ? { index: 0, reason: 'not-listed', want: String(want), list }
    : { index: i, reason: 'ok', want: String(want), list }
}

/* 两个值是不是同一个值。
 * '0' 与 '0.0'、'15' 与 15 算同一个；'15%' 与 '15' 不算 —— 百分号是相对值，语义不同。 */
export function sameValue(
  a: BbsValue | undefined,
  b: BbsValue | undefined,
  type?: string,
  idx = 0,
): boolean {
  const va = norm(a, idx); const vb = norm(b, idx)
  if (va === undefined || vb === undefined) return false
  const sa = String(va).trim(); const sb = String(vb).trim()
  if (sa === sb) return true
  if (type === 'bool') return truthy(va) === truthy(vb)
  const pa = sa.endsWith('%'); const pb = sb.endsWith('%')
  if (pa !== pb) return false
  const na = Number(sa.replace('%', '')); const nb = Number(sb.replace('%', ''))
  if (Number.isFinite(na) && Number.isFinite(nb)) return na === nb
  return sa.toLowerCase() === sb.toLowerCase()
}

export interface ResolveInput {
  chainMerged?: BbsValues
  defaults?: BbsValues
  userDoc?: BbsPresetDoc | null
  registry?: Record<string, BbsParamMeta>
  variantIdx?: number
}

export interface ResolveResult {
  baseline: BbsValues
  user: BbsValues
  values: BbsValues
  modifiedKeys: string[]
  unknownKeys: string[]
}

/* 基准（链合并 ⊕ 出厂默认兜底）⊕ 用户预设 → 每个 key 的最终值，外加三份清单：
 *   modifiedKeys：与基准不同的（标橙）
 *   unknownKeys ：registry 里没登记的（新版 BBS 加的参数，如实报出来而不是吞掉）
 *   baseline    ：算完的基准，页面要用它显示「恢复默认值 X」
 * registry 只用来拿 type（布尔按真值比），不传也能跑。
 */
export function resolvePreset({
  chainMerged = {},
  defaults = {},
  userDoc = null,
  registry = {},
  variantIdx = 0,
}: ResolveInput): ResolveResult {
  const baseline: BbsValues = { ...defaults, ...chainMerged }   // 链优先于出厂默认
  const user = stripMeta(userDoc)
  const values: BbsValues = { ...baseline, ...user }
  const modifiedKeys: string[] = []; const unknownKeys: string[] = []
  for (const k of Object.keys(user)) {
    if (!(k in registry)) unknownKeys.push(k)
    if (!sameValue(user[k], baseline[k], registry[k]?.type, variantIdx)) modifiedKeys.push(k)
  }
  modifiedKeys.sort(); unknownKeys.sort()
  return { baseline, user, values, modifiedKeys, unknownKeys }
}
