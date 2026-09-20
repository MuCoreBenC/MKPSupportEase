# SupportEase 开局：Tauri 2 壳 + v023 前端整体移植 + 三件地基 + 第一版就生效的 git 双层拦截

> 目标仓库：`/Users/wzy/projects/MKPSupportEase`（当前不存在，本轮第一步才创建 + `git init`）
> 来源仓库：`/Users/wzy/projects/mkp-adaptive-console`（main，工作区干净，tip = `5597f55`，版本 0.1.4）
> **试验场仓库本轮一行不动。** 所有搬运都是单向拷贝，来源仓库只读。
> 产品显示名 **SupportEase**；技术名一律 `mkp-support-ease` / `com.mkpsupport.ease`，两者刻意解耦（§1.1）。

---

## 0. 要做什么（一句话）

起一个 Tauri 2 + React 的产品仓，把试验场的 **v023 成品前端整体移植过来（不重写视觉）**，同时把四件后补代价极高的东西做进去：**唯一写盘出口 `atomic_write`**、**贯穿 IPC 的 trace id**、**结构化 `AppError`**、**main 只能由 PR 推进的双层拦截**。

本轮**不做任何业务功能**。验收标准是"壳跑起来、视觉一致、链路通、错误可追、规则拦得住"，不是"某个页面能用"。

---

## 1. 已定的决策

| 议题 | 结论 | 说明 |
| --- | --- | --- |
| 视觉怎么来 | **整体移植 v023，不重写** | v023 的效果是大量小细节堆出来的（`SlideDeck.tsx@LINE[50..66]` 的 peek 斜坡在 JS 算、平面层常驻合成层、clamp 连续缩放、像素对齐落位），重写必然丢细节且工时翻倍。代价是会一起带进试验期代码 —— 用**剥离清单**处理（§2.3），不靠重写 |
| 数据目录 | **两层** | 程序管理的（云端原件 / 归档 / 索引 / 日志 / 运行状态）→ `appDataDir`；用户自己的（预设副本 / 导出 / 报告）→ Documents 下。理由：macOS 若开了「桌面与文档」iCloud 同步，Documents 下的文件会被驱逐成占位 stub，读出来内容不对，会把 Preset 的 SHA 失效判定搞成误报 |
| 撤销重做栈 | **砍** | 不做通用 command pattern。真需要的地方（参数编辑、轴偏移）后续用「编辑前快照 + 单层撤销」，局部做，不进本轮 |
| git 纪律 | **第一版就双层拦截，main 只能由 PR 推进** | 本地 hook 拦手滑，GitHub ruleset 拦绕过，缺一层都不算拦住。骨架代码本身走第一个 PR 进 main，不是特例（§2.5） |
| 文档分工 | **两份，不合并** | `docs/ARCHITECTURE.md` 管工程约束；`docs/PRESET-PRODUCT-RULES.md` 管产品规则。后面做 Preset 时两份各管一头，不混成超级大文档（§2.6） |
| 命名 | **产品名与技术名解耦** | 见 §1.1 |

### 1.1 命名分层（这一层刻意解耦，改产品名不动代码）

| 层 | 值 | 以后改名要动吗 |
| --- | --- | --- |
| 产品显示名 / 窗口标题 / bundle `productName` | `SupportEase` | 改这一处就够 |
| 本地路径 | `/Users/wzy/projects/MKPSupportEase` | 不动 |
| GitHub 仓库名 | **待定**（已有同名私有仓库，见 §9 ①） | 不动 |
| `package.json` 的 `name` | `mkp-support-ease` | 不动 |
| Cargo crate | `mkp-support-ease`（lib 名 `mkp_support_ease`） | 不动 |
| bundle identifier | `com.mkpsupport.ease` | 不动 |
| 内部数据根 | `appDataDir()` = `~/Library/Application Support/com.mkpsupport.ease` | 不动 |
| 用户可见数据根 | `~/Documents/SupportEase`（见 §9 ②） | 改名要迁目录 |

只有最上和最下两行跟产品名绑定。中间那几层一旦定下就不再动 —— 这正是你说的"以后突然想改叫 MKP Helper，不该动 Rust crate 和 npm package"。

---

## 2. 仓库形态

### 2.1 目标目录结构

