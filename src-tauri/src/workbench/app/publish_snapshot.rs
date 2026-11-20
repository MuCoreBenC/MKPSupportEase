//! 「发布快照」—— 本地测试源里的**真发布演练**。
//!
//! # 它解决的那件事
//!
//! 想验「云端发了新版 → 客户端检查更新 → 下载 → 安装 → 重启后显示新版本号」整条真链，
//! 又不想真发到云端。这一层把这件事做成一次**本地的、可完整回退的**发布：
//!
//! ```text
//! 拍快照 ► 登记现场（五个版本文件 + 覆盖层 + 编译产物）并收进 target/local-publish-snapshot/
//! 模拟发布 ► 抬下一个预览号 → 真编译本平台安装包 → 写本地 release-preview.json → 后台完成
//! 还原 ► 按登记表把现场写回；模拟期间新冒出来的产物一律删掉
//! ```
//!
//! 客户端从 `http://127.0.0.1:8787/delivery` 拿到的是**真编出来的包**，走的是 catalog /
//! SHA / 下载 / 安装那条真链，只是"云端"在本机。`presets/delivery/` 一个字节不动，
//! 不打 tag、不推远端、不建 Release。
//!
//! # 为什么必须收走编译产物
//!
//! 抬号会改五个版本文件，真编会覆盖 client exe / payload / 安装器，并新增一支带版本号的
//! setup。这些都必须登记"模拟前在不在、原来是什么字节"，还原才有意义 —— 用户要的是
//! "模拟之前的样子"，不是"版本号回去了但盘上多了一堆新包"。
//!
//! 版本文件与固定路径的小文件用**拷贝**（编译期间要能读到），目录用 **rename**（同盘 O(1)，
//! 不占双份空间）并记下是否真的搬走了；还原时按登记表写回或删除。

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

/// 快照放在 `target/` 下（整个 `target` 已被 git 忽略）。
const SNAPSHOT_DIR: &str = "local-publish-snapshot";
/// 快照里的产物寄存处。
const STORE_DIR: &str = "store";
/// `server.mjs` 的本地发布覆盖层目录。
const LOCAL_RELEASE_DIR: &str = "local-release";
const SNAPSHOT_MANIFEST: &str = "snapshot.json";
/// 真编过程的日志（放在快照外面，还原后还能查）。
const APPLY_LOG: &str = "publish-snapshot-apply.log";
/// 真编进度（落盘，工作台重启后还能读到）。
const APPLY_STATE: &str = "apply.json";
/// 客户端「重新检查」读的预览通道清单。
const RELEASE_PREVIEW: &str = "release-preview.json";
const SCHEMA: u32 = 2;

/* ---------- DTO ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishSnapshotStatus {
    pub active: bool,
    pub taken_at: Option<String>,
    pub base_version: Option<String>,
    pub simulated_version: Option<String>,
    pub applied: bool,
    pub has_package: bool,
    pub package: Option<String>,
    pub note: String,
}

/// 真编那一趟的进度。**落在盘上、前端轮询**，不阻塞界面。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotApplyProgress {
    pub running: bool,
    /// `preparing` / `bumping` / `building` / `manifest` / `done` / `failed` / `interrupted`
    pub stage: String,
    /// 编译日志的最后一行
    pub tail: String,
    pub finished: Option<String>,
    pub error: Option<String>,
    /// 跑这一趟的子进程 PID；重启后据此判断它是不是中途没了
    #[serde(default)]
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntryKind {
    File,
    Dir,
}

/// 一个被管的路径：模拟前有还是没有、原来那份收在哪。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedEntry {
    /// 相对仓库根的路径（用 `/` 存，跨平台同一份快照）
    path: String,
    kind: EntryKind,
    existed: bool,
    /// 快照寄存处里的文件名；`existed=false` 时没有
    stored: Option<String>,
    /// 目录是不是整棵搬走的（失败则退回拷贝）
    moved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotManifest {
    schema: u32,
    taken_at: String,
    base_version: String,
    #[serde(default)]
    simulated_version: Option<String>,
    #[serde(default)]
    overlay_version: Option<String>,
    #[serde(default)]
    package: Option<String>,
    /// 上一趟模拟半途失败的那一句；成功 / 拍快照时清掉
    #[serde(default)]
    last_error: Option<String>,
    entries: Vec<ManagedEntry>,
}

/* ---------- 进度（落盘） ---------- */

fn progress_path(repo: &Path) -> PathBuf {
    snapshot_dir(repo).join(APPLY_STATE)
}

fn idle_progress() -> SnapshotApplyProgress {
    SnapshotApplyProgress {
        running: false,
        stage: "idle".to_owned(),
        tail: String::new(),
        finished: None,
        error: None,
        pid: None,
    }
}

