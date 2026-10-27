//! **校准素材的 3MF 身份** —— 读、校验、写回、移除，**全仓唯一实现**。
//!
//! # 为什么要有这一层
//!
//! 校准件身份只有一处真值：`3D/3dmodel.model` 里的
//! `<metadata name="MKPSE:Calibration">ZOffset</metadata>`（2026-10-18 定案，
//! G-code 标记那条链已整条删除）。围着这条真值有三张嘴要说话：
//!
//! | 谁 | 要什么 |
//! | --- | --- |
//! | 后处理钩子 | 只读（`gcode::model_identity::probe`） |
//! | `npm run check:calibration-assets` | 读 + 校验，**不许改文件** |
//! | 工作台资产面 | 读 + 校验 + **写**（补标 / 改写 / 移除） |
//!
//! 三张嘴各写一套解析，半年后必然对不上（"脚本说合格、工作台说没标记"）。所以读、校验、
//! 写三件事都收在本模块：检查脚本与工作台**都调这里**，谁都不许自己拆 zip。
//!
//! # 写回为什么是"最小重打包"
//!
//! 3MF 是 zip，改一个条目必须重打整包。本模块**只碰 `3D/3dmodel.model` 一个条目**：
//! 其余条目的压缩方式、时间戳、字节全部原样搬过去（deflate 的原始流直接拷贝，**不重新
//! 压缩** —— 重新压一遍那些 PNG 只会让文件变大、还会改掉 CRC）。
//!
//! 依赖只有 `flate2`（本 crate 已有的），**不引 zip 包**：理由与 `gcode::model_name`
//! 那段一致 —— 版本号只住 `[workspace.dependencies]` 一处、构建要能离线跑。
//! 已知边界：不认 zip64 与加密条目（真机 3MF 都是普通 zip；遇到就报错，不猜）。
//!
//! # 写盘纪律
//!
//! 先写同目录临时文件，再 rename 覆盖 —— **原文件在成功之前一个字节都不动**。
//! 中途失败（磁盘满 / 文件被占）留下的只是一个临时文件，用户的 3MF 还是能打开的。

use std::path::{Path, PathBuf};

use crate::gcode::model_identity::CALIBRATION_METADATA_KEY;
use crate::postproc::calibration::{Mode, parse_mode_id};

/// 装 metadata 的那个条目（OPC 规范内的固定名）—— 与 `gcode::model_identity` 同一个串。
const MODEL_ENTRY: &str = "3D/3dmodel.model";

/// 一份校准素材在磁盘上的样子。
///
/// `identity` 存**原值**（照实，不归一）：排查时要看的就是"文件里到底写了什么"，
/// 比如大小写不对、写了旧口径 `XY`、或者写了个错字。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationAsset {
    /// 读的那个文件（原样回显，便于报错时指认）。
    pub path: String,
    /// `MKPSE:Calibration` 的原值；没有这条 metadata = `None`。
    pub identity: Option<String>,
    /// 读不出来 / 结构异常的原因。**不是错误**，是"为什么没有身份"的解释 ——
    /// 与"文件在、但确实没打标记"是两件事，不许混成一句。
    pub problem: Option<String>,
}

impl CalibrationAsset {
    /// 原值能不能解析成合法身份（四个之一）。
    ///
    /// `Some(raw)` 但这里 `None` ⇒ **值非法**（写错了 / 旧口径 `XY`）。调用方要能分清
    /// "没标记" 与 "标记写错了"，因为处理方式不同（补 vs 改）。
    pub fn mode(&self) -> Option<Mode> {
        self.identity.as_deref().and_then(parse_mode_id)
    }

    /// 身份合法（有值且能解析）。
    pub fn is_valid(&self) -> bool {
        self.mode().is_some()
    }
}

