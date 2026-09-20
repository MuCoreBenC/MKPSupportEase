// Windows 的 release 构建不要顺带弹一个控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mkp_support_ease_lib::run()
}
