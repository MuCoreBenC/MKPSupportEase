# 交接台账（活）

> 更新时间：2026-10-19 · 分支：`feat/dark-mode`
> **本文只装"现在还用得上的东西"**：怎么验证、纪律、判据、仓库状态、最近的几条增量。
> 2026-10-01 ~ 10-11 那一批（第一/二/三圈 + 增量之二十五~三十五）已搬进
> [`docs/HANDOFF-ARCHIVE-2026-10-19.md`](docs/HANDOFF-ARCHIVE-2026-10-19.md)（**没删，只是挪走**）。
> 总纲：`DATA-ARCHITECTURE` / `PUBLISH-ARCHITECTURE` / `RELEASE-TRANSACTIONS` / `APP-STATE` /
> `HOOK-ERRORS-AND-MESSAGES` / `ARCHITECTURE §7.5`；全仓文档索引 `docs/DOC-INDEX.md`。
>
> **防再乱的规矩**：新东西一律写在 §3；**增量只留最近约 6 条**，更旧的搬进上面的归档文档
> （整段搬、不改字），本文只留一句指向。§1/§2 是活规则，改了要回头看有没有别处引用它的章节号。

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
| `scripts/probes/presets.mjs`（**手工**，非 CI） | 预设页探针：两轴可点 / 四张表可读 / 点行展开 / 右键菜单 / BBS 入口跨页 / **交付行的四态与动作（已下载·灰字、旧版本·「更新」、内容异常·「重新下载」）** / **存疑那两档没有「应用」也没有「改这份」、右键「另存为一份新的」带原因灰掉** / **批量那一层（批次行只含未下载+旧版本+内容异常、逐份结局各占一行且不许伪装成功）** / **我那份能被应用并说得出「基于旧版官方」** / **改我那份 → 保存回它自己（不产生第二份、血统还在）** / **第九层：读不出来的那份照常列在表里、画得出「文件无法读取」（角标带原因）、没有「应用」也没有「改这份」，能读的那份不受牵连** / **第十层：改名只动名字（坏的那份改完还是「文件无法读取」）、删除有二次确认且删完行没了、正在使用的那份「删除」灰掉带原因、改名不断「已应用」、草稿跟着走（再点「改这份」说「上次改到一半的那一份」）** / **第十一层：另存为一份新的只给「我的文件」（官方那份灰掉带原因）、名字不预填、字节复制（新那份正文带着原来改过的字）与血统原样带过去、撞名被拒不覆盖不自动改名、不碰使用中与草稿** / **第十二层：工具栏「导入文件…」选择器能进（结果条 + 列表立刻重读）、拖到窗口上有提示且重名进改名格（输入框预填原名）、改名后进来而原来那份不动、再撞被拒、取消不多出东西、`.zip` 收不了且不许被复制进来、导入不碰「已应用」** / **第十三层：右键「在 Finder 中显示」只给「我的文件」（官方那份灰掉带原因）、点了如实说失败（浏览器里没有文件管理器、真机上的样子说清楚）、不碰「已应用」** / **第十五层：分类边界（MKP 档不许有 `.svg` 与 `MKPProcess` 切片器配置、切片器档要出现 catalog 交付行且动作是「下载」、台账「仓库 N」跟着档走两档不同数）** / 控制台无 error、无 ≥400 响应。跑法见 §2.3；截图落 `tmp-shots/`（已 gitignore） |

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

## 3. 最近的增量

## 增量之三十六 · 图片引导（客户端弹窗 + 工作台标注台）：能缩放 / 缺图留空 / 源稿进仓库（2026-10-16，分支 `feat/dark-mode`）

> 交接文档：[`docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md`](docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md)
> —— 那一份写的是**下一刀**（① 图片按需下载 ② "一组/套装"的三件裁决）与不许违背的六条口径；
> 本节只记这一轮**做了什么**。

**§1 客户端那个弹窗**（`src/app/guide/`）

- 版式：图在左、**文字（状态句 + 步骤）全在右**；图 = `width/height:auto` + `max-width:min(314px,46vw)` + `max-height:min(46vh,460px)`
  ⇒ 两个方向谁紧听谁，任何窗高都完整显示、框身不滚。
- **图可点开放大**：走 `Modal` 新加的 `overlay` 槽（同一个遮罩里浮起来、不另开窗），进出 = **FLIP + WAAPI**
  （进 240ms / 收 200ms 倒放；遮罩底画在 `::before` 上、父级从不改 opacity —— 这是"缩小时会闪"的正解）。
- **放大后可缩放**：`wheel`（`ctrlKey` = 触控板双指捏合，否则普通滚轮）+ 两指 pointer；只写内联 `transform` + rAF、
  以指针为不动点、`scale ∈ [0.5,4]` 带阻尼；**每次打开复位成默认大小**。框的线 / 圆角 / 编号视觉大小恒定
  （`calc(… / var(--zoom-k))`；线改用 `box-shadow` —— `border-width` 被引擎钳在最小 1px）。
- **图取不到 = 整块留空**：`img.onError` ⇒ 与"台账里没有这条"同一条落法（不画那块、右列占满、不留裂图、不写 console）。

**§2 工作台「标注台」**（`src/workbench/views/AnnotatePage.tsx`，nav 排在「资产库」后）

- 图上拖框 / 拉边 / 方向键 / ⌘Z / 删除 + 图层列表 + 四格 px；**框的样子**三格（圆角 / 线宽 / 编号直径，缺省 8/2/18，
  与客户端**同名 CSS 变量 + 同缺省**）；选图**按组摆**（`<optgroup>` = 一组 2 张 · 共用一份标注）+ 页脚点名同组那张。
- **源稿进仓库**（作者：「像 PS 那样能再编辑、跟着项目仓库走，换设备也能接着编」）：
  `presets/assets/guides/<对>.anno.json`（与图同层），读写走 `wb_anno_read` / `wb_anno_write`
  （`src-tauri/src/workbench/app/annotation.rs`：路径由后端从台账派生、`updatedAt` 后端盖、坏值整次拒绝、
  **不进资产也不进交付**）；localStorage 退成"没写进仓库的暂存"（保存即清；**内容真的不一样**才弹冲突）。

**§3 「随包资产只兜底」收口**（"图片不见了"那次的根治）

- `runtime::catalog_for_ui` = ② + `Catalog::with_missing_bundled_from(&EMBEDDED)`
  （只补 ② 里没有的 `bundled` 条目、② 里已有的绝不动、下载面一条不补、**盘上字节一个不改**）；
  `ipc::catalog::get_runtime_catalog` 走它 ⇒ 随包的东西不再依赖任何一份云端文件。

**§4 验证**

- `cargo fmt` / 双 feature `clippy --all-targets -D warnings` / `cargo test --lib`（**550**）/
  `--features workbench --lib`（**911**，+4）/ `npx tsc -b` / `npm run lint` / `npm run build` + 三道 check。
