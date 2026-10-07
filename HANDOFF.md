> ⚠️ **历史快照（2026-10-05 加注）**：本文写于寻址改造之前，其中的 `presets/dist/`、
> `baseUrl` 同目录推导、release.json 位置等表述是**当时的现状记录**，不再是现行规则。
> 现行规则见 `docs/RESOURCE-ADDRESSING-ROADMAP.md` 与 `docs/PUBLISH-ARCHITECTURE.md` §2.2。

# 交接：数据架构重做 —— 第一圈完成，第二圈进行中

> 更新时间：2026-10-01
> 适用分支：`main`（只认 CI 绿的 tip；推送用 `ALLOW_PUSH_MAIN=1`，提交用 `ALLOW_COMMIT_ON_MAIN=1`——本地闸，见 §5）
> 上下文：按《四圈舞步》重做数据架构。**根规则**是 `docs/DATA-ARCHITECTURE.md`（四层世界 + 四条铁律 + 十问 + 准入问句），**对账单**是 `docs/DATA-INVENTORY.md`（43 件现状逐件归位 + 收口进展日志）。这两份先读，本文只讲"现在在哪、接下来去哪"。

## 0. 进度总览

| 圈 | 内容 | 状态 |
|---|---|---|
| 第一圈 | 骨架全立起来：数据世界 / 最小 Catalog / 文件系统 / Delivery 骨架 / 用户数据 / 页面闭环 | ✅ **100%**（六块全通，2026-10-01） |
| 第二圈 | 每块地基做厚 | ✅ **100%**（六项全通，2026-10-01：更新与归档 / R11 共用契约 / catalog 加厚换源 / **C4 localStorage 退役** / **Delivery 加厚（真数据源上线）** / **判据 2 启动零网络**） |
| 第三圈 | 统一资产入口（Catalog + Source + Delivery 一个世界），再让发布成为生产者，最后 Preset 成为第一个完整消费者 | ✅ 第 1、2 步收口；第 3 步（Preset 消费者）**十二层 + 第十三层收尾全部收口**、**第十五层（预设页数据边界修正）收口**、**「同步」页退役 + 最小设置页**（2026-10-02，见 §3.5）→ 用户文件生命周期闭环（创建 / 修改 / 管理 / 使用 / 外部管理）；**Bootstrap 官方源接通**（第十七刀）→「同步系统」从用户功能降为内部基础设施；**参数页底座 ①–④ 完成**（PR #22 = `2da318c`：补 `toml_key` / `patch_preset_toml` / 参数页接草稿链 / 操作记录底座）；「分享」不做、单文件「导出」暂缓（见 §5）；**A43 参数页移植收口**（2026-10-02，分支 `feat/a43-params-port`：deprecated 显示但只读 / 受控参数树 / Plate 独立实体 / 塔地图消费 Plate，见 §3.5）；**随包资产同步链四刀收口**（2026-10-03，分支 `feat/client-assets-pipeline`：台账 `delivery` → 对账式同步 → `client-assets/` → vite 装配；客户端取图改台账驱动 + 图位分层（品牌图 / 机型图 / 版本图）；资产检查面板 + 「在访达中显示」；`public/assets` 与 npm 前置退役，见增量之八～十一）；**当前阶段还剩两块**：① 修改历史 UI（独立原型）②「设置 → 备份与恢复」（ZIP）——**做完这两块再统一整理**（见 §3.5「整理那一刀」） |
| 第四圈 | 完整产品行为（三状态流转 / 冲突 / SHA 异常边界 / UI 状态） | ⬜ 未开始 |

**整体 ≈ 50%。** 判断依据：数据架构的四条主链（说明书=catalog、下载=mkp/、使用中=run/、更新=归档管道）全部收进 Internal 根，localStorage 不再住任何底账（默认 168 条 + workbench 349 条 Rust 测试、总纲判据落地 4 条）；**下载端与"零网络依赖"这条旧账也清了**——真数据源（HTTP）已经接进管道，"能不能联网"不再是空位、而是一个有判据的事实；**资产这一侧也分干净了**：产品数据资源走 Catalog + Delivery（第三圈第 1 步收口），界面展示素材随程序本体、不进台账。剩下的是业务侧（Preset 全功能 / 报告）还没接进新地基。

## 1. 第一圈留下的东西（全部在 main 上）

