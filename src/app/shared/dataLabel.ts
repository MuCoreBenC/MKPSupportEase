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
 * 单位符号的英册。**只收中文那三个**（`层` / `次` / `秒`）—— `mm` / `mm/s` / `°` / `%`
 * 两种语言写法一样，走 `?? unit` 原样返回。
 *
 * 为什么不做进注册表：单位是**符号**，不是这条参数的名字/说明；注册表已经用
 * `unit` 存了中文那一册，再长一个 `unitEn` 就是同一件事的第二份手抄。
 */
const UNIT_EN: Record<string, string> = { 层: 'layers', 次: 'times', 秒: 's' }

/** 单位按界面语言挑一册（`en` 只换上面那三个中文单位） */
export function unitText(unit: string | undefined, locale: Locale): string | undefined {
  if (unit === undefined || locale !== 'en') return unit
  return UNIT_EN[unit] ?? unit
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
    desc: string
    descEn?: string
    unit?: string
    field: FieldSchema
  },
>(def: T, locale: Locale): T {
  if (locale !== 'en') return def
  const label = bi(def.label, def.labelEn, locale)
  const desc = bi(def.desc, def.descEn, locale)
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
      choices: def.field.choices?.map((c) => ({ ...c, label: bi(c.label, c.labelEn, locale) })),
    },
  }
}

/**
 * 页签/分组按界面语言摊开。分组比页签多两个格子（`desc` / `descEn`），
 * 用泛型保住调用方自己的类型（参数页的 `ParamSection` 带 `count`，目录那边的没有）。
 */
export function localizeTab<
  S extends { label: string; labelEn?: string; desc?: string; descEn?: string },
  T extends { label: string; labelEn?: string; sections: S[] },
>(tab: T, locale: Locale): T {
  if (locale !== 'en') return tab
  return {
    ...tab,
    label: bi(tab.label, tab.labelEn, locale),
    sections: tab.sections.map((s) => ({
      ...s,
      label: bi(s.label, s.labelEn, locale),
      desc: s.desc === undefined ? undefined : bi(s.desc, s.descEn, locale),
    })),
  }
}

/**
 * 机型目录里的**版本显示名/说明**按界面语言摊开（`tag` 刻意不动：它还要给
 * `TAG_TONE` 当键用，显示时再单挑）。
 *
 * 只动 `name` / `description` 两格 —— 版本 id、套餐指向、图位全不动。
 */
export function localizeMachines<
  V extends {
    name: string
    nameEn?: string | null
    description?: string | null
    descriptionEn?: string | null
  },
  M extends { versions: V[] },
>(machines: M[], locale: Locale): M[] {
  if (locale !== 'en') return machines
  return machines.map((m) => ({
    ...m,
    versions: m.versions.map((v) => ({
      ...v,
      name: bi(v.name, v.nameEn ?? undefined, locale),
      description:
        v.description == null ? v.description : bi(v.description, v.descriptionEn ?? undefined, locale),
    })),
  }))
}

/**
 * BBS 注册表里的一条元信息 → 一个显示名。
 *
 * 那份数据的形状是 `label: { zh, en }`（拓竹原版的写法），**两册都在**；
 * 与 `labelEn` 那套只差一个位置，挑选规矩一模一样。
 */
export function localizeParamMeta(
  meta: { label?: { zh?: string; en?: string } | null } | null | undefined,
  locale: Locale,
  fallback = '',
): string {
  const zh = meta?.label?.zh ?? ''
  const en = meta?.label?.en ?? ''
  if (locale === 'en') return en !== '' ? en : zh !== '' ? zh : fallback
  return zh !== '' ? zh : en !== '' ? en : fallback
}
