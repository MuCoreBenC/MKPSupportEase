# B03 参数矩阵可用性修复 —— 完成小结

反馈里的七条缺陷加上截图里发现的两条渲染缺陷，九条全部落地。Task 1–14 全部完成。

---

## 1. 根因与最后的改法

| # | 反馈 | 根因 | 改法 |
|---|---|---|---|
| D1 | 排序既不按逻辑也不按字母 | `matrix()` 全局按 `layout.order` 排序，而那是 **section 内部的序号**，跨组必然撞号交叉 | 换成五段排序键 `tab.order → section.order → 父项 order → 自己的 order → key`，前端照 `sectionLabel` 变化插分组表头 |
| D2 | 不知道是哪个选项导致灰色 | 后端算出来的 `Cell.blocked` 只塞进 `<td title>`，而值按钮 `disabled` 之后浏览器连 tooltip 都不弹 | 三件事：行头**常驻**「受『X』控制」；格子去掉 `disabled`、点开「由『X』控制，需 Y」；「去改那一项」跳转 |
| D3 | 子项应该归到它的父类或给出关联 | `parentKey` 读进来了但矩阵没用 | 排序键第三段用父项的序号让子项紧跟父项；子行缩进一级 + 「属于：X」 |
| D4 | 开关变成了「开」字「关」字 | `switch` 分支渲染的是文字按钮 | 真开关（轨道+滑块，两态同尺寸），而且**布尔行常驻开关，点一下直接改**，不用先选中 |
| D5 | 点一下整个列表都下降，点不准 | 三处按需插入元素：选中条、列头角标（`display:block` 加一行）、文本换控件高度变化 | 选中条常驻占位；角标**原位替换**列头第三行；`--w-cell-h` 统一文本与控件高度 |
| D5b | 「正在改」那条提示多此一举 | 对「点一格」这种自解释手势没有信息量，代价是整表位移 | 点一格不再显示选中条，选中条只服务列编辑与批量 |
| D6 | 取消之后莫名其妙还有蓝色 | 蓝色被「选中」和「草稿未保存」两个含义共用 | **蓝只表示选中**；草稿改成左侧 3px 琥珀竖条（琥珀在 tokens 里的定义就是「值变了 / 要留意」） |
| D7 | 框选的取消不了、太显眼 | 退出只有两条隐蔽路径；列编辑给每一格画框 | 四条退出路（Esc / 点滚动区空白 / 再点一次列头 / 选中条链接）；列编辑改成整列淡底 + 无边框输入框，只有 hover/focus 才描边 |
| D8 | 第一行文字压在冻结表头上 | 选中态把 `th` 背景**换成半透明**的淡蓝（10% alpha），滚过去的行从里面透出来 | 选中态改用 `inset` 阴影叠色，背景保持不透明；格子显式 `z-index: 0` |
| D9 | 列编辑时表头文字和值叠在一起 | 同 D8，外加选中导致表头变高而 sticky 偏移已按旧高度算好 | 表头高度恒定（第三行原位替换）+ D8 那条 |

## 2. 与计划的不同

三处偏离 doc / tasks，都是实现时发现更对的做法：

**① 带变量的句子由后端整句渲染，而不是「模板发给前端填空」。**
doc §3 原写「模板进 `wording.rs`，前端只填空」。真写的时候发现那等于把「句子长什么样」分到两个仓库里，
迟早一边改了另一边没跟上；而且仓库已有的纪律就是「文本由后端格式化好，前端不做格式化」。
所以 `Row.controlNote` / `Row.parentNote` / `Cell.blockedNote` 都是**整句**，
`Words.relate` 里只剩一个不带变量的 `goFixIt`。

**② 排序键是五段不是四段。** 最后补了 `key` 兜底，否则同序时的顺序取决于遍历，
「同一份数据每次长得不一样」这件事会从后端漏出来。

**③ 布尔行的开关常驻，不再走「先选中再改」。**
tasks 只写了「switch 换真控件」。但改一个布尔项点两下、而中间那一下还触发位移，
正是 D4 与 D5 叠在一起的地方。开关是无状态的（值来自 `cell.raw`，点一下就提交），
「67 行 × 16 列受控输入」那个顾虑对它不成立。代价是误触，接受它 ——
一次手势一条撤销早就在了，而 D5 修完之后真正导致误触的原因也没了。

