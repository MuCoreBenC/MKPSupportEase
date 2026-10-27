//! 3MF 模型名 —— 「这一盘切的是哪个模型」，**三级兜底链，谁先有算谁**。
//!
//! 背景（作者 2026-10-11 真机核对，两份真记录 `帕姆.3mf` / `deepfish.3mf`）：
//! G-code 自己**没有** 3MF 原名。它里面只有 `; filename_format = {input_filename_base}…`
//! 这种命名模板，而 `{input_filename_base}` 展开出来的是切片器的内部临时名（`.63288.2`），
//! 于是 [`three_mf_file_name`](super::three_mf_file_name) 只能拼出 `.63288.2_.gcode.3mf` ——
//! 那条路拿不到「帕姆」。名字躺在 G-code 旁边的三处旁证里：
//!
//! 1. `<gcode 目录>/../origin.txt` —— 一行文本 = **原始 3MF 的绝对路径**（首选，最准）；
//! 2. G-code 同目录同名的 `<名>.3mf`（打包前的 gcode.3mf，zip）内
//!    `Metadata/slice_info.config` 的 `<object … name="…">` 属性（次选）；
//! 3. `<gcode 目录>/../_temp_1.config`（JSON）的 `filament_settings_id[0]` / `name` 里
//!    那个括号段（三级，`… @BBL A1M(帕姆.3mf)` → `帕姆`）。
//!
//! 三级都没有 ⇒ `None`。**不编名字**：页面上就只显示 G-code 名。
//!
//! # 时机是硬要求
//!
//! 这三处旁证都住在切片器的临时目录里（`%TEMP%\bamboo_model\<日期>\<时刻>#<id>#<rand>`），
//! G-code 一旦被复制/导出到别处就都够不着了。所以本函数**在钩子刚拿到路径时读一次**，
//! 结果落进执行记录（`_meta.json` 的 `modelName`）；报告页此后只读那份记录，不再读盘。
//!
//! # 刻意不走的两条路
//!
//! - 扫 BambuStudio 进程内存 / 扒窗口标题（`GetWindowText`）：脆弱，多实例即失效；
//! - 工程 `.3mf` 里的 `model_settings.config`：那是**零件级**名字（`hat.obj` / `skirt.obj` …
//!   一个 3MF 多个零件），不能当模型名；gcode.3mf 里那份更只有 `<plate>` 块，一个对象名都没有。
//!
//! # zip 只读一段文本，不引 zip 包
//!
//! 要的东西在 `.gcode.3mf` 里只有一个条目（`Metadata/slice_info.config`，真机 1.8 KB）。
//! 引一个 zip 包等于把一串新依赖塞进依赖树（本仓的版本号只住 `[workspace.dependencies]`
//! 一处、构建要能离线跑），代价与收益不成比例 —— 这里按 zip 的**本地文件头**扫一遍，
//! `deflate` 那一档交给依赖树里本来就有的 `flate2`（见根 `Cargo.toml` 那段）。
//! 形态登记（明知而窄，遇到就进下一级、不报错）：不认 zip64 与加密条目；
//! `origin.txt` / `_temp_1.config` 按 UTF-8 解（真机两份都是 UTF-8、无 BOM），换成别的
//! 本地编码会解成花字 —— 那时该修的是这里，不是往上层多加一层猜。

use std::path::Path;

/// 3MF 模型名。三级兜底链，谁先有算谁；三级都没有 ⇒ `None`。
pub fn model_name(gcode_path: &Path) -> Option<String> {
    from_origin_txt(gcode_path)
        .or_else(|| from_slice_info(gcode_path))
        .or_else(|| from_temp_config(gcode_path))
}

/// 首选：`<gcode 目录>/../origin.txt` —— 一行文本 = 原始 3MF 的绝对路径。
fn from_origin_txt(gcode_path: &Path) -> Option<String> {
    let text = read_text(&temp_root(gcode_path)?.join("origin.txt"))?;
    non_empty(stem_of(text.lines().next().unwrap_or("")))
}