/// 读一份 3MF 的校准身份。**只读，不改文件**。
///
/// 任何读不出来的情况都落到 [`CalibrationAsset::problem`]，不 panic：
/// 文件不在、不是合法 zip、没有 `3dmodel.model` 条目、zip64、加密条目……
pub fn read_identity(path: &Path) -> CalibrationAsset {
    let mut asset = CalibrationAsset {
        path: path.display().to_string(),
        identity: None,
        problem: None,
    };

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            asset.problem = Some(format!("读不出来（{e}）"));
            return asset;
        }
    };

    let xml = match read_model_entry(&bytes) {
        Ok(xml) => xml,
        Err(e) => {
            asset.problem = Some(e);
            return asset;
        }
    };

    match metadata_value(&xml, CALIBRATION_METADATA_KEY) {
        Some(v) => asset.identity = Some(v),
        None => {
            asset.problem = Some(format!(
                "3MF 里没有 {CALIBRATION_METADATA_KEY} 这条 metadata"
            ))
        }
    }
    asset
}

/// 校验一个身份值是不是那四个之一（唯一解析出口是 `parse_mode_id`）。
///
/// 给检查脚本与工作台共用：**写之前也走它**，非法值一律拒绝、不落盘。
pub fn validate_identity(raw: &str) -> Option<Mode> {
    parse_mode_id(raw)
}

/// 写入 / 改写身份。`mode` 必须是四个身份之一（由类型保证）。
pub fn write_identity(path: &Path, mode: Mode) -> Result<(), String> {
    let raw = crate::postproc::calibration::mode_to_calibration_string(mode);
    if raw.is_empty() {
        return Err("写入的身份不能是 `None`（要移除就用 remove_identity）".to_string());
    }
    rewrite_model_entry(path, |xml| set_metadata(xml, CALIBRATION_METADATA_KEY, raw))
}

/// 移除身份（把那条 metadata 整段删掉）。
///
/// 没有这条 metadata 时**不算失败**（结果一样：文件里没有身份）—— 幂等。
pub fn remove_identity(path: &Path) -> Result<(), String> {
    rewrite_model_entry(path, |xml| {
        Ok(remove_metadata(xml, CALIBRATION_METADATA_KEY))
    })
}

// ---------------------------------------------------------------- 写回

/// 重打包：只替换 `3D/3dmodel.model` 的内容，其余条目字节直通。
///
/// `edit` 拿原 XML 返回新 XML；它返回 `Err` 就**一个字节都不写**。
fn rewrite_model_entry(
    path: &Path,
    edit: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读不出来（{e}）"))?;
    let entries = parse_zip(&bytes)?;

    let target = entries
        .iter()
        .find(|e| e.name == MODEL_ENTRY)
        .ok_or_else(|| format!("3MF 里没有 {MODEL_ENTRY}（包不完整）"))?;

    let old_xml = decode_entry(&bytes, target)?;
    let new_xml = edit(&old_xml)?;
    /* 内容没变就别白白重写一遍（也免得改掉时间戳） */
    if new_xml == old_xml {
        return Ok(());
    }

    let rebuilt = repack(&bytes, &entries, MODEL_ENTRY, new_xml.as_bytes())?;
    write_atomic(path, &rebuilt)
}

/// 原子写：同目录临时文件 → `sync_all` → rename → fsync 父目录。
///
/// 与 `src-tauri/src/fsx/atomic.rs` 同一个套路（那一个本 crate 够不着 —— 它在承载层）。
/// 为什么不能用 `std::fs::write`：它**先把目标截断成 0 字节再写**，中途崩了用户的
/// 3MF 就没了（不是回到旧版本，是空文件）。临时文件必须与目标**同目录**：
/// 跨分区 rename 会退化成 copy + unlink，原子性就没了。
///
/// `clippy.toml` 把 `fs::write` / `File::create` 列进 disallowed-methods，
/// 全仓的洞只有 `fsx/atomic.rs` 那一处；内核这里照同一条规矩自己实现一遍，
/// 并显式开一个窄洞（只在本函数内）。
#[allow(clippy::disallowed_methods)]
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "asset.3mf".to_string());
    let tmp = dir.join(format!(".{name}.mkpse-tmp"));

    let mut file = std::fs::File::create(&tmp).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("建临时文件失败（{e}）")
    })?;
    let write = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(e) = write {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("写临时文件失败（{e}）"));
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("替换原文件失败（{e}）"));
    }
    /* 父目录也 fsync：rename 之后"这个名字指向哪个 inode"可能还在缓存里。
    打不开（罕见）不算失败 —— 数据已经在盘上了。 */
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

