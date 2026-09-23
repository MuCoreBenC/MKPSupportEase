# B04 修订计划：MKPSupportEase × mkp-ssr 整合

状态：基于消费端仓库的首轮代码证据修订；不是最终冻结的迁移方案。  
目标主仓：`MuCoreBenC/MKPSupportEase`（最终整合位置）。  
消费端来源：`MuCoreBenC/mkp-ssr`（私人仓库，当前 GitHub 连接可读取）。  
工作分支：`feat/b04-source-pipeline-plan`。

## 1. 本轮新增证据

已确认 `mkp-ssr` 的 `main` 分支可读取，且不是单纯的“JSON 消费客户端”：

- 根 `Cargo.toml` 是 Cargo workspace，成员包含 `crates/*` 与 `src-tauri`。
- Workspace 注释明确分为 `crates/core`（后处理内核）、`crates/preset`（预设 TOML → IR 映射）和 `src-tauri`（应用层）。
- `crates/preset/Cargo.toml` 将包命名为 `mkp-preset`，依赖方向为 `mkp-preset → mkp-pp`；并声明开发工具 `gen-presets`。
- `crates/preset/src/lib.rs` 明确：用户预设是 TOML，映射成 `mkp_pp::ir::Ir`；`load_ir()` 是产品路径入口，依次读取、校验配置、加载参数注册表、按注册表校验、build、规范化/校验机型并填充 machine facts。
- 同一文件还定义了编译进二进制的 `PARAM_REGISTRY_TOML` 和 `BUILTIN_PRESETS`（9 份预设）；注释说明内置预设随版本发布，由 `src-tauri` 的 `builtin::sync_builtin_presets` 同步到只读目录。
- `mkp-preset` 自带 TOML 注释/键序保留写回能力（`toml_edit`），并把写入风险与 K-P1/K-P2/K-P3、落盘前复检及备份关联起来。

证据文件：
- `mkp-ssr/Cargo.toml`
- `mkp-ssr/crates/preset/Cargo.toml`
- `mkp-ssr/crates/preset/src/lib.rs`

这组证据推翻了“消费端只是读取某个 JSON 包”的简化假设。至少当前 `mkp-ssr` 已有正式 TOML→IR 路径、内置预设嵌入与 TOML 编辑写回逻辑。后续必须核实这些实现与 `MKPSupportEase` 的 source/workbench 数据是否重叠、哪些应迁移/复用/替换，不能再另造一套并行 preset 管线。

## 2. 整合目标与边界

最终以 `MKPSupportEase` 为唯一整合仓库；`mkp-ssr` 是消费端代码与行为的来源，不是最终需要长期维护的第二个真源。

整合不等于直接复制整个仓库。先建立迁移映射：

| 领域 | 当前证据 | 下一步决策 |
|---|---|---|
| 预设源 TOML | `mkp-ssr/crates/preset` 支持读取/校验；`MKPSupportEase/presets/` 有源数据 | 比较字段、版本、来源与哈希，明确唯一权威源及目录归属 |
| TOML → IR | `mkp-preset::load_ir()` 已存在 | 核对目标内核 IR 兼容性；优先评估迁移/复用，不另写同义转换器 |
| 参数注册表 | `PARAM_REGISTRY_TOML` 编译嵌入 `mkp-ssr` | 与 `MKPSupportEase/presets/registry/param_registry.toml` 对照，明确生成/嵌入时机，禁止双份独立维护 |
| 内置预设 | 9 项 `BUILTIN_PRESETS` 编译嵌入并同步到只读目录 | 核对是否由目标仓库配方生成；改为可校验的单一生成入口，避免手写表漏项 |
| 编辑写回 | `toml_edit` 局部更新并复检 | 与工作台保存链路对照，复用经验证的安全机制；验证保存真正调用到 writer |
| Tauri 应用层 | `src-tauri` 为 workspace 成员 | 盘点 IPC、页面、数据根路径与依赖，分批迁入目标仓库 |

