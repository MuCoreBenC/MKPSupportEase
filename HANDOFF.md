# 交接台账（活）

> 更新时间：2026-10-20 · 分支：`feat/dark-mode`
> **本文只装"现在还用得上的东西"**：怎么验证、纪律、判据、仓库状态、**未结账**、最近的几条增量。
> 2026-10-01 ~ 10-11 那一批（第一/二/三圈 + 增量之二十五~三十五）已搬进
> [`docs/HANDOFF-ARCHIVE-2026-10-19.md`](docs/HANDOFF-ARCHIVE-2026-10-19.md)；
> 增量之三十六~四十四（10-16 ~ 10-19）搬进
> [`docs/HANDOFF-ARCHIVE-2026-10-20.md`](docs/HANDOFF-ARCHIVE-2026-10-20.md)（**都没删，只是挪走**）。
> 总纲：`DATA-ARCHITECTURE` / `PUBLISH-ARCHITECTURE` / `RELEASE-TRANSACTIONS` / `APP-STATE` /
> `HOOK-ERRORS-AND-MESSAGES` / `ARCHITECTURE §7.5`；全仓文档索引 `docs/DOC-INDEX.md`。
>
> **防再乱的规矩**：新东西一律写在 §3；**增量只留最近约 6 条**，更旧的搬成**一份新的归档**
> （整段搬、不改字；老归档件自标只读、不追加），本文只留一句指向；
> 搬走段里凡是「待验收 / 待拍板 / 已知红」的，**必须抄进 §2.4**，否则跟着搬丢。
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

### 2.4 未结账（2026-10-20 清点 · 找活干先看这张表）

**A. 已知红（不修就过不了 CI）** —— 2026-10-20 本地实跑核过，两条都还红：

| 判据 | 现状 |
|---|---|
| `workbench::app::build::tests::our_render_matches_the_machine_verified_baseline` | `presets/delivery/mkp/presets/A1-fastv3.3.toml` 盘上是 `offset_x = -0.7` / `offset_z = 3.7`，**机器核过的基线还是 `0.1` / `4`**（9 份里只差这两行）。**先定哪边对**，不能直接搬代码。 |
| `workbench::domain::variants::tests::real_data_matches_the_documented_split` | A1 的基底 / 覆盖 = **(1, 10)**，doc §3.3 那张表还是 **(1, 9)** —— 同一条数据改动的连带。定完上面那条再重算表。 |

**B. 等作者拍板**

1. **工作台减速开关**（`--dev-slow-ms`：只在进度事件之间 sleep、只改时序、`workbench` feature 专属）——
   8.5 MB 那份只跑 **8.4 s**，「够慢」这条验收没达成。三条路（换更大的件 / 加开关 / 不做）里我倾向加开关，
   **未点头**（增量之四十二 §4 第 3 条）。
2. **图片按需下载**（`docs/GUIDE-IMAGE-HANDOFF-2026-10-16.md` §2）：两条 guide 现在 `delivery = 'bundled'`，
   **用户端看不到新图**；要做的是台账去掉 bundled + 重跑 `gen-catalog` + 客户端补「读下载区图片」的口子
   （两张图会连带 4 条条数判据变红，照它说的核对，别改数字）。**必须配版本发布** —— 老客户端 + 新目录 = 破图。
3. **「一组 / 套装」那三件裁决**（同文档 §3：一组能不能超过两张 / 组名从哪来 / 客户端要不要动）——
   建议都先维持现状，等真需求。

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

**D. 小尾巴（不影响发版）**

- 8 条 **macOS-only 死代码 warning**（`ax.rs` / `guide.rs` 里在别的平台编不出来的表与函数），
  增量之三十八 记的，非那一轮引入。
- **M5 欠账**：`machine_dims::install()` 在发布物里的接线（校准模型档那条读取链的前置），增量之三十九 ②。

----

## 3. 最近的增量

> **更早的增量已归档**：增量之三十六 ~ 四十四（2026-10-16 ~ 10-19）整段搬进
> [`docs/HANDOFF-ARCHIVE-2026-10-20.md`](docs/HANDOFF-ARCHIVE-2026-10-20.md)（**没删**）；
> 再往前那一批在 [`docs/HANDOFF-ARCHIVE-2026-10-19.md`](docs/HANDOFF-ARCHIVE-2026-10-19.md)。
> 搬走段里的"待验收 / 待拍板 / 已知红"都抄进了 **§2.4**。

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

---

## 增量之四十九 · 预设页六件事：复制件不认云端 / 副本族排序 / 三循环 / 改名不失败 / 参数页不自动取字节 / 云端页首帧（2026-10-20，分支 `fix/preset-copy-cloud`）

> 起因：作者六张图 —— 复制出来的那一份也显示「有新版」并在参数页显示逐项差异（图 1 / 图 3）、
> `A1-fastv3.3 copy copy.toml` 排到原件上面（图 2）、原名改名受长度限制直接失败（图 5）、
> 原件改名后徽章要照旧（图 6）、云端页第一次进去没有「更新」按钮（图 7）。
> 作者另起两条口径：**排序三循环的"默认"那档 = 族优先**；
> **参数页不自动把云端新版字节取回来做比较**（理由见下）。

### 一、四条根因（都已定位到行）

| 现象 | 根因 |
| --- | --- |
| 复制件有更新徽章 | `ipc/local.rs` 的 `get_local_preset_index` **只按血统认亲**，而 `runtime::local::copy_as_new` 是**按字节复制、三行 `# based_on*` 血统原样带过去** ⇒ 复制件必然进账本。`presetTree.ts` 那段注释声称「只认账本、复制件不在里面」，**与实现不符** |
| 副本排到原件上面 | `presetTree.ts` 的 `copyRankOf` 正则**只剥一层** ` copy`：`A copy copy.toml` 反解出 base = `A copy.toml`，与原件 `A.toml` **不同族** ⇒ 退化成整串文件名比较 |
| 云端页首帧陈旧 | 目录换版后只重读 `readPresetFeedInto()`（机型树 + 交付），**没有重读身份账本**；而云端行三态正是「账本 × 当前目录」算出来的 |
| 参数页自动取字节 | `useParams` 打开时传了 `fetchMissing: true` |

