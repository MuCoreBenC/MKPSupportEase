# 发布架构：唯一源 → 发布根 → 客户端消费

> 起草：2026-10-04（三轮澄清后的定稿）。**这份是"发布链"的根规则**，
> 与 `docs/DATA-ARCHITECTURE.md`（四层 / 四铁律，管**归属**）配套：
> 那份管"谁拥有什么"，这份管"发布时这些怎么变成客户端能消费的东西"。
>
> **它替代**：`docs/PRESET-DELIVERY-CONTRACT.md` 里 §3 的 A/B 裁决（已废，见 §1.4）、
> §5 的旧发布闸设计（已按本文 §5 重写）。

---

## 0. 三条裁决（2026-10-04 作者定，本文的一切都从这三条推出来）

| # | 裁决 | 一句话 |
| --- | --- | --- |
| **①** | **A 类资产不复制** | BBS / 图片 / 模型 / 图标在 `presets/assets/` 就是**唯一实体**，发布不制造第二份 |
| **②** | **`catalog.path` 相对发布根** | **发布根 = `presets/`**；路径不写 `presets/` 前缀 |
| **③** | **保留薄 `dist/`** | 只放**渲染产物**（MKP TOML）+ **发布元数据**（catalog / source / manifest） |

**唯一寻址规则**（2026-10-05 资源寻址改造定稿，`docs/RESOURCE-ADDRESSING-ROADMAP.md`）：

```text
客户端下载 URL  =  SourceResolver（唯一寻址出口）算

     Source Manifest（source.json v2）声明寻址规则：
       catalog / manifest / release / content 相对 Manifest 自身目录（交付根）
       filesRoot: ".."          ← catalog.files[].path 的锚 = 交付根的上一层 = 发布根 presets/
     catalog.path               ← 资源清单，相对发布根（干净相对路径，禁止 ..）
```

于是：

```text
presets/assets/bbs/Process/0.2mm/A1.json
        │  台账登记 path = "bbs/Process/0.2mm/A1.json"（相对资产根 assets/）
        │  发布时补上「assets/」这一段 → catalog.path
        ▼
catalog.path = "assets/bbs/Process/0.2mm/A1.json"
        │  Resolver：filesRoot("..") + path
        ▼
https://raw.githubusercontent.com/<o>/<r>/main/presets/assets/bbs/Process/0.2mm/A1.json
```

> ★ **职责切分说死**：Manifest = **寻址规则声明**（各类东西从哪个锚点找）；
> Catalog = **资源清单**（有哪些 Entry、各自 `path`）。Catalog 不得自带第二套根/URL。
> `filesRoot` 是**全局锚点规则，不是 Entry 路径的组成部分** —— 它只出现这一次。
> 业务代码**永不拼路径**：全仓唯一被允许拼接的地方是 `runtime/resolver`
> （发布闸 ⑯ `client/url-reconciliation` + Golden Fixture 守着这条跨端契约）。
> **历史教训**：2026-10-04~05 的 `dist/dist` 事故就是"两个锚各说各话、没有对账"
> —— 本段旧文曾把那套错误算术写成"最硬的一条规则"，谁再看到旧版不要信。

---

## 1. 两类资产：这是整套架构的分水岭

| | **A 类：已存在的资产** | **B 类：渲染产物** |
| --- | --- | --- |
| 是什么 | BBS JSON / 图片 / 模型 / 图标 | MKP 预设 TOML |
| 源在哪 | `presets/assets/<kind>/…`（**真有一份文件**） | **源里没有完整实体** |
| 从哪来 | 人写 / 外部导入，进仓库即成实体 | 工作台从 `machines/` + `versions/` + `templates/` **渲染** |
| 发布时 | **不复制**，只登记路径 | **必须落盘**到 `dist/mkp/presets/` |
| 落点 | `presets/assets/…`（原地） | `presets/delivery/mkp/presets/…` |
| `catalog.path` | `assets/<kind>/…` | `dist/mkp/presets/….toml` |
| 台账 | `presets/assets.toml` 有它一条 | 台账那条**只登记身份**（哪个版本叫什么、归谁），文件条目由生成侧算 |

**判据**（一句话）：*源目录里本来就有这份字节 → A 类；源目录里没有、得算出来 → B 类。*

### 1.1 目录布局（定稿）

```text
presets/
├── assets/                      ← A 类：唯一实体（不进 dist）
│   ├── bbs/Process/0.2mm/…
│   ├── printers/…   icons/…   models/…   brands/…
│   └── assets.toml 的同级台账 = presets/assets.toml
├── machines/  versions/  templates/     ← B 类的渲染源
├── bundles/  plates/  forbidden_zones/  ← 关系 / 实体
├── assets.toml                  ← 资产台账（登记 id / kind / path / delivery）
└── dist/                        ← 发布产物面（本次要交给客户端的东西）
    ├── catalog.json             ← 发布索引（元数据）
    ├── source.json              ← Bootstrap（元数据）
    ├── manifest.json            ← 发布完整性（元数据）
    └── mkp/presets/*.toml       ← B 类渲染产物（唯一该被复制进来的东西）
```

**明确不许存在**（本次要清掉的）：

```text
presets/delivery/mkp/bbs/       ← A 类，不该有副本
presets/delivery/mkp/icons/
presets/delivery/mkp/models/
presets/delivery/mkp/printers/
```

> 它们现在是历史遗留（发布时把所有 `download` 档资产都复制了一遍）。
> **清掉它们不是"删文件"，是"停止制造第二份真源"。**

### 1.2 为什么 `mkp/` 这一层保留（澄清一个易混点）

`mkp/presets/*.toml` 里的 `mkp/` **不是**"客户端下载区那个 `mkp/`"的意思，
而是**发布根下的一个目录名**。两者同形是刻意的：客户端下载落点与发布落点同形，
下载完直接落 `<appDataDir>/mkp/presets/A1-standard.toml`，路径形状一致、好对账。

**A 类不再进 `mkp/`**，客户端下载落点 = `<appDataDir>/<catalog.path>`（唯一路径语义）：

```text
A 类下载落点：<appDataDir>/<catalog.path>   = <appDataDir>/assets/bbs/…
B 类下载落点：<appDataDir>/<catalog.path>   = <appDataDir>/delivery/mkp/presets/…
```

