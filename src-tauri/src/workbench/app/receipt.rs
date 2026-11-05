//! **生成事务回执**（2026-11-01）。
//!
//! 它是统一生成事务的**正式返回值**，不是生成完之后临时拼的 UI 文案：
//! 工作台拿它渲染模态框、CLI 直接输出这份 JSON、AI 读的也是同一份 ——
//! 三处读的是同一个结构，不存在"第二套报告"。
//!
//! 落点 **应用数据目录**（`<内部数据根>/workbench/receipts/<ID>.json`），
//! **不在仓库里**、按份数自动清理（[`KEEP`]）。
//!
//! 2026-11-01 的两轮裁定：
//! ① 它不该住仓库 —— 曾经落 `<repo>/workbench/receipts/`，攒了 91 份并被 git
//!   跟踪、混进提交，那是把本机运行时状态当仓库内容；
//! ② 但它也**不该只留内存** —— 作者要"关掉工作台之后仍能查上一次生成做了什么"。
//!
//! ★ **它是"生成"的凭据，不是完整操作历史。** 保存 / 发布 / 外部改文件都不在
//!   这里。作者明确不要把三种东西塞进同一套机制；将来要扩展，按"一次操作一条"
//!   记，不按"每个字段一条"。
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
    /// **真写了盘**的文件数。不是"处理了几个" —— 那个数会撒谎（2026-11-01）。
    pub written: usize,
    /// 内容与盘上那份相同、因此**没有写**的文件数。
    ///
    /// 它不能和 `written` 混成一个数：`written 0/9` 到底意味着
    /// "9 份都没变"（好事）还是"9 份都失败了"（坏事），只靠 written/total 分不出来。
    pub unchanged: usize,
    /// 真失败的文件数。
    pub failed: usize,
    /// 这次该处理的文件总数（= written + unchanged + failed）。
    pub total: usize,
}

impl Counts {
    /// 从真实的三个计数建一份（`total` 由三者相加，不另外传 —— 少一个能对不上的数）。
    pub fn of(written: usize, unchanged: usize, failed: usize) -> Self {
        Self {
            written,
            unchanged,
            failed,
            total: written + unchanged + failed,
        }
    }

    /// 这一组**什么都没发生需要人管**（没有失败）。
    pub fn clean(&self) -> bool {
        self.failed == 0
    }

    /// 一句话摘要：**0 写时要明确说"都没变"**，不许让它看起来像"没跑"。
    pub fn summary(&self) -> String {
        if self.failed > 0 {
            format!(
                "{}/{}/{}（写/未变/失败）",
                self.written, self.unchanged, self.failed
            )
        } else if self.written == 0 {
            format!("{} 份内容均未变化", self.total)
        } else {
            format!("写 {} 份、{} 份未变", self.written, self.unchanged)
        }
    }
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
    /// 这次**真写了盘**的文件（交付面那一批），用**文件名**（`A1-standard.toml`）。
    ///
    /// 2026-11-01：以前这里装的是 uid（`A1/STANDARD`）—— 字段说明写"文件名"、
    /// 实际却是另一套标识，谁也拿它定位不到盘上那份。回执必须能对上真实文件，
    /// 判定见 `the_receipt_matches_what_actually_landed_on_disk`。
    pub files: Vec<String>,
    /// 失败时的那一句（`AppError.message`）
    pub error: Option<String>,
}

impl GenerationReceipt {
    /// 开工时先立一份"什么都没做"的回执 —— 中途断了也能带着已完成的步子出门。
    pub fn start(_root: &Path, stamp: String) -> Self {
        Self {
            id: next_id(),
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

/// 回执目录：**应用数据根**下，不在仓库里（2026-11-01 作者裁定）。
///
/// 落点 `<内部数据根>/workbench/receipts/`，macOS 上是
/// `~/Library/Application Support/SupportEase/workbench/receipts/`。
///
/// 为什么不放仓库：曾经落在 `<repo>/workbench/receipts/`，攒了 91 份
/// `GEN-*.json` 并被 git 跟踪、混进提交 —— 那是把本机运行时状态当仓库内容。
///
/// 为什么还要落盘（而不是只留内存）：作者要的是"关掉工作台之后仍能查
/// '上一次生成为什么失败'"。只留内存做不到这一条。它落在**应用数据目录**、
/// 按份数自动清理，既不进 git 也不会无限堆。
///
/// ★ 它只是**生成**的凭据，不是完整操作历史。保存 / 发布 / 外部改文件
///   都不在这里 —— 别把三种东西塞进同一套机制（作者 2026-11-01 明确）。
pub fn receipts_dir() -> Option<PathBuf> {
    crate::fsx::paths::internal_root_headless().map(|r| r.join("workbench").join("receipts"))
}

/// 最多留几份回执。超了删最旧的 —— 它是"最近发生了什么"的凭据，不是档案。
pub const KEEP: usize = 50;

/// 下一条事务号：`GEN-<本地日期>-<当天第几笔>`。
///
/// 数**磁盘上当天已有的**份数（不是进程内计数器）：这样关掉重开不会重号，
/// 也与"最多留 KEEP 份"的清理口径一致。
pub fn next_id() -> String {
    let d = crate::archive::now_local().date();
    let day = format!("{:04}{:02}{:02}", d.year(), u8::from(d.month()), d.day());
    let prefix = format!("GEN-{day}-");
    let n = receipts_dir()
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with(&prefix) && n.ends_with(".json"))
                .count()
        })
        .unwrap_or(0);
    format!("{prefix}{:03}", n + 1)
}

