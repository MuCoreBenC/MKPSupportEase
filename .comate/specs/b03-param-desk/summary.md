# B03 配方台 —— 完成小结

15 条任务里做完 12 条（1–8、11、12、14，加验收 15 的自动化那部分）。
**没做的三条写在下面第 3 节，都是有原因的，不是漏了。**

---

## 1. 那个 bug：后端一直是对的

`wb_apply_draft` 每次都成功，但日志里有几次后面**没有跟着 `wb_matrix`** ——
矩阵的重取信号是 `dirtyKey={book.dirtyCount}`，而「某一层本来没有自有值，
把它从 `disk` 改成 `tower`」这一步**脏计数不变**（草稿里那一条还在，只是值换了）。
计数不变 ⇒ 不重取 ⇒ 格子上还是 `disk` ⇒ 再点开，下拉用旧的 `cell.raw` 回填 ⇒
看起来「怎么都改不回去」。

改成一个只增不减的 `tick`，**每写一次 +1**。并且落成一条纪律：
**刷新信号必须是「发生过一次写」，不能是任何业务量** —— 业务量可以在状态真的变了时保持不变。

后端侧的回归测试：`a_value_changed_back_and_forth_is_visible_each_time`，
它同时断言「有效值两次都变了」和「脏计数两次都是 1」—— 也就是把那个错的信号钉死了。

## 2. 卡：一次手势的开销砍掉三样

| | 之前 | 现在 |
|---|---|---|
| 磁盘 | **每次编辑一次原子写**（临时文件 + rename + fsync） | 0 次。停手 2 秒 / 失焦 / 关窗 / 保存前才写一次快照 |
| 派生 | 2 次（apply 一次，前端再 `wb_matrix` 一次） | 1 次（`refresh` 参数让 apply 顺带带回那一页） |
| IPC | 2 趟 | 1 趟 |
| 每条命令读盘 | `storage::load` + `read_draft` | 0 次（内存就是真相，只在开场/保存后/重载时读） |
| dev 下 | 所有首屏命令 ×2（StrictMode） | 一个 ref 挡住第二次，**StrictMode 留着** |

草稿现在的样子（你问的那个问题的答案）：

```
Rust 进程内存（Ctx.draft）      ← 唯一真相，编辑只碰它
      │ 懒写（dirty_seq != flushed_seq 才写）
      ↓
workbench/.draft/book.json    ← 只是崩溃恢复快照
```

三条纪律都落进代码了：
**快照写失败不阻断编辑**（只记一条提示 + 状态条一格）、
**丢弃改动立刻落盘**（否则崩一次会把丢掉的捞回来）、
**保存前先 flush**（save 中途挂了草稿还在）。

## 3. 没做的三条

**Task 9（完整的右抽屉）** —— 只做了 G-code 全文那一半。
「完整元数据」已经在行内展开的「高级元数据」里了，
「这一项在各机型上的值一览」与新的「对比」视角是同一件事 ——
再在抽屉里做一份，就是同一个信息的第三个入口。
真要做的话该先想清楚它和对比视角谁留谁走，而不是两个都摆上去。

**Task 10（批量模态框）** —— 没做。矩阵那边的批量还在（勾几列 → 点字段名 → 预览 → 确认），
功能没丢。列表侧的批量语义是「把这一项铺到哪几个版本」，
它和矩阵那套的预览接口（`wb_preview_bulk`）是同一个，所以是纯前端的活，
但我这一轮的上下文不够把它做到「预览 + 跳过原因 + 一条撤销」都对。

**Task 13（字段定义维护页）** —— 没动。它是唯一会改「上游数据」语义的一条，
而 doc §5 里那个问题还挂着：`content/param_registry.json` 是上游的**构建产物**，
直接回写会被上游下一次构建冲掉、而且不报错。我在 doc 里提了覆盖层的方案
（`workbench/registry-overlay.json`），**你点头之后我再动**。
在那之前那一页仍然是只读的。

另外 **Task 11.4（矩阵 → 列表 反向跳）** 也没做：正向（列表 → 对比）做了。
反向要决定「点了之后主选中跟不跟着换」——那会动到树的选中语义，值得单独想。

## 4. 与计划的不同

**① `state()` 返回克隆，不是引用。**
doc §3 写的是「`state()` 改成借用 `ctx.draft`」。真改的时候发现那要动 25 个调用点
（`build.rs` 里还有 5 个），而一次克隆是几千个小分配 —— 比它替掉的那一次 fsync
便宜三个数量级。所以 `state()` 的签名一个字没变，只是**不读盘了**；
热路径（`wb_apply_draft`）直接用 `&mut ctx.draft` / `&ctx.committed`，一次克隆都没有。

**② 空闲落盘用 `std::thread`，不是异步任务。**
仓库里没有 `tokio` 依赖，为「每秒醒一次看个计数器」拉一个进来不值得。

**③ 快照状态放进 `wording`，不是 `app`。**
`SnapshotState` 与 `SaveState` / `BuildState` 是同一类东西（状态 + 它的中文），
放 `app` 会让 `domain` 反向依赖 `app`。

