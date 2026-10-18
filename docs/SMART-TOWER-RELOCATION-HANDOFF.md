# 交接：智能塔位置（碰撞时自动摆塔）

> 日期：2026-10-18 ｜ 分支：`feat/dark-mode`（工作区未提交）
> 范围：碰撞检测 → 「自动挪塔 / 自己挪模型」那一问 → 两幅俯视图 + 坐标 → 设置页开关
> 状态：**代码全部落地并全绿**；「答 relocate 之后管线真的跑完」这一条**只差真机验收**

---

## 1. 作者拍板的四条口径（别改，改了先问）

| # | 问题 | 定的口径 |
|---|---|---|
| ① | 开关放哪儿 | **设置页**（`postproc prefs → ir.prefs`）。不动 25 份套餐、无 schema 迁移 |
| ② | 自动摆塔怎么找位置 | **区间内网格搜索**：挑离原位最近、且整框在板内 / 不撞模型 / 不在禁区的那个 |
| ③ | 确认后写到哪 | **只改这一趟**（IR 内存里的 `wiper_x` / `wiper_y`），**预设文件一字不动** |
| ④ | 什么时候问 | **每次碰撞都先问**（开关开着时）：`把塔挪到不撞的位置` / `我自己去挪模型` |

**顺序铁律**：先建议挪模型，挪不动才建议调塔（作者原话）。与 ④ 不冲突 —— 问句两颗按钮
就是把"要不要自动挪塔"的选择权先交给用户；选"自己挪模型"就落到原来那条"挪多少 mm"的建议。

---

## 2. 全链路（按数据流读）

```
gcode + 预设 toml
   ↓ preset::load_ir                       crates/preset
   ↓ postproc_prefs::apply_to_ir           src-tauri/src/runtime/postproc_prefs.rs
       └─ ir.prefs.auto_tower_position ← 设置页开关（PostprocModeSection「智能塔位置」）
   ↓ hook::run 的「碰撞预检」                 src-tauri/src/hook.rs
       ├─ detect_tower_collision（与管线第 6 步**同一个函数**）
       ├─ auto_tower_position_for_facts（内核算法）
       ├─ Client::ask_with_geo(QKIND_TOWER_RELOCATION, …, geo)
       │     geo = { collision: CollisionFacts, candidate: [x,y]|null, current: [x,y] }
       └─ 答 relocate ⇒ 改 ir.wiping.wiper_x/y，**继续进管线**
          答 model / 超时 / 没界面 ⇒ 什么都不改 ⇒ 管线第 6 步照旧撞、照旧报错归档
   ↓ pipeline 12 步（第 6 步 collision 复检：干净就一路跑完）
```

**为什么预检而不在管线里问**：管线是算法层，**不持问句通道**（机型那一问、校准件那一问
也都住 `hook.rs`）。预检用同一个 `detect_tower_collision` ⇒ **判据仍只有一份**，代价是
多扫一遍 G-code（几十毫秒）。

**问句通道复用**：`Client::ask_with_geo` 走现成的两段倒计时（8s 宽限 + 110s 答题 ≈ 118s，
作者要的"120 秒自动停"），超时 = `stop` ⇒ 管线照旧撞、照旧报错。

---

## 3. 改了哪些文件

### 内核 `crates/postprocess`

| 文件 | 改动 |
|---|---|
| `src/postproc/disk/collision.rs` | **+`printable_area_of`**（gcode `; printable_area`，上一刀）<br>**+`auto_tower_position` / `auto_tower_position_for_facts` / `tower_fits`**（本刀）<br>`CollisionFacts`：`kind/target` 改 `String`、补 `Deserialize` / `PartialEq`（挪塔 payload 要过线 + 前端契约要能读回来）<br>`Box4` / `TowerShape` 补 `Deserialize` / `PartialEq`<br>出界判据改按**板子**（`bed = printable`，上一刀）；模型包围盒板外闸 |
| `src/postproc/disk/mod.rs` | 导出 `auto_tower_position_for_facts` |
| `src/ir/types.rs` | `PrefsIr.auto_tower_position`（出厂 `false`） |
| `messages.toml` | `hook.tower_relocation_question`（`Q_COLLISION_RELOCATE_001`） |

