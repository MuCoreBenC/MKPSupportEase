# 数据收口盘点 —— 拿总纲当尺子

> **这份文档是总纲的对账单：**把项目里所有碰**文件 / 网络 / 下载 / catalog / 首启 / 云端**的东西一件件列出来，按 `docs/DATA-ARCHITECTURE.md` 归位。
> **它不是重构计划。**裁决只有四态：**保留**（已经在正确的抽屉里）/ **收口**（东西对、位置或形态不对，改造）/ **移动**（换目录）/ **删除**（不该存在）。收口一条，勾一条，同时更新总纲 §4。
> 对账日期 2026-10-01，基于 commit `58f4295` 的工作区。

---

## 0. 一屏总账

| 盘点范围 | 件数 | 结论 |
| --- | --- | --- |
| 基础设施（物流八环节） | 8 | **有 3 缺 5**：写盘、防穿越、解析在；manifest / 下载 / 校验 / 更新 / 归档全缺 |
| 数据文件（目录级） | 10 | 7 保留、2 收口、1 删除出包 |
| Rust 代码（模块级） | 12 | 9 保留、3 收口 |
| 前端代码（模块级） | 9 | 6 保留、3 收口（最大欠账：**用户数据住 localStorage**） |
| 构建与工具 | 4 | 4 保留 |

> 2026-10-01 增补：Rust 侧的 R2/R3/R4/R5 四条已在本圈收口（见 §3.1 勾账）——
> `client/` 目录删除、catalog 加厚出完整 definition、九条读命令换源。
> §1 八环节里的"安装包资源管理"随之有了正面向：catalog 的 `assets` 域已登记
> 19 条资产定义（载荷登记与下载仍欠，见 #3）。
>
> 2026-10-01 增补：**前端最大欠账 C4 收口**（见 §3.2 勾账）——localStorage 三格底账
> （说明书 / 本机预设 / 使用中）退役，消费方（预设页 / 参数页 / 首页 / 校准页 / 同步页）
> 全部切到新世界对应物。WebView 的 localStorage 从此只住纯前端偏好。

两个改变判断的发现：

1. **"做菜"的环节已经存在大半。** 工作台 `wb_publish` 的交付层结构已在代码里定稿（`content/` 三件 JSON + `presets/mkp/` 产物 + `assets/` 可达子集 + `manifest.json`，SHA/size 发布时算、残留文件拦截发布——见 `workbench/app/dist.rs` 头注释）；盘上目前实际生成过的只有 9 份预设产物（`presets/dist/presets/mkp/`，本机暂存不入库），入库真身在 `crates/preset/assets/presets/`。总纲欠账 #1 的解法**不是新写构建器**，是把这条已有的管道接给安装包。
2. **最大的一笔不在总纲 §4 里：客户端的用户数据住 localStorage**（`mkp.a40.package / presets / active`）。说明书、下载的预设、使用中状态都是③层运行时数据，现在住在 WebView 的 localStorage——换机器即丢、不可备份、绕过 `atomic_write` 纪律。违反铁律 4。

---

## 1. 物流系统八环节对照（公共基础设施）

总纲定的骨架：不管送的是 TOML、JSON、模型还是图片，物流只认七个问题（它是谁 → 从哪来 → SHA → 下到哪 → 有没有 → 更新怎么办 → 旧的去哪）。逐环节盘点：

