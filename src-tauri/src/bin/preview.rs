//! **本地预览交付的命令行入口**。
//!
//! 它调的就是工作台那颗按钮背后的事务（`workbench::app::preview_tx::deliver`），
//! 一行逻辑都不另写 —— 按钮和 CLI 必须是同一套检查、提交、构建、打 tag、记账、滚动。
//!
//! ```text
//! cargo run --bin preview -- deliver             # 预演：只说会做什么，一个字节都不写
//! cargo run --bin preview -- deliver --confirm   # 真的执行（人已明确授权这一次）
//! cargo run --bin preview -- recover             # 预演：上一趟停在哪儿
//! cargo run --bin preview -- recover --confirm   # 接着把上一趟走完
//! ```
//!
//! ★ **默认是预演**：不带 `--confirm` 不会提交、不会打 tag、不会滚版本。
//!   一次授权只对这一次有效，不构成以后每次发布的许可。

use std::process::ExitCode;

use mkp_support_ease_lib::workbench::app::preview_tx;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cmd = argv.first().map(String::as_str).unwrap_or("deliver");
    if matches!(cmd, "-h" | "--help" | "help") {
        println!(
            "用法：preview <deliver|recover> [--confirm]\n\
             不带 --confirm 只预演：会做什么、不会写任何东西。"
        );
        return ExitCode::SUCCESS;
    }
    let confirm = argv.iter().any(|a| a == "--confirm");

    let result = match cmd {
        "deliver" => preview_tx::deliver(confirm),
        "recover" => preview_tx::recover(confirm),
        other => {
            eprintln!("不认识的子命令：{other}（只认 deliver / recover）");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(r) => {
            println!("阶段：{}", r.stage);
            println!("版本：{}", r.version);
            if let Some(c) = &r.commit {
                println!("提交：{c}");
            }
            println!("tag ：{}", r.tag);
            println!("下一版：{}", r.next_version);
            println!("{}", r.message);
            if !confirm {
                println!("\n（这是预演，什么都没写。确认执行请加 --confirm）");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.message);
            if let Some(d) = &e.detail {
                eprintln!("{d}");
            }
            ExitCode::FAILURE
        }
    }
}
