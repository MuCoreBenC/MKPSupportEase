# 交接：数据架构重做 —— 第一圈完成，第二圈进行中

> 更新时间：2026-10-01
> 适用分支：`main`（只认 CI 绿的 tip；推送用 `ALLOW_PUSH_MAIN=1`，提交用 `ALLOW_COMMIT_ON_MAIN=1`——本地闸，见 §5）
> 上下文：按《四圈舞步》重做数据架构。**根规则**是 `docs/DATA-ARCHITECTURE.md`（四层世界 + 四条铁律 + 十问 + 准入问句），**对账单**是 `docs/DATA-INVENTORY.md`（43 件现状逐件归位 + 收口进展日志）。这两份先读，本文只讲"现在在哪、接下来去哪"。

## 0. 进度总览

| 圈 | 内容 | 状态 |
|---|---|---|
| 第一圈 | 骨架全立起来：数据世界 / 最小 Catalog / 文件系统 / Delivery 骨架 / 用户数据 / 页面闭环 | ✅ **100%**（六块全通，2026-10-01） |
| 第二圈 | 每块地基做厚 | 🔄 **刚开始**：更新与归档已落（本文 §3），manifest 契约 / catalog 加厚 / localStorage 退役未动 |
| 第三圈 | 所有业务接进新地基（Preset 全功能 / 模型 / BBS / 报告） | ⬜ 未开始 |
| 第四圈 | 完整产品行为（三状态流转 / 冲突 / SHA 异常边界 / UI 状态） | ⬜ 未开始 |

**整体 ≈ 25%。** 判断依据：第一圈是骨架（六块全绿、139 条 Rust 测试、四条总纲判据落地 2 条），但 catalog 还只有"清单"没有"完整定义"，云端是零依赖的空位，旧世界（`client/` 铺 13 份源 TOML + localStorage 三格）原样并存未退役。

## 1. 第一圈留下的东西（全部在 main 上）

| 块 | 落点 | 一句话 |
|---|---|---|
| ① 数据世界 | `src-tauri/src/runtime/`（mod/paths/release/catalog/delivery/state） | 新世界的落点、释放口、说明书、管道、状态全在这一个模块 |
| ② 最小 Catalog | `cargo run --bin gen-catalog` → `src-tauri/src/runtime/catalog.generated.json` | 读 `presets/` 源 + `crates/preset/assets/presets/` 入库产物真字节（SHA/大小），编进二进制；判据测试守"重建逐字节一致" |
| ③ 文件系统 | 复用 `fsx/`（两根 + 防穿越 + atomic_write）+ `runtime/paths.rs`（catalog.json / mkp/ / archive/ 落点） | 业务不许自拼目录名；新增子目录先改总纲 |
| ④ Delivery | `runtime/delivery.rs`：`Source` trait + `deliver` + `FileOnDisk` 三分态 | 源可插拔（现只有 LocalDirSource）；SHA/大小校验在落盘前；更新=重跑管道，旧份自动归档 |
| ⑤ 用户状态 | `runtime/state.rs` | 规则：一种状态一个文件、住 `run/` 下、atomic 写、坏档不静默。第一个住客：使用中指针（`run/active-preset.json`，全局唯一在构造上成立，带应用时刻 SHA 可查漂移） |
| ⑥ 页面闭环 | 同步页「数据骨架（新）」区（`src/app/pages/PagePackage.tsx`） | 读数 + 下载 + 使用/撤销 + 更新，全走新 API（`get_runtime_catalog` / `download_runtime_file` / `get_downloaded_files` / `get_stale_files` / `get_active_preset` / `apply_active_preset` / `clear_active_preset`——两份命令清单同步注册） |

## 2. 怎么验证（每一轮都要全绿才推）

```bash
cargo test                                   # 默认 feature（在仓库根跑，workspace 全成员）
cargo test -p mkp-support-ease --features workbench --lib   # **workbench 的判据也要跑**（CI 两个都跑）
cargo clippy --all-targets -- -D warnings    # clippy 两种 feature 都要：默认 + --features workbench
cargo fmt --check                            # CI 有格式 job
npm run build && npm run check:bundle        # 前端构建 + 判据 1（安装包产物扫描）
npx eslint <改过的文件>                       # CI 跑全量 lint
```

改了 `presets/` 源或交付产物后必须 `cargo run --bin gen-catalog` 重生成，否则判据测试 `embedded_matches_rebuild` 红。

## 3. 第二圈还剩什么（按建议顺序）

