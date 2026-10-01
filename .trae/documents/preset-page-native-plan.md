# 预设页接入真后端（A41 → native）· 实施方案 v2

> v1 把「代码共用」和「数据共用」混在了一起，问你的问题（A / A′ / B / C）也就问歪了。这一版先把**产品边界**定下来，再谈技术。
> 第 4 节是**真正需要你拍板的问题清单**（这次不预设答案）。

---

## 1. 产品边界（先定这个，其余都是推论）

```
        ┌──────────────────────────────┐
        │      预设数据解析核心          │   ← 一份代码
        │  Catalog / Assets / Bundles   │
        │  ParamRegistry / 三层取值      │
        │  「给我一个 root，我来读」      │
        └───────────────┬──────────────┘
                        │
         ┌──────────────┴──────────────┐
         ▼                             ▼
   【正式客户端】                   【开发工作台】
   appDataDir/presets/            <repo>/presets/
   用户机器 · 安装释放              开发机器 · Git 管理
   云端下载 · 用户导入              工作台直接改 · 进 git diff
   用户使用                        PR / review / commit

   代码：一套。   data root：两套。   两边永不互相读写。
```

三条硬结论（与你说的一致）：

1. **客户端读自己的 `appDataDir/presets/`，运行期绝不碰仓库路径**（用户装完没有仓库）。
2. **工作台只读写真仓库的 `<repo>/presets/`**，直接改、进 git diff / review。
3. **同一个开发机上同时跑两边，也不共用、不混用目录**。数据同步靠「改仓库 → review → 构建发布 → 生成新的客户端默认数据 / 云端资源 → 客户端更新」这条**单向**链路，不是靠共享目录。

---

## 2. 现状核对（我重看了仓库，确认可行的边界在哪）

读 `presets/*.toml` 的那层代码现在住在 `workbench` feature 里（`lib.rs` 里 `#[cfg(feature = "workbench")] pub mod workbench;`）。所以默认客户端构建连编译都不碰它 —— 这是**刻意**的隔离，要保留。

我逐个查了 `src-tauri/src/workbench/presets/` 的依赖，**它几乎是自洽的**，只有三处耦合到工作台：

| 文件 | 现状耦合 | 拆法 |
|---|---|---|
| `presets/mod.rs` | `Presets::load()` 里调 `workbench::paths::presets_root()`（自己决定 root） | **删掉 `load()`**；只留 `load_from(root)`，root 由调用方给 |
| `presets/assets.rs` L40 | 用 `workbench::paths::assets_root()`（`<repo>/public/assets`） | 改成由 root 相对定位 / 调用方传，见第 4 节问题 ② |
| `presets/*.rs` 的测试 | `use crate::workbench::paths` | 改用临时目录 / fixture（仓库已有 `domain::testkit::Fixture`） |

其余依赖只有 `crate::error`、`crate::fsx::{atomic,paths}`、`toml_edit` —— 都跟工作台无关。**没有一处用到 `mkpse-preset`**（那个 crate 带了 56 KB 注册表 + 9 份预设的 `include_str!`，必须挡在客户端之外，见第 3 节）。

`workbench/domain/` 则**明显分两半**：

- 纯计算：`layer.rs`（三层取值 出厂→机型基底→版本覆盖）、`variants.rs`（三步归并）→ 客户端读参数要用；
- 工作台业务：`patch` / `preview` / `derive(Book)` / `issues` / `wording` / `visibility(showWhen 链)` / `testkit` → 客户端**不用**（`showWhen` 的判定契约上明确「放前端算」，见 `contract.ts` L275）。

---

## 3. 目标结构（方案 D）

