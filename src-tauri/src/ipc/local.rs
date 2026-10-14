//! 用户线的两条读：**用户自己的预设文件**（`<appDataDir>/user/presets-local/`）。
//!
//! 云端线的读在 [`crate::ipc::catalog`] 里（`mkp/` 下载区、`archive/` 归档区）；
//! 这一条是**另一条线**（总纲 §1③「预设 TOML 的一生」），两条不许混：
//! 云端原件不可变、用户修改另存、用户那份**永远不回写云端原件**。
//!
//! 写命令都在这一层（**都只写用户根**，云端原件与下载区一概不碰），分几路：
//!
//! - **取回交付预设**（`catalog::fetch_cloud_preset`，2026-10-09 改判）：按需取回交付预设原件，
//!   再把它落成 `presets-local/<原名>`（还没有的话）—— **用户只面对一份**；
//! - 编辑走 `begin_preset_edit`（**临时文件账**：AppState 的 draft 格，`run/app-state.json`）
//!   → `commit_preset_draft`（**写回我那份自己**，第八层：同一路径、不产生第二份）；
//! - 校准值也写进同一份工作副本（`save_preset_calibration`：按注册表定位改
//!   `toolhead.offset.x/y/z`，注释 / 键序 / 血统一个字节不动）。
//!
//! 第十层再加两条**管理**命令：`rename_local_preset`（只改名字，字节一个不动；正指着它的
//! 使用中指针与该份的草稿跟着改）与 `delete_local_preset`（**真删除** —— 没有垃圾桶、
//! 没有归档；正在使用 / 还有草稿的不给删，两道闸在 [`runtime::local::delete_file`]）。
//! 第十三层收尾一条 **外部管理**：`reveal_in_folder`（在 Finder / 资源管理器里选中这一份）
//! —— 之后复制 / 压缩 / 发人 / 备份都随用户，不经过 SupportEase 的业务逻辑。
//! 今天真机上这两条读多半返回空 —— **空是合法状态，不是错误**（用户一份都没另存过）。
//! 它在界面上就是本地表里那一半「本地文件」：看得见、认得出、看得了、也能改。

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::fsx::paths::internal_root;

use crate::ipc::traced;
use crate::runtime;

/// 用户自己的一份文件（给界面看的形状）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPresetFileDto {
    /// 相对**用户根**的路径（`presets-local/A1-fast.toml`）—— 读正文 / 应用时把它交回来
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 最后改动时刻（UTC epoch 秒）。界面自己转人话：默认构建不引时间库
    pub modified_unix: Option<u64>,
    /// 认得出是哪一类就给；**认不出是 `null`**（见 [`runtime::local::kind_of`]）。
    /// 界面上认不出的那一档**在任何类型档下都列** —— 不藏，也不替用户猜
    pub kind: Option<String>,
    /// 第九层的**文件级**状态：`ok` / `unreadable`；`null` = 不是预设候选（认不出是哪一类）。
    ///
    /// **它不判"是不是一份合法 MKP Preset"** —— 那是 Preset 语义（客户端不复制 schema，
    /// 见 [`runtime::local`] 模块头）。`unreadable` 在界面上说"文件无法读取"，
    /// **不许应用 / 编辑**；外部修改过但仍是能读的 TOML ⇒ 照常是 `ok`，**不因 SHA 报警**
    pub state: Option<String>,
    /// 用不了时的一句人话原因（可直接显示，比如"TOML 语法不对（第 3 行第 1 列）"）；
    /// 能用 / 不适用是 `null`
    pub state_detail: Option<String>,
    /// 它当初基于的云端那一版，和目录里**现在**这一版是不是同一份：
    /// `current` / `outdated` / `unknown`（见 [`runtime::local::BasedOn`]）。
    ///
    /// **它不判"这份文件好不好"**：用户自己那份从来不是坏文件；
    /// `outdated` 只说"云端换版了，你这份是从旧版派生的"（第七层要说的那件事）。
    pub based_on: String,
    /// 血统里记的来源（`delivery/mkp/presets/A1-standard.toml`）。没有血统是 `null`
    pub based_on_label: Option<String>,
    /// 建副本那一刻来源文件头的版本号（给人看的，形如 `2026-08-19 01:38:13`）
    pub based_on_release: Option<String>,
    /// 来源那份**现在**对应哪台机型 / 哪个版本（认不出留 `null`，界面不猜）
    pub based_on_machine_id: Option<String>,
    pub based_on_version_id: Option<String>,
    /// **这一份自己的归属**（文件头 `# machine:` / `# variant:` 两行，对着目录
    /// 大小写无关地归一化到版本 id）；头里没有 / 认不出就回落到来源那份
    /// （`based_on_machine_id` / `based_on_version_id`）。列表的机型 / 版本两列读它。
    ///
    /// 「改归属」（[`set_local_preset_machine_version`]）改的就是文件头那两行 ——
    /// 归属是文件自己的属性，随文件走。
    pub machine_id: Option<String>,
    pub version_id: Option<String>,
    /// **出处账**记的来源：从用户自己的哪一份复制来的（相对用户根路径）。
    /// 没记过 / 是导入的 / 来源已删除 ⇒ `null` —— 界面退回别的说法，不编
    pub copied_from: Option<String>,
    /// 上面那条路径里的**文件名**（界面直接显示用）。没有同上
    pub copied_from_name: Option<String>,
    /// 出处档：`copy`（复制自另一份用户文件）/ `import`（外部导入）。都没记 ⇒ `null`
    pub provenance: Option<String>,
}

