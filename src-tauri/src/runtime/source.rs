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
//! # 两种入口（第十七刀起）
//!
//! "当前用哪个远端"有两个来源，语义**故意不同**：
//!
//! ```text
//! 手动覆盖（设置页 → 高级设置）  = 一个数据源根：根下就是 catalog.json 与各文件
//!                                 （开发 / 排查通道：本地 http.server、自建镜像…）
//! 内置（构建期注入）            = 一个 Bootstrap 地址：指向 source.json，
//!                                 由它说"catalog 在哪、文件下载根在哪"
//!                                 （正式通道：换部署位置只改 source.json，客户端不重发）
//! ```
//!
//! 覆盖优先。两条路解析出来的是**同一个形状**（[`ResolvedSource`]：catalog 地址 +
//! 文件下载根），下游（下载 / 检查更新 / 应用更新）只认它，不各自拼 URL。
//!
//! # 没有地址时怎么办
//!
//! **如实说还没配置，不编一个 URL 出来假装能下。** 默认地址由构建方注入：
//! 环境变量 `MKPSE_PRESET_SOURCE` > 工作台配置（`workbench/bootstrap.json`，
//! 见 `src-tauri/build.rs`）—— 注入了才有默认值。正常构建内置的是**官方 Bootstrap**；
//! "没配"只该发生在开发构建（既没注入也没手动指定）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::SOURCE_FILE;

/// 设置格式的代次。加字段不升号，改语义才升（与 catalog / active-preset 同一条）。
///
/// ★ **2**（2026-10-05，作者拍"两个官方源 + 收起的自定义"）：从"一个地址"改成
/// **"选哪一个源"** —— 语义变了，所以升号。
/// **读 1 代老档不是兼容层**：1 代那个 `baseUrl` 就是"用户手填的地址"，
/// 它的含义在新模型里**正好是 `custom`**（`customUrl`）。不猜、不迁移、不回填。
pub const SOURCE_SCHEMA: u32 = 2;

/// 目录（catalog）在远端根里的固定名字 —— **直接模式**的约定；
/// Bootstrap 模式由 `source.json` 自己说，客户端不猜。发布侧引用同一常量
/// （见 `workbench::app::dist` 的 `NEW_CATALOG_FILE`）
pub const CATALOG_FILE: &str = "catalog.json";

/// **软件发布信息**（`release.json`）的固定名字。
///
/// ★ 它**不是预设数据**：住发布根（`presets/`）的**上一级**，不进制 catalog / manifest，
/// 也不参与发布闸的内容校验（作者 2026-10-04 定死）。它是「有没有新版本的 SupportEase」
/// 这条**软件发布链**的信息源，与"预设数据能不能读"是两件事。
pub const RELEASE_FILE: &str = "release.json";

/// Bootstrap（`source.json`）的 schema 代次。认不得就拒绝 —— 不猜新版长什么样。
/// 注意它与上面的 [`SOURCE_SCHEMA`] 是两回事：那是**这台机器上设置文件**的代次，
/// 这是**发布出去那一口文件**的协议代次
pub const BOOTSTRAP_SCHEMA: u32 = 1;

/// 构建方可注入的默认地址（`MKPSE_PRESET_SOURCE`）—— **第十七刀起语义是 Bootstrap
/// 地址**（指向 `source.json` 的文件地址），不再是"直接根"。**变量没设就再看工作台
/// 配置**（那是 build.rs 的事，到这里时已经被合并成一个值）。
const DEFAULT_BASE_URL: Option<&str> = option_env!("MKPSE_PRESET_SOURCE");

/// **第二个内置源**（Gitee）的 Bootstrap 地址（`MKPSE_PRESET_SOURCE_GITEE`）。
///
/// ★ 为什么是两个而不是"一个地址 + 用户自己改"（作者 2026-10-05）：国内直连 Gitee、
///   直连 GitHub 要靠代理 —— **两个都是官方源，用户只管选**。
///   没注入就不出现在界面里（不硬编一个地址进来：那会把"部署在哪"这件事写死在客户端）。
const GITEE_BASE_URL: Option<&str> = option_env!("MKPSE_PRESET_SOURCE_GITEE");

/// 自定义源的**两种形状**（作者 2026-10-05 真机踩出来的）。
///
/// ```text
/// 根（root）      https://host/presets/dist/            根下就有 catalog.json 与各文件
/// Bootstrap       https://host/presets/dist/source.json  由它说 catalog 与文件根在哪
/// ```
///
/// ★ **同一个"官方地址"，用户十有八九会填后者**（他们看到的就是这个 `source.json` 地址）
///   —— 而手动指定这条路原本只认前者，于是"填官方地址反而不能用内置的"（0.0.2 真机）。
///   所以：**保存时两种都探一次，认出来哪一个记在盘上**（[`CustomShape::Unset`] = 还没探过）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CustomShape {
    /// 还没探过（老档、或刚写下来还没联网）：**运行时自适应**（先根后 Bootstrap）
    #[default]
    Unset,
    /// 数据源根
    Root,
    /// `source.json` 地址
    Bootstrap,
}

/// 用户在三个值里选哪一个（盘上就存这个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceMode {
    /// 内置：Gitee（国内直连）
    Gitee,
    /// 内置：GitHub
    Github,
    /// 自定义（折叠在高级设置里的那一个）
    Custom,
}

impl SourceMode {
    /// 线上名（盘上写的就是它）。**认不出 → `None`**（不猜：宁可报"认不出"，
    /// 也不要悄悄落到某个源上——那会让用户以为自己在用 A、其实在用 B）。
    ///
    /// 顺带 **trim**（调用方不必各写一遍；前后空格是"复制地址"最常见的脏东西）。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "gitee" => Some(Self::Gitee),
            "github" => Some(Self::Github),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// 界面上给用户看的那一句（**产品文案只有这一处**）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Gitee => "Gitee（国内直连）",
            Self::Github => "GitHub",
            Self::Custom => "自定义地址",
        }
    }

    /// 这个模式下"生效的地址"从哪来：内置看构建期注入，自定义看盘上那份。
    pub fn builtin_url(self) -> Option<&'static str> {
        match self {
            Self::Gitee => GITEE_BASE_URL,
            Self::Github => DEFAULT_BASE_URL,
            Self::Custom => None,
        }
    }
}