/// 写一次进度。落盘是唯一真相：跑编译的是 detached 子进程，工作台随时可能不在。
fn write_progress(repo: &Path, p: &SnapshotApplyProgress) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(p)
        .map_err(|e| AppError::io("进度写不出来").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&progress_path(repo), format!("{text}\n").as_bytes())
        .map_err(|e| AppError::io("写不进编译进度").with_detail(e.message))
}

fn read_progress_file(repo: &Path) -> Option<SnapshotApplyProgress> {
    let text = fs::read_to_string(progress_path(repo)).ok()?;
    serde_json::from_str(&text).ok()
}

/// 进程还活着吗。`sysinfo` 已在用（dev_source 同一套），不引新依赖。
fn pid_alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.process(Pid::from_u32(pid)).is_some()
}

/// 界面读到的那一份。核心：盘上说在跑、但那个 PID 已经不在 ⇒ 如实报 `interrupted`，
/// 不能让界面一直显示"正在编译"却永远等不到结果。
fn progress_for_ui(repo: &Path) -> SnapshotApplyProgress {
    let Some(mut p) = read_progress_file(repo) else {
        return idle_progress();
    };
    if p.running {
        match p.pid {
            Some(pid) if pid_alive(pid) => {}
            Some(_) => {
                p.running = false;
                p.stage = "interrupted".to_owned();
                p.finished = Some("interrupted".to_owned());
                p.error = Some("工作台退出，编译子进程也不在了。点「还原」回到模拟前。".to_owned());
            }
            None => {
                p.running = false;
                p.stage = "interrupted".to_owned();
                p.finished = Some("interrupted".to_owned());
                p.error =
                    Some("这一趟没有 PID 记录，按中断处理。点「还原」回到模拟前。".to_owned());
            }
        }
    }
    p
}

fn progress_stage(repo: &Path, pid: u32, stage: &str, tail: &str) {
    let _ = write_progress(
        repo,
        &SnapshotApplyProgress {
            running: true,
            stage: stage.to_owned(),
            tail: tail.to_owned(),
            finished: None,
            error: None,
            pid: Some(pid),
        },
    );
}

/* ---------- 路径 ---------- */

fn target_dir(repo: &Path) -> PathBuf {
    repo.join("target")
}

fn snapshot_dir(repo: &Path) -> PathBuf {
    target_dir(repo).join(SNAPSHOT_DIR)
}

fn store_dir(repo: &Path) -> PathBuf {
    snapshot_dir(repo).join(STORE_DIR)
}

fn local_release_dir(repo: &Path) -> PathBuf {
    target_dir(repo).join(LOCAL_RELEASE_DIR)
}

fn manifest_path(snap: &Path) -> PathBuf {
    snap.join(SNAPSHOT_MANIFEST)
}

fn read_manifest(snap: &Path) -> Option<SnapshotManifest> {
    let text = fs::read_to_string(manifest_path(snap)).ok()?;
    let m: SnapshotManifest = serde_json::from_str(&text).ok()?;
    (m.schema == SCHEMA).then_some(m)
}

fn write_manifest(snap: &Path, m: &SnapshotManifest) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(m)
        .map_err(|e| AppError::io("快照说明书写不出来").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&manifest_path(snap), format!("{text}\n").as_bytes())
        .map_err(|e| AppError::io("写不进快照说明书").with_detail(e.message))
}

/* ---------- 目录搬运 ---------- */

fn copy_tree(from: &Path, to: &Path) -> Result<(), AppError> {
    if !from.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(to)
        .map_err(|e| AppError::io("建不了快照目录").with_detail(e.to_string()))?;
    let entries = fs::read_dir(from)
        .map_err(|e| AppError::io("读不了快照源目录").with_detail(e.to_string()))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|e| AppError::io("认不出快照里的文件").with_detail(e.to_string()))?;
        if kind.is_dir() {
            copy_tree(&src, &dst)?;
        } else if kind.is_file() {
            fs::copy(&src, &dst).map_err(|e| {
                AppError::io(format!("拷不动 {}", src.display())).with_detail(e.to_string())
            })?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else if path.is_file() || path.is_symlink() {
        let _ = fs::remove_file(path);
    }
}

