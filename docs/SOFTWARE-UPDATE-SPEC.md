# 软件更新：一份快照、一套事务、所有 UI 只读

> 只回答一件事：**「有没有新版、这一版要不要紧、上次装成没成」在哪算、谁说、谁存**。
> 唯一事实源是 `run/update-state.json`（`src-tauri/src/runtime/update_state.rs`）；
> 唯一写入路径是 `runtime::update_check` 的四个事务；界面只读
> `src/app/shared/softwareUpdate.ts` 那一份快照。

## 1. 三件事，三个落点

| 事实 | 存在哪儿 | 谁写 | 生命周期 |
|---|---|---|---|
| 候选 / 检查记录 / 忽略 / 上次安装结果 | `run/update-state.json` | 检查、忽略、作废、启动迁移四个事务 | 跨重启 |
| 下载进度（downloading / paused / ready / failed） | 进程内存 `updater::session()` | 下载循环 | **重启归零**（不许恢复成"还在下载"） |
| 旧 `app-state.json` 的 `pendingUpdate`、`run/update-result.json` | 只服务一次性迁移 | 旧版本写的 | 迁完即删 |

`install` 在界面上只有两档：`idle` / `ready { version }`。**只有 `ready` 跨进业务语义**，
其余都是进程态；重启后 `ready` 也回 `idle`（暂存路径不跨进程）。

## 2. 用户故事

**S1 · 自动发现**
我开着「自动检查更新」。某次拉云端目录成功之后，后端跑一次**检查事务**：
查到比当前新的版本就整份替换候选、`checked.ok = true`，广播整份快照
（`software-update-state`）；标题栏出现「可更新」，右上角弹一条 8 秒通知。
通知由快照里「候选从无到有」派生，**不是第二个事件**。

**S2 · 点开**
点胶囊展开小面板，写着「发现 {版本}」：
- **查看**：跳到设置页「软件更新」，由我看说明再决定；
- **忽略**：调 `ignore_update`，**落盘**进快照的 `ignored`；胶囊与通知都不再为这一版出现。

**S3 · 关掉自动检查**
后端写开关的同一次调用里跑 `invalidate`：清候选、检查记录回到「没查过」。
**不清 `ignored`** —— 用户对某一版的忽略不因为关开关就失效。

**S4 · 换渠道**
同上：旧候选作废（否则会出现"切到正式版还挂着测试版的更新"），下一次检查按新渠道重来。

**S5 · 开始下载**
`start_update` **从快照的候选里挑资产**，不再联网重算一遍：用户点的是哪张卡，
下的就是那一版。进度由 `software-update-progress` 推（进程态）。

**S5.5 · 下载源**
`release.json` 的 `assets[]` 每份资产带 `sources.gitee` 与 `sources.github`，
客户端按当前源换 `url`。两边的文件名、大小、SHA-256 必须一致，下载后照旧校验。

**S6 · 复查没有更新**
检查事务成功、候选为空：整份替换成空数组，`checked.ok = true`。胶囊消失。
**失败**（离线 / 没配源 / 远端没部署）：只写 `checked.ok = false` + 原因，
**候选一个不动** —— 查不到不等于没有新版，界面也不许把旧候选说成"刚查到的"。

**S7 · 装完重启**
安装脚本写的 `run/update-result.json` 在下次启动时被 `startup` 迁进快照，
然后删掉那个文件；`reconcile(current)` 把低于当前版本的候选、忽略记录、安装结果
一次性作废。**胶囊与「上次更新失败」都不再出现** —— 不依赖自动检查是否成功。

## 3. 快照与派生（唯一判据）

```text
UpdateSnapshot { current, checked{at,ok,reason,trigger,channel},
                 candidates[], ignored, last_result }
```

```text
has_update     = candidates 非空
pending        = candidates 里**当前渠道**的第一条，且 version != ignored
installable    = 进程态 ready.version == 候选里的某一版
```

界面不再自己判版本：后端把 `hasUpdate` / `pendingVersion` / `latestVersion` / `asset`
算好了放进 DTO，前端只摆。

## 4. 谁写、谁清

| 事务 | 触发 | 写什么 | 不写 |
|---|---|---|---|
| `run_check(trigger)` | 拉目录成功（auto）/「检查更新」（manual） | 整份 `candidates` + `checked` | 失败时不动候选 |
| `ignore(version)` | 胶囊「忽略」 | `ignored` | 不动候选 |
| `invalidate()` | 换渠道 / 关自动检查 | 清 `candidates`、`checked` 回 never | 不清 `ignored` |
| `startup()` | 启动 | `reconcile` + 迁移旧账 | 不联网、不发事件 |
| 下载/安装 | 用户手势 | 进程态（`UpdateState`） | 不写快照 |

`reconcile(current)` 是唯一的失效出口：装上新版之后，旧候选 / 旧安装结果 / 旧忽略记录
一起作废。版本序复用 `release_info::is_newer_version`，不另写一套。

## 5. 迁移（一次性、幂等）

| 旧数据 | 迁进 | 什么时候删 |
|---|---|---|
| `app-state.json.pendingUpdate` | 比当前新才进 `candidates` | 新快照**落盘成功之后**清该格 |
| `run/update-result.json` | 不低于当前才进 `last_result` | 同上，随后删文件 |
| `localStorage['mkp.A40.update.ignoredVersion']` | `ignored` | 前端收到 `migrate_update_ignored` **成功返回**之后删键 |

localStorage 只能由前端读：前端先读旧键，把值交给后端，后端写盘成功才返回 `Ok`，
前端拿到 `Ok` 才 `removeItem`。任意一步中断，下次启动重来一遍 —— 不会丢忽略记录。

坏档 / 版本格式认不出：**不迁**（跳过该条），其余照常；读不出候选不等于失败。

## 6. 事件

| 事件 | 载荷 | 谁发 |
|---|---|---|
| `software-update-state` | 整份业务快照（`UpdateSnapshot`） | `run_check`（成功与失败都发） |
| `software-update-progress` | 下载进度（`UpdateState`） | 下载循环 |

没有 `software-update-available`：通知由前端按快照里的候选派生（`onNewPending`）。

## 7. 测试

| 判据 | 位置 |
|---|---|
| `reconcile` 清旧候选 / 留新候选 / 清旧结果 / 清旧忽略 | `runtime::update_state::tests` |
| 只有 `ready` 跨进快照；下载中/暂停不算可安装 | `runtime::update_state::tests` |
| 检查成功替换候选；失败保留候选且 `ok=false` | `runtime::update_check::tests` |
| 启动迁移旧账、搬完才删、可重跑 | `runtime::update_check::tests` |
| 快照与进程态的事件各一个 | `runtime::update_check`（源码判据） |
| 胶囊判据（进度 / 待更新 / 无） | `src/app/shared/softwareUpdate.test.ts` |
| 设置页与标题栏读同一份 | 同一快照类型（`UpdateInfo`），页面不再自己查 |

跑法：`cargo test runtime::update`、`npx vitest run src/app/shared`；全量按 `AGENTS.md` §8.2 先问。
