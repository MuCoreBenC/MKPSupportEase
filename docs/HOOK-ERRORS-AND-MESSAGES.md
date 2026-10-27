# 后处理的错误：谁看到哪一句话（2026-10-11）

> 适用分支：`feat/postprocess-progress-modal`（2026-10-11 那一刀）。
> 这一篇管三件事：**切片器那个报错框到底显示什么**、**我们的话从哪来**（一张表）、
> **日志为什么有两条流**。改文案、加错误码、排查"用户看到的是什么"之前先读它。

---

## 1. 切片器那一框到底吃什么（查了它的源码，不是猜）

对象 = Bambu Studio 的后处理那一段：`src/libslic3r/GCode/PostProcessor.cpp`
（本机源码副本 `G:\code_study\260412\BambuStudio_source`，行号为该副本）。

失败时它抛的那一句在 `run_post_process_scripts`（§315-321）：

```text
Post-processing script %1% on file %2% failed.
Error code: %3%
Output:
%4%            ← %4% = 钩子的 stderr
```

★ **`Output:` 那一段只在 `%4%` 非空时才拼**（同一处的三元表达式）。而 `%4%` 从哪来，
两个平台**根本不是一回事**：

| 平台 | 怎么起这个进程 | 我们的 stderr 去哪了 |
| --- | --- | --- |
| macOS / Linux | `#else` 那支（§156-186）：`process::child(shell, "-c", command, process::std_err > istd_err)`，把子进程读到底 | **被接走并显示**在 `Output:` 段里 —— 我们写什么，用户就看到什么 |
| Windows | `execute_process_winapi`（§63-99）：`CreateProcessW(..., bInheritHandles=false, CREATE_UNICODE_ENVIRONMENT, ...)`；`run_script` 那个 `std_err` 形参**压根没被用过**（§104 `/*std_err*/`） | **哪儿都没去**（没有任何重定向）⇒ 框里只有上面两行的命令 + `Error code: N` |

推论（都是"能不能控"的问题）：

1. **macOS 上是"我们说了算"**：那一框的内容就是钩子的 stderr。所以「让切片器多显示 /
   少显示」= 改我们往 stderr 写什么（这就是 §2 里 `slicer` 那一面存在的理由）。
2. **Windows 上我们说了算的只有退出码**（`Error code: N`）—— 那一支连管道都没建，
   想把文字塞进去**没有通道**，别在这儿花力气。
3. 于是"用户在 Windows 上到底看到了什么原因"这件事，**只能由我们自己的界面回答**：
   后处理模态框（原因 + 错误码 + 停在哪一步 + 退出码）与报告页那一条记录。窗口与记录
   现在都在，这条链是通的。

> 作者 2026-10-11 的两张截图（同一台机器、同一份 G-code）：macOS 那一框里
> `Output:` 完整带着我们那句 `处理失败 [E_CFG_INVALID_001]：配置无效: E_GCODE_BOUNDARY_001: …`；
> Windows 那一框只有命令 + `Error code: 1` —— 与上表逐条对上。
> 那句"两层码"的丑话也是那一刀要修的（见 §2 末尾）。

---

## 2. 一句话从哪来：一张表

**表 = `crates/postprocess/messages.toml`**（`crates/postprocess/src/diag/messages.rs`
用 `include_str!` 读它 —— **编译进产物**，不是运行时配置文件）。

### 面

同一条错误，三个面各说各的：

| 面 | 谁看 | 用表里哪个字段 | 形态 |
| --- | --- | --- | --- |
| 切片器 | 用户的切片器（macOS 那一框） | `slicer`（缺省 = `text`） | 一行结论：`处理失败：<原因>［<码>］` + 一行 `完整日志：<路径>` |
| 界面 + 执行记录 | 后处理模态框、报告页那条记录 | `text` | 原因 → `message`，码 → `code` 字段（不夹在句子里） |
| 日志 | `logs/hook.log.<日期>` | `text` + 结构化字段 | 带 `code=` / `exit_code=` 字段，便于按码检索 |

界面与切片器的差别是**刻意的**，例子就在表里：`E_CAL_MISMATCH_001` 的 `text` 结尾带着
一串机器判据（`expected=… actual=…`，排查要），`slicer` 里**没有**它 —— 用户要看的是
"别转我的校准模型"。

### 键

- `E_*_NNN` 开头的键 = **内核错误**。`PostprocError` 的每个码都必须有一条，
  判据 `every_variant_has_a_message_entry` 钉着（没写表 = 构建期红）。
- `hook.*` 的键 = **钩子层**的说法（前置检查、机型不匹配、取消、看门狗、归档降级警告、
  结论行的框子）。键名常量住 `src-tauri/src/hook.rs` 的 `keys` 模块，
  判据 `every_key_the_hook_uses_exists_in_the_table` 逐条问表里有没有。
