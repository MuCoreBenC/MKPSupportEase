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

pub mod chrome;
pub mod error;
pub mod fsx;
pub mod ipc;
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
/// **新数据世界**：随包 catalog 的释放口 + 下载区/说明书的落点规则。
/// 客户端首屏（九条预设读命令）从这里出数 —— 首屏唯一数据源 = catalog
/// （docs/DATA-ARCHITECTURE.md 判据 4；旧世界 `client/` 的 include_str! 铺盘已退役）。
pub mod runtime;

// 后厨工作台（B03）。**默认构建里下面这一行不成立**，所以 `src/workbench/` 整个子树连编译
// 都不会被碰，给用户的二进制里搜不到任何 `wb_` 命令。
// 见 .comate/specs/b03-backstage-workbench/doc.md §4
//
// （这里刻意用行注释：块注释在 Rust 里是可嵌套的，正文里出现 `/` 加 `*` 会开一个新注释，
// 而路径通配写法很容易写出那两个字符。）
#[cfg(feature = "workbench")]
pub mod workbench;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    with_commands(tauri::Builder::default())
        /* 系统文件选择器（第十二层：通用导入入口的"选择文件"那一半）。
        权限只开 `dialog:allow-open`（默认 capability），别的一律不给 */
        .plugin(tauri_plugin_dialog::init())
        /* 在文件管理器里显示（第十三层）。**只在 Rust 侧调**（我们自己的命令体里），
        所以不需要给它开任何 capability —— 前端够不着它的命令面 */
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
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

            /* 运行时 catalog：随包那份释放进内部根 + 建出空的下载区。
            这是安装包 → 用户本地（层② → 层③）的唯一铺盘：客户端首屏的全部定义
            （机型 / 资产 / 套餐 / 字段定义 / 布局）都从这一份出数。盘上已有且一致就
            一个字节不动；不同（升级）就旧份归档、新份生效（runtime::release）。
            失败只告警不挡启动：界面会显示「读不到说明书」，那是能据以行动的状态。 */
            match fsx::paths::internal_root(&handle)
                .and_then(|root| runtime::release::release_catalog(&root))
            {
                Ok(r) => tracing::info!(report = %r.summary(), "运行时 catalog 已就位"),
                Err(e) => tracing::warn!("运行时 catalog 没就位：{e}"),
            }

            /* 窗口外观：原生圆角 + 让 AppKit 按统一工具栏那一档摆红绿灯。
            两件事都只在 macOS 上有意义，失败都只打日志 —— 外观问题不该挡启动 */
            if let Some(win) = app.get_webview_window("main") {
                chrome::install_unified_toolbar(&win);
                chrome::apply_native_corners(&win);
            } else {
                eprintln!("[setup] 找不到 main 窗口，跳过窗口外观");
            }

            /* 后厨工作台的第二个窗口。**刻意在运行时建，而不是写进 tauri.conf.json**：
            配置里的 windows 数组是整体覆盖的，把工作台窗口写进一份"给工作台用的配置"
            就得把 main 窗口也抄一遍 —— 两份声明迟早漂移。写在这里，窗口的存在与
            feature 严格同生共死，不需要任何一份配置去声明它。
            开不出来只警告不中止：工作台开不出来是开发者的事，不该让客户端窗口也起不来 */
            #[cfg(feature = "workbench")]
            if let Err(e) = workbench::open_window(&handle) {
                tracing::warn!("工作台窗口开不出来：{e}");
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
        ipc::get_preset,
        ipc::save_offsets,
        ipc::get_calib_models,
        ipc::open_model,
        // 预设页（A41）的真后端。读客户端自己的数据根（`appDataDir/presets`）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 Documents/SupportEase/presets-mine）
        ipc::mine::get_user_preset_files,
        ipc::mine::read_user_preset_text,
        // 临时编辑那条链：把官方正文复制进临时文件 → 改 → 另存成用户文件
        ipc::mine::begin_preset_edit,
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
        // 第十二层：通用导入入口（看落点 / 提交；Preset 只是第一个消费者）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
        // 新数据世界（第一圈）：运行时 catalog，读 `<appDataDir>/catalog.json`
        ipc::catalog::get_runtime_catalog,
        ipc::catalog::get_downloaded_files,
        ipc::catalog::download_runtime_file,
        ipc::catalog::download_runtime_files,
        ipc::catalog::read_downloaded_text,
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        ipc::catalog::clear_active_preset,
        ipc::catalog::get_stale_files,
        // 这一份我们认得出是哪一版吗（第 6 层：SHA 报警）
        ipc::catalog::get_delivery_trust,
        ipc::catalog::get_archived_files,
        ipc::catalog::read_archived_text,
        ipc::catalog::check_remote_update,
        ipc::catalog::apply_remote_update,
        ipc::catalog::get_preset_source,
        ipc::catalog::set_preset_source,
    ])
}

