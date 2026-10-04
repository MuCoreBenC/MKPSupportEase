# 第四刀交接（发布软件版本：版本号 / tag / 安装包 / Release / release.json）

> **给新对话窗口的第一份材料。** 开工按这个顺序读：
>
> 1. 本文件（这一刀要做什么、现状有什么、哪些要作者拍）
> 2. `docs/RELEASE-TRANSACTIONS.md`（**合同**：§1.2 = 软件版本发布那一条链的定义；本文件只是施工计划）
> 3. 根 `HANDOFF.md`（进度台账；§0 进度 / §5 纪律 / §6 判据）
> 4. `docs/PUBLISH-ARCHITECTURE.md`（发布链根规则）+ `docs/DATA-ARCHITECTURE.md`（总纲）
>
> 规则 > 代码。**改规则（合同 / 判据口径 / 数据落点）要作者点头**，其余自己定、理由写台账。

---

## 0. 一句话现状

「发布预设」那条链**已经整条验收完**，并且 **#27 已合进 main**（squash：`c8e7e10 发布：17 份产物 (#27)`）：

```text
编辑 → 发布（工作台）→ 十五项闸 → 生成 → 定稿 → commit → push
     → PR（已有开着的就回读那一份）→ 回执屏 → 【合并】（软件内 squash）→ 客户端可取
```

判据：`cargo test` **314** / `cargo test --features workbench --lib` **575**。

**这一刀是两套事务里的第二套**：`发布软件版本` —— 产出一个**新的、可下载的 SupportEase 安装包**。
合同 §1.2 已经把八步定死，本文件把那八步落到"谁做、用什么做、缺什么"。

**开工前的一次性动作**：从最新 main 建新分支（#27 是 squash 合并，旧分支的谱系与 main 对不上）：

```bash
git switch main && git pull && git switch -c feat/software-release
```

---

## 1. 八步的现状与缺口

| 合同的步（§1.2） | 今天有什么 | 缺口 |
|---|---|---|
| 1 确认主线 | 无 | 工作台要有一格"要发的代码都在 main 了吗"（本地分支 vs 远端 main） |
| 2 确定版本号 | **`scripts/release.mjs` 已能改四处**（`package.json` / `src-tauri/Cargo.toml` / `tauri.conf.json` / `Cargo.lock`）+ `x.y.z` 校验 + tag 冲突检查 | 版本号"唯一真值"口径（§3-1）+ 三处一致的判据 |
| 3 tag | 同一个脚本：回 main 后打 `vX.Y.Z`（纪律：tag 必须打在 main 的 tip 上） | tag 之后**什么都没有** |
| 4 构建安装包 | `tauri build`（`bundle.targets = "all"`）本机可出 `.app` / `.dmg`；**CI 里没有打包 job**（`ci.yml` 只有 web / rust / rust-windows 三个校验 job） | 打包放哪做（本机 vs CI matrix）、签名 / 公证（§3-2） |
| 5 Release | 无 | `POST /repos/{o}/{r}/releases`（平台出口已有，见 §2） |
| 6 上传安装包 | 无 | `POST …/releases/{id}/assets` —— ★ 这是**二进制上传**，与现有 JSON 出口不是一回事（§3-4 备注） |
| 7 更新 `release.json` | 文件已在（**位置对不上，见 §3-3**）；`runtime::release_info`（形状 + 代次 + 比较）、`net::get_release`、`source::release_url` 都已就位 | 谁来写、什么时候写、写在**哪里** |
| 8 客户端发现新版本 | ✅ **已做完**：设置页「软件更新」三态（正在检查 / 有新版本 / 已是最新 / 这次没查到）+「查看更新」外链 +「重新检查」；**只在打开设置页时问一次**（铁律 2） | 无 —— 本轮就到"看得到、点得走"为止（§3-6） |

---

## 2. 已有什么（别重造）

**软件版本这一侧（第三刀下半留下的）**

