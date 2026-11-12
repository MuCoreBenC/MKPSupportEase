# 交接台账（活）

> 更新时间：2026-11-04 · 分支：`feat/workbench-gen-receipt`
> **本文只装"现在还用得上的东西"**：怎么验证、纪律、判据、仓库状态、**未结账**、最近约六条增量。
> 更早的增量（25 圈之前 / 36~44 / 45~58）已于 2026-11-04 清理，git 历史里可查。
> 总纲：`DATA-ARCHITECTURE` / `PUBLISH-ARCHITECTURE` / `RELEASE-TRANSACTIONS` / `APP-STATE` /
> `HOOK-ERRORS-AND-MESSAGES` / `ARCHITECTURE §7.5`；全仓文档索引 `docs/DOC-INDEX.md`。
>
> **防再乱的规矩**：新东西一律写在 §3；**增量只留最近约 6 条**，更旧的直接清掉（git 历史可查），本文只留一句说明；
> 清理之前，段里凡是「待验收 / 待拍板 / 已知红」的，**必须先抄进 §2.4**，否则跟着丢。
> §1/§2 是活规则，改了要回头看有没有别处引用它的章节号。

## 1. 怎么验证（每一轮都要全绿才推）

```bash
cargo test                                   # 默认 feature（在仓库根跑，workspace 全成员）
cargo test -p mkp-support-ease --features workbench --lib   # **workbench 的判据也要跑**（CI 两个都跑）
cargo clippy --all-targets -- -D warnings    # clippy 两种 feature 都要：默认 + --features workbench
cargo fmt --check                            # CI 有格式 job
npm run build && npm run check:bundle        # 前端构建 + 判据 1（安装包产物扫描）
npx eslint <改过的文件>                       # CI 跑全量 lint
```

改了 `presets/` 源或交付产物后必须 `cargo run --bin gen-catalog` 重生成，否则判据测试 `embedded_matches_rebuild` 红。

## 2. 纪律 / 判据 / 仓库状态

### 2.1 已知纪律 / 不要碰（每一条都是踩过的坑）

- **协作边界：什么事停下来问作者，什么事自己定**（2026-10-02 定死，不要再逐项回问）：
  **只有这四类停下来问** —— ① 产品行为变了；② 数据归属变了；③ 公开契约 / API / schema 的
  方向变了；④ 与已确定的架构原则冲突，或存在需要作者做产品取舍的多个方案。
  **其余一律自己定**：内部实现细节、代码组织与文件放哪、测试与判据怎么改、
  旧 schema 要不要暂时保留空位、同类资源要不要顺手迁移、叫什么名字 ——
  只要不改变产品行为、不违背已定的架构原则，就别拿回去让作者拍板。
  两条以上都合理时的优先序：① 守已定的总纲 → ② 不引入新的兼容层 / 临时方案 →
  ③ 守住当前这一圈的边界 → ④ 选改动更小、下一条最容易接的那个。
  **决定与理由写进台账**（本文件 §2.1 或 §3（增量）或 `DATA-INVENTORY.md` 的进展日志），
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
- **`presets/dist/` 是本机产物，不入库**；判据/构建器的输入用入库真身 `crates/preset/assets/presets/`。`BUILTIN_PRESETS` 由 `crates/preset/build.rs` 扫该目录自动生成（2026-11-12 起），份数与文件名都随清单走，改版本 id 不需要手改表。
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

