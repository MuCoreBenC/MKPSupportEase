//! 开发源数据的读写。
//!
//! **写盘一律走 [`crate::fsx::atomic`]**，不在这里另造一个 `atomic_write`。仓库里已经有
//! 那一个唯一出口，而且 `clippy.toml` 把 `std::fs::write` 与 `File::create` 列进了
//! `disallowed-methods` —— 再写一份不只是重复，是把那道机器判据绕开。
//! （tasks.md 3.1 原文写的是"实现 atomic_write"；这里改成复用，是因为它已经存在。）
//!
//! [`Store`] 带着根路径而不是每个函数都去查一次：单测里给它一个 tempdir 就能跑，
//! 不需要 `AppHandle`，也不会去碰真仓库里的数据。
//!
//! 几条硬规矩：
//! - **首次启动只建目录、只写全局配置（字段定义/回退表），一个配方都不写**（doc §7）。
//!   工作台没有 seed 配方，所以工作台升级永远不可能"用默认值覆盖你的开发成果"。
//! - id 只允许 `[A-Za-z0-9_-]`。这既是文件名安全，也让 `.` 与 `..` 在进 path 之前就被拒。
//! - 保存时比对 `baseHash`：对不上就拒绝，不静默覆盖别处的修改。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::{atomic_write, atomic_write_json};
use crate::fsx::paths::resolve_in;
use crate::workbench::clock;
use crate::workbench::model::{
    builtin_fallback, builtin_registry, Fallback, Machine, Params, Registry, Version,
};
use crate::workbench::paths;

const REGISTRY_FILE: &str = "registry.json";
const FALLBACK_FILE: &str = "fallback.json";

/// 未保存的编辑。**不入库**（.gitignore 里排掉了 `workbench/.draft/`）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    /// 打开这份配方时它的 hash。重开时对不上就说明配方在别处被改过
    pub base_hash: String,
    pub overrides: Params,
    pub saved_at: String,
}

/// 上次生成失败的记录。**产物不会因为失败被动过**，这里只记"为什么没成"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenFailure {
    pub at: String,
    pub reason: String,
}

/// 上次**成功**生成时的现场。状态判定与"恢复到上次能出货的那份"都靠它
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// 当时用的有效配方 hash —— 这是状态判定唯一的判据
    pub recipe_hash: String,
    pub generated_at: String,
    /// 当时的版本覆盖。恢复时写回的就是它
    pub overrides: Params,
    /// 当时的机型基底。**恢复时不写回** —— 基底是多个版本共享的，
    /// 为了一个版本回滚它会连带改掉别的版本
    pub base: Params,
    /// 产物相对发布目录的路径
    pub output_rel: String,
    pub output_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<GenFailure>,
}

/// 首次启动做了什么。界面据此决定要不要提示"这里还没有开发数据"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapReport {
    pub wrote_registry: bool,
    pub wrote_fallback: bool,
    /// 仓库里有没有机型。为 false 时界面提示，但**不会替你造一个**
    pub has_machines: bool,
}

pub struct Store {
    pub root: PathBuf,
}

impl Store {
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

