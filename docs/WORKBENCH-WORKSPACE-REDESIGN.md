# 工作台工作区改造（2026-11-02 作者裁定）

> 本文是施工图，不是设计稿。作者的裁定原文见 §0；每一条都与仓库里的**现状**对照过，
> 带文件行号。**没有列进来的东西不许顺手改**。

---

## 0. 裁定

> 「真实文件是唯一正式来源；工作台有一个隔离的临时工作区；所有工作台编辑先进入工作区，
> 保存时再安全提交到真实文件。」
>
> 「**我不建议只开 A**（只把值编辑内存化）。如果决定改，就按『真实预设 + 应用数据目录
> 临时工作区 + 统一事务保存』的完整模型改。旧的影子树不是换个路径继续保留，
> 而是从工作台架构里退出。」

三件事必须分开，且**各自只有一个位置**：

| 东西 | 放哪 | 谁能写 |
|---|---|---|
| 真实预设 | 仓库 `presets/` | **只有** `wb_save` 的提交那一半 |
| 工作台未保存编辑 | 应用数据目录 `workbench/workspace/` | 所有 `wb_*` 编辑命令 |
| 保存中的暂存 | `workspace/save-journal/` | 保存事务自己，成功后清理 |

---

## 1. 现状核实（这一节是事实，不是计划）

| 事实 | 位置 |
|---|---|
| 数据根 = `<appDataDir>/workbench`（生产）或仓库 `workbench/`（开发） | `Store { root }`（`store.rs:45`） |
| 工作区 = `<数据根>/.draft/presets` | `txn::shadow_root()`（`txn.rs:1199`） |
| 基线 = `<数据根>/.draft/base` | `txn::base_root()`（`txn.rs:1225`） |
| **每改一格值就落一次盘** | `Presets::apply_values` 末尾 `self.registry.write_back()`（`presetdata/mod.rs:383`） |
| 打开工作台时把真目录镜像进工作区 | `settle_shadow(store.root, real_root)`（`app/mod.rs:524`） |
| 事务快照**读目录** | `txn::snapshot(&ctx.shadow_root)`（`app/mod.rs:840`） |
| 崩溃后"恢复草稿" | `Txn::recovered()`（`txn.rs:1025`），`Ctx::open_at` 里 `if recovered` |
| 保存 = 工作区整份 commit 到真目录 | `save_in`（`app/mod.rs:2237`） |
| 外部改动拒存（已有） | `E_WB_SAVE_STALE_001`（刀 6） |
| 未保存 diff = 工作区 vs 真目录 | `wb_shadow_diff`（`shadow_diff.rs`） |

两处**与作者印象不符**、要在动手前说清：

1. **生产环境的工作区已经在应用数据目录**，不在仓库。仓库里的 `workbench/.draft/`
   只在**开发模式**出现（dev 的数据根就是仓库），且已在 `.gitignore:41`。
2. 所以"从仓库里退出"这一条，实际要做的是**让开发模式也把工作区放到应用数据目录**，
   而不是改生产路径。障碍：`Store::at(root)` 只拿到数据根，**拿不到 appDataDir**
   （`Ctx::open` 那条无参路径拿不到 `AppHandle`）。解法见 §5 刀 1。

---

## 2. 目标模型

```
<真实仓库>/presets/                 ← 正式预设（唯一真源）
        ▲
        │ 保存：事务性提交（暂存 → 逐个替换 → 更新基线）
        │
<appDataDir>/workbench/workspace/
        ├── files/                  ← 工作区当前内容（编辑只改这里）
        ├── baseline/               ← 打开时真目录的快照（判冲突、判净零）
        └── save-journal/           ← 保存事务阶段记录（成功后清理，崩溃后据此恢复）
```

打开 → 真目录铺一份到 `baseline/` 与 `files/`；编辑只动 `files/`；保存把 `files/` 的
差异提交到真目录，**成功后 `baseline/` 重新对齐**；放弃 → `files/ = baseline/`；
关闭 → `files/` 可以丢（下次从真目录重铺）。

**不保留跨重启草稿**（作者明确）：`Txn::recovered()` 与"上次会话未保存的改动"那条
提示一并删除。保留的是 **`save-journal/`**（保存中断要靠它，删了就没有恢复依据）。

---

## 3. 逐处改造

| 现有 | 新职责 | 影响面 |
|---|---|---|
| `Ctx.presets` | 指向 `workspace/files/`（不再叫 shadow） | 全仓 `ctx.presets` 调用点语义不变，只是根换了 |
| `registry.write_back()` | **只更新工作区文档**，不写真实目录 | `apply_values` 去掉落盘；`write_back` 改写成只写 `workspace/files/` |
| `txn::snapshot()` | 从**工作区文档**序列化取快照（不读磁盘） | `run_ctx` 的温度计改成内存序列化 |
| `wb_save` | 工作区差异 → 真目录（事务、阶段机、备份全保留） | `save_in` 重写提交段 |
| `wb_discard` | `files/ = baseline/`，不读真目录 | `discard_in` 已经是这个语义（刀 3），只需换根 |
| `wb_shadow_diff` | 工作区 vs 真目录，且**区分"外部改动"与"未保存修改"** | `shadow_diff.rs` 加一列 `external` |
| `settle_shadow` / `recover_dir_replace` / `recover_save_stage` | 只保留保存事务需要的；删掉"恢复草稿"那半 | `txn.rs` 删 `recovered`、`session.json` 里的草稿字段 |
| 资产/套餐/结构编辑 | **同一个工作区**（现在就写 `.draft/presets`，换根即可） | 无需分行为 |

