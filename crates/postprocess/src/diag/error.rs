//! `PostprocError` —— 后处理内核的唯一错误类型，错误码保持旧 Go 侧字符串。
//!
//! 旧码对照（全部实测自旧仓库）：
//! - `E_FILE_READ_001`  —— `engine/pipeline_failure.go:62`（read_gcode）与 `:132`
//!   （write_output；`codes.go:195` 注文明说 filesystem 模块复用它表示读写失败）
//! - `E_FS_NOT_FOUND_001` —— `codes.go:204`，预设/输入文件不存在
//! - `E_TOML_PARSE_001` —— `config/read.go:259` CRLF 归一重试仍失败
//! - `E_CFG_PARSE_001` —— `pipeline_failure.go:51`（ConfigError 包裹，含
//!   `process.go:242` 的「预设文件缺少 # machine: 声明」）
//! - `E_CFG_INVALID_001` —— `pipeline_failure.go:76`
//! - `E_GCODE_COLLISION_001` —— `pipeline_failure.go:87`
//! - `E_GCODE_FAILED_001` —— `pipeline_failure.go:98/:109`（first_pass/second_pass）
//! - `E_CAL_FAILED_001` —— `pipeline_failure.go:120`
//! - `E_CAL_MISMATCH_001` —— `internal/disk/generator_new.go:54`（宽高互换）、`:245`、
//!   `:432`（质心判 180°）、`:503`（质心判 90°），校准模型被旋转
//! - `E_CAL_MISMATCH_002` —— `internal/disk/generator_new.go:69`、`:260`，校准模型被缩放
//!
//! 一个诚实偏差，登记在此处而非隐瞒：`Cancelled` 在旧侧**没有**错误码
//! （`StepCancelled` 是特殊分支、非 Diagnostic 路径），Rust 侧为类型统一给出
//! `E_SYS_CANCELLED_001`，格式遵循旧词汇表但**是新造的**，不是复刻。
//!
//! 另一个同类的新造码不在这个枚举里：`E_CAL_EXEC_NO_ANSWER_001`
//! （「需要选择校准执行方式，但没等到答复」）属于**钩子层**，定义在
//! `src-tauri/src/hook.rs` 的 `E_CAL_EXEC_NO_ANSWER`。内核压根不知道有界面 ——
//! 给它在这里加一个变体等于把 GUI 的事塞进后处理内核。

use std::io;

/// 后处理错误。每个变体都必须能给出 `E_*_NNN` 形式的稳定错误码。
#[derive(Debug, thiserror::Error)]
pub enum PostprocError {
    /// 文件 IO。错误码按 io::ErrorKind 分流：NotFound → `E_FS_NOT_FOUND_001`
    /// （旧侧用于预设文件缺失），其余 → `E_FILE_READ_001`（旧侧 read_gcode 用）。
    #[error("{op}: {source}")]
    Io {
        /// 操作名（如 `read_gcode` / `read_preset`），进诊断的 operation 字段。
        op: &'static str,
        #[source]
        source: io::Error,
    },

    /// TOML 解析失败（CRLF 归一重试后仍失败），`E_TOML_PARSE_001`。
    #[error("TOML 解析失败 {path}: {message}")]
    TomlParse {
        path: String,
        /// 底层解析器消息（旧侧把 cause 存进 Diagnostic.Cause）。
        message: String,
    },

    /// 机型名不可用（空或不是已知机型），硬错误，`E_CFG_PARSE_001`。
    ///
    /// **文案与来源仓库不同，是有意改的**：那边写「预设文件缺少 `# machine:` 声明」
    /// （`process.go:242` 的对位），而本项目没有预设文件、也没有头部注释 ——
    /// 机型名是配置里的一个普通字段。照抄那句话会让用户去找一个不存在的东西。
    /// 错误码保持 `E_CFG_PARSE_001` 不变（对外契约）。
    #[error("机型不可用: {path}")]
    MissingMachine { path: String },

