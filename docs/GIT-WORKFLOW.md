# git 工作流

> 工程约束见 `docs/ARCHITECTURE.md`，产品规则见 `docs/PRESET-PRODUCT-RULES.md`。

一句话：**main 只能由 PR 推进，任何人（包括你自己）都不直接 push main。**

两层拦截，各管一件事：

- **第一层（本地 hook）拦手滑** —— 八道闸，判据写在 `scripts/hooks/` 里；
- **第二层（GitHub ruleset）拦绕过** —— `--no-verify` 能绕过全部本地钩子且连日志都不留（git 的设计如此），
  换台机器 clone 没跑过 `npm install` 也等于没闸。所以服务端必须有一层。

---

## 1. 本地八道闸

装法：`npm install` 会触发 `prepare` → `scripts/setup-hooks.mjs`，把 `core.hooksPath` 指到
版本化的 `scripts/hooks`。**不是复制进 `.git/hooks`** —— 复制是单向快照，改了判据不重装就等于没改。

| 闸 | 钩子 | 拦什么 | 逃生开关 |
| --- | --- | --- | --- |
| ① | `pre-commit` | 在 main 上直接提交 | `ALLOW_COMMIT_ON_MAIN=1` |
| ①b | `pre-merge-commit` | 手工合并进 main | `ALLOW_COMMIT_ON_MAIN=1` |
| ② | `pre-push` | **对 main 的任何 push** | `ALLOW_PUSH_MAIN=1` |
| ③ | `pre-push` | 非快进（force）推送 | `ALLOW_FORCE_PUSH=1` |
| ④ | `pre-push` | 删除远端的 `main` 或 `v*` tag（普通分支不拦） | `ALLOW_DELETE_REMOTE=1` |
| ⑤ | `pre-push` | tag 名与 `package.json` 不符 / 是轻量 tag / 不在 main 上 | `ALLOW_TAG_MISMATCH=1` |
| ⑥ | `pre-commit` | 分支名没有 `feat/ fix/ refactor/ chore/ docs/ style/` 前缀 | `ALLOW_ANY_BRANCH=1` |
| ⑦ | `pre-commit` | `.env*`、私钥（`*.pem` / `*.p12` / `id_rsa*`）、>2MB 的非 `public/` 文件 | `ALLOW_BIG_OR_SECRET=1` |

几条容易踩的细节：

- **①b 是实测出来的必需品**：git 2.53 实测 `git merge --no-ff` **完全不调用** `pre-commit`
  —— 放一个 `exit 1` 的 `pre-commit` 进去，merge 照样成功。只有 `pre-merge-commit` 拦得住。
- **闸②在本仓库是无条件拦**，与试验场不同。那边的判据是"main 的 tip 有没有对应版本的 tag"，
  因为那边 main 由本地合并推进。这里 main 的推进权整个交给了 GitHub。
- **逃生开关只放掉它自己那一道**。`ALLOW_COMMIT_ON_MAIN=1` 不会连带放过闸⑦。
- **闸④只管 main 与 tag。** 一开始它拦所有远端删除（照试验场抄的），结果删一条已合并的
  feature 分支都要动用 `ALLOW_DELETE_REMOTE=1`。而 main 与 `v*` tag 在服务端 ruleset 里
  本来就禁删 —— 闸④对它们是第二道，对普通分支则是纯摩擦。**逃生开关变成日常工具的那一刻，
  `bypass.log` 就失去了信号价值**，所以把它收窄到该管的两处。
- **留痕延后到"提交确定发生"之后**才写 `.git/bypass.log`。闸①放行、闸⑦拦下时提交并没有发生，
  那种情况下写一行就是假记录 —— 而这份日志的全部价值在于每条都是真的（发版时要逐条念）。
- `.env.example` / `.env.sample` / `.env.template` 不拦：它们按约定只有键名没有值，
  为它们留 bypass 记录会稀释日志信号。`public/` 下的大文件也不拦 —— 3D 资产天生就大。

### 闸门失效是静默的

试验场（`mkp-adaptive-console`）那七道闸**从来没有生效过**：四个钩子文件在磁盘和 git 索引里
都是 `100644`（不可执行），git 遇到不可执行的钩子只打一行 hint 就照常提交。
`core.hooksPath` 指对了、文件也齐、自检脚本报了一串 ok —— 因为它只查文件存在、不查可执行位。

所以 `setup-hooks.mjs` 现在做两件额外的事：自动修磁盘上的可执行位，并检查 **git 索引里的 mode**
（索引里那份才是跟着 clone 走的）。索引里是 `100644` 就显眼报出来。