/// 次选：G-code 同目录同名的 `.3mf`（打包前的 gcode.3mf）里 `Metadata/slice_info.config`
/// 的对象名。多对象拼 `名字A+名字B`（见 [`object_names`]）。
fn from_slice_info(gcode_path: &Path) -> Option<String> {
    let bytes = std::fs::read(gcode_path.with_extension("3mf")).ok()?;
    let xml = zip_entry_text(&bytes, "Metadata/slice_info.config")?;
    let names = object_names(&xml);
    (!names.is_empty()).then(|| names.join("+"))
}

/// 三级：`<gcode 目录>/../_temp_1.config`（JSON）里那个括号段。
fn from_temp_config(gcode_path: &Path) -> Option<String> {
    let bytes = std::fs::read(temp_root(gcode_path)?.join("_temp_1.config")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let id = v
        .get("filament_settings_id")
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.as_str())
        .or_else(|| v.get("name").and_then(|x| x.as_str()))?;
    non_empty(stem_of(last_paren_segment(id)?))
}

/// 切片器的临时目录 = G-code 所在目录（`Metadata`）的上一级。
///
/// 不校验目录名：G-code 被搬到别处时这里读不到东西（结果就是 `None`）—— 那正是要的
/// （搬走之后模型名**应当**丢，不编一个出来）。
pub(crate) fn temp_root(gcode_path: &Path) -> Option<&Path> {
    gcode_path.parent()?.parent()
}

fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn non_empty(s: String) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// 取路径最后一段并去掉扩展名 —— **两个分隔符都认**：`origin.txt` 里写的是切片器那台
/// 机器的绝对路径（真机是 Windows 的反斜杠），而判据还要在 Linux/macOS 上跑。
fn stem_of(path: &str) -> String {
    let last = path.rsplit(['/', '\\']).next().unwrap_or("").trim();
    match last.rfind('.') {
        Some(i) if i > 0 => last[..i].to_string(),
        // 没有点、或整段就是个隐藏名（`.3mf`）⇒ 原样
        _ => last.to_string(),
    }
}

/// 末尾那个**配平**的括号段：`Bambu PLA Basic @BBL A1M(帕姆.3mf)` → `帕姆.3mf`。
///
/// 从右往左数括号深度，闭到 0 的那一对就是它 —— 这样 `…(帕姆(1).3mf)` 不会只取到 `1`，
/// 而预设名自己带括号（`…(基本)(帕姆.3mf)`）时取的也仍是最后一段。
fn last_paren_segment(s: &str) -> Option<&str> {
    let t = s.trim_end();
    if !t.ends_with(')') {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in t.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&t[i + 1..t.len() - 1]);
                }
            }
            _ => {}
        }
    }
    None
}

/// `slice_info.config` 里的对象名。多对象拼 `名字A+名字B`：盘上确实有这些件，
/// 只报第一个等于替这盘撒谎。去重（同一件重复出现不算两件）、跳过没名字的。
fn object_names(xml: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<object") {
        let after = &rest[at + "<object".len()..];
        let Some(end) = after.find('>') else { break };
        if let Some(raw) = attr_value(&after[..end], "name") {
            let name = xml_unescape(&raw);
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
        rest = &after[end..];
    }
    out
}

