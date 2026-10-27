//! 校准件身份探测 —— 「这一盘切的是不是校准件、是哪一种」。
//!
//! # 身份只有一处真值
//!
//! **原始 3MF 的 `3D/3dmodel.model` 里的 `<metadata name="MKPSE:Calibration">`。**
//!
//! 早先这件事靠 G-code 里的一行注释（`custom_gcode_per_layer.xml` 灌进第 1 层的
//! `;MKPCalibration …`），2026-10-18 整条删除，理由是它会给出错答案：
//!
//! - 那行标记住在 `Metadata/custom_gcode_per_layer.xml`，**某些版本的 Bambu Studio
//!   存一次就把这个条目丢掉**（真机核对：原件 `BambuStudio-02.05.00.66` 有、
//!   被 `02.08.02.60` 存过一遍的那份没有，条目数 17 → 16）；
//! - `<metadata name="MKPSE:Calibration">` 经真机验证 **Bambu Studio 另存后完好保留**
//!   （BBS 还把它按字母序排进 metadata 组）；
//! - 但它**不会**自己进切片产物（`plate_1.gcode` 里出现 0 次，`CONFIG_BLOCK` 也没有），
//!   所以它只能由本模块在钩子里读，不能指望 G-code 里出现。
//!
//! 于是：**G-code 里即便还留着旧注释，也一律忽略** —— 本模块的签名里根本没有 G-code。
//! 没有身份 ⇒ `Mode::None`，**不许按尺寸猜**（尺寸只决定"已识别的校准件用哪一档"，
//! 见 `calib_models::find_profile`）。
//!
//! # 怎么找到原始 3MF
//!
//! 切片器的临时目录里有 `origin.txt`，第一行 = **原始 3MF 的绝对路径**（与 `model_name`
//! 同一个来源、同一时机）。只认 `.3mf`：导入 STL 时那里写的是 `.stl`，STL 没有 metadata
//! 这东西 —— 那不是"校准件没标记"，是"这个格式承载不了身份"，所以结论写进 `note`，
//! 不伪装成"查过了、没有"。
//!
//! # 时机是硬要求
//!
//! 这些旁证全在切片器的临时目录里，G-code 一被搬走就够不着。所以只能在钩子刚拿到路径时
//! 读一次，把结果写进执行记录（`_meta.json` 的 `detail.calibration`）；以后按记录复盘，
//! 不重新读盘 —— 用户把原始 3MF 删了也解释得清当时发生了什么。

use std::path::Path;

use super::model_name::{temp_root, xml_unescape, zip_entry_text};

/// 3MF 里那条身份 metadata 的键。**全仓唯一**：写模型、读模型、文档都只认这一个串。
pub const CALIBRATION_METADATA_KEY: &str = "MKPSE:Calibration";

/// 3MF 里装 metadata 的那个条目（OPC 规范内的固定名）。
const MODEL_ENTRY: &str = "3D/3dmodel.model";

/// 原始模型文件的情况 —— 供执行记录用，"当时到底看的是哪个文件、还在不在"。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOrigin {
    /// `origin.txt` 的原值（可能是 `.stl`，也可能是空）。
    pub origin_path: Option<String>,
    /// 仅当扩展名是 `.3mf` 时给出 —— 只有它才可能是身份来源。
    pub model_3mf: Option<String>,
    /// 该路径**当时**是否真实存在。
    pub exists: bool,
}

/// 身份探测的结论。
///
/// **只有两档**：是从 3MF 读到了身份，还是没读到。早先那张
/// `Agree / Only3mf / OnlyGcode / Conflict` 的表跟着 G-code 载体一起删了 ——
/// 只剩一个来源的时候没有"两处不一致"可言。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IdentityVerdict {
    /// 3MF 里读到了 `MKPSE:Calibration`（值能不能解析另说，见 [`IdentityProbe::mode_id`]）。
    From3mf,
    /// 没读到 ⇒ 普通模型、或读不到的原因在 `note` 里。**不猜**。
    #[default]
    Absent,
}

