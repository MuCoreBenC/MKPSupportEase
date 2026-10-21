//! `PostprocError` —— 后处理内核的唯一错误类型，错误码保持旧实现字符串。
//!
//! 错误码是本程序**对外的稳定契约**（界面与日志都认它），所以码值不随实现变化：
//!
//! - `E_FILE_READ_001` —— 读输入 / 写输出失败（同一个码，读写都归它）
//! - `E_FS_NOT_FOUND_001` —— 预设或输入文件不存在
//! - `E_TOML_PARSE_001` —— TOML 解析失败（含 CRLF 归一重试后仍失败）
//! - `E_CFG_PARSE_001` —— 配置结构对不上，含「预设文件缺少 `# machine:` 声明」
//! - `E_CFG_INVALID_001` —— 配置取值不合法
//! - `E_GCODE_COLLISION_001` —— 输入不是本程序能处理的 G-code
//! - `E_GCODE_FAILED_001` —— 一 / 二遍处理失败
//! - `E_CAL_FAILED_001` —— 校准失败
//! - `E_CAL_MISMATCH_001` / `_002` —— 校准模型被旋转 / 被缩放（与预期不符）
//! - `E_GCODE_BOUNDARY_001` / `_002` —— `calibrationfmt` 那两条边界（打印边界 / 涂胶范围）。
//!   ★ 它们以前**只活在字符串里**（`offset.rs` 把码拼进 message，外面套 `InvalidConfig`），
//!   2026-10-11 起升成结构化码（[`PostprocError::Boundary`]）—— 那正是「外层码在说假话」
//!   （把"模型超出打印边界"报成"配置错"）的来路。
//!
//! 一个诚实偏差，登记在此处而非隐瞒：`Cancelled` 在旧侧**没有**错误码
//! （`StepCancelled` 是特殊分支、非 Diagnostic 路径），Rust 侧为类型统一给出
//! `E_SYS_CANCELLED_001`，格式遵循旧词汇表但**是新造的**，不是复刻。
//!
//! 另外三个同类的新造码**刻意不住这个枚举里**（它们属于钩子层，内核压根不知道有界面）：
//! `E_CFG_MISMATCH_001`（预设与 G-code 机型不一致）、`E_SYS_STALLED_001`（无进展叫停）、
//! `E_SYS_TIMEOUT_001`（整趟超时）—— 定义与文案都在 `messages.toml` 的 `hook.*` 那几条。
//!
//! # 文案住哪
//!
//! **不在这里**。`#[error("…")]` 那种把句子钉在变体上的写法 2026-10-11 全部拆掉，
//! 改成「变体只带**结构化事实**（码 + 参数），句子由 `messages.toml` 那张表渲染」——
//! 理由与形态见 [`crate::diag::messages`]。于是：
//! - 改成 A 句还是 B 句 = 改表，不动代码；
//! - 同一个码可以给切片器一个说法、给界面另一个说法（表里的 `slicer` 字段）；
//! - 上层（钩子）能按同一个码 + 同一份参数**换一种说法再渲一遍**（[`PostprocError::params`]）。

use std::io;

use crate::diag::messages;
use crate::postproc::disk::collision::CollisionFacts;

/// 后处理错误。每个变体都必须能给出 `E_*_NNN` 形式的稳定错误码。
///
/// **变体字段是"事实"，不是"句子"**：`message` / `path` 这些是渲染参数（表里的
/// `{detail}` / `{path}` / `{diagnostics}`），拼接发生在 [`std::fmt::Display`]。
#[derive(Debug)]
pub enum PostprocError {
    /// 文件 IO。错误码按 io::ErrorKind 分流：NotFound → `E_FS_NOT_FOUND_001`
    /// （旧侧用于预设文件缺失），其余 → `E_FILE_READ_001`（旧侧 read_gcode 用）。
    Io {
        /// 操作名（如 `read_gcode` / `read_preset`），进诊断的 operation 字段。
        op: &'static str,
        source: io::Error,
    },

    /// TOML 解析失败（CRLF 归一重试后仍失败），`E_TOML_PARSE_001`。
    TomlParse {
        path: String,
        /// 底层解析器消息（旧侧把 cause 存进 Diagnostic.Cause）。
        message: String,
    },

    /// 机型名不可用（空或不是已知机型），硬错误，`E_CFG_PARSE_001`。
    ///
    /// **文案是有意这么写的**：不写「预设文件缺少 `# machine:` 声明」
    /// 而本项目没有预设文件、也没有头部注释 ——
    /// 机型名是配置里的一个普通字段。照抄那句话会让用户去找一个不存在的东西。
    /// 错误码保持 `E_CFG_PARSE_001` 不变（对外契约）。
    MissingMachine { path: String },

