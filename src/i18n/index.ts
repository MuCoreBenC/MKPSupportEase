/*
 * 前端文案的唯一出口（2026-10-25「全站 i18n」第一刀）。
 *
 * # 三件事
 *
 *   ① 键 → 当前语言的那句话（`t`），带 `{占位符}`；
 *   ② 语言档（`system` / `zh-CN` / `en`）解析成**真语言**（`resolveLocale`）——
 *      "跟随系统"只在**这里**翻译成具体语言，别处不许各自猜 `navigator.language`；
 *   ③ 语言换了要**通知订阅者重渲染**（`useLocale` / `useT` 走 `useSyncExternalStore`）。
 *
 * # 谁写这一格
 *
 * 只有 `app/state/appState.ts`：它读 AppState 的 `locale` 格（后端真账），
 * 落地时调 `setActiveLocale(resolveLocale(pref))`。组件**不自己设语言**。
 *
 * # 为什么首帧不等 IPC
 *
 * 初始值直接按系统语言算（`resolveLocale(null)`）—— 英文系统首帧就是英文，
 * 不会先闪一下中文。IPC 回来后再按用户偏好覆盖（用户明确选了中文就盖回中文）。
 *
 * # 缺键怎么办
 *
 * 开发期以前端中文表兜底并 `console.warn`；**生产不崩**。真正的闸门在构建期：
 * `en.ts` 是 `Record<StringKey, string>`（漏键 TypeScript 就红），
 * `npm run check:i18n` 再查空值 / 占位符 / 中文残留。
 */

import { useCallback, useSyncExternalStore } from 'react'

import type { LocalePref } from '../api/contract'
import { en } from './en'
import { zhCN, type StringKey } from './zh-CN'

/** 真语言（解析完的）。加语言时这里是第一个动的地方。 */
export type Locale = 'zh-CN' | 'en'

export type { StringKey }

/** 文案变量的值（`{count}` 这类占位符） */
export type Vars = Record<string, string | number>

const DICTS: Record<Locale, Record<string, string>> = { 'zh-CN': zhCN, en }

/**
 * 语言档 → 真语言。**"跟随系统"只在这里落地一次**：
 * 系统语言以 `zh` 开头 ⇒ 中文，其余一律英文（今天只发这两种）。
 *
 * `null`（还没选过）与 `'system'` 走同一条路 —— 它们的区别在盘上，不在渲染上。
 */
export function resolveLocale(pref: LocalePref | null): Locale {
  if (pref === 'zh-CN') return 'zh-CN'
  if (pref === 'en') return 'en'
  const tag = typeof navigator === 'undefined' ? '' : navigator.language
  return tag.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'
}

/* ---------- 活的那一格（模块级 store） ---------- */

let active: Locale = resolveLocale(null)
const listeners = new Set<() => void>()

function applyLangAttr(locale: Locale): void {
  if (typeof document !== 'undefined') document.documentElement.lang = locale
}

/* 首帧就把 `<html lang>` 落上（读屏与浏览器的断词/字体都看它） */
applyLangAttr(active)

/**
 * 落地一次语言。**幂等**：同一档重复调用不广播（避免 AppState 每次刷新都全站重渲染）。
 *
 * 读者只有 `appState.ts`；组件请用 `useLocale()` / `useT()`。
 */
export function setActiveLocale(next: Locale): void {
  if (next === active) return
  active = next
  applyLangAttr(next)
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function getActive(): Locale {
  return active
}

/** 现在用的是哪种语言。语言变了会重渲染（`useSyncExternalStore`） */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getActive)
}

/* ---------- 取文案 ---------- */

/** `{name}` 占位符替换。缺变量就原样留着 —— 宁可看见 `{name}` 也不吞掉整句 */
function fill(template: string, vars: Vars | undefined): string {
  if (vars === undefined) return template
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const v = vars[name]
    return v === undefined ? whole : String(v)
  })
}

function translate(locale: Locale, key: StringKey, vars?: Vars): string {
  const raw = DICTS[locale][key] ?? DICTS['zh-CN'][key]
  if (raw === undefined) {
    /* 理论上到不了这里（键类型 + 构建期校验两道）；真到了也不能让界面空着 */
    console.warn(`[i18n] 缺键：${key}`)
    return key
  }
  return fill(raw, vars)
}

/**
 * 取当前语言的那句话 —— **非组件代码**（事件处理、notify 参数、模块级函数）用它。
 *
 * 组件里优先用 `useT()`：它绑定当前语言，语言一变组件就会重渲染；
 * 直接用 `t` 的组件在语言切换后要等别的状态变化才刷新。
 */
export function t(key: StringKey, vars?: Vars): string {
  return translate(active, key, vars)
}

/**
 * 取文案函数的类型（`useT()` 的返回）。
 *
 * **模块级辅助函数一律收它当首参**（`presetTree` / `steps` / `slicer` / `download` 那些
 * 被组件与非组件模块共用的措辞函数）—— 传函数而不是让它们自己去读语言，
 * 才能在语言切换的同一帧换字（`t()` 读的是模块级那一格，组件不会因此重渲染）。
 */
export type Tr = (key: StringKey, vars?: Vars) => string

/** 组件用的 `t`：语言一变，`useLocale()` 触发重渲染，返回的函数也跟着换 */
export function useT(): Tr {
  const locale = useLocale()
  return useCallback((key: StringKey, vars?: Vars) => translate(locale, key, vars), [locale])
}