```text
MKPSupportEase/
├── .comate/specs/                  # SDD 产物，跟着仓库走
├── .github/workflows/ci.yml        # 新：PR 的必需状态检查
├── docs/
│   ├── ARCHITECTURE.md             # 新：工程约束（本 doc 收口后的常驻版）
│   ├── PRESET-PRODUCT-RULES.md     # 新：Preset 产品规则（§2.6）
│   ├── GIT-WORKFLOW.md             # 重写：PR 流程 + 两层拦截 + 新发版流程
│   ├── DESIGN-SPACING.md           # 搬：间距/动画原则
│   └── 3D-ASSET-CONTRACT.md        # 搬：上游资产契约 v3
├── scripts/
│   ├── hooks/                      # 搬 + 改：闸②语义变更、新增闸⑦
│   ├── setup-hooks.mjs             # 搬
│   └── release.mjs                 # 重写：PR-only 流程（§2.5.5）
├── public/                         # 搬：静态资产
├── src/
│   ├── api/                        # contract.ts / mock.ts / bridge.ts / index.ts / errors.ts
│   ├── app/                        # ← v023 提升为唯一前端（去掉 V023 后缀）
│   │   ├── App.tsx
│   │   ├── pages/{PageHome,PageCalib}.tsx
│   │   ├── components/             # v023/components 原样
│   │   └── {useCalibration,usePreset,calibAxes,flip}.ts
│   ├── components/                 # 搬：16 个跨版本共用组件
│   ├── hooks/                      # 搬：useDensity / usePlatform / useStickyState
│   ├── calib/                      # 搬：*.generated.ts 校准板产物
│   └── styles/                     # 搬：tokens.css / global.css
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── clippy.toml                 # 禁 std::fs 写入 API
│   ├── capabilities/default.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── error.rs                # AppError / ErrorCode
│       ├── obs/tracing.rs          # subscriber + 按天轮转
│       ├── fsx/
│       │   ├── atomic.rs           # 唯一写盘出口
│       │   └── paths.rs            # 两层根 + 防穿越
│       └── ipc/mod.rs              # command 注册 + span 包装
├── index.html
├── package.json
├── vite.config.ts
├── eslint.config.js / .stylelintrc.json / tsconfig*.json
└── .gitignore
```

### 2.2 原样搬过去的清单

| 来源（mkp-adaptive-console 内相对路径） | 目标 | 处理 |
| --- | --- | --- |
| `src/styles/tokens.css`、`global.css` | 同名 | 原样 |
| `src/components/*`（16 文件，8 组 tsx+css） | 同名 | 原样 |
| `src/hooks/*` | 同名 | 原样 |
| `src/calib/*.generated.ts` | 同名 | 原样，数据资产 |
| `src/versions/v023/**` | `src/app/**` | **改名去 V023 后缀**，import 路径跟着改 |
| `public/**` | 同名 | 原样 |
| `index.html`、`eslint.config.js`、`.stylelintrc.json`、`tsconfig*.json` | 同名 | 原样 |
| `docs/DESIGN-SPACING.md`、`docs/3D-ASSET-CONTRACT.md` | 同名 | 原样 |
| `scripts/hooks/**`、`scripts/setup-hooks.mjs` | 同名 | hooks 有改动，见 §2.5.2 |
| `src/api/{contract,mock,bridge,index,errors}.ts` | 同名 | contract 扩错误类型，bridge 重写内部，index 改判断（§4） |

### 2.3 剥离清单（搬过去之后要删掉/改掉的试验设施）

| 对象 | 为什么剥 | 怎么处理 |
| --- | --- | --- |
| `src/versions/registry.ts` + 多稿切换 + `localStorage` 的 `mkp.version` | 产品只有一个界面，没有"稿号" | 删。`registry.ts` 里那个 `id: 'v0.0.22'` 的历史包袱一并消失 |
| `src/App.tsx`（预览器外壳：Ultra/Wide/Compact 窗口尺寸预设 + 版本下拉） | 真窗口尺寸由 Tauri 管，不需要模拟 | 删。`src/app/App.tsx` 直接成为根组件 |
| `src/dev/DevPanel.tsx`、`CurveEditor.tsx`、`devStore.ts` | 调参面板是试验场工具 | **先核实 `ExplodedHero` / `SlideDeck` 是否读 `devStore`**；若读，把当前曲线值从 `src/dev/heroCurves.json` 固化成模块常量，再删面板。`heroCurves.json` 作为数据保留 |
| `src/versions/v001..v022` | 老稿 | 不搬 |
| `src/mock/{mkp,mkpFull,machine,testModels}.ts` | 产品数据从 Rust 取 | 不搬。浏览器调试路径需要的最小假数据内联进 `src/api/mock.ts` |
| `tools/stl-svg/**`、`tools/dev-server/**` + `vite.config.ts` 里的 `calibFs()` / `curvesFs()` | dev-only 无鉴权写盘端点，是工作台，不是产品 | 留在试验场。产品仓 `vite.config.ts` 只留 `react()` |
| `pic/`（16M）、`tmp-shots/`、根目录 7 张 PNG | 参考图 | 不搬 |
| 根目录 30 个 `feedback_*.md` / 9 个 `project_*.md` / `reference_*.md` / `MEMORY.md` | 记忆文件，属于试验场的历史 | 不搬。产品仓的约定写进 `docs/ARCHITECTURE.md` |
| `scripts/probe*.mjs`、`stl-to-svg.mjs` | 工作台配套 | 不搬 |
| 物理沙盘（cannon-es / `interactive-physics-sandbox`） | 独立试验页面 | 不搬。**需核实它是否有独立入口文件、是否牵连 `package.json` 依赖** |

