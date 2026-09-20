# MKPSupportUN 开局：Tauri 2 壳 + v023 前端整体移植 + 三件只能开局定的地基

> 目标仓库：`/Users/wzy/projects/MKPSupportUN`（当前为空目录，**不是** git 仓库，本轮第一步才 `git init`）
> 来源仓库：`/Users/wzy/projects/mkp-adaptive-console`（main，工作区干净，tip = `5597f55`，版本 0.1.4）
> **试验场仓库本轮一行不动。** 所有搬运都是单向拷贝，来源仓库只读。
> UN = **unofficial**。

---

## 0. 要做什么（一句话）

在 `MKPSupportUN` 起一个 Tauri 2 + React 的产品仓，把试验场的 **v023 成品前端整体移植过来（不重写视觉）**，同时把三件后补代价极高的地基做进去：**唯一写盘出口 `atomic_write`**、**贯穿 IPC 的 trace id**、**结构化 `AppError`**。

本轮**不做任何业务功能**。验收标准是"壳跑起来、视觉一致、链路通、错误可追"，不是"某个页面能用"。

---

## 1. 已定的四处决策

| 议题 | 结论 | 说明 |
| --- | --- | --- |
| 视觉怎么来 | **整体移植 v023，不重写** | 纠正上一轮"照着抄"的建议。v023 的效果是大量小细节堆出来的（`SlideDeck.tsx@LINE[50..66]` 的 peek 斜坡在 JS 算、平面层常驻合成层、clamp 连续缩放、像素对齐落位），重写必然丢细节且工时翻倍。代价是会一起带进试验期代码 —— 用**剥离清单**处理（§2.3），不靠重写 |
| 数据目录 | **两层** | 内部原件/归档/索引/日志 → Tauri `appDataDir`；用户可见文件 → `Documents/MKPSupportUN`。理由：macOS 若开了「桌面与文档」iCloud 同步，Documents 下的文件会被驱逐成占位 stub，读出来内容不对，会把后续 Preset 的 SHA 失效判定搞成误报 |
| 撤销重做栈 | **砍** | 不做通用 command pattern。真需要的地方（参数编辑、轴偏移）后续用「编辑前快照 + 单层撤销」，局部做，不进本轮 |
| 命名 | UN = unofficial | crate `mkpsupport-un`，package name `mkpsupport-un` |
| git 纪律 | **第一版就双层拦截，main 只能由 PR 推进** | 本地 hook + GitHub ruleset 两层，缺一层都不算拦住。骨架代码本身就走第一个 PR 进 main —— 详见 §2.5 |

---

## 2. 仓库形态

### 2.1 目标目录结构

```text
MKPSupportUN/
├── .comate/specs/                  # SDD 产物，跟着仓库走
├── docs/
│   ├── DESIGN-SPACING.md           # 搬：间距/动画原则
│   ├── 3D-ASSET-CONTRACT.md        # 搬：上游资产契约 v3
│   ├── GIT-WORKFLOW.md             # 搬：闸门说明
│   └── ARCHITECTURE.md             # 新：本 doc 收口后的常驻版
├── scripts/
│   ├── hooks/                      # 搬：七道 git 闸
│   ├── setup-hooks.mjs             # 搬
│   └── release.mjs                 # 搬（需改：多一步 cargo build）
├── public/                         # 搬：静态资产
├── src/
│   ├── api/                        # contract.ts / mock.ts / bridge.ts / index.ts / errors.ts
│   ├── app/                        # ← v023 提升为唯一前端（去掉 V023 后缀）
│   │   ├── App.tsx
│   │   ├── pages/{PageHome,PageCalib}.tsx
│   │   ├── components/             # v023/components 原样
│   │   └── {useCalibration,usePreset,calibAxes,flip}.ts
│   ├── components/                 # 搬：16 个跨版本共用组件
│   ├── hooks/                      # 搬：useDensity / usePlatform / useStickyState
│   ├── calib/                      # 搬：*.generated.ts 校准板产物
│   └── styles/                     # 搬：tokens.css / global.css
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── clippy.toml                 # 禁 std::fs 写入 API
│   ├── capabilities/default.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── error.rs                # AppError / ErrorCode
│       ├── obs/tracing.rs          # subscriber + 按天轮转
│       ├── fsx/
│       │   ├── atomic.rs           # 唯一写盘出口
│       │   └── paths.rs            # 两层根 + 防穿越
│       └── ipc/mod.rs              # command 注册 + span 包装
├── index.html
├── package.json
├── vite.config.ts
├── eslint.config.js / .stylelintrc.json / tsconfig*.json
└── .gitignore
```

