//! **应用内更新**（2026-10-05 第五刀）：在程序里下载新版本、显示进度、下载完自动
//! 替换 `/Applications/SupportEase.app` 并重启。
//!
//! # 形状：状态机 + 一个后台任务
//!
//! ```text
//! check_software_update ──有 asset──→ start_update
//!                                   ↓
//!   Idle ─→ Downloading ─→ Ready ──install──→ （退出）替换 + 重启
//!             ↓  ↑            ↑
//!          Paused ──resume──┘
//!             ↓
//!          Cancelled / Failed
//! ```
//!
//! # 为什么不引 zip 依赖
//!
//! 第一阶段只发 macOS，**`ditto`（打包）与 `unzip`（解压）系统自带**。为一个"解 zip"
//! 拉一个第三方压缩库进客户端不值当 —— 而且我们**不解析 zip 结构**，只是把系统工具
//! 的退出码当事实（判据 [`unpack_is_checked_by_exit_code`] 钉住这一点）。
//!
//! # 三条纪律
//!
//! 1. **只读命令 async、IO 命令 async**（作者铁律：同步命令跑在主线程，界面冻多久
//!    等于下载多久）；
//! 2. **进度是事件、状态是快照**（事件推给界面、快照回答"现在到底什么状态"，
//!    界面重新挂上来时不会丢状态）；
//! 3. **失败即停，如实说**（不重试到"看起来成功"、不拿半个文件去装）。
//!
//! # 安装那一步为什么要在"退出之后"做
//!
//! 一个正在运行的 `.app` **不能被自己替换**（代码段在执行）。所以：下载完先解压到
//! 暂存目录，再 spawn 一个脱离本进程的后台脚本，由它 `sleep` 一小会儿 → 替换 → 重新
//! `open`。**替换/Applications 那一步的失败我们看不见**（本进程已经退出）—— 所以脚本
//! 把结果写进 `<appData>/run/update-result.json`，**下次启动**读它并说清上次成没成
//! （判据 [`update_result_is_read_on_next_launch`]）。

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::error::AppError;

use super::release_info::ReleaseAsset;

/// 下载进度的**事件名**（前端 `listen` 订阅它）。
pub const PROGRESS_EVENT: &str = "software-update-progress";

/// 暂存目录（`<appData>/run/update/`）——**只放这一次更新的东西**，重启后由下一次
/// 启动按结果清理（不无限堆积）。
const STAGE_DIR: &str = "update";

/// 上次安装结果的落地文件名（见模块头纪律 3）。
pub const RESULT_FILE: &str = "update-result.json";

/// 界面上看得到的状态。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateState {
    /// 还没开始（也没有更新的那一档）
    Idle,
    /// 正在下载
    Downloading { received: u64, total: u64 },
    /// 用户按了暂停（**连接没断**，只是不往下写）
    Paused { received: u64, total: u64 },
    /// 下完了，可以装了
    Ready { path: String, size: u64 },
    /// 出错了（如实说，不装）
    Failed { reason: String },
    /// 用户按了取消（暂存目录已清）
    Cancelled,
}

