> ⚠️ **历史快照（2026-10-05 加注）**：本文写于寻址改造之前，其中的 `presets/dist/`、
> `baseUrl` 同目录推导、release.json 位置等表述是**当时的现状记录**，不再是现行规则。
> 现行规则见 `docs/RESOURCE-ADDRESSING-ROADMAP.md` 与 `docs/PUBLISH-ARCHITECTURE.md` §2.2。

# 预设交付契约（问题档案）

> **状态：部分已被取代。** 2026-10-04 作者定下三条裁决后，发布链的**正式规则**搬进了
> **[`PUBLISH-ARCHITECTURE.md`](./PUBLISH-ARCHITECTURE.md)**。
>
> 本文保留的价值：**§1 路径三层的定位过程**、**§2 幽灵行的成因**、**§3 404 的实测对照** ——
> 那是"问题怎么被查出来的"的档案。
> 已废：§3.1 的 A/B 裁决（→ 该文档 §1.4）、§5 的旧发布闸设计（→ 该文档 §5）。
>
> 起草：2026-10-04。上游根规则：`docs/DATA-ARCHITECTURE.md`（四层 / 四铁律）。

---

## 0. 一句话结论（先看这个）

> **路径不是硬编码。** 它是 `mkp/<kind 目录>/<资产在台账里的 path 去掉第一段>`，
> 由**唯一一处算法** `runtime::catalog::dest_of_asset` 算出，**发布侧与客户端共用同一处**。
> 你截图里那条 404 的**真因不是路径算错**（路径算得对），而是**登记面比实体面宽**了 ——
> 目录登记了 4 份 0.2mm 的 BBS，但发布时**没有把文件复制进交付根**。

---

## 0.5 四条主链与当前状态（2026-10-04 收口后的全景）

先把"预设这个东西的一生"摊成四条链。**这四条是本项目第三圈的全部内容**，
前两条（下载 / 修改）已收口，后两条（发布 / 演进）还缺关键闸门。

```text
① 客户端 → 云端（下载链）        ✅ 收口（2026-10-04）
   启动铺 bootstrap → 进预设页 OTA 一次 → 当前 catalog → 下载 + SHA 校验
   契约见 §3 / DATA-ARCHITECTURE §1③

② 客户端 → 用户（使用链）        ✅ 收口（第三圈 + 12 层）
   下载 → 本地文件 → 应用（run/active-preset.json）→ 改（草稿）→ 另存（presets-mine/）
   契约见 PRESET-PRODUCT-RULES.md

③ 工作台 → 客户端（发布链）      ⚠️ 半收口 —— 见 §4 / §5
   工作台生成 → dist/ → 入库 → PR → 合并
   缺：登记面 == 实体面（§3）、统一发布闸（§5.1）、工作台不感知 PR 状态（§5.3）

④ 客户端 ↔ 版本（演进链）        ❌ 未收口 —— 见 §5.2
   catalogSchema（加字段不升号）→ 客户端读不懂就拒
   缺：minVersion（结构变了 → 最小客户端版本跟着变 → 智能判定）
```

**一句话读法**：①② 是"一个已发布的东西怎么被用"，它们的问题都由 2026-10-04 这一轮修掉了；
③④ 是"一个新东西怎么被发布、怎么被旧客户端消化"，它们是**接下来两刀**的事。

---

## 1. 路径是谁决定的（三层，自上而下）

```text
presets/assets.toml            ← 台账：登记 id / kind / path(源相对路径)
        │
        │  dest_of_asset(asset)          ★ 唯一一处算法
        │   = "mkp/" + kind_dir(kind) + "/" + (path 去掉第一段)
        ▼
CatalogFile.path               ← 落点（相对内部根 / 相对交付根）
        │
        ├─ 客户端：下载地址 = 数据源 baseUrl + path      （runtime/net.rs）
        └─ 工作台：复制到 dist/<path>                    （workbench/app/dist.rs）
```

