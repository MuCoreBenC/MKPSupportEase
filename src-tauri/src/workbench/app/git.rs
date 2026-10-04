//! **本地 git**（发布事务的「本地那一半」）—— 子进程封装。
//!
//! # 为什么是子进程 `git`，而不是「远程平台的凭据系统」
//!
//! Git 是**本地版本控制工具**：它不需要任何"平台账号"，也不需要网络。
//! 把本地提交/推送交给系统上的 `git`，是桌面产品的正常做法（仓库本来就是这么建的）。
//!
//! ★ 它与**远程平台认证**（GitHub / Gitee 的 Token）是**两件不同的事**：
//! 平台认证与 PR/MR 全部由我们自己的 HTTP 模块处理（见 [`super::platform`]），
//! **不借用户的 `gh` / `git` 登录态**。这条边界是产品决定，不是实现细节。
//!
//! # 两条硬规矩
//!
//! 1. **参数一律显式数组**，绝不拼 shell 字符串（`Command::new("git").args([...])`）——
//!    拼字符串会带进注入面，而这里的参数里有文件名。
//! 2. ★ **只 stage 白名单路径**（[`STAGE_ALLOWLIST`]）。永不 `git add -A` ——
//!    "这次发布"要提交的是**交付产物那一批**，把工作区里别的改动一起卷进去是事故。
//!    这条口径与发布闸 ⑮ `git/clean`（排除 `presets/dist/` 之外不许有脏）互为反面：
//!    ⑮ 保证"除产物外没有别的改动"，本模块保证"只提交那一批"。
//!
//! # 取不到 git 怎么办
//!
//! 与 ⑮ 同一取向：**不假装成功**。每次调用返回 `Result`，取不到就如实报
//! （`AppError`，运输类）。发布闸可以在没有 git 的机器上"跳过"⑮，但**发布事务**
//! 要真推东西，拿不到 git 就是硬失败 —— 那正是它该有的样子。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

/// 允许被 stage 的路径白名单（**相对仓库根**，前缀匹配）。
///
/// ★ 这就是"这次发布提交什么"的**唯一答案**。想提交新东西？加到这份清单里来，
/// 别在这里写 `-A`。交付产物 `presets/dist/` 是主体；结构规则表与资产台账跟着
/// 结构 / 资产变化走，一起提交。
pub const STAGE_ALLOWLIST: [&str; 3] = [
    "presets/dist/",
    "presets/structure-signatures.toml",
    "presets/assets.toml",
];

/// 一条 git 命令的结果：成功 = stdout（已 trim 掉尾换行），失败 = 带 stderr 的 `AppError`。
///
/// **与 ⑮ 里那个只返回 `Option<String>` 的 `git()` 不同**：那个是"探测"（没有 git 就当
/// 没有），这里是"执行"（没成功就是错）。发布事务不能把"没提交上"当"没有 git"糊过去。
pub struct Git {
    repo: PathBuf,
}

impl Git {
    /// 打开仓库根这一处的 git。**不检查它是不是仓库** —— 检查落在第一次命令上
    /// （`rev-parse`），那时才拿得到真实原因。
    pub fn open(repo: impl Into<PathBuf>) -> Self {
        Self { repo: repo.into() }
    }

    /// 真仓库。
    pub fn at_repo_root() -> Self {
        Self::open(super::super::paths::repo_root())
    }

