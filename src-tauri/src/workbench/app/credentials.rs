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
//!
//! # 会话缓存：一次程序运行 = 至多一次读钥匙串（作者 2026-10-04）
//!
//! 真机上每次读 Keychain 都可能弹系统授权框（dev 构建尤其 —— 二进制签名一变，系统就把
//! 我们当陌生 App）；而发布链一次事务要读好几回（发布 / 合并 / 状态回读）。所以真机入口是
//! [`session`]：**进程一份**的 [`Credentials`]，后端用 [`CachedStore`] 包一层内存缓存。
//! **不改** Keychain 的存储方式与内容；`set` / `clear` 同步更新缓存；退出即失效。

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

/// 会话缓存装饰器：**进程内第一次读到什么就记住什么**，之后不再打扰系统钥匙串。
///
/// 为什么要它：Keychain 每次读都可能弹系统授权框（dev 构建尤其明显 —— 二进制签名一变，
/// 系统就把我们当陌生 App 重新问）；发布链一次事务要读好几回（发布 / 合并 / 状态回读）。
///
/// 边界（写清楚，别让它长歪）：
/// - **只活在内存里**，退出即失效；不落盘、不改 Keychain 的存储方式与内容；
/// - `set` / `delete` **同步更新缓存**（写完立刻读得到新值，且不再读后端）；
/// - 真机入口是 [`session`]（进程一份）；判据用计数替身证明"第二次不再读后端"。
pub struct CachedStore<S: SecretStore> {
    inner: S,
    /// key → 读到过的值（`None` = 读到过"没有"）。**"读不到"也是一种答案**，一样记住。
    seen: std::sync::Mutex<std::collections::BTreeMap<String, Option<String>>>,
}

impl<S: SecretStore> CachedStore<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            seen: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    fn seen_guard(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::BTreeMap<String, Option<String>>> {
        // 锁中毒不致命：这张表只是缓存 —— 拿回里面的数据继续用，
        // 别因为一次 panic 把"读凭据"整条路堵死
        self.seen.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl<S: SecretStore> SecretStore for CachedStore<S> {
    fn get(&self, key: &str) -> Option<String> {
        // ★ 锁**跨后端调用**是刻意的：同一时刻只放一个询问出去（不然并发读会各弹各的框）。
        let mut seen = self.seen_guard();
        if let Some(hit) = seen.get(key) {
            return hit.clone();
        }
        let value = self.inner.get(key);
        seen.insert(key.to_owned(), value.clone());
        value
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.inner.set(key, value)?;
        self.seen_guard()
            .insert(key.to_owned(), Some(value.to_owned()));
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        self.inner.delete(key)?;
        self.seen_guard().insert(key.to_owned(), None);
        Ok(())
    }
}

/// 真机入口的类型：带会话缓存的凭据管理器。**判据与真机同一套逻辑**，只是后端换内存替身。
pub type Session = Credentials<CachedStore<KeychainStore>>;

/// 真机入口：**进程一份**的凭据管理器（带会话缓存，见 [`CachedStore`]）。
///
/// ★ 必须是"一份"：缓存要跨调用点共享才有意义 —— 每个命令各自 `Credentials::new` 的话，
/// 缓存跟着每次调用重生，等于没有。
pub fn session() -> &'static Session {
    static ONE: std::sync::OnceLock<Session> = std::sync::OnceLock::new();
    ONE.get_or_init(|| Credentials::new(CachedStore::new(KeychainStore)))
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

    /// 计数替身：证明"第二次不再读后端"（真 Keychain 在判据里碰不得、也数不清）。
    #[derive(Clone, Default)]
    struct CountingStore {
        inner: std::sync::Arc<MemoryStore>,
        reads: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl CountingStore {
        /// 播一把种（模拟"上一轮运行里配好的 Token"），**播种不算读取**。
        fn seeded(platform: &str, token: &str) -> Self {
            let s = Self::default();
            s.inner.set(&key_for(platform), token).unwrap();
            s
        }

        fn reads(&self) -> usize {
            self.reads.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl SecretStore for CountingStore {
        fn get(&self, key: &str) -> Option<String> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.inner.get(key)
        }

        fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
            self.inner.set(key, value)
        }

        fn delete(&self, key: &str) -> Result<(), AppError> {
            self.inner.delete(key)
        }
    }

    /// ★ **一次程序运行 = 至多一次读钥匙串**：第一次读之后走内存缓存；set / clear 同步失效。
    ///
    /// 为什么值得一条判据：每一次读都可能弹系统授权框（作者真机抱怨"还是要我输两次密码"）。
    /// 这条钉住的是**发行体验**，不是性能。
    #[test]
    fn the_session_reads_the_store_at_most_once_per_process() {
        let store = CountingStore::seeded("github", "ghp_old_1234");
        let c = Credentials::new(CachedStore::new(store.clone()));

        assert_eq!(c.token("github").unwrap().as_deref(), Some("ghp_old_1234"));
        assert_eq!(store.reads(), 1, "第一次必须真读一次后端");

        // 之后：push / PR / merge / 状态回读都落在缓存上 —— 一次都不再读
        assert_eq!(c.token("github").unwrap().as_deref(), Some("ghp_old_1234"));
        let st = c.status("github").unwrap();
        assert!(st.configured);
        assert_eq!(st.hint.as_deref(), Some("…1234"));
        assert_eq!(store.reads(), 1, "第二次起必须命中缓存");

        // 存新值：缓存同步更新（读了也不该再打扰后端）
        c.set_token("github", "ghp_new_5678").unwrap();
        assert_eq!(c.token("github").unwrap().as_deref(), Some("ghp_new_5678"));
        assert_eq!(store.reads(), 1, "set 之后不该再读后端");

        // 清掉：缓存同步失效
        c.clear("github").unwrap();
        assert_eq!(c.token("github").unwrap(), None);
        assert_eq!(store.reads(), 1, "clear 之后不该再读后端");

        // 后端本身确实跟着动了：换一个全新会话（空缓存）看真身
        let fresh = Credentials::new(CachedStore::new(store.clone()));
        assert_eq!(fresh.token("github").unwrap(), None);
        assert_eq!(store.reads(), 2, "后端本身确实被清掉了");
    }

    /// 真机入口是**进程一份** —— 缓存跨调用点共享的前提（各自 new 一个等于没缓存）。
    #[test]
    fn the_real_session_is_a_singleton() {
        assert!(std::ptr::eq(session(), session()));
    }
}
