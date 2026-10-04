//! **发布账户配置** —— "发到哪个仓库、以谁的身份"，住 `<appDataDir>/publish-account.json`。
//!
//! # 配置与秘密分离（作者 2026-10-04 定死）
//!
//! ```text
//! publish-account.json   ← 发布账户配置：repositoryUrl + username（**没有 token**）
//! 系统 Keychain          ← Token：supportease.<platform>.token（见 [`super::credentials`]）
//! ```
//!
//! ★ **配置文件里绝不出现 token 字段** —— 判据 `publish_account_config_never_stores_a_token`
//! 钉住。Token 才是秘密，配置不是。
//!
//! # 为什么必须有这份配置（而不是靠 git remote 推断）
//!
//! 上一版靠 `git remote` 推平台。作者否掉了那条：**"工作台要明确知道发布到哪，而不是
//! 我猜当前 remote 是 GitHub 所以发 GitHub。"** 于是发布目标（平台 / 仓库地址）**由这份
//! 配置决定**；`git remote` 降级为**校验** —— 当前工作目录到底是不是配置的那个仓库
//! （[`super::git::Git::remote_matches`]），不是就如实拒绝，不发布。
//!
//! ★ 这条**取代**了上一轮"不为平台新增配置文件"的旧原则：`repositoryUrl` / `username`
//! 是**机器本地的发布配置**（不属于仓库内容、不该提交、也不含密钥），必须有个落点。
//!
//! # 存储规矩（沿用 [`crate::runtime::source`] 那一套，不另发明）
//!
//! 一种状态一个文件、住 `<appDataDir>` 根、带 `*Schema` 代次字段、写走 `atomic_write`、
//! **坏档报 `CORRUPTED` 不静默**（静默当"没配"会让用户以为配置丢了）。缺文件 = 空配置（合法）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 配置文件的固定名字（`<appDataDir>/publish-account.json`）。
pub const ACCOUNT_FILE: &str = "publish-account.json";

/// 配置格式的代次。加字段不升号、改语义才升（与 source / catalog 同一条）。
pub const ACCOUNT_SCHEMA: u32 = 1;

/// 一个平台的发布账户（**不含 token**）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAccount {
    /// 仓库地址（`https://github.com/o/r.git` 或 `git@github.com:o/r.git`）
    pub repository_url: String,
    /// 用户名（认证用，也是 PR/MR 的归属）
    pub username: String,
}

impl PlatformAccount {
    /// 两格都填了才算"配好了"（判据 / 界面共用这一处）。
    pub fn is_complete(&self) -> bool {
        !self.repository_url.trim().is_empty() && !self.username.trim().is_empty()
    }
}

/// 发布账户配置（`publish-account.json` 的根）。**每个平台一格，GitHub / Gitee 对称。**
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PublishAccountConfig {
    #[serde(default = "default_schema")]
    pub account_schema: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<PlatformAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gitee: Option<PlatformAccount>,
}

fn default_schema() -> u32 {
    ACCOUNT_SCHEMA
}

impl PublishAccountConfig {
    /// 取一个平台的账户（没有 = `None`）。
    pub fn get(&self, platform: &str) -> Option<&PlatformAccount> {
        match platform {
            "github" => self.github.as_ref(),
            "gitee" => self.gitee.as_ref(),
            _ => None,
        }
    }

    /// 写一个平台的账户。`None` = 清掉那一格。
    pub fn set(&mut self, platform: &str, account: Option<PlatformAccount>) {
        match platform {
            "github" => self.github = account,
            "gitee" => self.gitee = account,
            _ => {}
        }
    }
}

/// 配置文件路径（`<appDataDir>/publish-account.json`）。
pub fn config_path(root: &Path) -> PathBuf {
    root.join(ACCOUNT_FILE)
}

/// 读配置。缺文件 = 空配置（合法）；坏 JSON / 代次认不出 = `CORRUPTED`（**不静默当"没配"**）。
pub fn load(root: &Path) -> Result<PublishAccountConfig, AppError> {
    let path = config_path(root);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PublishAccountConfig {
                account_schema: ACCOUNT_SCHEMA,
                ..Default::default()
            })
        }
        Err(e) => {
            return Err(AppError::io("读不了发布账户配置").with_detail(e.to_string()));
        }
    };
    let cfg: PublishAccountConfig = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::corrupted("发布账户配置（publish-account.json）解析不了")
            .with_detail(e.to_string())
    })?;
    if cfg.account_schema != ACCOUNT_SCHEMA {
        return Err(AppError::corrupted(format!(
            "发布账户配置的格式代次认不了：文件是 {}，程序认 {}",
            cfg.account_schema, ACCOUNT_SCHEMA
        )));
    }
    Ok(cfg)
}