/// 用户自己有哪些文件（`presets-local/` 里躺着什么）。
///
/// **盘就是底账**：扫盘，不记账本 —— 这一份的主人就是用户，他随时可能在 Finder 里改它。
/// 它没有"云端身份"（云端没有它），但**可能带着血统**（`# based_on*` 三行，写在文件里）：
/// 从哪一份云端、哪一版拷出来改的 —— 于是"云端换版了没有"换台电脑也认得出。
#[tauri::command]
pub async fn get_local_preset_files(app: AppHandle) -> Result<Vec<LocalPresetFileDto>, AppError> {
    traced("getLocalPresetFiles", |_| {
        let root = crate::fsx::paths::user_root(&app)?;
        let internal = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&internal)?;
        /* 出处账整本读一次（小 JSON），扫出来的每份按路径对号 */
        let book = runtime::provenance::load(&root);
        Ok(runtime::local::local_files(&root)
            .into_iter()
            .map(|f| {
                let source = runtime::local::source_of(&catalog, f.lineage.as_ref());
                /* 出处账：这一份是复制来的还是导入来的（没记过就是 null，照实退回「本地」） */
                let provenance = book.iter().find(|e| e.to == f.path);
                let copied_from = provenance.and_then(|e| e.from.clone());
                /* 自己的归属（头两行优先、回落血统，见 [`owner_of`]） */
                let (owner_machine, owner_version) = owner_of(&catalog, &f);
                LocalPresetFileDto {
                    based_on: match runtime::local::based_on(&catalog, f.lineage.as_ref()) {
                        runtime::local::BasedOn::Current => "current",
                        runtime::local::BasedOn::Outdated => "outdated",
                        runtime::local::BasedOn::Unknown => "unknown",
                    }
                    .to_owned(),
                    based_on_label: f.lineage.as_ref().and_then(|l| l.based_on.clone()),
                    based_on_release: f
                        .lineage
                        .as_ref()
                        .and_then(|l| l.based_on_release_time.clone()),
                    based_on_machine_id: source.map(|s| s.machine_id.clone()),
                    based_on_version_id: source.map(|s| s.version_id.clone()),
                    /*
                     * **自己的归属**：头注释里 `# machine:` / `# variant:` 两行
                     * （[`owner_of`] 归一化过的那份）。认不出回落来源那份的归属 ——
                     * 不猜、也不编。
                     */
                    machine_id: owner_machine,
                    version_id: owner_version,
                    copied_from_name: copied_from
                        .as_deref()
                        .map(|p| p.rsplit('/').next().unwrap_or(p).to_owned()),
                    copied_from,
                    provenance: provenance.map(|e| e.kind.clone()),
                    path: f.path,
                    file_name: f.file_name,
                    size: f.size,
                    modified_unix: f.modified_unix,
                    kind: f.kind.map(str::to_owned),
                    state: f.state.map(|s| s.as_str().to_owned()),
                    state_detail: f.state_detail,
                }
            })
            .collect())
    })
}

/// 一份用户文件的**归属**：哪台机型 / 哪个版本（归一化到目录里的规范写法）。
///
/// 文件头 `# machine:` / `# variant:` 两行优先 —— variant 在 B 类产物里写的是版本
/// id 的小写（`fast`），所以对着目录登记**大小写无关**地找同一台机型下的版本，
/// 取目录里的规范写法（`FAST`），界面拿它查显示名才查得到；头里没有 / 认不出就
/// 回落到**血统指向的那份云端**的归属（不猜、也不编）。
///
/// 用户线列表的机型 / 版本两列与校准页"找哪一份"读的都是它 —— 只有这一处口径。
fn owner_of(
    catalog: &runtime::Catalog,
    f: &runtime::local::LocalFile,
) -> (Option<String>, Option<String>) {
    let own = f.machine.as_deref().and_then(|mid| {
        let vid = f.variant.as_deref()?;
        let canonical = catalog
            .files
            .iter()
            .find(|c| c.machine_id == mid && c.version_id.eq_ignore_ascii_case(vid))
            .map(|c| c.version_id.clone())?;
        Some((mid.to_owned(), canonical))
    });
    if let Some((machine, version)) = own {
        return (Some(machine), Some(version));
    }
    let source = runtime::local::source_of(catalog, f.lineage.as_ref());
    (
        source.map(|s| s.machine_id.clone()),
        source.map(|s| s.version_id.clone()),
    )
}

/// 编辑中的那一份（临时文件）给界面的形状。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraftDto {
    /// 改的是**哪条线**上那一份：`preset`（云端预设线）/ `local`（我自己的那份）。
    /// 界面靠它决定那一颗保存按钮说什么（另存 / 写回我这份）
    ///
    /// ★ 直接持枚举、由 serde 出字符串（与「使用中指针」那一个 DTO 同一套）：
    /// 2026-10-14 C 阶段收敛 —— 以前这里与 `get_preset_draft_summary` 各手写一段
    /// `match` 转字符串，同一条事实三处写法，改个名字要记得改三处。
    pub origin: runtime::state::ActiveOrigin,
    /// 从哪一份改出来的。云端线是 `mkp/` 里的**文件名**；用户线是给人看的文件名
    /// （落点看 `path`）
    pub source_file_name: String,
    /// **用户线**的落点（相对用户根）：保存时写回这里。云端线是 `null`（落点由目录给）
    pub path: Option<String>,
    /// 正文：用户改到哪算哪
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）
    pub updated_unix: u64,
    /// 这次打开是**接着上次改**（草稿本来就是这一份的），不是新建的
    pub reused: bool,
}

/// 另存完成的结果（用户文件落在哪、多大、是不是盖掉了上一次那份）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedDraftDto {
    /// 相对**用户根**的路径（`presets-local/A1-fast（已修改）.toml`）
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 盖掉了一份同名的用户文件（第二次保存就是这种）
    pub replaced: bool,
}

/// 一份用户文件的新落点与名字（第十层改名 / 第十一层另存为一份新的，两处共用这一个形状）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileIdentityDto {
    /// 落点（相对**用户根**，`presets-local/…`）
    pub path: String,
    /// 文件名
    pub file_name: String,
}