---

## 2. 服务端 ruleset

脚本化在 `scripts/setup-ruleset.mjs`（规则本身进版本控制，不靠"我记得在网页上勾过"），
跑 `node scripts/setup-ruleset.mjs` 建或更新，幂等。

**main（`main-pr-only`）**

| 规则 | 值 |
| --- | --- |
| Require a pull request | 开，approvals = 0（单人项目要的是"必须走 PR"这个动作，不是审批） |
| Require conversation resolution | 开 |
| Require linear history | 开（配 squash 合并） |
| Block force pushes | 开 |
| Restrict deletions | 开 |
| Required status checks | `web`、`rust` 两个 job |
| **Do not allow bypassing** | **开** —— `bypass_actors` 是空数组，仓库 admin 也绕不过 |

**tag（`tags-v-no-delete`）**：`refs/tags/v*` 只做 restrict deletions。
"tag 必须是 annotated、名字要对得上版本号"这类判据服务端没有对应规则，留在本地闸⑤ ——
两层各管自己管得住的那部分，不假装覆盖。

> **仓库是 public 的，这是被迫的选择。** ruleset API 对 private 仓库要求 GitHub Pro：
> 实测 `GET /repos/:o/:r/rulesets` 直接 403（`Upgrade to GitHub Pro or make this repository
> public to enable this feature.`）。private + Free 下第二层根本不存在，而"只有本地那层等于没拦住"。
> 由此产生一条常驻约束：**任何密钥、用户数据、私有资产都不能进这个仓库** ——
> 闸⑦从"防手滑"升级成唯一的防线。

---

## 3. 开局悖论怎么破

"第一个提交之前闸门就该生效"与"闸门本身是代码、得先提交"是一对矛盾。解法是把开仓阶段
显式记为例外，而不是事后补：

```text
1. git init -b main
2. 拷 scripts/hooks/ 与 setup-hooks.mjs（此时还没提交）
3. npm install → prepare 装闸门（core.hooksPath 指过去，chmod +x）
4. ALLOW_COMMIT_ON_MAIN=1 git commit --allow-empty -m 'chore: 开仓'   ← 第 1 条 bypass
5. gh repo create + ALLOW_PUSH_MAIN=1 git push -u origin main         ← 第 2 条 bypass
6. 立刻建 main 的 ruleset（main 必须已存在，ruleset 才挂得上）
7. 之后所有内容 —— 包括骨架代码本身 —— 走 feat/ 分支 + PR 进 main
```

`.git/bypass.log` 里**只应该有这两条**。第三条出现就说明流程被绕了。

---

## 4. 日常

```bash
git switch -c feat/<something>
# 改、提交（闸①⑥⑦ 在守着）
git push -u origin HEAD
npm run pr              # 建 PR —— 标题取自最新提交
npm run pr -- --merge   # 等 CI 全绿后 squash 合并
```

**`gh pr create` 不许直接用。** 走 `npm run pr`（见 §4.2）。

**squash 会重写提交。** 合进 main 的那一个提交与分支上的任何提交都不是同一个 SHA ——
所以 tag 必须在 `git switch main && git pull` 之后、在 main 的 tip 上打，
在分支上打的 tag 指向的是一个不在 main 历史里的提交（闸⑤的"tag 必须在 main 上"就是拦这个）。

### 4.1 一次交付的最短路径（SOP）

判据只有一条：**push 次数决定总时长**。一次 push 一轮 CI，改一回就重跑一整轮
（`rust-windows` 实测 20 分钟）。所以先在本地把所有能红的跑完，再推。

```bash
# ① 本地把闸跑完（这一步省下的是整轮 CI，不是几分钟）
cargo test --workspace                              # 默认 feature
cargo test -p mkp-support-ease --features workbench # ★ 少了这条，CI 的 rust job 会红
cargo clippy --workspace --all-targets
cargo fmt --all --check
npm run lint
npm run presets -- check && npm run presets -- diff

# ② 提交（工作区必须干净，见 §4.3 的三道闸）
git add -A <路径>
git commit -m "..."

# ③ 要发数据就现在发 —— 它会自己 commit + push（见 §4.4）
npm run presets -- publish --dry-run     # 先给人看
npm run presets -- publish --confirm

# ④ 版本号 + 预览包
npm run version:ensure
git add version.toml package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock
git commit -m "chore(version): 进待交付的预览号 x.y.z-preview.N"
npm run tauri -- build
node scripts/local-release.mjs

# ⑤ 一次 push，然后建 PR / 合并
git push -u origin HEAD
npm run pr -- --merge
```