    /// 配置内容非法（校验/回退拒绝），`E_CFG_INVALID_001`。
    #[error("配置无效: {message}")]
    InvalidConfig { message: String },

    /// 碰撞检测命中（`disk.DetectTowerCollision`），`E_GCODE_COLLISION_001`。
    #[error("碰撞检测命中: {message}")]
    Collision { message: String },

    /// 校准生成/插入失败，`E_CAL_FAILED_001`。
    #[error("校准失败: {message}")]
    Calibration { message: String },

    /// 校准模型形变检测命中（New 变体专属），码由 `code` 字段携带：
    /// `E_CAL_MISMATCH_001`（旋转，`generator_new.go:54/245/432/503`）与
    /// `E_CAL_MISMATCH_002`（缩放，`generator_new.go:69/260`）。
    ///
    /// **为什么不复用 `Calibration`**：那个码是「校准代码没生成出来」，
    /// 而这两个是「你的校准模型摆得不对」—— 一个让人去看日志，
    /// 一个让人回切片器把模型转回来。混成一个码等于把可操作的提示变成故障。
    #[error("校准模型不匹配: {message}")]
    CalibrationMismatch {
        /// `E_CAL_MISMATCH_001` 或 `E_CAL_MISMATCH_002`，直接作为对外码返回。
        code: &'static str,
        message: String,
    },

    /// pass1/pass2 等扫描处理失败，`E_GCODE_FAILED_001`。
    #[error("处理失败: {message}")]
    Processing { message: String },

    /// 输出写入失败（`.part` 写入/rename），旧侧复用 `E_FILE_READ_001`。
    #[error("写入 {path} 失败: {source}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },

    /// 用户取消。旧侧无码（见模块文档），`E_SYS_CANCELLED_001` 为新造。
    #[error("已取消: {message}")]
    Cancelled { message: String },
}

