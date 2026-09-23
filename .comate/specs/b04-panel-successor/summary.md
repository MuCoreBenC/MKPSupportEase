# B04 —— 本轮小结（Task 1 / 2 / 3 完成，地基立住了）

12 条任务做完 3 条。**这一轮的产出不是功能，是把方向定死 + 立住地基判据。**
没做的部分逐条标在 tasks.md 里，不含"其实差不多了"的说法。

---

## 1. 方向定了，代价是两次推翻

| 版本 | 假设 | 怎么塌的 |
|---|---|---|
| 第一版 | `content/*.json` 就是源 | **实测 `source/` 存在且完整**，`source/machines/A1.toml` 与 `content/machine_catalog.json` 逐字段对应 ⇒ content 是产物 |
| 第二版 | 产物必须与旧客户端字节一致 | 你说旧客户端也退役 ⇒ **没有现存消费者，不欠任何人字节级兼容** |
| 第三版（现在） | 只做工作台自己：编辑 `presets/*.toml` | —— |

第二版那条「往返保真」原本要去复刻 mkppanel 那个 Go 编码器的键序、缩进、转义。
**那是整件事里最贵也最脆的一块，它的依据没了。**

判据没丢，换了对象：**读进来不改再写出去，`presets/*.toml` 字节不变。**
它保护的是我们自己的数据 —— 否则第一次点保存就可能把六个机型文件的排版搅乱，
而那种损坏在 diff 里是一片红，真正改了什么反而看不出来。

## 2. 正面回答了你那两个质问

> **「加新机型在哪里加？」** —— 没有。`Patch` 那 12 个变体里没有 `NewMachine`。
> **「加新版本呢？」** —— 后端七个 patch 全在，**前端一个入口都没有**（b03 的 Task 12 一直排在后面）。

而且找到一处更糟的：因为"版本清单归上游"，`NewVersion` 新建的版本**保存后在树上看不见**，
我当时给它写了一条断言"必须有提示"就放过去了。**那不是功能，是一个有测试兜着的死胡同。**

现在这个死胡同有了出口：`presets/machines/A1.toml` 里就是 `[[versions]]` 列表，
**「加一个版本」就是往那个文件加一个块** —— 那就是之前答不出来的位置。

## 3. 真正落地的代码

```
MKPSupportEase/presets/                  ← 新建，数据搬进我们项目（12 个文件）
├── brands.toml / layout_schema.toml
├── machines/{A1,A1_MINI,A2L,P1S,P2S,X1C}.toml
├── forbidden_zones/{P1S,P2S,X1C}.toml
└── registry/param_registry.toml         56,902 B / 74 条字段

src-tauri/src/workbench/presets/         ← 新层，读它也写它
├── mod.rs        Presets::load / load_from + 共用的 read / parse_text
├── catalog.rs    机型 + 品牌 + 禁区
└── registry.rs   字段定义（74 条）+ 界面布局
```

`Cargo.toml` 加了一个依赖并写了理由：**`toml_edit` 而不是 `toml`** ——
后者要靠我复刻别人的 writer 才能保真，前者保留原文，**保真在构造上成立，不靠自觉**。
它挂在 `workbench` feature 的 optional 里，编译隔离纪律没破。

**模型是借来的，不是重写的**：`param_registry.toml` 的键名是 camelCase
（`tomlKey` / `defaultValue` / `machineFilter` / `sectionId`），与旧层读的那份 JSON
一模一样 —— 因为那份 JSON 就是从这个 TOML 构建出来的。所以 `ParamDef` / `TabMeta`
直接从 `upstream::registry` 借过来，**只换 loader**。
两份模型迟早分岔，一份模型两个 loader 不会。

## 4. 验证结果

| 项 | 结果 |
|---|---|
| `cargo test --features workbench` | **248 passed / 0 failed**（原 239，新增 9） |
| `cargo test`（默认 feature） | 18 passed / 0 failed |
| `cargo clippy --features workbench --all-targets -- -D warnings` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |

九条新测试**全部在真数据上跑**，其中四条是这一轮的核心：

- `reading_and_writing_back_changes_nothing` —— 6 个真机型文件读进来再写出去，**逐字节相同**
- `registry_and_layout_survive_a_round_trip` —— **56 KB 的 `param_registry.toml` 也逐字节相同**。
  它含多行 G-code 字符串与一堆浮点，**是保真最容易破的地方**；哪天有人换回
  "反序列化再重新序列化"，这条会第一个红
- `the_fidelity_judge_would_catch_a_real_edit` —— 反空转：改一个字段必须比出差异，
  且别的字段不许丢（否则上面两条在「`to_toml` 直接返回原文」这种假实现下也会绿）
- `a_multiline_gcode_value_is_preserved_verbatim` —— 专盯那条硬骨头

另外五条顺带验出两个真实不变量（都在真数据上通过）：
**布局引用的每个 `paramKey` 都真的存在**、**没有字段在布局里出现两次** ——
前者若破，界面上会排一个空位；后者若破，同一个值会有两个入口。

## 5. 两层并存是刻意的

新层 `workbench/presets/` 与旧层 `workbench/upstream/` 现在并存。
一次把 `Upstream` 换掉会让 `domain/` 与 `app/` 大面积编译失败，
那时新判据要和一大片编译错误同时处理，出问题分不清是哪边的锅。
**先让新层自己绿，再切换** —— 切换与删旧在 Task 4 与 Task 11。

## 6. 我做错的一件事，得说

读 `source/machines/A1.toml` 时，工具把 `mkpse-next-v3` 的 `AGENTS.md` 作为规则一并注入了。
我一直守的「不读那个仓库的源码与规则」这条纪律，**这一次实际上被破了**。

它的内容直接改变了判断，所以不能装作没看到（那个仓库里 `mkppanel` 读写 source、
`mkpsupporte` 只读 content、两者都已迁到 Wails v3）。
处置：`presets/` 下的文件已经搬进我们项目，**以后读数据不再碰那个目录**；
需要看那边时用 python 打印（已验证不触发注入），这条也记进了 memory。

## 7. 下一轮的第一步

按 tasks.md：**Task 4 —— 拆掉自造的「配方本」**，把 `Committed` 的来源从
`workbench/machines|versions/*.json` 换成 `presets/machines/*.toml`，
三层值解析改读 `[params.machineVariants]`。

这一步是**真正的切换**：它会让 `domain/` 与 `app/` 编译失败一片，然后逐个修回来。
地基已经立住了（五类源都读通、保真都绿），所以这次切换出问题时能判断是哪边的锅 ——
这正是上一轮刻意把两层并存的理由。

切换完之后才轮到界面（Task 7 换侧边栏壳、Task 8 机型与版本页），
也就是「加机型 / 加版本」真正出现在屏幕上的那一步。

## 8. 一个记下来的操作教训

改 `Cargo.toml` 的 `[features]` 和加依赖**必须在同一次编辑里完成**。
这一轮我先加了 `workbench = [..., "dep:toml_edit"]`、下一次编辑才补依赖行，
而 `tauri dev` 的文件监听正好在两次编辑之间触发，开了一次注定失败的构建
（`feature workbench includes dep:toml_edit, but toml_edit is not listed as a dependency`），
那个后台任务就此退出。不是真的坏了，但白烧一次构建。
