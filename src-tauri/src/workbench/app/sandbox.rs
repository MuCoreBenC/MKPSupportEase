//! **测试模式（沙箱）** —— 整套数据根切到仓库里另一棵树，正式目录一个字节都不碰。
//!
//! 作者 2026-10-08 的要求（原话）：
//!
//! > 「我希望它是完全另起炉灶那种……测试的归测试的，而且我测试的随时可以把这些清空，
//! >   从正式的复制一份过去，再进行测试」「我不能迷迷糊糊的，不知道我在测试版还是正式版」
//!
//! ```text
//! 正式                                 测试（沙箱）
//! <repo>/presets/          ← 真源      <repo>/workbench/.sandbox/presets/          ← 整棵副本
//! <repo>/presets/delivery/ ← 真交付                 └─ delivery/                     ← 生成只写这儿
//! <repo>/workbench/*.json  ← 台账                   └─ …（配置一起拷过去）
//! <repo>/workbench/        ← 草稿/回收站            <repo>/workbench/.sandbox/workbench/ ← 草稿也分开
//! ```
//!
//! # 拷什么、不拷什么
//!
//! - `presets/` **整棵**：人维护的 `*.toml`、机器生成的 `delivery/`、以及 `assets/`
//!   里的 3MF / 模型 / 图片（一个不落 —— 作者点名要"包括那些模型啊 3MF 都是"）；
//! - `workbench/` 根下那几个 **`.json`**（`bootstrap.json` 官方源配置、`built.json`
//!   生成台账、`delivery.json`）：有它们沙箱才"跟正式生成的一模一样" ——
//!   生成于是说"没变化"，而不是把九份产物重写一遍；
//! - **不带** `.draft` / `.trash` / `.snapshots`：那是**本机会话状态**（未保存的草稿、
//!   回收站、崩溃快照），带过去会让"我到底保存没保存"变得说不清。
//!
//! # 模式档住哪
//!
//! `<appDataDir>/sandbox.json`（与 `publish-account.json` / `credentials.json` 同一个桶：
//! **本机配置，不进仓库** —— 模式是"我这台机器现在开不开测试"，不是仓库内容）。
//! 缺文件 = 正式；坏档报 `CORRUPTED` 不静默（照那一套）。
//!
//! # 切换时那一下
//!
//! 根换了，内存里那个会话（`Presets` / `Store` / 草稿）全是旧根的 —— 留着它就是
//! "界面显示正式、写盘写沙箱"这种最坏的组合。所以切换之后**把会话丢掉**
//! （[`super::reset_session`]），下一条命令用新根重建。
//!
//! ★ **有未保存的改动时拒绝切换**：草稿也是两套，未保存的东西不会跟着过去，
//! 而"切个模式把我的改动悄悄吞了"是最不该发生的事。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;
use crate::fsx::paths::internal_root;
use crate::workbench::paths;

/// 模式档的名字（`<appDataDir>/sandbox.json`）
pub const SANDBOX_FILE: &str = "sandbox.json";
/// 代次：加字段不升号、改语义才升（与 source / catalog 同一条）
pub const SANDBOX_SCHEMA: u32 = 1;

/// 沙箱根底下那两个子目录的名字（与仓库那边同形）
const PRESETS: &str = "presets";
const WORKBENCH: &str = "workbench";
/// 拷贝时**不带过去**的三样：本机会话状态
const SESSION_DIRS: [&str; 3] = [".draft", ".trash", ".snapshots"];
/// 预设根齐没齐的标志文件（与 `paths::presets_root` 同一个判据）
const PRESETS_MARKER: &str = "registry/param_registry.toml";

/* ---------- DTO ---------- */

