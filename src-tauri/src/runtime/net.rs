//! 从远端（HTTP）拿回一段字节 —— 第二圈把「下载」从本地目录源换成真源头的那一刀。
//!
//! # 这一层的职责边界
//!
//! ```text
//! source.rs   远端在哪（baseUrl）        ── 设置，不认识 HTTP
//! net.rs      怎么从远端拿到这段字节     ── HTTP/重试/超时/水位，不认识业务逻辑
//! delivery.rs 拿到字节之后怎么落盘       ── SHA 校验 / 归档 / 落点，不认识 HTTP
//! ```
//!
//! 三条线互不相交：[`super::delivery::Source`] 是唯一的接口。第一圈的实现是「照着
//! catalog 的文件名去某个本地目录里读」，这里是同一个 trait 的 HTTP 实现。
//! **管道一行不改。**
//!
//! # 哪些错可以再来一次
//!
//! 只重试**运输过程**的抖动（连接建立不了、超时、主机解析不了、连接被对端掐了）。
//! **内容类的错一次都不重试** —— 404、500、字节超了目录记的大小，重试只会把同一个
//! 错误重复三遍，还让用户等了三倍时间。省下的时间是给"抖动"这种真会自愈的故障的。
//!
//! # 关于高性能的取舍
//!
//! 整份读进内存再交给管道：这批文件是几十 KB 的 TOML/JSON，而且 **校验必须在落盘之前**
//! （总纲产品质量线），所以没有"边写边算"的中间态可选。真到有几百 MB 的模型文件那天，
//! 改这里（流式喂给 SHA + 临时文件），管道依然不动。
//!
//! # 进度的口径
//!
//! 这一层只报两件事：连上了没有、收到了多少字节。"下载还活着"于是成为界面上一句
//! 能陈述的事实，而不是一个转圈的装饰。

use std::io::Read as _;
use std::time::Duration;

use crate::error::AppError;

/// **本地 IPC 的 loopback 通道**（钩子进程 ↔ 常驻界面进程）。
///
/// # 它为什么住在这里
///
/// `scripts/check-zero-network.mjs` 的 ①闸要求"网络字节只许住 `runtime/net.rs` 与
/// `workbench/app/platform/`"。钩子 IPC 走的是 **loopback TCP**（两个进程都在本机），
/// 按那条判据的字面它属于"网络字节" —— 于是它只能住这里。搬进来不是为了让扫描器闭嘴
/// （给文件加白名单才是），而是让"谁在谈网络"这个问题继续**只有一个答案**：
/// 这个文件与 `platform/` 就是那两处。
///
/// # 它不是对外网络
///
/// - 只 bind `127.0.0.1`（[`Ipv4Addr::LOCALHOST`]），**从不绑 `0.0.0.0`**；
/// - 端口是内核给的随机端口（`bind :0`），经内部根下的端点文件在两者之间交换；
/// - 连接带超时：钩子"只探一下"，探不到就自己起一个界面进程。
///
/// 这三条是它敢住在"网络面"里、而又不违反"客户端产品不许开端口"那条精神的原因：
/// **它没有对外开任何门**，只是同一台机器上两个进程之间的一条私线。
pub mod loopback {
    use std::io;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
    use std::time::Duration;

    /// 某个**本机端口**上有没有人在听（工作台的 dev 文件源起服务前用它探路）。
    ///
    /// 这是 [`LoopbackListener::bind_any_local`] 的**探测形态**：bind 一下、立刻释放。
    /// 两个回环地址都试 —— 对面可能只绑了 v4，也可能只绑 v6（vite 绑 `localhost`
    /// 在有些机器上只落在 `::1`）。
    ///
    /// 它**不长期占任何端口**（探测即释放），所以不违反"客户端产品不许开端口"。
    pub fn is_port_taken(port: u16) -> bool {
        let v4 = SocketAddr::from(([127, 0, 0, 1], port));
        let v6 = SocketAddr::from(([0u16, 0, 0, 0, 0, 0, 0, 1], port));
        [v4, v6].iter().any(|a| TcpListener::bind(a).is_err())
    }

    /// 监听端（界面那一侧；`bind` 成功就是"唯一那扇窗"，单实例不用锁文件）。
    pub struct LoopbackListener(TcpListener);

    /// 连接端（两侧各持一个；[`LoopbackStream::try_clone`] 给读写分家）。
    pub struct LoopbackStream(TcpStream);

    impl LoopbackListener {
        /// 在本机 loopback 上绑一个**随机端口**。
        pub fn bind_any_local() -> io::Result<Self> {
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).map(Self)
        }

        /// 内核给的那个端口（写进端点文件，钩子照它连）。
        pub fn port(&self) -> io::Result<u16> {
            self.0.local_addr().map(|a| a.port())
        }

        /// 收下一条连接。
        pub fn accept(&self) -> io::Result<LoopbackStream> {
            self.0.accept().map(|(stream, _)| LoopbackStream(stream))
        }

