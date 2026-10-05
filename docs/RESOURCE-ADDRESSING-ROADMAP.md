# 资源寻址改造总纲（长期大纲）

> 状态：**大纲已批准（2026-10-05，含三条文字修订：Manifest 定位为"寻址规则声明"、filesRoot 定位为"全局锚点规则"、M2 弃用"双锚"表述）；施工未开始**（进度台账见 §8）
> 定性：这不是"修 `dist/dist`"的施工单，而是**"资源从源数据 → 生成物 → 交付 → 客户端寻址"一整条链的定型总纲**。
> 用法：每次动地址相关代码**先读本文**；每完成一阶段在 §8 打勾；发现新的漏网面就往 §6 清单里加一行。
> 依据：2026-10-05 路径语义审计（数据均实测：`origin/main = b115c73`，最新发布客户端 v0.0.5）。
> 作者决策：趁未正式发布，把发布系统地址规则一次定型，**不需要为兼容而保留 `dist`**。

---

## 0. 心智模型（先说人话）

整个系统只有两个地方，想成两个仓库：

```text
presets/          ← 【仓库】我们自己拥有的东西（人手维护的源）
│   assets/  machines/  bundles/  registry/  plates/  forbidden_zones/  …
│
└── delivery/     ← 【出货仓库】工厂生产出来、给客户端看的东西
    source.json  catalog.json  manifest.json  release.json
    content/     mkp/presets/
```

三条分工，永不混：

| 角色 | 一句话职责 | 绝对不碰的事 |
| --- | --- | --- |
| **生成器**（wb_publish / gen-catalog / release_tx） | 生产什么、放哪里 —— 唯一出口 = `presets/delivery/` | 不知道客户端怎么找 |
| **SourceResolver** | 客户端拿到 source.json 之后，每个东西到底在哪 | 不生产、不下载、不校验 |
| **业务层**（下载管道 / 预设页 / 更新 / 工作台） | 只说"我要 Catalog / Release / 这个 Entry" | **永不出现 base + path、join、format 拼 URL** |

`assets/` 为什么留在外面：它是**唯一一份原始资产**（A 类，裁决①）。复制进 delivery 就有两份真相，改一份忘一份，鬼故事重演。`delivery/mkp/presets/` 为什么在里面：它是**渲染生成物**（B 类），源里本没有实体。

## 1. 决策记录（已拍板，2026-10-05）

| # | 决策 | 结论 |
| --- | --- | --- |
| ① | 交付目录名 | `dist/` → **`delivery/`**（"交付层"而非"构建产物"，一眼可读） |
| ② | release.json 位置 | **入 `delivery/`**（它本来就是交付元数据；`release_url` 的"上跳一级"推导随之废除） |
| ③ | 老客户端 v≤0.0.5 | **主动切断（测试版断代，不是"暂时不能更新"）**：delivery 改名 + sourceSchema 升 2 后它们的内置地址失效，且 OTA 同源同死 —— v0.0.5 及更早测试客户端**永久失去更新能力**，界面将永远显示"已是最新"（`software_update` 的 `Err → none_available` 静默降级）。这是未发布阶段的**有意决策**，作者机手动更新一次即出断代。**任何人不得把这条误写成"老客户端兼容，只是暂时收不到"** |
| ④ | `filesRoot` 声明位置 | **Source Manifest**（`"filesRoot": ".."`，唯一被允许的"向上"，单点声明） |
| ⑤ | 生成器出口 | **只认 `presets/delivery/`**；`scripts/publish-presets.mjs` 等第二出口收敛或退役 |
| ⑥ | 业务层寻址 | **禁止拼路径**；`catalog.path` 永远不允许 `..`；Source Manifest 是唯一的"寻址规则声明"，Catalog 只是资源清单、不得自带第二套根/URL |
| ⑦ | 生成物分类 | **维持现状**：A 类源资产留 `presets/assets/` 不复制；B 类渲染产物进 `delivery/mkp/presets/` |
| ⑧ | 节奏 | **长期大纲、分刀推进**（本文档即大纲），不搞一刀切；每刀独立可停、可验收 |