| 块 | 落点 | 一句话 |
|---|---|---|
| ① 数据世界 | `src-tauri/src/runtime/`（mod/paths/release/catalog/delivery/state） | 新世界的落点、释放口、说明书、管道、状态全在这一个模块 |
| ② 最小 Catalog（已加厚） | `cargo run --bin gen-catalog` → `src-tauri/src/runtime/catalog.generated.json` | 读 `presets/` 源 + `crates/preset/assets/presets/` 入库产物真字节（SHA/大小），编进二进制；**definition（机型/资产/套餐/字段定义/布局）全部进 catalog**，类型复用 `presetdata` 的 serde 形态；判据测试守"重建逐字节一致" |
| ③ 文件系统 | 复用 `fsx/`（两根 + 防穿越 + atomic_write）+ `runtime/paths.rs`（catalog.json / mkp/ / archive/ 落点） | 业务不许自拼目录名；新增子目录先改总纲 |
| ④ Delivery | `runtime/delivery.rs`：`Source` trait + `deliver` + `FileOnDisk` 三分态 | 源可插拔（现只有 LocalDirSource）；SHA/大小校验在落盘前；更新=重跑管道，旧份自动归档 |
| ⑤ 用户状态 | `runtime/state.rs` | 规则：一种状态一个文件、住 `run/` 下、atomic 写、坏档不静默。第一个住客：使用中指针（`run/active-preset.json`，全局唯一在构造上成立，带应用时刻 SHA 可查漂移） |
| ⑥ 页面闭环 | ~~同步页「数据骨架（新）」区~~（2026-10-02 随「同步」页退役删除；能力早已化入预设页的下载 / 应用那一套）| 读数 + 下载 + 使用/撤销 + 更新，全走新 API（`get_runtime_catalog` / `download_runtime_file` / `get_downloaded_files` / `get_stale_files` / `get_active_preset` / `apply_active_preset` / `clear_active_preset`——两份命令清单同步注册） |

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
   6. ✅ **官方文件异常修改检测**（SHA 报警：不许应用 / 不许编辑 / 不许复制，只能重下）
   7. ✅ **官方更新与用户修改并存**（用户那份也是真 Preset：能应用；血统随文件走；
      "官方换版了、你这份基于旧版"说得明白 —— 见下）
   8. ✅ **继续编辑我自己那份**（2026-10-02）—— 见下
   9. ✅ **用户文件的合法性 / 外部修改检测**（2026-10-02，本次）—— 见下。
      **口径（作者 2026-10-02 定，别再回问）**：客户端**不复制 `mkpse-preset` 的 schema**、
      不建第二套 Preset 真相；第九层只做**文件级**（存在 + 在 `presets-mine/` 内 + 防穿越 /
      防符号链接逃逸 + 能读 + UTF-8 + TOML 语法能解析）；**语义合法性**（"是不是一份合法
      MKP Preset"：结构 / 参数）不在客户端判，留给真正的 Preset 能力在应用 / 编辑入口上回答；
      **外部修改 ≠ 报警** —— 用户那份本来就允许改，只看"现在还能不能用"、**不比 SHA**。
      已同步写进总纲 §1③。
  10. ⬜ **用户文件管理（重命名 / 删除）** —— 它已经是"我的文件"，自然会有这两件事；
      尤其自动名字 `A1-standard（已修改）.toml` 用户多半想改。
      **注意：这是真正的用户数据操作，单独设计**（第十层），不在第八层顺手做
  11. ⬜ **另存为一份新的用户 Preset**（`A1-standard（已修改）.toml` → `A1-standard-我的高速版.toml`）——
      两份独立 Preset，都基于同一个官方版本
  12. ⬜ **导入 / 分享用户 Preset** —— 它已经是一份完整 TOML，分享天然成立：
      只要"合法 TOML + 是 MKP Preset"就能进用户目录，不必知道是不是本程序产生的
   - ⬜ **「把我的修改合并到新版官方」—— 作者 2026-10-02 降级：暂不做，不进当前主线**。
     理由（原文要点记在这里）：删除 / 新增 / 字段重构 / 冲突 / 合并后生成第三份文件都很麻烦，
     而收益没那么大；它会把 Preset 系统突然变成"版本迁移工具"，不是一个自然的下一层。
     将来真要做，也更可能是**"以新版官方为基础重新修改"**（让用户自己重新调），
     而不是程序猜怎么把两个版本揉成一个 —— 程序负责可靠地保存 / 识别 / 应用用户的 Preset。
     （第七层拍板时它叫"第八层"，现在这个编号让给"继续编辑我自己那份"。）
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
     - ✅ **第六层：官方文件异常修改检测（SHA 报警）**（本次）—— 盘上有字节 ≠ 那字节是官方内容：
      同一条 `getStaleFiles`（"与目录不一致"）底下其实是两件不同的事，这一层把它们**分开说**：

      ```text
      盘上这一份 vs 目录登记的当前版本
        ├─ 逐字节相同                    → 已下载
        ├─ 与 archive/ 里某一份相同       → 旧版本（认得出它是官方哪一版；证据 = 归档里那条路径）
        │   （或被归档的旧目录登记过它）
        └─ 哪儿都对不上                   → 内容异常（这台机器上查不出它属于哪一版）
      ```

      **两个问题不许混**：「本机这份是不是我们认可的官方内容」（这一层，`get_delivery_trust`）
      与「云端有没有更新」（`check_remote_update` 比目录指纹）是两件事 ——
      以前一律写成「需更新」，用户既不知道自己的文件是不是被改过，也不知道该不该等更新。
      判定在 `runtime::delivery::trust_entries` 一处，**只列有事的**（没下载 / 与目录一致的两种不出现）。
      存疑那两档共用一条边界（不能应用 / 不能编辑 / 不能复制，只能重新下载），
      三道闸各自落在**入口**而不只在界面：应用是 `apply_active_preset` 的 SHA 校验（既有）、
      编辑是 `delivery::official_text`（本次新加：取原文先核一遍，漂了就不给）、
      复制本来就没有命令（这一层不给按钮，右键里那项带原因灰掉）。
      判据：`runtime::delivery` 四条（认得出 / 认不出 / 归档旧版 / 被归档的旧目录）+ `official_text` 一条；
      探针第 5f 节（两档各一份演示、存疑那份没有「改这份」、右键「复制」带原因灰掉）。
      **登记一条已知边界**：`archive/` 里那份的语义是"换版本时从盘上换下来的字节"，
      不是"官方签发证明"—— 若一份被外部动过的字节正好在那一刻被换下来、而归档槽还空着，
      它会进归档，此后同字节再出现就会被认成"旧版本"。要堵它得让 deliver 在归档前先判定
      （那会改变第三层"旧份留档"的行为与判据），登记在案、这一层不动它。
      - ✅ **第七层：官方更新与用户修改并存**（本次）—— 作者 2026-10-02 拍了两件产品决定，
      照做（原文与理由记在总纲 §1③）：

      ```text
      ① 用户自己保存的 TOML **可以成为"正在使用的配置"**：
         "只读"是文件归属的属性，不是"能不能被使用"的属性 ——
         两条线（官方 / 用户）都进入同一份底账 run/active-preset.json，不是两套 Preset 模型。
      ② 血统**写进 TOML 文件本身**（不建 run/preset-user-state.json）：
         "凡描述这个文件本身是什么的信息随文件走；凡描述程序现在怎么管它的才住 run/"。
      ```

      落地（三块）：

      - **血统随文件走**：另存时写工作台那套既定的**三行头注释**
        （`# based_on: mkp/presets/A1-standard.toml` / `# based_on_release_time` /
        `# based_on_sha256`），正文逐字节不动 —— 与工作台建副本（`preset::lineage`，K-O2 判据）
        **同一条形状**。客户端这一份实现在 `runtime::lineage`（默认构建不编 `mkpse-preset`，
        所以是重写），**两端靠一条一致性判据钉住**：`workbench::lineage_parity` 拿 9 份入库产物
        逐字节比 `make_copy` / 读血统 / 摘要。
        **作者例子里那个 `[metadata]` 表不能用**：`TomlConfig` 是 `deny_unknown_fields`
        （只认 `toolhead` / `wiping`），多一个 `[metadata]` 表 = 我们自己的读取面报
        "这份预设比本程序新" —— 原则（元数据随文件走）一个字不改，载体换成头注释。
      - **我那份能应用**：`ActivePreset` 长出 `origin`（`official` / `mine`，**缺省 official ⇒
        旧档不用迁移、不升 schema**）与 `path`（用户线的落点：用户目录可以自己分文件夹，
        所以认路径；**官方线仍不存路径**——落点由目录给）。落点解析收在
        `state::active_target` 一处（顺带修掉一个真 bug：原来 `active_matches_disk` 拼的是
        `mkp/<文件名>`，而真布局是 `mkp/presets/…` —— `intact` 在真机上恒 false）。
        命令仍是**一个入口** `apply_active_preset(fileName, origin?, path?)`，两道闸各按自己的线：
        官方 = 目录里有一份 + 盘上字节与目录登记的**当前版本**逐字节一致；
        用户 = 落在 `presets-mine/` 那一格里 + 盘上真有 + 是一份 TOML 预设。
      - **"官方换版了"说得明白**：用户那份的血统摘要与目录里来源那一份比 →
        `current` / `outdated`（官方换新版了）/ `unknown`（没有血统、或来源已不在目录里）。
        `outdated` **不是坏文件**：行上多一枚中性的「基于旧版官方」（不是警示色），
        展开详情「基于」那一格说全（`A1 · 快拆版6月以前 · 官方已换新版`）；
        状态条上正在用的若是我的那份，多一枚「我的文件」（它不跟着官方更新）。
        **把改动挪到新版（合并）作者已降级为"暂不做"**（见上面第 8 条之后那一段）。
      - 判据：`runtime::lineage` 7 条（副本 = 来源 + 三行 / 副本再拷替换旧血统 / CRLF /
        没有 release_time 就两行 / 往返 / 缺一半也是血统 / 形近键不误命中）+
        `workbench::lineage_parity` 3 条（两端逐字节一致）+ `mine` 三条（血统读得出 /
        官方换版 → `outdated` / 说不清那几档）+ `state` 五条（官方线按目录解析 /
        旧档 = 官方线 / 用户线能应用 / 越界的落点不解析 / 两条线互相覆盖）+
        探针第 5g 节（我那份有「应用」、点了状态条说得出「我的文件」、徽章与「基于」那格）。
   - ✅ **第八层：继续编辑我自己那份**（2026-10-02，本次）—— 用户线的最后一段闭环：

     ```text
     我的 A1-standard（已修改）.toml ──改这份──▶ run/draft-preset.json ──保存──▶ 写回它自己
     ```

     **不会产生** `（已修改）2.toml` / `（再次修改）.toml` —— 就是"编辑自己的文件 → 保存回自己"。
     落地（三块）：
     - **写用户根从此有两条路，不许合并成一个"存一下"**：`mine::commit_draft`（另存，第 5 层）
       与 `mine::save_back`（写回自己，本次）。区别只有两件事：**落点是原来那条路径**、
       **三行血统照抄文件里原来那三行**（出处没变）—— 所以血统里的摘要不会变成
       "我自己改过的字节"，那份仍然说得清自己从哪一版官方派生。
       两条共用 `lineage` 的"剪掉 → 重新插入"（`splice_lineage`），于是
       **"打开又保存、什么都没改" = 文件逐字节不变**（判据咬着）。
     - **两条线同一个入口**：`begin_preset_edit(fileName, origin?, path?)`（与
       `apply_active_preset` 同一形状 —— 官方线认文件名、用户线认路径）；
       "接着上次改"也按同一把钥匙（`DraftSubject`：两条线同名很正常，只比文件名会把
       A 的草稿接到 B 上）。用户线的前置只有两条：**是 TOML** 与**盘上真有** ——
       **它不查 SHA**：用户那份本来就是允许改的，"字节必须还是当初那一份"是官方线的规矩
       （"它现在还是不是一份能被识别的 Preset"是第九层的事）。
       编辑期间那份被移走 / 删掉了 ⇒ `save_back` **拒绝并说清**，不去别处新建一份。
     - **界面只有两句话不同**：同一颗「改这份」、同一个编辑器抽屉；官方线是
       「保存为用户文件」（另存一份新的），用户线是「保存回我这份」（写回同一个文件）。
       编辑器里给的是**正文**（三行血统是程序的元数据，不是用户该改的内容）——
       两条线打开抽屉看到的是同一副样子；保存时那三行由 `save_back` 照抄回去，
       所以用户在编辑器里删掉它们也不会把出处弄丢。
     - 判据：`runtime::mine` 写回 5 条（**同一路径不产生第二份 + 血统照抄** /
       没改就逐字节不变 / 本来没血统就不编一个 / 被移走了拒绝且不新建 / 只认那一格）+
       `runtime::state` 两条（两条线钥匙不同 / 旧档 = 官方线）+
       `runtime::lineage` 四条（写回保留出处 / 没改就字节不变 / 无血统不编 / CRLF）+
       探针第 5h 节（我那份有「改这份」、编辑器里没有血统、按钮说「保存回我这份」、
       存完没有多出一份、「基于旧版官方」那枚与那三行都还在）。
     - ✅ **第九层：用户文件的合法性 / 外部修改检测**（2026-10-02，本次）—— **架构边界由作者
      定死**（原文要点见 §5 与总纲 §1③）：**客户端不复制 `mkpse-preset` 的 schema、不建
      第二套 Preset 真相**；这一层只做**文件级**检查 —— 语义那一档（"是不是一份合法
      MKP Preset"：结构 / 参数）留给真正的 Preset 能力在应用 / 编辑入口上回答
      （默认构建不编 `mkpse-preset` 的隔离纪律不为此破例）。

      判据（作者原文四条）：

      ```text
      正常：存在 + 路径在 presets-mine/ 内 + TOML 能读   → 照常应用 / 编辑
      损坏：文件在，但 TOML 读不出来                     → 显示「文件无法读取」→ 不许应用 / 编辑
      不是 MKP Preset：TOML 能读、语义解析不过           → 客户端不判（留给真正的 Preset 能力）
      外部改过但仍是能读的 TOML                          → 照常使用，**不因 SHA 报警**
      ```

      落地（两块）：
      - **`runtime::mine` 长出文件级状态**：`MineState`（`ok` / `unreadable`，**只此两档** ——
        没有"是不是合法 Preset"那一档）+ `toml_syntax_reason`（`toml_edit` 解析，
        **报得出第几行第几列**）+ `read_preset_text`（读 + UTF-8 + TOML 语法，**应用 / 编辑
        两个入口共用这一处**，不许各写一遍）。扫盘时每个 `.toml` 候选都算出状态；
        **符号链接逃逸在扫盘就先拦**（与读正文 / 应用 / 编辑同一道 `resolve_in`；
        指向用户根外的，**一个字节都不读**，血统也不读）。非预设候选（`.json`）没有这一档
        （`state: null`）。DTO 加 `state` / `stateDetail` 两个字段。
      - **界面**：读不出来的那份**照常列在表里**、名字旁边一枚琥珀色「文件无法读取」
        （title 里带后端给的原因，如"TOML 语法不对（第 3 行第 1 列）"）；**「应用」与
        「改这份」都不给**（不给必被后端拒的按钮），展开详情里多一格「文件」。
        **「看正文」照旧给** —— 读它不算"用"，用户要能看着它去修。
      - 判据：`runtime::mine` 第九层 5 条新增（坏 TOML 画成读不出来且报行号 / 非 UTF-8
        读不出来 / **根外符号链接一个字节都不读**（根内链接照常）/ 应用与编辑入口拦下坏
        TOML 而看正文照旧 / **外部改过仍合法 = 正常（不比 SHA）**）+ 既有 2 条搭上 `state`
        断言 + 探针第 5i 节（坏的那份画得出「文件无法读取」、没有「应用」也没有「改这份」、
        角标带原因；能读的那份不受牵连）。
   - ✅ **第十层：用户文件管理（重命名 / 删除）**（2026-10-02，本次）—— 范围由作者卡死：
     **只做重命名 + 删除**，不做导入 / 另存为 / 批量 / 文件夹管理；**不给用户文件套"归档"**
     （作者原话：官方 `archive/` 是版本更新的历史；用户自己删自己的文件**就是真删除**，
     不搞第二套"用户历史管理系统"）。

     规矩（逐条）：
     - **重命名 = 只动名字**：内容 / 那三行血统 / TOML **一个字节不重写**；只换名字不换目录；
       新名字不许空 / 不许带路径分隔符 / 后缀保持原样（改名不改类别）；落点已有东西**不覆盖**
       （同一个文件除外 —— 大小写只差一档的改名要放行）。改完还是同一份 Preset。
     - **正在使用的那一份也能改名**：`run/active-preset.json` 里那条指针**跟着改名**
       （路径与文件名换成新的，**指纹原样** —— 字节没变，摘要当然不动）；
       **有草稿的也能改名**：`run/draft-preset.json` 跟着改名（用户线认路径，
       「接着上次改」不接丢）—— 这两条 repoint 在 `runtime::state`（纯函数，有判据）。
       两本状态账**先读出来**：坏档就不动文件（宁可原地不动，也不留悬空指针）。
     - **删除 = 真删除**（没有垃圾桶、没有归档）。两道硬闸在入口：**正在使用的不许删**
       （删了「使用中」就指向一份不存在的文件）、**还有没保存的草稿的不许删**
       （删了草稿就永远存不回去）。菜单里对「正在使用」的行已经灰掉带原因，后端仍会再拦一次。
     - 范围卡死：只换名字**不换目录**（跨文件夹搬动是"文件夹管理"，不在这一层）；
       官方那两份（仓库文件 / 交付预设）的改名与删除这一层都不做（菜单按 origin 灰掉、
       各说各的原因）。
     - 判据：`runtime::mine` 第十层 9 条 + `runtime::state` repoint 3 条 + 探针第 5j 节
       （见 §6；改名只动名字且状态不变 / 删除二次确认与消失 / 正在使用的不给删 /
       改名不断「已应用」、草稿跟着走）。
   - ✅ **第十一层：另存为一份新的**（2026-10-02，本次）—— 只解决一个问题：
     **我的文件 → 我的文件**（与第八层"官方 → 我的文件"分开）。作者把边界定死：
     **不做任何"智能"** —— 名字由用户明确指定（抽屉**不预填**）、目标存在就拒绝
     （不覆盖、**不自动改名**）；**一个状态都不碰**（不改使用中指针、不迁移草稿、
     不建草稿、不进 archive）；新文件从诞生起就是独立的一份（之后能独立编辑 / 改名 /
     删除 / 应用）。

     规矩（逐条）：
     - **字节复制**：内容与那三行 `# based_on*` 血统**原样带过去**，不重算血统
       （来源已经是用户文件，重算会把"从哪一版官方派生"说错）；原文件一个字节不动；
     - 名字过**同一套门槛**（与改名共用 `check_new_name`：不许空 / 不许带路径分隔符 /
       后缀保持原样）；只换名字不换目录（新的一份落在原来那一格）；落点已有东西就拒。
     - 它**不读内容、不查状态**（"纯文件操作"，与改名 / 删除同族）：连读不出来的那份
       也能复制，复制出来还是读不出来的（状态照实）。
     - 范围卡死：官方那两份的副本走「改这份」→ 保存（菜单按 origin 灰掉、说清那条路）；
       内容存疑的字节不许换个名字继续活着（第 6 层，那一句优先）。
     - 判据：`runtime::mine` 第十一层 6 条 + 探针第 5k 节（见 §6；不预填 / 字节复制 /
       血统原样 / 不覆盖不自动改名 / 不碰使用中与草稿）。
   - ✅ **第十二层：通用文件导入入口**（2026-10-02，本次）—— 作者把这一层定成
     **"外部文件如何安全进入应用"，Preset 只是第一个消费者**（原话：「第十二层负责
     '外部文件如何安全进入应用'，而不是'实现 Preset 导入'」）。以后「设置 → 备份与恢复」
     的 ZIP / 备份包复用同一套接收机制（拖拽、选择器、重名处理、边界检查），
     所以入口**不挂在预设页**：

     ```text
     App ── 通用导入入口 ──┬── 文件选择器（plugin-dialog；权限只开 dialog:allow-open）
                           └── 拖拽（真机：Tauri 原生拖拽事件给路径；浏览器：HTML5 拖拽）
                                ↓
                          runtime::import（注册表认领 → 落点检查 → 复制进 presets-mine/）
                                ↓
                          现在只注册了 Preset（.toml）；ZIP / 备份包以后往这加
     ```

     边界（逐条定死）：
     - **源文件只读**：不改、不删、不移；复制 = 读字节 → 原子写进用户根；
     - **落点固定 `presets-mine/`**，不给用户选目录；新名字过改名 / 另存为**同一套门槛**；
     - **内容按字节复制**：有血统三行原样带过去，没有**允许导入、不编造来源**；
     - **不校验 TOML 内容**：能不能当 Preset 用是后面 Preset 语义入口的事 ——
       **"导入不是安装 Preset，只是把外部文件纳入我的文件所有权范围"**（作者原话，判据）；
     - **重名不是失败，是改名流程**：`stage_import` 说 `collision` → 界面开
       「导入：有同名文件」那一格（输入框预填原名，用户改到可用名才能继续；
       **不覆盖、不自动改名**）；取消 = 这些没进来，别的照常进（一份错不拖累别人）；
     - **ZIP / 备份包现在不处理**：注册表没有认领它的导入器 ⇒ 如实说"收不了"，
       **不许被当成预设复制进用户根**；`.json` 现在也不收（只认 `.toml`）；
     - **一个状态都不碰**：不改使用中指针、不迁移 / 不创建草稿、不进 archive。
     - 工程接线：`tauri-plugin-dialog`（Cargo）+ JS 包 `@tauri-apps/plugin-dialog` +
       `.plugin(init)` + `capabilities/default.json` **只加 `dialog:allow-open`**。
     - 判据：`runtime::import` 8 条 + 探针第 5l 节（见 §6）。
   - ✅ **第十三层：文件外部管理（只做「在 Finder 中显示」）**（2026-10-02，本次）——
     作者把这一块定性为"用户文件本已经是真文件，**不再造第二套分享 / 导出**"：
     - **在 Finder 中显示**（Windows 上就是文件资源管理器）：右键打开系统文件管理器
       **并选中**这一份 —— 之后复制 / 压缩 / 发人 / 备份全随用户，**不经过 SupportEase
       的业务逻辑**（"文件外部管理"的含义就这一句）。只给「我的文件」（官方那两份住
       程序自己管的下载区 / 或还没下载 —— 菜单里灰掉带原因）；**读不出来的那份也能
       显示**（文件管理同族：打开文件夹不吃内容）；**一个状态都不碰**；成功**没有提示条**
       （文件管理器窗口本身就是回执），失败如实说。平台话术：macOS「在 Finder 中显示」/
       Windows「在文件资源管理器中显示」。
     - **导出**：作者定过定义（"把我的文件复制到用户指定的位置" —— 原文件不动、不改
       Active / 草稿 / archive、同名进改名流程不覆盖）但**决定暂缓**：Finder 已经解决
       "拿出去"；真正有产品意义的导出是以后「设置 → 备份与恢复 → 导出备份 ZIP」。
     - **"分享"整块不做**（作者裁决）：我的文件本来就是真文件，再造分享就是重复的
       复制 / 导出逻辑。
     - 工程接线：`tauri-plugin-opener`（Cargo）+ `.plugin(init)` —— **只在 Rust 侧调**
       （`app.opener().reveal_item_in_dir`），所以**不加任何 capability**。
     - 判据：`runtime::mine::reveal_target` 3 条 + 探针第 5m 节（见 §6）。
     - ✅ **第十五层：预设页数据边界修正**（2026-10-02，本次）—— 作者拿截图点名三件事，
     一个**纯前端批次**（Rust 一个字没动），把"两个 Tab 后端没有真正分流"修掉：
     - **两档按 catalog 的 `kind` 分流**（不靠扩展名猜；`catalogKindToFileKind` 一处映射）：
       `mkp_preset` → MKP 配置；`bbs_config` / `orca_config` → 切片器配置；
       **图标（`icon`）/ 模型（`model`）哪一档都不出现** —— 登记在目录里 ≠ 进预设页。
       修的是 release 行那条**没写下来的前提**（"catalog 登记的只有预设"）：作者截图里
       `a1.svg` 与 `MKPProcess ….json` 全混在「MKP 配置 → 云端」表里（`ReleasePresetSource`
       带上 `kind`，两档的过滤跟着数据走）。
     - **台账「仓库 N」跟档**：`PresetTree.fileCounts` 把两支来源（`getVersionFiles` 的 MKP +
       `getPresetFiles` 的切片器）合起来按 path 去重、按 kind 计数 —— 旧 `totalFiles` 只数 repo
       一支（MKP 档下显示的是切片器的数）。「我的 N」也跟档（判据与本地表 mine 行同一条）；
       「本机 N」仍是官方副本总数（`getLocalFiles` 老契约，真机还没接）。「共 N 项」本来就对
       （表 bug 修完它自己就对了 —— MKP 档 3 项 / 切片器档 2 项）。
     - **下载错误展开**：`下载失败：[object Object]` 的病根是各页各写各的
       `e instanceof Error ? e.message : String(e)`，而跨 IPC 的错误是 `AppError`
       （普通对象、不是 `Error` 实例）—— 统一成 `api.errorText()`（`AppError` → `message`；
       19 处，含参数页 / 导入那一侧）。**后端错误结构一个字没动**。
     - **顺手修一个"点了没反应"的按钮**：切片器那一类的**交付行**（catalog 登记、能下载）
       在本地表里原来画「复制」——`runLive` 里 `assetId` 是 undefined 直接 return、点了静静
       没反应；现在给灰字 + 说明（`SLICER_RELEASE_WHY`）；云端那一行的「下载 / 更新」照旧真能下。
     - **不做（登记为"以后"）**：切片器交付行的**喷嘴 / 层高**（catalog.files 现在不带这两个
       字段，表里如实「—」——要显示得先给交付行补字段，另案）；`PresetFileRow.tsx` 是**死组件**
       （零引用，清扫遗留，另案删）。
     - 判据：探针新增第 2b 节（分类边界：MKP 档不许有 `.svg` / `.json`、切片器档要有 catalog
       交付行且动作是「下载」、台账两档不同数）+ mock 补 `bbs_config` / `icon` 各一条
       （真机 catalog 里长这样，浏览器里也能验）；`PRESET-PRODUCT-RULES.md` §1 / §2 / §4
       跟着补三笔（核准记录里记了）。
     - ✅ **第十六刀：「同步」页退役 + 最小设置页**（2026-10-02，本次）—— 作者裁决：
     "普通用户完全不需要'同步'这个概念；数据源配置降级成设置页里的开发后门。"
     同步页（数据源 / catalog 调试账 / 下载第一份 / 检查更新）是开发验证面板，不是产品页面：
     - **删**：导航「同步」+ `src/app/pages/`（`PagePackage.tsx` / 它的 css / 一个零引用的
       `pages.module.css` 死文件 —— 目录随页面清空）；参数页空态那颗「去「同步」页获取一份」
       跳转（没去处了）；工作台 `BuildPage` / `cloud.ts` / `clientPackage.ts` 与前端
       `contract` / `mock` / `storageKeys` 里"客户端「同步」页读的就是这一格"那套**失效指针**
       （C4 起客户端就不读 `STORAGE.cloud`，那一格只归工作台）。
     - **搬**：数据源那一格 → **设置页 · 高级设置 · 预设数据源**（内置官方源 / 手动指定
       （开发 / 排查）/ 恢复内置默认）。写动作全走按钮（不做"一选就写"的隐式动作）；
       「空地址不写盘」的语义不变。
     - **新增一条命令** `clear_preset_source`：「回到内置默认」只能靠删设置文件
       （`save_source` 拒空 = 不许制造第三种状态）—— 原来只有"用户手删文件"这个出口，
       现在是一次显式动作（幂等）。`PresetSourceDto` 加 `builtin` 一格：有覆盖时 `baseUrl`
       是覆盖值，界面得说得出"撤掉覆盖会回到什么"。
     - **保留（一个字没动）**：`check_remote_update` / `apply_remote_update` /
       `run/preset-source.json` / 下载管道 / catalog —— 检查更新现在**暂时没有 UI 入口**，
       Bootstrap 那一刀「预设页进入时后台检查一次」接上。
     - `remote_base` 的"去哪儿填"改指设置页；mock 的数据源改**内存镜像**（校验消息与真机
       同一套）。
     - 判据：`params-sync.mjs` → **`params-settings.mjs`**（参数页那半保留 + 设置页数据源
       全流程：当前状态如实 / 手动指定应用（尾斜杠砍掉，与真机 `normalize` 同一套）/
       非法地址如实拒 / 恢复内置默认能撤回），两档 **14 条全绿**；`tabs.mjs` **7 页签**
       （设置成真页面）；`chain.mjs` 第 ③ 步改点设置页，两档 **10/10 绿**；Rust
       `source::clear_source` 2 条新测试（撤覆盖 / 幂等）。
     - ✅ **第十七刀：Bootstrap 官方源接通**（2026-10-02，本次）—— 作者裁决：
     "整个'同步系统'从一个用户功能，变成内部基础设施。"链路定死：
     `工作台填 Bootstrap 地址 → 构建期注入 → 客户端读 source.json 拿 catalog + 文件根 →
     进预设后台检查一次 → 有新版才换本地 catalog → 用户点"下载"才下载文件`。
     - **两个入口，语义故意不同**（`runtime::source` 模块头）：
       **手动覆盖**（设置页，`run/preset-source.json`）= **数据源根**（根下直接是
       catalog.json，开发 / 排查通道，不联网）；**内置**（构建期注入）= **Bootstrap 地址**
       （指向 source.json，正式通道）——**覆盖优先**，两条路解析成**同一形状**
       `ResolvedSource`（catalog_url + base_url），下游只认它、不各自拼 URL。
     - **注入**（`src-tauri/build.rs`）：`MKPSE_PRESET_SOURCE` 环境变量 > 工作台配置
       `workbench/bootstrap.json`（`wb_set_bootstrap` 写，**入库**）> 都没有 = 没配。
       `rerun-if-changed` 登记它；**改了要重新构建**（dev 重启 `tauri dev`）。
     - **接 GitHub**：`dist::normalize_bootstrap_url` —— **blob 页 → raw 直链**
       （`github.com/o/r/blob/ref/path` → `raw.githubusercontent.com/o/r/ref/path`）；
       已经是 raw / 别的 http(s) 原样过（自建源合法）；github 但非 blob 页（仓库 / 目录页）
       如实拒并说"那是个目录"。工作台「设置」页**只有这一格可编辑**（其余数据根只读）。
     - **发布产物**（`dist::bootstrap_json` + `SOURCE_FILE`）：`source.json` 就两件事
       （`sourceSchema` + `catalog`），**不写 baseUrl**（客户端缺省理解成"同目录"——
       同一份 dist 推到哪里都对）；它进交付集合、收尾随 catalog / manifest 一起原子写。
       `NEW_CATALOG_FILE` 直接引用 `runtime::source::CATALOG_FILE`（同一份名字不再写两处）。
     - **客户端解析**（`parse_bootstrap`，纯函数、可单测）：代次认不得 / catalog 绝对 URL /
       `..` / baseUrl 非 http(s) 一律拒；`directory_of` 从 `source.json` 的 URL 回退目录。
     - **进入预设后台检查一次**（本次补的最后一段，`usePresetData`）：
       **先画本地 catalog，再在后台 `checkRemoteUpdate`**（不挡首屏）——
       模块级 `checkedBootstrapThisRun` 保证**本次运行只一次**（切 tab 来回不重发，
       关掉 App 再开才重置）；**有新版才 `applyRemoteUpdate`**（换本地 catalog）**再重读那一路**；
       **绝不自动下载任何预设文件**；**失败静默**（没内置源 / 离线 / 远端没部署都是开发期
       正常状态，不许让预设页报错或弹条子）—— 启动仍**零网络**。
     - 判据：Rust `source::parse_bootstrap` / `directory_of` / `dist::normalize_bootstrap_url`
       / `bootstrap_json` 一组新测试（含"发布侧写的那份客户端解析器读得动"两端互钉）；
       探针 `presets.mjs` 新增第 5n 节（进入预设不挡首屏 / 失败静默 / 不自动下载）。
       全绿：`cargo test` 253、`--features workbench --lib` 441、双 feature clippy 干净、
       探针通过、`check:bundle` / `check:zero-network` 干净。
     - **保留（第十七刀一个字没动）**：设置页「手动指定数据源」继续是开发 / 排查覆盖。
     - ⏳ **第十八刀：官方预设真实交付链**（进行中的后一半）—— 前一半已落地：
     - **发布动作定案**（2026-10-02 作者裁决）：`presets/dist/` 是 **main 上的正式交付目录**
       （不再是"本地构建产物、不入库"）。客户端 Bootstrap = 
       `raw.githubusercontent.com/.../main/presets/dist/source.json`。
     - **`.gitignore` 两处改判**：撤掉 `presets/dist/` 那条忽略；**并把裸 `dist` 锚定成 `/dist`**
       —— 裸 `dist` 会匹配任意层级，把 `presets/dist/` 也一起吞了（撤第 6 行不起作用，
       真正拦它的是第 3 行那个裸 `dist`）。根 `dist/` 是 Vite 输出，仍忽略。
     - **`scripts/publish-presets.mjs` + `npm run publish:presets`**：发布动作 = **一次性分支
       → PR**，**不碰 main 直推闸门**（闸②无条件拦，ruleset 再拦）。**刻意不复用
       `release.mjs`**（那条是软件版本发布：版本号 / tag / 完整 CI 链；预设数据是另一条线）。
     - 发布前五道校验（不过就一个字节都不写）：①产物存在 ②`source.json` 认得
       ③`catalog.json` 每条 `path` 真存在 ④**SHA / 大小对真字节** ⑤manifest 与 catalog
       交叉核对（防半成品）。这是 `publish_into` 收尾那段的**进库前副本** —— 两侧都错才可能漏。
     - 两个真实 bug 在写的时候踩到并修掉（值得记）：**① `run()` 会 `.trim()`**，
       而 porcelain 是 `XY<space>path`（` M path` 首字符是空格）—— trim 后再按偏移切会把
       `presets/…` 切成 `resets/…`，过滤静默失效（加 `runRaw` 不 trim 的读法）；
       **② 工作区检查必须排除 `presets/dist/`** —— 首次发布时它本就是未跟踪的，
       拿它自己的状态拦自己 = 死锁；且要用 `--untracked-files=all`（默认只报顶层目录）。
     - 验证：在 `/tmp` 隔离仓库造完整产物，跑通全绿路径（校验 → 分支 → 提交 → push）
       与四条拒绝路径（缺 source.json / path 不存在 / SHA 失配 / manifest 脱节），
       真仓库未被污染。
     - **下一步（第十八刀后半，待作者在工作台点一次「发布」）**：
       `presets/dist/` 现在只有 `mkp/presets/*.toml`（`wb_generate` 产物），**没有
       `source.json` / `catalog.json` / `manifest.json`**（`wb_publish` 才写这三个）。
       所以要先在工作台 **生成 → 发布**，再 `npm run publish:presets`，合并 PR，
       最后跑真实 raw → catalog → 单文件下载闭环。
     - ✅ **第十八刀后半：真实交付闭环已验通**（2026-10-02）
     - 作者在工作台点「生成 → 发布」后，`presets/dist/` 齐 22 个文件 →
       `npm run publish:presets` 走五道校验 → PR #15 → squash `869dd5f` 进 main。
       真实 raw 已验：`raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/dist/`
       的 `source.json` → `catalog.json`（revision `620bb442d24db768`）→
       `mkp/presets/A1-standard.toml`，**SHA + size 与 catalog 完全一致**（客户端校验会过）。
     - **脚本判据修正（重要）**：第一版要求 `catalog.json` **每条 path** 盘上存在 → **误拦**。
       真相：catalog 是整个目录（24 条，含可达性收窄掉的 3 个 3mf + 4 个 0.2mm 切片器），
       **进交付的只有被引用可达那批**（manifest 17 条 / 交付 17 个文件）。
       "必须存在" + SHA 校验**只对 manifest** 做。
     - **CI 抓到的真 bug**：`build.rs` 用 `serde_json` 但 `[build-dependencies]` 漏了它 ——
       本地靠主 crate 依赖缓存蒙混，CI 干净环境 `error[E0433]`（修于 `2e4e791`）。
       教训：**`src-tauri/Cargo.toml` 里的 build-dependency 属于功能本体，不是"在途依赖版本"**。
     - ⏳ **剩最后一步**：写 `workbench/bootstrap.json`（**入库**）——
       这是"官方源指向哪"的产品配置，属作者裁决范围。写完 `tauri dev`/正式构建即自动注入。
     - ✅ **第十九刀：Bootstrap 输入契约 + 工作台去隐式 mock**（2026-10-02）
     - **输入契约**（PR #16 = `06295d8`）：**只填仓库地址就够** ——
       `github.com/o/r` / `.git` 克隆地址 → 自动补 `main/presets/dist/source.json` 的 raw；
       blob 页按人指的转；raw / 自建源原样；`tree/…` 目录页 / 空 / 非 http(s) 拒。
       `.git` 只是克隆地址的写法，不是产品语义。改在 `dist::normalize_bootstrap_url`（3 组单测）。
     - **桩不再隐式装**（PR #17 = `6148afe`）：`main.tsx` 默认**不装**桩，只有 URL 带
       `?mock=1` 才装 —— 桩是**探针的测试后端**，不是产品运行时能力。
       `tauri:workbench:dev` 现在**永远走真 Tauri IPC**。
     - **设置页加「重新读取」**：调 `wb.reload()` 从磁盘重读 `workbench/bootstrap.json`
       并回填界面 —— 界面之外改过它时用它看真相。placeholder 改中性示例。
     - 验证：`chain.mjs` **17/17 全绿**；`cargo test` 253 / workbench lib 442 / 双 feature clippy /
       fmt / tsc / lint / build + build:workbench + check:bundle + check:zero-network 全绿。
     - **未碰** `src/api/mock.ts`（客户端另一套 mock，不扩大范围）。
     - **顺带发现（未修）**：`scripts/probes/workbench-build.mjs:225` 引用一个**不存在的
       「钉住」按钮**，跑到【二】参数台必挂 —— C15→A40 移植遗留的预存问题，与这两刀无关。
     - ✅ **第二十刀：offset 内联表拆成三个独立字段**（PR #20 = `7a10f8b`，2026-10-02）
     - **作者裁决**：趁参数页底层重做，把 TOML 结构一次定清楚 ——「**一个参数一个明确字段**」，
       不再为兼容旧文件保留特殊结构。`[toolhead] offset = { x, y, z }` → `offset_x/y/z`。
     - 改动面（46 文件）：`crates/preset`（**删 `ToolheadOffset` 结构体**；
       `EDITABLE_KEYS` 77→79 键；`build`/`validate`/`write`）/ 注册表三条 `tomlKey` 各自唯一
       （不再共享 `offset`）/ 37 处产物+夹具+配方（**三行都带 `tomlComment`**）/
       `catalog.generated.json` 重新生成（revision `e373b773c82c0daa`）/ workbench 渲染基线。
     - **刻意不兼容旧格式**（作者："不兼容，不需要管"）：删两条依赖**仓库外**旧文件的判据 ——
       `load_ir.rs::the_real_user_preset_loads`、`ranges.rs` 里扫 `~/Documents/MKPSupportSSR` 那段。
     - **关键**：IR 层（`postprocess/src/ir/types.rs`）本来就是三个独立字段 → 改动收敛在
       「TOML ↔ preset crate ↔ 注册表」一圈，**不碰后处理管线**。
     - **踩坑记录**：改完字段名后 `embedded_matches_rebuild` 红 → 必须重跑
       `cargo run --bin gen-catalog`；`our_render_matches_the_machine_verified_baseline` 红 →
       产物三行都要带 `tomlComment`（渲染器按注册表给 y/z 也带）。
     - **遗留（未清）**：`jsonKey`/`mergeGroup`/`subfieldsOrder` 这套「共享 tomlKey = 内联表」
       机制已**无人使用**（拆完即死代码），留待清理刀。
     - ✅ **参数页底座第一阶段 ① `RecipeParamDto` 下发 `toml_key`**（PR #21 = `8d82373`）
     - 施工前核清三事实：DTO 定义 `ipc/presets.rs:594` / `machine_params_dto`；
       `toml_key` 来自 `ParamDef.toml_key`；前端 `ParamMeta.tomlKey` **早已存在** ——
       所以①不是"从零加字段"，而是让**同一条数据走两条通道**（元信息 + 配方值）。
     - 改动：`RecipeParamDto.toml_key` + TS `RecipeParam.tomlKey` + 假后端 `resolveParams`；
       测试钉住"配方通道与元信息通道的 `toml_key` 必须是**同一个值**（不是两份真相）"。
     - **刻意不激活** `jsonKey`/`mergeGroup`/`subfieldsOrder`（已是死代码）。
     - ⏳ **下一步 = ② `patch_preset_toml`**：按 `toml_key` 用 `toml_edit` 改草稿 TOML 的某个字段
       （保注释/顺序/格式，**不重新生成整份**）。签名 `patch_preset_toml(text, param_key, value)` ——
       收**注册表主键**（如 `toolhead.offset.x`），不收裸 toml 键、不收 section（定位由注册表派生）。
     - ✅ **② 已落地**（分支 `feat/parameter-draft-foundation`，2026-10-02）：
       新增 `src-tauri/src/presetdata/patch.rs` ——
       `patch_preset_toml(raw, &reg, &FieldEdit{param_key, value}) -> 新正文`。
     - **住在 `presetdata` 而不是 `crates/preset`**：后者（`mkpse-preset`）是 workbench-only 的
       optional 依赖，而参数页是**客户端默认构建**就要用 —— 拉进来会破坏隔离边界。
       `toml_edit` 在 `presetdata` 本就可用（它读注册表就靠它）。
     - **定位靠注册表**：`param_key` → `ParamRegistry::param()` 拿 `(section, toml_key)`。
       不拆 `param_key` 去猜（那会把 `offset_x` 猜回 `offset.x`，正是 PR #20 拆掉的东西）。
     - **形态由 `valueType` 定**（float/int/bool/text），**不由"有没有 choices"定**
       （`prime_enabled` 选项写 off/on 但 TOML 里是布尔）；多行自动渲染 `"""…"""`。
     - **只改已存在的键**：段/字段缺了各自如实拒（不凭空造），**保注释/键序/格式**
       （decor 从旧值搬到新值上）。7 条单测拿**真注册表**跑。
     - **下一步 = ③ 参数页接草稿链**：打开 → `begin_preset_edit`；改值 → `patch_preset_toml`；
       保存 → `commit_preset_draft`。之后 ④ 操作记录底座（先不做历史 UI）。
     - ✅ **③ 已落地**（分支 `feat/parameter-draft-foundation`，2026-10-02）：参数页接上了草稿链。
     - **产品语义（作者定，解耦）**：**当前应用**（`active-preset.json`）只决定"默认打开哪一份"；
       **当前编辑**是参数页自己的状态。"用户切机型/版本 = 换编辑目标，**不等于应用它**"——
       改 P1S 不动 `active-preset`。两概念在 `useParams` 里各是一个变量，不互相写。
     - **后端**：新增 IPC `patch_preset_draft(paramKey, value)`（`ipc/mine.rs`）——
       读 `run/draft-preset.json` → `presetdata::patch::patch_preset_toml` → 写回草稿正文。
       `patch_preset_toml` 改成收 `&[ParamDef]`（不再收 `&ParamRegistry`）：字段定义有两个来源
       （工作台 `ParamRegistry` / 客户端 catalog 的 `CatalogRegistry`），算法只认那张表。
     - **前端**（`useParams`）：`editingPreset`（编辑目标，从 `catalog.editTargetByCombo` 查）+
       `patchDraft`（`apply`/`undo`/`redo` 每次改动都落草稿）+ `commitDraft`（`save` 时提交）。
       `editTargetByCombo` 官方线来自 catalog 的 files，用户线来自 `getUserPresetFiles`（血统反推机型/版本）。
     - **页面**：草稿没写进磁盘 / 这个 combo 没配 MKP 时，各显示一行红字（`PageParams.module.css` 的 `.warn`）。
     - **修了一个 mock 夹具漂移**：`param_registry.json` 快照 + `MOCK_OFFICIAL_TEXT` 还停在
       `offset = { x, y, z }` 的旧形状（PR #20 之后没同步）→ 参数页改 offset 会报
       "草稿的 [toolhead] 里没有 offset"。已同步成 `offset_x/y/z`（探针 ①b 节逮到的）。
     - 验证：`cargo test` 260 · workbench lib 449 · 双 feature clippy · fmt · tsc · lint ·
       build + check:bundle + check:zero-network · 探针 `params-settings.mjs` **18 条全绿**
       （含新增 ①b 节）/ `presets.mjs` 全绿。
     - ✅ **④ 操作记录底座已落地**（同一分支，2026-10-02）：
       `HistoryItem` 现在**成形那一刻就快照上下文**（`label` / `tab` / `section` / `unit`），
       渲染时**不再回查当前 combo 的字段定义** —— 回查有两个毛病：切机型/版本后那个 key
       可能不存在（只剩裸 key，正是"历史没上下文"的根因）、目录更新会改写"过去的事实"。
       历史一条现在读作：`偏移 / 空间偏移 · X 轴偏移` + `-1mm → 1.23mm · 改 X 轴偏移`。
       `HistoryDrawer` 的 `defOf` 参数删掉（不再需要）。
       **本轮只做底座，不做历史页面**（作者：历史 UI 是第二阶段独立原型）。
       验证：`params-settings.mjs` 新增 ①c 节（历史条目带分类/参数名）→ **20 条全绿**。
     - ✅ **参数页底座第一阶段（①–④）全部完成** —— 整刀待统一推送 / PR / CI（新流程）。
     - **参数页底座第一阶段全貌（作者定，施工中）**：
       ①`RecipeParamDto` 下发 `toml_key`（**PR #21**）②客户端 patch 能力（**`62c4d0d`**）
       ③参数页接草稿链（**`6bc417d`**）④**操作记录底座**（**`d259c33`**）
       —— **①–④ 全部完成并合并**（**PR #22 = `2da318c`**，CI 三档全绿）。
       ① 因流程变更前已单独进 main；②③④ 在一个分支上连续施工、一次 PR（新流程验证成功）。
       **参数编辑 = 编辑一份 TOML**，不是"改参数对象再想办法重新生成 TOML"。
       **B（保存=另存）与 C（草稿跨页/重启）本就是同一件事**（都在"持久化 TOML 草稿"里）。

     ### 双仓协作（2026-10-02 起，新增）

     **第二个仓库**：`/Users/wzy/projects/mkp-adaptive-console`（独立 GitHub），
     是**前端稿号试验台**（A32–A43 客户端 / C01–C15 工作台 / G 轨抽卡），后端是假的。
     A43 的 README 自己写着"**要搬去 MKPSupportEase 的就是这一半**"。
     **对比报告见 `docs/TWO-REPO-ALIGNMENT.md`**（逐契约 / 逐页面 / 逐业务，接手前先读）。

     - **角色分工**：B 继续迭代**前端 UI**（参数树交互 / 动效 / 布局），A 是**真产品**（后端 + 部分前端）。
     - **唯一对齐面 = `src/api/contract.ts`**。契约一致 → 各自推进；契约不一致 → **冲突，找作者裁决**。
     - **B 领先、值得搬的**：受控参数树（折叠统一 A43 / 装订子卡 / 条件小签 / 写值闸 / 弃用标记）、
       交接动效（手风琴头）、`inputsHash` / `minClientVersion` 版本判定（比 A 的 revision 细）。
     - **A 领先的**：用户文件两条线 + 血统、草稿链、交付信任 / SHA、通用导入、Bootstrap、真发布。
     - **契约缺口（A 要补）**：`ClientFieldDef.deprecated`（字段级 + 选项级）、`MachineDimensions.plate`。
     - **模型冲突（要作者裁决，倾向用 A 的）**：B 的 `LocalUserFile` / `AppliedPreset` 是假模型，
       搬预设页时**用 A 的 `UserPresetFile` / `ActivePreset`**。
     - **两边一起删**：「同步」页（A 已退役，B 后续也删）。
     - **注定分叉**：B 的稿号体系是**工作方式**，搬的是"挑中的那一稿**前端**"，不是整个稿号体系。
     - **移植账见 `docs/A43-PARAMS-PORT-LEDGER.md`**（2026-10-02，逐项"A43 需要什么 → A 真实来源"）。
       口径（作者定）：**搬的是 A43 的前端实现经验，不是它的数据假设**。
       - **A 的参数页本来就与 A43 同源**（同名同构），A43 是继续演进的版本 → 是**增量移植**。
       - **`useParams` 不搬**（A 那份领先：草稿链 / 编辑目标 / 操作记录是 A43 没有的）；
         搬 `ParamCard`(206→416) / `PageParams` / `ParamRow` / `GcodeBlock` + A43 独有 3 文件
         （`TowerMap` / `TowerCoreSvg` / `bedOutline`）。
       - **`deprecated` 上游早就真了**（真 TOML 8 处：7 字段级 + 1 选项级「护套」），
         断在下游：`catalog.generated.json` 需重生成 / IPC `RecipeParamDto` 无 / IPC `ChoiceDto` 无 /
         契约无 / `src/app/` 零消费。**这不是为 A43 硬造字段**（真数据里本来就有）。
       - **塔可以直接接**：A43 塔现算用的 5 个 key（`wiping.outer_structure` / `rib_width` /
         `rib_extra_length` / `rib_fillet_wall` / `sheath_base_expand`）**在 A 真注册表里全有** ✅。
       - **模型/3MF**：A = "有资产（3 个真 .3mf）/ 契约弱（`FileKind` 不含 model）/ UI 全缺"，
         **单独一刀**，不混进参数页 UI。
       - **施工顺序**：① `deprecated` 链路 → ② 参数页 UI → ③ Plate → ④ 塔地图 → ⑤ 模型/3MF。
       - **四件事全部拍完**（2026-10-02）：①范围（①②③ + 塔地图一并做）②Plate 引用=**数组+默认值**
         ③板数据放 `presets/plates/*.toml`④字段级 `deprecated` = **显示但只读**（下面的新铁律）。
       - **①–④ 已在 `feat/a43-params-port` 上一轮做完**（见 §3.5「A43 参数页移植收口」）。

     #### ★ 新铁律：`deprecated` = **显示 ≠ 可编辑 ≠ 会进入新产物**（作者 2026-10-02）

     作者裁决：`deprecated` 表示该字段**已退出正常编辑 / 产物生成，但仍属于已知参数**，
     所以参数页**继续展示它的历史状态**。理由原话要点：「不是单纯把旧参数清理掉，
     而是**有后处理功能**」—— 直接从参数页消失反而会让用户不知道它去哪了。

     ```text
     deprecated 参数
       ├── 参数定义：存在
       ├── 参数页：显示（行名红线 + 「已弃用」徽章）
       ├── 用户编辑：禁止（控件只读 + 写值闸原子拒绝 + 一句人话）
       └── 新 TOML：不生成
     ```

     **实现约束（不许偷改既有语义）**：`getMachineParams`（`visible_keys_of` / `effective_of`）
     **照旧排除**弃用字段、不下发其值 —— 不动；参数页的**字段清单改从 definition 通道取**
     （`catalog.registry.params` → `useParams` 的 `registryParams`），配方通道只补值，
     值读不到就空、**不伪造**。选项级弃用映射到既有共用件 `FieldOption.deprecated`（划线），
     **不新造第二个信号**。

     ### A43 参数页移植收口（2026-10-02，分支 `feat/a43-params-port`）

     **口径**：搬的是 A43 的**前端实现经验**，不是它的数据假设；数据一律接 A 的真实来源。
     一整刀连续施工，**①②③④ 一轮做完**（原计划里塔地图排在后面，作者本轮点名一并做）。

     | 段 | 落地 |
     | --- | --- |
     | ① `deprecated` | 后端双 DTO（`RecipeParamDto` / `ChoiceDto`）带标记；**字段清单改从 definition 取**；`ParamRow` 红线 + 「已弃用」徽章 + 只读；写值闸在 `apply`（唯一出口）原子拒绝；判据 = `ipc::presets::tests::deprecated_flags_travel_through_definition_channel_only` |
     | ② 参数页 UI | `ParamCard` **重写为受控参数树**（`buildTree` / 装订子卡 / 条件小签 / 折叠统一=条件满不满足 / 手风琴头 240ms）；`ParamRow` 加 `sep`+`data-off`+`data-dep`；`GcodeBlock` 三层代码编辑器 + 新 `gcode.ts`；`PageParams` 接 `condOn`/`tower`；**`useParams` 不搬**（只加 `condOn`/`plateOf`/`gateNote`/`deprecated`/`registryParams`） |
     | ③ Plate | 新增 `presets/plates/*.toml`（**2 块去重**：单卡舌 256 / 双卡舌 180）；五份机器加 `plateIds`+`defaultPlateId`；Rust `Plate`/`PlateFrame`/`load_plates`/`Catalog.plates()`/`check_plate_refs`；runtime catalog 带 `plates`；契约 + mock 镜像 |
     | ④ 塔地图 | 新增 `TowerMap.tsx` + `TowerCoreSvg.tsx`（逐字移植）；按 `defaultPlateId` 选板、参数值现算；**不留 `?? machineId` 兜底**（无板 = 画布退回圆角矩形） |

     **验收**：Rust 263 + workbench 452 全绿；双 feature clippy `-D warnings`；前端 build / bundle /
     zero-network / tsc / lint 全绿；探针 `params-settings.mjs` **22 条判定**（新增弃用 / 塔地图两条），
     两档尺寸 0 console error。截图 `tmp-shots/a43-tower.png` / `a43-dep.png`。
     **验收记录见 `docs/A43-PARAMS-PORT-LEDGER.md` §9。**

     ### 当前阶段：参数页底座 —— 还剩两块（**做完这两块再统一整理**）

     作者 2026-10-02 定：**这一阶段做完才做整理**（死文件 / 硬编码 / 复制未接 / 文档收敛
     那些一起到整理那一刀再动，别混进阶段性施工）。

     1. ⏳ **修改历史 UI**（第二阶段，作者原话："**独立 HTML 原型**"）——
        脱离参数页那个小空间，专门把**参数分类 / 修改上下文 / 批次 / 撤销 / 恢复 / 保存**
        这些交互做清楚。底座（④）已经供数：`HistoryItem` 自带 `label`/`tab`/`section`/`unit`。
        现在那个 `HistoryDrawer` 是**旧交互模型**（点"还原"是整份快照回滚，不是"撤掉这一件"），
        作者认为它**不适合参数台**；新原型做完再决定怎么替换。
     2. ⏳ **设置 → 备份与恢复**（ZIP）—— 复用第十二层的通用文件导入入口
        （注册表已留认领口子：现在只认 `.toml`，ZIP / 备份包往里加一个导入器）。
        **这是 PROJECT-AUDIT 建议次序里的第一个真块**（排 ①）。

     ### 生成前确认（2026-10-02，工作台「生成与发布」页）

     **起因**：作者在客户端预设页看到「下载失败：`A1-fastv3.3.toml` 的响应比目录登记的大
     （目录记 4300 字节，远端已经给了 4336 字节以上）」，转到工作台问「生成的时候如果原本已经
     存在了呢，我不希望他直接就这样子点了生成就生成」。

     **查出那条报错的真因（长期有效）**：`#20`（`7a10f8b`，offset 内联表拆成 `offset_x/y/z`）
     改了 `presets/dist/mkp/presets/` 里 9 份交付 TOML（每份 +36 字节），**但没有重跑
     `npm run publish:presets`** —— 于是 `presets/dist/catalog.json` / `manifest.json` 里的
     `size`/`sha256` 还是 `#15` 的旧值。实测 9/9 全部对不上；`git status presets/dist/` 干净
     ⇒ **main 上就是这样，客户端字节校验（判据 2）正确工作，发布侧漏了一步**。
     → **修法**：发布侧重跑 `npm run publish:presets`（重建 SHA/size → 一次性分支 → PR）。

     **本轮落地（工作台侧）**：`wb_generate` 是**直接原子覆盖**的，中间插一道确认：
     ```
     点「生成」→ wb_generate_preview（只算不写）→ 模态框（清单 + 行级 diff）
               → 确认生成 → wb_generate 真写 → 框内换结果页 → 完成
     ```
     - **后端**：新增 `wb_generate_preview(scope)`（`build.rs`），与 `wb_generate` **共用
       `planned_todos`**（同一批 + 同一套跳过理由）+ 同一道 `issues::inspect` 闸；
       新增 `DiffState`（added/modified/unchanged）+ `PreviewFile/PreviewReport`；
       手写最简**行级 LCS diff**（`diff_lines`，剥前缀/后缀 + 中间段 LCS；**不引第三方 diff 库**）；
       `preview_one` 判：磁盘读不到 → added（全绿）/ 逐字节相同 → unchanged / 否则 modified。
       `wb_generate` 本身**一行没改**（原子性 + `same_payload` 保留）。
     - **前端**：新增 `views/GenerateDiffModal.tsx` + `.module.css`（**「清单 + 详情」两栏**，
       不是把 N 份竖着堆 —— 作者点名）；有变化的排前、无变化折成「N 份无变化」可展开；
       行内未变段折成「… N 行未变 …」可点开；确认后框不关、换成结果页（读 `wb_generate` 的
       `written`/`unchanged`/`skipped`）。`BuildPage` 的生成按钮改走 `openGenerate → confirmGenerate`。
     - **演示桩**：`dev/mockBackend.ts` 加 `wb_generate_preview`（按 `builtRecords` 造同形报告）。
     - **判据**：Rust 7 条（含 **`preview_never_touches_the_disk`**）+ 探针
       `workbench-build.mjs` 的生成前确认四断言（点生成先弹框 / 确认前不写盘 / 确认后换结果页 / 名单才跟上）。
     - **口径**：预演**一个字节都不写**（`DiffLines` 拿真文本比，不用 `same_payload` 的"跳过
       `release_time`"等价 —— 那是判"要不要重写"的口径，不是给人看 diff 的口径）。
     - **增量（同日晚，作者看着确认框点名两处）**：
       · **「完整 / 对比」切换**：标题行右侧多一个两段切换（`Modal` 新增 `headerExtra`
         工具位 —— 各处框都能往标题行空白里放小切换了），**默认「完整」** = 选中那份的
         **新文件全文**（丢弃 `removed`、按新行号排回，无红绿底）；切「对比」才是原来的
         行级 diff。作者原话「就看完整的不看对比的」。
       · **行号口径**：后端 `removed` 行记**旧文件**行号、其余记**新文件**行号，以前混排在
         同一列（39、40 然后跳 41、42，作者看不懂）—— 现在 `removed` 行号槽留空
         （「−」符已说明它是删的），一列只剩「新文件第几行」一套语义。**后端没动**。
       · **模态框滚动穿透**：滚到框内滚动容器的头、或鼠标落在遮罩空白处，会滚到背后页面
         —— `Modal` 的遮罩（补 `overflow: hidden` 让它成为滚动链最后一环；代价：框贴到
         遮罩边时投影被裁一点）与 `.body`、确认框的 `.list`/`.detail` 全部
         `overscroll-behavior: contain`。修在 `Modal` 一处，二十几处框都受益。
       · 探针新增三断言：详情默认「完整」/ 切「对比」见行级 diff / 切回「完整」见全文。
    - **增量之二（同日再晚，作者追着框点名「还是滚动不了」）**：
      · **滚不动的真因在 `.panes` 自己**：它是 grid + `max-height` —— grid 的 auto 行在
        max-height 约束下**仍按内容高算**，超高部分被 `overflow: hidden` 裁掉、栏内
        永远不出滚动条（折叠视图内容矮没触发；完整视图一展开必现）。**改成 flex**：
        容器 max-height 约束下栏被压进容器高，栏自己出滚动条。实测（playwright 注
        120 行）：`detail scroll=2578 / client=410`，滚轮后 `scrollTop 0→400→2168` 到底。
      · **footer 按钮统一**：`GenerateDiffModal` 自养的那套圆角按钮是全工作台唯一的
        例外（其他框全用 `c14.module.css` 的矩形标准按钮）—— 弃掉自养套件，改用
        `c.btn` / `c.btnPrimary` / `c.grow`。作者：「右下角的按钮都长得一样、位置
        一样，是矩形」。**工作台的统一模态框 = `ModalC14`（外壳 `.shellBody` 宿主 +
        `components/modal`），这一轮没有第二套，标题行还多了 `headerExtra` 工具位。**
      · **「未生成」判定补了磁盘兜底**（机型与版本页版本行全说「未生成」、预演却说
        9 份全是「修改」—— 两个事实源打架）：`build_state` 在 `built` 表没记录时
        **stat 一下交付根的产物文件**（`presets.root()/dist/mkp/presets/<产物名>`），
        有 → `Stale`（待重新生成 = 有旧的），没有 → 照旧 `NeverBuilt`。路径源收进
        `paths`（`dist_root_path()` 只读版 + `MKP_DIR`/`MKP_PRESETS_DIR` 从
        `app::dist` 挪到 `paths`，`app::dist` re-export），**读侧不许第二处自拼**。
        判据 `build_state_falls_back_to_the_disk_when_the_record_is_gone`。
        **真机要重启工作台才生效**（Rust 侧改动）。
    - **增量之三（同日深夜，作者看着 diff 截图点名）**：
      · **对比视图不省略**：作者改了口径 ——「对比的不要省略吧，还是就像这种一样正常的」
        （指的是编辑器里那种整份摊开的 diff）。行内「… N 行未变 …」折叠删掉，所有行
        平铺（未变的也在、不带行号符号），对比与完整的差别只剩红绿底。
        探针断言文案跟着改（「整份摊开、不省略」）。
      · **检查卡补「旧版待重新生成」提示**：作者问「有旧版待生成新的的时候，为什么
        左侧检查里没有提示」—— 确实没有：`issues::collect` 只查数据矛盾/空/套餐孤儿，
        不看产物新旧。`delivery` 里补一条**汇总提示**（`build.stale_versions`，提示档
        —— 旧产物客户端还能下载到，不是要填的空；stale 可能一连十几个版本，逐版一条
        会淹掉检查卡）：「有 N 个版本的产物是旧的」+ 列 uid，去处理落生成页。
        判据 `stale_versions_show_up_as_a_hint`（跟得上时没有这条 / 改配方后有且是提示档）。
        **Rust 侧，真机要重启工作台生效。**
    - **增量之四（2026-10-03，作者对着三张截图点名）**：
      · **「待生成」改「待更新」**（`wording`：`BuildState::Stale` 与 `ArtifactState::Stale`）——
        这一档的前提是磁盘上有旧产物，词必须把「有旧的」说出来，与「未生成」分得开。
        演示桩词表同步。**生成页行上的状态签贴右**（以前紧跟名字，名字一长一短就歪）。
      · **渲染段内键序 = tomlKey 字母序（大小写不敏感）**：作者点名「明明都是 O 开头的
        offset 都是一起的，生成的时候却改变了它的顺序」—— 以前按界面顺序（`layout.order`）
        排，注册表条目一挪、产物键序就漂（M0 登记过的段内键序差异，这次作者拍板）。
        字母序谁都能预期：offset_x/y/z 永远连着，**生成不再改变没改过的行的位置**。
        判据 `sections_are_sorted_by_key_name_so_the_order_never_drifts`。
        **注意**：基线 9 份（手写历史序）与此序不同 —— 下次生成 diff 里会看到一次性的
        键序搬移（值不变），属预期；内置预设那条链（gen-presets）重跑时同序。
      · **生成收尾自动重算 catalog.json（治本）**：作者第三问 = 客户端「下载失败：响应比
        目录登记的大」再现 —— 根因是 `7a10f8b`（PR #20）改了 9 份交付 TOML **没重发清单**，
        catalog 记的 size/sha 全是旧值，客户端字节校验必挂。修法不是检查卡报阻断
        （作者先说要阻断，但 `blocked` 同时压死**生成与发布**两颗按钮，而重算清单恰是
        修复动作 —— 等于堵死修复的路），而是**让记录永远跟着文件走**：
        `wb_generate` 写完产物后调 `dist::write_catalog_json` 重算 catalog.json
        （manifest / source 仍归发布写）。工作台自己从此不再产出不一致；
        绕过工作台的手改/脚本改由预检新增的**交付自查**兜住：
        `dist::audit_catalog`（拿客户端口径逐份对 size+SHA）→
        `issues::preflight` 第三参 → 报**待办** `dist.catalog_mismatch`（显眼、进计数、
        不挡闸）。判据 `audit_passes_when_the_files_match_the_catalog` /
        `audit_catches_a_drifted_file` / `a_mismatched_delivery_catalog_is_a_todo`。
      · **⚠️ `7a10f8b` 的遗留还在 GitHub main 上**：本轮改动只修「以后」；把 main 上
        那份不一致修掉要跑 `npm run publish:presets`（重算 catalog/manifest → 一次性
        分支 → PR）。**先提交本轮代码**，再跑发布脚本（它要求干净工作区）。
    - **增量之五（2026-10-03，作者重启后两点追击）**：
      · **待办「没办法解决」的真相**：目录里的资产条目**按源字节算 SHA**
        （`Catalog::build_from_presets_lenient` 的口径）—— 重算后 models / BBS 那 7 份
        **还是登记着、dist 里还是没有**（从来没人把它们复制进去），所以照着待办文案
        「生成一次」做了也消不掉。修法：`wb_generate` 收尾在重算目录**之前**先
        `write_content` 把引用资产补进交付根 —— 生成一次 = 文件补齐 + 目录重算，
        两头对上，待办自动消失。`write_catalog_json` 注释补了这条依赖。
      · **时间显示转本机时区**：后端 stamp 刻意存 UTC ISO（跨时区一致，注释写明
        「界面上要显示本地时间由前端去转」），**前端漏了转** —— 生成页行、机型页
        版本卡两处补 `localStamp()`（`2026-10-02T16:30:09Z` → `2026-10-03 00:30`）。
        解析不动原样回（老记录可能不是 ISO）。
    - **⚠️ 新增待办（作者 2026-10-03 裁决：第三刀部分作废）**：**整机图归属重做**
      —— 作者原话：「那一刀就是错了，不显示到工作台直接硬编码进客户端完全不好，
      不会编程的用户怎么改图片呢？那不进云端也可以在工作台看到选择才对」。三条裁定：
      ① **文件住 `presets/assets/printers/`**（不是 `src/app/assets/printers/`）；
      作者追加「在 assets 吧，到时候 3mf 也要放」⇒ **所有产品数据资源统一搬
      `presets/assets/<kind>/`（models / icons / bbs / printers）**，`public/` 那个
      资产目录之后退役**（作者问「public 文件夹是不是以后不需要了」—— 是）。
      ② **到客户端 = 构建期从数据目录复制进客户端资源**（保留「不进 dist、不下载」）；
      ③ **交付身份新增 `bundled` 档**（台账登记、工作台可管，**明确不进交付集合、
      不被下载**），与「在菜单 / 仅归档」并列。
      - **增量之六：整机图归属重做落地**（2026-10-03，作者三条裁定全部执行）：
        - **文件搬回数据侧**：`public/assets/{bbs,icons,models}` + `src/app/assets/printers/`
          → **`presets/assets/<kind>/`**（git mv）。`paths::assets_root()`、`presetdata::repo_assets_root()`、
          `runtime::catalog::REPO_ASSET_ROOT` 三处定位同时改指。
        - **台账恢复 4 条 `image`**（id 与旧版一致：`a1-image` / `a1_mini-image` /
          `a1_mini-variant-image` / `p1s-image`），A1 / A1_MINI / P1S 三个机型文件
          的 `image` 字段一并恢复。
        - **交付身份新增 `bundled` 档**（`presetdata::assets::Delivery`：`download` 默认 /
          `bundled`）。判据改成**按档位拦**：`runtime::catalog::dest_of_asset` 见 bundled
          返 None → 不进 catalog files[] / 不进交付集合 / 不复制进 dist（**按类型拦的那支
          留在 `kind_of_asset`，因为下载区本来就没有「图片」这一段**）。
        - **构建期到客户端**：`scripts/copy-assets.mjs`（build / build:workbench 前置）
          做两件事 —— ① bundled 档复制进 `src/app/assets/printers/`（客户端静态 import 的
          落点，那目录是生成物、已 gitignore）；② 带 `workbench` 参数时把整个资产根复制进
          `public/assets/`（**只有工作台需要** `/assets/` 直通做预览；客户端从云端下载，
          拷贝它就是那份「随包副本」—— 客户端包因此从 20 个文件降到 15 个）。
        - **工作台看得见选得着**：资产库「机型图」分类恢复、列表与详情显示「随包」标、
          机型页「换一张…」重新有候选（4 条）；`wb_assets` 的 DTO 带 `delivery`。
          客户端菜单**过滤掉 bundled**（客户端下载不到它，列出来只会让人点一个拿不到的东西）。
        - 判据换向：「资产台账里已无 image 类」→ **「整机图在台账里、且不进 files[]」**
          （catalog.rs / assets.rs / paths.rs / check-bundle / DATA-INVENTORY /
          DATA-ARCHITECTURE / 演示桩的注释与条数锚点全部更新，19 条 = 4 整机图 + 9 BBS +
          3 图标 + 3 模型；可达集 11 = 那 11 条里 4 条是 bundled，`assets_copied` 仍 8）。
        - 验证：Rust 467 全绿、clippy 0、tsc/eslint/stylelint 过、`npm run build` +
          `check:bundle` 绿（15 个文件）、探针【一】29 条全绿。
    - **增量之七：MKP 预设进资产库（作者 2026-10-03「为什么资产库里面不放 mkp 预设」）**
      —— **同一天先做成了 uid 直引（`bundles.toml` 的 `presets` 字段），当天按这句裁决作废**：
      - **资产域新增 `type = 'mkPreset'`**（`AssetKind::MkPreset`）：登记 9 条
        （id = 产物名的 kebab：`a1-standard` / `a1-fastv3.3` / `a1_mini-…` / `p1s-lite` /
        `p2s-standard` / `x1c-lite`）。**不写 `path`**（文件是生成产物，路径由命名规则
        算出 —— 写进来就是第二份会过期的真相），改写 `machineId` + **`versionId`**
        （新字段）；跨文件校验加了「归属版本必须真实存在」。
      - **文件在不在都能登记、都能被套餐选中**（作者：「不只是没文件的时候可以选择，
        有文件也要可以选择」）—— 「有没有生成」是生成页那四档状态的事，不由资产域管。
        资产库列表与详情带**生成状态徽章**（`buildState` → 与生成页同一套判据与词：
        **待生成 / 待更新 / 已生成 / 暂无资源**）。
      - **套餐回到一份 `assetRefs`**：MKP 预设走资产引用，`bundles.toml` 的 `presets`
        字段与 `wb_set_bundle_refs` 的 `presetUids` 参数**全部回滚**（两套引用机制并存
        只会让人问「我到底该在哪挂」）。`BundleResourcesModal` 的 MKP 页签 = 资产库里
        `kind === 'mkPreset'` 的条目，勾选态跟 `assetRefs` 走。
      - catalog：`kind_of_asset(MkPreset) → None`（产物那 9 条 files 已经在了，不登记第二份）；
        客户端菜单过滤 `mkPreset`（它不是下载区文件）。资产 19 → **28 条**（+9）。
      - 顺带修：`npm run build`（客户端）现在会**清掉**上一次工作台构建留下的
        `public/assets/` 直通副本 —— 留着会被 vite 原样拷进安装包（随包副本回归）。
      - 验证：Rust 466 全绿（少了 1 条 = 作废的 uid 直引判据）、clippy 0、前端检查过、
        探针 29 条全绿、`check:bundle` 绿（15 个文件）。

    - **增量之八：随包资产同步链（2026-10-03，分支 `feat/client-assets-pipeline`）**
      —— 作者先要审计、再逐轮拍板，**否掉过一次 AI 的过度优化**：
      「**随包资产要不要复制 ≠ 是否需要 SHA 增量同步**」。AI 中途推过"既然 vite 能 import
      源文件就不用复制"，被作者否掉 —— 那是拿构建工具的便利去否决产品交付语义。
      定稿口径是**明确的交付流水线**（作者原话：工作台永远 `sourceRoot + path` 读源；
      客户端永远 `delivery → pipeline → output`；文件放在哪 ≠ 文件怎么交付）：
      - **链路**：唯一源 `presets/assets/**` + 唯一登记 `presets/assets.toml`（`delivery`）
        → **唯一随包交付根** `client-assets/`（生成物、不入库）→ 进 `dist/`。
      - **对账式同步**（`tools/assets/sync.mjs`）：期望集合 = `delivery='bundled'` 且有 `path`
        的条目（今天 4 张整机图）；**新增写 / SHA 同 skip / SHA 异覆盖 / 不再 bundled 删 /
        源文件缺 → 构建失败**。判据 = 源 SHA vs **目标文件实际 SHA**（目标即状态，不另立
        "上次同步了什么"的台账 —— 那种账在目标被外部删掉时不会自愈）；「删」按**期望集合**
        对账，否则"取消随包"永远清不掉。`client-assets/manifest.json` 是**构建期派生账**
        （不入库、不作为契约；字段 path / sha256 / bytes，刻意没有 generatedAt）。
      - **只由 vite 插件触发**（`tools/assets/plugin.mjs` 挂 `configResolved`）：任何走 vite 的
        命令都经过同一个同步器 —— 包括裸 `npx vite build` 与探针那次构建。三条 npm 前置
        **全部删掉**（作者：「两个入口很容易最后变成两个行为定义」）。
      - **两个集合有意分开**（不是架构裂缝）：`client-assets/` 只装 bundled；工作台构建另装配
        **全部有 `path` 的 19 份**进 `dist/assets/`（工作台要能预览任何登记资产）。两者共用同一条
        URL 规则 `/assets/<path>`（与后端那处前缀同值），但同步器不承担后者的职责。
      - **工作台 dev 直读源**：`/assets/*` 中间件每次请求回 `presets/assets` 取字节，**一个字节
        不落盘**（"读取源，不是同步目标"）；四道闸：只 GET / 路径形状 / realpath 收口 / 必须是文件。
        实测：把交付根搬走仍能取到图 = 读的确实是源。
      - **退役**：`public/assets/`、`src/app/assets/printers/`（一个东西两个落点 = 两个答案）、
        `scripts/copy-assets.mjs`、三条 npm 前置、vite 的 `dropDeliveredAssets`（`public/assets`
        没了它没东西可摘；顺带修掉它写死 `dist`、不认 `--outDir` 的隐患）。
        `heroArt.ts` **只改 import 前缀**（经别名 `@client-assets`），读取逻辑与机型表不动 ——
        **改成台账驱动是第二刀**；在那之前它有个已知代价：台账里减少一条 bundled ⇒ 构建红
        （fail loudly，不是静默破图）。
      - **判据**：新增 `scripts/probes/asset-preview.mjs` —— 逐行走资产库，凡渲染出预览图的行
        必须 `naturalWidth > 0`；按"源里在不在"分档（vite 自己的哈希产物也叫 `/assets/…`，
        桩里还有假路径，拿它们判会误报）。反向测试过：搬走预览产物的 `assets/printers` 与
        `assets/icons` → 报 5 行破图。
      - ★ **实测记一笔**：`vite preview` 有 SPA 回退 —— 取不到的资产返回 `index.html` + **HTTP 200**
        ⇒ **光看状态码永远查不出破图**，能指出"哪一行、哪个文件"的只有 img 解码。
      - **顺序无关实测**：客户端构建 ↔ 工作台构建 ↔ 客户端构建 交替跑，交付根始终 4 份资产、
        探针那份缓存产物 19 份一动没动（`--outDir` 也被正确识别）。
      - 验证：Rust 默认 **631** 通过 / `--features workbench --lib` **473+146+77** 通过、
        clean 后双 feature clippy **0 警告**、`tsc -b`、eslint + stylelint、
        `build` + `check:bundle`（15 个文件干净）+ `check:zero-network`、客户端探针全绿、
        工作台探针【一】全 ok（【二】是已知预存红 `button[title*="钉住"]`）。

    - **增量之九：客户端改成台账驱动取图 + 机型第二个图位（2026-10-03，分支同前）**
      —— 作者：「**客户端不再知道 `a1.webp` 这些具体文件，只认识 catalog 里的 asset id**」，
      并给了五个验收点（import 消失 / 没有第二张表 / URL 必须来自 `catalog.assets[].path` /
      源缺失仍红 / 改 path 后请求跟着变）。
      - **读图链路**：`catalog.assets[]` → 资产 id → `asset.path` → `/assets/<path>`
        （`heroArt.ts` 的 `assetUrlOf` + `pickArt`）。**4 条 import 与 `MODEL_ART` 整张表删掉**，
        `@client-assets` 别名一并撤销（源码不再指向任何生成物）。
      - **机型第二个图位**：`Machine.imageVariant`（= 装了快拆件那张外观图）——
        2026-10-03 之前这条关系只活在客户端硬编码表里，**台账里那张图谁都不引用**；
        现在 `presets/machines/A1_MINI.toml` 引用它、工作台可换（`MachineField::ImageVariant`），
        跨文件校验（`check_asset_refs`）也把它算进去了。
      - **契约订正**：`Machine.image` 的注释原来写着「文件名，前端自己拼资源路径」（旧世界的话），
        改成**资产 id**；`RuntimeCatalog` 补 `assets?:`（**可选** —— 盘上那份 catalog.json
        可能还是旧版，没有这一栏；缺了只是图回落 logo，不挡机型与版本）。
      - **链路侧**：客户端构建也装配 `delivery = 'bundled'` 进 `dist/assets/<path>`（URL 同形、
        **不带哈希** ⇒ 同一条资产在包里只有一份）；**dev 两边读的东西有意不同**：
        客户端读**交付根** `client-assets/`（与打包后一致），工作台读**源** `presets/assets/`
        （后厨要看得见任何登记资产）。
      - **桩数据跟上**：`machine_catalog.json` 的 `image` 从文件名改成资产 id（P2S / X1C 那个
        根本不存在的 `p2s.webp` / `x1c.webp` 清空 —— 那是移植时留下的错路）；
        `mock.ts` 的 `getRuntimeCatalog()` 补 `assets[]`（**真 id + 真 path**，于是浏览器演示里的
        大图是真取到的，不是画个占位）。
      - **判据**：`home-flow.mjs --pick` 补两条**分层大图**断言（`a1mini.webp` /
        `a1mini-variant.webp`）—— 量的是 `naturalWidth`，断的是「URL 是不是台账里的那个 path」。
      - **五个验收点的实测**：③ URL = `/assets/printers/p1s.webp`（探针实测，无哈希名）；
        ④ 源文件缺失 → 构建红并报出是哪条资产 + 怎么修；⑤ 改台账 path → 内嵌 catalog 的
        revision 变（`ad1b1189…` → `25da03d6…`）+ **交付根删掉旧文件**（同步日志
        `删除 1（printers/a1.webp）`）+ 装配跟着变；另在浏览器侧只改桩数据的一个 path，
        客户端请求就变成 `/assets/printers/p1s.webp` 且断言变红（证明它跟着数据走、判据是活的）。
      - **客户端 dev 的分源实测**：取到的字节与 `client-assets/` 逐字节一致；把交付根搬走 → 404，
        搬回 → 200（证明客户端 dev 读的是交付根，与工作台 dev 读源正好对照）。
      - 验证：Rust 默认 **631** / workbench **473+146+77**、双 feature clippy **0 警告**、
        `tsc -b`、eslint + stylelint、`build` + `check:bundle`（15 干净）+ `check:zero-network`、
        四个探针（home-flow / presets / workbench-build【一】/ asset-preview）全绿。
      - 顺手记一笔：`presets/dist/catalog.json` 那一版的 definition 还没带 `imageVariant` ——
        **不用手改**：它由工作台「生成 / 发布」重算（`write_catalog_json` 的调用者就那两个），
        而发布收尾本来就会重算一遍，所以到发布那一刻一定与源同代。

    - **增量之十：图位分层 · 版本图位（2026-10-03，第三刀第一片）**
      —— 作者定层级：**品牌图 / 机型图 / 版本图，版本没有图就回落机型图**；
      「标准版」与「快拆版」是同一台机器下两个独立的版本实体，各自有各自的图。
      并且：**`imageVariant`（装了快拆件那张）不与版本混** —— 它是**硬件外观变体**，
      客户端选择层级不再用它（数据留在机型文件里，等将来有真正的变体模型再说）。
      - **`[[versions]]` 新增 `image`（资产 id）**：`MachineVersion` / `VersionField::Image`
        （可清空 = 回落）/ 客户端 `MachineVersion.image` / 工作台 `VersionView.image` +
        版本详情卡那一格素材格与选择器（`VersionField` 加 `'image'`）。
      - **跨文件校验把版本这一格也算进去**（`check_asset_refs`）：版本图打错字的后果是
        **静默回落成机型图** —— 界面上看不出是打错了还是压根没配，所以升级成 error。
      - **客户端回落链**（`heroArt.pickArt`）：`版本图 → 机型图 → 品牌字标 → 空`；
        品牌图那一半（把 logo 抽成资产、`RuntimeCatalog.brands`）留给本刀第二片。
      - **判据**：`home-flow.mjs --pick` 的分层断言改成「第二级与第三级**同图**」——
        A1 mini 的 `STANDARD` 今天没有版本图，理应回落机型图；哪天给那一版配了版本图，
        这条断言会红，**那正是它该红的时候**（改数据要改判据）。
      - **能力反向验证**：临时给 `A1_MINI/STANDARD` 配 `image = 'a1_mini-variant-image'` →
        catalog 的 `versions[].image` 带上（revision 变）→ 客户端第三级真的换成
        `/assets/printers/a1mini-variant.webp`、第二级仍是机型图，断言如实变红 ⇒
        版本图**被认**、回落**只在该回落时**发生。验完已还原。
      - 验证：Rust 默认 **631** / workbench **473+146+77**、双 feature clippy **0 警告**、
        `tsc -b`、eslint + stylelint、`build` + `check:bundle`（15 干净）+ `check:zero-network`、
        四个探针全绿。**顺带记一条判据的功劳**：中途还原了 `A1_MINI.toml` 却忘了重算内嵌
        catalog，`embedded_matches_rebuild` 当场把 `cargo test` 拉红 —— 那条判据不是装饰。
      - **本刀第二片（已落地，提交 `823863a`，2026-10-03 追记）**：品牌 logo 进资产体系 —— `src/app/assets/bambuLogo.ts` 那个
        **双色 data URI 抽成 `presets/assets/brands/` 里的真文件**（实测**只有深色一版在被用**，
        `BAMBU_LOGO_LIGHT` 是死导出 ⇒ 没有颜色取舍）+ 台账登记一条（`type='image'`、
        **不写 `machineId`** = 公共素材，`Asset` 文档早就预留了这条路）+
        `brands.toml` 的 `logo` 从那个**早就不存在的文件名**改成资产 id +
        `RuntimeCatalog` 补 `brands`（客户端要靠它查品牌图）+ 客户端链上「品牌图 → 内置字标兜底」。

     - **增量之十一：资产检查面板（2026-10-03，第四刀，分支 `feat/client-assets-pipeline`）**
      —— 作者看着资产库说「右侧详细信息里没有文件的真实文件名、连路径也没有，还希望能用
      系统的文件管理器查看位置」。详情卡从"登记表"变成**资产检查面板**：真实文件名 /
      源绝对路径（未搬入时改成期望路径）/ 产物路径（mkPreset）/ SHA-256 / 尺寸 / 格式 /
      大小 / delivery / 引用 / 状态 + **【在访达中显示】**。
      - **落点与当初的建议有一处不同，理由记在这**：重读数**不并进 `wb_assets` 的
        `AssetView`**，而是新开一条**选中才问**的读命令 `wb_asset_inspect` ——
        SHA-256 与大小要读真实字节（模型实测 3.2 MB），而列表每次筛选 / 搜索词一变就重取，
        并进去就是"每敲一个字读 4 MB"。前端那条 effect 也只依赖选中（不依赖 list）。
      - **`wb_reveal_asset(id)`：前端只传资产 id**，路径由后端自己算（源根 + 台账 path /
        产物路径）—— 不给前端传任意路径的机会；**只读、只开窗口、一个状态都不碰**
        （照客户端 `ipc::mine::reveal_in_folder` 的纪律）；文件不在就如实拒绝并附期望路径，
        界面那一侧同时把按钮灰掉、原因写进 title（不给必被拒的按钮）。
      - **`mkPreset` 指向产物**：产物名走 `build::preset_file_name`（与生成端**同一个
        函数**），`productPath` 相对仓库根（`presets/dist/mkp/presets/A1-standard.toml`）。
        产物没生成时读数全空、路径那一栏改成**期望路径** —— 不去猜第二个落点。
      - **尺寸读文件头**（png / webp / svg 的 viewBox，几十行自读，不拖 `image` 整库）：
        读不出就如实 `None`（界面写"不是图片 / 读不出"），不猜。svg viewBox 有小数
        （品牌字标 485.05 × 175.15）→ 四舍五入，不截断。
      - **判据**：Rust 新增 5 条（真数据 webp 全套读数 + 品牌 svg 小数、mkPreset 产物、
        文件不在给期望路径且「在访达中显示」拒绝、三个读取器的纯字节用例、未知 id 如实拒）；
        异步读命令单子加两条（`wb_asset_inspect` / `wb_reveal_asset`）；探针
        `asset-preview.mjs` 补 6 条（真实文件名 / 源绝对路径 / 格式·尺寸·大小 / SHA-256
        64 位 / 「在访达中显示」演示里如实失败 / mkPreset 产物路径 + 未生成时灰按钮）。
      - **反向验证**：探针先红后绿 —— 第一版 `querySelector('[class*="cardBody"]')`
        撞上外壳里常驻的另外三张卡（实测 4 张），按「资产检查」认卡才绿。
      - 验证：Rust 默认 **631** / workbench **478 + 146 + 77**、双 feature clippy **0 警告**、
        `tsc -b`、eslint + stylelint、`build`（随包 5 份）+ `check:bundle`（16 干净）+
        `check:zero-network`、四个探针（工作台总探针只红在已知预存的那条 `钉住`）。
      - **交接文档已同步**：`ASSET-CHAIN-HANDOFF.md`（§2 提交表 / §5 已落地 / §7 数字 /
        §8 新坑 / §10 清单）。

    - **增量之十二：品牌升成一等条目 + 套餐页可读性（2026-10-03，分支 `feat/wb-brands-readability`）**
      —— 作者看着五张截图一口气点了五件事：
      - **① 机型与版本页加品牌**（「在这一页多加一个品牌吧，右侧也是一样可以编辑，哪些是这个品牌的
        机型之类的，品牌图、显示名，之类，不管客户端消不消费都提供」）：左列分「品牌 / 机型」两段，
        点品牌 → 右侧品牌卡（显示名可改 / 品牌图走资产选择器、存的是**资产 id** / 「这个品牌下的机型」
        反查 / 新增品牌）。机型的「品牌」那一格改成**引用品牌 id**（原来下拉给的是显示名 ——
        选一次会把显示名写进机型文件，是条静默脏写），旁边多一颗「看品牌」。数据层：
        `BrandField{Name,Logo}` + `set_brand_field` / `add_brand` / `write_brands`
        （**值面 + 文档面**一起改：`brands.toml` 里那段解释 logo 的注释必须原样留住），
        命令 `wb_set_brand_field` / `wb_add_brand`（写命令，即时落盘）。品牌 id 不查字符集
        （真数据叫 `Bambu Lab`，带空格与大小写），只查非空 + 不撞名（大小写不敏感）。
      - **② 套餐页的「名字」**（「左侧：前的是什么？用户看不懂，名字：左边右边都很像」）：
        左列行改**两行**（上行 = 显示名 + 版本数，下行 = id + 「装了 N 个文件」——
        四个事实挤一行时显示名会被省略成「官…」，实测），卡头大标题换显示名、id 退成附注。
      - **③ 选择版本弹窗去背景**（「这个没必要加背景吧」）：`rowPick` 那层垫底拿掉，
        由勾选框自己说状态。
      - **④ 弹窗改树状**（「为什么重复了，我希望的是像参数台这样的显示…像树状显示一样」）：
        机型组头 + 缩进的版本行（复用参数台左树那几颗类 `pMg` / `pMgName` / `pVrow` / `pVmark`），
        行里不再把「机型/版本」与 id 摊两遍。
      - **⑤ 状态栏与"碰到一起"**（「碰到一起了……左下角的也是，没必要显示这个吧，这么长」）：
        `upstream` 那块的病根是 `/Users/…/presets` **整条路径是一个不可断的"词"**，
        flex 项 `min-width: auto` 不肯缩 ⇒ 撑出导航栏压到内容区；治法是
        `min-width: 0 + overflow: hidden + ellipsis` 三件套，行上只留 `仓库名/目录名`
        （完整路径在 title 里）；「待确认」chip 里 uid 与「它现在指着谁」补上间距与 `→`
        （原来粘成 `X1C/LITEA1_MINI_STANDARD`）；`.addRow` 里的说明文字允许换行
        （原来被卡片右侧裁掉）。
      - **判据**：Rust +2（`editing_a_brand_keeps_the_file_comments` —— 没改就逐字节一样、
        改要落盘且**注释一行不少**、清 logo = 删键、清显示名当场拒；`adding_a_brand_appends_and_refuses_collisions`），
        机型清单那条补「五台机型都归 Bambu Lab + 显示名 / 品牌图照给」；新探针
        `scripts/probes/brands-bundles.mjs` **19 条**（品牌行 / 品牌卡与品牌图解码 / 改名跟着变 /
        新增与撞名 / 套餐两行读法 / 卡头显示名 / 树状与无背景 / chip 间距 / 状态栏短名与不越界）。
        顺带：机型行补 `t-machine-*` 锚点；界面文字里漏出来的 markdown 星号清掉
        （JSX 不认识它，会原样上屏）。
      - 验证：Rust 默认 **633** / workbench **480 + 146 + 77**、双 feature clippy 0 警告、
        `tsc -b`、eslint + stylelint、`build`（随包 5 份）+ `check:bundle`（16 干净）+
        `check:zero-network`、五条探针（`workbench-build` 只红在已知预存那条「钉住」）。

    - **增量之十三：品牌树 + 机型尺寸 + 禁区编辑器（2026-10-03，分支 `feat/wb-brands-readability`）**
      —— 作者两句原话：「我们现在这个品牌的我也不喜欢呀……我想象中的就像那种树状的感觉一样，
      加了新的品牌就有新的那个数，然后把某一个机型移到其他品牌下，就是那种正常的移动，
      很明显的能看到他们的父子关系。现在这种这么割裂」；「（照旧面板）它还有什么尺寸啊，
      什么进去那一些我们现在都没做」。
      - **① 左列改成一棵品牌树**：品牌是父节点（折叠三角 + 显示名 + 内部名 + 台数），
        机型缩进当子节点（左侧一条浅竖线是层级的唯一记号，不垫底色）。**默认全部展开**，
        品牌行可点收（收起的品牌id 存在一次会话的 `Set` 里，不落盘 —— 那是"我现在想少看几台"
        的临时动作）；筛选框同时管两边（品牌命中整组留下、机型命中只留那一支）。
        ★ **分组的依据是机型自己的 `brand` 一格**（不是后端 `BrandView.machines` 那张反查表）
        —— 两个来源不一致时以机型为准，否则会画出「品牌说有 A1、机型说自己归别家」这种自相矛盾的树。
      - **② 机型可移到别的品牌下**：机型行右键 →「移到品牌…」→ 列出各家（显示名 / 内部名 /
        台数，当前那家禁用并标「就在这儿」）→ 选一个即时落盘 + toast 说清"从哪家挪到哪家"。
        新命令 `wb_move_machine_to_brand`：**只改机型文件的 `brand` 一格**（复用
        `set_machine_field` 那条路），品牌侧的名单是反查不落盘 —— 所以不存在两份归属要同步；
        多一道「目标品牌真的存在」的校验（打错一个字会在盘上留下悬空归属，
        那台机器会从所有分组里消失）。
        ★ **右键菜单的键要与版本行分开**：版本行用的是没前缀的 `机型/版本`，机型行用
        `machine:<id>`（一个 `A1` 既可能是机型、也可能是某台机器的版本名）。
      - **③ 尺寸卡从「已配置 / 禁区 N 块」升成六组读数**（床身 / 移动范围 / 边缘 / 涂胶 /
        标定点 / 标志位，每组带原始键名），加「编辑尺寸」模态框（六组两列网格 + 43 格数字 +
        **「从相似机型复制标定点」** —— 那十格是实测值，谁都记不住）+「编辑禁区」入口。
        新命令 `wb_set_machine_dimensions`、数据层 `Machine::set_dimensions`（值面 + 文档面
        整表替换，保住 `[dimensions]` 外的注释）。
        ★★ **一处与旧面板不同、必须记住**：旧面板的 `stripEmptyGroups` 是**在 JSON 上**做的
        （缺字段 = 没配，合法）；而本仓 `load_dimensions` 是**「全有或全无」** ——
        `[dimensions]` 在，六个子表就都得在，缺一个是 `Corrupted`。所以"剔全零组"在这里
        只能是**写一组零值**（`TrimmedDimensions` 的 `Option` 只在 `flags` 上真删），
        真把子表删掉那份文件下次就读不回来了 —— **这是实测撞出来的**（判据先红后改）。
      - **④ 禁区编辑器**（`ZoneEditorModal`）：画布 `viewBox = 0 0 bedSize.width bedSize.depth`、
        `svgY = depth − y`（机器坐标原点在床身前左角、y 向上）；工具栏 添加/删除选中/复制/
        撤销/重做/清空 + 缩放 0.5~8；画布点击加顶点、拖顶点（0.1mm）、右键删顶点、
        右侧列表（点数 + 可展开改坐标）；**最少 3 点**；⌘Z / ⇧⌘Z 历史栈（编辑器内部，
        不进外壳草稿栈）。新命令 `wb_set_machine_zones`、数据层 `Catalog::set_zones`：
        ★ **清空 = 删掉 `forbidden_zones/<id>.toml`**（留一个空的 `[[zones]]` 会让"有没有禁区"
        多出一种写法：缺键 / 空表，而下游只当缺键是"没有"）；落盘走 toml_edit
        （不是拼字符串 —— 整份文件的注释要留住）。★ 坐标口径**不要**照搬
        `src/app/params/TowerMap.tsx` 那套：它算的是**可打印区（plate frame）**坐标，
        禁区是**板/床身**口径。
      - **判据**：Rust +4（默认 **637**、工作台 lib **484 + 146 + 77**）——
        `writing_dimensions_keeps_comments_and_drops_all_zero_groups`（改一格落盘 +
        注释留在 + 六组仍齐）、`writing_dimensions_refuses_a_zero_bed_and_non_finite_numbers`
        （0 床身 / NaN 当场拒且不碰文件）、`writing_zones_roundtrips_points_and_deleting_is_a_real_delete`
        （逐点回读 + 清空真删文件 + 幂等 + 新建目录）、
        `moving_a_machine_to_another_brand_rewrites_one_field`（只动 brand 那一行）。
        新探针 `scripts/probes/machines-dims-zones.mjs` **23 条**（树/折叠/筛选/新增品牌/
        移到品牌后两边台数都变/六组读数逐格一致/模态框改一格保存回读/viewBox 就是床身/
        两块多边形 6+4 点/`(0,0)` 落在画布左下角/列表点数/清空后那一格变「没有禁区文件」）。
        `brands-bundles.mjs` 19 条**不用改**（`t-brand-*` / `t-machine-*` 两个锚点在树里保住了）。
      - **踩过的坑（都写进代码注释了）**：① `!(x > 0.0)` 被 clippy 拦（NaN 语义绕）→
        写成 `is_finite() && > 0.0`；② 右键菜单点中一项的顺序是「先 `onClose()`（target 变 null）
        → 再 `onSelect()`」→ 机器 id 必须在**构造菜单时闭包进去**，不在 `onSelect` 里现读；
        ③ 探针点菜单项要用 `button[role="menuitem"]`，`text=移到品牌…` 的省略号会让文本匹配飘；
        ④ 「新增品牌」的提交键要在**弹窗内**点（左树那颗同名按钮会一起命中，`.last()` 点到树里）；
        ⑤ 弹窗提交后要**等遮罩真消失**再往下走，否则右键打到遮罩上。
      - 验证：双 feature clippy **0 警告**、`fmt`、`tsc -b`、eslint + stylelint、
        `build`（随包 5 份）+ `check:bundle`（16 干净）+ `check:zero-network`；
        **已知预存红**：`workbench-build.mjs` 现在红 **3 条**（`钉住` 那步超时中断，
        以及它之后的 `preset.toml × 2 份` / 上传那两条 —— 都在**生成与发布页**，
        与本刀无关；基线（stash 后）在 `钉住` 那步就中断，跑不到后面那两条，故未能对照，
        登记为独立观察项）。

    - **增量之十四：catalog 生命周期收口 + 预设页两个实机问题（2026-10-04，分支
      `feat/catalog-lifecycle-cleanup`）** —— 起因是作者两张客户端截图：切片器档出现 5 行
      「presets/」、MKP 预设「下载失败：内容对不上」。作者随后纠正了第一版报告的归因
      （**"客户端内置预设文件" ≠ "客户端内置预设目录"**；MKP TOML / BBS JSON 都是云端下载资源，
      随包那份只是 bootstrap 目录，不能拿它永久当下载校验依据）—— 这一条把归因改写成
      「**不是两批 TOML 不一致，而是下载校验用的是旧/bootstrap 目录，而 OTA 目录没生效**」。
      - **① 切片器档「presets/」幽灵行**（`ipc/presets.rs::version_files_dto`）：套餐的
        `assetRefs` 是**混合**的，MKP 预设那条 `path` 是空串；它被 `file_kind` 的
        `_ => "bbs_profile"` 兜底贴成切片器文件，落点写成 `format!("presets/{}", "")` = `presets/`、
        主名是空串 ⇒ 界面上一行「presets/」（同机型多版本按 path 去重 ⇒ 5 行）。
        改法：`file_kind` 返回 `Option`，**认不出就说认不出**（去掉兜底）；切片器一支跳过
        `MkPreset` 与空 path；`preset_files_dto` 那处 `expect` 成显式不可能分支。
      - **② ★★ 启动覆盖 OTA 目录（真 bug，本次核心）**：`lib.rs::setup` 每次启动都调
        `runtime::release::release_catalog`（= `release_bytes(EMBEDDED)`，**升级语义**：
        盘上不同就换成随包那份）。时序变成
        `启动铺 EMBEDDED(A) → OTA 成功写成 B → 再启动又铺回 A → 下载拿 A 的旧 SHA 比云端 B 的文件 → SHA_MISMATCH`。
        **下载链本身是正确的**（`download_runtime_file` → `load_released_catalog` 读的是盘上
        `Internal/catalog.json`；`apply_remote_update` 也确实原子替换了它）—— 坏的是**启动**
        把它覆盖回去。改法：`release.rs` 拆出 **`ensure_released`（只铺空位，有就一个字节不动）**
        给启动与 `load_released_catalog_bytes` 的兜底用；`release_bytes` 留给 `apply_remote_update`。
        判据：`startup_never_overwrites_an_ota_catalog` + `the_startup_path_only_ever_ensures_never_upgrades`
        （源码扫描：`lib.rs` 里 `release::` 之后只能是 `ensure_released`）。
      - **③ SHA 不匹配两种话**（`delivery::sha_mismatch_message`）：本机目录 revision ≠ 远端 →
        说「本地目录停在 X，云端已是 Y —— 先点『检查更新』刷新目录再下载」；两 revision 相同却字节
        不符才是源头坏件；取不到远端（离线）退回保守说法，**不拿本机目录当"远端"**。
      - **④ ★ 随包 catalog 去交付文件 SHA**（`CatalogFile.sha256/size` 改 `Option`，
        `build_from_repo` 走 `FileHashes::None`）：随包那份不再拿 `crates/preset/assets/presets/`
        的构建用 TOML 算 SHA（那份期望值注定与云端不同步）；期望值归 OTA / 发布侧目录。
        `gen-catalog` 重跑后随包 24 份文件**一份 SHA 都不带**。连带改的消费点：
        `deliver` / `file_status` / `inspect` / `official_text` / `other_known_versions` /
        `save_active`（改用盘上真字节算指纹）/ `mine::based_on` / `apply_active_preset` /
        进度水位 / `dist.rs` 发布侧核对（发布侧**必须**有期望值，没有当场报错）。
        新判据：`bundled_catalog_lists_files_without_expecting_their_bytes`、
        `published_catalog_expects_the_real_bytes`、
        `delivers_without_checking_when_the_catalog_expects_nothing`、
        `stale_catalog_gets_a_different_message_than_a_bad_file`。
      - **⑤ 契约落进总纲**：`docs/DATA-ARCHITECTURE.md` §1③ 新增「catalog 的一生：
        bootstrap → OTA → 当前」+ 三条硬规矩（启动绝不覆盖 / 期望值只来自当前 / 对不上报目录过期）。
      - **清理**：核查 98 条 Rust 命令**零死命令**；`bridge.ts` 两个 `notWired`
        （`copyToSlicer` / `downloadFiles`）是**契约内待接入口**（mock 有实现、界面有调用点），
        **不是死代码，保留**。注释按"留结论、删过程"压缩了本轮新增的几段长注释，
        并修掉 `catalog.rs` 一处**错位注释**（`REPO_ASSET_ROOT` 的说明被贴在 `kind` 文档块里）。
      - 判据：Rust 默认 **284**、workbench lib **499 + 146 + 77**；双 feature clippy 0 警告、
        `fmt`、`tsc -b`、eslint + stylelint、`build`（随包 5 份）+ `check:bundle`（16 干净）+
        `check:zero-network` 全绿。
      - **待作者裁决**：`A41-PORT-NOTES.md`（2026-10-01 那次移植的过程账，已进主线）只被记忆文件
        引用 = 孤立死文档，建议删（**删文档是产品取舍，未擅动**）。
        → **2026-10-04 作者裁决：删**。已删除（该轮工作早已进主线，账留在 HANDOFF 与记忆里）。

    - **增量之十五：交付面契约文档 + 两条欠账（2026-10-04，同一分支）** —— 作者看第二张截图
      （切片器档 A1 机型下点 0.2mm BBS → 「下载失败：远端还没有这份文件」）追问"这个路径到底是谁决定的"，
      并要一份**完整文档**（放 `docs/`，不是聊天里列）。查清如下：
      - **① 路径不是硬编码**：`mkp/<kind 目录>/<台账 path 去掉第一段>`，唯一算法 =
        `runtime::catalog::dest_of_asset`（两端共用；客户端拼 URL、工作台拼 dist 落点）。
        台账（`presets/assets.toml`）登记 `path`，其余全自动。`dest_of_asset` 返回 `None` 两种：
        `delivery='bundled'`（随包）/ 该 kind 无落点（今天的 `image`）。
      - **② 「远端还没有这份文件」真因 = 登记面 ≠ 实体面**（**未修**）：
        `presets/dist/catalog.json` 登记了 4 条 0.2mm，但 `presets/dist/mkp/bbs/Process/0.2mm/`
        **目录根本不存在**。`referenced_assets`（决定复制哪些）= 机型 image/icon + **套餐 assetRefs**；
        `build_from_presets_lenient`（决定登记哪些）= 全部有落点的 download 资产。0.2mm 没被任何套餐引用
        ⇒ 没复制；但登记了 ⇒ 客户端看得见、点得动、**必然 404**。`dist.rs:795-798` 那段的"登记面刻意比交付面宽"
        在"客户端只看不下载"前提下成立，但客户端把它们渲染成了可点的下载按钮。
        修法候选：A 实体面补齐（**倾向**：登记即承诺）／B 登记面收窄。
      - **③ 发布闸缺口**（作者点名）：现在没有"点一下、逐项打勾、全绿才允许推送"的发布闸。
        现有检查（`audit_catalog` / `deliverable_set` / `dist_strays` / `version_orphans` / `wb_preflight`）
        是**散落且不阻断**的；**`登记面 == 实体面` 这条检查不存在**（0.2mm 就是这么漏的）。
      - **④ `minVersion` / 结构代次不存在**：只有 `catalogSchema: u32`（`runtime/catalog.rs:41`，加字段不升号）。
        "结构变了 → 最小客户端版本跟着变 → 智能判定"是**空白**。
      - **⑤ 发布走 PR 还是本地**：现状是**两半靠人接** —— 工作台只写 `presets/dist/`（+ 按钮）；
        入库（git commit）你在 IDE 做；推分支 + 开 PR 走 `npm run publish:presets` 或手动；
        合并你在 GitHub 点；**工作台不感知 PR 状态**。
      - **产出文档**：`docs/PRESET-DELIVERY-CONTRACT.md`（路径三层 / 幽灵行 / 404 真假两说 / 发布现状 /
        还缺什么 / 判据表）；总纲 §4 欠账清单补 3 条（第 7/8/9 条）。
      - **删档**：`A41-PORT-NOTES.md` 按作者裁决删除。

    - **增量之十六：发布链重定义（2026-10-04，作者三轮澄清后定稿）** —— 作者指出
      `dist/mkp/bbs/...` 这种副本本身就不该存在：「一个文件不该因为被某个 MKP 使用就复制一份」。
      三条裁决：**① A 类资产不复制**（`presets/assets/` 是唯一实体）**② `catalog.path` 相对发布根**
      （**发布根 = `presets/`**；台账 `path` 相对资产根，发布时由 `kind_dir` 补 `assets/` 前缀）
      **③ 保留薄 `dist/`**（只放 B 类渲染产物 `mkp/presets/*.toml` + catalog/source/manifest 元数据）。
      - **唯一基准规则**：`下载 URL = baseUrl + catalog.path`；**`catalog.path` 同时是"取哪"与"落哪"**。
        ★ 已核实可行性：`source.json` 的 `baseUrl` **缺省 = 与 source.json 同目录 = `presets/dist/`**，
        A 类资产在 `presets/assets/`（兄弟目录），所以 `catalog.path` 以 `assets/` 开头就拼得对 ——
        **发布侧不需要改 source.json 的写法**。
      - **A/B 裁决作废**：共同前提（"`catalog.path` 必须指向 `dist/mkp/…`"）本身是错的；
        C 定下后"登记面 ≠ 实体面"这个说法**自动消失**（登记面与实体面本来就是同一个面）。
      - **★ 结构性障碍（唯一真取舍，待裁）**：客户端现在只有一个内部根、落点规则是
        `<appDataDir>/mkp/<kind>/…`；A 类改指 `assets/…` 后会在 `<appDataDir>` 下多出 `assets/`，
        连带影响 `paths::mkp_dir` / 归档结构 / `mkp_dir_is_created_empty` / 信任扫描面 / `get_local_files`。
        两条出路：**甲 = 落点也跟 `catalog.path` 走**（`<appDataDir>/<path>`，"一个字段管一切"，我倾向）；
        **乙 = `catalog.path` 只做 URL 尾段、落点另算**（客户端布局不变，但"取哪/放哪"分成两套规则）。
      - **发布定位**：Git 仓库本身即发布物（`assets/` 同时是源与云端资产，无复制）。
        工作台负责全程：检查 → 生成 → commit → push → 建 PR → 回读 CI 状态；
        **合并第一版仍留给你在 GitHub 点**（不可逆动作留给有审计的平台）。
      - **发布闸**：`PublishAudit { items: Vec<AuditItem> }`（id/name/status/severity/details/
        affected_files/fix_hint）+ 十五项逐项打勾 + 全绿才亮「创建发布 PR」。
        `wb_generate` / `wb_publish` 将来收进内部实现，对外只暴露一个【发布】。
      - **minVersion**：机器算**结构签名**（只加可选字段时签名不变）+ **显式规则表**（人工登记
        "签名→最低版本"）+ 查不到签名就**禁止发布**；客户端拿到目录先比再下（而不是走到下载才报 404）。
      - **产出文档**：**`docs/PUBLISH-ARCHITECTURE.md`（新建，本文是发布链的根规则）**；
        `docs/PRESET-DELIVERY-CONTRACT.md` 降级为**问题档案**（§3.1 的 A/B 已废、§5 已被取代）；
        总纲 §4 欠账第 7/8/9 条按新裁决重写；总纲文件头挂上本文档指引。
      - **落地三刀**（各自独立可验收，第一刀是地基）：① 路径语义切换（`dest_of_asset` 改发布根基准 +
        A 类不复制进 dist + 客户端落点跟改）② 发布闸（`wb_publish_preflight` + `PublishGateModal`）
        ③ minVersion + PR 建单/回读。**第一刀里"客户端落点"的甲/乙取舍必须最先定。**
    - **增量之十七：第一刀「路径语义切换」开工（2026-10-04，分支 `feat/param-def-controls-undo`）**
      —— **客户端落点裁甲**（`<appDataDir>/<catalog.path>`：一个字段同时管"取哪"与"放哪"）。
      规则源头仍是 `docs/PUBLISH-ARCHITECTURE.md`（§1.2 / §3.2），本文只记**任务与进度**。

      **任务清单（4/4）**：

      | # | 任务 | 状态 | 验收（判据） |
      |---|---|---|---|
      | 1 | **路径语义收敛**：`catalog.path` 同时定义云端位置与客户端落点，不再有 kind→目录 的第二套映射 | ✅ | `dest_of_asset` = `assets/<台账 path>`；B 类 = `dist/mkp/presets/<file>`；`kind_dir` 那套路由已消失 |
      | 2 | **A 类不再复制进 dist** + 清理 `presets/dist/mkp/{bbs,icons}` | ✅ | `write_content` 只体检不落盘；8 份副本 `git rm`；`dist/` 只剩 `mkp/presets/` + 元数据；manifest / assets_index 的 `relativePath` 改 `assets/…`、`dist/mkp/presets/…` |
      | 3 | **客户端落点改 `<appDataDir>/<catalog.path>`**（下载 / 归档 / 信任扫描 / 本地文件视图跟改） | ✅ | `paths::released_file` 是唯一实现处；归档 `<archive>/<path>` 同形；`resolve_in` 防穿越仍在；`mkp_dir` 已退役 |
      | 4 | **B 类渲染产物仍在 `dist/mkp/presets/`**；随包目录与 `gen-catalog` 同步 | ✅ | `PRESET_DEST_DIR` = `dist/mkp/presets`；`catalog.generated.json` 与 `presets/dist/catalog.json` 同一代（`embedded_matches_rebuild` 绿） |

      **★ 第 6 条判据（作者拍板，与本刀同一 commit）**：**`based_on` 不引旧路径兼容层**。
      老用户文件里记的 `mkp/presets/…` 在新目录（`dist/mkp/presets/…`）下**认不出就是 `unknown`**，
      不回填、不按文件名猜；**不许伪造 `current` / `outdated`**。老文件一个字节不动，
      照旧能读、能改、能应用 —— 掉的只是「基于哪一版」那句判断。
      判据：`runtime/mine.rs::an_old_shaped_based_on_is_unknown_and_never_guessed`
      （血统摘要与目录里那一份**完全相同**，唯一让答案变 `Unknown` 的是路径形状）。

      **本刀实际落地的改动（2）：**
      - **三条说旧规则的判据改口径**：`runtime/catalog.rs` 漏改的 `starts_with("mkp/presets/")` →
        `PRESET_DEST_DIR`；`dist.rs` 的 `dist_expected_set` 期望 7→**9**（注释里 3+1+1+1+3 的算式原本就写错）；
        `dist.rs` 那条「写盘 8 条资产」**反过来钉** —— `write_content` 之后 dist 下除三份目录 JSON 外
        **零交付文件**（A 类原地交付），核心那一半（登记面每一条在发布根取得到、字节对得上）保留。
      - **UI 收掉第三套路径**：`ipc/presets.rs` 的 `version_files_dto` / `preset_files_dto` 给界面的
        `presets/<台账 path>` 改成 `dest_of_asset` 同形的 `assets/<台账 path>`；幽灵行判据跟着改口径。
      - **发布脚本换基准**：`scripts/publish-presets.mjs` 的交付文件存在性 / 读真字节改以
        **发布根 `presets/`** 为基准（`PUBLISH` 常量）；`source.json` 的 `catalog` 字段仍是
        「相对 source.json 所在目录」，保持 `DIST` 基准不动。实跑：五道校验全过（17 条交付文件
        在发布根里真存在、SHA / 大小对得上）。
      - **注释与 mock 清扫**：`mkp/bbs` / `mkp/presets` / `mkp/icons` / `archive/mkp/…` 全部换成新形状
        （BBS 页、`presetTree.ts`、`contract.ts`、`mock.ts`、mockBackend、`runtime/mod.rs` 与 `state.rs`
        的布局示意、`dist.rs` 模块头目录图、`workbench/paths.rs` 的 `kind_dir` 注释）。
      - **`presets/dist/` 重出到同一代**：`git rm` 掉 `mkp/bbs`(5) + `mkp/icons`(3)，
        再走一次 `wb_publish`（与工作台点【发布】同一条命令）重出 catalog / manifest / assets_index。
        重出后 `catalog.json` 24 条：A 类 `assets/…`、B 类 `dist/mkp/presets/…`，**全部带真 SHA / 大小**；
        `dist/` 下只剩 `mkp/presets/*.toml`(9) + 元数据。
        ★ 顺带收掉的老账：**4 份 0.2mm BBS 与 3 份模型现在真能取到了**（以前登记了却不在交付面；
        新语义下它们原地住在 `presets/assets/`，取得到）。

      **遗留裂纹（本刀之后已清掉，见增量之十八）**：旧 `ClientDataPackage` 那一套
      模拟云端镜像已**直接退役**，没有迁移成兼容结构。

    - **增量之十八：旧 `ClientDataPackage` 直接退役（2026-10-04，作者拍板，同一分支）**
      —— 作者的原话：**「现在已经有一个真实发布契约了，再保留一套『假的云端数据结构』
      只会让以后的人继续误以为 `presets/mkp/...` 还是合法路径。直接退役，不要留悬案。」**
      - **删掉的四个模块**：`src/workbench/clientPackage.ts`（说明书生成器）、
        `src/workbench/cloud.ts`（模拟云端）、`src/workbench/compat.ts`（兼容性清单 / 自动判断）、
        `src/workbench/fixtures/cloud-presets.json`（108 KB 静态快照）。
      - **契约清掉六个类型**：`ClientDataMeta` / `ClientMachine` / `ClientFieldDef` /
        `ReleasePreset` / `Release` / `ClientDataPackage`（`src/api/contract.ts`）；
        `STORAGE.cloud`（`mkp.cloud.presets`）一并删。
      - **界面**：生成与发布页去掉「② 客户端数据包」（包版本三枚快捷 / 最低客户端版本 /
        兼容性清单 / 包里有什么）、「查看 JSON」弹窗、「云端 preset 文件夹（模拟）」那一格
        与「上传到云端」；**卡片重新编号**（① 生成 / ② 发布 / ③ 对照基线 / ④ 交付残留 /
        ⑤ 回收站 / ⑥ 子目录职责）。② 发布只留**真发布物**：产物名单（`BuildRow.mkpFile`，
        即 `dist/mkp/presets/*.toml`）+ 查看 TOML（`wb_preview_toml`）+ 本次发布落了多少文件。
      - **探针**：`scripts/probes/workbench-build.mjs` 去掉包版本 / 最低客户端版本 /
        查看 JSON / 包里有什么 / 云端那一格 / 上传那六段。
      - **没做的**：`src/app/presets/presetTree.ts` 的「云端表」是**另一个概念**
        （catalog 登记面，不是模拟云端），不动；`src/app/store/package.ts` 只剩时间格式化，不动。
      - 验证：`tsc -b` / `eslint` / `stylelint` / `npm run build` / `build:workbench` /
        `check:bundle` / `check:zero-network` 全绿。

    - **增量之十九：第二刀「发布闸」—— `PublishAudit` 成为唯一入口判定器（2026-10-04，同一分支）**
      —— 规则源头 `docs/PUBLISH-ARCHITECTURE.md` §5；本文只记实现与判据。

      **核心口径**：发布闸不是"再做一遍零散检查"，而是**唯一入口判定器**：
      `【发布】→ PublishAudit → 十五项逐项打勾 → 全绿 → 生成 → commit → push → PR`，
      **任何一项 Blocker 红，绝不许进 commit / push / PR**。

      - **Rust 核心 `src-tauri/src/workbench/app/audit.rs`（新）**：
        `publish_audit() -> Result<PublishAudit, AppError>` 是**唯一判定函数**；十五项 = 十五个
        `AuditItem`，**加一项检查只加一个 item、不改流程**。分档 `Pass | Fail | Warn | Skipped` +
        分量 `Blocker | Warning`；`can_publish` ⟺ **没有任何 Blocker 是 Fail**（这条等式唯一的落点）。
        ★ `Skipped` 是刻意留的一档（⑫ `version/structure`）—— 把"没实现"伪装成"通过"比红色更危险。
      - **两个壳、同一件事**：命令 `wb_publish_audit`（`app/build.rs`，`(async)` 薄壳，进了只读命令登记）
        与 `cargo test` 判据调的是同一个 `publish_audit()`。**界面不许自己再实现一套检查**（老病根）。
      - **一次真自锁（已修）**：闸本身在 `with_ctx` 里，而 `with_ctx` 的锁**不可重入** ——
        ⑤ 最初直接调 `wb_generate_preview` 命令，等于自己把自己挂死。修法：预演拆出**锁无关内核**
        `build::preview_with(ctx, &Scope)`，命令壳与发布闸共用它（`wb_generate_preview` 收成三行）。
      - **只读到底**：新增 `paths::assets_root_path()`（不建目录 —— `assets_root()` 会 `create_dir_all`），
        `preview_with` 也改走 `dist_root_path()`：自称"不写盘"的闸不该顺手造目录。
      - **两项按数据实情纠正**：⑨ `no_phantoms` 只查**要交付**的那几条（`Image` / `MkPreset` 本来就不持
        path —— 原样会把 9 条 MKP 预设全判成幽灵）；② `refs_resolve` 补齐真在的引用面
        （机型三图位 `image` / `imageVariant` / `icon` + 各版本图 + 品牌 `logo`）。
      - **界面 `PublishGateModal`（新）**：点②的「发布」**先开闸**（不再直接发）—— 十五项逐项打勾、
        红的排最前、「去修」写在行里、「重新检查」可反复跑（只读）；底下「确认发布」只在 `canPublish`
        为真时亮。**没摆「创建 PR」**（第三刀才做，不摆点不动的假按钮）。
      - **前端只画不判**：`wb.publishAudit()` + `AuditItem` / `PublishAudit` 契约（`src/workbench/api.ts`）。
        浏览器桩 `wb_publish_audit` 跟着 `mockStrays` 走（清理残留 → 闸由红转绿），两项桩不互相打脸。
      - **判据**（`cargo test --features workbench --lib`）：
        `the_gate_lists_every_item_and_only_opens_when_no_blocker_fails`（形状 + 闸门 + `Skipped` 只许在⑫）、
        `publish_audit_all_blockers_pass`（**壳与核心是同一个判定** + `can_publish` ⟺ 无 Blocker 红）。
      - **探针**（`scripts/probes/workbench-build.mjs`）：点发布先开闸 → 十五项 → ⑫ 写「未实现」→
        有残留时「确认发布」不亮 → 清理残留后转亮 → 点确认闸关掉、② 卡记上；
        截图落 `tmp-shots/wb-publish-gate*.png`。
      - 验证：`cargo fmt` / 双 feature `clippy -- -D warnings` / `cargo test` + `--lib`（502 条）/
        `tsc -b` / `eslint` / `stylelint` / `npm run build` / `build:workbench` / `check:bundle` /
        `check:zero-network` 全绿。**预存红照旧**：探针「钉住」（按钮已删）+ 生成页产物名单条数那条。
      - **还没做的（第三刀）**：⑫ 结构签名 + `minVersion` 规则表；`wb_generate` / `wb_publish` 收进内部
        实现（对外只暴露一个【发布】）；commit / push / 建 PR / CI 状态回读。**合并仍留给人**。

    - **增量之二十：第三刀上半「结构签名 + minVersion 规则表」—— 发布闸不再有 Skipped（2026-10-04）**
      —— 规则源头 `docs/PUBLISH-ARCHITECTURE.md` §5.3（已按实现重写）；本文只记实现与判据。

      **核心口径**（作者定的两条）：① **不让程序猜"这次改动破不破坏兼容"** —— 那永远是产品判断；
      ② **机器算得出来的自动算，算不出来的必须有人签过字**。于是这件事拆成两半：

      ```
      ① 结构签名        —— 机器算（runtime/structure.rs，从类型真值探）
      ② 签名 → 最低版本 —— 人登记（presets/structure-signatures.toml）
      查不到 ② = Blocker Fail = 禁止发布（不猜）
      ```

      - **签名怎么算（★ 这块的巧处）**：签名只收「**必填**字段的模板路径 + JSON 形态」，
        而"必填"是**探出来**的 —— 拿真 catalog 的 JSON，删掉某字段再解析一遍：
        还成功 = 有 `#[serde(default)]`（可选），失败 = 必填。这条是**构造性**的（一份能解析
        成功的 JSON，它缺席的键必然可选），不是启发式。于是：
        **加可选字段 → 签名不变**（用户要的那条）；加必填字段 / 可选改必填 / 类型变 → 签名变。
        - 探针写**扫描式模板**（`machines[*].zones[*]`）而不是 `[0]`：免得"第 0 台机型没禁区"
          这种数据事实让探针落空；`every_probe_template_resolves` 再钉"每个模板都落到了东西上"。
        - 签名用**模板路径**（不是具体下标）—— 否则"前面插一台机型"就会让签名变、规则表被刷爆。
        - 判据 `the_signature_does_not_depend_on_which_sample_it_is_taken_from`：随包那份与
          仓库重建那份各算一遍，必须相等（钉死"签名与数据无关"）。
      - **机器看不出来的那半**：路径语义 / asset kind 语义 / 客户端读取方式 / 数据的解释方式 ——
        类型上一模一样，所以由 **`STRUCTURE_EPOCH`**（人显式 +1）记。本次那一刀（落点从
        `mkp/…` 改成发布根基准）就是它的第一个理由。判据 `the_epoch_is_part_of_the_signature`。
      - **规则表 `presets/structure-signatures.toml`（新，进仓库）**：`signature` / `minClient` / `note`；
        解析器在 `structure::RuleTable`（重复签名 / 空 minClient / 签名位数不对都报 `CORRUPTED`，
        **不许静默当空表**；文件不存在才当空表）。第一条登记 `cb1080919d39b2bd → 0.0.1`。
        ★ `minClient` **不要求那个版本已经发布** —— 那是 Dev 场景要的缝（§5.3）。
      - **⑫ 真跑（Blocker）**：`audit::structure_gate` —— ① 规则表有主吗 ② 本构建读得懂吗
        （签名 ∈ `SUPPORTED_SIGNATURES`）。**查不到就把该登记的那一行印在详情/`fixHint` 里**，
        `affectedFiles` 指到规则表。judged 三条：真仓库这一代 Pass 且报得出 minClient、
        没规则表时必须 Blocker Fail、闸门等价式不变。**十五项现在一项 Skipped 都没有**
        （那条 `skipped.is_empty()` 留着当"哪天有新项没实现"的报警器）。
      - **写进发布物**：`Catalog` 加 `structureSignature` + `minClientVersion`（都 `#[serde(default)]`，
        **不进 `revision_of`** —— 它们是结构的函数不是内容）。`finalize()` 从**自己**算签名
        （不是从随包那份 —— 那是上一代）。manifest 的 `minimumClient` 改成**同一格**做唯一来源
        （`PublishMeta.minimum_client` 删掉：两个来源必然漂）。`write_catalog_json` 也同一条路。
      - **顺手清掉的悬案**：`compat.minimum_client` 那条待办（"上游 manifest 的 minimumClient
        是空串"）随上游一起退役 —— 桩与 `BuildPage` 里那段特例都删了（现在每条 issue 都有「去处理」）。
        ② 卡的回执加一格「最低客户端 0.0.1」。
      - **判据**（`runtime::structure` 15 条 + `audit` 2 条）：Rust 默认 **300**、workbench lib **519**。
      - **重出 dist 元数据**：一条一次性集成测试驱动 `wb_publish()`（跑完即删，见台账纪律）——
        `presets/dist/{catalog,manifest}.json` 已同代（catalog 带签名、manifest `minimumClient`
        从空串变 `0.0.1`）。
      - **还没做（第三刀下半）**：客户端侧「先比再下」（`apply_remote_update` 拒读不懂的数据 +
        「有新版 SupportEase」提示的落点）—— 规则**核心**已经在 `structure::can_read`
        （能力优先、版本兜底）且判据齐；**UI 那句话住哪一格还没定**（不塞进现有的 `error`）。
        再往后才是 commit / push / 建 PR / 回读 CI。**合并仍留给人**。

    - **增量之二十一：第三刀下半「两条链」—— 软件更新（release.json）+ 数据读不懂（NOT_SUPPORTED）（2026-10-04）**
      —— 规则源头 `docs/PUBLISH-ARCHITECTURE.md` §5.3.1（新）与 §7；本文只记实现与判据。

      **作者三个裁决**（本节按它落的字）：① 软件更新信息源 = `release.json`，住发布根 `presets/`
      **之外**，**不进 catalog / manifest / 发布闸**，暂不接 GitHub Releases；② 不兼容预设走
      **甲案（目录级）**——只在 catalog 这层判，预设页**不主动宣传**（列表照常、不整表标红），
      只在真读不懂时出现那两句话 + 「去更新」；③ 新增第九档 `NOT_SUPPORTED`。**两条链不合并**。

      **链一 · 数据读不懂（先比再下）**
      - `runtime/update.rs`：`RemoteUpdate` 加 `readable`（`can_read(remote.structure_signature,
        remote.min_client_version)`）。★ 它与 `up_to_date` 答的是**两件不同的事**
        （"有没有新目录" vs "这一代我读不读得懂"），判据 `the_update_check_reports_readability`
        钉"指纹不同但读不懂"时两字段各自成立、不互相覆盖。
      - `ipc/catalog.rs::apply_remote_update`：**在 `release_bytes` 之前**先 `can_read`，
        读不懂 → `AppError::not_supported(...)`，**不落盘、不归档、本机目录零改动**
        （判据 `an_unreadable_remote_catalog_is_refused_not_applied`）。`check_remote_update`
        的 DTO 加 `readable`（原三字段不动）。
      - `error.rs`：加 `ErrorCode::NotSupported` + `AppError::not_supported`；message 只写用户能懂的话，
        **不含**签名 / minClient / schema（技术细节进 `detail`）。用例表加 `(NotSupported, "NOT_SUPPORTED")`，
        判据 `not_supported_is_its_own_error_code`（且 `≠ CORRUPTED / ≠ INTERNAL`）。
        `contract.ts` 的 `ErrorCode` 联合加 `'NOT_SUPPORTED'`（两份声明靠这条测试钉）。
      - 前端：`usePresetData` 新增独立状态 `needsNewerClient`（**不是 `error`**）——
        `checkBootstrapOnce` 先看 `readable`，为假或捕获到 `NOT_SUPPORTED` 时**不换目录、不落页级错误**，
        只把它立起来。预设页在状态条位置出现「**此预设需要更新版 SupportEase** / 当前客户端版本过旧，
        暂不支持此预设文件。」+「**去更新**」（`App` 给 `onOpenSettings` 出口）——**列表照常**。

      **链二 · 软件版本（独立）**
      - 仓库根新增 `release.json`（`{releaseSchema, version, notes, url}`）；`runtime/source.rs` 加
        `RELEASE_FILE` + `release_url(base_url)`（从文件下载根往**上恰好一级**，`…/presets/dist`
        → `…/presets/release.json`；host 根如实拒）。
      - `runtime/net.rs` 加 `get_release(url)`（复用 `get_bytes`，网络只住这一处）；
        `runtime/release_info.rs`（新）：`ReleaseInfo` + `parse`（坏 JSON / 代次认不出 / 空版本 → `CORRUPTED`，
        **不静默当"没更新"**）+ `compare`（复用 `structure::version_at_least`，版本比较只有一处实现）。
      - 命令 `get_app_version`（返回 `Cargo.toml` 同源版本号，前端拿"当前版本"的唯一口子）+
        `check_software_update`（只读、`async`、**只打开设置页时才调**，铁律 2：云端不参与首屏）；
        `lib.rs` 两个 `with_commands` 一字不差地都加了这两条。
      - 设置页 `PageSettings` 新增第一块「软件更新」：「有新版本 SupportEase」/「已是最新版本」/
        「这次没能查到更新」三态 + 版本对照 + 「查看更新」/「重新检查」；「高级设置」降为第二块。
      - 契约：`contract.ts` 加 `SoftwareUpdate` 类型 + `MkpApi.getAppVersion/checkSoftwareUpdate`；
        `bridge.ts` 两条；`mock.ts` 给演示值（含"有新版"）。

      - **判据**：Rust 默认 **311**（+11）、workbench lib **530**（+11）。新增
        `not_supported_is_its_own_error_code` / `the_update_check_reports_readability` /
        `release_json_is_its_own_source_not_preset_data` / `software_version_compares_by_semver_and_ignores_dev_suffix`
        / `a_missing_source_reports_up_to_date_with_the_real_version` / `release_url_is_one_level_above_the_file_root`
        / `broken_json_is_corrupted_not_silently_up_to_date` 等。
      - **验证**：`cargo fmt` / 双 feature `clippy -- -D warnings` / `cargo test` + `--lib` /
        `tsc -b` / `eslint` / `stylelint` / `npm run build` / `check:bundle` / `check:zero-network` 全绿。
      - **还没做（第三刀下半剩下的）**：`wb_generate` / `wb_publish` 收进内部实现（对外只暴露一个【发布】）；
        工作台 commit / push / 建 PR / 回读 CI。**合并仍留给人**。

    - **增量之二十二：第三刀下半「发布事务」—— 单一入口把发布做成一次事务（2026-10-04）**
      —— 规则源头 `docs/PUBLISH-ARCHITECTURE.md` §7.1（新）；本文只记实现与判据。

      **作者边界**（这刀的全部意义）：`【发布】` 是**唯一的用户动作** ——
      `wb_generate` / 「创建 PR」都**降为内部步骤**。摆出「生成 / 发布 / 创建 PR」三个按钮，
      就退化成"给开发者包了一层 CLI"，那不是桌面产品。**合并留给平台网页**。

      **一次「发布」的事务链**（`workbench/app/publish_tx.rs::run`，**只收 `&Ctx` 的锁无关内核**）：
      `PublishAudit`（任一 Blocker 红 → 停在审计、**零写入**）→ `build::generate_with`（生成降内部）
      → `dist::publish_into`（定稿）→ 本地 git（白名单 stage → commit → push）→ 平台 create_review。
      ★ `with_ctx` **不可重入**：链内只调自由函数，绝不回头调命令壳 ——
      判据 `the_transaction_chain_never_calls_a_command_shell`（源码扫描）钉住。

      - **本地 git**（`workbench/app/git.rs`，新）：子进程 `git`（参数显式数组，不拼 shell）；
        `status` / `diff_stat` / `stage_allowed` / `commit` / `push` / `has_staged` / `remote_url` / `branch`。
        ★ **只 stage 白名单**（`STAGE_ALLOWLIST` = `presets/dist/` + 规则表 + 台账），**永不 `git add -A`**
        （判据 `stage_paths_are_an_explicit_allowlist` + 真仓库的 `a_real_repo_only_commits_the_allowlisted_paths`）。
      - **平台出口**（`workbench/app/platform/{mod,github,gitee}.rs`，新）：`trait Hosting`
        （`create_review` / `get_review`）；GitHub 走 REST、Gitee 走 OpenAPI，**都复用既有 `ureq`**（不新增 HTTP crate）。
        **平台从 git remote 推断**（`detect_platform`）；**方言收敛**（`collapse_state` / `collapse_checks`）——
        GitHub 的 PR/check-runs 与 Gitee 的 MR/status 各自多档 → 统一 `ReviewState` / `ChecksSummary`，
        **前端不认识任何平台方言**。★ Gitee 按公开 API 实现；作者说后续给旧版配置地址再单独做事实核对
        （不翻旧代码猜）；要改只动 `gitee.rs` 一个文件。
        ★ 这是**新开的第二个被批准的网络出口**：`check:zero-network` 第①道闸放宽为
        "网络只住 `runtime/net.rs` **与** `workbench/app/platform/`"。
      - **凭据**（`workbench/app/credentials.rs`，新）：**每平台一份**，住**系统 Keychain**
        （新依赖 `keyring`，挂 workbench feature），**绝不**写 config.toml / localStorage / .env；
        前端只知道"配没配"+ 尾号提示（判据 `credentials_never_echo_the_token`）。
        trait `SecretStore`：真机 `KeychainStore`，判据 `MemoryStore`。
      - **命令面**（`lib.rs` workbench 分支）：`wb_publish(opts)` 改为**发布事务**（返回阶段快照
        `PublishTxReport`）；新增 `wb_publish_account`（平台推断 + 凭据有无，只读 async）/
        `wb_set_publish_token` / `wb_clear_publish_token`（只进不出）/ `wb_publish_status`（**手动回读**，
        不做后台轮询）。旧的"只定稿"壳 `publish_deliverable_only` 留着当可单测入口，**前端不再用**。
      - **前端收口**：`BuildPage` 的 `publish()` 只调一次 `wb.publish()`、② 卡记事务 `summary`；
        `SettingsPage` 新增「发布账户」块（平台推断显示 + 每平台存/清 Token + 尾号提示）；
        `api.ts` 补事务/状态/账户方法与类型；`mockBackend.ts` 补桩（走向成功那一路，含建 PR）。
      - **判据**：Rust 默认 **311**、workbench lib **552**（+22）。新增 `the_transaction_chain_never_calls_a_command_shell`
        / `stage_paths_are_an_explicit_allowlist` / `a_real_repo_only_commits_the_allowlisted_paths`
        / `detect_platform_maps_remote_urls` / `parse_owner_repo_handles_https_and_ssh`
        / `remote_state_collapses_platform_dialects` / `checks_summary_collapses_to_four_buckets`
        / `credentials_never_echo_the_token` / `each_platform_has_its_own_credential` 等。
      - **验证**：`cargo fmt` / 双 feature `clippy -- -D warnings` / `cargo test` + `--lib` / `tsc -b` /
        `eslint` / `stylelint` / `npm run build` / `build:workbench` / `check:bundle` / `check:zero-network` 全绿。
        工作台探针 `workbench-build.mjs` 全过（发布事务回执那一条已改成认"已建 PR"）；
        **已知预存红照旧**：生成页「× 3 份」那条 + 参数台「钉住」（按钮已删）。
      - **还没做**：CI 状态的后台轮询（作者明确要**快照 + 手动刷新**，不做轮询）；
        旧版发布账户配置 → 新版的事实核对（等作者给地址）。**合并仍留给人**。

    - **增量之二十三：发布账户补完整 + Git 认证自持（2026-10-04）**
      —— 规则源头 `docs/PUBLISH-ARCHITECTURE.md` §7.2（新）；本文只记实现与判据。

      **背景**：作者看过旧版 mkppanel（Go/Wails，`/Users/wzy/projects/mkpse-workspace/mkpse-next_v3/mkppanel`）
      后**不重裁**已定的三项（PR/MR 终点 · 每平台一份 Keychain · SupportEase 自持认证），
      只把**发布账户补完整**。旧版真正值得吸收的 = "凭据由工作台管理、支持 GitHub/Gitee、发布链是一个
      完整事务"；**Tag 晋升**（`git_promote.go` 的 dev→main squash + annotated tag）与
      **token 拼 URL**（`git_push.go::buildAuthURL`）**不继承**。

      **配置与秘密分离**：
      - 新 `workbench/app/account.rs`：`<appDataDir>/publish-account.json`，每平台
        `{repositoryUrl, username}`（**无 token、无 email**）；缺文件 = 空配置，坏 JSON = `CORRUPTED`
        （不静默当"没配"）。`check_platform_matches` 校验"GitHub 格里别填 gitee 地址"（认不出的自建源不硬拒）。
        判据 `publish_account_config_never_stores_a_token` / `config_roundtrips_and_a_missing_file_is_empty`
        / `a_future_schema_is_refused` / `a_mismatched_platform_is_refused`。
      - Token 仍住 Keychain（`credentials.rs` 不变），每平台一份。

      **Git 认证自持**（`git.rs`）：
      - 新 `pub fn push_authenticated(branch, username, token)`：
        `git -c credential.helper= -c http.extraHeader="Authorization: Basic <base64(user:token)>" push -u origin <branch>`。
        ★ Token **不进 remote URL**；★ `credential.helper=`（空值）**清掉全局 helper ⇒ 不读用户已存凭据**；
        ★ 环境 `GIT_TERMINAL_PROMPT=0`；★ 失败 detail **不带** header 参数。base64 **手写**（不引 crate）。
      - 新 `pub fn remote_matches(repository_url)` + `normalize_repo_url_for_compare`（https/ssh/.git/大小写归一）。
        判据 `git_push_authenticates_without_putting_the_token_in_the_url` / `base64_encodes_known_vectors`
        / `repo_url_normalization_makes_equivalent_addresses_match` / `remote_matches_compares_the_configured_repository`。

      **发布目标来自配置**（`publish_tx.rs` / `build.rs`）：
      - 新 `PublishTarget{platform, repository_url, username, token, owner, repo}` + `resolve_target(root, platform?)`
        （命令壳里解析：读配置 + Keychain + remote 校验）；`run()` 加 `target` / `repo_root` 参数，
        push 走 `push_authenticated`、建 PR 的 owner/repo 来自**配置**（不再 `parse_owner_repo(remote)`）。
      - `TxOptions` 加 `platform: Option<String>`（`None` = 自动挑：唯一配好的 / 或多个时取"与 remote 一致"的）。
      - 命令：`wb_publish(app, opts)`、`wb_publish_account(app)`、`wb_set_publish_account(app, platform, repositoryUrl, username)`、
        `wb_set_publish_token(platform, token)`、`wb_clear_publish_account(app, platform)`、`wb_publish_status(app, number)`
        —— **都收 `AppHandle`**（`<appDataDir>` 只有它拿得到）。`lib.rs` workbench 分支同步登记。
        `PublishAccount` DTO 改成 `{platforms:[{platform,repositoryUrl,username,hasToken,tokenHint}], remoteUrl, remoteMatchesConfig, branch}`。
        ★ 没配发布账户时 `resolve_publish` 返回 `None` —— 事务退化成"生成+定稿+本地推送"，**不报错**。

      **前端**：`SettingsPage` 发布账户块做成 **GitHub/Gitee 对称三格表单**（仓库地址 / 用户名 / Token），
      磁盘真值回显 + Token 尾号 + 必填校验（未填时保存不亮）+ 保存后清空 Token 输入框；`api.ts` 契约同步
      （`PlatformAccountView` / `PublishAccount` 新形状 + `setPublishAccount` / `clearPublishAccount`）；
      `mockBackend.ts` 桩同步；`c14.module.css` 加 `.vlabel`。

      - **判据**：Rust 默认 **311**、workbench lib **562**（+10）。
      - **验证**：`cargo fmt` / 双 feature `clippy -- -D warnings` / `cargo test` + `--lib` / `tsc -b` /
        `eslint`+`stylelint` / `npm run build` / `check:bundle` / `check:zero-network` 全绿。
        工作台探针 `workbench-build.mjs` 发布事务那条保持绿（**预存红照旧**：生成页「× 3 份」+ 参数台「钉住」）。
        agent-browser 实机验收设置页：**三格表单、回显、尾号、必填禁用、一致性提示 ✓** 全对
        （截图 `tmp-shots/publish-account-form.png`）。
      - ★ **取代一条旧原则**：上一轮"不为平台新增配置文件"作废 —— `publish-account.json` 是必要的
        机器本地发布配置（不进仓库、不含密钥），已写进 §7.2。
      - **还没做**：CI 状态后台轮询（作者要快照+手动刷新，不做）；旧版发布账户配置 → 新版事实核对（等作者给地址）。

    - **增量之二十四：发布闸 ⑬ 本地校验修复 + ⑮ 闸门修复 + 「发布预设 / 软件版本」语义澄清（2026-10-04）**
      —— 规则源头：`docs/RELEASE-TRANSACTIONS.md`（**新**，两层事务合同）+ `PUBLISH-ARCHITECTURE.md` §5 补注。

      **① 修 `source/correct`（⑬）本地校验**（作者真机踩到：合法的 `dist/source.json` 被判"解析不出来"）
      - 根因：`source_correct` 把**本地文件路径**喂给远端解析器 `parse_bootstrap`，它在 `baseUrl` 缺省时
        要 `directory_of(url)` 从 **http(s) URL** 回退目录 —— 本地路径不是 URL ⇒ 报错 ⇒ 闸红。
      - 修法：`runtime/source.rs` 加 `pub fn validate_bootstrap_local(bytes) -> Result<String>`（**不要求 URL**，
        返回 trim 后 catalog 相对路径）+ 私有 `check_catalog_rel`（catalog 相对路径合法性，**与 `parse_bootstrap` 共用一处**）；
        `source_correct` 改调它（**不再传本地路径给 `parse_bootstrap`**），失败文案用 `e.message`，通过时说"认得出来，指向 <rel>"。
      - 判据：`source_correct_accepts_a_local_bootstrap_without_a_url` / `a_broken_bootstrap_is_still_refused_locally`
        / `local_validation_matches_what_the_remote_parser_accepts_first`；既有 `bootstrap_*` 三条保持绿。

      **② 修 `can_publish` 漏判 ⑮ `git/clean`**（真 bug，实测 `can_publish=true` 而 `blockers=1`）
      - 根因：`publish_audit()` 里 `can_publish` 在 `items.push(git)` **之前**就算完了 ⇒ ⑮ 不参与闸门，
        脏工作区时闸仍亮「确认发布」，**绕过**「除交付产物外工作区必须干净」那条保护。
      - 修法：把 `let can_publish = ...` 整体移到 `items.push(git);` **之后**（一行搬家，不改任何检查函数）。
      - 结果：`can_publish` 与 `blockers()` 严格等价 —— 两条真仓库判据
        `publish_audit_all_blockers_pass` / `the_gate_lists_every_item_and_only_opens_when_no_blocker_fails`
        **由红转绿**（本轮 workbench lib 565 全绿）。

      **③ 「发布预设 / 软件版本」语义澄清**（作者定：工作台那颗按钮做的是"发预设"，不是"发软件版本"）
      - `BuildPage` ② 卡标题「② 发布」→「**② 发布预设**」，卡注改为"把本次预设与数据变更提交到远端仓库；
        合并后，客户端即可获取这些更新"；按钮仍叫「发布」（不动 `PublishGateModal` 与探针按钮断言）。
      - 新增**只读**「软件版本」块（当前已安装 <版本> / 尚未有新的软件版本 + 一句"预设更新不需要新安装包"）；
        **本轮不放「发布新版本」按钮**（点不动的假按钮比不摆更糟）。
      - 新命令 `wb_app_version`（`#[tauri::command(async)]`，返回 `structure::APP_VERSION`，与客户端 `get_app_version` 同源），
        `lib.rs` workbench 分支登记；`api.ts` 加 `appVersion()`、`mockBackend.ts` 补桩。
      - 探针 `workbench-build.mjs` 第 134 行断言 `'② 发布'` → `'② 发布预设'`（退役旧文案同步裁探针）。
      - **新文档 `docs/RELEASE-TRANSACTIONS.md`**：两层事务（发布预设 / 发布软件版本）、变更分类（数据类 vs 程序类）、
        小改动怎么办（Latest vs 正式 Release）、tag 何时打、`release.json` 位置、两仓关系、状态模型、
        「发布」按钮可点条件（唯一来源 = `can_publish`，含 ⑮）。

      - **判据**：Rust 默认 **314**、workbench lib **565**（两条真仓库判据由红转绿）。
      - **验证**：`cargo fmt` / 双 feature `clippy -- -D warnings` / `cargo test` + `--lib` / `tsc -b` /
        `eslint`+`stylelint` / `npm run build` / `check:bundle` / `check:zero-network` / `build:workbench` 全绿。
        探针 `workbench-build.mjs` ② 卡改名后**发布流程全过**（预存红照旧：生成页「× 3 份」）。
        agent-browser 实机验收：② 区显示「发布预设」+ 只读「软件版本」块 ✓（截图 `tmp-shots/publish-preset-vs-software-version.png`）。
      - **还没做**：「发布软件版本」整层（tag / Release / 上传安装包 / release.json 联动）—— **单独立刀**。

    - **增量之二十五：发布事务真机挂死修复（自锁 + 主线程 + 网络无超时，2026-10-04 作者实点）**

      **现场**：作者在工作台点「发布预设」→ 系统弹两次 Keychain 授权框 → 之后窗口整段挂死
      （「卡住了，我什么都没办法点」）。`sample` 采样 + 日志定位：**主线程**卡在
      `wb_publish → with_ctx → publish_tx::run → audit::publish_audit → with_ctx_mut`
      —— `with_ctx` 的锁不可重入，事务里回头调"会自己取锁"的 `publish_audit()` = **自锁**。
      仓库侧零改动（没提交 / 没推送 / 没写 dist），force-quit 即恢复。

      **两处根因 + 一处帮凶**：
      1. **自锁**（挂死的直接原因）：审计拆成**锁无关内核** `audit::audit_with(ctx)` + 入口薄壳
         `publish_audit() = with_ctx(audit_with)`（与 `build::preview_with` 同形）；
         `publish_tx::run` 改调 `audit_with(ctx)`。源码扫描判据
         `the_transaction_chain_never_calls_a_command_shell` 的禁名单**补上 `publish_audit(`**
         —— "会自己取锁的入口函数"与命令壳同罪。
      2. **主线程冻结**（"点什么都没反应"的原因）：`wb_publish` 当时是同步命令 ⇒ 跑在**主线程**上，
         读 Keychain / 起 git 子进程 / 发平台 HTTP 全挂在主线程。改 `#[tauri::command(async)]`；
         同类三条 Keychain 命令（`wb_set_publish_account` / `wb_set_publish_token` /
         `wb_clear_publish_account`）一并改异步。判据
         `read_commands_are_async_so_they_never_freeze_the_window` 新增 **IO 单子**
         （碰网络 / Keychain 的命令必须 async；扫描面加 `publish_tx.rs`）。
      3. **网络没有超时**（"就算不挂死也没有尽头"）：平台 HTTP 原用裸 `ureq::get/post`
         （默认无总超时）。`platform/mod.rs` 新增 `agent()`（`API_TIMEOUT = 30s` /
         `API_CONNECT_TIMEOUT = 10s`），GitHub / Gitee 两处改走它 —— 与 `runtime/net.rs` 同一条纪律。

      **规则源头**：`docs/PUBLISH-ARCHITECTURE.md` §7.1 新增第 4 条「锁与线程边界」。
      **验证**：`cargo fmt` / 双 feature clippy `-D warnings` / 默认 **314** + workbench lib **565** /
      `tsc -b` / lint / `build` / `check:bundle` / `check:zero-network` 全绿。
      **真机复跑（作者再点一次发布）是这条修复的最终验收**。

    - **增量之二十六：发布事务收尾（回执屏 / 发布历史 / 软件内合并 / Token 会话缓存）**

      背景：复跑成功（**PR #27** 开出，事务 6 秒返回），但作者点完发现三件事：
      "关掉模态框再打开又是新的"、"要去浏览器合并吗、浏览器没登录怎么办"、
      "还是要我输两次密码"。这一刀只做**收尾体验**，不碰软件版本发布那一层。

      1. **发布回执屏**：`PublishGateModal` 成功后**不再关框**，就地切成回执 —— 阶段链
         （发布检查 → 生成 → 提交 `08ec040` → 推送 → PR #27 → CI → 合并）+ **PR 地址可点**
         （新命令 `wb_open_external`，只放行 `http(s)`）+「刷新状态」（复用 `wb_publish_status`，
         **不轮询**）+【合并】。`PublishTxReport` 只增一个 `commit`（短 sha，取自 `git rev-parse`）。
         ② 卡多两颗按钮：「查看发布结果」（把上次那份回执**再打开**，不重跑十五项）与「发布历史」。
      2. **软件内合并**：`Hosting::merge_review`（GitHub / Gitee 同形 `PUT …/pulls/{n}/merge`，
         走带超时的 `platform::agent()`）+ 命令 `wb_merge_review`（`async`，进 IO 单子）。
         作者拍的规则：**一律 squash**、**不强制等 CI** —— CI 没跑完 / 已经红了都在二次确认里
         说清（`CI 尚未完成 —— 确定继续合并吗？`），合完**回读**真状态。合同 §1.1 第 8 步随之更新。
      3. **发布历史**：`app/history.rs` + `<appDataDir>/publish-history.json`（与发布账户同形的
         存储规矩：schema + atomic_write + 坏档 `CORRUPTED` 不静默；**不是配置**，删了只丢展示）。
         写入点在**壳层** `wb_publish` 收尾（内核不碰 `AppHandle`）；新命令 `wb_publish_history`。
         界面 `HistoryModal`：最新在前、每条一个「刷新」手动回读；**打开时读一次，不轮询**。
         合并成功时把新状态**写回**历史里那一条（`history::update_review`）。
      4. **Token 会话缓存**：`credentials::CachedStore` + `session()`（进程一份）—— 一次程序运行
         **至多读一次**系统钥匙串；`set` / `clear` 同步失效；**不改 Keychain 的存储方式**。
      5. **⑮ `git/clean` 的 CI 红**（作者裁决 B）：取不到 Git / 取不到分支（CI 的游离 HEAD 检出）
         从 `skip` 改 **`pass`** 并写明"该检查不适用" —— **保持「Blocker 不许 Skipped」原判据不变**。
         新判据 `a_gitless_workspace_marks_git_clean_as_not_applicable`；规则写进
         `RELEASE-TRANSACTIONS.md` §7（⑮ 的适用范围）。
      6. 探针 `workbench-build.mjs` 同步：发布段改成量回执 / 合并 / 历史（三条新截图落 `tmp-shots/`）；
         **裁掉读退役结构的「钉住」那一段**（那个把手只在收起态才叫「钉住」，点了会让探针挂住整段）。

      **验证**：fmt / 双 feature clippy / 默认 **314** + workbench lib **574** / `tsc -b` / lint /
      `build` / `check:bundle` / `check:zero-network` 全绿；探针实机走查 —— 回执 / 合并二次确认 /
      已合并 / 重开回执 / 历史三条 / 手动刷新 全过（仅剩预存红「生成之后产物名单没跟上」）。

      **补（交接）**：第四刀「发布软件版本」的施工计划另起一份 **`PUBLISH-KNIFE4-HANDOFF.md`**
      （本文件不重复它的内容）：八步现状与缺口、现成件清单（★ `scripts/release.mjs` 已管版本号四处 +
      PR + 等 CI + 合并 + tag；设置页三态已完）、**六条待作者拍板**、五步施工顺序。
      ★ 最硬的一条：**`release.json` 现在住仓库根，而客户端会去 `…/presets/release.json` 找**
      （`source::release_url`：base 往上恰好一级）—— 开工前先定落点。

      **补（作者真机踩到）**：合并之前再点一次「发布」，`create_review` 落 **HTTP 422**
      （平台不许同一个 head→base 开两份 PR）—— 而 commit + push 其实已经成功。
      这是**发布可重跑**缺的一块：`Hosting::find_open_review`（`GET …/pulls?state=open&head=o:branch&base=…`）
      + `run` 在建 PR 失败时**回读那一份开着的 PR** 继续（摘要如实写"回读那一份，没有重复建
      （平台原话：…）"），找不到才抛原错。合同 §1.1 第 6 步补上这条口径。

    ### 切页立刻显示 + 生成页放开选择（2026-10-02，作者点名）

     **起因**：作者「点击生成与发布这个页面，它很慢才显示出来……**所有页面都应该优先显示出来**，
     必须立马显示，就是那个反馈。等待的时候可以用骨架屏」；并「已生成过了他就不让选择了，是不对的。
     那个按钮也不能只是『待生成』，还得给我一个全选按钮」。

     **① 切页立刻显示（骨架屏）**
     - **真因**：`App.renderPage` 在 `!book || !words` 时**返回 `null`** —— 整本（`wb_book`）
       没回来之前，点导航**什么都不显示**（黑屏）。
     - **改法**：新增 `components/Skeleton.tsx`（+ `.module.css`）—— 按页给一具骨架
       （机型/参数/套餐/资产 = 两栏，生成/设置 = 一叠卡）；`renderPage` 的
       `!book || !words` 改成 `skeletonFor(id)`；`build`/`settings` 的 `!boot` 同样给骨架。
     - **② 卡不阻塞**：`BuildPage` 的 `collectInputs()`（6 条读 + 每台机型一条 `wb_desk`）
       本来就是后台 `useEffect`，页面壳早就画出；它的「包里有什么」在装的时候改摆**骨架条**
       （`.chipSkel`）而不是一个空 chip。
     - **判据**：探针用 CDP CPU 节流造"慢"，断言「整本回来之前先画骨架屏（不是黑屏）+ 导航已在」。

     **② 生成页：已生成也能勾 + 「全选」**
     - 后端的 `row.buildable` 只覆盖 `stale | neverBuilt`（那是「要不要进**默认**生成队列」的口径）；
       而 `planned_todos` 在 `Scope::Picked` 下**本来就收任何 uid**（内容没变走 `unchanged`、不重写）。
       → 所以「已生成不让勾」**只是前端用了 `buildable` 当勾选闸**，后端早就支持重生成。
     - **改法**：前端加 `pickable(r) = r.state !== 'noResources'`（只有"压根没配方"那种真不能生成），
       勾选框改用它；补 **「全选」**（勾所有能勾的，含已生成）与 **「全不选」**，
       与既有「全选待生成」并存（两颗问的是不同问题：一个"哪些还没生成"、一个"全部"）。
     - 作者口径补充：「**就算它没有变化，我也可以重新去生成一次，反正我就是想看到那个模态框**」。
     - **判据**：探针断言「已生成的行也能勾（0 个被禁）」「「全选」勾上全部 N/N 行」。

     ### 性能修复：`ParamRegistry::fingerprint()` 缓存（2026-10-02）

     **症状**（作者）：「点击了生成与发布的页面，还是像一瞬间被冻结住了，过了好一会才有反应」。

     **实测定位**（临时 `perf_probe` test，拿**真 `presets/` + 真 `workbench/` store** 量，
     不是小夹具 —— 夹具量不出问题）：

     | 操作 | 修前 | 修后 |
     | --- | --- | --- |
     | `book_view()` | **321 ms** | **14 ms** |
     | `build_rows()` | **79 ms** | **12 ms** |
     | `registry.fingerprint()` ×27 | 230 ms | 0 ms |

     **根因**：`presetdata/resolve.rs` 的 `Layers::fingerprint()` 里每次都调
     `self.registry.fingerprint()` —— 它**序列化 74 条定义 + 整本布局再 SHA256**（8.5ms/次）。
     而 `book_view` / `build_rows` 按版本反复取它（每版 3 次 × 9 版 = **27 次**）。
     这个值**整份会话对所有机型所有版本都一样**，重算纯浪费。

     **修法（作者拍板：最小改动、不扩大战线）**：`ParamRegistry` 加
     `fingerprint_cache: std::sync::OnceLock<String>`，`fingerprint()` 改成
     `get_or_init(..).clone()` —— 仍是 `&self`、**返回值逐字节不变**，只是不再重算。
     **不动 `Layers` / `build_state` / `book_view` 的业务逻辑**；**不动 `effective_recipe()`**
     （剩下那 ~10ms 留着，若真机仍卡再单独拆）。

     **判据**：新增 `presetdata::registry::tests::fingerprint_is_cached_but_per_registry`
     （同一实例取 1000 次同值 / 同数据另一实例同值 / 重载后数据变了跟着变）。

     **收尾**：临时 `perf_probe` 已删（测试数回到 264 + 460），`workbench/built.json`
     （测量时 `Ctx::open` 写的）已清。

     ### 根本修复：只读命令改异步（2026-10-02，作者点名"不要修补补"）

     上一刀（缓存注册表指纹）只把 `book_view` 从 321ms 降到 14ms，作者**仍然觉得"顿一下"**，
     并给出了正确方向：「**能不能让这个页面显示出来再说？像那些游戏，优先显示了再改后面的。**」

     **真正的根因**（Tauri 官方文档原文）：**不带 `async` 的命令在主线程上执行，
     除非写成 `#[tauri::command(async)]`** —— 而主线程就是 webview 渲染那条线程。
     所以后端同步命令算多久，**整个界面就冻多久**：骨架屏画不出来、导航点不动。

     > 这不是"算得快不快"的问题，是"**在哪条线程上算**"的问题。
     > 算 14ms 还是 320ms 都是同一种病，只是轻重不同。缓存治标，这条治本。

     **改法**：**21 条只读命令**加 `#[tauri::command(async)]`（函数体照旧同步、无 await，
     所以 `traced` 那个 `!Send` 的 span guard 不受影响；`with_ctx` 的 `std::sync::Mutex`
     在 worker 线程上同步拿锁，不阻塞主线程、无跨 await 持锁 → 不会死锁）。
     - **改 async 的 21 条读命令**：`wb_boot` `wb_words` `wb_book` `wb_registry` `wb_matrix`
       `wb_desk` `wb_trash` `wb_ui` `wb_preview_bulk` `wb_diff_draft` `wb_preflight`
       `wb_preview_toml` `wb_generate_preview` `wb_revert_preview` `wb_dist_strays`
       `wb_baseline_diff` `wb_assets` `wb_asset_usage` `wb_bundles` `wb_machines` `wb_version_orphans`
     - **故意不动的 20 条写命令**（`wb_apply_draft` / `wb_save` / `wb_generate` / `wb_publish` …）：
       要落盘、要和草稿的锁配合，改异步是另一件要单独评估的事。
     - 前端**一个字没改**（本来就是 `await`）；上轮的骨架屏现在才真正生效 —— 主线程空出来了。

     **判据**：`workbench::app::tests::read_commands_are_async_so_they_never_freeze_the_window`
     —— 源码扫描，那 21 条读命令有一条没写 `(async)` 就红（已实测反向验证：把 `wb_book`
     改回同步，这条立刻报 `["wb_book"]`）。

     ### 整理那一刀（**排在上述两块之后**，先记着别现在做）

     - **死文件 / 死代码**：`jsonKey`/`mergeGroup`/`subfieldsOrder` 这套「共享 tomlKey = 内联表」
       机制（PR #20 后已无人使用）；`workbench-build.mjs:225` 引用不存在的「钉住」按钮（预存红）。
     - **半完成要点**（PROJECT-AUDIT 已列，别当没做）：切片器「复制」真机未接、
       `open_model` 只记日志、首页后处理命令是硬编码开发路径、校准无测试/历史、
       坏档在界面不可见、无启动对账/清理。
     - **文档收敛**：迭代久了 HANDOFF 856 行 + 多份 docs，需要一次"读之前先读什么"的梳理
       （作者："第一时间会读到什么，得整理清楚"）。
     - **旧世界清理**（PROJECT-AUDIT ⑩）：`~/Documents/MKPSupportSSR` 那类历史路径残留。

     ## 4. 续做入口（从哪接手）

