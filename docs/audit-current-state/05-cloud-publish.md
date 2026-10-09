# 05 · 云端、官方源与发布链路

> 本层的重点是**核实产品规则是否真的落实**，而不是复述设计。每条规则给"符合 / 部分符合 / 未核对"与证据。

## 1. 官方源：配置、注入、发现

| 项 | 结论 | 证据 | 等级 |
| --- | --- | --- | --- |
| 两个内置源地址从哪来 | **编译期注入**：`build.rs` 读 `../workbench/bootstrap.json` 的 `bootstrapUrl` / `giteeBootstrapUrl`，经 `cargo:rustc-env` 写 `MKPSE_PRESET_SOURCE` / `MKPSE_PRESET_SOURCE_GITEE` | `src-tauri/build.rs::28`、`::54` | 已证实 |
| 优先级 | 环境变量显式覆盖 > `workbench/bootstrap.json` > 都没有则**不编造 URL**（联网命令诚实报"还没配置"） | `src-tauri/build.rs::35` | 已证实 |
| 运行期"用哪个源"怎么存 | `run/app-state.json#presetSource`（整体读-改-写 + 原子替换）；旧 `run/preset-source.json` 已退役（读侧兜底、写成功后删） | `src-tauri/src/runtime/source.rs::26`、`app_state.rs::389` | 已证实 |
| `source.json` 怎么发现 | 入口 → `resolve_entry` → `parse_source_json` 取 `source.json` → `SourceManifest::parse` → `SourceResolver`；自定义地址支持"目录 / `source.json`"两形状自适应 | `source.rs::476`、`::550`、`::371` | 已证实 |
| 交付根锚点 | = `source.json` 自身所在目录；`filesRoot` 只认 `.` / `..` | `source.rs::548`、`resolver.rs::282` | 已证实 |
| 开发期覆盖 | `MKPSE_PRESET_SOURCE_URL` **只在 `#[cfg(debug_assertions)]` 认** | `source.rs::430` | 已证实 |
| `source.rs` vs `resolver.rs` 分工 | `source.rs`＝"云端在哪"（设置形状/校验/内置源清单/形状探测）；`resolver.rs`＝"资源具体在哪"，是**全仓唯一允许"根+相对段"拼 URL** 的地方（含百分号编码） | `resolver.rs::1`、`::352`、`::388` | 已证实 |

## 2. 目录（catalog）

| 项 | 结论 | 证据 | 等级 |
| --- | --- | --- | --- |
| 完整字段 | `Catalog{catalogSchema,revision,publishedAt?,brands,machines,assets,bundles,registry,plates,files,structureSignature,minClientVersion?}` | `runtime/catalog.rs::56` | 已证实 |
| `revision` 怎么算 | 对 brands+machines+assets+bundles+registry+plates+files 稳定序列化取 SHA256 前 16 位；**不含** revision 自身、`publishedAt`、`structureSignature` | `catalog.rs::651` | 已证实 |
| `publishedAt` 谁盖 | **发布侧**（工作台 `write_catalog_json`），与 `manifest.updated` 同一个戳 = `clock::now_iso8601()` | `workbench/app/delivery.rs::1031`、`publish_tx.rs::307` | 已证实 |
| 随包目录的 `publishedAt` | **刻意不带**（`None`）—— "随包"没有发布事件，界面照实说"未知" | `catalog.rs::30`、`::534` | 已证实 |
| `sha256` / `size` 从哪来 | **发布侧**对 `dist/` 真字节算（`FileHashes::FromArtifacts`）；随包侧 `None`（只验存在、不登记） | `catalog.rs::686`、`delivery.rs::812` | 已证实 |
| 随包 vs 远端目录差别 | 随包：无 `publishedAt` / 无 `sha256` / `size`、逐字节可复现；远端：有期望值（下载校验权威） | `catalog.rs::271`、`::352` | 已证实 |
| 生成 | `cargo run --bin gen-catalog` → `src-tauri/src/runtime/catalog.generated.json`（编入二进制）+ 重算 `presets/delivery/catalog.json` | `bin/gen-catalog.rs::39` | 已证实 |
| 一致性判据 | `embedded_matches_rebuild` 钉"仓库那份 = 可重建那份" | `runtime/mod.rs::118` | 已证实 |
| `ensure_released` 何时写盘 | **盘上有就一个字节不动**，只在缺失时铺底；先验后写；**绝不覆盖 OTA 成果** | `release.rs::102` | 已证实 |
| 启动只许调它 | 有源码扫描判据（`lib.rs` 里 `release::` 之后只能是 `ensure_released`） | `release.rs::318` | 已证实 |
| 能力判定 | `can_read`：**先能力后版本** —— 签名命中 `SUPPORTED_SIGNATURES` 或 `APP_VERSION >= minClientVersion` 即放行 | `structure.rs::456` | 已证实 |

## 3. 下载与更新链路

