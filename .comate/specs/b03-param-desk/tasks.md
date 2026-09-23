# B03 配方台 —— 任务计划

顺序原则：**先把「状态在哪」改对**（草稿内存化 + 懒落盘），再做新视角。
反过来做的话，新视角会建在「每次编辑一次 fsync」这个地基上，
到时候卡的地方是同一个，只是界面换了张脸。

一个说明：Task 13（字段定义维护页）**在 doc §5 那一点点头之后才动**。
它是唯一会改「上游数据」语义的一条，值得单独确认。前面 12 条与它无关。

---

- [x] Task 1: 草稿搬进内存，`state()` 不再回读磁盘
    - 1.1: `Ctx` 加 `committed` / `draft` 两个字段，开场读一次
    - 1.2: `state(ctx)` 从「每次 `read_draft`」改成「借用 `ctx.draft`」
    - 1.3: 需要改草稿的命令改成拿 `&mut ctx`（`with_ctx` 换成 `with_ctx_mut`，读路径保留只读版）
    - 1.4: `wb_reload` 重新读上游 + 重新读快照，其余命令一律不碰磁盘
    - 1.5: 测试：`editing_twice_reads_the_disk_zero_times` —— 用一个计数过的 Store 包装，断言读次数为 0
    - 1.6: 测试：`a_value_changed_back_and_forth_is_visible_each_time` —— 就是那个「改不回去」的后端侧回归

- [x] Task 2: 懒落盘
    - 2.1: `Ctx` 加 `dirty_seq` / `flushed_seq` / `last_edit: Instant`
    - 2.2: `storage::flush_snapshot(ctx)`：`flushed_seq == dirty_seq` 直接返回，不写
    - 2.3: 空闲计时：会话建立时 `tauri::async_runtime::spawn` 一个每秒醒一次的任务，
      「有未落盘 + 距上次编辑 ≥2s」才写
    - 2.4: 窗口 `Focused(false)` 与 `CloseRequested` 各同步 flush 一次
    - 2.5: `wb_save` / `wb_generate` / `wb_publish` 进入时先显式 flush
    - 2.6: **写失败不阻断编辑**：记 `BookView.notices` 一条 + 状态条上一格，不返回 Err
    - 2.7: 测试：`ten_edits_flush_once`、`flush_is_a_no_op_when_already_current`、
      `a_failing_snapshot_does_not_fail_the_edit`

- [x] Task 3: `Book::desk()` —— 分组 → 行的派生
    - 3.1: `Desk` / `DeskGroup` DTO：组名、组内项数、`rows: Vec<Row>`（复用矩阵那套 `Row`，单列）
    - 3.2: 分组树：tab → section 两级，各自带计数（界面左栏要用）
    - 3.3: 单列意味着 `Row.cells` 长度恒为 1；`Row` 不变，**不为了单列再造一套 DTO**
    - 3.4: 子项挂在父项下面：`DeskRow { row, children: Vec<Row> }`，只有两级
    - 3.5: 父项关着时给整组一句话（`group_blocked_note`），复用 `wording::relate`
    - 3.6: `wb_desk(machine, uid, tab, query)` 命令 + DTO
    - 3.7: 测试：`groups_carry_their_own_counts`、`children_hang_under_their_parent`、
      `a_blocked_family_reports_who_closed_it`、`searching_flattens_but_keeps_group_names`

- [x] Task 4: 一次手势一次 IPC
    - 4.1: `wb_apply_draft(label, patches, refresh: Option<Refresh>)`，`Refresh` 说「顺带给我哪一页」
    - 4.2: `ApplyResult` 加 `desk: Option<Desk>` / `matrix: Option<Matrix>`
    - 4.3: 前端不再在 apply 之后单独 `wb_desk` / `wb_matrix`
    - 4.4: 测试：`apply_returns_the_page_it_was_asked_for`、`apply_without_refresh_returns_none`

- [x] Task 5: api.ts 跟上
    - 5.1: `Desk` / `DeskGroup` / `DeskRow` / `Refresh` 类型，各自标注对应的 Rust 名
    - 5.2: `wb.desk(...)`、`wb.applyDraft(label, patches, refresh?)`
    - 5.3: `npm run lint`

- [x] Task 6: ParamDesk 骨架
    - 6.1: 顶部：机型:版本 选择器（来自 `BookView` 的树，不新开 IPC）+ 参数总数 + 未保存计数
    - 6.2: 左栏分组树：tab → section 两级，点了滚到对应分组卡片（不做过滤，只做定位）
    - 6.3: 中栏分组卡片：卡片头「组名 · N 项」，卡片体一行一项
    - 6.4: 一行四样：中文名 · 类型徽章 · 当前值（带来源色）· 展开箭头
    - 6.5: 搜索框：命中时压平成一张列表，行头补组名（与矩阵同一条规则）
    - 6.6: 行高固定 `--w-cell-h` 那一套纪律照搬 —— 展开不许把别的行挤走位置之外的地方

