//! 「本地测试源（开发）」—— 工作台里那颗**一键起本地假云端**的按钮。
//!
//! # 它解决的那件事
//!
//! 客户端要验「官方发了新版本 → 检查更新 → 下载 → 对比 → 恢复默认」这一整条链，
//! 今天得先去终端敲 `npm run dev:test-update`。可工作台自己就是**从仓库里跑起来**的，
//! 人在工作台里干活时不该再切到终端 —— 这一颗按钮就是那条命令。
//!
//! ```text
//! 按钮 ──► npm run dev:test-update （cwd = 仓库根）
//!            ├─ 夹具不在就先派生（make-fixtures）
//!            ├─ 起 scripts/preset-test-server/ 在 127.0.0.1:8787
//!            └─ 以 MKPSE_PRESET_SOURCE_URL=<那地址> 起客户端 tauri dev
//! ```
//!
//! # 为什么从工作台起是安全的
//!
//! 两份 dev 会话本来就是**分开的两条路**（`tauri-workbench.mjs` 头注那段）：
//! vite 端口 5321 / 5322，cargo 产物目录 `target/` / `target-workbench/`。
//! 所以"工作台在跑、再起客户端 dev"不会撞端口、也不会撞 Windows 那个
//! "不许覆盖正在运行的可执行文件"。
//!
//! # 边界
//!
//! - **不感知测试源是什么**：这一层只起命令、报状态。它不读夹具、不写任何配置、
//!   更不去改客户端的「预设数据源」那一格（那是运行时那份设置，住客户端自己的
//!   `appDataDir/run/app-state.json`，两个应用连 appDataDir 都不一样）。
//!   源地址是 `MKPSE_PRESET_SOURCE_URL` 注入进去的 —— 只在 debug 构建里认。
//! - **只在工作台构建里存在**（`src/workbench/` 整个子树挂在 `--features workbench`
//!   那道闸后面），给用户的二进制里既没有这颗按钮，也没有这三条命令。
//! - 输出**继承给终端**：服务与客户端 dev 的日志打在起工作台的那个终端里。
//!   这个按钮只是"替你敲了那条命令"，不另造一套日志窗口充当真相。
//! - **不拦"客户端 dev 已经在跑"**（那是最常见的一种点不动的原因）：那一份没法被
//!   重新指源（源是它启动时的环境变量），而新起的这一份会撞在同一个 vite 端口
//!   （5321，`strictPort`）与同一个可执行文件上。★ 但这件事**不在这里猜** ——
//!   进程的命令行 / 工作目录在某些环境里读不到（本机实测：`sysinfo` 对所有进程都
//!   返回空），猜错就成了"按钮点了没反应"，比不拦更坏。所以照常起，原因由那条命令
//!   自己打在终端里（`Port 5321 is in use`），界面把"它退出了（退出码 N）——
//!   日志在终端"如实说出来。

use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::error::AppError;

/// 起它的那条 npm script。**名字是契约**：`package.json` 里没有这一条，
/// 这颗按钮点的就是一个空壳（下面有测试钉着）。
const NPM_SCRIPT: &str = "dev:test-update";

/// 测试源的默认端口 —— 与 `scripts/preset-test-server/server.mjs` 同一个值。
/// 它读 `PRESET_TEST_PORT`（见 [`port_from`]），这里只是那个变量的默认档。
const DEFAULT_PORT: u16 = 8787;

/// 沿父指针往上找祖先时最多爬几层。**必须有上限**：父子表是从活进程里现取的，
/// 万一取到一张成环的表，没有上限的循环会把工作台卡死。
const MAX_ANCESTOR_HOPS: usize = 32;

/* ---------- DTO ---------- */

/// 本地测试源现在什么样。**`running` 每次现问子进程**（`try_wait`），
/// 不是我们自己记的一个布尔 —— 那个进程可能是用户在终端里 Ctrl+C 掉的。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSourceStatus {
    /// 那条 `npm run dev:test-update` 还活着吗
    pub running: bool,
    /// 包装层（Windows 上是 `cmd`）的 PID。不在跑时 `null`
    pub pid: Option<u32>,
    /// 服务地址 —— `MKPSE_PRESET_SOURCE_URL` 注入进客户端的**就是它**。
    /// 想只手起服务、然后在客户端设置页手填地址的人，填这一串
    pub url: String,
    /// 它在哪个仓库根下跑（命令的 cwd）
    pub repo_root: String,
    /// 实际跑的那条命令（给人对账用，界面不自己拼一遍）
    pub command: String,
    /// 不在跑的时候为什么：退出码那句 / "本来就没在跑"。在跑时 `null`
    pub note: Option<String>,
}

