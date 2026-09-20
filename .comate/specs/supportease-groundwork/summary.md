# 骨架收口：做完了什么、哪几条还没验

一句话：**SupportEase 的仓库、闸门、CI、前端、Rust 地基全部就位，第一个 PR 已经按 PR-only 流程
合进 main。**剩下四条验收项需要你的手（GUI 交互与发版），列在最后。

仓库：https://github.com/MuCoreBenC/MKPSupportEase（public，AGPL-3.0-only）

---

## 1. 执行中推翻的三处规格

SDD 的价值在这一轮体现得很直接：doc 里有三处假设是错的，都是在动手时撞出来的。

### 1.1 试验场那七道闸从来没生效过

`scripts/hooks/` 下四个钩子在磁盘与 git 索引里都是 `100644`（不可执行）。git 遇到不可执行的钩子
只打一行 hint 就照常提交 —— `core.hooksPath` 指对了、文件也齐、自检脚本报一串 ok，
因为它只查文件存在、不查可执行位。

处理：`setup-hooks.mjs` 现在自动修磁盘权限，并检查 **git 索引里的 mode**（索引那份才跟着 clone 走），
`100644` 就显眼报出来。试验场那边按约定只读，没动 —— 它的闸门目前仍是装饰品。

### 1.2 服务端 ruleset 对 private 仓库要收费

`GET /repos/:o/:r/rulesets` 在 Free 计划的 private 仓库上直接 403。private + Free 下
第二层根本不存在，而 §2.5 的前提是"只有本地那层等于没拦住"。

处理：**仓库改成 public**（你选的），改的时候库里只有那个空提交，没有内容被暴露过。
由此产生一条常驻约束：任何密钥、用户数据、私有资产都不能进这个仓库 ——
闸⑦（`.env*` / 私钥 / 大文件）从"防手滑"升级成唯一的防线。

### 1.3 v023 不是自洽的

doc §2.3 写着"`v001..v022` 不搬"与"`src/mock/**` 不搬"。实测 v023 的 import 图：
自身 30 文件 7246 行，**外部闭包 35 文件 5920 行** —— 它 import 了 `v003/TopTabs`、
`v005/ui` + 三个页面 + `ReportView`、`v008/CopyAction`、`v014` 四个模块、`mock/mkpFull`
的 6 个符号、以及 `dev/devStore`（914 行，被 6 个 v023 文件用）。

doc 只要求核实 `ExplodedHero` / `SlideDeck` 是否碰 devStore —— 实测 **`ExplodedHero` 根本不碰**
（只用 three + GLTFLoader），真正深度耦合的是 `HeroFade`（14 个符号，含 undo/redo/曲线编辑）。

三处决策（你定的）：

| 议题 | 结论 |
| --- | --- |
| 跨稿 UI | 只搬首页 + 校准页真正需要的，落位到 `src/app/` 按职责命名的目录，不保留稿号。`v005` 的三个页面与 `ReportView` 不搬 → 四处用统一占位页，页签结构不变 |
| mock 数据 | 拆两层：界面结构数据（tabs / brands / models / variants / 后处理脚本）→ `src/app/constants/`；业务假数据（presetIndex / calibModels / testModels）→ `src/api/mock.ts`。以后 Rust 接管替换的是 `mock.ts` → `rust.ts` 一个文件，页面一行不动 |
| devStore | 抽纯函数（`evalFill` / `evalNudge` / `clamp` + `heroCurves.json` 基线值）→ `src/app/heroCurves.ts`；编辑与上报整套删掉，调用点改成固化常量。不留"只读替身"那层死代码 |

同一个原则也用在共用层：`src/components/` 那 16 个文件里 v023 只用 `Icon`，
其余 14 个 + `useStickyState` 只被老稿使用 —— 没搬（它们还会把 `mock/machine.ts` 一起拖进来）。
`public/` 从 10M 砍到 1.2M：只留代码真正引用的 18 个资产（cat-poses、gacha、rive 预览页、
参考图那些都是试验场的历史）。

---

## 2. 落地的东西

### 第一层：八道本地闸

| 闸 | 变化 |
| --- | --- |
| ② `pre-push` | **语义改**：从"推 main 但 tip 没有对应 tag"改成无条件拦掉对 main 的 push，报错指向"开 PR" |
| ⑦ `pre-commit` | **新增**：`.env*`、私钥、>2MB 的非 `public/` 文件；大小取索引里的 blob，不看磁盘 |
| 结构 | `pre-commit` 三道各自判定、最后统一决定退出码 —— 逃生开关只放掉它自己那一道（原来一道过了就 `exit 0`，`ALLOW_COMMIT_ON_MAIN=1` 会连带放掉闸⑦） |
| 留痕 | 延后到"提交确定发生"之后才写 `bypass.log`。闸①放行、闸⑦拦下时提交没发生，那一行是假记录 —— 第一轮实测抓到三条假记录 |

一次性仓库里跑了 14 条实测，全部符合预期（含"tip 已有匹配 tag 仍然拦 main"这条语义变更的证据）。

### 第二层：GitHub ruleset

`scripts/setup-ruleset.mjs`（幂等，写完回读确认）：main 必须走 PR（approvals 0 + 评论必须解决）、
线性历史、禁 force push、禁删除、必需检查 `web` + `rust`、`bypass_actors` 为空（admin 也绕不过）；
`v*` tag 禁删。

