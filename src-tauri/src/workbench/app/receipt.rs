//! **生成事务回执**（2026-11-01）。
//!
//! 它是统一生成事务的**正式返回值**，不是生成完之后临时拼的 UI 文案：
//! 工作台拿它渲染模态框、CLI 直接输出这份 JSON、AI 读的也是同一份 ——
//! 三处读的是同一个结构，不存在"第二套报告"。
//!
//! 落点 `workbench/receipts/<ID>.json`（本机状态，与 `.draft` / `.snapshots`
//! 同一族，不入库）：事后要回答"上一次生成为什么失败"时读它，而不是重新猜。
//!
//! ## 一条硬规矩
//!
//! **失败也必须有回执。** 只弹一句"生成失败"是把现场丢了 —— 哪一步失败、
//! 哪些文件已经落盘、哪些没写，都得在里面。

use std::path::{Path, PathBuf};

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

/// 回执目录：`<repo>/workbench/receipts/`。
pub fn receipts_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("workbench").join("receipts")
}

/// 下一条事务号：`GEN-<本地日期>-<当天第几笔>`。
///
/// 取"当天已有几份"而不是全局计数器：删掉旧回执不会让新号重复到已删除的号上，
/// 人也一眼知道这是哪一天的。
pub fn next_id(repo_root: &Path) -> String {
    let d = crate::archive::now_local().date();
    let day = format!("{:04}{:02}{:02}", d.year(), u8::from(d.month()), d.day());
    let prefix = format!("GEN-{day}-");
    let n = std::fs::read_dir(receipts_dir(repo_root))
        .ok()
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with(&prefix) && n.ends_with(".json"))
                .count()
        })
        .unwrap_or(0);
    format!("{prefix}{:03}", n + 1)
}

/// 落一份回执。**写不进去不报给用户**（生成本身已经成了）—— 但要留日志。
pub fn save(repo_root: &Path, receipt: &GenerationReceipt) -> Result<PathBuf, String> {
    let dir = receipts_dir(repo_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("建 {} 失败：{e}", dir.display()))?;
    let path = dir.join(format!("{}.json", receipt.id));
    let text = serde_json::to_string_pretty(receipt).map_err(|e| format!("回执序列化失败：{e}"))?;
    crate::fsx::atomic::atomic_write(&path, text.as_bytes())
        .map_err(|e| format!("写 {} 失败：{}", path.display(), e.message))?;
    Ok(path)
}
