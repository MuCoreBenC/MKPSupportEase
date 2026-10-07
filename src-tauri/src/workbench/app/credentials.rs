//! **发布账户凭据** —— 每平台一份 Token，住**本机凭据文件**（0600）。
//!
//! # 三条定死的口径（作者 2026-10-04；第 2 条 2026-10-05 改判）
//!
//! 1. **Token 由 SupportEase 自己管理**，不借用户的 `gh` / `git` 登录态；
//! 2. 存**本机专属凭据文件** `<appDataDir>/credentials.json`（权限 0600、原子写），
//!    **绝不**写 localStorage / .env / 任何会进仓库或被网盘同步的目录。
//!    **为什么从系统 Keychain 改判**（作者 2026-10-05）：Keychain 的免弹窗授权完全
//!    建立在**稳定的代码签名**上 —— 这个应用不分发签名（Gatekeeper 警告是拍过板的
//!    代价），于是 dev 每次重编译、用户每升一个版本，签名一变就被当陌生 App，
//!    反复弹"输入登录钥匙串密码"。无签名软件存秘密的业界惯例就是 0600 文件
//!    （npm / gh / AWS CLI 都这么干）；威胁面与 Keychain 实际等价 —— 同一用户的
//!    任何进程两边都读得走，Windows Credential Manager 也本就没有按 App 的 ACL。
//! 3. **每平台一份** key（`supportease.github.token` / `supportease.gitee.token`）——
//!    一个 key 存多平台会导致切平台时反复覆盖。
//!
//! # 前端拿不到 Token —— 除非人**显式点那颗「眼睛」**
//!
//! 状态面（[`CredentialStatus`] / `publish_tx::PlatformAccountView`）只有"**有 / 没有**"
//! 加一个尾号提示，**没有原值**；判据 `credentials_never_echo_the_token` 钉住这条。
//!
//! ★ 2026-10-07 作者加了一条**显式**出口：设置页那颗「眼睛」调
//! `publish_tx::wb_get_publish_token` 取明文（换机器 / 重配时要看得见、要能复制）。
//! 它与状态面分家 —— 状态结构里照旧一个 token 字段都没有；这条出口**只在人点眼睛
//! 那一刻**才走（不是开场自动读，也不进发布链）。
//!
//! # 可测性：trait + 内存替身，**且真后端自己也可测**
//!
//! 业务逻辑（key 命名、每平台一份）对 [`SecretStore`] trait 编程：判据用
//! [`MemoryStore`]，保证"逻辑与后端无关"。真后端以前是系统 Keychain（CI 碰不得、
//! 会弹授权框）；换成文件后端之后 [`FileStore`] 在临时目录里真写真读，行为
//! （0600、坏档当空、清空撤档）有自己的判据钉着。
//!
//! # 会话缓存：一次程序运行 = 至多一次读盘（作者 2026-10-04）
//!
//! 发布链一次事务要读好几回（发布 / 合并 / 状态回读），所以真机入口是 [`session`]：
//! **进程一份**的 [`Credentials`]，用 [`CachedStore`] 包一层内存缓存。
//! **不改**文件的存储方式与内容；`set` / `clear` 同步更新缓存；退出即失效。
//! （Keychain 时代这层还兼任"少弹授权框"；文件后端不弹框，剩下的理由是读盘
//! 次数与进程内一致性。）

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 一个平台的凭据 key（**key 命名只此一处**）。
pub fn key_for(platform: &str) -> String {
    format!("supportease.{platform}.token")
}

/// 支持的平台 id 白名单。**认不出的平台不写凭据文件**（避免被塞进任意 key）。
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

/// 秘密存储后端。真机 = 本机凭据文件；判据 = 内存。
pub trait SecretStore: Send + Sync {
    /// 取一个 key 的值
    fn get(&self, key: &str) -> Option<String>;
    /// 写一个 key 的值
    fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
    /// 删一个 key（不存在也算成功）
    fn delete(&self, key: &str) -> Result<(), AppError>;
}

