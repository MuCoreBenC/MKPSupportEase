//! **GitHub Actions 通道** —— 只做三件事：按冻结的源码 SHA 触发一次构建、问它跑到哪、
//! 把产物（artifact）取回来。
//!
//! # 为什么构建在 CI 上跑
//!
//! macOS 上打不出 Windows 的 NSIS 包（`makensis`、MSVC 那一套只在 Windows 上）。
//! 但**构建权在平台、发布权在内核**：这里只负责把「某一笔提交编出来的包」取回来，
//! 建 Release / 上传 / 打 tag 一行都不做。
//!
//! # 与 `Hosting` 的分工
//!
//! - `Hosting`（[`super`]）：PR、Release、资产 —— 对外那一面；
//! - 本模块：`workflow_dispatch` → 轮询 → artifact 下载 —— 构建那一面。
//!
//! 两者共用同一枚 GitHub Token（`repo` scope 就够读 Actions 产物）。
//!
//! # 认证与重定向
//!
//! artifact 的下载地址是 GitHub 签好的 Azure Blob 直链（302 过去）。
//! **跨宿主不能带 `Authorization`** —— 带了会被对方拒。所以下载走一个显式设置
//! `RedirectAuthHeaders::SameHost` 的 Agent，不让令牌跟到别人的域名上。

use std::path::Path;

use serde_json::Value;

use crate::error::AppError;

fn api_base() -> String {
    std::env::var("MKPSE_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".to_owned())
}

/// 一次工作流运行的快照（只取发布内核要判的那几格）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    pub id: u64,
    /// `queued` / `in_progress` / `completed`
    pub status: String,
    /// `success` / `failure` / `cancelled` ……（`status != completed` 时是 `None`）
    pub conclusion: Option<String>,
    pub head_sha: String,
    pub created_at: String,
    pub url: String,
}

impl WorkflowRun {
    pub fn finished(&self) -> bool {
        self.status == "completed"
    }
    pub fn ok(&self) -> bool {
        self.conclusion.as_deref() == Some("success")
    }
}

/// 一次运行挂着的产物清单。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub id: u64,
    pub name: String,
    pub size_in_bytes: u64,
    pub expired: bool,
}

/// 一个绑定了 Token 的 Actions 客户端。
pub struct Actions {
    token: String,
}

impl Actions {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    fn get(&self, path: &str) -> Result<Value, AppError> {
        let url = format!("{}{}", api_base(), path);
        let resp = super::agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .call()
            .map_err(transport)?;
        read_json(resp)
    }

    /// 触发一次工作流。`ref` 是平台要 checkout 的分支（`main`），
    /// 真正的构建提交由 `inputs.source_sha` 决定 —— 两者分开是刻意的：
    /// 工作流文件本身必须取自一条存在的分支，而**被构建的代码**必须是我们冻结的那一笔。
    pub fn dispatch(
        &self,
        owner: &str,
        repo: &str,
        workflow_file: &str,
        git_ref: &str,
        inputs: &[(&str, String)],
    ) -> Result<(), AppError> {
        let mut body = serde_json::Map::new();
        body.insert("ref".to_owned(), Value::String(git_ref.to_owned()));
        let mut map = serde_json::Map::new();
        for (k, v) in inputs {
            map.insert((*k).to_owned(), Value::String(v.clone()));
        }
        body.insert("inputs".to_owned(), Value::Object(map));
        let url = format!(
            "{}/repos/{}/{}/actions/workflows/{}/dispatches",
            api_base(),
            owner,
            repo,
            workflow_file
        );
        super::agent()
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .header("Content-Type", "application/json")
            .send(Value::Object(body).to_string())
            .map_err(transport)?;
        Ok(())
    }