/// **开始改一份预设**：把正文复制进临时文件账（AppState 的 draft 格），**原件一动不动**。
///
/// 这是"临时编辑"那条链的第一步（总纲 §1③「预设 TOML 的一生」）：
/// 用户改的永远是临时文件，点保存才落到用户根 ——
/// 云端那份**另存**成 `（已修改）`、我那份**写回它自己**（第八层）。
///
/// 两条线走**同一个入口**（`origin` 缺省 = 云端预设线，老的调用点不用改），
/// 与「应用」那个入口同一形状（[`crate::ipc::catalog::apply_active_preset`]）：
/// 云端线认**文件名**（目录的键），用户线认**路径**（用户目录里可以自己分文件夹）。
/// 「接着上次改」也按同一把钥匙认 —— 两条线同名很正常，只比文件名会把 A 的草稿接到 B 上。
///
/// 云端线三条前置条件都是**前置**，不靠报错提示：
///   - 只改 MKP 预设（TOML）—— 其它资源不是这一层的对象；
///   - 盘上得真有那一份（没下载就没正文可改，先说"先去下载"）；
///   - **盘上这份得与目录逐字节一致**（第六层：旧版本 / 被改过的不许改 —— 复制就当成
///     存疑内容的原文，而另存之后它还会变成"我改过的那一份"）。判定在
///     [`runtime::delivery::cloud_text`] 一处，界面不许自己再判一次。
///
/// 用户线三条前置：**是 TOML**（`.json` 认不出是哪一类，不给改）、**盘上真有**
/// （被移走 / 删掉了照实说）、**过第九层的文件级检查**（能读 + UTF-8 + TOML 语法，
/// 见 [`runtime::local::read_preset_text`]）。它**不查 SHA** —— 用户那份本来就是允许改的，
/// "字节必须还是当初那一份"是云端线的规矩；**语义合法性**（"是不是一份合法 MKP Preset"）
/// 不在客户端判，留给真正的 Preset 能力在应用 / 编辑入口上回答。
///
/// 已经有一份**同一份**的草稿时：**接着改**（`reused: true`），不覆盖用户的改动
/// （那种情况不读盘上的字节，所以上面云端线第三条不成立）。
#[tauri::command]
pub async fn begin_preset_edit(
    app: AppHandle,
    file_name: String,
    origin: Option<runtime::state::ActiveOrigin>,
    path: Option<String>,
) -> Result<PresetDraftDto, AppError> {
    traced("beginPresetEdit", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let subject = draft_subject(&file_name, origin, path.as_deref())?;

        if let Some(draft) = runtime::app_state::draft(&root)? {
            if draft.subject().matches(&subject) {
                return Ok(draft_dto(draft, true));
            }
        }

        let (text, source_sha256) = match subject.origin {
            /* 云端线：正文来自下载区（第六层的闸在里面） */
            runtime::state::ActiveOrigin::Preset => {
                let catalog = runtime::load_released_catalog(&root)?;
                let file = catalog
                    .files
                    .iter()
                    .find(|f| f.file_name == file_name)
                    .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name} 这一份")))?;
                if file.kind != runtime::catalog::kind::PRESET {
                    return Err(AppError::invalid_argument(format!(
                        "{file_name} 不是 MKP 预设 —— 这一层只改 TOML 预设"
                    )));
                }
                /*
                 * 指纹算**盘上真实正文**，不抄目录的期望值：随包 bootstrap 目录给不出期望值，
                 * 抄它会得到一个假的"改前指纹"，草稿的"就绪/过期"判定立刻错乱。
                 * 读出来的字节本来就是权威（这一层就在读它），就地算一次两种目录下语义一致。
                 */
                let raw = runtime::delivery::cloud_text(&root, file)?;
                let sha = runtime::lineage::sha256_hex(&raw);
                (raw, sha)
            }
            /* 用户线：正文就是我那份现在的字节（它本来就在用户的地盘里） */
            runtime::state::ActiveOrigin::Local => {
                let rel = subject.path.as_deref().unwrap_or_default();
                if runtime::local::kind_of(&file_name) != Some(runtime::catalog::kind::PRESET) {
                    return Err(AppError::invalid_argument(format!(
                        "{file_name} 不是 MKP 预设（TOML）—— 这一层只改 TOML 预设"
                    )));
                }
                /* 第九层：读不出来（编码 / TOML 语法）就不许改 —— 检查在读的那一步里 */
                let raw = runtime::local::read_preset_text(&user_root, rel)?;
                let sha = runtime::lineage::sha256_hex(&raw);
                /*
                 * 编辑器里给的是**正文**（那三行血统是程序的元数据，不是用户该改的内容）——
                 * 与云端线同一副样子：两条线打开编辑器看到的都是"预设本身"。
                 * 保存时 `save_back` 会照抄文件里原来那三行，所以用户在编辑器里删掉它们
                 * 也不会把出处弄丢。
                 */
                let text = runtime::lineage::strip_lineage_lines(&raw);
                (text, sha)
            }
        };

        let draft = runtime::app_state::set_draft(&root, &subject, &source_sha256, &text)?;
        super::notify_app_state(&app);
        Ok(draft_dto(draft, false))
    })
}

/// 这份草稿改的是哪一条线上的哪一份（两条线各自的钥匙，见 [`begin_preset_edit`]）。
///
/// ★ `origin` 收的是**枚举**，不是字符串（2026-10-14 C 阶段收敛）：以前这里手写一套
/// `"cloud" / "local"` 的 match，与 serde 那一套（别名、非法值的报错文案）不一致 ——
/// 同一条事实两套规则，换一次名字就要记得改两处。现在只有一处规则：
/// **枚举怎么反序列化，这里就怎么认**；缺省值与 [`runtime::state::ActiveOrigin::default`] 一致。
fn draft_subject(
    file_name: &str,
    origin: Option<runtime::state::ActiveOrigin>,
    path: Option<&str>,
) -> Result<runtime::state::DraftSubject, AppError> {
    match origin.unwrap_or_default() {
        runtime::state::ActiveOrigin::Preset => Ok(runtime::state::DraftSubject::preset(file_name)),
        runtime::state::ActiveOrigin::Local => {
            let rel = path
                .ok_or_else(|| AppError::invalid_argument("改我自己那份要说是哪一份（缺 path）"))?;
            Ok(runtime::state::DraftSubject::local(file_name, rel))
        }
    }
}

/// 「编辑文件」那条链的草稿给界面的形状（2026-10-10：与 [`PresetDraftDto`] 分开存）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetTextDraftDto {
    /// 「改的是哪一份」的人话名
    pub file_name: String,
    /// 相对**用户根**的路径（保存就写回这里）
    pub path: String,
    /// 正文（**带着这一份自己的血统三行**）
    pub text: String,
    pub updated_unix: u64,
    /// 这次打开是**接着上次改**（草稿本来就是这一份的），不是新建的
    pub reused: bool,
}