        /// 收连接的迭代器（界面侧：`for stream in listener.incoming().flatten()`）。
        pub fn incoming(&self) -> LoopbackIncoming<'_> {
            LoopbackIncoming(self.0.incoming())
        }
    }

    /// [`LoopbackListener::incoming`] 的迭代器 —— 包一层是为了**不让 `std::net` 漏出去**
    /// （漏出去的那一刻，判据 ① 的"只住一处"就不成立了）。
    pub struct LoopbackIncoming<'a>(std::net::Incoming<'a>);

    impl Iterator for LoopbackIncoming<'_> {
        type Item = io::Result<LoopbackStream>;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(|r| r.map(LoopbackStream))
        }
    }

    impl LoopbackStream {
        /// 连本机的某个端口。**只探一下**：连不上就是 `Err`，调用方照"没有界面"降级。
        pub fn connect_local(port: u16, timeout: Duration) -> io::Result<Self> {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            TcpStream::connect_timeout(&addr.into(), timeout).map(Self)
        }

        /// 读写分家（读线程持 clone，写端留在原对象上）。
        pub fn try_clone(&self) -> io::Result<Self> {
            self.0.try_clone().map(Self)
        }

        /// 设读超时（界面侧的收连接循环用它做"该退出了"的检查节拍）。
        pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            self.0.set_read_timeout(dur)
        }

        /// 设写超时（钩子侧：对端卡住就放弃这条通道，不让切片器跟着等）。
        pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            self.0.set_write_timeout(dur)
        }

        /// 掐掉这条连接（会话结束 / 取消时用）。
        pub fn shutdown_both(&self) -> io::Result<()> {
            self.0.shutdown(std::net::Shutdown::Both)
        }
    }

    impl io::Read for LoopbackStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl io::Write for LoopbackStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}

use super::catalog::CatalogFile;
use super::delivery::Source;
use super::resolver::{ResourceRef, SourceResolver};
use super::source::{CATALOG_FILE, RELEASE_FILE};

/// 一次请求的总时间上限（含连接与传完整个响应体）
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(120);

/// 连接建立的上限。比总时短——连不上是最常见的一类故障，不该让人等满两分钟
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 一次 `get` 的预设趟数（`1 + RETRIES`，失败才再走）
pub const RETRIES: u32 = 2;

/// 退避基数：第 n 次重试前等 `BACKOFF_BASE * 2^(n-1)`
pub const BACKOFF_BASE: Duration = Duration::from_millis(300);

/// 每次从响应体读多少。它同时是水位上报的粒度：太细就一直在刷事件，
/// 太粗界面看不出在动
const CHUNK: usize = 64 * 1024;

/// 一次取字节的全部入参（positional 参数太长了，合成一个）
#[derive(Debug, Clone)]
pub struct GetPlan<'a> {
    /// 给进度事件用：告诉调用方"现在动的是哪一份"
    pub file_name: &'a str,
    /// 目录登记的字节数。**顺手当一道上限用**：超过就提前收摊，
    /// 不让一个撒谎的服务端把内存吃光
    pub expect_size: Option<u64>,
    /// 总共试几趟（含第一次）
    pub attempts: u32,
    /// 退避基数。测试里调成 0 —— 抖动是否真重试，一秒内见分晓
    pub backoff: Duration,
}

impl<'a> GetPlan<'a> {
    pub fn new(file_name: &'a str) -> Self {
        Self {
            file_name,
            expect_size: None,
            attempts: 1 + RETRIES,
            backoff: BACKOFF_BASE,
        }
    }
}

/// 传到哪一步了。**只有四个阶段** —— 刻意没有"校验中 / 落盘中"：
/// 那两步发生在管道内部（SHA 校验与原子写是一次 `deliver` 调用的内部行为），
/// 这一层拿不到它们的时机。与其报一个猜出来的时刻，不如只报真知道的事。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Connecting,
    Transferring,
    Done,
    Failed,
}

impl Stage {
    /// 发给界面的那个词。契约两侧认这一份字符串，
    /// 不靠 Rust 枚举名与 TS 字面量"正好长一样"
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Connecting => "connecting",
            Stage::Transferring => "transferring",
            Stage::Done => "done",
            Stage::Failed => "failed",
        }
    }
}

/// 一次转移的水位。`total` 为 `None` = 服务端没给长度 —— 前端就别说百分比，
/// 说"已收多少字节"—— 不为了凑百分比去编一个分母。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick {
    pub stage: Stage,
    pub file_name: String,
    pub received: u64,
    pub total: Option<u64>,
}

/// 进度回调。没人关心进度时传 [`noop_tick`]——调用方因此不必写 `Option<回调>`。
///
/// **带生命周期参数是为了不被推成 `'static`**：回调常常是借用局部变量写出来的闭包
/// （"把进度转给这条 IPC"），那种借用活不长
pub type OnTick<'a> = dyn Fn(&Tick) + Send + Sync + 'a;

pub fn noop_tick(_: &Tick) {}

/// 从远端取回一份文件（失败按计划重试，成功就回来）。
///
/// `plan.attempts` 趟之内，只要失败是运输类的就按 `2^n` 退避再来；内容类的错一次也不重来。
/// 每一趟开始时发一次 [`Stage::Connecting`]，之后每 CHUNK 发一次 [`Stage::Transferring`]。
pub fn get_bytes(url: &str, plan: &GetPlan<'_>, on_tick: &OnTick<'_>) -> Result<Vec<u8>, AppError> {
    let attempts = plan.attempts.max(1);
    let mut last: Option<AppError> = None;

    for attempt in 0..attempts {
        if attempt > 0 {
            let waited = plan.backoff.saturating_mul(1 << (attempt - 1));
            if !waited.is_zero() {
                std::thread::sleep(waited);
            }
        }
        tick(Stage::Connecting, plan.file_name, 0, None, on_tick);
        match try_once(url, plan, on_tick) {
            Ok(bytes) => return Ok(bytes),
            Err((err, retryable)) if retryable => {
                tracing::warn!(attempt = attempt + 1, "取字节失败，退出重试：{}", err);
                last = Some(err);
            }
            Err((err, _)) => return Err(err),
        }
    }

    Err(last
        .expect("趟数 >= 1，走不到这里却没有错误")
        .with_detail(format!(
            "试了 {attempts} 趟都没能从远端拿到 {}",
            plan.file_name
        )))
}

