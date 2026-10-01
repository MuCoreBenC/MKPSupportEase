//! 「当前用哪个远端」——这份设置的读写与校验。
//!
//! # 为什么单独一个小模块
//!
//! 第二圈把下载从"本地目录源"换成真远端源之后，**远端的地址是谁、写在哪**成了一个
//! 必须有人管的持久状态。它不属于 [`super::delivery`]（那是管道，不管地址从哪来），
//! 也不属于 [`super::net`]（那是取字节的动作）——这里只answer一件事：
//! **这一台机器上，云端在哪**。
//!
//! # 为什么是这个形状
//!
//! 地址**不写进 catalog**：`presets/` 是内容源，换 Gitee 或换成自己的 CDN 是部署的事，
//! 不该为此重新发布一次说明书。于是分工变成:
//!
//! ```text
//! catalog 说：有一个文件叫 A1-standard.toml，它在交付集合里的位置是 mkp/A1-standard.toml
//! 这份设置说：当前的数据源是 https://…（官方 / Gitee / 自己的服务器）
//! 下载地址 = 这份设置的 baseUrl + catalog 的 path
//! ```
//!
//! 两半都是它们各自领域的唯一主人，`path` 不会因为换发布会变，`baseUrl` 不会因为
//! 内容改版而重算。**换源不用重新设计目录。**
//!
//! # 存储规矩（沿用 [`super::state`] 那一套，不另发明）
//!
//! 一种状态一个文件、住内部根 `run/` 下、带 `*Schema` 代次字段、写走 atomic_write、
//! 坏档报 `CORRUPTED` 不静默。它也**不进 localStorage**——C4 之后 localStorage 只住
//! 纯前端偏好，而这份地址最终要交给 Rust 侧去发起下载。
//!
//! # 没有地址时怎么办
//!
//! **如实说还没配置，不编一个 URL 出来假装能下。** 默认地址允许由构建方注入
//! （`MKPSE_PRESET_SOURCE=<url>`），注入了才有默认值；官方源 / Gitee 的真实地址
//! 现在还不归这个文件管——那是一次产品决定，不是这里猜的东西。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::SOURCE_FILE;

/// 设置格式的代次。加字段不升号，改语义才升（与 catalog / active-preset 同一条）
pub const SOURCE_SCHEMA: u32 = 1;

/// 构建方可注入的默认地址（`MKPSE_PRESET_SOURCE`）。**变量没设就是没配**，
/// 落到产品上的表现是下载命令诚实报「还没配置数据源地址」
const DEFAULT_BASE_URL: Option<&str> = option_env!("MKPSE_PRESET_SOURCE");

/// 当前选中的远端数据源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PresetSource {
    pub source_schema: u32,
    /// 数据源根地址（`https://example.com/mkp-content/`）。末尾不带斜杠——见 [`normalize_base_url`]
    pub base_url: String,
}

/// 设置文件的落点：`<appDataDir>/run/preset-source.json`
pub fn source_file(root: &Path) -> PathBuf {
    root.join(SOURCE_FILE)
}

/// 读当前设置。文件不存在 → `Ok(None)`（**没配过是合法状态**）；
/// 读得出来但解析不了 / 代次认不出 → `Err(CORRUPTED)`，不把它当"没配"。
///
/// 静默吞配置等于骗人：用户明明选过 Gitee，界面却说"没配"，那是界面在编数据。
pub fn load_source(root: &Path) -> Result<Option<PresetSource>, AppError> {
    let bytes = match std::fs::read(source_file(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::io("读不到数据源设置").with_detail(e.to_string())),
    };
    let source: PresetSource = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("数据源设置读不出来").with_detail(e.to_string()))?;
    if source.source_schema != SOURCE_SCHEMA {
        return Err(AppError::corrupted(format!(
            "数据源设置的格式代次认不了：文件是 {}，程序认 {}",
            source.source_schema, SOURCE_SCHEMA
        )));
    }
    Ok(Some(source))
}

/// 记下"当前用这个源"。整份替换（地址就一条，写新盖旧）。
///
/// 空地址在这里就拒绝写盘：写进去等于制造一个"配了但配成空"的第三种状态，
/// 下游要为它单独想一套分支。**想回到没配的状态，删掉那个文件**（用户可手删）。
pub fn save_source(root: &Path, raw_base_url: &str) -> Result<PresetSource, AppError> {
    let source = PresetSource {
        source_schema: SOURCE_SCHEMA,
        base_url: normalize_base_url(raw_base_url)?,
    };
    let json = serde_json::to_vec_pretty(&source)
        .map_err(|e| AppError::internal("数据源设置序列化失败").with_detail(e.to_string()))?;
    atomic_write(&source_file(root), &json)?;
    Ok(source)
}

/// 构建期注入的默认地址（没注就是 `None`）。界面用它区分"出厂默认值"与"用户改过的"，
/// 免得读一次之后分不清当前地址是自己选的还是出厂的
pub fn builtin_default() -> Option<String> {
    DEFAULT_BASE_URL.map(str::to_owned)
}

/// 当前生效的地址：**设置文件优先，其次构建期注入的默认值，都没有就是没配。**
///
/// 返回 `None` 的那一路要被界面原样说出来——那是唯一诚实的答案。
pub fn current_base_url(root: &Path) -> Result<Option<String>, AppError> {
    let stored = load_source(root)?.map(|s| s.base_url);
    let resolved = stored.or_else(builtin_default);
    Ok(resolved)
}