/// 需要登记的现场。路径写死在这里 —— 哪些是模拟会动的东西，答案只有这一份。
fn managed_specs() -> Vec<(String, EntryKind)> {
    let exe = if cfg!(windows) { ".exe" } else { "" };
    vec![
        ("version.toml".to_owned(), EntryKind::File),
        ("package.json".to_owned(), EntryKind::File),
        ("src-tauri/tauri.conf.json".to_owned(), EntryKind::File),
        ("src-tauri/Cargo.toml".to_owned(), EntryKind::File),
        ("Cargo.lock".to_owned(), EntryKind::File),
        ("installer/payload.bin".to_owned(), EntryKind::File),
        ("installer/payload.xz".to_owned(), EntryKind::File),
        (
            format!("target/release/mkp-support-ease{exe}"),
            EntryKind::File,
        ),
        (
            format!("target-installer/release/pack{exe}"),
            EntryKind::File,
        ),
        (
            format!("target-installer/release/mkp-installer{exe}"),
            EntryKind::File,
        ),
        (
            format!("target-installer/release/mkp-uninstaller{exe}"),
            EntryKind::File,
        ),
        (format!("target/{LOCAL_RELEASE_DIR}"), EntryKind::Dir),
        ("target/release/bundle".to_owned(), EntryKind::Dir),
    ]
}

fn store_name(idx: usize, rel: &str) -> String {
    let leaf = rel
        .rsplit('/')
        .next()
        .unwrap_or("item")
        .replace(['/', '\\'], "_");
    format!("{idx:03}_{leaf}")
}

fn backup_entry(
    repo: &Path,
    store: &Path,
    idx: usize,
    rel: &str,
    kind: EntryKind,
) -> Result<ManagedEntry, AppError> {
    let path = repo.join(rel);
    if !path.exists() {
        return Ok(ManagedEntry {
            path: rel.to_owned(),
            kind,
            existed: false,
            stored: None,
            moved: false,
        });
    }
    let stored = store_name(idx, rel);
    let dest = store.join(&stored);
    let moved = if kind == EntryKind::Dir {
        match fs::rename(&path, &dest) {
            Ok(()) => true,
            Err(_) => {
                copy_tree(&path, &dest)?;
                false
            }
        }
    } else {
        fs::copy(&path, &dest).map_err(|e| {
            AppError::io(format!("存不了 {}", path.display())).with_detail(e.to_string())
        })?;
        false
    };
    Ok(ManagedEntry {
        path: rel.to_owned(),
        kind,
        existed: true,
        stored: Some(stored),
        moved,
    })
}

fn restore_entry(repo: &Path, store: &Path, entry: &ManagedEntry) -> Result<(), AppError> {
    let path = repo.join(&entry.path);
    let Some(stored) = entry.stored.as_deref() else {
        remove_path(&path);
        return Ok(());
    };
    let src = store.join(stored);
    if !src.exists() {
        return Err(AppError::invalid_argument(format!(
            "快照里缺了 {}，拒绝半途还原",
            entry.path
        )));
    }
    remove_path(&path);
    if entry.kind == EntryKind::Dir && entry.moved {
        fs::rename(&src, &path).map_err(|e| {
            AppError::io(format!("还原不了 {}", entry.path)).with_detail(e.to_string())
        })
    } else if entry.kind == EntryKind::Dir {
        copy_tree(&src, &path)
    } else {
        fs::copy(&src, &path).map(|_| ()).map_err(|e| {
            AppError::io(format!("还原不了 {}", entry.path)).with_detail(e.to_string())
        })
    }
}

/* ---------- 版本 / 地址 ---------- */

fn dev_version(repo: &Path) -> Result<String, AppError> {
    let text = fs::read_to_string(repo.join("version.toml"))
        .map_err(|e| AppError::io("读不到 version.toml").with_detail(e.to_string()))?;
    version_in(&text).ok_or_else(|| AppError::invalid_argument("version.toml 里没有 version 字段"))
}

fn version_in(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_owned)
}

