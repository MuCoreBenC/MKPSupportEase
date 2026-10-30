# SupportEase

MKP 打印机的桌面辅助：选机型 → 确认偏移 → 校准 Z / XY → 打测试件，外加预设管理、
切片器后处理与执行报告。**离线工具，不联机控制打印机** —— 唯一的出站网络是「检查更新 /
下载预设」，而且出站只住 Rust 的 `runtime/net.rs`（前端不做任何 fetch）。

技术栈：Tauri 2 + React 18 + TypeScript（Vite）+ Rust workspace
（`src-tauri` 客户端内核 / `crates/postprocess` 后处理内核 / `crates/preset` 预设解析）。

## 一个程序，三种身份

| 身份 | 怎么进来 | 干什么 |
| --- | --- | --- |
| 客户端 | 双击应用（`index.html`） | 用户界面：六页 |
| 后处理钩子 | 切片器按注册的脚本拉起：`SupportEase --Toml <预设> --Gcode <件>` | 不开第二个进程，把这一趟挂到界面那个常驻进程上跑（进度模态框 + 停止），跑完按退出码退 |
| 工作台 | `workbench.html`，只在 `--mode workbench` 构建里（`check:bundle` 盯着它不进安装包） | 开发工具：机型 / 参数 / 套餐 / 资产的编辑、标注台、生成与发布闸、沙箱、真实后处理矩阵 |

可执行物**就是主程序自己**：首页「复制后处理脚本」由后端 `get_post_process_command` 现拼
（可执行物 = `current_exe()`，预设 = 那台机器上真实的绝对落点），前端不拼任何一段路径。

## 客户端六页

| 页 | 干什么 |
| --- | --- |
| 首页 | 选机型 → 偏移 → 校准 → 测试件，五步向导与总览（落地页） |
| 预设 | 云端 / 本地两轴：下载、应用、逐参数对比与更新、改并写回、改名 / 另存 / 删除（进回收站） |
| 校准 | Z / XY / 测试模型三套流程（自绘标定板，坐标来自 `src/calib/*.generated.ts`） |
| 工艺 | Bambu Studio 工艺预设查看器（页签叫「工艺」，路由 id 仍是 `bbs`） |
| 报告 | 执行归档：每次后处理的记录 + 字段面 + 导出 |
| 设置 | 软件更新 / 外观与界面语言 / 切片器 / 引导图 / 执行记录清理 / 备份与恢复 / 高级（数据源后门） |

页签只有这六条：「同步」与「参数」两页已退役（前者降级成设置页里的后门，后者搬进预设行右键）。

## 后处理：切片器导出时的那一趟

```text
切片器导出 G-code
  → 按注册的脚本拉起本程序（--Toml <预设> --Gcode <件>，大小写敏感）
  → crates/postprocess（bin `mkpse-pp`）跑 12 步管线，产物原地覆盖（.part → rename 原子替换）
  → 每跑一次落三件套：gcode_history/<日期>/<名>_{_original.gcode, .gcode, _meta.json}
  → 退出码是对外契约：0 成功 / 1 处理失败或用户取消 / 2 输入或预设错
```

- **stderr 只写结论**：macOS 上切片器把整段塞进它的错误对话框，多写一行就把有用的那行推下去；
  Windows 上它根本不接管道，只显示命令与 `Error code: N`（见 `docs/HOOK-ERRORS-AND-MESSAGES.md`）。
- **校准件身份唯一来源 = 原始 3MF 的 `MKPSE:Calibration`**（不扫 G-code 注释）。管线只消费不判定。
- 撞塔那一问住内核第 6 步；用户可见的句子一律住 `crates/postprocess/messages.toml`（每句带码）。

## 预设：一份 TOML 从仓库到客户端

三层生成物**都在 git 里**（客户端按 URL 直取，`.gitignore` 挡不住，只能靠纪律）：