/// 参数草稿的**摘要**（预设列表行上的「未保存」标记用；没有草稿就是 `null`）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraftSummaryDto {
    /// 同 [`PresetDraftDto::origin`]：枚举直接出，不再手写转换
    pub origin: runtime::state::ActiveOrigin,
    pub source_file_name: String,
    pub path: Option<String>,
    /// **真改过才算**：与来源现在的字节比（只差换行/血统处置的差异不算改动）
    pub dirty: bool,
    pub updated_unix: u64,
}

/// 身份账本的一行：「我这边这一份」↔「云端那一版」
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPresetIdentityDto {
    /// 生成一次、永不变（界面上不显示，只用来对账）。派生视图里它是路径本身
    pub id: String,
    /// 现在落在哪（相对用户根）
    pub path: String,
    pub file_name: String,
    /// 从云端哪个条目下来的（目录里的文件名）
    pub cloud_source: String,
}

fn draft_dto(draft: runtime::state::PresetDraft, reused: bool) -> PresetDraftDto {
    PresetDraftDto {
        origin: draft.origin,
        source_file_name: draft.source_file_name,
        path: draft.path,
        text: draft.text,
        updated_unix: draft.updated_unix,
        reused,
    }
}

/// 把改动写进临时文件（界面边改边存）。**只动正文** —— 来源与那一刻的指纹保持不动。
#[tauri::command]
pub async fn put_preset_draft(app: AppHandle, text: String) -> Result<(), AppError> {
    traced("putPresetDraft", |_| {
        let root = internal_root(&app)?;
        let draft = runtime::app_state::draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份"))?;
        runtime::app_state::set_draft(&root, &draft.subject(), &draft.source_sha256, &text)?;
        super::notify_app_state(&app);
        Ok(())
    })
}

/// **按参数 key 改草稿里的一个值**（参数页底座 ③）—— 字段级写回，不重生成整份。
///
/// 与 [`put_preset_draft`] 的分工：那条是"把界面上那一整份正文写进去"（编辑器逐字改的场景），
/// 这条是"我只改这一个字段"（参数页用控件改值的场景）—— 后者**不碰**注释、键序、
/// 别人的行，只把那一处换掉。两条都只动 AppState 里 draft 格的正文，
/// 云端原件与下载区全程不碰。
///
/// 定位与取值形态归 [`crate::presetdata::patch`]：它拿**字段定义**把 `param_key`
/// 翻成 `(section, toml_key)`，并按 `valueType` 决定写成数字 / 布尔 / 字符串。
/// 字段定义从**运行时 catalog** 来（`<appDataDir>/catalog.json` 的 `registry`）——
/// 客户端不读仓库、也没有 `ParamRegistry` 那套编辑侧状态。
#[tauri::command]
pub async fn patch_preset_draft(
    app: AppHandle,
    param_key: String,
    value: String,
) -> Result<(), AppError> {
    traced("patchPresetDraft", |_| {
        let root = internal_root(&app)?;
        let draft = runtime::app_state::draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份，改不了参数"))?;
        let catalog = runtime::load_released_catalog(&root)?;

        let patched = crate::presetdata::patch::patch_preset_toml(
            &draft.text,
            &catalog.registry.params,
            &crate::presetdata::patch::FieldEdit::new(param_key, value),
        )?;
        /* 只换正文 —— 来源与打开那一刻的指纹保持不动（与 put 同一条规矩） */
        runtime::app_state::set_draft(&root, &draft.subject(), &draft.source_sha256, &patched)?;
        super::notify_app_state(&app);
        Ok(())
    })
}

/*
 * ---------- 「编辑文件」那条链（2026-10-10：与参数页那条链分开存） ----------
 *
 * 两条链的钥匙不同：参数页那条认 `(origin, file_name, path)`（见
 * [`begin_preset_edit`]），这一条只认**用户线的路径** —— 编辑器只改用户自己的文件。
 */

/// **开始改一份用户预设的正文**（预设页「编辑文件」）：把当前字节复制进 AppState 的
/// `text_draft` 格，**盘上那份一动不动**。已经有同一份（同路径）的草稿就**接着改**。
///
/// 与 [`begin_preset_edit`] 的两处不同：**正文带着这一份自己的血统三行**（编辑这份时
/// 它们就在文件里，保存时照写回），以及**入口只收用户线**（云端那份要先拷成自己的）。
#[tauri::command]
pub async fn begin_text_edit(
    app: AppHandle,
    file_name: String,
    path: String,
) -> Result<PresetTextDraftDto, AppError> {
    traced("beginTextEdit", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        if runtime::local::kind_of(&file_name) != Some(runtime::catalog::kind::PRESET) {
            return Err(AppError::invalid_argument(format!(
                "{file_name} 不是 MKP 预设（TOML）—— 这一层只改 TOML 预设"
            )));
        }

        /* 同一份（路径相同）已经有草稿 → 接着改，不覆盖用户已经敲进去的字 */
        if let Some(draft) = runtime::app_state::text_draft(&root)? {
            if draft.key() == path {
                return Ok(PresetTextDraftDto {
                    file_name: draft.file_name,
                    path: draft.path,
                    text: draft.text,
                    updated_unix: draft.updated_unix,
                    reused: true,
                });
            }
        }

        /* 第九层文件级检查在读的那一步里（读不出来就不许改） */
        let raw = runtime::local::read_preset_text(&user_root, &path)?;
        let draft = runtime::app_state::set_text_draft(&root, &file_name, &path, &raw)?;
        super::notify_app_state(&app);
        Ok(PresetTextDraftDto {
            file_name: draft.file_name,
            path: draft.path,
            text: draft.text,
            updated_unix: draft.updated_unix,
            reused: false,
        })
    })
}