**两者统一成一条规则**：`catalog.path` 就是**下载落点**（相对内部根），也是**URL 尾段**（经 Resolver 按 Manifest 声明锚定）。
**一个字段同时是"取哪"和"放哪"** —— 这是本方案最大的简化，也是它必须守的唯一性所在。

### 1.3 `delivery` 档位的最终语义

| 档位 | 含义 | 进 catalog？ | 进 dist？ | 客户端行为 |
| --- | --- | --- | --- | --- |
| `download` | 按需下载 | ✅ 登记（带 SHA/size） | A 类：否 / B 类：是（渲染产物） | 点下载才取 |
| `bundled` | 随安装包 | ❌ 不登记 | 否（构建期随包装配） | 装上就有（品牌图 / 整机图） |

### 1.4 A/B 裁决为什么废了（留个记录，防止绕回去）

旧的 A/B 建立在一个**错误前提**上：*"`catalog.path` 必须指向 `dist/mkp/bbs/…`，所以要么把文件复制过去（A），要么别登记（B）"*。

那个前提本身错了 —— `path` 没有义务指向 `dist/`。**C（= 本次裁决）：路径指向真实资产源路径。**
于是"登记面 ≠ 实体面"这个说法**自动消失**：登记面与实体面本来就是同一个面
（`catalog.path` 指哪，哪就必须在）。

---

## 2. 路径的三层语义（谁登记什么，谁补什么）

```text
① 台账       presets/assets.toml
             path = "bbs/Process/0.2mm/A1.json"      ← 相对【资产根 presets/assets/】

② 发布补前缀 kind_dir(kind)
             "bbs/…"  →  "assets/bbs/…"               ← 补上「资产根相对发布根的那一段」

③ catalog    CatalogFile.path = "assets/bbs/…"        ← 相对【发布根 presets/】
             （B 类 = "dist/mkp/presets/…"）

④ 客户端     URL    = SourceResolver（Manifest 声明 + filesRoot 锚）算
             落盘   = <appDataDir>/catalog.path
```

### 2.1 为什么台账不直接写 `assets/…`

台账 `path` 的基准是**资产根**（`presets/assets/`），因为资产引用它时不该关心
"资产根在仓库里叫什么"。发布时由**一处算法**补前缀 —— 这就是 `kind_dir` 的职责，
**只在这一处**，客户端与工作台都不再自己拼。

### 2.2 交付根与文件根的关系（★ 2026-10-05 定稿：锚点是声明，不是推导）

```text
仓库：          <repo>/presets/{assets, delivery}/
source.json：  <repo>/presets/delivery/source.json   ← Source Manifest v2（寻址规则声明）
交付根：        <repo>/presets/delivery/              ← Manifest 相对引用的基准（= 自身目录）
文件根：        <repo>/presets/                        ← filesRoot: ".." 声明 = 交付根上一层
catalog.path： 相对文件根（发布根）的干净相对路径
```

> ⚠️ **历史教训（钉在这防重犯）**：2026-10-04 那版曾写"只要 `source.json` 在
> `presets/dist/` 里，`baseUrl` 缺省（= 同目录）+ `catalog.path` 相对 `presets/`
> 一切自动成立"—— **那句算术是错的**（`presets/dist/ + dist/… = presets/dist/dist/…`），
> 并被写成了"最硬的一条规则"。结果是 2026-10-05 实测：catalog 里全部 24 条交付文件
> 在客户端拼出 404。修正方式不是再解释算术，而是**结构上消灭"猜"**：
> `source.json` 升级为 v2 寻址规则声明（`filesRoot` 显式声明锚点），客户端统一走
> `runtime/resolver`（唯一拼接出口），发布闸 ⑯ 逐条对账。别再回退到"目录推导"那条路。

---

## 3. 客户端改动的实际半径（★ 本章是 2026-10-04 改造时的施工推演，**历史价值**；
落地结果以 §2.2 与 `RESOURCE-ADDRESSING-ROADMAP.md` 为准 —— 甲案已裁、已落地，
落点规则是 `<appDataDir>/<catalog.path>`，不再是本节的 `mkp/<kind>/` 旧状）

### 3.1 已经对的部分（不用改）

- ~~`source.json` 的 `baseUrl` 缺省机制~~（**已废，2026-10-05**：那句推导的算术不成立，
  是 `dist/dist` 事故的病根；v2 起寻址规则全部显式声明，见 §2.2）。
- 客户端拼 URL 的地方只有一处（`net.rs::RemoteSource::fetch` 用 `join_url(base_url, file.path)`）。
- `catalog.path` 已经是"相对基准的一条相对路径"，只是基准从 `mkp/` 换成了发布根。

### 3.2 ★ 结构性障碍：客户端只有**一个**内部根，`assets/` 会落在它下面

客户端现在的落点规则是 `<appDataDir>/mkp/<kind 目录>/…`（`paths::mkp_dir`）。
新方案里 A 类要落 `<appDataDir>/assets/…`，B 类落 `<appDataDir>/mkp/presets/…`。

**后果**：`<appDataDir>` 下会多出一个 `assets/` 目录，它**不在** `mkp/` 里。于是：

| 受影响的地方 | 影响 |
| --- | --- |
| `paths::mkp_dir` / `ARCHIVE_DIR`（归档结构 `archive/mkp/…`） | 归档要不要跟着长出 `archive/assets/…`？ |
| `runtime::release` 的 `mkp_dir_is_created_empty` 判据 | 首启要建几个目录？ |
| 交付信任 / 旧版本识别（`trust_entries` 扫 `mkp/`） | A 类不在 `mkp/` 里了，扫描面要跟着改 |
| 客户端"本地文件"视图（`get_local_files`） | 读的目录面变了 |

**两条出路，需你裁（这是本方案唯一的真取舍）**：

| | 方案甲：**下载根与发布根同名对齐** | 方案乙：**`catalog.path` 只做 URL 尾段，落点另算** |
| --- | --- | --- |
| 做法 | 客户端落点 = `<appDataDir>/<catalog.path>`，即 A 类落 `assets/`、B 类落 `mkp/presets/` | 落点仍按 `mkp/<kind>/…`（现状态），`catalog.path` 只管 URL |
| 优点 | **一个字段同时管"取哪"和"放哪"**（§1.2 的简化成立） | 客户端运行时布局完全不变，改动最小 |
| 缺点 | 客户端内部根多出 `assets/`，上述四处都要跟改 | **"取哪"与"放哪"分成两套规则** —— 违反"唯一基准"的初衷 |

