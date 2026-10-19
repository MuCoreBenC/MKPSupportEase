//! **屏幕引导的那几条命令**（2026-10-15）：保存了参数之后，把"回切片器重切一次"那一步
//! **指给用户看**。
//!
//! # 为什么要有它
//!
//! 改了 MKP 参数（XYZ 偏移、涂胶…）之后，切片器**不知道**：它那一份参数里根本没有
//! 这些字段（见 `docs/INVESTIGATION-BBS-INVALIDATE.md` §4）。于是它仍认为"这一盘是
//! 切好的"，切片按钮不给点 —— 用户只能自己改点什么（现在最常做的是改一下板子类型）
//! 才切得动。
//!
//! 那条链我们**改不了切片器**：没有外部入口（那份文档 §5/§7），运行时注入又被
//! entitlement 封死（§6）。能做的是什么都不替用户做、只**把那一步指给他看**：
//! 在他自己的切片器窗口旁边放一张小卡片。动手的是他自己。
//!
//! # 卡片的三条活法（2026-10-15 作者定）
//!
//! | 什么时候 | 卡片 |
//! | --- | --- |
//! | 切片器在最前 | 露脸，贴着「板子类型」控件 |
//! | 切片器被别的软件压住 / 最小化 | **收起来**（用户这会儿看不到那个控件，留着就是指着空气） |
//! | 用户拖动了切片器窗口 | **跟过去**（重读坐标、挪卡片） |
//!
//! 这三件事靠 macOS 的 **AX 通知**驱动（[`crate::ax_watch`]），**不是定时器** ——
//! 作者原话"不要频繁的轮询"。通知只负责说"变了"，收到之后这里重读一次坐标。
//!
//! # 边界（这一层自己守着）
//!
//! - **只读**：只问坐标，没有任何 AX 写动作、不模拟点击、不碰别的进程的内存；
//! - **不改切片器任何参数**；
//! - **不给假坐标**：指不了就 `reason` 一句话，界面因此只显示原因、不画箭头；
//! - 卡片只占一小块（**不铺满屏幕**）：它活着的时候，用户照样能去点切片器。
//!
//! # 坐标系
//!
//! 实测：AX 给的 `AXPosition` 与 Tauri 的窗口位置是**同一套**屏幕坐标（原点左上）。
//! 判据是窗口右上角那个下拉箭头 —— 它报的坐标落在窗口顶部那一带，而不是屏幕底部。
//! 所以这里**不做上下翻转**，直接把 AX 的读数交给窗口定位。

use tauri::{AppHandle, Manager as _, WebviewUrl, WebviewWindowBuilder};

use crate::ax::{self, GuideTargetDto, RectDto, SlicerVisibility};
use crate::error::AppError;
/* "被盖住多少才算看不见"是全平台一个数，真值住在 [`crate::zorder`]（Windows 的
`ax::win::state` 也比它），这里只引用 —— 改一处就够。 */
use crate::zorder::COVERED_ENOUGH;

use super::traced;

/// 引导窗口的标签（与客户端 `main` 分开，`get_webview_window` 拿的是各自那一个）
const GUIDE_LABEL: &str = "guide";

/* ---------- 窗口几何：卡片 + 一条箭头竖条 + 四周给阴影的空 ---------- */

/// 引导卡片的尺寸（逻辑点）。**刻意做小**：它只该盖住自己那一块，
/// 用户这会儿正要回头去点切片器，铺满屏幕等于把人挡在外面。
///
/// ★ **`CARD_H` 要比"文案排版出来需要的高度"多留一点**：
///
/// - 卡片是 `top/bottom: SHADOW_PAD`，跟着窗口长 ⇒ 多出来的高度是**卡片里的空白**
///   （按钮被 `flex:1` 顶到最下边），不是一块透明的"死区"，看着不难看；
/// - 但**透明窗不是点击穿透的**，多高一寸就多挡一寸别人的点击 ⇒ 也别乱给。
///
/// 实测（Chromium，340 宽）：文案排出来 **179**（副标题 16 + 两段正文 39 + 59 +
/// 段间距 12 + 内边距 28 + 按钮 28 + 卡片竖排间距 10）；真机是 WKWebView，
/// 断行可能差一行 ⇒ 再留一行（≈19）⇒ **198**。★ 改文案要回头复核这个数。
const CARD_W: f64 = 340.0;
const CARD_H: f64 = 198.0;

/// 卡片与目标控件之间留的空。
///
/// ★★ 为什么它必须 **≥ [`ARROW_PAD`]**：窗口左边 = 控件右边（= `target + GAP`）
/// 再往左让一条箭头竖条。`CARD_GAP < ARROW_PAD` 时窗口就会**压到控件身上** ——
/// 那一块是透明的、但**照样吃点击**，用户点不到他要点的那个控件。
/// 现在 22 vs 20 ⇒ 窗口左边界落在控件右边**之外** 2px，谁也不挡。
const CARD_GAP: f64 = 22.0;

/// **卡片四周留给阴影的空**（逻辑点）。
///
/// ★★ 为什么要有它：立体感是 CSS 画的（`.root` 的 `filter: drop-shadow`），而
/// **窗口边界会把它切掉** —— 卡片贴着窗口边时那圈影子正好少一半，作者截图里
/// 圈出来的就是这几处（"阴影被窗口截断了"）。
/// 阴影是 `0 2px 6px`，往外约 9px ⇒ 给到 16 才能在窗口内淡完。
const SHADOW_PAD: f64 = 16.0;