/// 把改动写进临时文件（编辑框边改边存）。**只动正文**，[`begin_text_edit`] 那一刻的其它不变。
#[tauri::command]
pub async fn put_text_draft(app: AppHandle, text: String) -> Result<(), AppError> {
    traced("putTextDraft", |_| {
        let root = internal_root(&app)?;
        let draft = runtime::app_state::text_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份正文"))?;
        runtime::app_state::set_text_draft(&root, &draft.file_name, &draft.path, &text)?;
        super::notify_app_state(&app);
        Ok(())
    })
}

/// **保存正文**（预设页「编辑文件」→「保存」）：把正文草稿**原子地**写回它自己 ——
/// 一次替换**整份文件**（`atomic_write`：先写临时文件、再换上去）。
///
/// 写完丢草稿，并**把参数页那条链的草稿也作废**（同一份文件的另一份草稿已经过期）。
#[tauri::command]
pub async fn commit_text_draft(app: AppHandle) -> Result<CommittedDraftDto, AppError> {
    traced("commitTextDraft", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let draft = runtime::app_state::text_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份正文，没得存"))?;
        runtime::local::write_text(&user_root, &draft.path, &draft.text)?;
        runtime::app_state::clear_text_draft(&root)?;
        /* 另一侧作废：盘上字节换了，参数页手上那份就成了孤儿 */
        runtime::app_state::clear_draft(&root)?;
        super::notify_app_state(&app);
        Ok(CommittedDraftDto {
            path: draft.path.clone(),
            file_name: draft.file_name,
            size: draft.text.len() as u64,
            /* 写回它自己：永远是"盖掉了原来那一版" */
            replaced: true,
        })
    })
}

/// 放弃这次编辑：丢掉临时文件。**天然幂等**（没有草稿也不算错）
#[tauri::command]
pub async fn discard_text_draft(app: AppHandle) -> Result<(), AppError> {
    traced("discardTextDraft", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::clear_text_draft(&root)?;
        super::notify_app_state(&app);
        Ok(())
    })
}

/// **参数草稿的摘要**（预设列表行上的「未保存」标记、"要不要重开"两处用）。
/// **只摘要不正文**；现在没有草稿就是 `null`。
///
/// `dirty` 是"与来源现在的字节比"算出来的，不是"有没有敲进去过" —— 打开之后一个字没改、
/// 或改回原样，都不算脏。比不出来（目录里已经没有它了）时**保守说脏**：有草稿在就别瞒。
#[tauri::command]
pub async fn get_preset_draft_summary(
    app: AppHandle,
) -> Result<Option<PresetDraftSummaryDto>, AppError> {
    traced("getPresetDraftSummary", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let Some(draft) = runtime::app_state::draft(&root)? else {
            return Ok(None);
        };

        let dirty = match draft.origin {
            runtime::state::ActiveOrigin::Preset => {
                let catalog = runtime::load_released_catalog(&root)?;
                match catalog
                    .files
                    .iter()
                    .find(|f| f.file_name == draft.source_file_name)
                {
                    /* 比不出来 == 目录里已经没有它了：有草稿在就保守说"未保存" */
                    None => true,
                    Some(file) => match runtime::delivery::cloud_text(&root, file) {
                        Ok(base) => draft.text != base,
                        Err(_) => true,
                    },
                }
            }
            runtime::state::ActiveOrigin::Local => {
                let rel = draft.path.clone().unwrap_or_default();
                match runtime::local::read_preset_text(&user_root, &rel) {
                    /* 打开那一刻给的是**去了血统**的正文（两侧同一副样子） */
                    Ok(raw) => draft.text != runtime::lineage::strip_lineage_lines(&raw),
                    Err(_) => true,
                }
            }
        };

        Ok(Some(PresetDraftSummaryDto {
            origin: draft.origin,
            source_file_name: draft.source_file_name,
            path: draft.path,
            dirty,
            updated_unix: draft.updated_unix,
        }))
    })
}

/// **身份账本**：「我这边这一份」↔「云端那一版」的对应关系 —— 预设页判"下载区里这
/// 一份我有没有"的唯一依据，也是"更新"找要更新的是哪一份的依据。
///
/// **派生视图，不设持久化账本**（有一条纪律：期望值只来自当前目录）。所以：
/// 只有**现在目录里还活着**的那一版才进账 —— 那一版已被移出版次的（用户文件自己
/// 还能用，只是"跟不上更新"了）不进；没有血统的（手工拷的 / 自己写的）也不进。
#[tauri::command]
pub async fn get_local_preset_index(
    app: AppHandle,
) -> Result<Vec<LocalPresetIdentityDto>, AppError> {
    traced("getLocalPresetIndex", |_| {
        let internal = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&internal)?;
        let files = runtime::local::local_files(&user_root);

        Ok(files
            .into_iter()
            .filter_map(|f| {
                let lineage = f.lineage.as_ref()?;
                /* 那一版还在目录里才认这个关系（否则这笔关系已经无处对账） */
                let source = runtime::local::source_of(&catalog, Some(lineage))?;
                Some(LocalPresetIdentityDto {
                    id: f.path.clone(),
                    file_name: f.file_name.clone(),
                    path: f.path,
                    cloud_source: source.file_name.clone(),
                })
            })
            .collect())
    })
}

/// 放弃这次编辑：丢掉临时文件。
///
/// 这一步**天生安全**：云端原件与下载区全程没被碰过，所以"放弃"只是扔掉一份草稿
/// （而且是幂等的 —— 没有草稿时调它也不算错）。
#[tauri::command]
pub async fn discard_preset_draft(app: AppHandle) -> Result<(), AppError> {
    traced("discardPresetDraft", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::clear_draft(&root)?;
        super::notify_app_state(&app);
        Ok(())
    })
}

