# 全仓文档索引（整理与待合并清单）

> 整理日期：2026-10-11 ｜ 依据：各文档**自己的文首标注**（⚠️ 历史快照 / 已被取代 / 已实施 / 施工计划…）
> ＋ 各目录盘点。本文**不做裁决、不做合并** —— 哪份并入哪份、以谁为准，由作者自己裁决；
> 没有标注可引用、由盘点判断的，行内都标了**（推断）**。
>
> **2026-10-11 同日做过第一轮清理**（历史交接件 / 上一代工具 spec / 死代码 / 分支），处置记录见 **§6 C8**。
> 冲突 / 重叠 / 重复仍在 §6 逐簇列着 —— 合并完回来划掉一行即可。

## 0. 文档地图（一页看完）

| 群 | 位置 | 数量 | 性质 |
| --- | --- | --- | --- |
| 现行规则 | `docs/` 顶层 + 根 `README.md` + [`HANDOFF.md`](../HANDOFF.md) | 约 12 份 | 根规则 / 产品规则 / 工程约束 / 合同 |
| 记录与计划 | `docs/` 顶层（带日期的那些） | 约 19 份 | 裁决记录 / 台账 / 施工计划 / 测试协议 |
| 审计基线 | `docs/audit-current-state/` | 9 md + data/map | 重写前的只读审计（"建议，不是现状"） |
| 移植方案 | 根 [`C14-PORT-PLAN.md`](../C14-PORT-PLAN.md) / [`C15-A40-PORT-PLAN.md`](../C15-A40-PORT-PLAN.md) | 2 份 | 被 `src/workbench` 现行代码引用（方案 / 待裁决清单），保留 |
| 历史交接 | **已清（2026-10-11）** | — | KNIFE3/4、HANDOFF-5、ASSET-CHAIN、TWO-REPO、DELIVERY-CONTRACT、SupportEase 旧规则件已删（§6 C8） |
| 旧工具 spec | **已整包删除（2026-10-11）** | — | `.comate/specs/`（原 49 件），git 历史可查（§6 C8） |
| 计划文件 | `.codebuddy/plans/`（**本机、不入库**，见 `.gitignore`） | 9 份 | 现役工具的计划（含重复与未完成，未动） |
| 工作台本机状态 | `workbench/.draft/` `.sandbox/` `.snapshots/` `.trash/`（**本机、不入库**） | — | 审计工作树 `wt` 已移除（§6 C8） |

## 1. 现行规则（先读这一组）

| 文档 | 管什么 | 状态 |
| --- | --- | --- |
| [`README.md`](../README.md) | 入口：项目一句话 / 跑法 / 文档地图 | 现行（首段的"报告页"口径已于 2026-10-11 修正） |
| [`docs/DATA-ARCHITECTURE.md`](DATA-ARCHITECTURE.md) | **根规则**：四层世界 + 四铁律 + 十问 + 准入问句 | 现行 |
| [`docs/PRESET-PRODUCT-RULES.md`](PRESET-PRODUCT-RULES.md) | 产品规则：本地 / 云端 / 下载 / 更新 / 修改 / SHA / 归档 | 现行（正文已就地吸收 2026-10-08 三次改判，含三条 ⚠️ 注记） |
| [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) | 工程约束：目录、两层数据根、IPC、trace、原子写、权限 | 现行 |
| [`docs/GIT-WORKFLOW.md`](GIT-WORKFLOW.md) | git 纪律：本地八道闸 + 服务端 ruleset + 发版 | 现行（其中的合并动作由作者执行，AI 只提交+推送） |
| [`docs/PUBLISH-ARCHITECTURE.md`](PUBLISH-ARCHITECTURE.md) | **发布链根规则**：唯一源 → 发布根 → 客户端消费 | 现行（其替代的旧件 `PRESET-DELIVERY-CONTRACT` 已于 2026-10-11 删除） |
| [`docs/RELEASE-TRANSACTIONS.md`](RELEASE-TRANSACTIONS.md) | 发布事务合同：发布预设 ≠ 发布软件版本 | 现行（自标：改它要作者点头） |
| [`docs/APP-STATE.md`](APP-STATE.md) | 持久化状态地基：一个应用一个 AppState | 现行（10-08 盘点定案已并回本文 §5 / §10） |
| [`docs/DESIGN-SPACING.md`](DESIGN-SPACING.md) | 间距与动画原则（试验场搬来） | 现行 |
| [`docs/3D-ASSET-CONTRACT.md`](3D-ASSET-CONTRACT.md) | 上游 3D 资产契约 v3（对外） | 现行（自标：contract v3 取代 v2） |
| [`HANDOFF.md`](../HANDOFF.md) | **进度台账**：现在在哪、接下来去哪 | 现行·持续追加（最新到"增量之三十四"；头部三条 ⚠️ 注记：两条旧口径 + 一条 2026-10-11 清理） |
| [`docs/RESOURCE-ADDRESSING-ROADMAP.md`](RESOURCE-ADDRESSING-ROADMAP.md) | 寻址改造总纲（长期大纲） | 半现行：自标"大纲已批准 2026-10-05，施工进度以 §8 台账为准" |
| [`docs/PROJECT-AUDIT.md`](PROJECT-AUDIT.md) | 项目总盘点：已定义但未落地（四类） | 参考：2026-10-02 的对账，时效以日期计 |
| [`docs/DATA-INVENTORY.md`](DATA-INVENTORY.md) | 总纲的对账单（43 件归位 + 收口日志） | 对账单：基准 2026-10-01 `58f4295`，收口一条勾一条（是否勾完没终检） |

