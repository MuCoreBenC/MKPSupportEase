# B04 交接（写给接手的新对话）

更新日期：2026-09-24
本文件以 GitHub 上 `.comate/specs/b04-panel-successor/tasks.md` 为长期总纲，
Task 13 的 M0–M7 是**迁移专项拆分**，不是整个项目的长期任务编号。

> **本文件替代 2026-09-23 之前那份 Task 8 时代的交接。** 那份里的「下一步 Task 8」
> 「`presets/` 还是未跟踪状态」「用 python 与 `mkpse-presets/source/` 比 sha256」
> 全部已过期，不要再照着做。

---

## 0. 你现在的位置（先核一遍，不要相信任何一句转述）

| 项 | 值 | 怎么核 |
|---|---|---|
| 工作分支 | `feat/b04-p3-migration` | `git status -sb` |
| `main` | `203c287`（PR #10 的 squash） | `git log --oneline -1 origin/main` |
| 本分支领先 main | **23 笔**（M1 / B2-prep / M2a / M2b / M3 / **M4a–M4d** / **写盘纪律 6 笔** / 文档与格式）—— **这个数以命令为准，别信本行** | `git log --oneline origin/main..HEAD` |
| 草稿 PR | **#11** —— **只作 CI 载体**（不打算按批次再开 PR） | `gh pr view 11` |
| 判据基线 | 内核 **249 + 2 ignored**；预设 **114**（75 单元 + 39 集成）；我们 **18**（默认）/ **264**（workbench） | 见 §8 |
| `cargo tree -d` 基线 | **61 条 / 27 个名字**（M4b 起，原 58/26，新增 3 条的出处见 §5.7） | `cargo tree -d` |
| `clippy.toml` 在哪 | **仓库根**（2026-09-24 从 `src-tauri/` 上移；就近优先不合并，见 §9 第 12 条） | `ls clippy.toml` |
| M4 那批的 CI | ✅ 全绿（`9c0a8ea`，run `35936554601`） | `gh run list --branch feat/b04-p3-migration` |
| 写盘纪律那 6 笔的 CI | **还没推** —— 推之前本地九条全绿（见 §8） | 同上 |
| 工作区 | 干净 | `git status --short` |

**第一件事就是把上表核一遍。** GitHub 上的合并状态**不代表本机 checkout 已同步** ——
上一轮就出现过「PR 已合并但本机还停在旧 HEAD」。

---

## 1. 新对话先读什么（别读全部历史）

| 顺序 | 文件 | 为什么 |
|---|---|---|
| 1 | `tasks.md` | **唯一长期主线 Task 1–22。**状态以它为准 |
| 2 | `MIGRATION-PLAN.md` | Task 13 的施工图（M0–M7 分段与验收） |
| 3 | `TASK-13-MIGRATION-INVENTORY.md` | 迁移盘点与**拆批定案**（`§6` 是定案表） |
| 4 | `TASK-13-WRITE-DISCIPLINE-OPTIONS.md` | 写盘纪律三方案比较与**定案**（C 主 + A1 辅，B 推迟） |
| 5 | `DATA-CONTRACT.md` / `AUDIT-EVIDENCE.md` | 数据契约与审计证据 |

不要用本文件替代 `tasks.md`。**三处冲突时的优先级**：仓库最新文件 > 提交与 CI 证据 > 本文件。

---

## 2. 项目终点（按 `tasks.md`）

1. workspace 根运行 `cargo test`，三个 crate 的判据全绿
2. 非测试代码中旧包名 / 旧仓库路径 / 旧 `content/` 链路清零
3. 自动判据覆盖「改值 → 生成 → `load_ir` → IR 中值正确」
4. 至少一条真实 G-code 后处理用例可验证
5. `presets/` 真实数据迁移前后 SHA-256 一致；有例外就逐文件审阅并记录
6. 侧边栏一页一件事，未完成的功能不摆按钮
7. 文档与代码不再把本项目称作"下游 / 消费端"

---

## 3. Task 1–22 状态总览