fn set_dev_version(repo: &Path, version: &str) -> Result<(), AppError> {
    let path = repo.join("version.toml");
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::io("读不到 version.toml").with_detail(e.to_string()))?;
    let mut out = String::with_capacity(text.len() + 8);
    let mut changed = false;
    for line in text.lines() {
        if !changed && line.starts_with("version = \"") {
            out.push_str(&format!("version = \"{version}\""));
            changed = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !changed {
        return Err(AppError::invalid_argument(
            "version.toml 里没有 version 字段",
        ));
    }
    crate::fsx::atomic::atomic_write(&path, out.as_bytes())
        .map_err(|e| AppError::io("写不进 version.toml").with_detail(e.message))
}

fn json_version(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("version")?.as_str().map(str::to_owned)
}

/// 严格更大的下一个预览号（与 `release_info::is_newer_version` 同口径）。
fn next_preview(raw: &str) -> Option<String> {
    let (major, minor, patch, kind, seq) = parse_version(raw)?;
    let rank = match kind.as_deref() {
        Some("preview") | Some("pre") => 3,
        Some("rc") => 4,
        Some("dev") => 0,
        Some("alpha") | Some("a") => 1,
        Some("beta") | Some("b") => 2,
        None => 5,
        Some(_) => return None,
    };
    if rank == 3 {
        Some(format!("{major}.{minor}.{patch}-preview.{}", seq + 1))
    } else if rank >= 4 {
        Some(format!("{major}.{minor}.{}-preview.1", patch + 1))
    } else {
        Some(format!("{major}.{minor}.{patch}-preview.1"))
    }
}

fn parse_version(raw: &str) -> Option<(u32, u32, u32, Option<String>, u32)> {
    let raw = raw.trim();
    let (head, tail) = match raw.split_once('-') {
        Some((h, t)) => (h, Some(t)),
        None => (raw, None),
    };
    let mut nums = head.split('.');
    let major = nums.next()?.parse().ok()?;
    let minor = nums.next()?.parse().ok()?;
    let patch = nums.next()?.parse().ok()?;
    if nums.next().is_some() {
        return None;
    }
    match tail {
        None => Some((major, minor, patch, None, 0)),
        Some(t) => {
            let t = t.trim_start_matches('.');
            let digits = t.bytes().rev().take_while(|b| b.is_ascii_digit()).count();
            if digits == 0 {
                Some((major, minor, patch, Some(t.to_ascii_lowercase()), 0))
            } else {
                let at = t.len() - digits;
                Some((
                    major,
                    minor,
                    patch,
                    Some(t[..at].trim_end_matches('.').to_ascii_lowercase()),
                    t[at..].parse().ok()?,
                ))
            }
        }
    }
}

fn origin() -> String {
    let port = std::env::var("PRESET_TEST_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(8787);
    format!("http://127.0.0.1:{port}")
}

fn sha256_file(path: &Path) -> Result<String, AppError> {
    let bytes = fs::read(path).map_err(|e| {
        AppError::io(format!("读不了 {}", path.display())).with_detail(e.to_string())
    })?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/* ---------- 真编那一趟 ---------- */

fn npm_command(args: &[&str]) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("npm").args(args);
        c
    } else {
        let mut c = Command::new("npm");
        c.args(args);
        c
    }
}

type SharedLog = Arc<Mutex<Vec<String>>>;

fn log_push(log: &SharedLog, line: &str) {
    if let Ok(mut lines) = log.lock() {
        lines.push(line.to_owned());
        if lines.len() > 400 {
            lines.remove(0);
        }
    }
}

/// 跑一条命令，把两路输出并进日志与进度尾行。**不写日志文件**（clippy 禁 `File::create`），
/// 日志在事务结束时由 `atomic_write` 一次性落盘；进度尾行靠这里实时更新。
fn spawn_logged(
    repo: &Path,
    log: &SharedLog,
    mut cmd: Command,
    label: &str,
) -> Result<(), AppError> {
    let mut child = cmd
        .current_dir(repo)
        /* 工作台由 `tauri dev --config tauri.workbench.conf.json` 起，CLI 把合并后的配置
        放进 `TAURI_CONFIG`。不清掉的话，客户端构建会读到工作台配置（空窗口清单 +
        `SupportEase-Workbench` 标识），编出一颗带工作台标记、装上没窗的二进制。 */
        .env_remove("TAURI_CONFIG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::io(format!("起不了 {label}")).with_detail(e.to_string()))?;
    let mut handles = Vec::new();
    if let Some(stream) = child.stdout.take() {
        let log = Arc::clone(log);
        handles.push(std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                log_push(&log, &line);
            }
        }));
    }
    if let Some(stream) = child.stderr.take() {
        let log = Arc::clone(log);
        handles.push(std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                log_push(&log, &line);
            }
        }));
    }
    let status = child
        .wait()
        .map_err(|e| AppError::io("等不到命令结束").with_detail(e.to_string()))?;
    for h in handles {
        let _ = h.join();
    }
    if status.success() {
        Ok(())
    } else {
        Err(AppError::io(format!("{label} 失败（退出码 {status}）")))
    }
}

