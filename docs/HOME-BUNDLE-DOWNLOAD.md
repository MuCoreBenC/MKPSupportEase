# 首页「下载并应用」按套餐消费（施工计划，2026-10-06）

> 作者裁定（2026-10-06）：**① 修死路 + ② 首页按套餐下载一起做**；套餐里的 BBS **先只落盘**
> （不拷进切片器、不覆盖 BBS 文件夹、不复制过去），用途二期再定。
> **本文是施工计划，先只出文档、不动代码。**
> 关联：[`UX-FIX-PLAN-2026-10-06.md`](UX-FIX-PLAN-2026-10-06.md) §R5（那一刀改出的这颗按钮）、
> [`DATA-ARCHITECTURE.md`](DATA-ARCHITECTURE.md)（四铁律 / 十问）、[`PUBLISH-ARCHITECTURE.md`](PUBLISH-ARCHITECTURE.md) §4 路径语义。

## 0. 一句话

首页第二页那颗「下载并应用」现在是**死路**：它调的下载命令**参数不全**，Tauri 在参数反序列化那一步就
拒了，命令体一行没跑 —— 界面于是只拿到 bridge 兜底的「出了点问题，请重试」。而它本该做的那件事
（按**套餐**把缺的/旧的文件补齐，再应用 MKP 预设）在客户端**还没有这条消费面**。

## 1. 现场与根因（全部实测，不是推测）

### 1.1 那句「出了点问题，请重试」从哪来

`src/api/bridge.ts:43-56` 的 `normalizeError` 是**兜底**：只有当 reject 出来的东西**不是** `AppError`
（不是普通对象带 `code`/`message`/`traceId`）时，才落成 `INTERNAL` + 这句文案。真机日志同时给了正反两面：

```text
2026-10-06T15:19:20.134Z WARN ipc{command="applyActivePreset"}:
  P1S-lite.toml 盘上的内容与目录登记的当前版本对不上，拒绝应用 —— 先「更新」或「重新下载」换一份干净的
  code=ShaMismatch                      ← 这是 AppError，本可以原样显示给用户
2026-10-06T15:19:20.157Z INFO ipc{command="getStaleFiles"} ok
2026-10-06T15:19:20.888Z INFO ipc{command="getDownloadedFiles"} ok
（同一秒内没有任何 downloadRuntimeFile —— 连一条都没有）
```

链路：`PageHome.applyCurrent`（`src/app/home/PageHome.tsx:353-372`）

```ts
try {
  await api.applyActivePreset(comboFileName, 'official')
} catch {                                   // ← 真实原因在这里被吞掉
  await api.downloadCatalogFile(comboFileName)   // ← 这一步根本没跑到后端
  await api.applyActivePreset(comboFileName, 'official')
}
```

`api.downloadCatalogFile(fileName)` 没给第二参 `onTick`，而 `bridge.ts:84-92` 的 `withTick`
**不给回调就完全不挂** `onTick` 字段；`JSON.stringify` 再把 `undefined` 丢掉。Rust 侧这个参数是**必填**：

```rust
#[tauri::command]
pub async fn download_runtime_file(
    app: AppHandle,
    file_name: String,
    on_tick: Channel<DownloadTick>,   // ← 必填
) -> Result<String, AppError>
```

Tauri（`tauri-2.11.6/src/ipc/command.rs:100`）于是报
`command download_runtime_file missing required key onTick`（`InvalidArgs`）——
**发生在命令体之前**，所以 Rust 日志里一条都没有痕迹。这句字符串既不匹配 `not found` 也不匹配
`not registered`，于是落进 `INTERNAL` ⇒ 界面上是那句最没用的话。

**两侧不符是根因**：契约明写「`onTick` 可选：不给就是原来那个『点了等结果』」（`src/api/contract.ts:1259-1269`），
Rust 侧却是必填。`download_runtime_files`（批量）同病。

### 1.2 为什么本机会拒不应用（真实原因，值得知道）

| | 本机盘上那份 | 当前目录登记 |
|---|---|---|
| 路径 | `<appDataDir>/delivery/mkp/presets/P1S-lite.toml` | 同 |
| sha256 | `47671045…` | `f39b50a9…`（= 仓库 `presets/delivery/`） |
| 头注释 | `# release_time: 2026-10-05 08:59:27` | `2026-10-06 03:34:17` |
| 出自 | git `9d3c3c90` 的产物 | git `1b1286a2` 的产物 |

即：本机那份是**上一版**。`apply_active_preset` 的官方线入口闸（`ipc/catalog.rs:789-810`）就是干这个的
——盘上字节必须与目录登记逐字节一致，否则拒。**这个拒是对的**，而「下载并应用」这颗按钮的存在意义
正是「先换干净的字节再应用」。

