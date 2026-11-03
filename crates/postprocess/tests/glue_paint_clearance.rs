//! 成品 G-code「落笔高度贴面横移」诊断（按需运行）。
//!
//! 背景见 `docs/GLUE-TRAVEL-Z-SAFETY-2026-10-20.md`：涂胶块里的移动是从切片器
//! 的支撑面路径搬过来的。切片器在两段涂胶之间写的是「空走1（带 Z）→ 空走2（带 Z）
//! → 一条微小涂胶行」，`delete_wipe` 只保留 `WIPE_END` 之后第一条空走，第二条被
//! 静默丢弃；发射层又只把标记后紧邻的一条交给抬升 ⇒ 涂胶行从空走1 的终点直飞
//! 自己的终点，这段位移在**落笔高度**完成，笔尖压着模型表面走。
//!
//! 实测样本（`伊埃斯-bambu-20261011-最终`，A1 / offset_z 3.95 / inter_model 2.0）：
//! paint 块内 >20mm 横移 287 处，其中 **78 处落在落笔高度**，最长 65mm。
//!
//! 运行方式（样本体积大，不入仓）：
//!
//! ```text
//! MKP_GLUE_SAMPLE="<导出件.gcode>" cargo test -p mkpse-postprocess \
//!     --test glue_paint_clearance -- --nocapture
//! ```
//!
//! 没有设置环境变量时整条测试直接跳过，不影响 CI。

use std::path::PathBuf;

/// 「长距离」口径，只用于报告，不参与任何生产判断。
const LONG_MM: f64 = 5.0;

fn sample_path() -> Option<PathBuf> {
    let raw = std::env::var_os("MKP_GLUE_SAMPLE")?;
    let path = PathBuf::from(raw);
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

#[derive(Debug, Clone, Copy)]
struct Landing {
    line: usize,
    dist: f64,
    z: f64,
    layer_z: f64,
}

/// 扫出 paint 块内、**笔在落笔状态**时的长距离移动。
///
/// 「落笔状态」由 `G1 Z` 的升降方向判定：Z 抬高即认为抬笔，Z 降低即认为落笔。
/// 不看注释、不看 `E`、不用固定高度差——层高与笔长都可能是变量。
fn landing_height_travels(lines: &[String]) -> (Vec<Landing>, usize) {
    let (mut x, mut y, mut z) = (0.0f64, 0.0f64, f64::NAN);
    let (mut in_paint, mut landed) = (false, false);
    let mut layer_z = f64::NAN;
    let mut out = Vec::new();
    let mut paint_blocks = 0usize;

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.starts_with(";MKP_BEGIN paint") {
            in_paint = true;
            landed = false;
            paint_blocks += 1;
            layer_z = line
                .split("z=")
                .nth(1)
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(f64::NAN);
            continue;
        }
        if line.starts_with(";MKP_END paint") || line.starts_with(";MKP_BEGIN swap") {
            in_paint = false;
            continue;
        }

        let code = line.split(';').next().unwrap_or("").trim();
        let first = code.split_whitespace().next().unwrap_or("");
        if !matches!(first, "G0" | "G1" | "G2" | "G3") {
            continue;
        }
        let (mut nx, mut ny, mut nz) = (None, None, None);
        for tok in code.split_whitespace().skip(1) {
            let value = tok.get(1..).and_then(|v| v.parse::<f64>().ok());
            match (tok.chars().next(), value) {
                (Some('X'), Some(v)) => nx = Some(v),
                (Some('Y'), Some(v)) => ny = Some(v),
                (Some('Z'), Some(v)) => nz = Some(v),
                _ => {}
            }
        }
        if let Some(v) = nz {
            if !z.is_nan() {
                if v > z + 1e-9 {
                    landed = false;
                } else if v < z - 1e-9 {
                    landed = true;
                }
            }
            z = v;
        }
        let nx = nx.unwrap_or(x);
        let ny = ny.unwrap_or(y);
        if in_paint && landed && !z.is_nan() {
            let dist = ((nx - x).powi(2) + (ny - y).powi(2)).sqrt();
            if dist > LONG_MM {
                out.push(Landing {
                    line: idx + 1,
                    dist,
                    z,
                    layer_z,
                });
            }
        }
        x = nx;
        y = ny;
    }
    (out, paint_blocks)
}

#[test]
fn export_has_no_long_travel_at_landing_height() {
    let Some(path) = sample_path() else {
        println!("未设置 MKP_GLUE_SAMPLE，跳过（样本不入仓）");
        return;
    };
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到样本 {}: {e}", path.display()));
    let lines: Vec<String> = text
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_string)
        .collect();
    let (bad, blocks) = landing_height_travels(&lines);

    assert!(
        blocks >= 3,
        "只扫到 {blocks} 个涂胶块 —— 判据没接上（不是涂胶件？）"
    );
    println!(
        "涂胶块 {blocks} 个；落笔高度 >{LONG_MM}mm 横移 {} 处（修复前基线应为 118）",
        bad.len()
    );
    for item in bad.iter().take(20) {
        println!(
            "  第 {} 行：{:.1}mm，Z={:.2}（层顶 {:.2}）",
            item.line, item.dist, item.z, item.layer_z
        );
    }
    assert!(
        bad.is_empty(),
        "落笔高度还有 {} 处 >{LONG_MM}mm 的横移，笔尖会压着模型表面走",
        bad.len()
    );
}
