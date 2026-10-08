//! 「本地测试源（开发）」—— 工作台里那颗**起本地假云端**的按钮。
//!
//! # 它解决的那件事
//!
//! 客户端要验「官方发了新版本 → 检查更新 → 下载 → 对比 → 恢复默认」这一整条链，
//! 得先去终端敲 `npm run preset-source:dev`。可工作台自己就是**从仓库里跑起来**的，
//! 人在工作台里干活时不该再切到终端 —— 这一颗按钮就是那条命令。
//!
//! ```text
//! 按钮 ──► npm run preset-source:dev （cwd = 仓库根）
//!            └─ 起 scripts/preset-test-server/ 在 127.0.0.1:8787
//! ```
//!
//! # ★ 它**只起服务**，不碰客户端（作者 2026-10-08 裁决）
//!
//! 先前那一版按钮跑的是 `npm run dev:test-update`（连客户端 `tauri dev` 一起起、
//! 用 `MKPSE_PRESET_SOURCE_URL` 把源指过去）。作者的判断很直接：**没必要** ——
//! 「这不就是单开一个服务吗？我自己输入这个地址就可以」。
//!
//! 于是这一版只起服务：地址摆在界面上，人去客户端的
//! 「设置 → 高级设置 → 预设数据源 → 自定义地址」填一次（**保存即生效，不用重启**，
//! 见 `runtime/source.rs`）。那一颗按钮因此不会去动你正在用的客户端 dev，
//! 也不会去争它那个 vite 端口 —— 那份"一起起"的便利留在命令行里：
//! `npm run dev:test-update`（它才注入 `MKPSE_PRESET_SOURCE_URL`）。
//!
//! # 边界
//!
//! - **不感知测试源是什么**：这一层只起命令、报状态。它不读夹具内容、不写任何配置、
//!   更不去改客户端的「预设数据源」那一格（那是运行时那份设置，住客户端自己的
//!   `appDataDir/run/app-state.json`，两个应用连 appDataDir 都不一样）。
//! - **只在工作台构建里存在**（`src/workbench/` 整个子树挂在 `--features workbench`
//!   那道闸后面），给用户的二进制里既没有这颗按钮，也没有这几条命令。
//! - 输出**继承给终端**：服务的日志打在起工作台的那个终端里。
//!   这个按钮只是"替你敲了那条命令"，不另造一套日志窗口充当真相。
//! - **端口被占着时，先把占用者摆出来、问一句**（2026-10-08 踩出来的那条）：
//!   8787 上已经有东西（上一轮留下的服务、或你手动起的 `preset-source:dev`）时，
//!   这一份起不来。所以**起之前先探端口**（`conflicts_of`），把占用者连同 PID 摆到
//!   界面上，问一句"要不要把它停掉"（`wb_dev_source_clear_conflict`）。
//!
//!   ★ 判据是**端口**，不是命令行：进程的命令行 / 工作目录在某些环境里读不到
//!   （本机实测 `sysinfo` 对所有进程都返回空），拿它做判据会变成"按钮点了没反应"；
//!   而端口是硬事实 —— 谁在听，`netstat` / `lsof` 说得出来，`taskkill` 停得掉。
//!   （旧项目 `mkppanel/presets_server.go` 的 `ListOccupiedPorts` / `StopPortProcess`
//!   就是这一套，这里按同一个形状。）

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::error::AppError;

/// 起它的那条 npm script。**名字是契约**：`package.json` 里没有这一条，
/// 这颗按钮点的就是一个空壳（下面有测试钉着）。
const NPM_SCRIPT: &str = "preset-source:dev";

/// 夹具根（v1 / v2）：**服务端出去的就是这一份**。两代只差两处（发布日 + 一个参数值），
/// 够演示"官方发新版了 → 检查更新看得见"。路径只在 `scripts/preset-test-server/`
/// 那一处约定，这里照同一个相对位置认。
const FIXTURE_REL_V1: &str = "scripts/preset-test-server/fixtures/v1";
const FIXTURE_REL_V2: &str = "scripts/preset-test-server/fixtures/v2";

/// 夹具齐了的标志文件（`make-fixtures.mjs` 派生的第一件东西；`dev.mjs` 也拿它判"在不在"）。
const FIXTURE_MARKER: &str = "catalog.json";

/// 「当前交付」那一份齐了的标志：交付根的说明书（发布动作写出来的第一件东西）。
const DELIVERY_MARKER: &str = "source.json";

/// 交付根相对预设根的那一层目录名（与 `paths::DELIVERY_SUBDIR` 同一个值；
/// 那一处是 `pub(crate)`，这一层只管拼地址，仍照同一处约定认）。
const DELIVERY_SUBDIR: &str = "delivery";

/// 把「端哪个目录」交给 `server.mjs` 的那个环境变量。
///
/// ★ **为什么走环境变量、而不是给命令加 `--root` 参数**：这一层起的永远是
/// `npm run preset-source:dev` 这一条（`package.json` 里那行是契约，有测试钉着），
/// 命令行本身不带参数；环境变量能被它继承下去，于是"换源"不必给这条命令另开一条
/// argv 路径。`server.mjs` 那边 `--root` 参数**仍然优先**，这个变量只是它的第二档。
const ROOT_ENV: &str = "PRESET_TEST_ROOT";

/// 测试源的默认端口 —— 与 `scripts/preset-test-server/server.mjs` 同一个值。
/// 它读 `PRESET_TEST_PORT`（见 [`port_from`]），这里只是那个变量的默认档。
const DEFAULT_PORT: u16 = 8787;

/// 沿父指针往上找祖先时最多爬几层。**必须有上限**：父子表是从活进程里现取的，
/// 万一取到一张成环的表，没有上限的循环会把工作台卡死。
const MAX_ANCESTOR_HOPS: usize = 32;

/* ---------- 端什么（三选一） ---------- */

/// 测试源端哪一份。
///
/// 前两项是**夹具**（`make-fixtures.mjs` 从真交付根派生的两代，够演"官方发新版了"）。
/// 第三项是 2026-10-08 作者要的那一环：**当前交付** —— 端你（在沙箱里）生成出来的
/// 那一份，于是「改参数 → 生成 → 客户端检查更新 → 看到我的新版」这条闭环在本地
/// 整条走得通，而且**永远不碰正式**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    /// 夹具 v1：模拟「官方源，还没有新版本」
    #[serde(rename = "v1")]
    V1,
    /// 夹具 v2：模拟「官方发了新版」（revision 变了 → 「检查更新」看得见）
    #[serde(rename = "v2")]
    V2,
    /// **当前交付**：测试模式开着 = 沙箱那份，正式 = 仓库那份
    #[serde(rename = "delivery")]
    Delivery,
}

