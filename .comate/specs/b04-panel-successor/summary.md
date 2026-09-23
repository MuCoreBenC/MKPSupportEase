# Task 8 第 1 步：清单换源（`Committed` ← `presets/machines/*.toml`）

按 `HANDOFF.md` §1 的建议切法收口：**只做第 1 步，并让全套测试绿**，
没有把四步各做一半。

## 做了什么

### 1. 清单成了一等公民，而且是我们自己的数据

`Committed` 原来只有一个 `machine_ids: BTreeSet<String>`（机型 id 的集合，来自上游产物），
现在是 `catalog: Vec<CatalogMachine>`：

```rust
pub struct CatalogMachine {
    pub id: String,
    pub display: String,
    pub icon: Option<String>,
    pub default_bundle: Option<String>,
    pub has_dimensions: bool,
    /// 版本 id，照机型文件里的顺序
    pub version_ids: Vec<String>,
}
```

`BTreeSet` 换成 `Vec` 是因为**顺序会直接进界面**，而字典序不是作者写下的顺序。

`storage::load` 因此从 `&Upstream` 改收 `&Presets`：清单（有哪些机型/版本、版本叫什么、
角标、尺寸档）来自 `presets/machines/*.toml`，`workbench/*.json` 只剩「值」那一半。

### 2. `Book` 照 `Committed` 铺树，不再读 `up.catalog`

- `Book` 多一个 `catalog: &'a [CatalogMachine]`，`book_view` / `order_cols` /
  `machine_layers` / `machine_default_bbs` / 徽章计数全部改用它
- `digest` 从 `(registry, &upstream::Machine)` 改成 `(registry, 机型 id, 版本 id 列表)` ——
  这张表只认这两样，收一个具体类型只会把纯函数绑在某个 loader 上
- 产物（`mkpPreset`）仍按 `(机型, 版本)` 去上游查：查不到就是 `None` = 未生成，
  对新建的版本来说那正是对的

`Book::new` 的签名**没动**，所以 60 多处 `Book::new(&f.up, &c, &d)` 一处都不用改。

### 3. 那条死胡同真的通了

`Patch::NewVersion` 保存时，`save` 现在**先往 `presets/machines/{机型}.toml` 插一个
`[[versions]]` 块，再写值文件**。顺序不能反：清单写失败时先写的值文件会变成一个
谁都不认的孤儿。为此 `save` 收 `&mut Presets`。

`declared_upstream` 改名 `declared`，语义跟着变成「**我们的**清单里还有这一版吗」。
`PurgeVersion` 的拒绝话术也跟着改了：不再说"版本清单由上游维护"，
而是"这一页删的是值；要把它从清单里去掉，去「机型与版本」页删版本"。

`Ctx::reload_from_disk` 顺手把 `presets` 也重读一遍 —— 「机型与版本」页是自己读自己写的，
不重读的话这边的树会比盘上旧一步。

## 判据

| 判据 | 盯的是 |
|---|---|
| `a_new_version_shows_up_on_the_tree_after_saving` | 盘上有 `[[versions]]`（重读 loader 查）+ `declared` + **零提示** + 树上有它 |
| `saving_a_new_version_puts_it_on_the_tree_with_its_new_uid` | 同一件事在 IPC 一侧：`view.machines` 里有它、徽章 4→5 |
| `a_colliding_new_version_is_refused_not_overwritten` | **失败不留痕**：被拒之后机型文件的文本一字未动 |
| `Fixture::load` 里的构造时断言 | 手写的上游 JSON 与生成的 presets TOML 一旦分岔就 panic（机型/版本/尺寸档三项） |

**判据的判据**：把 `write_machine` 那一步临时拿掉，上面前两条都红
（"机型文件里没有 NEW1 —— 那就是那个死胡同"）。验完立刻还原。

夹具那份 `presets/`：机型文件手写（它的形状本身是被测对象），
`param_registry.toml` 与 `layout_schema.toml` **由上游那两份 JSON 序列化成 TOML**，
不手写第二遍 —— 两边键名本来就一样，一份来源两种格式不会分岔。

## 验证

- `cargo test --features workbench`：**264 passed / 0 failed**
- `cargo test`（默认 feature）：**18 passed / 0 failed**
- `cargo clippy --features workbench --all-targets -- -D warnings`：EXIT=0
- `cargo clippy --all-targets -- -D warnings`：EXIT=0
- `npm run lint` / `npm run build` / `npm run build:workbench`：全过
- 真数据未被污染：`presets/` 的 12 个 `.toml` 与
  `mkpse-next-v3/mkpse-presets/source/` 逐文件 sha256 比对，**12 比 12，0 不一致**
  （没有用 `git status` —— `presets/` 还未跟踪，它只会回一行 `?? presets/`）

## 没做的（Task 8 剩下两步，`tasks.md` 里已标 ⏸）

- **8.1** 删 `workbench/machines|versions/*.json` 的读写
- **8.3** 三层值解析改读 `presets` 的 `[params.machineVariants]`
  （`ParamDef` 已经是同一个类型，换的是 loader）

这两步是一件事：值也搬过去之后，那套自造的 json 才能整个删掉。
现在停在「清单已换、值还在老地方」这个**编译得过、测试全绿、界面行为正确**的中间态上。

另外留了一处刻意的不对称：`issues::machines` 的「床身尺寸未配置」还在看上游的
`dimensions`（尺寸页是 Task 11），而 `BookView` 的 `dimensions_missing` 已经读 presets 了。
夹具里那条对齐断言就是为这处不对称准备的 —— 两边说的不一样时它会先炸。