> 剥离过程中有两处"需核实"，执行阶段先读代码确认依赖边界再动手，不凭印象删。

#### 2.3 补记：实测依赖闭包（Task 5 执行前测出，推翻了上表两行）

上表写着"`v001..v022` 不搬"与"`src/mock/**` 不搬"。实测 v023 的 import 图证明这两条做不到：

- v023 自身 30 文件 7246 行；**外部闭包 35 文件 5920 行**。
- 跨稿依赖：`v003/TopTabs`、`v005/ui/{Controls,Modal}`、`v005/pages/{PagePreset,PageParams,PageSettings}`、`v005/report/ReportView`、`v008/components/CopyAction`、`v014/{plateLadder,heroArt,useArtLayers}`、`v014/components/{CalibPlate,MachinePicker}`。
- `src/mock/mkpFull.ts`（549 行）被 6 处 import：`AppV023`（`report` / `tabs`）、`CalibHead`（`presetIndex`）、`MachinePicker`（`brands` / `models` / `variants` / `Option`）、三个 v005 页面、`ReportView`、`api/mock.ts`（`calibModels` / `presetIndex`）。
- `src/dev/devStore.ts`（914 行）被 6 个 v023 文件 import。上表只要求核实 `ExplodedHero` / `SlideDeck` —— 实测 **`ExplodedHero` 根本不碰 devStore**（只用 three + GLTFLoader），真正深度耦合的是 `HeroFade`（14 个符号，含 `undo` / `redo` / `commitTune` / `bumpCurveAt` 这套曲线编辑）。

据此定下三条（Task 5 / 6 已按此重写）：

| 议题 | 结论 |
| --- | --- |
| 跨稿 UI | **只搬首页 + 校准页真正需要的**：`TopTabs`、`ui/{Controls,Modal}`、`CopyAction`、`v014` 四个模块，落位到 `src/app/` 内按职责命名的目录，不保留稿号。`v005` 的三个页面与 `ReportView` **不搬** —— Preset / Params / Settings / 报告四处先用统一占位组件，页签结构不变 |
| mock 数据 | **拆两层**。界面结构数据（`tabs` / `brands` / `models` / `variants`）属于应用本身 → `src/app/constants/`；业务假数据（`presetIndex` / `report` / `calibModels` / `TEST_MODELS`）属于 API 的临时实现 → `src/api/mock.ts`。理由：以后 Rust 接管时替换的是 `mock.ts` → `rust.ts` 这一个文件，页面一行不动。mock 是临时实现，不该变成新的"数据层祖宗" |
| devStore | **抽纯函数**：`evalFill` / `evalNudge` / `clamp` / `snapPct` / `NUDGE_LIMIT` + `heroCurves.json` 的基线值 → `src/app/heroCurves.ts`。编辑与上报（`undo` / `redo` / `commitTune` / `bumpCurveAt` / `setNudgeAt` / `resetNear` / `report*` / `setPresetPhase` / `useDevState`）全删，调用点改成固化常量。不留"只读替身"那层死代码 |

同一个原则也用在共用层：`src/components/` 那 16 个文件里 **v023 只用 `Icon`**，其余 14 个（AxisPanel / Card / GlueSpeed / PrinterStage / QuickActions / RecentFiles / StatusBar + CSS）与 `hooks/useStickyState` 只被老稿使用，搬过来是死代码，而且会把 `src/mock/machine.ts` 一起拖进产品仓。只搬 `Icon` + `useDensity` + `usePlatform` + `calib/*.generated.ts`。

### 2.4 依赖

前端保持极简，沿用试验场的 exact 版本钉法（`package.json` 里全是精确版本，不用 `^`）：

- 保留：`react` 18.3.1、`react-dom` 18.3.1、`three` 0.186.0、`@types/three` 0.186.0 + 现有工具链
- 新增：`@tauri-apps/api`、`@tauri-apps/cli`（devDep）、按需 `@tauri-apps/plugin-fs`、`plugin-dialog`、`plugin-opener`
- 移除：`playwright`（试验场用来截图比档，产品仓本轮不需要）

Rust 侧首批：`tauri` 2.x、`serde`、`serde_json`、`thiserror`、`tempfile`、`tracing`、`tracing-subscriber`、`tracing-appender`、`uuid`（v7）。

> **版本以 `cargo add` / `npm i` 实际解析到的为准，装完把确切版本回填这份 doc，并锁 `Cargo.lock` + `package-lock.json`。** 不在这里写死小版本号 —— 写死了大概率和实际装上的不一致，反而误导。

已核实的本机工具链：node v26.4.0、npm 11.17.0、rustc 1.97.1、cargo 1.97.1、Xcode CLT 就位、gh 2.93.0（已登录 `MuCoreBenC`，token 含 `repo` + `workflow` scope，够建仓和建 ruleset）。

