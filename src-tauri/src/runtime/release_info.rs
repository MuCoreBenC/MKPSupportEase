//! **软件发布信息**（`release.json`）——「有没有新版本的 SupportEase」这条链的信息源。
//!
//! # 它与预设数据是**两条链**
//!
//! ```text
//! release.json   软件版本     住发布根（presets/）上一级   这条链：软件更新
//! catalog.json   预设数据     住 presets/delivery/          那条链：数据能不能读
//! ```
//!
//! 作者 2026-10-04 定死：**`release.json` 不参与发布闸的预设数据内容校验，也不进
//! catalog / manifest**。所以它不进 [`super::catalog`]，也不参与[`super::structure`]的
//! 结构签名 —— 它属于**软件发布阶段**，与"这批数据读不读得懂"无关。
//!
//! # 为什么必须有它
//!
//! 客户端"读得懂这一代数据"（[`super::structure::can_read`] 为真）**不等于**
//! "正式版已经发布了"。两者混为一谈的下场是：Dev 构建能读新结构，就以为线上也有新版了。
//! 于是更新检查需要**自己的正式 release 信息源** —— 就是这个文件。
//!
//! # 形状
//!
//! 三格必备 + 一格可选，多说一个字都是两端要一起改的契约：
//!
//! ```json
//! {
//!   "releaseSchema": 1,
//!   "version": "0.0.4",
//!   "notes": "…",
//!   "url": "https://…/releases/tag/v0.0.4",
//!   "asset": {                       // ← **可选**（2026-10-05 应用内下载这一刀）
//!     "name": "SupportEase_0.0.4_aarch64.app.zip",
//!     "url":   "https://…/releases/download/v0.0.4/SupportEase_0.0.4_aarch64.app.zip",
//!     "size":  12345678,
//!     "sha256": "…"                  // 有就校验，没有就只验大小
//!   }
//! }
//! ```
//!
//! ★ **`asset` 是可选的，加它不升代次**（纪律：加字段不升号）。**没有它就退回
//!   "打开下载页"** —— 那是 0.0.2 / 0.0.3 的行为，仍然成立、仍然能用。
//!   换句话说：发布方**先具备**应用内下载能力，客户端**才**会用；反过来永远成立。
//!
//! 版本号**与 `Cargo.toml` 同源**（发布侧写它时取 `CARGO_PKG_VERSION`）——
//! "软件版本"全局只有一处真值。将来换成 GitHub Releases，只换**消费层**取这份数据的方式
//! （[`super::net`] 里换一个取法），不动预设发布架构（作者定死的演进路线）。

use postprocess::diag::messages;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::structure::{version_at_least, APP_VERSION};

/// 软件更新通道（2026-11-05 双通道）。
///
/// 逐档 match，不许 `_ =>` 兜底成某一档：认不出的通道名要**显式报错**，
/// 否则一句拼错的 `channel` 会悄悄把用户点中的预览版下成正式版。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateChannel {
    /// 正式版：会主动通知
    Stable,
    /// 预览版：只在用户主动点「重新检查」时出现
    Preview,
}

/// 缺省通道 = 正式版。
///
/// 存在的理由：发布事务的入参里 `channel` 是可省的一格（`#[serde(default)]`）——
/// 老调用方（工作台旧前端、老 CLI 参数）不带它，语义必须还是"发正式版"，
/// 而不是编译不过或者悄悄变成别的通道。
impl Default for UpdateChannel {
    fn default() -> Self {
        Self::Stable
    }
}

impl UpdateChannel {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "stable" => Some(Self::Stable),
            "preview" => Some(Self::Preview),
            _ => None,
        }
    }

    pub fn wire(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }
}

/// `release.json` 的协议代次。**加字段不升号、改语义才升**（与 source / catalog 同一条）。
pub const RELEASE_SCHEMA: u32 = 1;