### 二、改了什么

1. **复制件独立（判定沉到后端，不在前端按名字猜）**：账本出口 `get_local_preset_index` 与
   逐参数账 `get_preset_param_sync` **共一道闸** —— 查 `runtime::provenance`（出处账，本来就有，
   `copy_local_preset` 在写、`rename` 时 `repath` 改两端）里有 `kind: copy` 的那一份 ⇒ 排除。
   契约侧两处跟着变：`getPresetParamSync` 返回 `Promise<PresetParamSync \| null>`（null = 这一份不认云端，
   界面整段不画 —— **不许当成"零项待处理"**，那会让人以为云端这一版没动过）。
2. **开关**（作者：「复制出来的也保留显示？我突然感觉挺方便的」→ 设置页「预设」块，出厂 = 关）：
   `AppStateFile` 新开 `presets` 格（`copies_follow_cloud: Option<bool>`），两条命令
   `get_copies_follow_cloud` / `set_copies_follow_cloud`；**"没选过"（null）由后端解释成出厂档**，
   前端不拿 null 猜（与自动清理那条同一条口径）。改完广播新的 `presetPrefsMutated()` 代次 ——
   只用 `readLocalPresets()` 重读（**不发网络**），不并进 `deliveryMutated`（那条会引着整条目录链重读）。
3. **排序**：`copyRankOf` 递归剥到只剩原件名（链按**靠近原件的在前**存 ⇒ 剥完要 `reverse()`，
   否则 `copy 2 copy` 会排到 `copy 2` 前面）；名称列三循环 **`family`(默认) → `asc` → `desc`**，
   `PresetSortDir` 加 `'family'`，`SortTh` 那档不画方向箭头（`aria-sort='other'`）。
4. **改名不再失败**：`check_new_name` → `normalize_new_name` 返回 `(名字, adjusted)`：
   剔路径分隔符与控制字符、按 **255 字节**上限按 UTF-8 字符边界截断（后缀一个字节不动）；
   **空 / 后缀不一致仍旧拒绝**（那两档没有可修的东西）。改名 / 复制 / 第八层另存 / 导入改名
   四处共用这一道门槛，回执带 `adjusted` 供界面照实说一句。四个老测试跟着改判。
5. **参数页按需取字节**（作者裁定）：打开时 `fetchMissing: false`；`syncNeedsFetch`
   （`versionAdvanced && !cloudReady`）时不画逐项「云端 旧 → 新」与两颗按钮，只说「云端有新版本」。
   ★ 作者的原话理由：「有些本地的没改过的参数会因为自动更新就变了，这个是不好的体验」。
6. **云端页首帧**（作者要"两条都兜住"）：首屏 `boot.changed` 后补 `readLocalPresets()`（命中根因）；
   **外加**切进云端页时 `data.refreshLocal()` 对一眼（只扫盘、不发网络）。
   没有新增"目录代次"：经核，那两处是本地扫盘即可，再挂一条代次只是为同一个目的多跑一次。
7. **文案**：参数页「选择要跟随的更新」→ **「选择要更新的参数」**（作者：「选择要更新的参数的意思吧」）。

### 三、网络请求总账（作者问"我都没点更新，本地怎么知道参数变了"）

**静默（用户什么都没点，程序自己发）**

| 请求 | 位置 | 时机 | 次数 / 拉什么 |
| --- | --- | --- | --- |
| 远端目录指纹检查 `check_remote_update` | `ipc/catalog.rs` → `usePresetData.ts` 的 `checkBootstrapOnce` | 预设页挂载（页签常驻 ⇒ **打开软件就跑**），本次运行只一次 | 1 次 HTTP，拉 1 份 `catalog.json` |
| 应用远端目录 `apply_remote_update` | 同上 | 上一步发现目录换了才发 | 1 次 HTTP，拉 1 份 `catalog.json` |
| 软件更新 `update_info` | `ipc/update.rs` → `UpdateIndicator.tsx`（挂载后 **1.2s**） | 打开软件 1.2 秒后 | 1 次 HTTP，拉 1 份 `release.json` |

**主动（用户点了才发）**

| 请求 | 位置 | 用户点了什么 | 次数 / 拉什么 |
| --- | --- | --- | --- |
| 取回云端当前版 `ensure_cloud_bytes`（全应用只有两个调用者，见下） | `ipc/catalog.rs:586` | 云端行的「下载 / 更新」（含首页「下载并应用」）；点「更新」/「选择要更新的参数」（= `get_preset_param_sync` 的 `fetchMissing=true`） | 每份 1 次 HTTP；**本机已有当前版就 0 次** |
| 交付文件下载 `download_runtime_file(s)` | `ipc/catalog.rs:255 / 359` | 下载 / 批量下载 | 1 份 1 次（批量并发 N 次） |
| 下载安装包 `start_update` | `ipc/update.rs` + `runtime/updater.rs`（`stream_get`） | 「更新 SupportEase」 | 1 次流式，拉 1 个安装包 |
| 数据源探通 `probe_current` + 形状判定 `real_fetch` | `runtime/source.rs:706 / 383` | 设置页保存数据源地址 | 1～数次（形状判定依次试几个 URL） |

**本轮摘掉的那一条**：参数页打开时的 `getPresetParamSync(path, true)` → `false` ——
作者问的那句「我都没点更新，本地怎么知道参数变了」就是它。现在**打开参数页 0 次网络**。

**不联网的（别被名字骗了）**：本地行的「使用」（`applyActivePreset`）、「复制」（`copy_as_new` 按字节复制）、
改名 / 删除 / 编辑草稿 —— 都在 `presets-local/` 里做，0 次网络。
★ 旧词「另存为本地预设」2026-10-13 已整条退役（作者：「这些字都去掉」），
它对应的动作今天就是云端行的「下载 / 更新」（`fetch_cloud_preset` 内部会 `save_cloud_as_new`）——
走的是上面那条，**本机已有当前版时 0 次**。
`save_cloud_as_new` 自己**不取字节**（它收的是调用方给的 `cloud_text`），联网那一步在 `ensure_cloud_bytes`。

不算网络：`get_file_url`（复制链接，只算地址、不发请求）。

### 四、验证