---

### 2.5 git 纪律：两层拦截

**原则：本地 hook 拦"手滑"，GitHub ruleset 拦"绕过"。只有本地那层等于没拦住** —— `--no-verify` 能绕过全部本地钩子且连日志都不留（git 的设计如此，纯本地方案补不上），换台机器 clone 没跑过 `npm install` 也等于没闸。所以服务端必须有一层。

**主干规则一句话：main 只能由 PR 推进，任何人（包括你自己）都不直接 push main。**

#### 2.5.1 开局悖论怎么破

第一个 commit 必须落在 main 上 —— 那时 main 还不存在，闸①无从谈起。所以顺序必须精确，否则"第一版就严格执行"会是句空话：

```text
1. mkdir + git init -b main
2. 先把 scripts/hooks/、setup-hooks.mjs、package.json 落盘（还不提交）
3. npm install → prepare 触发 setup-hooks → core.hooksPath 指向 scripts/hooks
   ← 闸门从这一刻起生效
4. ALLOW_COMMIT_ON_MAIN=1 git commit --allow-empty -m 'chore: 开仓'
   ← 全仓唯一一次合法的 main 直提，会留痕到 .git/bypass.log
5. gh repo create + git push -u origin main
6. 立刻建 main 的 ruleset（main 必须已存在，ruleset 才挂得上）
7. 之后所有内容 —— 包括本轮骨架的全部代码 —— 走 feat/bootstrap-shell 分支 + PR 进 main
```

第 4 步那条空提交是唯一的例外，而且它留痕、下次发版会被复述。**骨架代码本身就是第一个 PR**，这样"第一版就严格执行"是字面意义上的真。

#### 2.5.2 第一层：本地 hook

七道闸从试验场原样搬，两处改、一处新增：

| 闸 | 钩子 | 拦什么 | 本轮变化 |
| --- | --- | --- | --- |
| ① | `pre-commit` | 在 main 上直接提交 | 不变，主力闸 |
| ①b | `pre-merge-commit` | 手工合并进 main | 不变。git 2.53 实测 `merge --no-ff` **不调用** `pre-commit`，只有这个钩子拦得住 |
| ② | `pre-push` | 推 main | **语义改**：原判据是"推 main 但 tip 没有对应版本的 annotated tag"（结果校验）。PR-only 之后本地永远不该 push main，改成**无条件拦掉对 main 的 push**，不再看 tag。main 的推进权已经交给 GitHub |
| ③ | `pre-push` | 非快进（force）推送 | 不变 |
| ④ | `pre-push` | 删除远端引用 | 不变 |
| ⑤ | `pre-push` | tag 名与 `package.json` 不符 / 是轻量 tag / 不在 main 上 | 不变。tag 仍由本地打并推 |
| ⑥ | `pre-commit` | 分支名没有 `feat/ fix/ refactor/ chore/ docs/ style/` 前缀 | 不变 |
| ⑦ | `pre-commit` | **新增**：提交里含 `.env*`、私钥（`*.pem` / `*.p12` / `id_rsa*`）、或 >2MB 的非 `public/` 文件 | 产品仓以后要碰用户数据和签名密钥，试验场没这个风险所以没这道闸 |

逃生开关沿用 `ALLOW_*=1` 那一套，开了往 `.git/bypass.log` 记一行（时间 / 开关名 / 分支 / HEAD），发版时复述。

#### 2.5.3 第二层：GitHub ruleset（服务端）

用 `gh api` 建，脚本化进 `scripts/setup-ruleset.mjs`，规则本身也进版本控制（不靠"我记得在网页上勾过"）。

> **仓库可见性：public（Task 3 执行时改的，原计划是 private）。**
> ruleset API 对 private 仓库要求 GitHub Pro —— 实测 `GET /repos/:o/:r/rulesets` 直接 403：
> `Upgrade to GitHub Pro or make this repository public to enable this feature.`
> 当前账号（`MuCoreBenC`）是 Free。也就是说 private + Free 下第二层根本不存在，
> §2.5 的"两层拦截"会退化成一层，而"只有本地那层等于没拦住"正是这一节的前提。
> 三个选项（升 Pro / 改 public / 放弃第二层）里选了改 public。
> 改的时候仓库里只有那个空提交，没有任何内容被动过，代价最小的时机就是这一刻。
> **由此产生一条常驻约束：任何密钥、用户数据、私有资产都不能进这个仓库**
> —— 闸⑦（`.env*` / 私钥 / 大文件）从"防手滑"升级成"唯一的防线"。

main 分支 ruleset：

