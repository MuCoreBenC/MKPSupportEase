//! **分支隔离**：每个分支一个文件夹（`presets-local/<folder>/`）。
//!
//! 为什么按目录分而不是靠状态判断：用户在 A 分支下载过、又去 B 分支试一下，两边
//! 同名文件必须能同时存在，而且**互不认领**。靠「来源记录不同」去解释"这份不算你有"
//! 只是在界面上绕圈；各放各的目录，盘上就没有歧义。
//!
//! # 名字怎么定
//!
//! 文件夹名 = 分支名，清洗掉路径分隔符（`feat/x` → `feat-x`）。两个仓库都有 `main`
//! 那种重名，用 `-2`、`-3` 往后排 —— 分配结果记在 `run/branch-dirs.json`，**同一个
//! 源身份每次拿到的都是同一个文件夹**，不会因为重启换了顺序。
//!
//! # 没有分支的源
//!
//! 自建目录源（ref = `-`）没有分支概念：返回 `None`，调用方保持原来的平铺布局。
//! 不编一个 `main` 出来 —— 那会让两个不同的目录源看起来是同一支。
//!
//! # 没有来源的文件
//!
//! 导入 / 手拖进来的文件不属于任何分支，统一放 [`UNFILED`]。这个目录名不参与
//! 分支分配（清洗时会避开它）。

use postprocess::diag::messages;

use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::error::AppError;

use super::source_identity::{SourceIdentity, NO_REF};

/// 没有来源分支的那一支（导入 / 手拖）。分支文件夹分配时避开这个名字。
pub const UNFILED: &str = "_local";

/// 分配结果住哪：内部根的 `run/`，与其他运行索引同一层。
const MAP_FILE: &str = "run/branch-dirs.json";

/// 文件夹名最长多少个字符。用户根下还要拼文件名，留足余量。
const MAX_FOLDER_CHARS: usize = 48;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Dirs {
    /// `平台#仓库@ref` → 文件夹名。**只增不改**：同一个身份不该换名字。
    #[serde(default)]
    by_key: BTreeMap<String, String>,
}

/// 当前身份该落哪个文件夹。`None` = 这个源没有分支概念，保持平铺。
pub fn folder_for(root: &Path, identity: &SourceIdentity) -> Result<Option<String>, AppError> {
    if identity.git_ref.is_empty() || identity.git_ref == NO_REF {
        return Ok(None);
    }
    let key = identity.key();
    let mut dirs = load(root);
    if let Some(dir) = dirs.by_key.get(&key) {
        return Ok(Some(dir.clone()));
    }
    let used: BTreeSet<&str> = dirs.by_key.values().map(String::as_str).collect();
    let dir = pick(clean(&identity.git_ref), &used);
    dirs.by_key.insert(key, dir.clone());
    save(root, &dirs)?;
    Ok(Some(dir))
}

/// 只读：这个身份之前分配过哪个文件夹（没有就是 `None`）。界面不写盘。
pub fn assigned(root: &Path, key: &str) -> Option<String> {
    load(root).by_key.get(key).cloned()
}

/// 已分配的全表（`平台#仓库@ref` → 文件夹）。
pub fn assignments(root: &Path) -> BTreeMap<String, String> {
    load(root).by_key
}

/// 分支名 → 目录名：路径分隔符与空白换成 `-`，避开 [`UNFILED`]，留一个非空名字。
fn clean(git_ref: &str) -> String {
    let mut out = String::with_capacity(git_ref.len());
    for ch in git_ref.chars() {
        if ch.is_whitespace() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        {
            out.push('-');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(|c| c == '.' || c == '-').to_owned();
    let mut name = if trimmed.is_empty() {
        "branch".to_owned()
    } else {
        trimmed
    };
    if name.chars().count() > MAX_FOLDER_CHARS {
        name = name.chars().take(MAX_FOLDER_CHARS).collect();
    }
    if name.eq_ignore_ascii_case(UNFILED) {
        name.push_str("-branch");
    }
    name
}

/// 撞名就往后排：`main`、`main-2`、`main-3`。
fn pick(base: String, used: &BTreeSet<&str>) -> String {
    if !used.contains(base.as_str()) {
        return base;
    }
    for n in 2..=999u32 {
        let candidate = format!("{base}-{n}");
        if !used.contains(candidate.as_str()) {
            return candidate;
        }
    }
    format!("{base}-{}", used.len() + 2)
}

fn path_of(root: &Path) -> std::path::PathBuf {
    root.join(MAP_FILE)
}

/// 读不出来（首次 / 被删）按空表处理：那份映射只是**分配记录**，丢了就按分支名重新
/// 分配一次；同名目录还在的，仍然落回同一个名字。
fn load(root: &Path) -> Dirs {
    let Ok(text) = std::fs::read_to_string(path_of(root)) else {
        return Dirs::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(root: &Path, dirs: &Dirs) -> Result<(), AppError> {
    let file = path_of(root);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::internal(messages::text("ui.runtime.branch_dir.1", &[]))
                .with_detail(e.to_string())
        })?;
    }
    let body = serde_json::to_vec_pretty(dirs).map_err(|e| {
        AppError::internal(messages::text("ui.runtime.branch_dir.1", &[]))
            .with_detail(e.to_string())
    })?;
    crate::fsx::atomic::atomic_write(&file, &body).map_err(|e| {
        AppError::internal(messages::text("ui.runtime.branch_dir.1", &[]))
            .with_detail(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("临时目录")
    }

    fn id(platform: &str, repo: &str, branch: &str) -> SourceIdentity {
        SourceIdentity {
            platform: platform.to_owned(),
            repo: repo.to_owned(),
            git_ref: branch.to_owned(),
        }
    }

    /// 名字就是分支名，带 `/` 的折成 `-`
    #[test]
    fn folder_is_the_branch_name() {
        let t = tmp();
        assert_eq!(
            folder_for(t.path(), &id("github", "o/r", "main")).unwrap(),
            Some("main".to_owned())
        );
        assert_eq!(
            folder_for(t.path(), &id("github", "o/r", "feat/pen")).unwrap(),
            Some("feat-pen".to_owned())
        );
    }

    /// 同一个身份每次都拿到同一个文件夹（不会因为重启换名字）
    #[test]
    fn the_same_identity_keeps_its_folder() {
        let t = tmp();
        let a = folder_for(t.path(), &id("github", "o/r", "main")).unwrap();
        let b = folder_for(t.path(), &id("github", "o/r", "main")).unwrap();
        assert_eq!(a, b);
    }

    /// 两个仓库的 `main` 不共用文件夹：第二个拿 `main-2`
    #[test]
    fn duplicate_branch_names_take_the_next_number() {
        let t = tmp();
        let one = folder_for(t.path(), &id("github", "a/one", "main")).unwrap();
        let two = folder_for(t.path(), &id("gitee", "b/two", "main")).unwrap();
        assert_eq!(one.as_deref(), Some("main"));
        assert_eq!(two.as_deref(), Some("main-2"));
    }

    /// 没有 ref 的源不铺文件夹：保持原来的布局
    #[test]
    fn a_source_without_a_branch_stays_flat() {
        let t = tmp();
        assert_eq!(
            folder_for(t.path(), &id("custom", "127.0.0.1:8787", NO_REF)).unwrap(),
            None
        );
    }

    /// 导入那些"没有来源"的目录名不许被某个分支占掉
    #[test]
    fn the_unfiled_folder_name_is_reserved() {
        let t = tmp();
        let got = folder_for(t.path(), &id("github", "o/r", UNFILED)).unwrap();
        assert_ne!(got.as_deref(), Some(UNFILED));
    }
}
