//! **演练用的假远端**（只在 `#[cfg(test)]` 里编）。
//!
//! 它存在的唯一理由：让 `run_with` 的整条编排在没有网络、没有 runner 的情况下能跑完
//! —— 中断、换机器、重复执行这些事在真远端上没法反复演，而它们正是发布可靠性要证明的东西。
//!
//! 两套方言照抄真实平台**会伤害发布正确性**的那几条差别：
//!
//! | | Gitee | GitHub |
//! |---|---|---|
//! | 资产大小 | 给 | 给 |
//! | 摘要 | **不给自己算**（`None`） | 给 `sha256` |
//! | 草稿 | 没有这个概念 | 有（可见性由适配器兜底，见 `ship --probe`） |
//!
//! 故意不模拟的东西：网络超时、限流、分页。那些由真实发布那一次覆盖 —— 这里只演状态机。

/* 演练/夹具里的写盘是**测试造现场**（临时仓库、假产物），不是生产写路径；
clippy 的写盘禁列只管生产代码，这里整模块豁免并写明理由。 */
#![allow(clippy::disallowed_methods)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;

use super::platforms::{self, ArtifactFact, Platform};
use crate::workbench::app::platform::actions::WorkflowRun;
use crate::workbench::app::platform::{
    AssetUpload, ChecksSummary, Hosting, MergeMethod, ReleaseSpec, ReleaseSummary, ReleaseUpdate,
    ReleaseView, RemoteAsset, RemoteRelease, RemoteReview, ReviewId, ReviewSpec, ReviewState,
    UploadedAsset,
};

/* ---------- 假远端 ---------- */

/// 平台方言（决定"读回来的资产身份长什么样"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// Gitee：附件表给 size + id，**不给摘要**
    Gitee,
    /// GitHub：给 size + digest；草稿按 tag 查不到
    GitHub,
}

#[derive(Debug, Clone)]
struct FakeRelease {
    id: u64,
    tag: String,
    draft: bool,
    #[allow(dead_code)] // 状态里要它（"这一份是不是这一笔源码建的"），演练暂时只读 tag
    commit: Option<String>,
    assets: Vec<FakeAsset>,
}

#[derive(Debug, Clone)]
struct FakeAsset {
    id: u64,
    name: String,
    size: u64,
    sha256: String,
    /// 这一份的字节从哪儿读（下载/重传时用）
    source: Option<PathBuf>,
}

/// 演练用的远端。**内存态**，删进程就没了。
pub struct FakeHosting {
    kind: &'static str,
    dialect: Dialect,
    inner: RefCell<FakeStore>,
    /// 让"读回来的身份"缺字段：模拟平台回了个形状异常的响应
    pub hide_size: RefCell<bool>,
    /// 下一次上传直接失败（模拟传到一半断了）
    pub fail_next_upload: RefCell<bool>,
    /// 记录每一次写操作（回执、断言的依据）
    pub writes: RefCell<Vec<String>>,
}

#[derive(Default)]
struct FakeStore {
    next_id: u64,
    releases: Vec<FakeRelease>,
    reviews: Vec<ReviewRecord>,
}

struct ReviewRecord {
    number: u64,
    head: String,
    base: String,
    title: String,
    state: ReviewState,
}

impl FakeHosting {
    pub fn new(kind: &'static str, dialect: Dialect) -> Self {
        Self {
            kind,
            dialect,
            inner: RefCell::new(FakeStore {
                next_id: 100,
                ..Default::default()
            }),
            hide_size: RefCell::new(false),
            fail_next_upload: RefCell::new(false),
            writes: RefCell::new(Vec::new()),
        }
    }

    pub fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }

    fn new_id(store: &mut FakeStore) -> u64 {
        store.next_id += 1;
        store.next_id
    }

    /// 预置一份资产（演练"别的平台已经发好"这种起点）。
    pub fn seed_asset(&self, tag: &str, commit: &str, name: &str, bytes: &[u8], file: &Path) {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("建临时产物目录");
        }
        std::fs::write(file, bytes).expect("写临时产物");
        let size = bytes.len() as u64;
        let sha = super::platforms::sha256_file(file).expect("算 SHA");
        let mut store = self.inner.borrow_mut();
        let id = Self::new_id(&mut store);
        let asset = FakeAsset {
            id,
            name: name.to_owned(),
            size,
            sha256: sha,
            source: Some(file.to_path_buf()),
        };
        match store.releases.iter_mut().find(|r| r.tag == tag) {
            Some(rel) => rel.assets.push(asset),
            None => {
                let rel_id = Self::new_id(&mut store);
                store.releases.push(FakeRelease {
                    id: rel_id,
                    tag: tag.to_owned(),
                    draft: self.dialect == Dialect::GitHub,
                    commit: Some(commit.to_owned()),
                    assets: vec![asset],
                });
            }
        }
    }

    fn release_by_tag<'a>(store: &'a FakeStore, tag: &str) -> Option<&'a FakeRelease> {
        store.releases.iter().find(|r| r.tag == tag)
    }

    fn asset_view(&self, a: &FakeAsset) -> RemoteAsset {
        let hide = *self.hide_size.borrow();
        RemoteAsset {
            name: a.name.clone(),
            size: if hide { None } else { Some(a.size) },
            sha256: match self.dialect {
                Dialect::Gitee => None,
                Dialect::GitHub => Some(a.sha256.clone()),
            },
            id: Some(a.id),
            url: format!("https://fake/{}/{}", self.kind, a.name),
        }
    }
}

