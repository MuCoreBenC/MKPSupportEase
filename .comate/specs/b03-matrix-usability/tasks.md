# B03 参数矩阵可用性修复 —— 任务计划

顺序原则：**后端排序先落地**（它是 D1/D3 的唯一权威，前端拿不到字段就没法画），
然后 CSS 地基（先把「一点就位移」和「表头透字」修掉，否则后面每次手动验证都在跟这两个打架），
最后才是交互。颜色与开关放在结构改造之前，因为它们是纯替换、不牵动 JSX 结构。

每一条都能单独编译通过、单独 `npm run lint` 通过。

---

- [x] Task 1: registry 补两个分组元数据查询
    - 1.1: `Registry::section_meta(&self, section_id) -> Option<&SectionMeta>` —— 从 `param_registry.tabs[].sections` 里找，中文名与 order 的唯一来源
    - 1.2: `Registry::tab_order(&self, tab_id) -> f64` —— 找不到返回 `f64::MAX`，不 panic
    - 1.3: 测试：`section_meta_falls_back_to_none_for_unknown_id`
    - 1.4: 测试：`tab_order_puts_unknown_tabs_last` —— 保证上游多一个未声明的 tab 时排到最后而不是最前

- [x] Task 2: 行序换成四段排序键，Row 带上分组与父子
    - 2.1: `Book::row_sort_key(&self, key) -> (i64, i64, i64, i64, &str)` —— tab.order → section.order → 父的 order → layout.order → key；`f64` 乘 1000 取整比较，避免 `total_cmp` 在元组里的写法炸开
    - 2.2: 父子相邻：有 `parent_key` 的用**父的** layout.order 当第三段，自己的当第四段；顶层行第三段用自己的 order
    - 2.3: `matrix()` 换掉 L622-631 的单键排序
    - 2.4: `Row` 加 `section_label` / `depth` / `parent_key` / `parent_label`，在构造处填好
    - 2.5: `depth` 最多算一层（上游只有两级），父不可见时 `parent_label` 为 `None` 但 `depth` 保持 1
    - 2.6: 测试：`rows_are_grouped_by_section_not_interleaved` —— 断言 `section_label` 在整个行序里**连续不重现**（出现过又出现就是交叉）
    - 2.7: 测试：`child_rows_follow_their_parent` —— 用 fixture 里 `wiping.child` 紧跟 `wiping.mode`
    - 2.8: 测试：`row_order_is_stable_for_equal_keys` —— 两次调用结果完全一致
    - 2.9: 测试：`unknown_section_sorts_last_and_labels_with_its_id`

- [x] Task 3: 新增的几句话进 wording，前端只填空
    - 3.1: `wording.rs` 加模板常量：`受「{}」控制` / `整组由「{}」控制` / `属于：{}` / `改不动：由「{}」控制，需{}` / `去改那一项` / `控制它的「{}」在这台机型上没有这一项`
    - 3.2: 用函数而不是裸格式串暴露（`blocked_sentence(label, need)` 之类），这样占位符顺序错不了
    - 3.3: `words.rs` 把它们挂进 `Words`（新一组 `relate`），补进 `keys_match_the_serialized_enum_values` 那类回归测试
    - 3.4: 测试：每个模板都被调用过一次（防止加了词没接线）

- [x] Task 4: api.ts 跟着后端改
    - 4.1: `Row` 加 `sectionLabel` / `depth` / `parentKey` / `parentLabel`，每个都标上对应的 Rust 字段
    - 4.2: `Words` 加 `relate` 一组
    - 4.3: `npm run lint` 过 —— 这一步之后前端会因为没用新字段而只是「多了字段」，不会红

- [x] Task 5: CSS 地基 —— 固定高度与表头不透字
    - 5.1: `--w-cell-h: 22px`，单元格内容区（文本行与控件）统一这个高度，`box-sizing: border-box`
    - 5.2: `border-spacing: 0`，修掉表头带上那两道透字的缝（D8）
    - 5.3: `thead th` 每个自己不透明背景 + `box-shadow: 0 1px 0 var(--w-line)` 补分隔线
    - 5.4: 列头三行文字高度写死，选中时**原位替换**第三行而不是加一行（D5/D9 的根）
    - 5.5: `.wb-mx__group` 分组行样式：左对齐、`sticky left: 0`、底色比表头浅一档
    - 5.6: `npm run lint`（stylelint 的 `no-duplicate-selectors` 上次咬过一次，这次改完立刻跑）