    /// 建目录 + 补全局配置。**一个配方都不写**
    pub fn bootstrap(&self) -> Result<BootstrapReport, AppError> {
        for sub in ["machines", "bbs", "capability", ".draft", ".trash", ".snapshots"] {
            let dir = self.root.join(sub);
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::io(format!("建不出目录：{}", dir.display())).with_detail(e.to_string())
            })?;
        }

        let reg_path = self.root.join(REGISTRY_FILE);
        let wrote_registry = !reg_path.exists();
        if wrote_registry {
            atomic_write_json(&reg_path, &builtin_registry())?;
            tracing::info!("字段定义不存在，已写出内置的最小字段集");
        }

        let fb_path = self.root.join(FALLBACK_FILE);
        let wrote_fallback = !fb_path.exists();
        if wrote_fallback {
            atomic_write_json(&fb_path, &builtin_fallback())?;
        }

        let has_machines = !self.machine_ids().unwrap_or_default().is_empty();

        Ok(BootstrapReport {
            wrote_registry,
            wrote_fallback,
            has_machines,
        })
    }

    /* ---------- 全局配置（只读） ---------- */

    /// 字段定义。解析不了就是 `CORRUPTED` —— 调用方据此进只读模式，
    /// **不允许在坏的字段定义上生成产物**
    pub fn registry(&self) -> Result<Registry, AppError> {
        read_json(&self.root.join(REGISTRY_FILE), "字段定义")
    }

    pub fn fallback(&self) -> Result<Fallback, AppError> {
        read_json(&self.root.join(FALLBACK_FILE), "回退登记表")
    }

    /* ---------- 机型与版本 ---------- */

    pub fn machine_ids(&self) -> Result<Vec<String>, AppError> {
        let dir = self.root.join("machines");
        let mut ids = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(ids);
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_owned());
                }
            }
        }
        ids.sort_unstable();
        Ok(ids)
    }

    pub fn machine(&self, id: &str) -> Result<Machine, AppError> {
        validate_id(id, "机型 id")?;
        read_json(&self.machine_path(id)?, &format!("机型 {id}"))
    }

    pub fn version_ids(&self, machine_id: &str) -> Result<Vec<String>, AppError> {
        validate_id(machine_id, "机型 id")?;
        let dir = self.root.join("machines").join(machine_id).join("versions");
        let mut ids = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(ids);
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_owned());
                }
            }
        }
        ids.sort_unstable();
        Ok(ids)
    }

    pub fn version(&self, machine_id: &str, version_id: &str) -> Result<Version, AppError> {
        let path = self.version_path(machine_id, version_id)?;
        read_json(&path, &format!("版本 {machine_id}/{version_id}"))
    }

    /// 写机型基底。**改基底只改基底** —— 这个函数拿不到任何版本，也拿不到 Registry 的可写引用
    pub fn save_machine(&self, m: &Machine) -> Result<(), AppError> {
        validate_id(&m.id, "机型 id")?;
        atomic_write_json(&self.machine_path(&m.id)?, m)
    }

    /// 写版本覆盖。同上，它碰不到机型基底与字段定义
    pub fn save_version(&self, v: &Version) -> Result<(), AppError> {
        let path = self.version_path(&v.machine_id, &v.id)?;
        atomic_write_json(&path, v)
    }

    /// 删掉版本文件本身。**改名/换机型时用** —— 那两个操作是"先写新的再删旧的"，
    /// 与"删除版本"（走回收站）是两件不同的事，所以刻意是两个方法
    pub fn remove_version_file(&self, machine_id: &str, version_id: &str) -> Result<(), AppError> {
        let p = self.version_path(machine_id, version_id)?;
        if p.exists() {
            std::fs::remove_file(&p)
                .map_err(|e| AppError::io("删不掉旧版本文件").with_detail(e.to_string()))?;
        }
        Ok(())
    }

    /// 生成快照跟着版本走。
    ///
    /// 不搬的后果很具体：改个名字，产物明明还对得上，状态却变成"从没生成过" ——
    /// 于是"只生成待更新项"会把它重新烤一遍，产物字节没变但时间戳变了，
    /// 用户端就会莫名要更新。这正是 doc §8 要避免的那件事
    pub fn rename_snapshot(
        &self,
        from_machine: &str,
        from_version: &str,
        to_machine: &str,
        to_version: &str,
    ) -> Result<(), AppError> {
        let from = self.snapshot_path(from_machine, from_version)?;
        if !from.exists() {
            return Ok(());
        }
        let snap: Snapshot = read_json(&from, "生成快照")?;
        atomic_write_json(&self.snapshot_path(to_machine, to_version)?, &snap)?;
        std::fs::remove_file(&from)
            .map_err(|e| AppError::io("删不掉旧快照").with_detail(e.to_string()))?;
        Ok(())
    }

    /// 删除版本 = 移进回收站。原文件名带时间戳前缀，同一个版本反复删也不会互相覆盖
    pub fn trash_version(&self, machine_id: &str, version_id: &str) -> Result<PathBuf, AppError> {
        let from = self.version_path(machine_id, version_id)?;
        let v: Version = read_json(&from, &format!("版本 {machine_id}/{version_id}"))?;
        let name = format!("{}__{}__{}.json", clock::now_stamp(), machine_id, version_id);
        let to = self.root.join(".trash").join(&name);
        // 先写副本再删原件：中途失败最坏是多一份回收站文件，不会两头都没有
        atomic_write_json(&to, &v)?;
        std::fs::remove_file(&from)
            .map_err(|e| AppError::io("删不掉原版本文件").with_detail(e.to_string()))?;
        self.clear_draft(machine_id, version_id)?;
        Ok(to)
    }

    /// 回收站清单，新的在前
    pub fn trash_entries(&self) -> Result<Vec<TrashEntry>, AppError> {
        let dir = self.root.join(".trash");
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(out);
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let parts: Vec<&str> = stem.splitn(3, "__").collect();
            if parts.len() != 3 {
                continue;
            }
            out.push(TrashEntry {
                file: stem.to_owned(),
                deleted_stamp: parts[0].to_owned(),
                machine_id: parts[1].to_owned(),
                version_id: parts[2].to_owned(),
            });
        }
        out.sort_by(|a, b| b.deleted_stamp.cmp(&a.deleted_stamp));
        Ok(out)
    }

    /// 从回收站还原。目标位置已有同名版本就拒绝 —— 让人先改名，不静默覆盖
    pub fn restore_from_trash(&self, file: &str) -> Result<Version, AppError> {
        validate_file_stem(file)?;
        let from = self.root.join(".trash").join(format!("{file}.json"));
        let v: Version = read_json(&from, "回收站里的版本")?;
        let to = self.version_path(&v.machine_id, &v.id)?;
        if to.exists() {
            return Err(AppError::invalid_argument(format!(
                "{}/{} 已经存在，还原会覆盖它",
                v.machine_id, v.id
            )));
        }
        atomic_write_json(&to, &v)?;
        std::fs::remove_file(&from)
            .map_err(|e| AppError::io("清不掉回收站里的那一份").with_detail(e.to_string()))?;
        Ok(v)
    }

    /// 彻底删除。调用方负责二次确认
    pub fn purge_from_trash(&self, file: &str) -> Result<(), AppError> {
        validate_file_stem(file)?;
        let p = self.root.join(".trash").join(format!("{file}.json"));
        std::fs::remove_file(&p)
            .map_err(|e| AppError::io("删不掉回收站里的那一份").with_detail(e.to_string()))
    }

    /* ---------- 草稿 ---------- */

    pub fn draft(&self, machine_id: &str, version_id: &str) -> Result<Option<Draft>, AppError> {
        let p = self.draft_path(machine_id, version_id)?;
        if !p.exists() {
            return Ok(None);
        }
        read_json(&p, "草稿").map(Some)
    }

    pub fn save_draft(&self, machine_id: &str, version_id: &str, d: &Draft) -> Result<(), AppError> {
        atomic_write_json(&self.draft_path(machine_id, version_id)?, d)
    }

    pub fn clear_draft(&self, machine_id: &str, version_id: &str) -> Result<(), AppError> {
        let p = self.draft_path(machine_id, version_id)?;
        if p.exists() {
            std::fs::remove_file(&p)
                .map_err(|e| AppError::io("删不掉草稿").with_detail(e.to_string()))?;
        }
        Ok(())
    }

    /* ---------- 机型基底的草稿 ---------- */

    /* 基底编辑同样是"未保存的编辑"，也该关掉再开还在那儿。
    文件名刻意用 `#base__<机型>.json` 打头：`#` 不在 id 的白名单里，所以
    **不可能**有哪个版本的草稿文件跟它撞名 —— 不需要把 "base" 列成保留版本 id。 */

    pub fn base_draft(&self, machine_id: &str) -> Result<Option<Draft>, AppError> {
        let p = self.base_draft_path(machine_id)?;
        if !p.exists() {
            return Ok(None);
        }
        read_json(&p, "基底草稿").map(Some)
    }

    pub fn save_base_draft(&self, machine_id: &str, d: &Draft) -> Result<(), AppError> {
        atomic_write_json(&self.base_draft_path(machine_id)?, d)
    }

    pub fn clear_base_draft(&self, machine_id: &str) -> Result<(), AppError> {
        let p = self.base_draft_path(machine_id)?;
        if p.exists() {
            std::fs::remove_file(&p)
                .map_err(|e| AppError::io("删不掉基底草稿").with_detail(e.to_string()))?;
        }
        Ok(())
    }

    /// 界面状态（矩阵的筛选与展开）。与草稿同一个目录 —— 都是本机状态，都不入库
    pub fn view_state(&self, name: &str) -> Result<Option<serde_json::Value>, AppError> {
        validate_id(name, "视图名")?;
        let p = self.root.join(".draft").join(format!("{name}.json"));
        if !p.exists() {
            return Ok(None);
        }
        read_json(&p, "视图状态").map(Some)
    }

    pub fn save_view_state(&self, name: &str, v: &serde_json::Value) -> Result<(), AppError> {
        validate_id(name, "视图名")?;
        atomic_write_json(&self.root.join(".draft").join(format!("{name}.json")), v)
    }

    /* ---------- 生成快照 ---------- */

    pub fn snapshot(
        &self,
        machine_id: &str,
        version_id: &str,
    ) -> Result<Option<Snapshot>, AppError> {
        let p = self.snapshot_path(machine_id, version_id)?;
        if !p.exists() {
            return Ok(None);
        }
        read_json(&p, "生成快照").map(Some)
    }

    pub fn save_snapshot(
        &self,
        machine_id: &str,
        version_id: &str,
        s: &Snapshot,
    ) -> Result<(), AppError> {
        atomic_write_json(&self.snapshot_path(machine_id, version_id)?, s)
    }

    /* ---------- 通用 JSON 文件（菜单 / 套餐） ---------- */

    pub fn read_doc<T: for<'de> Deserialize<'de>>(
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

    /* ---------- 路径 ---------- */

    fn machine_path(&self, id: &str) -> Result<PathBuf, AppError> {
        validate_id(id, "机型 id")?;
        Ok(self.root.join("machines").join(format!("{id}.json")))
    }

    fn version_path(&self, machine_id: &str, version_id: &str) -> Result<PathBuf, AppError> {
        validate_id(machine_id, "机型 id")?;
        validate_id(version_id, "版本 id")?;
        Ok(self
            .root
            .join("machines")
            .join(machine_id)
            .join("versions")
            .join(format!("{version_id}.json")))
    }

    fn draft_path(&self, machine_id: &str, version_id: &str) -> Result<PathBuf, AppError> {
        validate_id(machine_id, "机型 id")?;
        validate_id(version_id, "版本 id")?;
        Ok(self
            .root
            .join(".draft")
            .join(format!("{machine_id}__{version_id}.json")))
    }

    /// 基底草稿。`#` 开头保证与任何版本草稿都不会撞名（id 白名单里没有 `#`）
    fn base_draft_path(&self, machine_id: &str) -> Result<PathBuf, AppError> {
        validate_id(machine_id, "机型 id")?;
        Ok(self
            .root
            .join(".draft")
            .join(format!("#base__{machine_id}.json")))
    }

    fn snapshot_path(&self, machine_id: &str, version_id: &str) -> Result<PathBuf, AppError> {
        validate_id(machine_id, "机型 id")?;
        validate_id(version_id, "版本 id")?;
        Ok(self
            .root
            .join(".snapshots")
            .join(format!("{machine_id}__{version_id}.json")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// 前后空格、Unicode 同形字…），而这些 id 会直接进文件名。允许的字符集这么窄不会碍事 ——
/// 它们是内部标识，显示名另有字段。
pub fn validate_id(id: &str, what: &str) -> Result<(), AppError> {
    if id.is_empty() {
        return Err(AppError::invalid_argument(format!("{what}不能为空")));
    }
    if id.len() > 64 {
        return Err(AppError::invalid_argument(format!("{what}太长（上限 64）")));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            AppError::invalid_argument(format!("{what}只允许字母、数字、- 和 _"))
                .with_detail(format!("收到：{id}")),
        );
    }
    Ok(())
}

/// 回收站文件名：`<时间戳>__<机型>__<版本>`
fn validate_file_stem(stem: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = stem.splitn(3, "__").collect();
    if parts.len() != 3 {
        return Err(AppError::invalid_argument("回收站文件名格式不对")
            .with_detail(format!("收到：{stem}")));
    }
    for p in parts {
        validate_id(p, "回收站文件名的分段")?;
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, what: &str) -> Result<T, AppError> {
    let text = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => AppError::not_found(format!("{what}不存在"))
            .with_detail(path.display().to_string()),
        _ => AppError::io(format!("{what}读不出来")).with_detail(format!("{} / {e}", path.display())),
    })?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::corrupted(format!("{what}解析不了"))
            .with_detail(format!("{} / {e}", path.display()))
    })
}