// ---------------------------------------------------------------- zip 读

/// 中央目录里的一条例目。写回时按这组信息**原样**重建，所以时间戳、压缩方式都在这里。
#[derive(Debug, Clone)]
struct Entry {
    name: String,
    method: u16,
    crc: u32,
    compressed: usize,
    uncompressed: usize,
    /// 数据在文件里的起止（本地文件头之后那段压缩数据）
    data_start: usize,
    data_end: usize,
    /// 本地文件头原文（含文件名与 extra）—— 原样搬回去
    local_header: Vec<u8>,
    /// 中央目录里这一条的原文字节 —— 只改 `crc` / `sizes` 那几格，其余（时间戳、
    /// 属性、注释）全留着
    central: Vec<u8>,
    dos_time: u16,
    dos_date: u16,
}

/// 按中央目录解析一遍。比 `model_name::zip_entry_text` 那条"扫本地文件头"的路更严：
/// 写回必须先知道每一条的完整信息。
fn parse_zip(bytes: &[u8]) -> Result<Vec<Entry>, String> {
    let eocd =
        find_eocd(bytes).ok_or_else(|| "不是有效的 zip（找不到中央目录结尾）".to_string())?;
    let count = read_u16(bytes, eocd + 10)? as usize;
    let mut at = read_u32(bytes, eocd + 16)? as usize;
    let mut out = Vec::with_capacity(count);

    for _ in 0..count {
        if bytes.get(at..at + 4) != Some(b"PK\x01\x02") {
            return Err("中央目录结构异常（签名不对）".to_string());
        }
        let flags = read_u16(bytes, at + 8)?;
        let method = read_u16(bytes, at + 10)?;
        let dos_time = read_u16(bytes, at + 12)?;
        let dos_date = read_u16(bytes, at + 14)?;
        let crc = read_u32(bytes, at + 16)?;
        let compressed = read_u32(bytes, at + 20)? as usize;
        let uncompressed = read_u32(bytes, at + 24)? as usize;
        let name_len = read_u16(bytes, at + 28)? as usize;
        let extra_len = read_u16(bytes, at + 30)? as usize;
        let comment_len = read_u16(bytes, at + 32)? as usize;
        let local_at = read_u32(bytes, at + 42)? as usize;

        if flags & 0x0001 != 0 {
            return Err("3MF 里有加密条目（不支持）".to_string());
        }
        if compressed == 0xFFFF_FFFF || uncompressed == 0xFFFF_FFFF || local_at == 0xFFFF_FFFF {
            return Err("3MF 是 zip64（不支持）".to_string());
        }

        let name_bytes = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| "中央目录越界".to_string())?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        let central_end = at + 46 + name_len + extra_len + comment_len;
        let central = bytes
            .get(at..central_end)
            .ok_or_else(|| "中央目录越界".to_string())?
            .to_vec();

        // 本地文件头的长度可能和中央目录记的不一样（extra 段常不同）⇒ 现读一遍
        if bytes.get(local_at..local_at + 4) != Some(b"PK\x03\x04") {
            return Err(format!("条目「{name}」的本地文件头签名不对"));
        }
        let l_name_len = read_u16(bytes, local_at + 26)? as usize;
        let l_extra_len = read_u16(bytes, local_at + 28)? as usize;
        let header_end = local_at + 30 + l_name_len + l_extra_len;
        let data_start = header_end;
        let data_end = data_start
            .checked_add(compressed)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| format!("条目「{name}」的数据段越界"))?;
        let local_header = bytes
            .get(local_at..header_end)
            .ok_or_else(|| format!("条目「{name}」的本地文件头越界"))?
            .to_vec();

        out.push(Entry {
            name,
            method,
            crc,
            compressed,
            uncompressed,
            data_start,
            data_end,
            local_header,
            central,
            dos_time,
            dos_date,
        });
        at = central_end;
    }
    Ok(out)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    /* EOCD 在文件末尾 22 字节起，后面可能跟着最多 65535 字节的注释 */
    let start = bytes.len().saturating_sub(22 + 0xFFFF);
    bytes[start..]
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .map(|p| start + p)
}

