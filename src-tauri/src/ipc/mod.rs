//! IPC 边界：前端能调的全部命令。
//!
//! 每个命令都走 [`traced`] 包一层：生成 trace id → 开 span → 调业务 → 给错误盖上同一个 id。
//! 于是界面上显示的 traceId 与日志里的 span 是同一个值，按 id 能把一次调用的全过程捞出来。
//!
//! **这一份只剩三条锚在"应用本身"上的命令**：校准板清单（[`get_calib_models`]）、
//! 打开模型文件（[`open_model`]）与首页「复制后处理脚本」的**整条命令**
//! （[`get_post_process_command`]）。三件都读**真实的事实**：
//! 模型是 catalog 登记的三个交付文件（`assets/models/*.3mf`），本机有没有以盘为准；
//! 打开走系统默认程序（`.3mf` 关联的切片器）；后处理那一条由两段真值现拼 ——
//! 可执行物 = 本程序自己（`current_exe()`），预设 = 本机真实的绝对落点。
//! 命令**两个平台同一形状、没有任何写死的路径**：macOS 用 macOS 的 exe 与落点，
//! Windows 用 Windows 的（2026-10-10 作者裁定：按钮常驻、命令动态生成）。
//! 旧实现里"只记日志"的空壳与恒 `ready` 的假状态都已删掉。
//! 业务数据（预设 / 参数 / 下载区 / 使用中…）全在 [`presets`] / [`mine`] / [`catalog`]
//! 那几个模块里，读的是 catalog 与数据根 —— 与 `src/api/contract.ts` 一一对应。
//!
//! **2026-10-02 清扫**：首圈的 `get_preset`（一张 v023 时代的硬编码表，`preset_of`）删除 ——
//! 首页与校准页早已改走文件体系（`getVersionFiles` + `getMachineParams`），
//! 它只剩测试与注册清单在引用，是"平行真相"的残留。
//!
//! **2026-10-08 校准改判**：`save_offsets`（三轴落 `index/offsets.json`，只写不读的孤岛）
//! 退役 —— 校准值随**用户工作副本**走（`ipc::mine::save_preset_calibration` 写进那份
//! TOML 的 `offset_x/y/z`），落点与读写口都在用户线那一族里。

pub mod catalog;
/// **通用导入入口**（第十二层）：看落点（`stage_import`）与提交（`commit_import`）
pub mod import;
pub mod mine;
/// **逐参数「官方更新」的两条命令**（2026-10-09）：读三方账（我 / 官方旧值 / 官方新值）
/// 与落采用·保持的决定。官方那一版默认值住在隐藏 baseline 里，用户看不见它。
pub mod param_sync;
/// **钩子模式那三条命令**（2026-10-09）：看那一趟 / 停那一趟 / 答那一问 ——
/// 那一趟本身住 [`crate::hook_ui`]（切片器导出 G-code 时起的那一次）。
pub mod postprocess;
/// **baseline 的两条命令**（2026-10-08）：云端版本列表（已下载 / 新版本，**不是过时判定**）
/// 与「恢复默认」的基准值。baseline 是隐藏内部存储，用户看不见它。
pub mod preset_baseline;
/// **对比台的两条命令**（2026-10-08）：读一份用户预设的参数 / 把改过的几项写回它自己。
/// **只在用户自己的预设之间对比** —— 官方基线不进对比台。
pub mod preset_params;
pub mod presets;
/// **报告页的读口**：后处理执行报告与历史（`gcode_history/*_meta.json`，只读）
pub mod report;
/// **应用内更新 + 打开外链**（2026-10-05 第五刀）。`open_url` 是修「查看更新点了没反应」
/// 那个 bug 的：webview 没 opener 权限，外链一律过命令
pub mod update;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::obs::tracing::new_trace_id;

/// **AppState 变更事件**（`docs/APP-STATE.md` §7）：任何写命令成功改了
/// `run/app-state.json` 之后广播。前端唯一的 AppState 客户端接住它重读整份状态、
/// 再通知订阅的页面 —— 这是"改了就推"的唯一通道，页面自己不持有状态副本。
pub const APP_STATE_EVENT: &str = "app-state-changed";