/// 出厂默认选哪个。
///
/// ★ 0.0.3 先是 **GitHub**（Gitee 镜像还没全部就绪，默认指过去等于把用户往取不到
///   数据的地方带）；Gitee 内容确认就绪后的下一版再翻成 [`SourceMode::Gitee`]。
pub const DEFAULT_MODE: SourceMode = SourceMode::Github;

/// 两个内置源（给界面摆两个固定选项用；没注入的那个**不出现**）。
pub fn builtin_sources() -> Vec<(SourceMode, String)> {
    [
        (SourceMode::Gitee, GITEE_BASE_URL),
        (SourceMode::Github, DEFAULT_BASE_URL),
    ]
    .into_iter()
    .filter_map(|(mode, url)| url.map(|u| (mode, u.to_owned())))
    .collect()
}

/// 当前选中的远端数据源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PresetSource {
    pub source_schema: u32,
    /// 用户选了哪一个（`gitee` / `github` / `custom`）
    pub mode: SourceMode,
    /// `mode = custom` 时的那份地址（其余模式为 `null`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_url: Option<String>,
    /// 自定义那份地址是**哪种形状**（[`CustomShape::Unset`] = 还没探过 → 运行时自适应）
    #[serde(default)]
    pub custom_shape: CustomShape,
}

/// 当前这一台机器的"远端入口"（覆盖优先，见模块头）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEntry {
    /// 手动指定：**用户填的那份地址** + 它是哪种形状。
    /// 开发 / 排查通道（本地 http.server、自建镜像、另一个官方源…）。
    ///
    /// ★ `shape` 是探出来的那一种（[`CustomShape::Unset`] = 还没探过 → 运行时自适应）：
    /// **同一个地址两种读法**（根 / `source.json`）都能认，见 [`probe_custom_shape`]。
    Custom { url: String, shape: CustomShape },
    /// 构建期注入：**Bootstrap 地址** —— 指向 `source.json`，
    /// 由它说"catalog 在哪、文件下载根在哪"。正式通道
    Bootstrap { url: String },
}

/// 解析出来的"远端在哪"：两个地址都定了。**所有联网动作的唯一出发点**
/// （下载 / 检查更新 / 应用更新都从它拿地址，不各自拼 URL）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// catalog 的完整 URL（检查 / 应用更新取它）
    pub catalog_url: String,
    /// 文件下载根（`join_url(base, file.path)` 的那个 base）
    pub base_url: String,
}

/// Bootstrap（`source.json`）说的两件事 —— **发布侧**（`workbench::app::dist`）写它，
/// 客户端读它。形状小到只有这些：多说一个字都是两端要一起改的契约
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapFile {
    source_schema: u32,
    /// catalog 相对 Bootstrap 所在目录的路径（要相对、不许 `..`、不许绝对 URL）
    catalog: String,
    /// 文件下载根：省略 / `null` = 与 Bootstrap 同目录；给了必须是绝对 http(s)。
    /// **发布侧不写它**（同一份 dist 推到哪里都对），换 CDN 时才有它的用场
    #[serde(default)]
    base_url: Option<String>,
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
    // ★ **1 代老档**（2026-10-05 之前只有 `baseUrl`）：它的含义**正好**是"用户手填的
    //   地址" = 新模型里的 `custom`。这不是兼容层、也不猜：那份地址本来就是用户填的。
    if let Some(old) = read_v1(&bytes)? {
        return Ok(Some(old));
    }
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

/// 1 代档（`{sourceSchema: 1, baseUrl}`）读成 2 代的自定义源。**不是 1 代 → `None`**。
fn read_v1(bytes: &[u8]) -> Result<Option<PresetSource>, AppError> {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Ok(None); // 连 JSON 都不是 → 交给下面的 2 代解析去报错
    };
    if v.get("sourceSchema").and_then(|x| x.as_u64()) != Some(1) {
        return Ok(None);
    }
    let Some(base) = v.get("baseUrl").and_then(|x| x.as_str()) else {
        return Err(AppError::corrupted(
            "数据源设置是老格式，但里面没有地址 —— 认不出这一份要连到哪",
        ));
    };
    Ok(Some(PresetSource {
        source_schema: SOURCE_SCHEMA,
        mode: SourceMode::Custom,
        custom_url: Some(normalize_base_url(base)?),
        // ★ 老档**没探过形状** → `Unset` → 运行时自适应（先根后 Bootstrap）。
        //   0.0.2 之前用户手填 `source.json` 地址的那些档，就是这样被救回来的。
        custom_shape: CustomShape::Unset,
    }))
}

/// 记下"当前用这个源"。整份替换（写新盖旧）。
///
/// `mode = custom` 时地址**必须给且合法**（空地址在这里就拒：写进去等于制造一个
/// "配了但配成空"的第三种状态，下游要为它单独想一套分支）。
/// 内置两个模式**不许带地址**（带了说明调用方糊涂了 —— 内置地址是构建期注入的）。
pub fn save_source(
    root: &Path,
    mode: SourceMode,
    raw_custom_url: Option<&str>,
) -> Result<PresetSource, AppError> {
    save_source_with(root, mode, raw_custom_url, CustomShape::Unset)
}

