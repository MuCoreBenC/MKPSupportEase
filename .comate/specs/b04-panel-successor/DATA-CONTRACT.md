# B04 Source → Generator → Consumer 数据契约

状态：Draft 0.1（接口草案，尚未由消费端代码/测试确认）。
适用范围：B04 工作台、源数据解析/生成器，以及 MKP 消费端。
关联计划：[REBUILD-PLAN.md](./REBUILD-PLAN.md)

> 本文件先固定双方必须遵守的边界；凡标记为“待核实”的字段、路径或格式，不得直接当成已实现事实。P0 审计发现与本契约冲突时，先更新契约，再写代码。

## 1. 目标与非目标

目标：唯一权威源、确定性生成、消费端显式校验、错误可见、版本可协商、可复现验证。

非目标：让客户端直接编辑源 TOML；让 UI 自己解析/写文件；保留两份可编辑真相；用默认值或 fallback 掩盖契约不匹配；在未确认实际消费路径前强行规定最终文件扩展名。

## 2. 数据角色（不得混用）

| 角色 | 责任 | 是否权威 | 持有者 |
|---|---|---:|---|
| Source TOML | 人工维护的机型、参数定义/值及其他正式配置 | 是 | B04 source repository |
| Domain IR | 解析、校验、继承合并后的语义模型 | 否 | generator 进程内；默认不落盘 |
| Intermediate JSON | 仅当工具链确有跨进程/缓存需求时才允许；须另立 schema 与生命周期 | 否 | 待 ADR 决定 |
| Deliverable | 消费端实际读取的发布文件 | 否（可再生） | generator 输出目录/发布包 |
| UI Draft | 尚未保存的编辑态 | 否 | workbench session/storage |

“中间 JSON”不等于“客户端 JSON 交付物”。若客户端实际消费 JSON，它属于 Deliverable，必须单独定义 schema、版本和校验规则。

## 3. 权威源与写入规则

1. 正式配置仅以经确认的 Source TOML 为权威源。
2. `param_registry.toml` 的参数定义/范围/机型适用性规则保持 SSOT；不得在前端或消费端复制维护。
3. 所有修改经统一 IPC → application/domain → source writer；前端不得直接读写配置文件。
4. 参数保存必须对指定 machine + variant 执行 set/clear 语义；不得重写其他 variant 的有效值。
5. Writer 必须原子写入；保留无关注释、未知字段和未修改值的格式。若底层库无法保证格式保留，必须先给出可审查 diff，不得静默规范化整个文件。
6. 删除参数表示显式 clear/继承语义，不得擅自替换为零、空字符串或默认值。
7. UI Draft 与正式 Source 分离；关闭未保存编辑按产品规则丢弃或保留草稿，但不得悄悄提交正式源。

## 4. 生成协议

生成流程：

`Source TOML → Parse → Validate → Resolve inheritance/derive → Domain IR → Generate → Deliverable + Manifest`

- 相同 Source 内容、相同 generator 版本与相同选项，必须生成语义等价的产物。
- 生成器只读 Source；输出写入独立目标目录，禁止覆盖输入。
- 任一必需源文件缺失、语法错误、引用悬空、重复 ID、类型/范围不合法或 schema 不支持时，生成失败并返回定位信息；不得跳过文件继续发布。
- 未知字段默认报错或进入显式兼容策略；禁止静默丢弃。
- 发布采用 staging → 校验 → 原子替换/发布；失败不得留下“看似完整”的半套产物。
- 生成 diff 必须区分新增、删除、变更；删除项必须可见。
- Manifest 至少记录 `contract_version`、`generator_version`、产物相对路径、字节数及 SHA-256。字段最终命名在实现前与现有客户端协议对齐。

## 5. Deliverable 契约（待 P0 确认实际格式）

当前不得仅凭旧文件名决定 JSON/TOML。P0 必须给出：实际消费端入口、路径解析位置、反序列化类型、必需字段、现存夹具/测试、缺失文件行为。