/// **箭头那一条的宽**（窗口内、卡片外侧的那一竖条）。
///
/// ★ 为什么窗口要比卡片宽这么多：箭头**必须画在窗口里面** —— 透明窗口**画不出
/// 边界之外的东西**（第一版把三角摆在 `left: -12px`，结果整条箭头直接消失）。
/// 于是给它留一竖条，再把窗口整体朝目标挪这么多：视觉上箭头仍是"从卡片伸出去"，
/// 实际上它老老实实待在窗口内。
///
/// 20 = 半个方块（12×12 转 45° 的半对角线 ≈ 8.5）+ 箭尖外那圈阴影（往外 ≈ 9）+ 余量。
const ARROW_PAD: f64 = 20.0;

/// 窗口尺寸 = 卡片 + 两侧那条竖条 + 四周的阴影空（与 `GuideCard.module.css` 对齐）。
///
/// ★ 左边一律按 [`ARROW_PAD`] 算、右边一律按 [`SHADOW_PAD`] 算 —— 两档 `side`
/// 只是**卡片在窗口里的左/右不同**，窗口本身**同样大**（不必随 side 变尺寸）。
const WINDOW_W: f64 = ARROW_PAD + CARD_W + SHADOW_PAD;
const WINDOW_H: f64 = SHADOW_PAD * 2.0 + CARD_H;

/* ---------- 命令 ---------- */

/// 「板子类型」控件此刻在屏幕上的哪一块（**只读**）。
///
/// 前端只在用户点了「查看引导」之后调它 —— 不轮询、不在后台偷偷读别人的窗口。
#[tauri::command]
pub async fn locate_bed_type_control() -> Result<GuideTargetDto, AppError> {
    traced("locateBedTypeControl", |_| Ok(ax::locate_bed_type()))
}

/// 把引导卡片摆出去（贴着「板子类型」那个控件），顺带把坐标回给前端。
///
/// **只有一步**（2026-10-15 作者定）：卡片上一次说完"换成另一个、再换回原来那个"
/// 那件事就够。原来那个"下一步"是多余的 —— 起作用的是"换成不同值"那一次
/// （源码里那个 `if (old != new)`），换回来只是还原，**那件事用户自己记得住**；
/// 而换步要关掉窗口重建，中间那一瞬什么都没了，比不引导还差。
///
/// 已经开着就**先强制收掉再重建**（落点可能变了）。★ 见 [`destroy_guide`]：
/// 收旧的必须用 `destroy()`，而且**建新的也必须排在它后面**（同一个坑的另一半）。
#[tauri::command]
pub async fn guide_show(app: AppHandle) -> Result<GuideTargetDto, AppError> {
    traced("guideShow", |_| {
        /*
         * ★★★ **先把切片器抬起来，再建卡片**（2026-10-18 作者报"MKP 在桌面2，
         * 就找不到桌面1的 BBS"）：
         *
         * 卡片这扇窗**建在哪个桌面，跟着"建它那一刻哪个桌面在最前"走** ——
         * 旧顺序是"先建卡片（落在 MKP 所在的桌面2）→ 再抬切片器（系统切去
         * BBS 那个桌面1）"，结果人被带过去了、卡片留在桌面2，两边都见不着。
         *
         * 反过来：**先 `AXRaise` + `AXFrontmost` 把 BBS 抬到最前**（macOS 顺着
         * 切到 BBS 那个桌面），**等切换落定**（[`SPACE_SETTLE`]，盖过切空间的
         * 动画）**再定位、建卡片** —— 卡片就建在 BBS 旁边那个桌面，人也在那儿。
         *
         * ★ 定位仍读全局坐标（AX 坐标不分桌面），这里等一下主要是给"建窗"
         * 一个已经落定的空间，别让卡片建在切换动画的半路上。
         */
        raise_slicer_window();
        std::thread::sleep(SPACE_SETTLE);

        let target = ax::locate_bed_type();
        let Some(t) = target.target.filter(|_| target.ready) else {
            /* 指不了：原样把 reason 递上去，界面显示原因、不画箭头 */
            return Ok(target);
        };
        /* 这一趟刚读到的坐标顺手种进缓存：紧接着那一跳就能免 AX（见 [`LOCATE_TTL`]） */
        seed_locate(&target);

        /*
         * ★★ **抬切片器这件事必须发生在"建卡片"之后**（2026-10-15 作者报的
         * "我的软件会覆盖，完全挡住了"）：建一扇新窗会把**我们这个 App 激活**，
         * 于是我们的主窗压到切片器前面去了。所以顺序是
         * **先建卡片 → 再把切片器抬回最前**（卡片自己是 `always_on_top`，
         * 抬完仍浮在切片器上面）。
         */
        let (x, y, side) = place_card(&t, screen_width(&app));
        show_at(&app, x, y, side);
        mount_watcher(&app);

        tracing::info!(card_x = x, card_y = y, side, "引导卡片已摆好");
        Ok(target)
    })
}

/// 收起引导卡片。**没开着也是成功** —— 这是一句"我现在不需要它了"，不是"去关一个
/// 一定存在的窗口"。
#[tauri::command]
pub async fn guide_close(app: AppHandle) -> Result<(), AppError> {
    traced("guideClose", |_| {
        /* 卡片没了就不用再听切片器的窗口动静 */
        unmount_watcher(&app);
        destroy_guide(&app);
        Ok(())
    })
}

/* ---------- 跟随（AX 通知的落点） ---------- */

