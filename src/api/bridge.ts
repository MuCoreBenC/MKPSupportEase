import { invoke, Channel } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

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
 * 2026-10-09 起 `copyToSlicer`（复制进切片器目录）与 `getFileUrl`（复制官方链接）
 * 也接上了真 command；`downloadFiles`（官方仓库文件的「下载」残支）按用户裁断
 * **连入口一起撤了**，契约里不再留它。浏览器里（`npm run dev`）走的是 mock，不经过这一层。
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
 *
 * ★★ **不给回调也照样挂一条**（2026-10-06 修死路，别再改回去）：
 * Rust 侧 `on_tick: Channel<DownloadTick>` 是**必填参数** —— Tauri 的 `Channel` 只有
 * `CommandArg`、**没有 `Deserialize`**（它要 `Webview` 才能建），所以
 * `Option<Channel<T>>` 根本编译不出来：契约里的"`onTick` 可选"只能**在这一层**兑现。
 * 不挂的后果不是"少个回调"，而是 Tauri 在**参数反序列化**那一步就拒
 * （`command download_runtime_file missing required key onTick`）——**命令体一行都不跑**
 * （Rust 日志里连一条都没有），而界面只能拿到下面 `normalizeError` 兜底的
 * 「出了点问题，请重试」。首页那颗「下载并应用」就是这么死的（有一档 `ShaMismatch`
 * 的真实原因被它盖住了）。判据：`scripts/check-channel-args.mjs`。
 *
 * 挂着不给回调是安全的：JS 侧 `Channel` 的 `onmessage` 缺省就是空函数（收到即丢），
 * Rust 侧 `send_tick` 对"没人听"也只记一行 debug —— 与"点了等结果"共用同一个 command，
 * 没有第二个版本。
 */
function withTick(
  args: Record<string, unknown>,
  onTick?: (tick: DownloadTick) => void,
): Record<string, unknown> {
  const channel = new Channel<DownloadTick>()
  if (onTick !== undefined) channel.onmessage = onTick
  return { ...args, onTick: channel }
}

