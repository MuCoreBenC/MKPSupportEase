# B04 工作台接管 mkppanel：侧边栏 + 一页一件事

## 0. 你问的那两个问题，先正面回答

> **「现在想加一个新机型，在哪里加？有位置给我加吗？」**

**没有。而且按现在的设计，结构上就没有这个位置。**
`Patch` 那 12 个变体里没有 `NewMachine` —— 机型清单被定成"上游的"，工作台不许改。

> **「那加一个新版本呢？」**

后端**全套都在**：`NewVersion` / `CloneVersion` / `RenameVersion` / `MoveVersion` /
`ArchiveVersion` / `RestoreVersion` / `PurgeVersion` 七个 patch，带撤销、带回收站、带两步确认。
**前端一个入口都没有** —— 那是 b03 tasks.md 的 Task 12（配方本树补齐），
我从头到尾排在后面，先去做矩阵好不好看、再去做配方台的形态。这是排序错了。

**更糟的是还有一处自相矛盾**，得说清楚：因为"版本清单归上游"，
`NewVersion` 新建出来的版本**保存之后在树上看不见** —— 只会多一条提示说
「上游没有 A1/NEW1 这一版」。我当时给这个半成品写了一条测试
（`saving_a_new_version_remaps_its_uid_in_the_next_view`）并断言"必须有提示"，
就把它放过去了。那不是功能，那是一个有测试兜着的死胡同。

你说的「连这种最基础的功能都没做好，就想着去改参数注册表」——是对的。

---

## 1. 前提变了：我们不是"下游"，我们就是那个编辑器

你这一句把地基换了：

> mkppanel 是 wails 的，也不是我们现在用的，它那个已经要废弃了。
> 我们现在就是要做一个完整的工作台去替代它的。

于是 b03 doc 里那几条铁律**一起作废**：

| 原来的铁律 | 现在 |
|---|---|
| 上游 `mkpse-presets` 只读 | **我们是它的编辑器**，该写就写 |
| 机型清单归上游，工作台不许改 | 加机型 / 加版本必须有位置 |
| 字段定义只读，改了要落到一层"覆盖" | 直接改 `param_registry.json`，不要覆盖层那种绕法 |
| 版本清单归上游，新建的版本只能留一条提示 | 新建的版本就该出现在清单里 |

覆盖层那个方案（b03-param-desk doc §5）**撤销**。它是"我们不敢碰上游"的产物，
前提没了就不该留 —— 多一层覆盖就多一处"值到底从哪来"的疑问。

> **§1 那个待定点已核实（Task 1，2026-09-23 实测）——结论是红灯，`content/` 确实是产物。**
>
> `mkpse-presets/source/` **就在那个仓库里**，而且是完整的一套 TOML 源：
> `source/machines/{A1,A1_MINI,A2L,P1S,P2S,X1C}.toml`、`source/registry/param_registry.toml`、
> `source/assets/`（18 个）、`source/bundles/`（5 个）、`source/forbidden_zones/`（3 个）、
> `source/layout_schema.toml`、`source/brands.toml`…
>
> 逐字比对 `source/machines/A1.toml` 与 `content/machine_catalog.json`：
> id / name / display / brand / defaultBundle / externalAliases / image / icon /
> `[dimensions]` 全族 / `[[versions]]` 三条**一一对应**。
> 也就是说**机型与版本清单的源是 `source/machines/*.toml`，`content/machine_catalog.json` 是它的产物**。
>
> 按 tasks.md 的 1.5：**停下来问，不自己选一条绕法。** 下面 §3 之后的设计在这一点定下来之前不动。
>
> 顺带必须说明一件我做错的事：读 `source/machines/A1.toml` 时，工具把那个仓库的 `AGENTS.md`
> 作为规则一并注入了 —— 我一直守的「不读那个仓库的源码/规则」这条纪律，在这一次实际上被破了。
> 它的内容直接改变了判断，所以不能装作没看到：
>
> - 那个仓库里**已经有两个 app**：`mkppanel`「读写 `source/*.toml` + 通过 Builder 生成 `content/*.json`」、
>   `mkpsupporte`「**只读** `content/*.json`」。
> - 两者**都已迁到 Wails v3**（2026-08-18 收口），不是"还在老 wails 上等着废弃"。
> - `mkpse-presets` 是 public 仓库，在那个工作区里是仓库根下的普通克隆。
>
> 于是 §1 原来那句「mkppanel 废弃之后 `source/` 应该一起废弃」**没有依据** ——
> `source/` 是数据的源，它跟哪个 app 活着没关系；真正要回答的是「Builder（source → content）归谁」。

