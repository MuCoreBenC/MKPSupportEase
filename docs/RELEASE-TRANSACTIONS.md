# 发布事务规范：发布预设 ≠ 发布软件版本

> **这份文档是"发布"相关工作的固定合同。**
> 以后任何 AI / 人做发布相关的事，**先读这份**，不要重新猜我们的意图。
> 改它要作者点头 —— 它不改代码，但它决定代码怎么写。

## 0. 为什么要有这份文档

"发布"这一个词，曾经被三件**完全不同**的事共用，于是谁都看不懂工作台那颗按钮到底在干什么：

```text
① 提交代码            本地一个 commit
② 把变更合并进主线      PR → merge（数据/内容生效）
③ 把某个版本交付给用户   打 tag → Release → 安装包（软件生效）
```

三者不是一回事。**合并 ≠ 软件发布。** 这份文档把它们拆成**两层事务**，各自定死。

---

## 1. 两层事务

```text
                    SupportEase 发布
        ┌──────────────┴──────────────┐
        ↓                             ↓
   发布预设/数据                   发布软件版本
        │                             │
  审计 → 生成 → finalize          确认主线
  → commit → push → PR            → 确定版本号
  → CI → 等待合并                 → tag
  → 合并后数据生效                 → 构建安装包
                                  → GitHub/Gitee Release
                                  → 上传安装包
                                  → 更新 release.json
                                  → 客户端发现新版本
```

### 1.1 「发布预设」（唯一事务，已实现，工作台那颗「发布」）

**做什么**：把"当前准备好的预设 / 数据变更"安全地送进仓库，并走完 PR / CI，等人合并。

| 步 | 动作 | 产物 / 结果 | 失败怎么办 |
| --- | --- | --- | --- |
| 1 | **发布闸** `PublishAudit`（十五项） | 逐项结果；`can_publish` 是唯一判据 | 任一 Blocker 红 → **零写入**，停下 |
| 2 | **生成** | `dist/mkp/presets/*.toml`（B 类产物）+ 重算目录 | 任一项算不出来 → 整批不动 |
| 3 | **finalize（定稿）** | `catalog.json` / `manifest.json` / `source.json` | 有残留 → 一个字节都不写 |
| 3½ | **最终一致性核对**（`finalize_consistency`，2026-10-05 起） | 定稿刚写下的 manifest / catalog 对**真字节**逐条核对 + 无残留。**发布事务的最后一道安全检查，不是第二套闸** —— 刻意不含 ⑮ git/clean | 红 = 内部错误或并发改动 → Err 短路，**不 stage、不 commit** |
| 4 | **commit** | 只 stage 白名单（交付产物 + 台账） | 白名单外的东西进不去索引 |
| 5 | **push** | 分支上远端（自持 Token，不经用户 gh/git 登录态） | 认证失败 → 如实报，不重试到天荒地老 |
| 6 | **创建 PR / MR** | GitHub PR / Gitee MR；**这一支已有开着的 PR 时回读那一份**（发布**可重跑**，见下） | 建失败且没有开着的 → 停在"已推送"，如实说 |
| 7 | **CI** | 平台跑检查 | 红 → 如实报（工作台显示 CI 失败档） |
| 8 | **合并** | 人在**工作台回执屏**或平台网页显式点一次（工作台一律 **squash**） | 平台拒合 → 如实报 4xx，不掩盖 |
| 9 | **合并后数据生效** | 客户端下次检查更新即可取到 | —— |

★ **第 8 步的两个口径（作者 2026-10-04 拍）**：合并**一律 squash**；**不强制等 CI** ——
"CI 还没跑完 / 已经红了"要在二次确认里**说清楚**（`CI 尚未完成 —— 确定继续合并吗？`），
决定权在人。工作台**不做自动合并**，也不替人判断"该不该合"：它只把"现在是什么状态"摆出来。

★ **第 6 步的"可重跑"**：平台不许同一个 head→base 开两份 PR —— 在合并之前再点一次「发布」，
`create_review` 会落 422（"这一支已经有一份"）。**那不是失败**：产物已经推上去了，人只是又发了一次。
回读**那一份开着的 PR** 继续（摘要如实写"回读那一份，没有重复建（平台原话：…）"），
找不到开着的才把原错误抛出来。

