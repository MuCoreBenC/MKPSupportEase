//! 开发源数据的读写底座。
//!
//! 第三版把它**削到只剩底座**：原来那些带类型的 registry / machine / version 读写全部删掉了 ——
//! 字段定义改由 [`crate::workbench::upstream`] 从 `mkpse-presets` 读（doc §2），
//! 配方本的形状与规则归 [`crate::workbench::domain`]（doc §1 第二条铁律）。
//! 留在这里的只有四样与"业务是什么"无关的东西：
//!
//! 1. **写盘的唯一出口**：一律复用 [`crate::fsx::atomic`]，不在这里另造一个。
//!    仓库里已经有那一个出口，而 `clippy.toml` 把 `std::fs::write` 与 `File::create`
//!    列进了 `disallowed-methods` —— 再写一份不只是重复，是把那道机器判据绕开。
//! 2. **id 白名单**：这些 id 会直接进文件名，所以用白名单而不是"禁止 `..`"。
//! 3. **通用 JSON 文档 IO**：调用方给形状，这里只管读写与错误分类。
//! 4. **草稿 / 快照 / 回收站的落盘位置**：位置与命名归这里，**内容的形状归 domain**。
//!
//! 刻意**不带任何具体业务结构**（没有 `Draft` / `Snapshot` / `Version`）：第二版就是因为
//! 底座认识业务结构，一改模型就要跟着改一圈。这一版底座只认识 `serde_json::Value`
//! 与调用方给的泛型，Task 6 的 `domain::patch` 再把类型盖上去。
//!
//! 另一条硬规矩留着：**首次启动只建目录，一个配方都不写**（doc §7）。工作台没有
//! seed 配方，所以工作台升级永远不可能"用默认值覆盖你的开发成果"。

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::{atomic_write, atomic_write_json};
use crate::fsx::paths::resolve_in;
use crate::workbench::clock;
use crate::workbench::paths;

/// 开发源数据根下的子目录。**引用 [`paths::WORKBENCH_DIRS`] 而不是再抄一遍** ——
/// 同一份清单写在两处，迟早有一天只改了其中一处
use paths::WORKBENCH_DIRS as SUBDIRS;

