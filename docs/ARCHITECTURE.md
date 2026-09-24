# SupportEase 工程约束

> **这份文档管工程：**Tauri 结构、两层数据根、IPC 契约、trace、原子写、权限边界。
> **产品行为查另一份：**`docs/PRESET-PRODUCT-RULES.md`（本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 / 状态流转）。
> git 纪律与发版流程见 `docs/GIT-WORKFLOW.md`。间距与动画原则见 `docs/DESIGN-SPACING.md`，上游 3D 资产契约见 `docs/3D-ASSET-CONTRACT.md`。
> **版本身份与文件命名规范见本文 §10** —— 工作台、构建器、发布器、消费端共用那一套。

---

## 1. 这是什么

MKP 支撑辅助的桌面端：选机型 → 确认偏移 → 校准 Z / XY → 打测试件，外加预设管理与后处理报告。
离线工具，不联机控制打印机。

技术栈：Tauri 2 + React 18 + TypeScript + Vite，Rust 侧负责文件系统与将来的云端同步。

## 2. 命名分层

四个名字刻意解耦，改其中一个不动其他三个：

| 层 | 值 | 谁看得见 |
| --- | --- | --- |
| 产品名 / 窗口标题 | `SupportEase` | 用户 |
| bundle identifier | `com.mkpsupport.ease` | 系统（决定数据目录位置，**改它等于换一个新应用**） |
| 仓库名 | `MKPSupportEase` | 开发 |
| npm 包名 / crate 名 | `mkp-support-ease` | 工具链（必须小写） |

## 3. 目录结构

```text
src/                      前端
├── api/                  与后端之间唯一的约定
│   ├── contract.ts       只有类型：MkpApi 五个方法 + AppError
│   ├── mock.ts           临时实现（浏览器里跑的那份）
│   ├── bridge.ts         Tauri 实现：invoke 五个 command
│   └── index.ts          运行时探测走哪一份
├── app/                  应用本身（唯一的界面，没有"稿号"）
│   ├── App.tsx           根组件：页签 + 页面
│   ├── pages/            PageHome（五步向导）/ PageCalib（校准）
│   ├── components/       这一版界面自己的组件
│   ├── ui/               通用控件（Btn / Badge / Modal …）
│   ├── constants/        界面结构数据：页签、品牌 / 机型 / 版本、后处理脚本
│   └── heroCurves.ts     大图尺寸与微移的固化曲线
├── components/Icon.tsx   跨界面共用的图标
├── hooks/                useDensity（密度档）/ usePlatform / useWindowSize
├── calib/                标定板的生成产物（.generated.ts，别手改）
└── styles/               tokens.css + global.css

src-tauri/                Rust 侧
├── src/error.rs          AppError / ErrorCode
├── src/obs/tracing.rs    trace id + 按天滚动日志
├── src/fsx/paths.rs      两层数据根 + 防穿越
├── src/fsx/atomic.rs     唯一的写盘出口
├── src/ipc/mod.rs        五个 command
├── clippy.toml           禁用直接写盘的方法
└── capabilities/         权限（目前只有 core:default）
```

## 4. 两层数据根

| 根 | 位置 | 放什么 | 子目录 |
| --- | --- | --- | --- |
| Internal | `appDataDir()` | 程序管理的：云端原件、归档、索引、日志、运行状态 | `cloud/ archive/ index/ logs/ run/` |
| User | `documentDir()/SupportEase` | 用户自己要看要拷的：预设副本、导出、报告 | `exports/ reports/ presets-mine/` |

**为什么不都放 Documents**：macOS 开了「桌面与文档」iCloud 同步后，Documents 里的文件会被驱逐成
占位 stub —— 读出来内容不对，会把 Preset 的 SHA 失效判定变成误报。程序管理的数据不能放在
一个会被系统悄悄搬走的地方。

两个根在启动时 `create_dir_all` 建齐；建不出来**不阻断启动**，只打 warn 并把路径写进日志。

### 防穿越

所有相对路径都必须经过 `fsx::paths::resolve`，三道判据缺一不可：

1. 拒绝绝对路径；
2. 拒绝 `..` 等非普通分量；
3. 拼完之后用**规范化后的真实路径**再确认仍在根内 —— 符号链接指向根外时前两条都看不出来。

越界一律 `PERMISSION_DENIED`。目标文件还不存在是正常情况（第一次写），不判越界。

## 5. 唯一的写盘出口

`fsx::atomic::atomic_write`：同目录临时文件 → 写 → `sync_all` → `persist`（同分区 rename，
POSIX 原子）→ fsync 父目录。任何一步失败，目标文件都还是旧的那一份。

