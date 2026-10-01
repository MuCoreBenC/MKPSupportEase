# B04 Task 15 / M4 —— 预设解析迁进来，注册表合一

日期：2026-09-24　分支：`feat/b04-p3-migration`（领先 `main` 9 笔 + 1 笔本地 docs）

长期总纲仍然是 `.comate/specs/b04-panel-successor/tasks.md` 的 **Task 15**；
本目录只装这一轮的施工文档（`doc.md` / `tasks.md` / `summary.md`），
**不覆盖也不替代**那份 Task 1–22。冲突时优先级：`tasks.md` > 本文件。

---

## 0. 这一轮要达到什么

`tasks.md` Task 15 的六条原文（15.1–15.6）落地：

1. `mkp-ssr/crates/preset` → `crates/preset`，包名 `mkpse-preset` / lib `preset`
2. 注册表**只剩一份** —— 不搬那份 56 KB 的分岔副本，改读 `presets/registry/param_registry.toml`
3. 那 4 处分岔随之消失，并加判据咬住"只有一份"
4. `assets/presets/*.toml` 9 份暂留当基线（Task 18 删）
5. `BUILTIN_PRESETS` 手写表改成按清单校验，缺一份要报错
6. 它自带的 8 个测试文件全绿

---

## 1. 现状核准（全部实测，不引用转述）

| 项 | 实测值 | 怎么测的 |
|---|---|---|
| 源 crate 规模 | **35 个文件** = 21 `.rs`（13 src + 8 tests）+ 13 `.toml` + 1 `.snapshot`；Rust **6 212 行** | `Get-ChildItem -Recurse` + `Measure-Object -Line` |
| 源 crate 包名 | `mkp-preset`，lib 名默认 `mkp_preset` | `crates/preset/Cargo.toml:16` |
| 唯一内部依赖 | `mkp-pp = { path = "../core" }` | 同上 `:25` |
| 两份注册表差异 | **16 增 / 4 删，共 5 处**（见 §2） | `git diff --no-index` |
| 我方注册表 | `presets/registry/param_registry.toml`，2 460 行 / 56 902 B；74 条 `[[params]]` / 8 `[[tabs]]` / 27 sections | `read_file` + 计数 |
| `../core/` 硬编码路径 | **15 处**（8 个 tests 共 12 处 + `src/build.rs` `src/generate.rs` `src/validate.rs` 各 1） | `grep '\.\./core/'` |
| 迁移源盘点口径 | 盘点文档写"13 + 8 文件、7 482 行"，**文件数吻合、行数与实测 6 212 不符** | 以实测为准，口径差异按下 |

> 盘点里那个 7 482 行大概是另一种统计口径（含 assets 或含 `.toml`）。
> **判据不依赖这个数**，所以不去追；但别把它当验收值用。

---

## 2. 两份 `param_registry.toml` 到底差什么（实测 diff）

| # | 参数 | 字段 | mkp-ssr 侧 | 我方（新） |
|---|---|---|---|---|
| 1 | `toolhead.custom_mount_gcode` | `label` | 下笔 G-code | 装载胶箱 G-code |
| 2 | `toolhead.custom_unmount_gcode` | `label` | 收笔 G-code | 卸载胶箱 G-code |
| 3 | `wiping.disk_stagger_swing_mode` | `uiComponent` | `segmented` | `select` |
| 4 | `wiping.ironing_suppress_expand_mode` | `uiComponent` | `segmented` | `select` |
| 5 | `wiping.ironing_coverage_threshold` | `[[params.choices]]` | 无 | 多 3 条（0 / 50 / 90） |

**这 5 处都不影响 `build()` 与校验**，这一条是论证出来的，不是估计：

- `label` / `uiComponent` 在 `ParamEntry`（`src/registry.rs:47..116`）里只有 serde
  读写，没有任何分支读它们 ⇒ 只进 IPC 载荷与 UI 文案
- `choices` 的枚举白名单在 `src/validate.rs:264` 上有门：
  `if entry.value_type == "string" && !entry.choices.is_empty()`。
  而第 5 处那条参数 `valueType = 'float'`（`presets/registry/param_registry.toml:1060`）
  ⇒ **这一支进不去**
- `choices` 的另一处消费在 `src/build.rs:533`，条件是 `choice.deprecated == true`。
  我方新增那 3 条**都没写 `deprecated`** ⇒ 也不触发

