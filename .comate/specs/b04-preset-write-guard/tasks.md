# 写盘纪律补齐 施工计划 —— 让 `crates/*` 也受管，并给写真源那一处加真闸

按 `doc.md` §8 的推荐执行：**A（`clippy.toml` 上移到仓库根）+ B3（只给写真源那处加原子写）
+ `sync_baseline` 加路径断言**。要改方案就先改 `doc.md`，别在施工中途改。

顺序有讲究：**先让 5 处合规（Task 2–3），再打开禁列（Task 4）**。
反过来做的话，上移那一笔会让 CI 直接红 —— 而 `#[allow]` 在禁列还没生效时是无操作，
所以"先合规、后开闸"两笔都能绿。这是 M4b 那次"改名与 rustfmt 必须同笔"的反面教训：
能拆成两笔各自全绿的，就不要并成一笔。

> **施工位置（每完成一笔就更新这里，防对话中断丢位置）**
>
> - [x] Task 1 基线与命中点核准 —— 15 处（生产 5 + 测试 10）
> - [x] Task 2 生产四处豁免 —— `bbf4c64`
> - [x] Task 2b 测试十处一次性豁免 —— `9a29f26`
> - [x] Task 3 真源三道闸 + 基线落点断言 —— `7b5db00`
> - [x] Task 4 `clippy.toml` 上移到根（含反向探针） —— `e5635b9`
> - [x] Task 5 源码扫描断言（含探针打穿第一版后的口径修复） —— `20d4a55`
> - [x] Task 6 补上指向空气的门禁 + 两处立场互相点明 —— `9a93a13`
> - [ ] **Task 7 全量验收与收口（下一步从这里接）**
>
> 分支领先 `main` **22 笔**，工作区只剩本 spec 目录未提交。

- [x] Task 1: 基线与命中点核准
    - 1.1: ✅ 工作区干净、HEAD `9c0a8ea`、领先 main 16 笔；PR #11 上一轮 CI **全绿**（run `35936554601`）
    - 1.2: ✅ **命中点实测 15 处**（生产 5 + 测试 10，清单见 `doc.md` §1.3 / §1.3.1）。
      踩到一个坑并记进 doc：**`cargo clippy` 会跳过未变更 crate**，第一次只报 9 处、
      `postprocess` 一处都没有；`cargo clean -p` 两个 crate 后重跑才是 15 处
    - 1.3: 基线沿用上一轮实测：默认 18 / 内核 249+2 / 预设 106 / 工作台 264
    - 1.4: `presets/` 12 个文件 sha256 清单（验收后删）
    - 1.5: ✅ `crates/*/src` 下**没有** `file!()`（新扫描断言放心用 `CARGO_MANIFEST_DIR`）

- [ ] Task 2: 给生产 5 处加豁免与理由（不动一行逻辑）
    - 2.1: `crates/postprocess/src/pipeline/mod.rs:597` —— `#[allow]` + 理由：已是
      `.part` + rename 原子模式、rename 失败回落 copy；**退役条件**写清（Task 19 统一入口后收回）
    - 2.2: `crates/postprocess/src/main.rs:248` —— `#[allow]` + 理由：CLI 写用户指定的输出文件
    - 2.3: `crates/preset/src/generate.rs:145`（`write_all`）—— `#[allow]` + 理由：
      开发工具、写自己的 `assets/presets/`、默认动作是 `--check`
    - 2.4: `crates/preset/src/generate.rs:284`（`sync_baseline`）—— `#[allow]` + 理由，
      **并在函数文档顶部写明：它会改内核判据的夹具**（`crates/postprocess/tests/fixtures/presets/`）
    - 2.5: 判据：`cargo clippy --all-targets -- -D warnings` 仍绿（此时禁列尚未覆盖 crates，
      这一笔只是"提前合规"，不该有任何行为变化）；三组数字不变
    - 2.6: 提交

- [ ] Task 2b: 给测试里那 10 处一次性豁免（模块级 / 文件级，不逐行）
    - 2b.1: `crates/preset/src/generate.rs` 的 `#[cfg(test)] mod tests` 上加一处 `#[allow]`
      （盖住 `:335` 与 `:408`）
    - 2b.2: `crates/preset/tests/` 四个文件顶部各加 `#![allow(...)]`：
      `lineage.rs` / `load_ir.rs` / `recipe.rs` / `registry_branch_diff.rs`
    - 2b.3: `crates/postprocess/tests/` 三处：`cli.rs` / `pipeline.rs` 文件顶部，
      `golden_common/mod.rs:68` 那处按它的引入方式选 `#[allow]` 落点
    - 2b.4: 理由统一写「测试写临时文件是正当的；这条纪律管的是生产代码」，
      与源码扫描断言只看 `#[cfg(test)]` 之前的口径一致
    - 2b.5: 判据：`cargo clean -p mkpse-preset -p mkpse-postprocess` 之后再跑
      带根 `clippy.toml` 的 `--all-targets` ⇒ **零命中**（不 clean 的结果不算）
    - 2b.6: 提交