确认后，交付 schema 至少应定义：

- `contract_version`：必填，语义版本或明确的兼容编号。
- `machine_id`：稳定机器标识，不依赖展示名。
- `variant_id`：稳定版本/变体标识。
- `parameters`：规范化参数键值；数值类型、单位及允许范围必须明确。
- `provenance`（如确有需要）：源版本/生成器版本/来源摘要，不得混入业务参数命名空间。

上面是字段类别要求，不是对当前客户端现有 schema 的断言。最终字段名、嵌套结构、文件拆分及编码由 ADR 根据消费者证据确定。

## 6. 消费端读取协议

消费端必须：

1. 只从约定的 Deliverable 入口读取，不直接读取 workbench 草稿或编辑用 Source TOML（除非 ADR 明确它本身就是交付格式）。
2. 在应用数据前校验 contract version、machine/variant 标识、必需字段、类型与范围。
3. 对不支持的 major contract version、损坏文件、缺字段、重复键、未知必需语义返回明确错误；不得静默降级到 mock、旧缓存或内置猜测值。
4. 读取失败时保留现有已确认状态，不将部分解析结果应用为新状态；向调用方返回结构化错误。
5. 不回写或“修复”发布文件。需要编辑时必须回到 B04 Source 工作流。
6. 如需缓存，缓存必须可丢弃、可重建，并绑定产物 hash/contract version；缓存不能成为第二权威源。

## 7. 错误模型

错误至少包含稳定 `code`、`stage`、人类可读 `message`；可定位时附 `file`、`path`、`machine_id`、`variant_id`。不得把原始文件内容或敏感本地路径无必要地暴露到 UI 日志。

建议阶段码：`parse`、`validate`、`resolve`、`generate`、`publish`、`consume`。具体 enum 在共享类型实现时冻结。

## 8. 兼容策略

- 同一 major contract version 内允许向后兼容的可选字段扩展；消费端忽略可选字段的前提是 schema 明确其可忽略。
- major 不兼容变更必须提升 major 并由消费端显式支持；不支持则拒绝加载并显示版本不兼容。
- 删除/改名/改类型/改单位/改变继承语义均视为潜在破坏性变更，必须有迁移/版本策略和契约测试。
- 不做隐式自动迁移，不在读取时改写用户文件。

## 9. 双方契约测试

同一组 fixture 同时供 generator 与 consumer contract tests 使用。最低测试集：

- 有效的 machine + variant 可生成并被消费端读取。
- 缺少必需源字段时 generator 明确失败。
- 不支持的 contract major version 时 consumer 明确拒绝。
- 参数类型/范围错误时 generator 拒绝发布。
- 删除某个 variant 参数后，其他 variant 的输出不变。
- 相同输入连续生成，产物语义与 manifest hash 稳定。
- 输出损坏/截断时 consumer 不应用部分状态。
- 旧/未知字段按已声明策略处理，不静默丢弃。
- 源文件 hash 与非目标 variant 对照证明写入边界正确。

## 10. P0 / ADR 必须关闭的决策

- [ ] 真实消费端仓库、入口与调用链（含处理器/客户端）。
- [ ] 当前交付物究竟是 TOML、JSON，还是二者分别承担不同角色。
- [ ] 原始 TOML → 中间 JSON → 最终 TOML 是否为真实链路；中间 JSON 是否需要落盘。
- [ ] machine/variant ID 的现有规范及映射规则。
- [ ] 参数单位、浮点精度、缺省/继承/删除语义。
- [ ] Manifest 的消费者、发布路径及原子发布方式。
- [ ] contract version 的当前值、升级和兼容窗口。
- [ ] 源端与消费端是否同仓库/同发布节奏，如何共享 schema 与 fixtures。

在以上关键项未核实前，本文件是边界草案，不是可据此宣称互通的最终协议。