```text
presets/…（源：机型 / 参数注册表 / 套餐 / 资产）
  → crates/preset/assets/presets/*.toml + src-tauri/src/runtime/catalog.generated.json
      （include_str! 编进客户端：随包预设解析的就是它）
  → presets/delivery/**（catalog / manifest / 产物；客户端 Bootstrap 读它）
```

三件事别混：

| 动作 | 命令 | 推进 `publishSerial` |
| --- | --- | --- |
| 改源 TOML | 直接改 | 否 |
| 生成交付产物 | `npm run presets -- generate` | 否 |
| 正式发布 | `npm run presets -- publish` | **是（唯一一处）** |

**客户端存储只存一份原文**：`<appDataDir>/content/<sha256>.toml`，同 SHA 多路径共享；
`run/delivery-index.json` 记「当前目录里每个 path 用哪一版」—— 换代只换索引，旧版因此不会丢。
删除进 `recovery/<id>/`，清理按引用。

**要不要更新**由四问答，缺一不能编：`catalogId`（哪条发布线）· `ref`（换分支各数各的号，不比）·
`publishSerial`（有没有往前走）· 文件 SHA-256（内容变没变）。唯一实现 `runtime::local::based_on`。

## 仓库结构

```text
src/                 客户端前端
├── api/             与后端唯一的约定（contract.ts / bridge.ts / index.ts 运行时探测）
├── app/             六页 + 导入入口 + 通知 + 状态（home presets calib bbs report settings）
├── workbench/       工作台（只进 workbench 构建）
├── components/      共用件（field / check …）
├── i18n/            前端文案字典（zh-CN / en，闸门 `check:i18n`）
├── calib/           标定板生成产物（.generated.ts，别手改）
└── styles/          tokens.css + global.css

src-tauri/src/       客户端内核：fsx（原子写 / 路径）/ runtime（数据世界）/ ipc（命令面）/
                     hook*（后处理钩子）/ archive（执行归档）/ obs（日志）/ workbench（工作台后端）
crates/postprocess/  后处理内核 + `mkpse-pp` 命令行（可独立诊断：init / dump-ir / check）
crates/preset/       预设解析与写回（保真改 TOML、参数注册表、血统）
presets/             预设源 + assets/ + delivery/（生成产物）
docs/                现行规则 + 记录 + 归档
```

## 跑起来

```bash
npm install          # 顺带装好 git 闸门（prepare → scripts/setup-hooks.mjs）
npm run tauri dev    # 原生窗口，数据来自 Rust
npm run dev          # 只开浏览器 http://localhost:5321/，数据走 mock
npm run tauri:workbench:dev   # 工作台（另一份 identifier、另一套数据根）
```

浏览器 / 原生两条路都留着是刻意的：浏览器里改界面快，原生窗口里才碰得到真实的 IPC 链路；
走哪一份由运行时探测（`src/api/index.ts`）决定，不是构建期开关。

其他常用命令：

```bash
npm run presets -- check | diff | generate | verify | publish [--dry-run]
npm run lint         # eslint + stylelint
npm run build        # tsc -b + vite build
npm run release      # 发版：校验 → 开 PR → 等 CI → squash 合并 → 打 tag
cd src-tauri && cargo test
```

**故障处置**：客户端日志在 `<appDataDir>/logs/`（界面一条、钩子单开 `hook.log.*` 一条）；
钩子跑的那一趟还能在报告页里看回放，产物就在 `gcode_history/` 里。

