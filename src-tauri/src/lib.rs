//! SupportEase 的 Rust 侧。
//!
//! 结构就四块：
//! - [`error`] —— 跨 IPC 边界的错误模型（与 `src/api/contract.ts` 对齐）
//! - [`obs`] —— 日志与 trace id
//! - [`fsx`] —— 两层数据根、防穿越、唯一的写盘出口
//! - [`ipc`] —— 前端能调的命令
//!
//! 启动顺序有讲究：**先建数据根，再装日志**（日志要写进 `internal_root/logs`），
//! 而这两步失败都不阻断启动 —— 用户要的是软件能开，不是日志齐全。

/// **后处理执行归档**：原文件 / 输出文件 / 执行记录（`_meta.json`）三件套。
/// 落点 `<用户根>/gcode_history/<日期>/` —— 报告页的数据源（见 [`archive`] 模块头）。
pub mod archive;
/// **参数分流**：切片器（BambuStudio）带 `--Toml/--Gcode` 调我们时走钩子，否则开界面
/// （见 [`args::Mode`]）。它是本程序"一个可执行物两种角色"的那半张脸。
pub mod args;
/// **屏幕引导的取数口**（macOS）：读切片器窗口与「板子类型」控件的屏幕坐标。
/// **只读** —— 箭头怎么画是界面的事，这里只回答"在哪"。
/// 为什么用 Accessibility 而不是固定偏移、以及那条实测判据，都写在模块头里。
pub mod ax;
/// **切片器窗口的跟随信号**（macOS）：注册 AX 通知（移动 / 缩放 / 最小化 /
/// 还原 / 换最前窗），切片器自己 push 过来 —— **不起定时器**。
/// 卡片该不该露脸、该挪到哪，由 [`ipc::guide::sync`] 收到通知后重算。
///
/// ★ 只有 macOS 有这一份（`#![cfg(target_os = "macos")]`）；别的平台编译期就
/// 没有这个模块，调用点因此要用 `cfg` 挡一下（见 `ipc::guide`）。
#[cfg(target_os = "macos")]
pub mod ax_watch;
pub mod chrome;
/// **开发台的故障档位表**（2026-10-19）：给某一个 API 方法装一条错，客户端里所有走
/// 这个方法的界面从此真的报错。**无条件编译** —— 造错发生在客户端进程里（工作台那条
/// loopback 只过一个 id），而客户端是普通构建；正式包里没有命令发得进来，这张表恒不被问到。
pub mod devfault;
/// **开发台的演出脚本**（`hook-sim` 假钩子与开发台共用一份）。
///
/// ★ 2026-10-14 起**不再门在 workbench 档**：演出发生在**客户端进程**
/// （工作台那档没有客户端窗口，只能发一条 `dev-scene` 让客户端演，见模块头）。
/// 它**不是产品行为**：客户端侧唯一入口是那条通道消息（真钩子从不发它）。
pub mod devsim;
pub mod error;
pub mod fsx;
/// **后处理钩子**：切片器导出 G-code 时调的那一次 —— 预设 → IR → 12 步管线（原地覆盖），
/// 跑完**立刻退**（切片器一秒都不多等）。窗口不在这里，见 [`hook_ipc`] 与 [`hook_ui`]。
pub mod hook;
/// **钩子 ↔ 界面 的那条通道**（一行一个 JSON，走 127.0.0.1）：跑完就退的那一侧往里写，
/// 留着窗口的那一侧读它。为什么窗口不能住在钩子进程里 —— 见该模块头。
pub mod hook_ipc;
/// **通道的显示那一半**：进度模态框的数据（事件 + 全量快照）、那颗「停止」、机型那一问。
pub mod hook_ui;
pub mod ipc;
/// 旧世代（`mkp-ssr`）数据根的只读入口 —— 报告页的执行账与模型缓存的唯一真实来源
pub mod legacy;
/// 系统通知（2026-10-17）：设置页「处理完成发送系统通知」那一格的真实现。
pub mod notify;
pub mod obs;
/// 预设数据的**纯读写与解析核心**：机型目录 / 资产 / 套餐 / 字段定义 / 界面布局，
/// 加上三层取值。[`presetdata::Presets::load_from`] 只要一个根。
///
/// **不带 feature gate** —— 客户端与工作台共用同一份代码，但两边读的层不同：
/// 工作台读 `<repo>/presets`（仓库里的开发源数据），客户端只读 catalog
/// （`appDataDir/catalog.json`，definition 由发布构建从同一批源算出）。
///
/// **数据根不在这里决定**，也**不在运行时读仓库** —— 客户端跑在一台没有仓库的机器上。
///
/// 与 [`ipc`] 的关系：`ipc` 是客户端命令面（什么能调），这一层是数据面（怎么读）——
/// 命令面挂在 `ipc` 上，解析逻辑住在这里，工作台也直接用它。
/// **我是被谁拉起来的**（2026-10-20）：后处理钩子是被**那一趟切片的 Bambu Studio**
/// 拉起来的 ⇒ 父进程号 = "最后处理用的是哪个切片器"的唯一不靠猜的答案。
/// 记进执行记录（`archive::Meta::slicer_pid`），引导层据此认"我刚才切的那一份"。
pub mod parent;
pub mod presetdata;
/// **执行记录的自动清理**：设置页那个开关打开时，按「条数 + 空间」双限静默清掉
/// `gcode_history` 里最旧的记录（**最新那条永远不删** —— 不按日历天数清，
/// 停用多久都不会把用户最后一次的记录带走）。见 [`retention`] 模块头。
pub mod retention;
/// **新数据世界**：随包 catalog 的释放口 + 说明书 / 交付文件的落点规则
/// （落点 = `catalog.path`，见 [`runtime::paths`]）。
/// 客户端首屏（九条预设读命令）从这里出数 —— 首屏唯一数据源 = catalog
/// （docs/DATA-ARCHITECTURE.md 判据 4；旧世界 `client/` 的 include_str! 铺盘已退役）。
pub mod runtime;
/// **切片器窗口的跟随信号**（Windows）：细定时回读（700ms 一次本地窗口列表查询 +
/// 键比较，没变就返回）—— macOS 是"AX 推送 + 兜底回读"，这里没有等价的轻量推送，
/// 直接用同一频率的回读当唯一信号（为什么这不算"频繁的轮询"，见模块头）。
///
/// ★ 只有 Windows 有这一份（`#![cfg(target_os = "windows")]`）；调用点要用
/// `cfg` 挡一下（见 `ipc::guide`）。
#[cfg(target_os = "windows")]
pub mod win_watch;
/// **切片器窗口被谁盖住了**：问 CoreGraphics 的窗口列表（**按层序从前往后**）。
/// 用来判"别的软件压上来了 ⇒ 卡片该收起来" —— 用**层级**而不是 AX 的"焦点"。
/// ★ 底下的窗口列表 API：macOS 是 CoreGraphics，Windows 是 `EnumWindows` 的 z 序
/// （`mod mac` / `mod win` 在文件里各自 cfg）；其它平台 `slicer_window` 恒 `None`，
/// 调用点见 `ipc::guide`。
pub mod zorder;