/// 广播一次 AppState 变更（尽力而为：界面不在也不影响写本身的成功）
pub fn notify_app_state(app: &AppHandle) {
    use tauri::Emitter as _;
    let _ = app.emit(APP_STATE_EVENT, ());
}

/// 三轴偏移，单位 mm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Axes {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibModel {
    pub id: String,
    pub name: String,
    pub desc: String,
    /// 交付文件名（catalog 登记的那个）。**取回 / 打开都认它** —— 与下载管道同一个口径
    pub file_name: String,
    /// 本机那份的真实大小。`None` = 本机还没有这份文件 —— **不编一个假大小**（旧实现
    /// 写死 "284 KB" 那种，界面看着像有文件，其实一个字节都不在盘上）
    pub size: Option<String>,
    /// 本机有没有这份文件（下载区或旧缓存里找得到就算有），以磁盘为唯一权威
    pub ready: bool,
}

/// 命令包装：trace id 贯穿 span 与错误。
///
/// 写成高阶函数而不是宏：宏能少写一点字，但报错信息会指向宏展开后的位置，
/// 调试时要多绕一道。这里的重复只有一行。
///
/// `pub(crate)` 是为了让 `workbench` 模块也走同一层 —— 工作台出错时同样要能按
/// traceId 去日志里捞，不该为它再造一套包装。
pub(crate) fn traced<T>(
    name: &'static str,
    f: impl FnOnce(&str) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let trace_id = new_trace_id();
    let span = tracing::info_span!("ipc", command = name, trace_id = %trace_id);
    let _guard = span.enter();

    match f(&trace_id) {
        Ok(v) => {
            tracing::info!("ok");
            Ok(v)
        }
        Err(e) => {
            // 错误进日志一次（带 detail），给前端的那份 message 保持可展示
            tracing::warn!(code = ?e.code, detail = ?e.detail, "{}", e.message);
            Err(e.with_trace(&trace_id))
        }
    }
}

/* ---------- 校准模型（三个 3mf） ---------- */

/// 界面里的模型 id → 交付文件名。**id 是契约**（首页 / 校准页发的就是这三个），
/// 文件名是 catalog 的口径（「取回」走下载管道、「打开」找盘上那份，都认它）。
///
/// 映射只有这一处。id 对不上就报"没这个模型"，不猜、不静默少一项。
const MODEL_DEFS: [(&str, &str, &str, &str); 3] = [
    (
        "z",
        "Z 轴校准",
        "校准喷嘴高度与第一层，先打这个",
        "ZOffset_Calibration.3mf",
    ),
    (
        "xy",
        "XY 校准",
        "校准平面内的偏移，Z 轴之后打",
        "Precise_Calibration.3mf",
    ),
    (
        "test-models",
        "支撑测试",
        "校准完打这个看支撑效果",
        "MKP_support_test_models.3mf",
    ),
];

/// 本机那份模型文件在哪：**下载区**（catalog `path` 的落点）优先 —— 那是交付管道的
/// 口径；没有就退**旧世代缓存**（`<MKPSupportSSR>/models/`，用户机器上早就有的那三个，
/// 见 [`crate::legacy`]）。两处都没有 → `None`（"还没取回"，不是错误）。
///
/// catalog 里没有这个文件的登记时**不猜落点**：下载区那一半直接没有，只剩旧缓存可查。
fn locate_model_file(
    app: &AppHandle,
    file_name: &str,
) -> Result<Option<std::path::PathBuf>, AppError> {
    let internal = crate::fsx::paths::internal_root(app)?;
    let in_delivery = crate::runtime::load_released_catalog(&internal)
        .ok()
        .and_then(|catalog| {
            catalog
                .files
                .iter()
                .find(|f| f.kind == "model" && f.file_name == file_name)
                .map(|f| internal.join(&f.path))
        })
        .filter(|p| p.is_file());
    if in_delivery.is_some() {
        return Ok(in_delivery);
    }
    let legacy = crate::legacy::data_root(app)
        .map(|root| crate::legacy::models_dir(&root).join(file_name))
        .filter(|p| p.is_file());
    Ok(legacy)
}

