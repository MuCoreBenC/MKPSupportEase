> ⚠️ **历史快照（2026-10-05 加注）**：本文写于寻址改造之前，其中的 `presets/dist/`、
> `baseUrl` 同目录推导、release.json 位置等表述是**当时的现状记录**，不再是现行规则。
> 现行规则见 `docs/RESOURCE-ADDRESSING-ROADMAP.md` 与 `docs/PUBLISH-ARCHITECTURE.md` §2.2。

# 第三刀交接（发布链：结构代次 / 软件更新 / 不兼容提示）

> **给新对话窗口的第一份材料。** 开工按这个顺序读：
>
> 1. 本文件（要做什么、做到哪、哪些已经能直接用）
> 2. 根 `HANDOFF.md`（进度台账；**§0 进度 / §3.5 第三圈 / §5 纪律 / §6 判据**）
> 3. `docs/PUBLISH-ARCHITECTURE.md`（**发布链根规则**；§5.3 = 本刀的主题）
> 4. `docs/DATA-ARCHITECTURE.md`（数据架构总纲，四层 + 四铁律 + 十问）
>
> 这三份是"规则"，代码是"实现"——**冲突时以规则为准，改规则要作者点头**。

---

## 0. 一句话现状

MKPSupportEase（Tauri + React + Rust workspace）。发布链按 `PUBLISH-ARCHITECTURE.md` 分三刀施工：

| 刀 | 内容 | 状态 |
|---|---|---|
| 第一刀 | 路径语义切换（`catalog.path` 基准 = 发布根 `presets/`） | ✅ 已收口 |
| 第二刀 | 发布闸 `PublishAudit`（十五项，唯一入口判定器） | ✅ 已收口 |
| **第三刀 上半** | **结构签名 + `minVersion` 规则表**（⑫ 从 Skipped → Green） | ✅ 已收口（本文件的主语之一） |
| **第三刀 下半** | **软件更新入口 + 不兼容提示 + 下载前拦** | ⏳ **待做（本文件的任务）** |

判据规模：`cargo test` **300** / `cargo test --features workbench --lib` **519**。

分支：`feat/param-def-controls-undo` 系（一连串刀都在这一支上，**均未推**）。台账与代码同一 commit；
**验收通过即提交，别自发做全仓盘点式扩展**。

---

## 1. 第三刀下半要做什么（作者已拍板，照做）

### 总基调（作者原话的意思）

> **按正式产品的基调设计，不要因为现在还在开发就把 UX 做成"开发工具味"。**
> 别把后台的发布工程学暴露给用户。

用户**永远不该看到**：`structureSignature`、`minClientVersion`、`catalogSchema`、兼容矩阵。
用户只该看到两句话：**"你的 SupportEase 太旧了"** 和 **"去更新"**。

### A. 设置页 —— 软件更新状态的**唯一**用户入口

- 有新正式版本：**「有新版本 SupportEase」**（给当前版本 → 最新版本 + 一个「查看更新」动作）
- 没有：**「已是最新版本」**
- 它长期存在；用户主动去设置里能看到。
- ★ **"有新版"这个事实属于软件更新系统，不属于预设系统。**

### B. 预设页 —— **不主动宣传**版本兼容问题

- **列表照常显示**（`A1 Standard` / `A1 0.2mm` / …）。**不要**整表标红、不要"❌ 不兼容"、
  不要把一个普通小工具变成"版本兼容性管理器"。
- **只在用户实际使用一个当前客户端读不了的预设时**才提示（作者给定的文案，照用）：

  > **此预设需要更新版 SupportEase**
  > 当前客户端版本过旧，暂不支持此预设文件，请前往设置更新

- 按钮：**「去更新」**（跳设置页的软件更新那一块）。若判断预设页不该承担更新动作，
  用「知道了」也可接受 —— 两者取一，**别做成连环弹窗**。
- ★ **不要说"这个预设有更新"** —— 用户会理解成"这个预设自己有个更新按钮"，
  而实际事实是**我的客户端太旧，读不了它**。两个概念必须分开。

