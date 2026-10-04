//! **发布账户凭据** —— 每平台一份 Token，住**系统 Keychain**。
//!
//! # 三条定死的口径（作者 2026-10-04）
//!
//! 1. **Token 由 SupportEase 自己管理**，不借用户的 `gh` / `git` 登录态；
//! 2. 存**系统 Keychain**（macOS Keychain / Windows Credential Manager / Linux Secret Service），
//!    **绝不**写 config.toml / localStorage / .env；
//! 3. **每平台一份** key（`supportease.github.token` / `supportease.gitee.token`）——
//!    一个 key 存多平台会导致切平台时反复覆盖。
//!
//! # 前端看不到 Token
//!
//! 前端只知道"**有 / 没有**凭据"（[`CredentialStatus`]），**拿不到原值**。
//! 判据 `credentials_never_echo_the_token` 钉住这条 —— 状态结构里没有 token 字段。
//!
//! # 可测性：trait + 内存替身
//!
//! 真 Keychain 是系统调用，测试里不能碰（CI 上没有 Keychain、会弹授权框）。
//! 所以逻辑对 [`SecretStore`] trait 编程：真机用 [`KeychainStore`]，
//! 判据用 [`MemoryStore`]。**业务逻辑（key 命名、每平台一份）与后端无关**，两头都验得到。

use crate::error::AppError;

/// 一个平台的凭据 key（**key 命名只此一处**）。
pub fn key_for(platform: &str) -> String {
    format!("supportease.{platform}.token")
}

/// 支持的平台 id 白名单。**认不出的平台不写 Keychain**（避免被塞进任意 key）。
pub const PLATFORMS: [&str; 2] = ["github", "gitee"];

/// 校验平台 id。
pub fn ensure_known(platform: &str) -> Result<(), AppError> {
    if PLATFORMS.contains(&platform) {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "不认识的发布平台：{platform}（只支持 github / gitee）"
        )))
    }
}

/// 给前端看的凭据状态 —— ★ **只有"有没有"，没有原值**。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    /// 平台 id
    pub platform: String,
    /// 有没有存过凭据
    pub configured: bool,
    /// 凭据的"尾号"提示（如 `...a1b2`）—— 让人确认"是不是这一把"，**不是原值**
    pub hint: Option<String>,
}

/// 秘密存储后端。真机 = 系统 Keychain；判据 = 内存。
pub trait SecretStore: Send + Sync {
    /// 取一个 key 的值
    fn get(&self, key: &str) -> Option<String>;
    /// 写一个 key 的值
    fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
    /// 删一个 key（不存在也算成功）
    fn delete(&self, key: &str) -> Result<(), AppError>;
}

/// 系统 Keychain 后端。
///
/// ★ **仅在真机上用**；判据一律走 [`MemoryStore`]（CI 上没有 Keychain）。
pub struct KeychainStore;