/// [`save_source`] 的完整形状版：**形状已经探出来时**连它一起写（省掉一次网络）。
pub fn save_source_with(
    root: &Path,
    mode: SourceMode,
    raw_custom_url: Option<&str>,
    custom_shape: CustomShape,
) -> Result<PresetSource, AppError> {
    let custom_url = match mode {
        SourceMode::Custom => Some(normalize_base_url(raw_custom_url.unwrap_or_default())?),
        _ => {
            if raw_custom_url
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                return Err(AppError::invalid_argument(
                    "选内置源的时候不用给地址 —— 地址是程序自带的",
                ));
            }
            None
        }
    };
    let source = PresetSource {
        source_schema: SOURCE_SCHEMA,
        mode,
        custom_url,
        custom_shape: if mode == SourceMode::Custom {
            custom_shape
        } else {
            CustomShape::Unset
        },
    };
    let json = serde_json::to_vec_pretty(&source)
        .map_err(|e| AppError::internal("数据源设置序列化失败").with_detail(e.to_string()))?;
    atomic_write(&source_file(root), &json)?;
    Ok(source)
}

/// **探一个自定义地址是哪种形状**（真联网，两次机会：先根、再 `source.json`）。
///
/// ★ 顺序有讲究：**先试根** —— 根那一次不额外花请求（`{地址}/catalog.json` 本来
///   就要取）；不成再当 Bootstrap 读一次 `source.json`。
/// **两条都不通** → 如实报错，并把两条各自的失败原因都摆出来（只说"不行"等于让用户猜）。
/// **Bootstrap 形状下，这个地址可能是哪一个文件**（纯函数，判据钉它）。
///
/// ★ **用户复制来的地址通常已经带着 `source.json`**（他们从浏览器地址栏复制的就是它）
///   —— 那种情况再拼一层 `/source.json` 会拼出 `…/source.json/source.json`，永远不通
///   （2026-10-05 真机：手动指定"官方地址"反而不通，就是这一处）。
/// 所以：**原样先试**，不成再在根上补文件名。
fn bootstrap_candidates(url: &str) -> Vec<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let mut out = Vec::new();
    if trimmed.ends_with(SOURCE_FILE_NAME) {
        out.push(trimmed.to_owned());
    }
    let base = normalize_base_url(trimmed).unwrap_or_else(|_| trimmed.to_owned());
    let appended = format!("{base}/{SOURCE_FILE_NAME}");
    if !out.contains(&appended) {
        out.push(appended);
    }
    out
}

/// **取字节的注入点**（判据用假的，产品的门只用真的）。
///
/// ★ 为什么有这一层：形状判定（根还是 `source.json`）本质是"依次试几个 URL，看哪个
///   能取到东西"。把它写成"直接调 `net::get_bytes`"的话，判据就得**开一个本地端口**
///   才能跑 —— 而客户端产品不许开端口（`check:zero-network` 那道闸会报红，2026-10-05
///   真被它抓过一次）。**注入取字节的动作，判据就能在纯内存里跑完整条判定。**
pub type FetchBytes<'a> = &'a dyn Fn(&str, &str) -> Result<Vec<u8>, AppError>;

/// 真的那份取字节（产品路径唯一入口：网络只住 `net`）。
fn real_fetch(url: &str, what: &str) -> Result<Vec<u8>, AppError> {
    crate::runtime::net::get_bytes(
        url,
        &crate::runtime::net::GetPlan::new(what),
        &crate::runtime::net::noop_tick,
    )
}

/// 探一个自定义地址是哪种形状（真联网：先根、再按 [`bootstrap_candidates`] 试 Bootstrap）。
///
/// ★ 顺序有讲究：**先试根** —— 根那一次不额外花请求（`{地址}/catalog.json` 本来
///   就要取）；不成再当 Bootstrap 读。
/// **都不通** → 如实报错，并把每次尝试的原因都摆出来（只说"不行"等于让用户猜）。
pub fn probe_custom_shape(url: &str) -> Result<(CustomShape, ResolvedSource), AppError> {
    probe_custom_shape_with(url, &real_fetch)
}

/// [`probe_custom_shape`] 的注入版（判据走它，**不开端口**）。
pub fn probe_custom_shape_with(
    url: &str,
    fetch: FetchBytes<'_>,
) -> Result<(CustomShape, ResolvedSource), AppError> {
    let base = normalize_base_url(url)?;
    // ① 先当"数据源根"
    let as_root = ResolvedSource {
        catalog_url: join_url(&base, CATALOG_FILE),
        base_url: base.clone(),
    };
    if let Ok(resolved) = fetch_and_accept(fetch, &as_root) {
        return Ok((CustomShape::Root, resolved));
    }
    // ② 再当 Bootstrap（原样 / 补文件名两种）
    let mut reasons = Vec::new();
    for candidate in bootstrap_candidates(url) {
        match parse_source_json_with(fetch, &candidate).and_then(|r| fetch_and_accept(fetch, &r)) {
            Ok(resolved) => return Ok((CustomShape::Bootstrap, resolved)),
            Err(e) => reasons.push(format!("{candidate}（{e}）")),
        }
    }
    Err(AppError::invalid_argument(format!(
        "这个地址下面取不到预设目录：当作数据源根试过（{base}/{CATALOG_FILE}），\
         当作 {SOURCE_FILE_NAME} 试了 {} 次也不通 —— {}",
        reasons.len(),
        reasons.join("；")
    )))
}

/// `source.json` 的**文件**名（`RELEASE_FILE` 是 `release.json`，两者别混 —— 这个坑
/// 名字上就分不开，所以给它一个自己的常量）。
const SOURCE_FILE_NAME: &str = "source.json";

/// 取一次 catalog 并确认它是**一份可用的目录**（解析过了才算）。
fn fetch_and_accept(
    fetch: FetchBytes<'_>,
    resolved: &ResolvedSource,
) -> Result<ResolvedSource, AppError> {
    let bytes = fetch(&resolved.catalog_url, CATALOG_FILE)?;
    crate::runtime::Catalog::parse(&bytes)
        .map_err(|e| AppError::corrupted("取到的不是一份可用的预设目录").with_detail(e.message))?;
    Ok(resolved.clone())
}