/// 软件发布信息（`release.json` 的内容）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    /// 发布出去的正式版本号（应与构建期 `CARGO_PKG_VERSION` 同源）
    pub version: String,
    /// 更新说明（给人看的一句话 / 一段话），可省
    #[serde(default)]
    pub notes: String,
    /// 去哪更新 / 看详情，可省
    #[serde(default)]
    pub url: String,
    /// 协议代次。缺省按当前代次（老发布物没写这一格时不因此判坏）
    #[serde(default = "default_schema")]
    pub release_schema: u32,
    /// **可下载的安装包**（`.app.zip`）。**没有这一格 = 只能打开下载页** ——
    /// 老发布物天生如此，客户端会照旧退回那条路（不是错误状态）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<ReleaseAsset>,
    /// **双平台资产**（2026-11-05，可选格，加它不升 `releaseSchema`）。
    ///
    /// `asset` 保留为老客户端的**单资产入口**（macOS `.app.zip`），不删不改语义。
    /// 新客户端在 `assets[]` 里按当前平台挑一份；没给 `assets` 就照旧用 `asset`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<ReleaseAsset>>,
}

/// 发布出去的**安装包**（应用内下载这一刀新增，可选格）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    /// 文件名（形如 `SupportEase_0.0.4_aarch64.app.zip`）
    pub name: String,
    /// 下载地址
    pub url: String,
    /// 字节数。**给了就照它收**（下载完大小对不上 = 这一份坏了，不装）
    #[serde(default)]
    pub size: u64,
    /// 校验和。**有就校验**（没有就只验大小 —— 那是弱一档，但不是没有）
    #[serde(default)]
    pub sha256: String,
    /// 适用平台（`macos` / `windows`）。老发布物没这一格 ⇒ 按「macOS 老入口」处理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// 资产种类：`appZip`（macOS 解包替换）/ `nsis`（Windows 静默安装）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

impl ReleaseInfo {
    /// 挑**当前平台**能用的安装包。
    ///
    /// 优先在 `assets[]` 里按 `platform` 找；没给 `assets` 就退回老的单 `asset`
    /// （macOS `.app.zip` 那条路，0.0.2 起一直成立）。`platform` 缺省的资产
    /// 视为「不挑平台」，也能被选中。找不到就 `None` —— 界面退回「打开下载页」。
    pub fn asset_for_host(&self) -> Option<ReleaseAsset> {
        if let Some(list) = &self.assets {
            let here = host_platform();
            let picked = list
                .iter()
                .find(|a| a.platform.as_deref() == Some(here))
                .or_else(|| list.iter().find(|a| a.platform.is_none()))
                .cloned();
            if picked.is_some() {
                return picked.filter(|a| a.is_downloadable());
            }
        }
        self.asset.clone().filter(|a| a.is_downloadable())
    }
}

impl ReleaseAsset {
    /// 这一格**能不能拿来下载**（文件名与地址都得有，地址只认 http(s)）。
    ///
    /// 这是**纯判定**：发布方写错字段时客户端要能拒，而不是拿着空地址去下载。
    pub fn is_downloadable(&self) -> bool {
        !self.name.trim().is_empty()
            && (self.url.starts_with("https://") || self.url.starts_with("http://"))
    }

    /// 要校验的 SHA-256（小写十六进制；空 = 不校验）
    pub fn expect_sha256(&self) -> Option<String> {
        let t = self.sha256.trim().to_ascii_lowercase();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }
}

fn default_schema() -> u32 {
    RELEASE_SCHEMA
}

/// 解析 `release.json` 的字节。坏 JSON / 代次认不出 → `CORRUPTED`（不静默当"没更新"）。
pub fn parse(bytes: &[u8]) -> Result<ReleaseInfo, AppError> {
    let info: ReleaseInfo = serde_json::from_slice(bytes).map_err(|e| {
        AppError::corrupted(messages::text("ui.runtime.release_info.429", &[]))
            .with_detail(e.to_string())
    })?;
    if info.release_schema != RELEASE_SCHEMA {
        return Err(AppError::corrupted(messages::text(
            "ui.runtime.release_info.430",
            &[
                ("v1", &info.release_schema.to_string()),
                ("v2", &RELEASE_SCHEMA.to_string()),
            ],
        )));
    }
    if info.version.trim().is_empty() {
        return Err(AppError::corrupted(messages::text(
            "ui.runtime.release_info.431",
            &[],
        )));
    }
    Ok(info)
}

/// 「当前这一台机器该不该提示更新」的**纯判定** —— 给界面用的结果形状。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareUpdate {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub notes: Option<String>,
    pub url: Option<String>,
    /// **可下载的安装包**（`None` = 这一版只能打开下载页）
    pub asset: Option<ReleaseAsset>,
}