/// 本次运行里**最近一份**回执（内存里的那份，给"刚才那次"用）。
static LAST: std::sync::Mutex<Option<GenerationReceipt>> = std::sync::Mutex::new(None);

/// 落一份回执：写进应用数据目录 + 记进内存 + 按份数清理。
///
/// **写不进去不报给用户**（生成本身已经成了）—— 但要留日志，由调用方打。
pub fn save(receipt: &GenerationReceipt) -> Result<PathBuf, String> {
    let dir = receipts_dir().ok_or_else(|| "应用数据目录不可用".to_owned())?;
    save_at(&dir, receipt)
}

/// 同 [`save`]，但**目录由调用方给**（测试用：不碰进程级环境变量 ——
/// `set_var` 是全局的，测试并发跑时会串味）。
pub fn save_at(dir: &Path, receipt: &GenerationReceipt) -> Result<PathBuf, String> {
    if let Ok(mut g) = LAST.lock() {
        *g = Some(receipt.clone());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("建 {} 失败：{e}", dir.display()))?;
    let path = dir.join(format!("{}.json", receipt.id));
    let text = serde_json::to_string_pretty(receipt).map_err(|e| format!("回执序列化失败：{e}"))?;
    crate::fsx::atomic::atomic_write(&path, text.as_bytes())
        .map_err(|e| format!("写 {} 失败：{}", path.display(), e.message))?;
    prune(dir);
    Ok(path)
}

/// 只留最近 [`KEEP`] 份。**清理失败不影响主流程** —— 它只是打扫房间。
fn prune(dir: &Path) {
    let Ok(it) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = it
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    if files.len() <= KEEP {
        return;
    }
    // 文件名里带日期与序号，字典序就是时间序
    files.sort();
    for p in &files[..files.len() - KEEP] {
        let _ = std::fs::remove_file(p);
    }
}

/// 历次生成回执（**新的在前**）。给"在工作台里查以前生成了什么"用。
///
/// 读失败就当空 —— 回执是旁证，不是数据源，读不到不该让界面报错。
pub fn history(limit: usize) -> Vec<GenerationReceipt> {
    let Some(dir) = receipts_dir() else {
        return Vec::new();
    };
    history_at(&dir, limit)
}

/// 同 [`history`]，但目录由调用方给（测试用）。
pub fn history_at(dir: &Path, limit: usize) -> Vec<GenerationReceipt> {
    let Ok(it) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = it
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .rev()
        .take(limit)
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect()
}

/// 上一次生成的回执（本次运行内；没有就是 `None`）。
pub fn last() -> Option<GenerationReceipt> {
    LAST.lock().ok().and_then(|g| g.clone())
}
#[cfg(test)]
mod tests {
    use super::*;

    /// 回执落**应用数据目录**，且读得回来（关掉工作台之后仍能查）。
    ///
    /// 这条路 2026-11-01 走过两个极端：先是落仓库（91 份被 git 跟踪），
    /// 再是只留内存（关掉就没了）。现在落应用数据目录 —— 既不进仓库，
    /// 又留得住。这条判据钉住"留得住"那一半。
    #[test]
    fn a_receipt_lands_in_the_app_data_dir_and_reads_back() {
        /* 用**显式目录**那一组函数，不碰 `MKP_SUPPORT_EASE_DATA`：
         * `set_var` 是进程级的，测试并发跑时别的用例会看见它
         * （实测这条单独跑绿、全量跑红，就是这个原因）。 */
        let dir = tempfile::tempdir().unwrap();
        let receipts = dir.path().join("workbench").join("receipts");

        let mut r = GenerationReceipt::start(Path::new("."), "2026-11-01T00:00:00Z".to_owned());
        r.status = ReceiptStatus::Passed;
        r.step("delivery", true, "2/9");
        r.files = vec!["A1-standard.toml".to_owned()];

        let path = save_at(&receipts, &r).expect("回执该写下去");
        assert!(
            path.starts_with(dir.path()),
            "必须落在应用数据目录里，而不是仓库：{}",
            path.display()
        );

        let back = history_at(&receipts, 10);
        let got = back.first().expect("该读得回来");
        assert_eq!(got.id, r.id);
        assert_eq!(got.status, ReceiptStatus::Passed);
        assert_eq!(got.files, r.files);
        assert!(
            got.steps
                .iter()
                .any(|s| s.key == "delivery" && s.detail == "2/9"),
            "每一步的真实结果都要留着"
        );
    }

    /// 超过 [`KEEP`] 份要删最旧的 —— 它是"最近发生了什么"，不是无限档案。
    #[test]
    fn old_receipts_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let receipts = dir.path().join("workbench").join("receipts");
        std::fs::create_dir_all(&receipts).unwrap();

        for i in 0..(KEEP + 5) {
            crate::fsx::atomic::atomic_write(
                &receipts.join(format!("GEN-20260101-{:03}.json", i + 1)),
                b"{}",
            )
            .unwrap();
        }
        prune(&receipts);

        let left = std::fs::read_dir(&receipts).unwrap().count();
        assert_eq!(left, KEEP, "只留最近 {KEEP} 份");
        assert!(
            !receipts.join("GEN-20260101-001.json").exists(),
            "最旧的那份该被删掉"
        );
        assert!(
            receipts
                .join(format!("GEN-20260101-{:03}.json", KEEP + 5))
                .exists(),
            "最新的那份要留着"
        );
    }
}