- **总盘点在 `docs/PROJECT-AUDIT.md`**（2026-10-02）：把 HANDOFF / 总纲 / 工程约束 /
  产品规则 / 现有页面对了一遍，列成**已完成 / 半完成 / 未开始 / 已废弃**四类
  （按作者 ①–⑩ 排：备份恢复 / 设置页 / Preset 产品能力 / 机器型号 / 校准测试 /
  加工记录 / 状态收尾 / 云交付验收 / 跨平台 / 旧世界清理），外加**文档自身的欠账**
  与建议次序。**接手前先读它** —— 它说的"半完成"是"差哪一半"，不是"没做"。
- **清扫批次完成（2026-10-02，⑩ 的一小刀，只清残留不动功能）**：
  - **死 API 两处清除**：`get_preset`（首圈的硬编码表 `preset_of` —— Rust 命令 + 两份注册清单 +
    假后端夹具 + 测试一起删；首页/校准页早已走文件体系）与 `getAppliedPreset`
    （契约方法 + 假后端演示集合 + bridge 的 `notWired`；`AppliedPreset` 类型一并删）。
    真机 `notWired` 只剩 `copyToSlicer` / `downloadFiles` 两条（那两条**是有消费者的未接**，
    不是残留）。bridge 头注释同步改写。
  - **两个探针重写（同场病：C4 收口时一起烂的）**：
    - `chain.mjs`（P6 联动）：客户端那半（同步→下载→应用）在浏览器里**结构性走不通**
      （C4 底账进 Internal 根 + 真数据源；mock 没有盘没有源）——恒 FAIL 的死键断言
      （`mkp.a40.package` / `mkp.a40.active`）与旧链换成真行为：**工作台生成 → 上传
      （整份 release）→ 刷新那一格还在 → 客户端同步页如实说"catalog 随包走"**；
      两档 10/10 绿。"全选待生成"超时**复现不了**（按文件头三步构建即可跑通；
      A41 那次大概是拿没有桩的产物跑的）。客户端那半条链的覆盖在
      `params-sync.mjs` / `presets.mjs` + 真机验收（⑧ 的清单）。
      （2026-10-02 晚：「同步」页退役 —— `chain.mjs` 第 ③ 步换到设置页；
      `params-sync.mjs` → `params-settings.mjs`。**收口一层要回头把相关探针跑一遍**，
      这条又应验一次。）
    - `params-sync.mjs`（P3/P4）：走的老流程（参数页空态 → 去同步页获取一份 → 互跳）
      也没了 —— 参数页现在**直接读 catalog**、同步页不再有"自动同步 / 去看参数页"。
      重写成两页各自的现状（参数页照 catalog 画 / 同步页口径与账），两档 12/12 绿。
  - **文档刷新**：README（现状段 + 文档地图）· `ARCHITECTURE.md` §1/§3/§4/§6/§9/§10.5 ·
    总纲 §4 欠账 #5 收口（`mkp/` 命名）、#4 记为**半收口**（catalog 已统一登记 9 份，
    但没有"内置"标记）。
  - **产品规则反写**：`PRESET-PRODUCT-RULES.md` 19 节正文按**已落地行为**写齐
    （含 §17 留一条"启动对账要不要做"待作者定）—— **待作者逐节核准**，核准记录记在文末。
  - **schema 裁决**：`AssetKind::Image` **保留 + 登记**（复审依据写进 `presetdata/assets.rs`）。
