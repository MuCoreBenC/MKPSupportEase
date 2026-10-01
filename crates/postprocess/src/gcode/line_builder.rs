//! `LineBuilder` —— 可复用的行装配缓冲（对照 line_builder.go）。
//!
//! 语义与自由函数 [`crate::gcode::format`] 的对应成员**字节级等价**（等价性由测试钉住，
//! 对应旧侧 `processor.TestLineBuilderEquivalence` 的意图）。Rust 侧保留它：
//! pass2 有 27 个写行点，统一走一处格式化出口，杜绝「两处格式化悄悄分叉」。

/// 可复用字节缓冲。非线程安全（CLI 单线程消费，与旧侧同一约束）。
#[derive(Default)]
pub struct LineBuilder {
    buf: Vec<u8>,
}

impl LineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 截断长度到 0，保留容量复用。
    pub fn reset(&mut self) {
        self.buf.clear();
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// 借内部字节。返回的切片只在下一次变更前有效。
    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    /// 拷贝出 String（稳态下每行唯一一次分配，与旧侧 String() 同语义）。
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.buf).into_owned()
    }

    pub fn push_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn push_byte(&mut self, c: u8) {
        self.buf.push(c);
    }

    pub fn push_bytes(&mut self, p: &[u8]) {
        self.buf.extend_from_slice(p);
    }

    /// 追加非负整数十进制（≡ `strconv.Itoa`）。
    pub fn append_int(&mut self, n: i64) {
        self.buf.extend_from_slice(n.to_string().as_bytes());
    }

    /// 追加速度值（≡ `FormatSpeed`：最短十进制，整数补 ".0"）。
    pub fn append_speed(&mut self, v: f64) {
        self.append_shortest_with_dot_zero(v);
    }

    /// 追加整型速度（≡ `FormatSpeedInt`：int64 截断）。
    pub fn append_speed_int(&mut self, v: f64) {
        self.append_int(v as i64);
    }

    /// 追加 E 轴值（≡ `FormatE`）。
    pub fn append_e(&mut self, v: f64) {
        self.append_shortest_with_dot_zero(v);
    }

    /// 追加 Python repr 风格浮点（≡ `FormatPythonFloat`）。
    pub fn append_python_float(&mut self, v: f64) {
        self.append_shortest_with_dot_zero(v);
    }

    /// 追加 `%.3f` 后去尾零（≡ `FormatFloat` / `FormatZ`）。
    pub fn append_format_float(&mut self, v: f64) {
        let s = format!("{v:.3}");
        let trimmed = s.trim_end_matches('0');
        if let Some(stripped) = trimmed.strip_suffix('.') {
            // 保留一位小数：FormatFloat 对 "12." 补 "0" → "12.0"
            self.buf.extend_from_slice(stripped.as_bytes());
            self.buf.extend_from_slice(b".0");
        } else {
            self.buf.extend_from_slice(trimmed.as_bytes());
        }
    }

    /// 追加 Z 轴值（≡ `FormatZ` → `FormatFloat`）。
    pub fn append_z(&mut self, v: f64) {
        self.append_format_float(v);
    }

    /// 追加 E 值（≡ `FormatEValue`：最短十进制，严格 0<v<1 去前导 "0"）。
    pub fn append_e_value(&mut self, v: f64) {
        let s = format!("{v}");
        if v > 0.0 && v < 1.0 && s.starts_with('0') {
            self.buf.extend_from_slice(&s.as_bytes()[1..]);
        } else {
            self.buf.extend_from_slice(s.as_bytes());
        }
    }

    /// 追加伪随机序列的下一位数字（与 [`crate::gcode::format::get_pseudo_random`]
    /// 共享同一全局状态——等价性有测试钉住）。
    pub fn append_pseudo_random(&mut self) {
        let num = crate::gcode::format::next_pseudo_random_digit();
        self.buf.push(b'0' + num);
    }

    fn append_shortest_with_dot_zero(&mut self, v: f64) {
        let s = format!("{v}");
        self.buf.extend_from_slice(s.as_bytes());
        if !s.contains('.') {
            self.buf.extend_from_slice(b".0");
        }
    }
}