/// 字节数 → 界面上的大小。只在**盘上真有那份文件**时给 —— 没有就 `None`，不编数。
fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/* ---------- 三条命令（锚在应用本身上的） ---------- */

#[tauri::command]
pub fn get_calib_models(app: AppHandle) -> Result<Vec<CalibModel>, AppError> {
    traced("getCalibModels", |_| {
        let mut out = Vec::with_capacity(MODEL_DEFS.len());
        for (id, name, desc, file_name) in MODEL_DEFS {
            let path = locate_model_file(&app, file_name)?;
            let size = match &path {
                Some(p) => std::fs::metadata(p).ok().map(|m| size_text(m.len())),
                None => None,
            };
            out.push(CalibModel {
                id: id.to_owned(),
                name: name.to_owned(),
                desc: desc.to_owned(),
                file_name: file_name.to_owned(),
                size,
                ready: path.is_some(),
            });
        }
        Ok(out)
    })
}

/// 打开一份模型文件：用**系统默认程序**（`.3mf` 关联的切片器）打开盘上那份。
///
/// - 本机还没有这份文件 → `NOT_FOUND`，原话告诉用户先「取回」—— **不静默、不假装打开**；
/// - 系统打不开（没有关联程序那种）→ `IO` 原样冒上来，界面照实显示。
///
/// （下载是另一件事：「取回」走 `download_catalog_file`，一个动作一个命令。）
#[tauri::command]
pub fn open_model(app: AppHandle, model_id: String) -> Result<(), AppError> {
    traced("openModel", |_| {
        let id = model_id.trim();
        let Some((_, _, _, file_name)) = MODEL_DEFS.iter().find(|(mid, ..)| *mid == id) else {
            return Err(AppError::invalid_argument(format!(
                "没这个模型：{model_id}"
            )));
        };
        let Some(path) = locate_model_file(&app, file_name)? else {
            return Err(AppError::not_found(format!(
                "本机还没有 {file_name} —— 先在界面上「取回」，再打开"
            )));
        };
        use tauri_plugin_opener::OpenerExt as _;
        app.opener()
            .open_path(path.to_string_lossy().into_owned(), None::<&str>)
            .map_err(|e| AppError::io("系统打不开这个模型文件").with_detail(e.to_string()))?;
        tracing::info!(model_id = id, path = %path.display(), "模型已交给系统打开");
        Ok(())
    })
}

/* ---------- 「复制后处理脚本」那一整条命令（前端不拼路径） ---------- */

/// 首页那颗「复制后处理脚本」现在能不能给、给了是什么（**那一条命令的唯一数据面**）。
///
/// # 两个概念分开（2026-10-10 作者裁定）
///
/// - **按钮常驻**：界面规则 —— 前端**不拿** `ready` 决定摆不摆按钮，只拿它显示
///   "现在能不能抄"（不能抄时把 `reason` 说出来，点了也不抄假的）；
/// - **命令能不能给**：两条**真实事实**说了算 —— 本程序自己的路径（`current_exe()`）
///   与预设真在本机。给不出来只报原因，**绝不编一条指不到东西的命令**。
///
/// # 为什么 exe 是本程序自己
///
/// 后处理不是别人的事：内核就在本仓库（`crates/postprocess`），这条命令由**本程序自己**
/// 跑 —— 一个可执行物两种角色：**带 `--Toml/--Gcode` 时是切片器的后处理钩子，不带参数就是
/// 界面**（旧世代 `mkp-ssr` 也是这个形状）。这段路径没有第二种真值：开发态是
/// `target/debug/…`、装好了就是安装位置，只有 `current_exe()` 知道。前端**不许再写死一串**：
/// 2026-10-09 之前首页写死的是**另一个仓库**的 exe（`G:\project\mkp-ssr\…`），
/// 照着复制出来的命令贴进切片器必然报 `Error code: 2`（实测）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessCommandDto {
    /// 两个事实都成立才有命令
    pub ready: bool,
    /// `ready` 时 = 整条命令（**当前平台、当前实例、真实落点**现拼）；否则没有
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// `ready` 时 = `--Toml` 指的那份预设的**本机绝对路径**（给人看 / 给判据）；否则没有
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toml: Option<String>,
    /// 不能给时的一句人话（为什么 + 下一步做什么）；能给时没有
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 一段路径包成命令行参数：双引号是**两个平台的公共分母**（`cmd` 与 sh 都认它），
/// 空格 / 中文 / 括号在引号里原样活着。
///
/// 路径里的 `"` 本身要转义成 `\"` —— Windows 的文件名根本不许有它，
/// 所以那一支只可能在 macOS / Linux 上遇到（按 sh 的写法转义即可）。
fn quote_arg(raw: &str) -> String {
    format!("\"{}\"", raw.replace('"', "\\\""))
}