fn review(number: u64, spec: &ReviewSpec) -> RemoteReview {
    RemoteReview {
        platform: "github".to_owned(),
        number,
        url: format!("https://fake/pull/{number}"),
        state: ReviewState::Open,
        checks: ChecksSummary::Passed,
        title: spec.title.clone(),
        head: spec.head.clone(),
        base: spec.base.clone(),
    }
}

impl Hosting for FakeHosting {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn create_review(&self, spec: &ReviewSpec) -> Result<RemoteReview, AppError> {
        self.writes
            .borrow_mut()
            .push(format!("create_review {}", spec.head));
        let mut store = self.inner.borrow_mut();
        let number = Self::new_id(&mut store);
        store.reviews.push(ReviewRecord {
            number,
            head: spec.head.clone(),
            base: spec.base.clone(),
            title: spec.title.clone(),
            state: ReviewState::Open,
        });
        Ok(review(number, spec))
    }

    fn find_open_review(
        &self,
        _owner: &str,
        _repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<RemoteReview>, AppError> {
        let store = self.inner.borrow();
        Ok(store
            .reviews
            .iter()
            .find(|r| r.head == head && r.base == base && r.state == ReviewState::Open)
            .map(|r| RemoteReview {
                platform: "github".to_owned(),
                number: r.number,
                url: format!("https://fake/pull/{}", r.number),
                state: r.state,
                checks: ChecksSummary::Passed,
                title: r.title.clone(),
                head: r.head.clone(),
                base: r.base.clone(),
            }))
    }

    fn get_review(&self, id: &ReviewId) -> Result<RemoteReview, AppError> {
        let store = self.inner.borrow();
        store
            .reviews
            .iter()
            .find(|r| r.number == id.number)
            .map(|r| RemoteReview {
                platform: "github".to_owned(),
                number: r.number,
                url: format!("https://fake/pull/{}", r.number),
                state: r.state,
                checks: ChecksSummary::Passed,
                title: r.title.clone(),
                head: r.head.clone(),
                base: r.base.clone(),
            })
            .ok_or_else(|| AppError::not_found("没有这个 PR"))
    }

    fn merge_review(&self, id: &ReviewId, _method: MergeMethod) -> Result<RemoteReview, AppError> {
        self.writes
            .borrow_mut()
            .push(format!("merge_review #{}", id.number));
        let mut store = self.inner.borrow_mut();
        let r = store
            .reviews
            .iter_mut()
            .find(|r| r.number == id.number)
            .ok_or_else(|| AppError::not_found("没有这个 PR"))?;
        r.state = ReviewState::Merged;
        Ok(RemoteReview {
            platform: "github".to_owned(),
            number: r.number,
            url: format!("https://fake/pull/{}", r.number),
            state: r.state,
            checks: ChecksSummary::Passed,
            title: r.title.clone(),
            head: r.head.clone(),
            base: r.base.clone(),
        })
    }

    fn create_release(&self, spec: &ReleaseSpec) -> Result<RemoteRelease, AppError> {
        self.writes
            .borrow_mut()
            .push(format!("create_release {}", spec.tag_name));
        let mut store = self.inner.borrow_mut();
        if let Some(exist) = store.releases.iter().find(|r| r.tag == spec.tag_name) {
            return Ok(RemoteRelease {
                platform: self.kind.to_owned(),
                id: exist.id,
                tag_name: exist.tag.clone(),
                url: format!("https://fake/rel/{}", exist.tag),
            });
        }
        let id = Self::new_id(&mut store);
        store.releases.push(FakeRelease {
            id,
            tag: spec.tag_name.clone(),
            draft: spec.draft,
            commit: spec.target_commitish.clone(),
            assets: Vec::new(),
        });
        Ok(RemoteRelease {
            platform: self.kind.to_owned(),
            id,
            tag_name: spec.tag_name.clone(),
            url: format!("https://fake/rel/{}", spec.tag_name),
        })
    }

    fn find_release(
        &self,
        _owner: &str,
        _repo: &str,
        tag: &str,
    ) -> Result<Option<ReleaseView>, AppError> {
        let store = self.inner.borrow();
        /* 假远端站在 **Hosting trait 这一层**：草稿在适配器里已经被"按 tag 找回来"过了，
        这里就照实返回它。raw API 那条怪癖（草稿在 /releases/tags 上 404）由
        `ship --probe` 打真接口覆盖，演练不重复演它。 */
        Ok(Self::release_by_tag(&store, tag).map(|r| ReleaseView {
            platform: self.kind.to_owned(),
            id: r.id,
            tag_name: r.tag.clone(),
            name: r.tag.clone(),
            draft: r.draft,
            prerelease: false,
            url: format!("https://fake/rel/{}", r.tag),
        }))
    }

    fn update_release(&self, up: &ReleaseUpdate) -> Result<RemoteRelease, AppError> {
        self.writes
            .borrow_mut()
            .push(format!("update_release {}", up.release_id));
        let mut store = self.inner.borrow_mut();
        let r = store
            .releases
            .iter_mut()
            .find(|r| r.id == up.release_id)
            .ok_or_else(|| AppError::not_found("没有这个 Release"))?;
        if let Some(tag) = &up.tag_name {
            r.tag = tag.clone();
        }
        if let Some(d) = up.draft {
            r.draft = d;
        }
        Ok(RemoteRelease {
            platform: self.kind.to_owned(),
            id: r.id,
            tag_name: r.tag.clone(),
            url: format!("https://fake/rel/{}", r.tag),
        })
    }

    fn assets(&self, _owner: &str, _repo: &str, tag: &str) -> Result<Vec<RemoteAsset>, AppError> {
        let store = self.inner.borrow();
        Ok(Self::release_by_tag(&store, tag)
            .map(|r| r.assets.iter().map(|a| self.asset_view(a)).collect())
            .unwrap_or_default())
    }

    fn upload_asset(&self, up: &AssetUpload) -> Result<UploadedAsset, AppError> {
        if *self.fail_next_upload.borrow() {
            *self.fail_next_upload.borrow_mut() = false;
            return Err(AppError::io("演练注入：上传中断"));
        }
        let bytes = std::fs::read(&up.path)
            .map_err(|e| AppError::io("读不到要上传的文件").with_detail(e.to_string()))?;
        let sha = super::platforms::sha256_file(&up.path)?;
        let size = bytes.len() as u64;
        self.writes.borrow_mut().push(format!("upload {}", up.name));
        let mut store = self.inner.borrow_mut();
        let id = Self::new_id(&mut store);
        let r = store
            .releases
            .iter_mut()
            .find(|r| r.id == up.release_id)
            .ok_or_else(|| AppError::not_found("没有这个 Release"))?;
        r.assets.retain(|a| a.name != up.name);
        r.assets.push(FakeAsset {
            id,
            name: up.name.clone(),
            size,
            sha256: sha,
            source: Some(up.path.clone()),
        });
        Ok(UploadedAsset {
            name: up.name.clone(),
            size,
            url: format!("https://fake/{}/{}", self.kind, up.name),
        })
    }

    fn delete_asset(
        &self,
        _owner: &str,
        _repo: &str,
        release_id: u64,
        asset_id: u64,
    ) -> Result<(), AppError> {
        self.writes
            .borrow_mut()
            .push(format!("delete_asset {asset_id}"));
        let mut store = self.inner.borrow_mut();
        if let Some(r) = store.releases.iter_mut().find(|r| r.id == release_id) {
            r.assets.retain(|a| a.id != asset_id);
        }
        Ok(())
    }

    fn delete_release(&self, _owner: &str, _repo: &str, release_id: u64) -> Result<(), AppError> {
        self.writes
            .borrow_mut()
            .push(format!("delete_release {release_id}"));
        self.inner
            .borrow_mut()
            .releases
            .retain(|r| r.id != release_id);
        Ok(())
    }

    fn download_asset(
        &self,
        _owner: &str,
        _repo: &str,
        url: &str,
        dest: &Path,
    ) -> Result<u64, AppError> {
        let store = self.inner.borrow();
        let name = url.rsplit('/').next().unwrap_or_default();
        let asset = store
            .releases
            .iter()
            .flat_map(|r| r.assets.iter())
            .find(|a| a.name == name)
            .ok_or_else(|| AppError::not_found(format!("远端没有 {name}")))?;
        let src = asset
            .source
            .as_ref()
            .ok_or_else(|| AppError::io("这一份没有可下载的字节"))?;
        let bytes = std::fs::read(src)
            .map_err(|e| AppError::io("读不到远端那一份").with_detail(e.to_string()))?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).expect("建下载目录");
        }
        std::fs::write(dest, &bytes)
            .map_err(|e| AppError::io("写不了下载结果").with_detail(e.to_string()))?;
        Ok(bytes.len() as u64)
    }

    fn release_assets(
        &self,
        _owner: &str,
        _repo: &str,
        tag: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .assets(_owner, _repo, tag)?
            .into_iter()
            .map(|a| a.name)
            .collect())
    }

    fn list_releases(&self, _owner: &str, _repo: &str) -> Result<Vec<ReleaseSummary>, AppError> {
        let store = self.inner.borrow();
        Ok(store
            .releases
            .iter()
            .map(|r| ReleaseSummary {
                tag: r.tag.clone(),
                name: r.tag.clone(),
                prerelease: false,
                assets: r.assets.len(),
                bytes: Some(r.assets.iter().map(|a| a.size).sum()),
                at: "2026-11-23T00:00:00Z".to_owned(),
                url: format!("https://fake/rel/{}", r.tag),
            })
            .collect())
    }
}