/// 写配置（`atomic_write`，与全仓写盘同一条纪律）。
pub fn save(root: &Path, cfg: &PublishAccountConfig) -> Result<(), AppError> {
    let path = config_path(root);
    let mut text = serde_json::to_string_pretty(cfg)
        .map_err(|e| AppError::internal("发布账户配置序列化不了").with_detail(e.to_string()))?;
    text.push('\n');
    atomic_write(&path, text.as_bytes())
}

/// 校验一个平台账户的**平台归属**：`github` 那格填的地址必须属于 GitHub，反之亦然。
///
/// ★ 这是**校验**用途，不是"推断平台" —— 平台由用户在设置里的哪一组填显式决定
/// （作者 2026-10-04：不要"我猜 remote 是 GitHub 所以发 GitHub"）。
/// 地址认不出平台时不硬拒（可能是自建 GitHub Enterprise）—— 交给 [`is_complete`] 与
/// 发布时的 remote 校验兜底；这里只在**明显填错**（GitHub 格里填 gitee 地址）时报错。
pub fn check_platform_matches(platform: &str, account: &PlatformAccount) -> Result<(), AppError> {
    let Some(detected) = super::platform::detect_platform(&account.repository_url) else {
        // 认不出（自建源等）—— 不硬拒，放行
        return Ok(());
    };
    if detected == platform {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "这一格的仓库地址看起来属于 {detected}，却填在了 {platform} 里 —— 请检查是不是填错了"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acct(url: &str, name: &str) -> PlatformAccount {
        PlatformAccount {
            repository_url: url.to_owned(),
            username: name.to_owned(),
        }
    }

    /// ★★ **配置文件里绝不出现 token 字段** —— 这是这一刀最硬的一条。
    #[test]
    fn publish_account_config_never_stores_a_token() {
        let cfg = PublishAccountConfig {
            account_schema: ACCOUNT_SCHEMA,
            github: Some(acct("https://github.com/o/r.git", "ben")),
            gitee: Some(acct("https://gitee.com/o/r.git", "ben")),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.to_lowercase().contains("token"),
            "配置里漏了 token：{json}"
        );
        assert!(!json.contains("password"));
        // 该有的字段都在
        assert!(json.contains("repositoryUrl"));
        assert!(json.contains("username"));
    }

    /// 存取往返；缺文件 = 空配置（合法）。
    #[test]
    fn config_roundtrips_and_a_missing_file_is_empty() {
        let d = tempfile::tempdir().unwrap();
        // 缺文件 = 空配置
        let empty = load(d.path()).unwrap();
        assert!(empty.github.is_none() && empty.gitee.is_none());

        let cfg = PublishAccountConfig {
            account_schema: ACCOUNT_SCHEMA,
            github: Some(acct("https://github.com/o/r.git", "ben")),
            gitee: None,
        };
        save(d.path(), &cfg).unwrap();
        let back = load(d.path()).unwrap();
        assert_eq!(back, cfg);
        assert_eq!(
            back.get("github").unwrap().username,
            "ben",
            "取得到填进去的那个账户"
        );
        assert!(back.get("gitee").is_none());
    }

    /// 坏 JSON 报 `CORRUPTED` —— **不静默当"没配"**（那会让用户以为配置丢了）。
    #[test]
    fn broken_json_is_corrupted_not_silently_empty() {
        let d = tempfile::tempdir().unwrap();
        atomic_write(&config_path(d.path()), b"{ not json").unwrap();
        let e = load(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 代次认不出同样是坏档（不猜新版长什么样）。
    #[test]
    fn a_future_schema_is_refused() {
        let d = tempfile::tempdir().unwrap();
        atomic_write(
            &config_path(d.path()),
            br#"{"accountSchema":99,"github":{"repositoryUrl":"x","username":"y"}}"#,
        )
        .unwrap();
        assert_eq!(
            load(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// 两格都填了才算"配好了"。
    #[test]
    fn an_account_is_complete_only_when_both_fields_are_filled() {
        assert!(acct("https://github.com/o/r.git", "ben").is_complete());
        assert!(!acct("", "ben").is_complete());
        assert!(!acct("https://github.com/o/r.git", "  ").is_complete());
    }

    /// ★ 平台归属：GitHub 格里填 gitee 地址 = 明显填错，报错；认不出的地址不硬拒。
    #[test]
    fn a_mismatched_platform_is_refused() {
        // 填对了
        assert!(check_platform_matches("github", &acct("https://github.com/o/r.git", "b")).is_ok());
        assert!(check_platform_matches("gitee", &acct("git@gitee.com:o/r.git", "b")).is_ok());
        // 填错了（明显）
        assert_eq!(
            check_platform_matches("github", &acct("https://gitee.com/o/r.git", "b"))
                .unwrap_err()
                .code,
            crate::error::ErrorCode::InvalidArgument
        );
        // 认不出（自建 GitHub Enterprise）—— 放行，不硬拒
        assert!(check_platform_matches(
            "github",
            &acct("https://git.corp.example.com/o/r.git", "b")
        )
        .is_ok());
    }
}
