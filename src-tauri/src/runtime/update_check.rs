//! **自动检查软件更新**（2026-11-05）：挂在「客户端成功拉到云端目录」之后，顺路查一次
//! [`crate::runtime::release_info`]。
//!
//! ## 三条硬规矩
//!
//! 1. **不进启动 / 首屏路径**（铁律 2：启动零网络）。入口只有一处：
//!    [`crate::ipc::catalog::check_remote_update`] 成功返回前。拉目录本来就要联网，
//!    搭这一趟，不新开定时轮询 —— 用户多了也不会把公共仓库打爆。
//! 2. **本会话只查一次**。进程内一个 `AtomicBool`；开关关着时不占这个名额。
//!    查失败（离线 / 没发过版 / 源没配）把名额放回去，后面联网了还能再试。
//! 3. **失败一律静默**。这一层是"顺路"，不是功能：不许弹错、不许把目录检查
//!    拖成失败。有新版才推一个事件，由前端右上角通知决定怎么显示。
//!
//! 版本比较、地址解析、release.json 的形状都不在这里重写 —— 走
//! [`fetch_update`] 这一个口子；设置页手动「重新检查」也调同一个口子。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter as _};

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime::{app_state, net, release_info, source};

/// 前端监听的事件名（顶层只挂一次监听）。
pub const UPDATE_AVAILABLE_EVENT: &str = "software-update-available";

/// 本会话是否已经查过（`true` 之前的那次不论成功失败都算一次成功的"不再打扰"）。
static CHECKED: AtomicBool = AtomicBool::new(false);

/// 推给前端的那点信息。**版本号以外都不带** —— 通知只负责"有新版、点我过去"，
/// 详情（说明 / 下载地址 / 更新按钮）由设置页「软件更新」那一块自己读。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvailable {
    pub version: String,
}

/// 取**正式版**清单内容（自动通知与标题栏供数的唯一取数口子）。
///
/// 地址从 Source Manifest 的 `release_url()` 来（不许从 base_url 上跳一级去猜）。
/// ★ 这里**只读 `ResourceRef::Release`**，不碰 preview：老客户端与新版客户端
///   的自动通知都只能是正式版，结构上就不可能把预览版推给人。
pub fn fetch_stable_info(root: &Path) -> Result<release_info::ReleaseInfo, AppError> {
    let resolved = source::resolve_source(root)?;
    let release_url = resolved.release_url()?;
    let bytes = net::get_release(&release_url)?;
    release_info::parse(&bytes)
}

/// 取一份「有没有新版本」的结论（正式版，自动通知与标题栏共用这一处）。
pub fn fetch_update(root: &Path) -> Result<release_info::SoftwareUpdate, AppError> {
    Ok(release_info::to_update(&fetch_stable_info(root)?))
}

/// 由两份清单拼出**用户主动检查**时并列展示的候选（纯函数，判据不联网）。
///
/// - 正式版永远排第一（作者：正式版优先排序）；
/// - **只摆比当前构建新的**（[`release_info::is_newer_version`]）—— 旧版本一张卡都不出现，
///   没有更新的正式版时那一栏就空着（2026-11-06 作者："不要显示旧版"）；
/// - 预览版与正式版同版本号 = 占位清单（"当前没有预览版"），不摆第二张卡。
pub fn candidates_from_infos(
    stable: &release_info::ReleaseInfo,
    preview: Option<&release_info::ReleaseInfo>,
) -> Vec<release_info::UpdateCandidate> {
    /* ★ **只摆比当前版本新的**（2026-11-06 作者："没有新的正式版就不显示出来嘛，
     * 不要显示旧版"）。装了 `0.0.10-dev` 时，正式版线上还是 `0.0.6` —— 它比当前旧，
     * 不是"有新版"，也不该给一颗"切回正式版"的按钮：回退是用户自己要做的决定，
     * 不该被摆成一个看起来像更新的选项。
     *
     * 过滤放在**后端这一处**：界面、标题栏、下载选址（`ipc::update` 按 channel 找候选）
     * 从此共用同一个结论，不会出现"卡片藏了但按钮还在"的半截状态。 */
    let mut list = Vec::new();
    if release_info::is_newer_version(&stable.version, super::structure::APP_VERSION) {
        list.push(release_info::UpdateCandidate::from_info(
            release_info::UpdateChannel::Stable,
            stable,
        ));
    }
    if let Some(p) = preview {
        if p.version.trim() != stable.version.trim()
            && release_info::is_newer_version(&p.version, super::structure::APP_VERSION)
        {
            list.push(release_info::UpdateCandidate::from_info(
                release_info::UpdateChannel::Preview,
                p,
            ));
        }
    }
    list
}

/// **用户主动点「重新检查」**才走这里：正式版 + 预览版**各取一次**
/// （一次手势最多两次 GET，不逐版本串行、不重复取正式清单）。
///
/// 返回顶层正式版结论 + 并列候选，供 `check_software_update` 一次用掉 ——
/// 免得「先 fetch_update 再 fetch_update_candidates」把正式清单取两遍。
/// 预览通道是可选能力：没声明 / 取不到 / 解析不了都不算故障，正式版结论照给。
pub fn fetch_update_bundle(
    root: &Path,
) -> Result<
    (
        release_info::SoftwareUpdate,
        Vec<release_info::UpdateCandidate>,
    ),
    AppError,
