# AGENTS.md · 本仓铁律（AI 与人都要守）

> 这里只写**约束**。每一步怎么做、命令参数、数据从哪来、失败怎么恢复，
> 一律看 `docs/PRESET-WORKFLOW.md` —— 两份**不许各自维护一套规则**。
> 动手顺序（先看链路、先圈范围、一次改完、集中验证）看 `docs/WORK-STRATEGY.md`。

## 1. 权威源在 `presets/`

- 参数的唯一来源是 `presets/` 下的源 TOML（机型 / 参数 / 套餐 / 资产）。
- `presets/delivery/` 是**生成产物**，`crates/preset/assets/presets/` 是入库产物真字节。
- **可以直接改源 TOML**，不必先开工作台。改完必须走下面的验证流程。
- 只改了 `presets/delivery/mkp/presets/*.toml` 却没改源 = 下次生成会把你的改动覆盖掉。
  `presets verify` / `presets diff` 能看出"产物与源不同源"。

## 1.5 客户端存储（2026-10-29 改判）

- **云端原文只存一份**：`<appDataDir>/content/<sha256>.toml`。同 SHA 多路径共享；不再写
  `delivery/<path>` 副本。
- **当前目录是一张索引**：`run/delivery-index.json` 记 `path → 手上这一版的 sha`；
  换代只换索引，不动内容，旧版因此不会丢。
- **换代顺序是死的**：`先入库 → 再换目录 → 最后换索引 + 按引用清理`。换目录之前先用
  `content::ensure_referenced` 把旧索引仍引用的版本从 `archive/` 补进 `content/`；
  补不进去就停在旧目录（`E_FS_CONTENT_001`），**不许**出现"新目录已生效、旧版本已丢"。
- **"内容已在"只在同一条线、同一代里算数**：换源 / 换代之后必须真实下载一次才记进索引，
  否则等于把"从 A 源下过"冒充成"B 源这一份已下"。
- **用户文件自己的初始镜像**在 `baseline-local/`，与云端内容仓库是两件事。
- **显式下载永远联网**：网络失败报错，不许复用旧内容；取回并校验 SHA 后才允许在
  `content/` 里去重。"磁盘上有同 SHA" ≠ "这次下载成功"。
- **删除进回收站**：`recovery/<id>/` 收文件 + 镜像 + 来源账；恢复不自动设为使用中。
  删除时同步移除 `run/provenance.json` 的那一条。
- **清理按引用**：当前索引、用户来源记录的 `history[].sha256`、回收记录、待处理更新仍引用的
  SHA 不许删；30 天或 20 份先到先清。不许为了目录干净整批删基线。

## 2. 改完必须验证，不许"改完就宣布完成"

```text
presets check      # 结构 / 引用 / 交付约束
presets diff       # 盘上与"真生成会写成什么"的差异
presets generate   # 需要时刷新交付产物（**不推进发布序号**）
presets verify     # catalog / manifest / 账本 / SHA 一致性
```

有差异不是失败（`diff` 退出码仍为 0），但**必须看过**再往下走。

## 3. 禁止手拼发布元数据

**不许**手工编辑：`presets/delivery/catalog.json`、`manifest.json`、
`publish-serial.json`（发布账本），也不许自己伪造 `revision` / `publishSerial` / `catalogId`。

这三个数是发布契约的一部分，只能由 `presets generate`（沿用）与
`presets publish`（分配）产生。

## 3.5 三层生成物都不许"顺手带上"

它们住在 git 里（客户端要按 URL 直接取），所以**必须靠纪律而不是靠 ignore**：

```text
presets/…（源）
  → crates/preset/assets/presets/*.toml + catalog.generated.json（include_str! 进客户端）
  → presets/delivery/**（客户端 Bootstrap 读的那一份）
```

- 测试时生成过、又不想发出去 ⇒ **还原**（`git restore`），别跟着提交。
- 真要发 ⇒ 走 `npm run presets -- publish`，它会分配序号并把目录与账本放进同一笔提交。
- 闸：`npm run check:artifacts`（pre-push + CI 各跑一次）。红了先看它给的两条路。
- `npm run publish:presets` **已删除**（2026-10-28）：它是第二个生成出口、且不写账本。
  正式发布只有 `npm run presets -- publish` 一条路。

