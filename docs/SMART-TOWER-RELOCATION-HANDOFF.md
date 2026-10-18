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
| `app/postprocess/RelocatePanel.tsx` | **新文件**：那一问的一屏 —— **左图右文**。左：「现在」「挪后」两幅 `CollisionMap`（挪后 = 同一份事实只平移塔框）；右：标题 + 后端那句问话 + 坐标表（现在 / 自动摆放 / 冲突点，**不取整**）+ 倒计时 |
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
- **MSRV 1.77**：`Option::is_none_or` 没有；`map_or(true, …)` 又会被 clippy 咬 ⇒ 用显式 `match`。