## 2. 目标终态

### 2.1 目录结构（真实目标树）

```text
presets/                          ← Publish Root（catalog.files[].path 的锚）
├── assets/                       # A 类唯一实体（不复制，不进 delivery）
│   ├── bbs/Process/{0.2mm,0.4mm}/*.json
│   ├── brands/  icons/  models/  printers/   # printers = bundled 档，不进下载面
├── machines/  bundles/  plates/  forbidden_zones/  registry/
├── assets.toml  brands.toml  bundles.toml  layout_schema.toml  structure-signatures.toml
└── delivery/                     ← Delivery Root（唯一对客目录、生成器唯一出口）
    ├── source.json               # Source Manifest v2（全相对引用）
    ├── catalog.json
    ├── manifest.json
    ├── release.json              # ← 从 presets/ 入场
    ├── content/
    │   ├── machine_catalog.json
    │   ├── bundles.json
    │   └── assets_index.json
    └── mkp/presets/*.toml        # B 类渲染产物（catalog.path = "delivery/mkp/presets/…"）
```

> 事实修正：现仓库没有 `versions/`、`templates/` 独立目录（versions 内嵌于 `machines/*.toml`）——以实际树为准，不顺手重排源树。

### 2.2 Source Manifest v2 契约

```json
{
  "sourceSchema": 2,
  "catalog":   "catalog.json",
  "manifest":  "manifest.json",
  "release":   "release.json",
  "content":   "content/",
  "filesRoot": ".."
}
```

- **职责切分说死**：Manifest = **寻址规则声明**（各类东西从哪个锚点找）；Catalog = **资源清单**（有哪些 Entry、各自的 `path`）。Catalog 不得自带第二套根/URL —— "怎么找"只有 Manifest 一处说话。
- 全部字段为**相对引用**，以 Manifest 自身 URL 为基准。**交付面禁止出现 `http(s)://`** —— 同一次提交推 GitHub + Gitee 两个远端（实测同内容），写死任何绝对地址 = 把另一个镜像的用户指回去。
- `filesRoot: ".."` 是**全局锚点规则，不是 Entry 路径的组成部分**。它是一句全局的话："所有 catalog 文件的路径，从 `presets/`（仓库根）算" —— 而不是让 24 个资源各自偷偷往上爬。它只出现这一次、进过评审、被 Fixture 钉住；任何 Entry path 里出现 `..` 仍然一律拒绝（现行闸④与 `rejects_traversal_in_catalog_path` 保留并加强）。
- `sourceSchema ≠ 2` → 老客户端如实拒（`NOT_SUPPORTED` 语义），不猜。

### 2.3 地址总表（Golden Fixture 蓝本，目标态）

| 资源 | Manifest 声明 | 云端 URL（GitHub 例） | 本地落点 |
| --- | --- | --- | --- |
| catalog | `"catalog"` | `…/main/presets/delivery/catalog.json` | `<appDataDir>/catalog.json`（说明书释放逻辑不变） |
| manifest | `"manifest"` | `…/presets/delivery/manifest.json` | 客户端暂不落盘 |
| release | `"release"` | `…/presets/delivery/release.json` | 不落盘（只读取） |
| content | `"content"` | `…/presets/delivery/content/machine_catalog.json` 等 | 客户端暂不读（预留锚） |
| MKP ×9 | filesRoot=`".."` | `…/presets/delivery/mkp/presets/A1-fast.toml` | `<appDataDir>/delivery/mkp/presets/A1-fast.toml` |
| BBS ×9 | 同上 | `…/presets/assets/bbs/…/MKPProcess%20A1%200.2%200.10.json` | `<appDataDir>/assets/bbs/…` |
| icon ×3 | 同上 | `…/presets/assets/icons/a1.svg` | `<appDataDir>/assets/icons/a1.svg` |
| model ×3 | 同上 | `…/presets/assets/models/Precise_Calibration.3mf` | `<appDataDir>/assets/models/…` |

