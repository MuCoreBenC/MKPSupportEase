# 地图 3 · 技术架构地图（骨架）

> 最后核对：2026-11-07（目录与模块级；**逐条调用图未完成**）
> 定位：回答"代码怎么组织、一个动作从页面走到哪里"。主链部分已在 `02-flows.md` 里给到函数级；本图只铺骨架。

---

## 1. 仓库分三层（代码骨架）

```
crates/                 # 纯 Rust 库 + 独立二进制（客户端与工作台共用）
├── preset/             # 预设 TOML → IR（唯一映射点）
└── postprocess/        # G-code 后处理内核（12 步 pipeline）+ 自己的 CLI

src-tauri/src/          # Tauri 壳（客户端本体）
├── main.rs             # 入口：客户端 / Hook 模式 / CLI 分派（Mode::Hook）
├── lib.rs              # Tauri builder + IPC 注册表
├── hook.rs             # 切片器钩子主流程（161 KB，最重的一个文件）
├── hook_ipc.rs         # 钩子 → 工作台/客户端的事件通道
├── ipc/                # 所有 #[tauri::command]（按域分文件）
├── runtime/            # 客户端运行时：catalog / delivery / update / source / resolver / local / net
├── presetdata/         # 预设数据模型：catalog / bundles / registry / resolve / patch
├── workbench/          # 工作台后端（feature = workbench）
│   ├── app/            # 命令层 + 事务（build / delivery / audit / publish_tx / publish_book / bundles / machines / assets）
│   ├── domain/         # 纯领域：derive(Book) / patch / issues / history / derive
│   ├── paths.rs        # 数据根（= 仓库根）
│   └── store.rs        # workbench/ 下的会话存储
└── fsx/                # 原子写、路径解析、回收站

src/                    # 前端
├── app/                # 客户端 UI（首页、下载并应用、后处理弹窗）
├── workbench/          # 工作台 UI（8 个页面 + c14 组件库 + api.ts）
└── components/         # 共用组件
```

---

## 2. 前端 → 后端：只有一条通道

- 工作台与客户端都用 `invoke('<command>', args)`（`src/workbench/api.ts` 是工作台的唯一封装；`src/api/**` 是客户端）。
- 命令清单在 `src-tauri/src/lib.rs` 的 `invoke_handler` 注册；**读命令必须 `#[tauri::command(async)]`**（有源码扫描测试钉住：`workbench/app/mod.rs` 的 `read_commands_are_async_so_they_never_freeze_the_window`）。
- 网络/凭据类命令（`wb_publish` 等）也在 `(async)` 白名单里，避免主线程冻结。

---

## 3. 工作台后端的"事务"分层（读这层要看的三件事）

| 层 | 文件 | 职责 | 是否写盘 |
|---|---|---|---|
| 命令壳 | `workbench/app/*.rs` 的 `wb_*` | `with_ctx` 取会话 + `traced` | 视命令 |
| 锁无关内核 | `*_with(ctx)` / `*_inner(...)` | 可被其它事务复用（锁不可重入） | 视内核 |
| 事务 | `generate_transaction`（`delivery.rs:1723`）、`publish_tx::run` | 备份 → 写 → 回读验证 → 失败回滚 | ✅ |

**规矩**：事务里不许再调会自己取锁的命令（会把主线程锁死）；`publish_tx.rs` 有源码扫描测试守这条。

---

## 4. 待补（下一轮）

- [ ] 每个 `wb_*` 命令 → 内核函数 → 写盘落点的完整清单（当前只覆盖主链）。
- [ ] `hook.rs` 的模块切分（161 KB 里各段职责）。
- [ ] `runtime/*` 12 个文件的依赖关系图（谁调谁）。
- [ ] 前端 `src/app`（客户端）页面 → 命令清单（当前只看了首页与后处理弹窗）。
- [ ] `crates/postprocess` 子模块（`gcode/`、`pass1/`、`pass2/`、`tower/`、`disk/`、`glue/`）的依赖图。

---

## 5. 验收问题（这张图必须能回答）

1. 一个前端按钮点下去，怎么找到它对应的 Rust 函数？（api.ts → 命令名 → lib.rs → app/*.rs）
2. 哪些命令会写盘？哪些只读？（依据：内核里有没有 `atomic_write` / 事务）
3. 工作台与客户端共用哪些 crate？分界在哪？（`presetdata` / `crates/*` 共用；`workbench` 只在工作台）
4. 为什么事务必须写成"命令壳 + 锁无关内核"两层？
5. 新增一个页面/命令要改哪几处才不漏（api.ts / lib.rs / 命令白名单 / 页面注册）？
