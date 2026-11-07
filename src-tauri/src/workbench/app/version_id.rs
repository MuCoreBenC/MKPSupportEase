//! **版本身份四格**（只读）：已安装 / 本地已打包 / 最新已发布 / 当前开发。
//!
//! # 为什么不能拿 `wb_app_version` 当"已安装"
//!
//! `wb_app_version` 返回 `APP_VERSION`（编译期常量）—— 它回答"**这个二进制**是哪一版"，
//! 不回答"**这台机器上装着**哪一版"。2026-11-07 真机：界面写「已安装 0.0.10-preview.8」，
//! 而 `/Applications/SupportEase.app` 的 `Info.plist` 里是 `0.0.10-preview.3`。
//!
//! 四个号不一样是**常态**，不是错误。每一格都带**出处**（`at`）与**怎么去**（`open`）：
//! 出处是"这个数是从哪个文件读的"，`open` 是白名单键（前端只传键，路径由后端重算）。
//!
//! 读的全是本机文件，**没有网络请求**。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;

/// 安装包在 macOS 上的候选落点（**顺序即优先级**）。
const MACOS_APP_CANDIDATES: [&str; 2] = [
    "/Applications/SupportEase.app",
    "~/Applications/SupportEase.app",
];

/// 打包产物目录（相对仓库根）：`npm run preview:build` / `local-release` 写这里。
const LOCAL_RELEASE_DIR: &str = "target/local-release";

/// 清单落点（相对仓库根）—— 与发布事务那两个常量同一个位置。
const RELEASE_LISTS: [&str; 2] = [
    "presets/delivery/release-preview.json",
    "presets/delivery/release.json",
];

/// 有落点才给按钮：`key` 是白名单键（前端只传它），`label` 是按钮上那句话。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionOpen {
    pub key: String,
    pub label: String,
}

impl VersionOpen {
    fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
        }
    }
}

/// 一格读数：有值就给值，没有就给**为什么没有**；有出处就给出处。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 这个读数从哪来：`.app` 路径 / 打包目录 / Release 地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// 有可打开的位置才给
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<VersionOpen>,
}

impl VersionCell {
    fn ok(v: impl Into<String>) -> Self {
        Self {
            value: Some(v.into()),
            error: None,
            at: None,
            open: None,
        }
    }

    fn err(why: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(why.into()),
            at: None,
            open: None,
        }
    }

    fn at(mut self, at: impl Into<String>) -> Self {
        self.at = Some(at.into());
        self
    }

    fn open(mut self, key: &str, label: &str) -> Self {
        self.open = Some(VersionOpen::new(key, label));
        self
    }
}

/// 四个号 + 这个二进制自己的号。**五格各自独立**，一格读不到不影响别的格。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionIdentity {
    /// 这个工作台二进制自报的号（编译期常量）—— **不是**安装包版本
    pub build: String,
    pub installed: VersionCell,
    pub packaged: VersionCell,
    pub published: VersionCell,
    pub developing: VersionCell,
}

/// 安装包的候选路径（`~` 展开成 HOME）。**纯函数**，判据不依赖本机到底装没装。
fn app_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    MACOS_APP_CANDIDATES
        .iter()
        .map(|raw| match raw.strip_prefix("~/") {
            Some(rest) => match home {
                Some(h) => h.join(rest),
                /* 没有 HOME：这一条直接跳过（保留形状，交给 pick_app 跳过不存在的那条） */
                None => PathBuf::from(raw),
            },
            None => PathBuf::from(raw),
        })
        .collect()
}

/// 选中第一个 `Contents/Info.plist` 真的存在的候选。
fn pick_app(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|p| p.join("Contents/Info.plist").is_file())
        .cloned()
}

/// 读某个 `.app` 的 `CFBundleShortVersionString`。
///
/// 用 `PlistBuddy` 而不是自己解析：`Info.plist` 可能是二进制 plist，自己写解析器
/// 等于再造一个容易漂的格式实现。读不到就如实说原因，**不退回编译期常量**。
fn plist_version(app: &Path) -> Result<String, String> {
    let plist = app.join("Contents/Info.plist");
    let out = std::process::Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg("Print :CFBundleShortVersionString")
        .arg(&plist)
        .output()
        .map_err(|e| format!("跑不了 PlistBuddy：{e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(if why.is_empty() {
            "PlistBuddy 没读出来（退出码非 0）".to_owned()
        } else {
            format!("PlistBuddy 读不出来：{why}")
        });
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if v.is_empty() {
        return Err("Info.plist 里没有 CFBundleShortVersionString".to_owned());
    }
    Ok(v)
}

