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
2. ⬜ **发布接上**：`presets/` → publish → catalog / manifest → 发布源。产出要明确回答
   "这次发布了什么 / 叫什么 / path 是什么 / SHA / 大小 / 什么 kind"；`MKPSE_PRESET_SOURCE` 只决定"发到哪"。
   **工作台 dist 现在还是 `assets/…` 布局，与客户端 `mkp/…` 不同形 —— 这一刀必须对齐**，
   否则将来 URL 拼得上、落点却对不上（已登记在 §5）。
3. ⬜ **Preset 成为第一个完整消费者**：发现 → 下载 → 本地文件 → 页面，再一层层加
   下载状态 / 更新 / 修改 / 归档 / SHA 异常 / 用户版本。

## 4. 续做入口（从哪接手）

- **第三圈第 1 步已收口，下一刀是 §3.5 第 2 步：把工作台发布的布局从 `assets/…` 对齐成 `mkp/…`**（否则 URL 拼得上、落点却对不上）。**官方源 / Gitee 的真实地址**落进发布流水线（`MKPSE_PRESET_SOURCE`）是产品决定，等发布那一刀做。
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
| `crates/preset/tests/builtin_presets_match_dir` | 旧世界判据，仍有效 |

## 7. 仓库状态速记

- main 与 origin/main 同步，最新提交见 `git log`；CI 两个 job（web / rust）必须全绿。
- 未入库的 untracked：无（`workbench/.draft/` 被 ignore 属预期）。
- **下载要配数据源（本轮起）**：真机上先在同步页填 `Preset Source` 那个地址（官方源 / Gitee / 本地 `python3 -m http.server` 都行），下载与检查更新才能跑；没填时它们拒绝执行并说明去哪儿填——不猜 URL、不假装成功。`MKPSE_PRESET_SOURCE=<url>` 可把默认值编进二进制。
- 前端底账全在 Internal 根（C4 后）：真机调试时 WebView 的 localStorage 只住偏好（置顶/搜索词），清掉不影响任何底账；旧的 `mkp.a40.*` 三格已无人读，残留可删。
- **预设页现在读 catalog**（`<appDataDir>/catalog.json`）：改了 `presets/` 源要重跑 `cargo run --bin gen-catalog`，否则判据红；真机调试时删掉旧的 `<appDataDir>/presets/` 目录不会再有影响（没人读它了）。
- **界面素材全在 `src/app/assets/`**（2026-10-01 起）：品牌 logo `bambuLogo.ts`、机型整机图 `printers/`、测试模型合影 `hero/`。换一张图 = 换一个文件（引用方改成 `import`），不重跑 `gen-catalog`、不改 `presets/`、不碰资产台账。`public/` 里只该有：台账管的载荷根 `assets/{bbs,icons,models}` 与 BBS 页元数据 `bbs/`。
- **资产去哪一档看一把尺子**：产品数据资源（用户下载 / 更新 / 管理）→ `public/assets/` + `presets/assets.toml` + catalog；界面展示素材（程序自己看一眼）→ `src/app/assets/` 或 `public/`，不进台账。台账里今天 15 条 = 9 BBS + 3 图标 + 3 模型。