### C. 后端 —— **下载前拦，不下载不使用**

```text
用户点击预设
   ↓
structure::can_read(...)          ← 已有（`src-tauri/src/runtime/structure.rs`）
   ├─ 不能读 → 不下载 → 返回**专门的业务错误** → 界面按 B 显示那句话
   └─ 能读   → 下载 → SHA / size 校验 → 使用
```

- **不要先下载下来再发现不能读。**
- ★ **不要复用普通加载错误**（作者点名），也**不要把内部字段暴露给 UI**。
  ⇒ 这需要 `ErrorCode` 里**新加一档**（现状见 §3.5；**作者已在拍板里授权**，
  但它改的是公开契约，要连 `src/api/contract.ts` 与那条对齐判据一起改）。

### Dev 怎么办（作者特意强调）

```text
Dev 构建 → SUPPORTED_SIGNATURES 能力命中 → 放行（**不看版本号**）
正式旧客户端 → 两段都不满足 → 拒绝 + 提示
```

即使新安装包还没打，也完全不影响本地开发。**但反过来要小心**：

> ★ **不要因为"Dev 能读"就把它当成"正式版本已经发布了"。**
> 更新检查必须有自己的**正式 release 信息源**；结构兼容检查只决定
> **"我能不能读这个数据"**，它不回答"有没有新版本"。

---

## 2. 第三刀上半已经交付了什么（可直接用，别重造）

### 2.1 `src-tauri/src/runtime/structure.rs`（新，本文件最该先读的代码）

```rust
pub const STRUCTURE_EPOCH: u32 = 1;                       // 机器看不出来的那类变化，人工 +1
pub const SUPPORTED_SIGNATURES: &[&str] = &["cb1080919d39b2bd"];  // 本构建的「结构能力」
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");  // 唯一版本号来源
pub const RULES_FILE: &str = "structure-signatures.toml";
pub const RULES_REL_PATH: &str = "presets/structure-signatures.toml";

pub fn signature_from(sample: &serde_json::Value) -> String;
pub fn signature_of_catalog(catalog: &Catalog) -> String;
pub fn can_read(signature: &str, min_client: Option<&str>) -> bool;  // ★ 下半的主角
pub fn version_at_least(actual: &str, required: &str) -> bool;

pub struct RuleTable { ... }        // 人工维护的「签名 → 最低正式客户端版本」
impl RuleTable {
    pub fn load(publish_root: &Path) -> Result<Self, AppError>;   // 文件不在 = 空表（不是错）
    pub fn rule_of(&self, signature: &str) -> Option<&Rule>;
    pub fn suggestion(&self, signature: &str) -> String;          // 红时印给人照抄的那一行
}
```

**签名的算法本身**（改它前先读模块头，那里写着为什么）：签名 = 「**必填**字段的模板路径 + JSON 形态」。
"必填"是**探**出来的（删字段再 `from_value::<Catalog>`：还成功 = 有 `#[serde(default)]`）。
于是 **加可选字段 → 签名不变**；加必填 / 可选改必填 / 类型变 → 签名变；
「路径语义 / kind 语义 / 读取方式 / 解释方式」这类机器看不出来的变化 → 靠 `STRUCTURE_EPOCH` 人工记。

### 2.2 `Catalog` 多了两格（`runtime/catalog.rs`）

```rust
pub structure_signature: String;        // #[serde(default)]；不进 revision_of
pub min_client_version: Option<String>; // #[serde(default, skip_serializing_if)]；同上
pub fn apply_min_client(&mut self, rules: &RuleTable);
```

`finalize()` 会从**自己**算签名（不是从随包那份 —— 那是上一代）。
`publish_into` / `write_catalog_json` 都从规则表填 `min_client_version`，
manifest 的 `minimumClient` 与它**同一个来源**（`PublishMeta.minimum_client` 已删）。

### 2.3 发布闸 ⑫（`workbench/app/audit.rs::structure_gate`）