- Rust：`cargo fmt` / 双 feature `cargo clippy --all-targets -D warnings` / `cargo test` **全绿**（610 通过）。
  新名门槛补 6 条单测（含中文字符边界截断那一条）；带路径那四个老测试按新行为改判。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint`（eslint + stylelint）全绿；
  四道 check（bundle / zero-network / channel-args / release-source）全绿。
- 排序行为用临时脚本对着真模块验过一遍（副本链 + 三档全对），验完即删 —— 仓库没有前端测试框架，
  本轮不为了它引入 vitest。
- **分支上另有两处预先存在的红**（与本轮无关，本轮没碰 `presets/` 与 `workbench/`）：
  `workbench::app::build::tests::our_render_matches_the_machine_verified_baseline` 与
  `workbench::domain::variants::tests::real_data_matches_the_documented_split`
  （盘上 `A1-fastv3.3.toml` 已是 `offset_x = -0.7`，机器核过的基线还是 `0.1` —— 数据待同步）。

---

## 增量之五十 · 云端更新链路重做：点「更新」= 应用（0 项也真写）+ 弹窗默认不勾机器自己的数（2026-10-20，分支 `fix/preset-copy-cloud`）

> 起因：作者多张图 —— 云端只改了注释 / 时间戳（值一项没动），客户端却长期挂着「有新版 / 更新」，
> 点「更新」只吐一句底部文案什么也没写；点「更新」应该顺势出现「选择要更新的参数」那一屏；
> 那一屏 0 项时也该能「应用这 0 项」，应用之后「有新版」就看不到；
> XYZ 偏移与涂胶速度**不要替用户勾**（全选也要跳过、单独一组排末尾）；
> 「取消」改叫「暂不修改」，提示放在**底栏左侧**（不是页面级 toast）；
> 本地表那一行也要看到「有新版」。

### 一、根因

| 现象 | 根因 |
| --- | --- |
| 注释级换版永远挂「有新版」 | 血统比的是**全文**摘要（`lineage::sha256_hex`，含注释），换版就 `version_advanced`；而 `PagePresets.runUpdate` 在 `pending === 0` 时**提前 return**：不写盘、不推进血统 ⇒ 下一次读还是「有新版」 |
| 点「更新」只出 toast | `runUpdate` 三步里写盘那一步只写"我没改过"的项，且**从不弹框**；`ParamSyncModal` 只活在参数页 |
| 0 项不能应用 | 弹窗 `disabled={all === 0}`，且 `decideSync` 在 `keys.length === 0` 时直接 return |

### 二、改了什么

1. **写那一趟换形状（后端 `ipc::param_sync::apply_at`，2026-10-20 定的规矩）**：
   **骨架 = 云端当前版正文**（注释 / `# release_time` / 云端新增字段都在里面），
   值按决定填 —— `adopt` 写云端新值，其余（hold / 没决定）**写我这份现在写着的值**；
   血统三行换成当前版（`based_on` = 目录里那份路径、`based_on_sha256` = 当前版摘要）。
   于是 **0 项 = 一次真写**（"自动应用 0 项"）：值一个不动，注释 / 新字段 / 时间戳 / 血统跟着新版走。
   写之前留原文镜像、与原文相同不落盘（与参数页保存同一条规矩）。
2. **血统推不推进看"有没有落下的项"**：这一批之后还剩未表态的待处理项 ⇒ 血统照旧
   （那几项仍然挂着，不许被这一趟悄悄抹掉）；都处理了（含 0 项）⇒ 推到当前版，「有新版」收场。
3. **预设页「更新」顺势**：`pending === 0` ⇒ 直接 `applyParamDecisions(path, [])`；
   `pending > 0` ⇒ 打开「修改参数」并带开「选择要更新的参数」（`syncFirst`），**不再静默采用任何一项**。
   删掉原来那两句 toast（「没有要处理的参数差异」「N 项你改过 —— 去「修改参数」处理」）。
4. **弹窗**：`NEVER_PRESET_KEYS`（`toolhead.offset.x/y/z` + `toolhead.speed_limit`）默认不勾、
   **全选跳过**、单独一组（标题「机器自己的数（不替你勾）」）排在末尾；
   按钮「取消」→「暂不修改」，0 项可应用；底栏**左侧**新增常驻提示位（`hint` 可覆盖）；
   字节还没取回来时不显示「应用这 0 项」（那时 `pendingCount` 天然是 0，不是"没有要改的"）。
5. **一次写完**：新增 `useParams.decideSyncMany(decisions)`（可混合 adopt / hold，一趟写完）——
   分两趟写会出现"头一趟还有没表态的项 ⇒ 血统不推进"的中间态。`decideSync` 退化为它的同批薄封装。
6. **本地行也画「有新版」**（判据 = 这一份的 `basedOn === 'outdated'`），紧跟文件名，与云端行同一枚样式。
7. 假后端（`src/api/mock.ts`）同一条写规矩；演示那份 `A1 涂胶.toml` 的 `# based_on_sha256` 落到**旧版**那一档，
   于是它真挂着两条待处理（与那段注释说的一致），点「更新」才看得到那一屏。

### 三、验证

- Rust：`cargo fmt` / 双 feature `cargo clippy --all-targets -- -D warnings` / `cargo test --lib` **全绿（612）**；
  新增 2 条（注释级换版被 0 项应用消化 / 云端新增字段跟着进来），场景 C 按新规矩改判
  （保持 = **值还是我原来的**，不再是"文件一个字节不动"）。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check 全绿。

## 增量之五十一 · 「上一次保存之前的值」全站记账 + 写值统一浮点尾闸（2026-10-21，分支 `feat/dark-mode`）

### 一、作者的两条诉求（同一天三张截图）

1. 「我希望的是它在我点击完了保存之后还是那样显示的，不是这样都变成新的 —— 还需要一个我的旧值；
   **不只是更新才有，我希望所有的都能记录下来**，我的旧值，就是上一次保存之前的值。」
   —— 此前保存后 `sync.local` 立即变新值，「我的值 / 云端旧值 / 云端新值」三格全成同一个数，对比凭空消失。
2. 「前端显示可能截断了，但是后端怎么还是这种浮点数，能不能修好，**文件里面记录的就是两位小数的才对**，
   你可以看日志或者做一道**统一的闸门给所有动文件的用」。—— 修改历史里摆着 `3.8000000000000003`。

