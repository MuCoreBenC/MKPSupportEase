//! 数值格式化与伪随机序列 —— 逐字节等价的判定基础（对照 offset_fmt.go）。
//!
//! 这组函数是 pass1/pass2 输出的**字节面**：`FormatSpeed` 在旧侧有 71 个调用点，
//! 这里差一个字符，G1/G2 golden 就全红。所有函数按 Go 语义逐字复刻。
//!
//! Go↔Rust 语义对照（实测锁定，见 tests）：
//! - `strconv.FormatFloat(v,'f',-1,64)`（最短可往返十进制、定点）≡ Rust `{}`：
//!   两侧都用最短数字串 + 定点展开（无指数记法），对 G-code 值域逐字节一致。
//! - `fmt.Sprintf("%.3f")` ≡ Rust `{:.3}`：两侧定点舍入都是**精确十进制转换 +
//!   ties-to-even**（Go strconv/decimal.go shouldRoundUp；Rust flt2dec format_exact）。
//!   tests 里放了 0.0625/0.1875 两个真 tie 值锁这个行为。

use std::sync::atomic::{AtomicUsize, Ordering};

/// 偏移应用模式（由业务层 ProcessGCodeOffset 解释；此处只是词汇）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetMode {
    Ironing,
    Tower,
    Normal,
    Calibration,
}

impl OffsetMode {
    /// 与旧侧字符串值一致（日志/诊断用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ironing => "ironing",
            Self::Tower => "tower",
            Self::Normal => "normal",
            Self::Calibration => "calibration",
        }
    }
}

/// 最短定点十进制（≡ `strconv.FormatFloat(v,'f',-1,64)`）。
fn shortest_fixed(v: f64) -> String {
    format!("{v}")
}

/// `FormatFloat`：3 位小数，去尾随零，至少留一位小数。
///
/// 例：12.345→"12.345"，12.0→"12.0"，12.300→"12.3"，-0.5→"-0.5"，
/// 0.0001→"0.0"（%.3f 舍到 "0.000" 后只剩 "."，补回 "0"）。
pub fn format_float(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0');
    if let Some(stripped) = s.strip_suffix('.') {
        return format!("{stripped}.0");
    }
    s.to_string()
}

/// `FormatSpeed`：最短十进制，整数时补 ".0"（9000→"9000.0"，18000.5→"18000.5"）。
pub fn format_speed(v: f64) -> String {
    let mut s = shortest_fixed(v);
    if !s.contains('.') {
        s.push_str(".0");
    }
    s
}

/// `FormatSpeedInt`：整型截断（`int64(v)` 的截断语义，18000.9→"18000"）。
pub fn format_speed_int(v: f64) -> String {
    (v as i64).to_string()
}

/// `FormatE`：同 `FormatSpeed` 的规则。
pub fn format_e(v: f64) -> String {
    format_speed(v)
}

/// `FormatZ`：复用 [`format_float`]。
pub fn format_z(v: f64) -> String {
    format_float(v)
}

/// `FormatPythonFloat`：同 `FormatSpeed`（Python repr 风格）。
pub fn format_python_float(v: f64) -> String {
    format_speed(v)
}

/// `FormatEValue`：最短十进制，**严格** 0<v<1 时去前导 "0"（0.5→".5"，
/// -0.5→"-0.5"，0→"0"）。Go 条件是 `v < 1.0 && v > 0`——半开区间 `(0.0..1.0)`
/// 在 Rust 里**包含** 0.0，必须显式写 `v > 0.0`。
pub fn format_e_value(v: f64) -> String {
    let s = shortest_fixed(v);
    if v > 0.0 && v < 1.0 && s.starts_with('0') {
        return s[1..].to_string();
    }
    s
}

/// 伪随机序列的共享全局状态（与旧侧 `pseudoRandomIndex`/`pseudoRandomTable` 同源）。
/// 旧侧初始 index = 1，表 = {3,7,2,8,1,5,9,4,6}，模长回绕。
static PSEUDO_RANDOM_INDEX: AtomicUsize = AtomicUsize::new(1);
const PSEUDO_RANDOM_TABLE: [u8; 9] = [3, 7, 2, 8, 1, 5, 9, 4, 6];

/// `GetPseudoRandom`：返回序列中的下一位数字（1-9）。
pub fn get_pseudo_random() -> String {
    let idx = PSEUDO_RANDOM_INDEX.load(Ordering::Relaxed);
    let num = PSEUDO_RANDOM_TABLE[idx];
    PSEUDO_RANDOM_INDEX.store((idx + 1) % PSEUDO_RANDOM_TABLE.len(), Ordering::Relaxed);
    num.to_string()
}

/// 供 [`crate::gcode::LineBuilder::append_pseudo_random`] 复用的同一状态出口。
pub(crate) fn next_pseudo_random_digit() -> u8 {
    let idx = PSEUDO_RANDOM_INDEX.load(Ordering::Relaxed);
    let num = PSEUDO_RANDOM_TABLE[idx];
    PSEUDO_RANDOM_INDEX.store((idx + 1) % PSEUDO_RANDOM_TABLE.len(), Ordering::Relaxed);
    num
}

/// `ResetPseudoRandom`：重置序列到初始状态（index=1），供测试与再生流程使用。
pub fn reset_pseudo_random() {
    PSEUDO_RANDOM_INDEX.store(1, Ordering::Relaxed);
}