/// 一次下载任务的共享状态：**进度 + 暂停/取消两个开关**。
pub struct Session {
    state: Mutex<UpdateState>,
    paused: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

impl Session {
    fn new() -> Self {
        Self {
            state: Mutex::new(UpdateState::Idle),
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 当前状态（**快照**：界面重新挂上来问一次就知道现在怎样）。
    pub fn state(&self) -> UpdateState {
        self.state
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UpdateState::Idle)
    }

    fn set(&self, next: UpdateState) {
        match self.state.lock() {
            Ok(mut slot) => *slot = next,
            Err(_) => tracing::error!("更新状态锁坏了 —— 这一格只能如实停在未知"),
        }
    }
}

/// 全局那一个会话（客户端只有一个更新任务）。
pub fn session() -> &'static Session {
    static CELL: OnceLock<Session> = OnceLock::new();
    CELL.get_or_init(Session::new)
}

/// 进度百分比（**纯函数**，判据钉它：total 未知时给 0，不给 100 —— 那会骗人）。
pub fn percent(received: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    let p = received.saturating_mul(100) / total;
    p.min(100) as u8
}

/// 进度条上那一句（人话；`total` 未知就说"已下多少"，不猜总量）。
pub fn progress_text(received: u64, total: u64) -> String {
    let mb = |v: u64| format!("{:.1} MB", v as f64 / 1024.0 / 1024.0);
    if total == 0 {
        format!("已下载 {}", mb(received))
    } else {
        format!("{} / {}", mb(received), mb(total))
    }
}

/// **纯判定**：这份下载完了吗、能不能拿去装。
///
/// 尺寸对不上 = 这一份坏了（截断 / 中间被改）—— **不装**。SHA 不在预期里也一样。
pub fn verify_size(received: u64, expected: u64) -> Result<(), AppError> {
    if expected > 0 && received != expected {
        return Err(AppError::corrupted(format!(
            "下载下来的安装包大小不对（收到 {received}，应当 {expected}）—— 这一份坏了，不装"
        )));
    }
    Ok(())
}

/// 文件的 SHA-256（小写十六进制）。**空文件也给一个真值**（那本身就是"坏"的证据）。
pub fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| AppError::io("读不到下载好的安装包").with_detail(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| AppError::io("算校验和时读不了文件").with_detail(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// 校验和**对不上**时报错（调用方给 `None` = 这一版没给校验和 → 不校验）。
pub fn verify_sha256(actual: &str, expected: Option<&str>) -> Result<(), AppError> {
    let Some(want) = expected else {
        return Ok(());
    };
    if !actual.eq_ignore_ascii_case(want.trim()) {
        return Err(AppError::corrupted("下载下来的安装包校验和对不上 —— 不装"));
    }
    Ok(())
}

/// 解压：系统 `unzip`（macOS 自带）。
///
/// ★ **只认退出码**（判据 [`unpack_is_checked_by_exit_code`]）：我们不解析 zip 结构，
///   `unzip` 自己管格式。`ok = false` 时用它的 stderr 说人话。
pub fn unpack(zip: &Path, into: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(into)
        .map_err(|e| AppError::io("建不出解压目录").with_detail(e.to_string()))?;
    let out = std::process::Command::new("unzip")
        .args(["-q", "-o", zip.to_str().unwrap_or_default(), "-d"])
        .arg(into)
        .output()
        .map_err(|e| AppError::io("起不了系统 unzip").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("解压安装包失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    Ok(())
}

/// 解压出来的 `.app` 在哪（`…/SupportEase.app`）。
///
/// **只认恰好一份**（判据 `the_unpacked_app_is_the_only_one`）：多了或少了都要人来看，
/// 不许自己挑一个。
pub fn find_unpacked_app(dir: &Path) -> Result<PathBuf, AppError> {
    let mut apps: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| AppError::io("读不了解压目录").with_detail(e.to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .collect();
    apps.sort();
    match apps.as_slice() {
        [one] => Ok(one.clone()),
        [] => {
            Err(AppError::corrupted("解压出来的目录里没有 .app")
                .with_detail(dir.display().to_string()))
        }
        many => Err(AppError::invalid_argument(format!(
            "解压出来的目录里有 {} 个 .app，不知道该用哪一个：{}",
            many.len(),
            many.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、")
        ))),
    }
}

/// 上次安装的结果（**下次启动**读它；没有就是"没装过"）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    /// 装的是哪一版
    pub version: String,
    /// 成了没有
    pub ok: bool,
    /// 失败原因（成功时为空）
    pub reason: String,
    /// 什么时候（ISO8601，工作台 `clock` 那套）
    pub at: String,
}

/// 结果文件路径（`<appData>/run/update-result.json`）。
pub fn result_path(app_data: &Path) -> PathBuf {
    app_data.join("run").join(RESULT_FILE)
}

/// 读上次结果；坏档报 `CORRUPTED`（不静默当"没装过"）。
pub fn read_result(app_data: &Path) -> Result<Option<UpdateResult>, AppError> {
    let path = result_path(app_data);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::io("读不了上次安装结果").with_detail(e.to_string())),
    };
    serde_json::from_slice(&bytes).map(Some).map_err(|e| {
        AppError::corrupted("上次安装结果（update-result.json）解析不了").with_detail(e.to_string())
    })
}

/// 写安装结果（**由退出后那个脚本代写**）。
pub fn write_result(app_data: &Path, result: &UpdateResult) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(result)
        .map_err(|e| AppError::internal("安装结果序列化失败").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&result_path(app_data), &bytes)?;
    Ok(())
}

/// 安装脚本（**退出后**由它做：替换 + 重开 + 记结果）。
///
/// 返回一段 `sh -c` 的实参。★ 四件事都在这里，别处不许另写一份：
/// - `sleep 1`：**等本进程真的退出**（正在运行的 `.app` 换不掉，代码段在执行）；
/// - `rm -rf` 旧的再 `mv` 新的（`mv` 同一卷、不拷内容，比 ditto 快）；
/// - 结果写 `<appData>/run/update-result.json`，**下次启动**读它说清成没成；
/// - `open` 重新拉起（路径从参数来，不猜）。
///
/// 路径一律单引号包起来 —— `/Applications/SupportEase.app` 这种**带空格**是常态。
pub fn install_script(
    app_data: &Path,
    target: &Path,
    staged: &Path,
    version: &str,
    at: &str,
) -> String {
    let q = shell_quote;
    format!(
        "sleep 1; rm -rf {t} && mv {s} {t} && printf {ok} > {r} && open {t}",
        t = q(&target.display().to_string()),
        s = q(&staged.display().to_string()),
        r = q(&result_path(app_data).display().to_string()),
        // 成功才写账；失败时脚本以非 0 退出（下次启动看到"没有账" = 上次没成）
        ok = q(&format!(
            "{{\"version\":\"{version}\",\"ok\":true,\"reason\":\"\",\"at\":\"{at}\"}}"
        )),
    )
}

/// 单引号包一层（路径里有单引号时按 shell 的规矩转义）。
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// 从当前可执行文件反推**这次运行的那个 `.app`**（`.../SupportEase.app/Contents/MacOS/…`
/// → `.../SupportEase.app`）。找不到就如实说"不在 .app 里"（那就不是安装版）。
pub fn running_app() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::io("定位不到自己").with_detail(e.to_string()))?;
    exe.ancestors()
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            AppError::not_implemented("这个构建不是从 .app 里跑的（开发构建），没有可替换的目标")
        })
}

