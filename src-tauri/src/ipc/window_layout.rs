//! **主窗怎么摆**：置顶（三档）与「窄窗」（2026-10-17，作者要的"贴到切片器旁边对比"）。
//!
//! 两件事都是"窗口的动作"，不是业务数据 —— 所以副作用全部收在这一处，
//! 前端只发一条命令、只读一格账（`run/app-state.json` 的 `pin` / `compact`）。
//!
//! # 三条口径
//!
//! - **该不该真置顶是算出来的**，不落盘：`always` 恒真、`page` 只在工艺页、
//!   `off` 恒假。页签的真值在前端（`App.tsx` 的 `tab`），所以切页那一下由前端
//!   调 [`apply_pin`] —— 后端不复制一份"现在在哪一页"。
//! - **窄窗的还原点 = 用户上一次手动摆放的位置与尺寸**（`lastWindowBox`），
//!   不是"进窄窗那一刻"的几何。两者的差别只在一种情形上：用户先点了窄窗、
//!   再也没动过窗口，那时两者本来就一样。
//! - **程序自己搬窗的那一小段要静默**（[`hold_silence`]）：`set_size`/`set_position`
//!   会照常触发 `Resized`/`Moved`，不挡的话退出窄窗的中间帧会被记成"用户摆放"，
//!   下一次还原就还原到一半大 —— 这是本模块最容易出事的一处（作者口径：
//!   **判据不许两个源**，所以"是不是我们自己动的"只有这一个闸）。
//!
//! # 为什么记几何要防抖
//!
//! 拖窗 / 拉窗时 `Resized`/`Moved` 一趟几十上百条，每条都整份原子替换
//! `app-state.json` 是白白磨损。这里只做两件轻事：事件里**只记一个代数**
//! （[`touch_geometry`]，纯内存、不碰盘、不查窗口），最后一次事件之后 400ms
//! 由后台线程做判定与落盘（[`remember_manual_box`]）。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewWindow};

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;
use crate::runtime::app_state::{PinPref, WindowBox};

use super::{notify_app_state, traced};

/// 客户端主窗的 label（与 `tauri.conf.json` / `App.tsx` 同一个字面量）
const MAIN: &str = "main";

/// 窄窗的宽 = **窗口的最小宽**（`tauri.conf.json` 的 `minWidth`）。
/// 写成常量而不是"再小一点"：低于它后端会被系统钳住，目标与实际就分家了。
const COMPACT_W: f64 = 600.0;

/// 窄窗的高 = 当前屏幕**可用高度**的这一档（"拉长、显示更多参数行"）
const COMPACT_H_RATIO: f64 = 0.9;

/// 从没记过还原点时的兜底尺寸 = `tauri.conf.json` 的默认窗口
const FALLBACK_W: f64 = 900.0;
const FALLBACK_H: f64 = 640.0;

/// 程序自己搬窗之后的"静默窗口"：这段时间里来的 `Resized`/`Moved` 一律不算用户摆放。
/// 800ms 是量出来的余量 —— 一次 `set_size` + `set_position` 在 macOS / Windows
/// 都是一两帧的事，留足两倍再兜一层。
const SILENT: Duration = Duration::from_millis(800);

/// 最后一次几何事件之后等这么久再落盘（拖窗一次的中间帧不写）
const DEBOUNCE: Duration = Duration::from_millis(400);

/* ---------- 静默闸 ---------- */

fn silence() -> MutexGuard<'static, Option<Instant>> {
    static SLOT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn hold_silence() {
    *silence() = Some(Instant::now() + SILENT);
}

/// 现在是不是"我们自己正在搬窗"的那一小段
fn in_silent_window() -> bool {
    matches!(*silence(), Some(until) if Instant::now() < until)
}

/* ---------- 几何换算（只有这一处） ---------- */

fn main_window(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    app.get_webview_window(MAIN)
        .ok_or_else(|| AppError::internal("找不到主窗口（main）"))
}

