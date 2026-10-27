//! **用户线**：用户自己的预设文件（`<appDataDir>/user/presets-local/`）。
//!
//! ```text
//! 内部（程序管，住内部根）                       用户线（用户自己要看要拷，住用户根）
//!   <catalog.path>  云端模板的本机落点             presets-local/ 本地预设（可编辑 / 可删）
//!   archive/        换下来的云端旧版本
//!   baseline/<sha>  隐藏基准快照（切片器档的版本账在用）
//! ```
//!
//! 两界**不许混**（总纲 §1③「预设 TOML 的一生」；2026-10-08 资源库改判）：
//!
//! - **云端是模板**：它落在内部区（`<catalog.path>`），列表里只是一行「云端」——
//!   「使用」直接用它，**不产生副本**；云端永远只有当前这一版，没有第二份、没有旧版账；
//! - **本地预设**是用户自己的文件，只有两条路产生：「另存为本地预设」
//!   （[`save_cloud_as_new`]，云端 → 本地）与用户自己放进这一格；**下载不再自动产生副本**；
//! - 用户什么都没改 = 什么都没发生（浏览 / 使用 / 关开不产生文件）。
//!
//! # 用户根在哪（**不碰 `~/Documents`**）
//!
//! 两个根都住 `appDataDir`：内部根是它本身，用户根是它下面的 `<appDataDir>/user/`
//! （理由见 [`crate::fsx::paths`] 的模块头 —— 一碰 Documents，macOS 就要弹「访问你的
//! 文稿文件夹」的授权框，而这个文件本来就不是用户摆进文稿里的）。
//!
//! # 用户那份与云端的关系：**血统写在文件里**
//!
//! 它从哪一份云端、哪一版拷出来的（`# based_on*` 三行）随文件走，见 [`super::lineage`]。
//! 于是"云端换版了、你这份还是基于旧版"这件事，**换一台电脑也认得出** ——
//! 不需要程序另记一份账（作者 2026-10-02 定的原则：文件本身的信息随文件走）。
//!
//! # 第九层：文件级的"能不能用"（**不复制 Preset schema**）
//!
//! 用户文件与交付预设文件的可信规则**本来就不同**（作者 2026-10-02 定的口径）：
//!
//! - 云端线要 **SHA**：盘上那份与目录登记逐字节一致才可信（见 [`super::delivery`]）；
//! - 用户线**只看文件级**：路径在 `presets-local/` 里 + 能读 + 是 UTF-8 + **TOML 语法能解析**。
//!   外部修改是**正常事**（他拿 VS Code 改一行不是异常），**不因 SHA 变化报警**。
//!
//! 这里**不判"是不是一份合法 MKP Preset"**（有哪些字段、参数类型对不对）—— 那是 Preset 语义，
//! 属 `mkpse-preset`；默认构建**不编**那个 crate（隔离纪律，那是 workbench feature 的事），
//! 所以这一层**不许**复制一份 schema 来判结构 / 参数。语义那一档留给"应用 / 编辑"这类
//! 真正要解析的入口上的真正 Preset 能力（HANDOFF §3.5 第 9 层）。
//!
//! # 读在上面，写只有三条（都是"写我自己的文件"）
//!
//! 这一层读用户目录（列出 / 认类别 / 读正文 / 读血统）；**写用户根只有三个函数**，
//! 对应三件**不同**的事，不许合并成一个"存一下"：
//!
//! ```text
//! save_cloud_as_new 另存：云端模板 → presets-local/<你起的名字>（带血统三行，撞名就拒）
//! commit_draft         另存：云端那份改出来的 → presets-local/<原名>（已修改）.toml（第 5 层）
//! save_back            写回自己：我那份打开再存 → 同一个路径，不产生第二份（第 8 层）
//! ```
//!
//! 三条共用一句话：**云端基线（按 `catalog.path` 落盘的那一份）一概不碰**。
//! 区别只在"落点是谁"与"那三行血统从哪来"：下载收尾与另存都是新的一份、血统从
//! **来源**算；写回还是同一份、血统
//! **照抄文件里原来那三行**（出处没变 —— 见 [`super::lineage::rewrite_keeping_lineage`]）。
//! 于是"改我那份 → 保存"不会产出 `（已修改）2.toml`，也不会把出处改成"基于我自己"。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::fsx::paths::LOCAL_DIR;

use super::catalog::kind::PRESET;
use super::catalog::{Catalog, CatalogFile};
use super::lineage::{self, Lineage};
use super::source_identity::SourceIdentity;
use super::source_ledger::SourceNote;

/// 读血统时最多看文件头这么多字节。
///
/// 三行血统住在**文件头注释块**里（头几行），所以够用；而用户目录是**用户自己的地盘**，
/// 他可能往里扔一个几百 MB 的文件 —— 为了读三行注释把整份读进内存不是这一层该干的事。
pub const LINEAGE_READ_LIMIT: u64 = 8 * 1024;

/// 用户自己的一份文件。**盘就是底账**（与下载区、归档区同一套规矩）：扫盘得到，不记账本 ——
/// 账本一定会和盘漂移，而这一份的主人就是用户，他随时可能在 Finder 里动它。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    /// 相对**用户根**的路径（`presets-local/A1-fast.toml`）—— 读正文时把它交回来
    pub path: String,
    /// 文件名。用户自己的文件**没有"交付身份"**：云端没有它，所以没有 SHA、不属于任何版本
    pub file_name: String,
    pub size: u64,
    /// 最后改动时刻（UTC epoch 秒）。`None` = 文件系统没给（不是 0，也不编一个）
    pub modified_unix: Option<u64>,
    /// 认得出是哪一类就给（见 [`kind_of`]）；**认不出是 `None`**，不猜
    pub kind: Option<&'static str>,
    /// 它从哪一份云端、哪一版拷出来的（文件头那三行）。`None` = 这份没有血统
    /// （手工拷的、或别的程序写出来的）—— **不是错误**
    pub lineage: Option<Lineage>,
    /// 文件头的 `# machine:` 行（原样，不归一化）。没有就是 `None`
    ///
    /// 副本按字节拷自云端产物，头里本来就带着这两行 —— **它自己就说得出它属于哪台机型
    /// 哪一版**（作者定：文件本身的信息随文件走）。「改归属」改的就是这两行。
    pub machine: Option<String>,
    /// 文件头的 `# variant:` 行（B 类产物写的是版本 id 的小写）。没有就是 `None`
    pub variant: Option<String>,
    /// 第九层的**文件级**状态（见 [`LocalState`]）。`None` = 不是预设候选
    /// （[`kind_of`] 认不出是哪一类），它没有"能不能当预设用"这一档
    pub state: Option<LocalState>,
    /// 用不了时的一句人话原因（可直接显示，比如"TOML 语法不对（第 3 行第 1 列）"）；
    /// 能用 / 不适用时是 `None`
    pub state_detail: Option<String>,
}

/// 这份用户文件**基于的云端版本**现在怎么样了。
///
/// 三档只回答一个问题：**它当初基于的那一版，和目录里现在这一版是不是同一份** ——
/// 不回答"这份用户文件好不好"（它是用户自己的文件，从来不是"坏文件"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasedOn {
    /// 基于目录里**当前**那一版（同一个源、内容也一样）
    Current,
    /// 基于**同一个源的旧版**：云端那份已经换了。**这份用户文件照常能用、能改** ——
    /// 它只是"从旧版派生"的
    Outdated,
    /// **本地这份比云端新**：同一个源，但账上那一版的发布时刻晚于当前目录。
    ///
    /// 典型来路：刚从 GitHub 10-25 的目录取的副本，而"当前云端"这份目录的发布时刻
    /// 是 10-19（同一个源回退了一版，或者切回来的那一支本来就旧）。这种时候
    /// **不给「更新」动作** —— 那是把用户往回带（作者 2026-10-27）。
    Newer,
    /// **不是同一个源**（或说不清是不是）：换平台、换分支、换目录都会落到这里。
    ///
    /// 这时 SHA 不同只说明"两边内容不一样"，**不说明谁新**：界面上给「下载」而不是
    /// 「更新」，避免诱导用户切回旧源时把本地往旧内容上合并。
    Elsewhere,
    /// 说不清：没有血统，或血统指的那一份已经不在目录里（换源 / 下线 / 改过名）
    Unknown,
}

impl BasedOn {
    /// 跨 IPC 的稳定词（与 `LocalState` 同一规矩）。**只有这一处**拼字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            BasedOn::Current => "current",
            BasedOn::Outdated => "outdated",
            BasedOn::Newer => "newer",
            BasedOn::Elsewhere => "elsewhere",
            BasedOn::Unknown => "unknown",
        }
    }
}

/// 判据要摆给详情面板看的那几格事实（界面只说事实，不自己重算）。
///
/// 一律 `Option`：不知道就是 `null`，**不编**（"血统源未知"与"云端源未知"
/// 是两件事，界面各说各的）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerdictFacts {
    /// 这份副本**建的时候**哪个源在生效（[`SourceIdentity::key`]；账上没记 ⇒ None）
    pub lineage_source: Option<String>,
    /// 血统里记的来源版本标记（文件头的 `# release_time`，可能是版本号不是时间）
    pub lineage_release: Option<String>,
    /// 账上那个源（血统源）最近一次发布目录的时刻
    pub lineage_published_at: Option<String>,
    /// 当前生效的源身份
    pub cloud_source: Option<String>,
    /// 当前这份目录的发布时刻（随包 bootstrap 目录没有）
    pub cloud_published_at: Option<String>,
}

/// 一次判定的结果：结论 + 摆事实的那几格
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub based_on: BasedOn,
    pub facts: VerdictFacts,
}

/// 判定要用的**当前事实**（一次装配、整批文件复用；别在每份文件里各读一遍账）
pub struct Judge<'a> {
    pub catalog: &'a Catalog,
    /// 当前生效的源身份（没配置源 ⇒ `None` ⇒ 一律说不清"是不是同一个源"）
    pub cloud: Option<&'a SourceIdentity>,
    /// 目录账（见过哪些源、各自发布时刻）
    pub notes: &'a [SourceNote],
}

/// 第九层：这份用户文件**文件级**上能不能被安全使用。
///
/// 三档里只有两档 —— 别在这里加"是不是一份合法 MKP Preset"那一档：语义不在客户端判
/// （见模块头；判据见 [`toml_syntax_reason`] 与 [`read_preset_text`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalState {
    /// 存在 + 路径在用户根里 + 能读 + 是 UTF-8 + TOML 语法能解析
    Ok,
    /// 读不出来：读盘失败 / 不是 UTF-8 / TOML 语法不对 / 指向用户根之外。
    /// 界面上这一档显示"文件无法读取"，**不许应用 / 编辑**（原因在 `state_detail`）
    Unreadable,
}

impl LocalState {
    /// 跨 IPC 的稳定词（serde 到界面就是这两个；前端契约里同名）
    pub fn as_str(self) -> &'static str {
        match self {
            LocalState::Ok => "ok",
            LocalState::Unreadable => "unreadable",
        }
    }
}

/// 读一份文件的头注释（最多 [`LINEAGE_READ_LIMIT`] 字节）拿血统。
///
/// 读不出来（不是文本、没权限、文件太大被截断而三行不在前面）⇒ `None`：
/// 这一层不为此报错 —— 血统是**附加信息**，缺了不影响这份文件被认成用户的预设。
pub fn lineage_of_file(path: &Path) -> Option<Lineage> {
    head_text(path).and_then(|t| lineage::parse_lineage_from_content(&t))
}