/// 取一个条目的明文（store 直读 / deflate 走 flate2）。
fn decode_entry(bytes: &[u8], e: &Entry) -> Result<String, String> {
    let raw = &bytes[e.data_start..e.data_end];
    let plain = match e.method {
        0 => raw.to_vec(),
        8 => inflate(raw)?,
        m => return Err(format!("条目「{}」用了不支持的压缩方式 {m}", e.name)),
    };
    Ok(String::from_utf8_lossy(&plain).into_owned())
}

fn inflate(raw: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(raw)
        .read_to_end(&mut out)
        .map_err(|e| format!("解压失败（{e}）"))?;
    Ok(out)
}

// ---------------------------------------------------------------- zip 写

/// 重打整包：条目顺序、压缩方式、时间戳全按原来那份，只把 `replace_name` 的内容换掉。
fn repack(
    src: &[u8],
    entries: &[Entry],
    replace_name: &str,
    new_plain: &[u8],
) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(src.len() + new_plain.len());
    let mut central: Vec<u8> = Vec::new();

    for e in entries {
        let offset = out.len() as u32;

        /* 数据段：目标条目重新压，其余原样搬 */
        let (method, crc, comp_len, un_len, body): (u16, u32, usize, usize, Vec<u8>) =
            if e.name == replace_name {
                let deflated = deflate(new_plain)?;
                (
                    8,
                    crc32(new_plain),
                    deflated.len(),
                    new_plain.len(),
                    deflated,
                )
            } else {
                (
                    e.method,
                    e.crc,
                    e.compressed,
                    e.uncompressed,
                    src[e.data_start..e.data_end].to_vec(),
                )
            };

        /* 本地文件头：原样搬，只改 CRC 与两个长度 */
        let mut local = e.local_header.clone();
        local[8..10].copy_from_slice(&method.to_le_bytes());
        local[14..18].copy_from_slice(&crc.to_le_bytes());
        local[18..22].copy_from_slice(&(comp_len as u32).to_le_bytes());
        local[22..26].copy_from_slice(&(un_len as u32).to_le_bytes());
        out.extend_from_slice(&local);
        out.extend_from_slice(&body);

        /* 中央目录：原样搬，改同样那三格 + 本地头偏移 */
        let mut c = e.central.clone();
        c[10..12].copy_from_slice(&method.to_le_bytes());
        c[16..20].copy_from_slice(&crc.to_le_bytes());
        c[20..24].copy_from_slice(&(comp_len as u32).to_le_bytes());
        c[24..28].copy_from_slice(&(un_len as u32).to_le_bytes());
        c[42..46].copy_from_slice(&offset.to_le_bytes());
        /* 时间戳两格保持不变 —— 写回时一个字节都不该碰它们 */
        debug_assert_eq!(
            u16::from_le_bytes([c[12], c[13]]),
            e.dos_time,
            "中央目录的修改时间不该被动过"
        );
        debug_assert_eq!(
            u16::from_le_bytes([c[14], c[15]]),
            e.dos_date,
            "中央目录的日期不该被动过"
        );
        central.extend_from_slice(&c);
    }

    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);

    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes()); // 本盘号
    out.extend_from_slice(&0u16.to_le_bytes()); // 中央目录起始盘
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // 注释长度
    Ok(out)
}

