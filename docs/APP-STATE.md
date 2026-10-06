# AppState —— 应用持久化状态（架构决策，2026-10-06）

> 作者裁决 2026-10-06：**推翻 `runtime/state.rs` 模块头里「一种状态一个文件」那条规则**，
> 改为「一个应用一个 AppState，由 AppState 统一管理持久化状态」。
> 本文档是这次重定的地基；施工与后续评审都以这里为准。
> 起因是 A3 的根治验收（见 [`UX-FIX-PLAN-2026-10-06.md`](UX-FIX-PLAN-2026-10-06.md)）：
> 写链已收口而读链是「每页一份快照 + 回页签对账」—— 文件虽然只有一个真相，
> 整个应用里到处都是它的副本。继续在 `active-preset.json` 上补同步，是在旧地基上打补丁。

## 1. 一句话

**SupportEase 只有一个统一的「应用持久化状态」`AppState`。**
所有属于"应用当前状态、需要跨重启保存"的东西都由它统一管理；
读取、修改、保存全部经过唯一入口；保存统一走原子写入。

两个概念从一开始就分开，不许混着叫：

| 名字 | 是什么 |
| --- | --- |
| **AppState（应用持久化状态）** | 一个**状态模型**：应用现在记住什么 |
| **Atomic Write（原子写入）** | 一个**保存机制**：写 `app-state.json` 时先写临时文件再原子替换，要么完整成功、要么保持旧文件，绝不留半截 JSON |

"原子"不是 AppState 的名字，是它保存时的性质。（`fsx::atomic::atomic_write` 已经是全仓唯一写盘出口，这条不变，AppState 直接建在它上面。）

## 2. 状态文件

```text
run/app-state.json
```

代表"这个 SupportEase 应用当前记住的全部持久化状态"。形状（**只定骨架，不定死字段**
—— 真实施工时把现有三态逐项搬进来，字段以现有结构为准，不为架构决定发明字段）：

```json
{
  "appStateSchema": 1,

  "activePreset": { "origin": "…", "fileName": "…", "sha256": "…", "path": "…" },

  "draftPreset":  { "…": "编辑中的那一份（现 run/draft-preset.json 的内容）" },

  "presetSource": { "…": "数据源设置（现 run/preset-source.json 的内容）" }
}
```

规则：

- 顶层带 `appStateSchema` 代次；**每个 section 自带语义、加字段不升号，改语义才升**
  （与 catalog 同一条惯例，取代以前"每个文件各带 schema"的写法）；
- section 一律 `Option` + `serde(default)`：没有就是"这一格还没状态"，合法；将来加新
  section 时旧文件照读（缺省 None），不迁移、不升号；
- **AppState 只存指针与状态，不存内容**：`activePreset` 说的是"用哪一份"，预设正文仍在
  交付区 / `presets-mine/`；把 toml 正文塞进状态文件是错误设计。

## 3. AppState 的职责

1. 保存需要跨重启保留的应用状态；
2. 应用状态的**唯一读取入口**；
3. 应用状态的**唯一写入入口**；
4. 写入 = 读出 → 改这一格 → 整份原子写回（进程内互斥串行，杜绝两个写互相踩掉对方的格）；
5. 状态变化后统一**通知订阅者**（Rust 侧 emit `app-state-changed`，TS 侧唯一客户端接住再广播）；
6. 持久化统一原子写入；
7. 状态文件统一为 `run/app-state.json`。

## 4. 明确禁止

```text
❌ run/active-preset.json / run/draft-preset.json / run/preset-source.json（迁移完成后）
❌ 页面 / 组件 / 业务模块直接读写状态文件
❌ 页面自己维护持久化状态副本（Home 的 sel 反填快照、Presets 的 active state、Calib 的 selCache……）
❌ 某个 Rust command 绕过 AppState 模块偷偷改状态
❌ tab 切回来再「对账」
❌ 为了同步而增加各种 reload / refresh 补丁
```

纪律类比：AppState 是财务章 —— 谁都不能自己去改账，必须经过财务。
页面与业务模块的路径永远是：

```text
UI → 业务动作 → AppState API → （内存状态改变）→ 原子写入 → 通知订阅者
```

## 5. 边界：什么进 AppState，什么不进

AppState 放的是「**应用现在是什么状态**」，不是「应用所有数据是什么」。

