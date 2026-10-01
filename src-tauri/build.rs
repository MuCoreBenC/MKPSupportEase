fn main() {
    /* 数据源地址的默认值由**构建方**注入：`MKPSE_PRESET_SOURCE=<url> cargo build`。
     *
     * 变量没设，二进制里就没有默认地址，下载命令诚实回答「还没配置数据源地址」——
     * 不编一个 URL 出来假装能下。官方源 / Gitee 的真实地址是一次产品决定，
     * 不属这里猜的范围：它随发布通道变（GitHub Pages？对象存储？），所以留成构建期的。 */
    println!("cargo:rerun-if-env-changed=MKPSE_PRESET_SOURCE");
    if let Ok(value) = std::env::var("MKPSE_PRESET_SOURCE") {
        println!("cargo:rustc-env=MKPSE_PRESET_SOURCE={value}");
    }

    tauri_build::build()
}
