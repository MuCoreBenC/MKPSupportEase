//! **未保存改动的文本 diff**（2026-11-02）。
//!
//! # 它回答哪一问
//!
//! 「我改了这些，但还没保存 —— 盘上的那一份是什么样、我把它改成了什么样？」
//!
//! `wb_diff_draft` 走的是**对象视角**（「X 轴偏移：0.7 → 0.9」），它答不了这件事：
//! 改一个值往往连带改了好几行（值、译文、分组次序），最后落到文件里到底是哪几行不同，
//! 只有把**两份文件文本**摊开比才看得见。作者原话：「代码的 diff 就很直观了」。
//!
//! # 比的是哪两份
//!
//! **盘上的 `presets/`（真源）↔ 影子树 `workbench/.draft/presets/`（编辑态）**。
//!
//! 「保存」做的事就是把右边那份一次提交到左边，所以这里摊开的就是**点一次保存会改掉的
//! 全部字节**，不多一行也不少一行。
//!
//! ★ 前提：**值编辑只在内存里**（`Ctx::history`），影子树本身不含它们。所以调用方
//! （`wb_shadow_diff`）必须先 `Ctx::flush_values_into_shadow()` 把这批命令写成影子字节，
//! 再进来比两棵树 —— 少了这一步，本函数会如实报告"两棵树一样"，而页头还挂着「未保存 N」。
//! 本函数自己只读两棵树，一个字节都不写。
//!
//! # 行就是 LCS 的原样，**不配对**
//!
//! 2026-11-02 曾在这一层做过"配对"（把「整块删 + 整块增」合成一行「旧 → 新」），
//! 因为作者看到"只加了一条却在概览条上冒出两道 `−`"。配对的思路没错，但渲染成一行
//! 划掉式在长行上读不动 —— 作者定案：**要生成前确认那种上红下绿两行**
//! （原话：「你看这个生成的就很正常呀，上面是红色的，下面是绿色的」）。
//!
//! 所以这里只做一件事：把两侧文本摊成 `context / added / removed` 交给前端，
//! 红的在上、绿的在下，与生成前确认**同一个渲染器**（`views/DiffView` 的 `DiffLines`）。
//!
//! # 口径与别处一致
//!
//! - 行级 diff 用 `build.rs` 那一份（[`line_diff`]）—— 同一个算法产出
//!   `context / added / removed`（`DiffLines` 组件也同一份）；
//! - **不比 `delivery/`**：生成产物每生成一次就变一次，不是用户的编辑；
//! - **二进制不比行**：资产（webp / png）只报"换了 / 新增 / 删掉"，不假装能读出行；
//! - **顺序与 git 一致**：先把删除的摊完，再把新增的摊上来（作者 2026-11-02：
//!   「怎么只有加，没有减？」—— 反过来先加后删，读者会以为原文里本来就有这几行）。

use std::path::Path;

use serde::Serialize;

use super::txn::{self, FileState};
use crate::error::AppError;
use crate::workbench::app::build::{line_diff, DiffLineKind, PreviewDiffLine};

/// 一份文件的 diff：状态 + 行 + 计数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowFileDiff {
    /// 相对**预设根**的路径（永远用 `/`）
    pub rel: String,
    /// `added` 保存会新建它 / `modified` 改写 / `removed` 删掉
    pub state: FileState,
    /// 行级差异（含未变的上下文行）。二进制为空 —— 见 [`Self::binary`]
    pub lines: Vec<PreviewDiffLine>,
    pub added: usize,
    pub removed: usize,
    /// 非文本（webp / png 那批）：只有"变了"这件事，没有行
    pub binary: bool,
}

/// 整份未保存改动的 diff。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowDiff {
    /// 有变化的文件，按路径排。**空 = 影子与盘上逐字节相同**（没有未保存的改动）
    pub files: Vec<ShadowFileDiff>,
    /// 影子树与盘上各自的根 —— 界面要能说清"比的是哪两份"
    pub shadow_root: String,
    pub real_root: String,
}

