# 交接：数据架构重做 —— 第一圈完成，第二圈进行中

> 更新时间：2026-10-01
> 适用分支：`main`（只认 CI 绿的 tip；推送用 `ALLOW_PUSH_MAIN=1`，提交用 `ALLOW_COMMIT_ON_MAIN=1`——本地闸，见 §5）
> 上下文：按《四圈舞步》重做数据架构。**根规则**是 `docs/DATA-ARCHITECTURE.md`（四层世界 + 四条铁律 + 十问 + 准入问句），**对账单**是 `docs/DATA-INVENTORY.md`（43 件现状逐件归位 + 收口进展日志）。这两份先读，本文只讲"现在在哪、接下来去哪"。

## 0. 进度总览

| 圈 | 内容 | 状态 |
|---|---|---|
| 第一圈 | 骨架全立起来：数据世界 / 最小 Catalog / 文件系统 / Delivery 骨架 / 用户数据 / 页面闭环 | ✅ **100%**（六块全通，2026-10-01） |
| 第二圈 | 每块地基做厚 | ✅ **100%**（六项全通，2026-10-01：更新与归档 / R11 共用契约 / catalog 加厚换源 / **C4 localStorage 退役** / **Delivery 加厚（真数据源上线）** / **判据 2 启动零网络**） |
| 第三圈 | 统一资产入口（Catalog + Source + Delivery 一个世界），再让发布成为生产者，最后 Preset 成为第一个完整消费者 | ✅ 第 1、2 步收口；第 3 步（Preset 消费者）**十二层 + 第十三层收尾全部收口**（2026-10-02，见 §3.5）—— 用户文件生命周期闭环（创建 / 修改 / 管理 / 使用 / 外部管理）；「分享」不做、单文件「导出」暂缓（见 §5）；下一个真块是「设置 → 备份与恢复」（ZIP，复用第十二层入口） |
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

## 4. 续做入口（从哪接手）

- **总盘点在 `docs/PROJECT-AUDIT.md`**（2026-10-02）：把 HANDOFF / 总纲 / 工程约束 /
  产品规则 / 现有页面对了一遍，列成**已完成 / 半完成 / 未开始 / 已废弃**四类
  （按作者 ①–⑩ 排：备份恢复 / 设置页 / Preset 产品能力 / 机器型号 / 校准测试 /
  加工记录 / 状态收尾 / 云交付验收 / 跨平台 / 旧世界清理），外加**文档自身的欠账**
  与建议次序。**接手前先读它** —— 它说的"半完成"是"差哪一半"，不是"没做"。
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
| `runtime::mine` 血统 3 条 | `the_committed_copy_shows_up_in_mine_files` 里带上血统 / `the_official_update_shows_up_as_based_on_an_old_version`（**官方换版 → `outdated`**，且说得出机型）/ `unknown_when_the_source_cannot_be_resolved`（没有血统 / 没记摘要 / 来源已不在目录里）/ `lineage_is_read_from_the_head_only`（**只看头 8 KB**，用户目录里可能有几百 MB 的文件）/ `an_unreadable_head_is_just_no_lineage` |
| `runtime::state` 两条线 5 条（第七层） | `the_official_pointer_resolves_through_the_catalog`（**落点由目录给**，`mkp/presets/…` 真布局下 `intact` 才是 true；目录里没有了 ⇒ 漂了）/ `an_older_pointer_file_still_means_the_official_line`（**旧档不迁移**）/ `the_users_own_copy_can_be_the_active_one`（用户线按用户根解析；用户再改它算"漂了"）/ `a_pointer_pointing_outside_the_mine_dir_is_not_resolved` / `applying_one_line_replaces_the_other`（唯一性） |
| `scripts/probes/presets.mjs`（**手工**，非 CI） | 预设页探针：两轴可点 / 四张表可读 / 点行展开 / 右键菜单 / BBS 入口跨页 / **交付行的四态与动作（已下载·灰字、旧版本·「更新」、内容异常·「重新下载」）** / **存疑那两档没有「应用」也没有「改这份」、右键「另存为一份新的」带原因灰掉** / **批量那一层（批次行只含未下载+旧版本+内容异常、逐份结局各占一行且不许伪装成功）** / **我那份能被应用并说得出「基于旧版官方」** / **改我那份 → 保存回它自己（不产生第二份、血统还在）** / **第九层：读不出来的那份照常列在表里、画得出「文件无法读取」（角标带原因）、没有「应用」也没有「改这份」，能读的那份不受牵连** / **第十层：改名只动名字（坏的那份改完还是「文件无法读取」）、删除有二次确认且删完行没了、正在使用的那份「删除」灰掉带原因、改名不断「已应用」、草稿跟着走（再点「改这份」说「上次改到一半的那一份」）** / **第十一层：另存为一份新的只给「我的文件」（官方那份灰掉带原因）、名字不预填、字节复制（新那份正文带着原来改过的字）与血统原样带过去、撞名被拒不覆盖不自动改名、不碰使用中与草稿** / **第十二层：工具栏「导入文件…」选择器能进（结果条 + 列表立刻重读）、拖到窗口上有提示且重名进改名格（输入框预填原名）、改名后进来而原来那份不动、再撞被拒、取消不多出东西、`.zip` 收不了且不许被复制进来、导入不碰「已应用」** / **第十三层：右键「在 Finder 中显示」只给「我的文件」（官方那份灰掉带原因）、点了如实说失败（浏览器里没有文件管理器、真机上的样子说清楚）、不碰「已应用」** / 控制台无 error、无 ≥400 响应。跑法见 §7；截图落 `tmp-shots/`（已 gitignore） |

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
- **交付根（`presets/dist/`）的布局**：`catalog.json` + `manifest.json` 在根，产品资源一律在 `mkp/…`
  （`mkp/presets/` 是 `wb_generate` 落的、其余按 kind 分目录）——**与客户端下载区同形**。
  本机换过布局时，旧目录里的文件会成"残留"：发布页有清理（进 `workbench/.trash/dist/`），
  也可以直接把 `presets/dist/mkp/presets/` 之外的东西清掉重发（它本来就不入库）。
