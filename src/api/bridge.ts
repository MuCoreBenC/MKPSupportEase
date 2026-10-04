import { invoke, Channel } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

import { NotImplementedError } from './errors'
import {
  isAppError,
  type AppError,
  type DownloadTick,
  type MkpApi,
  type MkpApiMethod,
} from './contract'

/**
 * IPC 桥：把契约里的方法映射到 Rust 侧的 command。
 *
 * 命名两套、映射在这一处：TS 侧 camelCase（前端习惯），Rust 侧 snake_case（Rust 习惯）。
 * Tauri 会把 JS 传进去的 camelCase 参数名转成 snake_case，所以参数照常写 `{ path }`。
 *
 * 偏移 / 校准板 / 打开模型是 v023 移植时就接通的，走真 command（`get_preset` 那一条
 * 2026-10-02 清扫时删除：首圈的硬编码表，页面早已改走文件体系）。
 * 预设页（A41）的九个**读**接口也接上了真 command —— 读的是客户端自己的数据根
 * （`appDataDir/presets`），不是仓库，见 `src-tauri/src/ipc/presets.rs`。
 * 还剩两个写盘 / 下载的（`copyToSlicer` / `downloadFiles`）后端还没有：
 * 按仓里的纪律（HANDOFF 14.1：后端没有的命令**不渲染入口**），
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
 * 需要看进度时把回调挂成一条 Tauri Channel。
 *
 * **为什么不在调用方那边 new Channel**：一次调用一个 channel、参数名要与 Rust 侧的
 * `on_tick` 对上——这种细节在这层收一次，页面只见回调。
 * **不给回调就完全不挂** —— 与"点了等结果"那条路共用同一个 command，没有第二个版本。
 */
