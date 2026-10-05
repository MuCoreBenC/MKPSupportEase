//! 用户线的两条读：**用户自己的预设文件**（`<appDataDir>/user/presets-mine/`）。
//!
//! 官方线的读在 [`crate::ipc::catalog`] 里（`mkp/` 下载区、`archive/` 归档区）；
//! 这一条是**另一条线**（总纲 §1③「预设 TOML 的一生」），两条不许混：
//! 官方原件不可变、用户修改另存、用户那份**永远不回写官方原件**。
//!
//! 写命令都在这一层（**都只写用户根**，官方原件与下载区一概不碰），分两条路：
//! `begin_preset_edit` 改的是**临时文件**（`run/draft-preset.json`），`commit_preset_draft`
//! 才落到用户根 —— 官方那份 → **另存**成 `（已修改）`；我那份 → **写回自己**（第八层）。
//! 第十层再加两条**管理**命令：`rename_user_preset`（只改名字，字节一个不动；正指着它的
//! 使用中指针与该份的草稿跟着改）与 `delete_user_preset`（**真删除** —— 没有垃圾桶、
//! 没有归档；正在使用 / 还有草稿的不给删，两道闸在 [`runtime::mine::delete_file`]）。
//! 第十三层收尾一条 **外部管理**：`reveal_in_folder`（在 Finder / 资源管理器里选中这一份）
//! —— 之后复制 / 压缩 / 发人 / 备份都随用户，不经过 SupportEase 的业务逻辑。
//! 今天真机上这两条读多半返回空 —— **空是合法状态，不是错误**（用户一份都没另存过）。
//! 它在界面上就是本地表里那一半「我的文件」：看得见、认得出、看得了、也能改。

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
pub struct UserPresetFileDto {
    /// 相对**用户根**的路径（`presets-mine/A1-fast.toml`）—— 读正文 / 应用时把它交回来
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 最后改动时刻（UTC epoch 秒）。界面自己转人话：默认构建不引时间库
    pub modified_unix: Option<u64>,
    /// 认得出是哪一类就给；**认不出是 `null`**（见 [`runtime::mine::kind_of`]）。
    /// 界面上认不出的那一档**在任何类型档下都列** —— 不藏，也不替用户猜
    pub kind: Option<String>,
    /// 第九层的**文件级**状态：`ok` / `unreadable`；`null` = 不是预设候选（认不出是哪一类）。
    ///
    /// **它不判"是不是一份合法 MKP Preset"** —— 那是 Preset 语义（客户端不复制 schema，
    /// 见 [`runtime::mine`] 模块头）。`unreadable` 在界面上说"文件无法读取"，
    /// **不许应用 / 编辑**；外部修改过但仍是能读的 TOML ⇒ 照常是 `ok`，**不因 SHA 报警**
    pub state: Option<String>,
    /// 用不了时的一句人话原因（可直接显示，比如"TOML 语法不对（第 3 行第 1 列）"）；
    /// 能用 / 不适用是 `null`
    pub state_detail: Option<String>,
    /// 它当初基于的官方那一版，和目录里**现在**这一版是不是同一份：
    /// `current` / `outdated` / `unknown`（见 [`runtime::mine::BasedOn`]）。
    ///
    /// **它不判"这份文件好不好"**：用户自己那份从来不是坏文件；
    /// `outdated` 只说"官方换版了，你这份是从旧版派生的"（第七层要说的那件事）。
    pub based_on: String,
    /// 血统里记的来源（`delivery/mkp/presets/A1-standard.toml`）。没有血统是 `null`
    pub based_on_label: Option<String>,
    /// 建副本那一刻来源文件头的版本号（给人看的，形如 `2026-08-19 01:38:13`）
    pub based_on_release: Option<String>,
    /// 来源那份**现在**对应哪台机型 / 哪个版本（认不出留 `null`，界面不猜）
    pub based_on_machine_id: Option<String>,
    pub based_on_version_id: Option<String>,
}

