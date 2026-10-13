//! 本机 BBS 工艺预设（**只读**）—— 工艺页左边那个「本机工艺」档的数据面。
//!
//! # 为什么要有这两条命令
//!
//! 那一档列的是**你此刻在 Bambu Studio 里有哪几份工艺预设**（在 BBS 里另存一份、
//! 回来重扫就该看见），所以清单只能从本机目录现读 —— 本仓不打包那 285 份快照
//! （作者裁决 §6-2），读不到就是空态，不静默退到一份陈旧快照。
//!
//! 试验场里这件事是 dev-server 的一个只读端点（`tools/dev-server/bbsFs.mjs` 的
//! `GET /api/bbs/presets` 与 `/api/bbs/preset`）。**打包后的桌面版既没有 dev server
//! 也没有 preview server** ⇒ 在产品里那两条端点换成这两条命令，返回的 JSON 形状与
//! 端点逐字段一致（`available / app / root / user[] / system[]`），前端取数层不必
//! 为两边写两套解析。
//!
//! # 目录口径（三平台，2026-10-13 实测）
//!
//! ```text
//!   系统预设（出厂，278 份那一档）
//!     macOS   /Applications/BambuStudio.app/Contents/Resources/profiles/BBL/process
//!             ↑ **在 app 包里**，不在 ~/Library 下
//!     Windows %APPDATA%\BambuStudio\system\BBL\process
//!     Linux   ~/.config/BambuStudio/system/BBL/process
//!   用户预设（自己另存的，按账号分目录）
//!     macOS   ~/Library/Application Support/BambuStudio/user/<uid>/process
//!     Windows %APPDATA%\BambuStudio\user/<uid>/process
//!     Linux   ~/.config/BambuStudio/user/<uid>/process
//! ```
//!
//! 两层**分别判存在**（装了没跑过就没有用户层，那半页照常能看）；正式版优先、
//! 找不到才退 Beta。`user/` 下按账号分目录（数字 uid，也有 `BBL` / `default` 这种），
//! **只认里面真有 `process/` 子目录的**。
//!
//! # 读不到不是错误
//!
//! 没装 BBS = `available: false` + 一句人话（`reason`），与"读失败"是两件事 ——
//! 界面照实说"这台机器上没有那个目录"，与端点同一条口径。
//!
//! 单份正文**原文透传**（不校验是不是合法 JSON）：坏文件要由前端照实标出来，
//! 而不是在这里被吞掉。列表里那份会带 `broken` 的原话。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;

use super::traced;

/// 正式版优先，找不到才退 Beta（与 `ipc::presets::slicer_process_dir` 同一套口径）
const BBS_APPDIRS: [&str; 2] = ["BambuStudio", "BambuStudioBeta"];

/// 单份预设的字节上限（与 dev 端点同一个数）。工艺预设本体都在 1~4 KB，
/// 这个上限只是"别把一份几 MB 的东西当预设读进来"那道闸。
const MAX_PRESET_BYTES: u64 = 1024 * 1024;

/// 清单里的一项 —— 字段名与那批 json / dev 端点**逐字一致**（`snake_case` 照旧），
/// 前端拿到的形状与试验场里那份完全一样。
#[derive(Debug, Clone, Serialize)]
pub struct BbsPresetEntry {
    /// `user` / `system`
    pub scope: &'static str,
    /// 账号目录名；系统层那一档是 `null`
    pub uid: Option<String>,
    pub file: String,
    /// 文件名里写的那个名字（读不出来就是去掉 `.json` 的文件名）
    pub name: String,
    pub inherits: Option<String>,
    /// BBS 自己的写法就有两种（字符串 `"false"` 与布尔 `false` 都出现过）—— 照抄，不归一
    pub instantiation: Option<serde_json::Value>,
    pub compatible_printers: Option<Vec<String>>,
    pub from: Option<String>,
    /// 修改时间（毫秒，与 `fs::Metadata::modified` 的原样）
    pub mtime: f64,
    pub size: u64,
    /// **坏文件也要列出来**（解析失败的原话），让界面标灰而不是悄悄少一项
    pub broken: Option<String>,
}

