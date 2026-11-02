/*
 * 构建脚本 —— **把两个云端源（Bootstrap）地址注入二进制**。
 *
 * # 值的来源与优先级（2026-10-02 第十七刀）
 *
 *   ① 环境变量 `MKPSE_PRESET_SOURCE` / `MKPSE_PRESET_SOURCE_GITEE`：构建方显式覆盖
 *      （CI 打测试包用的就是它，也有权覆盖一切）——**在就不动它**；
 *   ② 工作台配置 `<repo>/workbench/bootstrap.json`（`wb_set_bootstrap` 写的，入库）：
 *      `npm run tauri dev` 与正式构建都从这里自动拿到云端源；
 *   ③ 都没有 → 二进制里没有那个默认地址，联网命令诚实回答「还没配置」，
 *      **不编一个 URL 出来假装能下**。
 *
 * # 语义
 *
 * 这两个值都是 **Bootstrap 地址**（指向发布产物里的 `source.json`），不是"直接根" ——
 * 客户端读它拿到 catalog 与文件下载根的地址（见 `runtime::source::resolve_source`）。
 *
 * ★ **两个都是云端源**（2026-10-05，作者拍"国内走 Gitee"）：`bootstrapUrl` = GitHub
 *   （默认），`giteeBootstrapUrl` = Gitee。**各自独立注入**：Gitee 那边没配就少一个
 *   选项（界面不出现它），不影响 GitHub 那个。
 *
 * # 改了配置什么时候生效
 *
 * 它是**编译期**注入：改完 `workbench/bootstrap.json` 要重新构建（dev 下重启
 * `tauri dev`）；`rerun-if-changed` 登记了它，cargo 会帮忙看见变化。
 */

/* ---------- 构建指纹（2026-11-02） ----------
 *
 * 为什么要有：作者反复被问「你是不是没重新编译后端」—— 而这个问题**以前只能靠猜**。
 * 光看文件日期也不可靠（`target/debug` 与 `target-workbench/debug` 是两份，
 * 谁新谁旧一眼看不出来，而且「编了但没重启」的进程还在跑旧字节）。
 *
 * 所以每次编译把三格刻进二进制，启动时打一行日志：
 *
 * | 格 | 回答什么 |
 * |---|---|
 * | `BUILD_TIME` | **这一次**是什么时候编的（不是源码时间，是 build.rs 跑的时间） |
 * | `GIT_COMMIT` | 编的是哪个提交（`-dirty` 后缀 = 有未提交改动） |
 * | `FEATURES` | 带没带 `workbench` —— 两档是同一个可执行文件名，光看名字分不出 |
 *
 * `rerun-if-changed` 不登记任何源码：否则每改一行都要重跑 build.rs。
 * 只有 `.git/HEAD` 与 `.git/index` 变了才重算这三格（切分支 / 提交后要能反映出来）。
 */
fn main() {
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE");
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE_GITEE");
    println!("cargo:rerun-if-changed=../workbench/bootstrap.json");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    emit_build_fingerprint();

    let configured = bootstrap_urls_from_workbench();

    if std::env::var_os("MKPSE_PRESET_SOURCE").is_none() {
        if let Some(url) = configured.as_ref().and_then(|(gh, _)| gh.clone()) {
            println!("cargo:rustc-env=MKPSE_PRESET_SOURCE={url}");
        }
    }
    if std::env::var_os("MKPSE_PRESET_SOURCE_GITEE").is_none() {
        if let Some(url) = configured.as_ref().and_then(|(_, gt)| gt.clone()) {
            println!("cargo:rustc-env=MKPSE_PRESET_SOURCE_GITEE={url}");
        }
    }

    tauri_build::build()
}

/// 把「这一次编译」的三格刻进二进制（见 [`emit_build_fingerprint`] 上方那段说明）。
///
/// 全部失败都不挡构建：拿不到就用 `unknown`，**绝不编一个看起来像真的值**。
fn emit_build_fingerprint() {
    // ① 编译时刻（build.rs 真正跑起来的这一刻，UTC，秒级）
    let stamp = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=MKPSE_BUILD_TIME={stamp}");

    // ② 提交号 + 有没有未提交改动。`--always` 让无 tag 的浅克隆也给短 sha
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir("..")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_owned();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    };
    let mut commit = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    if git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty()) {
        commit.push_str("-dirty");
    }
    println!("cargo:rustc-env=MKPSE_GIT_COMMIT={commit}");

    // ③ 这一档带没带 workbench。两档共用可执行文件名，光看进程名分不出
    let features = std::env::var("CARGO_FEATURE_WORKBENCH").is_ok();
    println!(
        "cargo:rustc-env=MKPSE_BUILD_TIER={}",
        if features { "workbench" } else { "client" }
    );
}

/// 读工作台配置里那两个字段。**读不出 / 解析不出 / 是空串都当"没配"**（返回 `None`）——
/// 构建脚本不挡构建：真正的解释权在运行时（没注入就是「还没配置」那一档）；
/// 手改坏的配置下次在工作台里再保存一次就会被覆盖成好档。
///
/// 两个字段**各读各的**：只写了一个不该让另一个跟着丢。
fn bootstrap_urls_from_workbench() -> Option<(Option<String>, Option<String>)> {
    let text = std::fs::read_to_string("../workbench/bootstrap.json").ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let pick = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let github = pick("bootstrapUrl");
    let gitee = pick("giteeBootstrapUrl");
    if github.is_none() && gitee.is_none() {
        return None;
    }
    Some((github, gitee))
}