/// 一份源**端出去时**的样子：服务根在哪、客户端该填哪个地址。
///
/// ★ 地址**不只端口那一截**：夹具自己就是一个交付根（`filesRoot: "."`），填到根上；
/// 而真交付根的 `source.json` 写的是 `filesRoot: ".."` —— 交付文件（`mkp/presets/*.toml`）
/// 住在**预设根**底下、不在 `delivery/` 里。所以那一份要端的目录是**预设根**，
/// 客户端填 `<地址>/delivery`：`..` 一解就落回预设根，客户端那条真链（catalog /
/// manifest / 寻址规则 / SHA 校验）一个环节都不变。
struct Served {
    /// 交给 `server.mjs` 的那个目录（`PRESET_TEST_ROOT`）
    root: PathBuf,
    /// 客户端「自定义地址」里该填的那一串
    url: String,
}

impl SourceKind {
    /// 服务根与地址的**纯派生**（不碰磁盘，所以测试钉得住）。
    fn derive(self, repo: &Path, presets_root: &Path) -> Served {
        match self {
            SourceKind::V1 => Served {
                root: repo.join(FIXTURE_REL_V1),
                url: origin(),
            },
            SourceKind::V2 => Served {
                root: repo.join(FIXTURE_REL_V2),
                url: origin(),
            },
            SourceKind::Delivery => Served {
                root: presets_root.to_path_buf(),
                url: format!("{}/{}", origin(), DELIVERY_SUBDIR),
            },
        }
    }

    /// 「这一份齐了吗」的那个标志文件。不齐**不许起**：服务起得来、底下什么都没有的话，
    /// 客户端那边只会说"取不到预设数据" —— 那句答案离原因太远。
    fn marker(self, served: &Served) -> PathBuf {
        match self {
            SourceKind::V1 | SourceKind::V2 => served.root.join(FIXTURE_MARKER),
            SourceKind::Delivery => served
                .root
                .join(DELIVERY_SUBDIR)
                .join(DELIVERY_MARKER),
        }
    }

    /// 不齐时那句人话：**缺什么 + 怎么补**（界面直接摆，不重写措辞）。
    fn missing(self, marker: &Path) -> AppError {
        match self {
            SourceKind::V1 | SourceKind::V2 => AppError::invalid_argument(format!(
                "这份夹具还没派生：{} 不在",
                marker.display()
            ))
            .with_detail(
                "跑一次 `npm run preset-source:make`（从真交付根派生两代夹具），再点「启动」"
                    .to_owned(),
            ),
            SourceKind::Delivery => AppError::invalid_argument(format!(
                "还没有交付产物：{} 不在",
                marker.display()
            ))
            .with_detail(
                "在「生成与发布」里点一次生成 —— 注意**当前模式**：测试模式开着时生成物落在沙箱里，\
                 端的就是沙箱那一份（这正是这一项要验的）"
                    .to_owned(),
            ),
        }
    }
}

/// 界面上那三行单选，一行一份。**名字与说法由后端给**（界面不自己拼一套措辞），
/// 连同"这一份在哪、填什么地址、齐没齐"一起摊开 —— 选之前就该看得见。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceOption {
    pub kind: SourceKind,
    /// 行上的名字（如 `夹具 v1`）
    pub label: String,
    /// 这一份是干什么用的（一句话）
    pub note: String,
    /// 服务根（端的是这个目录 —— 给排查用）
    pub root: String,
    /// 客户端「自定义地址」里要填的那一串（**每一份不一样**）
    pub url: String,
    /// 齐了吗（不齐时点「启动」会被如实拒绝）
    pub ready: bool,
    /// 不齐的话缺什么、怎么补
    pub missing: Option<String>,
}

/* ---------- DTO ---------- */

/// 本地测试源现在什么样。**`running` 每次现问子进程**（`try_wait`），
/// 不是我们自己记的一个布尔 —— 那个进程可能是用户在终端里 Ctrl+C 掉的。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSourceStatus {
    /// 那条 `npm run preset-source:dev` 还活着吗
    pub running: bool,
    /// 包装层（Windows 上是 `cmd`）的 PID。不在跑时 `null`
    pub pid: Option<u32>,
    /// 客户端该填的地址 —— **在跑时是那一份的地址**（夹具是根，交付带 `/delivery`）；
    /// 没起过时给默认地址当兜底。想只手起服务、再去客户端设置页手填的人，填这一串
    pub url: String,
    /// 在跑（没在跑时：上次起）的是哪一份。**从没起过 = `null`**
    pub source: Option<SourceKind>,
    /// 它端的目录（不在跑且没起过时为空串）
    pub source_root: String,
    /// 它在哪个仓库根下跑（命令的 cwd）
    pub repo_root: String,
    /// 实际跑的那条命令（给人对账用，界面不自己拼一遍）
    pub command: String,
    /// 不在跑的时候为什么：退出码那句 / "本来就没在跑"。在跑时 `null`
    pub note: Option<String>,
    /// **起之前该处理掉的占用者**（不在跑的时候才有）：8787 被别人占着的话，
    /// 点「启动」必死在那上面。空 = 端口干净
    pub conflicts: Vec<PortConflict>,
    /// 三行单选：每一份的名字 / 在哪儿 / 填什么地址 / 齐没齐
    pub sources: Vec<SourceOption>,
}

/// 谁占着那个关键端口。**摆给人看的**（`text` 是那句人话）+
/// **给按钮用的**（`port` / `pid` 原样交回去，后端再验一次明正身才动手）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortConflict {
    pub port: u16,
    /// 这个端口是干什么的（界面不自己拼这一句）
    pub role: String,
    /// 占着它的进程（`netstat` / `lsof` 拿不到身份时为 `null`）
    pub pid: Option<u32>,
    pub process: Option<String>,
    /// 那句话人话（**事实只有一处**：界面直接摆出来，不重写一遍措辞）
    pub text: String,
}

/* ---------- 进程状态 ---------- */