| 谁 | 代码 | 作用 |
| --- | --- | --- |
| **台账** | `presets/assets.toml` | 登记 `id` / `kind` / `path`（如 `bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json`） |
| **★ 唯一算法** | `runtime::catalog::dest_of_asset`（`runtime/catalog.rs:223`） | 台账 `path` → 交付落点。两端共用，**不许第二次拼接** |
| **客户端** | `net.rs::RemoteSource::fetch` | URL = `baseUrl` + `path` |
| **工作台** | `dist.rs::write_content` | 复制到 `dist_root.join(dest_of_asset(a))` |

**`kind_dir(kind)` 的映射**（`runtime/catalog.rs`）：

| 资产 kind | 落点目录 |
| --- | --- |
| `mkp_preset`（MKP 预设） | `mkp/presets/`（产物名由命名规则算，**不写 path**） |
| `bbs_config`（切片器配置） | `mkp/bbs/` |
| `model`（3mf） | `mkp/models/` |
| `icon` | `mkp/icons/` |
| `image`（图） | **没有落点**（`None`）——图片走随包 `delivery='bundled'` |

**两条拒绝规则**（`dest_of_asset` 返回 `None`）：
1. `delivery == bundled`（随包资产，如品牌图 / 整机图）——登记但**不下载、不进交付根 URL 面**；
2. 该 kind 在下载区没有落点（今天的 `image`）。

> 所以：**如果你在台账里登记了一条 `download` 档的资产、给了 path，它的下载地址就自动成立了。**
> 没有任何地方需要"再写一遍路径"。

---

## 2. `presets/` 幽灵行（2026-10-04 已修）

**现象**：切片器档出现 5 行主名为空、副行写着 `presets/` 的行（每台机型一行）。

**真身**：套餐 `assetRefs` 是**混合**的 —— 里面既有切片器配置，也有 **MKP 预设**那条，
而 MKP 预设的 `path` 是**空串**（它是"产物"，靠 machineId+versionId 定位）。

**错误的传播链**（`ipc/presets.rs`）：

```text
version_files_dto 的「切片器一支」直接摊 bundle.asset_refs（不按类型分流）
   → file_kind(a)：只看 a.slicer，None 一律 `_ => "bbs_profile"`（兜底把 MKP 预设也贴成切片器文件）
   → file_name_of("") = ""                      （主名是空串）
   → format!("presets/{}", "") = "presets/"     （副行小字，看着像个文件夹）
   → 同机型多版本按 path 去重 ⇒ 每台机型 1 行，共 5 行
```

**修法**：`file_kind` 改返回 `Option`（**认不出就说认不出**，去掉兜底）；切片器一支跳过
`MkPreset` 与空 `path`；`preset_files_dto` 那处 `expect` 改成显式不可能分支。

**教训**：**兜底分支会把"另一种东西"静默贴成"这一种"**。类型分流处不许有 `_ =>`。

---

## 3. 「远端还没有这份文件」（2026-10-04，**未修**）

**现象**：点下载 → 提示
`下载失败：远端还没有这份文件: …/presets/dist/mkp/bbs/Process/0.2mm/MKPProcess%20A1%20200.2%200.10.json`

**不是路径算错**。实测三方对照：

| 位置 | 0.2mm 的 4 份 |
| --- | --- |
| 台账 `presets/assets.toml` | ✅ 登记了 |
| 源 `presets/assets/bbs/Process/0.2mm/` | ✅ 文件在 |
| **交付根 `presets/dist/catalog.json`** | ✅ **登记了 4 条**（带 SHA / size） |
| **交付根 `presets/dist/mkp/bbs/Process/0.2mm/`** | ❌ **目录根本不存在** |
| 客户端 `Internal/catalog.json` | ✅ 拿到那 4 条（OTA 后）→ 按 URL 去取 → **404** |

**真因：登记面 ≠ 实体面。**

