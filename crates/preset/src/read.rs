//! 读取面 —— 头注释正则、CRLF 两段式、错误码（对照 config/read.go）。

use crate::model::{PresetFile, TomlConfig};
// 搬入 mkp-ssr 的唯一改动：mkp_diag → mkp_pp::diag（错误码本身一个字没动，
// 它们是对外契约：E_CFG_PARSE_001 / E_TOML_PARSE_001 …）。
use mkp_pp::diag::PostprocError;
use std::path::Path;

/// 读预设文件。文件缺失 → `E_FS_NOT_FOUND_001`；解析两段式均失败 →
/// `E_TOML_PARSE_001`；头缺 `# machine:` → `MissingMachine`（E_CFG_PARSE_001）。
pub fn read_preset(path: &Path) -> Result<PresetFile, PostprocError> {
    let raw = std::fs::read_to_string(path).map_err(|source| PostprocError::Io {
        op: "read_preset",
        source,
    })?;
    read_preset_from_bytes(raw)
}

/// `ReadTomlFromBytes` 的逐字复刻：先原样 Unmarshal，失败则把 `\r\n`/`\r`
/// 归一为 `\n` 再试一次，仍失败才报 `E_TOML_PARSE_001`。
pub fn read_preset_from_bytes(raw: String) -> Result<PresetFile, PostprocError> {
    let parsed: Result<TomlConfig, _> = toml::from_str(&raw);
    let config = match parsed {
        Ok(cfg) => cfg,
        Err(first_err) => {
            let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
            match toml::from_str(&normalized) {
                Ok(cfg) => cfg,
                Err(second_err) => {
                    return Err(PostprocError::TomlParse {
                        path: String::new(),
                        message: describe_parse_failure(&first_err, &second_err),
                    });
                }
            }
        }
    };

    let machine =
        parse_machine_from_content(&raw).ok_or_else(|| PostprocError::MissingMachine {
            path: String::new(),
        })?;
    let release_time = parse_release_time_from_content(&raw);
    let variant = parse_variant_from_content(&raw);
    let lineage = parse_lineage_from_content(&raw);

    Ok(PresetFile {
        config,
        release_time,
        machine,
        variant,
        lineage,
        raw,
    })
}

/// 解析失败的说法。**未知键要能被人看懂**。
///
/// `deny_unknown_fields` 的原话是 `unknown field \`future_knob\`, expected one of …`，
/// 后面跟着 77 个键名 —— 用户看到的是一屏英文清单，得不出「该怎么办」。
/// 而这条错误在现实里最可能的原因只有一个：**这份预设比本程序新**
/// （官方给参数改了名/加了键，而这个版本的程序还不认识它）。所以在原话前面加一句人话，
/// 并点名那个键。
///
/// **不放宽 `deny_unknown_fields`**：读不回来仍然是读不回来（多一个键的预设写回去会丢键，
/// 那比读不出来更坏）。改的只是「人能不能看懂」。
/// 错误码也**一个字不动**（仍是 `E_TOML_PARSE_001`）—— 钩子那边按码分支。
fn describe_parse_failure(first: &toml::de::Error, second: &toml::de::Error) -> String {
    let raw = format!("first: {first}; after CRLF normalize: {second}");
    match unknown_field_name(&second.to_string()).or_else(|| unknown_field_name(&first.to_string()))
    {
        Some(key) => format!(
            "这份预设比本程序新（多了 `{key}`），请升级程序；\
             或者确认这个键是不是写错了。\n{raw}"
        ),
        None => raw,
    }
}

/// 从 serde 的原话里抠出未知键的名字（``unknown field `xxx` ``）。
fn unknown_field_name(message: &str) -> Option<String> {
    let rest = message.split("unknown field `").nth(1)?;
    let name = rest.split('`').next()?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// `^#\s*release_time\s*:\s*(.+)$`（read.go:205 的正则，trim 后逐行匹配）。
/// 手写匹配器（无 regex 依赖）：'#'、任意空白、字面量、任意空白、':'、
/// 任意空白、至少一个字符的捕获（TrimSpace 后为空视同未命中）。
pub fn parse_release_time_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "release_time")
}

/// `^#\s*machine\s*:\s*(.+)$`（read.go:219；actualMachine 的 SSOT 数据源）。
pub fn parse_machine_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "machine")
}