/// **软件版本**专用解析（`major.minor.patch` + 可选预发布标识）。
///
/// 2026-11-05 之前这里只有 `structure::version_parts`：按非数字切开、取前三段，
/// 于是 `0.0.10-preview.1` 与 `0.0.10` 都变成 `[0,0,10]`，`compare` 只要
/// 字符串不等就判「有新版」—— 预览版会被当成比正式版还新，0.0.9 也会被它打扰。
///
/// ★ `structure::version_parts` / `version_at_least` **保持不动**：它们服务
///   `can_read()` / `minClient`（预设数据能不能读），属另一条链。软件版本比较
///   从这里另起一套，不牵动数据链。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// `None` = 正式版（同基础版本里最高）；`Some` = 预发布，序号越大越新
    pub pre: Option<(u8, u64)>,
}

/// 预发布标识的档次（SemVer 惯例）：dev < alpha < beta < preview < rc。
/// 认不出的标识一律按 `dev` 这一档（保守：不会把它拔高成正式版）。
fn pre_rank(kind: &str) -> u8 {
    match kind {
        "alpha" | "a" => 1,
        "beta" | "b" => 2,
        "preview" | "pre" => 3,
        "rc" => 4,
        _ => 0, // dev / dev2 / 未知后缀
    }
}

/// 解析 `v0.0.10-preview.1` / `0.0.10-dev2` / `0.0.10` 这类软件版本号。
/// 解不出就 `None`（调用方退回旧的数字比较，不静默编一个版本）。
pub fn parse_software_version(raw: &str) -> Option<SoftwareVersion> {
    let t = raw.trim().trim_start_matches('v');
    let (core, suffix) = match t.split_once('-') {
        Some((c, s)) => (c, Some(s)),
        None => (t, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let pre = suffix.map(|s| {
        let (kind, seq) = match s.rsplit_once('.') {
            Some((k, n)) => match n.parse::<u64>() {
                Ok(v) => (k, v),
                Err(_) => (s.trim_end_matches(|c: char| c.is_ascii_digit()), 0),
            },
            // `dev2` 这种把数字粘在词尾的写法也认
            None => {
                let cut = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
                let n = s[cut..].parse::<u64>().unwrap_or(0);
                (&s[..cut], n)
            }
        };
        (pre_rank(kind), seq)
    });
    Some(SoftwareVersion {
        major,
        minor,
        patch,
        pre,
    })
}

/// 这是不是预发布版本（`dev` / `preview.N` / `rc.N` / `alpha.N` / `beta.N`）。
pub fn is_prerelease(raw: &str) -> bool {
    parse_software_version(raw).is_some_and(|v| v.pre.is_some())
}

/// 比较"当前构建版本"与"远端发布版本"。
///
/// 旧口径：`version_parts` 只取前三段数字 + 字符串不等就算更新 —— 会把
/// `0.0.10-preview.1` 判成比 `0.0.10` 新，也会让 0.0.9 正式版用户收到预览版提示。
/// 新口径：软件版本专用解析，正式版 > 同基础版本的任何预发布；
/// **预发布版绝不主动提醒正式版用户**（哪怕它的数字段更大）。
///
/// 保住的：`latest` 严格新才算更新、相等不是更新、回退发布不算更新、
/// `-dev` 后缀不干扰三段比较、`compare("9.9.9", …)` 仍然为真。
/// 放弃的：把预发布版当"新版本"提示给正式版用户 —— 那正是本次要消灭的事故。
/// 为什么：预览版只该在用户主动打开更新页时并列出现，通道切换由用户点选决定。
pub fn compare(latest: &str, current: &str) -> bool {
    let l = latest.trim();
    let c = current.trim();
    /* ★ 守卫必须在**数字比较之前**：`0.0.10-preview.1` 的基础数字比 `0.0.9` 大，
     * 若先比数字就会返回 true，把预览版推给正式版用户 —— 正是本次要消灭的事故。 */
    if let (Some(lv), Some(cv)) = (parse_software_version(l), parse_software_version(c)) {
        if lv.pre.is_some() && cv.pre.is_none() {
            return false;
        }
    }
    is_newer_version(l, c)
}

/// **纯版本序**：`candidate` 是否真的比 `current` 新。
///
/// 三段数字 → 同基础时**正式版 > 任何预发布** → 同为预发布比档次与序号。
/// **不带**"预发布不提醒正式版用户"那条**通知**守卫。
///
/// ★ 两个问题必须分开问（2026-11-06 作者点破："0.0.6 当成有新版了，实际上应该要比
///   当前版本大才算"）：
///   · **要不要通知他** —— [`compare`]，带守卫：预览版绝不主动提醒正式版用户；
///   · **这张卡是不是"更新"** —— 本函数，不带守卫：0.0.9 上看到 0.0.10-preview.1，
///     它是更新的版本（只是不会弹通知，要用户自己点开看）。
///
/// 混用两个问题会同时错两头：拿 `compare` 判展示 → 更新的预览版被标成"较旧"；
/// 拿本函数判通知 → 预览版推给全量用户。所以这里只回答"谁大"，不掺通知策略。
pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    let l = candidate.trim();
    let c = current.trim();
    if l == c {
        return false;
    }
    let (Some(lv), Some(cv)) = (parse_software_version(l), parse_software_version(c)) else {
        // 认不出的形状：退回原来那套数字比较（老发布物/脏数据仍按旧口径走，不静默改判）
        return version_at_least(l, c);
    };
    let base = (lv.major, lv.minor, lv.patch).cmp(&(cv.major, cv.minor, cv.patch));
    if base != std::cmp::Ordering::Equal {
        return base == std::cmp::Ordering::Greater;
    }
    // 同基础：`#[derive(Ord)]` 直接比 `Option<pre>` 是错的（`None < Some(_)`），逐档写清
    match (lv.pre, cv.pre) {
        (Some(_), None) => false, // 预发布不比同基础的正式版新
        (None, Some(_)) => true,  // 正式版比同基础的预发布新（pre/dev → 正式版算更新）
        (Some(a), Some(b)) => a > b,
        (None, None) => false,
    }
}

/// 一份**可主动选择的更新候选**（2026-11-05 双通道）。
///
/// 与 `SoftwareUpdate` 的区别：那是「自动通知」的结论（只看正式版）；
/// 这是「用户主动刷新时能看见什么」（正式版 + 预览版并列）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCandidate {
    pub channel: UpdateChannel,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<ReleaseAsset>,
}

