//! `mkpse-preset` —— 预设 TOML（snake_case 用户参数）→ 内核 IR（PascalCase）的**唯一**映射点。
//!
//! 来源：`/Users/wzy/projects/mkp-rust/mkp-sr` 的 `crates/preset`（model / read /
//! registry / validate）与 `crates/ir/src/build.rs`。IR 类型**不搬**，直接吃
//! `postprocess::ir` 那份（内核已有，且被 A 档字节判据守着）。
//!
//! 为什么这条链必须存在：用户手上的预设长这样
//! （`/Users/wzy/Documents/MKPSupportSSR/presets/mkp/A1MF.toml`）
//!
//! ```text
//! # machine: A1_MINI
//! [toolhead]
//! speed_limit = 70          # mm/s
//! [wiping]
//! have_wiping_components = "tower"
//! ```
//!
//! 而内核的配置文件**就是 IR**（`SchemaVersion = 1` / `[Disk]` / `Toolhead.MaxSpeed` 是 mm/min）。
//! 两者不是同一个东西；后处理内核当初刻意把映射整层删掉了，本 crate 把它补回来。
//!
//! **两张脸的对外契约**（顺序不许重排，理由见 [`load_ir`]）：
//! - [`read_preset`] / [`validate`] / [`registry`]：预设文件侧；
//! - [`build::build`]：`TomlConfig` → `Ir` 的翻译本体。
//!
//! **不搬**：`ui.rs`（UI 元数据）。
//!
//! **立场变更（spec `preset-editing-amber-labels`）**：原来这里写着「本项目只读预设」，
//! 现在**改预设了** —— [`write`] 用 `toml_edit` 就地换值（保注释、保键序、保形态）。
//! 翻案的理由：GUI 的下一步就是让用户改参数，而唯一诚实的做法是改**用户的预设文件**
//! （真源单一、可回滚），不是另造一份配置。
//! 代价照实说：多了一整类「改坏用户预设」的事故面。兜着它的是三条判据
//! （K-P1 零编辑往返逐字节相同 / K-P2 单键改值只有那一行变 / K-P3 拒绝新增键）
//! 加上落盘前的 `read_preset` + `load_ir` 复检与写前备份。

pub mod build;
pub mod generate;
pub mod lineage;
pub mod model;
pub mod read;
pub mod recipe;
pub mod recipe_edit;
pub mod registry;
/// 改注册表的区间与步进（**只在开发态有意义**：注册表编进二进制，见模块头）。
pub mod registry_edit;
pub mod validate;
pub mod write;

pub use build::{CalibrationExecMode, build};
/// 产物 / 交付文件名的**唯一命名实现**。规范见 `docs/ARCHITECTURE.md` §10。
///
/// 从 crate 根导出，是为了让 `src-tauri` 侧（工作台的生成与发布）能复用同一份规则，
/// 而不是各拼一遍字符串。
pub use generate::file_name as preset_file_name;
pub use model::{
    IntOrFloat, Lineage, PresetFile, PresetKey, TomlConfig, ToolheadConfig, ToolheadOffset,
    WipingConfig,
};
pub use read::{
    parse_lineage_from_content, parse_variant_from_content, read_preset, read_preset_from_bytes,
};
pub use registry::{
    EffectiveRange, MachineBounds, ParamEntry, RangeSource, Registry, load_param_registry,
};
pub use write::{Edit, EditValue, KeySnapshot, apply_edits, snapshot};

/// param_registry 快照。
///
/// **指向仓库里那份唯一真源** `presets/registry/param_registry.toml`（Task 15 / M4c）。
/// 搬进来时 `crates/preset/assets/` 下那份 56 KB 副本已经删掉 —— 两份数据只差 5 处
/// （两处 label、两处 `uiComponent`、一块 `[[params.choices]]`），而那种差异
/// 在界面上看得见、在判据里看不见，留着就是把双真相固化。
///
/// 仍然是 `include_str!` 编进二进制、**不走任何分发管线**（换参数集 = 出新版本）。
/// 「改成运行时从数据根读」与 M3 欠的那次 `machine_dims::install()` 接线一起做（M5）：
/// `load_param_registry()` 有 20 多处调用点，混进搬运这一批会把它变成重构。
///
/// 消费方：`build()` 的零值默认与范围校验、弃用参数检查。
pub const PARAM_REGISTRY_TOML: &str = include_str!("../../../presets/registry/param_registry.toml");