**顺带纠正一条旧认识**：`getDownloadedFiles` / `getStaleFiles` **不是账本驱动**，是
`delivery.rs::entries_in_status`（遍历 `catalog.files` → 读盘 → `status_of` 比 sha），事件账只供「时间」。
所以 P1S-lite 会出现在漂移清单里（按钮因此显示「下载并应用」），而它的字节认不出出身（不属于本机任何
一代目录）⇒ 不记账、「时间」显示未知。

## 2. 目标行为（照作者原话落成规则）

「套餐里的文件，如果已经有了就不用下载，直接用。」「所有文件都有而且都是新的 ⇒ 显示**应用**；
有旧的 ⇒ **更新并应用**；缺少任意一个 ⇒ **下载并应用**。」

设当前 combo（机型:版本）的套餐文件集 `F`（见 §3），盘上状态取自 `getDownloadedFiles()` / `getStaleFiles()`：

判定**严格按顺序、四态互斥**（作者 2026-10-06 钉死）：

```text
底账正指着这个 combo ──────────→ 已应用（定格，点不了）
  否
   ↓
有缺（∃f: f 不在盘上）？ ── 是 ─→ 下载并应用      ← 缺 + 漂同时存在时显示这一态
  否
   ↓
有漂（∃f: f ∈ 漂移清单）？ ─ 是 ─→ 更新并应用
  否
   ↓
                                应用
```

**「在盘上」= 已下载 ∪ 漂移**（2026-10-06 施工时探针逮到的一处）：那两张单子都只收
盘上**真有**的文件 —— `runtime::delivery::entries_in_status` 遍历目录登记再读盘，字节对得上进
「下载区」、对不上进「漂移」，没读到的（盘上没有）两边都不进。所以**漂 ≠ 缺**：
漂的那份是"在盘上但字节旧/坏了"。把漂当缺，就会把「更新并应用」说成「下载并应用」。

| 条件（按顺序判） | 按钮 | 点击做什么 |
|---|---|---|
| 底账正指着这个 combo | **已应用**（定格，点不了） | 无事可做 |
| `∃f ∈ F`：`f` 不在盘上（∉ 已下载 **且** ∉ 漂移） | **下载并应用** | 只下「缺 ∪ 漂」（去重）→ 全部成功 → 应用 MKP |
| `∃f ∈ F`：`f` ∈ 漂移清单 | **更新并应用** | 同上（`deliver` 会把旧份归档再换新） |
| 其余（`F` 全在且都新） | **应用** | **不发任何网络请求**，直接应用 MKP |

### 2.1 三条实现纪律（作者 2026-10-06 要求，施工必须照做）

1. **`待下 = 缺 ∪ 漂` 最终按 `fileName` 去重成一个集合**。实现上不许 `missing.map(...)` 与
   `stale.map(...)` 各拼一段再连接 —— 同一份文件同时落在两张单子里时会下载两次（`deliver` 幂等，
   但那是白跑一趟网络，而且进度条上会闪两下）。**落成一个 `Set<fileName>` 再发出。**
2. **`VersionFiles` 不完整 ≠ 套餐为空**。`getVersionFiles` 成功但 `incomplete === true`（`missing` 非空）
   时，套餐事实是**不完整的**，此时**不许进入「全齐全新 → 应用」这一态**，也不许把 `F` 当 `∅` 处理
   ——否则会出现「后端没拿全套餐 → 前端认为没有文件 → 按钮变『应用』→ 直接 apply MKP」这条危险路径。
   取不到 / 不完整一律显示**不可用（加载或未配置）**状态，不编造「套餐为空」。
   （与 `useCatalog` 那条「拉不到就空着、不编」并不矛盾：那里空的是**可省**的展示素材，
   这里空的是**判据本身**。）
3. **`downloadCatalogFiles` 继续按 `fileName[]` 收**（`contract.ts:1275` / `download_runtime_files(file_names: Vec<String>)`，
   已核）：**套餐负责决定「哪些文件」，现有下载 API 负责决定「怎么下」**。
   不许因为这次套餐逻辑顺手造 `downloadBundleFiles()` 之类的新命令。

三条硬要求：

1. **只下该下的**：`待下清单 = 缺的 ∪ 漂的`；清单为空时一次网络都不发（作者原话：「已经有了就不用下载」）。
2. **失败说人话、不编成功**：批量下载按份给结局（`downloadCatalogFiles` 的 `DownloadOutcome[]`），
   有任何一份没成 ⇒ **不进入应用**，把「哪一份、为什么」原样显示（`message` 来自后端），
   不再像现在这样把真实原因吞掉。套餐的意义就是两份一起（发 MKP 不发配套 BBS，用户打出来是错的）。