/// 读一份文件的头注释原文（最多 [`LINEAGE_READ_LIMIT`] 字节，UTF-8 读不进就 `None`）。
pub fn head_text(path: &Path) -> Option<String> {
    let bytes = read_head(path, LINEAGE_READ_LIMIT)?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// 只读前 `limit` 字节（不把整个文件读进内存）
fn read_head(path: &Path, limit: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.take(limit).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// **这份用户文件是从哪一份云端派生的、那一版现在还在不在**。
///
/// 血统里存的是 `based_on`（**`catalog.path`**，形如 `delivery/mkp/presets/A1-standard.toml`；
/// 2026-10-04 前是 `mkp/presets/…`，那种老形状**认不出就是 Unknown**，不回填）
/// 与 `based_on_sha256`（那时候云端那一版**全文**的摘要）。拿它跟目录里现在登记的那一份比：
///
/// ```text
/// 没有血统 / 目录里没有那份路径      ⇒ Unknown
/// 不是同一个源（账上没有源身份）      ⇒ Elsewhere   ← 只给「下载」，不给「更新」
/// 同一个源 + 摘要一样                ⇒ Current
/// 同一个源 + 摘要不同 + 云端更旧      ⇒ Newer       ← 只显示「云端较旧」，不给任何动作
/// 同一个源 + 摘要不同                ⇒ Outdated    ← 「有新版」，可更新
/// ```
///
/// **判定顺序不许调**（2026-10-27）：先问"是不是同一个源"，再谈内容，最后才谈谁更新。
/// 反过来（先比 SHA）就是那个已经被推翻的行为 —— 从 GitHub 切到旧一版的 Gitee，
/// 每一份派生过的文件都变成「有新版」，把用户往回带。
///
/// 只看血统里记的那一份，**不看用户文件自己的字节**：用户改过的东西必然与云端不同，
/// 拿它去比只会得出"永远不一样"这种废话。
pub fn based_on(
    judge: &Judge<'_>,
    lineage: Option<&Lineage>,
    copy_source: Option<&SourceIdentity>,
) -> Verdict {
    let facts = VerdictFacts {
        lineage_source: copy_source.map(SourceIdentity::key),
        lineage_release: lineage.and_then(|l| l.based_on_release_time.clone()),
        lineage_published_at: copy_source
            .and_then(|id| source_note(judge.notes, id))
            .and_then(|n| n.published_at.clone()),
        cloud_source: judge.cloud.map(SourceIdentity::key),
        cloud_published_at: judge.catalog.published_at.clone(),
    };
    let verdict = |based_on| Verdict {
        based_on,
        facts: facts.clone(),
    };

    let Some(lineage) = lineage else {
        return verdict(BasedOn::Unknown);
    };
    let Some(source) = lineage.based_on.as_deref() else {
        return verdict(BasedOn::Unknown);
    };
    let known = judge
        .catalog
        .files
        .iter()
        .find(|f| f.path == source)
        .or_else(|| judge.catalog.files.iter().find(|f| f.file_name == source));
    let Some(file) = known else {
        return verdict(BasedOn::Unknown);
    };

    /*
     * **先问"是不是同一个源"**（作者 2026-10-27 定的判定顺序）：
     *
     * 不同源的两份目录各自演化，SHA 不同只说明"内容不一样"，谁新谁旧根本无从谈起。
     * 从前只比 SHA，于是从 GitHub 切到旧一版的 Gitee 会把**每一份**派生过的文件
     * 都翻成「有新版」，再配一颗绿色「更新」按钮 —— 那是把用户往回带。
     *
     * 血统账上没记源身份（这个功能之前建的副本）⇒ `Elsewhere`：
     * 不猜它当初来自哪个源，也不给「更新」。
     */
    if copy_source.is_none() || judge.cloud != copy_source {
        return verdict(BasedOn::Elsewhere);
    }

    match (lineage.based_on_sha256.as_deref(), file.expected_sha()) {
        (Some(sha), Some(want)) if sha == want => verdict(BasedOn::Current),
        (Some(_), Some(_)) => {
            /* 同一个源、内容不同：这时才轮到"谁更新" —— 而且只作辅助判断 */
            if cloud_is_older(judge, copy_source) {
                verdict(BasedOn::Newer)
            } else {
                verdict(BasedOn::Outdated)
            }
        }
        /*
         * 比不出来的两种都归 `Unknown`（"缺的那项就是不知道"）：
         * 草稿没记来源摘要，或**目录没登记期望值**（随包 bootstrap 目录）——
         * 后者不许当成"过期了"，那会凭空报一批假更新。
         */
        _ => verdict(BasedOn::Unknown),
    }
}

/// 当前目录的发布时刻**早于**血统那个源最近一次发布的时刻 ⇒ 云端更旧。
///
/// ★ 只在这一处用，而且只用来在 `Outdated` 与 `Newer` 之间二选一 ——
///   **绝不**单独据此声称"有新版本"（作者 2026-10-27：`publishedAt` 只能辅助）。
///   两端缺一 ⇒ `false`（退回 `Outdated`：同为旧版的说法，不凭空升格成"你可更新"）。
fn cloud_is_older(judge: &Judge<'_>, copy_source: Option<&SourceIdentity>) -> bool {
    let cloud = judge.catalog.published_at.as_deref().and_then(moment_of);
    let mine = copy_source
        .and_then(|id| source_note(judge.notes, id))
        .and_then(|n| n.published_at.as_deref())
        .and_then(moment_of);
    match (mine, cloud) {
        (Some(mine), Some(cloud)) => cloud < mine,
        _ => false,
    }
}

fn source_note<'a>(notes: &'a [SourceNote], id: &SourceIdentity) -> Option<&'a SourceNote> {
    notes.iter().find(|n| n.identity == *id)
}

/// RFC3339 → 可比较的秒数。**不引时间库**：只认 `YYYY-MM-DDTHH:MM:SS[.fff][Z|±HH:MM]`，
/// 认不出就 `None`（判据退回"不知道"，不拿字符串做字典序比较 —— 时区不同的两份会错）。
fn moment_of(raw: &str) -> Option<i64> {
    let s = raw.trim();
    let (date, rest) = s.split_once('T')?;
    let mut d = date.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let mo: i64 = d.next()?.parse().ok()?;
    let da: i64 = d.next()?.parse().ok()?;
    if d.next().is_some() || !(1..=12).contains(&mo) || !(1..=31).contains(&da) {
        return None;
    }

    /* 时区：`Z` / 空 = UTC；`±HH:MM` 按偏移折算 */
    let (clock, offset) = match rest.strip_suffix('Z') {
        Some(clock) => (clock, 0),
        None => match rest.rfind(['+', '-']) {
            Some(at) if at > 0 => {
                let (clock, tz) = rest.split_at(at);
                let sign = if tz.starts_with('-') { -1 } else { 1 };
                let mut parts = tz[1..].split(':');
                let hh: i64 = parts.next()?.parse().ok()?;
                let mm: i64 = parts.next().unwrap_or("0").parse().ok()?;
                if parts.next().is_some() {
                    return None;
                }
                (clock, sign * (hh * 3600 + mm * 60))
            }
            _ => (rest, 0),
        },
    };

    let mut t = clock.split(':');
    let hh: i64 = t.next()?.parse().ok()?;
    let mm: i64 = t.next()?.parse().ok()?;
    let ss: i64 = t.next()?.split('.').next()?.parse().ok()?;
    if t.next().is_some() || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }

    Some(days_from_civil(y, mo, da) * 86_400 + hh * 3600 + mm * 60 + ss - offset)
}

/// 公历日期 → 自 1970-01-01 起的天数（Howard Hinnant 的 `days_from_civil`）。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 血统指的那一份云端，现在**对应哪台机型的哪个版本**（界面要说人话用）。
/// 说不清就 `None` —— 不猜。
pub fn source_of<'a>(
    catalog: &'a Catalog,
    lineage: Option<&Lineage>,
) -> Option<&'a super::catalog::CatalogFile> {
    let source = lineage?.based_on.as_deref()?;
    catalog
        .files
        .iter()
        .find(|f| f.path == source)
        .or_else(|| catalog.files.iter().find(|f| f.file_name == source))
}

/// 按扩展名认类别。**只认 `.toml`（MKP 预设）**。
///
/// 切片器那两类（`bbs_profile` / `orca_profile`）都是 `.json` ——
/// 光看扩展名分不出是哪一种，所以**照实认不出**，不替用户猜一个。
/// 认不出的那种在界面上照样列出来（任何类型档下都列），只是不声称它是哪一类。
pub fn kind_of(file_name: &str) -> Option<&'static str> {
    file_name
        .to_ascii_lowercase()
        .ends_with(".toml")
        .then_some("mkp_preset")
}

/// TOML 语法这一关（**文件级**，不是 Preset schema）：能解析 ⇒ `None`；不能 ⇒ 一句人话原因。
///
/// 用的是客户端本来就在编的 `toml_edit`（`presetdata` 读 `presets/*.toml` 同一条管线）——
/// 只回答"这是不是一段能读的 TOML"，**不回答"是不是一份 MKP 预设"**。
pub fn toml_syntax_reason(text: &str) -> Option<String> {
    let err = text.parse::<toml_edit::DocumentMut>().err()?;
    let at = err.span().and_then(|span| {
        let upto = text.get(..span.start)?;
        let line = upto.matches('\n').count() + 1;
        let col = upto
            .rsplit('\n')
            .next()
            .map_or(1, |l| l.chars().count() + 1);
        Some((line, col))
    });
    Some(match at {
        Some((line, col)) => ::postprocess::diag::messages::text(
            "ui.runtime.local.toml_syntax",
            &[("line", &line.to_string()), ("col", &col.to_string())],
        ),
        None => "TOML 语法不对".to_owned(),
    })
}

/// 一份认得出的预设候选（`.toml`）的文件级判定：能读 + UTF-8 + TOML 语法。
/// 读不出来给一句人话原因（**读盘都失败**和**语法不对**分开说）。
fn preset_file_state(path: &Path) -> (Option<LocalState>, Option<String>) {
    let Ok(bytes) = std::fs::read(path) else {
        return (
            Some(LocalState::Unreadable),
            Some("读不出来（文件不在了 / 读不动）".to_owned()),
        );
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return (
            Some(LocalState::Unreadable),
            Some("不是 UTF-8 文本，读不出来".to_owned()),
        );
    };
    match toml_syntax_reason(&text) {
        None => (Some(LocalState::Ok), None),
        Some(reason) => (Some(LocalState::Unreadable), Some(reason)),
    }
}

/// 用户自己的文件有哪些（**按路径升序** —— `read_dir` 的顺序是文件系统说的，不稳定，
/// 界面要一个每次刷新都一样的表）。
///
/// 子目录也扫（用户可能自己分文件夹放）。目录不存在 = 一份都还没有：空表。
pub fn local_files(user_root: &Path) -> Vec<LocalFile> {
    let mut out = Vec::new();
    let mut stack = vec![user_root.join(LOCAL_DIR)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue; // 目录不存在 / 读不动：当作"这一支没有东西"，不让整条读失败
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let rel = path
                .strip_prefix(user_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let kind = kind_of(&file_name);
            /*
             * 第九层：先过"路径合法"这道闸（与读正文 / 应用 / 编辑同一道 `resolve_in`）——
             * 符号链接指向用户根外面的，在这里拦下，**一个字节都不读**（血统也一样不读）。
             * 只对预设候选暴露状态：别的文件本来就没有"能不能当预设用"这一档。
             */
            let (head, state, state_detail) = match crate::fsx::paths::resolve_in(user_root, &rel) {
                Err(_) => (
                    None,
                    (kind == Some(PRESET)).then_some(LocalState::Unreadable),
                    (kind == Some(PRESET)).then(|| {
                        "它指向 presets-local/ 外面 —— 程序不读用户根外面的东西".to_owned()
                    }),
                ),
                Ok(real) => {
                    let (state, detail) = if kind == Some(PRESET) {
                        preset_file_state(&real)
                    } else {
                        (None, None)
                    };
                    (head_text(&real), state, detail)
                }
            };
            /* 血统（三行）与归属（machine / variant 两行）读的是**同一段头注释** ——
            一次读出，不许为省一次文件头 IO 把两处事实抄成两份 */
            let lineage = head
                .as_deref()
                .and_then(lineage::parse_lineage_from_content);
            let machine = head
                .as_deref()
                .and_then(lineage::parse_machine_from_content);
            let variant = head
                .as_deref()
                .and_then(lineage::parse_variant_from_content);
            out.push(LocalFile {
                path: rel,
                kind,
                lineage,
                machine,
                variant,
                state,
                state_detail,
                file_name,
                size: meta.len(),
                modified_unix: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// 用户另存出来的那一份叫什么：`<原名（不含后缀）>（已修改）<后缀>`。
///
/// 例如 `A1-fast.toml` → `A1-fast（已修改）.toml`。用全角括号：这是**给人看的名字**，
/// 用户要在 Finder 里一眼认出"这是我改过的那一份"；半角括号在文件名里太像代码。
///
/// 再存一次**还是这个名字**（覆盖它自己）—— 用户改的就是"我那份"，不该越存越多。
pub fn edited_name(source_file_name: &str) -> String {
    let path = Path::new(source_file_name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| source_file_name.to_owned());
    match path.extension() {
        Some(ext) => ::postprocess::diag::messages::text(
            "ui.runtime.local.copy_name_with_ext",
            &[("stem", &stem), ("ext", &ext.to_string_lossy())],
        ),
        None => {
            ::postprocess::diag::messages::text("ui.runtime.local.copy_name", &[("stem", &stem)])
        }
    }
}

/// 另存完成的结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// 相对**用户根**的路径（`presets-local/A1-fast（已修改）.toml`）
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 盖掉了一份同名的用户文件（第二次保存就是这种）
    pub replaced: bool,
}

/// **临时编辑的收尾：把草稿另存成用户自己的文件**（`presets-local/<原名>（已修改）<后缀>`）。
///
/// 这一刀全层的核心不变式就在这个函数里：它**只写用户根**——
/// 云端原件（按 `catalog.path` 落盘的那一份）**一概不碰**（只有云端换版本能替换它）。
/// 再存一次就是**覆盖它自己**：用户改的是"我那份"，不该越存越多。
///
/// 写下去的是**副本的形状**：`text` + 头注释块里三行血统（[`super::lineage::make_copy`]）——
/// `based_on` = 来源那份的 **`catalog.path`**（`delivery/mkp/presets/A1-standard.toml`，
/// 与工作台建副本的"哪一版"同一形状）。于是这份文件拷到哪台电脑上都说得清
/// 自己从哪来、基于哪一版（第七层：「云端换版了、你这份还是基于旧版」就靠它判）。
pub fn commit_draft(
    user_root: &Path,
    source_file_name: &str,
    based_on: &str,
    text: &str,
) -> Result<Committed, AppError> {
    let file_name = edited_name(source_file_name);
    let rel = format!("{LOCAL_DIR}/{file_name}");
    /* 名字是从目录里的文件名派生的，但仍然过一遍防穿越闸 */
    let target = crate::fsx::paths::resolve_in(user_root, &rel)?;
    let replaced = target.exists();
    let body = super::lineage::make_copy(text, based_on);
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())?;
    Ok(Committed {
        path: rel,
        file_name,
        size: body.len() as u64,
        replaced,
    })
}

/// **把编辑后的正文写回它自己**（第八层）：**同一个路径、同一份文件，不产生第二份**。
///
/// 与 [`commit_draft`]（另存）只有两处不同：
///
/// - **落点是原来那条路径**，不是派生的新名字 —— 用户改的就是"我那份"，
///   所以不会出现 `（已修改）2.toml` / `（再次修改）.toml` 这种越改越多的名字；
/// - **三行血统照抄文件里原来那三行**（出处没变，见
///   [`super::lineage::rewrite_keeping_lineage`]）—— 保存之后它仍然说得清
///   "我从哪一版云端派生"，也不会因为重算摘要变成"基于我自己改过的字节"。
///   这份本来就没有血统（手工拷的 / 别的程序写出来的）⇒ 照实不写，**不编一个出处**。
///
/// 编辑期间这份被移走 / 删掉了 ⇒ **拒绝并说清**：不去别处新建一份（那不是用户点的
/// 那个"保存"），也不假装存成了。
pub fn save_back(user_root: &Path, rel: &str, text: &str) -> Result<Committed, AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.392",
            &[("rel", rel)],
        )));
    }
    /* 血统是**这份文件自己的属性**：先读出来再照抄回去（读不出来 = 它本来就没有） */
    let lineage = lineage_of_file(&target);
    let body = super::lineage::rewrite_keeping_lineage(text, lineage.as_ref());
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())?;
    Ok(Committed {
        path: rel.to_owned(),
        file_name: target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| rel.to_owned()),
        size: body.len() as u64,
        /* 写的就是原来那一份所在的位置：当然是覆盖 */
        replaced: true,
    })
}