### 二、改了什么

1. **`prevSaved`（上一次保存之前的值）**：`useParams` 每份文件一页（钥匙 = `historyKeyText`）；
   `save()` 里对这一批**真的动了值**的项记下保存前的已保存值。**跟着修改历史账本一起落盘**
   （`PresetHistory.prevSaved`：Rust `HistoryFile.prev_saved: BTreeMap<String,String>`，`serde(default)`
   旧账照读；IPC DTO / bridge / mock 同步），清空历史一并清。
2. **展示（`ParamRow` 新 prop `prevSavedValue`，`ParamCard.prevSavedOf` 通道）**：
   - 展开详情「已保存值」后面新增一格**「上一次保存前」**（有记录且与现值不同才摆）—— 所有参数都记，不限云端更新；
   - 云端对照栏：保存过后「我的值」已是新值 ⇒ 单摆一行**「保存前」**；
   - 行名下保存过的徽章也把数字留着：采用 =「我的旧值 → 云端新值」，保持 =「云端旧值 → 云端新值」。
3. **写值统一浮点尾闸（两道，同一形状：按 1e6 收 IEEE754 尾巴，修尾不补零）**：
   - 前端 `apply()`：任何来路的值（步进器算数 / 云端 JSON / 历史还原）进草稿、进历史 `from/to` 前先
     `formatNum(raw, null)`；历史**读回**也过同一道（旧账里已有的脏值显示、还原、下次整本写回都干净）；
   - 后端 `presetdata::patch::patch_preset_toml` 的 `Float` 臂（**参数页写盘唯一出口**）同样收尾 ——
     `3.8000000000000003` 落盘就是 `3.8`。补零（`3.8 → 3.80`）仍只发生在显示层（`displayDecimals`），不动文件。
4. 来源查证：`3.8000000000000003` 不是文件里的（TOML 里是 `3.8`），是**历史账本 JSON** 里存了带尾巴的草稿值；
   闸设在两处出口后，新账干净、旧账在下次写回时被洗净。

### 三、验证

- Rust：`cargo fmt --all --check`（本机 + `+1.99.0`）/ 双 feature `cargo clippy --all-targets -- -D warnings` /
  `cargo test --lib` **全绿（634）**；新增 3 条（patch 浮点尾闸 / prev_saved 往返与旧档兼容）。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check + `check:calibration-assets` 全绿。

## 增量之五十二 · 拖拽改窗口大小时露白：原生背景色随界面底（2026-10-22，分支 `feat/dark-mode`）

### 一、作者报的现象

「改窗口大小的时候，右边和底部会拖出一条白色，而且内容明显慢半拍」——
鼠标、触控板一样；`npm run tauri dev` 与打包版一样（**与构建模式无关**，这一点先排掉了）。
对照物：QQ / 微信 / 网易云 / Trae / Blender 都跟手；**EcoPaste 同为 Tauri，也有、只是轻**
（作者原话：「有的有一点，没我的这么迟缓」）。

### 二、根因（不是我们的代码，也不是拖动实现）

- `useTitlebarDrag` 走的是系统 `startDragging` → `performWindowDragWithEvent:`，与 QQ 同一条；
  且现象只在**改尺寸**时出现（拖动窗口位置时窗口 bounds 不变，不需要重新布局）。
- 真因是 **WebKit 的 live resize 呈现 bug**：`WebKit#323132`
  （`[macOS] WKWebView content lags behind its bounds during live window resize`），
  下游 **`tauri-apps/tauri#14288`**。机制：AppKit **同步**推进 WKWebView 的 bounds，
  而 WebContent 进程的远程图层树是**异步提交**的 —— 新图层到达前，屏幕上仍是上一次提交的
  旧尺寸页面表面，盖不满新 bounds 的边就露出来（上游量到 20~60px）。
  **Tauri 维护者已表态这个场景 wontfix。**
- issue 里"已验证无效、别再试"的清单（本次也逐条排掉了）：
  `Resized` 里强推 webview / superview 的 frame、`preservesContentDuringLiveResize`、
  `NSViewLayerContentsRedrawPolicy::DuringViewResize`、在 `html`/`body` 反复设背景色、
  在 webview 下面垫一层同色原生视图。
- 本地另排掉三条：**窗口拖动实现**（系统拖动）、**`contentView.layer.masksToBounds` 圆角裁切**
  （关掉无效，四角也不受影响）、**透明标题栏 + 空 `NSToolbar`**（撤掉无效）。
- **试金石实验**（已撤）：监听 `NSWindowWillStartLiveResize` / `DidEndLiveResize`，
  resize 期间把 `contentView` 挪出可见区 —— 作者看到的是一整块**纯色界面底 + 圆角完好、
  严格跟着窗口边长** ⇒ **窗口层完全正常，慢的只有 WKWebView 的页面内容**。结论闭环。
- 为什么做不到 Trae 那样：Chromium 自己有合成器，live resize 时会把**上一帧拉伸**顶住；
  WKWebView 的画面在另一个进程，AppKit 的 `cacheDisplay` 截不到，想截只能走
  `CGWindowList` + **屏幕录制权限** —— 为了拖窗口手感让用户授权录屏，不做。

### 三、改了什么（只治"白"，不治"慢"）

1. **`ipc::prefs::set_window_backdrop`**（同步命令 —— 碰 AppKit 必须在主线程）：
   走 `WebviewWindow::set_background_color`，它一次做三件：
   ① 窗口底色；② wry 侧关掉 webview 的 `drawsBackground`（**这才是露出来的那一层白**）；
   ③ `underPageBackgroundColor`。只设 `NSWindow.backgroundColor` 是上一版的弯路 ——
   露的是 WKWebView 自己的白，不是窗口底，所以"没效果"。
2. **颜色不存第二个源**：前端 `landTheme()`（`appState.ts`，替换两处 `applyTheme`）在
   `<html data-theme>` 落地之后读 `--bg-2` 报上来，真值仍然只在 `tokens.css`。
3. `contract` / `bridge` / `mock` 各一条；浏览器 mock 是空实现。

### 四、验证