不变量：Entry path 永远是干净相对路径；"取哪"（URL）与"放哪"（落点）用**同一个 path 值**、同一个 Resolver 算。

### 2.4 Resolver 形态（实现要点，细节施工时定）

- 新模块 `src-tauri/src/runtime/resolver.rs`：**纯函数、无 IO、无状态，体积上限 ~300 行含测试**。它只回答"在哪"，永不回答"怎么去"（重试/缓存/进度永远归管道）。
- 业务层的全部词汇 = 一个类型化请求 + 一个结果：`ResourceRef`（Catalog / Manifest / Release / Content / Entry）→ `ResolvedAddress { remote, local }`。**不引入五种字符串包装类型**：字符串包装可被拆包互转，墙不存在；枚举的穷尽 match 由编译器执法。
- 现有 4 个拼接/落点计算点全部收编：`net.rs:481`（文件下载）、`source.rs:760`（catalog 定位）、`source.rs:843` `release_url`（**删除**，改读 Manifest 声明）、`paths.rs:50` `released_file`（并入 local 侧）。
- 工作台与客户端共用同一个 Resolver：工作台拿仓库检出根当锚 + 同一份 Manifest；发布闸⑦⑧⑬与新闸⑯全部站在它上面。**两端一套算法。**
- **锚点是声明，不是概念**：Entry 从哪找由 Manifest 的 `filesRoot` 声明决定，Catalog / Release 等交付元数据相对 Manifest 自身解析 —— 对调用方只有 `resolve()`，不存在"系统里有两个 root"这种需要理解的知识。
- **术语与变量的纪律**：Publish Root / Delivery Root 只作为**文档与 Resolver 内部**的概念存在；业务层不得出现 `publish_root` / `delivery_root` 同名变量或类型。
- **规模红线（不再扩张）**：不加 `CatalogPath` / `AssetPath` / `PresetPath` / `ReleasePath` 之类的路径包装类型；不加 ResourceManager / AddressManager / PathManager 之类的管理层。Resolver 停在"纯函数、~300 行、只回答在哪"。

## 3. 为什么（证据压缩包，详情见 2026-10-05 审计）

- **事故**：客户端 base 推导 = "source.json 同目录"（`source.rs:816` `directory_of`）= `presets/dist`；而 #27（2026-10-04，`c8e7e10`）把 `catalog.path` 锚点从交付根改到发布根 `presets/`（B 类因此带 `dist/` 前缀、A 类改指 `assets/`）。两者相拼 = `presets/dist/dist/…`，全部 24 条交付文件 404。同提交改了生成侧、发布闸、本地落点，**唯独没改客户端 base 推导**。
- **为什么检查页看不见**：发布闸十五项全在工作台自己的坐标系里对账（发布根 + path，与磁盘一致，理应全绿）；"客户端怎么从 bootstrap 推 base"这条跨端契约无闸模拟。§7 第一刀验收"客户端下载 0.2mm BBS 成功"在 #27 之后**从未在真实 bootstrap URL 上重跑过**。
- **release.json 依赖证明**：`release_url`（`source.rs:843`）从 base 上跳恰好一级命中 `presets/release.json`，有钉位测试；旧客户端更新检查失败是**静默降级**（`ipc/update.rs` `Err → none_available`）。改名 + Manifest v2 后该推导废除，测试改钉新位置。
- **双镜像事实**：`workbench/bootstrap.json` 两个内置源（GitHub/Gitee）指向同一份内容的 source.json（Gitee 实测在线）。
- **结构代次机制就位**：`structure.rs` 的签名只收类型级必填形状，路径值变化不影响签名；但寻址语义变化按设计走 `STRUCTURE_EPOCH`（现为 1）**人工 +1**，闸⑫会逼着在 `structure-signatures.toml` 登记 minClient —— 这正是防漏的钉子之一。