/// 那个跑着的子进程，以及它退出之后我们才知道的那件事。
struct DevSource {
    child: Child,
    pid: u32,
    /// 它是按哪一份起的（换源 = 先收掉它、再以新的那一份起）
    source: SourceKind,
    /// 它端的目录（`PRESET_TEST_ROOT` 交下去的那个）
    root: PathBuf,
    /// 客户端该填的地址（随源不同：夹具在根上，交付带 `/delivery`）
    url: String,
    /// `try_wait` 看到它退出之后填上（退出码那句）。填上 = 不在跑了
    exited: Option<String>,
}

/// 一个进程一个工作台窗口，所以放模块级 —— 与 `mod.rs` 的 `SESSION` / `FLUSHER`
/// 同一条理由。存 `Option<DevSource>` 而不是一个"起没起过"的布尔：
/// 停掉之后要能再起，那就得把它从槽里拿走。
static RUNNING: OnceLock<Mutex<Option<DevSource>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<DevSource>> {
    RUNNING.get_or_init(|| Mutex::new(None))
}

/* ---------- 端口 / 命令 ---------- */

/// 端口：`PRESET_TEST_PORT` 给了就听它的 —— `server.mjs` 与 `dev.mjs` 读的是**同一个**
/// 变量，界面照它报地址才不会说错一个数。认不出的值（空 / 非数字 / 0）退回默认。
fn port_from(raw: Option<&str>) -> u16 {
    raw.map(str::trim)
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(DEFAULT_PORT)
}

fn port() -> u16 {
    port_from(std::env::var("PRESET_TEST_PORT").ok().as_deref())
}

fn origin() -> String {
    format!("http://127.0.0.1:{}", port())
}

/// 起 `npm run preset-source:dev`（cwd = 仓库根）——**只有服务**，不碰客户端。
///
/// ★ **Windows 上必须过一层 `cmd /C`**：那边 `npm` 是 `npm.cmd`，而
/// `Command::new("npm")` 走 `CreateProcess`，它**不替你补 `.cmd` 后缀** ——
/// 直接起会报"程序找不到"。与 `release_tx::npm_build` 同一条理由。
///
/// ★ 端哪一份由 [`ROOT_ENV`] 交下去（`server.mjs` 读它，`--root` 参数仍然优先）：
/// 命令本身不带参数，于是"换源"不必给这条命令另开一条 argv 路径。
fn spawn_server(repo: &Path, served: &Served) -> Result<Child, AppError> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("npm");
        c
    } else {
        Command::new("npm")
    };
    cmd.args(["run", NPM_SCRIPT])
        .current_dir(repo)
        .env(ROOT_ENV, &served.root)
        /* 输出继承下去：服务与客户端 dev 的日志就打在起工作台的那个终端里 */
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        /* stdin 给 null：父子都在读 stdin 的话，敲键归谁就成了碰运气
           （`tauri dev` 自己也读 stdin 做热重载） */
        .stdin(Stdio::null());
    cmd.spawn()
        .map_err(|e| AppError::io("起不了 npm").with_detail(e.to_string()))
}

/* ---------- 收掉整棵进程树 ---------- */

/// **一个进程的后代**（不含它自己），按"深的先"排。
///
/// 先收曾孙再收儿子：反过来的话，刚杀掉的中间层会把下面那几层一起带走
/// （Windows 上尤其明显），后面的 PID 就只剩一句"已经不在了"。
///
/// 抽成**只吃父子表的纯函数**是为了能单测 —— 真起一棵进程树来测这个排序太贵，
/// 而"谁是谁的后代"正是这里唯一会写错的地方。表里 `0` 表示"没有父进程"。
fn descendants_deepest_first(parents: &HashMap<u32, u32>, root: u32) -> Vec<u32> {
    let mut found: Vec<(usize, u32)> = Vec::new();
    for pid in parents.keys().copied() {
        if pid == root {
            continue;
        }
        let mut hops = 0usize;
        let mut cur = pid;
        while let Some(&parent) = parents.get(&cur) {
            if parent == 0 {
                break;
            }
            hops += 1;
            if parent == root {
                found.push((hops, pid));
                break;
            }
            if hops > MAX_ANCESTOR_HOPS {
                break;
            }
            cur = parent;
        }
    }
    /* 同深度按 PID 排：排序结果只由这张表决定，测试才钉得住 */
    found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    found.into_iter().map(|(_, pid)| pid).collect()
}

/// 把 `cmd → npm → node → npx tauri → cargo → vite …` 这一整棵收掉。
///
/// ★ **必须收整棵**：只杀包装层的话，8787 那个服务与客户端 dev 会变成孤儿 ——
/// 下一次「启动」会撞在占着的端口上，而界面上看不出是谁占的。
/// 杀法用 `sysinfo`（`release_tx` 已在用同一套，没有再拉依赖），逐个进程 kill。
fn kill_process_tree(root: u32) {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let parents: HashMap<u32, u32> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| (pid.as_u32(), proc.parent().map(|p| p.as_u32()).unwrap_or(0)))
        .collect();
    for pid in descendants_deepest_first(&parents, root) {
        if let Some(proc) = sys.process(Pid::from_u32(pid)) {
            proc.kill();
        }
    }
}

/* ---------- 端口：谁占着，怎么让它让开 ---------- */

/// 端口有没有人在听。**两个回环地址都试**：对面可能只绑了 v4，也可能只绑了 v6
/// （vite 绑的是 `localhost`，在有些机器上只落在 `::1`）。
fn port_taken(port: u16) -> bool {
    let v4 = SocketAddr::from(([127, 0, 0, 1], port));
    let v6 = SocketAddr::from(([0u16, 0, 0, 0, 0, 0, 0, 1], port));
    [v4, v6].iter().any(|a| TcpListener::bind(a).is_err())
}

/// `127.0.0.1:5321` / `[::1]:5321` / `0.0.0.0:5321` → `5321`
fn port_of_addr(addr: &str) -> Option<u16> {
    addr.rsplit_once(':')?.1.parse().ok()
}