/// **改一份用户预设的归属**（2026-10-07 作者要的）：把文件头的 `# machine:` /
/// `# variant:` 两行换成新值 —— 复制出来的那份从此自己说得出它属于哪台机型哪一版。
///
/// 与血统同一族文本手术：**只动那两行注释，正文一个字节不动**（[`super::lineage::
/// rewrite_machine_variant`]），写盘走 [`crate::fsx::atomic::atomic_write`]。
/// 归属是**文件自己的属性** —— 它随文件走，不记在程序账上。
pub fn set_machine_variant(
    user_root: &Path,
    rel: &str,
    machine_id: &str,
    version_id: &str,
) -> Result<(), AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.393",
            &[("rel", rel)],
        )));
    }
    let text = read_text(user_root, rel)?;
    let body = super::lineage::rewrite_machine_variant(&text, machine_id, version_id);
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())
}

/// 读用户自己那份的正文（**只认 `presets-local/`**，见 [`check_local_prefix`]）。
///
/// 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来，不装成空正文。
pub fn read_text(user_root: &Path, rel: &str) -> Result<String, AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    let bytes = std::fs::read(&target).map_err(|_| {
        AppError::not_found(messages::text("ui.runtime.local.394", &[("rel", rel)]))
    })?;
    String::from_utf8(bytes)
        .map_err(|_| AppError::corrupted(messages::text("ui.runtime.local.395", &[("rel", rel)])))
}

/// 第九层：读一份用户预设，并过**文件级**检查（能读 + UTF-8 + TOML 语法）。
///
/// 应用 / 编辑两个入口都走它 —— 检查只有一处，不许各写一遍。**它不查 SHA**：
/// 用户那份本来就是允许改的，"还能不能被认成一份能读的 TOML"才是用户线的可信度问题。
/// **它也不判 Preset 结构**（那要真正的 Preset 能力，见模块头）。
pub fn read_preset_text(user_root: &Path, rel: &str) -> Result<String, AppError> {
    let text = read_text(user_root, rel)?;
    if let Some(err) = text.parse::<toml_edit::DocumentMut>().err() {
        let reason = toml_syntax_reason(&text).unwrap_or_else(|| "TOML 语法不对".to_owned());
        return Err(AppError::corrupted(messages::text("ui.runtime.local.396", &[("rel", rel), ("reason", &reason.to_string())]))
        .with_detail(format!(
            "第九层（文件级检查）：能读 + UTF-8 + TOML 语法；不判\"是不是合法 MKP Preset\"（语义）。\
             解析器原文：{err}"
        )));
    }
    Ok(text)
}

/// 把新的正文**原子写回同一份**用户文件（读-改-写的收尾）。
///
/// 与 [`save_back`] 的分工：那个走编辑器那条链（正文由用户改、血统照抄回去）；
/// 这个是"正文已经处理好了、只管落盘"的裸写 —— **校准写回**（[`calibration_of`]
/// 是它的读侧）用它，血统三行原样留在 `text` 里。门槛与读侧同一条（前缀 + 防穿越），
/// 文件被移走 / 删掉了照实拒绝，不去别处新建一份。
pub fn write_text(user_root: &Path, rel: &str, text: &str) -> Result<(), AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.397",
            &[("rel", rel)],
        )));
    }
    crate::fsx::atomic::atomic_write(&target, text.as_bytes())
}

/// 一份预设正文里那四个校准值（三轴偏移 + 涂胶限速）。
///
/// **缺谁就是 `None`** —— 不拿 0 充数（0 是一个合法的偏移值，与"读不出来"是两件事）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Calibration {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub speed: Option<f64>,
}

impl Calibration {
    /// 三轴齐全才算一份能用的偏移基准（缺一个就是"说不清"，与校准页同一条口径）。
    pub fn axes(&self) -> Option<(f64, f64, f64)> {
        Some((self.x?, self.y?, self.z?))
    }
}

/// 从一份预设 TOML 的正文里读校准值（校准页初值 / 云端换版判定都用它）。
///
/// 定位一律从**参数注册表**派生（`param_key` → `(section, toml_key)`，与
/// [`crate::presetdata::patch`] 同一张表、同一条纪律）—— 这一层不硬编码 `offset_x`
/// 这类裸键。TOML 解析不出 / 键不在 / 值不是数 ⇒ 对应的 `None`：照实说不知道，不猜。
/// 整数写法（`offset_z = 4`）照收：预设里三轴本来就允许整数与小数混着写。
pub fn calibration_of(catalog: &Catalog, text: &str) -> Calibration {
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return Calibration::default();
    };
    let number = |param_key: &str| -> Option<f64> {
        let def = catalog.params().iter().find(|p| p.key == param_key)?;
        let item = doc.get(&def.section)?.as_table_like()?.get(&def.toml_key)?;
        item.as_float()
            .or_else(|| item.as_integer().map(|i| i as f64))
    };
    Calibration {
        x: number("toolhead.offset.x"),
        y: number("toolhead.offset.y"),
        z: number("toolhead.offset.z"),
        speed: number("toolhead.speed_limit"),
    }
}

/// 只认 `presets-local/` 里的东西：读正文的入参必须是 [`local_files`] 给的那条路径起头。
///
/// 两道闸（第三道在 [`crate::fsx::paths::resolve_in`] 里，比真实路径挡符号链接）：
/// 1. **前缀**：别越到用户根下别的目录去（`exports/` `reports/` 那些不是这一层的事）；
/// 2. **相对路径的基本形状**（[`crate::fsx::paths::check_relative`]）：绝对路径、`..` 一概拒。
///
/// 两道都放在读文件**之前**：不合形状的路径不该先碰到盘。
pub fn check_local_prefix(rel: &str) -> Result<(), AppError> {
    let prefix = format!("{LOCAL_DIR}/");
    if !rel.starts_with(&prefix) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.398",
            &[("prefix", &prefix.to_string()), ("rel", rel)],
        )));
    }
    crate::fsx::paths::check_relative(rel)
}

/* ---------- 第十层：用户文件管理（重命名 / 删除）—— 只动名字或删掉，不动字节 ---------- */

/// 一份用户文件的落点与名字（第十层改名 / 第十一层另存为一份新的，两处共用这一个形状）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIdentity {
    pub path: String,
    pub file_name: String,
    /// **这个名字被程序修过了**（剔了不合规的字符 / 截到字节上限）—— 落盘的就是
    /// `file_name` 这一串，界面据此照实说一句"名字太长，已截断为 X"。
    pub adjusted: bool,
}

/// 文件名的**字节上限**（APFS / NTFS 都是 255；按 UTF-8 算，一个汉字 3 字节）。
///
/// 名字超了**不许报错** —— 报了用户就得自己数字节（作者 2026-10-20：「改名有限制……
/// 不要失败，可以自动截断显示出来被截断了」）。截到上限是唯一有答案的那一档。
pub(crate) const NAME_MAX_BYTES: usize = 255;

/// 掰成「主名 + 后缀」（后缀**含点**，保留用户写的那个大小写）。
///
/// `A1-fast.toml` ⇒ `("A1-fast", ".toml")`；没有点 / 点在开头（`..`）⇒ 整个是主名。
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    }
}

/// **新名字的门槛：能修的就修好，修不了的才拒**（2026-10-20 改判）。
///
/// 原来那些不合规一律拒绝 —— 名字长一点改名就整个失败，而在文件系统上"截到上限"
/// 才是唯一有答案的那一档（作者要的正是这一条）。所以现在分两类：
///
/// - **修好**（并把"修过了"报出去，见返回的 `bool`）：
///   带路径分隔符 / 控制字符 ⇒ **剔掉这些字符**（这一层只改名字，不搬文件夹）；
///   超过 [`NAME_MAX_BYTES`] ⇒ 按**字符边界**截主名，**后缀一个字节都不动**
///   （后缀决定它是哪一类，见 [`kind_of`]）；
/// - **仍然拒绝**（这两档没有可修的东西，修了就是另一个名字 / 另一类文件）：
///   空名字；`.` / `..`；**后缀与原来不一致**（`.toml` 还是 `.toml`）。
pub(crate) fn normalize_new_name(
    old_name: &str,
    new_name: &str,
) -> Result<(String, bool), AppError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.399",
            &[],
        )));
    }
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !(c.is_control() || *c == '/' || *c == '\\'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.400",
            &[],
        )));
    }
    if cleaned == "." || cleaned == ".." {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.401",
            &[],
        )));
    }
    let (_, want_ext) = split_ext(cleaned);
    let (_, old_ext) = split_ext(old_name);
    /* 后缀**大小写无关**地比（用户写了 `.TOML` 也是同一类），但落盘用它自己那一串 */
    if !want_ext.eq_ignore_ascii_case(old_ext) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.402",
            &[],
        )));
    }
    if cleaned.len() <= NAME_MAX_BYTES {
        return Ok((cleaned.to_owned(), cleaned != trimmed));
    }
    let (stem, ext) = split_ext(cleaned);
    let budget = NAME_MAX_BYTES.saturating_sub(ext.len());
    let mut keep = String::new();
    for ch in stem.chars() {
        if keep.len() + ch.len_utf8() > budget {
            break;
        }
        keep.push(ch);
    }
    /* 截尾可能留个空格（那会被系统当成名字的一部分，看着像没截干净） */
    let keep = keep.trim_end();
    if keep.is_empty() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.403",
            &[],
        )));
    }
    let out = format!("{keep}{ext}");
    Ok((out, true))
}

/// 两个路径是不是同一个文件。只用来放行"大小写只差一档的改名"（见 [`rename_file`]）；
/// 别的平台退化成"不是"—— 那一边这种改名会被当成重名挡下，宁可响。
#[cfg(unix)]
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(ma), Ok(mb)) => ma.dev() == mb.dev() && ma.ino() == mb.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_a: &Path, _b: &Path) -> bool {
    false
}

/// `presets-mine/` → `presets-local/`：把**上一代目录**里用户真有的文件搬进新目录。
///
/// 一代目录名换过一次（`presets-mine` → `presets-local`），旧代码只认新前缀 ⇒ 老盘上
/// 那份"我自己的预设"从此在界面里看不见。这一趟把它接回来，规则是**只搬、不动字节**：
///
/// - **只搬文件**（子目录递归，保结构）；空目录就地留着（用户自己删）；
/// - 落点已有同名文件 ⇒ **跳过并记一条 `conflict`**，绝不覆盖（那是用户的两份东西，
///   谁顶谁不是程序能定的）；指针照旧指向新目录里那一份 —— 用户能看到"有两份同名的"；
/// - 任何一步失败（建目录 / 改名）⇒ **源文件原地不动**，记一条 `failed`，其余照搬；
/// - 不建"迁移完成"标记：跑第二遍时旧文件已经不在 ⇒ 自然什么也不做（幂等），
///   判断依据永远是"盘上还有没有"，不引入第二处状态。
///
/// 返回每一条的结果，调用方（[`crate::fsx::paths::user_root`] 那条冷启路径）只用来记日志。
pub fn adopt_legacy_dir(user_root: &Path) -> Vec<LegacyAdoption> {
    let legacy = user_root.join(LEGACY_MINE_DIR);
    if !legacy.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack = vec![legacy.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let from = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    stack.push(from);
                    continue;
                }
                Ok(_) => {}
                Err(_) => continue,
            }
            let rel = from
                .strip_prefix(user_root)
                .unwrap_or(&from)
                .to_string_lossy()
                .replace('\\', "/");
            /* 相对路径的第二段起是新目录下的同一位置（保留用户自己分的文件夹） */
            let tail = match rel.strip_prefix(&format!("{LEGACY_MINE_DIR}/")) {
                Some(tail) => tail,
                None => continue,
            };
            let to = user_root.join(LOCAL_DIR).join(Path::new(tail));
            let file_name = from
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if to.exists() {
                out.push(LegacyAdoption {
                    file_name,
                    path: rel,
                    outcome: LegacyOutcome::Conflict,
                });
                continue;
            }
            let moved = to
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::rename(&from, &to));
            match moved {
                Ok(()) => out.push(LegacyAdoption {
                    file_name,
                    path: format!("{LOCAL_DIR}/{tail}"),
                    outcome: LegacyOutcome::Moved,
                }),
                Err(e) => out.push(LegacyAdoption {
                    file_name,
                    path: rel,
                    outcome: LegacyOutcome::Failed(e.to_string()),
                }),
            }
        }
    }
    out
}