**另核**：唯一的快照判据夹具 `tests/fixtures/registry_branch_diff.snapshot` 全文只有
9 行「`=== A1: 0 处`」这种行，**不含 label / uiComponent** ⇒ 切数据源后
**不需要动任何 golden 或 snapshot**。这一条很重要：迁移期禁止 `UPDATE_GOLDEN=1`，
如果切源会动快照，这一批就必须换设计。

---

## 3. 技术路线：拆四笔，每笔只动一个不变量

沿用 M2a / M2b 验证过的形状（**先拿零 diff 证据，再改名，再改行为**）。
M4 比 M2 多一个麻烦：preset **依赖内核**，而内核在我们这边已经改名成
`postprocess`，所以"原样搬"的那一笔在 workspace 里是**编译不过**的。

### M4a —— 原样搬入，但**先不进 `members`**

- `crates/preset/` 全 35 个文件按源逐字节复制，**Cargo.toml 也不改**
  （仍写着 `mkp-pp = { path = "../core" }`）
- 根 `Cargo.toml` 加 `exclude = ["crates/preset"]`，**不加进 `members`**
- **判据**：35 个文件逐文件 sha256 与源一致；另核「索引 blob == 工作区内容」
  （防 `.gitattributes` 的 `eol=lf` 偷改 `.snapshot` 与 golden）；
  默认 18 / workbench 264 / 内核 249 三组数字**一个不变**（它根本没参与编译）

> 为什么用 `exclude` 而不是"先改依赖再入 members"：改依赖就破坏了零 diff，
> 而零 diff 是这一笔**唯一**的证据。`exclude` 让这笔提交在 CI 上是绿的，
> 代价只是"库里多一个暂时不编译的目录"，且下一笔立刻消除。

### M4b —— 机械改名 + 入 `members`，**这一笔第一次编译它**

一条有序替换规则，作用于 `crates/preset/` 全树：

| # | 旧 | 新 | 命中面 |
|---|---|---|---|
| 1 | `name = "mkp-preset"` | `name = "mkpse-preset"` | 只 `Cargo.toml`（**必须限定文件**，见下） |
| 2 | `mkp-pp = { path = "../core" }` | `mkpse-postprocess = { path = "../postprocess" }` | `Cargo.toml` |
| 3 | `mkp_pp::` | `postprocess::` | src + tests |
| 4 | `../core/tests/` / `../core/` | `../postprocess/...` | **15 处**路径字面量 |
| 5 | `mkp_preset::` | `preset::` | tests（src 内部用 `crate::`） |
| 6 | `toml_edit = "0.22"` | `toml_edit.workspace = true` | `Cargo.toml` + 根提 workspace 依赖 |

加 `[lib] name = "preset"`（包名带连字符时 lib 默认叫 `mkpse_preset`，
而 15.1 要的是 `preset::`）。

**M2b 的教训要照搬**：那次规则 1 本该只作用于 `Cargo.toml`，却命中了
`src/main.rs` 里 clap 的 `#[command(name = ...)]`，而"逐文件重放"这条判据
**原理上抓不到替换规则的语义错误**（它只证明"新内容 = 源内容经同一规则"）。
所以这一笔的验收里必须有**人读一遍 `name =` 的全部命中点**。

这一笔预期会带 rustfmt 重排：`mkp_pp::`（7 列）→ `postprocess::`（12 列）
会把一些行推过 100 列，与 M2b 同一原因，**重排必须和改名同属一笔**。

- **判据**：对只读源树重放同一规则再逐文件比对；`cargo test -p mkpse-preset`
  **8 个测试文件全绿**；`cargo tree -d` 的重复条数与基线 **58 条 / 26 个名字**一致
  （`toml_edit` 提到 workspace 之后不许出现第二档）

### M4c —— 注册表切源：**唯一真源**

改两个常量的指向，不动解析逻辑：

```rust
// crates/preset/src/lib.rs:65  —— 原来指 ../assets/param_registry.toml
pub const PARAM_REGISTRY_TOML: &str =
    include_str!("../../../presets/registry/param_registry.toml");
```

```rust
// crates/preset/src/registry_edit.rs:28  —— 写回目标（开发态改区间用）
fn registry_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/registry/param_registry.toml")
}
```

然后**删** `crates/preset/assets/param_registry.toml`（那份 56 KB 副本）。

