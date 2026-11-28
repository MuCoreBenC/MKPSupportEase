//! 擦料塔的 Z 连续性：**从首层底盘到最后一次涂胶层，每层一片、片与片之间不许留空**。
//!
//! 判据的来历：塔片原来按「名义层高」（fast 档 0.28）自己往上记账，模型层高只有 0.2，
//! 于是大约每 1.4 层才出一片；而塔片又打在模型层顶上、给料按名义层高算 —— 相邻两片
//! 之间就留下 0.1~0.26mm 的空气缝（实测一份 323 层的样件：141 对里有 31 对悬空、
//! 整根塔累计 5.38mm 空隙）。预览里看就是一摞互不相连的薄环。
//!
//! 这里钉四件事：
//! 1. 模型层除了首层（首层是底盘）到最后一次涂胶层，层**层**都有塔片；
//! 2. 塔片的 Z 就是本层模型层的 Z（容差与 `marks::check_tower_prints_at_layer_z` 同源）；
//! 3. 每片声明的层高 = 它与上一片的 Z 差 —— 不是名义值；
//! 4. 最后一次涂胶层之后不再出片（塔只服务涂胶与换笔，那一片就是收笔后打的那片）。
//!
//! 反空转：抽掉中间一整层（模型从 1.1 直接跳到 1.5），那一片就比上限厚，
//! 内核必须报出「塔片比上限还厚」那条警告。没有它的话，上面几条在"压根没出片"的
//! 输入上也会绿。

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mkpse-pp")
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

/// 跑一次 `run`，返回（产物文本，stdout）。
fn run(input: &std::path::Path, tag: &str) -> (String, String) {
    let out_path = std::env::temp_dir().join(format!("mkpse-pp-tower-{tag}.gcode"));
    let _ = std::fs::remove_file(&out_path);
    let out: Output = Command::new(bin())
        .arg("run")
        .arg(input)
        .arg("-c")
        .arg(config_a1())
        .arg("-o")
        .arg(&out_path)
        .output()
        .expect("CLI 二进制必须能起来");
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应当成功；stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&out_path).expect("输出文件应当存在");
    // 统计（含「警告 N 条」）走 stderr；两股都收，判据才看得到。
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    (text, said)
}

/// 一层塔片：本层 Z、片头声明的层高、塔自己落笔的 Z。
#[derive(Debug)]
struct Slice {
    layer_z: f64,
    declared_lh: f64,
    tower_z: f64,
}

/// 从成品里按顺序取出每一层与其塔片。
///
/// 片高写在**块头**（`;Tower_Layer_Gcode` 之前那一条 `; LAYER_HEIGHT:`），
/// 不是块内 —— 块内第一条已经是模板自己的 `G92 E0`。
fn tower_slices(gcode: &str) -> Vec<Slice> {
    let mut out = Vec::new();
    let mut layer_z: Option<f64> = None;
    let mut pending_lh: Option<f64> = None;
    let mut inside = false;
    let mut tower_z = None;

    for line in gcode.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("; Z_HEIGHT:") {
            layer_z = rest.trim().parse::<f64>().ok();
            continue;
        }
        if trimmed == ";Tower_Layer_Gcode" {
            inside = true;
            tower_z = None;
            continue;
        }
        if trimmed == ";Tower_Layer_Gcode Finished" {
            inside = false;
            out.push(Slice {
                layer_z: layer_z.expect("塔片必须在某一层之内"),
                declared_lh: pending_lh.expect("塔片头必须写层高"),
                tower_z: tower_z.expect("塔片必须写 `;Tower Z`"),
            });
            continue;
        }
        if inside {
            if trimmed.contains(";Tower Z") {
                tower_z = trimmed
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .find_map(|tok| tok.strip_prefix('Z'))
                    .and_then(|v| v.parse::<f64>().ok());
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("; LAYER_HEIGHT:") {
            pending_lh = rest.trim().parse::<f64>().ok();
        }
    }
    out
}

/// 模型层的 Z 序列（`; Z_HEIGHT:` 那一列）。
fn layer_zs(gcode: &str) -> Vec<f64> {
    gcode
        .lines()
        .filter_map(|l| l.trim().strip_prefix("; Z_HEIGHT:"))
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .collect()
}

/// 塔片层高上限：0.7 × 喷嘴。fixture 是 A1（0.4 喷嘴）⇒ 0.28。
const MAX_LH: f64 = 0.28;

/// 最后一次涂胶所在层的层顶（`;MKP_BEGIN paint … z=<层顶>` 的最大值）。
fn last_glue_z(gcode: &str) -> f64 {
    gcode
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix(";MKP_BEGIN paint ")?;
            rest.split_whitespace()
                .find_map(|tok| tok.strip_prefix("z="))
                .and_then(|v| v.parse::<f64>().ok())
        })
        .fold(0.0_f64, f64::max)
}

