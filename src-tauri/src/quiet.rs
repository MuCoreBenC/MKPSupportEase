//! **起子进程时不许弹那扇黑框**（Windows，2026-11-18 作者截图）。
//!
//! # 那个黑框是什么
//!
//! 它不是我画的窗口，是 Windows 给**控制台程序**（`powershell` / `cmd` / 别人的 exe）新开的
//! 控制台。本程序自己是 GUI 子系统（`main.rs` 的 `windows_subsystem = "windows"`），所以
//! 那扇黑框只会出现在"我们又去拉了一个控制台程序"的地方 —— 真机上看到的是：后处理跑完，
//! 旁边冒出一扇 `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`。
//!
//! 根子是 `std::process::Command` 默认按 `CREATE_NEW_CONSOLE` 之外的方式起进程，但
//! **没有把"别开窗"这一位带上**；GUI 程序自己没控制台，子进程就得到一个全新的控制台窗口。
//!
//! # 判据
//!
//! 凡是目标**可能是控制台程序**的 spawn，一律走 [`Quiet::quiet`]（挂 `CREATE_NO_WINDOW`）。
//! 目标是 GUI 程序时这一位不起作用，带着也无害 —— 所以"统一带上"比"逐个判断对方是不是
//! 控制台程序"可靠：后者一点没想全，用户就再看见一次黑框。
//!
//! ★ 与 `ipc::slicer::windows_creation_flags`（`DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`）
//!   不是一回事：那边要的是"把进程从我们这棵树上摘下去"（切片器比我们活得久），
//!   这里要的只是"别开窗"。

/// `CREATE_NO_WINDOW`（Win32 进程创建标志）：子进程不新建控制台窗口。
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 给 `std::process::Command` 加一句 `.quiet()`。
pub trait Quiet {
    /// 起进程时不弹控制台窗口（Windows 生效，其它平台原样返回）。
    fn quiet(&mut self) -> &mut Self;
}

impl Quiet for std::process::Command {
    fn quiet(&mut self) -> &mut Self {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt as _;
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 这一位是 Win32 的常量，写错了就等于没挂（黑框照旧）。
    /// 数值与 `winbase.h` 的 `CREATE_NO_WINDOW` 逐字一致。
    #[test]
    fn the_flag_is_the_one_windows_expects() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }
}
