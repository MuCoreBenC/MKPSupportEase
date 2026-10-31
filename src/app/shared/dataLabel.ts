/*
 * 双语数据挑一册（2026-10-26）。
 *
 * 这里的对象**不是句子**，是数据：注册表里的参数名/说明/分组名（`labelEn` / `descEn`，
 * 住 `presets/registry/param_registry.toml`）、BBS 注册表里的 `label.zh|en`。
 * 句子住 `src/i18n`，两件事别混 —— 句子按语言"翻"，数据只是"挑"。
 *
 * 为什么要有这一个文件：同一份数据有**三条读取路径**（参数页 `useParams`、
 * 对比台 `CompareModal`、工艺页 `BbsRow`），各自抄一份挑选逻辑必然走岔
 * —— 2026-10-26 那一轮就是对比台漏了，英文界面上整片中文。
 *
 * 挑选规矩**唯一**：`en` 界面优先英文、空着回落中文；`zh-CN` 界面一律中文。
 * 回落不是妥协，是防线：数据可能还没译完，缺一条不能变成"这行没有名字"。
 */
import type { FieldSchema } from '../../components/field'
import type { Locale } from '../../i18n'

/** 双册择一。`en` 为空串/undefined 时回落中文 */
export function bi(zh: string, en: string | undefined, locale: Locale): string {
  return locale === 'en' && en !== undefined && en !== '' ? en : zh
}

/**
 * 单位符号的四册。**只收中文那三个**（`层` / `次` / `秒`）—— `mm` / `mm/s` / `°` / `%`
 * 各语言写法一样，走 `?? unit` 原样返回。
 *
 * 为什么不做进注册表：单位是**符号**，不是这条参数的名字/说明；注册表已经用
 * `unit` 存了中文那一册，再长三个 `unitXxx` 就是同一件事的第二份手抄。
 */
const UNIT_TEXT: Record<string, Partial<Record<Locale, string>>> = {
  层: { en: 'layers', 'zh-TW': '層', ja: 'レイヤー' },
  次: { en: 'times', 'zh-TW': '次', ja: '回' },
  秒: { en: 's', 'zh-TW': '秒', ja: '秒' },
}

/** 单位按界面语言挑一册（只换上面那三个中文单位） */
export function unitText(unit: string | undefined, locale: Locale): string | undefined {
  if (unit === undefined || locale === 'zh-CN') return unit
  return UNIT_TEXT[unit]?.[locale] ?? unit
}

/**
 * 一条定义按界面语言摊开（参数页与对比台共用）。
 *
 * 只摊**显示名**：`label` / `desc` / 控件里的选项名 / 单位符号。键、范围、默认值一个字不动。
 */
export function localizeParamDef<
  T extends {
    label: string
    labelEn?: string
    labelJa?: string
    labelTw?: string
    desc: string
    descEn?: string
    descJa?: string
    descTw?: string
    unit?: string
    field: FieldSchema
  },
>(def: T, locale: Locale): T {
  if (locale === 'zh-CN') return def
  const label = multiLang(def.label, { En: def.labelEn, Tw: def.labelTw, Ja: def.labelJa }, locale)
  const desc = multiLang(def.desc, { En: def.descEn, Tw: def.descTw, Ja: def.descJa }, locale)
  return {
    ...def,
    label,
    desc,
    unit: unitText(def.unit, locale),
    field: {
      ...def.field,
      label,
      desc,
      unit: unitText(def.field.unit, locale),
      choices: def.field.choices?.map((c) => ({
        ...c,
        label: multiLang(c.label, { En: c.labelEn, Tw: c.labelTw, Ja: c.labelJa }, locale),
      })),
    },
  }
}

/**
 * 页签/分组按界面语言摊开。分组比页签多两个格子（`desc` / `descEn`），
 * 用泛型保住调用方自己的类型（参数页的 `ParamSection` 带 `count`，目录那边的没有）。
 */
export function localizeTab<
  S extends {
    label: string
    labelEn?: string
    labelJa?: string
    labelTw?: string
    desc?: string
    descEn?: string
    descJa?: string
    descTw?: string
  },
  T extends { label: string; labelEn?: string; labelJa?: string; labelTw?: string; sections: S[] },
>(tab: T, locale: Locale): T {
  if (locale === 'zh-CN') return tab
  return {
    ...tab,
    label: multiLang(tab.label, { En: tab.labelEn, Tw: tab.labelTw, Ja: tab.labelJa }, locale),
    sections: tab.sections.map((s) => ({
      ...s,
      label: multiLang(s.label, { En: s.labelEn, Tw: s.labelTw, Ja: s.labelJa }, locale),
      desc:
        s.desc === undefined
          ? undefined
          : multiLang(s.desc, { En: s.descEn, Tw: s.descTw, Ja: s.descJa }, locale),
    })),
  }
}