### 2.2 原样搬过去的清单

| 来源（mkp-adaptive-console 内相对路径） | 目标 | 处理 |
| --- | --- | --- |
| `src/styles/tokens.css`、`global.css` | 同名 | 原样 |
| `src/components/*`（16 文件，8 组 tsx+css） | 同名 | 原样 |
| `src/hooks/*` | 同名 | 原样 |
| `src/calib/*.generated.ts` | 同名 | 原样，数据资产 |
| `src/versions/v023/**` | `src/app/**` | **改名去 V023 后缀**，import 路径跟着改 |
| `public/**` | 同名 | 原样 |
| `index.html`、`eslint.config.js`、`.stylelintrc.json`、`tsconfig*.json` | 同名 | 原样 |
| `docs/DESIGN-SPACING.md`、`docs/3D-ASSET-CONTRACT.md`、`docs/GIT-WORKFLOW.md` | 同名 | 原样 |
| `scripts/hooks/**`、`scripts/setup-hooks.mjs` | 同名 | 原样 |
| `scripts/release.mjs` | 同名 | 需加一步 `cargo build`（§2.5） |
| `src/api/{contract,mock,bridge,index,errors}.ts` | 同名 | contract 扩错误类型，bridge 重写内部，index 改判断（§4） |

### 2.3 剥离清单（搬过去之后要删掉/改掉的试验设施）

| 对象 | 为什么剥 | 怎么处理 |
| --- | --- | --- |
| `src/versions/registry.ts` + 多稿切换 + `localStorage` 的 `mkp.version` | 产品只有一个界面，没有"稿号" | 删。`registry.ts` 里那个 `id: 'v0.0.22'` 的历史包袱一并消失 |
| `src/App.tsx`（预览器外壳：Ultra/Wide/Compact 窗口尺寸预设 + 版本下拉） | 真窗口尺寸由 Tauri 管，不需要模拟 | 删。`src/app/App.tsx` 直接成为根组件 |
| `src/dev/DevPanel.tsx`、`CurveEditor.tsx`、`devStore.ts` | 调参面板是试验场工具 | **先核实 `ExplodedHero` / `SlideDeck` 是否读 `devStore`**；若读，把当前曲线值从 `src/dev/heroCurves.json` 固化成模块常量，再删面板。`heroCurves.json` 作为数据保留 |
| `src/versions/v001..v022` | 老稿 | 不搬 |
| `src/mock/{mkp,mkpFull,machine,testModels}.ts` | 产品数据从 Rust 取 | 不搬。浏览器调试路径需要的最小假数据内联进 `src/api/mock.ts` |
| `tools/stl-svg/**`、`tools/dev-server/**` + `vite.config.ts` 里的 `calibFs()` / `curvesFs()` | dev-only 无鉴权写盘端点，是工作台，不是产品 | 留在试验场。产品仓 `vite.config.ts` 只留 `react()` |
| `pic/`（16M）、`tmp-shots/`、根目录 7 张 PNG | 参考图 | 不搬 |
| 根目录 30 个 `feedback_*.md` / 9 个 `project_*.md` / `reference_*.md` / `MEMORY.md` | 记忆文件，属于试验场的历史 | 不搬。产品仓的约定写进 `docs/ARCHITECTURE.md` |
| `scripts/probe*.mjs`、`stl-to-svg.mjs` | 工作台配套 | 不搬 |
| 物理沙盘（cannon-es / `interactive-physics-sandbox`） | 独立试验页面 | 不搬。**需核实它是否有独立入口文件、是否牵连 `package.json` 依赖** |

> 剥离过程中有两处"需核实"，我会在执行阶段先读代码确认依赖边界再动手，不凭印象删。

### 2.4 依赖

前端保持极简，沿用试验场的 exact 版本钉法（`package.json` 里全是精确版本，不用 `^`）：

- 保留：`react` 18.3.1、`react-dom` 18.3.1、`three` 0.186.0、`@types/three` 0.186.0 + 现有工具链
- 新增：`@tauri-apps/api`、`@tauri-apps/cli`（devDep）、按需 `@tauri-apps/plugin-fs`、`plugin-dialog`、`plugin-opener`
- 移除：`playwright`（试验场用来截图比档，产品仓本轮不需要）