Blocker：① 规则表里查得到当前签名吗 ② 本构建读得懂吗。查不到 → **禁止发布**，
并把该登记的那一行印在详情 / `fixHint` 里。**十五项现在一项 `Skipped` 都没有。**

### 2.4 规则表 `presets/structure-signatures.toml`（进仓库，人工维护）

第一条：`cb1080919d39b2bd → minClient = '0.0.1'`。
★ `minClient` 说的是「最低**正式**客户端版本」，**不要求那个版本已经发布** —— 那是 Dev 场景要的缝。

### 2.5 客户端那半的**判定**已经有了，**接线**没有

- ✅ 有：`structure::can_read`（**能力优先、版本兜底**）+ 判据
  `a_dev_build_is_let_in_by_capability_not_by_version` /
  `an_older_client_is_refused_the_data_it_cannot_read`
- ❌ 没有：任何命令 / 界面真的调它。

---

## 3. 现状清单（下半动手前必须知道的"实际上是怎样的"）

### 3.1 更新检查这条链

| 层 | 落点 | 现状 |
|---|---|---|
| 逻辑 | `runtime/update.rs::check(local, remote)` | 只比 `revision`，返回 `{up_to_date, local_revision, remote_revision}`。**还没有 readable 判定** |
| 命令 | `ipc/catalog.rs::check_remote_update` / `apply_remote_update` | 前者出 `RemoteUpdateDto`；后者直接 `release_bytes`（归档旧目录 + 新目录生效） |
| 契约 | `src/api/contract.ts::RemoteUpdateCheck` | `{upToDate, localRevision, remoteRevision}` |
| 前端 | `src/app/presets/usePresetData.ts::checkBootstrapOnce` | 进预设页**后台检查一次**（本次运行只一次）；有变化才 `applyRemoteUpdate`；★ **失败静默**（没配源 / 离线 / 远端没部署都不打扰用户） |

★ **这条链是下半的主战场**：`readable:false` 的判定加在 `update::check`（或它旁边），
然后 A（设置页）与 B（预设页）各自决定怎么呈现。

### 3.2 下载这条链

| 层 | 落点 |
|---|---|
| 命令 | `ipc/catalog.rs::download_runtime_file` / `download_runtime_files` |
| 契约 | `src/api/bridge.ts::downloadCatalogFile(s)` |
| 前端 | `usePresetData.ts::downloadRelease` / `downloadReleaseBatch`（→ `api.downloadCatalogFile(s)`） |
| 使用 | `usePresetData.ts::apply(fileName[, 'mine', path])` → `apply_active_preset` |

★ **注意一个事实**：客户端能枚举哪些文件，**只取决于本机那份 catalog**。
所以"下载前拦"真正能拦到的，是"本机目录被认为不可读"的那种情况；
主流程上（本机目录已可读）这一处通常不会红 —— 它是**兜底**，不是主触发面。
主触发面在 §4。

### 3.3 设置页现状

`src/app/settings/PageSettings.tsx` —— **今天只有一块「高级设置」**（预设数据源：内置官方源 / 手动覆盖）。
**没有任何版本号相关的东西**。

### 3.4 版本号现状（★ 现在是欠账状态）

