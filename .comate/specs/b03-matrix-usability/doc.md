# B03 参数矩阵可用性修复

这一份不加功能，只修**已经做出来但不好用**的地方。触发它的是一次真实的点击试用，
反馈里的每一条都对应矩阵视角里一个具体的实现选择 —— 这些选择当时都有理由，
但放在 67 行 × 4 列的真实数据上，理由不成立。

矩阵是这个工作台唯一每天要用几十分钟的屏幕。它「能用」和「顺手」之间的差距，
比再多一个视角的价值大。所以这份修复插在 Task 17（生成视角）之前做。

---

## 1. 反馈 → 根因

反馈是六句话，对应七个缺陷。把「现象」和「根因」分开列，是因为其中三条的根因
不在它表现出来的那个地方。

| # | 现象（原话） | 真正的根因 | 在哪 |
|---|---|---|---|
| D1 | 「字段的排序，它又没按逻辑排序，又没按首字母排序」 | 全局按 `layout.order` 排序，而 `layout.order` 是 **section 内部的序号**，跨组排必然交叉 | `derive.rs:622-631` |
| D2 | 「灰色之后非常糟糕，我都不知道是哪个选项导致它灰色」 | 后端**已经算出**是谁关的（`Cell.blocked`），前端只塞进 `<td title>`；而值按钮 `disabled` 后浏览器不再派发事件，**tooltip 也弹不出来** | `ParamsMatrix.tsx:438,462` |
| D3 | 「要不然他就分到它的子类，要不然得出一个关联的」 | `parentKey` 读进来了（74 条里 7 条有），但矩阵完全没用它。父子平铺成两行毫不相干的行 | `derive.rs:667-677` |
| D4 | 「好端端一个开关，变成了一个开字关字」 | `switch` 分支渲染的是一个写着「开」/「关」的文字按钮 | `FieldControl.tsx:38-54` |
| D5 | 「多出了这一条，导致整个列表都下降，点的时候点的不准」 | 三处**条件插入元素**把后面的内容顶下去：选中条（整条）、列头的「列编辑」角标（`display:block` 加一行）、文本换成控件（高度变了） | `ParamsMatrix.tsx:190,226` / `.wb-mx__colflag` |
| D5b | 「正在改的那个提示根本没必要出现在这个地方，多此一举」 | 选中条对「点一格」这种自解释手势没有信息量，代价却是整表位移 | `SelBar` |
| D6 | 「取消了之后，莫名其妙还是有这个蓝色」 | 蓝色被**两个含义**共用：选中（底色+边框）和草稿未保存（`inset` 描边）。取消选中后草稿描边还在，看起来像「取消没生效」 | `workbench.css:621,723` |
| D7 | 「那些框选了的都不知道怎么取消框选……要不然就改成背景吧，没必要这么明显」 | 列编辑把整列每一格都换成带边框的 `<input>`，一屏几十个方框；退出只有两条隐蔽路径（再点一次列头、选中条上的链接），**Esc 无效、点空白无效** | `ParamsMatrix.tsx:207-217` |

另外从截图里看到两个没被提到但确实存在的渲染缺陷，一起修：

| # | 现象 | 根因 |
|---|---|---|
| D8 | 第一行文字**压在冻结表头上**（截图 1、2 的「熨烫抑制扩展模式」） | `border-collapse: separate` 下 `<th>` 的背景没有覆盖整条表头带，且 `thead` 没有独立层 |
| D9 | 列编辑时表头文字**和值叠在一起**（截图 4 右上角） | 选中导致表头变高，而 `position: sticky; top: 0` 的偏移已按旧高度算好 |

---

## 2. 七条改法

### 2.1 D1 + D3：行序改成「分类 → 组 → 组内序 → 父子」，并且**后端排**

排序权威不动 —— 前端仍然一行都不重排（doc §8 的既有纪律）。改的是排序键：

