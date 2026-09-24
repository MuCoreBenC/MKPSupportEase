//! 真实 G-code 片段作为固定输入（tasks.md 5.7）。
//!
//! 两个片段逐字复制自旧仓库 `mkp-core/gcode/state_audit_test.go:20-69`，
//! 来源是真实后处理产物 42274.6_20260713_122939.gcode：
//! - `MKP_DIAGONAL_RIB_SEGMENT`：斜肋过渡段的 MKP 版本（L4221-4240）
//! - `BBS_UNMODIFIED_SEGMENT`：未被修改的 BBS 风格过渡段（L4967-4986）
//!
//! 它们比合成输入多出来的东西：真实字段间距、`.6`/`.4` 这种无前导零写法、
//! `; WIPE_*` 带空格变体、`E-.12056` 紧凑负值。扫描原语在这些形态上的
//! 行为是 pass1/pass2 字节等价的前提。

use postprocess::gcode::{
    extract_coord, format_speed, get_e_value, has_e_param, is_feature_line, match_any_feature,
    match_slicer_comment, parse_xyze,
};

/// 42274.6_20260713_122939.gcode L4221-4240（MKP 处理后）
const MKP_DIAGONAL_RIB_SEGMENT: &[&str] = &[
    "; WIPE_END",
    "G1 E-0.02000 F1800",
    "M204 S6000",
    "G17",
    "G3 Z1.080 I1.217 J0 P1 F42000",
    "G1 X42.986 Y47.701 F12000.0",
    "G1 X44.829 Y44.829 Z1.2 ;Leaving Wiping Tower",
    "; LAYER_HEIGHT: 0.3",
    "; update layer progress",
    "M73 L2",
    "M991 S0 P1 ;notify layer change",
    "; open powerlost recovery",
    "M1003 S1",
    "; OBJECT_ID: 474",
    "M204 S10000",
    "G17",
    "G1 X75.796 Y107.261 Z.6",
    "G1 Z.5",
    "G1 E.4 F1800",
    "; FEATURE: Support",
];

/// 42274.6_20260713_122939.gcode L4967-4986（BBS 原样）
const BBS_UNMODIFIED_SEGMENT: &[&str] = &[
    "; WIPE_TOWER_END",
    "",
    "; WIPE_START",
    "G1 F7311.186",
    "M204 S6000",
    "G1 X49.922 Y99.168 E-.12056",
    "G1 X50.051 Y98.878 E-.12049",
    "G1 X50.157 Y98.58 E-.12046",
    "G1 X50.17 Y98.533 E-.01849",
    "; WIPE_END",
    "G1 E-.02 F1800",
    "M204 S10000",
    "G17",
    "G3 Z.9 I1.217 J0 P1  F42000",
    "; OBJECT_ID: 474",
    "M204 S10000",
    "G1 X98.46 Y89.803",
    "G1 Z.5",
    "G1 E.4 F1800",
    "; FEATURE: Support interface",
];

/// 真实片段上的坐标扫描：`.6`（无前导零）与 `-.12056`（紧凑负 E）必须命中。
#[test]
fn extract_coord_on_real_segments() {
    assert_eq!(extract_coord(b"G1 X75.796 Y107.261 Z.6", b'Z'), Some(0.6));
    assert_eq!(
        extract_coord(b"G1 X75.796 Y107.261 Z.6", b'X'),
        Some(75.796)
    );
    assert_eq!(
        extract_coord(b"G1 X49.922 Y99.168 E-.12056", b'E'),
        Some(-0.12056)
    );
    // E-.02：'-' 在 [\d\.-] 类里，捕获 "-.02" → -0.02
    assert_eq!(extract_coord(b"G1 E-.02 F1800", b'E'), Some(-0.02));
    // E.4 → 0.4
    assert_eq!(extract_coord(b"G1 E.4 F1800", b'E'), Some(0.4));
}

/// 真实片段上的 E 值判定。注意一个**旧侧真实的不对称**（此处刻意钉住）：
/// `HasEParam` 扫描要求 E 后紧跟 `-` 或数字 ⇒ `E.4` **不命中**；
/// `GetEValue` 按字段拆分再 ParseFloat ⇒ `E.4` **能取到 0.4**。
/// 两个函数在旧 Go 侧就是不同判定，Rust 不「修」它。
#[test]
fn e_value_on_real_segments() {
    assert_eq!(get_e_value("G1 E-0.02000 F1800"), Some(-0.02));
    assert_eq!(get_e_value("G1 X49.922 Y99.168 E-.12056"), Some(-0.12056));
    assert!(has_e_param("G1 E-0.02000 F1800"));
    assert!(
        !has_e_param("G1 E.4 F1800"),
        "HasEParam 不认 E.4（E 后是 '.'）"
    );
    assert_eq!(
        get_e_value("G1 E.4 F1800"),
        Some(0.4),
        "GetEValue 按字段拆分认 E.4"
    );
    // G3 圆弧指令也属运动指令，但本行无 E
    assert_eq!(get_e_value("G3 Z1.080 I1.217 J0 P1 F42000"), None);
    // 空行与注释
    assert_eq!(get_e_value(""), None);
    assert_eq!(get_e_value("; WIPE_START"), None);
}

/// ParseXYZE 在真实片段上：负 E 不匹配是正则语义的既定行为。
#[test]
fn parse_xyze_on_real_segments() {
    let v = parse_xyze(b"G1 X50.051 Y98.878 E-.12049");
    assert_eq!(v.x, 50.051);
    assert_eq!(v.y, 98.878);
    assert!(
        !v.has_e,
        "E-.12049 负值不匹配（'-' 不在 \\d+\\.\\d+ 模式里）"
    );

    let v = parse_xyze(b"G1 X44.829 Y44.829 Z1.2 ;Leaving Wiping Tower");
    assert!(v.has_x && v.has_y && v.has_z);
    assert_eq!(v.z, 1.2);

    let v = parse_xyze(b"G1 X98.46 Y89.803");
    assert_eq!(v.x, 98.46);
    assert_eq!(v.y, 89.803);
}

/// 标记与 feature 识别在真实片段上。
#[test]
fn markers_on_real_segments() {
    assert!(match_slicer_comment("; WIPE_END", "WIPE_END"));
    assert!(match_slicer_comment("; WIPE_TOWER_END", "WIPE_TOWER_END"));
    assert_eq!(match_any_feature("; FEATURE: Support"), Some("Support"));
    assert_eq!(
        match_any_feature("; FEATURE: Support interface"),
        Some("Support interface")
    );
    assert_eq!(is_feature_line("; FEATURE: Support"), Some("Support"));
    // 真实速度值格式化冒烟（BBS 写法 7311.186 / 12000.0）
    assert_eq!(format_speed(7311.186), "7311.186");
    assert_eq!(format_speed(12000.0), "12000.0");
}

/// 反空转：两个片段必须逐行等长复制（20 行 / 20 行），防止手滑截断。
#[test]
fn segments_are_intact() {
    assert_eq!(MKP_DIAGONAL_RIB_SEGMENT.len(), 20);
    assert_eq!(BBS_UNMODIFIED_SEGMENT.len(), 20);
    assert!(MKP_DIAGONAL_RIB_SEGMENT.contains(&"G1 X75.796 Y107.261 Z.6"));
    assert!(BBS_UNMODIFIED_SEGMENT.contains(&"G1 X49.922 Y99.168 E-.12056"));
}