### 客户端后端 `src-tauri`

| 文件 | 改动 |
|---|---|
| `src/runtime/app_state.rs` | `PostprocPrefs.auto_tower_position` / `ResolvedPostprocPrefs.auto_tower_position` / `resolved()` 默认 `false` |
| `src/runtime/postproc_prefs.rs` | `apply_to_ir` 写这一格 + 两条判据（两端默认一致 / 真的落到 IR） |
| `src/hook_ipc.rs` | **+`QKIND_TOWER_RELOCATION`**、**+`CHOICE_RELOCATE` / `CHOICE_MODEL`**<br>**+`RelocateGeoPayload`**（`collision` / `candidate` / `current`）<br>`QuestionPayload.geo: Option<RelocateGeoPayload>`<br>**`FromHook::Question` 改为 `Box<QuestionPayload>`**（几何撑大枚举，clippy `large_enum_variant`）<br>`+Client::ask_with_geo`（`ask` 委托给它、`geo=None`） |
| `src/hook.rs` | `Asker::ask_tower_relocation`（默认 `None` = 没界面不替用户决定）＋ `ChannelAsker` 实现（两颗按钮：`relocate` 主 / `model` 次；摆不下就只给 `model`）<br>**+碰撞预检块**（开关开着才跑）<br>**+`tower_relocation_of`** 分派函数（只认 `CHOICE_RELOCATE`） |
| `src/hook_ui.rs` | 问句构造处补 `geo: None`；`Question` 装箱后快照取 `(*question).clone()` |

### 前端 `src`

| 文件 | 改动 |
|---|---|
| `api/contract.ts` | `PostProcessQuestion.geo?: RelocateGeo`；**+`RelocateGeo`**；`ResolvedPostprocPrefs.autoTowerPosition`；`PostprocPrefsWire.autoTowerPosition` |
| `app/postprocess/RelocatePanel.tsx` | 导出**两块**：`RelocateMaps`（只画图）与 `RelocateNote`（说明页）。<br>`RelocateMaps`：两幅 `CollisionMap` 并排、**不套外圈容器**、**`height={104}` 按高度定尺**（宽度由板纵横比反算 ⇒ 两幅同高、板不裁、框不长大）；**不摆「现在 / 挪后」小标题**；两板之间**一支虚线箭头**指向"挪后"（在缝里，不占高度）；画布里**不画箭头**（`hideArrow`）。<br>`RelocateNote`：问句正文 + **两列坐标小表**（标签定宽 76、值等宽数字、冲突点独占一行）+ 倒计时（**不重复标题**） |
| `app/postprocess/PostProcessModal.tsx` | 挪塔那一问：`lead` = `RelocateMaps`；**原来两栏事实那一格**换成 `.relocateSlot` 定高槽，在「说明 / 原始信息」之间翻页（`relocateTab`，换问句自动翻回说明）；**页脚左下角**一颗很小的翻页钮（`.slotToggle`，`margin-right:auto` 顶到最左，动作按钮仍靠右） |
| `app/postprocess/PostProcessModal.tsx` | `questionKind === 'tower-relocation' && geo.collision != null` ⇒ 渲染 `RelocatePanel`（否则退回三行字的 `AskHero`）<br>按钮不用改：照 payload 的 `choices` 通用渲染，点哪颗回哪个 `id` |
| `app/postprocess/PostProcessModal.module.css` | `.relocateHero` 一族（绿 = 流程选择，不是警告橙；窄屏 <720px 上下排） |
| `app/settings/PostprocModeSection.tsx` | 「智能塔位置」开关（`Switch`，写 `autoTowerPosition`） |
| `app/state/usePostprocPrefs.ts` / `api/mock.ts` | 乐观更新 / mock 解析值补这一格 |

---

## 4. 判据（都绿）

`crates/postprocess`（189 条全绿，其中与本刀相关）：

