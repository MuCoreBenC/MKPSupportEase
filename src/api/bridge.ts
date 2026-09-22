import { invoke } from '@tauri-apps/api/core'

import { isAppError, type AppError, type MkpApi, type MkpApiMethod } from './contract'

/**
 * IPC 桥：把契约里的四个方法映射到 Rust 侧的四个 command。
 *
 * 命名两套、映射在这一处：TS 侧是 `getPreset`（前端习惯），Rust 侧是 `get_preset`
 * （Rust 习惯）。Tauri 会把 JS 传进去的 camelCase 参数名转成 snake_case，
 * 所以参数照常写 `{ variantId }`。
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

export const bridgeApi: MkpApi = {
  getPreset: (variantId) => call('getPreset', 'get_preset', { variantId }),
  saveOffsets: (axes) => call('saveOffsets', 'save_offsets', { axes }),
  getCalibModels: () => call('getCalibModels', 'get_calib_models'),
  openModel: (modelId) => call('openModel', 'open_model', { modelId }),
}