/* ---------- 真后端：本机凭据文件 ---------- */

/// 凭据文件的固定名字（`<appDataDir>/credentials.json`）。
pub const CREDENTIALS_FILE: &str = "credentials.json";

/// 凭据文件格式的代次。加字段不升号、改语义才升（与 publish-account 同一条）。
pub const CREDENTIALS_SCHEMA: u32 = 1;

/// 应用标识 —— 与 `src-tauri/tauri.workbench.conf.json` 的 `identifier` 是同一份，判据
/// `the_credentials_path_is_pinned_to_the_tauri_identifier` 钉住两处不漂移。
///
/// ★ 是**工作台那一份**，不是客户端的：工作台与客户端是两个 identifier
/// （`tauri.workbench.conf.json` 覆盖主配置那一份），于是各有各的 `appDataDir`、
/// 各有各的 WebView2 数据目录 —— 两边才能同时开。凭据跟着工作台走：
/// 它只被工作台（GUI 与 `release` CLI）用到。
///
/// ★★ 为什么不是 `com.*` 那套反域名：identifier 在 Windows 上直接就是
/// `AppData\Roaming` 下的**文件夹名**，`com.` 前缀是 macOS / Java 的反域名传统
/// （bundle id 用它），Windows 的桌面软件一律叫产品名（`Code` / `BambuStudio` /
/// `obsidian`）。这台机器上是给人翻文件夹的，所以两边都用产品名说话。
const APP_ID: &str = "SupportEase-Workbench";

/// 凭据文件的形状（`credentials.json` 的根）。**只放秘密** —— 配置在
/// `publish-account.json`（判据 `publish_account_config_never_stores_a_token` 反向钉住）。
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialsFile {
    credentials_schema: u32,
    secrets: BTreeMap<String, String>,
}

/// 本机凭据文件后端（**全平台一个后端**，不再按 OS 分叉）。
///
/// 文件 `<appDataDir>/credentials.json`，走 [`crate::fsx::atomic::atomic_write`]
/// （同目录临时文件 + rename，断电不剩半截）；unix 上写完把权限钉在 0600。
///
/// 读规矩与 [`super::account`] **刻意不同**：坏档**不报 `CORRUPTED`，当"没存"**。
/// 理由：秘密读不出来 = 没有秘密，对用户与"没配"是同一件事（重贴一次 Token 即愈）；
/// 若坏档把 `status` 炸红，设置页会整页不可用，代价远大于收益。`tracing::warn!` 留痕。
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    /// 指定落点（测试用临时目录；真机走 [`default_path`]）。
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// 读整个秘密表。缺文件 = 空表；**读不了 / 解析不了也当空表**（见结构体文档）。
    fn read_map(&self) -> BTreeMap<String, String> {
        let bytes = match std::fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return BTreeMap::new(),
            Err(e) => {
                tracing::warn!(path = %self.path.display(), "读不了凭据文件，当作没存：{e}");
                return BTreeMap::new();
            }
        };
        match serde_json::from_slice::<CredentialsFile>(&bytes) {
            Ok(f) if f.credentials_schema == CREDENTIALS_SCHEMA => f.secrets,
            Ok(f) => {
                tracing::warn!(
                    path = %self.path.display(),
                    got = f.credentials_schema,
                    want = CREDENTIALS_SCHEMA,
                    "凭据文件的格式代次认不出，当作没存"
                );
                BTreeMap::new()
            }
            Err(e) => {
                tracing::warn!(path = %self.path.display(), "凭据文件解析不了，当作没存：{e}");
                BTreeMap::new()
            }
        }
    }

    /// 写整个秘密表。**表空 = 把文件撤掉** —— 盘上不留一个"看起来有秘密"的空壳。
    fn write_map(&self, secrets: BTreeMap<String, String>) -> Result<(), AppError> {
        if secrets.is_empty() {
            return match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                // 本来就没有 = 撤成功（幂等）
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(AppError::io("清不掉凭据文件").with_detail(e.to_string())),
            };
        }
        let file = CredentialsFile {
            credentials_schema: CREDENTIALS_SCHEMA,
            secrets,
        };
        let mut bytes = serde_json::to_vec_pretty(&file)
            .map_err(|e| AppError::internal("凭据文件序列化不了").with_detail(e.to_string()))?;
        bytes.push(b'\n');
        atomic_write(&self.path, &bytes)?;
        self.enforce_owner_only();
        Ok(())
    }

    /// unix 上把文件权限钉在 0600：atomic_write 的临时文件默认就是这个档，
    /// 这里再钉一次当保险 —— 旧文件权限松了也会被收回来。
    /// Windows 走用户目录的继承 ACL（profile 目录本就只对本人开放），不用动。
    #[cfg(unix)]
    fn enforce_owner_only(&self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
    }

    #[cfg(not(unix))]
    fn enforce_owner_only(&self) {}
}