| Task | 主题 | 状态 |
|---|---|---|
| 1–10 | 结构摸底 → 参数值写回能力 | ✅ 已完成 |
| 11 | **M0**：比值，再搬代码 | ✅ 值等值已证；**精确领头键收尾**与 `eprintln!`→`assert!` 挂在 Task 15 之后 |
| 12 | 删自造 workbench JSON、收口数据模型 | ✅ `b38cca9` + `6880403` |
| 13 | **M1+M2**：workspace 与内核迁入 | ⏳ **M1 / M2a / M2b 已做，M3 另立 Task 14；M4 另立 Task 15（已做）；M5–M7 待做**（见 §4） |
| 14 | **M3**：尺寸/别名/禁区改从 `presets/` 读 | ✅ `62a72b9` |
| 15 | **M4**：预设解析迁入、注册表合一 | ✅ 四笔：`e87b90f` / `168a2b6` / `a3ae1cf` / `f7c38f3`（+ `554f601` 补格式） |
| 16 | **M5**：纵向切片（改值→生成→`load_ir`→真实后处理） | ⏭ 下一步；**欠两条接线**（见 §7） |
| 17 | **M6**：删除 `upstream/` 整层 | 待做 |
| 18 | 其余源文件搬进 `presets/` | 待做；**二进制资源归属需动手前定案** |
| 19 | 写入纪律 | 待做；方案已定、CI 前置已补 |
| 20 | 侧边栏 + 只摆做完的页 | 待做 |
| 21 | 机型/资源页补齐 + 参数页并入 | 待做 |
| 22 | **M7** + 清场与验收 | 待做 |

---

## 4. Task 13 专项进度（M0–M7）

| 段 | 内容 | 状态 | 提交 |
|---|---|---|---|
| **M0** | 比值再搬 | ✅ | 结论在 `DATA-CONTRACT.md` / `AUDIT-EVIDENCE.md` |
| **M1** | 建 workspace，`src-tauri` 成成员；依赖统一到根 | ✅ | `70eed0c`（布局）+ `44c5ebe`（依赖）；`Cargo.lock` 搬到仓库根 |
| **M2a** | `crates/core` → `crates/postprocess`，**原样复制** | ✅ | `47436e4` + `fef9698`（lock 补提） |
| **M2b** | 单独一笔机械改名 | ✅ | `d3f542b` + `133fa01`（清单顶部说明修正） |
| **M3** | 尺寸/别名/禁区改从 `presets/` 读 | ✅ | `62a72b9`（= Task 14） |
| **M4** | `crates/preset` 迁入，注册表合一 | ✅ | 四笔：`e87b90f`（原样搬）+ `168a2b6`（改名+入 members）+ `a3ae1cf`（注册表切源）+ `f7c38f3`（内置预设清单判据）；`554f601` 补一行格式 |
| **M5** | 生成器接上，`load_ir` 复检 | ⏭ | — |
| **M6** | 删 `upstream/` 整层 + 源码扫描断言 | — | — |
| **M7** | 措辞清场 | — | — |

### M1 定下的三件事（别重新讨论）

1. **`toml` 取 0.8，不取更新的 1.x。** 内核用 0.8 档的 API（`toml::Value` /
   `toml::map::Map` / `Value::try_from`），而 M2a 的搬运证据是**内容零 diff** ——
   取 1.x 就得改内核代码，改代码与搬运不能混在同一条 diff 里。我们这侧只用
   `str::parse::<toml::Table>()` 与 `toml::to_string`，0.8 上都有 ⇒ 单向让步，谁都不用改代码。
2. **`serde_json` 开 `float_roundtrip` + `preserve_order`**（不是优化项，是内核 golden 的
   正确性开关）。代价是**我们自己的解析行为也跟着变了一档** ⇒ 落地时重跑过 264，全绿。
3. **`members` 显式列，不用 `crates/*` 通配。** 目录还不存在时 glob 会让 cargo 报
   `failed to load manifest for workspace member ...`，看起来像清单写错了。

### M2a / M2b 的判据（这两个数是证据，不是估计）

