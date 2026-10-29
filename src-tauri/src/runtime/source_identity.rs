//! **源身份**：把「云端在哪」规范化成一句可以拿来比的东西。
//!
//! ```text
//! github#mucorebenc/mkpsupportease@main
//! gitee#mucorebenc/mkpsupportease@main
//! custom#127.0.0.1:8787/presets@-
//! ```
//!
//! # 为什么需要它
//!
//! 「本地这份预设是不是落后于云端」原先只比 **SHA-256**：拿血统里记的那一版摘要，
//! 跟**当前源**目录里同路径那份比。问题出在「当前源」这三个字上 —— 换源之后，
//! 拿的还是同一把尺子，于是**任何**内容差异都成了「有新版」：
//!
//! ```text
//! GitHub 10-25 取的副本  ──切到──▶  Gitee 10-19 的目录
//!         sha A                        sha B
//!                         ⇒ A ≠ B ⇒ 「有新版」+ 绿色「更新」按钮（往回退）
//! ```
//!
//! 所以先要能回答一句：**这两份东西是不是同一个源的**。是，SHA 不同才叫"云端变了"；
//! 不是，SHA 不同只能说明"两边不一样"，谈不上新旧（见 [`super::local::BasedOn`]）。
//!
//! # 口径（作者 2026-10-27 定）
//!
//! **平台 + 仓库 + 分支/ref** —— 这三样才是"决定目录内容"的东西：
//!
//! - 只记域名不够：同一个 GitHub 仓库切了分支，内容照样是另一套；
//! - 只记仓库不够：GitHub 与 Gitee 的同名仓库是两个独立演化；
//! - 只记地址不够：`…/tree/main` 与 `…/raw/main/presets/delivery/source.json`
//!   是同一个源，两种写法。
//!
//! # 「不写死」是什么意思
//!
//! 平台名**从地址推**，不查"支持哪几家"的白名单 —— 自建源（`http://127.0.0.1:8787`
//! 那种）照样有自己的身份。唯一一张表是**主机同义词**（`raw.githubusercontent.com`
//! 与 `github.com` 是同一家），那是归一化，不是能力清单：多加一家不用改任何判定。
//!
//! # 不知道的时候怎么办
//!
//! `None`。猜一个"大概是同一个源"会让「有新版」重新变成不可信 —— 而这正是这个模块
//! 存在的理由。

use serde::{Deserialize, Serialize};

use super::source::{PresetSource, SourceMode};

/// 交付根在仓库里的约定位置（与 [`super::source`] 里那份同一个值）。
///
/// ★ 这里只用来**切掉尾巴**：`…/main/presets/delivery/source.json` 与
///   `…/tree/main` 指向同一份源，身份必须一样。
const DELIVERY_TAIL: &str = "presets/delivery";

/// 认不出 ref 时的显式占位。
///
/// ★ **不是**"默认分支"：`-` 就是"这个源没有 ref 概念"（自建目录源、本地 http.server）。
///   拿 `main` 之类的去填，会让两个不同的目录源看起来像同一支。
pub const NO_REF: &str = "-";

/// 规范化后的源身份。比较只看 [`SourceIdentity::key`]，别拿三个字段各自拼。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    /// 平台短名（`github` / `gitee`），自建源就是主机名本身（带端口）
    pub platform: String,
    /// `owner/repo`（小写）；自建源是去掉交付尾巴之后的那段路径
    pub repo: String,
    /// 分支 / ref（保留大小写，Git 语义）；没有这个概念时是 [`NO_REF`]
    pub git_ref: String,
}

impl SourceIdentity {
    /// 唯一比较口径：`平台#仓库@ref`。账本、日志、界面都写这一句。
    pub fn key(&self) -> String {
        format!("{}#{}@{}", self.platform, self.repo, self.git_ref)
    }
}

/// 把一份**已经存下来的**身份归一（老账里的 ref 可能还是 `%2F` 编码）。
///
/// 只动 `git_ref`：平台与仓库在写入时已经小写归一，不需要重算。
pub fn normalize(mut identity: SourceIdentity) -> SourceIdentity {
    identity.git_ref = decode_ref(&identity.git_ref);
    identity
}

/// **当前生效的源身份**。
///
/// 优先级与解析链同一条：自定义源先认**探通的那一份**（`customResolved`，
/// 见 [`PresetSource`] 的注释），没探过才退回用户填的那份地址；内置两个源
/// 用构建期注入的 Bootstrap 地址。
///
/// 内置地址一个都没注入（开发构建）⇒ `None`：**没配置就没有身份**，不编。
pub fn identity_of(source: &PresetSource) -> Option<SourceIdentity> {
    let url = match source.mode {
        SourceMode::Custom => source
            .custom_resolved
            .clone()
            .or_else(|| source.custom_url.clone())?,
        other => other.builtin_url()?.to_owned(),
    };
    parse_identity(&url)
}