/// 窗口现在的几何，**逻辑点**。宽高取 `inner`（与 [`WebviewWindow::set_size`] 同一个口径），
/// 位置取 `outer`（与 [`WebviewWindow::set_position`] 同一个口径）—— 两个平台都没有
/// 系统标题栏（macOS 是 Overlay、Windows 是 `decorations:false`），所以两者基本重合。
fn read_box(win: &WebviewWindow) -> Option<WindowBox> {
    let scale = win.scale_factor().ok().filter(|s| *s > 0.0)?;
    let pos = win.outer_position().ok()?;
    let size = win.inner_size().ok()?;
    Some(WindowBox {
        x: f64::from(pos.x) / scale,
        y: f64::from(pos.y) / scale,
        width: f64::from(size.width) / scale,
        height: f64::from(size.height) / scale,
    })
}

/// 当前窗口所在屏幕的**可用区域**（macOS 扣掉菜单栏 / Dock，Windows 扣掉任务栏），逻辑点。
fn work_area(win: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
    let mon = win.current_monitor().ok().flatten()?;
    let scale = mon.scale_factor();
    if scale <= 0.0 {
        return None;
    }
    let rect = mon.work_area();
    Some((
        f64::from(rect.position.x) / scale,
        f64::from(rect.position.y) / scale,
        f64::from(rect.size.width) / scale,
        f64::from(rect.size.height) / scale,
    ))
}

/// 窄窗的目标几何：宽 600、高 = 可用高 × 0.9、**贴右缘**、竖直居中。
fn compact_geometry(win: &WebviewWindow) -> (f64, f64, f64, f64) {
    match work_area(win) {
        Some((wx, wy, ww, wh)) => {
            let w = COMPACT_W.min(ww);
            let h = (wh * COMPACT_H_RATIO).round().max(1.0);
            let x = wx + (ww - w);
            let y = wy + ((wh - h) / 2.0).round();
            (x, y, w, h)
        }
        /* 读不到屏幕时**不退化成"什么都不做"**：挪到 600 宽也算完成了用户要的那一半，
        位置保持原样（比乱猜一个坐标诚实） */
        None => {
            let cur = read_box(win);
            let (x, y) = cur.map(|b| (b.x, b.y)).unwrap_or((0.0, 0.0));
            (x, y, COMPACT_W, FALLBACK_H)
        }
    }
}

/// 从没记过还原点时的落点：出厂尺寸、居中于当前屏
fn fallback_geometry(win: &WebviewWindow) -> (f64, f64, f64, f64) {
    match work_area(win) {
        Some((wx, wy, ww, wh)) => {
            let x = wx + ((ww - FALLBACK_W) / 2.0).round().max(0.0);
            let y = wy + ((wh - FALLBACK_H) / 2.0).round().max(0.0);
            (x, y, FALLBACK_W.min(ww), FALLBACK_H.min(wh))
        }
        None => (0.0, 0.0, FALLBACK_W, FALLBACK_H),
    }
}

/* ---------- 置顶 ---------- */

/// 按档位与"现在在不在工艺页"把窗口的置顶标志设成该有的一档。返回**实际设成了置顶吗**。
///
/// 失败只打日志：置顶失败不该让界面上的切换报错（用户看得见的是窗口层级，
/// 不是一条错误）。`off` 与 `page`（不在工艺页）都会主动 `false` ——
/// 切页那一下必须真的解除，否则"只有这一页置顶"就是句空话。
pub fn pin_window(app: &AppHandle, pref: PinPref, on_bbs: bool) -> bool {
    let on_top = match pref {
        PinPref::Always => true,
        PinPref::Page => on_bbs,
        PinPref::Off => false,
    };
    let Some(win) = app.get_webview_window(MAIN) else {
        return false;
    };
    if let Err(e) = win.set_always_on_top(on_top) {
        tracing::warn!("置顶没设成（{on_top}）：{e}");
        return false;
    }
    on_top
}

/* ---------- 手动几何：事件 → 防抖 → 落盘 ---------- */

static GEOMETRY_GEN: AtomicU64 = AtomicU64::new(0);
static GEOMETRY_TIMER: AtomicBool = AtomicBool::new(false);

