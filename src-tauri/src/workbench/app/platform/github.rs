//! **GitHub** 平台实现 —— PR / CI 回读，走 REST API（`ureq`，SupportEase 自持 Token）。
//!
//! # 认证
//!
//! Token 从系统 Keychain 来（[`super::super::credentials`]），构造本结构时注入。
//! Header 用 `Authorization: Bearer <token>`（GitHub 推荐）+ `Accept: application/vnd.github+json`。
//!
//! # 网络
//!
//! 这是被批准的网络出口之一（见 [`super`] 模块文档）。所有 HTTP 都走 `ureq`，
//! 与 `runtime/net.rs` 同一套库（依赖已在，不新增 HTTP crate）。
//!
//! # 状态映射
//!
//! - PR state：`open`/`closed`/`merged`（`merged` 要看 `merged` 字段，`state` 只有 open/closed）
//!   → [`collapse_state`]
//! - CI：先取 `GET /repos/{o}/{r}/commits/{sha}/status`（综合 status），
//!   fallback 到 check-runs 时先压成 `pending`/`success`/`failure`/`none` → [`collapse_checks`]

use serde_json::Value;

use crate::error::AppError;

use super::{
    collapse_checks, collapse_state, ChecksSummary, Hosting, MergeMethod, RemoteReview, ReviewId,
    ReviewSpec,
};

/// GitHub API 基址（可用环境变量覆盖给企业版 / 测试假服务器）。
fn api_base() -> String {
    std::env::var("MKPSE_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".to_owned())
}

/// 一个绑定了 Token 的 GitHub 客户端。
pub struct GitHub {
    token: String,
}

impl GitHub {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    /// 认证自检：`GET /user` 拿登录名。设置页"验证凭据"用它。
    pub fn authenticate(&self) -> Result<String, AppError> {
        let v = self.get("/user")?;
        v.get("login")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::internal("GitHub /user 返回里没有 login").with_detail(v.to_string())
            })
    }

    /// GET 一个 API 路径，解析成 JSON。
    fn get(&self, path: &str) -> Result<Value, AppError> {
        let url = format!("{}{}", api_base(), path);
        // ★ 走带超时的 Agent（[`super::agent`]）—— 裸 `ureq::get` 没有超时，
        //   网络不给答案就会一直挂着（发布事务"没有尽头"的第二种死法）。
        let resp = super::agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .call()
            .map_err(transport)?;
        read_json(resp)
    }

    /// POST 一个 API 路径（带 JSON body）。
    fn post(&self, path: &str, body: Value) -> Result<Value, AppError> {
        let url = format!("{}{}", api_base(), path);
        let resp = super::agent()
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(transport)?;
        read_json(resp)
    }

    /// PUT 一个 API 路径（带 JSON body）—— 合并 PR 用它。
    fn put(&self, path: &str, body: Value) -> Result<Value, AppError> {
        let url = format!("{}{}", api_base(), path);
        let resp = super::agent()
            .put(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(transport)?;
        read_json(resp)
    }
}

/// 读一个 `ureq` 响应体并解析成 JSON（与 `runtime/net.rs` 同一条 `into_reader` 取法）。
fn read_json(resp: ureq::http::Response<ureq::Body>) -> Result<Value, AppError> {
    let mut reader = resp.into_body().into_reader();
    let mut body = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut body)
        .map_err(|e| AppError::io("读 GitHub 响应失败").with_detail(e.to_string()))?;
    serde_json::from_slice(&body)
        .map_err(|e| AppError::corrupted("GitHub 响应不是合法 JSON").with_detail(e.to_string()))
}

impl Hosting for GitHub {
    fn kind(&self) -> &'static str {
        "github"
    }

    fn create_review(&self, spec: &ReviewSpec) -> Result<RemoteReview, AppError> {
        let body = serde_json::json!({
            "title": spec.title,
            "head": spec.head,
            "base": spec.base,
            "body": spec.body,
        });
        let path = format!("/repos/{}/{}/pulls", spec.owner, spec.repo);
        let v = self.post(&path, body)?;
        review_from_json(&v)
    }

    fn get_review(&self, id: &ReviewId) -> Result<RemoteReview, AppError> {
        let mut review = review_from_json(&self.get(&format!(
            "/repos/{}/{}/pulls/{}",
            id.owner, id.repo, id.number
        ))?)?;
        // 检查状态单独一个端点：拿 head sha 的综合 status
        if let Some(sha) = self.pr_head_sha(id)? {
            review.checks = self.checks_for_sha(&id.owner, &id.repo, &sha)?;
        }
        Ok(review)
    }

    fn merge_review(&self, id: &ReviewId, method: MergeMethod) -> Result<RemoteReview, AppError> {
        // `PUT /repos/{o}/{r}/pulls/{n}/merge`，body `{"merge_method":"squash"}`。
        // 平台拒合（不可合并 / 有保护规则 / head 变了）会落成 4xx → `transport` 如实报错。
        let body = serde_json::json!({ "merge_method": method.wire_name() });
        self.put(&super::merge_path(&id.owner, &id.repo, id.number), body)?;
        // 合完**回读**一份：调用方拿到的是"合并之后的真状态"，不是我们自己拼的
        self.get_review(id)
    }

    fn find_open_review(
        &self,
        owner: &str,
        repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<RemoteReview>, AppError> {
        // `GET /repos/{o}/{r}/pulls?state=open&head={o}:{branch}&base={base}` → 数组
        let path = format!(
            "/repos/{owner}/{repo}{}",
            super::open_reviews_query(owner, head, base)
        );
        let list = self.get(&path)?;
        let Some(first) = list.as_array().and_then(|a| a.first()) else {
            return Ok(None);
        };
        let mut review = review_from_json(first)?;
        // 顺手把 CI 档位也带上（回执 / 历史要显示它）—— 与 get_review 同一取法
        if let Some(sha) = first
            .get("head")
            .and_then(|h| h.get("sha"))
            .and_then(Value::as_str)
        {
            review.checks = self.checks_for_sha(owner, repo, sha)?;
        }
        Ok(Some(review))
    }
}