#[test]
fn every_layer_gets_one_slice_and_slices_touch() {
    let (gcode, stdout) = run(&golden_input(), "same");
    let layers = layer_zs(&gcode);
    let slices = tower_slices(&gcode);
    let glue_top = last_glue_z(&gcode);

    assert!(layers.len() > 10, "样本层数太少，判据没有意义");
    assert!(glue_top > 0.0, "样本里应当有涂胶事件");
    // 首层是底盘（`Trapezoidal Sheath`）；到最后一次涂胶层为止每层一片，之后停。
    let covered: Vec<f64> = layers
        .iter()
        .copied()
        .filter(|z| *z <= glue_top + 1e-3)
        .collect();
    assert_eq!(
        slices.len(),
        covered.len() - 1,
        "除首层底盘外、到最后一次涂胶层（{glue_top}）每层都该有塔片（层 {} / 片 {}）",
        covered.len(),
        slices.len()
    );

    for (i, s) in slices.iter().enumerate() {
        assert!(
            (s.tower_z - s.layer_z).abs() <= 0.01,
            "第 {} 片落在 Z={}，而本层是 {}",
            i + 2,
            s.tower_z,
            s.layer_z
        );
        assert!(
            s.layer_z <= glue_top + 1e-3,
            "最后涂胶层 {glue_top} 之后不该再有塔片，却在层 {} 出了一片",
            s.layer_z
        );
    }
    // 最后一片就是最后涂胶层顶（收笔后打的那片塔）。
    let last = slices.last().expect("至少要有塔片");
    assert!(
        (last.tower_z - glue_top).abs() <= 1e-3,
        "塔顶应停在最后涂胶层顶 {glue_top}，实际 {}",
        last.tower_z
    );

    for pair in slices.windows(2) {
        let dz = pair[1].tower_z - pair[0].tower_z;
        assert!(
            dz > 0.0,
            "塔片 Z 必须往前走：{} → {}",
            pair[0].tower_z,
            pair[1].tower_z
        );
        assert!(
            dz <= MAX_LH + 1e-6,
            "两片之间隔了 {dz}mm，超过 0.7×喷嘴（{MAX_LH}）—— 这就是「空出来」的那种缝"
        );
        assert!(
            (pair[1].declared_lh - dz).abs() <= 1e-3,
            "Z={} 那一片声明 {:.4}mm，实际与上一片差 {:.4}mm",
            pair[1].tower_z,
            pair[1].declared_lh,
            dz
        );
    }

    assert!(
        stdout.contains("警告 0 条"),
        "这一份不该有警告；stdout:\n{stdout}"
    );
}