/// 从一个地址（`source.json` 的 URL、交付目录、仓库/分支页都行）推源身份。
///
/// 三种形状殊途同归，判据只有一条：**切掉交付尾巴之后，剩下的前半截是谁**。
///
/// ```text
/// https://raw.githubusercontent.com/o/r/main/presets/delivery/source.json
///                       └ repo ┘└ref┘ └────── 切掉 ──────┘
/// https://gitee.com/o/r/raw/main/presets/delivery/source.json
/// https://github.com/o/r/tree/main          网页版：tree 之后全是 ref
/// http://127.0.0.1:8787/presets/delivery/   自建目录源：没有 ref
/// ```
pub fn parse_identity(url: &str) -> Option<SourceIdentity> {
    let trimmed = url.trim().trim_end_matches('/');
    let (scheme, scheme_rest) = trimmed.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let (host_raw, path) = match scheme_rest.split_once('/') {
        Some((h, p)) if !h.is_empty() => (h, p),
        Some((_h, _)) => return None,
        None => (scheme_rest, ""),
    };
    if host_raw.is_empty() {
        return None;
    }

    let host = host_raw.to_ascii_lowercase();
    let platform = platform_of(&host);
    let path = strip_delivery_tail(path);

    /* ① 网页 / raw 形状：`<owner>/<repo>/[tree|raw]/<ref…>` */
    if let Some(rest) = git_repo_rest(&host, &path) {
        let mut parts = rest.split('/').filter(|s| !s.is_empty());
        let owner = parts.next()?;
        let repo = parts.next()?.trim_end_matches(".git");
        if owner.is_empty() || repo.is_empty() {
            return None;
        }
        let after: Vec<&str> = parts.collect();
        let git_ref = match after.as_slice() {
            /* `tree/<ref…>` / `raw/<ref…>`：取完标记之后的全部（ref 本身可以带斜杠） */
            [marker, rest @ ..] if matches!(*marker, "tree" | "raw") => {
                if rest.is_empty() {
                    return None;
                }
                rest.join("/")
            }
            /* raw.githubusercontent.com 没有标记段，repo 之后直接是 ref */
            _ if host == "raw.githubusercontent.com" && !after.is_empty() => after.join("/"),
            _ => NO_REF.to_owned(),
        };
        return Some(SourceIdentity {
            platform,
            repo: format!("{}/{repo}", owner.to_ascii_lowercase()).to_ascii_lowercase(),
            git_ref: decode_ref(&git_ref),
        });
    }

    /* ② 目录形状（自建源 / 本地 http.server）：**主机 + 路径**才是"仓库"，没有 ref */
    let custom_repo = if path.is_empty() {
        host.clone()
    } else {
        format!("{host}/{}", path.trim_matches('/'))
    };
    Some(SourceIdentity {
        platform: "custom".to_owned(),
        repo: custom_repo,
        git_ref: NO_REF.to_owned(),
    })
}

/// 主机 → 平台短名。
///
/// ★ 这不是"支持哪几家"的清单 —— 判定不按平台分流，这里只做**同义词归一**：
///   `raw.githubusercontent.com` 与 `github.com` 是同一家，别让同一份源长出两个身份。
///   认不出的主机（自建、内网、localhost）就用它自己（带端口）。
fn platform_of(host: &str) -> String {
    if host == "github.com" || host.ends_with(".githubusercontent.com") {
        return "github".to_owned();
    }
    if host == "gitee.com" || host == "raw.gitee.com" {
        return "gitee".to_owned();
    }
    host.to_owned()
}

/// 只对「Git 托管页 / raw 地址」返回 `<owner>/<repo>/…` 剩下的那段；目录源返回 `None`。
fn git_repo_rest<'a>(host: &str, path: &'a str) -> Option<&'a str> {
    let is_git_host = matches!(platform_of(host).as_str(), "github" | "gitee")
        || host == "raw.githubusercontent.com";
    if !is_git_host {
        return None;
    }
    /* raw.githubusercontent.com 的路径**不带** /raw 标记段，其余两家都带 */
    Some(path.trim_start_matches('/'))
}

/// 切掉交付尾巴：`presets/delivery/source.json` / `presets/delivery` / 单独的 `source.json`。
///
/// 分支名里的百分号编码 → 原文。
///
/// Gitee / GitHub 的网页地址会把带斜杠的分支写成 `fix%2Fpublish-identity-matrix`；
/// 那和 `fix/publish-identity-matrix` 是**同一支**。不归一化，用户把地址从编码写法
/// 改成规范写法时，已下载的文件会突然被判成 `elsewhere` —— 那是把"换写法"误判成"换源"。
/// 非法序列原样保留（认不出就不猜）。
fn decode_ref(raw: &str) -> String {
    if !raw.contains('%') {
        return raw.to_owned();
    }
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_owned())
}