/// `WindowEvent::Moved` / `Resized` 里调 —— **纯内存**（不查窗口、不读盘、不写盘）。
///
/// 判定与落盘交给后台线程：事件这一侧要保持"什么都不做"，
/// 否则拖窗时每一帧都要读一次状态文件。
pub fn touch_geometry(app: &AppHandle) {
    let gen = GEOMETRY_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    if GEOMETRY_TIMER.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut seen = gen;
        loop {
            std::thread::sleep(DEBOUNCE);
            let now = GEOMETRY_GEN.load(Ordering::SeqCst);
            if now == seen {
                break;
            }
            seen = now;
        }
        GEOMETRY_TIMER.store(false, Ordering::SeqCst);
        if let Err(e) = remember_manual_box(&app) {
            tracing::debug!("窗口几何没记下来：{e}");
        }
    });
}

/// 启动之后这一小段里来的几何事件**一律不算用户摆放**。
///
/// 为什么需要它（2026-10-17 真机量出来的）：窗口是**先被系统按配置摆出来**（900×640 放在
/// OS 挑的位置）、`setup` 才跑。那一下的 `Resized`/`Moved` 若被记成"用户摆放"，
/// 每次开机都会把用户上次亲自拉出来的尺寸与位置覆盖掉 —— 于是"恢复尺寸"永远回不到
/// 他记得的那一版。2 秒足够覆盖"建窗 → 首帧 → 摆窗"这一串（实测 < 300ms）。
const STARTUP_GRACE: Duration = Duration::from_millis(2000);

fn startup_grace() -> MutexGuard<'static, Option<Instant>> {
    static SLOT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 把"窗口现在这一版几何"记成还原点 —— 只有**真的是用户在摆放**时才记：
/// 启动宽限期（系统摆的）、静默窗口内（我们自己搬的）、窄窗态里（那是程序的尺寸）、
/// 最大化 / 全屏（那不是摆放）都跳过。
fn remember_manual_box(app: &AppHandle) -> Result<(), AppError> {
    if matches!(*startup_grace(), Some(until) if Instant::now() < until) {
        return Ok(());
    }
    if in_silent_window() {
        return Ok(());
    }
    let root = internal_root(app)?;
    if runtime::app_state::compact(&root)? {
        return Ok(());
    }
    let Some(win) = app.get_webview_window(MAIN) else {
        return Ok(());
    };
    if win.is_maximized().unwrap_or(false) || win.is_fullscreen().unwrap_or(false) {
        return Ok(());
    }
    let Some(box_now) = read_box(&win) else {
        return Ok(());
    };
    if runtime::app_state::last_window_box(&root)? == Some(box_now) {
        return Ok(());
    }
    runtime::app_state::set_last_window_box(&root, box_now)
}

/* ---------- 进 / 出窄窗 ---------- */

fn enter_compact(app: &AppHandle) -> Result<(), AppError> {
    let win = main_window(app)?;
    /* 静默闸要在**动手之前**拉起来：几何事件是紧接着这几行来的 */
    hold_silence();
    /* 最大化 / 全屏态下直接改尺寸没意义（还容易被系统摆回去）——先退出来 */
    if win.is_fullscreen().unwrap_or(false) {
        let _ = win.set_fullscreen(false);
    }
    if win.is_maximized().unwrap_or(false) {
        let _ = win.unmaximize();
    }
    let (x, y, w, h) = compact_geometry(&win);
    /*
     * ★ **先挪位置、再改尺寸**（2026-10-17 真机栽出来的顺序）：
     * 先在当前 origin 上把窗口拉大，它会顶到屏幕边缘被系统钳住（从右边贴边的窄窗
     * 直接"放大"回 1100 宽 ⇒ 实际只剩 600）。先挪到目标左上角再改尺寸，两个方向都在
     * 屏幕内，钳不住。
     */
    win.set_position(LogicalPosition::new(x, y))
        .map_err(|e| AppError::internal("窄窗位置没设成").with_detail(e.to_string()))?;
    win.set_size(LogicalSize::new(w, h))
        .map_err(|e| AppError::internal("窄窗尺寸没设成").with_detail(e.to_string()))?;
    if let Some(got) = read_box(&win) {
        if (got.width - w).abs() > 1.0 || (got.height - h).abs() > 1.0 {
            tracing::warn!(
                want_w = w,
                want_h = h,
                got_w = got.width,
                got_h = got.height,
                "窄窗没摆成目标尺寸（被系统钳住了？）"
            );
        }
    }
    tracing::info!(x, y, w, h, "进入窄窗");
    Ok(())
}

