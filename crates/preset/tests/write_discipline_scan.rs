//! **写盘纪律的源码扫描断言**，覆盖 `crates/preset/src/` 与 `crates/postprocess/src/`
//! （b04 写盘纪律补齐 / Task 5）。
//!
//! # 为什么 clippy 之外还要这一条
//!
//! 根 `clippy.toml` 已经管住这两个 crate 了（Task 4），但它有两个盲区：
//!
//! 1. **禁列是按方法名匹配的**。换一个等效写法（比如自己包一层 `std::io::Write`、
//!    或者哪天 std 多一个新入口）它就不响；
//! 2. **一份 crate 级的 `clippy.toml` 就能整份屏蔽根那份**，而且不会有任何警告
//!    （两次探针实测的结论，写在根 `clippy.toml` 顶部）。也就是说纪律本身可以被
//!    一个文件静默关掉。
//!
//! 这条判据是文本扫描，不依赖 clippy 的配置解析，所以上面两种失效方式都绕不过它。
//! 两条一起才叫"机器拦得住"。
//!
//! # 口径
//!
//! - 只看**生产部分**：`#[cfg(test)]` 之前的那一段，再滤掉 `//` 开头的行
//!   （与 `src-tauri/src/workbench/upstream/mod.rs` 那条既有判据同一口径；
//!   测试写临时文件是正当的，纪律管生产代码）；
//! - **递归**扫（`crates/preset/src/bin/` 就在子目录里，上面那条既有判据只扫一层，
//!   照抄会漏）；
//! - 路径用 `CARGO_MANIFEST_DIR` 拼，**不用 `file!()`** —— 建了 workspace 之后
//!   `file!()` 的基准从 crate 根变成仓库根，用它拼路径的判据会以"报错"的形式失效
//!   （那段血泪写在 `upstream/mod.rs` 的注释里）。

use std::path::{Path, PathBuf};

/// 会截断已有文件、或者能拿到写句柄的那些入口。与根 `clippy.toml` 的禁列同一批。
const FORBIDDEN: &[&str] = &[
    "fs::write",
    "File::create",
    "OpenOptions",
    "File::options",
    "fs::remove_file",
    "fs::remove_dir",
    "create_dir_all",
];

/// 允许直接写盘的文件，**逐条写清为什么**。
///
/// 这里只放文件；具体到函数的理由与退役条件写在代码里那个 `#[allow]` 旁边。
/// 往这张表里加一行之前先问一句：这处写盘崩在半路的话，坏掉的是什么？
const ALLOWED: &[(&str, &str)] = &[
    (
        "preset/src/registry_edit.rs",
        "写 presets/ 真源（开发态改区间）。`.writing` 临时文件 + rename，\
         外加落盘前「只许声明过的键变」与落盘后回读复检，共三道闸",
    ),
    (
        "preset/src/generate.rs",
        "gen-presets 这个仓库内开发工具：write_all 写自己的 assets/presets/（默认动作是 --check）；\
         sync_baseline 写内核判据夹具，另有落点断言只允许真基线目录或临时目录",
    ),
    (
        "postprocess/src/pipeline/mod.rs",
        "写 `<out>.part` 再 rename 顶替 —— 目标文件到 rename 那一刻都没被碰过，\
         本身就是禁列想要的那个模式",
    ),
    (
        "postprocess/src/main.rs",
        "CLI 的 `init` 子命令：先判 exists 再写，已存在就报错退出；写的是用户在命令行指定的路径",
    ),
];

fn crates_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/preset
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/preset 必有父目录 crates/")
        .to_path_buf()
}

