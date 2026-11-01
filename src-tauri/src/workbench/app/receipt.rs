//! **生成事务回执**（2026-11-01）。
//!
//! 它是统一生成事务的**正式返回值**，不是生成完之后临时拼的 UI 文案：
//! 工作台拿它渲染模态框、CLI 直接输出这份 JSON、AI 读的也是同一份 ——
//! 三处读的是同一个结构，不存在"第二套报告"。
//!
//! **只在内存里，不落盘、不入库**（2026-11-01 作者裁定）。
//!
//! 它回答的是"刚才那次生成做了什么" —— 生成结束后由返回值带回界面。
//! 曾经落过 `workbench/receipts/<ID>.json`，结果 91 份 `GEN-*.json` 被 git 跟踪、
//! 混进提交：那是把本机运行时状态当成仓库内容。作者的原话是
//! "这些不要放到我这个仓库呀，就放临时路径，而且会自动清理，不需要存起来"。
//!
//! 要追溯历次生成，看 git 里的产物与 `built.json` 就够了，不需要第二套存档。
//!
//! ## 一条硬规矩
//!
//! **失败也必须有回执。** 只弹一句"生成失败"是把现场丢了 —— 哪一步失败、
//! 哪些文件已经落盘、哪些没写，都得在里面。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 这次生成是整笔成了，还是中途断了。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReceiptStatus {
    Passed,
    Failed,
}

/// 事务里的一步。`key` 是前端取文案的键（后端不写死用户能读的句子）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRow {
    pub key: String,
    pub ok: bool,
    /// 给机器看的补充（数字 / 文件名 / 报错原文）。用户读的那句话由前端按 `key` 取。
    pub detail: String,
}

/// 一组产物的"写了 / 共"。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub written: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationReceipt {
    /// `GEN-20261101-001`（按**本地日期**分段；同一天递增）
    pub id: String,
    /// 生成时刻（UTC，与产物头同一个时钟）
    pub stamp: String,
    pub status: ReceiptStatus,
    pub steps: Vec<StepRow>,
    pub delivery: Counts,
    pub assets: Counts,
    pub baseline: Counts,
    /// 这次真写了的文件名（交付面那一批）
    pub files: Vec<String>,
    /// 失败时的那一句（`AppError.message`）
    pub error: Option<String>,
}

impl GenerationReceipt {
    /// 开工时先立一份"什么都没做"的回执 —— 中途断了也能带着已完成的步子出门。
    pub fn start(root: &Path, stamp: String) -> Self {
        Self {
            id: next_id(root),
            stamp,
            status: ReceiptStatus::Failed,
            steps: Vec::new(),
            delivery: Counts::default(),
            assets: Counts::default(),
            baseline: Counts::default(),
            files: Vec::new(),
            error: None,
        }
    }

    /// 记一步。`ok = false` 时调用方应当立刻返回（后续步骤不会再有结果）。
    pub fn step(&mut self, key: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(StepRow {
            key: key.to_owned(),
            ok,
            detail: detail.into(),
        });
    }
}

/// 下一条事务号：`GEN-<本地日期>-<当天第几笔>`。
///
/// **进程内计数**（2026-11-01 作者裁定：回执不落盘、不进仓库）。
/// 取"这个进程今天已经开了几笔"，所以编号只在本次运行内有意义 ——
/// 它本来就是给人看"这是今天第几次生成"的，不该为此在仓库里留一堆文件。
pub fn next_id(_root: &Path) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let d = crate::archive::now_local().date();
    let day = format!("{:04}{:02}{:02}", d.year(), u8::from(d.month()), d.day());
    let n = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    format!("GEN-{day}-{n:03}")
}

/// 本次运行里**最近一份**回执 —— 就放在内存里，给"刚才那次生成做了什么"这个问题。
///
/// 2026-11-01 作者裁定：回执是**临时展示**的东西，不是要存起来的资产。
/// 原来它落到 `<repo>/workbench/receipts/`，结果 91 份 `GEN-*.json` 被 git 跟踪、
/// 混进提交里 —— 那是把本机运行时状态当成仓库内容了。
///
/// 现在：进程活着就在，进程退出就没了。生成结束后界面拿的是返回值里的那一份，
/// 不需要"过几天再回来查上一次"。真要把历次生成留成可追溯的记录，那是 Git 的事
/// （产物与 `built.json` 都在版本里），不该再造一套回执存档。
static LAST: std::sync::Mutex<Option<GenerationReceipt>> = std::sync::Mutex::new(None);

/// 把这份回执记为"最近一份"（**只在内存**，不写盘）。
///
/// 签名保留 `Result` 只是为了让调用点不用改形状；内存操作不会失败。
pub fn save(_repo_root: &Path, receipt: &GenerationReceipt) -> Result<(), String> {
    if let Ok(mut g) = LAST.lock() {
        *g = Some(receipt.clone());
    }
    Ok(())
}

/// 上一次生成的回执（本次运行内；没有就是 `None`）。
pub fn last() -> Option<GenerationReceipt> {
    LAST.lock().ok().and_then(|g| g.clone())
}