/// 谁在听这个端口。**平台各一套**，与旧项目 `mkppanel/presets_server.go` 的
/// `findListenPid` 同一个思路：Windows 用 `netstat -ano`，类 Unix 用 `lsof`。
fn listen_pid(port: u16) -> Option<u32> {
    if cfg!(windows) {
        let out = Command::new("netstat").arg("-ano").output().ok()?;
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            /* TCP    127.0.0.1:5321    0.0.0.0:0    LISTENING    1234 */
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() < 5
                || !f[0].eq_ignore_ascii_case("tcp")
                || !f[3].eq_ignore_ascii_case("listening")
                || port_of_addr(f[1]) != Some(port)
            {
                continue;
            }
            if let Ok(pid) = f[4].parse() {
                return Some(pid);
            }
        }
        return None;
    }
    let out = Command::new("lsof")
        .args([
            "-nP".to_owned(),
            format!("-iTCP:{port}"),
            "-sTCP:LISTEN".to_owned(),
        ])
        .output()
        .ok()?;
    for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
        /* node 1234 user 21u IPv4 … TCP *:5321 (LISTEN) */
        if let Some(pid) = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()) {
            return Some(pid);
        }
    }
    None
}

/// PID → 进程名（`sysinfo` 只刷这一个，别为一行字去扫全表）。
fn process_name(pid: u32) -> Option<String> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), false);
    sys.process(Pid::from_u32(pid))
        .map(|p| p.name().to_string_lossy().into_owned())
}

/// 那个端口现在是谁的：PID + 名字。认不出（工具不在 / 输出格式变了）就 `None` ——
/// 界面照实说"认不出是谁"，不编。
fn listen_owner(port: u16) -> Option<(u32, String)> {
    let pid = listen_pid(port)?;
    Some((
        pid,
        process_name(pid).unwrap_or_else(|| "认不出名字".to_owned()),
    ))
}

/// 那句人话。**事实只有一处**：界面直接摆 `text`，不重写一遍措辞。
fn describe_conflict(c: &PortConflict) -> String {
    match (c.pid, c.process.as_deref()) {
        (Some(pid), Some(name)) => format!("{}（{}）被 {}（PID {}）占着", c.port, c.role, name, pid),
        (Some(pid), None) => format!("{}（{}）被 PID {} 占着", c.port, c.role, pid),
        _ => format!("{}（{}）被占着（认不出是谁）", c.port, c.role),
    }
}

/// 服务要用的那个端口上有没有别人。**这就是"别让人去终端里猜"的那一眼**：
/// 上一轮留下的服务还在、或者你手动起了一份 `preset-source:dev`，这一份就起不来。
/// 摆出来的是"谁占着"，问的是"要不要把它停掉"。
fn conflicts_of() -> Vec<PortConflict> {
    let mut out: Vec<PortConflict> = Vec::new();
    let p = port();
    if !port_taken(p) {
        return out;
    }
    let (pid, process) = match listen_owner(p) {
        Some((pid, name)) => (Some(pid), Some(name)),
        None => (None, None),
    };
    let mut c = PortConflict {
        port: p,
        role: "本地测试源要用的端口".to_owned(),
        pid,
        process,
        text: String::new(),
    };
    c.text = describe_conflict(&c);
    out.push(c);
    out
}

/// 「不齐」那句话说全（缺什么 + 怎么补）——给界面直接摆，不重写措辞。
impl SourceKind {
    fn missing_sentence(self, marker: &Path) -> String {
        let e = self.missing(marker);
        match e.detail {
            Some(d) => format!("{} —— {d}", e.message),
            None => e.message,
        }
    }
}

/// 三行单选的内容：每一份**现在**在哪儿、客户端填什么地址、齐没齐。
///
/// ★ 每一份都现算：`delivery` 那一份随**模式**走（测试模式开着就是沙箱的预设根）——
/// 模式一切，这一行的根与"齐没齐"立刻就是新那棵树的读数，界面不必自己推一遍。
fn sources_of(repo: &Path) -> Vec<SourceOption> {
    let presets = crate::workbench::paths::presets_root_path();
    [SourceKind::V1, SourceKind::V2, SourceKind::Delivery]
        .into_iter()
        .map(|kind| {
            let served = kind.derive(repo, &presets);
            let marker = kind.marker(&served);
            let ready = marker.is_file();
            let (label, note) = match kind {
                SourceKind::V1 => (
                    "夹具 v1",
                    "模拟「官方源，还没有新版本」—— 客户端那边该显示「已下载、没有新版」",
                ),
                SourceKind::V2 => (
                    "夹具 v2",
                    "模拟「官方发了新版」—— 同一份预设的下一版（revision 变了）",
                ),
                SourceKind::Delivery => (
                    "当前交付",
                    "端你生成出来的那一份（测试模式开着 = 沙箱那份）—— 验「我改的东西客户端拿不拿得到」",
                ),
            };
            SourceOption {
                kind,
                label: label.to_owned(),
                note: note.to_owned(),
                root: served.root.display().to_string(),
                url: served.url,
                ready,
                missing: (!ready).then(|| kind.missing_sentence(&marker)),
            }
        })
        .collect()
}

/// 起不来时那句总结（错误消息用）。
fn conflict_sentence(conflicts: &[PortConflict]) -> String {
    let parts: Vec<&str> = conflicts.iter().map(|c| c.text.as_str()).collect();
    format!("起不来 —— {}", parts.join("；"))
}

/// 停掉一个进程：**先走 `sysinfo`**（`release_tx` 已在用同一套），
/// **拿不到再退回系统命令**（`taskkill /F` / `kill -9`）—— 权限不对时前者会悄悄失败。
fn kill_pid(pid: u32) -> Result<(), AppError> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), false);
    if let Some(p) = sys.process(Pid::from_u32(pid)) {
        if p.kill() {
            return Ok(());
        }
    }
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("taskkill");
        c.args(["/F".to_owned(), "/PID".to_owned(), pid.to_string()]);
        c
    } else {
        let mut c = Command::new("kill");
        c.args(["-9".to_owned(), pid.to_string()]);
        c
    };
    let out = cmd
        .output()
        .map_err(|e| AppError::io("停不了那个进程").with_detail(e.to_string()))?;
    if out.status.success() {
        return Ok(());
    }
    Err(AppError::permission_denied(format!("停不了 PID {pid} 那个进程"))
        .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()))
}

/* ---------- 状态视图 ---------- */

/// 退出码那句人话。`code() == None` = 被信号带走的（Unix）
fn exit_text(st: ExitStatus) -> String {
    match st.code() {
        Some(0) => "它自己退出了（退出码 0）".to_owned(),
        Some(c) => format!("它退出了（退出码 {c}）—— 刚才终端里那几条日志是原因"),
        None => "它被信号带走了（多半是在终端里按了 Ctrl+C）".to_owned(),
    }
}

