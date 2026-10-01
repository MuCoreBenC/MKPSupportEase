# 写盘纪律补齐 收尾总结 —— `crates/*` 从此受管，写真源那一处上了三道闸

日期：2026-09-24　分支：`feat/b04-p3-migration`
上游：Task 15 / M4 的收尾发现（`.comate/specs/b04-m4-preset-unify/summary.md` §5）
归属：长期 `tasks.md` 的 **Task 19.1**（提前做掉的一小块，剩余部分见那边的标注）

---

## 1. 六笔提交

| 提交 | 内容 |
|---|---|
| `bbf4c64` | Task 2 —— 生产四处写盘点写明豁免理由与退役条件（零逻辑改动） |
| `9a29f26` | Task 2b —— 测试十处一次性豁免（模块级 / 文件级，不逐行） |
| `7b5db00` | Task 3 —— `registry_edit` 三道闸 + `sync_baseline` 落点断言 |
| `e5635b9` | Task 4 —— `clippy.toml` 上移到仓库根 |
| `20d4a55` | Task 5 —— 源码扫描断言覆盖 `crates/*/src` |
| `9a93a13` | Task 6 —— 补上指向空气的幂等门禁 + 两处写值立场互相点明 |

顺序是刻意的：**先让 15 处合规（Task 2 / 2b），再打开禁列（Task 4）**。
反过来做，上移那一笔会直接红；而 `#[allow]` 在禁列未生效时是无操作，
所以两笔都能各自全绿 —— 与 M4b 那次"改名与 rustfmt 必须同笔"是同一条原则的反面：
**能拆成两笔各自全绿的，就不要并成一笔。**

---

## 2. 为什么现在做（不是等 Task 19）

M4c 把 `registry_edit::registry_path()` 从 crate 自带副本改成了
`presets/registry/param_registry.toml` —— 真数据。而当时：

- `clippy.toml` 住在 `src-tauri/`，**两个新 crate 完全不受管**；
- 唯一的源码扫描断言只扫 `src-tauri/src/workbench/upstream/` 一层目录；
- 于是那处写盘是**裸 `std::fs::write`**：截断式、无备份、无原子、无回读。

唯一的缓和因素是它**生产代码里零调用者**（`src-tauri` 根本不依赖 `mkpse-preset`）——
装好的枪还没接扳机。所以有时间把闸做对，而不是赶着补。

---

## 3. 每处的处置与退役条件

### 生产 5 处

| 位置 | 处置 | 退役条件 |
|---|---|---|
| `preset/registry_edit.rs` `set_range` | **三道闸**（见 §4） | Task 19 统一入口后改成转调它 |
| `preset/generate.rs` `sync_baseline` | `#[allow]` + 理由 + **落点断言** | Task 18 产物归属定案后与基线目录一起退役 |
| `preset/generate.rs` `write_all` | `#[allow]` + 理由（开发工具、写自己 assets、默认 `--check`） | 同上 |
| `postprocess/pipeline/mod.rs` `write_atomic` | `#[allow]` + 理由（`.part` + rename，本身就是禁列想要的模式） | Task 19 统一入口 |
| `postprocess/main.rs` `cmd_init` | `#[allow]` + 理由（先判 `exists` 再写，截断语义上不可能） | Task 19 统一入口 |

每处写的是**"为什么这处写盘不违反禁列想防的那件事"**，不是"这里需要写盘"。
`sync_baseline` 的理由单独拎出来说：它的风险不是"半个文件"，而是
**它写的是内核判据的期望值**（全仓唯一），与「禁止 `UPDATE_GOLDEN=1`」防的是同一件事。

### 测试 10 处

一次性豁免（4 个 preset 测试文件 + 2 个 postprocess 测试文件 + `golden_common/mod.rs`
+ `generate.rs` 与 `registry_edit.rs` 的 `mod tests`），理由统一一句：
**测试写临时文件是正当的，这条纪律管生产代码** —— 与扫描判据只看
`#[cfg(test)]` 之外那部分是同一口径，两边不会互相矛盾。

---

## 4. 写真源那一处的三道闸

1. **no-op 不写盘** —— 无意义的 mtime 跳动会让 `git status` 与构建缓存都变吵；
2. **落盘前** `only_declared_keys_changed`：改出来的文本只许在本次声明过的键上与原文不同，
   多一处差异就拒绝，**文件一个字节不动**。口径是**行的多重集合差**而不是逐行位置比
   （`toml_edit` 保序，但删一个键会让行数变，位置比会在"删掉 step"这种正常情形上误报）；
3. **落盘后**回读复检：字节必须与预期一致，且**还能被 `toml_edit` 解析回来**。

第 2 条原来只活在测试里（`only_those_lines_change`）。判据在测试里，意味着
"生产路径真的改出了别的差异"时没有任何东西会拦 —— 而这个函数写的是真数据。

落盘走同目录 `.writing` 临时文件 + rename，与内核 `pipeline::write_atomic` 同形状。
不用 `tempfile`：它在本 crate 只是 dev-dependency。

`set_range` 拆出 `set_range_at(path, …)` 的唯一理由是**可测** ——
三道闸都要在临时目录里真造一次失败。