impl ShadowDiff {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// 算一份 diff。**只读**：只做快照 + 读字节，一个字节都不写。
pub fn shadow_diff(real_root: &Path, shadow_root: &Path) -> Result<ShadowDiff, AppError> {
    /* 与保存同一道过滤：`delivery/` 不算用户编辑（`without_products`）。 */
    let real = txn::snapshot(real_root)?.without_products();
    let shadow = txn::snapshot(shadow_root)?.without_products();

    let mut files: Vec<ShadowFileDiff> = Vec::new();
    for (rel, state, before, after) in txn::changed_files(&real, &shadow, real_root, shadow_root)? {
        let old = before
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("");
        let new = after
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("");
        /* 任一侧不是文本（二进制）⇒ 只报"变了"。
        注意 `unwrap_or("")` 之后仍是空串的不算二进制 —— 那是真的空文件。 */
        let binary = before
            .as_ref()
            .is_some_and(|b| std::str::from_utf8(b).is_err())
            || after
                .as_ref()
                .is_some_and(|b| std::str::from_utf8(b).is_err());
        if binary {
            files.push(ShadowFileDiff {
                rel,
                state,
                lines: Vec::new(),
                added: 0,
                removed: 0,
                binary: true,
            });
            continue;
        }

        /* 真正摊行时用两侧的真实文本：新增 / 删除的另一侧按空文件算，
        于是"新建一份"整份全绿、"删掉一份"整份全红，与直觉一致 */
        let lines = line_diff(old, new);
        let added = count(&lines, DiffLineKind::Added);
        let removed = count(&lines, DiffLineKind::Removed);
        files.push(ShadowFileDiff {
            rel,
            state,
            lines,
            added,
            removed,
            binary: false,
        });
    }

    Ok(ShadowDiff {
        files,
        shadow_root: shadow_root.display().to_string(),
        real_root: real_root.display().to_string(),
    })
}

fn count(lines: &[PreviewDiffLine], kind: DiffLineKind) -> usize {
    lines.iter().filter(|l| l.kind == kind).count()
}

#[cfg(test)]
// 测试要造两棵树：`std::fs::write` 在这里是正当用法（写盘纪律管的是生产代码）
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::workbench::app::build::DiffLineKind;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn roots() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let real = d.path().join("presets");
        let shadow = d.path().join("workbench/.draft/presets");
        std::fs::create_dir_all(&shadow).unwrap();
        (d, real, shadow)
    }

    /// 两棵树逐字节相同 ⇒ 没有未保存的改动（空清单，不是"每份都无变化"）
    #[test]
    fn identical_trees_produce_no_files() {
        let (_d, real, shadow) = roots();
        write(&real, "machines/A1.toml", "id = 'A1'\nx = 0.7\n");
        write(&shadow, "machines/A1.toml", "id = 'A1'\nx = 0.7\n");
        let out = shadow_diff(&real, &shadow).unwrap();
        assert!(out.is_empty(), "同一份不该出现在清单里：{:?}", out.files);
    }

    /// 改一行 ⇒ 一份 `modified`，**一红一绿两行**（删在前、增在后），上下文行照给。
    ///
    /// 这是作者 2026-11-02 定案的样子：「上面是红色的，下面是绿色的」——
    /// 与生成前确认完全一致，不再折成一行的「旧 → 新」。
    #[test]
    fn one_changed_value_is_one_red_line_then_one_green_line() {
        let (_d, real, shadow) = roots();
        write(&real, "machines/A1.toml", "id = 'A1'\nx = 0.7\nz = 3.9\n");
        write(&shadow, "machines/A1.toml", "id = 'A1'\nx = 0.9\nz = 3.9\n");
        let out = shadow_diff(&real, &shadow).unwrap();
        assert_eq!(out.files.len(), 1);
        let f = &out.files[0];
        assert_eq!(f.rel, "machines/A1.toml");
        assert_eq!(f.state, FileState::Modified);
        assert!(!f.binary);
        assert_eq!((f.added, f.removed), (1, 1));
        let kinds: Vec<_> = f.lines.iter().map(|l| (l.kind, l.text.as_str())).collect();
        assert_eq!(
            kinds,
            vec![
                (DiffLineKind::Context, "id = 'A1'"),
                (DiffLineKind::Removed, "x = 0.7"),
                (DiffLineKind::Added, "x = 0.9"),
                (DiffLineKind::Context, "z = 3.9"),
            ],
            "删的一定在增的前面（先红后绿）"
        );
    }

    /// 纯新增（原文一个字没动）⇒ **一条删行都不许报**。作者 2026-11-02 报的就是这个：
    /// 「怎么只有加，没有减？原本的文件是空着的？」
    #[test]
    fn pure_insertion_reports_no_removals() {
        let (_d, real, shadow) = roots();
        let old = "a = 1\n\n[params.machineVariants]\n";
        let new = "a = 1\n\n[params.machineVariants]\n\"A1:STANDARD\" = 75.0\n\"A1:FAST\" = 75.0\nA1 = 75.0\n";
        write(&real, "t.toml", old);
        write(&shadow, "t.toml", new);
        let out = shadow_diff(&real, &shadow).unwrap();
        let f = &out.files[0];
        assert_eq!(f.removed, 0, "纯追加不该有删行：{:#?}", f.lines);
        assert_eq!(f.added, 3);
    }

    /// 影子里新多一份 / 少一份：方向必须是"保存会新建 / 会删掉"，不是反的
    #[test]
    fn direction_is_disk_then_shadow() {
        let (_d, real, shadow) = roots();
        write(&real, "old.toml", "a = 1\n");
        write(&shadow, "new.toml", "b = 2\n");
        let out = shadow_diff(&real, &shadow).unwrap();
        let by_rel = |rel: &str| {
            out.files
                .iter()
                .find(|f| f.rel == rel)
                .unwrap_or_else(|| panic!("清单里没有 {rel}：{:?}", out.files))
        };
        assert_eq!(by_rel("new.toml").state, FileState::Added);
        assert_eq!(by_rel("old.toml").state, FileState::Removed);
        // 新建的那份整份全绿（另一侧按空文件算）
        assert!(by_rel("new.toml")
            .lines
            .iter()
            .all(|l| l.kind == DiffLineKind::Added));
        assert!(by_rel("old.toml")
            .lines
            .iter()
            .all(|l| l.kind == DiffLineKind::Removed));
    }

    /// `delivery/` 是生成产物，每次生成都会变 —— 不算用户的未保存改动
    #[test]
    fn delivery_is_not_an_unsaved_change() {
        let (_d, real, shadow) = roots();
        write(&real, "machines/A1.toml", "id = 'A1'\n");
        write(&shadow, "machines/A1.toml", "id = 'A1'\n");
        write(&real, "delivery/mkp/presets/A1.toml", "old\n");
        write(&shadow, "delivery/mkp/presets/A1.toml", "new\n");
        let out = shadow_diff(&real, &shadow).unwrap();
        assert!(out.is_empty(), "delivery/ 不该进清单：{:?}", out.files);
    }

    /// 二进制只报"变了"，不假装能读出行
    #[test]
    fn binaries_report_state_without_lines() {
        let (_d, real, shadow) = roots();
        let png = [0x89u8, b'P', b'N', b'G', 0x00, 0xff, 0xfe];
        for root in [&real, &shadow] {
            std::fs::create_dir_all(root.join("assets/guide")).unwrap();
        }
        std::fs::write(real.join("assets/guide/on-open.webp"), png).unwrap();
        let mut other = png;
        other[5] = 0x01;
        std::fs::write(shadow.join("assets/guide/on-open.webp"), other).unwrap();

        let out = shadow_diff(&real, &shadow).unwrap();
        assert_eq!(out.files.len(), 1);
        let f = &out.files[0];
        assert!(f.binary, "webp 该按二进制处理");
        assert!(f.lines.is_empty());
        assert_eq!((f.added, f.removed), (0, 0));
        assert_eq!(f.state, FileState::Modified);
    }
}