### 4.2 建 PR 的三条判据（`scripts/pr.mjs`）

直接 `gh pr create` 会踩三个坑，脚本把它们钉住了：

| 判据 | 规则 | 为什么 |
| --- | --- | --- |
| 分支 | `main` 不许；`chore/next-preview-*`、`chore/version-*`、`chore/next-dev-version` 不许 | 进号是本地动作，合并它等于把 main 的号往回拨 |
| 标题 | **必须等于最新提交的主题** | 只允许 squash ⇒ main 上那一条的标题就是 PR 标题 |
| 已有 PR | 标题对得上才复用；对不上就停 | 复用旧 PR 会让合并记录写着上一轮的内容 |

```bash
npm run pr                  # 建 / 校验
npm run pr -- --replace     # 标题对不上：关掉旧的开新的
npm run pr -- --merge       # 建 + 等 CI + squash 合并
npm run pr -- --list        # 看还有哪些 PR 开着（收旧 PR 用）
```

PR 正文由 `origin/main..HEAD` 的提交列表自动生成 —— 手写正文正是"标题与内容对不上"的来源。
判据本身有自检：`node scripts/check-pr.mjs`（在 `npm run lint` 里）。

### 4.3 三道会把时间翻倍的闸

| 闸 | 症状 | 处理 |
| --- | --- | --- |
| `tauri build` 拒脏工作区 | `✗ 不开工：工作区不干净（1 处） M version.toml` | 只放行 4 个派生文件，**`version.toml` 不在内** —— 先提交它 |
| `preview:build` 的第二次 | 号从 `preview.1` 自己变成 `preview.2` | 它读本地清单算"下一个待交付号"。**同一个号只编一次**；重编用 `npm run tauri -- build` + `node scripts/local-release.mjs` |
| `presets publish` 自己推送 | 分支上凭空多一个提交、CI 多跑一轮 | 见 §4.4，把它排在最后一次 push 之前 |

### 4.4 会自动提交 / 推送的三处

| 命令 | 提交 | 推送 | 建 PR |
| --- | --- | --- | --- |
| `presets publish --confirm` | 是 | **是** | 否（没配发布账户） |
| `preview:build` 的 `version:ensure` | 否 | 否 | 否 |
| `release` | 是 | 是 | 是 |

所以 `presets publish` **不能**当"攒批中的一步"：它推的那一下会把当前分支上
所有未推的提交一起带上去。要省轮次就把排在它后面的改动先做完，或者接受多一轮。

### 4.5 时间账（2026-11-12 实测）

| 环节 | 耗时 | 备注 |
| --- | --- | --- |
| `web` | 1m50s | 含 `lint` / `tsc` / `check:artifacts` |
| `rust` | 9m21s | 含 `--features workbench` 那一档 |
| `rust-windows` | 20m01s | **最长的那个，决定整轮** |
| `windows-build`（NSIS） | 6m11s | 只在 `feat/**` 与 tag 上触发 |
| 本地全量（增量） | 1–2m | 含 feature 那一档 |
| `tauri build`（增量） | 2–3m | |

`ci.yml` 对同分支连续 push 开了 `cancel-in-progress` —— 旧的会取消，但**新的一轮从头跑**。
另有变更分类：只动 `presets/delivery/` 或文档时跳过编译与测试（job 仍 success），
所以纯数据发布很便宜。

## 4b. 版本号什么时候才用

**开发期不发版。** 版本号停在 `0.0.x`，改动靠 PR 进 main，不逐次 bump、不逐次打 tag。

理由：版本号只有一个用途 —— **让人说清「我用的是哪一份代码」**。没有人拿这个包的阶段，
它是纯仪式。而还原点你本来就有：main 上每个 squash 提交都是一个，
`git switch -c fix/x <sha>` 就回去了。tag 只是给还原点起个好记的名字，不是还原能力本身。

**什么时候开始用：第一次把包交给任何人的时候** —— 包括拷一个 `.dmg` 给自己另一台机器。
从那一刻起规矩反过来：每个交出去的包都必须是 `npm run release` 发的，否则
「用户说的 0.2.0」和「你手里的 0.2.0」可能不是同一份代码。

**版本号住在四个地方**，`release.mjs` 一起改，别手工改其中任何一个：

| 文件 | 字段 |
| --- | --- |
| `package.json` | `version` |
| `src-tauri/Cargo.toml` | `[package] version` |
| `Cargo.lock`（仓库根） | 本 crate 那一条的 `version`（漏了它下次 cargo 会自己改写，工作区凭空变脏）。建 workspace 之后它在根上，不在 `src-tauri/` 里了 |
| `src-tauri/tauri.conf.json` | `version` |

