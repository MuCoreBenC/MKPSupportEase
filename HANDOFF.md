# 交接：数据架构重做 —— 第一圈完成，第二圈进行中

> 更新时间：2026-10-01
> 适用分支：`main`（只认 CI 绿的 tip；推送用 `ALLOW_PUSH_MAIN=1`，提交用 `ALLOW_COMMIT_ON_MAIN=1`——本地闸，见 §5）
> 上下文：按《四圈舞步》重做数据架构。**根规则**是 `docs/DATA-ARCHITECTURE.md`（四层世界 + 四条铁律 + 十问 + 准入问句），**对账单**是 `docs/DATA-INVENTORY.md`（43 件现状逐件归位 + 收口进展日志）。这两份先读，本文只讲"现在在哪、接下来去哪"。

## 0. 进度总览

| 圈 | 内容 | 状态 |
|---|---|---|
| 第一圈 | 骨架全立起来：数据世界 / 最小 Catalog / 文件系统 / Delivery 骨架 / 用户数据 / 页面闭环 | ✅ **100%**（六块全通，2026-10-01） |
| 第二圈 | 每块地基做厚 | ✅ **100%**（六项全通，2026-10-01：更新与归档 / R11 共用契约 / catalog 加厚换源 / **C4 localStorage 退役** / **Delivery 加厚（真数据源上线）** / **判据 2 启动零网络**） |
| 第三圈 | 统一资产入口（Catalog + Source + Delivery 一个世界），再让发布成为生产者，最后 Preset 成为第一个完整消费者 | 🔄 **第 1 步已收口**（三刀：BBS → 模型+图标 → 整机图剥离，见 §3.5）；下一步是发布布局对齐 |
| 第四圈 | 完整产品行为（三状态流转 / 冲突 / SHA 异常边界 / UI 状态） | ⬜ 未开始 |

**整体 ≈ 45%。** 判断依据：数据架构的四条主链（说明书=catalog、下载=mkp/、使用中=run/、更新=归档管道）全部收进 Internal 根，localStorage 不再住任何底账（默认 168 条 + workbench 349 条 Rust 测试、总纲判据落地 4 条）；**下载端与"零网络依赖"这条旧账也清了**——真数据源（HTTP）已经接进管道，"能不能联网"不再是空位、而是一个有判据的事实；**资产这一侧也分干净了**：产品数据资源走 Catalog + Delivery（第三圈第 1 步收口），界面展示素材随程序本体、不进台账。剩下的是业务侧（Preset 全功能 / 报告）还没接进新地基。

## 1. 第一圈留下的东西（全部在 main 上）

| 块 | 落点 | 一句话 |
|---|---|---|
| ① 数据世界 | `src-tauri/src/runtime/`（mod/paths/release/catalog/delivery/state） | 新世界的落点、释放口、说明书、管道、状态全在这一个模块 |
| ② 最小 Catalog（已加厚） | `cargo run --bin gen-catalog` → `src-tauri/src/runtime/catalog.generated.json` | 读 `presets/` 源 + `crates/preset/assets/presets/` 入库产物真字节（SHA/大小），编进二进制；**definition（机型/资产/套餐/字段定义/布局）全部进 catalog**，类型复用 `presetdata` 的 serde 形态；判据测试守"重建逐字节一致" |
| ③ 文件系统 | 复用 `fsx/`（两根 + 防穿越 + atomic_write）+ `runtime/paths.rs`（catalog.json / mkp/ / archive/ 落点） | 业务不许自拼目录名；新增子目录先改总纲 |
| ④ Delivery | `runtime/delivery.rs`：`Source` trait + `deliver` + `FileOnDisk` 三分态 | 源可插拔（现只有 LocalDirSource）；SHA/大小校验在落盘前；更新=重跑管道，旧份自动归档 |
| ⑤ 用户状态 | `runtime/state.rs` | 规则：一种状态一个文件、住 `run/` 下、atomic 写、坏档不静默。第一个住客：使用中指针（`run/active-preset.json`，全局唯一在构造上成立，带应用时刻 SHA 可查漂移） |
| ⑥ 页面闭环 | 同步页「数据骨架（新）」区（`src/app/pages/PagePackage.tsx`） | 读数 + 下载 + 使用/撤销 + 更新，全走新 API（`get_runtime_catalog` / `download_runtime_file` / `get_downloaded_files` / `get_stale_files` / `get_active_preset` / `apply_active_preset` / `clear_active_preset`——两份命令清单同步注册） |

## 2. 怎么验证（每一轮都要全绿才推）

```bash
cargo test                                   # 默认 feature（在仓库根跑，workspace 全成员）
cargo test -p mkp-support-ease --features workbench --lib   # **workbench 的判据也要跑**（CI 两个都跑）
cargo clippy --all-targets -- -D warnings    # clippy 两种 feature 都要：默认 + --features workbench
cargo fmt --check                            # CI 有格式 job
npm run build && npm run check:bundle        # 前端构建 + 判据 1（安装包产物扫描）
npx eslint <改过的文件>                       # CI 跑全量 lint
```

