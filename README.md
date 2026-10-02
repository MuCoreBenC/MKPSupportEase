# SupportEase

MKP 支撑辅助的桌面端：选机型 → 确认偏移 → 校准 Z / XY → 打测试件。离线工具，不联机控制打印机
（唯一的出站网络是"检查更新 / 下载"，见数据总纲）。

Tauri 2 + React 18 + TypeScript。**预设这一条生命周期已经收口**：官方线（下载 / 状态 /
批量 / 归档 / 可信度）与用户线（能读能列 / 能应用 / 能改并写回 / 改名 / 删除 / 另存 /
导入 / 在 Finder 中显示）都通了，导入入口与下载管道走同一个数据世界（Catalog + Source +
Delivery，底账全在 Internal 根，localStorage 不再住任何底账）。首页向导、预设页、校准页、
参数页、BBS 查看器、设置页（最小版：高级设置 → 预设数据源）都是真数据流；
**报告页还是空态**（`PagePlaceholder`，明说"本版未接入"）。
**「同步」页已退役**（2026-10-02 作者裁决）：普通用户不需要"同步"这个概念 ——
catalog 随包走、更新是内部机制；数据源配置降级成设置页里的开发后门。
落地现状全景（含还没做的）见 [`docs/PROJECT-AUDIT.md`](docs/PROJECT-AUDIT.md)。

## 跑起来

```bash
npm install          # 顺带装好 git 闸门（prepare → scripts/setup-hooks.mjs）
npm run tauri dev    # 原生窗口，数据来自 Rust
npm run dev          # 只开浏览器 http://localhost:5321/，数据走 mock
```

两条路径都留着是刻意的：浏览器里改界面快，原生窗口里才碰得到真实的 IPC 链路。
走哪一份由运行时探测决定（`src/api/index.ts`），不是构建期开关。

其他命令：

```bash
npm run lint         # eslint + stylelint
npm run build        # tsc -b + vite build
npm run release      # 发版：校验 → 开 PR → 等 CI → squash 合并 → 打 tag
cd src-tauri && cargo test && cargo clippy -- -D warnings
```

## 文档地图

| 文档 | 管什么 |
| --- | --- |
| [`docs/DATA-ARCHITECTURE.md`](docs/DATA-ARCHITECTURE.md) | **根规则**：四层世界 + 四条铁律 + 十问 + 准入问句（每个文件属于哪一层、谁是唯一主人） |
| [`docs/PRESET-PRODUCT-RULES.md`](docs/PRESET-PRODUCT-RULES.md) | 产品规则：本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 / 状态流转 |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | 工程约束：目录结构、两层数据根、IPC 契约、错误与 trace、原子写、权限边界 |
| [`docs/PROJECT-AUDIT.md`](docs/PROJECT-AUDIT.md) | **落地现状总盘点**：已完成 / 半完成 / 未开始 / 已废弃 四类（接手前先读） |
| [`docs/DATA-INVENTORY.md`](docs/DATA-INVENTORY.md) | 对账单：现有代码与文件按总纲逐件归位 + 收口进展日志 |
| [`HANDOFF.md`](HANDOFF.md) | 交接：现在在哪、接下来去哪、纪律与判据清单 |
| [`docs/GIT-WORKFLOW.md`](docs/GIT-WORKFLOW.md) | 八道本地闸 + 服务端 ruleset + 发版流程 |

另外两份是从试验场原样搬来的：[`docs/DESIGN-SPACING.md`](docs/DESIGN-SPACING.md)（间距与动画原则）、
[`docs/3D-ASSET-CONTRACT.md`](docs/3D-ASSET-CONTRACT.md)（上游 3D 资产契约）。

## main 只能由 PR 推进

`git push origin main` 会被本地闸②拦住；绕过本地闸也会被 GitHub 的 ruleset 拒。细节见
`docs/GIT-WORKFLOW.md`。仓库是 public 的 —— private 仓库用不了 ruleset（要 GitHub Pro），
所以**任何密钥、用户数据都不能进这个仓库**。

## 界面是从哪来的

视觉来自试验场 [`mkp-adaptive-console`](https://github.com/MuCoreBenC/mkp-adaptive-console)
的第 23 稿，整体移植、不重写 —— 那些效果是大量小细节堆出来的（露出卡的 peek 斜坡在 JS 算、
平面层常驻合成层、clamp 连续缩放、像素对齐落位），重写必然丢细节。

试验场还留着调参面板与可视化曲线编辑器。**大图的尺寸与微移曲线要改就回那边调**，
调完把结果换进 `src/app/heroCurves.ts` —— 产品仓只存固化值，没有编辑器。

## 许可证

[AGPL-3.0-only](LICENSE)。与 Bambu Studio / PrusaSlicer 同一条血脉的传染性协议：
基于本仓库改出来的东西再分发（包括作为网络服务提供）必须同样开源。