★ **第 9 步是"数据生效"，不是"软件发布"** —— 客户端本来就从预设仓库读数据，合并即生效，
**不产生新的 SupportEase 安装包**。

### 1.2 「发布软件版本」（另一套事务，**已实现** · 第四刀 2026-10-04）

**做什么**：产出一个**新的、可下载的 SupportEase 安装包**，让用户能升级。

**六条边界（作者 2026-10-04 一次裁完，写死在这里）**：

| 项 | 裁定 |
| --- | --- |
| `release.json` 落点 | **`presets/release.json`**（客户端 `source::release_url` 从文件下载根往上恰好一级，正指向它） |
| 版本真值 | **`src-tauri/Cargo.toml` 唯一**，其余全部派生（见下） |
| 安装包 | 第一阶段**只做 macOS** |
| 签名 | **暂不做**正式签名 / 公证（接受 Gatekeeper 首次打开警告，写进发布说明） |
| Release 形态 | 正常 **GitHub Release + `vX.Y.Z` tag + macOS 安装包** |
| 客户端 | 只负责**发现新版本 + 展示信息 + 打开下载页**，**不做应用内自动更新** |

> ★ **2026-10-07 更新（安装包那一行）**：不再"只做 macOS" —— `release_tx` 按**宿主平台**
> 分流（macOS → dmg / Windows → NSIS `.exe`），两端各自在自己的机器上打，`installer_plan()`
> 是唯一判据。理由：dmg 要 `hdiutil`、NSIS 要 `makensis`，交叉打不是"多传一个参数"的事。
> 应用内更新的 `.app.zip` **仍只有 macOS 有**（客户端那条替换链替换的是 `.app`），
> Windows 上传完 `.exe` 后在 `release.json` 里不带 `asset`，客户端走"打开下载页"。
> 表里"GitHub Release"那一行也已由 M6 之后的几刀改成 **Gitee 发布通道**（总纲 §5-M6）。
>
> ★ **2026-10-07 再更新（一个版本可以有多个平台）**：判据从"tag 被占就不许发"改成
> **"这一版 × 本平台"**（`release_tx::release_slot`，闸与事务共用）：
> - tag 还没打 → 走完整流程（版本号 → PR → tag → 构建 → 上传）；
> - tag 打了、**本平台**的包还不在 Release 上 → **只补包**（不重打 tag、不开 PR、不动版本号）；
> - 本平台那一份已经在 → 换版本号。
>
> 补包那一趟**不许把别的平台的安装包抹掉**：`release.json` 的 `asset` 格按"同一版本就留住"
> （`release_info_text`），否则补一个 Windows 包会把 macOS 的应用内更新入口清掉。

**★ 版本号是单向派生，不是"四处都写着"**：

```text
src-tauri/Cargo.toml  [package].version            ← 唯一真值（人只改这一处）
    ├─ package.json / src-tauri/tauri.conf.json     （派生）
    ├─ Cargo.lock                                  （派生）
    └─ 构建期 env!("CARGO_PKG_VERSION") → APP_VERSION → 安装包版本 → tag → release.json
```

`workbench/app/version.rs` 是这条链的唯一实现（读真值 / 推派生 / 推进），
判据 `the_app_version_has_one_source_of_truth` 钉住"五处同源"。

**★ 与 `scripts/release.mjs` 的关系**：不废弃、不复制 —— **一个内核、两个入口**
（`workbench/app/release_tx::run`）：

```text
       发布软件版本事务核心（release_tx::run）
            │            └── CLI：src-tauri/src/bin/release.rs ← scripts/release.mjs（薄壳）
            └── 工作台：wb_release_preflight / wb_release_software
```

脚本保留它的三条纪律与"校验 + 提问 + 等 CI"那一半，内核负责"版本号 → tag →
构建 → Release → 上传 → 写 `release.json`"。

