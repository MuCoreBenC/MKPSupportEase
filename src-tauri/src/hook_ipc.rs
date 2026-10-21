//! 钩子 ↔ 界面 的那条通道（一行一个 JSON，走 127.0.0.1）。
//!
//! # 为什么要有它（沿用一直以来的分工）
//!
//! 切片器调我们的方式是「起一个子进程，**等它退出**，看退出码」。所以窗口**不能住在钩子
//! 进程里**：那样"跑完不自动关"就等于"切片器一直卡着"，"下次复用同一扇窗"更是做不到
//! （下一次是**新的**子进程）。把窗口搬进界面那个常驻进程之后三件事同时成立：
//!
//! ```text
//! 钩子进程（跑完就退，切片器继续）              界面进程（常驻：留着、下次复用）
//!   connect(127.0.0.1:<port>) ────────────►  LoopbackListener::bind（bind 即单实例）
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
// loopback 通道住在 `runtime::net::loopback`（判据 ①：网络字节只许住那一处）——
// 这里只拿两个包装类型，不直接碰 `std::net`。
use crate::runtime::net::loopback::{LoopbackListener, LoopbackStream};
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
/// 等界面露面的宽限期（见 [`Client::ask`] 里那段两段倒计时）。
/// 起间：钩子第一次连不上就会把界面拉起来（[`SPAWN_WAIT`]），而那扇窗要冷启动 webview
/// 再去订阅事件 —— 这一段时间里"没人答"是正常的，不该算进取用时限。
pub const ANSWER_GRACE: Duration = Duration::from_secs(8);
/// 宽限期过后，等用户真正作答的上限（两段相加仍是原来 120 秒的量级）
pub const ANSWER_STAKE: Duration = Duration::from_secs(110);

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
    ///
    /// ★★ **字段名不许叫 `kind`**（2026-10-17 真机事故）：`FromHook` 的枚举标签就是
    /// `kind`（`#[serde(tag = "kind")]`），payload 再来一个同名字段，线上就成了
    /// `{"kind":"question","kind":"machine-mismatch",…}` —— 解析直接报
    /// `duplicate field 'kind'`，读循环按"认不出的行"把它丢掉，钩子那边却一直在等答复
    /// （界面上什么都没有，两分钟后被判成"不跑"）。判据见
    /// `tests::every_message_survives_the_wire`。
    pub question_kind: String,
    pub text: String,
    /// **没人作答时，什么时候自动按"不跑"收场**（unix 毫秒；`0` = 这一问不会自己超时）。
    ///
    /// 界面拿它显示倒计时（「1 分 52 秒后自动终止」）—— 作者 2026-10-17：
    /// 「文案里面出现自动停止的倒计时」。给的是**绝对时刻**而不是"还剩多少秒"：
    /// 界面是后来才画出来的（甚至补推过一次），按剩余秒数会在补推时重新从头数。
    pub deadline_unix_ms: u64,
    /// **可选答项**（2026-10-17 泛化：一问多答，不再只有"继续 / 停下"）。
    ///
    /// 空表 = 这一问只有默认那两颗按钮（机型那一问就是）；非空 = 界面照这张表画按钮，
    /// 点哪一颗就回哪个 `id`。文案由这里给（而不是界面按 id 猜），钩子才知道每个选项
    /// 对管线意味着什么。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<QuestionChoice>,
    /// **挪塔那一问附带的画布几何**（2026-10-18）：界面照它画俯视图
    /// （原塔位置 / 自动摆放位置 / 坐标），别的问没有 ⇒ `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<RelocateGeoPayload>,
}