直接 `std::fs::write` 的问题不是不优雅，而是**它先把目标文件截断成 0 字节再写** ——
中途崩溃，用户的预设不是回到旧版本，是变成空文件。

这条纪律由 `clippy.toml` 的 `disallowed-methods` 强制：`std::fs::write` /
`std::fs::File::create` / `tokio::fs::write` 全仓禁用，唯一的洞是 `fsx/atomic.rs` 顶上的
`#[allow(clippy::disallowed_methods)]`。CI 的 `rust` job 跑 `cargo clippy -- -D warnings`，
所以这道拦截在每个 PR 上都生效（已实测：在 `ipc/` 里写一行 `std::fs::write` 会让 clippy 报错）。

## 6. IPC 契约

五个 command，形状定义在 `src/api/contract.ts`，Rust 侧在 `src-tauri/src/ipc/mod.rs`：

| TS | Rust command | 作用 |
| --- | --- | --- |
| `getPreset(variantId)` | `get_preset` | 取某个打印件版本的预设；`null` = 没有这一份（不是出错） |
| `saveOffsets(axes)` | `save_offsets` | 三轴偏移落盘（走 `atomic_write`） |
| `getCalibModels()` | `get_calib_models` | 校准板清单 |
| `openModel(modelId)` | `open_model` | 让壳去打开模型文件 |
| `getTestModels()` | `get_test_models` | 测试模型清单 |

命名两套、映射只在 `bridge.ts` 一处：TS 侧 camelCase，Rust 侧 snake_case。

**前端永不碰文件系统。** 所以 `capabilities/default.json` 里只有 `core:default` ——
既不需要 `fs:default`，也不需要逐条 fs 权限。将来真要用 `plugin-fs`（比如「在 Finder 里显示」），
在那里按命令加，并把 scope 限到两个数据根。

### 走 mock 还是走 Rust

`src/api/index.ts` 的判据是**运行时探测** `'__TAURI_INTERNALS__' in window`，
不是构建期的 `import.meta.env.DEV`。区别很实际：`npm run tauri dev` 是 dev 构建且跑在原生窗口里 ——
按 DEV 判会走 mock，于是整个开发期都碰不到真实链路，等打包那一刻才第一次接通，那时候的问题最难查。

浏览器直接开 5321 时探测不到 → 走 mock，调试路径不丢。

## 7. 错误与 trace

`AppError { code, message, traceId, detail? }`，八个 code：`NOT_FOUND` /
`PERMISSION_DENIED` / `INVALID_ARGUMENT` / `CORRUPTED` / `SHA_MISMATCH` / `IO` /
`NOT_IMPLEMENTED` / `INTERNAL`。

- `message` 是能直接显示的中文；技术细节（io error 原文、路径）进 `detail`；
- `traceId` 由 command 包装层生成（uuid v7），同时进 `info_span!` 与错误对象 —— 界面上显示的那个
  id 与日志里的是同一个值，按 id 能把一次调用的全过程捞出来；
- 前端 `bridge.ts` 的 `normalizeError` 兜住三类"不长这样"的 reject（command 没注册、
  参数反序列化失败、panic）：一律 `INTERNAL` / `NOT_IMPLEMENTED` + `traceId: '-'`，原值进 `detail`。
  `-` 的含义是"这条错误没经过 Rust 的包装层，日志里查不到"。

Rust 侧的字段名与 TS 那份靠 `error.rs` 里的单元测试钉住 —— 两份声明之间没有编译器。

日志：`tracing` + `tracing-appender` 按天滚动，写进 `internal_root/logs/`；dev 下同时输出 stderr。
日志目录不可用时退到只写 stderr，**不阻断启动**。用 `SUPPORTEASE_LOG` 环境变量调级别。

## 7.5 窗口外观（macOS）：三个坑

原生圆角与原生红绿灯都不是 CSS 能做的，`src-tauri/src/chrome.rs` 里那两个函数各对应一个坑。
做法参考 `neteasemusicc`（同机另一个项目，这两点上是对的）。

**① `trafficLightPosition` 在 `titleBarStyle: "Overlay"` 下不生效。**
Overlay 只给窗口加了 `fullSizeContentView`，**没有**真正的 `NSToolbar`。AppKit 摆灯有两档：

| 配置 | 标题栏高 | close 圆心 | min | zoom |
| --- | --- | --- | --- | --- |
| 只有 fullSizeContentView | 32 | (16, 16) | (39, 16) | (62, 16) |
| 带 `NSToolbar` `.unified` | 52 | (26, 26) | (49, 26) | (72, 26) |