/// 整条后处理命令：`"<exe>" --Toml "<toml>" --Gcode`。
///
/// 形状与钩子那一侧的约定对齐（[`crate::args`]）：`--Toml` 后面必须是**本机绝对路径**
/// —— 切片器起钩子时的工作目录不是我们的数据根，相对落点在那里找不到文件。
/// 两段都从入参来，**没有写死的路径、没有平台分支**：macOS 用得就是 macOS 上那个
/// `current_exe()` 与 macOS 的落点，Windows 同理（同一份代码、各自的真值）。
fn post_process_command(exe: &std::path::Path, toml: &std::path::Path) -> String {
    format!(
        "{} --Toml {} --Gcode",
        quote_arg(&exe.display().to_string()),
        quote_arg(&toml.display().to_string())
    )
}

fn post_process_refused(reason: impl Into<String>) -> PostProcessCommandDto {
    PostProcessCommandDto {
        ready: false,
        command: None,
        toml: None,
        reason: Some(reason.into()),
    }
}

/// 组装那一条命令（**纯函数**：喂假 exe 与临时根就能判 —— 见下面的判据）。
///
/// 预设那一份的先后：**我那一份**（用户工作副本，匹配口径在
/// [`mine::user_copy_for`]）> 官方交付那份，且**必须真在盘上**
/// （相对落点 / 不存在的文件贴进切片器都是同一种事故）。
fn post_process_dto(
    exe: Option<&std::path::Path>,
    catalog: &crate::runtime::Catalog,
    internal_root: &std::path::Path,
    user_root: &std::path::Path,
    machine_id: &str,
    version_id: &str,
) -> Result<PostProcessCommandDto, AppError> {
    let Some(exe) = exe else {
        return Ok(post_process_refused(
            "取不到本程序自己的路径（current_exe 失败），这一台上暂时拼不出命令",
        ));
    };

    let at = match mine::user_copy_for(internal_root, user_root, catalog, machine_id, version_id)? {
        /* 我那一份：扫出来时状态就已经是 Ok（能读 + TOML 语法过），绝对路径不是猜的 */
        Some(f) => user_root.join(&f.path),
        None => match catalog.file_of(machine_id, version_id) {
            None => {
                return Ok(post_process_refused(format!(
                    "目录里没有 {machine_id} / {version_id} 的预设文件 —— 这条命令没有可指的预设"
                )));
            }
            Some(f) => {
                let at = internal_root.join(&f.path);
                if !at.is_file() {
                    return Ok(post_process_refused(format!(
                        "「{}」还没下载到本机 —— 先点「下载并应用」，再复制这条命令",
                        f.file_name
                    )));
                }
                at
            }
        },
    };

    Ok(PostProcessCommandDto {
        ready: true,
        command: Some(post_process_command(exe, &at)),
        toml: Some(at.display().to_string()),
        reason: None,
    })
}