/// 单趟 GET。返回 `(错误, 这个错是否值得重试)` —— 分类型这件事在这一层做，
/// 免得每个调用方自己 if 一遍"500 要不要再试"
fn try_once(
    url: &str,
    plan: &GetPlan<'_>,
    on_tick: &OnTick<'_>,
) -> Result<Vec<u8>, (AppError, bool)> {
    fetch_once(&build_agent(), url, plan, on_tick)
}

/// 用指定的 agent 跑一趟 GET。返回 `(错误, 这个错是否值得重试)` ——
/// **分类型在这一层做**，免得每个调用方自己再 if 一遍"500 要不要再试"。
///
/// agent 单独入参是为了让**超时能被测到**：否则要验"服务端不回答也不吊死调用方"，
/// 就得真等满那个上限。
fn fetch_once(
    agent: &ureq::Agent,
    url: &str,
    plan: &GetPlan<'_>,
    on_tick: &OnTick<'_>,
) -> Result<Vec<u8>, (AppError, bool)> {
    let response = agent.get(url).call().map_err(|e| {
        let err = transport_error(url, &e);
        (err, retryable(&e))
    })?;

    let (parts, body) = response.into_parts();
    let total = content_length(&parts.headers);

    let mut reader = body.into_reader();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.map_or(CHUNK, |n| n as usize));
    let mut buf = [0u8; CHUNK];

    loop {
        let read = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let retry = matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                );
                if retry {
                    return Err((
                        AppError::io(format!("读 {} 的响应中断了", plan.file_name))
                            .with_detail(e.to_string()),
                        true,
                    ));
                }
                return Err((
                    AppError::io(format!("读 {} 的响应失败", plan.file_name))
                        .with_detail(e.to_string()),
                    false,
                ));
            }
        };
        bytes.extend_from_slice(&buf[..read]);

        // 目录登记的大小是一道硬闸：超了就提前收摊，不再往下读
        if let Some(expect) = plan.expect_size {
            if bytes.len() as u64 > expect {
                return Err((
                    AppError::corrupted(format!(
                        "{} 的响应比目录登记的大（目录记 {} 字节，远端已经给了 {} 字节以上）",
                        plan.file_name,
                        expect,
                        bytes.len()
                    )),
                    false,
                ));
            }
        }
        tick(
            Stage::Transferring,
            plan.file_name,
            bytes.len() as u64,
            total,
            on_tick,
        );
    }

    Ok(bytes)
}

/// 连接与传输的上限在这里定 —— **没有超时，一个不回答的服务端就能把界面吊到天荒地老**
fn build_agent() -> ureq::Agent {
    agent_with(FETCH_TIMEOUT, CONNECT_TIMEOUT)
}

fn agent_with(global: Duration, connect: Duration) -> ureq::Agent {
    let mut builder = ureq::Agent::config_builder()
        .timeout_global(Some(global))
        .timeout_connect(Some(connect));
    if let Some(url) = system_proxy_url() {
        match ureq::Proxy::new(url.as_str()) {
            Ok(proxy) => builder = builder.proxy(Some(proxy)),
            Err(e) => {
                // 认不出的代理地址**不该让整次下载失败** —— 直连还可能通
                tracing::warn!("系统代理地址认不出（按直连走）：{url}（{e}）")
            }
        }
    }
    builder.build().into()
}

/* ------------------------------- 系统代理 ------------------------------- */

/// **系统里配的那个代理**（`http://host:port` / `socks5://host:port`）；没有配就是 `None`。
///
/// # 为什么必须读它
///
/// 程序自己直连时，本机配了代理的用户会拿到一个**答不上来的远端**：
/// 真机表现是 `invalid peer certificate: UnknownIssuer`（2026-10-05 的 0.0.1 客户端），
/// 而同一个地址在浏览器 / `curl` 里是通的 —— 差别就在那两个走了系统代理、我们没有。
/// 用户不会为我们的程序单独再配一次网络。
///
/// # 顺序
///
/// ```text
/// 环境变量（HTTPS_PROXY / ALL_PROXY / HTTP_PROXY）→ macOS 的 scutil --proxy
/// ```
///
/// 环境变量优先：它是**用户显式**说出口的，比系统设置更具体。
/// 其它平台（Linux / Windows）这一步只走环境变量 —— 第一阶段只发 macOS 安装包，
/// 不为"以后可能要"提前把注册表也读进来。
pub fn system_proxy_url() -> Option<String> {
    if let Some(url) = proxy_from_env(&|k| std::env::var(k).ok()) {
        return Some(url);
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(text) = scutil_proxy_text() {
            return parse_scutil_proxy(&text);
        }
    }
    None
}

/// 环境变量里的代理。`lookup` 注入是为了**能测**（测试进程改全局环境变量会互相干扰）。
///
/// https 的那几个先看：我们要取的都是 `https://` 地址；`ALL_PROXY` 兜最后。
pub fn proxy_from_env(lookup: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        if let Some(v) = lookup(key) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_owned());
            }
        }
    }
    None
}