/// 凭据文件的默认落点。**GUI 与 `release` CLI 必须指到同一份**：
///
/// - 默认 = OS 应用数据目录 + [`APP_ID`]，与 Tauri 的 `app_data_dir()`（GUI 的
///   `internal_root`）同址 —— 按 Tauri 的公式手算一份，因为 [`session`] 是进程一份的
///   静态量，拿不到 `AppHandle`；
/// - `MKPSE_APP_DIR` 是 `release` CLI 的对齐开关（与发布账户配置用同一个），设了就
///   以它为准。★ 它只该在跑 CLI 的壳里设 —— GUI 不认这个开关（走 `AppHandle`）。
pub fn default_path() -> PathBuf {
    match std::env::var_os("MKPSE_APP_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join(CREDENTIALS_FILE),
        _ => os_app_data_dir().join(CREDENTIALS_FILE),
    }
}

/// OS 应用数据目录（与 Tauri `app_data_dir()` 同一公式，见 `dirs` crate 的取法）。
fn os_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support").join(APP_ID)
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home_dir)
            .join(APP_ID)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local/share"))
            .join(APP_ID)
    }
}

/// 家目录。`HOME` / `USERPROFILE` 都没有的极端环境退到当前目录 —— 宁可写错地方也不 panic。
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

impl SecretStore for FileStore {
    fn get(&self, key: &str) -> Option<String> {
        self.read_map().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut secrets = self.read_map();
        secrets.insert(key.to_owned(), value.to_owned());
        self.write_map(secrets)
    }

    fn delete(&self, key: &str) -> Result<(), AppError> {
        let mut secrets = self.read_map();
        // 本来就没有 → 不动文件（幂等，也不留时间戳变化）
        if secrets.remove(key).is_none() {
            return Ok(());
        }
        self.write_map(secrets)
    }
}

