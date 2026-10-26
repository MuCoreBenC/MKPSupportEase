// Windows 的 release 构建不要顺带弹一个控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

/// 把**我们自己开的那扇控制台窗口**藏起来（Windows；其他平台是空实现）。
///
/// # 为什么上面那条 `windows_subsystem` 不够
///
/// 它只对 **release** 生效。而调试构建是 console 子系统 —— 切片器里「复制后处理脚本」
/// 那条命令拼的是 `current_exe()`，开发时就是 `target/debug/mkp-support-ease.exe`，
/// 于是每跑一次后处理，用户眼前就闪一个**什么都没有**的黑框
/// （钩子的进度走 TCP、不走 stdout，那个框从头到尾都是空的。作者 2026-10-11 点的）。
///
/// # 为什么是"藏"而不是 `FreeConsole`
///
/// `FreeConsole` 会把控制台真拆掉，之后 `eprintln!` 那一堆写失败会**当场 panic**
/// （`std::io::stderr` 的写错误是 panic，不是忽略）—— 钩子那一路 panic 等于把
/// 记录与退出码一起丢掉，比一个黑框糟得多。`SW_HIDE` 只是不显示，写照样成功。
///
/// # 只藏"我们自己开的那一个"
///
/// `GetConsoleProcessList` 报 1 说明这个控制台是 Windows **为这个进程新建**的
/// （双击 exe、切片器拉起）；报 ≥2 说明我们**附在**别人已有的终端上
/// （`cargo run`、开发时手敲命令），那一份日志是开发者在看的，一个字节都不许动。
///
/// 残留：窗口是进程起来那一刻由系统建的，我们这行代码跑到之前它已经存在了 ——
/// 会有**极短**的一闪（实测几十毫秒）。release 构建连这一闪也没有（上面那条）。
#[cfg(windows)]
#[allow(unsafe_code)] // 三个 kernel32 的声明，别处没有别的办法
fn hide_own_console() {
    use std::ffi::c_void;

    unsafe extern "system" {
        /// 这个控制台上挂着哪几个进程（返回总数；`list` 里最多填 `count` 个）
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
        /// 本进程控制台的窗口句柄
        fn GetConsoleWindow() -> *mut c_void;
        fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
    }
    /* SW_HIDE */
    const SW_HIDE: i32 = 0;

    // SAFETY: 三个都是无副作用的控制台查询/显示开关；`hwnd` 只可能是本进程控制台的
    // 窗口句柄或空指针（空指针已经先判掉），不存在跨线程或生命周期问题。
    unsafe {
        let mut list = [0u32; 4];
        if GetConsoleProcessList(list.as_mut_ptr(), list.len() as u32) != 1 {
            return;
        }
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// 一个可执行物，两种角色：
///
/// - **带 `--Toml/--Gcode`** ⇒ 切片器的**后处理钩子**：干活、把进度写进通道、**跑完就退**
///   （退出码给切片器看；窗口住在另一个常驻进程里，见 `hook_ipc`）；
/// - 什么都不带（或不认识的参数）⇒ 界面。
///
/// **分流必须在这里（建窗口之前）**：钩子那一趟不许开窗、不许初始化 Tauri —— 切片器在等这个
/// 进程退出，开一扇窗口等用户关，就是让它卡在 95%（2026-10-09 实测过两次）。
fn main() -> ExitCode {
    /* 第一件事：我们自己开的那扇空控制台不许露出来（两种角色都算，见 `hide_own_console`） */
    #[cfg(windows)]
    hide_own_console();

    match mkp_support_ease_lib::args::parse(std::env::args_os()) {
        Ok(mkp_support_ease_lib::args::Mode::Hook(job)) => {
            /* 钩子这一趟也要按用户的界面语言说话：切片器那一框里显示的就是 stderr，
            「机型不匹配」那几句既进日志也进用户眼前。没有 `AppHandle`，走无头的根。 */
            mkp_support_ease_lib::runtime::app_state::apply_lang(
                mkp_support_ease_lib::fsx::paths::internal_root_headless()
                    .and_then(|root| mkp_support_ease_lib::runtime::app_state::locale(&root).ok())
                    .flatten(),
            );
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