## 2. 记录与计划（docs/ 顶层带日期的那些）

| 文档 | 日期 | 一句话 | 状态（自标优先） |
| --- | --- | --- | --- |
| [`HOOK-ERRORS-AND-MESSAGES.md`](HOOK-ERRORS-AND-MESSAGES.md) | 2026-10-11 | 后处理的错误：切片器那一框吃什么（BBS 源码证据）+ 文案表 + 两条日志流 | 现行（本分支新增，随"切片器能显示什么"那一刀） |
| [`A05-PORT-LEDGER.md`](A05-PORT-LEDGER.md) | 2026-10-11 | a05 前端整份搬运台账（源 = frontend-lab） | 现行（本分支新增，对应"前端照搬 a05"提交） |
| [`A06-PORT-LEDGER.md`](A06-PORT-LEDGER.md) | 2026-10-12 | a06 后处理那一屏搬运台账 + 读数那一刀（源 = frontend-lab a06 + lab/mkp-postprocess 定稿） | 现行（本分支新增，两刀都已完成） |
| [`A07-PORT-LEDGER.md`](A07-PORT-LEDGER.md) | 2026-10-14 | a07 全量搬运台账 + **术语改造（官方/我的 → 云端/本地）施工单**（源 = frontend-lab a07 + lab/term-rename-local-cloud.md） | 第一笔已落地（`b7ebafb`）；**术语那一刀未做**，施工单与验收都在这份里 |
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

## 4. 移植方案（保留：被现行代码引用）

| 文档 | 一句话 | 为什么留 |
| --- | --- | --- |
| [`C14-PORT-PLAN.md`](../C14-PORT-PLAN.md) | 工作台 C14 → 产品仓移植方案 | `src/workbench/views/*` 多处引用它的 §4「待裁决清单」（那几个"没接"的动作还挂在那里） |
| [`C15-A40-PORT-PLAN.md`](../C15-A40-PORT-PLAN.md) | 客户端 A21→A40 补齐 · 工作台 C14→C15 增量方案 | `src/api/contract.ts` 与 `src-tauri/src/ipc/bbs.rs` 引用它的 §6-2（不打包预设快照那条裁决的由来；`vite.config.ts` 那段引用 2026-10-13 随 bbsFs 退役摘掉） |

> 早期"刀 / 圈"时代的交接件（`PUBLISH-KNIFE3/4-HANDOFF.md`、`HANDOFF-5-APP-UPDATE.md`、
> `ASSET-CHAIN-HANDOFF.md`）与 `docs/` 里的历史快照件（`TWO-REPO-ALIGNMENT.md`、
> `PRESET-DELIVERY-CONTRACT.md`、`SupportEase-…规则.md`）已于 2026-10-11 删除 —— 见 §6 C8。

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

> 合并锚点（供参考，不代你决定）：这一簇看起来以 `PRESET-PRODUCT-RULES.md`（现行）+ `PRESET-ASSET-LIBRARY.md`（最后一次裁决）为轴，其余按"过程记录"处理。
> （原表里那份 HTML Demo 时代的 `SupportEase-…规则.md` 已随 2026-10-11 清理删除。）

### C2 · 发布 / 寻址 —— **已清（2026-10-11）**

历史件（`PUBLISH-KNIFE3/4-HANDOFF.md`、`PRESET-DELIVERY-CONTRACT.md`、`TWO-REPO-ALIGNMENT.md`）已删除。
本簇现在只剩现行两份（`PUBLISH-ARCHITECTURE.md` + `RELEASE-TRANSACTIONS.md`）与半现行的
`RESOURCE-ADDRESSING-ROADMAP.md` —— 它们是"根规则 / 合同 / 大纲"三司其职，**不再有冲突，无需合并**。

