/*
 * 清译文台账（2026-11-02）。
 *
 * 逃生通道的**命令行那一半**：译文由别的路写好之后（直接改文件、或按
 * `scripts/stale-translations.mjs` 的清单整批改），后端看不见那次"改"，
 * 台账里的标记就永远摘不掉。这条按 `key=labelEn,descJa` 的形式点名清。
 *
 * # 为什么另开一条 bin
 *
 * 清台账要的是「内存与盘对得上」——直接就地 `ParamRegistry::load_from` + 写回，
 * 走的是**与工作台同一条**实现（`clear_stale_translations`），不是第二套判据。
 * 放在 bin 里是因为它是一次性的运维动作（AI / 人偶发用），不该长成 IPC 命令。
 * 界面上的那颗在定义框里（`wb_clear_stale_translations`），两边同一套语义。
 *
 *   cargo run --bin clear-stale --features workbench -- \
 *     wiping.have_wiping_components=descEn,descJa,descTw \
 *     toolhead.offset.y=descEn,descJa,descTw
 */
use mkp_support_ease_lib::presetdata::ParamRegistry;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("用法：clear-stale [--draft] <key>=<格子>[,<格子>…] …");
        eprintln!("例：  clear-stale wiping.have_wiping_components=descEn,descJa");
        eprintln!("      --draft  改工作台的影子树（草稿里也挂着台账时用）");
        return std::process::ExitCode::from(2);
    }
    /*
     * 改哪一棵：默认真源 `presets/`（`paths::presets_root`，与 `presets` 命令同一处定位）。
     *
     * `--draft` 改的是**工作台的影子树**（`<workbench>/.draft/presets`）—— 工作台会话
     * 只写影子，草稿里也挂着台账时，光清真源是白清：一点「放弃」它又从草稿退回来。
     */
    let draft = args.iter().any(|a| a == "--draft");
    let rest: Vec<String> = args.into_iter().filter(|a| a != "--draft").collect();
    if rest.is_empty() {
        eprintln!("--draft 后面也得给 key=格子");
        return std::process::ExitCode::from(2);
    }
    let root = if draft {
        let wb = mkp_support_ease_lib::workbench::paths::real_workbench_root();
        mkp_support_ease_lib::workbench::domain::txn::shadow_root(&wb)
    } else {
        match mkp_support_ease_lib::workbench::paths::presets_root() {
            Some(p) => p,
            None => {
                eprintln!("找不到 <repo>/presets");
                return std::process::ExitCode::from(1);
            }
        }
    };
    let mut reg = match ParamRegistry::load_from(&root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("注册表读不出来：{}", e.message);
            return std::process::ExitCode::from(1);
        }
    };
    let mut failed = false;
    for arg in &rest {
        let Some((key, list)) = arg.split_once('=') else {
            eprintln!("这一条不成样子（要 key=格子,格子）：{arg}");
            failed = true;
            continue;
        };
        let keys: Vec<String> = list
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        match reg.clear_stale_translations(key, &keys) {
            Ok(left) => {
                if left.is_empty() {
                    println!("✓ {key}：台账已清空");
                } else {
                    println!("✓ {key}：还剩 {}", left.join(", "));
                }
            }
            Err(e) => {
                eprintln!("✗ {key}：{}", e.message);
                failed = true;
            }
        }
    }
    if failed {
        eprintln!("有失败项 —— 一处都没写盘（先整批验过再落笔）");
        return std::process::ExitCode::from(1);
    }
    match reg.write_back() {
        Ok(()) => {
            println!(
                "已写回 {}",
                root.join("registry/param_registry.toml").display()
            );
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("写盘失败：{}", e.message);
            std::process::ExitCode::from(1)
        }
    }
}
