//! 虚拟挤出：一条**只有 0.1 mm、E0.001** 的假挤出移动。
//!
//! # 为什么要有它
//!
//! Bambu Studio 判断「这盘能不能打印」靠产物里的**挤出点**：它拿挤出移动的坐标合成
//! 一个路径包围盒，再问这个盒是不是整个在床内（`LegacyRenderer` 建盒、
//! `BaseRenderer::update_toolpath_outside_state`、判据 `PartPlate::is_slice_result_ready_for_print`）。
//! 一条挤出都没有的产物（校准件、只涂胶、调试动作）会被判成「路径在床外」，
//! 切片完成那一屏的打印 / 发送全灰。
//!
//! 补一条这样的移动就把那个判据撑住：包围盒有了、在床内，按钮就不灰。
//! 真机几乎不出料（E0.001），但它在 G-code 里是一条货真价实的挤出移动。
//!
//! # 位置怎么给才对
//!
//! - **长度不能是 0**：原地挤出算不出线宽（`width = 体积 / 长度`），BBS 会当成没宽度的点
//!   而不参与包围盒。所以惯例是「先到位，再往外走 0.1 mm」—— 锚点因此落在原位本身
//!   （反过来先到 +0.1 再挤回原位，预览会从上一条挤出终点连一条线头，2026-11-05 涂胶定的）。
//! - **高差要提前抬**：这一行只有 XY（或写死一个安全 Z），不会替调用方抬 Z。
//!   要在低 Z 上用，先把 Z 抬到安全高度再平移，否则喷嘴贴着板拖过去
//!   （校准块的 `CALIBRATION_PRELIFT` 就是为这个）。
//!
//! 校准块（2026-10）与涂胶锚点（2026-11）本来各写各的一行，这里收成一处：
//! 新地方要用直接调 [`line`]，别再手拼 —— E 值、标记文本、参数顺序以前三处三个样。

/// 行尾标记。`filter_e_moves` 靠它把这一行当「假挤出」放过（见 [`is_tagged`]）。
pub const TAG: &str = "MKPSE: virtual extrusion for BBS G-code viewer compatibility";

/// 挤出量（mm）。够让 BBS 认出「这是一条有宽度的挤出移动」，真机几乎不出料。
pub const E: &str = "0.001";

/// 这一行是不是虚拟挤出。
///
/// 认的是宽口径的 `virtual extrusion`（不是 [`TAG`] 全串）：旧产物里那句标记的措辞
/// 改过，按全串认会把当年的行当成真挤出剔掉。
pub fn is_tagged(line: &str) -> bool {
    line.contains("virtual extrusion")
}

/// 一条虚拟挤出。
///
/// `xy` 是**已经拼好**的坐标段（`"X100 Y100 Z10"` / `"X20.1 Y0"`），`feed` 是已经格式化
/// 好的进给（`"3000"`）—— 数字格式各调用点不一样（校准写整数、涂胶走 `format_z`），
/// 硬在这里格式化会改掉它们的产物字节。
pub fn line(xy: &str, feed: &str) -> String {
    format!("G1 {xy} E{E} F{feed} ; {TAG}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与校准块当年那一行逐字相同 —— 这条是「收成一处」时不许改产物字节的证据。
    #[test]
    fn calibration_line_is_byte_identical() {
        assert_eq!(
            line("X100 Y100 Z10", "3000"),
            "G1 X100 Y100 Z10 E0.001 F3000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility"
        );
    }

    /// 涂胶那一档（无 Z）与调试那一档（无 Y）共用同一个拼法。
    #[test]
    fn line_keeps_the_callers_formatting() {
        assert_eq!(
            line("X30.1 Y0.0", "42000.0"),
            "G1 X30.1 Y0.0 E0.001 F42000.0 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility"
        );
        assert_eq!(
            line("X20.1", "10000"),
            "G1 X20.1 E0.001 F10000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility"
        );
    }

    /// 标记认宽口径：旧措辞也要认得（`filter_e_moves` 靠它不把旧行当真挤出）。
    ///
    /// 只按词认、不按全串 —— 标记文案改过措辞，按 [`TAG`] 全串比会把当年的行
    /// 当成真挤出剔掉。只有带 E 的行才会走到这个判据，所以宽一点不会误伤注释行。
    #[test]
    fn tagged_lines_are_recognized() {
        assert!(is_tagged(&line("X1 Y1", "3000")));
        assert!(is_tagged("G1 X2 Y2 E0.5 ; virtual extrusion (old wording)"));
        assert!(!is_tagged("G1 X2 Y2 E0.5"));
    }
}