- 探针两条：`scripts/probes/guide-image.mjs`（~70 条：版式 + 放大层 + 缩放 + 404 留空）、
  `scripts/probes/annotate.mjs`（一组 / 拖拽 / 样式 / 源稿读写）。**两条全绿**。

## 增量之三十七 · 客户端放大层可拖动 + trae 清场（2026-10-17，分支 `feat/dark-mode`）

**§1 客户端图片引导放大层：拖动平移（作者：「我希望客户端的也可以拖动，现在只有缩放还没法拖动」）**

- 病根与工作台标注台那条同款：`GuideFigure` 的 `<img>` 没禁**原生拖图**——按住一拖，
  WebKit 起的是"拖走这张图"，pointer 序列当场断掉 ⇒ 平移废了（平移的 pointer 逻辑本来就在，
  单指就是拖、双指是捏合，只是被原生拖拽截胡）。
- 治法：`GuideFigure` 的 img 加 `draggable={false}`；`.zoomLayer` 补 `user-select` / `-webkit-user-select`
  （WKWebView 只认带前缀的）+ `.zoomLayer img { -webkit-user-drag: none }`。
- 放大层提示句补全：「拖动挪位置 · 滚轮 / 捏合缩放 · 点任意处或 Esc 收起」。

**§2 trae 清场（作者：「检查一下有没有 trae 留下的记号之类的，清理掉」）**