/// 挪塔那一问的**画布几何 + 坐标**（机器坐标，mm；坐标不取整 —— 作者原话
/// 「可以显示坐标，不用整数也可以的」）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelocateGeoPayload {
    /// 碰撞那一整套事实（模型框 / 塔框 / 可打印区 / 冲突点 / 建议箭头）——
    /// 与报告页俯视图**同一份**结构，界面复用同一个画布。
    pub collision: Option<postprocess::postproc::disk::collision::CollisionFacts>,
    /// 自动摆塔算出来的塔**左下角**；`None` = 板子摆不下（那就没有这一问）
    pub candidate: Option<[f64; 2]>,
    /// 现在塔的左下角（`wiper_x` / `wiper_y`）
    pub current: [f64; 2],
}

/* ---------- 问句的稳定串（钩子与界面两边都认这几个字面量） ---------- */

/// 机型与预设对不上（两个默认选项：继续 / 终止）
pub const QKIND_MACHINE_MISMATCH: &str = "machine-mismatch";
/// 这是一份校准件：先打印再校准 / 直接校准
pub const QKIND_CALIBRATION_MODE: &str = "calibration-mode";
/// 模型与擦料塔相撞：自动把塔挪到不撞的位置 / 自己去挪模型（2026-10-18）
pub const QKIND_TOWER_RELOCATION: &str = "tower-relocation";

/// 机型那一问：继续（`keep = true`）
pub const CHOICE_KEEP: &str = "keep";
/// 任一问：停下 / 终止（**超时也回这个**）
pub const CHOICE_STOP: &str = "stop";
/// 挪塔那一问：自动把塔挪到算出来的那个位置（**只改这一趟**）
pub const CHOICE_RELOCATE: &str = "relocate";
/// 挪塔那一问：塔不动，用户自己回切片器挪模型 ⇒ 这一趟停下（与超时同一条）
pub const CHOICE_MODEL: &str = "model";
/// 校准那一问：先打印，再校准（模型原样跑）
pub const CHOICE_CALIB_PRINT: &str = "print";
/// 校准那一问：直接校准（去掉打印件，只留校准路径）
pub const CHOICE_CALIB_DIRECT: &str = "direct";

/// 问句里的一颗按钮。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionChoice {
    /// 回写给钩子的那个 id（`stop` / `keep` / `print` / `direct` …）—— 钩子按它分派
    pub id: String,
    /// 按钮上的字
    pub label: String,
    /// 一行补充说明（后果 / 何时选它）；`None` = 不写
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// 视觉强调：`true` = 主按钮（accent）
    #[serde(default)]
    pub accent: bool,
}

/// **开场那几个事实**（界面左栏那张表：这份 G-code 是哪台机、什么耗材、多大、喷嘴多粗）。
///
/// 只给界面看 —— 内核不知道有这张表，它是 [`crate::hook`] 从 IR 与输入正文里取的。
/// 一开场就报一条：不然用户对着一个光秃秃的进度条等十几秒，连"它在处理哪一份"都看不出来。
///
/// ★ 只报**开跑那一刻就确定**的那几格。总层数（pass1 才刮出来）与擦料塔类型
///   （support 步的判据才定）不在这一条里 —— 编一个出来比空着糟。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsPayload {
    /// G-code 自报的机型（`; printer_model` 那一档 —— 与预设那份比过的那一个）
    pub gcode_machine: String,
    /// 预设那份的机型（维度与打印边界都按它算）
    pub preset_machine: String,
    /// 耗材类型（`PLA` / `PETG` …）
    pub filament_type: String,
    /// 输入 G-code 的字节数
    pub file_size_bytes: u64,
    /// 喷嘴直径（mm）
    pub nozzle_diameter: f64,
    /// 层高（mm）—— 取常规层高那一档（旧世代那张表摆的就是「喷嘴直径 / 层高」）
    pub layer_height: f64,
    /// 涂胶速度（mm/s）——「切片与预估」那一格（跑着的时候就摆）。
    ///
    /// 换算（`toolhead.max_speed` 是 mm/min）与归档 `glueSpeed` 同一条：
    /// [`crate::archive::glue_speed_of`]。旧版钩子没报这一格 ⇒ `None`（界面不摆）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glue_speed: Option<f64>,
    /// 切片软件（预设里那个名字，如 `"BambuStudio"`）——
    /// **带版本号的完整串**要等跑完（`stats.slicer_version`，见 [`ResultFacts::slicer`]）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slicer: Option<String>,
    /// **切片器自报的预估总时长**（秒）—— G-code 头 `; total estimated time: 3h 22m 55s`
    /// 那一行（`postprocess::postproc::printtime::slicer_estimated_seconds`）。
    ///
    /// 「切片与预估」栏的「预估打印时间」用它（跑着的时候就摆）；跑完之后那一格换成
    /// 内核自己重算的（[`ResultFacts::print_time_seconds`]，含后增设）—— 两个数不一样正常。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slicer_print_time_seconds: Option<f64>,
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
    /// **结果读数**（界面上「结果」几扇要摆的那几格：指标条 + 擦料与胶水消耗）。
    ///
    /// 只有 `ok: true` 那一趟才带：失败 / 取消时内核还没把这几格算出来
    /// （它们住在 `ProcessResult` 里，见 `hook::result_facts`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultFacts>,
    /// 校准件那一趟选了哪种跑法：`print` = 先打印再校准（模型原样）/ `direct` = 直接校准
    /// （去掉打印件）。不是校准件 = `None`。
    ///
    /// 两种**都是成功**，界面同一个成功屏，只有那一句结论按它分文案；报告用同一个键。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calib_choice: Option<String>,
}

