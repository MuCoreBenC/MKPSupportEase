//! 内置默认的**定义类**源文件 —— 客户端第一次启动时铺进自己的数据根。
//!
//! # 为什么用 `include_str!` 而不是运行时读仓库
//!
//! 用户装完 SupportEase，机器上**没有这个仓库**。所以定义类文件的唯一来源只能是
//! 编译期打进二进制的那一份。`include_str!` 在编译时把文本编进去，运行时与仓库无关。
//!
//! # 这里只有「定义」，没有「载荷」
//!
//! `assets.toml` 里的路径指向资产**文件本体**（机型图、图标、BBS profile）。这些不铺：
//! 机型图前端走 vite 产物按 URL 取，不受影响；BBS 与文件大小 / 修改时间本轮诚实留空
//! （契约本来就允许，界面显示「未知」，见 `.trae/documents/preset-page-native-plan.md` 第 4 节 ②）。
//!
//! `mkp/` 是**用户产物区**（下载 / 导入的 MKP 预设落那里），初始为空 ——
//! 由 [`super::paths::seed_if_absent`] 建出空目录，一个文件都不放。
//!
//! # 改这份清单时
//!
//! 它必须与 [`crate::presetdata::Presets::load_from`] 期望的目录结构一致：少一份定义，
//! 客户端第一屏就是空的（或直接加载失败）。`paths.rs` 里那条「铺完能读起来」的测试盯着它。

/// 相对**预设根**（`<appDataDir>/presets`）的路径 → 文本。
///
/// 顺序按目录分组，与仓库 `presets/` 的布局一致：根下四份定义、`machines/`、
/// `forbidden_zones/` 各一台机器一份、`registry/` 一份。
pub const FILES: &[(&str, &str)] = &[
    ("brands.toml", include_str!("../../../presets/brands.toml")),
    (
        "machines/A1.toml",
        include_str!("../../../presets/machines/A1.toml"),
    ),
    (
        "machines/A1_MINI.toml",
        include_str!("../../../presets/machines/A1_MINI.toml"),
    ),
    (
        "machines/P1S.toml",
        include_str!("../../../presets/machines/P1S.toml"),
    ),
    (
        "machines/P2S.toml",
        include_str!("../../../presets/machines/P2S.toml"),
    ),
    (
        "machines/X1C.toml",
        include_str!("../../../presets/machines/X1C.toml"),
    ),
    // 禁区只有三台机器有（A1 与 A1_MINI 没有）
    (
        "forbidden_zones/P1S.toml",
        include_str!("../../../presets/forbidden_zones/P1S.toml"),
    ),
    (
        "forbidden_zones/P2S.toml",
        include_str!("../../../presets/forbidden_zones/P2S.toml"),
    ),
    (
        "forbidden_zones/X1C.toml",
        include_str!("../../../presets/forbidden_zones/X1C.toml"),
    ),
    ("assets.toml", include_str!("../../../presets/assets.toml")),
    (
        "bundles.toml",
        include_str!("../../../presets/bundles.toml"),
    ),
    (
        "layout_schema.toml",
        include_str!("../../../presets/layout_schema.toml"),
    ),
    (
        "registry/param_registry.toml",
        include_str!("../../../presets/registry/param_registry.toml"),
    ),
];