> **我倾向甲**：你三轮都在强调"唯一、不重复、通过引用组织"，
> 甲是唯一能让"一个 `path` 说清一切"成立的方案。乙虽然改动小，
> 但它把"URL 基准"与"落点基准"重新分成两件事 —— 那是旧的病根。

---

## 4. 发布链：Git 仓库本身就是发布物

### 4.1 定位（作者 2026-10-04 定）

```text
Git 仓库
├── presets/assets/…      ← 源资产，同时就是云端发布资产（同一份，无复制）
└── presets/delivery/…        ← 生成的发布元数据 + B 类渲染产物
```

**不存在**"源文件 → 再复制成云端文件"这一步。云端 = `main` 分支本身。

### 4.2 完整闭环（工作台负责全程）

```text
   你在工作台点【发布】
            ↓
   ①～⑮ 发布检查（逐项打勾，见 §5）
            ↓  全绿才亮按钮
      【创建发布 PR】
            ↓
   工作台：生成 → 提交 → 推分支 → 建 PR（全自动）
            ↓
   GitHub Actions 再跑一遍同样的检查
            ↓
   PR 状态回读进工作台
            ↓
   ┌────────┴────────┐
   ↓                 ↓
 ❌ 有问题         ✓ 全绿
   ↓                 ↓
 工作台显示        【打开 GitHub PR】→ 你点合并
                      ↓
                   main 更新
                      ↓
                   客户端 OTA 可消费
```

### 4.3 工作台负责到哪一步（**第一版边界**）

| 步骤 | 第一版 | 说明 |
| --- | --- | --- |
| 检查 | ✅ 工作台 | 本地全跑一遍 |
| 生成产物 | ✅ 工作台 | `dist/` + catalog/source/manifest |
| git commit | ✅ 工作台 | 只提交 `presets/` 下的变化 |
| push 分支 | ✅ 工作台 | 一次性分支名（照 `publish:presets` 现有命名） |
| 创建 PR | ✅ 工作台 | 调 `gh` 或 GitHub API |
| **跑 CI** | ✅ GitHub | 工作台轮询状态 |
| 回读 PR 状态 | ✅ 工作台 | 显示每项检查结果 |
| **合并** | ❌ **第一版不做** | **留给你在 GitHub 点** —— 权限 / 审计最简 |
| 合并后回读 | ✅ 工作台 | 读回"已合并 → main 已更新 → 客户端可消费" |

> 合并留给 GitHub 的理由：合并是**不可逆**的一步，把它留在有审计与权限的平台上是稳的；
> 工作台做"创建 + 回读"已经能消掉你现在"不知道该跑哪条命令"的痛点。

### 4.4 什么时候客户端才可消费

```text
PR 合并进 main  →  GitHub raw 立即生效（raw.githubusercontent 读 main 分支）
                →  客户端下次 OTA（进预设页那次 check）拿到新 catalog
                →  可消费
```

**没有额外的"部署"步骤** —— 这正是"Git 仓库即发布物"的含义。
（`raw.githubusercontent.com` 有 CDN 缓存，实际生效可能有几十秒延迟 ——
`source.json` 请求要带 cache-busting，见 §6 第 5 条。）

---

### 4.5 两本账的差集语义（catalog ⊇ manifest，**设计如此，不是漏生成**）

★ 2026-10-05 钉死（作者要求把"为什么"写进规则，防止后人把差集当缺陷）：

```text
catalog.json（files[]）   = 「所有客户端可见资源」的登记面 —— 台账里每一种有落点的
                            资产 + 全部 B 类产物，**无论当前有没有被引用**
manifest.json（assets）   = 「本版本完整性 / 下载管理范围」= 引用可达的交付子集
                            （被机型 / 套餐引用、且有落点的资产 + 全部 B 类产物）
不变式                    = manifest ⊆ catalog；差集 = 登记了但当前无引用的资产
```

当前真数据（2026-10-05）：catalog 24 条（9 mkp_preset + 3 icon + 3 model + 9 bbs_config），
manifest 17 条（9 mkp_preset + 3 icon + 5 slicerProfile）——差集 7 条 =
**0.2mm 档 BBS 4 份 + 校准 / 测试模型 3 份**，全部是"台账登记了、但当前没有任何
机型 / 套餐引用"的资产。它们：
- 照常被发布闸 ⑦「登记的每一条都取得到」管着（真要取，取得到）；
- 不进 manifest，所以不参与本版本的逐份 SHA 完整性账；
- 哪天被套餐 / 机型引用了，自动进入可达集、自动进 manifest —— **不需要改任何代码**。

钉子：判据 `the_manifest_covers_exactly_the_referenced_deliverable_set`（真数据），
以及 `doc §7` 的历史引用统一改指本节（那个 §7 属于已退役的上游文档，引用悬空）。

## 5. 发布闸（`PublishAudit`）—— 你要的那个模态框

### 5.1 结果模型（先定数据结构，再加检查项）

```rust
struct AuditItem {
    id: String,              // "assets/path-unique"
    name: String,            // 界面上那一行的名字
    status: AuditStatus,     // Pass | Fail | Warn | Skipped
    severity: Severity,      // Blocker | Warning（只有 Blocker 红才拦发布）
    details: String,         // 一句话说清"红了是什么、在哪"
    affected_files: Vec<String>,
    fix_hint: String,        // 界面上那颗"去修"的入口
}
struct PublishAudit {
    items: Vec<AuditItem>,
    files_added: usize, files_changed: usize, files_removed: usize,
    min_version: Option<String>,   // 见 §5.3
    can_publish: bool,             // 全部 Blocker 都 Pass（★ 含 ⑮ git/clean）
}
```

