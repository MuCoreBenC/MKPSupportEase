/*
 * AppState 客户端（`docs/APP-STATE.md` §7）：应用持久化状态在前端的**唯一读取/订阅口**。
 *
 * 后端把使用中指针 / 草稿 / 数据源统一存进 `run/app-state.json`（`runtime/app_state.rs`），
 * 任何写命令成功都会 emit `app-state-changed`。这里只做三件事：
 *
 *   ① 全应用只有这里调 `api.getActivePreset()`（唯一读取入口）；
 *   ② 接住后端的变更事件重读整份状态（"改了就推"，页面不点任何东西也跟着变）；
 *   ③ 用 `useSyncExternalStore` 把快照发给订阅的页面 —— 页面**不持副本、不回页签对账**。
 *
 * 写这一侧没有"入口"可言：状态变更都是一次业务动作（应用 / 撤销 / 改名 / 删除…），
 * 各自的命令成功后调 `appStateMutated()` 兜底刷新（浏览器 mock 没有事件，靠它；
 * 桌面端事件与它都只会触发一次重读，`inflight` 去重）。换来的 invariant：
 * 任何一条改状态的路径走完，所有订阅页面看到的是同一份快照。
 *
 * 本轮接入的格：**activePreset**（唯一被多个页面共享的一格）与 **theme**
 * （2026-10-12 深色模式那一刀加的 —— 外壳级偏好，与"哪些页面读它"无关：
 * 设置页读它显示当前档，`<html data-theme>` 由这里落地）。draft / presetSource
 * 等被多页共享时按同一模式扩到这里，不为单页的状态另开读取口。
 */

import { useSyncExternalStore } from 'react'

import { api, listen } from '../../api'
import type { ActiveOrigin, ActivePreset, PinPref, ThemePref } from '../../api/contract'
import { inTauri } from '../window'
/*
 * 主题那一句（档位 + 系统深浅 ⇒ 是不是深色 ⇒ 落 `<html data-theme>`）住在
 * `./theme`：**引导窗口也要用同一句**，而它跑不起这一整个模块（见那个文件头）。
 */
import { applyTheme, prefersDark } from './theme'

/** 与 Rust 侧 `ipc::APP_STATE_EVENT` 同名（两边没有编译器，靠这里对齐） */
const APP_STATE_EVENT = 'app-state-changed'

/**
 * 快照：**这一份就是"应用现在的状态"在界面这一侧的影子** —— 两格一起读、
 * 一起广播（同一个事件），页面各订阅各的那一格。
 */
interface AppStateSnapshot {
  /** 使用中指针（null = 还没用任何一份） */
  active: ActivePreset | null
  /** 主题档位（null = 还没选过 ⇒ 按"跟随系统"） */
  theme: ThemePref | null
  /** 置顶档位（2026-10-17；`off` = 没选过，与后端同一个出厂默认） */
  pin: PinPref
  /** 主窗在不在窄窗态（同上） */
  compact: boolean
}

let snapshot: AppStateSnapshot = { active: null, theme: null, pin: 'off', compact: false }
let loaded = false
let inflight: Promise<void> | null = null
const listeners = new Set<() => void>()

function notify(): void {
  for (const listener of listeners) listener()
}

/**
 * 主题落地：先落 `<html data-theme>`，再把界面底报给原生当**原生背景色**（2026-10-22）。
 *
 * 后一步治的是 macOS 拖拽改窗口大小时右侧/底部露出的那条白边 ——
 * 那是 WebKit 的已知呈现 bug（`WebKit#323132` / `tauri#14288`），修不了，
 * 只能把露出来的那层染成同色（完整来由见 `src-tauri/src/ipc/prefs.rs` 的
 * `set_window_backdrop`）。**露的是 WKWebView 自己的白**，所以必须走后端的
 * `set_background_color`，光设窗口底色不够。
 *
 * ★ 颜色**不在这儿写死**：读的就是 `--bg-2`。`applyTheme` 刚改完 `data-theme`，
 *   同一次里 `getComputedStyle` 会强制重算，拿到的是新档位的值。
 * ★ 失败吞掉 —— 它是外观的一层兜底，不该让换主题/首帧读取报错（浏览器 mock 里也一样）。
 */
function landTheme(pref: ThemePref | null): void {
  applyTheme(pref)
  const color = getComputedStyle(document.documentElement).getPropertyValue('--bg-2').trim()
  if (color !== '') void api.setWindowBackdrop(color).catch(() => {})
}

/* 跟随系统那一档：系统深浅变了要跟着翻 —— 用户点了浅 / 深则不理会（真值在他那一档里） */
prefersDark.addEventListener('change', () => {
  if (snapshot.theme === null || snapshot.theme === 'system') landTheme(snapshot.theme)
})

