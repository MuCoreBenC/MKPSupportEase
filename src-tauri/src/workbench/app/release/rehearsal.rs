//! **发布演练矩阵**：用假远端 + 假 CI 把整条编排跑一遍。
//!
//! 每一场都对应一条**发布正确性责任**（不是"把第三方 API 都测一遍"）：
//!
//! | 场景 | 责任 |
//! |---|---|
//! | 冻结 | 只建暂存面，正式 tag 一个都不许提前出现 |
//! | 补一个平台 | 另一个平台已上传的资产一个字节都不动 |
//! | 断点续跑 | 已齐的源不再传，缺的源只补缺的那几份 |
//! | 身份读不懂 | "不知道多大"不等于"一样大"——必须删了重传 |
//! | 封版前置 | 两源不齐 / 判据不过：不许建 tag |
//! | 已封版重跑 | 零写入（不重建暂存、不重传、不重打 tag） |
//! | 换机器 | 本地没有产物时从暂存面把字节补回来 |
//! | CI 失败 | 停在上传之前，不留半个发布面 |
//!
//! 这些场景 **不碰网络**：远端是内存里的 [`FakeHosting`]，CI 是 [`FakeCi`]，
//! git 用临时仓库 + 本地裸仓当 `origin`（push / tag 都是真的，只是不出本机）。

/* 演练/夹具里的写盘是**测试造现场**（临时仓库、假产物），不是生产写路径；
clippy 的写盘禁列只管生产代码，这里整模块豁免并写明理由。 */
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;
use crate::workbench::app::git::Git;
use crate::workbench::app::publish_tx::PublishTarget;

use super::testing::{asset_sizes, sample_bytes, Dialect, FakeCi, FakeHosting};
use super::{ci, platforms, run_with, Ports, ReleaseOptions, StopAfter, UpdateChannel};
use crate::workbench::app::platform::Hosting as _;

const VERSION: &str = "0.0.16";

/* ---------- 夹具 ---------- */

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    git: Git,
    ci: FakeCi,
    ports: Ports,
    gitee: &'static FakeHosting,
    github: &'static FakeHosting,
}

fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("起不了 git");
    assert!(
        out.status.success(),
        "git {:?} 失败：{}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn target(platform: &str, url: &str) -> PublishTarget {
    PublishTarget {
        platform: platform.to_owned(),
        repository_url: url.to_owned(),
        username: "tester".to_owned(),
        token: "T0KEN".to_owned(),
        owner: "o".to_owned(),
        repo: "r".to_owned(),
    }
}

