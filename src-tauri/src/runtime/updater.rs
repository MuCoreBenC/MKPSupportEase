//! **应用内更新**：在程序里下载新版本、显示进度、下载完自动
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

use postprocess::diag::messages;

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
    /// 下完了，可以装了。
    ///
    /// ★ **`version` / `channel` 是给界面判"这是不是这张卡的那一版"用的**
    /// 界面据这两格判"已就绪的是不是这张卡" —— 只看 `path` 会把另一通道的就绪当成这一张的。
    /// 只有 `path` 时界面分不清"已就绪"和"就绪的是别的通道那一版"。
    Ready {
        path: String,
        size: u64,
        version: String,
        channel: super::release_info::UpdateChannel,
    },
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
        ::postprocess::diag::messages::text(
            "ui.runtime.updater.downloaded",
            &[("v1", &mb(received))],
        )
    } else {
        format!("{} / {}", mb(received), mb(total))
    }
}

/// **纯判定**：这份下载完了吗、能不能拿去装。
///
/// 尺寸对不上 = 这一份坏了（截断 / 中间被改）—— **不装**。SHA 不在预期里也一样。
pub fn verify_size(received: u64, expected: u64) -> Result<(), AppError> {
    if expected > 0 && received != expected {
        return Err(AppError::corrupted(messages::text(
            "ui.runtime.updater.469",
            &[
                ("received", &received.to_string()),
                ("expected", &expected.to_string()),
            ],
        )));
    }
    Ok(())
}

/// 文件的 SHA-256（小写十六进制）。**空文件也给一个真值**（那本身就是"坏"的证据）。
pub fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut f = std::fs::File::open(path).map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.470", &[])).with_detail(e.to_string())
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| {
            AppError::io(messages::text("ui.runtime.updater.471", &[])).with_detail(e.to_string())
        })?;
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
        return Err(AppError::corrupted(messages::text(
            "ui.runtime.updater.472",
            &[],
        )));
    }
    Ok(())
}