/// **已安装**：候选路径里第一个真装着的那份。
fn installed_cell() -> VersionCell {
    if !cfg!(target_os = "macos") {
        return VersionCell::err(format!(
            "本机是 {} —— 安装包版本只在 macOS 上读得到",
            std::env::consts::OS
        ));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let candidates = app_candidates(home.as_deref());
    let Some(app) = pick_app(&candidates) else {
        /* 没找到安装包时，按钮变成"打开 /Applications"—— 那时人想知道的正是"到底装哪了" */
        return VersionCell::err("没在这些位置找到 SupportEase.app")
            .open("installed", "打开 /Applications");
    };
    let at = app.display().to_string();
    match plist_version(&app) {
        Ok(v) => VersionCell::ok(v).at(at).open("installed", "在访达中显示"),
        Err(why) => VersionCell::err(why)
            .at(at)
            .open("installed", "在访达中显示"),
    }
}

/// **本地已打包**：最后一次 `preview:build` 写出的号与目录。
fn packaged_cell(repo: &Path) -> VersionCell {
    let dir = repo.join(LOCAL_RELEASE_DIR);
    let path = dir.join("release-preview.json");
    if !path.is_file() {
        return VersionCell::err(format!("{} 不在（还没打包过）", path.display()));
    }
    let parsed = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    let value = parsed
        .as_ref()
        .and_then(|v| v.get("version").and_then(|x| x.as_str()));
    match value {
        Some(v) => VersionCell::ok(v.to_owned())
            .at(dir.display().to_string())
            .open("packaged", "打开打包目录"),
        None => VersionCell::err(format!("{} 读不出 version 字段", path.display()))
            .at(dir.display().to_string())
            .open("packaged", "打开打包目录"),
    }
}

/// **某一版有没有本机 Release 记录**：清单里的 `version` 必须**正好是这一版**才认。
///
/// 不编 URL：清单里没有这一版就返回 `None`（AGENTS §8.1）。依次看预览清单、正式清单。
fn release_url_for(repo: &Path, tag: &str) -> Option<String> {
    let want = tag.trim_start_matches('v');
    for rel in RELEASE_LISTS {
        let path = repo.join(rel);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(info) = crate::runtime::release_info::parse(&bytes) else {
            continue;
        };
        if info.version.trim() == want && !info.url.trim().is_empty() {
            return Some(info.url.trim().to_owned());
        }
    }
    None
}

/// **最新已发布**：最新 git tag；有本机 Release 记录才给「打开」。
fn published_cell(repo: &Path) -> VersionCell {
    let tags = match super::git::Git::open(repo).tags() {
        Ok(tags) => tags,
        Err(e) => return VersionCell::err(format!("读不到 git tag：{}", e.message)),
    };
    let Some(latest) = tags.first() else {
        return VersionCell::err("仓库里一个 tag 都没有（还没冻结过任何版本）");
    };
    let cell = VersionCell::ok(latest.tag.trim_start_matches('v'));
    match release_url_for(repo, &latest.tag) {
        Some(url) => cell.at(url).open("published", "打开 Release 页"),
        None => cell.at(format!(
            "这一版（{}）的 Release 地址不在本机记录里",
            latest.tag
        )),
    }
}

/// 四格读数。**除 `build` 外都来自盘 / git**，一格失败只写进那一格。
pub fn identity(repo: &Path) -> VersionIdentity {
    let developing = match super::version::app_version(repo) {
        Ok(v) => VersionCell::ok(v),
        Err(e) => VersionCell::err(e.message),
    };
    VersionIdentity {
        build: crate::runtime::structure::APP_VERSION.to_owned(),
        installed: installed_cell(),
        packaged: packaged_cell(repo),
        published: published_cell(repo),
        developing,
    }
}

/// **版本身份**（只读）：四格 + 这个二进制自己的号。不写盘、不联网。
#[tauri::command(async)]
pub fn wb_version_identity() -> Result<VersionIdentity, AppError> {
    crate::ipc::traced("wb_version_identity", |_| {
        Ok(identity(&crate::workbench::paths::repo_root()))
    })
}

/* ---------- 「打开位置」：前端只传白名单键 ---------- */

/// 允许打开的三格。**认不出就拒绝** —— 这是个"把字符串变成系统动作"的口子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    Installed,
    Packaged,
    Published,
}

