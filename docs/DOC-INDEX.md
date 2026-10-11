# 全仓文档索引（整理与待合并清单）

> 整理日期：2026-10-11 ｜ 依据：各文档**自己的文首标注**（⚠️ 历史快照 / 已被取代 / 已实施 / 施工计划…）
> ＋ 各目录盘点。本文**不做裁决、不做合并** —— 哪份并入哪份、以谁为准，由作者自己裁决；
> 没有标注可引用、由盘点判断的，行内都标了**（推断）**。
>
> **这份索引就是为了让合并有清单可勾**：冲突 / 重叠 / 重复全在 §6，合并完回来划掉一行即可。

## 0. 文档地图（一页看完）

| 群 | 位置 | 数量 | 性质 |
| --- | --- | --- | --- |
| 现行规则 | `docs/` 顶层 + 根 `README.md` + [`HANDOFF.md`](../HANDOFF.md) | 约 12 份 | 根规则 / 产品规则 / 工程约束 / 合同 |
| 记录与计划 | `docs/` 顶层（带日期的那些） | 约 22 份 | 裁决记录 / 台账 / 施工计划 / 测试协议 |
| 审计基线 | `docs/audit-current-state/` | 9 md + data/map | 重写前的只读审计（"建议，不是现状"） |
| 历史交接 | 根目录 6 份 + `docs/` 里自标历史的 | — | 早期"刀/圈"时代的交接与快照 |
| 旧工具 spec 存档 | [`.comate/specs/`](../.comate/specs/)（**入库**，49 个文件 / 13 个包） | 13 包 | 上一代工具时代的 spec（B03 / B04 / B05 / 开局地基） |
| 计划文件 | `.codebuddy/plans/`（**本机、不入库**，见 `.gitignore`） | 9 份 | 现役工具的计划（含重复与未完成） |
| 工作台本机状态 | `workbench/.draft/` `.sandbox/` `.snapshots/` `.trash/`（**本机、不入库**） | — | `.sandbox/` 里有一份仓库影子副本（见 §6 C6） |

## 1. 现行规则（先读这一组）

| 文档 | 管什么 | 状态 |
| --- | --- | --- |
| [`README.md`](../README.md) | 入口：项目一句话 / 跑法 / 文档地图 | 现行（首段"报告页还是空态"**已过期**，见 §6 C3） |
| [`docs/DATA-ARCHITECTURE.md`](DATA-ARCHITECTURE.md) | **根规则**：四层世界 + 四铁律 + 十问 + 准入问句 | 现行 |
| [`docs/PRESET-PRODUCT-RULES.md`](PRESET-PRODUCT-RULES.md) | 产品规则：本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 | 现行（正文已就地吸收 2026-10-08 三次改判，含三条 ⚠️ 注记） |
| [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) | 工程约束：目录、两层数据根、IPC、trace、原子写、权限 | 现行 |
| [`docs/GIT-WORKFLOW.md`](GIT-WORKFLOW.md) | git 纪律：本地八道闸 + 服务端 ruleset + 发版 | 现行（其中的合并动作由作者执行，AI 只提交+推送） |
| [`docs/PUBLISH-ARCHITECTURE.md`](PUBLISH-ARCHITECTURE.md) | **发布链根规则**：唯一源 → 发布根 → 客户端消费 | 现行（自标：替代 `PRESET-DELIVERY-CONTRACT` 的旧裁决） |
| [`docs/RELEASE-TRANSACTIONS.md`](RELEASE-TRANSACTIONS.md) | 发布事务合同：发布预设 ≠ 发布软件版本 | 现行（自标：改它要作者点头） |
| [`docs/APP-STATE.md`](APP-STATE.md) | 持久化状态地基：一个应用一个 AppState | 现行（10-08 盘点定案已并回本文 §5 / §10） |
| [`docs/DESIGN-SPACING.md`](DESIGN-SPACING.md) | 间距与动画原则（试验场搬来） | 现行 |
| [`docs/3D-ASSET-CONTRACT.md`](3D-ASSET-CONTRACT.md) | 上游 3D 资产契约 v3（对外） | 现行（自标：contract v3 取代 v2） |
| [`HANDOFF.md`](../HANDOFF.md) | **进度台账**：现在在哪、接下来去哪 | 现行·持续追加（最新到"增量之三十四"，随本分支提交；头部两条 ⚠️ 注记说明早期章节已过时） |
| [`docs/RESOURCE-ADDRESSING-ROADMAP.md`](RESOURCE-ADDRESSING-ROADMAP.md) | 寻址改造总纲（长期大纲） | 半现行：自标"大纲已批准 2026-10-05，施工进度以 §8 台账为准" |
| [`docs/PROJECT-AUDIT.md`](PROJECT-AUDIT.md) | 项目总盘点：已定义但未落地（四类） | 参考：2026-10-02 的对账，时效以日期计（README 曾写"接手前先读"） |
| [`docs/DATA-INVENTORY.md`](DATA-INVENTORY.md) | 总纲的对账单（43 件归位 + 收口日志） | 对账单：基准 2026-10-01 `58f4295`，收口一条勾一条（是否勾完没终检） |