fn deflate(plain: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(plain)
        .map_err(|e| format!("压缩失败（{e}）"))?;
    enc.finish().map_err(|e| format!("压缩失败（{e}）"))
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = flate2::Crc::new();
    c.update(data);
    c.sum()
}

// ---------------------------------------------------------------- XML

/// 取 `text` 里某个 metadata 的值。与 `gcode::model_identity::metadata_value` 同一口径
/// （成对标签 + 自闭合两种形态都认），但这里要的是**"没有"与"有"的区分**，不是探测。
pub(crate) fn metadata_value(xml: &str, key: &str) -> Option<String> {
    let mut at = 0usize;
    while let Some(found) = xml[at..].find("<metadata") {
        let start = at + found;
        let end = start + xml[start..].find('>')?;
        let tag = xml.get(start..end)?;
        let self_closing = tag.trim_end().ends_with('/');
        if tag.contains(&format!("name=\"{key}\"")) {
            if self_closing {
                return attr_value(tag, "value");
            }
            let close = xml[end..].find("</metadata>")? + end;
            let v = xml.get(end + 1..close)?.trim();
            return (!v.is_empty()).then(|| unescape(v));
        }
        at = end + 1;
    }
    None
}

/// 插入或改写一条 metadata。返回新 XML（调用方自己判断有没有变化）。
fn set_metadata(xml: &str, key: &str, value: &str) -> Result<String, String> {
    let tag = format!(" <metadata name=\"{key}\">{}</metadata>\n", escape(value));

    /* 已经有这条 ⇒ 就地改值 */
    if metadata_value(xml, key).is_some() {
        return replace_metadata_value(xml, key, value);
    }

    /* 没有 ⇒ 插在最后一个 `<metadata>` 之后（metadata 组内，绝不落到 resources 之后）*/
    if let Some(last) = last_metadata_end(xml) {
        let mut out = String::with_capacity(xml.len() + tag.len());
        out.push_str(&xml[..last]);
        out.push('\n');
        out.push_str(&tag);
        out.push_str(&xml[last..]);
        return Ok(out);
    }

    /* 连一条 metadata 都没有（极少见）⇒ 插在 `<model ...>` 开标签之后 */
    let model_open = xml
        .find("<model")
        .and_then(|s| xml[s..].find('>').map(|e| s + e + 1))
        .ok_or_else(|| "<model> 开标签都找不到（不是合法 3dmodel.model）".to_string())?;
    let mut out = String::with_capacity(xml.len() + tag.len());
    out.push_str(&xml[..model_open]);
    out.push_str(&tag);
    out.push_str(&xml[model_open..]);
    Ok(out)
}

fn replace_metadata_value(xml: &str, key: &str, value: &str) -> Result<String, String> {
    let needle = format!("name=\"{key}\"");
    let (hit, tag_end) =
        find_metadata(xml, &needle).ok_or_else(|| format!("找不到 {key} 那条 metadata"))?;

    if xml[hit..tag_end].trim_end().ends_with('/') {
        /* 自闭合 ⇒ 换成成对标签那种（值才有地方放） */
        let new = format!("<metadata name=\"{key}\">{}</metadata>", escape(value));
        let mut out = String::with_capacity(xml.len() + new.len());
        out.push_str(&xml[..hit]);
        out.push_str(&new);
        out.push_str(&xml[tag_end + 1..]);
        return Ok(out);
    }

    let close = tag_end + xml[tag_end..].find("</metadata>").ok_or("缺 </metadata>")?;
    let mut out = String::with_capacity(xml.len());
    out.push_str(&xml[..tag_end + 1]);
    out.push_str(&escape(value));
    out.push_str(&xml[close..]);
    Ok(out)
}

