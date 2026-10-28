# 预设工作流：改 TOML → 生成 → 发布

> 铁律在 `AGENTS.md`（精简、必须遵守）。这里是**怎么做**：命令、参数、数据来源、
> 发布事务的状态顺序、失败恢复与验收。两份不同步时，以 `AGENTS.md` 的约束为准。

## 0. 三件事，别混

```text
源文件改动        presets/*.toml（人 / AI 直接改）
生成目录变化      presets generate —— 交付产物与 catalog 重算
正式发布          presets publish —— 唯一推进 publishSerial 的动作
```

改了源 TOML 而忘了生成，**不会**让程序宣称云端有新版本；生成完而没发布，
**不会**让任何客户端看到新版本。这三件事在盘上是三个不同的状态。

## 1. 命令

| 命令 | 做什么 | 写盘 | 推进序号 | 退出码 |
|---|---|---|---|---|
| `presets check` | 跑发布闸（结构 / 引用 / 交付约束） | 否 | 否 | 0 / 2 |
| `presets generate [--all]` | 跑生成器、重算 catalog | 是 | **否** | 0 / 2 |
| `presets diff` | 盘上 vs "真生成会写成什么" | 否 | 否 | 0（有差异也 0） |
| `presets verify` | catalog / manifest / 账本 / SHA 一致性 | 否 | 否 | 0 / 1 |
| `presets publish --dry-run` | 预演说会写成第几号 | **否** | 否 | 0 / 2 |
| `presets publish` | 正式发布事务 | 是 | **是** | 0 / 1 / 2 |
| `npm run check:artifacts` | 产物边界：开发产物有没有混进正式交付 | 否 | 否 | 0 / 1 |

- **stdout** 一行 JSON（脚本 / AI 解析），**stderr** 人话摘要。
- 退出码：`0` 成功 · `1` 执行失败或校验未通过 · `2` 闸门拦下（要人处理）。
- 所有命令的 JSON 都有 `command` / `ok` / `changed` / `summary` 四格，其余按命令追加。
- 实现：`src-tauri/src/bin/presets.rs` → `workbench::app` 里那几个自由函数。
  **与工作台同一段代码**，不是第二套。

```bash
cargo run --bin presets --features workbench -- check
cargo run --bin presets --features workbench -- generate
cargo run --bin presets --features workbench -- publish --dry-run
cargo run --bin presets --features workbench -- publish
```

## 2. 日常改动（只改参数，不发布）

```text
1. 改 presets/ 下的源 TOML（随便改，AI 也一样）
2. presets check        闸有没有红
3. presets diff         哪些产物会变
4. presets generate     需要时刷新（序号不动）
5. presets verify       盘上还自洽吗
```

第 4 步之后 `catalog.json` 的 `revision` 会变（内容变了），但
`publishSerial` **不变** —— 那正是"改了但没发"的盘上样子。

## 3. 发布事务的状态顺序

```text
⓪ 草稿干净？                       脏 ⇒ 拒（零副作用）
① 发布闸（audit_with）             红 ⇒ BlockedAudit，零写入
② 读"发布前"的目录指纹（只读）
③ --dry-run 在这里分流             ← 早于任何写盘；零写入、不分配序号
④ 生成（Scope::Stale）
⑤ 分配序号 ← 唯一分配点
   账本在 ⇒ max(账本.serial, 盘上 catalog.publishSerial) + 1
   账本不在 ⇒ 1（这条线第一次发布）
⑥ 发布身份闸（publish/identity）    换线 / 不前进 ⇒ 拒
⑦ 落地：manifest → catalog → source → **账本**（四份都在 stage 之前写完）
⑧ stage 白名单（presets/delivery/ 等）★目录与账本进同一笔 commit★
⑨ commit
⑩ push（失败 ⇒ 下一次重跑走"只补 push"）
⑪ PR / MR（已有回读容忍）
```

### 为什么要"账本与目录同一笔 commit"

一次发布写四份 JSON，它们不是一次原子提交。真正的原子性由 git 给：
四份都在 `presets/delivery/` 下，一次 stage 收进同一笔 commit。
于是"目录说 43 号、账本说 42 号"这种半截状态要么根本没提交，
要么被 `publish/identity` 拦住。

### 写盘顺序为什么是 manifest → catalog → source → 账本

`catalog.publishSerial` 是权威，账本只记"我发到几号了"。
所以恢复规则是**单向**的：盘上 catalog 更大 ⇒ 上一次写到一半断了 ⇒
下一号取 `max` 再 +1（那一号已经露出去了，绝不复用）。

## 4. 失败恢复