/// macOS：`scutil --proxy` 的原始输出。拿不到就 `None`（不报错 —— 没有代理是合法的）。
#[cfg(target_os = "macos")]
fn scutil_proxy_text() -> Option<String> {
    let out = std::process::Command::new("scutil")
        .args(["--proxy"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// 解析 `scutil --proxy` 的输出（`<dictionary> { HTTPSProxy : 127.0.0.1 ... }`）。
///
/// ★ **纯函数**：这一整段是"读系统设置"里唯一值得测的部分 —— 子进程那一步测不出什么，
/// 而"哪些键算开了代理"极易写错（把 `HTTPSEnable : 0` 当成 1 就是所有人都连不通）。
///
/// 三种都认：**HTTPS → SOCKS → HTTP**（HTTPS 优先，因为我们要取的都是 https 地址）。
pub fn parse_scutil_proxy(text: &str) -> Option<String> {
    let map = scutil_keys(text);
    for (enable, host, port, scheme) in [
        ("HTTPSEnable", "HTTPSProxy", "HTTPSPort", "http"),
        ("SOCKSEnable", "SOCKSProxy", "SOCKSPort", "socks5"),
        ("HTTPEnable", "HTTPProxy", "HTTPPort", "http"),
    ] {
        if map.get(enable).map(|v| v == "1").unwrap_or(false) {
            if let (Some(h), Some(p)) = (map.get(host), map.get(port)) {
                if !h.is_empty() && !p.is_empty() {
                    return Some(format!("{scheme}://{h}:{p}"));
                }
            }
        }
    }
    None
}

/// `scutil` 输出里的 `键 : 值` 收成一张表（只收这两种形状，别的行一律略过）。
fn scutil_keys(text: &str) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || value.is_empty() {
            continue;
        }
        map.insert(key.to_owned(), value.to_owned());
    }
    map
}

fn content_length(headers: &ureq::http::HeaderMap) -> Option<u64> {
    headers
        .get(ureq::http::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

/// 运输类错误才重试。**404/500 是答案，不是故障** —— 它们下一次回答也一样
fn retryable(e: &ureq::Error) -> bool {
    matches!(
        e,
        ureq::Error::Io(_)
            | ureq::Error::Timeout(_)
            | ureq::Error::ConnectionFailed
            | ureq::Error::HostNotFound
    )
}

/// 把 ureq 的错说成人听得懂的话，并把错的性质留在 [`AppError.code`] 里
/// （404 = NotFound，其它一律按 IO 报——错误码不为此扩一圈：界面要能据以行动的分类
/// 就是这几个，"HTTP 502"这种细节进 detail，不进 code）
fn transport_error(url: &str, e: &ureq::Error) -> AppError {
    match e {
        ureq::Error::StatusCode(404) => AppError::not_found(format!("远端没有这份文件：{url}")),
        ureq::Error::StatusCode(code) => AppError::io(format!(
            "远端回了 HTTP {code}（既不是成功也不是没找到）：{url}"
        )),
        ureq::Error::Timeout(_) => AppError::io(format!(
            "远端没在规定时间内回答（{} 秒）：{url}",
            FETCH_TIMEOUT.as_secs()
        )),
        ureq::Error::HostNotFound => AppError::io(format!("数据源的域名解析不了：{url}")),
        other => AppError::io(format!("从远端取字节失败：{url}")).with_detail(other.to_string()),
    }
}

fn tick(stage: Stage, file_name: &str, received: u64, total: Option<u64>, on_tick: &OnTick<'_>) {
    on_tick(&Tick {
        stage,
        file_name: file_name.to_owned(),
        received,
        total,
    });
}

/// 远端的目录（catalog）。**收完整 URL** —— 地址怎么来的（手动根拼的 / Bootstrap
/// 说的）不归它管：那是 [`super::source::resolve_source`] 的事，它只负责取。
pub fn get_catalog(catalog_url: &str) -> Result<Vec<u8>, AppError> {
    get_bytes(catalog_url, &GetPlan::new(CATALOG_FILE), &noop_tick)
}

/// 远端的**软件发布信息**（`release.json`）。与 [`get_catalog`] 同形、同一个取名口 ——
/// 这一层只负责"取字节"，`release.json` 与 catalog 的**语义差别**（一个是软件版本、
/// 一个是预设数据）不在这一层表达，那是 [`super::release_info`] 的事。
/// **流式**取一个 URL —— 应用内更新下载安装包用（[`get_bytes`] 那种"整个取回内存"
/// 的形状在几十 MB 的包上等于拿内存当磁盘）。
///
/// 超时给得比平时宽（[`UPLOAD_TIMEOUT`] 那一档的量级）：下载要传几十 MB，
/// 30 秒的传输上限会**必然**掐断。连接超时仍是 10s（连不上就该立刻说连不上）。
pub fn stream_get(url: &str) -> Result<ureq::http::Response<ureq::Body>, AppError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60 * 60)))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build();
    let agent: ureq::Agent = config.into();
    agent
        .get(url)
        .header("User-Agent", "SupportEase")
        .call()
        .map_err(|e| transport_error(url, &e))
}

pub fn get_release(release_url: &str) -> Result<Vec<u8>, AppError> {
    get_bytes(release_url, &GetPlan::new(RELEASE_FILE), &noop_tick)
}