- **十三收尾走完，用户文件这一整套正式收口**（2026-10-02）：官方线（下载 / 状态 / 批量 /
  归档 / 可信度）与用户线（能读能列 / 能另存 / **能被应用** / **能被改并写回自己** /
  **读不出来就拦在应用·编辑门口（文件级检查，不比 SHA）** / **能改名、能删、能另存为一份
  新的**）都通了，**导入入口**（外部 → 我的文件）与**外部管理**（Finder 里显示）也立起来了。
  整套生命周期：**官方 → 我的文件 → 编辑 / 另存 / 改名 / 删除 / 应用 / 导入 → Finder 管理**。
  **「分享」不做、「单文件导出」暂缓**（理由与将来出口见 §5）；**下一个真块是
  「设置 → 备份与恢复」**（ZIP 备份 / 恢复）—— 到那时复用第十二层那套通用导入入口，
  不需要再重造接收机制。
- **第九层登记一条边界（不是漏做）**：**语义合法性**（"是不是一份合法 MKP Preset"：
  结构 / 参数对不对）**客户端不判** —— 判据只到文件级（能读 + UTF-8 + TOML 语法）。
  含义：TOML 能读但不是 MKP 预设的文件（以及空文件 / 只有注释的 TOML）现在会显示为正常；
  要堵它得让真正的 Preset 能力接到应用 / 编辑入口上（`mkpse-preset` 的事，默认构建的
  隔离纪律不为它破例）。**"用户文件被外面改过"也不再是任何形式的报警**：只要仍是能读的
  TOML 就照常能用 —— 与之对照的官方线 SHA 报警（第六层）一个字没动。