- Rust：`cargo fmt --all --check` / 双 feature `cargo clippy --all-targets -- -D warnings` /
  `cargo test` **641 全绿** / `cargo test --features workbench --lib` **1036 全绿**。
- 前端：`npx tsc -b --force` / `npm run build` / `npm run lint` / 四道 check + `check:calibration-assets` 全绿。
- 真机：`setWindowBackdrop` 调用成功（日志 `ok`，无 `[chrome]` 告警），快拖时不再闪白。
## 增量之五十三 · i18n 第二刀：`presetTree` 整文件全量迁 + 三个 shared 模块（2026-10-26，分支 `feat/dark-mode`）

### 一、为什么要这一刀

第一刀（1119 条键）只覆盖各子代理名单内的文件，名单外的四个模块仍是中文字面量 ——
切英文后预设页的分段控件、操作列、展开详情、右键菜单、归档抽屉、下载水位，
以及后处理 12 步、切片器四档、打开模型框里那句话，全都还是中文。

作者裁定：`presetTree.ts` **整文件全量迁**（含 `ARCHIVE_WHY` 这类长说明），
带文案的数据管线函数**一起迁、`tr` 一路穿下去**。

### 二、做法（一条型：常量改存 `StringKey`、函数收 `tr` 首参）

- `src/i18n/index.ts` 新增并导出 `type Tr`（`useT()` 的返回）。`PageReport` 里那份
  本地 `type Tr = ReturnType<typeof useT>` 删掉、改成 import —— 全仓一份定义。
- `presetTree.ts`：约 40 张词表改存 `StringKey`（`Record<Axis, StringKey>` 形状不变，
  调用处 `tr(TABLE[x])`）；13 个函数收 `tr` 首参：
  `sourceTextOf` / `originCellOf` / `archiveOpenText` / `usageCellText` / `basedOnCellText` /
  `localUnreadableWhy` / `presetBatchText` / `versionsText` / `presetPathText` /
  `buildPresetTree` / `localRows` / `cloudRows`。
  `EDIT_TEXT.draftFailed` / `savedBack`、`IMPORT_TAKEN_TEXT.note` 由「前缀拼接」改成
  整句占位符（`{detail}` / `{name}`+`{path}` / `{name}`+`{suggested}`），调用处同步。
- `postprocess/steps.ts`：`StepView.name: StringKey`，`stepName(tr, id)`；认不出的 id 仍原样。
- `shared/slicer.ts`：`SLICER_CHOICES` 的 label/hint 改键（**应用名也进表**，换语言不该
  让下拉认出另一个应用），`slicerOptionsOf(tr, …)` / `systemSlicerLine(tr, …)`。
- `shared/download.ts`：`tickText(tr, t)` / `outcomeText(tr, o)`。
- 词典 1119 → **1274 条**（含 `common.clauseSep` 这一条标点，页面里 `join('；')` 也随语言走）。

### 三、两处不是"换个写法"的决定

1. **`buildPresetTree` 的 `后端没有 X 这个组合`**：它产生在**加载 effect** 里（20+ 个请求）。
   直接把 `tr` 塞进那个 effect 的依赖 ⇒ 换语言就整屏重读一遍。
   所以 `usePresetData` 改成存**建树的四份原样输入**（`TreeSeed`）+ `useMemo([treeSeed, tr])`
   出树：换语言只重出一棵树，不重发请求。
   这一条与"页面常驻挂、切页只切 `data-hidden`"有关 —— 切语言时预设页并没卸载，
   若用模块级 `t()` 取那句话，它会一直是上一种语言。
2. **删死表**（全仓 grep 复核、0 引用）：`DELIVERY_TEXT` / `PRESET_STATE_TEXT` /
   `STATUS_WHY` / `ASSET_USE_WHY`。`ORIGIN_TEXT` / `ORIGIN_WHY` / `CLOUD_LOCAL_WHY`
   仍被活路径读，随函数迁成 `StringKey`，不删。
   目的是不留"同一个词两个出处"（`STATUS_TEXT` vs `LIVE_TEXT` 那种）。

### 四、验证

- `npm run check:i18n` **1274 条键，键集与占位符两语完全一致**。
- `npx tsc -b --force` / `npm run lint`（0 error，仅 `PagePresets:621` 那条既有 warning）/
  `npm run build` / 四道 check 全绿。
- 探针：`i18n` **13/13**、`window-gadgets` **36/36**。
- 浏览器实测（`vite preview` + playwright，切 English 后逐屏扫 CJK）：
  预设页本地表 / 展开详情 / 右键菜单（`Use … Delete`）/ 云端表、
  `TextDiffModal` 的「Enter edit mode」→ 底栏「Discard / Save」—— **界面侧 CJK = 0**，
  剩下的全是 mock 数据本身（机型/版本名、备注、文件正文）。
- Rust 四道这一刀没跑：**只改前端**。

### 五、仍未迁完的一批（下一刀，已盘点）

参数页与几处共用组件**整页尚未迁**（不是本刀范围，也不是本刀改出来的）：
`params/PageParams.tsx` 18 处（`撤销（Ctrl+Z）` / `重做` / `丢弃改动` / `恢复默认值` /
`确认保存？` 这一整块底栏与两张 sheet、空态标题、搜索框 aria-label）、
`params/ParamRow.tsx` 17 处（详情 `dt/dd` 那一列 + 云端更新块）、
`params/PresetParamsModal.tsx` 2 处、`calib/PageCalib.tsx` 7 处、
`bbs/PageBbs.tsx` 2 处、`bbs/BbsTabs.tsx` 1 处、`shared/Drawer.tsx` 1 处、`shared/NoteBar.tsx` 1 处、`notify/NotifyHost.tsx` 1 处、
`components/modal/Modal.tsx` 1 处、`app/components/TraceTag.tsx` 1 处，合计约 52 处。
（`presets/CompareModal.tsx` 那 3 处——标题「参数对比」与两句 subtitle——本刀顺手一起迁了。）
`src/workbench/**` 仍全中文 —— 那是开发台、只有作者看，不在这条线上。


## 增量之五十四 · i18n 第三刀：内核 + 主进程的文案也进表（2026-10-26，分支 `feat/dark-mode`）

### 一、作者的原话