impl IdentityVerdict {
    /// 存档里那一格用的稳定串（前端按它选文案，不认 Rust 的枚举名）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::From3mf => "from_3mf",
            Self::Absent => "absent",
        }
    }
}

/// 一次探测的全部原料与结论。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityProbe {
    pub origin: ModelOrigin,
    /// 3MF `MKPSE:Calibration` 的**原值**（原文里那段，未归一）。
    ///
    /// 存原值不存归一后的 id：排查时要看的是"它当时到底写了什么"，
    /// 比如把 `ZOffset ` 写成 `Zoffset` 或写错字，都在这里照实留着。
    pub identity_3mf: Option<String>,
    pub verdict: IdentityVerdict,
    /// 读不到的原因（人可读、一条）。不是错误，只是解释 —— 例如
    /// "origin.txt 指向 .stl（STL 承载不了身份）"、"3MF 里没有这条 metadata"。
    pub note: Option<String>,
}

/// 探测一次。**全程只读、不 panic**。
///
/// `origin.txt` 读不到、路径不存在、不是合法 zip、条目缺失、没有那条 metadata ——
/// 一律降级成 [`IdentityVerdict::Absent`] 并留一条 `note`，不静默吞掉原因。
pub fn probe(gcode_path: &Path) -> IdentityProbe {
    let mut p = IdentityProbe::default();
    let mut note: Option<String> = None;

    // ---- ① 原始模型路径 ----
    if let Some(root) = temp_root(gcode_path) {
        match std::fs::read(root.join("origin.txt")) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let first = text.lines().next().unwrap_or("").trim().to_string();
                if !first.is_empty() {
                    p.origin.origin_path = Some(first.clone());
                    if is_3mf(&first) {
                        p.origin.model_3mf = Some(first.clone());
                        p.origin.exists = Path::new(&first).exists();
                    } else {
                        note = Some(format!(
                            "origin.txt 指向的不是 .3mf（{first}）—— 该格式承载不了身份"
                        ));
                    }
                }
            }
            Err(_) => {
                note = Some("切片器临时目录里没有 origin.txt（G-code 可能已被搬走）".to_string());
            }
        }
    } else {
        note = Some("G-code 不在切片器临时目录结构里（没有上级目录可比对）".to_string());
    }

    // ---- ② 3MF 身份（唯一来源）----
    if let Some(path) = p.origin.model_3mf.as_deref() {
        if !p.origin.exists {
            note = Some(format!("原始 3MF 已不在原路径（{path}）"));
        } else {
            match std::fs::read(path) {
                Ok(bytes) => match zip_entry_text(&bytes, MODEL_ENTRY) {
                    Some(xml) => match metadata_value(&xml, CALIBRATION_METADATA_KEY) {
                        Some(v) => {
                            p.identity_3mf = Some(v);
                            p.verdict = IdentityVerdict::From3mf;
                        }
                        None => {
                            note = Some(format!(
                                "3MF 里没有 {CALIBRATION_METADATA_KEY} 这条 metadata"
                            ))
                        }
                    },
                    None => note = Some(format!("3MF 里读不出 {MODEL_ENTRY}（包可能坏了）")),
                },
                Err(e) => note = Some(format!("原始 3MF 读不出来（{e}）")),
            }
        }
    }

    p.note = note;
    p
}

fn is_3mf(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".3mf")
}