/// **结果读数** —— 成功那一趟的几格读数（界面「结果 · 成功」要摆的）。
///
/// 值与报告页那份同源：`_meta.json` 的 `detail.glue` / `detail.slicer` 就住在这里
/// （IR 终态 + pass1 统计 + 打印时间）。**这里只搬，不新算** ——
/// 唯一的变换是涂胶速度那次 mm/min → mm/s（[`crate::archive::glue_speed_of`]）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultFacts {
    /// 笔偏移 X / Y / Z（mm）—— 预设里那一组（`ir.toolhead.x_offset` …）
    pub pen_x: f64,
    pub pen_y: f64,
    pub pen_z: f64,
    /// 涂胶速度（mm/s）
    pub glue_speed: f64,
    /// 预估打印秒数。**延后估算那一趟这里是 `None`** —— 补算完由界面那条事件
    /// （`hook_ui::PRINT_TIME_EVENT`）再补回来，钩子不等它（切片器在等退出码）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_time_seconds: Option<f64>,
    /// 涂胶总长（mm）—— 报告页「涂胶」那一组的头一格（同一口径，不换算成体积）
    pub glue_length_mm: f64,
    /// 切片软件（`"BambuStudio 02.06.01.55"` —— pass1 从输入头行刮到的完整串）
    pub slicer: String,
}