```
src-tauri/src/
├── presetdata/              ← 非 feature gate，默认构建也编
│   ├── mod.rs               ← Presets 结构 + load_from(root)（不决定 root）
│   ├── catalog.rs           ← machines/ + forbidden_zones/ + brands.toml
│   ├── assets.rs            ← assets.toml
│   ├── bundles.rs           ← bundles.toml
│   ├── registry.rs          ← param_registry.toml + layout_schema.toml
│   └── resolve.rs           ← 纯取值（自 domain/layer.rs 抽），客户端读参数用
│
├── client/                  ← 正式客户端的数据根 + 首启释放
│   ├── paths.rs             ← appDataDir/presets
│   └── defaults.rs          ← include_str! 的 12 份源 TOML
│
├── ipc/                     ← 正式客户端命令（8 个读）
├── fsx/  error.rs  obs/  chrome.rs
│
└── workbench/               ← 仍然完整 feature gate，业务一行不动
    ├── paths.rs             ← <repo>/presets（工作台自己的根）
    ├── app/  store/  clock.rs
    ├── domain/              ← patch / preview / derive / issues / wording / visibility / testkit
    └── presets/ 或 → 改为 use crate::presetdata
```

关键点：`presetdata` **不决定数据在哪**，只提供 `Presets::load_from(root)`：

```rust
// 客户端
Presets::load_from(client_paths::presets_root(&app)?)   // appDataDir/presets

// 工作台
Presets::load_from(workbench::paths::presets_root()?)   // <repo>/presets
```

`Cargo.toml`：`toml_edit` 从 `optional = true` 提为常规依赖（它本来就在 workspace 根，不是新引一个 crate）。`mkpse-preset` **继续只挂在 workbench feature 下** —— 客户端不引它。

---

## 4. 这次真正需要你拍板的问题

> 每条都写了「背景 / 选项 / 我的建议」。你逐条给一句话即可。

### ① `presetdata` 的边界画在哪？

**背景**：搬太少 → 客户端要自己重算三层取值，出现第二份真相；搬太多 → 把工作台业务拖进客户端，又粘死了。
**选项**：
- (a) 只搬 `Presets` + 4 个数据文件读取（catalog/assets/bundles/registry）。
- (b) (a) 再 + 纯取值 `resolve`（来自 `domain/layer.rs`：版本→机型→出厂的取值与 `applies_to` 过滤）。
- (c) (b) 再 + `domain/variants.rs` 的三步归并。
**建议**：**(b)**。客户端「读有效值」只需要取值，不需要归并（归并是**写**路径：决定某个版本键该不该上提成机型基底）。`variants` / `patch` / `derive` 等继续留工作台。

### ② 资产文件本体怎么办？`sizeText` / `modifiedText` 给不给真值？

**背景**：`assets.toml` 里的路径指向 `<repo>/public/assets/`（机型图、图标、BBS profile 文件本体）。客户端 appDataDir 里没有这些文件。契约里 `sizeText` / `modifiedText` / `statFrom` 都是**可选**的。
**选项**：
- (a) 只释放「定义」（`assets.toml` 等），**不释放文件本体**。机型图走前端产物（vite 的 `public/assets/` 已经在 webview 包里，按 URL 取，不受影响）；BBS/文件大小与时间**留空 → 界面显示「未知」**（契约本来就允许，且仓库铁律「宁可留空也不造数字」）。
- (b) 把文件本体也释放进 appDataDir/presets，然后 `stat` 真值。
**建议**：**(a)**。图能显示（走前端），大小/时间诚实留空。真值等云端发布打 manifest 那一轮再算。

### ③ 释放进 appDataDir 的「定义类文件」，升级时要不要覆盖？

**背景**：首启铺一次没问题。但客户端升级会带来新的内置默认，而 appDataDir 里那份可能被用户动过（或只是过期了）。`machines/assets/bundles/registry` 属于「定义」，不是用户的产物。
**选项**：
- (a) 只在缺失时铺一次，**永不覆盖**（最保守，但升级后新机型/新参数永远进不来）。
- (b) 每次启动用内置默认覆盖定义类文件（简单，但会抹掉用户对定义文件的手改 —— 如果允许改的话）。
- (c) 定义类文件**只读、以内置为准**：启动时比对内置版本号（如内容指纹），不同就覆盖；`presets/mkp/`（用户产物区）永不覆盖。
**建议**：**(c)**，本轮可以先实现「缺失就铺 + 内存里比对指纹」，覆盖策略留到有真正升级需求时收紧。这条你只要告诉我方向即可。