- **M2a**：122 个文件**逐文件 sha256 与源一致**（只源有 0 / 只目标有 0 / 内容不同 0）；
  另核对「索引里的 blob == 工作区内容」（防 `.gitattributes` 的 `eol=lf` 归一化偷改 golden）
  ⇒ 122 个全部相同。内核自带测试 **246 passed + 2 ignored**。
  **逐字节比对的 golden 在 Windows/amd64 上全绿** —— 内核是在 macOS/arm64 上开发的，
  这条把「FMA 融合导致字节不等价」的担心实测排除了。
- **M2b**：对**只读源树**重放同一条有序替换规则，再逐文件比对 ⇒ **24 个文件全部相同**。
  两处例外必须知道：
  1. **rustfmt 的重排**（6 个文件 / 15 行删 / 27 行增）：`mkp_pp::`（7 列）变
     `postprocess::`（12 列）会把一些行推过 100 列，不重排 `fmt --check` 必红 ⇒ 只能同属一笔
  2. **一处手改**：那条「第一处 `name = "mkpse-pp"` → 包名」的规则本该只作用于
     `Cargo.toml`，却命中了 `src/main.rs` 里 clap 的 `#[command(name = ...)]`，已改回
     `mkpse-pp`。**这一处"逐文件重放"抓不到** —— 它只证明"新内容 = 源内容经同一规则"，
     规则本身错它照样全绿。是读结果读出来的。

### M3 定下的事

`presets/` 是尺寸 / 别名 / 禁区的**唯一来源**，两份编译期快照已删。数据改成**注入**
（`machine_dims::install`）或从目录读（`load_presets_dir`）：

- **`install` 第二次返回 Err 而不是覆盖** —— 静默替换会把"两处数据"变成"看谁先跑"
- 没人装时有一条**有顺序的逃生链**：`MKPSE_PRESETS_DIR` → 仓库相对 `../../presets` → panic。
  中间那条只为开发与测试成立（249 条判据会走到这里，而 `OnceLock` 只能装一次）
- **发布物必须在启动时 `install()`** —— 数据根运行时才知道，**这是 M5 欠的接线**
- 旧快照搬到 `tests/reference/legacy_snapshot/` 当**基线**，新判据
  `presets_are_the_only_source.rs` 证明 presets 能逐字段复现它（125 字段 / 23 别名 / 3 台禁区）。
  **这条判据一落地就抓到一次静默数据丢失**：`MachineFile` 少了 `rename_all = "camelCase"`
  ⇒ `externalAliases` 全被忽略、别名 23 条变 6 条、机型识别会全挂而**不会报错**。

---

## 5. 已定的关键纪律（迁移期，别重新讨论）

1. **先证明，再搬。** 搬运 / 改名 / 行为改动 / 依赖清理**分开**，每批只动一个不变量。
2. **迁移期间禁止 `UPDATE_GOLDEN=1`。** 一设就把判据变成"把现状抄成期望"。
3. **不以测试通过替代数据等值证据。** 除 M0 有逐文件结论外，**不改 `presets/` 里的真数据**。
4. **写盘纪律：源码扫描断言为主（C）、Clippy 为辅（A1）。**
   豁免必须**精确列出理由与退役条件**；「clippy 通过」**不等于**「写盘纪律已验证」。
   **覆盖面（2026-09-24 补齐，见 `.comate/specs/b04-preset-write-guard/`）**：
   `clippy.toml` 已从 `src-tauri/` **上移到仓库根**，`crates/postprocess` 与
   `crates/preset` 从此也受管（此前完全不受管，而 preset 有一处直接写 `presets/` 真源）。
   两层都用**探针**验过会响：摘掉内核那个 `#[allow]` ⇒ clippy 红；
   往白名单外的文件塞一行 `fs::write` ⇒ 扫描判据红并点名。
   扫描判据在 `crates/preset/tests/write_discipline_scan.rs`，白名单 4 个文件逐条写了
   "崩在半路会坏掉什么"。**它的口径不是"第一个 `#[cfg(test)]` 之前"** ——
   那样会漏掉写在 `mod tests` 之后的生产代码（既有那条 `upstream/mod.rs` 判据有这个洞，
   本轮探针打穿的就是它）。