| 步 | 动作 | 说明 |
| --- | --- | --- |
| 1 | 确认主线 | 工作区干净、不在 main 上、tag 没被占（`release_tx` 的预检，**零写入**） |
| 2 | 确定版本号 | **改一处** `src-tauri/Cargo.toml`，三处派生自动跟随 |
| 3 | **tag** | `vX.Y.Z` 打在 **main 的 tip** 上（squash 会重写提交） |
| 4 | 构建安装包 | 本机 `npm run tauri -- build`（第一阶段 macOS / 不签名）；产物 `bundle/dmg` 下**唯一一份** |
| 5 | **Release** | 基于 tag 建 GitHub Release（`Hosting::create_release`） |
| 6 | 上传安装包 | 作为 Release asset（`Hosting::upload_asset`，流式 + 15 分钟上限） |
| 7 | 更新 `release.json` | **上传成功之后**才写 `presets/release.json`，**另开一支分支走 PR** |
| 8 | 客户端发现新版本 | 设置页「软件更新」显示「有新版本 SupportEase 0.0.2」+「查看更新」 |

★ 第 7 步那个 PR **由人合并** —— 只有合并进 `main`，`raw.githubusercontent.com` 才吐得出
新版本信息。**合并之前客户端看不到新版本**，这是刻意的（发布留给人，不自动生效）。

★ 这一层**与预设数据链完全分开**：
- `release.json` 不进 catalog / manifest，不在预设发布的 stage 白名单里
  （两条链各有各的白名单：`STAGE_ALLOWLIST` 与 `RELEASE_STAGE_ALLOWLIST`）；
- **发布预设永远不产生软件 Release**（改 TOML → 合并 → 客户端拿到，软件仍是 `0.0.1`）；
- 两本历史也分开：`publish-history.json` 与 `release-history.json`。

---

## 2. 变更分类：哪些改动走哪条链

### 2.1 只需要「发布预设」（**不需要**软件版本）

```text
新增预设 / 修改预设参数 / 新增图片 / 修改机器数据 / 修改其他云端数据
```

这些是**数据**。客户端从预设仓库读，**合并即生效**，不产生新安装包。

### 2.2 必须发「软件版本」

```text
Rust / React 程序代码变化
IPC / API 契约变化
数据读取能力变化（结构签名变了、老客户端读不懂）
客户端 UI / 功能变化
```

这些是**程序**。用户机器上的旧 `.app` **不会**因为你 `main` 变了就自动变成新程序 ——
必须有一个新的可下载安装包。

---

## 3. 小改动怎么办（正式定死，不再每次讨论）

```text
数据小更新（改预设 / 加图片 / 改机型数据）
    ↓
发布预设即可 —— 不需要打 tag、不需要 Release

软件小修复（按钮点不动 / 某个页面崩 / 修一个 Rust bug）
    ↓
是否需要正式的软件版本？
    ├─ 不需要 → 走「持续构建 / Latest」渠道（覆盖最新安装包，用户可选更新）
    └─ 需要   → 走「发布软件版本」正式 Release
```

- **正式版本**（`v0.0.1` / `v0.0.2` …）：稳定、有发布说明、可回滚、有历史 —— 用户明确知道自己是什么版本。
- **Latest / 持续构建**：小修小补、不值得单独写发布说明 —— 覆盖式更新。
  ★ **不要把 Latest 混进正式版本历史**：它们是两栏，不是一栏。

**tag 什么时候打**：只在"要发一个正式软件版本"时打（§1.2 第 3 步）。
**小修改是否一定 Release**：不一定 —— 走 Latest 即可（上面那棵树）。

---

## 4. `release.json` 在这套里的位置

- 它是**软件版本**这条链的信息源（"有没有新版的 SupportEase 程序"），
  **住发布根 `presets/` 之外**，**不进** catalog / manifest，**不参与**发布闸的内容校验。
- 它由「发布软件版本」事务（§1.2 第 7 步）**更新**；「发布预设」事务**不碰它**。
- 客户端"能不能读这一代数据"（结构签名）与"有没有新版程序"（`release.json`）是**两条链**，
  **不许混成一句话**（见 `PUBLISH-ARCHITECTURE.md` §5.3.1）。

---

## 5. 预设仓库与软件仓库的关系