impl Fixture {
    /// 建一个真 git 仓库 + 本地裸仓当 origin（`push` 不出本机）。
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("建临时目录");
        let root = dir.path().join("repo");
        let remote = dir.path().join("remote.git");
        std::fs::create_dir_all(&root).expect("建仓库目录");
        sh(
            dir.path(),
            &["init", "--bare", "-q", remote.to_str().unwrap()],
        );
        sh(&root, &["init", "-q"]);
        sh(&root, &["config", "user.email", "t@t"]);
        sh(&root, &["config", "user.name", "t"]);
        /* 闸门脚本是给真仓库用的；演练里关掉，否则每次 push 都跑一遍 clippy */
        sh(&root, &["config", "core.hooksPath", "/dev/null"]);
        sh(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        write(
            &root.join("src-tauri/Cargo.toml"),
            &format!("[package]\nname = \"mkp-support-ease\"\nversion = \"{VERSION}\"\n"),
        );
        write(
            &root.join("version.toml"),
            &format!("version = \"{VERSION}\"\n"),
        );
        write(
            &root.join("package.json"),
            &format!("{{\"name\":\"mkp-support-ease\",\"version\":\"{VERSION}\"}}\n"),
        );
        write(
            &root.join("presets/delivery/release.json"),
            &manifest("0.0.15", &[]),
        );
        sh(&root, &["add", "-A"]);
        sh(&root, &["commit", "-q", "-m", "init"]);
        sh(
            &root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        sh(&root, &["push", "-q", "-u", "origin", "main"]);
        sh(&root, &["fetch", "-q", "origin"]);

        let gitee: &'static FakeHosting =
            Box::leak(Box::new(FakeHosting::new("gitee", Dialect::Gitee)));
        let github: &'static FakeHosting =
            Box::leak(Box::new(FakeHosting::new("github", Dialect::GitHub)));

        let ports = Ports {
            code: target("github", "https://github.com/o/r"),
            code_hosting: Box::new(FakeHostingPtr(github)),
            release: target("gitee", "https://gitee.com/o/r"),
            release_hosting: Box::new(FakeHostingPtr(gitee)),
        };

        Self {
            root: root.clone(),
            git: Git::open(&root),
            ci: FakeCi::default(),
            ports,
            gitee,
            github,
            _dir: dir,
        }
    }

    fn opts(&self, stop: StopAfter, only: Option<&str>) -> ReleaseOptions {
        ReleaseOptions {
            version: Some(VERSION.to_owned()),
            notes: "演练".to_owned(),
            channel: Some(UpdateChannel::Stable),
            dry_run: false,
            confirmed: true,
            stop_after: Some(stop),
            only_platform: only.map(str::to_owned),
        }
    }

    fn run(&self, opts: &ReleaseOptions) -> Result<super::ReleaseReport, AppError> {
        run_with(&self.root, opts, &self.ports, &self.ci)
    }

    fn source(&self) -> String {
        sh(&self.root, &["rev-parse", "origin/main"])
    }

    /// 把 main 上的清单写成这一版（模拟"清单 PR 已合并"）。
    fn put_manifest(&self, version: &str, names: &[&str]) {
        let assets: Vec<(String, &str)> = names.iter().map(|n| (n.to_string(), *n)).collect();
        let text = manifest(
            version,
            &assets.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>(),
        );
        write(&self.root.join("presets/delivery/release.json"), &text);
        sh(&self.root, &["add", "-A"]);
        sh(&self.root, &["commit", "-q", "-m", "manifest"]);
        sh(&self.root, &["push", "-q", "origin", "main"]);
        sh(&self.root, &["fetch", "-q", "origin"]);
    }
}

/// `Ports` 要 `Box<dyn Hosting>`，但夹具还要留着引用做断言 —— 这层薄壳转过去。
struct FakeHostingPtr(&'static FakeHosting);

impl crate::workbench::app::platform::Hosting for FakeHostingPtr {
    fn kind(&self) -> &'static str {
        self.0.kind()
    }
    fn create_review(
        &self,
        spec: &crate::workbench::app::platform::ReviewSpec,
    ) -> Result<crate::workbench::app::platform::RemoteReview, AppError> {
        self.0.create_review(spec)
    }
    fn find_open_review(
        &self,
        owner: &str,
        repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<crate::workbench::app::platform::RemoteReview>, AppError> {
        self.0.find_open_review(owner, repo, head, base)
    }
    fn get_review(
        &self,
        id: &crate::workbench::app::platform::ReviewId,
    ) -> Result<crate::workbench::app::platform::RemoteReview, AppError> {
        self.0.get_review(id)
    }
    fn merge_review(
        &self,
        id: &crate::workbench::app::platform::ReviewId,
        method: crate::workbench::app::platform::MergeMethod,
    ) -> Result<crate::workbench::app::platform::RemoteReview, AppError> {
        self.0.merge_review(id, method)
    }
    fn create_release(
        &self,
        spec: &crate::workbench::app::platform::ReleaseSpec,
    ) -> Result<crate::workbench::app::platform::RemoteRelease, AppError> {
        self.0.create_release(spec)
    }
    fn upload_asset(
        &self,
        up: &crate::workbench::app::platform::AssetUpload,
    ) -> Result<crate::workbench::app::platform::UploadedAsset, AppError> {
        self.0.upload_asset(up)
    }
    fn assets(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> Result<Vec<crate::workbench::app::platform::RemoteAsset>, AppError> {
        self.0.assets(owner, repo, tag)
    }
    fn find_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> Result<Option<crate::workbench::app::platform::ReleaseView>, AppError> {
        self.0.find_release(owner, repo, tag)
    }
    fn update_release(
        &self,
        up: &crate::workbench::app::platform::ReleaseUpdate,
    ) -> Result<crate::workbench::app::platform::RemoteRelease, AppError> {
        self.0.update_release(up)
    }
    fn delete_release(&self, owner: &str, repo: &str, release_id: u64) -> Result<(), AppError> {
        self.0.delete_release(owner, repo, release_id)
    }
    fn delete_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        asset_id: u64,
    ) -> Result<(), AppError> {
        self.0.delete_asset(owner, repo, release_id, asset_id)
    }
    fn download_asset(
        &self,
        owner: &str,
        repo: &str,
        url: &str,
        dest: &Path,
    ) -> Result<u64, AppError> {
        self.0.download_asset(owner, repo, url, dest)
    }
    fn release_assets(&self, owner: &str, repo: &str, tag: &str) -> Result<Vec<String>, AppError> {
        self.0.release_assets(owner, repo, tag)
    }
    fn list_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<crate::workbench::app::platform::ReleaseSummary>, AppError> {
        self.0.list_releases(owner, repo)
    }
}

fn write(path: &Path, text: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).expect("建目录");
    }
    std::fs::write(path, text).expect("写文件");
}