impl GitHub {
    /// 取 PR 的 head sha（`GET /pulls/{n}` 的 head.sha）。
    fn pr_head_sha(&self, id: &ReviewId) -> Result<Option<String>, AppError> {
        let v = self.get(&format!(
            "/repos/{}/{}/pulls/{}",
            id.owner, id.repo, id.number
        ))?;
        Ok(v.get("head")
            .and_then(|h| h.get("sha"))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }

    /// 一个 sha 的 CI 汇总（`GET /commits/{sha}/status` 的 `state` 字段）。
    fn checks_for_sha(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<ChecksSummary, AppError> {
        let v = self.get(&format!("/repos/{owner}/{repo}/commits/{sha}/status"))?;
        // 没有 status 时 `state` 会是 "pending"（GitHub 的约定：无检查也算 pending）。
        // 看 `total_count` 为 0 才算"没有检查"。
        let total = v.get("total_count").and_then(Value::as_u64).unwrap_or(0);
        if total == 0 {
            return Ok(ChecksSummary::None);
        }
        let raw = v.get("state").and_then(Value::as_str).unwrap_or("");
        Ok(collapse_checks(raw))
    }
}

/// PR JSON → [`RemoteReview`]。`merged` 要从 `merged` 布尔字段看（`state` 只有 open/closed）。
fn review_from_json(v: &Value) -> Result<RemoteReview, AppError> {
    let number = v.get("number").and_then(Value::as_u64).ok_or_else(|| {
        AppError::corrupted("GitHub PR 响应里没有 number").with_detail(v.to_string())
    })?;
    let url = v
        .get("html_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let title = v
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let head = v
        .get("head")
        .and_then(|h| h.get("ref"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let base = v
        .get("base")
        .and_then(|b| b.get("ref"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let merged = v.get("merged").and_then(Value::as_bool).unwrap_or(false);
    let raw_state = v.get("state").and_then(Value::as_str).unwrap_or("");
    let state = if merged {
        super::ReviewState::Merged
    } else {
        collapse_state(raw_state)
    };
    Ok(RemoteReview {
        platform: "github".to_owned(),
        number,
        url,
        state,
        // 检查状态由调用方另取（create 时不查，get 时补）
        checks: ChecksSummary::Unknown,
        title,
        head,
        base,
    })
}

/// `ureq` 的错误 → `AppError`。**分类**：4xx 是"你给的东西不对"（不重试），
/// 5xx / 网络是运输类（调用方可以重试）—— 与 `runtime/net.rs` 同一取向。
fn transport(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::StatusCode(code) => {
            let kind = if (400..500).contains(&code) {
                "请求被拒"
            } else {
                "服务端错误"
            };
            AppError::io(format!("GitHub {kind}（HTTP {code}）"))
        }
        other => AppError::io("连不上 GitHub").with_detail(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PR JSON（已合并）→ `RemoteReview`：**merged 看布尔字段，不看 state**。
    #[test]
    fn a_merged_pr_maps_to_the_merged_state() {
        let v = serde_json::json!({
            "number": 12,
            "html_url": "https://github.com/o/r/pull/12",
            "title": "发布 0.0.2",
            "state": "closed",
            "merged": true,
            "head": { "ref": "publish/0.0.2", "sha": "abc" },
            "base": { "ref": "main" }
        });
        let r = review_from_json(&v).unwrap();
        assert_eq!(r.number, 12);
        assert_eq!(r.state, super::super::ReviewState::Merged);
        assert_eq!(r.head, "publish/0.0.2");
        assert_eq!(r.base, "main");
        assert_eq!(r.platform, "github");
    }

    /// 开着的 PR → Open。
    #[test]
    fn an_open_pr_maps_to_open() {
        let v = serde_json::json!({
            "number": 1, "html_url": "", "title": "t", "state": "open", "merged": false,
            "head": { "ref": "h" }, "base": { "ref": "main" }
        });
        assert_eq!(
            review_from_json(&v).unwrap().state,
            super::super::ReviewState::Open
        );
    }

    /// 缺 number = 坏响应（不猜）。
    #[test]
    fn a_response_without_a_number_is_corrupted() {
        let v = serde_json::json!({ "html_url": "x" });
        assert_eq!(
            review_from_json(&v).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }
}