/// **把这一份存进用户根**，然后丢掉草稿。存到哪由**这份草稿改的是哪一份**决定：
///
/// ```text
/// 云端线  另存：presets-local/<原名>（已修改）<后缀>      原件全程不动
/// 用户线  写回：还是原来那条路径（第八层）               不产生第二份
/// ```
///
/// 三件事都不做（这一层的边界）：不碰云端原件、不碰下载区、**不碰使用中指针**
/// （"生效"是另一条线）。云端线再存一次就是**覆盖它自己** —— 用户改的是"我那份"，
/// 不该越存越多（`replaced` 说出来这次是不是盖掉了上一次那份）。
#[tauri::command]
pub async fn commit_preset_draft(app: AppHandle) -> Result<CommittedDraftDto, AppError> {
    traced("commitPresetDraft", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let draft = runtime::app_state::draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份，没得存"))?;

        let done = match draft.origin {
            runtime::state::ActiveOrigin::Preset => {
                /*
                 * 血统要写进那份用户文件里：来源 = 目录里那一份的 **`catalog.path`**
                 * （`delivery/mkp/presets/A1-standard.toml`，与工作台建副本的"哪一版"同一形状）。
                 *
                 * 目录里已经没有它了（编辑期间换源 / 下线）就只写文件名本身 —— 血统还认得出
                 * "从哪一份"，只是少了"在哪"。**不为这个拦住保存**：用户改了半天的东西
                 * 不该被一个地址问题卡住。
                 */
                let based_on = catalog_lookup(&root, &draft.source_file_name)
                    .map(|f| f.path)
                    .unwrap_or_else(|| draft.source_file_name.clone());

                /* 另存本体在 `runtime::local`（纯函数、有判据盯着"云端原件一动不动"） */
                runtime::local::commit_draft(
                    &user_root,
                    &draft.source_file_name,
                    &based_on,
                    &draft.text,
                )?
            }
            /*
             * 用户线：**写回它自己**（第八层）—— 同一个路径、同一份文件，
             * 血统照抄原来那三行（出处没变）。这一条不查目录、也不需要目录：
             * 我那份的出处写在它自己身上。
             */
            runtime::state::ActiveOrigin::Local => {
                let rel = draft.path.clone().ok_or_else(|| {
                    AppError::corrupted("这份草稿说它是我自己那份，却没记路径 —— 存不回去")
                })?;
                runtime::local::save_back(&user_root, &rel, &draft.text)?
            }
        };
        /* 存完就该丢掉草稿：它会盖住下一次「改这份」的"接着上次改" */
        runtime::app_state::clear_draft(&root)?;
        /*
         * 另一侧作废：**「编辑文件」那份草稿也丢掉**（2026-10-10 契约写死）——
         * 盘上字节已经换了，编辑器手上那份正文就成了孤儿，再改一个字就存不上。
         */
        runtime::app_state::clear_text_draft(&root)?;
        super::notify_app_state(&app);

        Ok(CommittedDraftDto {
            path: done.path,
            file_name: done.file_name,
            size: done.size,
            replaced: done.replaced,
        })
    })
}

/// **重命名一份用户文件**（第十层）：只改名字，**字节一个不动** ——
/// 内容、那三行血统、TOML 都不重写；改完还是同一份 Preset（判据逐字节盯着）。
///
/// 三件事按顺序做：
/// ① 文件改名（[`runtime::local::rename_file`]：名字的门槛、"不覆盖"、只换名字不换目录都在那里）；
/// ② **使用中指针跟着改**（正指着这一份时才动；指纹原样 —— 字节没变）；
/// ③ **这一份的草稿跟着改**（用户线认路径，「接着上次改」不接丢）。
///
/// 两本状态账**先读出来**：坏档就什么都不做（宁可原地不动，也不留悬空指针）。
/// ②③ 都在文件改名之后；万一它们失败，说清"文件其实已经改了名"，别让用户以为白点了。
#[tauri::command]
pub async fn rename_local_preset(
    app: AppHandle,
    path: String,
    new_name: String,
) -> Result<LocalFileIdentityDto, AppError> {
    traced("renameLocalPreset", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        /* 两本状态账先整份读出来：坏档就什么都不做（宁可原地不动，也不留悬空指针） */
        runtime::app_state::load(&root)?;

        let done = runtime::local::rename_file(&user_root, &path, &new_name)?;

        /* 使用中指针 + 草稿一起跟着改名走 —— AppState 一次原子写里改两格；
        万一失败，说清"文件其实已经改了名"，别让用户以为白点了 */
        runtime::app_state::repoint_local(&root, &path, &done.path, &done.file_name).map_err(
            |e| {
                AppError::internal(format!(
                    "状态账没跟上（使用中指针 / 草稿）—— 文件其实已经改名为 {}；重新「应用」一次那一份、再点一次「改这份」就能对齐",
                    done.file_name
                ))
                .with_detail(e.to_string())
            },
        )?;
        super::notify_app_state(&app);
        /* 出处账跟着走：副本改名了 to 跟走，来源改名了 from 跟走（「复制自 X」不失联） */
        let _ = runtime::provenance::repath(&user_root, &path, &done.path);

        Ok(LocalFileIdentityDto {
            path: done.path,
            file_name: done.file_name,
        })
    })
}

/// **另存为一份新的**（第十一层）：把我自己那一份**按字节**复制成同一格里另一份新的用户文件。
///
/// 与第八层"云端 → 本地文件"那条另存分开：这一层是**本地文件 → 本地文件** ——
/// 原文件一个字节不动；内容与那三行 `# based_on*` 血统**原样带过去**（来源已经是用户文件，
/// 不重算血统 —— 重算会把"从哪一版云端派生"说错）。新名字过同一套门槛、落点已有东西就拒绝
/// （不覆盖、也不自动改名 —— 名字由用户自己换）。
///
/// **一个状态都不碰**：不改使用中指针、不迁移草稿、不建草稿、不进 archive。
/// 它也不读内容、不查状态 —— 名字由 [`runtime::local::copy_as_new`] 那边把关。
#[tauri::command]
pub async fn copy_local_preset(
    app: AppHandle,
    path: String,
    new_name: String,
) -> Result<LocalFileIdentityDto, AppError> {
    traced("copyLocalPreset", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        let done = runtime::local::copy_as_new(&user_root, &path, &new_name)?;
        /* 出处账：副本从哪来的 —— 界面上「来源：复制自 X」靠它。记不上不影响复制本身 */
        let _ = runtime::provenance::record(
            &user_root,
            &done.path,
            Some(&path),
            runtime::provenance::ProvenanceKind::Copy,
        );
        Ok(LocalFileIdentityDto {
            path: done.path,
            file_name: done.file_name,
        })
    })
}

