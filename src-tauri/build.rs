/*
 * 构建脚本 —— **把两个官方源（Bootstrap）地址注入二进制**。
 *
 * # 值的来源与优先级（2026-10-02 第十七刀）
 *
 *   ① 环境变量 `MKPSE_PRESET_SOURCE` / `MKPSE_PRESET_SOURCE_GITEE`：构建方显式覆盖
 *      （CI 打测试包用的就是它，也有权覆盖一切）——**在就不动它**；
 *   ② 工作台配置 `<repo>/workbench/bootstrap.json`（`wb_set_bootstrap` 写的，入库）：
 *      `npm run tauri dev` 与正式构建都从这里自动拿到官方源；
 *   ③ 都没有 → 二进制里没有那个默认地址，联网命令诚实回答「还没配置」，
 *      **不编一个 URL 出来假装能下**。
 *
 * # 语义
 *
 * 这两个值都是 **Bootstrap 地址**（指向发布产物里的 `source.json`），不是"直接根" ——
 * 客户端读它拿到 catalog 与文件下载根的地址（见 `runtime::source::resolve_source`）。
 *
 * ★ **两个都是官方源**（2026-10-05，作者拍"国内走 Gitee"）：`bootstrapUrl` = GitHub
 *   （默认），`giteeBootstrapUrl` = Gitee。**各自独立注入**：Gitee 那边没配就少一个
 *   选项（界面不出现它），不影响 GitHub 那个。
 *
 * # 改了配置什么时候生效
 *
 * 它是**编译期**注入：改完 `workbench/bootstrap.json` 要重新构建（dev 下重启
 * `tauri dev`）；`rerun-if-changed` 登记了它，cargo 会帮忙看见变化。
 */

fn main() {
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE");
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE_GITEE");
    println!("cargo:rerun-if-changed=../workbench/bootstrap.json");

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