    /// 配置内容非法（校验/回退拒绝），`E_CFG_INVALID_001`。
    InvalidConfig { message: String },

    /// 碰撞检测命中（`disk.DetectTowerCollision`），`E_GCODE_COLLISION_001`。
    ///
    /// `facts` 是**撞在哪 + 建议挪多少 + 画俯视图要的几何**（2026-10-18 才有：
    /// 以前这一支无字段，用户只能读到"撞了"两个字，不知道往哪挪）。
    /// 建议那句也在里面（码 + 表里渲好的正文），上层原样落进执行记录。
    ///
    /// ★ `Box`：**这一支的载荷大**（冲突点采样几百个 `[f64;2]`），直接内联会把整个
    /// 枚举撑到几百字节 —— 于是每个 `Result<_, PostprocError>` 都跟着变大（clippy
    /// `large_enum_variant` / `result_large_err` 当场红）。装箱之后变体回到指针大小。
    Collision { facts: Box<CollisionFacts> },

    /// 校准生成/插入失败，`E_CAL_FAILED_001`。
    Calibration { message: String },

    /// 校准模型形变检测命中（New 变体专属），码由 `code` 字段携带：
    /// `E_CAL_MISMATCH_001`（旋转，）与
    /// `E_CAL_MISMATCH_002`（缩放，）。
    ///
    /// **为什么不复用 `Calibration`**：那个码是「校准代码没生成出来」，
    /// 而这两个是「你的校准模型摆得不对」—— 一个让人去看日志，
    /// 一个让人回切片器把模型转回来。混成一个码等于把可操作的提示变成故障。
    ///
    /// `message` 是**机器判据**（expected/actual/bbox…），进表里的 `{diagnostics}`：
    /// 界面那一面留着（排查用），切片器那一面**去掉**（用户要看的是"别转我的模型"）。
    CalibrationMismatch {
        /// `E_CAL_MISMATCH_001` 或 `E_CAL_MISMATCH_002`，直接作为对外码返回。
        code: &'static str,
        message: String,
    },

    /// pass1/pass2 等扫描处理失败，`E_GCODE_FAILED_001`。
    Processing { message: String },

    /// 输出写入失败（`.part` 写入/rename），旧侧复用 `E_FILE_READ_001`。
    Write { path: String, source: io::Error },

    /// 用户取消。旧侧无码（见模块文档），`E_SYS_CANCELLED_001` 为新造。
    Cancelled { message: String },

    /// 模型越界：`E_GCODE_BOUNDARY_001`（打印边界）/ `E_GCODE_BOUNDARY_002`（涂胶范围）。
    ///
    /// `sides` 是那几个"当前X：…，允许范围：…"从句**渲染好拼起来**的结果 ——
    /// 从句自己的模板也在表里（`boundary.clause` / `boundary.limit.min|max`），
    /// 所以这里不出现任何一个字。为什么拼在这里而不是表里：一条越界可能**两个轴同时**
    /// 超（X 与 Y 各一段），模板语言里没有循环，只能由懂几何的这一层展开。
    Boundary {
        /// 两个边界码之一。
        code: &'static str,
        /// 已拼好的从句串（表里 `["E_GCODE_BOUNDARY_00x"]` 的 `{sides}`）。
        sides: String,
    },
}

impl PostprocError {
    /// 稳定错误码（旧实现对外契约字符串）。
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
            Self::Boundary { code, .. } => code,
        }
    }

    /// 渲染参数：表里的 `{名字}` 取什么值。
    ///
    /// **为什么要交出来**（而不是只留一个 [`std::fmt::Display`]）：上层要拿同一个码换一种
    /// 说法再渲一遍 —— 切片器那一面用表里的 `slicer` 字段（比如把校准形变那串机器判据去掉）。
    /// 不给参数的话，上层就只能把渲染好的句子**反着拆**（旧版 `hook.rs::split_code`
    /// 干的就是这个，也正是"两层码糊在一句里"的来路）。
    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            /* `target` 一个槽位两种来源：读侧（Io）是操作名，写侧（Write）是落点路径 ——
            表里的模板只需要"是谁"这一格，两个变体共用同一个码（旧侧就这么复用的）。 */
            Self::Io { op, source } => {
                vec![("target", (*op).to_owned()), ("detail", source.to_string())]
            }
            Self::TomlParse { path, message } => {
                vec![("path", path.clone()), ("detail", message.clone())]
            }
            Self::MissingMachine { path } => vec![("path", path.clone())],
            Self::InvalidConfig { message } => vec![("detail", message.clone())],
            Self::Collision { .. } => Vec::new(),
            Self::Calibration { message } => vec![("detail", message.clone())],
            Self::CalibrationMismatch { message, .. } => vec![("diagnostics", message.clone())],
            Self::Processing { message } => vec![("detail", message.clone())],
            Self::Write { path, source } => {
                vec![("target", path.clone()), ("detail", source.to_string())]
            }
            Self::Cancelled { message } => vec![("detail", message.clone())],
            Self::Boundary { sides, .. } => vec![("sides", sides.clone())],
        }
    }

    /// 表渲染的正文（本程序里给用户看的那一句）。等价于 `self.to_string()`，
    /// 起个名字是给调用点读的：`err.body()` 比 `err.to_string()` 更容易看出
    /// "这句话是从表里来的"。
    pub fn body(&self) -> String {
        self.to_string()
    }

    /// **碰撞那一份事实**（2026-10-18）：只有 `Collision` 这一支有。
    ///
    /// 上层（钩子）拿它写进执行记录的 `collision` 一格；别的错误一律 `None`
    /// （于是"失败记录有没有俯视图"是数据说了算，不是某一层 if 说了算）。
    pub fn collision_facts(&self) -> Option<&CollisionFacts> {
        match self {
            Self::Collision { facts } => Some(facts.as_ref()),
            _ => None,
        }
    }
}

