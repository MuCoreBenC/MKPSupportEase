# SupportEase 开局任务清单：闸门先立 → 前端移植 → Rust 地基 → 文档与发版

> 分支纪律：Task 1–3 在 main 上完成（开仓阶段，唯一合法的例外），Task 4 起全部在 `feat/bootstrap-shell` 分支上，最后由 PR 合并进 main。
> 每个 Task 结束时工作区必须可编译、可运行 —— 不留"下一个任务才能跑起来"的中间状态。

---

## 第一段：开仓与闸门（先立规矩，再写代码）

- [x] Task 1: 建仓并让闸门在第一个提交之前就生效
    - 1.1: `mkdir /Users/wzy/projects/MKPSupportEase` + `git init -b main`
    - 1.2: 从试验场拷 `scripts/hooks/`（`_lib.sh` / `pre-commit` / `pre-merge-commit` / `pre-push`）与 `scripts/setup-hooks.mjs`
    - 1.3: 写最小 `package.json`：`name: mkp-support-ease`、`version: 0.0.0`、`type: module`、`prepare: node scripts/setup-hooks.mjs`
    - 1.4: 写 `.gitignore`（`node_modules`、`dist`、`src-tauri/target`、`.DS_Store`、`*.local`）
    - 1.5: `npm install` 触发 prepare，确认输出里 `core.hooksPath = scripts/hooks`
    - 1.6: `ALLOW_COMMIT_ON_MAIN=1 git commit --allow-empty -m 'chore: 开仓'`，确认 `.git/bypass.log` 留下这一条

- [x] Task 2: 按 PR-only 改闸门判据
    - 2.1: `pre-push` 闸②语义改写：从"推 main 但 tip 没有对应 annotated tag"改成**无条件拦掉对 main 的 push**，报错文案指向"开 PR"
    - 2.2: `pre-commit` 新增闸⑦：拦 `.env*`、`*.pem` / `*.p12` / `id_rsa*`、以及 >2MB 的非 `public/` 文件，逃生开关 `ALLOW_BIG_OR_SECRET=1`
    - 2.3: 更新 `setup-hooks.mjs` 启动时打印的闸门清单（现在是七道，加完是八道）
    - 2.4: 本地逐条实测：main 直提被拦、`git push origin main` 被拦、`wip/x` 分支名被拦、3MB 假文件被拦、逃生开关各自留痕

- [x] Task 3: 建远端与服务端 ruleset（暂不含状态检查）
    - 3.1: `gh repo create mkp-support-ease --private --source . --remote origin`（临时名，清理完旧仓库后再 rename）
    - 3.2: `git push -u origin main`（此时闸②会拦 —— 用 `ALLOW_PUSH_MAIN=1` 放行这一次，因为 ruleset 还没建起来，属于开仓阶段）
    - 3.3: 写 `scripts/setup-ruleset.mjs`：用 `gh api` 建 main ruleset —— require PR（approvals 0）、require conversation resolution、require linear history、block force push、restrict deletions、do not allow bypassing
    - 3.4: 同一脚本建 tag ruleset（`refs/tags/v*`）：只做 restrict deletions
    - 3.5: 跑一次脚本并用 `gh api` 回读确认规则已挂上
    - 3.6: **本轮不勾 required status checks** —— CI 还不存在，勾了第一个 PR 永远合不进去。Task 16 再补
    - 3.7: `git switch -c feat/bootstrap-shell`，之后所有任务都在这条分支上

---

## 第二段：前端移植（v023 整体搬过来，不重写视觉）

- [ ] Task 4: CI 工作流
    - 4.1: 写 `.github/workflows/ci.yml`，`on: pull_request` + `push: [main]`
    - 4.2: `web` job：`npm ci` → `npm run lint` → `tsc -b` → `npm run build`
    - 4.3: `rust` job：`cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`（Rust 侧还不存在时先允许跳过，Task 7 之后必须真跑）
    - 4.4: 缓存 `~/.cargo/registry` 与 `src-tauri/target`
    - 4.5: push 分支，确认 Actions 里两个 job 出现并跑完

