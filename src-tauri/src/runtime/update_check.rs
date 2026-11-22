//! **自动检查软件更新**（2026-11-05）：挂在「客户端成功拉到云端目录」之后，顺路查一次
//! [`crate::runtime::release_info`]。
//!
//! **查哪一路由用户选的渠道决定**（2026-11-20，`app_state::update_channel`）：
//! 正式版只读 `release.json`（预发布在结构上进不来）；测试版读候选集里的预览清单。
//! 不静默下载 —— 这一层只推一条"有新版"的通知，装不装由人点。
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

use super::update_state::{self, CheckRecord, CheckTrigger, UpdateSnapshot};

/// 更新状态的**唯一事件**：整份快照（业务事实）。
///
/// 事件只有两个：这一个（业务事实）与 `software-update-progress`（进程态）。
pub const UPDATE_STATE_EVENT: &str = "software-update-state";

/// 本会话是否已经查过（`true` 之前的那次不论成功失败都算一次成功的"不再打扰"）。
static CHECKED: AtomicBool = AtomicBool::new(false);

/// 换渠道之后**放回名额**：下一次拉目录立刻按新渠道再查一次。
///
/// 不放回的话，切到测试版要重启客户端才可能收到通知 —— 用户会以为"切了没用"。
pub fn reset_session_check() {
    CHECKED.store(false, Ordering::Release);
}

/// 取**正式版**清单内容（设置页与第一次检查的唯一取数口子）。
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
///
/// 资产地址按**用户当前选的源**挑：同一份清单里 `sources.gitee` / `sources.github`
/// 各有一条；选 GitHub 就下 GitHub 的包，不再退回“打开 Gitee 下载页”。
pub fn fetch_update(root: &Path) -> Result<release_info::SoftwareUpdate, AppError> {
    let resolved = source::resolve_source(root)?;
    let source_name = resolved.mode().wire();
    let release_url = resolved.release_url()?;
    let info = release_info::parse(&net::get_release(&release_url)?)?;
    Ok(release_info::to_update_for_source(
        &info,
        super::structure::APP_VERSION,
        Some(source_name),
    ))
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
    candidates_from_infos_with_source(stable, preview, None)
}