「我不想看到任何一个中文」。截图上是设置页更新卡里那段红框：
`The check failed: 从远端取字节失败：https://…` —— 后端那句中文原样冒进英文界面。

根因不是漏翻一条，而是**两套字典**：前端住 `src/i18n`（1274 条），
内核 `mkpse-pp` 那套住 `crates/postprocess/messages.toml`（`Lang` + `--lang` +
`text()`）。第二刀只迁了前端，**后端整条线还是中文源**。

### 二、做法

1. `crates/postprocess/messages.toml` 补了 **582 条 `ui.*`**（其中 51 条原本连 `text_en` 都没有，
   也一并补齐），现在全表 **661 条**、两语齐全、占位符与 `params` 逐条对齐。
2. `src-tauri/src/**` 里所有给用户看的句子改成 `messages::text("…键…", &[…])`：
   `AppError::*` 的参数、`with_detail(…)` 的说明句、`devsim` 演出脚本、
   `hook_ui` 的收场通知、归档那本 `_meta.json`、CLI 的 stderr。
   两个"名字"也进了表：`SlicerKind::label()`（系统默认程序 / 你指定的应用）与
   校准页那 5 张模型卡的名称与说明（`MODEL_DEFS` 改存键）。
3. **语言怎么到后端**：前端语言格已经存在（`app-state.locale`），钩子起进程时带 `--lang`；
   `mkpse-pp` 与 GUI 共用同一个 `Lang` 单值。
4. `net.rs` 那 8 条盖上了码（`E_NET_READ_001` / `E_NET_DNS_001` / `E_NET_FETCH_001` …），
   于是错误话术走"人话 + 尾注码"，前端不必自己拼。
5. **`SlicerKind::label()` 拆成两个**：`label()` 给人看（随语言）、
   `product_name()` 给 Windows 注册表查 `App Paths`（**产品名，不随语言**）——
   这两个用途混在一起时，翻成中文就找不到注册表项了。

### 三、两处判据要跟着改（不是"修测试"，是判据本身过时了）

- `presetdata::registry` 的三条单测原来断言 `contains("悬空")` 这类**中文原文**。
  文案进表后正确写法是断言**那个变量名**（`ghost` / `a.x` / `a.z`），与语言无关。
- `tests/cli.rs` 的 `run_cli` 钉上 `--lang zh-CN`：那一组判的是"报错点没点名那个东西"，
  下面几条断言的中文是**源语言约定**；不钉的话，机器上有个 `LC_ALL=en` 就会红。

### 四、验证

- `cargo fmt --all --check` / 双 feature `clippy --all-targets -D warnings` 0 条。
- `cargo test -p mkpse-postprocess --lib` **235**、`-p mkp-support-ease --lib` **641**、
  `--features workbench --lib` **1036**。
- 前端六道：`check:i18n` **1274 条**、`tsc -b --force`、`eslint` 0 error（仅既有 621 行 warning）、
  `npm run build`、`check:bundle` / `zero-network` / `channel-args` / `release-source` / `calibration-assets`。
- 探针 `i18n` **13/13**、`window-gadgets` **36/36**。
- 浏览器实测切 English 后逐屏扫：6 个页签里剩下的 CJK **全是数据**
  （机型名"拓竹"、版本名"快拆版6月以前"、参数名"层高"/"接缝"、mock 的演示文案、
  "简体中文"这个语言项本身）。
- **`process_with_ir_matches_the_end_to_end_reference` 红**：`git stash` 退回本刀全部改动
  后同一条同样红（长度差 447 字节，值一模一样），是**既有失败**，与本刀无关。

### 五、还没做的一批（已盘点 ≈52 处）

前端参数页整页、`PageCalib` 7 处、`PageBbs` 2 处、`BbsTabs`/`Drawer`/`NoteBar`/
`NotifyHost`/`Modal`/`TraceTag` 各 1 处（见增量之五十三第五节）。
`src/workbench/**` 仍全中文 —— 开发台只有作者看，不进这条线。

## 增量之五十五 · i18n 第四刀：参数页整页 / 校准页 / 工艺页与共用件（2026-10-26，分支 `feat/dark-mode`）

### 一、先修掉一个"打不开"

作者截图：设置页切语言报「没换成：这个功能还没接好（setLocale）」。

根因不是命令没注册，而是**这一刀之前那批改动根本编译不过**：
`AppStateFile` 新增了 `locale` 格，但 `Default::default()` 与 `legacy_snapshot()`
两处初始化没补上 ⇒ `cargo build` 直接 E0063 ⇒ `tauri dev` 一直在跑**旧二进制**，
旧二进制里没有 `set_locale` ⇒ 前端桥把 `Command set_locale not found` 翻成那句
「这个功能还没接好（setLocale）」。补两行 `locale: None` 后编译通过，
新二进制里 `get_locale` / `set_locale` 都在（`strings` 可验证）。

**教训**：加 AppState 的格 = 同时改三处（字段 / Default / legacy_snapshot），
漏一处不会红在测试里，只会红在"用户点了没反应"。

### 二、迁完的那一批（字典 1274 → 1430 条）

`PageParams` / `ParamRow` / `ParamCard` / `ParamSyncModal` / `PresetParamsModal` /
`PageCalib` / `PageBbs` / `BbsTabs` / `Drawer` / `NoteBar` / `NotifyHost` /
`Modal` / `TraceTag` / `useParams` —— 逐句进字典，`check:i18n` 1430 条两语一致。

两处结构性改动（不是换字符串）：

1. **`valueText` / `invalidNumberText` 收 `tr` 首参**。它们是模块级函数，
   原先自己调 `t()`（模块级那一格，语言换了组件不重渲染）。现在与
   `presetTree` / `steps` 那批同一条规矩：谁调谁把 `tr` 传进来。
2. **保存回执的语气由产出方给**：`useParams` 新增 `savedTone: 'ok' | 'warn' | 'bad'`，
   `PageParams` 不再用 `savedNote.startsWith('保存失败')` 猜红绿 —— 文案进字典之后
   那个前缀在英文界面里根本不存在，一猜就全绿。

条件三档（`等于/不等于/大于`）与「N 行」「开启/关闭」「空」也一并进表（`params.op.*` /
`params.value.*` / `params.invalid.*`）。

