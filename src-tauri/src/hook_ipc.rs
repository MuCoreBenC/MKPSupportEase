//! 钩子 ↔ 界面 的那条通道（一行一个 JSON，走 127.0.0.1）。
//!
//! # 为什么要有它（照旧世代 `mkp-ssr` 的 `progress_ipc.rs`）
//!
//! 切片器调我们的方式是「起一个子进程，**等它退出**，看退出码」。所以窗口**不能住在钩子
//! 进程里**：那样"跑完不自动关"就等于"切片器一直卡着"，"下次复用同一扇窗"更是做不到
//! （下一次是**新的**子进程）。把窗口搬进界面那个常驻进程之后三件事同时成立：
//!
//! ```text
//! 钩子进程（跑完就退，切片器继续）              界面进程（常驻：留着、下次复用）
//!   connect(127.0.0.1:<port>) ────────────►  TcpListener::bind（bind 即单实例）
//!   写 {"kind":"hello",…}\n
//!   写 {"kind":"progress",…}\n
//!   写 {"kind":"finished",…}\n
//!   ◄──────────────────────────────────────  写 {"kind":"cancel"}\n
//!   ◄──────────────────────────────────────  写 {"kind":"answer","keep":true}\n
//!   退出（带退出码）                            窗口留着显示结果，等下一次切片
//! ```
//!
//! 窗口留着、下次复用 —— **只有两个进程才同时做得到**这两件与"钩子跑完立刻退"。
//!
//! # 传输层**刻意换掉**了（与旧世代唯一不同的地方）
//!
//! 旧世代用 Unix domain socket，但它自己的 Windows 分支是**占位实现**（`connect` / `bind`
//! 恒返回 `Unsupported`，边界登记在它仓库的 `docs/handoff-windows-round.md`）。Windows 是
//! 我们的主场，所以这里用 `std::net` 的 **TCP + 一行一个 JSON**：同样零新依赖，同样四条纪律：
//!
//! 1. **`bind` 成功就是唯一那扇窗**（单实例不用锁文件）；已经有人在做显示时，钩子只 connect；
//! 2. **连不上就自己起一个**（无参数的界面进程），每 25ms 重试、最多等 [`SPAWN_WAIT`]；
//!    等不到就**当作这次没有界面**，照旧立刻开工；
//! 3. **写不出去就放弃这条通道**：本模块**没有任何 `?` 会传播到钩子的主流程**（产物优先于界面）；
//! 4. **`kind` 是稳定串**（`hello` / `progress` / `question` / `finished` / `cancel` / `answer`）
//!    —— 与内核 `Step::id()` 同一个理由：改个枚举名就静默失配，而失配的表现是"界面不动了"。
//!
//! 端点文件：`<内部根>/run/hook.port`（一行端口号）。钩子侧没有 `AppHandle`，根由
//! [`crate::fsx::paths::internal_root_headless`] 算出来（与界面侧同一个标识符，判据钉着）。
//!
//! # 一条连接 = 一次任务，一条连接只许一个读者
//!
//! 界面侧来一条连接就换一次"当前那一趟"（见 [`crate::hook_ui`]），连接断开 = 钩子退出了。
//! 钩子侧只有**一个读线程**：它把 `cancel` 直接接到取消线上（这样"按停止"能打断
//! "等你回答"那个等待），把 `answer` 转给等在 [`Client::ask`] 上的那个人。

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use postprocess::diag::CancelToken;

/// 端点文件（相对内部根）。
pub const PORT_FILE: &str = "run/hook.port";

/// 连不上界面时，自己起一个并等它把端点挂出来 —— 上限（旧世代同值）。
pub const SPAWN_WAIT: Duration = Duration::from_millis(1500);

/// 等用户答一句的上限（旧世代 180s；我们问的只有"机型不匹配还跑不跑"）。
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);

/// 每次 connect 的短超时：只是"探一探有没有人在听"，不该卡住。
const CONNECT_PROBE: Duration = Duration::from_millis(200);