/// 通知来了之后重算一次：该露脸就摆到新坐标，不该露脸就收起来。
///
/// ★ 这是 [`crate::ax_watch`] 的 `OnChange` 落点，**在主线程上**被叫起来 ——
/// 所以里面可以直接碰窗口、也可以放心读 AX 与起进程。
///
/// ★★ **可见性只在这里读一处**（从 AX 通知的载荷里读不到它，而"读"要动对方进程、
/// 还要 `pgrep`，那不该发生在 AX 回调那个线程上）。
///
/// - `Behind` / `Gone`：收掉卡片（切片器被压住、最小化、或者退了）；
/// - `Front`：**没开着就开**（用户把切片器切回最前了，卡片该回来）、
///   开着就重读坐标挪过去（拖动了 / 拉大了）；坐标读不到也收起来。
pub fn sync(app: &AppHandle) {
    let Some(action) = decide(app) else {
        return;
    };
    /* 只有这一段要求主线程（见 `apply`） */
    let inner = app.clone();
    let _ = app.run_on_main_thread(move || apply(&inner, action));
}

/// 「该干什么」——`decide` 的产物，交给主线程去执行（见 [`apply`]）。
enum Action {
    /// 收起来；`gone` = 切片器已经退了（顺手卸监听）
    Hide { gone: bool },
    /// 摆出去（旧的先收掉）
    Show { x: f64, y: f64, side: &'static str },
    /// 已经在开着，只是挪个位置（拖动 / 缩放跟随）
    Move { x: f64, y: f64, side: &'static str },
}

/// **第一段：读 AX、拿主意。**
///
/// ★★★ **绝不许在主线程上调**（2026-10-15 作者报"一直转圈、鼠标不变手"）：
/// 这里每一次 AX 调用都是**同步 IPC 打到切片器那个进程**去（读窗口矩形、扫控件树
/// 几十次）。切片器一忙（正在切片 / 自绘重绘），这些调用会**卡住好几秒**
/// （AX 的默认消息超时）—— 落在主线程上就是**整屏转圈 + 光标不更新 + 悬停提示出不来**。
///
/// AX 读**不要求**主线程，而"动窗口"才要求；所以两段分开，这一段谁来叫就在谁的线程上跑。
fn decide(app: &AppHandle) -> Option<Action> {
    /* pid 直接问 watcher 要（它装着的时候就记着）—— 别在这儿起 `pgrep` */
    let pid = watched_pid()?;
    let z = crate::zorder::slicer_window(pid);
    /*
     * ★★ **CG 答得上来就不去问切片器**（作者："怎么样节省就怎么样做吧"）。
     * AX 的每一次读都是**同步 IPC 打到切片器进程**（它一忙就卡好几秒），
     * 而 CG 那次**全在本地**、顺带把矩形也给了。所以：
     * `st` 只在 **CG 问不到**（系统不给窗口列表）时才读，用来兜底 + 当键。
     */
    let st = match z {
        Some(_) => None,
        None => Some(ax::slicer_state(pid)),
    };
    /*
     * ★★★ **"被谁盖住了"问系统的层序，不问 AX 的焦点**（2026-10-15 作者：
     * "其他软件盖住了 BBS，它还是这个还是一样的显示"）。
     *
     * AX 的 `AXFocusedApplication` 答的是**焦点**：一个窗口压上来、焦点没变时它
     * 一点反应都没有。而 [`crate::zorder`] 问的是 CoreGraphics 的窗口列表
     * ——**按层序从前往后排**，那才是"谁盖住谁"。
     * ★ 而且它**全在本地**（窗口服务器），不打给切片器进程；顺带把"最小化 / 隐藏"
     * 也一起答了，还把窗口矩形带出来（当"变没变"的键用）。
     */
    /*
     * ★★★ **便宜的闸门**（2026-10-15 作者："拖动了它没有跟随"）。
     *
     * 这条既由 AX 通知驱动、又由 [`crate::ax_watch`] 的细定时回读兜底（切片器是 Qt
     * 应用，拖动过程中**不发** `AXWindowMoved`）。既然会被叫得很勤，第一件事就是
     * 比键：**没变就一丁点多余的活都不干**（重扫控件树是几十次 AX 调用）。
     *
     * 一次比较罩住这几类变化：**窗口矩形**（拖动 / 缩放）、**被盖住 / 最小化**
     * （层序）、**最前是谁**（CG 问不到时那份 AX 兜底）。
     *
     * ★★ 但**只有这个键还不够**：卡片指的控件会在"窗口一动不动"时消失（用户切页）
     * —— 那时上面这几项**一样都不变**。所以闸门里还挂了一条 [`locate_is_fresh`]：
     * 定位缓存过期就**不许早退**，必须真去看一眼（见 [`LOCATE_TTL`]）。
     */
    /*
     * ★★★ **什么时候"必须去看一眼"**：
     *
     * - 卡片**开着**：要跟位置、也要确认它还该在（切页了就收）；
     * - ★ 或者**上一个控件找不到了**（[`TARGET_LOST`]）—— 那是在等它**回来**；
     *   ★★ 这一条不能省（我第一版省掉了，于是作者报"切换回来它就没出现"）：
     *   "控件没了"是**唯一没有信号**的隐藏理由 —— 被压住 / 最小化 / 拖动都会让上面
     *   那个键变，唯独"窗口内部的界面变了"什么信号都没有，只能靠定期回看。
     *
     * 两种情况都**还要**"刚刚没看过"（[`attempt_is_fresh`]）才真的去看，
     * 免得一跳一次地重扫控件树。
     */
    let card_open = app.get_webview_window(GUIDE_LABEL).is_some();
    let must_look = (card_open || target_lost()) && !attempt_is_fresh();
    if !changed(z, st) && !must_look {
        return None;
    }

    let visibility = match z {
        /* 屏上没有它 = 最小化 / 隐藏 → 收 */
        Some(z) if !z.on_screen => SlicerVisibility::Behind,
        /* 被别的 App 的普通窗盖住了 → 收（[`COVERED_ENOUGH`] 以下的小浮窗不算） */
        Some(z) if z.covered >= COVERED_ENOUGH => SlicerVisibility::Behind,
        /* 问到层序且它清白在最前 → 露脸 */
        Some(_) => SlicerVisibility::Front,
        /*
         * 问不到窗口列表（系统不给）→ 退回 AX 那套"最前是谁"的口径。
         * ★ 那边也是问不到时**当作露脸**（fail-open）：这是一张"指路"的卡，
         * 判断不了就别让它凭空消失。
         */
        None => st.map_or(SlicerVisibility::Front, |s| s.visibility),
    };
    if visibility == SlicerVisibility::Behind && just_shown() {
        /*
         * ★★★ **刚摆出去的这一小段里，"切片器不在最前"多半不是用户干的，是我们的回声**：
         * 建卡片那一下会把我们这个 App 激活 ⇒ 切片器（暂时）退到后面 ⇒ 我们自己的
         * 通知回过来判"该收起来" ⇒ **刚建出来的新卡片当场被收掉**。
         * 作者看到的就是"根本没出现新的"（而且旧的那张也被我们收掉了，两条一起没）。
         *
         * 所以这段时间**先不动它**：等 [`SETTLE`] 落定后那条**一次性回读**再判。
         * ★ 只挡 `Behind`，`Gone`（切片器退了）照收不误。
         */
        return None;
    }
    if visibility != SlicerVisibility::Front {
        /*
         * ★★★ **卡片收起来，但监听要留着**（2026-10-15 作者报"BBS 最小化了、不置顶了
         * 也没变化"——而且是"再也没有变化"）。
         *
         * 上一版这里顺手 `unmount_watcher()` 了，于是：最小化 ⇒ 收卡片 + 卸监听 ⇒
         * **还原时没人通知**（监听正被我们发现"用户看不见了"而卸掉的）⇒ 卡片永远
         * 回不来，之后拖动也不再跟随 —— 一次最小化就把整条跟随线掐死了。
         *
         * 监听的本钱只是一个 run loop source，留着它才收得到
         * `AXWindowDeminiaturized` / `AXFocusedWindowChanged`。真正该卸的是
         * **卡片被关掉**那两条路（`guide_close` 与切片器退了 ⇒ `Gone`）。
         */
        /* 收起来是因为"看不见它"（被压住 / 最小化 / 退了）—— 这类理由**都有信号**，
        键一变就会再来一次，所以这里把"控件找不到"那个标记清掉，省下定期回看 */
        mark_target_lost(false);
        return Some(Action::Hide {
            gone: visibility == SlicerVisibility::Gone,
        });
    }

    let Some(t) = target_rect(z.and_then(|z| z.rect)) else {
        /*
         * 在最前、但指不了（界面换了 / 还没读到）—— **不摆一个指错地方的箭头**。
         * ★ 这里同样**不卸监听**：读不到多半是切片器正在启动 / 换版，
         * 它自己会再发通知过来，留着才有下一次机会。
         * ★★ 但要**记下"控件没了"**（[`TARGET_LOST`]）：这是唯一没有信号的隐藏理由，
         * 不记就再也没人把它接回来（作者："切换回来，它就没出现"）。
         */
        mark_target_lost(true);
        return Some(Action::Hide { gone: false });
    };
    /* 又指得上了 —— 标记清掉 */
    mark_target_lost(false);

    /* 通知没挂满 ⇒ 趁它现在看得见，重挂一次（幂等，且带节流） */
    ensure_watcher(app);

    let (x, y, side) = place_card(&t, screen_width(app));
    if app.get_webview_window(GUIDE_LABEL).is_some() {
        Some(Action::Move { x, y, side })
    } else {
        Some(Action::Show { x, y, side })
    }
}

/// 上一次定位成功时那一对：**（切片器窗口矩形，目标控件矩形，读到的时刻）**。
///
/// ★★ 拿它**直接算**拖动后的新坐标（见 [`target_rect`]）—— 这就是"再省一点"的那一刀。
static LAST_LOCATE: std::sync::Mutex<Option<(ax::RectDto, ax::RectDto, std::time::Instant)>> =
    std::sync::Mutex::new(None);

/// 缓存那一对定位结果的**寿命**。
///
/// ★★★ 为什么必须有它（2026-10-15 作者报的那一串现象）：
///
/// **卡片指的那个控件会在"窗口一动不动"的情况下消失** —— 用户把切片器切到「设备」页，
/// 窗口没挪、没缩、焦点没变、也没最小化 ⇒ **我们一条信号都收不到**。
/// 而"窗口只平移就免 AX"那条缓存要是无限期有效，就**连"重新看一眼"的机会都没有**，
/// 于是卡片一直指着空气。
///
/// 作者观察到的正是这个的指纹：**改一下窗口尺寸就不显示了**（尺寸变了 ⇒ 绕开缓存 ⇒
/// 真读一次 AX ⇒ 发现控件不在 ⇒ 收起来），**尺寸改回去就又显示**（又命中缓存 ⇒ 老坐标）。
///
/// ⇒ 缓存只在这段时间内有效：拖动那几跳照样免 AX（**省**），过期就真读一次（**准**）。
/// 作者："那个卡片多等一会再跟上也没关系" —— 所以这里可以放心给到 2 秒。
const LOCATE_TTL: std::time::Duration = std::time::Duration::from_secs(2);

/// 目标控件这会儿在哪。
///
/// ★★★ **窗口只在平移时，一个 AX 调用都不发**（作者："怎么样节省就怎么样做吧，
/// 再节约一点都可以"）：控件在切片器窗口里的**相对位置是固定的**，窗口挪多少它就
/// 跟着挪多少 —— 所以"上次那一对"加上窗口位移就是新位置，纯算术。
///
/// ★ 什么时候才去读 AX（并把这一对重新记下）：
/// - 还没有缓存（这一趟第一次）；
/// - 缓存**过期**了（[`LOCATE_TTL`]）—— 这一条是"切页了"唯一的发现途径；
/// - **窗口尺寸变了**（拉大缩小 / 换布局）：相对位置可能跟着变，不能外推。
///
/// ★ 这条依赖"相对位置稳定"，而它正是 [`ax::locate_bed_type`] 挑控件的判据
/// （按**相对窗口**的 y 落区间挑），所以两边是同一个假设，不会打架。
fn target_rect(win_now: Option<ax::RectDto>) -> Option<ax::RectDto> {
    if let (Some(now), Some((old_win, old_target, at))) = (win_now, *lock_locate()) {
        let same_size = now.width == old_win.width && now.height == old_win.height;
        if same_size && at.elapsed() < LOCATE_TTL {
            return Some(ax::RectDto {
                x: old_target.x + (now.x - old_win.x),
                y: old_target.y + (now.y - old_win.y),
                width: old_target.width,
                height: old_target.height,
            });
        }
    }

    /* 老实读一次 AX，并把这一对记下来（下次平移就能免了） */
    stamp_attempt();
    let found = ax::locate_bed_type();
    let target = found.target.filter(|_| found.ready)?;
    if let Some(win) = win_now.or(found.window) {
        if let Ok(mut g) = LAST_LOCATE.lock() {
            *g = Some((win, target, std::time::Instant::now()));
        }
    }
    Some(target)
}

/// 把**刚读到**的那一对记进缓存（`guide_show` 那次定位也顺手种上，
/// 于是紧接着那一跳就能免 AX）。
fn seed_locate(found: &ax::GuideTargetDto) {
    let (Some(win), Some(target)) = (found.window, found.target) else {
        return;
    };
    if let Ok(mut g) = LAST_LOCATE.lock() {
        *g = Some((win, target, std::time::Instant::now()));
    }
    stamp_attempt();
}

/// **指的那个控件找不到了**（用户切到别的页 / 切片器界面换过）。
///
/// ★★★ 为什么单独记这一个状态（2026-10-15 作者："切换到别的页面，它确实消失了；
/// 但我**切换回来，它就没出现**"）：
///
/// 隐藏卡片有好几个理由，**只有这一个是"没有信号"的** ——
///
/// | 为什么收起来 | 怎么知道它该回来了 |
/// | --- | --- |
/// | 被别的 App 盖住 | 层序变了 ⇒ **键变** ⇒ 立刻重算 |
/// | 最小化 / 隐藏 | 窗口从屏上消失 ⇒ **键变** ⇒ 同上 |
/// | 切片器退了 | pid 没了 ⇒ **键变** ⇒ 同上 |
/// | ★ **控件没了（切页）** | **什么都不会变** —— 窗口没挪没缩、焦点没变 |
///
/// 所以这一档必须留个标记，靠 [`decide`] 里那条"定期去看一眼"把它接回来；
/// 不记就永远接不回来（第一版把它连同定期校验一起省掉了，就是这个后果）。
static TARGET_LOST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 记下 / 清掉"控件找不到"。
fn mark_target_lost(lost: bool) {
    TARGET_LOST.store(lost, std::sync::atomic::Ordering::Relaxed);
}

/// 上一条见 [`TARGET_LOST`]。
fn target_lost() -> bool {
    TARGET_LOST.load(std::sync::atomic::Ordering::Relaxed)
}

/// 上一次**尝试**定位是哪一刻（**成没成都算**）。
///
/// ★★ 为什么要跟"读到了"分开记：控件找不到时（用户切到别的页）我们会把卡片收掉，
/// 而那个状态下缓存**永远是过期的**（只有成功才写）⇒ 闸门每 700ms 放一次行、
/// 每 700ms 重扫一遍控件树 —— 那正是"频繁的轮询"。
/// 记下"刚才试过了"，**失败也等一个 [`LOCATE_TTL`] 再试**。
static LAST_ATTEMPT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// 记下"刚刚尝试过定位"。
fn stamp_attempt() {
    if let Ok(mut g) = LAST_ATTEMPT.lock() {
        *g = Some(std::time::Instant::now());
    }
}

/// 刚刚看过一眼了吗（**成没成都算**）—— 见 [`LAST_ATTEMPT`]。
///
/// ★ 它是闸门的一部分：见 [`decide`] —— 还没看过就**不许**早退，否则"切页了"
/// 永远等不到那次真实读取。
fn attempt_is_fresh() -> bool {
    let fresh = |g: &Option<std::time::Instant>| g.is_some_and(|t| t.elapsed() < LOCATE_TTL);
    match LAST_ATTEMPT.lock() {
        Ok(g) => fresh(&g),
        Err(e) => fresh(&e.into_inner()),
    }
}

/// 读缓存（锁坏了就当作没有 —— 那只是多读一次 AX，不影响对不对）。
fn lock_locate(
) -> std::sync::MutexGuard<'static, Option<(ax::RectDto, ax::RectDto, std::time::Instant)>> {
    match LAST_LOCATE.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// **第二段：动窗口。** 只该在主线程上执行 —— 这几句是唯一要求主线程的部分。
///
/// ★ 这里**不做任何 AX 读**（那会卡住主线程，见 [`decide`]）。
fn apply(app: &AppHandle, action: Action) {
    match action {
        Action::Hide { gone } => {
            if gone {
                /* 切片器整个没了：再留着监听也没意义（pid 都失效了） */
                unmount_watcher(app);
            }
            destroy_guide(app);
        }
        Action::Show { x, y, side } => show_at(app, x, y, side),
        Action::Move { x, y, side } => move_card(app, x, y, side),
    }
}

/// 已经开着 ⇒ **只挪窗口**，不重建。
///
/// 拖动是一串密集的回读，重建的话每一帧都在闪 —— 作者要的是"跟过去"，
/// 不是"关掉再冒出来"（那正是他说过的"只是消失了没出现新的"那种观感）。
fn move_card(app: &AppHandle, x: f64, y: f64, side: &'static str) {
    let Some(win) = app.get_webview_window(GUIDE_LABEL) else {
        return;
    };
    let _ = win.set_position(tauri::LogicalPosition::new(x, y));
    /*
     * 左右翻边（拖到屏幕另一头去了）要重画箭头：前端从 URL 的 `side` 读它，
     * 而窗口已经建好了 —— 所以这里**推一条事件**过去，别重建窗口。
     */
    use tauri::Emitter as _;
    let _ = win.emit(GUIDE_SIDE_EVENT, side);
}

/// 通知没挂满就重挂一次（幂等）。
///
/// ★★ **别在这儿比"窗口身份"**（我上一版比的是 AX 元素指针，那是错的方向）：
/// AXUIElement 是**每次读都新给一个对象**，同一个窗口两次读到的指针**不保证相等**
/// ⇒ 一比就判"换了窗"，于是**每来一条通知/每一跳回读都重挂一次监听**（churn），
/// 而重挂期间正好收不到通知。删掉它。
///
/// ★ 那"切片器换了一扇窗、旧窗那四条再也不响"怎么办？——**不怕**：
/// 位置跟随现在由 [`crate::ax_watch`] 的细定时回读兜底（它不看窗口身份，每跳都重新
/// 读坐标）；最小化/被压住由 `ax::slicer_state` 每次现读 `AXMinimized` 与"最前是谁"
/// 判 —— 都不依赖"当初挂在哪扇窗上"。
fn ensure_watcher(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        /*
         * ★★ **带节流**：重挂要主线程去 `AXObserverCreate` / `AddNotification`，
         * 而后者也是"打给切片器进程"的同步 IPC。要是某一趟没挂满（比如切片器还没开
         * 工程窗口），那 `is_complete()` 会一直是 false ⇒ 每一跳回读（350ms）都重挂一次，
         * 主线程被这些 IPC 反复按住。所以这里**最多每 [`REMOUNT_FLOOR`] 试一次**。
         */
        if crate::ax_watch::is_complete() || !remount_due() {
            return;
        }
        tracing::info!("AX 通知没挂满 —— 重挂一次");
        crate::ax_watch::start(app, sync);
    }
    #[cfg(target_os = "windows")]
    {
        /* Windows 那边没有"挂满没挂满"：watcher 活着就够，死了按同一个节流补装 */
        if crate::win_watch::is_complete() || !remount_due() {
            return;
        }
        tracing::info!("跟随回读没在跑 —— 补装一次");
        crate::win_watch::start(app, sync);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
    }
}

/// 两次重挂之间**至少**隔这么久（见 [`ensure_watcher`] 的注释）。
const REMOUNT_FLOOR: std::time::Duration = std::time::Duration::from_secs(3);

/// 上一次尝试重挂是什么时候。
static LAST_REMOUNT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// 现在允许重挂吗（顺手记下这一次的时间）。
fn remount_due() -> bool {
    let Ok(mut g) = LAST_REMOUNT.lock() else {
        return false;
    };
    if g.is_some_and(|t| t.elapsed() < REMOUNT_FLOOR) {
        return false;
    }
    *g = Some(std::time::Instant::now());
    true
}

/// 左右翻边时推给卡片的那条事件（载荷 = `"left"` / `"right"`）。
///
/// ★ 为什么不重建窗口：拖动是一串密集通知，重建会一闪一闪。挪窗口 + 推事件
/// 才是"跟过去"该有的观感。
pub const GUIDE_SIDE_EVENT: &str = "guide-side";

/// 把卡片摆到 `(x, y)`（窗口坐标），`side` 决定箭头指哪边。
///
/// ★★ **建窗要排在 Destroy 后面** —— 这是"消失了没出现新的、要再点一次"的另一半：
/// `destroy()` 只是把一条 Destroy **排进主线程队列**，而 `WebviewWindowBuilder::build()`
/// 在**当前线程同步建**。两者不同步 ⇒ build 执行时旧窗还在管理器里 ⇒ 直接失败
/// （“a webview with label `guide` already exists”），界面上就是"旧卡没了、新卡没来"。
///
/// 所以收与建**都要排进同一条队列**：`destroy_guide` 之后用
/// `app.run_on_main_thread(|| build)` —— Destroy 先、Create 后，先进先出。
fn show_at(app: &AppHandle, x: f64, y: f64, side: &'static str) {
    destroy_guide(app);
    mark_shown();
    let build = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        /* 排到这儿时前面的 Destroy 已经执行完了，label 空着 */
        if let Err(e) = build_card(&build, x, y, side) {
            tracing::warn!(detail = %e.message, "引导卡片没建出来");
        }
        /*
         * ★★ **建完再把切片器抬回最前**（顺序不能颠倒，理由见 `guide_show`）：
         * 建窗那一下把我们 App 激活了，切片器被压到后面 —— 这里把它抬回来。
         * ★★ 桌面（Space）同理（2026-10-18）：建窗那一下的激活可能把系统拽回
         * **我们窗口所在的那个桌面**，这一下把人带回 BBS 那个桌面 —— 卡片就建在那儿。
         * ★ 抬**两次**：`build()` 返回时那次激活可能还没落定（它是异步的），
         * 隔一个事件循环再补一次。两次都是**一次性**的，不是定时器。
         */
        raise_slicer_window();
        let _ = build.clone().run_on_main_thread(raise_slicer_window);
    });

    /*
     * **落定之后回读一次**（一次性，不是轮询）：这时"建窗激活我们"的回声已经过去，
     * 真被别的软件压住就收掉；一切正常就留着。
     */
    let settle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(SETTLE);
        let inner = settle.clone();
        let _ = settle.run_on_main_thread(move || sync(&inner));
    });
}