## 2. 记录与计划（docs/ 顶层带日期的那些）

| 文档 | 日期 | 一句话 | 状态（自标优先） |
| --- | --- | --- | --- |
| [`A05-PORT-LEDGER.md`](A05-PORT-LEDGER.md) | 2026-10-11 | a05 前端整份搬运台账（源 = frontend-lab） | 现行（本分支新增，对应"前端照搬 a05"提交） |
| [`REPORT-DETAIL-FIELDS.md`](REPORT-DETAIL-FIELDS.md) | 2026-10-11 | 报告页字段清单（照搬 frontend-lab spec + 本仓口径两条） | 现行（本分支新增） |
| [`PRESET-ASSET-LIBRARY.md`](PRESET-ASSET-LIBRARY.md) | 2026-10-08 | 预设资产库（一张表 · 来源降为轻标记） | 10-08 第三次改判（同日三次里的最后一次） |
| [`PRESET-WORKING-COPY-2026-10-08.md`](PRESET-WORKING-COPY-2026-10-08.md) | 2026-10-08 | 下载即得工作副本 | **已被同日第三次改判退役**（"下载不再自动产生副本"，见 `PRESET-PRODUCT-RULES.md` ⚠️ 三条） |
| [`PRESET-MULTI-VERSION.md`](PRESET-MULTI-VERSION.md) | 2026-10-08 | 多版本「我的预设」+ 参数对比台 | **已被取代**（文首自标：被 `PRESET-ASSET-LIBRARY.md` 改判） |
| [`PRESET-LIST-UX-ISSUES-2026-10-08.md`](PRESET-LIST-UX-ISSUES-2026-10-08.md) | 2026-10-08 | 预设列表问题清单 | **已闭环**（文首自标：模型已定并已实施，指向资产库） |
| [`PRESET-COPY-LINEAGE.md`](PRESET-COPY-LINEAGE.md) | 2026-10-06 | 复制与身世（另存放开 · 归属继承）设计稿 | 部分已被 10-08 改判取代（自标） |
| [`PRESET-TIME-SEMANTICS.md`](PRESET-TIME-SEMANTICS.md) | 2026-10-06 | 时间语义与删除策略梳理 | 已升级（自标：改定义为事件模型 → 见下一行） |
| [`PRESET-EVENT-TIME-MODEL.md`](PRESET-EVENT-TIME-MODEL.md) | 2026-10-06 | 预设事件时间模型（设计与实现） | 该主题的落地版（推断） |
| [`REMARK-SUBTITLE-2026-10-07.md`](REMARK-SUBTITLE-2026-10-07.md) | 2026-10-07 | 副标题 = 备注 + 复制件归属 | 已实施（自标：台账与代码同一 commit） |
| [`SLICER-PRESET-AXES-2026-10-07.md`](SLICER-PRESET-AXES-2026-10-07.md) | 2026-10-07 | 切片器档喷嘴/层高 bug + 陈旧数据核实 | 已修（自标：两件都是 bug，都已修） |
| [`RUST-LINT-CLEANUP.md`](RUST-LINT-CLEANUP.md) | 2026-10-07 | 增量之三十：Rust 工具链 / 纪律清理 | 已施工完成（自标） |
| [`HOME-BUNDLE-DOWNLOAD.md`](HOME-BUNDLE-DOWNLOAD.md) | 2026-10-06 | 首页「下载并应用」按套餐消费（施工计划） | 对应改动已进 main（#52「首页按套餐下载」）——**回收/标完结由你定**（推断） |
| [`UX-SCENARIO-TEST.md`](UX-SCENARIO-TEST.md) | 2026-10-06 | 用户故事测试协议（"人用起来顺不顺"） | 长期协议（可复用） |
| [`UX-SCENARIO-FINDINGS-2026-10-06.md`](UX-SCENARIO-FINDINGS-2026-10-06.md) | 2026-10-06 | 首轮黑盒发现（含原样报告附录） | 记录（那一轮的现场） |
| [`UX-FIX-PLAN-2026-10-06.md`](UX-FIX-PLAN-2026-10-06.md) | 2026-10-06 | 修复清单（只修三个 A 级） | 已完结（文内三轮记录 A1/A2/A3 全 ✅） |
| [`APPSTATE-INVENTORY-2026-10-08.md`](APPSTATE-INVENTORY-2026-10-08.md) | 2026-10-08 | 全仓持久化落点盘点（AppState 收编清单） | 记录（自标：定案已并回 `APP-STATE.md`；本轮零收编） |
| [`HANDOFF-FRONTEND-CLEANUP-2026-10-09.md`](HANDOFF-FRONTEND-CLEANUP-2026-10-09.md) | 2026-10-09 | 前端清理执行清单（用户故事口径） | 执行清单：后续"前端照搬 a05 四页 + 皮肤整份替换"与其口径是否冲突，**待你判**（推断） |
| [`TWO-REPO-ALIGNMENT.md`](TWO-REPO-ALIGNMENT.md) | 2026-10-02 | 双仓对齐报告（本仓 vs 试验场） | ⚠️ 历史快照（自标，2026-10-05 加注） |
| [`PRESET-DELIVERY-CONTRACT.md`](PRESET-DELIVERY-CONTRACT.md) | — | 预设交付契约（问题档案） | ⚠️ **部分已被取代**（自标：正式规则搬进 `PUBLISH-ARCHITECTURE.md`） |
| [`SupportEase-%E9%A2%84%E8%AE%BE%E5%AE%98%E6%96%B9%E6%9B%B4%E6%96%B0%E4%B8%8E%E7%94%A8%E6%88%B7%E8%87%AA%E5%AE%9A%E4%B9%89%E8%A7%84%E5%88%99.md`](SupportEase-%E9%A2%84%E8%AE%BE%E5%AE%98%E6%96%B9%E6%9B%B4%E6%96%B0%E4%B8%8E%E7%94%A8%E6%88%B7%E8%87%AA%E5%AE%9A%E4%B9%89%E8%A7%84%E5%88%99.md) | — | HTML Demo 时代的"官方更新与自定义规则"整理 | 历史（自标：根据当前 HTML Demo 的实际代码整理）——文件名还是 URL 编码的，一并由你处置 |
| [`supportease-preset-v3.html`](supportease-preset-v3.html) | — | 预设 v3 原型页（HTML） | 素材（原型） |