/// 撤掉用户覆盖：删掉设置文件（幂等）。
///
/// 「回到内置默认」只有这一条路 —— [`save_source`] 拒绝空地址（写空 = 制造第三种状态），
/// 所以原来的出口是"用户手删文件"；设置页把那件事变成一次显式动作。
/// 删完生效什么（内置默认 / 没配）由 [`current_base_url`] 回答，不在这里替它说。
pub fn clear_source(root: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(source_file(root)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io("撤掉数据源覆盖失败").with_detail(e.to_string())),
    }
}

/// 构建期注入的默认地址（没注就是 `None`）。界面用它区分"出厂默认值"与"用户改过的"，
/// 免得读一次之后分不清当前地址是自己选的还是出厂的
pub fn builtin_default() -> Option<String> {
    DEFAULT_BASE_URL.map(str::to_owned)
}

/// **当前选的是哪一个**（没写过设置 = 出厂默认 [`DEFAULT_MODE`]）。
pub fn current_mode(root: &Path) -> Result<SourceMode, AppError> {
    Ok(load_source(root)?.map_or(DEFAULT_MODE, |s| s.mode))
}

/// 当前生效的**入口**：按选中的模式解析（内置两个走构建期注入的 Bootstrap 地址，
/// 自定义走盘上那份地址 + **记下来的形状**），内置地址一个都没注入 → 没配。
///
/// 返回 `None` 的那一路要被界面原样说出来——那是唯一诚实的答案。
pub fn current_entry(root: &Path) -> Result<Option<SourceEntry>, AppError> {
    let stored = load_source(root)?;
    let mode = stored.as_ref().map_or(DEFAULT_MODE, |s| s.mode);
    if mode == SourceMode::Custom {
        return Ok(match stored {
            Some(s) => s.custom_url.map(|url| SourceEntry::Custom {
                url,
                shape: s.custom_shape,
            }),
            // 选了自定义却没地址 = 用户删了输入框里的字。**不静默回落**到默认源：
            // 那会让界面显示"用 Gitee"、实际连的是别的地方。
            None => None,
        });
    }
    Ok(mode.builtin_url().map(|url| SourceEntry::Bootstrap {
        url: url.to_owned(),
    }))
}

#[cfg(test)]
mod two_source_tests {
    //! **双源 + 自适应形状**的判据（2026-10-05）。
    //!
    //! 重点是最后那条：0.0.2 真机里用户把**官方 `source.json` 地址**填进「手动指定」
    //! 反而不能用（手动那条只认"数据源根"），而内置官方源同一个地址是通的 ——
    //! 那不是用户错，是我们把"一个地址有两种读法"当成了两种东西。

    use super::*;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// 1 代老档（只有 `baseUrl`）读成**自定义源** —— 那份地址本来就是用户手填的
    #[test]
    fn the_v1_file_reads_as_a_custom_source() {
        let d = dir();
        atomic_write(
            &source_file(d.path()),
            br#"{"sourceSchema":1,"baseUrl":"https://example.com/presets/dist/"}"#,
        )
        .unwrap();

        let s = load_source(d.path()).unwrap().expect("老档该读得出来");
        assert_eq!(s.mode, SourceMode::Custom);
        assert_eq!(
            s.custom_url.as_deref(),
            Some("https://example.com/presets/dist")
        );
        assert_eq!(
            s.custom_shape,
            CustomShape::Unset,
            "老档没探过形状 → 运行时自适应（这才救得回填了 source.json 地址的那份）"
        );
    }