/// 「本机有没有 BBS、有哪几份」
#[derive(Debug, Clone, Serialize)]
pub struct BbsPresetTree {
    pub available: bool,
    /// 命中的是哪个应用（`BambuStudio` / `BambuStudioBeta`）
    pub app: Option<String>,
    /// 这次读的是哪棵树（系统层在就用它，否则用户层）
    pub root: Option<String>,
    pub user: Vec<BbsPresetEntry>,
    pub system: Vec<BbsPresetEntry>,
    /// `available: false` 时那一句给人看的话
    pub reason: Option<String>,
}

impl BbsPresetTree {
    fn missing(reason: String) -> Self {
        Self {
            available: false,
            app: None,
            root: None,
            user: Vec::new(),
            system: Vec::new(),
            reason: Some(reason),
        }
    }
}

/// 一棵候选树：应用名 + 系统层的候选目录 + 用户基目录的候选
struct TreeSpec {
    app: String,
    system: Vec<PathBuf>,
    user_base: Vec<PathBuf>,
}

/// 挑中的那一棵树
struct Layout {
    app: String,
    /// `None` = 这一层不存在（只装了、没跑过就没有用户层；同理系统层可能缺）
    system: Option<PathBuf>,
    user_base: Option<PathBuf>,
    /// 用户层下真有 `process/` 的账号目录名
    uids: Vec<String>,
}

/* ---------- 平台分叉：只活在这一段 ---------- */

/// 家目录。**Windows 那两档不用它**（那边一切都在 `%APPDATA%` 下）——
/// 只在 macOS / Linux 的落脚表里出现，所以这里也跟着 cfg，免得 Windows 上留个死函数。
#[cfg(not(target_os = "windows"))]
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// 系统层的候选：**macOS 在 app 包里，Win/Linux 在数据根下**
#[cfg(target_os = "macos")]
fn system_dirs(app: &str) -> Vec<PathBuf> {
    let tail = ["Contents", "Resources", "profiles", "BBL", "process"];
    let bundle = format!("{app}.app");
    let mut out = Vec::new();
    if let Some(h) = home() {
        out.push(h.join("Applications").join(&bundle).join(tail.join("/")));
    }
    out.push(
        PathBuf::from("/Applications")
            .join(&bundle)
            .join(tail.join("/")),
    );
    out
}

#[cfg(target_os = "macos")]
fn user_base_dirs(app: &str) -> Vec<PathBuf> {
    home()
        .map(|h| vec![h.join("Library/Application Support").join(app).join("user")])
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn system_dirs(app: &str) -> Vec<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|base| vec![PathBuf::from(base).join(app).join("system/BBL/process")])
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn user_base_dirs(app: &str) -> Vec<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|base| vec![PathBuf::from(base).join(app).join("user")])
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_dirs(app: &str) -> Vec<PathBuf> {
    home()
        .map(|h| vec![h.join(".config").join(app).join("system/BBL/process")])
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn user_base_dirs(app: &str) -> Vec<PathBuf> {
    home()
        .map(|h| vec![h.join(".config").join(app).join("user")])
        .unwrap_or_default()
}

/// 这台机器上要依次找哪几棵树（正式版优先）
fn tree_specs() -> Vec<TreeSpec> {
    BBS_APPDIRS
        .iter()
        .map(|app| TreeSpec {
            app: (*app).to_owned(),
            system: system_dirs(app),
            user_base: user_base_dirs(app),
        })
        .collect()
}

/// 没找到时那句给人看的话 —— 说清"这一档找的是哪儿"，别让人猜
fn missing_reason() -> String {
    let where_text = if cfg!(target_os = "windows") {
        "%APPDATA%\\BambuStudio\\system\\BBL\\process"
    } else if cfg!(target_os = "macos") {
        "/Applications/BambuStudio.app/Contents/Resources/profiles/BBL/process"
    } else {
        "~/.config/BambuStudio/system/BBL/process"
    };
    format!(
        "本机没找到 {} 的工艺预设目录（这一档找的是 {where_text}）",
        BBS_APPDIRS.join(" / ")
    )
}

/* ---------- 挑树与扫描 ---------- */

/// 挑第一棵真有东西的树。**两层分别判存在**：有一层就算有。
///
/// 抽出来是为了能在测试里喂临时目录 —— 平台差异只活在 [`system_dirs`] /
/// [`user_base_dirs`] 那几段里。
fn pick_layout(specs: Vec<TreeSpec>) -> Option<Layout> {
    for spec in specs {
        let system = spec.system.into_iter().find(|d| d.is_dir());
        let user_base = spec.user_base.into_iter().find(|d| d.is_dir());
        let uids: Vec<String> = user_base
            .as_ref()
            .map(|base| {
                let mut out: Vec<String> = std::fs::read_dir(base)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().join("process").is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                out.sort();
                out
            })
            .unwrap_or_default();
        if system.is_none() && uids.is_empty() {
            continue;
        }
        return Some(Layout {
            app: spec.app,
            system,
            user_base,
            uids,
        });
    }
    None
}

/// 扫一层目录（不递归）。**坏文件也列出来**（带 `broken` 的原话），排序稳定。
fn scan_dir(dir: Option<&Path>, scope: &'static str, uid: Option<&str>) -> Vec<BbsPresetEntry> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .filter(|(name, _)| name.to_ascii_lowercase().ends_with(".json"))
        .collect();
    files.sort();

    files
        .into_iter()
        .filter_map(|(file, path)| {
            let meta = std::fs::metadata(&path).ok()?;
            let text = std::fs::read(&path).ok()?;
            let (doc, broken) = match serde_json::from_slice::<serde_json::Value>(&text) {
                Ok(v) => (Some(v), None),
                /* JSON 坏了也要列出来 —— 让界面标灰，而不是这一项凭空消失 */
                Err(e) => (None, Some(e.to_string())),
            };
            let field = |key: &str| -> Option<String> {
                doc.as_ref()?.get(key)?.as_str().map(str::to_owned)
            };
            Some(BbsPresetEntry {
                scope,
                uid: uid.map(str::to_owned),
                name: field("name").unwrap_or_else(|| file.trim_end_matches(".json").to_owned()),
                inherits: field("inherits"),
                instantiation: doc
                    .as_ref()
                    .and_then(|d| d.get("instantiation"))
                    .filter(|v| !v.is_null())
                    .cloned(),
                compatible_printers: doc
                    .as_ref()
                    .and_then(|d| d.get("compatible_printers"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect::<Vec<_>>()
                    }),
                from: field("from"),
                mtime: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0),
                size: meta.len(),
                broken,
                file,
            })
        })
        .collect()
}