/// 旧一代本地预设目录的名字。**只有迁移读它**（业务代码一律走 [`LOCAL_DIR`]）。
const LEGACY_MINE_DIR: &str = "presets-mine";

/// 一条旧目录文件的处置结果（只用来记日志，不进任何状态）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAdoption {
    pub file_name: String,
    /// 旧路径（搬成功时给的是新路径）
    pub path: String,
    pub outcome: LegacyOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyOutcome {
    /// 搬过去了（源已经不在）
    Moved,
    /// 新目录里已经有同名的 —— 没动它，也没动源
    Conflict,
    /// 没搬成，源原地不动
    Failed(String),
}

/// **重命名**一份用户文件（第十层）：只改名字，**字节一个不动** —— 内容、那三行血统、
/// TOML 都不重写（判据逐字节盯着）。改完还是同一份 Preset、还是同一类。
///
/// ★ 名字**先被修好**（[`normalize_new_name`]：剔不合规字符、超长截到上限），
/// 落盘的就是回执里那一串 —— 于是"名字太长"不会让改名整个失败（作者 2026-10-20）。
/// 回执里的 `adjusted` 就是"修过了没有"，界面照实说一句。
///
/// 使用中指针与草稿的跟改名不在这里做（那是 [`super::state`] 那两条 repoint 的事）——
/// 这一层只碰用户根里的文件。
pub fn rename_file(user_root: &Path, rel: &str, new_name: &str) -> Result<FileIdentity, AppError> {
    check_local_prefix(rel)?;
    let old = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !old.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.404",
            &[("rel", rel)],
        )));
    }
    let old_name = rel.rsplit('/').next().unwrap_or(rel);
    let (new_name, adjusted) = normalize_new_name(old_name, new_name)?;
    /* 只换名字，不换目录：`presets-local/本地/x.toml` 改完还在 `本地/` 里 */
    let dir = rel.strip_suffix(old_name).unwrap_or("presets-local/");
    let new_rel = format!("{dir}{new_name}");
    check_local_prefix(&new_rel)?;
    if new_rel == rel {
        return Ok(FileIdentity {
            path: new_rel,
            file_name: new_name,
            adjusted,
        });
    }
    let new_path = crate::fsx::paths::resolve_in(user_root, &new_rel)?;
    /*
     * 不覆盖：落点已经有东西就拒绝。**同一个文件**除外 —— 大小写只差一档的改名
     * 在大小写不敏感的文件系统（macOS 默认）上"落点已存在"指的正是它自己。
     */
    if new_path.exists() && !same_file(&old, &new_path) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.405",
            &[("new_name", &new_name.to_string())],
        )));
    }
    std::fs::rename(&old, &new_path).map_err(|e| {
        AppError::io(messages::text("ui.runtime.local.406", &[("rel", rel)]))
            .with_detail(e.to_string())
    })?;
    Ok(FileIdentity {
        path: new_rel,
        file_name: new_name,
        adjusted,
    })
}

/// **另存为一份新的**（第十一层）：把 `rel` 那一份**按字节**复制成同一格里另一份新的用户文件。
///
/// 与第八层"云端 → 本地文件"那条另存分开（那条改的是内容、要重写血统）—— 这一层是
/// **本地文件 → 本地文件**，而且**不做任何"智能"**：
///
/// - **字节复制**：内容与那三行 `# based_on*` 血统**原样带过去**，不重算
///   （来源已经是用户文件，重算会把"从哪一版云端派生"说错）；原文件一个字节不动；
/// - 新名字过[同一套门槛](normalize_new_name)（能修的修好、后缀保持原样）；
///   落点已有东西就**拒绝**（不覆盖、也**不自动改名** —— 名字由用户自己换）；
/// - **一个状态都不碰**：不改使用中指针、不迁移草稿、不建草稿、不进 archive ——
///   新文件从诞生起就是独立的一份（之后能独立编辑 / 改名 / 删除 / 应用）。
///
/// 它不读内容、不查状态（"纯文件操作"，与改名 / 删除同族）：连读不出来的那份也能复制，
/// 复制出来还是读不出来的（状态照实）。
pub fn copy_as_new(user_root: &Path, rel: &str, new_name: &str) -> Result<FileIdentity, AppError> {
    check_local_prefix(rel)?;
    let old = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !old.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.407",
            &[("rel", rel)],
        )));
    }
    let old_name = rel.rsplit('/').next().unwrap_or(rel);
    let (new_name, adjusted) = normalize_new_name(old_name, new_name)?;
    /* 只换名字，不换目录：新的一份落在原来那一格（与改名同一条规矩） */
    let dir = rel.strip_suffix(old_name).unwrap_or("presets-local/");
    let new_rel = format!("{dir}{new_name}");
    check_local_prefix(&new_rel)?;
    if new_rel == rel {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.408",
            &[],
        )));
    }
    let new_path = crate::fsx::paths::resolve_in(user_root, &new_rel)?;
    if new_path.exists() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.409",
            &[("new_name", &new_name.to_string())],
        )));
    }
    let bytes = std::fs::read(&old).map_err(|e| {
        AppError::io(messages::text("ui.runtime.local.410", &[("rel", rel)]))
            .with_detail(e.to_string())
    })?;
    /* 写盘走全仓唯一那个出口（`clippy.toml` 禁 `std::fs::write`） */
    crate::fsx::atomic::atomic_write(&new_path, &bytes)?;
    Ok(FileIdentity {
        path: new_rel,
        file_name: new_name,
        adjusted,
    })
}

/// **把交付预设原件落成我的一份**（交付预设 → 我的；2026-10-08 定，2026-10-09 改判用途）。
///
/// 用户世界里**只有一份**：云端原件只是内部数据（隐藏 baseline 的来源），
/// 他要改的、要用的永远是 `presets-local/` 里那一份。所以这条路现在被
/// [`crate::ipc::catalog::fetch_cloud_preset`] 直接用 —— 云端表「下载 / 更新」
/// 取回云端时落一份工作副本（已经在了就一个字节都不碰）。
///
/// - **名字**：传什么写什么，只过"是不是一个像样的名字"那道门槛
///   （[`normalize_new_name`]：能修的修好、后缀保持原样）；**撞名就拒** ——
///   不覆盖、也不自动改名（调用方先看盘上有没有，有就一个字节都不碰）；
/// - 写下去的是**副本的形状**：云端原文 + 头注释块里三行血统
///   （[`super::lineage::make_copy`]，`based_on` 指向来源交付文件的 `catalog.path`，
///   `based_on_sha256` 就是这一版云端的唯一身份 —— 逐参数云端更新那一套
///   就是拿它当"我上次处理到哪一版"的起点）；
/// - **一个状态都不碰**：不改使用中指针、不建草稿、不进 archive。
///
/// `file` 必须是一份 MKP 预设，`cloud_text` 的字节可信由调用方
/// （[`super::delivery::cloud_text`] 的 SHA 闸）保证。
pub fn save_cloud_as_new(
    user_root: &Path,
    file: &CatalogFile,
    cloud_text: &str,
    new_name: &str,
) -> Result<FileIdentity, AppError> {
    if file.kind != PRESET {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.411",
            &[("v1", &file.file_name.to_string())],
        )));
    }
    /* 这一路的名字一般是调用方照云端原名起的，与改名 / 复制同一套门槛（能修的修好） */
    let (name, adjusted) = normalize_new_name(&file.file_name, new_name)?;
    let rel = format!("{LOCAL_DIR}/{name}");
    let target = crate::fsx::paths::resolve_in(user_root, &rel)?;
    if target.exists() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.local.412",
            &[("name", &name.to_string())],
        )));
    }
    let body = super::lineage::make_copy(cloud_text, &file.path);
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())?;
    Ok(FileIdentity {
        path: rel,
        file_name: name,
        adjusted,
    })
}

/// **删除**一份用户文件（第十层）：**真删除** —— 没有垃圾桶，也没有归档
/// （作者 2026-10-02 定死：`archive/` 是云端版本生命周期的一部分，用户自己删自己的文件
/// 不搞第二套"用户历史"）。
///
/// 作者裁决（2026-10-06）：**正在使用 / 有草稿不再拦** —— 那两样是**状态**，不是文件；
/// 由调用方（ipc）在删之前把使用中指针撤下、草稿一并丢弃，确认框讲清这一步。
/// 文件层只管文件：前缀 + 防穿越两道闸，删完列表以磁盘为准（界面回来重读用户线）。
pub fn delete_file(user_root: &Path, rel: &str) -> Result<(), AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.413",
            &[("rel", rel)],
        )));
    }
    std::fs::remove_file(&target).map_err(|e| {
        AppError::io(messages::text("ui.runtime.local.414", &[("rel", rel)]))
            .with_detail(e.to_string())
    })?;
    Ok(())
}

