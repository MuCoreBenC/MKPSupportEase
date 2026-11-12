//! 测试用的最小 presets 树。
//!
//! 演练型判据（删版本 / 改 id / 撞车 / 复制 / 批量写值）要的是**可控输入**：
//! 由测试自己造一棵能过 `Presets::load_from` 与 `check_cross_consistency` 的最小树，
//! 不从 `presets/` 复制、也不在正式数据里找样本。
//!
//! **只提供最小合法数据**，不预设任何业务结果 —— 断言仍然写在各自的判据里。
//!
//! 为什么要有它：此前这些判据 `copy_tree(presets/)` 再动态找一个"符合条件的版本"。
//! 那只是把依赖从文件路径转移到了样本发现过程 —— 正式数据换一版，测试就可能找不到
//! 样本、或换一个样本后行为悄悄变了。树由测试自己写，这两件事就都不会发生。

use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// 一棵最小树的身份。全部由 [`minimal_presets_tree`] 决定，与正式数据无关。
pub(crate) struct MinimalTree {
    dir: TempDir,
    /// 唯一的机型
    pub(crate) machine: String,
    /// 带归属资产、且被套餐引用的版本（删 / 改 id 演练用它）
    pub(crate) version_with_assets: String,
    /// 另一个版本（撞车 / 复制 / 批量写值演练用它）
    pub(crate) other_version: String,
    /// 注册表里那一条参数
    pub(crate) param: String,
    /// 套餐 id
    pub(crate) bundle: String,
    /// `version_with_assets` 的 MKP 预设资产
    pub(crate) asset_of_version: String,
    /// 套餐里那条 BBS（check_cross_consistency 要求套餐至少一条）
    pub(crate) bbs_asset: String,
}

impl MinimalTree {
    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    /// 机型文件（撞车演练要往它尾部 append 一个版本）。
    pub(crate) fn machine_file(&self) -> PathBuf {
        self.dir
            .path()
            .join("machines")
            .join(format!("{}.toml", self.machine))
    }

    /// 注册表里那一项的 `机型:版本` 键（`M1:V1` 这种形状）。
    pub(crate) fn versioned_param_key(&self, version: &str) -> String {
        format!("{}:{version}", self.machine)
    }
}

/// 造一棵最小树：一台机型、两个版本、一条参数（版本层覆盖在 V1 上）、
/// 一条套餐（V1 的 MKP 预设 + 一条 BBS）。
pub(crate) fn minimal_presets_tree() -> MinimalTree {
    let tree = MinimalTree {
        dir: tempfile::tempdir().expect("临时目录"),
        machine: "M1".to_owned(),
        version_with_assets: "V1".to_owned(),
        other_version: "V2".to_owned(),
        param: "toolhead.MKP_retract".to_owned(),
        bundle: "M1_V1".to_owned(),
        asset_of_version: "m1-v1".to_owned(),
        bbs_asset: "m1-bbs".to_owned(),
    };
    let root = tree.path();

    write(
        root,
        "brands.toml",
        "[[brands]]\nid = 'Bambu Lab'\nname = '拓竹'\n",
    );

    write(
        root,
        &format!("machines/{}.toml", tree.machine),
        &format!(
            "id = '{m}'\ndisplay = '{m}'\nname = '{m}'\nbrand = 'Bambu Lab'\n\n\
             [[versions]]\nid = '{v1}'\nname = '第一版'\n\n\
             [[versions]]\nid = '{v2}'\nname = '第二版'\n",
            m = tree.machine,
            v1 = tree.version_with_assets,
            v2 = tree.other_version,
        ),
    );

    write(
        root,
        "assets.toml",
        &format!(
            "[[assets]]\nid = '{a1}'\ntype = 'mkPreset'\nmachineId = '{m}'\n\
             versionId = '{v1}'\nname = '第一版预设'\n\n\
             [[assets]]\nid = '{a2}'\ntype = 'mkPreset'\nmachineId = '{m}'\n\
             versionId = '{v2}'\nname = '第二版预设'\n\n\
             [[assets]]\nid = '{bbs}'\ntype = 'slicerProfile'\nmachineId = '{m}'\n\
             name = 'BBS 0.4'\npath = 'slicer/m1-bbs.json'\nslicer = 'bbs'\n\
             profile = 'process'\n",
            a1 = tree.asset_of_version,
            a2 = asset_of_other_version(&tree),
            bbs = tree.bbs_asset,
            m = tree.machine,
            v1 = tree.version_with_assets,
            v2 = tree.other_version,
        ),
    );

    write(
        root,
        "bundles.toml",
        &format!(
            "[[bundles]]\nid = '{bundle}'\ndisplay = '第一版套餐'\nmachineId = '{m}'\n\
             assetRefs = ['{a1}', '{bbs}']\n",
            bundle = tree.bundle,
            m = tree.machine,
            a1 = tree.asset_of_version,
            bbs = tree.bbs_asset,
        ),
    );

    write(
        root,
        "registry/param_registry.toml",
        &format!(
            "[[tabs]]\nid = 'motion'\nlabel = '运动'\norder = 10.0\n\n\
             [[tabs.sections]]\nid = 'motion'\nlabel = '运动与挤出'\norder = 100.0\n\n\
             [[params]]\n\
             key = '{key}'\nconfigKey = 'MKPRetract'\ntomlKey = 'MKP_retract'\n\
             jsonKey = 'MKP_retract'\nlabel = '回抽长度'\ndesc = '涂胶笔回抽长度'\n\
             tomlComment = '回抽长度(mm)'\nvalueType = 'float'\nuiComponent = 'number'\n\
             defaultValue = -5.0\nscope = 'universal'\nsection = 'toolhead'\n\
             layout = {{ order = 200.0, sectionId = 'motion' }}\n\n\
             [params.machineVariants]\n'{v1key}' = -3.5\n",
            key = tree.param,
            v1key = tree.versioned_param_key(&tree.version_with_assets),
        ),
    );

    write(
        root,
        "layout_schema.toml",
        &format!(
            "[[tabs]]\nid = 'motion'\nlabel = '运动'\n\n\
             [[tabs.sections]]\nid = 'motion'\n\n\
             [[tabs.sections.items]]\nid = 'item_0'\nparamKey = '{key}'\n",
            key = tree.param,
        ),
    );

    tree
}

/// 另一个版本那条 MKP 预设的 id（只在 assets.toml 内部用）。
fn asset_of_other_version(tree: &MinimalTree) -> String {
    format!("{}-v2", tree.machine.to_lowercase())
}

fn write(root: &Path, rel: &str, text: &str) {
    let at = root.join(rel);
    std::fs::create_dir_all(at.parent().expect("夹具必有父目录")).expect("建目录");
    crate::fsx::atomic::atomic_write(&at, text.as_bytes()).expect("写夹具");
}