### 2.2 判据清单（全是活的，别删）

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
| `runtime::mine` 写回 5 条（第八层） | `saving_back_writes_the_same_file_and_keeps_the_provenance`（**核心不变式**：**同一路径、不产生第二份**，来源记录原样保留 —— 摘要不许变成"我自己改过的字节"）/ `saving_back_an_untouched_file_changes_nothing`（没改就逐字节不变）/ `saving_back_a_file_without_provenance_invents_none`（本来没记录就不编一个）/ `saving_back_refuses_when_the_file_is_gone`（被移走了**拒绝且不新建**）/ `saving_back_stays_in_the_mine_dir` |
| `runtime::mine` 第九层 5 条（文件级：能读 + UTF-8 + TOML 语法） | `a_broken_toml_shows_up_as_unreadable`（坏 TOML → `unreadable`，原因**报得出第几行**）/ `a_non_utf8_file_is_unreadable_too` / `an_escaping_symlink_is_never_read`（**指向用户根外的链接一个字节都不读**（来源记录也不读）；根内链接照常能用）/ `read_preset_text_blocks_the_broken_one_but_viewing_still_works`（应用 · 编辑两个入口共用这一处闸；**看正文照旧** —— 读它不算"用"）/ `an_external_edit_that_still_parses_is_fine`（**不比 SHA**：外部改过但仍是能读的 TOML = 正常）。既有 `mine_files_lists…` / `the_committed_copy_shows_up_in_mine_files` 搭上 `state` 断言 |
| `runtime::mine` 第十层 9 条（改名 / 删除） | `renaming_keeps_the_bytes_and_the_provenance`（**核心不变式**：**字节一个不动**、来源记录原样、旧名字没了新名字在；子目录里的份留在子目录）/ `renaming_refuses_a_name_that_changes_the_kind_or_has_a_path`（换后缀 / 带路径 / `.` / `..` / 空全拒，原文件原地不动）/ `renaming_never_overwrites`（落点有东西就拒，被撞的那份一个字节没动）/ `renaming_to_the_same_name_is_a_no_op` / `deleting_really_deletes`（**真删除**：不在了，而且没有多出旁的东西）/ `deleting_refuses_while_it_is_the_active_one`（正在使用的拒；正在使用的是**别人**照删）/ `deleting_refuses_while_a_draft_is_open`（有草稿的拒；草稿改的是**别人**照删）/ `the_delete_gates_only_apply_to_the_mine_line`（官方线的指针 / 草稿挡不住）/ `deleting_stays_in_the_mine_dir`（越界路径拒） |
| `runtime::state` 跟改名 3 条（第十层） | `the_active_pointer_follows_a_rename`（路径与文件名换新、**指纹原样**；不指着它的、官方线的指针一概不动）/ `the_draft_follows_a_rename`（草稿路径与来源名换新，**正文与打开时指纹原样**；换完再点「改这份」还是接上这一份）/ `repointing_nothing_is_a_no_op`（没有那两本账 = 没跟，不是错误） |
| `runtime::mine` 第十一层 6 条（另存为一份新的） | `copying_keeps_the_bytes_and_the_provenance`（**核心不变式**：新的一份与原来**逐字节相同**（来源记录原样带过去）、原文件一个字节不动；子目录里的份复制出来还在子目录）/ `copying_uses_the_same_name_gate`（与改名同一套：换后缀 / 带路径 / `.` / `..` / 空全拒）/ `copying_never_overwrites_and_never_renames_itself`（**不覆盖**（被撞的与源都一个字节没动）、**也不自动改名**（名字和原来一样也拒））/ `copying_touches_no_state_at_all`（**一个状态都不碰**：使用中指针与草稿都还在原来那份上）/ `copying_does_not_require_readable_text`（不读内容：二进制那份照样原样复制）/ `copying_stays_in_the_mine_dir`（越界路径拒） |
| `runtime::import` 第十二层 8 条（通用导入入口） | `staging_says_ready_collision_and_rejected`（能收 / 重名 / 收不了各带原因；**同一批里重名的也认**）/ `importing_copies_the_bytes_and_leaves_the_source_alone`（**核心不变式**：字节复制、**源文件一个字节不动**；进来就是一份正常用户文件）/ `importing_does_not_invent_a_provenance`（没有来源记录**不编造**）/ `importing_does_not_reject_invalid_toml`（**不校验内容**：坏的照样进得来，第九层再如实说它读不出来）/ `importing_never_overwrites`（不覆盖，被撞的与源都一个字节没动）/ `importing_uses_the_same_name_gate_for_a_new_name`（重名改名走同一套门槛）/ `importing_touches_no_state`（使用中指针与草稿都不动）/ `importing_creates_the_mine_dir_when_missing`（全新安装也能进） |
| `runtime::mine` 第十三层 3 条（在文件管理器里显示） | `revealing_resolves_the_file_inside_the_user_root`（**核心不变式**：解析出用户根里那个**绝对路径**；**读不出来的也能显示**（文件管理同族）；子目录里的也认）/ `revealing_stays_in_the_mine_dir`（`exports/`、`../` 一概拒 —— 与改名 / 删除同一道闸）/ `revealing_a_missing_file_says_so`（外面删了就说找不到，不去猜） |
| `runtime::state` 草稿两条线 2 条（第八层） | `a_draft_knows_which_line_it_belongs_to`（**同名不同线 ≠ 同一份**：「接着改」不许接错）/ `an_older_draft_file_is_still_the_official_line`（旧档 = 官方线，**不升 schema**） |
| `runtime::provenance` 写回 4 条（第八层） | `saving_back_writes_the_same_file_and_keeps_the_provenance` / `saving_back_an_untouched_file_changes_nothing` / `saving_back_a_file_without_provenance_invents_none` / `saving_back_keeps_the_line_endings` |
| `runtime::delivery` 可信度 5 条（第 6 层） | `a_clean_copy_is_current_and_not_reported`（没问题就不报警）/ `tampered_bytes_are_not_recognized`（哪儿都对不上 = 异常，且没有证据可指）/ `a_copy_of_the_archived_version_is_recognized_as_old`（**认得出旧版本**，证据是归档里那条路径）/ `a_version_from_the_archived_catalog_is_recognized_as_old`（归档槽被占了也认得出更早那一版）/ `official_text_refuses_bytes_that_drifted`（**不许拿漂了的字节当原文改**） |
| `runtime::state` 草稿 4 条 | `draft_roundtrips_and_clears`（存/读/丢，丢是幂等）/ `absent_draft_is_none_not_error` / `corrupted_draft_is_an_error`（坏档不静默）/ `draft_and_active_are_separate_files`（**改一份 ≠ 在用它**：两个状态文件互不干扰） |
| `runtime::provenance` 7 条（第七层，客户端那份来源记录） | `ko2_a_copy_differs_from_its_source_only_by_the_provenance_block`（**副本 = 来源 + 尾部 `[mkp_source]`**，剥掉逐字节相同）/ `writing_twice_is_idempotent`（同一份记录写两遍逐字节相同，不叠加）/ `crlf_is_preserved` / `origin_only_writes_no_history`（另存只写来源、不长履历）/ `a_file_without_a_block_parses_as_none`（没有记录 = `None`，不是空壳）/ `history_appends_without_touching_old_entries`（只追加，旧条目一个字不改）/ `ko3_history_only_grows_when_a_cloud_version_is_merged_in`（另存不生长、合并才生长） |
| `workbench::provenance_parity` 3 条（**两端一致性**，只在 workbench feature 下编） | 客户端与工作台对 9 份入库产物给出**逐字节相同**的副本 / 读出来源记录相同（含"官方原件没有来源记录"两边都是 `None`）/ 摘要算法相同 —— 两份实现之间没有编译器，靠这三条钉住 |
| `workbench::app::build` 生成前预演 7 条（2026-10-02） | **`preview_never_touches_the_disk`（核心不变式：预演一个字节都不写 —— 新增的那份不被创建、已有的原字节不动）** / `a_new_file_shows_every_line_as_added`（新增全绿）/ `an_identical_file_has_no_diff_lines` / `a_changed_line_shows_one_removed_and_one_added`（删在前增在后，与 git 同序）/ `an_inserted_line_shows_only_an_added` / `a_deleted_line_shows_only_a_removed` / `the_counts_match_the_line_kinds`（新增/修改/无变化三档 + `+N −N` 计数） |
| `scripts/probes/workbench-build.mjs`（**手工**，非 CI） | 生成与发布页（① 生成 / ② 数据包 / 查看 JSON / 查看 TOML / 版本轴 / 云端那一格）· **切页立刻显示（2026-10-02）：整本没回来时先画骨架屏（不是黑屏）+ 导航已在** · **生成页放开选择：已生成的行也能勾（0 个被禁）+「全选」勾满 N/N** · **生成前确认：点「生成」先弹 diff 确认框（不是直接覆盖）· 确认**之前**不写盘 · 确认之后框内换结果页 · 确认之后产物名单才跟上** · 参数台增量。**注意**：`button[title*="钉住"]` 那一处是**预存红**（按钮已删，见 §3.5 整理那一刀）—— 不是本文改动引入的 |
| `runtime::mine` 来源记录 3 条 | `the_committed_copy_shows_up_in_mine_files` 里带上来源记录 / `the_official_update_shows_up_as_based_on_an_old_version`（**官方换版 → `outdated`**，且说得出机型）/ `unknown_when_the_source_cannot_be_resolved`（没有记录 / 没记摘要 / 来源已不在目录里）/ `provenance_is_read_from_the_tail_not_the_head`（记录在尾部；旧的头注释不再读）/ `an_unreadable_head_is_just_no_provenance` |
| `runtime::state` 两条线 5 条（第七层） | `the_official_pointer_resolves_through_the_catalog`（**落点由目录给**，`mkp/presets/…` 真布局下 `intact` 才是 true；目录里没有了 ⇒ 漂了）/ `an_older_pointer_file_still_means_the_official_line`（**旧档不迁移**）/ `the_users_own_copy_can_be_the_active_one`（用户线按用户根解析；用户再改它算"漂了"）/ `a_pointer_pointing_outside_the_mine_dir_is_not_resolved` / `applying_one_line_replaces_the_other`（唯一性） |
| `scripts/probes/params-settings.mjs`（**手工**，非 CI） | 参数页 + 设置页：参数页照目录画（分类 / 卡片 / 行 / 底栏）· **① 弃用字段显示但只读（摊开的弃用行有「已弃用」徽章 + 控件全 disabled）** · **④ 塔地图按默认板画出板轮廓并替下 X/Y 行（svg「塔」+ 板 evenodd 路径 + 槽位含坐标行）** · 设置页「高级设置 → 预设数据源」全流程（当前状态如实 / 手动指定应用（尾斜杠砍掉）/ 非法地址如实拒 / 恢复内置默认能撤回）；两档尺寸 0 console error / 0 个 ≥400。**原 `params-sync.mjs`** ——「同步」页退役那一刀改名重写 |
| `scripts/probes/machines-dims-zones.mjs`（**手工**，非 CI） | 机型与版本页的**品牌树 + 尺寸六组 + 禁区编辑器**（2026-10-03 增量之十三）：品牌是父节点（可点收、折叠后机型行消失、带台数）· 机型缩进当子节点且**确实嵌在品牌分组里** · 筛选同时管两边 · 新增品牌 → 树里多一棵 · 机型行右键「移到品牌…」→ 选一家 → **原来那家台数 −1、新家 +1** · 尺寸卡六组读数与源文件**逐格一致**（床身 260×255 / 标定点 68.21 与 126.373 / G-code 标记）· 尺寸模态框 43 格数字、改床身宽保存后**卡上跟着变** · 禁区编辑器 `viewBox` 就是**床身尺寸**（不是可打印区）、两块多边形 **6 + 4 点**、`(0,0)` 落在**画布左下角**（Y 翻转的定点）、右侧列表点数 · 清到 0 块保存 → 那一格变「没有禁区文件」。跑法见文件头（同 asset-preview 三步） |
| `scripts/probes/brands-bundles.mjs`（**手工**，非 CI） | 机型与版本页的**品牌** + 套餐页可读性 + 状态栏（2026-10-03 增量之十二）：左列有品牌段（显示名 / id / 台数）· 点品牌开品牌卡（显示名可改且**改了跟着变**、品牌图那枚 `<img>` **真解码**、列出「这个品牌下的机型」）· 新增品牌 → 左列多一行且右侧切过去、**撞名（只差大小写）当场被拒** · 机型卡「品牌」格显示的是**显示名**且「看品牌」跳得过去 · 套餐左列两行读法（显示名不被省略、下行是 id + 装了 N 个文件）· 卡头大标题 = 显示名、附注 = id · 「选择版本…」是**树状**（机型组头 + 缩进版本行）且**行没有垫背景色** · 待确认 chip 里 uid 与「它现在指着谁」有间距（不粘连）· 状态栏只摆 `仓库名/目录名`、导航缩到 150px 也不越界。跑法见文件头（同 asset-preview 三步） |
| `scripts/probes/presets.mjs`（**手工**，非 CI） | 预设页探针：两轴可点 / 四张表可读 / 点行展开 / 右键菜单 / BBS 入口跨页 / **交付行的四态与动作（已下载·灰字、旧版本·「更新」、内容异常·「重新下载」）** / **存疑那两档没有「应用」也没有「改这份」、右键「另存为一份新的」带原因灰掉** / **批量那一层（批次行只含未下载+旧版本+内容异常、逐份结局各占一行且不许伪装成功）** / **我那份能被应用并说得出「基于旧版官方」** / **改我那份 → 保存回它自己（不产生第二份、来源记录还在）** / **第九层：读不出来的那份照常列在表里、画得出「文件无法读取」（角标带原因）、没有「应用」也没有「改这份」，能读的那份不受牵连** / **第十层：改名只动名字（坏的那份改完还是「文件无法读取」）、删除有二次确认且删完行没了、正在使用的那份「删除」灰掉带原因、改名不断「已应用」、草稿跟着走（再点「改这份」说「上次改到一半的那一份」）** / **第十一层：另存为一份新的只给「我的文件」（官方那份灰掉带原因）、名字不预填、字节复制（新那份正文带着原来改过的字）与来源记录原样带过去、撞名被拒不覆盖不自动改名、不碰使用中与草稿** / **第十二层：工具栏「导入文件…」选择器能进（结果条 + 列表立刻重读）、拖到窗口上有提示且重名进改名格（输入框预填原名）、改名后进来而原来那份不动、再撞被拒、取消不多出东西、`.zip` 收不了且不许被复制进来、导入不碰「已应用」** / **第十三层：右键「在 Finder 中显示」只给「我的文件」（官方那份灰掉带原因）、点了如实说失败（浏览器里没有文件管理器、真机上的样子说清楚）、不碰「已应用」** / **第十五层：分类边界（MKP 档不许有 `.svg` 与 `MKPProcess` 切片器配置、切片器档要出现 catalog 交付行且动作是「下载」、台账「仓库 N」跟着档走两档不同数）** / 控制台无 error、无 ≥400 响应。跑法见 §2.3；截图落 `tmp-shots/`（已 gitignore） |