---

## 4c. 换到另一台机器（Windows）

**先跑 `npm install`，再看它的输出。** 那个脚本会自检三件事，任何一条报警都别忽略：
钩子是否齐全、是否可执行、**是否 LF 换行**。

三个已知的坑：

1. **CRLF 会让闸门静默失效。** Git for Windows 默认 `core.autocrlf=true`，clone 时把 LF 换成
   CRLF；钩子是 `#!/bin/sh` 脚本，变成 CRLF 后 shebang 成了 `/bin/sh\r`，内核找不到解释器，
   **git 静默跳过钩子**。仓库根的 `.gitattributes` 已经把这几类文件钉成 `eol=lf`，
   `setup-hooks.mjs` 装钩子时还会再查一遍。
2. **`core.hooksPath` 是 per-clone 配置**，不跟着仓库走 —— 新机器上不跑 `npm install` 就没有闸门。
3. **CI 对 Windows 的覆盖是后加的。** `rust-windows` job 只跑 `clippy` 与 `cargo test`，
   不出包。Windows 侧的窗口外观（`tauri.windows.conf.json` 的 `decorations: false` +
   `TopTabs` 自绘的三颗窗口键）**目前只有编译验证，没有人在真窗口里看过**。

工具链：Rust MSVC 工具链 + Visual Studio Build Tools（C++ 生成工具）+ WebView2
（Win11 自带）。装完 `npm run tauri dev`，首次冷编译几分钟是正常的。

---

## 5. 发版

**先跑 `npm run ship`，不要直接 `npm run release`。**

`ship` 用中文问「这次发什么」，四选一，把影响范围复述一遍再要你敲 yes，
发完用**可核对的事实**回答「到底生效没有」。

| 选 | 实际做什么 | 谁能看到 | tag |
| --- | --- | --- | --- |
| 1 只在本机看 | `tauri build` + `local-release` | 只有带 `--local-release` 的本地服务 | 无 |
| 2 发预览版 | `release --channel preview` | 主动点「检查更新」的人 | 有 |
| 3 发正式版 | `release --channel stable` | 所有客户端 | 有 |
| 4 只发预设数据 | `presets publish` | 所有人（下次检查预设时） | 无 |

### 最容易「以为发了其实没发」的地方

发版会写 `release.json` / `release-preview.json`，**那一笔要开第二个 PR**，
合并之后客户端才读得到（客户端只读 main）。所以「打了 tag」不等于「别人看得到」。

`ship` 结束时会核三件事，缺哪一步就用红字点名：

```
✓ tag v0.0.12-preview.1 在远端
✓ Release v0.0.12-preview.1 已建
✗ main 上的清单还是 0.0.11-preview.2
⇒ 还没生效 —— 别以为发了
```

随时回看现在发到哪一步：

```bash
npm run ship -- --check
```

### 人指挥 AI 发的时候

AI 只许跑 `npm run ship -- --what <local|preview|stable|presets>`，并且：

- `--what` **必须由人在当次对话里指定** —— AI 不得自己判断该发哪一档
  （「发预览版」和「发正式版」的差别是谁会收到通知，这不该由它替你想）；
- 发版那两档必须带 `--confirm`，代表人的当次授权（AGENTS §5.5）；
- 跑完必须把 `ship` 的核对结果原样贴出来，**尤其"还没生效"那一段**。

`ship` 自己会拦：版本号形状与通道不匹配（`-preview.N` 不能按正式版发）、
工作区脏（本地包那次）、没敲 yes / 没带 `--confirm`。

### `npm run release` 的细节

`npm run release`（`scripts/release.mjs`）是内核的壳，`ship` 最终调它。它串起：

1. 前置检查：不在 main 上、工作区干净、闸门已生效、`origin` 已配置、复述 `bypass.log` 里
   上次 tag 之后的绕过记录；
2. 校验链：`lint` → `tsc -b` → `build` → `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test`；
3. 问版本号与一句话说明，三处版本号一起改（`package.json`、`src-tauri/Cargo.toml`、
   `src-tauri/tauri.conf.json`），提交；
4. `git push -u origin HEAD` → `gh pr create`；
5. 轮询 CI，绿了才 `gh pr merge --squash --delete-branch`；
6. `git switch main && git pull` → 在 main 的 tip 打 annotated tag `v<ver>` → push tag。

任何一步失败就停住并打印精确的回退命令；破坏性命令只打印、不执行。