1. **R11：manifest 升为两端共用契约** —— 交付格式定义现在躺在 `src-tauri/src/workbench/app/dist.rs` 头注释里（manifest v3 的设计：全部交付文件的哈希清单、残留拦截）。把它提为工作台（发布方）与 `runtime/`（消费方）共用的类型，先于一切网络代码。**注意作者裁决：新命名不背 "v3" 的名字。**
2. **catalog 加厚 + 换源收口（R3/R4/R5）** —— catalog 现在只有机型/版本/文件清单，要长出完整定义（参数注册表、布局、套餐、资产）才能让客户端第一屏只读 catalog；然后 `client/defaults.rs`（include_str! 13 份源 TOML）与 `client/paths.rs`（seed）退役，`ipc/presets.rs::load_presets` 换源。这是总纲欠账 #1 的正主。
3. **C4：旧世界 localStorage 三格退役** —— `src/api/storageKeys.ts` 的 `a40.package / a40.presets / a40.active`（说明书/本机预设/使用中）。新世界的对应物已就位（catalog / mkp/ / run/active-preset.json），迁完删键。**这是盘点里最大的一笔欠账**。
4. **Delivery 再加厚** —— 下载进度、失败重试、并发；真云端 `Source` 实现（reqwest vs ureq 选型，零网络依赖的现状要打破，选型时考虑 `spawn_blocking`）。
5. **判据 2 / 4** —— 启动零网络（先源码扫描）、首屏唯一数据源 = catalog（随换源落地自然成立）。

## 4. 续做入口（从哪接手）

- 更新/归档（本轮刚落）：`runtime/delivery.rs` 的 `FileOnDisk` / `stale_files` / deliver 里的归档段；`runtime/release.rs` 的升级策略（盘上 catalog 与随包不同 → 旧份归档、新份生效）。
- catalog 加厚：`runtime/catalog.rs::build_from_repo`（加字段不升 `CATALOG_SCHEMA`，改语义才升）。
- 命令面：`src-tauri/src/ipc/catalog.rs`；**注册必须两份清单同步**（`lib.rs` 两个 `generate_handler!`）。
- 换源：`src-tauri/src/ipc/presets.rs::load_presets`（现在的缓存机制 `forget_cached_presets` 就是给这一天留的）。
- 假云端退役：`src/workbench/cloud.ts` + `src/workbench/fixtures/cloud-presets.json`（静态快照已挪出 public/，客户端产物已无假数据，判据 1 盯着）。

## 5. 已知纪律 / 不要碰（每一条都是踩过的坑）

- **总纲准入问句**（`DATA-ARCHITECTURE.md` §6）：任何新文件/新功能先答"属于哪一层？谁是唯一主人？什么时候允许联网？"答不出先改文档。
- **四条铁律**：开发文件不当运行时数据库；云端不参与首屏；用户没下载的不预置（catalog 是唯一例外——它是软件本体）；运行时只认自己的运行时数据。
- **clippy 禁列对测试也生效**（CI 是 `--all-targets`）：测试里写盘用 `fsx::atomic::atomic_write`，`std::fs::write` 会红。
- **`b"..."` 字节串装不下中文**：含中文用 `"…".as_bytes()`。
- **cargo fmt 是 CI 的独立 job**：推前跑 `cargo fmt`。
- **`presets/dist/` 是本机产物，不入库**；判据/构建器的输入用入库真身 `crates/preset/assets/presets/`（9 份，`BUILTIN_PRESETS` 编的就是它）。
- **新旧世界并存、互不读写**（`client/` ↔ `runtime/`）；收口一条勾一条，同步更新 `DATA-INVENTORY.md` §4 与总纲 §4。
- **`workbench/.snapshots/` 已入库**（`.gitignore` 的既定政策：除 `.draft/` 外入库）；`.codebuddy/` 已 ignore；`.trae/documents/` 留库（被代码注释引用）。
- **仓库有并行会话在动**：推送前先 `git fetch`；合并冲突大概率在 `ipc/presets.rs` / `usePresetData.ts`（预设页是热点）。

## 6. 判据清单（全是活的，别删）

| 判据 | 守什么 |
|---|---|
| `runtime::tests::embedded_matches_rebuild` | 编进二进制的 catalog = 重新构建的那份（源/产物改了没重跑 gen-catalog 就红） |
| `runtime/delivery` 9 条 | 校验在落盘前 / 防穿越 / 更新归档旧份 / 归档槽保最早 / 幂等 / Stale 可见 |
| `runtime/state` 7 条 | 往返 / 缺省 None / 坏档 CORRUPTED / 未来代次拒 / 后应用赢 / 撤销幂等 / 漂移检测 |
| `runtime/release` 4 条 | 首启铺 / 同版本不动 / 升级归档换新 / 归档槽保最早 / mkp/ 初始为空 |
| `client/paths` seed 测试 | 旧世界的 mkp/ 初始为空（铁律 3 在旧世界的影子，收口 C4 时随旧世界一起退役） |
| `scripts/check-bundle.mjs`（CI web job） | 判据 1：开发源 TOML / 模拟数据 / 工作台内容不进客户端安装包 |
| `crates/preset/tests/builtin_presets_match_dir` | 旧世界判据，仍有效 |

## 7. 仓库状态速记

- main 与 origin/main 同步，最新提交见 `git log`；CI 两个 job（web / rust）必须全绿。
- 未入库的 untracked：无（`workbench/.draft/` 被 ignore 属预期）。
- 本机开发源可用性：`crates/preset/assets/presets/` 存在时，同步页的下载/使用/更新全链路真机可跑；用户安装包里这些按钮诚实报"还没接"。