> ★ **`can_publish` 必须包含 ⑮ `git/clean`**：判定要**先 `items.push(git)`、再算 `can_publish`**，
> 否则脏工作区时闸会亮「确认发布」，绕过「除交付产物外工作区必须干净」那条保护。
> 它与 `blockers()`（遍历全部 items）严格等价 —— 判据
> `publish_audit_all_blockers_pass` / `the_gate_lists_every_item_and_only_opens_when_no_blocker_fails` 钉住这条等式。

**设计要点**：加一条检查 = 加一个 `AuditItem`，**不改流程**。这是你要的"以后扩展不用重设计"。

### 5.2 检查清单（十五项 + ⑯；⑯ 为 2026-10-05 寻址改造新增，见 §6 判据 10）

| # | id | 检查什么 | 现状 |
| --- | --- | --- | --- |
| ① | `sources/complete` | 源数据完整（machines/versions/templates/bundles 能读） | ✅ 第二刀（加载期校验的结论，摆出来） |
| ② | `refs/resolve` | 所有 `assetRef` 引用存在 | ✅ 第二刀（套餐 assetRefs + 机型三图位 + 版本图 + 品牌 `logo`） |
| ③ | `assets/exist` | 所有 A 类资产的**文件真在** `presets/assets/…` | ✅ 第二刀（只查**要交付**的那几条） |
| ④ | `assets/path-unique` | 所有 A 类 `catalog.path` 唯一且合法（无 `..`） | ✅ 第二刀 |
| ⑤ | `presets/renderable` | 所有 B 类预设能成功渲染 | ✅ 第二刀（走 `build::preview_with` —— 与"生成"同一台渲染器的**锁无关内核**） |
| ⑥ | `presets/rendered` | 渲染结果真在 `dist/mkp/presets/` | ✅ 第二刀（`dist_expected_set`） |
| ⑦ | ★ `catalog/matches-files` | **catalog 登记的每一条，其 `path` 在发布闭包里真能取到** | ✅ 第二刀（拿 **发布根 + `path`** 对，不再对 dist —— 0.2mm 就是这么漏的） |
| ⑧ | `catalog/sha-size` | SHA / size 对**真字节**算且一致 | ✅ 第二刀 |
| ⑨ | `catalog/no-phantoms`（幽灵） | 登记的条目**都有实体**（同 ⑦，此项管"指向不存在"） | ✅ 第二刀（**台账层面**：空 path / 越界 path） |
| ⑩ | `dist/no-strays`（残留） | 发布目录里没有未登记文件 | ✅ 第二刀（`dist_strays`） |
| ⑪ | `bundles/closure` | bundle 引用闭包完整（每条 ref 可达） | ✅ 第二刀（`Warning` —— 不拦发布） |
| ⑫ | `version/structure` | 结构签名 + `minVersion`（见 §5.3） | ✅ 第三刀（`runtime::structure` 算签名 + `presets/structure-signatures.toml` 查表；**查不到 = Blocker Fail**） |
| ⑬ | `source/correct` | `source.json` 正确（schema / catalog 相对路径合法） | ✅ 第二刀 |
| ⑭ | `manifest/correct` | manifest 与交付集合一致 —— **预检 = 提示（Warning）**：manifest 是**上一版发布**写下的账本（只有发布事务定稿会重写它），「生成过、还没发」的落后是常规状态，拦了就是「manifest 不一致 → 不许发布 → 无法通过发布修 manifest」的死循环（2026-10-05 真机踩过）。**严格的逐条核对移到发布事务定稿之后、commit 之前**（§7.1 的 3½ 步） | ✅ 第二刀 / 2026-10-05 改两层 |
| ⑮ | `git/clean` | Git 工作区状态正确（无未提交的无关改动 / 在正确分支） | ✅ 第二刀（★ `presets/delivery/` **排除在外**） |
| ⑯ | `client/url-reconciliation` | **客户端视角 URL 对账**：从 `workbench/bootstrap.json` 反推交付根，用生产 Resolver 把 catalog 全量条目 + catalog/release/manifest/content 逐条拼 URL，折回仓库相对路径与磁盘对账 —— 跨端锚点错位（`dist/dist` 事故）在发布前就红 | ✅ 2026-10-05 寻址改造（纯读不发网络；Golden Fixture 在 `runtime::resolver::tests`） |

**⑮ 为什么把 `presets/delivery/` 排除在外**：`dist/` 就是这次要提交的产物本身 ——
拿它的未跟踪状态去拦自己的发布是个死锁（与 `scripts/publish-presets.mjs` 同一条口径）。
**`dist/` 之外**有任何改动、或不在分支上，都拦发布。

**⑥ 与 ⑦ 的区别**（重要）：⑥ 管"该生成的生成了"，⑦ 管"登记的每一条都取得到"。
A 类资产**不进 dist**，所以 ⑦ 的判定必须走**"发布根 + `catalog.path`"**，不能走 dist：
**这正是本次架构变更后，⑦ 必须重写的原因**（旧的 ⑦ 是拿 dist 对，新 ⑦ 是拿发布根对）。

### 5.3 `minVersion` / 结构代次（机器算签名 + 显式规则表）✅ 第三刀 2026-10-04 落地

**核心原则**（作者定的）：**不让程序猜"这次改动破不破坏兼容"** —— 那永远是产品判断。

```text
① 算【结构签名】—— 机器真值（`runtime::structure`）
   签名只收「客户端读不动的那些结构事实」：**必填字段的模板路径 + 它的 JSON 形态**。
   "必填"从类型真值探出来（不靠人维护字段表）：
       删掉某个字段再解析一遍 —— 还成功 = 有 #[serde(default)]（可选）；
                                  失败 = 必填，进签名。
   ★ 关键性质（判据钉着）：**只加可选字段 → 签名不变**；加必填字段 / 可选改必填 /
     字段类型变 → 签名变。签名与"拿哪一份数据去探"无关（两份真实来源各算一遍，相等）。

   机器看不出来的那半（路径语义 / asset kind 语义 / 客户端读取方式 / 数据的解释方式）
   由 **`STRUCTURE_EPOCH`** 记 —— 那种改动发生时人工 +1，于是一定会逼出一条新登记。

② 查【显式规则表】（人工维护，进仓库）：`presets/structure-signatures.toml`
   ```toml
   [[signature]]
   signature = 'cb1080919d39b2bd'   # 第一代（落点 = 发布根 presets/）
   minClient = '0.0.1'
   note = '...'
   ```

③ 发布闸 ⑫（Blocker）：
   算签名 → 查表
   ├─ 查到 → Green，把「最低正式客户端版本」写进 `catalog.minClientVersion`
   │         （manifest 的 `minimumClient` 也用同一格 —— **一个来源**）
   └─ 查不到 → ★ **Fail、禁止发布**：把该登记的那一行直接印在详情里
              （人只需想清楚 `minClient` 填什么，不必猜格式）

④ 客户端能不能读 —— **先看结构能力，再看版本**：
      能读 = 签名 ∈ 本构建的 `SUPPORTED_SIGNATURES`   ← 「这个构建有读它的代码」
             OR 本构建版本 >= 该数据登记的 `minClientVersion`   ← 规则表的产品声明
```