/// 一台机型 / 一个版本对应的「本地那份」+ 它带的校准值（校准页的初值与保存落点）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCopyCalibrationDto {
    pub file_name: String,
    /// 相对用户根的落点（`presets-local/…`）—— 保存校准时把它交回来
    pub path: String,
    /// 三轴偏移；缺一个轴就是 `null`（不拿半个基准充数）
    pub axes: Option<crate::ipc::Axes>,
    /// 涂胶速度限速；读不出来是 `null`
    pub speed: Option<f64>,
}

/// **这台机型 / 这个版本，「本地那份」在哪**（按归属匹配，盘当底账）。
///
/// 找法（有先后）：**底账正用着的那一份**（origin=local 且归属匹配）> 用户目录里
/// 第一份匹配的。匹配按**归属**（`# machine:` / `# variant:` 归一化后回落血统，
/// 见 [`owner_of`]）；只有能读的 TOML 预设算数（读不出来的那份即便归属对得上也不能用）。
///
/// 两个消费者**共用这一处口径**：校准页取初值 / 存回（[`get_local_copy_for`]）与
/// 首页「复制后处理脚本」拼 `--Toml`（[`super::get_post_process_command`]）。
/// 一份都没有 ⇒ `None`（**不是错误**：还没另存过自己的一份）。
pub(crate) fn local_copy_for(
    internal_root: &std::path::Path,
    user_root: &std::path::Path,
    catalog: &runtime::Catalog,
    machine_id: &str,
    version_id: &str,
) -> Result<Option<runtime::local::LocalFile>, AppError> {
    let files = runtime::local::local_files(user_root);
    /* 候选 = 归属对得上 + 是能读的 TOML（读不出来的那份即便匹配也不能当基准） */
    let matched: Vec<&runtime::local::LocalFile> = files
        .iter()
        .filter(|f| f.kind == Some(runtime::catalog::kind::PRESET))
        .filter(|f| f.state == Some(runtime::local::LocalState::Ok))
        .filter(|f| {
            let owner = owner_of(catalog, f);
            owner.0.as_deref() == Some(machine_id) && owner.1.as_deref() == Some(version_id)
        })
        .collect();

    let active = runtime::app_state::active_preset(internal_root)?;
    let pick = active
        .as_ref()
        .filter(|a| a.origin == runtime::state::ActiveOrigin::Local)
        .and_then(|a| a.path.as_deref())
        .and_then(|p| matched.iter().find(|f| f.path == p).copied())
        .or_else(|| matched.first().copied());
    Ok(pick.cloned())
}

/// **这台机型 / 这个版本，本地那份在哪**（校准页：初值从它读、保存写它）。
///
/// 匹配口径在 [`local_copy_for`]（与首页后处理命令共用同一处）。
/// 一份都没有 / 都读不出来 ⇒ `None`：校准页照旧显示云端默认值，**保存会被拦**
/// （"先下载才有得存"—— 校准的对象是用户的工作副本，云端基线永远不可变）。
#[tauri::command]
pub async fn get_local_copy_for(
    app: AppHandle,
    machine_id: String,
    version_id: String,
) -> Result<Option<LocalCopyCalibrationDto>, AppError> {
    traced("getLocalCopyFor", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let Some(f) = local_copy_for(&root, &user_root, &catalog, &machine_id, &version_id)? else {
            return Ok(None);
        };

        let text = runtime::local::read_text(&user_root, &f.path)?;
        let calib = runtime::local::calibration_of(&catalog, &text);
        Ok(Some(LocalCopyCalibrationDto {
            file_name: f.file_name.clone(),
            path: f.path.clone(),
            axes: calib.axes().map(|(x, y, z)| crate::ipc::Axes { x, y, z }),
            speed: calib.speed,
        }))
    })
}

/// **把校准值写进「我的那一份」**（2026-10-08 作者改判：偏移随工作副本走，
/// `index/offsets.json` 那个只写不读的孤岛退役）。
///
/// 三个值都从**注册表派生**的位置改（`toolhead.offset.x/y/z` → 对应 TOML 键），
/// [`crate::presetdata::patch`] 只换那个值 —— 注释、键序、血统三行全程一个字节不动；
/// 写完就是那份工作副本的新内容（原子替换）。
///
/// 只认「本地文件」那一格（[`runtime::local::read_text`] / [`runtime::local::write_text`]
/// 的前缀 + 防穿越两道闸）：云端基线不可变 —— 没有副本就先下载，校准不落别处。
#[tauri::command]
pub async fn save_preset_calibration(
    app: AppHandle,
    path: String,
    axes: crate::ipc::Axes,
) -> Result<(), AppError> {
    traced("savePresetCalibration", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let mut text = runtime::local::read_text(&user_root, &path)?;
        for (key, value) in [
            ("toolhead.offset.x", axes.x),
            ("toolhead.offset.y", axes.y),
            ("toolhead.offset.z", axes.z),
        ] {
            text = crate::presetdata::patch::patch_preset_toml(
                &text,
                &catalog.registry.params,
                &crate::presetdata::patch::FieldEdit::new(key, value.to_string()),
            )?;
        }
        runtime::local::write_text(&user_root, &path, &text)
    })
}