fn base_status(repo: &Path, note: Option<String>) -> DevSourceStatus {
    DevSourceStatus {
        running: false,
        pid: None,
        url: origin(),
        source: None,
        source_root: String::new(),
        repo_root: repo.display().to_string(),
        command: format!("npm run {NPM_SCRIPT}"),
        note,
        /* 不在跑的时候才探端口：跑着的时候 8787 本来就是我们自己占着 */
        conflicts: conflicts_of(),
        sources: sources_of(repo),
    }
}

fn status_of(cur: &DevSource, repo: &Path) -> DevSourceStatus {
    let mut st = base_status(repo, cur.exited.clone());
    st.pid = Some(cur.pid);
    st.running = cur.exited.is_none();
    /* 端的哪一份 / 端的哪个目录 / 客户端填哪一串：**在跑没在跑都报**——
       界面据此把那一行选中摆在"上次起的是它"上，地址也跟着换成它的那一串 */
    st.source = Some(cur.source);
    st.source_root = cur.root.display().to_string();
    st.url = cur.url.clone();
    if st.running {
        st.conflicts.clear();
    } else if let Some(first) = st.conflicts.first() {
        /* 它自己退了 → 把"多半为什么"也指出来。通跑踩出来的正是这一条：退出码 1
           背后是 5321 被占着，而那句话原来在界面上根本看不见（只有终端里有） */
        let why = format!("多半是 {} 被占着顶下来的", first.port);
        st.note = Some(match st.note.take() {
            Some(n) => format!("{n} —— {why}"),
            None => why,
        });
    }
    st
}

/// 现问一次子进程，把"它自己退了"这件事记下来（记过一次就不再问）。
fn refresh_exit(cur: &mut DevSource) {
    if cur.exited.is_some() {
        return;
    }
    match cur.child.try_wait() {
        Ok(None) => {}
        Ok(Some(st)) => cur.exited = Some(exit_text(st)),
        Err(e) => cur.exited = Some(format!("看不到它的状态：{e}")),
    }
}

/* ---------- 命令 ---------- */

/// 等它真的走（最多 `timeout`）。**有界**：等不到就往下走 —— 端口那一眼会如实
/// 报出"还占着"，而不是把界面卡在这儿。
fn wait_gone(child: &mut Child, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 起本地测试源（= `npm run preset-source:dev`，**只起服务**），端 `source` 那一份。
///
/// **同一份幂等**：已经在跑的就是它，那就不动，把现状报回去 —— 第二份会撞在 8787 上，
/// 然后那个进程静悄悄地死掉，而人以为自己点了两下、起了两个。
///
/// **换了源 = 换源重启**：把那棵进程树整棵收掉，等端口让出来，再以新的那一份起。
/// 一件事一次点击，不用"先停再起"两步（中间那一步人容易忘，然后就撞端口了）。
///
/// **先验齐没齐，再动进程**：选了一份还没派生的夹具 / 还没生成的交付时**如实拒绝** ——
/// 而且是在**杀掉手上那一份之前**就拒绝，选错了不该把正在跑的服务带下水。
///
/// **起之前先探端口**（[`conflicts_of`]）：被占着就如实拒绝，把那句人话（谁占着、
/// PID 多少）交出去，界面据此摆出「停掉它」那颗按钮 —— 而不是硬起、让它撞死在那里、
/// 只留一句"退出码 1"。
#[tauri::command]
pub fn wb_dev_source_start(source: SourceKind) -> Result<DevSourceStatus, AppError> {
    crate::ipc::traced("wb_dev_source_start", |_| {
        let repo = crate::workbench::paths::repo_root();
        let presets = crate::workbench::paths::presets_root_path();
        let served = source.derive(&repo, &presets);
        let marker = source.marker(&served);
        if !marker.is_file() {
            return Err(source.missing(&marker));
        }

        let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());

        /* 手上那一份还在跑的：同一份 → 幂等；换了源 → 先整棵收掉 */
        let stale = match guard.as_mut() {
            Some(cur) => {
                refresh_exit(cur);
                if cur.running_now() {
                    if cur.source == source {
                        return Ok(status_of(cur, &repo));
                    }
                    Some(cur.pid)
                } else {
                    None
                }
            }
            None => None,
        };
        if let Some(pid) = stale {
            kill_process_tree(pid);
            if let Some(mut cur) = guard.take() {
                wait_gone(&mut cur.child, Duration::from_secs(5));
            }
        }

        /* ★ 端口那一眼：起之前把那个端口的占用者摆出来，而不是让它起一半死掉 */
        let conflicts = conflicts_of();
        if !conflicts.is_empty() {
            return Err(AppError::invalid_argument(conflict_sentence(&conflicts)).with_detail(
                "界面会把这几条摆出来，并给一颗「停掉它」（`wb_dev_source_clear_conflict`）"
                    .to_owned(),
            ));
        }
        let child = spawn_server(&repo, &served)?;
        let pid = child.id();
        tracing::info!(
            pid,
            source = ?source,
            root = %served.root.display(),
            "本地测试源起来了（npm run {NPM_SCRIPT}）"
        );
        let cur = DevSource {
            child,
            pid,
            source,
            root: served.root,
            url: served.url,
            exited: None,
        };
        let st = status_of(&cur, &repo);
        *guard = Some(cur);
        Ok(st)
    })
}

