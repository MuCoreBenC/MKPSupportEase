//! **平台表** —— 一次发布涉及哪些平台、每个平台产出哪几份包、包叫什么名字。
//!
//! 表驱动是刻意的：加 Linux、换架构、多一种安装包格式，都只是往 [`PLATFORMS`] 里
//! 加一行，**不需要再设计一套发布流程**。状态机与封版判据都只读这张表。
//!
//! # 与 `installer_plan()` 的区别
//!
//! 旧内核的 `installer_plan()` 回答「**本机**能打什么」（macOS 只认 dmg）。
//! 那张表只够描述构建，描述不了「这一代还要等 Windows CI」。这里描述的是
//! **一次发布的全貌**，构建落在哪台机器上是另一回事（见 [`super::build`] / [`super::ci`]）。

/// 一份产物的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactKind {
    /// 写进清单 `assets[].kind` 的词（`dmg` / `appZip` / `nsis` / `portable`）
    pub kind: &'static str,
    /// 文件名后缀（含点）
    pub ext: &'static str,
    /// 名字里的架构段（`aarch64` / `x64`）
    pub arch: &'static str,
}

/// 一个平台要产出哪几份、由谁构建。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// 写进清单 `assets[].platform` 的词（与客户端 `host_platform()` 同一词表）
    pub id: &'static str,
    /// 给人看的名字
    pub label: &'static str,
    pub artifacts: &'static [ArtifactKind],
    /// 这份包由谁构建：`local`（发布机）/ `ci`（GitHub Actions）
    pub built_by: &'static str,
}

/// 产品名（asset 命名与 Release 标题用它）。
pub const PRODUCT: &str = "SupportEase";

/// tag 前缀。`v0.0.16` —— 与 `scripts/hooks/pre-push` 闸⑤ 逐字一致。
pub const TAG_PREFIX: &str = "v";

/// 暂存 tag 的命名空间。**刻意不是 `v*`**：GitHub 的 `tags-v-no-delete` ruleset 保护
/// `refs/tags/v*`，暂存 tag 一旦落成 `v*` 就再也删不掉（封版后要清理它）。
pub const STAGING_PREFIX: &str = "staging/";

/// 一次发布涉及的全部平台。**顺序就是执行顺序**：先本机（快、能立刻看到），再 CI。
pub const PLATFORMS: &[Platform] = &[
    Platform {
        id: "macos",
        label: "macOS ARM64",
        built_by: "local",
        artifacts: &[
            ArtifactKind {
                kind: "dmg",
                ext: ".dmg",
                arch: "aarch64",
            },
            ArtifactKind {
                kind: "appZip",
                ext: ".app.zip",
                arch: "aarch64",
            },
        ],
    },
    Platform {
        id: "windows",
        label: "Windows x64",
        built_by: "ci",
        artifacts: &[
            ArtifactKind {
                kind: "nsis",
                ext: ".exe",
                arch: "x64",
            },
            ArtifactKind {
                kind: "portable",
                ext: ".zip",
                arch: "x64",
            },
        ],
    },
];

/// 这一代的正式 tag（`v0.0.16`）。
pub fn tag_of(version: &str) -> String {
    format!("{TAG_PREFIX}{version}")
}

/// 这一代的暂存 tag（`staging/0.0.16`）。
pub fn staging_tag_of(version: &str) -> String {
    format!("{STAGING_PREFIX}{version}")
}

/// 一份产物的文件名（**唯一模板**：清单、上传、状态派生都读它）。
///
/// Windows 的安装版在旧清单里叫 `…_x64-setup.exe`（自研安装器的真实文件名），
/// 所以这里按「架构 + 种类后缀」拼：`nsis` → `-setup.exe`，`portable` → `-portable.zip`。
pub fn artifact_name(version: &str, kind: &ArtifactKind) -> String {
    match kind.kind {
        "nsis" => format!("{PRODUCT}_{version}_{}-setup{}", kind.arch, kind.ext),
        "portable" => format!("{PRODUCT}_{version}_{}-portable{}", kind.arch, kind.ext),
        _ => format!("{PRODUCT}_{version}_{}{}", kind.arch, kind.ext),
    }
}

/// 这一代**全部**计划产物（平台 × 种类），顺序与 [`PLATFORMS`] 一致。
pub fn planned_artifacts(version: &str) -> Vec<(&'static Platform, &'static ArtifactKind, String)> {
    let mut out = Vec::new();
    for p in PLATFORMS {
        for k in p.artifacts {
            out.push((p, k, artifact_name(version, k)));
        }
    }
    out
}

/// 某个平台是不是本机（发布机）构建的。
pub fn built_locally(platform: &Platform) -> bool {
    platform.built_by == "local"
}