impl UpdateCandidate {
    pub fn from_info(channel: UpdateChannel, info: &ReleaseInfo) -> Self {
        Self {
            channel,
            version: info.version.clone(),
            notes: non_empty(&info.notes),
            url: non_empty(&info.url),
            asset: info.asset_for_host(),
        }
    }
}

/* --------------------------- 双平台资产（可选，加字段不升代次） --------------------------- */

/// 当前构建的平台名（写进 `assets[]` 的 `platform` 用同一个词表）。
#[cfg(target_os = "windows")]
pub fn host_platform() -> &'static str {
    "windows"
}
#[cfg(target_os = "macos")]
pub fn host_platform() -> &'static str {
    "macos"
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn host_platform() -> &'static str {
    "other"
}

/// 由一份发布信息得出给界面的更新状态（`current` 可注入，判据用）。
pub fn to_update_for(info: &ReleaseInfo, current: &str) -> SoftwareUpdate {
    SoftwareUpdate {
        has_update: compare(&info.version, current),
        current_version: current.to_owned(),
        latest_version: info.version.clone(),
        notes: non_empty(&info.notes),
        url: non_empty(&info.url),
        asset: info.asset_for_host(),
    }
}

/// 用本构建版本 [`APP_VERSION`] 判定（生产路径）。
pub fn to_update(info: &ReleaseInfo) -> SoftwareUpdate {
    to_update_for(info, APP_VERSION)
}

