//! 参数分流 —— 切片器（BambuStudio）怎么调我们，这里就得怎么认。
//!
//! # 注册进切片器的那一行（首页「复制后处理脚本」复制的就是它）
//!
//! ```text
//! "<本程序>" --Toml "<预设.toml>" --Gcode
//! ```
//!
//! 末尾那个 `--Gcode` **故意不带值**：切片器把 G-code 路径追加在末尾，所以真实 argv 是
//! `… --Toml <预设> --Gcode <gcode>`。这一行的形态不是我们挑的，是切片器那一侧说了算 ——
//! 旧世代 `mkp-ssr` 与成熟版 `mkpsupporte`（`internal/cli/runner_windows.go::parseGcodeArg`）
//! 认的是同一套，这里照抄它的三条取值形态。
//!
//! # 三条取值形态都要认
//!
//! 1. `--Gcode <path>`：正常；
//! 2. `--Gcode on file <path>`：某些切片器会插 `on file`；
//! 3. `--Gcode` 后面不是路径（或者根本没值）⇒ **兜底取 argv 最后一个参数**。
//!
//! 三条都不是洁癖，是实测遇到过的（来源 `mkp-ssr/src-tauri/src/args.rs`，那边逐条记着出处）。
//! `on file` 那条还有一条教训一起搬过来：判据必须**只有被测那一支**能让它通过，否则
//! 兜底那条会把用例接走、把判据变成空转。
//!
//! # 大小写敏感
//!
//! `--toml` 不认（来源实现是精确匹配 `switch arg`）。认了等于把"我们自己生成的固定形态"
//! 放宽成"随便怎么拼都行"，而放宽只会让"为什么这样也能跑"变多。
//!
//! # 只给一半要报错
//!
//! 只有 `--Toml` 或只有 `--Gcode` ⇒ 参数错（退出码 2），**不当成"顺手开界面"**：
//! 静默开出一扇界面会让用户以为后处理跑过了。
//!
//! 其余一切（双击图标、`tauri dev`、将来可能有的别的开关）⇒ [`Mode::Gui`]。

use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;

/// 一次钩子作业：**用这份预设处理这个 G-code**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookJob {
    pub toml: PathBuf,
    pub gcode: PathBuf,
}

/// 这一个进程要干什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// 切片器调我们了（`--Toml` + `--Gcode`）：跑后处理，**跑完就退**（见 [`crate::hook`]）。
    Hook(HookJob),
    /// 界面。什么都不带、或带的是不认识的开关，都走这条。
    Gui,
}

/// 参数错。**只给一半不算"顺手开界面"** —— 那会让用户以为处理成功了。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArgError {
    #[error("给了 --Gcode 但没给 --Toml（两者必须同时提供）")]
    MissingToml,
    #[error("给了 --Toml 但没给 --Gcode（两者必须同时提供）")]
    MissingGcode,
    #[error("--Toml 后面没有路径")]
    TomlValueMissing,
    #[error("--Gcode 后面认不出路径（完整 argv：{argv}）")]
    GcodeUnparsable { argv: String },
}

/// 认得出"这看起来是个路径"就行。判据与旧世代逐字一致：
/// 以 `-` 开头的不是路径（那是开关）；有短扩展名（≤6 字）的算路径；
/// 其余要求"够长且带 `.` 或 `_`"—— 放宽到"任何不含空格的字"会把开关也吃进来。
fn looks_like_path(s: &OsStr) -> bool {
    let s = s.to_string_lossy();
    if s.starts_with('-') {
        return false;
    }
    if let Some(dot) = s.rfind('.') {
        let ext = &s[dot + 1..];
        if !ext.is_empty() && ext.len() <= 6 {
            return true;
        }
    }
    s.len() > 4 && (s.contains('.') || s.contains('_'))
}

/// `--Gcode` 的取值：三种形态 + 兜底（见模块头）。
fn parse_gcode(args: &[OsString], idx: usize) -> Option<PathBuf> {
    let next = args.get(idx + 1);
    // 形态 1：`--Gcode <path>`
    if let Some(n) = next {
        if looks_like_path(n) {
            return Some(PathBuf::from(n));
        }
    }
    // 形态 2：`--Gcode on file <path>`
    let on = next.map(|n| n == "on").unwrap_or(false);
    let file = args.get(idx + 2).map(|a| a == "file").unwrap_or(false);
    if on && file {
        if let Some(c) = args.get(idx + 3) {
            if looks_like_path(c) {
                return Some(PathBuf::from(c));
            }
        }
    }
    // 形态 3：兜底取最后一个 —— 只看"像不像路径"，不挑位置
    args.last().filter(|l| looks_like_path(l)).map(PathBuf::from)
}