/// 解压：系统 `unzip`（macOS 自带）。
///
/// ★ **只认退出码**（判据 [`unpack_is_checked_by_exit_code`]）：我们不解析 zip 结构，
///   `unzip` 自己管格式。`ok = false` 时用它的 stderr 说人话。
pub fn unpack(zip: &Path, into: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(into).map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.473", &[])).with_detail(e.to_string())
    })?;
    let out = std::process::Command::new("unzip")
        .args(["-q", "-o", zip.to_str().unwrap_or_default(), "-d"])
        .arg(into)
        .output()
        .map_err(|e| {
            AppError::io(messages::text("ui.runtime.updater.474", &[])).with_detail(e.to_string())
        })?;
    if !out.status.success() {
        return Err(AppError::io(messages::text("ui.runtime.updater.475", &[]))
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
        .map_err(|e| {
            AppError::io(messages::text("ui.runtime.updater.476", &[])).with_detail(e.to_string())
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .collect();
    apps.sort();
    match apps.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(
            AppError::corrupted(messages::text("ui.runtime.updater.477", &[]))
                .with_detail(dir.display().to_string()),
        ),
        many => Err(AppError::invalid_argument(messages::text(
            "ui.runtime.updater.478",
            &[
                ("v1", &many.len().to_string()),
                (
                    "v2",
                    &many
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("、")
                        .to_string(),
                ),
            ],
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
        Err(e) => {
            return Err(AppError::io(messages::text("ui.runtime.updater.479", &[]))
                .with_detail(e.to_string()))
        }
    };
    serde_json::from_slice(&bytes).map(Some).map_err(|e| {
        AppError::corrupted(messages::text("ui.runtime.updater.480", &[]))
            .with_detail(e.to_string())
    })
}

/// 写安装结果（**由退出后那个脚本代写**）。
pub fn write_result(app_data: &Path, result: &UpdateResult) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(result).map_err(|e| {
        AppError::internal(messages::text("ui.runtime.updater.481", &[])).with_detail(e.to_string())
    })?;
    crate::fsx::atomic::atomic_write(&result_path(app_data), &bytes)?;
    Ok(())
}

/// 版本号进脚本 / 进账之前先洗一遍。
///
/// ★ 它是**外部输入**：`Ready` 带的是清单里那一版的号，不是本进程编进去的那个。
///   远端清单是外部输入，一个带引号的"版本号"能改写脚本里的 JSON 乃至 `printf` 的参数。
///   只留 `A-Za-z0-9.-`，洗空了就退回本构建的版本（宁可记错号，也不让脚本被改写）。
pub fn safe_version(raw: &str) -> String {
    let v: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
        .collect();
    if v.is_empty() {
        crate::runtime::structure::APP_VERSION.to_owned()
    } else {
        v
    }
}

/// 安装脚本（**退出后**由它做：等旧进程走 → 替换 → 重开 → 记结果）。
///
/// 返回一段 `sh -c` 的实参。★ 五件事都在这里，别处不许另写一份：
/// - `sleep 1` + **等 `old_pid` 真的消失**：正在运行的 `.app` 换不掉（代码段在执行）；
/// - `rm -rf` 旧的再 `mv` 新的（`mv` 同一卷、不拷内容，比 ditto 快）；
/// - `open -n` 重新拉起（路径从参数来，不猜；`-n` 见下面那段）；
/// - 结果写 `<appData>/run/update-result.json`，**下次启动**读它说清成没成；
/// - 每一步都往 `<appData>/run/update-install.log` 追加一行。
///
/// 路径一律单引号包起来 —— `/Applications/SupportEase.app` 这种**带空格**是常态。
pub fn install_script(
    app_data: &Path,
    target: &Path,
    staged: &Path,
    version: &str,
    at: &str,
    old_pid: u32,
) -> String {
    let q = shell_quote;
    let log = q(&install_log_path(app_data).display().to_string());
    let record = |ok: bool, reason: &str| {
        q(&format!(
            "{{\"version\":\"{version}\",\"ok\":{ok},\"reason\":\"{reason}\",\"at\":\"{at}\"}}"
        ))
    };
    /* 脚本钉四件事，每一件都对应一个真实的失败模式：
     *   ① **等旧进程退出再动手**（最多 3s；还不走就 TERM → KILL，并记在日志里）——
     *      `open <path>` 遇到"这个应用已经在跑"时只把旧窗口叫到前面，不会拉起新进程；
     *   ② **`open -n`** —— 即便还有别的实例在跑，也要拉起**新版本**这一个进程；
     *   ③ 不用 `&&` 串联：一步失败就把后面全静默跳过（写账与拉起都在后面）；
     *   ④ 每一步写日志、失败也写账（`ok:false` 带原因），并把退出码记下来。
     *
     * ★ 模板用占位符替换而不是 `format!`：这段脚本满屏 `$` 与 `{}`，
     *   塞进 `format!` 要转义一半的花括号。 */
    let script = r#"
# 日志目录必须**先**建出来：下面那条重定向在子 shell 起来之前就打开了文件，
# 目录不存在时整段脚本会当场失败、一个字都不留（这是真跑一次才发现的）。
L=@@LOG@@
R=@@RESULT@@
mkdir -p "$(dirname "$L")" 2>/dev/null
(
echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) install v@@VERSION@@" >> "$L"
echo "shell=$$ old_pid=@@PID@@ uid=$(id -u)" >> "$L"
echo "target=@@TARGET@@ staged=@@STAGED@@" >> "$L"
# ① 等旧进程真的退出：它还在时 `open` 只会把旧窗口叫到前面
#    ★ 3 秒（15×0.2s）已经很大方：`app.exit(0)` 正常在 0.1~0.5s 内退干净，
#      再久就该 TERM 了。
i=0
while [ "$i" -lt 15 ] && kill -0 @@PID@@ 2>/dev/null; do
  sleep 0.2
  i=$((i + 1))
done
if kill -0 @@PID@@ 2>/dev/null; then
  echo "✗ 旧进程 @@PID@@ 3 秒还没退出 —— 先 TERM，再 KILL" >> "$L"
  kill -TERM @@PID@@ 2>/dev/null
  sleep 0.5
  if kill -0 @@PID@@ 2>/dev/null; then
    kill -KILL @@PID@@ 2>/dev/null
    sleep 0.3
  fi
  if kill -0 @@PID@@ 2>/dev/null; then
    echo "✗ 旧进程 @@PID@@ 杀不掉（权限 / 被别的进程托管）" >> "$L"
    printf %s @@FAIL_PID@@ > "$R"
    exit 6
  fi
fi
echo "✓ 旧进程已退出" >> "$L"
# ② 换：先确认新的还在，再删旧的，再搬
if [ ! -e @@STAGED@@ ]; then
  echo "✗ 暂存不存在：@@STAGED@@" >> "$L"
  printf %s @@FAIL_MISSING@@ > "$R"
  exit 2
fi
rm -rf @@TARGET@@
if [ -e @@TARGET@@ ]; then
  echo "✗ 删不掉旧应用（权限 / 正在被别的进程使用）：@@TARGET@@" >> "$L"
  printf %s @@FAIL_REMOVE@@ > "$R"
  exit 3
fi
if ! mv @@STAGED@@ @@TARGET@@; then
  echo "✗ 移动失败（跨卷 / 权限）：@@STAGED@@ -> @@TARGET@@" >> "$L"
  printf %s @@FAIL_MOVE@@ > "$R"
  exit 4
fi
echo "✓ 已替换应用" >> "$L"
# ③ 拉起：`-n` 是必须的 —— 不带它时，只要还有别的实例在跑，LaunchServices
#    就只把那个旧窗口叫到前面，新版本一次都没被启动过（作者看到的就是"没反应"）
if ! open -n @@TARGET@@; then
  echo "✗ 替换成功但重新拉起失败：@@TARGET@@" >> "$L"
  printf %s @@FAIL_OPEN@@ > "$R"
  exit 5
fi
echo "✓ 已重新拉起" >> "$L"
printf %s @@OK@@ > "$R"
) >> "$L" 2>&1
# ★ 这三行是"断在哪一步"的最后一块拼图，且必须在**子 shell 外**：
#   写在里面时 `exit 2` 会把整条脚本带走，这一行永远跑不到（第一版就这么错的）。
code=$?
echo "exit=$code" >> "$L"
exit "$code"
"#;
    script
        .replace("@@LOG@@", &log)
        .replace("@@RESULT@@", &res_path(app_data))
        .replace("@@VERSION@@", version)
        .replace("@@PID@@", &old_pid.to_string())
        .replace("@@TARGET@@", &q(&target.display().to_string()))
        .replace("@@STAGED@@", &q(&staged.display().to_string()))
        .replace("@@OK@@", &record(true, ""))
        .replace("@@FAIL_PID@@", &record(false, "旧进程退不掉，安装没有开始"))
        .replace(
            "@@FAIL_MISSING@@",
            &record(false, "暂存里的应用不见了（可能在退出前被清理）"),
        )
        .replace(
            "@@FAIL_REMOVE@@",
            &record(false, "删不掉旧应用（权限 / 被占用）"),
        )
        .replace("@@FAIL_MOVE@@", &record(false, "移动失败（跨卷 / 权限）"))
        .replace("@@FAIL_OPEN@@", &record(false, "替换好了，但没能重新拉起"))
        .trim()
        .to_string()
}

/// 结果账的路径（单引号包好，直接进脚本）。
fn res_path(app_data: &Path) -> String {
    shell_quote(&result_path(app_data).display().to_string())
}

/// **安装过程日志**（`<appData>/run/update-install.log`）。
///
/// 为什么要它：安装脚本在应用**退出之后**才跑，那时没有任何界面能显示错误。
/// 安装发生在应用退出之后，界面上没有任何东西能报错，只剩这份日志 ——
/// 只能靠猜。这份日志是他下次能直接发给我的东西：哪一步、什么路径、退出码多少。
pub fn install_log_path(app_data: &Path) -> PathBuf {
    app_data.join("run").join("update-install.log")
}

/// 单引号包一层（路径里有单引号时按 shell 的规矩转义）。
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// 资产种类：显式 `kind` 优先，老档按文件名后缀兜底。
///
/// `kind` 只有两档，认不出的一律按 `appZip`（macOS 老路径）处理 —— 不猜第三套。
pub fn asset_kind(asset: &ReleaseAsset) -> &str {
    match asset.kind.as_deref().map(str::trim) {
        Some("nsis") => "nsis",
        Some("appZip") => "appZip",
        _ if asset.name.to_ascii_lowercase().ends_with(".exe") => "nsis",
        _ => "appZip",
    }
}

/// **Windows 帮手脚本**：退出后由 PowerShell 做静默安装 + 重启 + 记结果。
///
/// 返回 `powershell -Command` 的实参。与 macOS 的 [`install_script`] 同一套四件事：
/// 等本进程退出 → 装 → 写 `run/update-result.json` → 拉起新版本。
///
/// ★ **纯函数**：本机 macOS 也能跑判据（不碰注册表、不碰系统）。真机行为只承诺
///   Windows CI 编译通过 —— 作者已明确「本机没法真机验证」。
/// ★ NSIS 的 `/S` 是静默开关，`/D=` 必须**最后一个参数且不加引号**；这里不指定
///   `/D`，让安装器按自己的默认目录装（与用户第一次安装的目录一致）。
/// ★ 路径用 PowerShell 单引号（里面单引号翻倍），带空格是常态。
pub fn windows_install_script(
    app_data: &Path,
    setup: &Path,
    new_exe: &Path,
    version: &str,
    at: &str,
) -> String {
    let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
    let result = UpdateResult {
        version: version.to_owned(),
        ok: true,
        reason: String::new(),
        at: at.to_owned(),
    };
    // 结果 JSON 由 `ConvertTo-Json` 生成，不手拼引号（PowerShell 里手拼极易错）
    let ok_json = serde_json::to_string(&result).unwrap_or_default();
    format!(
        "Start-Sleep -Seconds 1; \
         $p = Start-Process -FilePath {setup} -ArgumentList '/S' -Wait -PassThru; \
         if ($p.ExitCode -ne 0) {{ exit 1 }}; \
         {ok} | Set-Content -LiteralPath {r} -Encoding utf8; \
         Start-Process -FilePath {exe}",
        setup = q(&setup.display().to_string()),
        ok = q(&ok_json),
        r = q(&result_path(app_data).display().to_string()),
        exe = q(&new_exe.display().to_string()),
    )
}

/// Windows 上「装完之后拉起哪一个 exe」：NSIS 装到与**当前进程**同一目录，
/// 所以直接从 `current_exe()` 反推，不猜 Program Files 的具体位置。
pub fn running_exe() -> Result<PathBuf, AppError> {
    std::env::current_exe().map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.482", &[])).with_detail(e.to_string())
    })
}

/// 从当前可执行文件反推**这次运行的那个 `.app`**（`.../SupportEase.app/Contents/MacOS/…`
/// → `.../SupportEase.app`）。找不到就如实说"不在 .app 里"（那就不是安装版）。
///
/// ★ **macOS 专用**：Windows 走 [`running_exe`]，两边别混。
pub fn running_app() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe().map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.482", &[])).with_detail(e.to_string())
    })?;
    exe.ancestors()
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::not_implemented(messages::text("ui.runtime.updater.483", &[])))
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
    version: &str,
    channel: super::release_info::UpdateChannel,
    app_data: &Path,
    emit: &dyn Fn(UpdateState),
) -> Result<(), AppError> {
    let sess = session();
    if matches!(
        sess.state(),
        UpdateState::Downloading { .. } | UpdateState::Paused { .. }
    ) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.updater.484",
            &[],
        )));
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
    std::fs::create_dir_all(&dir).map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.485", &[])).with_detail(e.to_string())
    })?;
    let zip_path = dir.join(&asset.name);
    let part_path = dir.join(format!("{}.part", asset.name));

    let result = download_into(
        asset, &part_path, &zip_path, &paused, &cancelled, emit, sess,
    );
    if result.is_err() {
        let _ = std::fs::remove_file(&part_path);
    }
    result?;

    /* ★ Windows：NSIS 安装包是**一个 .exe**，不解压。
     * 下载阶段已验过 size / sha；这里直接把 exe 交给 `Ready`，
     * 退出之后由帮手脚本 `/S` 静默安装并拉起新版本。
     * macOS 那条「解压 → 找 .app → 替换」整段保持不动。 */
    if asset_kind(asset) == "nsis" {
        let size = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
        sess.set(UpdateState::Ready {
            path: zip_path.display().to_string(),
            size,
            version: version.to_owned(),
            channel,
        });
        emit(sess.state());
        return Ok(());
    }

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
        version: version.to_owned(),
        channel,
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
    let mut f = std::fs::File::create(part).map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.486", &[])).with_detail(e.to_string())
    })?;

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
                return Err(AppError::io(messages::text("ui.runtime.updater.487", &[])));
            }
        }
        if cancelled.load(Ordering::SeqCst) {
            sess.set(UpdateState::Cancelled);
            emit(sess.state());
            return Err(AppError::io(messages::text("ui.runtime.updater.488", &[])));
        }
        let n = reader.read(&mut buf).map_err(|e| {
            let msg = ::postprocess::diag::messages::text(
                "ui.runtime.updater.interrupted_detail",
                &[("detail", &e.to_string())],
            );
            sess.set(UpdateState::Failed {
                reason: msg.clone(),
            });
            AppError::io(messages::text("ui.runtime.updater.489", &[])).with_detail(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| {
            AppError::io(messages::text("ui.runtime.updater.490", &[])).with_detail(e.to_string())
        })?;
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
    std::fs::rename(part, done).map_err(|e| {
        AppError::io(messages::text("ui.runtime.updater.491", &[])).with_detail(e.to_string())
    })?;
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
        //   判据也守"跨平台"这一条。
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
        let script = install_script(
            d.path(),
            target,
            &new_app,
            "0.0.4",
            "2026-10-05T00:00:00Z",
            dead_pid(),
        );
        assert!(
            script.contains("'/Applications/SupportEase.app'"),
            "路径要带引号（带空格是常态）"
        );
        assert!(script.contains("rm -rf"), "要删掉旧的那份");
        assert!(script.contains("mv"), "要把新的搬过去");
        assert!(script.contains("open -n"), "要重新拉起（且必须是新进程）");
        assert!(script.contains(RESULT_FILE), "结果要落账（下次启动读）");
    }

    /// ★ **安装脚本必须留下可查的证据**：装完之后人要能回答"它到底做到哪一步"。
    ///
    /// 旧脚本只有 `sleep; rm && mv && printf && open` 一条链，一个字都不留：
    /// 任何一步失败，后面的写账与拉起全被 `&&` 跳过，下次启动既没账、也没线索。
    ///
    /// 这条判据钉三件事：① 有日志文件且每一步都写；② 失败也写账（`ok:false`）；
    /// ③ **不再用 `&&` 串联**（否则失败静默）。
    #[test]
    fn the_install_script_leaves_a_trail_when_it_fails() {
        let d = tempfile::tempdir().unwrap();
        let staged = d.path().join("stage/SupportEase.app");
        let target = Path::new("/Applications/SupportEase.app");
        let script = install_script(
            d.path(),
            target,
            &staged,
            "0.0.11",
            "2026-11-06T00:00:00Z",
            dead_pid(),
        );

        assert!(
            script.contains(install_log_path(d.path()).to_str().unwrap()),
            "要写 run/update-install.log"
        );
        for step in ["暂存不存在", "删不掉旧应用", "移动失败", "重新拉起失败"]
        {
            assert!(script.contains(step), "失败分支要留话：缺 {step}");
        }
        assert!(
            script.contains("\"ok\":false"),
            "失败也要写账 —— 否则下次启动分不清「没装」和「装了但失败」"
        );
        assert!(
            !script.contains("&& mv"),
            "别再用 && 串联：一步失败就静默跳过后面全部"
        );
        assert!(script.contains("exit="), "退出码要记下来");
    }

    /// ★ **脚本得真的跑得起来**：只断言"字符串里有这几个字"
    /// 而从没让 `sh` 执行过它 —— 真跑一次才发现 `run/` 不存在时 `echo >>` 与
    /// `printf >` 全部失败，脚本一个字都留不下（正是"没反应"最难看的那种）。
    ///
    /// 这里跑**失败分支**（暂存不存在）：不碰 `/Applications`、不 `open`、不杀进程，
    /// 却把整条"写日志 → 判断 → 写账 → 退出码"的链路真的走一遍。
    #[cfg(unix)]
    #[test]
    fn the_install_script_really_runs_and_records_a_failure() {
        let d = tempfile::tempdir().unwrap();
        /* 注意：tempdir 里**没有** `run/` —— 脚本必须自己把目录补出来 */
        let staged = d.path().join("stage/SupportEase.app");
        let target = d.path().join("Applications/SupportEase.app");
        let script = install_script(
            d.path(),
            &target,
            &staged,
            "0.0.11",
            "2026-11-06T00:00:00Z",
            dead_pid(),
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("sh 该起得来");
        assert_eq!(out.status.code(), Some(2), "暂存不存在 ⇒ 退出码 2");

        let log = std::fs::read_to_string(install_log_path(d.path()))
            .expect("即使 run/ 原本不存在，日志也该被创建");
        assert!(log.contains("暂存不存在"), "日志要写清断在哪一步：{log}");
        assert!(log.contains("exit=2"), "退出码要记在日志里：{log}");

        let res = read_result(d.path()).unwrap().expect("失败也要写账");
        assert!(!res.ok, "这一趟没装成，账上不许写成功");
        assert_eq!(res.version, "0.0.11", "账上记的是**要装的那一版**");
        assert!(!res.reason.trim().is_empty(), "失败要有原因，不能空着");
    }

    /// 版本号来自远端清单 ⇒ 进脚本前必须洗过；洗空了退回本构建的版本。
    #[test]
    fn a_version_from_the_manifest_cannot_rewrite_the_script() {
        assert_eq!(safe_version("0.0.11-preview.1"), "0.0.11-preview.1");
        assert_eq!(safe_version(" 0.0.11 "), "0.0.11");
        assert_eq!(safe_version("0.0.11'; rm -rf /"), "0.0.11rm-rf");
        assert_eq!(
            safe_version("''''"),
            crate::runtime::structure::APP_VERSION,
            "洗空了退回本构建的版本"
        );
    }

    /// ★ **整趟真跑一遍**（替换 + 拉起 + 写账）：用一个假的 `open` 顶掉系统那一个，
    /// 于是"新版本真的被搬到位、真的是**新进程**被拉起"这两件事都验证得到，
    /// 又不会真的打开任何东西。
    #[cfg(unix)]
    #[test]
    fn the_install_script_replaces_the_app_and_relaunches_a_new_process() {
        use std::os::unix::fs::PermissionsExt as _;

        let d = tempfile::tempdir().unwrap();
        /* 假的 `open`：把收到的参数记下来（顺便证明是 `-n`） */
        let bin = d.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let stub = bin.join("open");
        crate::fsx::atomic::atomic_write(
            &stub,
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$(dirname \"$0\")/open.log\"\n".as_bytes(),
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

        let target = d.path().join("Applications/SupportEase.app");
        std::fs::create_dir_all(&target).unwrap();
        crate::fsx::atomic::atomic_write(&target.join("old.txt"), b"old").unwrap();

        let staged = d.path().join("stage/SupportEase.app");
        std::fs::create_dir_all(&staged).unwrap();
        crate::fsx::atomic::atomic_write(&staged.join("new.txt"), b"new").unwrap();

        let script = install_script(
            d.path(),
            &target,
            &staged,
            "0.0.11",
            "2026-11-06T00:00:00Z",
            dead_pid(),
        );
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .env("PATH", path)
            .output()
            .expect("sh 该起得来");
        assert_eq!(
            out.status.code(),
            Some(0),
            "整趟该成功：{}",
            String::from_utf8_lossy(&out.stderr)
        );

        assert!(target.join("new.txt").exists(), "新的 .app 该被搬到位");
        assert!(!target.join("old.txt").exists(), "旧的 .app 该被删掉");
        assert!(!staged.exists(), "暂存该被搬走（不是复制）");

        let res = read_result(d.path()).unwrap().expect("成功也要写账");
        assert!(res.ok, "这一趟成了，账上要写成功");
        assert_eq!(res.version, "0.0.11");

        let log = std::fs::read_to_string(install_log_path(d.path())).unwrap();
        for step in ["✓ 已替换应用", "✓ 已重新拉起", "exit=0"] {
            assert!(log.contains(step), "日志里缺「{step}」：{log}");
        }

        let opened = std::fs::read_to_string(bin.join("open.log")).unwrap();
        assert!(
            opened.trim_start().starts_with("-n "),
            "★ 必须以 `open -n` 拉起：不带 -n 时，只要还有旧实例在跑，\n\
             系统只会把那个旧窗口叫到前面，新版本一次都不会被启动：{opened}"
        );
        assert!(opened.contains("SupportEase.app"));
    }

    /// 一个**确定已经退出**的 PID（`kill -0` 对它为假）—— 判据里代替"旧进程"。
    ///
    /// 不能传测试进程自己的 PID：脚本会先等 10 秒、再 TERM/KILL，那等于把测试进程杀了。
    fn dead_pid() -> u32 {
        let mut cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", "exit 0"]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", "exit 0"]);
            c
        };
        let mut child = cmd.spawn().expect("起个短命进程该成功");
        let pid = child.id();
        let _ = child.wait();
        pid
    }

    /// **Windows 帮手脚本**：本机是 macOS，只验**纯函数产出的形状** ——
    /// 真机行为由 Windows CI 编译通过来守，不在这里假装跑过。
    ///
    /// 旧判据：只有 macOS 的 `install_script` 一条（替换 `.app` + `open`）。
    /// 新判据：Windows 走 `Start-Process … /S` 静默安装 + 记账 + 拉起 exe。
    /// 保住的：结果写同一份 `update-result.json`（下次启动读的是同一个口子）、
    /// 先等本进程退出、路径带引号。放弃的：把 Windows 也当 `.app` 去解包替换。
    /// 为什么：NSIS 是安装器，不是可直接替换的目录；`/S` 是它的静默开关。
    #[test]
    fn the_windows_install_script_installs_silently_and_records() {
        let d = tempfile::tempdir().unwrap();
        let setup = d.path().join("SupportEase_0.0.10_x64-setup.exe");
        let exe = Path::new(r"C:\Program Files\SupportEase\SupportEase.exe");
        let script =
            windows_install_script(d.path(), &setup, exe, "0.0.10", "2026-11-05T00:00:00Z");
        assert!(
            script.contains("Start-Sleep -Seconds 1"),
            "要先等本进程退出"
        );
        assert!(script.contains("-ArgumentList '/S'"), "NSIS 静默开关");
        assert!(
            script.contains("-Wait -PassThru"),
            "要等安装结束并检查退出码"
        );
        assert!(script.contains("ExitCode -ne 0"), "安装失败不许写成成功");
        assert!(script.contains(RESULT_FILE), "结果落同一份账");
        assert!(
            script.contains("Start-Process -FilePath"),
            "要重新拉起新版本"
        );
        assert!(script.contains("SupportEase.exe"), "拉起的是 exe 不是 .app");
        assert!(!script.contains("open "), "不该混进 macOS 的 open");
    }

    /// 资产种类：显式 `kind` 优先，老档按 `.exe` 后缀兜底；认不出一律走 macOS 老路径。
    #[test]
    fn asset_kind_decides_whether_to_unpack() {
        let base = ReleaseAsset {
            name: "SupportEase_0.0.10_x64-setup.exe".to_owned(),
            url: "https://gitee.com/o/r/setup.exe".to_owned(),
            size: 1,
            sha256: String::new(),
            platform: None,
            kind: None,
        };
        assert_eq!(asset_kind(&base), "nsis", "老档没 kind 时按后缀认");
        let mut zip = base.clone();
        zip.name = "SupportEase.app.zip".to_owned();
        assert_eq!(asset_kind(&zip), "appZip");
        let mut explicit = base;
        explicit.kind = Some("appZip".to_owned());
        assert_eq!(asset_kind(&explicit), "appZip", "显式 kind 优先");
    }
}
