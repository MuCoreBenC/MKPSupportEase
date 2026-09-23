# B04 P3 施工图：把消费端迁进来，只留一个项目

定案（2026-09-23，你拍的）：**不再有 mkp-ssr。** 后处理内核与预设解析迁进
`MKPSupportEase`，名字全部改成我们自己的，代码里不留"上游 / 消费端 / 引用别人"的措辞。
前端不迁 —— 要的是后处理那一段，它本来就是独立的。

证据基线：两仓 commit 固定 `8bf3fcb`（我们）/ `a2a47ca`（mkp-ssr），
审计结论见 [AUDIT-EVIDENCE.md](./AUDIT-EVIDENCE.md)，契约见 [DATA-CONTRACT.md](./DATA-CONTRACT.md)。

---

## 1. 要迁多少（实测）

| 来源 | src | tests | assets |
|---|---|---|---|
| `crates/core`（后处理内核） | 54 文件 / **21 217 行** | 20 文件 / 4 050 行 | 2 个 JSON |
| `crates/preset`（TOML → IR） | 13 文件 / **5 799 行** | 8 文件 / 1 683 行 | 12 个 |
| 合计 | **67 文件 / 27 016 行** | 28 文件 / 5 733 行 | 14 个 |

**95 个文件、32 749 行。** 这不是一次提交能完成的事，也不该试 ——
分段的判据是「每段结束时 `cargo test` 全绿」，不是「今天搬完」。

好消息（审计已证）：依赖方向本来就单向且干净 ——
`core` **不依赖任何本地 crate**，只用 9 个第三方；`preset → core`。
所以"容易提取"这个判断是对的，不是乐观估计。

## 2. 目标布局与命名

现在 `MKPSupportEase` 是单 crate（`src-tauri`）。迁入后变成 workspace：

```
MKPSupportEase/
  Cargo.toml              ← 新建：workspace 根，统一依赖版本
  crates/
    postprocess/          ← 原 mkp-ssr/crates/core
    preset/               ← 原 mkp-ssr/crates/preset
  src-tauri/              ← 现在这个，改成 workspace 成员
  presets/                ← 唯一真源（不动）
  dist-presets/           ← 产物（不动）
```

重命名表（**一次改完，不留过渡别名**）：

| 旧 | 新 | 说明 |
|---|---|---|
| 包 `mkp-pp` / lib `mkp_pp` | 包 `mkpse-postprocess` / lib `postprocess` | 代码里 `mkp_pp::` → `postprocess::` |
| 包 `mkp-preset` / lib 同名 | 包 `mkpse-preset` / lib `preset` | `mkp_preset::` → `preset::` |
| 包 `mkp-ssr` | **不迁** | 它是那边的 Tauri 应用层；我们已经有自己的 |
| bin `mkp-pp` | bin `mkpse-pp` | 命令行入口，保留（调试后处理用得上） |
| bin `gen-presets` | bin `gen-presets` | 保留，但输入要换源（见 §4） |
| 文档措辞「消费端」「上游」 | 「后处理」「预设解析」 | 只有一个项目了，这两个词失去了指代对象 |

`src-tauri` 刻意不领 workspace lints（它那边也是这么做的，`unsafe_code = "warn"` 而非
`forbid`，因为 Tauri 的宏会碰到）。这一条照抄。

## 3. 第三方依赖（要进 workspace 根）

`core`：`thiserror` `tracing` `tracing-subscriber` `serde` `serde_json` `toml`
`sha2` `clap` `ctrlc`（dev：`tempfile`）
`preset`：上面几个 + `toml_edit 0.22`（dev：`serde_json` `tempfile`）

我们现在已经有 `serde` `serde_json` `toml_edit` `sha2` `thiserror` `tracing` `tempfile` `toml`。
**新增的只有 `tracing-subscriber` `clap` `ctrlc`** —— 都是给那个命令行 bin 用的。
版本统一到 workspace 根，两边取较新的一档，改完 `cargo tree -d` 不许有重复。

## 4. assets 逐份处置（这一节是整个迁移的重点）

| 文件 | 大小 | 处置 | 理由 |
|---|---|---:|---|
| `preset/assets/param_registry.toml` | 56 KB | **不搬，改读我们的** | 它是我们 `presets/registry/param_registry.toml` 的分岔副本（74 条 key 相同，4 处实质差异）。搬过来就是把双真相固化 |
| `core/assets/machine_dimensions.json` | 3.6 KB | **不搬，改从 presets 读** | 已验证与我们 5 台机型的 `[dimensions]` **125/125 字段全等** |
| `core/assets/machine_catalog_extra.json` | 2 KB | **不搬，改从 presets 读** | 别名 23 条双向全覆盖、禁区点集逐点相同 |
| `preset/assets/presets/*.toml` | 9 × 4.3 KB | **不搬** | 那是产物，我们自己生成（`dist-presets/presets/mkp/`）。可暂留一份当对照基线，迁完就删 |
| `preset/assets/preset_recipes.toml` | **24.6 KB** | ⚠️ **必须定案，见下** | |
| `preset/assets/test_recipes.toml` | 26 KB | 跟着上一条 | 测试夹具 |