/// 首页「复制后处理脚本」要的**那一整条命令**（当前平台现拼）。
///
/// 前端点一下要一次、状态一变再要一次（投递面 / AppState 广播）—— **不缓存路径**：
/// 下载、应用、换份都可能把 `--Toml` 指的落点换掉。
#[tauri::command]
pub async fn get_post_process_command(
    app: AppHandle,
    machine_id: String,
    version_id: String,
) -> Result<PostProcessCommandDto, AppError> {
    traced("getPostProcessCommand", |_| {
        let internal = crate::fsx::paths::internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let catalog = crate::runtime::load_released_catalog(&internal)?;
        let exe = std::env::current_exe().ok();
        post_process_dto(
            exe.as_deref(),
            &catalog,
            &internal,
            &user_root,
            &machine_id,
            &version_id,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calib_models_have_three_entries() {
        assert_eq!(MODEL_DEFS.len(), 3);
    }

    /// 界面发的三个 id 就是契约 —— 换名字要两处一起改，判据钉在这里
    #[test]
    fn model_ids_match_the_pages() {
        let ids: Vec<&str> = MODEL_DEFS.iter().map(|(id, ..)| *id).collect();
        assert_eq!(ids, ["z", "xy", "test-models"]);
    }

    #[test]
    fn size_text_only_for_real_bytes() {
        assert_eq!(size_text(512), "512 B");
        assert_eq!(size_text(284 * 1024), "284 KB");
        assert_eq!(size_text(3_250_000), "3.1 MB");
    }

    /* ---------- 「复制后处理脚本」那一整条命令（2026-10-10） ---------- */

    /// 真仓库的源 + 入库产物 → 真目录（与 `ipc::presets` 判据同一个输入）
    fn catalog() -> crate::runtime::Catalog {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        crate::runtime::Catalog::build_from_repo(&repo).expect("真目录构建不出来")
    }

    /// **命令由两段真值现拼**：形状固定（钩子那一侧的约定），空格 / 中文都包在引号里。
    ///
    /// 这条判据在**两个平台的 CI 上跑同一份期望** —— 命令形状与平台无关，
    /// 两段内容各来自当时的入参（macOS 给 macOS 的、Windows 给 Windows 的）。
    #[test]
    fn the_command_is_built_from_the_two_real_paths_and_survives_spaces() {
        let exe = std::path::Path::new("程序 目录/SupportEase");
        let toml = std::path::Path::new("我的 预设/A1 mini.toml");
        let cmd = post_process_command(exe, toml);
        assert_eq!(
            cmd, "\"程序 目录/SupportEase\" --Toml \"我的 预设/A1 mini.toml\" --Gcode",
            "两段都包双引号（空格 / 中文能扛），参数只有 --Toml / --Gcode"
        );
        assert_eq!(cmd.matches("--Toml").count(), 1, "只有一处 --Toml：{cmd}");
        assert!(cmd.ends_with(" --Gcode"), "收尾固定：{cmd}");
    }

    /// 路径里真出现 `"` 时要**转义留着**，不能截断成半条命令
    /// （Windows 的文件名不许有 `"`，这一支只可能在 macOS / Linux 上遇到）
    #[test]
    fn a_quote_inside_a_path_is_escaped() {
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
    }

    /// 拿不到本程序自己是 **reason**，不是一条编出来的命令
    #[test]
    fn without_an_exe_the_answer_is_a_reason_not_a_fake_command() {
        let catalog = catalog();
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let dto = post_process_dto(
            None,
            &catalog,
            internal.path(),
            user.path(),
            "A1_MINI",
            "FASTV3.3",
        )
        .unwrap();
        assert!(!dto.ready);
        assert!(dto.command.is_none() && dto.toml.is_none(), "不编路径");
        assert!(
            dto.reason.unwrap_or_default().contains("本程序自己"),
            "原因要说清是 exe 拿不到"
        );
    }

    /// **预设不在本机 ⇒ 只报原因**（把是哪一份带出来），还是不给命令 ——
    /// 这正是"按钮常驻、命令不许假"的那一半
    #[test]
    fn a_preset_that_is_not_on_disk_gets_a_reason_not_a_command() {
        let catalog = catalog();
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let exe = std::path::Path::new("dev/mkp-support-ease");
        let dto = post_process_dto(
            Some(exe),
            &catalog,
            internal.path(),
            user.path(),
            "A1_MINI",
            "FASTV3.3",
        )
        .unwrap();
        assert!(!dto.ready, "新数据根里还没有这一份");
        assert!(
            dto.command.is_none() && dto.toml.is_none(),
            "不给命令、不编路径"
        );
        let reason = dto.reason.expect("要有原因");
        assert!(
            reason.contains("A1_MINI-fastv3.3.toml"),
            "原因要把是哪一份说出来：{reason}"
        );
    }

    /// 官方那份**真在盘上** ⇒ 命令指它；`toml` 就是 catalog 落点的绝对路径
    /// （内部根 + `path`，前端不拼任何一段）
    #[test]
    fn the_command_points_at_the_real_file_on_the_internal_root() {
        let catalog = catalog();
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let exe = std::path::Path::new("dev/mkp-support-ease");

        let f = catalog
            .file_of("A1_MINI", "FASTV3.3")
            .expect("目录里该有这一份");
        let at = internal.path().join(&f.path);
        crate::fsx::atomic::atomic_write(&at, b"# machine: A1_MINI\n[toolhead]\n").unwrap();

        let dto = post_process_dto(
            Some(exe),
            &catalog,
            internal.path(),
            user.path(),
            "A1_MINI",
            "FASTV3.3",
        )
        .unwrap();
        assert!(dto.ready);
        assert!(dto.reason.is_none(), "能给了就不该有拒绝原因");
        let toml = dto.toml.expect("ready 就该有 toml");
        assert_eq!(toml, at.display().to_string());
        let command = dto.command.expect("ready 就该有命令");
        assert!(
            command.contains(&toml),
            "命令里的 --Toml 就是那一份：{command}"
        );
    }

    /// **我那一份优先**：用户副本存在时命令指它 —— 与校准页读的是同一份
    /// （匹配口径共用 `mine::user_copy_for`），官方那份在不在盘上都不换
    #[test]
    fn the_users_own_copy_wins_over_the_official_file() {
        let catalog = catalog();
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let exe = std::path::Path::new("dev/mkp-support-ease");

        /* 官方那份也在盘上：顺序不影响结果，用户操作的那份优先 */
        let f = catalog
            .file_of("A1_MINI", "FASTV3.3")
            .expect("目录里该有这一份");
        crate::fsx::atomic::atomic_write(&internal.path().join(&f.path), b"[toolhead]\n").unwrap();

        let mine = "# machine: A1_MINI\n# variant: fastv3.3\n[toolhead]\noffset_x = -0.9\n";
        let mine_at = user.path().join("presets-mine/A1_MINI-fastv3.3.toml");
        crate::fsx::atomic::atomic_write(&mine_at, mine.as_bytes()).unwrap();

        let dto = post_process_dto(
            Some(exe),
            &catalog,
            internal.path(),
            user.path(),
            "A1_MINI",
            "FASTV3.3",
        )
        .unwrap();
        assert!(dto.ready);
        assert_eq!(
            dto.toml.expect("ready 就该有 toml"),
            mine_at.display().to_string(),
            "指我那一份（presets-mine/…），不是官方交付那份"
        );
    }

    /// 目录里根本没有这个组合 ⇒ 原因说清"没有登记"，同样不给命令
    #[test]
    fn a_combo_the_catalog_does_not_know_says_so() {
        let catalog = catalog();
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let exe = std::path::Path::new("dev/mkp-support-ease");
        let dto = post_process_dto(
            Some(exe),
            &catalog,
            internal.path(),
            user.path(),
            "NOPE",
            "NOPE",
        )
        .unwrap();
        assert!(!dto.ready);
        assert!(dto.command.is_none());
        assert!(
            dto.reason.unwrap_or_default().contains("NOPE"),
            "原因要把是哪个组合带出来"
        );
    }
}