**④ 里那条 `OR` 是为 Dev 场景留的缝**（作者点的）：本地代码已经支持结构 B，但正式安装包
还是 0.7.0 —— 此时规则表里写 `minClient = 0.8.0`（**不要求那个版本已经发布**），
而 Dev 构建靠 `SUPPORTED_SIGNATURES` 放行，**不看版本号**，于是不会把自己锁死。
正式 0.7.0 两段都不满足 → 拒绝使用新数据 + 「有新版 SupportEase」。

> 本次（第一刀）**就是一次真实的结构变更**：落点从 `mkp/<kind>/` 改成发布根基准（第二代：`delivery/` 改名，2026-10-05，见 `RESOURCE-ADDRESSING-ROADMAP.md`）。
> 字段名与类型一个都没动，所以是 `STRUCTURE_EPOCH = 1` 把它记下来的；
> 规则表里那一条的 `minClient` 写 `0.0.1` —— 事实是**这个项目还没发布过任何正式客户端**
> （三处版本号都是 0.0.1），所以"读得懂这一代的最老正式版"就是它。

#### 5.3.1 两条链：**软件版本**与**数据兼容**不许合并（第三刀下半，2026-10-04 作者定死）

③④ 那套判的是「**这批数据我读不读得懂**」。它**不等于**「有没有新版本的 SupportEase 这个软件」。
两者混成一句话，用户就既不知道"是不是该等更新"，也不知道"要不要升级客户端"。于是下半拆成两条**独立**链：

```text
链一【数据兼容】  catalog.structureSignature ──▶ structure::can_read ──▶ readable
                 （读不懂 = NOT_SUPPORTED）        ↓
                 apply_remote_update 落盘前拦 ──▶ 预设页「此预设需要更新版 SupportEase」+ 去更新

链二【软件版本】  release.json ──▶ release_info::to_update ──▶ 设置页「有新版本 SupportEase」
```

**链二的`release.json`语义**（作者 2026-10-04 定死）：

```text
仓库
└── presets/
    └── delivery/     ← 交付边界：catalog.json / manifest.json / source.json
        │               / content/ / mkp/presets/
        └── release.json ← 软件发布信息 {version, notes, url}（2026-10-05 起住这里，
                           由 Source Manifest 的 release 声明；仓库根与 presets/ 根
                           **不许**再出现第二份 —— `runtime::release_info` 钉位测试守着）
```

- 它是**软件发布信息，不是预设数据**：**不参与发布闸的预设数据内容校验，也不进
  catalog.json / manifest.json**；属于后面的**软件发布阶段**。
- 版本号**与 `Cargo.toml` 同源**（`CARGO_PKG_VERSION`）—— "软件版本"全局只有一处真值。
- **今天不接 GitHub Releases**：先把 `Workbench → PR → main → release.json → 客户端设置页`
  这条链跑通；将来换成 GitHub Releases **只换消费层数据源**（`runtime::net` 换一个取法），
  不动预设发布架构。
- 客户端**唯一用户入口 = 设置页**（「软件更新」块）。预设页**不主动宣传**
  （列表照常、不整表标红、无"不兼容"列），只在真遇到读不懂时出现那一句 + 「去更新」。

**链一的错误分档**：读不懂返回**专门业务错误** `NOT_SUPPORTED`（新增第九档）——
它与 `CORRUPTED`（内容坏了）/ `INTERNAL`（程序出错）**严格分开**：那两档用户要做的是
"重下 / 重试"，这一档要做的是"**去升级客户端**"。`NOT_SUPPORTED` 的 message 只写用户能懂的话，
**不含**结构签名 / minClient / schema（技术细节进 `detail`，桌面日志可查）。

### 5.4 发布闸**不是生成器**

`wb_generate` / `wb_publish` 将来应该**收进内部实现**，对外只暴露一个【发布】。
发布系统自己决定"要不要生成 / 要不要更新 catalog / 有没有结构升级"。
用户（你）看到的只有一个按钮 + 一个逐项打勾的模态框。

### 5.5 实现在哪 —— 「一个入口」落在哪一层（2026-10-04 第二刀落地）

```text
界面点【发布】 → PublishGateModal（**只画**）
                      │  invoke('wb_publish_audit')
                      ▼
                 wb_publish_audit（命令壳：with_ctx + trace，三行）
                      │
                      ▼
app::audit::publish_audit()   ← ★ 唯一判定函数；`cargo test` 判据也直接调它
                      │
                      ▼
              PublishAudit { items: [十六个 AuditItem], can_publish }
```

- **界面不许自己再实现一套检查** —— 那正是"发布闸"之前散落各处的老病根（两边各算一遍、各自看着都对）。
  判据 `publish_audit_all_blockers_pass` 咬的正是"壳与核心是同一个判定"。
- **闸是纯读的**：不写盘、不动 git、不发网络请求（`dist_root_path` / `assets_root_path` 都**不建目录**），
  所以「重新检查」可以反复点。
- ★ **锁的坑（留给将来的自己）**：闸本身跑在 `with_ctx` 里，而 `with_ctx` 的锁**不可重入**。
  闸里要复用任何"预演 / 渲染"能力，**必须调那个收 `&Ctx` 的自由函数**（如 `build::preview_with`），
  **绝不能调命令壳** —— 那是自锁（挂死，不是报错）。