---

## 5. 三次探针，两次打穿了东西

### ① Task 4.4 反向探针（成功）

摘掉内核 `write_atomic` 上那个 `#[allow]` ⇒ clippy 报
`crates/postprocess/src/pipeline/mod.rs:604` 违规并编译失败 ⇒ 证明根 `clippy.toml`
**真的**作用到了 `crates/postprocess`，不是"看着像生效"。探针已还原。

### ② Task 5.5 探针第一次没红 —— 发现判据真盲区

往白名单外的 `preset/src/read.rs` **末尾**加一个 `fs::write` 的函数，判据 `3 passed`。
原因不是探针放错位置：口径"第一个 `#[cfg(test)]` 之前"把 `mod tests`（165 行）
到文件末尾（400 行）整段切掉了。**Rust 允许在 `mod tests` 之后继续写生产代码**，
所以那是真盲区 —— **既有那条 `upstream/mod.rs` 判据有同样的洞**（那边目录小，还没踩到）。

改成按大括号配平剔除 `#[cfg(test)] mod {…}` 块，其余全部保留；两处选"宁可多扫"
（属性标在 `use` 上只跳过属性本身；配不平则保留剩余全部）—— 误红会被人看见，漏扫不会。
修复后重跑：**点名 `[("preset/src/read.rs", "fs::write")]` 并 FAILED**，还原后全绿。

顺手把这个场景固化成两条断言（`mod tests` 之后的写盘要抓到、属性标在 `use` 上不许吞代码）。

### ③ Task 1 的命中数第一次是假的

带根 `clippy.toml` 第一次跑 `--all-targets` 报 **9 处**且 `postprocess` 一处都没有；
`cargo clean -p` 两个 crate 后重跑是 **15 处**。**`cargo clippy` 会跳过未变更的 crate。**
已写进 HANDOFF §9 第 11 条与根 `clippy.toml` 顶部。

---

## 6. 两处"指向空气"的修复

- `generate.rs` 模块头指向的门禁 `scripts/check_generator_purity.py` **本仓不存在**
  （M4a 搬进来时就断了）。**没有删掉那句话，而是把门禁真的补上**：
  `the_generator_stays_pure` 扫 `generate.rs` 生产部分，`new_v4` / `SystemTime` /
  `Instant::now` / `Utc::now` / `Local::now` / `rand::` 一个都不许出现，带反空转。
  理由：产物要与基线逐字节比（K-G0'），生成器里有一处时间戳，那条黄金判据就会从
  "证明"退化成"每次重新同步基线"，而基线一旦开始随手同步就等于没有基线。
- `recipe_edit::set_override`（允许新增键）与 `write.rs` 的 K-P3（拒绝新增键）
  立场相反 —— 前者改我们自己的配方、后者改用户的预设。两边文档互相点明，
  并写了"别把一边的结论搬到另一边"。**没动代码。**

---

## 7. 验收结果（九条全绿）

| 项 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| 同上 + `-p mkp-support-ease --features workbench` | exit 0 |
| `cargo test`（默认三成员） | 18 + 内核 249+2 + 预设 114 全 ok |
| `cargo test -p mkp-support-ease --features workbench` | **264 passed** |
| `cargo test -p mkpse-postprocess` | **249 passed + 2 ignored** |
| `cargo test -p mkpse-preset` | **114 passed**（75 单元 + 39 集成） |
| `cargo tree -d` | **61 条 / 27 名** —— 与 M4b 的基线一致，本轮零新增 |
| `npm run lint` / `npm run build` | exit 0 / 902 ms |

**`presets/` 真源核验**（比"与某份旧清单比"更强的做法）：
`git diff --stat HEAD -- presets/` 为空、`git status --short -- presets/` 为空、
逐文件核「索引 blob == 工作区内容」**12/12 相同**、
`git log -1 -- presets/` 停在 **`203c287`（PR #10）** —— 也就是说
`presets/` 最后一次被改是本轮与 M4 之前。`registry/param_registry.toml`
的 sha256 仍是 `56BDCEC9…`，与 M4 期间实测的完全一致。

---

## 8. 留下的缺口（都写进了长期 `tasks.md` 的 Task 19）

1. **「所有写盘只经一处」没做到** —— 现在是"每处各自原子写 + 逐处豁免"。
   `fsx::atomic` 住在 `src-tauri`，而依赖方向是 `src-tauri → preset → postprocess`，
   要真正统一入口得把它下沉到内核（doc §3 的 B1）。本轮刻意没做：内核刚搬完，
   它的判据靠"零 diff"背书，不该在这一批动它。
2. **`presets/` 的 sha256 判据仍是人工核**，没有自动判据 —— 要先定"什么时候允许变"，
   而 Task 18 就要往 `presets/` 加文件。
3. 备份 / 回收站 / 撤销语义未定（Task 19.3）。
4. `registry_edit` 本身仍是**开发态工具**，且 `src-tauri` 不依赖 `mkpse-preset` ——
   这条写盘面到现在还没有生产调用者。接线是 M5 的事。