fn run_logged(
    repo: &Path,
    log: &SharedLog,
    cmd: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<(), AppError> {
    let mut c = Command::new(cmd);
    c.args(args).envs(envs.iter().copied());
    spawn_logged(repo, log, c, &format!("{cmd} {}", args.join(" ")))
}

fn run_npm_logged(
    repo: &Path,
    log: &SharedLog,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<(), AppError> {
    let mut c = npm_command(args);
    c.envs(envs.iter().copied());
    spawn_logged(repo, log, c, &format!("npm {}", args.join(" ")))
}

fn write_apply_log(repo: &Path, log: &SharedLog) {
    let text = log
        .lock()
        .map(|lines| format!("{}\n", lines.join("\n")))
        .unwrap_or_default();
    let path = target_dir(repo).join(APPLY_LOG);
    let _ = crate::fsx::atomic::atomic_write(&path, text.as_bytes());
}

/// Windows：`npm run build:installer` 造出自研安装器，写一份 `nsis` 清单指到它。
fn write_windows_manifest(repo: &Path, version: &str) -> Result<String, AppError> {
    let bundle = repo.join("target/release/bundle/nsis");
    let setup = fs::read_dir(&bundle)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("exe")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("SupportEase"))
        })
        .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .ok_or_else(|| AppError::io("编完了，但 bundle/nsis 下没有安装包"))?;

    let name = setup
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::io("安装包名字读不出来"))?
        .to_owned();
    let live = local_release_dir(repo);
    fs::create_dir_all(&live)
        .map_err(|e| AppError::io("建不了本地发布目录").with_detail(e.to_string()))?;
    let dest = live.join(&name);
    if setup != dest {
        fs::copy(&setup, &dest)
            .map_err(|e| AppError::io("拷不了安装包").with_detail(e.to_string()))?;
    }
    let size = fs::metadata(&dest)
        .map_err(|e| AppError::io("读不了安装包大小").with_detail(e.to_string()))?
        .len();
    let digest = sha256_file(&dest)?;
    let manifest = serde_json::json!({
        "releaseSchema": 1,
        "version": version,
        "notes": format!("本地快照模拟发布 {version}"),
        "url": format!("{}/delivery/{RELEASE_PREVIEW}", origin()),
        "asset": {
            "name": name,
            "url": format!("{}/delivery/{name}", origin()),
            "size": size,
            "sha256": digest,
            "platform": "windows",
            "kind": "nsis",
        },
    });
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|e| AppError::io("清单写不出来").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&live.join(RELEASE_PREVIEW), format!("{text}\n").as_bytes())
        .map_err(|e| AppError::io("写不进模拟清单").with_detail(e.message))?;
    let history = live.join("history");
    fs::create_dir_all(&history)
        .map_err(|e| AppError::io("建不了清单历史目录").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(
        &history.join(format!("{version}.json")),
        format!("{text}\n").as_bytes(),
    )
    .map_err(|e| AppError::io("写不进历史清单").with_detail(e.message))?;
    Ok(name)
}

/// macOS：`local-release.mjs` 已经按新包的自报号写好清单，这里只核对号。
fn verify_macos_manifest(repo: &Path, version: &str) -> Result<String, AppError> {
    let path = local_release_dir(repo).join(RELEASE_PREVIEW);
    let got = json_version(&path).ok_or_else(|| AppError::io("本地发布清单没写出来"))?;
    if got != version {
        return Err(AppError::io(format!(
            "清单号 {got} 与这次编的包 {version} 不一致"
        )));
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::io("读不了清单").with_detail(e.to_string()))?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| AppError::io("清单不是 JSON").with_detail(e.to_string()))?;
    let name = v
        .get("asset")
        .and_then(|a| a.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("—")
        .to_owned();
    Ok(name)
}

/// 跑在 detached 子进程里的那一趟。进度全程写盘，不依赖调用方进程还活着。
/// 给 bin 入口用：`snapshot-apply <repo>`。
pub fn run_apply_child(repo: PathBuf) {
    apply_transaction(repo);
}

fn apply_transaction(repo: PathBuf) {
    let pid = std::process::id();
    let log: SharedLog = Arc::new(Mutex::new(Vec::new()));
    let result = (|| -> Result<(String, String), AppError> {
        progress_stage(&repo, pid, "preparing", "读快照");
        let snap = snapshot_dir(&repo);
        let mut m = read_manifest(&snap)
            .ok_or_else(|| AppError::invalid_argument("快照不在了或被改坏了"))?;
        let version = next_preview(&m.base_version).ok_or_else(|| {
            AppError::invalid_argument(format!("版本号认不出：{}", m.base_version))
        })?;

        progress_stage(&repo, pid, "bumping", &format!("抬到 {version}"));
        set_dev_version(&repo, &version)?;
        run_logged(&repo, &log, "node", &["scripts/version.mjs", "sync"], &[])?;
        run_logged(&repo, &log, "node", &["scripts/version.mjs", "check"], &[])?;

        progress_stage(&repo, pid, "building", "编译安装包（这一步最久）");
        /* 本地演练在工作区里跑，源码本来就是脏的；只给这一趟开 `MKPSE_LOCAL_SNAPSHOT`，
        tauri-dev 的脏树闸（发版用）因此不动，工作台标记那道闸仍在。 */
        let env = [("MKPSE_LOCAL_SNAPSHOT", "1")];
        if cfg!(target_os = "windows") {
            run_npm_logged(&repo, &log, &["run", "build:installer"], &env)?;
        } else if cfg!(target_os = "macos") {
            run_npm_logged(&repo, &log, &["run", "tauri", "--", "build"], &env)?;
            run_npm_logged(&repo, &log, &["run", "local-release"], &env)?;
        } else {
            return Err(AppError::invalid_argument("本机不支持本地安装包构建"));
        }

        progress_stage(&repo, pid, "manifest", "写本地清单");
        let package = if cfg!(target_os = "windows") {
            write_windows_manifest(&repo, &version)?
        } else {
            verify_macos_manifest(&repo, &version)?
        };
        m.simulated_version = Some(version.clone());
        m.overlay_version = Some(version.clone());
        m.package = Some(package.clone());
        m.last_error = None;
        write_manifest(&snap, &m)?;
        Ok((version, package))
    })();

    write_apply_log(&repo, &log);
    if let Err(e) = &result {
        let snap = snapshot_dir(&repo);
        if let Some(mut m) = read_manifest(&snap) {
            m.last_error = Some(e.message.clone());
            let _ = write_manifest(&snap, &m);
        }
    }
    let mut p = idle_progress();
    p.pid = Some(pid);
    match result {
        Ok((version, package)) => {
            p.stage = "done".to_owned();
            p.finished = Some("done".to_owned());
            p.tail = format!("{version} 已就绪：{package}");
            tracing::info!(version = %version, "本地快照模拟发布完成");
        }
        Err(e) => {
            p.stage = "failed".to_owned();
            p.finished = Some("failed".to_owned());
            p.error = Some(e.message.clone());
            p.tail = e.message;
        }
    }
    let _ = write_progress(&repo, &p);
}