    /// 仓库根
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// 当前分支名（`git symbolic-ref --short HEAD`）。游离 HEAD 上会失败 —— 如实报。
    pub fn branch(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["symbolic-ref", "--short", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// 工作区状态（`git status --porcelain=v1 -z --untracked-files=all`），原样返回。
    ///
    /// 用 `-z`（NUL 分隔）而不是默认换行：文件名里可以有换行 / 引号，`-z` 是不需要转义
    /// 解析的那一种（与闸 ⑮ 同一取向）。
    pub fn status_raw(&self) -> Result<String, AppError> {
        self.run(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])
    }

    /// 差异统计（`git diff --stat`），给人看的一段文本。
    pub fn diff_stat(&self) -> Result<String, AppError> {
        self.run(&["diff", "--stat"])
    }

    /// 把白名单里**确实有变化**的路径加进索引。
    ///
    /// ★ 传进来的是候选路径清单，这里**再按 [`STAGE_ALLOWLIST`] 过滤一遍** ——
    /// 第二道闸。就算调用方一时手滑传了 `presets/assets/foo.png`，也进不去索引。
    /// 返回真正 stage 了的路径（相对仓库根），空 = 没有可提交的。
    pub fn stage_allowed(&self, candidates: &[String]) -> Result<Vec<String>, AppError> {
        let allowed: Vec<&String> = candidates.iter().filter(|p| is_allowed(p)).collect();
        if allowed.is_empty() {
            return Ok(Vec::new());
        }
        let mut args: Vec<&str> = vec!["add", "--"];
        args.extend(allowed.iter().map(|s| s.as_str()));
        self.run(&args)?;
        Ok(allowed.into_iter().cloned().collect())
    }

    /// 提交已 stage 的内容。**只提交索引里那批**（`git commit` 不带 `-a`）——
    /// 绝不把未 stage 的工作区改动卷进来。
    pub fn commit(&self, message: &str) -> Result<(), AppError> {
        self.run(&["commit", "-m", message])?;
        Ok(())
    }

    /// 当前 HEAD 的短 sha（`git rev-parse --short HEAD`）—— 发布回执里显示"提交了哪一笔"。
    pub fn head_short(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// 推分支（`git push -u origin <branch>`）。`branch` 为空 = 用当前分支。
    ///
    /// **不带认证** —— 只在凭据已由别处（如 CI 的 credential helper）提供时用。
    /// 发布事务走 [`push_authenticated`]（自持 Token）。
    pub fn push(&self, branch: &str) -> Result<(), AppError> {
        let target = self.resolve_branch(branch)?;
        self.run(&["push", "-u", "origin", &target])?;
        Ok(())
    }

    /// **带认证**推分支（发布事务用）：Token 经 `http.extraHeader` 临时喂给 git。
    ///
    /// 命令行形状（作者 2026-10-04 定死）：
    ///
    /// ```text
    /// git -c credential.helper= \
    ///     -c http.extraHeader="Authorization: Basic <base64(username:token)>" \
    ///     push -u origin <branch>
    /// ```
    ///
    /// ★ **Token 不进 remote URL**：URL 里没有凭据，`git remote -v`、`.git/config`、
    ///   push 回显里都不会出现 Token。
    /// ★ **`credential.helper=`（空值）清掉全局 helper**：Git 因此**不会**去读用户系统里
    ///   已存的 Git 凭据 —— 正是作者要避免的"不读用户已有凭据"。
    /// ★ 子进程环境加 `GIT_TERMINAL_PROMPT=0`：不许 git 弹交互提问（会把后台命令挂死）。
    /// ★ 失败时**不把带 header 的参数写进错误 detail**（否则 Token 进日志）。
    pub fn push_authenticated(
        &self,
        branch: &str,
        username: &str,
        token: &str,
    ) -> Result<(), AppError> {
        let target = self.resolve_branch(branch)?;
        let basic = base64_encode(format!("{username}:{token}").as_bytes());
        let header = format!("http.extraHeader=Authorization: Basic {basic}");
        let out = Command::new("git")
            // `-c credential.helper=` 清掉全局 helper；`-c http.extraHeader=` 临时带认证。
            // 两个 `-c` 必须在子命令（push）**之前**。
            .args([
                "-c",
                "credential.helper=",
                "-c",
                &header,
                "push",
                "-u",
                "origin",
                &target,
            ])
            .current_dir(&self.repo)
            // 禁交互：没有 TTY 时 git 会尝试提示，这里直接关掉（防挂死）
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        if !out.status.success() {
            // ★ detail 只用 stderr —— **不带那条 header 参数**
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::io("git push 失败了").with_detail(format!(
                "{}（退出码 {:?}）",
                err.trim(),
                out.status.code()
            )));
        }
        Ok(())
    }

    /// 当前工作目录的 remote（`origin`）是不是**配置的那个仓库**（发布前的一致性校验）。
    ///
    /// 规范化后比较：https / ssh 两种形状等价、`…/o/r.git` 与 `…/o/r` 等价、
    /// host 大小写不敏感。认不出形状（自建源等）→ `false`（不硬猜）。
    ///
    /// ★ 作者定死：**remote 只用来校验"是不是发到配置的那个仓库"**，不是用来决定发布目标。
    /// 不一致 = 当前目录不是配置的那个仓库 —— 如实拒绝，不发。
    pub fn remote_matches(&self, repository_url: &str) -> Result<bool, AppError> {
        let actual = self.remote_url()?;
        Ok(normalize_repo_url(&actual) == normalize_repo_url(repository_url))
    }

    fn resolve_branch(&self, branch: &str) -> Result<String, AppError> {
        if branch.trim().is_empty() {
            self.branch()
        } else {
            Ok(branch.trim().to_owned())
        }
    }

    /// 索引里有没有已 stage 的改动（`git diff --cached --quiet` 退出码反过来）。
    ///
    /// 用它决定"要不要 commit"：没有任何 staged 改动时提交会以 `nothing to commit` 失败，
    /// 那不是错误，是"这次没东西可提"。
    pub fn has_staged(&self) -> Result<bool, AppError> {
        let out = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&self.repo)
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        // 退出码 0 = 没有差异；1 = 有差异；其余 = 出错
        match out.status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(AppError::io("git diff --cached 没跑通")
                .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned())),
        }
    }

    /// 远端 URL（`origin`）。**平台推断**（[`super::platform::detect_platform`]）用它。
    pub fn remote_url(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["remote", "get-url", "origin"])?
            .trim()
            .to_owned())
    }

    /// 跑一条 git 命令。参数显式数组；工作目录固定仓库根。
    fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::io(format!(
                "git {} 失败了",
                args.first().copied().unwrap_or("")
            ))
            .with_detail(format!("{}（退出码 {:?}）", err.trim(), out.status.code())));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