/// `^#\s*variant\s*:\s*(.+)$` —— 与 `release_time` 同一条路。
///
/// **缺失返回 `None` 且不报错**：老预设可能没有这一行。变体名用来查注册表的
/// 按机型区间覆盖（`机型:变体`，注册表侧是大写），这里**原样返回**，不动用户文件里的字。
pub fn parse_variant_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "variant")
}

/// `# based_on` / `# based_on_release_time` / `# based_on_sha256` 三行。
///
/// 三项**全都缺** ⇒ 返回 `None`（"这份预设没有血统"，今天磁盘上所有预设都是这样）；
/// 只缺其中几项 ⇒ 返回 `Some`，缺的那几项是 `None`（"这几项不知道"）。
/// 这两种情况在界面上说的话不一样，所以类型上必须分得开。
///
/// **匹配顺序有讲究**：`based_on_release_time` 必须比 `based_on` 先试 ——
/// `match_header_key` 只要求键后跟可选空白 + 冒号，而 `based_on_release_time` 这一行
/// 用 `based_on` 去匹配时，键后紧跟的是 `_`（不是冒号）⇒ 不会误命中。
/// 这条已经由判据钉住（`# based_on_x: v` 不许被当成 `based_on`）。
pub fn parse_lineage_from_content(content: &str) -> Option<crate::model::Lineage> {
    let lineage = crate::model::Lineage {
        based_on: parse_header_value(content, "based_on"),
        based_on_release_time: parse_header_value(content, "based_on_release_time"),
        based_on_sha256: parse_header_value(content, "based_on_sha256"),
    };
    if lineage.is_empty() {
        return None;
    }
    Some(lineage)
}