### C3 · 总账口径（谁在说"现在到哪了"）

- `README.md` 首段的"报告页还是空态"已于 2026-10-11 修正为现状。
- `HANDOFF.md` 是最新台账（增量之三十四），`PROJECT-AUDIT.md`（10-02）/ `DATA-INVENTORY.md`（10-01）/ `audit-current-state/`（重写前）都是**更早的快照**。
- 合并时把"现状话术"统一到一处（大概率是 README + HANDOFF），其余为历史。

### C4 · `.comate/specs/` —— **已整包删除（2026-10-11）**

原 13 个包 / 49 个文件（B03 / B04 / B05 / 开局地基 / `asset-slimdown` / `win-*`）已删（git 历史可查）。
4 处引用已改写为"已删注记"：`src-tauri/src/lib.rs`、`src-tauri/src/presetdata/mod.rs`、
`src/app/shell/icons.tsx`、`docs/ARCHITECTURE.md`（§10）。**无需合并。**

### C5 · `.codebuddy/plans/`（**本机、不入库**，9 份计划，未动）

- `preset-multi-version-snapshots-and-compare` **三份重复**（`1840bc96` 与两份"(未完成)"），且与 `docs/PRESET-MULTI-VERSION.md` 同一主题 —— 计划与成果件各一份。
- 其余未完成：`preset-param-baseline-update`、`preset-view-update-and-create-variant`、`后处理性能优化与执行归档`；已完成/据实：`APPSTATE-INVENTORY`（对应 docs 交付物）、`preset-asset-library-refactor`（对应 docs 记录）、`后处理算法级性能审计`。
- **只登记、不动文件**（IDE 在管这个目录；删/并请你在 IDE 里操作）。

### C6 · `workbench/` 本机目录（**不入库**）

- `audit/wt`（仓库影子副本，含审计插桩）已于 2026-10-11 移除；
- 剩 `.sandbox/`（presets 副本 + 台账）、`.draft/`、`.snapshots/`、`.trash/`；
- `.gitignore` 自证"本机状态，删掉安全"——要不要再清一层由你定。

### C7 · 小账（顺手核一眼）

- `docs/HANDOFF-FRONTEND-CLEANUP-2026-10-09.md` 的"不重写"口径 vs 后续"前端照搬 a05 四页 + 皮肤整份替换"——两者关系待你判（推断）。

### C8 · 已处置记录（2026-10-11）

| 项 | 处置 |
| --- | --- |
| 历史交接件（6 份） | 删除：`PUBLISH-KNIFE3/4-HANDOFF.md`、`HANDOFF-5-APP-UPDATE.md`、`ASSET-CHAIN-HANDOFF.md`、`docs/TWO-REPO-ALIGNMENT.md`、`docs/PRESET-DELIVERY-CONTRACT.md`、`docs/SupportEase-…规则.md`；`HANDOFF.md` 头部加了清理注记 |
| `.comate/specs/`（49 件） | 整包删除；4 处引用改写为"已删注记" |
| 死代码 | `PresetTable.module.css` 的 `.actLive`（全仓无组件使用）删除 |
| 审计工作树 | `workbench/.sandbox/audit/wt`（审计插桩，按当时计划"不提交"）已移除 |
| tmp 残留 | `tmp-port/` 清空（保留两份 pr-body 草稿 + 分支备份 bundle）、`tmp-shots/` 清空（仅剩一个被安全删除拒绝回收的 `wv2-cdp-profile` 调试目录，可手动删） |
| README | 首段"报告页还是空态"更新为"已接真数据" |
| 分支 | 本地 + 远端只留 `main` 与 `fix/postprocess-command`；其余 5 条删除。删前核实：`refactor/preset-first` 已随 PR #57 合并、`feat/build-delivery-files` 已随 PR #56 合并、`chore/identifier-rename` 的改名内容已在 main（`tauri.conf.json` identifier = `SupportEase`）；两条无 PR 记录的旧线（`feat/preset-multi-version-compare`、`feat/preset-param-official-update`）已打包成 `tmp-port/branches-backup-2026-10-11.bundle`（`git bundle verify` 通过） |

## 7. 维护约定

1. **新文档落点**：现行规则 → `docs/` 顶层；裁决/计划/台账 → `docs/`（带日期或主题）；代码旁注 → 跟着代码；agent 产物 → `.codebuddy/`（不入库；`.comate/` 已于 2026-10-11 删除）。
2. **合并时**：合成一份后回到 §6 **划掉一行**，并顺手更新本文 §1/§2 的状态列。
3. **清理 / 处置记录**进 §6（C8 的样式），别散落。
4. **本文只做索引**：任何"以谁为准"的判断都不写进这里 —— 那是作者的事。