/* ---------- 状态 ---------- */

fn status_of(repo: &Path) -> PublishSnapshotStatus {
    let snap = snapshot_dir(repo);
    let Some(m) = read_manifest(&snap) else {
        return PublishSnapshotStatus {
            active: false,
            taken_at: None,
            base_version: None,
            simulated_version: None,
            applied: false,
            has_package: false,
            package: None,
            note: "没有快照。点「拍快照」先存一份当前状态。".to_owned(),
        };
    };
    /* 中断残留：快照在、没写 simulated，但版本已经被抬走了 —— 老实说出来，
    不能让人对着"还没模拟发布"却看到版本号不对的界面猜。 */
    let version_moved = dev_version(repo).ok().is_some_and(|v| v != m.base_version);
    let note = if let Some(err) = m.last_error.as_deref() {
        format!("上一趟没编成：{err}。点「还原」回到模拟前。")
    } else if m.simulated_version.is_none() && version_moved {
        format!(
            "上次编译在半途停了，版本已从 {} 抬走。点「还原」回到模拟前。",
            m.base_version
        )
    } else {
        match m.simulated_version.as_deref() {
            Some(v) => format!("已模拟发布 {v}；点「还原」回到模拟前。"),
            None => "快照已就绪，还没模拟发布。".to_owned(),
        }
    };
    PublishSnapshotStatus {
        active: true,
        taken_at: Some(m.taken_at),
        base_version: Some(m.base_version),
        simulated_version: m.simulated_version.clone(),
        applied: m.simulated_version.is_some(),
        has_package: m.package.is_some(),
        package: m.package,
        note,
    }
}

/* ---------- 命令 ---------- */

#[tauri::command]
pub fn wb_publish_snapshot_status() -> Result<PublishSnapshotStatus, AppError> {
    crate::ipc::traced("wb_publish_snapshot_status", |_| {
        Ok(status_of(&crate::workbench::paths::repo_root()))
    })
}

#[tauri::command]
pub fn wb_publish_snapshot_progress() -> Result<SnapshotApplyProgress, AppError> {
    crate::ipc::traced("wb_publish_snapshot_progress", |_| {
        Ok(progress_for_ui(&crate::workbench::paths::repo_root()))
    })
}

/// 拍一份完整现场。已有快照时拒绝 —— 覆盖它等于把"模拟前"丢了。
#[tauri::command]
pub fn wb_publish_snapshot_take() -> Result<PublishSnapshotStatus, AppError> {
    crate::ipc::traced("wb_publish_snapshot_take", |_| {
        let repo = crate::workbench::paths::repo_root();
        let snap = snapshot_dir(&repo);
        if read_manifest(&snap).is_some() {
            return Err(AppError::invalid_argument(
                "已经有快照了。先点「还原」再拍新的。",
            ));
        }
        if progress_for_ui(&repo).running {
            return Err(AppError::invalid_argument("模拟发布还在跑，等它结束。"));
        }
        let live = local_release_dir(&repo);
        let overlay_version = json_version(&live.join(RELEASE_PREVIEW));
        let base_version = dev_version(&repo)?;
        let package = fs::read_dir(&live)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                let n = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
                n.ends_with(".zip") || n.ends_with(".exe")
            })
            .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
            .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_owned));

        clear_snapshot(&repo);
        let store = store_dir(&repo);
        fs::create_dir_all(&store)
            .map_err(|e| AppError::io("建不了快照目录").with_detail(e.to_string()))?;
        let mut entries = Vec::new();
        for (idx, (rel, kind)) in managed_specs().into_iter().enumerate() {
            entries.push(backup_entry(&repo, &store, idx, &rel, kind)?);
        }
        write_manifest(
            &snap,
            &SnapshotManifest {
                schema: SCHEMA,
                taken_at: crate::workbench::clock::now_iso8601(),
                base_version,
                simulated_version: None,
                overlay_version,
                package,
                last_error: None,
                entries,
            },
        )?;
        let _ = write_progress(&repo, &idle_progress());
        Ok(status_of(&repo))
    })
}