- **当前同仓**：预设数据在 `presets/`，软件代码在 `src-tauri/` + `src/`，同一个仓库。
- 将来可拆成两个仓库：**预留**——数据源地址（`source.json` 的 `baseUrl`）与软件发布信息
  （`release.json` 的地址）各自独立，客户端分别取；拆分时只改这**两处指向**，不动架构。

---

## 6. 状态模型 & 工作台显示什么

### 6.1 「发布预设」的状态链（工作台显示）

```text
工作台「发布回执」（PublishGateModal 的第二段视图 —— 发布完**就地**切过来，不再关框）

✓ 发布检查      （发布闸十五项全绿）
✓ 生成
✓ 提交  08ec040
✓ 推送  → feat/…
✓ PR #27        （或 Gitee MR；地址直接摆出来，可点交给系统浏览器）
… CI：运行中     （pending / passed / failed / none / unknown 五档）
· 合并           （【合并 #27】一颗按钮；点了先二次确认）
✓ 已合并
```

**发布历史**（`HistoryModal`）＝ 回执日志，住 `<appDataDir>/publish-history.json`：
一次事务一条（时间 / 阶段 / commit / PR 号与状态 / 产物份数 / 一句话结论），最新在前、最多 100 条。
**打开时读一次**；每条一个「刷新」手动回读那份 PR/MR 的当下状态 —— **不做轮询**（作者定死：
状态是"看一看"，不是常驻任务）。合并成功时后端把新状态**写回**那条记录（那是我们亲手造成的
状态变化，记账不必问网络）。

对应后端的 `publish_tx::PublishStage`（`blockedAudit` / `generated` / `committed` /
`pushed` / `reviewOpened` / `statusRead`）与平台无关的 `ReviewState`（`open` / `merged` /
`closed`）—— **前端不认识 GitHub / Gitee 的方言**（见 `PUBLISH-ARCHITECTURE.md` §7.1）。

### 6.2 「软件版本」的展示（当前只读）

```text
当前：已安装 0.0.1
       尚未有新的软件版本
```

**本轮不放「发布新版本」按钮** —— 摆一个点不动的假按钮比不摆更糟。功能做出来再加。

---

## 7. 什么条件下「发布」按钮允许点击

**唯一来源 = 发布闸 `can_publish`**（没有任何 Blocker 红）。**不许有第二条判据。**

★ `can_publish` **包含** ⑮ `git/clean` —— "除交付产物外工作区必须干净"。
这道保护**故意保留**：发布事务只该提交"这次该提交的那批"，把工作区里乱七八糟的改动
（比如 AI 顺手改的别的东西）一起卷进发布提交是事故。

★ **⑮ 的适用范围（作者 2026-10-04 裁决）**：发布事务的 Git 工作区检查**只在实际发布
工作区中承担发布阻断责任**；无 Git 工作区的纯 CI 单元测试环境，**不以 Skipped 表示失败，
也不伪造工作区状态** —— 那一格如实报 Pass（"当前执行环境不是可用的 Git 工作区，该检查
不适用"），其余十四项照常验证。

> 为什么要有这一句：CI 的 `pull_request` 是**游离 HEAD 检出**，取不到分支名 —— 老写法
> 走 `skip`，撞上「Blocker 不许 Skipped」那条总闸 ⇒ **PR 的 rust job 恒红**，而本机恒绿
> （2026-10-04 PR #27 实测）。"执行不了"要如实说，别让它冒充"跑过了"，也别让它冒充"干净"。

前端只是把这份判据**提前画在按钮上**（`preflight` 的阻断计数），后端在
`wb_generate` / `wb_publish` 开头**还会再查一遍** —— 前端不是第二条闸。

---

## 8. 与既有文档的分工

| 文档 | 管什么 | 本文与它 |
| --- | --- | --- |
| `RELEASE-TRANSACTIONS.md`（本文） | **两层事务的语义与边界**：谁触发、走什么链、变更怎么分类、状态怎么显示、按钮何时可点 | **合同**（语义层） |
| `PUBLISH-ARCHITECTURE.md` | **发布链的实现规则**：路径语义 / 发布闸十五项 / minVersion / release.json / 自持认证 | **实现根规则**（本文引用它） |
| `DATA-ARCHITECTURE.md` | 四层 / 四铁律 / 归属 | 最上游 |
| `HANDOFF.md` | 进度台账 | 只记"做到哪了" |