- `src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、根 `package.json` 三处**都是 `0.0.1`**。
- 代码里**只在一处**读过版本号：`structure::APP_VERSION`（本次上半新建）。
- **前端拿不到版本号** —— 没有任何 IPC 命令暴露它。
- 有 `scripts/release.mjs`（软件版本发布线）与 `npm run publish:presets`（预设数据发布线），
  两条线是分开的（见 `scripts/publish-presets.mjs` 文件头）。
- ⇒ **A 要显示"当前版本 → 最新版本"，就得先有一条把版本号送到前端的路**。

### 3.5 `ErrorCode` 现状（★ 加一档 = 改公开契约）

`src-tauri/src/error.rs`：`NOT_FOUND` / `PERMISSION_DENIED` / `INVALID_ARGUMENT` / `CORRUPTED` /
`SHA_MISMATCH` / `IO` / `NOT_IMPLEMENTED` / `INTERNAL` —— **没有"不支持"这一档**。

作者已授权加 C 要的那个专门错误，但**联动面一个都不能漏**：
1. `error.rs`：加变体 + 一个构造函数；
2. `error.rs::tests::all_codes_are_screaming_snake_case`：把新档加进那张用例表（**它会红着提醒你**）；
3. `src/api/contract.ts`：`ErrorCode` 联合类型加同一串（**两份声明没有编译器对照，靠这条测试钉**）；
4. 前端若有按 code 分支的地方，跟着加。

命名建议：`NOT_SUPPORTED`，构造函数 `AppError::not_supported(...)`。
消息措辞照 B 的口径（**不出现签名 / minClient / catalog 这些词**）。

### 3.6 探针与验收脚本

- 前端验收 = **探针**（playwright-core + Edge，手工跑，截图落 `tmp-shots/`）。
  工作台那几支：`scripts/probes/workbench-build.mjs` / `chain.mjs` / `asset-preview.mjs` / `tabs.mjs`，
  4174，按文件头三步构建，产物 `node_modules/.cache/wb-probe/dist`，**用完关 4174**。
  客户端那两支：`presets.mjs` / `home-flow.mjs --pick`（4173）。
  ★ 别用 dev 5321（watcher 扫 `target/` 拖死）。
- 已知预存红（**不是你弄红的**）：`workbench-build.mjs` 的「钉住」（那个按钮已删）+ 生成页两条。

---

## 4. ★ 开工前必须先定的两件事（作者/下行会话拍板）

这两件都不是"实现细节"，是**产品决定**。别自己挑一个就干 —— 但也不要把它们当成四个问句丢回去，
**给出推荐 + 理由，让作者一句话点头**。

### 决定一：软件更新的信息源从哪来（A 的前提）

今天**没有**软件更新的信息源。`source.json`（Bootstrap）只回答「预设数据在哪」，
语义上是**预设数据源**，不是软件版本源 —— 作者原话：**更新检查应该有自己的正式 release 信息源**。

三条候选：

| 方案 | 做法 | 代价 |
|---|---|---|
| **(i) GitHub Releases** | 走 `scripts/release.mjs` 已经在打的 tag；客户端读 releases API | 要一个网络口 + 版本比较；仓库是 PR-only，release 流程已在 |
| **(ii) 发布侧多写一份 `release.json`** | 进 `presets/dist/`，随预设数据一起推；字段只有 `{version, notes, url}` | 与预设数据同一条发布线，最简单；但要**在语义上说清"它不属于预设数据"**（别塞进 `source.json` / `catalog.json`） |
| (iii) 复用 Bootstrap | 给 `source.json` 加一段 compat | ★ 作者已明确反对这条路（会把两件事又绑一起） |

**推荐 (ii)**：改动面最小、判据好写（发布侧写 + 消费侧读，与 catalog 同一套手法），
且天然带版本号与更新说明。**(i)** 更"正式"，但要多一个网络面和一套 GitHub 语义。

### 决定二：B 的触发面（★ 这条会卡住实现，必须说清）

作者要的场景是"**用户点使用时**才发现读不了"。但今天的事实是：

- **只有 catalog 有结构签名，单个交付文件没有代次。**
- 而客户端能列出哪些文件，靠的是**本机那份 catalog**；本机 catalog 若可读，
  它列出的文件就都可读 ⇒ **在预设页上"点一下就提示需要新版"这条路径，今天没有真实触发面。**
- 远端目录读不懂时，客户端**根本装不进来**（那正是不该装的东西）⇒ 真实触发面是
  **"远端有更新，但这一代我读不懂"**，也就是 **A（设置页）的那句话**，
  而不是预设页的某个条目。

两条可选路线：

| 路线 | 做法 | 覆盖到什么 |
|---|---|---|
| **(甲) 目录级（今天就能做）** | 只在 catalog 这一层判 `can_read`。远端读不懂 ⇒ 不采用 ⇒ **设置页说「有新版本 SupportEase」**；预设页**不出现**版本字样（作者要的"不主动宣传"天然满足） | 整批数据换代（今天的真实失败模式） |
| **(乙) 文件级（要改产物格式）** | 给交付的 MKP TOML 头部（血统三行旁边）加一个结构代次，使用前比对 ⇒ 才做得到"点一份外部拿来的预设 → 提示需要新版" | 外部带进来的单个文件（导入 / 人手拷 / 旧机器下的） |

**推荐：先做（甲），把 B 的那句话留在代码里等（乙）。** 理由：
① 今天唯一真实的失败模式是整批换代；② 单个文件级的代次要动产物格式，而"两批 renderer / 谁是
bootstrap 的源"本来就是**未清欠账**，不宜再叠一层；③ 甲已经把 C（下载前拦）与 A（设置页）都填满，
B 的呈现位在预设页留着（组件与方法都在），(乙) 来了只要多一个判断。

★ 无论选哪条：**列表照常显示**这条不能破。

---

## 5. 纪律（作者定死，别商量）

- **只有四类停下来问**：①产品行为变了 ②数据归属变了 ③公开契约 / API / schema 方向变了
  ④与已定架构原则冲突。其余**自己定，理由写台账**，别写成"要不要我做"的问句。
  优先序：守总纲 → 不引入兼容层 → 守圈边界 → 改动更小。
- **git 闸**：`main` 提交 / 推送要 `ALLOW_COMMIT_ON_MAIN=1` / `ALLOW_PUSH_MAIN=1`
  （feat 分支不需要）；推前 `git fetch`。一整刀一个分支连续施工、最后统一 PR；
  **台账与代码同一 commit**。
- **验证集合（全绿才推）**：
  ```bash
  cargo fmt
  cd src-tauri && cargo clippy --all-targets -- -D warnings
  cd src-tauri && cargo clippy --all-targets --features workbench -- -D warnings
  cd src-tauri && cargo test
  cd src-tauri && cargo test --features workbench --lib
  npm run build && npm run check:bundle && npm run check:zero-network
  npx tsc -b
  npm run lint
  ```
- 收工只报三件事：**达成 / 未达成 · 台账几条 · 下一步**；验收通过即提交。
- ★ **退役一个结构，必须同时裁掉读它的那一段探针** —— 读退役结构的探针恒红，比没有探针更糟。
  （本轮已经照这条裁过 `chain.mjs` 与 `workbench-build.mjs`。）

### 项目铁律（会咬到下半的几条）

- **只读命令一律 `#[tauri::command(async)]`**（判据 `read_commands_are_async_so_they_never_freeze_the_window`）。
- **网络只住 `runtime/net.rs`**；`src/api/` 不许出现 `fetch`（判据 2，`check:zero-network` 扫）。
  ⇒ A 的更新检查如果联网，必须走 `runtime/net.rs`。
