//! **开发台 · 真实后处理测试矩阵**（2026-10-19）。
//!
//! # 它解决什么
//!
//! 「单趟入口」（[`super::devreal`]）解决的是"我想手动跑一份看看"。这一块解决的是另一个问题：
//!
//! > 我怎么**确定性地**触发成功 / 失败 / 机型不匹配 / 挪塔碰撞 / 取消，
//! > 并且**每个结局都是真的**（真管线、真问句、真产物、真记录）？
//!
//! 答案不是"造一个更聪明的假"，而是：**给真入口换输入**。档位表里每一档只是一对
//! `(预设, G-code)`，结局由真管线决定 —— 夹具只提供**条件**，不提供**结论**。
//!
//! # 三条硬规矩（作者 2026-10-19 钉的）
//!
//! 1. **判定只读真凭据**：`_meta.json` 的 `error` / `cancelled` / `relocation`。
//!    不采信 stdout（stdio 接的是 null）、不由界面"认为"成功。
//!    碰撞档尤其：**必须有 `relocation`**，否则明确判失败 —— 不能因为退出码是 2 就当"撞过了"。
//! 2. **每档独立临时根、串行跑**：互不串味；工作台只负责启动 / 收集 / 判定，
//!    **不碰正式数据目录**。
//! 3. **问句档不代答**：机型不匹配 / 挪塔碰撞 / 取消都要人在**客户端窗口**里真操作 ——
//!    验的不只是"管线能问"，还有"界面能显示问句、能收答复、能继续/停下"。
//!
//! # 为什么数据能落临时根、界面却是真的
//!
//! 钩子那侧算根会先看 `MKP_SUPPORT_EASE_DATA`（[`crate::fsx::paths::DATA_ROOT_ENV`]），
//! 而**端点只是一个存端口号的文本文件**（`<根>/run/hook.port`）。所以：
//!
//! - 把临时根传给它 ⇒ 产物 / 执行记录 / 日志 / 用量账**全落临时根**；
//! - 在临时根里**预先写一份客户端的端口号** ⇒ 消息仍然发到**客户端窗口**，界面是真的。
//!
//! 代价一条（界面上要写明）：客户端收到 `Finished` 后会去**它自己那棵树**找那条记录补
//! 打印时间，找不到 ⇒ 一条 warn 后放弃 ⇒ 这一趟**不进报告页 / 历史**。这正是"临时"的含义。
//!
//! # 目录形状
//!
//! ```text
//! <repo>/target/dev-run/matrix/
//! ├── <档 id>-<时间戳>/
//! │   ├── run/hook.port          # 抄自客户端（消息发给它）
//! │   ├── run/app-state.json     # 抄自客户端（偏好跟着走，如 autoTowerPosition）
//! │   ├── in.gcode               # 夹具副本（真后处理原地覆盖的是它，源夹具不动）
//! │   ├── verdict.json           # 这一档的判定（本模块写，界面读）
//! │   ├── logs/                  # 钩子自己的日志（自动创建）
//! │   └── user/gcode_history/<日期>/<id>{.gcode,_original.gcode,_meta.json}
//! └── …（只留最近 [`KEEP_RECENT`] 个）
//! ```

/* 这一模块的写盘全部发生在 `target/dev-run/` 下（临时根），不是用户数据 ——
与 `devreal` 复制副本同一类，所以不走 `fsx::atomic` 那个洞。 */
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::paths;

use super::devreal::spawn_hook;

/// 矩阵目录（相对仓库根）。`target/` 已被 `.gitignore` 覆盖。
pub const MATRIX_DIR: &str = "target/dev-run/matrix";
/// 只保留最近这么多个档目录。
pub const KEEP_RECENT: usize = 10;

/// 一档的期望结局。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Expect {
    /// 跑完、没错误、没取消
    Success,
    /// 预设加载那一步真失败（`E_*`）
    HardFail,
    /// **必须**走到挪塔那一问（`relocation` 非 null）
    Collision,
    /// 机型那一问（人工档：结局取决于人在窗口里选什么）
    Mismatch,
    /// 人在客户端点「停止」
    Cancelled,
    /// **校准件跑完**：必须认成校准件（`detail.calibration.mode`）**且**认到档
    /// （`profile_id`）**且**没错误。人工档 —— 要在窗口里答「直接校准」或「先打印」。
    Calibration,
    /// **校准件问了没答**：按"停下"收场（`W_*`，不是 `E_*`）。人工档 —— 就是干等它超时。
    CalibrationStopped,
}

impl Expect {
    fn id(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::HardFail => "hard-fail",
            Self::Collision => "collision",
            Self::Mismatch => "mismatch",
            Self::Cancelled => "cancelled",
            Self::Calibration => "calibration",
            Self::CalibrationStopped => "calibration-stopped",
        }
    }
}