/// **删除一份用户文件**（第十层）：**真删除** —— 没有垃圾桶，也没有归档。
///
/// 作者裁决（2026-10-06）：**不再拦"正在使用 / 有草稿"** —— 一切皆可删。删之前把
/// 属于这一份的状态一并清掉：使用中指针撤下（悬空的「使用中」比「没在用」糟）、
/// 没保存的草稿一并丢弃（写回要文件在）。两本状态账先读出来（坏档不静默）；
/// 删完列表以磁盘为准（界面回来重读用户线）。
#[tauri::command]
pub async fn delete_local_preset(app: AppHandle, path: String) -> Result<(), AppError> {
    traced("deleteLocalPreset", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let active = runtime::app_state::active_preset(&root)?;
        let draft = runtime::app_state::draft(&root)?;
        /* 属于这一份的状态先撤（命中 = 用户线 + 同一条相对路径），再删文件 */
        let mut state_cleared = false;
        if active.as_ref().is_some_and(|a| {
            a.origin == runtime::state::ActiveOrigin::Local
                && a.path.as_deref() == Some(path.as_str())
        }) {
            runtime::app_state::clear_active_preset(&root)?;
            state_cleared = true;
        }
        if draft.as_ref().is_some_and(|d| {
            d.origin == runtime::state::ActiveOrigin::Local
                && d.path.as_deref() == Some(path.as_str())
        }) {
            runtime::app_state::clear_draft(&root)?;
            state_cleared = true;
        }
        runtime::local::delete_file(&user_root, &path)?;
        /* 备注覆盖跟着删（删了重新下载 / 重新复制 = 回到工作台那句；删账失败不拦删除） */
        let _ = runtime::remarks::remove(&user_root, &path);
        /*
         * 逐参数「云端更新」决定账跟着删：那份文件都不在了，"我处理过哪几项"再留着
         * 只会是一条悬空的账（而且下一次同名文件出现时会莫名其妙继承它）。
         */
        let _ = runtime::app_state::forget_param_decisions(&root, &path);
        if state_cleared {
            super::notify_app_state(&app);
        }
        Ok(())
    })
}

/// **在文件管理器里显示**（第十三层 · 文件外部管理的第一半）：打开 Finder / 资源管理器
/// 并**选中**这份用户文件 —— 拿出去（复制 / 压缩 / 发人 / 备份）全由用户自己来，
/// 不经过 SupportEase 的业务逻辑（"文件外部管理"的含义就这一句）。
///
/// - 只认「本地文件」：路径先过用户根那两道闸（[`runtime::local::reveal_target`]）；
/// - **读不出来的那份也能显示**（文件管理同族：它只是一份文件，打开文件夹不吃内容）；
/// - **一个状态都不碰**：不改使用中指针、不迁移草稿、不进 archive（它只是打开一个窗口）；
/// - 插件只在 Rust 侧调（`dialog:allow-open` 那种 capability 这里不需要）。
#[tauri::command]
pub async fn reveal_in_folder(app: AppHandle, path: String) -> Result<(), AppError> {
    traced("revealInFolder", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        let target = runtime::local::reveal_target(&user_root, &path)?;
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| AppError::io("打不开文件管理器").with_detail(e.to_string()))
    })
}

/* ---------- 备注覆盖账（客户端副标题）+ 改归属（2026-10-07） ---------- */

/// **用户改过的备注**整本给界面（键 = 文件身份，见 [`runtime::remarks`] 模块头）。
///
/// 界面拿它叠加在 catalog 版本备注上：账上有 ⇒ 用用户的；没有 ⇒ 用工作台那句。
#[tauri::command]
pub async fn get_preset_remarks(
    app: AppHandle,
) -> Result<std::collections::BTreeMap<String, String>, AppError> {
    traced("getPresetRemarks", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        Ok(runtime::remarks::load(&user_root))
    })
}

/// **改一份预设的备注**（客户端副标题）。`remark` 为 `null` / 空 = 清掉覆盖，
/// 回到工作台写的那句。只动这一本账 —— 文件字节与目录全程不碰。
#[tauri::command]
pub async fn set_preset_remark(
    app: AppHandle,
    key: String,
    remark: Option<String>,
) -> Result<(), AppError> {
    traced("setPresetRemark", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        runtime::remarks::set(&user_root, &key, remark.as_deref())
    })
}

/// **改一份用户预设的归属**（机型 / 版本）：把文件头的 `# machine:` / `# variant:`
/// 两行换成新值（正文一个字节不动）。复制出来的那份从此在列表的机型 / 版本两列
/// 显示你选的这台。
///
/// 机型**必须目录里真有**（机型是目录登记的实体，认不出的机型在界面上什么都
/// 查不到）；**版本随便写**（2026-10-07 作者：「版本也不一定是选择，加一个自定义」
/// —— 版本只是头注释里的一行字，界面遇到不认识的就照原文显示，不拦、不猜）。
#[tauri::command]
pub async fn set_local_preset_machine_version(
    app: AppHandle,
    path: String,
    machine_id: String,
    version_id: String,
) -> Result<(), AppError> {
    traced("setLocalPresetMachineVersion", |_| {
        let machine_id = machine_id.trim();
        let version_id = version_id.trim();
        if machine_id.is_empty() || version_id.is_empty() {
            return Err(AppError::invalid_argument(
                "机型与版本都要填 —— 归属写的是文件头的那两行，留空就认不出",
            ));
        }
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let known = catalog.machines.iter().any(|m| m.id == machine_id);
        if !known {
            return Err(AppError::invalid_argument(format!(
                "{machine_id} 不是目录里登记的机型 —— 归属的机型要选一台真的机器"
            )));
        }
        runtime::local::set_machine_variant(&user_root, &path, machine_id, version_id)
    })
}

/// 目录里按文件名找一份交付文件（用来给血统写上"来源的落点"）。
/// 读不到目录就 `None` —— 那不该拦住一次保存。
fn catalog_lookup(
    root: &std::path::Path,
    file_name: &str,
) -> Option<runtime::catalog::CatalogFile> {
    runtime::load_released_catalog(root)
        .ok()?
        .files
        .into_iter()
        .find(|f| f.file_name == file_name)
}

/// 读用户自己那份的正文。
///
/// **只认 `presets-local/`**：入参是 [`get_local_preset_files`] 给的那条路径，
/// 这里再核一次前缀（别越到 `exports/` `reports/` 去）并过防穿越。
/// 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来。
#[tauri::command]
pub async fn read_local_preset_text(app: AppHandle, path: String) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("readLocalPresetText", |_| {
            let user_root = crate::fsx::paths::user_root(&app)?;
            let rel = path.trim_start_matches('/').to_owned();
            /* 读正文的三道闸在 `runtime::local::read_text` 一处（两条写路也走它那一套） */
            runtime::local::read_text(&user_root, &rel)
        })
    });
    task.await
        .map_err(|e| AppError::internal("读用户文件没跑到终局").with_detail(e.to_string()))?
}