/// 同上，但资产地址按用户当前源挑（`gitee` / `github`）。
pub fn candidates_from_infos_with_source(
    stable: &release_info::ReleaseInfo,
    preview: Option<&release_info::ReleaseInfo>,
    source: Option<&str>,
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
        list.push(release_info::UpdateCandidate::from_info_with_source(
            release_info::UpdateChannel::Stable,
            stable,
            source,
        ));
    }
    if let Some(p) = preview {
        if p.version.trim() != stable.version.trim()
            && release_info::is_newer_version(&p.version, super::structure::APP_VERSION)
        {
            list.push(release_info::UpdateCandidate::from_info_with_source(
                release_info::UpdateChannel::Preview,
                p,
                source,
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
    let source_name = resolved.mode().wire();
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
        release_info::to_update_for_source(
            &stable,
            super::structure::APP_VERSION,
            Some(source_name),
        ),
        candidates_from_infos_with_source(&stable, preview.as_ref(), Some(source_name)),
    ))
}

/// 只要候选集时用这个（下载按通道挑资产）。
pub fn fetch_update_candidates(
    root: &Path,
) -> Result<Vec<release_info::UpdateCandidate>, AppError> {
    Ok(fetch_update_bundle(root)?.1)
}

/// **检查事务** —— 唯一写候选集的地方（手动、自动共用这一条）。
///
/// - 成功：整份替换候选与检查记录；
/// - 失败：**只**改检查记录（`ok=false` + 原因），旧候选保留 —— 查不到不等于没有新版；
/// - 两种都落盘；失败把 `Err` 交给调用方，由它决定说不说（手动要报，自动静默）。
pub fn run_check(root: &Path, trigger: CheckTrigger) -> Result<UpdateSnapshot, AppError> {
    let mut snap = update_state::load(root)?;
    update_state::reconcile(&mut snap, crate::runtime::structure::APP_VERSION);
    let channel = app_state::update_channel(root).unwrap_or_default();
    let at = update_state::now_iso8601();
    let fetched = fetch_update_bundle(root).map(|(_stable, candidates)| candidates);
    match fetched {
        Ok(candidates) => {
            apply_check(&mut snap, Ok(candidates), trigger, channel, &at);
            update_state::save(root, &snap)?;
            Ok(snap)
        }
        Err(e) => {
            apply_check(&mut snap, Err(e.message.clone()), trigger, channel, &at);
            /* 写盘失败不该把"这次没查成"也吞掉：记日志，原错误照旧往上抛 */
            if let Err(w) = update_state::save(root, &snap) {
                tracing::warn!("检查失败的记录没写进快照：{}", w.message);
            }
            Err(e)
        }
    }
}

/// 把一次检查的结论折进快照（**纯函数**，判据直接喂它）。
///
/// 成功：整份替换候选；失败：**候选一个不动**，只把检查记录写成 `ok=false`。
/// 后者是这条链上最容易写错的一处 —— 失败若顺手清候选，"这次没查成"就变成了
/// "没有新版"，用户会以为旧版本已经发完了。
pub fn apply_check(
    snap: &mut UpdateSnapshot,
    result: Result<Vec<release_info::UpdateCandidate>, String>,
    trigger: CheckTrigger,
    channel: release_info::UpdateChannel,
    at: &str,
) -> bool {
    let before = snap.clone();
    match result {
        Ok(candidates) => {
            snap.candidates = candidates;
            snap.checked = CheckRecord {
                at: Some(at.to_owned()),
                ok: true,
                reason: None,
                trigger: Some(trigger),
                channel,
            };
        }
        Err(reason) => {
            snap.checked = CheckRecord {
                at: Some(at.to_owned()),
                ok: false,
                reason: Some(reason),
                trigger: Some(trigger),
                channel,
            };
        }
    }
    *snap != before
}

/// 忽略某一版（落盘）。**只动 `ignored`，不碰候选集**。
pub fn ignore(root: &Path, version: &str) -> Result<UpdateSnapshot, AppError> {
    let mut snap = update_state::load(root)?;
    update_state::reconcile(&mut snap, crate::runtime::structure::APP_VERSION);
    let v = version.trim();
    if !v.is_empty() {
        snap.ignored = Some(v.to_owned());
    }
    update_state::save(root, &snap)?;
    Ok(snap)
}

/// 换渠道 / 关自动检查：旧结论作废（候选清空、检查记录回到"没查过"）。
/// **不清 `ignored`** —— 用户对某一版的忽略不因为换渠道就失效。
pub fn invalidate(root: &Path) -> Result<UpdateSnapshot, AppError> {
    let mut snap = update_state::load(root)?;
    snap.candidates.clear();
    snap.checked = CheckRecord {
        at: None,
        ok: false,
        reason: None,
        trigger: None,
        channel: app_state::update_channel(root).unwrap_or_default(),
    };
    update_state::save(root, &snap)?;
    Ok(snap)
}

/// 启动时读一次：收敛版本、迁移旧账。**不联网、不发事件。**
///
/// 迁移的顺序是死的：先把该搬的搬进内存 → 落盘成功 → 才删旧的。
/// 中途断电最坏是"旧数据还在、新快照没写"，下次启动重来一遍，不会丢忽略记录。
pub fn startup(root: &Path) -> Result<UpdateSnapshot, AppError> {
    let mut snap = update_state::load(root)?;
    let mut dirty = update_state::reconcile(&mut snap, crate::runtime::structure::APP_VERSION);
    let legacy = take_legacy(root, &mut snap);
    dirty |= legacy.any();
    if dirty {
        update_state::save(root, &snap)?;
        clear_legacy(root, legacy);
    }
    Ok(snap)
}

/// 旧账里要搬的那些（**只读**，不删）。
#[derive(Default)]
struct Legacy {
    /// `run/app-state.json` 的 `pendingUpdate`（retired 旧格）
    pending: bool,
    /// `run/update-result.json`（旧安装账）
    result: bool,
}

impl Legacy {
    fn any(&self) -> bool {
        self.pending || self.result
    }
}

fn take_legacy(root: &Path, snap: &mut UpdateSnapshot) -> Legacy {
    let current = crate::runtime::structure::APP_VERSION;
    let mut legacy = Legacy::default();

    if let Ok(Some(v)) = app_state::pending_update(root) {
        let v = v.trim().to_owned();
        legacy.pending = true;
        let usable = release_info::is_newer_version(&v, current)
            && !snap.candidates.iter().any(|c| c.version == v);
        if usable {
            snap.candidates.push(release_info::UpdateCandidate {
                channel: snap.checked.channel,
                version: v,
                notes: None,
                url: None,
                asset: None,
            });
        }
    }

    if let Ok(Some(r)) = super::updater::read_result(root) {
        legacy.result = true;
        let keep = r.version.trim() == current.trim()
            || release_info::is_newer_version(&r.version, current);
        if keep && snap.last_result.is_none() {
            snap.last_result = Some(r);
        }
    }

    legacy
}

/// 新快照**已经落盘**之后才删旧数据。
fn clear_legacy(root: &Path, legacy: Legacy) {
    if legacy.pending {
        if let Err(e) = app_state::set_pending_update(root, None) {
            tracing::warn!("旧待更新账没清掉：{}", e.message);
        }
    }
    if legacy.result {
        if let Err(e) = std::fs::remove_file(super::updater::result_path(root)) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("旧安装结果没删掉：{e}");
            }
        }
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

    /* ③ 跑**同一个检查事务**（渠道由事务自己从盘上读）。成功与失败都广播整份快照：
    失败也要让界面知道"这次没查成"，而不是停在旧结论上装作刚查过。 */
    match run_check(&root, CheckTrigger::Auto) {
        Ok(snap) => emit_state(app, &snap),
        Err(e) => {
            tracing::debug!("自动检查更新静默跳过：{}", e.message);
            CHECKED.store(false, Ordering::Release);
            if let Ok(snap) = update_state::load(&root) {
                emit_state(app, &snap);
            }
        }
    }
}