fn leave_compact(app: &AppHandle) -> Result<(), AppError> {
    let win = main_window(app)?;
    let root = internal_root(app)?;
    let (x, y, w, h) = match runtime::app_state::last_window_box(&root)? {
        Some(b) => (b.x, b.y, b.width, b.height),
        None => fallback_geometry(&win),
    };
    hold_silence();
    /* 同 `enter_compact`：**先位置、后尺寸** —— 贴着右缘的窄窗直接拉大会被屏幕钳住 */
    win.set_position(LogicalPosition::new(x, y))
        .map_err(|e| AppError::internal("还原位置没设成").with_detail(e.to_string()))?;
    win.set_size(LogicalSize::new(w, h))
        .map_err(|e| AppError::internal("还原尺寸没设成").with_detail(e.to_string()))?;
    if let Some(got) = read_box(&win) {
        if (got.width - w).abs() > 1.0 || (got.height - h).abs() > 1.0 {
            tracing::warn!(
                want_w = w,
                want_h = h,
                got_w = got.width,
                got_h = got.height,
                "还原没摆成目标尺寸（被系统钳住了？）"
            );
        }
    }
    tracing::info!(x, y, w, h, "退出窄窗");
    Ok(())
}

/// 启动时把上次的窗口摆回去（窄窗态 + 置顶档）。
///
/// ★ 置顶只按 `always` 落地：启动落在**首页**（前端 `tab` 的初值），`page` 档此刻
///   不该置顶 —— 真正的判定由前端挂载后那一次 `applyPin` 收敛（页签的真值在前端）。
/// 失败一律只告警，不挡启动。
pub fn restore_at_startup(app: &AppHandle) {
    let Ok(root) = internal_root(app) else {
        return;
    };
    /* 启动宽限期（见 [`STARTUP_GRACE`]）：从这一刻起 2 秒内来的几何事件不记账 */
    *startup_grace() = Some(Instant::now() + STARTUP_GRACE);
    let pref = runtime::app_state::pin(&root).unwrap_or(PinPref::Off);
    pin_window(app, pref, false);
    if runtime::app_state::compact(&root).unwrap_or(false) {
        if let Err(e) = enter_compact(app) {
            tracing::warn!("上次的窄窗没摆回去：{e}");
        }
    }
}

/* ---------- 命令 ---------- */

/// 窗口这一族的两格账（前端一次读齐，省一次 IPC）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPrefs {
    /// `off` / `page` / `always`（见 [`PinPref`]）
    pub pin: String,
    /// 现在是不是窄窗态
    pub compact: bool,
}

#[tauri::command]
pub async fn get_window_prefs(app: AppHandle) -> Result<WindowPrefs, AppError> {
    traced("getWindowPrefs", |_| {
        let root = internal_root(&app)?;
        Ok(WindowPrefs {
            pin: runtime::app_state::pin(&root)?.wire().to_owned(),
            compact: runtime::app_state::compact(&root)?,
        })
    })
}

/// 换置顶档位：**先落盘、再设窗口标志**，最后广播一次。
///
/// `on_bbs` 由前端给（"现在这一页是不是工艺页"）—— 只有 `page` 档用得到它。
#[tauri::command]
pub async fn set_pin(app: AppHandle, pin: String, on_bbs: bool) -> Result<String, AppError> {
    traced("setPin", |_| {
        let root = internal_root(&app)?;
        let pref = PinPref::parse(&pin).ok_or_else(|| {
            AppError::invalid_argument(format!(
                "认不出的置顶档：{pin}（只能是 off / page / always）"
            ))
        })?;
        runtime::app_state::set_pin(&root, pref)?;
        pin_window(&app, pref, on_bbs);
        notify_app_state(&app);
        Ok(pref.wire().to_owned())
    })
}

