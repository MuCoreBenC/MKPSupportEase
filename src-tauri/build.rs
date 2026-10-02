/*
 * 构建脚本 —— 今天只做一件事：**把官方源（Bootstrap）地址注入二进制**。
 *
 * # 值的来源与优先级（2026-10-02 第十七刀）
 *
 *   ① 环境变量 `MKPSE_PRESET_SOURCE`：构建方显式覆盖（CI 打测试包用的就是它，
 *      也有权覆盖一切）——**在就不动它**；
 *   ② 工作台配置 `<repo>/workbench/bootstrap.json`（`wb_set_bootstrap` 写的，入库）：
 *      `npm run tauri dev` 与正式构建都从这里自动拿到官方源；
 *   ③ 都没有 → 二进制里没有默认地址，联网命令诚实回答「还没配置」，
 *      **不编一个 URL 出来假装能下**。
 *
 * # 语义
 *
 * 这个值是一个 **Bootstrap 地址**（指向发布产物里的 `source.json`），不是"直接根" ——
 * 客户端读它拿到 catalog 与文件下载根的地址（见 `runtime::source::resolve_source`）。
 *
 * # 改了配置什么时候生效
 *
 * 它是**编译期**注入：改完 `workbench/bootstrap.json` 要重新构建（dev 下重启
 * `tauri dev`）；`rerun-if-changed` 登记了它，cargo 会帮忙看见变化。
 */

fn main() {
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE");
    println!("cargo:rerun-if-changed=../workbench/bootstrap.json");

    if std::env::var_os("MKPSE_PRESET_SOURCE").is_none() {
        if let Some(url) = bootstrap_url_from_workbench() {
            println!("cargo:rustc-env=MKPSE_PRESET_SOURCE={url}");
        }
    }

    tauri_build::build()
}

/// 读工作台配置里那一个字段。**读不出 / 解析不出 / 是空串都当"没配"**（返回 `None`）——
/// 构建脚本不挡构建：真正的解释权在运行时（没注入就是「还没配置」那一档）；
/// 手改坏的配置下次在工作台里再保存一次就会被覆盖成好档。
fn bootstrap_url_from_workbench() -> Option<String> {
    let text = std::fs::read_to_string("../workbench/bootstrap.json").ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let url = value.get("bootstrapUrl")?.as_str()?.trim();
    if url.is_empty() {
        return None;
    }
    Some(url.to_owned())
}