/// 路径在不在 stage 白名单里。**前缀匹配**（`presets/dist/` 收下它下头的一切）。
pub fn is_allowed(path: &str) -> bool {
    let p = path.trim().trim_matches('"');
    STAGE_ALLOWLIST
        .iter()
        .any(|a| p == a.trim_end_matches('/') || p.starts_with(a))
}

fn spawn_failed(repo: &Path, e: std::io::Error) -> AppError {
    AppError::io("起不了 git —— 它可能不在 PATH 上").with_detail(format!("{}：{e}", repo.display()))
}

/// 把仓库地址规范化成可比形状：`host/path`（小写、去协议、去 `.git`、去尾部斜杠）。
///
/// 让 `https://GitHub.com/o/r.git`、`git@github.com:o/r.git`、`https://github.com/o/r`
/// 三者等价 —— "当前 remote 是不是配置的那个仓库"才算得准。
/// 认不出形状 → 返回原串（小写、去尾斜杠）—— 那会让它跟配置对不上，**保守判"不匹配"**。
///
/// `pub`：发布账户命令（`publish_tx`）做"配置的仓库 vs 当前 remote"比对时也用它 ——
/// **规范化只有这一处实现**，两处调用方不许各写一遍。
pub fn normalize_repo_url_for_compare(url: &str) -> String {
    normalize_repo_url(url)
}

fn normalize_repo_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let u = u.strip_suffix(".git").unwrap_or(u);
    // ssh 形状 git@host:path → host/path
    if let Some(rest) = u.strip_prefix("git@") {
        return rest.replacen(':', "/", 1).to_ascii_lowercase();
    }
    // https://host/path 或 http://host/path → host/path
    for scheme in ["https://", "http://", "ssh://"] {
        if let Some(rest) = u.strip_prefix(scheme) {
            // ssh://git@host/path 还要再去一层 user@
            let rest = rest.rsplit('@').next().unwrap_or(rest);
            return rest.to_ascii_lowercase();
        }
    }
    u.to_ascii_lowercase()
}