    /// 选内置源的时候不许带地址（带了说明调用方糊涂了）
    #[test]
    fn a_builtin_mode_refuses_an_address() {
        let d = dir();
        let e = save_source(d.path(), SourceMode::Github, Some("https://x.example/")).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// 选了自定义却**没地址** → 没有入口。**不许静默回落到内置源** ——
    /// 那会让界面显示"自定义"、实际连着别的地方
    #[test]
    fn a_custom_mode_without_an_address_has_no_entry() {
        let d = dir();
        save_source(d.path(), SourceMode::Custom, None).unwrap_err(); // 空地址直接拒
                                                                      // 手工造一份"有 mode、没地址"的档（模拟手改坏的文件）
        atomic_write(
            &source_file(d.path()),
            br#"{"sourceSchema":2,"mode":"custom","customShape":"root"}"#,
        )
        .unwrap();
        assert!(
            current_entry(d.path()).unwrap().is_none(),
            "没有地址就没有入口 —— 界面会据此报错，不静默用默认源"
        );
    }

    /// 模式 id 是契约：认这三个，别的**不猜**
    #[test]
    fn only_three_source_modes_are_recognized() {
        assert_eq!(SourceMode::parse("github"), Some(SourceMode::Github));
        assert_eq!(SourceMode::parse(" gitee "), Some(SourceMode::Gitee));
        assert_eq!(SourceMode::parse("custom"), Some(SourceMode::Custom));
        assert_eq!(SourceMode::parse("gitee2"), None);
        assert_eq!(SourceMode::parse(""), None);
    }

    /// ★ **同一个地址，两种读法都认**（这次真机 bug 的钉子）。
    ///
    /// 服务端只有一个合法 catalog，摆在 `/catalog.json`；`/source.json` 也在（它指向
    /// 那个 catalog）。于是：
    /// - 填**根** → 探成 `Root`；
    /// - 填 **`source.json` 地址** → 根那次 404 → 探成 `Bootstrap`。
    ///
    /// 两条都通 —— 用户填哪一种都能用，这正是"填官方地址反而不通"的修法。
    #[test]
    fn a_custom_url_is_read_as_a_root_or_as_a_source_json() {
        // 假取字节：目录里有两个文件（`/catalog.json` 与 `/source.json`），别的 404。
        // ★ 判据**不开端口**（客户端产品不许开端口，`check:zero-network` 守着那道闸）——
        //   形状判定做成"可注入的取字节"，就能在纯内存里跑完整条判定。
        let fetch = fake_fetch(&["dist/catalog.json", "dist/source.json"]);
        let root_url = "https://mirror.example/presets/dist";

        let (root_shape, resolved) = probe_custom_shape_with(root_url, &fetch).expect("当根该通");
        assert_eq!(root_shape, CustomShape::Root);
        assert!(
            resolved.catalog_url.ends_with("/catalog.json"),
            "根形状：catalog 就在根下面，不多花请求"
        );

        // ★ 用户复制来的地址**本身带着 source.json**（0.0.2 真机就是这条路不通的）
        let bootstrap_url = "https://mirror.example/presets/dist/source.json";
        let (shape, resolved) =
            probe_custom_shape_with(bootstrap_url, &fetch).expect("当 source.json 也该通");
        assert_eq!(shape, CustomShape::Bootstrap);
        assert_eq!(
            resolved.catalog_url,
            format!("{root_url}/catalog.json"),
            "Bootstrap 形状下两个地址都由 source.json 说"
        );
    }

    /// 两条都不通 → 拒绝，且**两条的原因都摆出来**（只说"不行"等于让用户猜）
    #[test]
    fn an_unusable_address_is_refused_with_both_reasons() {
        // 目录里什么都没有：根那条与 source.json 那条都要试、都要说清为什么
        let fetch = fake_fetch(&[]);
        let e = probe_custom_shape_with("https://empty.example/presets/dist/", &fetch).unwrap_err();
        let text = format!("{}{}", e.message, e.detail.unwrap_or_default());
        assert!(text.contains("catalog.json"), "说了根那条为什么：{text}");
        assert!(
            text.contains(SOURCE_FILE_NAME),
            "说了 source.json 那条为什么：{text}"
        );
    }

    /// 判据用的**假取字节**：目录里有哪几个文件就答哪几个，其余一律 404。
    fn fake_fetch(
        files: &'static [&'static str],
    ) -> impl Fn(&str, &str) -> Result<Vec<u8>, AppError> + 'static {
        move |url: &str, what: &str| {
            // ★ 只看**最后两段**（`dist/catalog.json` 这样的形状）：
            //   前缀千变万化不该影响判定，但 `…/source.json/catalog.json` 的最后两段是
            //   `source.json/catalog.json` —— 它必须 404，否则"当根试"会误判成功
            //   （那正是真机上"填 source.json 地址"不通的那条路）。
            let path = url
                .split("://")
                .nth(1)
                .and_then(|rest| rest.find('/').map(|i| &rest[i..]))
                .unwrap_or("");
            let mut tail: Vec<&str> = path.rsplit('/').filter(|s| !s.is_empty()).take(2).collect();
            tail.reverse();
            let tail = tail.join("/");
            let hit = files.iter().any(|f| tail == *f);
            match hit {
                true => Ok(match what {
                    CATALOG_FILE => br#"{"catalogSchema":1,"revision":"r1","files":[]}"#.to_vec(),
                    SOURCE_FILE_NAME => br#"{"sourceSchema":1,"catalog":"catalog.json"}"#.to_vec(),
                    other => panic!("判据不该问 {other}"),
                }),
                false => Err(AppError::io("404").with_detail(url.to_owned())),
            }
        }
    }
}

/// 把入口解析成"远端在哪"。
///
/// - 自定义（[`SourceEntry::Custom`]）：**按记下来的形状**读；形状还没探过（老档）
///   就**自适应**：先当根，根下取不到 catalog 再当 `source.json` 读
///   —— 这正是 0.0.2 那个"填官方地址反而不通"的修法；
/// - 内置（[`SourceEntry::Bootstrap`]）：**读 `source.json`**（一次很小的网络请求），
///   按它说的算 —— 部署位置变了只改它，客户端不用重发。
pub fn resolve_entry(entry: SourceEntry) -> Result<ResolvedSource, AppError> {
    match entry {
        SourceEntry::Custom { url, shape } => {
            let base = normalize_base_url(&url)?;
            match shape {
                CustomShape::Root => Ok(ResolvedSource {
                    catalog_url: join_url(&base, CATALOG_FILE),
                    base_url: base,
                }),
                CustomShape::Bootstrap => {
                    // ★ 原样先试（用户复制来的地址通常已经带着 `source.json`）
                    let mut last = None;
                    for candidate in bootstrap_candidates(&url) {
                        match parse_source_json(&candidate) {
                            Ok(r) => return Ok(r),
                            Err(e) => last = Some(e),
                        }
                    }
                    Err(last.expect("候选表不为空（至少会补一个文件名）"))
                }
                // ★ 自适应：两条都试，报错时把两次尝试的原因都摆出来
                //   （只说"不行"等于让用户猜）
                CustomShape::Unset => {
                    let as_root = ResolvedSource {
                        catalog_url: join_url(&base, CATALOG_FILE),
                        base_url: base.clone(),
                    };
                    match crate::runtime::net::get_bytes(
                        &as_root.catalog_url,
                        &crate::runtime::net::GetPlan::new(CATALOG_FILE),
                        &crate::runtime::net::noop_tick,
                    ) {
                        Ok(_) => Ok(as_root),
                        Err(root_err) => {
                            let mut reasons = Vec::new();
                            for candidate in bootstrap_candidates(&url) {
                                if let Err(e) = parse_source_json(&candidate) {
                                    reasons.push(format!("{candidate}（{e}）"));
                                }
                            }
                            Err(AppError::invalid_argument(format!(
                                "这个地址既不是数据源根（{base}/{CATALOG_FILE}：{root_err}），\
                                 也不是一个可读的 {SOURCE_FILE_NAME} —— {}",
                                reasons.join("；")
                            )))
                        }
                    }
                }
            }
        }
        SourceEntry::Bootstrap { url } => parse_source_json(&url),
    }
}

/// 取一个 Bootstrap（`source.json`）并解析成两个地址。
/// **内置与自定义的 Bootstrap 形状共用这一处**（少一处就少一处不一致）。
fn parse_source_json(url: &str) -> Result<ResolvedSource, AppError> {
    parse_source_json_with(&real_fetch, url)
}

