//! **只读探针**：核对适配器仍然读得懂真接口的几条事实。
//!
//! 只查三条 —— 它们错了会让发布**静默地**发错东西（不是"第三方 API 没测全"）：
//!
//! 1. Gitee 对"这个 tag 上没有 Release"的返回形状（回了 `null` 而不是 404）；
//! 2. GitHub 的暂存草稿能不能被按 tag 找回来（草稿不创建 tag，`/releases/tags` 会 404）；
//! 3. 两边资产的身份字段齐不齐（Gitee 给 size 不给摘要；GitHub 两样都给）。
//!
//! **零写入**：不建、不改、不删任何远端对象，也不碰清单与 tag。

use std::path::Path;

use crate::error::AppError;

use super::platforms::staging_tag_of;
use super::{ports, short, version};

/// 一条探测的结论（实测值原样带上，便于对照）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeLine {
    pub check: String,
    pub measured: String,
    pub ok: bool,
}

/// 跑三条只读探测。**任一不合预期就返回 `ok=false`**，由调用方决定退出码。
pub fn run(app_dir: &Path, repo_root: &Path) -> Result<Vec<ProbeLine>, AppError> {
    let p = ports(app_dir)?;
    let version = version::app_version(repo_root)?;
    let staging = staging_tag_of(&version);
    let mut out = Vec::new();

    /* ① Gitee：一个绝不存在的 tag 必须是 None，而不是错误 */
    let ghost = format!("probe-nonexistent-{}", short(&"0".repeat(40)));
    let got = p
        .release_hosting
        .find_release(&p.release.owner, &p.release.repo, &ghost)?;
    out.push(ProbeLine {
        check: "Gitee 缺 Release 时返回 None".to_owned(),
        measured: match &got {
            None => "None".to_owned(),
            Some(v) => format!("Some(id={}, tag={})", v.id, v.tag_name),
        },
        ok: got.is_none(),
    });

    /* ② GitHub：暂存草稿按 tag 能否找回来（草稿不建 tag，端点会 404，适配器要退回列表） */
    let draft = p
        .code_hosting
        .find_release(&p.code.owner, &p.code.repo, &staging)?;
    out.push(ProbeLine {
        check: format!("GitHub 能按 tag 找到暂存面 {staging}"),
        measured: match &draft {
            None => "None（这一代还没冻结，属正常）".to_owned(),
            Some(v) => format!("draft={} id={}", v.draft, v.id),
        },
        ok: draft.as_ref().map(|v| v.draft).unwrap_or(true),
    });

    /* ③ 资产身份字段：形状变了就会把"读不懂"当成"同字节"或"没有"。
    暂存面被清理过（这一代已封版）就看正式面 —— 两边的字段形状是同一套。 */
    let formal = super::platforms::tag_of(&version);

    for (label, hosting, owner, repo) in [
        (
            "Gitee",
            p.release_hosting.as_ref(),
            p.release.owner.as_str(),
            p.release.repo.as_str(),
        ),
        (
            "GitHub",
            p.code_hosting.as_ref(),
            p.code.owner.as_str(),
            p.code.repo.as_str(),
        ),
    ] {
        let mut assets = hosting.assets(owner, repo, &staging)?;
        let which = if assets.is_empty() {
            assets = hosting.assets(owner, repo, &formal)?;
            formal.clone()
        } else {
            staging.clone()
        };
        if assets.is_empty() {
            out.push(ProbeLine {
                check: format!("{label} 暂存面资产身份"),
                measured: format!("{which} 上还没有资产（这一代还没构建完，属正常）"),
                ok: true,
            });
            continue;
        }
        for a in &assets {
            let size_ok = a.size.is_some();
            let sha = match &a.sha256 {
                Some(s) => format!("sha256={}", &s[..s.len().min(12)]),
                None => "sha256=(平台不提供)".to_owned(),
            };
            out.push(ProbeLine {
                check: format!("{label} {which} {}", a.name),
                measured: format!("size={:?} {sha} id={:?}", a.size, a.id),
                /* Gitee 不提供摘要属正常；**大小读不到**才是不正常（重跑会把旧包当同字节） */
                ok: size_ok,
            });
        }
    }

    Ok(out)
}