/// 建窗之后"等一下再判"的那段时间（见 [`just_shown`]）。
///
/// ★ 这是**防抖**，不是轮询：只在卡片刚摆出去时用一次。
const SETTLE: std::time::Duration = std::time::Duration::from_millis(700);

/// 抬起切片器之后、"建卡片之前"等的那段时间（见 [`guide_show`]）。
///
/// ★ 一次性的等待，不是轮询：macOS 切桌面（Space）的动画约半秒，等它走完再建窗，
/// 卡片才稳稳落在 BBS 那个桌面上。
const SPACE_SETTLE: std::time::Duration = std::time::Duration::from_millis(900);

/// 上一次见到的切片器状态（"变没变"就比它）。
///
/// ★ 记的是**读到的那一刻**，不是"我们摆到哪" —— 摆的动作可能失败、也可能被系统调过。
static LAST_STATE: std::sync::Mutex<Option<WatchKey>> = std::sync::Mutex::new(None);

/// 一次读到的**全部**判据（这就是那个"变没变"的键）。
#[derive(PartialEq)]
struct WatchKey {
    /// 层序处境（`None` = 问不到窗口列表）
    z: Option<crate::zorder::SlicerWindow>,
    /// CG 问不到时才读的那份 AX 兜底（`None` = 用不上）
    ax: Option<ax::SlicerState>,
}