/* ---------- 假 CI ---------- */

/// 演练用的构建通道：不联网、不起 runner，直接"造"出两份产物。
pub struct FakeCi {
    /// 造出来的包多大（字节数固定，便于断言 SHA 稳定）
    pub bytes: usize,
    /// 这一趟"构建失败"
    pub fail: bool,
    /// 已经存在的运行（测"复用已有产物、不重复构建"）
    pub existing: RefCell<Option<WorkflowRun>>,
    /// 触发次数（断言"复用时没有重新触发"）
    pub starts: RefCell<usize>,
}

impl Default for FakeCi {
    fn default() -> Self {
        Self {
            bytes: 2048,
            fail: false,
            existing: RefCell::new(None),
            starts: RefCell::new(0),
        }
    }
}

impl FakeCi {
    fn run(&self, id: u64) -> WorkflowRun {
        WorkflowRun {
            id,
            status: "completed".to_owned(),
            conclusion: Some(if self.fail { "failure" } else { "success" }.to_owned()),
            head_sha: "0".repeat(40),
            created_at: "2026-11-23T00:00:00Z".to_owned(),
            url: format!("https://fake/run/{id}"),
        }
    }
}

impl super::ci::Ci for FakeCi {
    fn reusable(
        &self,
        _owner: &str,
        _repo: &str,
        _source: &str,
    ) -> Result<Option<WorkflowRun>, AppError> {
        Ok(self.existing.borrow().clone())
    }