export const bridgeApi: MkpApi = {
  /* 校准值写进「我的预设」（2026-10-08：随用户那份走，不再是独立的 offsets.json） */
  savePresetCalibration: (path, axes) =>
    call('savePresetCalibration', 'save_preset_calibration', { path, axes }),
  getCalibModels: () => call('getCalibModels', 'get_calib_models'),
  openModel: (modelId) => call('openModel', 'open_model', { modelId }),
  /* 「复制后处理脚本」里那段可执行物路径：壳的 current_exe()（就是本程序自己） */
  getPostProcessExe: () => call('getPostProcessExe', 'get_post_process_exe'),
  /* 钩子那一趟：切片器导出时本程序被带参数拉起来的那一次（快照 / 停 / 答一问） */
  getPostProcessRun: () => call('getPostProcessRun', 'get_post_process_run'),
  cancelPostProcess: () => call('cancelPostProcess', 'cancel_post_process'),
  answerPostProcessMismatch: (keep) =>
    call('answerPostProcessMismatch', 'answer_post_process_mismatch', { keep }),

  /* ——— 预设页（A41）的读接口：走真 command —— */
  getMachines: () => call('getMachines', 'get_machines'),
  getVersionFiles: (machineId, versionId) =>
    call('getVersionFiles', 'get_version_files', { machineId, versionId }),
  getLocalFiles: () => call('getLocalFiles', 'get_local_files'),
  /* 用户线：用户自己的预设（住 <appDataDir>/user/presets-mine） */
  getUserPresetFiles: () => call('getUserPresetFiles', 'get_user_preset_files'),
  /* 备注覆盖账（副标题）+ 改归属（文件头 # machine/# variant 两行） */
  getPresetRemarks: () => call('getPresetRemarks', 'get_preset_remarks'),
  setPresetRemark: (key, remark) =>
    call('setPresetRemark', 'set_preset_remark', { key, remark }),
  setUserPresetMachineVersion: (path, machineId, versionId) =>
    call('setUserPresetMachineVersion', 'set_user_preset_machine_version', {
      path,
      machineId,
      versionId,
    }),
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
  /* 另存为我的预设：官方那一份 → 我的一份（撞名就拒、带血统）+ 按机型/版本找它（校准页） */

  getUserCopyFor: (machineId, versionId) =>
    call('getUserCopyFor', 'get_user_copy_for', { machineId, versionId }),
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
  /* 归档区（官方旧版本留档）：列 + 读正文 + 删（2026-10-06 起允许删，代价在确认框讲清） */
  getArchivedFiles: () => call('getArchivedFiles', 'get_archived_files'),
  readArchivedText: (path) => call('readArchivedText', 'read_archived_text', { path }),
  deleteArchivedFile: (path) =>
    call<void>('deleteArchivedFile', 'delete_archived_file', { path }),
  /* 删除本机那份官方交付文件（一切皆可删：删了回到「未下载」，随时可从云端重下） */
  deleteDeliveryFile: (fileName) =>
    call<void>('deleteDeliveryFile', 'delete_delivery_file', { fileName }),
  downloadCatalogFile: (fileName, onTick) =>
    call<void>('downloadCatalogFile', 'download_runtime_file', withTick({ fileName }, onTick)),
  downloadCatalogFiles: (fileNames, onTick) =>
    call('downloadCatalogFiles', 'download_runtime_files', withTick({ fileNames }, onTick)),
  readDownloadedText: (fileName) =>
    call('readDownloadedText', 'read_downloaded_text', { fileName }),
  getPresetSource: () => call('getPresetSource', 'get_preset_source'),
  // ★ 双源（2026-10-05）：选的是 `mode`；`customUrl` 只在 `custom` 时有意义。
  //   参数名用 `mode` / `customUrl`（Rust 侧 `Option<String>` 收 `customUrl`）。
  setPresetSource: (mode, customUrl) =>
    call('setPresetSource', 'set_preset_source', { mode, customUrl: customUrl ?? null }),
  clearPresetSource: () => call('clearPresetSource', 'clear_preset_source'),
  /* ——— 官方版本账 + 对比台（2026-10-08）———
     官方版本列表（已下载 / 新版本，**不是过时判定**）与对比台读/写用户预设的参数。
     baseline 是隐藏内部存储，前端拿不到它的路径。 */
  getOfficialVersions: (fileName) =>
    call('getOfficialVersions', 'get_official_versions', { fileName: fileName ?? null }),
  readPresetParams: (path) => call('readPresetParams', 'read_preset_params', { path }),
  savePresetParams: (path, edits) =>
    call<void>('savePresetParams', 'save_preset_params', { path, edits }),
  /* ——— 逐参数「官方更新」（2026-10-09）———
     读三方账（我 / 官方旧值 / 官方新值）+ 落采用·保持的决定。
     `fetchMissing` 只在打开某一份预设时给 true（允许为取官方新版发一次网络）。 */
  getPresetParamSync: (path, fetchMissing) =>
    call('getPresetParamSync', 'get_preset_param_sync', {
      path,
      fetchMissing: fetchMissing ?? false,
    }),
  applyPresetParamDecisions: (path, decisions) =>
    call('applyPresetParamDecisions', 'apply_preset_param_decisions', { path, decisions }),
  getActivePreset: () => call('getActivePreset', 'get_active_preset'),
  /* 两条线一个入口：`origin` 说这一份住哪条线，用户线还要给出它在用户根里的路径 */
  applyActivePreset: (fileName, origin, path) =>
    call('applyActivePreset', 'apply_active_preset', { fileName, origin, path }),
  /* 云端表那两个动作（2026-10-09 改判）：下载 / 更新 = 取回官方 + 落一份我的工作副本（都不改当前使用） */
  fetchOfficialPreset: (fileName) =>
    call('fetchOfficialPreset', 'fetch_official_preset', { fileName }),
  clearActivePreset: () => call('clearActivePreset', 'clear_active_preset'),
  checkRemoteUpdate: () => call('checkRemoteUpdate', 'check_remote_update'),
  applyRemoteUpdate: () => call('applyRemoteUpdate', 'apply_remote_update'),
  /* ——— 软件更新（release.json）—— 与预设数据两条链 ——— */
  getAppVersion: () => call('getAppVersion', 'get_app_version'),
  checkSoftwareUpdate: () => call('checkSoftwareUpdate', 'check_software_update'),
  /* 应用内更新（第五刀）：命令名与前端一一对应，后端在 `ipc/update.rs` */
  updateInfo: () => call('updateInfo', 'update_info'),
  startUpdate: () => call('startUpdate', 'start_update'),
  pauseUpdate: () => call('pauseUpdate', 'pause_update'),
  resumeUpdate: () => call('resumeUpdate', 'resume_update'),
  cancelUpdate: () => call('cancelUpdate', 'cancel_update'),
  installUpdate: () => call('installUpdate', 'install_update'),
  openUrl: (url) => call('openUrl', 'open_url', { url }),

  /* ——— 切片器目录（复制进去才生效）+ 复制官方链接（2026-10-09 接通）——— */
  copyToSlicer: (fileName) => call('copyToSlicer', 'copy_to_slicer', { fileName }),
  getFileUrl: (fileName) => call('getFileUrl', 'get_file_url', { fileName }),

  /* ——— 报告页：后处理执行报告与历史（gcode_history 的真账，只读）——— */
  getReportList: () => call('getReportList', 'get_report_list'),
  getReportDetail: (id) => call('getReportDetail', 'get_report_detail', { id }),
}