/* ---------- 两条命令 ---------- */

/// 本机 BBS 目录里现在有哪几份工艺预设（用户层 + 系统层，一次读全）。
#[tauri::command]
pub fn get_bbs_presets() -> Result<BbsPresetTree, AppError> {
    traced("getBbsPresets", |_| {
        let Some(layout) = pick_layout(tree_specs()) else {
            return Ok(BbsPresetTree::missing(missing_reason()));
        };
        let user = layout
            .uids
            .iter()
            .flat_map(|uid| {
                let dir = layout
                    .user_base
                    .as_ref()
                    .map(|base| base.join(uid).join("process"));
                scan_dir(dir.as_deref(), "user", Some(uid))
            })
            .collect();
        let system = scan_dir(layout.system.as_deref(), "system", None);
        let root = layout.system.clone().or_else(|| layout.user_base.clone());
        Ok(BbsPresetTree {
            available: true,
            app: Some(layout.app),
            root: root.map(|p| p.to_string_lossy().into_owned()),
            user,
            system,
            reason: None,
        })
    })
}

/// 读其中一份的**原文**（JSON 文本）。解析交给前端：坏 JSON 要由前端照实报错。
///
/// 路径不由前端拼 —— 只能给 `scope` + `uid` + `file`：
/// `file` 必须是**单个 `.json` 文件名**（不接受任何路径），`uid` 必须在扫到的账号
/// 目录里，拼出来之后再确认落在那一层目录之下。这几条与 dev 端点的安全边界逐条对应。
#[tauri::command]
pub fn read_bbs_preset(
    scope: String,
    uid: Option<String>,
    file: String,
) -> Result<String, AppError> {
    traced("readBbsPreset", |_| {
        let Some(layout) = pick_layout(tree_specs()) else {
            return Err(AppError::not_found(format!(
                "{} —— 先在 Bambu Studio 里另存一份工艺预设，再回来重扫",
                missing_reason()
            )));
        };
        let dir = scope_dir(&layout, &scope, uid.as_deref())?;
        read_one(&dir, &file)
    })
}