```rust
// derive.rs，取代现在的单键排序
/// 行序的排序键：**四段**。
///
/// 只按 `layout.order` 排是错的 —— 那是 section **内部**的序号（实测各组都从 1 开始，
/// 还有 0.5 这种插队值）。跨 section 拿它当全局键，等于把 16 组洗牌。
fn row_sort_key(&self, key: &str) -> (i64, i64, u8, i64, i64) { … }
```

四段依次是：

1. **tab.order** —— `param_registry.tabs[].order`，页签的先后
2. **section.order** —— `tabs[].sections[].order`，组的先后
3. **父子**：父行紧跟自己的子行。实现成「排序时用父的 order 再接一位」而不是
   `depth` 单独排，否则一个有子项的父行会被自己的子项挤到组尾
4. **layout.order** —— 组内序号，这才是它的语义
5. **key** —— 兜底，保证同序时也稳定（否则 `BTreeMap` 换个版本就换个顺序）

`Row` 随之多四个字段，都是**已经算好的显示用信息**，前端不再回查 registry：

```rust
pub struct Row {
    …
    /// 组的中文名。前端在它变化时插一条分组表头
    pub section_label: String,
    /// 0 = 顶层，1 = 某个父字段的子项。**只有两级**，上游没有更深的
    pub depth: u8,
    pub parent_key: Option<String>,
    /// 父字段的中文名 —— 子行上写「属于：擦料方式」
    pub parent_label: Option<String>,
}
```

前端渲染分组表头（不是新组件，就是一行 `colSpan` 的 `<tr>`）：

```tsx
{rows.map((r, i) => (
  <Fragment key={r.key}>
    {r.sectionLabel !== rows[i - 1]?.sectionLabel && (
      <tr className="wb-mx__group">
        <th colSpan={cols.length + 1}>{r.sectionLabel}</th>
      </tr>
    )}
    <RowView row={r} … />
  </Fragment>
))}
```

分组表头**不做折叠**。折叠要持久化、要和搜索交互、要处理「折起来的组里有草稿」——
换回来的只是少滚几屏。搜索已经能做到这件事。

> 搜索状态下**不插分组表头**：命中散落在各组，插进去会变成一堆只有一行的组。
> 这时候在行头上显示组名（`r.sectionLabel` 作为第二行小字）。

### 2.2 D2：灰色的格子必须点得开，而且说出是谁

三件事一起做，缺一件都还是猜：

**(a) 不再 `disabled`。** 被上级条件关着的格子照样能点，点开的不是编辑器，
是一条**行内说明**（插在同一格里，不是浮层）：

```
改不动：由「擦料方式」控制，需 等于 擦料塔        [去改那一项]
```

`label` / `need` 都是后端 `BlockedBy` 已经拼好的整句，前端不组装。
`blocked` 是**根在前**排序过的（`visibility.rs`），所以取 `blocked[0]` 就是
「最上游那个需要你先动的东西」，中间环节不用全列 —— 全列反而看不出先做哪个。

**(b)「去改那一项」跳过去。** 拿 `blocked[0].key` 在当前矩阵里找行，
选中「那一行 × 当前这一列」，并 `scrollIntoView({ block: 'center' })`。
两种边界：

- 那一行**不在当前分类页签里** → 先切到 `row.tabId` 再选中
- 那一行被搜索过滤掉了 → 先清空搜索
- 那一行在这台机型上**不适用**（`machineFilter` 排除）→ 不给跳转按钮，
  改成一句「控制它的『X』在这台机型上没有这一项」。这是数据问题，
  骗用户去点一个不存在的格子更糟

**(c) 常驻的关联标记，不靠悬停。** 有 `showWhen` 的行，行头上挂一个
`受「X」控制` 的小字（`scope === 'section'` 时写 `整组由「X」控制`）。
这样「为什么这一格是灰的」在**不点任何东西**的时候就能读到 —— 反馈里
「不知道是哪个选项导致」的直接解法是这一条，(a)(b) 是补救路径。

