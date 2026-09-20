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
| ④ | `pre-push` | 删除远端引用 | `ALLOW_DELETE_REMOTE=1` |
| ⑤ | `pre-push` | tag 名与 `package.json` 不符 / 是轻量 tag / 不在 main 上 | `ALLOW_TAG_MISMATCH=1` |
| ⑥ | `pre-commit` | 分支名没有 `feat/ fix/ refactor/ chore/ docs/ style/` 前缀 | `ALLOW_ANY_BRANCH=1` |
| ⑦ | `pre-commit` | `.env*`、私钥（`*.pem` / `*.p12` / `id_rsa*`）、>2MB 的非 `public/` 文件 | `ALLOW_BIG_OR_SECRET=1` |

几条容易踩的细节：

- **①b 是实测出来的必需品**：git 2.53 实测 `git merge --no-ff` **完全不调用** `pre-commit`
  —— 放一个 `exit 1` 的 `pre-commit` 进去，merge 照样成功。只有 `pre-merge-commit` 拦得住。
- **闸②在本仓库是无条件拦**，与试验场不同。那边的判据是"main 的 tip 有没有对应版本的 tag"，
  因为那边 main 由本地合并推进。这里 main 的推进权整个交给了 GitHub。
- **逃生开关只放掉它自己那一道**。`ALLOW_COMMIT_ON_MAIN=1` 不会连带放过闸⑦。
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
gh pr create
# 等 CI（web + rust）绿
gh pr merge --squash --delete-branch
```

**squash 会重写提交。** 合进 main 的那一个提交与分支上的任何提交都不是同一个 SHA ——
所以 tag 必须在 `git switch main && git pull` 之后、在 main 的 tip 上打，
在分支上打的 tag 指向的是一个不在 main 历史里的提交（闸⑤的"tag 必须在 main 上"就是拦这个）。

## 5. 发版

一条命令：`npm run release`（`scripts/release.mjs`）。它串起：

1. 前置检查：不在 main 上、工作区干净、闸门已生效、`origin` 已配置、复述 `bypass.log` 里
   上次 tag 之后的绕过记录；
2. 校验链：`lint` → `tsc -b` → `build` → `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test`；
3. 问版本号与一句话说明，三处版本号一起改（`package.json`、`src-tauri/Cargo.toml`、
   `src-tauri/tauri.conf.json`），提交；
4. `git push -u origin HEAD` → `gh pr create`；
5. 轮询 CI，绿了才 `gh pr merge --squash --delete-branch`；
6. `git switch main && git pull` → 在 main 的 tip 打 annotated tag `v<ver>` → push tag。

任何一步失败就停住并打印精确的回退命令；破坏性命令只打印、不执行。