## 3. 修订后的阶段计划

### P0 — 双仓只读审计（当前阶段）

1. 固定两仓分支/commit，分别建立目录与模块清单。
2. 在 `MKPSupportEase` 追踪：TOML 源 → workbench 编辑/保存 → generator → 产物/清单。
3. 在 `mkp-ssr` 追踪：`load_ir()` 调用者、`BUILTIN_PRESETS` 同步、预设目录扫描、IPC/UI 的预设读取与编辑路径。
4. 比较两边机型/variant 命名、注册表、预设 schema、IR、内置预设清单及错误语义。
5. 形成逐项迁移表：保留/迁入/复用/重写/删除，并附路径与调用证据。

**P0 验收：**关键链路有真实入口和调用点；数据所有权明确；未查明的项目标记待确认，不冻结契约、不删除代码。

### P1 — 单一契约与迁移 ADR

1. 明确权威源数据只在 `MKPSupportEase` 哪个目录维护。
2. 分清源 TOML、内存 IR、内置资源、生成工件、运行时用户副本；分别声明 owner、schema、生命周期。
3. 定义 `mkp-preset` 与后处理 core 的依赖方向，以及 UI/IPC 可调用的稳定应用层接口。
4. 规定构建/生成入口、输出位置、版本策略、错误返回、确定性与兼容判据。
5. 将双方真实 fixture 纳入共享契约测试；在实现迁移前先证明旧消费路径可由契约覆盖。

### P2 — 最小纵向切片

选一份真实预设、一个参数、一个 variant：从唯一源读取 → 工作台展示 → 修改并保存（保留注释/键序）→ 重载验证 → 经 `load_ir` 转成真实 IR → 后处理/消费端验证。检查未选 variant、非目标文件哈希不变。

### P3 — 分模块迁入 `MKPSupportEase`

按依赖方向迁移：core/IR 基础 → preset 读取/校验/写回 → generator/内置资源同步 → app/IPC → 前端消费。每一段单独提交和测试；不做一次性整仓复制，不让两个仓库同时成为权威源。

### P4 — 切换与清理

新主仓链路完成验收后，再删除旧路径/重复资源/旧 upstream 读取层。清理前必须有调用图、搜索结果和测试证明；保留迁移期间的对照判据，不保留运行时双写或静默 fallback。

### P5 — 本地整合验收

- Rust workspace tests、clippy、fmt；前端 typecheck/test/build。
- TOML 零编辑往返与单键编辑局部差异判据。
- 参数保存、清除、跨 variant 隔离、备份/失败回滚。
- 生成产物可重现，`BUILTIN_PRESETS` 与正式源清单一一对应。
- 实际 `load_ir()` 读取生成/同步后的预设；至少一条真实后处理用例通过。
- 明确列出未运行项目；GitHub 文件写入/提交成功不等于本地构建通过。

## 4. 当前禁止事项

- 不把 `mkp-ssr` 的 `load_ir()` 误判为“客户端 JSON loader”。
- 不在未查明双方字段与资源责任前，冻结 `DATA-CONTRACT.md` 的具体 schema。
- 不复制两套注册表/预设作为独立权威源。
- 不把 9 项手写 `BUILTIN_PRESETS` 表当作自动完整；必须以生成/校验判据兜底。
- 不直接删除 `mkp-ssr` 既有逻辑；先确认其行为、测试与迁移去向。
- 不改动正式 preset 数据，除非单独提出、逐文件审查并验证。

## 5. 下一步

继续读取 `mkp-ssr` 的 `src-tauri` builtin/IPC、`crates/preset/src/generate.rs`、预设资源目录及相关测试；同时核对 `MKPSupportEase` 的实际生成端和保存调用链。审计结果进入 `AUDIT-EVIDENCE.md`，再据此修订 `DATA-CONTRACT.md` 与任务清单。