### 4.1 `preset_recipes.toml`：**这是第二个真源**

文件头自己写着：「**这一份是真源**（spec `recipe-workbench` §0）」，
「一台机型一份正文，没有共享的模板」，覆盖表**只有变体级**（键 `机型:变体`），
`release_time` 写死在里面，生成物是 `assets/presets/`（`gen-presets --write`）。

也就是说那 9 份内置预设不是手写的，是**从这份 recipes 生成的**。
于是两边各有一套真源：

| | 我们 | mkp-ssr |
|---|---|---|
| 清单 | `presets/machines/*.toml` | recipes 的 `[[machines]]` |
| 参数值 | `param_registry.toml` 的 `machineVariants` | recipes 的机型正文 + 变体覆盖 |
| 产物 | `wb_generate` → `dist-presets/` | `gen-presets --write` → `assets/presets/` |

**两条链路做的是同一件事。** 这比注册表那处分岔更根本 —— 注册表只是字段定义，
recipes 是值。迁入前必须先回答：

- [ ] recipes 的机型正文与我们的「出厂默认 + machineVariants」**逐字段是否等值**？
      （方法：两边各生成一次 `A1-standard.toml`，比正文字节 —— 头部时间那行除外）
- [ ] 不等值的项，哪边是对的？（它的 9 份产物是真机验证过的，我们的没有）
- [ ] 定案后：要么 recipes 的内容并进我们的 `presets/`，要么承认 recipes 才是值的真源
      而我们的 `machineVariants` 是派生的 —— **不许两边都留着能改**

**这是下一轮第一件事，在搬任何代码之前。** 搬完代码再发现值对不上，
就会有一批"生成出来但和真机验证过的那份不一样"的产物，而那种错在产物上看不出来。

## 5. 分段顺序（每段一个提交，每段结束必须全绿）

| 段 | 内容 | 验收 |
|---|---|---|
| **M0** | 比对 recipes 与我们的值（§4.1），定案写进 DATA-CONTRACT | 两边各生成一份 `A1-standard.toml`，差异逐项有结论 |
| **M1** | 建 workspace 根 + `src-tauri` 变成成员 | `cargo test --features workbench` 仍 274 绿 |
| **M2** | `core` → `crates/postprocess`，**纯移动 + 改包名**，不动逻辑 | 它自带的 20 个测试文件全绿 |
| **M3** | 尺寸/别名/禁区改从 `presets/` 读（删掉那两个 JSON asset） | 125 字段等值那条判据从"跨仓比对"变成"同仓单一来源" |
| **M4** | `preset` → `crates/preset`，改包名；`param_registry` 改读我们那份 | 它的 8 个测试文件全绿；注册表分岔那 4 处消失（只有一份了） |
| **M5** | 生成器接上：`wb_generate` 产物经 `load_ir()` 复检 | **P2 那条纵向切片**：改一个值 → 生成 → `load_ir` 读通 → IR 里是新值 |
| **M6** | 删 `upstream/` 整层 + 源码扫描断言（不许出现 `mkpse-presets` / `content/`） | 两种 feature 的 test + clippy + 前端 lint/build |
| **M7** | 措辞清场：文档与注释里的「上游」「消费端」换掉 | 全仓搜不到把我们自己说成"下游"的地方 |

M2 与 M4 是**纯移动**：一个字都不重写。理由是那 27 000 行里有大量真机验证过的行为，
顺手"改好一点"等于把验证过的东西变成没验证过的。要改在 M3/M5 单独改，单独验。

## 6. 红线

- **不在 mkp-ssr 里写任何东西。** 它现在有几个未提交改动（`docs/` `make.bat` `scripts/`，
  与 preset 链路无关）—— 只读，不碰。迁移是复制过来改，不是双向同步。
- **不改真数据**：`presets/` 的 12 个文件在整个迁移里保持 sha256 不变，
  除了 M0 定案后可能要并入 recipes 的值 —— 那一步单独提交、逐文件审。
- **不留兼容层**：不做 `pub use mkp_pp as postprocess` 这种过渡别名。
  一次改完，`rg mkp_pp` 必须零命中。
- **不提前删 mkp-ssr 的目录**：迁完并且全绿之后，它还要留着当对照基线，
  直到 M5 的纵向切片过了。删不删那个仓库由你决定，代码里不再引用它就够了。

## 7. 现在的状态

- P0 审计：✅ 完成（AUDIT-EVIDENCE.md）
- P1 契约：✅ 1.0，已核实的部分冻结（DATA-CONTRACT.md）
- P2 纵向切片：⏸ 并进 M5（迁进来之后做，比跨仓跑更干净）
- P3 迁入：**这份文件就是它的施工图，从 M0 开始**