/// 停掉它（整棵进程树），把 8787 让出来。
///
/// ★ 收的是**整棵**：`npm` 底下那层进程若不一起收，服务就会变成孤儿占着端口。
/// 这里只起服务、不碰客户端，所以收掉的也只是这一条。
///
/// 没在跑时**幂等成功** —— 界面上的按钮不该因为"其实已经没了"而报错。
#[tauri::command]
pub fn wb_dev_source_stop() -> Result<DevSourceStatus, AppError> {
    crate::ipc::traced("wb_dev_source_stop", |_| {
        let repo = crate::workbench::paths::repo_root();
        let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
        let Some(mut cur) = guard.take() else {
            return Ok(base_status(&repo, Some("本来就没在跑".to_owned())));
        };
        kill_process_tree(cur.pid);
        /* 包装层自己收一次尸：Windows 上 PID 复用很快，不 wait 就分不清
           "我杀掉的那个"和"刚好新起来的另一个" */
        let _ = cur.child.kill();
        /* ★ **不报退出码**：这一下是**我们**按的信号，那个码只是终止的结果，
           摆到人眼前会被读成"出错了"（2026-10-08 烟测里读出来的原话：
           「已停（它退出了（退出码 1）—— 刚才终端里那几条日志是原因）」）。
           该说的是"端口让出来没有"——那才是人关心的那一件事 */
        let note = match cur.child.wait() {
            Ok(_) => {
                let p = port();
                let t0 = Instant::now();
                while t0.elapsed() < Duration::from_secs(3) {
                    if !port_taken(p) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(120));
                }
                if port_taken(p) {
                    format!("已停，但 {p} 还被占着 —— 可能还有别的进程在听")
                } else {
                    format!("已停（{p} 让出来了）")
                }
            }
            Err(e) => format!("已停（不过收尸时出了点岔子：{e}）"),
        };
        tracing::info!(pid = cur.pid, "本地测试源已停（端口让出来了）");
        Ok(base_status(&repo, Some(note)))
    })
}

/// **停掉占着某个端口的那个进程**（界面上那颗「停掉它」）。
///
/// ★ **先验明正身再动手**：前端把它当时看到的 `pid` 交回来，这里重新问一次
/// "这个端口现在是谁的" —— 对不上就拒绝（PID 会被复用，照
/// `release_tx::wb_release_kill_dev_watcher` 那条纪律：宁可让人自己动手，也不误伤）。
///
/// **幂等**：端口上没人了就直接说"不用停"，不报错（界面那颗按钮不该因为
/// "其实已经没了"而炸）。
#[tauri::command]
pub fn wb_dev_source_clear_conflict(
    port: u16,
    pid: Option<u32>,
) -> Result<DevSourceStatus, AppError> {
    crate::ipc::traced("wb_dev_source_clear_conflict", |_| {
        let repo = crate::workbench::paths::repo_root();
        {
            let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cur) = guard.as_mut() {
                refresh_exit(cur);
                if cur.running_now() {
                    return Err(AppError::invalid_argument(
                        "本地测试源正在跑 —— 先点「停止」，再处理端口上的占用者",
                    ));
                }
            }
        }

        if !port_taken(port) {
            return Ok(base_status(
                &repo,
                Some(format!("{port} 上已经没人了 —— 不用停")),
            ));
        }
        let Some((now_pid, name)) = listen_owner(port) else {
            return Err(AppError::permission_denied(format!(
                "认不出占着 {port} 的是哪个进程（没拿到 PID）—— 请在那个终端里自己把它停掉"
            )));
        };
        if let Some(said) = pid {
            if said != now_pid {
                return Err(AppError::invalid_argument(format!(
                    "{port} 现在的占用者换人了：你看到的是 PID {said}，现在是 {name}（PID {now_pid}）—— 拒绝动手，请刷新再看一眼"
                )));
            }
        }

        kill_pid(now_pid)?;
        /* 等端口真的让出来（**有界**）：进程收到了终止信号，但端口未必立刻释放 */
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(3) {
            if !port_taken(port) {
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        let note = if port_taken(port) {
            format!("{name}（PID {now_pid}）已经收到终止，但 {port} 还被占着 —— 可能还有别的进程在听")
        } else {
            format!("已停掉占着 {port} 的 {name}（PID {now_pid}）")
        };
        tracing::info!(port, pid = now_pid, name = %name, "端口占用者已停");
        Ok(base_status(&repo, Some(note)))
    })
}

/// 现在到底在不在跑。界面按它画按钮与状态那两行 —— **每次现问**，
/// 所以"人在终端里 Ctrl+C 了"这件事也看得见。
#[tauri::command]
pub fn wb_dev_source_status() -> Result<DevSourceStatus, AppError> {
    crate::ipc::traced("wb_dev_source_status", |_| {
        let repo = crate::workbench::paths::repo_root();
        let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(cur) => {
                refresh_exit(cur);
                Ok(status_of(cur, &repo))
            }
            None => Ok(base_status(&repo, None)),
        }
    })
}

impl DevSource {
    /// 还活着吗（`exited` 没填上就是）。给 `start` 的幂等判断用
    fn running_now(&self) -> bool {
        self.exited.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parents(pairs: &[(u32, u32)]) -> HashMap<u32, u32> {
        pairs.iter().copied().collect()
    }

    /// 深的后代排前面，且只收这一棵的
    #[test]
    fn descendants_are_collected_deepest_first() {
        let t = parents(&[
            (50, 0),   // 我们的包装层（root 自己）
            (100, 50), // 儿子
            (200, 100),
            (300, 200), // 曾孙
            (999, 1),   // 别人家的树
        ]);
        assert_eq!(descendants_deepest_first(&t, 50), vec![300, 200, 100]);
    }

    /// 起点自己与不相干的进程一个都不许动
    #[test]
    fn the_root_itself_and_strangers_are_left_alone() {
        let t = parents(&[(10, 1), (11, 10), (12, 1), (13, 12)]);
        assert_eq!(descendants_deepest_first(&t, 10), vec![11]);
        /* 1 是它俩共同的爹：孙子（11/13）先，儿子（10/12）后 */
        assert_eq!(descendants_deepest_first(&t, 1), vec![13, 11, 12, 10]);
        assert!(descendants_deepest_first(&t, 777).is_empty());
    }

    /// 成环的父子表**不许把循环挂死** —— 宁可少杀几个，也不能僵在那里
    #[test]
    fn a_cycle_never_hangs() {
        let t = parents(&[(1, 2), (2, 1)]);
        assert!(descendants_deepest_first(&t, 7).is_empty());
    }

    /// 没有父进程（`0`）的那一层到此为止，不再往上找
    #[test]
    fn an_orphan_chain_stops_at_zero() {
        let t = parents(&[(5, 0), (6, 5)]);
        assert_eq!(descendants_deepest_first(&t, 5), vec![6]);
    }

    /// 端口跟着 `PRESET_TEST_PORT` 走 —— 与 `server.mjs` / `dev.mjs` 同一个变量，
    /// 认不出的值一律退回默认（不能在界面上报一个服务其实没在听的地址）
    #[test]
    fn the_port_follows_the_same_env_var_as_the_server() {
        assert_eq!(port_from(None), DEFAULT_PORT);
        assert_eq!(port_from(Some("9000")), 9000);
        assert_eq!(port_from(Some(" 9001 ")), 9001);
        assert_eq!(port_from(Some("")), DEFAULT_PORT);
        assert_eq!(port_from(Some("abc")), DEFAULT_PORT);
        assert_eq!(port_from(Some("0")), DEFAULT_PORT);
        assert_eq!(port_from(Some("70000")), DEFAULT_PORT);
    }

    /// 我们点的就是 `package.json` 里真有的那条 script：改了名字这里会红，
    /// 而不是等人点了按钮才发现"没有这条 script"
    #[test]
    fn the_npm_script_we_run_exists_in_package_json() {
        let pkg = crate::workbench::paths::repo_root().join("package.json");
        let text = std::fs::read_to_string(&pkg)
            .unwrap_or_else(|e| panic!("读不了 {}：{e}", pkg.display()));
        let value: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} 不是 JSON：{e}", pkg.display()));
        assert!(
            value["scripts"][NPM_SCRIPT].is_string(),
            "package.json 的 scripts 里没有 {NPM_SCRIPT}"
        );
    }