5. **CI 必须覆盖默认与 workbench 两种 feature**（已补，见 §6）。
6. `mkp-ssr` 是**只读迁移源**；不迁前端、不做双向同步、**不留兼容别名**。
7. 判据的 `cargo tree -d` 口径：**"无重复依赖"不可能按字面执行**（Tauri 自己的树里
   本来就有 53 条）。它只能指"**每一条新增的重复都能追到一条决定**"。
   基线 **61 条 / 27 个 crate 名**（M4b 起；此前 58 / 26）。M4b 新增的 3 条逐条有出处：
   `toml_edit v0.20.2`（`toml 0.8` 的内部依赖）、`toml_edit v0.22.27`（preset 的写回保真，
   K-P1/P2/P3 的唯一实现）、`winnow v0.7.15`（前者 + tauri-build 那支各一档）——
   是「`toml` 取 0.8」与「写回必须用 toml_edit 0.22」两条决定叠加的必然结果。

---

## 6. 已并入 `main` 的 Task 13 前置（PR #10，`203c287`）

| 提交 | 内容 |
|---|---|
| `486b792` | 写盘纪律三方案比较与定案（C 主 + A1 辅，B 推迟） |
| `db4c12c` | 补齐**既有**格式欠账：189 处 rustfmt 差异 / 26 个文件，全在 `src/workbench/**` |
| `208a577` | **CI 补 workbench 覆盖**：`rust` job 的 clippy 与 test 各跑两遍 |
| `f90f57f` | **Clippy 禁列补全**：补 `File::options` 与 `OpenOptions::new`，并用探针验证会响 |

三条要点：

- **格式欠账是既有的**：`cargo fmt --check` 在 `bd1a173` 上就红。而 CI 的顺序是
  `格式 → clippy → 测试`，第一步失败后面根本不执行 ⇒ **不修格式，补 feature 也等于白补**
- **CI 的 `rust` job 现在在仓库根跑**（virtual manifest 默认成员 = 全部成员），
  缓存路径 `target`、key `hashFiles('Cargo.lock')`；`--features` 在 virtual 根上不合法，
  工作台那两条带 `-p mkp-support-ease`
- **禁列补全时发现** `presets::catalog::add_machine` 正当地用 `OpenOptions::new().create_new(true)`
  做"原子占住一个新路径"，于是换成语义逐字相同的 `File::create_new`（Rust 1.77 起稳定，
  本 crate MSRV 1.77.2），**没有引入第二个 `#[allow]` 逃生口**

---

## 7. Task 15–22 路线摘要

- **Task 15 / M4** ✅ 已做完（四笔，见 §4）。施工文档在
  `.comate/specs/b04-m4-preset-unify/`（`doc.md` / `tasks.md` / `summary.md`）。
  **它留给 M5 的两件事**：① 注册表仍是 `include_str!` 编进二进制（数据只有一份了，
  但发布物里改不了）；② `registry_edit` 的写盘目标已经是真源，**现在有能力污染
  `presets/`** —— 暂时靠"12 个文件 sha256 不变"那条验收兜着，没有代码级的闸
- **Task 16 / M5**：`wb_generate` 后**同进程** `load_ir` 复检；验证改值写盘 → 生成 → IR 值正确；
  未选变体产物**字节不变**、其他机型 SHA 不变；至少一条真实 G-code 后处理。
  **顺带必须做的两条接线**：启动时 `machine_dims::install()`（M3 欠的）+
  注册表改成运行时从数据根读（M4 欠的）—— 它们是同一件事的两半，一起做
- **Task 17 / M6**：删 `workbench/upstream/`、`paths::upstream_*`、`Roots.upstream`；
  修 manifest 悬空引用 / 资源缺失静默跳过 / 上游字段透传；落实非测试源码扫描断言
- **Task 18**：迁入 fallback registry、bundles、资源登记、release 元数据与版本；
  **二进制资源放 `presets/` 还是外挂，动手前必须定案**；FAQ/notification/theme/about/event
  不在本轮编辑范围