// 工作台（B03）。**默认构建里下面这一行不成立**，所以 `src/workbench/` 整个子树连编译
// 都不会被碰，给用户的二进制里搜不到任何 `wb_` 命令。
// 设计来源见 git 历史（旧 spec：.comate/specs/b03-backstage-workbench/doc.md §4，2026-10-11 删）
//
// （这里刻意用行注释：块注释在 Rust 里是可嵌套的，正文里出现 `/` 加 `*` 会开一个新注释，
// 而路径通配写法很容易写出那两个字符。）
#[cfg(feature = "workbench")]
pub mod workbench;

/* `Manager` 只被客户端窗口那一段（`get_webview_window`）用到 —— 工作台构建里没有
main 窗口，那段整个不编译，导入留着就是一条 unused import（CI 的 clippy 带
`-D warnings`，警告即失败），所以它跟着同一道闸门走 */
#[cfg(not(feature = "workbench"))]
use tauri::Manager;

/// 界面的入口（也是唯一入口）。
///
/// **钩子进程不走这里**：切片器带 `--Toml/--Gcode` 拉起来的那一次走
/// [`crate::hook::run_with_channel`]（干活 + 退出码，不开窗、不建 Tauri 应用）；
/// 窗口留在**这个**常驻进程里，靠 [`hook_ipc`] 那条通道喂它（见 [`hook_ui`] 模块头）。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let view = hook_ui::HookView::new();
    let for_setup = view.clone();

    with_commands(tauri::Builder::default())
        /* 系统文件选择器（第十二层：通用导入入口的"选择文件"那一半）。
        权限只开 `dialog:allow-open`（默认 capability），别的一律不给 */
        .plugin(tauri_plugin_dialog::init())
        /* 在文件管理器里显示（第十三层）。**只在 Rust 侧调**（我们自己的命令体里），
        所以不需要给它开任何 capability —— 前端够不着它的命令面 */
        .plugin(tauri_plugin_opener::init())
        /* 那一趟的显示状态（进度 / 问句 / 结论 + 回写线）—— 见 [`hook_ui`] */
        .manage(hook_ui::HookSlot(view))
        .setup(move |app| {
            let handle = app.handle().clone();

            /* 数据根：建不出来也继续，日志目录随之退到 stderr。
            把路径打出来是刻意的 —— 出问题时第一句要问的就是"它在找哪个目录" */
            match fsx::paths::internal_root(&handle) {
                Ok(root) => {
                    obs::tracing::init_tracing(&root.join("logs"));
                    tracing::info!(root = %root.display(), "内部数据根就位");
                    /* 界面语言先落地：后端这一侧的话术（`messages::text`）从这一句起
                    才跟界面同一种语言。读不出来就按"跟随系统"（`apply_lang(None)`），
                    不挡启动 —— 一句英文/中文不该让程序起不来 */
                    runtime::app_state::apply_lang(runtime::app_state::locale(&root).ok().flatten());
                    match fsx::paths::user_root(&handle) {
                        Ok(user) => tracing::info!(root = %user.display(), "用户数据根就位"),
                        Err(e) => tracing::warn!("用户数据根建不出来：{e}"),
                    }
                }
                Err(e) => {
                    eprintln!("[setup] 内部数据根建不出来，日志只写 stderr：{e}");
                    obs::tracing::init_tracing(std::path::Path::new("/tmp/supportease-logs"));
                    tracing::warn!("内部数据根不可用：{e}");
                }
            }

            /* 运行时 catalog：随包那份**只铺底**（层② → 层③ 的铺盘）。
            交付面不预建任何目录 —— 落点由 `catalog.path` 定，下载那一刻按需建。
            ★ 必须走 `ensure_released`（盘上没有才写）—— catalog 是 OTA 数据，用升级语义
            铺盘会让每次启动覆盖掉 OTA 拿到的新目录，下载随即 SHA 不匹配（2026-10-04 修）。
            失败只告警不挡启动：界面会显示「读不到说明书」，那是能据以行动的状态。 */
            match fsx::paths::internal_root(&handle)
                .and_then(|root| runtime::release::ensure_released(&root))
            {
                Ok(r) => tracing::info!(report = %r.summary(), "运行时 catalog 已就位"),
                Err(e) => tracing::warn!("运行时 catalog 没就位：{e}"),
            }

            /* 窗口外观：原生圆角 + 让 AppKit 按统一工具栏那一档摆红绿灯。
            两件事都只在 macOS 上有意义，失败都只打日志 —— 外观问题不该挡启动。
            ★ 只给客户端窗口做：工作台构建里**没有** main 窗口（见下一段），
            工作台那个用的是原生标题栏，不需要这套装饰 */
            #[cfg(not(feature = "workbench"))]
            if let Some(win) = app.get_webview_window("main") {
                chrome::install_unified_toolbar(&win);
                /* 圆角、以及**圆角外面那一圈的填色**。填色的深浅要按"用户选的那一档"算
                （`resolved_dark`：档位是"跟随系统"时才问原生外观）—— 直接读 webview 的
                `effectiveAppearance` 只会得到系统深浅，作者那台是"档位深色 + 系统浅色"，
                那样读会画错。 */
                let dark = ipc::prefs::resolved_dark(ipc::prefs::theme_pref(app.handle()), &win);
                chrome::apply_native_corners(&win, dark);
                /* 红绿灯：装了统一工具栏之后 AppKit 摆的是 (26,26) 那组默认坐标，
                我们用 Auto Layout 约束钉到作者要的位置（2026-10-17 起 —— 补摆 frame
                的老路在拖窗过程中一帧错一下，约束是布局的一部分，没有这个问题）。 */
                chrome::reposition_traffic_lights(&win);
                /* 上次是窄窗就还摆成窄窗、置顶档也照盘上那一格落（2026-10-17）。
                ★ 摆在装饰装好之后：搬窗会触发 `Resized`，那一下要重钉红绿灯 */
                ipc::window_layout::restore_at_startup(app.handle());
                /* 两个重钉时机：① 档位是"跟随系统"时系统深浅变了；
                ② 尺寸变了（拉窗 / 进出全屏）—— 进出全屏 AppKit 可能整棵标题栏视图树
                重建，按钮换新对象时要在它们身上重新钉约束（平时这些调用都是空转） */
                let watcher = win.clone();
                win.on_window_event(move |event| match event {
                    tauri::WindowEvent::ThemeChanged(_) => {
                        let pref = ipc::prefs::theme_pref(watcher.app_handle());
                        chrome::repaint_corner_fill(
                            &watcher,
                            ipc::prefs::resolved_dark(pref, &watcher),
                        );
                        chrome::reposition_traffic_lights(&watcher);
                    }
                    tauri::WindowEvent::Resized(_) => {
                        chrome::reposition_traffic_lights(&watcher);
                        /* 「上一次手动摆放」的记账点（2026-10-17）。事件这侧只补一个代数，
                        判定与落盘在 400ms 之后由后台线程做（见 `ipc::window_layout`） */
                        ipc::window_layout::touch_geometry(watcher.app_handle());
                    }
                    tauri::WindowEvent::Moved(_) => {
                        ipc::window_layout::touch_geometry(watcher.app_handle());
                    }
                    _ => {}
                });
            } else {
                eprintln!("[setup] 找不到 main 窗口，跳过窗口外观");
            }

            /* 工作台窗口。**刻意在运行时建，而不是写进配置**：工作台那份配置把
            `app.windows` 声明成**空数组** —— 于是这个构建里根本不开客户端窗口，
            `npm run tauri:workbench:dev` 起来就只有工作台这一页（客户端本体走
            `npm run tauri dev`，两件事从此是两条独立的路）。
            若改成在配置里声明窗口，windows 数组是整体覆盖的，就得把客户端那份也抄一遍，
            两份声明迟早漂移；写在这里，窗口的存在与 feature 严格同生共死。
            开不出来只警告不中止：工作台开不出来是开发者的事，不该让进程也起不来 */
            /* ★ **测试模式要在开窗口之前装好**：模式档记着"这台机器现在是不是测试环境"，
            它决定整套数据根指哪儿（`workbench::paths` 那一个开关）。装晚了的话，
            第一屏读的是正式、之后的命令读沙箱 —— 那种"我到底在看哪一个"的错位
            正是作者点名不要的（「我不能迷迷糊糊的，不知道我在测试版还是正式版」）。
            读不出来只告警并退回正式：正式那边的东西永远是真的 */
            #[cfg(feature = "workbench")]
            workbench::app::sandbox::install_at_startup(&handle);

            #[cfg(feature = "workbench")]
            if let Err(e) = workbench::open_window(&handle) {
                tracing::warn!("工作台窗口开不出来：{e}");
            }

            /* 这条进程当不当"那扇窗"：**bind 成功就是唯一那扇**（已经有人在做显示就不抢，
            见 `hook_ipc::bind_for_display`）。起监听器只是把通道摆好 —— 没有钩子来时
            它什么都不做，界面照旧。失败了只告警：等于"这台机器上没人能显示进度"，
            钩子那边照样把活干完（产物优先于界面）。 */
            match fsx::paths::internal_root(&handle) {
                Ok(root) => match hook_ipc::bind_for_display(&root) {
                    Some(listener) => {
                        let view = for_setup.clone();
                        let to_ui = handle.clone();
                        let writer = listener.serve(move |msg| {
                            /* 开发台的演出指令（工作台发的）与真钩子的消息**分流**：
                            它要在后台线程里按节奏演，不能堵住收消息这一段 */
                            match msg {
                                hook_ipc::FromHook::DevScene { scene } => {
                                    view.run_preview(&to_ui, &scene);
                                }
                                /* 开发台 · 故障注入（2026-10-19）：让后端失一次真 ——
                                客户端自己按 id 造那条错，emit 给前端的拦截表 */
                                hook_ipc::FromHook::DevFault { fault } => {
                                    view.run_fault(&to_ui, fault.as_deref());
                                }
                                other => view.apply(&to_ui, other),
                            }
                        });
                        for_setup.attach_writer(writer);
                        tracing::info!(root = %root.display(), "后处理通道已就位");
                    }
                    None => tracing::info!("已经有一扇窗在做显示，这一条只当界面"),
                },
                Err(e) => tracing::warn!("后处理通道没摆起来（这一侧不显示进度）：{e}"),
            }

            /*
             * 执行记录的自动清理（2026-10-13「设置里加一个自动清理开关」那一刀）：
             * 这一趟只**起个头** —— 走后台线程，不与首屏抢那一下盘（走一遍目录 + 数
             * 几个文件大小是毫秒级的事，但"启动该快的绝对不慢"这条不给例外）。
             * 开关读不出来 / 清理失败都只写日志，用户侧完全静默（见 `crate::retention`）。
             *
             * ★ 摆在整个 setup 的**末尾**（而不是"用户根就位"那一处）：上面这些步骤里
             * 有会动**数据根 / 模式**的动作（工作台那个沙箱开关就在上面），清理要按
             * **最终的**那一棵树去算 —— 摆早了，哪天有人真把根接上，这里就会清错树。
             *
             * 每次后处理跑完那一趟不在这一侧：切片器拉起的钩子进程自己清
             * （`hook::run_with_channel` 里那个调用点）—— 界面可能开在别的页面、甚至没开。
             */
            let clean_handle = handle.clone();
            std::thread::spawn(move || crate::retention::prune_if_enabled_app(&clean_handle));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}

/* ---------- 命令清单 ----------

Tauri 只允许调一次 `invoke_handler`，所以"客户端命令 + 可选的工作台命令"没法拼接，
只能按 feature 给出两份完整清单。

**两份里客户端那几个必须一字不差地同时出现。** 新增客户端命令时改两处 —— 这是
Tauri 的 API 形状决定的，不是这里想省事；把它放在相邻的两个函数里，是为了漏改时
一眼能看出来。 */

#[cfg(not(feature = "workbench"))]
fn with_commands(b: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    b.invoke_handler(tauri::generate_handler![
        ipc::get_calib_models,
        ipc::open_model,
        // 「跟随系统默认」到底是哪个应用：去问系统（macOS LaunchServices / Windows 注册表）
        ipc::get_system_slicer_app,
        // 「去系统里改默认应用」：我们不替用户改，只把他送到系统那一处
        ipc::open_default_app_settings,
        // 「自动化」授权被拒后的反悔入口：把用户送到系统设置那一页（macOS）
        ipc::open_automation_settings,
        // 「复制后处理脚本」那一整条命令：两段真值（本程序自己 + 预设真实落点）现拼
        ipc::get_post_process_command,
        // 钩子那一趟的三条（切片器导出时）：看进度 / 停 / 答机型不匹配那一问
        ipc::postprocess::get_post_process_run,
        ipc::postprocess::cancel_post_process,
        ipc::postprocess::reset_post_process_state,
        ipc::postprocess::answer_post_process,
        ipc::postprocess::replay_post_process_question,
        ipc::postprocess::replay_post_process_run,
        // 工艺页「本机工艺」那一档：读本机 BBS 目录（只读；见 `ipc/bbs.rs`）
        ipc::bbs::get_bbs_presets,
        ipc::bbs::read_bbs_preset,
        // 预设页（A41）的真后端。读客户端自己的数据根（`appDataDir/presets`）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 <appDataDir>/user/presets-local）
        ipc::local::get_local_preset_files,
        ipc::local::read_local_preset_text,
        // 备注覆盖账 + 改归属（客户端副标题 / 复制出来的那份标机型版本）
        ipc::local::get_preset_remarks,
        ipc::local::set_preset_remark,
        ipc::local::set_local_preset_machine_version,
        // 临时编辑那条链：把云端正文复制进临时文件 → 改 → 另存成用户文件
        ipc::local::begin_preset_edit,
        ipc::local::patch_preset_draft,
        ipc::local::patch_preset_draft_batch,
        ipc::local::check_preset_text,
        ipc::local::put_preset_draft,
        ipc::local::discard_preset_draft,
        ipc::local::commit_preset_draft,
        // 冷启动接回草稿层：后端只存正文，"改了哪几项"要从正文与来源比出来
        ipc::local::read_preset_draft_changes,
        // 「编辑文件」那条链（2026-10-10：与上面参数草稿分开存，两侧互相作废）
        ipc::local::begin_text_edit,
        ipc::local::put_text_draft,
        ipc::local::commit_text_draft,
        ipc::local::discard_text_draft,
        ipc::local::get_preset_draft_summary,
        // 「恢复默认值」（编辑文件那颗）读的原文镜像
        ipc::local::read_local_origin_text,
        // 身份账本（预设页判「我这边有没有一份跟云端对得上」的唯一依据）
        ipc::local::get_local_preset_index,
        // 第十层：用户文件管理（改名 / 删除 —— 只动名字或删掉，字节一个不动）
        ipc::local::rename_local_preset,
        ipc::local::delete_local_preset,
        // 第十三层：文件外部管理（在 Finder / 资源管理器里选中这一份）
        ipc::local::reveal_in_folder,
        // 第十一层：另存为一份新的（本地文件 → 本地文件，字节复制）
        ipc::local::copy_local_preset,
        // 找「我那一份」+ 把校准值写进它（2026-10-08 资源库改判）
        ipc::local::get_local_copy_for,
        ipc::local::save_preset_calibration,
        // 备份与恢复（2026-10-17）：设置页 —— 打一份 zip / 从一份 zip 全量恢复
        ipc::backup::create_backup,
        ipc::backup::restore_backup,
        // 修改历史（2026-10-17）：一份文件一本账，盘上留着
        ipc::history::get_preset_history,
        ipc::history::put_preset_history,
        ipc::history::clear_preset_history,
        // 「哪份预设用得最多」的本机学习账（2026-10-17）：只读；写入在"应用预设"那一步
        ipc::usage::get_preset_usage,
        // 对比台 + 云端版本账（2026-10-08）：只在**用户自己的预设**之间对比
        ipc::preset_baseline::get_cloud_versions,
        ipc::preset_params::read_preset_params,
        ipc::preset_params::save_preset_params,
        // 逐参数「云端更新」（2026-10-09）：读三方账 / 落采用·保持的决定
        ipc::param_sync::get_preset_param_sync,
        ipc::param_sync::apply_preset_param_decisions,
        // 第十二层：通用导入入口（看落点 / 提交；Preset 只是第一个消费者）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
        /* 打开两个落点所在的文件夹（2026-10-13）：我们那份（下载区 `assets/bbs/…`）
        + 切片器目录里那份（`user/<账号>/process/…`）—— 这两处不是一个地方，
        界面上要能分别打开 */
        ipc::presets::reveal_delivered_file,
        ipc::presets::reveal_slicer_file,
        // 切片器的「生效」= 复制进切片器自己的用户配置目录（2026-10-09 接通）
        ipc::presets::copy_to_slicer,
        // 新数据世界（第一圈）：运行时 catalog，读 `<appDataDir>/catalog.json`
        ipc::catalog::get_runtime_catalog,
        ipc::catalog::get_downloaded_files,
        ipc::catalog::download_runtime_file,
        ipc::catalog::download_runtime_files,
        ipc::catalog::read_downloaded_text,
        // 「复制链接」：与下载管道同一个寻址出口算出来的云端 URL
        ipc::catalog::get_file_url,
        // 报告页：后处理执行报告与历史（读 gcode_history 的真账 + 一处导出写）
        ipc::report::get_report_list,
        ipc::report::get_report_detail,
        ipc::report::get_report_export_plan,
        ipc::report::export_report_archive,
        // 导出之后那颗「打开位置」：打开系统文件管理器，落在**后端自己算出来的**那个目录
        ipc::report::reveal_report_export,
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        // 资源库那一条路（2026-10-08 改判）：交付预设「使用」= 按需取回 + 写成当前使用
        ipc::catalog::fetch_cloud_preset,
        ipc::catalog::clear_active_preset,
        ipc::catalog::get_stale_files,
        // 这一份我们认得出是哪一版吗（第 6 层：SHA 报警）
        ipc::catalog::get_delivery_trust,
        ipc::catalog::get_archived_files,
        ipc::catalog::read_archived_text,
        // 删除（作者裁决 2026-10-06：一切皆可删 —— 删了可重下/代价讲清）
        ipc::catalog::delete_delivery_file,
        ipc::catalog::delete_archived_file,
        ipc::catalog::check_remote_update,
        ipc::catalog::apply_remote_update,
        // 软件更新（release.json）—— 与预设数据两条链：设置页「软件更新」块的两个口子
        ipc::catalog::get_app_version,
        ipc::catalog::check_software_update,
        ipc::catalog::get_preset_source,
        ipc::catalog::set_preset_source,
        ipc::catalog::clear_preset_source,
        // 界面偏好（2026-10-12 深色模式那一刀）：主题（浅 / 深 / 跟随系统）——
        // 落 AppState 的 theme 格，读一次写一次，不联网
        ipc::prefs::get_theme,
        ipc::prefs::set_theme,
        ipc::prefs::get_locale,
        ipc::prefs::set_locale,
        // 窗口外观（2026-10-22）：原生背景色 = 界面底（`--bg-2` 由前端报上来）——
        // 治 macOS 拖拽 resize 露白边（WebKit#323132，见 `ipc::prefs::set_window_backdrop`）
        ipc::prefs::set_window_backdrop,
        // 界面偏好第二格（2026-10-13）：执行记录的自动清理开关 ——
        // 落 AppState 的 autoCleanHistory 格；真动手清理的是 `retention`，不联网
        ipc::prefs::get_auto_clean_history,
        ipc::prefs::set_auto_clean_history,
        // 预设页偏好（2026-10-20）：「复制出来的也跟着云端更新」——
        // 落 AppState 的 presets 格；判据在身份账本与逐参数更新账那两个出口，不联网
        ipc::prefs::get_copies_follow_cloud,
        ipc::prefs::set_copies_follow_cloud,
        ipc::prefs::get_postproc_prefs,
        ipc::app::open_preset_folder,
        ipc::app::uninstall,
        ipc::prefs::set_postproc_prefs,
        // 主窗怎么摆（2026-10-17）：置顶三档 + 窄窗。落 AppState 的
        // pin / compact / lastWindowBox 三格；窗口几何的副作用只在 `ipc::window_layout`
        ipc::window_layout::get_window_prefs,
        ipc::window_layout::set_pin,
        ipc::window_layout::apply_pin,
        ipc::window_layout::set_compact,
        // 屏幕引导（2026-10-15）：改了参数之后，把"回切片器换一下板子类型"那一步
        // **指给用户看**。只读切片器窗口与控件的屏幕坐标，不替用户点、不改任何参数。
        ipc::guide::locate_bed_type_control,
        ipc::guide::guide_show,
        ipc::guide::guide_close,
        // ★ **应用内更新 + 打开外链**（第五刀）—— 2026-10-09 补进客户端清单：
        // 这七条原来只在工作台那份里，正式客户端点「检查更新 / 下载安装」全落到
        // NOT_IMPLEMENTED（X-01）。与下面那份清单一字不差。
        ipc::update::update_info,
        ipc::update::start_update,
        ipc::update::pause_update,
        ipc::update::resume_update,
        ipc::update::cancel_update,
        ipc::update::install_update,
        ipc::update::open_url,
    ])
}

#[cfg(feature = "workbench")]
fn with_commands(b: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    use workbench::app;
    b.invoke_handler(tauri::generate_handler![
        ipc::get_calib_models,
        ipc::open_model,
        // 「跟随系统默认」到底是哪个应用：去问系统（macOS LaunchServices / Windows 注册表）
        ipc::get_system_slicer_app,
        // 「去系统里改默认应用」：我们不替用户改，只把他送到系统那一处
        ipc::open_default_app_settings,
        // 「自动化」授权被拒后的反悔入口：把用户送到系统设置那一页（macOS）
        ipc::open_automation_settings,
        // 「复制后处理脚本」那一整条命令：两段真值（本程序自己 + 预设真实落点）现拼
        ipc::get_post_process_command,
        // 钩子那一趟的三条（切片器导出时）：看进度 / 停 / 答机型不匹配那一问
        ipc::postprocess::get_post_process_run,
        ipc::postprocess::cancel_post_process,
        ipc::postprocess::reset_post_process_state,
        ipc::postprocess::answer_post_process,
        ipc::postprocess::replay_post_process_question,
        ipc::postprocess::replay_post_process_run,
        // 工艺页「本机工艺」那一档：读本机 BBS 目录（与上面那份清单一字不差）
        ipc::bbs::get_bbs_presets,
        ipc::bbs::read_bbs_preset,
        // 客户端命令：**两份清单一字不差**（漏一份就是「原生机能用、工作台构建不能用」）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 <appDataDir>/user/presets-local）
        ipc::local::get_local_preset_files,
        ipc::local::read_local_preset_text,
        // 备注覆盖账 + 改归属（与上面那份清单一字不差 —— 2026-10-09 对齐：这三条
        // 原来只在客户端那份里，工作台构建里发它们会落到 NOT_IMPLEMENTED）
        ipc::local::get_preset_remarks,
        ipc::local::set_preset_remark,
        ipc::local::set_local_preset_machine_version,
        // 临时编辑那条链：把云端正文复制进临时文件 → 改 → 另存成用户文件
        ipc::local::begin_preset_edit,
        ipc::local::patch_preset_draft,
        ipc::local::patch_preset_draft_batch,
        ipc::local::check_preset_text,
        ipc::local::put_preset_draft,
        ipc::local::discard_preset_draft,
        ipc::local::commit_preset_draft,
        // 冷启动接回草稿层：后端只存正文，"改了哪几项"要从正文与来源比出来
        ipc::local::read_preset_draft_changes,
        // 「编辑文件」那条链（2026-10-10：与上面参数草稿分开存，两侧互相作废）
        ipc::local::begin_text_edit,
        ipc::local::put_text_draft,
        ipc::local::commit_text_draft,
        ipc::local::discard_text_draft,
        ipc::local::get_preset_draft_summary,
        // 「恢复默认值」（编辑文件那颗）读的原文镜像
        ipc::local::read_local_origin_text,
        // 身份账本（预设页判「我这边有没有一份跟云端对得上」的唯一依据）
        ipc::local::get_local_preset_index,
        // 第十层：用户文件管理（与上面那份清单一字不差）
        ipc::local::rename_local_preset,
        ipc::local::delete_local_preset,
        // 第十三层：文件外部管理（与上面那份清单一字不差）
        ipc::local::reveal_in_folder,
        // 第十一层：另存为一份新的（与上面那份清单一字不差）
        ipc::local::copy_local_preset,
        // 找「本地那份」+ 把校准值写进它（与上面那份清单一字不差）
        ipc::local::get_local_copy_for,
        ipc::local::save_preset_calibration,
        // 备份与恢复（与上面那份清单一字不差）
        ipc::backup::create_backup,
        ipc::backup::restore_backup,
        // 修改历史（与上面那份清单一字不差）
        ipc::history::get_preset_history,
        ipc::history::put_preset_history,
        ipc::history::clear_preset_history,
        // 学习账（与上面那份清单一字不差）
        ipc::usage::get_preset_usage,
        // 对比台 + 云端版本账（与上面那份清单一字不差）
        ipc::preset_baseline::get_cloud_versions,
        ipc::preset_params::read_preset_params,
        ipc::preset_params::save_preset_params,
        // 逐参数「云端更新」（与上面那份清单一字不差）
        ipc::param_sync::get_preset_param_sync,
        ipc::param_sync::apply_preset_param_decisions,
        // 第十二层：通用导入入口（与上面那份清单一字不差）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
        // 打开两个落点所在的文件夹（与上面那份清单一字不差）
        ipc::presets::reveal_delivered_file,
        ipc::presets::reveal_slicer_file,
        // 切片器的「生效」= 复制进切片器自己的用户配置目录（与上面那份清单一字不差）
        ipc::presets::copy_to_slicer,
        // 新数据世界（第一圈）：与上面那份清单保持一字不差
        ipc::catalog::get_runtime_catalog,
        ipc::catalog::get_downloaded_files,
        ipc::catalog::download_runtime_file,
        ipc::catalog::download_runtime_files,
        ipc::catalog::read_downloaded_text,
        // 「复制链接」（与上面那份清单一字不差）
        ipc::catalog::get_file_url,
        // 报告页（与上面那份清单一字不差）
        ipc::report::get_report_list,
        ipc::report::get_report_detail,
        ipc::report::get_report_export_plan,
        ipc::report::export_report_archive,
        ipc::report::reveal_report_export,
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        // 资源库那一条路（2026-10-08 改判）：交付预设「使用」= 按需取回 + 写成当前使用
        ipc::catalog::fetch_cloud_preset,
        ipc::catalog::clear_active_preset,
        ipc::catalog::get_stale_files,
        // 这一份我们认得出是哪一版吗（第 6 层：SHA 报警）
        ipc::catalog::get_delivery_trust,
        ipc::catalog::get_archived_files,
        ipc::catalog::read_archived_text,
        // 删除（作者裁决 2026-10-06：一切皆可删 —— 删了可重下/代价讲清）
        ipc::catalog::delete_delivery_file,
        ipc::catalog::delete_archived_file,
        ipc::catalog::check_remote_update,
        ipc::catalog::apply_remote_update,
        // 软件更新（release.json）—— 与上面那份清单一字不差
        ipc::catalog::get_app_version,
        ipc::catalog::check_software_update,
        ipc::catalog::get_preset_source,
        ipc::catalog::set_preset_source,
        ipc::catalog::clear_preset_source,
        // 界面偏好（2026-10-12）：主题 —— 与客户端那份清单一字不差
        ipc::prefs::get_theme,
        ipc::prefs::set_theme,
        ipc::prefs::get_locale,
        ipc::prefs::set_locale,
        // 窗口外观（2026-10-22）：与客户端那份清单一字不差
        ipc::prefs::set_window_backdrop,
        // 界面偏好第二格（2026-10-13）：自动清理开关 —— 与上面那份清单一字不差
        ipc::prefs::get_auto_clean_history,
        ipc::prefs::set_auto_clean_history,
        // 预设页偏好（2026-10-20）：与上面那份清单一字不差
        ipc::prefs::get_copies_follow_cloud,
        ipc::prefs::set_copies_follow_cloud,
        ipc::prefs::get_postproc_prefs,
        ipc::app::open_preset_folder,
        ipc::app::uninstall,
        ipc::prefs::set_postproc_prefs,
        // 主窗怎么摆（2026-10-17）：与上面那份清单一字不差
        ipc::window_layout::get_window_prefs,
        ipc::window_layout::set_pin,
        ipc::window_layout::apply_pin,
        ipc::window_layout::set_compact,
        // 屏幕引导（2026-10-15）：改了参数之后，把"回切片器换一下板子类型"那一步
        // **指给用户看**。只读切片器窗口与控件的屏幕坐标，不替用户点、不改任何参数。
        ipc::guide::locate_bed_type_control,
        ipc::guide::guide_show,
        ipc::guide::guide_close,
        // 工作台（doc §6 的新契约）。**配方内容的写只有 wb_apply_draft 一条** ——
        // 其余全是读、查（只读推演）、或一件明确的事（`wb_set_bootstrap` 是单值配置写，
        // 不进制 draft 体系——见 app/mod.rs 那条命令的注释）。
        // 旧那 30 多个命令已全部作废：每个按钮各自写盘的话，撤销、脏计数、
        // 差异列表、状态一致性每一件都要挨个改十几处。
        app::wb_open,
        app::wb_boot,
        app::wb_reload,
        app::wb_presets_changed,
        app::wb_set_bootstrap,
        app::wb_book,
        app::wb_registry,
        // 参数定义编辑（2026-10-03 作者：「弃用是谁决定的？我没办法改」）：
        // 名称 / 单位 / 值类型 / 控件 / 范围 / 步进 / 出厂默认 / 属于 / 前置条件 / 弃用，
        // 就地写进 param_registry.toml 的 [[params]] 本体 —— 即时落盘，不走参数草稿
        app::wb_set_param_meta,
        // 状态词的唯一出处。开场取一次，前端按枚举值查 ——
        // 不给它的话，同一个词会在 TSX 里再写一遍（doc §13）
        app::words::wb_words,
        // 「机型与版本」那一页：读写 presets/machines/*.toml。
        // **不走 wb_apply_draft** —— 清单与参数值不共用状态机（见 app/machines.rs 头注）
        app::machines::wb_machines,
        // 品牌（2026-10-03 作者：「品牌也要像机型一样能编辑」）：显示名 + 品牌图，
        // 即时落盘（同机型那一套）；品牌图是资产 id，清空回落内置字标
        app::machines::wb_set_brand_field,
        app::machines::wb_add_brand,
        // 移动机型（2026-10-03 作者：「把某一个机型移到其他品牌下，就是那种正常的移动」）：
        // 只改机型文件的 brand 一格（复用 wb_set_machine_field 那条路），
        // 多一道「目标品牌真的存在」的校验 —— 打错一个字会在盘上留下悬空的归属
        app::machines::wb_move_machine_to_brand,
        // 尺寸六组（照旧面板移植）：整张 [dimensions] 一次写回，全零可选组由后端剔除
        app::machines::wb_set_machine_dimensions,
        // 禁区：空数组 = 删掉 forbidden_zones/<id>.toml（清空是删文件，不是留个空文件）
        app::machines::wb_set_machine_zones,
        // 「资产库」：读 `presets/assets.toml`（b05 Task 8），P4 起带三轴派生与筛选；
        // 删除走数据层的反查守卫（有人引用整次拒绝）
        app::assets::wb_assets,
        app::assets::wb_asset_usage,
        // 资产检查面板（第四刀）：选中一条才问 —— 重字段（SHA-256 / 尺寸）要读真实字节
        app::assets::wb_asset_inspect,
        // 「在访达中显示」：前端只传资产 id，路径由后端算（只读、只开窗口、不碰状态）
        app::assets::wb_reveal_asset,
        app::assets::wb_remove_asset,
        app::assets::wb_set_asset_delivery,
        app::assets::wb_set_calibration_identity,
        // 「分组」（2026-10-16）：哪几张图共用同一份标注源稿 —— 登记在台账 `[[groups]]`，
        // 即时落盘、回一份新清单。作者要的是"像台账那样的管理器"，不是靠文件名算
        app::assets::wb_set_group,
        app::assets::wb_remove_group,
        // 「标注台」（2026-10-16）：图上那些框的**源稿** —— 与图同层、随仓库走、不进交付目录。
        // 前端只给资产 id，落点由后端从台账派生（读不到 = 还没拖过，合法，不是错误）
        app::annotation::wb_anno_read,
        app::annotation::wb_anno_write,
        // 「生成预览」：按**当前视角**烘一张成品图**只回字节、一个盘都不写** ——
        // 相当于"拍一张照不存相册"（作者 2026-10-16）。确认满意了再 `wb_anno_write` 存进去。
        app::annotation::wb_anno_render,
        // 「套餐管理」：`presets/bundles.toml` 是套餐唯一真源（Task 13.6）。
        // P4 起：读视图带指向关系（一版一套）+ 换文件清单（即时落盘，不走参数草稿）；
        // 2026-10-03 起新建 / 编辑（改 id 连带重指机型文件）/ 复制 / 删除（被指着整次拒绝）
        app::bundles::wb_bundles,
        app::bundles::wb_set_bundle_refs,
        app::bundles::wb_add_bundle,
        app::bundles::wb_rename_bundle,
        app::bundles::wb_copy_bundle,
        app::bundles::wb_assign_bundle_versions,
        app::bundles::wb_remove_bundle,
        // 交付残留（b05 Task 13.4/13.5）：查询清单 + 显式清理（进 .trash 回收）
        app::build::wb_dist_strays,
        app::build::wb_clean_dist_strays,
        // 交付文件清单（2026-10-07）：「交付预设」卡看这次都会写出哪些文件 + 看某一份的原文
        app::build::wb_delivery_files,
        app::build::wb_delivery_file,
        // 版本复制与参数正文复制（b05 Task 14.3/14.5）：两步分离，各自单文件写入
        app::machines::wb_copy_version,
        app::wb_copy_recipe,
        // 对照基线（b05 Task 14.9）：diff 只读 + 确认后同步（落点闸在 preset 内部）
        app::build::wb_baseline_diff,
        app::build::wb_sync_baseline,
        app::machines::wb_add_machine,
        app::machines::wb_add_version,
        app::machines::wb_version_orphans,
        app::machines::wb_remove_version,
        app::machines::wb_set_version_field,
        app::machines::wb_set_machine_field,
        // 默认视角是分组列表（`wb_desk`）；矩阵退成「同时看几台机器的同一项」那个对比工具
        app::wb_desk,
        app::wb_matrix,
        app::wb_trash,
        app::wb_ui,
        app::wb_save_ui,
        app::wb_preview_bulk,
        app::wb_diff_draft,
        app::wb_apply_draft,
        app::wb_save,
        app::wb_discard,
        // 生成 / 校验 / 恢复 / 发布。**恢复走 wb_revert_preview 只算不写** ——
        // 算出来的 patch 交给 wb_apply_draft，于是恢复也是一条撤销、也进同一份差异清单
        app::build::wb_preflight,
        // 生成前预演（只算不写）：界面上「点生成 → 先看 diff → 再确认」的那一步
        app::build::wb_generate_preview,
        // 当前安装的版本号（只读）：仅用于「软件版本」展示位，不发版本
        app::wb_app_version,
        app::build::wb_generate,
        app::build::wb_revert_preview,
        // 发布闸（第二刀）：逐项打勾的结果；`can_publish` 是"能不能往下走"的唯一答案
        app::build::wb_publish_audit,
        // **发布事务**（第三刀下半）：唯一对外的发布动作 —— 审计 → 生成 → 定稿 → git → PR
        app::build::wb_publish,
        // 发布账户 / 状态（第三刀下半）：仓库地址 + 用户名（配置）+ Token（凭据文件；写入只进不出、
        // 明文只在设置页点「眼睛」时显式取）+ 手动回读 PR/MR
        app::publish_tx::wb_publish_account,
        app::publish_tx::wb_set_publish_account,
        app::publish_tx::wb_set_publish_token,
        app::publish_tx::wb_get_publish_token,
        app::publish_tx::wb_clear_publish_account,
        app::publish_tx::wb_publish_status,
        // 合并（作者 2026-10-04 拍：squash、不强制等 CI）—— 用户显式点过才走
        app::publish_tx::wb_merge_review,
        // 发布收尾（含回执屏）：历史只读一份；「查看 PR」把地址交给系统浏览器
        app::build::wb_publish_history,
        app::build::wb_open_external,
        // ★ **应用内更新 + 打开外链**（第五刀）：`open_url` 修的是「查看更新点了没反应」
        ipc::update::update_info,
        ipc::update::start_update,
        ipc::update::pause_update,
        ipc::update::resume_update,
        ipc::update::cancel_update,
        ipc::update::install_update,
        ipc::update::open_url,
        // ★ **发布软件版本**（第四刀）：闸 / 事务 / 历史各一条 —— 与 CLI 同一个内核
        app::release_tx::wb_release_preflight,
        app::release_tx::wb_release_software,
        app::release_tx::wb_release_history,
        // run-env 拦下时的一键解法：杀掉本进程自己的 dev 监视器（先验明正身再动手）
        app::release_tx::wb_release_kill_dev_watcher,
        // 「复制发布提示词」：把这一版要怎么发整成一段能直接贴给 AI 的任务书（只读）
        app::release_tx::wb_release_prompt,
        // 「打开安装包目录」：不管发没发出去，都让人能去文件管理器里看产物在哪个目录（只读）
        app::release_tx::wb_release_open_bundle,
        // 「本地测试源（开发）」：起 / 停 / 查那颗按钮背后的 `npm run dev:test-update`。
        // ★ 三条**只为工作台存在**：它起的是客户端 dev，但源地址由环境变量注入
        // （`MKPSE_PRESET_SOURCE_URL`，只在 debug 构建里认），这一层不写任何配置 ——
        // 客户端自己的「预设数据源」那一格始终归客户端设置页管
        app::dev_source::wb_dev_source_start,
        app::dev_source::wb_dev_source_stop,
        app::dev_source::wb_dev_source_status,
        // 端口被占着时的那一下：起之前就探（谁占着、PID 多少），界面摆出来问一句
        // 「要不要把它停了」—— 停的那一条单独给，先验明正身再动手
        app::dev_source::wb_dev_source_clear_conflict,
        // **测试模式（沙箱）**：整套数据根切到 `<repo>/workbench/.sandbox` 那一棵，
        // 正式目录一个字节都不碰。四条：现状 / 开关 / 清空并重拷 / 清空
        app::sandbox::wb_sandbox_status,
        app::sandbox::wb_sandbox_set,
        app::sandbox::wb_sandbox_refill,
        app::sandbox::wb_sandbox_wipe,
        // **开发台**：把"看不见的那几屏"按一下按钮显示出来（后处理 + 几个模态框）。
        // ★ 这两条命令**发给客户端进程**（走 hook 那条 loopback）—— 工作台这档没有
        // 客户端窗口，界面显示在**客户端窗口**里（见 `app::devsim` 模块头）。
        app::devsim::wb_dev_devsim_scenes,
        app::devsim::wb_dev_trigger,
        // **开发台 · 故障注入**（2026-10-19）：给某个 API 方法装一条错 ——
        // 客户端里所有走这个方法的界面从此真的报错（真组件 / 真按钮 / 真错误码）。
        // 同一条 loopback、同一个"客户端没开就如实报"的口径（见 `app::devfault` 模块头）。
        app::devfault::wb_dev_faults,
        app::devfault::wb_dev_fault_arm,
        app::devfault::wb_dev_fault_clear,
        // **开发台 · 运行真实后处理**（2026-10-19）：起一个**真的钩子子进程**跑真 G-code
        // （`--Toml` + `--Gcode`，与切片器注册的那一行同形）⇒ 客户端窗口显示的是真进度、
        // 真问句、真结论，那几颗真业务按钮也真的能停。**不加协议变体、不改产品逻辑**
        // （见 `app::devreal` 模块头）。
        app::devreal::wb_dev_run_real,
        // **开发台 · 真实后处理测试矩阵**（2026-10-19）：六档各跑一个独立临时根，
        // 串行、只按真凭据判定。问句档 / 取消档要人在客户端窗口真操作。
        app::devmatrix::wb_dev_matrix_cases,
        app::devmatrix::wb_dev_matrix_state,
        app::devmatrix::wb_dev_matrix_start,
        app::devmatrix::wb_dev_matrix_clear,
    ])
}

#[cfg(test)]
mod command_list_parity {
    //! **两份命令清单的一致性判据**（X-01 / X-08 的钉子）。
    //!
    //! Tauri 只许调一次 `invoke_handler`，「客户端命令 + 可选的工作台命令」没法拼接，
    //! 只能给出两份完整清单 —— **手写的两份一定会漂**，而且已经漂过：
    //!
    //! - `ipc::update::*` 七条只在工作台那份里 ⇒ 正式客户端的「检查更新 / 下载安装」
    //!   全落到 `NOT_IMPLEMENTED`（审计 X-01，2026-10-09 修）；
    //! - `ipc::local::set_local_preset_machine_version` 等三条只在客户端那份里
    //!   （审计 X-08 的同一个病）。
    //!
    //! 所以这里扫**本文件源码**（与 `fsx::paths` 那条"用户根不碰 Documents"同一路数）：
    //! 把两个 `generate_handler![…]` 块里的命令路径抽出来逐个比 ——
    //! 客户端有的工作台必须有；工作台多出来的只许是 `workbench::app` 那一支。

    use std::collections::BTreeSet;

    /// 一个 `generate_handler![…]` 块里的命令路径（`a::b::c` 形状的那些 token）。
    fn commands_in(block: &str) -> BTreeSet<String> {
        block
            .lines()
            .map(|l| l.split("//").next().unwrap_or(""))
            .flat_map(|l| l.split(','))
            .map(|t| t.trim().to_string())
            .filter(|t| t.contains("::") && !t.contains(' ') && !t.contains('('))
            .collect()
    }

    fn the_two_lists() -> Vec<BTreeSet<String>> {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("读得到 lib.rs");
        let mut lists = Vec::new();
        let mut rest = src.as_str();
        /* 锚 = **调用现场**那一句，现场拼出来：本模块自己的注释与常量里都写着
        片段字面量，整句拼接只在两处真实调用点出现 —— 免得把自己扫成第三份清单。
        收口认 `\n    ])`（两份清单的收尾行）而不是第一个 `]` —— 块里注释带 `]`
        会把清单从中间截断（实测就是这么把 update 七条截丢的） */
        let mark = ["invoke_handler(tauri::", "generate_handler!["].concat();
        let end_mark = "\n    ])";
        while let Some(pos) = rest.find(&mark) {
            let after = &rest[pos + mark.len()..];
            let end = after
                .find(end_mark)
                .expect("generate_handler! 块要以 ]\\n 收尾");
            lists.push(commands_in(&after[..end]));
            rest = &after[end..];
        }
        lists
    }

    #[test]
    fn the_two_command_lists_never_drift() {
        let lists = the_two_lists();
        assert_eq!(lists.len(), 2, "应该正好两份清单（客户端 / 工作台）");
        let client = &lists[0];
        let workbench = &lists[1];

        /* 工作台那份多出来的只许是 workbench 自己的（`workbench::app` / `app::` 一支），
        客户端命令一个都不许少 —— 少了就是「原机能用、工作台构建不能用」（或反过来） */
        let wb_only: BTreeSet<String> = workbench
            .difference(client)
            .filter(|c| !c.starts_with("workbench::") && !c.starts_with("app::"))
            .cloned()
            .collect();
        assert!(
            wb_only.is_empty(),
            "工作台清单里混进了客户端没有的非工作台命令：{wb_only:?}"
        );

        let missing: BTreeSet<String> = client.difference(workbench).cloned().collect();
        assert!(
            missing.is_empty(),
            "这些命令只在客户端清单里，工作台构建发它会落 NOT_IMPLEMENTED：{missing:?}"
        );
    }

    /// 更新那七条**必须在两份清单里都有**（这次修复的靶子，单独钉一遍）
    #[test]
    fn the_update_commands_are_registered_everywhere() {
        let lists = the_two_lists();
        for list in &lists {
            for cmd in [
                "ipc::update::update_info",
                "ipc::update::start_update",
                "ipc::update::pause_update",
                "ipc::update::resume_update",
                "ipc::update::cancel_update",
                "ipc::update::install_update",
                "ipc::update::open_url",
            ] {
                assert!(
                    list.contains(cmd),
                    "{cmd} 不在一份清单里 —— 界面点下去会 NOT_IMPLEMENTED"
                );
            }
        }
    }
}