/// 逐行找 `# <key>: <值>`，返回第一个非空值。四个头注释字段共用这一条路
/// （之前是四份一样的循环）。
fn parse_header_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = match_header_key(trimmed, key) {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// 行头 `#` + 可选空白 + 字面键 + 可选空白 + `:` + 可选空白，返回冒号后的
/// 原样剩余（捕获组 (.+) 要求非空）。
fn match_header_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let after_hash = line.strip_prefix('#')?;
    let after_ws1 = after_hash.trim_start();
    // trim_start 会剥所有 Unicode 空白；Go 的 \s 是 [\t\n\f\r ]——头部注释
    // 实际只有空格/制表，两者在此值域等价。
    let after_key = after_ws1.strip_prefix(key)?;
    // 键后必须是可选空白 + 冒号（防 "machine_x:" 误命中）
    let after_ws2 = after_key.trim_start();
    let after_colon = after_ws2.strip_prefix(':')?;
    let rest = after_colon.trim_start();
    if rest.is_empty() {
        return None; // (.+) 至少一个字符
    }
    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IntOrFloat;

    const MINIMAL: &str = "\
# uuid: 11111111-2222-3333-4444-555555555555
# release_time: 2026-08-19 01:38:13
# machine: A1
[toolhead]
offset = { x = -1, y = 18.6, z = 4 }
speed_limit = 70
MKP_retract = 0
first_pen_revitalization_flag = false
l803_leak_prevent_flag = false

[wiping]
have_wiping_components = 'tower'
wiper_x = 20.0
wiper_y = 20.0
wipetower_speed = 20.0
nozzle_cooling_flag = false
user_dry_time = 5
support_extrusion_multiplier = 1.0
interface_ironing_flag = false
use_ironing_path = false
prime_enabled = false
prime_per_model = false
prime_every_layer = false
rib_fillet_wall = true
disable_front_cover_alarm = false
disable_3rd_layer_clog_detect = false
disable_timelapse = false
min_support_interface_enabled = false
glue_z_compensation_enabled = false
";

    #[test]
    fn minimal_preset_parses_with_header_fields() {
        let f = read_preset_from_bytes(MINIMAL.to_string()).unwrap();
        assert_eq!(f.machine, "A1");
        assert_eq!(f.release_time.as_deref(), Some("2026-08-19 01:38:13"));
        assert_eq!(f.config.toolhead.speed_limit, 70.0);
        assert_eq!(f.config.toolhead.offset.y, 18.6);
        assert_eq!(f.config.wiping.wiper_x, 20.0);
        assert!(f.config.wiping.rib_fillet_wall);
    }

    #[test]
    fn missing_machine_is_hard_error() {
        let raw = MINIMAL.replacen("# machine: A1\n", "", 1);
        let err = read_preset_from_bytes(raw).unwrap_err();
        assert!(matches!(err, PostprocError::MissingMachine { .. }));
        assert_eq!(err.code(), "E_CFG_PARSE_001");
    }

    #[test]
    fn broken_toml_is_toml_parse_error() {
        let err = read_preset_from_bytes("not [valid toml".to_string()).unwrap_err();
        assert!(matches!(err, PostprocError::TomlParse { .. }));
        assert_eq!(err.code(), "E_TOML_PARSE_001");
    }

    /// CRLF 兜底两形态，与 Go 语义逐字对齐：
    /// - `\r\n` 文件：首轮可解析（或归一后可解析），头注释照常命中；
    /// - 裸 `\r` 文件：TOML 归一后**能**解析，但 Go 的 bufio.Scanner 同样
    ///   不按裸 \r 分行 ⇒ machine 头提取不到 ⇒ 引擎层硬错误。Rust 在读取点
    ///   报 MissingMachine，失败层级不同、终态一致，如实登记。
    #[test]
    fn crlf_and_bare_cr_behave_like_go() {
        let crlf = MINIMAL.replace('\n', "\r\n");
        let f = read_preset_from_bytes(crlf).unwrap();
        assert_eq!(f.machine, "A1");

        let bare_cr = MINIMAL.replace('\n', "\r");
        let err = read_preset_from_bytes(bare_cr).unwrap_err();
        assert!(matches!(err, PostprocError::MissingMachine { .. }));
    }

    /// 未知键 = 解析失败（deny_unknown_fields 的机械判据，见 lib.rs 文档）。
    #[test]
    fn unknown_key_fails_loudly() {
        let raw = MINIMAL.replace("[wiping]", "[wiping]\ntypo_key = 1\n");
        assert!(read_preset_from_bytes(raw).is_err());
    }

    /// K-O4：未知键的错误要说人话，并点名那个键。
    ///
    /// **这条错误在现实里最可能的原因只有一个**：预设比程序新（官方加了键/改了名）。
    /// serde 的原话是 `unknown field ... expected one of` 加 77 个键名 ——
    /// 一屏英文清单，得不出「该怎么办」。
    #[test]
    fn ko4_a_newer_preset_says_so_by_name() {
        let raw = MINIMAL.replace("[wiping]", "[wiping]\nfuture_knob = 1\n");
        let err = read_preset_from_bytes(raw).expect_err("多一个键必须读不回来");
        let msg = err.to_string();
        assert!(
            msg.contains("比本程序新"),
            "K-O4 红：错误里要说清「这份预设比本程序新」，实测：{msg}"
        );
        assert!(
            msg.contains("future_knob"),
            "K-O4 红：错误里要点名那个键，实测：{msg}"
        );
        // **不放宽 deny_unknown_fields、也不改错误码**（钩子那边按码分支）
        assert_eq!(err.code(), "E_TOML_PARSE_001");

        // 普通的语法错误不许被说成「比本程序新」
        let plain = read_preset_from_bytes("not [valid toml".to_string()).expect_err("语法错");
        assert!(
            !plain.to_string().contains("比本程序新"),
            "K-O4 红：语法错被误报成版本问题：{plain}"
        );
        println!("K-O4 绿：多一个键 ⇒ 「比本程序新（多了 future_knob）」，码仍是 E_TOML_PARSE_001");
    }

    /// tower_wipe_speed 的 int / float 双形态。
    #[test]
    fn tower_wipe_speed_accepts_int_and_float() {
        let base = MINIMAL.to_string();
        let with_int = base.replace("[wiping]", "[wiping]\ntower_wipe_speed = 40\n");
        let with_float = base.replace("[wiping]", "[wiping]\ntower_wipe_speed = 40.5\n");
        match read_preset_from_bytes(with_int)
            .unwrap()
            .config
            .wiping
            .tower_wipe_speed
        {
            Some(IntOrFloat::I64(40)) => {}
            other => panic!("int 形态解析错: {other:?}"),
        }
        match read_preset_from_bytes(with_float)
            .unwrap()
            .config
            .wiping
            .tower_wipe_speed
        {
            Some(IntOrFloat::F64(40.5)) => {}
            other => panic!("float 形态解析错: {other:?}"),
        }
    }

    /// K-O2a：血统三行读得回来，缺一行只让那一项 `None`，三行全缺 ⇒ `lineage` 是 `None`。
    #[test]
    fn ko2a_lineage_header_is_optional_per_line() {
        // 今天磁盘上所有预设都没有这三行 ⇒ 必须是 None（不是空 Lineage）
        let plain = read_preset_from_bytes(MINIMAL.to_string()).unwrap();
        assert_eq!(
            plain.lineage, None,
            "K-O2a 红：没有血统的预设不该带一个空壳 Lineage —— \
             『没有血统』与『血统未知』在界面上说的话不一样"
        );

        let full = MINIMAL.replacen(
            "# machine: A1\n",
            "# machine: A1\n\
             # based_on: mkp/A1MF_260628.toml\n\
             # based_on_release_time: 2026-08-19 01:38:13\n\
             # based_on_sha256: 115e061f00\n",
            1,
        );
        let f = read_preset_from_bytes(full.clone()).unwrap();
        let l = f.lineage.clone().expect("三行都在，血统必须有");
        assert_eq!(l.based_on.as_deref(), Some("mkp/A1MF_260628.toml"));
        assert_eq!(
            l.based_on_release_time.as_deref(),
            Some("2026-08-19 01:38:13")
        );
        assert_eq!(l.based_on_sha256.as_deref(), Some("115e061f00"));
        // 血统注释不许影响原有的头字段与 serde 面（回归）
        assert_eq!(f.machine, "A1");
        assert_eq!(f.release_time.as_deref(), Some("2026-08-19 01:38:13"));
        assert_eq!(f.config.toolhead.speed_limit, 70.0);

        // 逐行删掉一行：该项 None，其余照旧，read_preset 仍成功
        for (line, name) in [
            ("# based_on: mkp/A1MF_260628.toml\n", "based_on"),
            (
                "# based_on_release_time: 2026-08-19 01:38:13\n",
                "based_on_release_time",
            ),
            ("# based_on_sha256: 115e061f00\n", "based_on_sha256"),
        ] {
            let partial = full.replacen(line, "", 1);
            assert_ne!(partial, full, "构造失败：没找到 {name} 那一行");
            let l = read_preset_from_bytes(partial)
                .unwrap_or_else(|e| panic!("K-O2a 红：少一行血统不该读不出来（{name}）：{e}"))
                .lineage
                .unwrap_or_else(|| panic!("K-O2a 红：还剩两行，血统不该是 None（删的是 {name}）"));
            let missing = match name {
                "based_on" => l.based_on.is_none(),
                "based_on_release_time" => l.based_on_release_time.is_none(),
                _ => l.based_on_sha256.is_none(),
            };
            assert!(missing, "K-O2a 红：{name} 删了却还读出了值：{l:?}");
        }
    }

    /// `based_on` 不许把 `based_on_x` 那一行吃掉（前缀撞键）。
    #[test]
    fn ko2a_based_on_does_not_swallow_longer_keys() {
        assert_eq!(
            parse_lineage_from_content("# based_on_x: v"),
            None,
            "`based_on_x` 不是我们认的键，不许被当成 based_on"
        );
        let l = parse_lineage_from_content("# based_on_sha256: abc").expect("只有摘要那一行");
        assert_eq!(l.based_on, None, "摘要那一行不许被 based_on 吃掉");
        assert_eq!(l.based_on_sha256.as_deref(), Some("abc"));
    }

    #[test]
    fn header_key_matching_is_strict() {
        // 冒号后空 = 未命中；键后必须紧跟冒号（可隔空白）；大小写敏感
        assert_eq!(parse_machine_from_content("# machine:"), None);
        assert_eq!(
            parse_machine_from_content("# machine: A1"),
            Some("A1".into())
        );
        assert_eq!(parse_machine_from_content("#machine:A1"), Some("A1".into()));
        assert_eq!(
            parse_machine_from_content("#   machine :  A1 "),
            Some("A1".into())
        );
        assert_eq!(parse_machine_from_content("# machine_x: A1"), None);
        assert_eq!(
            parse_machine_from_content("gcode ; machine: A1"),
            None,
            "必须行首 #"
        );
        assert_eq!(
            parse_release_time_from_content("# release_time: t1"),
            Some("t1".into())
        );
    }
}
