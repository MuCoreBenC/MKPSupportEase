# 预设与客户端版本兼容

本文是预设数据交付的**兼容台账**。它回答三件事：哪个客户端能拿哪批预设、旧预设去哪拿、发布前必须过什么闸。

## 1. 三代判据

| 判据 | 回答什么 | 真源 |
|---|---|---|
| `catalog.minClientVersion` | **哪个客户端有资格拿这批数据** | `presets/structure-signatures.toml` |
| `presetSchemaSignature` | 这份预设的 TOML 形态是哪一代 | `crates/preset` 的 `TomlConfig` 键集合/类型 |
| `presetContractEpoch` | 形态没变但语义变了的代次 | `crates/preset/src/schema.rs` |

`minClientVersion` 不是手填的常量：生成器用合并后的结构签名查规则表，查不到就**禁止发布**。

## 2. 当前这一代

| 项目 | 值 |
|---|---|
| 客户端 | 0.0.18（旧）/ 0.0.19（新） |
| 预设发布序号 | 22（目标 23） |
| 最低客户端 | `0.0.19` |
| 旧预设权威 | git tag `v0.0.18`（`presets/delivery/mkp/presets/` + `catalog.json`） |
| 当前主线 | 只保留新一代预设，不复制旧预设回 `presets/delivery/` |

旧用户：

- 0.0.18 的 `readable` 闸会让它拒绝新目录，继续用本地旧预设；
- 需要重新下载旧预设时，从 `v0.0.18` tag/Release 取，不从 main 取。

## 3. 四种交叉情况

| 客户端 | 预设 | 行为 |
|---|---|---|
| 0.0.18 | 0.0.18 | 正常 |
| 0.0.19 | 0.0.19 | 正常 |
| 0.0.18 | 0.0.19 | 0.0.18 的 `deny_unknown_fields` 会因新键失败；或缺失键被默认值静默替换 —— 必须靠 `minClient` 排除 |
| 0.0.19 | 0.0.18 | 旧键（如 `bed_glue_preheat_temp`）被当前模型拒绝；本地列表标「与当前程序不匹配」 |

**能解析 ≠ 兼容。** 因此发布前必须用真实历史解析器跑矩阵，不能只看结构签名。

## 4. 历史解析器矩阵

```bash
npm run compat:matrix
```

- 读取 `presets/compat-clients.toml` 里登记的正式 tag；
- 对每个 tag 用临时 worktree 编译该 tag 的 `crates/preset` 探针；
- 再跑一遍**当前构建**，作为“新程序读得懂自己这一代”的基线；
- 读取 `presets/delivery/mkp/presets/*.toml`，输出 `presets/compat-matrix.json`（不进交付目录）。

发布闸（`presets publish` / `--dry-run`）会核对：

```text
每个候选文件 × 每个客户端：
  客户端版本 >= minClientVersion  ⇒ 必须 pass
  客户端版本 <  minClientVersion  ⇒ 允许 excluded，但不许冒充 pass
否则禁止发布。
```

新增一个受支持旧版本：在 `presets/compat-clients.toml` 追加 tag，然后重跑矩阵。

## 5. 发布顺序

1. 确认目标正式版（本次 0.0.19）已可获取；
2. `npm run presets -- generate`；
3. `npm run compat:matrix`；
4. `npm run presets -- publish --dry-run`，核对序号 23、`minClient 0.0.19`、矩阵结论；
5. 取得当次明确授权后 `npm run presets -- publish --confirm`。

**顺序不能反**：先发数据、后发客户端会让旧用户看到「需升级」却没有可升级版本。

## 6. 换数据源不是“云端数据异常”

`BasedOn` 的判定顺序：

1. 两侧都有分支 ref ⇒ 只比较 ref（GitHub/Gitee 镜像同提交仍同源）；
2. 任一侧没有分支（自定义源 `NO_REF`）⇒ 比较完整源身份（host + 路径）；
3. 一侧有分支、一侧没有 ⇒ `Elsewhere`；
4. 只有「可证明同源 + 同发布线 + 同序号 + SHA 不同」才是 `Inconsistent`。

动作：`Elsewhere` / `Inconsistent` / `Unverified` 都允许「下载 / 重新下载」；旧份走现有归档管道，不因为“曾经下载过”禁用。
