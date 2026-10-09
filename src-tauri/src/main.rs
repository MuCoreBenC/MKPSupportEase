// Windows 的 release 构建不要顺带弹一个控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

/// 一个可执行物，两种角色：
///
/// - **带 `--Toml/--Gcode`** ⇒ 切片器的**后处理钩子**：跑完就退，退出码给切片器看
///   （首页「复制后处理脚本」复制的那一行指的就是本程序，见 `ipc::get_post_process_exe`）；
/// - 什么都不带（或不认识的参数）⇒ 界面。
///
/// **分流必须在建窗口之前**：钩子那一趟不许开窗、不许初始化 Tauri —— 切片器在等这个进程
/// 退出，开一扇窗口等用户关，就是让它卡在 95%（2026-10-09 实测：那时候本程序还没有钩子角色）。
fn main() -> ExitCode {
    match mkp_support_ease_lib::args::parse(std::env::args_os()) {
        Ok(mkp_support_ease_lib::args::Mode::Hook(job)) => {
            /* 钩子那一趟：**开同一扇窗**把它跑完（进度模态框 + 一颗「停止」，见 `hook_ui`），
            跑完由那一趟带退出码退 —— 切片器在等这个退出码 */
            mkp_support_ease_lib::run_hook(job);
            ExitCode::SUCCESS
        }
        Ok(mkp_support_ease_lib::args::Mode::Gui) => {
            mkp_support_ease_lib::run();
            ExitCode::SUCCESS
        }
        Err(why) => {
            /* 参数错也**不退回界面**：静默开一扇窗会让用户以为后处理跑过了。
            两行：一句错在哪 + 一句正确用法（切片器会把 stderr 整段给用户看） */
            eprintln!("参数错误：{why}");
            eprintln!(
                "用法：mkp-support-ease --Toml <预设.toml> --Gcode <输入.gcode>\n\
                 （切片器里注册的命令串以光秃秃的 --Gcode 结尾，路径由切片器追加）"
            );
            ExitCode::from(mkp_support_ease_lib::hook::EXIT_BAD_INPUT)
        }
    }
}