Rust 侧首批：`tauri` 2.x、`serde`、`serde_json`、`thiserror`、`tempfile`、`tracing`、`tracing-subscriber`、`tracing-appender`、`uuid`（v7）。

> **版本以 `cargo add` / `npm i` 实际解析到的为准，装完把确切版本回填这份 doc，并锁 `Cargo.lock` + `package-lock.json`。** 我不在这里写死小版本号 —— 写死了大概率和实际装上的不一致，反而误导。

### 2.5 闸门与发版

`scripts/hooks/` 原样搬，七道闸的判据不变（main 直提、非快进、删远端、tag 与 `package.json` 不符、分支名前缀等）。`release.mjs` 要加的一步：`npm run build` 之后补 `cargo build --manifest-path src-tauri/Cargo.toml`，否则"lint+build 过了"不代表 Rust 侧能编译。

---

## 3. 三件地基

### 3.1 唯一写盘出口 `atomic_write`

```rust
// src-tauri/src/fsx/atomic.rs
use std::fs::File;
use std::path::Path;
use tempfile::NamedTempFile;

/// 全仓唯一的写盘出口。
///
/// 三步都不能省：
/// 1. 临时文件建在**目标的父目录**里 —— rename 只在同一文件系统内原子，
///    建在 /tmp 再 rename 跨分区会退化成 copy+delete，中途断电就是半个文件。
/// 2. persist 之前 sync_all —— 不然内容还在页缓存里，rename 完了目录项指向一个空文件。
/// 3. 最后 fsync 父目录 —— rename 本身也要落盘，否则崩溃后可能既没新文件也没旧文件。
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::invalid_argument("目标路径没有父目录"))?;
    std::fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut tmp, bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;

    File::open(parent)?.sync_all()?;
    Ok(())
}
```

**纪律怎么落地**：口头约定没用，用 clippy 强制。

```toml
# src-tauri/clippy.toml
disallowed-methods = [
  { path = "std::fs::write", reason = "走 fsx::atomic_write" },
  { path = "std::fs::File::create", reason = "走 fsx::atomic_write" },
  { path = "tokio::fs::write", reason = "走 fsx::atomic_write" },
]
```

`atomic.rs` 自身用 `#[allow(clippy::disallowed_methods)]` 开一个洞，其他文件一律拦。

### 3.2 路径分层 `paths.rs`

两个根，职责完全分开：

```text
internal_root()  = appDataDir()              # ~/Library/Application Support/<identifier>
├── cloud/          云端原件（只读、带 SHA 记录）
├── archive/        被更新替换掉的旧云端原件
├── index/          内部索引（预设关系、SHA、发布 ID）
├── logs/           tracing 按天轮转
└── run/            运行时临时状态

user_root()      = documentDir()/MKPSupportUN   # ~/Documents/MKPSupportUN
├── exports/        导出的 gcode
├── reports/        报告
└── presets-mine/   用户自己的预设副本（改过的、外部拖进来的）
```

延续 `~/Documents/MKPSupportSSR` 那套命名惯例（它现有 `baselines/ content/ gcode_history/ logs/ models/ presets/ run/ settings.json`），但按"内部 / 用户可见"重新归属：`logs`、`run`、`baselines` 归内部根，`gcode_history` 归用户根，`presets` 一分为二（云端原件在内部、用户副本在用户根）。

**防穿越规则**（照搬 `calibFs.mjs@LINE[56..57]` 的 `inside()` 思路）：任何 command **不接收绝对路径**，只接收 `(Root, 相对路径)` 或稳定 ID；拼接后 `canonicalize` 再断言仍在根内，否则报 `PermissionDenied`。

`capabilities/default.json` 里 fs 权限按命令逐个授权 + scope 限到这两个根，**不使用 `fs:default`**。

### 3.3 trace id + tracing

```rust
// 每个 command 入口生成 trace id 并开 span
#[tauri::command]
pub async fn get_preset(variant_id: String) -> Result<Option<Preset>, AppError> {
    let trace_id = new_trace_id();                       // uuid v7，时间有序，便于按时间翻日志
    let span = tracing::info_span!("get_preset", trace_id = %trace_id, variant_id = %variant_id);
    let _g = span.enter();
    service::get_preset(&variant_id).map_err(|e| e.with_trace(&trace_id))
}
```

**一处取舍：成功路径不包 Envelope。** 两个选项：

- A：只有错误带 `traceId`
- B：成功也包一层 `{ data, traceId }`