### 2.3 仓库状态速记

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
  两条只读命令：`get_user_preset_files`（列，**带来源记录、"基于哪一版官方"的判定与
  第九层的文件级状态 `state` / `stateDetail`**）/ `read_user_preset_text`（读正文，
  **只认那一格**）；**写用户根只有两条**
  （都在 `runtime::mine`，官方原件与下载区一概不碰）：`commit_draft`（另存，第 5 层）/
  `save_back`（写回自己，第 8 层）。
  界面上它在本地表里（「我的文件」那一半，展开详情里有「看正文」与「改这份」）。
  认不出的类别（`.json`）照实写「认不出是哪一类」，在任何类型档下都列。
- **来源记录三行写在文件里**（第七层）：另存出来的那份 = 草稿正文 + 头注释块里三行
  （`# based_on: mkp/presets/A1-standard.toml` / `# based_on_release_time:` / `# based_on_sha256:`），
  **正文逐字节不动**，与工作台建副本同一形状。客户端那份实现在 `runtime::provenance`
  （默认构建不编 `mkpse-preset`，所以是重写），两端靠 `workbench::provenance_parity` 逐字节钉住。
  **不许改用 `[metadata]` 表**：`TomlConfig` 是 `deny_unknown_fields`，多一张表 = 我们自己的
  读取面报"这份预设比本程序新"（作者原话里的例子按这条换了载体，原则没变）。
- **两条线都能成为"使用中"**（第七层）：`run/active-preset.json` 的 `origin` 说这一份住哪条线
  （`official` / `mine`，缺省 official ⇒ 旧档不用迁移），`path` 只给用户线用（用户目录可以自己
  分文件夹 ⇒ 认路径；官方线**仍不存路径**，落点由目录给）。落点解析只有一处
  （`state::active_target`），应用只有一个入口（`apply_active_preset(fileName, origin?, path?)`），
  两道闸各按自己的线；`applyActivePreset` 的 `origin` 缺省 `official`，老的调用点不用改。
- **"基于旧版官方"不是坏文件**：`mine::based_on` 拿来源记录里的 `based_on_sha256` 与目录里
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
  —— 第十层；以及**另存为一份新的**（字节复制、来源记录原样带过去、不覆盖、不自动改名）—— 第十一层；
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

### 2.4 未结账（2026-11-01 复核 · 找活干先看这张表）

**A. 已知红（不修就过不了 CI）** —— 2026-11-01 实跑 **2 条，全是「盘上数据改了、产物没重生成」**：

| 判据 | 现状 / 修法 |
|---|---|
| `runtime::tests::embedded_matches_rebuild` | 工作区里 `presets/**` 与 `crates/preset/assets/presets/*.toml` 有一批未提交的数据改动，`catalog.generated.json` 与"按现在的源重建"不一致（那份文件本身也被改过，仍不合）。修法 `cargo run --bin gen-catalog`。 |
| `workbench::app::delivery::tests::delivery_catalog_matches_rebuild` | 同一批数据的连带：`presets/delivery/catalog.json` 与重建不一致。修法 `npm run presets -- generate`（生成不推进序号）。 |

两条都是**生成物**，改它们算发东西，等作者点头再跑。
旧账那两条（`our_render_matches_…baseline` / `real_data_matches_the_documented_split`）
已随数据同步（`A1-fastv3.3` 的 `offset_x = -0.7` / `offset_z = 3.9`）转绿，别再来找。

**B. 等作者拍板**

1. **工作台减速开关**（`--dev-slow-ms`：只在进度事件之间 sleep、只改时序、`workbench` feature 专属）——
   8.5 MB 那份只跑 **8.4 s**，「够慢」这条验收没达成。三条路（换更大的件 / 加开关 / 不做）里我倾向加开关，
   **未点头**（增量之四十二 §4 第 3 条）。
2. **图片按需下载**（`docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md` §2）：两条 guide 现在 `delivery = 'bundled'`，
   **用户端看不到新图**；要做的是台账去掉 bundled + 重跑 `gen-catalog` + 客户端补「读下载区图片」的口子
   （两张图会连带 4 条条数判据变红，照它说的核对，别改数字）。**必须配版本发布** —— 老客户端 + 新目录 = 破图。
