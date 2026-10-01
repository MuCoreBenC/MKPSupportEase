/*
 * 选中一份预设 → 解继承链 → 算基准 → 出 values / 改动清单 / 条件显隐。
 *
 * # 这一层为什么独立出来
 *
 * 取数（useBbsData）关心的是「文件在哪、拿到没有」；这一层关心的是
 * 「**这一份**预设摊开来是什么样」。两件事的生命周期不同：换预设只重算这一层，
 * 清单与元数据一动不动。
 *
 * # 三条不能改的口径
 *
 *   1. **选系统预设时不该有任何橙色。** 它自己就是基准 —— 链从它自己起算，
 *      userDoc 只留身份字段（META_ONLY）。这与 BBS 一致（BBS 的 diff 参照是父预设，
 *      选中系统预设时参照就是它本身）。
 *   2. **用户 / 导入的预设从 `inherits` 起算链。** 它只存「相对父预设改了哪几项」。
 *   3. **改值只在内存里。** 这一页没有写盘的口子，刷新即还原 —— 上游也是这样。
 *      `baseDoc` 是刚载入的原样、`userDoc` 是可改的那份，「全部还原」就是把前者克隆回去。
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import { mergeChain, norm, resolvePreset, variantIndex } from './bbsMerge'
import type { VariantPick } from './bbsMerge'
import { illegalSupportStyle, styleFixNote } from './bbsSupportStyle'
import { evalToggles, printerModelOf } from './bbsToggleRules'
import type { BbsData } from './useBbsData'
import type {
  BbsPresetDoc,
  BbsPresetItem,
  BbsScope,
  BbsToggle,
  BbsValue,
  BbsValues,
} from './bbsTypes'

/** 开页先选哪一份：优先那份 A1M 测试（试验场那份样例用户预设，本机用户目录里也有同名一份），
    否则第一份用户预设，再否则第一份系统预设 */
const SAMPLE_NAME = '0.20mm Standard @BBL A1M 测试'

function pickInitial(presets: BbsPresetItem[]): string | null {
  const own = presets.filter((p) => (p.scope === 'imported' || p.scope === 'user') && p.selectable)
  const hit = own.find((p) => p.name === SAMPLE_NAME) || own[0]
  if (hit) return hit.key
  const sys = presets.find((p) => p.scope === 'system' && p.selectable)
  return sys ? sys.key : null
}

/* 系统预设自己就是基准：只留身份字段，参数集合视为空 → 一行橙色都不该有 */
function metaOnly(doc: BbsPresetDoc): BbsPresetDoc {
  return {
    name: doc.name,
    inherits: doc.inherits,
    from: doc.from,
    print_extruder_variant: doc.print_extruder_variant,
    print_extruder_id: doc.print_extruder_id,
  }
}

/** 顺 inherits 往上走，返回「链底 → 链顶」的名字。看清单不看文档，所以不用等 fetch */
function chainNamesOf(
  startName: string | null | undefined,
  byName: Map<string, BbsPresetItem>,
  maxDepth = 12,
): { names: string[]; missing: string[]; looped: boolean } {
  const names: string[] = []
  const seen = new Set<string>()
  const missing: string[] = []
  let cur: string | null | undefined = startName
  let looped = false
  while (cur) {
    if (seen.has(cur)) { looped = true; break }
    seen.add(cur)
    const e = byName.get(cur)
    if (!e) { missing.push(cur); break }
    names.unshift(cur)
    if (names.length >= maxDepth) break
    cur = e.inherits || null
  }
  return { names, missing, looped }
}

export interface BbsPreset {
  currentKey: string | null
  current: BbsPresetItem | null
  scope: BbsScope | null
  /** 状态条第一格那个名字。有改动带 * */
  title: string
  pick: (key: string) => void

  chainNames: string[]
  chainTop: BbsPresetDoc | null
  variant: VariantPick

  baseline: BbsValues
  user: BbsValues
  values: BbsValues
  modifiedKeys: string[]
  unknownKeys: string[]
  toggles: Map<string, BbsToggle>

  editedKeys: Set<string>
  dirty: boolean
  /** 一句话：链缺了 / 有环 / 变体对不上 / support_style 被拉回默认 / 未登记多少项 */
  note: string
  /** 读文件失败这类真问题 */
  error: string

  setValue: (key: string, value: BbsValue) => void
  resetKey: (key: string) => void
  revertAll: () => void
  /** 导入一份：进清单 +（如果能选）选中它 */
  importDoc: (fileName: string, doc: BbsPresetDoc) => void
  importError: string
}