- `the_tower_is_relocated_to_the_nearest_free_place` —— 挑离原位最近、整框在板内、不压模型
- `relocation_gives_up_when_the_plate_has_no_room` —— 板满 ⇒ `None`
- `relocation_skips_forbidden_zones` —— 禁区里不许摆
- `the_advised_move_judges_the_plate_not_the_motion_range` —— 出界按板子（不是运动范围）
- `the_advised_move_never_pushes_the_model_off_the_plate`
- `the_move_advice_carries_an_arrow` / `printable_bed_comes_from_the_gcode_when_declared` /
  `the_model_bbox_stays_inside_the_bed`

`src-tauri`（589 条 + workbench 965 条全绿）：

- `the_two_default_sets_agree` —— `PrefsIr::default()` 与 `ResolvedPostprocPrefs` 两端默认一致
  （**新加一格必须两边一起改**，否则这条红）
- `every_pref_lands_on_the_ir` —— `auto_tower_position` 真的落到 IR（不留假开关）
- `the_relocation_answer_only_moves_the_tower_when_it_was_chosen` —— 只认 `relocate`；
  `model` / 超时 / 没界面 / 摆不下 ⇒ 塔不动
- `every_message_survives_the_wire` —— 问句装箱 + `geo` 之后仍能过线

前端：`npx tsc -b --force`、`npx eslint`、`npm run build` 全过。

---

## 5. 还差什么（按优先级）

1. **真机验收（唯一未验的一条）**：拿一份真撞的 gcode（仓库里有现成的
   `.19966.1.gcode` 那类），开关打开跑一趟：
   - 弹出那一问时**左图右文**、两幅图、坐标对得上；
   - 点「把塔挪到不撞的位置」⇒ 这一趟**跑完**（不是报错），产物里塔在新位置；
   - 点「我自己去挪模型」/ 等超时 ⇒ 停下，报告页照旧有建议 + 俯视图；
   - 开关关掉 ⇒ 行为与今天**一字不差**（不预检、不问）。
2. **回执句（暂缓）**：自动挪塔那一趟的成功结论目前与普通成功一样，用户只能从"报告页
   塔位置"或产物看出来。想做的话加一条 `hook.tower_relocated`（`N_COLLISION_RELOCATE_001`）
   并想清楚**摆在哪一行**（`warnings` 会套"警告："前缀，语义不对）。
3. **报告页留痕（可选）**：`_meta.json` 里没有"这趟自动挪过塔"这一格。要留就加一个
   `auto_tower` 字段（记得升契约注释 + 报告页读它）。
4. **开发台档位（可选）**：`devsim.rs` 的场景清单里加一格「挪塔那一问」，方便离线演这一屏。
5. **探针（可选）**：这一屏目前只有前端 mock 能演 —— 真机那条路（钩子 → 问句 → 界面）
   只能真机验收，写探针要先能把 `QuestionPayload.geo` 塞进 mock。

---

## 5.5 模态框高度：唯一出处 + ESLint 约束（2026-10-18）

作者连着两轮说「模态框尺寸变化了」。两次都是同一个坏法：这一带**各自内联写死高度**
（一次是板子按宽度算、实际画出 160；一次是翻页槽比两栏事实高一截）。

现在：

- `src/app/postprocess/metrics.ts` 是**唯一出处**：`HERO_H`（上面那一带 104）、
  `FACTS_SLOT_H`（下面两栏事实 / 翻页槽 170）。板子按 `HERO_H` 定尺、翻页槽按
  `FACTS_SLOT_H` 高 ⇒ 两页同高，翻页与切屏都不改框的大小。
- `eslint.config.js` 里 `POSTPROC_FIXED_HEIGHTS`：在 `src/app/postprocess/**` 的 JSX 里
  **内联 `height` 写数字字面量一律报错**（规则已用探针验过会红、`src/app/postprocess`
  现存代码无假阳性）。量渲染高度 eslint 做不到 —— 那条得靠真机/探针。

## 5.6 「模态框等高」现在有真判据（2026-10-18）

作者三次说「模态框尺寸变化了」，前两次都是量出来才找到真凶。现在这件事**可量**：

- `src/api/mock.ts` 的 `devPostProcessRun()`：**只在 dev 且 URL 带 `?pp=…`** 时返回假快照
  （`result` / `relocate` / `mismatch`），于是那几屏在浏览器里也开得出来（生产构建里整段被摇掉）。
