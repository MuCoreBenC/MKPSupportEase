//! **修改历史的两条命令**（2026-10-17）：读一本、写一本。
//!
//! 界面那一侧（参数弹窗「修改历史」抽屉）原来把整本历史放在 React 的 state 里 ——
//! 关掉弹窗 / 切个组合 / 退出软件就没了。作者 2026-10-17 定的口径是
//! 「**类似 git 的保存那种形式吧，不要清空，每次重新打开软件都存在**」，
//! 于是历史必须落到盘上，且**按文件存**（钥匙的形状在 [`crate::runtime::history`]）。
//!
//! # 为什么是"整本读-改-写"而不是"追加一条"
//!
//! 一本账通常只有几十条、几 KB：整份读-改-写（走 [`crate::fsx::atomic::atomic_write_json`]）
//! 比"追加"简单得多 —— 追加要处理"撤销把上一条标成 undone"这类**改已有条目**的情形，
//! 追加模型反而要再造一层更新协议。整份写还有个白捡的好处：盘上永远是一份自洽的账，
//! 不存在"半条"。
//!
//! # 只认两条线的身份
//!
//! `origin` 只有 `local`（用户线，身份 = 相对用户根的路径）与 `preset`（云端线，
//! 身份 = 文件名）两种 —— 与参数弹窗正在编辑的目标同源。别的取值一律 `INVALID_ARGUMENT`
//! 照实报（不静默当成某一种，猜错了就是把 A 的历史接到 B 上）。

use postprocess::diag::messages;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::AppHandle;

use crate::error::AppError;
use crate::runtime::history::{self, HistoryEntry, HistoryFile, HistoryItem, HISTORY_SCHEMA};

/// 一条改动里的一项（与前端 `HistoryItem` 同一套字面量）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItemDto {
    pub key: String,
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(default)]
    pub tab: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub unit: Option<String>,
}

/// 一条改动（与前端 `HistoryEntry` 同一套字面量）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryDto {
    pub id: i64,
    pub no: i64,
    pub action: String,
    /// 发生那一刻（UTC epoch 秒）。**落盘之后才有它** —— 内存日志时代不需要
    #[serde(default)]
    pub ts_unix: i64,
    pub state: String,
    #[serde(default)]
    pub batch: Option<u32>,
    /// 这一条是「云端更新」落下来的（2026-10-20）—— 界面挂「更新」徽章。
    /// 旧前端 / 旧账本没有这一格：`default` 让它们照读，按普通改值处理。
    #[serde(default)]
    pub from_sync: bool,
    pub items: Vec<HistoryItemDto>,
}

/// 一份文件的整本历史（读那一侧的返回值）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetHistoryDto {
    pub schema: u32,
    /// 这一份保存过几次（批号的水位）
    pub batches: u32,
    pub entries: Vec<HistoryEntryDto>,
    /// 每个参数「上一次保存之前的值」（2026-10-21）。旧账本没有这一格：`default` 按空读。
    #[serde(default)]
    pub prev_saved: BTreeMap<String, String>,
}

impl From<HistoryItem> for HistoryItemDto {
    fn from(v: HistoryItem) -> Self {
        Self {
            key: v.key,
            from: v.from,
            to: v.to,
            label: v.label,
            tab: v.tab,
            section: v.section,
            unit: v.unit,
        }
    }
}

impl From<HistoryItemDto> for HistoryItem {
    fn from(v: HistoryItemDto) -> Self {
        Self {
            key: v.key,
            from: v.from,
            to: v.to,
            label: v.label,
            tab: v.tab,
            section: v.section,
            unit: v.unit,
        }
    }
}

impl From<HistoryEntry> for HistoryEntryDto {
    fn from(v: HistoryEntry) -> Self {
        Self {
            id: v.id,
            no: v.no,
            action: v.action,
            ts_unix: v.ts_unix,
            state: v.state,
            batch: v.batch,
            from_sync: v.from_sync,
            items: v.items.into_iter().map(HistoryItemDto::from).collect(),
        }
    }
}

impl From<HistoryEntryDto> for HistoryEntry {
    fn from(v: HistoryEntryDto) -> Self {
        Self {
            id: v.id,
            no: v.no,
            action: v.action,
            ts_unix: v.ts_unix,
            state: v.state,
            batch: v.batch,
            from_sync: v.from_sync,
            items: v.items.into_iter().map(HistoryItem::from).collect(),
        }
    }
}