### ④ 本地文件（`presets/mkp/`）怎么跟 asset id 对齐？

**背景**：`getLocalFiles` 返回的是 **asset id 集合**（不是文件名）。官方下载下来的文件能对上 assets.toml 里的 id；但**用户自己拖进来的文件**没有 id（契约里 `LocalUserFile.id` 是「真后端用绝对路径的哈希」）。
**选项**：
- (a) 官方文件：按文件路径/文件名反查 `assets.toml` 得到 id；用户文件：路径哈希成 `user_` 前缀 id。与假后端 `mockServer/localFiles.ts` 同口径。
- (b) 让客户端目录里多一份 index（记录「文件 → id」的映射），由下载时写。
**建议**：**(a)**。与现有假后端口径一致，且不需要新格式。本轮本地初始为空，这条主要是把规则定死。

### ⑤ 「已应用」这个底账落哪？

**背景**：契约 `getAppliedPreset()` 描述的是「后端才知道的、本机当前生效的那一套」（全局唯一）。现在页面实际上读的是**前端 localStorage**（`src/app/store/package.ts` 的 `activeEntry()`），没用后端。
**选项**：
- (a) 本轮维持前端 localStorage（页面不用 `getAppliedPreset`，后端可继续 `notWired`）。
- (b) 顺手把底账挪到 appDataDir 的一个文件，`getAppliedPreset` 接真实现。
**建议**：**(a)**，本轮不扩范围；等云端下载/应用那一轮一起挪，避免半套。

### ⑥ 要不要加一道「两个数据根必须不同」的运行期保护？

**背景**：你会同机同时跑客户端（appDataDir）和工作台（`<repo>/presets`）。理论上永远不会相等，但万一有人把 appDataDir 指到仓库里呢。
**选项**：
- (a) 不加，靠代码里两个 root 来源不同来保证。
- (b) 加一条断言/告警：两个根 canonicalize 后若相同就拒绝启动其中一边。
**建议**：**(a)** 本轮不加（避免过度设计）；如果你觉得这道保险值得，我可以加一条轻量告警日志。

### ⑦（不在本轮，先记着）工作台改完怎么进客户端

「改仓库 → review → 构建发布 → 生成新的客户端默认数据 / 云端资源 → 客户端更新」这条单向链路，是**下一轮**（云端联动）要设计的，本轮只做「客户端读自己的 appDataDir」。这里只是把它记在案，不问你。

---

## 5. 实现步骤（按 ④①② 的答复落地）

**第 0 步 · 抽 `presetdata`**
- 迁 `workbench/presets/{catalog,registry,bundles,assets}.rs` + `mod.rs` → `src-tauri/src/presetdata/`。
- `Presets::load()` 删掉（或移进 workbench）；`load_from(root)` 保留。
- 按 ① 的答复，把纯取值（`layer.rs` 的 `effective` / `keys` / `applies`）搬进 `presetdata/resolve.rs`；`variants/patch/derive/visibility/...` 留 workbench。
- 按 ② 的答复处理 `assets.rs` 的 `assets_root`。
- `lib.rs`：`pub mod presetdata;` 不带 cfg；`workbench` 侧改成 `use crate::presetdata::...`。
- `Cargo.toml`：`toml_edit` 去 optional；`mkpse-preset` 保持 optional。
- 测试里的 `workbench::paths` 依赖改掉。
- **验收**：默认构建（`cargo test`，不带 feature）现在会编译 `presetdata` 且全绿。