- `scripts/probes/pp-modal-height.mjs`：三屏各开一次，量 `div[role="dialog"]` 的实际高度，
  **差 ≤1px 才算过**。实测 453 / 453 / 453。
- 量出来的两个真数（写进 `metrics.ts`）：板子那条带子给 104 会渲染成 **106**（画布自带 2px 皮
  ⇒ 板高取 `HERO_H - MAP_CHROME`）；两栏事实的实际高度是 **167**（4 行 × 2 栏）而不是拍脑袋的数。
- ESLint（`POSTPROC_FIXED_HEIGHTS`）挡"又手写一个高度数字"这个**动作**；
  探针挡"最后长出来不一样"这个**结果**。两条一起才闭环。

## 5.7 两处箭头（别再二选一）与"板子放大"（已挂起）

**两处箭头说的是同一件事的两个尺度，两个都在**（作者 2026-10-18 定，改之前先看注释）：

1. **画布里那支白箭头**（`CollisionMap arrow={towerArrow}`）：塔的**首尾** —— 从现在的塔中心指到
   目标塔中心，长度 = 真实位移；镞尖正落在 `ghost` 目标虚框中心（同一个位移算的）。
2. **两块板之间那支箭头**（`RelocatePanel` 里的 `<span className={s.relocateArrow}>`）：
   只表示"从这一幅看到下一幅"（现在 → 挪后），跨图阅读的方向感。

> 血泪：这两支被来回换过三轮（先只有①，作者要"板间一支"⇒ 删①；再要①⇒ 删②；最后两个都要）。

**板子放大：已挂起**（作者 2026-10-18 选「先保持 104，尺寸以后再说」）。三种备选与代价记在这儿：

| 方案 | 板高 | 代价 |
|---|---|---|
| 往下压"说明"页留白 | ~140 | 切到"信息"页得缩回 104，否则盖住事实表 |
| 溢出模态框外（像首页浮钮） | ~150–170 | 框会"破形"，浮在框边的页面上 |
| 借事实表 20–24px | ~128 | 信息页要滚一小截（末行贴底） |

任何一条都要保持 `pp-modal-height.mjs` 的判据绿（两套环境、差 ≤1px）。

## 5.8 斜肋外墙：塔框必须按**这一趟的塔高**算（2026-10-18）

作者真机：「挪动后的还是碰撞了，因为斜肋外墙随着高度会变大，下面会比较大」。

根因：`calculate_tower_collision_bbox` 把 `tower_height` **传的是 0** ⇒ `calculate_tower_bbox`
里那段「最小底深随高度涨」（`get_limit_depth_by_height` / `MIN_DEPTH_PER_HEIGHT`）永远不生效
⇒ 高塔的底座被按"最矮的塔"算小 ⇒ 碰撞检测**和**自动摆塔都用了一个偏小的框。

修法：`detect_tower_collision` 先量 **`max_z_of(content)`**（这份 gcode 最高的 Z = 塔高），
一路带下去：

- `CollisionResult.tower_height`（新格）；
- `calculate_tower_collision_bbox(ir, tower_height)`；
- `auto_tower_position(..., tower_height)` / `auto_tower_position_for_facts(..., tower_height)`；
- `hook.rs` 两处预检都传 `collision.tower_height`（第二处原来还错传了 `&ir`，改成 `&probe`）。

判据：`a_tall_rib_tower_gets_a_wider_footprint`（240mm 的肋塔框必须比 0mm 宽）、
`the_tower_height_is_the_tallest_z`。

## 6. 坑（踩过的，别再踩）

- **`FromHook::Question` 必须装箱**：几何进来之后枚举被撑大，clippy `large_enum_variant`；
  装箱后快照那一句是 `(*question).clone()`（Box 只解一层，`**` 会报 "cannot be dereferenced"）。
- **选项 id ≠ 管线常量**：分派只认 `hook_ipc::CHOICE_RELOCATE`，别拿别的字去比
  （2026-10-17 校准那一问就是拿 `"direct"` 比 `"direct_calibrate"` 踩的）。