改了 `presets/` 源或交付产物后必须 `cargo run --bin gen-catalog` 重生成，否则判据测试 `embedded_matches_rebuild` 红。

## 3. 第二圈还剩什么（按建议顺序）

1. ~~**R11：manifest 升为两端共用契约**~~ ✅（2026-10-01：工作台发布产出 `presets/dist/catalog.json`，客户端 `check_remote_update` / `apply_remote_update` 接上）
2. ~~**catalog 加厚 + 换源收口（R3/R4/R5）**~~ ✅（2026-10-01，本轮）：
   - **加厚**：`runtime::Catalog` 长出完整 definition——brands / 机型的全部字段（含尺寸与禁区）/ assets（19）/ bundles（5）/ registry（74 条字段定义 + 页签元数据 + 布局）。definition 类型**直接复用** `presetdata` 的 serde 类型；revision 指纹把 definition 算进输入；`CATALOG_SCHEMA` 不升（加字段不升号，definition 全带 `#[serde(default)]`）。
   - **换源**：`ipc/presets.rs` 九条命令的 DTO 一个没动，数据源换成释放的 catalog（`<appDataDir>/catalog.json`，按**字节**缓存，目录换新自动失效——没有失效钩子可忘）。MKP 引用从此带真 size/SHA。三层取值提取成 `resolve::visible_keys_of / effective_of`，与 ParamRegistry 共用。
   - **退役**：`client/` 目录（13 份 `include_str!` + seed 铺盘）删除，启动铺盘只剩 catalog 一份。**总纲欠账 #1 收口，判据 4 落地**（`dto_builders_read_the_catalog_and_nothing_else`）。
3. ~~**C4：旧世界 localStorage 三格退役**~~ ✅（2026-10-01，本轮）：
   - **active 格 → `run/active-preset.json`**：预设页「应用」/ 首页与校准页反填 / 参数页默认落点切 `getActivePreset` / `applyActivePreset`；`ActiveEntry`（kind/ref）退役，统一契约的 `ActivePreset`（fileName + SHA + intact），「已应用」判据 = 文件名相等。
   - **presets 格 → `mkp/`**：预设页"工作台发布"两列换成**目录交付**（catalog.files，大小真值）+ 下载区（盘就是底账）；下载走 `downloadCatalogFile`；`fetchPreset`（TOML 正文塞 localStorage）退役。
   - **package 格 → catalog**：参数页整页切 catalog 命令（页签/分组树从 `getRuntimeCatalog` 的 registry 摊——契约加厚了 `CatalogRegistry` 声明，mock 用同一批快照摊同形状）；同步页旧"自动同步说明书"区退役。
   - **判据在编译期**：三键从 `STORAGE` 删除，再引用直接 tsc 失败。旧格数据不迁移——它们是演示管道的假数据，正式用户机上本来就是空的。
4. ~~**Delivery 再加厚**~~ ✅（2026-10-01，本轮）：
   - **真数据源上线**：新增 `runtime/net.rs`（ureq + rustls，**不引异步 HTTP 栈**：`ureq` 同步阻塞 + `spawn_blocking`，与其它部分同一套思路）实现 `delivery::Source`——**管道一行未改**，第一圈那句"真云端来了只换实现"兑现了。这是默认构建里第一条 HTTP 依赖，选型理由与体积成本写在 `src-tauri/Cargo.toml` 的注释里。
   - **地址是配置，不是 catalog 的一部分**（作者裁决）：新增 `runtime/source.rs`，住 `run/preset-source.json`，沿用 `runtime/state.rs` 那套状态规则；默认地址构建期可替换（`MKPSE_PRESET_SOURCE`）。**下载地址 = 数据源 baseUrl + `CatalogFile.path`**，换 Gitee / 换自建 CDN 不用重发说明书。没配就如实拒绝，不猜 URL。
   - **脚手架退场**：`download_runtime_file` 里探测仓库绝对路径那段（`presets/dist` / `crates/preset/assets/presets`）删除；检查 / 应用更新也从同一地址取清单。
   - **健壮性**：运输类故障退避重试（连接 / 超时 / 解析 / 被掐），**内容类一次都不重试**（404、500、字节超了）；超时必须有值，判据是"服务端不回答也不许把调用方吊住"。
   - **进度与并发**：一次调用一路 Tauri `Channel` 水位；批量走 `download_runtime_files` + `deliver_all`（固定 4 条道，**并发在 Rust 侧**，前端不发明并发与汇总口径），返回按请求顺序、**逐份给结局**。
5. ~~**判据 2**~~ ✅（2026-10-01，本轮）：`scripts/check-zero-network.mjs`（`npm run check:zero-network`，挂 CI web job）。三道闸：网络符号只许住 `runtime/net.rs` / 程序 `.setup()` 段零联网 / `src/api/` 不许绕过 IPC 自己发 HTTP。干净过、种脏能抓，两方向都实测。**它是源码扫描，不是运行时判据** —— 间接调用（A 调 B、B 最终联网）抓不到，那条留给将来在代理层补。