/// 内存后端（判据 / 演示用）。**绝不落盘**。
#[derive(Default)]
pub struct MemoryStore {
    inner: std::sync::Mutex<BTreeMap<String, String>>,
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

/// 会话缓存装饰器：**进程内第一次读到什么就记住什么**，之后不再读后端。
///
/// 为什么要有它：发布链一次事务要读好几回（发布 / 合并 / 状态回读），进程一份的
/// 缓存把它们都并到第一次读上（Keychain 时代它还兼任"少弹授权框"）。
///
/// 边界（写清楚，别让它长歪）：
/// - **只活在内存里**，退出即失效；不落盘、不改后端的存储方式与内容；
/// - `set` / `delete` **同步更新缓存**（写完立刻读得到新值，且不再读后端）；
/// - 真机入口是 [`session`]（进程一份）；判据用计数替身证明"第二次不再读后端"。
pub struct CachedStore<S: SecretStore> {
    inner: S,
    /// key → 读到过的值（`None` = 读到过"没有"）。**"读不到"也是一种答案**，一样记住。
    seen: std::sync::Mutex<BTreeMap<String, Option<String>>>,
}

impl<S: SecretStore> CachedStore<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            seen: std::sync::Mutex::new(BTreeMap::new()),
        }
    }

    fn seen_guard(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Option<String>>> {
        // 锁中毒不致命：这张表只是缓存 —— 拿回里面的数据继续用，
        // 别因为一次 panic 把"读凭据"整条路堵死
        self.seen.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl<S: SecretStore> SecretStore for CachedStore<S> {
    fn get(&self, key: &str) -> Option<String> {
        // ★ 锁**跨后端调用**是刻意的：同一时刻只放一个询问出去（后端共享同一份文件，
        //   并发读虽无弹框之忧，但一致性快照的语义照旧要保）。
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
pub type Session = Credentials<CachedStore<FileStore>>;

/// 真机入口：**进程一份**的凭据管理器（带会话缓存，见 [`CachedStore`]）。
///
/// ★ 必须是"一份"：缓存要跨调用点共享才有意义 —— 每个命令各自 `Credentials::new` 的话，
/// 缓存跟着每次调用重生，等于没有。
pub fn session() -> &'static Session {
    static ONE: std::sync::OnceLock<Session> = std::sync::OnceLock::new();
    ONE.get_or_init(|| Credentials::new(CachedStore::new(FileStore::new(default_path()))))
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

    /* ---------- FileStore：真后端自己的判据（临时目录真写真读） ---------- */

    /// 每个测试独占一个临时目录，互不串台。
    fn file_store() -> (tempfile::TempDir, FileStore) {
        let d = tempfile::tempdir().unwrap();
        let store = FileStore::new(d.path().join(CREDENTIALS_FILE));
        (d, store)
    }

    /// 存取往返；两个平台互不覆盖；删除幂等。
    #[test]
    fn the_file_store_roundtrips_and_delete_is_idempotent() {
        let (_d, s) = file_store();
        s.set(&key_for("github"), "gh-token-aaaa").unwrap();
        s.set(&key_for("gitee"), "gitee-token-bbbb").unwrap();
        assert_eq!(s.get(&key_for("github")).as_deref(), Some("gh-token-aaaa"));
        assert_eq!(
            s.get(&key_for("gitee")).as_deref(),
            Some("gitee-token-bbbb")
        );
        // 新进程（新实例）读同一份文件 —— 同一个答案
        let fresh = FileStore::new(_d.path().join(CREDENTIALS_FILE));
        assert_eq!(
            fresh.get(&key_for("github")).as_deref(),
            Some("gh-token-aaaa")
        );

        s.delete(&key_for("github")).unwrap();
        s.delete(&key_for("github")).unwrap(); // 再删一次不报错
        assert_eq!(s.get(&key_for("github")), None);
        assert_eq!(
            s.get(&key_for("gitee")).as_deref(),
            Some("gitee-token-bbbb"),
            "清 GitHub 不许动 Gitee"
        );
    }

    /// 读不落盘：从没写过的时候，get 返回"没有"，**也不许凭空建文件**。
    #[test]
    fn a_missing_file_reads_as_empty_without_creating_it() {
        let (d, s) = file_store();
        assert_eq!(s.get(&key_for("github")), None);
        assert!(
            !d.path().join(CREDENTIALS_FILE).exists(),
            "只是读了一下，不该出现凭据文件"
        );
    }

    /// ★ 坏档**当"没存"**，不把状态读炸（理由见 FileStore 的结构体文档）；
    ///    下一次 set 把文件写回来，等于自愈。
    #[test]
    fn a_broken_file_reads_as_empty_and_the_next_set_heals_it() {
        let (d, s) = file_store();
        crate::fsx::atomic::atomic_write(&d.path().join(CREDENTIALS_FILE), b"{ not json").unwrap();
        assert_eq!(s.get(&key_for("github")), None, "坏档当空，不许 Err");

        s.set(&key_for("github"), "ghp_fresh").unwrap();
        assert_eq!(s.get(&key_for("github")).as_deref(), Some("ghp_fresh"));
    }

    /// 代次认不出与坏档同一待遇（当"没存"），但 set 会用**程序认识的代次**重写。
    #[test]
    fn a_future_schema_reads_as_empty() {
        let (d, s) = file_store();
        crate::fsx::atomic::atomic_write(
            &d.path().join(CREDENTIALS_FILE),
            br#"{"credentialsSchema":99,"secrets":{}}"#,
        )
        .unwrap();
        assert_eq!(s.get(&key_for("github")), None);
        s.set(&key_for("github"), "ghp_fresh").unwrap();
        let back = s.get(&key_for("github"));
        assert_eq!(back.as_deref(), Some("ghp_fresh"));
    }

    /// ★ **秘密文件只许本人可读**：unix 上写完必须是 0600（含改写旧文件的情形）。
    #[cfg(unix)]
    #[test]
    fn the_secret_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (d, s) = file_store();
        s.set(&key_for("github"), "ghp_secret").unwrap();
        let mode = std::fs::metadata(d.path().join(CREDENTIALS_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "凭据文件的权限必须是 0600，实际是 {:o}",
            mode & 0o777
        );

        // 预先放一个权限松的文件，set 收紧它
        let loose = d.path().join(CREDENTIALS_FILE);
        crate::fsx::atomic::atomic_write(&loose, br#"{"credentialsSchema":1,"secrets":{}}"#)
            .unwrap();
        std::fs::set_permissions(&loose, std::fs::Permissions::from_mode(0o666)).unwrap();
        s.set(&key_for("github"), "ghp_secret").unwrap();
        let mode = std::fs::metadata(&loose).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "松掉的权限必须在写入时收回");
    }

    /// ★ 清空 = 文件从盘上消失：不留一个"看起来有秘密"的空壳。
    #[test]
    fn emptying_the_store_removes_the_file() {
        let (d, s) = file_store();
        s.set(&key_for("github"), "ghp_only_one").unwrap();
        assert!(d.path().join(CREDENTIALS_FILE).exists());
        s.delete(&key_for("github")).unwrap();
        assert!(
            !d.path().join(CREDENTIALS_FILE).exists(),
            "秘密清空后文件必须撤掉"
        );
    }

    /// 默认落点：文件名固定是 credentials.json（目录由 APP_ID / 环境开关决定）。
    #[test]
    fn the_default_path_is_a_fixed_file_name() {
        assert_eq!(
            default_path().file_name().unwrap(),
            CREDENTIALS_FILE,
            "凭据文件的名字只此一处"
        );
    }

    /// ★ 凭据目录的 identifier 与**工作台那份** Tauri 配置钉在同一处：改了
    /// `tauri.workbench.conf.json` 的 identifier 而不改 [`APP_ID`]，GUI（AppHandle
    /// 解析的 appDataDir）与文件落点就会各找各的 —— 源码比对当场红。
    ///
    /// 读的是工作台配置而不是主配置：本文件整份活在 `workbench` feature 里，
    /// 它认的 identifier 就是工作台那一份。
    #[test]
    fn the_credentials_path_is_pinned_to_the_tauri_identifier() {
        let conf = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tauri.workbench.conf.json"
        ))
        .expect("读得到 tauri.workbench.conf.json");
        let v: serde_json::Value = serde_json::from_str(&conf).expect("工作台配置是 JSON");
        let identifier = v["identifier"].as_str().expect("identifier 是字符串");

        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/workbench/app/credentials.rs"
        ))
        .expect("读得到本文件");
        assert!(
            src.contains(identifier),
            "凭据落点的 APP_ID（{identifier}）没跟着 tauri.workbench.conf.json 走 —— 两处必须一致"
        );
    }

    /* ---------- 会话缓存 ---------- */

    /// 计数替身：证明"第二次不再读后端"（逻辑判据与后端无关，内存替身就够）。
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

    /// ★ **一次程序运行 = 至多一次读后端**：第一次读之后走内存缓存；set / clear 同步失效。
    ///
    /// 为什么值得一条判据：发布链一次事务要读好几回（发布 / 合并 / 状态回读），
    /// 会话缓存把它们并成一次；这条钉住的是**发布体验与进程内一致性**。
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

        // 存新值：缓存同步更新（读了也不该再读后端）
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
