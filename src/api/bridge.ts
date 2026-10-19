import { invoke, Channel } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

import { armedFaultFor } from '../app/dev/fault'
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
 * 2026-10-09 起 `copyToSlicer`（复制进切片器目录）与 `getFileUrl`（复制云端链接）
 * 也接上了真 command；`downloadFiles`（云端仓库文件的「下载」残支）按用户裁断
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

/**
 * 从"挑中的那个路径"取显示名：去掉目录，再去掉 `.app` / `.exe` 那层壳
 * （`/Applications/PrusaSlicer.app` → `PrusaSlicer`）。
 *
 * 为什么在挑的时候就取好：界面直接拿它当下拉里那一条的名字，不用再解析路径。
 */
function appNameOf(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path
  return base.replace(/\.(app|exe)$/i, '')
}

function safeStringify(v: unknown): string {
  try {
    return JSON.stringify(v) ?? String(v)
  } catch {
    return String(v)
  }
}

/**
 * 调一个 command，失败时把错误规整成 AppError 再抛。
 *
 * ★ 2026-10-19：**开发台的故障注入在这一层查一次表**（`armedFaultFor`）。
 * 命中就**不调后端**、直接 reject 那一条错 —— 那条错是客户端进程按真值代码造的，
 * 于是界面上出问题的是真组件、真按钮、真文案、真错误码（作者：「要真实的 mock 触发」）。
 *
 * 查询在**这一层**而不是散在各页面：产品逻辑一行不改，一个开关覆盖所有走这个方法的
 * 界面（详见 `src/app/dev/fault.ts` 抬头）。正式包里那张表恒空 —— 恒不命中。
 */
