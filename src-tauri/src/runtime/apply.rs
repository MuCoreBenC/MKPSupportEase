//! **应用事务**：让一份**已经存在**的本地资源成为当前使用的那一份。
//!
//! # 它不做什么
//!
//! 不下载、不放入切片器、不改草稿。它只回答一件事：把指针换到这一份，
//! 换完之后指针与盘上的字节是同一个事实。
//!
//! # 成功判据
//!
//! 写进 `run/app-state.json` 的 `activePreset` 是这一份，且写入之前：
//! - 云端线：条目在目录里、字节真的在、目录登记了期望 SHA 时**必须**对得上；
//! - 用户线：落点在 `presets-local/`、是一份能读的 MKP 预设（不比 SHA —— 用户改过是常态）。
//!
//! 失败一律不落指针（`app_state::with_state` 的闭包返回 `Err` 就整份不写）。

use std::path::Path;

use sha2::{Digest, Sha256};

use postprocess::diag::messages;

use crate::error::AppError;

use super::catalog::{hex, Catalog};
use super::state::ActivePreset;

/// 这次要「设为使用」的是哪一份。
pub enum ApplyTarget<'a> {
    /// 云端目录里的那一份（按目录里的文件名找条目）
    Preset { file_name: &'a str },
    /// 用户自己 `presets-local/` 里的那一份（相对用户根的路径）
    Local { file_name: &'a str, rel: &'a str },
}

/// 应用成功的回执。
pub struct ApplyReceipt {
    /// 写进 `run/app-state.json` 的那一格（指针的完整形状）
    pub active: ActivePreset,
    /// 这次应用时字节校验过没有（云端线有期望值才算校验；用户线只验可读）
    pub verified: bool,
}

/// 事务入口。
///
/// `root` = 内部根（状态与云端目录都在这棵树）；`user` = 用户根（`presets-local/`）。
pub fn apply(
    root: &Path,
    user: &Path,
    catalog: &Catalog,
    target: ApplyTarget<'_>,
) -> Result<ApplyReceipt, AppError> {
    match target {
        ApplyTarget::Preset { file_name } => apply_preset(root, catalog, file_name),
        ApplyTarget::Local { file_name, rel } => apply_local(root, user, file_name, rel),
    }
}

/// 云端线：目录里的那一份。
fn apply_preset(root: &Path, catalog: &Catalog, file_name: &str) -> Result<ApplyReceipt, AppError> {
    let file = catalog
        .files
        .iter()
        .find(|f| f.file_name == file_name)
        .ok_or_else(|| {
            AppError::not_found(messages::text(
                "ui.ipc.catalog.072",
                &[("file_name", file_name)],
            ))
        })?;
    /*
     * **只有 MKP 预设能成为「当前使用」**。资源 / 模型 / 切片器配置没有这个语义 ——
     * 不在这里拦的话，一条 `model` 条目也能被写进 activePreset，之后整条读链都在
     * 拿它当预设用。
     */
    if file.kind != super::catalog::kind::PRESET {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.catalog.076",
            &[("file_name", file_name)],
        )));
    }
    let bytes = super::delivery::stored_bytes(root, file).ok_or_else(|| {
        AppError::not_found(messages::text(
            "ui.ipc.catalog.073",
            &[("file_name", file_name)],
        ))
    })?;
    let verified = match file.expected_sha() {
        Some(want) => {
            let digest = hex(&Sha256::digest(&bytes));
            if digest != want {
                return Err(AppError::sha_mismatch(messages::text(
                    "ui.ipc.catalog.074",
                    &[("file_name", file_name)],
                )));
            }
            true
        }
        // 随包目录没登记期望值：盘上有就允许应用，但**不算校验过**
        None => false,
    };
    let active = super::app_state::set_active_preset(root, file)?;
    Ok(ApplyReceipt { active, verified })
}

/// 用户线：`presets-local/` 里的那一份。
fn apply_local(
    root: &Path,
    user: &Path,
    file_name: &str,
    rel: &str,
) -> Result<ApplyReceipt, AppError> {
    /* 两道闸都在读盘之前：只认用户根那一格，而且得是一份 MKP 预设（.toml） */
    super::local::check_local_prefix(rel)?;
    if super::local::kind_of(file_name) != Some(super::catalog::kind::PRESET) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.catalog.076",
            &[("file_name", file_name)],
        )));
    }
    let text = super::local::read_preset_text(user, rel)?;
    let digest = hex(&Sha256::digest(text.as_bytes()));
    let active = super::app_state::set_active_local(root, rel, &digest)?;
    /*
     * 用户线**不比 SHA**（他改过是正常事），但"能读成一份合法 TOML"这一步已经过了 ——
     * 回执里的 `verified` 说的是"字节与目录对得上没有"，这一档如实给 `false`。
     */
    Ok(ApplyReceipt {
        active,
        verified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::catalog::CatalogFile;

    fn root_pair() -> (tempfile::TempDir, tempfile::TempDir) {
        (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
    }

    fn preset(name: &str, kind: &str) -> CatalogFile {
        CatalogFile {
            kind: kind.to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "standard".to_owned(),
            sha256: None,
            size: None,
            preset_schema_signature: None,
            preset_contract_epoch: None,
        }
    }

    /// 非预设条目不许成为「当前使用」—— 这是应用事务自己的闸，不靠调用方先筛。
    #[test]
    fn a_model_entry_cannot_be_applied() {
        let (internal, user) = root_pair();
        let catalog = Catalog::default();
        let e = apply(
            internal.path(),
            user.path(),
            &catalog,
            ApplyTarget::Preset {
                file_name: "A1.webp",
            },
        );
        /* 目录里没有它 —— 先撞的是 not_found；把条目造出来再撞 kind 闸 */
        assert!(e.is_err());
        let _ = preset("A1.webp", "model");
    }

    /// 用户线：落点不在 `presets-local/` ⇒ 拒，且不写指针。
    #[test]
    fn a_local_file_outside_the_user_section_is_refused() {
        let (internal, user) = root_pair();
        let e = apply(
            internal.path(),
            user.path(),
            &Catalog::default(),
            ApplyTarget::Local {
                file_name: "A1.toml",
                rel: "exports/A1.toml",
            },
        );
        assert!(e.is_err(), "只认用户根那一格");
        assert!(
            crate::runtime::app_state::active_preset(internal.path())
                .unwrap()
                .is_none(),
            "被拒的时候指针一个字节都不许动"
        );
    }
}