> {
    let resolved = source::resolve_source(root)?;
    let stable_url = resolved.release_url()?;
    let stable = release_info::parse(&net::get_release(&stable_url)?)?;

    let preview = match resolved.release_preview_url()? {
        None => None,
        Some(url) => match net::get_release(&url) {
            Ok(bytes) => release_info::parse(&bytes).ok(),
            Err(e) => {
                tracing::debug!("预览清单这次没取到（不算故障）：{}", e.message);
                None
            }
        },
    };
    Ok((
        release_info::to_update(&stable),
        candidates_from_infos(&stable, preview.as_ref()),
    ))
}

/// 只要候选集时用这个（下载按通道挑资产）。
pub fn fetch_update_candidates(
    root: &Path,
) -> Result<Vec<release_info::UpdateCandidate>, AppError> {
    Ok(fetch_update_bundle(root)?.1)
}

/// **自动通知的纯判定**：只吃正式版清单，预览版在类型上就进不来。
///
/// 再加一道保险：即便有人把预览清单误塞进正式版入口，预发布版也不会
/// 触发事件（[`release_info::compare`] 已保证预发布不提醒正式版用户）。
pub fn auto_notification(
    stable: &release_info::ReleaseInfo,
    current: &str,
) -> Option<UpdateAvailable> {
    let u = release_info::to_update_for(stable, current);
    if u.has_update {
        Some(UpdateAvailable {
            version: u.latest_version,
        })
    } else {
        None
    }
}