| 环节 | 现状 | 在哪 | 裁决 |
| --- | --- | --- | --- |
| 本地数据管理 | **有**。两根 + 防穿越三判据 | `src-tauri/src/fsx/paths.rs` | 保留。这就是总纲③的落点 |
| 保存（唯一写盘出口） | **有**。临时文件→persist→fsync，clippy 全仓拦直写 | `src-tauri/src/fsx/atomic.rs` | 保留。物流的"签收落架"环节 |
| 解析（读数据） | **有**。代码一套、根两套（客户端 `<appDataDir>/presets`，工作台 `<repo>/presets`） | `src-tauri/src/presetdata/` | 保留为工作台解析层；客户端侧等 catalog 落地后退役（见 §3 收口 C2） |
| 安装包资源管理 | **缺**。public/ 的资源没有清单、没有版本、没有 SHA | — | 该建：发布构建登记（总纲判据 1 的正面向） |
| 云端清单（manifest） | **缺**。零网络依赖（Cargo.toml 无 reqwest 系）。但格式已定稿：manifest v3，字段在 `workbench/app/dist.rs` 头注释 | — | 该建。**读 manifest 是客户端的事，别在工作台模块里长出来** |
| 下载 | **缺**。契约有 `downloadFiles(refs)` 占位，mock 里都是 `NotImplementedError` | `src/api/contract.ts:634` | 该建。唯一的入口已画好 |
| 校验（SHA） | **缺一半**。发布侧"发布时按真实字节算"已定；客户端侧收到后验、失效判定（产品规则 §10）无代码 | — | 该建 |
| 更新 / 归档 | **缺**。`archive/` 只在总纲图里存在；seed 的 drift（`SeedReport::drifted`）是它的雏形 | `client/paths.rs` | 该建。drift 只报不覆盖的策略是对的，归档收口时沿用 |

**五缺共同的纪律：必须建成一个公共模块（与 `fsx` 同级，比如 `cloud/` 或 `delivery/`），业务（Preset / 模型 / BBS / 未来的图片）只许调用，不许自己发明下载、校验、归档。** 这正是"半年后又变成现在这个样子"的防波堤。

---

## 2. 数据文件逐目录归层

| # | 现在在哪 | 是什么 | 按总纲 | 裁决 |
| --- | --- | --- | --- | --- |
| F1 | `presets/*.toml`（13 份源） | 机型 / 禁区 / 套餐 / 资产 / 布局 / 注册表的定义源 | ① 源 | **保留**。与总纲 §1① 的布局一字不差 |
| F2 | `presets/dist/` | 发布器产物暂存（实际已有 `presets/mkp/` 9 份；content JSON 与 manifest 是设计稿、尚未生成）。**本机生成、gitignore、不入库** | ① 的本机暂存，**不是判据输入** | **保留**（本机）；判据输入用 F2b |
| F2b | `crates/preset/assets/presets/`（9 份，入库） | **入库产物目录**：`BUILTIN_PRESETS` 编进二进制的同一批真字节 | ① 的②半成品真身 | **保留**；构建器与判据都认它 |
| F3 | `public/assets/icons|models|printers` | 图标 / 3mf / 机型图，随 vite 进包 | ② 内置资源（源） | **保留**；收口：发布时登记进 catalog（版本/SHA） |
| F4 | `public/assets/bbs/Process/`（9 份 JSON） | BBS 切片配置成品 | ② 内置资源 | **收口**（总纲欠账 #3）：登记，不是裸放 |
| F5 | `public/bbs/`（PROVENANCE / registry / layout / defaults / icons / _sync.json） | BBS 页元数据（提取器产物，随本仓分发） | ② 内置资源 | **保留**；与 F4 同一批登记 |
| F6 | `public/models/hero_pile*.webp` | 首页大图 | ② 内置资源（UI 资产） | **保留**；登记 |
| F7 | ~~`public/cloud/presets.json`~~ → `src/workbench/fixtures/cloud-presets.json` | 模拟云端的静态快照（工作台演示管道的另一半） | ① 开发夹具 | **已收口 2026-10-01**：静态 import，只进工作台构建；客户端产物 grep 验证无此字节 |
| F8 | `workbench/.snapshots/ .draft/` | 工作台运行态 | ① 工具暂存 | **保留**（现状未入库，维持） |
| F9 | `crates/*/tests/fixtures/` | 判据资产（对照基线等） | ① 判据 | **保留**（`ARCHITECTURE.md` §10.6 已定） |
| F10 | `public/` 本身这个约定 | vite 原样拷贝进包的目录 | ② 的**源**与②**本体**在这里重合 | **收口**（结构性）：引入发布构建后，"进包的东西"应是构建产物而非 public/ 原样拷贝。过渡期先靠判据 1（安装包扫描）兜底 |