- `boundary.*` = 越界那句的几个零件（从句 / `≥` `≤` 的说法 / 分隔符）—— 它们同样是文案。

### 码与"该不该给码"

对外码由**表**决定：条目写了 `code` 就用它，键本身就是码（`E_*`）就按键算。
于是"这一档给不给码"是数据、不是散在各处的 if：

- 用户自己按的停止（`hook.cancelled`）**没有码** —— 那是他的选择，不是错误；
- 看门狗那两种叫停（`hook.stalled` / `hook.overrun`）**有码**
  （`E_SYS_STALLED_001` / `E_SYS_TIMEOUT_001`，新造码，见 `diag/error.rs` 模块头的登记）
  —— 那是系统替用户停的，报问题时得能对上号。

### 为什么拆掉"字符串里夹码"

改之前：内核 `#[error("配置无效: {message}")]` 里塞着 `E_GCODE_BOUNDARY_001: …`，
钩子再拿"最后一个像 `E_*_NNN` 的 token"当码、裁掉前缀当原因（旧 `split_code`）。
用户读到的是 `处理失败 [E_CFG_INVALID_001]：配置无效: E_GCODE_BOUNDARY_001: 模型超出打印边界…`
—— 两个码、一句假话（"配置无效"是变体名，而真因是越界）。

改之后：错误只带**码 + 参数**（`PostprocError::params()` / `hook::Reason`），句子在渲染那一刻
从表里取。判据 `the_rendered_sentence_never_carries_a_code` 咬住"人话里不许再出现码"。

### 改一句话 / 加一条错误

1. 改**文案或可见性**：只改 `messages.toml`（`text` / `slicer` / `params`），重新编译。
   表是自检的：占位符与 `params` 登记不一致、有占位符没登记、登记了没用上 —— 判据全红。
2. 加**新错误码**：`diag/error.rs` 加变体（`code()` 的 match 不写通配臂，漏了编译不过）
   → 表里加一条同名键 → 若它有动态值，`params()` 里给它名字。
3. 加**钩子层的说法**：`keys` 模块加常量 + 写进 `ALL_KEYS`（判据要遍历）→ 表里加条目。

---

## 3. 退出码（没变，三档）

`0` 成功 / `1` 处理失败或取消 / `2` 输入·预设错。**刻意不按错误变体分**
（理由与代价写在 `crates/postprocess/src/main.rs` 模块头）：按变体分会把"模型超出打印边界"
报成"配置错"，因为边界检查复用的就是 `InvalidConfig` 这个变体。

Windows 上它是**唯一**能进切片器那一框的东西（§1），所以这三档继续照原样守着。

---

## 4. 日志：两条流，不许混

| 流 | 住哪 | 记什么 | 谁装 |
| --- | --- | --- | --- |
| 应用日志 | `<内部根>/logs/supportease.log.<日期>` | 界面那个常驻进程：命令、IPC、数据根、发布链 | `obs::tracing::init_tracing`（启动时） |
| 后处理日志 | `<内部根>/logs/hook.log.<日期>` | 这一趟：哪份输入、哪个预设、内核的 warn（机型维度缺失 / 收笔 G-code 未定义…）、看门狗叫停、结论 + 退出码 | `obs::hook_log::init`（钩子进程入口） |

两条流分家的理由（作者 2026-10-11 点名要的）：读者不同、量级不同、**进程也不同**
（后处理是切片器拉起的那一次，跑完就退）。混在界面日志里，这几条会被启动与 IPC 的噪音埋掉。

★ 钩子这条**只有文件、刻意不叠 stderr**：stderr 是对外契约（§1），多一行 INFO 就会把
真正的原因往下推。失败/取消时结论行后面会跟一行 `完整日志：<路径>`，开发者与用户都靠它找到
这一份日志。

---

## 5. 问句队列：钩子中途要用户挑一件事

后处理跑到一半可能要**停下来问一句**。到 2026-10-17 为止有两种，走的是同一条线
（`FromHook::Question` → 界面答 → `ToHook::Answer { id }`）：

| `questionKind` | 什么时候问 | 选项 id | 超时 / 不答 |
| --- | --- | --- | --- |
| `machine-mismatch` | G-code 的机型与预设的机型对不上 | `keep`（强制继续）/ `stop`（终止） | `stop` |
| `calibration-print` | 内核认出这是**校准件**（Z 偏移 / XY 精度 / XY 粗 / 重复性） | `print`（先打印，再校准）/ `direct`（直接校准） | `stop` |