/// 一档的定义（界面照它摆一行）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCase {
    pub id: String,
    pub label: String,
    /// 这一档"凭什么触发什么"
    pub note: String,
    /// 期望结局（稳定串：`success` / `hard-fail` / `collision` / `mismatch` / `cancelled`）
    pub expect: String,
    /// 需要人在客户端窗口里真操作
    pub human: bool,
    /// 夹具都在（缺了就把这一档标出来，点了只报"夹具不在"）
    pub ready: bool,
    /// 输入说明（预设名 + 件名），界面直接摆
    pub input: String,
}

/// 一档跑完的判定（写进该档目录的 `verdict.json`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub case_id: String,
    /// `passed` / `failed`
    pub status: String,
    /// 一句话结论（人话，带真凭据）
    pub note: String,
    pub dir: String,
    pub started_at: String,
    pub elapsed_ms: u64,
}

/// 界面一次拿到的全貌。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixState {
    /// 档位表
    pub cases: Vec<MatrixCase>,
    /// 最近 [`KEEP_RECENT`] 个目录（新的在前）
    pub runs: Vec<Verdict>,
    /// 现在正在跑哪一档（没跑 = null）
    pub current: Option<String>,
}

/// 一档的输入**怎么摆** —— 校准档与普通档的差别只有这一处。
enum Layout {
    /// 普通档：`<临时根>/in.gcode`。
    Flat,
    /// 校准档：照**切片器临时目录**的形状摆
    /// （`postprocess::gcode::model_identity` 就是按这个形状找身份的）：
    ///
    /// ```text
    /// <临时根>/origin.txt          ← 一行 = 原始 3MF 的绝对路径（这就是"身份"）
    /// <临时根>/input/plate.gcode   ← 这一份
    /// ```
    ///
    /// ★ `temp_root()` 取的是 **gcode 的上一级**，所以 `origin.txt` 必须放
    /// `input/` 的**上一级**（即临时根根目录），不是和 gcode 并排。
    SlicerTemp { model: PathBuf },
}

impl Layout {
    /// 这一档额外依赖的模型文件（普通档没有）。
    fn model(&self) -> Option<&Path> {
        match self {
            Self::Flat => None,
            Self::SlicerTemp { model } => Some(model),
        }
    }
}

/// 一档的内部定义（路径都是绝对路径）。
struct Spec {
    id: &'static str,
    label: &'static str,
    note: &'static str,
    expect: Expect,
    human: bool,
    preset: PathBuf,
    gcode: PathBuf,
    layout: Layout,
}

fn fixtures(repo: &Path) -> PathBuf {
    repo.join("crates/postprocess/tests/fixtures")
}

fn delivery_preset(repo: &Path, name: &str) -> PathBuf {
    repo.join("presets/delivery/mkp/presets").join(name)
}