- **为什么仍用 `include_str!` 而不是像 M3 那样改成运行时注入**：
  `load_param_registry()` 有 20+ 处调用点（`src/lib.rs:167`、`src/validate.rs` 7 处、
  `src/build.rs` 2 处、4 个测试文件），改注入等于把"搬运"变成"重构"。
  M3 的 `machine_dims::install()` 已经把数据根注入的机制建好了，
  **注册表跟着走注入这件事挂到 M5**（和 M3 欠的那次 `install()` 接线同一笔做）。
  这一笔只解决「两份数据」，不解决「数据编进二进制」。
- **判据**：
  1. 全仓 `rg 'param_registry\.toml'` 命中的**文件路径只有一个**
  2. 新判据 `registry_is_the_only_source.rs`：从 `presets/registry/` 读进来的
     74 条 key 与 `PARAM_REGISTRY_TOML` 解析结果**逐条相等**（防"改了常量但没删副本"）
  3. 8 个测试仍然全绿，**且 `registry_branch_diff.snapshot` 未被修改**（§2 论证的那条）

### M4d —— `BUILTIN_PRESETS` 改成按清单校验

现状：`src/lib.rs:76..113` 手写 9 条 `include_str!`，注释自己写着
「这张表是手写的，漏一份不会报错」。

改法：保留 `include_str!`（路径必须字面量，这是语言限制），
但加一条**编译期之外的判据**——测试里读 `assets/presets/` 目录，
与 `BUILTIN_PRESETS` 的文件名集合比：**多一份、少一份都报错**。

9 份 `assets/presets/*.toml` 按 15.4 **暂留当基线**，Task 18 再删。

---

## 4. 受影响文件

| 类型 | 路径 | 说明 |
|---|---|---|
| 新增（复制） | `g:\project\MKPSupportEase\crates\preset\**`（35 个文件） | M4a 逐字节；M4b 按规则改名 |
| 改 | `g:\project\MKPSupportEase\Cargo.toml` | M4a 加 `exclude`；M4b 改 `members` + 提 `toml_edit` |
| 改 | `crates\preset\Cargo.toml` | 包名 / lib 名 / 内部依赖 / `toml_edit` |
| 改 | `crates\preset\src\lib.rs:65` | 注册表 `include_str!` 指向我方真源 |
| 改 | `crates\preset\src\registry_edit.rs:28` | 写回目标指向我方真源 |
| 删 | `crates\preset\assets\param_registry.toml` | 分岔副本，不留 |
| 新增 | `crates\preset\tests\registry_is_the_only_source.rs` | "只有一份"的判据 |
| 新增 | `crates\preset\tests\builtin_presets_match_dir.rs` | 15.5 的清单校验 |
| **不动** | `presets\**`（12 个文件） | sha256 全程不变 |
| **不动** | `src-tauri\**` | 本轮不接线（M5 才接） |

`preset_recipes.toml`（24.6 KB）与 `test_recipes.toml`（26 KB）**必须跟着搬**：
`tests/recipe.rs:16` 与 `src/bin/gen_presets.rs:30` 用 `include_str!` 吃它们，
不搬则 8 个测试不可能编过。但要在 M4a 的提交信息里写明：
**它们是「第二真源」的载体**（`MIGRATION-PLAN.md §4.1`），这一轮只当夹具搬进来，
**真源归属仍未定案**，定案在 Task 18 之前必须做。

---

## 5. 边界与异常

- **默认构建的编译单元变多**：preset 的 `toml_edit` 不是 optional，
  于是默认 `cargo test` 也会编它。我方 `src-tauri` 那三个 optional
  （`sha2` / `time` / `toml_edit`）的**隔离仍然成立** —— 隔离靠的是
  "默认构建里 `use toml_edit` 编不过"，不是整个依赖图里没有它
- **edition / MSRV 不统一是既有状态**：preset 与 postprocess 领
  `workspace.package` 的 2024 / 1.97，`src-tauri` 是 2021 / 1.77.2。
  混用在 workspace 里合法，M2a 已经这样跑过
- **`file!()` 基准**：源 crate 里没有 `file!()`（已 grep 确认），
  路径全走 `CARGO_MANIFEST_DIR` ⇒ 不会踩 M2b 那个坑
- **`registry_edit` 现在会写到真源**：它是开发态工具（模块头自己写着
  "只在开发态有意义"）。切源后它的写盘目标变成 `presets/registry/param_registry.toml`
  ⇒ **它有能力污染真数据**。本轮不给它加闸，但验收时 `git status --short`
  必须证明 `presets/` 一个字节没变