/// [`parse_source_json`] 的注入版。
fn parse_source_json_with(fetch: FetchBytes<'_>, url: &str) -> Result<ResolvedSource, AppError> {
    let bytes = fetch(url, SOURCE_FILE_NAME)?;
    parse_bootstrap(url, &bytes)
}

/// 这台机器上的远端入口 → 两个地址；**没配就报错、并且说清去哪儿配**。
pub fn resolve_source(root: &Path) -> Result<ResolvedSource, AppError> {
    let entry = current_entry(root)?.ok_or_else(|| {
        AppError::not_implemented(
            "这个构建没有内置官方源、你也没手动指定 —— 开发 / 排查时可以到「设置 → 高级设置 → 预设数据源」指定一个",
        )
    })?;
    resolve_entry(entry)
}

/// **探一次当前这个源**：解析入口 + 真去取一次 catalog。
///
/// ★ 它是"换源之前先看一眼"的实现（作者 2026-10-05：用户容易输错地址）：
/// **取不到就整次拒绝**，而不是"先存上再说" —— 错地址留在设置里比"没配"更难查。
/// 自定义源探的是 `{地址}/catalog.json`（那一条是"直接根"语义）；内置源探的是它
/// 自己在 `source.json` 里说的那个 catalog 地址。
pub fn probe_current(root: &Path) -> Result<ResolvedSource, AppError> {
    let entry = current_entry(root)?.ok_or_else(|| {
        AppError::invalid_argument("还没有选数据源 —— 先在上面选一个官方源，或填一个地址")
    })?;
    let resolved = resolve_entry(entry)?;
    let bytes = crate::runtime::net::get_bytes(
        &resolved.catalog_url,
        &crate::runtime::net::GetPlan::new(CATALOG_FILE),
        &crate::runtime::net::noop_tick,
    )?;
    // 解析一次：能取到但不是合法目录，那也是"这个地址不对"（用户看得懂的话）
    crate::runtime::Catalog::parse(&bytes).map_err(|e| {
        AppError::corrupted("这个地址下面不是一份可用的预设目录").with_detail(e.message)
    })?;
    Ok(resolved)
}

/// 解析 `source.json`（[`BootstrapFile`]）成两个地址。**纯函数**（字节已由调用方取回）——
/// 联网那一小步在 [`resolve_entry`] 里，这里全部能单测。
///
/// ★ 它要 `url` 的唯一理由是：`baseUrl` 缺省时得从"这个文件在哪个 URL"回退出目录。
/// **在本地校验本地文件时不该走这条路**（本地路径不是 http URL）—— 那种场景用
/// [`validate_bootstrap_local`]。
pub fn parse_bootstrap(url: &str, bytes: &[u8]) -> Result<ResolvedSource, AppError> {
    /* 前一半（JSON 形状 + 代次 + catalog 相对路径合法性）与本地校验**同一处实现** ——
    不许两处各写一遍（那正是「两边各算一遍、各自看着都对」的老病根） */
    let catalog = validate_bootstrap_local(bytes)?;
    let file: BootstrapFile = serde_json::from_slice(bytes).map_err(|e| {
        AppError::corrupted("Bootstrap（source.json）解析不了").with_detail(format!("{url} / {e}"))
    })?;
    let base_url = match file.base_url.as_deref() {
        /* 显式给的：与手动填的根走同一套收拾规矩（砍尾斜杠、只认 http(s)） */
        Some(raw) => normalize_base_url(raw)?,
        /* 省略 = 与 Bootstrap 同目录 —— 发布产物推到哪里都对 */
        None => directory_of(url)?,
    };
    Ok(ResolvedSource {
        catalog_url: join_url(&base_url, &catalog),
        base_url,
    })
}

/// **本地发布产物校验**：验一份 `source.json` 的**内容**是否合法 ——
/// **不需要 URL、不解析 baseUrl、不拼地址**，因此本地文件（`presets/dist/source.json`）
/// 也能验。
///
/// 验三件事（与 [`parse_bootstrap`] 共用同一段实现，只有"base_url 从哪来"这一步不同）：
/// 1. 是合法 JSON 且能反序列化成 [`BootstrapFile`]；
/// 2. `sourceSchema` 对得上 [`BOOTSTRAP_SCHEMA`]；
/// 3. `catalog` 是合法的**相对**路径（非空、非 http(s) 绝对 URL、不含 `..`）。
///
/// 返回 trim 后的 catalog 相对路径（调用方拿它定位"它指向的那份目录文件在不在"）。
///
/// ★ 为什么单独有这个入口：发布闸 ⑬ `source/correct` 要验的是**本地交付根里那一份**，
/// 早先误用了要求 http URL 的 [`parse_bootstrap`]，于是"文件明明合法、闸却判解析不出来"
/// （`directory_of` 拒本地绝对路径）。本地校验就不该要求 URL。
pub fn validate_bootstrap_local(bytes: &[u8]) -> Result<String, AppError> {
    let file: BootstrapFile = serde_json::from_slice(bytes).map_err(|e| {
        AppError::corrupted("Bootstrap（source.json）解析不了").with_detail(e.to_string())
    })?;
    if file.source_schema != BOOTSTRAP_SCHEMA {
        return Err(AppError::corrupted(format!(
            "Bootstrap 的格式代次认不了：文件是 {}，程序认 {}",
            file.source_schema, BOOTSTRAP_SCHEMA
        )));
    }
    let catalog = file.catalog.trim();
    check_catalog_rel(catalog, &file.catalog)?;
    Ok(catalog.to_owned())
}

