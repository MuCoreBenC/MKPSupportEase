# 把「挪塔那一问」挪进内核第 6 步 —— 交接文档（方案 B，待拍板）

> 状态：**只写了设计，一行代码没动**。作者 2026-10-18「先做交接文档吧，我等会认真考虑一下」。
> 配套阅读：`docs/SMART-TOWER-RELOCATION-HANDOFF.md`（那一问现在的实现、报告页、归档字段）。

---

## 0. 一句话

把"撞了就停下来问人"这件事，从**管线开跑之前的预检**挪到**管线第 6 步 `Step::Collision` 内部**：
撞了就地暂停、问、按答复改 `wiper_x/y`、复检、继续跑完剩下的步骤。

换来的三件事：**进度口径天然统一**（问在 ~55%，答完接着走）、**少扫三遍 G-code**、
**记录里能看到"停在第 6 步"**（现在那一栏是空的）。

---

## 1. 为什么（现状三条事实，都对着代码核过）

### 1.1 预检不发任何进度

`src-tauri/src/hook.rs::run`：

```
read_to_string(&job.gcode)        ~694   ← 读第一遍
…IR / 机型核对…
if ir.prefs.auto_tower_position   ~811   ← 预检块
    detect_mode / decide_wiping / apply_wiping_decision（在 probe 克隆上）
    detect_tower_collision        ~823
    ask_tower_relocation          ~845   ← 弹窗在这里
process_with_ir(...)              ~983   ← 管线在这里才开始
```

这一段**没有 `sink.emit` / `StepTimer`** ⇒ 撞塔那一刻快照里 `progress = null`，
hero 画的是 `percent = 0 / step = '准备中'`。**那个 0% 既不是预检进度，也不是假进度，
是"没有数据"** —— 但用户看到的就是"处理了 0% 就停住了"。

### 1.2 正式管线从 0 计

`crates/postprocess/src/pipeline/mod.rs` 第一条进度是 `Step::Input` 0%（407 行），
前端 `percentOf()` 把 9 步映射成 0→100 ⇒ 答完必然从 0 起。

前端 `bind(PROGRESS_EVENT, (prev, p) => ({ ...prev, progress: p }))` 是**直接替换**，
但因为预检从没发过进度，不存在"覆盖"——**这也意味着方案 B 不需要改前端进度那一层**。

### 1.3 重复扫描（四个）

| # | 重复的东西 | 预检（hook.rs） | 管线（pipeline/mod.rs） |
|---|---|---|---|
| 1 | 读文件 | `read_to_string(&job.gcode)` ~694 | `Step::Input` 再读 + sha256（407） |
| 2 | `detect_mode` | ~819 | step 4（~470） |
| 3 | `decide_wiping` / `apply_wiping_decision` | `probe` 克隆上（~820-822） | step 5（~485） |
| 4 | `detect_tower_collision` | ~823 | step 6（488） |

量级：15:35 那条记录「管线计时 9 步 · 8129 毫秒」，总耗时 14.0 秒 ⇒ 预检＋归档＋外壳 ≈ 6 秒。

⚠️ **方案 B 只消 2/3/4 三处**：第 1 处消不掉 —— 钩子仍然要读一遍文件来算 facts、备份原件、
核对机型（这几件事发生在管线之前，且失败要能说人话）。真想连这一处也消，得让管线把
`content` 回吐给钩子（改 `ProcessResult`），那是**另一刀**，别混进来。

---

## 2. 目标语义（这一条先定，再谈实现）

界面上的进度条描述的是：**"从打开这份 G-code 到产出结果"这一整件任务**。

```
0% ──── 读输入 / 配置 / 机型 / 解析 / 支撑 ──── ~55%（第 6 步入口）
                                                │
                                     撞了 → 停在 ~55%，弹窗（倒计时照走）
                                                │
                     答"挪塔" → 复检 → 接着从 ~55% 往下走 → 100%
                     答"自己挪" / 超时 → 结算为失败（停在第 6 步）
```

