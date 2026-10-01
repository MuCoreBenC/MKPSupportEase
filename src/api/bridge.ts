import { invoke } from '@tauri-apps/api/core'

import { NotImplementedError } from './errors'
import { isAppError, type AppError, type MkpApi, type MkpApiMethod } from './contract'

/**
 * IPC 桥：把契约里的方法映射到 Rust 侧的 command。
 *
 * 命名两套、映射在这一处：TS 侧是 `getPreset`（前端习惯），Rust 侧是 `get_preset`
 * （Rust 习惯）。Tauri 会把 JS 传进去的 camelCase 参数名转成 snake_case，
 * 所以参数照常写 `{ variantId }`。
 *
 * 前四个方法（预设 / 偏移 / 校准板 / 打开模型）是 v023 移植时就接通的，走真 command。
 * 预设页（A41）的九个**读**接口也接上了真 command —— 读的是客户端自己的数据根
 * （`appDataDir/presets`），不是仓库，见 `src-tauri/src/ipc/presets.rs`。
 * 还剩三个写盘 / 应用 / 下载的（`getAppliedPreset` / `copyToSlicer` / `downloadFiles`）
 * 后端还没有：按仓里的纪律（HANDOFF 14.1：后端没有的命令**不渲染入口**），
 * 它们在真机上抛 `NotImplementedError`，页面因此显示「本版未接入」那一块，
 * 而不是白屏、也不是假装成功。浏览器里（`npm run dev`）走的是 mock，不经过这一层。
 *
 * 试验场那份桥读的是 `window.__mkp_api`（假设壳会往 window 上注入方法）。那个方案在 Tauri 下
 * 是多一层没必要的间接：`invoke` 本身就是那座桥。
 */

/**
 * 把任何 reject 出来的东西规整成 `AppError`。
 *
 * Rust 侧的 command 返回 `Err(AppError)` 时，前端拿到的就是本结构 —— 直接过。
 * 但还有三类 reject 不长这样，全都得兜住，否则界面上会出现 `[object Object]`：
 * 1. command 没注册 → Tauri 抛一个字符串（`Command xxx not found`）；
 * 2. 参数反序列化失败 → 同上，字符串；
 * 3. panic → 字符串。
 * 兜底一律 `INTERNAL` + `traceId: '-'`（表示"这条错误没经过 Rust 的包装层，日志里查不到"），
 * 原值塞进 `detail` 不丢信息。
 */
function normalizeError(err: unknown, method: MkpApiMethod): AppError {
  if (isAppError(err)) return err

  const detail = typeof err === 'string' ? err : safeStringify(err)
  /* 「命令不存在」是骨架阶段最常见的一种，单独给它一句能看懂的话 */
  const notRegistered = typeof err === 'string' && /not\s*found|not\s*registered/i.test(err)

  return {
    code: notRegistered ? 'NOT_IMPLEMENTED' : 'INTERNAL',
    message: notRegistered ? `这个功能还没接好（${method}）` : '出了点问题，请重试',
    traceId: '-',
    detail,
  }
}

function safeStringify(v: unknown): string {
  try {
    return JSON.stringify(v) ?? String(v)
  } catch {
    return String(v)
  }
}

/** 调一个 command，失败时把错误规整成 AppError 再抛 */
async function call<T>(method: MkpApiMethod, command: string, args?: Record<string, unknown>) {
  try {
    return await invoke<T>(command, args)
  } catch (err) {
    const app = normalizeError(err, method)
    console.error(`[api] ${method} 失败`, app)
    throw app
  }
}

/**
 * 一个**还没接**的接口。
 *
 * 不做成"返回空数组"：空数组与"后端说没有"在界面上长得一样，
 * 而这两件事要分开（见 `errors.ts` 那段）。抛出来，页面上是一块写明方法名的空态。
 *
 * **必须是 async**：契约上这些方法返回 `Promise`，调用方把「失败」接在
 * `.then(ok, err)` / `.catch` 上 —— 直接同步 throw 会绕过那条 reject 通道，
 * 在 `Promise.all([api.getMachines(), ...])` 这种**数组字面量**处就炸穿出去
 * （异常发生在 `Promise.all` 被调用之前），于是调用方的兜底永远收不到它。
 * 落在 `useEffect` 里就是 React 渲染期异常，没有 error boundary 时整棵树卸载 ——
 * 白屏，而不是这块「未接入」空态。async 之后异常才走 reject，兜底才接得住。
 */
async function notWired(method: MkpApiMethod): Promise<never> {
  throw new NotImplementedError(method)
}

export const bridgeApi: MkpApi = {
  getPreset: (variantId) => call('getPreset', 'get_preset', { variantId }),
  saveOffsets: (axes) => call('saveOffsets', 'save_offsets', { axes }),
  getCalibModels: () => call('getCalibModels', 'get_calib_models'),
  openModel: (modelId) => call('openModel', 'open_model', { modelId }),

  /* ——— 预设页（A41）的读接口：走真 command —— */
  getMachines: () => call('getMachines', 'get_machines'),
  getVersionFiles: (machineId, versionId) =>
    call('getVersionFiles', 'get_version_files', { machineId, versionId }),
  getLocalFiles: () => call('getLocalFiles', 'get_local_files'),
  getLocalUserFiles: () => call('getLocalUserFiles', 'get_local_user_files'),
  getSlicerCopied: () => call('getSlicerCopied', 'get_slicer_copied'),
  getPresetFiles: () => call('getPresetFiles', 'get_preset_files'),
  getMenu: () => call('getMenu', 'get_menu'),
  getParamMeta: () => call('getParamMeta', 'get_param_meta'),
  getMachineParams: (machineId, versionId) =>
    call('getMachineParams', 'get_machine_params', { machineId, versionId }),

  /* ——— 新数据世界（第一圈）：运行时 catalog + 下载管道，走真 command ——— */
  getRuntimeCatalog: () => call('getRuntimeCatalog', 'get_runtime_catalog'),
  getDownloadedFiles: () => call('getDownloadedFiles', 'get_downloaded_files'),
  downloadCatalogFile: (fileName) =>
    call('downloadCatalogFile', 'download_runtime_file', { fileName }),
  getActivePreset: () => call('getActivePreset', 'get_active_preset'),
  applyActivePreset: (fileName) =>
    call('applyActivePreset', 'apply_active_preset', { fileName }),
  clearActivePreset: () => call('clearActivePreset', 'clear_active_preset'),

  /* ——— 还要等后端的那几个（写盘 / 应用 / 下载）——— */
  getAppliedPreset: () => notWired('getAppliedPreset'),
  copyToSlicer: () => notWired('copyToSlicer'),
  downloadFiles: () => notWired('downloadFiles'),
}