## 4. 六条铁律（从此以后，任何一刀都不得违反）

1. **物理结构只有两层**：`presets/`（源）与 `presets/delivery/`（交付）。不发明第三、第四种发布目录。
2. **生成器只有一个出口**：一切生成物 → `presets/delivery/`。任何脚本/命令往别处写交付物 = 违规。
3. **客户端只有一个寻址入口**：`SourceResolver`。全仓唯一被允许拼接路径的地方。
4. **业务代码完全不知道路径规则**：只说"我要 Catalog / Release / 这个 Entry"。
5. **`catalog.path` 永远不允许 `..`**；唯一的"向上"是 Manifest 的 `filesRoot` —— 它是**全局锚点规则**（"所有 catalog 文件从仓库根算"），不属于任何 Entry path。
6. **Source Manifest 是唯一的"寻址规则声明"；Catalog 只描述资源本身及其 `path`，不得自行定义另一套根/URL**：以后 delivery 改名、搬目录、变 `v1/`，客户端不重新猜 —— 改的只是 Manifest 内容与 Resolver 实现。

> 附加硬规则：**交付面（source.json）禁止绝对 URL**（双镜像中立）；**URL/本地落点同一套逻辑模型**（一个 path 值两用）；**SHA 在 Resolver 给出真实地址、取到字节之后校验**（管道职责，不变）。

## 5. 分阶段路线图（每阶段独立可停、可验收；做完在 §8 打勾）

### M0 · 立此存照（可立即做，半天）
- 本档入库；HANDOFF.md 加一行指针（"寻址改造看这里"）。
- **闸⑯先行**（旧布局版）：发布闸新增"客户端视角 URL 对账"——用现生产函数（`directory_of`/`join_url`）模拟客户端拼 URL，对 catalog 24 条 + catalog/release/content 对账磁盘。当前它应该 **24 条全红**（URL 多一层 dist）——先把病灶钉在闸上，防止有人在 M2 前误以为系统是健康的。
- **Golden Fixture v1**：按**现布局**把地址总表写成字面量测试（先红后绿随 M2 翻转）。
- 验收：闸⑯红得有理有据（每条都印出"多了一层 dist"）；其余测试不因新增而红。

### M1 · 引入 Resolver（零行为变化）
- 新建 `runtime/resolver.rs`；4 个拼接点改走它；`join_url`/`directory_of` 降为模块私有。
- 验收：全量测试绿；`grep join_url src-tauri/src` 只剩 resolver 与测试；Fixture v1 绿（布局未动，地址应逐字节不变）。

### M2 · 修锚（404 死亡，可独立发版止血）
- 修正 Entry 的寻址锚点：由 Manifest 的 `filesRoot` 声明决定（现布局下即交付目录的上一层）；Catalog / Release 等交付元数据仍相对 Manifest 自身解析。**不引入"双 root"的说法** —— 调用方只看见 `resolve()`，锚点差异是 Resolver 内部的实现细节。
- 顺带补上 #27 欠的端到端验收。
- 验收：Fixture v1 翻绿；**真机 9 预设 + 9 BBS + 3 图标 + 3 模型全部下载成功、SHA 通过、归档正确**。
- 可停点：若 M3 暂缓，M2 单独发一版客户端，线上即恢复。

### M3 · 重排（delivery / release 入场 / Manifest v2）
- `presets/dist/` → `presets/delivery/`（mv）；`release.json` 入场；`bootstrap_json()` 升 v2；`workbench/bootstrap.json` 两地址换 `…/presets/delivery/source.json`；`STRUCTURE_EPOCH 1→2` + 规则表登记（新签名 `2242174c52e8a9b6`，minClient 0.0.6）；`gen-catalog` 重建 embedded **并新增交付面 catalog 重算出口**；血统 `based_on` **不迁移**（按既有裁决降级 Unknown）；模块/函数改名 `app::dist → app::delivery`、`dist_root → delivery_root`（**编译器兜底，改名漏不了**）。
- 验收：发布事务全链真跑到 GitHub；Gitee 手动同步后 delivery 全族 200；Fixture v2 全绿；闸 16 项全绿。

