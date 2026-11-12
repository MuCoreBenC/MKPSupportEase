//! 导出日志前的**脱敏**：家目录 → `~`，IP 字面量 → `x.x.x.x` / `[x:x::x]`。
//!
//! # 为什么只做这两件
//!
//! 日志是给开发者排查用的：预设名、机型名、错误码、命令名**一个都不能动** ——
//! 抹掉它们等于把要查的东西抹掉。会泄露身份的是另两类：用户家目录（含用户名）与
//! 网络地址。
//!
//! 纯函数、不读盘、不认环境：脱敏规则要能被判据逐条钉住（见本文件测试）。
//!
//! # 时间不是 IP
//!
//! `10:20:30` 这种时刻与 IPv6 长得像。判据因此认死两条：含 `::`，或者至少 3 个冒号。
//! 少了这一条，日志里每一行的时间戳都会被打成 `[x:x::x]` —— 那份日志就没法看了。

/// 把一行日志里的身份信息打码。
///
/// `home` 是用户家目录（拿不到就只打码 IP）：日志里凡是以它开头的路径都变成 `~/…`，
/// Windows 的反斜杠形态与正斜杠形态一起认。
pub fn scrub(line: &str, home: Option<&str>) -> String {
    let out = match home.map(str::trim).filter(|h| !h.is_empty()) {
        Some(home) => {
            let alt = home.replace('\\', "/");
            let mut s = replace_at_boundary(line, home, "~");
            if alt != home {
                s = replace_at_boundary(&s, &alt, "~");
            }
            s
        }
        None => line.to_owned(),
    };
    scrub_ips(&out)
}

/// 只替换"整段路径开头"的那种出现：`/Users/wzy` 不会把 `/Users/wzy2` 也啃掉。
fn replace_at_boundary(hay: &str, needle: &str, repl: &str) -> String {
    let mut out = String::with_capacity(hay.len());
    let mut rest = hay;
    while let Some(at) = rest.find(needle) {
        let after = &rest[at + needle.len()..];
        /* 后面紧跟字母数字 = 这是另一个名字（`wzy2`），不是家目录 */
        if after.chars().next().is_some_and(|c| c.is_alphanumeric()) {
            out.push_str(&rest[..at + needle.len()]);
            rest = after;
            continue;
        }
        out.push_str(&rest[..at]);
        out.push_str(repl);
        rest = after;
    }
    out.push_str(rest);
    out
}

/// 扫一遍 `[0-9a-fA-F:.\[\]]+` 形状的片段，认得出的地址原地打码，其余原样保留。
fn scrub_ips(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut token = String::new();
    for ch in s.chars() {
        if ch.is_ascii_hexdigit() || matches!(ch, '.' | ':' | '[' | ']') {
            token.push(ch);
            continue;
        }
        flush(&mut token, &mut out);
        out.push(ch);
    }
    flush(&mut token, &mut out);
    out
}

fn flush(token: &mut String, out: &mut String) {
    if token.is_empty() {
        return;
    }
    match mask(token) {
        Some(masked) => out.push_str(&masked),
        None => out.push_str(token),
    }
    token.clear();
}

/// 认得出就给出打码后的形状（端口原样留着 —— 排查要认"哪个口没通"）。
fn mask(token: &str) -> Option<String> {
    /* `127.0.0.1:8080` / `[::1]:8080`：尾部纯数字才算端口。
    尾部是 `::1` 里那个 `1` 时不能切（那一位属于地址本身） */
    let (host, port) = match token.rsplit_once(':') {
        Some((h, p))
            if !h.is_empty()
                && !h.ends_with(':')
                && !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit()) =>
        {
            (h, Some(p))
        }
        _ => (token, None),
    };
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    if is_ipv4(bare) {
        return Some(match port {
            Some(p) => format!("x.x.x.x:{p}"),
            None => "x.x.x.x".to_owned(),
        });
    }
    if is_ipv6(bare) {
        return Some(match port {
            Some(p) => format!("[x:x::x]:{p}"),
            None => "[x:x::x]".to_owned(),
        });
    }
    None
}

fn is_ipv4(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.len() <= 3
            && p.bytes().all(|b| b.is_ascii_digit())
            && p.parse::<u16>().is_ok_and(|n| n <= 255)
    })
}

/// `::1` / `fe80::1` / `2001:db8:0:0:0:0:0:1` 都算；`10:20:30` 这种时刻不算。
fn is_ipv6(token: &str) -> bool {
    let colons = token.bytes().filter(|b| *b == b':').count();
    let compressed = token.contains("::");
    if colons < 3 && !compressed {
        return false;
    }
    if token.contains('.') {
        return false;
    }
    let groups: Vec<&str> = token.split(':').collect();
    if groups.len() < 3 || groups.len() > 9 {
        return false;
    }
    /* 压缩写法（`::1`）只剩一组，"至少两组"会把最常用的那个回环地址漏掉 */
    let hex_groups = groups.iter().filter(|g| !g.is_empty()).count();
    hex_groups >= 1
        && groups
            .iter()
            .all(|g| g.is_empty() || (g.len() <= 4 && g.bytes().all(|b| b.is_ascii_hexdigit())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_prefix_becomes_a_tilde() {
        let home = Some("/Users/wzy");
        assert_eq!(
            scrub("/Users/wzy/Library/Logs/x.log", home),
            "~/Library/Logs/x.log"
        );
        /* 边界：另一个用户的名字不许被啃 */
        assert_eq!(scrub("/Users/wzy2/x", home), "/Users/wzy2/x");
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_are_redacted_in_both_slash_styles() {
        let home = Some(r"C:\Users\wzy");
        assert_eq!(scrub(r"C:\Users\wzy\AppData\x", home), r"~\AppData\x");
        assert_eq!(scrub("C:/Users/wzy/AppData/x", home), "~/AppData/x");
    }

    #[test]
    fn ipv4_and_its_port_are_masked() {
        assert_eq!(scrub("GET 127.0.0.1:8787 ok", None), "GET x.x.x.x:8787 ok");
        assert_eq!(scrub("from 192.168.1.20", None), "from x.x.x.x");
    }

    #[test]
    fn ipv6_is_masked_but_clock_time_is_not() {
        assert_eq!(scrub("dial [::1]:9000", None), "dial [x:x::x]:9000");
        assert_eq!(scrub("peer fe80::1c9 ok", None), "peer [x:x::x] ok");
        /* 时间戳一格不动 —— 这是这份日志还能看的前提 */
        assert_eq!(scrub("2026-11-12T10:20:30Z", None), "2026-11-12T10:20:30Z");
        assert_eq!(scrub("10:20:30", None), "10:20:30");
    }

    #[test]
    fn ordinary_words_and_numbers_survive() {
        assert_eq!(
            scrub("preset mkp_a1 v0.0.12 ok", None),
            "preset mkp_a1 v0.0.12 ok"
        );
        assert_eq!(scrub("byte 1024 deadbeef", None), "byte 1024 deadbeef");
    }
}