/// 八档。**顺序 = 跑全部时的顺序**（快的先跑，慢的 / 要人的放后面）。
fn specs(repo: &Path) -> Vec<Spec> {
    let fx = fixtures(repo);
    let big = fx.join("real/.5926.1.gcode");
    vec![
        Spec {
            id: "small-ok",
            label: "小文件成功",
            note: "一份最小但合规的件（机型头齐全）—— 几十毫秒跑完，用来快速回归：管线通、记录落、窗口收得到。",
            expect: Expect::Success,
            human: false,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: fx.join("dev/small-ok.gcode"),
            layout: Layout::Flat,
        },
        Spec {
            id: "big-ok",
            label: "大文件成功",
            note: "真正的切片件（8.5 MB）—— 看完整流程：预检、进度、塔位、结论。约 8 秒。",
            expect: Expect::Success,
            human: false,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: big.clone(),
            layout: Layout::Flat,
        },
        Spec {
            id: "hard-fail",
            label: "硬失败",
            note: "预设是坏的 TOML —— 加载预设那一步真失败，记录里带 E_ 码（实测 E_TOML_PARSE_001）。",
            expect: Expect::HardFail,
            human: false,
            preset: fx.join("dev/broken-preset.toml"),
            gcode: fx.join("dev/small-ok.gcode"),
            layout: Layout::Flat,
        },
        Spec {
            id: "mismatch",
            label: "机型不匹配",
            note: "A1 的件配 P1S 的预设 —— 客户端弹出机型那一问。★ 要你在窗口里真选：选「继续」则跑完，选「不跑」则停下。",
            expect: Expect::Mismatch,
            human: true,
            preset: delivery_preset(repo, "P1S-lite.toml"),
            gcode: big.clone(),
            layout: Layout::Flat,
        },
        Spec {
            id: "collision",
            label: "挪塔碰撞",
            note: "带支撑标记的件 + 模型点压在塔框里 —— 第 6 步一定撞，客户端弹出「自动挪塔」那一问。★ 要你在窗口里真选（最长等 110 秒）。",
            expect: Expect::Collision,
            human: true,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: fx.join("dev/collide-tower.gcode"),
            layout: Layout::Flat,
        },
        Spec {
            id: "cancel",
            label: "取消（人工）",
            note: "大件跑起来后，在客户端窗口点「停止」。只有真取消（记录里 cancelled + W_SYS_CANCELLED_001）才算通过。",
            expect: Expect::Cancelled,
            human: true,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: big,
            layout: Layout::Flat,
        },
        /*
         * 校准件两档（2026-10-19）：输入是**真 3MF 切片出来的件** + 一行 `origin.txt`
         * 指向仓库里那份 3MF —— 身份与认档两关都走产品代码，不是演的。
         *
         * ★ 为什么两档：作者钉的"判定只读真凭据"要求每档有唯一期望结局，
         *   而"答了跑完"与"问了没答"是两个不同结局（前者 `error=null` + 认到档，
         *   后者 `W_*` 停下），合成一档就只能含糊判过。
         */
        Spec {
            id: "calib-ok",
            label: "校准件 · 跑完（人工答）",
            note: "真校准件（仓库里的 3MF 切片而来）。客户端弹「先打印 / 直接校准」—— 随便答一个，跑完要求记录里认成校准件**且认到档**。",
            expect: Expect::Calibration,
            human: true,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: fx.join("dev/calib-zoffset-ext.gcode"),
            layout: Layout::SlicerTemp {
                model: repo.join("presets/assets/models/ZOffset_Calibration_ext.3mf"),
            },
        },
        Spec {
            id: "calib-question",
            label: "校准件 · 问了没答（人工）",
            note: "同一份校准件，弹了那一问**不答** —— 约 110 秒后自己按「停下」收场。判定：记录里有 W_ 停下且不是硬错。",
            expect: Expect::CalibrationStopped,
            human: true,
            preset: delivery_preset(repo, "A1-fastv3.3.toml"),
            gcode: fx.join("dev/calib-zoffset-ext.gcode"),
            layout: Layout::SlicerTemp {
                model: repo.join("presets/assets/models/ZOffset_Calibration_ext.3mf"),
            },
        },
    ]
}

fn cases_of(specs: &[Spec]) -> Vec<MatrixCase> {
    specs
        .iter()
        .map(|s| MatrixCase {
            id: s.id.to_owned(),
            label: s.label.to_owned(),
            note: s.note.to_owned(),
            expect: s.expect.id().to_owned(),
            human: s.human,
            ready: s.preset.is_file()
                && s.gcode.is_file()
                && s.layout.model().map_or(true, Path::is_file),
            input: format!(
                "{} + {}",
                s.preset
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                s.gcode
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            ),
        })
        .collect()
}

/// 现在正在跑的那一档（`None` = 空闲）。
///
/// 为什么放在进程内而不是盘上：钩子子进程的句柄只在这个进程里活着（`Child::wait`），
/// "还在不在跑"没有比它更准的东西。盘上的 `verdict.json` 才是跨会话的凭据。
static CURRENT: Mutex<Option<(String, PathBuf, Instant)>> = Mutex::new(None);
/// 串行闸：同时只许一个 worker。
static BUSY: Mutex<bool> = Mutex::new(false);

fn matrix_root() -> PathBuf {
    crate::workbench::paths::repo_root().join(MATRIX_DIR)
}

/// 客户端那棵树的内部根（**只读它拿端口号与偏好**，一个字节都不往里写）。
fn client_root() -> Result<PathBuf, AppError> {
    paths::internal_root_headless().ok_or_else(|| AppError::internal("算不出客户端内部数据根"))
}