| 规则 | 值 | 为什么 |
| --- | --- | --- |
| Require a pull request before merging | 开，approvals = 0 | 单人项目要 approval 会把自己锁死；要的是"必须走 PR"这个动作，不是审批 |
| Require status checks to pass | 开，必需项 = `web`、`rust` 两个 job | 没有 CI 的话这条勾了也无从检查，所以 §2.5.4 必须一起做 |
| Require linear history | 开 | 配 squash merge，历史是一条线，`git log --oneline` 可读 |
| Require conversation resolution | 开 | 自己给自己留的 review 意见不会被顺手无视 |
| Block force pushes | 开 | |
| Restrict deletions | 开 | |
| **Do not allow bypassing** | **开** | 单人项目勾这个等于把自己也锁在外面。但"能绕的规则等于没有"；真要改得打开浏览器改 ruleset，这个成本刚好合适 —— 足够挡住手滑，又没有堵死出口 |

tag ruleset（`v*`）：先只做 **block deletion**。创建限制不加 —— tag 由本地 `release` 流程打，加了会互相打架。

#### 2.5.4 CI（`.github/workflows/ci.yml`）

`on: pull_request` + `push: [main]`，两个 job：

- `web`：`npm ci` → `npm run lint` → `tsc -b` → `npm run build`
- `rust`：`cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`

必须缓存 `~/.cargo/registry` 与 `src-tauri/target`，否则每个 PR 冷编译几分钟，你会很快开始想绕过它 —— 规则被绕过往往不是因为不认同，是因为太慢。

#### 2.5.5 发版流程随之重写

试验场的 `release.mjs` 是"本地合并进 main + 推 main"，与 PR-only **直接冲突**，必须改，不能原样搬。新流程：

```text
在 feat/ 分支上：
1. lint + tsc + build + cargo clippy + cargo test        ← 失败就停，此时还没有任何写操作
2. 问版本号与一句话说明；改 package.json + Cargo.toml + tauri.conf.json 三处版本号，提交
3. git push -u origin HEAD → gh pr create
4. 等 CI 绿 → gh pr merge --squash --delete-branch
5. git switch main && git pull
6. 在 main 的 tip 打 annotated tag v<ver> → git push origin v<ver>
```

两处和旧流程的关键差别：

- **tag 在 PR 合并之后才打**，指向合并后的 main tip。旧流程是"先打 tag 再推 main"，tag 指向的是本地合并提交 —— 现在这个更准。
- **squash 会重写提交**，所以分支上的 SHA 和 tag 指向的 SHA 不同。这是预期的，不是 bug；`docs/GIT-WORKFLOW.md` 里要写明，不然以后自己会困惑。

任何一步失败就停住并打印精确的回退命令，破坏性命令只打印、不执行 —— 这条纪律从试验场继承。

### 2.6 `docs/PRESET-PRODUCT-RULES.md`

把你整理的那套 Preset 产品规则**原样落成文档**，本轮只写文档、不写实现。目录按你的原文结构：页面结构（位置 × 类型两个维度）→ 本地页面字段与排序 → MKP 配置的"应用"唯一性 → 切片器配置没有"应用" → 云端页面与三个状态 → 更新提示 → 下载 → 更新与归档 → 发布时间作为用户可见版本 → SHA 与失效 → 云端原件只读 → Finder 复制脱钩 → 用户修改副本 → 与云端更新完全脱钩 → 归档 → 外部导入 TOML 校验 → 本地文件被外部删除 → 第一版明确不做 → 最终状态模型图。

分工写在两份文档的开头互相指认：

```text
ARCHITECTURE.md          → Tauri / 两层路径 / IPC / trace / atomic_write / 权限 scope
PRESET-PRODUCT-RULES.md  → 本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 / 状态流转
```

后面做 Preset 时，工程约束查前者，产品行为查后者，不混。

---

## 3. 三件地基

### 3.1 唯一写盘出口 `atomic_write`

```rust
// src-tauri/src/fsx/atomic.rs
use std::fs::File;
use std::path::Path;
use tempfile::NamedTempFile;

/// 全仓唯一的写盘出口。
///
/// 三步都不能省：
/// 1. 临时文件建在**目标的父目录**里 —— rename 只在同一文件系统内原子，
///    建在 /tmp 再 rename 跨分区会退化成 copy+delete，中途断电就是半个文件。
/// 2. persist 之前 sync_all —— 不然内容还在页缓存里，rename 完了目录项指向一个空文件。
/// 3. 最后 fsync 父目录 —— rename 本身也要落盘，否则崩溃后可能既没新文件也没旧文件。
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::invalid_argument("目标路径没有父目录"))?;
    std::fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut tmp, bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;

    File::open(parent)?.sync_all()?;
    Ok(())
}
```

**纪律怎么落地**：口头约定没用，用 clippy 强制。

```toml
# src-tauri/clippy.toml
disallowed-methods = [
  { path = "std::fs::write", reason = "走 fsx::atomic_write" },
  { path = "std::fs::File::create", reason = "走 fsx::atomic_write" },
  { path = "tokio::fs::write", reason = "走 fsx::atomic_write" },
]
```