3. **「一组 / 套装」那三件裁决**（同文档 §3：一组能不能超过两张 / 组名从哪来 / 客户端要不要动）——
   建议都先维持现状，等真需求。
4. **品牌名英文**：`machines[].brand` 仍是单串（现在值里自带 `Bambu Lab`）；版本名那批已加 `nameEn`
   （2026-10-31），品牌这一格**等作者发话**（增量之五十七 §四）。

**C. 待真机验收（自动化做不到，只能在真机点一次）**

1. **校准件认档**：拿作者那份 `.44432.0.gcode` 跑后处理 —— 不再报 `E_CAL_MISMATCH_002`，
   且插的 11 个方块相对旧版**只在 Y 基准上平移 1 mm**（增量之三十九）。
2. **数据源认分支页**：设置页贴 `…/tree/feat/dark-mode` → 校准页「从云端获取」拿到
   `ZOffset_Calibration_ext.3mf` → 「打开模型」能开（增量之四十）。
3. **矩阵里要人在客户端窗口真操作的三档**：`calib-ok`（校准件认成档 + 跑完）、
   `mismatch`（机型不匹配那一问真答一次）、`cancel`（真点一次停止）（增量之四十三 / 四十四）。
4. **「停止并关闭」**：真机造一次「答晚了」（例如碰撞档等超时之后再点选项）。
5. **Windows 的图片引导**：「设置 → 去系统里改…」→ 卡片贴控件那一步的端到端观感
   （透明窗在 Windows 上到底长什么样，一直没人工看过）。
6. **智能塔位置**：碰撞时回答 `relocate` 之后**真的把塔挪过去并跑完**这一条
   （见 `docs/SMART-TOWER-RELOCATION-HANDOFF.md`；矩阵 `collision` 档目前只跑到这一问超时为止）。
7. **分组改名 / 换位：保存才落盘、放弃真退回** —— 后端那桩「保存不亮」**已破并修好**
   （2026-11-01，见增量之六十四 §③；两条回归判据钉着），**等真机点一次**：
   换位 / 改名 → 保存·放弃点亮 → 放弃退回 → 保存后重开仍是新状态。

**D. 小尾巴（不影响发版）**

- 8 条 **macOS-only 死代码 warning**（`ax.rs` / `guide.rs` 里在别的平台编不出来的表与函数），
  增量之三十八 记的，非那一轮引入。
- **M5 欠账**：`machine_dims::install()` 在发布物里的接线（校准模型档那条读取链的前置），增量之三十九 ②。
- **`src/workbench/**` 的 i18n**：仍全中文（作者裁定「单排一轮」；那是开发台，不进客户端这条线）。
- **`run/preset-history` 的 `action` 仍是句子**：换语言不复译 —— 要复译得改成 `actionKey` + 参数，
  那是账本形状的改动（增量之五十五 §三.2）。
- **低频/失败态那几句**：`usePreset.ts` 的 `PENDING_NAME`、`useBundleFiles` 的三句套餐提示、
  `openWithGuide` 的 `alt`（描述的是中文系统截图，译了反而对不上图）。

----

## 3. 最近的增量

> **更早的增量已清理**（2026-11-04，git 历史可查）：
> 清理前段里的"待验收 / 待拍板 / 未做 / 已知红"都抄进了 **§2.4**；这里只留最近六条。

## 增量之五十九 · 客户端看不见「只盖身份」的那次发布（2026-10-29）

作者真机：第 1 号发布之后，预设页每份文件都是「版本关系未知」，点「更新」之后还是「未知」，
按钮永远挂着。

**根因**：`runtime::update::check` 只比 `revision`，而 `revision` 刻意**不含**
`catalogId` / `publishSerial`（它们是发布事件痕迹，不是内容）⇒ 第 1 号那次
「内容一字未变、只盖身份」的发布**永远到不了客户端**。客户端手里一直是 10-19 那代
没有身份的目录（截图「云端更新时间 2026-10-19 22:12」就是它，而云端真实是
`publishedAt 2026-10-29T04:54:13Z`），于是每份文件 `based_on` 第 ① 步就 `Unverified`；
点「更新」只能以「没有身份」再建一次基线 ⇒ 死循环。

修两处：

- `update::check` 补上「发布身份前进」这一半：`revision` 变 **或** 同线
  `publishSerial` 变大 **或** 本机没身份而远端有 ⇒ 有新目录。远端没身份 /
  不同线不比序号。五个单测钉住。
- 前端兜底：`cloudRowState` 在 `unverified` 且**云端目录自己没身份**时不给「更新」，
  改说「目录旧了」—— 那个按钮点了也建不出基线，比没有更坏
  （新键 `presets.tree.cloudLocal.noIdentity`，中英各一条）。

验证：`cargo test --features workbench --lib` 1076 + 238 + 78 全绿；双 feature clippy 0；
`fmt --check` ✓；`tsc -b --force` ✓；eslint/stylelint 0；`build` ✓；i18n 1583 键。
**未提交**。

## 增量之六十 · 矩阵写回被当成「过期派生」丢掉（2026-10-29）

作者实测：对照矩阵里手动输入改数值，格子只变红、数字还是旧的（点步进箭头"像是对的"）；
G-code 格只读态写着「10 行 · 点开」，点开却是空白。

**同一个根因**：`ParamsPage` 的 `refreshKeyRef.current` 恒按 `page: 'desk'` 算，
而对照模式 `onWrite` 递出去的是 `page: 'matrix'` 的 `refresh`
（`refreshKeyOf` 前缀就不同）⇒ `sendEdits` 把矩阵模式**每一次**写回的派生
都判成"过期"丢掉 ⇒ `setMatrix` 从不执行 ⇒ `cell.text` 永远是旧值。
手输后关闭编辑器看到的是旧 `text`（只有 `raw` 变了 ⇒ 红），
而步进箭头全程开着编辑器、读的是 `raw`，所以"看起来只有箭头生效"。
批量编辑那一支同理（`BatchEdit` 的 `onApply` 也按同一颗 ref 判）。

修两处：

- `refreshKeyRef.current` 按 `mode` 算（matrix 用 `cols` + `tab:null` + `query`），
  与 `onWrite` 递的 `pageRefresh` 一字不差；
- `editQueue.patchRow` 乐观层顺手换 `text`：数值用原文（后端那份带单位/小数位，
  回来替换），**空串显示也留空**（原来会留着旧摘要 —— "写着 10 行、点开空白"正是它）。
  枚举/开关/G-code 摘要不在前端重算。

验证：`tsc -b --force` ✓ · eslint ✓ · `build` ✓ · `build:workbench` ✓。**未提交**。

## 增量之六十一 · 「所属分组」自绘下拉 + 顺手新建分组（2026-11-01）

作者：「我希望在这里能加上新增的按钮，这个下拉菜单可能需要自己写了不能用原生的」
（截图：定义框的「所属分组」，六个页签名上各圈了一处）。

**为什么原生 `<select>` 做不到**：一行只能是选项，页签只能当 `optgroup` 的死标签，
没有任何地方挂按钮。换成 `views/SectionPicker.tsx`（`FieldPopover` + 自绘面板）：
每个页签那一行右端一颗「＋ 新增分组」，点了就地长出输入框（Enter / 添加 / Esc 收回）。

**新分组跟定义同一笔落盘**（这是这次唯一有分量的裁定）：

