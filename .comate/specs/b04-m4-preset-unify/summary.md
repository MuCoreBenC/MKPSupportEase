# Task 15 / M4 收尾总结 —— 预设解析迁进来，注册表收成一份

日期：2026-09-24　分支：`feat/b04-p3-migration`（领先 `main` 15 笔）
长期总纲：`.comate/specs/b04-panel-successor/tasks.md` 的 Task 15（已勾上并补实测数字）

---

## 1. 五笔提交

| 提交 | 内容 | 规模 |
|---|---|---|
| `e87b90f` | **M4a** 原样搬入 `crates/preset`，`exclude` 不进 `members` | 36 文件 / +12 156 |
| `168a2b6` | **M4b** 机械改名 + 入 `members` + `toml_edit` 提到 workspace | 22 文件 / +136 −120 |
| `a3ae1cf` | **M4c** 注册表切到唯一真源，删 56 KB 分岔副本 | 4 文件 / +113 −2 452 |
| `f7c38f3` | **M4d** `BUILTIN_PRESETS` 按清单校验（3 条判据） | 2 文件 / +89 −1 |
| `554f601` | 补 M4c 漏掉的一行 rustfmt | 1 文件 / +1 −2 |

拆法照 M2a/M2b 验证过的形状：**先拿零 diff 证据 → 再改名 → 再改行为**。

---

## 2. 每一笔的判据结果（数字是证据）

**M4a**：35 个文件逐文件 sha256 与源一致（只源有 0 / 只目标有 0 / 内容不同 0）；
`git add` 后核「索引 blob == 工作区内容」**35/35**（`.snapshot` 没被 `eol=lf` 改写）；
三组老数字一个不变 —— 它根本没参与编译。

> 这一笔用 `exclude` 而不是"顺手改依赖"：preset 的 `Cargo.toml` 里写着
> `mkp-pp = { path = "../core" }`，在我们这边编译不过。改它就毁了零 diff 这唯一证据。
> 代价是库里短暂存在一个不参与编译的目录，下一笔立刻消除。

**M4b**：对**只读源树**重放同一条规则再跑 rustfmt，逐文件比对 ⇒ **32/35 完全相同、
零处意外差异**。3 处例外是事先声明的手改，实测 diff 精确到 3 行。
另外人工读了 `name =` 的全部 48 处命中 —— 因为"逐文件重放"**原理上抓不到替换规则
本身的语义错误**（M2b 踩过：规则命中了 clap 的 `#[command(name)]`）。

**M4c**：`cargo test -p mkpse-preset` 106 条全绿，**切数据源后老判据一条没红**，
`registry_branch_diff` 的快照也没动 ⇒ **本轮没有动任何 golden**（迁移期禁止
`UPDATE_GOLDEN=1`，这条很重要）。

**M4d**：**用探针验过判据会响** —— 往 `assets/presets/` 塞一份 `ZZZ-probe.toml`
⇒ `FAILED. 2 passed; 1 failed` 且报错点名那个文件；撤掉 ⇒ `ok. 3 passed`，无残留。

**全量验收**（仓库根）：`fmt --check` 绿、两条 clippy `-D warnings` 绿、
默认 18 + 内核 249+2 + 预设 106、workbench 264、`npm run lint` 与 `build` 绿（962 ms）。
`presets/` 12 个文件全程 sha256 不变；`crates/preset` 下 `mkp_pp` / `mkp_preset` /
`mkp-preset` 零命中。

---

## 3. 三处与计划不符、按实测改了口径的地方

1. **分岔是 5 处，不是 4 处**：`uiComponent` 有**两处**（`disk_stagger_swing_mode`
   与 `ironing_suppress_expand_mode`），原计划写的是一处
2. **`cargo tree -d` 基线 58/26 → 61/27**，不是"不许新增"。新增 3 条
   （`toml_edit` 0.20.2 + 0.22.27、`winnow` 0.7.15）是「`toml` 取 0.8」与
   「写回必须用 toml_edit 0.22」两条既有决定叠加的必然结果，用临时 worktree
   回到 M4a 那个提交实测对比出来的。`Cargo.lock` 只多了一个包条目
3. **`preset_recipes.toml` / `test_recipes.toml` 必须跟着搬**（原计划没写）：
   `tests/recipe.rs` 与 `gen-presets` 用 `include_str!` 吃它们，不搬则测试编不过。
   它们是「第二真源」的载体，本轮只当夹具搬入，**真源归属仍未定案**

---

## 4. 那 5 处差异为什么不会让判据变红（这是设计的依据，不是事后解释）

- `label` / `uiComponent` 在 `ParamEntry` 里只有 serde 读写，没有任何分支读
- `choices` 的白名单在 `validate.rs:264` 有 `value_type == "string"` 这道门，
  而那条参数是 `float`（真源第 1060 行）
- `build.rs:533` 那处 deprecated 检查要求 `choice.deprecated == true`，新增 3 条都没写

**正因为静默，才必须专门加一条判据。** `registry_is_the_only_source.rs` 比的是
**全文字节**而不是逐字段 —— 逐字段只覆盖 `ParamEntry` 认识的键，`jsonKey` /
`mergeGroup` / `showWhen` / `serialization` 这些它不读的字段漂了会看不见。

---

## 5. 留给 M5 的两件事（都写进了 HANDOFF §7）

1. **注册表仍是 `include_str!` 编进二进制** —— 数据只有一份了，但发布物里改不了。
   改成运行时从数据根读，与 M3 欠的 `machine_dims::install()` 接线**是同一件事的两半**，
   一起做。本轮没做的理由：`load_param_registry()` 有 20 多处调用点，混进搬运这一批
   会把它变成重构
2. **`registry_edit` 的写盘目标现在是真源** —— 它从此**有能力污染 `presets/`**。
   暂时靠"12 个文件 sha256 不变"那条验收兜着，**没有代码级的闸**

另外 M0 欠的两条收尾（精确领头键清单、`eprintln!` → `assert!`，长期 `tasks.md`
的 11.5/11.6）前提是 `crates/preset` 已进来 —— **现在解锁了**，但不在本轮做。

---

## 6. 状态与下一步

- 五笔**只在本地**，尚未推送；推了会在草稿 PR #11 上触发三个 job
- `presets/` 未动、没有动任何 golden 或 snapshot、没有引入新的第三方依赖版本
- 下一步按长期 `tasks.md` 是 **Task 16 / M5**（纵向切片 + 上面那两条接线）