/// 拿到的判据跟上一次**不一样**吗？（一样 = 无事可做）
///
/// 不一样就顺手把它记下来 —— 所以这是**有副作用的比较**，名字取成 `changed`
/// 而不是 `is_same`，好让人一眼看出它会写状态。
fn changed(z: Option<crate::zorder::SlicerWindow>, ax: Option<ax::SlicerState>) -> bool {
    let now = WatchKey { z, ax };
    let Ok(mut g) = LAST_STATE.lock() else {
        /* 锁坏了：当作"变了"，宁可多做一次也别卡住 */
        return true;
    };
    if g.as_ref() == Some(&now) {
        return false;
    }
    *g = Some(now);
    true
}

/// 卡片是**什么时候**摆出去的（`None` = 从没摆过）。
static SHOWN_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// 记下"刚摆出去"这一刻。
fn mark_shown() {
    if let Ok(mut g) = SHOWN_AT.lock() {
        *g = Some(std::time::Instant::now());
    }
}

/// 是不是**刚摆出去**（还在落定窗口里）。
fn just_shown() -> bool {
    let Ok(g) = SHOWN_AT.lock() else {
        return false;
    };
    g.is_some_and(|t| t.elapsed() < SETTLE)
}

/// 真正建那扇窗。**只该由 [`show_at`] 调**（要先保证同 label 的旧窗已经销毁）。
fn build_card(app: &AppHandle, x: f64, y: f64, side: &str) -> Result<(), AppError> {
    let url = format!("index.html?guide=1&side={side}");
    let win = WebviewWindowBuilder::new(app, GUIDE_LABEL, WebviewUrl::App(url.into()))
        .title("SupportEase 引导")
        /* 卡片 + 一条箭头竖条 + 四周那圈给阴影的空（见 [`WINDOW_W`] / [`WINDOW_H`]） */
        .inner_size(WINDOW_W, WINDOW_H)
        .position(x, y)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        /*
         * ★★ **关掉原生窗口阴影**（2026-10-15）。透明无边框窗口留着系统阴影时，
         * 那圈灰是**按窗口矩形**画的 ⇒ 在卡片之外的透明区域能看到一块**灰底**
         * （作者原话："不够透明，看到灰灰的"）。
         * 立体感交给 CSS —— 阴影挂在 `GuideCard.module.css` 的 `.root` 上，
         * 跟着"卡片 ∪ 箭头"的轮廓走，比原生那圈干净。
         */
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .build()
        .map_err(|e| AppError::internal("建不出引导窗口").with_detail(e.to_string()))?;
    let _ = win.show();
    Ok(())
}