| 面 | 由谁决定 | 范围 |
| --- | --- | --- |
| **登记面**（`catalog.json.files[]`） | `Catalog::build_from_presets_lenient` | **全部**有落点的台账资产（2026-10-03 起 `delivery='download'` 的都进） |
| **实体面**（真的复制进 dist 的） | `dist.rs::referenced_assets` | **只有**「机型 image/icon + **套餐 `assetRefs`**」 |

0.2mm 那 4 份**没有被任何套餐引用** ⇒ 没进 `referenced_assets` ⇒ 文件没复制；
但登记面照收 ⇒ 客户端看得见、点得动、**必然 404**。

`dist.rs:795-798` 的注释把这件事写成了"设计"：

> 目录的登记面**刻意比交付面宽**（让客户端认识所有资产；没被版本 / 套餐引用的资产「登记但不发」）

**这句话在"客户端只看不下载"的前提下成立，但现在客户端把它们渲染成了可点的下载按钮** ——
于是"登记但不发"从设计变成了**点不动的死按钮**。

### 3.1 修法：**A/B 已废，正确解是 C**（2026-10-04 作者裁决）

| 方案 | 做法 | 状态 |
| --- | --- | --- |
| ~~A. 实体面补齐~~（把文件复制进 `dist/mkp/bbs/`） | 制造第二份真源 | ❌ **废** |
| ~~B. 登记面收窄~~（不登记这些资产） | 客户端丢失"认识所有资产"的能力 | ❌ **废** |
| **★ C. 路径指向真实资产源路径** | `catalog.path = "assets/bbs/…"`，A 类资产**不复制**；发布根 = `presets/` | ✅ **采纳** |

A/B 共同的前提（"`catalog.path` 必须指向 `dist/mkp/…`"）本身就是错的。
**C 定下来之后，"登记面 ≠ 实体面"这个说法自动消失** —— 登记面与实体面是同一个面：
`catalog.path` 指哪，哪就必须在。

> **完整规则见 [`PUBLISH-ARCHITECTURE.md`](./PUBLISH-ARCHITECTURE.md)**（本文 §3 保留为问题定位档案，
> §5 的旧发布闸设计已被该文档 §5 取代）。

---

## 4. 发布到底做了什么（现状，逐条）

**入口**：工作台「生成与发布」页 → 点「生成」→ `GenerateDiffModal` 确认 → `wb_generate` → `wb_publish`。
**另有一条纯命令行**：`npm run publish:presets`（五道校验 → 一次性分支 → PR）。

```text
① 生成 wb_generate
   ├─ 渲染 MKP TOML 产物 → 写 dist/mkp/presets/*.toml
   ├─ 资产补齐（referenced_assets → 复制）
   └─ 重算 catalog.json（write_catalog_json）
② 发布 wb_publish（dist.rs::publish_into）
   ├─ 交付集合 deliverable_set(book)  ← 引用可达
   ├─ 逐份核对「集合里的文件真的在 dist 根」（audit）
   ├─ 写 manifest.json + source.json
   └─ catalog.json 收尾
③ 主仓入库 presets/dist/（**这一步是 git 提交，人在 IDE 里做**）
④ npm run publish:presets → 一次性分支 → PR → 合并
```

### 4.1 你问的"到底走 PR 还是本地"

**现状是两半，中间靠人接**：

| 环节 | 谁做 | 在哪 |
| --- | --- | --- |
| 生成 + 发布（写 `presets/dist/`） | **工作台**（按钮） | 本机 |
| `presets/dist/` **入库**（git commit） | **你**（手动） | IDE / 终端 |
| 推分支 + 开 PR | `npm run publish:presets` **或**你手动 | 终端 |
| 合并 PR | **你**（手动点） | GitHub |
| 工作台感知"PR 已合并" | **没有** —— 工作台不知道 PR 的状态 | — |

**所以现在的答案是：工作台只负责"把菜做好"，上菜（提交 / 推 / PR / 合并）全在终端与 GitHub。**

---

## 5. 还缺什么（你要的"发布闸"）

你说的是对的：**现在没有一个"点一下、全绿才允许推送"的发布闸**。缺三样。

