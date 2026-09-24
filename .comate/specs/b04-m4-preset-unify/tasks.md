# Task 15 / M4 施工计划 —— 四笔提交把 preset crate 迁进来、注册表收成一份

顺序不可换：**先拿零 diff 证据（Task 2）→ 再改名（Task 3）→ 再改行为（Task 4/5）**。
每个顶层任务 = 一笔提交，提交前本地必须全绿。判据数字见 `doc.md §1 / §7`。

- [x] Task 1: 基线核准（搬之前把"期望值"全部定下来）
    - 1.1: 核 `git status -sb` 工作区干净、HEAD = `e46c53b`、领先 main 10 笔
    - 1.2: 跑三组基线并抄下数字：默认 `cargo test`、`-p mkp-support-ease --features workbench`（264）、`-p mkpse-postprocess`（249 + 2 ignored）
    - 1.3: 抄下 `cargo tree -d` 基线：58 条 / 26 个 crate 名
    - 1.4: 生成 `presets/` 12 个文件的 sha256 清单，存成临时文件（全程比对用，验收后删）
    - 1.5: 生成源树 `g:\project\mkp-ssr\crates\preset` 35 个文件的 sha256 清单 —— 这是 Task 2 的期望值
    - 1.6: 确认 `mkp-ssr` 侧无未提交改动会影响 preset 目录（只读源，不碰）

- [x] Task 2: M4a —— 原样搬入 `crates/preset`，**不进 members**
    - 2.1: 逐字节复制 35 个文件到 `g:\project\MKPSupportEase\crates\preset\`（含 `assets/` 13 份 toml 与 `tests/fixtures/*.snapshot`）
    - 2.2: 根 `Cargo.toml` 加 `exclude = ["crates/preset"]`，**members 不动**
    - 2.3: 判据一：35 个文件逐文件 sha256 与 1.5 的清单一致（只源有 0 / 只目标有 0 / 内容不同 0）
    - 2.4: 判据二：`git add` 之后核「索引 blob == 工作区内容」35/35（防 `eol=lf` 归一化偷改 snapshot）
    - 2.5: 判据三：三组基线数字与 1.2 **一个不变**（它没参与编译）；`cargo fmt --check` 不该看它
    - 2.6: 提交。提交信息必须写明：`preset_recipes.toml` / `test_recipes.toml` 是**第二真源的载体**，本轮只当夹具搬入，真源归属未定案

- [x] Task 3: M4b —— 机械改名 + 入 `members`，第一次编译它
    - 3.1: 根 `Cargo.toml`：`members` 加 `crates/preset`、删 `exclude`、把 `toml_edit = { version = "0.22", features = ["serde"] }` 提到 `[workspace.dependencies]`
    - 3.2: `src-tauri/Cargo.toml` 的 `toml_edit` 改成 `{ workspace = true, optional = true }`（保住 optional 隔离）
    - 3.3: `crates/preset/Cargo.toml`：包名 → `mkpse-preset`；加 `[lib] name = "preset"`；内部依赖 → `mkpse-postprocess = { path = "../postprocess" }`；`toml_edit.workspace = true`
    - 3.4: 按 `doc.md §3` 的 6 条有序规则替换全树：`mkp_pp::` → `postprocess::`、`../core/` → `../postprocess/`（15 处）、`mkp_preset::` → `preset::`
    - 3.5: **人工读一遍** `rg 'name = '` 在 `crates/preset/` 的全部命中点 —— M2b 那次的语义错误（规则命中 clap 的 `#[command(name)]`）只能这样抓
    - 3.6: `cargo fmt` 重排与改名**同属这一笔**（列宽变化的必然后果），在提交信息里点明文件数与行数
    - 3.7: 判据一：对只读源树重放同一条规则，逐文件与工作区比对，全部相同
    - 3.8: 判据二：`cargo test -p mkpse-preset` 全绿，记录 8 个测试文件的条数作为新基线
    - 3.9: 判据三：`cargo tree -d` 仍是 58 / 26（`toml_edit` 不许出现第二档）；三组老数字不变；`cargo clippy --all-targets -- -D warnings` 绿
    - 3.10: 提交

- [x] Task 4: M4c —— 注册表切到唯一真源
    - 4.1: `crates/preset/src/lib.rs:65` 的 `include_str!` 改指 `../../../presets/registry/param_registry.toml`
    - 4.2: `crates/preset/src/registry_edit.rs:28` 的写回目标改指 `../../presets/registry/param_registry.toml`
    - 4.3: 删 `crates/preset/assets/param_registry.toml`（56 KB 分岔副本，不留）
    - 4.4: 新判据 `crates/preset/tests/registry_is_the_only_source.rs`：直接读盘那份与 `PARAM_REGISTRY_TOML` 解析结果**逐条相等**（74 条 key + min/max/step/defaultValue/choices），防"改了常量但副本还在"
    - 4.5: 判据：`rg 'param_registry\.toml'` 全仓命中的**数据文件路径只有一个**
    - 4.6: 判据：8 个测试仍全绿，**且 `tests/fixtures/registry_branch_diff.snapshot` 未被修改**（`git status` 里不该出现它）
    - 4.7: 判据：`presets/` 12 个文件 sha256 与 1.4 一致 —— `registry_edit` 现在有能力写真源，这条是它的闸
    - 4.8: 提交

- [x] Task 5: M4d —— `BUILTIN_PRESETS` 改成按清单校验
    - 5.1: 新判据 `crates/preset/tests/builtin_presets_match_dir.rs`：读 `assets/presets/` 目录名集合与 `BUILTIN_PRESETS` 的 9 条比，**多一份、少一份都报错**
    - 5.2: 用探针验证这条判据会响：临时加/删一份，确认它红，然后还原（不留痕）
    - 5.3: 9 份 `assets/presets/*.toml` 保留不动（15.4：暂留当基线，Task 18 才删）
    - 5.4: 提交

- [x] Task 6: 全量验收与收口
    - 6.1: `doc.md §7` 的 8 条命令全跑一遍，逐条记下 `test result: ok. N passed`
    - 6.2: `rg 'mkp_pp|mkp_preset|mkp-preset'` 在 `crates/preset/` 下零命中（不留兼容别名）
    - 6.3: `git status --short` 只剩有意改的文件；`presets/` 12 份、内核 golden 全部未动
    - 6.4: 更新 `.comate/specs/b04-panel-successor/HANDOFF.md`：§0 的领先笔数与判据基线、§3 的 Task 15 状态、§4 的 M4 行与提交号、§7 删掉 Task 15 摘要、§8 补 `-p mkpse-preset`
    - 6.5: 更新长期 `.comate/specs/b04-panel-successor/tasks.md`：Task 15 的 `[ ]` → `[x]`，六条子项标 ✅ 并补实测数字
    - 6.6: 推分支、看 PR #11 三个 job（`web` / `rust` / `rust-windows`）—— **推之前问一次**
    - 6.7: 写 `summary.md`：四笔提交号、每笔的判据结果、M5 接手时欠的两件事（`machine_dims::install()` 接线 + 注册表改运行时读）