/// `MathRound`：按 n 位小数四舍五入（Go `math.Round` = half away from zero，
/// 与 Rust `f64::round` 相同；`10^n` 用 powi，n 在 G-code 值域内精确可表示）。
pub fn math_round(x: f64, n: i32) -> f64 {
    let pow = 10f64.powi(n);
    (x * pow).round() / pow
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 5.2 要求「先写格式化单元测试再写别的」——本表是 Go 侧
    /// line_builder_test.go 各 EdgeCase 表的并集，期望值按 Go 语义手推。
    #[test]
    fn format_float_table() {
        // (输入, 期望)
        let cases: &[(f64, &str)] = &[
            (0.0, "0.0"),
            (-0.0, "-0.0"), // Go：%.3f(-0.0)="-0.000" → 去零 → "-." → 补回 "-0.0"
            (0.1, "0.1"),
            (0.5, "0.5"),
            (0.999, "0.999"),
            (1.0, "1.0"),
            (1.5, "1.5"),
            (12.0, "12.0"),
            (12.345, "12.345"),
            (12.3, "12.3"),
            (-0.5, "-0.5"),
            (-1.0, "-1.0"),
            (-12.345, "-12.345"),
            (-12.0, "-12.0"),
            (100.0, "100.0"),
            (1000.5, "1000.5"),
            (99999.999, "99999.999"),
            (0.001, "0.001"),
            (0.0001, "0.0"), // 舍到 0.000 → "0.0"
        ];
        for (v, want) in cases {
            assert_eq!(&format_float(*v), want, "format_float({v})");
        }
    }

    #[test]
    fn format_float_negative_zero_keeps_sign() {
        // Go：%.3f(-0.0) → "-0.000" → TrimRight "0" → "-." → 补 "0" → "-0.0"
        // Rust {:.3}(-0.0) 同样给 "-0.000"。这条锁定符号不被丢。
        assert_eq!(format_float(-0.0), "-0.0");
    }

    #[test]
    fn format_float_ties_to_even_matches_go() {
        // 0.0625 与 0.1875 是二进制精确值的真 tie（第 4 位小数是 5 且其后无尾数）：
        // Go 与 Rust 都按 ties-to-even 处理 → "0.062" / "0.188"。
        assert_eq!(format_float(0.0625), "0.062");
        assert_eq!(format_float(0.1875), "0.188");
        assert_eq!(format_float(1.0625), "1.062");
        assert_eq!(format_float(1.1875), "1.188");
    }

    #[test]
    fn format_speed_table() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0.0"),
            (1.0, "1.0"),
            (1.5, "1.5"),
            (9000.0, "9000.0"),
            (18000.0, "18000.0"),
            (18000.5, "18000.5"),
            (-1.0, "-1.0"),
            (-1.5, "-1.5"),
            (-9000.0, "-9000.0"),
            (0.0001, "0.0001"),
            (0.5, "0.5"),
        ];
        for (v, want) in cases {
            assert_eq!(&format_speed(*v), want, "format_speed({v})");
            assert_eq!(&format_e(*v), want, "format_e({v})");
            assert_eq!(&format_python_float(*v), want, "format_python_float({v})");
        }
    }

    #[test]
    fn format_speed_int_truncates_like_int64() {
        assert_eq!(format_speed_int(0.0), "0");
        assert_eq!(format_speed_int(9000.0), "9000");
        assert_eq!(format_speed_int(1.7), "1");
        assert_eq!(format_speed_int(18000.9), "18000");
        assert_eq!(format_speed_int(-1.0), "-1");
    }

    #[test]
    fn format_e_value_table() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (0.1, ".1"),
            (0.5, ".5"),
            (0.999, ".999"),
            (1.0, "1"),
            (1.5, "1.5"),
            (5.5, "5.5"),
            (-0.5, "-0.5"),
            (-1.0, "-1"),
            (-5.5, "-5.5"),
            (0.0001, ".0001"),
            (0.4, ".4"),
        ];
        for (v, want) in cases {
            assert_eq!(&format_e_value(*v), want, "format_e_value({v})");
        }
    }

    /// 伪随机序列与重置：从 index=1 起，表 {3,7,2,8,1,5,9,4,6} 的取值顺序。
    #[test]
    fn pseudo_random_sequence_and_reset() {
        reset_pseudo_random();
        // index=1 起：7,2,8,1,5,9,4,6,3（回绕到 0）,7（回绕）…
        let want = ["7", "2", "8", "1", "5", "9", "4", "6", "3", "7"];
        for w in want {
            assert_eq!(&get_pseudo_random(), w);
        }
        reset_pseudo_random();
        let first = get_pseudo_random();
        reset_pseudo_random();
        assert_eq!(get_pseudo_random(), first, "reset 必须恢复同一序列");
    }

    #[test]
    fn math_round_matches_go() {
        assert_eq!(math_round(0.12345, 2), 0.12);
        assert_eq!(math_round(0.125, 2), 0.13); // half away from zero（Go math.Round）
        assert_eq!(math_round(-0.125, 2), -0.13);
        assert_eq!(math_round(2.5, 0), 3.0);
        assert_eq!(math_round(-2.5, 0), -3.0);
    }
}