/* ---------------------- 驱动：下载任务 ---------------------- */

/// 暂存目录（`<appData>/run/update/`）。
pub fn stage_dir(app_data: &Path) -> PathBuf {
    app_data.join("run").join(STAGE_DIR)
}

/// 开始下载（**后台跑**：调用方在 `spawn_blocking` 里）。
///
/// - 已经在下 → 如实拒（不并起两个任务）；
/// - 边写边查暂停/取消两个开关（**下载过程中随时能按停**）；
/// - 下完 → 验大小 → 验 SHA → 解压 → 找到那份 `.app` → [`UpdateState::Ready`]。
///
/// `emit` 每收到一块就报一次进度（前端只管画环）。
pub fn download(
    asset: &ReleaseAsset,
    app_data: &Path,
    emit: &dyn Fn(UpdateState),
) -> Result<(), AppError> {
    let sess = session();
    if matches!(
        sess.state(),
        UpdateState::Downloading { .. } | UpdateState::Paused { .. }
    ) {
        return Err(AppError::invalid_argument("已经在下载了"));
    }
    let paused = Arc::clone(&sess.paused);
    let cancelled = Arc::clone(&sess.cancelled);
    paused.store(false, Ordering::SeqCst);
    cancelled.store(false, Ordering::SeqCst);

    let dir = stage_dir(app_data);
    // 上一次 leftovers 先清（不无限堆积；★ 只删我们自己的暂存目录）
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::io("建不出更新暂存目录").with_detail(e.to_string()))?;
    let zip_path = dir.join(&asset.name);
    let part_path = dir.join(format!("{}.part", asset.name));

    let result = download_into(
        asset, &part_path, &zip_path, &paused, &cancelled, emit, sess,
    );
    if result.is_err() {
        let _ = std::fs::remove_file(&part_path);
    }
    result?;

    // 解压 → 找那份 .app
    let unpacked = dir.join("unpacked");
    if let Err(e) = unpack(&zip_path, &unpacked) {
        let _ = std::fs::remove_dir_all(&dir);
        sess.set(UpdateState::Failed {
            reason: e.message.clone(),
        });
        return Err(e);
    }
    let app = match find_unpacked_app(&unpacked) {
        Ok(app) => app,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            sess.set(UpdateState::Failed {
                reason: e.message.clone(),
            });
            return Err(e);
        }
    };
    let size = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
    sess.set(UpdateState::Ready {
        path: app.display().to_string(),
        size,
    });
    emit(sess.state());
    Ok(())
}

