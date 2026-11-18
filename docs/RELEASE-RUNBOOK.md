# 发版手册

> 2026-11-18 立。作者：「我以后能不能一句话，你就按照固定的流程做好？打包发布。」
>
> 这一页就是那句「一句话」对应的固定流程。**入口只有一个**，别自己拼 `git tag` /
> `gh release` / 手写清单 —— 2026-11-18 那趟就是这么漏掉清单与版本号的。

## 一句话入口

```bash
npm run ship -- --check                                  # 只看现在发到哪一步（零写入）
npm run ship -- --what local                             # 只出本地包（只有本机看得到）
npm run ship -- --what preview --confirm --note "改了什么"   # 预览通道
npm run ship -- --what stable  --confirm --note "改了什么"   # 正式通道（全量通知用户）
npm run ship -- --what presets                           # 只换预设目录，软件版本不动
```

`--what` 取自人当次那句话（AGENTS §5.5），**AI 不许自己判断该发哪一档**。

## 两条分岔：新版本 vs 补平台

| 情形 | 走哪条 | 行为 |
|---|---|---|
| **新版本**（tag 还不存在） | `npm run ship -- --what X --confirm` | 改号 → 提交 → PR → 等 CI → 合并 → 打 tag → 构建 → Release → 上传 → 写清单 → 开清单 PR |
| **补平台**（tag 已存在，某个平台缺包） | **工作台 →「发布软件版本」对话框** | 只往已有 tag 上补一个包 / 补 macOS 的 dmg，**不重打 tag、不开 PR、不动版本号** |

**为什么补平台不能用 CLI**：`release.mjs` 见到「tag 已存在」直接拒绝（「已发布的号永远不再用」），
而工作台那一侧有「平台补齐」分支。2026-11-18 试过 `ship --what preview`，它把号推进到
`-preview.3` 然后报「tag v0.0.11-preview.2 已经存在」—— 那条路只对**新版本**有效。

工作台那条路的前置闸里有一条会拦「不能在 main 上发版」，先切分支即可：

```bash
git switch -c chore/release-v<版本>
```

## 发完必须核对三格

```bash
npm run ship -- --check
```

```text
✓ tag v0.0.11-preview.2 在远端
✓ Release v0.0.11-preview.2 已建
✗ main 上的清单还是 0.0.10-preview.8
✗ 还没生效 —— 别以为发了
```

**第三格（main 上的清单）不等 = 没生效**。缺的那一步永远是**清单 PR**：

```bash
npm run pr -- --list          # 看有没有开着的清单 PR
```

合并它之后重跑 `--check`，三格全绿才算发完。

## CI 红了怎么定位（三步，别猜）

```bash
gh pr checks <n>                                    # 1. 哪个 job 红
RID=$(gh run list --branch <branch> --limit 5 --json databaseId,conclusion \
  -q '.[] | select(.conclusion=="failure") | .databaseId' | head -1)
gh api repos/<owner>/<repo>/actions/jobs/<job-id>/logs \
  | grep -E '##\[error\]|panicked at|test result: FAILED'   # 2. 失败点
gh api .../logs | grep -A 10 'panicked at'          # 3. 断言原文
```

`gh run view --log-failed` 在 run 未结束时会回「logs will be available when it is complete」，
那时直接用 job 级 API（上面第三条）。

## 三类最常红的（2026-11-18 全踩过）

**① clippy 版本漂移** —— 本地 clippy 不报的 lint，CI 的新版报。

已治：`rust-toolchain.toml` 锁 1.97.1，CI 用 `rustup show` 按它装。
**升版本要主动改那一行**，别让 CI 自己飘。

**② 本地口径小于 CI** —— `check:rust` 原来只查一个包。

已治：`check:rust` = fmt + clippy 整仓 + clippy workbench，与 CI 两条一字不差。
推前闸（`scripts/hooks/pre-push` 闸⑩）跑的就是它 —— **推不上去就是 CI 会红**。

**③ 判据资产过期** —— 改了默认值 / 后处理行为之后没重派生。

```bash
# 配置 fixture（IR 派生）
cargo test -p mkpse-postprocess --test pipeline -- --ignored regenerate_config_fixture_from_ir_json

# 整链基线
cargo test -p mkpse-postprocess --test end_to_end -- --ignored regenerate_the_chain_baseline

# 黄金清单是硬编码的：上面两步之后必须手改 sha256 与字节数
#   crates/postprocess/tests/golden_integrity.rs 的 VENDORED 数组
shasum -a 256 crates/postprocess/tests/golden/<文件> && wc -c crates/postprocess/tests/golden/<文件>
```

改 `presets/client-defaults.toml` 一定会触发第三条 —— 那是 `PrefsIr::default()` 的源。

## 提交与 PR

```bash
git add -A && git commit -F - <<'MSG'
...
MSG
git push
npm run pr                    # 只走这一个入口，别直接 gh pr create（AGENTS §5.6）
```

PR 标题必须等于最新提交主题（`scripts/pr.mjs` 会拦），只允许 squash。

## 一轮完整的时间预算

| 阶段 | 耗时 |
|---|---|
| 推前闸（fmt + clippy 整仓） | 30–60s |
| CI（web / rust / rust-windows / windows-nsis） | 12–20 分钟 |
| 合并 + 构建 + 上传 | 5–10 分钟 |

`rust` / `rust-windows` 是最慢的两个（全量测试），`windows-nsis` 约 8 分钟。
**别在 CI 期间反复 sleep 轮询**，用 `gh pr checks <n>` 看一次就够。