## 4. 只有正式发布才推进序号

| 动作 | 推进 `publishSerial` |
|---|---|
| 改源 TOML | 否 |
| `presets generate` | **否** |
| `presets publish --dry-run` | 否（一个字节都不写） |
| `presets publish` | **是**（唯一一处） |

"生成成功"≠"发布了"。AI **不得**因为检查通过、测试通过就自动升级为发布。

## 5. 发布前

- 先看工作区有没有**无关改动**：发布只 stage 白名单里的路径，其余不许混进来。
- 先 `presets publish --dry-run`，把"会写成第几号、改了什么、闸怎么样"给人看，
  **等明确授权**再真发。
- 发布失败时**保留现场**，按 `docs/PRESET-WORKFLOW.md` 的恢复规则处理；
  不得为了"让它过去"而放宽闸门、改测试、手工补发布元数据。

## 5.5 真发布必须先问人（2026-11-05 作者定的硬约束）

**AI 未获用户"当次"明确许可，不得执行任何真发布。** 会话里早先的一句通用授权、
"检查全绿"、"测试通过"、"你之前说过可以"都**不算**许可；每一趟都要新问一次。

先分清是哪一种（两条独立事务，别混）：

| | 做什么 | tag | 安装包 | 写 `release.json` | 客户端看到什么 |
|---|---|---|---|---|---|
| **数据发布** `presets publish` | 交付预设目录换代、序号 +1 | 否 | 否 | 否 | 拉到目录后看到新预设 |
| **软件发版** `scripts/release.mjs` / `bin release` | 改版本号 → 打 tag → 构建 dmg/exe → 建 Release → 写 `release.json` | **是** | **是** | **是** | 检查更新时看到"有新版本" |

两条都要问，但**软件发版尤其不能自作主张**：它会让所有用户收到"有新版"，
是最难撤回的那一类动作。dev 版本（`0.0.10-dev` 之类）**不写 `release.json`**，
所以不发版就不会惊动用户 —— 这正是 dev 静默的道理。

**代码里的确认位**（不是只写在纸上）：

- `TxOptions.confirmed` / `ReleaseOptions.confirmed` 默认 `false`；
  真执行缺它 ⇒ 内核零副作用地拒绝（`INVALID_ARGUMENT`），预演 `dry_run` 不需要。
- 工作台那颗「确认发布」按钮 = 人当次点的确认，显式传 `true`。
- CLI / CI 必须显式带 `--confirm`：`presets publish --confirm`、
  `cargo run --bin release -- … --confirm`。`scripts/release.mjs` 非交互跑也必须带；
  交互跑要人亲口敲 `yes`。**只给了版本号和说明不算许可**。
- 想看会做什么 ⇒ 用 `--dry-run`，那趟一个字节都不写，不需要问人。

**打了 tag 就得让 `release.json` 同代**：版本号一变，信息源必须跟着变，否则
"我有新版、这里不记录，别人永远不知道"。`release_tx` 里有一道纯读判据盯着
（`assert_release_info_matches_tag`），版本与 tag 推不出彼此就红。

## 6. 判定口径（别在别处另立一套）

「这份本地预设要不要更新」由三件事各答一问：

```text
catalogId      → 是不是同一条发布线（本程序自己的 `support-ease`，
                 **不是任何厂商的官方目录** —— 这套预设是第三方整理分发的）
               ★ 跨镜像共享：GitHub ↔ Gitee 换 host 不算换线
ref（分支）     → 能不能比较：同仓库换分支 = 各数各的号，不比较（判 elsewhere）
publishSerial  → 发布有没有往前走（单调，回滚也 +1；从 1 开始是计数器，不是版本号）
文件 SHA-256   → 这一份内容一不一样
```

`publishedAt` 与来源记录里的 `history[].at` **只做展示**，不决定更新方向。
唯一实现在 `runtime::local::based_on`；前端只摊开结论，不重算。

老副本缺 `catalogId` / `publishSerial` ⇒ 判 `unverified`：不声称有新版、不给更新按钮。
**下载 ≠ 接受版本**，不许因为"用户点过下载"就替它建立基线。

