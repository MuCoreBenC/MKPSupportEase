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

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::structure::{version_at_least, APP_VERSION};

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
        AppError::corrupted("软件发布信息（release.json）解析不了").with_detail(e.to_string())
    })?;
    if info.release_schema != RELEASE_SCHEMA {
        return Err(AppError::corrupted(format!(
            "软件发布信息的格式代次认不了：文件是 {}，程序认 {}",
            info.release_schema, RELEASE_SCHEMA
        )));
    }
    if info.version.trim().is_empty() {
        return Err(AppError::corrupted(
            "软件发布信息里没有版本号（version 是空的）",
        ));
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

/// 比较"当前构建版本"与"远端发布版本"。
///
/// 用 [`version_at_least`] 反向复用（同一套 `major.minor.patch`、同一套 `-dev` 忽略规则）——
/// **版本比较只有一处实现**，这里不另写一遍。
///
/// `latest` 比当前**严格新**才算有更新（相等不是更新）；比当前旧（回退发布 / 误配）也不算
/// "有更新" —— 提示一个更旧的版本没有任何意义。
pub fn compare(latest: &str, current: &str) -> bool {
    let l = latest.trim();
    let c = current.trim();
    if l == c {
        return false;
    }
    version_at_least(l, c)
}

/// 由一份发布信息得出给界面的更新状态（`current` 缺省 = 本构建的 [`APP_VERSION`]）。
pub fn to_update(info: &ReleaseInfo) -> SoftwareUpdate {
    SoftwareUpdate {
        has_update: compare(&info.version, APP_VERSION),
        current_version: APP_VERSION.to_owned(),
        latest_version: info.version.clone(),
        notes: non_empty(&info.notes),
        url: non_empty(&info.url),
        asset: info.asset.clone().filter(|a| a.is_downloadable()),
    }
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

    #[test]
    fn software_version_compares_by_semver_and_ignores_dev_suffix() {
        assert!(compare("0.0.2", "0.0.1"), "更高 = 有更新");
        assert!(!compare("0.0.1", "0.0.1"), "相等 = 没有更新");
        assert!(!compare("0.0.0", "0.0.1"), "更低（回退）不算有更新");
        // `-dev` 后缀不干扰三段比较（与 struct 的 version_at_least 同一套规则）
        assert!(compare("0.1.0-dev", "0.0.9"), "-dev 后缀不该干扰");
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