- **开关不许做假的**：`PrefsIr` / `ResolvedPostprocPrefs` / `apply_to_ir` / 设置页四处
  必须同一次落地 —— 只摆一个不生效的开关，就是 `notify_on_finish` 那个事故。
- **预检与管线用同一个检测函数**：不许在 hook 里另写一份"撞没撞"的判据。
- **探针中途被杀会弄脏夹具**：`scripts/probes/*.mjs` 临改 `mockServer` 的补丁留在仓库里
  （这次就是 `reports.ts`）；跑探针别 Ctrl-C，脏了手工还原。
- **两幅图的箭头不是同一件事**（2026-10-18 真机截图踩的）：报告页那支是"挪**模型**多少"
  （`E_GCODE_COLLISION_002` 的语义）；挪塔那一屏要的是"**塔**从 A 到 B"。
  `CollisionMap` 为此长了 `arrow` 覆盖参数（给了就如实一支、镞尖落在目标上），
  别把报告页那支直接搬过来 —— 用户一眼就能看出画错了。
- **两幅图不给确定宽就会一大一小**：`CollisionMap` 的 `.wrap` 是 flex 列、宽度跟着内容走，
  底下的图例会把其中一幅撑宽。`hideLegend` + `.relocateFig { width: 200px }` 两条一起给。
- **MSRV 1.77**：`Option::is_none_or` 没有；`map_or(true, …)` 又会被 clippy 咬 ⇒ 用显式 `match`。

## 7. 第二轮修复（2026-10-18 晚）：挪完仍撞 + 挪塔记录

作者真机：「点了自动移动塔位置了，但是还是碰撞了」，并要求把挪塔记录进报告。

### 7.1 根因（三条，全部实证）

1. **两套样本**：「撞没撞」用的是**逐个挤出点** ∩ 塔框，而自动摆塔避的是
   `model_bbox`（**只统计板内点**的 AABB）。只有原擦料塔撞上时 `model_bbox` 根本没被算出来
   ⇒ 自动摆塔「避开一个空框」，挪完照撞。
2. **挪完不复检**：改完 `wiper_x/y` 直接进管线，靠第 6 步兜底；仍撞就整趟失败，
   没有第二次寻位、也没有一句「挪了但仍撞」。
3. **塔高口径**：`max_z_of` 只认 `G0/G1 Z`（弧线抬 Z 会漏）—— 现改为**先认 header 的
   `; max_z_height:`**，读不到才逐行扫；`detect_tower_collision_all_layers` 重建
   `CollisionResult` 时曾把 `tower_height` / `occupancy` 重置回默认（**与当年 `printable`
   同一个坑**），这次一并修掉（显式传参并写进结构体）。

### 7.2 修法

- **`Occupancy`（占用格）**：把所有**板内挤出点**打进 2mm 格网（`occupancy_of`），
  自动摆塔按格子判「会不会压到东西」—— 与 detect **同一批样本**，比 AABB 精确，
  原擦料塔 / 支撑 / 清料点天然都在里面。
- **第二道闸**：改完 `wiper_x/y` 立刻用同一个 `detect_tower_collision` 复检；
  仍撞 ⇒ `W_COLLISION_RELOCATE_001` 停下（**不静默继续**），报告里照样有俯视图（挪后的位置）。
- 答「自己挪模型 / 超时」⇒ `W_COLLISION_RELOCATE_002` 停下；板上没位 ⇒ `W_COLLISION_RELOCATE_003`。
  三条都**跳过管线**（不在第 6 步再撞一次、再报一句没头没尾的冲突）。

### 7.3 报告里的挪塔记录

`_meta.json` 顶层新增 `relocation`（`archive::RelocationRecord`，camelCase）：

| 字段 | 含义 |
|---|---|
| `choice` | `relocate` / `model` / `timeout` / `unavailable` |
| `from` / `to` | 塔左下角（mm，**不取整**）；`to = null` 表示没挪 |
| `movedMm` | 直线位移 |
| `beforeConflictCount` / `afterConflictCount` | 挪前 / 挪后冲突点数 |
| `verified` | 复检过没过（`null` = 没挪） |
| `towerHeightMm` | 这一趟用的塔高 |