/// 测试模式现在什么样。界面按它画那颗开关、那条横幅、以及"沙箱里有什么"。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxStatus {
    /// 测试模式开着吗（= 整套数据根现在指着沙箱）
    pub enabled: bool,
    /// 沙箱根（`<repo>/workbench/.sandbox`）
    pub sandbox_root: String,
    /// 沙箱里那份 `presets/` 建起来了吗（没建 = 刚打开、还没拷）
    pub ready: bool,
    /// 沙箱这一棵里有几个文件 / 几字节（给人一个"真拷了东西"的读数）
    pub files: u64,
    pub bytes: u64,
    /// 正式那一份的预设根（界面摆出"从哪儿拷的"）
    pub real_presets_root: String,
    /// 上一次动作的结果（"已从正式拷了一份：42 个文件 4.3 MB"）
    pub note: Option<String>,
}

/* ---------- 模式档 ---------- */

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SandboxMode {
    #[serde(default = "default_schema")]
    sandbox_schema: u32,
    /// 开着 = 测试模式（整套数据根指着沙箱）
    enabled: bool,
}

fn default_schema() -> u32 {
    SANDBOX_SCHEMA
}

fn mode_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(internal_root(app)?.join(SANDBOX_FILE))
}

/// 读模式。**缺文件 = 正式**（合法）；坏档报 `CORRUPTED`（静默当"正式"会让人以为
/// 自己的测试数据丢了）。
fn read_mode(app: &AppHandle) -> Result<bool, AppError> {
    let path = mode_path(app)?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(
                AppError::io("读不了测试模式档").with_detail(format!("{}：{e}", path.display()))
            )
        }
    };
    let doc: SandboxMode = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::corrupted("测试模式档读不懂 —— 它记着「现在是不是测试环境」，猜错代价太大")
            .with_detail(format!("{}：{e}", path.display()))
    })?;
    if doc.sandbox_schema != SANDBOX_SCHEMA {
        return Err(AppError::corrupted(format!(
            "测试模式档是别的代次（{}，本版认 {SANDBOX_SCHEMA}）",
            doc.sandbox_schema
        ))
        .with_detail(path.display().to_string()));
    }
    Ok(doc.enabled)
}

fn write_mode(app: &AppHandle, enabled: bool) -> Result<(), AppError> {
    atomic_write_json(
        &mode_path(app)?,
        &SandboxMode {
            sandbox_schema: SANDBOX_SCHEMA,
            enabled,
        },
    )
}

/// 开机时把模式读进来装上。**失败只告警、退回正式** —— 模式是"我在哪个环境"，
/// 读不出来时最安全的答案就是"正式"（正式那边的东西永远是真的）。
pub fn install_at_startup(app: &AppHandle) {
    match read_mode(app) {
        Ok(on) => {
            paths::set_sandbox(on);
            if on {
                tracing::warn!(
                    "工作台在**测试模式（沙箱）**下启动 —— 读写都在沙箱里，正式目录不动"
                );
            }
        }
        Err(e) => {
            tracing::warn!(error = %e.message, "测试模式档读不出来，按正式处理");
        }
    }
}

/* ---------- 沙箱那棵树 ---------- */

/// 拷贝的读数（"42 个文件 4.3 MB"）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyReport {
    files: u64,
    bytes: u64,
}