## 1.1 定案（2026-09-23，五问已答）

| 问题 | 答案 |
|---|---|
| 工作台编辑哪一层 | **`source/*.toml`** —— 机型、版本、尺寸、禁区、参数定义都在这里 |
| 数据放哪 | **搬进我们项目**：`MKPSupportEase/presets/`。不去改 `mkpse-next-v3` 里那份克隆 |
| 自造的「配方本」怎么办 | **废弃**。`presets/machines/*.toml` 就是配方本，`workbench/machines\|versions/*.json` 删掉 |
| mkppanel | **完全不再使用**，不继承它的实现（我们是 Rust 重写），当前只搬**核心功能** |
| 旧客户端 `mkpsupporte` | **也退役**。`content/*.json` 没有现存消费者了 |
| 那 9 个 MKP 预设 | **不搬，是产物** —— 到时候由我们生成 |

### 已经作废的那条「第一原则」

上一版把「往返保真：生成的 `content/*.json` 必须与盘上那份逐字节相同」当地基，
理由是 `mkpsupporte` 还在只读它。**那个理由没了**，所以：

- **不做字节级复刻。** 去猜 mkppanel 那个 Go 编码器的键序、缩进、转义，是整件事里最贵也最脆的一块 —— 现在它没有依据了，直接删掉。
- **这一轮不决定产物格式。** 我们自己的客户端还没做，不猜它要读什么。
- **`content/*.json` 这一轮不生成。** 它是给旧客户端的聚合缓存，旧客户端退役了。

**但那条判据换个对象保留**：**读进来不改再写出去，`presets/*.toml` 必须字节不变。**
它便宜，而且保护的是我们自己的数据 —— 否则第一次点保存就可能把 6 个机型文件的格式搅乱，
而那种损坏在 diff 里是一片红、根本看不出真正改了什么。**这是往那个目录写第一个字节之前必须先绿的东西。**

### 1.2 已经搬进来的（实测，12 个文件）

```
MKPSupportEase/presets/
├── brands.toml                     80 B      1 个品牌
├── layout_schema.toml            8,446 B     字段的分组与排序
├── machines/                                 6 个机型 = 配方本
│   ├── A1.toml       1,276 B   3 版
│   ├── A1_MINI.toml  1,321 B   3 版
│   ├── A2L.toml        215 B   1 版（四折空：无尺寸、无禁区）
│   ├── P1S.toml        872 B
│   ├── P2S.toml        858 B
│   └── X1C.toml        889 B
├── forbidden_zones/                          3 个（P1S / P2S / X1C）
└── registry/param_registry.toml 56,902 B    74 条字段定义
```

**这一轮不搬**：`assets/`（18）、`bundles/`（5）、`preset_registry.toml`、
`release/`、`theme/`、以及 `about` / `faq` / `event` / `notification` / `model_copy` /
`application_defaults` / `fallback_registry`。

机型文件里的 `defaultBundle` / `recommendedBundle` / `presetFile` 会因此成为**悬空引用** ——
**这是刻意的**：它们只是字符串，编辑机型时照样显示照样改，只有到"生成"和"校验"那一步才需要解析。
在那之前把 bundle 那一摊搬过来，等于把「套餐」提前做了。

### 1.3 形状已经摸清（实测，不用再猜）

`param_registry.toml` 的 74 个 `[[params]]` 块，字段与我们 B03 那份 `ParamDef`
**几乎一一对应**：`key` / `label` / `desc` / `tomlKey` / `unit` / `valueType` /
`uiComponent` / `defaultValue` / `section` / `configKey` / `jsonKey` / `selectable` /
`mergeable` / `min` / `max` / `step` / `scope` / `tomlComment` / `machineFilter` /
`variantMode` / `parentKey`，加四个子表 `[params.layout]` / `[params.showWhen]` /
`[[params.choices]]` / `[params.machineVariants]`。