/// **句子从表里来**：本 impl 只负责把变体字段翻成参数名（见 [`PostprocError::params`]）。
impl std::fmt::Display for PostprocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params = self.params();
        let refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        f.write_str(&messages::text(self.code(), &refs))
    }
}

impl std::error::Error for PostprocError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::Write { source, .. } => Some(source),
            _ => None,
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
                facts: Box::new(CollisionFacts::default()),
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
            PostprocError::Boundary {
                code: "E_GCODE_BOUNDARY_001",
                sides: "当前Y：181.4mm，允许范围：≤180.0mm".into(),
            },
            PostprocError::Boundary {
                code: "E_GCODE_BOUNDARY_002",
                sides: "当前X：10.0mm，允许范围：≥20.0mm".into(),
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

    /// **每个码在文案表里都要有一条**（键就是码）。
    ///
    /// 这条是「把文案从代码里赶出去」之后的接力棒：以前加变体只要跟着写一行 `#[error]`，
    /// 现在不写表就没有句子 —— 而那种缺法在运行时的表现是用户看到
    /// `［未登记的文案 E_XXX_001］`。判据在这里，构建期就红。
    #[test]
    fn every_variant_has_a_message_entry() {
        for err in all_variants() {
            let code = err.code();
            assert!(
                messages::has(code),
                "{code} 在 messages.toml 里没有条目 —— 新增码必须同时补表"
            );
        }
    }

    /// 渲染出来的正文**不含错误码**：码是单独的字段（`code()`），句子给人读。
    ///
    /// 这条防的是旧毛病复发 —— 曾经 message 里带着 `E_XXX_001: ` 的自我引用，
    /// 用户在一句话里读到两个码，谁也不知道哪个才是原因。
    #[test]
    fn the_rendered_sentence_never_carries_a_code() {
        for err in all_variants() {
            let body = err.body();
            assert!(!body.contains(err.code()), "{body} 里又出现了码");
            assert!(!body.contains("未登记的文案"), "{body} 来自表里的兜底分支");
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
                facts: Box::new(CollisionFacts::default())
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

    /// 越界那两条：**码自己说话**，句子由表渲染。
    ///
    /// 旧形态是「`InvalidConfig` 里塞了一句带着 `E_GCODE_BOUNDARY_001:` 的正文」——
    /// 于是外层码（E_CFG_INVALID_001 = "配置无效"）在说假话。
    #[test]
    fn the_boundary_carries_its_own_code_and_no_prefix_lie() {
        let boundary = PostprocError::Boundary {
            code: "E_GCODE_BOUNDARY_001",
            sides: "当前X：185.9mm，允许范围：≤180.0mm".into(),
        };
        assert_eq!(boundary.code(), "E_GCODE_BOUNDARY_001");
        let body = boundary.body();
        assert_eq!(body, "模型超出打印边界，当前X：185.9mm，允许范围：≤180.0mm");
        assert!(!body.contains("配置无效"), "外层那句假话又回来了：{body}");
    }

    /// `params()` 的名字与表的模板**对齐**：把参数塞回去渲染，句子里的占位符必须被填满。
    ///
    /// 这条咬的是"改名忘改表"：`params()` 交出去的名字是上层换面重渲的契约，
    /// 名字漂了的表现是用户那边多出一截 `{detail}`。
    #[test]
    fn params_fill_the_template() {
        for err in all_variants() {
            let params = err.params();
            let refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let rendered = messages::text(err.code(), &refs);
            assert!(!rendered.contains('{'), "{rendered} 还有没填上的占位符");
            assert_eq!(rendered, err.body(), "两条渲染路径给了不一样的话");
        }
    }
}