3. **预判 + 兜底**：三态是**预判**（按钮文案）。若清单为空的「应用」仍然失败于 `NOT_FOUND` /
   `SHA_MISMATCH`（预判过期，比如盘上刚被外部改过），**补一次下载再应用一遍**，并把结果如实说出来。

## 3. 数据面：套餐 → 文件（**复用既有链，不新增第二套算法**）

关键发现：**这条路已经存在，且已经是 catalog 驱动**，就是预设页在用的 `getVersionFiles`：

```rust
// src-tauri/src/ipc/presets.rs::version_files_dto（330-428）
//   MKP 一支：catalog.file_of(machine, version)      → path / size / sha256 都是登记真值
//   切片器一支：version.recommended_bundle ?? machine.default_bundle → bundle.assetRefs
//               → catalog.asset(id) → file_kind(a) + dest_of_asset(a)  (= "assets/<台账 path>")
```

- 返回 `VersionFiles { files: FileRef[], incomplete, missing }`，`FileRef = {kind, fileName, path, size?, sha256?}`；
  契约里**已经声明**（`src/api/contract.ts:283-290 / 1072`）⇒ **本轮不动契约、不新增字段**。
- 落点算法只有一处（`runtime::catalog::dest_of_asset`，两端共用），所以**不在 TS 里重算路径**、
  也不在 `RuntimeCatalog` 里另长一份 `bundles` —— 那会变成同一条事实的第二份手抄
  （总纲判据 4 / 「同一条事实别手抄四份」）。
- `version → bundle` 的取值口径（`recommendedBundle ?? defaultBundle`）也只在上面那一处，
  前端不再判一遍。

**两条要说清的边界**（都是既有语义，不新编）：

1. 切片器那两支的 `size` / `sha256` 现在是 `None`（`FileRefDto` 的注释写明）。
   ⇒ 三态的**缺/漂判定不靠它**，靠 `getDownloadedFiles()` / `getStaleFiles()` 的**文件名口径** ——
   这两份清单才是「盘上有没有、与目录一致不一致」的答案来源（首页已经在这两份单子上）。
2. **随包 bootstrap 目录不登记期望值**（`sha256 = None`），`status_of` 对 `None` 一律算 `Current`
   （`delivery.rs:115-121`，注释写明这是有意的：那一侧的目录没资格为字节背书）。
   ⇒ 未 OTA 时「更新并应用」这一态判不出来，只会是「应用 / 下载并应用」两态。**照实如此，不补编**。

## 4. 改动清单

### 4.1 修死路（★ 施工时改道：改在 **bridge**，不在 Rust）

**原计划**是 `on_tick: Channel<DownloadTick>` → `Option<Channel<DownloadTick>>`。
**施工时证伪**：Tauri 的 `Channel<T>` 只实现了 `CommandArg`（它要 `Webview` 才能建），
**没有 `Deserialize`** —— 于是 blanket 的 `impl<D: Deserialize> CommandArg for D` 套不到
`Option<Channel<T>>` 上，`Option<Channel<T>>` **根本编译不出来**（tauri 2.11.6 / 2.12.1 都如此）。
Rust 侧无法表达「这个参数可省」，契约里那句「`onTick` 可选」**只能在 IPC 边界兑现**。

| 文件 | 改什么 | 为什么 |
|---|---|---|
| `src/api/bridge.ts` | `withTick` 改成**无条件**挂一条 `Channel`，只按有没有回调决定挂不挂 `onmessage`（JS 侧 `Channel` 的 `onmessage` 缺省就是空函数，收到即丢） | 契约说「可选」，Rust 说「必填」⇒ 桥这一层兜住：两条下载命令**同时**修好（`download_runtime_file` 与 `download_runtime_files`），Rust 侧一个字节不动 |
| `scripts/check-channel-args.mjs`（**新判据**） | ① 凡 Rust 命令签名里含 `Channel<` 的，bridge 里调它的那个 `call(...)` 必须包在 `withTick(` 里；② `withTick` 不许「没有回调就原样返回入参」（这一行就是这次的死路） | 这类错**编译期抓不到**（TS 侧看不到 Rust 签名）、**运行期只在真机上炸**（浏览器 mock 不过 `invoke`）；红的时候把「哪条命令、为什么」直接打出来。已接进 `npm run check:channel-args` 与 CI |
| `src-tauri/src/runtime/delivery.rs`（**本轮不做**） | `deliver` 在**目录登记了期望值且盘上一致**时不调 `source.fetch` | 作者 2026-10-06 拍板：客户端已预过滤，那是额外的网络优化，不与本刀混一起 |