## 3.5 第三圈：统一资产入口（按作者定的三步走）

1. ✅ **资产载荷全部纳入 catalog**（✅ 2026-10-01 收口；先不谈官方源 / Gitee 最终用哪个）
   - ✅ **第一刀：BBS 配置**（`ec5009f`）——“Catalog → kind=bbs_config → Source → Delivery → SHA → `mkp/bbs/`” 闭环；
     随包副本从客户端产物退役；BBS 页改从下载区载入。**判据：真 HTTP 拉一份 BBS 配置落进 `mkp/bbs/…`**。
   - ✅ **第二刀：模型 + 图标**（`3819807`）—— 各加一个 kind（`model` / `icon`）与落点目录，
     **管道、命令、判据结构一行没改**；随包副本同样退役。
   - ✅ **第三刀：整机图剥离台账**（本次）—— 判定整机图**不是产品数据资源、而是界面展示素材**
     （用户不下载 / 不更新 / 不管理它），于是**不接进 Delivery**：
     `public/assets/printers/`（4 张 webp）→ `src/app/assets/printers/`，
     首页 `heroArt.ts` 改成 `import` 自己的应用资源；台账删掉 4 条 `type='image'`（19 → 15 条定义）、
     机型文件的 `image` 引用清空。**判据换向**：原"别忘了整机图"删除，改守
     **「资产台账里已无 image 类」**。同一条规则下的最后一件顺手搬了：测试模型合影
     （`public/models/hero_pile*.webp`，首页第五步与校准页在用）→ `src/app/assets/hero/`。
     `public/` 从此只剩台账管的载荷根与 BBS 页元数据。
     第 1 步到此收口：**Catalog / Delivery 管产品数据资源，`src/app/assets` 管界面自带素材**。
2. 🔄 **发布接上**：`presets/` → publish → catalog / manifest → 发布源。产出要明确回答
   "这次发布了什么 / 叫什么 / path 是什么 / SHA / 大小 / 什么 kind"；`MKPSE_PRESET_SOURCE` 只决定"发到哪"。
   - ✅ **第 2 步（发布布局对齐 `mkp/…`）**（本次）——交付根**就是客户端按 URL 直取的那个根**，
     所以相对路径是契约的一半，不是发布侧的自由：原来这里是 `assets/<载荷 path>` + `presets/mkp/<名字>`，
     客户端认的却是 `mkp/<kind 目录>/…`，**URL 拼得上、落点对不上**。
     现在两边**共用 `runtime::catalog::dest_of_asset` 一处算法**：`wb_generate` 落 `dist/mkp/presets/`，
     资产落 `dist/mkp/{bbs,models,icons}/…`，`assets_index.json` 的 `path`、
     `manifest` 的 `relativePath`、`catalog.json` 的 `files[].path` 三者同值。
     **判据**：`publish_into` 收尾逐条核对"发出去的每份文件都在说明书里按同一 path 登记、字节与 SHA 一致"
     （运行时）+ 真数据判据 `the_published_layout_lands_where_the_catalog_says`（写盘面 == 登记面）。
   - ⬜ **还没做的**：`MKPSE_PRESET_SOURCE` 落进流水线（"发到哪"是产品决定）；上传那一环还是手工/云盘。