所以装一个**空** `NSToolbar` + `NSWindowToolbarStyle::Unified`，把位置算回给系统；
`--titlebar-h` 必须是 52 灯才竖直居中，左侧占位 77px（AppKit 量出来的：灯组右缘 79、
第一个 toolbar item 左缘 97）。不要逐颗改 `standardWindowButton(...).frame` ——
resize / 进出全屏 / 切外观时 AppKit 都会重摆，手动补摆每次都有一帧错位。

**② 圆角是 tauri#14165。** 窗口的原生边框是圆的，但 WKWebView 没被裁 ——
它把内容画到圆角外面，看上去就是直角。`windowEffects.radius` 只圆了底下那层
NSVisualEffectView。解法是 `window-vibrancy` 的 `apply_liquid_glass(...).content_view(webview)`：
把 webview 摘下来挂进玻璃视图，**圆角由 AppKit 裁**。必须 `opaque(true)`（系统菜单与
系统窗口都是"不透明底 + 一层玻璃"，传 false 会让背后窗口的内容读进来）+ 窗口
`transparent: true` + `macOSPrivateApi: true`。低于 macOS 26 时 crate 返回
`UnsupportedPlatformVersion`，那时只打日志、**不退回毛玻璃**（毛玻璃会让界面变半透明，
比直角更糟）。

**③ 标题栏拖动与改大小由前端自己接管，不用 `data-tauri-drag-region`。**
Tauri 注入的 `drag.js` 在 Windows 上第一次 `mousedown`（`detail === 1`）就立刻发
`start_dragging`，系统随即进入模态拖窗循环，WebView2 收不到那次的 `mouseup`——
Chromium 的连击计数被打断，双击最大化要点得极快才触发。
`useTitlebarDrag` 把 `startDragging` 推迟到指针真的移动 4px 之后：
没移动就什么都不发，点击序列完整闭合，原生 `dblclick` 正常触发，
双击间隔等于系统的 `GetDoubleClickTime()`（和其它软件一致）。
`decorations: false` 之后系统的 resize 边框在可见窗口**之外** 8px，
`ResizeEdges` 在窗口**内侧**补一圈 5px 命中区（角 12px），
`mousedown` 调 `startResizeDragging(dir)`，两圈一夹，
抓取带跨在可见边界上。两个 API 都需要各自的 ACL 权限
（`allow-start-dragging` / `allow-start-resize-dragging`）。

## 8. 前端约定

- **密度档**由容器宽度决定（`useDensity`），四档 `mini / compact / wide / ultra`，
  切的是 CSS 变量，不是两套布局；
- **大图的尺寸与位置**由窗口宽高经固化曲线算出（`src/app/heroCurves.ts`）。
  那套值是在试验场（`mkp-adaptive-console`）用可视化曲线编辑器调出来的，产品仓只存结果；
  要改回试验场调，别在这里手改数字；
- **滚动条**统一在 `global.css` 里做成零占位（"幽灵滚动条"），组件里不要覆盖；
  Stylelint 有一条规则专门拦这件事；
- `overflow` 不许写成内联 style（ESLint 的 `no-restricted-syntax` 拦），否则绕过 Stylelint 检查。

## 9. 本轮明确没做

- 预设管理、参数页、设置页、报告页 —— 四个页签在，内容是占位页（`PagePlaceholder`）；
- 标题栏窗口按钮（最小化 / 最大化 / 关闭）已接通真实 IPC（`src/app/window.ts`），
  Windows 上走自绘撑满式 caption button（`TopTabs`），macOS 上由系统交通灯接管。
  Snap Layouts（悬停最大化键弹 Win11 布局菜单）尚未做，需要 Rust 侧接 `WM_NCHITTEST`；
- 云端同步、SHA 校验、归档 —— 产品规则已经定稿（见 `PRESET-PRODUCT-RULES.md`），实现未开始；
- 撤销重做栈：不做通用 command pattern，将来在需要的地方用「编辑前快照 + 单层撤销」。

## 10. 版本身份与文件命名规范

来源：`.comate/specs/b05-content-pipeline-charter/doc.md` §6。这一节是**唯一权威**，
工作台、构建器、发布器、消费端四方共用。

### 10.1 身份与文件名是两件事

| 概念 | 是什么 | 谁拥有 | 能不能变 |
| --- | --- | --- | --- |
| **内部身份** | 机型 id + 版本 id（`A1` + `FASTV3.3`） | 源定义文件 | 稳定。改它等于换一个业务对象 |
| **外部文件名** | 产物与交付文件叫什么 | **唯一的命名函数** | 可以调整，因为它是算出来的 |

**不变式：身份稳定，文件名可算。**
文件名不承担业务身份，不承担版本关系，不承担消费端寻址之外的任何职责。

### 10.2 规则

```text
文件名 = <机型 id> + "-" + <版本 id 转小写> + ".toml"
```