/// 用户自己有哪些文件（`presets-mine/` 里躺着什么）。
///
/// **盘就是底账**：扫盘，不记账本 —— 这一份的主人就是用户，他随时可能在 Finder 里改它。
/// 它没有"官方身份"（云端没有它），但**可能带着血统**（`# based_on*` 三行，写在文件里）：
/// 从哪一份官方、哪一版拷出来改的 —— 于是"官方换版了没有"换台电脑也认得出。
#[tauri::command]
pub async fn get_user_preset_files(app: AppHandle) -> Result<Vec<UserPresetFileDto>, AppError> {
    traced("getUserPresetFiles", |_| {
        let root = crate::fsx::paths::user_root(&app)?;
        let internal = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&internal)?;
        Ok(runtime::mine::mine_files(&root)
            .into_iter()
            .map(|f| {
                let source = runtime::mine::source_of(&catalog, f.lineage.as_ref());
                UserPresetFileDto {
                    based_on: match runtime::mine::based_on(&catalog, f.lineage.as_ref()) {
                        runtime::mine::BasedOn::Current => "current",
                        runtime::mine::BasedOn::Outdated => "outdated",
                        runtime::mine::BasedOn::Unknown => "unknown",
                    }
                    .to_owned(),
                    based_on_label: f.lineage.as_ref().and_then(|l| l.based_on.clone()),
                    based_on_release: f
                        .lineage
                        .as_ref()
                        .and_then(|l| l.based_on_release_time.clone()),
                    based_on_machine_id: source.map(|s| s.machine_id.clone()),
                    based_on_version_id: source.map(|s| s.version_id.clone()),
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

/// 编辑中的那一份（临时文件）给界面的形状。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraftDto {
    /// 改的是**哪条线**上那一份：`official`（交付文件）/ `mine`（我自己的那份）。
    /// 界面靠它决定那一颗保存按钮说什么（另存 / 写回我这份）
    pub origin: String,
    /// 从哪一份改出来的。官方线是 `mkp/` 里的**文件名**；用户线是给人看的文件名
    /// （落点看 `path`）
    pub source_file_name: String,
    /// **用户线**的落点（相对用户根）：保存时写回这里。官方线是 `null`（落点由目录给）
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
    /// 相对**用户根**的路径（`presets-mine/A1-fast（已修改）.toml`）
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 盖掉了一份同名的用户文件（第二次保存就是这种）
    pub replaced: bool,
}

/// 一份用户文件的新落点与名字（第十层改名 / 第十一层另存为一份新的，两处共用这一个形状）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFileIdentityDto {
    /// 落点（相对**用户根**，`presets-mine/…`）
    pub path: String,
    /// 文件名
    pub file_name: String,
}

/// **开始改一份预设**：把正文复制进临时文件（`run/draft-preset.json`），**原件一动不动**。
///
/// 这是"临时编辑"那条链的第一步（总纲 §1③「预设 TOML 的一生」）：
/// 用户改的永远是临时文件，点保存才落到用户根 ——
/// 官方那份**另存**成 `（已修改）`、我那份**写回它自己**（第八层）。
///
/// 两条线走**同一个入口**（`origin` 缺省 `official`，老的调用点不用改），
/// 与「应用」那个入口同一形状（[`crate::ipc::catalog::apply_active_preset`]）：
/// 官方线认**文件名**（目录的键），用户线认**路径**（用户目录里可以自己分文件夹）。
/// 「接着上次改」也按同一把钥匙认 —— 两条线同名很正常，只比文件名会把 A 的草稿接到 B 上。
///
/// 官方线三条前置条件都是**前置**，不靠报错提示：
///   - 只改 MKP 预设（TOML）—— 其它资源不是这一层的对象；
///   - 盘上得真有那一份（没下载就没正文可改，先说"先去下载"）；
///   - **盘上这份得与目录逐字节一致**（第六层：旧版本 / 被改过的不许改 —— 复制就当成
///     存疑内容的原文，而另存之后它还会变成"我改过的那一份"）。判定在
///     [`runtime::delivery::official_text`] 一处，界面不许自己再判一次。
///
/// 用户线三条前置：**是 TOML**（`.json` 认不出是哪一类，不给改）、**盘上真有**
/// （被移走 / 删掉了照实说）、**过第九层的文件级检查**（能读 + UTF-8 + TOML 语法，
/// 见 [`runtime::mine::read_preset_text`]）。它**不查 SHA** —— 用户那份本来就是允许改的，
/// "字节必须还是当初那一份"是官方线的规矩；**语义合法性**（"是不是一份合法 MKP Preset"）
/// 不在客户端判，留给真正的 Preset 能力在应用 / 编辑入口上回答。
///
/// 已经有一份**同一份**的草稿时：**接着改**（`reused: true`），不覆盖用户的改动
/// （那种情况不读盘上的字节，所以上面官方线第三条不成立）。
#[tauri::command]
pub async fn begin_preset_edit(
    app: AppHandle,
    file_name: String,
    origin: Option<String>,
    path: Option<String>,
) -> Result<PresetDraftDto, AppError> {
    traced("beginPresetEdit", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let subject = draft_subject(&file_name, origin.as_deref(), path.as_deref())?;

        if let Some(draft) = runtime::state::load_draft(&root)? {
            if draft.subject().matches(&subject) {
                return Ok(draft_dto(draft, true));
            }
        }

        let (text, source_sha256) = match subject.origin {
            /* 官方线：正文来自下载区（第六层的闸在里面） */
            runtime::state::ActiveOrigin::Official => {
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
                let raw = runtime::delivery::official_text(&root, file)?;
                let sha = runtime::lineage::sha256_hex(&raw);
                (raw, sha)
            }
            /* 用户线：正文就是我那份现在的字节（它本来就在用户的地盘里） */
            runtime::state::ActiveOrigin::Mine => {
                let rel = subject.path.as_deref().unwrap_or_default();
                if runtime::mine::kind_of(&file_name) != Some(runtime::catalog::kind::PRESET) {
                    return Err(AppError::invalid_argument(format!(
                        "{file_name} 不是 MKP 预设（TOML）—— 这一层只改 TOML 预设"
                    )));
                }
                /* 第九层：读不出来（编码 / TOML 语法）就不许改 —— 检查在读的那一步里 */
                let raw = runtime::mine::read_preset_text(&user_root, rel)?;
                let sha = runtime::lineage::sha256_hex(&raw);
                /*
                 * 编辑器里给的是**正文**（那三行血统是程序的元数据，不是用户该改的内容）——
                 * 与官方线同一副样子：两条线打开编辑器看到的都是"预设本身"。
                 * 保存时 `save_back` 会照抄文件里原来那三行，所以用户在编辑器里删掉它们
                 * 也不会把出处弄丢。
                 */
                let text = runtime::lineage::strip_lineage_lines(&raw);
                (text, sha)
            }
        };

        let draft = runtime::state::save_draft(&root, &subject, &source_sha256, &text)?;
        Ok(draft_dto(draft, false))
    })
}

/// 这份草稿改的是哪一条线上的哪一份（两条线各自的钥匙，见 [`begin_preset_edit`]）
fn draft_subject(
    file_name: &str,
    origin: Option<&str>,
    path: Option<&str>,
) -> Result<runtime::state::DraftSubject, AppError> {
    match origin.unwrap_or("official") {
        "official" => Ok(runtime::state::DraftSubject::official(file_name)),
        "mine" => {
            let rel = path
                .ok_or_else(|| AppError::invalid_argument("改我自己那份要说是哪一份（缺 path）"))?;
            Ok(runtime::state::DraftSubject::mine(file_name, rel))
        }
        other => Err(AppError::invalid_argument(format!(
            "不认得 {other} 这条线（只有 official / mine）"
        ))),
    }
}

fn draft_dto(draft: runtime::state::PresetDraft, reused: bool) -> PresetDraftDto {
    PresetDraftDto {
        origin: match draft.origin {
            runtime::state::ActiveOrigin::Official => "official",
            runtime::state::ActiveOrigin::Mine => "mine",
        }
        .to_owned(),
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
        let draft = runtime::state::load_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份"))?;
        runtime::state::save_draft(&root, &draft.subject(), &draft.source_sha256, &text)?;
        Ok(())
    })
}

/// **按参数 key 改草稿里的一个值**（参数页底座 ③）—— 字段级写回，不重生成整份。
///
/// 与 [`put_preset_draft`] 的分工：那条是"把界面上那一整份正文写进去"（编辑器逐字改的场景），
/// 这条是"我只改这一个字段"（参数页用控件改值的场景）—— 后者**不碰**注释、键序、
/// 别人的行，只把那一处换掉。两条都只动 `run/draft-preset.json` 的正文，
/// 官方原件与下载区全程不碰。
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
        let draft = runtime::state::load_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份，改不了参数"))?;
        let catalog = runtime::load_released_catalog(&root)?;

        let patched = crate::presetdata::patch::patch_preset_toml(
            &draft.text,
            &catalog.registry.params,
            &crate::presetdata::patch::FieldEdit::new(param_key, value),
        )?;
        /* 只换正文 —— 来源与打开那一刻的指纹保持不动（与 put 同一条规矩） */
        runtime::state::save_draft(&root, &draft.subject(), &draft.source_sha256, &patched)?;
        Ok(())
    })
}