3. 🔄 **Preset 成为第一个完整消费者**：发现 → 下载 → 本地文件 → 页面，再一层层加。
   **总模型**（2026-10-02 定，写在 `DATA-ARCHITECTURE.md` §1③「预设 TOML 的一生」里）：
   **官方线**（云端 → `mkp/` → `archive/`）与**用户线**（另存 → `presets-mine/`）**不许混** ——
   官方原件不可变、用户修改另存、归档只服务官方线。七步按这个模型走，一次只做一层：
   1. ✅ **官方 TOML 下载 / 已下载 / 需更新**（`ac42520`）
   2. ✅ **批量下载 / 更新**（`3c12fcf`）
   3. ✅ **官方旧版本归档及必要的可见性**（本次）
   4. ✅ **用户自己的 TOML**（`（已修改）` 那一条线）
   5. ✅ **临时编辑 → 保存 → 用户文件**（`.tmp` 从来不落进 `mkp/`）
   6. ⬜ 官方文件异常修改检测（SHA 报警：不许应用 / 不许编辑 / 不许复制，只能重下）
   7. ⬜ 官方更新与用户修改文件同时存在时的 UI / 行为
   - ✅ **第一层：下载状态 + 更新**（2026-10-02）——交付行现在答得出「**这一份在本机是什么样**」：
     未下载 / 已下载 / **需更新**（盘上有、字节与目录不符）。
     三态来自**两个读的组合**（`getDownloadedFiles` = 与目录一致、`getStaleFiles` = 不一致），
     前端不猜；`stale` 必须单独一档 —— 只问"文件在不在"会把一份旧版 / 被手动动过的文件
     说成「已下载」，而它应用时会被 SHA 校验拒掉。
     动作跟着状态走：需更新 → 「更新」（**同一条下载管道**：再下一遍、旧份进归档，
     没有第二个命令）；本地表那一行**不给「应用」**（点了必被拒 —— 不给必报错的按钮）。
     过程也如实说：单份下载挂 Channel 水位（`shared/download.ts`，与同步页**同一份文案**），
     失败分态转述后端理由（没配数据源 / 源上没有这份 / 字节不符）。
     **判据**：`scripts/probes/presets.mjs` 第 5 节（浏览器模式两态画得对、动作对、
     点「更新」如实失败、展开详情说「需更新」）；跑法见 §7。
   - ✅ **第二层：批量补齐 / 批量更新**（本次）——即"多份一起处理"，**不是第二套机制**：
     云端表上多一行，写清这一批里有几份未下载、几份需更新，一颗按钮**一次交给同一个
     `downloadCatalogFiles`**（Rust 侧逐份跑同一个 `deliver`，并发也在那边）。
     范围 = 当前机型 + 这一档类型，**不受搜索词影响**（搜索是"我在找什么"，不该改变
     "按一下动几份"）；**已下载的不进这一批**（这一层只解决"多份一起处理"，不解决"再下一遍"）。
     结局**逐份**收：提示条主句说总数、**没成的那几份各占一行带后端给的原因** ——
     既不许压成一句"批量失败"，也不许一份失败被说成整批成功；命令级失败（比如没配数据源，
     一份都没发出去）是**另一档**说法，与"逐份被拒"分开。
   - ✅ **第三层：官方旧版本归档及必要的可见性**（本次）——只解决一件事：
     **因为更新而进了 `archive/` 的旧版本，用户看得见、认得出、看得了**。
     两条读（`get_archived_files` / `read_archived_text`，都只读、只认归档区），
     一行界面（交付行的展开详情里多一格「旧版本 N 份」→ 点开是抽屉：列表 + 正文）。
     **认人不解析文件名**：按"`archive/` 去掉前缀之后与目录里哪一份同位"匹配 ——
     同位是一个不需要额外知识的事实；目录里已经没有的那几份，机型 / 版本**如实留空**。
     **不提供删除、不提供恢复、也没有"用这份旧版本"**（归档管理不在这一层）；
     读不出来就照实说，不许显示空正文。判据：探针第 5b 节 + `delivery` 那三条新判据。
   - ✅ **第四层：用户自己的 TOML**（本次）——**用户线**能读了：
     `~/Documents/SupportEase/presets-mine/` 里有什么列得出来（`get_user_preset_files`）、
     认得出是哪一类、正文看得了（`read_user_preset_text`），界面上是本地表那一半「我的文件」
     （展开详情里「看正文」→ 抽屉，**只读**）。
     **认不出就不认**：只按扩展名认 `.toml`（MKP 预设）；切片器那两类都是 `.json`，
     分不出 bbs 还是 orca → 照实 `kind: null`，这一档**在任何类型档下都列**（不藏、不猜）。
     机型这一层没有来源 → 任何机型档下都列、机上是「—」+「未标机型」。
     老的 `getLocalUserFiles`（恒空）与它的 DTO **一起退役**：那 DTO 上的「用户自己标的适用机型」
     在新世界里没有任何地方能标，留着就是两套口径 + 一个永远空的字段。
     早先"还没有写入者"——第 5 步补上了（见下）。用户那份**仍不能被应用**
     （`run/active-preset.json` 只认官方交付文件）—— 那是"用户文件算不算一种使用中的配置"
     这条产品规则的活，登记在案（第 7 步附近）。
   - ✅ **第五层：临时编辑 → 保存 → 用户文件**（本次）——官方原件**不可变**这条在代码里成立了：
     点「改这份」把官方正文复制进**临时文件**（`run/draft-preset.json`），用户改的是它
     （边改边存，700ms 停手落一次；改到一半关掉也还在）；点「保存为用户文件」才另存成
     `presets-mine/<原名>（已修改）<后缀>`，然后丢掉草稿。**官方原件与下载区全程没被碰过** ——
     判据逐字节盯着（另存前后 `mkp/` 里那份的字节不变、下载区文件数不变）。
     **临时文件为什么不跟官方原件同目录**（示意里的 `A1MF_260701.tmp.toml`）：`mkp/` 的判据是
     "盘上每个文件都在目录里登记"（`stale_files` 靠它认陈旧），塞 `.tmp` 进去立刻变成
     "目录里没有的陈旧文件"，把交付那层的判据全污染了 —— 草稿是**运行状态**，住 `run/`。
     入口只给"与目录一致"的交付行（没下载就没正文可改；需更新那份内容存疑，先更新再改）；
     **关掉编辑器 ≠ 放弃**（草稿还在，回头接着改）；真正丢草稿只有「放弃这次编辑」那一颗。

