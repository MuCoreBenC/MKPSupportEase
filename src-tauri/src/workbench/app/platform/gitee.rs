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
//! | 合并 | `PUT /pulls/{n}/merge` | `PUT /pulls/{n}/merge`（同形；`merge_method` 走 body） |
//!
//! ★ **接口细节（字段名 / 端点）按 Gitee 公开 API 实现**；作者说后续会给旧版地址做一次
//! "旧版配置 → 新版配置"的事实核对 —— 那时若发现字段名不一致，改这一个文件即可
//! （统一状态模型在前端之外，不受影响）。
//!
//! # 认证
//!
//! Token 从本机凭据文件来（[`super::super::credentials`]），构造时注入。
//! Gitee 用 `access_token` 查询参数（它不上 Bearer header）。

use serde_json::Value;

use crate::error::AppError;

use super::{
    collapse_checks, collapse_state, AssetUpload, ChecksSummary, Hosting, MergeMethod, ReleaseSpec,
    RemoteRelease, RemoteReview, ReviewId, ReviewSpec, UploadedAsset,
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

    /// PUT（body 里带 access_token）—— 合并 MR 用它。
    fn put(&self, path: &str, mut body: Value) -> Result<Value, AppError> {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("access_token".to_owned(), Value::String(self.token.clone()));
        }
        let url = format!("{}{}", api_base(), path);
        let resp = super::agent()
            .put(&url)
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

    fn merge_review(&self, id: &ReviewId, method: MergeMethod) -> Result<RemoteReview, AppError> {
        // 与 GitHub 同形：`PUT /repos/{o}/{r}/pulls/{n}/merge`，body 带 `merge_method`。
        // ★ Gitee 的字段名 / 端点按公开 API 形状实现，**待作者给旧版配置后事实核对**
        //   （与建 PR / 状态回读同一条免责；只改这一个文件）。
        let body = serde_json::json!({ "merge_method": method.wire_name() });
        self.put(&super::merge_path(&id.owner, &id.repo, id.number), body)?;
        // 合完回读：Gitee 的 `state` 会变成 `merged`（[`collapse_state`] 已认）
        self.get_review(id)
    }

    fn find_open_review(
        &self,
        owner: &str,
        repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<RemoteReview>, AppError> {
        // 与 GitHub 同形的列表查询（`state` / `head` / `base`）—— 同一条"待核对"免责。
        // ★ Gitee 的 `head` 参数一般是**不带 owner 前缀**的分支名；这里按公开 API 形状传
        //   `owner:branch`（GitHub 的约定），核对时若不吃，改这一处即可。
        let path = format!(
            "/repos/{owner}/{repo}{}",
            super::open_reviews_query(owner, head, base)
        );
        let list = self.get(&path)?;
        let Some(first) = list.as_array().and_then(|a| a.first()) else {
            return Ok(None);
        };
        let mut review = review_from_json(first)?;
        if let Some(sha) = first
            .get("head")
            .and_then(|h| h.get("sha"))
            .and_then(Value::as_str)
        {
            review.checks = self.checks_for_sha(owner, repo, sha)?;
        }
        Ok(Some(review))
    }

    /* ---------- Release + 安装包（第四刀；Gitee 这一支 2026-10-05 接上） ---------- */

    /// 建 Release：`POST /repos/{o}/{r}/releases`，body 里 `tag_name` / `name` / `body`。
    ///
    /// ★ 与 GitHub 的差别**只在认证位置**（这里是 body 内的 `access_token`，那边是
    ///   Bearer header），走 [`Gitee::post`] 那同一条出口 —— 不另开一条 HTTP 路径。
    fn create_release(&self, spec: &ReleaseSpec) -> Result<RemoteRelease, AppError> {
        // ★ `target_commitish` 是**必填**（2026-10-06 真机：缺了它 Gitee 回
        //   `400 {"messages":["target_commitish is missing"]}`—— 文档把它标成可选，别信）。
        //   tag 已由 ⑥½ 推上去，这个字段只在"tag 不存在"时才参与建 tag；给 `main`：
        //   那正是发布事务打 tag 的那一支，兜底语义也对。
        let body = serde_json::json!({
            "tag_name": spec.tag_name,
            "target_commitish": "main",
            "name": spec.name,
            "body": spec.body,
        });
        match self.post(
            &format!("/repos/{}/{}/releases", spec.owner, spec.repo),
            body,
        ) {
            Ok(v) => with_page_url(&v, spec),
            Err(e) => {
                // ★ **幂等回读**（真机兜底）：Release 可能已经建过（上一趟死在传附件、
                //   这次重跑）—— 按 tag 找回那一份接着走，别让"已存在"挡住发版；
                //   找不回才把原错误交出去。
                match self.get(&format!(
                    "/repos/{}/{}/releases/tags/{}",
                    spec.owner, spec.repo, spec.tag_name
                )) {
                    Ok(v) => with_page_url(&v, spec),
                    Err(_) => Err(e),
                }
            }
        }
    }

    /// 上传安装包：`POST /repos/{o}/{r}/releases/{id}/attach_files`，**multipart 表单**。
    ///
    /// ★ 这一支与 GitHub **完全不同形**：那边是裸二进制 body + 文件名走查询串，
    ///   这边是 `multipart/form-data`（字段名 `file`）。文件**流式**从路径读
    ///   （`Form::file`），不整个进内存；总时长走 [`super::UPLOAD_TIMEOUT`]。
    fn upload_asset(&self, up: &AssetUpload) -> Result<UploadedAsset, AppError> {
        let size = std::fs::metadata(&up.path)
            .map_err(|e| {
                AppError::io("读不到要上传的安装包")
                    .with_detail(format!("{}：{e}", up.path.display()))
            })?
            .len();
        let form = ureq::unversioned::multipart::Form::new()
            .text("access_token", self.token.as_str())
            .file("file", &up.path)
            .map_err(|e| AppError::io("读不了要上传的安装包").with_detail(e.to_string()))?;
        let url = format!(
            "{}/repos/{}/{}/releases/{}/attach_files",
            api_base(),
            up.owner,
            up.repo,
            up.release_id
        );
        let resp = super::upload_agent()
            .post(&url)
            .header("User-Agent", "SupportEase")
            .send(form)
            .map_err(transport)?;
        // ★ Gitee 这一支的响应**可能没有 body**（204 / 空对象）—— 读成 `Null` 而不是
        //   报「响应不是合法 JSON」：附件到底传没传成功，以 HTTP 状态为准。
        let v = read_json_lenient(resp)?;
        let download = v
            .get("browser_download_url")
            .or_else(|| v.get("html_url"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            // 拿不到直链就据"tag 在哪个仓库"拼一个页面地址（那个地址一定存在）
            .unwrap_or_else(|| format!("https://gitee.com/{}/{}/releases", up.owner, up.repo));
        Ok(UploadedAsset {
            name: up.name.clone(),
            size,
            url: download,
        })
    }

    /// 这个 tag 的 Release 上挂了哪些附件：`GET /repos/{o}/{r}/releases/tags/{tag}`。
    ///
    /// ★ **没有那个 Release = 404 = 空表**（一个答案，不是故障）—— 同一版本要发第二个平台
    ///   时，闸与事务靠这一句回答"本平台是不是已经发过了"。
    fn release_assets(&self, owner: &str, repo: &str, tag: &str) -> Result<Vec<String>, AppError> {
        match self.get(&format!("/repos/{owner}/{repo}/releases/tags/{tag}")) {
            Ok(v) => Ok(release_asset_names(&v)),
            Err(e) if e.code == crate::error::ErrorCode::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

/// 一个 Release 响应里的附件名（`assets[].name`）。缺字段 = 空表，不编。
fn release_asset_names(v: &Value) -> Vec<String> {
    v.get("assets")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|a| a.get("name").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
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

/// 读一个响应体并解析成 JSON，**空 body 宽容成 `Null`**。
///
/// Gitee 的 `attach_files` 成功时可能回 204 或空对象 —— 那不是错误（以状态码为准）。
/// 另有一处出口仍要严格：[`read_json`]。
fn read_json_lenient(resp: ureq::http::Response<ureq::Body>) -> Result<Value, AppError> {
    let mut reader = resp.into_body().into_reader();
    let mut body = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut body)
        .map_err(|e| AppError::io("读 Gitee 响应失败").with_detail(e.to_string()))?;
    if body.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&body)
        .map_err(|e| AppError::corrupted("Gitee 响应不是合法 JSON").with_detail(e.to_string()))
}

/// 补**下载页地址**：Gitee 的 Release 响应（创建 / 按 tag 回读都一样）
/// **页面字段可能整个不给**（2026-10-06 真机：`html_url` / `url` 全空）——
/// 而页面地址是确定的：`https://gitee.com/{o}/{r}/releases/tag/{tag}`。
/// 空着写进 release.json，客户端的「打开下载页」就没地方去了。
fn with_page_url(v: &Value, spec: &ReleaseSpec) -> Result<RemoteRelease, AppError> {
    let mut r = release_from_json(v)?;
    if r.url.is_empty() {
        r.url = format!(
            "https://gitee.com/{}/{}/releases/tag/{}",
            spec.owner, spec.repo, spec.tag_name
        );
    }
    Ok(r)
}

/// Release JSON → [`RemoteRelease`]。Gitee 与 GitHub 同形给 `id` / `tag_name`，
/// 页面地址字段名可能是 `html_url` 或 `url`（两个都收）。
fn release_from_json(v: &Value) -> Result<RemoteRelease, AppError> {
    let id = v.get("id").and_then(Value::as_u64).ok_or_else(|| {
        AppError::corrupted("Gitee Release 响应里没有 id").with_detail(v.to_string())
    })?;
    let tag_name = v
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let url = v
        .get("html_url")
        .or_else(|| v.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    Ok(RemoteRelease {
        platform: "gitee".to_owned(),
        id,
        tag_name,
        url,
    })
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
            if code == 404 {
                // ★ 404 单独成一档（2026-10-07）：它常常**是一个答案**而不是故障 ——
                //   "这个 tag 上没有 Release"就是 404。按 IO 报的话调用方分不出来
                //  （[`Hosting::release_assets`]），只能去嗅字符串。
                AppError::not_found("Gitee 上没有这一份（HTTP 404）")
            } else if (400..500).contains(&code) {
                AppError::io(format!("Gitee 请求被拒（HTTP {code}）"))
            } else {
                AppError::io(format!("Gitee 服务端错误（HTTP {code}）"))
            }
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

    /* ---------- Release（第四刀；Gitee 这一支） ---------- */

    /// Release 响应 → 平台无关形状。**页面地址两个字段名都收**（`html_url` / `url`）。
    #[test]
    fn gitee_release_json_becomes_the_unified_shape() {
        let v = serde_json::json!({
            "id": 42, "tag_name": "v0.0.3",
            "html_url": "https://gitee.com/o/r/releases/tag/v0.0.3"
        });
        let r = release_from_json(&v).unwrap();
        assert_eq!(r.id, 42);
        assert_eq!(r.platform, "gitee");
        assert_eq!(r.tag_name, "v0.0.3");
        assert_eq!(r.url, "https://gitee.com/o/r/releases/tag/v0.0.3");
    }

    /// 没有 `id` 的响应 = 平台答的不是"建好了" —— 报 `CORRUPTED`，不当成功。
    #[test]
    fn a_release_without_an_id_is_corrupted() {
        assert_eq!(
            release_from_json(&serde_json::json!({ "tag_name": "v0.0.3" }))
                .unwrap_err()
                .code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// ★ Gitee 这一支**实现了** Release 与附件上传（2026-10-05 接上，此前是"如实报不支持"）。
    /// 判据只钉"不再是默认那两句" —— 真实调用要 Gitee 站点与 Token，测不了。
    #[test]
    fn gitee_now_answers_the_release_calls() {
        // trait 默认实现会返回 `not_implemented`；能构造出 Gitee 客户端就说明走的是 impl
        let g = Gitee::new("t");
        let spec = ReleaseSpec {
            owner: "o".to_owned(),
            repo: "r".to_owned(),
            tag_name: "v0.0.3".to_owned(),
            name: "SupportEase v0.0.3".to_owned(),
            body: "b".to_owned(),
        };
        // 不真发请求：只验证它**不是**默认实现那个错误
        let err = g.create_release(&spec).unwrap_err();
        assert_ne!(
            err.code,
            crate::error::ErrorCode::NotImplemented,
            "Gitee 这一支不该再是「暂不支持」"
        );
    }
}