另外 `testkit` 的 fixture 动了两个数：`wiping.mode` 的 order 从 4 改成 1.05、
`wiping.child` 从 5 改成 1.5，让两个分组的号段**真的重叠**。
不这么改，`rows_are_grouped_by_section_not_interleaved` 是一条恒真的空测试 ——
老代码也能通过它。

## 3. 改了哪些文件

| 文件 | 改了什么 |
|---|---|
| `src-tauri/src/workbench/upstream/registry.rs` | 新增 `section_meta` / `tab_order`（查不到给 `f64::MAX`，不是 0）+ 2 个测试 |
| `src-tauri/src/workbench/domain/derive.rs` | `row_sort_key` + `milli` 饱和转换 + `section_label` / `control_note` / `label_of`；`Row` 加 6 个字段、`Cell` 加 2 个字段 + 5 个测试 |
| `src-tauri/src/workbench/domain/wording.rs` | 新增 `mod relate`（5 句 + 1 个常量）+ 1 个测试 |
| `src-tauri/src/workbench/domain/testkit.rs` | 两个 order 改成与另一组重叠，附理由 |
| `src-tauri/src/workbench/app/words.rs` | `Words.relate` |
| `src/workbench/api.ts` | `Row` / `Cell` / `Words` 跟着后端补字段 |
| `src/workbench/tokens.css` | `--w-cell-h: 22px`（**不随密度变**） |
| `src/workbench/workbench.css` | 分组行、开关、草稿琥珀竖条、列编辑淡底、选中改 inset 阴影、`.wb-mx__slot` 固定高度、灰格说明条、mini 档 |
| `src/workbench/views/FieldControl.tsx` | 导出独立的 `Switch`；`switch` 分支换真控件 |
| `src/workbench/views/ParamsMatrix.tsx` | 分组表头、行头关联标记与缩进、灰格可点 + 说明 + 跳转、常驻开关、四条退出路、选中条常驻占位、列头原位替换 |
| `.comate/specs/b03-backstage-workbench/doc.md` | §7 视觉规范修正（蓝/琥珀分工、位移纪律、禁用也要能点）；§8.2 补行序五段键与「冻结层必须不透明」；§8.3 四条分支改为含开关 |

后端的写入路径**一个字没动**：这一批全在呈现层，没加一条 IPC。

## 4. 自动化验证结果

| 项 | 结果 |
|---|---|
| `cargo test --features workbench` | 229 passed / 0 failed（原 220，新增 9 条） |
| `cargo test`（默认 feature） | 18 passed / 0 failed |
| `cargo clippy --features workbench --all-targets -- -D warnings` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `npm run lint`（eslint + stylelint） | 干净 |
| `npm run build` | 通过，`dist/` 里没有 `workbench.html` |
| `npm run build:workbench` | 通过 |

## 5. 还得你自己点的八条

doc §6 那八条里有六条自动化测不出来。工作台窗口已经起起来了（`npm run tauri:workbench:dev`），
按这个顺序点最快：

1. 滚 67 行 —— 字段按组分段，每段一条组名，同组内顺序和客户端设置页一致
2. `wiping.rib_width` 这类子项紧跟父项、缩进一级、行头写「属于：外围结构」
3. 任一灰格子：**不点**也能从行头读出「受『擦料方式』控制」；点一下出「需 等于 擦料塔」；点「去改那一项」落到那一格上
4. 布尔行是真开关，点一下就变
5. **连续点同一列的三格，三次都命中**（这条是 D5 的验收，最关键）
6. 取消选中后一处蓝色都不剩；改过未保存的是左侧一条琥珀竖条
7. 列编辑：整列淡蓝底、没有一屏方框；Esc 退出、点表格空白处也退出
8. 滚到任意位置，冻结表头下面不透字

第 3 条要先制造一个灰格子：把某一列的「擦料方式」改成「圆盘擦拭」，它下面那几项就会灰。
第 8 条建议顺便试一下**选中一列再滚动** —— D8/D9 都是在那个状态下暴露的。