## 4. 续做入口（从哪接手）

- **第三圈第 2 步的布局对齐已完成**（2026-10-02）：交付根相对路径 = 客户端下载区的相对路径（`mkp/…`），两端共用 `runtime::catalog::dest_of_asset`。**下一步是 §3.5 第 3 步：Preset 成为第一个完整消费者**（发现 → 下载 → 本地文件 → 页面，再逐层加下载状态 / 更新 / 修改 / 归档 / SHA 异常 / 用户版本）。**官方源 / Gitee 的真实地址**落进发布流水线（`MKPSE_PRESET_SOURCE`）是产品决定，等那一刀做。
- **整机图那一刀的遗留（已登记，不是漏做）**：`presetdata::AssetKind::Image` 变体与机型 `image` 字段还在（值为空）。**schema 暂不清理**（2026-10-02 定）："现在没有数据"不等于"这个概念永远不存在"，保持 schema 稳定，将来确认不用了再单独做一次 schema 清理（会连带改前端契约 `Machine.image`、mock、工作台机型页与资产页）。工作台的"机型图"筛选页签与机型图下拉现在**如实为空**（台账里确实没有这一类），不是坏了。界面素材已全部搬离 `public/`（整机图 + 测试模型合影），`public/` 只剩载荷根与 BBS 页元数据。
- **下载这条链的入口**：`ipc::catalog::download_runtime_file`（一份，带 `Channel` 进度）与 `download_runtime_files`（一份清单，逐份结局）；源由 `runtime::source::current_base_url` 给出；字节在生产后走 `runtime::delivery::deliver`。
- 前端消费新世界的样板：预设页（`usePresetData.readRelease` = catalog.files + `getDownloadedFiles`，写走 `downloadCatalogFiles` / `applyActivePreset` 后**重读底账**）；参数页（`useParams` = `getMachines` + `getParamMeta` + `getRuntimeCatalog` 的 registry 摊页签树 + `getMachineParams` 按 combo 拉值）。
- 更新/归档：`runtime/delivery.rs` 的 `FileOnDisk` / `stale_files` / deliver 里的归档段；`runtime/release.rs` 的升级策略。
- catalog（现在很厚了）：类型与构建在 `runtime/catalog.rs`（definition 复用 `presetdata` 的 serde 类型，改语义才升 `CATALOG_SCHEMA`）；消费端只读访问面在它的 impl 块。
- 命令面：`src-tauri/src/ipc/catalog.rs`；**注册必须两份清单同步**（`lib.rs` 两个 `generate_handler!`）。
- 三层取值共用算法：`presetdata/resolve.rs` 的 `visible_keys_of` / `effective_of`（ParamRegistry 与 catalog 两边转调，**不许分叉**）。
- 假云端退役：`src/workbench/cloud.ts` + `src/workbench/fixtures/cloud-presets.json`（静态快照已挪出 public/，客户端产物已无假数据，判据 1 盯着）。

- **网络只许住 `runtime/net.rs`**（2026-10-01 起由 `check:zero-network` 盯着）：

  - 要联网就在那一个文件里加，别在自己顺手的地方 `reqwest` / `fetch` 一下 —— 那一下同时破了"唯一可写主人"与判据 2；
  - **前端不许自己发 HTTP**（`src/api/` 里不许出现 `fetch`）：与后端的对话只有 Tauri IPC 一条。过去不喜欢本地 HTTP 是因为本地端口会被占、本机也被当成服务器；Rust → 远端的**出站**请求没有这两个问题，所以这条只针对"前端绕过命令层"；
  - `Source` trait 是管道唯一的口子：将来换 HTTP 库、加断点续传、加镜像源，都只动 `net.rs`，`delivery.rs` 不动。

## 5. 已知纪律 / 不要碰（每一条都是踩过的坑）

- **协作边界：什么事停下来问作者，什么事自己定**（2026-10-02 定死，不要再逐项回问）：
  **只有这四类停下来问** —— ① 产品行为变了；② 数据归属变了；③ 公开契约 / API / schema 的
  方向变了；④ 与已确定的架构原则冲突，或存在需要作者做产品取舍的多个方案。
  **其余一律自己定**：内部实现细节、代码组织与文件放哪、测试与判据怎么改、
  旧 schema 要不要暂时保留空位、同类资源要不要顺手迁移、叫什么名字 ——
  只要不改变产品行为、不违背已定的架构原则，就别拿回去让作者拍板。
  两条以上都合理时的优先序：① 守已定的总纲 → ② 不引入新的兼容层 / 临时方案 →
  ③ 守住当前这一圈的边界 → ④ 选改动更小、下一条最容易接的那个。
  **决定与理由写进台账**（本文件 §3 / §5 或 `DATA-INVENTORY.md` 的进展日志），
  不要在汇报里写成"要不要我做"的问句。作者定舞步，这一侧负责把舞步跳完整。