impl CopyReport {
    pub fn files(&self) -> u64 {
        self.files
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

/// 递归拷一棵树。**跳过 `skip` 里那些目录名**（拷 `workbench/` 时用来甩掉
/// `.draft` / `.trash` / `.snapshots`）。
fn copy_tree(from: &Path, to: &Path, skip: &[&str]) -> Result<CopyReport, AppError> {
    let mut out = CopyReport::default();
    copy_into(from, to, skip, &mut out)?;
    Ok(out)
}

fn copy_into(from: &Path, to: &Path, skip: &[&str], acc: &mut CopyReport) -> Result<(), AppError> {
    std::fs::create_dir_all(to).map_err(|e| {
        AppError::io(format!("建不出目录：{}", to.display())).with_detail(e.to_string())
    })?;
    let entries = std::fs::read_dir(from).map_err(|e| {
        AppError::io(format!("读不了目录：{}", from.display())).with_detail(e.to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io("目录项读不出来").with_detail(e.to_string()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skip.iter().any(|s| *s == name) {
            continue;
        }
        let src = entry.path();
        let dst = to.join(&*name);
        /* 符号链接一律不跟（沙箱要的是**字节的副本**；跟出去会把正式那份也牵连进来） */
        let meta = std::fs::symlink_metadata(&src).map_err(|e| {
            AppError::io(format!("读不了：{}", src.display())).with_detail(e.to_string())
        })?;
        if meta.file_type().is_symlink() {
            tracing::warn!(path = %src.display(), "沙箱拷贝跳过符号链接");
            continue;
        }
        if meta.is_dir() {
            copy_into(&src, &dst, &[], acc)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| {
                AppError::io(format!("拷不了：{}", src.display())).with_detail(e.to_string())
            })?;
            acc.files += 1;
            acc.bytes += meta.len();
        }
    }
    Ok(())
}

/// 沙箱里的预设根。
fn sandbox_presets() -> PathBuf {
    paths::sandbox_root().join(PRESETS)
}

/// 沙箱里那份 `presets/` 齐了吗。
fn sandbox_ready() -> bool {
    sandbox_presets().join(PRESETS_MARKER).is_file()
}

/// **从正式拷一份过去**（`presets/` 整棵 + `workbench/*.json`）。
///
/// 落点与来源都由调用方给 —— 命令层传"正式那两处 → 沙箱"，测试传两个临时目录
/// （于是这条判据能在临时目录里真跑一遍，而不是拿真仓库当试验台）。
///
/// 调用方保证"目标是空的或被清过"：这里**不删目标**，只往里写
/// （删是 [`wipe_root`] 的事，两件事分开，谁都不会意外删掉别人的东西）。
fn refill_from(
    real_presets: &Path,
    real_workbench: &Path,
    dest: &Path,
) -> Result<CopyReport, AppError> {
    if !real_presets.join(PRESETS_MARKER).is_file() {
        return Err(
            AppError::not_found("正式那边没有 presets/ —— 没东西可拷").with_detail(format!(
                "期望 {}",
                real_presets.join(PRESETS_MARKER).display()
            )),
        );
    }
    let mut total = copy_tree(real_presets, &dest.join(PRESETS), &[])?;

    /* 配方本：只拷根下那几个 `.json`（配置与台账），**不碰**会话状态那三个点号目录 */
    if !real_workbench.is_dir() {
        return Ok(total);
    }
    let entries = std::fs::read_dir(real_workbench)
        .map_err(|e| AppError::io("读不了正式的 workbench/").with_detail(e.to_string()))?;
    let dst = dest.join(WORKBENCH);
    std::fs::create_dir_all(&dst).map_err(|e| {
        AppError::io(format!("建不出目录：{}", dst.display())).with_detail(e.to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io("目录项读不出来").with_detail(e.to_string()))?;
        let src = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        /* 会话状态那三个点号目录**明确甩掉**（不靠"只挑 .json"间接做到） */
        if SESSION_DIRS.iter().any(|s| *s == name) {
            continue;
        }
        /* 只要那几份 `.json`（配置与台账）—— 目录一概不带 */
        let is_json = src.extension().map(|e| e == "json").unwrap_or(false);
        if !src.is_file() || !is_json {
            continue;
        }
        let meta = std::fs::metadata(&src)
            .map_err(|e| AppError::io("读不了文件").with_detail(e.to_string()))?;
        std::fs::copy(&src, dst.join(name.as_ref())).map_err(|e| {
            AppError::io(format!("拷不了：{}", src.display())).with_detail(e.to_string())
        })?;
        total.files += 1;
        total.bytes += meta.len();
    }
    Ok(total)
}

/// 正式 → 沙箱那一次拷（命令层用的那一份）。`pub` 是给真机烟测用的
/// （`paths::set_sandbox` 之后直接调它，不必去造一个 AppHandle 来走命令壳）。
pub fn refill_now() -> Result<CopyReport, AppError> {
    refill_from(
        &paths::real_presets_root_path(),
        &paths::real_workbench_root(),
        &paths::sandbox_root(),
    )
}

/// 把一棵树整个删掉（**只删给定那一棵**）。
fn wipe_root(root: &Path) -> Result<(), AppError> {
    if !root.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(root)
        .map_err(|e| AppError::io(format!("删不掉：{}", root.display())).with_detail(e.to_string()))
}

/// 清沙箱（正式那边一根汗毛都不碰）。`pub`：同 [`refill_now`]。
pub fn wipe() -> Result<(), AppError> {
    wipe_root(&paths::sandbox_root())
}

/// 沙箱这一棵有多大 / 多少文件（界面上的读数，给人"真拷了东西"的证据）。
fn measure_sandbox() -> CopyReport {
    fn walk(dir: &Path, acc: &mut CopyReport) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                walk(&entry.path(), acc);
            } else if meta.is_file() {
                acc.files += 1;
                acc.bytes += meta.len();
            }
        }
    }
    let mut acc = CopyReport::default();
    walk(&paths::sandbox_root(), &mut acc);
    acc
}

fn status(note: Option<String>) -> SandboxStatus {
    let size = measure_sandbox();
    SandboxStatus {
        enabled: paths::sandbox_on(),
        sandbox_root: paths::sandbox_root().display().to_string(),
        ready: sandbox_ready(),
        files: size.files,
        bytes: size.bytes,
        real_presets_root: paths::real_presets_root_path().display().to_string(),
        note,
    }
}

/// 一条动作在测试模式下不许走（发布那一类）。
///
/// 沙箱里的东西只在本机、也不在 git 仓库的索引里 —— 让它走到"提交/推送"那一步
/// 只会把一个说不清的世界推给别人。
pub fn require_real_mode(what: &str) -> Result<(), AppError> {
    if !paths::sandbox_on() {
        return Ok(());
    }
    Err(AppError::invalid_argument(format!(
        "{what}：现在在**测试模式（沙箱）**里，这条路是关着的"
    ))
    .with_detail("沙箱那套只在本机（不在 git 索引里）—— 先关掉测试模式，再发布".to_owned()))
}

/* ---------- 命令 ---------- */

/// 测试模式现状（界面那颗开关 + 那条横幅的读数）。
#[tauri::command]
pub fn wb_sandbox_status() -> Result<SandboxStatus, AppError> {
    crate::ipc::traced("wb_sandbox_status", |_| Ok(status(None)))
}

/// 开 / 关测试模式。
///
/// - **开**：沙箱还不齐就先**从正式拷一份**（`presets/` 整棵 + config 那几个 json），
///   然后把整套数据根切过去；
/// - **关**：只切回来，**沙箱原样留着**（下次回来还在，作者要的"测试的归测试的"）；
/// - 两个方向都**先把内存里的会话收干净**（有未保存的改动 → 如实拒绝，见模块头）。
#[tauri::command]
pub fn wb_sandbox_set(app: AppHandle, enabled: bool) -> Result<SandboxStatus, AppError> {
    crate::ipc::traced("wb_sandbox_set", |_| {
        let before = paths::sandbox_on();
        if enabled == before {
            return Ok(status(Some(if enabled {
                "本来就在测试模式里".to_owned()
            } else {
                "本来就在正式里".to_owned()
            })));
        }
        /* ★ 有未保存的改动就不许切：草稿也是两套，吞掉它是最不该发生的事 */
        if let Some(n) = super::dirty_count_now() {
            if n > 0 {
                return Err(AppError::invalid_argument(format!(
                    "还有 {n} 处未保存的改动 —— 先保存或放弃，再切{}",
                    if enabled { "测试模式" } else { "回正式" }
                ))
                .with_detail("草稿是两套：切过去之后未保存的改动不会跟着走".to_owned()));
            }
        }

        let mut note = None;
        if enabled {
            let copied = if sandbox_ready() {
                CopyReport::default()
            } else {
                refill_now()?
            };
            if copied.files > 0 {
                note = Some(format!(
                    "已从正式拷了一份：{} 个文件 {:.1} MB —— 之后所有读写都在这份副本上",
                    copied.files,
                    copied.bytes as f64 / (1024.0 * 1024.0)
                ));
            }
        }

        paths::set_sandbox(enabled);
        write_mode(&app, enabled)?;
        /* 标题栏也要跟着走：它是唯一"切到别的窗口还看得见"的那一处
        （任务栏 / Alt+Tab / 截图里都在）—— 界面里那一圈换装看不见的时候还有它 */
        crate::workbench::refresh_window_title(&app);
        /* 根换了：把旧会话丢掉，下一条命令按新根重建 */
        super::reset_session();
        tracing::warn!(
            enabled,
            root = %paths::presets_root_path().display(),
            "测试模式已切换（数据根跟着换）"
        );
        Ok(status(note))
    })
}

/// **清空沙箱，再从正式拷一份过去**（作者要的那一步：
/// 「我测试的随时可以把这些清空，从正式的复制一份过去，再进行测试」）。
#[tauri::command]
pub fn wb_sandbox_refill() -> Result<SandboxStatus, AppError> {
    crate::ipc::traced("wb_sandbox_refill", |_| {
        wipe()?;
        let copied = refill_now()?;
        super::reset_session();
        Ok(status(Some(format!(
            "已清空、并从正式重新拷了一份：{} 个文件 {:.1} MB",
            copied.files,
            copied.bytes as f64 / (1024.0 * 1024.0)
        ))))
    })
}

/// 清空沙箱（**留在当前模式**；沙箱空了之后，测试模式下工作台会如实说"读不到预设根"，
/// 点「从正式拷一份」就能继续）。
#[tauri::command]
pub fn wb_sandbox_wipe() -> Result<SandboxStatus, AppError> {
    crate::ipc::traced("wb_sandbox_wipe", |_| {
        wipe()?;
        super::reset_session();
        Ok(status(Some(
            "沙箱已清空 —— 想继续测就点「从正式拷一份」".to_owned(),
        )))
    })
}

#[cfg(test)]
mod tests {
    // 判据要造真实沙箱目录 —— 生产代码的写盘走 `fsx::atomic` 那一个洞。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    fn tree(root: &Path, files: &[&str]) {
        for rel in files {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, rel.as_bytes()).unwrap();
        }
    }