`atomic.rs` 自身用 `#[allow(clippy::disallowed_methods)]` 开一个洞，其他文件一律拦。CI 的 `clippy -D warnings` 让这条规则对 PR 生效，不只是本地提示。

### 3.2 路径分层 `paths.rs`

两个根，职责完全分开：

```text
internal_root()  = appDataDir()          # ~/Library/Application Support/com.mkpsupport.ease
├── cloud/          云端原件（只读、带 SHA 记录）
├── archive/        被更新替换掉的旧云端原件
├── index/          内部索引（预设关系、SHA、发布 ID）
├── logs/           tracing 按天轮转
└── run/            运行时临时状态

user_root()      = documentDir()/SupportEase
├── exports/        导出的 gcode
├── reports/        报告
└── presets-mine/   用户自己的预设副本（改过的、外部拖进来的）
```

这个划分正好对上 Preset 规则里"程序管理的东西"与"用户自己的东西"那条线：`cloud/` `archive/` `index/` 归程序，`presets-mine/` 归用户。云端原件的只读徽章、SHA 失效判定、归档，全发生在 `internal_root` 内，不受 iCloud 驱逐影响。

参考旧世代 `~/Documents/MKPSupportSSR` 的命名（现有 `baselines/ content/ gcode_history/ logs/ models/ presets/ run/ settings.json`），按两层重新归属：`logs`、`run`、`baselines` 归内部根，`gcode_history` 归用户根，`presets` 一分为二。

**防穿越规则**（照搬 `calibFs.mjs@LINE[56..57]` 的 `inside()` 思路）：任何 command **不接收绝对路径**，只接收 `(Root, 相对路径)` 或稳定 ID；拼接后 `canonicalize` 再断言仍在根内，否则报 `PermissionDenied`。

`capabilities/default.json` 里 fs 权限按命令逐个授权 + scope 限到这两个根，**不使用 `fs:default`**。

### 3.3 trace id + tracing

```rust
#[tauri::command]
pub async fn get_preset(variant_id: String) -> Result<Option<Preset>, AppError> {
    let trace_id = new_trace_id();          // uuid v7，时间有序，便于按时间翻日志
    let span = tracing::info_span!("get_preset", trace_id = %trace_id, variant_id = %variant_id);
    let _g = span.enter();
    service::get_preset(&variant_id).map_err(|e| e.with_trace(&trace_id))
}
```

**一处取舍：成功路径不包 Envelope。** 两个选项 —— A 只有错误带 `traceId`；B 成功也包一层 `{ data, traceId }`。

选 **A**。B 会污染所有前端调用点的解构（每个 `await api.x()` 都要多剥一层），而成功路径本来就不需要用户报 id —— 日志里有 span，按时间和参数照样能查到。

subscriber：`tracing-appender` 的 `rolling::daily` 落 `internal_root/logs/`，dev 下同时输出 stderr。**日志目录写不进去时只退到 stderr，不阻断启动** —— 日志失败不该让软件打不开。

### 3.4 `AppError` 契约

```rust
// src-tauri/src/error.rs
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    /// 直接给用户看的一句中文，不含技术细节
    pub message: String,
    pub trace_id: String,
    /// 技术细节，折叠在"详情"里，可复制
    pub detail: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NotFound, PermissionDenied, InvalidArgument,
    Corrupted, ShaMismatch, Io, NotImplemented, Internal,
}
```

`ShaMismatch` 和 `Corrupted` 现在就进枚举，是给 Preset 那一轮预留的：SHA 不符 → `ShaMismatch`（对应"失效"红色状态），TOML 校验不过 → `Corrupted`。本轮不产生这两种错误，但契约先定好，免得下一轮又改一遍前端的错误分支。

前端 `src/api/contract.ts` 增加对应类型 + 判定函数，让**错误也进契约**：

```ts
export type ErrorCode =
  | 'NOT_FOUND' | 'PERMISSION_DENIED' | 'INVALID_ARGUMENT'
  | 'CORRUPTED' | 'SHA_MISMATCH' | 'IO' | 'NOT_IMPLEMENTED' | 'INTERNAL'

export interface AppError {
  code: ErrorCode
  /** 可直接展示给用户的中文 */
  message: string
  /** 报错时显示在角落，用户截图就能定位 */
  traceId: string
  detail?: string
}

export function isAppError(e: unknown): e is AppError
```

`bridge.ts` 里统一 `normalizeError`：`invoke` 的 reject 值不一定是我们的结构（panic、序列化失败、command 不存在时 Tauri 会抛字符串），兜底成 `{ code: 'INTERNAL', message: '内部错误', traceId: '-', detail: String(e) }`。

---

## 4. 前端接线改动（3 个文件）