#[cfg(feature = "workbench")]
fn with_commands(b: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    use workbench::app;
    b.invoke_handler(tauri::generate_handler![
        ipc::get_preset,
        ipc::save_offsets,
        ipc::get_calib_models,
        ipc::open_model,
        // 客户端命令：**两份清单一字不差**（漏一份就是「原生机能用、工作台构建不能用」）
        ipc::presets::get_machines,
        ipc::presets::get_version_files,
        ipc::presets::get_preset_files,
        ipc::presets::get_menu,
        ipc::presets::get_param_meta,
        ipc::presets::get_machine_params,
        ipc::presets::get_local_files,
        // 用户线（用户自己的预设，住 Documents/SupportEase/presets-mine）
        ipc::mine::get_user_preset_files,
        ipc::mine::read_user_preset_text,
        // 临时编辑那条链：把官方正文复制进临时文件 → 改 → 另存成用户文件
        ipc::mine::begin_preset_edit,
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
        // 第十二层：通用导入入口（与上面那份清单一字不差）
        ipc::import::stage_import,
        ipc::import::commit_import,
        ipc::presets::get_slicer_copied,
        // 新数据世界（第一圈）：与上面那份清单保持一字不差
        ipc::catalog::get_runtime_catalog,
        ipc::catalog::get_downloaded_files,
        ipc::catalog::download_runtime_file,
        ipc::catalog::download_runtime_files,
        ipc::catalog::read_downloaded_text,
        ipc::catalog::get_active_preset,
        ipc::catalog::apply_active_preset,
        ipc::catalog::clear_active_preset,
        ipc::catalog::get_stale_files,
        // 这一份我们认得出是哪一版吗（第 6 层：SHA 报警）
        ipc::catalog::get_delivery_trust,
        ipc::catalog::get_archived_files,
        ipc::catalog::read_archived_text,
        ipc::catalog::check_remote_update,
        ipc::catalog::apply_remote_update,
        ipc::catalog::get_preset_source,
        ipc::catalog::set_preset_source,
        // 后厨工作台（doc §6 的新契约）。**写只有 wb_apply_draft 一条** ——
        // 其余全是读、查（只读推演）、或一件明确的事。
        // 旧那 30 多个命令已全部作废：每个按钮各自写盘的话，撤销、脏计数、
        // 差异列表、状态一致性每一件都要挨个改十几处。
        app::wb_open,
        app::wb_boot,
        app::wb_reload,
        app::wb_book,
        app::wb_registry,
        // 状态词的唯一出处。开场取一次，前端按枚举值查 ——
        // 不给它的话，同一个词会在 TSX 里再写一遍（doc §13）
        app::words::wb_words,
        // 「机型与版本」那一页：读写 presets/machines/*.toml。
        // **不走 wb_apply_draft** —— 清单与参数值不共用状态机（见 app/machines.rs 头注）
        app::machines::wb_machines,
        // 「资产库」：读 `presets/assets.toml`（b05 Task 8），P4 起带三轴派生与筛选；
        // 删除走数据层的反查守卫（有人引用整次拒绝）
        app::assets::wb_assets,
        app::assets::wb_asset_usage,
        app::assets::wb_remove_asset,
        // 「套餐管理」：`presets/bundles.toml` 是套餐唯一真源（Task 13.6）。
        // P4 起：读视图带指向关系（一版一套）+ 换文件清单（即时落盘，不走参数草稿）
        app::bundles::wb_bundles,
        app::bundles::wb_set_bundle_refs,
        // 交付残留（b05 Task 13.4/13.5）：查询清单 + 显式清理（进 .trash 回收）
        app::build::wb_dist_strays,
        app::build::wb_clean_dist_strays,
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
        app::build::wb_preview_toml,
        app::build::wb_generate,
        app::build::wb_revert_preview,
        app::build::wb_publish,
    ])
}