impl PostprocError {
    /// 稳定错误码（旧 Go 侧对外契约字符串）。
    ///
    /// 穷尽性保障：本 match **刻意不写通配臂** —— 新增变体忘了给码是编译错误，
    /// 这就是 tasks.md 2.3 要求的「编译或测试失败」中的编译分支；
    /// 测试分支见 `tests` 里对 `all_variants()` 的格式/唯一性断言。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { source, .. } => {
                if source.kind() == io::ErrorKind::NotFound {
                    "E_FS_NOT_FOUND_001"
                } else {
                    "E_FILE_READ_001"
                }
            }
            Self::TomlParse { .. } => "E_TOML_PARSE_001",
            Self::MissingMachine { .. } => "E_CFG_PARSE_001",
            Self::InvalidConfig { .. } => "E_CFG_INVALID_001",
            Self::Collision { .. } => "E_GCODE_COLLISION_001",
            Self::Calibration { .. } => "E_CAL_FAILED_001",
            Self::CalibrationMismatch { code, .. } => code,
            Self::Processing { .. } => "E_GCODE_FAILED_001",
            Self::Write { .. } => "E_FILE_READ_001",
            Self::Cancelled { .. } => "E_SYS_CANCELLED_001",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 每个变体各造一个实例，供格式/唯一性断言遍历。
    /// 新增变体时必须同步扩这里与 `code()` 的 match —— 前者由本测试的
    /// 「变体计数」注释提醒，后者由编译器强制。
    fn all_variants() -> Vec<PostprocError> {
        vec![
            PostprocError::Io {
                op: "read_gcode",
                source: io::Error::other("boom"),
            },
            PostprocError::Io {
                op: "read_preset",
                source: io::Error::from(io::ErrorKind::NotFound),
            },
            PostprocError::TomlParse {
                path: "A1.toml".into(),
                message: "bad".into(),
            },
            PostprocError::MissingMachine {
                path: "A1.toml".into(),
            },
            PostprocError::InvalidConfig {
                message: "bad".into(),
            },
            PostprocError::Collision {
                message: "hit".into(),
            },
            PostprocError::Calibration {
                message: "cal".into(),
            },
            PostprocError::CalibrationMismatch {
                code: "E_CAL_MISMATCH_001",
                message: "rotated".into(),
            },
            PostprocError::CalibrationMismatch {
                code: "E_CAL_MISMATCH_002",
                message: "scaled".into(),
            },
            PostprocError::Processing {
                message: "proc".into(),
            },
            PostprocError::Write {
                path: "out.gcode".into(),
                source: io::Error::other("disk full"),
            },
            PostprocError::Cancelled {
                message: "ctrl-c".into(),
            },
        ]
    }

    /// tasks.md 2.3 的测试分支：每个变体的码都必须非空、格式合法、互不碰撞
    /// （Io 的两档除外——那是旧侧本来就有的复用），且三个契约码逐字保持。
    #[test]
    fn every_variant_has_unique_wellformed_code() {
        let mut seen: HashSet<&str> = HashSet::new();
        for err in all_variants() {
            let code = err.code();
            assert!(!code.is_empty(), "{err:?} 的码为空");
            assert!(code.starts_with("E_"), "{err:?} 的码 {code} 不以 E_ 开头");
            let digits: String = code
                .chars()
                .rev()
                .take(3)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            assert!(
                digits.chars().all(|c| c.is_ascii_digit()) && digits.len() == 3,
                "{err:?} 的码 {code} 末段不是三位数字"
            );
            // 唯一性：Io/NotFound 与 Io/其他 允许同码之外的碰撞都不允许。
            // 这里用 (变体名, 码) 对去重更精确，但简化为：非 Io 变体码不重复。
            if !matches!(err, PostprocError::Io { .. }) {
                assert!(seen.insert(code), "错误码 {code} 被两个非 Io 变体共用");
            }
        }
    }

    /// 三个对外契约码逐字保持（tasks.md 2.3 点名的三个）。
    #[test]
    fn contract_codes_are_byte_identical() {
        assert_eq!(
            PostprocError::TomlParse {
                path: String::new(),
                message: String::new()
            }
            .code(),
            "E_TOML_PARSE_001"
        );
        assert_eq!(
            PostprocError::Io {
                op: "read_preset",
                source: io::Error::from(io::ErrorKind::NotFound)
            }
            .code(),
            "E_FS_NOT_FOUND_001"
        );
        assert_eq!(
            PostprocError::Collision {
                message: String::new()
            }
            .code(),
            "E_GCODE_COLLISION_001"
        );
    }

    /// NotFound 分流：同一个 Io 变体按 io::ErrorKind 给出不同码。
    #[test]
    fn io_error_kind_routes_the_code() {
        let not_found = PostprocError::Io {
            op: "read_preset",
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        let other = PostprocError::Io {
            op: "read_gcode",
            source: io::Error::other("permission denied"),
        };
        assert_eq!(not_found.code(), "E_FS_NOT_FOUND_001");
        assert_eq!(other.code(), "E_FILE_READ_001");
    }

    /// 校准模型形变检测的两个码（`generator_new.go` 的 New 变体）必须能取到，
    /// 且**不许**退化成 `E_CAL_FAILED_001` —— 用户看到的那一行是靠码区分
    /// 「生成失败」和「你把校准模型转了/缩放了」的。
    #[test]
    fn calibration_mismatch_codes_are_distinguishable() {
        let rotated = PostprocError::CalibrationMismatch {
            code: "E_CAL_MISMATCH_001",
            message: "检测到校准模型可能被旋转。请勿旋转校准模型。".into(),
        };
        let scaled = PostprocError::CalibrationMismatch {
            code: "E_CAL_MISMATCH_002",
            message: "检测到校准模型尺寸已变化。请勿缩放校准模型，仅允许移动位置。".into(),
        };
        assert_eq!(rotated.code(), "E_CAL_MISMATCH_001");
        assert_eq!(scaled.code(), "E_CAL_MISMATCH_002");
        assert_ne!(
            rotated.code(),
            PostprocError::Calibration {
                message: String::new()
            }
            .code(),
            "形变检测的码被 E_CAL_FAILED_001 吃掉了"
        );
    }
}
