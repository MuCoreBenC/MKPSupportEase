/*
 * 外壳快捷键的闸（2026-10-03，作者：「打开模态框撤销重做它还是撤销那外面的」）。
 *
 * 撤销 / 重做 / 保存的键盘入口装在外壳（App）的 window 上 —— 只要有一个模态框
 * 开着，Cmd+Z 就会穿透到遮罩后面，把看不见的草稿改掉。工作台里所有模态框都走
 * `Modal` 这一个件，所以闸也装在这一处：开着的框各记一票，外壳处理快捷键前
 * 先问一句 `blocking()`。
 *
 * 让路（`shellShortcuts`）只给一种框：G-code 模态框 —— 它的每一笔写都进外壳的
 * 草稿栈，页脚那句「Ctrl+Z 可撤回」说的就是外壳那条栈，不能断。
 */

let openCount = 0
let bypassCount = 0

export const modalShortcutGate = {
  enter(shellShortcuts: boolean): void {
    openCount += 1
    if (shellShortcuts) bypassCount += 1
  },
  exit(shellShortcuts: boolean): void {
    openCount = Math.max(0, openCount - 1)
    if (shellShortcuts) bypassCount = Math.max(0, bypassCount - 1)
  },
  /** true = 有模态框开着且没让路 —— 外壳的快捷键该装没听见 */
  blocking(): boolean {
    return openCount > bypassCount
  },
}