/// 标签里取属性值：`<object identify_id="237" name="帕姆" …>` → `帕姆`。
/// 按 `名字="` 找，且**要求前一字符是空白** —— 免得 `file_name="x"` 里的 `name="` 命中。
fn attr_value(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let mut rest = tag;
    while let Some(at) = rest.find(&pat) {
        let ok = rest[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        if ok {
            let after = &rest[at + pat.len()..];
            return after.find('"').map(|end| after[..end].to_string());
        }
        rest = &rest[at + pat.len()..];
    }
    None
}

/// XML 的五个预定义实体。模型名里真会出现 `&`（`Tom &amp; Jerry.3mf`），
/// 不解的话名字会带着 `&amp;` 显示。`&amp;` 放最后替换（先解它会把 `&amp;lt;` 变成 `<`）。
pub(crate) fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// zip 里某个条目的文本。
///
/// 按**本地文件头**线性扫：`PK\x03\x04` 起 30 字节定长头（字段偏移见 PKWARE APPNOTE
/// 4.3.7：压缩方式 8、压缩后长度 18、名字长 26、扩展长 28），名字比中即取数据区。
/// `store` 直读，`deflate` 交给 flate2 —— deflate 流自带结束标记，所以压缩后长度
/// 是不是写在头里（有些写法放数据描述符）都不影响。
pub(crate) fn zip_entry_text(zip: &[u8], want: &str) -> Option<String> {
    const SIG: &[u8; 4] = b"PK\x03\x04";
    let mut at = 0usize;
    while let Some(rel) = find_bytes(zip.get(at..)?, SIG) {
        let p = at + rel;
        at = p + SIG.len();
        let head = zip.get(p..p + 30)?;
        let method = u16::from_le_bytes([head[8], head[9]]);
        let comp = u32::from_le_bytes([head[18], head[19], head[20], head[21]]) as usize;
        let name_len = u16::from_le_bytes([head[26], head[27]]) as usize;
        let extra_len = u16::from_le_bytes([head[28], head[29]]) as usize;
        if zip.get(p + 30..p + 30 + name_len)? != want.as_bytes() {
            continue;
        }
        let data = zip.get(p + 30 + name_len + extra_len..)?;
        return match method {
            // store：长度没写在头里（0 / 未知）就判不出来 ⇒ 进下一级，不瞎读到文件尾
            0 => {
                if comp == 0 || comp > data.len() {
                    return None;
                }
                Some(String::from_utf8_lossy(&data[..comp]).into_owned())
            }
            8 => inflate(data),
            _ => None,
        };
    }
    None
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// deflate 解压成文本（不是 UTF-8 就 `None` —— 解出来的是 XML 文本，本来就该是 UTF-8）。
fn inflate(data: &[u8]) -> Option<String> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(data)
        .read_to_end(&mut out)
        .ok()?;
    Some(String::from_utf8_lossy(&out).into_owned())
}

#[cfg(test)]
mod tests {
    // 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
    // （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use std::io::Write;

    /// 当前目录形态：`<root>\<时刻>#<id>#<rand>\Metadata\<gcode>`（真机长这样）。
    fn layout() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("16_49_59#63288#211");
        let meta = root.join("Metadata");
        std::fs::create_dir_all(&meta).unwrap();
        let gcode = meta.join(".63288.2.gcode");
        std::fs::write(&gcode, "G1 X0 Y0\n").unwrap();
        (d, root, gcode)
    }

    #[test]
    fn origin_txt_is_the_first_source() {
        let (_d, root, gcode) = layout();
        // 真机形态：一行文本、CRLF、无 BOM、Windows 反斜杠、中文名。
        std::fs::write(
            root.join("origin.txt"),
            "C:\\Users\\WZY\\Downloads\\帕姆.3mf\r\n",
        )
        .unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆"));

        // 空格与多个点也照旧（去的是**最后**那个扩展名）。
        std::fs::write(root.join("origin.txt"), "D:\\模型\\帕姆 v2.1.3mf\n").unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆 v2.1"));
    }

    #[test]
    fn origin_txt_wins_over_the_other_two() {
        let (_d, root, gcode) = layout();
        std::fs::write(root.join("origin.txt"), "C:\\x\\帕姆.3mf\n").unwrap();
        let other = slice_info("别的名字");
        std::fs::write(
            gcode.with_extension("3mf"),
            zip_with(&[("Metadata/slice_info.config", &other, true)]),
        )
        .unwrap();
        std::fs::write(
            root.join("_temp_1.config"),
            r#"{"filament_settings_id":["P @BBL A1M(第三个名字.3mf)"]}"#,
        )
        .unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆"));
    }

    /// 次选走真 zip：目标条目用 deflate（真机 `.63288.2.3mf` 就是压的），
    /// 旁边那个用 store（真机里那些 png 是 store），两种都要能读。
    #[test]
    fn zip_slice_info_is_the_second_source() {
        let (_d, _root, gcode) = layout();
        let xml = slice_info("帕姆");
        let zip = zip_with(&[
            ("Metadata/plate_1.png", b"\x89PNG\r\n\x1a\n", false),
            ("Metadata/slice_info.config", &xml, true),
        ]);
        std::fs::write(gcode.with_extension("3mf"), &zip).unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆"));

        // 同名 zip 存在但读不出东西（包坏了 / 条目不在里头）⇒ 不当成"读出个空名字"，
        // 直接进下一级、最后给 None。
        std::fs::write(gcode.with_extension("3mf"), b"not a zip").unwrap();
        assert_eq!(model_name(&gcode), None);
    }

    #[test]
    fn temp_config_is_the_third_source() {
        let (_d, root, gcode) = layout();
        // 真机原文（键序一致）：filament_settings_id[0] 与 name 都是 `…(帕姆.3mf)`。
        std::fs::write(
            root.join("_temp_1.config"),
            r#"{"filament_settings_id":["Bambu PLA Basic @BBL A1M(帕姆.3mf)"],"from":"project","name":"Bambu PLA Basic @BBL A1M(帕姆.3mf)"}"#,
        )
        .unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆"));

        // 预设名自己带括号、模型名里也有括号：取的是**末尾那个配平段**。
        std::fs::write(
            root.join("_temp_1.config"),
            r#"{"filament_settings_id":["PLA (基本) @BBL A1M(帕姆(1).3mf)"]}"#,
        )
        .unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("帕姆(1)"));

        // 只有 name 那一格有（filament_settings_id 不在）也认。
        std::fs::write(
            root.join("_temp_1.config"),
            r#"{"name":"X @BBL(老张.3mf)"}"#,
        )
        .unwrap();
        assert_eq!(model_name(&gcode).as_deref(), Some("老张"));
    }

    /// 三级都没有 ⇒ `None`（页面上只显示 G-code 名）。G-code 被搬到别处就是这个形态。
    #[test]
    fn nothing_beside_the_gcode_is_none() {
        let (_d, _root, gcode) = layout();
        assert_eq!(model_name(&gcode), None);

        // 空白 / 空路径也算没有（不产出空名字）。
        std::fs::write(
            gcode.parent().unwrap().parent().unwrap().join("origin.txt"),
            "\r\n",
        )
        .unwrap();
        assert_eq!(model_name(&gcode), None);

        let elsewhere = tempfile::tempdir().unwrap();
        let moved = elsewhere.path().join(".63288.2.gcode");
        std::fs::write(&moved, "G1\n").unwrap();
        assert_eq!(model_name(&moved), None);
    }

    #[test]
    fn object_names_join_dedup_and_unescape() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n  <plate>\n    \
                   <object identify_id=\"1\" name=\"帽子\" skipped=\"false\" />\n    \
                   <object identify_id=\"2\" name=\"Tom &amp; Jerry\" />\n    \
                   <object identify_id=\"3\" name=\"帽子\" />\n    \
                   <object identify_id=\"4\" skipped=\"true\" />\n  </plate>\n</config>\n";
        assert_eq!(object_names(xml), vec!["帽子", "Tom & Jerry"]);

        // 属性顺序反过来（name 在前）也认；`file_name="x"` 不算 `name`。
        assert_eq!(
            object_names("<object name=\"甲\" identify_id=\"9\" />"),
            vec!["甲"]
        );
        assert_eq!(
            object_names("<object file_name=\"乙\" />"),
            Vec::<String>::new()
        );
        assert_eq!(object_names(""), Vec::<String>::new());
    }

    #[test]
    fn stem_of_takes_the_last_segment_of_both_separators() {
        assert_eq!(stem_of("C:\\Users\\WZY\\Downloads\\帕姆.3mf"), "帕姆");
        assert_eq!(stem_of("/home/wzy/帕姆.3mf"), "帕姆");
        assert_eq!(stem_of("帕姆.3mf"), "帕姆");
        assert_eq!(stem_of("没有扩展名"), "没有扩展名");
        assert_eq!(stem_of(".3mf"), ".3mf");
        assert_eq!(stem_of(""), "");
    }

    #[test]
    fn last_paren_segment_takes_the_trailing_balanced_group() {
        assert_eq!(
            last_paren_segment("Bambu PLA Basic @BBL A1M(帕姆.3mf)"),
            Some("帕姆.3mf")
        );
        assert_eq!(last_paren_segment("PLA(基本)(帕姆.3mf)"), Some("帕姆.3mf"));
        assert_eq!(last_paren_segment("PLA(帕姆(1).3mf)"), Some("帕姆(1).3mf"));
        assert_eq!(last_paren_segment("没有括号"), None);
        assert_eq!(last_paren_segment("半个(括号"), None);
    }

    /// `slice_info.config` 的真机形态（2026-10-11 从 `.63288.2.3mf` 里读出来的那份，
    /// 只留被本模块消费的那几行）。
    fn slice_info(name: &str) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n  <header>\n    \
             <header_item key=\"X-BBL-Client-Version\" value=\"02.08.02.61\"/>\n  </header>\n  \
             <plate>\n    <metadata key=\"index\" value=\"1\"/>\n    \
             <object identify_id=\"237\" name=\"{name}\" skipped=\"false\" />\n    \
             <filament id=\"1\" type=\"PETG\" used_m=\"5.83\" />\n  </plate>\n</config>\n"
        )
        .into_bytes()
    }

    // ---------- 造一个真 zip 的测试夹具 ----------
    //
    // 判据要过的是**真形态**：zip 得是切片器写的那种（本地头 + 中央目录 + EOCD，
    // CRC32 用 flate2 的现成件算），不是"我们读法的一厢情愿"。

    fn push_u16(out: &mut Vec<u8>, v: u16) {
        out.extend_from_slice(&v.to_le_bytes());
    }

    fn push_u32(out: &mut Vec<u8>, v: u32) {
        out.extend_from_slice(&v.to_le_bytes());
    }

    /// `(条目名, 内容, 是否 deflate)`。
    fn zip_with(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        for (name, data, deflate) in entries {
            let (method, body): (u16, Vec<u8>) = if *deflate {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(data).unwrap();
                (8, enc.finish().unwrap())
            } else {
                (0, data.to_vec())
            };
            let crc = {
                let mut c = flate2::Crc::new();
                c.update(data);
                c.sum()
            };
            let offset = out.len() as u32;

            out.extend_from_slice(b"PK\x03\x04");
            push_u16(&mut out, 20);
            push_u16(&mut out, 0);
            push_u16(&mut out, method);
            push_u16(&mut out, 0);
            push_u16(&mut out, 0);
            push_u32(&mut out, crc);
            push_u32(&mut out, body.len() as u32);
            push_u32(&mut out, data.len() as u32);
            push_u16(&mut out, name.len() as u16);
            push_u16(&mut out, 0);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&body);

            central.extend_from_slice(b"PK\x01\x02");
            push_u16(&mut central, 20);
            push_u16(&mut central, 20);
            push_u16(&mut central, 0);
            push_u16(&mut central, method);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, crc);
            push_u32(&mut central, body.len() as u32);
            push_u32(&mut central, data.len() as u32);
            push_u16(&mut central, name.len() as u16);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, 0);
            push_u32(&mut central, offset);
            central.extend_from_slice(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        let count = entries.len() as u16;
        out.extend_from_slice(&central);

        out.extend_from_slice(b"PK\x05\x06");
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, count);
        push_u16(&mut out, count);
        push_u32(&mut out, cd_size);
        push_u32(&mut out, cd_offset);
        push_u16(&mut out, 0);
        out
    }
}