| 项 | 定 | 为什么 |
| --- | --- | --- |
| 分量 | 只有机型 id 与版本 id | 多一个分量就等于把关系塞进文件名 |
| 分隔符 | `-` | 机型 id 本身含 `_`（`A1_MINI`），用 `_` 会歧义 |
| 大小写 | 身份保持原样，文件名里**版本 id 转小写**；机型 id 不动 | 转换只在命名函数里发生一次 |
| 扩展名 | `.toml` | —— |
| 方向 | **单向可算**：身份 → 文件名 | 反向查找走目录 JSON，不解析文件名 |

例：`A1` + `FASTV3.3` → `A1-fastv3.3.toml`；`A1_MINI` + `STANDARD` → `A1_MINI-standard.toml`。

### 10.3 必须覆盖的五个使用点

版本定义 · 参数源关联 · 构建产物 · 消费端查找 · 发布文件名。
**五处共用同一个函数**，不许任何一处自己拼字符串。

### 10.4 大小写带来的唯一性约束

文件名里版本 id 转小写，所以**仅大小写不同的 id 会塌成同一个文件**。

因此唯一性检查必须是**大小写不敏感**的，覆盖四类：机型 id（全局）、版本 id（机型内）、
资产 id（全局）、套餐 id（全局）。只比字面量会放过 `Fast` 与 `FAST` 这种冲突，
而冲突的表现不是报错，是后写的产物**静默覆盖**先写的。

### 10.5 历史包袱：曾经有三套命名（已收口）

| 套 | 样本 | 曾经在哪 | 现状 |
| --- | --- | --- | --- |
| A 产物命名 | `A1-fastv3.3.toml` | 入库产物目录、`BUILTIN_PRESETS` | 标准 |
| B 云端历史命名 | `A1F_260628.toml` | 对照基线目录、机型定义的 `presetFile`、IR 夹具 | 已从**基线夹具**消失（b05 Task 5）；`presetFile` 待删（G-2）；旧仓与用户目录里仍是这些名字 |
| C 消费端期望 | `A1-fastv3.3.toml` | 发布路径（多做一次 `to_lowercase()`） | 标准，且与 A **合并成同一份实现**（b05 Task 2） |

A 与 C 形状相同但曾是两份独立实现，B 与 C 对不上——**后果不是报错，是消费端找不到文件**。
本规范取 A/C 的形状作为唯一标准：现在只有 `preset::preset_file_name` 一处实现
（`src-tauri` 侧转调它），对照基线与入库产物**同名**，配对按名字直配、不再读文件头。

B 只作为一次性迁移输入：旧名到新身份的映射表在
`.comate/specs/b05-content-pipeline-charter/migration-map.md`，**迁移完成后即失效**，
不进入任何长期代码路径。B 套名字怎么读（`M`=mini、`F`=快拆、`_260628`=开源版日期）
也记在那份映射表里，那是历史解释，不是规则。

### 10.6 对照基线的维护：它是判据资产，不是产物副本

`crates/postprocess/tests/fixtures/presets/*.toml`（9 份）是**判据资产**：
它守的不是「数值对」（那是 K-G7 的事），而是「**没有未经审阅的变化**」。
它和入库产物看起来同名同内容，但职责不同，**不合并它们** ——
合了就等于让产物自己给自己当期望值，判据从「证明」退化成「自比自」。

| 项 | 定 |
| --- | --- |
| 是什么 | 上一次人工审阅通过的样子。**不是**产物的副本，**不是**交付文件 |
| 谁可改 | 只有**显式同步**一个动作：`gen-presets --sync-baseline`。它要求人先看过差异（`--baseline` 或 `git diff`） |
| 何时更新 | 只在「本次变更确实**预期**产物变化」时。它既不是生成/发布的自动步骤，也不是每次发布的必经步骤 |
| 生成与发布路径 | **不写基线目录**。判据两条：`write_discipline_scan.rs::the_baseline_has_exactly_one_write_path`（源码扫描 —— 调用点只有那一个 CLI）与 `tests/baseline_stays_untouched_on_the_generate_path.rs`（真跑一遍生成/检查路径，基线逐份 sha256 不变） |
| 落点闸 | `generate::check_baseline_target`：只允许真基线目录或系统临时目录，别处一律拒绝 |
| 删除后 | **不可自动恢复** —— 「恢复」等于无条件接受当前值，安全网就废了（doc §3.8） |
| 工作台里若给入口（**尚未做**） | 必须是**显式、带 diff 确认的独立入口**，不出现在「新增版本」的必经步骤里（doc §4.3 第 10 步） |

两条判据各堵一半，缺一条都留口子：源码扫描证明**没有别的调用点**，
运行时那条证明**真跑起来确实没动**（「没有调用」这件事本身没有运行时表现）。