/// 从 `3D/3dmodel.model` 里取 `<metadata name="KEY">值</metadata>` 的值。
///
/// 只认**成对标签**那种（值在元素文本里）；自闭合形态（`<metadata name="…" value="…"/>`）
/// 也认 —— Bambu Studio 自己写 `DesignerUserId` 用前者，但别的工具可能用后者。
fn metadata_value(xml: &str, key: &str) -> Option<String> {
    let mut rest = xml;
    let needle = format!("name=\"{key}\"");
    let mut offset = 0usize;
    while let Some(at) = rest.find("<metadata") {
        let start = offset + at;
        let after = &xml[start..];
        let end = after.find('>')?;
        let tag = &after[..end];
        let self_closing = tag.ends_with('/');
        if let Some(pos) = tag.find(&needle) {
            // 值在元素文本里
            if !self_closing {
                let text = &after[end + 1..];
                if let Some(close) = text.find("</metadata>") {
                    let v = text[..close].trim();
                    return (!v.is_empty()).then(|| xml_unescape(v));
                }
            } else {
                // `<metadata name="KEY" value="V"/>` 形态
                let attr = &tag[pos..];
                if let Some(vat) = attr.find("value=\"") {
                    let vrest = &attr[vat + 7..];
                    if let Some(vend) = vrest.find('"') {
                        let v = vrest[..vend].trim();
                        return (!v.is_empty()).then(|| xml_unescape(v));
                    }
                }
            }
        }
        offset = start + end + 1;
        rest = &xml[offset..];
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use std::io::Write;

    fn layout() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        tempfile::TempDir,
    ) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("21_34_12#95870#3");
        let meta = root.join("Metadata");
        std::fs::create_dir_all(&meta).unwrap();
        let gcode = meta.join(".5926.6.gcode");
        std::fs::write(&gcode, "G1 X0 Y0\n").unwrap();
        let model_dir = tempfile::tempdir().unwrap();
        (d, root, gcode, model_dir)
    }

    fn three_mf(value: &str) -> Vec<u8> {
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\">\n  \
             <metadata name=\"BambuStudio:3mfVersion\">1</metadata>\n  \
             <metadata name=\"{CALIBRATION_METADATA_KEY}\">{value}</metadata>\n  \
             <resources/>\n</model>\n"
        );
        zip_with(&[(MODEL_ENTRY, xml.as_bytes(), true)])
    }

    #[test]
    fn reads_identity_from_the_original_3mf() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("ZOffset_Calibration.3mf");
        std::fs::write(&m, three_mf("ZOffset")).unwrap();
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();

        let p = probe(&gcode);
        assert_eq!(p.identity_3mf.as_deref(), Some("ZOffset"));
        assert!(p.origin.exists);
        assert_eq!(p.verdict, IdentityVerdict::From3mf);
    }

    /// **G-code 里写着别的身份也不影响**（2026-10-18 之后的硬口径）：
    /// 旧注释一律忽略，身份只看 3MF。这条判据防的是"有人把 G-code 扫描加回来"。
    #[test]
    fn a_stale_gcode_marker_cannot_change_the_identity() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("ZOffset_Calibration.3mf");
        std::fs::write(&m, three_mf("ZOffset")).unwrap();
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();

        /* 正文里塞满各种旧标记 —— 探测根本不看正文，签名里也没有它 */
        let body = ";MKPCalibration Precise\n; CUSTOM_GCODE\n; Rough Calibration\n";
        std::fs::write(&gcode, body).unwrap();

        let p = probe(&gcode);
        assert_eq!(p.identity_3mf.as_deref(), Some("ZOffset"));
    }

    /// 3MF 有身份、G-code 是干净切片（另存后常见形态）⇒ 正常读出。
    #[test]
    fn resaved_model_without_any_gcode_marker_still_works() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("ZOffset_Calibration.3mf");
        std::fs::write(&m, three_mf("ZOffset")).unwrap();
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();

        std::fs::write(
            &gcode,
            "; layer num/total_layer_count: 1/2\nM106 S0\nG1 X10 Y10\n",
        )
        .unwrap();
        let p = probe(&gcode);
        assert_eq!(p.verdict, IdentityVerdict::From3mf);
    }

    /// 没有 3MF ⇒ 就是 `Absent`。**不许**因为 G-code 里有旧标记就认成校准件。
    #[test]
    fn no_3mf_means_absent_even_if_gcode_has_a_marker() {
        let (_d, _root, gcode, _md) = layout();
        std::fs::write(&gcode, ";MKPCalibration ZOffset\n").unwrap();
        let p = probe(&gcode);
        assert_eq!(p.verdict, IdentityVerdict::Absent);
        assert!(p.identity_3mf.is_none());
    }

    /// 值写错（不是那四个身份之一）⇒ 原值照留、模式认不出。
    #[test]
    fn an_unknown_value_is_kept_verbatim_but_yields_no_mode() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("weird.3mf");
        std::fs::write(&m, three_mf("随便写的")).unwrap();
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();

        let p = probe(&gcode);
        assert_eq!(p.verdict, IdentityVerdict::From3mf);
        assert_eq!(p.identity_3mf.as_deref(), Some("随便写的"));
    }

    /// `origin.txt` 指向 STL：不是身份来源，且要说清原因（不是"没标记"）。
    #[test]
    fn stl_origin_is_explained_not_silent() {
        let (_d, root, gcode, _md) = layout();
        std::fs::write(root.join("origin.txt"), "/Users/x/model.stl").unwrap();
        let p = probe(&gcode);
        assert!(p.origin.model_3mf.is_none());
        assert!(p.note.as_deref().unwrap_or("").contains(".3mf"));
        assert_eq!(p.verdict, IdentityVerdict::Absent);
    }

    /// 原始 3MF 被移走：记下"不在原路径"，而不是"没有身份"。
    #[test]
    fn moved_3mf_is_recorded_as_missing() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("gone.3mf");
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();
        let p = probe(&gcode);
        assert!(p.origin.model_3mf.is_some());
        assert!(!p.origin.exists);
        assert!(p.note.as_deref().unwrap_or("").contains("已不在原路径"));
    }

    /// 3MF 里没有这条 metadata（正是今天云端那份的形态）⇒ 不伪造成"没身份"以外的结论。
    #[test]
    fn three_mf_without_metadata_degrades_to_absent() {
        let (_d, root, gcode, md) = layout();
        let m = md.path().join("plain.3mf");
        let xml = "<model unit=\"millimeter\"><metadata name=\"Title\">x</metadata></model>";
        std::fs::write(&m, zip_with(&[(MODEL_ENTRY, xml.as_bytes(), true)])).unwrap();
        std::fs::write(root.join("origin.txt"), m.to_string_lossy().to_string()).unwrap();

        let p = probe(&gcode);
        assert!(p.identity_3mf.is_none());
        assert!(
            p.note
                .as_deref()
                .unwrap_or("")
                .contains(CALIBRATION_METADATA_KEY)
        );
        assert_eq!(p.verdict, IdentityVerdict::Absent);
    }

    #[test]
    fn metadata_value_handles_both_shapes() {
        let a = "<model><metadata name=\"MKPSE:Calibration\">Precise</metadata></model>";
        assert_eq!(
            metadata_value(a, CALIBRATION_METADATA_KEY).as_deref(),
            Some("Precise")
        );

        let b = "<model><metadata name=\"MKPSE:Calibration\" value=\"Rough\"/></model>";
        assert_eq!(
            metadata_value(b, CALIBRATION_METADATA_KEY).as_deref(),
            Some("Rough")
        );

        assert_eq!(metadata_value("<model/>", CALIBRATION_METADATA_KEY), None);
    }

    // ---------- 造真 zip（与 model_name.rs 同一套夹具口径） ----------

    fn push_u16(out: &mut Vec<u8>, v: u16) {
        out.extend_from_slice(&v.to_le_bytes());
    }

    fn push_u32(out: &mut Vec<u8>, v: u32) {
        out.extend_from_slice(&v.to_le_bytes());
    }

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
