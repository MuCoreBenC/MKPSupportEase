//! **我是被谁拉起来的**（2026-10-20）：后处理钩子模式下，**父进程就是那一趟切片的
//! Bambu Studio** —— 它是"这一盘是哪个切片器切出来的"唯一不靠猜的答案。
//!
//! # 为什么需要它
//!
//! BBS 会**多开**（实测三份），而改完 MKP 参数之后要引导用户回"**我刚才切的那一份**"
//! 去重新切片。按"窗口排在最前那份"挑是不准的：用户可能正切着 A 份、手上却点着 B 份，
//! 或者几份都最大化叠在一起 —— 于是卡片指向了不该指的那一份。
//!
//! ⇒ 钩子这一趟把**父进程 pid** 记进执行记录（`archive::Meta::slicer_pid`），引导层
//! 读最近一条记录就知道"最后处理用的是哪个 BBS"（校验它还在跑 ⇒ 用它；不在 ⇒ 退回
//! z 序最前那份）。
//!
//! # 边界
//!
//! **只读**：只问进程树，不发信号、不碰父进程的内存、不改任何东西。
//! 读不到（系统不给）就 `None` —— 记 `null` 而不是编一个。

/// 父进程号（`None` = 读不到）。
///
/// ★ **1 号（init / launchd 那一族）不算**：那种父进程说明我们是**被界面启动**的，
/// 不是被切片器拉起来的钩子 —— 记它进去只会让引导指向一个不相干的进程。
pub fn pid() -> Option<i32> {
    #[cfg(target_os = "windows")]
    {
        win::parent_pid()
    }
    #[cfg(not(target_os = "windows"))]
    {
        /* macOS / Linux：`getppid()`。libc 的符号这两边都在，声明一下就能用。 */
        extern "C" {
            fn getppid() -> i32;
        }
        let ppid = unsafe { getppid() };
        (ppid > 1).then_some(ppid)
    }
}

#[cfg(target_os = "windows")]
mod win {
    #[repr(C)]
    struct ProcessEntry {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    impl Default for ProcessEntry {
        fn default() -> Self {
            /* 全零对这块 POD 是合法初值（`dw_size` 下面立刻填成 sizeof） */
            unsafe { std::mem::zeroed() }
        }
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
        fn CloseHandle(handle: isize) -> i32;
        fn GetCurrentProcessId() -> u32;
    }

    /// `TH32CS_SNAPPROCESS`
    const SNAP_PROCESS: u32 = 0x2;

    /// 本进程的父进程：在进程快照里找**自己**那一项，读它的 `th32ParentProcessID`。
    ///
    /// ★ 为什么不是"读命令行猜父进程"：快照是一次系统调用、只读、不依赖任何格式约定。
    pub(super) fn parent_pid() -> Option<i32> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(SNAP_PROCESS, 0) };
        if snapshot == -1 {
            return None;
        }
        let mut entry = ProcessEntry {
            dw_size: std::mem::size_of::<ProcessEntry>() as u32,
            ..Default::default()
        };
        let mut found: Option<i32> = None;
        let mine = unsafe { GetCurrentProcessId() };
        unsafe {
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    if entry.th32_process_id == mine {
                        let parent = entry.th32_parent_process_id as i32;
                        /* 系统 idle / 不存在 ⇒ 不当父进程 */
                        if parent > 0 {
                            found = Some(parent);
                        }
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试进程一定有个父进程（cargo / IDE 之类）—— 拿不到才该怀疑实现写错了。
    #[test]
    fn a_running_process_has_a_parent() {
        assert!(pid().is_some(), "连父进程都读不到，实现有问题");
    }

    /// 系统那两个特殊进程（0 / 1）不该被当成"父进程"报出来 ——
    /// 那是"我们是被界面启动的、不是被切片器拉起来的钩子"那一档。
    #[test]
    fn the_parent_is_never_the_idle_or_init_process() {
        assert!(pid().is_none_or(|p| p > 1));
    }
}