---

## 3. 代码逐件对账

### 3.1 Rust（`src-tauri/src/` + `crates/`）

| # | 在哪 | 干什么 | 按总纲 | 裁决 |
| --- | --- | --- | --- | --- |
| R1 | `fsx/paths.rs`、`fsx/atomic.rs` | 两根 + 防穿越 + 唯一写盘出口 | 基础设施 | **保留** |
| R2 | `client/mod.rs` | 数据面骨架："代码一套，数据根两套，永不互读写" | ③ 的管理者 | **已收口 2026-10-01（随 R3/R4）**：模块退役。"数据根两套"的纪律由新的分工接手——工作台读仓库源（presetdata::load_from），客户端只读 catalog，仍永不互读写 |
| R3 | `client/defaults.rs` | 13 份源 TOML `include_str!` 进二进制 | **违反铁律 1**（总纲欠账 #1） | **已收口 2026-10-01（R3）**：模块整个退役。definition 由发布构建从同一批源算进 catalog（definition 类型直接复用 presetdata 的 serde 类型），客户端不再持有源文件字节 |
| R4 | `client/paths.rs` `seed_if_absent` | 首启铺定义 + 建 `mkp/` 空目录；drift 只报不覆盖；**已有测试钉死 mkp/ 初始为空** | 首启释放 | **已收口 2026-10-01（R4）**：`client/` 目录删除。铺盘只剩 catalog 一份（`runtime::release`，升级=旧份归档、新份生效）；`mkp/` 建目录与空判据由 release 的 `mkp_dir_is_created_empty` 原样承担 |
| R5 | `ipc/presets.rs`（9 条读命令 + 缓存） | 首屏与预设页的全部数据源。零网络；SHA/size 栏留空"等发布打 manifest"；`forget_cached_presets` 给未来写命令留了钩子 | 首屏读本地 = 铁律 2 的正面样本 | **已收口 2026-10-01（R5）**：九条命令的 DTO 一个没动，数据源换成释放的 catalog（按**字节**缓存，目录换新自动失效）；MKP 引用带真 size/SHA；三层取值改走与 ParamRegistry 共用的 `resolve::visible_keys_of / effective_of`。判据 `dto_builders_read_the_catalog_and_nothing_else` 钉死"唯一数据源" |
| R6 | `ipc/mod.rs`（4 条基础命令） | 预设值 / 偏移量 / 校准模型 / 开模型 | ③ run 状态 + ② 内置模型 | **保留** |
| R7 | `presetdata/`（catalog/assets/bundles/registry/resolve） | TOML 树解析 + 三层取值 | ①↔③ 的解析层 | **保留**给工作台；客户端侧已随 R5 换源退役（2026-10-01）——但它的**类型**成了两端共用契约的 definition（serde 序列化进 catalog），三层取值（`resolve`）提取成 `visible_keys_of / effective_of` 继续两边共用 |
| R8 | `workbench/`（paths/app/domain/store…） | 后厨：源编辑、生成、发布、manifest v3、残留拦截 | ① 的工具 | **保留**。`wb_publish` 就是总纲"发布构建"的现成本体 |
| R9 | `crates/preset`（BUILTIN_PRESETS + `preset_file_name`） | 命名唯一实现 + 9 份内置预设编译进二进制，判据锚入库产物目录 | ② 内置内容 | **保留**形态；**收口**清单归 catalog（总纲欠账 #4） |
| R10 | `client/defaults.rs` 之外的 `chrome/obs/error/lib` | 窗口 / 日志 / 错误 | 基础设施 | **保留**（不属数据架构，列此备查） |
| R11 | `workbench/app/dist.rs` 头注释里的 **manifest v3 定义** | 交付文件全集的哈希清单， bundles 字段已删 | ④ manifest 的格式 | **收口**：把这份定义**提升**为两端共用契约（不是工作台私有物）。真云端来之前，它是④的最终形状 |
| R12 | （不存在） | 网络层 | ④ 客户端侧 | **该建**（§1 五缺），公共模块 |