| 文件 | 现状（已核对源码） | 改动 |
| --- | --- | --- |
| `src/api/index.ts:15` | `export const api: MkpApi = import.meta.env.DEV ? mockApi : bridgeApi` | 换成运行时探测。Tauri dev 下 `DEV` 也是 `true`，照现在的写法壳永远接不上 |
| `src/api/bridge.ts:28..34` | 读 `window.__mkp_api?.xxx?.() ?? missing(...)` | 内部换成 `invoke()` + `normalizeError`，`contract.ts` 的方法签名一个字节不动 |
| `vite.config.ts:26` | `server: { host: true, port: 5178, strictPort: true, open: false }` + `calibFs()` + `curvesFs()` | 去掉两个 dev 插件；`host` 改成 `process.env.TAURI_DEV_HOST ?? false`；加 `clearScreen: false` |

```ts
/**
 * 用运行时探测而不是 import.meta.env.DEV：
 * Tauri dev 下 DEV 也是 true，按构建期判断会让壳永远接不上。
 * 探测之后同一个 dev server 两用 —— 浏览器里走 mock 调视觉，Tauri 窗口里走真壳。
 */
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
export const api: MkpApi = inTauri ? bridgeApi : mockApi
```

`vite.config.ts` 的 `host`：原来无条件 `host: true` 是为了局域网预览，代价是把两个无鉴权写盘端点暴露给同网段（原注释 `@LINE[23..24]` 自己写了这条风险）。产品仓里那两个端点已经不存在，也没必要默认绑全网卡 —— 改成只在 Tauri 真机调试需要时（`TAURI_DEV_HOST` 有值）才绑。`5178` + `strictPort` 沿用，`tauri.conf.json` 的 `devUrl` 必须写同一个端口，不然 WebView 加载到 404。

---

## 5. 数据流

```text
前端页面
   │  await api.getPreset(variantId)
   ▼
src/api/index.ts   ── 运行时探测 ──┬─→ mock.ts（浏览器）
   │                              └─→ bridge.ts（Tauri 窗口）
   ▼
invoke('get_preset', { variantId })
   ▼
src-tauri/src/ipc/mod.rs
   │  new_trace_id() → info_span!(trace_id)
   ▼
service 层
   │  fsx::paths::internal_root() / user_root()   ← 只认 (Root, 相对路径)
   │  canonicalize + inside() 断言
   ▼
fsx::atomic_write(path, bytes)          ← 全仓唯一写盘出口
   │  NamedTempFile(同父目录) → write_all → sync_all → persist → fsync(parent)
   ▼
磁盘

错误回路：
io::Error ──From──→ AppError{code:IO} ──with_trace(id)──→ serde ──→ invoke reject
                                                                      │
                                                     bridge.normalizeError
                                                                      ▼
                                              界面提示 + 角落显示 traceId（可复制）
                                                                      │
                                              internal_root/logs/ 按 id 可查
```

---

## 6. 边界与异常

| 情形 | 行为 |
| --- | --- |
| 目标目录不存在 | `atomic_write` 先 `create_dir_all`，失败报 `IO` |
| 临时文件与目标跨分区 | 不会发生：`NamedTempFile::new_in(parent)` 保证同目录 |
| `persist` 时目标被占用（Windows 常见） | 报 `IO`，带上原路径，**不自动重试** —— 重试会掩盖"有别的进程占着"这个真问题 |
| Documents 被 iCloud 驱逐成占位 stub | 只影响 `user_root` 下用户可见文件；云端原件、归档、索引都在 `internal_root`，不受影响。这正是分两层的目的 |
| Tauri 窗口里但 Rust 侧还没实现某 command | `invoke` reject → normalize 成 `NOT_IMPLEMENTED`，界面显示可读提示 + traceId，不白屏 |
| 浏览器里打开 5178 | 探测不到 Tauri，走 mock，不报错。纯前端调视觉的路径完整保留 |
| 日志目录写不进去 | tracing 退到 stderr，**不阻断启动** |
| 两个数据根首次不存在 | 启动时 `create_dir_all` 建齐；建不出来（权限）报错并显示具体路径 |
| command 收到绝对路径 | 一律拒绝，报 `INVALID_ARGUMENT` |
| 本地闸门没装（clone 后没跑 `npm install`） | 本地全裸，但服务端 ruleset 照样拦住 push main / force / 无 PR 合并。这就是要两层的原因 |
| CI 挂了但想合 PR | 合不了（必需状态检查）。要合只能去网页改 ruleset —— 显式动作，留痕在 GitHub 的 audit log |

---

## 7. 明确不做

- 任何业务功能页面的新行为（Preset 第一版是**下一轮**，本轮只铺地基 + 落产品规则文档）
- 撤销重做栈（已砍）
- 自动更新、代码签名、公证
- Windows / Linux 打包验证（本轮只保证 macOS `tauri dev` + `tauri build` 跑通）
- 搬 `tools/stl-svg` 工作台
- 动试验场仓库（一行不动，纯只读来源）
- TOML 解析（跟着 Preset 那一轮走，届时用 Rust 的 `toml` crate，前端不引 TOML 库）
- 清理你其他的历史仓库（那是独立一件事，本轮只解决"新仓库叫什么、建在哪"）

