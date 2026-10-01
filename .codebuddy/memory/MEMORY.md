# MEMORY（长期记忆）

## 项目：MKPSupportEase（Tauri + React + Rust workspace）

- 交接分支 `feat/b04-p3-migration`（spec 文档内称 b05，同一件事）；交接文档在仓库根 `HANDOFF.md`；任务台账在 `.comate/specs/b05-content-pipeline-charter/tasks.md`；产品契约 doc 的 §4.2（六页面映射）/§4.3（11 步端到端）在 `.comate/specs/b05-content-pipeline-charter/doc.md`。
- 提交纪律：每个 Task 收口提交一次；工作分支直推，**不合并 main**；九份产物逐字节不变是硬防线。2026-09-25 状态：Task 14（e4f85bd）+ Task 15（c349858）已推送；**Task 16（7a468cf）+ 17.0（282a55e）+ 17.0b（364a762）+ 17.1（5a07e08/2c156e4）+ 17.2 六页骨架（d463bb5）+ 17.2a-1 UX 走查（2ff2237/a93fce6）已提交、均未推送**（ahead 8）。新前端六页骨架 + UX 整理完成（页头职责句/主操作突出/折叠收纳/来源标注/步骤序号/状态徽章/流程引导）；旧前端冻结在 e4f85bd。**骨架后待补**：参数台草稿编辑（单独一轮）、生成前预览、版本文本格编辑、文件选择器、C-2/C-3 后端命令。设计稿在 `docs/workbench-next-design.md`（已审稿通过）。`.codebuddy/` 不入库。开发模式（2026-09-24 与用户约定）：**后端契约先行 → 前端消费契约 → 端到端验收**，不再逐步询问是否开工，只有影响实现的产品决策才停下来问。
- 工作台前端在 `src/workbench/`（入口 workbench.html / `npm run tauri:workbench:dev`）；纪律：**前端不算业务**——状态词/禁用理由/diff 口径全部来自后端返回（words.* / IssueReport / artifact），api.ts 不许判状态、不许拼中文句子；写只有 `wb_apply_draft` 一条。
- 后端纪律：基线写入口唯一（`wb_sync_baseline` → `preset::generate::sync_baseline`，落点闸在 crate 内）；`the_baseline_has_exactly_one_write_path` 是活白名单判据。版本复制只写版本定义、不抄 `presetFile`（G-2 待删）；14.5 复制是**独立快照**（裁决 A）。
- 验证命令集：`npx tsc -b` / `npm run build:workbench` / `npx eslint`+`stylelint`；`cargo test -p mkp-support-ease --features workbench --lib`（注意包名 mkp-support-ease，crate 名易拼错）；`cargo clippy -p mkp-support-ease --features workbench --all-targets -- -D warnings`；写纪律判据在 `crates/preset/tests/write_discipline_scan.rs`。
- 沟通风格：用户以"监管/审核"口吻下达任务，重视：裁决要可追溯（写进 tasks.md 注记）、失败语义要如实分态、UI 不摆假按钮、验收判据要反空转。用户会显式说"不推送"。

## 推进规则（2026-09-24 与用户约定，务必遵守）

- **每个节点只有：一句话完成条件 + 一个下一步。** 收工只报三件事：达成/未达成 · 台账登记了几条 · 下一步。
- **问题只分两类**：阻断（当前功能会错 / 数据会坏 / 下一节点做不了）→ 现在解决；其余 → 记进台账，继续走。**验收通过即提交；提交后非阻断项不得成为重开该节点的理由。**
- **验收报告只回答"完成条件达成没有"**；扫描中顺手发现的优化/疑似项一律进台账（tasks.md 遗留段），不写进报告正文——写进去就等于把节点重新打开。这是我此前反复拖长节点的直接原因。
- **不自发做"全仓盘点"式扩展动作**；只有用户点名要时才做。
- **断言仓库状态前先跑 `git log` / `git status`**（曾连续多轮照着记忆说"未提交"，实际已提交并推送 c349858，给用户造成额外认知负担）。