/// 解析 argv（**含** argv[0]，与 `std::env::args_os()` 一致）。
pub fn parse<I: IntoIterator<Item = OsString>>(argv: I) -> Result<Mode, ArgError> {
    let args: Vec<OsString> = argv.into_iter().collect();
    let mut toml: Option<PathBuf> = None;
    let mut gcode: Option<PathBuf> = None;

    let mut i = 1; // 跳过 argv[0]（本程序自己）
    while i < args.len() {
        if args[i] == "--Toml" {
            let v = args.get(i + 1).ok_or(ArgError::TomlValueMissing)?;
            if !looks_like_path(v) {
                return Err(ArgError::TomlValueMissing);
            }
            toml = Some(PathBuf::from(v));
            i += 2;
            continue;
        }
        if args[i] == "--Gcode" {
            gcode = Some(parse_gcode(&args, i).ok_or_else(|| ArgError::GcodeUnparsable {
                argv: args
                    .iter()
                    .map(|a| a.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            })?);
            i += 2;
            continue;
        }
        // 不认识的参数一律跳过：**不改语义**（照旧开界面），也不在这里报错 ——
        // 那会让将来给界面加开关时必须先来这里登记一次。
        i += 1;
    }

    match (toml, gcode) {
        (Some(toml), Some(gcode)) => Ok(Mode::Hook(HookJob { toml, gcode })),
        (Some(_), None) => Err(ArgError::MissingGcode),
        (None, Some(_)) => Err(ArgError::MissingToml),
        (None, None) => Ok(Mode::Gui),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<OsString> {
        std::iter::once("mkp-support-ease.exe")
            .chain(items.iter().copied())
            .map(OsString::from)
            .collect()
    }

    /// 正常形态：切片器追加的路径紧跟 `--Gcode`
    #[test]
    fn the_normal_slicer_form_parses() {
        let got = parse(argv(&[
            "--Toml",
            "C:\\Users\\x\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF.toml",
            "--Gcode",
            "C:\\Users\\x\\AppData\\Local\\Temp\\bamboo_model\\45600.0.gcode",
        ]))
        .expect("正常形态必须解析成功");
        assert_eq!(
            got,
            Mode::Hook(HookJob {
                toml: PathBuf::from("C:\\Users\\x\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF.toml"),
                gcode: PathBuf::from("C:\\Users\\x\\AppData\\Local\\Temp\\bamboo_model\\45600.0.gcode"),
            })
        );
    }

    /// 形态 2：`--Gcode on file <path>`。**末尾故意再挂一个非路径参数** ——
    /// 否则形态 3（兜底取末参）会把这条用例接走，判据等于什么都没咬住。
    #[test]
    fn the_on_file_form_parses() {
        let got = parse(argv(&[
            "--Toml",
            "C:\\p\\A1.toml",
            "--Gcode",
            "on",
            "file",
            "C:\\tmp\\a.gcode",
            "--quiet",
        ]))
        .expect("on file 形态必须解析成功");
        match got {
            Mode::Hook(j) => assert_eq!(j.gcode, PathBuf::from("C:\\tmp\\a.gcode")),
            other => panic!("应是 Hook，实测 {other:?}"),
        }
    }

    /// 形态 3：`--Gcode` 后面跟的是开关，路径落在 argv 末尾
    #[test]
    fn the_last_arg_fallback_parses() {
        let got = parse(argv(&[
            "--Toml",
            "C:\\p\\A1.toml",
            "--Gcode",
            "--verbose",
            "C:\\tmp\\b.gcode",
        ]))
        .expect("末参兜底必须解析成功");
        match got {
            Mode::Hook(j) => assert_eq!(j.gcode, PathBuf::from("C:\\tmp\\b.gcode")),
            other => panic!("应是 Hook，实测 {other:?}"),
        }
    }

    /// 只给一半 = 参数错，**不许**退化成界面
    #[test]
    fn only_toml_is_an_error() {
        assert_eq!(
            parse(argv(&["--Toml", "C:\\p\\A1.toml"])),
            Err(ArgError::MissingGcode)
        );
    }

    #[test]
    fn only_gcode_is_an_error() {
        assert_eq!(
            parse(argv(&["--Gcode", "C:\\tmp\\a.gcode"])),
            Err(ArgError::MissingToml)
        );
    }

    /// 小写不认 ⇒ 等于"什么参数都没给" ⇒ 界面（刻意如此，见文件头）
    #[test]
    fn lowercase_flags_are_not_recognized() {
        assert_eq!(
            parse(argv(&["--toml", "C:\\p\\A1.toml", "--gcode", "C:\\tmp\\a.gcode"]))
                .expect("不该报错"),
            Mode::Gui
        );
    }

    #[test]
    fn toml_without_a_value_is_an_error() {
        assert_eq!(
            parse(argv(&["--Gcode", "C:\\tmp\\a.gcode", "--Toml"])),
            Err(ArgError::TomlValueMissing)
        );
    }

    /// 什么都不带 ⇒ 界面（双击图标、`tauri dev` 都是这条）
    #[test]
    fn no_args_is_the_gui() {
        assert_eq!(parse(argv(&[])).expect("不该报错"), Mode::Gui);
    }

    /// 认不出路径的 `--Gcode`：报错并**把完整 argv 带上**（用户要能自己看出拼错在哪）
    #[test]
    fn an_unparsable_gcode_names_the_whole_argv() {
        let got = parse(argv(&["--Toml", "C:\\p\\A1.toml", "--Gcode"]));
        // 兜底会去取末参 `--Gcode` 本身 —— 它以 `-` 开头，所以不是路径
        match got {
            Err(ArgError::GcodeUnparsable { argv }) => {
                assert!(argv.contains("--Gcode"), "argv 要原样带上：{argv}")
            }
            other => panic!("应是 GcodeUnparsable，实测 {other:?}"),
        }
    }
}