- [ ] Task 5: 搬共用前端层
    - 5.1: 拷 `src/styles/`、`src/components/`（16 文件）、`src/hooks/`、`src/calib/`、`public/`、`index.html`
    - 5.2: 拷 `eslint.config.js`、`.stylelintrc.json`、`tsconfig.json` / `tsconfig.app.json` / `tsconfig.node.json`（`tsconfig.tools.json` 不搬，工作台不在这个仓库）
    - 5.3: `package.json` 补依赖（exact 版本）：react 18.3.1、react-dom 18.3.1、three 0.186.0、@types/three 0.186.0 + 现有工具链；**不装 playwright**
    - 5.4: 补 scripts：`dev` / `build` / `preview` / `lint` / `lint:fix`
    - 5.5: 写精简版 `vite.config.ts`：只留 `react()`，`port: 5178` + `strictPort: true` + `open: false`，`host: process.env.TAURI_DEV_HOST ?? false`，`clearScreen: false`
    - 5.6: `tsc -b` + `npm run lint` 过

- [ ] Task 6: 把 v023 提升为唯一前端 `src/app/`
    - 6.1: **先核实依赖边界**：读 `ExplodedHero.tsx` / `SlideDeck.tsx` / `HomeGuide.tsx`，确认是否 import `src/dev/devStore`，把结果记下来
    - 6.2: 拷 `src/versions/v023/**` → `src/app/**`，文件名去 `V023` 后缀（`AppV023.tsx` → `App.tsx`、`PageMachineV023.tsx` → `pages/PageHome.tsx`、`PageCalibV023.tsx` → `pages/PageCalib.tsx`，CSS Module 同步改名）
    - 6.3: 修所有 import 路径（原来指向 `../v005/...`、`../../components/...` 的一律重指）
    - 6.4: 按 6.1 的结论处理 devStore：把当前生效的曲线值从 `src/dev/heroCurves.json` 固化成 `src/app/heroCurves.ts` 模块常量，删掉对 DevPanel / devStore 的引用
    - 6.5: 搬 `src/api/{contract,mock,bridge,index,errors}.ts`，`mock.ts` 里内联最小假数据（替代不搬的 `src/mock/**`），保证浏览器模式能跑
    - 6.6: 写 `src/main.tsx` 直接挂 `src/app/App.tsx`；**不搬** `src/versions/registry.ts` 与旧 `src/App.tsx` 预览器外壳
    - 6.7: 处理 `TopTabs` 的页签来源：原来读 `src/mock/mkpFull.ts` 的 `tabs`，改成 `src/app/` 内的模块常量，并沿用 v023 的覆盖（`machine` → 「首页」+ home 图标、`settings` → gear 图标）
    - 6.8: `npm run dev` + 浏览器 5178，**逐档比对 mini / compact / wide / ultra 四档与试验场是否一致**，不一致就修到一致
    - 6.9: `tsc -b` / `lint` / `stylelint` / `build` 全过

---

## 第三段：Rust 侧地基

- [ ] Task 7: Tauri 2 脚手架跑通
    - 7.1: 装 `@tauri-apps/cli`（devDep）与 `@tauri-apps/api`，`package.json` 加 `tauri` script
    - 7.2: `npx tauri init` 生成 `src-tauri/`，crate 名 `mkp-support-ease`
    - 7.3: `tauri.conf.json`：`identifier: com.mkpsupport.ease`、`productName: SupportEase`、窗口标题 `SupportEase`、`devUrl: http://localhost:5178`、`beforeDevCommand: npm run dev`、`beforeBuildCommand: npm run build`、`frontendDist: ../dist`
    - 7.4: `npm run tauri dev` 首次跑通（冷编译几分钟是正常的），确认原生窗口里显示的是 v023 界面
    - 7.5: 把 CI 的 `rust` job 从"允许跳过"改成真跑
    - 7.6: 回填实际装上的 Tauri / crate 版本号到 `doc.md` §2.4

