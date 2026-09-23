//! 上游只读层 —— `mkpse-presets` 里**还没搬过来**的那几份数据。
//!
//! **这一层只读，而且正在缩小。** b04 Task 9 把字段定义与布局搬进了
//! [`crate::workbench::presets`]（连同那三条一致性断言），所以这里只剩资源与套餐清单、
//! 机型的产物视图、以及应急规则。Task 12 会把它们也搬完，然后整个目录删掉。
//!
//! 三个子模块，每个文件**只有一个所有者**：
//!
//! | 子模块 | 读什么 | 实测规模 |
//! |---|---|---|
//! | [`manifest`] | `manifest.json` + `content/preset_registry.json` | 72 资源（18 交付物 + 54 素材）/ 5 套餐 / 15 内容文件 |
//! | [`catalog`] | `content/machine_catalog.json`（+ 清单的机型视图） | 6 机型 / 10 版本 / 5 份尺寸（缺 A2L）/ 3 份禁区 |
//! | [`fallback`] | `content/fallback_registry.json` | 20 条 + guide 长文 |
//!
//! 机型与版本的**清单**已经不在这一层了（b04 Task 8：清单来自
//! `presets/machines/*.toml`）。这里的 [`catalog`] 现在只剩一个用处：
//! 按 `(机型, 版本)` 查它的产物（`mkpPresetAssetId`）。
//!
//! # 刻意不读的两个文件
//!
//! - `content/assets_index.json`：18 条，`sha256` 全空、`size` 全 0、`isRegistered`
//!   全 false，而这 18 个路径**全部**已在 `manifest.assets` 里登记。读它会让文件清单页
//!   给已登记的文件打「未登记」、给有真哈希的文件显示「大小未知」。
//! - `content/bundles.json`：与 `manifest.bundles` 逐字节相同，且不在 `contentFiles`
//!   里 —— 客户端从头到尾不会下载它。
//!
//! 理由写在 [`manifest`] 的模块文档里。
//!
//! # 一致性断言都在这一层
//!
//! 上游没有这些门禁。它们不成立时的后果**没有一条会自己报错**：矩阵静默少行、
//! 多出永远为空的分类、把 X 和 Y 调个头、套餐指向一个下载时才 404 的文件、
//! 客户端说「暂无尺寸」而工作台显示有值。所以判据必须在加载时就拦住。
//!
//! # 这一层没有任何写入函数
//!
//! 不是"约定不写"，是有一条**扫源码的断言**盯着（`upstream_layer_has_no_write_path`）：
//! 非测试代码里出现写盘相关的符号就红。`clippy.toml` 只拦 `std::fs::write`，
//! 拦不住走仓库那个合法出口写到上游路径去。

pub mod catalog;
pub mod fallback;
pub mod manifest;

pub use catalog::{Catalog, Machine, MachineVersion};
pub use fallback::FallbackRegistry;
pub use manifest::{Asset, Bundle, Manifest, ResourceType};

use std::path::Path;

use crate::error::AppError;
use crate::workbench::paths;

/// 一次读齐的上游快照。
///
/// 为什么要有这个聚合：几份数据之间有交叉断言（版本指向的产物在不在、尺寸两份对不对）。
/// 分开各读一次，就没有任何时刻能把它们放在一起查。
///
/// **它是一个快照，不是缓存。** 读完之后上游文件改了，这份不会跟着变 ——
/// 这是刻意的：一次操作里各处看到的上游必须是同一份，否则派生出来的状态会自相矛盾。
pub struct Upstream {
    pub manifest: Manifest,
    pub catalog: Catalog,
    pub fallback: FallbackRegistry,
}

impl std::fmt::Debug for Upstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Upstream({:?} / {:?} / {:?})",
            self.manifest, self.catalog, self.fallback
        )
    }
}

impl Upstream {
    pub fn load() -> Result<Self, AppError> {
        let root = paths::upstream_root().ok_or_else(|| {
            AppError::not_found("找不到上游预设仓库 mkpse-presets")
                .with_detail(format!("试过：{}", paths::upstream_candidates().join("；")))
        })?;
        Self::load_from(&root)
    }

    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let manifest = Manifest::load_from(root)?;
        let catalog = Catalog::load_from(root, &manifest)?;
        Ok(Self {
            fallback: FallbackRegistry::load_from(root)?,
            manifest,
            catalog,
        })
    }
}

#[cfg(test)]
mod tests {
    /// **上游只读层零写入**。
    ///
    /// 为什么要扫源码而不是靠 code review：往上游写一行的后果是**静默的** ——
    /// 本机看起来一切正常，直到下一次 mkppanel 全量重建把它覆盖掉，
    /// 而那次覆盖不会有任何报错。`clippy.toml` 只拦 `std::fs::write`，
    /// 拦不住走 `fsx::atomic` 写到上游路径去。
    ///
    /// 判据落在**源码文本**上而不是类型上：Rust 没法表达"这个模块不许调用某个函数"。
    #[test]
    fn upstream_layer_has_no_write_path() {
        let dir = std::path::Path::new(file!())
            .parent()
            .unwrap()
            .to_path_buf();
        let mut scanned = 0usize;
        for e in std::fs::read_dir(&dir).expect("读不出上游层目录").flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            scanned += 1;

            for bad in [
                "atomic_write",
                "fs::write",
                "fs::remove",
                "create_dir_all",
                "File::create",
                "OpenOptions",
            ] {
                assert!(
                    !production_part(&src).contains(bad),
                    "{name} 的非测试代码里出现了 {bad} —— 上游只读，写进去会被下一次上游构建静默覆盖"
                );
            }
        }
        assert!(scanned >= 3, "只扫到 {scanned} 个文件，判据在空转");
    }

    /// 测试夹具要造临时上游，所以只查 `#[cfg(test)]` 之前的那一段；注释行也去掉 ——
    /// 本层的模块文档里就写着"没有一个调用 atomic_write 的口子"，
    /// 照原文扫会被自己的说明绊倒（第一版就是这么红的）
    fn production_part(src: &str) -> String {
        src.split("#[cfg(test)]")
            .next()
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 反空转：上面那条判据得真能抓到东西。
    /// 少了这一条，某天把 `production_part` 改坏（比如把整份源码都过滤掉），
    /// 零写入判据会变成一条永远通过的绿灯
    #[test]
    fn the_zero_write_judge_would_catch_a_write() {
        let fake = "pub fn save() {\n    atomic_write(&p, b\"x\").unwrap();\n}\n";
        assert!(
            production_part(fake).contains("atomic_write"),
            "判据对真的写入都视而不见"
        );
        // 注释里的同名符号不该算
        let commented = "// 这里不调用 atomic_write\npub fn read() {}\n";
        assert!(!production_part(commented).contains("atomic_write"));
    }

    /// 聚合读一次真上游，跨文件那条断言也要过
    #[test]
    fn real_upstream_loads_as_one_snapshot() {
        let Some(root) = crate::workbench::paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let u = super::Upstream::load_from(&root).expect("三份上游数据读不齐或交叉断言不过");
        assert!(!u.catalog.machines().is_empty());
        assert!(!u.manifest.deliverables().is_empty());
        assert!(!u.fallback.rules().is_empty());
    }
}