/// **强制**收掉引导窗口（没开着就什么都不做，也是成功）。
///
/// ★★★ 为什么不能用 `close()` —— 那是**请求**关闭，不是关掉了：
///
/// - `close()` 往主线程排一条 **Close** 消息，主线程收到后才走"关窗处理器 → 销毁"
///   再排一条 **Destroy** 消息；
/// - 而我们紧接着要拿**同一个 label** 再建一扇窗，那条 Create 排在 Close 后面、
///   **排在 Destroy 前面**；
/// - 于是 Create 执行时旧窗还在管理器里 ⇒ 直接报 "a webview with label `guide`
///   already exists" ⇒ 界面上就是作者看到的那句 **「建不出引导窗口」**。
///
/// `destroy()` 是**直接**排 Destroy ⇒ 它先于后面那条 Create 执行（见 [`show_at`]）。
fn destroy_guide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(GUIDE_LABEL) {
        let _ = win.destroy();
    }
}

/// 装上"切片器窗口动了/被压住了"的监听（**AX 推送，不是定时器**）。
///
/// ★ 这一层只有 macOS 有（`ax_watch` 整个模块都 cfg 掉了）—— 别处 `locate` 本来就
/// 回"这一台上指不了"，卡片根本不会露脸，也就没有"跟随"可言。
fn mount_watcher(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        /* 成败不在这里看：`start` 自己会派回主线程（CFRunLoop 的线程亲和性），
        真结果走日志与 `is_complete()` —— 见 `ax_watch` 模块头 */
        crate::ax_watch::start(app, sync);
    }
    #[cfg(target_os = "windows")]
    {
        /* Windows：装的是细定时回读（无推送），见 `win_watch` 模块头 */
        crate::win_watch::start(app, sync);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
    }
}

