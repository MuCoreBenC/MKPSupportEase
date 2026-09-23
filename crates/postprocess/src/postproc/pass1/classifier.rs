//! 行分类器（对照 processor/line_classifier.go 逐行）。
//!
//! 单次字节级扫描取代主循环里 30+ 个 Contains/HasPrefix —— 语义与 Go 完全一致：
//! ';' 行优先级 Feature > ZHeight > LayerNum > Comment；代码行 Motion > MCode > Other。

/// LineClass —— G-code 行的分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClass {
    Other,
    Comment,
    Feature,
    ZHeight,
    LayerNum,
    Motion,
    MCode,
}

/// `ClassifyLine`：单次字节扫描返回行类别。
pub fn classify_line(line: &str) -> LineClass {
    let line = line.as_bytes();
    // 1. 跳过前导空白（空格 / tab）
    let mut i = 0;
    while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
        i += 1;
    }
    if i >= line.len() {
        return LineClass::Comment; // 空行或纯空白
    }
    let rest = &line[i..];
    match rest[0] {
        b';' => {
            if has_comment_marker(rest, b"FEATURE:") {
                return LineClass::Feature;
            }
            if has_comment_marker(rest, b"Z_HEIGHT:") {
                return LineClass::ZHeight;
            }
            if contains(rest, b"; layer num/total_layer_count") {
                return LineClass::LayerNum;
            }
            LineClass::Comment
        }
        b'G' | b'g' => {
            if is_motion_code(rest) {
                LineClass::Motion
            } else {
                LineClass::Other
            }
        }
        b'M' | b'm' => {
            if rest.len() > 1 && rest[1].is_ascii_digit() {
                LineClass::MCode
            } else {
                LineClass::Other
            }
        }
        _ => LineClass::Other,
    }
}

/// `hasCommentMarker`：rest 以 ';' 开头后，允许至多一个空格再匹配 marker。
fn has_comment_marker(rest: &[u8], marker: &[u8]) -> bool {
    let mut s = &rest[1..];
    if !s.is_empty() && s[0] == b' ' {
        s = &s[1..];
    }
    s.starts_with(marker)
}

/// `isMotionCode`：G{0,1,2,3} / G0{0,1,2,3} 后跟分隔符或行尾；其余 G 码不算。
fn is_motion_code(rest: &[u8]) -> bool {
    if rest.len() < 2 {
        return false;
    }
    let d1 = rest[1];
    if !d1.is_ascii_digit() {
        return false;
    }
    // 单数字形式：G + {0,1,2,3} + 分隔/行尾
    if is_delim_or_end(rest, 2) {
        return d1 == b'0' || d1 == b'1' || d1 == b'2' || d1 == b'3';
    }
    // 双数字形式：只有 G0x（x∈{0,1,2,3}）合格 —— G10/G28 等不合格
    if d1 != b'0' {
        return false;
    }
    if rest.len() < 3 {
        return false;
    }
    let d2 = rest[2];
    if d2 != b'0' && d2 != b'1' && d2 != b'2' && d2 != b'3' {
        return false;
    }
    is_delim_or_end(rest, 3)
}

fn is_delim_or_end(rest: &[u8], i: usize) -> bool {
    if i >= rest.len() {
        return true;
    }
    let b = rest[i];
    b == b' ' || b == b'\t' || b == b'\r' || b == b'\n'
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_matrix() {
        assert_eq!(classify_line("; FEATURE: Outer wall"), LineClass::Feature);
        assert_eq!(classify_line(";FEATURE: Inner wall"), LineClass::Feature);
        assert_eq!(classify_line("; Z_HEIGHT: 0.2"), LineClass::ZHeight);
        assert_eq!(
            classify_line("; layer num/total_layer_count: 1/233"),
            LineClass::LayerNum
        );
        assert_eq!(classify_line("; plain comment"), LineClass::Comment);
        assert_eq!(classify_line("   "), LineClass::Comment);
        assert_eq!(classify_line(""), LineClass::Comment);
        assert_eq!(classify_line("G1 X1 Y2"), LineClass::Motion);
        assert_eq!(classify_line("G01 X1"), LineClass::Motion);
        assert_eq!(classify_line("G28"), LineClass::Other);
        assert_eq!(classify_line("G10"), LineClass::Other);
        assert_eq!(classify_line("M106 S255"), LineClass::MCode);
        assert_eq!(classify_line("m106 S255"), LineClass::MCode);
        assert_eq!(classify_line("T0"), LineClass::Other);
        // Go：';' 后至多剥一个空格再前缀匹配；两个空格即不匹配
        assert_eq!(classify_line("; FEATURE: X"), LineClass::Feature);
        assert_eq!(classify_line(";  FEATURE: X"), LineClass::Comment);
    }
}