async function call<T>(method: MkpApiMethod, command: string, args?: Record<string, unknown>) {
  try {
    const injected = armedFaultFor(method)
    if (injected !== null) throw injected
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
  openModel: (modelId, opts) =>
    call('openModel', 'open_model', {
      modelId,
      slicer: opts?.slicer ?? null,
      /* 「从磁盘指定的那个」：只在这一档给路径（壳读不到 webview 的 localStorage） */
      slicerApp: opts?.slicerApp ?? null,
    }),
  /*
   * 「其他应用…」—— 从磁盘挑一个切片软件（2026-10-13 第二刀）。
   *
   * 与 `pickExportDir` 同一个插件、同一半权限（`dialog:allow-open`），**不新增 Rust 命令**：
   * 挑一个文件本来就是界面上的事，落点只是"记下这个名字与路径"（纯前端偏好，
   * `STORAGE.clientSlicerApp`）。
   *
   * 两边成熟的那一套不一样：macOS 的切片软件是 `.app` 包（**是个目录**）⇒ 停在
   * `/Applications` 让人挑包；Windows 挑 `.exe`。**取消 = `null`，不是错误**。
   */
  pickSlicerApp: async () => {
    const mac = /Mac/i.test(navigator.userAgent)
    const picked = await open({
      /* macOS 挑的是 `.app` 包（目录本身）；Windows 挑可执行文件 */
      directory: mac,
      multiple: false,
      title: '选择用来打开模型的切片软件',
      defaultPath: mac ? '/Applications' : undefined,
      filters: mac ? undefined : [{ name: '程序', extensions: ['exe'] }],
    })
    if (picked === null || Array.isArray(picked)) return null
    return { name: appNameOf(picked), path: picked }
  },
  /*
   * 「跟随系统默认」到底是哪个应用（2026-10-16）：**问系统真值** ——
   * macOS 走 LaunchServices、Windows 走注册表，两套都在壳那一侧
   * （`ipc::slicer::system_default_app`：前端不扫盘、不猜安装位置）。
   * 问不出来给 `null`，界面回落原来那句话术。
   */
  getSystemSlicerApp: () => call('getSystemSlicerApp', 'get_system_slicer_app'),
  /*
   * 「去系统里改默认应用」：**我们不替用户改**，只把他送到系统那一处 ——
   * macOS 打开那份空的示例 `.3mf` 的**「显示简介」窗口**（要过一次「控制访达」的授权；
   * 只把文件夹定位出来不算，作者 2026-10-16 纠过），Windows 打开「打开方式」框。
   * 回来的那句是下一步提示，直接弹给用户。
   */
  openDefaultAppSettings: () => call('openDefaultAppSettings', 'open_default_app_settings'),
  /* 「自动化」授权被拒后的反悔入口：直达系统设置那一页（macOS 拒过一次就不再弹框） */
  openAutomationSettings: () => call('openAutomationSettings', 'open_automation_settings'),
  /* 「复制后处理脚本」那一整条命令：壳侧现拼（current_exe() + 预设真实绝对落点）。
     `path` = 被点名的那一份（预设页右键那一行）—— 不传就是"当前使用的那一套" */
  getPostProcessCommand: (machineId, versionId, path) =>
    call('getPostProcessCommand', 'get_post_process_command', { machineId, versionId, path }),
  /* 钩子那一趟：切片器导出时本程序被带参数拉起来的那一次（快照 / 停 / 答一问） */
  getPostProcessRun: () => call('getPostProcessRun', 'get_post_process_run'),
  cancelPostProcess: () => call('cancelPostProcess', 'cancel_post_process'),
  /* 回答屏幕上那一问：`id` 是那一颗按钮的 id（`keep` / `stop` / `print` / `direct` …） */
  answerPostProcess: (questionKind, id) =>
    call('answerPostProcess', 'answer_post_process', { questionKind, id }),
  /* 首帧那两条路的收尾：快照里有一问、但自己没收到事件 ⇒ 让它补推一次（只重新 emit） */
  replayPostProcessQuestion: () =>
    call('replayPostProcessQuestion', 'replay_post_process_question'),
  /* 首帧竞态的另一半：开场那条落在缝里 ⇒ 把这一趟整份重推（只重新 emit） */
  replayPostProcessRun: () => call('replayPostProcessRun', 'replay_post_process_run'),

  /* ——— 预设页（A41）的读接口：走真 command —— */
  getMachines: () => call('getMachines', 'get_machines'),
  getVersionFiles: (machineId, versionId) =>
    call('getVersionFiles', 'get_version_files', { machineId, versionId }),
  getLocalFiles: () => call('getLocalFiles', 'get_local_files'),
  /* 用户线：用户自己的预设（住 <appDataDir>/user/presets-local） */
  getLocalPresetFiles: () => call('getLocalPresetFiles', 'get_local_preset_files'),
  /* 备注覆盖账（副标题）+ 改归属（文件头 # machine/# variant 两行） */
  getPresetRemarks: () => call('getPresetRemarks', 'get_preset_remarks'),
  setPresetRemark: (key, remark) =>
    call('setPresetRemark', 'set_preset_remark', { key, remark }),
  setLocalPresetMachineVersion: (path, machineId, versionId) =>
    call('setLocalPresetMachineVersion', 'set_local_preset_machine_version', {
      path,
      machineId,
      versionId,
    }),
  readLocalPresetText: (path) => call('readLocalPresetText', 'read_local_preset_text', { path }),
  /* 临时编辑那条链：改的是临时文件，原件全程不动（保存才落用户根：云端另存 / 本地写回） */
  beginPresetEdit: (fileName, origin, path) =>
    call('beginPresetEdit', 'begin_preset_edit', { fileName, origin, path }),
  putPresetDraft: (text) => call('putPresetDraft', 'put_preset_draft', { text }),
  patchPresetDraft: (paramKey, value) =>
    call('patchPresetDraft', 'patch_preset_draft', { paramKey, value }),
  discardPresetDraft: () => call('discardPresetDraft', 'discard_preset_draft'),
  /* 冷启动接回草稿层：后端草稿还在，界面那一层（内存）没了 —— 差异由后端按参数算 */
  readPresetDraftChanges: () => call('readPresetDraftChanges', 'read_preset_draft_changes'),
  /* 修改历史账本（2026-10-17）：一份文件一本，盘上留着、重开软件还在 */
  getPresetHistory: (origin, id) => call('getPresetHistory', 'get_preset_history', { origin, id }),
  /* 「哪份用得最多」：只读；写在后端"应用预设"那一步（2026-10-17） */
  getPresetUsage: () => call('getPresetUsage', 'get_preset_usage'),
  putPresetHistory: (origin, id, history) =>
    call('putPresetHistory', 'put_preset_history', {
      origin,
      id,
      batches: history.batches,
      entries: history.entries,
    }),
  clearPresetHistory: (origin, id) =>
    call('clearPresetHistory', 'clear_preset_history', { origin, id }),
  /* 备份与恢复（2026-10-17）：设置页，zip 快照 + 全量覆盖 */
  createBackup: (destDir) => call('createBackup', 'create_backup', { destDir }),
  restoreBackup: (zipPath) => call('restoreBackup', 'restore_backup', { zipPath }),
  commitPresetDraft: () => call('commitPresetDraft', 'commit_preset_draft'),
  /* 「编辑文件」那条链（2026-10-10：与参数草稿分开存，两侧互相作废） */
  getPresetDraftSummary: () =>
    call('getPresetDraftSummary', 'get_preset_draft_summary'),
  beginTextEdit: (fileName, path) =>
    call('beginTextEdit', 'begin_text_edit', { fileName, path }),
  /* 「恢复默认值」（编辑文件那颗）读的原文镜像 —— `baseline-local/`，见 Rust `runtime::baseline` */
  readLocalOriginText: (path) =>
    call('readLocalOriginText', 'read_local_origin_text', { path }),
  putTextDraft: (text) => call('putTextDraft', 'put_text_draft', { text }),
  commitTextDraft: () => call('commitTextDraft', 'commit_text_draft'),
  discardTextDraft: () => call('discardTextDraft', 'discard_text_draft'),
  /* 身份账本（派生视图：盘上文件 + 血统 → 云端那一版） */
  getLocalPresetIndex: () => call('getLocalPresetIndex', 'get_local_preset_index'),
  /* 第十层：用户文件管理（改名 / 删除）—— 只动名字或删掉，字节一个不动 */
  renameLocalPreset: (path, newName) =>
    call('renameLocalPreset', 'rename_local_preset', { path, newName }),
  deleteLocalPreset: (path) => call('deleteLocalPreset', 'delete_local_preset', { path }),
  /* 第十一层：本地文件 → 本地文件（字节复制；不覆盖、不碰任何状态） */
  copyLocalPreset: (path, newName) =>
    call('copyLocalPreset', 'copy_local_preset', { path, newName }),
  /* 另存为本地预设：云端那一份 → 本地这份（撞名就拒、带血统）+ 按机型/版本找它（校准页） */

  getLocalCopyFor: (machineId, versionId) =>
    call('getLocalCopyFor', 'get_local_copy_for', { machineId, versionId }),
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
  /* 第十三层：文件外部管理（打开系统文件管理器并选中这一份；只在「本地文件」上做） */
  revealInFolder: (path) => call('revealInFolder', 'reveal_in_folder', { path }),
  /* 两个落点各自的「打开所在文件夹」（2026-10-13）：我们那份（下载区）与切片器里那份 */
  revealDeliveredFile: (fileName) =>
    call('revealDeliveredFile', 'reveal_delivered_file', { fileName }),
  revealSlicerFile: (fileName) => call('revealSlicerFile', 'reveal_slicer_file', { fileName }),
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
  /* 归档区（交付预设旧版本留档）：列 + 读正文 + 删（2026-10-06 起允许删，代价在确认框讲清） */
  getArchivedFiles: () => call('getArchivedFiles', 'get_archived_files'),
  readArchivedText: (path) => call('readArchivedText', 'read_archived_text', { path }),
  deleteArchivedFile: (path) =>
    call<void>('deleteArchivedFile', 'delete_archived_file', { path }),
  /* 删除本机那份交付预设文件（一切皆可删：删了回到「未下载」，随时可从云端重下） */
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
  /* 界面偏好（2026-10-12）：主题（浅 / 深 / 跟随系统）—— 落 AppState 的 theme 格 */
  getTheme: () => call('getTheme', 'get_theme'),
  setTheme: (theme) => call('setTheme', 'set_theme', { theme }),
  /* 界面偏好第二格（2026-10-13）：执行记录的自动清理开关 —— 落 autoCleanHistory 格 */
  getAutoCleanHistory: () => call('getAutoCleanHistory', 'get_auto_clean_history'),
  setAutoCleanHistory: (enabled) =>
    call('setAutoCleanHistory', 'set_auto_clean_history', { enabled }),
  /* 后处理偏好（2026-10-17）：设置页「后处理模式 / 校准与补偿 / 热床预涂胶」那几块。
     读回的是**解析后**的值 + 量程；写是**整格覆盖**（改一项传全量） */
  getPostprocPrefs: () => call('getPostprocPrefs', 'get_postproc_prefs'),
  setPostprocPrefs: (prefs) => call('setPostprocPrefs', 'set_postproc_prefs', { prefs }),
  /* 重置后处理状态（设置页那一颗按钮）：先让钩子别等，再清界面状态 */
  resetPostProcessState: () => call('resetPostProcessState', 'reset_post_process_state'),
  /* 应用本体那两条（2026-10-17）：「预设位置」打开目录；「卸载」删程序本体、数据不动 */
  openPresetFolder: () => call('openPresetFolder', 'open_preset_folder'),
  uninstall: () => call('uninstall', 'uninstall'),
  /* 主窗怎么摆（2026-10-17）：置顶三档 + 窄窗 —— 落 pin / compact 两格。
     窗口几何的副作用全在 `ipc::window_layout`，前端只发意图、不碰 setSize */
  getWindowPrefs: () => call('getWindowPrefs', 'get_window_prefs'),
  setPin: (pin, onBbs) => call('setPin', 'set_pin', { pin, onBbs }),
  applyPin: (onBbs) => call('applyPin', 'apply_pin', { onBbs }),
  setCompact: (compact) => call('setCompact', 'set_compact', { compact }),
  /* 屏幕引导（2026-10-15）：改了参数之后指给用户看那一步 —— 只读坐标，不替用户点 */
  locateBedTypeControl: () => call('locateBedTypeControl', 'locate_bed_type_control'),
  guideShow: () => call('guideShow', 'guide_show'),
  guideClose: () => call<void>('guideClose', 'guide_close'),
  /* ——— 云端版本账 + 对比台（2026-10-08）———
     云端版本列表（已下载 / 新版本，**不是过时判定**）与对比台读/写用户预设的参数。
     baseline 是隐藏内部存储，前端拿不到它的路径。 */
  getCloudVersions: (fileName) =>
    call('getCloudVersions', 'get_cloud_versions', { fileName: fileName ?? null }),
  readPresetParams: (path) => call('readPresetParams', 'read_preset_params', { path }),
  savePresetParams: (path, edits) =>
    call<void>('savePresetParams', 'save_preset_params', { path, edits }),
  /* ——— 逐参数「云端更新」（2026-10-09）———
     读三方账（我 / 云端旧值 / 云端新值）+ 落采用·保持的决定。
     `fetchMissing` 只在打开某一份预设时给 true（允许为取云端新版发一次网络）。 */
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
  /* 云端表那两个动作（2026-10-09 改判）：下载 / 更新 = 取回交付预设 + 落一份我的工作副本（都不改当前使用） */
  fetchCloudPreset: (fileName, renameIfTaken) =>
    call('fetchCloudPreset', 'fetch_cloud_preset', { fileName, renameIfTaken }),
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

  /* ——— 切片器目录（复制进去才生效）+ 复制云端链接（2026-10-09 接通）——— */
  copyToSlicer: (fileName) => call('copyToSlicer', 'copy_to_slicer', { fileName }),
  getFileUrl: (fileName) => call('getFileUrl', 'get_file_url', { fileName }),

  /* ——— 本机 BBS 目录（2026-10-13）：工艺页「本机工艺」那一档 ——— */
  getBbsPresets: () => call('getBbsPresets', 'get_bbs_presets'),
  readBbsPreset: (scope, uid, file) =>
    call('readBbsPreset', 'read_bbs_preset', { scope, uid, file }),

  /* ——— 报告页：后处理执行报告与历史（读 gcode_history 的真账 + 一处导出写）——— */
  getReportList: () => call('getReportList', 'get_report_list'),
  getReportDetail: (id) => call('getReportDetail', 'get_report_detail', { id }),
  getReportExportPlan: (id) => call('getReportExportPlan', 'get_report_export_plan', { id }),
  exportReportArchive: (id, dir) =>
    call('exportReportArchive', 'export_report_archive', { id, dir }),
  revealReportExport: (id, dir) =>
    call('revealReportExport', 'reveal_report_export', { id, dir }),
  /*
   * 导出框那颗「更改位置」：与 `pickImportFiles` 同一半权限（`dialog:allow-open` ——
   * 选目录与选文件在插件里是同一个 `open`，这里只是多个 `directory`）。
   * 取消 = `null`（不是错误），界面据此保持原来那个位置。
   */
  pickExportDir: async (current) => {
    const picked = await open({
      directory: true,
      title: '选择保存位置',
      defaultPath: current,
    })
    return typeof picked === 'string' ? picked : null
  },
}