/// 首次启动的报告。界面据此决定要不要提示"这里还没有开发数据"
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapReport {
    /// 仓库里有没有机型配方。为 false 时界面提示，但**不会替你造一个**
    pub has_machines: bool,
}

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    /// 整本草稿的位置。**只存"改了什么"，不存整本**（doc §4.3）：
    /// 存整本的话，上游数据一变，草稿就会把旧值糊回去
    pub const DRAFT_REL: &'static str = ".draft/book.json";
    /// 界面状态（折叠 / 页签 / 勾选）。本机状态，不入库
    pub const UI_REL: &'static str = ".draft/ui.json";

    /// 真仓库里的那一份
    pub fn open() -> Result<Self, AppError> {
        Ok(Self {
            root: paths::workbench_root()?,
        })
    }

    /// 指定根。给单测用，也给"仓库不在默认位置"留口子
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /* ---------- 首次启动 ---------- */

    /// 建目录。**一个配方都不写**（doc §7）
    pub fn bootstrap(&self) -> Result<BootstrapReport, AppError> {
        for sub in SUBDIRS {
            let dir = self.root.join(sub);
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::io(format!("建不出目录：{}", dir.display())).with_detail(e.to_string())
            })?;
        }
        Ok(BootstrapReport {
            has_machines: !self.machine_ids()?.is_empty(),
        })
    }

    /* ---------- 通用 JSON 文档 ---------- */

    /// 读一份文档。**不存在返回 `None` 而不是错误** —— 菜单、套餐这类文件
    /// "还没建"是正常状态；解析不了才是 `CORRUPTED`
    pub fn read_doc<T: DeserializeOwned>(
        &self,
        rel: &str,
        what: &str,
    ) -> Result<Option<T>, AppError> {
        let p = resolve_in(&self.root, rel)?;
        if !p.exists() {
            return Ok(None);
        }
        read_json(&p, what).map(Some)
    }

    pub fn write_doc<T: Serialize>(&self, rel: &str, value: &T) -> Result<(), AppError> {
        atomic_write_json(&resolve_in(&self.root, rel)?, value)
    }

    /// 原样存放一份外部文件（BBS）。**不解析、不重排、不改一个字节**
    pub fn put_raw(&self, rel: &str, bytes: &[u8]) -> Result<PathBuf, AppError> {
        let p = resolve_in(&self.root, rel)?;
        atomic_write(&p, bytes)?;
        Ok(p)
    }

    pub fn remove(&self, rel: &str) -> Result<(), AppError> {
        let p = resolve_in(&self.root, rel)?;
        if p.exists() {
            std::fs::remove_file(&p)
                .map_err(|e| AppError::io(format!("删不掉 {rel}")).with_detail(e.to_string()))?;
        }
        Ok(())
    }

    /* ---------- 机型与版本的文件位置 ---------- */

    pub fn machine_ids(&self) -> Result<Vec<String>, AppError> {
        json_stems(&self.root.join("machines"))
    }

    pub fn version_ids(&self, machine_id: &str) -> Result<Vec<String>, AppError> {
        validate_id(machine_id, "机型 id")?;
        json_stems(&self.root.join("machines").join(machine_id).join("versions"))
    }

    pub fn machine_rel(&self, machine_id: &str) -> Result<String, AppError> {
        validate_id(machine_id, "机型 id")?;
        Ok(format!("machines/{machine_id}.json"))
    }

    pub fn version_rel(&self, machine_id: &str, version_id: &str) -> Result<String, AppError> {
        validate_id(machine_id, "机型 id")?;
        validate_id(version_id, "版本 id")?;
        Ok(format!("machines/{machine_id}/versions/{version_id}.json"))
    }

    pub fn snapshot_rel(&self, machine_id: &str, version_id: &str) -> Result<String, AppError> {
        validate_id(machine_id, "机型 id")?;
        validate_id(version_id, "版本 id")?;
        Ok(format!(".snapshots/{machine_id}__{version_id}.json"))
    }

    /* ---------- 回收站 ---------- */

    /// 移进回收站。文件名带时间戳前缀，同一个版本反复删也不会互相覆盖。
    ///
    /// **先写副本再删原件**：中途失败最坏是多一份回收站文件，不会两头都没有
    pub fn trash(&self, machine_id: &str, version_id: &str) -> Result<String, AppError> {
        let from_rel = self.version_rel(machine_id, version_id)?;
        let from = resolve_in(&self.root, &from_rel)?;
        let bytes = std::fs::read(&from).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::not_found(format!("版本 {machine_id}/{version_id} 不存在"))
            }
            _ => AppError::io("读不出要删的版本").with_detail(e.to_string()),
        })?;

        let stem = format!("{}__{}__{}", clock::now_stamp(), machine_id, version_id);
        self.put_raw(&format!(".trash/{stem}.json"), &bytes)?;
        self.remove(&from_rel)?;
        Ok(stem)
    }

    /// 回收站清单，新的在前
    pub fn trash_entries(&self) -> Result<Vec<TrashEntry>, AppError> {
        let mut out = Vec::new();
        for stem in json_stems(&self.root.join(".trash"))? {
            let parts: Vec<&str> = stem.splitn(3, "__").collect();
            if parts.len() != 3 {
                tracing::warn!(file = %stem, "回收站里有名字对不上格式的文件，跳过");
                continue;
            }
            out.push(TrashEntry {
                file: stem.clone(),
                deleted_stamp: parts[0].to_owned(),
                machine_id: parts[1].to_owned(),
                version_id: parts[2].to_owned(),
            });
        }
        out.sort_by(|a, b| b.deleted_stamp.cmp(&a.deleted_stamp));
        Ok(out)
    }

    /// 从回收站取回字节。**放回哪里由调用方决定** —— 目标位置已有同名版本时
    /// 该不该覆盖是业务判断，底座不替它定
    pub fn read_trash(&self, file: &str) -> Result<Vec<u8>, AppError> {
        validate_trash_stem(file)?;
        let p = resolve_in(&self.root, &format!(".trash/{file}.json"))?;
        std::fs::read(&p)
            .map_err(|e| AppError::io("读不出回收站里那一份").with_detail(e.to_string()))
    }

    pub fn purge_trash(&self, file: &str) -> Result<(), AppError> {
        validate_trash_stem(file)?;
        self.remove(&format!(".trash/{file}.json"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    /// 不带扩展名的文件名。还原与彻底删除都用它定位
    pub file: String,
    pub deleted_stamp: String,
    pub machine_id: String,
    pub version_id: String,
}

/// id 白名单。
///
/// 为什么是白名单而不是"禁止 `..`"：黑名单要穷举所有会出问题的写法（分隔符、保留名、
/// 前后空格、Unicode 同形字…），而这些 id 会直接进文件名。允许的字符集这么窄不碍事 ——
/// 它们是内部标识，显示名另有字段。
///
/// **点号在白名单里是照真数据放的，不是随手加的**：上游有一个版本 id 叫 `FASTV3.3`
/// （A1 与 A1_MINI 各一个），白名单里少个点就会让这两台机型的第三个版本整个读不出来。
/// 代价是 `.` / `..` 的每个字符都合法，所以它们要单独判掉。
pub fn validate_id(id: &str, what: &str) -> Result<(), AppError> {
    if id.is_empty() {
        return Err(AppError::invalid_argument(format!("{what}不能为空")));
    }
    if id.len() > 64 {
        return Err(AppError::invalid_argument(format!("{what}太长（上限 64）")));
    }
    if id == "." || id == ".." {
        return Err(AppError::invalid_argument(format!("{what}不能是 . 或 ..")));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(
            AppError::invalid_argument(format!("{what}只允许字母、数字、- _ 和 ."))
                .with_detail(format!("收到：{id}")),
        );
    }
    Ok(())
}

/// 回收站文件名：`<时间戳>__<机型>__<版本>`
fn validate_trash_stem(stem: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = stem.splitn(3, "__").collect();
    if parts.len() != 3 {
        return Err(
            AppError::invalid_argument("回收站文件名格式不对").with_detail(format!("收到：{stem}"))
        );
    }
    for p in parts {
        validate_id(p, "回收站文件名的分段")?;
    }
    Ok(())
}

/// 目录下所有 `.json` 的文件名（不含扩展名），已排序。
/// **目录不存在返回空而不是错误** —— 首次启动时它们本来就还没建
fn json_stems(dir: &Path) -> Result<Vec<String>, AppError> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            out.push(stem.to_owned());
        }
    }
    out.sort_unstable();
    Ok(out)
}

