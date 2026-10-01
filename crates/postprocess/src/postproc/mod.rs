//! `postproc` —— 全部后处理算法，按模块分（来源仓库也刻意**不**拆成多个 crate：
//! crate 是架构边界，不是文件夹）。
//!
//! pass1 / pass2 / tower / disk / calibration / printtime / offset / geom /
//! machine_dims / support / cancel，共 31 个文件、12,975 行。
//!
//! 硬边界：公开签名只吃 `&Ir`，**不得出现任何预设/TOML 类型**。这条在来源仓库由 gatecheck 的
//! `postproc_does_not_read_toml` 机械保证（禁 `mkp-preset` / `toml` / `toml_edit` 产品依赖）；
//! 本项目里预设体系整个不存在，所以那条判据**没有对应物可守** —— 它是这次没搬的 10 条门禁之一
//! （spec doc.md §3.1 / Task 13.3）。也正因为这层从来没见过 TOML，抽取才可能做到零算法改动。
//!
//! 搬家时的改动：**只有 217 行路径改写**（`crate::x` → `crate::postproc::x`、
//! `mkp_gcode::` → `crate::gcode::`、`mkp_ir::` → `crate::ir::`、`mkp_diag::` → `crate::diag::`），
//! 算法一行未动。证据是 G1/G2/G3 三份字节判据在接上的**第一次运行就全绿**。

pub mod calibration;
pub mod cancel;
pub mod disk;
pub mod geom;
pub mod machine_dims;
pub mod marks;
pub mod offset;
pub mod pass1;
pub mod pass2;
pub mod printtime;
pub mod support;
pub mod tower;