选 **A**。B 会污染所有前端调用点的解构（每个 `await api.x()` 都要多剥一层），而成功路径本来就不需要用户报 id —— 日志里有 span，按时间和参数照样能查到。

subscriber 配置：`tracing-appender` 的 `rolling::daily` 落 `internal_root/logs/un.log`，dev 下同时输出 stderr。**日志目录写不进去时只退到 stderr，不阻断启动** —— 因为日志失败不该让软件打不开。

### 3.4 `AppError` 契约

```rust
// src-tauri/src/error.rs
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    /// 直接给用户看的一句中文，不含技术细节
    pub message: String,
    pub trace_id: String,
    /// 技术细节，折叠在"详情"里，可复制
    pub detail: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NotFound, PermissionDenied, InvalidArgument,
    Corrupted, ShaMismatch, Io, NotImplemented, Internal,
}
```

前端 `src/api/contract.ts` 增加对应类型 + 判定函数，让**错误也进契约**：

```ts
export type ErrorCode =
  | 'NOT_FOUND' | 'PERMISSION_DENIED' | 'INVALID_ARGUMENT'
  | 'CORRUPTED' | 'SHA_MISMATCH' | 'IO' | 'NOT_IMPLEMENTED' | 'INTERNAL'

export interface AppError {
  code: ErrorCode
  /** 可直接展示给用户的中文 */
  message: string
  /** 报错时显示在角落，用户截图就能定位 */
  traceId: string
  detail?: string
}

export function isAppError(e: unknown): e is AppError
```

`bridge.ts` 里统一 `normalizeError`：`invoke` 的 reject 值不一定是我们的结构（panic、序列化失败、command 不存在时 Tauri 会抛字符串），兜底成 `{ code: 'INTERNAL', message: '内部错误', traceId: '-', detail: String(e) }`。

---

## 4. 前端接线改动（3 个文件）

| 文件 | 现状（已核对源码） | 改动 |
| --- | --- | --- |
| `src/api/index.ts:15` | `export const api: MkpApi = import.meta.env.DEV ? mockApi : bridgeApi` | 换成运行时探测。Tauri dev 下 `DEV` 也是 `true`，照现在的写法壳永远接不上 |
| `src/api/bridge.ts:28..34` | 读 `window.__mkp_api?.xxx?.() ?? missing(...)` | 内部换成 `invoke()` + `normalizeError`，`contract.ts` 的方法签名一个字节不动 |
| `vite.config.ts:26` | `server: { host: true, port: 5178, strictPort: true, open: false }` + `calibFs()` + `curvesFs()` | 去掉两个 dev 插件；`host` 改成 `process.env.TAURI_DEV_HOST ?? false`；加 `clearScreen: false` |

`index.ts` 的新判断：

```ts
/**
 * 用运行时探测而不是 import.meta.env.DEV：
 * Tauri dev 下 DEV 也是 true，按构建期判断会让壳永远接不上。
 * 探测之后同一个 dev server 两用 —— 浏览器里走 mock 调视觉，Tauri 窗口里走真壳。
 */
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
export const api: MkpApi = inTauri ? bridgeApi : mockApi
```

`vite.config.ts` 的 `host`：原来无条件 `host: true` 是为了局域网预览，代价是把两个无鉴权写盘端点暴露给同网段（原注释 `@LINE[23..24]` 自己写了这条风险）。产品仓里那两个端点已经不存在，但也没必要默认绑全网卡 —— 改成只在 Tauri 需要时（`TAURI_DEV_HOST` 有值，真机调试）才绑。`5178` + `strictPort` 沿用，`tauri.conf.json` 的 `devUrl` 必须写同一个端口，不然 WebView 加载到 404。

---

## 5. 数据流

```text
前端页面
   │  await api.getPreset(variantId)
   ▼
src/api/index.ts   ── 运行时探测 ──┬─→ mock.ts（浏览器）
   │                              └─→ bridge.ts（Tauri 窗口）
   ▼
invoke('get_preset', { variantId })
   ▼
src-tauri/src/ipc/mod.rs
   │  new_trace_id() → info_span!(trace_id)
   ▼
service 层
   │  fsx::paths::internal_root() / user_root()   ← 只认 (Root, 相对路径)
   │  canonicalize + inside() 断言
   ▼
fsx::atomic_write(path, bytes)          ← 全仓唯一写盘出口
   │  NamedTempFile(同父目录) → write_all → sync_all → persist → fsync(parent)
   ▼
磁盘

错误回路：
io::Error ──From──→ AppError{code:IO} ──with_trace(id)──→ serde ──→ invoke reject
                                                                      │
                                                     bridge.normalizeError
                                                                      ▼
                                              界面提示 + 角落显示 traceId（可复制）
                                                                      │
                                              internal_root/logs/un.log 按 id 可查
```