/// 给一档开一个独立临时根，并把它需要的几样东西摆好。
///
/// 返回 `(临时根, 这一趟要处理的那份 gcode)` —— 普通档是 `<根>/in.gcode`；
/// 校准档是 `<根>/input/plate.gcode`（这样 `temp_root()` 才能读到上一级的 `origin.txt`）。
fn prepare(row: &Spec, client: &Path) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = matrix_root().join(format!("{}-{}", row.id, crate::hook_ipc::unix_ms_now()));
    std::fs::create_dir_all(dir.join("run"))
        .and_then(|_| std::fs::create_dir_all(dir.join("user")))
        .map_err(|e| {
            AppError::io(format!("建不出临时根：{}", dir.display())).with_detail(e.to_string())
        })?;

    /* ★ 这一行是"数据落临时根、消息发客户端"的全部秘密：端点只是一个端口号文本 */
    let port = client.join(crate::hook_ipc::PORT_FILE);
    if !port.is_file() {
        return Err(AppError::not_found(
            "客户端窗口没启动 —— 先跑 npm run tauri dev（这里不替你拉进程）",
        ));
    }
    std::fs::copy(&port, dir.join(crate::hook_ipc::PORT_FILE))
        .map_err(|e| AppError::io("抄不出端点端口号").with_detail(e.to_string()))?;

    /* 偏好跟着走：没有它，`autoTowerPosition` 回到出厂关（false）⇒ 碰撞档就不问了 */
    let state = client.join("run/app-state.json");
    if state.is_file() {
        let _ = std::fs::copy(&state, dir.join("run/app-state.json"));
    }

    /* 夹具副本：真后处理**原地覆盖输入**，所以覆盖的必须是副本 */
    let gcode = match &row.layout {
        Layout::Flat => {
            let p = dir.join("in.gcode");
            std::fs::copy(&row.gcode, &p)
                .map_err(|e| AppError::io("复制不出夹具副本").with_detail(e.to_string()))?;
            p
        }
        Layout::SlicerTemp { model } => {
            let input = dir.join("input");
            std::fs::create_dir_all(&input).map_err(|e| {
                AppError::io(format!("建不出 input：{}", input.display()))
                    .with_detail(e.to_string())
            })?;
            let p = input.join("plate.gcode");
            std::fs::copy(&row.gcode, &p)
                .map_err(|e| AppError::io("复制不出夹具副本").with_detail(e.to_string()))?;
            /* ★ 这一行是"身份"：钩子顺 `temp_root()`（= gcode 的上一级）读它，
            再去那份 3MF 里找 `MKPSE:Calibration`。**不复制 3MF**，只读。 */
            std::fs::write(dir.join("origin.txt"), format!("{}\n", model.display()))
                .map_err(|e| AppError::io("写不出 origin.txt").with_detail(e.to_string()))?;
            p
        }
    };
    Ok((dir, gcode))
}

/// 判定要看的那几个字段（`_meta.json` 的子集 —— 别的键不认识也不影响）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MetaView {
    error: Option<String>,
    cancelled: bool,
    relocation: Option<serde_json::Value>,
    detail: MetaDetail,
}

/// `_meta.json` 的 `detail` 里我们只关心校准那一组。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MetaDetail {
    calibration: Option<CalibrationView>,
}

/// 校准件那一趟的证据链（`archive::CalibrationFacts` 的子集）。
///
/// ★ 这一组是"这一趟到底被当成什么、用了哪一档"的**唯一凭据** ——
/// 判定"真的认出校准件了"要它，不能靠退出码或那句结论猜。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CalibrationView {
    /// 命中的档 id（`presets/calibration_models.toml` 的 `[[models]] id`）
    profile_id: Option<String>,
    /// 最终采用的校准身份（`ZOffset` / `Precise` / `Rough` / `Repetition`）
    mode: Option<String>,
}

/// 找这一档落下来的那条记录（`user/gcode_history/<日期>/<id>_meta.json`）。
fn read_meta(dir: &Path) -> Option<MetaView> {
    let history = dir.join("user").join(paths::GCODE_HISTORY_DIR);
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for day in std::fs::read_dir(&history).ok()?.flatten() {
        for f in std::fs::read_dir(day.path()).ok()?.flatten() {
            let p = f.path();
            if p.to_string_lossy().ends_with("_meta.json") {
                let mtime = f
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                    best = Some((mtime, p));
                }
            }
        }
    }
    let text = std::fs::read_to_string(best?.1).ok()?;
    serde_json::from_str(&text).ok()
}

/// 这条文案里的码是不是**硬错**（`E_`）。
///
/// ★ 不能只 `contains("E_")`：`W_CALIB_MODE_001` 里就含 `E_`（"MOD**E_**001"），
/// 那样会把"问了没答的停下"误判成硬失败。文案里的码都写在 `［…］` 里
/// （`messages.toml` 的渲染格式），所以连着左括号一起认。
fn is_hard_error(text: &str) -> bool {
    text.contains("［E_") || text.contains("[E_")
}