### 3.2 前端（`src/`）

| # | 在哪 | 干什么 | 按总纲 | 裁决 |
| --- | --- | --- | --- | --- |
| C1 | `api/contract.ts`（14 方法） | 前后端唯一约定 | 契约 | **保留**。`downloadFiles` 是物流"下载"环节的入口占位——它已经画在契约里，实现别另起炉灶 |
| C2 | `api/index.ts` `bridge.ts` `mock.ts` `errors.ts` | 运行时探测走真桥或 mock | 契约 | **保留** |
| C3 | `api/mockServer/*`（10 件） | 浏览器调试用的假后端（machines/params/bbs/files/resources） | ① 开发工具 | **保留**；真实下载落地后，其"云端 / 文件"两块对应退役 |
| C4 | `api/storageKeys.ts` + 五格 localStorage（`cloud` / `a40.package` / `a40.presets` / `a40.active` / `pinned`） | **用户运行时数据住 WebView localStorage** | **违反铁律 4**（新发现，见 §0） | **已收口 2026-10-01**：`package / presets / active` 三格删除，消费方切到新世界对应物（catalog / `mkp/` / `run/active-preset.json`，全部走 Rust 侧 `atomic_write`）——不是把 localStorage 搬个地方，是切到本来就位的数据。`cloud` 格是演示管道随 C7 退役；`pinned` 等纯前端偏好保留。**判据就是编译期**：三键从 `STORAGE` 删除后，任何再引用直接编译失败 |
| C5 | `app/store/package.ts` | 客户端同步层：说明书自动同步 / 本机预设 / 使用中，三件互不混同 | ③ 的前端视图 | **已收口 2026-10-01（随 C4）**：三格底账整体退役，"三件互不混同"的职责模型由新世界对应物接手（catalog / `mkp/` / `run/` 使用中指针）；文件只剩时间格的格式化工具 |
| C6 | `app/home/useCatalog.ts` | 首页消费真目录，不硬编码 | 首屏读本地 | **保留** |
| C7 | `workbench/cloud.ts` | 模拟云端：静态快照 + localStorage 上传 | ① 演示管道 | **收口→退役**：真下载落地后整块删；`public/cloud/presets.json`（F7）随它走。`clientPackage.ts` 头注释自己写着"归宿是 Rust"——装配逻辑迁 R8 发布链路 |
| C8 | `app/bbs/*` + `tools/dev-server/bbsFs.mjs` | BBS 页读**本机 Bambu Studio 目录**（serve 期端点，build 产物里不存在） | ① 开发/预览能力 | **保留**。注意两点已自洽：无鉴权端点的安全边界写在文件头；产品包里没有这个端点，页面是空态不是报错 |
| C9 | `workbench/c14/* views/*` | 后厨界面 | ① 工具 UI | **保留**（不属数据架构，列此备查） |

### 3.3 构建与工具

| # | 在哪 | 干什么 | 裁决 |
| --- | --- | --- | --- |
| T1 | `scripts/release.mjs` | 发版流程（校验→PR→tag），不碰数据 | **保留**（① 工具） |
| T2 | `vite.config.ts` | workbench 开关 + bbsFs 挂载；public/ 原样进包 | **保留**；F10 收口后这里加"产物即包内容"的管线 |
| T3 | `tauri.conf.json` | 无 `resources` 字段——安装包 = 二进制 + vite 产物 | **收口**时在这里定 catalog 与内置资源的进包方式 |
| T4 | `clippy.toml` 写盘禁列 | 唯一写盘出口的守门人 | **保留** |

---

## 4. 收口顺序建议（不是计划，是依赖关系）

依赖关系摆出来，顺序自然浮出，谁先谁后按当时人力定：