/* ---------- 消息 ---------- */

/// 一条进度（推给界面，也留在界面那份快照里）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    /// 阶段 id（内核的稳定标识：`input` / `pass1` …）—— 中文名归界面
    pub step: String,
    /// 步内比例 `0.0..=1.0`；`None` = 这一步此刻给不出比例
    pub fraction_in_step: Option<f32>,
    pub message: String,
}

/// 要用户回答的一个问题（目前只有"机型不匹配还跑不跑"）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPayload {
    /// 问题种类（界面按它决定给什么按钮）：`machine-mismatch` = 继续 / 停下
    pub kind: String,
    pub text: String,
}

/// 那一趟的结论（成功 / 失败 / 取消都走这一条）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPayload {
    pub ok: bool,
    /// 用户按了停止（或 Ctrl-C）—— 不是"失败"，界面说法不一样
    pub cancelled: bool,
    /// **原因**（人话）。稳定错误码已经剥到 [`Self::code`] 里（见 `hook::describe`）
    pub message: String,
    /// 稳定错误码（`E_*_NNN`，报问题时带上它）；取消与没有码的几句是 `None`
    pub code: Option<String>,
    /// 停在哪一阶段（内核的阶段 id）；还没出过进度时是 `None`
    pub stage: Option<String>,
    /// 给切片器的退出码
    pub exit_code: u8,
    pub elapsed_ms: u64,
    /// 产物落点（成功时就是输入那一份 —— 原地覆盖）
    pub output: Option<String>,
    pub warnings: Vec<String>,
}

/// 钩子 → 界面。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FromHook {
    /// 开场：告诉界面这一趟在处理哪两份文件（界面拿它写标题）
    Hello { preset_name: String, gcode_name: String },
    Progress(ProgressPayload),
    Question(QuestionPayload),
    /// 打印时间估算**交给你了**：钩子写完盘就退，界面按 id 补全那条记录。
    ///
    /// 为什么钩子不能自己留后台任务：它必须**立刻退出**（切片器在等退出码）。
    /// 界面本来就常驻，是唯一合适的承接方。
    DeferPrintTime { record_id: String, output: String },
    /// 结论。之后钩子就退了（界面把它留在屏上，等下一次）
    Finished(Box<FinishedPayload>),
}

/// 界面 → 钩子。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ToHook {
    /// 界面那颗「停止」
    Cancel,
    /// 回答"机型不匹配还跑不跑"
    Answer { keep: bool },
}

/* ---------- 钩子那一侧 ---------- */

/// 钩子那一侧的线。**写不出去不是错误** —— 只是这次没有界面在看（产物照旧）。
///
/// 里面全是内部可变：进度出口（`&self`）与问句（`&self`）会**同时**被拿去用
/// （管线的 `sink` 与 `Asker` 是两个参数），所以不能要求 `&mut self`。
pub struct Client {
    conn: Mutex<Option<TcpStream>>,
    /// 答复从读线程转过来（只有 [`Client::ask`] 一个人消费）
    answers: Mutex<Option<Receiver<bool>>>,
    /// 取消线（读线程收到 `cancel` 直接置位 —— 这样它能**打断**等答复那个等待）
    cancel: CancelToken,
}