---

## 9. 一句话总结

> **发预设** = 把数据送进仓库、合并即生效，**不开新版本**。
> **发软件版本** = 打 tag、出安装包、更新 `release.json`，**用户才能升级**。
> 合并 ≠ 发布软件版本。两条链各走各的，**不许混**。

---

## 10. 自动镜像与验收（2026-11-05 追加）

**GitHub 是唯一开发源，Gitee 是只读镜像。** 代码合并只走 GitHub 的 PR；Gitee 不再开 PR、
不再重复审查。合并之后由 `.github/workflows/mirror-to-gitee.yml` 把 `main` 快进同步过去：
只推 `main` 一个 refspec、不推 tag、不推别的分支、不用 `--force`、不开 PR。

工作台里 `wb_merge_review` **不再自动推镜像**（回执如实显示「未同步 · 由自动流程负责」）。
旧能力保留成「手动补同步」按钮，只有人主动点才推，成功 / 已是同一版 / 未配置 / 失败
四种结果如实回显。正常同步状态以 GitHub Actions 为准。

### 10.1 一次性配置

1. 在 **Gitee** 生成一枚**专用**私人令牌（只给这个仓库的写权限）。
2. 在 **GitHub 仓库 → Settings → Secrets and variables → Actions → Secrets** 加两条：

   | Secret | 值 |
   | --- | --- |
   | `GITEE_USERNAME` | Gitee 账号名 |
   | `GITEE_TOKEN` | 上面那枚令牌 |

3. **不启用** Gitee 官方 Pull 镜像 —— 两套同步机制同时跑会互相覆盖。Actions 不稳时
   才考虑用它**替代**，不是并行。

> 凭据走 URL 内嵌（`https://user:token@gitee.com/...`），不是 `http.extraHeader` 的 Basic 头：
> `src-tauri/src/workbench/app/git.rs` 有 2026-10-06 真机实测，Gitee 的 git-http 不吃先发式
> Basic 头。工作流里令牌、编码后的令牌、整条带凭据的 URL 三者都 `::add-mask::`。
>
> 工作流**不跑 `npm ci`**：`scripts/setup-hooks.mjs` 会把 `core.hooksPath` 装成本地的
> `scripts/hooks`，其中 `pre-push` 闸②见到推 main 就拦。同步时显式
> `-c core.hooksPath=/dev/null`。

### 10.2 两种触发，别混

| 触发 | 凭据没配时 | 凭据齐了 |
| --- | --- | --- |
| `push` 到 `main`（合并之后自动跑） | **跳过**并打印原因，不红、不写远端 | 自动同步 |
| `workflow_dispatch`（人主动点） | **直接红**（手动触发就是明确授权） | 执行一次同步 |

所以「先把工作流合进 main」那一笔 push 会跳过 —— 这正是"配置阶段不偷偷写远端"的保证。
推送失败一律红；`up to date` 是幂等成功。

### 10.3 第一次补齐与验收

配好 Secret 之后**手动触发一次**（这一步才向 Gitee `main` 写入）：

```bash
gh workflow run mirror-to-gitee.yml --ref main
gh run list --workflow mirror-to-gitee.yml --limit 1
```

验收判据（两条都要成立）：

```bash
# ① 两端 main 指向同一个 commit
git ls-remote origin refs/heads/main
git ls-remote gitee  refs/heads/main

# ② Gitee 能读到这一代目录
curl -sL https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/catalog.json \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["revision"])'
# ⇒ dc0884e1b1b0c421（第 7 号）
```

2026-11-05 那一轮的基准：GitHub main `cbb80e0c3c94…`、第 7 号目录
`dc0884e1b1b0c421`；同步前 Gitee main 是 `3babb63…`（第 4 号 `c03b5304a766f173`）。
后续每一轮以「两端 `git ls-remote` 相同」为准，别写死 sha。

### 10.4 双通道清单（2026-11-05）