    /// 这条工作流最近的一批运行（新的在前）。
    ///
    /// ★ 按 `event=workflow_dispatch` 过滤：tag push 与分支 push 那一类不是这一趟的。
    pub fn runs(
        &self,
        owner: &str,
        repo: &str,
        workflow_file: &str,
    ) -> Result<Vec<WorkflowRun>, AppError> {
        let v = self.get(&format!(
            "/repos/{owner}/{repo}/actions/workflows/{workflow_file}/runs?event=workflow_dispatch&per_page=50"
        ))?;
        Ok(v.get("workflow_runs")
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(run_of).collect())
            .unwrap_or_default())
    }

    pub fn run(&self, owner: &str, repo: &str, run_id: u64) -> Result<WorkflowRun, AppError> {
        let v = self.get(&format!("/repos/{owner}/{repo}/actions/runs/{run_id}"))?;
        run_of(&v).ok_or_else(|| {
            AppError::corrupted("GitHub Actions 运行响应里没有 id").with_detail(v.to_string())
        })
    }

    pub fn artifacts(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<Vec<ArtifactRef>, AppError> {
        let v = self.get(&format!(
            "/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts"
        ))?;
        Ok(v.get("artifacts")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|a| {
                        Some(ArtifactRef {
                            id: a.get("id").and_then(Value::as_u64)?,
                            name: a.get("name").and_then(Value::as_str)?.to_owned(),
                            size_in_bytes: a
                                .get("size_in_bytes")
                                .and_then(Value::as_u64)
                                .unwrap_or_default(),
                            expired: a.get("expired").and_then(Value::as_bool).unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// 把一个 artifact 的 zip 下到本地，返回字节数。
    ///
    /// ★ zip 里是工作流上传的那几个文件（`upload-artifact` 会再套一层目录），
    /// 解包与「哪一份是安装包」由调用方判 —— 这里只管把字节搬回来。
    pub fn download_artifact(
        &self,
        owner: &str,
        repo: &str,
        artifact_id: u64,
        dest: &Path,
    ) -> Result<u64, AppError> {
        let url = format!(
            "{}/repos/{}/{}/actions/artifacts/{}/zip",
            api_base(),
            owner,
            repo,
            artifact_id
        );
        /* 302 → Azure Blob：**不把令牌带过去**（`SameHost` 只在同一宿主上重发认证头）。 */
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(super::UPLOAD_TIMEOUT))
            .timeout_connect(Some(super::API_CONNECT_TIMEOUT))
            .redirect_auth_headers(ureq::config::RedirectAuthHeaders::SameHost)
            .build()
            .into();
        let resp = agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "SupportEase")
            .call()
            .map_err(transport)?;
        let mut reader = resp.into_body().into_reader();
        let mut written: u64 = 0;
        crate::fsx::atomic::atomic_write_stream(dest, |file| {
            written = std::io::copy(&mut reader, file)
                .map_err(|e| AppError::io("下载产物失败").with_detail(e.to_string()))?;
            Ok(())
        })?;
        Ok(written)
    }
}

fn run_of(v: &Value) -> Option<WorkflowRun> {
    Some(WorkflowRun {
        id: v.get("id").and_then(Value::as_u64)?,
        status: v
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        conclusion: v
            .get("conclusion")
            .and_then(Value::as_str)
            .map(str::to_owned),
        head_sha: v
            .get("head_sha")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        created_at: v
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        url: v
            .get("html_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    })
}

fn read_json(resp: ureq::http::Response<ureq::Body>) -> Result<Value, AppError> {
    let mut reader = resp.into_body().into_reader();
    let mut body = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut body)
        .map_err(|e| AppError::io("读 GitHub Actions 响应失败").with_detail(e.to_string()))?;
    serde_json::from_slice(&body).map_err(|e| {
        AppError::corrupted("GitHub Actions 响应不是合法 JSON").with_detail(e.to_string())
    })
}

fn transport(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::StatusCode(code) => {
            if code == 404 {
                AppError::not_found("GitHub Actions 上没有这一份（HTTP 404）")
            } else if (400..500).contains(&code) {
                AppError::io(format!("GitHub Actions 请求被拒（HTTP {code}）"))
            } else {
                AppError::io(format!("GitHub Actions 服务端错误（HTTP {code}）"))
            }
        }
        other => AppError::io("连不上 GitHub Actions").with_detail(other.to_string()),
    }
}