```text
C7 退役 + F7 删除            ← 不依赖任何新东西，随时可做，删掉进包的假云端
R11 manifest 定义升为两端契约  ← 只是搬家+改归属，先于一切网络代码
R3/R4 catalog 化（含 F10）    ← 依赖 R11 定稿；解总纲欠账 #1，R5 换源、C4 迁移跟着走
C4 localStorage 迁 Internal   ← 依赖 R4 的新落点；解本盘点最大欠账
§1 五缺建公共模块             ← 依赖 R11；manifest → 下载 → 校验 → 更新/归档 一个模块内长
判据 1/2/4 落地               ← 判据 1 已落地（scripts/check-bundle.mjs，CI web job 跑）；3 已存在（R4 的测试）；2/4 随对应环节走
```

**进展（第一圈，按舞步走，不是按本表的收口顺序）**：

- 2026-10-01：**第一块地基立起来了** —— 新模块 `src-tauri/src/runtime/`（新世界落点 +
  随包 catalog 的释放口）、构建器 `cargo run --bin gen-catalog`（层① → 层②，产物
  `catalog.generated.json` 编进二进制，判据测试守着重建逐字节一致）、命令
  `get_runtime_catalog`（读释放进数据根的那份，零网络）、同步页新增「数据骨架（新）」
  读数区。最小闭环 **页面 → 新 API → 运行时 catalog → 真实数据** 已通；
  下载区 `mkp/` 初始为空有测试钉着（铁律 3）。旧世界一概未动。
- 2026-10-01：**F7 收口**（假云端快照挪出 `public/`，只进工作台构建）；
  **④ Delivery 骨架立起来** —— `runtime/delivery.rs`：`Source` 可插拔
  （第一圈只有 `LocalDirSource`，真云端来了换实现管道不动）、SHA/大小校验在落盘之前、
  防穿越、原子落盘 `mkp/`、"已下载"不记账本——盘就是底账；命令
  `download_runtime_file` / `get_downloaded_files`；同步页可点下载看结果。
  §1 八缺里的"下载 / 校验"两个环节有了骨架，"更新 / 归档"第二圈接。
- 2026-10-01：**⑤ 用户数据的持久化规则落地** —— `runtime/state.rs`：
  一种状态一个文件（JSON + `*Schema` 代次）、住内部根 `run/` 下、写走 atomic_write、
  **坏档不静默**（`CORRUPTED`，不装作"没有"）。第一个真数据是**使用中指针**
  （`run/active-preset.json`）：全局唯一在构造上成立（就一个文件），指针带应用时刻的
  SHA——盘上字节漂了界面看得见。命令 `get/apply/clear_active_preset`；
  同步页可"使用 / 撤销"。旧世界 localStorage 三格仍留在原处（收口 C4 时退役），
  但新世界从此有了自己的状态住所，业务不必再往 localStorage 里塞。
- 2026-10-01：**判据 1 落地** —— `scripts/check-bundle.mjs`：扫客户端构建产物，
  开发源 TOML / 模拟数据 / 工作台内容出现即红（黑名单制，工作台构建不在管辖内）；
  挂进 CI web job。干净过、种脏能抓，两种方向都实测过。

- 2026-10-01：**R11 落地（第二圈）** —— 两端共用契约成形：工作台发布
  `wb_publish` 现在额外产出 `presets/dist/catalog.json`（与客户端
  `runtime::Catalog` 同 schema、同指纹算法，`build_from_presets_lenient`
  宽松构建）；客户端新增检查/应用更新（`check_remote_update` /
  `apply_remote_update`）：指纹比较 → 应用走既有 release 归档管道 →
  Stale 文件走既有下载管道。**更新没有第三条路径**。下载源跟着目录走
  （本地 revision 与远端一致才从远端拿）。真云端来了只换清单来源。