/// 把人填进来的地址收拾干净：去首尾空白、砍掉末尾多余的斜杠。
///
/// 只认 `http(s)`：这是**出站下载**用的地址。`file://`、UNC 路径这类Scheme
/// 在下载栈里会被理解成别的东西——那既是"下载能读到本机任意文件"的门，
/// 也绕过了「云端是远端」这个前提，趁它还没有第二个用途时关上。
pub fn normalize_base_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::invalid_argument("数据源地址是空的"));
    }
    let ok = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    if !ok {
        return Err(AppError::invalid_argument(format!(
            "数据源地址只认 http:// 或 https://，填进来的是 {trimmed}"
        )));
    }
    Ok(trimmed.to_owned())
}

/// 拼下载地址：`baseUrl` + catalog 记的相对位置（`path`）拼出最终 URL。
///
/// 两边各自可能带斜杠（人多敲一个、路径以 `/` 开头都常见），在这里抹一次，
/// 免得下游出现 `a//b` 这种"看起来对但服务端不认"的地址。
/// 路径段的百分号编码。**保留 `/`**（那是路径分隔符），其余按 RFC 3986 逐字节编码。
///
/// 为什么必须做这件事：资产的文件名里**实测有空格**（`MKPProcess A1 0.2 0.10.json`），
/// 不编码的话拼出来的 URL 在网络层才炸（`invalid uri character`），而那个报错指向
/// HTTP 客户端，往回查到"文件名带个空格"要大半天。编码放在拼地址这一处，
/// 于是"怎么拼"和"怎么编"只有一个答案。
pub fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            // 中文文件名也是逐字节编码（UTF-8 的百分号形式，服务端按同一套解）
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 下载地址 = 数据源的 base + 目录记的相对位置。
/// **位置在这里编码一次**（不许调用方各编各的，也不许编两次）
pub fn join_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        encode_path(path.trim_start_matches('/'))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().expect("临时目录建不出来")
    }

    /// 拼出来的地址必须是一个**合法 URL**。文件名里的空格是实测存在的
    /// （BBS 配置：`MKPProcess A1 0.2 0.10.json`），漏了编码就是网络层的怪错
    #[test]
    fn join_url_encodes_the_path_but_keeps_the_slashes() {
        assert_eq!(
            join_url(
                "https://example.com/mkp",
                "bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json"
            ),
            "https://example.com/mkp/bbs/Process/0.2mm/MKPProcess%20A1%200.2%200.10.json"
        );
        // 地址自身不动：它已经是一个 URL 了，再编一次会把 `:` `/` 也吃掉
        assert_eq!(
            join_url("https://example.com/", "catalog.json"),
            "https://example.com/catalog.json"
        );
        // 中文文件名：逐字节编码，服务端按 UTF-8 解回来
        assert_eq!(
            encode_path("mkp/bbs/中文.json"),
            "mkp/bbs/%E4%B8%AD%E6%96%87.json"
        );
    }

    /// 存进去再读出来必须还是那份——这条看着弱，但它钉的是"写的那套格式就是读的那套"
    #[test]
    fn roundtrips_the_chosen_source() {
        let dir = root();
        let saved =
            save_source(dir.path(), "https://cdn.example.com/mkp/").expect("写设置不该失败");
        let loaded = load_source(dir.path()).expect("读设置不该失败");

        assert_eq!(loaded, Some(saved));
        assert_eq!(
            loaded.expect("刚写完的").base_url,
            "https://cdn.example.com/mkp",
            "末尾多余的斜杠在写盘前就砍掉了"
        );
    }

    /// 没配过 ≠ 坏了：文件不在就是 `None`，不报错（默认值的分支由 `current_base_url` 管）
    #[test]
    fn missing_file_is_not_corrupted() {
        let dir = root();
        assert_eq!(load_source(dir.path()).expect("没配过不该报错"), None);
    }

    /// 坏档不静默：字节坏了要说出来，不能当成"没配"让用户再选一次却依然读不出
    #[test]
    fn unreadable_file_is_reported_not_swallowed() {
        let dir = root();
        atomic_write(&source_file(dir.path()), "这不是 JSON".as_bytes()).expect("写脏文件");

        let e = load_source(dir.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 代次认不出同样是坏档：老版本写的东西不说"看不懂"，就会在下一次读的时候变成谜
    #[test]
    fn future_schema_is_rejected() {
        let dir = root();
        atomic_write(
            &source_file(dir.path()),
            r#"{"sourceSchema": 99, "baseUrl": "https://x.example.com"}"#.as_bytes(),
        )
        .expect("写脏文件");

        let e = load_source(dir.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 空地址不许写盘：它会制造"配了但等于没配"的第三种状态
    #[test]
    fn empty_base_url_is_refused_before_writing() {
        let dir = root();
        let e = save_source(dir.path(), "   ").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(!source_file(dir.path()).exists(), "拒绝时不留半个文件");
    }

    /// 只放出站 HTTP：**本地文件 Scheme 走同一个下载栈会造成"下载读到本机任意文件"**
    #[test]
    fn local_paths_are_not_accepted_as_a_source() {
        for raw in [
            "file:///Users/me/presets",
            "/Users/me/presets",
            "\\\\server\\share",
            "ftp://example.com/presets",
        ] {
            let e = normalize_base_url(raw).unwrap_err();
            assert_eq!(
                e.code,
                crate::error::ErrorCode::InvalidArgument,
                "拒绝 {raw}"
            );
        }
    }

    /// 拼 URL 两边多带斜杠都要抹平，否则服务端拿到的不是一个它认的路径
    #[test]
    fn joining_url_tolerates_sloppy_slashes() {
        assert_eq!(
            join_url("https://cdn.example.com/mkp/", "/A1-standard.toml"),
            "https://cdn.example.com/mkp/A1-standard.toml"
        );
        assert_eq!(
            join_url("https://cdn.example.com/mkp", "A1-standard.toml"),
            "https://cdn.example.com/mkp/A1-standard.toml"
        );
    }
}