**也就是说 B03 的领域模型不用重写，换的是 loader** —— 从读 `content/*.json` 改成读 `presets/*.toml`。
这是这一轮最大的一条好消息，它把 Task 2 从"重建模型"缩成"换一层解析"。

## 2. 形态：侧边栏，一页一件事，互不交错

你要的两条纪律，逐字落进设计：

1. **不同界面之间的功能不要交错、不要相互影响**
2. **Bundle 管理与机型目录合成一个功能**

现在的顶部四页签（配方 / 对比 / 套餐与菜单 / 生成）+ 左下维护四项 = 8 个入口，
其中只有 1 个能用，而且「配方」里还塞了树、矩阵、批量、抽屉四件事。这就是"太多了"。

改成侧边栏，**一页 = 一个数据文件 = 一件事**：

```
┌──────────────┬───────────────────────────────────────┐
│ 内容          │  SOURCE …  ARTIFACT …  LAST BUILD …    │
│  参数注册表   │                                       │
│  机型与资源   │  （当前页的主区）                      │
│  应急规则     │                                       │
│              │                                       │
│ 交付          │                                       │
│  预设文件     │                                       │
│  构建         │                                       │
│              │                                       │
│ 系统          │                                       │
│  回收站       │                                       │
└──────────────┴───────────────────────────────────────┘
```

七页，每一页**只编辑一个文件**，这样"互不交错"是结构保证的，不是靠自律：

| 页 | 编辑哪个文件 | 一句话职责 |
|---|---|---|
| 参数注册表 | `content/param_registry.json` | 74 条字段的定义与每机型/每版本的值 |
| **机型与资源** | `content/machine_catalog.json` + `manifest.json` | 机型、版本、尺寸、禁区、以及这一版用哪些资源（**Bundle 合进来**） |
| 应急规则 | `content/fallback_registry.json` | 20 条回退规则，只读 + 启用开关 |
| 预设文件 | `presets/mkp/*.toml` + `presets/bbs/*.json` | 文件清单、内容预览、导入 |
| 构建 | 产出 `presets/mkp/*.toml` + `manifest.json` | Diff → 写入 → 完成 三步 |
| 回收站 | `.trash/` | 两步确认的彻底删除 |

**「对比」那一屏（矩阵）从导航里拿掉**，降成参数注册表页里的一个开关
（「横向比较多个版本」）。它是一个查看模式，不是一个页面 —— 这正是"功能交错"的典型：
它和参数页编辑的是同一份数据、同一套写入口。

### 2.1 Bundle 与机型目录怎么合

你说它们重复，对。看清楚就是同一件事的两半：

- **机型目录**：A1 → 3 个版本 → 每个版本有 `presetFile` / `推荐预设` / 最终资源（MKP + BBS）
- **Bundle 管理**：`A1_default` → 1 个 Asset（那个 BBS json）→ 引用关系：A1_MINI → 3 个版本

也就是 Bundle 是"从资源看有哪些版本在用"，机型目录是"从版本看用了哪些资源"。
**同一张关系表的两个方向**。合成一页之后：

```
机型与资源
├ 左：6 个机型卡（品牌 / 机型 ID / N 版 / presetFile）  [+ 新增机型]
└ 右：
   ├ 元信息（折叠）
   ├ 尺寸与禁区（两个二级 Tab，照 mkppanel 的机型尺寸页）
   └ 版本（每张卡可折叠）                      [+ 新增版本]
      版本 #1 (STANDARD) 推荐            [克隆] [删除]
        版本 ID / 版本名称 / presetFile / 推荐预设 / 标签 / 描述
        最终资源  MKP A1.toml · BBS MKPProcess A1 0.4 0.20.json   [换]
   └ 危险操作：删除此机型（保存前可撤销恢复）
```

Bundle 那个"反向视图"不另开一页，做成资源行上的一句
「这个资源还被 A1_MINI 的 3 个版本用着」+ 可跳转。
**同一个信息只给一个编辑入口，另一个方向只读。**

## 3. 后端要补什么

好消息是大部分已经在了：`wb_apply_draft` 单写入口、撤销/重做、草稿内存化 + 懒落盘、
三档校验、TOML 渲染与生成、回收站、真哈希盘点 —— 这些跟形态无关，全部留用。