- **第十层登记两条边界（不是漏做）**：① **用户文件没有"归档"** —— 用户删自己的文件
  就是真删除（官方线的 `archive/` 是版本更新历史，不给用户搞第二套）；② **改名只换名字、
  不换目录** —— 跨文件夹搬动属于"文件夹管理"，将来真要做单独设计。
- **第十一层登记边界（不是漏做）**：① 复制**不设内容门槛**（连读不出来的 / 非 TOML 的
  都能复制，复制出来状态照实）—— 与改名 / 删除同族的纯文件操作；② **不做"复制后自动改名"**
  —— 目标存在就拒、让用户自己换名字（作者定死）；③ 跨文件夹复制也不做（同"文件夹管理"）。
- **第十二层登记边界（不是漏做）**：① 导入**不校验内容**（TOML 语法坏 / 二进制都收 ——
  "能不能用"归 Preset 语义入口，判据：导入不是安装 Preset）；② **ZIP / 备份包现在不处理**
  （注册表留了认领的口子；设置页「备份与恢复」到那一层再注册导入器）；③ `.json` 现在也
  不收（要收就在注册表加一条）。
- **第七层留下的一条可收口项**（不影响功能，登记）：血统三行的文本逻辑现在两份实现
  （`runtime::lineage` 与 `preset::lineage`），靠 `workbench::lineage_parity` 钉住。
  更彻底的一条路是把它下移到共享小 crate、两边转调。