/// **内置预设**：由配方生成、随版本走的那 9 份（文件名 + 内容）。
///
/// `include_str!` 编进二进制、**不走任何分发管线**（与参数注册表同一条路：
/// 换预设 = 出新版本，零网络）。消费方是 `src-tauri` 的 `builtin::sync_builtin_presets`，
/// 它把这些内容铺到 `<数据根>/presets/builtin/` —— 那是只读目录，整体覆盖是安全的。
///
/// **这张表是手写的**（`include_str!` 的路径必须是字面量），漏一份不会报错，
/// 所以判据咬两头：`gen-presets --check` 保证入库产物与配方一致，
/// `tests/builtin_presets_match_dir.rs` 保证这张表与入库目录**一份不差**
/// （文件名集合相同 + 每条内容与同名文件逐字节相等 + 无重名）。
pub const BUILTIN_PRESETS: &[(&str, &str)] = &[
    (
        "A1-standard.toml",
        include_str!("../assets/presets/A1-standard.toml"),
    ),
    (
        "A1-fast.toml",
        include_str!("../assets/presets/A1-fast.toml"),
    ),
    (
        "A1-fastv3.3.toml",
        include_str!("../assets/presets/A1-fastv3.3.toml"),
    ),
    (
        "A1_MINI-standard.toml",
        include_str!("../assets/presets/A1_MINI-standard.toml"),
    ),
    (
        "A1_MINI-fast.toml",
        include_str!("../assets/presets/A1_MINI-fast.toml"),
    ),
    (
        "A1_MINI-fastv3.3.toml",
        include_str!("../assets/presets/A1_MINI-fastv3.3.toml"),
    ),
    (
        "P1S-lite.toml",
        include_str!("../assets/presets/P1S-lite.toml"),
    ),
    (
        "P2S-standard.toml",
        include_str!("../assets/presets/P2S-standard.toml"),
    ),
    (
        "X1C-lite.toml",
        include_str!("../assets/presets/X1C-lite.toml"),
    ),
];

