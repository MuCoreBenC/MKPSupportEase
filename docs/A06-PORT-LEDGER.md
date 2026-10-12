# a06 后处理那一屏 · 搬运台账（2026-10-12）

> 源 = `frontend-lab/src/versions/a06/postprocess/`（试验场定稿）＋
> 同仓 `lab/mkp-postprocess/`（那一屏的抽卡原型，58 轮定稿，作者逐像素验收的那一套）；
> 落点 = 本仓 `src/app/postprocess/`、`src/app/ui/Controls.tsx`、`src/api/contract.ts`。
> 方向裁决沿用 [`A05-PORT-LEDGER.md`](A05-PORT-LEDGER.md) 那条：
> **试验场前端 → 本仓**（本仓是唯一产品本体，Rust / 发布链 / CI 不动）。

## 1. 为什么有这一刀

作者 2026-10-12 在试验场里看到后处理弹窗还是旧样子（facts 表 + `Progress` 条 +
「✓ 处理完成」一行），问的是："我们才刚做好了后处理的……你有没有帮我用过去
MKPSupportEase 呀？" —— 原型（`lab/mkp-postprocess/`）里的六态定稿一直没落进真源码。
这一刀就是把那份定稿整份搬过来，**旧的实现删掉，不留两套**。

## 2. 搬了什么

| 文件 | 动作 |
| --- | --- |
| `app/postprocess/PostProcessModal.tsx` | 整份换：六态定稿 —— ①绿 hero 进度（大数 / 步骤 + 倒计时 / 凹槽条）②机型不匹配红卡（强制继续 / 终止）③成功指标条（笔偏移 X/Y/Z + 涂胶速度）④已停止 sm4 柔和内影 ⑤失败红卡（`failWarn`）⑥结果那几扇无 ×（收窗走页脚「关闭」） |
| `app/postprocess/PostProcessModal.module.css` | 整份换：定稿的几何与配色（`.hero*` / `.warn*` / `.strip*` / `.res*` / `.sm*` / `.failWarn` / `.groups` / `.pill`）+ `[data-theme='dark']` 那一档（跟着应用主题走，不另开开关） |
| `app/postprocess/PostProcessStrip.module.css` | 只动一条：`.open`（「看进度」）竖向内衬 2 → 5 —— 作者两次提"按钮没变高" |
| `app/ui/ui.module.css` + 五处页面私有按钮 | **按钮语言定稿**（2026-10-12）：① 胶囊形状**收进 `.btn` 默认**（原来只有 `[data-page]` 页面槽里才是胶囊，窗口级弹窗拿不到 ⇒ 一个程序两种按钮；`postprocess` 里那份抄来的 `.pill` 已删）；② `.btnPrimary` 改**反色配对**（`--text-1` 底 / `--bg-1` 字 ⇒ 浅色墨底白字、深色**自动**亮底深字）、深色 `.btnAccent` = 实心绿 + 白字、`.btnDanger` 与 danger 徽章的字色走 `--danger`（原来写死暗红，深色底上读不出）；③ 各页那批"墨底白字"实心按钮（首页 `.cta`、预设页 `.useBtn/.actBtn/.saveBtn`、校准浮动条 `.changeSave`、参数同步 `.primary`、复制按钮 hover）一起换成同一配方，hover/active 的写死深灰换 `--text-2`，**它们各自的深色临补丁全删**（那些补丁是"换中性亮面 + 主文字色"，在深色里与卡片底同色 —— 作者截图："深色的时候一些按钮的重点就没了"） |
| `api/contract.ts` | 新增 `PostProcessResult`（**原始数值**口径）+ `PostProcessFinished.result?` + `PostProcessPrintTime`（见 §4） |
| `src-tauri/src/hook_ipc.rs` | `FinishedPayload.result`（新 `ResultFacts`）+ `FactsPayload` 三格（涂胶速度 / 切片软件 / 切片器自报时长）—— 见 §4 |
| `src-tauri/src/hook.rs` | `result_facts()`：从内核产物里摘读数（只搬不算）；`facts_from` 补三格 |
| `src-tauri/src/hook_ui.rs` | `PRINT_TIME_EVENT` + `complete_archive` 返回秒数并推那条事件 |
| `src-tauri/src/archive.rs` | `glue_speed_of()`：mm/min → mm/s 的**唯一换算点**（归档与开场事实共用） |
| `crates/postprocess/.../printtime.rs` | `slicer_estimated_seconds()`：切片器自报时长的小口子（开场就能给） |
| `src-tauri/src/bin/hook-sim.rs` | 假钩子同步新字段（开发时能看全那几格） |