**排队 = 识别先后**（作者 2026-10-17 定稿）：先机型、后校准件。前一问答"停"或判成拒绝就
**直接收场**，不再问后面的（都没在跑了，问怎么跑没有意义）。以后新的冲突类问题照这条排，
**不许并排弹**。

**倒计时**：两问都是 `ANSWER_GRACE`(8s) + `ANSWER_STAKE`(112s) = 120 秒，界面上的倒计时读
`QuestionPayload.deadline_unix_ms`（绝对时刻；`0` = 不超时，界面就不摆那一行）。

**两个选项对模型的实际后果**（不是界面在"演"）：`print` = 预设原样跑（保留打印件）；
`direct` = 走校准那一支，去掉打印件、只留空驶路径。选择写进 `ir.safety.calibration_execution_mode`
（`CALIBRATION_EXEC_PRINT_THEN_CALIBRATE` / `CALIBRATION_EXEC_DIRECT_CALIBRATE`），并由内核执行。

**结论文案按选择分键**：`hook.calib_done_print` / `hook.calib_done_direct`，界面（`finished.calibChoice`）
与报告（`_meta.json` 的 `calib_choice`）读同一组键，两处不许各写一份。

**铁律**：新弹窗**一律复用后处理那扇 hero 模态框**（同一口 104px、内容在 `.warnScroll` 里滚）。
颜色只表达语义：警告橙 `--warn-*` = 可能出错、要你拦一下；流程选择 `--ask-*`（绿）= 没出错、
只是请你挑；红只留「处理失败」。

---

## 6. 错误码：**每一句都有码**，前三个字母是类别（2026-10-17）

作者的原话：「这样就方便我给 AI 去看啊，这样子，就算信息不全，也能通过错误码找到对应的」。

所以定了一条硬口径：**用户能读到的每一句都必须带码**。以前只有"失败"才带码，
而**最常出现的那几条**（用户自己按停止、在问句里选了停）是 `code=""` ——
偏偏这几条最需要查。

### 三档前缀

| 前缀 | 意思 | 例子 |
| --- | --- | --- |
| `E_` | **故障** —— 系统判定这件事没成 | `E_CAL_FAILED_001` / `E_CFG_MISMATCH_001` |
| `W_` | **停下 / 降级** —— *不是故障*：用户让它停的，或某一步降级了 | `W_SYS_CANCELLED_001` / `W_ARCHIVE_META_001` |
| `N_` | **成功** —— 跑完了，不同结果各有各的码 | `N_CALIB_PRINT_001` / `N_CALIB_DIRECT_001` |
| `Q_` | **提问** —— 钩子停下来问用户的那几句 | `Q_CFG_MISMATCH_001` / `Q_CALIB_MODE_001` |

`*_NNN` 是序号：**加新条目往后排，不许复用或改号** —— 改了号，用户贴在 issue 里的旧码就搜不到了。

### 怎么查

```bash
node scripts/message-codes.mjs                      # 整张码表
node scripts/message-codes.mjs W_SYS_CANCELLED_001  # 按码查
node scripts/message-codes.mjs hook.cancelled       # 按键查
node scripts/message-codes.mjs 校准                  # 模糊：码/键/文案
```

打出来的是这句的**全部口径**：本程序怎么说、切片器怎么说、用哪些参数、为什么这么写。
排查时只要用户给一个码（或一行日志），就能反查到完整上下文 —— **不需要复现**。

### 判据（钉住的四条）

| 判据 | 钉住什么 |
| --- | --- |
| `every_hook_line_carries_a_code_with_its_class` | 每一条 `hook.*` 都有码，且是 `E_/W_/N_/Q_` 之一 |
| `codes_do_not_collide_silently` | 同码下不许有两句一模一样的话（同码不同话 = 两层各有说法，允许） |
| `code_of_answers_the_three_shapes` | 键即码 / 声明码 / 无码三种取法 |
| `the_conclusion_line_frames_the_three_kinds` | 结论行三档各自的框子与码 |

### 实现上的一条坑

日志那个字段是 `tracing::warn!(code, …)` —— `code` 是 `String`，
**`unwrap_or_default()` 会给出空串**，日志里就成了 `code=""`（等于查不到）。
所以两处取码都改成"兜底给 `W_UNCLASSIFIED_001`"，绝不给空。

---

## 7. 相关

- `crates/postprocess/messages.toml` —— 表本体（字段说明写在表头）
- `crates/postprocess/src/diag/messages.rs` —— 载入与渲染（缺键、缺参数的表现写在模块头）
- `src-tauri/src/hook.rs` —— 模块头：退出码 / stderr 契约 / 三个面
- `src-tauri/src/obs/hook_log.rs` —— 为什么单开一条日志流
- `docs/REPORT-DETAIL-FIELDS.md` —— 执行记录（`_meta.json`）的字段面