### M4 · 立法（防复发三层）
- 可见性收口（锚点表/解析函数私有）；CI grep 绊线 + 交付面无绝对 URL 的负向断言（规则见 §6.4）；文档修正（§6.2 清单中 6 份文档逐条改错语义 + 全仓"发布根"术语统一为 Publish/Delivery Root）。
- 验收：负向测试（业务层拼 URL → 编译失败/CI 红；Fixture 改错一个地址 → 红并打印人话 diff）。

### M5 · 清算（长期收尾，允许拖）
- `scripts/publish-presets.mjs` 退役或改调 wb_publish 内核（它现在是**第二个生成出口**，12 处 dist 引用）。
- `src/workbench/dev/mockBackend.ts`、`api.ts`、`SettingsPage`/`BuildPage`/`PageSettings` 文案、`tools/assets/sync.mjs`、`scripts/probes/*` 与新地址对齐。
- HANDOFF / PUBLISH-KNIFE3/4 / ASSET-CHAIN 等流水账文档加"历史快照"头注（不重写）。
- 相邻项处置评估：`release.json.asset.url` 绝对 GitHub Releases 地址（Gitee 用户更新流量走 GitHub）——已升格为 **M6**（§5），独立一刀。

### M6 · Gitee Release 发布链路（下一刀，独立做；**不与寻址改造混 PR**）

★ 作者 2026-10-05 拍板：`release.json` 里的安装包下载地址最终必须**彻底切到 Gitee**，
现在就定成架构规则（不是以后手工改）。**这是发布层的事，不动 SourceResolver** ——
Resolver 管"source.json 声明的资源在哪"；Release Asset 是另一条链：
`构建产物 → 发布到哪个 Release 平台 → 拿到真实附件 URL → 写进 delivery/release.json`。

目标链路（点「发布软件版本」）：

```text
① 打安装包 → ② 创建 Gitee Release（tag = vX.Y.Z） → ③ 上传安装包附件
→ ④ Gitee API 返回附件真实下载 URL → ⑤ 用返回值写 delivery/release.json（**不自己拼**）
→ ⑥ 定稿 / 闸 / PR → main
```

施工前先查清四件事：

1. 现在 `release.json` 是谁生成的（`release_tx` / `release_info` 写入点）；
2. 谁负责创建 GitHub Release（平台层 `workbench::platform`）；
3. 谁上传安装包、上传到哪；
4. `asset.url` 最终在哪一笔写入、以什么值。

然后：

- 平台目标从 GitHub Release 换成 **Gitee Release**（Gitee 支持基于 tag 的发行版 + 附件 API；
  附件 URL 取 **API 返回值**，格式不同也不许自己拼）；
- 新增最终判据（进发布闸 / CI）：**生成后的 `release.json` 里所有安装包 URL 必须来自
  Gitee Release，禁止出现 `github.com/.../releases`**；
- 边界：客户端继续只读 `release.json → asset.url`，永远不知道发布平台细节；
  `SourceResolver` 一个字不改。

## 6. 防漏机制（本文档的核心价值：迁移最容易漏东西）

### 6.1 迁移防漏清单（每次动地址，逐条过；M3 必全过）