- **`gen-presets` bin 的输出目标**是 `assets/presets/`（`src/generate.rs:44`），
  搬进来后仍指 `crates/preset/assets/presets/` —— 与我方 `dist-presets/` 并存。
  本轮**不动它**，换源是 M5/Task 18 的事

---

## 6. 数据流（M4 完成后）

```
presets/registry/param_registry.toml   ← 唯一真源（2 460 行 / 74 条 params）
        │
        ├── include_str! ──→ preset::PARAM_REGISTRY_TOML ──→ load_param_registry()
        │                        └─→ validate_against_registry / build 的零值兜底与区间
        │
        └── 运行时读盘 ──→ src-tauri workbench::presets::ParamRegistry::load_from()
                              └─→ wb_registry / param_view / 写回（toml_edit 保真）

presets/machines/*.toml + forbidden_zones/*.toml
        └── M3 的 machine_dims::install / load_presets_dir ──→ postprocess
```

两条读取路径、**一份数据**。两个解析器（`preset::Registry` 与
`workbench::ParamRegistry`）在本轮**刻意保留** —— 合并解析器是重构，
不在 Task 15 范围，硬塞进来会毁掉"纯移动"的证据。

---

## 7. 预期结果与验证命令

```powershell
cd G:\project\MKPSupportEase
cargo test                                            # 默认：三个成员
cargo test -p mkp-support-ease --features workbench   # 264
cargo test -p mkpse-postprocess                       # 249 + 2 ignored
cargo test -p mkpse-preset                            # 8 个测试文件，本轮新基线
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets -p mkp-support-ease --features workbench -- -D warnings
cargo tree -d                                         # 基线从 58/26 变成 61/27，见下

npm run lint; npm run build
```

- 判成败看 `test result: ok. N passed`，不看管道退出码（PowerShell 会吞）
- `git status --short` 必须只剩有意改的文件；`presets/` 12 个文件 sha256 不变
- `rg 'mkp_pp|mkp_preset|mkp-preset'` 在 `crates/preset/` 下**零命中**（不留兼容别名）

### `cargo tree -d` 的新基线：**61 条 / 27 个名字**（M4b 实测更正）

原先写的 58 / 26 是 M4a 那个状态（preset 还在 `exclude` 里、不参与编译）。
入 `members` 之后新增 3 条，逐条能追到决定：

| 新增条目 | 谁拉的 | 追到哪条决定 |
|---|---|---|
| `toml_edit v0.20.2` | `toml 0.8.2` 的内部依赖（postprocess + preset） | M1 定的「`toml` 取 0.8」 |
| `toml_edit v0.22.27` | preset 的显式依赖 | 写回保真（保注释 / 保键序）只有它能做，是 K-P1/P2/P3 的唯一实现 |
| `winnow v0.7.15` | 两支：`toml 0.9.12`（← tauri-build，基线就有）+ `toml_edit 0.22.27`（← preset） | 同上 |

基线里这三个名字各只有一档，所以不算重复；是这两条既有决定叠加的必然结果，
**不推翻其中一条就消不掉**。`Cargo.lock` 只多了 `mkpse-preset` 一个包条目，
没有引入任何新的第三方版本。


---

## 8. 不在本轮范围

- 合并两个注册表解析器 / 两套数据模型（`ParamEntry` vs `ParamDef`）
- 注册表改成运行时读（跟 M3 欠的 `machine_dims::install()` 接线一起，M5）
- `src-tauri` 调用 `preset::load_ir()`（M5 / Task 16）
- `preset_recipes.toml` 的真源归属定案（Task 18 之前必须做）
- 删 `assets/presets/*.toml` 9 份基线（Task 18）
- M0 欠的两条收尾（精确领头键清单、`eprintln!` → `assert!`，`tasks.md` 11.5/11.6）——
  它们的前提是 `crates/preset` 已进来，本轮结束即解锁，但不在本轮做

---

## 9. 需要你点头的三处

1. **M4a 用 `exclude` 让"零 diff 那一笔"保持 CI 绿** —— 代价是库里短暂存在一个
   不参与编译的目录。替代方案是允许一笔红的提交，我不建议
2. **M4c 仍用 `include_str!` 指真源**，"注册表运行时读"推到 M5。
   理由是 20+ 调用点，改注入会把搬运变重构
3. **两个解析器本轮不合并**。15.2/15.3 的"合一"我理解为**数据只有一份**，
   不是"代码只有一个解析器" —— 如果你要的是后者，M4 的范围要整体重估
