# AGENTS.md · 本仓铁律（AI 与人都要守）

> 只写**约束**。每一步怎么做、命令参数、失败怎么恢复看 `docs/PRESET-WORKFLOW.md`；
> 动手顺序看 `docs/WORK-STRATEGY.md`。三份不许各维护一套规则。

## 1. 权威源在 `presets/`

- 参数唯一来源是 `presets/` 下的源 TOML（机型 / 参数 / 套餐 / 资产）。
- `presets/delivery/` 是生成产物；`crates/preset/assets/presets/` 是入库产物真字节。
- 可以直接改源 TOML，不必先开工作台；改完走 §2。
- 只改产物不改源 ⇒ 下次生成会覆盖。`presets diff` 能看出"产物与源不同源"。

## 1.5 客户端存储

- 云端原文只存一份：`<appDataDir>/content/<sha256>.toml`，同 SHA 多路径共享。
- 当前目录是一张索引：`run/delivery-index.json` 记 `path → 手上这一版的 sha`；换代只换索引。
- **换代顺序是死的**：先入库 → 再换目录 → 最后换索引 + 按引用清理。换目录前先用
  `content::ensure_referenced` 把旧索引仍引用的版本补进 `content/`；补不进就停在旧目录
  （`E_FS_CONTENT_001`）。不许出现"新目录已生效、旧版本已丢"。
- "内容已在"只在同一条线、同一代里算数：换源 / 换代后必须真实下载一次才记进索引。
- 用户文件的初始镜像在 `baseline-local/`，与云端内容仓库是两件事。
- 显式下载永远联网，失败报错、不复用旧内容；校验 SHA 后才允许去重。
- 删除进回收站 `recovery/<id>/`（文件 + 镜像 + 来源账），恢复不自动设为使用中，
  同步移除 `run/provenance.json` 那一条。
- 清理按引用：当前索引、`history[].sha256`、回收记录、待处理更新引用的 SHA 不删；
  30 天或 20 份先到先清。不许为了目录干净整批删基线。

## 2. 验证

```text
presets check      # 结构 / 引用 / 交付约束
presets diff       # 盘上与"真生成会写成什么"的差异
presets generate   # 刷新交付产物（不推进发布序号）
presets verify     # catalog / manifest / 账本 / SHA 一致性
```

`diff` 有差异不算失败（退出码 0），但必须看过。

## 3. 发布元数据只能由命令产生

不许手改 `presets/delivery/catalog.json`、`manifest.json`、`publish-serial.json`，
也不许伪造 `revision` / `publishSerial` / `catalogId`。它们只由 `presets generate`（沿用）
与 `presets publish`（分配）产生。

## 3.5 三层生成物不许顺手带上

```text
presets/…（源）→ crates/preset/assets/presets/*（include_str! 进客户端）
              → presets/delivery/**（客户端 Bootstrap 读的那一份）
```

- 测试时生成过、不想发 ⇒ `git restore`，别提交。
- 真要发 ⇒ `npm run presets -- publish`（分配序号 + 目录与账本同一笔提交）。
- 闸：`npm run check:artifacts`（pre-push + CI）。正式发布只有 `presets publish` 一条路。

## 4. 序号只由发布推进

| 动作 | 推进 `publishSerial` |
|---|---|
| 改源 TOML / `presets generate` / `publish --dry-run` | 否 |
| `presets publish` | 是（唯一一处） |

"生成成功"≠"发布了"。不得因为检查通过、测试通过就升级为发布。

## 4.5 软件版本（客户端 `.app`）

| 动作 | 改版本号 | 打 tag |
|---|---|---|
| 改代码 / `npm run tauri -- build` | 否 | 否 |
| `npm run preview:build` | 是（预览序号 +1） | 否 |
| `npm run release` | 是（冻结） | 是 |

- 唯一真值是 `src-tauri/Cargo.toml` 的 `version`；`package.json` / `tauri.conf.json` /
  `Cargo.lock` 派生。`npm run version:check` 是闸。
- 版本号只有 `x.y.z` 与 `x.y.z-preview.N`（大小写归一成小写）。**没有 `-dev`。**
- 预览号只在**交付成功**后 +1；改代码、编译、打包都不动。`npm run version:ensure` 幂等，
  收敛到"下一个待交付的预览号"，发布后自动进下一版。