impl Client {
    /// 连界面；连不上就自己起一个（无参数的界面进程），最多等 [`SPAWN_WAIT`]。
    ///
    /// 起不来 / 等不到 ⇒ 返回一条"没有界面"的线：钩子照常干活，只是没人看进度
    /// （旧世代同一条取舍：头几步可能没人看见，产物不会少一个字节）。
    pub fn connect_or_spawn(internal_root: &Path, cancel: &CancelToken) -> Self {
        let deadline = Instant::now() + SPAWN_WAIT;
        let mut spawned = false;

        loop {
            if let Some(conn) = connect(internal_root) {
                return Self::with_conn(conn, cancel);
            }
            /* 先把界面拉起来，再每 25ms 试一次（总预算 SPAWN_WAIT） */
            if !spawned {
                spawn_display();
                spawned = true;
            }
            if Instant::now() >= deadline {
                return Self::headless(cancel);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// 一条没有界面的线（无头场景 / 判据用）。
    pub fn headless(cancel: &CancelToken) -> Self {
        Self {
            conn: Mutex::new(None),
            answers: Mutex::new(None),
            cancel: cancel.clone(),
        }
    }

    fn with_conn(conn: TcpStream, cancel: &CancelToken) -> Self {
        let cancel = cancel.clone();
        let answers = conn.try_clone().ok().map(|read_side| {
            let (tx, rx) = mpsc::channel::<bool>();
            let cancel = cancel.clone();
            std::thread::spawn(move || read_loop(read_side, tx, cancel));
            rx
        });
        Self {
            conn: Mutex::new(Some(conn)),
            answers: Mutex::new(answers),
            cancel,
        }
    }

    /// 有没有界面在听（决定要不要问那一句）。
    pub fn connected(&self) -> bool {
        self.conn.lock().expect("锁没坏").is_some()
    }

    /// 往界面写一行。**永不返回错误**：超时 / 对端关闭都只是"这条线到此为止"。
    pub fn send(&self, msg: &FromHook) {
        let mut guard = self.conn.lock().expect("锁没坏");
        let Some(conn) = guard.as_mut() else {
            return;
        };
        let wrote = serde_json::to_string(msg).ok().and_then(|mut line| {
            line.push('\n');
            conn.set_write_timeout(Some(CONNECT_PROBE)).ok()?;
            conn.write_all(line.as_bytes()).ok()?;
            conn.flush().ok()
        });
        if wrote.is_none() {
            /* 对端卡住 / 关了：放弃这条通道（旧世代同一条：写超时即弃） */
            *guard = None;
            *self.answers.lock().expect("锁没坏") = None;
        }
    }

    /// 问一句并等答复；没有界面 / 等超时 / 用户按了停止 ⇒ `false`（**不替用户决定**）。
    pub fn ask(&self, question: &str) -> bool {
        if !self.connected() {
            return false;
        }
        self.send(&FromHook::Question(QuestionPayload {
            kind: "machine-mismatch".to_owned(),
            text: question.to_owned(),
        }));

        let deadline = Instant::now() + ANSWER_TIMEOUT;
        loop {
            /* 按停止 = 不跑：取消要能打断"等你决定"那个等待（读线程直接置位取消线） */
            if self.cancel.is_cancelled() {
                return false;
            }
            let guard = self.answers.lock().expect("锁没坏");
            let Some(rx) = guard.as_ref() else {
                return false;
            };
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(keep) => return keep,
                Err(RecvTimeoutError::Timeout) => {
                    if Instant::now() >= deadline {
                        return false;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return false,
            }
        }
    }
}

/// 钩子侧唯一的读循环：`cancel` 直接置取消线，`answer` 转给等答复的人。
fn read_loop(read_side: TcpStream, answers: Sender<bool>, cancel: CancelToken) {
    let mut lines = BufReader::new(read_side);
    let mut line = String::new();
    loop {
        line.clear();
        match lines.read_line(&mut line) {
            Ok(0) | Err(_) => return, // 界面走了 / 读不动了：这条线到此为止
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<ToHook>(trimmed) {
                    Ok(ToHook::Cancel) => {
                        cancel.cancel();
                        return;
                    }
                    Ok(ToHook::Answer { keep }) => {
                        if answers.send(keep).is_err() {
                            return; // 没人等了
                        }
                    }
                    // 认不出的行**丢掉**，不改语义（界面比我们新时，老钩子不该因此停下）
                    Err(_) => continue,
                }
            }
        }
    }
}

/// 读端点文件 → 连上去（**只探一下**，探不到就是 `None`）。
pub fn connect(internal_root: &Path) -> Option<TcpStream> {
    let port = read_port(internal_root)?;
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    TcpStream::connect_timeout(&addr.into(), CONNECT_PROBE).ok()
}

fn read_port(internal_root: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(internal_root.join(PORT_FILE)).ok()?;
    text.trim().parse::<u16>().ok()
}

/// 自己起一个界面进程（**分离**：stdin/out/err 全接 null）。
///
/// 为什么必须把所有管道接 null：钩子的 stdout/stderr 是**切片器的管道**，
/// 子进程若继承了它们，钩子退出后切片器还在等管道关闭 —— 那就成了"看起来还卡着"。
fn spawn_display() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = std::process::Command::new(exe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/* ---------- 界面那一侧 ---------- */

/// 界面那一侧的监听器（**bind 成功就是唯一那扇窗**）。
pub struct Listener {
    listener: TcpListener,
    /// 当前那条连接的写端（界面回写"取消 / 答复"用它）
    current: Arc<Mutex<Option<TcpStream>>>,
}

/// 界面能不能当"那扇窗"：已经有人在做显示时给 `None`。
///
/// 判据与旧世代的 `bind_single_instance` 同形：**先连一下**，连得上 = 已经有一扇窗，
/// 那就不抢（`None`）；连不上 = 端点文件过期或没人在听，自己 bind 一个并把端口写下。
pub fn bind_for_display(internal_root: &Path) -> Option<Listener> {
    if connect(internal_root).is_some() {
        return None;
    }
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    write_port(internal_root, port);
    Some(Listener {
        listener,
        current: Arc::new(Mutex::new(None)),
    })
}

impl Listener {
    /// 开一条接受循环：每条连接一个线程，逐行解析后交给 `on_message`。
    ///
    /// 返回的 [`Handle`] 用来回写（取消 / 答复）—— 它指向**当前那条连接**。
    pub fn serve<F>(self, on_message: F) -> Handle
    where
        F: Fn(FromHook) + Send + Sync + 'static,
    {
        let current = self.current.clone();
        let handle = Handle {
            current: current.clone(),
        };
        let on_message = Arc::new(on_message);
        std::thread::spawn(move || {
            for stream in self.listener.incoming().flatten() {
                let writer = stream.try_clone().ok();
                *current.lock().expect("锁没坏") = writer;

                let on_message = on_message.clone();
                let current = current.clone();
                std::thread::spawn(move || {
                    let mut lines = BufReader::new(stream);
                    let mut line = String::new();
                    loop {
                        line.clear();
                        match lines.read_line(&mut line) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                let trimmed = line.trim();
                                if trimmed.is_empty() {
                                    continue;
                                }
                                match serde_json::from_str::<FromHook>(trimmed) {
                                    Ok(msg) => on_message(msg),
                                    // 认不出的行丢掉：钩子比界面新时，界面不该因此崩
                                    Err(_) => continue,
                                }
                            }
                        }
                    }
                    /* 钩子退出了：这一趟到此为止，写端收掉（结果留在屏上等用户看） */
                    *current.lock().expect("锁没坏") = None;
                });
            }
        });
        handle
    }
}

/// 回写那一头（界面 → 钩子）。
pub struct Handle {
    current: Arc<Mutex<Option<TcpStream>>>,
}

impl Handle {
    /// 有没有钩子连着（没有的话按钮点了也没对象，界面据此变灰）。
    pub fn connected(&self) -> bool {
        self.current.lock().expect("锁没坏").is_some()
    }

    /// 写一行回去。写不出去就算了（钩子可能已经退出了）。
    pub fn send(&self, msg: ToHook) {
        let mut guard = self.current.lock().expect("锁没坏");
        let Some(conn) = guard.as_mut() else {
            return;
        };
        let wrote = serde_json::to_string(&msg).ok().and_then(|mut line| {
            line.push('\n');
            conn.write_all(line.as_bytes()).ok()?;
            conn.flush().ok()
        });
        if wrote.is_none() {
            *guard = None;
        }
    }
}

fn write_port(internal_root: &Path, port: u16) {
    let at = internal_root.join(PORT_FILE);
    if let Some(dir) = at.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&at, format!("{port}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_until(mut ok: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if ok() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// 一条真连接走一遍：钩子发 hello/progress/finished，界面回 cancel —— 两边都读得到。
    #[test]
    fn the_wire_carries_both_ways() {
        let d = tempfile::tempdir().expect("临时根");
        let listener = bind_for_display(d.path()).expect("第一个界面进程该能当显示");
        let got = Arc::new(Mutex::new(Vec::<FromHook>::new()));
        let seen = got.clone();
        let handle = listener.serve(move |msg| seen.lock().expect("锁没坏").push(msg));

        let cancel = CancelToken::new();
        let client = Client::connect_or_spawn(d.path(), &cancel);
        assert!(client.connected(), "端点文件刚写出来，该连得上");
        client.send(&FromHook::Hello {
            preset_name: "A1_MINI-fast.toml".to_owned(),
            gcode_name: "45600.0.gcode".to_owned(),
        });
        client.send(&FromHook::Progress(ProgressPayload {
            step: "pass1".to_owned(),
            fraction_in_step: Some(0.25),
            message: "第一遍处理中".to_owned(),
        }));
        client.send(&FromHook::Finished(Box::new(FinishedPayload {
            ok: true,
            cancelled: false,
            message: "处理完成".to_owned(),
            code: None,
            stage: Some("printtime".to_owned()),
            exit_code: 0,
            elapsed_ms: 123,
            output: None,
            warnings: Vec::new(),
        })));

        assert!(
            wait_until(|| got.lock().expect("锁没坏").len() >= 3),
            "三条都该到：{:?}",
            got.lock().expect("锁没坏")
        );
        let messages = got.lock().expect("锁没坏").clone();
        assert!(matches!(messages[0], FromHook::Hello { .. }));
        assert!(matches!(messages[1], FromHook::Progress(_)));
        assert!(matches!(messages[2], FromHook::Finished(_)));

        /* 回写：界面 → 钩子（取消）。读线程直接把它接到取消线上 */
        assert!(handle.connected(), "钩子还连着");
        handle.send(ToHook::Cancel);
        assert!(wait_until(|| cancel.is_cancelled()), "取消该传到钩子的取消线上");
    }

    /// 已经有界面在做显示时，第二个界面**不抢**（`None`）；没人听时能重新当显示（自愈）。
    #[test]
    fn only_one_display_wins() {
        let d = tempfile::tempdir().expect("临时根");
        let cancel = CancelToken::new();
        assert!(!Client::headless(&cancel).connected(), "无头线不该有连接");

        let first = bind_for_display(d.path()).expect("第一个该成");
        assert!(
            bind_for_display(d.path()).is_none(),
            "第二个界面不该抢显示（它连得上第一个）"
        );
        drop(first);
        /* 第一个走了之后：端点文件还在，但没人听 ⇒ 可以重新 bind */
        assert!(
            bind_for_display(d.path()).is_some(),
            "没人听了就该能重新当显示"
        );
    }

    /// **没有界面也照旧干活**：连不上 = 一条无头线，`send` / `ask` 都不炸，`ask` 给"不跑"。
    #[test]
    fn without_a_display_the_hook_still_works() {
        let cancel = CancelToken::new();
        /* 把 SPAWN_WAIT 花在"起不来的界面"上太慢，这里直接造一条无头线 ——
        语义与 connect_or_spawn 的超时分支同一条（`headless`） */
        let client = Client::headless(&cancel);
        client.send(&FromHook::Hello {
            preset_name: "a.toml".to_owned(),
            gcode_name: "b.gcode".to_owned(),
        });
        assert!(!client.ask("继续吗？"), "没有界面看的时候不许替你决定");
    }
}