/// 放弃这次编辑：丢掉临时文件。
///
/// 这一步**天生安全**：官方原件与下载区全程没被碰过，所以"放弃"只是扔掉一份草稿
/// （而且是幂等的 —— 没有草稿时调它也不算错）。
#[tauri::command]
pub async fn discard_preset_draft(app: AppHandle) -> Result<(), AppError> {
    traced("discardPresetDraft", |_| {
        let root = internal_root(&app)?;
        runtime::state::clear_draft(&root)
    })
}

/// **把这一份存进用户根**，然后丢掉草稿。存到哪由**这份草稿改的是哪一份**决定：
///
/// ```text
/// 官方线  另存：presets-mine/<原名>（已修改）<后缀>      原件全程不动
/// 用户线  写回：还是原来那条路径（第八层）               不产生第二份
/// ```
///
/// 三件事都不做（这一层的边界）：不碰官方原件、不碰下载区、**不碰使用中指针**
/// （"生效"是另一条线）。官方线再存一次就是**覆盖它自己** —— 用户改的是"我那份"，
/// 不该越存越多（`replaced` 说出来这次是不是盖掉了上一次那份）。
#[tauri::command]
pub async fn commit_preset_draft(app: AppHandle) -> Result<CommittedDraftDto, AppError> {
    traced("commitPresetDraft", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let draft = runtime::state::load_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份，没得存"))?;

        let done = match draft.origin {
            runtime::state::ActiveOrigin::Official => {
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

                /* 另存本体在 `runtime::mine`（纯函数、有判据盯着"官方原件一动不动"） */
                runtime::mine::commit_draft(
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
            runtime::state::ActiveOrigin::Mine => {
                let rel = draft.path.clone().ok_or_else(|| {
                    AppError::corrupted("这份草稿说它是我自己那份，却没记路径 —— 存不回去")
                })?;
                runtime::mine::save_back(&user_root, &rel, &draft.text)?
            }
        };
        /* 存完就该丢掉草稿：它会盖住下一次「改这份」的"接着上次改" */
        runtime::state::clear_draft(&root)?;

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
/// ① 文件改名（[`runtime::mine::rename_file`]：名字的门槛、"不覆盖"、只换名字不换目录都在那里）；
/// ② **使用中指针跟着改**（正指着这一份时才动；指纹原样 —— 字节没变）；
/// ③ **这一份的草稿跟着改**（用户线认路径，「接着上次改」不接丢）。
///
/// 两本状态账**先读出来**：坏档就什么都不做（宁可原地不动，也不留悬空指针）。
/// ②③ 都在文件改名之后；万一它们失败，说清"文件其实已经改了名"，别让用户以为白点了。
#[tauri::command]
pub async fn rename_user_preset(
    app: AppHandle,
    path: String,
    new_name: String,
) -> Result<UserFileIdentityDto, AppError> {
    traced("renameUserPreset", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let active = runtime::state::load_active(&root)?;
        let draft = runtime::state::load_draft(&root)?;

        let done = runtime::mine::rename_file(&user_root, &path, &new_name)?;

        runtime::state::repoint_active_mine(
            &root,
            active.as_ref(),
            &path,
            &done.path,
            &done.file_name,
        )
        .map_err(|e| {
            AppError::internal(format!(
                "底账没跟上（使用中指针）—— 文件其实已经改名为 {}；重新「应用」一次那一份就能对齐",
                done.file_name
            ))
            .with_detail(e.to_string())
        })?;
        runtime::state::repoint_draft_mine(&root, draft.as_ref(), &path, &done.path, &done.file_name)
            .map_err(|e| {
                AppError::internal(format!(
                    "底账没跟上（没保存的那份草稿）—— 文件其实已经改名为 {}；再点一次「改这份」会从新名字上重来",
                    done.file_name
                ))
                .with_detail(e.to_string())
            })?;

        Ok(UserFileIdentityDto {
            path: done.path,
            file_name: done.file_name,
        })
    })
}

/// **另存为一份新的**（第十一层）：把我自己那一份**按字节**复制成同一格里另一份新的用户文件。
///
/// 与第八层"官方 → 我的文件"那条另存分开：这一层是**我的文件 → 我的文件** ——
/// 原文件一个字节不动；内容与那三行 `# based_on*` 血统**原样带过去**（来源已经是用户文件，
/// 不重算血统 —— 重算会把"从哪一版官方派生"说错）。新名字过同一套门槛、落点已有东西就拒绝
/// （不覆盖、也不自动改名 —— 名字由用户自己换）。
///
/// **一个状态都不碰**：不改使用中指针、不迁移草稿、不建草稿、不进 archive。
/// 它也不读内容、不查状态 —— 名字由 [`runtime::mine::copy_as_new`] 那边把关。
#[tauri::command]
pub async fn copy_user_preset(
    app: AppHandle,
    path: String,
    new_name: String,
) -> Result<UserFileIdentityDto, AppError> {
    traced("copyUserPreset", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        let done = runtime::mine::copy_as_new(&user_root, &path, &new_name)?;
        Ok(UserFileIdentityDto {
            path: done.path,
            file_name: done.file_name,
        })
    })
}

/// **删除一份用户文件**（第十层）：**真删除** —— 没有垃圾桶，也没有归档。
///
/// 两道硬闸（都在 [`runtime::mine::delete_file`]）：**正在使用的不许删**（删了「使用中」
/// 就指向一份不存在的文件）、**还有没保存的草稿的不许删**（删了草稿就永远存不回去）。
/// 两本状态账先读出来（坏档不静默）；删完列表以磁盘为准（界面回来重读用户线）。
#[tauri::command]
pub async fn delete_user_preset(app: AppHandle, path: String) -> Result<(), AppError> {
    traced("deleteUserPreset", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let active = runtime::state::load_active(&root)?;
        let draft = runtime::state::load_draft(&root)?;
        runtime::mine::delete_file(&user_root, &path, active.as_ref(), draft.as_ref())
    })
}

/// **在文件管理器里显示**（第十三层 · 文件外部管理的第一半）：打开 Finder / 资源管理器
/// 并**选中**这份用户文件 —— 拿出去（复制 / 压缩 / 发人 / 备份）全由用户自己来，
/// 不经过 SupportEase 的业务逻辑（"文件外部管理"的含义就这一句）。
///
/// - 只认「我的文件」：路径先过用户根那两道闸（[`runtime::mine::reveal_target`]）；
/// - **读不出来的那份也能显示**（文件管理同族：它只是一份文件，打开文件夹不吃内容）；
/// - **一个状态都不碰**：不改使用中指针、不迁移草稿、不进 archive（它只是打开一个窗口）；
/// - 插件只在 Rust 侧调（`dialog:allow-open` 那种 capability 这里不需要）。
#[tauri::command]
pub async fn reveal_in_folder(app: AppHandle, path: String) -> Result<(), AppError> {
    traced("revealInFolder", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        let target = runtime::mine::reveal_target(&user_root, &path)?;
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| AppError::io("打不开文件管理器").with_detail(e.to_string()))
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
/// **只认 `presets-mine/`**：入参是 [`get_user_preset_files`] 给的那条路径，
/// 这里再核一次前缀（别越到 `exports/` `reports/` 去）并过防穿越。
/// 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来。
#[tauri::command]
pub async fn read_user_preset_text(app: AppHandle, path: String) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("readUserPresetText", |_| {
            let user_root = crate::fsx::paths::user_root(&app)?;
            let rel = path.trim_start_matches('/').to_owned();
            /* 读正文的三道闸在 `runtime::mine::read_text` 一处（两条写路也走它那一套） */
            runtime::mine::read_text(&user_root, &rel)
        })
    });
    task.await
        .map_err(|e| AppError::internal("读用户文件没跑到终局").with_detail(e.to_string()))?
}