**语义先立住，命名押后**：`dev` 是作者本机自测用，绝不通知任何用户；对外只有
「可主动查看的预览版」与「会主动通知的正式版」。预览版发布后不主动通知普通用户，
只有用户点「重新检查」时才与正式版并列出现（正式版优先、颜色区分）。

| 文件 | 谁读它 | 什么时候读 |
| --- | --- | --- |
| `release.json` | 所有客户端（含 0.0.9 老版） | 自动通知 / 标题栏 / 主动检查 |
| `release-preview.json` | 只有带 `releasePreview` 解析分支的新客户端 | **仅**用户主动点「重新检查」 |

- `release.json` 形状、`version`、单 `asset` **一个字节都没动**（仍 `0.0.6`）；
  `source.json` 只加可选相对字段 `releasePreview`，`sourceSchema` 保持 2，
  `releaseSchema` 保持 1 —— 加字段不升代次。
- **老客户端结构上读不到预览清单**：0.0.9 的二进制里没有 `releasePreview`
  这一格与 `ResourceRef::ReleasePreview` 分支，永远不会发起那次请求。
  独立判据 `a_stable_0_0_9_is_never_notified_about_a_preview_release` 直接钉住
  自动通知链路，不靠版本号规则推断。
- 预览清单本轮是**占位**（版本与正式版相同 `0.0.6`、无 `asset`），
  界面不会凭空多出一张预览卡。预览版的生成与发布事务见 **§10.8**（2026-11-05 补齐）。
- 自动检查（拉目录成功后顺路）**一次网络请求都不新增**，仍只读正式清单；
  主动检查最多两次 GET（正式 + 预览）。预览清单缺失 / 解析失败不算故障，
  正式版结论照给。
- `start_update` 按用户点中的 `channel` 取资产；认不出的通道显式报错，
  不兜底成正式版 —— 否则「尝试测试版」会下成正式包。
- `scripts/check-release-source.mjs` 用同一条 Gitee 断代线检查两份清单的
  `url` / `asset.url` / `assets[].url`。

### 10.5 Windows 应用内安装（2026-11-05）

- `release.json` 可选 `assets[]`（`platform` / `kind` 标注），老的单 `asset`
  保留为 macOS 入口；客户端按当前平台挑，`kind = nsis` 的 exe **不解压**，
  验 size/sha 后直接进 `Ready`。
- 退出后由 PowerShell 帮手 `Start-Process … -ArgumentList '/S' -Wait`
  静默安装，写同一份 `run/update-result.json` 并拉起新 exe。
- 本机是 macOS，帮手脚本做成纯函数并单测
  （`the_windows_install_script_installs_silently_and_records`），
  **真机行为只承诺 Windows CI 编译通过**，未在本机验证过。
- 顺手修掉一个既有时序 bug：安装脚本 `sleep 1` 后才用暂存里的包，
  而 `install_update` 原来 200ms 后就 `cleanup_stage()` 把暂存删了 ——
  现在清理交给下一次 `download` 开头。

### 10.6 数据发布与代码发布跑的不是同一套闸（2026-11-05）

作者问过一句「为什么就发布一个预设，他就要走这么完整的 PR」——问得对。两者的
风险面不一样，闸也该不一样：

| 这次改了什么 | 必需检查实际做的事 |
| --- | --- |
| 只动 `presets/delivery/**`、`docs/**`、图片与 `*.md` | `web` 跑全套前端与交付闸；`rust` / `rust-windows` **只 checkout + 分类**就 success |
| 动了任何代码（`src/`、`src-tauri/`、`scripts/`、`.github/`、`Cargo.*` …） | 全套照跑（fmt / clippy ×2 / test ×2） |

- 分类住在 `ci.yml` 每个重 job 的第一步，比的是 `base...HEAD` 的文件清单。
- **不能用 workflow 级 `paths` 过滤**：那会让 workflow 不触发，而 `rust` /
  `rust-windows` 是 ruleset 的必需检查 —— 检查永远不出现，PR 永远合不了。
  所以是「job 跑起来、只是不做重活」。
- 拿不到基线（首次 push / 全 0 的 before）时**按代码变更处理**：宁可多跑，不放行。
- 交付闸一条都没省：`check:artifacts` 仍在 `web` 里跑，数据 PR 的
  「产物必须与账本同批」正是它在管。