fn manifest(version: &str, names: &[&str]) -> String {
    let assets: Vec<String> = names
        .iter()
        .map(|n| {
            format!(
                "{{\"name\":\"{n}\",\"url\":\"https://gitee.com/o/r/releases/download/v{version}/{n}\",\"size\":1,\"sha256\":\"{}\",\"platform\":\"windows\",\"kind\":\"nsis\"}}",
                "0".repeat(64)
            )
        })
        .collect();
    format!(
        "{{\n  \"releaseSchema\": 1,\n  \"version\": \"{version}\",\n  \"notes\": \"演练\",\n  \"url\": \"https://gitee.com/o/r/releases/tag/v{version}\",\n  \"assets\": [{}]\n}}\n",
        assets.join(",")
    )
}

fn staging_tag() -> String {
    platforms::staging_tag_of(VERSION)
}

fn name(kind: &str) -> String {
    match kind {
        "nsis" => format!("SupportEase_{VERSION}_x64-setup.exe"),
        "portable" => format!("SupportEase_{VERSION}_x64-portable.zip"),
        "dmg" => format!("SupportEase_{VERSION}_aarch64.dmg"),
        _ => format!("SupportEase_{VERSION}_aarch64.app.zip"),
    }
}

/* ---------- 场景 ---------- */

#[test]
fn freezing_creates_only_staging_surfaces() {
    let f = Fixture::new();
    let report = f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结应成功");

    assert_eq!(report.phase, crate::workbench::app::release::Phase::Frozen);
    assert!(
        !f.git.tag_exists(&format!("v{VERSION}")).unwrap_or(false),
        "冻结阶段不许有正式 tag"
    );
    assert!(f.gitee.write_count() > 0, "Gitee 暂存面应建起来");
    let staged = f.gitee.assets("o", "r", &staging_tag()).expect("读暂存");
    assert!(staged.is_empty(), "还没构建，暂存面上不该有资产");
}