| 步骤 | 细节 | 证据 | 等级 |
| --- | --- | --- | --- |
| 触发检查 | 进预设页后台检查**一次**（本次运行只一次，失败静默）；有变化才采用 | `usePresetData.ts::403` | 已证实 |
| URL 拼接 | `SourceResolver::resolve(Entry)`：`filesRoot` + `catalog.path`，逐段百分号编码；**走 HTTPS 原生 `ureq`，不是 git** | `resolver.rs::266`、`net.rs::480` | 已证实 |
| 校验 | 仅当目录给了期望值时：先算 SHA 不符即拒、再比 `size`；随包侧无期望值则照收。SHA 不符时按"本地/远端 revision 是否不同"分两种成因说法 | `delivery.rs::184`、`::129` | 已证实 |
| 重试 | 趟数 = 1 + 2，退避 `300ms * 2^n`；**仅运输类**（IO/超时/连接/主机解析）重试，404/500/超大**不重试** | `net.rs::129`、`::387` | 已证实 |
| 超大拦截 | 目录登记的 `size` 当硬上限，超了提前收摊 | `net.rs::221` | 已证实 |
| 落地 | 旧份归档 → 原子写新份 → baseline（kind==PRESET）→ 事件账 | `delivery.rs::220` | 已证实 |
| 进度 | Channel `DownloadTick`：connecting / transferring / done / failed（**没有"校验中/落盘中"**——那两个报出来就是编的） | `ipc/catalog.rs::191` | 已证实 |
| 批量 | Rust 侧并发；先对清单（缺一份整次拒绝）；结果按请求顺序、水位按完成顺序 | `ipc/catalog.rs::323` | 已证实 |
| 目录换代 | `checkRemoteUpdate` 只比 `revision`（附 `readable`）；`applyRemoteUpdate` 读不懂则**在写之前**回 `NOT_SUPPORTED`（盘零改动），否则旧目录进 `archive/catalog.json` + `archive/catalogs/<rev>.json` | `ipc/catalog.rs::984`、`::1012` | 已证实 |
| 旧文件更新 | 采用目录后 Stale 文件重走 `deliver` —— **没有第三条更新路径** | `runtime/update.rs::11` | 已证实 |

## 4. 发布端（工作台）

| 步骤 | 细节 | 证据 | 等级 |
| --- | --- | --- | --- |
| 产物生成（随包） | `cargo run --bin gen-catalog` → `catalog.generated.json` + `presets/delivery/catalog.json` | `bin/gen-catalog.rs::32` | 已证实 |
| 产物生成（交付） | `wb_generate` / `publish_into` 写 `delivery/mkp/presets/` + 重算 catalog；定稿写 `manifest.json` / `catalog.json` / `source.json` | `workbench/app/delivery.rs::770` | 已证实 |
| 预设数据发布 | `scripts/publish-presets.mjs`：校 5 条（source/catalog/manifest 存在 + SHA/size 对真字节 + 交叉核对）→ 开一次性分支 → PR；**不碰 main 直推闸** | `scripts/publish-presets.mjs::279` | 已证实 |
| 软件版本发布 | `scripts/release.mjs` → `bin/release`（`release_tx::run`）→ 等 CI → 第二趟合并/tag/构建/Release/`release.json` | `scripts/release.mjs::85` | 已证实 |
| 发布到哪 | 软件 Release 与附件**只发 Gitee**（M6 断代 `0.0.6`）；`release_tx` 写前 `validate_release_source` 拦 GitHub 地址 | `release_tx.rs::158`、`scripts/check-release-source.mjs::17` | 已证实 |
| 两条链的区别 | `release.json`＝软件本体（住 `presets/delivery/release.json`，Manifest 的 `release` 声明）；`catalog.json`＝预设数据；version 同源 `CARGO_PKG_VERSION`；**`release.json` 不进 catalog、不参与发布闸、不进结构签名** | `runtime/release_info.rs::10` | 已证实 |
| 发布事务 | `publish_tx::run`：草稿干净检查 → 审计（Blocker 即零写返回）→ 生成 → 定稿 → **最终一致性核对**（红即 Err 短路、不提交）→ git 白名单 stage/commit/push → 平台 PR。**没有显式回滚**，靠"写前判定 + 原子写 + 定稿后核对短路" | `publish_tx.rs::225`、`::391` | 已证实 |
| 平台网络出口 | 只许 `runtime/net.rs` 与 `workbench/app/platform/`（check-zero-network 第一道闸） | `check-zero-network.mjs::44` | 已证实 |
| 发布账户 / Token | 配置 + 凭据文件，Token 只进不出；`wb_get_publish_token` 显式取 | `publish_tx.rs::491`、`lib.rs::384` | 已证实 |

## 5. 产品规则逐条核实

> 这 6 条是**预期规则**，不是"已经实现的事实"。逐条核实结果如下。