- **总纲准入问句**（`DATA-ARCHITECTURE.md` §6）：任何新文件/新功能先答"属于哪一层？谁是唯一主人？什么时候允许联网？"答不出先改文档。
- **四条铁律**：开发文件不当运行时数据库；云端不参与首屏；用户没下载的不预置（catalog 是唯一例外——它是软件本体）；运行时只认自己的运行时数据。
- **clippy 禁列对测试也生效**（CI 是 `--all-targets`）：测试里写盘用 `fsx::atomic::atomic_write`，`std::fs::write` 会红。
- **`b"..."` 字节串装不下中文**：含中文用 `"…".as_bytes()`。
- **cargo fmt 是 CI 的独立 job**：推前跑 `cargo fmt`。
- **本地 clippy 缓存会掩盖新 lint**（2026-10-01 踩过）：CI 工具链 track stable 会自己升级，
  新版 clippy 抓出本地旧版放过的代码（如 single_element_loop）。本地看着绿不一定是真的绿——
  推前用 `cargo clean -p mkp-support-ease && cargo clippy --all-targets -- -D warnings` 才算数。
- **`presets/dist/` 是本机产物，不入库**；判据/构建器的输入用入库真身 `crates/preset/assets/presets/`（9 份，`BUILTIN_PRESETS` 编的就是它）。
- **~~新旧世界并存、互不读写~~（已收口）**：`client/` 目录删除，客户端只读 catalog、工作台读仓库源，仍互不读写；收口一条勾一条，同步更新 `DATA-INVENTORY.md` §4 与总纲 §4。
- **`workbench/.snapshots/` 已入库**（`.gitignore` 的既定政策：除 `.draft/` 外入库）；`.codebuddy/` 已 ignore；`.trae/documents/` 留库（被代码注释引用）。
- **仓库有并行会话在动**：推送前先 `git fetch`；合并冲突大概率在 `ipc/presets.rs` / `usePresetData.ts`（预设页是热点）。
- **真机下载要先配数据源**：C4 之后的下载不再探测仓库路径 —— 没配 `Preset Source`（同步页那格）时，下载 / 检查更新都会拒绝并说明去哪儿配。**这是对的**，不是 bug：用户机器上本来就没有仓库。开发期要验真，把本地静态目录（如 `python3 -m http.server`）的地址填进去即可。
- **随包资产退役是"接一类摘一类"**：`vite.config.ts` 的 `DELIVERED_ASSET_DIRS` 与
  `scripts/check-bundle.mjs` 的 `assets/<类>/` 是**同一份清单的两处**，加一类就改两处（判据会盯着另一处）。
  今天这份清单恰好覆盖 `public/assets/` 下台账管的全部三类（bbs / models / icons）。
- **整机图不在这份清单里**（2026-10-01）：它已经不经过 `public/` —— 住 `src/app/assets/printers/`，
  由 vite 资源管线打进 `dist/assets/`（带内容哈希）。**它的边界判据在 Rust 侧**：
  `runtime::catalog` 的「资产台账里已无 image 类」——别拿目录名去 `check-bundle` 堵它。
- **判一类东西进不进 Catalog 的尺子**：**"它是不是产品数据资源"**，不是"文件是不是图片"。
  用户要下载 / 更新 / 管理它 ⇒ 进台账、走 Delivery；界面为了展示自己取一张 ⇒ 住 `src/app/assets/`。
  整机图那一刀就是按这把尺子改判的（原来写在台账里的 4 条 `image` 条目已删）。
- **下载区按 kind 分层**（`mkp/presets/`、`mkp/bbs/`…）：新增资源只加 [`runtime::catalog::kind`] 一个常量
  + [`kind_dir`] 一个分支。**不要**为某一种资源另开一个目录另写一套下载 —— 那是这一圈唯一的硬规矩。
- **交付根的相对路径 = 客户端落点**（2026-10-02）：发布根就是数据源地址指向的根，
  客户端下载地址 = `数据源地址 + catalog 的 `files[].path``。所以**发布侧不许自己拼路径** ——
  算落点只有一处：[`runtime::catalog::dest_of_asset`]（`wb_generate` 的 `mkp/presets/` 也照它）。
  两套拼接的典型症状是"上传成功、用户点下载却 404"，而两边各自看着都对。
  判据在 `publish_into` 收尾（发出去的每份都在说明书里按同一 path 登记、字节与 SHA 一致）。

## 6. 判据清单（全是活的，别删）