灰色本身保留：它表示「现在不该改」，是真的信息。但压暗从 `opacity: .5`
改成只压字色（`--w-t-off`），因为 `opacity` 会把来源徽章和挂回按钮一起压淡，
那两个和「能不能改」无关。

### 2.3 D4：开关就是开关

`switch` 分支换成真控件 —— 一条轨道 + 一个滑块，两个状态**尺寸完全一样**：

```tsx
<button type="button" className="wb-sw" role="switch" aria-checked={on} …>
  <span className="wb-sw__knob" />
</button>
```

顺带改掉一个更值得改的地方：**布尔行不再需要「先选中再改」**。

现在改一个开关要点两下（选中格子 → 点开关），而中间那一下还会触发位移。
开关控件是无状态的（值从 `cell.raw` 来，点一下直接 `onCommit(!on)`），
不存在「67 行 × 16 列受控输入」那个问题 —— 那个顾虑针对的是带本地 state 的文本框。
所以布尔格子**常驻显示开关，点一下直接落一条撤销**。

代价是误触。接受它，理由是：一次手势一条撤销已经在了，误触的恢复成本是点一下「撤销」；
而 D5 修完之后，「点的时候点不准」这个真正导致误触的原因也没了。

被条件关着的布尔格子：开关渲染成关闭态且 `aria-disabled`，点它走 2.2(a) 的说明，
**不翻转**。

### 2.4 D5 + D5b + D9：一次点击不许让任何东西移动

这是四条独立的位移源，逐条堵：

| 位移源 | 改法 |
|---|---|
| 选中条按需插入 | **删掉「点一格」的选中条**。它说的是「正在改：X · Y」——而那一格里正亮着一个输入框，自解释 |
| 列/行选中条 | 保留（列编辑和批量确实需要一句解释和一个退出口），但**改成常驻占位**：`visibility: hidden` 而不是不渲染，高度恒定 |
| 「列编辑」角标 `display:block` | 改成表头第三行文字的**原位替换**（那一行本来是「自有 N · 版本覆盖」），不新增行 |
| 文本 → 控件高度变化 | 单元格内容区固定 `--w-cell-h: 22px`，控件 `height: var(--w-cell-h); box-sizing: border-box`，文本行同高 |

表头高度恒定同时修掉 D9：`sticky` 的 `top` 偏移是浏览器按当前高度算的，
高度在选中瞬间变化就会和已绘制的行错位。高度不变，错位无从发生。

批量条（点字段名后行下面插一条）**保留插入**，那是用户主动要求的一个面板，
而且插在被点的那一行下面，不会让他正要点的东西移走。

### 2.5 D6：蓝色只留给「选中」，草稿改用左侧竖条

现在蓝色同时表示两件事，这是反馈里「取消了还是有蓝色」的全部原因。拆开：

| 信息 | 现在 | 改成 |
|---|---|---|
| 选中（我正在操作这里） | 底色 `--w-primary-soft` + 边框 | **不变**，蓝仍然是这个意思 |
| 草稿未保存 | `inset 0 0 0 1px --w-primary`（整格蓝框） | 格子**左侧 3px 琥珀色竖条** |
| 列编辑中的整列 | 每格一个带边框的 input | 整列**淡底色**，输入框无边框，只有焦点/悬停时描边 |

琥珀是对的颜色 —— tokens 里橙/琥珀的定义就是「值变了 / 要留意」，而蓝是「主操作」。
这一条修正 doc §7 里「草稿用蓝色描边」那句：当时没考虑到它会和选中同色。

竖条而不是整格描边，还顺带解决一件事：一列改了 28 项时，28 个蓝框连成一片
根本读不出「哪几格改了」；28 条左侧竖条排下来，是一条可以一眼扫的垂直轨迹。

### 2.6 D7：退出选中要有三条路，而且不再「框住」