/// `catalog` 必须是**合法的相对路径**：非空、不是 http(s) 绝对 URL、不含 `..`。
///
/// 抽成一处：[`parse_bootstrap`] 与 [`validate_bootstrap_local`] **共用**它 ——
/// 这条口径只写一遍（`..` 会被 URL 层解释成向上爬，等于换了一份目录；绝对 URL 则
/// 把"文件根"从 Bootstrap 身边挪走，与"相对 Bootstrap 所在目录"的约定冲突）。
fn check_catalog_rel(trimmed: &str, raw: &str) -> Result<(), AppError> {
    if trimmed.is_empty()
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.contains("..")
    {
        return Err(AppError::corrupted(
            "Bootstrap 里的 catalog 路径不合法（要相对路径、不许 ..）",
        )
        .with_detail(format!("catalog = {raw:?}")));
    }
    Ok(())
}

/// `https://host/a/b/source.json` → `https://host/a/b`。
/// 只认 http(s)、scheme 后面要有 host；"https://host" 这种没有路径段的形状也拒
/// （它不是一个"文件地址"，回退不出目录）
fn directory_of(url: &str) -> Result<String, AppError> {
    let bad =
        || AppError::invalid_argument(format!("Bootstrap 地址不像一个 http(s) 文件地址：{url}"));
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(bad)?;
    if rest.is_empty() || rest.starts_with('/') {
        return Err(bad());
    }
    /* scheme 双斜杠之后的第一个 '/' 才是目录的分界（8 = "https://" 的长度） */
    let idx = url.rfind('/').filter(|&i| i >= 8).ok_or_else(bad)?;
    let dir = url[..idx].trim_end_matches('/');
    if dir.ends_with(':') {
        return Err(bad());
    }
    Ok(dir.to_owned())
}