## 3. 审计与重写基线（`docs/audit-current-state/`）

只读审计（自标："本次没有修改任何产品源码"；"建议不是现状"）。是前端清理清单的事实依据；部分已实施、部分未。

| 文件 | 一句话 |
| --- | --- |
| [`README.md`](audit-current-state/README.md) | 审计基线与读法（说的是哪一版） |
| [`01-product-spec.md`](audit-current-state/01-product-spec.md) | 交付物 A：产品规格层（逐页逐动作） |
| [`02-domain-state.md`](audit-current-state/02-domain-state.md) | 交付物 B：领域与状态层（还原真实数据模型） |
| [`03-contract.md`](audit-current-state/03-contract.md) | 交付物 C：契约层（命令面 / DTO / 前后端边界） |
| [`04-persistence.md`](audit-current-state/04-persistence.md) | 副作用、持久化与一致性审计 |
| [`05-cloud-publish.md`](audit-current-state/05-cloud-publish.md) | 云端、官方源与发布链路核对 |
| [`06-complexity-findings.md`](audit-current-state/06-complexity-findings.md) | 已确认问题 18 条 + 复杂性来源 |
| [`rewrite-decisions.md`](audit-current-state/rewrite-decisions.md) | 交付物 D：重写决策清单（RD-01 ~ RD-20） |
| [`rewrite-roadmap.md`](audit-current-state/rewrite-roadmap.md) | 交付物 E：重写路线图（AC-01 ~ AC-12） |
| `data/*.json`、`map/`、`index.html` | 证据数据与审计浏览页 |

## 4. 历史交接（根目录的"刀 / 圈"时代文档）

