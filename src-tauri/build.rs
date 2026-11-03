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
 * 所以每次编译把两格刻进二进制，启动时打一行日志：
 *
 * | 格 | 回答什么 |
 * |---|---|
 * | `GIT_COMMIT` | 编的是哪个提交（`-dirty` 后缀 = 有未提交改动） |
 * | `FEATURES` | 带没带 `workbench` —— 两档是同一个可执行文件名，光看名字分不出 |
 *
 * # ★ 时间那格的三次迭代（2026-11-03 很重要，别再往回改）
 *
 * **第一版**：build.rs 跑的那一刻当「编译于」。两个死穴 ——
 * ① 它跑在**链接之前**，必然比 exe 早（实测差 1 分钟）；
 * ② cargo 判定"产物已是最新"时**根本不跑它** ⇒ 印出来的是上一次的时间
 *    （实测差 11 小时）。而「你是不是没重新编译」正是靠这一格回答的，
 * 它一过期，这句话就变成没有证据的指责（作者原话：「我实际上早就编译了，
 * 只不过它的时间是错的」）。
 *
 * **第二版**：改成运行时读 `current_exe()` 的 mtime
 * （`lib.rs::exe_modified_local`）—— 那是链接完成那一刻，不会过期。
 *
 * **第三版（现在）**：把 `MKPSE_BUILD_TIME` 这个 `rustc-env` **删掉**。
 * 理由：`rustc-env` 一变，整个 app crate 就失效重编；而下面
 * `rerun-if-changed=src` 让 build.rs 每次改 Rust 都跑 ⇒ 每次注入的时间戳都不同
 * ⇒ **改一行 Rust 就要把 `mkp_support_ease_lib` 全量 codegen 一遍**。
 * 作者原话：「我感觉我每次编译时间也挺长的」—— 就是它。
 * 时间那格既然改由运行时读 exe，这条注入已经没有任何用处，只剩拖慢编译。
 *
 * 源码登记（`src` / `../crates`）**保留**：build.rs 重跑只做几次 `git` 调用
 * （毫秒级）；只要它输出的 `GIT_COMMIT` 不变，cargo 就不会因此重编。
 *
 * 另两个"时刻会漂"的地方也一并登记：
 * - `.git/HEAD` / `.git/index`：切分支、暂存、提交后提交号要能反映出来；
 * - `Cargo.toml`：改了依赖或版本，指纹也该更新。
 */
fn main() {
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE");
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE_GITEE");
    println!("cargo:rerun-if-changed=../workbench/bootstrap.json");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=Cargo.toml");
    // 源码也登记：build.rs 因此会随源码重跑（它只做几次 git 调用，毫秒级）。
    // **但它不再注入时间戳** —— 所以这个重跑不会让整个 crate 失效重编（见上）。
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../crates");
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

/// 把「这一次编译」的两格刻进二进制（见上方那段说明）。
///
/// **时间那格故意不在这里**：它由 `lib.rs::exe_modified_local` 在运行时读 exe 的
/// mtime —— 那是链接完成那一刻，既不会早、也不会因为 build.rs 没跑而过期。
/// 全部失败都不挡构建：拿不到就用 `unknown`，**绝不编一个看起来像真的值**。
fn emit_build_fingerprint() {
    // ① 提交号 + 有没有未提交改动。`--always` 让无 tag 的浅克隆也给短 sha
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

    // ② 这一档带没带 workbench。两档共用可执行文件名，光看进程名分不出
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