/// 拉目录成功之后调用：**能查就查，不能查就当没这回事**。
///
/// 全程不 `unwrap`、不向上抛；调用方不必处理返回值。
pub fn maybe_check_after_catalog(app: &AppHandle) {
    /* ① 开关：盘上"没选过"= 开（解释权在 app_state，这里不自己猜） */
    let root = match internal_root(app) {
        Ok(root) => root,
        Err(e) => {
            tracing::debug!("自动检查更新跳过（拿不到数据根）：{}", e.message);
            return;
        }
    };
    let enabled = match app_state::auto_check_update(&root) {
        Ok(enabled) => enabled,
        Err(e) => {
            tracing::debug!("自动检查更新跳过（偏好读不回来）：{}", e.message);
            return;
        }
    };
    if !enabled {
        return;
    }

    /* ② 本会话去重：先抢名额再联网，两个窗口同时拉目录也只打一次 */
    if CHECKED.swap(true, Ordering::AcqRel) {
        return;
    }

    /* ③ 查 —— **只读正式版清单**；失败静默，且把名额放回去（离线时别把机会用掉） */
    match fetch_stable_info(&root) {
        Ok(stable) => {
            if let Some(payload) =
                auto_notification(&stable, crate::runtime::structure::APP_VERSION)
            {
                if let Err(e) = app.emit(UPDATE_AVAILABLE_EVENT, payload) {
                    tracing::warn!("自动检查更新：通知事件没发出去：{e}");
                }
            }
        }
        Err(e) => {
            tracing::debug!("自动检查更新静默跳过：{}", e.message);
            CHECKED.store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::release_info::{self, UpdateChannel};

    fn info(version: &str) -> release_info::ReleaseInfo {
        release_info::parse(
            format!(
                r#"{{"releaseSchema":1,"version":"{version}","notes":"","url":"https://gitee.com/o/r/releases"}}"#
            )
            .as_bytes(),
        )
        .unwrap()
    }

    /// ★ **作者点名的独立判据**（2026-11-05）：`0.0.9` 不会收到
    /// `0.0.10-preview.1` 的自动更新通知。
    ///
    /// 旧判据：只测 `compare()` 的三段数字比较，预发布标识被吃掉，
    /// `0.0.10-preview.1` 与 `0.0.10` 都变成 `[0,0,10]`，字符串不等就算有新版。
    /// 新判据：直接钉**自动通知这条链** —— 给定正式版 0.0.6、预览版
    /// 0.0.10-preview.1、当前 0.0.9，断言不推 `software-update-available`；
    /// 同时断言主动检查的候选集里预览版仍然看得见（不是把预览版藏了）。
    /// 保住的：正式版真的更新时照样通知；主动检查仍能看到两个候选。
    /// 放弃的：「任何版本号字符串不等就算有新版」这一条旧口径。
    /// 为什么：预览版只该在用户主动打开更新页时并列出现。
    #[test]
    fn a_stable_0_0_9_is_never_notified_about_a_preview_release() {
        let stable = info("0.0.6");
        let preview = info("0.0.10-preview.1");

        // ① 自动通知入口只吃正式版清单；正式版没动 ⇒ 不推事件
        assert_eq!(
            auto_notification(&stable, "0.0.9"),
            None,
            "正式版还停在 0.0.6，0.0.9 不该收到任何通知"
        );
        // ② 就算有人把预览清单误塞进自动入口，也不许推
        assert_eq!(
            auto_notification(&preview, "0.0.9"),
            None,
            "预览版绝不能触发自动通知事件"
        );
        // ③ 源码判据：自动路径不许引用预览取数口子
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/runtime/update_check.rs"
        ))
        .unwrap();
        let start = src.find("pub fn maybe_check_after_catalog").unwrap();
        let body = &src[start..src.find("#[cfg(test)]").unwrap()];
        assert!(
            !body.contains("release_preview_url") && !body.contains("fetch_update_candidates"),
            "自动通知路径不许读预览清单（启动零网络 + 预览不主动通知）"
        );
        assert!(body.contains("fetch_stable_info"));

        // ④ **当前构建一定看不到比它旧的候选**（旧正式版 0.0.6 必然被滤掉）。
        //
        // ★ 这里不写死"几个候选"：`candidates_from_infos` 按**编译期 APP_VERSION** 判，
        //   而这个值在发版前后是变的（`0.0.10-dev` → `0.0.10-preview.2`）。早先这里写死
        //   `len()==1`，发版当天就有两条判据红在 tag 之前 —— 判据自己要跟版本无关。
        let cs = candidates_from_infos(&stable, Some(&preview));
        assert!(
            cs.iter()
                .all(|c| release_info::is_newer_version(&c.version, crate::runtime::structure::APP_VERSION)),
            "候选里不许出现比当前构建旧的：{cs:?}"
        );
        assert!(
            !cs.iter().any(|c| c.version == "0.0.6"),
            "旧正式版 0.0.6 一张卡都不摆"
        );
        // 预览版在不在，取决于当前构建与它的先后 —— 但不许"藏着更新的预览版"
        if release_info::is_newer_version("0.0.10-preview.1", crate::runtime::structure::APP_VERSION) {
            assert!(
                cs.iter()
                    .any(|c| c.channel == UpdateChannel::Preview
                        && c.version == "0.0.10-preview.1"),
                "更新的预览版必须摆得出来"
            );
        }
    }

    /// ★ **只摆比当前构建新的候选**（2026-11-06 作者："不要显示旧版"）。
    ///
    /// 现场：装了 `0.0.10-dev`，正式版线上还是 `0.0.6` —— 设置页却把它摆在
    /// "推荐 / 更新正式版"上。
    ///
    /// ★ **判据不能写死版本号**（2026-11-06 发版当天踩到）：`APP_VERSION` 是编译期常量，
    ///   发版时会从 `0.0.10-dev` 变成 `0.0.10-preview.2`，写死"0.0.10-preview.1 更新"
    ///   的判据当场变红，把发版卡在 tag 之前。所以这里用**绝对更旧 / 绝对更新**的号，
    ///   同基础那一条按 `APP_VERSION` 现算。
    #[test]
    fn only_newer_candidates_are_shown() {
        let older_stable = info("0.0.1"); // 一定比任何 0.0.x 构建旧
        let newer_stable = info("999.0.0"); // 一定更新
        let newer_preview = info("999.0.0-preview.1");

        // ① 一个都不新 → 空数组（界面退回"只有状态行、没有按钮"）
        assert!(candidates_from_infos(&older_stable, None).is_empty());

        // ② 两个都比当前新 → 都在，正式版排第一
        let cs = candidates_from_infos(&newer_stable, Some(&newer_preview));
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].channel, UpdateChannel::Stable);
        assert_eq!(cs[1].channel, UpdateChannel::Preview);

        // ③ 旧的正式版 + 更新的预览版 → 只留预览版
        let cs = candidates_from_infos(&older_stable, Some(&newer_preview));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].channel, UpdateChannel::Preview);

        // ④ 同基础的**正式版**：当前构建是预发布（dev / preview）时算更新，
        //    当前构建本身就是正式版时不算。按 APP_VERSION 现算，不写死。
        let base = crate::runtime::structure::APP_VERSION.split('-').next().unwrap();
        let same_base = info(base);
        let is_prerelease = crate::runtime::structure::APP_VERSION.contains('-');
        assert_eq!(
            !candidates_from_infos(&same_base, None).is_empty(),
            is_prerelease,
            "同基础正式版对预发布构建是更新（{}），对正式版构建不是",
            crate::runtime::structure::APP_VERSION
        );
    }

    /// 占位预览清单（与正式版同版本号）不该凭空多出一张「预览版」卡。
    #[test]
    fn a_placeholder_preview_list_does_not_create_a_card() {
        // 用比当前构建（0.0.10-dev）新的号，才验得到"占位不摆卡"而不是"旧版被滤掉"
        let stable = info("0.0.11");
        let placeholder = info("0.0.11");
        let cs = candidates_from_infos(&stable, Some(&placeholder));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].channel, UpdateChannel::Stable);
        assert_eq!(candidates_from_infos(&stable, None).len(), 1);
    }

    /// 正式版真的更新了 ⇒ 自动通知照旧推（别把功能一起改没了）。
    #[test]
    fn a_newer_stable_release_still_notifies() {
        let stable = info("0.0.11");
        let n = auto_notification(&stable, "0.0.9").expect("该通知");
        assert_eq!(n.version, "0.0.11");
    }
}
