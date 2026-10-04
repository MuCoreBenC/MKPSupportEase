//! **Gitee** 平台实现 —— PR / MR 与检查状态，走 Gitee OpenAPI（`ureq`，SupportEase 自持 Token）。
//!
//! # 与 GitHub 的差异（在 impl 内消化，前端看不到）
//!
//! | | GitHub | Gitee |
//! |---|---|---|
//! | 名字 | Pull Request | Pull Request（网页叫"Pull Request"，API 也叫 `pulls`） |
//! | 认证 | `Authorization: Bearer` | `access_token=<token>` 查询参数 |
//! | 建 PR | `POST /repos/{o}/{r}/pulls` | `POST /repos/{o}/{r}/pulls` |
//! | 状态字段 | `state` + `merged` 布尔 | `state`：`open`/`merged`/`closed` |
//! | 检查 | `/commits/{sha}/status` | `/repos/{o}/{r}/commits/{sha}/status`（同形） |
//!
//! ★ **接口细节（字段名 / 端点）按 Gitee 公开 API 实现**；作者说后续会给旧版地址做一次
//! "旧版配置 → 新版配置"的事实核对 —— 那时若发现字段名不一致，改这一个文件即可
//! （统一状态模型在前端之外，不受影响）。
//!
//! # 认证
//!
//! Token 从系统 Keychain 来（[`super::super::credentials`]），构造时注入。
//! Gitee 用 `access_token` 查询参数（它不上 Bearer header）。

use serde_json::Value;

use crate::error::AppError;

use super::{
    collapse_checks, collapse_state, ChecksSummary, Hosting, RemoteReview, ReviewId, ReviewSpec,
};

fn api_base() -> String {
    std::env::var("MKPSE_GITEE_API").unwrap_or_else(|_| "https://gitee.com/api/v5".to_owned())
}

/// 一个绑定了 Token 的 Gitee 客户端。
pub struct Gitee {
    token: String,
}

impl Gitee {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    /// 认证自检：`GET /user` 拿登录名。设置页"验证凭据"用它。
    pub fn authenticate(&self) -> Result<String, AppError> {
        let v = self.get("/user")?;
        v.get("login")
            .or_else(|| v.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::internal("Gitee /user 返回里没有用户名").with_detail(v.to_string())
            })
    }

    /// GET（Gitee 用 `access_token` 查询参数）。
    fn get(&self, path: &str) -> Result<Value, AppError> {
        let url = format!("{}{}", api_base(), path);
        let sep = if path.contains('?') { '&' } else { '?' };
        let full = format!("{url}{sep}access_token={}", self.token);
        // ★ 走带超时的 Agent（[`super::agent`]）—— 与 GitHub 同一条纪律：
        //   裸 `ureq::get` 没有超时，网络不给答案就会一直挂着。
        let resp = super::agent()
            .get(&full)
            .header("User-Agent", "SupportEase")
            .call()
            .map_err(transport)?;
        read_json(resp)
    }

    /// POST（body 里带 access_token 与各字段）。
    fn post(&self, path: &str, mut body: Value) -> Result<Value, AppError> {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("access_token".to_owned(), Value::String(self.token.clone()));
        }
        let url = format!("{}{}", api_base(), path);
        let resp = super::agent()
            .post(&url)
            .header("User-Agent", "SupportEase")
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(transport)?;
        read_json(resp)
    }
}

/// 读一个 `ureq` 响应体并解析成 JSON。
fn read_json(resp: ureq::http::Response<ureq::Body>) -> Result<Value, AppError> {
    let mut reader = resp.into_body().into_reader();
    let mut body = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut body)
        .map_err(|e| AppError::io("读 Gitee 响应失败").with_detail(e.to_string()))?;
    serde_json::from_slice(&body)
        .map_err(|e| AppError::corrupted("Gitee 响应不是合法 JSON").with_detail(e.to_string()))
}

impl Hosting for Gitee {
    fn kind(&self) -> &'static str {
        "gitee"
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
        let v = self.get(&format!(
            "/repos/{}/{}/pulls/{}",
            id.owner, id.repo, id.number
        ))?;
        let mut review = review_from_json(&v)?;
        if let Some(sha) = v
            .get("head")
            .and_then(|h| h.get("sha"))
            .and_then(Value::as_str)
        {
            review.checks = self.checks_for_sha(&id.owner, &id.repo, sha)?;
        }
        Ok(review)
    }
}

impl Gitee {
    fn checks_for_sha(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<ChecksSummary, AppError> {
        let v = self.get(&format!("/repos/{owner}/{repo}/commits/{sha}/status"))?;
        // Gitee 与 GitHub 同形：total_count + state
        let total = v.get("total_count").and_then(Value::as_u64).unwrap_or(0);
        if total == 0 {
            return Ok(ChecksSummary::None);
        }
        let raw = v.get("state").and_then(Value::as_str).unwrap_or("");
        Ok(collapse_checks(raw))
    }
}

/// PR JSON → [`RemoteReview`]。Gitee 的 `state` 直接给 `open`/`merged`/`closed`。
fn review_from_json(v: &Value) -> Result<RemoteReview, AppError> {
    let number = v.get("number").and_then(Value::as_u64).ok_or_else(|| {
        AppError::corrupted("Gitee PR 响应里没有 number").with_detail(v.to_string())
    })?;
    let url = v
        .get("html_url")
        .or_else(|| v.get("url"))
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
    let raw_state = v.get("state").and_then(Value::as_str).unwrap_or("");
    Ok(RemoteReview {
        platform: "gitee".to_owned(),
        number,
        url,
        state: collapse_state(raw_state),
        checks: ChecksSummary::Unknown,
        title,
        head,
        base,
    })
}

fn transport(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::StatusCode(code) => {
            let kind = if (400..500).contains(&code) {
                "请求被拒"
            } else {
                "服务端错误"
            };
            AppError::io(format!("Gitee {kind}（HTTP {code}）"))
        }
        other => AppError::io("连不上 Gitee").with_detail(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gitee 的 `state` 直接是 `open`/`merged`/`closed` → 统一档。
    #[test]
    fn gitee_states_collapse_to_the_unified_model() {
        let open = serde_json::json!({
            "number": 5, "html_url": "https://gitee.com/o/r/pulls/5", "title": "t",
            "state": "open", "head": { "ref": "h" }, "base": { "ref": "master" }
        });
        assert_eq!(
            review_from_json(&open).unwrap().state,
            super::super::ReviewState::Open
        );

        let merged = serde_json::json!({
            "number": 5, "title": "t", "state": "merged",
            "head": { "ref": "h" }, "base": { "ref": "master" }
        });
        assert_eq!(
            review_from_json(&merged).unwrap().state,
            super::super::ReviewState::Merged
        );
    }

    /// Gitee 的 url 字段名可能是 `url` 而不是 `html_url` —— 两个都收。
    #[test]
    fn gitee_accepts_url_or_html_url() {
        let v = serde_json::json!({
            "number": 1, "url": "https://gitee.com/o/r/pulls/1", "title": "t",
            "state": "open", "head": { "ref": "h" }, "base": { "ref": "m" }
        });
        assert_eq!(
            review_from_json(&v).unwrap().url,
            "https://gitee.com/o/r/pulls/1"
        );
    }

    #[test]
    fn a_response_without_a_number_is_corrupted() {
        let v = serde_json::json!({ "title": "t" });
        assert_eq!(
            review_from_json(&v).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }
}