/// 把整份快照推给界面（**唯一**的更新状态事件）。
fn emit_state(app: &AppHandle, snap: &UpdateSnapshot) {
    if let Err(e) = app.emit(UPDATE_STATE_EVENT, snap) {
        tracing::warn!("更新状态事件没发出去：{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::release_info::{self, UpdateChannel};
    use crate::runtime::{app_state, updater};

    fn candidate(version: &str) -> release_info::UpdateCandidate {
        release_info::UpdateCandidate {
            channel: UpdateChannel::Stable,
            version: version.to_owned(),
            notes: None,
            url: None,
            asset: None,
        }
    }

    fn info(version: &str) -> release_info::ReleaseInfo {
        release_info::parse(
            format!(
                r#"{{"releaseSchema":1,"version":"{version}","notes":"","url":"https://gitee.com/o/r/releases"}}"#
            )
            .as_bytes(),
        )
        .unwrap()
    }

    /// **预览版不许进正式版用户的候选集**：给定正式版 0.0.6、预览版
    /// 0.0.10-preview.1，当前构建看到的候选里既没有旧正式版、也没有被当成更新的预览版。
    ///
    /// 版本序的守卫在 `release_info::is_newer_version`：同基础的预发布不比正式版新，
    /// 而基础版本更高的预发布仍然是"更新的版本"（用户主动打开时看得到，只是不通知）。
    #[test]
    fn an_older_stable_and_a_preview_never_become_candidates() {
        let stable = info("0.0.6");
        let preview = info("0.0.10-preview.1");

        // ① 源码判据：自动路径只调**同一个检查事务**，不再自己取数、不再写旧账。
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/runtime/update_check.rs"
        ))
        .unwrap();
        let start = src.find("pub fn maybe_check_after_catalog").unwrap();
        let body = &src[start..src.find("#[cfg(test)]").unwrap()];
        assert!(
            body.contains("run_check"),
            "自动检查必须走统一事务，不许再自己取数"
        );
        assert!(
            body.contains("UPDATE_STATE_EVENT"),
            "成功与失败都要广播整份快照"
        );
        assert!(
            !body.contains("set_pending_update"),
            "旧账已 retired，自动路径不许再写它"
        );

        // ④ **当前构建一定看不到比它旧的候选**（旧正式版 0.0.6 必然被滤掉）。
        //
        // ★ 这里不写死"几个候选"：`candidates_from_infos` 按**编译期 APP_VERSION** 判，
        //   而这个值在发版前后是变的（`0.0.10-dev` → `0.0.10-preview.2`）。早先这里写死
        //   `len()==1`，发版当天就有两条判据红在 tag 之前 —— 判据自己要跟版本无关。
        let cs = candidates_from_infos(&stable, Some(&preview));
        assert!(
            cs.iter().all(|c| release_info::is_newer_version(
                &c.version,
                crate::runtime::structure::APP_VERSION
            )),
            "候选里不许出现比当前构建旧的：{cs:?}"
        );
        assert!(
            !cs.iter().any(|c| c.version == "0.0.6"),
            "旧正式版 0.0.6 一张卡都不摆"
        );
        // 预览版在不在，取决于当前构建与它的先后 —— 但不许"藏着更新的预览版"
        if release_info::is_newer_version(
            "0.0.10-preview.1",
            crate::runtime::structure::APP_VERSION,
        ) {
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
        let base = crate::runtime::structure::APP_VERSION
            .split('-')
            .next()
            .unwrap();
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
        // 用**绝对比当前构建新**的号：写死 0.0.12 会在发 0.0.12 那天当场变红
        // （那时它不再是"更新"，被候选过滤器滤掉，测的就不是"占位不摆卡"了）。
        let stable = info("999.0.0");
        let placeholder = info("999.0.0");
        let cs = candidates_from_infos(&stable, Some(&placeholder));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].channel, UpdateChannel::Stable);
        assert_eq!(candidates_from_infos(&stable, None).len(), 1);
    }

    /// 正式版真的更新了 ⇒ 快照里有一条待更新（胶囊与通知都由它派生，别把功能改没了）。
    #[test]
    fn a_newer_stable_release_still_produces_a_pending_candidate() {
        let stable = info("999.0.1");
        let snap = crate::runtime::update_state::UpdateSnapshot {
            candidates: candidates_from_infos(&stable, None),
            ..Default::default()
        };
        let p = crate::runtime::update_state::pending(&snap).expect("该有一条待更新");
        assert_eq!(p.version, "999.0.1");
    }

    /// 检查成功：整份替换候选，记录写 `ok=true`。
    #[test]
    fn a_successful_check_replaces_the_candidates() {
        let mut snap = UpdateSnapshot {
            candidates: vec![candidate("999.0.0")],
            ..Default::default()
        };
        let changed = apply_check(
            &mut snap,
            Ok(vec![candidate("999.0.1")]),
            CheckTrigger::Manual,
            UpdateChannel::Stable,
            "2026-11-22T00:00:00Z",
        );
        assert!(changed);
        assert_eq!(snap.candidates[0].version, "999.0.1");
        assert!(snap.checked.ok);
        assert_eq!(snap.checked.trigger, Some(CheckTrigger::Manual));
    }

    /// 检查失败：候选一个不动，记录写 `ok=false` —— 界面据此说"这次没查成"。
    #[test]
    fn a_failed_check_keeps_the_old_candidates_and_says_so() {
        let mut snap = UpdateSnapshot {
            candidates: vec![candidate("999.0.0")],
            ..Default::default()
        };
        let changed = apply_check(
            &mut snap,
            Err("连不上发布地址".to_owned()),
            CheckTrigger::Manual,
            UpdateChannel::Stable,
            "2026-11-22T00:00:00Z",
        );
        assert!(changed);
        assert_eq!(snap.candidates.len(), 1, "旧候选不许被失败擦掉");
        assert!(!snap.checked.ok, "失败不许长得像刚查过");
        assert_eq!(snap.checked.reason.as_deref(), Some("连不上发布地址"));
    }

    /// 启动时把旧账搬进快照，**搬完才删旧的**；再启动一次是幂等的。
    #[test]
    fn startup_migrates_the_old_ledgers_then_clears_them() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        let newer = "999.0.0";
        app_state::set_pending_update(d.path(), Some(newer)).unwrap();
        updater::write_result(
            d.path(),
            &updater::UpdateResult {
                version: newer.to_owned(),
                ok: true,
                reason: String::new(),
                at: "2026-11-21T00:00:00Z".to_owned(),
            },
        )
        .unwrap();

        let snap = startup(d.path()).unwrap();
        assert_eq!(snap.candidates.len(), 1);
        assert_eq!(snap.candidates[0].version, newer);
        assert_eq!(
            snap.last_result.as_ref().map(|r| r.version.as_str()),
            Some(newer)
        );
        assert!(
            app_state::pending_update(d.path()).unwrap().is_none(),
            "旧格要清掉"
        );
        assert!(!updater::result_path(d.path()).exists(), "旧结果文件要删掉");

        let again = startup(d.path()).unwrap();
        assert_eq!(again.candidates.len(), 1, "再跑一次不该多出候选");
    }
}