    /// 整棵拷：目录结构、字节、以及文件数 / 字节数那个读数
    #[test]
    fn copy_tree_takes_the_whole_tree() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("from");
        let to = d.path().join("to");
        tree(&from, &["a.toml", "sub/b.json", "sub/deep/c.3mf"]);
        let r = copy_tree(&from, &to, &[]).unwrap();
        assert_eq!(r.files, 3);
        assert!(r.bytes > 0);
        assert_eq!(
            std::fs::read_to_string(to.join("sub/deep/c.3mf")).unwrap(),
            "sub/deep/c.3mf"
        );
    }

    /// 拷配方本时要**甩掉那三个点号目录**（本机会话状态不带过去）——
    /// 别的（配置 / 台账）一个不落
    #[test]
    fn copy_tree_can_skip_the_session_state() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("wb");
        let to = d.path().join("cpy");
        tree(
            &from,
            &[
                "built.json",
                "bootstrap.json",
                ".draft/book.json",
                ".trash/x.json",
                ".snapshots/y.json",
            ],
        );
        let r = copy_tree(&from, &to, &SESSION_DIRS).unwrap();
        assert_eq!(r.files, 2, "只该拷那两个 .json");
        assert!(to.join("built.json").is_file());
        assert!(to.join("bootstrap.json").is_file());
        for gone in SESSION_DIRS {
            assert!(!to.join(gone).exists(), "{gone} 不该被带过去");
        }
    }

    /// 拷是**加**上去，不删目标里已有的东西（删是 [`wipe`] 的事 —— 两件事分开）
    #[test]
    fn copy_tree_never_deletes_anything() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("from");
        let to = d.path().join("to");
        tree(&from, &["a.toml"]);
        tree(&to, &["only-here.txt"]);
        copy_tree(&from, &to, &[]).unwrap();
        assert!(to.join("only-here.txt").is_file(), "拷不许顺手删东西");
        assert!(to.join("a.toml").is_file());
    }

    /// **从正式拷一份**：`presets/` 整棵（含 3mf / delivery）+ `workbench/*.json`，
    /// 会话状态那三样不带，**源目录一字不动**
    #[test]
    fn refill_takes_the_presets_tree_and_the_config_json_only() {
        let d = tempfile::tempdir().unwrap();
        let real_presets = d.path().join("presets");
        let real_wb = d.path().join("workbench");
        let dest = d.path().join(".sandbox");
        tree(
            &real_presets,
            &[
                "registry/param_registry.toml",
                "machines/A1.toml",
                "assets/models/x.3mf",
                "delivery/mkp/presets/A1-standard.toml",
            ],
        );
        tree(
            &real_wb,
            &[
                "bootstrap.json",
                "built.json",
                ".draft/book.json",
                ".trash/y.json",
            ],
        );

        let r = refill_from(&real_presets, &real_wb, &dest).unwrap();
        assert_eq!(r.files, 6, "4 份 presets + 2 份 json，实测 {r:?}");
        for rel in [
            "presets/registry/param_registry.toml",
            "presets/assets/models/x.3mf",
            "presets/delivery/mkp/presets/A1-standard.toml",
            "workbench/bootstrap.json",
        ] {
            assert!(dest.join(rel).is_file(), "沙箱里少了 {rel}");
        }
        assert!(
            !dest.join("workbench/.draft").exists(),
            "会话状态不该带过去"
        );
        assert!(
            real_presets.join("machines/A1.toml").is_file()
                && real_wb.join(".draft/book.json").is_file(),
            "拷不许动源那边"
        );
    }

    /// 正式那边压根没有 `presets/` → 如实拒绝（别静默拷出一个空沙箱，
    /// 那会让人以为"测试模式开了、其实里面什么都没有"）
    #[test]
    fn refill_refuses_when_the_real_tree_is_missing() {
        let d = tempfile::tempdir().unwrap();
        let e = refill_from(
            &d.path().join("nope"),
            &d.path().join("wb"),
            &d.path().join("dest"),
        )
        .unwrap_err();
        assert!(e.message.contains("没东西可拷"), "{}", e.message);
    }

    /// 清空**只删你指的那一棵**（"随时可以清空"不能顺手波及正式）
    #[test]
    fn wipe_removes_only_the_tree_you_point_at() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        let b = d.path().join("b");
        tree(&a, &["x/1.toml"]);
        tree(&b, &["y/2.toml"]);
        wipe_root(&a).unwrap();
        assert!(!a.exists());
        assert!(b.join("y/2.toml").is_file(), "别人那一棵不许动");
        wipe_root(&a).unwrap(); // 幂等：本来就没有也算清干净了
    }

    /// 沙箱根就在 `workbench/` 底下、点号打头（与 `.draft` 同一族，`.gitignore` 一行挡住）
    #[test]
    fn the_sandbox_lives_under_the_workbench_dir() {
        let root = paths::sandbox_root();
        assert_eq!(root.file_name().unwrap(), ".sandbox");
        assert_eq!(root.parent().unwrap().file_name().unwrap(), "workbench");
        assert_eq!(sandbox_presets(), root.join("presets"));
        /* 与仓库那边**同形**：一个 `presets/`、一个 `workbench/` */
        assert_eq!((PRESETS, WORKBENCH), ("presets", "workbench"));
    }

    /// 测试模式里那几条"只在本机"的动作要被拦住；正式里一路放行
    #[test]
    fn publishing_is_refused_only_in_the_sandbox() {
        assert!(require_real_mode("不许发布").is_ok(), "正式里不该拦");
    }
}