/// **判定**：只读真凭据，绝不"认为"。
///
/// 返回 `(是否通过, 一句话结论)`。结论里必须带上**判据本身**（哪一条字段说明通过），
/// 否则界面上一句"通过"没法复核。
fn judge(expect: Expect, meta: Option<&MetaView>) -> (bool, String) {
    let Some(meta) = meta else {
        return (
            false,
            "没有留下执行记录（`user/gcode_history/*/*_meta.json` 找不到）—— 这一趟没真跑起来"
                .to_owned(),
        );
    };
    let err = meta.error.as_deref().unwrap_or("");

    match expect {
        Expect::Success => {
            if meta.error.is_none() && !meta.cancelled {
                (true, "记录里 error 为空、未取消 —— 真跑完了".to_owned())
            } else {
                (
                    false,
                    format!(
                        "期望成功，实际 error={} cancelled={}",
                        if err.is_empty() { "null" } else { err },
                        meta.cancelled
                    ),
                )
            }
        }
        Expect::HardFail => {
            if is_hard_error(err) && !meta.cancelled {
                (true, format!("记录里是硬错：{err}"))
            } else {
                (
                    false,
                    format!(
                        "期望一条 E_ 硬错，实际 error={}",
                        if err.is_empty() { "null" } else { err }
                    ),
                )
            }
        }
        Expect::Collision => {
            if let Some(relocation) = &meta.relocation {
                let choice = relocation
                    .get("choice")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                (
                    true,
                    format!("碰撞那一问真出现过（`relocation.choice = {choice}`）"),
                )
            } else {
                (
                    false,
                    format!(
                        "**没产生碰撞问句**（`relocation` 是 null）—— 退出码/error 都不算数。error={}",
                        if err.is_empty() { "null" } else { err }
                    ),
                )
            }
        }
        Expect::Mismatch => {
            /* 人工档：问了之后，你选"继续"⇒ 成功；选"不跑"⇒ 停下。两种都算"问句真出现过"，
            硬错（E_）才算这一档失败 */
            if meta.cancelled {
                (false, "这一趟被取消，不是机型那一问的结局".to_owned())
            } else if is_hard_error(err) {
                (false, format!("期望机型那一问，实际是硬错：{err}"))
            } else {
                (
                    true,
                    if err.is_empty() {
                        "机型那两行对不上却跑完了 —— 说明你选了「继续」".to_owned()
                    } else {
                        format!("机型那一问走了停下那条：{err}")
                    },
                )
            }
        }
        Expect::Cancelled => {
            if meta.cancelled && err.contains("W_SYS_CANCELLED_001") {
                (true, format!("真取消：{err}"))
            } else {
                (
                    false,
                    format!(
                        "期望真取消（cancelled + W_SYS_CANCELLED_001），实际 cancelled={} error={}",
                        meta.cancelled,
                        if err.is_empty() { "null" } else { err }
                    ),
                )
            }
        }
        Expect::Calibration => {
            /* 「答了跑完」那一档：**认成校准件**且**认到档**，两条都从记录里读 */
            if meta.cancelled {
                (false, "这一趟被取消，不是校准件跑完的结局".to_owned())
            } else if !err.is_empty() {
                (false, format!("期望校准件跑完，实际 error={err}"))
            } else {
                match meta.detail.calibration.as_ref() {
                    Some(c) if c.mode.is_some() && c.profile_id.is_some() => (
                        true,
                        format!(
                            "认成校准件（{}）且认到档 `{}` —— 真跑完",
                            c.mode.as_deref().unwrap_or("?"),
                            c.profile_id.as_deref().unwrap_or("?")
                        ),
                    ),
                    Some(c) => (
                        false,
                        format!(
                            "记录里认成校准件（{}），但**没认到档**（profileId={:?}）\
                             —— 尺寸没落在 calibration_models.toml 的容差里",
                            c.mode.as_deref().unwrap_or("?"),
                            c.profile_id
                        ),
                    ),
                    None => (
                        false,
                        "记录里没有 `detail.calibration` —— 这一趟没被当成校准件（身份没读到？）"
                            .to_owned(),
                    ),
                }
            }
        }
        Expect::CalibrationStopped => {
            /* 「问了没答」那一档：停下（`W_*`），**不是**硬错。
             *
             * ★ 实测（2026-10-19）：这一种停下会同时把 `cancelled` 置 true
             *   —— 因为原文件确实没动、这一趟确实没跑。所以判据**不能**用
             *   "`cancelled` 为真 ⇒ 判失败"（那是取消档的语义），
             *   只认 `W_` 码 + 真的被标成停下。 */
            if is_hard_error(err) {
                (false, format!("期望那一问没人答会停下，实际是硬错：{err}"))
            } else if err.is_empty() {
                (
                    false,
                    "期望那一问没人答会停下，实际跑完了 —— 是不是有人答了？".to_owned(),
                )
            } else if !meta.cancelled {
                (
                    false,
                    format!("结束得不对：记录里 cancelled 不是 true（error={err}）"),
                )
            } else {
                (true, format!("问了没答，按停下收场：{err}"))
            }
        }
    }
}

fn write_verdict(dir: &Path, v: &Verdict) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(v)
        .map_err(|e| AppError::internal("判定写出不去").with_detail(e.to_string()))?;
    std::fs::write(dir.join("verdict.json"), text)
        .map_err(|e| AppError::io("判定写出不去").with_detail(e.to_string()))
}