- **Task 19**：写盘统一入口、外部修改提示、回收站/撤销语义与界面文案复查
- **Task 20**：侧边栏按内容/交付/系统分组；删未完成页签/按钮；顶部状态与保存常驻
- **Task 21**：补齐机型尺寸/禁区/套餐/资源引用/删机型恢复；`ParamDesk` 并入参数页；
  版本矩阵比较；字段定义可改但 `tomlKey` 不可改
- **Task 22**：清场与验收（含手动主线：新增机型 → 新增版本 → 改值 → 生成 → `load_ir` → 打开 TOML）

---

## 8. 验证命令（照抄，**现在都在仓库根跑**）

```powershell
cd G:\project\MKPSupportEase
cargo test                                     # 默认 feature（三个成员）
cargo test -p mkp-support-ease --features workbench   # 工作台那 264 条
cargo test -p mkpse-postprocess                # 内核那 249 条
cargo test -p mkpse-preset                     # 预设那 114 条
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets -p mkp-support-ease --features workbench -- -D warnings
cargo tree -d                                  # 61 条 / 27 名（口径见 §5.7）
npm run lint; npm run build
```

- **判成败看 `test result: ok. N passed`**，不要看管道退出码
- 期望值：内核 `249 passed + 2 ignored`（2 条是性能探针）/ 预设 `114`
  （75 单元 + 39 集成）/ 我们 `18` 与 `264`
- **`cargo test` 之外别忘了 `fmt --check`**：M4c 就是只跑了测试，漏了一行格式，
  全量验收才报出来（补在 `554f601`）。CI 的顺序是格式 → clippy → 测试，格式红了后面不跑
- **动过 `clippy.toml` 之后，clippy 那两条要先 clean 再跑**（见 §9 第 11 条）：
  `cargo clean -p mkpse-preset -p mkpse-postprocess -p mkp-support-ease`
- 真数据是否被污染：`git status --short` 必须只剩你**有意**改的那些文件；
  `presets/` 的 12 个文件在整个迁移里**一个字节都不该变**。
  核法（比"和某份旧清单比"更强，它证明的是与 HEAD 一致）：
  `git diff --stat HEAD -- presets/` 为空 + `git log --oneline -1 -- presets/`
  应停在 `203c287`（PR #10）+ 逐文件核「索引 blob == 工作区内容」12/12

---

## 9. 环境陷阱与操作备忘（这一轮踩出来的）

1. **`gh` 不读 Windows 系统代理。** 这台机器走本地代理 `127.0.0.1:7890`：
   git 读自己的 `http.proxy`、PowerShell/.NET 读系统代理，而 **Go 写的 `gh` 只读环境变量**。
   症状是"git push 能过、`gh api` 能过、但 `gh auth refresh` 在 `POST /login/device/code`
   连接超时"。
   **本机已按用户环境变量永久设好**（2026-09-24）：
   `HTTPS_PROXY=http://127.0.0.1:7890`、`NO_PROXY=localhost,127.0.0.1,::1`。
   - **副作用要知道**：凡是读这两个变量的程序都会跟着走代理 —— `cargo` / `npm` /
     `node` / `pip` / 各家 CLI。代理没开时它们会**连不出去**，而不是回落直连。
     临时关掉：`$env:HTTPS_PROXY=''`（只影响当前会话）。
   - `NO_PROXY` 是为 `tauri dev` / vite 的本机回环准备的，别删。
   - 新开的终端才能看到 `setx` 的结果；**当前会话仍需 `$env:HTTPS_PROXY=...`**。
2. **改 `.github/workflows/` 需要令牌带 `workflow` scope**，否则 push 被 GitHub 拒
   （`refusing to allow an OAuth App to create or update workflow ...`）。
   解法：`gh auth refresh -h github.com -s workflow`（**要本人过一遍浏览器**）。