impl SecretStore for KeychainStore {
    fn get(&self, key: &str) -> Option<String> {
        // keyring 的 Entry::new(service, user)。这里 service 固定，user = key。
        let entry = keyring::Entry::new("SupportEase", key).ok()?;
        entry.get_password().ok()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new("SupportEase", key)
            .map_err(|e| AppError::io("打不开系统钥匙串").with_detail(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| AppError::io("写不进系统钥匙串").with_detail(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new("SupportEase", key)
            .map_err(|e| AppError::io("打不开系统钥匙串").with_detail(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            // 本来就没有 = 删除成功（幂等）
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::io("清不掉系统钥匙串里的凭据").with_detail(e.to_string())),
        }
    }
}

/// 内存后端（判据 / 演示用）。**绝不落盘**。
#[derive(Default)]
pub struct MemoryStore {
    inner: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().ok()?.get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.inner
            .lock()
            .map_err(|_| AppError::internal("内存凭据表锁中毒了"))?
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        self.inner
            .lock()
            .map_err(|_| AppError::internal("内存凭据表锁中毒了"))?
            .remove(key);
        Ok(())
    }
}

/// 凭据管理器：把 [`SecretStore`] 包上"平台 → key"的语义。
pub struct Credentials<S: SecretStore> {
    store: S,
}

impl<S: SecretStore> Credentials<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// 取一个平台的 Token（**内部用**：构造平台客户端）。没有 = `None`。
    pub fn token(&self, platform: &str) -> Result<Option<String>, AppError> {
        ensure_known(platform)?;
        Ok(self.store.get(&key_for(platform)))
    }

    /// 存一个 Token。空串拒绝（"存了个空的"和"没存"分不清是灾难）。
    pub fn set_token(&self, platform: &str, token: &str) -> Result<(), AppError> {
        ensure_known(platform)?;
        let t = token.trim();
        if t.is_empty() {
            return Err(AppError::invalid_argument("Token 不能是空的"));
        }
        self.store.set(&key_for(platform), t)
    }

    /// 清一个平台的凭据（幂等）。
    pub fn clear(&self, platform: &str) -> Result<(), AppError> {
        ensure_known(platform)?;
        self.store.delete(&key_for(platform))
    }

    /// 给前端的状态：**只有"有没有"，加上一个尾号提示，没有原值**。
    pub fn status(&self, platform: &str) -> Result<CredentialStatus, AppError> {
        ensure_known(platform)?;
        let token = self.store.get(&key_for(platform));
        Ok(CredentialStatus {
            platform: platform.to_owned(),
            configured: token.is_some(),
            hint: token.as_deref().map(tail_hint),
        })
    }
}

/// 尾号提示：`...a1b2`。**只给末尾 4 位** —— 够确认"是不是这一把"，不够还原。
fn tail_hint(token: &str) -> String {
    let n = token.chars().count();
    if n <= 4 {
        // 太短的（理论上不该有）不提示任何内容，免得把整把露出去
        return "…".to_owned();
    }
    let tail: String = token.chars().skip(n - 4).collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds() -> Credentials<MemoryStore> {
        Credentials::new(MemoryStore::new())
    }

    /// 每平台一份 key：两个平台的凭据**互不覆盖**。
    #[test]
    fn each_platform_has_its_own_credential() {
        let c = creds();
        c.set_token("github", "gh-token-aaaa").unwrap();
        c.set_token("gitee", "gitee-token-bbbb").unwrap();
        assert_eq!(c.token("github").unwrap().as_deref(), Some("gh-token-aaaa"));
        assert_eq!(
            c.token("gitee").unwrap().as_deref(),
            Some("gitee-token-bbbb")
        );
        // key 命名也钉住
        assert_eq!(key_for("github"), "supportease.github.token");
        assert_eq!(key_for("gitee"), "supportease.gitee.token");
    }

    /// ★ **凭据状态绝不回显原值** —— 只有 configured 与尾号 hint。
    #[test]
    fn credentials_never_echo_the_token() {
        let c = creds();
        c.set_token("github", "ghp_secret_value_1234").unwrap();
        let status = c.status("github").unwrap();
        assert!(status.configured);
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("ghp_secret_value"), "状态里漏了原值：{json}");
        assert!(!json.contains("secret"), "状态里漏了原值：{json}");
        // 尾号提示可以出现（末尾 4 位）
        assert_eq!(status.hint.as_deref(), Some("…1234"));
    }

    /// 没存过 = configured false，hint 为 None。
    #[test]
    fn an_unconfigured_platform_reports_false() {
        let c = creds();
        let s = c.status("gitee").unwrap();
        assert!(!s.configured);
        assert!(s.hint.is_none());
    }

    /// 清凭据是幂等的；清完 configured 变 false。
    #[test]
    fn clearing_is_idempotent() {
        let c = creds();
        c.set_token("github", "x-token-9999").unwrap();
        c.clear("github").unwrap();
        assert!(!c.status("github").unwrap().configured);
        c.clear("github").unwrap(); // 再清一次不报错
    }

    /// 空 Token 被拒（"存了个空的"和"没存"分不清是灾难）。
    #[test]
    fn an_empty_token_is_refused() {
        let c = creds();
        assert_eq!(
            c.set_token("github", "   ").unwrap_err().code,
            crate::error::ErrorCode::InvalidArgument
        );
    }

    /// 不认识的平台被拒 —— 不许往任意 key 里写。
    #[test]
    fn an_unknown_platform_is_refused() {
        let c = creds();
        assert!(c.status("gitlab").is_err());
        assert!(c.set_token("gitlab", "x").is_err());
    }
}