**进**：当前使用哪个预设（activePreset）、当前编辑哪个预设（draftPreset）、预设源配置
（presetSource）、当前选择、应用设置、用户偏好 —— 一切"需要跨重启记住的当前状态"。

**不进**：预设 toml 正文、事件历史（`preset_events.json`）、出处账（`provenance.json`）、
下载文件、图片、模型、缓存、日志。那些是**业务数据 / 文件 / 历史记录**——各有各的落点
（`catalog.path` 语义、事件账、用户根），塞进状态文件只会养出一个几百 MB 的
`app-state.json`。

待定项（不随本轮搬）：`index/offsets.json`（校准偏移）。它介于"状态"与"业务数据"之间，
本轮不动它，等作者定性后再决定是否成为 AppState 的一格。

## 6. 迁移（一次性、惰性）

现有三态逐项搬入 `app-state.json`：

| 旧 | 新 |
| --- | --- |
| `run/active-preset.json` | `activePreset` 格 |
| `run/draft-preset.json` | `draftPreset` 格 |
| `run/preset-source.json` | `presetSource` 格 |

迁移规则：

- **读**：`app-state.json` 在 → 只信它；不在 → 按格读旧文件拼出快照（旧文件缺席 = None，
  坏档照旧报 `CORRUPTED`，坏档不静默不变）；
- **写**：第一次写时若 `app-state.json` 不在，先把旧文件的内容并进来一起写下去，
  写成功后删掉旧文件（各自幂等）—— 从这一刻起旧文件退役；
- 迁移不改任何语义：三个 section 的字段就是原文件的字段，原样搬家；
- 回滚兼容不在目标内：老二进制不认识 `app-state.json`，降级会退回旧文件 —— 接受。

## 7. 读写与订阅（这条直接解决 A3）

Rust 侧：唯一模块 `runtime/app_state.rs`。页面与命令**不得**另立状态读写；
现有散落的 `load_active / save_active / save_draft / set_source …` 全部收编为
AppState 的 section 操作。每个**写命令成功后 emit `app-state-changed`**。

TS 侧：唯一客户端（快照 + `subscribe` + `useSyncExternalStore` 钩子），全应用只有它调
`getActivePreset()` 这类状态读；页面订阅派生，**不持副本、不回页签对账**。
由此退役：`useActivePresetOnTab`、首页/校准页回页签反填、参数页回页签弹回 combo、
`selCache`、预设页的启动快照（两个删除滞留 bug——`remove` / `removeRelease` 后横幅
不更新——随事件通知自然消失）。

「正在使用」与「正在编辑」就此解耦：`activePreset` 只喂"正在使用"显示，用户在参数页
自选的 combo 不再被回页签弹回（A3 验收照出来的设计问题，就此按解耦落地）。

**外部修改策略（开放点）**：程序内的写必然产生事件、所有页面立即一致；手改
`app-state.json` 这类带外变化不产生事件，在下一次读取时生效（盘是底账，不缓存）。
要不要文件监听 / 多窗口广播，留待作者定，不在本轮范围。

## 8. 施工顺序（本轮按此执行）

```text
① 架构决策钉进 spec（本文档）
② Rust：runtime/app_state.rs（app-state.json + 惰性迁移 + 互斥），三态搬入
③ 统一读写入口：catalog / mine / settings 的命令全部改走 AppState
④ 统一订阅：写命令 emit；TS 唯一客户端；四页面改订阅派生
⑤ 删除旧补丁：useActivePresetOnTab / selCache / tab-return 对账 / 参数页弹回
⑥ 重新验收 A3（判据见 UX-FIX-PLAN「A3 收口复验点」）
```

## 9. 与既有文档的关系

- `ARCHITECTURE.md` §4 的 `run/` 定位（运行状态目录）**不变**——变的是里面从"一态一文件"
  收成一个 `app-state.json`；
- `runtime/state.rs` 模块头第 1 条规则（一种状态一个文件）由本决策**取代**，施工时同步
  改注释；其余三条（原子写唯一出口、坏档不静默、schema 代次）原样保留并升格为 AppState
  的纪律；
- `UX-FIX-PLAN-2026-10-06.md` 的「A3 读链收口施工」由本文档接管：那个方案里"唯一读取/
  订阅层"的长相现在定为 AppState，`active-preset.json` 独立文件的前提作废。