- 载荷 `ParamMetaEdit.newSection = { id, tabId, label }`；后端 `ensure_section` 先把
  两份文件里的 `[[tabs.sections]]` 各补一条（布局那份 `items = []`），再走原来那条
  「搬组」逻辑 —— 两份文件的写法只有一条路。
- **不先建一个空组**：这里没有删分组的入口，建了又取消留下的空组谁也删不掉。
  所以草稿里那个 `newSection` 只在「`sectionId` 正指着它」时才交出去；
  改选别的组 = 那份作废（`onSection`）。
- **id 由前端给**（`custom_N`，跳过盘上与草稿里已占的）：撤销 / 重做会把同一笔载荷
  原样重放，后端按「确保这个分组在」处理（已在且同页签 = 空操作）。
  中文标签没法当 id —— 它会进两份 TOML，也会进客户端目录。
- 名字上限 24 字、id 只收 `a-z0-9_`（新消息键 `ui.presetdata.registry.345`–`350`）。

顺手修了假后端一处**早就存在**的不一致：`SECTION_LABEL` 里存的是「页签 · 分组」，
而真后端的 `row.sectionLabel` 是纯分组名（桌面卡片头才是「页签 · 分组」）——
于是抽屉头会印成「擦料 · 擦料 · 擦料方式」。现在桩里两处与真后端同口径。

验证：`cargo fmt --all --check` ✓ · 双 feature `clippy --all-targets -D warnings` ✓ ·
`cargo test --features workbench --lib` 1135 passed（新增两条：建组+搬组+重放幂等 /
非法 id 被拒且文档与盘都没动）· 消息表闸门 ✓ · `tsc -b --force` ✓ · `npm run lint` ✓ ·
`npm run build` ✓ · `check:i18n` ✓ · `check:bundle` ✓ · 浏览器桩里真点过一遍
（建组 → 保存 → 卡片落到「擦料 · 试验分组」）。**未提交**。

## 增量之六十二 · 分组抽屉四稿收敛 + 「只有右上角才算保存」（2026-11-01）

作者连着两轮把上一稿推翻，口径很硬，值得逐条记：

- 「**我不要点击的改名**，我要点击的打开右侧的抽屉……上下这些因为如果标题就能上下
  的话，**很容易不小心点到的就是排序**」⇒ 卡头里的输入框与箭头全部撤掉；
- 「我是希望**上面这个也可以点了去改名字**」「**选择是哪一个，它哪一个就亮一点**」
  ⇒ 名字本身就是编辑入口（点名字 = 就地改名），选中那一行铺底色 + 左边竖标 + 实心圆点；
- 「这样我就可以**顺便点击右侧的抽屉去切换**，就跟在左侧原本那个点击切换的一样」
  ⇒ 抽屉里的分组清单也带选中圆点，点它 = 换一组看（抽屉跟着换内容、页面滚过去闪一下）；
- 「我那些模态框里面的保存按钮……应该叫**确认**吧」+「**只有右上角的保存才是真的保存**，
  其他的都只是应用」⇒ `ParamDefModal` / `MachineDimensionsModal` 的按钮改成「确认 / 应用中…」，
  副标题改成「确认只进草稿，右上角「保存」才写文件」。

落地：`SectionList` 重做成「名字块 + 选中圆点 + ↑↓」三块（抽屉与「所属分组」下拉共用
一份）；`SectionDrawer` 里点圆点走 `onShowSection`；定义框那颗「确认」改完会调
`onEditApplied()` 刷状态栏（否则界面上写着「已保存」，草稿里却已经躺着两笔）。

**踩到的两个坑（都记在代码注释里）**：
① `FieldLayer` 自己 `pointer-events: none` —— 抽屉挂在它里面，不加回 `auto` 就整块点不动
（现象：看得见，`elementFromPoint` 命中的是底下的卡片）；
② toml_edit 0.22 的 `ArrayOfTables` 没有 `insert`，而 `mem::take` 出来的表带着旧 span，
重新 `push` 回去序列化会**丢表** ⇒ 分组换位改成「两位内容对调」
（`replace_table_contents`，保留各自 decor）。`cargo test` 有一条测试同时钉住
「注册表数组次序 + order 重排 + 布局那份数组也跟着换」。

验证：`cargo fmt --check` ✓ · workbench 特性 clippy `-D warnings` ✓ · lib 1132 passed /
4 failed —— **这 4 条红与本次改动无关**（`every_entry_has_english` /
`every_entry_has_japanese_and_traditional` / `embedded_matches_rebuild` /
`delivery_catalog_matches_rebuild`），它们是工作区里已有的一处未提交的注册表改动
（`wiping.support_extrusion_multiplier` 被搬进 `custom_1`、少了英文/日文/繁中名）
加上没跟着重新生成目录造成的；`tsc -b --force` ✓ · `npm run lint` ✓ · `npm run build` ✓ ·
浏览器桩里真点过（名字即改 / 选中高亮 / 圆点切换 / 抽屉里换位）。**未提交**。

### 补记：分组行第五稿 —— 点行＝切换，改名只认右边那支笔

作者看完第四稿：「点击了就是切换，而不是说点击的是改名字。你把那个改名字加一支笔的
图标放在右侧，点击了笔才是改名字」；抽屉标题「要加也可以，顺手的事情」。

- 行里那颗名字按钮换回**整行 = 选中/切换**（与最早那版一致），名字不再是编辑入口；
  笔挪进右侧动作列，与 ↑↓ 并排**常显**（藏到悬停里人就找不到，这一次就是被这么挑出来的）。
- 选中态保留：淡底 + 左边竖标 + 实心圆点（圆点降级成指示，不再单独可点）。
- 抽屉标题加一支笔（点笔才变输入框），与行里那支同一尺寸。
- 顺手修第四稿的排版 Bug：改名那一行原来是输入框 + 一句「回车确定 · Esc 撤回」挤一行，
  在 380px 抽屉里把下一行盖住了（截图里糊成一片）⇒ 那句提示撤掉（`title` 里写着），
  并给 `.editRow` 铺实底。

### 补记二：组件卡的卡头点不开 + 工作台也要有热床占位图

作者：「我好像看出来了，为什么这个没有了，因为它没有到这个分组里面……它总得有个位置吧？
那就算它是未分组，那为什么会在这个擦料里面呢」「客户端那边有个画布，这个工作台其实
也应该要有」。

**"点不开"的真身**：`RegistryView.sections` 走 `layout_sections()`，而它把
`component.is_some()` 的分组整条滤掉了 —— 组件卡的卡头在页面上，注册表里没有它，
抽屉里 `here === null` 直接 `return null`，看着就是点下去没反应。改法：
- 后端拆出 `layout_sections_all()`（含组件、带组件名），`layout_sections()` 变成它的
  过滤视图（**保持原语义**：「所属分组」下拉与 `move_param` 一样不许组件卡当目标）；
- `SectionTarget` 加 `component: Option<String>`，前端「所属分组」自己筛掉组件卡
  （唯一例外是这条参数现在就住在里面 —— 否则触发器会说"这一组找不到了"）；
- 抽屉查不到分组时**照样开**，用卡片自己的名字兜底，副标题写"注册表里没有这一组"。
  新测试 `layout_sections_all_keeps_component_cards` 钉住两边的取舍。

**占位图**：新增 `views/BedPreview.tsx`（纯 SVG，无依赖）。组件卡里**行在左、图在右**，
用的是行里现成的值（`wiping.wiper_x/y` + `machine.bedSize.x/y`，草稿值也算），
缺一个就不画 —— 画一张不知比例的空床比不画更误导。图注写明"示意"：客户端那张按
板件轮廓画，工作台手里没有 `Plate` 资产。塔的包围盒照客户端口径估（60 打底、按床宽收）。
`.pScroll[data-cols='1']` 时并排掉成一列。