**退出**（现在只有两条隐蔽路径，加到四条，其中两条是肌肉记忆）：

1. `Esc` —— 全局 `keydown`，任何选中状态都清空（G-code 抽屉开着时优先关抽屉）
2. 点表格**空白处/滚动区背景** —— 冒泡到容器，`sel = none`
3. 再点一次同一个列头/字段名 —— 已有，保留
4. 选中条上的「退出列编辑」—— 已有，但文案从「取消选中」改成明确的动作名

**视觉**从「框住」改成「垫底」：

```css
/* 列编辑：整列淡底色。**不给每一格画框** —— 一屏几十个方框就是反馈里的「很简陋」 */
.wb-mx__cell[data-coledit="yes"] { background: var(--w-primary-soft); }
.wb-mx__cell[data-coledit="yes"] .wb-ctl { border-color: transparent; background: none; }
.wb-mx__cell[data-coledit="yes"] .wb-ctl:hover,
.wb-mx__cell[data-coledit="yes"] .wb-ctl:focus { border-color: var(--w-primary); background: var(--w-card); }
```

也就是：整列**是**可编辑的这件事用底色说一次，具体哪一格正在被编辑用焦点说。
两件事分两个强度，不是 40 格同时用最高强度喊。

### 2.7 D8：冻结表头要真的挡住下面的行

```css
.wb-mx__table { border-collapse: separate; border-spacing: 0; }
.wb-mx__table thead th { background: var(--w-card-inner); }        /* 每个 th 自己不透明 */
.wb-mx__colhead { box-shadow: 0 1px 0 var(--w-line); }             /* 边框在 separate 下不可靠，用阴影补 */
```

`border-spacing: 0` 是关键：默认 2px 的间隙会在表头带上开两道缝，
滚动时下面的行正好从缝里露出来 —— 截图里那条「压在表头上」的文字就是从缝里透出来的。

---

## 3. 受影响的文件

| 文件 | 改动 | 具体位置 |
|---|---|---|
| `src-tauri/src/workbench/upstream/registry.rs` | 新增 `section_meta(&self, id) -> Option<&SectionMeta>`、`tab_order(&self, tab_id) -> f64` | `impl Registry`，`param_tabs` 附近 |
| `src-tauri/src/workbench/domain/derive.rs` | 新增 `row_sort_key`；`matrix()` 换排序键；`Row` 加 `section_label` / `depth` / `parent_key` / `parent_label` | `matrix()` L607-701、`struct Row` L1026 |
| `src/workbench/api.ts` | `Row` 四个新字段 | `export interface Row` |
| `src/workbench/views/ParamsMatrix.tsx` | 分组表头行；`BlockedNote` 行内说明 + 跳转；行头关联标记；删掉 cell 选中条；列编辑改 `data-coledit`；Esc / 点空白退出 | 全文 |
| `src/workbench/views/FieldControl.tsx` | `switch` 换真开关；控件统一 `--w-cell-h` | L38-54 |
| `src/workbench/workbench.css` | 分组行、开关、草稿竖条、列编辑底色、表头 `border-spacing`、固定行高 | 矩阵段落 |
| `.comate/specs/b03-backstage-workbench/doc.md` | §7 视觉规范修正一句：草稿标记从蓝色描边改琥珀竖条，蓝色专用于选中 | §7 |

**不动**的地方，写出来免得后面误改：

- 写入口仍然只有 `wb_apply_draft`。这一批全是呈现层，一条新 IPC 都不加
- 排序权威仍在 Rust。前端新增的分组表头是**照着 `section_label` 的变化分段**，不是前端自己分组
- 「不适用 / G-code / 值」三分支的判定仍在后端
- 状态词仍从 `wb_words` 来。新增的句子（「受 X 控制」「去改那一项」）里带变量，
  模板进 `wording.rs`，前端只填空

---

## 4. 边界与异常