原型里那一套 `cd-*` / `sm-*` 类名在本仓换成了本地语义类名（`hero` / `warnHero` /
`resHero` / `stripFinal` / `smSunk`…），数值与选择器结构照搬。

## 3. 数据口径（真机给什么就摆什么）

- 左栏「机型与切片基础」四格全部来自 `PostProcessFacts`（层高那一行没有"首层"，
  原型的"首层 / 层高"落成"层高"）；右栏跑着是「切片与预估」、跑完是「擦料与胶水消耗」。
- 「处理耗时 / 大小」= `PostProcessFinished.elapsedMs` + `facts.fileSizeBytes` —— 现成。
- 文案按定稿处理：成功态**不再摆那一句 message**（结论由指标条 + facts 说）；
  已停止去掉「已停止 —— 」半句前缀（标题那行就是「已停止」）；
  失败**不说"停在哪一步"**（原型第 46 轮作者点名删），错误码与退出码并成一行。
- 机型不匹配那一张：标题是固定的「预设与机型不匹配」，正文用后端那一句
  `question.text`（不另编文案）；两颗按钮「强制继续 / 终止」= `answerPostProcessMismatch(true/false)`。

## 4. 读数那一刀（同一天的第二刀，已做）

`FinishedPayload.result`（`ResultFacts`）由钩子在成功那一条里报上来 ——
值全从内核产物里搬（`hook::result_facts`：IR 终态 + pass1 统计 + 打印时间）：

| 格 | 来源 |
| --- | --- |
| 笔偏移 X / Y / Z | `ir.toolhead.x_offset / y_offset / z_offset` |
| 涂胶速度 | `archive::glue_speed_of`（mm/min → mm/s，**全仓唯一换算点**，与归档 `glueSpeed` 同一处） |
| 涂胶总长 | `stats.pass1.glue_total_length`（**不换算成体积** —— 全仓没有 ml，报告页也是 mm，不编） |
| 切片软件 | `stats.pass1.slicer_version`（`"BambuStudio 02.06.01.55"`） |
| 预估打印秒数 | `done.print_time`；**有界面时延后算 ⇒ 先没有**，由 `PRINT_TIME_EVENT` 补 |

**开场事实**（`FactsPayload`）同一天补了三格，让「跑着」那一扇的右栏也摆满：
`glue_speed`（IR）、`slicer`（IR 里那个名字）、`slicer_print_time_seconds`
（输入头 `; total estimated time` 那一行 —— `printtime::slicer_estimated_seconds`）。

**打印时间是后补的**：钩子不等它（切片器在等退出码），常驻进程算完推
`postprocess-print-time`（`hook_ui::complete_archive` 把算出的秒数返回给调用方再发）——
界面把那一格填进**已经在屏上的结论**里（`usePostProcessRun` 订阅）。算不出来就不发，
那一格继续空着（**不许拿 0 凑数**）。

失败 / 取消那一趟**没有读数**（内核还没把那几个数算出来）——界面摆的是
开场事实里有的那几格 + 「处理耗时 / 大小」（10-12 真机实测：已停止 / 失败两档是 3 行）。

## 5. 验证与怎么看

- 静态检查全绿（2026-10-12，两刀都跑过）：`npx tsc -b`、`npx stylelint "src/**/*.css"`、
  `npx eslint src/app/postprocess src/app/ui/Controls.tsx src/api/contract.ts`；
  `cargo fmt --all`、`cargo clippy --all-targets` 两种 feature（`-D warnings`）、
  `cargo test` 两种 feature（486 / 839 通过）。
- 新增的钉子：`hook::tests::the_opening_facts_come_straight_from_the_ir`（开场三格 +
  「头里没有那一行就不摆」）、端到端那条加了 `out.result` 的四格断言、
  `hook_ui::tests::deferred_print_time_is_completed_by_the_display_side`（补出的秒数
  要交回给调用方，失败路径是 `None`）。
- 浏览器预览（`npm run dev` ⇒ `?mock=1`）**看不到这一屏** —— mock 的
  `getPostProcessRun()` 如实答 `null`（浏览器里没有那一趟进程）。
  要看六态请在试验场：`frontend-lab` 预览外壳的「预览状态 → 后处理 · 进行中 / 等你回答 /
  成功 / 已停止 / 失败」；真机验收就跑一次切片（或 `hook-sim`）。