**作者的硬要求**：不允许出现"值编辑暂存、结构编辑立即改盘"两套行为。现状其实已经是
统一的（都写工作区），换根之后仍然统一 —— 这是这次改造最省力的地方。

---

## 4. 保存与冲突

```
保存
 ├─ 真目录 == baseline？ ── 否 ──▶ 拒绝保存（E_WB_SAVE_STALE_001），
 │                                  提示「真实文件被外部改过」+ 三个选项：
 │                                  重新载入 / 只提交不冲突的部分（后续）/ 取消
 ├─ 生成变更清单（工作区 files/ vs baseline/）
 ├─ 写暂存（同盘临时名）→ 记阶段 → 逐个替换真文件
 ├─ 全部成功 ⇒ baseline/ = files/，清 save-journal/
 └─ 失败 ⇒ 按 save-journal 回滚，保持 baseline 不动
```

`E_WB_SAVE_STALE_001` 已有（刀 6），**这条不用新写**，只需把"重新载入"接上
（现在是提示 + 引导，没有一键）。

---

## 5. 刀序（每刀自带验收）

### 刀 1 · 工作区搬出数据根 —— ✅ 已完成（2026-11-02，未提交）

落地形态：`paths::set_workspace_root`（Tauri `setup` 注入 `<appDataDir>/workbench/workspace`）
+ `txn::shadow_root/base_root` 优先读它、没注入才退回数据根下 `.draft`（所以 `cargo test`
与 CLI 一个字节没改）。`settle_shadow` 开头补建目录。

- `Store` 增加 `workspace: PathBuf`；生产/开发都指向 `<appDataDir>/workbench/workspace`。
- `txn::shadow_root/base_root` 改成读它；目录名 `presets/` → `files/`、`base/` → `baseline/`。
- **障碍**：`Ctx::open()` 没有 `AppHandle`。做法：Tauri 层在 `wb_open` 之前把
  `appDataDir` 注入一个一次性全局（`paths::set_workspace_root`），`Ctx::open_at` 测试
  仍用临时目录。
- 验收：dev 模式下仓库里不再出现 `workbench/.draft/`；工作台照常可编辑/保存/放弃。

### 刀 2 · 编辑不再每格落盘 —— ✅ **不需要**（2026-11-02 复核）

`registry.write_back()` 写的是 `Presets::root()`，也就是**工作区里的文件**，
它从来没写过真实目录（真实目录只有 `save` 的提交那一半碰）。刀 1 把工作区搬进
应用数据目录之后，这条行为**正好就是作者选的方案**：

> 「它可以是内存，也可以是软件自己的数据目录里的临时文件。我建议采用软件数据目录里
> 的临时工作区……而不是把所有内容都塞进内存。」

所以不做内存化，也不改 `txn::snapshot`（它读工作区目录是对的）。

### 刀 3 · 保存提交 —— ✅ **已经满足**

`save_in` 的提交段本来就是「工作区 → 真目录」：事务（阶段机 + 备份 +
`recover_save_stage`）+ 外部改动冲突拦截（`E_WB_SAVE_STALE_001`，刀 6）。
成功后才推进 base。要做的只是刀 1 的换根。

### 刀 4 · 删掉"跨重启草稿" —— ✅ 已完成（2026-11-02）

- `settle_shadow` 识别出「上次有草稿 / 有冲突」之后不再保留，改为**以磁盘为准重铺**，
  并打一条 WARN 留痕；返回值恒为 `false`。
- `Ctx::open_at` 里 `if recovered { Txn::recovered() }` 那支删掉，`diskStamp` 每次打开
  都重记（本来就该如此：草稿不留，就没有"跨重启的冲突"要保护）。
- 两条 e2e 判据按新口径改断言：`e2e_conflict_survives_reopening`（改名口径：
  重开丢弃草稿、跟随磁盘）与 `e2e_missing_base_never_silently_drops_drafts`
  （基准缺失也不再兜草稿）。

### 刀 5 · 发布身份边界 —— ✅ **已经满足**（复核结论）

两个结构本来就是分开的，不存在"生成成功被读成发布成功"：

- `receipt::GenerationReceipt`：只有生成那一路（id / steps / delivery / assets / files），
  **没有任何发布字段**；
- `publish_tx::PublishTxReport`：`published_at` / `revision` / `publish_serial` /
  `catalog_id` 全部 `skip_serializing_if = None` —— 审计没过就如实省掉，不编值。

AGENTS.md §3 早就禁止手拼这三样。**不改代码。**

---

## 6. 不做的事

- 不把草稿放纯内存（作者否了）：数据量大，且工作区本来就在本机目录。
- 不做物理换根式的"影子树改名"（作者否了）。
- 不删保存事务的阶段机 / 备份 / 恢复日志（作者明确要求保留）。
- 不让 AI 碰 `publishedAt` / `publishSerial` / `catalogId` / SHA（AGENTS.md §3 已有硬规矩）。