---

## 6. 边界与异常

| 情形 | 行为 |
| --- | --- |
| 目标目录不存在 | `atomic_write` 先 `create_dir_all`，失败报 `IO` |
| 临时文件与目标跨分区 | 不会发生：`NamedTempFile::new_in(parent)` 保证同目录 |
| `persist` 时目标被占用（Windows 常见） | 报 `IO`，带上原路径，**不自动重试** —— 重试会掩盖"有别的进程在占着"这个真问题 |
| Documents 被 iCloud 驱逐成占位 stub | 只影响 `user_root` 下用户可见文件；云端原件、归档、索引都在 `internal_root`，不受影响。这正是分两层的目的 |
| Tauri 窗口里但 Rust 侧还没实现某 command | `invoke` reject → normalize 成 `NOT_IMPLEMENTED`，界面显示可读提示 + traceId，不白屏 |
| 浏览器里打开 5178 | 探测不到 Tauri，走 mock，不报错。纯前端调视觉的路径完整保留 |
| 日志目录写不进去 | tracing 退到 stderr，**不阻断启动** |
| `internal_root` / `user_root` 首次不存在 | 启动时 `create_dir_all` 建齐；建不出来（权限）报错并显示具体路径 |
| command 收到绝对路径 | 一律拒绝，报 `INVALID_ARGUMENT`。路径只能是 `(Root, 相对路径)` 或 ID |

---

## 7. 明确不做

- 任何业务功能页面的新行为（Preset 第一版是**下一轮**，本轮只把地基铺好）
- 撤销重做栈（已砍）
- 自动更新、代码签名、公证
- Windows / Linux 打包验证（本轮只保证 macOS `tauri dev` + `tauri build` 跑通）
- 搬 `tools/stl-svg` 工作台
- 动试验场仓库（一行不动，纯只读来源）
- TOML 解析（跟着 Preset 那一轮走，届时用 Rust 的 `toml` crate，前端不引 TOML 库）

---

## 8. 预期结果

1. `cd /Users/wzy/projects/MKPSupportUN && npm run tauri dev` 起一个原生窗口，显示 v023 的首页与校准页，**视觉与试验场逐档一致**（mini / compact / wide / ultra 四档比对）
2. `npm run dev` + 浏览器开 `http://localhost:5178/` 仍可用，走 mock，纯前端调试路径不丢
3. Tauri 窗口里 `getPreset` / `getCalibModels` 的数据来自 Rust（先返回硬编码值，只为证明链路通）
4. 故意把某个 command 从 `generate_handler!` 里摘掉 → 界面出现可读错误 + traceId，`internal_root/logs/` 当天日志里按该 id 能查到那一条
5. `internal_root/logs/` 下有按天命名的日志文件；两个数据根目录首次启动自动建齐
6. clippy 拦截生效：在 `ipc/` 里写一行 `std::fs::write` 会被 `cargo clippy -- -D warnings` 拒掉
7. `tsc -b` / `eslint` / `stylelint` / `vite build` / `cargo build` / `cargo clippy` 全过
8. `npm run release` 的闸门在新仓库生效（`core.hooksPath` 指向 `scripts/hooks`）

---

## 9. 需要你点头的两处

**① Bundle identifier 决定内部目录的真实路径。**
Tauri 的 `appDataDir()` 等于 `~/Library/Application Support/<identifier>`。如果 identifier 按惯例写反域名 `com.mkpsupport.un`，那内部根就是 `~/Library/Application Support/com.mkpsupport.un/` —— 路径不好认但符合规范。想要目录名是人类可读的 `MKPSupportUN`，就得不用 `appDataDir()`，改成手工拼 `dataDir()/MKPSupportUN`。

我倾向 **identifier = `com.mkpsupport.un` + 直接用 `appDataDir()`**：少一层手工拼接就少一处平台差异。你要是更看重能在 Finder 里一眼找到，我就改手工拼。可以

**② 产品名与窗口标题。**
`MKPSupport UN`？还是别的？这个字符串会同时出现在窗口标题、`Cargo.toml` 的 crate 名、`package.json` 的 `name`、以及 bundle 的 `productName`，定了之后改起来要动好几处。