/// 找到含 `needle` 的那个 `<metadata>`：返回（标签起点, `>` 的位置）。
fn find_metadata(xml: &str, needle: &str) -> Option<(usize, usize)> {
    let mut at = 0usize;
    while let Some(found) = xml[at..].find("<metadata") {
        let start = at + found;
        let tag_end = start + xml[start..].find('>')?;
        if xml[start..tag_end].contains(needle) {
            return Some((start, tag_end));
        }
        at = tag_end + 1;
    }
    None
}

fn remove_metadata(xml: &str, key: &str) -> String {
    let needle = format!("name=\"{key}\"");
    let mut out = String::with_capacity(xml.len());
    let mut at = 0usize;

    while let Some(found) = xml[at..].find("<metadata") {
        let start = at + found;
        let Some(rel_end) = xml[start..].find('>') else {
            break;
        };
        let tag_end = start + rel_end;

        if !xml[start..tag_end].contains(&needle) {
            /* 不是这一条：原样搬到标签结束 */
            out.push_str(&xml[at..tag_end + 1]);
            at = tag_end + 1;
            continue;
        }

        /* 是这一条：连同**行首缩进**一起跳过（别留下一个空行）。
        `out` 里最后一段是 `xml[at..start]`，把那段的空白尾巴削掉。 */
        let lead = xml[at..start].len() - xml[at..start].trim_end().len();
        out.truncate(out.len() - lead);

        let after = if xml[start..tag_end].trim_end().ends_with('/') {
            tag_end + 1
        } else {
            match xml[tag_end..].find("</metadata>") {
                Some(c) => tag_end + c + "</metadata>".len(),
                None => tag_end + 1,
            }
        };
        /* 元素后面紧跟的那个换行也一起吃掉 */
        at = if xml[after..].starts_with('\n') {
            after + 1
        } else {
            after
        };
    }

    out.push_str(&xml[at..]);
    out
}

/// 最后一个 `<metadata>` 元素**结束**的位置（成对标签或自闭合都算）。
///
/// 新标记插在它后面 —— 这样一定落在 metadata 组里，不会掉到 `<resources>` 之后。
fn last_metadata_end(xml: &str) -> Option<usize> {
    let mut last = None;
    let mut at = 0usize;
    while let Some(found) = xml[at..].find("<metadata") {
        let start = at + found;
        let tag_end = start + xml[start..].find('>')?;
        let end = if xml[start..tag_end].trim_end().ends_with('/') {
            tag_end + 1
        } else {
            tag_end + xml[tag_end..].find("</metadata>")? + "</metadata>".len()
        };
        last = Some(end);
        at = end;
    }
    last
}

/// 读 `3D/3dmodel.model` 的明文（探测用，与写回走同一套 zip 解析）。
fn read_model_entry(bytes: &[u8]) -> Result<String, String> {
    let entries = parse_zip(bytes)?;
    let e = entries
        .iter()
        .find(|e| e.name == MODEL_ENTRY)
        .ok_or_else(|| format!("3MF 里没有 {MODEL_ENTRY}（包不完整）"))?;
    decode_entry(bytes, e)
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let at = tag.find(&needle)? + needle.len();
    let end = tag[at..].find('"')? + at;
    let v = tag[at..end].trim();
    (!v.is_empty()).then(|| unescape(v))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn read_u16(b: &[u8], at: usize) -> Result<u16, String> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| "zip 结构越界".to_string())
}

fn read_u32(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| "zip 结构越界".to_string())
}