    /// **真起一棵 node 进程树（node → node），验整棵被收掉**（`--ignored` 才跑）。
    ///
    /// 为什么值得有这一条：`descendants_deepest_first` 是纯函数、好测，但
    /// "**收了之后它们真的不在进程表里了**"只有真起一次才答得了 —— 而这正是
    /// 「停止」那颗按钮唯一的承诺（漏收一个，8787 就留在那儿，下一次「启动」撞端口）。
    ///
    /// 为什么 `#[ignore]`：它要这台机器上有 `node`（本仓的 dev 流程本来就要），
    /// 而且真起进程。**显式跑才有意义**：
    /// `cargo test --lib --features workbench -- --ignored smoke_kill_tree`
    ///
    /// **处处有界**：收树那一步放到独立线程里、只等 10 秒 —— 真机上点「停止」
    /// 卡住界面的那种坏法，在这里会变成一条失败，而不是一条把测试套件挂死的用例。
    #[test]
    #[ignore]
    fn smoke_kill_tree() {
        let script = "require('child_process').spawn(process.execPath,\
                      ['-e','setInterval(function(){},1e3)'],{stdio:'ignore'});\
                      setInterval(function(){},1e3)";
        let mut root = Command::new("node")
            .args(["-e", script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("起不了 node（这台机器上没有 node 就该忽略这条）");
        let root_pid = root.id();

        /* 等儿子生出来：最多等 5 秒 */
        let mut child_pid = None;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut sys = System::new();
            sys.refresh_processes(ProcessesToUpdate::All, true);
            child_pid = sys
                .processes()
                .iter()
                .find(|(_, p)| p.parent().map(|q| q.as_u32()) == Some(root_pid))
                .map(|(pid, _)| pid.as_u32());
            if child_pid.is_some() {
                break;
            }
        }
        let child_pid = child_pid.expect("儿子没生出来（node 起了但没 spawn？）");

        /* 收树：独立线程 + 10 秒上限 */
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            kill_process_tree(root_pid);
            let _ = tx.send(());
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(10)).is_ok(),
            "kill_process_tree 超过 10 秒没回来 —— 生产里点「停止」会卡住界面"
        );