### 三、这一步仍然做不到的（要作者裁）

1. **参数名与说明**：`presets/registry/param_registry.toml` 是**中文数据**
   （136 个参数 + 15 个页签 + 40 个分组），随 `presets/delivery/catalog.json` 发布。
   要随语言变，得给它加可选 `label_en` / `desc_en` 并重发 catalog —— 那是**数据形状**
   的改动，还会牵到工作台的参数定义编辑器（它要能编第二种语言，否则一保存就抹掉）。
   另一条路是英文名只住前端字典（`param.label.<paramKey>`），registry 一个字不动。
2. **历史条目**：`run/preset-history/<hash>.json` 里存的是**当时那句话**（`action`）。
   现在按当前语言生成，但**老条目不会复译** —— 要复译得把 `action` 从句子改成
   记号（`actionKey` + 参数），是账本形状的改动。
3. **`src/workbench/**`**：2147 行含中文（剥注释后），且**一行 i18n 都还没接**。
   它是另一个应用（`workbench.html` + 自己那套视图），不在这一条线上。

### 四、验证

- `cargo fmt --all --check` ✓；双 feature `clippy --all-targets -- -D warnings` 0 条；
  客户端 **641**、内核 **235** 全绿（`process_with_ir` 那条既有红不在本刀范围）。
- 前端：`check:i18n` **1430 条**、`tsc -b --force`、`eslint src` 0 error
  （仅既有 `PagePresets` 621 行那条 warning）、`npm run build`、五道 check 全绿。
- 探针 `i18n` **13/13**（切 English 后设置页 / 页签 / 窗口胶囊逐项断言）。

### 五、参数名/说明：改成双语数据（作者裁定：走 registry，不走前端字典）

`presets/registry/param_registry.toml` 现在每条都带英文：

- `labelEn` / `descEn`：75 个参数全覆盖；`[[params.choices]]` 45 条有 `labelEn`；
  `[[tabs]]` 8 个、`[[tabs.sections]]` 27 个有 `labelEn`（分组还有 `descriptionEn`）。
- 两套解析结构同步加字段：`crates/preset/src/registry.rs`（内核/CLI 读的那套）
  与 `src-tauri/src/presetdata/registry.rs`（应用读的那套，另有 `ParamMetaEdit`）。
- **新的覆盖闸** `presetdata::registry::tests::every_entry_has_english`：真数据里
  任何一条缺英文就红。以后加参数必须顺手写英文（回落中文是安全网，不是常态）。

**语言在哪一层挑**：界面。IPC 两条通道（`getRuntimeCatalog` 的 `registry` 与
`getMachineParams`）把两册一起下发，`useParams` 在**读定义时**按 `locale` 摊开
（`localizeDef` / `localizeTab`，出口是 `defOf` / `defs` / `tabs` / `sectionsOf` /
`defsOfSection`）。这样切语言不必重拉数据 —— 重拉会把用户选的机型顶回"正在用的那一份"。

**工作台不会被保存抹掉**：`ParamMetaEdit` 的 `label_en` / `desc_en` 是 `Option`
（`None` = 不动，只有它主动发了才改写），选项那条按 `value` 沿用文件里原有英文。
工作台的参数编辑器**还没暴露这两格**（界面仍是中文），所以它保存不会破坏英文，
但也编不了英文 —— 那一格等做工作台 i18n 时一起开。

**代价（已付）**：`catalog.generated.json` 与 `presets/delivery/catalog.json` 都重新生成了
（`cargo run --bin gen-catalog`），结构签名随之变化（`revision b654d984…` / `5aeab99c…`）。
**发布预设是另一件事** —— 这两份已提交但没发；下次 `npm run publish:presets`
或工作台「发布」时才真正出门。在那之前，走 OTA 拿到的是旧目录（`registry` 里没有英文），
英文界面会回落中文，不会崩。

**已知的、故意留着的一处**：搜索只匹配**当前语言**的名字（英文界面搜不到中文名、
中文界面搜不到英文名）。要做双语搜索就在 `PageParams` 的匹配里把两册都喂进去。

## 增量之五十六 · 英文界面清扫：三条读取路径收口 + 工艺页转英文 + 复述型 tooltip 退场（2026-10-26）

作者拿 6 张英文界面截图点名：参数弹窗的分类/卡片/参数名、对比台的页签与「一致」、工艺页的组名与
字段名、报告页的管线明细、塔冲突那两颗按钮。裁定三件事：截图放 `tmp-shots/i18n-screens/{before,after}/`、
只删「复述型」tooltip、字典按页分段 + 出一张对照表。

### 一、根因是「同一份数据有两条读取路径」

上一刀把双语挑一册做进了 `useParams`（`localizeDef` / `localizeTab`），**对比台不经过它** ——
它自己从 `getRuntimeCatalog` 摊一遍（`CompareModal.rowsOf`），于是英文界面上整片中文。
工艺页同理（`BbsRow` 硬取 `meta.label.zh`）。

修法不是补第二份挑选逻辑，而是**收口成一个工具**：`src/app/shared/dataLabel.ts`
（`bi` / `localizeParamDef` / `localizeTab` / `localizeParamMeta`）。现在三条路共用它，
`useParams` 里那两个只是改了 import。**以后谁再读这层数据，也走这里。**

另：假后端的 `param_registry.json` 与 `params.ts` 同步补了 `labelEn` / `descEn` / `name_en`
透传 —— 它是真 IPC 的镜像，缺了这层，界面上看着「修好了」而真机没修（或反过来）。

### 二、工艺页（BBS）

- 五组标题（MKP 工艺 / 导入的 / 用户配置 / 系统配置 / 其他机型 / 读不了的）从模块级中文字面量
  改成**字典键**（`bbs.group.*`），`BbsMenuGroup.titleKey`。
- 页签名与分组名读 `layout.json` 的 `name_en`（数据本来就有两册）。
- 参数名 / 选项名按 `locale` 挑 `label.zh|en`（`BbsRow` / `BbsControl`）。
- 灰着的原因改成 `reasonKey` + `reasonArgs`：**数据层只说事实，取字在渲染处**
  （`bbsSource.reasonText(it, tr)`）—— 模块级函数拿不到 hook，也不该把某一刻的语言冻进数据。