- `Skipped` 是**刻意留的一档**（界面画成虚线灰，不画成绿勾：把"没实现"伪装成"通过"
  比红色更危险）。**第三刀之后十五项都用不上它了**（⑫ 曾经是最后一项；2026-10-05 新增的 ⑯ 也真跑）——
  留着这一档是因为那条规矩还要用：将来加一项还没实现的检查时，它必须显式 `Skipped`。
  判据 `the_gate_lists_every_item_and_only_opens_when_no_blocker_fails` 里那条
  `skipped.is_empty()` 就是"哪天有新项没实现"的报警器。
- ★ **⑫ 的「逼登记」是怎么咬住人的**（两个方向各一条判据）：
  `cargo test` 里 `supported_signatures_cover_the_current_structure` 与
  `the_rule_table_registers_the_current_signature` 会在结构签名变化时立刻红，
  **并把该抄的值直接印出来**；发布闸 ⑫ 再在发布那一刻拦一道（即使有人没跑测试）。
  两道各自独立，谁漏了另一道还挡得住。

---

## 6. 判据（写给将来的自己）

| # | 判据 | 咬什么 |
| --- | --- | --- |
| 1 | **`catalog_path_is_relative_to_the_publish_root`** | `catalog.path` 以 `assets/` 或 `delivery/` 开头；锚点由 Source Manifest v2 的 `filesRoot` **声明**（不再由目录推导）—— 见闸⑯与 Golden Fixture |
| 2 | **`no_a_class_assets_in_dist`** | `presets/delivery/` 下不许出现 `bbs/` `icons/` `models/` `printers/` —— 停止制造第二份真源 |
| 3 | **`every_registered_path_is_reachable`**（新 ⑦） | catalog 每条 `path`，在「发布根 + path」处真能取到（A 类查 `presets/`，B 类查 `delivery/`） |
| 4 | **`publish_audit_all_blockers_pass`** | `PublishAudit` 全绿才 `can_publish` |
| 5 | **`bootstrap_request_bypasses_cdn_cache`** | `source.json` 请求带 cache-busting（CDN 缓存会让新发布延迟生效） |
| 6 | `embedded_matches_rebuild`（既有） | 随包目录 == `gen-catalog` 重建 |
| 7 | `startup_never_overwrites_an_ota_catalog`（既有） | 启动不覆盖 OTA 目录 |
| 8 | 切片器档无幽灵行（既有） | `version_files_dto` 的类型分流 |
| 9 | **`structure_signature_stable_for_optional_additions`** | 只加可选字段时签名不变（防止签名过度敏感、天天要跳 minVersion） |
| 10 | **`client_url_reconciliation`（闸⑯，2026-10-05）** | 用生产 Resolver 从 `workbench/bootstrap.json` 反推交付根，catalog 全量条目 + catalog/release/manifest/content 逐条拼 URL 折回仓库对账 —— `dist/dist` 那类跨端锚点错位在发布前就红 |
| 11 | **`golden_addresses_match_the_address_table`（Golden Fixture，2026-10-05）** | 全家族 × GitHub/Gitee × 本地落点的期望地址写成字面量；锚点/声明/布局任何改动，第一个看到的就是这张表的可读 diff |
| 12 | **交付面无绝对 URL**（CI 负向断言） | source.json 是 GitHub/Gitee 双镜像共用的寻址规则声明，写死任何一家的绝对地址 = 把另一家镜像的用户指回去 |
| 13 | **`the_manifest_covers_exactly_the_referenced_deliverable_set`**（2026-10-05） | manifest == 引用可达交付集（B 类全部 + 有落点可达资产），manifest ⊆ catalog；差集只许是"登记了但无引用"的资产（§4.5）—— 差集语义从注释升格为判据 |

---

## 7. 落地顺序（三刀，各自独立可验收）

| 刀 | 内容 | 验收 |
| --- | --- | --- |
| **第一刀：路径语义切换** ✅ | `dest_of_asset` 改为"发布根基准"；A 类不再复制进 dist；清掉 `dist/mkp/{bbs,icons,models,printers}`；客户端落点规则跟着改（§3.2 **裁甲**） | 判据 1/2/3 绿；客户端下载 0.2mm BBS 成功 |
| **第二刀：发布闸** ✅ | `app::audit::publish_audit`（十五项 → 2026-10-05 起**十六项**，**唯一判定函数**）+ 命令壳 `wb_publish_audit` + `PublishGateModal`（逐项打勾）。「创建 PR」**留到第三刀** —— 今天不摆点不动的假按钮 | 判据 4 绿；模态框截图（`tmp-shots/wb-publish-gate*.png`） |
| **第三刀（上半）：结构签名 + minVersion** ✅ | `runtime::structure`（签名从类型真值算 + `STRUCTURE_EPOCH` 记语义变化）+ `presets/structure-signatures.toml` 规则表 + ⑫ 真跑（查不到 = Blocker）+ 写进 `catalog.minClientVersion` / manifest | ⑫ 不再有 `Skipped`；`an_unregistered_structure_generation_blocks_the_publish` 绿 |
| **第三刀（下半）· 不兼容链** ✅ 2026-10-04 | 客户端**先比再下**：`update::check` 出 `readable`（能力优先、版本兜底），`apply_remote_update` **在落盘前**拒读不懂的目录并返回新档 `NOT_SUPPORTED`；预设页只在"远端这一代读不懂"时出现「此预设需要更新版 SupportEase」+「去更新」（**列表照常、不整表标红**） | 判据 `an_unreadable_remote_catalog_is_refused_not_applied` / `the_update_check_reports_readability` / `not_supported_is_its_own_error_code` 绿 |
| **第三刀（下半）· 软件更新链** ✅ 2026-10-04 | 新开**独立**信息源 `release.json`（住发布根 `presets/` **之外**、不进 catalog/manifest/发布闸）→ `runtime::release_info` → 设置页「软件更新」块（「有新版本 SupportEase」/「已是最新版本」）。与上面那条**两条链不合并** | 判据 `release_json_is_its_own_source_not_preset_data` / `software_version_compares_by_semver_and_ignores_dev_suffix` 绿 |
| **第三刀（下半）· 发布事务** ✅ 2026-10-04 | `app::publish_tx`（**锁无关内核**）：审计 → 生成 → 定稿 → 本地 git（白名单 stage/commit/push）→ 平台 PR/MR；`wb_publish` 是**唯一对外的发布动作**，`wb_generate` / `wb_publish_audit` 降为内部步骤；`wb_publish_account` / `wb_set_publish_token` / `wb_clear_publish_token` / `wb_publish_status`（手动回读）。前端只剩一个「发布」 | 判据 `the_transaction_chain_never_calls_a_command_shell` / `stage_paths_are_an_explicit_allowlist` / `detect_platform_maps_remote_urls` / `remote_state_collapses_platform_dialects` / `credentials_never_echo_the_token` 绿 |

