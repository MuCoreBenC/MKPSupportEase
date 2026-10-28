# AGENTS.md · 本仓铁律（AI 与人都要守）

> 这里只写**约束**。每一步怎么做、命令参数、数据从哪来、失败怎么恢复，
> 一律看 `docs/PRESET-WORKFLOW.md` —— 两份**不许各自维护一套规则**。

## 1. 权威源在 `presets/`

- 参数的唯一来源是 `presets/` 下的源 TOML（机型 / 参数 / 套餐 / 资产）。
- `presets/delivery/` 是**生成产物**，`crates/preset/assets/presets/` 是入库产物真字节。
- **可以直接改源 TOML**，不必先开工作台。改完必须走下面的验证流程。
- 只改了 `presets/delivery/mkp/presets/*.toml` 却没改源 = 下次生成会把你的改动覆盖掉。
  `presets verify` / `presets diff` 能看出"产物与源不同源"。

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
- `npm run publish:presets` **已退役**（第二个生成出口、不写账本）。它会直接把话说明白。

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

## 6. 判定口径（别在别处另立一套）

「这份本地预设要不要更新」由三件事各答一问：

```text
catalogId      → 是不是同一条发布线（本程序自己的 `support-ease`，
                 **不是任何厂商的官方目录** —— 这套预设是第三方整理分发的）
publishSerial  → 发布有没有往前走（单调，回滚也 +1；从 1 开始是计数器，不是版本号）
文件 SHA-256   → 这一份内容一不一样
```

`publishedAt` 与血统里的 `# release_time` **只做展示**，不决定更新方向。
唯一实现在 `runtime::local::based_on`；前端只摊开结论，不重算。

老副本缺 `catalogId` / `publishSerial` ⇒ 判 `unverified`：不声称有新版、不给更新按钮。
**下载 ≠ 接受版本**，不许因为"用户点过下载"就替它建立基线。

## 7. 收工口径

达成 / 未达成 · 台账几条 · 下一步。验证集见 `docs/PRESET-WORKFLOW.md` §验收。