| 情形 | 处理 |
|---|---|
| `section_id` 在 `layout_schema` 里有、`param_registry.tabs` 里没有 label | 排序键用 `f64::MAX` 落到最后，`section_label` 退化成 `section_id`。**不 panic** —— 这是上游数据问题，不该让工作台打不开；但启动断言里记一条 notice |
| `parent_key` 指向一个**不可见**的字段（被 `machineFilter` 排掉 / 已废弃） | `depth` 仍为 1，但 `parent_label` 为 `None`，行头不显示「属于」。不把子行提到顶层 —— 它在数据上确实是子项 |
| `parent_key` 成环 | 计算 `depth` 时最多走一层（上游只有两级），第二层直接停。不做环检测 —— `showWhen` 那边才需要 |
| 搜索命中一个子行、父行没命中 | 只显示子行，不自动带出父行。行头的「属于：X」已经说明了归属；自动带出会让「显示 N / 共 M」对不上 |
| 被点开说明的那一格随后因撤销变成可编辑 | 说明条跟着消失（渲染由 `cell.blocked` 驱动，不存独立状态） |
| 布尔开关所在的格子 `editable === false` | 渲染关闭态 + `aria-disabled`，点击走说明分支。**不翻转** |
| G-code 格子 | 不受这批改动影响，仍然只报行数 + 右抽屉 |
| 密度 `mini`（宽度 < 640） | 分组表头保留（它是信息不是装饰），行头的「受 X 控制」小字隐藏 —— 那一列只有 140px |

---

## 5. 数据流

```
wb_matrix(cols, tab, query)
  └ Book::matrix
      ├ order_cols          列序：照配方本，不照勾选顺序        （不变）
      ├ keys 并集           任一列的机型有这个字段              （不变）
      ├ row_sort_key        ★ tab.order → section.order → 父子 → layout.order → key
      └ Row{ …, section_label, depth, parent_key, parent_label }   ★
          └ cells[] ← Book::cell                                （不变）
                       ├ blocked ← Gate::blocked   根在前        （不变）
                       └ dirty   ← Draft::pending                （不变）

前端 ParamsMatrix
  ├ section_label 变化 → 插一条分组表头行                        ★
  ├ depth === 1        → 行头缩进 + 「属于：parent_label」        ★
  ├ showWhen 存在      → 行头「受 X 控制」                       ★
  ├ !cell.editable     → 可点，点开 BlockedNote（blocked[0]）     ★
  │                       └ 「去改那一项」→ 切页签/清搜索 → 选中 → 滚动居中
  └ 任何改动 → onApply(label, patches) → wb_apply_draft          （不变）
```

★ = 这次新增。写入路径一个字没动。

---

## 6. 预期结果

修完之后，下面每一句都应该能在真机上当场验证：

1. 滚动 67 行，字段按「偏移 / 擦料 / 风扇 / 涂胶 / 切换」分段出现，每段有一条组名；同组内的顺序和客户端设置页一致
2. `wiping.rib_width` 这类子项紧跟在 `wiping.outer_structure` 下面，缩进一级，行头写「属于：外围结构」
3. 任何灰格子：不点也能从行头读出「受『擦料方式』控制」；点一下能读到「需 等于 擦料塔」；点「去改那一项」直接落到那一格上
4. 布尔行是一个真开关，点一下就变，不需要先选中
5. 从头到尾**没有任何一次点击让表格上下移动**。连续点同一列的三格，三次都命中
6. 取消选中后，剩下的蓝色**一处都没有**；改过还没保存的格子是左侧一条琥珀竖条
7. 列编辑：整列淡蓝底，没有一屏方框；按 `Esc` 退出，点表格空白处也退出
8. 滚动到任意位置，冻结表头下面**不透字**

验证方式：`cargo test --features workbench` + `cargo clippy --all-targets -- -D warnings`
+ `npm run lint` + `npm run build:workbench`，然后**手动点一遍上面八条** ——
这八条里有六条是自动化测不出来的，只能点。