**④ `Refresh` 那个枚举踩了一个坑，值得记下来。**
`#[serde(rename_all = "camelCase")]` 加在枚举上只改**变体名**，
变体里的字段要 `rename_all_fields` 才变小驼峰。少写一个的话前端传 `machineId`、
后端等 `machine_id`，**不会报错**，只是那一页没回来。
`a_refresh_request_deserializes_from_the_wire_shape` 这条测试就是为这个写的，
而且它第一次跑就抓到了。

## 5. 新形态长什么样

```
配方 | 对比 | 套餐与菜单 | 生成        ← 默认是「配方」
┌──────────┬────────────────────────────────────┐
│ 全部      │ ┌ 空间偏移                 3 项 ─┐ │
│ 偏移   5  │ │ X 轴偏移   float    -1 mm 版本 │ │
│  空间偏移3│ │ ▼ 当前值 [-1] mm  ⤺            │ │
│  运动挤出2│ │   喷嘴在 X 上的偏移             │ │
│ 擦料  27  │ │   ▸ 高级元数据                  │ │
│  擦料方式1│ │   在所有机型上看这一项 →        │ │
│  塔位置 4 │ └────────────────────────────────┘ │
└──────────┴────────────────────────────────────┘
```

- 一行四样：**中文名 · 类型 · 当前值（带来源色）· 展开箭头**
- 布尔项的开关**常驻**，点一下直接改
- 子项缩进挂在父项下面；父项把它们关掉时**整组收起**，只留一句
  「『擦拭部件』选了圆盘擦拭，下面这 N 项现在不生效」+ 一个「仍然展开看」
- 左栏计数**不随搜索变**（它是换分组看的工具）
- 改了没保存的行：**左侧琥珀竖条**（蓝色在这套界面里只表示选中）
- mkppanel 里那些「当时想自由、现在多余」的都没做：拖拽排序、增删 section、改父子挂靠

## 6. 改了哪些文件

| 文件 | 改动 |
|---|---|
| `src-tauri/src/workbench/app/mod.rs` | `Ctx` 持 `committed`/`draft`/`notices`/两个 seq/`flush_error`；`with_ctx_mut`；`flush` / `flush_if_due` / `flush_now` / `spawn_flusher`；`state()` 不读盘；`wb_desk`；`Refresh` + `ApplyResult.desk/matrix`；`wb_save`/`wb_discard` 改成内存 + 显式 flush；7 条新测试 |
| `src-tauri/src/workbench/mod.rs` | 窗口 `Focused(false)` / `CloseRequested` 落盘；起空闲落盘线程 |
| `src-tauri/src/workbench/domain/derive.rs` | `Book::desk()` + `desk_nav()` + `family_off_note` / `group_off_note`；`Desk`/`DeskNavTab`/`DeskNavSection`/`DeskGroup`/`DeskItem`；`BookView.snapshot`；4 条新测试 |
| `src-tauri/src/workbench/domain/wording.rs` | `SnapshotState` 三态 + `SNAPSHOT_FAILED`；`relate::family_off` / `group_off` / `SHOW_ANYWAY` |
| `src-tauri/src/workbench/app/words.rs` | `Words.snapshot` 表；`relate.showAnyway` |
| `src-tauri/src/lib.rs` | 注册 `wb_desk` |
| `src/workbench/views/ParamDesk.tsx` | **新**：默认视角（分组树 + 分组卡片 + 行内展开 + G-code 抽屉） |
| `src/workbench/views/ParamsMatrix.tsx` | 加 `focusKey`（从配方页跳过来定位那一行） |
| `src/workbench/App.tsx` | 四个视角、默认「配方」、`tick` 刷新信号、`run` 带 `refresh`、首屏防双发 |
| `src/workbench/shell/StatusBar.tsx` | 快照那一格（**与「未保存」分开说**） |
| `src/workbench/api.ts` | `Desk` 一族、`Refresh`、`SnapshotState`、`ApplyResult.desk/matrix`、`wb.desk` |
| `src/workbench/workbench.css` | 配方台那一整段（左栏、分组卡片、一行、展开区、关掉的那一句） |

## 7. 验证结果

| 项 | 结果 |
|---|---|
| `cargo test --features workbench` | 239 passed / 0 failed（比上一轮 +10） |
| `cargo test`（默认 feature） | 18 passed / 0 failed |
| `cargo clippy --features workbench --all-targets -- -D warnings` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `npm run lint` | 干净 |
| `npm run build` / `npm run build:workbench` | 都过 |

## 8. 你要点的几条

1. 把「擦料方式」改成圆盘、再改回擦料塔 —— **两次都要立刻生效**（这次的正主）
2. 连着改 10 个值：全程不卡；停手 2 秒之后 `workbench/.draft/book.json` 的修改时间才变**一次**
3. 默认进来是分组列表；左栏点一个组能定位过去；搜索时左栏计数不变
4. 把父项关掉 → 下面那几项整组收起来 + 一句话说明 + 「仍然展开看」能点开
5. 布尔项点一下就变（不用先展开）
6. 「在所有机型上看这一项 →」跳到「对比」，能定位到同一行
7. 改了没保存时：状态条右边同时有「待落盘」和「未保存 N 处」两格；停手 2 秒后前者消失