**三刀之间有依赖**：第一刀是其余两刀的地基（路径语义不定，发布闸无从检查）。
**第一刀里那个"客户端落点"的取舍（方案甲/乙）必须最先定。**

### 7.1 发布事务 = 本地 git + 自持平台认证 + 统一状态模型（第三刀下半，2026-10-04 作者定死）

一次「发布」= 一条**事务**，用户只点一次：

```text
【发布】
   ↓
PublishAudit        （任一 Blocker 红 → 停在审计，零写入）
   ↓ 全绿
内部 generate       （wb_generate 降为内部步骤：写 dist/mkp/presets/ + 重算目录）
   ↓
publish_into        （定稿 catalog / manifest / source）
   ↓
finalize_consistency（3½ 最终一致性核对：定稿刚写下的三本账对真字节、无残留；
   ↓                  红 = 内部错误或并发改动 → Err 短路，**不 stage、不 commit**）
本地 git            （子进程 git：白名单 stage → commit → push）
   ↓
平台 API            （GitHub / Gitee：create PR/MR）
   ↓
回读状态            （快照 + 手动刷新；**不做后台轮询**）
   ↓
用户去平台网页合并   （**合并留给平台**，工作台不做）
```

**两层检查的裁定**（2026-10-05）：预检 ⑭ `manifest/correct` 是**提示档**（Warning）——
manifest 是上一版发布写下的账本，落后于 dist 只说明"生成过、还没发"；**交付账本的
最终一致由事务自己在 3½ 步断言**（manifest 严格版 + catalog/sha-size + no-strays，
刻意不含 ⑮ git/clean —— 定稿后的工作区理应带着 `presets/delivery/` 的改动）。
预检 ⑮ `git/clean` 的口径不变：源状态必须可追溯，`presets/delivery/` 之外必须干净。

**三条边界**（守住它，这刀才不是"给开发者包了一层 CLI"）：

1. **前端只有一个「发布」入口**：`wb_generate` / 「创建 PR」都**不再是用户动作**，
   是事务的内部步骤。摆四个按钮的那一刻，"发布"就退化成开发工具了。

2. **本地 git 用子进程，远程平台认证用我们自己的 HTTP**：
   - 本地：`workbench/app/git.rs`，`std::process::Command::new("git")`，**参数显式数组**、
     **只 stage 白名单路径**（`STAGE_ALLOWLIST`），永不 `git add -A`；
   - 远程：`workbench/app/platform/{github,gitee}.rs`，**SupportEase 自持 Token**，
     **不借用户的 `gh` / `git` 登录态**（用户不该为了发布先装好 GitHub CLI）。

3. **凭据住本机凭据文件，前端拿不到**（2026-10-05 从系统 Keychain 改判：无签名分发下
   Keychain 的免弹窗授权不成立 —— 签名一变就当陌生 App，反复要登录密码；无签名软件
   存秘密的业界惯例就是 0600 文件，npm / gh / AWS CLI 同款，完整理由见
   `credentials.rs` 头部）：`workbench/app/credentials.rs`，每平台一份
   （`supportease.github.token` / `supportease.gitee.token`），住
   `<appDataDir>/credentials.json`（0600、`atomic_write`、**坏档当"没存"** 不炸设置页），
   **绝不**写 localStorage / .env / 会被网盘同步的目录；前端只知道"**配没配**"+ 一个
   尾号提示（判据 `credentials_never_echo_the_token`）。
   ★ 2026-10-07 作者补了一条**显式**出口：设置页那颗「眼睛」调 `wb_get_publish_token`
   取明文（换机器 / 重配时要看得见、要能复制）。状态面**形状一个字段没变** —— 这条出口
   只在人点眼睛那一刻才走，不是开场自动读、也不进发布链。

4. ★ **锁与线程边界**（2026-10-04 真机事故后补的硬规矩 —— 点一次发布，窗口直接挂死）：
   - **`wb_publish` 必须 `#[tauri::command(async)]`**：它起 git 子进程、发平台 HTTP，
     **跑在主线程上就是整个窗口一动不动**（那次事故里它还在主线程读 Keychain 弹密码框；
     凭据 2026-10-05 起改住本地文件，等网络这条不变）。凡"碰网络 / 凭据盘"的命令同理 ——
     它们在 `read_commands_are_async_so_they_never_freeze_the_window`
     的 `IO` 单子上，忘了 `(async)` 就红。
   - **事务内核只许调锁无关自由函数**：`with_ctx` 的锁**不可重入** —— 事务里回头调
     `audit::publish_audit()`（它自己会 `with_ctx`）就是**自锁挂死**（不是报错）。
     审计因此拆成 `audit_with(ctx)`（锁无关内核）+ `publish_audit()`（入口薄壳），
     与 `build::preview_with` / `generate_with` 同形；判据
     `the_transaction_chain_never_calls_a_command_shell` 把"**取锁入口**"和命令壳一起拦。
   - **平台 HTTP 一律走 `platform::agent()`**（带 `API_TIMEOUT=30s` / `API_CONNECT_TIMEOUT=10s`）——
     **没有超时就没有尽头**；发布链上的网络调用不许用裸 `ureq::get/post`。