- `.trae/documents/` 两份设计文档**搬进 docs/**：`PRESET-DATA-SHAPE.md`、`PRESET-PAGE-NATIVE-PLAN.md`
  （`git mv`，内容一字未动），`DOC-INDEX.md` §2 各补一行；`.trae/`、`.comate/`、`.zcodeignore` 删除。
- `.gitignore` 那条注释同步改口径：设计文档一律住 `docs/`。
- `PageCalib.module.css` 里几处 "trae-counter" 是**出处注**（那几条玻璃样式的来源），保留；
  `HANDOFF` / `UpdateIndicator` 里"像 Trae 那样"是作者的原话引用，保留。

**§3 「去重新允许」按钮（同日作者追问：「如果用户后悔了，有再次触发的开关吗」）**

- macOS 的死规矩：**拒绝过一次就永远不会再弹**授权框（TCC 没有"再问一次"的 API）⇒
  反悔的唯一正路是系统设置 → 自动化里把「访达」勾回来。
- 落法：`ipc::slicer::open_automation_settings` 用系统直达 URL
  （`x-apple.systempreferences:com.apple.preference.security?Privacy_Automation`）
  开**那一页**（依旧不替他勾）；前端只在错误码 `PERMISSION_DENIED` 时在引导的
  `statusExtra` 槽摆一颗「去重新允许（系统设置 → 自动化）」+ 小字
  「勾上「访达」后，回来再点一次「去系统里改…」就行」。
  命令注册两份清单都加了（`lib.rs`）；mock 照实拒（浏览器没有系统设置）。
- 验证：`tsc -b` / 双 feature clippy / `cargo test --lib`（558）/ `--features workbench`（934）/
  `npm run lint` / `npm run build` + 三道 check。全绿。

## 增量之三十八 · Windows 构建补平台兜底：`zorder` 跨平台 + `slicer` 名字类型归位（2026-10-18，分支 `feat/dark-mode`）

**病根：拉取最新（`b97f4e7`）后 Windows 上 `npm run tauri dev` 编译不过（5 错）**

- `error[E0433] cannot find zorder` ×3（`ipc/guide.rs` 218 / 631 / 640）：`zorder.rs` 整文件挂着
  `#![cfg(target_os = "macos")]`，Windows 上这个模块是**空的**，而 `ipc::guide` 无条件引用
  `crate::zorder::SlicerWindow` / `slicer_window` —— 文件级 cfg 没给别处留兜底。
- `error[E0308]` ×2（`ipc/slicer.rs:282`）：`known_label` 回 `Option<&'static str>`、
  `app_stem` 回 `String`，`unwrap_or_else` 两侧不同型；下游 `SlicerApp.name` 要 `String`。

**治法（沿用仓里既有的「平台兜底」双 cfg，见 `ax.rs` / `ipc::guide`）**

- `zorder.rs`：删掉文件级 `#![cfg(target_os = "macos")]`，改为只把 `mod mac`（CoreGraphics 那半）
  挂 `#[cfg(target_os = "macos")]`；`slicer_window` 拆双 cfg，非 macOS **恒 `None`**
  —— 调用方自然退回 AX 口径（本就没有"层序"这一说）。文件头「兜底」节补一句非 macOS 说明。
- `lib.rs`（`pub mod zorder` 旁注）：口径同步改成「窗口列表 API 只有 macOS 有（`mod mac` cfg 掉了）；
  其它平台 `slicer_window` 恒 `None`」。
- `ipc/slicer.rs`：`known_label(&path).map(str::to_string).unwrap_or_else(|| app_stem(&path))`。

**验证**

- `cargo check --no-default-features`（Windows）：绿。仅剩 8 条 macOS-only 死代码 warning
  （`ax.rs` / `guide.rs` 里那几张表和函数在别的平台上本来就编不出来，非本次引入）。

## 增量之三十九 · 校准模型档：几何搬进 `presets/calibration_models.toml`，旧版/加长版并存（2026-10-18，分支 `feat/dark-mode`）

**病根：作者把自己那套 Z 校准模型的打印区加长了，后处理直接把正常模型拦下来**

- 改动：低沿往下延长 1mm、高沿往上延长 2mm（中间圆点没动）⇒ 打印区网格 120.000 × 13.484 → **16.484**，
  切片后实测外形 119.58 × 13.063 → **119.58 × 16.063**（宽不变）。实测证据：旧模型 `presets/assets/models/ZOffset_Calibration.3mf`
  打印区相对模型 bbox 低沿 11.417..24.901，作者那份 STL 是 10.417..26.901（低沿 −1、高沿 +2），X 区间逐位相同。
- 后处理报 `E_CAL_MISMATCH_002`「检测到校准模型尺寸已变化」：标称外形与图案基准原来**写死在
  `disk/calibration.rs` 的常量里**（`Z_PATTERN_MODEL_WIDTH/HEIGHT`、`Z_PATTERN_OFFSET_X/Y`、`Z_SQUARE_*`、`XY_PATTERN_*`）
  —— 也就是说「同一台机器换一块校准板」= 改代码重发版。

**治法：一档一条数据；认档按实测外形尺寸自动匹配**

- 新数据文件 **`presets/calibration_models.toml`**（① 开发源层，单文件 `[[models]]` 多档）：每档写标称外形 + 容差、
  图案基准（`datumX/datumY`）、方块间距/数量、XY 模板七个偏移、质心旋转判据的期望。首批四条：
  `precise`（XY 50.5×50.5，数值逐字照搬旧常量）、`zoffset`（119.58×13.063）、**`zoffset-ext`（119.58×16.063）**、
  `repetition`（只给基准，L 形件不做尺寸校验）。
- 读取并进 `machine_dims`：`MachineTables.calibration_models` + `load_presets_dir` 顺手读它，
  **与机型表同一条逃生链**（显式 `install` → `MKPSE_PRESETS_DIR` → 仓库相对 `presets/`）。
  `calib_models::load` 把坏表挡在装载期：`deny_unknown_fields`、逐档点名缺键（ZOffset 缺 `datumY`、XY 缺 `pattern`）、id 撞名。
- `disk/calibration.rs`：删掉那一组常量，改为 `resolve_profile(mode, bbox)` 认档 —— 同一朝向命中就用该档；
  宽高互换命中 ⇒ `E_CAL_MISMATCH_001`（旋转）；都不命中 ⇒ `E_CAL_MISMATCH_002`，**正文把已知档位的外形尺寸列出来**
  （缩放还是换档，用户自己就能看出来）。基准/间距/数量/质心期望全部读档。
- 新档的基准：`datumX` 不变（0.079）；**`datumY = 3.755 + 1 = 4.755`** —— 包围盒低沿往下移了 1mm，而图案要落在
  同一个物理点上；方块间距 11.0 / 数量 11 / 质心期望不变。写成 3.755 的话模型照样认得出、图案却整体印偏 1mm，
  所以判据 `z_new_breaks_the_extended_model_into_its_own_datum` 专门钉这一点。
- 资产与前端：`presets/assets.toml` 加 `zoffset-calibration-ext` —— **正式 3mf 已入库**
  （159132 字节、sha256 `b22822cb…`；实测打印区网格 120.000 × 16.484 ⇒ 切片预期 119.579 × 16.063，
  与 `zoffset-ext` 档的 119.58 × 16.063 一致，占位那次已换成真字节）。
  重算 catalog（`cargo run --bin gen-catalog`：随包 25 份文件；交付面 revision `6374b7c4…`）；IPC `MODEL_DEFS` 加 `z-ext`；
  校准页「打开模型」模态框加「模型版本」（旧版 / 加长版）两颗 —— 选版本只换要打开的那份 3mf，
  板面预览两版共用一张（版式相同、只差 3mm 深）。mock 同步补一条。
- **登记的边界**：① 这两份档位的 3mf 是同一块板的两个尺寸版本，后处理**只按外形尺寸认档** ——
  旧模型整体放大 3mm 会被认成加长版（作者 2026-10-18 认可，记为已知局限）；
  ② 校准模型档走的是机型表那条读取链，`machine_dims::install()` 在发布物里的接线（M5 欠账）也是它的前置。

**验证**

- `cargo test -p mkpse-postprocess`：**全绿**（208 lib + 各集成套件，含 `tests/reference_calibration.rs` 的
  逐字节判据 —— 旧档数值逐字照搬，所以「迁移零变」有据）；新增档位用例：加长版认档 + 基准生效、认不出时列清单。
- `npm run build`（`tsc -b` + vite）、`npm run lint`（eslint + stylelint）、`cargo check -p mkp-support-ease`：绿。
- 剩一条**真机验收**：拿作者那份 `.44432.0.gcode` 跑一遍后处理 —— 应不再报 `E_CAL_MISMATCH_002`，
  且插入的 11 个方块相对旧版同参数**只在 Y 基准上平移 1mm**。

## 增量之四十 · 自定义数据源认仓库 / 分支页（2026-10-18，分支 `feat/dark-mode`）

**症状**：设置页「预设数据源 → 自定义地址」贴 `https://github.com/MuCoreBenC/MKPSupportEase/tree/feat/dark-mode`
点「应用」→「这个地址取不到预设数据，没有改，现在用的还是 GitHub」；校准页「打开模型」于是报
「目录里没有 ZOffset_Calibration_ext.3mf」—— 客户端读的一直是 `main`，加长版只在分支上。

**根因**（两层，第二层才是真原因）：

1. 网页地址 → raw 的转换本身是通的（2026-10-16 那条），`tree/feat/dark-mode` 能变成
   `raw.githubusercontent.com/…/feat/dark-mode`；
2. **但 `source.json` 从来不在仓库根**，它在 `presets/delivery/` 下面 —— 候选表只试了
   `<ref>/source.json`（真机 404），所以必然失败。`curl` 实证：根 404、`presets/delivery` 200；
3. 附带隐患：落盘只有 `customUrl + customShape`，重启后要**按形状再推一遍**才解析得回来 ——
   推导是第二套判据，推不回来就是"设置页看着成、实际读的还是 main"。

**治法**：

- `runtime::source` 加 `CustomShape::Repo`（仓库 / 分支页）+ 约定常量 `REPO_DELIVERY_DIR = "presets/delivery"`；
  候选表顺序 = 按约定（`…/<ref>/presets/delivery/source.json`）→ 仓库根 → 交付目录。
  `tree/` 后面**不假设**第一段就是 ref（`feat/dark-mode` 与"ref + 子目录"在 URL 上同形，
  两条解释都留给候选表）；人已经指到交付目录本身或指着某个文件就不再猜。
- 探通的那一份**存进设置**（`PresetSource::custom_resolved`，加字段不升号）：解析第一依据是它，
  `customShape` 只在老档兜底（`bootstrap_candidates` 稳定排序把记下来的形状排最前 ——
  推导仍只有一张候选表）。用户填的 `customUrl` 原样留着，界面显示与「查看更新」跳转都用它。
- 设置页文案 / 占位符补一句：仓库 / 分支页（`…/tree/<分支>`）也认。

**判据**（`runtime::source::two_source_tests`）：
`a_tree_url_tries_the_repo_delivery_dir_first`（候选顺序 + 指到子目录不再猜）、
`a_branch_page_is_probed_and_remembered`（探 → 落盘 → 重启读入口，实际读的那一份跟着走）。

**顺手修掉三笔既有红**（都是 `eaba9fb` / `8e6ba47` 带进来、CI 会拦的；作者看完报告说「可以修」）：

1. `crates/postprocess` 的 clippy `collapsible_match`（`eaba9fb`）：`calib_models.rs` 的
   `"Repetition"` 臂是「只有一个 `if` + `return`」⇒ 合并成 match guard，行为等价；
2. `cargo fmt --check` 四处（`eaba9fb` 三处：`calib_models.rs:219`、`disk/calibration.rs:818`、
   `postproc/mod.rs:13` 的 `mod` 字母序；`8e6ba47` 一处：`zorder.rs` 的块注释缩进）——
   **块注释那处 1.97 与 1.99 的要求相反**（对齐 `/*` vs 拉回代码缩进），所以改成 `//` 行注释，
   两个 rustfmt 版本才都认；
3. 工作台四条判据的期望值还停在「3 个模型」（`assets.rs` 总条数 31 / model 3 / 可选 10、
   `delivery.rs` 目录 15 条）：`eaba9fb` 加进 `zoffset-calibration-ext` 之后应为 32 / 4 / 11 / 16。

**验证**：`cargo test`（工作区，默认 feature）**全绿**；`cargo test -p mkp-support-ease --features workbench --lib`
**966 全绿**；`cargo clippy --all-targets -- -D warnings` 两档 feature（默认 / workbench）+ `+1.99.0` 全 **exit=0**；
`cargo fmt --all -- --check` 在 **1.97.1 与 1.99.0** 下都干净。

**真机待验收**：设置页贴分支页 → 「当前：你指定：…/tree/feat/dark-mode」→ 校准页「从云端获取」
拿到 `ZOffset_Calibration_ext.3mf` → 「打开模型」能开。

## 增量之四十一 · 交付错误带码 + 内容区给"怎么办" + 开发台故障注入（2026-10-19，分支 `feat/dark-mode`）

**三句话**：① 用户能读到的句子必须带稳定码；② 只给错误标题不够，内容区要给**怎么办**；
③ 开发台要能**真实地**触发任意模态框的错误态。

### §1 错误码与文案瘦身（`src-tauri/src/error.rs` / `runtime/delivery.rs`）

- `AppError` 新增 `messageCode`（稳定码 `E_*_NNN`）+ `AppError::text()`（人话 + 尾注码）
  + `text_err()`；`ipc::traced` 交前端的 `message` 从此**自带码** ⇒ 前端所有 `errorText()`
  调用点天然带码，**不用逐处拼**（两份都带就成了「（码）（码）」）。
- 交付链先盖三个码：`E_DELIVERY_STALE_001`（目录过期）/ `_SHA_001`（文件真坏）/ `_SIZE_001`（大小不对）。
- **文案瘦身**（作者：「这段话这么长」）：界面那句一个技术值都不摆 —— 两串 64 位 SHA 与两个
  revision 全部下线到 `detail` + `tracing::warn!`。原来那句把排查用的因果解释塞给了用户。

### §2 内容区的「怎么办」（前端）

- 前端 `src/api/errors.ts` 出 `UiError{text,code}` + `uiErrorOf()`；两页 `modelErr` 从
  `string | null` 改存整条。
- 新增 `src/app/shared/staleCatalog.ts`（`STALE_CATALOG_CODE` + `isStaleCatalogError`）
  + `StaleCatalogHelp.tsx`（内容区「怎么解决」块，绿 `--ask-*`，**只给动作不解释成因**）。
- ★ **按码出现、不按文案猜**：目录过期 → 给"重启"这条路；文件真坏 → 重启没用，**不许给**
  （指错路比不指更糟）。判据是那个码，不是句子。
- 两页（校准页 / 首页）同时删掉自挂的 `window keydown` 关 Esc —— `Modal` 自己就接 Esc，
  原来一次 Esc 会连关两层。`ExportModal` 那条重复的框身红字删掉，只留标题格。
- 判据/常量与组件分两个文件：`react-refresh/only-export-components` 要求 `.tsx` 只导出组件
  （同 `note.ts` / `NoteBar.tsx` 那套）。

### §3 开发台 · 故障注入（**真机制**，不是 UI 预览）

- **挂在 API 方法上**（作者选的粒度）：一个开关覆盖所有走这个方法的界面，产品代码一行没改。
- `src-tauri/src/devfault.rs`（**无条件编译**，与 `devsim` 同路 —— 造错发生在客户端进程，
  而客户端不带 `workbench` feature）+ `workbench/app/devfault.rs`（三条命令薄壳）。
- 过线：`FromHook::DevFault { fault: Option<String> }` —— **只过 id，不过 `AppError`**，
  错由客户端进程 `devfault::armed()` 自己造 ⇒ **演示的错与真机那条一字不差**
  （为此把 `delivery` 那三个句子/码函数提成 `pub(crate)`，有测试钉住这条）。
- 四档：`delivery-stale` / `delivery-sha` / `delivery-size` / `open-model-stale`。
- 工作台 `DevPage` 加了「故障注入」区（装 / 卸同一颗按钮）。加一档只改 `devfault::faults()`。
- ★ **诚实边界**（作者强调）：它拦在 API 调用层，**不代表真实业务真的执行失败了** ——
  被拦的那次调用可能根本没进后端。这是开发测试用的故障注入，不是真去破坏文件或网络。

### §4 未做：后处理预览档（**只处置了一半，剩下的另开交接**）

- 查证到的硬约束：`HookView::cancel()` → `write()` 在 `!hook_connected()` 时报
  「这一趟已经结束了，没有正在跑的处理」；而演出时钩子根本没连 ⇒
  **那几颗真业务按钮点了必然报错**。所以 10-14「预览里不摆真业务按钮」那条裁定站得住，
  **不是"忘了接"，是接不上** ⇒ 「换成真按钮」不可行。
- 作者的判断（接受）：**不是实现不了真的，而要分清"已实现成真功能"与"只是演示场景"**。
  故障注入属于前者，后处理预览属于后者。真正要的是**从工作台触发真实后处理流程** ——
  与"后处理预览档"是两回事。
- 撤预览档（作者选的方向 A）**本轮没做**：建议等真实入口做完再撤（有对照物）。
- ★ 交接文档：[`docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md`](docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md)
  —— 需求 / 六条验收判据 / 三步路径 / **待核三件事**（真实入口在哪、A1 基准 G-code 在哪、
  现有 hook 协议够不够起一趟真任务）/** 未验证的假设**（工作台直接 spawn 真钩子，可能不必加协议变体）。

**验证**：`cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings`（默认 /
`workbench` 两档）、`cargo test` **598 全绿** / `--features workbench --lib` **975 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` 全绿。

## 增量之四十二 · 开发台「运行真实后处理」（② 落地，2026-10-19，分支 `feat/dark-mode`）

> 接着增量之四十一 §4 那份交接做。交接文档 `docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md`
> 的 **§4.3 已核完、§6 那个假设已验证成立、§4.4 是这一刀的落地记录**。

**一句话**：工作台「开发台」第二块 = **真的跑一趟后处理** —— 选一份预设 + 一份测试 G-code，
工作台 spawn 一个**真的钩子子进程**，客户端窗口跑真管线、显示真进度，界面上**没有"预览"标记**，
那几颗真业务按钮**点了真的有用**。

### §1 先核的三件事（交接文档 §4.3）

1. **真实入口** = 本程序带参数起子进程：`"<本程序>" --Toml "<预设.toml>" --Gcode <输入.gcode>`
   ⇒ `args::Mode::Hook` ⇒ `hook::run_with_channel`（真跑 `process_with_ir_with_tower_ask`，
   产物**原地覆盖**输入，归档真写 `user/gcode_history/<日期>/`，退出码 0/1/2）。
   ★ 顺带澄清：**内核 `mkpse-pp` 不在这条路上** —— 它是诊断 CLI，钩子是进程内调管线、不 spawn 它。
2. **那份 A1 基准（约 23.8 MB）没找到**。本机最大真件是
   `crates/postprocess/tests/fixtures/real/.5926.1.gcode`（**8.5 MB**，BambuStudio 切的 A1 件）。没新造测试数据。
3. **hook 协议够用 ⇒ 一个变体都没加**。真钩子本来就是独立进程（假钩子 `src/bin/hook-sim.rs`
   就是先例），靠 `internal_root_headless()` 认客户端那棵树连 loopback ⇒ 工作台 spawn 一个带
   `--Toml/--Gcode` 的子进程即可，客户端收到的就是真 `Hello/Progress/Question/Finished`，
   `preview` 恒 false。**客户端侧与产品逻辑一行没改。**

### §2 落点（全是工作台这一侧的薄壳）

- `src-tauri/src/workbench/app/devreal.rs`（**新**）：`wb_dev_run_real(toml, gcode)` ——
  ① 校验两个文件 ② 复制副本到 `<客户端内部根>/run/dev-run/` ③ `connect_or_headless` 探客户端在不在
  ④ spawn `current_exe() --Toml --Gcode`（不等）。含 5 条单测。
- `workbench/app/mod.rs`（模块声明）+ `lib.rs`（命令清单，`command_list_parity` 那道闸）
- `src/workbench/api.ts`（`DevRunAck` + `devRunReal`）+ `views/DevPage.tsx`（新区块）
- `src-tauri/capabilities/workbench.json` 补 `dialog:allow-open`（工作台要开系统文件选择器）

**四个取舍**（都写在 `devreal.rs` 模块头）：spawn `current_exe()`（与真机同形，产品同名产物、
都走 Hook 分支）；**必须先复制副本**（真后处理原地覆盖，不复制则第二趟跑的是已处理产物，
"可重复跑"当场失效）；**先探客户端再 spawn**（不替用户拉进程）；stdio 全 null。

### §3 实测（次日真机，8.5 MB 那份 + `A1-fastv3.3.toml`）

```text
处理完成：…/run/dev-run/1792355145-5926-real.gcode（8435 ms）
钩子来了：这一趟开始 gcode=… preset=A1-fastv3.3.toml        ← 客户端 hook_ui 真路径（不是 run_preview）
钩子那一趟结束 exit_code=0 ok=true cancelled=false
归档：user/gcode_history/2026-10-19/…{,.gcode,_meta.json,_original.gcode}
```

**六条判据**：真任务 ✅ / 真界面 ✅（走 `hook_ui` 真路径，`preview` 恒 false）/
真按钮 ⚠️（**链路已成立**：真有一条连接 ⇒ `Handle.connected()` 为真 ⇒ 不再报"没有正在跑的处理"；
最后一步要在客户端点一次「停止」**人工确认**）/ 够慢 ❌（**8.4 s**，一闪而过）/
可重复 ✅（副本机制）/ 不进正式包 ✅（`workbench` feature）。

★ 一个容易误会的事实：**工作台的沙箱管不到这一趟** —— 沙箱是模块级 `AtomicBool`、不写环境变量，
子进程不继承 ⇒ 钩子进程走**客户端正式那棵数据树**，会真写执行记录与归档。界面已写明"别拿正式数据试"。

### §4 待拍板（交接文档 §7）

1. ✅ 三件事已核完 ⇒ 不加协议变体。
2. ⏳ **撤预览档**：可以撤了，但建议**先留着当对照物**（真 vs 演，一眼看出"预览"标记与按钮差别）。
3. ❌ **减速开关要不要做**：8.5 MB 只 8.4 s，**不够慢**；release 只会更快。三条路 ——
   ① 换更大的件（要 30 s+ 得 30 MB+，作者那份 23.8 MB 没找到）；
   ② `#[cfg(feature = "workbench")]` 的 `--dev-slow-ms`（只在进度事件之间 sleep：只改时序不改状态，
   正式包编不到，与 `DevScene`/`DevFault` 住产品文件同一类格局）；③ 不做。
   **我倾向 ②**，但它要在 `args.rs`/`hook.rs` 开 workbench-only 分支 —— 不算零成本，**等你点头**。

**验证**：`cargo fmt --all --check`、双档 `cargo clippy --all-targets -- -D warnings`（默认 /
`workbench`）、`cargo test -p mkp-support-ease --lib --features workbench` **981 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `npm run build:workbench` 全绿。

## 增量之四十三 · 开发台「真实后处理测试矩阵」六档（2026-10-19，分支 `feat/dark-mode`）

> 接着增量之四十二（② 单趟入口）做的第二层。作者：「我怎么触发它成功、怎么触发它失败、
> 怎么触发它碰撞检测？模拟出来的真实可以做到吗？落盘的话，也到时候是用临时文件」。
> 交接文档 `docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md` **§8** 是需求 + 方案 + 落地台账。

**一句话**：六档确定性输入各跑一个**独立临时根**，串行、跑完只按 `_meta.json` 真凭据判定；
问句档（不匹配 / 碰撞 / 取消）**要人在客户端窗口真操作**，数据一个字节不碰正式树。

### §1 六档与实测

| 档 | 判定（只读 `_meta.json`） | 实测 |
| --- | --- | --- |
| 小文件成功 | `error=null && !cancelled` | 101 ms / 9 步 complete |
| 大文件成功 | 同上 | 8435 ms |
| 硬失败 | `error` 含 `E_` | 坏 TOML ⇒ 退出码 2 + `E_TOML_PARSE_001` |
| 挪塔碰撞 | **`relocation != null`**（没有就判失败，不看退出码） | `relocation.choice=timeout`、冲突点 `[30,30]`、建议 X 挪 22.8mm、`W_COLLISION_RELOCATE_002` |
| 机型不匹配 | 人工档：非 `E_` = 问过了 | 待客户端在听时真答一次 |
| 取消 | `cancelled && W_SYS_CANCELLED_001`，别的都不算 | 待人工点停止 |

### §2 落点

- `src-tauri/src/workbench/app/devmatrix.rs`（**新**）：档位表 / `prepare`（临时根 + 抄端口 + 抄偏好
  + 复制夹具）/ 串行 worker（`CURRENT` + `BUSY` 两个静态 + `Child::wait`）/ `judge` / `recent_verdicts`
  / 清空（`BUSY` 时不删）。6 条单测（碰撞要有 relocation、取消只认那个码、没记录一律失败、成功要干净、硬失败要 `E_`、档表 6 档不重）。
- `devreal.rs`：抽出 `spawn_hook(toml, gcode, data_root)` —— **矩阵"数据落临时根、消息发客户端"的全部秘密**
  就一个字：`data_root/run/hook.port` 里写的是客户端端口（端点只是一行文本）。单趟入口也改用它。
- 四条命令：`wb_dev_matrix_cases` / `_state` / `_start`（空数组 = 全部）/ `_clear`。
- 前端：`api.ts` 三个类型 + 四条封装；`DevPage` 加「真实后处理测试矩阵」区块（2 秒轮询，不走事件推送）。
- 夹具：`crates/postprocess/tests/fixtures/dev/{small-ok.gcode,collide-tower.gcode,broken-preset.toml,README.md}`。

### §3 三条坑（都进了夹具 README / §8.4）

1. **缺 `printer_model` 不是硬失败**（管线宽容，111 ms 跑完判成功）⇒ 硬失败档改用坏 TOML。
2. **`rib-tall-min.gcode` 不会撞**（没有 `; FEATURE: Support` ⇒ 擦料决策判不需要 ⇒ `use_towers=false`
   ⇒ `detect_tower_collision` 首行 return）；新夹具补上那一行才真撞。
3. **客户端没在听时，机型不匹配走 `Refuse`** ⇒ `E_CFG_MISMATCH_001` 直接硬错、没有问句；
   矩阵会正确地判失败 —— 所以跑矩阵前客户端必须真在听。也印证"只看 `_meta.json`"是必须的。

★ 另一个环境事实：工作台用 `CARGO_TARGET_DIR=target-workbench`（`scripts/tauri-workbench.mjs`），
与客户端的 `target/debug/` 隔离 —— 但**别手动 `cargo build --features workbench`**（会覆盖客户端产物）。

**验证**：`cargo fmt --all` 干净、双档 `clippy -D warnings` 绿、`cargo test` 绿、
`cargo test -p mkp-support-ease --lib --features workbench` **987 全绿**（981 + 6 条矩阵判定）、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `build:workbench` 全绿。

## 增量之四十四 · 校准件接进矩阵 + 问句屏「停止并关闭」（2026-10-19，分支 `feat/dark-mode`）

> 交接文档 §8.5 是这一刀的完整记录。作者拍板：校准件用**仓库现成的 3MF**；
> 「点下去已经晚了」按**加一颗「停止并关闭」**解决（不做排队）；**预览档这一轮不删**，
> 只把「成功·没有结果读数」「成功·好几行警告」两格删掉。

### §1 校准件是怎么"真跑"起来的

1. **身份**：临时根摆成切片器形状 —— `<根>/input/plate.gcode` + `<根>/origin.txt`（一行 = 3MF 绝对路径）。
   `temp_root()` 取 **gcode 的上一级**，所以 `origin.txt` 放 `input/` 的上一级。
   实测记录：`detail.calibration.identity3mf = "ZOffset"`、`verdict = "from_3mf"`。
2. **切片件**：本机 Bambu Studio **有 CLI** ——
   `BambuStudio --slice 0 --outputdir <OUT> presets/assets/models/ZOffset_Calibration_ext.3mf`
   ⇒ `plate_1.gcode`，挤出 bbox **119.511 × 16.063**，正落 `zoffset-ext`（119.58 × 16.063 ±0.5）。
   已落仓 `fixtures/dev/calib-zoffset-ext.gcode`（121 KB），**运行时不再依赖外部工具**；
   重生成方法写进夹具 README（`--export-3mf` 会报错并给 243，不影响 `--slice`）。
3. **两档判定**：`calib-ok` 要 `error=null` **且** `mode`/`profileId` 都在；
   `calib-question` 要 `W_` + `cancelled=true`（实测 `W_CALIB_MODE_001`）。

★ 反直觉：**「问了没答」的停下，`cancelled` 也是 true**（原文件没动）⇒ 这一档不能照取消档判。
★ 顺手修掉真 bug：`err.contains("E_")` 会把 `W_CALIB_MODE_001`（"MOD**E_**001"）误判成硬错 ⇒
  改成 `is_hard_error`（连括号认 `［E_` / `[E_`）。

### §2 「点下去已经晚了」从死局到一键收场

病根不是缺排队，是**报错之后没出路**：点了选项 ⇒ 问句屏让位给 hero（防连点）⇒ 这一答晚了一步
⇒ 红字落在标题右边、问句屏已经收走 ⇒ 既没按钮也关不掉框。

- 新增 `src/app/postprocess/questionUi.ts`（**纯判据**）：
  `showAsk = asking && (!answering || err !== null)`、
  `offerStopAndClose = running && err !== null`。
- `PostProcessModal`：报错时问句屏**放回来**；报错且还在跑 ⇒ **只摆一颗「停止并关闭」**
  （先 `cancelPostProcess()` **吞掉失败** —— 钩子可能已经没了，这颗按钮的语义是"收场"不是"保证停成功"
  —— 再 `dismiss()`）；`answering` **不再因 `err` 松手**（否则按钮闪回来、人接着点接着错）。

### §3 删两格预览场景

`postprocess-success-no-metrics`（真跑一定算得出读数，摆着会误导）与
`postprocess-success-long-warning`（要凑内核警告条件、且那是排版问题）。
`devsim::scenes()` / `src/app/dev/scene.ts` / `hook_ui.rs` 三处同步删；
`devsim::Options` 的 `metrics` / `many_warnings` 两个**能力留着**（演出引擎不因少两格就拆）。
其余 13 格一条未动。

### §4 未验的与下一步

- `calib-ok`（认成校准件 + 认到档 + 跑完）要在客户端**真答一次** —— 这一"答"没法自动化。
- 「停止并关闭」需在真机制造一次"答晚了"（例如碰撞档等超时后再点）。
- 下一轮：等这两条真机验完，再撤预览（作者：「等校准件到位再撤」）。

**验证**：`cargo fmt --all -- --check`、双档 `clippy -D warnings`、`cargo test`、
`cargo test -p mkp-support-ease --lib --features workbench` **989 全绿**、
`npx tsc -b --force`、`npm run lint`、`npm run build` + `build:workbench` 全绿。

## 增量之四十五 · 撤掉后处理「预览」整组（2026-10-19，分支 `feat/dark-mode`）

> 作者：「把预览全部取消吧」（截图为证：开发台里那 15 格预览还在，而矩阵那一块当时跑的是
> 旧的工作台进程，所以看不到校准两档 —— 代码里其实已经有了）。
> 前置条件已满足：校准件两档真入口做通了（增量之四十四）。

**撤掉的九格 + 两处前端钩子**（都在 `devsim::scenes()` 与两处前端）：

| 撤掉的预览格 | 真入口里怎么看到 |
| --- | --- |
| 成功 / 失败 / 已停止 / 机型不匹配那一问 | 矩阵 `small-ok` / `big-ok` / `hard-fail` / `mismatch` |
| 那一问 · 等超时 | 矩阵 `mismatch` 不理它，两分钟自己收场 |
| 那一问 · 点下去已经晚了 | 真机就会遇到；报错那一屏现在有「停止并关闭」 |
| 跑着 · 收进后台 | 真跑起来点那颗「后台运行」 |
| 校准件 · 问怎么跑 / 等超时 | 矩阵 `calib-ok` / `calib-question`（真 3MF 切片件） |
| 校准件 · 先打印成功 / 直接校准成功 | 矩阵 `calib-ok` 答完就是那两屏 |
| 碰撞 · 问要不要挪塔 | 矩阵 `collision` 档（真撞） |

**唯一留下的**：`postprocess-running`（处理中·停住不动）—— 当时理由是它**没有真实等价物**
（真跑那一屏会自己往前跑）。

### 追加（同日）：这一格也撤了

作者看完截图说「这个不需要了吧」⇒ **后处理那一组一格都不剩**。
定格的需求改由矩阵 `big-ok`（8.5 MB / 约 8 秒）或直接对真跑那一屏截图承担，
不再维护一条只为了"停住"的演出路径。

判据再次反转：`the_postprocess_group_is_gone_from_the_dev_desk`（`pp.is_empty()`）。
演出引擎（`crate::devsim` 的 `Mode` / `Options` / `drive`）**一行没拆** ——
假钩子 CLI（`src/bin/hook-sim.rs`）还在用，撤的只是"开发台上架的那份清单"。

**配套清理**：`src/app/dev/scene.ts` 层类表只剩那一格；
`PostProcessModal` 撤掉 `useDevPreviewNonce('postprocess-answer-stale' | 'postprocess-backgrounded')`
两处开发台钩子（`setErr` / `dismiss` 本身是产品行为，**没动**）。

**判据反过来了**：`the_postprocess_group_is_keep_only_the_frozen_running_frame`
从"四档都在"改成"四档**都不在**、只剩一格" —— 防止以后有人顺手把某一格加回来"方便调试"。

**教训（给下次的自己和用户）**：改完开发台/矩阵**要重启工作台**（`npm run tauri:workbench:dev`）
才看得见新档位；工作台用的是 `target-workbench/` 那套产物，不随客户端的 `target/debug/` 更新。

**验证**：fmt / 双档 `clippy -D warnings` / `cargo test` /
`cargo test -p mkp-support-ease --lib --features workbench` **989 全绿** /
`npx tsc -b --force` / `npm run lint` / `npm run build` ×2 全绿。

## 增量之四十六 · 补 `tauri:workbench` 入口：`npm run tauri:workbench dev` 现在能用了（2026-10-19，分支 `feat/dark-mode`）

**症状**：`npm run tauri:workbench dev` 报 `npm error Missing script: "tauri:workbench"`。

**根因**（不是设备问题，任何机器一样）：npm 的 `npm run <script> [args]` 里，
**脚本名之后的一律当参数透传**（本次已用 `npm run tauri info` 实证 —— 跑的就是 `tauri info`）。
仓库里 `tauri` 这个名字**存在**（`"tauri": "tauri"`），所以 `npm run tauri dev` 能用；
但工作台**从来没有** `tauri:workbench` 这个脚本 —— 它的入口一直叫 `tauri:workbench:dev`
（`npm run tauri:workbench:dev`）。即"照 `npm run tauri dev` 的形状写"少写了一位。

**修法**（只加、不动既有）：`package.json` 补一条
`"tauri:workbench": "node scripts/tauri-workbench.mjs"`。此后两种写法都成立 ——
`npm run tauri:workbench dev` / `npm run tauri:workbench build`（与 `npm run tauri dev` 对称）
和原有的 `npm run tauri:workbench:dev` / `:build` 都照旧，既有习惯不受影响。

**验证**：`npm run tauri:workbench` 命中脚本并打印用法（退出码 2，符合脚本预期）；
参数透传已由 `npm run tauri info` 实证，故 `... dev` / `... build` 会分别转发 `dev` / `build`。

## 增量之四十七 · 参数编辑「跟手化」：本地即时 + 单飞合批 + 橙色「改之前的值」（2026-10-19，分支 `feat/dark-mode`）

**症状**（作者实测）：「改个参数它一直在跳 —— 很慢很卡；我连续点击，停下之后它才慢慢追上我点击的次数。」

**根因**：一次值写入 = `ParamsPage.writeValue` → 外壳 `run()` → `await wb.applyDraft(...)`，紧接着 `setTick(n + 1)` 与 `refreshReport()`。
`tick` 又让 `ParamsPage` 三个 effect 各再发一条 IPC（`wb_registry` / **整页 `wb_desk`** / `wb_matrix`），而 Rust 侧这些 `#[tauri::command]`
**全是同步的（跑主线程）**，`wb_preflight` 更重（parse recipes + 摸盘建 catalog）。
连点步进器时主线程被这一串命令占死 ⇒ 数字不跟手、停下才"追账"。

**改法**（前端为主，**后端草稿那条架构一行没动**）：

1. **`src/workbench/editQueue.ts`（新）** —— 值写入的单飞合批队列 + 乐观覆盖层。
   同一格 latest-wins；**真尾随 120ms**（每次 push 重新计时，人还在连点就一条都不发）；**单飞**（同一时刻只有一条 `applyDraft` 在飞）；
   响应**只写 base**、overlay 永远最后叠；回来先比 `refreshKey`，对不上（切了机型 / 改了搜索）就是过期派生，丢弃。
2. **外壳 `run(..., { quiet: true })`**（`App.tsx`）—— 值写入那一路**不拨 `tick`、不置 `busy`**；`refreshReport` 改 `refreshReportSoon`（700ms 尾随去抖）。
   `undo` / `redo` / `save` / `discard` 仍是低频结构操作、照旧拨 `tick`，但动手前先 `await settleAll()` 把飞在空中的写落完
   （`save` / `discard` 另读 `bookRef` 判脏 —— 否则"刚点完步进器马上点保存"会**静默丢一笔**）。
3. **参数台吃 `out.desk` / `out.matrix`** —— 后端 `wb_apply_draft` 本来就按 `refresh` 顺带返回（以前只吃了 matrix），省掉事后再问一次。
4. **`NumberField`** 两条：手势进行中不回写 prop（`gestureRef`）；本地跑在 prop 前面时也不回写（`aheadRef`）。
   少了这两条，**连点 10 下只进 8**（被动的 `[value]` 回写 effect 会晚一步落在两次点击之间，把 `numRef` 拽回上一步 ⇒ 下一次 `commit()` 当"没变"不提交）。
5. **橙色「改之前的值」** —— `cell.dirty && cell.rest !== null` 时，中间行与右栏「各版本取值」各画一枚 `↩ <改前那个数>`（新增 `.pRevert`，
   复用两边同源的 `--warn-*` token），点了写回 `Cell.rest`（盘上钉着的那个）。与右栏那枚 `↶`（挂回继承、写 `null` 删键）**语义不同**，title 里说清。
6. **其余页** —— `AssetsPage.applyVis` 改走 quiet（它写完自己就 `load()`，再吃一次 `tick` 只是白读第二遍）；
   `BundlesPage` / `BuildPage` / `MachinesPage` 的 `tick` 只在生成 / 保存 / 放弃 / 建删版本等**结构操作**上动，语义正确 —— **不动**。

**验证**（全绿）：
- `npx tsc -b --force` / `npm run lint`（eslint + stylelint）/ `npm run build` / `npm run build:workbench` / `npm run check:bundle` / `cargo fmt --check`；
  Rust 一行没改，故 clippy / cargo test 不受影响（照旧口径仍应各跑一遍）。
- **新探针 `scripts/probes/params-lag.mjs`**（连点 10 下，起 `npm run dev:workbench` 后 `node scripts/probes/params-lag.mjs`）：
  **跟手 +10** · `wb_apply_draft = 1` · `wb_desk = 0` · `wb_registry = 0` · `wb_preflight = 0` · 橙色 chip `↩ 45` 出现 · 点它回 45 且 chip 消失 —— **7 条全 PASS**。
- 手动（`npm run tauri:workbench:dev`）：连点步进器跟手不回跳、长按松手只进 1 条撤销、Ctrl+Z 一次退一格、chip 点一下回干净态。

## 增量之四十八 · 跟手化补修（真机回跳）+ 客户端草稿「圆点不亮 / 重开记不住」（2026-10-19，分支 `feat/dark-mode`）

### 一、工作台：显示在"在飞那一趟"里掉回旧值（增量之四十七的第二刀）

**症状**（作者录屏 + 文字）：「最后跳到的是正确的值，只不过过程不好看、不跟手 —— 从 0.1 点一下该变 0.05，可它先像是 0.15 才变成 0.05。」

**根因**（已定位到行）：`editQueue.pump()` **在发出之前就把这一批从队列里删掉** ⇒ 乐观覆盖层消失 ⇒
IPC 在飞的那几十~几百毫秒里 `shownDesk` 退回 base（旧值）⇒ 屏上数字先跳回旧值、响应回来才跳到正确值。
假后端的 `wb_apply_draft` 几乎瞬时返回，所以第一版探针漏掉了这个窗口。

**第一刀**：`pump` 只挑不删；批次**落地之后**才收（否则 IPC 在飞的那几十~几百毫秒里显示就退回旧值）。

**第二刀（作者 18:43 录屏抓到的那一帧）**：把数值框单独抽帧看（12 帧 @ 1/60s）抓到
**`0.2 → 0.15 → 0.2`** —— 数字只在中间**一帧**变成别的值。根因：**显示值仍然被后端返回值牵着走**。
第一刀只是把"收"挪到落地之后，落地那一刻本地层照样消失 ⇒ 显示落回后端那份 desk（手上还是上一次的值）。
**客户端不闪的真正原因**：它的显示**只认自己那份本地 `draft` 层，后端往返从不回写显示**
（`useParams` 的 `valueOf = draft[key] ?? savedValueOf(key)`）。
→ 改成同一套模型：`EditEntry` 加 `sent` 标记，**落地之后也不撤**（它本身就是这一屏的草稿层）；
`settleAll` 的循环条件改成"还有没发的 / 还有在飞的"（按 `queue.size` 判会死循环）；
新增 `editQueue.clear()`，由外壳在 **undo / redo / save / discard** 时整层丢掉（那几刻后端那份 desk 才是真相）；
失败也**不回落显示**（与客户端 `patchDraft` 同一条口径：失败只记 `draftError`，不回滚界面值）。

**判据**：`scripts/probes/params-lag.mjs` —— 跟手 +10 / **全程单调不回跳** / **到最终值之后一帧都不许再动** /
`apply_draft=1` / `desk=registry=preflight=0` / 橙 chip / chip 回退；给 `wb_apply_draft` 人为加
**300ms 延迟**（真机量级）才照得出来 —— **9 条全绿**。

### 二、客户端：改了不保存关掉之后，草稿"看不见 / 圆点点不亮"

三处都已定位到行：

1. **广播根本没接上（主因）**：`state/draftState.ts` 的头注写着「发：参数页（`useParams`：改值 / 保存 / 取消 / 采用云端新值）」，
   可 `git grep draftMutated` 只有 `usePresetData` 那两处（正文链）—— 参数链**一次都不发** ⇒
   预设列表行上那枚橙色「未保存」圆点（`PresetTable` 的 `unsavedDot`）只能等整表重读才可能亮（作者：「那个橙色的点也不显示了」）。
   → `useParams` 在 **改值（`patchDraft`）/ 保存（`commitDraft`）/ 取消（`discardAll`）** 三处广播；改值走 150ms 尾随合并，保存 / 取消当场发。
2. **判据漏了 `dirty`**：两处 `setDraftPath(d?.path ?? null)` 没看 `dirty`，而 `begin_preset_edit` 一打开就写出草稿文件
   ⇒「只打开过、一个字没改」也会亮圆点。契约与头注写的都是「**真改过才算**」→ 两处都加 `d && d.dirty`。
3. **重开看不到改动、也回不到编辑模式**：`useParams` 拿 `beginPresetEdit` 的返回值时把 `reused` 丢了、
   把 `d.text`（含没保存改动的草稿正文）**一次都没用**；内存 `draft` 从 `{}` 起、**从不 seed**
   ⇒ 重开显示的是盘上那份文件的旧值（而草稿还在盘上，点保存会把它一起存下去）。
   → ① **`src/app/params/paramSessionCache.ts`（新）** —— 照 `PagePresets` 的 `textHistCache` 抄：按"编辑目标"记住
   **草稿层 + 撤销/重做栈**，卸载时写、`reused` 时读回来（**不存** 修改历史 `log`/`seq` —— 那两样后端按文件自己持久化，
   存了会打架）；② `useParams` 新增 **`draftOnDisk`**（读一眼 `get_preset_draft_summary().dirty`），
   `PageParams` 据此**只自动进一次编辑模式**（与兄弟链「编辑文件」的 `reused && dirty` 同一条口径）。

**附带修一处假后端与真机口径不一致**：`src/api/mock.ts` 的 `getPresetDraftSummary` 用户线原来拿
「带血统三行的盘上正文」直接比草稿（草稿是去血统的）⇒ **恒判脏**，"只打开没改"也会亮圆点。
改成与真机同一条比法（`withoutLineage(base)`）—— 不修这条，预览会一直骗人，第 2 条改动看着像坏的。

**判据**：新增 `scripts/probes/params-draft-dot.mjs`（`npm run build` + `npx vite preview --port 4173` 后跑）——
本地那份行右键「修改参数」→ 只打开不改（**不亮**）→ 进编辑模式改一个值 → Esc 关掉（**亮**）→
重开（**直接编辑模式 + 改动还在**）→ 保存（**灭**）—— **10 条全 PASS**。

**验证总账**：`npx tsc -b --force` / `npm run lint`（eslint + stylelint）/ `npm run build` / `npm run build:workbench` /
`npm run check:bundle` / `cargo fmt --check` 全绿；**Rust 一行未改**（照旧口径仍应各跑一遍 clippy / cargo test）。