- `src-tauri/src/runtime/release_info.rs` —— `ReleaseInfo`（`version` / `notes` / `url` / `releaseSchema`）+ 代次。
- `src-tauri/src/runtime/net.rs::get_release` —— 取字节（网络只住这一处）。
- `src-tauri/src/runtime/source.rs::release_url` —— 从数据源 base 往上**恰好一级**推 `release.json` 的地址。
- 客户端 `src/app/settings/PageSettings.tsx` 的「软件更新」块；`src/api/contract.ts` 的 `SoftwareUpdate`；`src/api/mock.ts` 的演示桩。

**工作台这一侧（发布预设那一刀留下的，形态可直接照抄）**

- `workbench/app/publish_tx.rs` —— **锁无关内核 + 壳层命令**的分法（`run(ctx, opts, …)` vs `wb_publish`）。
- 闸 → 回执 → 历史 →（每条）手动刷新 的四件套：`audit.rs` / `PublishGateModal` / `HistoryModal` /
  `<appDataDir>/publish-history.json`（`app/history.rs`：schema + atomic_write + 坏档 `CORRUPTED` 不静默）。
- 平台出口：`workbench/app/platform/{github,gitee}.rs`（`Hosting` trait；`agent()` 带 30s / 10s 超时）。
- Keychain 会话缓存（`credentials::session()`）+ 发布账户配置（`account.rs`，配置与秘密分离）。
- `wb_app_version`（当前安装的版本号）+ `wb_open_external`（只放行 http(s)）。
- **★ `scripts/release.mjs`（现成的 CLI）**：前置检查 → 版本号四处 → 校验（前端 + Rust）→ 推分支开 PR →
  等 CI → squash 合并 → 回 main 打 tag。三条纪律写在文件头：**任何写操作前先跑完校验**、
  **破坏性命令只打印不执行**、**tag 打在 main 的 tip 上**（squash 会重写提交）。

---

## 3. 待作者拍板（每条带建议）

1. **版本号的唯一真值**：现在三处（+ `Cargo.lock`）靠脚本一起改。
   **建议**：`src-tauri/Cargo.toml` 是唯一真值（`CARGO_PKG_VERSION` → `APP_VERSION`，客户端已按它走），
   另两处由脚本同步，并**加一条判据钉住三处一致**（不一致 = 红）—— 它已经是"软件版本"的全局真值。
2. **安装包谁来打 / 打哪个平台 / 要不要签名**：
   **建议**第一步**只打 macOS**（本机 `tauri build` 或 CI 的 `macos-latest`），Windows 留给后续；
   **签名 / 公证先不做** —— 代价是用户首次打开有 Gatekeeper 警告，**要作者确认能接受**
   （若不接受，这一刀就要把签名与公证一起做掉，工作量另算）。
3. **★ `release.json` 的落点（现状对不上，必须先定）**：
   客户端按 `release_url(base)` 找这个文件；以现发布形态（bootstrap 指 `…/presets/dist/source.json`）
   base = `…/presets/dist` ⇒ 它会去 **`…/presets/release.json`**，而文件现在住**仓库根** ⇒ 实际读不到。
   两个自洽解：
   - **①（推荐）把文件挪到 `presets/release.json`**：与"发布根 `presets/` 之外（= `presets` 这一级）"一致，
     且"发布软件版本"事务顺手把它提交掉（它**不进** 发布预设的白名单，两条链不混）。
   - ② 客户端的数据源根改成指到 `presets/`：会牵动 `catalog.path` 的基准，代价大。
4. **这一刀与 `scripts/release.mjs` 的关系**：**建议"内核一次、两个壳"**（与发布闸同一条思路）——
   把版本号推进 / tag / Release 变成 Rust 侧**可被判据调用的自由函数**，工作台是一条壳，脚本先留着当第二条壳
   （它那三条纪律与"tag 打在 main tip"的口径原样继承）。
   备注：**上传安装包是二进制**（`STAGE_ALLOWLIST` 那套 `read_json` 出口不适用），要单独写一条
   "把文件当 body 发上去"的路，超时口径也要另给（30s 装不下一个 dmg）。