function withTick(
  args: Record<string, unknown>,
  onTick?: (tick: DownloadTick) => void,
): Record<string, unknown> {
  if (onTick === undefined) return args
  const channel = new Channel<DownloadTick>()
  channel.onmessage = onTick
  return { ...args, onTick: channel }
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
  saveOffsets: (axes) => call('saveOffsets', 'save_offsets', { axes }),
  getCalibModels: () => call('getCalibModels', 'get_calib_models'),
  openModel: (modelId) => call('openModel', 'open_model', { modelId }),

  /* ——— 预设页（A41）的读接口：走真 command —— */
  getMachines: () => call('getMachines', 'get_machines'),
  getVersionFiles: (machineId, versionId) =>
    call('getVersionFiles', 'get_version_files', { machineId, versionId }),
  getLocalFiles: () => call('getLocalFiles', 'get_local_files'),
  /* 用户线：用户自己的预设（住 <appDataDir>/user/presets-mine） */
  getUserPresetFiles: () => call('getUserPresetFiles', 'get_user_preset_files'),
  readUserPresetText: (path) => call('readUserPresetText', 'read_user_preset_text', { path }),
  /* 临时编辑那条链：改的是临时文件，原件全程不动（保存才落用户根：官方另存 / 我的写回） */
  beginPresetEdit: (fileName, origin, path) =>
    call('beginPresetEdit', 'begin_preset_edit', { fileName, origin, path }),
  putPresetDraft: (text) => call('putPresetDraft', 'put_preset_draft', { text }),
  patchPresetDraft: (paramKey, value) =>
    call('patchPresetDraft', 'patch_preset_draft', { paramKey, value }),
  discardPresetDraft: () => call('discardPresetDraft', 'discard_preset_draft'),
  commitPresetDraft: () => call('commitPresetDraft', 'commit_preset_draft'),
  /* 第十层：用户文件管理（改名 / 删除）—— 只动名字或删掉，字节一个不动 */
  renameUserPreset: (path, newName) =>
    call('renameUserPreset', 'rename_user_preset', { path, newName }),
  deleteUserPreset: (path) => call('deleteUserPreset', 'delete_user_preset', { path }),
  /* 第十一层：我的文件 → 我的文件（字节复制；不覆盖、不碰任何状态） */
  copyUserPreset: (path, newName) =>
    call('copyUserPreset', 'copy_user_preset', { path, newName }),
  /*
   * 第十二层：通用导入入口。拖拽那一半住在 App 层（`FileImportProvider`），
   * 这里管的是"选择器 + 两段式导入"：
   * · 选择器走 plugin-dialog 的 `open`（权限只开了 `dialog:allow-open`）；
   * 取消 = 空数组（不是错误）。源文件全程只读 —— 复制是 Rust 侧的事。
   */
  pickImportFiles: async () => {
    const picked = await open({
      multiple: true,
      title: '选择要导入的文件',
      filters: [
        { name: '预设文件', extensions: ['toml'] },
        { name: '所有文件', extensions: ['*'] },
      ],
    })
    return picked === null ? [] : Array.isArray(picked) ? picked : [picked]
  },
  stageImport: (sources) => call('stageImport', 'stage_import', { sources }),
  commitImport: (items) => call('commitImport', 'commit_import', { items }),
  /* 第十三层：文件外部管理（打开系统文件管理器并选中这一份；只在「我的文件」上做） */
  revealInFolder: (path) => call('revealInFolder', 'reveal_in_folder', { path }),
  getSlicerCopied: () => call('getSlicerCopied', 'get_slicer_copied'),
  getPresetFiles: () => call('getPresetFiles', 'get_preset_files'),
  getMenu: () => call('getMenu', 'get_menu'),
  getParamMeta: () => call('getParamMeta', 'get_param_meta'),
  getMachineParams: (machineId, versionId) =>
    call('getMachineParams', 'get_machine_params', { machineId, versionId }),

  /* ——— 新数据世界（第一圈）：运行时 catalog + 下载管道，走真 command ——— */
  getRuntimeCatalog: () => call('getRuntimeCatalog', 'get_runtime_catalog'),
  getDownloadedFiles: () => call('getDownloadedFiles', 'get_downloaded_files'),
  getStaleFiles: () => call('getStaleFiles', 'get_stale_files'),
  /* 第 6 层：盘上这几份认得出是哪一版吗（旧版本 / 查不出它是哪一版）—— 只列有事的 */
  getDeliveryTrust: () => call('getDeliveryTrust', 'get_delivery_trust'),
  /* 归档区（官方旧版本留档）：只列 + 读正文。删除 / 恢复**没有命令** —— 这一层不做 */
  getArchivedFiles: () => call('getArchivedFiles', 'get_archived_files'),
  readArchivedText: (path) => call('readArchivedText', 'read_archived_text', { path }),
  downloadCatalogFile: (fileName, onTick) =>
    call<void>('downloadCatalogFile', 'download_runtime_file', withTick({ fileName }, onTick)),
  downloadCatalogFiles: (fileNames, onTick) =>
    call('downloadCatalogFiles', 'download_runtime_files', withTick({ fileNames }, onTick)),
  readDownloadedText: (fileName) =>
    call('readDownloadedText', 'read_downloaded_text', { fileName }),
  getPresetSource: () => call('getPresetSource', 'get_preset_source'),
  setPresetSource: (baseUrl) =>
    call('setPresetSource', 'set_preset_source', { baseUrl }),
  clearPresetSource: () => call('clearPresetSource', 'clear_preset_source'),
  getActivePreset: () => call('getActivePreset', 'get_active_preset'),
  /* 两条线一个入口：`origin` 说这一份住哪条线，用户线还要给出它在用户根里的路径 */
  applyActivePreset: (fileName, origin, path) =>
    call('applyActivePreset', 'apply_active_preset', { fileName, origin, path }),
  clearActivePreset: () => call('clearActivePreset', 'clear_active_preset'),
  checkRemoteUpdate: () => call('checkRemoteUpdate', 'check_remote_update'),
  applyRemoteUpdate: () => call('applyRemoteUpdate', 'apply_remote_update'),
  /* ——— 软件更新（release.json）—— 与预设数据两条链 ——— */
  getAppVersion: () => call('getAppVersion', 'get_app_version'),
  checkSoftwareUpdate: () => call('checkSoftwareUpdate', 'check_software_update'),

  /* ——— 还要等后端的那几个（写盘 / 应用 / 下载）——— */
  copyToSlicer: () => notWired('copyToSlicer'),
  downloadFiles: () => notWired('downloadFiles'),
}