/// **在文件管理器里显示**要用的落点（第十三层 · 文件外部管理的第一半）：
/// 把一份用户文件解析成盘上的绝对路径 —— 打开窗口是系统的事（`ipc::local::reveal_in_folder`
/// 交给 `tauri-plugin-opener`），这一层只管"解析得对、门槛不破"。
///
/// 与改名 / 删除**同族**（纯文件操作）：
/// - 只认「本地文件」那一格（`presets-local/` 前缀 + [`crate::fsx::paths::resolve_in`]
///   两道闸，含符号链接逃逸）；
/// - **读不出来的那份也能显示** —— 它只是一份文件，打开文件夹不吃内容；
/// - 文件不在了（外面删的）就说找不到（列表以磁盘为准，不去猜）。
pub fn reveal_target(user_root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    check_local_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(messages::text(
            "ui.runtime.local.415",
            &[("rel", rel)],
        )));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份用户文件。写盘走仓库唯一那个出口（`clippy.toml` 禁 `std::fs::write`）
    fn write(root: &Path, rel: &str, text: &str) {
        crate::fsx::atomic::atomic_write(&root.join(rel), text.as_bytes()).unwrap();
    }

    /// 一份**语法上**过得去的 TOML。第九层的文件级检查只看语法（能读 + UTF-8 + TOML），
    /// **不看 Preset 结构** —— 结构 / 参数是语义，客户端不判（模块头那一段）。
    const VALID_TOML: &str = "[toolhead]\noffset_x = 1.0\n";

    /// 造一份目录登记（SHA/大小都对得上），用来试「基于的云端那一版现在是什么样」
    fn entry_bytes(name: &str, content: &str) -> super::super::catalog::CatalogFile {
        super::super::catalog::CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            // 落点 = 发布根基准（B 类在 `dist/` 下），与 `catalog.files[].path` 同形
            path: format!("{}/{name}", super::super::catalog::PRESET_DEST_DIR),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: Some(lineage::sha256_hex(content)),
            size: Some(content.len() as u64),
        }
    }

    fn catalog_with(files: Vec<super::super::catalog::CatalogFile>) -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            files,
            ..Catalog::default()
        }
    }

    /* ---------- 新名字：能修的修好，修不了的才拒（2026-10-20） ---------- */

    #[test]
    fn a_good_new_name_goes_through_untouched() {
        let (name, adjusted) = normalize_new_name("A1.toml", "B2.toml").unwrap();
        assert_eq!(name, "B2.toml");
        assert!(!adjusted, "没动过就不许报修过");
    }

    #[test]
    fn trimming_the_edges_is_not_adjusted() {
        /* 前后的空白根本不是名字的一部分 —— 剔掉它不算"把用户的名字改了" */
        let (name, adjusted) = normalize_new_name("A1.toml", " B2.toml ").unwrap();
        assert_eq!(name, "B2.toml");
        assert!(!adjusted);
    }

    #[test]
    fn path_separators_are_stripped_not_rejected() {
        /* 作者 2026-10-20：「改名有限制…… 不要失败」 —— 这一档只有"剔掉"一个答案 */
        let (name, adjusted) = normalize_new_name("A1.toml", "本\\地/name.toml").unwrap();
        assert_eq!(name, "本地name.toml");
        assert!(adjusted);
    }

    #[test]
    fn a_too_long_name_is_cut_at_the_limit_with_its_ext_kept() {
        let long = format!("{}.toml", "a".repeat(300));
        let (name, adjusted) = normalize_new_name("A1.toml", &long).unwrap();
        assert!(adjusted, "截过了要照实报 —— 界面靠它说一句");
        assert_eq!(name.len(), NAME_MAX_BYTES, "正好卡在上限，不多不少");
        assert!(name.ends_with(".toml"), "后缀一个字节都不动");
    }

    #[test]
    fn the_limit_is_bytes_so_cutting_never_splits_a_character() {
        /* 一个汉字 3 字节：250 字节装不下 84 个字（252），只能装 83 个（249） */
        let long = format!("{}.toml", "中".repeat(100));
        let (name, adjusted) = normalize_new_name("中.toml", &long).unwrap();
        assert!(adjusted);
        assert!(name.len() <= NAME_MAX_BYTES, "不许溢出");
        assert!(name.ends_with(".toml"));
        let stem = &name[..name.len() - 5];
        assert_eq!(stem.chars().count(), 83, "按字符边界截，不许截出半个字");
    }

    #[test]
    fn an_empty_name_and_a_changed_ext_are_still_rejected() {
        /* 那两档没有可修的东西：空就是没有名字，后缀变了就是另一类文件 */
        assert!(normalize_new_name("A1.toml", "   ").is_err());
        assert!(normalize_new_name("A1.toml", "///").is_err());
        assert!(normalize_new_name("A1.toml", "..").is_err());
        assert!(normalize_new_name("A1.toml", "A1.json").is_err());
    }

    /* ---------- 旧目录接管（presets-mine → presets-local） ---------- */

    /// 旧目录里的文件搬进新目录，**字节一个不动**，而且**跑第二遍什么也不做**（幂等）。
    #[test]
    fn adopting_the_legacy_dir_moves_the_bytes_and_is_idempotent() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-mine/A1.toml", VALID_TOML);
        write(root.path(), "presets-mine/子目录/B2.toml", VALID_TOML);

        let done = adopt_legacy_dir(root.path());
        assert_eq!(done.len(), 2);
        assert!(done.iter().all(|o| o.outcome == LegacyOutcome::Moved));
        assert_eq!(
            std::fs::read(root.path().join("presets-local/A1.toml")).unwrap(),
            VALID_TOML.as_bytes(),
            "字节照搬"
        );
        assert!(
            root.path().join("presets-local/子目录/B2.toml").is_file(),
            "子目录结构保住"
        );
        assert!(
            !root.path().join("presets-mine/A1.toml").exists(),
            "源已经不在"
        );

        /* 第二遍：旧目录里已经没文件了 ⇒ 什么都不做（没有"迁移完成"标记这类第二处状态） */
        assert!(adopt_legacy_dir(root.path()).is_empty());
    }

    /// 新目录里已经有同名的那份 ⇒ **两份都留着**，只记一条 `Conflict`，绝不覆盖。
    #[test]
    fn adopting_never_overwrites_an_existing_file() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-mine/A1.toml", "-旧-");
        write(root.path(), "presets-local/A1.toml", "-新-");

        let done = adopt_legacy_dir(root.path());
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].outcome, LegacyOutcome::Conflict);
        assert_eq!(
            std::fs::read_to_string(root.path().join("presets-local/A1.toml")).unwrap(),
            "-新-",
            "新目录那份不许被动"
        );
        assert!(
            root.path().join("presets-mine/A1.toml").is_file(),
            "旧的两份都留着，等用户自己处理"
        );
    }

    /// 旧目录本来就不在 ⇒ 什么也不做（不报错、不建目录）
    #[test]
    fn adopting_a_missing_legacy_dir_does_nothing() {
        let root = tempfile::tempdir().unwrap();
        assert!(adopt_legacy_dir(root.path()).is_empty());
        assert!(!root.path().join("presets-mine").exists());
    }

    /// 一份都没有 = 空表，不是错误（今天真机上就是这个状态：产生用户文件的是下一层）
    #[test]
    fn local_files_is_empty_when_the_user_has_nothing() {
        let root = tempfile::tempdir().unwrap();
        assert!(local_files(root.path()).is_empty());
    }

    /// 盘当底账：用户放进去了什么就列什么。路径**相对用户根**（`presets-local/…`），
    /// 大小与时刻是真值；子目录里的也算。**第九层**：`.toml` 带上文件级状态，
    /// 认不出的 `.json` 没有这一档（`None`，它本来就不是预设候选）
    #[test]
    fn local_files_lists_what_the_user_put_there() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1-fast.toml", VALID_TOML);
        write(root.path(), "presets-local/另存.toml", VALID_TOML);
        write(root.path(), "presets-local/切片器.json", "{}");
        write(root.path(), "exports/不该看见.json", "{}");

        let got = local_files(root.path());
        let paths: Vec<&str> = got.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "presets-local/A1-fast.toml",
                "presets-local/切片器.json",
                "presets-local/另存.toml",
            ],
            "按路径升序、只出 presets-local 里的、子目录也出"
        );
        assert_eq!(got[0].file_name, "A1-fast.toml");
        assert_eq!(got[0].size, VALID_TOML.len() as u64);
        assert!(got[0].modified_unix.is_some(), "时刻要带上");
        assert_eq!(got[0].kind, Some("mkp_preset"), ".toml 认得出来是 MKP 预设");
        assert_eq!(got[0].state, Some(LocalState::Ok), "能读 + TOML 语法过");
        assert_eq!(got[0].state_detail, None);
        assert_eq!(got[1].kind, None, ".json 认不出是哪一类");
        assert_eq!(got[1].state, None, "认不出的没有\"能不能当预设用\"这一档");
    }

    /// **认不出就不认**：`.json` 可能是 bbs 也可能是 orca，光看扩展名分不出
    #[test]
    fn kind_is_only_claimed_when_the_extension_says_so() {
        assert_eq!(kind_of("A1.toml"), Some("mkp_preset"));
        assert_eq!(kind_of("A1.TOML"), Some("mkp_preset"), "扩展名大小写无所谓");
        assert_eq!(
            kind_of("Process.json"),
            None,
            "bbs 与 orca 都是 .json，分不出"
        );
        assert_eq!(kind_of("说明.md"), None);
        assert_eq!(kind_of("没有扩展名"), None);
    }

    /// 另存出来的名字：原名 + `（已修改）`，后缀留在最后
    #[test]
    fn edited_name_keeps_the_stem_and_marks_it() {
        assert_eq!(edited_name("A1-fast.toml"), "A1-fast（已修改）.toml");
        assert_eq!(edited_name("A1 涂胶.toml"), "A1 涂胶（已修改）.toml");
        assert_eq!(edited_name("没有后缀"), "没有后缀（已修改）");
        assert_eq!(
            edited_name("a.b.toml"),
            "a.b（已修改）.toml",
            "只剥最后那一段后缀"
        );
    }

    /// **这一层的核心不变式**：另存只写用户根 —— 云端原件（`mkp/`）字节不变、
    /// 下载区里不会多出文件（临时文件不住那儿）。再存一次是覆盖它自己。
    ///
    /// 写下去的是**副本的形状**：正文 + 头注释里三行血统（剪掉三行必须与正文逐字节相同）。
    #[test]
    fn commit_writes_the_edited_copy_and_leaves_the_official_alone() {
        let user = tempfile::tempdir().unwrap();
        let cloud = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(cloud.path().join("mkp/presets")).unwrap();
        crate::fsx::atomic::atomic_write(
            &cloud.path().join("mkp/presets/A1-fast.toml"),
            "涂胶宽度 = 1.0".as_bytes(),
        )
        .unwrap();
        let before = std::fs::read(cloud.path().join("mkp/presets/A1-fast.toml")).unwrap();
        let files_before = std::fs::read_dir(cloud.path().join("mkp/presets"))
            .unwrap()
            .count();

        let label = "mkp/presets/A1-fast.toml";
        let done = commit_draft(user.path(), "A1-fast.toml", label, "涂胶宽度 = 1.4").unwrap();
        assert_eq!(done.path, "presets-local/A1-fast（已修改）.toml");
        assert!(!done.replaced, "第一次另存没有盖掉谁");
        let saved = std::fs::read_to_string(user.path().join(&done.path)).unwrap();
        assert_eq!(
            crate::runtime::lineage::strip_lineage_for_compare(&saved),
            "涂胶宽度 = 1.4",
            "剪掉血统三行就是用户改过的正文"
        );
        let got = crate::runtime::lineage::parse_lineage_from_content(&saved).expect("该带上血统");
        assert_eq!(got.based_on.as_deref(), Some(label), "说得出是从哪一份拷的");

        assert_eq!(
            std::fs::read(cloud.path().join("mkp/presets/A1-fast.toml")).unwrap(),
            before,
            "云端原件一个字节都没动"
        );
        assert_eq!(
            std::fs::read_dir(cloud.path().join("mkp/presets"))
                .unwrap()
                .count(),
            files_before,
            "下载区里不会多出东西：临时文件不住 mkp/"
        );

        /* 再存一次：还是同一个名字，盖掉它自己（血统也还是三行，不叠加） */
        let again = commit_draft(user.path(), "A1-fast.toml", label, "涂胶宽度 = 1.5").unwrap();
        assert!(again.replaced, "第二次是覆盖");
        assert_eq!(local_files(user.path()).len(), 1, "不会越存越多");
        let saved = std::fs::read_to_string(user.path().join(&again.path)).unwrap();
        assert_eq!(
            crate::runtime::lineage::strip_lineage_for_compare(&saved),
            "涂胶宽度 = 1.5"
        );
        assert_eq!(saved.matches("# based_on:").count(), 1, "血统不叠加");
    }

    /// 另存出来的那份，接着就能被用户线列出来、血统也读得回来（一条链的收尾连上了）
    #[test]
    fn the_committed_copy_shows_up_in_local_files() {
        let user = tempfile::tempdir().unwrap();
        let label = "delivery/mkp/presets/A1-standard.toml";
        let done = commit_draft(user.path(), "A1 涂胶.toml", label, VALID_TOML.trim_end()).unwrap();

        let listed = local_files(user.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].path, done.path);
        assert_eq!(listed[0].kind, Some("mkp_preset"), ".toml 认得出");
        assert_eq!(
            listed[0].state,
            Some(LocalState::Ok),
            "另存出来的那份：第九层的文件级检查过（能读 + TOML 语法）"
        );
        assert_eq!(
            listed[0]
                .lineage
                .as_ref()
                .and_then(|l| l.based_on.as_deref()),
            Some(label),
            "列出来的时候就把血统读出来了"
        );
        assert!(check_local_prefix(&done.path).is_ok(), "落点在那一格里");
    }

    /* ---------- 写回自己（第八层：改我那份 → 保存） ---------- */

    /// **这一层的核心不变式**：改我那份 → 保存 = **写回同一份** ——
    /// 不产生 `（已修改）2.toml`，血统还是当初那三行（出处没变）
    #[test]
    fn saving_back_writes_the_same_file_and_keeps_the_lineage() {
        let user = tempfile::tempdir().unwrap();
        let label = "delivery/mkp/presets/A1-standard.toml";
        let done = commit_draft(user.path(), "A1-standard.toml", label, "涂胶宽度 = 1.0").unwrap();
        let lineage = lineage_of_file(&user.path().join(&done.path)).expect("副本该有血统");
        let edited = "涂胶宽度 = 1.4";

        let back = save_back(user.path(), &done.path, edited).unwrap();

        assert_eq!(back.path, done.path, "还是同一条路径，没有第二份");
        assert_eq!(back.file_name, done.file_name);
        assert!(back.replaced, "写的就是原来那一份所在的位置");
        assert_eq!(local_files(user.path()).len(), 1, "不会越存越多");

        let after = std::fs::read_to_string(user.path().join(&back.path)).unwrap();
        assert_eq!(
            lineage::strip_lineage_for_compare(&after),
            edited,
            "正文就是改过的那份"
        );
        assert_eq!(
            lineage_of_file(&user.path().join(&back.path)),
            Some(lineage),
            "出处那三行一个字都没变 —— 不会变成\"基于我自己改过的字节\""
        );
    }

    /// **什么都没改就什么都没发生**：打开又保存，文件逐字节不变
    #[test]
    fn saving_back_an_untouched_file_changes_nothing() {
        let user = tempfile::tempdir().unwrap();
        let done = commit_draft(
            user.path(),
            "A1-standard.toml",
            "delivery/mkp/presets/A1-standard.toml",
            "涂胶宽度 = 1.0",
        )
        .unwrap();
        let before = std::fs::read(user.path().join(&done.path)).unwrap();
        let body = lineage::strip_lineage_for_compare(&String::from_utf8_lossy(&before));

        save_back(user.path(), &done.path, &body).unwrap();

        assert_eq!(
            std::fs::read(user.path().join(&done.path)).unwrap(),
            before,
            "打开又保存：逐字节不变"
        );
    }

    /// 本来就没有血统的那份（手工拷的 / 别的程序写的）写回时**不编一个出处**
    #[test]
    fn saving_back_a_file_without_lineage_invents_none() {
        let user = tempfile::tempdir().unwrap();
        write(user.path(), "presets-local/本地.toml", "涂胶宽度 = 1.0");

        let back = save_back(user.path(), "presets-local/本地.toml", "涂胶宽度 = 1.2").unwrap();

        let after = std::fs::read_to_string(user.path().join(&back.path)).unwrap();
        assert_eq!(after, "涂胶宽度 = 1.2", "没有血统就不写血统");
        assert_eq!(lineage_of_file(&user.path().join(&back.path)), None);
    }

    /// 编辑期间这份被移走 / 删掉了 ⇒ **拒绝并说清**，不去别处新建一份
    #[test]
    fn saving_back_refuses_when_the_file_is_gone() {
        let user = tempfile::tempdir().unwrap();
        let e = save_back(user.path(), "presets-local/已经不在了.toml", "涂胶 = 1").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
        assert!(
            !user.path().join("presets-local/已经不在了.toml").exists(),
            "没有顺手建一份出来"
        );
    }

    /// 写回也只认用户自己那一格：`exports/`、`../` 一概不碰
    #[test]
    fn saving_back_stays_in_the_mine_dir() {
        let user = tempfile::tempdir().unwrap();
        assert!(save_back(user.path(), "exports/x.toml", "涂胶 = 1").is_err());
        assert!(save_back(user.path(), "../x.toml", "涂胶 = 1").is_err());
    }

    /* ---------- 基于云端哪一版（第七层：云端换版了没有） ---------- */

    /// 目录里登记着来源那一份，摘要一样 ⇒ 基于当前版；云端换了版 ⇒ 旧版
    #[test]
    fn the_official_update_shows_up_as_based_on_an_old_version() {
        let v1 = entry_bytes("A1-standard.toml", "云端第一版");
        let mut v2 = entry_bytes("A1-standard.toml", "云端第二版");
        let based_on_v1 = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("云端第一版")),
        };
        let gh = github();
        let notes: Vec<SourceNote> = Vec::new();

        /* 目录里还是 v1：基于当前版 */
        let c1 = catalog_with(vec![v1.clone()]);
        assert_eq!(
            based_on(
                &judge(&c1, Some(&gh), &notes),
                Some(&based_on_v1),
                Some(&gh)
            )
            .based_on,
            BasedOn::Current
        );

        /* 云端换到 v2：同一份用户文件变成「基于旧版」—— 而它不是坏文件 */
        v2.path = v1.path.clone();
        let c2 = catalog_with(vec![v2.clone()]);
        assert_eq!(
            based_on(
                &judge(&c2, Some(&gh), &notes),
                Some(&based_on_v1),
                Some(&gh)
            )
            .based_on,
            BasedOn::Outdated,
            "云端换版了，用户那份还是基于旧版"
        );
        assert_eq!(
            source_of(&c2, Some(&based_on_v1)).map(|f| f.machine_id.as_str()),
            Some("A1"),
            "界面要说得出它是哪台机型哪一版改出来的"
        );
    }

    /// 说不清那几档：没有血统 / 血统里没摘要 / 来源已经不在目录里（换源、下线）
    #[test]
    fn unknown_when_the_source_cannot_be_resolved() {
        let catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "云端")]);
        let gh = github();
        let notes: Vec<SourceNote> = Vec::new();
        let j = judge(&catalog, Some(&gh), &notes);

        assert_eq!(
            based_on(&j, None, Some(&gh)).based_on,
            BasedOn::Unknown,
            "没有血统"
        );

        let no_sha = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: None,
        };
        assert_eq!(
            based_on(&j, Some(&no_sha), Some(&gh)).based_on,
            BasedOn::Unknown,
            "记了来源没记摘要 —— 比不了就是比不了"
        );

        let gone = Lineage {
            based_on: Some("delivery/mkp/presets/A1-gone.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("云端")),
        };
        assert_eq!(
            based_on(&j, Some(&gone), Some(&gh)).based_on,
            BasedOn::Unknown,
            "来源不在目录里（换源或下线）"
        );
        assert!(source_of(&catalog, Some(&gone)).is_none());

        /* 随包 bootstrap 目录没登记期望值 ⇒ 拼不出摘要 ⇒ 也是 Unknown（不许报假更新） */
        let mut bare = entry_bytes("A1-standard.toml", "云端");
        bare.sha256 = None;
        let catalog = catalog_with(vec![bare]);
        let j = judge(&catalog, Some(&gh), &notes);
        let sha = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("云端")),
        };
        assert_eq!(
            based_on(&j, Some(&sha), Some(&gh)).based_on,
            BasedOn::Unknown,
            "目录没登记期望值 —— 不许当成过期"
        );
    }

    /// **换源之后不许再报「有新版」**（作者 2026-10-27 的核心诉求）。
    ///
    /// 同一份血统、同一份内容，只把"当前源"从 GitHub 换成 Gitee：
    /// 结论必须是 [`BasedOn::Elsewhere`]（界面上给「下载」），而不是 `Outdated`
    /// —— 否则从 10-25 的 GitHub 切到 10-19 的 Gitee，全表都成了"有新版"，一点
    /// 就把用户往旧内容上带。
    #[test]
    fn another_source_is_never_reported_as_a_newer_cloud() {
        let catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "云端第二版")]);
        let gh = github();
        let gt = gitee();
        let notes = vec![
            SourceNote {
                identity: gh.clone(),
                published_at: Some("2026-10-25T12:00:00Z".to_owned()),
                revision: "rev-gh".to_owned(),
                seen_at: 0,
            },
            SourceNote {
                identity: gt.clone(),
                published_at: Some("2026-10-19T12:00:00Z".to_owned()),
                revision: "rev-gt".to_owned(),
                seen_at: 0,
            },
        ];
        let mined_from_gh = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("云端第一版")),
        };

        let on_gh = based_on(
            &judge(&catalog, Some(&gh), &notes),
            Some(&mined_from_gh),
            Some(&gh),
        );
        assert_eq!(on_gh.based_on, BasedOn::Outdated, "同一个源、内容变了");

        let on_gt = based_on(
            &judge(&catalog, Some(&gt), &notes),
            Some(&mined_from_gh),
            Some(&gh),
        );
        assert_eq!(on_gt.based_on, BasedOn::Elsewhere, "换了源就不谈新旧");
        assert_eq!(
            on_gt.facts.cloud_source.as_deref(),
            Some("gitee#o/r@main"),
            "详情要说得清当前云端是哪个源"
        );
        assert_eq!(
            on_gt.facts.lineage_source.as_deref(),
            Some("github#o/r@main"),
            "以及这一份当初来自哪个源"
        );
        assert_eq!(
            on_gt.facts.cloud_published_at, None,
            "这份目录没记发布时刻 —— 照实说不知道"
        );
    }

    /// 老副本（这个功能之前建的，账上没记源身份）⇒ `Elsewhere`：不猜它来自哪个源
    #[test]
    fn a_copy_without_a_recorded_source_is_elsewhere() {
        let catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "云端")]);
        let gh = github();
        let notes: Vec<SourceNote> = Vec::new();
        let lineage = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("云端")),
        };
        assert_eq!(
            based_on(&judge(&catalog, Some(&gh), &notes), Some(&lineage), None).based_on,
            BasedOn::Elsewhere,
            "源身份是未知的那一类，不许拿 SHA 相同就判 Current"
        );
        /* 当前源都没配 ⇒ 同样说不清是不是同一个源 */
        assert_eq!(
            based_on(&judge(&catalog, None, &notes), Some(&lineage), Some(&gh)).based_on,
            BasedOn::Elsewhere
        );
    }

    /// 同一个源、内容不同、且**当前目录比那一版更早** ⇒ `Newer`（云端较旧，不给更新）
    #[test]
    fn a_cloud_older_than_the_copy_is_reported_as_newer() {
        let gh = github();
        let mut catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "云端第二版")]);
        catalog.published_at = Some("2026-10-19T12:00:00Z".to_owned());
        let notes = vec![SourceNote {
            identity: gh.clone(),
            published_at: Some("2026-10-25T12:00:00Z".to_owned()),
            revision: "rev-gh".to_owned(),
            seen_at: 0,
        }];
        let lineage = Lineage {
            based_on: Some("delivery/mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: Some("2026-10-25T12:00:00Z".to_owned()),
            based_on_sha256: Some(lineage::sha256_hex("云端第一版")),
        };
        let v = based_on(
            &judge(&catalog, Some(&gh), &notes),
            Some(&lineage),
            Some(&gh),
        );
        assert_eq!(v.based_on, BasedOn::Newer, "云端那份更早 ⇒ 别给更新按钮");
        assert_eq!(
            v.facts.lineage_published_at.as_deref(),
            Some("2026-10-25T12:00:00Z")
        );

        /* 两边有个先后反过来 ⇒ 照旧是「有新版」 */
        catalog.published_at = Some("2026-10-27T12:00:00Z".to_owned());
        assert_eq!(
            based_on(
                &judge(&catalog, Some(&gh), &notes),
                Some(&lineage),
                Some(&gh)
            )
            .based_on,
            BasedOn::Outdated
        );

        /* 缺一端的发布时刻 ⇒ 不许升格成 Newer（发布时间只作辅助判断） */
        catalog.published_at = None;
        assert_eq!(
            based_on(
                &judge(&catalog, Some(&gh), &notes),
                Some(&lineage),
                Some(&gh)
            )
            .based_on,
            BasedOn::Outdated,
            "没有发布时刻就谈不上新旧"
        );
    }

    fn github() -> SourceIdentity {
        crate::runtime::source_identity::parse_identity("https://github.com/o/r/tree/main").unwrap()
    }

    fn gitee() -> SourceIdentity {
        crate::runtime::source_identity::parse_identity("https://gitee.com/o/r/tree/main").unwrap()
    }

    fn judge<'a>(
        catalog: &'a Catalog,
        cloud: Option<&'a SourceIdentity>,
        notes: &'a [SourceNote],
    ) -> Judge<'a> {
        Judge {
            catalog,
            cloud,
            notes,
        }
    }

    /// ★ **老形状的 `based_on` 认不出就是认不出**（2026-10-04 作者裁决：**不引兼容层**）。
    ///
    /// 两次改名（2026-10-04 发布根基准：`mkp/…` → `dist/…`；2026-10-05 寻址改造：
    /// `dist/…` → `delivery/…`）都不改写老血统。老用户文件里那三行记的仍是旧值 ——
    /// **不为它写改写规则**（不按文件名回填、不做前缀映射）：认不出就落 `Unknown`
    /// （"说不出新旧"），绝不能猜出一份云端然后声称它是最新版或过期版。
    ///
    /// 老文件一个字节都不动，照旧能读、能改、能应用 —— 掉的只是「基于哪一版」那句判断。
    /// 这条判据钉的就是"不伪造"：下面这份血统的摘要**与新目录里那一份完全相同**，
    /// 只因路径形状是旧的，答案也必须是 `Unknown`。
    #[test]
    fn an_old_shaped_based_on_is_unknown_and_never_guessed() {
        let catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "云端")]);
        let old = Lineage {
            based_on: Some("mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            // 摘要与目录里那一份**逐字节相同** —— 所以唯一能让答案变 Unknown 的
            // 就是"路径形状对不上"，而不是"字节变了"
            based_on_sha256: Some(lineage::sha256_hex("云端")),
        };
        let gh = github();
        let notes: Vec<SourceNote> = Vec::new();
        let judge = judge(&catalog, Some(&gh), &notes);
        assert_eq!(
            based_on(&judge, Some(&old), Some(&gh)).based_on,
            BasedOn::Unknown,
            "老形状的 based_on 不许被猜成 Current / Outdated"
        );
        // 2026-10-05 的第二次改名同理：`dist/…` 形状也落 Unknown（不迁移、不猜）
        let dist_shaped = Lineage {
            based_on: Some("dist/mkp/presets/A1-standard.toml".to_owned()),
            ..old.clone()
        };
        assert_eq!(
            based_on(&judge, Some(&dist_shaped), Some(&gh)).based_on,
            BasedOn::Unknown,
            "上一代形状（dist/…）同样不猜"
        );
        assert!(
            source_of(&catalog, Some(&old)).is_none(),
            "认不出就是认不出 —— 不回填一份云端给它"
        );
    }

    /// 读血统只看文件头那一小段：一个几百 MB 的文件不许为了让界面读三行注释就整份读进来
    #[test]
    fn lineage_is_read_from_the_head_only() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("big.toml");
        let mut text = String::from("# based_on: mkp/presets/A1-standard.toml\n");
        text.push_str(&"x".repeat((LINEAGE_READ_LIMIT as usize) * 2));
        crate::fsx::atomic::atomic_write(&path, text.as_bytes()).unwrap();

        assert!(
            lineage_of_file(&path).is_some(),
            "三行在文件头，头 8 KB 里就读得到"
        );
    }

    /// 不是文本 / 读不出来 ⇒ 没有血统（**不是错误**：血统是附加信息）
    #[test]
    fn an_unreadable_head_is_just_no_lineage() {
        let root = tempfile::tempdir().unwrap();
        assert!(lineage_of_file(&root.path().join("根本没有这个文件")).is_none());
    }

    /// 读正文只认用户自己那一格：别的地方（`exports/`、`../`）一概不碰
    #[test]
    fn only_the_mine_subdir_is_readable() {
        assert!(check_local_prefix("presets-local/A1.toml").is_ok());
        assert!(check_local_prefix("exports/report.md").is_err());
        assert!(
            check_local_prefix("presets-local").is_err(),
            "目录本身不是一份文件"
        );
        assert!(check_local_prefix("../secret.toml").is_err());
        assert!(check_local_prefix("presets-local/../../secret.toml").is_err());
    }

    /* ---------- 第九层：文件级检查（能读 + UTF-8 + TOML 语法；语义不在这里判） ---------- */

    /// 语法坏的那一份：**列出来、照实说读不出来**（不藏起来，也不画成正常），
    /// 原因要指得出是哪一行 —— 用户照着去修
    #[test]
    fn a_broken_toml_shows_up_as_unreadable() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/好的.toml", VALID_TOML);
        write(
            root.path(),
            "presets-local/坏的.toml",
            "[toolhead]\noffset_x = (1",
        );

        let got = local_files(root.path());
        let good = got.iter().find(|f| f.file_name == "好的.toml").unwrap();
        assert_eq!(good.state, Some(LocalState::Ok));
        let broken = got.iter().find(|f| f.file_name == "坏的.toml").unwrap();
        assert_eq!(broken.state, Some(LocalState::Unreadable));
        let why = broken.state_detail.as_deref().unwrap_or_default();
        assert!(why.contains("第 2 行"), "原因要说得出在哪一行：{why}");
    }

    /// 非 UTF-8 的那一份：也是"读不出来"（不是"没血统"那种附加信息的缺 —— 这份是用不了）
    #[test]
    fn a_non_utf8_file_is_unreadable_too() {
        let root = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(
            &root.path().join("presets-local/二进制的.toml"),
            &[0xff, 0xfe, 0x00, 0x01],
        )
        .unwrap();

        let got = local_files(root.path());
        assert_eq!(got[0].state, Some(LocalState::Unreadable));
        assert!(got[0]
            .state_detail
            .as_deref()
            .unwrap_or_default()
            .contains("UTF-8"));
    }

    /// 符号链接**指向用户根外面**：认得出名字，但**一个字节都不读**（血统也不读），
    /// 状态是"读不出来"并说清为什么；根**内**的链接不算逃逸，照常能用
    #[cfg(unix)]
    #[test]
    fn an_escaping_symlink_is_never_read() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(
            outside.path(),
            "别人的.toml",
            "# based_on: delivery/mkp/presets/别人的.toml\n[toolhead]\noffset_x = 1.0\n",
        );
        std::fs::create_dir_all(root.path().join("presets-local")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("别人的.toml"),
            root.path().join("presets-local/跑出去.toml"),
        )
        .unwrap();
        write(root.path(), "presets-local/真的在的.toml", VALID_TOML);
        std::os::unix::fs::symlink(
            root.path().join("presets-local/真的在的.toml"),
            root.path().join("presets-local/指回自己.toml"),
        )
        .unwrap();

        let got = local_files(root.path());
        let escaping = got.iter().find(|f| f.file_name == "跑出去.toml").unwrap();
        assert_eq!(escaping.state, Some(LocalState::Unreadable));
        assert!(
            escaping
                .state_detail
                .as_deref()
                .unwrap_or_default()
                .contains("外面"),
            "原因要说清它指向外面"
        );
        assert_eq!(
            escaping.lineage, None,
            "根外的头一个字节都不读（血统也不许读出来）"
        );

        let inside = got.iter().find(|f| f.file_name == "指回自己.toml").unwrap();
        assert_eq!(inside.state, Some(LocalState::Ok), "根内链接照常能用");
        assert_eq!(inside.lineage, None, "那份没有血统，链接也变不出血统来");
    }

    /// 应用 / 编辑那一道读：坏 TOML 被拦下（带着原因），但**看正文照旧**读得出来
    /// （用户要能看着它去修）
    #[test]
    fn read_preset_text_blocks_the_broken_one_but_viewing_still_works() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "presets-local/坏的.toml",
            "[toolhead]\noffset_x = (1",
        );
        write(root.path(), "presets-local/好的.toml", VALID_TOML);

        let ok = read_preset_text(root.path(), "presets-local/好的.toml").unwrap();
        assert_eq!(ok, VALID_TOML);

        let err = read_preset_text(root.path(), "presets-local/坏的.toml").unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
        assert!(err.message.contains("读不出来"), "{}", err.message);
        assert!(
            err.message.contains("不能应用、也不能改"),
            "要说清这一份现在用不了：{}",
            err.message
        );

        assert_eq!(
            read_text(root.path(), "presets-local/坏的.toml").unwrap(),
            "[toolhead]\noffset_x = (1",
            "看正文不是\"用\"：坏文件也要读得出来，用户才能看着它去修"
        );
    }

    /// **不拿 SHA 说话**：用户在外面改过一轮（字节全变了），只要还是能读的 TOML，
    /// 照常通过 —— 第九层检的不是"和当初一样"，是"现在还能不能用"
    #[test]
    fn an_external_edit_that_still_parses_is_fine() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/本地.toml", VALID_TOML);
        /* 拿别处的编辑器改过：改内容、写回（字节与上一行完全不同） */
        write(
            root.path(),
            "presets-local/本地.toml",
            "[wiping]\nspeed = 80\n",
        );

        let got = local_files(root.path());
        assert_eq!(got[0].state, Some(LocalState::Ok));
        assert!(read_preset_text(root.path(), "presets-local/本地.toml").is_ok());
    }

    /* ---------- 第十层：改名 / 删除（只动名字或删掉，不动字节） ---------- */

    /// 改名：**字节一个不动**（血统原样），旧名字没了、新名字在；
    /// 子目录里的那份改完还在同一个子目录
    #[test]
    fn renaming_keeps_the_bytes_and_the_lineage() {
        let root = tempfile::tempdir().unwrap();
        let with_lineage = "# based_on: mkp/presets/A1-standard.toml\n[toolhead]\noffset_x = 1.0\n";
        write(root.path(), "presets-local/本地 A1.toml", with_lineage);
        let before = format!(
            "{:?}",
            lineage_of_file(&root.path().join("presets-local/本地 A1.toml"))
        );

        let done =
            rename_file(root.path(), "presets-local/本地 A1.toml", "A1-高速版.toml").unwrap();
        assert_eq!(done.path, "presets-local/A1-高速版.toml");
        assert_eq!(done.file_name, "A1-高速版.toml");
        assert!(
            !root.path().join("presets-local/本地 A1.toml").exists(),
            "旧名字那边不该还在"
        );
        let new_path = root.path().join("presets-local/A1-高速版.toml");
        assert_eq!(
            std::fs::read(&new_path).unwrap(),
            with_lineage.as_bytes(),
            "字节一个不动"
        );
        assert_eq!(
            format!("{:?}", lineage_of_file(&new_path)),
            before,
            "血统跟着文件走，改名动不了它"
        );

        /* 子目录里的那份：改完还在同一个子目录 */
        write(root.path(), "presets-local/另存.toml", VALID_TOML);
        let done = rename_file(root.path(), "presets-local/另存.toml", "另存-2.toml").unwrap();
        assert_eq!(done.path, "presets-local/另存-2.toml");
    }

    /*
     * 改名不改类别：换后缀被拒；`.` / `..` / 空名字被拒；拒了之后原文件原地不动。
     * ★ 带路径分隔符 2026-10-20 起**不再拒** —— 剔掉那些字符照落
     *   （作者：「改名有限制…… 不要失败，可以自动截断」）。
     */
    #[test]
    fn renaming_refuses_a_name_that_changes_the_kind_but_strips_a_path() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);

        for bad in ["A1.json", "A1", "A1.TOMLX", "..", "  "] {
            let e = rename_file(root.path(), "presets-local/A1.toml", bad).unwrap_err();
            assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument, "{bad}");
        }
        assert!(
            root.path().join("presets-local/A1.toml").exists(),
            "拒了之后原文件必须还在原地"
        );

        /*
         * 名字里的路径分隔符**剔掉**（它是非法字符，不是目录）——
         * "只换名字、不换目录"那条规矩一条没动。
         */
        let done = rename_file(root.path(), "presets-local/A1.toml", "子目录/A1.toml").unwrap();
        assert_eq!(
            done.file_name, "子目录A1.toml",
            "剔掉分隔符，剩下的还是名字"
        );
        assert!(done.adjusted, "修过了要照实报 —— 界面那一句话靠它");
        assert!(root.path().join("presets-local/子目录A1.toml").is_file());
        assert!(
            !root.path().join("presets-local/A1.toml").exists(),
            "原文件已经改名（改名不复制）"
        );
    }

    /// **不覆盖**：落点已经有东西就拒绝，被撞的那份一个字节没动
    #[test]
    fn renaming_never_overwrites() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        write(
            root.path(),
            "presets-local/B1.toml",
            "[wiping]\nspeed = 80\n",
        );

        let e = rename_file(root.path(), "presets-local/A1.toml", "B1.toml").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.message.contains("不覆盖"), "{}", e.message);
        assert_eq!(
            std::fs::read(root.path().join("presets-local/B1.toml")).unwrap(),
            "[wiping]\nspeed = 80\n".as_bytes(),
            "被撞的那份一个字节没动"
        );
        assert!(root.path().join("presets-local/A1.toml").exists());
    }

    /// 改成同一个名字（前后空白去掉之后一样）：原地不动，不报错
    #[test]
    fn renaming_to_the_same_name_is_a_no_op() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        let done = rename_file(root.path(), "presets-local/A1.toml", " A1.toml ").unwrap();
        assert_eq!(done.path, "presets-local/A1.toml");
        assert_eq!(
            std::fs::read(root.path().join("presets-local/A1.toml")).unwrap(),
            VALID_TOML.as_bytes()
        );
    }

    /// 删除是**真删除**：文件不在了，而且没有多出旁的东西（没有垃圾桶、没有归档）
    #[test]
    fn deleting_really_deletes() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);

        delete_file(root.path(), "presets-local/A1.toml").unwrap();
        assert!(!root.path().join("presets-local/A1.toml").exists());
        let left: Vec<String> = std::fs::read_dir(root.path().join("presets-local"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(left.is_empty(), "删掉就是删掉，不留档：{left:?}");
    }

    /// **正在使用也照删**（作者裁决 2026-10-06：一切皆可删）—— 文件层不碰状态：
    /// 使用中指针的撤下是调用方（ipc）的事，这一层只保证文件本身删得掉
    #[test]
    fn deleting_works_even_while_it_is_the_active_one() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        crate::runtime::app_state::set_active_local(root.path(), "presets-local/A1.toml", "sha")
            .unwrap();

        delete_file(root.path(), "presets-local/A1.toml").unwrap();
        assert!(!root.path().join("presets-local/A1.toml").exists());
    }

    /// **有草稿也照删** —— 草稿的丢弃同样是调用方（ipc）的事，文件层只管文件
    #[test]
    fn deleting_works_even_while_a_draft_is_open() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        crate::runtime::app_state::set_draft(
            root.path(),
            &crate::runtime::state::DraftSubject::local("A1.toml", "presets-local/A1.toml"),
            "sha",
            "正文",
        )
        .unwrap();

        delete_file(root.path(), "presets-local/A1.toml").unwrap();
        assert!(!root.path().join("presets-local/A1.toml").exists());
    }

    /// 删除和改名一样只在 `presets-local/` 那一格里：越界路径一概拒
    #[test]
    fn deleting_stays_in_the_mine_dir() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "exports/别动我.txt", "x");
        assert!(delete_file(root.path(), "exports/别动我.txt").is_err());
        assert!(delete_file(root.path(), "../外面.txt").is_err());
        assert!(root.path().join("exports/别动我.txt").exists());
    }

    /* ---------- 第十一层：另存为一份新的（本地文件 → 本地文件，字节复制） ---------- */

    /// 复制是**字节复制**：新文件与原来那份逐字节相同（内容与血统原样带过去）、
    /// 原文件不动；子目录里的那份复制出来还在同一个子目录
    #[test]
    fn copying_keeps_the_bytes_and_the_lineage() {
        let root = tempfile::tempdir().unwrap();
        let with_lineage = "# based_on: mkp/presets/A1-standard.toml\n[toolhead]\noffset_x = 1.0\n";
        write(root.path(), "presets-local/本地 A1.toml", with_lineage);

        let done = copy_as_new(
            root.path(),
            "presets-local/本地 A1.toml",
            "本地 A1-第二份.toml",
        )
        .unwrap();
        assert_eq!(done.path, "presets-local/本地 A1-第二份.toml");
        assert_eq!(done.file_name, "本地 A1-第二份.toml");
        let new_path = root.path().join("presets-local/本地 A1-第二份.toml");
        assert_eq!(
            std::fs::read(&new_path).unwrap(),
            with_lineage.as_bytes(),
            "字节复制：内容与血统一个字节不差"
        );
        assert_eq!(
            std::fs::read(root.path().join("presets-local/本地 A1.toml")).unwrap(),
            with_lineage.as_bytes(),
            "原文件一个字节不动"
        );
        assert_eq!(
            format!("{:?}", lineage_of_file(&new_path)),
            format!(
                "{:?}",
                lineage_of_file(&root.path().join("presets-local/本地 A1.toml"))
            ),
            "血统原样带过去（不重算）"
        );

        /* 子目录里的那份：复制出来还在同一个子目录 */
        write(root.path(), "presets-local/另存.toml", VALID_TOML);
        let done = copy_as_new(root.path(), "presets-local/另存.toml", "另存-副本.toml").unwrap();
        assert_eq!(done.path, "presets-local/另存-副本.toml");
    }

    /*
     * 复制与改名同一套名字门槛：**换后缀 / `.` / `..` / 空**全拒（那几档没有可修的东西）。
     * ★ 带路径分隔符与超长 2026-10-20 起**不再拒** —— 剔字符 / 截到上限照落。
     */
    #[test]
    fn copying_uses_the_same_name_gate() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        for bad in ["A1.json", "..", "  "] {
            assert!(
                copy_as_new(root.path(), "presets-local/A1.toml", bad).is_err(),
                "{bad} 这种名字该被拒"
            );
        }
        /*
         * 原件名本来就长的时候 `<原名> copy.toml` 会超过 255 字节 —— 那一档
         * **不许让复制失败**（界面照 `adjusted` 说一句"截短了一点"）。
         */
        let long = format!("{}.toml", "a".repeat(300));
        let done = copy_as_new(root.path(), "presets-local/A1.toml", &long).unwrap();
        assert_eq!(done.file_name.len(), NAME_MAX_BYTES, "卡在上限");
        assert!(done.file_name.ends_with(".toml"), "后缀一个字节都不动");
        assert!(done.adjusted, "截过了要照实报");
    }

    /// **不覆盖**（目标已有就拒，被撞的那份与源都一个字节没动）；
    /// **也不自动改名**（名字和原来一样也拒 —— 复制要起个不同的名字）
    #[test]
    fn copying_never_overwrites_and_never_renames_itself() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        write(
            root.path(),
            "presets-local/B1.toml",
            "[wiping]\nspeed = 80\n",
        );

        let e = copy_as_new(root.path(), "presets-local/A1.toml", "B1.toml").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.message.contains("不覆盖"), "{}", e.message);
        assert_eq!(
            std::fs::read(root.path().join("presets-local/B1.toml")).unwrap(),
            "[wiping]\nspeed = 80\n".as_bytes(),
            "被撞的那份一个字节没动"
        );
        assert_eq!(
            std::fs::read(root.path().join("presets-local/A1.toml")).unwrap(),
            VALID_TOML.as_bytes()
        );

        let e = copy_as_new(root.path(), "presets-local/A1.toml", "A1.toml").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.message.contains("不同的名字"), "{}", e.message);
    }

    /// 复制**一个状态都不碰**：使用中指针还是指着原来那份、草稿也还在原来那份上；
    /// 新文件不会自称"使用中"，也没有跟着冒出一张草稿
    #[test]
    fn copying_touches_no_state_at_all() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/A1.toml", VALID_TOML);
        let active = crate::runtime::app_state::set_active_local(
            root.path(),
            "presets-local/A1.toml",
            "sha",
        )
        .unwrap();
        let subject =
            crate::runtime::state::DraftSubject::local("A1.toml", "presets-local/A1.toml");
        crate::runtime::app_state::set_draft(root.path(), &subject, "sha", "改到一半").unwrap();

        copy_as_new(root.path(), "presets-local/A1.toml", "A1-第二份.toml").unwrap();

        let after_active = crate::runtime::app_state::active_preset(root.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            after_active.path.as_deref(),
            Some("presets-local/A1.toml"),
            "使用中没动"
        );
        assert_eq!(after_active.file_name, active.file_name);
        let after_draft = crate::runtime::app_state::draft(root.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            after_draft.path.as_deref(),
            Some("presets-local/A1.toml"),
            "草稿没被迁移"
        );
        assert_eq!(after_draft.text, "改到一半");
    }

    /// 复制的对象是**文件**，不是"能被读成 TOML 的文本"：二进制那份照样原样复制
    /// （与改名 / 删除同族：纯文件操作，不设内容门槛）
    #[test]
    fn copying_does_not_require_readable_text() {
        let root = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(
            &root.path().join("presets-local/二进制的.toml"),
            &[0xff, 0xfe, 0x00, 0x01],
        )
        .unwrap();

        let done = copy_as_new(
            root.path(),
            "presets-local/二进制的.toml",
            "二进制的-副本.toml",
        )
        .unwrap();
        assert_eq!(
            std::fs::read(root.path().join(&done.path)).unwrap(),
            &[0xff, 0xfe, 0x00, 0x01]
        );
    }

    /// 复制和改名一样只在 `presets-local/` 那一格里：越界路径一概拒
    #[test]
    fn copying_stays_in_the_mine_dir() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "exports/别动我.txt", "x");
        assert!(copy_as_new(root.path(), "exports/别动我.txt", "副本.txt").is_err());
        assert!(copy_as_new(root.path(), "../外面.txt", "副本.txt").is_err());
    }

    /* ---------- 另存为我的预设：save_cloud_as_new（2026-10-08 资源库改判） ---------- */

    /// 正路：云端那一份另存成本地这份 —— 云端原文 + 血统三行（`based_on` 指向来源
    /// 交付文件）；落点用**用户起的名字**（不自动加日期、也不自动编号）
    #[test]
    fn saving_official_as_new_writes_a_copy_with_lineage() {
        let user = tempfile::tempdir().unwrap();
        let file = entry_bytes("A1-standard.toml", "涂胶宽度 = 1.0\n");

        let done = save_cloud_as_new(
            user.path(),
            &file,
            "涂胶宽度 = 1.0\n",
            "本地 A1 标准版.toml",
        )
        .unwrap();
        assert_eq!(done.path, "presets-local/本地 A1 标准版.toml");

        let text = String::from_utf8(std::fs::read(user.path().join(&done.path)).unwrap()).unwrap();
        assert!(
            text.contains("# based_on: delivery/mkp/presets/A1-standard.toml"),
            "血统要指向来源交付文件：{text}"
        );
        assert!(
            text.contains("# based_on_sha256:"),
            "摘要那行也要在：{text}"
        );
        assert!(
            text.contains("涂胶宽度 = 1.0"),
            "云端原文要原样在副本里：{text}"
        );
    }

    /// 撞名就拒：不覆盖、也不自动改名（名字由用户自己换）
    #[test]
    fn saving_official_as_new_refuses_an_existing_name() {
        let user = tempfile::tempdir().unwrap();
        write(user.path(), "presets-local/已经在了.toml", "别动我\n");
        let file = entry_bytes("A1-standard.toml", "涂胶宽度 = 1.0\n");

        let e =
            save_cloud_as_new(user.path(), &file, "涂胶宽度 = 1.0\n", "已经在了.toml").unwrap_err();
        assert!(e.message.contains("换个名字"), "{}", e.message);
        assert_eq!(
            String::from_utf8(
                std::fs::read(user.path().join("presets-local/已经在了.toml")).unwrap()
            )
            .unwrap(),
            "别动我\n",
            "已经在盘上的那一份一个字节都不能动"
        );
    }

    /*
     * 名字过的是与改名 / 另存同一道门槛：**不许空、后缀保持原样**（那两档真的没有
     * 可修的东西）。★ 带路径分隔符 2026-10-20 起**不再拒** —— 剔掉照落。
     */
    #[test]
    fn saving_official_as_new_checks_the_new_name() {
        let user = tempfile::tempdir().unwrap();
        let file = entry_bytes("A1-standard.toml", "涂胶宽度 = 1.0\n");
        assert!(save_cloud_as_new(user.path(), &file, "涂胶宽度 = 1.0\n", "  ").is_err());
        assert!(
            save_cloud_as_new(user.path(), &file, "涂胶宽度 = 1.0\n", "别的.txt").is_err(),
            "后缀不许换"
        );
        /* 路径分隔符剔掉照落 —— 落点仍在 presets-local/ 里（那些字符不是目录） */
        let done =
            save_cloud_as_new(user.path(), &file, "涂胶宽度 = 1.0\n", "../外面.toml").unwrap();
        assert_eq!(done.file_name, "..外面.toml", "剔掉那两个分隔符");
        assert!(done.adjusted);
    }

    /// 只有 MKP 预设能另存成本地这份：切片器 profile（json）那类拒
    #[test]
    fn saving_official_as_new_refuses_non_presets() {
        let user = tempfile::tempdir().unwrap();
        let file = super::super::catalog::CatalogFile {
            kind: "bbs_profile".to_owned(),
            ..entry_bytes("MKPProcess.json", "{}")
        };
        let e = save_cloud_as_new(user.path(), &file, "{}", "本地.json").unwrap_err();
        assert!(e.message.contains("不是 MKP 预设"), "{}", e.message);
    }

    /// **一个状态都不碰**：使用中指针还是指着原来那份、草稿也还在原来那份上
    #[test]
    fn saving_official_as_new_touches_no_state_at_all() {
        let user = tempfile::tempdir().unwrap();
        write(user.path(), "presets-local/A1.toml", VALID_TOML);
        crate::runtime::app_state::set_active_local(user.path(), "presets-local/A1.toml", "sha")
            .unwrap();
        let subject =
            crate::runtime::state::DraftSubject::local("A1.toml", "presets-local/A1.toml");
        crate::runtime::app_state::set_draft(user.path(), &subject, "sha", "改到一半").unwrap();

        let file = entry_bytes("A1-standard.toml", "涂胶宽度 = 1.0\n");
        save_cloud_as_new(user.path(), &file, "涂胶宽度 = 1.0\n", "新的一份.toml").unwrap();

        let after_active = crate::runtime::app_state::active_preset(user.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            after_active.path.as_deref(),
            Some("presets-local/A1.toml"),
            "使用中没动"
        );
        let after_draft = crate::runtime::app_state::draft(user.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            after_draft.path.as_deref(),
            Some("presets-local/A1.toml"),
            "草稿没被动过"
        );
        assert_eq!(after_draft.text, "改到一半");
    }

    /* ---------- 校准值：从工作副本正文里读（2026-10-08） ---------- */

    /// 真目录：校准读值要真的注册表（`toolhead.offset.x` → `offset_x` 这张映射在目录里）
    fn real_catalog() -> Catalog {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.parent().expect("src-tauri 上面就是仓库根");
        super::super::catalog::Catalog::build_from_repo(repo).expect("目录构建不出来")
    }

    /// 三个偏移 + 限速都从**注册表派生的位置**读出来；整数写法（`offset_z = 4`）照收
    #[test]
    fn calibration_reads_axes_and_speed_from_the_registry() {
        let catalog = real_catalog();
        let text = "[toolhead]\noffset_x = -0.7\noffset_y = 26.3\noffset_z = 4\nspeed_limit = 70\n";
        let c = calibration_of(&catalog, text);
        assert_eq!(c.axes(), Some((-0.7, 26.3, 4.0)));
        assert_eq!(c.speed, Some(70.0));
    }

    /// 读不出来就是 `None` —— 不拿 0 充数（缺一个轴 = 整份 axes 不成立）
    #[test]
    fn calibration_is_none_when_values_are_not_there() {
        let catalog = real_catalog();
        let c = calibration_of(&catalog, "[toolhead]\noffset_x = 1\n");
        assert_eq!(c.axes(), None);
        assert_eq!(c.x, Some(1.0));
        assert_eq!(c.speed, None);

        let broken = calibration_of(&catalog, "[toolhead]\noffset_x = (1");
        assert_eq!(broken, Calibration::default(), "语法不对 ⇒ 全部未知");
    }

    /* ---------- 第十三层：在文件管理器里显示（只解析落点，窗口是系统的事） ---------- */

    /// 解析出来的是用户根里那个**绝对路径**；读不出来的那份也能显示（文件管理同族）；
    /// 子目录里的那份也认
    #[test]
    fn revealing_resolves_the_file_inside_the_user_root() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-local/能读的.toml", VALID_TOML);
        write(
            root.path(),
            "presets-local/读不出的.toml",
            "[toolhead]\noffset_x = (1",
        );

        let target = reveal_target(root.path(), "presets-local/能读的.toml").unwrap();
        assert!(target.is_absolute(), "给系统的是绝对路径");
        assert!(target.ends_with("presets-local/能读的.toml"));
        /* 坏的那份照样显示 —— 打开文件夹不吃内容 */
        assert!(reveal_target(root.path(), "presets-local/读不出的.toml").is_ok());

        /* 子目录里的那份也认（用户可能自己分文件夹放） */
        write(root.path(), "presets-local/另存.toml", VALID_TOML);
        let nested = reveal_target(root.path(), "presets-local/另存.toml").unwrap();
        assert!(nested.ends_with("presets-local/另存.toml"));
    }

    /// 只认「本地文件」那一格：`exports/`、`../` 一概拒（与改名 / 删除同一道闸）
    #[test]
    fn revealing_stays_in_the_mine_dir() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "exports/别动我.txt", "x");
        assert!(reveal_target(root.path(), "exports/别动我.txt").is_err());
        assert!(reveal_target(root.path(), "../外面.toml").is_err());
    }

    /// 文件不在了（外面删的）就说找不到 —— 不去猜、不去别处找
    #[test]
    fn revealing_a_missing_file_says_so() {
        let root = tempfile::tempdir().unwrap();
        let e = reveal_target(root.path(), "presets-local/不存在.toml").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
        assert!(e.message.contains("找不到"), "{}", e.message);
    }
}