/** 重读 AppState（唯一的那次 IPC 读），落地后广播给订阅者 */
export function appStateMutated(): void {
  if (inflight !== null) return
  inflight = Promise.all([
    /*
     * 两格各读各的：一格失败**不把另一格打成"没有"**（读不到主题不该让
     * 「使用中」也显示空）。失败就照上面那一句的旧口径 —— 那一格当"没有"。
     */
    api.getActivePreset().catch(() => null),
    api.getTheme().catch(() => null),
    /*
     * 窗口那两格（2026-10-17）。读不到就按**后端也认同的出厂默认**兜底
     * （不置顶 / 不在窄窗）—— 与主题那条"失败当没有"同一个口径。
     */
    api.getWindowPrefs().catch(() => null),
  ])
    .then(([active, theme, win]) => {
      snapshot = {
        active,
        theme,
        pin: win?.pin ?? 'off',
        compact: win?.compact ?? false,
      }
      landTheme(theme)
    })
    .finally(() => {
      loaded = true
      inflight = null
      notify()
    })
}

let eventBound = false

/** 订阅快照。首个订阅者出现时补一次初始读取，并把后端事件接上（一次性，应用生命周期） */
function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  if (!loaded && inflight === null) appStateMutated()
  if (inTauri && !eventBound) {
    eventBound = true
    void listen(APP_STATE_EVENT, () => appStateMutated())
  }
  return () => {
    listeners.delete(listener)
  }
}

function getSnapshot(): ActivePreset | null {
  return snapshot.active
}

/**
 * 正在使用的那一份（AppState 的 activePreset 格）。null = 还没用任何一份。
 *
 * 页面只读这个 hook —— 不保存副本、不在"回到本页"时重新读、不对账：
 * 底账变了（应用 / 撤销 / 改名 / 删除，包括删除使用中那份的连带撤指针）事件会推过来，
 * 所有订阅页面同帧换账。
 */
export function useActivePreset(): ActivePreset | null {
  return useSyncExternalStore(subscribe, getSnapshot)
}

/**
 * 主题档位（AppState 的 theme 格）。`null` = **还没选过** ⇒ 按"跟随系统"。
 *
 * 读者是设置页（「外观」那一块显示当前档）—— `<html data-theme>` 不用页面操心：
 * 每次快照落地由 `applyTheme` 写一次（含"跟随系统"时系统深浅的变化）。
 */
export function useThemePref(): ThemePref | null {
  return useSyncExternalStore(subscribe, getThemePref)
}

function getThemePref(): ThemePref | null {
  return snapshot.theme
}

/**
 * 置顶档位（AppState 的 `pin` 格，2026-10-17）。
 *
 * 读者两处：工艺页那枚胶囊（显示 / 切档）与外壳 `App.tsx`（按"档位 + 当前页签"
 * 算该不该真置顶 —— 那条 effect 是**唯一**落地的地方）。
 */
export function usePinPref(): PinPref {
  return useSyncExternalStore(subscribe, getPinPref)
}

function getPinPref(): PinPref {
  return snapshot.pin
}

/**
 * 主窗在不在窄窗态（AppState 的 `compact` 格）。
 *
 * 只有工艺页那枚胶囊读它 —— 窗口几何的真实状态在 Rust 侧，
 * 前端不量 `window.innerWidth` 去猜（判据不许两个源）。
 */
export function useCompactWindow(): boolean {
  return useSyncExternalStore(subscribe, getCompactWindow)
}

function getCompactWindow(): boolean {
  return snapshot.compact
}

/* 系统深浅那一档的订阅（`prefers-color-scheme`）。只有"要拿真值去画"的读者需要它 ——
   CSS 那侧不用：`<html data-theme>` 已经在 applyTheme 里落过地了 */
function subscribeSystemDark(listener: () => void): () => void {
  prefersDark.addEventListener('change', listener)
  return () => prefersDark.removeEventListener('change', listener)
}

function getSystemDark(): boolean {
  return prefersDark.matches
}

/**
 * 现在到底是不是深色 —— **把 `theme` 那一档与系统深浅解析完的真值**。
 *
 * 给"不能只靠 CSS 变量、必须拿真值去画"的地方用：校准板那两块 SVG 就是
 * （配色在生成器里写死成两套 `THEMES['浅色' | '深色']`，读不到 CSS 变量）。
 * 普通界面不用它 —— 那些走 `<html data-theme>` 换一套变量就够。
 */
export function useIsDark(): boolean {
  const pref = useThemePref()
  const systemDark = useSyncExternalStore(subscribeSystemDark, getSystemDark)
  if (pref === 'dark') return true
  if (pref === 'light') return false
  return systemDark
}