### 4.2 前端（主体）

| 文件 | 改什么 |
|---|---|
| `src/app/home/useBundleFiles.ts`（**新增**） | 按 combo 取 `api.getVersionFiles(model, variant)`（跨挂载缓存，key = `机型/版本`；**失败的缓存不写**，下次进这一页再试一次），产出 `{files, problems, presetFileName}`；`null` / `incomplete` / 读失败一律进 `problems`（纪律②） |
| `src/app/home/PageHome.tsx` | ① 用套餐文件集替换 `fileReady` 布尔；② 四态（§2 表）；③ 盘的现状仍取 `getDownloadedFiles` / `getStaleFiles`（与预设页同一套账）；④ 动作 = 「只下缺 ∪ 漂（去重）→ 逐份看结局 → 全成才 apply」；⑤ 兜底只留给 `NOT_FOUND / SHA_MISMATCH` 且**仅当预判为空**；⑥ 内层 `catch {}` 拆掉，`bundle.problems` 也照实显示 |
| `src/api/mockServer/catalogFiles.ts`（**新增**） | mock 的目录登记收成**一处事实**：`getRuntimeCatalog().files[]` 与「这个 combo 的套餐那一支 MKP」共用它（以前后者用的是上游老命名 `A1.toml`，与目录/下载区对不上 ⇒ 首页套餐会一律显示「缺」） |
| `src/api/mockServer/files.ts` | 套餐里那份 MKP 改成**查目录登记**（`catalogFileOf`，与真机 `catalog.file_of` 同口径） |
| `src/api/mock.ts` | `MOCK_DOWNLOADED` 补一条**套餐的另一半**（`MKPProcess A1 0.4 0.20.json`）—— 不然浏览器里画不出「应用」那一态；四态各占一档的对照表写在该常量头上 |
| `scripts/probes/home-flow.mjs` | 加四态断言 + 「零下载」断言（点「应用」不该出现「套餐没下全」；点另两态必须出现，且不许 apply）。三条坑写在代码里：① 向导是 SlideDeck，**同名按钮别处也有**（预设浮层里的「应用」），只能认 Playwright 说可见的那颗；② 量每一档都**从新加载的页面走**（「更换机型」挂在另一张卡上，量完一次就点不到）；③ 品牌必须点（分级揭示） |

### 4.3 台账

- 本文（`docs/HOME-BUNDLE-DOWNLOAD.md`）；
- `docs/UX-FIX-PLAN-2026-10-06.md` §R5 行补一句指向本文（那一刀的措辞「这是套餐」当时只是注释，没有实现）。

## 5. 硬规矩（本轮**不做**的事，写在这里防以后走偏）

1. **BBS 只落盘，不碰切片器**：落点永远 = `<appDataDir>/<catalog.path>`（= `assets/bbs/Process/…`），
   由 `deliver` 原子写入。**不往切片器自己的配置目录写、不覆盖、不复制、不"顺手装一下"**。
   「拷进切片器」是一条还没存在的写命令，二期单独定（现在后端没有）。
2. **「应用」只认 MKP**：`apply_active_preset` 的官方线入口闸只接受 `mkp_preset`
   （`ipc/catalog.rs:789-810` 与 `mine::kind_of`）。**落盘 ≠ 生效** —— BBS 下下来只是躺在那儿。
3. **写底账仍然只有一个口**：首页这颗按钮走 `applyActivePreset` / `activateCombo` 既有的那条路；
   `effect` 一律只读（R6 的纪律），不许在 effect 里写共享状态。
4. **不引兼容层**：不为「旧目录 / 旧路径 / 没有套餐登记的目录」补第二套解析；取不到套餐就是空集 + 照实说。

## 6. 已拍板（作者 2026-10-06）

1. ✅ **「已应用」优先**：底账已指向当前 combo 就保持「已应用」定格，BBS 是否补齐留二期。
   不为了 BBS 反过来破坏「应用状态只由 MKP 生效底账决定」这条语义。
2. ✅ **本轮不做 `deliver` 的二次省网络优化**（§4.1 那行降为「不做」）：前端已经明确算出「缺 ∪ 漂」，
   正常路径不会把已最新的文件送进去。要改 `deliver` 属于额外优化，不与「修死路 + 套餐消费」混在一刀里；
   将来实测发现仍有重复 fetch，再单独做。