## 验证（推之前全绿）

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features workbench -- -D warnings
cargo test                                   # 全 workspace
cargo test -p mkp-support-ease --features workbench --lib
npm run build && npx tsc -b --force && npm run lint
npm run check:i18n && npm run check:bundle && npm run check:artifacts
```

只改前端时 Rust 那几道可略（写明即可）。`check:artifacts` 是三层生成物的边界闸
（pre-push 与 CI 各一次）：交付目录变了账本没变 / 内核产物变了源没变 / 该忽略的路径混进来，
三类都拦。

## 三条不许破的

1. **发布元数据不许手拼**：`catalog.json` / `manifest.json` / `publish-serial.json` 只能由
   `presets generate`（沿用）与 `presets publish`（分配）产生。
2. **改了源就要走验证**：`check → diff → generate → verify`，不许"改完就宣布完成"。
3. **数据归属不另立一套**：哪个文件属于哪一层、谁是唯一主人、什么时候允许联网，
   以 `docs/DATA-ARCHITECTURE.md` 为准（新文件先答准入问句再写代码）。

`main` 只能由 PR 推进：本地闸②会拦 `push origin main`，绕过本地也会被 GitHub ruleset 拒。
仓库是 public 的 —— **任何密钥、用户数据都不能进这个仓库**。

## 文档地图

| 文档 | 管什么 |
| --- | --- |
| [`AGENTS.md`](AGENTS.md) | **铁律**（最短、必须遵守）：权威源、三层生成物、序号口径、发布前必看 |
| [`docs/PRESET-WORKFLOW.md`](docs/PRESET-WORKFLOW.md) | 预设怎么做：命令、状态顺序、失败恢复、验收五项 |
| [`docs/DATA-ARCHITECTURE.md`](docs/DATA-ARCHITECTURE.md) | **根规则**：四层世界 + 四铁律 + 十问 + 准入问句 |
| [`docs/PRESET-PRODUCT-RULES.md`](docs/PRESET-PRODUCT-RULES.md) | 产品规则：本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 / 状态流转 |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | 工程约束：目录、两层数据根、IPC 契约、错误与 trace、原子写、权限边界 |
| [`docs/PUBLISH-ARCHITECTURE.md`](docs/PUBLISH-ARCHITECTURE.md) | 发布链：唯一源 → 发布根 → 客户端消费 |
| [`docs/RELEASE-TRANSACTIONS.md`](docs/RELEASE-TRANSACTIONS.md) | 发布事务合同：发布预设 ≠ 发布软件版本 |
| [`docs/APP-STATE.md`](docs/APP-STATE.md) | 持久化地基：一个应用一个 AppState |
| [`docs/GIT-WORKFLOW.md`](docs/GIT-WORKFLOW.md) | git 纪律：本地八道闸 + 服务端 ruleset + 发版 |
| [`HANDOFF.md`](HANDOFF.md) | **活台账**：现在在哪、接下来去哪、未结账清单 |
| [`docs/DOC-INDEX.md`](docs/DOC-INDEX.md) | 全仓文档索引（哪份现行、哪份历史） |
| [`docs/I18N-MAP.md`](docs/I18N-MAP.md) · [`docs/HOOK-ERRORS-AND-MESSAGES.md`](docs/HOOK-ERRORS-AND-MESSAGES.md) | 两套文案字典：前端 `src/i18n`、后端 `messages.toml` 与错误码 |
| [`docs/3D-ASSET-CONTRACT.md`](docs/3D-ASSET-CONTRACT.md) · [`docs/DESIGN-SPACING.md`](docs/DESIGN-SPACING.md) | 上游 3D 资产契约 / 间距与动画原则 |

带日期的那些（`docs/*-2026-*.md`）是过程记录与裁决现场，**只读**；更早的审计与对账
（`PROJECT-AUDIT.md`、`DATA-INVENTORY.md`、`audit-current-state/`）是快照，别当现状。

## 界面是从哪来的

视觉整体移植自试验场 [`mkp-adaptive-console`](https://github.com/MuCoreBenC/mkp-adaptive-console)：
露出卡的 peek 斜坡在 JS 算、平面层常驻合成层、clamp 连续缩放、像素对齐落位 —— 那些效果是
大量小细节堆出来的，重写必然丢细节。大图尺寸与微移曲线要改就回试验场调，调完把固化值换进
`src/app/heroCurves.ts`（产品仓只存固化值，没有编辑器）。

## 许可证

[AGPL-3.0-only](LICENSE)。与 Bambu Studio / PrusaSlicer 同一条血脉的传染性协议：
基于本仓库改出来的东西再分发（包括作为网络服务提供）必须同样开源。