/// 这一档读哪个目录。**scope 与 uid 都要对上**才给目录（对不上整次拒绝）
fn scope_dir(layout: &Layout, scope: &str, uid: Option<&str>) -> Result<PathBuf, AppError> {
    match scope {
        "system" => match &layout.system {
            Some(dir) => Ok(dir.clone()),
            None => Err(AppError::not_found(
                "这台机器上没有系统预设那一层（只有用户层）".to_owned(),
            )),
        },
        "user" => {
            let Some(uid) = uid else {
                return Err(AppError::invalid_argument(
                    "读用户预设要给 uid（账号目录名）".to_owned(),
                ));
            };
            if !layout.uids.iter().any(|u| u == uid) {
                return Err(AppError::invalid_argument(format!(
                    "uid 不在扫到的账号目录里：{uid}"
                )));
            }
            match &layout.user_base {
                Some(base) => Ok(base.join(uid).join("process")),
                None => Err(AppError::not_found(
                    "这台机器上没有用户预设那一层（只有系统层）".to_owned(),
                )),
            }
        }
        other => Err(AppError::invalid_argument(format!(
            "scope 只能是 user 或 system：{other}"
        ))),
    }
}

/// 在**白名单目录里**读一个单文件名。三条闸：文件名合法、结果仍在目录之下、体积有上限。
fn read_one(dir: &Path, file: &str) -> Result<String, AppError> {
    if file.is_empty()
        || !file.to_ascii_lowercase().ends_with(".json")
        || Path::new(file)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            != Some(file.to_owned())
    {
        return Err(AppError::invalid_argument(format!(
            "file 必须是单个 .json 文件名，不接受路径：{file}"
        )));
    }
    // join 完再确认一次：结果必须仍在这一层目录之下
    let full = dir.join(file);
    if !full.starts_with(dir) {
        return Err(AppError::invalid_argument("路径越界".to_owned()));
    }
    if !full.is_file() {
        return Err(AppError::not_found(format!("没有这个文件：{file}")));
    }
    let meta = std::fs::metadata(&full)
        .map_err(|e| AppError::io(format!("看不了这份文件：{file}")).with_detail(e.to_string()))?;
    if meta.len() > MAX_PRESET_BYTES {
        return Err(AppError::invalid_argument(format!(
            "文件过大：{} > {MAX_PRESET_BYTES}",
            meta.len()
        )));
    }
    std::fs::read_to_string(&full)
        .map_err(|e| AppError::io(format!("读这份文件失败：{file}")).with_detail(e.to_string()))
}