验证：`cargo fmt --check` ✓ · workbench clippy `-D warnings` ✓ · lib 1133 passed /
4 failed（同一批已知红：工作区里那份未提交的注册表改动缺英/日/繁中名 + 目录没重生成）·
`tsc -b --force` ✓ · `npm run lint` ✓ · `npm run build` ✓ · 桩里看到图（1600×1000 实测）。

### 补记三：占位块换成「纯占位」+ 窄卡漏宽的真凶

作者要的是「不用像客户端一样真实，占个位置就行；宽度不用很多，主要在左侧」——
所以热床示意图整个撤掉，改成 `views/ComponentSlot.tsx`：一块虚线方块 + 「画布 + 组件名」，
哪一侧由 `views/componentSide.ts` 的 `COMPONENT_SIDE` 表说（`tower-map` = left）。
**这张表是唯一判据**，别的卡以后有画布就在表里加一行。

**「窗口一变就不行」的真凶不是这条布局，而是参数行的 `content-visibility: auto`**
（C14 为拖窗口做的性能优化）+ `contain-intrinsic-size`：被浏览器跳过渲染的行，
宽度参与不了外层分配 ⇒ 483px 的卡里右列只拿到 349px，卡右半边一直空着。
拿 grid 替 flex、给 `flex: 1 1 0`、加 `min-width: 0` 全都一样（一路试过来的结论），
因为问题在「行的固有宽度不可知」这件事本身。

**定案**：并排这块不靠内容撑 —— 占位块 `flex: none; width: 132px`，右列
`width: calc(100% - 132px)`。四种窗口（1080/1200/1440/1920）实测：
两列模式下 `占位块 132 + 右列 = 卡片宽`，行宽 = 右列宽（铺满）；
单列模式（正文 < 880）占位块掉成上面一条 72px 高的窄带，右列仍吃满。
`settings` 那些 `layout = 'component'` 的整块卡由后端标 `standalone`，抽屉里不再列。

验证：`tsc -b --force` ✓ · `npm run lint` ✓ · `npm run build` ✓ · 四种窗口尺寸量过上述不变量。

### 补记四：占位块**任何宽度都在左侧** + 删掉 `settings` 页签

作者 2026-11-01：「为什么这个画布现在在正上方了，而不是在正左方」——
那是我在补记三里让它「窄了就掉成上面一条」的。**位置是它的意义**，所以改成：
占位块宽度用 `--slot-w`（两列 132px / 单列 104px），右列 `calc(100% - var(--slot-w))`
跟着让位 —— **只压窄，不换位置**；任何窗口宽度下都在左、且与参数列等高。

同时按作者要求清掉 `settings` 页签（「那些是旧的，可以清理掉，不需要再存在那个
文件里面了」）：`presets/layout_schema.toml` 删掉整个 `[[tabs]] id='settings'`（11 张
`layout='component'` 的空卡，客户端那边是代码固定的），`param_registry.toml` 删掉同名
`[[tabs]]` 及其 10 个 `[[tabs.sections]]`（这些组下**没有任何参数**，已逐条核对）。

顺带把工作区里那笔没做完的编辑收干净：`custom_1` 当时只有中文名（工具写进去的，
还是双引号风格），补上 `labelEn='Support' / labelJa='サポート' / labelTw='支撐'`
并统一回单引号，`gen-catalog` 重建 —— 于是之前那 4 条已知红（缺英/日/繁中名 +
目录不一致）全绿，`cargo test --lib` **1137 passed / 0 failed**。

⚠️ `presets verify` 仍报「账本第 4 号与盘上目录不是同一代」：**这条在我动手之前就有**
（stash 掉全部改动后跑一遍是同样的 4 处不一致），属发布面的事，要靠
`npm run presets -- publish` 走正规流程，不许手拼。

## 增量之六十三 · 「使用后自动复制后处理脚本」+ 「清理数据」/ 卸载连数据一起清（2026-11-01）

作者：「我使用之后自动复制后处理脚本。你在设置里面加上这个开关，就是预设列表。我点击使用啊，
或者在首页点击那个使用。反正只要变了那个使用的状态，就从一个文件变成另外一个文件的时候，
就复制那一个正在使用的后处理脚本到那个剪切板，它们的后端都是一样的，就是那个复制后处理脚本的
操作，执行一次需要有通知。还有不只是加一个卸载的，还有一个清理数据的，就是只清理不卸载，
或者卸载的时候一起清理。」

### ① 自动复制（设置 → 预设，第二行，出厂关）

- **判据只挂一处**：「使用」有五个入口（预设表「应用」/ 首页抽屉 / 首页「下载并应用」/
  参数页换组合 / 校准页换组合），它们最后都收敛成「`appState.active` 这一格变了」。
  于是监听放在 `app/state/appState.ts` 的 `appStateMutated()` 里，与上一份比身份
  （`origin + 落点 + 文件名`，**不带 SHA**：内容更新过的还是同一份）。挂到按钮上就有五处要写、
  漏一处就是"某个入口不抄"。
- **抄的动作仍是那一个实现**：新文件 `app/postprocess/autoCopy.ts` 只做"要不要抄"，
  真的抄走 `copyPostProcessScript` —— 命令由后端现拼、通知话术与手动那颗「复制脚本」
  一字不差（`copyScript.*` 那几条键）。所以用户看到的还是「已复制后处理脚本 / {name} ——
  去切片器的后处理栏粘贴」。
- **不打扰的三种情形**：首读（启动时把盘上的账读进来，`wasLoaded === false`）、
  撤销使用（`next === null`）、同一份再点一次（身份相同）。这一档读不出来时**当作关**。
- 存储：`run/app-state.json` 的 `presets.autoCopyScript`（`PresetPrefs` 第二格），
  两个命令 `get/set_auto_copy_script`。后端**不读**这一格 —— 它不改任何产物，
  唯一读者是前端那一条。

### ② 清理数据 / 卸载连数据一起清（设置 → 清理与卸载）

- 新命令 `clear_app_data`：删掉 `<appDataDir>` 整棵树（预设 / 云端原文 / 归档 / 回收站 /
  执行记录 / 偏好 / 日志），**程序本体留着**，然后 `exit`。
- `uninstall` 多一个 `cleanData`：点名时**先清数据再删本体**，不点名与从前一样一个字节不碰。
- **两条都清完就退**（这是这次唯一有分量的裁定）：`run/app-state.json` 被删了，
  进程要继续活着的话，界面上"当前状态"下一次任何写都会把刚清掉的文件原样写回去 ——
  用户看到"清了个寂寞"。所以 `exit`，重新打开才是一张白纸。
- Windows 上删不掉被本进程开着的日志 ⇒ `remove_data_root` 走 detach 的 `cmd`
  （等两秒再 `rmdir`），与卸载那条同一个手法。
- 前端 `DataSection` 这两颗危险按钮共用一颗 `useArmed()`（3 秒回退的两段式确认）——
  各写一份的话"3 秒回退"就有了两个实现。那颗「卸载时一并删除数据」的开关还管着
  **风险那一段的字**：关着说"您的数据会保留"，开着改说"连数据一起删掉"（界面与后端
  那两个分支必须一一对上）。
- 顺手把 WebView 自己那批偏好也抹掉：`storageKeys.ts` 新增 `clearClientPrefs()`
  （按 `mkp` 前缀扫，`STORAGE` 以后加一格不必来补）。不抹的话重开之后预设全没了、
  置顶表却还在。