| # | 容易漏的东西 | 现值 → 目标值 | 谁兜底（漏了谁红） |
| --- | --- | --- | --- |
| 1 | catalog.files 24 条 path 值 | `dist/mkp/presets/…` → `delivery/mkp/presets/…`；`assets/…` 不变 | `embedded_matches_rebuild`（忘跑 gen-catalog 即红） |
| 2 | manifest.json 的 relativePath | 同上（生成物） | 发布事务 `finalize_consistency` |
| 3 | source.json 字段集 | `{"sourceSchema":1,"catalog"}` → v2 六字段 | 闸⑬ + Manifest 解析测试 |
| 4 | workbench/bootstrap.json 两个 URL | `…/presets/dist/source.json` → `…/presets/delivery/source.json` | build.rs `rerun-if-changed` + 真机 + 闸⑯ |
| 5 | release.json 位置与钉位测试 | `presets/release.json` → `presets/delivery/release.json` | `release_info.rs` 钉位测试（需同步改写断言） |
| 6 | content/ 三件 | 随目录改名 | 闸⑯ 固定物对账 |
| 7 | 血统 based_on 字符串 | 旧形状（`dist/…` / `mkp/…`）**不迁移** —— 按 2026-10-04 既有裁决（`runtime::mine` "老形状认不出就是认不出，绝不猜"）优雅降级为 `Unknown`；field 数据为空，零实际影响 | `an_old_shaped_based_on_is_unknown_and_never_guessed`（已扩展覆盖两代旧形状） |
| 8 | 模块与函数名 | `app::dist` / `dist_root` / `DIST_REL_PATH` / `PRESET_DEST_DIR` 值 | **编译器**（改模块名全仓编译失败，漏不了——这是主动选择把名字一起改的原因） |
| 9 | scripts/publish-presets.mjs | 12 处 dist 引用；第二生成出口 | M5 退役/收敛；过渡期 grep 清单 |
| 10 | .gitignore | `/dist` 锚定规则保留（vite 输出仍叫 dist）；注释里 presets/dist 表述更新 | 人工（CI 无法盯注释） |
| 11 | 前端文案与假后端 | SettingsPage placeholder / BuildPage 说明 / api.ts / mockBackend | grep 清单 + 工作台手测 |
| 12 | scripts/probes/*、tools/assets/sync.mjs | 各 1 处 | grep 清单 |
| 13 | 文档 6 份 | PUBLISH-ARCHITECTURE（17）/ PRESET-DELIVERY-CONTRACT（8）/ DATA-INVENTORY（6）/ DATA-ARCHITECTURE（6）+ HANDOFF 系 | M4 清单逐条打勾 |
| 14 | Gitee 镜像 | 改名后必须手动同步才对客户端可见 | M3 验收含抽查 |
| 15 | STRUCTURE_EPOCH + 规则表 | 1→2，登记新行 minClient | 闸⑫ + `supported_signatures_cover_the_current_structure` 两条逼登记判据 |
| 16 | 老客户端决策 | v≤0.0.5 搁浅 | 决策记录（§1③），无技术兜底——**这是唯一靠记性的一项** |

### 6.2 全量引用清单（2026-10-05 实测 `presets/dist` 31 文件 / 143 处）

> 改名时按此清单逐个打勾；**清单本身也要维护**：M3 开工前重跑
> `grep -rc "presets/dist" src src-tauri/src crates scripts tools .github docs *.md | grep -v ':0$'`
> 以当时 tip 为准。

代码（必须改逻辑或常量）：`src-tauri/src/workbench/app/audit.rs`(18)、`dist.rs`(13)、`source.rs`(12)、`git.rs`(11)、`paths.rs`(5)、`assets.rs`(4)、`catalog.rs`(4)、`release_info.rs`(3)、`publish_tx.rs`(2)、`presetdata/assets.rs`(2)、`workbench/mod.rs`(1)、`history.rs`(1)、`build.rs`(1)、`ipc/catalog.rs`(1)、`crates/preset/tests/baseline_stays_untouched_on_the_generate_path.rs`(1)
前端：`src/workbench/views/SettingsPage.tsx`(4)、`src/workbench/dev/mockBackend.ts`(3)、`src/workbench/api.ts`(2)、`src/app/settings/PageSettings.tsx`(1)
脚本工具：`scripts/publish-presets.mjs`(12)、`tools/assets/sync.mjs`(1)、`scripts/probes/chain.mjs`(1)、`scripts/probes/asset-preview.mjs`(1)
文档：`HANDOFF.md`(28，流水账加头注即可)、`docs/PUBLISH-ARCHITECTURE.md`(17)、`docs/PRESET-DELIVERY-CONTRACT.md`(8)、`docs/DATA-INVENTORY.md`(6)、`docs/DATA-ARCHITECTURE.md`(6)、`ASSET-CHAIN-HANDOFF.md`(3)、`PUBLISH-KNIFE3/4-HANDOFF.md`(2×2)、`docs/TWO-REPO-ALIGNMENT.md`(1)
配置：`workbench/bootstrap.json`、`.gitignore`（注释）、`presets/dist/*` 生成物（gen-catalog 重建）

### 6.3 钉子测试对照表（"漏改 → 谁红"，新增判据按此表登记）

| 漏改 | 谁变红 |
| --- | --- |
| 忘跑 gen-catalog / 重建 embedded | `embedded_matches_rebuild` |
| 忘改 bootstrap.json | 闸⑯（对账）+ 真机 404 |
| 忘迁/忘改写 release.json 钉位 | `the_release_file_lives_next_to_the_publish_root`（改写后） |
| 忘登记结构代次 | 闸⑫ + `supported_signatures_cover_the_current_structure` + `the_rule_table_registers_the_current_signature` |
| 忘改某处交付 path/文件 | 闸⑦⑧ + `finalize_consistency` |
| 业务层偷拼 URL | 编译失败（可见性）+ CI grep 绊线 |
| Manifest 混入绝对 URL | CI 负向断言 |
| Golden Fixture 与现实漂移 | Fixture 自身红（这就是它的意义） |

### 6.4 CI 常驻负向断言（M4 落地）

```bash
# ① 拼接越界（豁免：resolver 模块与其测试）
! grep -rnE 'join_url\s*\(|format!\s*\(\s*"[^"]*\{[^}]*\}/\{' \
    src-tauri/src --include='*.rs' | grep -v 'runtime/resolver' | grep -v '#\[test\]'
# ② 交付面禁止绝对 URL（双镜像中立）
! grep -E 'https?://' presets/delivery/source.json
```

### 6.5 全链验收（本改造的总验收，替代"A1 下载成功"）

```text
源文件(presets/*.toml, assets/)
  ↓ 生成器（唯一出口）
presets/delivery/
  ↓ source.json（Manifest v2 声明）
catalog（24 条 files）
  ↓ Resolver（唯一寻址）
真实 URL（GitHub / Gitee 各验一遍）
  ↓ 真实文件（HTTP 200）
SHA 校验
  ↓
本地落点 <appDataDir>/<catalog.path>（归档同形）
```

落地为一条命名测试（如 `full_chain_addressing_walk`，远端部分以 Fixture 字面量 + 手动 Gitee 抽查覆盖）：**这条链上任何一环断开，测试红在断点，而不是用户手机上的 404。**

### 6.6 迁移期三问仪式（每次动到地址相关代码，提交前自问）

1. 我改的是**源、生成逻辑、还是寻址**？只该动其中一层。
2. 这次改动在 **Golden Fixture** 里对应哪几行？没对应行 = 有资源种类没进表，先补表。
3. **Gitee 镜像**和**老客户端**这两个"慢世界"知道这次变化吗？（同步了没有 / 需不需要兼容）

## 7. 边界外登记（不在本改造内，防遗忘）

| 项 | 说明 |
| --- | --- |
| Gitee 镜像的**真实边界**：手动同步后 Gitee 只是**内容源镜像**（source/catalog/assets 随仓库走），**不是完整的更新分发镜像** —— `release.json` 的 `asset.url` 仍指向 GitHub Releases，Gitee 模式用户的应用内更新流量走 GitHub | 已认清并登记为 **M6**（§5）：切到 Gitee Release 是独立一刀，不与寻址改造混做 |
| `crates/preset/assets/presets/` | 构建期嵌入用的 9 份 TOML 副本（文件名不含路径前缀，改名基本无感）；`gen-catalog` 输入口径在 M3 顺带核对 |
| `tools/dev-server` | 开发用 `/presets` 路由（BBS 代理），与交付面无关；M5 顺带确认 |
| `tmp-shots/`、`workbench/.snapshots` | 历史截图/快照含旧目录名，纯史料，不改 |

## 8. 进度台账（完成一项填日期；新发现的面往 §6 加行）

| 阶段 | 内容 | 状态 | 完成日期 | 备注 |
| --- | --- | --- | --- | --- |
| M0 | 总纲入库 + 闸⑯ + Golden Fixture | ✅ 完成 | 2026-10-05 | 闸⑯直接落在目标态（含 bootstrap 反推 + 生产 Resolver 对账）；Golden Fixture = `resolver::tests::golden_addresses_match_the_address_table` |
| M1 | Resolver 引入（收编 4 个拼接点） | ✅ 完成 | 2026-10-05 | `runtime/resolver.rs`；`join_url`/`directory_of` 已从业务面消失（directory_of 留在 source 内部做"Manifest 所在目录"这一件事） |
| M2 | 修锚（Entry 锚由 Manifest `filesRoot` 声明决定） | ✅ 完成 | 2026-10-05 | 端到端验收由闸⑯在真仓库上完成：catalog 24 条 + 固定物逐条对上磁盘 |
| M3 | delivery 重排 + release 入场 + Manifest v2 + epoch 2 | ✅ 完成 | 2026-10-05 | 新结构签名 `2242174c52e8a9b6`（minClient 0.0.6）；embedded + 交付面 catalog 由 gen-catalog 重建；血统不迁移（见 §6.1-7 修正） |
| M4 | 立法（CI 绊线 + 交付面无绝对 URL + 文档修正） | ✅ 完成 | 2026-10-05 | `ci.yml` 寻址立法扫描；PUBLISH-ARCHITECTURE §0/§2.2/§5.3.1/§6 重写 + 历史教训钉防重犯 |
| M5 | 清算（脚本收敛 / mock 对齐 / 术语清扫 / HANDOFF 头注） | ✅ 完成 | 2026-10-05 | `publish-presets.mjs` 加清算中头注并跟新路径（退役裁决留待下一刀）；HANDOFF 系加历史快照头注 |
| M6 | Gitee Release 发布链路（release.asset.url 全量切 Gitee） | ✅ 完成 | 2026-10-05 | 四件事查清（写入点 ⑨ / 建释放 ⑧ / 上传 ⑧+⑧之二 / asset.url=平台返回值）；`ReleaseChannel` 与 PR/tag 的 target 刻意分开；⑥½ 把 tag+main 推到发布仓库（数据源主线同步）；`validate_release_source` 写盘前拦 + CI `check:release-source`（断代线 0.0.6 双侧同值）；preflight 新增 `release-channel` 项。**Gitee 适配层字段名待真机核对**（#32 既有免责）；CI 的 JS 判据从 0.0.6 起真正咬人 |

> **施工记录（2026-10-05）**：`cargo test`（默认 + workbench feature）全绿；
> 老客户端按 §1③ 有意搁浅（作者机手动更新）；`release.json` 钉位测试改钉
> `presets/delivery/release.json` 并禁止根上与 `presets/` 根出现第二份。

---

*本文档由 2026-10-05 路径语义审计直接产出；审计完整证据（URL 解剖、git 历史逐提交对比、五方案 URL 模拟、release.json 依赖的代码级证明）保存在会话记录与桌面评审稿《MKP-资源寻址系统-方案对比.html》。本文档为唯一长期有效的版本。*