两层都动手验过：

- `git push --no-verify origin <sha>:main`（本地八道闸全绕）→ 服务端 `GH013: Changes must be
  made through a pull request`；
- 故意让 CI 红（一个未使用的局部变量）→ `gh pr merge` 被拒：`the base branch policy prohibits
  the merge` → 撤掉探针 → 绿了才合进去。

### CI

`web`（ubuntu：npm ci → lint → tsc -b → build）+ `rust`（macOS：fmt → clippy -D warnings → test）。
Rust 放 macOS 是因为 Tauri 的 Rust 侧在 Linux 上要先装一串 GTK/WebKit 系统包，而本产品只发 macOS；
public 仓库的 macOS runner 不计费。早期那段"未就位就大声跳过"的逻辑在两边都到位后整个删掉了。

### 前端

`src/app/` 是唯一界面（v023 去掉稿号）。`tsc -b` / `eslint` / `stylelint` / `vite build` 全过，
浏览器 5178 四档 `data-density` 正确切换、零控制台错误、六个页签齐全。

### Rust 侧（20 个单测全绿）

| 模块 | 内容 |
| --- | --- |
| `error.rs` | `AppError { code, message, traceId, detail? }` + 8 个 code；单测逐字段对齐 `contract.ts` |
| `obs/tracing.rs` | uuid v7 的 trace id、按天滚动日志、目录不可用退 stderr 且不阻断启动 |
| `fsx/paths.rs` | 两层数据根（`appDataDir` / `Documents/SupportEase`）+ 三道防穿越判据；`../`、绝对路径、**符号链接指向根外**都被拒 |
| `fsx/atomic.rs` | 临时文件 → `sync_all` → `persist` → fsync 父目录；`clippy.toml` 禁掉 `std::fs::write` 等，唯一的洞在本文件 |
| `ipc/mod.rs` | 五个 command，统一包装生成 trace id → 开 span → 给错误盖章；`save_offsets` 走原子写落到 `index/offsets.json` |

clippy 拦截动手验过：在 `ipc/` 里写一行 `std::fs::write` → `error: use of a disallowed method`，
撤掉后恢复通过。

启动实测：两个数据根连子目录一次建齐（`cloud/ archive/ index/ logs/ run/` 与
`exports/ reports/ presets-mine/`），按天日志文件写出来了。

### 前端接线

`bridge.ts` 换成 `invoke()` + `normalizeError`（command 没注册 / 参数反序列化失败 / panic
三类非结构化 reject 兜底成 `INTERNAL` 或 `NOT_IMPLEMENTED`，`traceId: '-'`，原值进 `detail`）；
`index.ts` 换成运行时探测 `'__TAURI_INTERNALS__' in window` —— 按 `import.meta.env.DEV` 判会让
`tauri dev` 也走 mock，于是整个开发期碰不到真实链路。界面上 `TraceTag` 显示 traceId 前 8 位、点击复制。

顺手修掉的两个"假实现"：预设失败态的「重试」在试验场调的是调参面板开关（只改显示、不重发），
现在 `usePreset` 给出真的 `retry`；`errors.ts` 的 `NotImplementedError` 没有调用点了，删掉。

### 发版与文档

`scripts/release.mjs` 重写为 PR-only：前置检查（不在 main / 干净 / 闸门生效 / origin / gh 已登录 /
复述 bypass.log）→ 六项校验 → 三处版本号 → PR → 轮询 CI → squash 合并 → 回 main 打 annotated tag。
破坏性命令只打印不执行。

`docs/ARCHITECTURE.md`、`docs/GIT-WORKFLOW.md`、`README.md` 写完；`DESIGN-SPACING.md` 与
`3D-ASSET-CONTRACT.md` 原样搬入。

---

## 3. 还没验的四条（需要你的手）

| # | 内容 | 为什么卡住 |
| --- | --- | --- |
| 1 | **四档逐档像素比对**（doc §8.1） | 只做到结构级：四档切换正确、零控制台错误、页签与首页向导都在。试验场把 v023 渲染在预览器的模拟窗口里，像素比对得先把那个预览器驱动到同样的窗口尺寸 |
| 2 | **原生窗口里点一遍**：数据来自 Rust、摘 command → 报错带 traceId → 按 id 查日志（doc §8.3–8.4） | `npm run tauri dev` 起得来、两个根与日志都写了，但点按钮要 GUI 交互 |
| 3 | **`docs/PRESET-PRODUCT-RULES.md` 的正文** | 19 节目录已按 doc §2.6 落下，正文空着。规则原文不在骨架 spec 里，凭印象补写会让后面的实现照着错条文做 —— 把原文贴进来，或告诉我在哪 |
| 4 | **`npm run release` 发 v0.1.0** | 脚本要交互输入版本号与说明，而且发版会再走一轮 PR + 合并 —— 这一步由你亲手跑，顺带就验了 Task 14 |

另外两处已知缺口（写在 `ARCHITECTURE.md` §9）：标题栏那三颗窗口按钮仍是从试验场搬来的装饰件，
还没接 Tauri 的窗口 API；`bypass_actors=[]` 只靠回读确认，没真拿 `gh pr merge --admin` 撞过
—— 那次撞击若失败会把红色构建合进 main，不值得为验证而冒。