- 支撑样式、孤儿组、导入/取数失败、抽屉与详情栏的名字，一并进字典（`bbs.*` 新增 ~35 条）。

### 三、提问按钮

`hook.rs` 的两张 `QuestionChoice` 表（挪塔 / 校准）原先写死中文，英文界面下**按钮本身是中文**。
现在走 `messages::text`，新增 8 条（`hook.choice_*`，各带 `Q_*` 码，`every_entry_has_every_language` 钉着）。

### 四、复述型 tooltip 退场（作者口径：只删「把可见文字再说一遍」的）

删了 5 处：分类胶囊的「名字 · 条数」、BBS 值文本与下拉的 `title`、详情栏预设名、
`Switch` 那个**没人传**的 `title` 口子（整个 prop 删掉）。补充型（完整文件名、快捷键、
被截断的全称）与关键解释（采用/保持/还原的后果、脏态关闭、参数键+模式+不可用原因）**全留**。
清单：`tmp-shots/i18n-screens/tooltip-inventory.md`。

### 五、字典分段与对照表

`zh-CN.ts` / `en.ts` 按页插了 **36 段**分节注释（`// ---- 工艺页（BBS） ----`）。文件是逐刀追加长的，
**没有重排键**（重排会让以后每处 diff 对不上行号），所以同一页可能出现好几段，带「（续）」。新增
`docs/I18N-MAP.md`：四类「名字」各住哪（句子 / 后端句子 / 参数名 / 数据名）+ 前缀→页面→文件→条数。

### 六、取证工具

- `scripts/probes/i18n-shots.mjs <before|after>`：9 个页面 × zh/en，视口 + 整页各一张，
  落 `tmp-shots/i18n-screens/<阶段>/<页>/`（不入库）。比对台与参数弹窗是真点开的（右键两下那条老入口）。
- `scripts/probes/i18n-cjk.mjs`：英文模式下扫 `document.body.innerText` 里含中文的行 +
  它所在的 class。**这是截图看漏时的兜底**，当前结果：**29 行**，其中 27 行是报告页管线明细
  （见下）、1 行是语言选项「简体中文」本身、1 行是假后端的「演示：…」注释。

### 七、这一刀**没做**的（都记在案）

1. **报告页管线明细**（`archive.rs::step_detail`）：那是写进 `_meta.json` 的**归档记录**。
   要双语得像「结构化事实 + 前端渲染」那样改归档契约（老记录仍保持原样），单独一轮 —— 不能
   在拼接处换 `messages::text` 了事：那样记录的语言就被写它那一刻的界面语言冻住了。
2. `usePreset.ts` 的 `PENDING_NAME`、`useBundleFiles` 的三句套餐提示、`openWithGuide` 的 `alt`
   —— 都是低频/失败态，且 `alt` 描述的是中文系统截图（译了反而对不上图）。
3. `src/workbench/**`：作者裁定单排一轮。

### 八、验证

`cargo fmt --all --check` ✓ · 双 feature `clippy --all-targets -- -D warnings` 0 ·
客户端 **642** / 内核 **235** 全绿 · `tsc -b --force` ✓ · `eslint src` 0 error
（仅既有 `PagePresets` 621 行 warning）· `npm run build` ✓ · 六道 check 全绿 ·
探针 `i18n` **13/13**、`i18n-cjk` 29 行（逐条见清单）。

## 增量之五十七 · 管线明细存事实、首页文案与字距（2026-10-26）

### 一、管线明细不再存句子

`archive.rs::step_detail()` 那套 `format!("读取 {} 行…")` 全删了，换成
`step_detail_lines()`：每行一个**字典键 + 原始参数**（`report.line.*`，值全是原始事实
`disk` / `true` / `0.70`）。`PipelineStep` 加 `detailLines` 字段。

规矩：**记录里存事实，词的映射在界面**（`PageReport` 的 `argText`）。于是同一份
`_meta.json` 中文界面读中文、英文界面读英文，归档文件本身不再被写它那一刻的语言冻住。
老记录只有 `detail`（当年那句中文）—— 界面回落，**不復译**，这是记录该有的样子。

顺带：报告页的**进度句**（`pipeline[].message`，内核 `timer.begin` 那 13 句）也按步骤 id
换成本地文案，内核那边一个字没动（它写的是日志与归档的事实，不是界面）。

三条老测试（`step_details_carry_the_facts_the_report_shows` 等）从比中文改成
**比键与事实** —— 换字典不会红，漏了行才会红。

假后端同步成同形状，否则浏览器预览永远看不到英文那套。

### 二、首页

- 五步说明精简到 3–4 个词（`Latest reference values` / `Z plate tells which cell` /
  `XY plate tells which line` / `Confirm the result`）。原来那几句在四列档挤成两三行，
  还会压到相邻格上。
- **「看起来像有空格」的真凶**：`.groupTitle` 的 `letter-spacing: 0.18em` —— 字距在
  **最后一个字母后面也加一格**，英文标题后面凭空多出一个空格，再叠 flex 的 gap 就成了
  「Machine   Sets …」。用等量的负右边距（`margin-right: -0.18em`）抵掉，中文看不出来。

### 三、顺手两处

- BBS 单位栏原来硬取 `meta.unit.zh`（`mm 或 %`），改成按语言挑（`mm or %`）。
- Z 轴图例英文那串里的全角竖线 `｜` 换半角。

### 四、现在的残留（`i18n-cjk` 9 行，全是数据）

拓竹 (Bambu Lab) / 两个 `.toml` 文件名 / 三个版本名（快拆版6月以前、标准版）/
一个模型名（帕姆）/ 语言选项「简体中文」/ 假后端的演示注释。

**其中品牌名与版本名是作者点名要英文的下一刀**：catalog 的 `machines[].brand` 与
`versions[].name` 加可选 `nameEn`，界面按 locale 挑，工作台里能编。等作者发话再动。

### 五、验证

`fmt` ✓ · 双 clippy 0 · 客户端 **642** / 内核 **235** 全绿 · `tsc` ✓ · eslint 0 error ·
build ✓ · 六道 check ✓ · 探针 `i18n` 13/13 · `i18n-cjk` **9 行**（29 → 9）。