pub fn read_json<T: DeserializeOwned>(path: &Path, what: &str) -> Result<T, AppError> {
    let text = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            AppError::not_found(format!("{what}不存在")).with_detail(path.display().to_string())
        }
        _ => {
            AppError::io(format!("{what}读不出来")).with_detail(format!("{} / {e}", path.display()))
        }
    })?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::corrupted(format!("{what}解析不了"))
            .with_detail(format!("{} / {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        (d, s)
    }

    /// 首次启动：目录建齐、**一个文件都没写**（doc §7 的那条）
    #[test]
    fn bootstrap_creates_dirs_and_writes_nothing() {
        let (d, s) = store();
        for sub in SUBDIRS {
            assert!(d.path().join(sub).is_dir(), "{sub} 没建出来");
        }
        let files = walk_files(d.path());
        assert!(files.is_empty(), "不该写文件，却写了：{files:?}");
        assert!(!s.bootstrap().unwrap().has_machines);
    }

    /// 目录不存在时枚举返回空而不是错误 —— 首次启动时它们本来还没建
    #[test]
    fn enumerating_missing_dir_is_empty_not_error() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        assert_eq!(s.machine_ids().unwrap().len(), 0);
        assert_eq!(s.version_ids("A1").unwrap().len(), 0);
    }

    #[test]
    fn doc_roundtrip_and_missing_is_none() {
        let (_d, s) = store();
        assert!(s
            .read_doc::<serde_json::Value>("catalog.json", "菜单")
            .unwrap()
            .is_none());
        s.write_doc("catalog.json", &serde_json::json!({"a": 1}))
            .unwrap();
        let back: serde_json::Value = s.read_doc("catalog.json", "菜单").unwrap().unwrap();
        assert_eq!(back["a"], 1);
    }

    /// 坏 JSON 必须是 CORRUPTED，不能静默当空
    #[test]
    fn broken_json_is_corrupted() {
        let (_d, s) = store();
        s.put_raw("catalog.json", b"{ not json").unwrap();
        let e = s
            .read_doc::<serde_json::Value>("catalog.json", "菜单")
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 原样存放：字节一个都不许改
    #[test]
    fn put_raw_is_byte_for_byte() {
        let (d, s) = store();
        let raw = "{\n  \"drink\": \"进口\"\n}\n".as_bytes();
        s.put_raw("bbs/BBS-01.json", raw).unwrap();
        assert_eq!(
            std::fs::read(d.path().join("bbs/BBS-01.json")).unwrap(),
            raw
        );
    }

    /// **上游真实 id 必须全部通得过白名单**。
    /// `FASTV3.3` 带点，白名单里少个点就会让 A1 与 A1_MINI 的第三个版本整个读不出来
    #[test]
    fn real_upstream_ids_pass_the_whitelist() {
        for id in ["A1", "A1_MINI", "A2L", "P1S", "P2S", "X1C"] {
            validate_id(id, "机型 id").unwrap_or_else(|e| panic!("{id} 被拒了：{}", e.message));
        }
        for id in ["STANDARD", "FAST", "FASTV3.3", "LITE"] {
            validate_id(id, "版本 id").unwrap_or_else(|e| panic!("{id} 被拒了：{}", e.message));
        }
    }

    /// 能穿越或落到目录外的写法一律拒绝。
    /// `.` 与 `..` 要单独判 —— 它们的每个字符都在白名单里
    #[test]
    fn rejects_ids_that_could_escape() {
        for bad in ["", ".", "..", "a/b", "a\\b", "a b", "a:b", "a*b"] {
            assert!(
                validate_id(bad, "id").is_err(),
                "{bad:?} 该被拒绝，它能落到目录外或撞上保留名"
            );
        }
    }

    /// 删 = 进回收站；字节原样保留；清单能解析出机型与版本
    #[test]
    fn trash_keeps_bytes_and_parses_entry() {
        let (_d, s) = store();
        let rel = s.version_rel("A1", "FASTV3.3").unwrap();
        s.put_raw(&rel, b"{\"overrides\":{}}").unwrap();

        let stem = s.trash("A1", "FASTV3.3").unwrap();
        assert!(s
            .read_doc::<serde_json::Value>(&rel, "版本")
            .unwrap()
            .is_none());

        let entries = s.trash_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].machine_id, "A1");
        assert_eq!(entries[0].version_id, "FASTV3.3");
        assert_eq!(s.read_trash(&stem).unwrap(), b"{\"overrides\":{}}");

        s.purge_trash(&stem).unwrap();
        assert_eq!(s.trash_entries().unwrap().len(), 0);
    }

    #[test]
    fn trashing_missing_version_is_not_found() {
        let (_d, s) = store();
        let e = s.trash("A1", "STANDARD").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /// 回收站里名字对不上格式的文件被跳过，而不是让整份清单失败
    #[test]
    fn malformed_trash_file_is_skipped() {
        let (_d, s) = store();
        s.put_raw(".trash/whatever.json", b"{}").unwrap();
        assert_eq!(s.trash_entries().unwrap().len(), 0);
    }

    fn walk_files(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk_files(&p));
            } else {
                out.push(p);
            }
        }
        out
    }
}