/// 钩子 → 界面。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FromHook {
    /// 开场：告诉界面这一趟在处理哪两份文件（界面拿它写标题）—— **连左栏那张表的事实一起**
    /// （机型 / 耗材 / 大小 / 喷嘴 / 层高，见 [`FactsPayload`]）。
    ///
    /// ★ 2026-10-14 作者：「需要一起带过来」。原来事实是**紧跟着的单独一条**
    /// （`Hello` → `Facts`，差几十毫秒）：界面上就是"框先开成一条、数字再一个个蹦出来"——
    /// 作者说的"闪一下"。并成一条之后，框**一出现就是完整形态**，一个数都不会在眼前换。
    ///
    /// `facts: None` 只有一种情况：**还没走到读数那一步就失败了**（预设不在 / G-code 不在 /
    /// 读不动 / 载不动）—— 界面照旧要开窗（它得把失败那句话摆出来），只是表里全是 `—`。
    Hello {
        preset_name: String,
        gcode_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        facts: Option<FactsPayload>,
    },
    Progress(ProgressPayload),
    /// 问句（带倒计时与可选答项）。
    ///
    /// ★ `Box`：挪塔那一问（2026-10-18）把整份碰撞几何塞进来了 —— 不装箱会把这个
    /// 枚举撑大（clippy `large_enum_variant`，与内核 `PostprocError::Collision` 同一个原因）。
    Question(Box<QuestionPayload>),
    /// 打印时间估算**交给你了**：钩子写完盘就退，界面按 id 补全那条记录。
    ///
    /// 为什么钩子不能自己留后台任务：它必须**立刻退出**（切片器在等退出码）。
    /// 界面本来就常驻，是唯一合适的承接方。
    DeferPrintTime {
        record_id: String,
        output: String,
    },
    /// 结论。之后钩子就退了（界面把它留在屏上，等下一次）
    Finished(Box<FinishedPayload>),
    /// **开发台的演出指令**：让界面把某一屏"演"出来（进度 / 成功 / 失败 / 那一问…）。
    ///
    /// 它**不是钩子发的** —— 是工作台「开发台」发来的（跨进程，走同一条 loopback 口）。
    /// 收消息那一侧（`hook_ui`）会把 `scene` 交给 `devsim` 演一遍，演出走的是与真钩子
    /// **同一个落点**（`HookView::apply`），但会置一个 `preview` 标记 —— 界面据此把
    /// 那几颗真业务按钮禁掉、标上「预览模式」（作者 2026-10-14 裁定）。
    ///
    /// `scene` = `devsim::Mode::id()`（`success` / `fail` / `cancel` / `question`）
    /// 之外的稳定串；认不出的串**什么都不做**（如实，不编一屏出来）。
    DevScene {
        scene: String,
    },
    /// **开发台的故障注入**（2026-10-19）：让客户端把某一档错**装到某个 API 方法上**。
    ///
    /// 它**不是钩子发的** —— 是工作台「开发台」发来的（跨进程，走同一条 loopback 口），
    /// 与 [`FromHook::DevScene`] 同一族。
    ///
    /// ★★ **只过 `id`，不过那条 `AppError`**：错由**客户端进程**按 id 自己造
    /// （`workbench::app::devfault::fault_error`），于是演示出来的那条错与真机那条
    /// 走的是同一份代码、同一个码 —— 这正是作者要的"真实的 mock 触发"。
    ///
    /// `None` = **卸下**（客户端恢复正常）。
    DevFault {
        fault: Option<String>,
    },
}

/// 界面 → 钩子。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ToHook {
    /// 界面那颗「停止」
    Cancel,
    /// 回答某一问：`id` 就是 [`QuestionChoice::id`] 里的那一个。
    ///
    /// ★ 2026-10-17 从 `Answer { keep: bool }` 泛化而来（校准件那一问有两个以上选项，
    /// 布尔表达不了）。`question_kind` 一并带回：钩子要能判"你答的是不是我现在问的那一问"
    /// （界面上那一屏可能已经过期，见 [`replay_post_process_question`]）。
    Answer { question_kind: String, id: String },
}

/* ---------- 钩子那一侧 ---------- */