/// 最近 [`KEEP_RECENT`] 个目录里读得到 verdict 的那些（新的在前）。
fn recent_verdicts() -> Vec<Verdict> {
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(matrix_root())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let m = e.metadata().and_then(|m| m.modified()).ok()?;
                    Some((m, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    dirs.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
    dirs.truncate(KEEP_RECENT);
    dirs.into_iter()
        .filter_map(|(_, p)| {
            let text = std::fs::read_to_string(p.join("verdict.json")).ok()?;
            serde_json::from_str(&text).ok()
        })
        .collect()
}

fn now_stamp() -> String {
    /* 只给界面看：`2026-10-19 12:30:05`（本地时区） */
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:05.2}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        f64::from(now.second()) + f64::from(now.nanosecond()) / 1e9
    )
}

/// 跑一档（在 worker 线程里，串行）。
fn run_one(row: &Spec, client: &Path) -> Result<(), AppError> {
    if !(row.preset.is_file() && row.gcode.is_file()) {
        return Err(AppError::not_found(format!(
            "「{}」的夹具不在（{} / {}）—— 大件是 gitignore 的，换台机器要单独放一份",
            row.label,
            row.preset.display(),
            row.gcode.display()
        )));
    }
    let (dir, in_gcode_path) = prepare(row, client)?;
    let started_at = now_stamp();
    let began = Instant::now();

    {
        let mut cur = CURRENT.lock().expect("锁没坏");
        *cur = Some((row.id.to_owned(), dir.clone(), began));
    }

    let in_gcode = in_gcode_path.display().to_string();
    let preset = row.preset.display().to_string();
    let mut child = spawn_hook(&preset, &in_gcode, &dir)?;
    /* 这一等可能要 110 秒（问句档等人答）—— 串行 worker 就是为此存在的 */
    let _ = child.wait();

    let meta = read_meta(&dir);
    let (ok, note) = judge(row.expect, meta.as_ref());
    let verdict = Verdict {
        case_id: row.id.to_owned(),
        status: if ok { "passed" } else { "failed" }.to_owned(),
        note,
        dir: dir.display().to_string(),
        started_at,
        elapsed_ms: began.elapsed().as_millis() as u64,
    };
    tracing::info!(
        case = row.id,
        status = %verdict.status,
        elapsed_ms = verdict.elapsed_ms,
        "开发台矩阵：一档跑完"
    );
    write_verdict(&dir, &verdict)
}

/// 档位表（界面照它摆；`ready: false` 的档点了只报夹具不在）。
#[tauri::command(async)]
pub fn wb_dev_matrix_cases() -> Vec<MatrixCase> {
    cases_of(&specs(&crate::workbench::paths::repo_root()))
}

/// 现在是不是在跑 + 跑到哪一档 + 最近的判定。
#[tauri::command(async)]
pub fn wb_dev_matrix_state() -> MatrixState {
    let repo = crate::workbench::paths::repo_root();
    let current = CURRENT
        .lock()
        .expect("锁没坏")
        .as_ref()
        .map(|(id, _, _)| id.clone());
    MatrixState {
        cases: cases_of(&specs(&repo)),
        runs: recent_verdicts(),
        current,
    }
}

/// **跑这些档**（按给定顺序串行；立刻返回，进度看 `wb_dev_matrix_state`）。
///
/// 传空 = 跑全部。问句档（不匹配 / 碰撞 / 取消）也会照跑 —— 它们只是**等你在客户端操作**，
/// 工作台不代答、也不替你点停止。
#[tauri::command(async)]
pub fn wb_dev_matrix_start(ids: Vec<String>) -> Result<(), AppError> {
    let repo = crate::workbench::paths::repo_root();
    let all = specs(&repo);
    let picked: Vec<Spec> = if ids.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|s| ids.iter().any(|id| id == s.id))
            .collect()
    };
    if picked.is_empty() {
        return Err(AppError::invalid_argument("没有这个档"));
    }

    {
        let mut busy = BUSY.lock().expect("锁没坏");
        if *busy {
            return Err(AppError::invalid_argument(
                "上一轮矩阵还在跑 —— 等它结束（问句档要等你在窗口里答完）",
            ));
        }
        *busy = true;
    }

    /* 先探一次客户端：没有窗口的话，每一档都会白跑一遍才报错，不如现在就说完 */
    let client = client_root()?;
    if !client.join(crate::hook_ipc::PORT_FILE).is_file() {
        *BUSY.lock().expect("锁没坏") = false;
        return Err(AppError::not_found(
            "客户端窗口没启动 —— 先跑 npm run tauri dev（这里不替你拉进程）",
        ));
    }

    std::thread::spawn(move || {
        for row in &picked {
            if let Err(e) = run_one(row, &client) {
                /* 一档起不来不该拖垮后面的：写一条失败判定（带原因）再继续 */
                tracing::warn!(case = row.id, error = %e.message, "开发台矩阵：这一档没跑起来");
                let dir =
                    matrix_root().join(format!("{}-{}", row.id, crate::hook_ipc::unix_ms_now()));
                let _ = std::fs::create_dir_all(&dir);
                let _ = write_verdict(
                    &dir,
                    &Verdict {
                        case_id: row.id.to_owned(),
                        status: "failed".to_owned(),
                        note: e.message.clone(),
                        dir: dir.display().to_string(),
                        started_at: now_stamp(),
                        elapsed_ms: 0,
                    },
                );
            }
        }
        *CURRENT.lock().expect("锁没坏") = None;
        *BUSY.lock().expect("锁没坏") = false;
    });
    Ok(())
}