/// 「没有发布信息源 / 还没部署」时的状态：如实报"已是最新"（**不是更新**），
/// 当前版本仍给出本构建的真实版本 —— 界面永远有东西可说。
pub fn none_available() -> SoftwareUpdate {
    SoftwareUpdate {
        has_update: false,
        current_version: APP_VERSION.to_owned(),
        latest_version: APP_VERSION.to_owned(),
        notes: None,
        url: None,
        asset: None,
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[cfg(test)]
mod tests {
    /// ★ **老发布物没有 `asset` 格** —— 那不是坏档，客户端退回"打开下载页"
    #[test]
    fn a_release_without_an_asset_still_parses() {
        let v = br#"{"releaseSchema":1,"version":"0.0.3","notes":"x","url":"https://h/t"}"#;
        let info = parse(v).expect("老格式该读得懂");
        assert_eq!(info.asset, None);
        let u = to_update(&info);
        assert_eq!(u.asset, None, "没有资产 ⇒ 界面只摆「查看更新」");
    }

    /// 有 `asset` 且地址合法 → 客户端拿它做应用内下载
    #[test]
    fn a_downloadable_asset_is_offered_to_the_client() {
        let v = br#"{"releaseSchema":1,"version":"0.0.4","url":"https://h/t",
            "asset":{"name":"S_0.0.4_aarch64.app.zip","url":"https://h/d/S.zip","size":123,
                     "sha256":"ABCD"}}"#;
        let u = to_update(&parse(v).unwrap());
        let a = u.asset.expect("有资产就该给");
        assert_eq!(a.name, "S_0.0.4_aarch64.app.zip");
        assert_eq!(a.expect_sha256().as_deref(), Some("abcd"), "校验和按小写比");
    }

    /// ★ **地址不是 http(s) 的资产要被拒**（发布方写错了不能让客户端拿着去下载）
    #[test]
    fn an_unusable_asset_url_is_refused() {
        let v = br#"{"releaseSchema":1,"version":"0.0.4","url":"https://h/t",
            "asset":{"name":"x.zip","url":"file:///etc/passwd","size":1}}"#;
        let u = to_update(&parse(v).unwrap());
        assert_eq!(u.asset, None, "file: 这种形状不能拿去下载");
    }

    use super::*;

    #[test]
    fn parses_a_well_formed_release() {
        let json = r#"{"version":"9.9.9","notes":"修了几个问题","url":"https://x.example.com"}"#;
        let info = parse(json.as_bytes()).expect("好档该解析得动");
        assert_eq!(info.version, "9.9.9");
        assert_eq!(info.notes, "修了几个问题");
        assert_eq!(info.url, "https://x.example.com");
        assert_eq!(info.release_schema, RELEASE_SCHEMA, "缺省代次 = 当前代次");
    }

    /// 坏 JSON 不静默：不许当成"没有更新"糊过去（那是在骗用户"你是最新的"）
    #[test]
    fn broken_json_is_corrupted_not_silently_up_to_date() {
        let e = parse(b"not json at all").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 代次认不出同样是坏档：不猜新版长什么样
    #[test]
    fn future_schema_is_refused() {
        let e = parse(br#"{"version":"1.0.0","releaseSchema":99}"#).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 没有版本号 = 坏档（一份 release.json 说"我发布了"却说不出是哪个版本，是坏的）
    #[test]
    fn empty_version_is_corrupted() {
        let e = parse(br#"{"version":""}"#).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// `release.json` 是**软件版本**这一条链：比较只看版本号，
    /// 与预设数据（catalog / 结构签名）没有任何关系。
    #[test]
    fn release_json_is_its_own_source_not_preset_data() {
        // 一份"只谈软件版本"的 JSON 就能得出更新状态，不需要任何 catalog 字段
        let info = parse(br#"{"version":"999.0.0"}"#).expect("该解析得动");
        let up = to_update(&info);
        assert!(up.has_update, "远高于当前版本 = 有更新");
        assert_eq!(up.current_version, APP_VERSION);
        assert_eq!(up.latest_version, "999.0.0");
    }

    /// **旧判据 → 新判据**（2026-11-05）。
    ///
    /// 旧：`-dev` 后缀不干扰三段数字比较，所以 `0.1.0-dev` 比 `0.0.9` 新就算更新。
    /// 新：软件版本真正识别预发布标识，**任何预发布版都不主动提醒正式版用户**
    ///     （作者：「dev 是我自己测试用，不想让用户收到」）。
    /// 保住：三段数字比较、相等 / 回退不提示；正式版升级照旧。
    /// 放弃：「预发布版只要数字更大就当普通新版提示」—— 那正是本次要消灭的事故。
    /// 为什么：预览 / dev 只该在用户主动打开更新页时并列出现。
    #[test]
    fn software_version_compares_by_semver_and_ignores_dev_suffix() {
        assert!(compare("0.0.2", "0.0.1"), "更高 = 有更新");
        assert!(!compare("0.0.1", "0.0.1"), "相等 = 没有更新");
        assert!(!compare("0.0.0", "0.0.1"), "更低（回退）不算有更新");
        assert!(
            !compare("0.1.0-dev", "0.0.9"),
            "★ 开发构建再新也不主动提醒正式版用户"
        );
        assert!(
            compare("0.1.0", "0.0.9-dev"),
            "正式版比旧的开发构建新 —— 主动切回正式版成立"
        );
    }

    /// **软件版本真正识别预发布标识**（2026-11-05 双通道）。
    ///
    /// 旧判据：只测三段数字 + `-dev` 不干扰，`0.0.10-preview.1` 与 `0.0.10`
    /// 都变成 `[0,0,10]`，字符串不等就判「有新版」。
    /// 新判据：正式版 > 同基础版本任何预发布；预发布绝不主动提醒正式版用户。
    /// 保住的：三段数字比较、相等/回退不提示、`-dev` 不干扰基础版本比较。
    /// 放弃的：把预发布版当「新版本」提示 —— 那正是本次要消灭的事故。
    /// 为什么：预览版只该在用户主动打开更新页时并列出现，通道切换靠用户点选。
    #[test]
    fn prerelease_versions_are_recognized_and_never_announced_to_stable_users() {
        assert!(
            !compare("0.0.10-preview.1", "0.0.9"),
            "★ 0.0.9 正式版用户绝不能被预览版打扰"
        );
        assert!(
            !compare("0.0.10-preview.1", "0.0.10"),
            "预览版不能被判成比同基础正式版新"
        );
        assert!(
            compare("0.0.10", "0.0.10-preview.1"),
            "正式版比同基础预览版新（预览 → 正式可以切回）"
        );
        assert!(
            compare("0.0.10-preview.2", "0.0.10-preview.1"),
            "同一预览通道里序号大的更新"
        );
        assert!(
            compare("0.0.10-preview.1", "0.0.10-dev"),
            "★ 装的是 0.0.10-dev 的人要能看到这个预览版（两边都是预发布 → 比档次与序号）"
        );
        assert!(!compare("0.0.10-preview.1", "0.0.10-preview.1"));
        assert!(
            !compare("0.0.11-preview.1", "0.0.10"),
            "哪怕基础版本更高，预览版也不主动提醒正式版用户"
        );
        assert!(compare("999.0.0", "0.0.10-dev"), "正式版远高于开发构建");
    }

    /// ★ **"谁比谁新"与"要不要通知"是两个问题**（2026-11-06 作者点破）。
    ///
    /// 症状：装了 `0.0.10-dev`，设置页把正式版 `0.0.6` 摆在"推荐 / 更新正式版"上 ——
    /// 它比当前版本**旧**，根本不是更新。
    #[test]
    fn is_newer_version_answers_size_only_and_ignores_the_notify_guard() {
        // 比当前旧 ⇒ 不是更新（正是那条 bug）
        assert!(!is_newer_version("0.0.6", "0.0.10-dev"));
        assert!(!is_newer_version("0.0.9", "0.0.10"));
        // 相等不是更新
        assert!(!is_newer_version("0.0.10", "0.0.10"));
        // 同基础：正式版比 pre / dev 新（用户："0.0.10 的 pre 或者 dev 都把 0.0.10 当新版"）
        assert!(is_newer_version("0.0.10", "0.0.10-dev"));
        assert!(is_newer_version("0.0.10", "0.0.10-preview.1"));
        // 同基础：pre 比 dev 新，序号大者新
        assert!(is_newer_version("0.0.10-preview.1", "0.0.10-dev"));
        assert!(is_newer_version("0.0.10-preview.2", "0.0.10-preview.1"));
        // 同基础：预发布不比正式版新（用户："0.0.11 不会把 0.0.11 的 pre/dev 当新版"）
        assert!(!is_newer_version("0.0.11-preview.1", "0.0.11"));
        // 更高基础：一律更新（用户："只会把 0.0.12 的 pre 或者 0.0.12 正式版当新版"）
        assert!(is_newer_version("0.0.12-preview.1", "0.0.11-dev"));
        assert!(is_newer_version("0.0.12", "0.0.11"));

        /* ★ 不带通知守卫：0.0.9 正式版看到 0.0.10-preview.1 —— 它**是**更新的版本
         *  （用户主动点开才看得到），只是不会被**通知**。两个函数刻意分开：
         *  混用会一头把"较旧"标成"更新"，另一头把预览版推给全量用户。 */
        assert!(is_newer_version("0.0.10-preview.1", "0.0.9"));
        assert!(!compare("0.0.10-preview.1", "0.0.9"), "但绝不主动通知");
    }

    /// 解析形状：`v` 前缀、`.N` 序号、`dev2` 粘连写法、未知后缀一律按预发布处理。
    #[test]
    fn software_version_parser_shapes() {
        assert_eq!(
            parse_software_version("v0.0.10"),
            Some(SoftwareVersion {
                major: 0,
                minor: 0,
                patch: 10,
                pre: None,
            })
        );
        let p = parse_software_version("0.0.10-preview.1").unwrap();
        assert_eq!(p.pre, Some((3, 1)));
        let d = parse_software_version("0.0.10-dev2").unwrap();
        assert_eq!(d.pre, Some((0, 2)));
        assert!(is_prerelease("0.0.10-rc.1"));
        assert!(!is_prerelease("0.0.10"));
        assert_eq!(parse_software_version("not-a-version"), None);
        assert_eq!(parse_software_version("1.2.3.4"), None);
    }

    /// 没有信息源时：如实报"已是最新"，但当前版本照给 —— 界面不会因此出现空白
    #[test]
    fn a_missing_source_reports_up_to_date_with_the_real_version() {
        let up = none_available();
        assert!(!up.has_update);
        assert_eq!(up.current_version, APP_VERSION);
        assert_eq!(up.latest_version, APP_VERSION);
    }

    /// ★ **落点**（2026-10-05 寻址改造裁定，总纲 §1②）：`release.json` 入住交付根
    /// **`presets/delivery/release.json`** —— 它本来就是客户端消费的交付元数据，
    /// 与 catalog / manifest / source 同边界；地址由 Source Manifest 的 `release`
    /// 声明给出（旧的"从 base 上跳一级"推导已废除）。
    ///
    /// 三条一起钉：① 交付根**有**这一份 ② 仓库根与 `presets/` 根**都没有**第二份
    /// （两份就会有一份是假的）③ Resolver 解出来的 URL 停在这一份上。
    /// 文件挪错位置的当天这条就红 —— 这正是它存在的理由。
    #[test]
    fn the_release_file_lives_in_the_delivery_root() {
        // `CARGO_MANIFEST_DIR` = <repo>/src-tauri（编译期填的仓库路径，与其余判据同一取法）
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri 上面就是仓库根")
            .to_path_buf();

        for stray in [root.join("release.json"), root.join("presets/release.json")] {
            assert!(
                !stray.exists(),
                "{} 不该有 release.json —— 它住 presets/delivery/ 了",
                stray.display()
            );
        }

        let bytes = std::fs::read(root.join("presets/delivery/release.json"))
            .expect("presets/delivery/release.json 该存在（客户端按 Manifest 声明取它）");
        let info = parse(&bytes).expect("这一份该解析得动");
        assert_eq!(info.release_schema, RELEASE_SCHEMA, "代次该是当前代次");
        assert!(!info.version.trim().is_empty(), "该写着一个版本号");

        // 地址由 Source Manifest 的 release 声明给出（交付根 = source.json 所在目录）
        let resolver = crate::runtime::resolver::SourceResolver::from_bootstrap(
            "https://host/main/presets/delivery",
            br#"{"sourceSchema":2,"catalog":"catalog.json","release":"release.json","filesRoot":".."}"#,
        )
        .expect("Manifest 该解析得动");
        let url = resolver
            .resolve(crate::runtime::resolver::ResourceRef::Release)
            .expect("该解析得出")
            .remote_or("软件发布信息（release）")
            .expect("该推得出地址");
        assert!(
            url.ends_with("/presets/delivery/release.json"),
            "客户端会去 {url} —— 它该是 delivery/release.json"
        );
    }

    #[test]
    fn empty_notes_and_url_become_none() {
        let info = parse(br#"{"version":"1.0.0","notes":"  ","url":""}"#).expect("该解析得动");
        let up = to_update(&info);
        assert_eq!(up.notes, None);
        assert_eq!(up.url, None);
    }
}