- [ ] Task 3: `sync_baseline` 的路径断言 + `registry_edit` 的真闸
    - 3.1: `sync_baseline` 加断言：目标路径必须落在 `crates/postprocess/tests/` 之下，
      否则 `Err` —— 它是唯一能动判据基线的写盘点
    - 3.2: `registry_edit::set_range` 改成**原子写**（同目录临时文件 + fsync + persist，就地实现；
      不下沉到内核，理由见 `doc.md` §3 的 B3）
    - 3.3: `set_range` 落盘**前**自检：把测试里那条"只有 min/max/step 那几行变"
      （`registry_edit.rs:227` 的 `only_those_lines_change`）提到生产路径；
      不满足就**拒绝落盘并报错**，文件一个字节不动
    - 3.4: `set_range` 落盘**后**回读复检：重读文件、重新解析，与期望不一致就报错
    - 3.5: 判据三条（都要真造一次失败，不是只看绿）：
      - 构造"会多改一行"的输入 ⇒ 拒绝落盘，且目标文件 sha256 不变
      - 写后回读不一致 ⇒ 报错（临时目录造）
      - 正常路径：改一个 `min` ⇒ 只有那一行变、回读能解析、`load_param_registry()` 仍是 74 条
    - 3.6: 判据：`presets/` 12 个文件 sha256 与 1.4 一致（这一批最容易把真数据写坏）
    - 3.7: 提交

- [ ] Task 4: `clippy.toml` 上移到仓库根
    - 4.1: `src-tauri/clippy.toml` 整份移到仓库根，`src-tauri` 那份删掉；5 条禁列原样带过来
    - 4.2: 在根那份顶部补一段说明：**就近优先、不合并**（两次探针的结论），
      所以任何 crate 一旦自带 `clippy.toml` 就会**整份屏蔽**根这份 —— 这是陷阱，写明
    - 4.3: 判据：`cargo clippy --all-targets -- -D warnings` 绿、工作台那档也绿
      （Task 2/3 已让 5 处合规，这一笔应该直接绿）
    - 4.4: **反向探针**：临时去掉 `pipeline/mod.rs:597` 那个 `#[allow]` ⇒ clippy 必须红，
      证明根那份真的作用到了 `crates/postprocess`；撤销探针
    - 4.5: 提交

- [ ] Task 5: 源码扫描断言覆盖 `crates/*/src`
    - 5.1: 新判据文件（放 `crates/preset/tests/` 下），扫 `crates/preset/src/` 与
      `crates/postprocess/src/`，**递归**（preset 有 `src/bin/`）
    - 5.2: 路径一律用 `CARGO_MANIFEST_DIR` + 显式相对路径，**不用 `file!()`** ——
      理由照抄 `upstream/mod.rs:108` 那段（workspace 之后基准会变，判据会以"报错"形式失效）
    - 5.3: 只看生产部分：`#[cfg(test)]` 之前、去掉 `//` 开头的行（与既有那条同口径）
    - 5.4: 白名单逐条写理由（就是 Task 2/3 处置过的那 5 处），白名单之外命中即红
    - 5.5: 反空转：扫到的文件数有下限断言；**探针**：往一个白名单外的文件临时加
      `std::fs::write` ⇒ 判据必须红且点名该文件；撤掉 ⇒ 绿
    - 5.6: 提交

- [ ] Task 6: 顺手修两处失效引用（不扩大范围）
    - 6.1: `crates/preset/src/generate.rs:12` 引用的 `scripts/check_generator_purity.py`
      本仓不存在 —— 改成指向本轮的扫描断言，或直接删掉那句（别留指向空气的门禁）
    - 6.2: `recipe_edit::set_override`（`recipe_edit.rs:129`）"字段不存在就新增一行"
      与 `write.rs` 的 K-P3「拒绝新增键」立场相反 —— **只在两处文档里互相点明**
      （配方是我们的资产、用户预设是用户的），**不动代码**
    - 6.3: 提交

- [ ] Task 7: 全量验收与收口
    - 7.1: 八条命令全跑（fmt / 两条 clippy / 三组测试 / `cargo tree -d` / npm）
    - 7.2: `presets/` 12 个文件 sha256 不变；`git status --short` 只剩有意改的文件
    - 7.3: 更新 `HANDOFF.md`：§5 的写盘纪律那条补"现在覆盖 `crates/*`"、
      §0 基线行、§8 命令表；`clippy.toml` 位置变了要写进去
    - 7.4: 更新长期 `tasks.md` 的 Task 19：标注"提前做了哪一小块、还欠什么"
    - 7.5: 写 `summary.md`（提交号、每处处置的理由与退役条件、两次探针的结果）
    - 7.6: 推分支、看 PR #11 三个 job —— **推之前问一次**