验证：`cargo fmt --all --check` ✓ · 双 feature `clippy --all-targets -D warnings` ✓ ·
`cargo test -p mkpse-postprocess --lib` 238 passed ✓ · `check:i18n` 1772 条 ✓ ·
`tsc -b --force` ✓ · `npm run lint` ✓ · `npm run build` ✓ · `check:bundle` ✓ ·
`check:zero-network` ✓ · `cargo test -p mkp-support-ease --lib` 700 passed /
3 failed（同一批已知红：工作区里那份未提交的注册表改动缺英/日/繁中名 + 目录没重生成）。
**未提交**，等作者真机点过。

## 增量之六十四 · 分组改名/换位并入「保存/放弃」生命周期 + 放弃回滚修复 + 一桩未破的案（2026-11-01，分支 `feat/workbench-gen-receipt`）

> 作者原话（复述，新会话先读这段）：
> 「我希望这个排序就跟编辑参数一样呀，要保存才算的……你就想和 linus 一样，都是编辑文件而已嘛，
> 不管是什么编辑，排序，还是改名字，都是一样的需要保存的，可以放弃的。」
> 「组件卡跟别的一样呀，只是它的样子不一样而已。」「只有右上角的保存才是真的保存，
> 其他的都只是应用……点了放弃，那就真的是放弃。」

### ① 作者的要求（产品口径，别再走样）

- 分组（**含组件卡**）改名与上下换位：入口在右侧抽屉（笔=改名、箭头=换位，常显），
  与参数值编辑**同一套生命周期** —— 改动只是应用（影子树预览，前端实时刷新位置/名字），
  **右上角「保存」才真落盘**，「放弃」就真退回磁盘原状。
- 改完之后：保存/放弃按钮要**点亮**、状态栏「未保存改动 (N)」要涨；放弃后按钮回灰、界面退回。
- 抽屉与左侧卡片的分组顺序必须一致（两侧认同一份真理）。

### ② 这轮改了什么（全部未提交）

1. **抽屉与卡片同序**：`registry::layout_sections_all`（`presetdata/registry.rs` ~2116）
   原来按**布局数组次序**吐分组，而卡片侧 `derive::row_sort_key` 按**注册表 `SectionMeta.order`** 排
   ——「支撑」能两侧对不上。改成内层按 `order` 排序（找不到排最后）。
2. **组件卡同权**：删掉 `set_section_meta` 里「组件卡拒收改名」的 352 拦截
   （数据层不再替界面决定哪些能改）；`SectionList.tsx` 组件行的笔/箭头恢复。
3. **上箭头方向**：`SectionList.module.css` 补 `.caretUp{transform:rotate(180deg)}`。
4. **「放弃退不回去」修复（真机 Bug）**：`wb_discard`（`workbench/app/mod.rs` ~1956）
   原来只 `txn::mirror(real→shadow)` 回滚**影子文件**，**内存态没回滚** ——
   换位是"改内存→写影子"，不重读的话 `view_of` 吐的还是换位后次序。
   现在的顺序：`ctx.draft` 清空后**先 `ctx.flush()`**（干净草稿=删文件，防 reload 把旧草稿读回来）
   → mirror real→shadow → mirror real→base → **`ctx.reload_from_disk()`**
   （保存路径一直有这步，放弃路径此前漏了）。
5. **演示桩对齐（mock 此前在骗人）**：`dev/mockBackend.ts` 的结构类命令直接改夹具本体、
   放弃退不回去、且没实现 `wb_txn_state`（保存/放弃永远不亮）。补：
   `SHADOW_MUTATORS` 名单 + `captureShadow/restoreShadow`（首改前深拷贝
   PARAMS/TABS/MACHINE_VIEWS/BUNDLES/GROUPS/ASSETS/BRANDS/MOCK_CALIBRATION；
   `wb_save` 成功后快照换当前份=新基线，`wb_discard` 整体还原）；
   `wb_txn_state` 如实报 `structDirty`（成功一条记一笔；canUndo/canRedo=false 与桩无撤销栈一致）。
6. **数据**：`custom_1`（支撑）组建 + `wiping.support_extrusion_multiplier` 搬入 + 英/日/繁中名补齐；
   换位算法已改「正文块整块对调」（`swap_section_blocks`，先改文件再改内存），
   回归测试 `moving_a_group_lands_in_the_transaction_and_reorders_the_shadow`
   钉着「换位必须进事务 + 影子真被换」。
7. 验证：第一轮 `cargo test --lib`（workbench feature）1139 passed / 0 failed；
   tsc / eslint / stylelint / build 干净；Playwright（mock）闭环
   换位→点亮→放弃→还原+按钮回灰 ✓。
   **第二轮（破案后）**：1139 passed / 2 failed + 1 ignored —— 两条红都是
   `§2.4 A` 那两条「数据改了、目录没重生成」（与 22:22 那两笔无日志写入同源），
   不是这一段代码；`cargo fmt --check` / 双 feature clippy 干净。

### ③ ✅ 悬案已破：「改了排序保存不亮」（2026-11-01 深夜，同一分支）

**根因（一条同时解释两桩怪事）**：`Ctx::open` 把会话的 `Presets` 读到了
`settle_shadow` 返回的 `source_root`，而"干净"时它返回的是**真仓库**。于是——

- 编辑（`write_back` / `apply_values`）直接写进 `presets/` ⇒ 22:22:03 那两份真仓库文件
  被无日志改写（本来就不该有"保存"这一步）；
- `run_ctx` 拍的是**影子**快照 ⇒ 前后差集恒空 ⇒ `txn.record` 返回 None ⇒ dirty=0
  ⇒ 保存/放弃永不亮（22:17 "影子被写、日志却没有「编辑已进事务」"）。

`source_root` 是这一轮 R1/R2/R3（基准快照）改造的副产物：`HEAD` 那版一直是
`Presets::load_from(&shadow_root)`，改签名时顺带把"读哪一份"也改了 —— **是回归，不是设计**。
21:28 那次换位有记账，是因为当时影子里留着上次会话的草稿（走了 `recovered` 分支）。

**修法**（`set_section_meta` 一个字没动，静态链路本来是对的）：

1. `settle_shadow` 只回 `recovered`；`Ctx::open_at` **永远** `Presets::load_from(&shadow_root)`
   （真源只用来铺影子 / 基准）。
2. 收回三处"根应该指真仓库"的旁证：`reload_from_disk` 的盘上戳改盯 `real_root`（否则自己
   每改一笔都被判"外部改过"）；`audit_with` 的交付根 / 载荷根改用 `real_root`（影子里的
   `delivery/` 是开场拷贝，生成之后就是旧的）；`generation_source_note` 改成**比两棵树的内容**
   （比根路径会永远说"按草稿生成"）。
3. `wb_copy_recipe` 还在 `load_presets()`（真仓库）→ 改走会话（影子），同一种病。
4. `wb_save` / `wb_discard` 的本体抽成 `save_in` / `discard_in`，判据走的就是那颗按钮的代码。

**判据（两条都先红后绿，红的时候正是作者报的那句话）**：

- `a_session_opened_from_disk_edits_the_shadow_until_save`：从磁盘开会话 → 编辑只进影子
  → 事务 +1 → 放弃真退回 → 再改并保存才写 `presets/`；
- `a_group_move_from_a_fresh_session_only_touches_the_shadow`：**悬案原样重放**（换位 →
  事务 +1、真仓库那两份文件逐字节不动 → 放弃回滚、新建的组消失）。