- **四铁律**：开发文件不当运行时数据库；**云端不参与首屏**；用户没下载的不预置；
  运行时只认自己的运行时数据。⇒ **更新检查不能在启动 / 首屏路径上**（现在就是"进预设页后台问一次"）。
- ★★ **`with_ctx` 的锁不可重入**：已在 `with_ctx` 里的代码要复用能力，只能调收 `&Ctx` 的自由函数，
  **绝不能调命令壳** —— 那是自锁（挂死，不是报错）。
- 写盘一律 `fsx::atomic_write`；`serde` 的 `rename_all` **只管变体名**，变体字段要逐个
  `#[serde(rename = "camelCase")]`。
- 前端已知坑：React 合成 `onWheel` 是 passive；grid + `max-height` 的 auto 行仍按内容高算（要滚动用 flex）；
  flex `min-width:auto` 会撑破导航栏。

---

## 6. 验收清单（做完了长什么样）

- [ ] `structure::can_read` 在**更新链**上真的被调用（不是只写在库里）。
- [ ] 远端目录读不懂时：**不采用**、不改本机目录、**不下载**；返回**专门的**业务错误
      （新 `ErrorCode`，消息里**没有**签名 / minClient / schema 这些词）。
- [ ] 设置页有「软件更新」一块：有新版本 → **「有新版本 SupportEase」** + 当前版本 → 最新版本 + 动作；
      没有 → **「已是最新版本」**。（信息源按 §4 决定一。）