- 2026-10-01：**catalog 加厚 + 换源收口（R3/R4/R5，第二圈）** ——
  1. **catalog 长出完整 definition**：brands / 机型的全部字段（含尺寸与禁区）/
     assets（19 条）/ bundles（5 份）/ registry（74 条字段定义 + 页签元数据 +
     参数摆放）全部进 `runtime::Catalog`，definition 类型**直接复用**
     presetdata 的 serde 类型（不造镜像结构）；revision 指纹把 definition
     算进输入（改一个字段定义，检查更新就看得见）。`CATALOG_SCHEMA` 不升
     （加字段不升号；definition 全带 `#[serde(default)]`）。
  2. **客户端换源**：九条预设读命令的 DTO 一个没动，数据源从"解析
     `<appDataDir>/presets` 的 13 份 TOML"换成"读释放的 catalog"（按字节缓存，
     目录换新自动失效）。三层取值提取成 `resolve::visible_keys_of /
     effective_of`，与 ParamRegistry 共用一份算法。判据
     `dto_builders_read_the_catalog_and_nothing_else` +
     `the_definition_travels_with_the_catalog` 落地。
  3. **旧世界退役**：`client/` 目录（defaults 13 份 include_str! + seed 铺盘）
     删除，启动铺盘只剩 catalog 一份（release 管道）。**总纲欠账 #1 收口**。
     判据 4（首屏唯一数据源 = catalog）在 Rust 侧由类型 + 判据钉死。

- 2026-10-01：**C4 收口（第二圈）—— localStorage 三格底账退役**（本盘点最大欠账）。
  1. **active 格 → `run/active-preset.json`**：预设页「应用」、首页与校准页反填、
     参数页默认落点全部切 `api.getActivePreset / applyActivePreset`；
     `ActiveEntry`（kind/ref 形状）退役，统一用契约的 `ActivePreset`（fileName +
     SHA + intact），「已应用」判据变成文件名相等 —— 全表唯一在底账的形状里成立。
  2. **presets 格 → `mkp/`**：预设页的"工作台发布"两列换成**目录交付**（catalog 的
     files 域，大小是登记真值）+ 下载区（`getDownloadedFiles`，盘就是底账）；
     下载走 `downloadCatalogFile`（新管道），旧世界 `fetchPreset`（TOML 正文塞
     localStorage）退役。
  3. **package 格 → catalog**：参数页整页数据源切 catalog 命令（页签/分组树从
     `getRuntimeCatalog` 的 registry 摊、值走 `getMachineParams`、字段走
     `getParamMeta`、文件名走 files 域）；同步页旧世界的"自动同步说明书"区退役，
     catalog 的账就是那一页的账。
  4. **判据在编译期**：三键从 `STORAGE` 删除，再引用直接 tsc 失败；
     `ClientDataPackage`/`ReleasePreset` 类型保留（工作台侧还在用），
     客户端消费面零引用。

- 2026-10-01：**④ Delivery 加厚收口（第二圈）—— 真源头上线的那一刀**。
  1. **地址不在 catalog 里**（总纲 §1④ 的分工定死）：新增 `runtime/source.rs`
     管"当前用哪个云端"——住内部根 `run/preset-source.json`，沿用
     `runtime/state.rs` 那一套规则（一种状态一个文件 + `*Schema` 代次 +
     atomic_write + 坏档 CORRUPTED），默认地址由构建期 `MKPSE_PRESET_SOURCE`
     注入；**没注入就是没配**，下载与检查更新都如实拒绝，不猜 URL。
  2. 下载地址 = `baseUrl` + catalog 记的相对位置（总纲 §1④ 那个分工），
     所以换 Gitee / 换自建 CDN 不用重发说明书。命令
     `get_preset_source` / `set_preset_source`。
  3. **网络只住一个文件**：新增 `runtime/net.rs` —— ureq（同步客户端 + rustls，
     不引异步 HTTP 栈）装在 `Source` trait 后面，`delivery.rs` 的管道一行未改。
     阻塞网络与磁盘丢进 `tauri::async_runtime::spawn_blocking`，不堵异步运行时。
  4. **失败与进度的口径**：只有**运输类**故障（连接/超时/解析/被掐）退避重试；
     404、500、字节超了目录记的大小一次都不重试（那是答案不是抖动）。
     进度走 Tauri `Channel`，只报真知道的事（`connecting` / `transferring` /
     `done` / `failed`）—— 没有"校验中 / 落盘中"，那两步在管道内部，报了就是编的。
     批量并发在 Rust 侧（`deliver_all`，固定 4 条道），**逐份给结局**，返回按请求顺序。
  5. **脚手架退场**：原来探测仓库绝对路径的那段（探测 `presets/dist` 与
     `crates/preset/assets/presets` 两个目录）全部删除，检查 / 应用更新改走同一个地址概念。
  6. **判据 2 落地**：`scripts/check-zero-network.mjs`，
     三道闸——网络符号只许住 `runtime/net.rs`、程序的 `.setup()` 段零联网、
     `src/api/` 不许绕过 IPC 自己发 HTTP。挂 CI web job，干净过并用种脏实测过。