| 判据 | 守什么 |
|---|---|
| `runtime::tests::embedded_matches_rebuild` | 编进二进制的 catalog = 重新构建的那份（源/产物改了没重跑 gen-catalog 就红） |
| `runtime::catalog` 5 条 | 5 机型 9 文件 / definition 随目录走（15 资产 5 套餐 74 字段，反空转计数 + 与台账逐条对齐） / **资产台账里已无 image 类**（整机图剥离的判据）/ **JSON 往返无损** / 指纹跟输入（含 definition） / 拒未来代次 |
| `ipc::presets` 判据 | **DTO 构建只吃 catalog（判据 4 的钉子）** / 缓存按字节配对与自失效 |
| `runtime/delivery` 13 条 | 校验在落盘前 / 防穿越 / 更新归档旧份 / 归档槽保最早 / 幂等 / Stale 可见 / **批量：每份都落盘 / 结果按请求顺序 / 坏档不拖累别人 / 空清单不动手** |
| `runtime/net` 12 条 | **真 HTTP 拿到字节** / 水位单调且带文件名 / **抖一次真重试成功（不是数次数）** / 重试有上限 / 404 不重试 / 撒谎的字节不重试 / 不回答的服务端不会吊死调用方 / Source 拼的是 catalog 的 path / 清单与文件同地址 / 阶段词稳定 |
| `runtime/source` 7 条 | 往返 / 没配不是坏档 / 坏档不静默 / 未来代次拒 / 空地址不写盘 / **只放出站 HTTP（挡住 file:// 等）** / 拼 URL 容错 |
| `runtime/state` 7 条 | 往返 / 缺省 None / 坏档 CORRUPTED / 未来代次拒 / 后应用赢 / 撤销幂等 / 漂移检测 |
| `runtime/release` 4 条 | 首启铺 / 同版本不动 / 升级归档换新 / 归档槽保最早 / mkp/ 初始为空 |
| `STORAGE` 三键已删（C4） | localStorage 不再住底账：`STORAGE.clientPackage / clientPresets / clientActive` 引用直接 tsc 失败——编译期判据 |
| `scripts/check-bundle.mjs`（CI web job） | 判据 1：开发源 TOML / 模拟数据 / 工作台内容不进客户端安装包 |
| `scripts/check-zero-network.mjs`（CI web job） | 判据 2：启动零网络 —— 网络符号只许住 `runtime/net.rs` / `.setup()` 段零联网 / `src/api/` 不许绕过 IPC 发 HTTP |
| `workbench::app::dist` 判据 | **交付根相对路径 == 客户端落点**：`the_published_layout_lands_where_the_catalog_says`（真数据：写盘面 == catalog 登记面、字节与 SHA 一致）/ 夹具发布：manifest 每条都在 `mkp/…` 下且 catalog 里按同一 path 登记 / 残留拦截 / 回收站 / 可达集计数（真数据 8 条资产） |
| `crates/preset/tests/builtin_presets_match_dir` | 旧世界判据，仍有效 |
| `runtime::delivery` 归档 3 条 | `archived_files_is_empty_before_anything_was_archived`（没归档过 = 空表，不是错误）/ `…lists_what_the_update_pushed_aside`（列出来的按它自己的路径能读回**旧版本的原字节**）/ `…reflects_the_single_archive_slot`（连升两版仍只列一份 = 最早那份：**归档 ≠ 每次更新的历史**） |
| `runtime::mine` 用户线 4 条 | `mine_files_is_empty_when_the_user_has_nothing`（一份都没有 = 空表，不是错误）/ `…lists_what_the_user_put_there`（路径相对用户根、按升序、子目录也算、只出 `presets-mine/`）/ `kind_is_only_claimed_when_the_extension_says_so`（`.json` **照实认不出**）/ `only_the_mine_subdir_is_readable`（读正文只认那一格：`exports/`、`..`、目录本身都拒） |
| `runtime::mine` 另存 2 条 | `commit_writes_the_edited_copy_and_leaves_the_official_alone`（**核心不变式**：另存只写用户根、官方原件字节不变、下载区文件数不变、再存是覆盖它自己）/ `the_committed_copy_shows_up_in_mine_files`（另存出来的那份接着就能被列出来、读得回来） |
| `runtime::state` 草稿 4 条 | `draft_roundtrips_and_clears`（存/读/丢，丢是幂等）/ `absent_draft_is_none_not_error` / `corrupted_draft_is_an_error`（坏档不静默）/ `draft_and_active_are_separate_files`（**改一份 ≠ 在用它**：两个状态文件互不干扰） |
| `scripts/probes/presets.mjs`（**手工**，非 CI） | 预设页探针：两轴可点 / 四张表可读 / 点行展开 / 右键菜单 / BBS 入口跨页 / **交付行的两态与动作（已下载·灰字、需更新·按钮）** / **批量那一层（批次行只含未下载+需更新、逐份结局各占一行且不许伪装成功）** / 控制台无 error、无 ≥400 响应。跑法见 §7；截图落 `tmp-shots/`（已 gitignore） |

## 7. 仓库状态速记

