//! 切片器「看不看得见」这份产物 —— 机械判据，防的是 2026-11-12 那个坑。
//!
//! # 那个坑
//!
//! 调试产物里补了一条虚拟挤出（`E0.001`、0.1 mm），切片器却仍报「路径在床外」、
//! 打印按钮全灰，而且**只有第一盘能打**。
//!
//! 原因不在坐标：产物开头那段 `; FEATURE: Custom` 会让它后面**所有**移动的挤出角色
//! 都变成 `erCustom`，而切片器建路径包围盒时明确排除这个角色
//! （`LegacyRenderer` 与 `BuildVolume::all_paths_inside` 两处都是 `role != erCustom`）。
//! 于是一条货真价实的挤出也没进包围盒 —— 包围盒保持空盒 `(0,0,0)`，
//! 而这个空盒恰好落在第一盘的床内、落在其余盘的床外（那些床在场景里是另一格），
//! 现象就成了「只有第一盘能打印」。
//!
//! # 判据（照抄切片器那两处，只判"它一定会排除"的几种）
//!
//! - 是挤出移动：带 E 且 **E > 0**（回抽是负的，不算挤出）
//! - **有 X 或 Y**：只写 E 的行算不出线宽（`delta_xyz == 0`），切片器同样排除
//! - 当前角色**不是 `Custom`**（`; FEATURE:` 一直生效到下一个 FEATURE 标记）
//!
//! 宽度公式不在这里复刻：宁可放过，不可误杀。

use crate::gcode::coord::parse_xyze;
use crate::gcode::postprocess::get_e_value;

/// `; FEATURE: Outer wall` 里的角色名。
fn feature_of(line: &str) -> Option<&str> {
    let at = line.find("; FEATURE:")?;
    Some(line[at + "; FEATURE:".len()..].trim())
}

/// 这一行是不是挤出移动（`has_x` / `has_y` 由 [`parse_xyze`] 判，要求小数点两侧都有数字）。
fn is_extrude_move(line: &str) -> bool {
    let Some(e) = get_e_value(line) else {
        return false;
    };
    if e <= 0.0 {
        return false;
    }
    let pos = parse_xyze(line.as_bytes());
    pos.has_x || pos.has_y
}

/// 这份产物里有没有一条**会被切片器算进路径包围盒**的挤出移动。
///
/// `false` 就意味着：包围盒是空的，切片器会把它判成「路径在床外」，
/// 第一盘之外全都发不了打印 —— 而且**不会**提示「没有路径」。
pub fn has_countable_extrude(lines: &[String]) -> bool {
    let mut custom = false;
    for line in lines {
        if let Some(role) = feature_of(line) {
            custom = role.eq_ignore_ascii_case("custom");
            continue;
        }
        if custom {
            continue;
        }
        if is_extrude_move(line) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// 2026-11-12 的原样复现：有挤出，但它在 `; FEATURE: Custom` 之下 ⇒ 切片器看不见。
    #[test]
    fn a_custom_role_hides_every_extrude() {
        let src = lines(&[
            "; FEATURE: Custom",
            "G1 X20.1 E0.001 F10000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility",
        ]);
        assert!(!has_countable_extrude(&src));
    }

    /// 同一个文件，只要角色不是 Custom 就算数 —— 这正是那次的修法。
    #[test]
    fn a_real_role_counts() {
        let src = lines(&[
            "; FEATURE: Custom",
            "; FEATURE: Outer wall",
            "G1 X20.1 E0.001 F10000",
        ]);
        assert!(has_countable_extrude(&src));
    }

    /// 回抽（E 为负）与只写 E 的行都不算：前者不是挤出，后者算不出线宽。
    #[test]
    fn retractions_and_pure_e_moves_do_not_count() {
        let src = lines(&[
            "; FEATURE: Outer wall",
            "G1 X20.1 E-0.5 F1800",
            "G1 E5 F3000",
        ]);
        assert!(!has_countable_extrude(&src));
    }

    /// 角色名大小写不敏感（切片器自己也不敏感），且 `Custom` 只在它自己的作用域里生效。
    #[test]
    fn custom_scope_ends_at_the_next_feature() {
        let src = lines(&[
            "; FEATURE: custom",
            "G1 X1.0 E0.01 F3000",
            "; FEATURE: Sparse infill",
            "G1 X2.0 E0.01 F3000",
        ]);
        assert!(has_countable_extrude(&src));
    }
}