/// 卸掉监听（卡片关了 / 切片器不在了）。别处是 no-op，理由同上。
fn unmount_watcher(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::ax_watch::stop(app);
    #[cfg(target_os = "windows")]
    crate::win_watch::stop(app);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
    }
}

/// 正在盯着哪个切片器进程（给 [`sync`] 用）。
///
/// ★ 别处恒 `None`：没有取数层就没有"跟随"可言，`sync` 会直接返回。
fn watched_pid() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        crate::ax_watch::watched_pid()
    }
    #[cfg(target_os = "windows")]
    {
        crate::win_watch::watched_pid()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/* ---------- 摆放与平台小工具 ---------- */

/// 卡片放哪：**贴着目标的右侧**，右边放不下就翻到左侧。
///
/// y 取 `target.y - SHADOW_PAD` —— 窗口顶边在上，卡片本体（`top: SHADOW_PAD`）
/// 正好落在 `target.y` 上，与它指向的那个控件齐平，眼睛不用上下找。
///
/// 第三个返回值是**卡片在目标的哪一侧**（`right` / `left`）—— 前端拿它决定箭头
/// 指左还是指右，不用去猜自己被摆在哪。
///
/// 返回的 x 是**窗口**的左边（不是卡片的）—— 卡片那一侧要往窗口里让出
/// [`SHADOW_PAD`]（或 [`ARROW_PAD`]），窗口整体再相应朝外挪，卡片本体才正好落在
/// `target ± GAP` 上。
fn place_card(target: &RectDto, screen_w: f64) -> (f64, f64, &'static str) {
    let top = sane(target.y - SHADOW_PAD, 0.0);
    /* 卡片在右：窗口左边 = 卡片左边再往左让一条箭头竖条 */
    let right = sane(target.x + target.width + CARD_GAP - ARROW_PAD, 0.0);
    let screen_w = sane(screen_w, 1920.0);
    if right + WINDOW_W <= screen_w {
        (right, top, "right")
    } else {
        /* 卡片在左：窗口左边再往左让出卡片与右侧那圈阴影空 */
        (
            sane(target.x - CARD_GAP - CARD_W - SHADOW_PAD, 0.0),
            top,
            "left",
        )
    }
}

/// 把非有限数（NaN / ±Inf）换成兜底值。
///
/// ★★ 为什么必须挡一下：AX 报的坐标来自**别的进程**，多屏 + 缩放混搭时可能给出
/// 非有限数，而把 NaN 交给窗口定位会**直接掀掉进程**（不是画歪，是崩）。
/// 宁可摆到 0 那个角上（用户看得见、也知道不对），也不要闪退。
fn sane(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

/// 把切片器**那一扇窗**抬到最前 —— 他要看着 BBS 里那个板型控件做操作。
///
/// ★★★ **不要用 `open -a BambuStudio`**（2026-10-15 作者报的"有时候会把 MKP 置顶了"）：
/// `open -a` 是"**激活整个应用**"，在 macOS 上会把那个应用**所有**窗口一起带到前面。
/// 于是我们自己的 `main`（SupportEase）与工作台窗口也跟着冒到切片器上面 ——
/// 用户点了「查看引导」，看到的反而是我们这一屏，正是这条引导要避免的事。
///
/// 正解是 AX 的 **`kAXRaiseAction`**：它只抬**这一扇窗**，不动别的。
/// 抬不动（没授权 / 控件树里找不到）就**静静过去** —— 卡片照样摆出去，
/// 用户自己切一下窗口即可，这一件事不值得让整条引导失败。
fn raise_slicer_window() {
    #[cfg(target_os = "macos")]
    {
        /*
         * 两下一起上，缺一不可：
         * - `AXRaise`：把**那一扇窗**抬到它自己 App 的层序顶部（并成为 key window）；
         * - `AXFrontmost`：把**整个切片器 App** 激活 —— 建窗那一下激活的是**我们**，
         *   不把切片器整体拉回来，我们的主窗就压在它前面（作者截图里那一片遮挡）。
         */
        let raised = crate::ax::raise_slicer();
        let activated = crate::ax::activate_slicer();
        if !raised && !activated {
            tracing::warn!(
                "抬不动切片器（没辅助功能权限？）—— 卡片仍会摆出去，但可能被我们自己的窗口压住"
            );
        }
    }
    #[cfg(target_os = "windows")]
    {
        /*
         * Windows 没有窗 / App 那层区分，`raise` 与 `activate` 是同一件事
         * （还原 + `SetForegroundWindow`）。★ 系统对 `SetForegroundWindow` 有限制
         * （前台进程才准换前台），失败时**照实 warn**、不硬来 —— 与 macOS 同一条
         * "抬不动就静静过去"的口径，卡片仍会摆出去。
         */
        let raised = crate::ax::raise_slicer();
        let activated = crate::ax::activate_slicer();
        if !raised && !activated {
            tracing::warn!(
                "抬不动切片器（系统不准换前台？）—— 卡片仍会摆出去，但可能被我们自己的窗口压住"
            );
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { /* 别处本来就没有这一层（`locate` 会回"指不了"），无从抬 */ }
}

/// 屏幕有多宽（逻辑点）。拿不到时给一个常见值 —— 这只用来决定卡片往左还是往右放，
/// 差一点不影响"指得准不准"。
fn screen_width(app: &AppHandle) -> f64 {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.size().width as f64 / m.scale_factor())
        .unwrap_or(1920.0)
}