export function useBbsPreset(data: BbsData): BbsPreset {
  const { presets, presetByName, registry, defaults, getDoc, loadDocs, docsVersion } = data

  const [currentKey, setCurrentKey] = useState<string | null>(null)
  /* 可改的那一份 + 原样那一份。null = 还没载入 */
  const [userDoc, setUserDoc] = useState<BbsPresetDoc | null>(null)
  const [baseDoc, setBaseDoc] = useState<BbsPresetDoc | null>(null)
  const [editedKeys, setEditedKeys] = useState<Set<string>>(() => new Set())
  const [error, setError] = useState('')
  const [importError, setImportError] = useState('')
  /** 切预设时丢掉的未还原改动数，要说一声 */
  const [dropped, setDropped] = useState(0)

  const current = useMemo(
    () => presets.find((p) => p.key === currentKey) ?? null,
    [presets, currentKey],
  )

  /*
   * 清单来了、或者当前那一项没了（换来源 / 重扫）→ 落到默认那一份。
   *
   * 清单**空了**（重扫之后没读到本机目录）要落到 null，并且把上一次那份连同它的文档一起放下：
   * 不这么做的话，参数区会一边写着「没读到本机 BBS 预设目录」一边照旧画着上一份的行 ——
   * 自相矛盾，而且那些行来路不正（表现上像「这一份还在」，实际是出厂默认摊出来的）。
   */
  useEffect(() => {
    if (!presets.length) {
      setCurrentKey(null)
      setUserDoc(null)
      setBaseDoc(null)
      setEditedKeys(new Set())
      setError('')
      return
    }
    if (currentKey && presets.some((p) => p.key === currentKey)) return
    setCurrentKey(pickInitial(presets))
  }, [presets, currentKey])

  /* 选中项变了：拉它自己 + 整条链，然后把 userDoc/baseDoc 换成新的 */
  useEffect(() => {
    if (!current) return
    if (!current.selectable) {
      setError(`${current.name}：${current.reason ?? '不可选'}`)
      return
    }
    let alive = true
    void (async () => {
      const start = current.scope === 'system' ? current.name : (current.inherits ?? null)
      const { names } = chainNamesOf(start, presetByName)
      /* 自己那一份也要拉：系统预设的链已经含它，用户预设的不含 */
      await loadDocs(Array.from(new Set([current.name, ...names])))
      if (!alive) return
      const doc = getDoc(current.name)
      if (!doc) { setError(`读「${current.name}」失败`); return }
      setError('')
      setDropped(editedKeys.size)
      setBaseDoc(structuredClone(doc))
      setUserDoc(structuredClone(doc))
      setEditedKeys(new Set())
    })()
    return () => { alive = false }
    /* editedKeys 故意不进依赖：它只是用来报「丢了几处改动」，进来会把这段变成每改一次就重载 */
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current, presetByName, loadDocs, getDoc])

  /* 链上的文档（按名字取缓存）。docsVersion 是「缓存又进货了」的信号 */
  const chain = useMemo(() => {
    if (!current) return { names: [] as string[], missing: [] as string[], looped: false }
    const start = current.scope === 'system' ? current.name : (current.inherits ?? null)
    return chainNamesOf(start, presetByName)
  }, [current, presetByName])

  const resolved = useMemo(() => {
    void docsVersion   // 缓存变了要重算
    const chainDocs = chain.names
      .map((n) => getDoc(n))
      .filter((d): d is BbsPresetDoc => Boolean(d))
    const chainMerged = mergeChain(chainDocs)
    const chainTop = chainDocs[chainDocs.length - 1] ?? null
    /* 变体登记在系统预设的 META 字段里，mergeChain 会把 META 剔掉，所以从链顶原文取 */
    const variant = variantIndex(userDoc, chainTop ?? {})
    const isSystem = current?.scope === 'system'

    const run = (doc: BbsPresetDoc | null) => resolvePreset({
      chainMerged,
      defaults,
      userDoc: doc && isSystem ? metaOnly(doc) : doc,
      registry,
      variantIdx: variant.index,
    })

    let r = run(userDoc)
    /*
     * BBS 会把「不在当前 support_type 候选集里」的 support_style 硬写回 default
     * （ConfigManipulation.cpp:658-668），所以「普通 + 有机树」这种组合在 BBS 里根本不存在。
     * 这里**不改 state**（在 useMemo 里 setState 会打成循环），而是就地拿一份改过的 doc 重算，
     * 并把原值记下来交给状态条说。系统预设是基准、不动它。
     */
    let styleFix = ''
    if (!isSystem && userDoc) {
      const bad = illegalSupportStyle(r.values, variant.index)
      if (bad) {
        styleFix = styleFixNote(bad, r.values, variant.index)
        r = run({ ...userDoc, support_style: 'default' })
      }
    }

    /* 条件显隐要按「当前这份配置的值」算。printer_model 从链顶的 compatible_printers 取 ——
       擦料塔那条规则只对 H2C/H2D/X2D 成立，判不出来就当显示。 */
    const toggles = evalToggles(r.values, {
      printer_model: printerModelOf(chainTop?.compatible_printers ?? null),
    })

    return { ...r, chainTop, variant, toggles, styleFix }
  }, [chain.names, getDoc, docsVersion, userDoc, defaults, registry, current])

  const dirty = resolved.modifiedKeys.length > 0

  const note = useMemo(() => {
    const out: string[] = []
    /* 「读不到本机目录」不在这里说 —— 那时清单是空的、`current` 是 null，整块参数区都是空的，
       页面另有一块空态把理由说清楚（见 PageBbs）。在这儿再抄一遍只会重复。 */
    if (chain.missing.length) {
      out.push(`父预设「${chain.missing[0]}」不在清单里：基准只剩出厂默认，改动判定会偏松`)
    }
    if (chain.looped) out.push('继承链里有环，已截断')
    if (resolved.variant.reason === 'not-listed') {
      out.push(
        `变体「${resolved.variant.want}」没在系统预设里登记`
        + `（登记的是 ${(resolved.variant.list ?? []).join(' / ')}），按第 1 个变体比对`,
      )
    }
    const n = Object.keys(resolved.user).length
    if (current?.scope !== 'system' && n > 0 && n < 5) {
      out.push(`只识别到 ${n} 个工艺参数，确认这是一份工艺预设？`)
    }
    if (resolved.unknownKeys.length) {
      out.push(`有 ${resolved.unknownKeys.length} 项参数表里没登记（新版 BBS 加的？）`)
    }
    const extruders = userDoc?.print_extruder_id
    if (Array.isArray(extruders) && extruders.length > 1) {
      out.push(`这份预设有 ${extruders.length} 个挤出头，当前显示第 ${resolved.variant.index + 1} 个`)
    }
    if (dropped) out.push(`丢弃了上一份预设里 ${dropped} 处未还原的改动`)
    if (resolved.styleFix) out.push(resolved.styleFix)
    return out.join(' · ')
  }, [chain.missing, chain.looped, resolved, current, userDoc, dropped])

  const pick = useCallback((key: string) => {
    setCurrentKey(key)
  }, [])

  /* 写一个参数的值。数组值只写当前变体那个下标，其余元素原样留着 ——
     BBS 的多挤出头/多变体就是这么存的，整个替换会把别的变体抹掉。 */
  const setValue = useCallback((key: string, value: BbsValue) => {
    setUserDoc((prev) => {
      if (!prev) return prev
      const next: BbsPresetDoc = { ...prev }
      const cur = (prev[key] ?? undefined) as BbsValue | undefined
      if (Array.isArray(cur)) {
        const arr = [...cur]
        arr[resolved.variant.index] = value as never
        next[key] = arr
      } else {
        next[key] = value
      }
      /* Tab.cpp:2315-2320「BBS set support style to default when support type changes」：
         切支撑类型时样式无条件回默认，不是「只在非法时才回」。 */
      if (key === 'support_type') next.support_style = 'default'
      return next
    })
    setEditedKeys((prev) => {
      const s = new Set(prev)
      s.add(key)
      if (key === 'support_type') s.add('support_style')
      return s
    })
  }, [resolved.variant.index])

  /* 点 ↺：把这一项从预设里摘掉，值自然落回基准、橙色消失。
     只改内存里的副本，不写文件 —— 这一页没有写盘的口子。 */
  const resetKey = useCallback((key: string) => {
    setUserDoc((prev) => {
      if (!prev) return prev
      const next: BbsPresetDoc = {}
      for (const k of Object.keys(prev)) {
        if (k.toLowerCase() === key) continue
        next[k] = prev[k]
      }
      return next
    })
    setEditedKeys((prev) => new Set(prev).add(key))   // 摘掉一项也算改动，「还原」要能拿回来
  }, [])

  const revertAll = useCallback(() => {
    if (!baseDoc) return
    setUserDoc(structuredClone(baseDoc))
    setEditedKeys(new Set())
  }, [baseDoc])

  /* 外部数据的边界：整份要么收下、要么拒掉，不做「部分收下」那种半吊子状态。 */
  const importDoc = useCallback((fileName: string, doc: BbsPresetDoc) => {
    if (!doc || typeof doc !== 'object' || Array.isArray(doc)) {
      setImportError(`${fileName}：顶层不是一个 JSON 对象，整份拒绝`)
      return
    }
    const probe = resolvePreset({ userDoc: doc, registry })
    if (Object.keys(probe.user).length === 0) {
      setImportError(`${fileName}：没有可识别的工艺参数（可能是机器或耗材预设），整份拒绝`)
      return
    }
    setImportError('')
    const key = data.addImported(fileName, doc)
    setCurrentKey(key)
  }, [registry, data])

  const title = useMemo(() => {
    const name = String(userDoc?.name ?? current?.name ?? '—')
    return `${dirty ? '* ' : ''}${name}`
  }, [userDoc, current, dirty])

  return {
    currentKey,
    current,
    scope: current?.scope ?? null,
    title,
    pick,
    chainNames: chain.names,
    chainTop: resolved.chainTop,
    variant: resolved.variant,
    baseline: resolved.baseline,
    user: resolved.user,
    values: resolved.values,
    modifiedKeys: resolved.modifiedKeys,
    unknownKeys: resolved.unknownKeys,
    toggles: resolved.toggles,
    editedKeys,
    dirty,
    note,
    error,
    setValue,
    resetKey,
    revertAll,
    importDoc,
    importError,
  }
}

/** 变体角标 `[1/2]` 的 title 要用到它 —— 放这里免得渲染层再 import bbsMerge */
export { norm }
