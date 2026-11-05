//! **请求号回执**（2026-11-02）：让前端知道"刚跑完的这一笔叫什么号"。
//!
//! # 要解决什么
//!
//! 一次点击会打出七八行日志（写队列 → IPC → 值进影子树 → 参数定义已改 → 派生 → 事务）。
//! 日志每一行都带 `req=r0000002a`（[`super::current_req`]），按这个号 `grep` 就能把
//! 一次点击的全过程捞出来。**但前端拿不到号**：它发了请求、收下返回值，却不知道
//! 这一次对应日志里的哪一串 —— 作者排查时只能靠时间戳猜。
//!
//! # 为什么不改返回值形状
//!
//! 试过两条路，都不行：
//!
//! 1. **给每条命令套 `.map()`** —— 二百多处、每处要标类型，且客户端那 130 条命令的
//!    返回形状是 `src/api/contract.ts` 的契约，不该被搅动。
//! 2. **在注册表里包一层 `reqed(app::wb_x)`** —— `tauri::command` 生成的
//!    `__cmd__wb_x` / `__tauri_command_name_wb_x` 两个隐藏项挂在**原路径**上，
//!    被包进 `app::reqed::wb_x` 之后 `generate_handler!` 就找不到了。
//!
//! 所以号从**另一条通道**走：[`crate::ipc::traced`] 每跑完一条工作台命令就 emit
//! 一个 `wb-command` 事件（`{req, command, ok}`）。前端订阅一次，把号记在手上；
//! 界面上的"查看日志"按这个号过滤。返回值一个字节都不动。
//!
//! # 只读命令也有号
//!
//! 只读命令（`wb_txn_state` 那种 800ms 轮询）同样有号 —— 但它不写盘，日志里没有它
//! 的行，前端按号过滤自然查不到。**不区分**：区分就得在每个命令上标一次"是否写"，
//! 而那个标记本身就会漂。

use serde::Serialize;

/// `wb-command` 事件的名字（与 `src/workbench/api.ts` 里的订阅逐字一致）
pub const EVENT: &str = "wb-command";

/// 一条命令回执。
#[derive(Debug, Clone, Serialize)]
pub struct CommandReceipt {
    /// 这一笔的请求号（`r0000002a`）；不在命令里时是空串
    pub req: String,
    /// 命令名（`wb_apply_draft`），与日志里的 `label` 同一个词
    pub command: &'static str,
    /// 成功还是报错。报错时日志里有同一号的 `warn` 行
    pub ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 事件名是前后端的接头，改一个字前端就收不到 —— 钉住它
    #[test]
    fn event_name_is_stable() {
        assert_eq!(EVENT, "wb-command");
    }

    /// 回执的 JSON 形状（前端 `api.ts` 按这三个字段读）
    #[test]
    fn receipt_serializes_flat() {
        let r = CommandReceipt {
            req: "r0000002a".to_owned(),
            command: "wb_apply_draft",
            ok: true,
        };
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["req"], "r0000002a");
        assert_eq!(j["command"], "wb_apply_draft");
        assert_eq!(j["ok"], true);
    }
}