/// 抽掉中间几层造出台阶（可变层高/多模型合并后就是这种不规则 Z 差）。
///
/// 期望：塔**在层内补片**把台阶填平 —— 不是一片打满（那会把塔片拉虚、层间留缝），
/// 也不是报一条告警了事。补出来的中间片不对应任何模型层，这是允许的：
/// 预览层表会因此多出"补片数"那么多层，但每层收尾仍然落在层顶。
#[test]
#[allow(clippy::disallowed_methods)] // 写一份改过的输入到临时目录：判据只跑一次，不碰仓库
fn a_thick_gap_is_bridged_by_extra_slices() {
    // (抽掉哪几层, 台阶, 应当补成几片)：0.28 上限下 0.4 → 2 片、0.6 → 3 片。
    let cases: [(&[&str], f64, usize); 2] = [
        (&["; Z_HEIGHT: 1.3"], 0.4, 2),
        (&["; Z_HEIGHT: 1.3", "; Z_HEIGHT: 1.5"], 0.6, 3),
    ];
    for (removed, gap, want) in cases {
        let (gcode, stdout) =
            run_on_layers_removed(removed, &format!("gap{}", (gap * 10.0) as i32));
        assert!(
            stdout.contains("警告 0 条"),
            "{gap} 的台阶应当靠补片解决，不该报塔层高告警；stdout:\n{stdout}"
        );

        let slices = tower_slices(&gcode);
        let from = 1.1;
        let to = 1.1 + gap;
        let bridged: Vec<&Slice> = slices
            .iter()
            .filter(|s| s.tower_z > from + 1e-6 && s.tower_z <= to + 1e-6)
            .collect();
        assert_eq!(
            bridged.len(),
            want,
            "{gap} 的台阶（上限 {MAX_LH}）应当补成 {want} 片：{bridged:?}"
        );
        for pair in bridged.windows(2) {
            let dz = pair[1].tower_z - pair[0].tower_z;
            assert!(
                dz > 0.0 && dz <= MAX_LH + 1e-6,
                "{gap} 台阶里两片之间是 {dz}mm"
            );
            assert!(
                (pair[1].declared_lh - dz).abs() <= 1e-3,
                "第 {} 片声明 {:.4}mm 与实际步距 {dz:.4}mm 不一致",
                pair[1].tower_z,
                pair[1].declared_lh
            );
        }
        let last = bridged.last().expect("台阶里必须有塔片");
        assert!(
            (last.tower_z - to).abs() <= 1e-3,
            "台阶收尾必须落回层顶 {to}，实际 {}",
            last.tower_z
        );
    }
}

/// 把指定的 `; Z_HEIGHT: …` 那一整层抽掉，写成临时输入再跑一遍。
#[allow(clippy::disallowed_methods)]
fn run_on_layers_removed(marks: &[&str], tag: &str) -> (String, String) {
    let raw = std::fs::read_to_string(golden_input()).expect("输入 fixture 应当存在");
    let lines: Vec<&str> = raw.split_inclusive('\n').collect();

    // 每命中一层就把它整块（`; Z_HEIGHT` 到下一层的 `; Z_HEIGHT` 之前）划掉。
    let mut drop = vec![false; lines.len()];
    for mark in marks {
        let start = lines
            .iter()
            .position(|l| l.trim() == *mark)
            .unwrap_or_else(|| panic!("样本里应当有一层 {mark}"));
        let end = lines[start + 1..]
            .iter()
            .position(|l| l.trim().starts_with("; Z_HEIGHT:"))
            .map(|i| start + 1 + i)
            .expect("抽掉的那一层后面还得有下一层");
        for slot in drop.iter_mut().take(end).skip(start) {
            *slot = true;
        }
    }

    let mut patched = String::with_capacity(raw.len());
    for (i, line) in lines.iter().enumerate() {
        if !drop[i] {
            patched.push_str(line);
        }
    }

    // 名字不能与 `run()` 的产出撞（那个以 tag 命名）：撞了就会把自己删掉。
    let input = std::env::temp_dir().join(format!("mkpse-pp-tower-{tag}-in.gcode"));
    std::fs::write(&input, patched).expect("写临时输入");
    run(&input, tag)
}