- **包里自报的号必须等于要发的号**。`APP_VERSION` 是编译期常量，改了号不重编包里还是旧号；
  发版内核构建后核对 `Info.plist`（`assert_artifact_speaks_version`），不等就停在"已构建"之前。
- 发版：`npm run release -- <号> "说明" --channel preview|stable --confirm`。已存在的 tag 拒绝。

## 4.6 客户端与工作台不同名、不同兜

- 工作台那份配置的产物叫 `mkp-workbench`（`mainBinaryName`），客户端叫 `mkp-support-ease`。
  两档 cargo 产物目录分开：客户端 `target/`，工作台 `target-workbench/`。
- 不许绕过 `npm run tauri:workbench:build` 手敲
  `tauri build --config src-tauri/tauri.workbench.conf.json`：工作台配置的 `app.windows`
  是空数组，那颗二进制装上去进程起得来、没有窗。
- `npm run tauri -- build` 开跑前两道闸：盘上 `target/release/mkp-support-ease` 带工作台标记
  ⇒ 拒；工作区脏（版本派生那四个文件除外）⇒ 拒。要发出去的包必须对应一个提交。

## 5. 发布前与发布时

- 发布只 stage 白名单路径；无关改动不许混进来。
- 先 `presets publish --dry-run`，把结果给人看，**等明确授权**再真发。
- 失败保留现场，按 `docs/PRESET-WORKFLOW.md` 处理；不许为"让它过去"而放宽闸门、改测试、
  手工补元数据。

## 5.5 真发布必须先问人

**未获当次明确许可，不得执行任何真发布。** 早先的授权、"检查全绿"都不算，每趟都要新问。

| | 做什么 | tag | 安装包 | 写 `release.json` |
|---|---|---|---|---|
| **数据发布** `presets publish` | 预设目录换代、序号 +1 | 否 | 否 | 否 |
| **软件发版** `npm run release` | 改号 → tag → 构建 → Release → 写清单 | 是 | 是 | 是 |

- 代码里的确认位：`TxOptions.confirmed` / `ReleaseOptions.confirmed` 默认 `false`，
  缺它内核零副作用拒绝；CLI 必须 `--confirm`；工作台那颗按钮 = 人当次点的确认。
- 打了 tag 就得让 `release.json` 同代（`assert_release_info_matches_tag`）。
- 想看会做什么用 `--dry-run`（零写入，不用问）。

## 6. 判定口径

```text
catalogId      → 是不是同一条发布线（本程序的 support-ease，不是任何厂商官方目录；
                 跨镜像共享：GitHub ↔ Gitee 换 host 不算换线）
ref（分支）     → 能不能比较：换分支各数各的号（判 elsewhere）
publishSerial  → 发布有没有往前走（单调，回滚也 +1）
文件 SHA-256   → 这一份内容一不一样
```

- 唯一实现 `runtime::local::based_on`；前端只摊开结论，不重算。
- `publishedAt` 与 `history[].at` 只做展示，不决定方向。
- 缺 `catalogId` / `publishSerial` / **来源身份** ⇒ 判 `unverified`：不声称有新版、不给更新按钮。
  来源记录为空时不许退回"只按 publishSerial 判"。
- **下载 ≠ 接受版本**，不许因为"点过下载"就建立基线。
- 来源身份在**下载落盘那一刻**写进文件尾部 `[mkp_source]`；复用缓存时不写。
  运行索引另记在 `run/provenance.json`（`catalogId/serial/revision/publishedAt`）。
- 界面上两个时间并排、各说一件事：「云端发布时刻」= 这一代目录实际发布的时刻（不代表内容变过）；
  「来源版本时间」= `history[].at`。缺记录各写「未知」，不许互相顶替，也不许拿 mtime 顶。

## 6.5 跑的是哪一版

- 判据只有一个：**读二进制里的构建指纹**。禁止说"你是不是没重新编译"。
- 工作台：`./target-workbench/debug/presets build`；客户端：启动日志第一行。
  输出 `档位 · 提交 · 编译于 …` —— `编译于` 是主程序 exe 的 mtime。