- `windows-build.yml` 对交付 / 文档变更不触发（`paths-ignore`；它对 tag push
  不生效，所以发版那条路不受影响）。

### 10.7 失败与边界

- Actions 红了：先看 job 日志；**不要**改用本地 `git push gitee main`（PR-only 闸②会拦、
  服务端 ruleset 也拒），也不要用 `ALLOW_PUSH_MAIN=1` / `--no-verify` 绕。
- 需要立刻补一次又不想等 CI：用工作台「手动补同步」按钮，它同样会如实回报结果。
- 本机制**不碰** `release.json`、不打 tag、不建 Release、不跑 `presets publish`。
  软件发版那条链（§1 第二层）照旧。

### 10.8 预览版发布事务（2026-11-05）

预览版走的是**同一条内核**（`release_tx::run`），只多一个 `channel` 参数。不是第二套流程。

| | 正式通道（缺省） | 预览通道 `--channel preview` |
| --- | --- | --- |
| 版本号 | `x.y.z` | `x.y.z-preview.N`（N ≥ 1） |
| 写入清单 | `presets/delivery/release.json` | `presets/delivery/release-preview.json` |
| Release 标记 | 普通 | `prerelease: true` |
| 谁会知道 | 所有客户端（标题栏会提示） | **只有**主动点「检查更新」的人 |

一条命令（`MKPSE_APP_DIR` 指向工作台应用数据目录，复用发布账户与凭据）：

```bash
MKPSE_APP_DIR="$HOME/Library/Application Support/SupportEase-Workbench" \
  npm run release -- 0.0.10-preview.1 "试试双通道更新" --channel preview
```

底层等价：`cargo run --bin release --features workbench -- 0.0.10-preview.1 "…" --channel=preview --merge --confirm`

**五条硬约束**（都有判据钉着，不是口头约定）：

1. **版本形状按通道判**。正式通道只认 `x.y.z`；预览通道只认 `x.y.z-preview.N`。
   `-preview` / `-preview.0` / `-preview.01` 都不合法 —— 三种写法都会让"谁更新"说不清。
2. **预览不消耗正式号**。发过 `0.0.10-preview.1` 之后，`0.0.10` 仍可照常发；
   `version.mjs check` 只把**正式版** tag 当作"这个号用掉了"。
3. **发预览不动正式版那份清单**。`release.json` 一个字节都不改 ——
   判据 `a_preview_publish_never_touches_the_stable_manifest` 逐字比对盘上内容。
4. **通道与版本身份不许串台**。预览通道写正式号、正式通道写预览号，事务在写盘前红
   （`a_channel_never_writes_the_other_channels_version`）。理由是后果都很难撤：
   前者会让人把预览当正式候选，后者会让全量用户收到预览通知。
5. **重复发布是 no-op**。同一版同一说明重写一次，清单内容逐字相同，事务据此
   不切分支、不开 PR（`republishing_the_same_preview_version_is_a_no_op`）。

**重复发布与补包**：`release_slot` 按「tag × 本平台」判 —— 预览 tag 与正式 tag 走同一套。
已经发过本平台 = `Taken`（换号）；tag 在而本平台没包 = `Append`（只补包，不动版本号）。

**回滚的现实**：远端已经发生的事（tag、Release、上传的包）内核**不回滚** ——
它会如实报出停在哪一步，并把回退命令打印给人看。特别地，服务端 ruleset
`tags-v-no-delete` 生效在 `refs/tags/v*` 上：**预览 tag 推上去也删不掉**，
打错号只能往前发新号，不能收回。

**发完之后怎么验**：

```bash
# ① 正式版那一份没动
git show main:presets/delivery/release.json | grep version      # 应仍是上一版正式号
# ② 预览清单已更新（合并 PR 之后）
curl -sL https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/release-preview.json
# ③ 0.0.9 不被通知：release.json 的版本低于 0.0.9 ⇒ 自动通知链自然不响
```

**边界**：这一节只谈软件版本。预览发布**不碰** `presets/delivery/catalog.json` 与
`publish-serial.json`，不跑 `presets publish`，不推进预设数据序号。