- 2026-10-01：**第三圈第一刀 —— BBS 配置纳入统一资产入口**（作者圈定：先吃资产，不为兼容旧体系保留旧名）。
  1. **catalog 认识它**：新增 `runtime::catalog::kind`（`mkp_preset` / `bbs_config`）与落点规矩
     `mkp/<kind 目录>/…`；9 份预设的落点随之统一到 `mkp/presets/…`（旧形状 `mkp/<文件名>` 退役）。
     BBS 那 9 份由构建器从 `presets/assets.toml` 的 `slicerProfile` 条目 + 载荷真字节算出
     SHA / 大小登记进来（**定义与载荷分家**：定义是①，字节是构建期读出来的）。
  2. **同一条管道**：与预设共用 `deliver`（校验在落盘前 / 防穿越 / 旧份归档 / 盘当底账），
     **没有为资产新增下载系统**。端到端判据是真 HTTP → SHA → 落 `mkp/bbs/Process/0.2mm/…`。
  3. **随包副本退役**：客户端产物不再带 `assets/bbs/`（`vite.config.ts` 的
     `DELIVERED_ASSET_DIRS` + 判据 1 的同一份清单两处同步）；dev / 工作台模式照旧能取到。
  4. **消费面切换**：新增 `read_downloaded_text`（前端不碰文件系统，只认 catalog 登记的落点，
     没下载就 NOT_FOUND）；BBS 页改从 `mkp/bbs/` 载入，一份都没下载时页面如实说
     "产品配置 N 份，还没下载"，不摆点不动的假入口。
  5. **顺手修掉一个真 bug**：资产文件名带空格（`MKPProcess A1 0.2 0.10.json`），
     拼 URL 不编码会直接 `invalid uri character` —— 编码收在 `source::join_url` 一处（判据盯着）。
  6. 判据：catalog 侧新增「种类与落点」「文件名与落点唯一」「大小 = 盘上真字节」三条；
     随包退役由判据 1 的 `assets/bbs/` 那条守。

- 2026-10-01：**第三圈第二刀 —— 模型与图标照同一条路接进来**。
  `kind_of_asset` 一张表决定"台账里的哪一类登记成哪个 kind"：切片器预设 → `bbs_config`、
  模型 → `model`、图标 → `icon`，落点分别是 `mkp/bbs/`、`mkp/models/`、`mkp/icons/`。
  **管道、命令、判据结构一行没改** —— 这一刀的代码量几乎全在那张映射表里，
  这就是"新增一种资源只加一个 kind"的实证。随包副本同样退役（产物 `dist/assets/`
  现在只剩 `printers/`）。**整机图故意没登记**：它由首页当 UI 装饰直读，
  接进管道要连首页取图方式一起改（判据里留了一条"别忘了它"的断言）。
  判据：每一种登记类都与资产台账条数对齐 + 落点前缀 + 大小=盘上真字节；
  新增一条"模型也走同一管道"的端到端判据。

每收口一条：勾掉本表一行 + 更新总纲 §4 对应欠账。**新增任何数据相关代码前，先过总纲 §6 准入问句。**