/* ---------- 进程状态 ---------- */

/// 那个跑着的子进程，以及它退出之后我们才知道的那件事。
struct DevSource {
    child: Child,
    pid: u32,
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

/// 起 `npm run dev:test-update`（cwd = 仓库根）。
///
/// ★ **Windows 上必须过一层 `cmd /C`**：那边 `npm` 是 `npm.cmd`，而
/// `Command::new("npm")` 走 `CreateProcess`，它**不替你补 `.cmd` 后缀** ——
/// 直接起会报"程序找不到"。与 `release_tx::npm_build` 同一条理由。
fn spawn_dev_update(repo: &Path) -> Result<Child, AppError> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("npm");
        c
    } else {
        Command::new("npm")
    };
    cmd.args(["run", NPM_SCRIPT])
        .current_dir(repo)
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
        repo_root: repo.display().to_string(),
        command: format!("npm run {NPM_SCRIPT}"),
        note,
    }
}

fn status_of(cur: &DevSource, repo: &Path) -> DevSourceStatus {
    let mut st = base_status(repo, cur.exited.clone());
    st.pid = Some(cur.pid);
    st.running = cur.exited.is_none();
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

/// 起本地测试源（= `npm run dev:test-update`）。
///
/// **幂等**：已经在跑就把现状报回去，不再起第二个 —— 第二份会撞在 8787 上，
/// 然后那个进程静悄悄地死掉，而人以为自己点了两下、起了两个。
///
/// ★ **不拦"已经有一个客户端 dev 在跑"**：那一份没法被重新指源（源是它启动时的
/// 环境变量），而这一份会撞在同一个 vite 端口与同一个可执行文件上。但那件事
/// **不在这里猜** —— 进程的命令行 / 工作目录在某些环境里读不到（本机实测：sysinfo
/// 对所有进程都返回空），猜错就变成"按钮点了没反应"，比不拦更坏。
/// 让它照常起，理由由那条命令自己打在终端里（`Port 5321 is in use`），界面上的
/// `note` 会把"它退出了（退出码 N）—— 日志在终端"如实说出来。
#[tauri::command]
pub fn wb_dev_source_start() -> Result<DevSourceStatus, AppError> {
    crate::ipc::traced("wb_dev_source_start", |_| {
        let repo = crate::workbench::paths::repo_root();
        let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cur) = guard.as_mut() {
            refresh_exit(cur);
            /* 还活着 = 幂等返回；已经退了 = 换掉它，重新起一份 */
            if cur.running_now() {
                return Ok(status_of(cur, &repo));
            }
        }
        let child = spawn_dev_update(&repo)?;
        let pid = child.id();
        tracing::info!(pid, repo = %repo.display(), "本地测试源起来了（npm run {NPM_SCRIPT}）");
        let cur = DevSource {
            child,
            pid,
            exited: None,
        };
        let st = status_of(&cur, &repo);
        *guard = Some(cur);
        Ok(st)
    })
}

/// 停掉它**连同它起的客户端 dev**（整棵进程树），把 8787 让出来。
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
        let note = match cur.child.wait() {
            Ok(st) => format!("已停（{}）", exit_text(st)),
            Err(e) => format!("已停（不过收尸时出了点岔子：{e}）"),
        };
        tracing::info!(pid = cur.pid, "本地测试源已停（连同它起的客户端 dev）");
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

    /// 报给界面的那条命令与真起的那条是同一个（界面不自己拼一遍，也就不该漂）
    #[test]
    fn the_reported_command_is_the_one_we_spawn() {
        let st = base_status(Path::new("/tmp/repo"), None);
        assert_eq!(st.command, "npm run dev:test-update");
        assert!(st.url.starts_with("http://127.0.0.1:"));
        assert!(!st.running && st.pid.is_none());
    }
}