| 文档 | 日期 | 一句话 | 状态（自标优先） |
| --- | --- | --- | --- |
| [`PUBLISH-KNIFE3-HANDOFF.md`](../PUBLISH-KNIFE3-HANDOFF.md) | — | 第三刀交接（发布链：结构代次 / 软件更新 / 不兼容提示） | ⚠️ 历史快照（自标） |
| [`PUBLISH-KNIFE4-HANDOFF.md`](../PUBLISH-KNIFE4-HANDOFF.md) | — | 第四刀交接（发布软件版本：版本号 / tag / 安装包 / Release） | ⚠️ 历史快照（自标） |
| [`HANDOFF-5-APP-UPDATE.md`](../HANDOFF-5-APP-UPDATE.md) | 2026-10-05 | 第五刀交接（应用内更新：标题栏图标 + 环形进度 + 重启安装） | 交接（未标历史；时效以现状为准） |
| [`ASSET-CHAIN-HANDOFF.md`](../ASSET-CHAIN-HANDOFF.md) | 2026-10-03 | 资产交付链与图位分层交接 | 交接（其成果后来以"增量之八~十一"落进 `HANDOFF.md`） |
| [`C14-PORT-PLAN.md`](../C14-PORT-PLAN.md) | — | 工作台 C14 → 产品仓移植方案 | 计划（状态见文内；与 `C15-A40-PORT-PLAN.md` 同一线） |
| [`C15-A40-PORT-PLAN.md`](../C15-A40-PORT-PLAN.md) | — | 客户端 A21→A40 补齐 · 工作台 C14→C15 增量方案 | 计划（同上） |

## 5. 代码旁注小文档（跟着代码走，不参与合并）

| 文档 | 管什么 |
| --- | --- |
| [`crates/postprocess/tests/reference/README.md`](../crates/postprocess/tests/reference/README.md) | tests/reference 与 fixtures 的证据等级声明 |
| [`public/bbs/PROVENANCE.md`](../public/bbs/PROVENANCE.md) | public/bbs 四份 json 的来源与许可 |
| [`scripts/preset-test-server/README.md`](../scripts/preset-test-server/README.md) | 本地官方源测试服务（把云端源指向本机） |

## 6. 冲突 / 重叠 / 重复（**待合并清单 —— 只列不并**）

### C1 · 预设线（最大的一簇：同一主题、多轮改判、散成十几份）

时间线：2026-10-06（复制/身世 · 时间语义）→ 10-07（备注/副标题 · 切片器列 bug）→ 10-08（**同日三次改判**：工作副本 → 多版本+基线 → 资产库）→ 10-09/10-10 之后（列表与状态模型继续微调，**这些轮次目前只在代码注释与 `HANDOFF.md` 里，没有对应的 docs 文档**（推断））。

| 文档 | 与谁冲突 / 重叠 |
| --- | --- |
| `PRESET-PRODUCT-RULES.md`（现行） | 已用三条 ⚠️ 注记把 10-08 三次改判就地吸收；**是不是要再吸收 10-09/10-10 的后续**由你定 |
| `PRESET-WORKING-COPY-2026-10-08.md` | 被同日第三次改判**退役**，但正文没加"已退役"头注（只存在于 PRODUCT-RULES 的注记里） |
| `PRESET-MULTI-VERSION.md` | 文首已自标"已被取代"，指向下面那份 |
| `PRESET-ASSET-LIBRARY.md` | 10-08 的最后一次裁决；但它与现在代码里的"本地/云端"两轴是否完全一致，**待核**（推断：之后又有微调） |
| `PRESET-LIST-UX-ISSUES-2026-10-08.md` | 已闭环，正文大多过时，靠文首一句指向资产库 |
| `PRESET-COPY-LINEAGE.md` / `REMARK-SUBTITLE-2026-10-07.md` | 归属/身世主题的两份；10-08 改判后各有一部分退役，台账价值仍在 |
| `PRESET-TIME-SEMANTICS.md` → `PRESET-EVENT-TIME-MODEL.md` | 前后两篇同一主题，后者是落地版；前者留着当梳理稿 |
| `SupportEase-…规则.md`（URL 编码文件名） | HTML Demo 时代的旧口径，与现行产品规则**冲突**（旧文档早于真后端） |

> 合并锚点（供参考，不代你决定）：这一簇看起来以 `PRESET-PRODUCT-RULES.md`（现行）+ `PRESET-ASSET-LIBRARY.md`（最后一次裁决）为轴，其余按"过程记录"处理。

### C2 · 发布 / 寻址

`PUBLISH-KNIFE3/4-HANDOFF.md`（⚠️ 快照）+ `docs/PRESET-DELIVERY-CONTRACT.md`（⚠️ 部分被取代）+ `docs/TWO-REPO-ALIGNMENT.md`（⚠️ 快照）+ `docs/RESOURCE-ADDRESSING-ROADMAP.md`（半现行）vs 现行 `docs/PUBLISH-ARCHITECTURE.md` + `docs/RELEASE-TRANSACTIONS.md`。**前一组里凡提 `presets/dist/`、`baseUrl` 同目录推导的段落都已过时**（各文首自标）。

