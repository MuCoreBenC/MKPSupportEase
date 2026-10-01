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
| R2 | `client/mod.rs` | 数据面骨架："代码一套，数据根两套，永不互读写" | ③ 的管理者 | **保留** |
| R3 | `client/defaults.rs` | 13 份源 TOML `include_str!` 进二进制 | **违反铁律 1**（总纲欠账 #1） | **收口**：换成随包 catalog 一份。模块头的三条理由（用户机器没仓库 / 只定义无载荷 / 与 load_from 对齐）在 catalog 方案里全部仍成立，换的是**形态**不是动机 |
| R4 | `client/paths.rs` `seed_if_absent` | 首启铺定义 + 建 `mkp/` 空目录；drift 只报不覆盖；**已有测试钉死 mkp/ 初始为空** | 首启释放 | **收口**：铺 13 份 → 释放/校验 catalog 一份；`mkp/` 建目录与空判据测试**原样保留**（总纲判据 3 已经在这了） |
| R5 | `ipc/presets.rs`（9 条读命令 + 缓存） | 首屏与预设页的全部数据源。零网络；SHA/size 栏留空"等发布打 manifest"；`forget_cached_presets` 给未来写命令留了钩子 | 首屏读本地 = 铁律 2 的正面样本 | **保留**命令形状；**收口**数据源：`load_presets` 从"解析 TOML 树"换成"读 catalog"。九条命令的 DTO 一个都不用动——这是 catalog 化成本最低的原因 |
| R6 | `ipc/mod.rs`（4 条基础命令） | 预设值 / 偏移量 / 校准模型 / 开模型 | ③ run 状态 + ② 内置模型 | **保留** |
| R7 | `presetdata/`（catalog/assets/bundles/registry/resolve） | TOML 树解析 + 三层取值 | ①↔③ 的解析层 | **保留**给工作台；客户端侧随 R5 换源后退役。三层取值（`resolve`）是纯计算，两边继续共用 |
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
| C4 | `api/storageKeys.ts` + 五格 localStorage（`cloud` / `a40.package` / `a40.presets` / `a40.active` / `pinned`） | **用户运行时数据住 WebView localStorage** | **违反铁律 4**（新发现，见 §0） | **收口**（最大一笔）：`package / presets / active` 三格迁 Internal 根（与 catalog、`mkp/` 汇合，走 `atomic_write`）；`cloud` 格是演示管道随 C7 退役；`pinned` 是纯前端偏好可留。键名集中处保留——改名只动这一处的设计是对的 |
| C5 | `app/store/package.ts` | 客户端同步层：说明书自动同步 / 本机预设 / 使用中，三件互不混同 | ③ 的前端视图 | **保留职责模型**（三态分离与产品规则同构）；存储随 C4 迁移 |
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
判据 1/2/4 落地               ← 判据 3 已存在（R4 的测试）；其余随对应环节走
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

每收口一条：勾掉本表一行 + 更新总纲 §4 对应欠账。**新增任何数据相关代码前，先过总纲 §6 准入问句。**