| 失败点 | 现场 | 重跑怎么办 |
|---|---|---|
| ③⑤ 中途写失败 | catalog 新的、账本旧的 | 正常跑：`max(账本, catalog) + 1`，回执会说上次有中断 |
| ⑥ 闸红 | 未提交，盘上脏 | 按闸的说明修；不许手改账本 |
| ⑧⑨ stage / commit 失败 | 未提交 | 生成是幂等的，重跑即可 |
| **⑩ push 失败** | **本地已有提交（含 N 号），远端没有** | 重跑 → 判据"HEAD 改过 catalog.json 且账本与盘上目录同代" ⇒ **只补一次 push**，不重新生成、不重新分配 |
| ⑪ PR 失败 | 已推送 | 回读那一份开着的 PR，不重复建 |

`presets verify` 会报"账本与目录不是同一代" —— 那就是半截发布的信号。

## 5. 判定口径（客户端侧）

```text
catalogId     发布线身份（本程序自己的线 `support-ease`；GitHub / Gitee 镜像因为
              同一笔提交而天然同 ID。**与切片器厂商的官方目录无关** —— 这套预设是
              第三方整理分发的，别把它写成 official）
publishSerial 单调序号（只增不减；回滚也 +1）。**从 1 开始**是计数器，不是版本号
文件 SHA-256  这一份内容
```

| 情形 | 结论 | 给更新按钮 |
|---|---|---|
| 同线、同号、同字节 | `current` | 否 |
| 同线、云端号更大、字节变了 | `outdated` | **是（唯一一档）** |
| 同线、云端号更大、字节没变 | `unchanged` | 否 |
| 同线、云端号更小 | `newer`（云端较旧） | 否 |
| 不同线 | `elsewhere` | 否 |
| 血统缺发布线 / 老目录没身份 / 来源不在目录 | `unverified` | 否 |
| 同线、同号、字节却变了 | `inconsistent`（报异常） | 否 |

`publishedAt` 与 `# release_time` 只做展示。唯一实现：`runtime::local::based_on`。

## 6. 基线规则（下载 ≠ 接受版本）

| 动作 | 血统 |
|---|---|
| 云端「下载」 | 写 `based_on` + `sha256` + 目录当刻的 `catalogId` / `serial`；**不代表已接受** |
| 逐参数「更新」全部处理完 | **唯一**把血统推到当前版的动作用；序号前进 |
| 部分更新（还有没表态的项） | 血统原样保留，序号**不前进**（仍显示有新版） |
| 编辑保存 | 保留血统，序号不动 |
| 撞名导入 / 另存 | 新文件按"下载"规则建血统，旧文件一个字节不动 |
| 老血统（无发布线） | 不写、不猜 ⇒ `unverified` |

## 7. 产物边界（开发时别把生成物带进正式交付）

三层生成物都在 git 里（客户端要按 URL 直接取），所以 `.gitignore` 挡不住它们：

```text
presets/…（源）
  → crates/preset/assets/presets/*.toml、src-tauri/src/runtime/catalog.generated.json
      （include_str! 编进客户端：随包预设解析的就是它）
  → presets/delivery/**（catalog / manifest / 9 份产物；客户端 Bootstrap 读它）
```

`npm run check:artifacts` 执行三条判据，**pre-push 与 CI 各一次**（`--no-verify` 绕不过远端）：

| 判据 | 红了说明 |
|---|---|
| R1 交付目录变了、`publish-serial.json` 没变 | 这批产物**不是发布产生的**（正式发布必写账本） |
| R2 内核产物变了、`presets/` 源没变 | 产物无源，几乎必然是测试时重生成留下的 |
| R3 本该被忽略的路径进了变更 | `dist/`、沙箱、临时脚本被 `git add -f` 强加了 |

**它抓不到**"源改了但忘了还原"—— 那是 diff 里一处正常改动，机器分不出有意还是遗忘。
那件事要靠人看，或者用 `presets diff` 把改动摆出来。

逃生口 `ALLOW_PRESET_ARTIFACTS=1`（只放掉这一道，会写进 CI 日志）。
本地只跑某个范围：`npm run check:artifacts -- --base origin/main` 或 `-- --staged`。

### 老发布链已退役

`npm run publish:presets`（`scripts/publish-presets.mjs`）是**第二个生成出口**，它在入口
直接拒绝并指向 `npm run presets -- publish`。两个理由：

- 它**不生成**，只提交 `presets/delivery/` 的现存字节（测试残留会被原样发出去）；
- 它**不写发布账本**，发出去的目录没有 `catalogId` / `publishSerial`，客户端会判
  `unverified` —— 谁都不提示更新。

确实要临时用它：`ALLOW_LEGACY_PRESET_PUBLISH=1`（会留痕）。

## 8. 验收

```bash
cargo fmt --all --check
cargo clippy --all-targets --features workbench -- -D warnings
cargo test --lib
cargo test --features workbench --lib
npm run build && npm run check:i18n && npm run lint && npx tsc -b --force
npm run check:artifacts          # 产物边界（CI 同一条）
```

故障注入判据在 `src-tauri/src/workbench/app/fault_tests.rs`（F1–F8），
判定矩阵在 `src-tauri/src/runtime/local.rs` 的 `based_on` 测试组。