| # | 规则 | 结论 | 证据 | 等级 |
| --- | --- | --- | --- | --- |
| R-01 | 应用启动时**不**主动联网 | **符合**。`setup` 只做：数据根/日志、`ensure_released`（纯本地）、窗口外观、开工作台窗口；无任何网络命令。另有静态闸 `check-zero-network.mjs::checkStartup` | `lib.rs::59`、`check-zero-network.mjs::129` | 已证实 |
| R-02 | 官方源在 Workbench 配置、**构建时注入**客户端 | **符合**。`wb_set_bootstrap` 写 `workbench/bootstrap.json`，`build.rs` 编译期注入 | `build.rs::28`、`lib.rs::295` | 已证实 |
| R-03 | 进入预设页时**允许**后台检查官方目录 | **符合**。`checkBootstrapOnce` 每次运行只做一次，失败静默 | `usePresetData.ts::403` | 已证实 |
| R-04 | "目录检查"**只**更新目录、**不**下载预设文件 | **符合**。`checkRemoteUpdate` 只取 catalog，`applyRemoteUpdate` 只 `release_bytes`（换目录）；文件级更新要另触发 | `ipc/catalog.rs::984`、`::1012` | 已证实 |
| R-05 | **只有用户明确选某个预设时才下载** | **部分符合**。官方预设确实如此（都由用户动作触发，无自动下载）；但随包资产档 `delivery='bundled'` 随安装包进包、**不经下载** | `catalog.rs::262` | 部分证实 |
| R-06 | **本地磁盘**是"已交付内容实际状态"的权威来源 | **符合（运行时内）**。`file_status` 以盘上字节算，`Absent/Current/Stale` 三态盘当底账 | `delivery.rs::107`、`::30` | 已证实 |
| R-07 | `check-zero-network` 被 CI 调用 | **符合**（`.github/workflows/ci.yml` 的 `web` job） | `ci.yml::60` | 已证实 |
| R-08 | `check-zero-network` 能抓到**间接**联网 | **抓不到**（只抓直接符号，脚本自述） | `check-zero-network.mjs::15` | 待验证 |
| R-09 | Gitee 平台适配字段名与真机一致 | **未核对**（仓库文档自述） | `docs/RESOURCE-ADDRESSING-ROADMAP.md::298` | 待验证 |
| R-10 | CI 三个 job 实际绿灯 | **本次未跑** | `.github/workflows/ci.yml` | 待验证 |

### 5.1 与预期不一致的地方（逐条）

1. **R-05 的例外**：随包资产（整机图/品牌图/图标等 `delivery='bundled'`）随安装包进包，客户端不下载也不登记 —— 这对"客户端首次可用"是必要的，但要注意"bundled 与 download 两档"在数据模型里是一等公民（`CatalogAsset.delivery`）。
2. **R-08 的盲区**：`check-zero-network` 的判据是"源码里出现网络符号"，因此 `A→B→联网` 这种间接调用抓不到。这意味着"启动零网络"这条纪律**在新代码里依赖人工**，而不是自动判据（见 `RD-16` 的同类思路）。

## 6. 开发 / 测试面（哪些会进正式产品）

| id | 面 | 会进正式产品？ | 说明 | 证据 | 等级 |
| --- | --- | --- | --- | --- | --- |
| DT-01 | `scripts/preset-test-server/*`（fixtures v1/v2 + `node:http` 服务） | **否** | 本地假云端脚手架（node 脚本，不进二进制） | `server.mjs::32` | 已证实 |
| DT-02 | `MKPSE_PRESET_SOURCE_URL` 覆盖 | **否** | 仅 `#[cfg(debug_assertions)]` 认 | `source.rs::430` | 已证实 |
| DT-03 | 工作台 `dev_source` / `sandbox` | **否** | 整棵 `src/workbench/` 在 feature 门内 | `lib.rs::41` | 已证实 |
| DT-04 | `scripts/check-bundle.mjs` | **否**（CI 工具） | 判据：客户端 `dist/` 不许含开发源 TOML / 模拟数据 / 工作台内容 | `check-bundle.mjs::27` | 已证实 |
| DT-05 | `scripts/probes/*`（11 个 playwright 探针） | **否** | 实机走查脚本；是前端"测试"的实际替代物 | `scripts/probes/presets.mjs` | 已证实 |
| DT-06 | `src/api/mockServer/*` | **取决构建** | `npm run dev`（非 Tauri）时的假后端；进不进 `dist` 由 `check-bundle` 管 | `check-bundle.mjs::27` | 部分证实 |

## 7. 这一层的结论

1. **发布链路是这仓库里最规整的部分**：两条链（软件本体 / 预设数据）严格分开、发布事务有"最终一致性核对短路"、有 4 条静态判据守着（bundle 内容、启动零网络、Channel 参数、release 源断代）。
2. **下载链路的口径也清楚**：目录给期望值时校验、旧份归档、原子落盘、事件记账、无第三条更新路径。
3. **薄弱处集中在两端**：
   - 判据是"源码符号级"的，抓不到间接调用（R-08）；
   - 客户端侧的**消费面**（预设页/首页）与云端链路之间存在状态口径不一致（CF-01 / CF-08 / X-05），这不是发布链路的问题，而是"本机状态"的表达问题。