- **第三圈第 2 步的布局对齐已完成**（2026-10-02）：交付根相对路径 = 客户端下载区的相对路径（`mkp/…`），两端共用 `runtime::catalog::dest_of_asset`。**下一步是 §3.5 第 3 步：Preset 成为第一个完整消费者**（发现 → 下载 → 本地文件 → 页面，再逐层加下载状态 / 更新 / 修改 / 归档 / SHA 异常 / 用户版本）。**官方源 / Gitee 的真实地址**落进发布流水线（`MKPSE_PRESET_SOURCE`）是产品决定，等那一刀做。
- **整机图那一刀的遗留（已登记，不是漏做）**：`presetdata::AssetKind::Image` 变体与机型 `image` 字段还在（值为空）。**schema 暂不清理**（2026-10-02 定）："现在没有数据"不等于"这个概念永远不存在"，保持 schema 稳定，将来确认不用了再单独做一次 schema 清理（会连带改前端契约 `Machine.image`、mock、工作台机型页与资产页）。工作台的"机型图"筛选页签与机型图下拉现在**如实为空**（台账里确实没有这一类），不是坏了。界面素材已全部搬离 `public/`（整机图 + 测试模型合影），`public/` 只剩载荷根与 BBS 页元数据。**2026-10-02 清扫批次复审：保留 + 登记（去留已明确）** —— 确认它不是纯未使用枚举（`runtime::catalog` 的"不登记"分支与判据都在引用），也不是旧残留（剥离是改判）；删除属另案 schema 清理。理由写进 `presetdata/assets.rs` 的 `AssetKind` 头注释。
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
- **不许给第九层复制 Preset schema**（2026-10-02 作者定死，写进总纲 §1③）：客户端对
  用户文件只做**文件级**检查（能读 + UTF-8 + TOML 语法），**不复制 `mkpse-preset` 的
  结构 / 参数规则、不建第二套 Preset 真相**；语义合法性留给真正的 Preset 能力在
  应用 / 编辑入口上回答。"以后 schema 改一次要改两边"的平行真相，正是这次重建一直在
  消灭的东西。