/// 从**文件下载根**（`presets/dist` 或 `presets`）推出软件发布信息（`release.json`）的地址。
///
/// 语义（作者 2026-10-04 定死）：`release.json` 住发布根 `presets/` **之外** —— 它不是预设数据。
/// 这里从 base 往上走**恰好一级**，再拼固定文件名：`…/presets/dist` → `…/presets/release.json`；
/// 手动根 `…/presets` → `…/release.json`。**刻意不"多走几级去找"** —— 那会让部署位置一变
/// 就悄悄指错。直接根与 Bootstrap 解析出来的 base 都走这一处，只有这一个答案。
///
/// 只认 http(s) 且要有可退回的目录段：host 根（`https://host`）如实拒，不编一个地址。
pub fn release_url(base_url: &str) -> Result<String, AppError> {
    let trimmed = base_url.trim_end_matches('/');
    let bad = || {
        AppError::invalid_argument(format!(
            "数据源根不像一个可退回上一级的 http(s) 地址：{base_url}"
        ))
    };
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(bad)?;
    if rest.is_empty() || !rest.contains('/') {
        return Err(bad());
    }
    let idx = trimmed.rfind('/').filter(|&i| i >= 8).ok_or_else(bad)?;
    let parent = trimmed[..idx].trim_end_matches('/');
    if parent.ends_with(':') {
        return Err(bad());
    }
    Ok(format!("{parent}/{RELEASE_FILE}"))
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
        let saved = save_source(
            dir.path(),
            SourceMode::Custom,
            Some("https://cdn.example.com/mkp/"),
        )
        .expect("写设置不该失败");
        let loaded = load_source(dir.path()).expect("读设置不该失败");

        assert_eq!(loaded, Some(saved));
        assert_eq!(
            loaded.expect("刚写完的").custom_url.as_deref(),
            Some("https://cdn.example.com/mkp"),
            "末尾多余的斜杠在写盘前就砍掉了"
        );
    }

    /// 没配过 ≠ 坏了：文件不在就是 `None`，不报错（默认值的分支由 `current_entry` 管）
    #[test]
    fn missing_file_is_not_corrupted() {
        let dir = root();
        assert_eq!(load_source(dir.path()).expect("没配过不该报错"), None);
    }

    /// 撤覆盖 = 删文件：撤完 load 就是 `None`（回不回内置默认由 `current_entry` 管）
    #[test]
    fn clearing_removes_the_override() {
        let dir = root();
        save_source(
            dir.path(),
            SourceMode::Custom,
            Some("https://cdn.example.com/mkp"),
        )
        .expect("写设置");
        assert!(load_source(dir.path()).expect("读设置").is_some());

        clear_source(dir.path()).expect("撤覆盖不该失败");
        assert_eq!(load_source(dir.path()).expect("读设置"), None);
        assert!(!source_file(dir.path()).exists());
    }

    /// 撤一个本来就没有的覆盖 = 幂等（与"撤销使用"同一条规矩：没有不是错）
    #[test]
    fn clearing_when_nothing_was_set_is_fine() {
        let dir = root();
        clear_source(dir.path()).expect("没有覆盖时撤覆盖也不该失败");
        assert_eq!(load_source(dir.path()).expect("读设置"), None);
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

    /* ---------- 第十七刀：Bootstrap（source.json）的解析 ---------- */

    /// 最完整的一份：显式 baseUrl + 相对 catalog —— 两个地址都按它说的算
    #[test]
    fn bootstrap_gives_both_addresses() {
        let r = parse_bootstrap(
            "https://raw.githubusercontent.com/o/r/main/release/presets/source.json",
            br#"{"sourceSchema": 1, "catalog": "catalog.json", "baseUrl": "https://cdn.example.com/mkp"}"#,
        )
        .expect("好档该解析得动");
        assert_eq!(r.base_url, "https://cdn.example.com/mkp");
        assert_eq!(r.catalog_url, "https://cdn.example.com/mkp/catalog.json");
    }

    /// 省略 baseUrl = 与 Bootstrap 同目录 —— 发布产物推到哪里都对（工作台就不写它）
    #[test]
    fn bootstrap_defaults_the_base_to_its_own_directory() {
        let r = parse_bootstrap(
            "https://raw.githubusercontent.com/o/r/main/release/presets/source.json",
            br#"{"sourceSchema": 1, "catalog": "catalog.json"}"#,
        )
        .expect("缺省 baseUrl 该走「同目录」");
        assert_eq!(
            r.base_url,
            "https://raw.githubusercontent.com/o/r/main/release/presets"
        );
        assert_eq!(
            r.catalog_url,
            "https://raw.githubusercontent.com/o/r/main/release/presets/catalog.json"
        );
    }

    /// 代次认不得 / catalog 越过目录 / catalog 写成绝对 URL / baseUrl 不是 http(s) ——
    /// 一律如实拒，不猜（catalog 的 `..` 会被 URL 层解释成向上爬，等于换了一份目录）
    #[test]
    fn bootstrap_bad_shapes_are_refused() {
        let url = "https://host/a/source.json";
        for bad in [
            r#"{"sourceSchema": 99, "catalog": "catalog.json"}"#,
            r#"{"sourceSchema": 1, "catalog": ""}"#,
            r#"{"sourceSchema": 1, "catalog": "../outside.json"}"#,
            r#"{"sourceSchema": 1, "catalog": "https://elsewhere.example.com/catalog.json"}"#,
            r#"{"sourceSchema": 1, "catalog": "catalog.json", "baseUrl": "file:///tmp"}"#,
            r#"不是 JSON"#,
        ] {
            assert!(
                parse_bootstrap(url, bad.as_bytes()).is_err(),
                "{bad} 该被拒"
            );
        }
    }

    /* ---------- 本地校验（发布闸 ⑬ 用；不要求 URL） ---------- */

    /// ★★ **本地 `source.json`（无 baseUrl）必须校验得过** —— 这条直接钉住那个 bug：
    /// 以前发布闸拿本地路径喂 `parse_bootstrap`，`directory_of` 拒"非 http" ⇒ 合法文件被判"解析不出来"。
    #[test]
    fn source_correct_accepts_a_local_bootstrap_without_a_url() {
        // 与 `dist::bootstrap_json()` 产出同形：只有 sourceSchema + catalog，没有 baseUrl
        let bytes = br#"{"sourceSchema":1,"catalog":"catalog.json"}"#;
        let rel = validate_bootstrap_local(bytes).expect("本地合法档该验得过，不需要 URL");
        assert_eq!(rel, "catalog.json");
    }

    /// 本地校验**不等于放水**：坏 JSON / 代次认不出 / catalog 绝对 URL 或含 `..` 一律如实拒。
    #[test]
    fn a_broken_bootstrap_is_still_refused_locally() {
        for bad in [
            b"not json".as_slice(),
            br#"{"sourceSchema":99,"catalog":"catalog.json"}"#,
            br#"{"sourceSchema":1,"catalog":""}"#,
            br#"{"sourceSchema":1,"catalog":"../outside.json"}"#,
            br#"{"sourceSchema":1,"catalog":"https://elsewhere.example.com/catalog.json"}"#,
        ] {
            assert!(
                validate_bootstrap_local(bad).is_err(),
                "{} 该被拒",
                String::from_utf8_lossy(bad)
            );
        }
    }

    /// 本地校验与远端解析**共用同一段**：同一份字节，本地过 ⟺ 远端那半的前置也过。
    /// （只有"base_url 从哪来"不同 —— 本地不要求 URL，远端才需要。）
    #[test]
    fn local_validation_matches_what_the_remote_parser_accepts_first() {
        let bytes = br#"{"sourceSchema":1,"catalog":"catalog.json"}"#;
        assert!(validate_bootstrap_local(bytes).is_ok());
        // 同一份字节配一个远端 URL：远端解析器也过（前半段走的是同一处校验）
        assert!(parse_bootstrap("https://host/a/source.json", bytes).is_ok());
        // 坏档：两边**都**拒（口径一致）
        let broken = br#"{"sourceSchema":1,"catalog":"../x.json"}"#;
        assert!(validate_bootstrap_local(broken).is_err());
        assert!(parse_bootstrap("https://host/a/source.json", broken).is_err());
    }

    /// 目录回退的边界：host 后面的第一段才是目录；没有路径段 / 没有 host 都拒
    #[test]
    fn directory_fallback_edges() {
        assert_eq!(
            directory_of("https://host/source.json").expect("host 根"),
            "https://host"
        );
        assert_eq!(
            directory_of("https://host/a/b/source.json").expect("两层"),
            "https://host/a/b"
        );
        for bad in [
            "https://host",
            "https://",
            "source.json",
            "file:///a/source.json",
        ] {
            assert!(directory_of(bad).is_err(), "{bad} 该被拒");
        }
    }

    /// 自定义源探成**根**之后那一路不联网、目录名按约定直接拼上
    #[test]
    fn a_custom_root_joins_the_conventional_catalog_name() {
        let r = resolve_entry(SourceEntry::Custom {
            url: "https://cdn.example.com/mkp".to_owned(),
            shape: CustomShape::Root,
        })
        .expect("根形状不联网，不该失败");
        assert_eq!(r.base_url, "https://cdn.example.com/mkp");
        assert_eq!(r.catalog_url, "https://cdn.example.com/mkp/catalog.json");
    }

    /// 空地址不许写盘：它会制造"配了但等于没配"的第三种状态
    #[test]
    fn empty_base_url_is_refused_before_writing() {
        let dir = root();
        let e = save_source(dir.path(), SourceMode::Custom, Some("   ")).unwrap_err();
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

    /* ---------- 软件发布信息（release.json）的地址 ---------- */

    /// `release.json` 住发布根 `presets/` **之外**：从文件下载根往上**恰好一级**。
    /// 直接根（`…/presets/dist`）与 Bootstrap 解析出来的 base 都走这一处。
    #[test]
    fn release_url_is_one_level_above_the_file_root() {
        assert_eq!(
            release_url("https://host/release/presets/dist").expect("该推得出"),
            "https://host/release/presets/release.json"
        );
        // 手动根（只到 presets/）也是往上恰好一级
        assert_eq!(
            release_url("https://host/mkp-content").expect("该推得出"),
            "https://host/release.json"
        );
        // host 后面只有一段（没有可退回的目录）：如实拒，不编一个地址
        assert!(release_url("https://host").is_err());
    }
}