3. **提交信息文件写进 `.git/` 偶发落成 0 字节**（本轮中过两次，表现为
   `Aborting commit due to empty commit message`）。稳的做法：写到**仓库根的临时文件**，
   **同一条命令里先验长度再提交，成功后立刻删**。
4. **管道会吞掉退出码**：`cargo test | Select-String ...` 全通过也可能返回 1。
5. **这台机器的默认 shell 是 PowerShell 5.1，不认 `&&` / `||`。**
   写了就报 `标记"&&"不是此版本中的有效语句分隔符`，看起来像命令本身错了。
   用 `;` 串（不判成败）、或写成分行、或要判成败时用
   `cmd1; if ($LASTEXITCODE -eq 0) { cmd2 }`。第 3 条那个"先验长度再提交"就得这么写。
   **另一个同类坑：双引号串里不能用 `\"` 转义引号。** 本轮栽了两次 ——
   一次 `git commit -m "… \"../core\" …"` 把 `--check` 当成 git 的选项（报
   `unknown option 'check'`），一次探针脚本直接解析失败。要在字符串里放引号，
   用单引号串（`'let _ = fs::write(p, "x");'`）或 `[char]34`。
6. **PowerShell 的 `>` 重定向写 UTF-16**，拿它比对中文文本会得到"不一致"的假象；
   用 `[System.IO.File]::ReadAllText(path, UTF8)`。
7. **`.NET` API 的相对路径按进程 CWD 解析**，不是按 PowerShell 的当前位置 —— 混用会
   报"找不到路径"。
8. **`cargo metadata` 的输出带 BOM**，`node` 里 `JSON.parse` 前要 `.replace(/^\uFEFF/,'')`。
9. **`file!()` 的基准会随构建方式变**：单 crate 时相对 crate 根，建了 workspace 之后
   相对**仓库根**。凡是用它拼路径的判据都要改成 `CARGO_MANIFEST_DIR` + 显式相对路径
   （本轮就有一条判据因此以"报错"形式失效，见 §4 M2b 那类坑）。
10. `tauri dev` 异常退出会留两种残留（vite 占 5321、app exe 占文件锁），症状是**白屏**。
11. **`cargo clippy` 会跳过未变更的 crate —— 改完 `clippy.toml` 直接跑，命中清单是假的。**
    实测：把禁列放到仓库根之后第一次跑报 **9 处**且 `crates/postprocess` 一处都没有；
    `cargo clean -p mkpse-preset -p mkpse-postprocess` 之后重跑是 **15 处**
    （生产 5 + 测试 10）。这与"管道吞退出码"是同一类陷阱 ——
    **改任何 lint 配置之后，先 clean 再验，否则你在看一份空转的结果。**
12. **`clippy.toml` 就近优先、不合并。** 成员自带一份就会**整份屏蔽**仓库根那份，
    而且没有任何警告（两次探针实测，理由写在根 `clippy.toml` 顶部）。
    所以要给某个成员放宽，**别新建 crate 级配置**，用 `#[allow]` 逐处写理由。

---

## 10. 常用仓库与文件

- 仓库：`MuCoreBenC/MKPSupportEase`；PR #11（草稿，CI 载体）
- 长期总纲：`.comate/specs/b04-panel-successor/tasks.md`
- 本轮施工文档：`.comate/specs/b04-m4-preset-unify/`（Task 15 / M4）、
  `.comate/specs/b04-preset-write-guard/`（写盘纪律补齐，Task 19 的提前一小块）
- 写盘禁列：**仓库根** `clippy.toml`（就近优先不合并，见 §9 第 12 条）；
  扫描判据：`crates/preset/tests/write_discipline_scan.rs`
- 迁移计划：`MIGRATION-PLAN.md`　盘点：`TASK-13-MIGRATION-INVENTORY.md`
- 写盘纪律方案比较：`TASK-13-WRITE-DISCIPLINE-OPTIONS.md`　数据契约：`DATA-CONTRACT.md`
- 审计证据：`AUDIT-EVIDENCE.md`　重建计划：`REBUILD-PLAN.md`
- 只读迁移源：`G:\project\mkp-ssr`（**不在里面写任何东西**）