/// 空参数表。写代码时比 `BTreeMap::new()` 读起来清楚一点
pub fn empty_params() -> Params {
    BTreeMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::model::BbsBinding;

    fn store() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        (d, s)
    }

    /// 首次启动：目录建齐、全局配置补上、**一个配方都没写**
    #[test]
    fn bootstrap_creates_dirs_and_global_config_only() {
        let (d, s) = store();
        let report = s.bootstrap().unwrap();

        assert!(report.wrote_registry);
        assert!(report.wrote_fallback);
        assert!(!report.has_machines, "空仓库不该有机型");

        for sub in ["machines", "bbs", "capability", ".draft", ".trash", ".snapshots"] {
            assert!(d.path().join(sub).is_dir(), "{sub} 没建出来");
        }
        // machines 下必须是空的 —— 这是 doc §7 的那条
        assert_eq!(s.machine_ids().unwrap().len(), 0);
    }

    /// 已有字段定义时不覆盖 —— 工作台升级不该动开发成果
    #[test]
    fn bootstrap_does_not_overwrite_existing_registry() {
        let (d, s) = store();
        s.bootstrap().unwrap();
        let p = d.path().join(REGISTRY_FILE);
        let before = std::fs::read_to_string(&p).unwrap();

        // 手动改一笔，再跑一次 bootstrap
        let mut reg = s.registry().unwrap();
        reg.fields.truncate(1);
        atomic_write_json(&p, &reg).unwrap();
        let edited = std::fs::read_to_string(&p).unwrap();
        assert_ne!(before, edited);

        let report = s.bootstrap().unwrap();
        assert!(!report.wrote_registry, "不该再写一遍");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), edited, "被覆盖了");
    }

    #[test]
    fn machine_and_version_roundtrip() {
        let (_d, s) = store();
        s.bootstrap().unwrap();

        let mut base = empty_params();
        base.insert("toolhead.z_offset".into(), serde_json::json!(0.1));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base,
            default_bbs: vec![],
        })
        .unwrap();

        let mut overrides = empty_params();
        overrides.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&Version {
            id: "quickswap".into(),
            display_name: "快拆版".into(),
            machine_id: "A1".into(),
            overrides,
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        assert_eq!(s.machine_ids().unwrap(), vec!["A1"]);
        assert_eq!(s.version_ids("A1").unwrap(), vec!["quickswap"]);
        let v = s.version("A1", "quickswap").unwrap();
        assert_eq!(v.overrides["toolhead.z_offset"], serde_json::json!(0.15));
    }

    /// 坏 JSON 必须是 CORRUPTED，不能是"读不出来"也不能静默当空表
    #[test]
    fn broken_registry_is_corrupted_not_empty() {
        let (d, s) = store();
        s.bootstrap().unwrap();
        atomic_write(&d.path().join(REGISTRY_FILE), b"{ not json").unwrap();
        let e = s.registry().unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    #[test]
    fn rejects_ids_that_could_escape_the_root() {
        let (_d, s) = store();
        for bad in ["..", "a/b", "a\\b", "", "a b", "a.json"] {
            assert!(
                s.machine(bad).is_err(),
                "{bad:?} 该被拒绝"
            );
        }
    }

    /// 删除 = 进回收站 + 能还原；且草稿要跟着清掉
    #[test]
    fn trash_then_restore() {
        let (_d, s) = store();
        s.bootstrap().unwrap();
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base: empty_params(),
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();
        s.save_draft(
            "A1",
            "std",
            &Draft {
                base_hash: "x".into(),
                overrides: empty_params(),
                saved_at: clock::now_iso8601(),
            },
        )
        .unwrap();

        s.trash_version("A1", "std").unwrap();
        assert!(s.version("A1", "std").is_err(), "原件还在");
        assert!(s.draft("A1", "std").unwrap().is_none(), "草稿没跟着清");

        let entries = s.trash_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].machine_id, "A1");
        assert_eq!(entries[0].version_id, "std");

        let back = s.restore_from_trash(&entries[0].file).unwrap();
        assert_eq!(back.id, "std");
        assert!(s.version("A1", "std").is_ok());
        assert_eq!(s.trash_entries().unwrap().len(), 0);
    }

    /// 还原时目标已存在：拒绝，不覆盖
    #[test]
    fn restore_refuses_to_overwrite() {
        let (_d, s) = store();
        s.bootstrap().unwrap();
        let v = Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        };
        s.save_version(&v).unwrap();
        s.trash_version("A1", "std").unwrap();
        s.save_version(&v).unwrap(); // 又建了一个同名的

        let entries = s.trash_entries().unwrap();
        let e = s.restore_from_trash(&entries[0].file).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// **再生入口，不是判据。** 没有 `MKPSE_WRITE_BUILTIN=1` 就直接返回。
    ///
    /// 用途：把内置的全局配置写进真仓库，让 `workbench/registry.json` 与
    /// `workbench/fallback.json` 能入库、能被 review、能被 diff。
    /// 用法：`MKPSE_WRITE_BUILTIN=1 cargo test --features workbench regenerate_builtin`
    ///
    /// 写成测试而不是单独的 bin：它需要的全部能力（Store、内置表、原子写）都在这里，
    /// 再开一个 bin target 只会多一条要维护的入口。
    #[test]
    fn regenerate_builtin_global_config() {
        if std::env::var_os("MKPSE_WRITE_BUILTIN").is_none() {
            return;
        }
        let s = Store::open().expect("打不开真仓库的工作台根");
        atomic_write_json(&s.root.join(REGISTRY_FILE), &builtin_registry()).unwrap();
        atomic_write_json(&s.root.join(FALLBACK_FILE), &builtin_fallback()).unwrap();
        eprintln!("已再生：{}", s.root.display());
    }

    /// 仓库里那两份全局配置必须与代码里的内置表**语义一致**。
    ///
    /// 为什么需要这条：bootstrap 只在文件缺失时才写，所以文件一旦入库，它和代码就是
    /// 两份声明。漂移是静默的 —— 界面按文件渲染、测试按代码断言，两边都"正常"。
    ///
    /// 比的是解析后的 JSON 而不是字节：缩进与键序不该让这条判据变成格式检查。
    /// **文件缺失一律 FAIL，不跳过** —— 静默跳过的门禁等于没有门禁。
    #[test]
    fn repo_global_config_matches_builtin() {
        let s = Store::open().expect("打不开真仓库的工作台根");

        let on_disk: serde_json::Value = serde_json::to_value(
            s.registry()
                .expect("workbench/registry.json 读不出来；要再生它见 regenerate_builtin_global_config"),
        )
        .unwrap();
        let in_code = serde_json::to_value(builtin_registry()).unwrap();
        assert_eq!(on_disk, in_code, "registry.json 与内置表漂了");

        let fb_disk: serde_json::Value = serde_json::to_value(
            s.fallback()
                .expect("workbench/fallback.json 读不出来；要再生它见 regenerate_builtin_global_config"),
        )
        .unwrap();
        assert_eq!(
            fb_disk,
            serde_json::to_value(builtin_fallback()).unwrap(),
            "fallback.json 与内置表漂了"
        );
    }

    #[test]
    fn draft_roundtrip_and_clear() {
        let (_d, s) = store();
        s.bootstrap().unwrap();
        assert!(s.draft("A1", "std").unwrap().is_none());

        let mut overrides = empty_params();
        overrides.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_draft(
            "A1",
            "std",
            &Draft {
                base_hash: "hash-1".into(),
                overrides,
                saved_at: clock::now_iso8601(),
            },
        )
        .unwrap();

        let d = s.draft("A1", "std").unwrap().unwrap();
        assert_eq!(d.base_hash, "hash-1");
        s.clear_draft("A1", "std").unwrap();
        assert!(s.draft("A1", "std").unwrap().is_none());
    }

    /// 草稿是**下次打开还在**的东西：换一个 Store 实例（等价于重开工作台）仍要读得到
    #[test]
    fn draft_survives_reopen() {
        let d = tempfile::tempdir().unwrap();
        {
            let s = Store::at(d.path());
            s.bootstrap().unwrap();
            let mut overrides = empty_params();
            overrides.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
            s.save_draft(
                "A1",
                "std",
                &Draft {
                    base_hash: "h".into(),
                    overrides,
                    saved_at: clock::now_iso8601(),
                },
            )
            .unwrap();
        }
        let again = Store::at(d.path());
        let got = again.draft("A1", "std").unwrap().expect("重开后草稿没了");
        assert_eq!(got.overrides["toolhead.z_offset"], serde_json::json!(0.15));
    }

    /// 基底草稿与版本草稿是两个槽位，**互不覆盖**。
    ///
    /// 这条防的是文件名撞车：基底草稿走 `#base__A1.json`，而 `#` 不在 id 白名单里，
    /// 所以任何版本 id 都造不出这个名字。把它写成断言，是因为哪天有人"顺手"把
    /// 基底草稿改成 `A1__base.json`，一个叫 base 的版本就会静默互相覆盖。
    #[test]
    fn base_draft_and_version_draft_do_not_collide() {
        let (_d, s) = store();
        s.bootstrap().unwrap();

        let mk = |v: f64| {
            let mut p = empty_params();
            p.insert("toolhead.z_offset".into(), serde_json::json!(v));
            Draft {
                base_hash: "h".into(),
                overrides: p,
                saved_at: clock::now_iso8601(),
            }
        };

        s.save_draft("A1", "base", &mk(1.0)).unwrap();
        s.save_base_draft("A1", &mk(2.0)).unwrap();

        assert_eq!(
            s.draft("A1", "base").unwrap().unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(1.0),
            "版本 base 的草稿被基底草稿盖了"
        );
        assert_eq!(
            s.base_draft("A1").unwrap().unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(2.0)
        );

        s.clear_base_draft("A1").unwrap();
        assert!(s.base_draft("A1").unwrap().is_none());
        assert!(s.draft("A1", "base").unwrap().is_some(), "清基底草稿连带清掉了版本草稿");
    }
}
