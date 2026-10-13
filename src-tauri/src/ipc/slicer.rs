//! 「用哪个切片软件打开模型」（2026-10-13）。
//!
//! 前端那一格（设置页「切片」+ 打开模型框里的「仅这一次」）只给一个 **id**：
//! `system` / `bambu` / `orca` —— 与 `src/app/shared/slicer.ts` 的 `SlicerId` 逐字对齐，
//! **id 是契约**。找应用、拼命令行都在这一侧，前端既不碰路径也不猜安装位置。
//!
//! # 三档各自是什么意思
//!
//! - `system`：交给**系统关联程序**（`.3mf` 双击是什么就是什么）—— 走 opener 插件，
//!   与这一条命令原来的行为一字不差；
//! - `bambu` / `orca`：**指定的那个应用**。用户明确点了哪一档才走到这里，
//!   所以"没装那个应用"要**照实报**（`NOT_FOUND` + 一句人话），**不许静默回落**成
//!   系统关联 —— 那等于把用户的选择当没听见，界面还会以为真按他说的开了。
//!
//! # 为什么是"候选路径"而不是一个写死的路径
//!
//! 两个都是用户自己装的第三方软件，装在哪不由我们定：macOS 有 `/Applications`
//! 与 `~/Applications` 两处；Windows 有系统级 `Program Files` 与用户级
//! `%LOCALAPPDATA%\Programs` 两种安装位（OrcaSlicer 的安装器两种都提供）。
//! 逐个 `exists()`，第一个命中的说了算；一个都没命中就是"没装"，不挑一个最近的猜。
//!
//! 认不出的 id **整次拒绝**（`INVALID_ARGUMENT`），理由同上：宁可报错也不悄悄换一档。

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 前端那一格的三个 id。**与 `src/app/shared/slicer.ts` 的 `SlicerId` 一一对应**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicerKind {
    /// 跟随系统关联程序（默认；不传参数时也是这一档）
    System,
    /// Bambu Studio
    Bambu,
    /// OrcaSlicer
    Orca,
}

impl SlicerKind {
    /// 认 id。`None` / 空串 = 没传 = 跟随系统（旧调用一个都不用改）
    pub fn parse(raw: Option<&str>) -> Result<Self, AppError> {
        match raw.map(str::trim) {
            None | Some("") | Some("system") => Ok(Self::System),
            Some("bambu") => Ok(Self::Bambu),
            Some("orca") => Ok(Self::Orca),
            Some(other) => Err(AppError::invalid_argument(format!(
                "不认识的切片软件：{other}（只有 system / bambu / orca 三档）"
            ))),
        }
    }

    /// 给日志与错误话术用的 id（与前端那份同字）
    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Bambu => "bambu",
            Self::Orca => "orca",
        }
    }

    /// 给人看的名字（错误话术里用）
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "系统默认程序",
            Self::Bambu => "Bambu Studio",
            Self::Orca => "OrcaSlicer",
        }
    }
}

/// 这台机器上那个应用的可能落脚（顺序即优先级）。`system` 这一档没有落脚 —— 它走系统关联。
///
/// macOS 给的是 **.app 包**（启动时交给 `/usr/bin/open -a`，让 LaunchServices 处理
/// "已经在跑就复用那一个实例"）；Windows 给的是 **exe**（直接起进程）。
#[cfg(target_os = "macos")]
fn slicer_candidates(kind: SlicerKind) -> Vec<PathBuf> {
    let bundle = match kind {
        SlicerKind::Bambu => "BambuStudio.app",
        SlicerKind::Orca => "OrcaSlicer.app",
        SlicerKind::System => return Vec::new(),
    };
    let mut out = vec![PathBuf::from("/Applications").join(bundle)];
    if let Some(home) = std::env::var_os("HOME") {
        out.push(PathBuf::from(home).join("Applications").join(bundle));
    }
    out
}

#[cfg(target_os = "windows")]
fn slicer_candidates(kind: SlicerKind) -> Vec<PathBuf> {
    let (dir, exe) = match kind {
        SlicerKind::Bambu => ("Bambu Studio", "bambu-studio.exe"),
        SlicerKind::Orca => ("OrcaSlicer", "orca-slicer.exe"),
        SlicerKind::System => return Vec::new(),
    };
    let mut out = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(var) {
            out.push(PathBuf::from(root).join(dir).join(exe));
        }
    }
    /* 用户级安装位（不需要管理员权限的那种安装器默认落这） */
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join("Programs").join(dir).join(exe));
    }
    out
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn slicer_candidates(_kind: SlicerKind) -> Vec<PathBuf> {
    Vec::new()
}

/// 那个应用在这台机器上的确切位置（找不到 = `None`，**不猜**）
pub fn locate(kind: SlicerKind) -> Option<PathBuf> {
    slicer_candidates(kind).into_iter().find(|p| p.exists())
}

/// 把一份模型交给**指定**的切片软件打开。
///
/// `system` 那一档不走这里（它是"系统关联程序"的事，见 [`crate::ipc::mod::open_model`]）。
pub fn launch(kind: SlicerKind, file: &Path) -> Result<(), AppError> {
    let Some(target) = locate(kind) else {
        return Err(AppError::not_found(format!(
            "这台机器上没找到 {} —— 装好它，或者在设置里改回「跟随系统默认」",
            kind.label()
        )));
    };

    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("/usr/bin/open")
        .arg("-a")
        .arg(&target)
        .arg(file)
        .spawn();

    #[cfg(not(target_os = "macos"))]
    let spawned = std::process::Command::new(&target).arg(file).spawn();

    spawned.map(|_| ()).map_err(|e| {
        AppError::io(format!(
            "{} 没打开这个文件（应用在 {}）",
            kind.label(),
            target.display()
        ))
        .with_detail(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_ids_are_rejected_instead_of_falling_back() {
        /* 不传 / 空串 = 跟随系统（旧调用不用改） */
        assert_eq!(SlicerKind::parse(None).unwrap(), SlicerKind::System);
        assert_eq!(SlicerKind::parse(Some("")).unwrap(), SlicerKind::System);
        assert_eq!(
            SlicerKind::parse(Some(" system ")).unwrap(),
            SlicerKind::System
        );
        assert_eq!(SlicerKind::parse(Some("bambu")).unwrap(), SlicerKind::Bambu);
        assert_eq!(SlicerKind::parse(Some("orca")).unwrap(), SlicerKind::Orca);
        /* 认不出来就整次拒绝：不静默换成 system —— 那等于把用户点的当没听见 */
        for bad in ["prusa", "Bambu", "bamboo", "orcaslicer"] {
            assert!(SlicerKind::parse(Some(bad)).is_err(), "{bad} 该被拒");
        }
    }

    #[test]
    fn the_system_choice_has_no_place_of_its_own() {
        /* 它交给系统关联程序，没有"我们去找"这一步 */
        assert!(slicer_candidates(SlicerKind::System).is_empty());
        assert!(locate(SlicerKind::System).is_none());
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn the_two_slicers_each_look_at_a_path_named_after_itself() {
        for kind in [SlicerKind::Bambu, SlicerKind::Orca] {
            let candidates = slicer_candidates(kind);
            assert!(!candidates.is_empty(), "{:?} 至少要有一个落脚", kind);
            /* 末段必须是那个应用自己的名字 —— 不是随便一个目录 */
            let name = candidates[0]
                .file_name()
                .expect("候选一定带末段")
                .to_string_lossy()
                .to_lowercase();
            let want = match kind {
                SlicerKind::Bambu => "bambu",
                _ => "orca",
            };
            assert!(name.contains(want), "{name} 里没有 {want}");
        }
    }
}