/// 取"生产部分"：剔除 `#[cfg(test)]` 标注的 `mod` 块，其余**全部保留**，
/// 最后滤掉整行注释。
///
/// # 为什么不是"第一个 `#[cfg(test)]` 之前"
///
/// 第一版就是那么写的（照抄 `upstream/mod.rs` 那条既有判据），而 Task 5 的探针立刻
/// 打穿了它：往 `preset/src/read.rs` **末尾**加一个直接 `fs::write` 的函数，
/// 判据**没红** —— 那个文件的 `mod tests` 在第 165 行、全文 400 行，末尾那段被整体切掉了。
/// Rust 语法允许在 `mod tests` 之后继续写生产代码，所以那是判据的真盲区，
/// 不是探针放错了位置。
///
/// 现在的口径：只把 `#[cfg(test)] mod … { … }` 这种块按大括号配平剔掉。
/// 两处刻意选"宁可多扫"的方向 —— 误红会被人看见，漏扫不会：
///
/// - 属性标注的不是 `mod`（比如 `#[cfg(test)] use …`）：只跳过属性本身，后面照扫；
/// - 大括号配不平（字符串里有孤立的 `{`）：剩余部分**全部保留**。
fn production_part(src: &str) -> String {
    const CFG_TEST: &str = "#[cfg(test)]";
    let bytes = src.as_bytes();
    let mut kept = String::with_capacity(src.len());
    let mut cursor = 0usize;

    while cursor < src.len() {
        let Some(rel) = src[cursor..].find(CFG_TEST) else {
            kept.push_str(&src[cursor..]);
            break;
        };
        let at = cursor + rel;
        kept.push_str(&src[cursor..at]);
        let after = at + CFG_TEST.len();

        if !src[after..].trim_start().starts_with("mod ") {
            cursor = after;
            continue;
        }
        let Some(brace_rel) = src[after..].find('{') else {
            break;
        };
        let mut depth = 0usize;
        let mut j = after + brace_rel;
        let mut closed = false;
        while j < src.len() {
            match bytes[j] {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        j += 1;
                        closed = true;
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if closed {
            cursor = j;
        } else {
            kept.push_str(&src[after..]);
            break;
        }
    }

    kept.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("读不到 {}：{e}", dir.display()));
    for e in entries {
        let p = e.expect("目录项").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// 扫一遍，返回 (扫过的文件数, 违规清单)。违规项是 `(相对路径, 命中的 API)`。
fn scan() -> (usize, Vec<(String, &'static str)>) {
    let root = crates_root();
    let mut files = Vec::new();
    for member in ["preset", "postprocess"] {
        rust_files(&root.join(member).join("src"), &mut files);
    }

    let mut bad = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .expect("在 crates/ 之下")
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED.iter().any(|(f, _)| *f == rel) {
            continue;
        }
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读不到 {rel}：{e}"));
        let prod = production_part(&src);
        for api in FORBIDDEN {
            if prod.contains(api) {
                bad.push((rel.clone(), *api));
            }
        }
    }
    (files.len(), bad)
}

#[test]
fn only_the_listed_files_may_write_to_disk() {
    let (scanned, bad) = scan();
    assert!(
        bad.is_empty(),
        "这些生产文件直接写盘，但不在白名单里：{bad:?}\n\
         要么改成走已有的原子写、要么把它加进 ALLOWED 并写清「崩在半路会坏掉什么」"
    );
    // 反空转：扫不到文件的话上面那条永远是绿的
    assert!(
        scanned >= 60,
        "只扫到 {scanned} 个 .rs —— 路径大概拼错了（内核 src 一家就有 50 多个）"
    );
}

/// 白名单里的每一条都必须**真的对应一个存在的文件**，而且**真的用到了写盘 API**。
///
/// 否则白名单会烂成一张只增不减的清单：文件改名了、写盘改走统一入口了，
/// 那一行还留着，等于给未来的写盘偷偷留了口子。
#[test]
fn the_allowlist_has_no_dead_entries() {
    let root = crates_root();
    for (rel, why) in ALLOWED {
        let path = root.join(rel);
        assert!(path.is_file(), "白名单里的 {rel} 不存在了 —— 该删这一行");
        assert!(!why.trim().is_empty(), "{rel} 的豁免理由是空的");
        let src = std::fs::read_to_string(&path).expect("读得到");
        let prod = production_part(&src);
        assert!(
            FORBIDDEN.iter().any(|api| prod.contains(api)),
            "{rel} 的生产部分已经不直接写盘了 —— 把它从白名单里删掉，\
             别留着给未来的写盘当口子"
        );
    }
}

/// 反空转探针：喂一段构造出来的源码，扫描逻辑必须认出它。
///
/// 这条防的是"扫描函数自己坏了"（比如 `production_part` 把整个文件都切掉了），
/// 那种情况下上面两条判据会一直绿。
#[test]
fn the_scan_would_catch_a_write() {
    let sneaky = "fn save(p: &str) {\n    std::fs::write(p, \"x\").unwrap();\n}\n";
    let prod = production_part(sneaky);
    assert!(
        FORBIDDEN.iter().any(|api| prod.contains(api)),
        "扫描认不出一行直接的 fs::write —— 判据空转了"
    );

    // 而 `#[cfg(test)]` 之后的同一行**不该**被算进去（否则测试面会误红）
    let only_in_tests = format!("fn ok() {{}}\n#[cfg(test)]\nmod tests {{\n{sneaky}}}\n");
    let prod = production_part(&only_in_tests);
    assert!(
        !FORBIDDEN.iter().any(|api| prod.contains(api)),
        "测试里的写盘被算成了生产违规 —— 口径与既有判据不一致"
    );

    // **写在 `mod tests` 之后**的生产代码也必须被扫到。
    // 这就是 Task 5 的探针打穿第一版的那个场景：那时的口径是"第一个 `#[cfg(test)]`
    // 之前"，于是往 `read.rs` 末尾加的 `fs::write` 完全不在视野里。
    let after_tests = format!("#[cfg(test)]\nmod tests {{\n    fn t() {{}}\n}}\n{sneaky}");
    let prod = production_part(&after_tests);
    assert!(
        FORBIDDEN.iter().any(|api| prod.contains(api)),
        "写在 mod tests 之后的生产写盘漏掉了 —— 这正是第一版的盲区"
    );

    // `#[cfg(test)]` 标注在 `use` 上时不许把后面的生产代码一起吞掉
    let attr_on_use = format!("#[cfg(test)]\nuse std::io::Write;\n{sneaky}");
    let prod = production_part(&attr_on_use);
    assert!(
        FORBIDDEN.iter().any(|api| prod.contains(api)),
        "属性标在 use 上时后面的生产代码被误剔除了"
    );
}

/// **生成器的幂等门禁**（与写盘无关，但同属"源码级文本门禁"，所以住在这个文件里）。
///
/// `generate.rs` 的模块头写着"这里不许出现 `uuid::new_v4()` / `now()` / `SystemTime`"，
/// 并注明门禁是 `scripts/check_generator_purity.py` —— **那个脚本没有跟着搬进来**，
/// 从 M4a 到现在那句话一直指向一个不存在的文件。
///
/// 指向空气的门禁比没有门禁更糟：它让人以为有东西在看着。所以这条判据把它补上。
///
/// 为什么这件事要拦：产物要逐字节与基线比（K-G0'）。生成器里只要有一处时间戳或随机
/// uuid，同一份配方每次生成的结果就不一样，那条黄金判据会从"证明"变成"每次都得重新
/// 同步基线"，而基线一旦开始随手同步就等于没有基线。
#[test]
fn the_generator_stays_pure() {
    const IMPURE: &[&str] = &[
        "new_v4",
        "SystemTime",
        "Instant::now",
        "Utc::now",
        "Local::now",
        "rand::",
    ];
    let path = crates_root().join("preset/src/generate.rs");
    let src = std::fs::read_to_string(&path).expect("读得到 generate.rs");
    let prod = production_part(&src);
    // 反空转：切完之后还得剩下正文（`render` / `write_all` 那些都在里面）
    assert!(
        prod.contains("pub fn write_all"),
        "generate.rs 的生产部分被切空了 —— 这条判据在空转"
    );
    for bad in IMPURE {
        assert!(
            !prod.contains(bad),
            "generate.rs 的生产部分出现了 `{bad}` —— 生成器必须幂等：\
             uuid 与发布时间只能来自配方，否则产物与基线的逐字节判据（K-G0'）会从\
             「证明」退化成「每次重新同步基线」"
        );
    }
}