5. **回执与历史**（2026-10-04，发布事务收尾那一刀）：
   - **回执** = `PublishTxReport` 的阶段快照；界面（`PublishGateModal` 的第二段视图）只画不判 ——
     阶段链、PR 地址、CI 档位全来自后端。PR 地址交给系统浏览器走 `wb_open_external`
     （**只放行 `http(s)`** —— 那个口子只该开这么窄）。
   - **历史** = `<appDataDir>/publish-history.json`（`app/history.rs`）：与 `publish-account.json`
     同形（`*Schema` 代次 + `atomic_write` + 坏档 `CORRUPTED` 不静默；缺文件 = 空）。
     **它不是配置** —— 删掉只丢展示，不影响发布能力。写入点在**壳层**（`wb_publish` 收尾写一条），
     因为写它要 `AppHandle` 拿 appDataDir；内核 `publish_tx::run` **不碰 AppHandle**（见上面第 2 条）。
   - **合并** = `Hosting::merge_review`（两个平台同形：`PUT /repos/{o}/{r}/pulls/{n}/merge`）：
     一律 **squash**、**不强制等 CI**（口径源头 = `RELEASE-TRANSACTIONS.md` §1.1 第 8 步）；
     **只在人显式点过之后调**，合完**回读**一份真状态。
   - **Token 会话缓存** = `credentials::session()`（进程一份；`CachedStore` 包着 `FileStore`）：
     一次程序运行**至多读一次**凭据文件，`set` / `clear` 同步失效；**不改文件的存储方式与内容**。

**平台抽象与统一状态模型**（`workbench/app/platform/mod.rs`）：

- `trait Hosting` = 平台同一张脸：`create_review` / `get_review`；
- **平台选择从 git remote 推断**（`detect_platform`：`github.com` → github，`gitee.com` → gitee，
  认不出 = `None`，要求用户在设置里选）；
- **方言收敛**（`collapse_state` / `collapse_checks`）：GitHub 的 PR + check runs 与 Gitee 的
  MR + status 各自的多档 → 统一的 `ReviewState { open, merged, closed, unknown }` 与
  `ChecksSummary { pending, passed, failed, none, unknown }`。**前端不认识任何一个平台方言。**

**网络出处**：本刀**新开了第二个被批准的网络面**（`workbench/app/platform/`）——
`scripts/check-zero-network.mjs` 第①道闸随之从"网络只住 `runtime/net.rs`"放宽为
"网络只住 `runtime/net.rs` **与** `workbench/app/platform/`"。★ 边界因此更硬：
**发布相关的 HTTP 只许加在 `platform/` 内**，别处一律不许。

**锁的边界**（★ 与 §5.5 同一个坑）：事务跑在 `with_ctx` 里，而 `with_ctx` **不可重入** ——
事务内核（`publish_tx::run`）**只收 `&Ctx`**，复用 `generate_with` / `publish_audit` /
`publish_into` 这些自由函数，**绝不回头调命令壳**（那是自锁挂死，不是报错）。
判据 `the_transaction_chain_never_calls_a_command_shell` 用源码扫描钉住。

**Gitee 的落地程度**：本刀 GitHub 与 Gitee 都实现了（`create_review` / `get_review` / 认证）。
Gitee 的接口细节**按公开 API 实现**；作者说后续会给旧版配置地址，届时单独做一次
"旧版配置 → 新版配置"的事实核对（**不翻旧代码猜**）。平台差异在各自 impl 内消化，
统一状态模型在前端之外 —— 若 Gitee 字段名要对，只改 `gitee.rs` 一个文件。

### 7.2 发布账户配置 + Git 认证自持（2026-10-04，作者定死）

**发布账户（配置与秘密分离）**：

```text
<appDataDir>/publish-account.json      ← 配置：每平台 repositoryUrl + username（**无 token / 无 email**）
<appDataDir>/credentials.json（0600）  ← 秘密：supportease.github.token / supportease.gitee.token
```

- 每平台一格，**GitHub / Gitee 完全对称**；设置页三格 = 仓库地址 + 用户名 + Token。
- ★ **配置文件里绝不出现 token 字段**（判据 `publish_account_config_never_stores_a_token`）；
  Token 住凭据文件，前端只拿得到 `hasToken` + 尾号（判据 `credentials_never_echo_the_token`）。
- ★ 发布目标（平台 / owner / repo）**由这份配置决定**，`git remote` 降级为**校验**
  （当前工作目录是不是配置的那个仓库，见 `git::Git::remote_matches`），不是就如实拒绝。

**★ 取代一条旧原则**：上一轮定的"不为平台新增配置文件"**从此作废** ——
`repositoryUrl` / `username` 是机器本地的发布配置（不进仓库、不含密钥），必须有个落点。

**Git 认证自持**（不拼 URL、不借用户登录态）：

```text
git -c credential.helper= \
    -c http.extraHeader="Authorization: Basic <base64(username:token)>" \
    push -u origin <branch>
```

- ★ **Token 不进 remote URL**（`git remote -v` / `.git/config` / push 回显里都没有）；
- ★ **`credential.helper=`（空值）清掉全局 helper** —— git **不会**去读用户系统里已存的凭据；
- ★ 子进程环境 `GIT_TERMINAL_PROMPT=0`（禁交互，防挂死）；失败 detail **不带**那条 header 参数；
- base64 **手写**（`git.rs::base64_encode`），不为此引 crate。
- 旧版 mkppanel 把 token 拼进 HTTPS URL（`buildAuthURL`）—— **那是旧方案，不继承**。

---

## 8. 与既有文档的关系（防止规则散落）

| 文档 | 管什么 | 本文与它的关系 |
| --- | --- | --- |
| `DATA-ARCHITECTURE.md` | 四层 / 四铁律 / 归属 | **上游**。本文是它在"发布链"上的展开 |
| `PUBLISH-ARCHITECTURE.md`（本文） | 发布链 / 路径语义 / 发布闸 / minVersion / PR | **本主题的根规则** |
| `RELEASE-TRANSACTIONS.md` | **两层事务的语义与边界**（发布预设 vs 发布软件版本） | **互补**：本文管"发布链怎么实现"，它管"两层事务分别是什么、变更怎么分类" |
| `PRESET-DELIVERY-CONTRACT.md` | 路径三层 / 幽灵行 / 404 定位 | **降级为"问题档案"**：§3 的 A/B 已废、§5 已被本文 §5 取代；保留 §1/§2 的定位过程 |
| `PRESET-PRODUCT-RULES.md` | 三状态 / SHA / 归档 / 状态流转 | **互补**：管客户端侧行为，本文管发布侧 |
| `HANDOFF.md` | 进度与台账 | 只记"做到哪了"，规则一律指向本文 |