/// 派一趟真编到**独立进程**（`snapshot-apply`），立刻返回。
///
/// 编译要好几分钟，还改工作区；放在独立进程里有两个好处：工作台崩了它照样跑完，
/// 进度写盘；工作台重启后读盘就知道走到哪一步。用 `cargo run` 起它（这个 bin 带
/// workbench feature，默认构建编不到）——只在工作台开发期用。
#[tauri::command]
pub fn wb_publish_snapshot_apply() -> Result<PublishSnapshotStatus, AppError> {
    crate::ipc::traced("wb_publish_snapshot_apply", |_| {
        let repo = crate::workbench::paths::repo_root();
        let snap = snapshot_dir(&repo);
        read_manifest(&snap)
            .ok_or_else(|| AppError::invalid_argument("没有快照。先点「拍快照」。"))?;
        if progress_for_ui(&repo).running {
            return Err(AppError::invalid_argument("这一趟还在跑。"));
        }
        /* 先把进程起起来，拿到 **真实 PID** 再写进度。
        反过来（先写 running 再起进程）会留出一个没有 PID 的窗口，而读的人把
        "running 却没有 PID" 当成中断 —— 一点下去界面就报"中断"，看着像没反应。 */
        let pid = spawn_apply_child(&repo)?;
        let mut p = idle_progress();
        p.running = true;
        p.stage = "preparing".to_owned();
        p.tail = "正在启动编译进程".to_owned();
        p.pid = Some(pid);
        write_progress(&repo, &p)?;
        Ok(status_of(&repo))
    })
}

/// 起 detached 的 `snapshot-apply`，返回它的 PID（进度要用它判活）。
/// Windows 上 npm/cargo 经 cmd 垫片，加 `CREATE_NO_WINDOW`。
fn spawn_apply_child(repo: &Path) -> Result<u32, AppError> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("cargo");
        c
    } else {
        Command::new("cargo")
    };
    cmd.args([
        "run",
        "--quiet",
        "--features",
        "workbench",
        "--bin",
        "snapshot-apply",
        "--",
    ])
    .arg(repo)
    .current_dir(repo)
    /* 工作台的 `TAURI_CONFIG` 不进这一支：它会让里头的 tauri build 合并工作台配置 */
    .env_remove("TAURI_CONFIG")
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0800_0000); // DETACHED_PROCESS | CREATE_NO_WINDOW
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child = cmd
        .spawn()
        .map_err(|e| AppError::io("起不了编译子进程").with_detail(e.to_string()))?;
    Ok(child.id())
}

/// **丢掉快照**：不还原，直接删掉登记表与寄存的产物。
///
/// 用途：模拟发布成功、这一版就是要留下继续做，不需要回到旧状态 —— 把旧快照一丢，
/// 再拍一份新的，基准就是现在这一版。
#[tauri::command]
pub fn wb_publish_snapshot_discard() -> Result<PublishSnapshotStatus, AppError> {
    crate::ipc::traced("wb_publish_snapshot_discard", |_| {
        let repo = crate::workbench::paths::repo_root();
        read_manifest(&snapshot_dir(&repo))
            .ok_or_else(|| AppError::invalid_argument("没有快照可清理。"))?;
        if progress_for_ui(&repo).running {
            return Err(AppError::invalid_argument("这一趟还在跑，等它结束再清理。"));
        }
        clear_snapshot(&repo);
        Ok(status_of(&repo))
    })
}

#[tauri::command]
pub fn wb_publish_snapshot_restore() -> Result<PublishSnapshotStatus, AppError> {
    crate::ipc::traced("wb_publish_snapshot_restore", |_| {
        let repo = crate::workbench::paths::repo_root();
        restore(&repo)?;
        Ok(status_of(&repo))
    })
}

fn clear_snapshot(repo: &Path) {
    let snap = snapshot_dir(repo);
    if snap.is_dir() {
        let _ = fs::remove_dir_all(&snap);
    }
}

/// 按登记表还原现场：原来有的写回去，原来没有的删掉。
pub fn restore(repo: &Path) -> Result<(), AppError> {
    let snap = snapshot_dir(repo);
    let Some(m) = read_manifest(&snap) else {
        return Ok(());
    };
    if progress_for_ui(repo).running {
        return Err(AppError::invalid_argument("这一趟还在跑，等它结束再还原。"));
    }
    let store = store_dir(repo);
    for entry in &m.entries {
        restore_entry(repo, &store, entry)?;
    }
    clear_snapshot(repo);
    Ok(())
}