#[cfg(test)]
mod tests {
    // 判据要造一棵真实形状的目录树 —— 生产代码的写盘走 `fsx::atomic` 那一个洞。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    /// 造一棵临时树：`BambuStudio/system/BBL/process` + `BambuStudio/user/<uid>/process`
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("BambuStudio");
        std::fs::create_dir_all(app.join("system/BBL/process")).unwrap();
        std::fs::create_dir_all(app.join("user/123456/process")).unwrap();
        std::fs::create_dir_all(app.join("user/BBL/process")).unwrap();
        /* 没有 process/ 的账号目录：不算一层（与端点同一条判据） */
        std::fs::create_dir_all(app.join("user/no_process")).unwrap();
        (dir, app)
    }

    fn write(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).unwrap();
    }

    fn spec_for(app: &Path) -> TreeSpec {
        TreeSpec {
            app: "BambuStudio".to_owned(),
            system: vec![app.join("system/BBL/process")],
            user_base: vec![app.join("user")],
        }
    }

    #[test]
    fn scan_lists_json_files_even_broken_ones() {
        let (_tmp, app) = fixture();
        let sys = app.join("system/BBL/process");
        write(
            &sys,
            "b.json",
            r#"{ "name": "B", "inherits": "fdm_process_common", "from": "system",
                 "instantiation": "false", "compatible_printers": ["Bambu Lab A1 0.4 nozzle"] }"#,
        );
        write(&sys, "a.json", "{ 这不是 json }");
        write(&sys, "note.txt", "{}");
        std::fs::create_dir_all(sys.join("dir.json")).unwrap();

        let got = scan_dir(Some(&sys), "system", None);
        assert_eq!(got.len(), 2, "只认 .json 文件（目录与非 json 都不算）");
        assert_eq!(got[0].file, "a.json", "按文件名排，顺序稳定");
        assert!(got[0].broken.is_some(), "坏 JSON 也要列出来，带原话");
        assert_eq!(
            got[0].name, "a",
            "解析不出来就用去掉 .json 的文件名（不编一个名字）"
        );
        assert_eq!(got[1].name, "B");
        assert_eq!(got[1].inherits.as_deref(), Some("fdm_process_common"));
        assert_eq!(got[1].from.as_deref(), Some("system"));
        assert_eq!(
            got[1].compatible_printers.as_deref(),
            Some(["Bambu Lab A1 0.4 nozzle".to_owned()].as_slice())
        );
        assert_eq!(
            got[1].instantiation,
            Some(serde_json::Value::String("false".to_owned()))
        );
        assert_eq!(got[1].scope, "system");
        assert!(got[1].uid.is_none());
        assert!(got[1].size > 0 && got[1].mtime > 0.0);
    }

    #[test]
    fn a_tree_counts_when_only_one_layer_is_there() {
        let (_tmp, app) = fixture();
        /* 系统层是空的、用户层有两份 —— 照样算"这台机器上有"（两层分别判存在） */
        let layout = pick_layout(vec![spec_for(&app)]).expect("有用户层就算有");
        assert!(
            layout.system.is_some(),
            "目录存在就是有（空目录也算这一层在）"
        );
        assert_eq!(
            layout.uids,
            vec!["123456".to_owned(), "BBL".to_owned()],
            "只认真正有 process/ 的账号目录，且顺序稳定"
        );
    }

    #[test]
    fn trees_are_tried_in_order_and_empty_ones_are_skipped() {
        let (tmp, _app) = fixture();
        let beta = tmp.path().join("BambuStudioBeta");
        std::fs::create_dir_all(beta.join("system/BBL/process")).unwrap();
        write(&beta.join("system/BBL/process"), "only.json", "{}");

        /* 第一棵树两层都不在 → 落到第二棵 */
        let empty = TreeSpec {
            app: "BambuStudio".to_owned(),
            system: vec![tmp.path().join("没有这一层")],
            user_base: vec![tmp.path().join("也没有")],
        };
        let beta_spec = TreeSpec {
            app: "BambuStudioBeta".to_owned(),
            system: vec![beta.join("system/BBL/process")],
            user_base: vec![beta.join("user")],
        };
        let layout = pick_layout(vec![empty, beta_spec]).expect("第二棵该被挑中");
        assert_eq!(layout.app, "BambuStudioBeta");
        assert!(pick_layout(Vec::new()).is_none());

        let tree = BbsPresetTree::missing(missing_reason());
        assert!(!tree.available && tree.reason.is_some() && tree.user.is_empty());
    }

    #[test]
    fn reading_one_file_goes_through_the_three_gates() {
        let (_tmp, app) = fixture();
        let sys = app.join("system/BBL/process");
        write(&sys, "ok.json", r#"{ "name": "OK" }"#);

        assert_eq!(read_one(&sys, "ok.json").unwrap(), r#"{ "name": "OK" }"#);

        /* ① 文件名：不接受路径、只认 .json */
        for bad in ["../ok.json", "sub/ok.json", "ok.txt", ""] {
            assert!(
                read_one(&sys, bad).is_err(),
                "{bad} 该被挡（file 只能是单个 .json 文件名）"
            );
        }
        /* ② 不在这一层目录里 */
        assert!(read_one(&sys, "nope.json").is_err());
        /* ③ 体积上限 */
        let big = sys.join("big.json");
        std::fs::write(&big, vec![b'x'; (MAX_PRESET_BYTES + 1) as usize]).unwrap();
        let err = read_one(&sys, "big.json").unwrap_err();
        assert!(err.message.contains("文件过大"), "{}", err.message);
    }

    #[test]
    fn scope_and_uid_must_match_the_scanned_tree() {
        let (_tmp, app) = fixture();
        let layout = pick_layout(vec![spec_for(&app)]).unwrap();

        let sys = scope_dir(&layout, "system", None).expect("系统层在");
        assert_eq!(sys, app.join("system/BBL/process"));

        let user = scope_dir(&layout, "user", Some("123456")).expect("这个 uid 扫到过");
        assert_eq!(user, app.join("user/123456/process"));

        /* 没扫到的 uid、缺 uid、乱写的 scope 一律拒绝 —— 不猜目录 */
        assert!(scope_dir(&layout, "user", Some("nobody")).is_err());
        assert!(scope_dir(&layout, "user", None).is_err());
        assert!(scope_dir(&layout, "other", None).is_err());
    }
}