/// 钩子那一侧的线。**写不出去不是错误** —— 只是这次没有界面在看（产物照旧）。
///
/// 里面全是内部可变：进度出口（`&self`）与问句（`&self`）会**同时**被拿去用
/// （管线的 `sink` 与 `Asker` 是两个参数），所以不能要求 `&mut self`。
pub struct Client {
    conn: Mutex<Option<LoopbackStream>>,
    /// 答复从读线程转过来（只有 [`Client::ask`] 一个人消费）：`(question_kind, id)`
    answers: Mutex<Option<Receiver<(String, String)>>>,
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
            /* 先把界面拉起来，再每 25ms 试一次（总预算 SPAWN_WAIT）。
            ★ 只有 `ui_mode == gui` 才起窗（2026-10-17）：设置页「静默 / GUI 界面」
            那一档以前只是存了个值，钩子照旧会把窗拉起来 —— 勾了静默也没用。
            静默档下钩子照常干活、照常发消息，只是没有窗来接（`headless` 那一条）。 */
            if !spawned {
                if wants_display(internal_root) {
                    spawn_display();
                }
                spawned = true;
            }
            if Instant::now() >= deadline {
                return Self::headless(cancel);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// 连界面，**连不上就算了**（一个进程也不拉起）。
    ///
    /// 给开发用的假钩子（`src/bin/hook-sim.rs`）走的这一条：它要的只是"把消息发给**已经在跑**
    /// 的那扇窗"，**不能**走 [`Self::connect_or_spawn`] —— 那个拉起的是 `current_exe()`，
    /// 对假钩子来说就是它自己（一跑就套娃）。
    pub fn connect_or_headless(internal_root: &Path, cancel: &CancelToken) -> Self {
        match connect(internal_root) {
            Some(conn) => Self::with_conn(conn, cancel),
            None => Self::headless(cancel),
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

    fn with_conn(conn: LoopbackStream, cancel: &CancelToken) -> Self {
        let cancel = cancel.clone();
        let answers = conn.try_clone().ok().map(|read_side| {
            let (tx, rx) = mpsc::channel::<(String, String)>();
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

    /// 问一句并等答复；**返回被选中的 `id`**。
    ///
    /// 没有界面 / 等超时 / 用户按了停止 ⇒ `None`（**不替用户决定**，由调用方决定怎么收场）。
    ///
    /// - `question_kind`：这一问的类别（`machine-mismatch` / `calibration-mode` …）——
    ///   答复回来时要对得上，不然是界面上过期那一屏的答案；
    /// - `choices`：可选答项（空表 = 界面用默认那两颗按钮，回 `keep` / `stop`）；
    /// - `on_timeout`：倒计时走完**没有答复**时返回的那个 id（机型与校准两问都给 `stop`，
    ///   作者 2026-10-17：「超时就停止」）。**只有这一处声明**，界面不许另写一份"几秒后干嘛"。
    pub fn ask(
        &self,
        question_kind: &str,
        question: &str,
        choices: Vec<QuestionChoice>,
        on_timeout: &str,
    ) -> Option<String> {
        self.ask_with_geo(question_kind, question, choices, on_timeout, None)
    }

    /// 同 [`Self::ask`]，但**带画布几何**（2026-10-18 挪塔那一问：界面要画俯视图
    /// 与坐标，光有一句话答不明白）。
    pub fn ask_with_geo(
        &self,
        question_kind: &str,
        question: &str,
        choices: Vec<QuestionChoice>,
        on_timeout: &str,
        geo: Option<RelocateGeoPayload>,
    ) -> Option<String> {
        if !self.connected() {
            return None;
        }
        /* 界面上那个倒计时读的就是它：**两段都算进去**（界面露面那 8 秒也等） */
        let deadline_unix_ms = unix_ms_now() + (ANSWER_GRACE + ANSWER_STAKE).as_millis() as u64;
        self.send(&FromHook::Question(Box::new(QuestionPayload {
            question_kind: question_kind.to_owned(),
            text: question.to_owned(),
            deadline_unix_ms,
            choices,
            geo,
        })));

        /*
         * ★★ 两段倒计时（2026-10-17，作者真机踩到的那一版）：
         *
         * 原来的写法是"问出去就开始 120 秒"，但**问句到界面、界面画出来**之间有个真实的
         * 时间差（最坏是这扇窗正被这次钩子拉起：webview 冷启动 + 订阅，秒级）。那段时间
         * 用户在屏上看到的是"准备中 6%"—— 没有任何"要你回答"的提示，两分钟后被静默终止。
         *
         * 现在：
         *   · `ANSWER_GRACE`（8 秒）—— **等界面露面**。这段时间没人答是正常的；
         *   · 之后 `ANSWER_STAKE`（110 秒）—— 等用户真正作答。
         * 两段加起来仍是原来的 118 秒量级。**按停止随时生效**（取消线最优先）。
         */
        let appeared = Instant::now() + ANSWER_GRACE;
        let deadline = appeared + ANSWER_STAKE;
        let mut warned = false;
        loop {
            /* 按停止 = 不跑：取消要能打断"等你决定"那个等待（读线程直接置位取消线） */
            if self.cancel.is_cancelled() {
                return None;
            }
            let guard = self.answers.lock().expect("锁没坏");
            let rx = guard.as_ref()?;
            match rx.recv_timeout(Duration::from_millis(100)) {
                /* 答案到了：**只要是对这一问的**。界面上过期那一屏的答复不算（丢掉继续等） */
                Ok((kind, id)) => {
                    if kind == question_kind {
                        return Some(id);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    if now >= appeared && !warned {
                        warned = true;
                        /*
                         * 只是**留痕**（不吓人）：有人正在看那一屏、一时没点，也会走到这一行 ——
                         * 所以它说的是"还在等"，不是"界面坏了"。真出问题（问句没画出来）时，
                         * 这条正好是第一个线索：界面上本该有一个问句框。
                         */
                        eprintln!(
                            "[mkp] 还在等这一问的答复（{question_kind}；界面上应当有一个选择框）—— 再等 {} 秒就按\"{on_timeout}\"处理",
                            ANSWER_STAKE.as_secs()
                        );
                    }
                    if now >= deadline {
                        return Some(on_timeout.to_owned());
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
}

/// 钩子侧唯一的读循环：`cancel` 直接置取消线，`answer` 转给等答复的人。
fn read_loop(read_side: LoopbackStream, answers: Sender<(String, String)>, cancel: CancelToken) {
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
                    Ok(ToHook::Answer { question_kind, id }) => {
                        if answers.send((question_kind, id)).is_err() {
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
pub fn connect(internal_root: &Path) -> Option<LoopbackStream> {
    let port = read_port(internal_root)?;
    LoopbackStream::connect_local(port, CONNECT_PROBE).ok()
}

fn read_port(internal_root: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(internal_root.join(PORT_FILE)).ok()?;
    text.trim().parse::<u16>().ok()
}

/// 自己起一个界面进程（**分离**：stdin/out/err 全接 null）。
///
/// 为什么必须把所有管道接 null：钩子的 stdout/stderr 是**切片器的管道**，
/// 子进程若继承了它们，钩子退出后切片器还在等管道关闭 —— 那就成了"看起来还卡着"。
/// 这一趟**该不该起窗**（设置页「静默 / GUI 界面」那一档，2026-10-17）。
///
/// `true` = 起（GUI 档，也是出厂档）；`false` = 静默 —— 钩子照常跑、照常写盘，
/// 只是不拉那扇窗出来看。
///
/// ★ 读不出来（状态文件坏了 / 目录不在）**按 GUI 起窗**：静默是用户主动选的
///   "我这次不想看"，而不是默认；读不到偏好时退回"看得见"这一侧，
///   免得用户以为程序没动静。这与 `notify_on_finish` 那条兜底同一个取向。
///
/// ★★ 只影响"**新起**一扇窗"：已经开着的界面不受影响（用户自己开的窗当然留着）。
fn wants_display(internal_root: &Path) -> bool {
    crate::runtime::app_state::resolved_postproc_prefs(internal_root)
        .map(|p| p.ui_mode == crate::runtime::app_state::PostprocUiMode::Gui)
        .unwrap_or(true)
}

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
    listener: LoopbackListener,
    /// 当前那条连接的写端（界面回写"取消 / 答复"用它）
    current: Arc<Mutex<Option<LoopbackStream>>>,
}

/// 界面能不能当"那扇窗"：已经有人在做显示时给 `None`。
///
/// 判据与旧世代的 `bind_single_instance` 同形：**先连一下**，连得上 = 已经有一扇窗，
/// 那就不抢（`None`）；连不上 = 端点文件过期或没人在听，自己 bind 一个并把端口写下。
pub fn bind_for_display(internal_root: &Path) -> Option<Listener> {
    if connect(internal_root).is_some() {
        return None;
    }
    let listener = LoopbackListener::bind_any_local().ok()?;
    let port = listener.port().ok()?;
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
                                    /*
                                     * 认不出的行丢掉：钩子比界面新时，界面不该因此崩。
                                     *
                                     * ★★ 但**不许悄无声息**（2026-10-17）：问句那条消息的
                                     * payload 字段与枚举标签同名（`duplicate field 'kind'`），
                                     * 就是在这里被吞掉的 —— 界面上"什么都没有"，钩子那边一直
                                     * 在等答复，日志里一条线索都没有。
                                     */
                                    Err(err) => {
                                        tracing::warn!(
                                            error = %err,
                                            line = %head_of(trimmed, 160),
                                            "钩子发来的这一行认不出，丢掉"
                                        );
                                        continue;
                                    }
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
    current: Arc<Mutex<Option<LoopbackStream>>>,
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

/// 现在（unix 毫秒）。给问句那个"自动终止"的绝对时刻用。
pub(crate) fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 日志里露出来的那一截正文（一行可能很长：hello 里带着整张事实表）
fn head_of(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_owned(),
    }
}

fn write_port(internal_root: &Path, port: u16) {
    let at = internal_root.join(PORT_FILE);
    if let Some(dir) = at.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 走唯一的写盘出口（`fsx::atomic`）：端点文件也是本程序的数据，
    // 崩溃中途留半个端口号会让下一次钩子连到一个不存在的端口。
    let _ = crate::fsx::atomic::atomic_write(&at, format!("{port}\n").as_bytes());
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

    /// 每一档消息都必须**穿得过线**：序列化 → 反序列化还认得。
    ///
    /// ★ 2026-10-17 事故（作者真机）：`FromHook` 用 `tag = "kind"`，而 `QuestionPayload`
    /// 里**又有一个 `kind` 字段** —— 线上就成了
    /// `{"kind":"question","kind":"machine-mismatch","text":…}`。界面解析时按 tag 找变体，
    /// 读到的却是后面那个值，于是这条消息被当成"认不出的行"**静默丢掉**
    /// （读循环那条 `Err(_) => continue`），而钩子那边一直在等答复 —— 用户看到的是
    /// "没有选择框、卡在 6%"，两分钟后被当成"不跑"。
    ///
    /// 原来那条走真连接的测试只发了 hello / progress / finished，**唯一没有发的就是问句**，
    /// 所以这个 bug 一路穿到了真机。
    #[test]
    fn every_message_survives_the_wire() {
        let msgs = vec![
            FromHook::Hello {
                preset_name: "A1_MINI-fast.toml".to_owned(),
                gcode_name: "45600.0.gcode".to_owned(),
                facts: None,
            },
            FromHook::Progress(ProgressPayload {
                step: "pass1".to_owned(),
                fraction_in_step: Some(0.25),
                message: "第一遍处理中".to_owned(),
            }),
            FromHook::Question(Box::new(QuestionPayload {
                question_kind: "machine-mismatch".to_owned(),
                text: "G-code 是 A1，预设是 A1_MINI —— 机型对不上，结果可能出错。还继续吗？"
                    .to_owned(),
                deadline_unix_ms: 1_763_000_000_000,
                choices: Vec::new(),
                geo: None,
            })),
            /* 带选项表的那一问（2026-10-17 新增的校准件那一问就是这种） */
            FromHook::Question(Box::new(QuestionPayload {
                question_kind: "calibration-mode".to_owned(),
                text: "这是 Z 偏移校准件 —— 怎么跑？".to_owned(),
                deadline_unix_ms: 1_763_000_000_000,
                choices: vec![
                    QuestionChoice {
                        id: "print".to_owned(),
                        label: "先打印，再校准".to_owned(),
                        hint: Some("打印件保留".to_owned()),
                        accent: true,
                    },
                    QuestionChoice {
                        id: "direct".to_owned(),
                        label: "直接校准".to_owned(),
                        hint: Some("去掉打印件，只走校准路径".to_owned()),
                        accent: false,
                    },
                ],
                geo: None,
            })),
            FromHook::DeferPrintTime {
                record_id: "r1".to_owned(),
                output: "/tmp/o.gcode".to_owned(),
            },
            FromHook::Finished(Box::new(FinishedPayload {
                ok: true,
                cancelled: false,
                message: "处理完成".to_owned(),
                code: None,
                stage: Some("printtime".to_owned()),
                exit_code: 0,
                elapsed_ms: 123,
                output: None,
                warnings: Vec::new(),
                result: None,
                calib_choice: None,
            })),
            FromHook::DevScene {
                scene: "question".to_owned(),
            },
        ];
        for msg in msgs {
            let line = serde_json::to_string(&msg).expect("序列化");
            let back: FromHook = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("这一条过不了线：{line} —— {e}"));
            assert_eq!(back, msg, "往返之后要一字不差：{line}");
        }
    }

    /// **反方向**（界面 → 钩子）每一档也要过线：`cancel` 与带 id 的答句。
    ///
    /// 2026-10-17 泛化答句（`Answer{keep}` → `Answer{questionKind,id}`）时补的 ——
    /// 上一轮 `QuestionPayload.kind` 与枚举标签撞名那个事故，就是因为只测了从钩子出发的方向。
    #[test]
    fn every_inbound_message_survives_the_wire() {
        let msgs = vec![
            ToHook::Cancel,
            ToHook::Answer {
                question_kind: "machine-mismatch".to_owned(),
                id: "keep".to_owned(),
            },
            ToHook::Answer {
                question_kind: "calibration-mode".to_owned(),
                id: "direct".to_owned(),
            },
        ];
        for msg in msgs {
            let line = serde_json::to_string(&msg).expect("序列化");
            let back: ToHook = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("这一条过不了线：{line} —— {e}"));
            assert_eq!(back, msg, "往返之后要一字不差：{line}");
        }
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
            /* 线上那条只验"载得动"：事实那一包并进开场了，给 None 就够 */
            facts: None,
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
            /* 线上那条只验"载得动"：读数那一包不是这一条测的东西 */
            result: None,
            calib_choice: None,
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
        assert!(
            wait_until(|| cancel.is_cancelled()),
            "取消该传到钩子的取消线上"
        );
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

    /// 开发台那条演出指令也走这条线：`dev-scene` 一行 JSON 载得动、认得回。
    #[test]
    fn the_wire_carries_a_dev_scene() {
        let d = tempfile::tempdir().expect("临时根");
        let listener = bind_for_display(d.path()).expect("第一个界面进程该能当显示");
        let got = Arc::new(Mutex::new(Vec::<FromHook>::new()));
        let seen = got.clone();
        let _handle = listener.serve(move |msg| seen.lock().expect("锁没坏").push(msg));

        let cancel = CancelToken::new();
        let client = Client::connect_or_headless(d.path(), &cancel);
        assert!(client.connected(), "端点文件刚写出来，该连得上");
        /* 工作台发的那一条：只带一个稳定场景串（`devsim::Mode::id()`） */
        client.send(&FromHook::DevScene {
            scene: "fail".to_owned(),
        });

        assert!(
            wait_until(|| !got.lock().expect("锁没坏").is_empty()),
            "这一条该到"
        );
        let messages = got.lock().expect("锁没坏").clone();
        assert_eq!(
            messages[0],
            FromHook::DevScene {
                scene: "fail".to_owned()
            }
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
            facts: None,
        });
        assert!(
            client
                .ask(QKIND_MACHINE_MISMATCH, "继续吗？", Vec::new(), CHOICE_STOP)
                .is_none(),
            "没有界面看的时候不许替你决定"
        );
    }
}