---

## 8. 预期结果

1. `cd /Users/wzy/projects/MKPSupportEase && npm run tauri dev` 起一个原生窗口，标题 `SupportEase`，显示 v023 的首页与校准页，**视觉与试验场逐档一致**（mini / compact / wide / ultra 四档比对）
2. `npm run dev` + 浏览器开 `http://localhost:5178/` 仍可用，走 mock，纯前端调视觉的路径不丢
3. Tauri 窗口里 `getPreset` / `getCalibModels` 的数据来自 Rust（先返回硬编码值，只为证明链路通）
4. 故意把某个 command 从 `generate_handler!` 里摘掉 → 界面出现可读错误 + traceId，`internal_root/logs/` 当天日志里按该 id 能查到那一条
5. `internal_root/logs/` 下有按天命名的日志文件；两个数据根首次启动自动建齐
6. clippy 拦截生效：在 `ipc/` 里写一行 `std::fs::write`，`cargo clippy -- -D warnings` 拒掉
7. `tsc -b` / `eslint` / `stylelint` / `vite build` / `cargo build` / `cargo clippy` / `cargo test` 全过
8. **git 拦截实测**（这条必须动手验，不是看配置）：
   - 在 main 上 `git commit` → 被闸①拦
   - `git push origin main` → 被闸②拦；就算 `--no-verify` 绕过本地，也被 GitHub ruleset 拒
   - `git switch -c wip/x && git commit` → 被闸⑥拦（前缀不合法）
   - 造一个 3MB 的假文件放 `src/` 提交 → 被闸⑦拦
   - 开一个 PR 让 CI 故意失败 → `gh pr merge` 被拒
9. `.git/bypass.log` 里只有开仓那一条绕过记录，没有第二条
10. `docs/ARCHITECTURE.md`、`docs/PRESET-PRODUCT-RULES.md`、`docs/GIT-WORKFLOW.md` 三份就位，开头互相指认分工

---

## 9. 需要你点头的三处

**① GitHub 仓库名，以及那个已存在的同名私有仓库怎么办。** —— **已定（Task 3 执行时）**

- 远端是 **`MuCoreBenC/MKPSupportEase`**（public），与本地目录名一致。
- 旧的 `MuCoreBenC/SupportEase` 已重命名为 **`SupportEase-archive`**（仍 private，未删、未设 archived，随时可改回）。
- 中间过渡名 `mkp-support-ease` 已 rename 掉，GitHub 会为旧地址做跳转；本地只改了一条 `git remote set-url`。
- ruleset 挂在仓库 id 上，rename 后原样有效（已回读确认）。
- npm 包名仍是 `mkp-support-ease`（npm 名必须小写），crate 名同上 —— 与仓库名、产品名解耦，见 §1.1。
- **许可证：AGPL-3.0-only**（与 Bambu Studio / PrusaSlicer 一致的传染性协议）。`LICENSE` 用 GNU 官方全文，`package.json` 的 `license` 字段同步。选它的直接后果：任何人基于本仓库改出来的东西再分发，必须同样开源。

原先的三条路与倾向留在下面备查：

你说已经有一个 `SupportEase` 的私有仓库，打算清理后再用这个名字。三条路：

- 先用一个临时名（比如 `mkp-support-ease`）建仓，清理完再 `gh repo rename` —— GitHub 会自动做旧地址跳转，本地只需改一次 remote，代价很小
- 先把旧仓库改名腾位（`gh repo rename` 旧的，比如加 `-archive` 后缀），新仓库直接叫 `SupportEase`
- 本轮先只建本地仓库、不建远端，等你清理完再 `gh repo create` + push

我倾向第一条：**先叫 `mkp-support-ease`，以后再 rename**。理由是 ruleset 和 CI 要挂在真实远端上才能验，卡在"等清理"会让本轮第 8 条预期结果验不了；而 rename 的代价确实只有一条 `git remote set-url`。

**② 用户可见目录叫什么。**
`~/Documents/SupportEase`（跟窗口标题一致，用户一眼认得）还是 `~/Documents/MKPSupportEase`（挂靠 `MKPSupport*` 家族，Finder 里和旧世代排在一起）。
我倾向 **`SupportEase`** —— 这个目录是给用户看的，就该叫产品名；`MKPSupport*` 家族是旧世代的历史，新一代不必挂靠。但如果你希望以后几代数据在 Finder 里排成一列好找，就选后者。

**③ PR 合并方式：squash 还是 rebase。**
squash（一个 PR = 一个提交，main 的 `git log` 极干净，但分支上的细碎提交历史丢掉）还是 rebase（保留每个提交，历史仍是线性，但 main 上会多出很多小提交）。
我倾向 **squash**，因为你的 SDD 流程里一个 spec 就是一个完整改动单元，PR 标题就是那个单元的名字，细碎的中间提交对以后查 blame 帮助不大。