把 `open_at` 临时改回读真源复验：前者红在「会话必须读影子树」，后者红在
「换位没进事务 —— 右上角那颗保存就不会亮」，与 22:17 的现场一字不差。

### ④ 教训（写给下一轮）

- 命令直接改模块级单例夹具的（mock/测试桩），「放弃/保存」语义必须配快照，
  不然桩在骗人，真机行为对不上。
- 排查「改了没反应」时先分清三件事：**前端有没有发命令**（日志有无 IPC 条目）、
  **命令有没有写盘**（文件 mtime）、**写盘有没有进事务**（「编辑已进事务」日志）。
  三者对不齐的地方就是洞。
- **换"从哪一份读"这种一刀，必须同时收回它的旁证**（盘上戳 / 交付与载荷根 / 生成依据），
  并且判据要从**真入口开一次会话**再断言盘上字节 —— 只测内部函数（`reconcile`、
  `set_section_meta`）的话，根错了也全绿，这正是本次回归溜过去的原因。
## 增量之六十五 · 工作台状态模型重构报告（2026-11-02，分支 `feat/workbench-gen-receipt`）

> 作者原话（复述，新会话先读这段）：
> 「不要去做这种小小的修修补补了，我要的不是这个徽章修好，我要的是整个这个系统，
> 整个架构的重构，破坏性的重构。你先探讨一下这个影子树的必要性，出一个报告」
> 「放弃 = 把工作台当前的整个工作状态，恢复到打开工作台时的状态……不是只恢复参数，
> 也不是清除未保存标记，而是整个工作台的一次 GET。」
> 「基准快照必须独立于当前编辑状态，不能随着编辑一起变化。」

### ① 交付物

`docs/WORKBENCH-STATE-REARCH-2026-11-02.md`（**只出报告，零代码改动**）。
六部分：现状证据（带行号）/ 目标架构 / 改动清单（后端 25 + 前端 9）/ 15 条回归判据 /
8 刀迁移表 / 提交风险清单。

### ② 病根（一条链，不是四个 bug）

`derive.rs:963 rest_at()` 读 `self.presets.registry...machine_variants`，而会话的
`presets` 根**永远指影子树**（`open_at` → `Presets::load_from(&shadow_root)`），
`wb_apply_draft` 又**先写影子再拍快照**（`mod.rs:1965` → `:1973`）。
于是「保存前的值」与「现在的值」出自同一份文件：

- 70→65：影子已是 65 ⇒ `rest=65`、`dirty=false` ⇒ **徽章不出现**；
- 65→60：`rest` 读到上一笔 65 ⇒ **显示上一个值**。

与作者实测逐条吻合。`dirty_of` 里的 `draft.pending` 分支是 10-31 之前的死代码。

### ③ 关键判定

- **影子树退场**，改成 `BASE`（打开时快照，只读）+ `WORK`（编辑树）。
  `BASE` **已经存在**（`txn::base_root()` = `.draft/base/`），只是全仓只拿它做三方对账、
  从不拿它算 `rest` —— 所以这次不是新造概念，是**把已有基准接上**。
- **放弃 = `mirror(BASE → WORK)`，一个字节都不读 `presets/`**。现在的 `discard_in`
  是 `mirror(real → shadow)`，回到的是「磁盘现在」；两者只在外部改过 `presets/` 时分岔，
  而那正是最危险的情形（判据 11 钉住）。
- **撤销改粒度**：现在撤销是整文件字节回写（`txn.rs:840`），改成按操作重放（判据 14）。
- **保存的原子性已经有**：`txn::commit` 逐份 `atomic_write` + 失败 `apply_bytes` 回滚 +
  删除进 `.trash/<stamp>/`（`txn.rs:645-706`），判据 10 **不需要新写**，只需补 `.trash/pre` 整份留底。
- **徽章两处 JSX 不改**（`ParamsPage.tsx:2221` / `ParamDetail.tsx:266`），`Cell.rest` 语义修好即可。

### ④ 待裁决（卡在这里，未动手）

1. `Cell.rest` 的三档口径：`None` / `Some(钉着的值)` / `Fallback(退到出厂)`。
   `A1:STANDARD` 的 `toolhead.speed_limit` 在注册表里 `machineVariants` 是**空的**
   （70 = `defaultValue`，登记表注明「空表是刻意」），所以「回到 70」= **删键挂回继承**，
   徽章要显示 70 就必须走回落链。这一条决定判据 1 的实现形态。
2. 提交：工作区 105 项改动里有三个必须先处理 —— `presets/src-tauri/`（畸形嵌套目录，
   应删）、`tmp-scroll-probe.mjs`（临时探针，应删）、三层生成物（要么整批 `presets generate`
   + `gen-catalog`，要么 `git restore`）。**本轮未提交任何东西。**

### ⑤ 下一步

作者点头后：另开分支，按报告第五部分的 8 刀走，第 3 刀之前不删任何老代码（每刀可单独 revert）。

### ⑥ 第二轮审查后补（2026-11-02，同日）

审查指出三处空白（崩溃恢复 / 外部冲突 / 旁路写入入口），已逐条落到代码行号，写进报告 §4.2：

- **Git 归因已定案**：`58f4295`（10-01）`rest_at` + `open_at` 读**真源**；
  `3babb63`（10-31）`open_at` 改读 `shadow_root`，`rest_at` 一行没改。
  `merge-base --is-ancestor 3babb63 58f4295` = NO ⇒ **重构中引入的回归，分界点 10-31**。
- **漏洞 A（真实缺陷）**：`save_in` 是四步（`mod.rs:2165` commit → `:2191` mirror(base)），
  ①→④ 之间崩溃 ⇒ 磁盘已新、`BASE` 仍旧 ⇒ 用户点放弃会把刚保存的退回去。
  补「提交阶段机」`prepared → real_committed → baseline_advanced` + 启动恢复表。
- **漏洞 B（真实缺陷）**：`txn::mirror`（`txn.rs:546`）逐个 `remove_file + std::fs::copy`，
  非原子、无回滚，且 `let _ = remove_file` 吞错误。而「放弃」正要用它。
  补「拷到 `.reset/` + 校验 sha256 + 整目录换名」。
- **漏洞 C（已出过事故）**：`build.rs:585` 注释自陈 —— 10-31「A1 三份 0.7 被写回 0.8」
  就是「外部改了源、影子是旧拷贝」这条链。现在只拦生成，不拦保存，也没有重启冲突规则。
  补「`diskStamp` 比对 + 冲突三选」。
- **漏洞 D**：放弃侧由 `mirror(BASE→WORK)` 天然覆盖（删多余/补缺失），**不需要按操作写回退**；
  撤销侧采纳审查意见 —— **不做 typed `Op`**，改存「受影响路径的 before/after 字节」。
- **漏洞 E**：写入入口归属表。**唯一必须改行为的是「生成」** —— 现在写真仓库再 mirror 回影子
  （`build.rs:579`、`:1630`），新模型下改成写 `WORK/delivery/`，同时解掉
  「改了个值没保存就不能生成」这个不合理限制。发布仍 `require_clean`。
- **八条硬约束**已列表（编辑/基线/放弃/保存/外部变化/结构操作/旁路写入/撤销），建议进 `AGENTS.md`。
- **刀序重排**：8 刀 → **10 刀**，机制类（原子换名=刀2、阶段机=刀5、冲突=刀6）前置于语义类。
  新增判据 16-20（组合场景：关掉再开再放弃 / 保存中途强杀 / 外部 checkout / 放弃中途强杀 / 生成时有未保存改动）。