- [ ] Task 8: `error.rs` —— AppError 与 ErrorCode
    - 8.1: 定义 `ErrorCode` 八个变体（`NotFound` / `PermissionDenied` / `InvalidArgument` / `Corrupted` / `ShaMismatch` / `Io` / `NotImplemented` / `Internal`），serde `SCREAMING_SNAKE_CASE`
    - 8.2: 定义 `AppError { code, message, trace_id, detail }`，serde `camelCase`
    - 8.3: 构造辅助：`AppError::not_found()` / `invalid_argument()` / `io()` 等，以及 `with_trace(&str)`
    - 8.4: `From<std::io::Error>`、`From<tempfile::PersistError>` 等转换，`message` 一律是可展示的中文，技术细节进 `detail`
    - 8.5: 单元测试：序列化结果的字段名与 `contract.ts` 的 `AppError` 逐字段对得上

- [ ] Task 9: `obs/tracing.rs` —— 日志与 trace id
    - 9.1: `new_trace_id()` 用 uuid v7
    - 9.2: `init_tracing(log_dir)`：`tracing-appender` 的 `rolling::daily` + dev 下并输出 stderr
    - 9.3: 日志目录建不出来 / 写不进去时退到纯 stderr，**返回 Ok 不阻断启动**，但打一条 warn
    - 9.4: `lib.rs` 的 setup 里调用，日志目录取 `internal_root()/logs`

- [ ] Task 10: `fsx/paths.rs` —— 两层根与防穿越
    - 10.1: `internal_root()` = `appDataDir()`，首次 `create_dir_all` 建齐 `cloud/ archive/ index/ logs/ run/`
    - 10.2: `user_root()` = `documentDir()/SupportEase`，建齐 `exports/ reports/ presets-mine/`
    - 10.3: `enum Root { Internal, User }` + `resolve(root, rel) -> Result<PathBuf, AppError>`：拒绝绝对路径、拒绝 `..`、拼接后校验仍在根内，否则 `PermissionDenied`
    - 10.4: 单元测试：`../` 穿越、绝对路径、符号链接指向根外，三种都要被拒
    - 10.5: `capabilities/default.json` 按命令逐个授权 fs 权限 + scope 限到这两个根，**不使用 `fs:default`**

- [ ] Task 11: `fsx/atomic.rs` —— 唯一写盘出口
    - 11.1: 实现 `atomic_write(path, bytes)`：`NamedTempFile::new_in(parent)` → `write_all` → `sync_all` → `persist` → fsync 父目录
    - 11.2: 写 `src-tauri/clippy.toml`，把 `std::fs::write` / `std::fs::File::create` / `tokio::fs::write` 列进 `disallowed-methods`
    - 11.3: `atomic.rs` 内部 `#[allow(clippy::disallowed_methods)]` 开唯一的洞
    - 11.4: 单元测试：写入后内容正确；目录不存在时自动建；临时文件不残留
    - 11.5: 验证拦截确实生效 —— 在 `ipc/` 里临时写一行 `std::fs::write`，`cargo clippy -- -D warnings` 必须报错，然后删掉

- [ ] Task 12: `ipc/mod.rs` —— 五个 command 接上链路
    - 12.1: 一个 `traced_command!` 宏（或统一包装函数）：生成 trace id → 开 `info_span!` → 调 service → `map_err(with_trace)`
    - 12.2: 按 `contract.ts` 现有签名实现 `get_preset` / `save_offsets` / `get_calib_models` / `open_model` / `get_test_models`，**先返回硬编码值**，只为证明链路通
    - 12.3: `generate_handler!` 注册全部五个
    - 12.4: `save_offsets` 走 `atomic_write` 落到 `internal_root/index/offsets.json`，作为原子写的第一个真实调用点

- [ ] Task 13: 前端接线（3 个文件 + contract 扩展）
    - 13.1: `contract.ts` 增加 `ErrorCode` / `AppError` / `isAppError`，方法签名一个字节不动
    - 13.2: `bridge.ts` 内部换成 `invoke()`，加 `normalizeError`（非本结构的 reject 兜底成 `INTERNAL`，`traceId: '-'`，原值进 `detail`）
    - 13.3: `index.ts` 换成运行时探测 `'__TAURI_INTERNALS__' in window`，并把原注释改写说明"为什么不用 `import.meta.env.DEV`"
    - 13.4: 界面上有个能看见 traceId 的位置（错误提示右下角小字，可点复制）
    - 13.5: 实测：Tauri 窗口里数据来自 Rust；摘掉一个 command 的注册 → 界面报 `NOT_IMPLEMENTED` + traceId，日志里按该 id 能查到
    - 13.6: 实测：浏览器开 5178 仍走 mock，不报错

