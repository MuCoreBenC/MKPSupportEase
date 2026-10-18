/*
 * **开发台 · 故障注入**：客户端这一侧的接收口（作者 2026-10-19）。
 *
 * # 它做什么
 *
 * 工作台「开发台」点一档故障 → 发一条指令 → 客户端进程**按 id 自己造那条错** →
 * emit `dev-fault` → 这里记下一张表：**方法名 → 那一条 `AppError`**。
 * `src/api/bridge.ts` 的 `call()` 调后端之前查一次表，命中就 `reject` 那条错。
 *
 * ```text
 *   工作台 ──invoke──► wb_dev_fault_arm(id) ──loopback──► 客户端进程
 *                                                            │ 按 id 造错（真值同一份代码）
 *                                                            ▼
 *                                            本文件（listen `dev-fault` → 记表）
 *                                                            │
 *                                            bridge.ts 的 call() 查表 → reject
 * ```
 *
 * # ★★ 为什么"真实"
 *
 * 这一层**不碰任何界面**：它只让"后端这一次调用失败"。于是出问题的是
 * **真组件、真按钮、真文案、真错误码** —— 没有"预览模式"那一层假
 * （作者：「按钮都不一样，按钮还是关闭预览，根本不真实」）。
 *
 * # 为什么挂在「方法」上
 *
 * 一个开关覆盖**所有**走这个方法的界面：产品代码一行不改，加一档只改后端
 * `src-tauri/src/devfault.rs` 的 `faults()`。挂在"某个模态框"上就得逐个在产品
 * 逻辑里留钩子 —— 那是演示用代码住进产品。
 *
 * # 不在产品逻辑里留钩子
 *
 * 正式包里没有 `wb_dev_fault_arm`，也没有谁会 emit `dev-fault` —— 事件收不到时
 * 这张表恒为空，`call()` 那一次查询是恒不命中的。产品逻辑一行没改。
 */

import type { AppError } from '../../api/contract'

/** Tauri 事件名 —— 与 Rust 侧 `hook_ui::DEV_FAULT_EVENT` **必须逐字一样** */
export const DEV_FAULT_EVENT = 'dev-fault'

/** 一条故障注入指令的形状（Rust `hook_ui::DevFaultDto`） */
export interface DevFaultDto {
  /** 装在哪个方法上（`MkpApiMethod` 名）。`null` = 卸下 */
  method: string | null
  /** 那一条错。`method` 为 `null` 时它是 `null` */
  error: AppError | null
}

/**
 * 现在装着的那一档 —— **模块级**（不是 React state）。
 *
 * 为什么不放 context：`bridge.ts` 不在 React 树里，它要的是"现在这一刻查一次"。
 * 放在 React 里就得把整棵 api 层塞进 Provider。
 */
let armed: { method: string; error: AppError } | null = null

/** 装上 / 卸下。`null` = 卸下 */
export function setArmedFault(next: { method: string; error: AppError } | null): void {
  armed = next
}

/** `call()` 查这一下：这个方法现在装着错吗（装着就返回那一条） */
export function armedFaultFor(method: string): AppError | null {
  return armed !== null && armed.method === method ? armed.error : null
}

/** 现在装着的那一档（给界面回显用；没装是 `null`） */
export function currentArmedFault(): { method: string; error: AppError } | null {
  return armed
}
