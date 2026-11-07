# 地图 5 · 模块问题地图（种子）

> 最后核对：2026-11-07
> 只记录**已确认**的问题（带证据）与**待核**清单。架构裁决不在这里做，见 `docs/ARCHITECTURE.md` / `PUBLISH-ARCHITECTURE.md` 与作者裁决。

---

## A. 已确认（有证据，可复现）

### A1. `enabled` 只在 catalog 一处生效，交付集合有 5 套算法

- 现象：停用一台机器后，产物仍被当作"应该存在"，catalog 却不登记它 → 发布核对永远失败。
- 证据：`runtime/catalog.rs:606` 是唯一认 `enabled` 的地方；`delivery_expected_set`（`delivery.rs:642/671`）、`candidate_once`（`:810/825`）、`planned_todos`（`build.rs:580/618`）、`plan_publish`（`delivery.rs:1984/2166`）都不认。
- 影响：P2S 现场（`presets/machines/P2S.toml:13` enabled=false，产物仍在，catalog 只有 8 条预设）。
- 归属：重构阶段 1/4。

### A2. `manifest.json` 被当成"持久凭据"，但它只是发布快照

- 现象：生成一次产物就报"发布凭据失效"，还需要人工修复入口。
- 证据：唯一写入口是发布事务（`delivery.rs:1916`）；客户端 0 处业务读取（`ResourceRef::Manifest` 仅 `resolver.rs:257` 与 `audit.rs:1182`）。
- 影响：一整类"生成后要修凭据"的状态与 UI（`FinalizeModal`、`wb_update_publish_credentials`）。
- 归属：阶段 4（删除）。

### A3. 发布身份双存储，且实测已不同代

- 现象：界面报"发布账本与当前目录不是同一代"。
- 证据：`catalog.revision = 9d6cd17786a456f0` vs `publish-serial.json.revision = dc0884e1b1b0c421`；判定 `publish_book::inspect`（`publish_book.rs:233`）。
- 影响：核对器被迫做"三账相等"，任一落后即故障。
- 归属：阶段 4。

### A4. `catalog` 与 `manifest` 登记的是不同集合，却用同一组字段表达

- 证据：catalog 25 条（mkp_preset 8 / bbs_config 9 / model 5 / icon 3）；manifest 17 条（mkp_preset 9 / slicerProfile 5 / icon 3）。
- 影响：⑭"两账一致"永远有噪音（当前 3 条"manifest 有、catalog 里没有"）。
- 归属：阶段 4（删 manifest 后消失）。

### A5. `content/*.json` 三份客户端不读

- 证据：`ResourceRef::Content` 只在 `audit.rs:1203`（⑯ URL 对账）与 resolver 测试；`runtime/**`、`ipc/**`、`src/**` 无业务读取（客户端展示机型走 `ipc/presets.rs:341 get_machines` → 本地预设）。
- 注意：本仓内无消费者 ≠ 产品上不需要；外部消费者**未核实**。
- 归属：阶段 4（需作者确认）。

### A6. `source.json` 每代重写但字节恒定

- 证据：正文来自常量 `bootstrap_json()`（`delivery.rs:134-147`）；`PublishPlan::write` 无条件写（`delivery.rs:1920`）。
- 影响：无功能危害，但属"写一次没人读"的噪音，也让"发布会动几份文件"的数字虚高。
- 归属：阶段 4。

### A7. 交付检查器依赖"定稿算得出来"，一度吞掉可修项

- 现象（已修）：`plan_publish` 因 P2S 报错 → 定稿 manifest 算不出 → 依赖它的"9 份凭据"问题条目一起消失。
- 证据：修复前 `build_issues` 用 `manifest_preview` 结果判；现改为 `manifest_drift`（`delivery.rs:1486`）。
- 根因仍在 A1/A2；本条记录的是"检查器不应依赖被检查对象的计算结果"。
- 归属：阶段 3（统一检查器）。

### A8. 「更新发布凭据」这个用户操作本身是设计错误

- 证据：它是本轮新增的写入口（`build.rs:2039/2129`），为了让"manifest 与盘上一致"而存在。
- 结论：manifest 定位 B（A2）⇒ 该操作应删除，而不是加保护条件。
- 归属：阶段 4（删除），UI 阶段 5。

### A9. `wb_assign_bundle_versions` 回执不含"哪几条真变了"

- 证据：内核算了 `changed`（`mod.rs:1070`），命令只回 `BundleList`（`app/bundles.rs:422`）。
- 影响：前端无法区分"批量指向"里哪些版本被改了、哪些本来就是这个套餐。
- 归属：小修（若保留该命令）。