/// 流式下载（**不整个进内存** —— 安装包是几十 MB）。
#[allow(clippy::too_many_arguments)]
fn download_into(
    asset: &ReleaseAsset,
    part: &Path,
    done: &Path,
    paused: &AtomicBool,
    cancelled: &AtomicBool,
    emit: &dyn Fn(UpdateState),
    sess: &Session,
) -> Result<(), AppError> {
    let resp = crate::runtime::net::stream_get(&asset.url)?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = resp.into_body().into_reader();
    // ★ 这里**必须**直接开文件流式写，不能走 `fsx::atomic_write`：
    //   安装包几十 MB，atomic 那条是"整个字节数组进内存再落盘"（`fsx::atomic` 的定义）。
    //   这是 clippy 禁列 `File::create` 的**第二处逃生口**（第一处是 `fsx/atomic.rs` 本身）。
    //   退役条件：**一旦**下载改成"先落 .part 再 atomic rename"的那套（postprocess 的
    //   `write_atomic` 已经是那个形状），这里就换成它。
    #[allow(clippy::disallowed_methods)]
    let mut f = std::fs::File::create(part)
        .map_err(|e| AppError::io("建不出下载暂存文件").with_detail(e.to_string()))?;

    let mut received: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut buf = [0u8; 128 * 1024];
    loop {
        // 暂停：连接留着，只是**不往下写**（用户按了继续就从这儿接上）
        while paused.load(Ordering::SeqCst) {
            sess.set(UpdateState::Paused { received, total });
            emit(sess.state());
            std::thread::sleep(std::time::Duration::from_millis(120));
            if cancelled.load(Ordering::SeqCst) {
                sess.set(UpdateState::Cancelled);
                emit(sess.state());
                return Err(AppError::io("下载已取消"));
            }
        }
        if cancelled.load(Ordering::SeqCst) {
            sess.set(UpdateState::Cancelled);
            emit(sess.state());
            return Err(AppError::io("下载已取消"));
        }
        let n = reader.read(&mut buf).map_err(|e| {
            let msg = format!("下载中断：{e}");
            sess.set(UpdateState::Failed {
                reason: msg.clone(),
            });
            AppError::io("下载中断").with_detail(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n])
            .map_err(|e| AppError::io("写不了下载暂存文件").with_detail(e.to_string()))?;
        received += n as u64;
        // 进度**别太密**：每 120ms 一次就够画环了（每块都发会把界面打爆）
        if last_report.elapsed() >= std::time::Duration::from_millis(120) {
            sess.set(UpdateState::Downloading { received, total });
            emit(sess.state());
            last_report = std::time::Instant::now();
        }
    }
    use std::io::Write as _;
    f.flush().ok();
    drop(f);
    sess.set(UpdateState::Downloading { received, total });
    emit(sess.state());

    verify_size(received, asset.size)?;
    if let Some(want) = asset.expect_sha256() {
        verify_sha256(&sha256_file(part)?, Some(&want))?;
    }
    std::fs::rename(part, done)
        .map_err(|e| AppError::io("下载完成但改名失败").with_detail(e.to_string()))?;
    Ok(())
}

/// 暂停 / 继续 / 取消 —— 都只动那两个开关（**状态由下载循环如实推进**）。
pub fn pause() {
    session().paused.store(true, Ordering::SeqCst);
}

pub fn resume() {
    session().paused.store(false, Ordering::SeqCst);
}

/// 取消：置开关 + **立刻把当前状态报成已取消**（下载循环下一轮会收拾残局）。
pub fn cancel() {
    let sess = session();
    sess.cancelled.store(true, Ordering::SeqCst);
    sess.paused.store(false, Ordering::SeqCst);
    sess.set(UpdateState::Cancelled);
}

/// 上次安装的结果（给界面一句"上次更新成没成"）。
pub fn last_result(app_data: &Path) -> Option<UpdateResult> {
    read_result(app_data).ok().flatten()
}

/// 启动后清掉上次 leftovers（**只清暂存目录，不碰别的**）。
pub fn cleanup_stage(app_data: &Path) {
    let dir = stage_dir(app_data);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_honest_about_unknown_totals() {
        assert_eq!(percent(0, 0), 0, "总量未知时给 0，不给 100 —— 那会骗人");
        assert_eq!(percent(50, 100), 50);
        assert_eq!(percent(200, 100), 100, "超额收到也不许超 100");
    }

    #[test]
    fn size_mismatch_is_refused() {
        assert!(verify_size(100, 100).is_ok());
        assert!(verify_size(99, 100).is_err());
        assert!(verify_size(100, 0).is_ok(), "发布方没给大小 = 不按大小拒");
    }

    #[test]
    fn sha256_is_checked_only_when_given() {
        // ★ 用临时目录里的空文件，**不用 `/dev/null`** ——
        //   `/dev/null` 在 Windows 上不存在，这条判据在 rust-windows job 上真红过一次
        //   （2026-10-05，作者贴的 CI 截图）。判据也守"跨平台"这一条。
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.bin");
        crate::fsx::atomic::atomic_write(&empty, b"").unwrap();
        let real = sha256_file(&empty).unwrap();
        assert_eq!(real.len(), 64, "SHA-256 是 32 字节 = 64 个十六进制字符");
        assert!(verify_sha256(&real, None).is_ok(), "没给校验和就不校验");
        assert!(verify_sha256(&real, Some(&real)).is_ok());
        assert!(verify_sha256(&real, Some("deadbeef")).is_err());
    }

    /// 判据：`.app.zip` 里的 `.app` **必须恰好一份**（多了不许自己挑）
    #[test]
    fn the_unpacked_app_is_the_only_one() {
        let d = tempfile::tempdir().unwrap();
        assert!(find_unpacked_app(d.path()).is_err(), "一个都没有 = 报错");
        std::fs::create_dir(d.path().join("SupportEase.app")).unwrap();
        assert_eq!(
            find_unpacked_app(d.path()).unwrap().file_name().unwrap(),
            "SupportEase.app"
        );
        std::fs::create_dir(d.path().join("Another.app")).unwrap();
        let e = find_unpacked_app(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// ★ 判据：**解压只看退出码**（不解析 zip 结构），且坏包要报错
    #[test]
    fn unpack_is_checked_by_exit_code() {
        let d = tempfile::tempdir().unwrap();
        // 造一个真 zip（系统 zip 造得出：先写一个文件再压）
        let src = d.path().join("a.txt");
        crate::fsx::atomic::atomic_write(&src, b"hello").unwrap();
        let zip = d.path().join("a.zip");
        // `-j` = 不把绝对路径存进去（我们要的就是"解开就在那儿"）
        let ok = std::process::Command::new("zip")
            .args(["-q", "-j"])
            .arg(&zip)
            .arg(&src)
            .output();
        if !matches!(ok, Ok(o) if o.status.success()) {
            return; // 环境没有 zip，跳过（不假装通过）
        }
        let into = d.path().join("out");
        unpack(&zip, &into).expect("好包该解开");
        assert_eq!(
            std::fs::read_to_string(into.join("a.txt")).unwrap(),
            "hello"
        );

        // 坏包：退出码非 0 ⇒ 报错
        let bad = d.path().join("bad.zip");
        crate::fsx::atomic::atomic_write(&bad, b"not a zip at all").unwrap();
        assert!(unpack(&bad, &d.path().join("out2")).is_err());
    }

    /// ★ 判据：**安装结果下次启动读得到**（退出后那个脚本写的账，程序认）
    #[test]
    fn update_result_is_read_on_next_launch() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(read_result(d.path()).unwrap(), None, "没装过 = 没有账");
        let r = UpdateResult {
            version: "0.0.4".to_owned(),
            ok: true,
            reason: String::new(),
            at: "2026-10-05T00:00:00Z".to_owned(),
        };
        write_result(d.path(), &r).unwrap();
        assert_eq!(read_result(d.path()).unwrap(), Some(r));

        // 坏档报 CORRUPTED（不静默当"没装过"）
        crate::fsx::atomic::atomic_write(&result_path(d.path()), b"{oops").unwrap();
        assert_eq!(
            read_result(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// 安装脚本：替换 + 重开 + 记结果，三件事都在
    #[test]
    fn the_install_script_replaces_and_reopens_and_records() {
        let d = tempfile::tempdir().unwrap();
        let new_app = d.path().join("staged/SupportEase.app");
        let target = Path::new("/Applications/SupportEase.app");
        let script = install_script(d.path(), target, &new_app, "0.0.4", "2026-10-05T00:00:00Z");
        assert!(
            script.contains("'/Applications/SupportEase.app'"),
            "路径要带引号（带空格是常态）"
        );
        assert!(script.contains("rm -rf"), "要删掉旧的那份");
        assert!(script.contains("mv"), "要把新的搬过去");
        assert!(script.contains("open"), "要重新拉起");
        assert!(script.contains(RESULT_FILE), "结果要落账（下次启动读）");
    }
}