/// 停服务时问一句：有快照就还原（界面上写明了"停服务会还原"）。
pub fn restore_on_stop(repo: &Path) -> Option<String> {
    read_manifest(&snapshot_dir(repo))?;
    match restore(repo) {
        Ok(()) => Some("已停，并还原了发布快照".to_owned()),
        Err(e) => Some(format!("已停，但还原快照失败：{}", e.message)),
    }
}

/* ---------- 测试 ---------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_bumps_are_strictly_newer() {
        assert_eq!(
            next_preview("0.0.11-preview.6").as_deref(),
            Some("0.0.11-preview.7")
        );
        assert_eq!(next_preview("0.0.11").as_deref(), Some("0.0.12-preview.1"));
        assert_eq!(
            next_preview("0.0.11-rc.2").as_deref(),
            Some("0.0.12-preview.1")
        );
        assert_eq!(
            next_preview("0.0.11-alpha.1").as_deref(),
            Some("0.0.11-preview.1")
        );
        assert_eq!(next_preview("not-a-version"), None);
    }

    #[test]
    fn restore_brings_back_old_bytes_and_drops_new_files() {
        let base = std::env::temp_dir().join(format!("mkpse-snapshot-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let repo = base.join("repo");
        let old = repo.join("version.toml");
        fs::create_dir_all(repo.join("target/local-release/history")).unwrap();
        crate::fsx::atomic::atomic_write(&old, b"version = \"0.0.11-preview.6\"\n").unwrap();
        crate::fsx::atomic::atomic_write(
            &repo.join("target/local-release/release-preview.json"),
            b"{\"version\":\"0.0.11-preview.6\"}\n",
        )
        .unwrap();

        let snap = snapshot_dir(&repo);
        let store = store_dir(&repo);
        fs::create_dir_all(&store).unwrap();
        let mut entries = Vec::new();
        for (idx, (rel, kind)) in managed_specs().into_iter().enumerate() {
            entries.push(backup_entry(&repo, &store, idx, &rel, kind).unwrap());
        }
        write_manifest(
            &snap,
            &SnapshotManifest {
                schema: SCHEMA,
                taken_at: "2026-11-20T00:00:00Z".to_owned(),
                base_version: "0.0.11-preview.6".to_owned(),
                simulated_version: None,
                overlay_version: None,
                package: None,
                last_error: None,
                entries,
            },
        )
        .unwrap();

        crate::fsx::atomic::atomic_write(&old, b"version = \"0.0.11-preview.7\"\n").unwrap();
        crate::fsx::atomic::atomic_write(
            &repo.join("target/local-release/SupportEase_new.exe"),
            b"new\n",
        )
        .unwrap();

        restore(&repo).unwrap();
        assert_eq!(
            fs::read_to_string(&old).unwrap(),
            "version = \"0.0.11-preview.6\"\n"
        );
        assert!(!repo
            .join("target/local-release/SupportEase_new.exe")
            .exists());
        assert_eq!(
            fs::read_to_string(repo.join("target/local-release/release-preview.json")).unwrap(),
            "{\"version\":\"0.0.11-preview.6\"}\n"
        );
        assert!(!snap.exists());
        let _ = fs::remove_dir_all(&base);
    }

    /// 模拟前没有的产物，还原后也不能留下。
    #[test]
    fn absent_paths_are_removed_on_restore() {
        let base = std::env::temp_dir().join(format!("mkpse-absent-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let repo = base.join("repo");
        fs::create_dir_all(&repo).unwrap();
        let snap = snapshot_dir(&repo);
        let store = store_dir(&repo);
        fs::create_dir_all(&store).unwrap();
        let entry = ManagedEntry {
            path: "target/release/bundle".to_owned(),
            kind: EntryKind::Dir,
            existed: false,
            stored: None,
            moved: false,
        };
        write_manifest(
            &snap,
            &SnapshotManifest {
                schema: SCHEMA,
                taken_at: "2026-11-20T00:00:00Z".to_owned(),
                base_version: "0.0.11-preview.6".to_owned(),
                simulated_version: None,
                overlay_version: None,
                package: None,
                last_error: None,
                entries: vec![entry],
            },
        )
        .unwrap();
        fs::create_dir_all(repo.join("target/release/bundle/nsis")).unwrap();
        crate::fsx::atomic::atomic_write(
            &repo.join("target/release/bundle/nsis/setup.exe"),
            b"new\n",
        )
        .unwrap();
        restore(&repo).unwrap();
        assert!(!repo.join("target/release/bundle").exists());
        let _ = fs::remove_dir_all(&base);
    }
}