**本轮施工范围（作者原话整理）**：

```text
① 修死路：Channel<T> → Option<Channel<T>>，无 onTick 也能正常执行命令
② 首页真正消费套餐：getVersionFiles(combo) → 套餐 F = MKP + BBS → 已下载/漂移
   → 缺 ∪ 漂 → downloadCatalogFiles → 全部成功 → applyActivePreset（只认 MKP）
③ 三态（+ 已应用 共四态，严格按 §2 顺序判）
④ 失败不吞：DownloadOutcome[] → 哪一份失败 + message → 直接告诉用户
```

★ **这一刀真正的边界**（作者点名）：首页此后**不是在判断「一个 TOML 文件能不能应用」，
而是在消费一个 combo 的交付套餐**。所以以后再看到 `fileReady` 这种**单文件布尔值**就该警觉
—— 它已经表达不了这个页面真正的状态。

**施工顺序（Task 1→7，严格按此走）**：

| Task | 内容 | 完成条件 |
|---|---|---|
| 1 | 修死路（**改在 bridge**，见 §4.1 的证伪）+ 新判据 `check-channel-args` | 无 `onTick` 也能正常执行；判据在 CI 上会红 |
| 2 | 首页接 `getVersionFiles(model, version)`，换掉 `fileReady`（**先不碰按钮动作**） | 套餐文件集能拿到，`incomplete` 按纪律 ② 处理 |
| 3 | 四态 | 严格按 §2 顺序，四态互斥；**「在盘上」= 已下载 ∪ 漂移**（施工时探针逮到的一处，见 §2） |
| 4 | 下载动作：只发 `缺 ∪ 漂`（纪律 ① 去重），逐份看 `DownloadOutcome[]`，**任何失败即停、不 apply** | 失败原样透出（纪律 ④） |
| 5 | 过期兜底：**仅**预判为空、`apply` 得到 `NOT_FOUND / SHA_MISMATCH` 时补一次下载 → 再 apply | 不是重新套一个 `catch {}` |
| 6 | mock 演四态 + 探针补「四态 + 零下载」断言 | 浏览器预览可复现（见 §4.2 的三条坑） |
| 7 | 全绿集合 + 真机验收（尤其 §7 第 3 条：删 BBS 后只落 `appData/assets/bbs/…`，切片器目录不动） | 全绿 + 作者真机 |

## 7. 验收

**真机（作者）**
1. 首页选 P1S / lite（本机那份是旧版）→ 按钮显示「**更新并应用**」→ 点击 → 日志出现
   `downloadRuntimeFile ok`（本轮之前一条都没有），状态转为「已应用」。
2. 预设页 / 首页对同一 combo 的说法一致（都认「已应用」）。
3. 把 BBS 从盘上删掉 → 按钮转「**下载并应用**」→ 点击后落 `<appDataDir>/assets/bbs/Process/0.4mm/MKPProcess P1S 0.4 0.24.json`，
   **切片器目录一个字节都没变**。
4. 全齐且新时点「应用」→ 日志里**没有**任何 `downloadRuntime*` 调用。

**全绿集合**（与既有纪律一致）：`cargo fmt`；双 feature `cargo clippy --all-targets -- -D warnings`；
`cargo test` + `-p mkp-support-ease --features workbench --lib`；`npm run build && check:bundle && check:zero-network`
（+ 本轮新加的 `check:channel-args`）；`npx tsc -b`；`npm run lint`；探针 `home-flow.mjs --pick`（4173）。

**浏览器层已经量过（2026-10-06）**：`home-flow.mjs --pick` 全绿 —— 四态逐档量到
（A1/标准版「应用」、A1/快拆版6月以前「更新并应用」、A1 mini/标准版「下载并应用」；目录里没登记的机型「套餐未配置」），
且「应用」那一档点下去**没有**出现「套餐没下全」（= 没走下载分支）。`presets.mjs` 也全绿
（mock 那两处改动没碰坏预设页）。

★ **本机 Rust 侧的预存红与本刀无关，也不由本刀修**（本次一个 `.rs` 都没改，`git diff --stat` 里没有任何
Rust 文件）：`cargo fmt --check` 3 处漂移 + 双 feature clippy 各 10 条 `disallowed_methods` + 1 条 doc 缩进。
作者 2026-10-07 裁定：**单独记成下一刀的「预存工具链 / 纪律清理」**（见 `docs/RUST-LINT-CLEANUP.md`）——
本刀不碰 Rust ⇒ 本刀不负责修 Rust 红；**真机四条全绿后这一刀就提交、合并，两件事不许搅在一起**。
