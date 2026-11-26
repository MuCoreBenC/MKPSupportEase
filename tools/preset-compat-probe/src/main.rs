//! 用**某一个历史 tag 的真实 `crates/preset`** 读候选预设，输出稳定 JSON。
//!
//! 这个探针只调用 `read_preset_from_bytes`（0.0.18 起就有的入口），
//! 不依赖新版本才加的 `probe_preset_compat` —— 否则就不是“真实旧解析器”了。

use std::path::Path;

use serde_json::json;

fn main() {
    let mut out = Vec::new();
    for file in std::env::args().skip(1) {
        let path = Path::new(&file);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.clone());
        let result = match std::fs::read(path) {
            Err(_) => json!({
                "file": name,
                "result": "fail",
                "reason": "read",
                "detail": "读不出文件",
            }),
            Ok(bytes) => match String::from_utf8(bytes) {
                Err(_) => json!({
                    "file": name,
                    "result": "fail",
                    "reason": "utf8",
                    "detail": "不是 UTF-8",
                }),
                Ok(text) => match preset::read_preset_from_bytes(text) {
                    Ok(_) => json!({
                        "file": name,
                        "result": "pass",
                    }),
                    Err(err) => {
                        let message = err.to_string();
                        let unknown = field_of(&message, "unknown field `");
                        let missing = field_of(&message, "missing field `");
                        json!({
                            "file": name,
                            "result": "fail",
                            "reason": if unknown.is_some() {
                                "unknownKey"
                            } else if missing.is_some() {
                                "missingRequired"
                            } else if message.contains("invalid type:") {
                                "typeMismatch"
                            } else {
                                "syntax"
                            },
                            "key": unknown.or(missing).unwrap_or_default(),
                            "detail": message,
                        })
                    }
                },
            },
        };
        out.push(result);
    }
    println!(
        "{}",
        serde_json::to_string(&out).expect("探针结果必须能序列化")
    );
}

fn field_of(message: &str, marker: &str) -> Option<String> {
    let rest = message.split(marker).nth(1)?;
    let name = rest.split('`').next()?;
    (!name.is_empty()).then(|| name.to_owned())
}