### 5.1 发布前的**逐项自检模态框**（缺 —— 建议作为下一刀的主菜）

**期望形态**（你的原话整理 + 现有检查归位）：

```text
点「发布」→ 模态框逐项跑，每项跑完就地打勾 / 打叉（跑的过程中不锁界面）

  ① 台账完整性         每条资产 id 唯一、kind 认得出、path 非空          已有零件
  ② 落点算法           dest_of_asset 每条都给得出、无 `..`、无撞车        已有判据
  ③ ★ 登记面 == 实体面  catalog 登记了 path 的，dist/ 里必须有那份文件     ❌ 不存在
  ④ 产物齐             每个"有产物的版本"，mkp/presets/<名>.toml 真在     已有（deliverable_set）
  ⑤ SHA 对真字节       发布侧目录必须登记期望值、且与文件一致              已有（audit_catalog）
  ⑥ 结构代次 / 最小版本 catalogSchema 认得、算出 minVersion（§5.2）        ❌ 不存在
  ⑦ 残留               dist 里有、交付集合里没有的（走 .trash）            已有（dist_strays）
  ⑧ 孤儿               登记了但没有任何套餐 / 机型引用的                   已有（version_orphans）

→ ⑧ 项全绿 → 才允许「推送到远端」这一步按钮变亮
→ 有任何一项红 → 就地列出是哪些文件 / 哪些条目，并给出"去修"的入口
```

**现状盘点**：这些检查**大部分零件已经存在**（`audit_catalog` / `deliverable_set` /
`dist_strays` / `version_orphans` / `wb_preflight`），缺的是三件事：

1. **`登记面 == 实体面` 这条检查本身**（第 ③ 项）—— 它不存在，0.2mm 就是从这儿漏的；
2. **一个统一的、逐项打勾的模态框**把八项串起来（现在它们是散落的命令，且 `audit_catalog`
   进预检但**只报待办、不阻断**）；
3. **"全绿才允许推送"这道闸**（现在没有任何东西阻止你在有红灯时推）。

> **落地建议**：新增一条读命令 `wb_publish_preflight()` 返回 `Vec<PreflightItem>`（每项：
> 名字 / 状态 / 明细 / 修复入口），前端一个 `PublishGateModal` 逐项渲染。**不改现有命令**，
> 只把散件聚合 —— 改动小、可增量。

### 5.2 `minVersion` / 结构代次（缺）

**现状：`minVersion` 根本不存在。** 只有 `catalogSchema: u32`（`runtime/catalog.rs:41`，当前 `= 1`），
规则是"**加字段不升号**"，客户端读到的 schema 不等于自己认识的就当错拒掉（`Catalog::parse`）。

你设想的"**结构变了 → 最小版本跟着变 → 智能判定**"这条能力**是空白**。要它成立，分四步：

**第一步：区分"内容变了"与"结构变了"**（这是智能算法的核心）。

| | 例子 | 该不该卡旧客户端 |
| --- | --- | --- |
| **内容变** | 加了一台机型 / 改了一个参数默认值 / 换了张图 | ❌ 不卡 —— 老客户端照用 |
| **结构变** | 给 `CatalogFile` 加了个**必填**字段 / 落点规则从 `mkp/<kind>` 改成别的 / 删了一个客户端会读的字段 | ✅ 卡 —— 老客户端读不懂 |
| **能力变** | 新增一种资产 kind（老客户端不认这个 kind） | ⚠️ 半卡 —— 老客户端可以选择性忽略 |

**第二步：结构指纹怎么算。** 不能拿 `revision`（那是"输入变了"，内容一变它变，太敏感）。
建议做法：**在构建期对"客户端会读的那组字段"算一份结构签名** ——
把 `CatalogFile` / `CatalogMachine` / `Bundle` / `registry.params` 的**字段名集合 + 必填性**
序列化成一个规范字符串，取 SHA。**字段只增不减、且新增都是可选时，签名不变**；
一旦出现"减字段 / 可选变必填 / 落点规则变"，签名就变。