- **算不算重编只看 exe 的 mtime 变没变**；`Compiling …` / `Building [====]` 都不算。
- 两个 dev 入口（`tauri-dev.mjs` / `tauri-workbench.mjs`）开跑前打同样三行：
  `本次启动` / `这次要跑` / `✔ 编译完`，实现共用 `scripts/build-stamp.mjs`。
- 客户端产物在仓库根 `target/`，不是 `src-tauri/target/`（后者是死目录，别拿错）。
- 工作台 dev 带 `--no-watch`：改 Rust 必须手动重启。

## 7. 收工口径

达成 / 未达成 · 台账几条 · 下一步。

## 8. AI 怎么干活

### 8.1 缺字段就报错，不许兜底

少一个必填字段就报错，并附上读到的原件。不许拿默认值圆过去 —— 缺字段本身就是信息。
反例：`version` 读不到退回 `package.json`、`size` 读不到写 0、`channel` 认不出当正式版。

### 8.2 测试只跑改动相关的；全量要先问

- 只跑对应模块的 `cargo test <模块>`、改到的文件的 eslint / tsc、相关的那几条 vitest。
- 跑全量**先问**，并说清为什么非全量不可；跑完贴数字（`753 + 1271`），不要只说"绿了"。
- 版本 / 发布 / 更新链的改动、交付前最后一次，按 §2 走全量。

### 8.3 临时文件放 `.scratch/`

探针、日志、截图一律放仓库根 `.scratch/`（git 忽略）。做完主动问要不要删；
不往 `src/`、`scripts/`、`docs/` 里放。

### 8.4 注释只写现状，不写历史

注释答"现在为什么是这样"，不写"以前怎样、哪天谁改的、作者当时说了什么"。
历史在 `git log -p`。判据名与错误文案同理。
## 9. 界面文字

**文字是布局的补丁。** 判据只有一句：删掉它，用户会不会做错？不会就删。
写法细则与四语术语表见 `docs/COPY-STYLE.md`。

### 9.1 三条铁律

1. **一处只说一次**：行状态、详情、提示条说同一件事 ⇒ 只留用户做决定时看的那一处。
2. **说明只讲后果，不讲机制**：「删了会回到未下载、可以重新下载」是后果；
   「云端那份一直留着」「BBS 已同步到账号云端」是机制，只允许进折叠区或 tooltip。
3. **布局优先**：能用位置、分组、对齐说清的关系，不写句子。
   页脚枚举右键菜单、控件旁边堆小字，都是布局欠账 —— 记去布局轮，别拿文案补。

### 9.2 硬指标

| 位置 | 上限 |
| --- | --- |
| 主按钮 / 动作 | 4 字 |
| 列表状态词 | 8 字 |
| 行内副标题 | 12 字 |
| 成组的说明句 | 30 字（1 句） |
| 提示条 | 40 字（1 行） |
| 确认框正文 | 2 句 / 60 字 |
| tooltip | 50 字，只答「点了会怎样」 |

### 9.3 禁写

- **同义小字**：标签已经写「更新渠道」，旁边不再写「选择更新的发布渠道」。
  小字只留**后果**（切换后要点一次「检查更新」）或**单位/范围**。
- **破折号补解释**：`刷新云端目录 —— 只换目录，不下载任何预设`
- **括号补丁**：`（云端那份一直留着，不会丢什么）`
- **字段名当句子**：判定 / 基线 / 发布序号 / 补齐字段 / 来源发布线
- **元话术与安抚**：如实、不冒充、不会丢什么、一个字都没动、请耐心等待
- **同一句在行、详情、提示条各来一遍**

### 9.4 拦截

`npm run check:copy` 只查**描述位**（键以 `.note/.hint/.tip/.title/.desc/.subtitle` 结尾），
五条机械规则：破折号、括号补丁、超 40 字、安抚话术、内部说法。
已挂进 `npm run lint`，CI 的 lint 那一步就是它。

- 自己新写 / 改写的文案触发它 ⇒ 按 `docs/COPY-STYLE.md` 改。
- `scripts/copy-baseline.json` 是**欠账清单**，现在是空的。往里加一条等于给自己发白名单；
  只有存量确实这次不改时才 `node scripts/check-copy.mjs --update`，修好后重跑让它消失。