### A10. 草稿里的套餐补丁（`Patch::SetBundle`）是**潜在死路径**

- 现象：`Book::bundle_bbs` 优先读草稿 overlay（`derive.rs:298-301`），而该 overlay 只能由 `Patch::SetBundle` 产生；但前端 `api.ts:806` 只有**类型定义**，全仓搜不到构造点 ⇒ 今天这条路径走不到。
- 为什么不删：它是"套餐内容进草稿"的唯一通道，将来若要让套餐编辑也走撤销/保存（与参数台一致），会直接接上。
- 风险：接上那天，"套餐页显示草稿态、生成按已保存态"就会变成真问题 —— 除非同时把 `require_clean` 的语义一并想清。
- 归属：待裁决（保留则补测试；不保留则删 Patch 变体与 overlay 分支）。

### A11. `docs/HOME-BUNDLE-DOWNLOAD.md` 与代码不一致（已过期）

- 文档说「BBS 只落盘、不碰切片器…『拷进切片器』是一条**还没存在**的写命令」（`:200-204`）。
- 现状：`copy_to_slicer`（`ipc/presets.rs:1062`，2026-10-13 改判「有相同的就覆盖」）与 `get_slicer_copied`（`:1019`）都已存在并注册（`lib.rs:562/754`），前端有按钮（`PagePresets.tsx:90`、`PresetTable.tsx:55`）。
- 处置：以代码为准；文档要么改，要么标注"二期已落地"。

---

## B. 待核（需要运行或读未读代码）

| # | 问题 | 状态 | 结论 / 怎么核 |
|---|---|---|---|
| B1 | 客户端下载的 BBS 文件落盘后由谁消费 | **已核实（2026-11-07）** | 落点 `<appDataDir>/assets/bbs/Process/<喷嘴>mm/`（`runtime/catalog.rs:258/300`）；被 `ipc/catalog.rs:239` 与 `src/app/bbs/useBbsDelivered.ts:106` 读取，参与 BBS 页（`useBbsData.ts:142-185`），并可一键写入切片器（`ipc/presets.rs:1062`）。**后处理 0 依赖**（`crates/postprocess` 仅注释、`hook.rs` 0 命中）。详见 `04-data.md` §4.1 |
| B2 | `public/bbs/*.json` 与套餐/资产是否有 join | **部分核实** | 与**交付 bbs 预设**靠参数名字符串 join（`bbsMerge.ts:192/188`）、预设间按 `name`/`inherits`（`:100-105`）；与**套餐**无直接 join。702 条参数与 9 份 json 的覆盖率未逐条核 |
| B3 | `machine_dims::install()` 在发布物里是否真的接线 | 待核 | 搜 `src-tauri/`；看 `HANDOFF.md:259` 的欠账 |
| B4 | 生成时是否用到机型尺寸数值 | **已核实（2026-11-07）** | **不参与产物**：`render` 只用 `machine.id`（`build.rs:309`）；尺寸只进 `catalog.machines[].dimensions`（`runtime/catalog.rs:194/628`）进 `revision`（`:842`），不进 `structureSignature`（`structure.rs:148-156`）。改尺寸**不需要重新生成产物**，但要让客户端看见需重发目录。详见 `04-data.md` §4.2 |
| B5 | 草稿 overlay 是否造成页面显示与产物不一致 | **已核实（2026-11-07）** | **不是活动路径**：套餐页两个写操作都即时落盘（`app/bundles.rs:255`、`app/machines.rs:291`）；`Patch::SetBundle` 前端只有类型无构造（`api.ts:806`）；生成前 `require_clean` 拒绝任何脏（`build.rs:696-700`）。唯一差异窗口是"有未保存草稿时页面读 `Book`（含草稿）"，但那时生成被拦。见 A10 |
| B6 | 停用机型后 `content/machine_catalog.json` 与 `catalog.json` 条目是否同时消失 | 待核（Lab） | 在 Delivery Lab 跑一次（不在此仓动手） |

---

## C. 验收问题（这张图必须能回答）

1. 为什么"生成一次"会引出需要人处理的"凭据失效"？（A2）
2. 为什么停用一台机器会产生孤立产物？谁该负责清理？（A1）
3. 同一代发布事实现在被记在几处？哪一处是权威？（A3/A4）
4. 哪些"用户操作"其实是在替内部状态擦屁股？（A2/A8）
5. 如果只修一个地方，哪里能让 A1-A4 同时收敛？（答案应指向"唯一交付集合 + 单一索引 + 单一序号"，见 `../DELIVERY-ARCHITECTURE-RESEARCH.md` §6）