### C3 · 总账口径（谁在说"现在到哪了"）

- `README.md` 首段（"报告页还是空态"）**已过期** —— 报告页早已落地（#57 + 本分支"字段面补齐"）。
- `HANDOFF.md` 是最新台账（增量之三十四），`PROJECT-AUDIT.md`（10-02）/ `DATA-INVENTORY.md`（10-01）/ `audit-current-state/`（重写前）都是**更早的快照**。
- 合并时把"现状话术"统一到一处（大概率是 README + HANDOFF），其余为历史。

### C4 · `.comate/specs/`（**入库**，49 个文件 / 13 个包，上一代工具的 spec 存档）

| 包 | 一句话 | 备注 |
| --- | --- | --- |
| `b03-backstage-workbench` | 工作台第三版（**自标：这一版推翻了前两版**） | 含参考实现分析 |
| `b03-matrix-usability` | 参数矩阵可用性修复 | 九条缺陷 |
| `b03-param-desk` | 配方台默认视角改版 | — |
| `b04-panel-successor` | 工作台接管 mkppanel（包内 12 个文件） | 最大的一个包 |
| `b04-m4-preset-unify` | 预设解析迁入 + 注册表合一 | — |
| `b04-preset-write-guard` | crates 写盘纪律 | — |
| `b05-content-pipeline-charter` | 新工作台流程与数据契约章程 | 含资产域设计/清单 |
| `c15-a40-client-port` | 客户端移植 spec（只有 tasks.md） | 与根 `C15-A40-PORT-PLAN.md` 同一线 |
| `supportease-groundwork` | 本仓开局 spec（Tauri 壳 + v023 移植 + git 双层拦截） | 与下面那份是一对 |
| `un-tauri-groundwork` | **另一个项目（MKPSupportUN）** 的开局 spec | 误留在本仓？由你定（推断） |
| `asset-slimdown` | 资产瘦身（自标：**Task 6 提交 + 重写历史没动**） | — |
| `win-chrome-interactions` / `windows-caption-bar` | Windows 窗口三修 / 标题栏窗口键 | 已落（标题栏在代码里） |

> 这一群是否收进 `docs/`（做历史归档）还是原地不动，**由你定**；目前它们没有被任何现行文档引用。

### C5 · `.codebuddy/plans/`（**本机、不入库**，9 份计划）

- `preset-multi-version-snapshots-and-compare` **三份重复**（`1840bc96` 与两份"(未完成)"），且与 `docs/PRESET-MULTI-VERSION.md` 同一主题 —— 计划与成果件各一份。
- 其余未完成：`preset-param-baseline-update`、`preset-view-update-and-create-variant`、`后处理性能优化与执行归档`；已完成/据实：`APPSTATE-INVENTORY`（对应 docs 交付物）、`preset-asset-library-refactor`（对应 docs 记录）、`后处理算法级性能审计`。
- **只登记、不动文件**（IDE 在管这个目录；删/并请你在 IDE 里操作）。

### C6 · `workbench/` 本机目录（**不入库**）

`.sandbox/`（含 `audit/wt/` —— 仓库某时刻的**整份影子副本**：`HANDOFF.md`、`README.md`、`PUBLISH-KNIFE3/4-HANDOFF.md`、`pr-body.md` 全各有一份旧版，**与现行文档同名不同版**）、`.draft/`、`.snapshots/`、`.trash/`。`.gitignore` 自证"本机状态，删掉安全"。处置由你定。

### C7 · 小账（顺手核一眼）

- `docs/REPORT-DETAIL-FIELDS.md` 里写"spec 五组字段本仓后端尚未透传"——**本分支已补齐**（报告页字段面），口径可能已过期（推断）。
- `docs/HANDOFF-FRONTEND-CLEANUP-2026-10-09.md` 的"不重写"口径 vs 后续"前端照搬 a05 四页 + 皮肤整份替换"——两者关系待你判（推断）。

## 7. 维护约定（建议，等你点头）

1. **新文档落点**：现行规则 → `docs/` 顶层；裁决/计划/台账 → `docs/`（带日期或主题）；代码旁注 → 跟着代码；agent 产物 → `.comate/` / `.codebuddy/`（后者不入库）。
2. **合并时**：合成一份后回到 §6 **划掉一行**，并顺手更新本文 §1/§2 的状态列。
3. **本文只做索引**：任何"以谁为准"的判断都不写进这里 —— 那是作者的事。
