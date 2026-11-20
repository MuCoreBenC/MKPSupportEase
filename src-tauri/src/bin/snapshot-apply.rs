//! **发布快照的命令行入口**。
//!
//! 两个动作，都调工作台里同一份内核，不另写逻辑：
//!
//! ```text
//! cargo run --features workbench --bin snapshot-apply -- restore <repo>   # 还原到拍快照时
//! cargo run --features workbench --bin snapshot-apply -- <repo>           # 真编（工作台按钮调起）
//! ```
//!
//! 工作台那颗「模拟发布」按钮把这一支当**独立进程**起，跑
//! `publish_snapshot::run_apply_child()` —— 抬号 / 真编安装包 / 写本地清单。进度全程写
//! `<repo>/target/local-publish-snapshot/apply.json`，所以起它的工作台即使退出，编译照跑完，
//! 重启后也能读到真实阶段。
//!
//! 带 `workbench` feature 才编得出来（见 `Cargo.toml` 的 `required-features`），给用户的
//! 二进制里没有这个入口。

use std::path::PathBuf;
use std::process::ExitCode;

use mkp_support_ease_lib::workbench::app::publish_snapshot;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "用法：snapshot-apply <repo>            真编（工作台「模拟发布」调起）\n\
                   snapshot-apply restore <repo>   还原到拍快照时的状态"
        );
        return ExitCode::SUCCESS;
    }
    if argv.first().map(String::as_str) == Some("restore") {
        let Some(repo) = argv.get(1) else {
            eprintln!("缺仓库根参数。用法：snapshot-apply restore <repo>");
            return ExitCode::from(2);
        };
        return match publish_snapshot::restore(&PathBuf::from(repo)) {
            Ok(()) => {
                println!("已还原到拍快照时的状态。");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("还原失败：{}", e.message);
                ExitCode::FAILURE
            }
        };
    }
    let Some(repo) = argv.first() else {
        eprintln!("缺仓库根参数。用法：snapshot-apply <repo>");
        return ExitCode::from(2);
    };
    publish_snapshot::run_apply_child(PathBuf::from(repo));
    ExitCode::SUCCESS
}
