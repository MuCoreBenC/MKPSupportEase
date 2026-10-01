//! 层模板嵌入（对照 embed_templates.go：go:embed 的 include_str! 等价物）。

const TOWER_LAYER_GCODE: &str = include_str!("../../../templates/tower_layer.gcode");

fn split_template(s: &str) -> Vec<String> {
    let mut result: Vec<String> = s.split('\n').map(|l| l.to_string()).collect();
    if let Some(last) = result.last()
        && last.is_empty()
    {
        result.pop();
    }
    result
}

/// `GetWipingGcodeLines`：pass2 的塔层模板行。
pub fn get_wiping_gcode_lines() -> Vec<String> {
    split_template(TOWER_LAYER_GCODE)
}