/* ------------------------------- 远端的 Source 实现 ------------------------------- */

/// 网络源：**地址由 [`SourceResolver`] 说**（业务层不拼 URL —— 寻址唯一出口）。
///
/// 它就是第一圈那句"真云端来了加一个实现，管道不动"的兑现——管道仍然只见到
/// 一个 [`Source`]`::fetch`，不认识 HTTP，也不需要认识。
pub struct RemoteSource<'a> {
    resolver: SourceResolver,
    on_tick: &'a OnTick<'a>,
}

impl<'a> RemoteSource<'a> {
    /// `file_name` 之外的进度都吐给 `on_tick`；不关心进度就传 [`noop_tick`]
    pub fn new(resolver: SourceResolver, on_tick: &'a OnTick<'a>) -> Self {
        Self { resolver, on_tick }
    }

    pub fn no_progress(resolver: SourceResolver) -> Self {
        Self {
            resolver,
            on_tick: &noop_tick,
        }
    }
}

impl Source for RemoteSource<'_> {
    fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError> {
        let url = self
            .resolver
            .resolve(ResourceRef::Entry(file))?
            .remote_or("这份交付文件")?;
        let plan = GetPlan {
            file_name: &file.file_name,
            // 期望大小是 `Option`（随包 bootstrap 目录不登记它）：没有就不设水位
            expect_size: file.expected_size(),
            ..GetPlan::new(&file.file_name)
        };
        get_bytes(&url, &plan, self.on_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::super::delivery::FileOnDisk;
    use super::*;
    use sha2::Digest as _;
    use std::io::Write as _;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// 服务端按第几次请求给哪种回答。最后一个回答会被之后的请求一直复用
    #[derive(Debug, Clone)]
    enum Reply {
        Bytes(Vec<u8>),
        Status(u16, Vec<u8>),
        /// 掐掉连接（不发 HTTP 响应）：测试里唯一能造出"运输类故障"的办法
        Drop,
        /// 收了请求就是不回答：用来证明"超时真的会把调用方放出来"
        Hang,
    }

    /// 测试专用的最小 HTTP 服务端。**只认 GET、只发响应、不长连接**。
    ///
    /// 用 `std::net` 手写而不是引一个服务端框架：它虚构的服务端不属于产品架构，
    /// 为一个测试断言拉一条依赖，不值；而且手写的这个逐字节可读 —— 将来要造
    /// "响应体发一半就断"这类场景，看这里的人知道该改哪一行。
    struct TestServer {
        addr: std::net::SocketAddr,
        hits: Arc<AtomicUsize>,
        stop: Arc<Mutex<bool>>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn start(script: Vec<Reply>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("端口绑不上");
            let addr = listener.local_addr().expect("拿不到地址");
            let hits = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(Mutex::new(false));

            let (threads_hits, threads_stop) = (hits.clone(), stop.clone());
            let worker = std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if *threads_stop.lock().expect("stop 锁坏了") {
                        break;
                    }
                    let Ok(mut stream) = stream else { continue };
                    // 请求头读到空行就算读完（这只够最朴素的 GET，够用）
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while head.len() < 64 * 1024 {
                        match stream.read(&mut byte) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                head.push(byte[0]);
                                if head.ends_with(b"\r\n\r\n") {
                                    break;
                                }
                            }
                        }
                    }
                    let n = threads_hits.fetch_add(1, Ordering::SeqCst);
                    let reply = script.get(n).or_else(|| script.last()).cloned();
                    match reply {
                        Some(Reply::Bytes(body)) => {
                            let _ = write_response(&mut stream, 200, "OK", &body);
                        }
                        Some(Reply::Status(code, body)) => {
                            let reason = if code == 404 { "Not Found" } else { "Error" };
                            let _ = write_response(&mut stream, code, reason, &body);
                        }
                        Some(Reply::Hang) => {
                            std::thread::sleep(Duration::from_secs(5));
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                        Some(Reply::Drop) | None => {
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                    }
                }
            });

            Self {
                addr,
                hits,
                stop,
                worker: Some(worker),
            }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}/{}", self.addr, path.trim_start_matches('/'))
        }

        fn hits(&self) -> usize {
            self.hits.load(Ordering::SeqCst)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            *self.stop.lock().expect("stop 锁坏了") = true;
            // 让它从 accept 里出来：连一下就断，不要把它的整个套路走完
            let _ = TcpStream::connect(self.addr);
            if let Some(h) = self.worker.take() {
                let _ = h.join();
            }
        }
    }

    fn write_response(
        stream: &mut TcpStream,
        code: u16,
        reason: &str,
        body: &[u8],
    ) -> std::io::Result<()> {
        stream.write_all(
            format!(
                "HTTP/1.1 {code} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .as_bytes(),
        )?;
        stream.write_all(body)?;
        stream.flush()
    }

    fn quick_plan(file_name: &str) -> GetPlan<'_> {
        GetPlan {
            backoff: Duration::from_millis(1),
            ..GetPlan::new(file_name)
        }
    }

    /// 这条是"零网络依赖真的破了"的证据：**真的发了一个 HTTP 请求、真的拿回了字节**
    #[test]
    fn gets_bytes_over_real_http() {
        let content = "# A1 standard\nspeed_limit = 60\n".as_bytes().to_vec();
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);

        let got = get_bytes(
            &server.url("mkp/A1-standard.toml"),
            &quick_plan("A1-standard.toml"),
            &noop_tick,
        )
        .expect("真 HTTP 该拿到字节");

        assert_eq!(got, content, "字节逐份一致");
        assert_eq!(server.hits(), 1, "一次尝试就成");
    }

    /// 水位要能讲出一个过程：阶段是「先连、再传」，已收字节单调递增且最后是全长
    #[test]
    fn reports_a_monotonic_progress() {
        let content = vec![b'x'; CHUNK * 2 + 17];
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);
        let seen: Arc<Mutex<Vec<Tick>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let seen = seen.clone();
            move |t: &Tick| seen.lock().expect("锁坏了").push(t.clone())
        };

        get_bytes(&server.url("big.toml"), &quick_plan("big.toml"), &sink).expect("下载该成功");

        let ticks = seen.lock().expect("锁坏了").clone();
        assert!(!ticks.is_empty(), "至少报一次");
        assert_eq!(ticks.first().expect("有").stage, Stage::Connecting);
        assert!(
            ticks.iter().skip(1).all(|t| t.stage == Stage::Transferring),
            "第一报之后全都是在传"
        );
        let mut prev = 0;
        for t in &ticks {
            assert!(t.received >= prev, "已收字节不许回退");
            prev = t.received;
        }
        assert_eq!(ticks.last().expect("有").received, content.len() as u64);
        assert_eq!(ticks.last().expect("有").total, Some(content.len() as u64));
        assert_eq!(
            ticks.last().expect("有").file_name,
            "big.toml",
            "水位要说清动的是哪一份"
        );
    }

    /// 抖动**真的重试成功了** —— 这条不能只数次数：要证明最后一次拿到的是真字节，
    /// 且服务端被掐的那次确实发生在成功之前
    #[test]
    fn retries_a_dropped_connection_and_succeeds() {
        let content = "# retry me\n".as_bytes().to_vec();
        let server = TestServer::start(vec![Reply::Drop, Reply::Bytes(content.clone())]);

        let got = get_bytes(
            &server.url("A1-standard.toml"),
            &quick_plan("A1-standard.toml"),
            &noop_tick,
        )
        .expect("抖一次之后该成功");

        assert_eq!(got, content, "重试拿到的是真字节，不是空壳");
        assert_eq!(server.hits(), 2, "掐一次、成一次");
    }

    /// 重试有上限：一直掐下去，最后要如实报错，而不是无限试下去把界面挂住
    #[test]
    fn gives_up_after_the_planned_attempts() {
        let server = TestServer::start(vec![Reply::Drop]);
        let plan = GetPlan {
            attempts: 3,
            ..quick_plan("A1-standard.toml")
        };

        let e = get_bytes(&server.url("A1-standard.toml"), &plan, &noop_tick).unwrap_err();

        assert_eq!(e.code, crate::error::ErrorCode::Io);
        assert!(
            e.detail
                .as_deref()
                .unwrap_or_default()
                .contains("试了 3 趟"),
            "原因里要说清试了几趟：{e:?}"
        );
        assert_eq!(server.hits(), 3, "一趟不多一趟不少");
    }

    /// **内容类的错一次也不重试**：404 是答案，再问三遍还是同一个答案，
    /// 但用户要等三倍的时间
    #[test]
    fn not_found_is_not_retried() {
        let server = TestServer::start(vec![Reply::Status(404, b"no such file".to_vec())]);

        let e = get_bytes(
            &server.url("ghost.toml"),
            &quick_plan("ghost.toml"),
            &noop_tick,
        )
        .unwrap_err();

        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
        assert_eq!(server.hits(), 1, "404 不需要再来一次");
    }

    /// 服务端撒谎（给得比目录登记的多）要在落地之前被拦住、且不重试
    #[test]
    fn oversized_response_is_stopped_early() {
        let content = vec![b'z'; CHUNK + 111];
        let server = TestServer::start(vec![Reply::Bytes(content)]);
        let plan = GetPlan {
            expect_size: Some(64),
            ..quick_plan("bloated.toml")
        };

        let e = get_bytes(&server.url("bloated.toml"), &plan, &noop_tick).unwrap_err();

        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert_eq!(server.hits(), 1, "内容错了不重试");
    }

    /// Source 实现： catalog 记的 `path` 就是相对地址，`baseUrl` 只拼在前面
    #[test]
    fn remote_source_fetches_the_catalog_path() {
        let content = "# from the remote source\n".as_bytes().to_vec();
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);
        let file = entry("A1-standard.toml", &content);

        let source = RemoteSource::no_progress(test_resolver(server.addr));
        let got = source.fetch(&file).expect("Source 该拿到字节");

        assert_eq!(got, content);
        assert_eq!(
            Some(got.len() as u64),
            file.expected_size(),
            "size 顺手当上限使"
        );
    }

    /* ---------- 端到端：把「下载」这件事从头走到尾 ---------- */

    /// 目录条目：SHA 与大小都对 `content` 负责（远端与期望值由此同源）
    fn entry(name: &str, content: &[u8]) -> CatalogFile {
        CatalogFile {
            kind: super::super::catalog::kind::PRESET.to_owned(),
            file_name: name.to_owned(),
            path: format!("{}/{name}", super::super::catalog::PRESET_DEST_DIR),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: Some(super::super::catalog::hex(&sha2::Sha256::digest(content))),
            size: Some(content.len() as u64),
        }
    }

    fn fresh_root() -> tempfile::TempDir {
        // 交付面不再有"预建的下载区根"：目录由 `deliver` 按 `catalog.path` 落盘时按需建
        tempfile::tempdir().expect("临时目录建不出来")
    }

    /// 判据用的 Resolver：`filesRoot = "."`（测试服务器的根就是文件根）。
    /// 地址全部由 resolver 说 —— 与产品同一条寻址路径。
    fn test_resolver(addr: std::net::SocketAddr) -> crate::runtime::resolver::SourceResolver {
        crate::runtime::resolver::SourceResolver::from_bootstrap(
            format!("http://{addr}"),
            br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":"."}"#,
        )
        .expect("测试 Manifest 该解析得动")
    }

    /// 这一轮唯一能证明「客户端真的能从远端把文件拿回来落到下载区」的那条。
    ///
    /// 路径与之前 Section 单测的最大差别是它经过 [`super::super::delivery::deliver`]：
    /// 校验、防穿越、落点全由管道负责，网络只是送字节的那一段。
    #[test]
    fn delivers_a_remote_file_all_the_way_into_mkp() {
        let content = "# A1 standard\nspeed_limit = 60\n".as_bytes().to_vec();
        let file = entry("A1-standard.toml", &content);
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);
        let root = fresh_root();

        let source = RemoteSource::no_progress(test_resolver(server.addr));
        let target =
            super::super::delivery::deliver(root.path(), &file, &source).expect("该走得通");

        assert_eq!(
            target,
            super::super::paths::released_file(root.path(), &file.path),
            "落在 catalog.path 说的那个位置（唯一路径语义）"
        );
        assert_eq!(std::fs::read(&target).expect("读到"), content);
        assert_eq!(
            super::super::delivery::file_status(root.path(), &file),
            FileOnDisk::Current,
            "盘上当底账：下完立刻查得到"
        );
    }

    /// 第三圈第一刀的端到端：**BBS 配置走同一条管道**（真 HTTP → SHA → 落在
    /// `catalog.path` 说的那一格，A 类是 `assets/bbs/…`）。
    ///
    /// 这条守的是"新增一种资源不再新增一套下载系统"：除了 `kind` 与落点目录，
    /// 它与预设那一支走的是**同一个 `deliver`、同一道 SHA 闸、同一套归档**。
    #[test]
    fn bbs_config_rides_the_same_pipeline() {
        let content = "{\"type\":\"process\",\"version\":\"0.2\"}\n"
            .as_bytes()
            .to_vec();
        let file = CatalogFile {
            kind: super::super::catalog::kind::BBS_CONFIG.to_owned(),
            file_name: "MKPProcess A1 0.2 0.10.json".to_owned(),
            path: "assets/bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json".to_owned(),
            machine_id: "A1".to_owned(),
            version_id: String::new(),
            sha256: Some(super::super::catalog::hex(&sha2::Sha256::digest(&content))),
            size: Some(content.len() as u64),
        };
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);
        let root = fresh_root();

        let source = RemoteSource::no_progress(test_resolver(server.addr));
        let target =
            super::super::delivery::deliver(root.path(), &file, &source).expect("该走得通");

        assert_eq!(
            target,
            root.path()
                .join("assets/bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json"),
            "BBS 配置落在 assets/bbs/ 下（`catalog.path` 说的那一格），与预设分开"
        );
        assert_eq!(std::fs::read(&target).expect("读到"), content);
        assert_eq!(
            super::super::delivery::file_status(root.path(), &file),
            FileOnDisk::Current
        );
    }

    /// 模型（3mf）走的是**同一条管道**：除了 kind 与落点目录，与 BBS 那一支没有差别。
    /// 这条钉的是"接一种新资源只是加一个常量"—— 如果哪天为了模型另开一条路径，这条会先红。
    #[test]
    fn a_model_rides_the_same_pipeline_too() {
        let content = b"3MF\x00binary-ish".to_vec();
        let file = CatalogFile {
            kind: super::super::catalog::kind::MODEL.to_owned(),
            file_name: "MKP_support_test_models.3mf".to_owned(),
            path: "assets/models/MKP_support_test_models.3mf".to_owned(),
            machine_id: String::new(),
            version_id: String::new(),
            sha256: Some(super::super::catalog::hex(&sha2::Sha256::digest(&content))),
            size: Some(content.len() as u64),
        };
        let server = TestServer::start(vec![Reply::Bytes(content.clone())]);
        let root = fresh_root();

        let source = RemoteSource::no_progress(test_resolver(server.addr));
        let target =
            super::super::delivery::deliver(root.path(), &file, &source).expect("该走得通");

        assert_eq!(
            target,
            root.path()
                .join("assets/models/MKP_support_test_models.3mf")
        );
        assert_eq!(std::fs::read(&target).expect("读到"), content);
    }

    /// 远端撒谎且**长度一模一样**：大小那道闸放行，SHA 那道必须拦住。
    /// （真机最常见的一类: 服务端上错文件 / 代理塞了错误页 —— 字节数常能对上。）
    #[test]
    fn lying_bytes_never_reach_the_download_area() {
        let good = "# A1 standard\nspeed_limit = 60\n".as_bytes().to_vec();
        let lying = vec![b'x'; good.len()];
        let file = entry("A1-standard.toml", &good);
        let server = TestServer::start(vec![Reply::Bytes(lying)]);
        let root = fresh_root();

        let source = RemoteSource::no_progress(test_resolver(server.addr));
        let e = super::super::delivery::deliver(root.path(), &file, &source).unwrap_err();

        assert_eq!(
            e.code,
            crate::error::ErrorCode::ShaMismatch,
            "对不上字节 = 拒绝"
        );
        assert!(
            !super::super::paths::released_file(root.path(), &file.path).exists(),
            "坏字节连碰盘的机会都没有"
        );
    }

    /// 目录收取：地址由调用方定好（`resolve_source` 拼的 / Bootstrap 说的），这里只取
    #[test]
    fn catalog_comes_from_the_given_url() {
        let manifest = r#"{"catalogSchema":1}"#.as_bytes().to_vec();
        let server = TestServer::start(vec![Reply::Bytes(manifest.clone())]);

        let got = get_catalog(&format!("http://{}/catalog.json", server.addr)).expect("该拿到目录");

        assert_eq!(got, manifest);
        assert_eq!(server.hits(), 1);
    }

    /// 服务端收了请求却不回答 —— 调用方必须在超时处收摊。
    ///
    /// 这条不是为了数不胜数的字段配置，是为了**不允许无限等待**：没有超时的话，
    /// 这条测试自己就会一直挂在 accept 后面等五个回复周期。
    #[test]
    fn a_silent_server_is_abandoned_at_the_timeout() {
        let server = TestServer::start(vec![Reply::Hang]);
        let started = Instant::now();

        let (err, worth_retrying) = fetch_once(
            &agent_with(Duration::from_millis(300), Duration::from_millis(300)),
            &server.url("silent.toml"),
            &quick_plan("silent.toml"),
            &noop_tick,
        )
        .unwrap_err();

        assert_eq!(
            err.code,
            crate::error::ErrorCode::Io,
            "卡住要报 IO：{err:?}"
        );
        assert!(worth_retrying, "超时是运输类故障，值得再试一次");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "没实现超时的话这里会一直等下去：{:?}",
            started.elapsed()
        );
    }

    /* ---------- 系统代理（2026-10-05：真机那条 UnknownIssuer 逼出来的） ---------- */

    /// macOS `scutil --proxy` 的输出：开了 HTTPS 代理 —— 拿它的地址
    #[test]
    fn scutil_https_proxy_is_read() {
        let text =
            "<dictionary> {\n  HTTPSEnable : 1\n  HTTPSProxy : 127.0.0.1\n  HTTPSPort : 7890\n}";
        assert_eq!(
            parse_scutil_proxy(text).as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    /// ★ **关掉的不算** —— 把 `HTTPSEnable : 0` 当成 1 是这类解析最容易犯的错，
    /// 后果是所有人（包括没开代理的）都连不上，而且看不出来是这里错了
    #[test]
    fn a_disabled_scutil_proxy_is_not_used() {
        let text =
            "<dictionary> {\n  HTTPSEnable : 0\n  HTTPSProxy : 127.0.0.1\n  HTTPSPort : 7890\n}";
        assert_eq!(parse_scutil_proxy(text), None);
    }

    /// 只开了 SOCKS：如实给 socks5（认不出的那一头会 warn 直连，不会崩）
    #[test]
    fn scutil_socks_proxy_is_read_too() {
        let text =
            "<dictionary> {\n  SOCKSEnable : 1\n  SOCKSProxy : 127.0.0.1\n  SOCKSPort : 1080\n}";
        assert_eq!(
            parse_scutil_proxy(text).as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
    }

    /// 认不出的输出 = 没有代理（**不猜**，直连还可能通）
    #[test]
    fn unreadable_scutil_output_means_no_proxy() {
        assert_eq!(parse_scutil_proxy(""), None);
        assert_eq!(parse_scutil_proxy("no such service"), None);
        assert_eq!(parse_scutil_proxy("HTTPSEnable : 1"), None, "缺地址就不算");
    }

    /// 环境变量：https 那几个先看，**空串不算**（设了却没值 = 没设）
    #[test]
    fn env_proxy_is_preferred_and_empty_values_are_skipped() {
        let env = [("HTTP_PROXY", ""), ("HTTPS_PROXY", "http://127.0.0.1:7890")];
        let lookup = |k: &str| {
            env.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| (*v).to_owned())
        };
        assert_eq!(
            proxy_from_env(&lookup).as_deref(),
            Some("http://127.0.0.1:7890")
        );

        let empty = [("HTTPS_PROXY", "   ")];
        let lookup = |k: &str| {
            empty
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| (*v).to_owned())
        };
        assert_eq!(
            proxy_from_env(&lookup),
            None,
            "环境变量设了却是空白 ⇒ 当作没设"
        );
    }

    /// 一个能用的代理地址，ureq 认得出来（**这条防的是"加了代理却从来没生效"** ——
    /// 解析写对了但构造失败，症状与没加一模一样）
    #[test]
    fn a_usable_proxy_url_is_accepted_by_ureq() {
        assert!(ureq::Proxy::new("http://127.0.0.1:7890").is_ok());
    }

    /// `Stage` 的词是发给界面看的形状：界面照着字符串分支，不用猜 Rust 枚举的顺序
    #[test]
    fn stage_words_are_stable() {
        assert_eq!(Stage::Connecting.as_str(), "connecting");
        assert_eq!(Stage::Transferring.as_str(), "transferring");
        assert_eq!(Stage::Done.as_str(), "done");
        assert_eq!(Stage::Failed.as_str(), "failed");
    }
}