**第 1 步 · 客户端数据根 + 首启释放**
- `client/defaults.rs`：`&[(&str,&str)]` 表，`include_str!` 那 12 份源 TOML（`brands.toml`、`machines/*.toml`×5、`assets.toml`、`bundles.toml`、`layout_schema.toml`、`forbidden_zones/*.toml`×3、`registry/param_registry.toml`）。
- `client/paths.rs`：`presets_root(app)` = `appDataDir/presets`；`seed_if_absent(root)`（标志文件判据同 `workbench::paths::presets_root`）。按 ③ 的答复决定覆盖策略。
- `lib.rs` 的 `setup` 里调用释放；失败只告警不挡启动。
- `presets/mkp/` **不释放**，初始为空。

**第 2 步 · 8 个客户端读命令**（逐字段对齐 `src/api/contract.ts`，参照 `src/api/mockServer/*.ts`）

| 命令 | 来源 | 参照文件 |
|---|---|---|
| `getMachines` | catalog + 禁区 + image/icon | `mockServer/machines.ts` |
| `getPresetFiles` | assets + 套餐归属 + 版本反查 → `delivery`/`inBundles`/`usedByVersions` | `mockServer/resources.ts` |
| `getMenu` | 套餐归属 + 版本引用 → `bundled`/`optional`/`archived` | `mockServer/menu.ts` |
| `getVersionFiles(machine,version)` | 版本 → 套餐 → assetRefs → `FileRef[]`（+ `incomplete`/`missing`） | `mockServer/files.ts` |
| `getMachineParams(machine,version)` | `presetdata::resolve` 三层 → `RecipeParam[]` | `mockServer/params.ts` |
| `getParamMeta` | registry + layout | `mockServer/params.ts` |
| `getLocalFiles` | **真扫** `<root>/presets/mkp/`（按 ④ 的规则给 id，初始为空） | `mockServer/localFiles.ts` |
| `getLocalUserFiles` | 用户目录，本轮空集合 | 同上 |
| `getSlicerCopied` | 切片器目录，本轮空集合 | 同上 |

- `getAppliedPreset` 按 ⑤ 决定；`copyToSlicer` / `downloadFiles` 本轮继续 `notWired`。

**第 3 步 · 接线**
- `src-tauri/src/lib.rs`：客户端命令**两份清单都要出现**（非 workbench 与 workbench）—— 漏一份就是「原生机能用、工作台构建不能用」。
- `src/api/bridge.ts`：对应方法从 `notWired` 换成 `call(...)`。

**第 4 步 · 验证与交付**
- 跑第 7 节那 9 条 → 新分支 → PR → CI 绿 → squash 合并。

---

## 6. 明确的「不做」

- 不把 `workbench/presets` + `workbench/domain` 整体搬出去（那会把工作台业务带进客户端）。
- 不给客户端开 `--features workbench`（`wb_*` 命令与工作台窗口不许进用户二进制）。
- 不让两边共用/混用数据目录；不自动同步。
- 工作台的 `draft/snapshot/trash/publish` 继续留在工作台的 `store/` 体系里，不搬去 appDataDir。
- 客户端默认构建不引 `mkpse-preset`（避免 56 KB 注册表 + 9 份预设进用户二进制）。
- 本轮不做云端下载 / 真实网络。

---

## 7. 验证方式

**端到端**：`npm run tauri dev` → 「预设」页 → 机型三级树 + 文件清单正常渲染；本机表显示「暂无」（本地目录初始为空）；**不再整页 error**（现在的 `加载失败：[API] 未实现的接口: getPresetFiles`）。

**九条验签命令（与 CI 逐字一致）**：

```
npm run lint
npx tsc -b
npm run build
npm run build:workbench
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets -p mkp-support-ease --features workbench -- -D warnings
cargo test
cargo test -p mkp-support-ease --features workbench
```

**提交前**：工作台会往仓库 `presets/` 写盘（历史踩过 `param_registry.toml` 残渣导致 CI 红），先 `git status` 确认数据没被改脏。