要点：**等待期间进度停住是正确表达**（任务真的停在那儿等人），不是"卡死"。

---

## 3. 设计

### 3.1 内核只认一个窄接口（不认归档、不认 UI 类型）

`crates/postprocess/src/postproc/disk/collision.rs`（或新建 `collision/ask.rs`）：

```rust
/// 第 6 步撞了之后要不要问人 —— 内核只认这一条缝。
/// 内核不知道"界面""归档记录"这些东西的存在。
pub trait TowerAsk {
    /// 问一次。返回新的塔左下角（机器坐标）；`None` = 不问 / 用户选择自己挪模型 / 超时。
    ///
    /// `cancel` 由实现方自己看（钩子侧那一份已经在等答复的循环里看它）。
    fn ask(
        &mut self,
        facts: &CollisionFacts,
        candidate: Option<[f64; 2]>,
        current: [f64; 2],
    ) -> Option<[f64; 2]>;

    /// 复检结果回告（内核替它重算了一遍 `detect_tower_collision`）。
    /// 默认空实现 —— 只想知道"后来还撞不撞"的实现方才覆写。
    fn verified(&mut self, _still_colliding: bool) {}
}
```

### 3.2 入口：**加一个新函数，不改 `IrProcessRequest`**

`IrProcessRequest` 是按值传的结构体。往里塞 `Option<&'a mut dyn TowerAsk>` 会给它加一个
生命周期参数 ⇒ 5 个构造点（`hook.rs` 两处、两个测试）全要改签名，而且以后每加一条缝都再来一次。
改成**并列的入口**，老入口原样保留（内部转调新入口、传 `None`）：

```rust
pub fn process_with_ir_with_tower_ask(
    request: IrProcessRequest,
    ask: Option<&mut dyn TowerAsk>,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<ProcessResult, PostprocError>;

pub fn process_with_ir_deferred_print_time_with_tower_ask(…) -> …;  // 同上，延后打印时间那一档
```

`Job`（私有）加一格 `tower_ask: Option<&'a mut dyn TowerAsk>` —— `Job` 是内部结构，加生命周期
只影响这一个文件。**公开 API 的其它三个函数一字不改。**

### 3.3 第 6 步的改法

`pipeline/mod.rs` 现在（488-506）：

```rust
if need_collision_check(calib_mode) {
    let collision = detect_tower_collision(&content, &ir);
    if collision.has_collision {
        return Err(PostprocError::Collision { facts: Box::new(collision.facts(&ir)) });
    }
}
```

改成（伪代码，注释按项目习惯写足）：

```rust
if need_collision_check(calib_mode) {
    let collision = detect_tower_collision(&content, &ir);
    if collision.has_collision {
        let facts = collision.facts(&ir);
        /* 没人问（CLI / 没装界面 / 开关关着）⇒ 一字不差的原行为 */
        let Some(ask) = job.tower_ask.as_deref_mut() else {
            return Err(PostprocError::Collision { facts: Box::new(facts) });
        };
        let current = [ir.wiping.wiper_x, ir.wiping.wiper_y];
        /* 候选位置在这里算：`collision` 手上就有 occupancy 与 tower_height，
           与预检那条是同一个函数（`auto_tower_position_for_facts`） */
        let candidate = disk::auto_tower_position_for_facts(
            &ir, &facts, &collision.occupancy, collision.tower_height,
        );
        let moved = ask.ask(&facts, candidate, current);
        let Some([x, y]) = moved else {
            /* 自己挪 / 超时 / 按了停止 —— 仍是"撞了"，交给调用方按它手里的记录说话 */
            return Err(PostprocError::Collision { facts: Box::new(facts) });
        };
        ir.wiping.wiper_x = x;
        ir.wiping.wiper_y = y;
        /* ★ 复检：同一个判据、改完之后的 IR。仍撞就**不静默继续** */
        let after = detect_tower_collision(&content, &ir);
        ask.verified(after.has_collision);
        if after.has_collision {
            return Err(PostprocError::Collision { facts: Box::new(after.facts(&ir)) });
        }
    }
}
```

