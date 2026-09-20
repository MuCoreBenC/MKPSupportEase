# SupportEase 工程约束

> **这份文档管工程：**Tauri 结构、两层数据根、IPC 契约、trace、原子写、权限边界。
> **产品行为查另一份：**`docs/PRESET-PRODUCT-RULES.md`（本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 / 状态流转）。
> git 纪律与发版流程见 `docs/GIT-WORKFLOW.md`。间距与动画原则见 `docs/DESIGN-SPACING.md`，上游 3D 资产契约见 `docs/3D-ASSET-CONTRACT.md`。

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

浏览器直接开 5178 时探测不到 → 走 mock，调试路径不丢。

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
- 标题栏那三颗窗口按钮还是**装饰**：Tauri 窗口目前带系统装饰，那三颗是从试验场搬来的假控件，
  尚未接 `getCurrentWindow().minimize()` 等真实动作；
- 云端同步、SHA 校验、归档 —— 产品规则已经定稿（见 `PRESET-PRODUCT-RULES.md`），实现未开始；
- 撤销重做栈：不做通用 command pattern，将来在需要的地方用「编辑前快照 + 单层撤销」。