- 成功那趟也会写（`verified = true`），结论句换成 `N_COLLISION_RELOCATE_001`
  （「擦料塔已从 X … / Y … 挪到 X … / Y …（… mm），复检没有冲突」）；
- 报告页多一组「挪塔」（`PageReport.buildGroups` 的 `relocation` 组）：你的选择 / 塔位置
  （从 → 到）/ 挪动距离 / 复检 / 塔高；`relocation == null` ⇒ 整组不摆（旧记录通吃）；
- 透传链：`Meta.relocation` → `ipc::report::DetailDto.relocation` → `contract.ts ReportDetail.relocation`。

### 7.4 测试样例

- `crates/postprocess/tests/fixtures/real/README.md`：说明怎么把切片器的真文件拷进来；
- `.gitignore`：`*.gcode` 全忽略，**只留** `rib-tall-min.gcode`；
- `rib-tall-min.gcode`（1.3KB，入库）：`; max_z_height: 245.60` + `; printable_area` +
  一小片模型点 + 板外清料点；
- 判据 `the_real_rib_sample_moves_the_tower_off_the_points`：撞 → 塔高认 245.6 →
  自动摆位**不许压到任何模型点**；
- 本机那份 `.5926.1.gcode`（8.9MB，被前一次原地覆盖过）已拷进该目录（git 忽略）。

### 7.5 跑法

```bash
cargo test -p mkpse-postprocess --lib collision     # 18 条（含真实样例）
node scripts/probes/pp-modal-height.mjs             # 模态框等高 / 翻页槽 / 标题栏命中
```

### 7.6 还差什么

- **真机复跑 `.5926.1.gcode`**：确认「自动挪塔」之后这一趟真的跑完、产物里塔在新位置；
  以及三条停下文案在真机上的读感。
- 成功屏那句已换成 `N_COLLISION_RELOCATE_001`，但 `PostProcessFinished` 里**还没有结构化的
  `relocation` 字段**（界面只拿到一句话）；要做结构化展示（例如成功屏小表）需要扩契约。

## 8. 报告页两幅板子 + 按钮即时反馈（2026-10-18 晚）

### 8.1 成功挪塔也画「现在 / 挪后」

作者：「成功挪塔的也可以显示，画出来碰撞位置，就是把后处理的那两个板子也做进报告页面」。

- 事实**跟着记录走**：`RelocationRecord` 新增 `before: Option<CollisionFacts>`（挪前那一份）。
  为什么不能用 `Meta.collision`：成功那趟撞了就进不了成功出口，那一格恒 `null`。
- 报告页新组件 `src/app/report/RelocationBoards.tsx`（+ `.module.css`）：两幅 `CollisionMap`
  （`height = 132`），左「现在」带**画布内白色塔首尾箭头 + 目标虚框**，右「挪后」只平移塔框、
  `hideArrow`；两幅之间一支**虚线绿箭**。没挪（`to == null`）就只画左幅。
- 新的组：`id: 'relocation-map'`（标题「塔位变化」，`sum = 现在 / 挪后`），
  渲染分支在 `PageReport.tsx` 的 `g.id === 'collision'` 旁边。
- ⚠️ **旧记录没有 `before`**（这一版之前写的）⇒ 那一组不摆，只有文字表格。

### 8.2 选项按钮"点了没反应"

作者：「点击之后感觉不够灵敏…塔会卡住的感觉，我希望视觉优先，先做出反应再执行都可以」。

- `act()` 现在**返回 promise**；选项按钮点击时先 `setAnswering(id)` → 那颗按钮立刻变成
  **8px 转圈 + 「正在挪塔…」/「正在处理…」**，后端回执后恢复（`finally`）。
- `answering` 声明在 hooks 区（`busy` 旁边）—— 放到 `act` 旁边会触发
  `react-hooks/rules-of-hooks`（`act` 在早退之后）。
- `.spin` 样式带 `prefers-reduced-motion` 兜底（不转圈时改半透明）。
- 注意：真正的耗时在后面（复检要重扫一遍 G-code，管线十几秒），所以**文字先把话说在前面**，
  不让用户以为卡死。