注意：
- 这一段后面（step 7 起）一字不用改：`wiper_x/y` 变了，pass1/pass2 自然按新塔位算。
- `timer` 在 `Step::Collision` 里已经 `begin` 过，等待时**不会再发新事件** ⇒ 进度条停在
  第 6 步入口那一帧（~55%）。这正是我们要的"停在这儿等人"。
- 复检那一次 `detect_tower_collision` 是**新增的第二次扫描**，但只发生在"撞了且挪了"这一条路上；
  预检被删掉之后，正常路径的扫描次数**净减少一次**（原来恒定扫两次）。

### 3.4 钩子侧：谁记 relocation、谁说话

`hook.rs` 里实现 `TowerAsk` 的那个小结构自己记账（**内核不碰归档类型**）：

```rust
struct ChannelTowerAsk<'a> {
    client: &'a hook_ipc::Client,
    /// 这一趟的选择与几何 —— 记在这儿，`run` 之后原样搬进 `RelocationRecord`
    record: Option<RelocationRecord>,
    tower_height_mm: f64,     // 从 facts/几何带出来的
}
impl TowerAsk for ChannelTowerAsk<'_> {
    fn ask(&mut self, facts, candidate, current) -> Option<[f64;2]> { /* 与今天的 ask_tower_relocation 同一套：拼 choices、ask_with_geo、tower_relocation_of */ }
    fn verified(&mut self, still: bool) { /* 填 record.verified / after_conflict_count */ }
}
```

`run()` 之后（成功或失败）从 `asker.record.take()` 拿到 `RelocationRecord`，走今天那条
`archive_*(…, relocation.as_ref())` —— **归档格式一个字都不用改**，报告页那两幅板子照旧。

### 3.5 取消 / 超时 / 无界面

| 情形 | 今天 | 方案 B |
|---|---|---|
| 用户按「停止」 | `run` 的 `cancel` 线打断 `ask_with_geo` 的等待循环 | 同一条线，`ask` 实现里照样看 `cancel`；内核拿到 `None` → `Err(Collision)`，外层再看 `cancel.is_cancelled()` 结算成"已停止" |
| 118 秒没人答 | `ask_with_geo` 返回兜底 `stop` ⇒ `None` | 一模一样（这段代码整块搬过去，不改） |
| 无界面 / 静默档 | 预检块根本不进（`ui.client()` 为 `None`） | 钩子侧 `ask = None` ⇒ 内核走"没人问"那条短路 |
| 开关 `auto_tower_position` 关 | 预检块不进 | 钩子侧不装 asker ⇒ 同一条短路 |

### 3.6 错误与文案的分派

今天是**预检块自己**决定说哪句（`TOWER_RELOCATE_DECLINED` / `_UNAVAILABLE` / `_RELOCATED_STILL`）。
搬进内核后，内核只会返回 `PostprocError::Collision`，所以：

- 保留一个**只读**的小函数 `tower_failure_reason(&RelocationRecord) -> MsgKey`：
  `choice == "unavailable"` → `_UNAVAILABLE`；`choice == "model"/"timeout"` → `_DECLINED`；
  `verified == Some(false)` → `_RELOCATED_STILL`。
- 钩子把 `PostprocError::Collision` 翻成 `HookError` 时，**若 `relocation.is_some()` 就用这个
  函数挑 key**（今天那条 `err.collision = Some(facts)` 的填充也在这里做）。
- 判据：同一份输入，改造前后**四句话一字不差**（`messages.toml` 的键与参数都对得上）。

---

## 4. 要改的文件清单