- [ ] 预设页**列表照常**（不整表标红、无"不兼容"列）；用户点使用时遇不兼容 → 那两句话 + 「去更新」。
- [ ] Dev 构建（`SUPPORTED_SIGNATURES` 命中）**不被版本号卡住**；判据钉着。
- [ ] 判据与文档同一 commit：`HANDOFF.md` 增量之二十一、`docs/PUBLISH-ARCHITECTURE.md`
      §5.3 的 ④ 与 §7 的"下半"行跟着改；新判据名写进台账。
- [ ] 探针：客户端 `presets.mjs`（4173）与工作台那几支都跑一遍，截图落 `tmp-shots/`；
      预存红之外的**新红为零**。
- [ ] 验证集合全绿（见 §5）。

## 7. 禁区

- ❌ **不要把 `structureSignature` / `minClientVersion` / `catalogSchema` 显示给用户**，也不要在错误消息里带。
- ❌ **不要把"有新版"和"读不懂"混成一句话**（两个不同的系统，两个入口）。
- ❌ **不要在预设列表上做整表兼容性标记**。
- ❌ **不要复用普通加载错误**来表达"不支持"（作者点名）。
- ❌ **不要复用 `source.json` / `catalog.json` 装软件版本信息**（那是预设数据源）。
- ❌ **不要把更新检查接到启动 / 首屏路径上**（铁律：云端不参与首屏）。
- ❌ **不要为了下半去动第一刀 / 第二刀 / 上半已经收口的东西**（路径语义、发布闸十五项、签名算法）。

---

## 附：本刀（上半）动了哪些文件（改之前先读，别重复造）

**新**
- `src-tauri/src/runtime/structure.rs`（签名 / 规则表 / `can_read`）
- `presets/structure-signatures.toml`（人工维护的规则表）
- `src-tauri/src/workbench/app/audit.rs`（第二刀的发布闸；上半往里加了 `structure_gate`）
- `src/workbench/views/PublishGateModal.tsx` + `.module.css`（第二刀的界面）
- `docs/PUBLISH-ARCHITECTURE.md`（发布链根规则）

**改**
- `src-tauri/src/runtime/catalog.rs`（两个新字段 + `finalize` 算签名 + `apply_min_client`）
- `src-tauri/src/runtime/mod.rs`（注册 `structure`）
- `src-tauri/src/workbench/app/dist.rs`（`minimumClient` 唯一来源；`PublishMeta` 去掉那一格）
- `src-tauri/src/workbench/app/build.rs`（`wb_publish_audit`；`PublishReport.minimum_client` 用真值）
- `src-tauri/src/workbench/domain/wording.rs`（`未声明` 的例子换了个活的）
- `src/workbench/api.ts` / `views/BuildPage.tsx` / `dev/mockBackend.ts`（契约 + 界面 + 桩）
- `presets/dist/{catalog,manifest}.json`、`src-tauri/src/runtime/catalog.generated.json`（已重出同代）
- `scripts/probes/workbench-build.mjs`（发布闸一节）、`scripts/probes/chain.mjs`（裁掉读退役结构的半截）
- `HANDOFF.md`（增量之十九 / 二十）

**重出元数据的办法（没有 CLI）**：`wb_publish()` 是无 AppHandle 的纯函数，
用一条一次性集成测试 `src-tauri/tests/*.rs` 驱动（**跑完即删**）；
顺序必须**先清残留 → 再发布**。