- [x] Task 7: 行内展开
    - 7.1: 展开区：当前值控件（复用 `FieldControl`）+ desc + 「挂回继承」
    - 7.2: 「高级元数据」二级折叠：键名 / TOML 键 / TOML 节 / 来源与出厂默认 / 受谁控制
    - 7.3: 一次只展开一个（再点一次收起，Esc 收起）
    - 7.4: G-code 项不在行内展开，直接开右抽屉
    - 7.5: 「在所有机型上看这一项 →」跳到矩阵并定位那一行

- [x] Task 8: 子项与「被关着」的收纳
    - 8.1: 子项缩进一级挂在父项下面
    - 8.2: 父项条件不满足时整组折起 + 一句「X 选了 Y，这 N 项现在不生效」
    - 8.3: 那句话可以点开看（值还在、会进产物），默认不占地方
    - 8.4: 折起状态**不持久化** —— 它由数据决定，不是用户偏好

- [ ] Task 9: 右抽屉 FieldDrawer
    - 9.1: 完整元数据（照 doc §4.2 那一份，只读）
    - 9.2: G-code 全文编辑（把矩阵那个抽屉的实现搬过来，两处共用）
    - 9.3: 「这一项在各机型上的值」一览表（复用 `wb_matrix` 单行）
    - 9.4: Esc / 点遮罩关闭；**抽屉里不放第二份编辑控件实现**

- [ ] Task 10: 批量模态框
    - 10.1: 勾选要铺到哪些机型/版本，默认只勾当前这一个
    - 10.2: 影响预览走 `wb_preview_bulk`，前端不判
    - 10.3: 跳过的列出来并写原因（暂无资源 / 不适用 / 被关着）
    - 10.4: 确认后 **N 处压一条撤销**
    - 10.5: G-code 一律拒绝（沿用现有那句话）

- [x] Task 11: 矩阵降级与两个视角互跳
    - 11.1: 视角顺序改成 `配方 / 对比 / 套餐与菜单 / 生成`，默认 `配方`
    - 11.2: 「对比」页签上写清它是干什么的（同时看几台机器的同一项）
    - 11.3: 列表 → 矩阵：带上要定位的 key，矩阵滚到并选中
    - 11.4: 矩阵 → 列表：点行头的字段名可以「只看这一项的详情」回到列表并展开
      —— **没做**，见 summary 的「与计划的不同」

- [x] Task 12: 刷新信号与首屏双发
    - 12.1: 首屏三个请求用 ref 挡住 StrictMode 的第二次（**保留 StrictMode**）
    - 12.2: `tick` 那一处补注释与来由（已改，补齐说明）
    - 12.3: 加一条判据：`dirtyCount` 不许再出现在任何 `useEffect` 的依赖里当刷新信号
      —— 用 eslint 注释 + 一句 grep 断言的脚本都不合适，这条写进 `App.tsx` 的文件头注释

- [ ] Task 13: 字段定义维护页（**等 doc §5 点头**）
    - 13.1: `domain/overlay.rs`：覆盖层的形状、只存差异、开场丢弃指向不存在 key 的条目
    - 13.2: 叠加顺序：上游 registry → 覆盖层，界面与生成 TOML 走同一份叠加结果
    - 13.3: 白名单：只有 label / desc / unit / min / max / step / defaultValue / deprecated 可改
    - 13.4: 页面顶部常驻「这一页改的是字段定义，不回写 mkpse-presets」
    - 13.5: 覆盖层改了 min 而已有配方值越界 → 记一条待办，不阻断
    - 13.6: 测试：白名单外的字段被拒、覆盖层影响指纹（否则产物不会变成待生成）

- [x] Task 14: 状态条两条独立信息
    - 14.1: 「未保存 N 处」照旧
    - 14.2: 新增快照状态：`已落盘` / `待落盘` / `写不进去（原因）`
    - 14.3: 两者不许合成一句 —— 「未保存」说的是仓库文件，「待落盘」说的是崩溃快照

- [ ] Task 15: 验收
    - 15.1: `cargo test --features workbench` 与默认 feature 两套都绿
    - 15.2: `cargo clippy --all-targets -- -D warnings` 两种 feature 组合零警告
    - 15.3: `npm run lint` + `npm run build` + `npm run build:workbench`
    - 15.4: 手动：把「擦料方式」改成圆盘再改回擦料塔，**两次都立刻生效**
    - 15.5: 手动：连续改 10 个值，全程不卡；停手 2 秒后 `.draft/book.json` 的 mtime 才变一次
    - 15.6: 手动：父项关掉时子项整组收起并写明是谁关的
    - 15.7: 手动：从列表跳到对比视角能定位到同一项，跳回来也能
