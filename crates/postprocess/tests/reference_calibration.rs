//! Task 9.5 判据：校准参考输入/输出对逐字节比对（tests/reference/calibration/）。
//!
//! 输入 JSON 自包含（IR 为 FillDefaults 后快照 + BBox + 质心 + dwell），由
//! Task 4.3 从旧仓库导出。证据等级：对照当前 Go 实现的输出（见
//! tests/reference/README.md），Go 若有 bug 此处逐字节复制。

use postprocess::ir::Ir;
use postprocess::postproc::disk::{BBox, CentroidResult, generate_calibration_gcode_with_centroid};
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn repo_relative(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CalibInput {
    mode: String,
    machine_type: String,
    #[serde(rename = "ZDwellSec")]
    z_dwell_sec: i64,
    #[serde(rename = "XYDwellSec")]
    xy_dwell_sec: i64,
    #[serde(rename = "BBox", default)]
    bbox: Option<GBBoxShadow>,
    centroid: CentroidShadow,
    #[serde(rename = "IR")]
    ir: Ir,
}

#[derive(Deserialize)]
struct GBBoxShadow {
    #[serde(rename = "XMin")]
    x_min: f64,
    #[serde(rename = "XMax")]
    x_max: f64,
    #[serde(rename = "YMin")]
    y_min: f64,
    #[serde(rename = "YMax")]
    y_max: f64,
}

#[derive(Deserialize, Default)]
struct CentroidShadow {
    #[serde(rename = "CentroidX", default)]
    centroid_x: f64,
    #[serde(rename = "CentroidY", default)]
    centroid_y: f64,
    #[serde(rename = "Valid", default)]
    valid: bool,
}

fn join_lines(lines: &[String]) -> String {
    let mut out = String::new();
    for l in lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

fn check_mode(mode: &str) {
    let input_text = std::fs::read_to_string(repo_relative(&format!(
        "tests/reference/calibration/{mode}.input.json"
    )))
    .unwrap();
    let expected = std::fs::read_to_string(repo_relative(&format!(
        "tests/reference/calibration/{mode}.expected.gcode"
    )))
    .unwrap();
    let input: CalibInput = serde_json::from_str(&input_text)
        .unwrap_or_else(|e| panic!("{mode}.input.json 反序列化失败: {e}"));

    let bbox: Option<BBox> = input.bbox.map(|b| BBox {
        x_min: b.x_min,
        x_max: b.x_max,
        y_min: b.y_min,
        y_max: b.y_max,
    });
    let centroid = CentroidResult {
        centroid_x: input.centroid.centroid_x,
        centroid_y: input.centroid.centroid_y,
        valid: input.centroid.valid,
    };
    let got = generate_calibration_gcode_with_centroid(
        &input.mode,
        &input.machine_type,
        &input.ir,
        bbox.as_ref(),
        centroid,
        input.z_dwell_sec,
        input.xy_dwell_sec,
    )
    .unwrap_or_else(|e| panic!("{mode}: 生成失败: {e}"))
    .unwrap_or_else(|| panic!("{mode}: 参考输出非空，生成器却返回空"));

    let got_text = join_lines(&got);
    if got_text != expected {
        let got_lines: Vec<&str> = got_text.lines().collect();
        let want_lines: Vec<&str> = expected.lines().collect();
        let mut first_diff = None;
        for (i, (g, w)) in got_lines.iter().zip(want_lines.iter()).enumerate() {
            if g != w {
                first_diff = Some((i + 1, *w, *g));
                break;
            }
        }
        panic!(
            "{mode}: 输出与参考不一致（got {} 行 / want {} 行），首差异：{:?}",
            got_lines.len(),
            want_lines.len(),
            first_diff
        );
    }
}

#[test]
fn calibration_matches_go_reference_all_modes() {
    for mode in ["ZOffset", "Precise", "Rough", "Repetition"] {
        check_mode(mode);
    }
}

/// New 变体（BBox 动态定位）的字节级参考。
///
/// 导出方式与上面四份同源但**不是同一次**：这三份是 2026-09-08 从
/// `/Users/wzy/projects/mkpse-workspace/mkpse-next_v3/mkpsupporte`（go 1.26.4）
/// 用一次性测试导出的，细节见 tests/reference/README.md。
///
/// 输入是把 `Precise/Rough/ZOffset.input.json` 的 `Safety.XYCalibration` /
/// `ZCalibration` 改成 `"new"`，再补上**平移过的** BBox 与质心 —— 平移是刻意的：
/// 真机样例恰好摆在机型默认校准位，不平移的话 New 与经典分支的坐标完全重合，
/// 参考就验不出「原点跟着模型走」这件事。
#[test]
fn calibration_new_variants_match_go_reference() {
    for mode in ["ZOffset.new", "Precise.new", "Rough.new"] {
        check_mode(mode);
    }
}