要补的是**"清单也能改"这件事带来的 patch 与断言**：

```rust
// patch.rs 新增（当前 12 个 → 18 个）
NewMachine   { machine_id, brand, display }   // 机型 ID 唯一、不许和别名撞
RenameMachine{ machine_id, display }
DeleteMachine{ machine_id }                   // 连带版本一起进回收站
SetDimensions{ machine_id, dims }             // 尺寸与移动范围
SetForbidden { machine_id, zones }            // 禁区
SetParamDef  { key, patch }                   // 字段定义（label/min/max/默认值…）
```

跟着要动的：

- `upstream/` 那一层**改名并去掉"只读"的语义**。现在它叫 `upstream` 且有一条
  `upstream_layer_has_no_write_path` 的源码扫描测试钉着"这一层不许有写路径" ——
  前提变了，那条测试要改成"写路径只许经过 `storage.rs`"（原 Task 10.8，一直挂着）。
- 新建机型/版本之后，`Committed` 不能再"只从上游声明的版本预填" ——
  §0 那个死胡同就在这里，要从根上改掉。
- 生成 TOML 时，新机型没有 `presetFile` → 那是一条**阻断**，不是提示。

## 4. 这一份不做什么

- **不重新设计 SOP。** 你试过、做不出更顺的，那就照 mkppanel 原本的结构做。
  我不再自己发明一套"更有逻辑"的流程 —— 这一轮的教训就是我发明的那些（矩阵当默认、
  三种改法、列编辑）没有一个比它原来的形态更好用。
- **不做拖拽排序、不做自由改父子挂靠、不做布局 OTA。** 你说了多余。
- **不做覆盖层。** 见 §1。
- **不动客户端那 4 条 IPC 与三道编译隔离。**

## 5. 边界与异常

| 情形 | 处理 |
|---|---|
| 新建机型没填 `presetFile` | 能存草稿，**生成时阻断**，并指到那一格 |
| 机型 ID 与已有的别名撞 | 当场拒绝（`catalog.rs` 的别名冲突断言已经在了，复用） |
| 删机型时它下面有版本 | 版本一起进回收站，提示写清"连带 N 个版本" |
| 删了机型又撤销 | 从回收站原位恢复（`RestoreVersion` 已有，机型要补一个） |
| 改字段定义的 min，已有值越界 | 记**待办**不阻断：值还能生成，但下次编辑会被控件挡住 |
| 改 `tomlKey` | **拒绝**。产物的键名，改了客户端读不到。这一条是例外的"不许改" |
| `content/*.json` 被外部改过（mtime 变了） | 开场检测到就提示"外面改过，要不要重读"，不静默覆盖 |
| 侧边栏切页时有未保存草稿 | **不拦**。草稿是全局的一份，切页不丢；保存按钮常驻在顶部状态条 |

## 6. 预期结果

1. 侧边栏七页，每页只干一件事，**页与页之间不共享任何编辑状态**
2. 「+ 新增机型」在机型与资源页右上角；「+ 新增版本」在版本区右上角；两个都能一路走到保存
3. 新建的版本**保存后出现在清单里**（§0 那个死胡同消失）
4. Bundle 不再是独立一页；资源的反向引用只读可跳转
5. 矩阵变成参数页里的一个查看开关，导航里没有它
6. 现在界面上摆着但没做的入口全部消失 —— 做完一页再加下一页

---

## 7. 我的判断，说给你听

这一轮我要承认两件事：

**第一，我把顺序做错了。** 应该先有"加机型 / 加版本"，再谈参数怎么改得顺手。
我却先花了两轮在"矩阵的排序与颜色"和"换成分组列表"上 ——
那两轮的产出（后端排序权威、草稿内存化、一次手势一次 IPC、灰格子说清谁关的）是有价值的，
但它们都是在**一个连基础对象都建不出来的系统**上做打磨。

**第二，我不该用测试把半成品钉住。** `NewVersion` 保存后在树上看不见，
我给它写了一条断言"必须有提示"就当处理完了。测试通过不等于功能成立 ——
那条测试证明的是"这个死胡同的行为是确定的"，不是"这件事能用"。