/// **清空矩阵目录**（只删本功能自己的东西）。
///
/// 拒绝条件：正在跑的那一档目录 / 还有 worker 在跑 —— 作者钉的"清空前要确保没有任务在用"。
/// 返回删掉几个。
#[tauri::command(async)]
pub fn wb_dev_matrix_clear() -> Result<usize, AppError> {
    if *BUSY.lock().expect("锁没坏") {
        return Err(AppError::invalid_argument(
            "矩阵还在跑 —— 等它结束再清（问句档要先在窗口里答完）",
        ));
    }
    let root = matrix_root();
    let Ok(rd) = std::fs::read_dir(&root) else {
        return Ok(0);
    };
    let mut removed = 0usize;
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if std::fs::remove_dir_all(&path).is_ok() {
            removed += 1;
        }
    }
    tracing::info!(removed, "开发台矩阵：临时根已清空");
    Ok(removed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;

    /// **碰撞档的判据咬在 `relocation` 上**：没有它，哪怕 error 里写着碰撞也算失败
    /// （作者 2026-10-19：「不能只凭退出码判断成功，要确认确实产生了预期的碰撞问句」）
    #[test]
    fn collision_fails_without_a_relocation_record() {
        let no_reloc = MetaView {
            error: Some("处理失败：撞了［E_GCODE_COLLISION_001］".to_owned()),
            cancelled: false,
            relocation: None,
            ..Default::default()
        };
        let (ok, note) = judge(Expect::Collision, Some(&no_reloc));
        assert!(!ok, "没有 relocation 就不算撞过：{note}");
        assert!(note.contains("没产生碰撞问句"), "{note}");

        let with_reloc = MetaView {
            error: Some("处理失败：这一盘没做后处理…［W_COLLISION_RELOCATE_002］".to_owned()),
            cancelled: false,
            relocation: Some(serde_json::json!({ "choice": "timeout" })),
            ..Default::default()
        };
        let (ok, note) = judge(Expect::Collision, Some(&with_reloc));
        assert!(ok, "{note}");
        assert!(note.contains("timeout"), "{note}");
    }

    /// **取消档只认真取消**：普通成功、别的错误一律不过
    /// （作者：「不能把普通成功或其他错误算成取消测试通过」）
    #[test]
    fn cancel_needs_the_cancel_code() {
        let success = MetaView::default();
        let (ok, _) = judge(Expect::Cancelled, Some(&success));
        assert!(!ok, "成功不是取消");

        let other = MetaView {
            error: Some("处理失败：撞了［E_GCODE_COLLISION_001］".to_owned()),
            ..Default::default()
        };
        let (ok, _) = judge(Expect::Cancelled, Some(&other));
        assert!(!ok, "别的错误不是取消");

        let cancelled = MetaView {
            error: Some("已停止：用户取消［W_SYS_CANCELLED_001］".to_owned()),
            cancelled: true,
            relocation: None,
            ..Default::default()
        };
        let (ok, note) = judge(Expect::Cancelled, Some(&cancelled));
        assert!(ok, "{note}");
    }

    /// 没留下记录 ⇒ 一律失败（"没跑起来"不能被当成通过）
    #[test]
    fn a_missing_record_is_never_a_pass() {
        for expect in [
            Expect::Success,
            Expect::HardFail,
            Expect::Collision,
            Expect::Mismatch,
            Expect::Cancelled,
            Expect::Calibration,
            Expect::CalibrationStopped,
        ] {
            let (ok, note) = judge(expect, None);
            assert!(!ok, "{expect:?} 没记录也该失败");
            assert!(note.contains("没有留下执行记录"), "{note}");
        }
    }

    /// 成功档：error 为空且未取消
    #[test]
    fn success_needs_a_clean_record() {
        assert!(judge(Expect::Success, Some(&MetaView::default())).0);
        let failed = MetaView {
            error: Some("处理失败：…［E_TOML_PARSE_001］".to_owned()),
            ..Default::default()
        };
        assert!(!judge(Expect::Success, Some(&failed)).0);
        let cancelled = MetaView {
            cancelled: true,
            ..Default::default()
        };
        assert!(!judge(Expect::Success, Some(&cancelled)).0);
    }

    /// 硬失败档要的是 `E_` 码 —— 取消（W_）不算硬失败
    #[test]
    fn hard_fail_needs_an_error_code() {
        let e = MetaView {
            error: Some("处理失败：…［E_TOML_PARSE_001］".to_owned()),
            ..Default::default()
        };
        assert!(judge(Expect::HardFail, Some(&e)).0);
        let w = MetaView {
            error: Some("已停止：…［W_SYS_CANCELLED_001］".to_owned()),
            cancelled: true,
            relocation: None,
            ..Default::default()
        };
        assert!(!judge(Expect::HardFail, Some(&w)).0, "W_ 不是硬失败");
    }

    /// 六档的 id 不许重、期望不许漏；`human` 只给三档人工
    #[test]
    fn the_case_table_is_sane() {
        let repo = std::path::Path::new("/repo");
        let rows = specs(repo);
        assert_eq!(rows.len(), 8);
        let mut ids: Vec<_> = rows.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 8, "档 id 不许重");

        let human: Vec<_> = rows.iter().filter(|r| r.human).map(|r| r.id).collect();
        assert_eq!(
            human,
            vec![
                "mismatch",
                "collision",
                "cancel",
                "calib-ok",
                "calib-question"
            ],
            "问句档 / 取消档 / 校准两档都要人在客户端操作"
        );
    }

    /// **校准档的判据咬在 `detail.calibration` 上**：认成校准件但没认到档 ⇒ 失败。
    /// （作者：「判定只读真凭据」—— 退出码 0 不代表用的是哪一档）
    #[test]
    fn calibration_needs_a_matched_profile() {
        let matched = MetaView {
            detail: MetaDetail {
                calibration: Some(CalibrationView {
                    profile_id: Some("zoffset-ext".to_owned()),
                    mode: Some("ZOffset".to_owned()),
                }),
            },
            ..Default::default()
        };
        let (ok, note) = judge(Expect::Calibration, Some(&matched));
        assert!(ok, "{note}");
        assert!(note.contains("zoffset-ext"), "{note}");

        let no_profile = MetaView {
            detail: MetaDetail {
                calibration: Some(CalibrationView {
                    profile_id: None,
                    mode: Some("ZOffset".to_owned()),
                }),
            },
            ..Default::default()
        };
        let (ok, note) = judge(Expect::Calibration, Some(&no_profile));
        assert!(!ok, "没认到档不算跑完：{note}");
        assert!(note.contains("没认到档"), "{note}");

        let not_calibration = MetaView::default();
        let (ok, note) = judge(Expect::Calibration, Some(&not_calibration));
        assert!(!ok, "{note}");
        assert!(note.contains("detail.calibration"), "{note}");
    }

    /// 「问了没答」档：**`W_` 停下**才算通过；跑完了 / 硬错都不算
    /// （与取消档同一条纪律：不能拿"别的结局"冒充）
    ///
    /// ★ 这一档的停下会**同时置 `cancelled = true`**（实测：
    ///   `error = …［W_CALIB_MODE_001］`、`cancelled: true`），
    ///   所以判据不能写成"cancelled 为真就失败"。
    #[test]
    fn calibration_stopped_needs_a_warning_not_an_error() {
        let stopped = MetaView {
            error: Some("已停止：校准件没有选校准方式…［W_CALIB_MODE_001］".to_owned()),
            cancelled: true,
            ..Default::default()
        };
        assert!(judge(Expect::CalibrationStopped, Some(&stopped)).0);

        let done = MetaView::default();
        assert!(
            !judge(Expect::CalibrationStopped, Some(&done)).0,
            "跑完了不是超时停下"
        );

        let hard = MetaView {
            error: Some("处理失败：…［E_CAL_MISMATCH_002］".to_owned()),
            ..Default::default()
        };
        assert!(
            !judge(Expect::CalibrationStopped, Some(&hard)).0,
            "硬错不是停下"
        );

        let not_stopped = MetaView {
            error: Some("已停止：…［W_CALIB_MODE_001］".to_owned()),
            cancelled: false,
            ..Default::default()
        };
        assert!(
            !judge(Expect::CalibrationStopped, Some(&not_stopped)).0,
            "记录里没标成停下，不算"
        );
    }
}