/// 认身份：`origin` 只许 `local` / `preset`，身份不许空。
///
/// ★ 不静默猜 —— 猜错就是把一份文件的历史接到另一份上（用户目录里同名文件很常见）。
fn key_for(origin: &str, id: &str) -> Result<String, AppError> {
    let origin = origin.trim();
    let id = id.trim();
    if origin != "local" && origin != "preset" {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.history.082",
            &[("origin", origin)],
        )));
    }
    if id.is_empty() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.history.083",
            &[],
        )));
    }
    Ok(history::key_of(origin, id))
}

/// 读一本。**没写过 = 空本**（界面照常显示"还没有改动记录"，不是错误）
#[tauri::command]
pub fn get_preset_history(
    app: AppHandle,
    origin: String,
    id: String,
) -> Result<PresetHistoryDto, AppError> {
    crate::ipc::traced("getPresetHistory", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let key = key_for(&origin, &id)?;
        let file = history::read(&root, &key)?;
        Ok(PresetHistoryDto {
            schema: file.schema,
            batches: file.batches,
            prev_saved: file.prev_saved,
            entries: file
                .entries
                .into_iter()
                .map(HistoryEntryDto::from)
                .collect(),
        })
    })
}

/// **整本写回**（界面改一条 / 撤销一条 / 保存一次之后都走这里）。
///
/// 前端给的是它眼里**整本**账（含 `batches` 水位），后端照写 ——
/// 于是"撤销把上一条标成 undone"这类改已有条目的动作也不需要额外协议。
#[tauri::command]
pub fn put_preset_history(
    app: AppHandle,
    origin: String,
    id: String,
    batches: u32,
    entries: Vec<HistoryEntryDto>,
    prev_saved: BTreeMap<String, String>,
) -> Result<(), AppError> {
    crate::ipc::traced("putPresetHistory", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let key = key_for(&origin, &id)?;
        history::write(
            &root,
            &key,
            &HistoryFile {
                schema: HISTORY_SCHEMA,
                batches,
                prev_saved,
                entries: entries.into_iter().map(HistoryEntry::from).collect(),
            },
        )
    })
}

/// **清空一本**（抽屉右上那颗「清空历史」；二次确认在前端做）。
///
/// 与"写一本空的"是同一件事，单独开一条命令是因为它要留一条**看得见的日志**：
/// 谁清的、清的哪一份 —— 这是不可撤销的动作。
#[tauri::command]
pub fn clear_preset_history(app: AppHandle, origin: String, id: String) -> Result<(), AppError> {
    crate::ipc::traced("clearPresetHistory", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let key = key_for(&origin, &id)?;
        history::clear(&root, &key)?;
        tracing::info!(origin = %origin, id = %id, "这一份的修改历史被清掉了");
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `origin` 只认两条线 —— 别的取值照实报，不猜（猜错 = 把 A 的历史接到 B 上）
    #[test]
    fn only_the_two_lines_are_recognized() {
        assert!(key_for("local", "presets-local/A1.toml").is_ok());
        assert!(key_for("preset", "A1-standard.toml").is_ok());
        assert!(
            key_for("cloud", "A1-standard.toml").is_err(),
            "不认的线要报错，不许静默当某一种"
        );
        assert!(key_for("preset", "  ").is_err(), "空身份要报错");
    }

    /// DTO ↔ 账本模型来回一趟不丢字段（`tsUnix` 与 `batch` 是持久化之后才有的那两个）
    #[test]
    fn a_round_trip_keeps_the_persisted_fields() {
        let dto = HistoryEntryDto {
            id: 7,
            no: 3,
            action: "改 Y 轴偏移".to_owned(),
            ts_unix: 1_700_000_000,
            state: "saved".to_owned(),
            batch: Some(2),
            from_sync: true,
            items: vec![HistoryItemDto {
                key: "toolhead.offset_y".to_owned(),
                from: "22.4".to_owned(),
                to: "1.23".to_owned(),
                label: "Y 轴偏移".to_owned(),
                tab: "偏移".to_owned(),
                section: "空间偏移".to_owned(),
                unit: Some("mm".to_owned()),
            }],
        };
        let entry: HistoryEntry = dto.clone().into();
        assert_eq!(entry.ts_unix, 1_700_000_000);
        assert_eq!(entry.batch, Some(2));
        let back: HistoryEntryDto = entry.into();
        assert_eq!(back.ts_unix, dto.ts_unix);
        assert_eq!(back.batch, dto.batch);
        assert_eq!(back.items[0].unit.as_deref(), Some("mm"));
    }
}