- **导入入口是通用的，别在页面里造专属拖拽**（第十二层，作者定死）：接收机制（拖拽 /
  文件选择器 / 重名改名 / 边界检查）住在 App 层 `FileImportProvider` 与 `runtime::import`；
  页面只消费 `pickFiles` 与 `revision`。以后「备份与恢复」复用同一套，不另写一份。
- **不做"分享"、不做单文件"导出"**（第十三层，2026-10-02 作者裁决）：**「我的文件」本来
  就是真文件**（`Documents/SupportEase/presets-mine/`），再造一套分享 / 导出就是把复制 /
  导出逻辑写第二遍；"拿出去"由「在 Finder 中显示」解决。将来真要做导出，**是
  「设置 → 备份与恢复 → 导出备份 ZIP」**（把多种运行数据打包，有产品意义），不是复制一个
  TOML —— 到那时复用第十二层的通用入口。**若哪天仍要"单文件导出"，定义照作者给的**：
  把我的文件复制到用户指定的位置 —— 原文件不动、不改 Active / 草稿 / archive、同名进
  改名流程（不覆盖）。
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
- **真机下载要先配数据源**：C4 之后的下载不再探测仓库路径 —— 没配数据源时，下载 / 检查更新都会拒绝并说明去哪儿配。**这是对的**，不是 bug：用户机器上本来就没有仓库。开发期要验真，把本地静态目录（如 `python3 -m http.server`）的地址填进 **设置 → 高级设置 → 预设数据源**（「同步」页 2026-10-02 退役后那一格搬到了这里）；正式构建由 `MKPSE_PRESET_SOURCE` 编入默认值（Bootstrap 那一刀把它接到工作台）。
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
| `ipc::presets` 判据 | **DTO 构建只吃 catalog（判据 4 的钉子）** / 缓存按字节配对与自失效 / **`deprecated_flags_travel_through_definition_channel_only`（① 的钉子：definition 通道带 7 条字段级 / 配方通道**0 条**（排除语义没被偷改）/ 选项级 1 条「护套」走配方通道）** |
| `runtime::catalog::tests::plates_ride_along_and_machine_refs_resolve`（③ 的钉子） | 板随 catalog 下发（2 块去重：单卡舌 256 / 双卡舌 180，几何 + frame 一格不少）/ 机型引用可解析 / 默认板在自己 `plateIds` 里 / **指纹跟着板定义走**（改几何 revision 就变） |
| `presetdata::tests::every_machine_plate_ref_points_at_a_real_plate`（③ 的钉子） | 真数据里 5 台机型各引 1 块板、目录恰好 2 块（去重生效）；`check_plate_refs` 把悬空引用 / 默认板越界升级成 error |
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
| `runtime::mine` 写回 5 条（第八层） | `saving_back_writes_the_same_file_and_keeps_the_lineage`（**核心不变式**：**同一路径、不产生第二份**，血统照抄 —— 摘要不许变成"我自己改过的字节"）/ `saving_back_an_untouched_file_changes_nothing`（没改就逐字节不变）/ `saving_back_a_file_without_lineage_invents_none`（本来没血统就不编一个）/ `saving_back_refuses_when_the_file_is_gone`（被移走了**拒绝且不新建**）/ `saving_back_stays_in_the_mine_dir` |
| `runtime::mine` 第九层 5 条（文件级：能读 + UTF-8 + TOML 语法） | `a_broken_toml_shows_up_as_unreadable`（坏 TOML → `unreadable`，原因**报得出第几行**）/ `a_non_utf8_file_is_unreadable_too` / `an_escaping_symlink_is_never_read`（**指向用户根外的链接一个字节都不读**（血统也不读）；根内链接照常能用）/ `read_preset_text_blocks_the_broken_one_but_viewing_still_works`（应用 · 编辑两个入口共用这一处闸；**看正文照旧** —— 读它不算"用"）/ `an_external_edit_that_still_parses_is_fine`（**不比 SHA**：外部改过但仍是能读的 TOML = 正常）。既有 `mine_files_lists…` / `the_committed_copy_shows_up_in_mine_files` 搭上 `state` 断言 |
| `runtime::mine` 第十层 9 条（改名 / 删除） | `renaming_keeps_the_bytes_and_the_lineage`（**核心不变式**：**字节一个不动**、血统原样、旧名字没了新名字在；子目录里的份留在子目录）/ `renaming_refuses_a_name_that_changes_the_kind_or_has_a_path`（换后缀 / 带路径 / `.` / `..` / 空全拒，原文件原地不动）/ `renaming_never_overwrites`（落点有东西就拒，被撞的那份一个字节没动）/ `renaming_to_the_same_name_is_a_no_op` / `deleting_really_deletes`（**真删除**：不在了，而且没有多出旁的东西）/ `deleting_refuses_while_it_is_the_active_one`（正在使用的拒；正在使用的是**别人**照删）/ `deleting_refuses_while_a_draft_is_open`（有草稿的拒；草稿改的是**别人**照删）/ `the_delete_gates_only_apply_to_the_mine_line`（官方线的指针 / 草稿挡不住）/ `deleting_stays_in_the_mine_dir`（越界路径拒） |
| `runtime::state` 跟改名 3 条（第十层） | `the_active_pointer_follows_a_rename`（路径与文件名换新、**指纹原样**；不指着它的、官方线的指针一概不动）/ `the_draft_follows_a_rename`（草稿路径与来源名换新，**正文与打开时指纹原样**；换完再点「改这份」还是接上这一份）/ `repointing_nothing_is_a_no_op`（没有那两本账 = 没跟，不是错误） |
| `runtime::mine` 第十一层 6 条（另存为一份新的） | `copying_keeps_the_bytes_and_the_lineage`（**核心不变式**：新的一份与原来**逐字节相同**（血统原样带过去）、原文件一个字节不动；子目录里的份复制出来还在子目录）/ `copying_uses_the_same_name_gate`（与改名同一套：换后缀 / 带路径 / `.` / `..` / 空全拒）/ `copying_never_overwrites_and_never_renames_itself`（**不覆盖**（被撞的与源都一个字节没动）、**也不自动改名**（名字和原来一样也拒））/ `copying_touches_no_state_at_all`（**一个状态都不碰**：使用中指针与草稿都还在原来那份上）/ `copying_does_not_require_readable_text`（不读内容：二进制那份照样原样复制）/ `copying_stays_in_the_mine_dir`（越界路径拒） |
| `runtime::import` 第十二层 8 条（通用导入入口） | `staging_says_ready_collision_and_rejected`（能收 / 重名 / 收不了各带原因；**同一批里重名的也认**）/ `importing_copies_the_bytes_and_leaves_the_source_alone`（**核心不变式**：字节复制、**源文件一个字节不动**；进来就是一份正常用户文件）/ `importing_does_not_invent_a_lineage`（没有血统**不编造**）/ `importing_does_not_reject_invalid_toml`（**不校验内容**：坏的照样进得来，第九层再如实说它读不出来）/ `importing_never_overwrites`（不覆盖，被撞的与源都一个字节没动）/ `importing_uses_the_same_name_gate_for_a_new_name`（重名改名走同一套门槛）/ `importing_touches_no_state`（使用中指针与草稿都不动）/ `importing_creates_the_mine_dir_when_missing`（全新安装也能进） |
| `runtime::mine` 第十三层 3 条（在文件管理器里显示） | `revealing_resolves_the_file_inside_the_user_root`（**核心不变式**：解析出用户根里那个**绝对路径**；**读不出来的也能显示**（文件管理同族）；子目录里的也认）/ `revealing_stays_in_the_mine_dir`（`exports/`、`../` 一概拒 —— 与改名 / 删除同一道闸）/ `revealing_a_missing_file_says_so`（外面删了就说找不到，不去猜） |
| `runtime::state` 草稿两条线 2 条（第八层） | `a_draft_knows_which_line_it_belongs_to`（**同名不同线 ≠ 同一份**：「接着改」不许接错）/ `an_older_draft_file_is_still_the_official_line`（旧档 = 官方线，**不升 schema**） |
| `runtime::lineage` 写回 4 条（第八层） | `saving_back_keeps_the_original_lineage` / `saving_an_untouched_copy_changes_nothing` / `saving_back_without_lineage_writes_no_lineage` / `saving_back_keeps_the_line_endings` |
| `runtime::delivery` 可信度 5 条（第 6 层） | `a_clean_copy_is_current_and_not_reported`（没问题就不报警）/ `tampered_bytes_are_not_recognized`（哪儿都对不上 = 异常，且没有证据可指）/ `a_copy_of_the_archived_version_is_recognized_as_old`（**认得出旧版本**，证据是归档里那条路径）/ `a_version_from_the_archived_catalog_is_recognized_as_old`（归档槽被占了也认得出更早那一版）/ `official_text_refuses_bytes_that_drifted`（**不许拿漂了的字节当原文改**） |
| `runtime::state` 草稿 4 条 | `draft_roundtrips_and_clears`（存/读/丢，丢是幂等）/ `absent_draft_is_none_not_error` / `corrupted_draft_is_an_error`（坏档不静默）/ `draft_and_active_are_separate_files`（**改一份 ≠ 在用它**：两个状态文件互不干扰） |
| `runtime::lineage` 7 条（第七层，客户端那份血统） | `the_copy_is_the_source_plus_three_lines`（**副本 = 来源 + 三行**，剪掉逐字节相同）/ `copying_a_copy_replaces_the_old_lineage`（不叠加）/ `crlf_source_keeps_crlf` / `a_source_without_release_time_gets_two_lines`（不知道就别写）/ `lineage_roundtrips_and_absence_is_none`（没有血统 = `None`，不是空壳）/ `a_half_lineage_is_still_a_lineage` / `a_lookalike_key_is_not_matched`（`# based_on_extra` 不许被当成 `based_on`） |
| `workbench::lineage_parity` 3 条（**两端一致性**，只在 workbench feature 下编） | 客户端与工作台对 9 份入库产物给出**逐字节相同**的副本 / 读出血统三项相同（含"官方原件没有血统"两边都是 `None`）/ 摘要算法相同 —— 两份实现之间没有编译器，靠这三条钉住 |
| `workbench::app::build` 生成前预演 7 条（2026-10-02） | **`preview_never_touches_the_disk`（核心不变式：预演一个字节都不写 —— 新增的那份不被创建、已有的原字节不动）** / `a_new_file_shows_every_line_as_added`（新增全绿）/ `an_identical_file_has_no_diff_lines` / `a_changed_line_shows_one_removed_and_one_added`（删在前增在后，与 git 同序）/ `an_inserted_line_shows_only_an_added` / `a_deleted_line_shows_only_a_removed` / `the_counts_match_the_line_kinds`（新增/修改/无变化三档 + `+N −N` 计数） |
| `scripts/probes/workbench-build.mjs`（**手工**，非 CI） | 生成与发布页（① 生成 / ② 数据包 / 查看 JSON / 查看 TOML / 版本轴 / 云端那一格）· **切页立刻显示（2026-10-02）：整本没回来时先画骨架屏（不是黑屏）+ 导航已在** · **生成页放开选择：已生成的行也能勾（0 个被禁）+「全选」勾满 N/N** · **生成前确认：点「生成」先弹 diff 确认框（不是直接覆盖）· 确认**之前**不写盘 · 确认之后框内换结果页 · 确认之后产物名单才跟上** · 参数台增量。**注意**：`button[title*="钉住"]` 那一处是**预存红**（按钮已删，见 §3.5 整理那一刀）—— 不是本文改动引入的 |
| `runtime::mine` 血统 3 条 | `the_committed_copy_shows_up_in_mine_files` 里带上血统 / `the_official_update_shows_up_as_based_on_an_old_version`（**官方换版 → `outdated`**，且说得出机型）/ `unknown_when_the_source_cannot_be_resolved`（没有血统 / 没记摘要 / 来源已不在目录里）/ `lineage_is_read_from_the_head_only`（**只看头 8 KB**，用户目录里可能有几百 MB 的文件）/ `an_unreadable_head_is_just_no_lineage` |
| `runtime::state` 两条线 5 条（第七层） | `the_official_pointer_resolves_through_the_catalog`（**落点由目录给**，`mkp/presets/…` 真布局下 `intact` 才是 true；目录里没有了 ⇒ 漂了）/ `an_older_pointer_file_still_means_the_official_line`（**旧档不迁移**）/ `the_users_own_copy_can_be_the_active_one`（用户线按用户根解析；用户再改它算"漂了"）/ `a_pointer_pointing_outside_the_mine_dir_is_not_resolved` / `applying_one_line_replaces_the_other`（唯一性） |
| `scripts/probes/params-settings.mjs`（**手工**，非 CI） | 参数页 + 设置页：参数页照目录画（分类 / 卡片 / 行 / 底栏）· **① 弃用字段显示但只读（摊开的弃用行有「已弃用」徽章 + 控件全 disabled）** · **④ 塔地图按默认板画出板轮廓并替下 X/Y 行（svg「塔」+ 板 evenodd 路径 + 槽位含坐标行）** · 设置页「高级设置 → 预设数据源」全流程（当前状态如实 / 手动指定应用（尾斜杠砍掉）/ 非法地址如实拒 / 恢复内置默认能撤回）；两档尺寸 0 console error / 0 个 ≥400。**原 `params-sync.mjs`** ——「同步」页退役那一刀改名重写 |
| `scripts/probes/machines-dims-zones.mjs`（**手工**，非 CI） | 机型与版本页的**品牌树 + 尺寸六组 + 禁区编辑器**（2026-10-03 增量之十三）：品牌是父节点（可点收、折叠后机型行消失、带台数）· 机型缩进当子节点且**确实嵌在品牌分组里** · 筛选同时管两边 · 新增品牌 → 树里多一棵 · 机型行右键「移到品牌…」→ 选一家 → **原来那家台数 −1、新家 +1** · 尺寸卡六组读数与源文件**逐格一致**（床身 260×255 / 标定点 68.21 与 126.373 / G-code 标记）· 尺寸模态框 43 格数字、改床身宽保存后**卡上跟着变** · 禁区编辑器 `viewBox` 就是**床身尺寸**（不是可打印区）、两块多边形 **6 + 4 点**、`(0,0)` 落在**画布左下角**（Y 翻转的定点）、右侧列表点数 · 清到 0 块保存 → 那一格变「没有禁区文件」。跑法见文件头（同 asset-preview 三步） |
| `scripts/probes/brands-bundles.mjs`（**手工**，非 CI） | 机型与版本页的**品牌** + 套餐页可读性 + 状态栏（2026-10-03 增量之十二）：左列有品牌段（显示名 / id / 台数）· 点品牌开品牌卡（显示名可改且**改了跟着变**、品牌图那枚 `<img>` **真解码**、列出「这个品牌下的机型」）· 新增品牌 → 左列多一行且右侧切过去、**撞名（只差大小写）当场被拒** · 机型卡「品牌」格显示的是**显示名**且「看品牌」跳得过去 · 套餐左列两行读法（显示名不被省略、下行是 id + 装了 N 个文件）· 卡头大标题 = 显示名、附注 = id · 「选择版本…」是**树状**（机型组头 + 缩进版本行）且**行没有垫背景色** · 待确认 chip 里 uid 与「它现在指着谁」有间距（不粘连）· 状态栏只摆 `仓库名/目录名`、导航缩到 150px 也不越界。跑法见文件头（同 asset-preview 三步） |
| `scripts/probes/presets.mjs`（**手工**，非 CI） | 预设页探针：两轴可点 / 四张表可读 / 点行展开 / 右键菜单 / BBS 入口跨页 / **交付行的四态与动作（已下载·灰字、旧版本·「更新」、内容异常·「重新下载」）** / **存疑那两档没有「应用」也没有「改这份」、右键「另存为一份新的」带原因灰掉** / **批量那一层（批次行只含未下载+旧版本+内容异常、逐份结局各占一行且不许伪装成功）** / **我那份能被应用并说得出「基于旧版官方」** / **改我那份 → 保存回它自己（不产生第二份、血统还在）** / **第九层：读不出来的那份照常列在表里、画得出「文件无法读取」（角标带原因）、没有「应用」也没有「改这份」，能读的那份不受牵连** / **第十层：改名只动名字（坏的那份改完还是「文件无法读取」）、删除有二次确认且删完行没了、正在使用的那份「删除」灰掉带原因、改名不断「已应用」、草稿跟着走（再点「改这份」说「上次改到一半的那一份」）** / **第十一层：另存为一份新的只给「我的文件」（官方那份灰掉带原因）、名字不预填、字节复制（新那份正文带着原来改过的字）与血统原样带过去、撞名被拒不覆盖不自动改名、不碰使用中与草稿** / **第十二层：工具栏「导入文件…」选择器能进（结果条 + 列表立刻重读）、拖到窗口上有提示且重名进改名格（输入框预填原名）、改名后进来而原来那份不动、再撞被拒、取消不多出东西、`.zip` 收不了且不许被复制进来、导入不碰「已应用」** / **第十三层：右键「在 Finder 中显示」只给「我的文件」（官方那份灰掉带原因）、点了如实说失败（浏览器里没有文件管理器、真机上的样子说清楚）、不碰「已应用」** / **第十五层：分类边界（MKP 档不许有 `.svg` 与 `MKPProcess` 切片器配置、切片器档要出现 catalog 交付行且动作是「下载」、台账「仓库 N」跟着档走两档不同数）** / 控制台无 error、无 ≥400 响应。跑法见 §7；截图落 `tmp-shots/`（已 gitignore） |

## 7. 仓库状态速记

- main 与 origin/main 同步，最新提交见 `git log`；CI 两个 job（web / rust）必须全绿。
- 未入库的 untracked：无（`workbench/.draft/` 被 ignore 属预期）。
- **下载要配数据源**：真机上在 **设置 → 高级设置 → 预设数据源** 填地址（官方源 / Gitee / 本地 `python3 -m http.server` 都行），下载与检查更新才能跑；没填时它们拒绝执行并说明去哪儿填——不猜 URL、不假装成功。`MKPSE_PRESET_SOURCE=<url>` 可把默认值编进二进制（设置页的「恢复内置默认」撤掉用户覆盖、回到它）。
- 前端底账全在 Internal 根（C4 后）：真机调试时 WebView 的 localStorage 只住偏好（置顶/搜索词），清掉不影响任何底账；旧的 `mkp.a40.*` 三格已无人读，残留可删。
- **预设页现在读 catalog**（`<appDataDir>/catalog.json`）：改了 `presets/` 源要重跑 `cargo run --bin gen-catalog`，否则判据红；真机调试时删掉旧的 `<appDataDir>/presets/` 目录不会再有影响（没人读它了）。
- **界面素材全在 `src/app/assets/`**（2026-10-01 起）：品牌 logo `bambuLogo.ts`、机型整机图 `printers/`、测试模型合影 `hero/`。换一张图 = 换一个文件（引用方改成 `import`），不重跑 `gen-catalog`、不改 `presets/`、不碰资产台账。`public/` 里只该有：台账管的载荷根 `assets/{bbs,icons,models}` 与 BBS 页元数据 `bbs/`。
- **资产去哪一档看一把尺子**：产品数据资源（用户下载 / 更新 / 管理）→ `public/assets/` + `presets/assets.toml` + catalog；界面展示素材（程序自己看一眼）→ `src/app/assets/` 或 `public/`，不进台账。台账里今天 15 条 = 9 BBS + 3 图标 + 3 模型。
- **临时编辑那条链的落点**：编辑中的正文住 `<appDataDir>/run/draft-preset.json`
  （**不是** `mkp/`：那是下载区，判据是"每个文件都在目录里登记"，塞 `.tmp` 进去就污染交付那层）。
  保存 = 另存成 `<Documents>/SupportEase/presets-mine/<原名>（已修改）<后缀>`，然后丢掉草稿。
  四条命令：`begin_preset_edit` / `put_preset_draft` / `discard_preset_draft` / `commit_preset_draft`。
  界面：本地表与云端表的交付行（**与目录一致**的那种）**与我自己那份**的展开详情里
  都有「改这份」→ 编辑器抽屉；关掉抽屉**不丢**（草稿在盘上，回头点「改这份」接着改），
  丢草稿只有 footer 那颗「放弃这次编辑」。
  **保存去哪由这份草稿改的是哪一份决定**（第八层）：官方线另存成 `（已修改）`、
  用户线**写回它自己**（同一路径，不产生第二份）。
