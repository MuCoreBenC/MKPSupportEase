//! `gen-presets`：把配方渲染成预设。**仓库内的开发工具，不进 `.app`**。
//!
//! ```text
//! cargo run -q -p mkpse-preset --bin gen-presets                 # --check（默认）
//! cargo run -q -p mkpse-preset --bin gen-presets -- --write      # 写进 assets/presets/
//! cargo run -q -p mkpse-preset --bin gen-presets -- --print A1:standard
//! ```
//!
//! # 为什么 `--check` 是默认
//!
//! 默认动作应当是**只读的那一个**：这条命令进 `make judge`，而判据不该在跑的时候改仓库。
//! 想写就明说 `--write`。
//!
//! # 这个文件是薄壳
//!
//! 渲染、逐字节比、扫多余产物三件事都在 `preset::generate` 里 ——
//! 工作台（spec `recipe-workbench`）的命令调的是同一套函数。这里只负责认参数与打印。
//!
//! **一处红字的措辞变了**（挪进库的代价，照实记）：入库产物读不到时，
//! 原来那句里有「忘了 `--write`？」，现在库里不提 CLI 的开关名，改成「忘了写盘？」。
//! 绿字与其余红字逐字未变。

use std::path::PathBuf;

use preset::generate::{
    assets_dir, check_all, check_baseline, fixtures_dir, sync_baseline, write_all,
};
use preset::recipe::{Recipe, render};

const RECIPE: &str = include_str!("../../assets/preset_recipes.toml");

enum Mode {
    Check,
    Write,
    Print(String),
    /// K-G0'：产物与对照基线九对九逐字节相同（只读）。
    Baseline,
    /// 把产物同步成对照基线 —— **人看过 diff 之后才该跑**（doc §0 的 ③）。
    SyncBaseline,
}

fn parse_args() -> Result<Mode, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Ok(Mode::Check),
        [flag] if flag == "--check" => Ok(Mode::Check),
        [flag] if flag == "--write" => Ok(Mode::Write),
        [flag] if flag == "--baseline" => Ok(Mode::Baseline),
        [flag] if flag == "--sync-baseline" => Ok(Mode::SyncBaseline),
        [flag, target] if flag == "--print" => Ok(Mode::Print(target.clone())),
        other => Err(format!(
            "不认识的参数：{other:?}\n用法：--check（默认）| --write | --baseline | \
             --sync-baseline | --print <机型>:<变体>"
        )),
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            std::process::ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let mode = parse_args()?;
    let recipe = Recipe::parse(RECIPE).map_err(|e| format!("K-G7 红：配方读不回来：{e}"))?;
    let dir: PathBuf = assets_dir();

    match mode {
        Mode::Print(target) => {
            let (machine, variant) = target
                .split_once(':')
                .ok_or_else(|| format!("`{target}` 不是 `<机型>:<变体>`"))?;
            let text = render(&recipe, machine, variant)
                .map_err(|e| format!("{machine}:{variant} 渲染失败：{e}"))?;
            print!("{text}");
            Ok(format!(
                "（以上是 {machine}:{variant}，{} 行）",
                text.lines().count()
            ))
        }
        Mode::Write => {
            let written = write_all(&recipe, &dir)?;
            Ok(format!("写出 {written} 份产物到 {}", dir.display()))
        }
        Mode::Check => {
            let report = check_all(&recipe, &dir).map_err(|e| format!("K-G7 红：{e}"))?;
            match report.first_diff {
                Some(why) => Err(format!("K-G7 红：{why}")),
                None => Ok(format!(
                    "K-G7 绿：{} 份入库产物与配方生成的结果逐字节相同",
                    report.checked
                )),
            }
        }
        Mode::Baseline => {
            let base = fixtures_dir();
            let report = check_baseline(&dir, &base).map_err(|e| format!("K-G0' 红：{e}"))?;
            match report.first_diff {
                Some(why) => Err(format!(
                    "K-G0' 红：{why}\n\
                     —— 这不一定是错：如果那处改动是你要的，看过 diff 之后跑 `--sync-baseline`"
                )),
                None => Ok(format!(
                    "K-G0' 绿：{} 对（机型:变体）产物与对照基线逐字节相同",
                    report.checked
                )),
            }
        }
        Mode::SyncBaseline => {
            let base = fixtures_dir();
            let synced = sync_baseline(&dir, &base)?;
            Ok(format!(
                "同步了 {synced} 份到对照基线 {} —— 内容相同的没写（mtime 不动）",
                base.display()
            ))
        }
    }
}