/// 切页之后重新算一次"该不该置顶"（**不写盘、不广播** —— 它只是把已经选好的档
/// 落到窗口上；档位本身没变，广播会把前端绕回这个 effect 里）。
#[tauri::command]
pub async fn apply_pin(app_handle: AppHandle, on_bbs: bool) -> Result<bool, AppError> {
    traced("applyPin", |_| {
        let root = internal_root(&app_handle)?;
        let pref = runtime::app_state::pin(&root)?;
        Ok(pin_window(&app_handle, pref, on_bbs))
    })
}

/// 进 / 出窄窗。**窄窗一开就顺手置顶**（作者 2026-10-17 第二轮：
/// 「点击窄窗的时候自动置顶吧，还原的时候置顶也还原成原本的」）——
/// 理由是它的用法本身就是"贴到切片器旁边看"，不置顶就会被切片器盖住。
///
/// **两边的落账时机刻意不一样**（2026-10-17 真机量出来的一条）：
///
/// - **进**：先把 `compact` 写 `true`、再搬窗 —— 搬窗那一下 `remember_manual_box`
///   就是靠这一格才不去记程序的尺寸；
/// - **出**：先搬窗、**最后**才把这一格写 `false` —— 反过来的话，从"写入 false"到
///   "窗口到位"之间会有一个缝，那时 `compact` 已经是 false、而窗口还停在窄窗上，
///   一个迟到的 `Resized` 就会把 600×709 记成"用户上次的摆放"，下一次还原就还原到
///   窄窗自己（真机实测栽过：`lastWindowBox` 被写成 600×534）。
///
/// 进的时候搬窗失败要把那两格回滚，出的时候搬窗失败就保持 `true` ——
/// 两种都指向同一条：盘上那几格不能与窗口的实际样子分家。
#[tauri::command]
pub async fn set_compact(app: AppHandle, compact: bool) -> Result<bool, AppError> {
    traced("setCompact", |_| {
        let root = internal_root(&app)?;
        let before = runtime::app_state::compact(&root)?;
        if before == compact {
            return Ok(compact);
        }
        if compact {
            let pin_before = runtime::app_state::pin(&root)?;
            runtime::app_state::set_pin_before_compact(&root, Some(pin_before))?;
            runtime::app_state::set_compact(&root, true)?;
            /* 替用户打开置顶（"小窗对比"时它不能被切片器盖住） */
            runtime::app_state::set_pin(&root, PinPref::Always)?;
            pin_window(&app, PinPref::Always, true);
            if let Err(e) = enter_compact(&app) {
                let _ = runtime::app_state::set_compact(&root, before);
                let _ = runtime::app_state::set_pin(&root, pin_before);
                let _ = runtime::app_state::set_pin_before_compact(&root, None);
                pin_window(&app, pin_before, true);
                return Err(e);
            }
        } else {
            if let Err(e) = leave_compact(&app) {
                /* 半路失败：尽量退回窄窗，让窗口与盘上那一格（还是 true）一致 */
                if let Err(again) = enter_compact(&app) {
                    tracing::warn!("还原失败之后也没退回窄窗：{again}");
                }
                return Err(e);
            }
            runtime::app_state::set_compact(&root, false)?;
            /*
             * 置顶也还原成原本那一档。**只在"还停在我们替他设的 always"时才回**：
             * 窄窗期间用户自己去点过置顶，那就是他的选择，不该被这一下盖掉。
             */
            if runtime::app_state::pin(&root)? == PinPref::Always {
                if let Some(prev) = runtime::app_state::pin_before_compact(&root)? {
                    runtime::app_state::set_pin(&root, prev)?;
                    pin_window(&app, prev, true);
                }
            }
            runtime::app_state::set_pin_before_compact(&root, None)?;
        }
        notify_app_state(&app);
        Ok(compact)
    })
}