    fn start(
        &self,
        _owner: &str,
        _repo: &str,
        _source: &str,
        _version: &str,
        summary: &mut Vec<String>,
    ) -> Result<(), AppError> {
        *self.starts.borrow_mut() += 1;
        summary.push("演练：触发了一次假构建".to_owned());
        Ok(())
    }

    fn wait(
        &self,
        _owner: &str,
        _repo: &str,
        _created_after: &str,
        prefer: Option<u64>,
        _deadline: std::time::Instant,
        summary: &mut Vec<String>,
    ) -> Result<WorkflowRun, AppError> {
        if self.fail {
            return Err(AppError::io("演练注入：构建失败"));
        }
        summary.push("演练：构建完成".to_owned());
        Ok(self.run(prefer.unwrap_or(900)))
    }

    fn collect(
        &self,
        _owner: &str,
        _repo: &str,
        run_id: u64,
        _source: &str,
        version: &str,
        platform: &'static Platform,
        dest_root: &Path,
        summary: &mut Vec<String>,
    ) -> Result<Vec<ArtifactFact>, AppError> {
        let dir = dest_root.join(format!("run-{run_id}"));
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::io("建不了演练产物目录").with_detail(e.to_string()))?;
        let mut facts = Vec::new();
        for (i, kind) in platform.artifacts.iter().enumerate() {
            let name = platforms::artifact_name(version, kind);
            let path = dir.join(&name);
            /* 每份内容各不相同：同名同大小同 SHA 的判据才有意义 */
            let body = vec![b'a' + i as u8; self.bytes];
            std::fs::write(&path, &body)
                .map_err(|e| AppError::io("写不了演练产物").with_detail(e.to_string()))?;
            let fact = ArtifactFact::from_path(version, platform, kind, &path)?;
            summary.push(format!("演练：造出 {}", fact.name));
            facts.push(fact);
        }
        Ok(facts)
    }
}

/// 一份"已有资产"的字节表（seed 用）。
pub fn sample_bytes(seed: u8, len: usize) -> Vec<u8> {
    vec![seed; len]
}

/// 记录用：把一份远端资产表摊成 `名 → 大小`（断言"另一个平台没被动过"）。
pub fn asset_sizes(h: &FakeHosting, tag: &str) -> BTreeMap<String, u64> {
    h.assets("o", "r", tag)
        .expect("读资产表")
        .into_iter()
        .map(|a| (a.name, a.size.unwrap_or_default()))
        .collect()
}