- **用户自己那一份住哪**：`~/Documents/SupportEase/presets-mine/`（**用户根**，与内部根分开 ——
  程序管的数据不放 Documents，因为 iCloud 会把文件驱逐成占位 stub，见 `fsx::paths`）。
  两条只读命令：`get_user_preset_files`（列，**带血统、"基于哪一版官方"的判定与
  第九层的文件级状态 `state` / `stateDetail`**）/ `read_user_preset_text`（读正文，
  **只认那一格**）；**写用户根只有两条**
  （都在 `runtime::mine`，官方原件与下载区一概不碰）：`commit_draft`（另存，第 5 层）/
  `save_back`（写回自己，第 8 层）。
  界面上它在本地表里（「我的文件」那一半，展开详情里有「看正文」与「改这份」）。
  认不出的类别（`.json`）照实写「认不出是哪一类」，在任何类型档下都列。
- **血统三行写在文件里**（第七层）：另存出来的那份 = 草稿正文 + 头注释块里三行
  （`# based_on: mkp/presets/A1-standard.toml` / `# based_on_release_time:` / `# based_on_sha256:`），
  **正文逐字节不动**，与工作台建副本同一形状。客户端那份实现在 `runtime::lineage`
  （默认构建不编 `mkpse-preset`，所以是重写），两端靠 `workbench::lineage_parity` 逐字节钉住。
  **不许改用 `[metadata]` 表**：`TomlConfig` 是 `deny_unknown_fields`，多一张表 = 我们自己的
  读取面报"这份预设比本程序新"（作者原话里的例子按这条换了载体，原则没变）。
- **两条线都能成为"使用中"**（第七层）：`run/active-preset.json` 的 `origin` 说这一份住哪条线
  （`official` / `mine`，缺省 official ⇒ 旧档不用迁移），`path` 只给用户线用（用户目录可以自己
  分文件夹 ⇒ 认路径；官方线**仍不存路径**，落点由目录给）。落点解析只有一处
  （`state::active_target`），应用只有一个入口（`apply_active_preset(fileName, origin?, path?)`），
  两道闸各按自己的线；`applyActivePreset` 的 `origin` 缺省 `official`，老的调用点不用改。
- **"基于旧版官方"不是坏文件**：`mine::based_on` 拿血统里的 `based_on_sha256` 与目录里
  来源那一份比，得 `current` / `outdated` / `unknown`；`outdated` 的行上多一枚中性徽章
  （不是警示色），详情里那格写全来源。**"把改动挪到新版"（合并）作者 2026-10-02 已降级为
  暂不做的高级功能**（见 §3.5 第 8 条之后那一段）；第八层现在是"继续编辑我自己那份"。
- **归档（`archive/`）住哪、怎么来的**：`<appDataDir>/archive/` + **交付根相对路径**
  （`archive/mkp/presets/A1-fast.toml`）—— 与 `mkp/` 同形，所以"归档里这份是谁"不用猜：
  去掉前缀与目录里哪一份同位，就是谁。只有"换版本"往里放东西，**保留最早一份**（不覆盖、不删）。
  界面上它挂在交付行的展开详情里（「旧版本 N 份」→ 抽屉，能看正文）。
  **三条读只读不写**：删除 / 恢复**连命令都没有**（归档管理不在这一层）。
  它服务的是**官方线**，不是用户修改历史 —— 见总纲 §1③「预设 TOML 的一生」。
  它还兼一个身份：**判定"盘上这份是旧版本"的证据**（第 6 层）；
  第二份证据是被归档的旧目录 `archive/catalog.json`（归档槽只有一份时仍认得出更早的版本）。
- **交付预设在本机有四档**（第 6 层）：`未下载 / 已下载 / 旧版本 / 内容异常`。
  三个读合起来才够：`getDownloadedFiles`（与目录一致）+ `getStaleFiles`（盘上有但不一致）
  + `getDeliveryTrust`（**那不一致的字节认得出是哪一版吗**，`{fileName, verdict: 'old'|'tampered',
  archivedPath}`，**只列有事的**）。判定只在 `runtime::delivery::trust_entries` 一处，
  前端 `usePresetData.readRelease` 合成一次、两张表都读它。
  **「不一致」与「云端有更新」是两件事**：后者是 `checkRemoteUpdate`（比目录指纹）。
  旧版本 / 内容异常那两档共用一条边界：**不能应用 / 不能编辑 / 不能复制，只能重新下载** ——
  三道闸都在入口：应用 = `apply_active_preset` 的 SHA 校验；编辑 = `delivery::official_text`
  （取原文先核 SHA，漂了报 `SHA_MISMATCH`，`begin_preset_edit` 用它）；复制 = 这一层没有命令（界面灰掉带原因）。
  两档的按钮字不同：旧版本「更新」（同一条管道，旧份进归档）、内容异常「重新下载」（那份我们不认）。
- **预设页那条链路怎么手工看**（浏览器模式 = 假后端）：`npm run build` →
  `npx vite preview --port 4173 --strictPort` → 开 `http://localhost:4173/` 的「预设」页 ——
  交付行按假后端的**固定演示集合**画四档（`A1-standard.toml` 已下载、`A1-fast.toml` 旧版本、
  `A1mini-standard.toml` 内容异常 —— 最后一份在「全部机型」档下才看得到）；
  用户线三份演示：`我的 A1 涂胶.toml`（基于旧版官方、能应用能改、能改名）、`Process_0.2mm.json`
  （认不出哪一类）、**`坏了的涂胶.toml`（第九层：画「文件无法读取」、不给应用 / 改这份）**；
  右键「我的文件」还有**重命名**（只动名字，字节一个不动）与**删除**（二次确认；正在使用的不给删）
  —— 第十层；以及**另存为一份新的**（字节复制、血统原样带过去、不覆盖、不自动改名）—— 第十一层；
  工具栏「**导入文件…**」与**把文件拖进窗口**走的是通用导入入口（第十二层，假后端给演示路径 /
  假路径）：重名的会开「导入：有同名文件」改名格，ZIP 如实说收不了；
  右键「**在 Finder 中显示**」在浏览器里会如实说"没有文件管理器"（真机上打开并选中，
  第十三层）；点「更新」会如实报「未实现的接口」（浏览器里没有盘、没有源）。
  自动化跑一遍：`node scripts/probes/presets.mjs`（要 `playwright-core` + Edge；截图落 `tmp-shots/`）。
  **别用 dev（5321）**：那台 watcher 会扫 `target/` 下几万个文件，自己把自己拖死（探针文件头也这么说）。
- **交付根（`presets/dist/`）的布局**（2026-10-04 第一刀之后）：**发布根是 `presets/`**，
  `catalog.path` 相对它 —— A 类资产原地住在 `presets/assets/…`（**不复制进 dist**），
  B 类渲染产物落在 `presets/dist/mkp/presets/…`。于是 `dist/` 下只有
  `catalog.json` / `manifest.json` / `source.json` / `content/**` / `mkp/presets/*.toml`
  —— **客户端落点与 `catalog.path` 同形**（A 类 `<appDataDir>/assets/…`、
  B 类 `<appDataDir>/dist/mkp/presets/…`）。换过布局时旧文件会成"残留"：
  发布页有清理（进 `workbench/.trash/dist/`），`wb_publish` 也会先拦残留再动字节。
  规则源头见 `docs/PUBLISH-ARCHITECTURE.md`。

----

## 增量之二十五 · 第四刀「发布软件版本」（2026-10-04，分支 `feat/software-release`）

**六条边界作者一次裁完**：① `release.json` 住 **`presets/release.json`**（已 `git mv`；
客户端 `source::release_url` 从文件下载根往上恰好一级，正指向 `…/main/presets/release.json`）
② **`src-tauri/Cargo.toml` 是版本唯一真值** ③ 第一阶段**只做 macOS** ④ **不做签名 / 公证**
⑤ 正常 **GitHub Release + `vX.Y.Z` tag + macOS 安装包** ⑥ 客户端只做**发现新版 + 打开下载页**，
不做应用内自动更新。**预设发布与软件版本发布依然完全分开**（发预设不产生软件 Release）。

### 版本真值：单向派生（`workbench/app/version.rs`，新）

```text
src-tauri/Cargo.toml [package].version   ← 唯一真值（人只改这一处）
   ├─ package.json / src-tauri/tauri.conf.json（派生 → 安装包版本）
   ├─ Cargo.lock                         （派生）
   └─ env!("CARGO_PKG_VERSION") → APP_VERSION → tag → release.json
```

- `app_version` / `derived_mismatch` / `write_derived` / `bump`。**没有"从别处读回来当真值"的入口**。
- ★ 派生改成**纯文件操作**（含 lock 那一行），**不再跑 `cargo update`**：判据能在临时目录验，
  事务也不会因为环境里没有 cargo 就整条链走不下去。`refresh_lock` 保留给真仓库显式用。
- ★ workspace 根 `Cargo.toml` 的 `[workspace.package] version = "0.2.0"` 是 `crates/preset` 的，
  **不是** app 版本，别动。
- 判据：`the_app_version_has_one_source_of_truth`（五处同源）、`one_bump_moves_the_truth_and_every_derived_file`、
  `bump_only_moves_forward`、`the_truth_slot_is_the_package_table_not_any_version_line`

### 事务内核：一个核心、两个入口（`workbench/app/release_tx.rs`，新）

- 阶段快照（`ReleaseStage` 十四个档 → camelCase 与 `wire_name()` 逐字一致，判据钉）：
  预检 → 版本号 → 提交 → 推送 → PR → 合并 → 切 main 打 tag → 构建 → Release → 上传 →
  写 `release.json` → 它的 PR。**失败即停，不回滚远端**。
- ★ **不收 `&Ctx`**（与 `publish_tx` 的**刻意**差别）：本事务不碰 `Presets`，从根上躲开不可重入锁。
- ★ `release.json` 只在**上传成功之后**才写（先宣告后上传 = 用户点进空下载页）；
  它走**第二支分支 + 第二个 PR**，**合并留给人** —— 合并之后 raw 才吐得出去，客户端才看得到新版本。
- ★ 切 main + 打 tag 是"会动本机工作区"的动作：内核做之前会在回执里说，界面给独立警告条。
- 判据：假 `Hosting` 在临时仓库真跑一遍到 `Tagged`（含 bare remote 真推送）；预检零写入；
  主线发版被拦；演练停在 `Ready`；写出来的 `release.json` 客户端读得懂。
- **入口二**：`src-tauri/src/bin/release.rs`（`required-features = ["workbench"]`，客户端编不到它）
  + `scripts/release.mjs` 收敛成薄壳（保留三条纪律 / 提问 / 校验链 / 等 CI，其余交给内核）。

### 平台与界面

- `Hosting` 增 `create_release` / `upload_asset`（`UPLOAD_TIMEOUT = 15min`、流式 body、
  `Content-Length` 显式给、`.dmg → application/x-apple-diskimage`）。**Gitee 这一支如实报"暂不支持"**
  （trait 默认实现 + `platform_not_supported`，不许假装支持）。
- `git.rs`：第二份 stage 白名单 `RELEASE_STAGE_ALLOWLIST`（与预设那份刻意分开）+ fetch /
  ahead_behind / switch / pull_ff / tag / push_tag / tag_exists / is_clean / rev_parse_short。
- 工作台：② 卡「软件版本」从只读块升级为**入口**（`ReleaseGateModal`：闸 → 回执 → 历史三段）；
  历史独立成 `<appDataDir>/release-history.json`（**与 `publish-history.json` 两本账**）。

----

## 增量之二十六 · 0.0.2：三项真机修复（图标 / 系统代理 / 用户根搬出 Documents）

装了 0.0.1 之后作者真机踩到三件事，一次改完（分支 `feat/0.0.2-fixes`）：

### ① 应用图标换成正式那张

`npm run tauri -- icon <1024×1024 png>` 重出 `src-tauri/icons/` 全套（含 `icon.icns`）。
**android/ 与 ios/ 两套产物删掉** —— 这个项目不做移动端，留着只是让人以为要发。

### ② HTTP 出口**自动用系统代理**

- 真机症状：0.0.1 客户端取 `source.json` / `release.json` 报
  `invalid peer certificate: UnknownIssuer`；同一个地址在浏览器与 `curl` 里通 ——
  差的就是那两个走了**系统代理**，我们没有。
- 改：`runtime/net.rs` 新增 `system_proxy_url()`（环境变量 `HTTPS_PROXY`/`ALL_PROXY`/`HTTP_PROXY`
  → macOS `scutil --proxy`），**两个 HTTP 出口共用**：客户端 `net::agent_with` 与工作台
  `platform::agent_with`（含上传那只）。认不出的代理地址只 warn 并按直连走，不让整次下载失败。
- 判据：`scutil_https_proxy_is_read` / `a_disabled_scutil_proxy_is_not_used`（把
  `HTTPSEnable : 0` 当 1 是这类解析最容易犯的错）/ `scutil_socks_proxy_is_read_too` /
  `unreadable_scutil_output_means_no_proxy` / `env_proxy_is_preferred_and_empty_values_are_skipped` /
  `a_usable_proxy_url_is_accepted_by_ureq`（防"解析对了但构造失败，症状与没加一样"）。
  ★ 其它平台这轮只走环境变量 —— 第一阶段只发 macOS 安装包，不为"以后可能要"提前读注册表。
  ★ ureq 的 socks 需要 `socks-proxy` feature（未开）⇒ socks 地址认不出会 warn 直连，不崩。

### ③ 用户根搬出 `~/Documents`（作者拍）

- 症状：点开预设页弹 macOS 的「要访问你的文稿文件夹」—— 用户根在 `Documents/SupportEase`。
- 改：`fsx::paths::user_root()` = **`<appDataDir>/user`**（与内部根并排，仍是独立一层，
  只是落点从系统目录换成程序目录）。旧目录是空的，**没有存量数据、不做迁移**。
- 判据 `the_user_root_never_touches_documents`（源码扫描；★ 扫 `document_dir(` **带括号**、
  且字符串**拼出来再比** —— 直接写那个名字这条断言会把自己撞红）。
- 文档：总纲 §1③ 两根那一段改写（"程序写的文件不放在一个会被系统悄悄搬走的地方"）；
  `ipc/mine.rs` / `runtime/mine.rs` / `ipc/presets.rs` / `lib.rs` / `bridge.ts` / `contract.ts` /
  `mock.ts` / `localFiles.ts` / `presetTree.ts` 里那句 `Documents/SupportEase/…` 一并改口径。

**这三件都是程序改动 ⇒ 走「发布软件版本」链发 0.0.2**（正好是第四刀第二段的验收对象）。

## 增量之二十七 · 0.0.3 的前半：双源 + 自定义地址两种形状都认（分支 `feat/gitee-release`）

作者 2026-10-05 拍：**两个官方源（Gitee 默认 / GitHub 备选）+ 收起的自定义**，
并且**用户容易输错地址**这件事要一并解决。另一半（应用内下载 .app.zip + 标题栏环形进度 +
自动替换重启）是下一刀。

### 根因（真机）

0.0.2 客户端里**手动指定"官方那个 source.json 地址"反而不通**，而内置默认同一个地址是通的：

- 手动指定那条路原本**只认"数据源根"**（根下直接有 `catalog.json`）；
- 而用户复制来的地址**本身就以 `source.json` 结尾** → 程序当根拼出
  `…/source.json/catalog.json` → 404。
- ⇒ 不是用户错，是我们把"一个地址有两种读法"当成了两种东西。

### 改法

- `runtime::source`：**从"一个地址"改成"选哪一个源"**（`SourceMode{Gitee,Github,Custom}`，
  `SOURCE_SCHEMA` 升 **2**）。★ 读 1 代老档**不是兼容层**：那份 `baseUrl` 本来就是
  "用户手填的地址"，含义正好等于新模型的 `custom`。
- **两个内置源**构建期注入（`build.rs` 从 `workbench/bootstrap.json` 读
  `bootstrapUrl` / `giteeBootstrapUrl` → `MKPSE_PRESET_SOURCE` / `…_GITEE`；
  没注入的那个**不出现**在界面里）。
- **自定义地址两种形状都认**（`CustomShape`）：`Root` / `Bootstrap` / `Unset`。
  `bootstrap_candidates()`：**原样先试**（用户复制的地址通常已经带着 `source.json`），
  不成再在根上补文件名 —— 之前"再拼一层"会拼出 `…/source.json/source.json`，永远不通。
- **保存前先探一次**（`probe_custom_shape`），通了才落盘；不通则**每次尝试的原因都摆出来**。
  内置两个源不联网不探（地址是程序自带的）。
- 界面：两个固定单选 + 收起的自定义；「恢复默认」只在"当前不是默认档"时出现。

### 判据（+7，默认 328）

`the_v1_file_reads_as_a_custom_source` / `a_builtin_mode_refuses_an_address` /
`a_custom_mode_without_an_address_has_no_entry`（不许静默回落到内置源）/
`only_three_source_modes_are_recognized` /
**`a_custom_url_is_read_as_a_root_or_as_a_source_json`**（本次真机 bug 的钉子：同一个
服务，填根探成 `Root`、填 `source.json` 地址探成 `Bootstrap`，两条都通）/
`an_unusable_address_is_refused_with_both_reasons`。
判据里那个 `TinyServer` 是手写 `std::net` 的最小服务端（只按路径答两种内容）——
它属于**判据脚手架**，不进产品架构。

**★ 判据不许开端口（2026-10-05 CI 抓到的）**：形状判定第一版写成"起一个本地 `TcpListener`
当假服务器"，`check:zero-network` 当场报红（`启动零网络扫描`：原始套接字 / TCP 监听 / TCP 连接）。
那是对的 —— **客户端产品不开端口，判据也算客户端**。改成**注入取字节的动作**
（`FetchBytes` + `probe_custom_shape_with` / `parse_source_json_with`），判据在纯内存里跑完整条判定，
产品的门仍走 `net::get_bytes` 唯一出口。假 fetch 只看 URL 的**最后两段**
（`…/source.json/catalog.json` 必须 404 —— 那正是真机上"填 source.json 地址"不通的那条路）。

## 增量之二十八 · 第五刀「应用内更新」（交接见 `HANDOFF-5-APP-UPDATE.md`）

作者 2026-10-05 拍板：「以后下载要像 Trae / WorkBuddy 那样」——标题栏常驻小图标 + 环形进度、
点开详情面板（下载/暂停/取消）、下完「重启并安装」点了就重启。**一次提完，不再切碎。**

- `release.json` 加**可选** `asset` 格（name/url/size/sha256）：**加字段不升代次**，
  没有它客户端退回"打开下载页"（0.0.2 / 0.0.3 的行为仍然成立）。
- 新增 `runtime::updater.rs`：流式下载（`net::stream_get`，**不引 zip 依赖**——
  `ditto` 打包 / `unzip` 解压都是 macOS 自带）+ 进度 + 暂停/继续/取消 + 验大小/验 SHA + 解压。
- 新增 `ipc/update.rs`：六个命令 + **`open_url`**（★ 修「查看更新点了没反应」：
  webview 没 opener 权限，`<a target="_blank">` 必然打不开，一律过命令）。
- **装上去并重启**：正在运行的 `.app` 换不掉 ⇒ spawn 后台脚本（`sleep 1`）→ `app.exit(0)`；
  成没成本进程看不见 ⇒ 脚本写 `run/update-result.json`，**下次启动**读它说清。
- 标题栏指示器 `UpdateIndicator.tsx`：环形进度 + 面板 + 「重启并安装」；
  **idle 时什么都不渲染**（不占位），且**挂载后延迟 1.2s 才问**（首屏不等云端）。
- **mock 夹具能触发整套界面**（作者 2026-10-05：不想为验界面真发一版）：
  `src/api/mock.ts` 的 `MOCK_NEW_VERSION` 决定"有新版本 / 已是最新"，假下载可暂停/取消/装。
- 工作台发版多两步：`ditto` 打 `.app.zip` → 上传 → 写进 `release.json` 的 `asset`。
  **失败不挡发版**（如实说"这一版退回打开下载页"）。
- 判据：默认 **338**、workbench lib **624**（新增 10 条，含"老发布物没有 asset 仍读得懂"、
  "`file:` 地址被拒"、"解压只看退出码"、"账读得回来"）。
- 写盘纪律**第二处逃生口**：`updater` 流式写用 `File::create`（atomic 是"整个进内存"），
  代码里写了理由与退役条件。

### 纪律追加（作者 2026-10-05，因为 PR 号涨太快）

**一轮只提一次代码 PR**（手上攒的改动一起提）；**发版固定 +2**（版本号 + `release.json`）；
端到端验收放在**发版之后**，不在发版前插验证轮次。

### 纪律追加二（作者 2026-10-05，因为 PR #36 的 CI 我没盯）

**开完 PR 要自己盯 CI，全绿了才通知作者合并。** 推上去就不管 = 把"红"这件事
变成作者去发现 —— 2026-10-05 的一条判据（用了 `/dev/null`）在 rust-windows 上红了，
是作者贴截图告诉我的。**判据也守跨平台**（CI 有 macos / ubuntu / windows 三个 job）。

## 增量之二十九 · 首页「下载并应用」按套餐消费（2026-10-06/07，分支 `feat/app-state`）

作者 2026-10-06 拍板：①修死路 + ②首页真正消费套餐**一起做**；BBS **先只落盘**（不拷进切片器，
用途二期再定）；「已应用」优先；本轮**不做** `deliver` 的二次省网络优化。

- **死路修在 bridge，不在 Rust**：原计划的 `on_tick: Option<Channel<T>>` **编译不出来** ——
  Tauri 的 `Channel<T>` 只实现了 `CommandArg`（要 `Webview` 才能建）、**没有 `Deserialize`**，
  于是 `Option<Channel<T>>` 落不到任何 impl 上。改成 `src/api/bridge.ts::withTick`
  **无条件挂一条 Channel**（回调可省、参数不可省）。新判据 `scripts/check-channel-args.mjs`
  （接进 `npm run check:channel-args` + CI，**负向验证过**：把死路那行放回去会红）。
- **首页接 `getVersionFiles()`** —— 「套餐 → 文件」这条链本来就存在（就是预设页在用的那一支），
  所以**不在契约里另长 `bundles`、不在 TS 里重算路径**（那会是同一条事实的第二份手抄）。
  四态：已应用 / 下载并应用（**缺优先于漂**）/ 更新并应用 / 应用；只下「缺 ∪ 漂」（按 fileName 去重）；
  任何一份没成就**停下、不 apply**；失败原样透出（`errorText`），内层吞异常的 `catch {}` 拆掉。
- ★ 施工时探针逮到：**「在盘上」= 已下载 ∪ 漂移，漂 ≠ 缺** —— 那两张单子都只收盘上真有的文件。
- mock 两处收口：目录登记收成一处（新 `mockServer/catalogFiles.ts`；上游老命名 `A1.toml` 与消费端
  `A1-standard.toml` 对不上会让首页套餐一律显示「缺」）＋ `MOCK_DOWNLOADED` 补一条套餐的另一半。
- 判据：Rust 一个没动（`cargo test` 全绿、`--features workbench --lib` **687 passed**）；
  客户端两条探针全绿（`home-flow --pick` 新增四态 + 「零下载」断言；`presets.mjs` 未受影响）。
- **真机已验**（作者 2026-10-07 日志）：三次 `downloadRuntimeFiles` + 六次 `applyActivePreset` 全 ok，
  全齐且全新那几档**零下载**；A1 mini 那次**只下了缺的 BBS**（已最新的 MKP 没重下）；
  BBS 落在 `<appDataDir>/assets/bbs/Process/0.4mm/…`。
- 施工计划与验收清单：`docs/HOME-BUNDLE-DOWNLOAD.md`。

## 增量之三十 · Rust 工具链 / 纪律清理（2026-10-07，**已施工**）

**作者裁定：单独一刀做，不混进增量之二十九。** 且"第一步不是改代码，是先确认 CI 工具链"。

- **CI 实际用 rustc 1.99.0 / rustfmt 1.10.0**（两个 Rust job 都是 floating `stable`；从真实 run 日志读到）；
  本机当时是 1.97.1 ⇒ 落后两版。装上 1.99.0 复核后**报的是同一批** ⇒ **不是工具链漂移**，
  是 feat/app-state 新写的代码踩了 `clippy.toml` 既有的写盘纪律（那批提交没推过，CI 从没见过）。
- 修法按**仓库既有形状**：9 处测试里的 `std::fs::write` → `fsx::atomic::atomic_write`
  （`src-tauri` 里 atomic_write 有 117 处、`std::fs::write` 只在注释里），**没有加 `#[allow]`**；
  1 处 doc 列表缩进；另 `cargo +1.99.0 fmt` 收 11 个文件（AppState 那批新行没跑过 fmt，1.97 / 1.99 都认）。
- 全绿：`fmt --check`（两个版本）、双 feature `clippy --all-targets -D warnings` 都干净；
  判据 **729**（默认）+ **687**（workbench lib）全过。本刀零 TS 改动。
- 明细与遗留项（**要不要钉 `rust-toolchain.toml` —— 没做**）见 `docs/RUST-LINT-CLEANUP.md`。

## 增量之三十一 · 设置页 Token 明文可见 / 可复制（2026-10-07，分支 `feat/app-state`）

作者 2026-10-07 反馈（截图 = 设置页「发布账户」）：**点一下掩码就变成空的输入框** ⇒
"不知道它到底还有没有"；要"看着它、复制它"。两问定案（施工前问过）：

- 查看门槛 = **不要密码** —— 本地凭据文件本就仅本人可读，门槛只是象征性的，点眼睛直接出明文；
- 改 Token 的入口**保持点掩码进去**，但进去后输入框用掩码点当**占位**（不再显得空）。

- **后端**：新命令 `wb_get_publish_token(platform) -> Option<String>`（`publish_tx.rs`，
  `#[tauri::command(async)]`，进 `read_commands_are_async_…` 的 **IO 单子**；`lib.rs` 登记）。
  ★ **口径变更写进文档**：原来"前端拿不到原值"是硬承诺，现在改成"状态面照旧不含原值 +
  另开一条**显式**出口"。`PlatformAccountView` **形状一个字段没变** ⇒ 判据
  `credentials_never_echo_the_token` 照旧成立；出口只在人点眼睛那一刻走（不是开场自动读、
  不进发布链）。三处口径同步：`credentials.rs` 头注释、`docs/PUBLISH-ARCHITECTURE.md` §7.1 第 3 条、`api.ts`。
- **前端**（`SettingsPage.tsx`）：Token 行三种形态 —— 掩码（可点进编辑）/ **明文**（只读、
  点一下整段选中）/ 编辑（已配置时 placeholder = 掩码点）；右侧**眼睛**（显示 ↔ 收起）
  + 明文旁一颗**【复制】**。明文只活在界面状态里：收起 / 保存 / 清除 / 重新读取一律收回
  （`loadAccount` 里 `setRevealed({})`）。新样式 `.tokenEye`（`c14.module.css`）；桩同步
  （`mockBackend.ts` 的 `wb_get_publish_token`：GitHub 有、Gitee `null`）。
- 验证：`cargo fmt --check` / 双 feature `clippy --all-targets -D warnings` / `cargo test` +
  `--features workbench --lib` / `npx tsc -b` / `npm run lint`（只剩既有 2 条 react-refresh warning）/
  `npm run build` + 三道闸。
