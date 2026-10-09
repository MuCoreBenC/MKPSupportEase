// Windows 的 release 构建不要顺带弹一个控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

/// 一个可执行物，两种角色：
///
/// - **带 `--Toml/--Gcode`** ⇒ 切片器的**后处理钩子**：干活、把进度写进通道、**跑完就退**
///   （退出码给切片器看；窗口住在另一个常驻进程里，见 `hook_ipc`）；
/// - 什么都不带（或不认识的参数）⇒ 界面。
///
/// **分流必须在这里（建窗口之前）**：钩子那一趟不许开窗、不许初始化 Tauri —— 切片器在等这个
/// 进程退出，开一扇窗口等用户关，就是让它卡在 95%（2026-10-09 实测过两次）。
fn main() -> ExitCode {
    match mkp_support_ease_lib::args::parse(std::env::args_os()) {
        Ok(mkp_support_ease_lib::args::Mode::Hook(job)) => {
            mkp_support_ease_lib::hook::run_with_channel(&job)
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