/// ★ 切完还要再切一次 `source.json`：`…/presets/delivery/source.json` 与
///   `…/presets/delivery` 是同一份源，别让它们长出两个身份。
fn strip_delivery_tail(path: &str) -> String {
    let mut out = path.trim_matches('/').to_owned();
    for tail in [
        DELIVERY_TAIL,
        &format!("{DELIVERY_TAIL}/source.json"),
        "source.json",
    ] {
        if let Some(cut) = out.strip_suffix(tail) {
            out = cut.trim_end_matches('/').to_owned();
            break;
        }
    }
    out.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(url: &str) -> SourceIdentity {
        parse_identity(url).expect("这条地址该认得出身份")
    }

    /// 两个内置源：同仓库不同平台 ⇒ **两个身份**（各自演化，谁也不比谁新）
    #[test]
    fn the_two_builtin_sources_are_different_identities() {
        let gh = id("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/source.json");
        let gt =
            id("https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/source.json");
        assert_eq!(gh.key(), "github#mucorebenc/mkpsupportease@main");
        assert_eq!(gt.key(), "gitee#mucorebenc/mkpsupportease@main");
        assert_ne!(gh, gt, "两个平台是两条线");
    }

    /// **同一个源的几种写法必须同一个身份**（否则换写法就被判成"换了源"）
    #[test]
    fn the_same_source_written_differently_is_one_identity() {
        let by_page = id("https://github.com/MuCoreBenC/MKPSupportEase/tree/main");
        let by_raw_dir =
            id("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery");
        let by_file = id("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/source.json");
        let by_slash = id("https://github.com/MuCoreBenC/MKPSupportEase/tree/main/");
        assert_eq!(by_page, by_raw_dir);
        assert_eq!(by_raw_dir, by_file);
        assert_eq!(by_file, by_slash);
    }

    /// **同一个分支的两种写法**（网页把 `/` 编码成 `%2F`）⇒ 同一份源
    #[test]
    fn a_percent_encoded_branch_is_the_same_source() {
        let plain =
            id("https://gitee.com/MuCoreBenC/MKPSupportEase/tree/fix/publish-identity-matrix");
        let encoded =
            id("https://gitee.com/MuCoreBenC/MKPSupportEase/tree/fix%2Fpublish-identity-matrix");
        let raw = id("https://gitee.com/MuCoreBenC/MKPSupportEase/raw/fix%2Fpublish-identity-matrix/presets/delivery/source.json");
        assert_eq!(plain.git_ref, "fix/publish-identity-matrix");
        assert_eq!(plain, encoded, "编码写法与规范写法是同一支");
        assert_eq!(plain, raw, "raw / tree 两种形状也是同一支");
    }

    /// 同仓库换分支 ⇒ 换了源（作者点名的第二种误判）
    #[test]
    fn another_branch_is_another_source() {
        let main = id("https://github.com/MuCoreBenC/MKPSupportEase/tree/main");
        let feat = id("https://github.com/MuCoreBenC/MKPSupportEase/tree/feat/dark-mode");
        assert_eq!(feat.git_ref, "feat/dark-mode");
        assert_ne!(main, feat);
    }

    /// 自建目录源：主机 + 路径就是仓库，没有 ref（不编一个 `main` 出来）
    #[test]
    fn a_self_hosted_directory_has_no_ref() {
        let local = id("http://127.0.0.1:8787/presets/delivery/source.json");
        assert_eq!(local.platform, "custom");
        assert_eq!(
            local.repo, "127.0.0.1:8787",
            "切掉交付尾巴之后什么都不剩：交付根就是源根，主机名已经够区分了"
        );
        assert_eq!(local.git_ref, NO_REF);
        assert_eq!(local.key(), "custom#127.0.0.1:8787@-");
        assert_eq!(
            id("http://127.0.0.1:8787/mirror/presets/delivery/source.json").repo,
            "127.0.0.1:8787/mirror",
            "同一台机器上放两份源也分得开"
        );
        assert_eq!(
            local,
            id("http://127.0.0.1:8787/presets/delivery/"),
            "带不带 source.json 是同一份"
        );
    }

    /// 认不出（非 http(s)、空主机）⇒ `None`，不猜
    #[test]
    fn unreadable_addresses_have_no_identity() {
        assert!(parse_identity("file:///tmp/presets").is_none());
        assert!(parse_identity("https:///nohost").is_none());
        assert!(parse_identity("").is_none());
        assert!(
            parse_identity("github.com/o/r").is_none(),
            "没写 scheme 就不认"
        );
    }

    /// 内置模式：构建期没注入地址的源没有身份（开发构建）
    #[test]
    fn a_source_without_address_has_no_identity() {
        let custom = PresetSource {
            source_schema: super::super::source::SOURCE_SCHEMA,
            mode: SourceMode::Custom,
            custom_url: Some("https://gitee.com/o/r/raw/main/presets/delivery/source.json".into()),
            custom_shape: super::super::source::CustomShape::Unset,
            custom_resolved: None,
        };
        assert_eq!(
            identity_of(&custom).map(|i| i.key()),
            Some("gitee#o/r@main".to_owned()),
            "没探过就按用户填的那份认"
        );

        let untested = PresetSource {
            custom_resolved: Some(
                "https://gitee.com/o/r/raw/feat/x/presets/delivery/source.json".into(),
            ),
            ..custom
        };
        assert_eq!(
            identity_of(&untested).map(|i| i.key()),
            Some("gitee#o/r@feat/x".to_owned()),
            "探通的那一份优先（它是真正在读的那份）"
        );
    }
}