#[test]
fn staging_one_platform_does_not_touch_the_other() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();

    /* 预置 macOS 两份（模拟"另一个平台早就发好了"） */
    let tmp = f.root.join(".scratch-mac");
    for kind in ["dmg", "appZip"] {
        let n = name(kind);
        let bytes = sample_bytes(9, 1024);
        f.gitee
            .seed_asset(&staging_tag(), &source, &n, &bytes, &tmp.join(&n));
        f.github
            .seed_asset(&staging_tag(), &source, &n, &bytes, &tmp.join(&n));
    }
    let before = asset_sizes(f.gitee, &staging_tag());

    f.run(&f.opts(StopAfter::Staging, Some("windows")))
        .expect("补 Windows 应成功");
    let after = asset_sizes(f.gitee, &staging_tag());

    for kind in ["dmg", "appZip"] {
        let n = name(kind);
        assert_eq!(before.get(&n), after.get(&n), "macOS 的 {n} 被动了");
    }
    assert!(after.contains_key(&name("nsis")), "Windows 安装包应进暂存");
    assert!(
        after.contains_key(&name("portable")),
        "Windows 绿色版应进暂存"
    );
}

#[test]
fn resume_tops_up_the_missing_source_only() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();
    let tmp = f.root.join(".scratch-win");
    /* 与 [`FakeCi`] 造出来的同样大小（2048）：这样"已在远端且同字节"才成立 */
    for kind in ["nsis", "portable"] {
        let n = name(kind);
        let bytes = sample_bytes(3, 2048);
        f.gitee
            .seed_asset(&staging_tag(), &source, &n, &bytes, &tmp.join(&n));
    }
    /* GitHub 上一份都没有 = 上一趟传完 Gitee 就被打断 */

    let _ = f
        .run(&f.opts(StopAfter::Staging, Some("windows")))
        .expect("续跑应成功");

    let gh = asset_sizes(f.github, &staging_tag());
    assert!(gh.contains_key(&name("nsis")), "GitHub 应被补齐");
    let uploads_to_gitee = f
        .gitee
        .writes
        .borrow()
        .iter()
        .filter(|w| w.starts_with("upload ") && w.contains(&name("nsis")))
        .count();
    assert_eq!(uploads_to_gitee, 0, "Gitee 上已有同字节，不该重传");
}

#[test]
fn an_unknown_remote_size_is_not_the_same_bytes() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();
    let tmp = f.root.join(".scratch-win");
    /* Gitee 上先放一份"名字对、大小读不出来"的旧包 */
    let n = name("nsis");
    f.gitee.seed_asset(
        &staging_tag(),
        &source,
        &n,
        &sample_bytes(1, 111),
        &tmp.join(&n),
    );
    *f.gitee.hide_size.borrow_mut() = true;

    let _ = f.run(&f.opts(StopAfter::Staging, Some("windows")));

    let deleted = f
        .gitee
        .writes
        .borrow()
        .iter()
        .any(|w| w.starts_with("delete_asset"));
    assert!(deleted, "读不到大小就不能判『同字节』，必须删了重传");
}

#[test]
fn seal_is_refused_while_a_source_is_incomplete() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();
    let tmp = f.root.join(".scratch-win");
    for kind in ["nsis", "portable"] {
        let n = name(kind);
        let bytes = sample_bytes(4, 256);
        /* 只在 Gitee 上齐：GitHub 缺一份，封版判据不该过 */
        f.gitee
            .seed_asset(&staging_tag(), &source, &n, &bytes, &tmp.join(&n));
    }

    let report = f
        .run(&f.opts(StopAfter::Seal, Some("windows")))
        .expect("这一趟不该是硬错，而是停在判据之前");
    assert_ne!(
        report.phase,
        crate::workbench::app::release::Phase::Sealed,
        "两源不齐时不许封版"
    );
    assert!(!report.seal_blockers.is_empty(), "应列出缺什么");
    assert!(
        !f.git.tag_exists(&format!("v{VERSION}")).unwrap_or(false),
        "判据不过时不许建正式 tag"
    );
}