| 文件 | 位置 | 改什么 |
|---|---|---|
| `crates/postprocess/src/postproc/disk/`（或 `collision.rs` 末尾） | 新增 | `pub trait TowerAsk`（3.1） |
| `crates/postprocess/src/pipeline/mod.rs` | 90-100（`IrProcessRequest` 旁） | 不改结构；新增两个 `*_with_tower_ask` 入口 |
| 同上 | `struct Job`（~118） | 加 `tower_ask: Option<&'a mut dyn TowerAsk>`（私有，生命周期只在本文件） |
| 同上 | 488-506 | 第 6 步按 3.3 改 |
| 同上 | `run()` 签名 | 收 `Job`，不用改（`Job` 里已经带着 asker） |
| `crates/postprocess/src/lib.rs` | re-export | `pub use ...TowerAsk` |
| `src-tauri/src/hook.rs` | 391-430（`trait Asker`） | `ask_tower_relocation` 从 `Asker` 上摘掉（它不再属于"钩子问界面"的通用面），搬进 3.4 的适配器 |
| 同上 | ~811-960 | **删掉整个预检块**（连同它那三段重复扫描与 `probe` 克隆） |
| 同上 | ~983 | 改成 `process_with_ir_with_tower_ask(request, asker, sink, cancel)`；**注意 `ir` 不再被预检改过**，归档里 `relocation` 从 `asker.record` 取 |
| 同上 | 错误映射那一处 | 用 `tower_failure_reason()` 挑 key（3.6） |
| `crates/postprocess/tests/process_with_ir.rs` | — | 加"没人问 ⇒ 行为不变"的判据；现有用例不动 |
| `src-tauri/src/hook.rs` 的 `#[cfg(test)]` | — | 预检相关的单测要重写成"asker 被调到 / 没被调到"两类 |

前端：**零改动**（1.2 那条推论）。

---

## 5. 判据（先写这些，再动代码）

1. **没人问 = 行为一字不差**：`process_with_ir`（老入口）对同一份输入的输出 G-code
   与今天**逐字节相同**，`PostprocError::Collision.facts` 也相同。
2. **问了且挪**：`ask` 返回候选位置 ⇒ 输出里没有冲突（用 `reference` 那份 `a.gcode` 之类
   已知会撞的样本），`verified(false)` 被调到。
3. **问了但复检不过**：`ask` 返回一个仍然撞的位置 ⇒ `Err(Collision)`，`verified(true)`。
4. **问句期间取消**：`ask` 里 wait 时 `cancel.cancel()` ⇒ 内核在 `checkpoint` 处返回
   `Cancelled`（不是 `Collision`）。
5. **进度不回退**：一次"撞→挪→继续"的整趟里，`ProgressEvent` 的 `percentOf` 序列**单调不减**，
   且等待段只有一条（第 6 步入口）。
6. **归档**：成功 / 自己挪 / 超时 / 复检不过 四条，`_meta.json` 的 `relocation`
   （含 `before`）与今天**同构**；"自己挪"那两条的 `pipeline.steps` 从"空"变成"1-6 步"
   （见 §6）。
7. **钩子层**：`auto_tower_position` 关 / 静默档 / 无界面 三种情况，`ask` 一次都不被调，
   归档里没有 `relocation`。
8. 全套：clippy ×2 `-D warnings`、`cargo test`（589 / 965 / 194）、前端 `tsc + eslint + build`、
   `scripts/probes/pp-modal-height.mjs`。

---

## 6. 会变的可见行为（作者需要知道，也是这次"认真考虑"的重点）

1. **弹窗时机**：从"窗口一开就弹"（0%）变成"跑到第 6 步才弹"（~55%）。
   好处是用户先看见进度在动、知道程序在干活；坏处是**要等几百毫秒到一两秒**才弹
   （读文件 + 机型 + 解析 + 支撑）。
2. **等待期间进度条停住**（停在 ~55%），这是对的，但用户第一次见可能以为卡住
   —— 问句那一屏的倒计时会说明"1 分 57 秒后自动终止"，已经够。
3. **失败记录的「管线计时」不再是 0 步 0 毫秒**：自己挪 / 超时那两条会显示跑到第 6 步
   （报告页那一栏从空变成 1-6 步）。这是更正，不是退化。