        let _ = root.kill();
        let _ = root.wait();
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut sys = System::new();
            sys.refresh_processes(ProcessesToUpdate::All, true);
            let alive = |pid: u32| sys.process(Pid::from_u32(pid)).is_some();
            if !alive(root_pid) && !alive(child_pid) {
                return;
            }
        }
        panic!("收完还活着（包装层 {root_pid} / 儿子 {child_pid}）—— 只杀到包装层");
    }

    /// 端口从地址里剥出来：`netstat` 给的形式有两种（v4 / v6）
    #[test]
    fn the_port_is_read_out_of_the_address() {
        assert_eq!(port_of_addr("127.0.0.1:5321"), Some(5321));
        assert_eq!(port_of_addr("0.0.0.0:8787"), Some(8787));
        assert_eq!(port_of_addr("[::1]:5321"), Some(5321));
        assert_eq!(port_of_addr("[::]:8787"), Some(8787));
        assert_eq!(port_of_addr("127.0.0.1"), None);
        assert_eq!(port_of_addr("*:*"), None);
    }

    /// **三份源各自端哪儿、客户端填什么地址** —— 一行一行钉住。
    ///
    /// 最要紧的是 `delivery` 那一条：它端的目录是**预设根**、填的地址带 `/delivery`。
    /// 真交付根的 `source.json` 写着 `filesRoot: ".."`（交付文件 `mkp/presets/*.toml`
    /// 住在预设根底下，不在 `delivery/` 里），所以只端 `delivery/` 那一层的话，
    /// 客户端按 `..` 解出来的文件全在服务根外面 —— 一个一个都是 403。
    /// 这条链就是在这一处悄悄断的，值得专门钉一条。
    #[test]
    fn each_source_serves_the_root_that_makes_files_addressable() {
        let repo = Path::new("/tmp/repo");
        let presets = Path::new("/tmp/repo/presets");

        let v1 = SourceKind::V1.derive(repo, presets);
        assert!(
            v1.root.ends_with("fixtures/v1"),
            "v1 端的是夹具根：{}",
            v1.root.display()
        );
        assert_eq!(v1.url, origin(), "夹具自己就是一个交付根，地址填到根上");

        let v2 = SourceKind::V2.derive(repo, presets);
        assert!(
            v2.root.ends_with("fixtures/v2"),
            "v2 端的是夹具根：{}",
            v2.root.display()
        );
        assert_eq!(v2.url, v1.url, "两代夹具同一个地址（换的是内容，不是端口）");
        assert_ne!(v1.root, v2.root, "两代必须是两个不同的目录");

        let d = SourceKind::Delivery.derive(repo, presets);
        assert_eq!(
            d.root, presets,
            "交付那一份端的是**预设根**（filesRoot: \"..\" 得解得到那儿）"
        );
        assert_eq!(
            d.url,
            format!("{}/delivery", origin()),
            "客户端要填 <地址>/delivery，不是裸地址"
        );
    }

    /// **「当前交付」跟着调用方给的预设根走** —— 工作台传的是
    /// `paths::presets_root_path()`（**模式感知**），于是测试模式开着时端出去的就是
    /// 沙箱里生成的那份交付，正式那份从头到尾不参与。这正是闭环那一环的要害。
    #[test]
    fn the_delivery_source_serves_the_presets_root_it_is_given() {
        let repo = Path::new("/tmp/repo");
        let real = repo.join("presets");
        let sandbox = repo.join("workbench").join(".sandbox").join("presets");
        let a = SourceKind::Delivery.derive(repo, &real);
        let b = SourceKind::Delivery.derive(repo, &sandbox);
        assert_eq!(a.root, real);
        assert_eq!(b.root, sandbox);
        assert_ne!(a.root, b.root, "两份预设根必须端出两个不同的目录");
        /* 缺的那份标志文件也各说各的（界面据此告诉人"去哪儿生成"） */
        assert!(SourceKind::Delivery.marker(&a).ends_with("delivery/source.json"));
        assert_ne!(
            SourceKind::Delivery.marker(&a),
            SourceKind::Delivery.marker(&b)
        );
    }

    /// 不齐的两句人话**各指各的补法**：夹具去 `preset-source:make`，交付去点「生成」——
    /// 指错方向（让还没生成过的人去跑 make）比不提示还坏
    #[test]
    fn a_source_that_is_not_ready_says_how_to_make_it_ready() {
        let repo = Path::new("/tmp/repo");
        let presets = Path::new("/tmp/repo/presets");

        let served = SourceKind::V1.derive(repo, presets);
        let fixture = SourceKind::V1.missing_sentence(&served.root.join(FIXTURE_MARKER));
        assert!(fixture.contains("preset-source:make"), "实测：{fixture}");

        let served = SourceKind::Delivery.derive(repo, presets);
        let delivery = SourceKind::Delivery
            .missing_sentence(&served.root.join("delivery").join(DELIVERY_MARKER));
        assert!(delivery.contains("生成"), "实测：{delivery}");
        assert!(
            !delivery.contains("preset-source:make"),
            "别把交付那一条指去跑夹具：{delivery}"
        );
    }

    /// 界面上那三行：**名字与说法由后端给**，三份都在、都带根与地址、齐没齐都说得出
    /// （界面不自己拼一套措辞，也就不存在"两处说法不一样"）
    #[test]
    fn the_three_rows_are_told_by_the_backend() {
        let rows = sources_of(Path::new("/tmp/repo"));
        assert_eq!(rows.len(), 3, "三行单选：v1 / v2 / 当前交付");
        assert_eq!(rows[0].kind, SourceKind::V1);
        assert_eq!(rows[1].kind, SourceKind::V2);
        assert_eq!(rows[2].kind, SourceKind::Delivery);
        for r in &rows {
            assert!(!r.label.is_empty() && !r.note.is_empty(), "{:?}", r.kind);
            assert!(r.url.starts_with("http://127.0.0.1:"), "{:?}", r.kind);
            assert!(!r.root.is_empty(), "{:?}", r.kind);
            /* 齐 / 不齐两种情形都要说得出话：不齐时补齐那句，齐时就没有那句 */
            assert_eq!(r.missing.is_none(), r.ready, "{:?}", r.kind);
        }
        assert!(
            rows[2].note.contains("沙箱"),
            "「当前交付」那一行要说清它跟着模式走：{}",
            rows[2].note
        );
    }

    /// 三档说法：认得名 / 只有 PID / 认不出 —— **认不出时不许编一个名字出来**
    #[test]
    fn a_conflict_says_who_holds_the_port_or_admits_it_cannot_tell() {
        let mk = |pid: Option<u32>, process: Option<&str>| {
            let mut c = PortConflict {
                port: 5321,
                role: "客户端 dev 的 vite 端口".to_owned(),
                pid,
                process: process.map(str::to_owned),
                text: String::new(),
            };
            c.text = describe_conflict(&c);
            c
        };
        assert_eq!(
            mk(Some(1234), Some("node.exe")).text,
            "5321（客户端 dev 的 vite 端口）被 node.exe（PID 1234）占着"
        );
        assert_eq!(
            mk(Some(1234), None).text,
            "5321（客户端 dev 的 vite 端口）被 PID 1234 占着"
        );
        assert!(mk(None, None).text.contains("认不出是谁"));

        /* 起不来那句总结：几条并成一句、以"起不来"开头 */
        let s = conflict_sentence(&[mk(Some(1), Some("node.exe"))]);
        assert!(s.starts_with("起不来"), "实测「{s}」");
        assert!(s.contains("PID 1"));
    }

    /// 报给界面的那条命令与真起的那条是同一个（界面不自己拼一遍，也就不该漂）——
    /// **而且是"只起服务"的那一条**（作者 2026-10-08 裁决：别去动客户端）
    #[test]
    fn the_reported_command_is_the_one_we_spawn() {
        let st = base_status(Path::new("/tmp/repo"), None);
        assert_eq!(st.command, "npm run preset-source:dev");
        assert!(st.url.starts_with("http://127.0.0.1:"));
        assert!(!st.running && st.pid.is_none());
        /* 从没起过：不谎报"端的是哪一份"（免得界面把某一行当成"正在端"） */
        assert!(st.source.is_none() && st.source_root.is_empty());
        assert_eq!(st.sources.len(), 3);
    }

    /// **端哪一份靠 `PRESET_TEST_ROOT` 交下去**，而它得是 `server.mjs` 真读的那个名字 ——
    /// 改名 / 拼错会表现成"服务起得来、但端的是默认那份"（夹具 v1），
    /// 那是一种看不出错的错：界面上写着"当前交付"，客户端拿到的还是 v1。
    #[test]
    fn the_root_env_var_is_the_one_the_server_reads() {
        let server = crate::workbench::paths::repo_root()
            .join("scripts")
            .join("preset-test-server")
            .join("server.mjs");
        let text = std::fs::read_to_string(&server)
            .unwrap_or_else(|e| panic!("读不了 {}：{e}", server.display()));
        assert!(
            text.contains(ROOT_ENV),
            "server.mjs 不认 {ROOT_ENV} —— 换源那一下会静悄悄地端回默认那份"
        );
        /* 而且它必须比默认值优先：`--root` 参数仍然最先认 */
        assert!(
            text.contains("--root") && text.contains(ROOT_ENV),
            "server.mjs 那边 `--root` 与 {ROOT_ENV} 得都在"
        );
    }
}