#[test]
fn a_sealed_version_is_a_no_op() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();
    let tmp = f.root.join(".scratch-all");
    let names = [name("nsis"), name("portable"), name("dmg"), name("appZip")];
    for n in &names {
        let bytes = sample_bytes(7, 300);
        f.gitee
            .seed_asset(&staging_tag(), &source, n, &bytes, &tmp.join(n));
        f.github
            .seed_asset(&staging_tag(), &source, n, &bytes, &tmp.join(n));
        /* 正式面也要齐（模拟已封版） */
        f.gitee
            .seed_asset(&format!("v{VERSION}"), &source, n, &bytes, &tmp.join(n));
        f.github
            .seed_asset(&format!("v{VERSION}"), &source, n, &bytes, &tmp.join(n));
    }
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    f.put_manifest(VERSION, &refs);
    f.git
        .tag_at(&format!("v{VERSION}"), &source, "sealed")
        .expect("打 tag");

    let writes_before = f.gitee.write_count() + f.github.write_count();
    let report = f.run(&f.opts(StopAfter::Seal, None)).expect("重跑应成功");
    let writes_after = f.gitee.write_count() + f.github.write_count();

    assert_eq!(report.phase, crate::workbench::app::release::Phase::Sealed);
    assert_eq!(
        writes_before, writes_after,
        "已封版重跑不许有任何远端写操作"
    );
    assert_eq!(*f.ci.starts.borrow(), 0, "已封版重跑不许再触发构建");
}

#[test]
fn a_ci_failure_stops_before_anything_is_uploaded() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let ci = FakeCi {
        fail: true,
        ..FakeCi::default()
    };
    let err = run_with(
        &f.root,
        &f.opts(StopAfter::Staging, Some("windows")),
        &f.ports,
        &ci,
    )
    .expect_err("构建失败应停下");
    assert!(err.message.contains("构建"), "实际错误：{}", err.message);
    assert!(
        f.gitee
            .writes
            .borrow()
            .iter()
            .all(|w| !w.starts_with("upload ")),
        "构建失败时不许上传任何东西"
    );
}

#[test]
fn another_machine_resumes_by_downloading_from_staging() {
    let f = Fixture::new();
    f.run(&f.opts(StopAfter::Freeze, None)).expect("冻结");
    let source = f.source();
    let tmp = f.root.join(".scratch-remote");
    /* 四个平台都已在暂存面 = 上一台机器全部构建完；本机 target/ 里一个产物都没有 */
    let names = [name("nsis"), name("portable"), name("dmg"), name("appZip")];
    for n in &names {
        let bytes = sample_bytes(8, 400);
        f.gitee
            .seed_asset(&staging_tag(), &source, n, &bytes, &tmp.join(n));
        f.github
            .seed_asset(&staging_tag(), &source, n, &bytes, &tmp.join(n));
    }

    let report = f
        .run(&f.opts(StopAfter::Manifest, None))
        .expect("换机写清单应成功");
    assert!(
        report.summary.iter().any(|l| l.contains("清单")),
        "应走到清单那一步：{:?}",
        report.summary
    );

    /* 清单 PR 合并（真仓库里是人合，这里是夹具替它合） */
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    f.put_manifest(VERSION, &refs);

    let sealed = f.run(&f.opts(StopAfter::Seal, None)).expect("封版应成功");
    assert_eq!(sealed.phase, crate::workbench::app::release::Phase::Sealed);
    assert_eq!(
        sealed.source_commit.as_deref(),
        Some(source.as_str()),
        "封版后仍要报出冻结源码（暂存 tag 已清理，来源是正式 tag）"
    );
    let formal = asset_sizes(f.gitee, &format!("v{VERSION}"));
    for n in &names {
        assert!(formal.contains_key(n), "正式面上应有 {n}");
    }
    assert_eq!(
        sh(&f.root, &["rev-parse", &format!("v{VERSION}^{{commit}}")]),
        source,
        "正式 tag 必须指向冻结源码那一笔，不是 main 的 tip"
    );
}

/// 演练里用不到但需要出现在 `use` 里保持编译的常量（防未来误删）。
#[test]
fn the_ci_trait_is_the_one_used_by_the_orchestration() {
    fn assert_ci<T: ci::Ci>() {}
    assert_ci::<FakeCi>();
}