4. **整趟更快**：少扫 `detect_mode` / `decide_wiping` / 一次 `detect_tower_collision`。
   预计省 0.5-2 秒（取决于文件大小），大文件上更明显。
5. 预检那句"提前问"就没了 —— **不再有"开跑前就知道会撞"这回事**。如果作者其实喜欢
   "先问再干活"，那就该选方向 A（保留预检、只把进度折算进 0-10%），而不是 B。

---

## 7. 分步落地（三刀，每刀都能独立验收、都能停）

- **第 1 刀（无行为变化）**：加 `TowerAsk` trait + `*_with_tower_ask` 入口 + 第 6 步那段
  "问了就改、复检、继续"。判据 1 / 3 / 4 / 5 在这一刀钉住。**钩子一行不动**（还是预检那条路），
  所以真机上行为完全不变 —— 这一刀可以单独合并。
- **第 2 刀（切流量）**：钩子侧实现 `ChannelTowerAsk`，删预检块，改入口调用与错误映射；
  归档仍用 `RelocationRecord`。判据 2 / 6 / 7。
- **第 3 刀（收尾）**：删掉预检留下的死代码与测试（`tower_relocation_of` 若无人用一并删）、
  更新文档与报告页文案（若有提到"开跑前预检"的地方）。

回滚：第 2 刀是一次 commit 的事 —— 恢复预检块 + 改回 `process_with_ir` 即可；
第 1 刀的内核增量对老入口没有任何影响（`ask = None` 走原分支）。

---

## 8. 风险

| 风险 | 说明 | 处置 |
|---|---|---|
| 内核里"等人" | `ask` 最长 118 秒，这期间管线握着 `content`（内存）不动 | 可接受：今天预检也是等 118 秒，只是那时管线还没开始。文件没写、锁没拿 |
| 错误语义被稀释 | 内核只会说 `Collision`，四句话靠 `tower_failure_reason` 还原 | 判据 §3.6：改造前后四句一字不差 |
| `Job` 加生命周期 | 只在 `pipeline/mod.rs` 内部 | 不改公开 API（3.2） |
| 复检仍撞但 `ask` 说"挪好了" | 内核算、不信回调 | 3.3 的 `verified` + `Err(Collision)` |
| 测试面 | `hook.rs` 预检那一批单测要重写 | 第 2 刀里做，判据 7 兜住 |

---

## 9. 请作者拍板的三件事

1. **A 还是 B？** 如果你其实喜欢"开跑前就知道会撞、先问再干活"，选 A（改动小、进度靠折算）；
   如果你要"进度真实、不重复扫、问在 55%"，选 B。
2. **如果选 B，失败记录的"管线计时"从 0 步变成 1-6 步**，接受吗？（我倾向接受：那是事实。）
3. **要不要顺手把"文件读两遍"也消掉**（让管线把 `content` 回吐给钩子）？
   我建议**不要**混进这一刀 —— 那条会动 `ProcessResult`，影响面比这一刀大。

---

## 附：相关代码坐标速查

| 东西 | 位置 |
|---|---|
| 预检块（要删的） | `src-tauri/src/hook.rs` ~811-960 |
| 挪塔问句拼装 | `src-tauri/src/hook.rs` ~1690（`ChannelAsker::ask_tower_relocation`） |
| `tower_relocation_of` 分派 | `src-tauri/src/hook.rs` ~425 |
| 第 6 步碰撞 | `crates/postprocess/src/pipeline/mod.rs` 488-506 |
| `IrProcessRequest` | `crates/postprocess/src/pipeline/mod.rs` 90 |
| `CollisionResult` / `CollisionFacts` | `crates/postprocess/src/postproc/disk/collision.rs` 131 / 68 |
| `auto_tower_position_for_facts` | 同上 735 |
| 归档 `RelocationRecord` | `src-tauri/src/archive.rs`（含 `before`，2026-10-18 加的） |
| 问句那一屏 / hero 互斥 | `src/app/postprocess/PostProcessModal.tsx`（`showAsk` / `answering`） |