5. **Release 的形态**：asset 命名（如 `SupportEase_0.0.1_aarch64.dmg`）、Release 正文用什么
   （changelog / `notes`）、要不要 pre-release。
6. **"用户更新"的边界**：**建议本轮只到"打开下载页"**（客户端已有 `url` 外链）。
   **不做**应用内下载 / 替换 / 自动重启 —— 那是自动更新，另一条链、另一把刀。

---

## 4. 建议的施工顺序（每步一句话完成条件）

1. **落点与版本号**（§3-1 / §3-3 拍完就做）：`presets/release.json` 就位 + 三处版本号一致的判据。
   完成条件：判据在"三处不一致"时能报红。
2. **软件版本事务的锁无关内核**（照 `publish_tx` 的形状）：`release_tx::run(ctx, opts, …)` =
   确认主线 → 版本号 → tag → Release → 上传 → 写 `release.json`；**先只让判据调它，先不做界面**。
   完成条件：假平台（注入的 `Hosting`）跑一遍，阶段报告与 `publish_tx` 同形（stage 快照 + summary）。
3. **平台动作**：`Hosting` 之外加 `create_release` / `upload_asset`（二进制 body）。
   完成条件：超时 + 4xx 分类 + "不借用户 gh 登录态"三件事与现有 `platform` 一致。
4. **界面**：工作台「发布软件版本」页/事务 —— 照 `PublishGateModal` 的闸 → 回执 → 历史三件套；
   ②卡那颗只读的「软件版本」块升级成入口。完成条件：探针能走查"发布软件版本"的回执与历史。
5. **端到端验收**：`0.0.1` 打第一个正式安装包 → Release → 写 `release.json` →
   旧客户端打开设置页看到「有新版本 SupportEase」→「查看更新」跳得走；再发 `0.0.2` 验证第二次。
   完成条件：**两个正式版本的真实链路各走通一次**。

---

## 5. 范围纪律（这一刀不做）

- 自动更新（下载 / 替换 / 重启）、应用内更新器；
- 安装包签名 / 公证的复杂化（除非 §3-2 拍成"做"）；
- Windows 打包（除非 §3-2 拍成"做"）；
- 不动「发布预设」那条链的语义 —— **两条链不混**：`release.json` 不进 catalog / manifest / 发布闸；
- 不为"以后可能要"提前抽象。

---

## 6. 纪律速查（借来的，原样有效）

- `with_ctx` 锁**不可重入**：事务内核只收 `&Ctx`，**绝不**调命令壳或"会自己取锁的入口"
  （判据 `the_transaction_chain_never_calls_a_command_shell`）。
- 命令：只读的、以及碰网络 / Keychain 的一律 `#[tauri::command(async)]`
  （判据 `read_commands_are_async_so_they_never_freeze_the_window` 的 READ + IO 两张单子）。
- 平台 HTTP 一律走 `platform::agent()`；二进制上传另给超时口径，别拿 30s 当够用。
- 网络只住 `runtime/net.rs` 与 `workbench/app/platform/`；`src/api/` 不许 fetch。
- 写盘一律 `fsx::atomic_write`；坏档报 `CORRUPTED`，不静默。
- **台账与代码同一 commit**；`main` 的提交 / 推送要 `ALLOW_COMMIT_ON_MAIN=1` / `ALLOW_PUSH_MAIN=1`；
  **tag 打在 main 的 tip 上**（squash 会重写提交，分支上打的 tag 指向不在 main 历史里的提交）。
- 验收 = 验证集全绿 + 探针实机走查（截图落 `tmp-shots/`，用完关预览端口）；**合并留给人**。