- [x] Task 6: 颜色两个通道分开
    - 6.1: 草稿标记从 `inset` 蓝框改成左侧 3px 琥珀竖条（`--w-warn` 系）
    - 6.2: 选中（cell / col / row）保持蓝，且**只有选中用蓝**
    - 6.3: 列编辑整列改 `data-coledit` 淡底色；列内输入框默认无边框，`:hover` / `:focus` 才描边
    - 6.4: 被条件关着的格子压暗从 `opacity: .5` 改成只压字色，来源徽章与挂回按钮不跟着淡
    - 6.5: 注释里写清「蓝=选中、琥珀=改了没保存」，并说明它修正了原 doc §7 的哪一句

- [x] Task 7: FieldControl 真开关
    - 7.1: `switch` 分支换成 `role="switch"` 的轨道+滑块，两态同尺寸
    - 7.2: `.wb-sw` / `.wb-sw__knob` 样式，高度对齐 `--w-cell-h`
    - 7.3: 其余三种控件（select / number / text）统一高度，保证文本↔控件切换零位移
    - 7.4: `aria-checked` 与 `aria-disabled` 都给全，键盘 Space/Enter 可切

- [x] Task 8: ParamsMatrix 结构改造
    - 8.1: 分组表头行：`sectionLabel` 与上一行不同就插一条 `colSpan` 行；**搜索中不插**，改在行头显示组名
    - 8.2: 行头缩进：`depth === 1` 加一级缩进 + 「属于：X」小字
    - 8.3: 行头关联标记：有 `showWhen` 的行常驻「受『X』控制」（`scope === 'section'` 用另一句）
    - 8.4: 删掉 `sel.kind === 'cell'` 的选中条；`SelBar` 只服务 col / row，且常驻占位不参与布局变化
    - 8.5: 列头「列编辑」改原位替换第三行文字
    - 8.6: 更新文件头注释 —— 现在的注释里「选中条常驻不是浮层」等几句已经和实现不符

- [x] Task 9: 灰格子点得开、说得清、跳得过去
    - 9.1: 值按钮去掉 `disabled`，被关着时点击进入说明分支而不是编辑分支
    - 9.2: `BlockedNote` 行内说明：一句 `改不动：由「X」控制，需 Y` + 一个「去改那一项」
    - 9.3: 跳转：找到 `blocked[0].key` 那一行 → 必要时切页签 / 清搜索 → `sel = { cell }` → `scrollIntoView({ block: 'center' })`
    - 9.4: 目标行在这台机型上不适用时不给按钮，改成「控制它的『X』在这台机型上没有这一项」
    - 9.5: 说明条由 `cell.blocked` 驱动，不存独立 state —— 撤销后自动消失

- [x] Task 10: 布尔格子常驻开关
    - 10.1: `cell.kind === 'value'` 且 `param.uiComponent === 'switch'` 时，不管选不选中都渲染开关
    - 10.2: 点一下直接 `onApply` 落一条撤销，不经过「先选中」
    - 10.3: `editable === false` 时渲染但不翻转，点击走 Task 9 的说明分支
    - 10.4: 列编辑模式下布尔列的表现与单格一致（不要出现两套开关样式）

- [x] Task 11: 退出选中的四条路
    - 11.1: 全局 `Esc`：抽屉开着优先关抽屉，否则清空选中
    - 11.2: 点滚动区背景清空选中（表格内元素 `stopPropagation` 到位，别把「点格子」也清掉）
    - 11.3: 再点一次同一列头 / 同一字段名切回 none（已有，回归验证）
    - 11.4: 选中条上的链接文案改成「退出列编辑」/「退出批量」

- [x] Task 12: 密度适配回归
    - 12.1: `mini` 下分组行保留、行头的「受 X 控制」隐藏
    - 12.2: `compact` 下 `--w-cell-h` 不变（行高不能随密度变，否则位移又回来了），只缩字号与内边距

- [x] Task 13: 回写主 spec 的视觉规范
    - 13.1: `b03-backstage-workbench/doc.md` §7 把「草稿用蓝色描边」改成「草稿用琥珀竖条，蓝色专用于选中」，并注明原因
    - 13.2: §8 补一句行序规则（四段排序键），因为原文只写了「按 layout.order」

- [x] Task 14: 验收
    - 14.1: `cargo test --features workbench` 与默认 feature 两套都绿
    - 14.2: `cargo clippy --all-targets -- -D warnings` 两种 feature 组合零警告
    - 14.3: `npm run lint` + `npm run build` + `npm run build:workbench`
    - 14.4: 手动点 doc §6 的八条，逐条记结果（其中六条自动化测不出来）
    - 14.5: 重点手动项：连续点同一列三格都命中；取消选中后没有任何蓝色；A2L 那一列（暂无资源）在分组与灰格改动后仍然正确