/// 预设 TOML → **可以直接喂 `pipeline::process_with_ir` 的 IR**。
///
/// 这是本 crate 对外的唯一入口；`build()` 等零件仍是公开的，但产品路径只走这一条。
///
/// # 顺序照抄，不许重排
///
/// 逐步对应 `mkp-sr/crates/engine/src/lib.rs:187..215`（那 11 行是行为规格）：
///
/// | 步 | 做什么 | 为什么在这个位置 |
/// |---|---|---|
/// | 1 | [`read_preset`] | 头注释 `# machine:` 缺失即 `E_CFG_PARSE_001`；CRLF 两段式重试；顺带读 `# variant:`（缺失不报错） |
/// | 1.5 | 机型规范名 = `normalize_to_canonical(machine)`，**只算不写** | 第 4 步要按 `机型:变体` 查区间覆盖。归一失败在这里**不报错**（报错点留在第 8 步，那边的消息点名了写错的值与尺寸表缺口）；拿不到规范名就当没有机型信息 ⇒ 区间回落全局，只会更宽不会误拦 |
/// | 2 | [`validate::validate_config`] | **吃 TOML 原值**（速度还是 mm/s）。放到 build 之后就没法按用户单位报区间 |
/// | 3 | [`load_param_registry`] | 只解析一次，后面两处共用 |
/// | 4 | [`validate::validate_against_registry`]（吃规范名 + 变体） | 同上，仍在原值域；区间取自注册表的**有效区间**（`机型:变体` 覆盖优先，否则全局） |
/// | 5 | 机型非空检查 | 空机型往下走会得到 0×0 的运动范围，报错点离真因很远 |
/// | 6 | [`build`] | 映射本体：零值兜底 → 逐字段映射 → 兼容迁移 → 派生 → 范围校验 |
/// | 7 | `meta.preset_name` = **带扩展名**的文件名 | 来源实测就是带 `.toml`，这是对外可见字符串 |
/// | 8 | `machine_type` 归一 + 尺寸表命中检查 | 别名认识 ≠ 尺寸表里有（见下）。**这里才是归一的报错点** |
/// | 9 | [`postprocess::pipeline::fill_machine_facts`] | 机型维度 + 禁区 + G-code 自报机型 |
///
/// 第 9 步**刻意复用内核那个函数**而不是在这里再写一遍查表：那三件事在 CLI 路径上也要做，
/// 两处实现必然漂移。
///
/// `fill_defaults`（零值兜底那批）**不在这里调**：时机是「G-code 元数据提取之后」，
/// 属于编排层，`build()` 也刻意不含它 —— 与来源仓库同一边界。
///
/// # 硬错误，不兜底
///
/// - 机型别名不认识 ⇒ `MissingMachine`，点名那个写错的值；
/// - 别名认识但**内置尺寸表没有**（实测 aliasMap 有 `A2L`、`machine_dimensions.json`
///   只有 5 个机型）⇒ 也是 `MissingMachine`。不许留 0×0 的运动范围继续跑：
///   内核那边曾因此对一台 0×0 的机器说「配置可用」，真跑才在边界检查处失败。
///
/// `gcode_text = None` ⇒ `gcode_machine_type` 填 `"UNKNOWN"`（与「检测不到」同一条兜底，
/// 不新造第三种取值）。GUI 只想看预设摘要时传 `None`。
pub fn load_ir(
    preset_path: &std::path::Path,
    gcode_text: Option<&str>,
) -> Result<postprocess::ir::Ir, postprocess::diag::PostprocError> {
    use postprocess::diag::PostprocError;
    use postprocess::postproc::machine_dims::{has_machine_dimensions, normalize_to_canonical};

    // 1
    let file = read_preset(preset_path)?;
    // 1.5 —— **只算不写**规范名：第 4 步要按机型查区间覆盖，而 `ir` 的归一仍在第 8 步原地做。
    // 归一失败在这里**不报错**：报错点留在第 8 步（那边的消息点名了写错的值与尺寸表缺口），
    // 这里拿不到规范名就当「没有机型信息」，区间回落全局 —— 校验只会更宽，不会误拦。
    let canonical_machine = normalize_to_canonical(file.machine.trim());
    // 2
    validate::validate_config(&file.config)?;
    // 3
    let registry = load_param_registry();
    // 4
    validate::validate_against_registry(
        &file.config,
        &registry,
        &canonical_machine,
        file.variant.as_deref(),
    )?;
    // 5
    if file.machine.trim().is_empty() {
        return Err(PostprocError::MissingMachine {
            path: format!("{}（预设头缺 `# machine:`）", preset_path.display()),
        });
    }
    // 6 —— 走 Some(registry) 那一支（钩子的真实路径；差异判据见 tests/registry_branch_diff.rs）
    let mut ir = build(&file.config, Some(&registry), CalibrationExecMode::Fallback)?;
    // 7
    ir.meta.preset_name = preset_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    // 8
    let raw_machine = file.machine.trim().to_string();
    ir.machine.machine_type = normalize_to_canonical(&raw_machine);
    if ir.machine.machine_type.is_empty() {
        return Err(PostprocError::MissingMachine {
            path: format!(
                "{}（`# machine: {raw_machine}` 不是已知机型）",
                preset_path.display()
            ),
        });
    }
    if !has_machine_dimensions(&ir.machine.machine_type) {
        return Err(PostprocError::MissingMachine {
            path: format!(
                "{}（`# machine: {raw_machine}` 归一为 {:?}，但内置机型尺寸表里没有它 —— \
                 别名表认识这个名字、尺寸表却没有对应条目，属于内置数据不一致，请换一个机型）",
                preset_path.display(),
                ir.machine.machine_type
            ),
        });
    }
    // 9
    postprocess::pipeline::fill_machine_facts(&mut ir, gcode_text);
    Ok(ir)
}
