# 文案对照表（前缀 → 页面 → 文件）

> 2026-10-26 建。**这张表是索引，不是真值** —— 真值永远是 `src/i18n/zh-CN.ts`（键的类型定义）
> 与 `src/i18n/en.ts`（英文那册）。两边键集与占位符由 `npm run check:i18n` 钉着（当前 **1481** 条）。

## 一、四类「名字」，别弄混

| 类别 | 住哪 | 随语言变？ | 改动要动什么 |
| --- | --- | --- | --- |
| **界面句子**（按钮、标题、提示、回执） | `src/i18n/{zh-CN,en}.ts` | 是 | 两本字典各加一条；`check:i18n` 会核对占位符 |
| **后端句子**（错误、提问、归档里的话） | `crates/postprocess/messages.toml` | 是 | `text` + `text_en` + `params`，取用一律 `messages::text("键", &[...])` |
| **参数名 / 说明 / 分组名** | `presets/registry/param_registry.toml` 的 `labelEn` / `descEn` / `descriptionEn` | 是（数据双语） | 改数据；`every_entry_has_english` 钉着不许缺 |
| **数据本身的名字**（机型、版本、文件名、BBS 预设名） | 各自的 JSON / 目录 | **否** | 不译 —— 它是识别用的键，不是句子 |

挑选规矩只有一处：`src/app/shared/dataLabel.ts` 的 `bi` / `localizeParamDef` / `localizeTab` /
`localizeParamMeta`。**三条读取路径共用它**：参数页（`useParams`）、对比台（`CompareModal`）、
工艺页（`BbsRow` / `BbsControl` / `bbsPanel`）。别在别处再抄一份挑选逻辑 —— 2026-10-26 对比台
就是漏了这一处，英文界面上整片中文。

## 二、前缀 → 页面 → 主要文件

| 前缀 | 条数 | 页面 | 主要文件 |
| --- | ---: | --- | --- |
| `presets.` | 393 | 预设页（含对比台 / 编辑模式 / 修改历史） | `src/app/presets/*` |
| `params.` | 229 | 参数页（含修改历史、云端更新） | `src/app/params/*` |
| `report.` | 199 | 报告页 | `src/app/report/*` |
| `settings.` | 167 | 设置页（含「关于」那一节） | `src/app/settings/*` |
| `bbs.` | 146 | 工艺页（BBS） | `src/app/bbs/*` |
| `home.` | 101 | 首页 | `src/app/home/*` |
| `postproc.` | 74 | 后处理进度 / 静默档 / 通知 | `src/app/postprocess/*` |
| `calib.` | 48 | 校准页 | `src/app/calib/*` |
| `import.` | 26 | 导入 | `src/app/import/*` |
| `guide.` | 20 | 引导卡 | `src/app/guide/*` |
| `update.` | 17 | 更新检查与进度 | `src/app/components/UpdateIndicator.tsx` |
| `tabs.` | 12 | 外壳页签 | `src/app/components/TopTabs.tsx` |
| `field.` | 10 | 字段控件（数值 / 开关 / 选项） | `src/components/field/*` |
| `slicer.` | 10 | 切片器（打开方式 / 后处理栏） | `src/app/settings/*` |
| `common.` | 8 | 通用（关闭 / 取消 / 保存 / 空） | 跨页 |
| `download.` | 8 | 下载与资源区 | `src/app/presets/*`、`src/app/home/*` |
| `copyScript.` | 7 | 复制后处理脚本的通知卡 | `src/app/postprocess/copyScript.ts` |
| `zoom.` | 2 | 缩放 | `src/app/shell/*` |
| `trace.` | 2 | 诊断（traceId） | `src/app/components/TraceTag.tsx` |
| `app.` / `menu.` | 1 / 1 | 外壳 | `src/app/shell/*` |

## 三、两本字典里怎么分段

`zh-CN.ts` / `en.ts` 里插了 `// ---- 设置页 ----` 这样的**分节注释**（36 段）。
文件是逐刀追加长的，**没有重排键**（重排会让以后每处 diff 都对不上行号），所以同一个页面
可能出现好几段 —— 带「（续）」的就是它。分节注释只为人眼，不参与构建。

## 四、还欠着的

- **归档记录**（`_meta.json` 里的管线明细）：`src-tauri/src/archive.rs::step_detail` 仍是
  生成时那句话，写完就定住 —— 老记录不会复译。要做双语得像「结构化事实 + 前端渲染」那样改
  归档契约，单独一轮。
- **修改历史的 `action`**：存的是当时那句话，同一条规矩（老条目不復译）。
- `src/workbench/**`（2147 行含中文）：开发台，单排一轮，作者裁定暂不动。