/// 最小 base64 编码（标准字母表 + `=` 补齐）。
///
/// **手写而不引 crate**：只用在这一处（构造 Basic 认证头），
/// 引入一个 `base64` 依赖不值当（作者定的"能不加依赖就不加"）。
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18 & 0x3f) as usize] as char);
        out.push(TABLE[(n >> 12 & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6 & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ **`git add -A` 的替代是"白名单前缀匹配"** —— 这条钉住"什么进得去、什么进不去"。
    /// 交付产物、结构规则表、资产台账进得去；源码、配置、别的东西进不去。
    #[test]
    fn stage_paths_are_an_explicit_allowlist() {
        for ok in [
            "presets/dist/catalog.json",
            "presets/dist/mkp/presets/A1-standard.toml",
            "presets/structure-signatures.toml",
            "presets/assets.toml",
        ] {
            assert!(is_allowed(ok), "{ok} 该在允许清单里");
        }
        for no in [
            "src-tauri/src/lib.rs",
            "package.json",
            "presets/machines/A1.toml",
            "presets/assets/bbs/Process/0.2mm/A1.json",
            "handoff.md",
        ] {
            assert!(
                !is_allowed(no),
                "{no} 不该被 stage —— 那会把无关改动卷进发布提交"
            );
        }
    }

    /// 白名单路径**是相对仓库根的前缀**，不是绝对路径（提交时 git 也按仓库根解释）。
    #[test]
    fn the_allowlist_is_relative_to_the_repo_root() {
        for a in STAGE_ALLOWLIST {
            assert!(!a.starts_with('/'), "{a} 不该是绝对路径");
            assert!(a.starts_with("presets/"), "{a} 该在 presets/ 下");
        }
    }

    /// ★★ **push 带认证但 Token 不进 URL** —— 这条钉住这一刀的认证方式。
    ///
    /// 只验"参数构造"（不真推）：`--dry-run` 那种真推需要网络，判据里不做。
    /// 断言：Basic 头是 `base64(username:token)`，且**没有**把 token 塞进 remote URL 的形状。
    #[test]
    fn git_push_authenticates_without_putting_the_token_in_the_url() {
        // 构造与 push_authenticated 内一致的那条 header
        let token = "ghp_secrettoken";
        let basic = base64_encode(format!("ben:{token}").as_bytes());
        let header = format!("http.extraHeader=Authorization: Basic {basic}");
        assert!(header.contains("Basic "), "该是一条 Basic 认证头");
        // base64 解出来的原文 = user:token
        assert_eq!(base64_decode_for_test(&basic), format!("ben:{token}"));
        // 头里不出现裸 token（base64 之后本就看不到），且**没有任何地方把它拼进 URL**
        let args = [
            "-c".to_owned(),
            "credential.helper=".to_owned(),
            "-c".to_owned(),
            header.clone(),
            "push".to_owned(),
            "-u".to_owned(),
            "origin".to_owned(),
            "publish/x".to_owned(),
        ];
        let joined = args.join(" ");
        // 关键：URL/参数里没有 `user:token@` 那种形状
        assert!(
            !joined.contains("ben:ghp"),
            "Token 不该以明文进参数：{joined}"
        );
        assert!(
            !joined.contains("ghp_secrettoken@"),
            "Token 不该进 URL：{joined}"
        );
        assert!(
            joined.contains("credential.helper="),
            "要清掉全局 credential helper"
        );
    }

    /// base64 编码正确性（对照几个已知向量）。
    #[test]
    fn base64_encodes_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // Basic 认证头的典型向量
        assert_eq!(base64_encode(b"ben:pass"), "YmVuOnBhc3M=");
    }

    /// 仓库地址规范化：https / ssh / `.git` / 大小写 / 尾斜杠都归一到同一形状。
    #[test]
    fn repo_url_normalization_makes_equivalent_addresses_match() {
        let a = normalize_repo_url("https://github.com/o/r.git");
        assert_eq!(a, "github.com/o/r");
        assert_eq!(normalize_repo_url("git@github.com:o/r.git"), a);
        assert_eq!(normalize_repo_url("https://GitHub.com/o/r"), a);
        assert_eq!(normalize_repo_url("https://github.com/o/r/"), a);
        assert_eq!(normalize_repo_url("ssh://git@github.com/o/r.git"), a);
        // 不同仓库不该相等
        assert_ne!(normalize_repo_url("https://github.com/o/r2.git"), a);
    }

    /// remote 与配置一致 / 不一致（临时真仓库验一遍）。
    #[test]
    fn remote_matches_compares_the_configured_repository() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("起 git");
            assert!(out.status.success(), "git {args:?} 失败");
        };
        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "git@github.com:MuCoreBenC/MKPSupportEase.git",
        ]);

        let g = Git::open(root);
        // 同一仓库的不同写法都算匹配
        assert!(g
            .remote_matches("https://github.com/MuCoreBenC/MKPSupportEase.git")
            .unwrap());
        assert!(g
            .remote_matches("git@github.com:MuCoreBenC/MKPSupportEase.git")
            .unwrap());
        // 别的仓库 = 不匹配
        assert!(!g
            .remote_matches("https://github.com/other/repo.git")
            .unwrap());
        assert!(!g
            .remote_matches("https://gitee.com/MuCoreBenC/MKPSupportEase.git")
            .unwrap());
    }

    /// 测试用：把 base64 解回原文（只覆盖 ASCII + 补齐，够验这几个向量）。
    fn base64_decode_for_test(s: &str) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let clean: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
        let val = |b: u8| TABLE.iter().position(|t| *t == b).unwrap() as u32;
        let mut out = Vec::new();
        for chunk in clean.chunks(4) {
            let mut n = 0u32;
            for (i, b) in chunk.iter().enumerate() {
                n |= val(*b) << (18 - 6 * i);
            }
            out.push((n >> 16 & 0xff) as u8);
            if chunk.len() > 2 {
                out.push((n >> 8 & 0xff) as u8);
            }
            if chunk.len() > 3 {
                out.push((n & 0xff) as u8);
            }
        }
        String::from_utf8(out).unwrap()
    }

    /// 一个真实的 git 仓库走一遍：stage 白名单 + 提交，**别的东西一个都不许进去**。
    ///
    /// 用临时目录造一个真仓库（`git init`）—— 这是少数值得真跑 git 的地方：
    /// "白名单真的拦住了 `git add`"只有真调 git 才验得出来（假脚本验不了 git 自己的行为）。
    #[test]
    fn a_real_repo_only_commits_the_allowlisted_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ok = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("起 git");
            assert!(
                out.status.success(),
                "git {args:?} 失败：{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        ok(&["init", "-q"]);
        ok(&["config", "user.email", "t@example.com"]);
        ok(&["config", "user.name", "t"]);
        // 切到任务分支再提交：这套仓库的开发机装了「禁止直接在 main 上提交」的全局钩子，
        // 临时仓库默认落在 main 上会被拦（真实发布流程也从任务分支起）。这也顺带验了
        // `symbolic-ref --short HEAD` 拿得到非 main 分支名。
        ok(&["checkout", "-q", "-b", "publish/demo"]);

        // 造两类文件：白名单里的（交付产物）与白名单外的（源码）。
        // 走 `atomic_write`（clippy 对测试也生效，且与产品代码同一条写盘纪律）。
        std::fs::create_dir_all(root.join("presets/dist")).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("presets/dist/catalog.json"), b"{}").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("src/lib.rs"), b"// not ours").unwrap();

        let git = Git::open(root);
        let staged = git
            .stage_allowed(&[
                "presets/dist/catalog.json".to_owned(),
                "src/lib.rs".to_owned(), // 意图混进来 —— 必须被第二道闸挡掉
            ])
            .expect("stage");
        assert_eq!(staged, vec!["presets/dist/catalog.json".to_owned()]);
        assert!(git.has_staged().unwrap(), "该有一份 staged");

        // 索引里只有白名单那一份
        let out = Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(root)
            .output()
            .unwrap();
        let names = String::from_utf8_lossy(&out.stdout);
        assert!(names.contains("presets/dist/catalog.json"));
        assert!(
            !names.contains("src/lib.rs"),
            "白名单外的文件被 stage 进去了：{names}"
        );

        git.commit("发布：演示").expect("commit");
        // 提交之后 src/lib.rs 仍是未跟踪 —— 它没被卷进去
        assert!(git.status_raw().unwrap().contains("src/lib.rs"));
    }
}