/**
 * 首读落地了吗。挂载初期快照还在路上（null 只代表"还没读到"，
 * 不代表"没有已应用"）——要区分这两种 null 的页面用它把关。
 */
export function useActivePresetReady(): boolean {
  return useSyncExternalStore(subscribe, getReady)
}

function getReady(): boolean {
  return loaded
}

/** 一次性拿当前快照（首读落地即返回）。挂载时"默认落在正在使用那一份"的页面用 */
export function activePresetSnapshot(): Promise<ActivePreset | null> {
  if (loaded) return Promise.resolve(snapshot.active)
  if (inflight === null) appStateMutated()
  return (inflight ?? Promise.resolve()).then(() => snapshot.active)
}

/**
 * **把「本机真有的某一份」变成正在使用的那一份**（2026-10-14）。
 *
 * 与 [`activateCombo`] 的区别是**判据的粒度**：
 *
 *   activateCombo    按 **combo**（机型 + 版本）告诉底账"我用这一版"，
 *                    文件名由它自己从目录里查（`fileOf`）—— 一个 combo 只能对应一份；
 *   本函数            按**具体那一份文件**（`fileName` + 可选 `path`）——
 *                    同一个 combo 下有几份时（下载来的一份 + 你复制出来的一份），
 *                    只有这条路说得清"我要的是**这一份**"。
 *
 * 为什么必须单开一条：预设抽屉现在列的是**本机文件**，同一个 combo 下可能有好几行。
 * 用 `activateCombo` 那条路，`pickPreset` 的 `uid === currentUid` 与
 * `activateCombo` 的"combo 相同就不动"**两层都会直接退货** ⇒ 用户点另一份没反应
 * （作者 2026-10-14 实测：「copy出来的点不了，再用无论怎么切换都是这一个」）。
 *
 * 守则与 `activateCombo` 一致：**应用不了就保持原账**（页面选择仍成立，不弹错误打断）。
 * 但**不保留**"combo 相同就不动"那一条 —— 这条路的全部意义就是"同 combo 也要换一份"。
 */
export function activateFile(fileName: string, path: string | null): void {
  if (fileName === '') return
  /*
   * 浏览器预览（mock）里没有交付字节 —— 与 `activateCombo` 同一条守则：如实拒、
   * 选择仍成立、底账保持原样（那是给显式按钮的提示通道，浏览行为不触发它）。
   */
  if (!inTauri) return
  const origin: ActiveOrigin = path === null ? 'preset' : 'local'
  void api
    .applyActivePreset(fileName, origin, path ?? undefined)
    .then(() => appStateMutated())
    .catch(() => {
      console.debug(`[appState] ${fileName} 这一步应用不了 —— 底账保持原样`)
    })
}

/**
 * 切换到某台机型的某个版本 = 把它变成「正在使用」（作者 2026-10-06 真机反馈：
 * 不能只有预设页点「应用」才算切换 —— 首页三级选齐一台、抽屉里点一份、
 * 参数页换组合，都要真切换，四个页面同账）。
 *
 * 两条守则：
 * - **底账已经指着这个 combo 时不动** —— 正在用的可能是「本地文件」，
 *   随手浏览回来不该把它顶成云端底稿；
 * - **应用不了（还没下载 / 内容与目录漂了）就保持原账**：页面选择仍成立，
 *   「未下载 / 内容异常」的三态如实显示 —— 不弹错误打断浏览，也不编一句"切换成功"。
 *
 * ★ 2026-10-14：抽屉那一路**不再走这里**（同一个 combo 下有几份时它表达不了"哪一份"），
 *   改用 [`activateFile`]；这条仍服务"首页三级选齐 / 参数页换组合"那种**按版本浏览**的动作。
 */
export function activateCombo(
  model: string | null,
  variant: string | null,
  fileOf: (machineId: string, versionId: string) => string | null,
  active: ActivePreset | null,
): void {
  if (model === null || variant === null || model === '' || variant === '') return
  if (active !== null && active.machineId === model && active.versionId === variant) return
  /*
   * 浏览器预览（mock）里没有交付字节 —— 云端线的应用在那里**如实拒**
   * （NotImplementedError 的技术形式进控制台，那是给显式按钮的提示通道）。
   * 浏览行为不去触发它：选择仍然成立、底账保持原样，与"应用不了"同一条守则。
   */
  if (!inTauri) return
  const fileName = fileOf(model, variant)
  if (fileName === null) return
  void api
    .applyActivePreset(fileName, 'preset')
    .then(() => appStateMutated())
    .catch(() => {
      console.debug(`[appState] ${fileName} 这一步应用不了（多半还没下载）—— 底账保持原样`)
    })
}