/**
 * 数据的多语言挑选规矩（2026-10-31 起不止两册）：中文打底，`en` / `zh-TW` / `ja`
 * 界面优先本语种那一册，空着逐级回落中文。与 `bi` 同一条规矩的推广 —— 回落不是妥协，
 * 是防线：数据可能还没译完，缺一册不能变成"这行没有名字"。
 */
export function multiLang(
  zh: string,
  t: { En?: string | null; Tw?: string | null; Ja?: string | null },
  locale: Locale,
): string {
  if (locale === 'en' && t.En) return t.En
  if (locale === 'zh-TW' && t.Tw) return t.Tw
  if (locale === 'ja' && t.Ja) return t.Ja
  return zh
}

/**
 * 机型目录里的**版本显示名/说明/角标**按界面语言摊开（`tag` 摊开的是显示值；
 * `TAG_TONE` 当键用的地方在**摊开前**先取原值）。
 *
 * 只动 `name` / `description` / `tag` 三格 —— 版本 id、套餐指向、图位全不动。
 */
export function localizeMachines<
  V extends {
    name: string
    nameEn?: string | null
    nameTw?: string | null
    nameJa?: string | null
    description?: string | null
    descriptionEn?: string | null
    descriptionTw?: string | null
    descriptionJa?: string | null
    tag?: string | null
    tagEn?: string | null
    tagTw?: string | null
    tagJa?: string | null
  },
  M extends { versions: V[] },
>(machines: M[], locale: Locale): M[] {
  if (locale !== 'en' && locale !== 'zh-TW' && locale !== 'ja') return machines
  return machines.map((m) => ({
    ...m,
    versions: m.versions.map((v) => ({
      ...v,
      name: multiLang(v.name, { En: v.nameEn, Tw: v.nameTw, Ja: v.nameJa }, locale),
      description:
        v.description == null
          ? v.description
          : multiLang(v.description, { En: v.descriptionEn, Tw: v.descriptionTw, Ja: v.descriptionJa }, locale),
      tag: v.tag == null ? v.tag : multiLang(v.tag, { En: v.tagEn, Tw: v.tagTw, Ja: v.tagJa }, locale),
    })),
  }))
}

/**
 * BBS 注册表里的一条元信息 → 一个显示名。
 *
 * 那份数据的形状是 `label: { zh, en }`（拓竹原版的写法），**两册都在**；
 * 与 `labelEn` 那套只差一个位置，挑选规矩一模一样。
 *
 * ★ 2026-10-31 起是**四册**：`ja` / `zh_tw` 由 BBS 官方语言包
 * （`resources/i18n/{ja,zh_TW}/BambuStudio.mo`）按英文原文补进 `public/bbs/*.json`。
 * 作者口径：「这些就是 BBS 开源仓库里的」—— 术语要跟切片器一致（層高 / 積層ピッチ），
 * 不是我们另翻一遍。缺一册回落中文，同 `bi` 的防线。
 */
export function localizeParamMeta(
  meta: { label?: { zh?: string; en?: string; ja?: string; zh_tw?: string } | null } | null | undefined,
  locale: Locale,
  fallback = '',
): string {
  const zh = meta?.label?.zh ?? ''
  const en = meta?.label?.en ?? ''
  const ja = meta?.label?.ja ?? ''
  const tw = meta?.label?.zh_tw ?? ''
  if (locale === 'ja') return ja !== '' ? ja : zh !== '' ? zh : en !== '' ? en : fallback
  if (locale === 'zh-TW') return tw !== '' ? tw : zh !== '' ? zh : en !== '' ? en : fallback
  if (locale === 'en') return en !== '' ? en : zh !== '' ? zh : fallback
  return zh !== '' ? zh : en !== '' ? en : fallback
}

/**
 * BBS 版面那一份（页签 / 分组）的名字，四册同样挑法。
 *
 * 形状是 `{ name, name_en, name_ja, name_zh_tw }` —— 与 `label` 那套只差字段位置，
 * 唯一实现放这里，免得 `bbsPanel` 再抄一份挑选。缺册回落中文。
 */
export function bbsLayoutName(
  item: { name?: string; name_en?: string; name_ja?: string; name_zh_tw?: string },
  locale: Locale,
  fallback = '',
): string {
  const zh = item.name ?? ''
  const en = item.name_en ?? ''
  const ja = item.name_ja ?? ''
  const tw = item.name_zh_tw ?? ''
  if (locale === 'ja') return ja !== '' ? ja : zh !== '' ? zh : en !== '' ? en : fallback
  if (locale === 'zh-TW') return tw !== '' ? tw : zh !== '' ? zh : en !== '' ? en : fallback
  if (locale === 'en') return en !== '' ? en : zh !== '' ? zh : fallback
  return zh !== '' ? zh : en !== '' ? en : fallback
}