impl PartialEq<str> for LineBuilder {
    fn eq(&self, other: &str) -> bool {
        self.buf == other.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcode::format::{
        format_e, format_e_value, format_float, format_python_float, format_speed,
        format_speed_int, get_pseudo_random, reset_pseudo_random,
    };

    fn build(b: &mut LineBuilder, f: impl FnOnce(&mut LineBuilder)) -> String {
        b.reset();
        f(b);
        b.to_string_lossy()
    }

    /// 与自由函数的等价性（旧侧 TestLineBuilder*EdgeCases 家族的对应面）。
    #[test]
    fn append_members_match_free_functions() {
        let mut b = LineBuilder::new();
        let float_cases = [
            0.0, -0.0, 0.1, 0.5, 0.999, 1.0, 1.5, 12.0, 12.345, -0.5, -1.0, -12.345, 100.0, 1000.5,
            99999.999, 0.001, 0.0001, 0.0625,
        ];
        for v in float_cases {
            assert_eq!(
                build(&mut b, |b| b.append_format_float(v)),
                format_float(v),
                "v={v}"
            );
            // append_z 与 append_format_float 同源
            assert_eq!(build(&mut b, |b| b.append_z(v)), format_float(v), "z v={v}");
        }
        let speed_cases = [
            0.0, 1.0, 1.5, 9000.0, 18000.0, 18000.5, -1.0, -1.5, -9000.0, 0.0001,
        ];
        for v in speed_cases {
            assert_eq!(
                build(&mut b, |b| b.append_speed(v)),
                format_speed(v),
                "v={v}"
            );
            assert_eq!(build(&mut b, |b| b.append_e(v)), format_e(v), "v={v}");
            assert_eq!(
                build(&mut b, |b| b.append_python_float(v)),
                format_python_float(v),
                "v={v}"
            );
        }
        for v in [
            0.0, 1.0, 9000.0, 18000.0, 3600.0, 5400.0, -1.0, -9000.0, 1.7, 18000.9,
        ] {
            assert_eq!(
                build(&mut b, |b| b.append_speed_int(v)),
                format_speed_int(v),
                "v={v}"
            );
        }
        let evalue_cases = [
            0.0, -0.0, 0.1, 0.5, 0.999, 1.0, 1.5, 5.5, -0.5, -1.0, -5.5, 0.0001, 0.4,
        ];
        for v in evalue_cases {
            assert_eq!(
                build(&mut b, |b| b.append_e_value(v)),
                format_e_value(v),
                "v={v}"
            );
        }
    }

    /// 一次典型行装配的整行输出（格式成员的组合冒烟）。
    #[test]
    fn assembles_full_line() {
        let mut b = LineBuilder::new();
        b.reset();
        b.push_str("G1 X180.0 Y180.0 ");
        b.append_speed(9000.0);
        b.push_byte(b'\n');
        assert_eq!(b.to_string_lossy(), "G1 X180.0 Y180.0 9000.0\n");
        assert_eq!(b.len(), 24);
        b.reset();
        assert!(b.is_empty());
    }

    /// AppendPseudoRandom 与 GetPseudoRandom 从同一初始 index 出发序列相同
    /// （20 次取样覆盖回绕；对应旧侧 TestLineBuilderPseudoRandomMatchesGetPseudoRandom）。
    #[test]
    fn append_pseudo_random_matches_get_pseudo_random() {
        const N: usize = 20;
        reset_pseudo_random();
        let want: Vec<String> = (0..N).map(|_| get_pseudo_random()).collect();

        reset_pseudo_random();
        let mut b = LineBuilder::new();
        for _ in 0..N {
            b.append_pseudo_random();
        }
        let got: Vec<String> = b.to_string_lossy().chars().map(|c| c.to_string()).collect();
        assert_eq!(got, want);
    }
}