---

## 第四段：发版流程与文档

- [ ] Task 14: `release.mjs` 重写为 PR-only
    - 14.1: 前置检查沿用（不在 main / 工作区干净 / 闸门生效 / 复述 `bypass.log`），加一条：必须已配置 `origin`
    - 14.2: 校验链加 Rust：`lint` → `tsc -b` → `build` → `cargo clippy -D warnings` → `cargo test`
    - 14.3: 版本号三处一起改：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`
    - 14.4: `git push -u origin HEAD` → `gh pr create`（标题取那句说明）
    - 14.5: 轮询 CI 状态，绿了才 `gh pr merge --squash --delete-branch`
    - 14.6: `git switch main && git pull` → 在 main 的 tip 打 annotated tag `v<ver>` → push tag
    - 14.7: 任何一步失败停住并打印精确回退命令；破坏性命令只打印不执行

- [ ] Task 15: 三份文档就位
    - 15.1: `docs/ARCHITECTURE.md` —— 由 doc.md 的 §1.1 / §3 / §4 / §5 / §6 收口成常驻版，开头指认"产品规则见 PRESET-PRODUCT-RULES.md"
    - 15.2: `docs/PRESET-PRODUCT-RULES.md` —— 原样落成产品规则：位置×类型两维度、本地字段与排序、MKP 应用唯一性、切片器无应用、云端三状态、更新提示、下载、更新与归档、发布时间作用户可见版本、SHA 与失效、云端原件只读、Finder 复制脱钩、用户修改副本、与云端更新完全脱钩、归档、外部导入 TOML 校验、外部删除、第一版不做、最终状态模型图
    - 15.3: `docs/GIT-WORKFLOW.md` 重写：八道本地闸 + ruleset 清单 + 开局悖论那段 + 新发版流程 + **squash 会重写提交、tag 指向的 SHA 与分支不同**这条说明
    - 15.4: 搬 `docs/DESIGN-SPACING.md`、`docs/3D-ASSET-CONTRACT.md`
    - 15.5: 写 `README.md`：一句话定位、启动命令、三份文档的指路、以及"试验场在 mkp-adaptive-console"

- [ ] Task 16: 补上 required status checks 并合掉第一个 PR
    - 16.1: `setup-ruleset.mjs` 加 required status checks（`web`、`rust`），重跑脚本
    - 16.2: 造一次故意失败的 CI（临时引入一个 lint 错误）→ 确认 `gh pr merge` 被拒 → 撤掉
    - 16.3: `feat/bootstrap-shell` 开 PR，等 CI 绿，`--squash` 合进 main
    - 16.4: 确认 main 上历史是线性的，且 `.git/bypass.log` 里只有开仓那两条（Task 1.6 与 3.2），没有第三条

---

## 第五段：校验（逐条对 doc.md §8）

- [ ] Task 17: 收口校验
    - 17.1: `npm run tauri dev` 起原生窗口，标题 `SupportEase`，首页与校准页视觉与试验场四档逐档一致
    - 17.2: `npm run dev` + 浏览器 5178 走 mock，路径不丢
    - 17.3: Tauri 窗口里数据来自 Rust；摘 command → 报错带 traceId → 日志可查
    - 17.4: `internal_root/logs/` 有按天日志；两个数据根首次启动自动建齐（先删掉再启一次验）
    - 17.5: clippy 拦截、路径穿越拦截、原子写单测全绿
    - 17.6: `tsc -b` / `eslint` / `stylelint` / `vite build` / `cargo build` / `cargo clippy` / `cargo test` 全过
    - 17.7: git 拦截五条实测（main 直提 / push main / 分支名 / 大文件 / CI 失败不能合）
    - 17.8: `npm run tauri build` 出一个 macOS 包并能打开
    - 17.9: 回填实际依赖版本号到 `doc.md` §2.4，然后 `npm run release` 发 `v0.1.0`（这一步同时验证 Task 14 的新流程）
