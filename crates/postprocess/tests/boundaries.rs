//! 架构边界的机械门禁。
//!
//! 来源仓库 mkp-sr 有 **12** 条这样的断言（靠 `crates/gatecheck` 读 Cargo.toml 实现），
//! 本项目只有**恰好 2** 条。这是**证据等级的真实下降，不是清理**：
//! 文本扫描比 Cargo 的依赖图弱（换个写法就绕过去了）。
//!
//! ## 被丢掉的 10 条，逐条实测归类（Task 13.3）
//!
//! 计划里原本写的是「那 10 条守护的对象在新项目里都不存在」。**这句话经实测是错的** ——
//! 其中 3 条在这里**完全适用、当前恰好成立、但没有任何守卫**。照抄那句话等于给自己发免罪符，
//! 所以改成下面这张表：
//!
//! ### (a) 扫描面真的不存在（5 条）
//! - `no_legacy_project_name` —— 本项目没有改名史、也没有非 ASCII 品牌字符。
//! - `gatecheck_itself_is_isolated_and_wired` —— 没有独立门禁 crate；本文件就是普通
//!   集成测试目标，`cargo test` 必然带上它，「接线」这件事无从腐烂。
//! - `emitted_event_names_are_registered` / `event_registry_path_is_not_rotten` ——
//!   没有前端、没有事件注册表。
//! - `dependency_direction_is_monotonic` —— 需要多 crate 与分层秩登记表；单 package
//!   里没有这个东西。最要紧的那条方向（gcode 是叶子）由本文件判据 1 覆盖，其余方向**无覆盖**。
//!
//! ### (b) 守护对象暂时不存在，但会长出来（2 条）
//! - `no_async_runtime_below_cli` —— 全项目零 async；哪天有人加了 tokio，**没有任何拦阻**。
//! - `guard_self_path_is_not_rotten` —— 本项目的两条判据不用豁免路径（禁词一律拆两段字面量
//!   拼接），所以没有「豁免路径腐烂」这件事；对应的反空转由下面每条判据自带的
//!   文件数下限 + 标记词哨兵承担。
//!
//! ### (c) 适用、当前成立、**没有守卫** —— 这三条是真缺口（3 条）
//! | 不变量 | 当前实测 | 缺的守卫长什么样 |
//! |---|---|---|
//! | 速度换算只在 `ir` 一处 | `src/` 里 `* 60.0` / `* 60f64` 在 `src/ir/` 之外 **0 命中** | 扫 `src/`，跳过 `src/ir/`，命中即红 |
//! | `postproc` 不读 TOML | `src/postproc/` 里 `toml::` **0 命中**（只有 `toml_machine` 这个参数名和注释） | 扫 `src/postproc/`，禁 `toml::` 形态（不能裸禁 `toml` 这个词） |
//! | subscriber 只在可执行边界装配 | `tracing_subscriber` 只出现在 `src/main.rs` 与 `src/pipeline/progress.rs` 的 `#[cfg(test)] mod tests`（那条「进度不写日志」的判据要捕获 layer） | 扫 `src/`，只许 `main.rs`；**必须放过 cfg(test) 段**，否则会咬住那条判据 |
//!
//! 三条都没写，理由是本轮计划把门禁定在「恰好 2 条」（doc.md §3.1）；
//! **代价是这三个不变量今天只靠人守**。将来这个项目长出 async / 第二个可执行 / 前端，
//! 或者上面任何一格从「0 命中」变成非零，对应判据要补上。

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 递归收集 `.rs` 文件。读不到目录直接 panic：判据的扫描面消失必须响亮，
/// 「0 个文件所以全部通过」是本仓库最贵的一类坑（来源仓库 AGENTS §7③）。
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("扫描面读不到：{}（{e}）—— 判据已空转", dir.display()));
    for entry in entries {
        let path = entry.expect("目录项读取失败").path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// 判据 1：gcode 模块保持纯
// ---------------------------------------------------------------------------

/// 禁词**拆两段字面量拼接**。
///
/// 今天本文件不在扫描面内（只扫 `src/gcode/`），所以严格说不拼也不会自我指涉；
/// 拼接是**保险**：来源仓库第一次跑门禁时 6 条断言里有 2 条只命中守卫自己
/// （AGENTS §7①），代价是当时花了一轮才看清。将来若有人把扫描面扩到全 `src/`，
/// 这里不用改一行就仍然成立。
fn forbidden_in_gcode() -> Vec<String> {
    vec![
        // diag 与 gcode 是**同层的两个叶子，互不依赖** —— 来源仓库那条 gcode_stays_pure 的
        // 原话是「零内部依赖」，不是「不依赖上层」。第一版漏了这个名字：那时 `crate::diag`
        // 已经存在（Task 3 先落地），所以漏掉它意味着 gcode 可以静默依赖 diag 而判据全绿。
        format!("{}{}", "crate::", "diag"),
        format!("{}{}", "crate::", "ir"),
        format!("{}{}", "crate::", "postproc"),
        format!("{}{}", "crate::", "pipeline"),
        format!("{}{}", "crate::", "config"),
        // tracing 只禁**代码形态**，不禁这两个词出现在文档里 ——
        // 本模块的文档头就写着「不依赖 tracing」，裸扫这个词会让判据咬到自己的说明文字，
        // 报出来的红长得像真违规（§7③ 的「0 命中 vs 没执行」同族：**假红同样污染归因**）。
        format!("{}{}", "use ", "tracing"),
        format!("{}{}", "tracing", "::"),
    ]
}

/// 掐掉行注释后的代码部分。
///
/// **为什么必须有这一步（实测踩到的，不是预防性洁癖）**：`src/gcode/mod.rs` 的文档头为了
/// 说明「不许引用 `crate::ir` / `crate::postproc` / `crate::pipeline` / `crate::config`」
/// 而**逐个点名了这四个模块**，于是第一次跑本判据得到 4 处命中、全部落在那段说明文字上 ——
/// 判据咬住了自己要解释的规则。这是来源仓库 AGENTS §7① 自我指涉的**镜像**：
/// 守卫本体是干净的，被扫的文件的**文档**才是命中源。
///
/// 处置选的是「让判据对齐意图」而不是「把文档写得含糊」：注释里出现一个模块名**不构成依赖**
/// （注释不参与编译），规则的意图从来是「代码层不许引用」。删掉文档里那四个名字也能过，
/// 但那样最需要说明的地方反而说不清 —— 判据不该逼人把话说糊。
///
/// **这一步引入的洞（登记，不掩盖）**：从第一个 `//` 起截断，遇到字符串字面量里含 `//`
/// 的行（例如一个 URL）会把后半行一起丢掉，那半行里若真有违规引用就漏了。
/// 当前 `src/gcode/` 实测零处此类字面量；真出现时正确修法是接一个词法分析，不是放宽断言。
fn code_part(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

#[test]
fn gcode_module_stays_pure() {
    let gcode_dir = repo_root().join("src").join("gcode");
    let files = rust_files(&gcode_dir);

    // 反空转哨兵 A：文件数下限。实测 10 个（mod + 9 个子模块）；
    // 写 8 是留改动余量，但**低于 8 就说明路径写错或文件丢了**，不是「恰好都合规」。
    assert!(
        files.len() >= 8,
        "扫描面只有 {} 个 .rs 文件（期望 ≥ 8）—— 路径写错或文件丢了，判据不成立：{}",
        files.len(),
        gcode_dir.display()
    );

    // 反空转哨兵 B：标记词至少命中一次，证明读到的真是 Rust 源码而不是空串。
    let mut saw_marker = false;
    let forbidden = forbidden_in_gcode();
    let mut hits: Vec<String> = Vec::new();

    for path in &files {
        let text =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("读不到 {}（{e}）", path.display()));
        if text.contains("pub fn ") {
            saw_marker = true;
        }
        let rel = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        for (i, line) in text.lines().enumerate() {
            let code = code_part(line);
            for pat in &forbidden {
                if code.contains(pat.as_str()) {
                    hits.push(format!("{rel}:{}  [{pat}]  {}", i + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        saw_marker,
        "扫描面里一个 `pub fn ` 都没有 —— 读到的不是 Rust 源码，判据已空转"
    );
    assert!(
        hits.is_empty(),
        "gcode 模块不许引用本 crate 的其他模块、不许依赖 tracing。\n\
         它是最底层叶子：不知道 TOML / MKP / machine / wiping / tower / 编排层的存在，\n\
         需要诊断就把信息返回给上层。命中 {} 处：\n{}",
        hits.len(),
        hits.join("\n")
    );
}

// ---------------------------------------------------------------------------
// 判据 2：全项目禁 FMA
// ---------------------------------------------------------------------------

/// 禁词同样拆两段拼接 —— **这一条是真需要**：本文件在自己的扫描面内（扫全 `src/`
/// 与 `tests/`），不拼就会命中自己，正是来源仓库 AGENTS §7① 那个坑。
fn fma_pattern() -> String {
    format!("{}{}", "mul", "_add")
}

/// 禁止 `f64::mul_add`。
///
/// **这是实测事实，不是洁癖**：旧 Go 侧 rib 圆角的 epsilon 比较（`polygon.go:23` 排序 /
/// `removeDuplicatePoints` / `rib.go:351` 的 `math.Abs(cosangle) < cosAngleTol`）撞上
/// **arm64 允许 FMA 融合、amd64 不融合** ⇒ 阈值一翻就**换几何分支**。
/// `rib/mid_tower_h250` 实测 1201 vs 1195 行、坐标 X/Y 转置，**在同一份 Go 代码上就已经
/// 不可复现**，那条用例已被移出字节 golden（vendored 的 23 块里没有它）。
///
/// 所以几何层必须按 Go 源码的运算顺序逐字复刻，禁止 `mul_add` 与任何可被编译器自由融合的
/// 写法。clippy 的 `suboptimal_flops` 给的建议正是 mul_add，因此 Cargo.toml 里**刻意不
/// deny 它** —— 真守卫就是这条文本扫描。
///
/// 某条 CASE 字节不等时的流程（不许临场放松）：先打印中间 cos 值与分支走向，证明是 FMA 类
/// 差异而非逻辑错，才允许把该条降档并写进诚实边界。**不允许直接加容差蒙过去**（行数都不同，
/// 容差无意义）。
#[test]
fn no_fma_anywhere() {
    let pattern = fma_pattern();
    let mut files = rust_files(&repo_root().join("src"));
    files.extend(rust_files(&repo_root().join("tests")));

    // 反空转哨兵：实测 70 个文件（src 53 + tests 17，随 Task 推进只增不减）。
    assert!(
        files.len() >= 30,
        "扫描面只有 {} 个 .rs 文件（期望 ≥ 30）—— 路径写错或文件丢了，判据不成立",
        files.len()
    );

    let mut hits: Vec<String> = Vec::new();
    let mut saw_marker = false;
    for path in &files {
        let text =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("读不到 {}（{e}）", path.display()));
        if text.contains("f64") {
            saw_marker = true;
        }
        let rel = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        for (i, line) in text.lines().enumerate() {
            if code_part(line).contains(pattern.as_str()) {
                hits.push(format!("{rel}:{}  {}", i + 1, line.trim()));
            }
        }
    }

    assert!(
        saw_marker,
        "扫描面里一个 `f64` 都没有 —— 读到的不是本项目的源码，判据已空转"
    );
    assert!(
        hits.is_empty(),
        "禁止 FMA 融合（arm64/amd64 分叉会换几何分支，见本判据文档）。命中 {} 处：\n{}",
        hits.len(),
        hits.join("\n")
    );
}