### 8.3 答完那一屏"按钮一闪一闪"（2026-10-18 夜，真机）

作者：「点击把塔挪到不撞的位置之后出来问题了……这个时候应该显示原本的 hero 进度条那些，
但是现在是按钮在一闪一闪的，还能一直点击，这不对」。

**病根不在按钮，在问句什么时候被收掉**：

- `answer_post_process` → `HookView::answer` → `Client::ask_with_geo` 收到答复就返回 ⇒ **投递即返回**，
  IPC 立刻 Ok；
- 而 `HookView.shared.question` 只在 `FromHook::Finished` 那一支被清空（`hook_ui.rs:418`）
  ⇒ 从"答复"到"这一趟结束"之间，问句**一直挂着**；
- 前端 `busy` 在 IPC 回来的那一下被清成 false ⇒ 问句那一屏和按钮复活 ⇒ 再点又闪一下。

**修法（前端收口，不动协议）**：`PostProcessModal` 里 `answering = { kind, deadline }`（点下去那一刻
记下**这一问的身份**），并派生 `showAsk = asking && answering === null` ——
问句那一屏与页脚按钮那一排都改用 `showAsk`，于是点下去**这一屏立刻让位给 hero**（`showProgress`
那条分支本来就在，进度条照常走）。松手条件由一条 effect 管：问句换了人 / 收掉 / 本次报错 /
兜底 130 秒（那一问的自动终止是 118 秒）。

★ 别在 `.finally()` 里松手 —— 那正是原来一闪一闪的来源。
★ 按钮上那颗转圈也随之删了：screen 已经不在，`.spin` 是死代码（`tsc` 会把 `answering` 收窄成
  `never` 直接报错，这也是一道提醒）。

### 8.4 报告页两组的图标

`GROUP_SHAPES` 补了 `relocation`（一座塔 + 右向箭头）与 `relocation-map`（两幅板子 + 中间一小段
箭头）；漏了就是标题左边一片空白（作者红框圈出来的就是那两处）。

### 8.5 答完之后"下半截还是问句" + 条子倒车（2026-10-18 夜，真机第二轮）

作者：「我点击之后它又从 0% 开始了，没有从中断的百分比继续，而且这个显示的内容没有变回去」。

- **下半截没变回去**：`isRelocate` 的判据还写着 `asking`，而那一格（`RelocateNote` + 倒计时）
  是 `lead` 之外的正文块 ⇒ 答完 `lead` 让位给 hero，正文却还挂着问句的说明与"1 分 57 秒后
  自动终止"。改成 `showAsk` 之后正文跟着落回两栏事实（`baseGroup/runGroup`）。
  **口径：凡是"问句那一屏"的东西，判据一律 `showAsk`，不许用 `asking`。**
- **条子不往回跳**：`answering` 多带一格 `from`（点下去那一刻的百分比），hero 收
  `Math.max(真实, from)` —— 只是下限，不是伪造；真实值追上来之后它自然失效
  （`answering` 在问句收掉时清空，下限也就跟着没了）。
- ⚠️ **0% 这件事的实情**：那一问是**管线开跑之前**的预检（`hook.rs` 里
  `auto_tower_position` 那一段自己先扫一遍，扫完才 `process_with_ir`），所以答复之后
  这一趟真是从 0% 起跑 —— 不是"重来"，是"还没开始"。要让弹窗出现在 60% 那种位置
  （"撞了才问、问完接着跑"），得把这一问挪进内核第 6 步（`pipeline/mod.rs` 的
  `Step::Collision`）：那里现在只会 `return Err(PostprocError::Collision)`，
  要给它一条"问 → 改 `wiper_x/y` → 复检 → 继续"的通道（`IrProcessRequest` 加一个回调）。
  这是跨 crate 的改动，单独一刀。

## 9. 查证：预检 vs 正式管线（2026-10-18 夜）

作者要求先查清三件事再决定动不动跨 crate 的代码。结论（都对着代码核过）：

### 9.1 预检阶段发不发进度事件？