fn parse_which(key: &str) -> Option<Which> {
    match key.trim() {
        "installed" => Some(Which::Installed),
        "packaged" => Some(Which::Packaged),
        "published" => Some(Which::Published),
        _ => None,
    }
}

/// **打开某一格对应的位置**（只读：只开系统文件管理器 / 浏览器，不写任何状态）。
///
/// - `installed`：在访达里**选中**那个 `.app`（`reveal_item_in_dir`；不能用 `open_path`，
///   那会把 app **启动**起来）；没找到安装包就打开 `/Applications` 让人自己看。
/// - `packaged`：优先选中清单里那份安装包文件，其次打开打包目录。
/// - `published`：打开该版本的本机 Release 记录里的地址；记录不在就拒绝（不猜链接）。
#[tauri::command(async)]
pub fn wb_open_version_location(app: tauri::AppHandle, which: String) -> Result<(), AppError> {
    crate::ipc::traced("wb_open_version_location", |_| {
        use tauri_plugin_opener::OpenerExt as _;
        let opener = app.opener();
        let repo = crate::workbench::paths::repo_root();
        let Some(target) = parse_which(&which) else {
            return Err(AppError::invalid_argument(format!(
                "认不出的位置「{which}」—— 只认 installed / packaged / published"
            )));
        };
        match target {
            Which::Installed => {
                let home = std::env::var_os("HOME").map(PathBuf::from);
                match pick_app(&app_candidates(home.as_deref())) {
                    Some(app_path) => opener.reveal_item_in_dir(&app_path).map_err(|e| {
                        AppError::io("打不开系统文件管理器").with_detail(e.to_string())
                    }),
                    None => opener
                        .open_path("/Applications", None::<&str>)
                        .map_err(|e| {
                            AppError::io("打不开系统文件管理器").with_detail(e.to_string())
                        }),
                }
            }
            Which::Packaged => {
                let dir = repo.join(LOCAL_RELEASE_DIR);
                if !dir.is_dir() {
                    return Err(AppError::not_found(format!(
                        "打包目录还没建出来（{}）—— 先跑一次 npm run preview:build",
                        dir.display()
                    )));
                }
                /* 清单里登记了安装包文件名就选中它（人要看的多半是那个包） */
                let asset = std::fs::read(dir.join("release-preview.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| {
                        v.get("asset")
                            .and_then(|a| a.get("name"))
                            .and_then(|n| n.as_str())
                            .map(str::to_owned)
                    })
                    .map(|name| dir.join(name))
                    .filter(|p| p.is_file());
                match asset {
                    Some(file) => opener.reveal_item_in_dir(&file).map_err(|e| {
                        AppError::io("打不开系统文件管理器").with_detail(e.to_string())
                    }),
                    None => opener
                        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
                        .map_err(|e| {
                            AppError::io("打不开系统文件管理器").with_detail(e.to_string())
                        }),
                }
            }
            Which::Published => {
                let tag = super::git::Git::open(&repo)
                    .tags()
                    .map_err(|e| AppError::io("读不到 git tag").with_detail(e.message))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| AppError::not_found("仓库里一个 tag 都没有"))?;
                let url = release_url_for(&repo, &tag.tag).ok_or_else(|| {
                    AppError::not_found(format!(
                        "这一版（{}）的 Release 地址不在本机记录里",
                        tag.tag
                    ))
                })?;
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Err(AppError::invalid_argument(
                        "只允许打开 http(s) 链接 —— 别的形状不交给系统",
                    ));
                }
                opener
                    .open_url(url, None::<&str>)
                    .map_err(|e| AppError::io("打不开浏览器").with_detail(e.to_string()))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 缺文件是**合法的空**，但要说清是"还没打包"，不能给个空值装成读到了。
    #[test]
    fn a_missing_package_says_it_was_never_packaged() {
        let dir = tempfile::tempdir().unwrap();
        let cell = packaged_cell(dir.path());
        assert!(cell.value.is_none());
        let why = cell.error.unwrap_or_default();
        assert!(why.contains("还没打包过"), "要说清原因：{why}");
    }

    /// 打包清单在就照实读，并带出**目录**与「打开打包目录」这颗按钮。
    #[test]
    fn a_packed_version_carries_its_directory() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join(LOCAL_RELEASE_DIR);
        std::fs::create_dir_all(&out).unwrap();
        crate::fsx::atomic::atomic_write(
            &out.join("release-preview.json"),
            br#"{"version":"0.0.10-preview.7"}"#,
        )
        .unwrap();
        let cell = packaged_cell(dir.path());
        assert_eq!(cell.value.as_deref(), Some("0.0.10-preview.7"));
        assert!(cell.at.unwrap_or_default().ends_with(LOCAL_RELEASE_DIR));
        assert_eq!(cell.open.map(|o| o.key), Some("packaged".to_owned()));

        crate::fsx::atomic::atomic_write(&out.join("release-preview.json"), b"{}").unwrap();
        let cell = packaged_cell(dir.path());
        assert!(cell.value.is_none());
        assert!(cell.error.is_some_and(|e| e.contains("读不出 version")));
    }

    /// `~/` 要展开成 HOME 下那一份；`/Applications` 那条保持绝对路径。
    #[test]
    fn the_home_candidate_expands() {
        let home = tempfile::tempdir().unwrap();
        let candidates = app_candidates(Some(home.path()));
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0],
            PathBuf::from("/Applications/SupportEase.app")
        );
        assert_eq!(
            candidates[1],
            home.path().join("Applications/SupportEase.app"),
            "~/Applications 要展开成 HOME 下那一份"
        );
    }

    /// 候选按顺序取第一个真装着的（用临时目录造两份，不碰这台机器上真装的那个）。
    #[test]
    fn the_first_existing_app_wins() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("Applications/SupportEase.app");
        let second = root.path().join("Other/SupportEase.app");
        let candidates = vec![first.clone(), second.clone()];
        assert!(pick_app(&candidates).is_none(), "两个都不存在 ⇒ 没装");

        let install = |p: &Path| {
            std::fs::create_dir_all(p.join("Contents")).unwrap();
            crate::fsx::atomic::atomic_write(&p.join("Contents/Info.plist"), b"plist").unwrap();
        };
        /* 只有第二份时取第二份 */
        install(&second);
        assert_eq!(pick_app(&candidates), Some(second.clone()));
        /* 两份都在时取第一份（顺序即优先级） */
        install(&first);
        assert_eq!(pick_app(&candidates), Some(first));
    }

    /// 本机清单里**没有这一版**就不给 URL —— 不编链接。
    #[test]
    fn a_release_url_requires_the_exact_version() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = dir.path().join("presets/delivery");
        std::fs::create_dir_all(&delivery).unwrap();
        crate::fsx::atomic::atomic_write(
            &delivery.join("release-preview.json"),
            br#"{"releaseSchema":1,"version":"0.0.10-preview.2","url":"https://example.test/tag/v0.0.10-preview.2"}"#,
        )
        .unwrap();

        assert_eq!(
            release_url_for(dir.path(), "v0.0.10-preview.2").as_deref(),
            Some("https://example.test/tag/v0.0.10-preview.2"),
            "版本正好对上才给"
        );
        assert!(
            release_url_for(dir.path(), "v0.0.10-preview.7").is_none(),
            "清单里没有 .7 ⇒ 不给（不编）"
        );

        crate::fsx::atomic::atomic_write(&delivery.join("release-preview.json"), b"{ broken")
            .unwrap();
        assert!(
            release_url_for(dir.path(), "v0.0.10-preview.2").is_none(),
            "坏清单不许 panic，也不许猜"
        );
    }

    /// 「打开位置」只认三个键；别的（含穿越路径）一律认不出。
    #[test]
    fn only_three_location_keys_are_allowed() {
        assert_eq!(parse_which("installed"), Some(Which::Installed));
        assert_eq!(parse_which("packaged"), Some(Which::Packaged));
        assert_eq!(parse_which("published"), Some(Which::Published));
        assert_eq!(parse_which("../../etc/passwd"), None);
        assert_eq!(parse_which(""), None);
    }
}