**缺来源身份也判 `unverified`**（2026-10-29）：来源记录为空时**不许**"退回只按
`publishSerial` 判"—— 两个分支各数各的号，硬比出来的新旧是编的。来源身份在**下载落盘
那一刻**写进文件尾部 `[mkp_source]` 记录（`origin` + 履历 `history`），运行索引另记在 `run/provenance.json`（目录四格
`catalogId/serial/revision/publishedAt`）；盘上字节本就在、只是复用缓存时**不写**源身份
（那批字节可能是别的源取的）。界面上两个时间**并排、各说一件事**：「**云端发布时刻**」=
这一代目录**实际发布**的时刻（只说明目录什么时候发布，**不代表内容变过** —— 只盖身份的
发布一个字节都没改）；「**来源版本时间**」= 文件尾部来源记录里 `history[].at`（接受 / 下载那一刻写入），`# release_time` 只是配方自己的标记
（不保证等于文件生成 / 修改时间）。两格缺记录各写「未知」，不许互相顶替，也不许拿 mtime 顶。

## 6.5 「跑的是哪一版」不许靠猜（2026-11-02）

**禁止**对作者说「你是不是没重新编译」。这句话以前出现过多次，全部没有证据；
作者也明确说过他已经重编了。判断只有一个办法 —— **读二进制里的构建指纹**：

```text
./target-workbench/debug/presets build      # 工作台这一档
```

客户端那档**编不出 `presets`**（它挂着 `required-features = workbench`），所以它的指纹
只能读**启动日志第一行**（`构建指纹：client · <sha> · 编译于 …`）。

输出形如 `workbench · cb31830-dirty · 编译于 2026-11-03 14:56:12 +08:00`，三格各有各的意思：

| 格 | 只看它会怎样骗人 |
|---|---|
| `编译于 …` | 这是**主程序 exe 的 mtime**（链接完成那一刻）—— 既不是 build.rs 跑的时刻，也不是源码 mtime。它比源码新 ⇒ 这一次确实重编了；没变 ⇒ 这一趟没走到链接（报错 / 被打断） |
| `提交` | 带 `-dirty` = 编的时候工作区有未提交改动（改完没提交也能对上） |
| `档位` | 两档**共用一个可执行文件名**，光看进程名分不出客户端还是工作台 |

**「算不算重新编译」只有一个判据**：主程序 exe 的 mtime 变没变。命令行里出现
`Compiling …` / `Building [====] 450/452` **都不算** —— cargo 可以编完某个 bin 却不重链
主程序，也可以编到一半报错或被 Ctrl+C 掐掉，这两种情况下 exe 都还是旧的。

启动日志第一行也会打同一句（`构建指纹：…`）。**两个入口**开跑前都打同样三行
（客户端走 `scripts/tauri-dev.mjs`、工作台走 `scripts/tauri-workbench.mjs`，实现共用
`scripts/build-stamp.mjs`，两处不会各说各话）：`本次启动`（永远是当下）/
`这次要跑`（盘上那份是谁 + 源码比它新还是旧）/ `✔ 编译完`（**这一行才是本趟的编译时刻**；
等不到就直说「这一趟没有重编」）。

**注意三件事**：

- `target/` 与 `target-workbench/` 是**两份产物**，比较日期时别拿错目录；
- ★ **客户端的产物在仓库根 `target/`，不是 `src-tauri/target/`**。根 `Cargo.toml` 是
  workspace，`cargo metadata` 的 `target_directory` 就是仓库根。`src-tauri/target/` 是
  2026-09-21 留下的死目录（约 3.4GB）：2026-11-04 实测，`tauri dev` 不带
  `CARGO_TARGET_DIR` 时会去那里取 exe，而 cargo 编在根 `target/` —— 两边差六周，
  症状是「改了没反应」，不报任何错。`scripts/tauri-dev.mjs` 现在显式钉死同一个目录。
- `tauri-workbench.mjs` 给 dev 加了 `--no-watch`，所以**改 Rust 必须手动重启**；
  但「重启了没重编」与「重编了没重启」在界面上长得一样 —— 只有指纹能分开。

## 7. 收工口径

达成 / 未达成 · 台账几条 · 下一步。验证集见 `docs/PRESET-WORKFLOW.md` §验收。