- main 与 origin/main 同步，最新提交见 `git log`；CI 两个 job（web / rust）必须全绿。
- 未入库的 untracked：无（`workbench/.draft/` 被 ignore 属预期）。
- **下载要配数据源（本轮起）**：真机上先在同步页填 `Preset Source` 那个地址（官方源 / Gitee / 本地 `python3 -m http.server` 都行），下载与检查更新才能跑；没填时它们拒绝执行并说明去哪儿填——不猜 URL、不假装成功。`MKPSE_PRESET_SOURCE=<url>` 可把默认值编进二进制。
- 前端底账全在 Internal 根（C4 后）：真机调试时 WebView 的 localStorage 只住偏好（置顶/搜索词），清掉不影响任何底账；旧的 `mkp.a40.*` 三格已无人读，残留可删。
- **预设页现在读 catalog**（`<appDataDir>/catalog.json`）：改了 `presets/` 源要重跑 `cargo run --bin gen-catalog`，否则判据红；真机调试时删掉旧的 `<appDataDir>/presets/` 目录不会再有影响（没人读它了）。
- **界面素材全在 `src/app/assets/`**（2026-10-01 起）：品牌 logo `bambuLogo.ts`、机型整机图 `printers/`、测试模型合影 `hero/`。换一张图 = 换一个文件（引用方改成 `import`），不重跑 `gen-catalog`、不改 `presets/`、不碰资产台账。`public/` 里只该有：台账管的载荷根 `assets/{bbs,icons,models}` 与 BBS 页元数据 `bbs/`。
- **资产去哪一档看一把尺子**：产品数据资源（用户下载 / 更新 / 管理）→ `public/assets/` + `presets/assets.toml` + catalog；界面展示素材（程序自己看一眼）→ `src/app/assets/` 或 `public/`，不进台账。台账里今天 15 条 = 9 BBS + 3 图标 + 3 模型。
- **临时编辑那条链的落点**：编辑中的正文住 `<appDataDir>/run/draft-preset.json`
  （**不是** `mkp/`：那是下载区，判据是"每个文件都在目录里登记"，塞 `.tmp` 进去就污染交付那层）。
  保存 = 另存成 `<Documents>/SupportEase/presets-mine/<原名>（已修改）<后缀>`，然后丢掉草稿。
  四条命令：`begin_preset_edit` / `put_preset_draft` / `discard_preset_draft` / `commit_preset_draft`。
  界面：本地表与云端表的交付行（**与目录一致**的那种）展开详情里有「改这份」→ 编辑器抽屉；
  关掉抽屉**不丢**（草稿在盘上，回头点「改这份」接着改），丢草稿只有 footer 那颗「放弃这次编辑」。
- **用户自己那一份住哪**：`~/Documents/SupportEase/presets-mine/`（**用户根**，与内部根分开 ——
  程序管的数据不放 Documents，因为 iCloud 会把文件驱逐成占位 stub，见 `fsx::paths`）。
  两条只读命令：`get_user_preset_files`（列）/ `read_user_preset_text`（读正文，**只认那一格**）。
  **没有任何写命令** —— 临时编辑 → 保存是第 5 步的事，所以今天这个目录是空的也正常。
  界面上它在本地表里（「我的文件」那一半，展开详情里有「看正文」→ 抽屉，只读）。
  认不出的类别（`.json`）照实写「认不出是哪一类」，在任何类型档下都列。
- **归档（`archive/`）住哪、怎么来的**：`<appDataDir>/archive/` + **交付根相对路径**
  （`archive/mkp/presets/A1-fast.toml`）—— 与 `mkp/` 同形，所以"归档里这份是谁"不用猜：
  去掉前缀与目录里哪一份同位，就是谁。只有"换版本"往里放东西，**保留最早一份**（不覆盖、不删）。
  界面上它挂在交付行的展开详情里（「旧版本 N 份」→ 抽屉，能看正文）。
  **三条读只读不写**：删除 / 恢复**连命令都没有**（归档管理不在这一层）。
  它服务的是**官方线**，不是用户修改历史 —— 见总纲 §1③「预设 TOML 的一生」。
- **预设页那条链路怎么手工看**（浏览器模式 = 假后端）：`npm run build` →
  `npx vite preview --port 4173 --strictPort` → 开 `http://localhost:4173/` 的「预设」页 ——
  交付行按假后端的**固定演示集合**画两态（`A1-standard.toml` 已下载、`A1-fast.toml` 需更新）；
  点「更新」会如实报「未实现的接口」（浏览器里没有盘、没有源）。
  自动化跑一遍：`node scripts/probes/presets.mjs`（要 `playwright-core` + Edge；截图落 `tmp-shots/`）。
  **别用 dev（5321）**：那台 watcher 会扫 `target/` 下几万个文件，自己把自己拖死（探针文件头也这么说）。
- **交付根（`presets/dist/`）的布局**：`catalog.json` + `manifest.json` 在根，产品资源一律在 `mkp/…`
  （`mkp/presets/` 是 `wb_generate` 落的、其余按 kind 分目录）——**与客户端下载区同形**。
  本机换过布局时，旧目录里的文件会成"残留"：发布页有清理（进 `workbench/.trash/dist/`），
  也可以直接把 `presets/dist/mkp/presets/` 之外的东西清掉重发（它本来就不入库）。