/// 按文件名认出「这是哪一份计划产物」（CI 拉回来的包里挑安装包用它）。
pub fn classify(version: &str, name: &str) -> Option<(&'static Platform, &'static ArtifactKind)> {
    planned_artifacts(version)
        .into_iter()
        .find(|(_, _, n)| n == name)
        .map(|(p, k, _)| (p, k))
}

/// 一份**已落盘**产物的完整身份：名字、平台、种类、字节数、SHA-256、本地路径。
///
/// 身份在「读这一份文件」的那一刻定下，之后清单、上传、核对都只读这一份 ——
/// 中间文件被换掉的话，清单写的就是别的东西了。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactFact {
    pub name: String,
    pub platform: &'static str,
    pub kind: &'static str,
    pub size: u64,
    pub sha256: String,
    /// CI 拿回来的产物没有本地长期路径（解包在 `.scratch` 里），给 `None`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<std::path::PathBuf>,
}

impl ArtifactFact {
    pub fn from_path(
        version: &str,
        platform: &'static Platform,
        kind: &'static ArtifactKind,
        path: &std::path::Path,
    ) -> Result<Self, crate::error::AppError> {
        let name = artifact_name(version, kind);
        let meta = std::fs::metadata(path).map_err(|e| {
            crate::error::AppError::io(format!("读不到产物：{}", path.display()))
                .with_detail(e.to_string())
        })?;
        if !meta.is_file() {
            return Err(crate::error::AppError::io(format!(
                "产物不是文件：{}",
                path.display()
            )));
        }
        Ok(Self {
            name,
            platform: platform.id,
            kind: kind.kind,
            size: meta.len(),
            sha256: sha256_file(path)?,
            path: Some(path.to_path_buf()),
        })
    }

    /// CI 拉回来的那一份：文件在临时目录，字节数与自己算的 SHA 都要留。
    pub fn from_temp(
        version: &str,
        platform: &'static Platform,
        kind: &'static ArtifactKind,
        path: &std::path::Path,
    ) -> Result<Self, crate::error::AppError> {
        let mut fact = Self::from_path(version, platform, kind, path)?;
        fact.path = None;
        Ok(fact)
    }
}

/// 文件的 SHA-256（小写十六进制）。
pub fn sha256_file(path: &std::path::Path) -> Result<String, crate::error::AppError> {
    use sha2::{Digest as _, Sha256};
    use std::io::Read as _;

    let mut file = std::fs::File::open(path).map_err(|e| {
        crate::error::AppError::io(format!("读不了文件：{}", path.display()))
            .with_detail(e.to_string())
    })?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).map_err(|e| {
            crate::error::AppError::io(format!("读文件失败：{}", path.display()))
                .with_detail(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // 测试写临时文件是正当的（clippy.toml 的禁列只管生产代码）
fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mkp-se-{}", uuid_like()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[cfg(test)]
fn uuid_like() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

/// 本机的平台 id（`macos` / `windows` / 其它 `None`）。
pub fn host_platform_id() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("macos")
    } else if cfg!(target_os = "windows") {
        Some("windows")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_names_match_the_shipped_manifest() {
        let names: Vec<String> = planned_artifacts("0.0.16")
            .into_iter()
            .map(|(_, _, n)| n)
            .collect();
        assert_eq!(
            names,
            vec![
                "SupportEase_0.0.16_aarch64.dmg",
                "SupportEase_0.0.16_aarch64.app.zip",
                "SupportEase_0.0.16_x64-setup.exe",
                "SupportEase_0.0.16_x64-portable.zip",
            ]
        );
    }

    #[test]
    fn staging_tag_is_not_under_the_protected_namespace() {
        let staging = staging_tag_of("0.0.16");
        assert!(staging.starts_with("staging/"));
        assert!(!staging.starts_with('v'));
        assert_eq!(tag_of("0.0.16"), "v0.0.16");
    }

    #[test]
    fn every_platform_has_one_local_builder_or_ci() {
        for p in PLATFORMS {
            assert!(
                p.built_by == "local" || p.built_by == "ci",
                "{} 的 built_by 只有 local / ci 两种",
                p.id
            );
        }
    }

    #[test]
    fn sha256_and_size_are_read_from_the_real_file() {
        let path = write_temp("probe.bin", b"hello");
        assert_eq!(
            sha256_file(&path).unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let mac = PLATFORMS.iter().find(|p| p.id == "macos").unwrap();
        let fact = ArtifactFact::from_path("0.0.16", mac, &mac.artifacts[0], &path).unwrap();
        assert_eq!(fact.size, 5);
        assert_eq!(fact.name, "SupportEase_0.0.16_aarch64.dmg");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