**第三步：签名 → minVersion 的映射表。** 一张**人工维护**的表（不进智能推断）：

```toml
# docs/ 或 presets/ 里一份显式表：结构签名 → 最低客户端版本
["sha:abc123…"]  min = "0.2.0"   # 这一代结构需要 0.2.0 起的客户端
["sha:def456…"]  min = "0.1.0"   # 最初那一代
```

发布时算出当前签名 → 查表 → 写进 `catalog.json` 的 `minVersion`。
**表里没有的签名 → 发布闸红灯**（逼你显式登记，而不是默默放过）。

**第四步：客户端先比再下。** `apply_remote_update` 拿到目录后，先比
`minVersion` vs 自己的版本：不满足就**明说"这个目录需要 x.y.z 以上的客户端"**，
而不是走到下载那一步才报 404 / SHA 不匹配（今天就是后者 —— 报错在最难懂的地方）。

> **为什么"表"比"智能推断"好**：智能推断永远会漏（"这个字段改动算不算破坏性"是个产品判断）。
> 表把判断留给发布的人，算法只负责**算签名、查表、红灯**。这与你定的"发布闸逐项打勾"是同一思路。

### 5.3 工作台 ↔ PR 的状态回读（缺）

现状：工作台**不知道** PR 开没开、合没合 —— 这是你问的核心。

**完整现状时序**：

```text
你在工作台点「生成」     → 确认框 → wb_generate（写 dist/mkp/presets/*.toml + 重算 catalog.json）
你在工作台点「发布」     → wb_publish（写 manifest.json + source.json + catalog.json 收尾）
      ↓ 到这里，产物全在本机 presets/dist/，还没进 git
【空白】入库：git add presets/dist && git commit           ← 你在 IDE 做
【空白】推分支：npm run publish:presets（一次性分支）       ← 终端，或你手动
【空白】开 PR                                              ← 脚本开，或你手动
在 GitHub 点「合并」                                       ← 你手动
【空白】工作台毫无感知 —— 它不知道 PR 存在，也不知道合没合
```

**两种方向（待你裁决）**：

| | A. 工作台只到"全绿 + 一条命令" | B. 工作台直连 GitHub |
| --- | --- | --- |
| 做什么 | 跑完 §5.1 八项 → 生成一条**复制即可用**的发布命令 → 你贴到终端 | 工作台调 `gh` / GitHub API：开 PR、读回状态、能在工作台里合并 |
| 守铁律 2（云端不参与首屏） | ✅ 完全不碰网络 | ⚠️ 引入网络与凭据（**只影响工作台，不影响客户端启动**） |
| 改动量 | 小（纯前端 + 一条命令拼装） | 大（凭据管理 / 失败重试 / 状态轮询 / 错误展示） |
| 风险 | 低 | 中（token 泄漏面、网络抖动、部分成功态） |

> **我的建议：先做 A，B 单独一刀**。理由：A 已经能解掉你"不知道该怎么发布"的痛点
> （工作台明确告诉你"现在该跑哪条命令、跑完会发生什么"），而 B 的价值主要在"少切一次窗口"，
> 却要引入凭据与网络状态机 —— 那个复杂度值得单独评估，不该和发布闸挤在一刀里。

---

## 6. 判据（写给将来的自己）

| 判据 | 咬什么 |
| --- | --- |
| `dest_of_asset` 单测 | 落点算法的唯一性（两端同源） |
| **`登记面 == 实体面`**（**待补**） | 凡 `catalog.json` 登记了 path 的，`dist/` 里必须有那份文件；缺一条就红 |
| `audit_catalog` | 交付集合内逐份字节与目录登记一致 |
| `embedded_matches_rebuild` | 随包目录 == `gen-catalog` 重建（改了源必须重跑） |
| `startup_never_overwrites_an_ota_catalog` | 启动不许覆盖 OTA 目录（2026-10-04 修） |
| 切片器档无幽灵行 | `version_files_dto` 的类型分流（2026-10-04 修） |