/// 备份路径（工作台在改之前用它留一份，出问题能退回）。
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// 改之前留一份 `.bak`（已存在就不覆盖 —— 保住用户第一次的原始件）。
pub fn keep_backup(path: &Path) -> Result<(), String> {
    let bak = backup_path(path);
    if bak.exists() {
        return Ok(());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("读不出来（{e}）"))?;
    write_atomic(&bak, &bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use std::io::Write;

    fn sample_3mf() -> Vec<u8> {
        let model = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                     <model unit=\"millimeter\">\n \
                     <metadata name=\"Application\">BambuStudio-02.08.02.60</metadata>\n \
                     <metadata name=\"Title\"></metadata>\n \
                     <resources/>\n</model>\n";
        zip_with(&[
            ("[Content_Types].xml", b"<Types/>".as_slice(), true),
            (MODEL_ENTRY, model.as_bytes(), true),
            ("Metadata/pick_1.png", b"\x89PNG\r\n\x1a\nfake", false),
        ])
    }

    fn write_tmp(dir: &Path, bytes: &[u8]) -> PathBuf {
        let p = dir.join("ZOffset_Calibration.3mf");
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn an_unmarked_file_reads_as_no_identity_with_a_reason() {
        let d = tempfile::tempdir().unwrap();
        let p = write_tmp(d.path(), &sample_3mf());

        let a = read_identity(&p);
        assert!(a.identity.is_none());
        assert!(!a.is_valid());
        assert!(
            a.problem
                .as_deref()
                .unwrap_or("")
                .contains(CALIBRATION_METADATA_KEY),
            "原因里要点名缺的是哪条 metadata，实际: {:?}",
            a.problem
        );
    }

    #[test]
    fn write_read_remove_round_trip() {
        let d = tempfile::tempdir().unwrap();
        let p = write_tmp(d.path(), &sample_3mf());

        write_identity(&p, Mode::ZOffset).unwrap();
        let a = read_identity(&p);
        assert_eq!(a.identity.as_deref(), Some("ZOffset"));
        assert!(a.is_valid());
        assert_eq!(a.mode(), Some(Mode::ZOffset));

        write_identity(&p, Mode::Repetition).unwrap();
        let b = read_identity(&p);
        assert_eq!(b.identity.as_deref(), Some("Repetition"));
        let xml = read_model_entry(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(
            xml.matches(CALIBRATION_METADATA_KEY).count(),
            1,
            "改写不许留下第二条"
        );

        remove_identity(&p).unwrap();
        let c = read_identity(&p);
        assert!(c.identity.is_none());
        let xml = read_model_entry(&std::fs::read(&p).unwrap()).unwrap();
        assert!(!xml.contains(CALIBRATION_METADATA_KEY), "移除要干净");
    }

    #[test]
    fn rewriting_touches_only_the_model_entry() {
        let d = tempfile::tempdir().unwrap();
        let p = write_tmp(d.path(), &sample_3mf());
        let before = parse_zip(&std::fs::read(&p).unwrap()).unwrap();
        let png_before = {
            let raw = std::fs::read(&p).unwrap();
            raw[before[2].data_start..before[2].data_end].to_vec()
        };

        write_identity(&p, Mode::XyPrecise).unwrap();
        let after_bytes = std::fs::read(&p).unwrap();
        let after = parse_zip(&after_bytes).unwrap();

        assert_eq!(before.len(), after.len(), "条目数不许变");
        for (b, a) in before.iter().zip(after.iter()) {
            assert_eq!(b.name, a.name, "条目顺序不许变");
            if b.name == MODEL_ENTRY {
                assert_ne!(b.crc, a.crc, "目标条目应该真的变了");
                continue;
            }
            assert_eq!(b.method, a.method, "{} 的压缩方式变了", b.name);
            assert_eq!(b.crc, a.crc, "{} 的内容变了", b.name);
            assert_eq!(b.dos_time, a.dos_time, "{} 的时间戳变了", b.name);
            assert_eq!(b.dos_date, a.dos_date, "{} 的日期变了", b.name);
        }
        assert_eq!(
            &after_bytes[after[2].data_start..after[2].data_end],
            png_before.as_slice(),
            "PNG 一个字节都不该被动"
        );
    }

    #[test]
    fn the_marker_lands_inside_the_metadata_group() {
        let d = tempfile::tempdir().unwrap();
        let p = write_tmp(d.path(), &sample_3mf());
        write_identity(&p, Mode::ZOffset).unwrap();

        let xml = read_model_entry(&std::fs::read(&p).unwrap()).unwrap();
        let marker = xml.find(CALIBRATION_METADATA_KEY).unwrap();
        let resources = xml.find("<resources").unwrap();
        assert!(
            marker < resources,
            "标记必须落在 metadata 组内（resources 之前），实际: {xml}"
        );
    }

    #[test]
    fn a_self_closing_marker_can_be_rewritten() {
        let xml = "<model><metadata name=\"MKPSE:Calibration\" value=\"Rough\"/></model>";
        assert_eq!(
            metadata_value(xml, CALIBRATION_METADATA_KEY).as_deref(),
            Some("Rough")
        );
        let out = set_metadata(xml, CALIBRATION_METADATA_KEY, "Precise").unwrap();
        assert_eq!(
            metadata_value(&out, CALIBRATION_METADATA_KEY).as_deref(),
            Some("Precise")
        );
    }

    #[test]
    fn broken_inputs_are_explained_not_swallowed() {
        let d = tempfile::tempdir().unwrap();

        let not_zip = d.path().join("x.3mf");
        std::fs::write(&not_zip, b"not a zip at all").unwrap();
        let a = read_identity(&not_zip);
        assert!(
            a.problem.as_deref().unwrap_or("").contains("zip"),
            "{:?}",
            a.problem
        );

        let no_model = d.path().join("y.3mf");
        std::fs::write(&no_model, zip_with(&[("a.txt", b"hi".as_slice(), false)])).unwrap();
        let b = read_identity(&no_model);
        assert!(
            b.problem.as_deref().unwrap_or("").contains(MODEL_ENTRY),
            "{:?}",
            b.problem
        );

        let gone = d.path().join("nope.3mf");
        let c = read_identity(&gone);
        assert!(c.problem.is_some(), "文件不在也要有原因");
    }

    #[test]
    fn only_the_four_identities_are_valid() {
        for (raw, want) in [
            ("ZOffset", Some(Mode::ZOffset)),
            ("Precise", Some(Mode::XyPrecise)),
            ("Rough", Some(Mode::XyRough)),
            ("Repetition", Some(Mode::Repetition)),
        ] {
            assert_eq!(validate_identity(raw), want, "{raw}");
        }
        assert_eq!(validate_identity("XY"), None, "旧口径不认");
        assert_eq!(validate_identity("随便写的"), None);
        assert_eq!(validate_identity(""), None);
    }

    #[test]
    fn a_wrong_value_is_kept_verbatim_but_invalid() {
        let d = tempfile::tempdir().unwrap();
        let xml = "<model><metadata name=\"MKPSE:Calibration\">Zoffset!</metadata></model>";
        let p = d.path().join("w.3mf");
        std::fs::write(&p, zip_with(&[(MODEL_ENTRY, xml.as_bytes(), true)])).unwrap();

        let a = read_identity(&p);
        assert_eq!(a.identity.as_deref(), Some("Zoffset!"));
        assert!(!a.is_valid());
        assert!(a.problem.is_none(), "有值就不该报 没有这条 metadata");
    }

    fn zip_with(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        for (name, data, deflate_it) in entries {
            let (method, body): (u16, Vec<u8>) = if *deflate_it {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(data).unwrap();
                (8, enc.finish().unwrap())
            } else {
                (0, data.to_vec())
            };
            let crc = crc32(data);
            let offset = out.len() as u32;

            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0x21u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&body);

            central.extend_from_slice(b"PK\x01\x02");
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&method.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0x21u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(body.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        let count = entries.len() as u16;
        out.extend_from_slice(&central);

        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }
}