**不发，一条都没有。** `hook.rs::run` 里从 `std::fs::read_to_string(&job.gcode)`（~694）
到预检块（`auto_tower_position` → `detect_tower_collision`，~823）再到
`pipeline::process_with_ir`（~955），中间**没有 `sink.emit` / `StepTimer`**；进度只有
内核 `StepTimer` 在管线里才发。所以预检那段时间快照里 `progress = null`，hero 画的是
`percent = 0 / step = '准备中'` —— **那个 0% 不是"预检进度"，是"没有数据"**。

### 9.2 正式管线是不是从 0 计？前端会不会覆盖？

- 内核第一条是 `Step::Input` 0%（`pipeline/mod.rs:407`），`percentOf()` 把 9 步映射成 0→100
  ⇒ 答复之后确实从 0 起。
- 前端 `bind(PROGRESS_EVENT, (prev, p) => ({...prev, progress}))` 是**直接替换**。
  但预检从没发过进度 ⇒ 不存在"覆盖掉预检进度"这回事。
- 换句话说：**方向 B 不需要改前端进度那一层** —— 问句挪进第 6 步之后，`progress` 继续
  被替换成 6 步那一帧（~55%），条子自然接着走。

### 9.3 预检与管线有没有完整重复扫描？

**有，四处：**

| # | 重复的东西 | 预检（hook.rs） | 管线（pipeline/mod.rs） |
|---|---|---|---|
| 1 | 读文件 | `read_to_string(&job.gcode)` ~694 | `Step::Input` 再读一遍 + sha256（407） |
| 2 | `detect_mode` | ~819 | step 4（与 `decide_wiping` 并行，~470） |
| 3 | `decide_wiping` / `apply_wiping_decision` | 在 `probe` 克隆上跑（~820-822） | step 5（~485） |
| 4 | `detect_tower_collision` | ~823 | step 6（488） |

外加 `auto_tower_position_for_facts` 的网格搜索（只在撞了才跑，不重复）。

**量级**：15:35 那条记录「管线计时 9 步 · 8129 毫秒」，总耗时 14.0 秒 ⇒ 预检 + 归档 +
外壳大约 6 秒 —— 所以预检不是"零成本"，它也确实是**真实工作**，只是没进这条进度条。

### 9.4 取舍

- **方向 B（推荐）**：把问句挪进 `Step::Collision`。`IrProcessRequest` 加一条可选回调
  （`FnMut(&CollisionFacts, Option<[f64;2]>, [f64;2]) -> Option<[f64;2]>`），钩子把
  `ChannelAsker` 递进去；第 6 步撞了就"问 → 改 `wiper_x/y` → 复检 → 继续"。
  收益：进度语义天然统一（问在 ~55%，答完接着走）+ 删掉 9.3 那四处重复 + 整趟更快。
  代价：跨 crate API、取消/错误传播、`IrProcessRequest` 的构造点（测试里很多）全要跟着改。
- **方向 A（兜底）**：保留预检，把它的工作**折算**进统一进度（比如预检给出一个
  0~10% 的前置段，管线映射到 10~100%）。改动只在两侧的映射表，但重复扫描还在。

### 9.5 「最后还闪一下」（与碰撞无关，已修）

`PostProcessModal` 里结果块那条淡入 effect 原来把 **`finished` 整个对象**写进依赖。
「预估打印时间」是后补的：常驻进程算完推 `postprocess-print-time`，
`usePostProcessRun` 会把同一个结论换成**新对象** ⇒ effect 当成"结果变了"重跑：
`getAnimations().cancel()` + 从 0 再淡入一遍 = 屏上闪那一下。

修法：依赖只认真正该重播的两个时机（`showProgress` / `flipBack`）；收尾窗口那条
effect 改成 `finishedNull` / `finishedOk` 两格标量。**凡是"后补字段会换对象"的那条流，
依赖里不许写对象本身。**

---

## 10. 方案 B 的独立交接文档（2026-10-18 夜）

上面 §9.4 那两条路的展开、接口设计、改动清单、判据、分步落地与风险，单独写在
**`docs/PIPELINE-TOWER-ASK-HANDOFF.md`** —— 只写了设计，代码一行没动，等作者拍板。
