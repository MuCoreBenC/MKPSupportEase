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
pub mod chrome;
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
pub mod presetdata;
/// **新数据世界**：随包 catalog 的释放口 + 说明书 / 交付文件的落点规则
/// （落点 = `catalog.path`，见 [`runtime::paths`]）。
/// 客户端首屏（九条预设读命令）从这里出数 —— 首屏唯一数据源 = catalog
/// （docs/DATA-ARCHITECTURE.md 判据 4；旧世界 `client/` 的 include_str! 铺盘已退役）。
pub mod runtime;

// 工作台（B03）。**默认构建里下面这一行不成立**，所以 `src/workbench/` 整个子树连编译
// 都不会被碰，给用户的二进制里搜不到任何 `wb_` 命令。
// 见 .comate/specs/b03-backstage-workbench/doc.md §4
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
                chrome::apply_native_corners(&win);
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
                        let writer = listener.serve(move |msg| view.apply(&to_ui, msg));
                        for_setup.attach_writer(writer);
                        tracing::info!(root = %root.display(), "后处理通道已就位");
                    }
                    None => tracing::info!("已经有一扇窗在做显示，这一条只当界面"),
                },
                Err(e) => tracing::warn!("后处理通道没摆起来（这一侧不显示进度）：{e}"),
            }

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
        // 「复制后处理脚本」里那段可执行物路径：就是本程序自己（`current_exe()`）
        ipc::get_post_process_exe,
        // 钩子那一趟的三条（切片器导出时）：看进度 / 停 / 答机型不匹配那一问
        ipc::postprocess::get_post_process_run,
        ipc::postprocess::cancel_post_process,
        ipc::postprocess::answer_post_process_mismatch,
        // 预设页（A41）的真后端。读客户端自己的数据根（`appDataDir/presets`）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 <appDataDir>/user/presets-mine）
        ipc::mine::get_user_preset_files,
        ipc::mine::read_user_preset_text,
        // 备注覆盖账 + 改归属（客户端副标题 / 复制出来的那份标机型版本）
        ipc::mine::get_preset_remarks,
        ipc::mine::set_preset_remark,
        ipc::mine::set_user_preset_machine_version,
        // 临时编辑那条链：把官方正文复制进临时文件 → 改 → 另存成用户文件
        ipc::mine::begin_preset_edit,
        ipc::mine::patch_preset_draft,
        ipc::mine::put_preset_draft,
        ipc::mine::discard_preset_draft,
        ipc::mine::commit_preset_draft,
        // 第十层：用户文件管理（改名 / 删除 —— 只动名字或删掉，字节一个不动）
        ipc::mine::rename_user_preset,
        ipc::mine::delete_user_preset,
        // 第十三层：文件外部管理（在 Finder / 资源管理器里选中这一份）
        ipc::mine::reveal_in_folder,
        // 第十一层：另存为一份新的（我的文件 → 我的文件，字节复制）
        ipc::mine::copy_user_preset,
        // 找「我那一份」+ 把校准值写进它（2026-10-08 资源库改判）
        ipc::mine::get_user_copy_for,
        ipc::mine::save_preset_calibration,
        // 对比台 + 官方版本账（2026-10-08）：只在**用户自己的预设**之间对比
        ipc::preset_baseline::get_official_versions,
        ipc::preset_params::read_preset_params,
        ipc::preset_params::save_preset_params,
        // 逐参数「官方更新」（2026-10-09）：读三方账 / 落采用·保持的决定
        ipc::param_sync::get_preset_param_sync,
        ipc::param_sync::apply_preset_param_decisions,
        // 第十二层：通用导入入口（看落点 / 提交；Preset 只是第一个消费者）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
        // 切片器的「生效」= 复制进切片器自己的用户配置目录（2026-10-09 接通）
        ipc::presets::copy_to_slicer,
        // 新数据世界（第一圈）：运行时 catalog，读 `<appDataDir>/catalog.json`
        ipc::catalog::get_runtime_catalog,
        ipc::catalog::get_downloaded_files,
        ipc::catalog::download_runtime_file,
        ipc::catalog::download_runtime_files,
        ipc::catalog::read_downloaded_text,
        // 「复制链接」：与下载管道同一个寻址出口算出来的官方 URL
        ipc::catalog::get_file_url,
        // 报告页：后处理执行报告与历史（gcode_history 的真账，只读）
        ipc::report::get_report_list,
        ipc::report::get_report_detail,
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        // 资源库那一条路（2026-10-08 改判）：官方预设「使用」= 按需取回 + 写成当前使用
        ipc::catalog::fetch_official_preset,
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
        // 「复制后处理脚本」里那段可执行物路径：就是本程序自己（`current_exe()`）
        ipc::get_post_process_exe,
        // 钩子那一趟的三条（切片器导出时）：看进度 / 停 / 答机型不匹配那一问
        ipc::postprocess::get_post_process_run,
        ipc::postprocess::cancel_post_process,
        ipc::postprocess::answer_post_process_mismatch,
        // 客户端命令：**两份清单一字不差**（漏一份就是「原生机能用、工作台构建不能用」）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 <appDataDir>/user/presets-mine）
        ipc::mine::get_user_preset_files,
        ipc::mine::read_user_preset_text,
        // 备注覆盖账 + 改归属（与上面那份清单一字不差 —— 2026-10-09 对齐：这三条
        // 原来只在客户端那份里，工作台构建里发它们会落到 NOT_IMPLEMENTED）
        ipc::mine::get_preset_remarks,
        ipc::mine::set_preset_remark,
        ipc::mine::set_user_preset_machine_version,
        // 临时编辑那条链：把官方正文复制进临时文件 → 改 → 另存成用户文件
        ipc::mine::begin_preset_edit,
        ipc::mine::patch_preset_draft,
        ipc::mine::put_preset_draft,
        ipc::mine::discard_preset_draft,
        ipc::mine::commit_preset_draft,
        // 第十层：用户文件管理（与上面那份清单一字不差）
        ipc::mine::rename_user_preset,
        ipc::mine::delete_user_preset,
        // 第十三层：文件外部管理（与上面那份清单一字不差）
        ipc::mine::reveal_in_folder,
        // 第十一层：另存为一份新的（与上面那份清单一字不差）
        ipc::mine::copy_user_preset,
        // 找「我那一份」+ 把校准值写进它（与上面那份清单一字不差）
        ipc::mine::get_user_copy_for,
        ipc::mine::save_preset_calibration,
        // 对比台 + 官方版本账（与上面那份清单一字不差）
        ipc::preset_baseline::get_official_versions,
        ipc::preset_params::read_preset_params,
        ipc::preset_params::save_preset_params,
        // 逐参数「官方更新」（与上面那份清单一字不差）
        ipc::param_sync::get_preset_param_sync,
        ipc::param_sync::apply_preset_param_decisions,
        // 第十二层：通用导入入口（与上面那份清单一字不差）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
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
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        // 资源库那一条路（2026-10-08 改判）：官方预设「使用」= 按需取回 + 写成当前使用
        ipc::catalog::fetch_official_preset,
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
        // 工作台（doc §6 的新契约）。**配方内容的写只有 wb_apply_draft 一条** ——
        // 其余全是读、查（只读推演）、或一件明确的事（`wb_set_bootstrap` 是单值配置写，
        // 不进制 draft 体系——见 app/mod.rs 那条命令的注释）。
        // 旧那 30 多个命令已全部作废：每个按钮各自写盘的话，撤销、脏计数、
        // 差异列表、状态一致性每一件都要挨个改十几处。
        app::wb_open,
        app::wb_boot,
        app::wb_reload,
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
        // 交付文件清单（2026-10-07）：「发布预设」卡看这次都会写出哪些文件 + 看某一份的原文
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
    //! - `ipc::mine::set_user_preset_machine_version` 等三条只在客户端那份里
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
            let end = after.find(end_mark).expect("generate_handler! 块要以 ]\\n 收尾");
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
                assert!(list.contains(cmd), "{cmd} 不在一份清单里 —— 界面点下去会 NOT_IMPLEMENTED");
            }
        }
    }
}
